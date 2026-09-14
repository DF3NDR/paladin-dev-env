//! `GraphShape` -- the rendering-agnostic shape both a [`WarGraphDoc`] and a
//! compiled [`WarGraph`] can produce (D-18): the shared contract
//! [`super::to_mermaid`] and [`super::to_dot`] render, and the future
//! execution overlay (28-10) and inspector (28-15) build on.
//!
//! # Two sources, one shape (D-18)
//!
//! 27-CONTEXT D-33 scoped `WarGraphDoc` to exactly `{Paladin, Gate,
//! Workflow}` -- no `Function` nodes, no worker templates. A doc-only
//! exporter could therefore never render a `Function` node or a Muster
//! worker template, which the "muster" golden fixture and every code-built
//! assistant need. `GraphShape::from_graph` reads the compiled [`WarGraph`]
//! directly, so it sees everything: [`GraphShape::from_doc`] stays the fast,
//! registries-free path for a document that has not been (or cannot be)
//! compiled.
//!
//! # Security (T-28-07-01, T-28-07-02)
//!
//! A `GraphShape` carries only ids, kinds, flags and condition KINDS --
//! never a Paladin's `system_prompt`, an `input_template`, or any
//! Battlefield state value. Node ids and condition text do flow into
//! rendered diagram markup ([`super::to_mermaid`]/[`super::to_dot`]);
//! sanitizing the DIAGRAM's own identifiers (never the label content
//! itself) is those exporters' responsibility.

use std::collections::BTreeSet;

use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::waypoint::NodeId;

use crate::engine::export::overlay::ExecutionOverlay;
use crate::engine::graph::{NodeSpec, WarGraph};
use crate::engine::graph_doc::{EdgeConditionDoc, NodeKindDoc, WarGraphDoc};

/// The rendering-agnostic shape of a graph (D-18): every node's kind and
/// flags, every edge and its condition kind, and the entry points -- built
/// from EITHER a [`WarGraphDoc`] ([`GraphShape::from_doc`]) or a compiled
/// [`WarGraph`] ([`GraphShape::from_graph`]). For a doc-expressible graph,
/// both constructors produce an equal `GraphShape` (asserted in
/// `tests/export_golden.rs`'s `doc_and_graph_shapes_agree`).
///
/// # Examples
///
/// ```
/// use paladin_battalion::engine::export::GraphShape;
/// use paladin_battalion::engine::graph::{EngineLimits, WarGraph};
/// use paladin_core::platform::container::battlefield::BattlefieldSchema;
///
/// let graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
/// let shape = GraphShape::from_graph(&graph);
/// assert!(shape.nodes.is_empty());
/// assert!(shape.edges.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphShape {
    /// Every node in this graph, in declaration order.
    pub nodes: Vec<ShapeNode>,
    /// Every edge in this graph, in declaration order.
    pub edges: Vec<ShapeEdge>,
    /// This graph's entry-point node ids, in declaration order.
    pub entry: Vec<NodeId>,
}

/// One node's rendering-relevant shape (D-18). `deferred` and
/// `worker_template` are two INDEPENDENT booleans -- never `ShapeKind`
/// variants -- read from [`WarGraph::is_deferred`]/[`WarGraph::is_worker_template`]
/// on the [`GraphShape::from_graph`] path, or from `NodeDoc.defer` (with
/// `worker_template` always `false`, since a document cannot express one) on
/// the [`GraphShape::from_doc`] path.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeNode {
    /// This node's id.
    pub id: NodeId,
    /// This node's rendered kind -- overridden to
    /// [`ShapeKind::WorkerTemplate`] when `worker_template` is `true`,
    /// regardless of the underlying `NodeSpec` variant, so the diagram's own
    /// badge always reads `«worker»` for a Muster worker template (D-19,
    /// 28-UI-SPEC.md).
    pub kind: ShapeKind,
    /// Whether this node was registered via [`WarGraph::add_deferred_node`]
    /// (always `false` on the `from_doc` path unless `NodeDoc.defer` is
    /// set).
    pub deferred: bool,
    /// Whether this node was registered via [`WarGraph::add_worker_template`]
    /// (always `false` on the `from_doc` path -- worker templates are not
    /// expressible in a `WarGraphDoc`).
    pub worker_template: bool,
    /// The nested shape of a `Workflow` node's child graph, or `None` for
    /// every other kind.
    pub subgraph: Option<Box<GraphShape>>,
}

