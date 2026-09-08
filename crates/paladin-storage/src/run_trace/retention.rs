//! `run_traces` retention/cleanup routine (D-17): a small sibling to
//! `waypoint::retention::prune`, NOT a generalisation of it -- see
//! `run_trace/mod.rs`'s own doc comment and 28-RESEARCH.md Q14 / Pitfall 3
//! for why `waypoint::retention::prune`'s `&dyn WaypointPort` typing cannot
//! accept [`RunTracePort`] too.
//!
//! This routine has no opinion on what `before_superstep_for` should return
//! for a given thread -- that policy (deriving a boundary from the SAME
//! `max_age_days` / `max_waypoints_per_thread` bounds
//! `WaypointRetentionService` already applies to Waypoints) lives at the
//! application layer, in `src/application/services/waypoint_retention.rs`
//! (X-01: the decision crosses into this adapter as an argument, the
//! adapter carries no copy of it).

use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};

/// Prune `run_traces` for every thread in `threads`, calling
/// `before_superstep_for(thread)` once per thread to get that thread's
/// prune boundary and forwarding it unchanged to
/// [`RunTracePort::prune_thread`]. Returns the TOTAL number of records
/// removed across every thread.
///
/// An empty `threads` slice is a no-op that returns `Ok(0)` without calling
/// the port at all.
pub async fn prune(
    port: &dyn RunTracePort,
    threads: &[ThreadId],
    before_superstep_for: impl Fn(&ThreadId) -> u64,
) -> Result<u64, RunTraceError> {
    let mut total_removed = 0u64;
    for thread in threads {
        let before_superstep = before_superstep_for(thread);
        total_removed += port.prune_thread(thread, before_superstep).await?;
    }
    Ok(total_removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_trace::contract_tests::sample_record;
    use crate::run_trace::in_memory::InMemoryRunTraceStore;

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    #[tokio::test]
    async fn prune_calls_before_superstep_for_once_per_thread_and_sums_removed() {
        let store = InMemoryRunTraceStore::new();
        let a = thread("run-trace-retention-a");
        let b = thread("run-trace-retention-b");

        for superstep in 0..5u64 {
            store
                .append(&[sample_record(&a, superstep + 1, superstep)])
                .await
                .unwrap();
            store
                .append(&[sample_record(&b, superstep + 1, superstep)])
                .await
                .unwrap();
        }

        let total = prune(&store, &[a.clone(), b.clone()], |_| 3).await.unwrap();
        // 3 removed per thread (superstep 0, 1, 2), two threads.
        assert_eq!(total, 6);

        assert_eq!(store.read(&a, 0, 100).await.unwrap().len(), 2);
        assert_eq!(store.read(&b, 0, 100).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn prune_derives_a_different_boundary_per_thread() {
        let store = InMemoryRunTraceStore::new();
        let a = thread("run-trace-retention-per-thread-a");
        let b = thread("run-trace-retention-per-thread-b");

        for superstep in 0..5u64 {
            store
                .append(&[sample_record(&a, superstep + 1, superstep)])
                .await
                .unwrap();
            store
                .append(&[sample_record(&b, superstep + 1, superstep)])
                .await
                .unwrap();
        }

        let bounds: std::collections::HashMap<ThreadId, u64> =
            [(a.clone(), 1u64), (b.clone(), 4u64)].into_iter().collect();
        let total = prune(&store, &[a.clone(), b.clone()], |t| {
            bounds.get(t).copied().unwrap_or(0)
        })
        .await
        .unwrap();

        assert_eq!(total, 1 + 4);
        assert_eq!(store.read(&a, 0, 100).await.unwrap().len(), 4);
        assert_eq!(store.read(&b, 0, 100).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn prune_empty_thread_list_is_a_no_op() {
        let store = InMemoryRunTraceStore::new();
        let total = prune(&store, &[], |_| 0).await.unwrap();
        assert_eq!(total, 0);
    }
}
