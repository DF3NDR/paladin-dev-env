//! Golden Mermaid/DOT export tests for [`GraphShape`] (OBS-03, D-20): five
//! fixtures -- `linear`, `branch_join`, `loop`, `muster`, `subgraph` -- each
//! rendered in both formats and compared byte-for-byte against a committed
//! golden file under `tests/golden/export/`. `UPDATE_GOLDEN=1` regenerates
//! every golden, mirroring `graph_doc_round_trip.rs`'s own
//! `UPDATE_WARGRAPH_SCHEMA=1` bless idiom and `crates/paladin-web/openapi.rs`'s
//! `UPDATE_OPENAPI=1` (`Makefile:370`); `make bless-golden` runs the bless
//! path for this file specifically.
//!
//! Four of the five fixtures are `WarGraphDoc` files under
//! `tests/fixtures/graph_docs/`; `doc_and_graph_shapes_agree` asserts
//! `GraphShape::from_doc` and `GraphShape::from_graph` (of the compiled
//! doc) produce an EQUAL shape for each. The fifth, `muster`, is built
//! directly in code: a Muster worker template and a `defer: true`
//! aggregator are not expressible in a `WarGraphDoc` (27-CONTEXT D-33), so
//! `GraphShape::from_graph` is the only path that can render it.
//!
//! `overlay_goldens` (28-10, D-20/D-21) extends this file with the ONE
//! overlay golden the plan names: a scripted run over the `branch_join`
//! fixture (`split` routes to `branch_a`, `branch_b` never fires, `join`
//! completes), rendered through [`ExecutionOverlay::from_waypoints`] and
//! [`ExecutionOverlay::from_trace_records`] into two DISTINCT goldens --
//! `branch_overlay_waypoints.mermaid` (derived fired edges, no evaluated
//! edges) and `branch_overlay_trace.mermaid` (exact fired AND
//! evaluated-but-not-fired edges) -- so the difference between the derived
//! and exact sources is visible in the committed corpus itself.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use paladin_battalion::engine::export::{
    ExecutionOverlay, GraphShape, to_dot, to_mermaid, to_mermaid_overlay,
};
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, InputMapping, NodeContext, NodeSpec, StateNode, StateNodeError,
    WarGraph, WarGraphDoc,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::trace::{TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{
    FrontierSnapshot, GraphFingerprint, NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId,
    Waypoint, WaypointStatus,
};

/// `tests/fixtures/graph_docs/` -- shared with `graph_doc_round_trip.rs`.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/graph_docs"
    ))
}

/// `tests/golden/export/` -- this file's own golden corpus (D-20).
fn golden_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/export"))
}

fn load_doc(fixture_file: &str) -> WarGraphDoc {
    let path = fixtures_dir().join(fixture_file);
    let contents =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Compares `actual` against the committed `<name>.<ext>` golden, or --
/// under `UPDATE_GOLDEN=1` -- (re)writes it and returns without asserting
/// (D-20's bless idiom, mirroring `graph_doc_round_trip.rs`'s
/// `UPDATE_WARGRAPH_SCHEMA=1`).
fn compare_or_bless(name: &str, ext: &str, actual: &str) {
    let path = golden_dir().join(format!("{name}.{ext}"));

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        return;
    }

    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        actual,
        committed,
        "{}.{ext} rendering drifted from {}. If intentional, regenerate with: \
         UPDATE_GOLDEN=1 cargo test -p paladin-battalion --test export_golden -- --quiet \
         (or `make bless-golden`)",
        name,
        path.display()
    );
}

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("{name} prompt"),
        name: name.to_string(),
        user_name: "TestUser".to_string(),
        model: "test-model".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

/// A `StateNode` whose `run` body is never exercised here -- this file only
/// builds and renders `WarGraph`s, it never runs one through the engine.
struct NoopFunctionNode;

#[async_trait]
impl StateNode for NoopFunctionNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(StateDelta::new().into())
    }
}

