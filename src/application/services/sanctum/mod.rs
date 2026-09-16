//! Sanctum Use Cases — re-exported from `paladin-memory`.
//!
//! Memory extraction and RAG retrieval services have been extracted into the
//! `paladin-memory` crate. This module provides backward-compatible re-exports.

pub use paladin_memory::services::{
    ExtractedMemory, MemoryExtractionService, MemoryExtractionStrategy, RagConfig,
    RagRetainedMemory, RagRetrievalError, RagRetrievalResult, RagRetrievalService,
    RetrievalTrigger, ShedItem, rag_omission_marker, retrieve_context_with_timeout,
};

/// Memory extraction service (backward-compatible sub-module path).
pub mod memory_extraction_service {
    pub use paladin_memory::services::{
        ExtractedMemory, MemoryExtractionService, MemoryExtractionStrategy,
    };
}

/// RAG retrieval service (backward-compatible sub-module path).
pub mod rag_retrieval_service {
    pub use paladin_memory::services::{
        RagConfig, RagRetainedMemory, RagRetrievalError, RagRetrievalResult, RagRetrievalService,
        RetrievalTrigger, ShedItem, rag_omission_marker, retrieve_context_with_timeout,
    };
}
