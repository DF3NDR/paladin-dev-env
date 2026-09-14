//! Shared LLM adapter conformance suite (RT-06, D-31; Phase 26 plan 26-14).
//!
//! Before this module, `openai_compatible`, `gemini` and `ollama` each carried their own
//! hand-written mockito test module, so no two adapters were measured against the same bar
//! (26-CONTEXT.md D-31, 26-PATTERNS.md "No Analog Found"). [`ConformanceFixture`] plus the
//! [`llm_conformance_suite!`] macro fix that: one fixed case list -- generate success/usage
//! extraction, streaming assembly with a terminal stop, a mid-stream error before AND after the
//! first chunk, dedicated 401/404/400/402 status mappings, transience-by-value for
//! 408/429/5xx-vs-other-4xx, credential redaction, and refused-redirect handling -- instantiated
//! once per adapter so every provider answers the identical set of questions.
//!
//! **Measurement, not re-implementation.** This suite exercises each adapter's real `generate()`
//! / `generate_stream()` path end-to-end against a `mockito` server, so it measures whether
//! Phase 25's shared `crate::http_status::map_http_status` (D-03) is actually *reached* by every
//! adapter -- it does not re-implement or duplicate that classification (see
//! `crate::http_status` and `crate::redaction` for the mapping and redaction logic this suite
//! proves is wired in).
//!
//! `#[cfg(test)]`-only: [`ConformanceFixture`] and [`llm_conformance_suite!`] never ship in a
//! release build.

use std::sync::Arc;

use futures::StreamExt;
use mockito::{Matcher, Server};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::llm_port::{LlmError, LlmPort, LlmRequest};

use crate::redaction::RESPONSE_EXCERPT_CHAR_BUDGET;

/// Which response-wire shape a [`ConformanceFixture`]'s adapter speaks.
///
/// Exactly two shapes exist in this crate today (26-PATTERNS.md): the OpenAI-style
/// `choices[].message.content` / `choices[].delta.content` framing every `CompatEngine`-based
/// preset speaks (terminated by a `data: [DONE]` sentinel), and Gemini's bespoke
/// `candidates[].content.parts[].text` framing (which has no `[DONE]` sentinel -- the stream
/// simply ends when the body ends). Recorded on the fixture for documentation; the suite's cases
/// themselves do not currently branch on it -- every case is expressed generically against
/// [`LlmPort`], with each fixture supplying wire-correct bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // documentation-only today (see doc comment above); no case currently branches on it
pub enum Wire {
    /// OpenAI-compatible `choices[]` framing (openai_compatible, ollama, and every other
    /// `CompatEngine`-based preset).
    OpenAiChat,
    /// Gemini's bespoke `candidates[]` framing.
    Gemini,
}

/// One adapter's real-body conformance fixture (D-31).
///
/// A concrete `Fixture` type is a zero-sized marker implementing this trait, instantiated once
/// per adapter via [`llm_conformance_suite!`]. Every method returns REAL bodies -- copied from
/// (or structurally identical to) that adapter's own pre-existing hand-written tests -- never
/// synthetic ones invented for this suite (26-PATTERNS.md "No Analog Found": "use the per-adapter
/// test modules only as the source of *case content*").
pub trait ConformanceFixture {
    /// The wire shape this fixture's adapter speaks (documentation only -- see [`Wire`]).
    #[allow(dead_code)]
    const WIRE: Wire;

    /// Build a live adapter instance pointed at `base_url` (a `mockito` server's URL).
    fn adapter(base_url: &str) -> Arc<dyn LlmPort>;

    /// A well-formed non-streaming success response body containing non-empty content and a
    /// populated usage/token count.
    fn success_body() -> String;

    /// A well-formed streaming success response body that assembles, in wire order, to
    /// `"Hello world"` with a terminal stop signal.
    fn stream_body() -> String;

    /// A well-formed error response body for `status`. Content need only be plausible for the
    /// wire shape -- none of this suite's cases depend on specific error-body text (the
    /// credential-redaction case constructs its own body independently; see
    /// [`cases::credential_never_appears_in_a_rendered_error`]).
    fn error_body(status: u16) -> String;
}

