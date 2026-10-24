//! Test-only mock-endpoint plumbing shared by every adapter's wire tests: a mock server, a
//! capturing mock for the JSON body a request sends, and helpers that drive one request or count
//! the attempts made against a failing endpoint. Compiled only under `cfg(test)` and not gated
//! on any adapter feature, so any one adapter's tests can use it alone.

// Each adapter's tests use a different subset of these helpers, and a single-feature test build
// compiles only that adapter's tests, so any helper may be unused in some feature set.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use mockito::{Server, ServerGuard};
use serde_json::{Value, json};

/// Register a `/chat/completions` mock on `server` that records the JSON body of the request
/// it receives and answers with a minimal well-formed completion. Returns the capture slot.
pub(crate) async fn mock_completion(server: &mut ServerGuard) -> Arc<Mutex<Option<Value>>> {
    mock_capture(
        server,
        "/chat/completions",
        json!({
            "id": "cmpl-1",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }),
    )
    .await
}

/// Register a `POST path` mock on `server` that records the JSON body of the request it
/// receives and answers HTTP 200 with `response`. Returns the capture slot.
///
/// The shape every wire's knob test uses: the compat presets and OpenAI/DeepSeek answer at
/// `/chat/completions`, Anthropic at `/messages`, Gemini at `/models/<model>:generateContent`.
pub(crate) async fn mock_capture(
    server: &mut ServerGuard,
    path: &str,
    response: Value,
) -> Arc<Mutex<Option<Value>>> {
    let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&captured);
    let body = response.to_string().into_bytes();
    server
        .mock("POST", path)
        .with_status(200)
        .with_body_from_request(move |req| {
            let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
            *slot.lock().unwrap() = serde_json::from_str(&text).ok();
            body.clone()
        })
        .create_async()
        .await;
    captured
}

/// A fresh mock server.
pub(crate) async fn server() -> ServerGuard {
    Server::new_async().await
}

/// Send one plain user request for `model` through `port`, with `max_tokens` as the
/// request's own value (every other sampling parameter unset), and return the JSON body the
/// mock endpoint captured in `captured`.
///
/// # Errors
///
/// Whatever `port.generate` returns, so a test can assert a refusal before any HTTP call.
pub(crate) async fn send_with_max_tokens(
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    captured: &Arc<Mutex<Option<Value>>>,
    model: &str,
    max_tokens: Option<u32>,
) -> Result<Value, paladin_ports::output::llm_port::LlmError> {
    use paladin_core::platform::container::prompt::{
        PromptItem, PromptParameters, PromptType, UserPrompt,
    };
    use paladin_ports::output::llm_port::LlmRequest;

    let mut request = LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    );
    request.prompt.entity.data.parameters = PromptParameters {
        max_tokens,
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop_sequences: None,
    };
    *captured.lock().unwrap() = None;
    port.generate(request).await?;
    Ok(captured
        .lock()
        .unwrap()
        .take()
        .expect("the mock endpoint received a request"))
}

/// A plain one-line user request for `model`, with no effort and no sampling parameter set
/// (Phase 51: the base the reasoning-effort tests add `with_reasoning` to).
pub(crate) fn plain_request(model: &str) -> paladin_ports::output::llm_port::LlmRequest {
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

    paladin_ports::output::llm_port::LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    )
}

/// Send `request` through `port` and return the JSON body the mock endpoint captured in
/// `captured` together with the response (`None` for the body when no HTTP call was made).
///
/// # Errors
///
/// Whatever `port.generate` returns, so a test can assert a refusal before any HTTP call.
pub(crate) async fn send_capturing(
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    captured: &Arc<Mutex<Option<Value>>>,
    request: paladin_ports::output::llm_port::LlmRequest,
) -> Result<
    (Value, paladin_ports::output::llm_port::LlmResponse),
    paladin_ports::output::llm_port::LlmError,
> {
    *captured.lock().unwrap() = None;
    let response = port.generate(request).await?;
    let body = captured
        .lock()
        .unwrap()
        .take()
        .expect("the mock endpoint received a request");
    Ok((body, response))
}

/// How many times `port` hits a mock endpoint that answers every request with HTTP 500,
/// for one `generate` call. Asserts nothing about the error beyond it being one.
pub(crate) async fn attempts_against_a_failing_endpoint(
    server: ServerGuard,
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    model: &str,
    expected_attempts: usize,
) {
    attempts_against_a_failing_endpoint_at(
        server,
        "/chat/completions",
        port,
        model,
        expected_attempts,
    )
    .await;
}

/// [`attempts_against_a_failing_endpoint`] for a wire that answers at `path`.
pub(crate) async fn attempts_against_a_failing_endpoint_at(
    mut server: ServerGuard,
    path: &str,
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    model: &str,
    expected_attempts: usize,
) {
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::LlmRequest;

    let mock = server
        .mock("POST", path)
        .with_status(500)
        .with_body(r#"{"error":"boom"}"#)
        .expect(expected_attempts)
        .create_async()
        .await;
    let request = LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    );
    assert!(port.generate(request).await.is_err());
    mock.assert_async().await;
}

