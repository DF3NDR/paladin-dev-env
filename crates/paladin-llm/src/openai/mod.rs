//! OpenAI LLM adapter module.
//!
//! Exports [`OpenAIAdapter`](crate::openai::OpenAIAdapter) (and [`OpenAIConfig`](crate::openai::OpenAIConfig)) for use with GPT-3.5, GPT-4,
//! and other OpenAI models.
//!
//! Enable the `openai-embeddings` feature flag to also get
//! `OpenAIEmbeddingAdapter`.

pub mod adapter;

#[cfg(feature = "openai-embeddings")]
pub mod embedding;

#[cfg(feature = "vision")]
pub mod vision;

pub use adapter::{OpenAIAdapter, OpenAIConfig};

#[cfg(feature = "openai-embeddings")]
pub use embedding::{OpenAIEmbeddingAdapter, OpenAIEmbeddingConfig};

#[cfg(feature = "vision")]
pub use vision::{VisionConfig, VisionProviderConfig, VisionRetryConfig};
