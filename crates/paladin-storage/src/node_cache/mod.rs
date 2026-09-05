//! Node cache storage adapters (Doc 04 FT-FR-18…20, D-27).
//!
//! Implementations of `paladin_ports::output::node_cache_port::NodeCachePort`.
//! Mirrors `waypoint/`'s module layout: an always-available in-memory
//! backend, a shared contract suite every backend runs unchanged, and (Task
//! 2) a feature-gated Redis backend.

/// In-memory implementation, always available (no feature gate, mirroring
/// `waypoint::in_memory`'s D-01 precedent): used for tests, local
/// development, and any deployment that has not opted into a durable cache
/// backend.
pub mod in_memory;

/// Shared `NodeCachePort` contract suite (mirroring `waypoint::contract_tests`'s
/// D-09 precedent): one generic async function per contract clause, run
/// unchanged by every backend's own `#[tokio::test]`s.
pub mod contract_tests;

pub use in_memory::InMemoryNodeCache;
