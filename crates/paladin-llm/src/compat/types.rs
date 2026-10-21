//! Generalized OpenAI-compatible wire types, private to the crate.
//!
//! Derived from `deepseek/adapter.rs`'s private `DeepSeek*` structs
//! (`:104-187`), generalized so every preset built on [`super::engine`]
//! shares one definition of the wire shape rather than one copy per
//! provider.

use serde::{Deserialize, Serialize};

use crate::redaction::deserialize_null_as_empty_string;

/// Outgoing chat-completions request body.
#[derive(Debug, Serialize)]
pub(crate) struct CompatRequest {
    pub model: String,
    pub messages: Vec<CompatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// The caller's stop sequences, in the order declared, when the model's profile admits them
    /// (ADR-0061). Absent -- never `null` -- when the caller set none or the profile does not
    /// admit them, so a body without stop sequences stays byte-identical to a pre-Phase-50 one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    pub stream: bool,
    /// `LlmRequest.response_format` on the wire (RT-FR-17, D-28). Omitted
    /// entirely when the caller sets no hint, keeping the body byte
    /// -identical to a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<CompatResponseFormat>,
    /// Requests the OpenAI-family `usage` object on the final SSE frame of a
    /// streaming call (D-13/D-15). `Some({"include_usage": true})` whenever
    /// `stream` is `true`; omitted entirely on a non-streaming request, so
    /// that request body stays byte-identical to a pre-0.10 one (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<CompatStreamOptions>,
    /// `LlmRequest.reasoning` as a listed level on the `TopLevelEffort` wire (Phase 51, D-00f):
    /// the level's spelling, or `"none"` for an on-wire `Off`. Absent unless that wire is
    /// declared and an effort was admitted, so an unset request's body stays byte-identical
    /// (D-00i). The last four fields are the reasoning wires, one per declared `ReasoningWire`;
    /// no wire sets a field of another.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<&'static str>,
    /// The `ThinkingObject` wire: `{"type": "enabled" | "disabled", "effort": "L"}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<CompatThinking>,
    /// The `EnableThinking` wire's switch: `false` for `Off`, `true` for `Auto` and a budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_thinking: Option<bool>,
    /// The `EnableThinking` wire's token budget, sent beside `enable_thinking: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<u32>,
}

/// `CompatRequest.thinking`'s shape on the `ThinkingObject` wire: `type` is `enabled` or
/// `disabled`; `effort` is the level's spelling and is absent for `disabled` and for an
/// unspecified-depth `enabled`.
#[derive(Debug, Serialize)]
pub(crate) struct CompatThinking {
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<&'static str>,
}

/// The `stream_options` request field's only shape this engine sends
/// (D-15): ask every OpenAI-compatible endpoint to emit a trailing
/// empty-`choices` frame carrying the call's `usage` object.
#[derive(Debug, Serialize)]
pub(crate) struct CompatStreamOptions {
    pub include_usage: bool,
}

/// The provider-agnostic `response_format` hint, compiled down to this
/// engine's own plain JSON-object wire shape (D-28). Whether one is sent at all, and whether a
/// schema request is downgraded to this form, follows the model's profile (ADR-0061 (i)); a
/// downgrade is recorded as `paladin.response_format_adjusted`.
///
/// Every preset built on [`super::engine::CompatEngine`] — Kimi, Qwen, Grok,
/// Ollama and the generic OpenAI-compatible preset — speaks the de-facto
/// chat-completions dialect without the schema-carrying extension OpenAI
/// itself offers, so a `ResponseFormat::JsonSchema` request degrades to this
/// same `{"type":"json_object"}` form rather than being omitted
/// (EDGE(RT-05/wire shape)): correctness never depends on it, because the
/// caller also appends the schema-conformance instruction block (D-27).
#[derive(Debug, Serialize)]
pub(crate) struct CompatResponseFormat {
    #[serde(rename = "type")]
    pub kind: &'static str,
}

