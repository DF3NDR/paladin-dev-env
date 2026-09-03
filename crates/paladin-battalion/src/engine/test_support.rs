//! Engine unit-test doubles (Phase 22 Plan 05).
//!
//! `#[cfg(test)]`-only: [`RecordingWaypointStore`] and [`CountingFunctionNode`]
//! are the two doubles every later engine plan's unit tests assert against.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{ThreadId, Waypoint, WaypointId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceSink, TraceSinkError};
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

    async fn delete_waypoint(
        &self,
        thread: &ThreadId,
        id: &WaypointId,
    ) -> Result<bool, WaypointError> {
        self.inner.delete_waypoint(thread, id).await
    }

    async fn prune_thread(
        &self,
        thread: &ThreadId,
        keep: &[WaypointId],
    ) -> Result<u64, WaypointError> {
        self.inner.prune_thread(thread, keep).await
    }
}

/// A closure computing a [`CountingFunctionNode`]'s [`Directive`] from its
/// zero-indexed run number, the Battlefield snapshot it observed, and its
/// [`NodeContext`] (CF-03: the vehicle for reading `ctx.muster_payload()`/
/// `ctx.task_key()` in a Muster worker-template test double).
type ContextDirectiveFn = dyn Fn(usize, &Battlefield, &NodeContext) -> Directive + Send + Sync;

/// A [`StateNode`] test double: records how many times it ran, the raw
/// pointer address of the Battlefield snapshot it observed on each run (for
/// asserting `Arc`-shared-snapshot identity across concurrently-executing
/// peers), and returns a [`Directive`] computed by a caller-supplied
/// closure over the zero-indexed run number, the observed snapshot, and the
/// execution's [`NodeContext`].
pub struct CountingFunctionNode {
    run_count: Arc<AtomicUsize>,
    observed_ptrs: Arc<Mutex<Vec<usize>>>,
    directive_fn: Arc<ContextDirectiveFn>,
}

impl CountingFunctionNode {
    /// Construct a node whose delta is computed by `delta_fn(run_index,
    /// snapshot)`, where `run_index` is 0 on the node's first execution, 1
    /// on its second, and so on. Always routes via `NextStep::Edges`
    /// (`impl From<StateDelta> for Directive`'s default) -- use
    /// [`CountingFunctionNode::with_directive`] for a node whose routing
    /// (`Goto`/`End`/`Muster`/`Parley`) is also caller-controlled, or
    /// [`CountingFunctionNode::with_context_directive`] for one that also
    /// needs its `NodeContext` (e.g. a Muster worker reading
    /// `ctx.muster_payload()`).
    pub fn new(
        delta_fn: impl Fn(usize, &Battlefield) -> StateDelta + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::with_directive(move |run, state| delta_fn(run, state).into())
    }

    /// Construct a node whose full [`Directive`] -- delta AND routing -- is
    /// computed by `directive_fn(run_index, snapshot)`, so a test can drive
    /// a node through `NextStep::Goto`/`End`/`Muster`/`Parley` (CF-02),
    /// optionally varying it by run index (e.g. a refine-loop reviewer that
    /// `Goto`es back for its first few runs, then routes via `Edges`).
    pub fn with_directive(
        directive_fn: impl Fn(usize, &Battlefield) -> Directive + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::with_context_directive(move |run, state, _ctx| directive_fn(run, state))
    }

