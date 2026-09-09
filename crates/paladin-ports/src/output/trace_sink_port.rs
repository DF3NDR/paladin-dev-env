//! # Trace Sink Port — Standardized Execution Observability (ENG-FR-21, OBS-01)
//!
//! Defines [`TraceSink`] over the authoritative [`TraceRecord`] envelope
//! (now defined in `paladin-core`, re-exported here per D-01), plus the
//! synchronous [`TraceEmitter`] handle producers use to reach a dispatcher
//! (D-03) and [`CompositeSink`], the panic-isolating fan-out (D-08).
//!
//! ## Why the authoritative types live in `paladin-core`, re-exported here (D-01)
//!
//! ADR-0016 has core own port value types, with ports re-exporting them so
//! every existing `use paladin_ports::output::trace_sink_port::TraceEvent`
//! keeps compiling unchanged. `TraceEvent`/`TraceRecord`/`FieldChange`/
//! `NodeProgressKind`/`MiddlewareAction`/`RunFinishStatus`/
//! `TRACE_SCHEMA_VERSION` are defined in
//! `paladin_core::platform::container::trace` and `pub use`d below.
//!
//! ## Why a dedicated port, not a re-used one
//!
//! Every other output port in this crate abstracts a *dependency the run
//! needs* (a database, an LLM, a queue) — a failure there is the run's
//! failure too. `TraceSink` is the opposite shape by design: a run's
//! correctness must be **independent** of whether observability is attached,
//! connected, or even working. Folding this into an existing port would
//! import that port's own failure-matters semantics onto a trait that is
//! explicitly failure-*doesn't*-matter (X-01: no port may import a transport
//! or SDK client either way, but the difference here is behavioral, not
//! structural).
//!
//! ## `TraceRecord` carries field NAMES, not field VALUES
//!
//! See `paladin_core::platform::container::trace`'s own module docs for the
//! full "field NAMES, not VALUES" and redact-then-truncate rules (D-05).
//!
//! ## Errors are diagnostics only
//!
//! [`TraceSink::on_event`] returns `Result<(), TraceSinkError>` so an
//! implementation can surface its OWN failures (a network sink logging a
//! dropped connection, for instance) — but the return value is never
//! inspected by anything that decides a run's outcome. `TraceDispatcher`
//! (`paladin-battalion::engine::hooks`) discards it unconditionally.
//!
//! ## `TraceEmitter`: the synchronous handle producers reach a dispatcher
//! through (D-03)
//!
//! `TraceSink` is the CONSUMER side (what a sink implements to receive
//! records). `TraceEmitter` is the PRODUCER side: a synchronous,
//! never-awaiting, object-safe trait a `TraceDispatcher` implements behind
//! a cheap clonable `Arc<dyn TraceEmitter>` handle, so a producer below the
//! superstep engine (`FallbackLlmAdapter`, the facade's middleware chain,
//! `PaladinExecutionService`'s stream/tool paths) can emit an event without
//! ever awaiting anything or importing `paladin-battalion` itself.
//!
//! ## `CompositeSink`: panic-isolated fan-out (D-08)
//!
//! Forwards each record to every child sequentially, each child wrapped in
//! its own panic guard, so one panicking or erroring child never starves or
//! fails its siblings. Returns `Ok(())` unless every child errored — the
//! same "errors are diagnostics only" contract `TraceSink` itself carries.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use futures::FutureExt;
use thiserror::Error;
use tokio::sync::mpsc;

use paladin_core::platform::container::run::RunId;
pub use paladin_core::platform::container::trace::{
    FieldChange, MiddlewareAction, NodeProgressKind, RunFinishStatus, TRACE_SCHEMA_VERSION,
    TraceEvent, TraceRecord,
};
use paladin_core::platform::container::waypoint::ThreadId;

/// Errors a [`TraceSink`] implementation may report from its own handling of
/// an event.
///
/// Purely diagnostic: nothing that decides a run's outcome ever inspects
/// this. See the module-level "Errors are diagnostics only" section.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TraceSinkError {
    /// The sink failed to record or forward the event.
    #[error("trace sink error: {0}")]
    Failed(String),
}

