//! The executable graph a [`crate::engine::WarEngine`] runs.
//!
//! [`WarGraph`] is deliberately NOT the acyclic, dependency-order-validated
//! graph `campaign_service.rs` builds: a `WarGraph` permits cycles,
//! including self-loops (ENG-FR-02). `WarGraph::validate` never calls a
//! graph-library cycle-rejection helper and never wraps the graph in an
//! adjacency-graph type whose convenience validators would reach it
//! indirectly — validation is built over the plain node map and edge vector
//! instead.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName,
};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{GraphFingerprint, NodeId};

use crate::engine::EngineError;
use crate::engine::input_mapping::InputMapping;
use crate::engine::node::StateNode;

/// One node in a [`WarGraph`].
///
/// Declared `#[non_exhaustive]`: Doc 02 adds a `Battalion` (subgraph) variant
/// without this being a breaking change.
#[non_exhaustive]
pub enum NodeSpec {
    /// A node backed by an existing string-in/string-out `Paladin`, bridged
    /// into typed state via `input_template` and `output_field` (X-03).
    Paladin {
        /// The Paladin this node wraps. Boxed: `Paladin` is a large value
        /// type (a `Node<PaladinData>`), and boxing it keeps `NodeSpec`
        /// itself small regardless of how many `Function` nodes a graph
        /// declares alongside it.
        paladin: Box<Paladin>,
        /// Renders the Paladin's string input from the Battlefield.
        input_template: InputMapping,
        /// The field the Paladin's output is written to as a delta.
        output_field: FieldName,
    },
    /// A pure, deterministic state -> delta node.
    Function(Arc<dyn StateNode>),
}

impl std::fmt::Debug for NodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeSpec::Paladin { output_field, .. } => f
                .debug_struct("NodeSpec::Paladin")
                .field("output_field", output_field)
                .finish(),
            NodeSpec::Function(_) => f.debug_tuple("NodeSpec::Function").field(&"<fn>").finish(),
        }
    }
}

/// A static edge between two nodes in a [`WarGraph`]. Dynamic routing (Doc
/// 02) is out of scope for this phase; edges here are declared up front.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EdgeSpec {
    /// The source node.
    pub from: NodeId,
    /// The target node.
    pub to: NodeId,
    /// An optional condition gating traversal, reusing the existing
    /// Campaign `EdgeCondition` vocabulary.
    pub condition: Option<EdgeCondition>,
}

/// Bounds on a `WarGraph` run, required so cyclic graphs (ENG-FR-02) always
/// terminate.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLimits {
    /// Maximum number of supersteps before the run fails with
    /// `EngineError::RecursionLimitExceeded`. Must be `>= 1`.
    pub max_supersteps: u64,
    /// Maximum number of times any single node may execute within one run,
    /// before `EngineError::NodeVisitLimitExceeded`. Must be `>= 1`.
    pub max_node_visits: u32,
    /// Optional wall-clock timeout for the whole run. Carried and validated
    /// but not acted on this phase — Doc 04 owns timeout semantics.
    pub run_timeout: Option<Duration>,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_supersteps: 50,
            max_node_visits: 25,
            run_timeout: None,
        }
    }
}

