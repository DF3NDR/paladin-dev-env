//! Memory services — application-layer orchestration over garrison and sanctum.
//!
//! Services depend only on port traits and contain no concrete adapter imports.

pub mod memory_extraction_service;
pub use memory_extraction_service::{
    ExtractedMemory, MemoryExtractionService, MemoryExtractionStrategy,
};

pub mod rag_retrieval_service;
pub use rag_retrieval_service::{
    RagConfig, RagRetainedMemory, RagRetrievalError, RagRetrievalResult, RagRetrievalService,
    RetrievalTrigger, ShedItem, retrieve_context_with_timeout,
};
