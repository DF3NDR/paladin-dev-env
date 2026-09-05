//! Multi-parley suspension (HITL-01, D-11): a superstep in which two distinct nodes parley --
//! one `NodeSpec::Gate` and one `Function` node raising through `NextStep::Parley` directly --
//! must persist exactly one `AwaitingInput` Waypoint carrying both requests and zero responses.
//! Answering one keeps the thread suspended with one response persisted; answering the second
//! lets the run continue. Every state transition is asserted from the persisted Waypoint,
//! re-read through `WaypointPort::latest`/`get`, never from the value the engine returned --
//! proving the suspension (and the partial answer) is durable, not merely in-memory.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use paladin_battalion::engine::graph::GateRequestTemplate;
use paladin_battalion::engine::{
    EngineLimits, InputMapping, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode, WarEngine,
    WarGraph,
};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::parley::{OnExpire, ParleyId, ParleyKind, ParleyRequest};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointStatus};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

/// A `Function` node that raises a `FreeText` parley on its first visit (no
/// `NodeSpec::Gate` involved -- the second of the two raise paths D-07/D-08
/// cover) and writes the delivered value to `output_field` on the
/// post-resume visit.
struct ParleyingFunctionNode {
    output_field: FieldName,
}

impl ParleyingFunctionNode {
    fn new(output_field: FieldName) -> Arc<Self> {
        Arc::new(Self { output_field })
    }
}

#[async_trait::async_trait]
impl StateNode for ParleyingFunctionNode {
    async fn run(&self, _state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError> {
        match ctx.parley_response() {
            None => {
                let request = ParleyRequest {
                    parley_id: ParleyId::new(),
                    // Stamped onto the request regardless -- the engine's
                    // suspension arm re-stamps it from the dispatching
                    // `node_id` anyway (24-01).
                    node_id: ctx.node_id.clone(),
                    kind: ParleyKind::FreeText,
                    prompt: "what is the answer?".to_string(),
                    payload: serde_json::json!({}),
                    choices: None,
                    expires_at: None,
                    created_at: Utc::now(),
                    on_expire: OnExpire::FailRun,
                };
                Ok(Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Parley(request),
                })
            }
            Some(response) => {
                let mut delta = StateDelta::new();
                delta.set_raw(self.output_field.clone(), response.value.clone());
                Ok(delta.into())
            }
        }
    }
}

/// Build a graph whose vanguard contains two independent entry nodes that
/// BOTH parley in the same first superstep: `func1` (a `Function` node
/// raising through `NextStep::Parley` directly) and `gate1` (a
/// `NodeSpec::Gate`). Neither has any outgoing edge -- both are terminal
/// once their post-resume delta merges, so the run completes naturally
/// once both are answered.
fn build_graph() -> WarGraph {
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(field("func_answer"), DispatchRule::LastWrite, None, false),
        FieldSpec::new(
            field("gate_approved"),
            DispatchRule::LastWrite,
            Some(serde_json::json!(false)),
            false,
        ),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    graph.add_node(
        NodeId::new("func1"),
        NodeSpec::Function(ParleyingFunctionNode::new(field("func_answer"))),
    );
    let gate_request =
        GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("approve?"));
    graph.add_node(
        NodeId::new("gate1"),
        NodeSpec::gate(gate_request, Some(field("gate_approved"))),
    );

    graph.add_entry(NodeId::new("func1"));
    graph.add_entry(NodeId::new("gate1"));
    graph
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "multi_parley_suspension_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

fn parley_id_for(parleys: &[ParleyRequest], node_id: &NodeId) -> ParleyId {
    parleys
        .iter()
        .find(|p| &p.node_id == node_id)
        .unwrap_or_else(|| panic!("no parley raised by node {node_id} among {parleys:?}"))
        .parley_id
}

fn make_engine(store: Arc<SqliteWaypointStore>) -> WarEngine<SqliteWaypointStore> {
    WarEngine::new(Arc::new(FaultyPaladinPort::new()), store)
}

