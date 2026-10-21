//! Shared OpenAI-compatible protocol engine (D-05, Phase 17).
//!
//! One engine owns the request/response types, streaming chunk assembly,
//! retry and error mapping shared by every OpenAI-compatible provider
//! preset (Kimi, and future presets under this feature gate). Presets supply
//! only `base_url`, credential, default model, curated model-list fallback
//! and a capabilities block.

/// Generalized OpenAI-compatible wire types (request/response/stream shapes).
pub mod types;

/// The shared engine: request building, HTTP transport, retry, error
/// mapping, credential redaction and memoized model-list resolution.
pub mod engine;

pub use crate::reasoning::ReasoningWire;
pub use engine::{CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters};

/// Test-only plumbing shared by the five presets' wire tests; the helpers themselves live in
/// the crate-level [`crate::test_support`] so the bespoke adapters' tests can use them without
/// compiling this module.
#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) use crate::test_support::*;
}
