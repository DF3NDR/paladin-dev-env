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
//! Plan 31-04 (D-19) adds a ninth case, `streaming_usage_equals_non_streaming_usage`, proving the
//! terminal-chunk streaming usage contract (D-14) holds for every adapter: the streamed usage on
//! the finish-reason-bearing chunk equals the non-streaming `LlmResponse.usage` field-for-field.
//!
//! Phase 43 (D-02, PACE-01) adds a tenth case, `rate_limit_is_surfaced_once_with_its_retry_delay`:
//! a mocked `429` carrying `Retry-After: 7` is observed by the provider exactly once, is
//! `RateLimitExceeded`, classifies `Transient`, and carries `Some(7s)` -- so no adapter in the
//! suite retries a rate limit below the Cadence decorator, which owns waiting.
//!
//! Phase 50 (plan 50-15, PARITY-07) puts Anthropic in the suite -- all nine adapters now answer
//! the identical list -- and adds seven cases for the behaviours that phase made uniform:
//! `capability_declarations_match_the_wire` (the default model is profiled, the declared window
//! matches, an admitted parameter reaches the wire and a `NotSent` one never does, and
//! `Pricing(Cadence(adapter))` returns the same profile), `response_format_reaches_the_wire`,
//! `empty_stop_completion_is_empty_completion` and `empty_stop_stream_is_empty_completion`
//! (D-11a, buffered and streamed), `refusal_is_typed_and_permanent` (D-09),
//! `usage_fields_are_mapped` and `unsupported_parameters_are_omitted_and_recorded`. A later review
//! (CR-02) added `stream_frames_split_across_reads_are_reassembled`, which serves the fixture's
//! stream body in writes cut inside a frame. Plan 50-23 (G-50-5a) tightened
//! `usage_fields_are_mapped` (sub-counts stay inside their parents, and an agreeing body records
//! no `paladin.usage_unreconciled`) and added `vendor_total_disagreement_is_recorded`, which raises
//! the fixture's own vendor total by 1000 and asserts the mapped usage is unchanged (D-02) and the
//! disagreement recorded. Plan 50-27 (G-50-5a) added `billed_failures_carry_their_usage`: an
//! empty completion (buffered and streamed) and a typed refusal carry the usage their body
//! reported on the error ([`LlmError::billed_usage`]), bringing the suite to twenty cases. Phase 51 (plan 51-01, D-00i) added
//! `unset_reasoning_leaves_the_wire_body_unchanged`, which compares each adapter's raw request
//! body for an unset reasoning effort with a golden file, for twenty-one. Plan 51-12 added three
//! more, for twenty-four: `explicit_reasoning_on_an_unprofiled_model_refuses_before_http`
//! (REASON-02), `seeded_effort_levels_reach_the_wire` (REASON-03) and
//! `every_seeded_effort_level_round_trips_to_its_wire_body` (the assumption-delta invariant), the
//! last two driven by [`ConformanceFixture::effort_wire_value`]. A
//! vendor that
//! lacks a feature never skips a case: it states its documented alternative shape through the
//! fixture ([`RefusalShape::ContentFilterOnly`], [`MalformedFrame::SkippedWithWarning`]), so every
//! adapter is measured against every case. The default bodies for these cases live in
//! [`wire_defaults`], keyed by [`Wire`]; a fixture overrides one only where its vendor differs.
//!
//! `#[cfg(test)]`-only: [`ConformanceFixture`] and [`llm_conformance_suite!`] never ship in a
//! release build.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use mockito::{Matcher, Server};
use paladin_core::platform::container::prompt::PromptParameters;
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::llm_port::{
    FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ParamSupport,
    ReasoningEffort, ResponseFormat, SamplingParam, StopSupport,
};
use paladin_ports::output::model_profile::EffortSupport;
use serde_json::Value;

use crate::redaction::RESPONSE_EXCERPT_CHAR_BUDGET;
use crate::sampling::{PARAMS_OMITTED_METADATA_KEY, RESPONSE_FORMAT_ADJUSTED_METADATA_KEY};
use crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY;

/// Which response-wire shape a [`ConformanceFixture`]'s adapter speaks.
///
/// Three shapes exist in this crate: the OpenAI-style `choices[].message.content` /
/// `choices[].delta.content` framing every `CompatEngine`-based preset (and the OpenAI and
/// DeepSeek adapters) speaks, terminated by a `data: [DONE]` sentinel; Gemini's bespoke
/// `candidates[].content.parts[].text` framing, which has no `[DONE]` sentinel (the stream simply
/// ends when the body ends); and Anthropic's Messages API (`content[]` blocks; an SSE stream of
/// `message_start`, `content_block_delta`, `message_delta` and `message_stop` events).
///
/// Phase 50 plan 50-15: the cases branch on it for the bodies a fixture does not override (the
/// empty-completion, refusal and usage bodies, and where a parameter sits on the wire), so a
/// fixture only overrides a body when its vendor genuinely differs from its wire's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// A variant is constructed only by a fixture of that wire, and each adapter's fixture is compiled
// only with its own feature, so a single-feature test build leaves the others unconstructed.
#[allow(dead_code)]
pub enum Wire {
    /// OpenAI-compatible `choices[]` framing (openai, deepseek, openai_compatible, ollama, and
    /// every other `CompatEngine`-based preset).
    OpenAiChat,
    /// Gemini's bespoke `candidates[]` framing.
    Gemini,
    /// Anthropic's Messages API framing (`content[]` blocks, `event:`/`data:` SSE events ending
    /// at `message_stop`).
    Anthropic,
}

/// How a fixture's vendor signals that the model declined to answer (plan 50-15, D-09).
///
/// Every fixture is measured by `refusal_is_typed_and_permanent`; a vendor with no refusal signal
/// does not skip the case, it states the documented alternative shape. Each variant carries a
/// complete non-streaming response body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalShape {
    /// The vendor has a typed refusal signal (OpenAI-chat `message.refusal`, Anthropic
    /// `stop_reason: refusal`): the adapter must return [`LlmError::Refusal`].
    Typed(String),
    /// The vendor has no refusal signal, only a content-filter finish (Gemini `SAFETY`): the
    /// adapter must return `Ok` with `FinishReason::ContentFilter` and no text.
    ContentFilterOnly(String),
}

/// What a fixture's adapter does with an unparsable `data:` frame that arrives after valid ones
/// (plan 50-15; asserted by `stream_error_after_the_first_chunk`).
///
/// Never a reason to skip the case: both variants are asserted, each against the content the valid
/// frames carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Only the Anthropic fixture (feature `anthropic`) constructs `SkippedWithWarning`.
#[allow(dead_code)]
pub enum MalformedFrame {
    /// The frame surfaces as a typed [`LlmError`] on the stream (every adapter but Anthropic).
    Error,
    /// The frame is logged by event type and skipped, and the stream finishes normally
    /// (Anthropic: its streaming reference tells clients to tolerate event types they do not
    /// know, so one unparsable event does not end an otherwise complete response; the adapter
    /// warns instead of skipping silently, plan 50-13).
    SkippedWithWarning,
}

/// The usage figures a fixture's [`ConformanceFixture::usage_body`] must map to.
///
/// `None` in a sub-count means the vendor does not report that field. `total` is
/// `prompt + completion`: `TokenUsage` derives `total_tokens` that way for every adapter (D-02,
/// ADR-0061 era decision, WINDOWS row 74), so the suite asserts the invariant rather than a vendor
/// figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedUsage {
    /// Expected `prompt_tokens` (cache read and write tokens included).
    pub prompt: u32,
    /// Expected `completion_tokens` (reasoning tokens included).
    pub completion: u32,
    /// Expected `cache_read_tokens`.
    pub cache_read: Option<u32>,
    /// Expected `cache_write_tokens`.
    pub cache_write: Option<u32>,
    /// Expected `reasoning_tokens`.
    pub reasoning: Option<u32>,
    /// Expected `total_tokens`; always `prompt + completion`.
    pub total: u32,
}

/// One adapter's real-body conformance fixture (D-31).
///
/// A concrete `Fixture` type is a zero-sized marker implementing this trait, instantiated once
/// per adapter via [`llm_conformance_suite!`]. Every method returns REAL bodies -- copied from
/// (or structurally identical to) that adapter's own pre-existing hand-written tests -- never
/// synthetic ones invented for this suite (26-PATTERNS.md "No Analog Found": "use the per-adapter
/// test modules only as the source of *case content*").
pub trait ConformanceFixture {
    /// The wire shape this fixture's adapter speaks; the default bodies below branch on it.
    const WIRE: Wire;

    /// Build a live adapter instance pointed at `base_url` (a `mockito` server's URL).
    fn adapter(base_url: &str) -> Arc<dyn LlmPort>;

    /// A well-formed non-streaming success response body containing non-empty content and a
    /// populated usage/token count.
    fn success_body() -> String;

    /// A well-formed streaming success response body that assembles, in wire order, to
    /// `"Hello world"` with a terminal stop signal.
    ///
    /// **D-19 (plan 31-04):** the terminal chunk's usage MUST carry the SAME figures
    /// [`Self::success_body`] carries — [`cases::streaming_usage_equals_non_streaming_usage`]
    /// asserts the two are equal field-for-field, so a fixture whose streaming and
    /// non-streaming usage figures diverge fails that case by construction.
    fn stream_body() -> String;

