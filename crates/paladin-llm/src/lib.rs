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
/// Services that compose port traits — the Quartermaster prompt-budgeting service.
pub mod services;

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
    #[test]
    fn test_capabilities_tool_calling_matches_request_surface() {
        // `LlmRequest` has no tools field today, so no adapter's request path can
        // carry a tool call. This is the single source of truth the correspondence
        // below is checked against.
        const REQUEST_SURFACE_SUPPORTS_TOOL_CALLING: bool = false;

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

        for (name, declared) in [
            ("openai", openai.get_capabilities().supports_tool_calling),
            (
                "anthropic",
                anthropic.get_capabilities().supports_tool_calling,
            ),
            (
                "deepseek",
                deepseek.get_capabilities().supports_tool_calling,
            ),
        ] {
            assert_eq!(
                declared, REQUEST_SURFACE_SUPPORTS_TOOL_CALLING,
                "{name}'s declared supports_tool_calling ({declared}) must match whether a \
                 tool-calling request path exists on LlmRequest ({REQUEST_SURFACE_SUPPORTS_TOOL_CALLING})"
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
