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
