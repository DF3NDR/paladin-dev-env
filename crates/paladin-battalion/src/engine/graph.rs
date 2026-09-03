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
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeId, canonical_edge_condition,
};

use crate::edge_evaluator::EdgeEvaluatorRegistry;
use crate::engine::EngineError;
use crate::engine::directive_parser::DirectiveParser;
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
        /// How this node's raw string output is turned into a routing
        /// `Directive` (CF-02, D-11). Defaults to `DirectiveParser::PlainOutput`
        /// via [`NodeSpec::paladin`], reproducing pre-CF-02 behavior exactly.
        directive_parser: DirectiveParser,
    },
    /// A pure, deterministic state -> delta node.
    Function(Arc<dyn StateNode>),
}

impl NodeSpec {
    /// Construct a `NodeSpec::Paladin` with `DirectiveParser::PlainOutput`
    /// (D-11's default): the raw output is written to `output_field` and the
    /// node routes via its static outgoing edges, byte-identical to a
    /// pre-CF-02 Paladin node. The constructor every in-tree call site uses,
    /// so no call site needs to name the new `directive_parser` field.
    pub fn paladin(
        paladin: Paladin,
        input_template: InputMapping,
        output_field: FieldName,
    ) -> Self {
        NodeSpec::paladin_with_directive_parser(
            paladin,
            input_template,
            output_field,
            DirectiveParser::PlainOutput,
        )
    }

    /// Construct a `NodeSpec::Paladin` with an explicit `DirectiveParser`
    /// (D-11), for a node that opts into `DirectiveParser::StructuredDirective`.
    pub fn paladin_with_directive_parser(
        paladin: Paladin,
        input_template: InputMapping,
        output_field: FieldName,
        directive_parser: DirectiveParser,
    ) -> Self {
        NodeSpec::Paladin {
            paladin: Box::new(paladin),
            input_template,
            output_field,
            directive_parser,
        }
    }
}

