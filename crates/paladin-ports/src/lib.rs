//! # paladin-ports
//!
//! Port trait definitions for the Paladin multi-agent orchestration framework.
//!
//! This crate sits at the **application boundary** of the hexagonal architecture,
//! defining the abstract contracts (ports) that connect the core domain to external
//! infrastructure. It depends only on `paladin-core` and standard utility crates —
//! never on infrastructure SDKs, database drivers, or LLM provider libraries.
//!
//! ## Hexagonal Architecture Position
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                  Infrastructure Layer                    │
//! │   (adapters: OpenAI, SQLite, Redis, MinIO, MCP, …)      │
//! │              implements ↓ these ports                    │
//! ├─────────────────────────────────────────────────────────┤
//! │                  paladin-ports  ◄── YOU ARE HERE         │
//! │   output/: LlmPort, GarrisonPort, CitadelPort, …        │
//! │   input/:  ContentInputPort, DocumentPort, …            │
//! │              depends on ↓                               │
//! ├─────────────────────────────────────────────────────────┤
//! │                  paladin-core                            │
//! │   (domain entities, value objects, aggregates)          │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`output`] — Port traits implemented by infrastructure adapters (LLM providers,
//!   storage backends, queue brokers, tool registries, etc.)
//! - [`input`] — Port traits representing API boundaries into the application
//!   (content ingestion, RPC services, document processing, etc.)
//!
//! ## Dependency Policy
//!
//! `paladin-ports` must **never** introduce dependencies on:
//! - LLM provider SDKs (`openai`, `anthropic`, etc.)
//! - Database drivers (`sqlx`, `redis`, etc.)
//! - Storage clients (`aws-sdk-s3`, `minio`, etc.)
//! - HTTP clients (`reqwest`) in an infrastructure capacity
//!
//! All such dependencies belong exclusively in the infrastructure adapters that
//! implement these port traits.

#![warn(missing_docs)]
// Broken intra-doc links are reported at `warn` and nothing in this crate suppresses
// them: both ADR-0033 bars (`make doc-check`) fail on any diagnostic, so a link to a
// higher layer is written as a plain code span and a link to `paladin-core` carries
// its explicit path.
#![warn(rustdoc::broken_intra_doc_links)]

/// Input port traits — defines how external stimuli enter the application.
pub mod input;
/// Output port traits — defines how the application reaches external systems.
pub mod output;
