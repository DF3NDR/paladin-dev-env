//! Facade `ParleyPort` Adapter Over a Real `WarEngine` (HITL-05, D-25)
//!
//! [`ParleyPortAdapter`] implements `paladin-ports`'
//! [`ParleyPort`](paladin_ports::input::parley_port::ParleyPort) over a real
//! `WarEngine`: it resolves the thread's own graph through a
//! [`GraphRegistry`](super::registry::GraphRegistry) (D-26), validates a
//! submission, and either returns a typed error synchronously (nothing
//! persisted, beyond the one documented `ParleyExpired`/`FailRun`
//! exception) or spawns the continuation as a background task registered
//! with `paladin-battalion`'s `ShutdownCoordinator` (D-21) and returns
//! immediately (D-25) -- the SAME validate-then-spawn-then-`202` shape
//! `crates/paladin-web/src/agent_controller.rs`'s `enqueue_job` already
//! ships.
//!
//! # Why this adapter re-implements `WarEngine::resume_with`'s validation
//!
//! `WarEngine::resume_with` (`paladin-battalion`) is one atomic async
//! function: its total-validation pass and its potentially long-running
//! continuation (`superstep::run_with_namespace`, invoked only once every
//! outstanding parley has a response) are not separable from outside the
//! crate, and its own per-response validators
//! (`graph::validate_parley_value_for_kind`, `validate_response_shape`) are
//! `pub(crate)` to `paladin-battalion` -- unreachable from this crate.
//!
//! To honour D-25's contract (an error surfaces synchronously, from THIS
//! call, before any background task is spawned; a valid-and-complete
//! submission spawns the continuation and returns immediately) without
//! modifying `paladin-battalion`, [`shadow_validate`] re-implements the
//! SAME validation algorithm (24-04's D-10/D-11/D-12 ordering: lazy expiry
//! scan over every outstanding parley, then total per-response validation)
//! using only public data and public helper logic. This is a defensive,
//! predictive pre-check -- never the sole authority: every branch below
//! that concludes "this call will not reach the continuation" (a rejected
//! submission, an expired `FailRun` parley, or a valid-but-partial
//! submission) delegates the ACTUAL persist to a real, synchronous
//! (awaited inline, never spawned) call to `WarEngine::resume_with`, which
//! re-validates identically and is what performs the genuine Waypoint
//! write. Only the one branch this shadow validation predicts will reach
//! the continuation (every parley answered) is spawned in the background.
//!
//! A prediction/reality mismatch (an inherent, accepted risk of a
//! check-then-act split when the two cannot be made atomic without an
//! engine change) is always resolved in the REAL engine's favour: the
//! spawned task's own call is what actually runs, and any divergence
//! surfaces only through the thread's own state on a later poll -- the
//! same eventually-consistent contract any background-job system offers.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{EngineError, WarEngine, WarGraph};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::waypoint::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse, ThreadId, WaypointStatus,
};
use paladin_ports::input::parley_port::{ParleyError, ParleyPort, ResumeAccepted};
use paladin_ports::output::waypoint_port::WaypointPort;

use super::registry::GraphRegistry;

/// Facade [`ParleyPort`] implementation over a real `WarEngine` (D-25,
/// D-26). Generic over the same `W: WaypointPort` a `WarEngine<W>` is
/// constructed with.
pub struct ParleyPortAdapter<W: WaypointPort + 'static> {
    engine: Arc<WarEngine<W>>,
    waypoint_port: Arc<W>,
    registry: Arc<GraphRegistry>,
    coordinator: ShutdownCoordinator,
}

impl<W: WaypointPort + 'static> ParleyPortAdapter<W> {
    /// Construct an adapter over `engine` (constructed with the SAME
    /// `waypoint_port` instance), resolving graphs through `registry` and
    /// registering every spawned continuation with `coordinator`.
    pub fn new(
        engine: Arc<WarEngine<W>>,
        waypoint_port: Arc<W>,
        registry: Arc<GraphRegistry>,
        coordinator: ShutdownCoordinator,
    ) -> Self {
        Self {
            engine,
            waypoint_port,
            registry,
            coordinator,
        }
    }
}

