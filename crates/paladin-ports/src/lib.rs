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
// Some port files contain cross-crate doc links (e.g. `crate::infrastructure::…`)
// that resolved in the original `paladin` crate but are unavailable in this
// isolated crate.  Downgrade from deny → warn so `cargo doc` still succeeds.
#![warn(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::redundant_explicit_links)]

/// Input port traits — defines how external stimuli enter the application.
pub mod input;
/// Output port traits — defines how the application reaches external systems.
pub mod output;
