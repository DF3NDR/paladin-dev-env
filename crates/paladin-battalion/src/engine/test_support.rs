//! Engine unit-test doubles (Phase 22 Plan 05).
//!
//! `#[cfg(test)]`-only: [`RecordingWaypointStore`] and [`CountingFunctionNode`]
//! are the two doubles every later engine plan's unit tests assert against.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::output::waypoint_port::{
    ThreadSummary, WaypointError, WaypointPort, WaypointSummary,
};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

use crate::engine::node::{NodeContext, NodeError, StateNode};

/// A [`WaypointPort`] test double wrapping an [`InMemoryWaypointStore`],
/// additionally recording every `save` call and able to fail its NEXT save
/// on demand (one-shot; auto-resets after firing).
#[derive(Default)]
pub struct RecordingWaypointStore {
    inner: InMemoryWaypointStore,
    save_calls: AtomicUsize,
    fail_next_save: AtomicBool,
}

impl RecordingWaypointStore {
    /// Construct a new, empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Arrange for the NEXT `save` call to fail with
    /// `WaypointError::Backend`. Resets automatically once it has fired, so
    /// only that one call fails.
    pub fn fail_next_save(&self) {
        self.fail_next_save.store(true, Ordering::SeqCst);
    }

    /// How many times `save` has been called (successful or not).
    pub fn save_call_count(&self) -> usize {
        self.save_calls.load(Ordering::SeqCst)
    }

    /// All Waypoints saved for `thread`, newest-first (delegates to the
    /// inner store's `history`).
    pub async fn saved_waypoints(&self, thread: &ThreadId) -> Vec<Waypoint> {
        let summaries = self
            .inner
            .history(thread, None, None)
            .await
            .unwrap_or_default();
        let mut out = Vec::with_capacity(summaries.len());
        for summary in summaries {
            if let Ok(Some(wp)) = self.inner.get(thread, &summary.waypoint_id).await {
                out.push(wp);
            }
        }
        out
    }
}

#[async_trait]
impl WaypointPort for RecordingWaypointStore {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError> {
        self.save_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_next_save.swap(false, Ordering::SeqCst) {
            return Err(WaypointError::Backend {
                source: Box::<dyn std::error::Error + Send + Sync>::from(
                    "simulated save failure (RecordingWaypointStore::fail_next_save)",
                ),
            });
        }
        self.inner.save(wp).await
    }

    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError> {
        self.inner.latest(thread).await
    }

    async fn get(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<Option<Waypoint>, WaypointError> {
        self.inner.get(thread, id).await
    }

    async fn history(
        &self,
        thread: &ThreadId,
        limit: Option<u32>,
        before: Option<WaypointId>,
    ) -> Result<Vec<WaypointSummary>, WaypointError> {
        self.inner.history(thread, limit, before).await
    }

    async fn list_threads(
        &self,
        limit: Option<u32>,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<ThreadSummary>, WaypointError> {
        self.inner.list_threads(limit, before).await
    }

    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError> {
        self.inner.delete_thread(thread).await
    }
}

/// A closure computing a [`CountingFunctionNode`]'s delta from its
/// zero-indexed run number and the Battlefield snapshot it observed.
type DeltaFn = dyn Fn(usize, &Battlefield) -> StateDelta + Send + Sync;

/// A [`StateNode`] test double: records how many times it ran, the raw
/// pointer address of the Battlefield snapshot it observed on each run (for
/// asserting `Arc`-shared-snapshot identity across concurrently-executing
/// peers), and returns a delta computed by a caller-supplied closure over
/// the zero-indexed run number and the observed snapshot.
pub struct CountingFunctionNode {
    run_count: Arc<AtomicUsize>,
    observed_ptrs: Arc<Mutex<Vec<usize>>>,
    delta_fn: Arc<DeltaFn>,
}

impl CountingFunctionNode {
    /// Construct a node whose delta is computed by `delta_fn(run_index,
    /// snapshot)`, where `run_index` is 0 on the node's first execution, 1
    /// on its second, and so on.
    pub fn new(
        delta_fn: impl Fn(usize, &Battlefield) -> StateDelta + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            run_count: Arc::new(AtomicUsize::new(0)),
            observed_ptrs: Arc::new(Mutex::new(Vec::new())),
            delta_fn: Arc::new(delta_fn),
        })
    }

