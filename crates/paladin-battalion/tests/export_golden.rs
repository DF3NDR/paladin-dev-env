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

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_battalion::engine::export::{GraphShape, to_dot, to_mermaid};
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
use paladin_core::platform::container::waypoint::NodeId;

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
