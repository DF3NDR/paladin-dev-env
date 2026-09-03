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
/// cycle-rejecting graph-order validation, `WarGraph::validate` enforces
/// that both `EngineLimits` are non-zero, that every edge and entry
/// endpoint names a declared node, that every `DispatchRule::Custom` name
/// in the schema has a registered resolver, and — ENG-FR-02a / BUG-02, the
/// last clause checked — that every declared node is in the **eligible
/// set**: reachable from `entry` over static edges, or marked
/// [`WarGraph::mark_dynamic_target`]. Iterative workflows (retry-and-refine,
/// evaluate-optimize loops) can still be expressed here even though
/// Campaign cannot express them; what is rejected is a node that could
/// NEVER become ready, not a cycle.
pub struct WarGraph {
    nodes: HashMap<NodeId, NodeSpec>,
    /// Node ids in registration order (ENG-FR-04): a `HashMap`'s own
    /// iteration order is randomized per process, so any iteration over
    /// "all nodes" that must be deterministic (defer tie-breaking, the
    /// dead-frontier fixpoint) walks this `Vec` instead of `nodes` directly.
    node_order: Vec<NodeId>,
    /// Node ids registered via [`WarGraph::add_deferred_node`] (ENG-FR-06).
    defer_flags: HashSet<NodeId>,
    /// Node ids marked via [`WarGraph::mark_dynamic_target`] (ENG-FR-02a):
    /// the declared escape hatch for a node reachable only as a runtime
    /// jump target, seeded into `validate`'s eligible-set worklist
    /// alongside `entry` so such a node is not rejected as stranded.
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

    /// Mark `id` — an already-registered node — as a **dynamic target**
    /// (ENG-FR-02a / BUG-02): the declared escape hatch for a node
    /// reachable only as the target of a runtime jump (a Goto-style
    /// dynamic route CF-FR-07 will own in a later phase), never by any
    /// statically-declared [`EdgeSpec`]. `WarGraph::validate` seeds its
    /// eligible-set worklist from `entry` UNION every `dynamic_target`, so
    /// a marked node validates and can be scheduled without any edge
    /// pointing to it.
    ///
    /// Marking is a separate method rather than a second `add_*_node`
    /// constructor — the same shape [`WarGraph::add_deferred_node`]'s
    /// `defer` flag already established — so a node can be BOTH deferred
    /// AND a dynamic target; composing two `add_*_node` constructors could
    /// not express that.
    ///
    /// This marker shifts responsibility for checking that a runtime jump
    /// actually lands on a node marked here to CF-FR-07, the later phase
    /// that owns runtime jump validation — `WarGraph::validate` trusts the
    /// declaration and does not itself verify any jump ever targets it.
    ///
    /// Jump targets are deliberately **not** inferred from any directive
    /// parser's output: they are runtime values — computed from
    /// Battlefield state or an LLM's own routing decision — and a parser
    /// cannot know at graph-construction time which branch a live run will
    /// take, so inferring a static edge set from them would be unsound.
    /// This omission is a decision recorded here, not a gap:
    /// `mark_dynamic_target` is the intentional, explicit substitute.
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
    /// point naming an undeclared node, a schema `Custom` dispatch name
    /// with no resolver registered in `custom_dispatch` (ENG-FR-09), or —
    /// checked LAST, once every clause above has passed — a declared node
    /// outside the **eligible set** (ENG-FR-02a / BUG-02): the fixed point
    /// of nodes reachable from `entry` over static edges, unioned with
    /// nodes marked [`WarGraph::mark_dynamic_target`]. All of this runs
    /// before any node executes.
    ///
    /// The eligible-set check is last for two reasons: the more specific
    /// structural errors above stay the ones a caller sees first, and a
    /// graph that already failed one of them has not been shown to have
    /// meaningful reachability at all — reporting "node X is unreachable"
    /// on top of "edge Y names an undeclared node" would bury the actual
    /// mistake under a symptom of it.
    ///
    /// `RunOutcome::Completed` means "the Vanguard emptied"; before
    /// ENG-FR-02a that could be reported over a graph containing a node
    /// that could never have become ready (BUG-02, the silent stranded
    /// node). This check is what makes that claim truthful: every declared
    /// node was at least eligible to run. A self-loop remains legal on an
    /// entry node, or on any node reachable from entry by a normal edge —
    /// this check rejects strandedness, a node that could NEVER become
    /// ready, not cycles.
    ///
    /// A THIRD clause -- [`WarGraph::validate_schedulable`], D-03's
    /// unschedulable-cycle guard (BUG-03) -- runs after the eligible-set
    /// clause, for the same "more specific error stays first" reason: a
    /// component fed only from within itself IS statically reachable from
    /// `entry` (once any one of its members is), so the eligible-set clause
    /// never catches it, and it is checked last so it functions purely as
    /// defence-in-depth against a shape the ENG-FR-06a starvation release
    /// still cannot schedule, never masking a more fundamental structural
    /// error above it.
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