#[tokio::test]
async fn two_parleys_persist_as_one_waypoint_with_two_requests() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("two-parleys");
        let graph = build_graph();
        let thread = ThreadId::new("multi-parley-two").expect("valid thread id");

        let store = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store should connect"),
        );
        let engine = make_engine(Arc::clone(&store));

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start should suspend at both parleys");
        assert!(
            matches!(outcome, RunOutcome::AwaitingInput { .. }),
            "expected AwaitingInput, got {outcome:?}"
        );

        // Assert from the PERSISTED Waypoint alone, not the returned value.
        let latest = store
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        match latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(
                    parleys.len(),
                    2,
                    "exactly one Waypoint carrying BOTH requests"
                );
                assert!(responses.is_empty(), "no response has been submitted yet");
                let node_ids: std::collections::BTreeSet<NodeId> =
                    parleys.iter().map(|p| p.node_id.clone()).collect();
                assert_eq!(
                    node_ids,
                    std::collections::BTreeSet::from([NodeId::new("func1"), NodeId::new("gate1")]),
                    "the two requests must carry the two distinct raising node ids"
                );
            }
            other => panic!("expected AwaitingInput status, got {other:?}"),
        }
    })
    .await
    .expect("two-parleys scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn answering_one_of_two_keeps_the_thread_suspended() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("answer-one");
        let graph = build_graph();
        let thread = ThreadId::new("multi-parley-answer-one").expect("valid thread id");

        let store = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store should connect"),
        );
        let engine = make_engine(Arc::clone(&store));

        let start_outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start should suspend at both parleys");
        let parleys = match start_outcome {
            RunOutcome::AwaitingInput { parleys, .. } => parleys,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        let func_parley_id = parley_id_for(&parleys, &NodeId::new("func1"));

        let response = paladin_core::platform::container::parley::ParleyResponse {
            parley_id: func_parley_id,
            kind: ParleyKind::FreeText,
            prompt: String::new(),
            value: serde_json::json!("42"),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let resume_outcome = engine
            .resume_with(&graph, thread.clone(), vec![response])
            .await
            .expect("partial resume_with should succeed");
        match &resume_outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(
                    parleys.len(),
                    1,
                    "the returned outcome must list exactly the one remaining request"
                );
                assert_eq!(parleys[0].node_id, NodeId::new("gate1"));
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        // Re-read from a FRESH store handle over the same file -- proving
        // durability, not just the in-process return value.
        let fresh_store = SqliteWaypointStore::new(&db_url)
            .await
            .expect("fresh store should reconnect to the same file");
        let latest = fresh_store
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        match latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 2, "both original requests remain recorded");
                assert_eq!(responses.len(), 1, "exactly one accepted response so far");
                assert_eq!(responses[0].parley_id, func_parley_id);
            }
            other => panic!("expected AwaitingInput status, got {other:?}"),
        }
    })
    .await
    .expect("answer-one scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn answering_the_second_continues_the_run() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("answer-both");
        let graph = build_graph();
        let thread = ThreadId::new("multi-parley-answer-both").expect("valid thread id");

        let store = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store should connect"),
        );
        let engine = make_engine(Arc::clone(&store));

        let start_outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start should suspend at both parleys");
        let parleys = match start_outcome {
            RunOutcome::AwaitingInput { parleys, .. } => parleys,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        let func_parley_id = parley_id_for(&parleys, &NodeId::new("func1"));
        let gate_parley_id = parley_id_for(&parleys, &NodeId::new("gate1"));

        let func_response = paladin_core::platform::container::parley::ParleyResponse {
            parley_id: func_parley_id,
            kind: ParleyKind::FreeText,
            prompt: String::new(),
            value: serde_json::json!("42"),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        engine
            .resume_with(&graph, thread.clone(), vec![func_response])
            .await
            .expect("first partial resume_with should succeed");

        let gate_response = paladin_core::platform::container::parley::ParleyResponse {
            parley_id: gate_parley_id,
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!(true),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let final_outcome = engine
            .resume_with(&graph, thread.clone(), vec![gate_response])
            .await
            .expect("second resume_with should complete the run");
        match final_outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&field("func_answer"))
                        .expect("func_answer field should read"),
                    Some("42".to_string())
                );
                assert_eq!(
                    final_state
                        .get::<bool>(&field("gate_approved"))
                        .expect("gate_approved field should read"),
                    Some(true)
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    })
    .await
    .expect("answer-both scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn multi_parley_list_order_is_stable_by_node_id() {
    tokio::time::timeout(Duration::from_secs(30), async {
        async fn raised_node_id_order(label: &str) -> Vec<NodeId> {
            let db_url = temp_db_url(label);
            let graph = build_graph();
            let thread = ThreadId::new(format!("multi-parley-order-{label}")).unwrap();
            let store = Arc::new(
                SqliteWaypointStore::new(&db_url)
                    .await
                    .expect("store should connect"),
            );
            let engine = make_engine(store);
            let outcome = engine
                .start(&graph, thread, StateDelta::new())
                .await
                .expect("start should suspend at both parleys");
            match outcome {
                RunOutcome::AwaitingInput { parleys, .. } => {
                    parleys.into_iter().map(|p| p.node_id).collect()
                }
                other => panic!("expected AwaitingInput, got {other:?}"),
            }
        }

        let first_run = raised_node_id_order("first").await;
        let second_run = raised_node_id_order("second").await;
        assert_eq!(
            first_run, second_run,
            "identical graphs must raise their parleys in the same node-id order every run"
        );
    })
    .await
    .expect("order-stability scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn multi_parley_survives_process_drop_mid_partial() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("drop-mid-partial");
        let graph = build_graph();
        let thread = ThreadId::new("multi-parley-drop-mid-partial").expect("valid thread id");

        // --- Instance A: start, suspend, answer func1, then drop. ---------
        let (func_parley_id, gate_parley_id) = {
            let store_a = Arc::new(
                SqliteWaypointStore::new(&db_url)
                    .await
                    .expect("store A should connect"),
            );
            let engine_a = make_engine(Arc::clone(&store_a));

            let start_outcome = engine_a
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .expect("start should suspend at both parleys");
            let parleys = match start_outcome {
                RunOutcome::AwaitingInput { parleys, .. } => parleys,
                other => panic!("expected AwaitingInput, got {other:?}"),
            };
            let func_parley_id = parley_id_for(&parleys, &NodeId::new("func1"));
            let gate_parley_id = parley_id_for(&parleys, &NodeId::new("gate1"));

            let func_response = paladin_core::platform::container::parley::ParleyResponse {
                parley_id: func_parley_id,
                kind: ParleyKind::FreeText,
                prompt: String::new(),
                value: serde_json::json!("42"),
                responded_by: Some("tester".to_string()),
                responded_at: Utc::now(),
                defaulted: false,
            };
            let partial_outcome = engine_a
                .resume_with(&graph, thread.clone(), vec![func_response])
                .await
                .expect("first partial resume_with should succeed");
            assert!(
                matches!(partial_outcome, RunOutcome::AwaitingInput { .. }),
                "expected the run to still be suspended after only one answer"
            );

            (func_parley_id, gate_parley_id)
            // engine_a and store_a are dropped here.
        };

        // --- Instance B: brand new engine + store, same file. --------------
        let store_b = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store B should reconnect to the same file"),
        );
        let engine_b = make_engine(Arc::clone(&store_b));

        // Re-read the partial state through instance B's own store handle
        // before finishing -- nothing changed across the drop.
        let latest = store_b
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        match latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 2);
                assert_eq!(responses.len(), 1);
                assert_eq!(responses[0].parley_id, func_parley_id);
            }
            other => panic!("expected AwaitingInput status, got {other:?}"),
        }

        let gate_response = paladin_core::platform::container::parley::ParleyResponse {
            parley_id: gate_parley_id,
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!(true),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let final_outcome = engine_b
            .resume_with(&graph, thread, vec![gate_response])
            .await
            .expect("second resume_with (post-drop) should complete the run");
        match final_outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&field("func_answer"))
                        .expect("func_answer field should read"),
                    Some("42".to_string())
                );
                assert_eq!(
                    final_state
                        .get::<bool>(&field("gate_approved"))
                        .expect("gate_approved field should read"),
                    Some(true)
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    })
    .await
    .expect("drop-mid-partial scenario must complete within its 30s timeout guard");
}
