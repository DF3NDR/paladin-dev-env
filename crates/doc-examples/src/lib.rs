//! Compile-verified examples for the Paladin documentation book.
//!
//! Each `// ANCHOR: name` / `// ANCHOR_END: name` region is pulled into a
//! markdown guide via mdBook's `{{#include}}`. Because this crate is part of the
//! workspace, `cargo check -p paladin-doc-examples` guarantees every documented
//! example compiles against the current APIs.

pub mod support;

pub mod bridge;
pub mod content;
pub mod deployment_topologies;
pub mod fault_tolerance;
pub mod http_service_host;
pub mod orchestration;
pub mod queue_worker;
pub mod readme;
pub mod sidecar;