    /// A well-formed error response body for `status`. Content need only be plausible for the
    /// wire shape -- none of this suite's cases depend on specific error-body text (the
    /// credential-redaction case constructs its own body independently; see
    /// [`cases::credential_never_appears_in_a_rendered_error`]).
    fn error_body(status: u16) -> String;

    /// The model the adapter built by [`Self::adapter`] defaults to. The capability, structured
    /// output and parameter-policy cases send it, so they measure the profile the adapter really
    /// resolves for its own default model (a seeded entry, or the adapter-wide fallback).
    fn default_model() -> &'static str;

    /// The provider name the adapter reports (`get_provider_name`) and that a typed
    /// [`LlmError::Refusal`] must carry. Declared by the fixture rather than read back from the
    /// adapter, so a renamed or mistyped provider fails the refusal case.
    fn provider_name() -> &'static str;

    /// What the adapter does with an unparsable frame after valid ones; see [`MalformedFrame`].
    fn malformed_frame() -> MalformedFrame {
        MalformedFrame::Error
    }

    /// The API key the adapter built by [`Self::adapter`] is configured with. The refusal case
    /// plants it in the vendor's refusal text and asserts it never reaches the error's `detail`,
    /// so a fixture whose adapter uses a different key must override this (Ollama's fixed
    /// placeholder), or the case fails rather than passing vacuously.
    fn api_key() -> &'static str {
        "test-key"
    }

    /// A non-streaming body whose model finished `stop`-equivalently with no text at all. The
    /// adapter must answer [`LlmError::EmptyCompletion`] (D-11a), never an `Ok` empty string.
    fn empty_stop_body() -> String {
        wire_defaults::empty_stop_body(Self::WIRE)
    }

    /// The wire's normal stream framing with zero text deltas and a `stop`-equivalent terminal
    /// (`[DONE]`-terminated chat chunks; Anthropic `message_stop` after `end_turn`; Gemini's last
    /// chunk with `STOP`). `generate_stream` must end with [`LlmError::EmptyCompletion`] (D-11a).
    fn empty_stop_stream_body() -> String {
        wire_defaults::empty_stop_stream_body(Self::WIRE)
    }

    /// How this vendor signals a refusal, as a complete non-streaming body. `secret` is the
    /// adapter's own API key ([`Self::api_key`]); it is planted in the vendor's refusal text so
    /// the case can prove the redaction reaches `detail`.
    fn refusal_shape(secret: &str) -> RefusalShape {
        wire_defaults::refusal_shape(Self::WIRE, secret)
    }

    /// A non-streaming success body whose usage object reports every sub-count this vendor
    /// reports (cache read, cache write, reasoning), with figures that differ from each other so
    /// a field mapped from the wrong source is caught.
    fn usage_body() -> String {
        wire_defaults::usage_body(Self::WIRE)
    }

    /// What [`Self::usage_body`] must map to; `None` for a field the vendor does not report.
    fn expected_usage() -> ExpectedUsage {
        wire_defaults::expected_usage(Self::WIRE)
    }

    /// [`Self::usage_body`] with the vendor's own reported total raised by 1000, so it no longer
    /// agrees with the mapped `prompt + completion` (plan 50-23, G-50-5a); `None` when the wire
    /// reports no total (Anthropic). The default edits the fixture's own usage body -- a chat
    /// `usage.total_tokens` or a Gemini `usageMetadata.totalTokenCount` -- so no fixture needs
    /// to override it.
    fn disagreeing_usage_body() -> Option<String> {
        wire_defaults::disagreeing_usage_body(&Self::usage_body())
    }

    /// The effort level the adapter put on the wire in `body` (a request body this fixture's
    /// adapter sent), in Paladin's lowercase spelling, or `None` when the body carries no level.
    ///
    /// Required, with no default: each fixture states where its adapter puts a level
    /// (`reasoning_effort`, `thinking.effort`, `output_config.effort`,
    /// `generationConfig.thinkingConfig.thinkingLevel`), so an adapter that gains a wire form
    /// cannot join the suite without saying where it is. `None` is the answer for a wire with no
    /// level field (Qwen's `enable_thinking`, a self-hosted adapter whose wire is `Unsupported`).
    /// Used by [`cases::seeded_effort_levels_reach_the_wire`] and
    /// [`cases::every_seeded_effort_level_round_trips_to_its_wire_body`] (Phase 51, plan 51-12).
    fn effort_wire_value(body: &serde_json::Value) -> Option<String>;
}