/// One edge's rendering-relevant shape (D-18).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeEdge {
    /// The source node id.
    pub from: NodeId,
    /// The target node id.
    pub to: NodeId,
    /// The condition's rendered label, or `None` for an unconditional
    /// (`Always`) edge -- `Some("contains(\"value\")")`,
    /// `Some("regex")`, or `Some("custom(name)")` (D-19).
    pub condition: Option<String>,
}

/// A node's kind, exactly the five values a diagram badges (D-18, D-19,
/// 28-UI-SPEC.md): `«paladin»`, `«function»`, `«gate»`, `«workflow»`,
/// `«worker»`. `Function` and `WorkerTemplate` are reachable only from
/// [`GraphShape::from_graph`] -- a document cannot express either (27-CONTEXT
/// D-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// A `NodeSpec::Paladin` node (or a `paladin` document node).
    Paladin,
    /// A `NodeSpec::Function` node -- code-only, never doc-expressible.
    Function,
    /// A `NodeSpec::Gate` node (or a `gate` document node).
    Gate,
    /// A `NodeSpec::Battalion` node (or a `workflow` document node),
    /// rendered as a nested cluster over `ShapeNode::subgraph`.
    Workflow,
    /// A node registered via [`WarGraph::add_worker_template`] -- code-only,
    /// never doc-expressible. Overrides whichever `NodeSpec` variant the
    /// template wraps (D-18).
    WorkerTemplate,
}

impl GraphShape {
    /// Build a `GraphShape` from a [`WarGraphDoc`] (D-18): reads
    /// `NodeKindDoc` for each node's kind and `NodeDoc.defer` for
    /// `deferred`; `worker_template` is always `false` -- a document cannot
    /// express one (27-CONTEXT D-33). A node whose `kind`/body do not
    /// resolve (a document that would fail [`WarGraphDoc::compile`]) is
    /// rendered as [`ShapeKind::Paladin`] rather than panicking: `from_doc`
    /// is meant to work on an ALREADY-validated document (its caller should
    /// have called `compile` first, per D-22's CLI resolution order), so
    /// this fallback is never exercised by a compilable document.
    pub fn from_doc(doc: &WarGraphDoc) -> Self {
        let nodes = doc
            .nodes
            .iter()
            .map(|node| {
                let (kind, subgraph) = match node.kind_doc() {
                    Ok(NodeKindDoc::Paladin(_)) => (ShapeKind::Paladin, None),
                    Ok(NodeKindDoc::Gate(_)) => (ShapeKind::Gate, None),
                    Ok(NodeKindDoc::Workflow(workflow)) => (
                        ShapeKind::Workflow,
                        Some(Box::new(GraphShape::from_doc(&workflow.graph))),
                    ),
                    Err(_) => (ShapeKind::Paladin, None),
                };
                ShapeNode {
                    id: NodeId::new(node.id.clone()),
                    kind,
                    deferred: node.defer,
                    worker_template: false,
                    subgraph,
                }
            })
            .collect();

        let edges = doc
            .edges
            .iter()
            .map(|edge| ShapeEdge {
                from: NodeId::new(edge.from.clone()),
                to: NodeId::new(edge.to.clone()),
                condition: condition_label_doc(&edge.condition),
            })
            .collect();

        let entry = doc.entry.iter().map(|id| NodeId::new(id.clone())).collect();

        GraphShape {
            nodes,
            edges,
            entry,
        }
    }