impl std::fmt::Debug for NodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeSpec::Paladin {
                output_field,
                directive_parser,
                ..
            } => f
                .debug_struct("NodeSpec::Paladin")
                .field("output_field", output_field)
                .field("directive_parser", directive_parser)
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
    pub fn validate(
        &self,
        custom_dispatch: &CustomDispatchResolver,
        edge_evaluators: &EdgeEvaluatorRegistry,
    ) -> Result<(), EngineError> {
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

        self.validate_edge_evaluators(edge_evaluators)?;

        self.validate_eligible_set()?;
        self.validate_schedulable()
    }

    /// BUG-01 / CF-FR-02's fail-closed clause: every declared edge carrying
    /// `EdgeCondition::Custom(name)` must resolve to a registered evaluator
    /// in `edge_evaluators`, checked here -- before any node executes --
    /// so an unregistered custom condition never silently defaults to
    /// always-true (the defect this clause replaces, `BUG-01`). Collects
    /// EVERY offending name, sorted and deduplicated, rather than failing
    /// on the first, matching [`WarGraph::validate_eligible_set`]'s
    /// "report the whole problem at once" discipline. Checked before the
    /// eligible-set clause since an unregistered custom condition is a
    /// more specific, more actionable problem than a reachability failure
    /// the same graph might also have.
    fn validate_edge_evaluators(
        &self,
        edge_evaluators: &EdgeEvaluatorRegistry,
    ) -> Result<(), EngineError> {
        let mut names: Vec<String> = self
            .edges
            .iter()
            .filter_map(|edge| match &edge.condition {
                Some(EdgeCondition::Custom(name)) if !edge_evaluators.contains(name) => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        if names.is_empty() {
            return Ok(());
        }
        Err(EngineError::UnregisteredEdgeCondition { names })
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

    /// Compute this graph's stable content fingerprint (ENG-FR-14 / CR-01,
    /// D-15): a hash over every scheduling- and merge-relevant graph
    /// property --
    ///
    /// - node ids, plus each `NodeSpec::Paladin`'s `output_field` (a
    ///   `Function` node writes an unambiguous "no output field" marker
    ///   instead, so a node named `x` with no output field can never
    ///   produce the same bytes as one with an empty one);
    /// - edges, by `from`/`to` plus the edge's serde-canonical
    ///   `EdgeCondition`;
    /// - schema field names, plus each field's serde-canonical
    ///   `DispatchRule`;
    /// - the declared entry set (`self.entry`);
    /// - `self.defer_flags`;
    /// - `self.dynamic_targets`.
    ///
    /// Deliberately NOT covered (ENG-FR-14): Paladin prompts, model names,
    /// `InputMapping` templates, or `EngineLimits` -- raising a limit or
    /// tuning a prompt to let a resumed run continue is a legitimate
    /// operator action and must not trip `EngineError::GraphMismatch`.
    ///
    /// The edge condition and dispatch rule are hashed through their serde
    /// representation (`serde_json::to_string`), never through `Debug`: a
    /// `#[derive(Debug)]` change on either type would otherwise silently
    /// move every stored fingerprint with no compiler warning, while the
    /// serde representation is a stable, versioned contract (D-16). A
    /// serialization failure degrades to stable empty bytes rather than a
    /// panic, matching `evaluate_edge_condition`'s existing
    /// `unwrap_or_default()` convention.
    ///
    /// Every collection above is sorted before hashing (node ids, edges,
    /// schema fields, entry set, defer flags, dynamic targets), so the
    /// result never depends on `HashMap`/`HashSet` iteration order
    /// (RESEARCH.md Pitfall 5).
    ///
    /// **Length-prefixed encoding (Phase 22.1 CR-01, D-17, `v2`).** Every
    /// variable-length field ([`push_field`]) is preceded by its byte length
    /// as a fixed-width 8-byte little-endian integer before its bytes are
    /// written, so no byte sequence can be reinterpreted as a different
    /// split across a field or node/edge boundary. The prior `v1` encoding
    /// joined fields with unescaped ASCII delimiters (`|`, `-`, `:`) that
    /// were themselves legal characters inside a `NodeId`/`FieldName`
    /// (neither type restricts its character set beyond non-emptiness), so
    /// two structurally different graphs could be crafted to fingerprint
    /// identically -- e.g. two Function nodes `"a"` and `"b"` versus a
    /// single Function node named `"a|nf|b"`. `GRAPH_FINGERPRINT_VERSION`
    /// was bumped to `"v2"` alongside this fix (`paladin-core`'s
    /// `waypoint.rs`) so every fingerprint stored under the old encoding is
    /// recognized as a version-tag mismatch on `resume` rather than
    /// silently reinterpreted under the new one.
    ///
    /// A golden hex test pins the exact output of a fixture exercising
    /// every hashed property (`engine::graph::tests::
    /// fingerprint_golden_hex_pins_canonical_bytes`); changing this
    /// function's byte layout invalidates every stored Waypoint's
    /// fingerprint and must not be done without a deliberate format-version
    /// bump (D-17).
    pub fn fingerprint(&self) -> GraphFingerprint {
        let mut node_ids: Vec<&NodeId> = self.nodes.keys().collect();
        node_ids.sort();

        let mut edges: Vec<&EdgeSpec> = self.edges.iter().collect();
        edges.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str()))
        });

        let mut fields: Vec<_> = self.schema.fields.iter().collect();
        fields.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));

        let mut entry_ids: Vec<&NodeId> = self.entry.iter().collect();
        entry_ids.sort();

        let mut defer_ids: Vec<&NodeId> = self.defer_flags.iter().collect();
        defer_ids.sort();

        let mut dynamic_target_ids: Vec<&NodeId> = self.dynamic_targets.iter().collect();
        dynamic_target_ids.sort();

        let mut buf = Vec::new();
        buf.extend_from_slice(b"nodes:");
        for id in &node_ids {
            push_field(&mut buf, id.as_str().as_bytes());
            match self.nodes.get(*id) {
                Some(NodeSpec::Paladin { output_field, .. }) => {
                    buf.push(1); // "has output field" tag
                    push_field(&mut buf, output_field.as_str().as_bytes());
                }
                _ => {
                    buf.push(0); // "no output field" tag
                }
            }
        }
        buf.extend_from_slice(b";edges:");
        for edge in &edges {
            push_field(&mut buf, edge.from.as_str().as_bytes());
            push_field(&mut buf, edge.to.as_str().as_bytes());
            let condition_json = canonical_edge_condition(&edge.condition);
            push_field(&mut buf, condition_json.as_bytes());
        }
        buf.extend_from_slice(b";schema:");
        for field in &fields {
            push_field(&mut buf, field.name.as_str().as_bytes());
            let dispatch_json = serde_json::to_string(&field.dispatch).unwrap_or_default();
            push_field(&mut buf, dispatch_json.as_bytes());
        }
        buf.extend_from_slice(b";entry:");
        for id in &entry_ids {
            push_field(&mut buf, id.as_str().as_bytes());
        }
        buf.extend_from_slice(b";defer_flags:");
        for id in &defer_ids {
            push_field(&mut buf, id.as_str().as_bytes());
        }
        buf.extend_from_slice(b";dynamic_targets:");
        for id in &dynamic_target_ids {
            push_field(&mut buf, id.as_str().as_bytes());
        }

        GraphFingerprint::from_canonical_bytes(&buf)
    }
}

