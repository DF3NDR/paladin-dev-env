//! # paladin-llm
//!
//! LLM provider adapters for the Paladin framework.
//!
//! This crate provides concrete adapter implementations for multiple LLM providers,
//! all implementing the [`paladin_ports::output::llm_port::LlmPort`] trait defined
//! in `paladin-ports`.
//!
//! ## Supported Providers
//!
//! | Feature flag | Provider | Types |
//! |---|---|---|
//! | `openai` (default) | OpenAI | [`openai::OpenAIAdapter`], [`openai::OpenAIConfig`] |
//! | `anthropic` | Anthropic | [`anthropic::AnthropicAdapter`], [`anthropic::AnthropicConfig`] |
//! | `deepseek` | DeepSeek | [`deepseek::DeepSeekAdapter`], [`deepseek::DeepSeekConfig`] |
//! | `kimi` | Kimi (Moonshot AI) | [`kimi::KimiAdapter`], [`kimi::KimiConfig`] |
//! | `qwen` | Qwen (Alibaba DashScope) | [`qwen::QwenAdapter`], [`qwen::QwenConfig`] |
//! | `grok` | Grok (xAI) | [`grok::GrokAdapter`], [`grok::GrokConfig`] |
//! | `ollama` | Ollama (self-hosted, keyless) | [`ollama::OllamaAdapter`], [`ollama::OllamaConfig`] |
//! | `openai-compatible` | Any OpenAI-compatible endpoint (operator-configured) | [`openai_compatible::OpenAiCompatibleAdapter`], [`openai_compatible::OpenAiCompatibleConfig`] |
//! | `gemini` | Google Gemini (text-only, bespoke protocol) | [`gemini::GeminiAdapter`], [`gemini::GeminiConfig`] |
//! | `mock` (default) | Testing | [`mock::MockLlmAdapter`], [`mock::MultiStepMockLlmPort`] |
//! | `openai-embeddings` | OpenAI Embeddings | [`openai::OpenAIEmbeddingAdapter`] |
//! | `vision` | Vision (multimodal) | Extends OpenAI and Anthropic adapters |
//!
//! ## Architecture
//!
//! Follows the Hexagonal Architecture pattern — this crate is a pure adapter
//! layer. It depends only on `paladin-core` (domain types) and `paladin-ports`
//! (port trait contracts). It has no dependency on the root `paladin` crate.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! # #[cfg(feature = "openai")]
//! # {
//! use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};
//! use paladin_llm::provider_factory::LlmProviderFactory;
//!
//! // From environment variables
//! let factory = LlmProviderFactory::new();
//! let provider = factory.create("openai").expect("OPENAI_API_KEY must be set");
//! # }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(rustdoc::broken_intra_doc_links)]

/// Shared configuration types for LLM providers and request behavior.
#[allow(missing_docs)]
pub mod config;
/// Error types returned by provider adapters.
#[allow(missing_docs)]
pub mod error;
/// LLM-backed content analysis service orchestration.
#[allow(missing_docs)]
pub mod llm_analysis_service;
/// Factory for selecting provider adapters from runtime configuration.
#[allow(missing_docs)]
pub mod provider_factory;
/// Credential redaction shared by every provider adapter (not feature-gated
/// — reused by the shared compatible core and by bespoke adapters alike).
#[allow(missing_docs)]
pub mod redaction;
/// The one shared HTTP-status-to-`LlmError` mapping every provider adapter
/// routes its non-2xx branch through (FT-FR-01, D-03) — redacts before it
/// bounds, and emits a typed `ProviderError { status }` for every status
/// without a dedicated variant. Not feature-gated.
pub mod http_status;

#[cfg(any(
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "openai-compatible"
))]
/// Shared OpenAI-compatible protocol engine (D-05) that thin provider
/// presets sit on. Widen this `any(...)` list as later presets land.
#[allow(missing_docs)]
pub mod compat;

#[cfg(feature = "openai")]
/// OpenAI provider adapter and related configuration.
#[allow(missing_docs)]
pub mod openai;

#[cfg(feature = "anthropic")]
/// Anthropic provider adapter and related configuration.
#[allow(missing_docs)]
pub mod anthropic;

#[cfg(feature = "deepseek")]
/// DeepSeek provider adapter and related configuration.
#[allow(missing_docs)]
pub mod deepseek;

#[cfg(feature = "mock")]
/// Mock provider adapters for tests and deterministic workflows.
#[allow(missing_docs)]
pub mod mock;

