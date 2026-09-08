//! Engine seams with no consumers yet (Phase 22 Plan 09, ENG-FR-21/22/23):
//! the bounded, drop-oldest [`TraceDispatcher`] forwarding to an optional
//! [`TraceSink`], the ordered [`NodeInterceptor`] chain, and the
//! cancellation-to-`Halted` path's supporting types. Docs 03, 05 and 07 wire
//! consumers into these; this plan proves each seam is non-interfering on
//! its own.
//!
//! # Why fire-and-forget (T-22-30, T-22-31)
//!
//! A slow or permanently blocking `TraceSink` must never stall a run, and a
//! sink erroring on every call must never fail one. `TraceDispatcher::emit`
//! (the ONLY thing the superstep loop calls) only ever touches a
//! `std::sync::Mutex` for a few instructions and a non-blocking channel
//! `try_send` — it never awaits the sink's own handler and never awaits
//! channel backpressure. A single background task is the only caller of
//! `TraceSink::on_event`; a handler that never returns simply leaves that
//! one task permanently busy, `emit` keeps working, and the queue's
//! drop-oldest policy — counted in an atomic rather than silently discarding
//! (T-22-31) — is what keeps memory bounded when the sink cannot keep up.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::mpsc;

use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::trace_sink_port::{TraceEmitter, TraceEvent, TraceRecord, TraceSink};

use crate::engine::node::{NodeContext, StateNodeError};

/// Default queue capacity for a [`TraceDispatcher`] constructed via
/// [`TraceDispatcher::new`]. Arbitrary but generous for a single run's event
/// volume; callers with unusual throughput needs can use
/// [`TraceDispatcher::with_capacity`] instead.
pub(crate) const DEFAULT_CAPACITY: usize = 1024;

/// Shared state between a [`TraceDispatcher`] and its background consumer
/// task.
struct TraceQueue {
    /// Buffered, not-yet-forwarded records. A `std::sync::Mutex`, not a
    /// `tokio::sync::Mutex`: every critical section here is synchronous and
    /// brief (push/pop on a `VecDeque`), so there is nothing to gain from an
    /// async-aware lock and a real cost (an extra allocation/state machine)
    /// to paying for one.
    buffer: Mutex<VecDeque<TraceRecord>>,
    /// The configured capacity. When `buffer` is at this length, `emit`
    /// drops the OLDEST buffered record to make room for the new one.
    capacity: usize,
    /// Total events dropped so far due to a full queue (T-22-31) — readable
    /// via [`TraceDispatcher::dropped_count`], never silently lost.
    dropped: AtomicU64,
    /// This dispatcher's own monotonic sequence counter (D-03): 1-based,
    /// stamped onto every [`TraceRecord`] at enqueue time, so `seq` order
    /// IS causal order regardless of how many concurrent callers share this
    /// dispatcher.
    seq: AtomicU64,
}

/// The engine-owned trace event dispatcher (ENG-FR-21, D-03): sits between
/// the superstep loop (and every producer sharing its
/// [`TraceEmitter`] handle) and an optional `Arc<dyn TraceSink>`,
/// stamping `seq`/`at`/`thread_id`/`run_id` at enqueue time and forwarding
/// the resulting [`TraceRecord`]s over a bounded, drop-oldest queue via a
/// single background task.
///
/// With no sink configured, [`TraceDispatcher::new`] allocates no channel and
/// [`TraceDispatcher::emit`] is a zero-cost no-op — the untraced path costs
/// nothing (a must-have truth carried from Phase 22).
///
/// Constructed with the specific `thread_id` (and optional `run_id`) it
/// stamps every record for (D-03): production always builds one engine —
/// and so one dispatcher — per run (the Phase 27 worker's own pattern), so
/// `seq` starting at 1 per dispatcher IS `seq` starting at 1 per run.
pub struct TraceDispatcher {
    /// `None` when no sink is configured. `Some` pairs the shared queue with
    /// the lightweight "doorbell" sender that wakes the consumer task —
    /// ordinary drop glue (no explicit `Drop` impl needed) drops this sender
    /// when the `TraceDispatcher` goes away, which is what lets the
    /// consumer's `recv().await` return `None` and exit once the buffer it
    /// can see has drained, rather than leaking a task that loops forever.
    inner: Option<(Arc<TraceQueue>, mpsc::Sender<()>)>,
    /// The thread every record this dispatcher stamps belongs to.
    thread_id: ThreadId,
    /// The Platform API run every record this dispatcher stamps belongs to,
    /// when known.
    run_id: Option<RunId>,
}

