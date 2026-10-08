//! Cadence storage adapters (PACE-02, D-07).
//!
//! Implementations of `paladin_ports::output::cadence_port::CadencePort`, the
//! shared rate-pacing state a provider 429 opens a gate in. Mirrors
//! `node_cache/`'s module layout: an always-available in-memory backend now;
//! the Redis backend ([`redis`], behind the `redis-cadence` feature, 43-07) shares
//! pacing state across a worker fleet; [`contract_tests`] is the shared contract suite
//! every backend runs unchanged (43-05, then 43-07 and 43-08); [`resilient`] composes a shared
//! backend with an in-process fallback so an outage degrades pacing instead of failing a call.

/// In-process implementation, always available (no feature gate, mirroring
/// `node_cache::in_memory`): the default backend (D-08), process-wide when one
/// instance is shared by every composition root.
pub mod in_memory;

/// Shared `CadencePort` contract suite: one clause per pacing rule, run unchanged
/// by every backend (plain, not `#[cfg(test)]`, mirroring `node_cache`).
pub mod contract_tests;

/// The one capturing logger the cadence tests share (`log::set_logger` is once per process).
#[cfg(test)]
pub(crate) mod test_logger;

/// Redis implementation: one atomic server-clock Lua script per operation, shared by every worker
/// pointed at the same server (PACE-03, D-09), behind the `redis-cadence` feature.
#[cfg(feature = "redis-cadence")]
pub mod redis;

/// Composite that serves from a shared primary while it answers and from a stricter in-process
/// fallback while it does not -- Redis unavailability degrades pacing, never an LLM call
/// (PACE-05, D-05). Always compiled: the primary is a `dyn CadencePort`.
pub mod resilient;

pub use in_memory::{DEFAULT_KEY_CAPACITY, InMemoryCadence};
#[cfg(feature = "redis-cadence")]
pub use redis::{
    CADENCE_GATE_LUA, CADENCE_RECORD_429_LUA, CADENCE_RECORD_SUCCESS_LUA,
    DEFAULT_CADENCE_KEY_PREFIX, RedisCadence, RedisCadenceConfig,
};
pub use resilient::{DEFAULT_PROBE_INTERVAL, ResilientCadence};
