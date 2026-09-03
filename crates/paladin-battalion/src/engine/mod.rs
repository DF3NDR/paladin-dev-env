//! War Engine — Superstep Execution Engine
//!
//! This module implements the execution engine for [`WarGraph`]s: typed,
//! potentially-cyclic graphs of [`StateNode`]s whose shared state is a
//! [`Battlefield`] (`paladin-core`), automatically checkpointed as a
//! [`Waypoint`] after every superstep through a [`WaypointPort`]
//! (`paladin-ports`).
//!
//! Phase 22 Plan 01 proved the tracer: a single-entry, single-`Function`-node,
//! zero-edge graph, run through [`WarEngine::start`], checkpointed as exactly
//! one `Waypoint`, and resumed by a freshly constructed `WarEngine` with zero
//! re-execution. Plan 05 expands this into the real superstep engine
//! (`engine::superstep`): the general multi-node loop with cycles, snapshot
//! isolation, bounded concurrency, and both engine limits. Dispatch-conflict
//! surfacing, precise join/defer semantics and full `resume` are later
//! plans' expansion (22-07, 22-08) — this module's types are shaped so that
//! expansion does not require changing these signatures.
//!
//! Submodules:
//! - [`bridges`] — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
//!   (ENG-FR-19, X-03): additive legacy bridges reproducing
//!   `FormationExecutionService`/`PhalanxExecutionService`/
//!   `CampaignExecutionService`'s data flow byte for byte, without touching
//!   any of those legacy services.
//! - [`graph`] — `WarGraph`, `NodeSpec`, `EdgeSpec`, `EngineLimits`, and
//!   `WarGraph::validate`/`fingerprint`.
//! - [`input_mapping`] — `InputMapping`, `InputMappingError`: the X-03
//!   string bridge a `NodeSpec::Paladin` node renders its input through.
//! - [`node`] — `StateNode`, `NodeContext`, `NodeError`.
//! - [`dispatch_registry`] — `DispatchRegistry`, the engine-owned
//!   `DispatchRule::Custom` name -> closure registration (ENG-FR-09).
//! - [`hooks`] — `TraceDispatcher` (ENG-FR-21's bounded, drop-oldest
//!   `TraceSink` forwarder), `NodeInterceptor`/`InterceptDecision`
//!   (ENG-FR-22's ordered, empty-by-default chain). Both are seams with no
//!   consumer yet (Docs 05, 07); ENG-FR-23's cancellation-to-`Halted` path
//!   lives inline in `superstep`/`WarEngine` since it needs no dedicated
//!   type beyond `tokio_util::sync::CancellationToken`.
//! - `superstep` (private) — the superstep loop `start`/`resume` reduce to.
//! - `test_support` (`#[cfg(test)]`) — `RecordingWaypointStore`,
//!   `RecordingPaladinPort` and `CountingFunctionNode`, the doubles this and
//!   later engine plans assert against.

pub mod bridges;
pub mod dispatch_registry;
pub mod graph;
pub mod hooks;
pub mod input_mapping;
pub mod node;
mod superstep;
#[cfg(test)]
pub(crate) mod test_support;

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeEvaluatorRegistry};
#[cfg(test)]
use paladin_core::platform::container::battlefield::CustomDispatchResolver;
use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
#[cfg(test)]
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeId, ParleyRequest, ThreadId, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceSink};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

pub use bridges::{CAMPAIGN_FAN_IN_SEPARATOR, campaign_node_ids, dedicated_output_field};
pub use dispatch_registry::DispatchRegistry;
pub use graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
pub use hooks::{InterceptDecision, NodeInterceptor, TraceDispatcher};
pub use input_mapping::{InputMapping, InputMappingError};
pub use node::{NodeContext, NodeError, StateNode};

/// Whether a `WaypointPort::save` failure fails the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaypointDurability {
    /// A `save` failure fails the run with `EngineError::WaypointWrite`
    /// (default; durable-by-default, ENG-FR-11).
    #[default]
    Strict,
    /// A `save` failure is logged as a warning and the run continues. **Do
    /// not** select this in any example, doc snippet, config template or
    /// shared test helper: a failed checkpoint write silently downgrades to
    /// a logged warning, and a whole superstep of work can be lost with no
    /// other signal. Opt in explicitly and locally, only where the
    /// consequence is understood and accepted.
    BestEffort,
}

/// The outcome of a `WarEngine::start` or `WarEngine::resume` call.
#[derive(Debug)]
pub enum RunOutcome {
    /// The run finished normally.
    Completed {
        /// The final Battlefield state.
        final_state: Battlefield,
        /// The waypoint written for the run's final superstep.
        waypoint: WaypointId,
    },
    /// The run is paused awaiting external input (Doc 03).
    AwaitingInput {
        /// The outstanding input request.
        parley: ParleyRequest,
        /// The waypoint recording the pause.
        waypoint: WaypointId,
    },
    /// The run was gracefully halted: a `CancellationToken` was observed
    /// cancelled at a superstep boundary (ENG-FR-23). The in-flight
    /// superstep, if any, was allowed to finish and merge before the
    /// `Halted` `Waypoint` was persisted, so it is always a consistent
    /// restart point — `WarEngine::resume`/`resume_with_options` can
    /// continue from it exactly as from a `Running` waypoint (Doc 03 lands
    /// the dedicated pause/resume API this shares its plumbing with).
    Halted {
        /// The waypoint recording the halt.
        waypoint: WaypointId,
    },
    /// The run failed — a bounded-iteration limit was hit, or a node's
    /// execution or the merge it fed returned an error. A Waypoint carrying
    /// `WaypointStatus::Failed` has already been persisted (subject to
    /// `WaypointDurability`) by the time this variant is returned.
    Failed {
        /// The engine error that caused the run to fail.
        error: EngineError,
        /// The waypoint just written recording the failure, if persistence
        /// was attempted.
        waypoint: Option<WaypointId>,
    },
}