#[cfg(feature = "kimi")]
/// Kimi (Moonshot AI) provider adapter and related configuration.
#[allow(missing_docs)]
pub mod kimi;

#[cfg(feature = "qwen")]
/// Qwen (Alibaba DashScope) provider adapter and related configuration.
#[allow(missing_docs)]
pub mod qwen;

#[cfg(feature = "grok")]
/// Grok (xAI) provider adapter and related configuration.
#[allow(missing_docs)]
pub mod grok;

#[cfg(feature = "ollama")]
/// Ollama (self-hosted, keyless) provider adapter and related configuration.
#[allow(missing_docs)]
pub mod ollama;

#[cfg(feature = "openai-compatible")]
/// Generic operator-configured OpenAI-compatible provider adapter and
/// related configuration (D-03) — points at any OpenAI-compatible endpoint
/// with no new code.
#[allow(missing_docs)]
pub mod openai_compatible;

#[cfg(feature = "gemini")]
/// Google Gemini (text-only) provider adapter and related configuration
/// (D-08) — implements [`paladin_ports::output::llm_port::LlmPort`]
/// directly against Gemini's own bespoke `generateContent` protocol; does
/// not sit on [`compat::CompatEngine`].
#[allow(missing_docs)]
pub mod gemini;

/// Cross-adapter capability invariants (WEB-03, ADR-0004).
///
/// These tests need all three shipped adapters in scope simultaneously, which none of
/// the per-adapter `#[cfg(test)]` modules can see on their own (each only compiles
/// under its own feature flag) — so they live here, gated on all three features being
/// enabled together (as they are for `cargo test --workspace`, since the root
/// `paladin-ai` package requests `openai`, `anthropic` and `deepseek` together).
#[cfg(all(test, feature = "openai", feature = "anthropic", feature = "deepseek"))]
mod capability_invariants {
    use crate::anthropic::{AnthropicAdapter, AnthropicConfig};
    use crate::deepseek::{DeepSeekAdapter, DeepSeekConfig};
    use crate::openai::{OpenAIAdapter, OpenAIConfig};
    use paladin_ports::output::llm_port::LlmPort;

    /// WEB-03's own success criterion 3: a test asserting the correspondence between
    /// the declared tool-calling capability and whether a tool-calling request path
    /// actually exists. `LlmRequest`'s complete field set is `id`, `model`, `prompt`,
    /// `attachments`, `stream`, `metadata` — no field through which a tool definition
    /// could travel — so the request surface never supports tool calling today, and
    /// every shipped adapter's declared capability must match that fact.
    ///
    /// Extended for D-12: no shipped adapter's `generate()` ever returns a populated
    /// `function_call` on its `LlmResponse` — every occurrence of a populated one in
    /// the workspace is in a test double under `tests/` — so `supports_function_calling`
    /// is pinned here too, by the same test, so the two flags cannot drift apart
    /// independently again.
    #[test]
    fn test_capabilities_tool_calling_matches_request_surface() {
        // `LlmRequest` has no tools field today, so no adapter's request path can
        // carry a tool call. This is the single source of truth the correspondence
        // below is checked against.
        const REQUEST_SURFACE_SUPPORTS_TOOL_CALLING: bool = false;
        // No shipped adapter's `generate()` ever returns `Some(FunctionCall)` on the
        // `LlmResponse` it builds. This is the single source of truth the second
        // correspondence below is checked against (D-12).
        const RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING: bool = false;

        let openai = OpenAIAdapter::new(OpenAIConfig::new("test-key".to_string())).unwrap();
        let anthropic = AnthropicAdapter::new(AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        ))
        .unwrap();
        let deepseek = DeepSeekAdapter::new(DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        ))
        .unwrap();

        for (name, declared_tool_calling, declared_function_calling) in [
            (
                "openai",
                openai.get_capabilities().supports_tool_calling,
                openai.get_capabilities().supports_function_calling,
            ),
            (
                "anthropic",
                anthropic.get_capabilities().supports_tool_calling,
                anthropic.get_capabilities().supports_function_calling,
            ),
            (
                "deepseek",
                deepseek.get_capabilities().supports_tool_calling,
                deepseek.get_capabilities().supports_function_calling,
            ),
        ] {
            assert_eq!(
                declared_tool_calling, REQUEST_SURFACE_SUPPORTS_TOOL_CALLING,
                "{name}'s declared supports_tool_calling ({declared_tool_calling}) must match whether a \
                 tool-calling request path exists on LlmRequest ({REQUEST_SURFACE_SUPPORTS_TOOL_CALLING})"
            );
            assert_eq!(
                declared_function_calling, RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING,
                "{name}'s declared supports_function_calling ({declared_function_calling}) must match \
                 whether generate() ever returns a populated function_call \
                 ({RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING})"
            );
        }
    }

    /// The assumption-delta invariant test: every shipped adapter must declare a
    /// `Some((min, max))` temperature range, never silently fall back to `None` (the
    /// framework's `[0.0, 1.0]` default). Catches a future adapter reintroducing the
    /// singular global-clamp assumption the moment it lands.
    #[test]
    fn test_every_adapter_declares_a_temperature_range() {
        let openai = OpenAIAdapter::new(OpenAIConfig::new("test-key".to_string())).unwrap();
        let anthropic = AnthropicAdapter::new(AnthropicConfig::new(
            "sk-ant-test123".to_string(),
            "https://api.anthropic.com/v1".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            4096,
        ))
        .unwrap();
        let deepseek = DeepSeekAdapter::new(DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        ))
        .unwrap();

        assert!(
            openai.get_capabilities().temperature_range.is_some(),
            "openai must declare a temperature_range"
        );
        assert!(
            anthropic.get_capabilities().temperature_range.is_some(),
            "anthropic must declare a temperature_range"
        );
        assert!(
            deepseek.get_capabilities().temperature_range.is_some(),
            "deepseek must declare a temperature_range"
        );
    }
}

