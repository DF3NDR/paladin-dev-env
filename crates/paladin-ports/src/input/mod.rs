//! Input port modules — port traits for data ingestion and processing pipelines.

/// Assistant admin port (D-28, D-31, D-46) — validate-then-publish assistant
/// definitions, no update method by construction (D-29).
pub mod assistant_admin_port;
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
/// Run event stream port (D-27, PLAT-FR-07) — core-typed only, so
/// `paladin-web` can depend on it without a default-build edge to
/// `paladin-battalion` (ADR-0031).
pub mod run_event_stream_port;
/// Run submission port (D-12) — core-typed only, so `paladin-web` can
/// depend on it without a default-build edge to `paladin-battalion`
/// (ADR-0031).
pub mod run_submission_port;