/// The `muster` fixture (D-20): `planner` (Function, entry) musters
/// `worker` (Paladin worker template, no static incoming edge, D-12) ->
/// `aggregator` (Function, `defer: true`) -- mirrors
/// `tests/integration/e2e_muster_defer_order_test.rs::build_graph`'s exact
/// shape. Neither the worker template nor the deferred aggregator is
/// expressible in a `WarGraphDoc` (27-CONTEXT D-33), so this fixture is
/// code-built, not loaded from a fixture file.
fn muster_graph() -> WarGraph {
    let worker_out = field("worker_out");
    let aggregated = field("aggregated");
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(worker_out.clone(), DispatchRule::Append, None, false),
        FieldSpec::new(aggregated, DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let planner = NodeId::new("planner");
    let worker = NodeId::new("worker");
    let aggregator = NodeId::new("aggregator");

    graph.add_node(
        planner.clone(),
        NodeSpec::Function(Arc::new(NoopFunctionNode)),
    );
    graph.add_worker_template(
        worker.clone(),
        NodeSpec::paladin(
            make_paladin("worker"),
            InputMapping::new("{muster.payload}"),
            worker_out,
        ),
    );
    graph.add_deferred_node(
        aggregator.clone(),
        NodeSpec::Function(Arc::new(NoopFunctionNode)),
    );
    graph.add_edge(EdgeSpec {
        from: worker,
        to: aggregator,
        condition: None,
    });
    graph.add_entry(planner);

    graph
}

/// One `(fixture name, GraphShape)` pair -- the table both `golden_exports`
/// and `doc_and_graph_shapes_agree` are built from.
struct Fixture {
    name: &'static str,
    shape: GraphShape,
}

fn doc_fixtures() -> Vec<(&'static str, &'static str)> {
    vec![
        ("linear", "linear.json"),
        ("branch_join", "branch_join.json"),
        ("loop", "loop.json"),
        ("subgraph", "nested_workflow.json"),
    ]
}

fn all_fixtures(registries: &EngineRegistries) -> Vec<Fixture> {
    let mut fixtures: Vec<Fixture> = doc_fixtures()
        .into_iter()
        .map(|(name, file)| {
            let doc = load_doc(file);
            let compiled = doc
                .compile(registries)
                .unwrap_or_else(|e| panic!("compile {file}: {e}"));
            Fixture {
                name,
                shape: GraphShape::from_graph(&compiled),
            }
        })
        .collect();

    fixtures.push(Fixture {
        name: "muster",
        shape: GraphShape::from_graph(&muster_graph()),
    });

    fixtures
}

/// Ten golden files -- Mermaid and DOT for each of the five fixtures --
/// compared byte-for-byte (or regenerated under `UPDATE_GOLDEN=1`).
#[test]
fn golden_exports() {
    let registries = EngineRegistries::new();
    for fixture in all_fixtures(&registries) {
        let mermaid = to_mermaid(&fixture.shape);
        let dot = to_dot(&fixture.shape);
        compare_or_bless(fixture.name, "mermaid", &mermaid);
        compare_or_bless(fixture.name, "dot", &dot);
    }
}

/// `GraphShape::from_doc(doc)` equals `GraphShape::from_graph(&compiled)`
/// for every doc-expressible fixture (D-18, D-20) -- the "one shape, two
/// sources" contract that lets the exporters, the overlay and the CLI treat
/// a raw document and a compiled graph interchangeably.
#[test]
fn doc_and_graph_shapes_agree() {
    let registries = EngineRegistries::new();
    for (name, file) in doc_fixtures() {
        let doc = load_doc(file);
        let compiled = doc
            .compile(&registries)
            .unwrap_or_else(|e| panic!("compile {file}: {e}"));

        let from_doc = GraphShape::from_doc(&doc);
        let from_graph = GraphShape::from_graph(&compiled);

        assert_eq!(from_doc, from_graph, "{name}: from_doc != from_graph");
    }
}

/// The `muster` fixture's worker template renders dashed and badged
/// `«worker»`, and its deferred aggregator renders dashed too (D-19,
/// 28-UI-SPEC.md) -- the behavior a doc-only exporter could never prove.
#[test]
fn muster_renders_worker_template_and_deferred_node() {
    let shape = GraphShape::from_graph(&muster_graph());
    let mermaid = to_mermaid(&shape);

    assert!(mermaid.contains("«worker»"));
    assert!(mermaid.contains("dashed"));
    assert!(mermaid.contains("classDef"));

    let worker_node = shape
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "worker")
        .expect("worker node present");
    assert!(worker_node.worker_template);
    assert!(!worker_node.deferred);

    let aggregator_node = shape
        .nodes
        .iter()
        .find(|n| n.id.as_str() == "aggregator")
        .expect("aggregator node present");
    assert!(aggregator_node.deferred);
    assert!(!aggregator_node.worker_template);
}

/// The overlay golden's own thread (D-20): a single scripted run over the
/// `branch_join` fixture, shared by both the Waypoint-sourced and
/// trace-sourced overlay builders below.
fn overlay_thread() -> ThreadId {
    ThreadId::new("branch-overlay-golden").expect("valid thread id")
}

/// A minimal root `Waypoint` carrying only what `ExecutionOverlay::from_waypoints`
/// reads: `superstep`, `vanguard`, `completed`.
fn overlay_waypoint(
    superstep: u64,
    vanguard: Vec<&str>,
    completed: Vec<NodeExecutionRecord>,
) -> Waypoint {
    Waypoint::new_root(
        overlay_thread(),
        superstep,
        GraphFingerprint::from_canonical_bytes(b"branch-overlay-golden-graph"),
        Battlefield::new(BattlefieldSchema::new(Vec::new())),
        vanguard.into_iter().map(NodeId::new).collect(),
        completed,
        WaypointStatus::Running,
        BTreeMap::new(),
        FrontierSnapshot::default(),
    )
}