/// Cross-adapter capability invariants for the six providers this phase adds
/// (PROV-02, RESEARCH.md Pitfall 4, Open Question 3).
///
/// A **sibling** module to [`capability_invariants`] above, not a widening of its
/// `cfg` gate: widening the existing module's gate would make the shipped three's
/// invariant stop compiling unless every one of `kimi`/`qwen`/`grok`/`ollama`/`gemini`/
/// `openai-compatible` were also enabled, silently disabling a test that runs today
/// under `cargo test --workspace`. This module carries its own two source-of-truth
/// constants rather than importing the sibling module's private ones, for the same
/// reason: the two modules must be independently compilable under their own feature
/// sets.
#[cfg(all(
    test,
    feature = "kimi",
    feature = "qwen",
    feature = "grok",
    feature = "ollama",
    feature = "gemini",
    feature = "openai-compatible"
))]
mod capability_invariants_new_providers {
    use crate::gemini::adapter::{GEMINI_DEFAULT_BASE_URL, GEMINI_DEFAULT_MODEL};
    use crate::gemini::{GeminiAdapter, GeminiConfig};
    use crate::grok::adapter::{GROK_DEFAULT_BASE_URL, GROK_DEFAULT_MODEL};
    use crate::grok::{GrokAdapter, GrokConfig};
    use crate::kimi::adapter::{KIMI_DEFAULT_BASE_URL, KIMI_DEFAULT_MODEL};
    use crate::kimi::{KimiAdapter, KimiConfig};
    use crate::ollama::adapter::{OLLAMA_DEFAULT_BASE_URL, OLLAMA_DEFAULT_MODEL};
    use crate::ollama::{OllamaAdapter, OllamaConfig};
    use crate::openai_compatible::{
        OpenAiCompatibleAdapter, OpenAiCompatibleCapabilitiesConfig, OpenAiCompatibleConfig,
    };
    use crate::qwen::adapter::{QWEN_DEFAULT_BASE_URL, QWEN_DEFAULT_MODEL};
    use crate::qwen::{QwenAdapter, QwenConfig};
    use paladin_ports::output::llm_port::LlmPort;