/// The executable graph a [`crate::engine::WarEngine`] runs.
///
/// Deliberately does **not** reject cycles (ENG-FR-02): unlike Campaign's
/// cycle-rejecting graph-order validation, `WarGraph::validate` only
/// enforces that both `EngineLimits` are non-zero, that every edge and entry
/// endpoint names a declared node, and that every `DispatchRule::Custom`
/// name in the schema has a registered resolver — so iterative workflows
/// (retry-and-refine, evaluate-optimize loops) can be expressed here even
/// though Campaign cannot express them.
pub struct WarGraph {
    nodes: HashMap<NodeId, NodeSpec>,
    /// Node ids in registration order (ENG-FR-04): a `HashMap`'s own
    /// iteration order is randomized per process, so any iteration over
    /// "all nodes" that must be deterministic (defer tie-breaking, the
    /// dead-frontier fixpoint) walks this `Vec` instead of `nodes` directly.
    node_order: Vec<NodeId>,
    /// Node ids registered via [`WarGraph::add_deferred_node`] (ENG-FR-06).
    defer_flags: HashSet<NodeId>,
    /// Node ids marked via [`WarGraph::mark_dynamic_target`] (ENG-FR-02a).
    /// TODO(22-15 Task 2): full rustdoc and eligible-set wiring land with
    /// the `validate` implementation; the field and its accessors exist
    /// here only so Task 1's regression tests compile against the real API
    /// shape.
    dynamic_targets: HashSet<NodeId>,
    edges: Vec<EdgeSpec>,
    schema: BattlefieldSchema,
    entry: Vec<NodeId>,
    limits: EngineLimits,
}

impl WarGraph {
    /// Construct an empty `WarGraph` with the given schema and limits.
    pub fn new(schema: BattlefieldSchema, limits: EngineLimits) -> Self {
        Self {
            nodes: HashMap::new(),
            node_order: Vec::new(),
            defer_flags: HashSet::new(),
            dynamic_targets: HashSet::new(),
            edges: Vec::new(),
            schema,
            entry: Vec::new(),
            limits,
        }
    }

