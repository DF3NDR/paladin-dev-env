//! Golden output-equivalence tests (ENG-FR-19, T-22-37): a 3-node Formation, a 3-node
//! Phalanx and a branching Campaign, each run once through its legacy execution service and
//! once through `WarEngine` over the matching `WarGraph::from_formation`/`from_phalanx`/
//! `from_campaign` bridge, driven by the SAME configured mock port. Every comparison in this
//! file is a raw, unnormalized `assert_eq!` over the plain strings involved -- this plan's own
//! acceptance grep forbids whitespace trimming, case folding, or substring substitution
//! anywhere in this file. A normalized comparison would hide exactly the class of bridging
//! defect these tests exist to catch: the legacy Campaign fan-in separator and every
//! `InputMapping` template both turn on exact whitespace.
//!
//! Two independently-constructed `FaultyPaladinPort` instances are used per test (one per
//! path) rather than one shared instance: `FaultyPaladinPort::execute` is a pure function of
//! `(paladin name, input)` with no cross-call state beyond its own call counters, so two
//! unconfigured instances behave identically -- this keeps each path's own call log free of
//! the other path's entries, with no index-slicing required to separate them.

use std::sync::Arc;

use paladin_battalion::campaign_service::CampaignExecutionService;
use paladin_battalion::engine::{
    RunOutcome, WarEngine, WarGraph, campaign_node_ids, dedicated_output_field,
};
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin_core::platform::container::battalion::formation::Formation;
use paladin_core::platform::container::battalion::phalanx::Phalanx;
use paladin_core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_core::platform::container::paladin::{Paladin, PaladinData};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this standalone
// [[test]] target only needs `FaultyPaladinPort`, matching the pattern already established
// by `tests/integration/e2e_crash_resume_test.rs`.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        name: name.to_string(),
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

fn input_field() -> FieldName {
    FieldName::new("input").unwrap()
}

fn initial_with_input(value: &str) -> StateDelta {
    let mut delta = StateDelta::new();
    delta.set(input_field(), value).unwrap();
    delta
}

// --- 3-node Formation --------------------------------------------------------------------

#[tokio::test]
async fn formation_3_node_matches_legacy_final_output_and_per_paladin_inputs() {
    let paladins = vec![
        make_paladin("form-p1"),
        make_paladin("form-p2"),
        make_paladin("form-p3"),
    ];

    let legacy_port = Arc::new(FaultyPaladinPort::new());
    let legacy_formation =
        Formation::new(paladins.clone(), BattalionConfig::new("golden-formation")).unwrap();
    let legacy_service = FormationExecutionService::new(legacy_port.clone());
    let legacy_result = legacy_service
        .execute(&legacy_formation, "seed input")
        .await
        .unwrap();

    let bridged_graph = WarGraph::from_formation(paladins);
    let bridged_port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(bridged_port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let outcome = engine
        .start(
            &bridged_graph,
            ThreadId::new("golden-formation").unwrap(),
            initial_with_input("seed input"),
        )
        .await
        .unwrap();
    let bridged_final = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state
            .get::<String>(&FieldName::new("output").unwrap())
            .unwrap()
            .unwrap(),
        other => panic!("expected Completed, got {other:?}"),
    };

    assert_eq!(bridged_final, legacy_result.final_output);

    // Formation nodes execute strictly one after another in both paths (no
    // concurrency in either implementation), so the raw call log order is
    // itself deterministic and comparable directly, with no sort needed.
    assert_eq!(legacy_port.execution_log(), bridged_port.execution_log());
}

// --- 3-node Phalanx ------------------------------------------------------------------------

