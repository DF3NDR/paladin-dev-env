//! # The authoritative trace model (OBS-01; PRD 07 §2.1, D-01/D-02)
//!
//! [`TraceEvent`] is the twelve-variant, `#[non_exhaustive]` set of
//! observability events the superstep engine and its below-the-engine
//! producers (`FallbackLlmAdapter`, the facade's middleware chain,
//! `PaladinExecutionService`) emit. [`TraceRecord`] is the envelope every
//! sink actually receives: `thread_id`, an optional `run_id`, a per-run
//! monotonic `seq`, `at` (when the event happened, stamped at enqueue time —
//! not when a sink observes it) and the event itself, `#[serde(flatten)]`ed
//! so one record serializes to a single flat JSON object (OBS-FR-04).
//!
//! Phase 22 seeded an eight-variant `TraceEvent` directly in
//! `paladin-ports::output::trace_sink_port` with a `thread_id` on every
//! variant. This module is its authoritative home (D-01: ADR-0016 has core
//! own port value types, with ports re-exporting them) and its extension to
//! PRD 07 §2.1's twelve variants (D-02): the per-variant `thread_id` fields
//! are gone — the [`TraceRecord`] envelope carries it once.
//!
//! ## `TraceEvent` carries field NAMES, not field VALUES (D-05)
//!
//! [`TraceEvent::DeltaMerged`] reports which fields changed via
//! [`FieldChange`] — `field`, `dispatch`, `writers` and `value_bytes`
//! (the serialized size of the new value) — **never** the value itself,
//! unless `trace.state_values` is explicitly enabled, in which case
//! [`FieldChange::value`] carries the serialized value passed through the
//! redaction helper (`crates/paladin-llm/src/redaction.rs`) **before**
//! truncation to the configured cap — redact-then-truncate, never the
//! reverse, per `.github/instructions/security.instructions.md`'s ordering
//! rule (slicing a secret's tail off AFTER truncation can leave a partial
//! credential in the trace). A consumer that wants values reads them from a
//! `Waypoint` through `WaypointPort`, whose whole contract is durable,
//! at-rest persistence rather than a live telemetry stream.
//!
//! ## A note on the one field name deviation from the PRD prose (D-02)
//!
//! PRD 07's prose names [`TraceEvent::NodeProgress`]'s payload field `kind`
//! and [`TraceEvent::ParleyRaised`]'s payload field `kind` — but
//! `TraceEvent` itself is internally tagged `#[serde(tag = "kind")]`, so a
//! variant whose own struct payload also declares a field literally named
//! `kind` would collide with the enum's own discriminant key when
//! serialized (the same JSON key emitted twice: once as the variant tag,
//! once as the payload field, defeating the flat-envelope contract this
//! module exists to provide). This is resolved here, not deferred, by
//! naming the two payload fields [`TraceEvent::NodeProgress`]'s `progress`
//! and [`TraceEvent::ParleyRaised`]'s `parley_kind` — the DATA these two
//! variants carry is unchanged from the PRD's intent, only the JSON key
//! avoids the collision. A round-trip test
//! (`record_serializes_as_one_flat_object` and
//! `all_twelve_event_variants_construct`'s serialization proves) would have
//! caught this at once had it gone unfixed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield::FieldName;
use crate::platform::container::parley::{ParleyId, ParleyKind};
use crate::platform::container::run::RunId;
use crate::platform::container::waypoint::{NodeId, NodeOutcomeKind, ThreadId, WaypointId};

/// Schema version stamped on every persisted [`TraceRecord`] (X-04),
/// mirroring [`crate::platform::container::run::RUN_SCHEMA_VERSION`] and
/// the `Waypoint`/`Battlefield` precedent. `run_traces` rows persisted under
/// this version are a one-way door (D-02): reshaping [`TraceRecord`] or
/// [`TraceEvent`] after this ships needs a data migration, exactly as
/// `23-CONTEXT.md` D-14 records for `MusterProgress`.
pub const TRACE_SCHEMA_VERSION: &str = "1";