    /// Convenience: a node that always writes the same fixed value to one
    /// field, ignoring the observed state.
    pub fn fixed(
        field: paladin_core::platform::container::battlefield::FieldName,
        value: serde_json::Value,
    ) -> Arc<Self> {
        Self::new(move |_run, _state| {
            let mut delta = StateDelta::new();
            delta.set_raw(field.clone(), value.clone());
            delta
        })
    }

    /// How many times this node has run so far.
    pub fn run_count(&self) -> usize {
        self.run_count.load(Ordering::SeqCst)
    }

    /// The raw pointer address (as `usize`) of the Battlefield snapshot
    /// observed on each run, in run order. Two nodes sharing the same
    /// per-superstep `Arc<Battlefield>` snapshot report identical addresses
    /// for runs in the same superstep.
    pub fn observed_ptrs(&self) -> Vec<usize> {
        self.observed_ptrs.lock().unwrap().clone()
    }
}

#[async_trait]
impl StateNode for CountingFunctionNode {
    async fn run(&self, state: &Battlefield, _ctx: &NodeContext) -> Result<StateDelta, NodeError> {
        let run_index = self.run_count.fetch_add(1, Ordering::SeqCst);
        self.observed_ptrs
            .lock()
            .unwrap()
            .push(state as *const Battlefield as usize);
        Ok((self.delta_fn)(run_index, state))
    }
}

/// A [`StateNode`] test double that tracks the maximum number of instances
/// running concurrently, for asserting a `Semaphore`-bounded parallelism
/// limit. Each run sleeps briefly to force overlap with sibling nodes in the
/// same superstep before returning a fixed delta.
pub struct ConcurrencyTrackingNode {
    field: paladin_core::platform::container::battlefield::FieldName,
    value: serde_json::Value,
    in_flight: Arc<AtomicUsize>,
    max_seen: Arc<AtomicUsize>,
    hold: std::time::Duration,
}

impl ConcurrencyTrackingNode {
    /// Construct a node sharing `in_flight`/`max_seen` counters with its
    /// siblings in the same test graph.
    pub fn new(
        field: paladin_core::platform::container::battlefield::FieldName,
        value: serde_json::Value,
        in_flight: Arc<AtomicUsize>,
        max_seen: Arc<AtomicUsize>,
        hold: std::time::Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            field,
            value,
            in_flight,
            max_seen,
            hold,
        })
    }
}

#[async_trait]
impl StateNode for ConcurrencyTrackingNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<StateDelta, NodeError> {
        let now_in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_seen.fetch_max(now_in_flight, Ordering::SeqCst);
        tokio::time::sleep(self.hold).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta)
    }
}

/// A [`StateNode`] test double that always fails with a fixed error message,
/// for exercising the engine's node-execution-error path.
pub struct FailingFunctionNode {
    message: String,
}

impl FailingFunctionNode {
    /// Construct a node that always returns `NodeError(message)`.
    pub fn new(message: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            message: message.into(),
        })
    }
}

#[async_trait]
impl StateNode for FailingFunctionNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<StateDelta, NodeError> {
        Err(NodeError(self.message.clone()))
    }
}

/// A [`StateNode`] wrapper that awaits `tokio::task::yield_now()` a
/// caller-supplied number of times before delegating to `inner`, used to
/// perturb concurrent scheduling for the ENG-FR-08 randomized-scheduling
/// determinism tests (Phase 22 Plan 07): a different yield count per node
/// per iteration forces different real completion interleavings across
/// iterations, so a determinism assertion that only holds by accident of a
/// single-threaded runtime's incidental scheduling is caught rather than
/// passing silently.
pub struct YieldingNode {
    inner: Arc<dyn StateNode>,
    yields: usize,
}

impl YieldingNode {
    /// Construct a node that yields `yields` times before running `inner`.
    pub fn new(inner: Arc<dyn StateNode>, yields: usize) -> Arc<Self> {
        Arc::new(Self { inner, yields })
    }
}

#[async_trait]
impl StateNode for YieldingNode {
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<StateDelta, NodeError> {
        for _ in 0..self.yields {
            tokio::task::yield_now().await;
        }
        self.inner.run(state, ctx).await
    }
}

/// Shuffle `items` in place with a seeded, reproducible RNG (ENG-FR-08): a
/// determinism test perturbs node spawn order (e.g. the order a
/// [`crate::engine::graph::WarGraph`]'s entries are declared) by shuffling
/// with a different `seed` per iteration, so a failure is reproducible from
/// the seed printed in the assertion message rather than depending on
/// whatever order the process happened to run in.
pub fn shuffle_seeded<T>(items: &mut [T], seed: u64) {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    items.shuffle(&mut rng);
}
