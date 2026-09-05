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

use paladin_core::platform::container::battlefield::{Battlefield, FieldName, StateDelta};
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
use tokio_util::sync::CancellationToken;

use crate::engine::hooks::{InterceptDecision, NodeInterceptor};
use crate::engine::node::{NodeContext, StateNode, StateNodeError};

/// A [`WaypointPort`] test double wrapping an [`InMemoryWaypointStore`],
/// additionally recording every `save` call and able to fail its NEXT save
/// on demand (one-shot; auto-resets after firing), or a specific Nth save
/// call (also one-shot).
#[derive(Default)]
pub struct RecordingWaypointStore {
    inner: InMemoryWaypointStore,
    save_calls: AtomicUsize,
    fail_next_save: AtomicBool,
    fail_at_call: AtomicUsize,
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

    /// Arrange for the `n`th `save` call (1-indexed, across the whole
    /// store's lifetime) to fail with `WaypointError::Backend`. Fires
    /// exactly once, at that specific call, then never again -- lets a test
    /// target a save deep inside a multi-write superstep (e.g. the second
    /// of several progress-Waypoint writes inside a Muster superstep,
    /// CF-FR-12/D-14) without needing to synchronize with the run in
    /// progress.
    pub fn fail_nth_save(&self, n: usize) {
        self.fail_at_call.store(n, Ordering::SeqCst);
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
        let call_number = self.save_calls.fetch_add(1, Ordering::SeqCst) + 1;
        let fail_next = self.fail_next_save.swap(false, Ordering::SeqCst);
        // Only clear `fail_at_call` when THIS call is the targeted one --
        // reading it on every call would otherwise disarm the target before
        // it is ever reached.
        let fail_nth = self.fail_at_call.load(Ordering::SeqCst) == call_number
            && call_number != 0
            && self
                .fail_at_call
                .compare_exchange(call_number, 0, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok();
        if fail_next || fail_nth {
            return Err(WaypointError::Backend {
                source: Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "simulated save failure (RecordingWaypointStore::fail_next_save / \
                     fail_nth_save, call #{call_number})"
                )),
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
    async fn run(
        &self,
        state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
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
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let now_in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_seen.fetch_max(now_in_flight, Ordering::SeqCst);
        tokio::time::sleep(self.hold).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta.into())
    }
}

/// A [`StateNode`] test double for the shutdown-grace race (Phase 24 Plan
/// 08, HITL-04, D-19): increments a shared run counter, THEN sleeps for
/// `hold` before returning a fixed delta. Incrementing before the `.await`
/// point means aborting the task mid-sleep (as the mid-superstep grace
/// race's deadline branch does via `JoinHandle::abort`) still leaves
/// `run_count` incremented for that attempt -- a test can assert an EXACT
/// run count across an aborted-then-resumed scenario (D-19 acceptance 5:
/// `run_count == 2`, one aborted, one completed).
pub struct SlowFunctionNode {
    field: paladin_core::platform::container::battlefield::FieldName,
    value: serde_json::Value,
    hold: std::time::Duration,
    run_count: Arc<AtomicUsize>,
    /// `Some` deterministically places a mid-superstep cancellation exactly
    /// at the moment this node starts (before its own `.await` point),
    /// mirroring `engine::mod`'s own `four_node_chain_graph_with_cancel_at`
    /// convention (a node cancelling ITS OWN token synchronously) rather
    /// than racing a background poller against a real-time sleep.
    cancel_on_start: Option<tokio_util::sync::CancellationToken>,
}

impl SlowFunctionNode {
    /// Construct a node that increments `run_count`, sleeps for `hold`,
    /// then writes `value` to `field`. Never cancels any token itself.
    pub fn new(
        field: paladin_core::platform::container::battlefield::FieldName,
        value: serde_json::Value,
        hold: std::time::Duration,
        run_count: Arc<AtomicUsize>,
    ) -> Arc<Self> {
        Arc::new(Self {
            field,
            value,
            hold,
            run_count,
            cancel_on_start: None,
        })
    }

    /// As [`SlowFunctionNode::new`], but also cancels `token` the instant
    /// this node starts executing (before incrementing `run_count` or
    /// sleeping) -- deterministically placing a mid-superstep cancellation
    /// without any real-time race against sibling nodes in the same
    /// dispatch batch.
    pub fn cancelling(
        field: paladin_core::platform::container::battlefield::FieldName,
        value: serde_json::Value,
        hold: std::time::Duration,
        run_count: Arc<AtomicUsize>,
        token: tokio_util::sync::CancellationToken,
    ) -> Arc<Self> {
        Arc::new(Self {
            field,
            value,
            hold,
            run_count,
            cancel_on_start: Some(token),
        })
    }
}

#[async_trait]
impl StateNode for SlowFunctionNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        if let Some(token) = &self.cancel_on_start {
            token.cancel();
        }
        self.run_count.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(self.hold).await;
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
    /// Construct a node that always returns `StateNodeError(message)`.
    pub fn new(message: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            message: message.into(),
        })
    }
}