/// What a [`CheckVerdictPort`] was asked: the `(model, effort, fallback)` of one
/// `check_reasoning` call.
pub(crate) type CheckAsked = (
    String,
    Option<paladin_ports::output::llm_port::ReasoningEffort>,
    Option<paladin_ports::output::llm_port::ReasoningFallback>,
);

/// A minimal [`LlmPort`](paladin_ports::output::llm_port::LlmPort) whose `check_reasoning`
/// returns a scripted verdict and records what it was asked (Phase 51, D-09), so a decorator
/// test can prove the question and the answer pass through unchanged. It reports no model
/// profile and cannot generate or stream, so a decorator that fell back to the trait default
/// would not reproduce its verdict.
pub(crate) struct CheckVerdictPort {
    refusal: Option<String>,
    asked: Mutex<Vec<CheckAsked>>,
}

impl CheckVerdictPort {
    /// A port that refuses every check with an `unsupported_parameter` error carrying `reason`.
    pub(crate) fn refusing(reason: &str) -> Self {
        Self {
            refusal: Some(reason.to_string()),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// A port that admits every check.
    pub(crate) fn admitting() -> Self {
        Self {
            refusal: None,
            asked: Mutex::new(Vec::new()),
        }
    }

    /// The `(model, effort, fallback)` of every `check_reasoning` call, in order.
    pub(crate) fn asked(&self) -> Vec<CheckAsked> {
        self.asked
            .lock()
            .map(|asked| asked.clone())
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl paladin_ports::output::llm_port::LlmPort for CheckVerdictPort {
    async fn generate(
        &self,
        _request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        paladin_ports::output::llm_port::LlmResponse,
        paladin_ports::output::llm_port::LlmError,
    > {
        Err(paladin_ports::output::llm_port::LlmError::ProcessingError(
            "CheckVerdictPort does not generate".to_string(),
        ))
    }

    async fn generate_stream(
        &self,
        _request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<
                        paladin_ports::output::llm_port::StreamingResponse,
                        paladin_ports::output::llm_port::LlmError,
                    >,
                > + Send,
        >,
        paladin_ports::output::llm_port::LlmError,
    > {
        Err(paladin_ports::output::llm_port::LlmError::ProcessingError(
            "CheckVerdictPort does not stream".to_string(),
        ))
    }

    async fn validate_model(
        &self,
        _model: &str,
    ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
        Ok(true)
    }

    async fn get_available_models(
        &self,
    ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
        Ok(Vec::new())
    }

    fn get_provider_name(&self) -> &'static str {
        "check-stub"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }

    fn check_reasoning(
        &self,
        model: &str,
        effort: Option<paladin_ports::output::llm_port::ReasoningEffort>,
        fallback: Option<paladin_ports::output::llm_port::ReasoningFallback>,
    ) -> Result<(), paladin_ports::output::llm_port::LlmError> {
        if let Ok(mut asked) = self.asked.lock() {
            asked.push((model.to_string(), effort, fallback));
        }
        match &self.refusal {
            Some(reason) => Err(
                paladin_ports::output::llm_port::LlmError::unsupported_parameter(
                    "reasoning_effort",
                    effort.map(|e| e.to_string()).unwrap_or_default(),
                    "check-stub",
                    model,
                    reason.as_str(),
                ),
            ),
            None => Ok(()),
        }
    }
}

/// Send `effort` / `fallback` through `port`'s `generate` and ask its `check_reasoning` the same
/// question (Phase 51, D-09). Asserts the two paths agree on both the verdict and the refusal
/// reason text, and returns that reason (`None` when the effort is admitted).
///
/// # Panics
///
/// When `generate` and `check_reasoning` disagree, or either fails other than with
/// `UnsupportedParameter`.
pub(crate) async fn agreed_verdict(
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    model: &str,
    effort: Option<paladin_ports::output::llm_port::ReasoningEffort>,
    fallback: Option<paladin_ports::output::llm_port::ReasoningFallback>,
) -> Option<String> {
    use paladin_ports::output::llm_port::LlmError;

    let mut request = plain_request(model);
    if let Some(effort) = effort {
        request = request.with_reasoning(effort);
    }
    if let Some(fallback) = fallback {
        request = request.with_reasoning_fallback(fallback);
    }
    let sent = port.generate(request).await;
    let checked = port.check_reasoning(model, effort, fallback);
    match (sent, checked) {
        (Ok(_), Ok(())) => None,
        (
            Err(LlmError::UnsupportedParameter {
                reason: sent_reason,
                ..
            }),
            Err(LlmError::UnsupportedParameter {
                reason: checked_reason,
                ..
            }),
        ) => {
            assert_eq!(sent_reason, checked_reason);
            Some(checked_reason)
        }
        other => panic!("generate and check disagree: {other:?}"),
    }
}
