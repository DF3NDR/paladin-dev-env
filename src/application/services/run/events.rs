//! `RunEventBus` -- the per-run broadcast bus `GET /v1/runs/{run_id}/stream`
//! reads from (D-24, PLAT-FR-07).
//!
//! Lives in the facade, never in `paladin-web`, mirroring `worker.rs`'s own
//! reason: driving anything the engine touches needs `paladin-battalion`
//! (ADR-0031). `paladin-web` sees only [`RunEventStreamService`] through the
//! `RunEventStreamPort` trait object (D-27) -- it never names this bus, a
//! `TraceEvent`, or `WarEngine`.
//!
//! # Two producers, not one (D-24, D-25 correction)
//!
//! Today's `TraceEvent` (`paladin-ports::output::trace_sink_port`) has eight
//! variants with no `parley` case, no `error` case, and a `RunFinished` that
//! does not distinguish success from failure. So this bus has two producers:
//!
//! - [`RunEventBusSink`] (a [`TraceSink`] implementation) bridges
//!   `superstep`/`node_started`/`node_finished`/`state_delta` live from the
//!   engine, through [`map_trace_event`].
//! - [`RunWorkerPool`](super::worker::RunWorkerPool) publishes
//!   `parley`/`done`/`error` directly from the `RunOutcome` it already
//!   matches on in `run_once` -- see that module's own docs.
//!
//! Phase 28 can later collapse this into `map_trace_event` alone once the
//! authoritative trace enum carries those three cases.
//!
//! # Never blocks the engine (D-24, T-27-10-02)
//!
//! `RunEventBus::publish` never awaits a slow consumer:
//! `tokio::sync::broadcast::Sender::send` is synchronous and returns
//! immediately regardless of how many subscribers are attached or how full
//! any one of their individual queues is. A subscriber that falls behind the
//! bounded channel capacity ([`RUN_EVENT_CHANNEL_CAPACITY`]) has its oldest
//! unread events dropped and counted by `tokio::sync::broadcast` itself; this
//! module folds that count into the `dropped` field of the next event the
//! lagging subscriber DOES receive (see [`RunEventStreamPort::stream`]'s live
//! path, `receiver_to_stream`).
//!
//! # Degraded mode (D-26)
//!
//! When a run is not bound on THIS instance (executing elsewhere, or already
//! terminal), [`RunEventStreamService::stream`] falls back to polling
//! `RunRepositoryPort::get` + `WaypointPort::latest` at `poll_interval`,
//! synthesizing `superstep`/`parley`/`done`/`error` events from what it
//! observes. This path gives NO ordering guarantee relative to the live path
//! and may coalesce several supersteps into one event, but always ends with
//! `done` or `error`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{RwLock, broadcast};

use paladin_core::platform::container::run::{
    Run, RunId, RunStatus, RunStreamEvent, RunStreamEventKind, RunStreamMode,
};
use paladin_core::platform::container::waypoint::{ThreadId, WaypointStatus};
use paladin_ports::input::run_event_stream_port::{
    RunEventStream, RunEventStreamPort, RunStreamError,
};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceRecord, TraceSink, TraceSinkError};
use paladin_ports::output::waypoint_port::WaypointPort;

/// Bounded per-run broadcast capacity (D-24, T-27-10-02): a subscriber that
/// falls this far behind has its oldest unread events dropped and reported
/// on the next it does receive, rather than ever slowing down `publish`.
pub const RUN_EVENT_CHANNEL_CAPACITY: usize = 64;