    /// Register a node under `id`.
    pub fn add_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        if !self.nodes.contains_key(&id) {
            self.node_order.push(id.clone());
        }
        self.nodes.insert(id, spec);
        self
    }

    /// Register a node under `id`, marked `defer` (ENG-FR-06): an otherwise
    /// executable `defer` node is held back from the computed Vanguard
    /// until it contains no non-deferred executable node, giving
    /// aggregate-after-all-branches semantics. Multiple deferred nodes
    /// released in the same superstep are ordered by this graph's node
    /// registration order (`node_order`), never by `HashMap` order.
    pub fn add_deferred_node(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.add_node(id.clone(), spec);
        self.defer_flags.insert(id);
        self
    }

    /// Whether `id` was registered via [`WarGraph::add_deferred_node`].
    pub fn is_deferred(&self, id: &NodeId) -> bool {
        self.defer_flags.contains(id)
    }

    /// Mark `id` as a dynamic target (ENG-FR-02a). TODO(22-15 Task 2): full
    /// rustdoc lands with the `validate` implementation.
    pub fn mark_dynamic_target(&mut self, id: NodeId) -> &mut Self {
        self.dynamic_targets.insert(id);
        self
    }

    /// Whether `id` was marked via [`WarGraph::mark_dynamic_target`].
    pub fn is_dynamic_target(&self, id: &NodeId) -> bool {
        self.dynamic_targets.contains(id)
    }

    /// This graph's node ids in registration order (ENG-FR-04).
    pub fn node_order(&self) -> &[NodeId] {
        &self.node_order
    }

    /// Add a static edge.
    pub fn add_edge(&mut self, edge: EdgeSpec) -> &mut Self {
        self.edges.push(edge);
        self
    }

    /// Mark `id` as an entry-point node (part of superstep 1's Vanguard).
    pub fn add_entry(&mut self, id: NodeId) -> &mut Self {
        self.entry.push(id);
        self
    }

    /// The graph's Battlefield schema.
    pub fn schema(&self) -> &BattlefieldSchema {
        &self.schema
    }

    /// The graph's entry-point nodes, in registration order.
    pub fn entry(&self) -> &[NodeId] {
        &self.entry
    }

    /// The graph's declared edges, in insertion order.
    pub fn edges(&self) -> &[EdgeSpec] {
        &self.edges
    }

    /// The graph's execution limits.
    pub fn limits(&self) -> &EngineLimits {
        &self.limits
    }

    /// Look up a node's declaration by id.
    pub fn node(&self, id: &NodeId) -> Option<&NodeSpec> {
        self.nodes.get(id)
    }

    /// Validate structural invariants. Does NOT reject cycles (ENG-FR-02):
    /// rejects a graph whose limits could never terminate, an edge or entry
    /// point naming an undeclared node, or a schema `Custom` dispatch name
    /// with no resolver registered in `custom_dispatch` (ENG-FR-09) — the
    /// last check runs before any node executes.
    pub fn validate(&self, custom_dispatch: &CustomDispatchResolver) -> Result<(), EngineError> {
        if self.limits.max_supersteps == 0 {
            return Err(EngineError::InvalidLimits {
                reason: "max_supersteps must be at least 1".to_string(),
            });
        }
        if self.limits.max_node_visits == 0 {
            return Err(EngineError::InvalidLimits {
                reason: "max_node_visits must be at least 1".to_string(),
            });
        }

        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) {
                return Err(EngineError::UnknownNode(edge.from.clone()));
            }
            if !self.nodes.contains_key(&edge.to) {
                return Err(EngineError::UnknownNode(edge.to.clone()));
            }
        }

        for entry in &self.entry {
            if !self.nodes.contains_key(entry) {
                return Err(EngineError::UnknownNode(entry.clone()));
            }
        }

        for field in &self.schema.fields {
            if let DispatchRule::Custom(name) = &field.dispatch
                && !custom_dispatch.contains_key(name)
            {
                return Err(EngineError::Battlefield(
                    BattlefieldError::CustomDispatchNotRegistered { name: name.clone() },
                ));
            }
        }

        Ok(())
    }

    /// Compute this graph's stable content fingerprint (ENG-FR-14): a hash
    /// over node ids, edge specs and schema field names — deliberately NOT
    /// over prompts or models. Node ids, edges (by `from`/`to`) and schema
    /// field names are sorted before hashing so the result never depends on
    /// `HashMap` iteration order (RESEARCH.md Pitfall 5).
    pub fn fingerprint(&self) -> GraphFingerprint {
        let mut node_ids: Vec<&NodeId> = self.nodes.keys().collect();
        node_ids.sort();

        let mut edges: Vec<&EdgeSpec> = self.edges.iter().collect();
        edges.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str()))
        });

        let mut field_names: Vec<&FieldName> = self.schema.fields.iter().map(|f| &f.name).collect();
        field_names.sort();

        let mut buf = Vec::new();
        buf.extend_from_slice(b"nodes:");
        for id in &node_ids {
            buf.extend_from_slice(id.as_str().as_bytes());
            buf.push(b'|');
        }
        buf.extend_from_slice(b";edges:");
        for edge in &edges {
            buf.extend_from_slice(edge.from.as_str().as_bytes());
            buf.push(b'-');
            buf.extend_from_slice(edge.to.as_str().as_bytes());
            buf.push(b':');
            buf.extend_from_slice(format!("{:?}", edge.condition).as_bytes());
            buf.push(b'|');
        }
        buf.extend_from_slice(b";schema:");
        for name in &field_names {
            buf.extend_from_slice(name.as_str().as_bytes());
            buf.push(b'|');
        }

        GraphFingerprint::from_canonical_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{Battlefield, FieldSpec};
    use std::sync::Arc as StdArc;

    use crate::engine::RunOutcome;

    fn one_field_schema() -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("result").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )])
    }

    struct NoopNode;

    #[async_trait::async_trait]
    impl StateNode for NoopNode {
        async fn run(
            &self,
            _state: &paladin_core::platform::container::battlefield::Battlefield,
            _ctx: &crate::engine::node::NodeContext,
        ) -> Result<
            paladin_core::platform::container::battlefield::StateDelta,
            crate::engine::node::NodeError,
        > {
            Ok(paladin_core::platform::container::battlefield::StateDelta::new())
        }
    }

    #[test]
    fn validate_accepts_two_node_cycle() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("b"),
            to: NodeId::new("a"),
            condition: None,
        });
        graph.add_entry(NodeId::new("a"));

        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());
    }

    #[test]
    fn validate_accepts_self_loop() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("a"),
            condition: None,
        });
        graph.add_entry(NodeId::new("a"));

        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());
    }

    #[test]
    fn validate_rejects_zero_max_supersteps() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_supersteps: 0,
                ..EngineLimits::default()
            },
        );
        assert!(matches!(
            graph.validate(&CustomDispatchResolver::new()),
            Err(EngineError::InvalidLimits { .. })
        ));
    }

    #[test]
    fn validate_rejects_zero_max_node_visits() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_node_visits: 0,
                ..EngineLimits::default()
            },
        );
        assert!(matches!(
            graph.validate(&CustomDispatchResolver::new()),
            Err(EngineError::InvalidLimits { .. })
        ));
    }

    #[test]
    fn validate_accepts_max_supersteps_of_one() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_supersteps: 1,
                ..EngineLimits::default()
            },
        );
        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());
    }

    #[test]
    fn validate_rejects_edge_with_unknown_from() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("ghost"),
            to: NodeId::new("b"),
            condition: None,
        });
        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        assert!(matches!(err, EngineError::UnknownNode(id) if id == NodeId::new("ghost")));
    }

    #[test]
    fn validate_rejects_edge_with_unknown_to() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("ghost"),
            condition: None,
        });
        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        assert!(matches!(err, EngineError::UnknownNode(id) if id == NodeId::new("ghost")));
    }

    #[test]
    fn validate_rejects_unknown_entry() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_entry(NodeId::new("ghost"));
        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        assert!(matches!(err, EngineError::UnknownNode(id) if id == NodeId::new("ghost")));
    }

    #[test]
    fn validate_rejects_unregistered_custom_dispatch() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("special").unwrap(),
            DispatchRule::Custom("merge_scores".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_entry(NodeId::new("a"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::Battlefield(BattlefieldError::CustomDispatchNotRegistered { name }) => {
                assert_eq!(name, "merge_scores");
            }
            other => panic!("expected CustomDispatchNotRegistered, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_registered_custom_dispatch() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("special").unwrap(),
            DispatchRule::Custom("merge_scores".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_entry(NodeId::new("a"));

        let mut registry = CustomDispatchResolver::new();
        registry.insert(
            "merge_scores".to_string(),
            StdArc::new(|_c: &serde_json::Value, d: &serde_json::Value| Ok(d.clone())),
        );

        assert!(graph.validate(&registry).is_ok());
    }

    #[test]
    fn fingerprint_is_deterministic_across_calls() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("solo"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("solo"));

        let a = graph.fingerprint();
        let b = graph.fingerprint();
        assert_eq!(a, b);
        assert_eq!(
            a.as_str(),
            "v1:f5532b613066cb2d1972451bad73120abafbf7cbafd8ecf572a043448c31d2d6"
        );
    }

    #[test]
    fn fingerprint_is_unchanged_by_insertion_order() {
        let mut graph_a = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph_a.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph_a.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph_a.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: None,
        });
        graph_a.add_entry(NodeId::new("a"));

        let mut graph_b = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph_b.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph_b.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph_b.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: None,
        });
        graph_b.add_entry(NodeId::new("a"));

        assert_eq!(graph_a.fingerprint(), graph_b.fingerprint());
    }

    #[test]
    fn engine_limits_default_is_50_and_25() {
        let limits = EngineLimits::default();
        assert_eq!(limits.max_supersteps, 50);
        assert_eq!(limits.max_node_visits, 25);
    }

    // --- BUG-02 / ENG-FR-02a: eligible-set reachability regression tests
    // (Phase 22 Plan 15, gap G-22-3). These fail before the fix lands in
    // Task 2 -- see 22-15-SUMMARY.md for the pre-fix evidence capture that
    // proves the defect (validate() accepts a stranded self-loop-only node,
    // and a run over it reports `Completed` with that node's run_count() at
    // 0).

    use crate::engine::hooks::TraceDispatcher;
    use crate::engine::test_support::{
        CountingFunctionNode, RecordingPaladinPort, RecordingWaypointStore,
    };
    use crate::engine::{WaypointDurability};
    use paladin_core::platform::container::battlefield::StateDelta;
    use paladin_core::platform::container::waypoint::ThreadId;

    /// Run `graph` to completion through the real superstep loop (Function
    /// nodes only -- no `PaladinPort` calls are configured), the same way
    /// `engine::superstep::tests::run_default` does, so a "reachable
    /// variant runs" assertion exercises the real engine rather than only
    /// `validate`.
    async fn run_to_completion(graph: &WarGraph) -> RunOutcome {
        let store = RecordingWaypointStore::new();
        let paladin_port: StdArc<dyn paladin_ports::output::paladin_port::PaladinPort> =
            StdArc::new(RecordingPaladinPort::new());
        let trace = StdArc::new(TraceDispatcher::new(None));
        let interceptors: Vec<StdArc<dyn crate::engine::hooks::NodeInterceptor>> = Vec::new();

        crate::engine::superstep::run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            graph,
            ThreadId::new("reachability-regression").unwrap(),
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
            graph.entry().to_vec(),
            std::collections::BTreeMap::new(),
            None,
            1,
            &paladin_port,
            &trace,
            &interceptors,
            &None,
        )
        .await
        .unwrap()
    }

    #[test]
    fn validate_rejects_self_loop_only_stranded_node_naming_it() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(
            NodeId::new("stranded"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("stranded"),
            to: NodeId::new("stranded"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::UnreachableNode { nodes, reason } => {
                assert_eq!(nodes, vec![NodeId::new("stranded")]);
                assert!(
                    reason.contains("stranded"),
                    "reason should name the offending node: {reason}"
                );
            }
            other => panic!("expected UnreachableNode, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_multiple_stranded_nodes_in_one_error_registration_order() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("c"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("d"), NodeSpec::Function(StdArc::new(NoopNode)));
        for id in ["b", "c", "d"] {
            graph.add_edge(EdgeSpec {
                from: NodeId::new(id),
                to: NodeId::new(id),
                condition: None,
            });
        }
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::UnreachableNode { nodes, reason } => {
                assert_eq!(
                    nodes,
                    vec![NodeId::new("b"), NodeId::new("c"), NodeId::new("d")],
                    "all three offenders must be reported together, in registration order"
                );
                for id in ["b", "c", "d"] {
                    assert!(reason.contains(id), "reason should name {id}: {reason}");
                }
            }
            other => panic!("expected UnreachableNode, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn validate_accepts_and_runs_stranded_node_once_made_reachable_from_entry() {
        // Deliberately no self-loop on "stranded" here (unlike the
        // rejection test's fixture): a node whose incoming edges are BOTH
        // a self-loop and an external edge can never resolve its own
        // self-loop edge's Pending state before it first runs (ENG-FR-06's
        // join semantics require every incoming edge to resolve, and a
        // self-loop's source is the node itself) -- an unrelated engine
        // property, not what this check is pinning. This fixture isolates
        // "made reachable from entry" cleanly: an otherwise-edgeless node,
        // exactly the kind the isolated-node rejection test above also
        // uses, now wired to entry.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let entry = CountingFunctionNode::fixed(
            FieldName::new("result").unwrap(),
            serde_json::json!("entry-ran"),
        );
        let formerly_stranded = CountingFunctionNode::fixed(
            FieldName::new("result").unwrap(),
            serde_json::json!("stranded-ran"),
        );
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(entry));
        graph.add_node(
            NodeId::new("stranded"),
            NodeSpec::Function(formerly_stranded.clone()),
        );
        // The fix: an edge from entry making "stranded" reachable.
        graph.add_edge(EdgeSpec {
            from: NodeId::new("entry"),
            to: NodeId::new("stranded"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());

        let outcome = run_to_completion(&graph).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            formerly_stranded.run_count(),
            1,
            "the formerly-stranded node must actually execute once reachable"
        );
    }

    #[tokio::test]
    async fn validate_accepts_and_runs_stranded_node_once_marked_dynamic_target() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let entry = CountingFunctionNode::fixed(
            FieldName::new("result").unwrap(),
            serde_json::json!("entry-ran"),
        );
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(entry));
        graph.add_node(
            NodeId::new("jump-target"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("entry"));
        graph.mark_dynamic_target(NodeId::new("jump-target"));

        // No edge at all into "jump-target" -- the marker alone is enough.
        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());

        let outcome = run_to_completion(&graph).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    #[tokio::test]
    async fn self_loop_on_entry_node_still_validates_and_runs() {
        let field_name = FieldName::new("status").unwrap();
        let node = CountingFunctionNode::new(move |run_index, _state| {
            let status = if run_index == 1 { "approved" } else { "looping" };
            let mut delta = StateDelta::new();
            delta.set(field_name.clone(), status).unwrap();
            delta
        });
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("status").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(node.clone()));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("a"),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(NodeId::new("a"));

        assert!(
            graph.validate(&CustomDispatchResolver::new()).is_ok(),
            "self-loops remain legal on entry nodes -- the check rejects strandedness, not loops"
        );

        let outcome = run_to_completion(&graph).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 2);
    }

    #[test]
    fn validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("b"),
            to: NodeId::new("b"),
            condition: None,
        });
        graph.add_entry(NodeId::new("a"));

        assert!(graph.validate(&CustomDispatchResolver::new()).is_ok());
    }

    #[test]
    fn validate_rejects_graph_with_no_entry_point_naming_absent_entry() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        // No add_entry call at all.

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::UnreachableNode { nodes, reason } => {
                assert_eq!(nodes, vec![NodeId::new("a")]);
                assert!(
                    reason.contains("entry"),
                    "reason should name the absent entry point as the cause, distinguishing \
                     this from the ordinary stranded case: {reason}"
                );
            }
            other => panic!("expected UnreachableNode, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_isolated_node_with_no_edges_when_graph_has_entry() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(
            NodeId::new("isolated"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::UnreachableNode { nodes, .. } => {
                assert_eq!(nodes, vec![NodeId::new("isolated")]);
            }
            other => panic!("expected UnreachableNode, got {other:?}"),
        }
    }

    #[test]
    fn validate_prefers_limit_error_over_unreachable_node() {
        let mut graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_supersteps: 0,
                ..EngineLimits::default()
            },
        );
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(
            NodeId::new("stranded"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("stranded"),
            to: NodeId::new("stranded"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        assert!(matches!(
            graph.validate(&CustomDispatchResolver::new()),
            Err(EngineError::InvalidLimits { .. })
        ));
    }

    #[test]
    fn validate_prefers_unknown_node_error_over_unreachable_node() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(
            NodeId::new("stranded"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("stranded"),
            to: NodeId::new("stranded"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("entry"),
            to: NodeId::new("ghost"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        assert!(matches!(err, EngineError::UnknownNode(id) if id == NodeId::new("ghost")));
    }

    #[test]
    fn validate_prefers_custom_dispatch_error_over_unreachable_node() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("special").unwrap(),
            DispatchRule::Custom("merge_scores".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(NodeId::new("entry"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(
            NodeId::new("stranded"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("stranded"),
            to: NodeId::new("stranded"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        match err {
            EngineError::Battlefield(BattlefieldError::CustomDispatchNotRegistered { name }) => {
                assert_eq!(name, "merge_scores");
            }
            other => panic!("expected CustomDispatchNotRegistered, got {other:?}"),
        }
    }
}