/// Port trait for standardized execution observability (ENG-FR-21).
///
/// # Purpose
///
/// Gives the superstep engine one storage-and-transport-agnostic interface
/// for reporting its own execution as a typed event stream, without
/// depending on whether the consumer is a `println!` recorder in a test, an
/// OTel exporter, or the `paladin-eval` harness (Doc 07).
///
/// # Fire-and-forget contract
///
/// Nothing in this trait's signature enforces fire-and-forget — that
/// guarantee is the CALLER's responsibility
/// (`paladin-battalion::engine::hooks::TraceDispatcher`), not this trait's.
/// An implementor may do slow, fallible I/O inside `on_event` without
/// affecting engine correctness, because the dispatcher never awaits this
/// method on the engine's own execution path.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`: a sink may be invoked from a
/// background task while the engine itself keeps running concurrently.
#[async_trait]
pub trait TraceSink: Send + Sync {
    /// Handle one stamped [`TraceRecord`].
    ///
    /// The returned `Result` is diagnostic only — see the module-level
    /// "Errors are diagnostics only" section. An implementation that wants
    /// to observe every event exactly once, in order, still can: the
    /// dispatcher forwards records to a single sink instance sequentially
    /// (never concurrently), it just never blocks the run while doing so.
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError>;
}

/// The synchronous, never-awaiting handle a producer BELOW the superstep
/// engine reaches a `TraceDispatcher` through (D-03): the engine's own
/// [`TraceDispatcher`](paladin_core) stamps `seq`/`at`/`thread_id` at
/// enqueue time, so every producer sharing one handle for a run stamps from
/// the SAME counter, keeping `seq` order causal.
///
/// Object-safe (`Box<dyn TraceEmitter>` and `Arc<dyn TraceEmitter>` both
/// compile) and deliberately synchronous — `emit` never awaits, so a
/// producer can call it from a hot, non-async path without a runtime.
pub trait TraceEmitter: Send + Sync {
    /// Emit one [`TraceEvent`]. Never blocks and never fails visibly — a
    /// full queue drops the oldest buffered record (counted, never silent,
    /// see `TraceDispatcher::dropped_count`).
    fn emit(&self, event: TraceEvent);
}

/// A panic-isolated fan-out to every child [`TraceSink`] (D-08).
///
/// Forwards each record to every child SEQUENTIALLY, in vector order, each
/// child wrapped in its own `catch_unwind` guard, so one panicking or
/// erroring child neither starves nor fails its siblings. Returns `Ok(())`
/// unless EVERY child errored (or panicked) — the same "errors are
/// diagnostics only" contract [`TraceSink`] itself carries.
pub struct CompositeSink {
    sinks: Vec<Arc<dyn TraceSink>>,
}

impl CompositeSink {
    /// Construct a fan-out over `sinks`, forwarded to in vector order.
    pub fn new(sinks: Vec<Arc<dyn TraceSink>>) -> Self {
        Self { sinks }
    }

    /// Append one more child sink, forwarded to after every sink already
    /// present.
    pub fn push(&mut self, sink: Arc<dyn TraceSink>) {
        self.sinks.push(sink);
    }
}

#[async_trait]
impl TraceSink for CompositeSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        if self.sinks.is_empty() {
            return Ok(());
        }
        let mut all_failed = true;
        for sink in &self.sinks {
            let outcome = AssertUnwindSafe(sink.on_event(record.clone()))
                .catch_unwind()
                .await;
            match outcome {
                Ok(Ok(())) => all_failed = false,
                Ok(Err(_)) => {
                    // Diagnostic only — this child failed, its siblings
                    // still run.
                }
                Err(_panic) => {
                    // A panicking child must never starve or fail its
                    // siblings (D-08).
                    log::error!(
                        target: "paladin::trace",
                        "a CompositeSink child panicked while handling a trace record"
                    );
                }
            }
        }
        if all_failed {
            return Err(TraceSinkError::Failed(
                "every CompositeSink child failed or panicked".to_string(),
            ));
        }
        Ok(())
    }
}

