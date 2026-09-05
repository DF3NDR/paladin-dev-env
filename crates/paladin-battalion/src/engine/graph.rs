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
    BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::{OnExpire, ParleyKind};
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
    /// A child [`WarGraph`] embedded as a node (CF-FR-14, D-19): runs to
    /// completion within ONE parent superstep regardless of how many
    /// supersteps the child itself takes (CF-FR-16, D-21), seeded from and
    /// returning only its [`StateMap`]-mapped fields under the PARENT's
    /// dispatch rules, inheriting every parent engine resource -- the
    /// `PaladinPort`, `WaypointPort`, durability, parallelism, dispatch
    /// resolver, edge-evaluator registry, trace sink, interceptors and
    /// cancellation token -- while using its OWN graph's [`EngineLimits`].
    /// Pre-announced in this file's own rustdoc (line 31) on the
    /// already-open-ended [`NodeSpec`] enum, so this addition needs no
    /// X-10 register row.
    ///
    /// **Limit (HITL-01, D-04):** a child run that suspends awaiting a
    /// Parley (`RunOutcome::AwaitingInput`) is NOT supported this phase --
    /// the parent's own dispatch of this node fails with
    /// `EngineError::ParleyInChildUnsupported`, naming the node and the
    /// child thread. Raise the parley in the PARENT graph instead, today;
    /// propagating a child's parley to the parent is a deferred idea a
    /// later phase may promote.
    Battalion {
        /// The embedded child graph.
        graph: Arc<WarGraph>,
        /// The declared channel between the parent's and the child's
        /// state -- the ONLY fields that cross the boundary in either
        /// direction (CF-FR-14): everything else the child touches stays
        /// private, never copied into the parent's Battlefield, this
        /// node's own delta, or the parent thread's Waypoint payload.
        state_map: StateMap,
        /// Whether a resumed run restarts this node's child from scratch
        /// rather than continuing a partially-completed child thread.
        /// Declared here for Plan 23-09 (child thread identity /
        /// `checkpoint_ns` / resume-mid-child), which owns interpreting
        /// this flag -- this plan carries it and defaults it to `false`
        /// via [`NodeSpec::battalion`], but does not itself act on it.
        restart_on_resume: bool,
    },
    /// A first-class human-input node (HITL-01, D-05): has **no `run` body
    /// of its own**. On its first visit (`engine::superstep`'s dispatch)
    /// it renders `request.prompt_template` and, if declared,
    /// `request.payload_template` from the Battlefield and raises a
    /// `ParleyRequest`, entering the same `NextStep::Parley` suspension
    /// path (plan 24-01) any other node's `Directive` can enter. On the
    /// post-resume visit it writes the delivered, normalised value to
    /// `output_field` -- or, for `ParleyKind::StateEdit`, returns the
    /// response's `StateDelta` as this node's own delta and writes no
    /// field -- then routes via its static outgoing edges exactly like a
    /// [`NodeSpec::Paladin`] node's `output_field` (D-06). An approval
    /// gate is therefore three lines of graph: one `Gate` node plus a
    /// `Contains("true")` and a `Contains("false")` edge.
    ///
    /// `output_field` is **required** for `ParleyKind::Approval`/`Choice`/
    /// `FreeText` and **must be `None`** for `ParleyKind::StateEdit` --
    /// [`WarGraph::validate`] rejects every other combination, and also
    /// rejects an `output_field` absent from the schema or declared with an
    /// incompatible type (D-05).
    ///
    /// **Limit (D-04):** a Gate raised inside a nested
    /// [`NodeSpec::Battalion`] child is not supported this phase -- the
    /// parent's dispatch of that Battalion node fails with the typed
    /// `EngineError::ParleyInChildUnsupported` (landed in plan 24-01).
    /// Raise the parley in the parent graph instead, today; propagating a
    /// child's parley to the parent is a deferred idea a later phase may
    /// promote.
    Gate {
        /// This Gate's declarative request template.
        request: GateRequestTemplate,
        /// The field the delivered value is written to, or `None` for
        /// `ParleyKind::StateEdit`.
        output_field: Option<FieldName>,
    },
}

/// The declared channel between a [`NodeSpec::Battalion`] node's parent
/// and child state (CF-FR-14, D-19) -- the COMPLETE contract for what
/// crosses the composition boundary in either direction; a child field NOT
/// named in `outputs` never leaves the child, and a parent field NOT named
/// in `inputs` is never visible to the child.
///
/// `inputs` pairs are `(parent, child)`: read from the parent's superstep
/// snapshot under the parent field name, written into the child's seeded
/// initial state under the child field name. `outputs` pairs are `(child,
/// parent)`: read from the child's final Battlefield under the child field
/// name, returned as the Battalion node's own delta under the parent field
/// name -- merged through the PARENT's dispatch rules like any other
/// node's delta, exactly as `NodeSpec::Paladin`'s `output_field` is.
///
/// Two shapes the sources leave open (23-RESEARCH.md's spec-less probe
/// returned one `unclassified` row for CF-04) are resolved here by
/// decision, each pinned by a named test: mapping one child field to two
/// DIFFERENT parent fields across two `outputs` pairs is ACCEPTED -- a
/// fan-out of one value through two parent dispatch rules, with no
/// ambiguity -- and an empty `inputs` list is ACCEPTED -- a child needing
/// no seeded state is legitimate (a generator subgraph).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateMap {
    /// `(parent field, child field)` pairs seeding the child's initial
    /// state from the parent's superstep snapshot.
    pub inputs: Vec<(FieldName, FieldName)>,
    /// `(child field, parent field)` pairs returning the child's final
    /// state as this node's delta.
    pub outputs: Vec<(FieldName, FieldName)>,
}

impl StateMap {
    /// Construct an empty `StateMap` (no fields cross the boundary either
    /// way).
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add one `(parent, child)` input pair.
    pub fn with_input(mut self, parent: FieldName, child: FieldName) -> Self {
        self.inputs.push((parent, child));
        self
    }

    /// Add one `(child, parent)` output pair.
    pub fn with_output(mut self, child: FieldName, parent: FieldName) -> Self {
        self.outputs.push((child, parent));
        self
    }
}

impl Default for StateMap {
    fn default() -> Self {
        Self::new()
    }
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

    /// Construct a `NodeSpec::Battalion` embedding `graph`, defaulting
    /// `restart_on_resume` to `false` (Claude's discretion, CONTEXT.md) --
    /// every in-tree call site not otherwise concerned with Plan 23-09's
    /// resume semantics uses this constructor.
    pub fn battalion(graph: Arc<WarGraph>, state_map: StateMap) -> Self {
        NodeSpec::Battalion {
            graph,
            state_map,
            restart_on_resume: false,
        }
    }

    /// Construct a `NodeSpec::Gate` (HITL-01, D-05): a first-class
    /// human-input node with no `run` body of its own. `WarGraph::validate`
    /// enforces `output_field`'s presence/absence and type against
    /// `request.kind` -- see [`NodeSpec::Gate`]'s own rustdoc for the exact
    /// rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::graph::{GateRequestTemplate, NodeSpec};
    /// use paladin_battalion::engine::InputMapping;
    /// use paladin_core::platform::container::battlefield::FieldName;
    /// use paladin_core::platform::container::parley::ParleyKind;
    ///
    /// let gate = NodeSpec::gate(
    ///     GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("Proceed?")),
    ///     Some(FieldName::new("approved").unwrap()),
    /// );
    /// assert!(matches!(gate, NodeSpec::Gate { .. }));
    /// ```
    pub fn gate(request: GateRequestTemplate, output_field: Option<FieldName>) -> Self {
        NodeSpec::Gate {
            request,
            output_field,
        }
    }
}