        self.validate_eligible_set()?;
        self.validate_schedulable()
    }

    /// ENG-FR-02a / BUG-02's eligible-set reachability check, factored out
    /// of [`WarGraph::validate`] only for readability -- always called last
    /// from there, never on its own.
    fn validate_eligible_set(&self) -> Result<(), EngineError> {
        // A graph that declares nodes but never calls `add_entry` at all:
        // every node is trivially unreachable regardless of edges or
        // dynamic-target markers, since nothing seeds the worklist below
        // and a dynamic target only ever fires from a LIVE run that has to
        // start somewhere. Naming the absent entry point as the cause
        // avoids listing every node in the graph with a generic
        // reachability message that would bury the actual mistake.
        if self.entry.is_empty() && !self.nodes.is_empty() {
            return Err(EngineError::UnreachableNode {
                nodes: self.node_order.clone(),
                reason: format!(
                    "no entry point declared: {} node(s) registered but WarGraph::add_entry \
                     was never called, so every node is unreachable regardless of edges or \
                     dynamic_target markers -- declare at least one entry node",
                    self.node_order.len()
                ),
            });
        }

        // The eligible set: entry nodes and dynamic-target-marked nodes,
        // expanded by following declared edges to a fixed point (edge
        // CONDITIONS are ignored here -- they are runtime values, and a
        // statically-declared edge is what proves intent for this static
        // check). Two future sources of eligibility plug into this SAME
        // worklist and nowhere else: nodes marked as worker templates,
        // reachable via dynamic fan-out (Phase 23 / Muster), and nodes
        // named as Route { to } targets in an eligible node's Aegis
        // `on_error` policy (Phase 25 / CF-FR handler routing) -- which is
        // why this is a fixed point rather than a single pass: a route
        // target discovered late can itself carry outgoing edges that need
        // re-expanding. Neither concept exists in this tree yet; nothing
        // is fabricated here to stand in for either -- these are insertion
        // points, not stubs.
        let mut eligible: HashSet<NodeId> = HashSet::new();
        let mut worklist: Vec<NodeId> = Vec::new();
        for id in self.entry.iter().chain(self.dynamic_targets.iter()) {
            if eligible.insert(id.clone()) {
                worklist.push(id.clone());
            }
        }
        while let Some(current) = worklist.pop() {
            for edge in &self.edges {
                if edge.from == current && eligible.insert(edge.to.clone()) {
                    worklist.push(edge.to.clone());
                }
            }
        }

        // Every declared node outside the eligible set is an offender,
        // collected in `node_order` (registration order, ENG-FR-04) so the
        // message is deterministic across runs, never `HashMap` order.
        let offenders: Vec<NodeId> = self
            .node_order
            .iter()
            .filter(|id| !eligible.contains(*id))
            .cloned()
            .collect();
        if offenders.is_empty() {
            return Ok(());
        }

        let names = offenders
            .iter()
            .map(NodeId::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        Err(EngineError::UnreachableNode {
            nodes: offenders,
            reason: format!(
                "unreachable from entry and not marked dynamic_target: {names} -- make \
                 reachable via a static edge from an entry node, or mark with \
                 WarGraph::mark_dynamic_target if it is a runtime jump target"
            ),
        })
    }

    /// D-03's fixpoint: every declared node that can NEVER receive a fired
    /// edge from outside its own component, and therefore can never be
    /// bootstrapped by ENG-FR-06a's starvation-release pass
    /// (`superstep::starved_release`) -- that pass only ever releases a node
    /// already holding at least one fresh fired incoming edge, and a
    /// component fed only by its own members can never produce one.
    ///
    /// Seeds `unfed` with every declared node that is NOT an entry point and
    /// has at least one incoming edge (a node with zero incoming edges is
    /// the eligible-set clause's problem, not this one's), then repeatedly
    /// removes any node that has an incoming edge whose source is NOT
    /// itself still in `unfed` -- such a node can receive a fired edge from
    /// outside the shrinking set, and is therefore reachable by the
    /// starvation release once that outside source runs. What survives to a
    /// fixpoint is exactly the node set of one or more components with no
    /// edge crossing in from anywhere else.
    ///
    /// Nodes marked [`WarGraph::mark_dynamic_target`] are removed from the
    /// survivors after the fixpoint converges: a dynamic target is the
    /// declared runtime-entry escape hatch (ENG-FR-02a / BUG-02) and is
    /// exempt from this check for the same reason it is exempt from the
    /// eligible-set check -- ENG-FR-02a's future worker-template (Phase 23)
    /// and Route-target (Phase 25) exemptions join this same exclusion list
    /// when those features land; nothing is fabricated here to stand in for
    /// either.
    ///
    /// Returns the survivors in `self.node_order` order (ENG-FR-04,
    /// deterministic, never `HashMap`/`HashSet` order).
    fn unschedulable_unfed_nodes(&self) -> Vec<NodeId> {
        let entry_set: HashSet<&NodeId> = self.entry.iter().collect();
        let mut incoming: HashMap<&NodeId, Vec<&NodeId>> = HashMap::new();
        for edge in &self.edges {
            incoming.entry(&edge.to).or_default().push(&edge.from);
        }

        let mut unfed: HashSet<NodeId> = self
            .node_order
            .iter()
            .filter(|id| !entry_set.contains(id) && incoming.contains_key(id))
            .cloned()
            .collect();

        loop {
            let to_remove: Vec<NodeId> = unfed
                .iter()
                .filter(|node| {
                    let sources = incoming.get(*node).cloned().unwrap_or_default();
                    sources.iter().any(|source| !unfed.contains(*source))
                })
                .cloned()
                .collect();
            if to_remove.is_empty() {
                break;
            }
            for node in to_remove {
                unfed.remove(&node);
            }
        }

        self.node_order
            .iter()
            .filter(|id| unfed.contains(*id) && !self.dynamic_targets.contains(*id))
            .cloned()
            .collect()
    }

    /// D-03's `validate()` clause consuming
    /// [`WarGraph::unschedulable_unfed_nodes`] -- see that method for the
    /// fixpoint rule, and [`EngineError::UnschedulableCycle`] for why this
    /// check exists and why it is checked last. Factored out only for
    /// readability, exactly as [`WarGraph::validate_eligible_set`] is --
    /// always called last from [`WarGraph::validate`], never on its own.
    fn validate_schedulable(&self) -> Result<(), EngineError> {
        let offenders = self.unschedulable_unfed_nodes();
        if offenders.is_empty() {
            return Ok(());
        }

        let names = offenders
            .iter()
            .map(NodeId::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        Err(EngineError::UnschedulableCycle {
            nodes: offenders,
            reason: format!(
                "no edge from outside the component reaches: {names} -- ENG-FR-06a's \
                 starvation release can only bootstrap a cycle already holding a fresh fired \
                 edge from an external source, so this component can never take its first \
                 turn -- feed it from an entry-reachable node outside the component, or mark \
                 its entry point with WarGraph::mark_dynamic_target if it is a runtime jump \
                 target"
            ),
        })
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
        // `a` is declared entry here for a reason unrelated to strandedness
        // (this is a `validate`-only test; nothing runs). It is a readiness
        // dodge: `a` has no other node to be fed by, so its self-loop edge
        // is its ONLY incoming edge -- `Frontier::is_ready`
        // (`engine::superstep`) treats that edge as `Pending` until `a` has
        // executed once, meaning a non-entry `a` could never take its first
        // turn regardless of reachability. Making it entry is what lets it
        // run at all; this is the readiness-dodge classification audited in
        // Phase 22 Plan 16 (`22-deferred-items.md`), not a stranded-node
        // workaround -- `a` was never at risk of BUG-02's rejection since
        // entry nodes are always eligible.
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

    use crate::engine::WaypointDurability;
    use crate::engine::hooks::TraceDispatcher;
    use crate::engine::test_support::{
        CountingFunctionNode, RecordingPaladinPort, RecordingWaypointStore,
    };
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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
        // Readiness dodge, not a strandedness workaround: `a` is the only
        // node in this graph, so its self-loop is its only incoming edge.
        // `Frontier::is_ready` (`engine::superstep`) leaves a self-loop edge
        // `Pending` until the node has executed once, so a non-entry `a`
        // could never take its first turn -- entry status is what bootstraps
        // it, independent of BUG-02 (Phase 22 Plan 16 audit,
        // `22-deferred-items.md`).
        let field_name = FieldName::new("status").unwrap();
        let node = CountingFunctionNode::new(move |run_index, _state| {
            let status = if run_index == 1 {
                "approved"
            } else {
                "looping"
            };
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
        // Deliberately `validate`-only -- this is the "unrelated" bucket of
        // the Phase 22 Plan 16 fixture audit (`22-deferred-items.md`): `b`
        // here has BOTH a self-loop and an external incoming edge, which is
        // exactly the combination the readiness rule (`Frontier::is_ready`
        // in `engine::superstep`) can never schedule -- a `Pending` self-loop
        // edge blocks `b`'s first turn even though the external edge from
        // `a` fires. Running this graph to completion would reproduce that
        // defect rather than test what this fixture is actually pinning
        // (that `validate` itself, a static check, has no opinion about
        // runtime readiness). See the ignored reproduction
        // `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`
        // in `engine::superstep` for the runnable defect.
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
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

    // --- D-03 / BUG-03: unschedulable-cycle validate-time guard (Phase
    // 22.1 Plan 02). `unschedulable_unfed_nodes` is the fixpoint
    // `validate_schedulable` consumes -- see both methods' rustdoc for the
    // rule. These tests exercise the private fixpoint directly (same-module
    // access) so the fixpoint's own behaviour is pinned independently of
    // `validate`'s clause ordering, which the fourth test below pins
    // separately.

    #[test]
    fn unschedulable_unfed_nodes_is_empty_for_a_cycle_fed_from_entry() {
        // entry -> a, a -> b, b -> a; only "entry" is a declared entry
        // point. This is exactly the shape ENG-FR-06a's starvation release
        // (Plan 22.1-01) fixed: "a" is fed from outside the {a, b}
        // component by "entry", so the whole component is reachable from a
        // single external firing -- this fixpoint must NOT reject it.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("entry"),
            to: NodeId::new("a"),
            condition: None,
        });
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
        graph.add_entry(NodeId::new("entry"));

        assert!(graph.unschedulable_unfed_nodes().is_empty());
        assert!(
            graph.validate(&CustomDispatchResolver::new()).is_ok(),
            "a cycle fed from entry must validate cleanly -- it is schedulable via the \
             starvation release"
        );
    }

    #[test]
    fn unschedulable_unfed_nodes_names_every_node_of_an_externally_unfed_cycle() {
        // An entry node plus a disjoint 2-cycle x -> y, y -> x: neither x
        // nor y is entry, and no edge reaches the pair from outside it --
        // the component can never receive a fired edge from anywhere but
        // its own members, so the starvation release can never bootstrap
        // it.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(NodeId::new("x"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("y"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("x"),
            to: NodeId::new("y"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("y"),
            to: NodeId::new("x"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        assert_eq!(
            graph.unschedulable_unfed_nodes(),
            vec![NodeId::new("x"), NodeId::new("y")],
            "both cycle members must be named, in node_order order"
        );
    }

    #[test]
    fn unschedulable_unfed_nodes_exempts_runtime_entry_marked_nodes() {
        // Same externally-unfed 2-cycle as above, but both members are
        // marked dynamic_target -- the declared runtime-entry escape hatch
        // must exempt them from this check exactly as it does from the
        // eligible-set check.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(NodeId::new("x"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("y"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("x"),
            to: NodeId::new("y"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("y"),
            to: NodeId::new("x"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));
        graph.mark_dynamic_target(NodeId::new("x"));
        graph.mark_dynamic_target(NodeId::new("y"));

        assert!(
            graph.unschedulable_unfed_nodes().is_empty(),
            "runtime-entry-marked nodes must be exempted from the unschedulable-cycle check"
        );
    }

    #[test]
    fn validate_prefers_unreachable_node_error_over_unschedulable_cycle() {
        // The externally-unfed 2-cycle, unmarked: validate_eligible_set
        // rejects "x" and "y" as UnreachableNode first (they are not
        // statically reachable from "entry" and carry no dynamic_target
        // marker), so validate() must never reach validate_schedulable's
        // UnschedulableCycle clause for this graph -- the ordering pinned
        // by D-02(d).
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(NodeId::new("x"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("y"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("x"),
            to: NodeId::new("y"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("y"),
            to: NodeId::new("x"),
            condition: None,
        });
        graph.add_entry(NodeId::new("entry"));

        let err = graph.validate(&CustomDispatchResolver::new()).unwrap_err();
        assert!(
            matches!(err, EngineError::UnreachableNode { .. }),
            "expected UnreachableNode (eligible-set clause runs first), got {err:?}"
        );
    }
}
