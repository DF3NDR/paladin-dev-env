//! `PersistingTraceSink` — buffered, superstep-boundary flushes of the
//! trace record stream into `run_traces` (OBS-02, D-17).
//!
//! # Best-effort; the Waypoint is the durability truth
//!
//! Trace persistence is a REPLAY convenience, never the run's own
//! correctness or resumability boundary -- that is
//! [`Waypoint`](paladin_core::platform::container::waypoint::Waypoint),
//! written through `WaypointPort` on every superstep regardless of whether
//! this sink is attached at all (mirrors
//! [`RunTracePort`](paladin_ports::output::run_trace_port::RunTracePort)'s
//! own module docs). A crash between two flushes loses at most the
//! un-flushed tail of one superstep's records; it never loses the ability
//! to resume the thread, and a `RunTracePort::append` failure is logged and
//! swallowed into `Ok(())` -- this sink never fails, stalls or alters the
//! run it observes (the same prohibition `28-06-PLAN.md` records for every
//! other [`TraceSink`]).
//!
//! # Flush boundaries
//!
//! Records are buffered in memory and flushed as one batch through
//! [`RunTracePort::append`] whenever:
//! - a [`TraceEvent::WaypointSaved`] record arrives -- the superstep
//!   boundary a durable checkpoint just crossed;
//! - a [`TraceEvent::RunFinished`] record arrives -- the run's own terminal
//!   record, included in that final flush;
//! - the buffer reaches [`PersistingTraceSink::with_threshold`]'s configured
//!   threshold (default [`DEFAULT_FLUSH_THRESHOLD`]) -- bounding memory for
//!   a long superstep that never reaches a `WaypointSaved` in between (a
//!   node retried many times, for instance).

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceRecord, TraceSink, TraceSinkError};

/// The default record-count flush threshold, when
/// [`PersistingTraceSink::new`] is used instead of
/// [`PersistingTraceSink::with_threshold`].
pub const DEFAULT_FLUSH_THRESHOLD: usize = 256;

/// Buffers [`TraceRecord`]s and flushes them as one batch through a
/// [`RunTracePort`] at superstep boundaries, on the run's terminal record,
/// or once the buffer reaches its configured threshold. See the module
/// docs for the full best-effort/durability contract.
pub struct PersistingTraceSink {
    port: Arc<dyn RunTracePort>,
    buffer: Mutex<Vec<TraceRecord>>,
    threshold: usize,
}

impl PersistingTraceSink {
    /// Construct a sink over `port`, flushing at [`DEFAULT_FLUSH_THRESHOLD`]
    /// records.
    pub fn new(port: Arc<dyn RunTracePort>) -> Self {
        Self::with_threshold(port, DEFAULT_FLUSH_THRESHOLD)
    }

    /// Construct a sink over `port`, flushing once the buffer reaches
    /// `threshold` records (in addition to the `WaypointSaved`/
    /// `RunFinished` boundaries every instance flushes on regardless).
    pub fn with_threshold(port: Arc<dyn RunTracePort>, threshold: usize) -> Self {
        Self {
            port,
            buffer: Mutex::new(Vec::new()),
            threshold,
        }
    }

    /// Append every buffered record as one batch, clearing the buffer
    /// afterwards. A backend failure is logged once at error level and
    /// swallowed -- see the module docs' best-effort contract.
    async fn flush(&self, buffer: &mut Vec<TraceRecord>) {
        if buffer.is_empty() {
            return;
        }
        let batch = std::mem::take(buffer);
        let count = batch.len();
        if let Err(error) = self.port.append(&batch).await {
            log::error!(
                target: "paladin::trace",
                "PersistingTraceSink failed to persist {count} record(s): {error}"
            );
        }
    }
}