/// [`ConformanceFixture::effort_wire_value`] for a chat-completions body that carries the level
/// at the top-level `reasoning_effort` (OpenAI, DeepSeek, Grok), else `None`.
pub fn chat_effort_wire_value(body: &Value) -> Option<String> {
    body.get("reasoning_effort")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// [`ConformanceFixture::effort_wire_value`] for a chat-completions body that carries the level
/// inside the `thinking` object (Kimi's `thinking.effort`), else `None`.
///
/// Gated on the three adapters whose fixtures call it, so a default-feature test build carries no
/// unused helper.
#[cfg(any(feature = "kimi", feature = "ollama", feature = "openai-compatible"))]
pub fn thinking_effort_wire_value(body: &Value) -> Option<String> {
    body.get("thinking")
        .and_then(|thinking| thinking.get("effort"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Wire-default response bodies for the cases added in Phase 50 plan 50-15, shaped from the
/// adapters' own tests (real vendor shapes, not invented ones). A fixture overrides a body only
/// where its vendor genuinely differs from its wire's default.
pub mod wire_defaults {
    use super::{ExpectedUsage, RefusalShape, Wire};
    use serde_json::json;

    /// A chat-completions body with `message` and `finish_reason`.
    fn chat_body(message: serde_json::Value, finish_reason: &str) -> String {
        json!({
            "id": "cmpl-1",
            "model": "conformance-model",
            "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 0, "total_tokens": 5}
        })
        .to_string()
    }

    /// An Anthropic Messages body.
    fn anthropic_body(
        content: serde_json::Value,
        stop_reason: &str,
        extra: serde_json::Value,
    ) -> String {
        let mut body = json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "conformance-model",
            "content": content,
            "stop_reason": stop_reason,
            "stop_sequence": null,
            "usage": {"input_tokens": 5, "output_tokens": 0}
        });
        if let (Some(target), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        body.to_string()
    }

    /// Gemini's `usageMetadata` for the empty and refusal bodies.
    fn gemini_usage() -> serde_json::Value {
        json!({"promptTokenCount": 5, "candidatesTokenCount": 0, "totalTokenCount": 5})
    }

    /// See [`super::ConformanceFixture::empty_stop_body`].
    pub fn empty_stop_body(wire: Wire) -> String {
        match wire {
            Wire::OpenAiChat => chat_body(json!({"role": "assistant", "content": ""}), "stop"),
            Wire::Anthropic => anthropic_body(json!([]), "end_turn", json!({})),
            Wire::Gemini => json!({
                "candidates": [{
                    "content": {"role": "model", "parts": [{"text": ""}]},
                    "finishReason": "STOP"
                }],
                "usageMetadata": gemini_usage()
            })
            .to_string(),
        }
    }

    /// See [`super::ConformanceFixture::empty_stop_stream_body`].
    pub fn empty_stop_stream_body(wire: Wire) -> String {
        match wire {
            Wire::OpenAiChat => concat!(
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                "data: {\"id\":\"1\",\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":0,\"total_tokens\":5}}\n\n",
                "data: [DONE]\n\n",
            )
            .to_string(),
            Wire::Anthropic => concat!(
                "event: message_start\n",
                "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"conformance-model\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":5,\"output_tokens\":1}}}\n\n",
                "event: message_delta\n",
                "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":0}}\n\n",
                "event: message_stop\n",
                "data: {\"type\":\"message_stop\"}\n\n",
            )
            .to_string(),
            Wire::Gemini => concat!(
                "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"\"}],\"role\":\"model\"},",
                "\"finishReason\":\"STOP\"}],\"usageMetadata\":{\"promptTokenCount\":5,",
                "\"candidatesTokenCount\":0,\"totalTokenCount\":5}}\n\n",
            )
            .to_string(),
        }
    }

    /// See [`super::ConformanceFixture::refusal_shape`].
    pub fn refusal_shape(wire: Wire, secret: &str) -> RefusalShape {
        match wire {
            Wire::OpenAiChat => RefusalShape::Typed(chat_body(
                json!({
                    "role": "assistant",
                    "content": null,
                    "refusal": format!("I can't help with that request (ref {secret}).")
                }),
                "stop",
            )),
            Wire::Anthropic => RefusalShape::Typed(anthropic_body(
                json!([]),
                "refusal",
                json!({"stop_details": {
                    "type": "refusal",
                    "category": "cyber",
                    "explanation": format!("This request seeks malware (ref {secret}).")
                }}),
            )),
            Wire::Gemini => RefusalShape::ContentFilterOnly(
                json!({
                    "candidates": [{
                        "content": {"role": "model", "parts": []},
                        "finishReason": "SAFETY"
                    }],
                    "usageMetadata": gemini_usage()
                })
                .to_string(),
            ),
        }
    }

    /// An OpenAI-chat body whose only signal is `finish_reason: content_filter` with no text: the
    /// shape for an OpenAI-chat vendor that has no refusal field (DeepSeek) to hand to
    /// [`RefusalShape::ContentFilterOnly`].
    #[allow(dead_code)] // used only by the DeepSeek fixture (feature `deepseek`)
    pub fn chat_content_filter_body() -> String {
        chat_body(
            json!({"role": "assistant", "content": ""}),
            "content_filter",
        )
    }

    /// See [`super::ConformanceFixture::usage_body`].
    pub fn usage_body(wire: Wire) -> String {
        match wire {
            // The OpenAI-chat usage object: cached tokens and reasoning tokens in the two
            // `*_details` objects. No cache-write figure: OpenAI does not report one.
            Wire::OpenAiChat => json!({
                "id": "cmpl-1",
                "model": "conformance-model",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "Hi there"},
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 40,
                    "total_tokens": 140,
                    "prompt_tokens_details": {"cached_tokens": 30},
                    "completion_tokens_details": {"reasoning_tokens": 15}
                }
            })
            .to_string(),
            // `input_tokens` excludes the two cache counts, so the adapter's prompt is
            // 50 + 30 + 20 = 100.
            Wire::Anthropic => json!({
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "model": "conformance-model",
                "content": [{"type": "text", "text": "Hi there"}],
                "stop_reason": "end_turn",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 50,
                    "cache_creation_input_tokens": 20,
                    "cache_read_input_tokens": 30,
                    "output_tokens": 40,
                    "output_tokens_details": {"thinking_tokens": 15}
                }
            })
            .to_string(),
            // `candidatesTokenCount` excludes thoughts, so the adapter's completion is
            // 25 + 15 = 40; `promptTokenCount` excludes the tool-use prompt tokens, so the
            // adapter's prompt is 100 + 50 = 150 (plan 50-23, G-50-5a), and the vendor total
            // 190 agrees with the mapped one. Gemini reports no cache-write figure.
            Wire::Gemini => json!({
                "candidates": [{
                    "content": {"role": "model", "parts": [{"text": "Hi there"}]},
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 100,
                    "candidatesTokenCount": 25,
                    "toolUsePromptTokenCount": 50,
                    "cachedContentTokenCount": 30,
                    "thoughtsTokenCount": 15,
                    "totalTokenCount": 190
                }
            })
            .to_string(),
        }
    }

    /// How far [`disagreeing_usage_body`] raises a body's vendor total.
    pub const DISAGREEMENT: u32 = 1000;

    /// See [`super::ConformanceFixture::disagreeing_usage_body`]: `usage_body` with its vendor
    /// total (`usage.total_tokens` or `usageMetadata.totalTokenCount`) raised by
    /// [`DISAGREEMENT`], or `None` when it carries neither.
    pub fn disagreeing_usage_body(usage_body: &str) -> Option<String> {
        let mut body: serde_json::Value = serde_json::from_str(usage_body).ok()?;
        for pointer in ["/usage/total_tokens", "/usageMetadata/totalTokenCount"] {
            if let Some(slot) = body.pointer_mut(pointer)
                && let Some(total) = slot.as_u64()
            {
                *slot = json!(total + u64::from(DISAGREEMENT));
                return Some(body.to_string());
            }
        }
        None
    }

    /// See [`super::ConformanceFixture::expected_usage`].
    pub fn expected_usage(wire: Wire) -> ExpectedUsage {
        let (cache_read, cache_write, reasoning) = match wire {
            Wire::OpenAiChat | Wire::Gemini => (Some(30), None, Some(15)),
            Wire::Anthropic => (Some(30), Some(20), Some(15)),
        };
        // Gemini's prompt carries the 50 tool-use prompt tokens its body reports beside
        // `promptTokenCount`.
        let prompt = match wire {
            Wire::Gemini => 150,
            Wire::OpenAiChat | Wire::Anthropic => 100,
        };
        ExpectedUsage {
            prompt,
            completion: 40,
            cache_read,
            cache_write,
            reasoning,
            total: prompt + 40,
        }
    }
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

    /// Case (CR-02): a stream whose frames straddle network reads still assembles in full.
    ///
    /// The fixture's own [`ConformanceFixture::stream_body`] is served in separate writes cut
    /// inside the first and the last `data:` payload, with a pause between writes so each reaches
    /// the adapter as its own read. The adapter must reassemble the split frames: every delta
    /// arrives, no item is an error, and the terminal chunk -- the frame that carries the finish
    /// reason, usage and the Phase 50 terminal guarantees -- is still delivered.
    pub async fn stream_frames_split_across_reads_are_reassembled<F: ConformanceFixture>() {
        let body = F::stream_body().into_bytes();
        let marker = b"data: ";
        let positions: Vec<usize> = body
            .windows(marker.len())
            .enumerate()
            .filter(|(_, window)| *window == marker)
            .map(|(index, _)| index)
            .collect();
        let (Some(first), Some(last)) = (positions.first(), positions.last()) else {
            panic!("stream_body() must contain at least one `data: ` frame");
        };
        // Two bytes past a frame's `data: ` prefix is inside its payload on every wire.
        let mut cuts = vec![first + marker.len() + 2, last + marker.len() + 2];
        cuts.dedup();
        let mut segments: Vec<Vec<u8>> = Vec::new();
        let mut start = 0;
        for cut in cuts {
            segments.push(body[start..cut].to_vec());
            start = cut;
        }
        segments.push(body[start..].to_vec());

        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_chunked_body(move |writer| {
                for (index, segment) in segments.iter().enumerate() {
                    if index > 0 {
                        std::thread::sleep(Duration::from_millis(60));
                    }
                    writer.write_all(segment)?;
                    writer.flush()?;
                }
                Ok(())
            })
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
            let chunk =
                item.expect("a frame split across network reads must not surface as an error");
            assembled.push_str(&chunk.delta);
            if chunk.finish_reason.is_some() {
                last_finish_reason = chunk.finish_reason;
            }
        }

        assert_eq!(
            assembled, "Hello world",
            "a frame split across network reads must be reassembled, not dropped"
        );
        assert!(
            last_finish_reason.is_some(),
            "the terminal frame split across network reads must still surface its finish reason"
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

    /// Case: a malformed frame arriving AFTER valid ones never erases the content already
    /// assembled from the valid frames that preceded it (the first-chunk preservation rule Phase
    /// 25's D-25 established for the fallback chain, applied here to a single adapter's own
    /// stream). What the frame itself becomes is the fixture's [`MalformedFrame`]: a typed
    /// [`LlmError`] (the default) or, for Anthropic, a warned skip with the stream finishing
    /// normally.
    pub async fn stream_error_after_the_first_chunk<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        // Deliberately malformed JSON appended after a fully valid stream body: every wire parser
        // in this crate looks for `data: `-prefixed lines and calls `serde_json::from_str` on the
        // remainder, so an unparseable trailing line is seen by each of them regardless of which
        // `Wire` shape is in play.
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
        let mut saw_terminal = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => {
                    assembled.push_str(&chunk.delta);
                    saw_terminal |= chunk.finish_reason.is_some();
                }
                Err(_typed_llm_error) => {
                    saw_error = true;
                    break;
                }
            }
        }

        match F::malformed_frame() {
            MalformedFrame::Error => assert!(
                saw_error,
                "a malformed frame arriving after valid ones must surface as a typed LlmError"
            ),
            MalformedFrame::SkippedWithWarning => {
                assert!(
                    !saw_error,
                    "a fixture declaring SkippedWithWarning must finish the stream without an error"
                );
                assert!(
                    saw_terminal,
                    "the terminal chunk must still arrive when an unparsable frame is skipped"
                );
            }
        }
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

    /// Case: a `429` carrying `Retry-After: 7` is observed by the provider exactly once, is
    /// [`LlmError::RateLimitExceeded`], classifies [`Transience::Transient`], and carries
    /// `Some(7s)` (Phase 43 D-02 verification clause, PACE-01).
    ///
    /// **Why exactly once.** A 429 is the one error whose cure is waiting, and the Cadence
    /// decorator above every adapter owns waiting with a gate shared by all callers of the same
    /// provider and model. An adapter that re-sent a 429 inside its own retry loop would hit a
    /// limit the provider had just announced and hide that thrash below the decorator, so the
    /// first 429 must be surfaced. The mock's `expect(1)` is asserted at the end -- a second hit
    /// fails the case. [`transience_by_value`]'s 429 row deliberately keeps `expect_at_least(1)`:
    /// that case measures classification, this one measures attempt count.
    ///
    /// **Coverage notes.** Anthropic is instantiated like the rest (plan 50-15); its hand-written
    /// `anthropic_429_is_surfaced_once_with_its_retry_delay_and_four_dimensions` (plan 43-03) stays
    /// because it also pins the four rate-limit dimensions and the no-sleep bound, which this
    /// case does not. The OpenAI fixture is built with `max_retries: 0`, so the "exactly once" half
    /// is vacuous for it here; its once-only proof with `max_retries: 3` is
    /// `openai_429_is_surfaced_on_the_first_attempt` (plan 43-01), and
    /// `openai_429_carries_retry_after_and_ratelimit_dimensions` (plan 43-03) covers the header
    /// carriage. The delay and classification halves still run for it. The Anthropic fixture has
    /// the same `max_retries: 0` shape.
    pub async fn rate_limit_is_surfaced_once_with_its_retry_delay<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .with_status(429)
            .with_header("retry-after", "7")
            .with_body(F::error_body(429))
            .expect(1)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(conformance_request()).await;

        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("a 429 must be an error, got Ok"),
        };
        assert!(
            matches!(err, LlmError::RateLimitExceeded { .. }),
            "a 429 must map to RateLimitExceeded, got {err:?}"
        );
        assert_eq!(
            err.retry_after(),
            Some(Duration::from_secs(7)),
            "the provider's Retry-After must reach the error (err: {err:?})"
        );
        assert_eq!(
            err.transience(),
            Transience::Transient,
            "a rate limit is transient by value (err: {err:?})"
        );
        // The provider must have been hit exactly once: a retry inside the adapter would fail
        // this assertion.
        mock.assert_async().await;
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

    /// Case (D-19, plan 31-04): the streaming path's terminal-chunk usage equals the
    /// non-streaming path's usage, field-for-field, including the three optional sub-counts.
    /// Opens `success_body()` through `generate()` and `stream_body()` through
    /// `generate_stream()` against separate mockito servers, and asserts: exactly one yielded
    /// chunk carries `usage`; that chunk is the SAME one carrying `finish_reason`; and its usage
    /// equals `LlmResponse.usage` from the non-streaming call.
    pub async fn streaming_usage_equals_non_streaming_usage<F: ConformanceFixture>() {
        let mut non_stream_server = Server::new_async().await;
        non_stream_server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(F::success_body())
            .create_async()
            .await;

        let non_streaming_adapter = F::adapter(&non_stream_server.url());
        let non_streaming = non_streaming_adapter
            .generate(conformance_request())
            .await
            .expect("success_body() must produce a well-formed response");

        let mut stream_server = Server::new_async().await;
        stream_server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(F::stream_body())
            .create_async()
            .await;

        let streaming_adapter = F::adapter(&stream_server.url());
        let stream = streaming_adapter
            .generate_stream(conformance_request())
            .await
            .expect("stream_body() must open successfully");
        let mut stream = Box::into_pin(stream);

        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.expect("stream_body() must assemble with no error"));
        }

        let finish_indices: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.finish_reason.is_some())
            .map(|(i, _)| i)
            .collect();
        let usage_indices: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.usage.is_some())
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            usage_indices.len(),
            1,
            "exactly one chunk must carry a usage"
        );
        assert_eq!(
            finish_indices, usage_indices,
            "the finish-reason chunk and the usage chunk must be the SAME chunk"
        );
        assert_eq!(
            chunks[usage_indices[0]].usage,
            Some(non_streaming.usage),
            "streamed terminal usage must equal the non-streaming usage field-for-field, \
             including the three optional sub-counts"
        );
    }

    /// Case: a redirect to a different host is never followed while carrying a credential header
    /// -- proven by asserting the redirect TARGET is never contacted at all
    /// (security.instructions.md; T-26-46).
    ///
    /// **Detection power (plan 50-15, the same repair plan 50-09 made).** The redirect is a `307`
    /// and the target mocks BOTH `POST` and `GET` with `expect(0)`. The earlier shape answered
    /// `302` with a `POST`-only target mock; a client that followed a `302` re-sent the request
    /// as a `GET` (the method change `302` permits), which that mock never matched, so a followed
    /// redirect passed unnoticed. A `307` keeps the method, and the `GET` mock catches a client
    /// that downgrades it anyway.
    pub async fn redirect_is_not_followed_with_a_credential_header<F: ConformanceFixture>() {
        let mut redirect_target = Server::new_async().await;
        let post_mock = redirect_target
            .mock("POST", Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let get_mock = redirect_target
            .mock("GET", Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(307)
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
        post_mock.assert_async().await;
        get_mock.assert_async().await;
    }

    // ── Phase 50 plan 50-15: the behaviours the phase made uniform ──

    /// [`conformance_request`], for `model`.
    fn request_for_model(model: &str) -> LlmRequest {
        let mut request = conformance_request();
        request.model = model.to_string();
        request
    }

    /// Run one buffered request through `F`'s adapter against a mock answering `response_body`,
    /// returning the JSON body the adapter put on the wire and its result.
    async fn send_capturing<F: ConformanceFixture>(
        request: LlmRequest,
        response_body: String,
    ) -> (Value, Result<LlmResponse, LlmError>) {
        use std::sync::Mutex;

        let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&captured);
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                if let Ok(mut guard) = slot.lock() {
                    *guard = serde_json::from_str(&text).ok();
                }
                response_body.clone().into_bytes()
            })
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(request).await;
        let wire_body = captured
            .lock()
            .expect("the capture slot is never poisoned")
            .take()
            .expect("the adapter must have sent a JSON request body");
        (wire_body, result)
    }

    /// [`send_capturing`], but returning the request body as the exact UTF-8 text the adapter
    /// sent, not the parsed JSON -- so a change in the order or spelling of a field is visible
    /// (Phase 51, D-00i).
    async fn send_capturing_raw<F: ConformanceFixture>(
        request: LlmRequest,
        response_body: String,
    ) -> (String, Result<LlmResponse, LlmError>) {
        use std::sync::Mutex;

        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&captured);
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                if let Ok(mut guard) = slot.lock() {
                    *guard = Some(text);
                }
                response_body.clone().into_bytes()
            })
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(request).await;
        let wire_body = captured
            .lock()
            .expect("the capture slot is never poisoned")
            .take()
            .expect("the adapter must have sent a request body");
        (wire_body, result)
    }

    /// Where `param` sits in a request body of `wire`: the OpenAI-chat and Anthropic bodies carry
    /// it at the top level (Anthropic names stop sequences `stop_sequences`, and has no penalty
    /// keys -- a profile that admitted one would therefore fail the capability case), Gemini
    /// nests it in `generationConfig` under camel-case names.
    fn wire_param(body: &Value, wire: Wire, param: SamplingParam) -> Option<&Value> {
        // `SamplingParam` is `#[non_exhaustive]`: a parameter this suite does not yet place on a
        // wire fails loudly instead of reading as "absent".
        let unknown =
            || -> ! { panic!("the conformance suite does not place {param:?} on a wire") };
        match wire {
            Wire::OpenAiChat => body.get(match param {
                SamplingParam::Temperature => "temperature",
                SamplingParam::TopP => "top_p",
                SamplingParam::FrequencyPenalty => "frequency_penalty",
                SamplingParam::PresencePenalty => "presence_penalty",
                SamplingParam::StopSequences => "stop",
                _ => unknown(),
            }),
            Wire::Anthropic => body.get(match param {
                SamplingParam::Temperature => "temperature",
                SamplingParam::TopP => "top_p",
                SamplingParam::FrequencyPenalty => "frequency_penalty",
                SamplingParam::PresencePenalty => "presence_penalty",
                SamplingParam::StopSequences => "stop_sequences",
                _ => unknown(),
            }),
            Wire::Gemini => body.get("generationConfig")?.get(match param {
                SamplingParam::Temperature => "temperature",
                SamplingParam::TopP => "topP",
                SamplingParam::FrequencyPenalty => "frequencyPenalty",
                SamplingParam::PresencePenalty => "presencePenalty",
                SamplingParam::StopSequences => "stopSequences",
                _ => unknown(),
            }),
        }
    }

    /// A value `support` admits (the midpoint of a range, the one fixed value), or `None` when it
    /// admits nothing.
    fn admitted_value(support: ParamSupport) -> Option<f32> {
        let value = match support {
            ParamSupport::Range(lo, hi) => lo + (hi - lo) / 2.0,
            ParamSupport::Fixed(x) => x,
            ParamSupport::NotSent => return None,
        };
        support.admits(value).then_some(value)
    }

    /// Case (assumption-delta invariant, plan 50-15): what an adapter DECLARES about its default
    /// model is what its wire DOES.
    ///
    /// - `model_profile(default)` is `Some` (an adapter that falls back to `temperature_range`
    ///   alone, with no profile at all, fails here);
    /// - `get_capabilities().max_context_tokens` equals the profile's window when it has one;
    /// - every sampling parameter and stop sequences the profile admits reaches the wire with the
    ///   value sent, and every one it marks `NotSent` (set on the request all the same) is absent;
    /// - the production composition `Pricing(Cadence(adapter))` returns the same profile (a
    ///   decorator that stops forwarding `model_profile` silently disables the whole policy).
    pub async fn capability_declarations_match_the_wire<F: ConformanceFixture>() {
        use crate::cadence::{CadenceSettings, CadenceWiring, with_cadence};
        use crate::pricing::PricingLlmAdapter;
        use paladin_core::platform::container::cost::{CurrencyCode, PriceTable};
        use paladin_ports::output::cadence_port::CadencePolicy;
        use paladin_storage::cadence::InMemoryCadence;

        let model = F::default_model();
        // Profile and capability queries need no network.
        let adapter = F::adapter("http://127.0.0.1:9");
        let profile: ModelProfile = adapter.model_profile(model).unwrap_or_else(|| {
            panic!(
                "{}: model_profile({model:?}) must be Some for the default model",
                F::provider_name()
            )
        });

        if let Some(window) = profile.context_window {
            assert_eq!(
                adapter.get_capabilities().max_context_tokens,
                Some(window),
                "get_capabilities().max_context_tokens must equal the default model's profiled window"
            );
        }

        // Decorators forward the profile unchanged.
        let wiring = CadenceWiring::new(
            Arc::new(InMemoryCadence::new(CadencePolicy::default())),
            CadenceSettings::default(),
        );
        let paced = with_cadence(Arc::clone(&adapter), Some(&wiring));
        let table = PriceTable::new(CurrencyCode::new("USD").expect("USD is a valid code"));
        let decorated: Arc<dyn LlmPort> = Arc::new(PricingLlmAdapter::new(paced, Arc::new(table)));
        assert_eq!(
            decorated.model_profile(model),
            Some(profile.clone()),
            "Pricing(Cadence(adapter)) must return the adapter's own profile"
        );

        // The wire: set every parameter, an admitted value where the profile admits one and an
        // arbitrary value where it does not.
        const NOT_ADMITTED_PROBE: f32 = 0.5;
        let supports = [
            (SamplingParam::Temperature, profile.temperature),
            (SamplingParam::TopP, profile.top_p),
            (SamplingParam::FrequencyPenalty, profile.frequency_penalty),
            (SamplingParam::PresencePenalty, profile.presence_penalty),
        ];
        let sent: Vec<(SamplingParam, Option<f32>, f32)> = supports
            .iter()
            .map(|(param, support)| {
                let admitted = admitted_value(*support);
                (*param, admitted, admitted.unwrap_or(NOT_ADMITTED_PROBE))
            })
            .collect();
        let stop_supported = matches!(profile.stop_sequences, StopSupport::Supported { .. });

        let mut request = request_for_model(model);
        request.prompt.entity.data.parameters = PromptParameters {
            max_tokens: None,
            temperature: Some(sent[0].2),
            top_p: Some(sent[1].2),
            frequency_penalty: Some(sent[2].2),
            presence_penalty: Some(sent[3].2),
            stop_sequences: Some(vec!["END".to_string()]),
        };
        let (wire_body, result) = send_capturing::<F>(request, F::success_body()).await;
        result.expect("a request the profile partly admits must still succeed");

        for (param, admitted, value) in &sent {
            let on_wire = wire_param(&wire_body, F::WIRE, *param);
            match admitted {
                Some(_) => {
                    let got = on_wire.and_then(Value::as_f64).unwrap_or_else(|| {
                        panic!(
                            "{}: an admitted {} must reach the wire: {wire_body}",
                            F::provider_name(),
                            param.as_str()
                        )
                    });
                    assert!(
                        (got - f64::from(*value)).abs() < 1e-4,
                        "{}: {} reached the wire as {got}, sent {value}",
                        F::provider_name(),
                        param.as_str()
                    );
                }
                None => assert!(
                    on_wire.is_none(),
                    "{}: {} is NotSent in the profile and must never reach the wire: {wire_body}",
                    F::provider_name(),
                    param.as_str()
                ),
            }
        }
        let stop = wire_param(&wire_body, F::WIRE, SamplingParam::StopSequences);
        if stop_supported {
            assert_eq!(
                stop,
                Some(&serde_json::json!(["END"])),
                "{}: supported stop sequences must reach the wire in declared order: {wire_body}",
                F::provider_name()
            );
        } else {
            assert!(
                stop.is_none(),
                "{}: unsupported stop sequences reached the wire: {wire_body}",
                F::provider_name()
            );
        }
    }

    /// Case: a requested `JsonObject` reaches the wire in the shape the vendor documents, exactly
    /// when the default model's profile says it supports JSON mode -- OpenAI-chat
    /// `response_format.type = json_object`, Gemini `generationConfig.responseMimeType =
    /// application/json`, and Anthropic NO `response_format` key (prompt-level JSON only, as its
    /// adapter documents). When the profile says it does not, the key is absent and the response
    /// records the drop under `paladin.response_format_adjusted = omitted`.
    pub async fn response_format_reaches_the_wire<F: ConformanceFixture>() {
        let model = F::default_model();
        let adapter = F::adapter("http://127.0.0.1:9");
        let profile = adapter
            .model_profile(model)
            .expect("the default model must be profiled");

        let mut request = request_for_model(model);
        request.response_format = Some(ResponseFormat::JsonObject);
        let (wire_body, result) = send_capturing::<F>(request, F::success_body()).await;
        let response = result.expect("a JSON-object request must succeed");

        let sent_json_mode = match F::WIRE {
            Wire::OpenAiChat => {
                let kind = wire_body
                    .get("response_format")
                    .and_then(|f| f.get("type"))
                    .and_then(Value::as_str);
                if profile.json_object {
                    assert_eq!(kind, Some("json_object"), "{wire_body}");
                }
                kind == Some("json_object")
            }
            Wire::Gemini => {
                let mime = wire_body
                    .get("generationConfig")
                    .and_then(|c| c.get("responseMimeType"))
                    .and_then(Value::as_str);
                if profile.json_object {
                    assert_eq!(mime, Some("application/json"), "{wire_body}");
                }
                mime == Some("application/json")
            }
            Wire::Anthropic => {
                assert!(
                    wire_body.get("response_format").is_none(),
                    "Anthropic has no response_format key; JSON is requested in the prompt: {wire_body}"
                );
                false
            }
        };
        assert_eq!(
            sent_json_mode,
            profile.json_object,
            "{}: the profile says json_object = {} but the wire {} a JSON mode: {wire_body}",
            F::provider_name(),
            profile.json_object,
            if sent_json_mode {
                "carried"
            } else {
                "did not carry"
            },
        );
        assert_eq!(
            response
                .metadata
                .get(RESPONSE_FORMAT_ADJUSTED_METADATA_KEY)
                .map(String::as_str),
            (!profile.json_object).then_some("omitted"),
            "an omitted response_format must be recorded, and a sent one must not be"
        );
    }

    /// Case (D-11a, buffered path): a `stop`-equivalent finish with no text is
    /// [`LlmError::EmptyCompletion`], permanent -- never an `Ok` empty string.
    pub async fn empty_stop_completion_is_empty_completion<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(F::empty_stop_body())
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(conformance_request()).await;

        match result {
            Err(err @ LlmError::EmptyCompletion { .. }) => assert_eq!(
                err.transience(),
                Transience::Permanent,
                "an empty completion is permanent (retrying the same call does not help)"
            ),
            other => panic!("an empty stop completion must be EmptyCompletion, got {other:?}"),
        }
    }

    /// Case (D-11a, streaming path): the wire's normal stream framing with zero text deltas and a
    /// `stop`-equivalent terminal ends with [`LlmError::EmptyCompletion`] as the LAST item,
    /// permanent.
    pub async fn empty_stop_stream_is_empty_completion<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(F::empty_stop_stream_body())
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let stream = adapter
            .generate_stream(conformance_request())
            .await
            .expect("the stream must open; the emptiness is only known at the terminal frame");
        let mut stream = Box::into_pin(stream);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }

        match items.last() {
            Some(Err(err @ LlmError::EmptyCompletion { .. })) => assert_eq!(
                err.transience(),
                Transience::Permanent,
                "an empty streamed completion is permanent"
            ),
            other => panic!(
                "the last stream item must be Err(EmptyCompletion), got {other:?} (all items: {items:?})"
            ),
        }
    }

    /// Case (D-09): a model that declines to answer.
    ///
    /// A vendor with a typed refusal signal ([`RefusalShape::Typed`]) yields
    /// [`LlmError::Refusal`] naming the fixture's provider, permanent, with the adapter's own API
    /// key -- planted in the vendor's refusal text -- redacted out of `detail`. A vendor with no
    /// refusal signal ([`RefusalShape::ContentFilterOnly`], Gemini) asserts its documented
    /// alternative: a `SAFETY` block is `Ok` with [`FinishReason::ContentFilter`] and no text.
    /// Neither shape is ever a reason to skip the case.
    pub async fn refusal_is_typed_and_permanent<F: ConformanceFixture>() {
        let (body, typed) = match F::refusal_shape(F::api_key()) {
            RefusalShape::Typed(body) => (body, true),
            RefusalShape::ContentFilterOnly(body) => (body, false),
        };
        let mut server = Server::new_async().await;
        // A refusal is permanent, so the request must leave the process exactly once: a retry
        // re-bills the full prompt for an answer the vendor has already declined to give.
        let mock = server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .expect(1)
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let result = adapter.generate(conformance_request()).await;
        mock.assert_async().await;

        if typed {
            match result {
                Err(err @ LlmError::Refusal { .. }) => {
                    assert_eq!(
                        err.transience(),
                        Transience::Permanent,
                        "a refusal is permanent"
                    );
                    let LlmError::Refusal {
                        provider, detail, ..
                    } = &err
                    else {
                        unreachable!("matched as Refusal above");
                    };
                    assert_eq!(
                        provider,
                        F::provider_name(),
                        "the refusal must name the provider"
                    );
                    assert_eq!(
                        provider,
                        adapter.get_provider_name(),
                        "the fixture's provider_name() must match the adapter's own"
                    );
                    assert!(
                        !detail.trim().is_empty(),
                        "the refusal must carry its detail"
                    );
                    assert!(
                        !detail.contains(F::api_key()),
                        "the refusal detail leaked the adapter's API key: {detail}"
                    );
                }
                other => panic!("a typed refusal must be Err(Refusal), got {other:?}"),
            }
        } else {
            let response = result.expect("a content-filter block is a finish reason, not an error");
            assert!(
                matches!(response.finish_reason, FinishReason::ContentFilter),
                "a content-filter block must finish ContentFilter, got {:?}",
                response.finish_reason
            );
            assert!(
                response.content.trim().is_empty(),
                "a content-filter block carries no text, got {:?}",
                response.content
            );
        }
    }

    /// Case: a usage object that reports cache read, cache write and reasoning tokens maps to
    /// exactly the figures the fixture declares for its vendor (`None` where the vendor does not
    /// report the field), and `total_tokens` is `prompt + completion` -- the invariant
    /// `TokenUsage` guarantees, not a vendor figure (WINDOWS row 74).
    ///
    /// Plan 50-23 (G-50-5a): the mapped sub-counts also stay inside their parents (`reasoning <=
    /// completion`, `cache_read + cache_write <= prompt`), and an agreeing fixture's response
    /// carries no [`USAGE_UNRECONCILED_METADATA_KEY`] -- its vendor total, where it reports one,
    /// agrees with the mapped total.
    pub async fn usage_fields_are_mapped<F: ConformanceFixture>() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(F::usage_body())
            .create_async()
            .await;

        let adapter = F::adapter(&server.url());
        let response = adapter
            .generate(conformance_request())
            .await
            .expect("usage_body() must produce a well-formed response");
        let usage = response.usage;
        let expected = F::expected_usage();

        if let Some(reasoning) = usage.reasoning_tokens {
            assert!(
                reasoning <= usage.completion_tokens,
                "{}: reasoning_tokens {reasoning} escapes completion_tokens {}",
                F::provider_name(),
                usage.completion_tokens
            );
        }
        let cache = usage
            .cache_read_tokens
            .unwrap_or(0)
            .saturating_add(usage.cache_write_tokens.unwrap_or(0));
        assert!(
            cache <= usage.prompt_tokens,
            "{}: cache read + write {cache} escape prompt_tokens {}",
            F::provider_name(),
            usage.prompt_tokens
        );
        assert!(
            !response
                .metadata
                .contains_key(USAGE_UNRECONCILED_METADATA_KEY),
            "{}: an agreeing usage body must record no {USAGE_UNRECONCILED_METADATA_KEY}, got {:?}",
            F::provider_name(),
            response.metadata.get(USAGE_UNRECONCILED_METADATA_KEY)
        );

        assert_eq!(usage.prompt_tokens, expected.prompt, "prompt_tokens");
        assert_eq!(
            usage.completion_tokens, expected.completion,
            "completion_tokens"
        );
        assert_eq!(
            usage.cache_read_tokens, expected.cache_read,
            "cache_read_tokens"
        );
        assert_eq!(
            usage.cache_write_tokens, expected.cache_write,
            "cache_write_tokens"
        );
        assert_eq!(
            usage.reasoning_tokens, expected.reasoning,
            "reasoning_tokens"
        );
        assert_eq!(usage.total_tokens, expected.total, "total_tokens");
        assert_eq!(
            usage.total_tokens,
            usage.prompt_tokens + usage.completion_tokens,
            "total_tokens must equal prompt_tokens + completion_tokens"
        );
    }

    /// Case (plan 50-23, G-50-5a): a vendor total that disagrees with the mapped `prompt +
    /// completion` leaves the mapped usage exactly as [`ConformanceFixture::expected_usage`]
    /// declares (D-02: the vendor figure is never copied) and is recorded under
    /// [`USAGE_UNRECONCILED_METADATA_KEY`] as `total:vendor=<n>,mapped=<m>`. A wire with no
    /// vendor total ([`ConformanceFixture::disagreeing_usage_body`] is `None`) is measured on its
    /// plain usage body instead, which must carry no finding.
    pub async fn vendor_total_disagreement_is_recorded<F: ConformanceFixture>() {
        let disagreeing = F::disagreeing_usage_body();
        let body = disagreeing.clone().unwrap_or_else(F::usage_body);
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let response = F::adapter(&server.url())
            .generate(conformance_request())
            .await
            .expect("a usage body must produce a well-formed response");
        let expected = F::expected_usage();
        let usage = &response.usage;
        let mapped = ExpectedUsage {
            prompt: usage.prompt_tokens,
            completion: usage.completion_tokens,
            cache_read: usage.cache_read_tokens,
            cache_write: usage.cache_write_tokens,
            reasoning: usage.reasoning_tokens,
            total: usage.total_tokens,
        };
        assert_eq!(
            mapped,
            expected,
            "{}: the mapped usage must not move with the vendor total",
            F::provider_name()
        );

        let recorded = response.metadata.get(USAGE_UNRECONCILED_METADATA_KEY);
        if disagreeing.is_some() {
            let want = format!(
                "total:vendor={},mapped={}",
                expected.total + wire_defaults::DISAGREEMENT,
                expected.total
            );
            assert_eq!(
                recorded.map(String::as_str),
                Some(want.as_str()),
                "{}: a disagreeing vendor total must be recorded under \
                 {USAGE_UNRECONCILED_METADATA_KEY}",
                F::provider_name()
            );
        } else {
            assert!(
                recorded.is_none(),
                "{}: a wire with no vendor total records no finding on an agreeing body, got \
                 {recorded:?}",
                F::provider_name()
            );
        }
    }

    /// Case (G-50-5a, plan 50-27): a call the vendor answered and billed but that the shared
    /// guard classifies as a failure carries its usage on the error, on both paths.
    ///
    /// The empty-stop body (buffered), the empty-stop stream (its terminal `Err` item) and, where
    /// the wire has one ([`RefusalShape::Typed`]), the typed refusal each report a usage object, so
    /// each error's [`LlmError::billed_usage`] must be `Some` with a non-zero
    /// `total_tokens == prompt_tokens + completion_tokens` (D-02) on a named model, and no cost
    /// (pricing is the decorator's job). A vendor with no refusal signal states its
    /// content-filter alternative, which is an `Ok` finish and so carries no error to bill.
    pub async fn billed_failures_carry_their_usage<F: ConformanceFixture>() {
        fn assert_billed<F: ConformanceFixture>(err: &LlmError, path: &str) {
            let billed = err.billed_usage().unwrap_or_else(|| {
                panic!(
                    "{}: a billed {path} must carry its usage on the error, got {err:?}",
                    F::provider_name()
                )
            });
            let usage = &billed.usage;
            assert_eq!(
                usage.total_tokens,
                usage.prompt_tokens + usage.completion_tokens,
                "{}: the {path} bill's total must be prompt + completion (D-02)",
                F::provider_name()
            );
            assert!(
                usage.total_tokens > 0,
                "{}: the {path} body reported usage, so its bill is never a zero",
                F::provider_name()
            );
            assert!(
                !billed.model.is_empty(),
                "{}: the {path} bill must name its model",
                F::provider_name()
            );
            assert_eq!(
                billed.cost,
                None,
                "{}: an adapter never prices its own bill",
                F::provider_name()
            );
        }

        // Buffered empty completion.
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(F::empty_stop_body())
            .create_async()
            .await;
        match F::adapter(&server.url())
            .generate(conformance_request())
            .await
        {
            Err(err @ LlmError::EmptyCompletion { .. }) => {
                assert_billed::<F>(&err, "buffered empty completion");
            }
            other => panic!("an empty stop completion must be EmptyCompletion, got {other:?}"),
        }

        // Streamed empty completion: the terminal `Err` item carries the bill.
        let mut server = Server::new_async().await;
        server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/event-stream")
            .with_body(F::empty_stop_stream_body())
            .create_async()
            .await;
        let stream = F::adapter(&server.url())
            .generate_stream(conformance_request())
            .await
            .expect("the stream must open; the emptiness is only known at the terminal frame");
        let mut stream = Box::into_pin(stream);
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        match items.last() {
            Some(Err(err @ LlmError::EmptyCompletion { .. })) => {
                assert_billed::<F>(err, "streamed empty completion");
            }
            other => panic!(
                "the last stream item must be Err(EmptyCompletion), got {other:?} (all items: \
                 {items:?})"
            ),
        }

        // Typed refusal, where the wire has one.
        if let RefusalShape::Typed(body) = F::refusal_shape(F::api_key()) {
            let mut server = Server::new_async().await;
            server
                .mock("POST", Matcher::Any)
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(body)
                .create_async()
                .await;
            match F::adapter(&server.url())
                .generate(conformance_request())
                .await
            {
                Err(err @ LlmError::Refusal { .. }) => assert_billed::<F>(&err, "typed refusal"),
                other => panic!("a typed refusal must be Err(Refusal), got {other:?}"),
            }
        }
    }

    /// Case (ADR-0061 (a)): a temperature `99.0` -- admitted by no profile -- is omitted from every
    /// wire, never clamped or substituted, and the response records the omission under
    /// `paladin.params_omitted`.
    pub async fn unsupported_parameters_are_omitted_and_recorded<F: ConformanceFixture>() {
        let mut request = request_for_model(F::default_model());
        request.prompt.entity.data.parameters = PromptParameters {
            max_tokens: None,
            temperature: Some(99.0),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: None,
        };
        let (wire_body, result) = send_capturing::<F>(request, F::success_body()).await;
        let response =
            result.expect("an inadmissible temperature is omitted, not refused, by an adapter");

        assert!(
            wire_param(&wire_body, F::WIRE, SamplingParam::Temperature).is_none(),
            "{}: a temperature no profile admits reached the wire: {wire_body}",
            F::provider_name()
        );
        let omitted = response
            .metadata
            .get(PARAMS_OMITTED_METADATA_KEY)
            .unwrap_or_else(|| {
                panic!(
                    "{}: the omission must be recorded under {PARAMS_OMITTED_METADATA_KEY}",
                    F::provider_name()
                )
            });
        assert!(
            omitted
                .split(',')
                .any(|name| name == SamplingParam::Temperature.as_str()),
            "{PARAMS_OMITTED_METADATA_KEY} must list temperature, got {omitted:?}"
        );
    }

    /// Case (Phase 51, D-00i, REASON-01): a request that sets no reasoning effort leaves the
    /// adapter's wire body **byte for byte** what the adapter sent before the effort feature
    /// existed. The expected body is `tests/golden/reasoning_unset/<provider_name>.json`,
    /// captured from the unmodified adapters before any of them was edited. `Auto` is a set
    /// value and is not exercised here: this case sends `None`.
    ///
    /// Run with `UPDATE_REASONING_GOLDEN=1` to (re)write the file instead of comparing; the
    /// compared text ignores one trailing newline, which the repository's end-of-file hook adds.
    pub async fn unset_reasoning_leaves_the_wire_body_unchanged<F: ConformanceFixture>() {
        let request = request_for_model(F::default_model());
        assert!(
            request.reasoning.is_none() && request.reasoning_fallback.is_none(),
            "{}: the conformance request must leave the effort unset",
            F::provider_name()
        );
        let (raw_body, result) = send_capturing_raw::<F>(request, F::success_body()).await;
        result.expect("an unset-effort request must succeed");

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/reasoning_unset")
            .join(format!("{}.json", F::provider_name()));
        if std::env::var("UPDATE_REASONING_GOLDEN").is_ok_and(|v| v == "1") {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).expect("create the golden directory");
            }
            std::fs::write(&path, format!("{raw_body}\n")).expect("write the golden body");
            return;
        }
        let golden = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: cannot read the golden body {}: {error}. Bless it with \
                 UPDATE_REASONING_GOLDEN=1 only from an unmodified adapter.",
                F::provider_name(),
                path.display()
            )
        });
        let golden = golden.strip_suffix('\n').unwrap_or(&golden);
        assert!(
            raw_body == golden,
            "{}: the unset-effort wire body changed (D-00i). Golden file {} (bless switch \
             UPDATE_REASONING_GOLDEN=1, only for an intended change).\n  golden: {golden}\n  actual: {raw_body}",
            F::provider_name(),
            path.display()
        );
    }

    // ── Phase 51 plan 51-12: effort admission and the seeded wire round trip ──

    /// A model id no seeded table lists, so it carries no effort data on any adapter.
    const UNPROFILED_MODEL: &str = "conformance-unprofiled-model";

    /// The model ids `provider`'s seeded table lists, in table order (empty for a provider with
    /// no table: Ollama and OpenAI-compatible).
    fn seeded_models(provider: &str) -> Vec<&'static str> {
        #[allow(unused_imports)]
        use crate::registry::{SeededProfile, seeded};
        let table: &[SeededProfile] = match provider {
            #[cfg(feature = "openai")]
            "openai" => seeded::OPENAI,
            #[cfg(feature = "anthropic")]
            "anthropic" => seeded::ANTHROPIC,
            #[cfg(feature = "deepseek")]
            "deepseek" => seeded::DEEPSEEK,
            #[cfg(feature = "kimi")]
            "kimi" => seeded::KIMI,
            #[cfg(feature = "qwen")]
            "qwen" => seeded::QWEN,
            #[cfg(feature = "grok")]
            "grok" => seeded::GROK,
            #[cfg(feature = "gemini")]
            "gemini" => seeded::GEMINI,
            _ => &[],
        };
        table.iter().map(|entry| entry.model).collect()
    }

    /// One mock server that records the last request body it received, and an adapter of `F`
    /// pointed at it.
    struct Session {
        _server: mockito::ServerGuard,
        slot: Arc<std::sync::Mutex<Option<String>>>,
        adapter: Arc<dyn LlmPort>,
    }

    impl Session {
        async fn new<F: ConformanceFixture>() -> Self {
            let slot: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
            let writer = Arc::clone(&slot);
            let mut server = Server::new_async().await;
            let response_body = F::success_body();
            server
                .mock("POST", Matcher::Any)
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body_from_request(move |req| {
                    let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                    if let Ok(mut guard) = writer.lock() {
                        *guard = Some(text);
                    }
                    response_body.clone().into_bytes()
                })
                .create_async()
                .await;
            let adapter = F::adapter(&server.url());
            Self {
                _server: server,
                slot,
                adapter,
            }
        }

        /// Send `request` buffered; returns the JSON body that reached the wire (`None` when
        /// no HTTP call was made) and the result.
        async fn send(
            &self,
            request: LlmRequest,
        ) -> (Option<Value>, Result<LlmResponse, LlmError>) {
            let result = self.adapter.generate(request).await;
            let text = self
                .slot
                .lock()
                .expect("the capture slot is never poisoned")
                .take();
            let body = text.map(|text| {
                serde_json::from_str(&text).expect("the adapter must send a JSON request body")
            });
            (body, result)
        }
    }

    /// `body` with every field an effort can add removed: the top-level `reasoning_effort`,
    /// `thinking`, `enable_thinking`, `thinking_budget` and `output_config`, and Gemini's
    /// `generationConfig.thinkingConfig`. What is left must be identical with and without an
    /// effort.
    fn without_effort_fields(mut body: Value) -> Value {
        if let Some(object) = body.as_object_mut() {
            for key in [
                "reasoning_effort",
                "thinking",
                "enable_thinking",
                "thinking_budget",
                "output_config",
            ] {
                object.remove(key);
            }
            if let Some(config) = object
                .get_mut("generationConfig")
                .and_then(Value::as_object_mut)
            {
                config.remove("thinkingConfig");
            }
        }
        body
    }

    /// Case (Phase 51, REASON-02): an explicit `with_reasoning(High)` on a model id the adapter
    /// has no effort data for is refused before any HTTP call, on every adapter: an
    /// [`LlmError::UnsupportedParameter`], [`Transience::Permanent`], zero requests captured (a
    /// mock that expects none). Both the buffered and the streaming path.
    pub async fn explicit_reasoning_on_an_unprofiled_model_refuses_before_http<
        F: ConformanceFixture,
    >() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", Matcher::Any)
            .with_status(200)
            .with_body(F::success_body())
            .expect(0)
            .create_async()
            .await;
        let adapter = F::adapter(&server.url());

        let request = request_for_model(UNPROFILED_MODEL).with_reasoning(ReasoningEffort::High);
        let error = adapter
            .generate(request.clone())
            .await
            .expect_err("an explicit effort on an unprofiled model must be refused");
        assert!(
            matches!(error, LlmError::UnsupportedParameter { .. }),
            "{}: expected UnsupportedParameter, got {error:?}",
            F::provider_name()
        );
        assert_eq!(error.transience(), Transience::Permanent);

        let streamed = adapter.generate_stream(request).await;
        assert!(
            matches!(streamed, Err(LlmError::UnsupportedParameter { .. })),
            "{}: the streaming path must refuse too",
            F::provider_name()
        );
        mock.assert_async().await;
    }

    /// Case (Phase 51, REASON-03): the first seeded model whose profile lists an effort level
    /// puts that level on the wire exactly: the body it sent equals the model's unset body plus
    /// only the effort field(s) ([`without_effort_fields`]), and
    /// [`ConformanceFixture::effort_wire_value`] reads the level back. A fixture whose seeded
    /// table lists no model with a level (Qwen's budget-only rows, and the self-hosted adapters,
    /// which have no table and are measured on their default model) asserts instead that every
    /// one of its ids refuses an explicit level before HTTP.
    pub async fn seeded_effort_levels_reach_the_wire<F: ConformanceFixture>() {
        let provider = F::provider_name();
        let mut ids = seeded_models(provider);
        if ids.is_empty() {
            ids.push(F::default_model());
        }
        let session = Session::new::<F>().await;
        let first_with_a_level =
            ids.iter()
                .find_map(|id| match session.adapter.model_profile(id)?.effort {
                    EffortSupport::Supported(levels) => {
                        levels.levels.first().map(|level| (*id, *level))
                    }
                    _ => None,
                });

        let Some((model, level)) = first_with_a_level else {
            for id in ids {
                let (body, result) = session
                    .send(request_for_model(id).with_reasoning(ReasoningEffort::High))
                    .await;
                assert!(
                    body.is_none(),
                    "{provider}/{id}: a refused effort reached the wire"
                );
                assert!(
                    matches!(result, Err(LlmError::UnsupportedParameter { .. })),
                    "{provider}/{id}: expected a refusal, got {result:?}"
                );
            }
            return;
        };

        let (unset, result) = session.send(request_for_model(model)).await;
        result.expect("the unset request must succeed");
        let unset = unset.expect("the unset request reached the wire");
        let (set, result) = session
            .send(request_for_model(model).with_reasoning(level))
            .await;
        result.unwrap_or_else(|e| panic!("{provider}/{model}/{level}: {e}"));
        let set = set.expect("the effort request reached the wire");

        assert_eq!(
            F::effort_wire_value(&set),
            Some(level.to_string()),
            "{provider}/{model}: the level is not on the wire in {set}"
        );
        assert_eq!(
            F::effort_wire_value(&unset),
            None,
            "{provider}/{model}: an unset body carries a level: {unset}"
        );
        assert_eq!(
            without_effort_fields(set.clone()),
            without_effort_fields(unset),
            "{provider}/{model}: the effort body differs from the unset body by more than the \
             effort fields: {set}"
        );
    }

    /// Case (Phase 51, the assumption-delta invariant): for every seeded model of the adapter
    /// whose effort is `Supported`, every listed level the profile reports reaches the wire as
    /// exactly that level ([`ConformanceFixture::effort_wire_value`]). A seeded level the adapter
    /// does not send, or sends as another, fails here.
    pub async fn every_seeded_effort_level_round_trips_to_its_wire_body<F: ConformanceFixture>() {
        let provider = F::provider_name();
        let session = Session::new::<F>().await;
        for id in seeded_models(provider) {
            let Some(EffortSupport::Supported(levels)) =
                session.adapter.model_profile(id).map(|p| p.effort)
            else {
                continue;
            };
            for level in levels.levels {
                let (body, result) = session
                    .send(request_for_model(id).with_reasoning(level))
                    .await;
                result.unwrap_or_else(|e| panic!("{provider}/{id}/{level}: {e}"));
                let body = body.expect("the effort request reached the wire");
                assert_eq!(
                    F::effort_wire_value(&body),
                    Some(level.to_string()),
                    "{provider}/{id}: seeded level {level} did not reach the wire: {body}"
                );
            }
        }
    }
}