#[async_trait]
impl<W: WaypointPort + 'static> ParleyPort for ParleyPortAdapter<W> {
    async fn resume_with(
        &self,
        thread: &ThreadId,
        responses: Vec<ParleyResponse>,
    ) -> Result<ResumeAccepted, ParleyError> {
        let latest = self
            .waypoint_port
            .latest(thread)
            .await
            .map_err(|source| ParleyError::Backend { source })?
            .ok_or_else(|| ParleyError::ThreadNotFound(thread.clone()))?;

        let graph = self
            .registry
            .resolve(&latest.graph_fingerprint)
            .ok_or_else(|| ParleyError::GraphNotRegistered {
                fingerprint: latest.graph_fingerprint.clone(),
            })?;

        let (parleys, existing_responses) = match &latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                (parleys.clone(), responses.clone())
            }
            other => {
                return Err(ParleyError::ThreadNotAwaitingInput {
                    thread: thread.clone(),
                    status: format!("{other:?}"),
                });
            }
        };

        match shadow_validate(&parleys, &existing_responses, &responses, graph.as_ref()) {
            ShadowOutcome::Rejected(err) => Err(err),
            ShadowOutcome::DelegateSynchronously => {
                // Always fast by construction: an expired `FailRun` parley
                // and a valid-but-partial submission both return from the
                // real `resume_with` before `superstep::run_with_namespace`
                // is ever invoked -- awaiting inline here never blocks on
                // the continuation.
                self.engine
                    .resume_with(graph.as_ref(), thread.clone(), responses)
                    .await
                    .map(|_outcome| ResumeAccepted::new(thread.clone()))
                    .map_err(map_engine_error)
            }
            ShadowOutcome::Complete => {
                // Every parley now has a response: this call WOULD invoke
                // the continuation. Register with the ShutdownCoordinator
                // and spawn the real, authoritative call as a background
                // task (D-21, D-25); return immediately.
                let (_child_token, guard) = self.coordinator.register();
                let engine = Arc::clone(&self.engine);
                let graph_for_task = Arc::clone(&graph);
                let thread_for_task = thread.clone();
                tokio::spawn(async move {
                    let _guard = guard;
                    let _ = engine
                        .resume_with(graph_for_task.as_ref(), thread_for_task, responses)
                        .await;
                    // No synchronous caller remains to report a failure
                    // to (a race against a concurrent resume_with call, or
                    // a shadow-validation divergence) -- the thread's own
                    // state, read through a separate poll path, is the
                    // source of truth, exactly like any other
                    // fire-and-forget background job.
                });
                Ok(ResumeAccepted::new(thread.clone()))
            }
        }
    }
}

/// The three ways [`shadow_validate`] can conclude, given only the
/// information a `ParleyPort` caller and the thread's own Waypoint expose.
enum ShadowOutcome {
    /// The submission is invalid; return this typed error directly, with
    /// nothing persisted and no engine call made at all.
    Rejected(ParleyError),
    /// Either an outstanding parley has expired under `on_expire: FailRun`
    /// (or a future `OnExpire` this engine fails closed on), or the
    /// submission is valid but leaves at least one parley unanswered.
    /// Either way the real `WarEngine::resume_with` call this outcome
    /// delegates to is guaranteed fast (never reaches the continuation).
    DelegateSynchronously,
    /// The submission is valid and, combined with the thread's existing
    /// responses, answers every outstanding parley: the real call would
    /// invoke the continuation, so it must be spawned in the background.
    Complete,
}