    /// Construct a node whose full [`Directive`] is computed by
    /// `directive_fn(run_index, snapshot, ctx)`, additionally observing its
    /// [`NodeContext`] -- CF-03's vehicle for a Muster worker-template test
    /// double to read `ctx.muster_payload()`/`ctx.task_key()`.
    pub fn with_context_directive(
        directive_fn: impl Fn(usize, &Battlefield, &NodeContext) -> Directive + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            run_count: Arc::new(AtomicUsize::new(0)),
            observed_ptrs: Arc::new(Mutex::new(Vec::new())),
            directive_fn: Arc::new(directive_fn),
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
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError> {
        let run_index = self.run_count.fetch_add(1, Ordering::SeqCst);
        self.observed_ptrs
            .lock()
            .unwrap()
            .push(state as *const Battlefield as usize);
        Ok((self.directive_fn)(run_index, state, ctx))
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
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        let now_in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_seen.fetch_max(now_in_flight, Ordering::SeqCst);
        tokio::time::sleep(self.hold).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta.into())
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
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
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
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError> {
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

/// A [`PaladinPort`] test double that returns a configured output (and
/// token count) per Paladin name, and records every `execute` call, IN
/// ORDER, with the exact `(paladin_name, input)` it received (Phase 22 Plan
/// 08). This ordering-exact log is what the `resume` and E2E-1 tests use to
/// prove non-re-execution: a repeat of an already-completed node's name in
/// the log after a resume is a re-execution bug, not a coincidence.
///
/// The "Paladin name" key is `paladin.node.name` (`PaladinData::name`),
/// matching the convention `tests/helpers/mock_paladin_port.rs`'s
/// `FaultyPaladinPort` already established.
#[derive(Default)]
pub struct RecordingPaladinPort {
    outputs: Mutex<HashMap<String, (String, u32)>>,
    calls: Mutex<Vec<(String, String)>>,
}

impl RecordingPaladinPort {
    /// Construct a port with no configured outputs: every unconfigured
    /// Paladin name returns an empty output string and zero tokens.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the output string a Paladin named `name` returns, with a
    /// token count of `0`.
    pub fn set_output(&self, name: impl Into<String>, output: impl Into<String>) {
        self.set_output_with_tokens(name, output, 0);
    }

    /// Configure the output string AND reported token count a Paladin named
    /// `name` returns.
    pub fn set_output_with_tokens(
        &self,
        name: impl Into<String>,
        output: impl Into<String>,
        token_count: u32,
    ) {
        self.outputs
            .lock()
            .unwrap()
            .insert(name.into(), (output.into(), token_count));
    }

    /// The ordered call log: one `(paladin_name, input)` entry per `execute`
    /// call so far, in invocation order.
    pub fn call_log(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    /// The total number of `execute` calls made so far, across every
    /// Paladin.
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl PaladinPort for RecordingPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        let name = paladin.node.name.clone();
        self.calls
            .lock()
            .unwrap()
            .push((name.clone(), input.to_string()));

        let (output, token_count) = self
            .outputs
            .lock()
            .unwrap()
            .get(&name)
            .cloned()
            .unwrap_or_default();

        Ok(PaladinResult {
            output,
            token_count,
            execution_time_ms: 0,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("RecordingPaladinPort only supports execute() (Phase 22 Plan 08)")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

// --- Phase 22 Plan 09: TraceSink test doubles -----------------------------

/// A [`TraceSink`] test double recording every event it receives, in the
/// exact order it received them.
#[derive(Default)]
pub struct RecordingTraceSink {
    events: tokio::sync::Mutex<Vec<TraceEvent>>,
}

impl RecordingTraceSink {
    /// Construct an empty recorder.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The events recorded so far, in receipt order.
    pub async fn events(&self) -> Vec<TraceEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait]
impl TraceSink for RecordingTraceSink {
    async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError> {
        self.events.lock().await.push(event);
        Ok(())
    }
}

/// A [`TraceSink`] test double whose handler never returns, for proving a
/// permanently blocking sink cannot stall the engine or the
/// `TraceDispatcher`'s own `emit` (T-22-30).
#[derive(Default)]
pub struct BlockingTraceSink {
    /// Set the first time `on_event` is called, so a test can confirm the
    /// sink was actually invoked before it hung.
    pub entered: Arc<AtomicBool>,
}

impl BlockingTraceSink {
    /// Construct a sink sharing `entered` with the caller so an assertion
    /// can confirm the handler actually started before hanging forever.
    pub fn new(entered: Arc<AtomicBool>) -> Arc<Self> {
        Arc::new(Self { entered })
    }
}

#[async_trait]
impl TraceSink for BlockingTraceSink {
    async fn on_event(&self, _event: TraceEvent) -> Result<(), TraceSinkError> {
        self.entered.store(true, Ordering::SeqCst);
        std::future::pending::<()>().await;
        unreachable!("std::future::pending() never resolves")
    }
}

/// A [`TraceSink`] test double that returns `Err` on every call, for proving
/// a failing sink never affects a run's outcome (T-22-30).
#[derive(Default)]
pub struct AlwaysErroringTraceSink {
    calls: AtomicUsize,
}

impl AlwaysErroringTraceSink {
    /// Construct a sink with no calls recorded yet.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// How many times `on_event` has been called so far.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl TraceSink for AlwaysErroringTraceSink {
    async fn on_event(&self, _event: TraceEvent) -> Result<(), TraceSinkError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(TraceSinkError::Failed("simulated failure".to_string()))
    }
}

/// A [`TraceSink`] test double whose very FIRST call blocks on a
/// caller-controlled `tokio::sync::Notify` before recording it; every
/// subsequent call records immediately. Lets a test force the dispatcher's
/// background consumer to sit idle on one event while more events accumulate
/// in the queue, then release it and inspect exactly which events survived
/// -- proving drop-OLDEST (not drop-newest) precisely (T-22-31), rather than
/// only proving the drop counter incremented.
pub struct GatedTraceSink {
    events: tokio::sync::Mutex<Vec<TraceEvent>>,
    gate: Arc<tokio::sync::Notify>,
    gated_once: AtomicBool,
}

impl GatedTraceSink {
    /// Construct a sink whose first `on_event` call blocks until `gate` is
    /// notified.
    pub fn new(gate: Arc<tokio::sync::Notify>) -> Arc<Self> {
        Arc::new(Self {
            events: tokio::sync::Mutex::new(Vec::new()),
            gate,
            gated_once: AtomicBool::new(false),
        })
    }

    /// The events recorded so far (including the gated first one, once
    /// released), in receipt order.
    pub async fn events(&self) -> Vec<TraceEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait]
impl TraceSink for GatedTraceSink {
    async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError> {
        if !self.gated_once.swap(true, Ordering::SeqCst) {
            self.gate.notified().await;
        }
        self.events.lock().await.push(event);
        Ok(())
    }
}