    /// Build a `GraphShape` from a compiled [`WarGraph`] (D-18): walks
    /// `WarGraph::node_order` (never `HashMap` iteration, ENG-FR-04),
    /// reading `NodeSpec` for each node's kind -- mapping `Battalion` to
    /// [`ShapeKind::Workflow`] and recursing into the nested graph for
    /// `subgraph` -- and calling [`WarGraph::is_worker_template`] and
    /// [`WarGraph::is_deferred`] as two independent lookups.
    pub fn from_graph(graph: &WarGraph) -> Self {
        let nodes = graph
            .node_order()
            .iter()
            .map(|id| {
                let is_worker_template = graph.is_worker_template(id);
                let is_deferred = graph.is_deferred(id);
                let (base_kind, subgraph) = match graph.node(id) {
                    Some(NodeSpec::Paladin { .. }) => (ShapeKind::Paladin, None),
                    Some(NodeSpec::Function(_)) => (ShapeKind::Function, None),
                    Some(NodeSpec::Gate { .. }) => (ShapeKind::Gate, None),
                    Some(NodeSpec::Battalion { graph: child, .. }) => (
                        ShapeKind::Workflow,
                        Some(Box::new(GraphShape::from_graph(child))),
                    ),
                    // Every id in `node_order` was inserted alongside its
                    // spec by `WarGraph::add_node` -- this arm is
                    // unreachable in practice, and `Paladin` is a harmless,
                    // panic-free default for it.
                    None => (ShapeKind::Paladin, None),
                };
                let kind = if is_worker_template {
                    ShapeKind::WorkerTemplate
                } else {
                    base_kind
                };
                ShapeNode {
                    id: id.clone(),
                    kind,
                    deferred: is_deferred,
                    worker_template: is_worker_template,
                    subgraph,
                }
            })
            .collect();

        let edges = graph
            .edges()
            .iter()
            .map(|edge| ShapeEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                condition: condition_label(&edge.condition),
            })
            .collect();

        let entry = graph.entry().to_vec();

        GraphShape {
            nodes,
            edges,
            entry,
        }
    }

    /// Build the observed-only fallback shape (D-22): when no static graph
    /// document is available for a thread, this reconstructs a `GraphShape`
    /// from only what `overlay` actually saw -- the visited node ids and the
    /// union of its `fired_edges`/`evaluated_edges`, with no entry points
    /// (an observed subgraph has no declared entry, only what happened to
    /// run first).
    ///
    /// Every observed node's kind is rendered as [`ShapeKind::Paladin`] --
    /// the true kind is unknowable from a Waypoint/trace alone (a Waypoint
    /// carries only a `graph_fingerprint`, D-22), and `Paladin` is this
    /// module's existing least-assuming badge (Claude's Discretion, no
    /// dedicated "unknown" `ShapeKind` variant -- adding one would force a
    /// matching change in `dot.rs`, which this plan (D-21's Mermaid-only
    /// overlay) deliberately does not touch). Callers rendering this shape
    /// should set `ExecutionOverlay::observed_only = true` so
    /// [`super::mermaid::to_mermaid_overlay`] carries the locked
    /// observed-only title (D-22, 28-UI-SPEC.md).
    pub fn observed(overlay: &ExecutionOverlay) -> Self {
        let mut node_ids: BTreeSet<NodeId> = overlay.visits.keys().cloned().collect();
        for (from, to) in overlay
            .fired_edges
            .iter()
            .chain(overlay.evaluated_edges.iter())
        {
            node_ids.insert(from.clone());
            node_ids.insert(to.clone());
        }

        let nodes = node_ids
            .into_iter()
            .map(|id| ShapeNode {
                id,
                kind: ShapeKind::Paladin,
                deferred: false,
                worker_template: false,
                subgraph: None,
            })
            .collect();

        let mut edge_pairs: BTreeSet<(NodeId, NodeId)> = overlay.fired_edges.clone();
        edge_pairs.extend(overlay.evaluated_edges.iter().cloned());
        let edges = edge_pairs
            .into_iter()
            .map(|(from, to)| ShapeEdge {
                from,
                to,
                condition: None,
            })
            .collect();

        GraphShape {
            nodes,
            edges,
            entry: Vec::new(),
        }
    }
}

/// Renders an [`EdgeCondition`] to the label text D-19 fixes for the golden
/// files: `Always` (or no condition) is unlabeled, `Contains` shows its
/// match value, `Regex` renders bare (the pattern itself is template-shaped
/// content this module does not carry, T-28-07-02), `Custom` shows the
/// registered evaluator's name.
fn condition_label(condition: &Option<EdgeCondition>) -> Option<String> {
    match condition {
        None | Some(EdgeCondition::Always) => None,
        Some(EdgeCondition::Contains(value)) => Some(format!("contains(\"{value}\")")),
        Some(EdgeCondition::Regex(_)) => Some("regex".to_string()),
        Some(EdgeCondition::Custom(name)) => Some(format!("custom({name})")),
    }
}