/// Map ONE [`TraceRecord`] onto its [`RunStreamEventKind`] + payload and the
/// [`ThreadId`] it belongs to (read off the envelope, D-02 -- no longer off
/// the event itself), or `None` for the eight variants this bus does not yet
/// bridge live: `RunStarted`, `NodeProgress`, `EdgeEvaluated`,
/// `WaypointSaved`, `ParleyRaised`, `RunFinished` (the worker publishes
/// `parley`/`done`/`error` directly from `RunOutcome` instead -- see
/// `worker.rs`), `FallbackHop` and `MiddlewareEvent`. D-14's completion to
/// seven-of-twelve mapped kinds (collapsing the worker's own direct
/// publishes into this one function) is 28-11's; this plan keeps today's
/// four mapped kinds unchanged, only re-based on the twelve-variant envelope.
///
/// `state_delta`'s `bytes` field is the summed UTF-8 length of the changed
/// field NAMES only -- `TraceEvent::DeltaMerged` itself carries no value to
/// leak by default, so this satisfies the "no values on the wire" prohibition
/// (T-27-10-01) by construction rather than by redacting anything.
///
/// `node_finished`'s `outcome` field is reported `"unknown"`: this bus does
/// not yet surface `TraceEvent::NodeFinished`'s real `outcome` field on the
/// wire (a later plan's job); guessing one from it here would be premature
/// scope for this plan.
pub fn map_trace_event(
    record: TraceRecord,
) -> Option<(ThreadId, RunStreamEventKind, serde_json::Value)> {
    let thread_id = record.thread_id.clone();
    match record.event {
        TraceEvent::SuperstepStarted { superstep, .. } => Some((
            thread_id,
            RunStreamEventKind::Superstep,
            serde_json::json!({ "superstep": superstep }),
        )),
        TraceEvent::NodeStarted {
            superstep, node_id, ..
        } => Some((
            thread_id,
            RunStreamEventKind::NodeStarted,
            serde_json::json!({ "superstep": superstep, "node_id": node_id.as_str() }),
        )),
        TraceEvent::NodeFinished {
            superstep, node_id, ..
        } => Some((
            thread_id,
            RunStreamEventKind::NodeFinished,
            serde_json::json!({
                "superstep": superstep,
                "node_id": node_id.as_str(),
                "outcome": "unknown",
            }),
        )),
        TraceEvent::DeltaMerged {
            superstep,
            field_changes,
        } => {
            let fields: Vec<&str> = field_changes.iter().map(|f| f.field.as_str()).collect();
            let bytes: u64 = fields.iter().map(|f| f.len() as u64).sum();
            Some((
                thread_id,
                RunStreamEventKind::StateDelta,
                serde_json::json!({ "superstep": superstep, "fields": fields, "bytes": bytes }),
            ))
        }
        TraceEvent::RunStarted { .. }
        | TraceEvent::NodeProgress { .. }
        | TraceEvent::EdgeEvaluated { .. }
        | TraceEvent::WaypointSaved { .. }
        | TraceEvent::ParleyRaised { .. }
        | TraceEvent::RunFinished { .. }
        | TraceEvent::FallbackHop { .. }
        | TraceEvent::MiddlewareEvent { .. } => None,
        // `TraceEvent` is `#[non_exhaustive]` (a future variant this bus
        // does not yet understand is dropped, never a compile break or a
        // panic).
        _ => None,
    }
}

#[derive(Default)]
struct RunEventBusState {
    channels: HashMap<RunId, broadcast::Sender<RunStreamEvent>>,
    bound: HashMap<ThreadId, RunId>,
    seqs: HashMap<RunId, u64>,
}

/// The per-run broadcast bus (D-24).
///
/// A worker [`Self::bind`]s a thread to a run before dispatching it, the two
/// producers described in the module docs [`Self::publish`] onto it, and the
/// SSE handler (through [`RunEventStreamService`]) [`Self::subscribe`]s.
/// [`Self::unbind`] removes the run's channel entirely; a subscriber that has
/// not yet drained a just-published terminal event still receives it -- a
/// `tokio::sync::broadcast` channel's already-queued messages remain
/// deliverable to a receiver that has not yet read them, even after every
/// `Sender` clone (including this bus's own) is dropped. The receiver's NEXT
/// call after draining the queue then observes the stream end
/// (`RecvError::Closed`), never a hang.
pub struct RunEventBus {
    state: RwLock<RunEventBusState>,
}

