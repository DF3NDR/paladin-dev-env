//! Convenience re-exports for the most commonly used types in `paladin-memory`.
//!
//! Import everything at once with `use paladin_memory::prelude::*;`.

// Garrison
pub use crate::garrison::InMemoryGarrison;
#[cfg(feature = "sqlite")]
pub use crate::garrison::SqliteGarrison;
#[cfg(feature = "content-processing")]
pub use crate::garrison::{TiktokenCounter, TokenCounter, TokenCounterFactory};

// Sanctum
#[cfg(feature = "qdrant")]
pub use crate::sanctum::QdrantSanctumAdapter;
pub use crate::sanctum::{InMemorySanctum, InMemorySanctumConfig};

// Vault
pub use crate::vault::InMemoryVault;
pub use crate::vault::SemanticVault;
#[cfg(feature = "sqlite")]
pub use crate::vault::SqliteVault;

// Services
pub use crate::services::{
    MemoryExtractionService, MemoryExtractionStrategy, RagConfig, RagRetrievalService,
};

// Config types
pub use crate::config::{
    GarrisonSettings, MemoryExtractionConfig, QdrantSanctumConfig, SanctumAdapterType,
    SanctumConfig,
};
