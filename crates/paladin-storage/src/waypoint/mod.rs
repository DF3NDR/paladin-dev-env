//! Waypoint storage adapters.
//!
//! Implementations of `paladin_ports::output::waypoint_port::WaypointPort`.

/// In-memory implementation, always available (no feature gate, D-01): used
/// for tests and local development.
pub mod in_memory;

/// Shared `WaypointPort` contract suite (D-09): generic async functions every
/// backend runs, unchanged, from its own `#[tokio::test]`s. See
/// `contract_tests::run_all` for a single-call smoke aggregate.
pub mod contract_tests;

/// SQLite implementation of `WaypointPort`, over a versioned migration
/// (Plan 22-06, ENG-05).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// Credential redaction for connection-string-derived error text (T-22-18).
/// Consumed by the `sqlite` backend now; the `postgres` backend (Task 2)
/// will widen this gate to `any(feature = "sqlite", feature = "postgres")`
/// once that feature exists in this crate's `Cargo.toml`.
#[cfg(feature = "sqlite")]
pub(crate) mod redact;
