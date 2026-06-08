// ── Sub-modules ────────────────────────────────────────────────────────────────
#[allow(missing_docs)]
pub mod agents;
#[allow(missing_docs)]
pub mod arsenal;
#[allow(missing_docs)]
pub mod citadel;
#[allow(missing_docs)]
pub mod env_utils;
#[allow(missing_docs)]
pub mod file_storage;
#[allow(missing_docs)]
pub mod herald;
#[allow(missing_docs)]
pub mod notifications;
#[allow(missing_docs)]
pub mod queue;
#[allow(missing_docs)]
pub mod scheduler;
#[allow(missing_docs)]
pub mod settings;
#[allow(missing_docs)]
pub mod setup;
#[allow(missing_docs)]
pub mod user_config;
#[allow(missing_docs)]
pub mod web_server;

// ── Re-exports ─────────────────────────────────────────────────────────────────
// Kept for backwards compatibility — consumers can write
// `use paladin::config::XxxConfig;` without going through a sub-module path.
pub use crate::config::herald::{
    HeraldConfig, JsonHeraldConfig, MarkdownHeraldConfig, TableHeraldConfig,
};
// Vision configuration types live in the paladin-llm crate (Task 5.0)
pub use crate::config::agents::AgentDefinition;
pub use crate::config::arsenal::{ArsenalConfig, MCPServerConfig};
pub use crate::config::citadel::CitadelConfig;
pub use crate::config::file_storage::FileStorageConfig;
#[cfg(feature = "notifications")]
pub use crate::config::notifications::NotificationConfig;
pub use crate::config::queue::QueueConfig;
pub use crate::config::scheduler::SchedulerConfig;
pub use crate::config::web_server::{MessageServiceSettings, ServerConfig, SourceConfig};
pub use paladin_llm::config::vision::{VisionConfig, VisionProviderConfig, VisionRetryConfig};
// Garrison, Sanctum, RAG and MemoryExtraction config types live in the paladin-memory crate (Task 6.0)
pub use paladin_memory::config::garrison::GarrisonSettings;
pub use paladin_memory::config::rag::{
    MemoryExtractionConfig, MemoryExtractionStrategy, RagConfig,
};
pub use paladin_memory::config::sanctum::{QdrantSanctumConfig, SanctumAdapterType, SanctumConfig};
// LLM configuration types live in the paladin-llm crate (Task 5.0)
pub use paladin_llm::config::llm::{LlmConfig, LlmProviderConfig};
// Settings struct — the top-level application configuration entry-point
pub use settings::Settings;