fn overlay_record(
    node_id: &str,
    outcome: NodeOutcomeKind,
    duration_ms: u64,
    token_count: u64,
) -> NodeExecutionRecord {
    NodeExecutionRecord {
        node_id: NodeId::new(node_id),
        paladin_id: None,
        started_at: Utc::now(),
        duration_ms,
        token_count,
        outcome,
        attempt: 1,
        attempts: Vec::new(),
        cache_hit: false,
    }
}

/// The scripted `branch_join` run's Waypoint history (D-20, D-21): `split`
/// routes to `branch_a` (`route` contains `"a"`), `branch_b` never fires,
/// `join` completes -- three Waypoints, one per superstep.
fn branch_join_waypoints() -> Vec<Waypoint> {
    vec![
        overlay_waypoint(
            1,
            vec!["branch_a"],
            vec![overlay_record("split", NodeOutcomeKind::Succeeded, 120, 45)],
        ),
        overlay_waypoint(
            2,
            vec!["join"],
            vec![overlay_record(
                "branch_a",
                NodeOutcomeKind::Succeeded,
                80,
                30,
            )],
        ),
        overlay_waypoint(
            3,
            vec![],
            vec![overlay_record("join", NodeOutcomeKind::Succeeded, 60, 20)],
        ),
    ]
}

/// The SAME scripted `branch_join` run (D-20, D-21) as a persisted trace
/// record stream: exact `EdgeEvaluated` records for both branches of
/// `split` (only `branch_a` fires), so the trace-sourced overlay's
/// `evaluated_edges` differs from the Waypoint-sourced overlay's (always
/// empty).
fn branch_join_trace_records() -> Vec<TraceRecord> {
    let events = vec![
        TraceEvent::NodeFinished {
            superstep: 1,
            node_id: NodeId::new("split"),
            attempt: 1,
            outcome: NodeOutcomeKind::Succeeded,
            duration_ms: 120,
            token_count: 45,
            cache_hit: false,
        },
        TraceEvent::EdgeEvaluated {
            from: NodeId::new("split"),
            to: NodeId::new("branch_a"),
            condition_kind: "contains".to_string(),
            fired: true,
        },
        TraceEvent::EdgeEvaluated {
            from: NodeId::new("split"),
            to: NodeId::new("branch_b"),
            condition_kind: "contains".to_string(),
            fired: false,
        },
        TraceEvent::NodeFinished {
            superstep: 2,
            node_id: NodeId::new("branch_a"),
            attempt: 1,
            outcome: NodeOutcomeKind::Succeeded,
            duration_ms: 80,
            token_count: 30,
            cache_hit: false,
        },
        TraceEvent::EdgeEvaluated {
            from: NodeId::new("branch_a"),
            to: NodeId::new("join"),
            condition_kind: "always".to_string(),
            fired: true,
        },
        TraceEvent::NodeFinished {
            superstep: 3,
            node_id: NodeId::new("join"),
            attempt: 1,
            outcome: NodeOutcomeKind::Succeeded,
            duration_ms: 60,
            token_count: 20,
            cache_hit: false,
        },
    ];
    events
        .into_iter()
        .enumerate()
        .map(|(i, event)| TraceRecord {
            thread_id: overlay_thread(),
            run_id: None,
            seq: i as u64 + 1,
            at: Utc::now(),
            event,
        })
        .collect()
}

/// The overlay golden (D-20): the ONE overlay golden the plan names -- the
/// `branch_join` fixture with a scripted run -- rendered through BOTH
/// overlay sources into two distinct goldens
/// (`branch_overlay_waypoints.mermaid`, `branch_overlay_trace.mermaid`) so
/// the derived-vs-exact difference in `evaluated_edges` is visible in the
/// committed corpus itself (D-21), checked by the same
/// `UPDATE_GOLDEN=1`/`compare_or_bless` idiom as the static goldens.
#[test]
fn overlay_goldens() {
    let registries = EngineRegistries::new();
    let doc = load_doc("branch_join.json");
    let compiled = doc.compile(&registries).expect("compile branch_join.json");
    let shape = GraphShape::from_graph(&compiled);

    let waypoint_overlay = ExecutionOverlay::from_waypoints(&branch_join_waypoints(), &shape);
    let waypoint_mermaid = to_mermaid_overlay(&shape, &waypoint_overlay);
    compare_or_bless("branch_overlay_waypoints", "mermaid", &waypoint_mermaid);

    let trace_overlay = ExecutionOverlay::from_trace_records(&branch_join_trace_records());
    let trace_mermaid = to_mermaid_overlay(&shape, &trace_overlay);
    compare_or_bless("branch_overlay_trace", "mermaid", &trace_mermaid);

    // The two sources agree on fired edges but only the trace source knows
    // evaluated-but-not-fired (D-21) -- the exact reason two goldens exist
    // for the SAME scripted run.
    assert_eq!(waypoint_overlay.fired_edges, trace_overlay.fired_edges);
    assert!(waypoint_overlay.evaluated_edges.is_empty());
    assert!(!trace_overlay.evaluated_edges.is_empty());
}