/// Write `bytes` to `buf` preceded by its length as a fixed-width 8-byte
/// little-endian integer (Phase 22.1 CR-01, D-17). Used exclusively by
/// [`WarGraph::fingerprint`] to build a canonical byte stream in which no
/// field's bytes can be reinterpreted as a different split across a
/// node/edge boundary -- the defect a delimiter-only encoding (`v1`) was
/// vulnerable to whenever a `NodeId`/`FieldName` legally contained one of
/// the delimiter bytes.
fn push_field(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(bytes);
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
            paladin_core::platform::container::directive::Directive,
            crate::engine::node::NodeError,
        > {
            Ok(paladin_core::platform::container::battlefield::StateDelta::new().into())
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

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
    }

    #[test]
    fn validate_accepts_self_loop() {
        // `a` is declared entry here for a reason unrelated to strandedness
        // (this is a `validate`-only test; nothing runs). `a` is this
        // graph's only node, so its self-loop is its ONLY possible incoming
        // edge; a single-node self-loop graph needs an entry to ever start
        // at all -- that is why `a` is entry, not because the readiness
        // rule (`Frontier::is_ready`, `engine::superstep`) requires it.
        // `a` was never at risk of BUG-02's rejection either, since entry
        // nodes are always eligible. This shape has no feed from outside
        // itself at all, so BUG-03's starvation-release fix
        // (`Frontier::starved_release`, `engine::superstep`) does not apply
        // here either -- a non-entry `a` genuinely could never take a first
        // turn in this exact shape, unlike the fed-from-outside shapes
        // BUG-03 fixed. This is the readiness-dodge classification audited
        // in Phase 22 Plan 16 (`22-deferred-items.md`).
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("a"),
            condition: None,
        });
        graph.add_entry(NodeId::new("a"));

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
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
            graph.validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new()
            ),
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
            graph.validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new()
            ),
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
        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
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
        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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
        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::UnknownNode(id) if id == NodeId::new("ghost")));
    }

    #[test]
    fn validate_rejects_unknown_entry() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_entry(NodeId::new("ghost"));
        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        assert!(
            graph
                .validate(&registry, &EdgeEvaluatorRegistry::new())
                .is_ok()
        );
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
        // Recomputed for Phase 22.1 CR-01 (D-15, D-17): this fixture
        // declares "solo" as entry, and the entry set is now part of the
        // hashed bytes; the length-prefixed `v2` re-encoding (CR-01 fix)
        // also changes the literal independently of any property change.
        // `fingerprint_golden_hex_pins_canonical_bytes` (Task 2) is the
        // dedicated golden test guarding future canonicalization changes;
        // this assertion only re-confirms same-input determinism.
        assert_eq!(
            a.as_str(),
            "v2:9c4b3ff495ed7f1872420455f7691b991a58707e902a3211a5c19c1da8613520"
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

    // --- CR-01 / D-15 / D-17: golden hex plus per-property difference
    // tests pinning WarGraph::fingerprint()'s extended canonical bytes.
    // `golden_fingerprint_fixture` builds one graph exercising every
    // hashed property (a Paladin node with an output_field, two edges
    // with different conditions, two schema fields with different
    // DispatchRules, a declared entry set, one defer-marked node and one
    // dynamic_target-marked node) so a single spec, varied one field at a
    // time, can build both the golden fixture and each difference test's
    // one-property variant.

    fn make_fixture_paladin(name: &str, prompt: &str, model: &str) -> Paladin {
        let data = paladin_core::platform::container::paladin::PaladinData {
            name: name.to_string(),
            system_prompt: prompt.to_string(),
            model: model.to_string(),
            ..Default::default()
        };
        paladin_core::base::entity::node::Node::new(data, Some(name.to_string()))
    }

    /// The one-property-at-a-time knobs `golden_fingerprint_fixture`
    /// builds a graph from. `Default` matches the golden fixture exactly;
    /// each difference test clones the default and overrides exactly one
    /// field.
    struct FingerprintFixtureSpec {
        entry: Vec<&'static str>,
        defer_aggregator: bool,
        dynamic_target_jump: bool,
        worker_to_aggregator_condition: EdgeCondition,
        notes_dispatch: DispatchRule,
        worker_output_field: &'static str,
        worker_prompt: &'static str,
        worker_model: &'static str,
        worker_input_template: &'static str,
        limits: EngineLimits,
    }

    impl Default for FingerprintFixtureSpec {
        fn default() -> Self {
            Self {
                entry: vec!["entry"],
                defer_aggregator: true,
                dynamic_target_jump: true,
                worker_to_aggregator_condition: EdgeCondition::Contains("done".to_string()),
                notes_dispatch: DispatchRule::Append,
                worker_output_field: "notes",
                worker_prompt: "process {status}",
                worker_model: "gpt-4",
                worker_input_template: "process {status}",
                limits: EngineLimits::default(),
            }
        }
    }

    fn golden_fingerprint_fixture(spec: &FingerprintFixtureSpec) -> WarGraph {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("status").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(
                FieldName::new("notes").unwrap(),
                spec.notes_dispatch.clone(),
                None,
                false,
            ),
        ]);
        let mut graph = WarGraph::new(schema, spec.limits.clone());

        graph.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(
            NodeId::new("worker"),
            NodeSpec::paladin(
                make_fixture_paladin("worker", spec.worker_prompt, spec.worker_model),
                InputMapping::new(spec.worker_input_template),
                FieldName::new(spec.worker_output_field).unwrap(),
            ),
        );
        if spec.defer_aggregator {
            graph.add_deferred_node(
                NodeId::new("aggregator"),
                NodeSpec::Function(StdArc::new(NoopNode)),
            );
        } else {
            graph.add_node(
                NodeId::new("aggregator"),
                NodeSpec::Function(StdArc::new(NoopNode)),
            );
        }
        graph.add_node(
            NodeId::new("jump_target"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        if spec.dynamic_target_jump {
            graph.mark_dynamic_target(NodeId::new("jump_target"));
        }

        graph.add_edge(EdgeSpec {
            from: NodeId::new("entry"),
            to: NodeId::new("worker"),
            condition: Some(EdgeCondition::Always),
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("worker"),
            to: NodeId::new("aggregator"),
            condition: Some(spec.worker_to_aggregator_condition.clone()),
        });

        for id in &spec.entry {
            graph.add_entry(NodeId::new(*id));
        }

        graph
    }

    /// Pins the exact canonical-bytes output of the golden fixture. Phase
    /// 22's D-04 rated the canonical byte layout one-way after release
    /// (changing it invalidates every stored Waypoint's fingerprint); this
    /// test is what makes that hazard observable in CI rather than in a
    /// released user's silently-broken `resume` (D-17). The pinned literal
    /// may only be updated together with a deliberate format-version bump.
    ///
    /// Re-pinned for Phase 22.1 CR-01/D-17's length-prefixed `v2` encoding
    /// (see `fingerprint_distinguishes_length_prefix_collision_*` below for
    /// the collision the prior `v1` delimiter-based encoding was vulnerable
    /// to).
    #[test]
    fn fingerprint_golden_hex_pins_canonical_bytes() {
        let graph = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        assert_eq!(
            graph.fingerprint().as_str(),
            "v2:8eaa709a9ed7356799747694382d508608f1b76f3ec9e2bdae079868f2f60711",
            "canonicalization changed -- this invalidates every stored Waypoint's \
             fingerprint; only update this literal together with a deliberate \
             format-version bump"
        );
    }

    // --- CR-01 (Phase 22.1) regression tests: the two collisions the prior
    // delimiter-based `v1` encoding was vulnerable to must now fingerprint
    // differently under the length-prefixed `v2` encoding.

    #[test]
    fn fingerprint_distinguishes_length_prefix_collision_two_nodes_vs_one() {
        // Two independent Function nodes "a" and "b" vs. a single Function
        // node named "a|nf|b". Under the old delimiter encoding both
        // produced the same bytes ("a|nf|b|nf|"); under the length-prefixed
        // encoding the byte length of each field is unambiguous.
        let schema = one_field_schema();

        let mut two_nodes = WarGraph::new(schema.clone(), EngineLimits::default());
        two_nodes.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        two_nodes.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        two_nodes.add_entry(NodeId::new("a"));

        let mut one_node = WarGraph::new(schema, EngineLimits::default());
        one_node.add_node(
            NodeId::new("a|nf|b"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        one_node.add_entry(NodeId::new("a|nf|b"));

        assert_ne!(two_nodes.fingerprint(), one_node.fingerprint());
    }

    #[test]
    fn fingerprint_distinguishes_length_prefix_collision_edge_split() {
        // Edge "a" -> "b-c" vs. edge "a-b" -> "c". Under the old delimiter
        // encoding both produced the same bytes ("a-b-c:null|"); under the
        // length-prefixed encoding the split point is unambiguous.
        let schema = one_field_schema();

        let mut split_at_a = WarGraph::new(schema.clone(), EngineLimits::default());
        split_at_a.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        split_at_a.add_node(
            NodeId::new("b-c"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        split_at_a.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b-c"),
            condition: None,
        });
        split_at_a.add_entry(NodeId::new("a"));

        let mut split_at_b = WarGraph::new(schema, EngineLimits::default());
        split_at_b.add_node(
            NodeId::new("a-b"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        split_at_b.add_node(NodeId::new("c"), NodeSpec::Function(StdArc::new(NoopNode)));
        split_at_b.add_edge(EdgeSpec {
            from: NodeId::new("a-b"),
            to: NodeId::new("c"),
            condition: None,
        });
        split_at_b.add_entry(NodeId::new("a-b"));

        assert_ne!(split_at_a.fingerprint(), split_at_b.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_entry_set_changes() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            entry: vec!["entry", "jump_target"],
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_a_node_is_deferred() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            defer_aggregator: false,
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_a_node_is_marked_dynamic_target() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            dynamic_target_jump: false,
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_an_edge_condition_changes() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            worker_to_aggregator_condition: EdgeCondition::Contains("finished".to_string()),
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_a_field_dispatch_rule_changes() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            notes_dispatch: DispatchRule::LastWrite,
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    #[test]
    fn fingerprint_changes_when_a_paladin_output_field_changes() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            worker_output_field: "other_notes",
            ..FingerprintFixtureSpec::default()
        });
        assert_ne!(base.fingerprint(), variant.fingerprint());
    }

    /// ENG-FR-14 exclusions: a Paladin prompt, a model name, an
    /// `InputMapping` template and `EngineLimits` must NOT move the
    /// fingerprint -- raising a limit or tuning a prompt to let a resumed
    /// run continue is a legitimate operator action.
    #[test]
    fn fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits() {
        let base = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        let variant = golden_fingerprint_fixture(&FingerprintFixtureSpec {
            worker_prompt: "a completely different prompt asking for something else entirely",
            worker_model: "claude-opus-4",
            worker_input_template: "a totally different template referencing {notes}",
            limits: EngineLimits {
                max_supersteps: 999,
                max_node_visits: 999,
                run_timeout: None,
            },
            ..FingerprintFixtureSpec::default()
        });
        assert_eq!(base.fingerprint(), variant.fingerprint());
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
            &EdgeEvaluatorRegistry::new(),
            graph,
            ThreadId::new("reachability-regression").unwrap(),
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
            graph.entry().to_vec(),
            std::collections::BTreeMap::new(),
            None,
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );

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
        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );

        let outcome = run_to_completion(&graph).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    #[tokio::test]
    async fn self_loop_on_entry_node_still_validates_and_runs() {
        // Readiness dodge, not a strandedness workaround: `a` is the only
        // node in this graph, so its self-loop is its only incoming edge --
        // a single-node self-loop graph needs an entry to ever start at
        // all, regardless of BUG-02's eligible-set check (entry nodes are
        // always eligible) or BUG-03's starvation-release fix
        // (`Frontier::starved_release`, `engine::superstep`), neither of
        // which this shape needs: `a` has no feed from outside itself, so a
        // non-entry `a` genuinely could never take its first turn here.
        // Entry status is what bootstraps it (Phase 22 Plan 16 audit,
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
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok(),
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
        // here has BOTH a self-loop and an external incoming edge from `a`.
        // Before BUG-03 was fixed, running this graph to completion would
        // have reproduced BUG-03's cycle-bootstrap starvation (a `Pending`
        // self-loop edge blocked `b`'s first turn even though the external
        // edge from `a` fired); `Frontier::starved_release`
        // (`engine::superstep`) now releases `b` in exactly this shape, so
        // running it would no longer reproduce a defect. This fixture
        // stays `validate`-only regardless, because that is still the
        // point it pins -- that `validate`, a static check, has no opinion
        // about runtime readiness. See the now-passing regression tests
        // `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`
        // and `cycle_node_fed_from_outside_the_cycle_takes_its_first_turn`
        // in `engine::superstep` for the runnable BUG-03 coverage.
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

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
    }

    #[test]
    fn validate_rejects_graph_with_no_entry_point_naming_absent_entry() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        // No add_entry call at all.

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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
            graph.validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new()
            ),
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
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
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok(),
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

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::UnreachableNode { .. }),
            "expected UnreachableNode (eligible-set clause runs first), got {err:?}"
        );
    }

    // --- BUG-01 / CF-01: registered-evaluator edge conditions, engine
    // validation half. These reproduce BUG-01 on the `WarEngine` path and
    // are committed FAILING (RED) before the fix (GREEN) lands in the same
    // task, per D-05 / traceability protocol step 4.

    #[test]
    fn unregistered_custom_edge_condition_fails_graph_validation() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: Some(EdgeCondition::Custom("is_urgent".to_string())),
        });
        graph.add_entry(NodeId::new("a"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::UnregisteredEdgeCondition { names } => {
                assert_eq!(names, vec!["is_urgent".to_string()]);
            }
            other => panic!("expected UnregisteredEdgeCondition, got {other:?}"),
        }
    }

    #[test]
    fn every_unregistered_custom_name_is_listed_sorted_and_deduped() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("c"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: Some(EdgeCondition::Custom("zeta".to_string())),
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("c"),
            condition: Some(EdgeCondition::Custom("alpha".to_string())),
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("b"),
            to: NodeId::new("c"),
            condition: Some(EdgeCondition::Custom("alpha".to_string())),
        });
        graph.add_entry(NodeId::new("a"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::UnregisteredEdgeCondition { names } => {
                assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
            }
            other => panic!("expected UnregisteredEdgeCondition, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_registered_custom_edge_condition() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_node(NodeId::new("b"), NodeSpec::Function(StdArc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("a"),
            to: NodeId::new("b"),
            condition: Some(EdgeCondition::Custom("is_urgent".to_string())),
        });
        graph.add_entry(NodeId::new("a"));

        struct AlwaysTrue;
        #[async_trait::async_trait]
        impl crate::edge_evaluator::EdgeConditionEvaluator for AlwaysTrue {
            async fn evaluate(
                &self,
                _output: &str,
                _ctx: &crate::edge_evaluator::EdgeContext<'_>,
            ) -> Result<bool, crate::edge_evaluator::EdgeEvaluatorError> {
                Ok(true)
            }
        }
        let mut evaluators = EdgeEvaluatorRegistry::new();
        evaluators.register("is_urgent", StdArc::new(AlwaysTrue));

        assert!(
            graph
                .validate(&CustomDispatchResolver::new(), &evaluators)
                .is_ok()
        );
    }
}