/// Build the single [`LlmRequest`] every case in this suite sends.
fn conformance_request() -> LlmRequest {
    LlmRequest::new(
        "conformance-test-model",
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .expect("a plain user prompt always constructs"),
    )
}

/// The fixed case list every [`ConformanceFixture`] is measured against (D-31).
///
/// Each case is an ordinary generic async function rather than inline macro-generated logic, so
/// it is debuggable and testable like any other Rust code; [`llm_conformance_suite!`] is a thin
/// dispatcher that wires these into named `#[tokio::test]` functions per fixture.
pub mod cases {
    use super::*;

    /// Case: a success body yields non-empty content and a populated, internally-consistent
    /// usage/token count.
    pub async fn generate_and_usage_extraction<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(F::success_body())
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let response = adapter
            .generate(conformance_request())
            .await
            .expect("success_body() must produce a well-formed response");

        assert!(
            !response.content.is_empty(),
            "generate() must extract non-empty content from success_body()"
        );
        assert!(
            response.usage.total_tokens > 0,
            "generate() must extract a populated usage/token count from success_body()"
        );
        assert_eq!(
            response.usage.total_tokens,
            response.usage.prompt_tokens + response.usage.completion_tokens,
            "total_tokens must equal prompt_tokens + completion_tokens"
        );
    }

    /// Case: a scripted stream body assembles chunks in wire order and terminates on the wire's
    /// stop signal.
    pub async fn streaming_assembles_in_wire_order_with_a_terminal_stop<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(F::stream_body())
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let stream = adapter
            .generate_stream(conformance_request())
            .await
            .expect("stream_body() must open successfully");
        let mut stream = Box::into_pin(stream);

        let mut assembled = String::new();
        let mut last_finish_reason = None;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("stream_body() must assemble with no error");
            assembled.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert_eq!(
            assembled, "Hello world",
            "stream_body() must assemble to \"Hello world\" in wire order"
        );
        assert!(
            last_finish_reason.is_some(),
            "the wire's terminal stop must surface as a finish_reason on the last chunk"
        );
    }

    /// Case: an error status on the connection-opening request surfaces a typed [`LlmError`]
    /// directly -- no stream item is ever produced.
    pub async fn stream_error_before_the_first_chunk<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(500)
            .with_body(F::error_body(500))
            .expect_at_least(1)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate_stream(conformance_request()).await;

        assert!(
            result.is_err(),
            "an error status on the connection-opening request must surface as Err before any \
             stream item, got Ok"
        );
    }

    /// Case: a malformed frame arriving AFTER valid ones surfaces a typed [`LlmError`] without
    /// erasing the content already assembled from the valid frames that preceded it (the
    /// first-chunk preservation rule Phase 25's D-25 established for the fallback chain, applied
    /// here to a single adapter's own stream).
    pub async fn stream_error_after_the_first_chunk<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        // Deliberately malformed JSON appended after a fully valid stream body: every wire
        // parser in this crate looks for `data: `-prefixed lines and calls `serde_json::from_str`
        // on the remainder, so an unparseable trailing line produces a typed parse error
        // regardless of which of the two `Wire` shapes is in play.
        let body = format!("{}data: {{not valid json\n\n", F::stream_body());
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(body)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let stream = adapter
            .generate_stream(conformance_request())
            .await
            .expect("the connection must open successfully before the malformed frame arrives");
        let mut stream = Box::into_pin(stream);

        let mut assembled = String::new();
        let mut saw_error = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => assembled.push_str(&chunk.delta),
                Err(_typed_llm_error) => {
                    saw_error = true;
                    break;
                }
            }
        }

        assert!(
            saw_error,
            "a malformed frame arriving after valid ones must surface as a typed LlmError"
        );
        assert_eq!(
            assembled, "Hello world",
            "content already assembled from valid frames must survive a later malformed frame"
        );
    }

    /// Case: 401/404/400/402 each map to their own dedicated [`LlmError`] variant, asserted by
    /// variant identity -- never by message text.
    pub async fn dedicated_status_mappings<F: ConformanceFixture>() {
        for status in [401u16, 404, 400, 402] {
            let mut server = Server::new_async().await;
            server
                .mock("POST", Matcher::Any)
                .with_status(status as usize)
                .with_body(F::error_body(status))
                .expect_at_least(1)
                .create_async()
                .await;

            let adapter = F::adapter(&server.url());
            let result = adapter.generate(conformance_request()).await;

            match (status, result) {
                (401, Err(LlmError::AuthenticationError(_))) => {}
                (404, Err(LlmError::ModelNotAvailable(_))) => {}
                (400, Err(LlmError::InvalidPrompt(_))) => {}
                (402, Err(LlmError::UsageLimitExceeded { .. })) => {}
                (status, other) => panic!(
                    "status {status}: expected its dedicated LlmError variant, got {other:?}"
                ),
            }
        }
    }

    /// Case: 408/429 and every 5xx map to [`Transience::Transient`] BY VALUE via
    /// `LlmError::transience()`; other 4xx (403, 422) map to [`Transience::Permanent`]. Asserted
    /// on the returned value, never by parsing a rendered string (Phase 25 D-03, FT-FR-01).
    pub async fn transience_by_value<F: ConformanceFixture>() {
        for (status, expected) in [
            (408u16, Transience::Transient),
            (429, Transience::Transient),
            (500, Transience::Transient),
            (502, Transience::Transient),
            (503, Transience::Transient),
            (403, Transience::Permanent),
            (422, Transience::Permanent),
        ] {
            let mut server = Server::new_async().await;
            server
                .mock("POST", Matcher::Any)
                .with_status(status as usize)
                .with_body(F::error_body(status))
                .expect_at_least(1)
                .create_async()
                .await;

            let adapter = F::adapter(&server.url());
            let result = adapter.generate(conformance_request()).await;

            let err = match result {
                Err(e) => e,
                Ok(_) => panic!("status {status} must be an error, got Ok"),
            };
            assert_eq!(
                err.transience(),
                expected,
                "status {status}: expected transience {expected:?}, got {:?} (err: {err:?})",
                err.transience()
            );
        }
    }

    /// Case: with a credential-shaped token positioned to straddle the response-excerpt
    /// truncation boundary, no rendered error -- `Display`, `Debug`, nor a prefix of the token
    /// longer than a few characters -- ever contains it (redact-then-bound,
    /// security.instructions.md; T-26-11).
    ///
    /// Deliberately uses a SYNTHETIC `sk-`-prefixed token rather than each fixture's own
    /// configured credential: `redact_credentials`'s exact-match pass covers the adapter's own
    /// key specifically, but its shape-based `sk-`/`Bearer` passes run regardless of what the
    /// adapter was configured with (defense in depth) -- and a synthetic token sidesteps a real
    /// naming collision in this crate: Ollama's own credential is the fixed, non-secret
    /// placeholder `"ollama"`, which is also the literal provider name every `ProviderError`'s
    /// `Display`/`Debug` legitimately carries, so asserting the WHOLE rendered string never
    /// contains `"ollama"` would fail on the provider name, not a leak.
    pub async fn credential_never_appears_in_a_rendered_error<F: ConformanceFixture>() {
        const SECRET: &str = "sk-conformance-canary-0123456789abcdef";
        let padding = "x".repeat(RESPONSE_EXCERPT_CHAR_BUDGET.saturating_sub(5));
        let body = format!("{padding}{SECRET} trailing text");

        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(500)
            .with_body(body)
            .expect_at_least(1)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let err = adapter
            .generate(conformance_request())
            .await
            .expect_err("a 500 status must be an error");

        let rendered = err.to_string();
        let debugged = format!("{err:?}");
        let leaked_prefix = &SECRET[..8];

        assert!(
            !rendered.contains(SECRET),
            "Display leaked the secret: {rendered}"
        );
        assert!(
            !debugged.contains(SECRET),
            "Debug leaked the secret: {debugged}"
        );
        assert!(
            !rendered.contains(leaked_prefix),
            "Display leaked a boundary-straddling prefix of the secret: {rendered}"
        );
        assert!(
            !debugged.contains(leaked_prefix),
            "Debug leaked a boundary-straddling prefix of the secret: {debugged}"
        );
    }

    /// Case: a redirect to a different host is never followed while carrying a credential header
    /// -- proven by asserting the redirect TARGET is never contacted at all
    /// (security.instructions.md; T-26-46).
    pub async fn redirect_is_not_followed_with_a_credential_header<F: ConformanceFixture>() {
        let mut redirect_target = Server::new_async().await;
        let target_mock = redirect_target
            .mock("POST", Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(302)
            .with_header(
                "location",
                &format!("{}/chat/completions", redirect_target.url()),
            )
            .with_body("redirecting")
            .expect_at_least(1)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(conformance_request()).await;

        assert!(
            result.is_err(),
            "a refused redirect must surface as an error, never a followed 2xx"
        );
        target_mock.assert_async().await;
    }
}