/// The `WarGraphDoc` mirror of [`condition_label`] -- produces the SAME
/// label text for the same logical condition, so `from_doc` and
/// `from_graph` agree on a doc-expressible fixture.
fn condition_label_doc(condition: &Option<EdgeConditionDoc>) -> Option<String> {
    match condition {
        None | Some(EdgeConditionDoc::Always) => None,
        Some(EdgeConditionDoc::Contains { value }) => Some(format!("contains(\"{value}\")")),
        Some(EdgeConditionDoc::Regex { .. }) => Some("regex".to_string()),
        Some(EdgeConditionDoc::Custom { name }) => Some(format!("custom({name})")),
    }
}

#[cfg(test)]
mod tests {
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::paladin::{
        MaxLoops, Paladin, PaladinData, PaladinStatus,
    };

    use super::*;
    use crate::engine::graph::{EdgeSpec, EngineLimits, NodeSpec};
    use crate::engine::graph_doc::{
        EdgeConditionDoc, EdgeDoc, LimitsDoc, NodeDoc, PaladinNodeDoc, SchemaDoc,
        WARGRAPH_DOC_SCHEMA_VERSION, WarGraphDoc,
    };
    use crate::engine::input_mapping::InputMapping;
    use crate::engine::registries::EngineRegistries;

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

    fn linear_doc() -> WarGraphDoc {
        WarGraphDoc {
            schema_version: WARGRAPH_DOC_SCHEMA_VERSION.to_string(),
            entry: vec!["a".to_string()],
            nodes: vec![
                NodeDoc {
                    id: "a".to_string(),
                    kind: "paladin".to_string(),
                    paladin: Some(PaladinNodeDoc {
                        name: "A".to_string(),
                        model: "gpt-4".to_string(),
                        system_prompt: "step a: {topic}".to_string(),
                        temperature: None,
                        max_loops: None,
                        stop_words: vec![],
                        input_template: "{topic}".to_string(),
                        output_field: "out_a".to_string(),
                        output_schema: None,
                    }),
                    gate: None,
                    workflow: None,
                    aegis: None,
                    defer: false,
                },
                NodeDoc {
                    id: "b".to_string(),
                    kind: "paladin".to_string(),
                    paladin: Some(PaladinNodeDoc {
                        name: "B".to_string(),
                        model: "gpt-4".to_string(),
                        system_prompt: "step b: {out_a}".to_string(),
                        temperature: None,
                        max_loops: None,
                        stop_words: vec![],
                        input_template: "{out_a}".to_string(),
                        output_field: "out_b".to_string(),
                        output_schema: None,
                    }),
                    gate: None,
                    workflow: None,
                    aegis: None,
                    defer: false,
                },
                NodeDoc {
                    id: "c".to_string(),
                    kind: "paladin".to_string(),
                    paladin: Some(PaladinNodeDoc {
                        name: "C".to_string(),
                        model: "gpt-4".to_string(),
                        system_prompt: "step c: {out_b}".to_string(),
                        temperature: None,
                        max_loops: None,
                        stop_words: vec![],
                        input_template: "{out_b}".to_string(),
                        output_field: "out_c".to_string(),
                        output_schema: None,
                    }),
                    gate: None,
                    workflow: None,
                    aegis: None,
                    defer: false,
                },
            ],
            edges: vec![
                EdgeDoc {
                    from: "a".to_string(),
                    to: "b".to_string(),
                    condition: None,
                },
                EdgeDoc {
                    from: "b".to_string(),
                    to: "c".to_string(),
                    condition: Some(EdgeConditionDoc::Always),
                },
            ],
            schema: SchemaDoc {
                fields: vec![
                    schema_field("topic"),
                    schema_field("out_a"),
                    schema_field("out_b"),
                    schema_field("out_c"),
                ],
            },
            limits: LimitsDoc::default(),
            default_aegis: None,
        }
    }

    fn schema_field(name: &str) -> crate::engine::graph_doc::FieldDoc {
        crate::engine::graph_doc::FieldDoc {
            name: name.to_string(),
            kind: crate::engine::graph_doc::FieldKindDoc::String,
            reducer: crate::engine::graph_doc::ReducerDoc::LastWrite,
            default: None,
            required: false,
        }
    }