/// A [`NodeSpec::Gate`] node's declarative request template (HITL-01,
/// D-05): rendered from the Battlefield through [`InputMapping`] on the
/// node's first visit, with `expires_at = now + expires_in` stamped at
/// raise time (`engine::superstep`).
///
/// Carries no `serde` derive, and needs none: `NodeSpec` itself is never
/// serialized -- it holds `Arc<dyn StateNode>` and `Box<Paladin>` (see this
/// file's own top, where [`NodeSpec`] is declared) -- so a
/// `GateRequestTemplate` is, like every other `NodeSpec` payload,
/// Rust-constructed graph-building code, never round-tripped through
/// serde. `expires_in` is therefore a plain [`std::time::Duration`] with no
/// seconds-based wire representation to design (RESEARCH.md Open Question
/// 2, settled).
#[derive(Debug, Clone)]
pub struct GateRequestTemplate {
    /// The shape of input this Gate awaits.
    pub kind: ParleyKind,
    /// Renders the raised `ParleyRequest.prompt` from the Battlefield.
    pub prompt_template: InputMapping,
    /// Renders the raised `ParleyRequest.payload` from the Battlefield, if
    /// declared. `None` raises with an empty object payload.
    pub payload_template: Option<InputMapping>,
    /// Valid choices, for [`ParleyKind::Choice`].
    pub choices: Option<Vec<String>>,
    /// How long after raising this request expires, if ever --
    /// `expires_at = now + expires_in`, stamped at raise time.
    pub expires_in: Option<Duration>,
    /// What happens if this request expires unanswered.
    pub on_expire: OnExpire,
}

impl GateRequestTemplate {
    /// Construct a `GateRequestTemplate` of `kind`, rendering `prompt_template`,
    /// with no payload template, no choices, no expiry, and
    /// [`OnExpire::FailRun`] (its `Default`) -- the fluent `with_*` methods
    /// below customize any of these.
    pub fn new(kind: ParleyKind, prompt_template: InputMapping) -> Self {
        Self {
            kind,
            prompt_template,
            payload_template: None,
            choices: None,
            expires_in: None,
            on_expire: OnExpire::default(),
        }
    }

    /// Render the raised `ParleyRequest.payload` from `payload_template`.
    pub fn with_payload_template(mut self, payload_template: InputMapping) -> Self {
        self.payload_template = Some(payload_template);
        self
    }

    /// Declare the valid choices for a [`ParleyKind::Choice`] gate.
    pub fn with_choices(mut self, choices: Vec<String>) -> Self {
        self.choices = Some(choices);
        self
    }

    /// Set how long after raising this request expires
    /// (`expires_at = now + expires_in`, stamped at raise time).
    pub fn with_expires_in(mut self, expires_in: Duration) -> Self {
        self.expires_in = Some(expires_in);
        self
    }