/// A minimal, self-contained [`TraceEmitter`] for a producer that has no
/// engine-owned [`TraceDispatcher`](paladin_battalion) to reach (28-06,
/// D-03): a bare `FallbackLlmAdapter` under unit test, or any below-engine
/// producer constructed standalone (outside a run the worker's own
/// composition root wired).
///
/// Stamps its own gapless, 1-based `seq` counter under a fixed `thread_id`/
/// `run_id` pair, exactly like `TraceDispatcher` stamps one per run --
/// `StandaloneEmitter` is simply a dispatcher scoped to "this producer's own
/// process lifetime" instead of "this run". With no sink attached, `emit` is
/// a zero-cost no-op (no channel, no consumer task) — the same untraced-path
/// guarantee `TraceDispatcher::new` gives with `sink: None`. With a sink
/// attached, one background task (spawned at construction) drains an
/// unbounded channel and forwards each record — this is a low-volume,
/// standalone producer, not a run-scale queue, so no drop-oldest bound is
/// needed here the way `TraceDispatcher`'s own queue needs one.
pub struct StandaloneEmitter {
    thread_id: ThreadId,
    run_id: Option<RunId>,
    seq: AtomicU64,
    sender: Option<mpsc::UnboundedSender<TraceRecord>>,
}

impl StandaloneEmitter {
    /// Construct a `StandaloneEmitter` stamping every record with
    /// `thread_id`/`run_id`, forwarding to `sink` if attached. `None`
    /// spawns no background task and allocates no channel.
    pub fn new(
        thread_id: ThreadId,
        run_id: Option<RunId>,
        sink: Option<Arc<dyn TraceSink>>,
    ) -> Self {
        let Some(sink) = sink else {
            return Self {
                thread_id,
                run_id,
                seq: AtomicU64::new(0),
                sender: None,
            };
        };
        let (tx, mut rx) = mpsc::unbounded_channel::<TraceRecord>();
        tokio::spawn(async move {
            while let Some(record) = rx.recv().await {
                let outcome = AssertUnwindSafe(sink.on_event(record)).catch_unwind().await;
                if outcome.is_err() {
                    log::error!(
                        target: "paladin::trace",
                        "a TraceSink panicked while handling a StandaloneEmitter record; the consumer task continues"
                    );
                }
            }
        });
        Self {
            thread_id,
            run_id,
            seq: AtomicU64::new(0),
            sender: Some(tx),
        }
    }
}

