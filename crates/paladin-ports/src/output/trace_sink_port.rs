//! # Trace Sink Port — Standardized Execution Observability (ENG-FR-21)
//!
//! Defines [`TraceSink`] and its typed [`TraceEvent`] stream: the seam Doc 07
//! (`paladin-eval`, OTel export, graph visualization) plugs an observability
//! consumer into. Phase 22 Plan 09 lands the trait and its event shape with
//! **no consumer** — `paladin-battalion`'s `engine::hooks::TraceDispatcher`
//! is the only thing that calls [`TraceSink::on_event`], and it does so
//! fire-and-forget over a bounded, drop-oldest queue so a slow or failing
//! sink can never stall or fail a run (T-22-30).
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
//! ## `TraceEvent` carries field NAMES, not field VALUES
//!
//! `DeltaMerged` reports which [`FieldName`]s changed in a merge, never their
//! `serde_json::Value` contents (T-22-32). Attaching an exporter that ships
//! events off-process must not, by itself, export the shared Battlefield
//! state — a consumer that wants values reads them from the `Waypoint`
//! through `WaypointPort`, a port whose whole contract is durable, at-rest
//! persistence rather than a live telemetry stream.
//!
//! ## Errors are diagnostics only
//!
//! [`TraceSink::on_event`] returns `Result<(), TraceSinkError>` so an
//! implementation can surface its OWN failures (a network sink logging a
//! dropped connection, for instance) — but the return value is never
//! inspected by anything that decides a run's outcome. `TraceDispatcher`
//! (`paladin-battalion::engine::hooks`) discards it unconditionally.

use async_trait::async_trait;
use thiserror::Error;

use paladin_core::platform::container::battlefield::FieldName;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointId};

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

/// One typed observability event emitted by the superstep engine
/// (ENG-FR-21) — plus [`TraceEvent::FallbackHop`], emitted by the
/// `FallbackLlmAdapter` below the engine (Doc 04 D-25). Marked `#[non_exhaustive]`
/// because Doc 07 is expected to extend this set for the eval harness and
/// OTel export without that being a breaking change for existing sinks (a
/// `match` over `TraceEvent` must always carry a wildcard arm).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TraceEvent {
    /// A run started (`WarEngine::start` or `WarEngine::resume`).
    RunStarted {
        /// The thread whose run started.
        thread_id: ThreadId,
    },
    /// A superstep began.
    SuperstepStarted {
        /// The thread this superstep belongs to.
        thread_id: ThreadId,
        /// The superstep index that began.
        superstep: u64,
    },
    /// One attempt of a node's execution began (Doc 04 D-16): emitted once
    /// PER ATTEMPT, so a node retried under an Aegis retry policy produces
    /// one `NodeStarted`/`NodeFinished` pair per attempt, each carrying its
    /// own `attempt` number. Field names match PRD 07 §2's span-per-attempt
    /// shape so Phase 28's OTel export renames nothing.
    NodeStarted {
        /// The thread this execution belongs to.
        thread_id: ThreadId,
        /// The superstep this execution belongs to.
        superstep: u64,
        /// The node that started executing.
        node_id: NodeId,
        /// The 1-indexed attempt this event belongs to; `1` for a node
        /// with no retry policy.
        attempt: u32,
    },
    /// One attempt of a node's execution finished, successfully or not
    /// (Doc 04 D-16): emitted once per attempt, paired with the
    /// `NodeStarted` carrying the same `attempt`.
    NodeFinished {
        /// The thread this execution belongs to.
        thread_id: ThreadId,
        /// The superstep this execution belongs to.
        superstep: u64,
        /// The node that finished executing.
        node_id: NodeId,
        /// The 1-indexed attempt this event belongs to; `1` for a node
        /// with no retry policy.
        attempt: u32,
        /// Whether this attempt's outcome was served from the node cache
        /// (FT-06) instead of by executing the node. Always `false` until
        /// plan 25-13 wires the cache lookup.
        cache_hit: bool,
    },
    /// A superstep's collected deltas were merged into the Battlefield.
    DeltaMerged {
        /// The thread this merge belongs to.
        thread_id: ThreadId,
        /// The superstep this merge belongs to.
        superstep: u64,
        /// The fields whose value changed by this merge (names only — see
        /// the module-level "carries field NAMES" section).
        field_changes: Vec<FieldName>,
    },
    /// A Waypoint was persisted.
    WaypointSaved {
        /// The thread the waypoint belongs to.
        thread_id: ThreadId,
        /// The persisted waypoint's identity.
        waypoint_id: WaypointId,
    },
    /// A run finished, with any terminal `RunOutcome`.
    RunFinished {
        /// The thread whose run finished.
        thread_id: ThreadId,
    },
    /// A `FallbackLlmAdapter` chain (Doc 04 FT-FR-16, D-25) gave up on one
    /// provider and moved to the next. Emitted once PER HOP, before the next
    /// provider is called, so a three-provider chain that lands on its third
    /// element produces exactly two of these. Field names match PRD 07 §2's
    /// shape so Phase 28's OTel export renames nothing.
    FallbackHop {
        /// The node the hop happened on behalf of. Always `None` when the
        /// event comes from the adapter itself: a plain `LlmPort` composed
        /// below the superstep engine cannot know which node it is serving.
        /// Phase 28 may enrich this from the engine side.
        node_id: Option<NodeId>,
        /// `get_provider_name()` of the provider that failed.
        from_provider: String,
        /// `get_provider_name()` of the provider the chain moves to.
        to_provider: String,
    },
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
    /// Handle one [`TraceEvent`].
    ///
    /// The returned `Result` is diagnostic only — see the module-level
    /// "Errors are diagnostics only" section. An implementation that wants
    /// to observe every event exactly once, in order, still can: the
    /// dispatcher forwards events to a single sink instance sequentially
    /// (never concurrently), it just never blocks the run while doing so.
    async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTraceSink;

    #[async_trait]
    impl TraceSink for MockTraceSink {
        async fn on_event(&self, _event: TraceEvent) -> Result<(), TraceSinkError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn mock_sink_implements_trait() {
        let sink = MockTraceSink;
        let event = TraceEvent::RunStarted {
            thread_id: ThreadId::new("t1").unwrap(),
        };
        assert!(sink.on_event(event).await.is_ok());
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Option<Box<dyn TraceSink>> = None;
    }

    #[test]
    fn all_seven_event_variants_construct() {
        let thread_id = ThreadId::new("t1").unwrap();
        let _ = TraceEvent::RunStarted {
            thread_id: thread_id.clone(),
        };
        let _ = TraceEvent::SuperstepStarted {
            thread_id: thread_id.clone(),
            superstep: 1,
        };
        let _ = TraceEvent::NodeStarted {
            thread_id: thread_id.clone(),
            superstep: 1,
            node_id: NodeId::new("n1"),
            attempt: 1,
        };
        let _ = TraceEvent::NodeFinished {
            thread_id: thread_id.clone(),
            superstep: 1,
            node_id: NodeId::new("n1"),
            attempt: 1,
            cache_hit: false,
        };
        let _ = TraceEvent::DeltaMerged {
            thread_id: thread_id.clone(),
            superstep: 1,
            field_changes: vec![FieldName::new("x").unwrap()],
        };
        let _ = TraceEvent::WaypointSaved {
            thread_id: thread_id.clone(),
            waypoint_id: WaypointId::generate(),
        };
        let _ = TraceEvent::RunFinished { thread_id };
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
}