    /// Set what happens if this request expires unanswered.
    pub fn with_on_expire(mut self, on_expire: OnExpire) -> Self {
        self.on_expire = on_expire;
        self
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
            NodeSpec::Battalion {
                graph,
                state_map,
                restart_on_resume,
            } => f
                .debug_struct("NodeSpec::Battalion")
                // Never the child's whole graph -- only its fingerprint and
                // the two map sizes (CF-FR-14's privacy boundary extends to
                // debug output, not just runtime data flow).
                .field("child_fingerprint", &graph.fingerprint().as_str())
                .field("inputs_len", &state_map.inputs.len())
                .field("outputs_len", &state_map.outputs.len())
                .field("restart_on_resume", restart_on_resume)
                .finish(),
            NodeSpec::Gate {
                request,
                output_field,
            } => f
                .debug_struct("NodeSpec::Gate")
                .field("kind", &request.kind)
                .field("output_field", output_field)
                .finish(),
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
    /// Maximum number of tasks a single `NextStep::Muster` directive may
    /// request (CF-FR-13, D-16, T-23-18). Must be `>= 1`. Enforced at
    /// Directive-receipt time, before any task is dispatched
    /// (`engine::superstep`) — never inside the worker-dispatch loop.
    /// Deliberately excluded from [`WarGraph::fingerprint`] like every
    /// other `EngineLimits` field: raising this limit to let a resumed run
    /// continue is a legitimate operator action.
    pub max_muster_tasks: u32,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_supersteps: 50,
            max_node_visits: 25,
            run_timeout: None,
            max_muster_tasks: 100,
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
    /// Node ids registered via [`WarGraph::add_worker_template`] (CF-03,
    /// D-12): a node that runs only as a `NextStep::Muster` worker task,
    /// never on its own. Seeded into `validate`'s eligible-set worklist
    /// alongside `entry` and `dynamic_targets` (the unfilled seam
    /// `validate_eligible_set`'s own rustdoc names) so a worker template is
    /// not rejected as unreachable despite having no static incoming edge.
    worker_templates: HashSet<NodeId>,
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
            worker_templates: HashSet::new(),
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

    /// Register a node under `id`, marked as a **worker template** (CF-03,
    /// D-12): mirrors [`WarGraph::add_deferred_node`]'s exact shape --
    /// insert the node, then insert the id into the marker set. A worker
    /// template runs only as a `NextStep::Muster` task dispatch, never on
    /// its own: `WarGraph::validate` rejects one declared as an entry point
    /// or as the `to` of any static edge (Task 2), and
    /// `validate_eligible_set` exempts it from the "unreachable from
    /// entry" rejection the same way a [`WarGraph::mark_dynamic_target`]
    /// node already is.
    pub fn add_worker_template(&mut self, id: NodeId, spec: NodeSpec) -> &mut Self {
        self.add_node(id.clone(), spec);
        self.worker_templates.insert(id);
        self
    }

    /// Whether `id` was registered via [`WarGraph::add_worker_template`].
    pub fn is_worker_template(&self, id: &NodeId) -> bool {
        self.worker_templates.contains(id)
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
        self.validate_non_recursive(custom_dispatch, edge_evaluators)?;

        // --- CF-FR-16, D-19: checked LAST -- the deepest, most expensive
        // clause -- so every shallower structural error above is still
        // what a caller sees first for a graph that has one of those too.
        //
        // WR-01 (23-REVIEW.md): the sole recursive descent into a
        // `NodeSpec::Battalion` child's OWN Battalion children happens
        // inside `validate_battalion_children` below, which validates each
        // child's non-recursive structural rules via
        // `child.validate_non_recursive` (never `child.validate`, which
        // would re-enter this same `validate_battalion_children` walk a
        // second time with a truncated single-fingerprint ancestry that
        // can never itself detect anything the explicit, full-ancestry
        // call here does not already catch) -- avoiding the redundant,
        // compounding-per-level re-validation that made a deeply nested,
        // acyclic `Battalion` chain's `validate()` cost exponential in
        // nesting depth rather than linear.
        self.validate_battalion_children(custom_dispatch, edge_evaluators, &[self.fingerprint()])
    }

    /// Every structural check `validate` performs EXCEPT the recursive
    /// descent into `NodeSpec::Battalion` children's own children
    /// (`validate_battalion_children`'s job). Split out so
    /// `validate_battalion_children` can validate a child's own rules
    /// exactly once per child (via this method) while still driving the
    /// recursive-embedding walk itself with the correct, full-ancestry
    /// vector -- see `validate`'s own rustdoc note (WR-01, 23-REVIEW.md)
    /// for why calling `child.validate()` there instead would be
    /// redundant, compounding, exponential-in-depth work for a chain of
    /// nested `Battalion` nodes.
    fn validate_non_recursive(
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
        if self.limits.max_muster_tasks == 0 {
            return Err(EngineError::InvalidLimits {
                reason: "max_muster_tasks must be at least 1".to_string(),
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

        self.validate_muster_prefix_schema_fields()?;
        self.validate_gates()?;
        self.validate_edge_evaluators(edge_evaluators)?;
        self.validate_worker_templates()?;
        self.validate_battalion_state_maps()?;

        self.validate_eligible_set()?;
        self.validate_schedulable()
    }

    /// CF-FR-14 / D-19's StateMap field-existence clause: for every
    /// `NodeSpec::Battalion` node, checks that each `inputs` pair's
    /// `parent` field exists in the PARENT schema and `child` field exists
    /// in the CHILD graph's schema, and that each `outputs` pair's `child`
    /// field exists in the CHILD schema and `parent` field exists in the
    /// PARENT schema. Collects EVERY offender across every Battalion node
    /// before returning, mirroring
    /// [`WarGraph::validate_edge_evaluators`]'s "report the whole problem
    /// at once" discipline -- never failing fast on the first.
    fn validate_battalion_state_maps(&self) -> Result<(), EngineError> {
        let mut offenders: Vec<String> = Vec::new();
        for id in &self.node_order {
            let Some(NodeSpec::Battalion {
                graph: child,
                state_map,
                ..
            }) = self.nodes.get(id)
            else {
                continue;
            };
            let child_schema = child.schema();
            for (parent_field, child_field) in &state_map.inputs {
                if self.schema.field_spec(parent_field).is_none() {
                    offenders.push(format!(
                        "{id}: input parent field '{}' not declared in the parent schema",
                        parent_field.as_str()
                    ));
                }
                if child_schema.field_spec(child_field).is_none() {
                    offenders.push(format!(
                        "{id}: input child field '{}' not declared in the child schema",
                        child_field.as_str()
                    ));
                }
            }
            for (child_field, parent_field) in &state_map.outputs {
                if child_schema.field_spec(child_field).is_none() {
                    offenders.push(format!(
                        "{id}: output child field '{}' not declared in the child schema",
                        child_field.as_str()
                    ));
                }
                if self.schema.field_spec(parent_field).is_none() {
                    offenders.push(format!(
                        "{id}: output parent field '{}' not declared in the parent schema",
                        parent_field.as_str()
                    ));
                }
            }
        }
        if offenders.is_empty() {
            return Ok(());
        }
        offenders.sort();
        Err(EngineError::BattalionStateMapUnknownField {
            fields: offenders,
            reason: "every StateMap-mapped field must exist in its declared schema (CF-FR-14) \
                     -- rename the field or add it to the missing schema"
                .to_string(),
        })
    }

    /// CF-FR-16 / D-19's recursive-embedding + child-validation clause: for
    /// every `NodeSpec::Battalion` node, walks a path-set of CHILD
    /// FINGERPRINTS (never pointer identity -- an immutable `Arc<WarGraph>`
    /// cannot literally self-contain, so this check is defensive against
    /// two structurally identical graphs embedded in a cycle, and it
    /// bounds nesting depth by construction) and rejects a repeated
    /// fingerprint with `EngineError::RecursiveEmbedding`, naming the full
    /// descent path. Otherwise validates the child recursively with the
    /// SAME `custom_dispatch`/`edge_evaluators` registries the parent was
    /// given (D-19) -- extending CF-01's fail-closed contract into
    /// subgraphs, so a `Custom` dispatch rule or edge condition inside a
    /// child fails closed exactly as it does in the parent -- before
    /// descending into that child's OWN Battalion children.
    fn validate_battalion_children(
        &self,
        custom_dispatch: &CustomDispatchResolver,
        edge_evaluators: &EdgeEvaluatorRegistry,
        ancestry: &[GraphFingerprint],
    ) -> Result<(), EngineError> {
        for id in &self.node_order {
            let Some(NodeSpec::Battalion { graph: child, .. }) = self.nodes.get(id) else {
                continue;
            };
            let child_fp = child.fingerprint();
            if ancestry.contains(&child_fp) {
                let mut path = ancestry.to_vec();
                path.push(child_fp);
                let rendered = path
                    .iter()
                    .map(GraphFingerprint::as_str)
                    .collect::<Vec<_>>()
                    .join(" -> ");
                return Err(EngineError::RecursiveEmbedding {
                    path,
                    reason: format!(
                        "battalion node {id} embeds a child whose fingerprint already appears \
                         on its own descent path: {rendered}"
                    ),
                });
            }
            // D-19: the child is validated with the SAME registries, so an
            // unregistered Custom dispatch/edge name inside it fails
            // validation exactly as it would in the parent -- this is what
            // extends CF-01's fail-closed contract into subgraphs, and it
            // also runs the child's own eligible-set/schedulable checks, so
            // a structurally broken child fails the PARENT's validate too.
            //
            // WR-01 (23-REVIEW.md): calls `validate_non_recursive`, NOT
            // `validate` -- `validate` would re-enter this same
            // `validate_battalion_children` walk a second time, seeded with
            // a freshly-truncated single-fingerprint ancestry that can
            // never itself detect a cycle spanning more than the immediate
            // child, making that inner traversal pure redundant work. Only
            // the explicit call below, carrying the correct full-depth
            // `next_ancestry`, can ever find a recursive embedding. Doing
            // both (the old behavior) compounded level over level into
            // O(2^N) total validate() calls for a chain of N nested
            // Battalion nodes rather than O(N).
            child.validate_non_recursive(custom_dispatch, edge_evaluators)?;
            let mut next_ancestry = ancestry.to_vec();
            next_ancestry.push(child_fp);
            child.validate_battalion_children(custom_dispatch, edge_evaluators, &next_ancestry)?;
        }
        Ok(())
    }

    /// CF-03 / D-15's namespace-reservation clause: a Battlefield schema
    /// field whose name starts with the `muster.` prefix is rejected, so
    /// `{muster.payload}`/`{muster.task_key}` in an `InputMapping` template
    /// are unambiguously `NodeContext.muster` references and can never be
    /// shadowed by a same-named schema field the Battlefield would
    /// otherwise resolve them from. Collects every offending field name,
    /// mirroring [`WarGraph::validate_edge_evaluators`]'s "report the whole
    /// problem at once" discipline.
    fn validate_muster_prefix_schema_fields(&self) -> Result<(), EngineError> {
        let mut fields: Vec<String> = self
            .schema
            .fields
            .iter()
            .filter(|f| f.name.as_str().starts_with("muster."))
            .map(|f| f.name.as_str().to_string())
            .collect();
        if fields.is_empty() {
            return Ok(());
        }
        fields.sort_unstable();
        Err(EngineError::MusterPrefixSchemaField {
            fields,
            reason: "the muster. prefix is reserved for {muster.payload}/{muster.task_key} \
                     InputMapping placeholders, resolved from a Muster worker's NodeContext, \
                     never from the Battlefield -- rename the schema field"
                .to_string(),
        })
    }

    /// HITL-01, D-05's Gate well-formedness clause: for every
    /// [`NodeSpec::Gate`] node, `output_field` is required for
    /// `ParleyKind::Approval`/`Choice`/`FreeText` and must be `None` for
    /// `ParleyKind::StateEdit`; when present, the field must exist in the
    /// graph's schema with a type compatible with the Gate's `kind`
    /// (`Approval` -> `Bool` or `String`; `Choice`/`FreeText` -> `String`,
    /// per [`gate_output_field_is_type_compatible`]). An
    /// `on_expire: OnExpire::ResumeWithDefault` value is checked against its
    /// own `kind` through [`validate_parley_value_for_kind`] -- the SAME
    /// per-kind validator `WarEngine::resume_with` applies to a real
    /// submitted response (D-12, T-24-06) -- never a second, weaker check.
    ///
    /// Checked one Gate at a time, in `node_order`, returning the FIRST
    /// violation found -- unlike
    /// [`WarGraph::validate_muster_prefix_schema_fields`]'s "report every
    /// offender" discipline, a single Gate's several distinct rules produce
    /// distinctly-shaped, differently-typed errors that do not collapse
    /// into one `Vec<String>`.
    fn validate_gates(&self) -> Result<(), EngineError> {
        for id in &self.node_order {
            let Some(NodeSpec::Gate {
                request,
                output_field,
            }) = self.nodes.get(id)
            else {
                continue;
            };

            let requires_output_field = !matches!(request.kind, ParleyKind::StateEdit);
            match (requires_output_field, output_field) {
                (true, None) => {
                    return Err(EngineError::GateOutputFieldRequired {
                        node: id.clone(),
                        kind: request.kind.clone(),
                    });
                }
                (false, Some(field)) => {
                    return Err(EngineError::GateOutputFieldMustBeAbsent {
                        node: id.clone(),
                        field: field.clone(),
                    });
                }
                _ => {}
            }

            if let Some(field) = output_field {
                let spec = self.schema.field_spec(field).ok_or_else(|| {
                    EngineError::GateOutputFieldUnknown {
                        node: id.clone(),
                        field: field.clone(),
                    }
                })?;
                if let Err(reason) = gate_output_field_is_type_compatible(&request.kind, spec) {
                    return Err(EngineError::GateOutputFieldTypeIncompatible {
                        node: id.clone(),
                        field: field.clone(),
                        kind: request.kind.clone(),
                        reason,
                    });
                }
            }

            if let OnExpire::ResumeWithDefault(value) = &request.on_expire
                && let Err(reason) =
                    validate_parley_value_for_kind(&request.kind, request.choices.as_deref(), value)
            {
                return Err(EngineError::GateResumeWithDefaultInvalid {
                    node: id.clone(),
                    kind: request.kind.clone(),
                    reason,
                });
            }
        }
        Ok(())
    }

    /// CF-03 / D-12's worker-template well-formedness clause: a node marked
    /// [`WarGraph::add_worker_template`] runs only when mustered (CF-FR-10),
    /// so it may not double as an entry point, and no static edge may
    /// target it (only `NextStep::Muster` ever dispatches it). A worker
    /// template MAY have static outgoing edges (e.g. to a `defer`-marked
    /// aggregator, D-17) -- only its INCOMING side is restricted. Collects
    /// every offender of each clause, mirroring
    /// [`WarGraph::validate_edge_evaluators`]'s discipline.
    fn validate_worker_templates(&self) -> Result<(), EngineError> {
        let mut entry_offenders: Vec<NodeId> = self
            .worker_templates
            .iter()
            .filter(|id| self.entry.contains(id))
            .cloned()
            .collect();
        if !entry_offenders.is_empty() {
            entry_offenders.sort();
            let names = entry_offenders
                .iter()
                .map(NodeId::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(EngineError::WorkerTemplateIsEntry {
                nodes: entry_offenders,
                reason: format!(
                    "worker template(s) declared as entry point(s): {names} -- a worker \
                     template runs only when mustered, never on its own"
                ),
            });
        }

        let mut incoming_offenders: Vec<NodeId> = self
            .worker_templates
            .iter()
            .filter(|id| self.edges.iter().any(|e| &e.to == *id))
            .cloned()
            .collect();
        if !incoming_offenders.is_empty() {
            incoming_offenders.sort();
            let names = incoming_offenders
                .iter()
                .map(NodeId::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(EngineError::WorkerTemplateHasStaticIncomingEdge {
                nodes: incoming_offenders,
                reason: format!(
                    "worker template(s) with a static incoming edge: {names} -- a worker \
                     template runs only as a NextStep::Muster task dispatch, so no static edge \
                     may target it"
                ),
            });
        }

        Ok(())
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

        // The eligible set: entry nodes, dynamic-target-marked nodes and
        // worker-template-marked nodes, expanded by following declared
        // edges to a fixed point (edge CONDITIONS are ignored here -- they
        // are runtime values, and a statically-declared edge is what
        // proves intent for this static check). Worker templates (CF-03,
        // D-12) are reachable only via dynamic fan-out (`NextStep::Muster`),
        // never a static edge, so they are seeded exactly like a
        // `dynamic_target` -- the SAME worklist this function's rustdoc
        // already named as the unfilled seam for exactly this concept. One
        // more future source of eligibility plugs into this same worklist:
        // nodes named as `Route { to }` targets in an eligible node's Aegis
        // `on_error` policy (Phase 25 / CF-FR handler routing) -- which is
        // why this is a fixed point rather than a single pass: a route
        // target discovered late can itself carry outgoing edges that need
        // re-expanding. That concept does not exist in this tree yet;
        // nothing is fabricated here to stand in for it -- it remains an
        // insertion point, not a stub.
        let mut eligible: HashSet<NodeId> = HashSet::new();
        let mut worklist: Vec<NodeId> = Vec::new();
        for id in self
            .entry
            .iter()
            .chain(self.dynamic_targets.iter())
            .chain(self.worker_templates.iter())
        {
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
    /// **`v3` (Phase 23, D-18)** adds three more sections, hashed for the
    /// same reason as everything above: each changes scheduling or merge
    /// semantics, so a resumed run over a graph that changed one of them
    /// without a fingerprint move would silently execute a materially
    /// different graph than the one that was checkpointed.
    ///
    /// - `self.worker_templates` (CF-03): the sorted worker-template node
    ///   id set.
    /// - Per `NodeSpec::Battalion` node, walked in `node_order` (not
    ///   sorted -- `node_order` is already `HashMap`-independent): the node
    ///   id, the child graph's own [`WarGraph::fingerprint`] string (hashing
    ///   the child's fingerprint rather than walking its structure inline
    ///   keeps this compositional and bounded), the `StateMap`'s `inputs`
    ///   and `outputs` pairs in their declared order (semantic, not sorted
    ///   -- a `StateMap` is a list of mappings), and `restart_on_resume`.
    /// - Per `NodeSpec::Paladin` node, also walked in `node_order`: its
    ///   `DirectiveParser` (hashed through its serde-canonical
    ///   representation, matching the existing `EdgeCondition`/
    ///   `DispatchRule` precedent below -- this captures both the parser
    ///   kind and, where `StructuredDirective` carries one, its
    ///   `on_parse_error` in a single field).
    ///
    /// Still deliberately NOT covered: every `EngineLimits` field including
    /// `max_muster_tasks` (RESEARCH.md Pitfall 5) -- raising an operator
    /// limit to let a resumed run continue must not trip `GraphMismatch`
    /// any more under `v3` than it did under `v2`.
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
    /// The three `v3` sections above follow the exact same discipline: every
    /// variable-length field goes through [`push_field`], and the two
    /// variable-length `StateMap` lists are each preceded by their own
    /// element COUNT as a fixed-width 8-byte little-endian integer (written
    /// directly, matching the "has output field" tag byte's existing
    /// fixed-width-marker precedent) before their length-prefixed pairs --
    /// without an explicit count, a pair could be shifted from `outputs`
    /// into `inputs` (or vice versa) without changing the concatenated
    /// bytes, reintroducing exactly the split-ambiguity class CR-01 fixed.
    /// `restart_on_resume` is written as a single fixed-width tag byte for
    /// the same reason the "has output field" marker is.
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

        let mut worker_template_ids: Vec<&NodeId> = self.worker_templates.iter().collect();
        worker_template_ids.sort();

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
        // --- v3 (Phase 23, D-18): three new scheduling/merge-relevant
        // sections, see `fingerprint`'s rustdoc above for the collision
        // analysis behind the StateMap element-count prefixes.
        buf.extend_from_slice(b";worker_templates:");
        for id in &worker_template_ids {
            push_field(&mut buf, id.as_str().as_bytes());
        }
        buf.extend_from_slice(b";battalion:");
        for id in &self.node_order {
            let Some(NodeSpec::Battalion {
                graph: child,
                state_map,
                restart_on_resume,
            }) = self.nodes.get(id)
            else {
                continue;
            };
            push_field(&mut buf, id.as_str().as_bytes());
            push_field(&mut buf, child.fingerprint().as_str().as_bytes());
            buf.extend_from_slice(&(state_map.inputs.len() as u64).to_le_bytes());
            for (parent_field, child_field) in &state_map.inputs {
                push_field(&mut buf, parent_field.as_str().as_bytes());
                push_field(&mut buf, child_field.as_str().as_bytes());
            }
            buf.extend_from_slice(&(state_map.outputs.len() as u64).to_le_bytes());
            for (child_field, parent_field) in &state_map.outputs {
                push_field(&mut buf, child_field.as_str().as_bytes());
                push_field(&mut buf, parent_field.as_str().as_bytes());
            }
            buf.push(if *restart_on_resume { 1 } else { 0 });
        }
        buf.extend_from_slice(b";directive_parsers:");
        for id in &self.node_order {
            let Some(NodeSpec::Paladin {
                directive_parser, ..
            }) = self.nodes.get(id)
            else {
                continue;
            };
            push_field(&mut buf, id.as_str().as_bytes());
            let parser_json = serde_json::to_string(directive_parser).unwrap_or_default();
            push_field(&mut buf, parser_json.as_bytes());
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

/// Whether `spec`'s declared type is compatible with a Gate of `kind`
/// (HITL-01, D-05): `ParleyKind::Approval` accepts a `Bool` or `String`
/// field; `Choice`/`FreeText` accept only a `String` field.
///
/// `FieldSpec` carries no separate type declaration -- a field's "type" is
/// inferred here from its schema `default` value, the only per-field type
/// signal `BattlefieldSchema` carries. A field declared with **no**
/// default cannot be type-checked and is therefore rejected: an
/// `output_field` intended for a Gate must declare a default of the
/// intended type (a Bool or empty-string default for a fresh field is a
/// normal, cheap way to satisfy this).
fn gate_output_field_is_type_compatible(kind: &ParleyKind, spec: &FieldSpec) -> Result<(), String> {
    let Some(default) = spec.default.as_ref() else {
        return Err(
            "the field declares no schema default, so its type cannot be inferred -- declare a \
             default value of the Gate's intended output type"
                .to_string(),
        );
    };
    match kind {
        ParleyKind::Approval => match default {
            serde_json::Value::Bool(_) | serde_json::Value::String(_) => Ok(()),
            other => Err(format!(
                "Approval requires a Bool or String output_field (inferred from its schema \
                 default); found default {other}"
            )),
        },
        ParleyKind::Choice | ParleyKind::FreeText => match default {
            serde_json::Value::String(_) => Ok(()),
            other => Err(format!(
                "{kind:?} requires a String output_field (inferred from its schema default); \
                 found default {other}"
            )),
        },
        // `ParleyKind::StateEdit` never reaches here (`validate_gates`
        // requires `output_field: None` for it, checked before this call
        // is ever made) -- `ParleyKind` is `#[non_exhaustive]`, so a match
        // in this crate still needs a catch-all for a future kind. Fails
        // CLOSED (mirrors `EngineError::UnregisteredEdgeCondition`'s
        // fail-closed stance): a Gate output_field type check for a kind
        // this function does not yet know how to check is rejected, not
        // silently passed.
        other => Err(format!(
            "no output_field type-compatibility rule is registered for ParleyKind {other:?} -- \
             add one alongside the kind"
        )),
    }
}

/// Normalise an Approval [`ParleyResponse::value`](paladin_core::platform::container::parley::ParleyResponse)
/// (or a Gate's `on_expire: OnExpire::ResumeWithDefault` value, checked at
/// graph-validate time) to a canonical `bool` (HITL-FR-05, D-06): a JSON
/// `Bool` passes through; a JSON `String` matches case-insensitively
/// against `true`/`false`/`yes`/`no`/`approve`/`deny`. Every other shape,
/// and every other string, is `None`.
pub(crate) fn normalize_approval_value(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "yes" | "approve" => Some(true),
            "false" | "no" | "deny" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Validate a submitted or defaulted value against its own [`ParleyKind`]
/// (D-12, T-24-06): the SAME validator [`WarGraph::validate`] applies to a
/// Gate's `on_expire: OnExpire::ResumeWithDefault` value at graph-validate
/// time, and the one a later plan's `WarEngine::resume_with` applies to a
/// real submitted [`ParleyResponse::value`](paladin_core::platform::container::parley::ParleyResponse) --
/// never a second, weaker check for either caller.
///
/// - `Approval`: accepted per [`normalize_approval_value`].
/// - `Choice`: must be a JSON string; if `choices` is `Some`, the string
///   must be a member of it.
/// - `FreeText`: must be a JSON string (any content).
/// - `StateEdit`: must deserialize as a
///   [`StateDelta`](paladin_core::platform::container::battlefield::StateDelta)
///   (its own schema-field compatibility is a later plan's concern, once a
///   graph schema is in scope for the check).
pub(crate) fn validate_parley_value_for_kind(
    kind: &ParleyKind,
    choices: Option<&[String]>,
    value: &serde_json::Value,
) -> Result<(), String> {
    match kind {
        ParleyKind::Approval => normalize_approval_value(value).map(|_| ()).ok_or_else(|| {
            format!(
                "Approval value must be a bool or one of true/false/yes/no/approve/deny \
                 (case-insensitive); found {value}"
            )
        }),
        ParleyKind::Choice => {
            let Some(s) = value.as_str() else {
                return Err(format!("Choice value must be a string; found {value}"));
            };
            if let Some(choices) = choices
                && !choices.iter().any(|c| c == s)
            {
                return Err(format!(
                    "Choice value '{s}' is not one of the declared choices: {choices:?}"
                ));
            }
            Ok(())
        }
        ParleyKind::FreeText => {
            if value.is_string() {
                Ok(())
            } else {
                Err(format!("FreeText value must be a string; found {value}"))
            }
        }
        ParleyKind::StateEdit => serde_json::from_value::<
            paladin_core::platform::container::battlefield::StateDelta,
        >(value.clone())
        .map(|_| ())
        .map_err(|e| format!("StateEdit value must deserialize as a StateDelta: {e}")),
        // `ParleyKind` is `#[non_exhaustive]`: a future kind reaching here
        // fails CLOSED rather than being accepted sight-unseen (T-24-06 --
        // an unchecked `ResumeWithDefault` value is an elevation-of-
        // privilege risk) -- a later plan adding the kind also adds its
        // own arm here.
        other => Err(format!(
            "no value validator is registered for ParleyKind {other:?} -- add one alongside \
             the kind"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::directive_parser::OnParseError;
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

    fn field_with_default(name: &str, default: serde_json::Value) -> FieldSpec {
        FieldSpec::new(
            FieldName::new(name).unwrap(),
            DispatchRule::LastWrite,
            Some(default),
            false,
        )
    }

    #[test]
    fn gate_requires_output_field_for_approval_choice_freetext() {
        for kind in [
            ParleyKind::Approval,
            ParleyKind::Choice,
            ParleyKind::FreeText,
        ] {
            let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
            graph.add_node(
                NodeId::new("gate"),
                NodeSpec::gate(
                    GateRequestTemplate::new(kind.clone(), InputMapping::new("go?")),
                    None,
                ),
            );
            graph.add_entry(NodeId::new("gate"));

            let err = graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new(),
                )
                .unwrap_err();
            assert!(
                matches!(
                    &err,
                    EngineError::GateOutputFieldRequired { node, kind: k }
                        if *node == NodeId::new("gate") && *k == kind
                ),
                "expected GateOutputFieldRequired for kind {kind:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn gate_rejects_output_field_for_state_edit() {
        let schema = BattlefieldSchema::new(vec![field_with_default(
            "approved",
            serde_json::json!(false),
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(
            NodeId::new("gate"),
            NodeSpec::gate(
                GateRequestTemplate::new(ParleyKind::StateEdit, InputMapping::new("edit?")),
                Some(FieldName::new("approved").unwrap()),
            ),
        );
        graph.add_entry(NodeId::new("gate"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::GateOutputFieldMustBeAbsent { node, field }
                if node == NodeId::new("gate") && field == FieldName::new("approved").unwrap()
        ));
    }

    #[test]
    fn gate_output_field_must_exist_in_schema() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("gate"),
            NodeSpec::gate(
                GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?")),
                Some(FieldName::new("missing").unwrap()),
            ),
        );
        graph.add_entry(NodeId::new("gate"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::GateOutputFieldUnknown { node, field }
                if node == NodeId::new("gate") && field == FieldName::new("missing").unwrap()
        ));
    }

    #[test]
    fn gate_output_field_type_must_be_compatible() {
        let schema = BattlefieldSchema::new(vec![
            field_with_default("b", serde_json::json!(false)),
            field_with_default("s", serde_json::json!("")),
            field_with_default("n", serde_json::json!(0)),
        ]);

        let cases: &[(ParleyKind, &str, bool)] = &[
            (ParleyKind::Approval, "b", true),
            (ParleyKind::Approval, "s", true),
            (ParleyKind::Approval, "n", false),
            (ParleyKind::Choice, "s", true),
            (ParleyKind::Choice, "n", false),
            (ParleyKind::Choice, "b", false),
            (ParleyKind::FreeText, "s", true),
            (ParleyKind::FreeText, "b", false),
        ];

        for (kind, field, expect_ok) in cases {
            let mut graph = WarGraph::new(schema.clone(), EngineLimits::default());
            graph.add_node(
                NodeId::new("gate"),
                NodeSpec::gate(
                    GateRequestTemplate::new(kind.clone(), InputMapping::new("go?")),
                    Some(FieldName::new(*field).unwrap()),
                ),
            );
            graph.add_entry(NodeId::new("gate"));

            let result = graph.validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            );
            if *expect_ok {
                assert!(
                    result.is_ok(),
                    "expected {kind:?} + field '{field}' to validate, got {result:?}"
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Err(EngineError::GateOutputFieldTypeIncompatible { .. })
                    ),
                    "expected {kind:?} + field '{field}' to be rejected as type-incompatible, \
                     got {result:?}"
                );
            }
        }
    }

    #[test]
    fn gate_resume_with_default_value_is_validated_at_graph_validate_time() {
        let schema = BattlefieldSchema::new(vec![field_with_default(
            "approved",
            serde_json::json!(false),
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let bad_request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"))
            .with_on_expire(OnExpire::ResumeWithDefault(serde_json::json!(42)));
        graph.add_node(
            NodeId::new("gate"),
            NodeSpec::gate(bad_request, Some(FieldName::new("approved").unwrap())),
        );
        graph.add_entry(NodeId::new("gate"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::GateResumeWithDefaultInvalid {
                node,
                kind: ParleyKind::Approval,
                ..
            } if node == NodeId::new("gate")
        ));
    }

    #[test]
    fn gate_with_valid_wiring_passes_validation() {
        // The E2E-2 shape: one Approval Gate node plus two conditional
        // edges is a complete approval gate.
        let schema = BattlefieldSchema::new(vec![field_with_default(
            "approved",
            serde_json::json!(false),
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        graph.add_node(
            NodeId::new("approve"),
            NodeSpec::gate(
                GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("Proceed?")),
                Some(FieldName::new("approved").unwrap()),
            ),
        );
        graph.add_node(
            NodeId::new("act"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_node(
            NodeId::new("cancel"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("approve"),
            to: NodeId::new("act"),
            condition: Some(EdgeCondition::Contains("true".to_string())),
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("approve"),
            to: NodeId::new("cancel"),
            condition: Some(EdgeCondition::Contains("false".to_string())),
        });
        graph.add_entry(NodeId::new("approve"));

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
        // Re-pinned again for Phase 23 D-18's `v3` bump (Plan 23-10): the
        // version tag alone moves the literal even though this fixture has
        // no worker templates, Battalion nodes, or Paladin nodes.
        // `fingerprint_golden_hex_pins_canonical_bytes` (Task 2) is the
        // dedicated golden test guarding future canonicalization changes;
        // this assertion only re-confirms same-input determinism.
        assert_eq!(
            a.as_str(),
            "v3:64e5f08db24bd94d05b337fe56105b3cc4c7ef2f2ee06d94fc9f1f523db1f798"
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
    ///
    /// Re-pinned again for Phase 23 D-18's `v3` bump (Plan 23-10): the
    /// reference graph's construction (`golden_fingerprint_fixture`,
    /// `FingerprintFixtureSpec::default()`) is UNCHANGED -- no worker
    /// templates or Battalion nodes are added to it -- only the version tag
    /// and the (empty) new section markers move the literal, plus the
    /// fixture's existing "worker" Paladin node now contributes a
    /// `directive_parsers:` record for its default `DirectiveParser::PlainOutput`.
    #[test]
    fn fingerprint_golden_hex_pins_canonical_bytes() {
        let graph = golden_fingerprint_fixture(&FingerprintFixtureSpec::default());
        assert_eq!(
            graph.fingerprint().as_str(),
            "v3:a67a12f2947ce17d60d9357fea366ad4539cdeaa174a3a19a4841182725f20e2",
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
    /// `InputMapping` template and `EngineLimits` (including CF-03's
    /// `max_muster_tasks`, RESEARCH Pitfall 5) must NOT move the
    /// fingerprint -- raising a limit or tuning a prompt to let a resumed
    /// run continue is a legitimate operator action. Extended in place
    /// (not a new sibling test) per Plan 23-05's Task 2.
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
                max_muster_tasks: 999,
            },
            ..FingerprintFixtureSpec::default()
        });
        assert_eq!(base.fingerprint(), variant.fingerprint());
    }

    // --- D-18 (Plan 23-10): `v3` hashes three new scheduling-relevant
    // sections -- the worker-template set, each Battalion node's child
    // fingerprint/StateMap/restart_on_resume, and each Paladin node's
    // DirectiveParser kind/on_parse_error -- each written through the
    // existing `push_field` helper, never a delimiter join (22.1 CR-01).

    /// A minimal one-node, one-entry child graph for `NodeSpec::Battalion`
    /// fixtures below, parameterized only by its Function node's id so two
    /// children can be made to differ structurally.
    fn simple_child_graph(node_name: &str) -> Arc<WarGraph> {
        let mut child = WarGraph::new(one_field_schema(), EngineLimits::default());
        child.add_node(
            NodeId::new(node_name),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        child.add_entry(NodeId::new(node_name));
        StdArc::new(child)
    }

    #[test]
    fn fingerprint_version_tag_is_v3() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("solo"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("solo"));

        assert!(graph.fingerprint().as_str().starts_with("v3:"));
    }

    #[test]
    fn fingerprint_differs_when_a_node_is_marked_a_worker_template() {
        let schema = one_field_schema();

        let mut plain = WarGraph::new(schema.clone(), EngineLimits::default());
        plain.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        plain.add_node(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        plain.add_entry(NodeId::new("entry"));

        let mut templated = WarGraph::new(schema, EngineLimits::default());
        templated.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        templated.add_worker_template(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        templated.add_entry(NodeId::new("entry"));

        assert_ne!(plain.fingerprint(), templated.fingerprint());
    }

    #[test]
    fn worker_template_section_is_order_independent() {
        let schema = one_field_schema();

        let mut order_ab = WarGraph::new(schema.clone(), EngineLimits::default());
        order_ab.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        order_ab.add_entry(NodeId::new("entry"));
        order_ab.add_worker_template(
            NodeId::new("worker_a"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        order_ab.add_worker_template(
            NodeId::new("worker_b"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );

        let mut order_ba = WarGraph::new(schema, EngineLimits::default());
        order_ba.add_node(
            NodeId::new("entry"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        order_ba.add_entry(NodeId::new("entry"));
        order_ba.add_worker_template(
            NodeId::new("worker_b"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        order_ba.add_worker_template(
            NodeId::new("worker_a"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );

        assert_eq!(order_ab.fingerprint(), order_ba.fingerprint());
    }

    #[test]
    fn fingerprint_differs_when_an_embedded_child_graph_differs() {
        let schema = one_field_schema();

        let mut parent_a = WarGraph::new(schema.clone(), EngineLimits::default());
        parent_a.add_node(
            NodeId::new("sub"),
            NodeSpec::battalion(simple_child_graph("child_a"), StateMap::new()),
        );
        parent_a.add_entry(NodeId::new("sub"));

        let mut parent_b = WarGraph::new(schema, EngineLimits::default());
        parent_b.add_node(
            NodeId::new("sub"),
            NodeSpec::battalion(simple_child_graph("child_b"), StateMap::new()),
        );
        parent_b.add_entry(NodeId::new("sub"));

        assert_ne!(parent_a.fingerprint(), parent_b.fingerprint());
    }

    #[test]
    fn fingerprint_differs_when_a_state_map_differs() {
        let schema = one_field_schema();
        let child = simple_child_graph("child");

        let mut with_empty_map = WarGraph::new(schema.clone(), EngineLimits::default());
        with_empty_map.add_node(
            NodeId::new("sub"),
            NodeSpec::battalion(StdArc::clone(&child), StateMap::new()),
        );
        with_empty_map.add_entry(NodeId::new("sub"));

        let mut with_mapped = WarGraph::new(schema, EngineLimits::default());
        with_mapped.add_node(
            NodeId::new("sub"),
            NodeSpec::battalion(
                StdArc::clone(&child),
                StateMap::new().with_input(
                    FieldName::new("result").unwrap(),
                    FieldName::new("result").unwrap(),
                ),
            ),
        );
        with_mapped.add_entry(NodeId::new("sub"));

        assert_ne!(with_empty_map.fingerprint(), with_mapped.fingerprint());
    }

    #[test]
    fn fingerprint_differs_when_restart_on_resume_differs() {
        let schema = one_field_schema();
        let child = simple_child_graph("child");

        let mut no_restart = WarGraph::new(schema.clone(), EngineLimits::default());
        no_restart.add_node(
            NodeId::new("sub"),
            NodeSpec::Battalion {
                graph: StdArc::clone(&child),
                state_map: StateMap::new(),
                restart_on_resume: false,
            },
        );
        no_restart.add_entry(NodeId::new("sub"));

        let mut restart = WarGraph::new(schema, EngineLimits::default());
        restart.add_node(
            NodeId::new("sub"),
            NodeSpec::Battalion {
                graph: child,
                state_map: StateMap::new(),
                restart_on_resume: true,
            },
        );
        restart.add_entry(NodeId::new("sub"));

        assert_ne!(no_restart.fingerprint(), restart.fingerprint());
    }

    #[test]
    fn fingerprint_differs_when_a_directive_parser_kind_differs() {
        let schema = one_field_schema();

        let mut plain = WarGraph::new(schema.clone(), EngineLimits::default());
        plain.add_node(
            NodeId::new("worker"),
            NodeSpec::paladin(
                make_fixture_paladin("worker", "prompt", "gpt-4"),
                InputMapping::new("prompt"),
                FieldName::new("result").unwrap(),
            ),
        );
        plain.add_entry(NodeId::new("worker"));

        let mut structured = WarGraph::new(schema, EngineLimits::default());
        structured.add_node(
            NodeId::new("worker"),
            NodeSpec::paladin_with_directive_parser(
                make_fixture_paladin("worker", "prompt", "gpt-4"),
                InputMapping::new("prompt"),
                FieldName::new("result").unwrap(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        structured.add_entry(NodeId::new("worker"));

        assert_ne!(plain.fingerprint(), structured.fingerprint());
    }

    #[test]
    fn fingerprint_differs_when_on_parse_error_differs() {
        let schema = one_field_schema();

        let mut fail_run = WarGraph::new(schema.clone(), EngineLimits::default());
        fail_run.add_node(
            NodeId::new("worker"),
            NodeSpec::paladin_with_directive_parser(
                make_fixture_paladin("worker", "prompt", "gpt-4"),
                InputMapping::new("prompt"),
                FieldName::new("result").unwrap(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        fail_run.add_entry(NodeId::new("worker"));

        let mut fallback = WarGraph::new(schema, EngineLimits::default());
        fallback.add_node(
            NodeId::new("worker"),
            NodeSpec::paladin_with_directive_parser(
                make_fixture_paladin("worker", "prompt", "gpt-4"),
                InputMapping::new("prompt"),
                FieldName::new("result").unwrap(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FallbackPlain,
                },
            ),
        );
        fallback.add_entry(NodeId::new("worker"));

        assert_ne!(fail_run.fingerprint(), fallback.fingerprint());
    }

    #[test]
    fn engine_limits_default_is_50_and_25() {
        let limits = EngineLimits::default();
        assert_eq!(limits.max_supersteps, 50);
        assert_eq!(limits.max_node_visits, 25);
    }

    #[test]
    fn engine_limits_default_max_muster_tasks_is_100() {
        assert_eq!(EngineLimits::default().max_muster_tasks, 100);
    }

    #[test]
    fn validate_rejects_zero_max_muster_tasks() {
        let graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_muster_tasks: 0,
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

    // --- CF-03, D-12: worker-template well-formedness (Plan 23-05).

    #[test]
    fn worker_template_is_exempt_from_the_unreachable_rejection() {
        // A worker template with no static incoming edges -- reachable only
        // via runtime NextStep::Muster dispatch -- must still validate,
        // exactly like a WarGraph::mark_dynamic_target node.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("planner"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_worker_template(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("planner"));

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
        assert!(graph.is_worker_template(&NodeId::new("worker")));
    }

    #[test]
    fn worker_template_may_not_be_an_entry_node() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_worker_template(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_entry(NodeId::new("worker"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::WorkerTemplateIsEntry { .. }));
    }

    #[test]
    fn worker_template_may_not_have_static_incoming_edges() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("planner"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_worker_template(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("planner"),
            to: NodeId::new("worker"),
            condition: None,
        });
        graph.add_entry(NodeId::new("planner"));

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::WorkerTemplateHasStaticIncomingEdge { .. }
        ));
    }

    #[test]
    fn worker_template_may_have_static_outgoing_edges() {
        // A worker template MAY route to a defer-marked aggregator (D-17) --
        // only its INCOMING side is restricted.
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            NodeId::new("planner"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_worker_template(
            NodeId::new("worker"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_deferred_node(
            NodeId::new("aggregator"),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("worker"),
            to: NodeId::new("aggregator"),
            condition: None,
        });
        graph.add_entry(NodeId::new("planner"));

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
    fn schema_field_named_with_the_muster_prefix_is_rejected() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("muster.payload").unwrap(),
            DispatchRule::LastWrite,
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
            EngineError::MusterPrefixSchemaField { fields, .. } => {
                assert_eq!(fields, vec!["muster.payload".to_string()]);
            }
            other => panic!("expected MusterPrefixSchemaField, got {other:?}"),
        }
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
            None,
            1,
            &paladin_port,
            &trace,
            &interceptors,
            &None,
            None,
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

    // --- Plan 23-08: StateMap and recursive-embedding validation ---------

    fn trivial_graph_named(node_name: &str) -> WarGraph {
        let mut g = WarGraph::new(one_field_schema(), EngineLimits::default());
        g.add_node(
            NodeId::new(node_name),
            NodeSpec::Function(StdArc::new(NoopNode)),
        );
        g.add_entry(NodeId::new(node_name));
        g
    }

    fn trivial_graph() -> WarGraph {
        trivial_graph_named("only")
    }

    #[test]
    fn state_map_input_naming_an_unknown_parent_field_fails_validation() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_input(
            FieldName::new("ghost_parent").unwrap(),
            FieldName::new("result").unwrap(),
        );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::BattalionStateMapUnknownField { fields, .. } => {
                assert!(
                    fields.iter().any(|f| f.contains("ghost_parent")),
                    "fields: {fields:?}"
                );
            }
            other => panic!("expected BattalionStateMapUnknownField, got {other:?}"),
        }
    }

    #[test]
    fn state_map_input_naming_an_unknown_child_field_fails_validation() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_input(
            FieldName::new("result").unwrap(),
            FieldName::new("ghost_child").unwrap(),
        );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::BattalionStateMapUnknownField { fields, .. } => {
                assert!(
                    fields.iter().any(|f| f.contains("ghost_child")),
                    "fields: {fields:?}"
                );
            }
            other => panic!("expected BattalionStateMapUnknownField, got {other:?}"),
        }
    }

    #[test]
    fn state_map_output_naming_an_unknown_child_field_fails_validation() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(
            FieldName::new("ghost_child").unwrap(),
            FieldName::new("result").unwrap(),
        );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::BattalionStateMapUnknownField { fields, .. } => {
                assert!(
                    fields.iter().any(|f| f.contains("ghost_child")),
                    "fields: {fields:?}"
                );
            }
            other => panic!("expected BattalionStateMapUnknownField, got {other:?}"),
        }
    }

    #[test]
    fn state_map_output_naming_an_unknown_parent_field_fails_validation() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(
            FieldName::new("result").unwrap(),
            FieldName::new("ghost_parent").unwrap(),
        );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::BattalionStateMapUnknownField { fields, .. } => {
                assert!(
                    fields.iter().any(|f| f.contains("ghost_parent")),
                    "fields: {fields:?}"
                );
            }
            other => panic!("expected BattalionStateMapUnknownField, got {other:?}"),
        }
    }

    #[test]
    fn every_offending_mapped_field_is_reported_not_just_the_first() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new()
            .with_input(
                FieldName::new("ghost_parent_1").unwrap(),
                FieldName::new("result").unwrap(),
            )
            .with_input(
                FieldName::new("result").unwrap(),
                FieldName::new("ghost_child_1").unwrap(),
            )
            .with_output(
                FieldName::new("ghost_child_2").unwrap(),
                FieldName::new("result").unwrap(),
            )
            .with_output(
                FieldName::new("result").unwrap(),
                FieldName::new("ghost_parent_2").unwrap(),
            );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        match err {
            EngineError::BattalionStateMapUnknownField { fields, .. } => {
                assert_eq!(
                    fields.len(),
                    4,
                    "all four offenders must be reported at once: {fields:?}"
                );
            }
            other => panic!("expected BattalionStateMapUnknownField, got {other:?}"),
        }
    }

    #[test]
    fn child_graph_is_validated_with_the_parents_registries() {
        let mut child = WarGraph::new(one_field_schema(), EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        child.add_node(a.clone(), NodeSpec::Function(StdArc::new(NoopNode)));
        child.add_node(b.clone(), NodeSpec::Function(StdArc::new(NoopNode)));
        child.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: Some(EdgeCondition::Custom("special".to_string())),
        });
        child.add_entry(a);

        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_entry(sub);

        // Unregistered on either side: fails, because the child is
        // validated with the SAME registry the parent was given (D-19).
        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::UnregisteredEdgeCondition { .. }),
            "got {err:?}"
        );

        // Registered on the PARENT's own registry: the child validates too.
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
        let mut registry = EdgeEvaluatorRegistry::new();
        registry.register("special", StdArc::new(AlwaysTrue));
        assert!(
            parent
                .validate(&CustomDispatchResolver::new(), &registry)
                .is_ok()
        );
    }

    #[test]
    fn child_graph_with_its_own_structural_defect_fails_the_parent_validate() {
        let child_schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("special").unwrap(),
            DispatchRule::Custom("missing".to_string()),
            None,
            false,
        )]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        child.add_node(NodeId::new("a"), NodeSpec::Function(StdArc::new(NoopNode)));
        child.add_entry(NodeId::new("a"));

        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_entry(sub);

        let err = parent
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .unwrap_err();
        assert!(
            matches!(
                err,
                EngineError::Battlefield(BattlefieldError::CustomDispatchNotRegistered { .. })
            ),
            "got {err:?}"
        );
    }

    // NOTE (Plan 23-10, D-18): `v3`'s new `battalion:` section correctly
    // distinguishes a `NodeSpec::Battalion` node from a `NodeSpec::Function`
    // node -- v2 taken alone did not (both got the same "no output field"
    // tag byte), which is exactly the coincidental blindness that let the
    // two tests below previously fake self-containment by embedding a
    // structurally-"identical" (under v2's blind spot) leaf graph and
    // relying on it hashing the same as its Battalion-wrapping parent.
    // Under `v3`'s now depth-sensitive, Merkle-style encoding (each
    // Battalion node hashes its child's OWN fingerprint string), a
    // descendant's fingerprint can never algebraically equal an ancestor's
    // via honest finite construction: the ancestor's hash is partly
    // DEFINED BY the descendant's hash through the wrapping chain, so
    // requiring equality asks for a hash fixed point, which a
    // collision-resistant hash function does not yield except by
    // astronomically unlikely accident. That is a genuine correctness
    // improvement (RecursiveEmbedding can no longer misfire OR be
    // legitimately reachable through an encoding blind spot), but it means
    // these two tests must now exercise `validate_battalion_children`'s
    // ancestry-collision check directly, seeding a hand-built ancestry
    // list, rather than manufacturing an impossible hash coincidence
    // end-to-end through the public `validate` entry point.

    #[test]
    fn directly_recursive_embedding_is_rejected() {
        // Simulates self-containment -- an immutable Arc<WarGraph> cannot
        // literally self-contain (D-19's own defensive rationale for a
        // fingerprint path-set walk rather than pointer identity) -- by
        // calling the private ancestry-walk helper with a starting ancestry
        // that already contains the embedded child's own fingerprint, as if
        // an outer caller had already visited a graph with that exact
        // fingerprint. The collision fires on the FIRST node checked (no
        // recursion needed), matching "direct" self-reference.
        let child = trivial_graph_named("child");
        let child_fp = child.fingerprint();

        let mut outer = WarGraph::new(one_field_schema(), EngineLimits::default());
        outer.add_node(
            NodeId::new("only"),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        outer.add_entry(NodeId::new("only"));

        let err = outer
            .validate_battalion_children(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
                &[child_fp],
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::RecursiveEmbedding { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn transitively_recursive_embedding_is_rejected() {
        // A -> B -> collision: the starting ancestry is seeded with the
        // GRANDCHILD's fingerprint (simulating a third level up the real
        // descent chain), so the collision is only discovered after
        // `validate_battalion_children` has recursed one level down from A
        // into B -- exercising the same multi-level `next_ancestry`
        // accumulation a genuine multi-level embedding would use, without
        // an impossible self-referential fingerprint coincidence (see
        // `directly_recursive_embedding_is_rejected`'s comment above).
        let grandchild = trivial_graph_named("grandchild");
        let grandchild_fp = grandchild.fingerprint();

        let mut b = WarGraph::new(one_field_schema(), EngineLimits::default());
        b.add_node(
            NodeId::new("mid"),
            NodeSpec::battalion(Arc::new(grandchild), StateMap::new()),
        );
        b.add_entry(NodeId::new("mid"));

        let mut a = WarGraph::new(one_field_schema(), EngineLimits::default());
        a.add_node(
            NodeId::new("only"),
            NodeSpec::battalion(Arc::new(b), StateMap::new()),
        );
        a.add_entry(NodeId::new("only"));

        let err = a
            .validate_battalion_children(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
                &[grandchild_fp],
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::RecursiveEmbedding { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn deep_but_acyclic_nesting_validates() {
        let level3 = trivial_graph_named("level3_leaf");
        let mut level2 = WarGraph::new(one_field_schema(), EngineLimits::default());
        level2.add_node(
            NodeId::new("level2_node"),
            NodeSpec::battalion(Arc::new(level3), StateMap::new()),
        );
        level2.add_entry(NodeId::new("level2_node"));

        let mut level1 = WarGraph::new(one_field_schema(), EngineLimits::default());
        level1.add_node(
            NodeId::new("level1_node"),
            NodeSpec::battalion(Arc::new(level2), StateMap::new()),
        );
        level1.add_entry(NodeId::new("level1_node"));

        assert!(
            level1
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
    }

    #[test]
    fn state_map_mapping_one_child_field_to_two_parent_fields_is_accepted() {
        let child = trivial_graph();
        let parent_schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("result").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(
                FieldName::new("result2").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
        ]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new()
            .with_output(
                FieldName::new("result").unwrap(),
                FieldName::new("result").unwrap(),
            )
            .with_output(
                FieldName::new("result").unwrap(),
                FieldName::new("result2").unwrap(),
            );
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        assert!(
            parent
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
    }

    #[test]
    fn state_map_with_empty_inputs_is_accepted() {
        let child = trivial_graph();
        let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(
            FieldName::new("result").unwrap(),
            FieldName::new("result").unwrap(),
        );
        assert!(state_map.inputs.is_empty());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        assert!(
            parent
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok()
        );
    }
}