impl TraceDispatcher {
    /// Construct a dispatcher stamping every record for `thread_id`
    /// (and `run_id`, when known), forwarding to `sink` (if any) with the
    /// default capacity. `None` allocates no channel and spawns no task.
    pub fn new(
        thread_id: ThreadId,
        run_id: Option<RunId>,
        sink: Option<Arc<dyn TraceSink>>,
    ) -> Self {
        Self::with_capacity(thread_id, run_id, sink, DEFAULT_CAPACITY)
    }

    /// As [`TraceDispatcher::new`], with an explicit queue `capacity`.
    pub fn with_capacity(
        thread_id: ThreadId,
        run_id: Option<RunId>,
        sink: Option<Arc<dyn TraceSink>>,
        capacity: usize,
    ) -> Self {
        let Some(sink) = sink else {
            return Self {
                inner: None,
                thread_id,
                run_id,
            };
        };

        let queue = Arc::new(TraceQueue {
            buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity: capacity.max(1),
            dropped: AtomicU64::new(0),
            seq: AtomicU64::new(0),
        });
        // Capacity 1: the doorbell only ever needs to prove "there is at
        // least one more thing to check for" -- the consumer always drains
        // the whole buffer before waiting again, so a coalesced signal
        // never loses an event (only ever a redundant wakeup).
        let (doorbell_tx, mut doorbell_rx) = mpsc::channel::<()>(1);

        let consumer_queue = Arc::clone(&queue);
        tokio::spawn(async move {
            loop {
                loop {
                    let record = {
                        let mut buf = consumer_queue
                            .buffer
                            .lock()
                            .expect("trace queue mutex poisoned");
                        buf.pop_front()
                    };
                    match record {
                        Some(record) => {
                            // Fire-and-forget: this await can block or hang
                            // forever without affecting `emit` or the run,
                            // which have already returned by the time this
                            // task runs. The return value is diagnostic only
                            // (see trace_sink_port's module docs) and is
                            // deliberately discarded. Panic isolation around
                            // this call lands in a later commit of this same
                            // plan (D-08).
                            let _ = sink.on_event(record).await;
                        }
                        None => break,
                    }
                }
                // Blocks until `emit` sends a doorbell signal, or returns
                // `None` once every `TraceDispatcher` (and thus every
                // `doorbell_tx`, dropped via ordinary drop glue -- no
                // separate shutdown flag needed) has gone away.
                if doorbell_rx.recv().await.is_none() {
                    break;
                }
            }
        });

        Self {
            inner: Some((queue, doorbell_tx)),
            thread_id,
            run_id,
        }
    }

    /// Stamp `event` into a [`TraceRecord`] (`seq`/`at`/`thread_id`/`run_id`,
    /// D-03) and enqueue it. Never awaits the sink and never awaits channel
    /// backpressure (see the module-level "Why fire-and-forget" section):
    /// with no sink configured this is a no-op (no `seq` observably
    /// advances); with a sink configured, a full queue drops the OLDEST
    /// buffered record (incrementing the counter
    /// [`TraceDispatcher::dropped_count`] reports) to make room for the new
    /// one.
    pub fn emit(&self, event: TraceEvent) {
        let Some((queue, doorbell)) = &self.inner else {
            return;
        };
        let seq = queue.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let record = TraceRecord {
            thread_id: self.thread_id.clone(),
            run_id: self.run_id.clone(),
            seq,
            at: Utc::now(),
            event,
        };
        {
            let mut buf = queue.buffer.lock().expect("trace queue mutex poisoned");
            if buf.len() >= queue.capacity {
                buf.pop_front();
                queue.dropped.fetch_add(1, Ordering::SeqCst);
            }
            buf.push_back(record);
        }
        // A full doorbell channel means a signal is already pending and the
        // consumer will drain everything (including this event) once it
        // wakes -- `try_send` failing here is expected and safe to ignore.
        let _ = doorbell.try_send(());
    }

    /// Total events dropped so far because the queue was full when `emit`
    /// was called (T-22-31). Always `0` with no sink configured.
    pub fn dropped_count(&self) -> u64 {
        self.inner
            .as_ref()
            .map_or(0, |(queue, _)| queue.dropped.load(Ordering::SeqCst))
    }

    /// The thread every record this dispatcher stamps belongs to.
    pub fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }
}