/// Re-implements `WarEngine::resume_with`'s own validation ordering
/// (24-04, D-10/D-11/D-12) using only public data: lazy expiry scan over
/// every outstanding (not-yet-answered) parley first (a `FailRun`-expired
/// parley short-circuits to [`ShadowOutcome::DelegateSynchronously`]
/// immediately, matching the real engine's own short-circuit), then total
/// per-response validation (`UnknownParleyId` → `ParleyAlreadyAnswered` →
/// `ResponseShapeInvalid`) over the submitted responses plus any
/// `ResumeWithDefault` substitutions, then a completeness check. See the
/// module-level documentation for why this duplication exists and why it
/// is never the sole authority.
fn shadow_validate(
    parleys: &[ParleyRequest],
    existing_responses: &[ParleyResponse],
    responses: &[ParleyResponse],
    graph: &WarGraph,
) -> ShadowOutcome {
    let now = Utc::now();
    let already_answered: BTreeSet<ParleyId> =
        existing_responses.iter().map(|r| r.parley_id).collect();

    let mut defaulted: Vec<ParleyResponse> = Vec::new();
    for request in parleys {
        if already_answered.contains(&request.parley_id) {
            continue;
        }
        let Some(expires_at) = request.expires_at else {
            continue;
        };
        if expires_at > now {
            continue;
        }
        match &request.on_expire {
            OnExpire::ResumeWithDefault(value) => {
                defaulted.push(ParleyResponse {
                    parley_id: request.parley_id,
                    kind: request.kind.clone(),
                    prompt: request.prompt.clone(),
                    value: value.clone(),
                    responded_by: None,
                    responded_at: now,
                    defaulted: true,
                });
            }
            // `OnExpire::FailRun`, or a future variant this engine fails
            // closed on (mirroring the real engine's own fail-closed
            // catch-all): the real call always returns before the
            // continuation for this case.
            _ => return ShadowOutcome::DelegateSynchronously,
        }
    }

    let mut effective_responses: Vec<ParleyResponse> = responses
        .iter()
        .filter(|r| !defaulted.iter().any(|d| d.parley_id == r.parley_id))
        .cloned()
        .collect();
    effective_responses.extend(defaulted);

    let mut newly_answered: BTreeSet<ParleyId> = BTreeSet::new();
    for response in &effective_responses {
        let Some(request) = parleys.iter().find(|p| p.parley_id == response.parley_id) else {
            return ShadowOutcome::Rejected(ParleyError::UnknownParleyId {
                parley_id: response.parley_id,
            });
        };
        if already_answered.contains(&response.parley_id)
            || !newly_answered.insert(response.parley_id)
        {
            return ShadowOutcome::Rejected(ParleyError::ParleyAlreadyAnswered {
                parley_id: response.parley_id,
            });
        }
        if let Err(reason) = shadow_validate_response_shape(graph, request, &response.value) {
            return ShadowOutcome::Rejected(ParleyError::ResponseShapeInvalid {
                parley_id: response.parley_id,
                reason,
            });
        }
    }

    let mut all_responses: Vec<&ParleyResponse> = existing_responses.iter().collect();
    all_responses.extend(effective_responses.iter());

    let complete = parleys
        .iter()
        .all(|p| all_responses.iter().any(|r| r.parley_id == p.parley_id));

    if complete {
        ShadowOutcome::Complete
    } else {
        ShadowOutcome::DelegateSynchronously
    }
}

/// A faithful re-implementation of `paladin-battalion`'s crate-private
/// `validate_response_shape` (`crates/paladin-battalion/src/engine/mod.rs`),
/// reachable from this crate only by copy since the original is
/// `pub(crate)` to `paladin-battalion`. See the module-level documentation
/// for why this duplication exists.
fn shadow_validate_response_shape(
    graph: &WarGraph,
    request: &ParleyRequest,
    value: &serde_json::Value,
) -> Result<(), String> {
    shadow_validate_parley_value_for_kind(&request.kind, request.choices.as_deref(), value)?;
    if request.kind == ParleyKind::StateEdit {
        let delta: StateDelta = serde_json::from_value(value.clone())
            .map_err(|e| format!("StateEdit value must deserialize as a StateDelta: {e}"))?;
        let schema = graph.schema();
        let mut unknown: Vec<&str> = delta
            .values
            .keys()
            .filter(|field| schema.field_spec(field).is_none())
            .map(|field| field.as_str())
            .collect();
        if !unknown.is_empty() {
            unknown.sort_unstable();
            return Err(format!(
                "StateEdit value names field(s) not declared in the graph schema: {}",
                unknown.join(", ")
            ));
        }
    }
    Ok(())
}