#[async_trait]
impl StateNode for FailingFunctionNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Err(StateNodeError(self.message.clone()))
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
    async fn run(
        &self,
        state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
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
            served_by: None,
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

/// A [`PaladinPort`] test double whose `execute` always fails with the
/// `PaladinError` a caller-supplied factory produces (a factory rather than
/// a stored error, following `llm_failure`'s test-double precedent: the
/// double stays `Send + Sync` without a `Mutex` and every call's error is
/// fresh), counting its calls, for exercising the engine's Paladin-node
/// failure path (Doc 04 D-07: `NodeFailure::Paladin` ->
/// `NodeErrorSource::Paladin`/`Llm`).
pub struct FailingPaladinPort {
    factory: fn() -> PaladinError,
    calls: AtomicUsize,
}

impl FailingPaladinPort {
    /// Construct a port whose every `execute` fails with `factory()`.
    pub fn new(factory: fn() -> PaladinError) -> Arc<Self> {
        Arc::new(Self {
            factory,
            calls: AtomicUsize::new(0),
        })
    }

    /// How many times `execute` has been called.
    pub fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PaladinPort for FailingPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err((self.factory)())
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("FailingPaladinPort only supports execute()")
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

// --- Plan 25-01: Aegis retry loop test doubles ---------------------------

/// A [`StateNode`] test double that fails with a fixed message on its first
/// `fail_until_attempt - 1` runs, then succeeds on and after
/// `fail_until_attempt` (1-indexed), for exercising the Aegis retry loop's
/// "fails once, retries in place, run completes" path
/// (`transient_function_node_failure_is_retried_and_run_completes`). Records
/// the run count and each run's Battlefield snapshot pointer, mirroring
/// [`CountingFunctionNode`]'s own snapshot-identity assertions.
pub struct FailThenSucceedNode {
    fail_until_attempt: usize,
    message: String,
    field: paladin_core::platform::container::battlefield::FieldName,
    success_value: serde_json::Value,
    run_count: Arc<AtomicUsize>,
    observed_snapshots: Arc<Mutex<Vec<Battlefield>>>,
}

impl FailThenSucceedNode {
    /// Construct a node that fails (with `message`) on every run before its
    /// `fail_until_attempt`-th (1-indexed), then succeeds by writing
    /// `success_value` to `field`. A failing run's `StateNode::run` returns
    /// `Err` with no `Directive` at all -- so no delta from a failing
    /// attempt can ever reach the merge on any code path
    /// (`failed_attempt_delta_never_reaches_the_battlefield` proves this
    /// end-to-end: the merged Battlefield after the run contains only the
    /// succeeding attempt's `success_value`, never any earlier attempt's
    /// state).
    pub fn new(
        fail_until_attempt: usize,
        message: impl Into<String>,
        field: paladin_core::platform::container::battlefield::FieldName,
        success_value: serde_json::Value,
    ) -> Arc<Self> {
        Arc::new(Self {
            fail_until_attempt,
            message: message.into(),
            field,
            success_value,
            run_count: Arc::new(AtomicUsize::new(0)),
            observed_snapshots: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// How many times this node has run so far (across every attempt).
    pub fn run_count(&self) -> usize {
        self.run_count.load(Ordering::SeqCst)
    }

    /// The Battlefield snapshot observed on each run, in run order --
    /// `each_attempt_reads_an_identical_battlefield_snapshot` compares
    /// these for equality (never identity: each attempt clones the same
    /// underlying data out of the shared `Arc<Battlefield>`, D-14).
    pub fn observed_snapshots(&self) -> Vec<Battlefield> {
        self.observed_snapshots.lock().unwrap().clone()
    }
}

#[async_trait]
impl StateNode for FailThenSucceedNode {
    async fn run(
        &self,
        state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let run_index = self.run_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.observed_snapshots.lock().unwrap().push(state.clone());
        if run_index < self.fail_until_attempt {
            return Err(StateNodeError(self.message.clone()));
        }
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.success_value.clone());
        Ok(delta.into())
    }
}

/// A [`NodeInterceptor`] test double recording every `before`/`after` call,
/// in receipt order, for
/// `interceptors_run_once_per_attempt_not_once_per_node` to assert the exact
/// `before, after, before, after, ...` sequence a 2-attempt retry produces.
/// Always decides `Proceed`.
#[derive(Default)]
pub struct RecordingInterceptor {
    calls: Mutex<Vec<&'static str>>,
}

impl RecordingInterceptor {
    /// Construct a recorder with no calls yet.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The ordered call log: `"before"`/`"after"` per hook invocation.
    pub fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl NodeInterceptor for RecordingInterceptor {
    async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
        self.calls.lock().unwrap().push("before");
        InterceptDecision::Proceed
    }

    async fn after(&self, _ctx: &NodeContext, _delta: &mut StateDelta) {
        self.calls.lock().unwrap().push("after");
    }
}

/// A [`NodeInterceptor`] test double whose `before` always returns a fixed
/// decision, for `interceptor_fail_decision_is_not_retried`'s
/// `InterceptDecision::Fail`/`Skip` cases. Records how many times `before`
/// was called, so a test can assert the decision was reached exactly once
/// (never retried).
pub struct FixedDecisionInterceptor {
    decision_fn: Arc<dyn Fn() -> InterceptDecision + Send + Sync>,
    before_calls: Arc<AtomicUsize>,
}

impl FixedDecisionInterceptor {
    /// Construct an interceptor whose `before` always returns
    /// `decision_fn()`'s result.
    pub fn new(decision_fn: impl Fn() -> InterceptDecision + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            decision_fn: Arc::new(decision_fn),
            before_calls: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// How many times `before` has been called.
    pub fn before_call_count(&self) -> usize {
        self.before_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl NodeInterceptor for FixedDecisionInterceptor {
    async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
        self.before_calls.fetch_add(1, Ordering::SeqCst);
        (self.decision_fn)()
    }

    async fn after(&self, _ctx: &NodeContext, _delta: &mut StateDelta) {}
}

// --- Phase 25 Plan 07: per-task Muster retry doubles (D-17) ---------------

/// One observed call of a [`MusterFailThenSucceedWorker`]: the task's
/// `task_key`, when (on the tokio clock) the call started, and how many
/// `save`s the observed [`RecordingWaypointStore`] had received by then.
#[derive(Debug, Clone)]
pub struct WorkerCall {
    /// The `ctx.task_key()` the call ran under.
    pub task_key: String,
    /// The tokio-clock instant the call started (paused-clock friendly).
    pub at: tokio::time::Instant,
    /// `RecordingWaypointStore::save_call_count()` at call start, or `0`
    /// with no observed store.
    pub saves_seen: usize,
}

/// A Muster worker-template [`StateNode`] test double keyed by
/// `ctx.task_key()`: each task fails (with `StateNodeError("transient")`)
/// on its first `failures[task_key]` runs and succeeds afterwards by
/// appending its own key to `field`; every other key succeeds at once.
/// Records a per-key run count and an ordered log of every call, so a test
/// can assert that one task's retries never re-ran or delayed a sibling
/// (FT-FR-06) and that no Waypoint was written between attempts (FT-FR-07).
pub struct MusterFailThenSucceedWorker {
    field: FieldName,
    failures: HashMap<String, usize>,
    counts: Mutex<HashMap<String, usize>>,
    calls: Mutex<Vec<WorkerCall>>,
    observed_store: Option<Arc<RecordingWaypointStore>>,
}

impl MusterFailThenSucceedWorker {
    /// Construct a worker whose tasks named in `failures` fail that many
    /// times before succeeding, optionally observing `store`'s save count
    /// at every call.
    pub fn new(
        field: FieldName,
        failures: impl IntoIterator<Item = (&'static str, usize)>,
        observed_store: Option<Arc<RecordingWaypointStore>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            field,
            failures: failures
                .into_iter()
                .map(|(k, n)| (k.to_string(), n))
                .collect(),
            counts: Mutex::new(HashMap::new()),
            calls: Mutex::new(Vec::new()),
            observed_store,
        })
    }

    /// How many times the task keyed `task_key` has run so far.
    pub fn run_count(&self, task_key: &str) -> usize {
        self.counts
            .lock()
            .unwrap()
            .get(task_key)
            .copied()
            .unwrap_or(0)
    }

    /// Every call so far, in call order.
    pub fn calls(&self) -> Vec<WorkerCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl StateNode for MusterFailThenSucceedWorker {
    async fn run(
        &self,
        _state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let key = ctx.task_key().unwrap_or_default().to_string();
        let run_index = {
            let mut counts = self.counts.lock().unwrap();
            let count = counts.entry(key.clone()).or_insert(0);
            *count += 1;
            *count
        };
        self.calls.lock().unwrap().push(WorkerCall {
            task_key: key.clone(),
            at: tokio::time::Instant::now(),
            saves_seen: self
                .observed_store
                .as_ref()
                .map(|s| s.save_call_count())
                .unwrap_or(0),
        });
        if run_index <= self.failures.get(&key).copied().unwrap_or(0) {
            return Err(StateNodeError("transient".to_string()));
        }
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), serde_json::json!(key));
        Ok(delta.into())
    }
}

/// A vanguard [`StateNode`] test double that fails on every run before its
/// `fail_until_attempt`-th (1-indexed), records the observed
/// [`RecordingWaypointStore`] save count at the start of EVERY run (so a
/// test can assert no Waypoint was written between two attempts,
/// FT-FR-07), and can cancel a run's `CancellationToken` from inside its
/// first failing run (so a test can interrupt a run mid-backoff
/// deterministically and prove a resume re-executes it from attempt 1).
pub struct AttemptObservingNode {
    fail_until_attempt: usize,
    field: FieldName,
    run_count: AtomicUsize,
    saves_seen: Mutex<Vec<usize>>,
    observed_store: Arc<RecordingWaypointStore>,
    cancel_on_first_failure: Option<CancellationToken>,
}

impl AttemptObservingNode {
    /// Construct a node that fails before its `fail_until_attempt`-th run,
    /// observing `store`'s save count on every run, and cancelling
    /// `cancel_on_first_failure` (if given) from inside its first failing
    /// run.
    pub fn new(
        fail_until_attempt: usize,
        field: FieldName,
        observed_store: Arc<RecordingWaypointStore>,
        cancel_on_first_failure: Option<CancellationToken>,
    ) -> Arc<Self> {
        Arc::new(Self {
            fail_until_attempt,
            field,
            run_count: AtomicUsize::new(0),
            saves_seen: Mutex::new(Vec::new()),
            observed_store,
            cancel_on_first_failure,
        })
    }

    /// How many times this node has run so far, across every attempt and
    /// every run of the thread.
    pub fn run_count(&self) -> usize {
        self.run_count.load(Ordering::SeqCst)
    }

    /// The observed store's `save_call_count()` at the start of each run,
    /// in run order.
    pub fn saves_seen(&self) -> Vec<usize> {
        self.saves_seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl StateNode for AttemptObservingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let run_index = self.run_count.fetch_add(1, Ordering::SeqCst) + 1;
        self.saves_seen
            .lock()
            .unwrap()
            .push(self.observed_store.save_call_count());
        if run_index < self.fail_until_attempt {
            if run_index == 1
                && let Some(token) = &self.cancel_on_first_failure
            {
                token.cancel();
            }
            return Err(StateNodeError("transient".to_string()));
        }
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), serde_json::json!("recovered"));
        Ok(delta.into())
    }
}