impl TraceEmitter for TraceDispatcher {
    /// Delegates to the inherent [`TraceDispatcher::emit`] (D-03): lets a
    /// producer below the superstep engine hold `Arc<dyn TraceEmitter>`
    /// rather than a concrete dispatcher type.
    fn emit(&self, event: TraceEvent) {
        TraceDispatcher::emit(self, event);
    }
}

/// A [`NodeInterceptor::before`] decision for one vanguard node (ENG-FR-22).
///
/// Marked `#[non_exhaustive]`: a `match` over this must always carry a
/// wildcard arm, so a later variant does not silently become a compile error
/// everywhere it is matched.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InterceptDecision {
    /// Execute the node normally.
    Proceed,
    /// Do not execute the node. Contributes no delta to this superstep's
    /// merge and is recorded in the `Waypoint` as
    /// `NodeOutcomeKind::Skipped { reason }` — visible in the checkpoint
    /// history, never a silent no-op (T-22-33).
    Skip(String),
    /// Do not execute the node; fail it exactly as if its own execution had
    /// returned this `StateNodeError`.
    Fail(StateNodeError),
}

/// An ordered hook wrapping node execution (ENG-FR-22): observes or
/// overrides a node's execution decision before it runs, and can mutate its
/// resulting delta before it reaches the merge.
///
/// # Default chain is empty
///
/// [`crate::engine::WarEngine`] holds an ordered `Vec<Arc<dyn
/// NodeInterceptor>>` that defaults to empty. With an empty chain, a run's
/// node executions and final Battlefield are identical to a run with no
/// chain configured at all — this plan's must-have truth, proven by an
/// equivalence test in `engine::hooks`'s own test module.
///
/// # Ordering
///
/// `before` runs first-to-last across the chain, short-circuiting on the
/// first non-`Proceed` decision (a later interceptor's `before` is never
/// called once an earlier one has decided `Skip` or `Fail`). `after` runs
/// first-to-last over the node's resulting delta, each observing the
/// mutation the previous `after` made.
///
/// # Doc 04's Aegis wraps OUTSIDE this chain
///
/// Per-node fault tolerance (retry, timeout, typed error handlers, model
/// fallback) is a WRAPPER around a node's whole interceptor-wrapped
/// execution, not a participant inside this chain. Nesting Aegis's policy
/// INSIDE the interceptor chain would mean an interceptor's `Skip`/`Fail`
/// decision could itself be retried as if it were a node's own transient
/// failure, which is never the intended semantics — an interceptor's
/// decision is deliberate policy, not a fault to recover from.
///
/// Landed (plan 25-01): the retry loop in `crate::engine::superstep::run`'s
/// spawned per-node dispatch closure wraps the ENTIRE sequence this trait's
/// `before`/`after` chain participates in -- the `NodeStarted` trace emit,
/// this whole `before` chain, `execute_vanguard_node`, this whole `after`
/// chain, and the `NodeFinished` trace emit -- once per attempt. `Skip` and
/// `Fail` are decided fresh on every attempt (an interceptor cannot tell,
/// from its own perspective, that it is being asked again), but a `Skip`/
/// `Fail` decision itself is never retried BECAUSE it produces
/// `NodeRunOutcome::Skipped`/`Failed(NodeFailure::Node(_))` immediately --
/// exactly the outcomes `crate::engine::retry::should_retry` classifies via
/// a `NodeError.transience`, so an interceptor's own policy decision is
/// retried only if the caller's `Aegis` says a `Function`-sourced error at
/// that transience is retryable, same as any other node failure.
///
/// # A third layer sits INSIDE this chain (Phase 26, D-05)
///
/// For a `NodeSpec::Paladin` node specifically, the facade's own
/// `ExecutionMiddleware` chain (`paladin::application::services::paladin::middleware`)
/// runs inside the node's own execution -- once per model call and once per
/// tool/handoff dispatch -- while THIS trait continues to bracket the whole
/// node once per Aegis attempt. The two layers are independent and neither
/// wraps a registry the other reads: a `PaladinExecutionService` carrying
/// middleware applies its chain unchanged when the engine dispatches it
/// through `PaladinPort::execute_observed`, with no engine-side change.
#[async_trait]
pub trait NodeInterceptor: Send + Sync {
    /// Decide whether `ctx`'s node should execute against `state` this
    /// superstep.
    async fn before(&self, ctx: &NodeContext, state: &Battlefield) -> InterceptDecision;

