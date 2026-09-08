//! Run queue storage adapters.
//!
//! Implementations of `paladin_ports::output::run_queue_port::RunQueuePort`.

/// In-memory, lease-aware implementation, always available (no feature
/// gate, D-08's Tier 1 twin): a `VecDeque` of queued runs plus a map of
/// in-flight leases with real expiry semantics -- not stubbed, since plan
/// 27-03's queue contract suite runs against this adapter unchanged.
pub mod in_memory;

/// Shared `RunQueuePort` contract suite (D-06): one generic async function
/// per contract clause, run unchanged by both `in_memory` and `redis`.
pub mod contract_tests;
