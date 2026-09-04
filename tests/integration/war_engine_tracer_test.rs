//! End-to-end tracer test for Phase 22 Plan 01: one typed node, checkpointed
//! automatically as a Waypoint, resumed by a freshly constructed engine with
//! zero re-execution.
//!
//! Proves the layer boundary chain end-to-end: `paladin-core` (Battlefield,
//! Waypoint) -> `paladin-ports` (WaypointPort) -> `paladin-storage`
//! (InMemoryWaypointStore) -> `paladin-battalion` (WarEngine).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use paladin_battalion::engine::{
    EngineError, EngineLimits, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode, WarEngine,
    WarGraph,
};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// A `PaladinPort` that is never called: this tracer only exercises
/// `NodeSpec::Function` nodes, but `WarEngine::new` requires a port anyway
/// (the seam `NodeSpec::Paladin` execution wires into, later plan scope).
struct UnimplementedPaladinPort;

#[async_trait]
impl PaladinPort for UnimplementedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unimplemented!("the tracer only runs Function nodes")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("the tracer only runs Function nodes")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A deterministic `Function` node that records how many times it ran and
/// always returns the same fixed delta.
struct CountingNode {
    field: FieldName,
    run_count: Arc<AtomicUsize>,
}

#[async_trait]
impl StateNode for CountingNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        self.run_count.fetch_add(1, Ordering::SeqCst);
        let mut delta = StateDelta::new();
        delta
            .set(self.field.clone(), "checkpointed")
            .map_err(|e| NodeError(e.to_string()))?;
        Ok(delta.into())
    }
}

fn one_field_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![FieldSpec::new(
        FieldName::new("result").expect("non-empty field name"),
        DispatchRule::LastWrite,
        None,
        false,
    )])
}

fn build_graph(run_count: Arc<AtomicUsize>) -> WarGraph {
    let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
    let node_id = NodeId::new("solo");
    graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(CountingNode {
            field: FieldName::new("result").expect("non-empty field name"),
            run_count,
        })),
    );
    graph.add_entry(node_id);
    graph
}

#[tokio::test]
async fn start_checkpoints_once_and_resume_never_reexecutes() {
    let store = Arc::new(InMemoryWaypointStore::new());
    let run_count = Arc::new(AtomicUsize::new(0));
    let graph = build_graph(run_count.clone());
    let thread = ThreadId::new("tracer-thread").expect("valid thread id");

    // First engine: start the run.
    let engine_one = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
    let outcome = engine_one
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("start should succeed");

    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let field = FieldName::new("result").unwrap();
            assert_eq!(
                final_state.get::<String>(&field).unwrap(),
                Some("checkpointed".to_string())
            );
        }
        other => panic!("expected RunOutcome::Completed, got {other:?}"),
    }
    assert_eq!(
        run_count.load(Ordering::SeqCst),
        1,
        "node must run exactly once on start"
    );

    let history = store
        .history(&thread, None, None)
        .await
        .expect("history should succeed");
    assert_eq!(history.len(), 1, "start must persist exactly one Waypoint");
    assert_eq!(history[0].status, WaypointStatus::Completed);

    // Second, FRESHLY CONSTRUCTED engine over the SAME store: resume.
    let engine_two = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
    let resumed = engine_two
        .resume(&graph, thread.clone())
        .await
        .expect("resume should succeed");

    match resumed {
        RunOutcome::Completed { .. } => {}
        other => panic!("expected RunOutcome::Completed on resume, got {other:?}"),
    }
    assert_eq!(
        run_count.load(Ordering::SeqCst),
        1,
        "resume must not re-execute the completed node"
    );

    let history_after_resume = store
        .history(&thread, None, None)
        .await
        .expect("history should succeed");
    assert_eq!(
        history_after_resume.len(),
        1,
        "resume must not write a second Waypoint"
    );
}

#[tokio::test]
async fn resume_on_unknown_thread_returns_thread_not_found() {
    let store = Arc::new(InMemoryWaypointStore::new());
    let run_count = Arc::new(AtomicUsize::new(0));
    let graph = build_graph(run_count);
    let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store);
    let thread = ThreadId::new("never-started").expect("valid thread id");

    let err = engine
        .resume(&graph, thread)
        .await
        .expect_err("resume on an unknown thread must fail");
    assert!(
        matches!(err, EngineError::ThreadNotFound(_)),
        "expected ThreadNotFound, got {err:?}"
    );
}

#[tokio::test]
async fn resume_with_altered_graph_returns_graph_mismatch() {
    let store = Arc::new(InMemoryWaypointStore::new());
    let run_count = Arc::new(AtomicUsize::new(0));
    let graph = build_graph(run_count.clone());
    let thread = ThreadId::new("mismatch-thread").expect("valid thread id");

    let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
    engine
        .start(&graph, thread.clone(), StateDelta::new())
        .await
        .expect("start should succeed");

    // Alter the graph's schema (add a second field) before resuming -- this
    // changes WarGraph::fingerprint()'s output (ENG-FR-14).
    let mut altered_schema = one_field_schema();
    altered_schema.fields.push(FieldSpec::new(
        FieldName::new("extra").expect("non-empty field name"),
        DispatchRule::LastWrite,
        None,
        false,
    ));
    let mut altered_graph = WarGraph::new(altered_schema, EngineLimits::default());
    let node_id = NodeId::new("solo");
    altered_graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(CountingNode {
            field: FieldName::new("result").expect("non-empty field name"),
            run_count,
        })),
    );
    altered_graph.add_entry(node_id);

    let err = engine
        .resume(&altered_graph, thread)
        .await
        .expect_err("resume against a graph with a different fingerprint must fail");
    assert!(
        matches!(err, EngineError::GraphMismatch { .. }),
        "expected GraphMismatch, got {err:?}"
    );
}