/// Generate one `#[tokio::test]` per fixed case in [`cases`] for a concrete
/// [`ConformanceFixture`], plus a `CASE_COUNT` derived from the SAME identifier list the tests
/// are generated from -- so a case silently dropped from this list shrinks `CASE_COUNT` too,
/// which `suite_generates_the_full_case_list_for_a_fixture` (below) pins against the documented
/// `8`, turning a silently-dropped case into a build/test failure rather than a quiet pass.
#[macro_export]
macro_rules! llm_conformance_suite {
    ($fixture:ty) => {
        $crate::llm_conformance_suite!(@cases $fixture;
            generate_and_usage_extraction,
            streaming_assembles_in_wire_order_with_a_terminal_stop,
            stream_error_before_the_first_chunk,
            stream_error_after_the_first_chunk,
            dedicated_status_mappings,
            transience_by_value,
            credential_never_appears_in_a_rendered_error,
            redirect_is_not_followed_with_a_credential_header,
        );
    };
    (@cases $fixture:ty; $($case:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $case() {
                $crate::conformance::cases::$case::<$fixture>().await;
            }
        )+

        /// The number of cases the invocation above just generated for this fixture --
        /// mechanically derived from the same token list the `#[tokio::test]` functions above
        /// were generated from, never hand-copied.
        #[allow(dead_code)]
        pub const CASE_COUNT: usize = $crate::llm_conformance_suite!(@count $($case),+);
    };
    (@count) => { 0usize };
    (@count $head:ident $(, $tail:ident)*) => {
        1usize + $crate::llm_conformance_suite!(@count $($tail),*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::platform::container::token_usage::TokenUsage;
    use paladin_ports::output::llm_port::{
        FinishReason, LlmResponse, ProviderCapabilities, StreamingResponse,
    };
    use serde::Deserialize;
    use serde_json::json;

    // ── A minimal, fully self-contained fixture used ONLY to prove the macro itself is
    // correct (below) -- not built on any feature-gated adapter or `CompatEngine`, so this
    // compile/behavior-safety check runs under every feature combination `paladin-llm` is ever
    // tested with, not only `--all-features`. It routes its own non-2xx branch through the same
    // `crate::http_status::map_http_status` every real adapter uses, so the 8 generated cases
    // exercise real, shared production logic rather than a second, parallel implementation.

    struct TrivialAdapter {
        client: reqwest::Client,
        base_url: String,
    }

    #[derive(Deserialize)]
    struct TrivialSuccessBody {
        content: String,
        usage: TrivialUsage,
    }

    #[derive(Deserialize)]
    struct TrivialUsage {
        prompt_tokens: u32,
        completion_tokens: u32,
    }

    #[derive(Deserialize)]
    struct TrivialStreamFrame {
        #[serde(default)]
        delta: String,
        #[serde(default)]
        done: bool,
    }

    const TRIVIAL_PROVIDER: &str = "trivial-fixture";
    const TRIVIAL_API_KEY: &str = "trivial-canary-key";

    #[async_trait]
    impl LlmPort for TrivialAdapter {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            let response = self
                .client
                .post(format!("{}/generate", self.base_url))
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            if status >= 300 {
                return Err(crate::http_status::map_http_status(
                    TRIVIAL_PROVIDER,
                    status,
                    &body,
                    TRIVIAL_API_KEY,
                ));
            }

            let parsed: TrivialSuccessBody = serde_json::from_str(&body)
                .map_err(|e| LlmError::ProcessingError(e.to_string()))?;

            Ok(LlmResponse {
                id: uuid::Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
                content: parsed.content,
                finish_reason: FinishReason::Stop,
                usage: TokenUsage::new(parsed.usage.prompt_tokens, parsed.usage.completion_tokens),
                created_at: chrono::Utc::now(),
                metadata: Default::default(),
                function_call: None,
            })
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            let response = self
                .client
                .post(format!("{}/generate", self.base_url))
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;
            let status = response.status().as_u16();

            if status >= 300 {
                let body = response
                    .text()
                    .await
                    .map_err(|e| LlmError::NetworkError(e.to_string()))?;
                return Err(crate::http_status::map_http_status(
                    TRIVIAL_PROVIDER,
                    status,
                    &body,
                    TRIVIAL_API_KEY,
                ));
            }

            let stream = response.bytes_stream().flat_map(|chunk| {
                let items: Vec<Result<StreamingResponse, LlmError>> = match chunk {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        let mut items = Vec::new();
                        for line in text.lines() {
                            let Some(json_str) = line.strip_prefix("data: ") else {
                                continue;
                            };
                            match serde_json::from_str::<TrivialStreamFrame>(json_str) {
                                Ok(frame) => items.push(Ok(StreamingResponse {
                                    id: uuid::Uuid::new_v4(),
                                    delta: frame.delta,
                                    finish_reason: if frame.done {
                                        Some(FinishReason::Stop)
                                    } else {
                                        None
                                    },
                                })),
                                Err(e) => items.push(Err(LlmError::ProcessingError(e.to_string()))),
                            }
                        }
                        items
                    }
                    Err(e) => vec![Err(LlmError::NetworkError(e.to_string()))],
                };
                futures::stream::iter(items)
            });

            Ok(Box::new(stream))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["trivial-model".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            TRIVIAL_PROVIDER
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    struct TrivialFixture;

    impl ConformanceFixture for TrivialFixture {
        const WIRE: Wire = Wire::OpenAiChat;

        fn adapter(base_url: &str) -> Arc<dyn LlmPort> {
            Arc::new(TrivialAdapter {
                client: reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .expect("reqwest client must build"),
                base_url: base_url.to_string(),
            })
        }

        fn success_body() -> String {
            json!({
                "content": "Hi there",
                "usage": {"prompt_tokens": 1, "completion_tokens": 1}
            })
            .to_string()
        }

        fn stream_body() -> String {
            concat!(
                "data: {\"delta\":\"Hel\",\"done\":false}\n\n",
                "data: {\"delta\":\"lo \",\"done\":false}\n\n",
                "data: {\"delta\":\"world\",\"done\":true}\n\n",
            )
            .to_string()
        }

        fn error_body(status: u16) -> String {
            format!("trivial error body for status {status}")
        }
    }

    crate::llm_conformance_suite!(TrivialFixture);

    /// Test 1: instantiating `llm_conformance_suite!` for a trivial in-file fixture produces one
    /// test per fixed case, and the generated module's test count equals the documented case
    /// count -- so a case silently dropped from the macro's list fails THIS build rather than
    /// passing quietly (the macro's `CASE_COUNT` and its `#[tokio::test]` functions are generated
    /// from the identical token list above, see `llm_conformance_suite!`'s own doc comment).
    #[test]
    fn suite_generates_the_full_case_list_for_a_fixture() {
        assert_eq!(
            CASE_COUNT, 8,
            "llm_conformance_suite! must generate exactly the documented 8 fixed cases -- if this \
             fails, a case was added to or removed from the macro's list without updating this \
             pinned expectation"
        );
    }
}