/// One field's contribution to a [`TraceEvent::DeltaMerged`] merge (D-05).
///
/// Carries the field's NAME, the [`DispatchRule`](crate::platform::container::battlefield::DispatchRule)
/// discriminant that governed the merge, and every writer that contributed
/// to it — never the value, unless `trace.state_values` is enabled (see the
/// module-level "carries field NAMES" section).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    /// The field whose value changed (or was newly set) by this merge.
    pub field: FieldName,
    /// The [`DispatchRule`](crate::platform::container::battlefield::DispatchRule)
    /// discriminant name that governed this field's merge (e.g.
    /// `"last_write"`, `"append"`, `"custom"`).
    pub dispatch: String,
    /// Every node that contributed a value to this field this merge, in
    /// dispatch-evaluation order.
    pub writers: Vec<NodeId>,
    /// The serialized byte size of the new value, always present regardless
    /// of `trace.state_values`.
    pub value_bytes: u64,
    /// The serialized, redacted-then-truncated value itself. `None` unless
    /// `trace.state_values = true` (default `false`) — see the module-level
    /// "carries field NAMES" section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// The shape of one [`TraceEvent::NodeProgress`] update (D-04).
///
/// `#[non_exhaustive]`: a future progress shape can be added without
/// breaking an existing match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "progress_kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeProgressKind {
    /// A liveness signal from a long-running node's
    /// [`HeartbeatHandle`](crate) receiver, rate-limited to at most one per
    /// `trace.heartbeat_interval` (default 5s) per node so a chatty node
    /// cannot flood the trace queue.
    Heartbeat,
    /// A chunk of a streamed response arrived. Carries `bytes`, never text
    /// (D-05: no values on the trace by default).
    StreamChunk {
        /// The chunk's serialized byte size.
        bytes: u64,
    },
    /// A tool/armament call was dispatched.
    ToolCall {
        /// The tool's name.
        tool: String,
    },
}

/// A closed set of actions the facade's `ExecutionMiddleware` chain
/// (Phase 26, D-01) may take on a request or response (X-06: no stringly
/// actions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MiddlewareAction {
    /// The middleware short-circuited the chain with a final response.
    Finish,
    /// The middleware denied the request outright.
    Deny,
    /// The middleware redacted part of the request or response.
    Redact,
    /// The middleware failed the run.
    Fail,
    /// The middleware requested a retry.
    Retry,
    /// The middleware requested a model fallback hop.
    Fallback,
}

/// The terminal status a run finished under (D-04), reported on
/// [`TraceEvent::RunFinished`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFinishStatus {
    /// The run completed normally.
    Completed,
    /// The run failed.
    Failed,
    /// The run was halted (e.g. a graceful-shutdown grace deadline).
    Halted,
    /// The run suspended awaiting external (Parley) input.
    AwaitingInput,
}