#[tokio::test]
async fn phalanx_3_node_matches_legacy_collected_results_and_per_paladin_inputs() {
    let paladins = vec![
        make_paladin("phal-q1"),
        make_paladin("phal-q2"),
        make_paladin("phal-q3"),
    ];

    let legacy_port = Arc::new(FaultyPaladinPort::new());
    let legacy_phalanx =
        Phalanx::new(paladins.clone(), BattalionConfig::new("golden-phalanx")).unwrap();
    let legacy_service = PhalanxExecutionService::new(legacy_port.clone());
    let legacy_result = legacy_service
        .execute(&legacy_phalanx, "phalanx input")
        .await
        .unwrap();
    let legacy_outputs: Vec<String> = legacy_result
        .paladin_results
        .iter()
        .map(|r| r.output.clone())
        .collect();

    let bridged_graph = WarGraph::from_phalanx(paladins);
    let bridged_port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(bridged_port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let outcome = engine
        .start(
            &bridged_graph,
            ThreadId::new("golden-phalanx").unwrap(),
            initial_with_input("phalanx input"),
        )
        .await
        .unwrap();
    let history: Vec<String> = match outcome {
        RunOutcome::Completed { final_state, .. } => final_state
            .get(&FieldName::new("history").unwrap())
            .unwrap()
            .unwrap(),
        other => panic!("expected Completed, got {other:?}"),
    };

    // `PhalanxExecutionService::execute_collect_all` awaits each spawned
    // task's `JoinHandle` sequentially BY INDEX (not by completion order), so
    // `paladin_results` is always in `phalanx.paladins()`'s own Vec order;
    // `Battlefield::merge`'s Append dispatch is always sorted by `NodeId`,
    // and `from_phalanx`'s zero-padded NodeIds keep that sort aligned with
    // the same Vec order -- both sides are deterministic despite concurrent
    // real execution, so a direct in-order comparison is valid.
    assert_eq!(history, legacy_outputs);

    // The raw per-call execution log's REAL order is not deterministic on
    // either side (both paths run their three Paladins concurrently); sort
    // before comparing so this assertion checks exact per-paladin input
    // content, not incidental completion timing.
    let mut legacy_log = legacy_port.execution_log();
    let mut bridged_log = bridged_port.execution_log();
    legacy_log.sort();
    bridged_log.sort();
    assert_eq!(legacy_log, bridged_log);
}

// --- Branching Campaign (diamond fan-out / fan-in) ------------------------------------------

struct DiamondCampaign {
    campaign: Campaign,
    d: uuid::Uuid,
}

fn build_diamond_campaign(name: &str) -> DiamondCampaign {
    let mut campaign = Campaign::new(BattalionConfig::new(name));
    let a = campaign.add_paladin(make_paladin("paladin_a"));
    let b = campaign.add_paladin(make_paladin("paladin_b"));
    let c = campaign.add_paladin(make_paladin("paladin_c"));
    let d = campaign.add_paladin(make_paladin("paladin_d"));
    campaign
        .add_edge(CampaignEdge::new(a, b, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(a, c, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(b, d, EdgeCondition::Always))
        .unwrap();
    campaign
        .add_edge(CampaignEdge::new(c, d, EdgeCondition::Always))
        .unwrap();
    campaign.set_entry_point(a).unwrap();
    DiamondCampaign { campaign, d }
}

#[tokio::test]
async fn campaign_diamond_matches_legacy_final_output_including_fan_in_concatenation() {
    let DiamondCampaign { campaign, d } = build_diamond_campaign("golden-campaign-diamond");

    let legacy_port = Arc::new(FaultyPaladinPort::new());
    let legacy_service = CampaignExecutionService::new(legacy_port.clone());
    let legacy_result = legacy_service
        .execute(&campaign, "diamond input")
        .await
        .unwrap();

    let node_ids = campaign_node_ids(&campaign);
    let bridged_graph = WarGraph::from_campaign(&campaign);
    let bridged_port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(bridged_port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let outcome = engine
        .start(
            &bridged_graph,
            ThreadId::new("golden-campaign-diamond").unwrap(),
            initial_with_input("diamond input"),
        )
        .await
        .unwrap();
    let bridged_final = match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let d_field = dedicated_output_field(&node_ids[&d]);
            final_state.get::<String>(&d_field).unwrap().unwrap()
        }
        other => panic!("expected Completed, got {other:?}"),
    };

    // `compute_final_output` is `results.last().output`, and D is always the
    // last node in the campaign's own toposort order (its two parents must
    // both complete first); `from_campaign`'s per-parent template order is
    // built from the SAME `campaign.graph().edges_directed(..., Incoming)`
    // call the legacy service's own `aggregate_inputs_for_node` makes at
    // runtime, over the SAME `Campaign` value -- so D's fan-in concatenation
    // order (and therefore D's own configured mock response, which echoes
    // its received input) is identical on both paths, not merely
    // order-tolerant.
    assert_eq!(bridged_final, legacy_result.final_output);

    // B and C run concurrently in the bridged path (same superstep) but
    // sequentially, in a fixed toposort order, in the legacy path -- sort
    // both logs before comparing so this checks exact per-paladin input
    // content rather than incidental execution-order differences between a
    // sequential loop and a concurrently-spawned superstep.
    let mut legacy_log = legacy_port.execution_log();
    let mut bridged_log = bridged_port.execution_log();
    legacy_log.sort();
    bridged_log.sort();
    assert_eq!(legacy_log, bridged_log);
}

// --- Campaign false-branch case --------------------------------------------------------------

#[tokio::test]
async fn campaign_false_branch_condition_produces_the_same_outcome_from_both_paths() {
    let mut campaign = Campaign::new(BattalionConfig::new("golden-campaign-false-branch"));
    let a = campaign.add_paladin(make_paladin("branch_a"));
    let b = campaign.add_paladin(make_paladin("branch_b"));
    // A marker `FaultyPaladinPort` never emits on its own, so this edge
    // never fires on either path -- proving the false-branch case rather
    // than a condition that happens to be unreachable for unrelated reasons.
    campaign
        .add_edge(CampaignEdge::new(
            a,
            b,
            EdgeCondition::Contains("A_TEXT_NO_MOCK_PORT_EVER_EMITS".to_string()),
        ))
        .unwrap();
    campaign.set_entry_point(a).unwrap();

    let legacy_port = Arc::new(FaultyPaladinPort::new());
    let legacy_service = CampaignExecutionService::new(legacy_port.clone());
    let legacy_result = legacy_service
        .execute(&campaign, "false branch input")
        .await
        .unwrap();
    assert_eq!(
        legacy_port.call_count(),
        1,
        "branch_b must never execute once its incoming edge condition is false"
    );

    let node_ids = campaign_node_ids(&campaign);
    let bridged_graph = WarGraph::from_campaign(&campaign);
    let bridged_port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(bridged_port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let outcome = engine
        .start(
            &bridged_graph,
            ThreadId::new("golden-campaign-false-branch").unwrap(),
            initial_with_input("false branch input"),
        )
        .await
        .unwrap();
    let bridged_final = match outcome {
        RunOutcome::Completed { final_state, .. } => {
            let a_field = dedicated_output_field(&node_ids[&a]);
            final_state.get::<String>(&a_field).unwrap().unwrap()
        }
        other => panic!("expected Completed, got {other:?}"),
    };
    assert_eq!(
        bridged_port.call_count(),
        1,
        "branch_b must never execute once its incoming edge is resolved not-firing"
    );

    assert_eq!(bridged_final, legacy_result.final_output);
    assert_eq!(legacy_port.execution_log(), bridged_port.execution_log());
}

// --- Empty paladin list ----------------------------------------------------------------------

#[tokio::test]
async fn empty_paladin_list_produces_the_same_outcome_from_both_paths() {
    // Formation: the legacy DOMAIN TYPE rejects an empty list outright
    // (BattalionError::ValidationError) since `Formation::new` enforces a
    // minimum of one Paladin; the bridge takes a raw `Vec<Paladin>` with no
    // such domain constraint, so its outcome is "the graph validates and its
    // engine run completes immediately with zero executions" -- the same
    // underlying fact (no Paladin ever runs) expressed the only way each
    // API shape can express it.
    let formation_err =
        Formation::new(vec![], BattalionConfig::new("golden-empty-formation")).unwrap_err();
    assert!(matches!(formation_err, BattalionError::ValidationError(_)));

    let formation_graph = WarGraph::from_formation(vec![]);
    let formation_port = Arc::new(FaultyPaladinPort::new());
    let formation_engine = WarEngine::new(
        formation_port.clone(),
        Arc::new(InMemoryWaypointStore::new()),
    );
    let formation_outcome = formation_engine
        .start(
            &formation_graph,
            ThreadId::new("golden-empty-formation").unwrap(),
            StateDelta::new(),
        )
        .await
        .unwrap();
    assert!(matches!(formation_outcome, RunOutcome::Completed { .. }));
    assert_eq!(formation_port.call_count(), 0);

    // Phalanx: same story, with Phalanx's own minimum of two Paladins.
    let phalanx_err =
        Phalanx::new(vec![], BattalionConfig::new("golden-empty-phalanx")).unwrap_err();
    assert!(matches!(phalanx_err, BattalionError::ValidationError(_)));

    let phalanx_graph = WarGraph::from_phalanx(vec![]);
    let phalanx_port = Arc::new(FaultyPaladinPort::new());
    let phalanx_engine =
        WarEngine::new(phalanx_port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let phalanx_outcome = phalanx_engine
        .start(
            &phalanx_graph,
            ThreadId::new("golden-empty-phalanx").unwrap(),
            StateDelta::new(),
        )
        .await
        .unwrap();
    assert!(matches!(phalanx_outcome, RunOutcome::Completed { .. }));
    assert_eq!(phalanx_port.call_count(), 0);
}