/// Generate one `#[tokio::test]` per fixed case in [`cases`] for a concrete
/// [`ConformanceFixture`], plus a `CASE_COUNT` derived from the SAME identifier lists the tests
/// are generated from -- so a case silently dropped from the lists shrinks `CASE_COUNT` too,
/// which `suite_generates_the_full_case_list_for_a_fixture` (below) pins against the documented
/// `24`, turning a silently-dropped case into a build/test failure rather than a quiet pass.
///
/// The lists are split in two groups: the ten transport cases (status mapping, streaming
/// framing, redaction, redirects, rate limits) and the fourteen behaviour cases
/// (capability truthfulness, structured output, empty completion, refusal, usage, vendor-total
/// reconciliation, billed failures, the parameter policy, the unset-effort golden body, the
/// unprofiled-model effort refusal, the seeded effort wire body and the seeded round trip).
/// `llm_conformance_suite!(Fixture)` generates both.
/// `llm_conformance_suite!(Fixture, transport_only)` generates only the first group and exists
/// for one caller, the macro's own
/// self-test fixture, which is a bare transport with no profile, sampling or refusal surface;
/// `CASE_COUNT` is the full suite's size either way. Every one of the nine adapter fixtures uses
/// the full form.
#[macro_export]
macro_rules! llm_conformance_suite {
    ($fixture:ty) => {
        $crate::llm_conformance_suite!(@all $fixture, all);
    };
    ($fixture:ty, transport_only) => {
        $crate::llm_conformance_suite!(@all $fixture, transport_only);
    };
    // The one place the case lists are written down.
    (@all $fixture:ty, $mode:ident) => {
        $crate::llm_conformance_suite!(@groups $fixture, $mode;
            [
                generate_and_usage_extraction,
                streaming_assembles_in_wire_order_with_a_terminal_stop,
                stream_error_before_the_first_chunk,
                stream_error_after_the_first_chunk,
                dedicated_status_mappings,
                transience_by_value,
                credential_never_appears_in_a_rendered_error,
                redirect_is_not_followed_with_a_credential_header,
                streaming_usage_equals_non_streaming_usage,
                rate_limit_is_surfaced_once_with_its_retry_delay,
            ];
            [
                capability_declarations_match_the_wire,
                response_format_reaches_the_wire,
                empty_stop_completion_is_empty_completion,
                empty_stop_stream_is_empty_completion,
                stream_frames_split_across_reads_are_reassembled,
                refusal_is_typed_and_permanent,
                usage_fields_are_mapped,
                vendor_total_disagreement_is_recorded,
                billed_failures_carry_their_usage,
                unsupported_parameters_are_omitted_and_recorded,
                unset_reasoning_leaves_the_wire_body_unchanged,
                explicit_reasoning_on_an_unprofiled_model_refuses_before_http,
                seeded_effort_levels_reach_the_wire,
                every_seeded_effort_level_round_trips_to_its_wire_body,
            ]
        );
    };
    (@groups $fixture:ty, $mode:ident;
        [$($transport:ident),+ $(,)?];
        [$($behaviour:ident),+ $(,)?]
    ) => {
        $crate::llm_conformance_suite!(@tests $fixture; $($transport),+);
        $crate::llm_conformance_suite!(@behaviour_tests $mode, $fixture; $($behaviour),+);

        /// The number of cases in the full suite -- mechanically derived from the same token
        /// lists the `#[tokio::test]` functions above were generated from, never hand-copied.
        #[allow(dead_code)]
        pub const CASE_COUNT: usize =
            $crate::llm_conformance_suite!(@count $($transport,)+ $($behaviour),+);
    };
    (@behaviour_tests all, $fixture:ty; $($case:ident),+) => {
        $crate::llm_conformance_suite!(@tests $fixture; $($case),+);
    };
    (@behaviour_tests transport_only, $fixture:ty; $($case:ident),+) => {};
    (@tests $fixture:ty; $($case:ident),+ $(,)?) => {
        $(
            #[tokio::test]
            async fn $case() {
                $crate::conformance::cases::$case::<$fixture>().await;
            }
        )+
    };
    (@count) => { 0usize };
    (@count $head:ident $(, $tail:ident)* $(,)?) => {
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
    // `crate::http_status::map_http_status_with_hints` every real adapter uses, so the generated cases
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
        /// D-19: present only on the terminal (`done: true`) frame, carrying the SAME figures
        /// [`TrivialFixture::success_body`] carries.
        #[serde(default)]
        usage: Option<TrivialUsage>,
    }

    /// Snapshot a `429`'s rate-limit headers through the generic family, as every real adapter
    /// does (PACE-01); any other status carries no hints.
    fn snapshot_trivial_hints(
        response: &reqwest::Response,
    ) -> Option<paladin_ports::output::rate_limit_hints::RateLimitHints> {
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return None;
        }
        crate::rate_limit_headers::hints_from_headers(
            crate::rate_limit_headers::RateLimitHeaderFamily::Generic,
            std::time::SystemTime::now(),
            |name| response.headers().get(name).and_then(|v| v.to_str().ok()),
        )
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
            // PACE-01: snapshot the 429 headers BEFORE the body read consumes the response.
            let hints = snapshot_trivial_hints(&response);
            let body = response
                .text()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            if status >= 300 {
                return Err(crate::http_status::map_http_status_with_hints(
                    TRIVIAL_PROVIDER,
                    status,
                    &body,
                    TRIVIAL_API_KEY,
                    hints,
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
                cost: None,
                created_at: chrono::Utc::now(),
                metadata: Default::default(),
                function_call: None,
                reasoning: None,
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
                let hints = snapshot_trivial_hints(&response);
                let body = response
                    .text()
                    .await
                    .map_err(|e| LlmError::NetworkError(e.to_string()))?;
                return Err(crate::http_status::map_http_status_with_hints(
                    TRIVIAL_PROVIDER,
                    status,
                    &body,
                    TRIVIAL_API_KEY,
                    hints,
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
                                Ok(frame) => {
                                    let mut item = StreamingResponse::delta(frame.delta);
                                    if frame.done {
                                        item.finish_reason = Some(FinishReason::Stop);
                                    }
                                    if let Some(usage) = frame.usage {
                                        item = item.with_usage(TokenUsage::new(
                                            usage.prompt_tokens,
                                            usage.completion_tokens,
                                        ));
                                    }
                                    items.push(Ok(item));
                                }
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
        /// The bare transport has no effort wire.
        fn effort_wire_value(_body: &serde_json::Value) -> Option<String> {
            None
        }

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
            // D-19: the terminal frame carries the SAME usage figures as `success_body()`
            // above (prompt_tokens 1, completion_tokens 1) -- the shared parity case asserts
            // equality between the two.
            concat!(
                "data: {\"delta\":\"Hel\",\"done\":false}\n\n",
                "data: {\"delta\":\"lo \",\"done\":false}\n\n",
                "data: {\"delta\":\"world\",\"done\":true,\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
            )
            .to_string()
        }

        fn error_body(status: u16) -> String {
            format!("trivial error body for status {status}")
        }

        fn default_model() -> &'static str {
            "trivial-model"
        }

        fn provider_name() -> &'static str {
            TRIVIAL_PROVIDER
        }
    }

    // The trivial fixture is a bare transport (no sampling, profile or refusal surface), so it
    // runs the transport group only; `CASE_COUNT` is still the full suite's size.
    crate::llm_conformance_suite!(TrivialFixture, transport_only);

    /// Test 1: instantiating `llm_conformance_suite!` for a trivial in-file fixture produces one
    /// test per fixed case, and the generated module's test count equals the documented case
    /// count -- so a case silently dropped from the macro's list fails THIS build rather than
    /// passing quietly (the macro's `CASE_COUNT` and its `#[tokio::test]` functions are generated
    /// from the identical token list above, see `llm_conformance_suite!`'s own doc comment).
    #[test]
    fn suite_generates_the_full_case_list_for_a_fixture() {
        assert_eq!(
            CASE_COUNT, 24,
            "llm_conformance_suite! must generate exactly the documented 24 fixed cases -- if this \
             fails, a case was added to or removed from the macro's list without updating this \
             pinned expectation"
        );
    }
}