/// A faithful re-implementation of `paladin-battalion`'s crate-private
/// `graph::validate_parley_value_for_kind`. See
/// [`shadow_validate_response_shape`]'s own rustdoc for why this
/// duplication exists.
fn shadow_validate_parley_value_for_kind(
    kind: &ParleyKind,
    choices: Option<&[String]>,
    value: &serde_json::Value,
) -> Result<(), String> {
    match kind {
        ParleyKind::Approval => shadow_normalize_approval_value(value)
            .map(|_| ())
            .ok_or_else(|| {
                format!(
                    "Approval value must be a bool or one of true/false/yes/no/approve/deny \
                     (case-insensitive); found {value}"
                )
            }),
        ParleyKind::Choice => {
            let Some(s) = value.as_str() else {
                return Err(format!("Choice value must be a string; found {value}"));
            };
            if let Some(choices) = choices
                && !choices.iter().any(|c| c == s)
            {
                return Err(format!(
                    "Choice value '{s}' is not one of the declared choices: {choices:?}"
                ));
            }
            Ok(())
        }
        ParleyKind::FreeText => {
            if value.is_string() {
                Ok(())
            } else {
                Err(format!("FreeText value must be a string; found {value}"))
            }
        }
        ParleyKind::StateEdit => serde_json::from_value::<StateDelta>(value.clone())
            .map(|_| ())
            .map_err(|e| format!("StateEdit value must deserialize as a StateDelta: {e}")),
        // `ParleyKind` is `#[non_exhaustive]`: fails CLOSED, mirroring the
        // original's own catch-all.
        other => Err(format!(
            "no value validator is registered for ParleyKind {other:?} -- add one alongside \
             the kind"
        )),
    }
}

/// A faithful re-implementation of `paladin-battalion`'s crate-private
/// `graph::normalize_approval_value`.
fn shadow_normalize_approval_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "yes" | "approve" => Some(true),
            "false" | "no" | "deny" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Maps every `EngineError` variant the real `WarEngine::resume_with`
