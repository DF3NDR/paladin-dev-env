//! Input port modules — port traits for data ingestion and processing pipelines.

/// Content ingestion port.
pub mod content_input_port;
/// Document parsing port.
pub mod document_port;
/// Event listener / webhook port.
pub mod listener_port;
pub mod ml_port;
pub mod nlp_port;
/// Parley resume-trigger port (HITL-05, D-25) — core-typed only, so
/// `paladin-web` can depend on it without a default-build edge to
/// `paladin-battalion` (ADR-0031).
pub mod parley_port;
pub mod rpc_port;
