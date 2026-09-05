//! RED state (Phase 24 Plan 10, Task 2): `ParleyPortAdapter`, `GraphRegistry`
//! usage, and `map_engine_error` do not exist yet. See the GREEN commit
//! that follows for the full implementation and rustdoc.

use std::sync::Arc;

use chrono::Utc;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_battalion::engine::{EngineError, WarEngine, WarGraph};
use paladin_core::platform::container::waypoint::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse, ThreadId, WaypointStatus,
};
use paladin_ports::input::parley_port::{ParleyError, ParleyPort};
use paladin_ports::output::waypoint_port::WaypointPort;

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