impl RunEventBus {
    /// Construct an empty bus.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(RunEventBusState::default()),
        }
    }

    /// Bind `thread_id` to `run_id`, creating the run's channel if this is
    /// its first bind.
    pub async fn bind(&self, thread_id: ThreadId, run_id: RunId) {
        let mut state = self.state.write().await;
        state.bound.insert(thread_id, run_id.clone());
        state
            .channels
            .entry(run_id)
            .or_insert_with(|| broadcast::channel(RUN_EVENT_CHANNEL_CAPACITY).0);
    }

    /// Unbind `thread_id`, dropping the run's channel. See the struct docs
    /// for why an already-published terminal event still reaches a live
    /// subscriber before the stream ends.
    pub async fn unbind(&self, thread_id: &ThreadId) {
        let mut state = self.state.write().await;
        if let Some(run_id) = state.bound.remove(thread_id) {
            state.channels.remove(&run_id);
            state.seqs.remove(&run_id);
        }
    }

    /// The run currently bound to `thread_id`, if any.
    pub async fn run_id_for(&self, thread_id: &ThreadId) -> Option<RunId> {
        self.state.read().await.bound.get(thread_id).cloned()
    }

    /// Subscribe to `run_id`'s live events. `None` if this instance is not
    /// currently dispatching that run (never bound here, or already
    /// unbound) -- the caller's cue to fall back to the degraded polling
    /// path (D-26).
    pub async fn subscribe(&self, run_id: &RunId) -> Option<broadcast::Receiver<RunStreamEvent>> {
        self.state
            .read()
            .await
            .channels
            .get(run_id)
            .map(|tx| tx.subscribe())
    }

    /// Publish one event for `run_id`/`thread_id`, stamping the next
    /// per-run sequence number. A no-op if the run is not currently bound
    /// (a [`TraceSink`] callback racing an [`Self::unbind`], or an unbound
    /// thread) -- always `Ok`, never an error, per `TraceSink`'s own
    /// fire-and-forget contract. Never awaits a receiver -- see the module
    /// docs' "Never blocks the engine" section.
    pub async fn publish(
        &self,
        run_id: &RunId,
        thread_id: &ThreadId,
        kind: RunStreamEventKind,
        mode: RunStreamMode,
        payload: serde_json::Value,
    ) {
        let mut state = self.state.write().await;
        let Some(tx) = state.channels.get(run_id).cloned() else {
            return;
        };
        let seq_slot = state.seqs.entry(run_id.clone()).or_insert(0);
        *seq_slot += 1;
        let seq = *seq_slot;
        let event = RunStreamEvent::new(
            run_id.clone(),
            thread_id.clone(),
            kind,
            seq,
            mode,
            0,
            payload,
        );
        // `send`'s `Err` means nobody is currently subscribed -- a normal,
        // non-error case (no client has connected yet, or every client has
        // disconnected) -- intentionally discarded.
        let _ = tx.send(event);
    }
}

impl Default for RunEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// The [`TraceSink`] half of the bus's two producers (D-24, D-25
/// correction): bridges `superstep`/`node_started`/`node_finished`/
/// `state_delta` live from the engine through [`map_trace_event`].
/// `parley`/`done`/`error` are published directly by
/// [`RunWorkerPool`](super::worker::RunWorkerPool) from the `RunOutcome` it
/// already matches on.
pub struct RunEventBusSink {
    bus: Arc<RunEventBus>,
}

impl RunEventBusSink {
    /// Wrap `bus` as a `TraceSink`.
    pub fn new(bus: Arc<RunEventBus>) -> Self {
        Self { bus }
    }
}

#[async_trait]
impl TraceSink for RunEventBusSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        let Some((thread_id, kind, payload)) = map_trace_event(record) else {
            return Ok(());
        };
        let Some(run_id) = self.bus.run_id_for(&thread_id).await else {
            // No run is dispatching this thread on THIS instance right now
            // -- a no-op `Ok`, never an error (the `TraceSink` fire-and-
            // forget contract this module's docs describe).
            return Ok(());
        };
        self.bus
            .publish(&run_id, &thread_id, kind, RunStreamMode::Live, payload)
            .await;
        Ok(())
    }
}