/// A single chat message, on both the request and response paths.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CompatMessage {
    pub role: String,
    /// The visible answer. Deserialized through
    /// [`deserialize_null_as_empty_string`] because a reasoning model that
    /// spends its whole budget on hidden reasoning may report the empty
    /// answer as `null` rather than `""`. Serialization is unaffected —
    /// outgoing messages always emit a plain JSON string.
    #[serde(default, deserialize_with = "deserialize_null_as_empty_string")]
    pub content: String,
    /// Hidden chain-of-thought content some compatible providers' reasoning
    /// models emit. Only ever present on responses; omitted from outgoing
    /// requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// The vendor's dedicated refusal text (OpenAI-shaped `message.refusal`), when the model
    /// declined to answer. Only ever present on responses; omitted from outgoing requests.
    /// `null` and absent both mean "no refusal" (D-09).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// Non-streaming chat-completions response body.
#[derive(Debug, Deserialize)]
pub(crate) struct CompatResponse {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    pub _id: String,
    pub model: String,
    pub choices: Vec<CompatChoice>,
    /// Optional so an empty or refused response with no usage object still reaches the guard
    /// (G-50-5a): such a failure carries no bill. A successful response without it is still a
    /// schema mismatch (`CompatEngine::generate`).
    #[serde(default)]
    pub usage: Option<CompatUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatChoice {
    #[serde(rename = "index")]
    #[allow(dead_code)]
    pub _index: u32,
    pub message: CompatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// The vendor's own reported total (G-50-5a). Some compatible providers
    /// omit it, so it is `None` then. When present it is parsed and
    /// reconciled against the mapped `prompt_tokens + completion_tokens`
    /// (`crate::usage_reconcile`), never copied into `TokenUsage`, which
    /// keeps `total_tokens == prompt_tokens + completion_tokens` (D-02): a
    /// disagreement is recorded under
    /// [`crate::usage_reconcile::USAGE_UNRECONCILED_METADATA_KEY`] instead
    /// of being discarded.
    #[serde(default)]
    pub total_tokens: Option<u32>,
    /// OpenAI-shaped `prompt_tokens_details.cached_tokens` (D-20). Absent on
    /// providers that never report a cache split; `None` in that case (D-03)
    /// — never fabricated as `Some(0)`.
    #[serde(default)]
    pub prompt_tokens_details: Option<CompatPromptTokensDetails>,
    /// OpenAI-shaped `completion_tokens_details.reasoning_tokens` (D-20).
    #[serde(default)]
    pub completion_tokens_details: Option<CompatCompletionTokensDetails>,
}

/// `CompatUsage.prompt_tokens_details` (D-20).
#[derive(Debug, Deserialize)]
pub(crate) struct CompatPromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<u32>,
    /// Moonshot's `prompt_tokens_details.cache_write_tokens` (D-11; the field the handoff
    /// measured and `50-SEED-PROBE.md` re-checked for Kimi). Absent elsewhere.
    #[serde(default)]
    pub cache_write_tokens: Option<u32>,
}

/// `CompatUsage.completion_tokens_details` (D-20).
#[derive(Debug, Deserialize)]
pub(crate) struct CompatCompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
}

/// A single SSE `data: {...}` streaming chunk.
#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamResponse {
    /// `#[serde(default)]` because the trailing empty-`choices` usage frame
    /// (D-14/D-15) is not guaranteed to repeat the stream's `id` on every
    /// vendor.
    #[serde(rename = "id", default)]
    #[allow(dead_code)]
    pub _id: String,
    pub choices: Vec<CompatStreamChoice>,
    /// Present only on the trailing empty-`choices` frame a
    /// `stream_options: {"include_usage": true}` request elicits (D-14/D-15).
    #[serde(default)]
    pub usage: Option<CompatUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamChoice {
    pub delta: CompatStreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamDelta {
    pub content: Option<String>,
    /// Hidden chain-of-thought text streamed on this delta, in wire order (D-11).
    #[serde(default)]
    pub reasoning_content: Option<String>,
    /// A fragment of the vendor's refusal text (D-09). `null` and absent mean none.
    #[serde(default)]
    pub refusal: Option<String>,
}

/// `GET {base_url}/models` response shape: `{ "data": [ { "id": "..." } ] }`.
#[derive(Debug, Deserialize)]
pub(crate) struct CompatModelsResponse {
    pub data: Vec<CompatModelEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatModelEntry {
    pub id: String,
}
