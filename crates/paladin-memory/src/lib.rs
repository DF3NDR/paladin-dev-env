//! # paladin-memory
//!
//! Memory adapters for the Paladin framework.
//!
//! This crate provides two categories of memory storage:
//!
//! - **Garrison** (`garrison` module): Conversation history storage adapters.
//!   - [`garrison::InMemoryGarrison`] — always available, zero-dependency in-process store.
//!   - `garrison::SqliteGarrison` — persistent SQLite-backed store (requires feature `sqlite`).
//!   - `garrison::TiktokenCounter` / `garrison::TokenCounter` — token counting utilities
//!     (requires feature `content-processing`).
//!
//! - **Sanctum** (`sanctum` module): Vector / semantic memory adapters.
//!   - [`sanctum::InMemorySanctum`] — always available, in-process vector store.
//!   - `sanctum::QdrantSanctumAdapter` — production-grade Qdrant-backed store (requires feature `qdrant`).
//!
//! - **Services** (`services` module): Application-layer memory orchestration.
//!   - [`services::MemoryExtractionService`] — extracts and persists memories from conversations.
//!   - [`services::RagRetrievalService`] — retrieves context for RAG pipelines.
//!
//! - **Vault** (`vault` module): Cross-thread namespaced key/value storage.
//!   - [`vault::InMemoryVault`] — always available, zero-dependency in-process store.
//!
//! ## Feature flags
//!
//! | Feature              | Enables                                          |
//! |----------------------|--------------------------------------------------|
//! | `sqlite`             | `SqliteGarrison` (depends on `sqlx`)             |
//! | `qdrant`             | `QdrantSanctumAdapter` (depends on `qdrant-client`) |
//! | `content-processing` | `TiktokenCounter`, `TokenCounter`, `TokenCounterFactory` (depends on `tiktoken-rs`) |
//!
//! No features are enabled by default.

#![deny(unsafe_code)]
#![warn(missing_docs)]

/// Citadel state-persistence adapters (file-based).
#[allow(missing_docs)]
pub mod citadel;
/// Configuration types for garrison, sanctum, and memory extraction behavior.
#[allow(missing_docs)]
pub mod config;
/// Garrison adapters and supporting utilities.
#[allow(missing_docs)]
pub mod garrison;
/// Convenience re-exports for commonly used memory types.
#[allow(missing_docs)]
pub mod prelude;
/// Sanctum adapters and semantic-memory abstractions.
#[allow(missing_docs)]
pub mod sanctum;
/// Application-layer services for memory extraction and retrieval.
#[allow(missing_docs)]
pub mod services;
/// Vault adapters -- cross-thread namespaced key/value storage (D-18).
#[allow(missing_docs)]
pub mod vault;