/// Convert a subscribed `broadcast::Receiver` into the [`RunEventStream`]
/// the port returns, folding a lagged receiver's skipped-event count into
/// the `dropped` field of the next event it DOES receive (D-24).
fn receiver_to_stream(rx: broadcast::Receiver<RunStreamEvent>) -> RunEventStream {
    Box::pin(futures::stream::unfold(
        (rx, 0u64),
        |(mut rx, mut dropped)| async move {
            loop {
                match rx.recv().await {
                    Ok(mut event) => {
                        event.dropped = dropped;
                        return Some((event, (rx, 0u64)));
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        dropped = dropped.saturating_add(skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return None;
                    }
                }
            }
        },
    ))
}

/// Facade-internal state the degraded polling loop threads through
/// `futures::stream::unfold` (D-26).
struct DegradedState {
    run_id: RunId,
    thread_id: ThreadId,
    run_repo: Arc<dyn RunRepositoryPort>,
    waypoints: Arc<dyn WaypointPort>,
    poll_interval: Duration,
    last_superstep: Option<u64>,
    parley_emitted: bool,
    seq: u64,
    finished: bool,
}

impl DegradedState {
    fn next_event(
        &mut self,
        kind: RunStreamEventKind,
        payload: serde_json::Value,
    ) -> RunStreamEvent {
        self.seq += 1;
        RunStreamEvent::new(
            self.run_id.clone(),
            self.thread_id.clone(),
            kind,
            self.seq,
            RunStreamMode::Degraded,
            0,
            payload,
        )
    }
}

/// The terminal `done`/`error` payload for a `Run` whose status is already
/// terminal (D-26). The `_` arm is unreachable given every caller only
/// invokes this after checking `run.status.is_terminal()`, but returns a
/// safe generic error rather than panicking (WR-01, Phase 22.1).
fn terminal_payload(run: &Run) -> (RunStreamEventKind, serde_json::Value) {
    match run.status {
        RunStatus::Completed => (
            RunStreamEventKind::Done,
            serde_json::json!({ "status": "completed", "waypoint_id": run.final_waypoint_id }),
        ),
        RunStatus::Cancelled => (
            RunStreamEventKind::Done,
            serde_json::json!({ "status": "cancelled", "waypoint_id": run.final_waypoint_id }),
        ),
        RunStatus::Halted => (
            RunStreamEventKind::Done,
            serde_json::json!({ "status": "halted", "waypoint_id": run.final_waypoint_id }),
        ),
        RunStatus::Failed => (
            RunStreamEventKind::Error,
            serde_json::json!({
                "status": "failed",
                "message": run.error,
                "waypoint_id": run.final_waypoint_id,
            }),
        ),
        RunStatus::Queued | RunStatus::Running | RunStatus::AwaitingInput => (
            RunStreamEventKind::Error,
            serde_json::json!({
                "status": "failed",
                "message": "degraded stream asked for a terminal payload on a non-terminal run",
            }),
        ),
    }
}

/// Build the degraded polling stream for a run this instance is not
/// currently dispatching (D-26): polls `RunRepositoryPort::get` +
/// `WaypointPort::latest` every `poll_interval`, emitting at most one
/// `superstep` event per newly observed Waypoint (coalescing any missed
/// between polls), one `parley` event on the run's first observed
/// `AwaitingInput`, and always ending with `done`/`error` once the run
/// reaches a terminal status.
fn degraded_stream(
    run_id: RunId,
    thread_id: ThreadId,
    run_repo: Arc<dyn RunRepositoryPort>,
    waypoints: Arc<dyn WaypointPort>,
    poll_interval: Duration,
) -> RunEventStream {
    let state = DegradedState {
        run_id,
        thread_id,
        run_repo,
        waypoints,
        poll_interval,
        last_superstep: None,
        parley_emitted: false,
        seq: 0,
        finished: false,
    };
    Box::pin(futures::stream::unfold(state, |mut state| async move {
        if state.finished {
            return None;
        }
        loop {
            tokio::time::sleep(state.poll_interval).await;

            let run = match state.run_repo.get(&state.run_id).await {
                Ok(Some(run)) => run,
                Ok(None) => {
                    state.finished = true;
                    let event = state.next_event(
                        RunStreamEventKind::Error,
                        serde_json::json!({
                            "status": "failed",
                            "message": "run vanished from the repository",
                        }),
                    );
                    return Some((event, state));
                }
                Err(error) => {
                    state.finished = true;
                    let event = state.next_event(
                        RunStreamEventKind::Error,
                        serde_json::json!({ "status": "failed", "message": error.to_string() }),
                    );
                    return Some((event, state));
                }
            };

            let latest = state
                .waypoints
                .latest(&state.thread_id)
                .await
                .ok()
                .flatten();

            if let Some(wp) = &latest {
                let is_new = state
                    .last_superstep
                    .map(|seen| wp.superstep > seen)
                    .unwrap_or(true);
                if is_new {
                    state.last_superstep = Some(wp.superstep);
                    let event = state.next_event(
                        RunStreamEventKind::Superstep,
                        serde_json::json!({ "superstep": wp.superstep }),
                    );
                    return Some((event, state));
                }
            }

            if run.status == RunStatus::AwaitingInput && !state.parley_emitted {
                state.parley_emitted = true;
                let (waypoint_id, parleys) = match &latest {
                    Some(wp) => match &wp.status {
                        WaypointStatus::AwaitingInput { parleys, .. } => {
                            (Some(wp.waypoint_id.to_string()), parleys.clone())
                        }
                        _ => (Some(wp.waypoint_id.to_string()), Vec::new()),
                    },
                    None => (None, Vec::new()),
                };
                let event = state.next_event(
                    RunStreamEventKind::Parley,
                    serde_json::json!({ "waypoint_id": waypoint_id, "parleys": parleys }),
                );
                return Some((event, state));
            }

            if run.status.is_terminal() {
                state.finished = true;
                let (kind, payload) = terminal_payload(&run);
                let event = state.next_event(kind, payload);
                return Some((event, state));
            }
        }
    }))
}

/// The facade [`RunEventStreamPort`] implementation (D-27): decides live vs
/// degraded, but names neither the engine nor `TraceEvent` in its own
/// interface -- `paladin-web` sees only [`Self::stream`]'s
/// [`RunEventStream`] return type.
pub struct RunEventStreamService {
    bus: Arc<RunEventBus>,
    run_repo: Arc<dyn RunRepositoryPort>,
    waypoints: Arc<dyn WaypointPort>,
    poll_interval: Duration,
}

impl RunEventStreamService {
    /// Construct a service reading `run_repo`/`waypoints` for the degraded
    /// path and `bus` for the live path, polling at `poll_interval` when
    /// degraded (D-26 recommends `1s` in production).
    pub fn new(
        bus: Arc<RunEventBus>,
        run_repo: Arc<dyn RunRepositoryPort>,
        waypoints: Arc<dyn WaypointPort>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            bus,
            run_repo,
            waypoints,
            poll_interval,
        }
    }
}

