//! Waypoint storage adapters.
//!
//! Implementations of `paladin_ports::output::waypoint_port::WaypointPort`.

/// In-memory implementation, always available (no feature gate, D-01): used
/// for tests and local development.
pub mod in_memory;
