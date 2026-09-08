//! Run storage adapters.
//!
//! Implementations of `paladin_ports::output::run_repository_port::RunRepositoryPort`,
//! mirroring `crate::waypoint`'s module layout (D-03).

/// In-memory implementation, always available (no feature gate): used for
/// tests, local development, and this plan's tracer end-to-end test.
pub mod in_memory;

/// Shared `RunRepositoryPort` contract suite (D-04, D-17, D-18), mirroring
/// `crate::waypoint::contract_tests`. Plain module (not `#[cfg(test)]`) so
/// both unit tests inside each backend crate and future integration tests
/// can call it.
pub mod contract_tests;

/// SQLite implementation, behind the `sqlite` feature (D-03).
#[cfg(feature = "sqlite")]
pub mod sqlite;