#[async_trait]
impl RunEventStreamPort for RunEventStreamService {
    async fn stream(&self, run_id: &RunId) -> Result<RunEventStream, RunStreamError> {
        let run = self
            .run_repo
            .get(run_id)
            .await
            .map_err(|error| RunStreamError::Backend {
                message: error.to_string(),
            })?
            .ok_or_else(|| RunStreamError::NotFound {
                run_id: run_id.clone(),
            })?;

        if let Some(rx) = self.bus.subscribe(run_id).await {
            return Ok(receiver_to_stream(rx));
        }

        Ok(degraded_stream(
            run_id.clone(),
            run.thread_id.clone(),
            self.run_repo.clone(),
            self.waypoints.clone(),
            self.poll_interval,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    use paladin_core::platform::container::battlefield::FieldName;
    use paladin_core::platform::container::parley::{ParleyId, ParleyKind};
    use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, WaypointId};
    use paladin_ports::output::trace_sink_port::{FieldChange, MiddlewareAction, RunFinishStatus};

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

    /// The mapping table test the plan's own behavior text calls for: all
    /// twelve `TraceEvent` variants are enumerated, exactly four map to
    /// `Some` (unchanged from the eight-variant enum -- this plan keeps
    /// today's four mapped kinds, D-14's seven-of-twelve completion is
    /// 28-11's), and eight map to `None`.
    #[test]
    fn map_trace_event_maps_exactly_four_of_twelve_variants() {
        let thread_id = ThreadId::new("t1").unwrap();
        let cases: Vec<(TraceEvent, bool)> = vec![
            (
                TraceEvent::RunStarted {
                    run_id: None,
                    graph_fingerprint: "fp".to_string(),
                },
                false,
            ),
            (
                TraceEvent::SuperstepStarted {
                    superstep: 1,
                    vanguard: vec![NodeId::new("n1")],
                },
                true,
            ),
            (
                TraceEvent::NodeStarted {
                    superstep: 1,
                    node_id: NodeId::new("n1"),
                    attempt: 1,
                    muster_task_key: None,
                },
                true,
            ),
            (
                TraceEvent::NodeProgress {
                    node_id: NodeId::new("n1"),
                    progress: paladin_ports::output::trace_sink_port::NodeProgressKind::Heartbeat,
                },
                false,
            ),
            (
                TraceEvent::NodeFinished {
                    superstep: 1,
                    node_id: NodeId::new("n1"),
                    attempt: 1,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms: 5,
                    token_count: 0,
                    cache_hit: false,
                },
                true,
            ),
            (
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("a"),
                    to: NodeId::new("b"),
                    condition_kind: "always".to_string(),
                    fired: true,
                },
                false,
            ),
            (
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
                true,
            ),
            (
                TraceEvent::WaypointSaved {
                    waypoint_id: WaypointId::generate(),
                    superstep: 1,
                    status: "completed".to_string(),
                },
                false,
            ),
            (
                TraceEvent::ParleyRaised {
                    parley_id: ParleyId::new(),
                    node_id: NodeId::new("n1"),
                    parley_kind: ParleyKind::Approval,
                },
                false,
            ),
            (
                TraceEvent::RunFinished {
                    status: RunFinishStatus::Completed,
                    total_supersteps: 1,
                    total_tokens: 0,
                    duration_ms: 5,
                    trace_dropped_total: 0,
                },
                false,
            ),
            (
                TraceEvent::FallbackHop {
                    node_id: None,
                    from_provider: "openai".to_string(),
                    to_provider: "anthropic".to_string(),
                },
                false,
            ),
            (
                TraceEvent::MiddlewareEvent {
                    name: "limit".to_string(),
                    action: MiddlewareAction::Finish,
                },
                false,
            ),
        ];
        assert_eq!(cases.len(), 12, "must enumerate all twelve variants");

        let mut mapped = 0;
        let mut dropped = 0;
        for (seq, (event, expect_some)) in cases.into_iter().enumerate() {
            let record = wrap(thread_id.clone(), seq as u64 + 1, event);
            match map_trace_event(record) {
                Some(_) if expect_some => mapped += 1,
                None if !expect_some => dropped += 1,
                other => panic!("unexpected mapping result: {other:?}"),
            }
        }
        assert_eq!(mapped, 4, "exactly four variants must map to Some");
        assert_eq!(dropped, 8, "exactly eight variants must map to None");
    }

    #[tokio::test]
    async fn bus_publish_is_delivered_to_a_subscriber_after_bind() {
        let bus = RunEventBus::new();
        let run_id = RunId::new_v7();
        let thread_id = sample_thread();
        bus.bind(thread_id.clone(), run_id.clone()).await;

        let mut rx = bus
            .subscribe(&run_id)
            .await
            .expect("bound run must have a channel");
        bus.publish(
            &run_id,
            &thread_id,
            RunStreamEventKind::Superstep,
            RunStreamMode::Live,
            serde_json::json!({ "superstep": 1 }),
        )
        .await;

        let event = rx.recv().await.unwrap();
        assert_eq!(event.run_id, run_id);
        assert_eq!(event.kind, RunStreamEventKind::Superstep);
        assert_eq!(event.mode, RunStreamMode::Live);
        assert_eq!(event.seq, 1);
        assert_eq!(event.dropped, 0);
    }

    #[tokio::test]
    async fn bus_lagged_subscriber_reports_a_nonzero_skip_count() {
        let bus = RunEventBus::new();
        let run_id = RunId::new_v7();
        let thread_id = sample_thread();
        bus.bind(thread_id.clone(), run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.unwrap();

        // Publish well past the channel's capacity without ever reading --
        // the receiver falls behind and its next `recv` reports the skip.
        for i in 0..(RUN_EVENT_CHANNEL_CAPACITY as u64 + 6) {
            bus.publish(
                &run_id,
                &thread_id,
                RunStreamEventKind::Superstep,
                RunStreamMode::Live,
                serde_json::json!({ "superstep": i }),
            )
            .await;
        }

        match rx.recv().await {
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                assert_eq!(
                    skipped, 6,
                    "exactly 6 of the 70 published events must be skipped"
                );
            }
            other => panic!("expected Lagged, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn receiver_to_stream_folds_lag_into_the_next_events_dropped_field() {
        let bus = RunEventBus::new();
        let run_id = RunId::new_v7();
        let thread_id = sample_thread();
        bus.bind(thread_id.clone(), run_id.clone()).await;
        let rx = bus.subscribe(&run_id).await.unwrap();
        let mut stream = receiver_to_stream(rx);

        for i in 0..(RUN_EVENT_CHANNEL_CAPACITY as u64 + 6) {
            bus.publish(
                &run_id,
                &thread_id,
                RunStreamEventKind::Superstep,
                RunStreamMode::Live,
                serde_json::json!({ "superstep": i }),
            )
            .await;
        }

        let first = stream.next().await.expect("stream must yield after a lag");
        assert_eq!(
            first.dropped, 6,
            "the fold must report exactly the 6 skipped events"
        );
        assert_eq!(
            first.seq, 7,
            "the first delivered event is the 7th published"
        );

        let second = stream.next().await.expect("stream must keep yielding");
        assert_eq!(second.dropped, 0, "dropped resets once caught up");
    }

    #[tokio::test]
    async fn bus_unbind_removes_the_channel_and_ends_the_stream() {
        let bus = RunEventBus::new();
        let run_id = RunId::new_v7();
        let thread_id = sample_thread();
        bus.bind(thread_id.clone(), run_id.clone()).await;
        let mut rx = bus.subscribe(&run_id).await.unwrap();

        bus.publish(
            &run_id,
            &thread_id,
            RunStreamEventKind::Done,
            RunStreamMode::Live,
            serde_json::json!({ "status": "completed" }),
        )
        .await;
        bus.unbind(&thread_id).await;

        // The already-queued `done` event is still delivered...
        let event = rx.recv().await.unwrap();
        assert_eq!(event.kind, RunStreamEventKind::Done);
        // ...and the stream then observes the channel is closed.
        assert!(matches!(
            rx.recv().await,
            Err(broadcast::error::RecvError::Closed)
        ));
        assert!(bus.subscribe(&run_id).await.is_none());
    }

    #[tokio::test]
    async fn sink_on_event_is_a_no_op_for_an_unbound_thread() {
        let bus = Arc::new(RunEventBus::new());
        let sink = RunEventBusSink::new(bus);
        let thread_id = sample_thread();
        let result = sink
            .on_event(wrap(
                thread_id,
                1,
                TraceEvent::SuperstepStarted {
                    superstep: 1,
                    vanguard: Vec::new(),
                },
            ))
            .await;
        assert!(result.is_ok());
    }
}
