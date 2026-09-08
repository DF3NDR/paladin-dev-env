//! Run storage adapters.
//!
//! Implementations of `paladin_ports::output::run_repository_port::RunRepositoryPort`,
//! mirroring `crate::waypoint`'s module layout (D-03).

/// In-memory implementation, always available (no feature gate): used for
/// tests, local development, and this plan's tracer end-to-end test.
pub mod in_memory;