impl TraceEmitter for StandaloneEmitter {
    fn emit(&self, event: TraceEvent) {
        let Some(sender) = &self.sender else {
            return;
        };
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let record = TraceRecord {
            thread_id: self.thread_id.clone(),
            run_id: self.run_id.clone(),
            seq,
            at: Utc::now(),
            event,
        };
        // Fire-and-forget: an unbounded channel's `send` never blocks and
        // fails only if the consumer task has already been dropped, which
        // never happens while `self` (which owns `sender`) is alive.
        let _ = sender.send(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::waypoint::ThreadId;
    use tokio::sync::Mutex;

    struct MockTraceSink;

    #[async_trait]
    impl TraceSink for MockTraceSink {
        async fn on_event(&self, _record: TraceRecord) -> Result<(), TraceSinkError> {
            Ok(())
        }
    }

    fn sample_record(event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id: ThreadId::new("t1").unwrap(),
            run_id: None,
            seq: 1,
            at: chrono::Utc::now(),
            event,
        }
    }

    #[tokio::test]
    async fn mock_sink_implements_trait() {
        let sink = MockTraceSink;
        let record = sample_record(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        assert!(sink.on_event(record).await.is_ok());
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn TraceSink>> = None;
    }

    /// D-03: `TraceEmitter` must be object-safe too — a producer holds
    /// `Arc<dyn TraceEmitter>`, never a concrete dispatcher type.
    #[test]
    fn emitter_trait_is_object_safe() {
        let _: Option<Box<dyn TraceEmitter>> = None;
        let _: Option<Arc<dyn TraceEmitter>> = None;
    }

    struct RecordingSink {
        events: Mutex<Vec<TraceRecord>>,
    }

    impl RecordingSink {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
            })
        }
    }

    #[async_trait]
    impl TraceSink for RecordingSink {
        async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
            self.events.lock().await.push(record);
            Ok(())
        }
    }

    struct PanickingSink;

    #[async_trait]
    impl TraceSink for PanickingSink {
        async fn on_event(&self, _record: TraceRecord) -> Result<(), TraceSinkError> {
            panic!("simulated panic in CompositeSink child");
        }
    }

    struct AlwaysErroringSink;

    #[async_trait]
    impl TraceSink for AlwaysErroringSink {
        async fn on_event(&self, _record: TraceRecord) -> Result<(), TraceSinkError> {
            Err(TraceSinkError::Failed("simulated failure".to_string()))
        }
    }

    /// D-08: every child receives the record, in vector order, and a
    /// panicking first child does not prevent a healthy second child from
    /// receiving it.
    #[tokio::test]
    async fn composite_sink_forwards_to_every_child_even_when_one_panics() {
        let a = Arc::new(PanickingSink);
        let b = RecordingSink::new();
        let composite = CompositeSink::new(vec![a, b.clone()]);

        let record = sample_record(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        let result = composite.on_event(record).await;
        assert!(
            result.is_ok(),
            "a panicking child must not fail on_event when a sibling succeeds"
        );
        assert_eq!(b.events.lock().await.len(), 1);
    }

    /// D-08: `on_event` returns `Err` only when EVERY child failed.
    #[tokio::test]
    async fn composite_sink_errs_only_when_every_child_fails() {
        let a = Arc::new(AlwaysErroringSink);
        let b = Arc::new(PanickingSink);
        let composite = CompositeSink::new(vec![a, b]);

        let record = sample_record(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        let result = composite.on_event(record).await;
        assert!(
            result.is_err(),
            "on_event must error when every child failed or panicked"
        );
    }

    /// D-08: with at least one succeeding child, `on_event` is `Ok`.
    #[tokio::test]
    async fn composite_sink_ok_when_at_least_one_child_succeeds() {
        let a = Arc::new(AlwaysErroringSink);
        let b = RecordingSink::new();
        let composite = CompositeSink::new(vec![a, b.clone()]);

        let record = sample_record(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        assert!(composite.on_event(record).await.is_ok());
        assert_eq!(b.events.lock().await.len(), 1);
    }

    #[test]
    fn all_twelve_event_variants_construct_via_reexport() {
        let thread_id = ThreadId::new("t1").unwrap();
        let _ = TraceRecord {
            thread_id: thread_id.clone(),
            run_id: None,
            seq: 1,
            at: chrono::Utc::now(),
            event: TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 0,
                total_tokens: 0,
                duration_ms: 0,
                trace_dropped_total: 0,
            },
        };
    }

    /// D-25: the fallback adapter's hop event carries an OPTIONAL node id
    /// (always `None` from the adapter) plus both provider names.
    #[test]
    fn fallback_hop_variant_constructs_with_no_node_id() {
        let event = TraceEvent::FallbackHop {
            node_id: None,
            from_provider: "openai".to_string(),
            to_provider: "anthropic".to_string(),
        };
        match event {
            TraceEvent::FallbackHop {
                node_id,
                from_provider,
                to_provider,
            } => {
                assert!(node_id.is_none());
                assert_eq!(from_provider, "openai");
                assert_eq!(to_provider, "anthropic");
            }
            _ => panic!("expected FallbackHop"),
        }
    }

    /// 28-06: `StandaloneEmitter` stamps its own gapless, 1-based `seq`
    /// sequence and forwards every record to its attached sink -- a
    /// below-engine producer constructed with no run-scoped
    /// `TraceDispatcher` at all still produces well-formed records.
    #[tokio::test]
    async fn standalone_emitter_stamps_its_own_gapless_sequence() {
        let sink = RecordingSink::new();
        let emitter = StandaloneEmitter::new(
            ThreadId::new("standalone").unwrap(),
            None,
            Some(sink.clone()),
        );

        emitter.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        emitter.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });

        // The background consumer task drains asynchronously -- give it a
        // chance to run before asserting.
        tokio::task::yield_now().await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let events = sink.events.lock().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[0].thread_id, ThreadId::new("standalone").unwrap());
    }

    /// With no sink attached, `emit` is a zero-cost no-op: no channel, no
    /// consumer task, and `seq` never observably advances (mirrors
    /// `TraceDispatcher`'s own no-sink contract).
    #[test]
    fn standalone_emitter_with_no_sink_is_a_no_op() {
        let emitter = StandaloneEmitter::new(ThreadId::new("standalone").unwrap(), None, None);
        emitter.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".to_string(),
        });
        // No panic, no observable effect -- nothing further to assert
        // without a sink to inspect.
    }
}
