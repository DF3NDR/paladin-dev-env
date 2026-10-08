//! Cadence storage adapters (PACE-02, D-07).
//!
//! Implementations of `paladin_ports::output::cadence_port::CadencePort`, the
//! shared rate-pacing state a provider 429 opens a gate in. Mirrors
//! `node_cache/`'s module layout: an always-available in-memory backend now;
//! the Redis backend arrives behind the `redis-cache`-style `redis-cadence`
//! feature in a later Phase 43 plan (43-07); [`contract_tests`] is the shared
//! contract suite every backend runs unchanged (43-05, then 43-07 and 43-08).

/// In-process implementation, always available (no feature gate, mirroring
/// `node_cache::in_memory`): the default backend (D-08), process-wide when one
/// instance is shared by every composition root.
pub mod in_memory;

/// Shared `CadencePort` contract suite: one clause per pacing rule, run unchanged
/// by every backend (plain, not `#[cfg(test)]`, mirroring `node_cache`).
pub mod contract_tests;

pub use in_memory::{DEFAULT_KEY_CAPACITY, InMemoryCadence};
