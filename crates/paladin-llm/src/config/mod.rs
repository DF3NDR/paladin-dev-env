//! Per-domain configuration types for the paladin-llm crate.

pub mod bridge;
pub(crate) mod layers;
pub mod llm;
pub mod vision;

pub use llm::{LlmConfig, LlmProviderConfig};
pub use vision::{VisionConfig, VisionProviderConfig, VisionRetryConfig};