    /// Builds the same three-node linear graph directly in code (the
    /// `WarGraph` a `compile` of [`linear_doc`] would also produce).
    fn linear_graph_from_code() -> WarGraph {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(field("topic"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("out_a"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("out_b"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("out_c"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        graph.add_node(
            a.clone(),
            NodeSpec::paladin(
                make_paladin("a"),
                InputMapping::new("{topic}"),
                field("out_a"),
            ),
        );
        graph.add_node(
            b.clone(),
            NodeSpec::paladin(
                make_paladin("b"),
                InputMapping::new("{out_a}"),
                field("out_b"),
            ),
        );
        graph.add_node(
            c.clone(),
            NodeSpec::paladin(
                make_paladin("c"),
                InputMapping::new("{out_b}"),
                field("out_c"),
            ),
        );
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: b,
            to: c,
            condition: None,
        });
        graph.add_entry(a);
        graph
    }

    #[test]
    fn linear_doc_and_graph_produce_the_same_shape() {
        let doc = linear_doc();
        let compiled = doc
            .compile(&EngineRegistries::new())
            .expect("linear doc compiles");

        let from_doc = GraphShape::from_doc(&doc);
        let from_graph = GraphShape::from_graph(&compiled);

        assert_eq!(from_doc, from_graph);

        // Also matches an equivalent graph built directly in code -- proves
        // `from_graph` is not accidentally coupled to compile()'s own
        // internal node ordering.
        let code_graph = linear_graph_from_code();
        assert_eq!(from_graph, GraphShape::from_graph(&code_graph));
    }

    #[test]
    fn linear_mermaid_is_deterministic() {
        let shape = GraphShape::from_graph(&linear_graph_from_code());
        let rendered = super::super::mermaid::to_mermaid(&shape);

        assert!(rendered.starts_with("flowchart TD"));
        assert!(rendered.contains("n0"));
        assert!(rendered.contains("n1"));
        assert!(rendered.contains("n2"));
        assert!(rendered.contains('a'));
        assert!(rendered.contains('b'));
        assert!(rendered.contains('c'));
        // Two unlabeled (Always) edges: n0 --> n1, n1 --> n2.
        assert_eq!(rendered.matches("-->").count(), 2);

        let rendered_again = super::super::mermaid::to_mermaid(&shape);
        assert_eq!(rendered, rendered_again);
    }

    #[test]
    fn linear_dot_is_deterministic() {
        let shape = GraphShape::from_graph(&linear_graph_from_code());
        let rendered = super::super::dot::to_dot(&shape);

        assert!(rendered.contains("digraph"));
        assert!(rendered.contains("\"n0\""));
        assert!(rendered.contains("\"n1\""));
        assert!(rendered.contains("\"n2\""));
        assert_eq!(rendered.matches("->").count(), 2);

        let rendered_again = super::super::dot::to_dot(&shape);
        assert_eq!(rendered, rendered_again);
    }

    #[test]
    fn single_node_zero_edge_shape_renders() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("out"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let only = NodeId::new("only");
        graph.add_node(
            only.clone(),
            NodeSpec::paladin(make_paladin("only"), InputMapping::new("hi"), field("out")),
        );
        graph.add_entry(only);

        let shape = GraphShape::from_graph(&graph);
        assert_eq!(shape.nodes.len(), 1);
        assert!(shape.edges.is_empty());

        let mermaid = super::super::mermaid::to_mermaid(&shape);
        assert!(mermaid.starts_with("flowchart TD"));
        assert!(mermaid.contains("only"));
        assert!(!mermaid.contains("-->"));

        let dot = super::super::dot::to_dot(&shape);
        assert!(dot.contains("digraph"));
        assert!(dot.contains("only"));
        assert!(!dot.contains("->"));
    }

    #[test]
    fn worker_template_and_deferred_are_independent_flags() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field("out"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let worker = NodeId::new("worker");
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::paladin(
                make_paladin("worker"),
                InputMapping::new("hi"),
                field("out"),
            ),
        );

        let shape = GraphShape::from_graph(&graph);
        let node = &shape.nodes[0];
        assert!(node.worker_template);
        assert!(!node.deferred);
        assert_eq!(node.kind, ShapeKind::WorkerTemplate);
    }
}