    /// This phase's own restatement of the sibling module's success criterion 3: a
    /// test asserting the correspondence between the declared tool-calling
    /// capability and whether a tool-calling request path actually exists.
    /// `LlmRequest`'s complete field set is `id`, `model`, `prompt`, `attachments`,
    /// `stream`, `metadata` — no field through which a tool definition could
    /// travel — so the request surface never supports tool calling today, and
    /// every one of the six adapters this phase adds must declare that fact
    /// truthfully.
    ///
    /// The same reasoning covers `supports_function_calling`: no adapter added by
    /// this phase ever returns a populated `function_call` on the `LlmResponse` it
    /// builds — every occurrence of a populated one in the workspace is in a test
    /// double under `tests/` — so both flags are pinned here together, by the same
    /// test, so they cannot drift apart independently (D-12).
    ///
    /// [`OpenAiCompatibleAdapter`] is constructed from an **empty** capability
    /// declaration (every field its pessimistic default) rather than a populated
    /// one, so this test covers the exact configuration an operator gets when they
    /// declare nothing (D-04) — the path most likely to regress if a future change
    /// to the generic provider's defaulting logic reintroduces an over-reporting
    /// capability flag (RESEARCH.md Pitfall 4/5).
    #[test]
    fn test_new_adapter_capabilities_match_request_surface() {
        // `LlmRequest` has no tools field today, so no adapter's request path can
        // carry a tool call. This is the single source of truth the correspondence
        // below is checked against.
        const REQUEST_SURFACE_SUPPORTS_TOOL_CALLING: bool = false;
        // No adapter this phase adds ever returns `Some(FunctionCall)` on the
        // `LlmResponse` it builds. This is the single source of truth the second
        // correspondence below is checked against (D-12).
        const RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING: bool = false;

        // A throwaway test key and a mockito-free placeholder base URL — none of
        // these adapters make a request in this test, so the URL need not resolve.
        let kimi = KimiAdapter::new(KimiConfig::new(
            "test-key".to_string(),
            KIMI_DEFAULT_BASE_URL.to_string(),
            KIMI_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let qwen = QwenAdapter::new(QwenConfig::new(
            "test-key".to_string(),
            QWEN_DEFAULT_BASE_URL.to_string(),
            QWEN_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let grok = GrokAdapter::new(GrokConfig::new(
            "test-key".to_string(),
            GROK_DEFAULT_BASE_URL.to_string(),
            GROK_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let ollama = OllamaAdapter::new(OllamaConfig::new(
            OLLAMA_DEFAULT_BASE_URL.to_string(),
            OLLAMA_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        let gemini = GeminiAdapter::new(GeminiConfig::new(
            "test-key".to_string(),
            GEMINI_DEFAULT_BASE_URL.to_string(),
            GEMINI_DEFAULT_MODEL.to_string(),
        ))
        .unwrap();
        // The pessimistic-default path (D-04): every field its own conservative
        // default, exactly what an operator gets when they declare nothing.
        let empty_capabilities = OpenAiCompatibleCapabilitiesConfig {
            supports_streaming: true,
            supports_tool_calling: false,
            supports_function_calling: false,
            supports_vision: false,
            supports_embeddings: false,
            supports_system_messages: false,
            max_context_tokens: None,
            temperature_range: None,
        };
        let openai_compatible = OpenAiCompatibleAdapter::new(OpenAiCompatibleConfig::new(
            "test-key".to_string(),
            "http://localhost:8080".to_string(),
            "test-model".to_string(),
            empty_capabilities,
        ))
        .unwrap();

        for (name, declared_tool_calling, declared_function_calling) in [
            (
                "kimi",
                kimi.get_capabilities().supports_tool_calling,
                kimi.get_capabilities().supports_function_calling,
            ),
            (
                "qwen",
                qwen.get_capabilities().supports_tool_calling,
                qwen.get_capabilities().supports_function_calling,
            ),
            (
                "grok",
                grok.get_capabilities().supports_tool_calling,
                grok.get_capabilities().supports_function_calling,
            ),
            (
                "ollama",
                ollama.get_capabilities().supports_tool_calling,
                ollama.get_capabilities().supports_function_calling,
            ),
            (
                "gemini",
                gemini.get_capabilities().supports_tool_calling,
                gemini.get_capabilities().supports_function_calling,
            ),
            (
                "openai-compatible (empty declaration)",
                openai_compatible.get_capabilities().supports_tool_calling,
                openai_compatible
                    .get_capabilities()
                    .supports_function_calling,
            ),
        ] {
            assert_eq!(
                declared_tool_calling, REQUEST_SURFACE_SUPPORTS_TOOL_CALLING,
                "{name}'s declared supports_tool_calling ({declared_tool_calling}) must match whether a \
                 tool-calling request path exists on LlmRequest ({REQUEST_SURFACE_SUPPORTS_TOOL_CALLING})"
            );
            assert_eq!(
                declared_function_calling, RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING,
                "{name}'s declared supports_function_calling ({declared_function_calling}) must match \
                 whether generate() ever returns a populated function_call \
                 ({RESPONSE_SURFACE_SUPPORTS_FUNCTION_CALLING})"
            );
        }
    }
}