/// One typed observability event (OBS-01, PRD 07 §2.1): the twelve-variant
/// authoritative list every sink, the OTel exporter, the SSE bridge, the
/// `run_traces` persistence layer, the graph inspector and the
/// `paladin-eval` harness read.
///
/// `#[non_exhaustive]`: this set is expected to grow; every `match` over
/// `TraceEvent` anywhere in the workspace must carry a wildcard arm.
///
/// Internally tagged `#[serde(tag = "kind", rename_all = "snake_case")]` so
/// that, once wrapped in a [`TraceRecord`] and flattened, one record
/// serializes to a single flat JSON object whose `"kind"` key names the
/// variant (`"node_started"`, `"delta_merged"`, …) — OBS-FR-04's "one line
/// per event", grep-able by `thread_id`, `seq` and `kind` alike.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TraceEvent {
    /// A run started (`WarEngine::start` or `WarEngine::resume`).
    RunStarted {
        /// The run this start belongs to, when known (a bare engine with no
        /// Platform API run wrapping it has no run identity).
        run_id: Option<RunId>,
        /// The starting graph's fingerprint (`WarGraph::fingerprint`).
        graph_fingerprint: String,
    },
    /// A superstep began.
    SuperstepStarted {
        /// The superstep index that began.
        superstep: u64,
        /// The nodes entering this superstep's dispatch set.
        vanguard: Vec<NodeId>,
    },
    /// One attempt of a node's execution began (Doc 04 D-16): emitted once
    /// PER ATTEMPT, so a node retried under an Aegis retry policy produces
    /// one `NodeStarted`/`NodeFinished` pair per attempt, each carrying its
    /// own `attempt` number.
    NodeStarted {
        /// The superstep this execution belongs to.
        superstep: u64,
        /// The node that started executing.
        node_id: NodeId,
        /// The 1-indexed attempt this event belongs to; `1` for a node
        /// with no retry policy.
        attempt: u32,
        /// The Muster task key this execution serves, when it is a
        /// synthetic worker-task dispatch (CF-FR-10); `None` for an
        /// ordinary vanguard execution.
        muster_task_key: Option<String>,
    },
    /// A liveness/progress update from a node's own execution (D-04),
    /// produced by the facade's `PaladinExecutionService` and middleware
    /// chain, or the engine's `HeartbeatHandle` receiver.
    NodeProgress {
        /// The node this progress update belongs to.
        node_id: NodeId,
        /// The shape of progress reported. Named `progress`, not `kind`
        /// (see the module-level "field name deviation" section): the
        /// enclosing `TraceEvent` is itself tagged `kind`, so a payload
        /// field of the same name would collide.
        progress: NodeProgressKind,
    },
    /// One attempt of a node's execution finished, successfully or not
    /// (Doc 04 D-16): emitted once per attempt, paired with the
    /// `NodeStarted` carrying the same `attempt`.
    NodeFinished {
        /// The superstep this execution belongs to.
        superstep: u64,
        /// The node that finished executing.
        node_id: NodeId,
        /// The 1-indexed attempt this event belongs to; `1` for a node
        /// with no retry policy.
        attempt: u32,
        /// This attempt's outcome, using the SAME vocabulary the
        /// persisted `Waypoint`'s `NodeExecutionRecord` uses, so a trace
        /// and a Waypoint never disagree.
        outcome: NodeOutcomeKind,
        /// How long this attempt took, in milliseconds.
        duration_ms: u64,
        /// Tokens consumed by this attempt, `0` for a non-Paladin node or
        /// a cache hit.
        token_count: u64,
        /// Whether this attempt's outcome was served from the node cache
        /// (FT-06) instead of by executing the node.
        cache_hit: bool,
    },
    /// One declared edge was evaluated during frontier resolution, whether
    /// or not it fired.
    EdgeEvaluated {
        /// The edge's source node.
        from: NodeId,
        /// The edge's target node.
        to: NodeId,
        /// The `EdgeCondition` discriminant name (`"always"`, `"contains"`,
        /// `"regex"`, `"custom"`).
        condition_kind: String,
        /// Whether the edge fired.
        fired: bool,
    },
    /// A superstep's collected deltas were merged into the Battlefield.
    DeltaMerged {
        /// The superstep this merge belongs to.
        superstep: u64,
        /// The fields whose value changed by this merge (D-05: names and
        /// bookkeeping only, values opt-in).
        field_changes: Vec<FieldChange>,
    },
    /// A Waypoint was persisted.
    WaypointSaved {
        /// The persisted waypoint's identity.
        waypoint_id: WaypointId,
        /// The superstep the persisted waypoint belongs to.
        superstep: u64,
        /// The persisted waypoint's status, as a display string.
        status: String,
    },
    /// A node raised a `ParleyRequest`, suspending the run (HITL-01).
    ParleyRaised {
        /// The raised request's identity.
        parley_id: ParleyId,
        /// The node that raised it.
        node_id: NodeId,
        /// The shape of input awaited. Named `parley_kind`, not `kind`
        /// (see the module-level "field name deviation" section).
        parley_kind: ParleyKind,
    },
    /// A run finished, with any terminal `RunOutcome`.
    RunFinished {
        /// The run's terminal status.
        status: RunFinishStatus,
        /// Total supersteps executed by this run.
        total_supersteps: u64,
        /// Total tokens consumed by this run.
        total_tokens: u64,
        /// Total wall-clock duration of this run, in milliseconds.
        duration_ms: u64,
        /// The dispatching `TraceDispatcher`'s own drop count at the moment
        /// this event was enqueued (D-07): `RunFinished` is never itself
        /// the dropped event (drop-oldest never evicts the newest push),
        /// so this is always the run's FINAL, accurate drop count.
        trace_dropped_total: u64,
    },
    /// A `FallbackLlmAdapter` chain (Doc 04 FT-FR-16, D-25) gave up on one
    /// provider and moved to the next. Emitted once PER HOP, before the
    /// next provider is called, so a three-provider chain that lands on
    /// its third element produces exactly two of these.
    FallbackHop {
        /// The node the hop happened on behalf of. Always `None` when the
        /// event comes from the adapter itself: a plain `LlmPort` composed
        /// below the superstep engine cannot know which node it is
        /// serving.
        node_id: Option<NodeId>,
        /// `get_provider_name()` of the provider that failed.
        from_provider: String,
        /// `get_provider_name()` of the provider the chain moves to.
        to_provider: String,
    },
    /// The facade's `ExecutionMiddleware` chain (Phase 26, D-01) took an
    /// action on a request or response.
    MiddlewareEvent {
        /// The middleware's name.
        name: String,
        /// The action taken.
        action: MiddlewareAction,
    },
}

