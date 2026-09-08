//! Run trace storage adapters (OBS-02, D-17).
//!
//! Implementations of `paladin_ports::output::run_trace_port::RunTracePort`.

use paladin_core::platform::container::trace::TraceEvent;

/// In-memory implementation, always available (no feature gate, mirroring
/// `waypoint`'s D-01 precedent): used for tests and local development.
pub mod in_memory;

/// Shared `RunTracePort` contract suite: generic async functions every
/// backend runs, unchanged, from its own `#[tokio::test]`s. See
/// `contract_tests::run_all` for a single-call smoke aggregate.
pub mod contract_tests;

/// SQLite implementation of `RunTracePort`, over migration `006`.
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// The superstep a `TraceRecord`'s row is filed under for
/// `RunTracePort::prune_thread`'s `(thread_id, superstep < ?)` predicate.
///
/// Only five of `TraceEvent`'s twelve variants carry an explicit
/// `superstep` field (`SuperstepStarted`, `NodeStarted`, `NodeFinished`,
/// `DeltaMerged`, `WaypointSaved`); `RunFinished` uses its own
/// `total_supersteps` (the run's final superstep count, so a completed
/// run's `RunFinished` row is eligible for pruning at the same boundary as
/// its own last superstep). Every other variant (`RunStarted`,
/// `NodeProgress`, `EdgeEvaluated`, `ParleyRaised`, `FallbackHop`,
/// `MiddlewareEvent`) has no superstep concept of its own and is stamped
/// `0`, the oldest possible bucket -- so a `prune_thread(thread, N)` call
/// with `N > 0` always removes these alongside genuinely stale superstep
/// rows, never protects them past the run's own progress. `#[non_exhaustive]`
/// on `TraceEvent` means this match always needs (and has) a wildcard arm.
pub(crate) fn superstep_of(event: &TraceEvent) -> u64 {
    match event {
        TraceEvent::SuperstepStarted { superstep, .. }
        | TraceEvent::NodeStarted { superstep, .. }
        | TraceEvent::NodeFinished { superstep, .. }
        | TraceEvent::DeltaMerged { superstep, .. }
        | TraceEvent::WaypointSaved { superstep, .. } => *superstep,
        TraceEvent::RunFinished {
            total_supersteps, ..
        } => *total_supersteps,
        _ => 0,
    }
}