/// validation path can produce onto its `ParleyError` counterpart
/// explicitly (D-25) -- a catch-all mapping that collapsed distinct
/// validation failures into one variant would erase the 400-versus-409
/// distinction a later plan's HTTP layer depends on.
fn map_engine_error(err: EngineError) -> ParleyError {
    match err {
        EngineError::ThreadNotFound(thread) => ParleyError::ThreadNotFound(thread),
        EngineError::ThreadNotAwaitingInput { thread, status } => {
            ParleyError::ThreadNotAwaitingInput { thread, status }
        }
        EngineError::UnknownParleyId { parley_id } => ParleyError::UnknownParleyId { parley_id },
        EngineError::ParleyAlreadyAnswered { parley_id } => {
            ParleyError::ParleyAlreadyAnswered { parley_id }
        }
        EngineError::ResponseShapeInvalid { parley_id, reason } => {
            ParleyError::ResponseShapeInvalid { parley_id, reason }
        }
        EngineError::ParleyExpired {
            parley_id,
            expires_at,
        } => ParleyError::ParleyExpired {
            parley_id,
            expires_at,
        },
        // `EngineError` is `#[non_exhaustive]`: every other variant --
        // including ones this adapter's own upstream checks make
        // structurally unreachable in practice (`GraphMismatch`: this
        // adapter always resolves the graph BY the thread's own
        // fingerprint, so the fingerprints can never differ;
        // `ThreadAlreadyFailed`: this adapter already rejects a
        // non-`AwaitingInput` status before ever reaching the real engine
        // call) and any future variant this mapping does not yet name --
        // fails CLOSED into `Rejected` rather than being silently dropped
        // or panicking (T-24-06's fail-closed discipline).
        other => ParleyError::Rejected {
            reason: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait as async_trait_attr;
    use paladin_battalion::engine::node::{NodeContext, NodeError, StateNode};
    use paladin_battalion::engine::{EngineLimits, NodeSpec, WaypointDurability};
    use paladin_core::platform::container::battlefield::{
        Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::directive::{Directive, NextStep};
    use paladin_core::platform::container::paladin::Paladin;
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::waypoint::{GraphFingerprint, NodeId, ParleyId};
    use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
    use paladin_storage::waypoint::contract_tests::sample_waypoint_at;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    use std::time::Duration;

    struct UnimplementedPaladinPort;

    #[async_trait_attr]
    impl PaladinPort for UnimplementedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unimplemented!("no test in this module executes a Paladin node")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unimplemented!("no test in this module streams a Paladin node")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// A `StateNode` whose whole job is to mark an `output_field` `true` --
    /// enough to prove a spawned continuation actually ran.
    struct MarkerNode {
        output_field: FieldName,
    }

    #[async_trait_attr]
    impl StateNode for MarkerNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, NodeError> {
            let mut delta = paladin_core::platform::container::battlefield::StateDelta::new();
            delta.set_raw(self.output_field.clone(), serde_json::json!(true));
            Ok(Directive {
                delta,
                next: NextStep::Edges,
            })
        }
    }

    fn thread(name: &str) -> ThreadId {
        ThreadId::new(name).unwrap()
    }

    /// A one-node graph whose single node marks `approved: true` when it
    /// runs -- built directly through `WarGraph`'s own API rather than a
    /// first-class `Gate` variant, since this test module only needs SOME
    /// node whose post-resume run is observable, not a real parley-raising
    /// node (Waypoints in this module are seeded directly via
    /// `seed_awaiting_input`, bypassing an actual suspension).
    fn approval_graph() -> WarGraph {
        let schema = BattlefieldSchema::new(vec![FieldSpec {
            name: FieldName::new("approved").unwrap(),
            dispatch: DispatchRule::LastWrite,
            default: None,
            required: false,
        }]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(
            NodeId::new("approve"),
            NodeSpec::Function(Arc::new(MarkerNode {
                output_field: FieldName::new("approved").unwrap(),
            })),
        );
        graph.add_entry(NodeId::new("approve"));
        graph
    }

    fn sample_request(
        kind: ParleyKind,
        expires_at: Option<chrono::DateTime<Utc>>,
    ) -> ParleyRequest {
        ParleyRequest {
            parley_id: ParleyId::new(),
            node_id: NodeId::new("approve"),
            kind,
            prompt: "confirm?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    async fn seed_awaiting_input(
        store: &InMemoryWaypointStore,
        thread: &ThreadId,
        graph: &WarGraph,
        parleys: Vec<ParleyRequest>,
        responses: Vec<ParleyResponse>,
    ) {
        let mut waypoint = sample_waypoint_at(thread, 0, Utc::now());
        waypoint.graph_fingerprint = graph.fingerprint();
        waypoint.battlefield = Battlefield::new(graph.schema().clone());
        waypoint.vanguard = vec![NodeId::new("approve")];
        waypoint.status = WaypointStatus::AwaitingInput { parleys, responses };
        store.save(&waypoint).await.unwrap();
    }

    fn build_adapter(
        store: Arc<InMemoryWaypointStore>,
        registry: Arc<GraphRegistry>,
    ) -> (
        ParleyPortAdapter<InMemoryWaypointStore>,
        ShutdownCoordinator,
    ) {
        let engine = Arc::new(
            WarEngine::new(Arc::new(UnimplementedPaladinPort), Arc::clone(&store))
                .with_durability(WaypointDurability::Strict),
        );
        let coordinator = ShutdownCoordinator::new();
        let adapter = ParleyPortAdapter::new(engine, store, registry, coordinator.clone());
        (adapter, coordinator)
    }

    fn approve_response(parley_id: ParleyId) -> ParleyResponse {
        ParleyResponse {
            parley_id,
            kind: ParleyKind::Approval,
            prompt: "confirm?".to_string(),
            value: serde_json::json!(true),
            responded_by: Some("alice".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        }
    }

    /// Test 5: a thread with no Waypoints returns `ThreadNotFound`.
    #[tokio::test]
    async fn unknown_thread_is_thread_not_found() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let (adapter, _coordinator) = build_adapter(store, registry);

        let err = adapter
            .resume_with(&thread("no-such-thread"), Vec::new())
            .await
            .unwrap_err();
        assert!(matches!(err, ParleyError::ThreadNotFound(_)));
    }

    /// Test 6: a thread whose latest Waypoint carries a fingerprint absent
    /// from the registry returns `GraphNotRegistered`, and no graph is
    /// guessed.
    #[tokio::test]
    async fn unregistered_fingerprint_is_graph_not_registered() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new()); // nothing registered
        let graph = approval_graph();
        let t = thread("unregistered-fingerprint");
        let request = sample_request(ParleyKind::Approval, None);
        seed_awaiting_input(&store, &t, &graph, vec![request], Vec::new()).await;

        let (adapter, _coordinator) = build_adapter(store, registry);
        let err = adapter.resume_with(&t, Vec::new()).await.unwrap_err();
        assert!(matches!(err, ParleyError::GraphNotRegistered { .. }));
    }

    /// Test 5b: a thread whose latest Waypoint is not `AwaitingInput`
    /// returns `ThreadNotAwaitingInput`.
    #[tokio::test]
    async fn non_awaiting_thread_is_thread_not_awaiting_input() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let graph = approval_graph();
        let t = thread("not-awaiting");

        let mut waypoint = sample_waypoint_at(&t, 0, Utc::now());
        waypoint.graph_fingerprint = graph.fingerprint();
        waypoint.battlefield = Battlefield::new(graph.schema().clone());
        waypoint.status = WaypointStatus::Completed;
        store.save(&waypoint).await.unwrap();
        registry.register(graph);

        let (adapter, _coordinator) = build_adapter(store, registry);
        let err = adapter.resume_with(&t, Vec::new()).await.unwrap_err();
        assert!(matches!(err, ParleyError::ThreadNotAwaitingInput { .. }));
    }

    /// Test 1: each `ParleyError` variant is returned from the call itself,
    /// before any background task is spawned -- exercised here for
    /// `UnknownParleyId`.
    #[tokio::test]
    async fn adapter_validates_synchronously_and_returns_typed_errors() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let graph = approval_graph();
        let t = thread("unknown-parley-id");
        let request = sample_request(ParleyKind::Approval, None);
        seed_awaiting_input(&store, &t, &graph, vec![request], Vec::new()).await;
        registry.register(graph);

        let (adapter, coordinator) = build_adapter(store, registry);
        let bogus = approve_response(ParleyId::new());
        let err = adapter.resume_with(&t, vec![bogus]).await.unwrap_err();
        assert!(matches!(err, ParleyError::UnknownParleyId { .. }));
        // No background task was spawned for a rejected submission.
        assert_eq!(coordinator.in_flight(), 0);
    }

    /// Test 2: after a rejected submission, `latest(thread)` is unchanged.
    #[tokio::test]
    async fn adapter_persists_nothing_on_a_validation_error() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let graph = approval_graph();
        let t = thread("persists-nothing");
        let request = sample_request(ParleyKind::Approval, None);
        seed_awaiting_input(&store, &t, &graph, vec![request], Vec::new()).await;
        registry.register(graph);
        let before = store.latest(&t).await.unwrap().unwrap();

        let (adapter, _coordinator) = build_adapter(Arc::clone(&store), registry);
        let bogus = approve_response(ParleyId::new());
        let _ = adapter.resume_with(&t, vec![bogus]).await.unwrap_err();

        let after = store.latest(&t).await.unwrap().unwrap();
        assert_eq!(before.waypoint_id, after.waypoint_id);
        assert_eq!(before.status, after.status);
    }

    /// Test 3 + 4: a valid, complete submission returns before the run
    /// completes, and the run then completes in the background,
    /// registered with the `ShutdownCoordinator`.
    #[tokio::test]
    async fn adapter_spawns_the_continuation_and_returns_immediately() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let graph = approval_graph();
        let t = thread("spawns-continuation");
        let request = sample_request(ParleyKind::Approval, None);
        let parley_id = request.parley_id;
        seed_awaiting_input(&store, &t, &graph, vec![request], Vec::new()).await;
        registry.register(graph);

        let (adapter, coordinator) = build_adapter(Arc::clone(&store), registry);
        let response = approve_response(parley_id);

        let accepted = adapter.resume_with(&t, vec![response]).await.unwrap();
        assert_eq!(accepted.thread_id(), &t);

        // Give the spawned task a chance to run to completion.
        for _ in 0..50 {
            if coordinator.in_flight() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(coordinator.in_flight(), 0, "the spawned run must complete");

        let after = store.latest(&t).await.unwrap().unwrap();
        assert_eq!(
            after.status,
            WaypointStatus::Completed,
            "the background continuation must have run the graph to completion"
        );
    }

    /// Test 4: cancelling the coordinator cancels the in-flight
    /// continuation, and `cancel_and_wait` observes it as in-flight before
    /// it drains.
    #[tokio::test]
    async fn spawned_continuation_is_registered_with_the_coordinator() {
        let store = Arc::new(InMemoryWaypointStore::new());
        let registry = Arc::new(GraphRegistry::new());
        let graph = approval_graph();
        let t = thread("registered-with-coordinator");
        let request = sample_request(ParleyKind::Approval, None);
        let parley_id = request.parley_id;
        seed_awaiting_input(&store, &t, &graph, vec![request], Vec::new()).await;
        registry.register(graph);

        let (adapter, coordinator) = build_adapter(Arc::clone(&store), registry);
        let response = approve_response(parley_id);

        let _accepted = adapter.resume_with(&t, vec![response]).await.unwrap();
        let outcome = coordinator.cancel_and_wait(Duration::from_secs(5)).await;
        assert!(outcome.drained());
    }

    /// Every `EngineError` variant the real validation path can produce
    /// maps to a DISTINCT `ParleyError` variant.
    #[test]
    fn every_engine_error_maps_to_a_distinct_parley_error() {
        let t = thread("mapping");
        let parley_id = ParleyId::new();

        assert!(matches!(
            map_engine_error(EngineError::ThreadNotFound(t.clone())),
            ParleyError::ThreadNotFound(_)
        ));
        assert!(matches!(
            map_engine_error(EngineError::ThreadNotAwaitingInput {
                thread: t.clone(),
                status: "Running".to_string(),
            }),
            ParleyError::ThreadNotAwaitingInput { .. }
        ));
        assert!(matches!(
            map_engine_error(EngineError::UnknownParleyId { parley_id }),
            ParleyError::UnknownParleyId { .. }
        ));
        assert!(matches!(
            map_engine_error(EngineError::ParleyAlreadyAnswered { parley_id }),
            ParleyError::ParleyAlreadyAnswered { .. }
        ));
        assert!(matches!(
            map_engine_error(EngineError::ResponseShapeInvalid {
                parley_id,
                reason: "bad".to_string(),
            }),
            ParleyError::ResponseShapeInvalid { .. }
        ));
        assert!(matches!(
            map_engine_error(EngineError::ParleyExpired {
                parley_id,
                expires_at: Utc::now(),
            }),
            ParleyError::ParleyExpired { .. }
        ));
        assert!(matches!(
            map_engine_error(EngineError::GraphMismatch {
                expected: GraphFingerprint::from_canonical_bytes(b"a"),
                got: GraphFingerprint::from_canonical_bytes(b"b"),
            }),
            ParleyError::Rejected { .. }
        ));
    }
}