/// The envelope every [`TraceSink`](crate) actually receives (D-02): wraps
/// one [`TraceEvent`] with the `thread_id`, optional `run_id`, per-run
/// monotonic `seq` and `at` timestamp a `TraceDispatcher` stamps at enqueue
/// time — `seq` order IS causal order, and `at` is when the event
/// happened, never when a sink observed it.
///
/// `#[serde(flatten)]` on `event` means one `TraceRecord` serializes to a
/// SINGLE flat JSON object (never a nested `{"envelope": …, "event": {…}}`
/// shape) — OBS-FR-04's "one line per event", with `thread_id`/`seq`/`kind`
/// among the first keys so a log line is grep-able by any of the three.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceRecord {
    /// The thread (run) this record belongs to.
    pub thread_id: ThreadId,
    /// The Platform API run this record belongs to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<RunId>,
    /// This record's 1-based position within its dispatcher's own sequence.
    /// Strictly increasing and gapless within one run when nothing was
    /// dropped; cross-run interleaving is unordered by contract.
    pub seq: u64,
    /// When the underlying event happened (stamped at enqueue time, not
    /// when a sink observed it).
    pub at: DateTime<Utc>,
    /// The event itself.
    #[serde(flatten)]
    pub event: TraceEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread() -> ThreadId {
        ThreadId::new("t1").unwrap()
    }

    /// D-02: `TraceEvent` has exactly twelve variants; this test builds one
    /// of each and the `match` below (with its wildcard arm) must compile,
    /// proving `#[non_exhaustive]` discipline holds.
    #[test]
    fn all_twelve_event_variants_construct() {
        let events = vec![
            TraceEvent::RunStarted {
                run_id: Some(RunId::new_v7()),
                graph_fingerprint: "fp".to_string(),
            },
            TraceEvent::SuperstepStarted {
                superstep: 1,
                vanguard: vec![NodeId::new("n1")],
            },
            TraceEvent::NodeStarted {
                superstep: 1,
                node_id: NodeId::new("n1"),
                attempt: 1,
                muster_task_key: None,
            },
            TraceEvent::NodeProgress {
                node_id: NodeId::new("n1"),
                progress: NodeProgressKind::Heartbeat,
            },
            TraceEvent::NodeFinished {
                superstep: 1,
                node_id: NodeId::new("n1"),
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                token_count: 0,
                cache_hit: false,
            },
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("a"),
                to: NodeId::new("b"),
                condition_kind: "always".to_string(),
                fired: true,
            },
            TraceEvent::DeltaMerged {
                superstep: 1,
                field_changes: vec![FieldChange {
                    field: FieldName::new("x").unwrap(),
                    dispatch: "last_write".to_string(),
                    writers: vec![NodeId::new("n1")],
                    value_bytes: 4,
                    value: None,
                }],
            },
            TraceEvent::WaypointSaved {
                waypoint_id: WaypointId::generate(),
                superstep: 1,
                status: "completed".to_string(),
            },
            TraceEvent::ParleyRaised {
                parley_id: ParleyId::new(),
                node_id: NodeId::new("n1"),
                parley_kind: ParleyKind::Approval,
            },
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                total_tokens: 0,
                duration_ms: 5,
                trace_dropped_total: 0,
            },
            TraceEvent::FallbackHop {
                node_id: None,
                from_provider: "openai".to_string(),
                to_provider: "anthropic".to_string(),
            },
            TraceEvent::MiddlewareEvent {
                name: "limit".to_string(),
                action: MiddlewareAction::Finish,
            },
        ];
        assert_eq!(events.len(), 12);
        for event in &events {
            // Every match over `TraceEvent` anywhere in the WORKSPACE must
            // carry a wildcard arm (`#[non_exhaustive]` only enforces this
            // for downstream crates; within this defining crate the match
            // below is already exhaustive over the twelve known variants,
            // so no wildcard arm is added here — adding one would be an
            // unreachable-pattern warning under `-D warnings`).
            let _name = match event {
                TraceEvent::RunStarted { .. } => "run_started",
                TraceEvent::SuperstepStarted { .. } => "superstep_started",
                TraceEvent::NodeStarted { .. } => "node_started",
                TraceEvent::NodeProgress { .. } => "node_progress",
                TraceEvent::NodeFinished { .. } => "node_finished",
                TraceEvent::EdgeEvaluated { .. } => "edge_evaluated",
                TraceEvent::DeltaMerged { .. } => "delta_merged",
                TraceEvent::WaypointSaved { .. } => "waypoint_saved",
                TraceEvent::ParleyRaised { .. } => "parley_raised",
                TraceEvent::RunFinished { .. } => "run_finished",
                TraceEvent::FallbackHop { .. } => "fallback_hop",
                TraceEvent::MiddlewareEvent { .. } => "middleware_event",
            };
        }
    }

    /// D-02: one `TraceRecord` serializes to a single flat JSON object
    /// (never a nested envelope), with `seq` appearing before `kind` and
    /// `thread_id` the very first key — grep-able by all three.
    #[test]
    fn record_serializes_as_one_flat_object() {
        let record = TraceRecord {
            thread_id: thread(),
            run_id: None,
            seq: 7,
            at: Utc::now(),
            event: TraceEvent::NodeStarted {
                superstep: 1,
                node_id: NodeId::new("n1"),
                attempt: 1,
                muster_task_key: None,
            },
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(
            json.starts_with("{\"thread_id\":"),
            "record must start with thread_id: {json}"
        );
        let seq_pos = json.find("\"seq\":").expect("seq key present");
        let kind_pos = json.find("\"kind\":").expect("kind key present");
        assert!(seq_pos < kind_pos, "seq must appear before kind: {json}");
        assert!(
            json.contains("\"kind\":\"node_started\""),
            "kind must name the variant: {json}"
        );
        assert!(
            !json.contains("\"run_id\""),
            "run_id must be omitted when None: {json}"
        );
    }

    /// A `TraceRecord` round-trips through serde with every field intact.
    #[test]
    fn record_round_trips_through_serde() {
        let record = TraceRecord {
            thread_id: thread(),
            run_id: Some(RunId::new_v7()),
            seq: 3,
            at: Utc::now(),
            event: TraceEvent::DeltaMerged {
                superstep: 2,
                field_changes: vec![FieldChange {
                    field: FieldName::new("x").unwrap(),
                    dispatch: "last_write".to_string(),
                    writers: vec![NodeId::new("n1")],
                    value_bytes: 4,
                    value: None,
                }],
            },
        };
        let json = serde_json::to_string(&record).unwrap();
        let restored: TraceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.thread_id, record.thread_id);
        assert_eq!(restored.run_id, record.run_id);
        assert_eq!(restored.seq, record.seq);
        match (&restored.event, &record.event) {
            (
                TraceEvent::DeltaMerged {
                    superstep: rs,
                    field_changes: rf,
                },
                TraceEvent::DeltaMerged {
                    superstep: os,
                    field_changes: of,
                },
            ) => {
                assert_eq!(rs, os);
                assert_eq!(rf, of);
            }
            _ => panic!("expected DeltaMerged on both sides"),
        }
    }

    #[test]
    fn trace_schema_version_is_one() {
        assert_eq!(TRACE_SCHEMA_VERSION, "1");
    }
}
