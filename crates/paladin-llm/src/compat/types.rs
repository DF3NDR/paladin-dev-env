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
    pub stream: bool,
    /// `LlmRequest.response_format` on the wire (RT-FR-17, D-28). Omitted
    /// entirely when the caller sets no hint, keeping the body byte
    /// -identical to a pre-0.10 request (X-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<CompatResponseFormat>,
}

/// The provider-agnostic `response_format` hint, compiled down to this
/// engine's own plain JSON-object wire shape (D-28).
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
}

/// Non-streaming chat-completions response body.
#[derive(Debug, Deserialize)]
pub(crate) struct CompatResponse {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    pub _id: String,
    pub model: String,
    pub choices: Vec<CompatChoice>,
    pub usage: CompatUsage,
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
    /// Some compatible providers omit `total_tokens` from their usage
    /// object. When absent, the engine computes `prompt_tokens +
    /// completion_tokens` rather than reporting zero (PROV-02 precision
    /// edge) — see [`super::engine::CompatEngine`]'s usage-construction
    /// site.
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

/// A single SSE `data: {...}` streaming chunk.
#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamResponse {
    #[serde(rename = "id")]
    #[allow(dead_code)]
    pub _id: String,
    pub choices: Vec<CompatStreamChoice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamChoice {
    pub delta: CompatStreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompatStreamDelta {
    pub content: Option<String>,
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