    /// Observe or mutate `delta`, the node's own successful execution
    /// result, before it joins this superstep's merge set. Never called for
    /// a node whose `before` returned `Skip`/`Fail`, nor for a node whose own
    /// execution returned an error.
    async fn after(&self, ctx: &NodeContext, delta: &mut StateDelta);
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    fn ctx() -> NodeContext {
        NodeContext {
            node_id: NodeId::new("n"),
            thread_id: ThreadId::new("t").unwrap(),
            superstep: 1,
            muster: None,
            parley_response: None,
            attempt: 1,
            heartbeat: crate::engine::heartbeat::HeartbeatHandle::new(),
            vault: None,
        }
    }

    // --- TraceDispatcher ---------------------------------------------

    use crate::engine::test_support::{
        AlwaysErroringTraceSink, BlockingTraceSink, GatedTraceSink, RecordingTraceSink,
    };

    fn event_name(event: &TraceEvent) -> &'static str {
        match event {
            TraceEvent::RunStarted { .. } => "RunStarted",
            TraceEvent::SuperstepStarted { .. } => "SuperstepStarted",
            TraceEvent::NodeStarted { .. } => "NodeStarted",
            TraceEvent::NodeProgress { .. } => "NodeProgress",
            TraceEvent::NodeFinished { .. } => "NodeFinished",
            TraceEvent::EdgeEvaluated { .. } => "EdgeEvaluated",
            TraceEvent::DeltaMerged { .. } => "DeltaMerged",
            TraceEvent::WaypointSaved { .. } => "WaypointSaved",
            TraceEvent::ParleyRaised { .. } => "ParleyRaised",
            TraceEvent::RunFinished { .. } => "RunFinished",
            TraceEvent::FallbackHop { .. } => "FallbackHop",
            TraceEvent::MiddlewareEvent { .. } => "MiddlewareEvent",
            _ => "unknown",
        }
    }

    fn run_started() -> TraceEvent {
        TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        }
    }

    fn run_finished() -> TraceEvent {
        TraceEvent::RunFinished {
            status: paladin_ports::output::trace_sink_port::RunFinishStatus::Completed,
            total_supersteps: 0,
            total_tokens: 0,
            duration_ms: 0,
            trace_dropped_total: 0,
        }
    }

    fn superstep_started(superstep: u64) -> TraceEvent {
        TraceEvent::SuperstepStarted {
            superstep,
            vanguard: Vec::new(),
        }
    }

    #[tokio::test]
    async fn no_sink_emit_is_a_no_op_and_dropped_count_is_zero() {
        let dispatcher = TraceDispatcher::new(ThreadId::new("t").unwrap(), None, None);
        dispatcher.emit(run_started());
        assert_eq!(dispatcher.dropped_count(), 0);
    }

    #[tokio::test]
    async fn recording_sink_receives_emitted_events_in_order() {
        let sink = RecordingTraceSink::new();
        let thread_id = ThreadId::new("t").unwrap();
        let dispatcher = TraceDispatcher::new(thread_id.clone(), None, Some(sink.clone()));

        dispatcher.emit(run_started());
        dispatcher.emit(superstep_started(1));
        dispatcher.emit(run_finished());

        // Give the background consumer a chance to drain.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let records = sink.events().await;
        let names: Vec<&str> = records.iter().map(|r| event_name(&r.event)).collect();
        assert_eq!(names, vec!["RunStarted", "SuperstepStarted", "RunFinished"]);
        // D-03: seq is 1-based and gapless for a freshly constructed
        // dispatcher.
        let seqs: Vec<u64> = records.iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3]);
        assert!(records.iter().all(|r| r.thread_id == thread_id));
    }

    #[tokio::test]
    async fn permanently_blocking_sink_never_stalls_emit() {
        let entered = Arc::new(AtomicBool::new(false));
        let sink = BlockingTraceSink::new(entered.clone());
        let dispatcher = TraceDispatcher::new(ThreadId::new("t").unwrap(), None, Some(sink));

        let result = tokio::time::timeout(Duration::from_secs(5), async {
            dispatcher.emit(run_started());
            // A second event proves `emit` itself never awaits the
            // sink's own handler, even after the handler has started
            // blocking.
            tokio::time::sleep(Duration::from_millis(50)).await;
            dispatcher.emit(run_finished());
        })
        .await;

        assert!(
            result.is_ok(),
            "emit must complete inside the timeout even with a permanently blocking sink"
        );
        assert!(
            entered.load(Ordering::SeqCst),
            "the blocking sink must actually have been invoked"
        );
    }

    #[tokio::test]
    async fn always_erroring_sink_does_not_panic_or_block_dispatcher() {
        let sink = AlwaysErroringTraceSink::new();
        let dispatcher =
            TraceDispatcher::new(ThreadId::new("t").unwrap(), None, Some(sink.clone()));
        for _ in 0..5 {
            dispatcher.emit(run_started());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(sink.call_count(), 5);
        assert_eq!(dispatcher.dropped_count(), 0);
    }

    #[tokio::test]
    async fn full_queue_drops_the_oldest_event_not_the_newest() {
        // Capacity 2, sink gated on its FIRST call: the consumer picks up
        // event 0 immediately and blocks on it, so everything emitted after
        // that just accumulates in the queue rather than being drained.
        let gate = Arc::new(tokio::sync::Notify::new());
        let sink = GatedTraceSink::new(gate.clone());
        let dispatcher = TraceDispatcher::with_capacity(
            ThreadId::new("t").unwrap(),
            None,
            Some(sink.clone()),
            2,
        );

        dispatcher.emit(superstep_started(0));
        // Let the consumer pick event 0 up and start blocking on the gate.
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Fill the queue to capacity (2) then overflow it by one: the
        // OLDEST of these three (superstep 1) must be the one dropped, not
        // superstep 3 (the newest).
        dispatcher.emit(superstep_started(1));
        dispatcher.emit(superstep_started(2));
        dispatcher.emit(superstep_started(3));
        assert_eq!(dispatcher.dropped_count(), 1);

        gate.notify_one();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let supersteps: Vec<u64> = sink
            .events()
            .await
            .iter()
            .map(|r| match &r.event {
                TraceEvent::SuperstepStarted { superstep, .. } => *superstep,
                other => panic!("unexpected event: {other:?}"),
            })
            .collect();
        assert_eq!(
            supersteps,
            vec![0, 2, 3],
            "superstep 1 (the oldest buffered) must be dropped, not superstep 3 (the newest)"
        );
    }

    // --- NodeInterceptor / InterceptDecision --------------------------

    struct AlwaysSkip;

    #[async_trait]
    impl NodeInterceptor for AlwaysSkip {
        async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
            InterceptDecision::Skip("always skips".to_string())
        }

        async fn after(&self, _ctx: &NodeContext, _delta: &mut StateDelta) {
            panic!("after must not be called when before returned Skip");
        }
    }

    #[tokio::test]
    async fn skip_decision_short_circuits_before_reaching_after() {
        let interceptor = AlwaysSkip;
        let decision = interceptor
            .before(
                &ctx(),
                &Battlefield::new(
                    paladin_core::platform::container::battlefield::BattlefieldSchema::new(vec![]),
                ),
            )
            .await;
        assert!(matches!(decision, InterceptDecision::Skip(reason) if reason == "always skips"));
    }

    #[tokio::test]
    async fn fail_decision_carries_the_given_node_error() {
        struct AlwaysFail;
        #[async_trait]
        impl NodeInterceptor for AlwaysFail {
            async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
                InterceptDecision::Fail(StateNodeError("intercepted failure".to_string()))
            }
            async fn after(&self, _ctx: &NodeContext, _delta: &mut StateDelta) {}
        }
        let decision = AlwaysFail
            .before(
                &ctx(),
                &Battlefield::new(
                    paladin_core::platform::container::battlefield::BattlefieldSchema::new(vec![]),
                ),
            )
            .await;
        match decision {
            InterceptDecision::Fail(StateNodeError(msg)) => assert_eq!(msg, "intercepted failure"),
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn after_can_mutate_the_delta() {
        struct AppendMarker;
        #[async_trait]
        impl NodeInterceptor for AppendMarker {
            async fn before(&self, _ctx: &NodeContext, _state: &Battlefield) -> InterceptDecision {
                InterceptDecision::Proceed
            }
            async fn after(&self, _ctx: &NodeContext, delta: &mut StateDelta) {
                delta.set_raw(
                    paladin_core::platform::container::battlefield::FieldName::new("marker")
                        .unwrap(),
                    serde_json::json!("stamped"),
                );
            }
        }
        let mut delta = StateDelta::new();
        AppendMarker.after(&ctx(), &mut delta).await;
        assert_eq!(
            delta.values.get(
                &paladin_core::platform::container::battlefield::FieldName::new("marker").unwrap()
            ),
            Some(&serde_json::json!("stamped"))
        );
    }
}