/// Errors returned by [`WarEngine::start`] and [`WarEngine::resume`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The run's superstep count reached `EngineLimits::max_supersteps`.
    #[error("recursion limit exceeded: {limit} supersteps for thread {thread_id}")]
    RecursionLimitExceeded {
        /// The configured limit that was hit.
        limit: u64,
        /// The thread whose run hit the limit.
        thread_id: ThreadId,
    },

    /// A single node exceeded `EngineLimits::max_node_visits` within one run.
    #[error("node visit limit exceeded: node {node} exceeded {limit} visits")]
    NodeVisitLimitExceeded {
        /// The node that exceeded its visit limit.
        node: NodeId,
        /// The configured limit that was hit.
        limit: u32,
    },

    /// `WarGraph::validate` rejected the graph's limits.
    #[error("invalid engine limits: {reason}")]
    InvalidLimits {
        /// Why the limits were rejected.
        reason: String,
    },

    /// `WarGraph::validate` found an edge or entry point naming a `NodeId`
    /// not present in the graph's node map.
    #[error("unknown node referenced in graph: {0}")]
    UnknownNode(NodeId),

    /// `WarGraph::validate` found one or more declared nodes outside the
    /// **eligible set** (ENG-FR-02a / BUG-02): the fixed point of nodes
    /// reachable from `entry` over static edges, unioned with nodes marked
    /// [`graph::WarGraph::mark_dynamic_target`]. Carries EVERY offending
    /// node in one error, in the graph's registration order, rather than
    /// one variant per node, so a caller sees the whole problem at once
    /// rather than fixing one stranded node per validate/retry cycle.
    ///
    /// Checked last among `validate`'s clauses, so any earlier, more
    /// specific structural error (limits, an unknown node, an unregistered
    /// custom dispatch name) is still what a caller sees first.
    #[error("unreachable node(s) in graph: {reason}")]
    UnreachableNode {
        /// Every declared node outside the eligible set, in the graph's
        /// registration order (deterministic, never `HashMap` order).
        nodes: Vec<NodeId>,
        /// Explains the eligible-set rule and names the two ways to fix an
        /// ordinary stranded node: make it reachable from entry via a
        /// static edge, or mark it `dynamic_target`. For a graph that
        /// declares nodes but never calls `add_entry` at all, names the
        /// absent entry point as the cause instead, since every node is
        /// then trivially unreachable and listing them individually would
        /// bury the actual mistake.
        reason: String,
    },

    /// An `EdgeCondition::Regex` pattern failed to compile.
    #[error("invalid edge condition: {reason}")]
    InvalidEdgeCondition {
        /// Why the condition was rejected.
        reason: String,
    },

    /// Persisting a Waypoint failed under `WaypointDurability::Strict`.
    #[error("failed to persist waypoint: {source}")]
    WaypointWrite {
        /// The underlying port error.
        #[source]
        source: WaypointError,
    },

    /// Reading a Waypoint back from the port failed.
    #[error("failed to read waypoint: {source}")]
    WaypointRead {
        /// The underlying port error.
        #[source]
        source: WaypointError,
    },

    /// `resume` found a stored Waypoint whose `graph_fingerprint` does not
    /// match the graph passed to `resume` (ENG-FR-14).
    #[error("graph fingerprint mismatch: expected {expected}, got {got}")]
    GraphMismatch {
        /// The fingerprint of the graph passed to `resume`.
        expected: GraphFingerprint,
        /// The fingerprint stored on the latest Waypoint.
        got: GraphFingerprint,
    },

    /// `resume` was called for a thread with no stored Waypoint.
    #[error("thread not found: {0}")]
    ThreadNotFound(ThreadId),

    /// A Battlefield operation (merge, typed accessor, required-field
    /// check) failed.
    #[error("battlefield error: {0}")]
    Battlefield(#[from] BattlefieldError),

    /// A node's execution returned an error.
    #[error("node execution error: {0}")]
    Node(#[from] NodeError),

    /// `DispatchRegistry::register` was asked to register a custom
    /// dispatch rule under a name that collides with a built-in
    /// `DispatchRule` variant name (ENG-FR-09). Rejected at registration so
    /// a schema author cannot believe they have overridden e.g.
    /// `LastWrite` when they have not.
    #[error("cannot register custom dispatch rule '{name}': reserved built-in rule name")]
    ReservedDispatchName {
        /// The rejected registration name.
        name: String,
    },

    /// A `NodeSpec::Paladin` node's `InputMapping::render` call failed: an
    /// undeclared field, or a declared field with no value and no default
    /// (X-03).
    #[error("input mapping error: {0}")]
    InputMapping(#[from] input_mapping::InputMappingError),

    /// `resume` with `allow_graph_change` restored a Vanguard `NodeId` the
    /// new graph does not declare (ENG-FR-14's explicit-override path).
    /// Never silently dropped: a resume that continued without a vanguard
    /// node the caller expected to run would look like a successful resume
    /// that quietly skipped work.
    #[error("resume vanguard node missing from the new graph: {node}")]
    VanguardNodeMissing {
        /// The restored vanguard node absent from the new graph.
        node: NodeId,
    },

    /// `WarGraph::validate` found one or more declared nodes forming a
    /// component that can never receive a fired edge from outside itself,
    /// and that carries no declared runtime-entry marker (D-03, the guard
    /// half of BUG-03's fix). ENG-FR-06a's starvation-release fallback pass
    /// in `compute_next_vanguard` bootstraps a cycle's first execution ONLY
    /// when at least one of its members is fed by an edge from a node
    /// outside the cycle -- a component with no such external feed can
    /// never take its first turn, no matter how many supersteps run.
    /// Carries EVERY offending node in one error, in this graph's
    /// registration order (`WarGraph::node_order`, never `HashMap` order),
    /// mirroring [`EngineError::UnreachableNode`]'s "report the whole
    /// problem at once" discipline.
    ///
    /// Exists so a graph shape the starvation release can never schedule
    /// fails before any node executes, rather than running to a false
    /// `Completed` -- the same class of silent lie BUG-02's eligible-set
    /// check ended for static unreachability, applied here to a dynamic
    /// scheduling limitation the eligible-set check cannot see (a cycle
    /// fed only from within itself IS statically reachable from `entry`
    /// once any one of its members is, so `UnreachableNode` never fires on
    /// it).
    ///
    /// Checked LAST among `validate`'s clauses -- after
    /// `validate_eligible_set` -- so for a graph with no declared
    /// runtime-entry marker the eligible-set clause is what a caller sees
    /// first; this clause is defence-in-depth against a future relaxation
    /// of that clause or a misapplied [`graph::WarGraph::mark_dynamic_target`]
    /// marker, not the primary guard for an ordinary stranded node.
    ///
    /// Distinct from [`EngineError::StarvedNodeAtCompletion`]: this is a
    /// validate-time "this shape cannot be scheduled" failure, decided
    /// before any node runs; that is a run-end "the engine's own invariant
    /// broke" failure, decided after a run's own Vanguard emptied. The two
    /// never share a message because they are different failure classes
    /// caught at different times for different reasons.
    #[error("unschedulable cycle in graph: {reason}")]
    UnschedulableCycle {
        /// Every node in an externally-unfed component, in the graph's
        /// registration order (deterministic, never `HashMap` order).
        nodes: Vec<NodeId>,
        /// Explains the fixpoint rule and names the two ways to fix an
        /// offending component: feed it from an entry-reachable node
        /// outside the component, or mark its entry point
        /// [`graph::WarGraph::mark_dynamic_target`].
        reason: String,
    },

    /// `superstep::run`'s run-end truthful-outcome check (D-04) found a
    /// non-dead, declared node still holding an unconsumed fired incoming
    /// edge at the exact moment the run was about to report
    /// `RunOutcome::Completed`. The engine refuses to report `Completed`
    /// here: a node with work waiting that the scheduler never dispatched
    /// means the scheduler's own invariant -- every node that can fire is
    /// eventually run before `Completed` is reported -- broke, and BUG-03's
    /// entire premise is that such breakage must be loud, not silent.
    ///
    /// This check is deliberately INDEPENDENT of `compute_next_vanguard`
    /// and the ENG-FR-06a starvation-release pass it calls: it re-derives
    /// its answer from the same `Frontier` state those passes already
    /// updated, rather than re-invoking their logic, so a future regression
    /// in the release mechanism cannot silently satisfy both the release
    /// and this check at once. Carries EVERY such node in one error, in
    /// this graph's registration order, mirroring
    /// [`EngineError::UnreachableNode`] and
    /// [`EngineError::UnschedulableCycle`].
    ///
    /// Distinct from [`EngineError::UnschedulableCycle`]: that is a
    /// validate-time "this shape cannot be scheduled" failure, decided
    /// before any node runs; this is a run-end "the engine's own invariant
    /// broke" failure, decided after a run's own Vanguard emptied. The two
    /// never share a message because they are different failure classes
    /// caught at different times for different reasons.
    #[error("starved node(s) at completion: {reason}")]
    StarvedNodeAtCompletion {
        /// Every node holding an unconsumed fired incoming edge at the
        /// moment `Completed` was about to be reported, in the graph's
        /// registration order (deterministic, never `HashMap` order).
        nodes: Vec<NodeId>,
        /// Names the invariant that broke: a node in the eligible set held
        /// an unconsumed fired incoming edge while the Vanguard was empty
        /// (ENG-FR-06a).
        reason: String,
    },

    /// `WarGraph::validate` found one or more edges carrying
    /// `EdgeCondition::Custom(name)` with no evaluator registered via
    /// [`WarEngine::with_edge_evaluator`] (BUG-01, CF-FR-02). Checked
    /// before any node executes, replacing the pre-fix behavior of
    /// silently evaluating an unregistered `Custom` condition as `true`.
    /// Carries EVERY offending name, sorted and deduplicated, mirroring
    /// [`EngineError::UnreachableNode`]'s "report the whole problem at
    /// once" discipline.
    #[error("unregistered custom edge condition(s): {}", names.join(", "))]
    UnregisteredEdgeCondition {
        /// Every unregistered `EdgeCondition::Custom` name, sorted and
        /// deduplicated.
        names: Vec<String>,
    },

    /// A registered `EdgeConditionEvaluator::evaluate` call returned `Err`
    /// while resolving an `EdgeCondition::Custom` edge (BUG-01, CF-FR-03).
    /// Never treated as a default branch: the run fails, naming the edge
    /// and the evaluator that failed.
    #[error("edge evaluator '{evaluator}' failed for edge {from} -> {to}: {source}")]
    EdgeEvaluatorFailed {
        /// The edge's source node.
        from: NodeId,
        /// The edge's target node.
        to: NodeId,
        /// The registered evaluator's name.
        evaluator: String,
        /// The evaluator's own structured error.
        #[source]
        source: crate::edge_evaluator::EdgeEvaluatorError,
    },
}

/// Options controlling [`WarEngine::resume_with_options`]'s behavior.
///
/// The default (`allow_graph_change: false`, matching [`WarEngine::resume`])
/// is the safe choice: a graph-fingerprint mismatch always fails resume
/// unless the caller explicitly opts into continuing against a changed
/// graph.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumeOptions {
    /// When `true`, a graph-fingerprint mismatch does not fail `resume` on
    /// its own (ENG-FR-14's explicit override); a restored vanguard
    /// `NodeId` absent from the new graph still fails, with
    /// `EngineError::VanguardNodeMissing`.
    pub allow_graph_change: bool,
}

/// Executes [`WarGraph`]s: runs nodes, merges their deltas into the shared
/// [`Battlefield`], and automatically checkpoints a [`Waypoint`] after every
/// superstep through `W: WaypointPort` (ENG-FR-11).
pub struct WarEngine<W: WaypointPort> {
    paladin_port: Arc<dyn PaladinPort>,
    waypoint_port: Arc<W>,
    durability: WaypointDurability,
    /// In-flight node execution cap per superstep. `None` defaults to the
    /// Vanguard's own size (D-12) — i.e. effectively unbounded unless
    /// explicitly lowered.
    parallelism: Option<usize>,
    /// Engine-owned custom dispatch rule registrations (ENG-FR-09). Never
    /// referenced from `paladin-core` (X-01) -- handed to
    /// `WarGraph::validate` and `Battlefield::merge` as a
    /// `CustomDispatchResolver` at `start`.
    dispatch_registry: DispatchRegistry,
    /// Registered `EdgeCondition::Custom` evaluators (BUG-01, CF-01). Empty
    /// by default: a v0.9 configuration with no `Custom` edges boots
    /// identically (D-26). Never referenced from `paladin-core` (X-01) --
    /// handed to `WarGraph::validate` and `superstep::run` as an
    /// `EdgeEvaluatorRegistry` at `start`/`resume`.
    edge_evaluators: EdgeEvaluatorRegistry,
    /// The bounded, drop-oldest `TraceSink` forwarder (ENG-FR-21). Always
    /// present -- constructed with no sink (`TraceDispatcher::new(None)`) by
    /// default, in which case `emit` is a no-op and no channel is
    /// allocated.
    trace_dispatcher: Arc<TraceDispatcher>,
    /// The ordered `NodeInterceptor` chain (ENG-FR-22). Empty by default: an
    /// empty chain is proven (in `engine::hooks`'s own tests) to change
    /// nothing about a run's node executions or final state.
    interceptors: Vec<Arc<dyn NodeInterceptor>>,
    /// The optional cancellation signal observed at superstep boundaries
    /// (ENG-FR-23). `None` behaves identically to a token that is never
    /// cancelled.
    cancellation_token: Option<CancellationToken>,
}

impl<W: WaypointPort> WarEngine<W> {
    /// Construct a `WarEngine` over the given Paladin execution port and
    /// Waypoint persistence port, with `WaypointDurability::Strict`, no
    /// explicit parallelism cap, no custom dispatch rules registered, no
    /// trace sink, an empty interceptor chain and no cancellation token.
    pub fn new(paladin_port: Arc<dyn PaladinPort>, waypoint_port: Arc<W>) -> Self {
        Self {
            paladin_port,
            waypoint_port,
            durability: WaypointDurability::Strict,
            parallelism: None,
            dispatch_registry: DispatchRegistry::new(),
            edge_evaluators: EdgeEvaluatorRegistry::new(),
            trace_dispatcher: Arc::new(TraceDispatcher::new(None)),
            interceptors: Vec::new(),
            cancellation_token: None,
        }
    }

    /// Override the default `WaypointDurability::Strict`.
    pub fn with_durability(mut self, durability: WaypointDurability) -> Self {
        self.durability = durability;
        self
    }

    /// Bound the number of nodes executed concurrently within one
    /// superstep. Defaults to the Vanguard's own size (D-12) when not set.
    pub fn with_parallelism(mut self, limit: usize) -> Self {
        self.parallelism = Some(limit);
        self
    }

    /// Register a `(current, delta) -> merged` closure under `name`
    /// (ENG-FR-09), applied when a Battlefield field declares
    /// `DispatchRule::Custom(name)`. Rejects a `name` colliding with a
    /// built-in `DispatchRule` variant name with
    /// `EngineError::ReservedDispatchName` -- registration is where that
    /// collision is caught, not silently ignored later.
    pub fn with_dispatch_rule(
        mut self,
        name: impl Into<String>,
        rule: Arc<paladin_core::platform::container::battlefield::CustomDispatchFn>,
    ) -> Result<Self, EngineError> {
        self.dispatch_registry.register(name, rule)?;
        Ok(self)
    }

    /// Register a named evaluator for `EdgeCondition::Custom(name)` edges
    /// (BUG-01, CF-01), shaped like [`WarEngine::with_dispatch_rule`] but
    /// infallible -- unlike a `DispatchRule::Custom` name, an
    /// `EdgeCondition::Custom` name collides with no built-in
    /// `EdgeCondition` variant, so there is no reserved-name failure mode.
    /// An unregistered `Custom` name still fails [`WarGraph::validate`]
    /// (and therefore [`WarEngine::start`]/[`WarEngine::resume`]) before any
    /// node executes; it is never silently treated as always-true.
    pub fn with_edge_evaluator(
        mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn EdgeConditionEvaluator>,
    ) -> Self {
        self.edge_evaluators.register(name, evaluator);
        self
    }

    /// Attach `sink` as this engine's `TraceSink` (ENG-FR-21). Replaces any
    /// previously configured sink; events are forwarded fire-and-forget over
    /// a bounded, drop-oldest queue -- see `engine::hooks::TraceDispatcher`.
    pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
        self.trace_dispatcher = Arc::new(TraceDispatcher::new(Some(sink)));
        self
    }

    /// Set the ordered `NodeInterceptor` chain (ENG-FR-22), replacing any
    /// previously configured chain. An empty `Vec` (the default) is
    /// equivalent to never calling this method at all.
    pub fn with_interceptors(mut self, interceptors: Vec<Arc<dyn NodeInterceptor>>) -> Self {
        self.interceptors = interceptors;
        self
    }

    /// Attach a `CancellationToken` this engine observes at superstep
    /// boundaries (ENG-FR-23). A token that is never cancelled produces
    /// behavior identical to no token configured at all.
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Start a new run of `graph` under `thread`, seeded with `initial`.
    ///
    /// Runs the full superstep loop (ENG-FR-01): validates the graph,
    /// resolves the initial Battlefield state, then executes supersteps
    /// until the Vanguard is empty (`RunOutcome::Completed`) or a limit or
    /// node/merge failure intervenes (`RunOutcome::Failed`). A
    /// `NodeSpec::Paladin` node renders its input through its
    /// `InputMapping` and calls `PaladinPort::execute` (ENG-FR-13, X-03); an
    /// `InputMapping::render` failure or a `PaladinPort::execute` error both
    /// fail that node exactly as a `Function` node's own error would.
    pub async fn start(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        initial: StateDelta,
    ) -> Result<RunOutcome, EngineError> {
        let registry = self.dispatch_registry.resolver();
        graph.validate(registry, &self.edge_evaluators)?;

        let battlefield = Battlefield::initialize(graph.schema().clone(), &initial)?;
        battlefield.validate_required()?;

        self.trace_dispatcher.emit(TraceEvent::RunStarted {
            thread_id: thread.clone(),
        });
        let outcome = superstep::run(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.edge_evaluators,
            graph,
            thread.clone(),
            battlefield,
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            &self.paladin_port,
            &self.trace_dispatcher,
            &self.interceptors,
            &self.cancellation_token,
        )
        .await;
        self.trace_dispatcher
            .emit(TraceEvent::RunFinished { thread_id: thread });
        outcome
    }

    /// Resume `thread` from its latest Waypoint, with the default
    /// [`ResumeOptions`] (`allow_graph_change: false`) — a graph-fingerprint
    /// mismatch always fails. Use [`WarEngine::resume_with_options`] to opt
    /// into resuming against a changed graph.
    pub async fn resume(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
    ) -> Result<RunOutcome, EngineError> {
        self.resume_with_options(graph, thread, ResumeOptions::default())
            .await
    }

    /// Resume `thread` from its latest Waypoint (ENG-FR-12).
    ///
    /// Loads the latest Waypoint through the port (absent ->
    /// `ThreadNotFound`); compares its `graph_fingerprint` against
    /// `graph.fingerprint()` (differing -> `GraphMismatch`, unless
    /// `options.allow_graph_change` is set); when the loaded status is
    /// `Completed`, returns `RunOutcome::Completed` immediately, executing
    /// nothing and writing no Waypoint. Otherwise every restored Vanguard
    /// `NodeId` is checked against the (possibly new) graph -- one absent is
    /// `VanguardNodeMissing` -- and the Battlefield, Vanguard and per-node
    /// visit counts are restored and handed to the SAME superstep loop
    /// `start` uses, continuing from the superstep after the loaded
    /// Waypoint's.
    ///
    /// What this guarantees, precisely (D-18): the fingerprint comparison
    /// above detects exactly the properties [`WarGraph::fingerprint`]
    /// documents as covered (see its rustdoc for the full, current list) --
    /// it says nothing about any property that function does not cover.
    /// `ResumeOptions::allow_graph_change` deliberately bypasses this check
    /// entirely, trusting the caller that whatever changed is safe to
    /// resume against for this thread.
    ///
    /// The restored-frontier guarantee (BUG-04 / ENG-FR-12a): the loaded
    /// Waypoint's `frontier` -- every incoming edge resolved before the
    /// interruption, keyed by edge identity, plus each node's last-executed
    /// superstep -- is restored into the `Frontier` this call's superstep
    /// loop runs with, not rebuilt from scratch. A pre-crash fired edge into
    /// a join node that was not yet ready is therefore seen again on
    /// resume, so a resumed run schedules the same nodes in the same
    /// supersteps as the uninterrupted run would have. Under
    /// `options.allow_graph_change`, this degrades precisely: a restored
    /// edge resolution whose identity the new graph no longer declares is
    /// dropped, and an edge the new graph adds starts `Pending` --
    /// unresolved, never mis-assigned a stale resolution from a
    /// same-source-or-target edge that used to occupy that identity.
    pub async fn resume_with_options(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        options: ResumeOptions,
    ) -> Result<RunOutcome, EngineError> {
        let latest = self
            .waypoint_port
            .latest(&thread)
            .await
            .map_err(|source| EngineError::WaypointRead { source })?
            .ok_or_else(|| EngineError::ThreadNotFound(thread.clone()))?;

        let expected = graph.fingerprint();
        if latest.graph_fingerprint != expected && !options.allow_graph_change {
            return Err(EngineError::GraphMismatch {
                expected,
                got: latest.graph_fingerprint,
            });
        }

        if matches!(latest.status, WaypointStatus::Completed) {
            self.trace_dispatcher.emit(TraceEvent::RunStarted {
                thread_id: thread.clone(),
            });
            self.trace_dispatcher
                .emit(TraceEvent::RunFinished { thread_id: thread });
            return Ok(RunOutcome::Completed {
                final_state: latest.battlefield,
                waypoint: latest.waypoint_id,
            });
        }

        for node in &latest.vanguard {
            if graph.node(node).is_none() {
                return Err(EngineError::VanguardNodeMissing { node: node.clone() });
            }
        }

        let registry = self.dispatch_registry.resolver();
        graph.validate(registry, &self.edge_evaluators)?;

        self.trace_dispatcher.emit(TraceEvent::RunStarted {
            thread_id: thread.clone(),
        });
        let outcome = superstep::run(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.edge_evaluators,
            graph,
            thread.clone(),
            latest.battlefield,
            latest.vanguard,
            latest.visit_counts,
            Some(latest.frontier),
            Some(latest.waypoint_id),
            latest.superstep + 1,
            &self.paladin_port,
            &self.trace_dispatcher,
            &self.interceptors,
            &self.cancellation_token,
        )
        .await;
        self.trace_dispatcher
            .emit(TraceEvent::RunFinished { thread_id: thread });
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::platform::container::battalion::campaign::EdgeCondition;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::waypoint::{NodeOutcomeKind, Waypoint};
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream};
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use crate::engine::graph::EdgeSpec;
    use crate::engine::test_support::{
        CountingFunctionNode, RecordingPaladinPort, RecordingWaypointStore,
    };

    struct UnimplementedPaladinPort;

    #[async_trait]
    impl PaladinPort for UnimplementedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unimplemented!("not exercised by this plan's Function-node tests")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unimplemented!("not exercised by this plan's Function-node tests")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    struct FixedDeltaNode {
        field: FieldName,
        value: serde_json::Value,
    }

    #[async_trait]
    impl StateNode for FixedDeltaNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<StateDelta, NodeError> {
            let mut delta = StateDelta::new();
            delta.set_raw(self.field.clone(), self.value.clone());
            Ok(delta)
        }
    }

    fn one_field_schema() -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("result").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )])
    }

    fn engine() -> WarEngine<InMemoryWaypointStore> {
        WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
    }

    #[tokio::test]
    async fn start_runs_one_node_and_persists_one_completed_waypoint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("solo");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("done"),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("thread-1").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("result").unwrap())
                        .unwrap(),
                    Some("done".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resume_on_unknown_thread_errors() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_entry(NodeId::new("solo"));

        let engine = engine();
        let thread = ThreadId::new("never-started").unwrap();
        let err = engine.resume(&graph, thread).await.unwrap_err();
        assert!(matches!(err, EngineError::ThreadNotFound(_)));
    }

    // --- Task 2: engine-level custom dispatch registry -------------------

    #[tokio::test]
    async fn engine_with_dispatch_rule_applies_custom_merge_end_to_end() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("max".to_string()),
            Some(serde_json::json!(0)),
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("scorer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: field_name.clone(),
                value: serde_json::json!(7),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine()
            .with_dispatch_rule(
                "max",
                Arc::new(|current: &serde_json::Value, delta: &serde_json::Value| {
                    let c = current.as_i64().unwrap_or(i64::MIN);
                    let d = delta.as_i64().unwrap_or(i64::MIN);
                    Ok(serde_json::json!(c.max(d)))
                }),
            )
            .unwrap();
        let thread = ThreadId::new("custom-dispatch").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(final_state.get::<i64>(&field_name).unwrap(), Some(7));
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn engine_start_fails_before_execution_for_unregistered_custom_dispatch() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("missing".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node = crate::engine::test_support::CountingFunctionNode::fixed(
            field_name,
            serde_json::json!(1),
        );
        let node_id = NodeId::new("n");
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("unregistered-custom").unwrap();
        let err = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap_err();
        match err {
            EngineError::Battlefield(BattlefieldError::CustomDispatchNotRegistered { name }) => {
                assert_eq!(name, "missing");
            }
            other => panic!("expected CustomDispatchNotRegistered, got {other:?}"),
        }
        assert_eq!(
            node.run_count(),
            0,
            "no node executes before graph validation passes"
        );
    }

    #[tokio::test]
    async fn engine_two_writer_last_write_conflict_surfaces_field_superstep_and_writers() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let n1 = NodeId::new("n1");
        let n2 = NodeId::new("n2");
        graph.add_node(
            n1.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("a"),
            })),
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: FieldName::new("result").unwrap(),
                value: serde_json::json!("b"),
            })),
        );
        graph.add_entry(n1.clone());
        graph.add_entry(n2.clone());

        let engine = engine();
        let thread = ThreadId::new("dispatch-conflict").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => match error {
                EngineError::Battlefield(BattlefieldError::DispatchConflict {
                    field,
                    superstep,
                    writers,
                }) => {
                    assert_eq!(field, FieldName::new("result").unwrap());
                    assert_eq!(superstep, 1);
                    let mut sorted = writers.clone();
                    sorted.sort();
                    assert_eq!(sorted, vec![n1.clone(), n2.clone()]);
                }
                other => panic!("expected DispatchConflict, got {other:?}"),
            },
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn engine_custom_dispatch_closure_error_fails_the_run_not_swallowed() {
        let field_name = FieldName::new("score").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Custom("boom".to_string()),
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("n");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(Arc::new(FixedDeltaNode {
                field: field_name.clone(),
                value: serde_json::json!(1),
            })),
        );
        graph.add_entry(node_id);

        let engine = engine()
            .with_dispatch_rule(
                "boom",
                Arc::new(|_c: &serde_json::Value, _d: &serde_json::Value| {
                    Err(BattlefieldError::TypeMismatch {
                        field: FieldName::new("score").unwrap(),
                        expected: "never".to_string(),
                        got: "boom".to_string(),
                    })
                }),
            )
            .unwrap();
        let thread = ThreadId::new("custom-dispatch-error").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => {
                assert!(matches!(
                    error,
                    EngineError::Battlefield(BattlefieldError::TypeMismatch { .. })
                ));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn input_mapping_renders_string_field_raw() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("name").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(FieldName::new("name").unwrap(), "world").unwrap();
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchResolver::new(),
            )
            .unwrap();

        let mapping = InputMapping::new("hello {name}!");
        assert_eq!(mapping.render(&battlefield).unwrap(), "hello world!");
    }

    // --- Task 1: NodeSpec::Paladin execution ------------------------------

    fn make_paladin(name: &str) -> Paladin {
        let data = paladin_core::platform::container::paladin::PaladinData {
            name: name.to_string(),
            ..Default::default()
        };
        paladin_core::base::entity::node::Node::new(data, Some(name.to_string()))
    }

    fn engine_with_port(
        port: Arc<crate::engine::test_support::RecordingPaladinPort>,
    ) -> WarEngine<InMemoryWaypointStore> {
        WarEngine::new(port, Arc::new(InMemoryWaypointStore::new()))
    }

    #[tokio::test]
    async fn paladin_node_writes_output_into_declared_field() {
        let field_name = FieldName::new("summary").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("topic").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(field_name.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("summarizer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Paladin {
                paladin: Box::new(make_paladin("summarizer")),
                input_template: InputMapping::new("summarize {topic}"),
                output_field: field_name.clone(),
            },
        );
        graph.add_entry(node_id);

        let port = Arc::new(crate::engine::test_support::RecordingPaladinPort::new());
        port.set_output("summarizer", "a short summary");
        let engine = engine_with_port(port.clone());

        let mut initial = StateDelta::new();
        initial
            .set(FieldName::new("topic").unwrap(), "rust")
            .unwrap();
        let thread = ThreadId::new("paladin-write").unwrap();
        let outcome = engine.start(&graph, thread, initial).await.unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&field_name).unwrap(),
                    Some("a short summary".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        assert_eq!(
            port.call_log(),
            vec![("summarizer".to_string(), "summarize rust".to_string())]
        );
        assert_eq!(port.call_count(), 1);
    }

    #[tokio::test]
    async fn paladin_node_append_output_field_accumulates() {
        let field_name = FieldName::new("notes").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let n1 = NodeId::new("first");
        let n2 = NodeId::new("second");
        graph.add_node(
            n1.clone(),
            NodeSpec::Paladin {
                paladin: Box::new(make_paladin("first")),
                input_template: InputMapping::new("note one"),
                output_field: field_name.clone(),
            },
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::Paladin {
                paladin: Box::new(make_paladin("second")),
                input_template: InputMapping::new("note two"),
                output_field: field_name.clone(),
            },
        );
        graph.add_entry(n1);
        graph.add_entry(n2);

        let port = Arc::new(crate::engine::test_support::RecordingPaladinPort::new());
        port.set_output("first", "alpha");
        port.set_output("second", "beta");
        let engine = engine_with_port(port);

        let thread = ThreadId::new("paladin-append").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let values: Vec<String> = final_state.get(&field_name).unwrap().unwrap();
                let mut sorted = values.clone();
                sorted.sort();
                assert_eq!(sorted, vec!["alpha".to_string(), "beta".to_string()]);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn paladin_node_execution_record_carries_reported_token_count() {
        let field_name = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("counter");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Paladin {
                paladin: Box::new(make_paladin("counter")),
                input_template: InputMapping::new("go"),
                output_field: field_name,
            },
        );
        graph.add_entry(node_id.clone());

        let port = Arc::new(crate::engine::test_support::RecordingPaladinPort::new());
        port.set_output_with_tokens("counter", "done", 42);
        let store = Arc::new(crate::engine::test_support::RecordingWaypointStore::new());
        let engine = WarEngine::new(port, store.clone());

        let thread = ThreadId::new("paladin-tokens").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let saved = store.saved_waypoints(&thread).await;
        assert_eq!(saved.len(), 1);
        let record = saved[0]
            .completed
            .iter()
            .find(|r| r.node_id == node_id)
            .expect("counter node record present");
        assert_eq!(record.token_count, 42);
        assert_eq!(record.attempt, 1);
        assert!(matches!(record.outcome, NodeOutcomeKind::Succeeded));
    }

    #[tokio::test]
    async fn paladin_port_execute_error_fails_the_node_and_the_run() {
        struct FailingPaladinPort;

        #[async_trait]
        impl PaladinPort for FailingPaladinPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Err(PaladinError::ExecutionError("boom".to_string()))
            }

            async fn execute_stream(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinStream, PaladinError> {
                unimplemented!("not exercised by this test")
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let field_name = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            field_name.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("failer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Paladin {
                paladin: Box::new(make_paladin("failer")),
                input_template: InputMapping::new("go"),
                output_field: field_name,
            },
        );
        graph.add_entry(node_id.clone());

        let engine = WarEngine::new(
            Arc::new(FailingPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("paladin-failure").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(error, EngineError::Node(_)));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }
    // --- Task 2: full resume -- restore state, vanguard, visit counts ----

    /// Fetch a thread's full Waypoint history, sorted ascending by
    /// superstep (`RecordingWaypointStore::saved_waypoints`'s own order
    /// follows `history()`'s descending-`created_at` contract, which is not
    /// what these tests want to iterate over).
    async fn ascending_history(store: &RecordingWaypointStore, thread: &ThreadId) -> Vec<Waypoint> {
        let mut waypoints = store.saved_waypoints(thread).await;
        waypoints.sort_by_key(|w| w.superstep);
        waypoints
    }

    fn two_node_chain_graph() -> (WarGraph, NodeId, NodeId) {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        graph.add_node(
            a.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("a"),
            )),
        );
        graph.add_node(
            b.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("b"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_entry(a.clone());
        (graph, a, b)
    }

    #[tokio::test]
    async fn resume_completed_short_circuit_writes_no_new_waypoint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("solo");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("done"),
            )),
        );
        graph.add_entry(node_id);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-completed").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert_eq!(store.save_call_count(), 1);

        let resumed = engine.resume(&graph, thread).await.unwrap();
        assert!(matches!(resumed, RunOutcome::Completed { .. }));
        assert_eq!(
            store.save_call_count(),
            1,
            "resume on an already-Completed waypoint must write no new Waypoint"
        );
    }

    #[tokio::test]
    async fn resume_with_graph_mismatch_fails_without_allow_graph_change() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("solo");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("done"),
            )),
        );
        graph.add_entry(node_id);

        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-mismatch").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let mut altered_schema = one_field_schema();
        altered_schema.fields.push(FieldSpec::new(
            FieldName::new("extra").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        ));
        let altered_graph = WarGraph::new(altered_schema, EngineLimits::default());

        let err = engine.resume(&altered_graph, thread).await.unwrap_err();
        assert!(matches!(err, EngineError::GraphMismatch { .. }));
    }

    #[tokio::test]
    async fn resume_allow_graph_change_missing_vanguard_node_fails_precisely() {
        let (graph, _a, b) = two_node_chain_graph();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-vanguard-missing").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let waypoints = ascending_history(&store, &thread).await;
        assert_eq!(waypoints.len(), 2, "a two-node chain takes two supersteps");
        let waypoint_after_a = waypoints[0].clone();

        let store2 = InMemoryWaypointStore::new();
        store2.save(&waypoint_after_a).await.unwrap();
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), Arc::new(store2));

        // An altered graph containing only "a" -- "b" (the restored
        // Vanguard node) is absent.
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut altered = WarGraph::new(schema, EngineLimits::default());
        altered.add_node(
            NodeId::new("a"),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("a"),
            )),
        );
        altered.add_entry(NodeId::new("a"));
        assert_ne!(graph.fingerprint(), altered.fingerprint());

        let err = engine2
            .resume_with_options(
                &altered,
                thread,
                ResumeOptions {
                    allow_graph_change: true,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::VanguardNodeMissing { node } if node == b));
    }

    #[tokio::test]
    async fn resume_allow_graph_change_proceeds_when_vanguard_node_present() {
        let (graph, _a, _b) = two_node_chain_graph();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-allow-change-ok").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let waypoints = ascending_history(&store, &thread).await;
        assert_eq!(waypoints.len(), 2);
        let waypoint_after_a = waypoints[0].clone();

        let store2 = InMemoryWaypointStore::new();
        store2.save(&waypoint_after_a).await.unwrap();
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), Arc::new(store2));

        // Altered graph: the same two nodes, PLUS an extra node "c" --
        // fingerprint differs (node ids are hashed; entry status is not),
        // but the restored vanguard node ("b") is still present. "c" is
        // declared as its own entry point (ENG-FR-02a / BUG-02: a declared
        // node with no incoming edge and no entry status would be rejected
        // at validate() as unreachable) -- it is never part of the
        // RESTORED vanguard this resume actually schedules, so it never
        // executes here; the entry declaration exists solely to make it a
        // legitimately eligible node rather than a stranded one.
        let (mut altered, _a2, _b2) = two_node_chain_graph();
        altered.add_node(
            NodeId::new("c"),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("c"),
            )),
        );
        altered.add_entry(NodeId::new("c"));
        assert_ne!(graph.fingerprint(), altered.fingerprint());

        let outcome = engine2
            .resume_with_options(
                &altered,
                thread,
                ResumeOptions {
                    allow_graph_change: true,
                },
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    #[tokio::test]
    async fn resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit() {
        // `a` is this graph's ONLY node, self-looping and declared entry.
        // Readiness dodge, not a strandedness workaround (Phase 22 Plan 16
        // audit, `22-deferred-items.md`): `a`'s self-loop is its sole
        // incoming edge, and `Frontier::is_ready` (`engine::superstep`)
        // leaves a self-loop `Pending` until the node has run once, so a
        // non-entry `a` could never take its first turn. `a` has no feed
        // from outside itself, so this shape is unaffected by either
        // BUG-02's eligible-set reachability check (which `a` would
        // satisfy either way, as entry always does) or BUG-03's
        // starvation-release fix (`Frontier::starved_release`,
        // `engine::superstep`), which only releases a cycle node already
        // holding a fresh fired edge from outside the cycle -- `a` never
        // has one. Entry status is what bootstraps it here.
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("status").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let limits = EngineLimits {
            max_supersteps: 20,
            max_node_visits: 5,
            run_timeout: None,
        };
        let mut graph = WarGraph::new(schema, limits);
        let a = NodeId::new("a");
        graph.add_node(
            a.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("status").unwrap(),
                serde_json::json!("looping"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: a.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(a);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-visit-counts").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        match outcome {
            RunOutcome::Failed { error, .. } => {
                assert!(matches!(
                    error,
                    EngineError::NodeVisitLimitExceeded { limit: 5, .. }
                ));
            }
            other => panic!(
                "expected the uninterrupted control run to trip the visit limit, got {other:?}"
            ),
        }

        let waypoints = ascending_history(&store, &thread).await;
        // 4 successful (Running) visits + 1 Failed waypoint (the tripped
        // 5th attempt, which never executed) = 5.
        assert_eq!(waypoints.len(), 5);

        let store2 = InMemoryWaypointStore::new();
        for wp in &waypoints[0..4] {
            store2.save(wp).await.unwrap();
        }
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), Arc::new(store2));
        let resumed = engine2.resume(&graph, thread).await.unwrap();
        match resumed {
            RunOutcome::Failed { error, .. } => {
                assert!(
                    matches!(error, EngineError::NodeVisitLimitExceeded { limit: 5, .. }),
                    "restored visit counts must trip the SAME limit on the very next post-resume \
                     visit, not silently reset and allow four more; got {error:?}"
                );
            }
            other => panic!("expected Failed(NodeVisitLimitExceeded), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resume_parameterized_at_every_superstep_index_matches_control_and_skips_completed_nodes()
     {
        // A deterministic 5-superstep linear chain of Paladin nodes, driven
        // by a call-recording port -- ENG-FR-12's parameterized proof.
        let field_names: Vec<FieldName> = (1..=5)
            .map(|i| FieldName::new(format!("f{i}")).unwrap())
            .collect();
        let mut schema_fields = vec![FieldSpec::new(
            FieldName::new("topic").unwrap(),
            DispatchRule::LastWrite,
            None,
            true,
        )];
        for f in &field_names {
            schema_fields.push(FieldSpec::new(
                f.clone(),
                DispatchRule::LastWrite,
                None,
                false,
            ));
        }
        let schema = BattlefieldSchema::new(schema_fields);

        let node_ids: Vec<NodeId> = (1..=5).map(|i| NodeId::new(format!("n{i}"))).collect();

        let build_graph = || {
            let mut graph = WarGraph::new(schema.clone(), EngineLimits::default());
            for (i, node_id) in node_ids.iter().enumerate() {
                let input_field = if i == 0 {
                    "topic".to_string()
                } else {
                    format!("f{i}")
                };
                graph.add_node(
                    node_id.clone(),
                    NodeSpec::Paladin {
                        paladin: Box::new(make_paladin(&format!("n{}", i + 1))),
                        input_template: InputMapping::new(format!("{{{input_field}}}")),
                        output_field: field_names[i].clone(),
                    },
                );
            }
            for pair in node_ids.windows(2) {
                graph.add_edge(EdgeSpec {
                    from: pair[0].clone(),
                    to: pair[1].clone(),
                    condition: None,
                });
            }
            graph.add_entry(node_ids[0].clone());
            graph
        };

        let control_port = Arc::new(RecordingPaladinPort::new());
        for i in 1..=5 {
            control_port.set_output(format!("n{i}"), format!("out{i}"));
        }
        let control_store = Arc::new(RecordingWaypointStore::new());
        let control_graph = build_graph();
        let control_engine = WarEngine::new(control_port, control_store.clone());
        let mut initial = StateDelta::new();
        initial
            .set(FieldName::new("topic").unwrap(), "seed")
            .unwrap();
        let control_thread = ThreadId::new("resume-parameterized-control").unwrap();
        let control_outcome = control_engine
            .start(&control_graph, control_thread.clone(), initial.clone())
            .await
            .unwrap();
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        let control_waypoints = ascending_history(&control_store, &control_thread).await;
        assert_eq!(
            control_waypoints.len(),
            5,
            "5 linear nodes take 5 supersteps"
        );

        for k in 1..=5usize {
            let completed_before_drop: std::collections::HashSet<String> = control_waypoints[0..k]
                .iter()
                .flat_map(|wp| wp.completed.iter().map(|r| r.node_id.as_str().to_string()))
                .collect();

            let store2 = InMemoryWaypointStore::new();
            for wp in &control_waypoints[0..k] {
                store2.save(wp).await.unwrap();
            }
            let resume_port = Arc::new(RecordingPaladinPort::new());
            for i in 1..=5 {
                resume_port.set_output(format!("n{i}"), format!("out{i}"));
            }
            let graph_for_resume = build_graph();
            let engine2 = WarEngine::new(resume_port.clone(), Arc::new(store2));
            let resumed = engine2
                .resume(&graph_for_resume, control_thread.clone())
                .await
                .unwrap();

            let resumed_final = match resumed {
                RunOutcome::Completed { final_state, .. } => final_state,
                other => panic!("k={k}: expected resumed run to complete, got {other:?}"),
            };
            assert_eq!(
                resumed_final, control_final,
                "k={k}: resumed final Battlefield must equal the control run's"
            );

            for (name, _input) in resume_port.call_log() {
                assert!(
                    !completed_before_drop.contains(&name),
                    "k={k}: node {name} completed before the drop but appears again post-resume"
                );
            }
        }
    }

    // --- BUG-04 / ENG-FR-12a: the Frontier is restored on resume ----------

    /// The D-24 join shape: `entry -> a`, `entry -> b`, `a -> d`, `b -> c`,
    /// `c -> d`, only `entry` declared entry. `conditional_c_to_d` selects
    /// between the mandated RED proof (`c -> d` carries a condition that
    /// evaluates false against the Battlefield `c` produces, so `a -> d`'s
    /// pre-crash fire is the ONLY thing that can ever make `d` ready) and
    /// the plain equivalence shape (every edge unconditional).
    fn bug_04_join_shape_graph(
        conditional_c_to_d: bool,
    ) -> (WarGraph, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let entry = NodeId::new("entry");
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        let d = NodeId::new("d");
        let label = |value: &str| {
            CountingFunctionNode::fixed(FieldName::new("trace").unwrap(), serde_json::json!(value))
        };
        graph.add_node(entry.clone(), NodeSpec::Function(label("ENTRY")));
        graph.add_node(a.clone(), NodeSpec::Function(label("A")));
        graph.add_node(b.clone(), NodeSpec::Function(label("B")));
        graph.add_node(c.clone(), NodeSpec::Function(label("C")));
        graph.add_node(d.clone(), NodeSpec::Function(label("D")));
        graph.add_edge(EdgeSpec {
            from: entry.clone(),
            to: a.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: entry.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: b.clone(),
            to: c.clone(),
            condition: None,
        });
        // None of ENTRY/A/B/C ever write "UNLOCK", so this condition always
        // evaluates false -- c -> d never fires by itself, and only a -> d's
        // pre-crash fire can ever make d ready.
        let c_to_d_condition =
            conditional_c_to_d.then(|| EdgeCondition::Contains("UNLOCK".to_string()));
        graph.add_edge(EdgeSpec {
            from: c.clone(),
            to: d.clone(),
            condition: c_to_d_condition,
        });
        graph.add_entry(entry.clone());
        (graph, entry, a, b, c, d)
    }

    #[tokio::test]
    async fn resume_restores_pre_crash_edge_resolutions_and_executes_the_pending_join() {
        let (control_graph, _entry, _a, _b, c, d) = bug_04_join_shape_graph(true);
        let control_store = Arc::new(RecordingWaypointStore::new());
        let control_engine =
            WarEngine::new(Arc::new(UnimplementedPaladinPort), control_store.clone());
        let thread = ThreadId::new("bug-04-join-conditional").unwrap();
        let control_outcome = control_engine
            .start(&control_graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        let control_waypoints = ascending_history(&control_store, &thread).await;
        assert_eq!(
            control_waypoints.len(),
            4,
            "entry -> {{a,b}} -> c -> d takes four supersteps"
        );
        assert_eq!(
            control_waypoints[1].vanguard,
            vec![c.clone()],
            "the crash point's persisted vanguard must be exactly [c] (D-24)"
        );

        // Simulate the crash: re-save only the Waypoints up to and including
        // the crash point (superstep 1: entry ran; superstep 2: a and b
        // ran) into a fresh store, then resume with a fresh WarEngine.
        let store2 = Arc::new(RecordingWaypointStore::new());
        for wp in &control_waypoints[0..2] {
            store2.save(wp).await.unwrap();
        }
        let (resume_graph, ..) = bug_04_join_shape_graph(true);
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), store2.clone());
        let resumed = engine2.resume(&resume_graph, thread.clone()).await.unwrap();

        let resumed_waypoints = ascending_history(&store2, &thread).await;
        let d_executions = resumed_waypoints
            .iter()
            .flat_map(|wp| wp.completed.iter())
            .filter(|r| r.node_id == d && matches!(r.outcome, NodeOutcomeKind::Succeeded))
            .count();
        assert_eq!(
            d_executions, 1,
            "d must execute exactly once in the resumed run, matching the control run -- \
             BUG-04: resume rebuilding the Frontier from scratch loses the pre-crash a -> d \
             fire, so d never becomes ready and the run reports Completed without it"
        );

        match resumed {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state, control_final,
                    "resumed final Battlefield must equal the control run's"
                );
            }
            other => panic!("expected resumed run to complete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn resume_after_a_join_shape_crash_matches_the_control_run_superstep_for_superstep() {
        let (control_graph, _entry, _a, _b, c, _d) = bug_04_join_shape_graph(false);
        let control_store = Arc::new(RecordingWaypointStore::new());
        let control_engine =
            WarEngine::new(Arc::new(UnimplementedPaladinPort), control_store.clone());
        let thread = ThreadId::new("bug-04-join-unconditional").unwrap();
        let control_outcome = control_engine
            .start(&control_graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        let control_waypoints = ascending_history(&control_store, &thread).await;
        assert_eq!(control_waypoints.len(), 4);
        let crash_superstep = control_waypoints[1].superstep;
        assert_eq!(control_waypoints[1].vanguard, vec![c.clone()]);

        let store2 = Arc::new(RecordingWaypointStore::new());
        for wp in &control_waypoints[0..2] {
            store2.save(wp).await.unwrap();
        }
        let (resume_graph, ..) = bug_04_join_shape_graph(false);
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), store2.clone());
        let resumed = engine2.resume(&resume_graph, thread.clone()).await.unwrap();
        let resumed_final = match resumed {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected resumed run to complete, got {other:?}"),
        };
        assert_eq!(
            resumed_final, control_final,
            "resumed final Battlefield must equal the control run's"
        );

        let resumed_waypoints = ascending_history(&store2, &thread).await;
        let control_by_superstep: std::collections::HashMap<
            u64,
            std::collections::HashSet<NodeId>,
        > = control_waypoints
            .iter()
            .map(|wp| {
                (
                    wp.superstep,
                    wp.completed.iter().map(|r| r.node_id.clone()).collect(),
                )
            })
            .collect();
        let resumed_by_superstep: std::collections::HashMap<
            u64,
            std::collections::HashSet<NodeId>,
        > = resumed_waypoints
            .iter()
            .map(|wp| {
                (
                    wp.superstep,
                    wp.completed.iter().map(|r| r.node_id.clone()).collect(),
                )
            })
            .collect();

        for (superstep, control_set) in &control_by_superstep {
            if *superstep <= crash_superstep {
                continue;
            }
            let resumed_set = resumed_by_superstep.get(superstep).unwrap_or_else(|| {
                panic!(
                    "resumed run has no waypoint for superstep {superstep}; control executed \
                     {control_set:?} there"
                )
            });
            assert_eq!(
                resumed_set, control_set,
                "superstep {superstep}: resumed executed-node set must equal the control run's"
            );
        }

        let completed_before_crash: std::collections::HashSet<NodeId> = control_waypoints[0..2]
            .iter()
            .flat_map(|wp| wp.completed.iter().map(|r| r.node_id.clone()))
            .collect();
        let post_resume_executed: std::collections::HashSet<NodeId> = resumed_waypoints
            .iter()
            .filter(|wp| wp.superstep > crash_superstep)
            .flat_map(|wp| wp.completed.iter().map(|r| r.node_id.clone()))
            .collect();
        for node in &completed_before_crash {
            assert!(
                !post_resume_executed.contains(node),
                "{node} completed before the crash point but ran again post-resume"
            );
        }
    }

    #[tokio::test]
    async fn resume_with_allow_graph_change_drops_unknown_snapshot_edges_and_starts_new_edges_pending()
     {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph_a = WarGraph::new(schema.clone(), EngineLimits::default());
        let p = NodeId::new("p");
        let q = NodeId::new("q");
        let t = NodeId::new("t");
        graph_a.add_node(
            p.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("p"),
            )),
        );
        graph_a.add_node(
            q.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("q"),
            )),
        );
        graph_a.add_node(
            t.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("t"),
            )),
        );
        graph_a.add_edge(EdgeSpec {
            from: p.clone(),
            to: q.clone(),
            condition: None,
        });
        graph_a.add_edge(EdgeSpec {
            from: q.clone(),
            to: t.clone(),
            condition: None,
        });
        graph_a.add_entry(p.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-allow-change-drops-unknown-and-starts-pending").unwrap();
        engine
            .start(&graph_a, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let waypoints = ascending_history(&store, &thread).await;
        assert_eq!(waypoints.len(), 3, "p -> q -> t takes three supersteps");
        let waypoint_after_q = waypoints[1].clone();
        assert_eq!(waypoint_after_q.vanguard, vec![t.clone()]);

        let store2 = InMemoryWaypointStore::new();
        store2.save(&waypoint_after_q).await.unwrap();
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), Arc::new(store2));

        // Graph B: p -> q UNCHANGED; q -> t REMOVED, q -> u ADDED. `t` keeps
        // no incoming edge at all in the new graph -- it is the restored
        // vanguard node and must still validate, so it is declared a second
        // entry rather than given a new incoming edge.
        let mut graph_b = WarGraph::new(schema, EngineLimits::default());
        graph_b.add_node(
            p.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("p"),
            )),
        );
        graph_b.add_node(
            q.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("q"),
            )),
        );
        graph_b.add_node(
            t.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("trace").unwrap(),
                serde_json::json!("t"),
            )),
        );
        let u = NodeId::new("u");
        let u_node =
            CountingFunctionNode::fixed(FieldName::new("trace").unwrap(), serde_json::json!("u"));
        graph_b.add_node(u.clone(), NodeSpec::Function(u_node.clone()));
        graph_b.add_edge(EdgeSpec {
            from: p.clone(),
            to: q.clone(),
            condition: None,
        });
        graph_b.add_edge(EdgeSpec {
            from: q.clone(),
            to: u.clone(),
            condition: None,
        });
        graph_b.add_entry(p.clone());
        graph_b.add_entry(t.clone());
        assert_ne!(graph_a.fingerprint(), graph_b.fingerprint());

        let outcome = engine2
            .resume_with_options(
                &graph_b,
                thread,
                ResumeOptions {
                    allow_graph_change: true,
                },
            )
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            u_node.run_count(),
            0,
            "the NEW q -> u edge must start Pending -- the OLD q -> t snapshot resolution must \
             not be mis-assigned onto it merely because both edges share source node q"
        );
    }

    // --- Task 1: TraceSink, end to end through a real WarEngine run ------

    use crate::engine::test_support::{
        AlwaysErroringTraceSink, BlockingTraceSink, RecordingTraceSink,
    };
    use paladin_ports::output::trace_sink_port::TraceEvent;
    use std::sync::atomic::AtomicBool;

    fn trace_event_name(event: &TraceEvent) -> &'static str {
        match event {
            TraceEvent::RunStarted { .. } => "RunStarted",
            TraceEvent::SuperstepStarted { .. } => "SuperstepStarted",
            TraceEvent::NodeStarted { .. } => "NodeStarted",
            TraceEvent::NodeFinished { .. } => "NodeFinished",
            TraceEvent::DeltaMerged { .. } => "DeltaMerged",
            TraceEvent::WaypointSaved { .. } => "WaypointSaved",
            TraceEvent::RunFinished { .. } => "RunFinished",
            _ => "unknown",
        }
    }

    #[tokio::test]
    async fn trace_sink_receives_exact_ordered_event_sequence_for_two_superstep_run() {
        let (graph, _a, _b) = two_node_chain_graph();
        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone());
        let thread = ThreadId::new("trace-two-superstep").unwrap();

        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        // Give the background trace consumer a chance to drain.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let names: Vec<&str> = sink.events().await.iter().map(trace_event_name).collect();
        assert_eq!(
            names,
            vec![
                "RunStarted",
                "SuperstepStarted",
                "NodeStarted",
                "NodeFinished",
                "DeltaMerged",
                "WaypointSaved",
                "SuperstepStarted",
                "NodeStarted",
                "NodeFinished",
                "DeltaMerged",
                "WaypointSaved",
                "RunFinished",
            ]
        );
    }

    #[tokio::test]
    async fn permanently_blocking_trace_sink_does_not_stall_a_real_run() {
        let (graph, _a, _b) = two_node_chain_graph();
        let entered = Arc::new(AtomicBool::new(false));
        let sink = BlockingTraceSink::new(entered.clone());
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink);
        let thread = ThreadId::new("trace-blocking-sink").unwrap();

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            engine.start(&graph, thread, StateDelta::new()),
        )
        .await
        .expect("the run must complete inside the timeout despite a permanently blocking sink")
        .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        // Give the background consumer a moment to have actually been
        // invoked (it then hangs forever on the first event -- that hang is
        // exactly the point, and must not have been on the run's path).
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            entered.load(std::sync::atomic::Ordering::SeqCst),
            "the blocking sink must actually have been invoked"
        );
    }

    #[tokio::test]
    async fn always_erroring_trace_sink_leaves_run_outcome_and_battlefield_unchanged() {
        let (plain_graph, _a, _b) = two_node_chain_graph();
        let plain_engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let plain_thread = ThreadId::new("trace-none").unwrap();
        let plain_outcome = plain_engine
            .start(&plain_graph, plain_thread, StateDelta::new())
            .await
            .unwrap();
        let plain_final = match plain_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };

        let (traced_graph, _a2, _b2) = two_node_chain_graph();
        let sink = AlwaysErroringTraceSink::new();
        let traced_engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone());
        let traced_thread = ThreadId::new("trace-erroring").unwrap();
        let traced_outcome = traced_engine
            .start(&traced_graph, traced_thread, StateDelta::new())
            .await
            .unwrap();
        let traced_final = match traced_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };

        assert_eq!(
            plain_final, traced_final,
            "an always-erroring sink must not change the run's final Battlefield"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            sink.call_count() > 0,
            "the erroring sink must actually have been invoked"
        );
    }

    // --- Task 2: NodeInterceptor chain, end to end through a real run ----

    use crate::engine::hooks::{InterceptDecision, NodeInterceptor};

    #[tokio::test]
    async fn empty_interceptor_chain_is_identical_to_no_chain_configured() {
        let (graph_a, _a, _b) = two_node_chain_graph();
        let engine_no_chain = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(RecordingWaypointStore::new()),
        );
        let thread_a = ThreadId::new("no-chain").unwrap();
        let outcome_a = engine_no_chain
            .start(&graph_a, thread_a, StateDelta::new())
            .await
            .unwrap();

        let (graph_b, _a2, _b2) = two_node_chain_graph();
        let store_b = Arc::new(RecordingWaypointStore::new());
        let engine_empty_chain =
            WarEngine::new(Arc::new(UnimplementedPaladinPort), store_b.clone())
                .with_interceptors(Vec::new());
        let thread_b = ThreadId::new("empty-chain").unwrap();
        let outcome_b = engine_empty_chain
            .start(&graph_b, thread_b.clone(), StateDelta::new())
            .await
            .unwrap();

        match (outcome_a, outcome_b) {
            (
                RunOutcome::Completed {
                    final_state: state_a,
                    ..
                },
                RunOutcome::Completed {
                    final_state: state_b,
                    ..
                },
            ) => assert_eq!(state_a, state_b),
            other => panic!("expected both runs to complete, got {other:?}"),
        }
        let waypoints_b = store_b.saved_waypoints(&thread_b).await;
        assert_eq!(
            waypoints_b.len(),
            2,
            "an empty chain must not change the number of supersteps/waypoints"
        );
        for wp in &waypoints_b {
            for record in &wp.completed {
                assert!(matches!(
                    record.outcome,
                    paladin_core::platform::container::waypoint::NodeOutcomeKind::Succeeded
                ));
            }
        }
    }

    struct SkipEverything;

    #[async_trait]
    impl NodeInterceptor for SkipEverything {
        async fn before(
            &self,
            _ctx: &crate::engine::node::NodeContext,
            _state: &Battlefield,
        ) -> InterceptDecision {
            InterceptDecision::Skip("skipped by test interceptor".to_string())
        }
        async fn after(&self, _ctx: &crate::engine::node::NodeContext, _delta: &mut StateDelta) {}
    }

    #[tokio::test]
    async fn skip_decision_produces_skipped_execution_record_and_no_delta() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("skip-me");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("should-never-appear"),
            )),
        );
        graph.add_entry(node_id.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_interceptors(vec![Arc::new(SkipEverything)]);
        let thread = ThreadId::new("skip-everything").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("result").unwrap())
                        .unwrap(),
                    None,
                    "a Skipped node contributes no delta"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 1);
        let record = &waypoints[0].completed[0];
        assert_eq!(record.node_id, node_id);
        match &record.outcome {
            paladin_core::platform::container::waypoint::NodeOutcomeKind::Skipped { reason } => {
                assert_eq!(reason, "skipped by test interceptor");
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    struct FailEverything;

    #[async_trait]
    impl NodeInterceptor for FailEverything {
        async fn before(
            &self,
            _ctx: &crate::engine::node::NodeContext,
            _state: &Battlefield,
        ) -> InterceptDecision {
            InterceptDecision::Fail(NodeError("interceptor-forced failure".to_string()))
        }
        async fn after(&self, _ctx: &crate::engine::node::NodeContext, _delta: &mut StateDelta) {}
    }

    #[tokio::test]
    async fn fail_decision_fails_the_node_and_the_run() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("fail-me");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("x"),
            )),
        );
        graph.add_entry(node_id);

        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![Arc::new(FailEverything)]);
        let thread = ThreadId::new("fail-everything").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(error, EngineError::Node(_)));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    struct OrderRecordingInterceptor {
        label: &'static str,
        order: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl NodeInterceptor for OrderRecordingInterceptor {
        async fn before(
            &self,
            _ctx: &crate::engine::node::NodeContext,
            _state: &Battlefield,
        ) -> InterceptDecision {
            self.order
                .lock()
                .unwrap()
                .push(format!("before:{}", self.label));
            InterceptDecision::Proceed
        }

        async fn after(&self, _ctx: &crate::engine::node::NodeContext, delta: &mut StateDelta) {
            self.order
                .lock()
                .unwrap()
                .push(format!("after:{}", self.label));
            let marker_field = FieldName::new("marker").unwrap();
            let existing = delta
                .values
                .get(&marker_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            delta.set_raw(
                marker_field,
                serde_json::json!(format!("{existing}{}", self.label)),
            );
        }
    }

    #[tokio::test]
    async fn two_interceptors_run_before_first_to_last_and_after_observes_prior_mutation() {
        let field_name = FieldName::new("marker").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("result").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(field_name.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("ordered");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("x"),
            )),
        );
        graph.add_entry(node_id);

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let first = Arc::new(OrderRecordingInterceptor {
            label: "A",
            order: order.clone(),
        });
        let second = Arc::new(OrderRecordingInterceptor {
            label: "B",
            order: order.clone(),
        });
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![first, second]);
        let thread = ThreadId::new("ordered-interceptors").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&field_name).unwrap(),
                    Some("AB".to_string()),
                    "each after() must observe the previous after()'s mutation"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(
            order.lock().unwrap().clone(),
            vec!["before:A", "before:B", "after:A", "after:B"]
        );
    }

    #[tokio::test]
    async fn skip_from_first_interceptor_short_circuits_second_interceptors_before() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("short-circuit");
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("x"),
            )),
        );
        graph.add_entry(node_id);

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let never_called = Arc::new(OrderRecordingInterceptor {
            label: "never",
            order: order.clone(),
        });
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![Arc::new(SkipEverything), never_called]);
        let thread = ThreadId::new("skip-short-circuits").unwrap();
        let _ = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(
            order.lock().unwrap().is_empty(),
            "the second interceptor's before() must never be called after the first Skips"
        );
    }

    // --- Task 3: CancellationToken -> Halted, resumable -------------------

    fn four_node_chain_graph() -> (WarGraph, Vec<NodeId>) {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let ids: Vec<NodeId> = (1..=4).map(|i| NodeId::new(format!("n{i}"))).collect();
        for id in &ids {
            graph.add_node(
                id.clone(),
                NodeSpec::Function(CountingFunctionNode::fixed(
                    FieldName::new("trace").unwrap(),
                    serde_json::json!(id.as_str()),
                )),
            );
        }
        for pair in ids.windows(2) {
            graph.add_edge(EdgeSpec {
                from: pair[0].clone(),
                to: pair[1].clone(),
                condition: None,
            });
        }
        graph.add_entry(ids[0].clone());
        (graph, ids)
    }

    /// As [`four_node_chain_graph`], except the node at `cancel_at_index`
    /// calls `token.cancel()` (a synchronous method) from directly inside
    /// its own execution -- deterministically placing the cancellation
    /// mid-superstep rather than racing a background poller against an
    /// in-memory chain that runs to completion in well under a millisecond.
    fn four_node_chain_graph_with_cancel_at(
        token: CancellationToken,
        cancel_at_index: usize,
    ) -> (WarGraph, Vec<NodeId>) {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("trace").unwrap(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let ids: Vec<NodeId> = (1..=4).map(|i| NodeId::new(format!("n{i}"))).collect();
        for (i, id) in ids.iter().enumerate() {
            let value = serde_json::json!(id.as_str());
            if i == cancel_at_index {
                let token = token.clone();
                graph.add_node(
                    id.clone(),
                    NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                        token.cancel();
                        let mut d = StateDelta::new();
                        d.set_raw(FieldName::new("trace").unwrap(), value.clone());
                        d
                    })),
                );
            } else {
                graph.add_node(
                    id.clone(),
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        FieldName::new("trace").unwrap(),
                        value,
                    )),
                );
            }
        }
        for pair in ids.windows(2) {
            graph.add_edge(EdgeSpec {
                from: pair[0].clone(),
                to: pair[1].clone(),
                condition: None,
            });
        }
        graph.add_entry(ids[0].clone());
        (graph, ids)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancellation_during_superstep_finishes_it_then_halts_before_the_next() {
        let token = CancellationToken::new();
        // n2 (index 1) cancels the token from within its own execution, so
        // superstep 2 (which n2 belongs to) is always allowed to finish and
        // merge before the top-of-loop check for superstep 3 observes the
        // cancellation.
        let (graph, ids) = four_node_chain_graph_with_cancel_at(token.clone(), 1);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_cancellation_token(token);
        let thread = ThreadId::new("cancel-mid-run").unwrap();

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            engine.start(&graph, thread.clone(), StateDelta::new()),
        )
        .await
        .expect("cancellation must not hang the run")
        .unwrap();

        let waypoint_id = match outcome {
            RunOutcome::Halted { waypoint } => waypoint,
            other => panic!("expected Halted, got {other:?}"),
        };

        let waypoints = ascending_history(&store, &thread).await;
        let halted = waypoints
            .iter()
            .find(|w| w.waypoint_id == waypoint_id)
            .expect("the returned waypoint id must exist in the thread's history");
        assert_eq!(halted.status, WaypointStatus::Halted);
        assert_eq!(
            halted.vanguard,
            vec![ids[2].clone()],
            "the Halted waypoint's vanguard must be exactly the node that would run next (n3)"
        );

        // n4 (superstep 3's downstream node) must never have run.
        let all_node_ids: std::collections::HashSet<String> = waypoints
            .iter()
            .flat_map(|w| w.completed.iter().map(|r| r.node_id.as_str().to_string()))
            .collect();
        assert!(!all_node_ids.contains(ids[3].as_str()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancellation_before_first_superstep_still_yields_a_halted_waypoint() {
        let (graph, ids) = four_node_chain_graph();
        let token = CancellationToken::new();
        token.cancel();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_cancellation_token(token);
        let thread = ThreadId::new("cancel-before-start").unwrap();

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            engine.start(&graph, thread.clone(), StateDelta::new()),
        )
        .await
        .expect("cancellation must not hang the run")
        .unwrap();

        match outcome {
            RunOutcome::Halted { .. } => {}
            other => panic!("expected Halted, got {other:?}"),
        }
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 1);
        assert_eq!(waypoints[0].status, WaypointStatus::Halted);
        assert_eq!(waypoints[0].vanguard, vec![ids[0].clone()]);
        assert!(
            waypoints[0].completed.is_empty(),
            "no node ever ran before the pre-first-superstep cancellation"
        );
    }

    #[tokio::test]
    async fn resume_continues_a_halted_thread_to_normal_completion() {
        let token = CancellationToken::new();
        // n1 (index 0) cancels the token from within its own execution, so
        // exactly one waypoint (superstep 1) is written before the halt.
        let (graph, ids) = four_node_chain_graph_with_cancel_at(token.clone(), 0);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_cancellation_token(token);
        let thread = ThreadId::new("resume-halted").unwrap();

        let halted = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            engine.start(&graph, thread.clone(), StateDelta::new()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(matches!(halted, RunOutcome::Halted { .. }));
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(
            waypoints
                .iter()
                .filter(|w| w.status == WaypointStatus::Halted)
                .count(),
            1
        );

        // A fresh engine, no cancellation token, resumes to completion.
        let resume_engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let resumed = resume_engine.resume(&graph, thread).await.unwrap();
        match resumed {
            RunOutcome::Completed { final_state, .. } => {
                let trace: Vec<String> = final_state
                    .get(&FieldName::new("trace").unwrap())
                    .unwrap()
                    .unwrap_or_default();
                for id in &ids {
                    assert!(trace.contains(&id.as_str().to_string()));
                }
            }
            other => panic!("expected resumed run to complete, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn uncancelled_token_behaves_identically_to_no_token() {
        let (graph_a, _ids_a) = four_node_chain_graph();
        let engine_no_token = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let outcome_a = engine_no_token
            .start(
                &graph_a,
                ThreadId::new("no-token").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();

        let (graph_b, _ids_b) = four_node_chain_graph();
        let engine_uncancelled = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_cancellation_token(CancellationToken::new());
        let outcome_b = engine_uncancelled
            .start(
                &graph_b,
                ThreadId::new("uncancelled-token").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();

        match (outcome_a, outcome_b) {
            (
                RunOutcome::Completed {
                    final_state: state_a,
                    ..
                },
                RunOutcome::Completed {
                    final_state: state_b,
                    ..
                },
            ) => assert_eq!(state_a, state_b),
            other => panic!("expected both runs to complete, got {other:?}"),
        }
    }

    // --- BUG-01 / CF-01: registered-evaluator edge conditions, engine
    // runtime half (`WarEngine::start`). These reproduce BUG-01 on the
    // `WarEngine` path and are committed FAILING (RED) before the fix
    // (GREEN) lands in the same task, per D-05 / traceability protocol
    // step 4.

    /// Evaluator returning a fixed verdict every call.
    struct FixedVerdictEvaluator(bool);

    #[async_trait]
    impl EdgeConditionEvaluator for FixedVerdictEvaluator {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &crate::edge_evaluator::EdgeContext<'_>,
        ) -> Result<bool, crate::edge_evaluator::EdgeEvaluatorError> {
            Ok(self.0)
        }
    }

    /// Evaluator that always fails.
    struct FailingEdgeEvaluator;

    #[async_trait]
    impl EdgeConditionEvaluator for FailingEdgeEvaluator {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &crate::edge_evaluator::EdgeContext<'_>,
        ) -> Result<bool, crate::edge_evaluator::EdgeEvaluatorError> {
            Err(crate::edge_evaluator::EdgeEvaluatorError::Evaluation {
                evaluator: "is_urgent".to_string(),
                reason: "simulated failure".to_string(),
            })
        }
    }

    /// A two-node graph, `source` (entry) -> `target`, connected by one
    /// edge carrying `EdgeCondition::Custom("is_urgent")`. `source` and
    /// `target` write to DIFFERENT fields, so `target`'s field staying
    /// unset is unambiguous evidence `target` never ran (rather than
    /// merely being masked by `source`'s own write).
    fn source_target_custom_edge_graph() -> (WarGraph, NodeId, NodeId) {
        let source_field = FieldName::new("source_marker").unwrap();
        let target_field = FieldName::new("target_marker").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(source_field.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(target_field.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let source = NodeId::new("source");
        let target = NodeId::new("target");
        graph.add_node(
            source.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                source_field,
                serde_json::json!("n/a"),
            )),
        );
        graph.add_node(
            target.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                target_field,
                serde_json::json!("ran"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: source.clone(),
            to: target.clone(),
            condition: Some(EdgeCondition::Custom("is_urgent".to_string())),
        });
        graph.add_entry(source.clone());
        (graph, source, target)
    }

    #[tokio::test]
    async fn registered_engine_evaluator_true_and_false_route_correctly() {
        let target_field = FieldName::new("target_marker").unwrap();

        let (graph_true, ..) = source_target_custom_edge_graph();
        let engine_true = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_edge_evaluator("is_urgent", Arc::new(FixedVerdictEvaluator(true)));
        let outcome_true = engine_true
            .start(
                &graph_true,
                ThreadId::new("engine-evaluator-true").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        match outcome_true {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&target_field).unwrap(),
                    Some("ran".to_string()),
                    "true verdict should route to and execute the target"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        let (graph_false, ..) = source_target_custom_edge_graph();
        let engine_false = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_edge_evaluator("is_urgent", Arc::new(FixedVerdictEvaluator(false)));
        let outcome_false = engine_false
            .start(
                &graph_false,
                ThreadId::new("engine-evaluator-false").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        match outcome_false {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&target_field).unwrap(),
                    None,
                    "false verdict should not route to or execute the target"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn engine_evaluator_error_fails_the_run_naming_edge_and_evaluator() {
        let (graph, source, target) = source_target_custom_edge_graph();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_edge_evaluator("is_urgent", Arc::new(FailingEdgeEvaluator));

        let err = engine
            .start(
                &graph,
                ThreadId::new("engine-evaluator-error").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap_err();

        match err {
            EngineError::EdgeEvaluatorFailed {
                from,
                to,
                evaluator,
                ..
            } => {
                assert_eq!(from, source);
                assert_eq!(to, target);
                assert_eq!(evaluator, "is_urgent");
            }
            other => panic!("expected EdgeEvaluatorFailed, got {other:?}"),
        }
    }
}