#[async_trait]
impl TraceSink for PersistingTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        let is_boundary = matches!(
            record.event,
            TraceEvent::WaypointSaved { .. } | TraceEvent::RunFinished { .. }
        );
        let mut buffer = self.buffer.lock().await;
        buffer.push(record);
        let over_threshold = buffer.len() >= self.threshold;
        if is_boundary || over_threshold {
            self.flush(&mut buffer).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use paladin_battalion::engine::{
        EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine,
        WarGraph,
    };
    use paladin_core::platform::container::battlefield::{
        Battlefield, BattlefieldSchema, StateDelta,
    };
    use paladin_core::platform::container::directive::Directive;
    use paladin_core::platform::container::execution_result::PaladinResult;
    use paladin_core::platform::container::paladin::Paladin;
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointId};
    use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
    use paladin_ports::output::run_trace_port::RunTraceError;
    use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    fn sample_thread() -> ThreadId {
        ThreadId::new(format!("thread-{}", RunId::new_v7())).unwrap()
    }

    fn wrap(thread_id: ThreadId, seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id,
            run_id: None,
            seq,
            at: chrono::Utc::now(),
            event,
        }
    }

    fn superstep_started(seq: u64, thread_id: ThreadId) -> TraceRecord {
        wrap(
            thread_id,
            seq,
            TraceEvent::SuperstepStarted {
                superstep: seq,
                vanguard: vec![NodeId::new("n1")],
            },
        )
    }

    fn waypoint_saved(seq: u64, thread_id: ThreadId) -> TraceRecord {
        wrap(
            thread_id,
            seq,
            TraceEvent::WaypointSaved {
                waypoint_id: WaypointId::generate(),
                superstep: 1,
                status: "completed".to_string(),
            },
        )
    }

    fn run_finished(seq: u64, thread_id: ThreadId) -> TraceRecord {
        wrap(
            thread_id,
            seq,
            TraceEvent::RunFinished {
                status: paladin_ports::output::trace_sink_port::RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 0,
                duration_ms: 5,
                trace_dropped_total: 0,
            },
        )
    }

    /// Behavior: records buffered since the last flush are appended as one
    /// batch when a `WaypointSaved` record arrives, and the buffer is empty
    /// afterwards (proven by the SAME record NOT being appended a second
    /// time on the next boundary).
    #[tokio::test]
    async fn persisting_sink_flushes_on_waypoint_saved() {
        let store = Arc::new(InMemoryRunTraceStore::new());
        let port: Arc<dyn RunTracePort> = store.clone();
        let sink = PersistingTraceSink::with_threshold(port, 1024);
        let thread_id = sample_thread();

        sink.on_event(superstep_started(1, thread_id.clone()))
            .await
            .unwrap();
        sink.on_event(waypoint_saved(2, thread_id.clone()))
            .await
            .unwrap();

        let persisted = store.read(&thread_id, 0, 100).await.unwrap();
        assert_eq!(
            persisted.len(),
            2,
            "both buffered records must reach the port in one flush"
        );

        // Nothing left buffered: a THIRD event, on its own, only persists
        // itself -- not a re-flush of records already appended (append is
        // idempotent on (thread_id, seq) anyway, but the buffer itself must
        // also be empty after the flush).
        sink.on_event(superstep_started(3, thread_id.clone()))
            .await
            .unwrap();
        sink.on_event(waypoint_saved(4, thread_id.clone()))
            .await
            .unwrap();
        let persisted_after = store.read(&thread_id, 0, 100).await.unwrap();
        assert_eq!(persisted_after.len(), 4);
    }

    /// Behavior: the terminal `RunFinished` record triggers a final flush
    /// including itself.
    #[tokio::test]
    async fn persisting_sink_flushes_on_run_finished() {
        let store = Arc::new(InMemoryRunTraceStore::new());
        let port: Arc<dyn RunTracePort> = store.clone();
        let sink = PersistingTraceSink::with_threshold(port, 1024);
        let thread_id = sample_thread();

        sink.on_event(superstep_started(1, thread_id.clone()))
            .await
            .unwrap();
        sink.on_event(run_finished(2, thread_id.clone()))
            .await
            .unwrap();

        let persisted = store.read(&thread_id, 0, 100).await.unwrap();
        assert_eq!(
            persisted.len(),
            2,
            "the RunFinished record itself must be included in its own flush"
        );
        assert!(
            persisted
                .iter()
                .any(|r| matches!(r.event, TraceEvent::RunFinished { .. })),
            "RunFinished must be among the persisted records"
        );
    }

    /// Behavior: a long superstep producing more than the configured
    /// buffer threshold flushes mid-superstep (no `WaypointSaved`/
    /// `RunFinished` involved at all), bounding memory.
    #[tokio::test]
    async fn persisting_sink_flushes_on_threshold() {
        let store = Arc::new(InMemoryRunTraceStore::new());
        let port: Arc<dyn RunTracePort> = store.clone();
        let threshold = 3;
        let sink = PersistingTraceSink::with_threshold(port, threshold);
        let thread_id = sample_thread();

        // Below threshold: nothing flushed yet.
        sink.on_event(superstep_started(1, thread_id.clone()))
            .await
            .unwrap();
        sink.on_event(superstep_started(2, thread_id.clone()))
            .await
            .unwrap();
        assert_eq!(store.read(&thread_id, 0, 100).await.unwrap().len(), 0);

        // The third record crosses the threshold -- all three flush.
        sink.on_event(superstep_started(3, thread_id.clone()))
            .await
            .unwrap();
        let persisted = store.read(&thread_id, 0, 100).await.unwrap();
        assert_eq!(
            persisted.len(),
            3,
            "the buffer must flush the instant it reaches the threshold"
        );
    }

    struct FailingRunTraceStore;

    #[async_trait]
    impl RunTracePort for FailingRunTraceStore {
        async fn append(&self, _records: &[TraceRecord]) -> Result<(), RunTraceError> {
            Err(RunTraceError::Backend {
                source: "deliberate test failure".into(),
            })
        }

        async fn read(
            &self,
            _thread: &ThreadId,
            _after_seq: u64,
            _limit: u32,
        ) -> Result<Vec<TraceRecord>, RunTraceError> {
            Ok(vec![])
        }

        async fn prune_thread(
            &self,
            _thread: &ThreadId,
            _before_superstep: u64,
        ) -> Result<u64, RunTraceError> {
            Ok(0)
        }
    }

    /// A [`PaladinPort`] that must never be called -- this test's graph has
    /// no `NodeSpec::Paladin` nodes.
    struct UnusedPaladinPort;

    #[async_trait]
    impl PaladinPort for UnusedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// A [`StateNode`] that counts its own executions.
    struct CountingNode {
        count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl StateNode for CountingNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, StateNodeError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(StateDelta::new().into())
        }
    }

    fn build_counting_graph(count: Arc<AtomicUsize>) -> Arc<WarGraph> {
        let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        graph.add_node(
            a.clone(),
            NodeSpec::Function(Arc::new(CountingNode {
                count: count.clone(),
            })),
        );
        graph.add_node(
            b.clone(),
            NodeSpec::Function(Arc::new(CountingNode { count })),
        );
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_entry(a);
        Arc::new(graph)
    }

    /// Behavior: a port returning `Err` on `append` is logged once and the
    /// sink returns `Ok(())`; a run executed with that failing port
    /// produces an identical outcome (terminal status, per-node execution
    /// count) to the same graph run with no persisting sink at all.
    #[tokio::test(flavor = "multi_thread")]
    async fn persisting_sink_write_failure_does_not_fail_the_run() {
        let store = Arc::new(InMemoryWaypointStore::new());

        let counter_without = Arc::new(AtomicUsize::new(0));
        let engine_without = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone());
        let outcome_without = engine_without
            .start(
                &build_counting_graph(counter_without.clone()),
                ThreadId::new("t-without-sink").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();

        let counter_with = Arc::new(AtomicUsize::new(0));
        let failing_port: Arc<dyn RunTracePort> = Arc::new(FailingRunTraceStore);
        let sink: Arc<dyn TraceSink> = Arc::new(PersistingTraceSink::new(failing_port));
        let engine_with =
            WarEngine::new(Arc::new(UnusedPaladinPort), store.clone()).with_trace_sink(sink);
        let outcome_with = engine_with
            .start(
                &build_counting_graph(counter_with.clone()),
                ThreadId::new("t-with-failing-sink").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();

        assert!(matches!(
            outcome_without,
            paladin_battalion::engine::RunOutcome::Completed { .. }
        ));
        assert!(matches!(
            outcome_with,
            paladin_battalion::engine::RunOutcome::Completed { .. }
        ));
        assert_eq!(
            counter_without.load(Ordering::SeqCst),
            counter_with.load(Ordering::SeqCst),
            "a persisting sink whose backend always fails must not change how many times a \
             node executes"
        );
        assert_eq!(counter_with.load(Ordering::SeqCst), 2);
    }

    /// Behavior: `build_run_sink` includes the persisting sink only when
    /// `trace.persist` is set AND a `RunTracePort` is available -- proven
    /// against the real `build_run_sink` composition point, not a re-
    /// implementation of its own logic.
    #[test]
    fn persisting_sink_only_attached_when_persist_is_on() {
        use crate::config::trace::TraceConfig;
        use crate::infrastructure::telemetry::build_run_sink;

        let store: Arc<dyn RunTracePort> = Arc::new(InMemoryRunTraceStore::new());

        // persist: false, port available -- must NOT attach (no other sink
        // configured either, so the whole result is None).
        let config_off = TraceConfig {
            log_sink: false,
            persist: false,
            ..TraceConfig::default()
        };
        assert!(build_run_sink(&config_off, None, Some(store.clone())).is_none());

        // persist: true, but no port available -- must NOT attach either.
        let config_on_no_port = TraceConfig {
            log_sink: false,
            persist: true,
            ..TraceConfig::default()
        };
        assert!(build_run_sink(&config_on_no_port, None, None).is_none());

        // persist: true AND a port available -- must attach.
        let config_on = TraceConfig {
            log_sink: false,
            persist: true,
            ..TraceConfig::default()
        };
        assert!(build_run_sink(&config_on, None, Some(store)).is_some());
    }
}
