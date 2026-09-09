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
//! - [`directive_parser`] — `DirectiveParser`, `OnParseError`: how a
//!   `NodeSpec::Paladin` node's raw string output becomes a routing
//!   `Directive` (CF-02, D-11). `PlainOutput` is the backward-compatible
//!   default; `StructuredDirective` parses a documented JSON envelope.
//! - [`input_mapping`] — `InputMapping`, `InputMappingError`: the X-03
//!   string bridge a `NodeSpec::Paladin` node renders its input through.
//! - [`node`] — `StateNode`, `NodeContext`, `StateNodeError`.
//! - [`dispatch_registry`] — `DispatchRegistry`, the engine-owned
//!   `DispatchRule::Custom` name -> closure registration (ENG-FR-09).
//! - [`hooks`] — `TraceDispatcher` (ENG-FR-21's bounded, drop-oldest
//!   `TraceSink` forwarder), `NodeInterceptor`/`InterceptDecision`
//!   (ENG-FR-22's ordered, empty-by-default chain). Both are seams with no
//!   consumer yet (Docs 05, 07); ENG-FR-23's cancellation-to-`Halted` path
//!   lives inline in `superstep`/`WarEngine` since it needs no dedicated
//!   type beyond `tokio_util::sync::CancellationToken`.
//! - [`retry`] — `backoff_delay`/`wait_backoff`/`should_retry` (Doc 04
//!   FT-FR-02, D-15): the Aegis retry loop's own backoff math and
//!   cancellation-aware wait, wrapped around the whole per-node dispatch
//!   closure in `superstep` -- OUTSIDE the `hooks` interceptor chain
//!   (D-14).
//! - `superstep` (private) — the superstep loop `start`/`resume` reduce to.
//! - `test_support` (`#[cfg(test)]`) — `RecordingWaypointStore`,
//!   `RecordingPaladinPort` and `CountingFunctionNode`, the doubles this and
//!   later engine plans assert against.

pub mod bridges;
/// Node-cache key composition (Doc 04 FT-FR-20, D-28): the graph
/// fingerprint, node id, resolved input and Paladin configuration
/// fingerprint that address a `NodeCachePort` entry.
pub mod cache_key;
pub mod directive_parser;
pub mod dispatch_registry;
/// `GraphShape`, `to_mermaid`, `to_dot` (OBS-03, OBS-FR-08, D-18): rendering
/// a [`graph::WarGraph`] or [`graph_doc::WarGraphDoc`] as a Mermaid
/// flowchart or a Graphviz digraph for a human to read.
pub mod export;
pub mod graph;
/// `WarGraphDoc` -- the serde/schemars document form of a [`graph::WarGraph`]
/// (PLAT-FR-12, D-31/D-33/D-34): what an assistant version persists, and
/// [`graph_doc::WarGraphDoc::compile`] turns into an executable, validated
/// `WarGraph`.
pub mod graph_doc;
pub mod heartbeat;
pub mod hooks;
pub mod input_mapping;
pub mod node;
/// `EngineRegistries`: the one bundle `WarGraph::validate` and `WarEngine`
/// carry every named-registration registry through (D-13, D-30).
pub mod registries;
pub mod retry;
pub mod shutdown;
mod superstep;
#[cfg(test)]
pub(crate) mod test_support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::edge_evaluator::EdgeConditionEvaluator;
use crate::error_handler::ErrorHandler;
use crate::retry_predicate::RetryPredicateEvaluator;
use paladin_core::platform::container::battalion::BattalionError;
#[cfg(test)]
use paladin_core::platform::container::battlefield::CustomDispatchResolver;
use paladin_core::platform::container::battlefield::{Battlefield, FieldName, StateDelta};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
use paladin_core::platform::container::node_error::{NodeError, NodeErrorSource};
#[cfg(test)]
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::vault::Namespace;
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeId, ThreadId, WaypointId, WaypointStatus,
};
use paladin_ports::output::cancellation_probe::CancellationProbe;
use paladin_ports::output::node_cache_port::NodeCachePort;
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::structured_executor_port::StructuredExecutorPort;
use paladin_ports::output::trace_sink_port::{
    RunFinishStatus, TraceEmitter, TraceEvent, TraceSink,
};
use paladin_ports::output::vault_confined::ConfinedVault;
use paladin_ports::output::vault_port::VaultPort;
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

pub use bridges::{CAMPAIGN_FAN_IN_SEPARATOR, campaign_node_ids, dedicated_output_field};
pub use directive_parser::{DirectiveParseError, DirectiveParser, OnParseError};
pub use dispatch_registry::DispatchRegistry;
pub use export::{GraphShape, ShapeEdge, ShapeKind, ShapeNode, to_dot, to_mermaid};
pub use graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
pub use graph_doc::{CompileError, WARGRAPH_DOC_SCHEMA_VERSION, WarGraphDoc};
pub use heartbeat::HeartbeatHandle;
pub use hooks::{InterceptDecision, NodeInterceptor, TraceDispatcher};
pub use input_mapping::{InputMapping, InputMappingError};
pub use node::{NodeContext, StateNode, StateNodeError};
pub use registries::EngineRegistries;

/// The display line for an [`EngineError::NodeFailed`]: the failing
/// source's own message (a `Function` node's `StateNodeError` text, or a
/// `PaladinError`'s `Display`), so the rendered line matches what
/// `EngineError::Node(StateNodeError(that_text))` rendered for the same
/// failure before the structured variant existed. Sources that carry no
/// message (`Timeout`, `Cancelled`) render through their own `Display`.
fn node_failed_message(err: &NodeError) -> String {
    match &err.source {
        NodeErrorSource::Paladin { message, .. }
        | NodeErrorSource::Llm { message, .. }
        | NodeErrorSource::Function { message } => message.clone(),
        other => other.to_string(),
    }
}

/// A named, engine-registered schema resolved by `SchemaRef::Registered`
/// (D-29, RT-FR-19, plan 26-18).
///
/// Declared here -- engine-registry machinery, not a core/ports value type
/// -- mirroring [`RetryPredicateEvaluator`]/[`ErrorHandler`]'s own placement
/// (D-13, CF-01 precedent): application-layer responsibility, registered
/// under a name via [`WarEngine::with_output_schema`], resolved by
/// [`WarGraph::validate`] (an unregistered name is
/// [`EngineError::UnregisteredOutputSchema`]) before any node runs, and by
/// `engine::superstep`'s Paladin dispatch (an infallible lookup once
/// validation has proven the name present) at execution time.
///
/// Object-safe (no generic method), so `Arc<dyn StructuredSchema>` is the
/// registry's value type -- the SAME object-safety-at-the-JSON-level
/// discipline [`paladin_ports::output::structured_executor_port::StructuredExecutorPort`]
/// (D-27) already establishes for this phase's structured-output surface.
pub trait StructuredSchema: Send + Sync {
    /// Validate `value` against this schema. [`TypedSchema<T>`]'s
    /// implementation is `serde_json::from_value::<T>` -- FULL typed
    /// validation by deserialization (D-29, D-30), not the object-safe
    /// port's partial [`paladin_core::platform::container::structured::shape_check`].
    fn validate(&self, value: &serde_json::Value) -> Result<(), String>;

    /// The JSON Schema this registration resolves to, for a
    /// `SchemaRef::Registered(name)` node -- resolved once, before
    /// dispatch, by `engine::superstep`'s Paladin arm, exactly as a
    /// `SchemaRef::Inline` node's own schema value is used directly.
    fn to_json_schema(&self) -> serde_json::Value;
}

/// A [`StructuredSchema`] whose [`StructuredSchema::validate`] is
/// `serde_json::from_value::<T>` -- full typed validation by
/// deserialization (D-29, D-30), not the object-safe port's partial shape
/// check.
///
/// Carries its JSON Schema as a plain `serde_json::Value` supplied at
/// construction, rather than deriving it via `schemars::schema_for!`:
/// `schemars` is a direct dependency of the facade crate ONLY (D-26,
/// ADR-0015's core/ports dependency allowlist) -- `paladin-battalion` (this
/// crate) does not depend on it, and must not gain the dependency just for
/// this type. A caller that already has a `schemars`-derived schema (e.g.
/// the facade, or a test) passes its rendered `serde_json::Value` in
/// directly.
pub struct TypedSchema<T> {
    schema: serde_json::Value,
    _marker: std::marker::PhantomData<T>,
}

impl<T> TypedSchema<T> {
    /// Construct a `TypedSchema<T>` from a JSON Schema value. `T`'s own
    /// `Deserialize` implementation is what [`StructuredSchema::validate`]
    /// checks a resolved value against -- `schema` itself is never
    /// introspected or regenerated from `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::{StructuredSchema, TypedSchema};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Weather {
    ///     city: String,
    /// }
    ///
    /// let schema = TypedSchema::<Weather>::new(serde_json::json!({
    ///     "type": "object",
    ///     "required": ["city"],
    ///     "properties": {"city": {"type": "string"}}
    /// }));
    /// assert!(schema.validate(&serde_json::json!({"city": "Oslo"})).is_ok());
    /// assert!(schema.validate(&serde_json::json!({"city": 4})).is_err());
    /// ```
    pub fn new(schema: serde_json::Value) -> Self {
        Self {
            schema,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> StructuredSchema for TypedSchema<T>
where
    T: serde::de::DeserializeOwned + Send + Sync,
{
    fn validate(&self, value: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<T>(value.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn to_json_schema(&self) -> serde_json::Value {
        self.schema.clone()
    }
}

impl EngineError {
    /// The structured [`NodeError`] this error carries, if it is an
    /// [`EngineError::NodeFailed`] -- `None` for every other variant,
    /// including an engine-limit failure and a no-Aegis node failure.
    pub fn node_error(&self) -> Option<&NodeError> {
        match self {
            EngineError::NodeFailed(err) => Some(err),
            _ => None,
        }
    }
}

impl From<EngineError> for BattalionError {
    /// [`EngineError::NodeFailed`] maps to [`BattalionError::Node`] carrying
    /// the identical [`NodeError`] (D-08); every other variant, which had no
    /// `BattalionError` mapping before this phase, renders through the
    /// existing generic [`BattalionError::CampaignError`] line (the graph
    /// engine is the Campaign pattern's execution surface) so no other
    /// variant gains a new structured shape here.
    fn from(err: EngineError) -> Self {
        match err {
            EngineError::NodeFailed(node_error) => BattalionError::Node(node_error),
            other => BattalionError::CampaignError(other.to_string()),
        }
    }
}

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
    /// The run is paused awaiting external input (HITL-01, D-02). Carries
    /// only the STILL-UNANSWERED requests -- for the initial suspension
    /// this phase produces, every request in the persisted `Waypoint`'s
    /// `AwaitingInput.parleys` list.
    AwaitingInput {
        /// Every outstanding (unanswered) request.
        parleys: Vec<ParleyRequest>,
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

impl RunOutcome {
    /// The structured [`NodeError`] a [`RunOutcome::Failed`] carries when its
    /// `error` is an [`EngineError::NodeFailed`] -- the same value the
    /// failed `Waypoint`'s `WaypointStatus::Failed.node_error` records
    /// (D-08). `None` for every other outcome and every other failure.
    pub fn node_error(&self) -> Option<&NodeError> {
        match self {
            RunOutcome::Failed { error, .. } => error.node_error(),
            _ => None,
        }
    }
}

/// The [`RunFinishStatus`] a `superstep::run`/`run_with_namespace` call's own
/// `Result<RunOutcome, EngineError>` maps onto (D-02, D-04; closes
/// 27-CONTEXT D-25's correction, T-28-03-04). An engine-limit failure (e.g.
/// `RecursionLimitExceeded`) is returned as a bare `Err` via `?`, never
/// wrapped in `Ok(RunOutcome::Failed { .. })` -- so both paths map to
/// `Failed` here, or `RunFinished.status` could never be trusted to
/// distinguish success from failure for exactly the run shapes most likely
/// to fail.
fn run_finish_status(outcome: &Result<RunOutcome, EngineError>) -> RunFinishStatus {
    match outcome {
        Ok(RunOutcome::Completed { .. }) => RunFinishStatus::Completed,
        Ok(RunOutcome::Failed { .. }) | Err(_) => RunFinishStatus::Failed,
        Ok(RunOutcome::Halted { .. }) => RunFinishStatus::Halted,
        Ok(RunOutcome::AwaitingInput { .. }) => RunFinishStatus::AwaitingInput,
    }
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

    /// The run's total wall clock exceeded `EngineLimits::run_timeout`
    /// (Doc 04 FT-FR-10, D-20, ENG-FR-03; plan 25-09). Takes the SAME
    /// `Failed`-Waypoint and `RunOutcome::Failed` path
    /// [`EngineError::RecursionLimitExceeded`] and
    /// [`EngineError::NodeVisitLimitExceeded`] take -- consistent by
    /// construction (`engine::superstep`'s one limit-failure helper), not a
    /// second implementation. Raised either at a superstep boundary (budget
    /// already exhausted) or mid-superstep when the budget cuts an in-flight
    /// attempt, in which case that attempt's structured
    /// `NodeError { source: Timeout(EngineRun), .. }` rides on the
    /// Waypoint's `WaypointStatus::Failed.node_error`.
    #[error("run timeout exceeded: {elapsed:?} elapsed against a limit of {limit:?}")]
    RunTimeoutExceeded {
        /// How long the run had been executing when the budget was found
        /// exhausted.
        elapsed: std::time::Duration,
        /// The configured `EngineLimits::run_timeout` that was hit.
        limit: std::time::Duration,
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
    Node(#[from] StateNodeError),

    /// An Aegis-governed node's execution failed (Doc 04 D-08, FT-FR-02):
    /// its retries were exhausted, or its error was not retry-eligible
    /// under its policy. Carries the structured [`NodeError`] the failed
    /// `Waypoint`'s `WaypointStatus::Failed.node_error` also records, so
    /// `RunOutcome::Failed` and `BattalionError::Node` expose the same
    /// value. Renders as `node execution error: {message}` -- byte-identical
    /// to the `EngineError::Node` line the same failure produced before
    /// this variant existed, so the display line a human reads is unchanged
    /// (X-03).
    ///
    /// Supersedes `EngineError::Node` ONLY on the exhausted-failure path of
    /// a node that has a resolved Aegis; a node with no Aegis still fails
    /// through `EngineError::Node` exactly as before Phase 25 (D-09), and
    /// `EngineError::Node` remains the engine's generic internal-error
    /// variant everywhere else (join errors, missing worker resources, ...).
    #[error("node execution error: {}", node_failed_message(.0))]
    NodeFailed(NodeError),

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

    /// A `Directive`'s `NextStep::Goto` named a target not declared in the
    /// graph (CF-02, D-08a). Validated the moment the Directive is
    /// received, before any routing state changes -- a `Goto` never
    /// silently drops or ignores an unknown target.
    #[error("node {from} returned NextStep::Goto naming undeclared node {to}")]
    GotoUnknownNode {
        /// The node whose `Directive` named the unknown target.
        from: NodeId,
        /// The undeclared `Goto` target.
        to: NodeId,
    },

    /// **Superseded (Phase 24, HITL-01):** a `Directive`'s `NextStep::Parley`
    /// no longer fails the run -- it suspends it, persisting a
    /// `WaypointStatus::AwaitingInput` checkpoint and returning
    /// `RunOutcome::AwaitingInput` (see `superstep`'s Parley arm). This
    /// variant is retained, unconstructed, because X-03 forbids removing a
    /// public `EngineError` variant before v0.11.0 -- it is no longer
    /// reachable from any production code path in this engine.
    #[error(
        "node {node} returned NextStep::Parley, which this phase does not support (Phase 24 lands suspension)"
    )]
    ParleyNotSupported {
        /// The node whose `Directive` returned `Parley`.
        node: NodeId,
    },

    /// `WarEngine::resume_with` loaded a thread whose latest `Waypoint`
    /// status is NOT `AwaitingInput` (HITL-02, D-10): only a suspended
    /// thread can be advanced by delivering parley responses.
    #[error("thread {thread} is not awaiting input (status: {status})")]
    ThreadNotAwaitingInput {
        /// The thread `resume_with` was called against.
        thread: ThreadId,
        /// The loaded Waypoint's actual status, `Debug`-formatted.
        status: String,
    },

    /// `WarEngine::resume_with` was given a response whose `parley_id` does
    /// not match any request on the loaded `AwaitingInput` Waypoint
    /// (HITL-02, D-10, T-24-01): checked against the loaded Waypoint's OWN
    /// `parleys` list for the requested thread only -- never a global
    /// parley-id lookup across threads.
    #[error("unknown parley id: {parley_id}")]
    UnknownParleyId {
        /// The response's `parley_id`, absent from the loaded thread's
        /// outstanding parleys.
        parley_id: ParleyId,
    },

    /// A plain `WarEngine::resume`/`resume_with_options` call loaded a
    /// thread whose latest `Waypoint` status is `AwaitingInput` (HITL-01,
    /// D-11): only `WarEngine::resume_with` may advance a suspended thread.
    /// Returned BEFORE the generic vanguard-restore fallthrough that
    /// `resume_with_options` otherwise uses -- without this guard, that
    /// fallthrough would silently re-run the parleying node(s) as ordinary
    /// vanguard entries, discarding the pending suspension (RESEARCH.md
    /// Pitfall 2). No Waypoint is written.
    #[error("thread {thread} is awaiting input and cannot be resumed with plain `resume`")]
    ThreadAwaitingInput {
        /// The thread a plain `resume`/`resume_with_options` was called
        /// against.
        thread: ThreadId,
        /// Every outstanding (unanswered) request from the loaded
        /// `AwaitingInput` Waypoint.
        parleys: Vec<ParleyRequest>,
    },

    /// `WarEngine::resume_with` was given a response naming a `parley_id`
    /// that is already answered -- either already present in the loaded
    /// `AwaitingInput` Waypoint's own `responses` list (a prior
    /// `resume_with` call already accepted a response for it), or named a
    /// SECOND time by a later response within the SAME call (HITL-02,
    /// D-10, X-06). Two responses answering the same `parley_id` in one
    /// submission are BOTH rejected: the first is accepted into the
    /// working set before the second is checked, so the second is what
    /// this error reports -- never last-wins, never first-wins silently
    /// accepting the first and ignoring the second.
    #[error("parley already answered: {parley_id}")]
    ParleyAlreadyAnswered {
        /// The `parley_id` a response was submitted for that already has
        /// an accepted answer.
        parley_id: ParleyId,
    },

    /// `WarEngine::resume_with` was given a response whose `value` does not
    /// satisfy its own request's `ParleyKind` (HITL-02, D-10): `Approval`
    /// must be a bool or one of true/false/yes/no/approve/deny
    /// (case-insensitive); `Choice` must be a string among the request's
    /// own `choices`; `FreeText` must be a string; `StateEdit` must
    /// deserialise as a `StateDelta` naming only fields declared in the
    /// graph's own schema -- an undeclared field rejects THIS response,
    /// never the run and never a partial edit (T-24-13). Checked through
    /// the SAME per-kind validator
    /// [`graph::validate_parley_value_for_kind`] a Gate's own `on_expire`
    /// default (`WarGraph::validate`, 24-02) and a Directive's raise-time
    /// default (`DirectiveParser::parse`, 24-03) are checked against
    /// (T-24-06) -- never a second, weaker check for the same structural
    /// rules.
    #[error("parley {parley_id} response shape invalid: {reason}")]
    ResponseShapeInvalid {
        /// The parley whose submitted value failed shape validation.
        parley_id: ParleyId,
        /// Why the value was rejected.
        reason: String,
    },

    /// `WarEngine::resume_with` found an outstanding parley whose
    /// `expires_at` has passed, evaluated lazily against `Utc::now()` at
    /// resume time -- no timer, no clock abstraction (HITL-02, D-12,
    /// D-13). Under `on_expire: FailRun`, this error is returned AFTER a
    /// `Failed` Waypoint naming the expired parley is persisted; the
    /// thread is thereafter advanced only by `replay`/`fork` from an
    /// earlier Waypoint, never by `resume` or `resume_with` again. A
    /// future `OnExpire` variant this engine does not yet recognise also
    /// fails closed with this same error, rather than being silently
    /// treated as still open.
    #[error("parley {parley_id} expired at {expires_at}")]
    ParleyExpired {
        /// The expired parley.
        parley_id: ParleyId,
        /// When it expired.
        expires_at: chrono::DateTime<Utc>,
    },

    /// A plain `WarEngine::resume`/`resume_with_options` call loaded a
    /// thread whose latest Waypoint is `Failed` (HITL-02, D-12) -- e.g. a
    /// `FailRun` parley expiry. Returned BEFORE the generic
    /// vanguard-restore fallthrough `resume_with_options` otherwise uses,
    /// mirroring `EngineError::ThreadAwaitingInput`'s guard: a `Failed`
    /// Waypoint records a terminal outcome, never "more work pending," so
    /// the thread is thereafter advanced only by `replay`/`fork` from an
    /// earlier Waypoint (a later plan), never by `resume`/`resume_with`
    /// again. No Waypoint is written.
    #[error("thread {thread} already failed: {error}")]
    ThreadAlreadyFailed {
        /// The thread a plain `resume`/`resume_with_options` was called
        /// against.
        thread: ThreadId,
        /// The recorded failure reason from the loaded `Failed` Waypoint.
        error: String,
        /// The node whose execution caused the failure.
        failed_node: NodeId,
    },

    /// A `NodeSpec::Battalion` node's child run suspended awaiting a Parley
    /// (HITL-01, D-04): not supported this phase. PRD 03 is silent on
    /// suspension propagating through a nested Battalion; propagating a
    /// child's parley to the parent is a deferred idea for a later phase to
    /// promote, not a design this phase attempts. Raise the parley in the
    /// PARENT graph instead, today.
    #[error(
        "battalion node {node} (child thread {child_thread}): child run paused awaiting input, \
         which this phase does not support -- raise the parley in the parent graph instead; \
         propagating a child's parley to the parent is a deferred idea for a later phase"
    )]
    ParleyInChildUnsupported {
        /// The Battalion node whose child run suspended.
        node: NodeId,
        /// The child thread that suspended.
        child_thread: ThreadId,
    },

    /// A `NodeSpec::Paladin` node's `DirectiveParser::StructuredDirective`
    /// (CF-02, D-11) could not extract a valid JSON envelope from the
    /// node's output under `OnParseError::FailRun`. Never resolved by any
    /// default branch: `OnParseError::FallbackPlain` is the node author's
    /// explicit opt-in to a different resolution, not something the engine
    /// chooses on its own.
    #[error("node {node} failed to parse a StructuredDirective envelope: {reason}")]
    DirectiveParseFailed {
        /// The node whose output failed to parse.
        node: NodeId,
        /// Why extraction/deserialization of the envelope failed.
        reason: String,
    },

    /// A `Directive`'s `NextStep::Muster` carried an empty task list (CF-03,
    /// D-13). `NextStep::Edges` and `NextStep::End` are the two ways to
    /// express "no fan-out"; an empty `Muster` is a planner defect, rejected
    /// at Directive-receipt time before any task starts, never silently
    /// treated as a no-op.
    #[error("node {node} returned an empty NextStep::Muster task list")]
    EmptyMuster {
        /// The mustering node.
        node: NodeId,
    },

    /// Two tasks in the same `NextStep::Muster` shared a `task_key` (CF-03,
    /// D-13). Detected before any task is dispatched, so worker deltas can
    /// merge in a total `task_key` order with no tie to break.
    #[error("node {node} returned NextStep::Muster with a duplicate task_key: {task_key}")]
    DuplicateMusterTaskKey {
        /// The mustering node.
        node: NodeId,
        /// The duplicated `task_key`.
        task_key: String,
    },

    /// A `NextStep::Muster` requested more tasks than
    /// `EngineLimits::max_muster_tasks` allows (CF-FR-13, D-16, T-23-18).
    /// Detected before any task is dispatched; `requested` is widened from
    /// `usize`, never compared by narrowing `limit` with `as u32`, so a task
    /// list longer than `u32::MAX` cannot wrap into a passing count.
    #[error(
        "node {node} returned NextStep::Muster requesting {requested} task(s), exceeding \
         max_muster_tasks ({limit})"
    )]
    MusterTaskLimitExceeded {
        /// The mustering node.
        node: NodeId,
        /// The number of tasks requested.
        requested: usize,
        /// The configured `EngineLimits::max_muster_tasks`.
        limit: u32,
    },

    /// A `NextStep::Muster` task's `worker` named a `NodeId` not declared in
    /// the graph (CF-03, T-23-22). Detected before any task is dispatched.
    #[error("node {node} returned NextStep::Muster naming undeclared worker {worker}")]
    MusterUnknownWorker {
        /// The mustering node.
        node: NodeId,
        /// The undeclared worker id.
        worker: NodeId,
    },

    /// A `NextStep::Muster` task's `worker` named a node declared in the
    /// graph but not registered via [`graph::WarGraph::add_worker_template`]
    /// (CF-03, D-12, T-23-22). Detected before any task is dispatched.
    #[error(
        "node {node} returned NextStep::Muster naming {worker}, which is not a worker template"
    )]
    MusterWorkerNotATemplate {
        /// The mustering node.
        node: NodeId,
        /// The worker id, declared but not marked as a worker template.
        worker: NodeId,
    },

    /// `WarGraph::validate` found one or more nodes marked
    /// [`graph::WarGraph::add_worker_template`] also declared as an entry
    /// point (CF-03, D-12): a worker template runs only when mustered,
    /// never on its own.
    #[error("worker template(s) declared as entry point(s): {reason}")]
    WorkerTemplateIsEntry {
        /// Every offending worker-template node, in the graph's
        /// registration order.
        nodes: Vec<NodeId>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found one or more nodes marked
    /// [`graph::WarGraph::add_worker_template`] with a static incoming edge
    /// (CF-03, D-12): a worker template runs only as a `NextStep::Muster`
    /// task dispatch, so no static edge may target it.
    #[error("worker template(s) with a static incoming edge: {reason}")]
    WorkerTemplateHasStaticIncomingEdge {
        /// Every offending worker-template node, in the graph's
        /// registration order.
        nodes: Vec<NodeId>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a Battlefield schema field named with the
    /// `muster.` prefix (CF-03, D-15): that namespace is reserved for
    /// `InputMapping`'s `{muster.payload}`/`{muster.task_key}` placeholders,
    /// resolved from a Muster worker's `NodeContext`, never from the
    /// Battlefield.
    #[error("schema field(s) reserved for the muster. namespace: {reason}")]
    MusterPrefixSchemaField {
        /// Every offending schema field name, sorted.
        fields: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a Battlefield schema field named with the
    /// `parley.` prefix (HITL-01, D-07, T-24-09): that namespace is reserved
    /// for `InputMapping`'s `{parley.value}`/`{parley.prompt}`/
    /// `{parley.kind}`/`{parley.responded_by}` placeholders, resolved from a
    /// parleying node's own `NodeContext` `ParleyResponse`, never from the
    /// Battlefield.
    #[error("schema field(s) reserved for the parley. namespace: {reason}")]
    ParleyPrefixSchemaField {
        /// Every offending schema field name, sorted.
        fields: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `resume`/`resume_with_options` loaded a mid-muster progress
    /// Waypoint (CF-FR-12, D-14) whose `MusterProgress.tasks` names a
    /// `worker` the (possibly new, under `allow_graph_change`) graph no
    /// longer declares, or declares but no longer registers as a worker
    /// template. Never silently dropped -- mirrors
    /// [`EngineError::VanguardNodeMissing`]'s "a resume that continued
    /// without a node the caller expected to run would look like a
    /// successful resume that quietly skipped work" rationale, applied to a
    /// restored Muster's worker set.
    #[error(
        "resume mid-muster progress record (mustering node {node}) names a worker missing or \
         not a worker template in the resume graph: {worker}"
    )]
    MusterProgressWorkerMissing {
        /// The node whose `NextStep::Muster` produced the loaded progress
        /// record.
        node: NodeId,
        /// The restored task's `worker`, absent from the resume graph or
        /// no longer a worker template.
        worker: NodeId,
    },

    /// `WarGraph::validate` found one or more `NodeSpec::Battalion`
    /// `StateMap` pairs naming a field absent from the relevant schema
    /// (CF-FR-14, D-19): an `inputs` pair's `parent` field must exist in
    /// the parent schema and `child` field in the child graph's schema; an
    /// `outputs` pair's `child` field must exist in the child schema and
    /// `parent` field in the parent schema. Carries EVERY offending field
    /// in one error, mirroring [`EngineError::MusterPrefixSchemaField`]'s
    /// "report the whole problem at once" discipline.
    #[error("invalid battalion StateMap field(s): {reason}")]
    BattalionStateMapUnknownField {
        /// Every offending field, pre-formatted with its owning node,
        /// direction (`input`/`output`) and side (`parent`/`child`).
        fields: Vec<String>,
        /// Explains the rule.
        reason: String,
    },

    /// `WarGraph::validate` found a `NodeSpec::Battalion` node whose child
    /// graph -- or a descendant of it -- has a fingerprint already present
    /// on its own descent path (CF-FR-16, D-19): recursive embedding,
    /// caught by a path-set walk over CHILD FINGERPRINTS rather than
    /// pointer identity, before any node ever executes. This bounds
    /// nesting depth by construction; a deep but genuinely ACYCLIC nesting
    /// of distinct graphs still validates.
    #[error("recursive battalion embedding: {reason}")]
    RecursiveEmbedding {
        /// The fingerprint path from the outermost graph down to the
        /// re-encountered child.
        path: Vec<GraphFingerprint>,
        /// Explains the fixpoint rule and names the offending node.
        reason: String,
    },

    /// A `NodeSpec::Battalion` node's child run failed (CF-FR-16, D-21):
    /// the child's own `WarEngine`-equivalent superstep loop returned
    /// `RunOutcome::Failed` or an outright `Err`. Named structurally --
    /// the failing Battalion node and the child thread it ran under (X-06)
    /// -- rather than folded into a bare interpolated `NodeError` string,
    /// with the child's own typed error preserved as the source.
    #[error("battalion node {node} child run failed on thread {child_thread}: {source}")]
    BattalionChildFailed {
        /// The Battalion node whose child run failed.
        node: NodeId,
        /// The child thread the failing run executed under.
        child_thread: ThreadId,
        /// The child engine's own error.
        #[source]
        source: Box<EngineError>,
    },

    /// `WarGraph::validate` found a [`crate::engine::graph::NodeSpec::Gate`]
    /// node whose `kind` requires an `output_field` (`Approval`/`Choice`/
    /// `FreeText`) but declares `None` (HITL-01, D-05).
    #[error("gate {node} of kind {kind:?} requires an output_field")]
    GateOutputFieldRequired {
        /// The offending Gate node.
        node: NodeId,
        /// The Gate's kind.
        kind: ParleyKind,
    },

    /// `WarGraph::validate` found a `ParleyKind::StateEdit` Gate node
    /// declaring `output_field: Some(_)` (HITL-01, D-05): a `StateEdit`
    /// gate returns its response as the node's own delta and writes no
    /// named field.
    #[error("gate {node} of kind StateEdit must not declare an output_field (found '{field}')")]
    GateOutputFieldMustBeAbsent {
        /// The offending Gate node.
        node: NodeId,
        /// The `output_field` that must be absent.
        field: FieldName,
    },

    /// `WarGraph::validate` found a Gate node's `output_field` naming a
    /// field absent from the graph's schema (HITL-01, D-05).
    #[error("gate {node}'s output_field '{field}' is not declared in the graph schema")]
    GateOutputFieldUnknown {
        /// The offending Gate node.
        node: NodeId,
        /// The undeclared field name.
        field: FieldName,
    },

    /// `WarGraph::validate` found a Gate node's `output_field` declared
    /// with a schema-default type incompatible with its `kind` (HITL-01,
    /// D-05): `Approval` accepts a `Bool` or `String` default;
    /// `Choice`/`FreeText` accept only a `String` default.
    #[error(
        "gate {node}'s output_field '{field}' has an incompatible type for kind {kind:?}: {reason}"
    )]
    GateOutputFieldTypeIncompatible {
        /// The offending Gate node.
        node: NodeId,
        /// The incompatible field.
        field: FieldName,
        /// The Gate's kind.
        kind: ParleyKind,
        /// Explains the incompatibility.
        reason: String,
    },

    /// `WarGraph::validate` found a Gate node's
    /// `on_expire: OnExpire::ResumeWithDefault` value that does not satisfy
    /// its own `kind` (HITL-01, D-12, T-24-06): checked at graph-validate
    /// time through the SAME per-kind validator a real submitted response
    /// is checked against, so an unchecked default can never bypass an
    /// approval gate.
    #[error("gate {node}'s on_expire default value is invalid for kind {kind:?}: {reason}")]
    GateResumeWithDefaultInvalid {
        /// The offending Gate node.
        node: NodeId,
        /// The Gate's kind.
        kind: ParleyKind,
        /// Explains why the default value is invalid.
        reason: String,
    },

    /// [`WarEngine::replay`] or [`WarEngine::fork`] named a `from` Waypoint
    /// id that does not exist on `thread` (HITL-03, D-16): the SAME
    /// "missing is `None`, not an error" contract
    /// [`WaypointPort::get`](paladin_ports::output::waypoint_port::WaypointPort::get)
    /// documents is here turned into a typed engine error, mirroring how
    /// [`EngineError::ThreadNotFound`] turns [`WaypointPort::latest`]'s own
    /// `None` into a typed error at THIS layer. Nothing is persisted when
    /// this is returned.
    #[error("waypoint {waypoint} not found on thread {thread}")]
    WaypointNotFound {
        /// The thread `replay`/`fork` was called against.
        thread: ThreadId,
        /// The unknown starting `WaypointId`.
        waypoint: WaypointId,
    },

    /// `WarGraph::validate` found one or more `RetryPredicate::Custom(name)`
    /// values (D-13, CF-01 precedent, FT-FR-13) reachable from any node's
    /// resolved `Aegis` with no evaluator registered via
    /// [`WarEngine::with_retry_predicate`]. Checked before any node
    /// executes; never silently degraded to "do not retry" at runtime.
    /// Carries EVERY offending name, sorted and deduplicated, mirroring
    /// [`EngineError::UnregisteredEdgeCondition`]'s discipline.
    #[error("unregistered custom retry predicate(s): {}", names.join(", "))]
    UnregisteredRetryPredicate {
        /// Every unregistered `RetryPredicate::Custom` name, sorted and
        /// deduplicated.
        names: Vec<String>,
    },

    /// `WarGraph::validate` found one or more `ErrorHandlerSpec::Custom(name)`
    /// values (D-13, CF-01 precedent, FT-FR-13) reachable from any node's
    /// resolved `Aegis` with no handler registered via
    /// [`WarEngine::with_error_handler`]. Checked before any node executes;
    /// never silently degraded to a default at runtime. Carries EVERY
    /// offending name, sorted and deduplicated, mirroring
    /// [`EngineError::UnregisteredEdgeCondition`]'s discipline.
    #[error("unregistered custom error handler(s): {}", names.join(", "))]
    UnregisteredErrorHandler {
        /// Every unregistered `ErrorHandlerSpec::Custom` name, sorted and
        /// deduplicated.
        names: Vec<String>,
    },

    /// `WarGraph::validate` found one or more node ids registered via
    /// [`graph::WarGraph::set_aegis`] that are not declared nodes (D-10,
    /// plan 25-03) -- the node was never added, or was renamed/removed
    /// after `set_aegis` was called. Carries EVERY offending node id, in
    /// sorted order, mirroring [`EngineError::UnreachableNode`]'s "report
    /// the whole problem at once" discipline.
    #[error("aegis set on undeclared node(s): {reason}")]
    AegisOnUndeclaredNode {
        /// Every offending node id, sorted.
        nodes: Vec<NodeId>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found an `Aegis` policy attached to a node whose
    /// kind does not support it (D-12, plan 25-03): a `NodeSpec::Battalion`
    /// node rejects `retry`/`cache` (its child's Waypoints are durable
    /// state, and attempt isolation / cache replay would need per-attempt
    /// child-thread namespacing this phase does not build); a
    /// `NodeSpec::Gate` node rejects any `Aegis` at all (no attempt to
    /// retry, time or cache; expiry is `on_expire`'s job). Carries EVERY
    /// offending (node, policy, reason) triple, pre-formatted, mirroring
    /// [`EngineError::BattalionStateMapUnknownField`]'s discipline.
    #[error("aegis unsupported for node kind: {reason}")]
    AegisUnsupportedForNodeKind {
        /// Every offending node/policy pairing, pre-formatted with its
        /// reason.
        offenders: Vec<String>,
        /// Explains the rule.
        reason: String,
    },

    /// `WarGraph::validate` found a resolved `Aegis`'s `RetryPolicy` with
    /// `max_attempts == 0` (D-09, plan 25-03) -- never interpreted as
    /// unlimited retries, and never silently treated as a single attempt.
    #[error("invalid retry policy for node {node}: {reason}")]
    RetryPolicyInvalid {
        /// The offending node.
        node: NodeId,
        /// Why the policy was rejected.
        reason: String,
    },

    /// `WarGraph::validate` found a resolved `Aegis`'s `TimeoutPolicy` with
    /// `Some(Duration::ZERO)` on `run_timeout` or `idle_timeout` (D-09, plan
    /// 25-03) -- never interpreted as an immediate-kill timeout. A
    /// `TimeoutPolicy` with both fields `None` is a valid no-op and never
    /// reaches this error.
    #[error("invalid timeout policy for node {node}: {reason}")]
    TimeoutPolicyInvalid {
        /// The offending node.
        node: NodeId,
        /// Why the policy was rejected.
        reason: String,
    },

    /// `WarGraph::validate` found one or more `ErrorHandlerSpec::Route { to }`
    /// values (D-21, plan 25-10, FT-FR-11) reachable from any node's
    /// resolved `Aegis` whose `to` is not a declared node. Checked before
    /// any node executes -- a Route never silently drops its target at
    /// runtime. Carries EVERY offending (node, target) pairing, pre-formatted,
    /// mirroring [`EngineError::AegisUnsupportedForNodeKind`]'s discipline.
    #[error("route target undeclared: {reason}")]
    RouteTargetUnknown {
        /// Every offending node/target pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found one or more `ErrorHandlerSpec::Route { to }`
    /// values (D-21, D-22, plan 25-10) whose `to` is a worker template
    /// (`WarGraph::add_worker_template`). A worker template runs only as a
    /// `NextStep::Muster` task dispatch, never as an ordinary vanguard
    /// entry, so it can never be a routing target -- the message names the
    /// alternative (handle the failure at the aggregator, or `Absorb` it on
    /// the template). Carries EVERY offending pairing, pre-formatted.
    #[error("route target is a worker template: {reason}")]
    RouteTargetIsWorkerTemplate {
        /// Every offending node/target pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule, names the offenders and the alternative.
        reason: String,
    },

    /// `WarGraph::validate` found one or more
    /// `ErrorHandlerSpec::Route { error_field }` values (D-21, plan 25-10)
    /// naming a field the `BattlefieldSchema` does not declare. The
    /// serialized `NodeError` is written into `error_field` as an ordinary
    /// delta, and `Battlefield::merge` would reject an undeclared field at
    /// runtime -- this clause rejects it before any node executes instead.
    /// Carries EVERY offending (node, field) pairing, pre-formatted.
    #[error("route error_field undeclared: {reason}")]
    RouteErrorFieldUndeclared {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found one or more
    /// `ErrorHandlerSpec::Route { error_field }` values (D-21, plan 25-10,
    /// T-25-46) naming a declared field whose `DispatchRule` is `Sum`. The
    /// value written there is a serialized `NodeError` JSON object, and an
    /// object cannot be summed -- the merge would fail with `TypeMismatch`
    /// at the worst possible moment (after the node has already failed).
    /// Declare the field with any other dispatch (`LastWrite` is the usual
    /// choice). Carries EVERY offending (node, field) pairing, pre-formatted.
    #[error("route error_field dispatch invalid: {reason}")]
    RouteErrorFieldDispatchInvalid {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains why a serialized error object is not summable and
        /// names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found one or more
    /// `ErrorHandlerSpec::Absorb { fallback_delta }` values (D-21, plan
    /// 25-10, FT-FR-12) whose delta writes a field the `BattlefieldSchema`
    /// does not declare. An EMPTY `fallback_delta` is legal (it merges
    /// nothing); only undeclared fields are rejected. Carries EVERY
    /// offending (node, field) pairing, pre-formatted.
    #[error("absorb fallback_delta invalid: {reason}")]
    AbsorbDeltaSchemaInvalid {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found one or more worker templates
    /// (`WarGraph::add_worker_template`) whose resolved `Aegis.on_error` is
    /// `ErrorHandlerSpec::Route` (D-22, plan 25-11). A worker template runs
    /// only as a `NextStep::Muster` task dispatch, and a mustered task's
    /// result is exactly ONE contribution to its Muster's aggregation --
    /// routing out of a single task would leave that aggregation with an
    /// undefined shape (what does the aggregator read for the routed slot?),
    /// so the case is rejected rather than guessed. Only `Absorb` and a
    /// delta-only `Custom` handler are permitted on a template; the message
    /// names the alternative -- handle the failure at the aggregator node.
    /// Carries EVERY offending template, pre-formatted.
    #[error("handler not allowed on worker template: {reason}")]
    HandlerNotAllowedOnWorkerTemplate {
        /// Every offending template/handler pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule, names the offenders and the alternative.
        reason: String,
    },

    /// An `ErrorHandlerSpec::Custom` handler dispatched for a FAILED mustered
    /// task (D-22, plan 25-11) returned a `Directive` whose `NextStep` is not
    /// `Edges`. Inside a Muster a handler may only contribute a delta -- that
    /// delta becomes the task's contribution to the aggregation, in the same
    /// shape a successful task contributes -- because `Goto`, `End`,
    /// `Parley` and `Muster` all change control flow for the WHOLE run from
    /// inside one of many concurrent tasks, and the aggregation's semantics
    /// for that are undefined. Whether a handler is delta-only is a runtime
    /// property, so unlike a `Route` (rejected at validation) this surfaces
    /// when the handler actually runs; both `node` and `task_key` are named
    /// so the failing task is identifiable in a wide fan-out. Handle the
    /// failure at the aggregator node instead.
    #[error(
        "muster handler must be delta-only: worker template `{node}` task `{task_key}` handler \
         returned NextStep::{returned}; inside a Muster a handler may only return \
         NextStep::Edges (its delta becomes the task's contribution to the aggregation) -- \
         handle the failure at the aggregator node instead"
    )]
    MusterHandlerMustBeDeltaOnly {
        /// The worker template whose task failed.
        node: NodeId,
        /// The `task_key` of the failed mustered task.
        task_key: String,
        /// The offending `NextStep` arm's name (`Goto`, `End`, `Parley` or
        /// `Muster`).
        returned: String,
    },

    /// `WarGraph::validate_node_cache_backend` found one or more nodes
    /// carrying a resolved `Aegis.cache` policy while the `WarEngine` has no
    /// backend wired via [`WarEngine::with_node_cache`] (D-29, plan 25-13,
    /// FT-FR-18). Fail-closed: a graph author who asked for caching and
    /// silently got none would have no signal, so this is a typed error
    /// before any node runs -- never a degradation to "no caching". A node
    /// inside a `NodeSpec::Battalion` child graph is named as
    /// `{battalion node}/{child node}`. Carries EVERY offender, sorted,
    /// mirroring [`EngineError::AegisOnUndeclaredNode`]'s discipline.
    #[error("cache policy without a cache backend: {reason}")]
    CachePolicyWithoutCacheBackend {
        /// Every node carrying a `CachePolicy`, sorted.
        nodes: Vec<NodeId>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a node carrying a resolved `Aegis.cache`
    /// policy whose `NodeSpec::Paladin` `output_field` is declared
    /// `CacheMarker::Deny` in the `BattlefieldSchema` (D-29, plan 25-13,
    /// FT-FR-20). A Paladin node's write set is exactly its `output_field`,
    /// so the denial is checked here, before any node executes; a
    /// `Function` node's write set is not statically knowable, so ITS
    /// denial is enforced at store time instead (a delta touching a `Deny`
    /// field is never written to the cache). Carries EVERY offending
    /// (node, field) pairing, pre-formatted, mirroring
    /// [`EngineError::RouteErrorFieldUndeclared`]'s discipline.
    #[error("cache policy on a denied field: {reason}")]
    CachePolicyOnDeniedField {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a node whose resolved `Aegis.cache`
    /// policy uses `CacheKeySpec::Fields` naming a field the
    /// `BattlefieldSchema` does not declare (D-28, plan 25-13). An
    /// undeclared field would silently contribute an "absent" marker to
    /// every key, so a typo would narrow the key to less than the author
    /// intended and serve stale hits -- rejected before any node executes
    /// instead. Carries EVERY offending (node, field) pairing,
    /// pre-formatted.
    #[error("cache key field undeclared: {reason}")]
    CacheKeyFieldUndeclared {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate_structured_executor_backend` found one or more
    /// nodes carrying an `output_schema` while the `WarEngine` has no
    /// structured executor wired via [`WarEngine::with_structured_executor`]
    /// (D-29, RT-FR-19, plan 26-18). Fail-closed, mirroring
    /// [`EngineError::CachePolicyWithoutCacheBackend`]'s discipline: a graph
    /// author who declared a schema and silently got a plain string written
    /// instead would have no signal.
    #[error("output_schema without a structured executor: {reason}")]
    StructuredExecutorMissing {
        /// Every node declaring an `output_schema`, sorted.
        nodes: Vec<NodeId>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a `SchemaRef::Registered(name)` naming a
    /// schema not present in the engine's schema registry
    /// ([`WarEngine::with_output_schema`], D-29, RT-FR-19, plan 26-18).
    /// Carries EVERY offending node/name pairing, pre-formatted.
    #[error("unregistered output schema: {reason}")]
    UnregisteredOutputSchema {
        /// Every offending node/name pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a node with BOTH `output_schema` and a
    /// non-`PlainOutput` `directive_parser` (D-29, RT-FR-19, plan 26-18) --
    /// combining them is a Deferred Idea, never a silent precedence rule.
    /// Carries EVERY offending node, pre-formatted.
    #[error("output_schema with a structured directive: {reason}")]
    OutputSchemaWithStructuredDirective {
        /// Every offending node, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
    },

    /// `WarGraph::validate` found a node whose `output_schema` writes to an
    /// `output_field` declared with a `DispatchRule` that cannot hold a
    /// JSON value (D-29, RT-FR-19, plan 26-18). Carries EVERY offending
    /// node/field pairing, pre-formatted.
    #[error("output_schema field not JSON-compatible: {reason}")]
    OutputSchemaFieldNotJson {
        /// Every offending node/field pairing, pre-formatted.
        offenders: Vec<String>,
        /// Explains the rule and names the offenders.
        reason: String,
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
    /// The bundle of every named-registration registry (`EdgeCondition::
    /// Custom` evaluators BUG-01/CF-01; `RetryPredicate::Custom` and
    /// `ErrorHandlerSpec::Custom` D-13, plan 25-03). Empty by default: a
    /// v0.9 configuration with no `Custom` names anywhere boots identically
    /// (D-26). Never referenced from `paladin-core` (X-01) -- handed to
    /// `WarGraph::validate` and `superstep::run` (its `edge_evaluators`
    /// field only, for now) at `start`/`resume` as an `&EngineRegistries`.
    registries: EngineRegistries,
    /// The `TraceSink` this engine forwards to, if any (ENG-FR-21, D-03).
    /// `None` by default. A fresh, thread-scoped `TraceDispatcher` is
    /// constructed from this (and `trace_capacity`) at the top of every
    /// entry point (`start`/`resume`/`resume_with`/`replay`/`fork`) rather
    /// than held as one engine-lifetime dispatcher: `TraceDispatcher` stamps
    /// `seq` per its own `thread_id` (D-03), and this engine's own entry
    /// points each take a `thread: ThreadId` argument that can differ call
    /// to call (most concretely in this crate's own unit tests, which reuse
    /// one `WarEngine` across many threads) -- the production shape (the
    /// Phase 27 worker builds one engine per run) makes this equivalent to
    /// "one dispatcher per run" in the served path either way.
    trace_sink: Option<Arc<dyn TraceSink>>,
    /// The queue capacity every per-call `TraceDispatcher` above is
    /// constructed with. Defaults to `TraceDispatcher`'s own default.
    trace_capacity: usize,
    /// The ordered `NodeInterceptor` chain (ENG-FR-22). Empty by default: an
    /// empty chain is proven (in `engine::hooks`'s own tests) to change
    /// nothing about a run's node executions or final state.
    interceptors: Vec<Arc<dyn NodeInterceptor>>,
    /// The optional cancellation signal observed at superstep boundaries
    /// (ENG-FR-23). `None` behaves identically to a token that is never
    /// cancelled.
    cancellation_token: Option<CancellationToken>,
    /// The optional durable, possibly cross-instance cancellation probe
    /// consulted at every superstep boundary BESIDE --  never instead of --
    /// `cancellation_token` above (D-14, PLAT-FR-04), wired via
    /// [`WarEngine::with_cancellation_probe`]. `None` behaves identically to
    /// a probe that never reports cancellation
    /// ([`paladin_ports::output::cancellation_probe::NeverCancelled`]).
    /// Forwarded wholesale into every `NodeSpec::Battalion` child engine
    /// run, like every other engine resource, so a cancelled parent
    /// thread's children observe it too.
    cancellation_probe: Option<Arc<dyn CancellationProbe>>,
    /// The grace window a mid-superstep cancellation races the in-flight
    /// batch of spawned node tasks against (HITL-04, D-19, D-20). A runtime
    /// setting, never part of `EngineLimits` and never hashed into the
    /// graph fingerprint -- see [`WarEngine::with_shutdown_grace`].
    /// Defaults to 30 seconds.
    shutdown_grace: std::time::Duration,
    /// The per-node result cache backend (Doc 04 FT-FR-18, D-29), wired via
    /// [`WarEngine::with_node_cache`]. `None` by default: a graph with no
    /// `CachePolicy` anywhere runs identically with or without one, and a
    /// graph WITH a `CachePolicy` fails validation
    /// (`EngineError::CachePolicyWithoutCacheBackend`) rather than silently
    /// running uncached. Forwarded wholesale into every
    /// `NodeSpec::Battalion` child run, like every other engine resource.
    node_cache: Option<Arc<dyn NodeCachePort>>,
    /// This engine's confined Vault handle (Doc 05 RT-04, D-21), wired via
    /// [`WarEngine::with_vault`]. `None` by default: an engine with no
    /// Vault store gives every node's `NodeContext::vault()` `None`, never
    /// a handle silently granted the root namespace. When `Some`, the SAME
    /// grant is given to every node of every run on this engine --
    /// including a `NodeSpec::Battalion` child run, like every other engine
    /// resource -- cross-thread memory is the point; a per-thread
    /// sub-namespace is the host's own choice via the `base` passed to
    /// `with_vault`, never something this engine derives on its own.
    vault: Option<ConfinedVault>,
    /// This engine's structured-output executor (RT-05, RT-FR-19, D-29;
    /// plan 26-18), wired via [`WarEngine::with_structured_executor`].
    /// `None` by default: a graph with no `output_schema` anywhere runs
    /// identically with or without one, and a graph WITH an `output_schema`
    /// fails validation (`EngineError::StructuredExecutorMissing`) rather
    /// than silently falling back to writing a plain string -- the same
    /// fail-closed discipline `node_cache` above already establishes.
    /// Forwarded wholesale into every `NodeSpec::Battalion` child run, like
    /// every other engine resource.
    structured_executor: Option<Arc<dyn StructuredExecutorPort>>,
    /// The most recently constructed per-run `TraceDispatcher` (D-03),
    /// populated by every `start`/`resume*` entry point's own dispatcher
    /// construction (never rebuilt from this cell -- each entry point
    /// always builds its OWN fresh, thread-scoped dispatcher exactly as
    /// D-03 requires, then also stores a clone here) and, lazily, by
    /// [`WarEngine::trace_emitter`] itself when called before this engine
    /// has ever run. See `trace_emitter`'s own doc comment for the
    /// before-any-run / after-a-run distinction this cell exists for.
    /// `Mutex`, not `RwLock`: every access is a quick swap/clone, never
    /// held across an `.await`.
    trace_dispatcher_cell: std::sync::Mutex<Option<Arc<TraceDispatcher>>>,
    /// A `TraceDispatcher` pre-bound to a specific `(thread, run_id)` pair
    /// via [`WarEngine::with_bound_trace`] (28-06, D-03), consumed by the
    /// next `start`/`resume*` call whose own `thread` argument matches.
    /// Closes the gap 28-03 documented on [`WarEngine::trace_emitter`]:
    /// without this, calling `trace_emitter()` before `start()` hands a
    /// caller a placeholder dispatcher that `start()` then orphans by
    /// building its own fresh one. `None` by default -- every entry point
    /// builds its own fresh dispatcher exactly as before this field
    /// existed. `Mutex`, not `RwLock`: a quick take-or-leave, never held
    /// across an `.await`.
    bound_trace: std::sync::Mutex<Option<(ThreadId, Arc<TraceDispatcher>)>>,
}

// --- CF-FR-16, D-21: `+ 'static` is required here (not on the struct
// declaration above) because `start`/`resume_with_options` forward
// `Arc<W>` into `superstep::run`, which in turn may capture it inside a
// `tokio::spawn`'d task for a `NodeSpec::Battalion` node's child run --
// every real `WaypointPort` implementor (`InMemoryWaypointStore`,
// `sqlite`/`postgres` backends, `RecordingWaypointStore`) already
// satisfies this trivially, since none carries a borrowed lifetime.
impl<W: WaypointPort + 'static> WarEngine<W> {
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
            registries: EngineRegistries::new(),
            trace_sink: None,
            trace_capacity: crate::engine::hooks::DEFAULT_CAPACITY,
            interceptors: Vec::new(),
            cancellation_token: None,
            cancellation_probe: None,
            shutdown_grace: std::time::Duration::from_secs(30),
            node_cache: None,
            vault: None,
            structured_executor: None,
            trace_dispatcher_cell: std::sync::Mutex::new(None),
            bound_trace: std::sync::Mutex::new(None),
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
        self.registries.edge_evaluators.register(name, evaluator);
        self
    }

    /// Register a named evaluator for `RetryPredicate::Custom(name)`
    /// policies (D-13, plan 25-03), shaped like [`WarEngine::with_edge_evaluator`]:
    /// no reserved-name failure mode, infallible. An unregistered `Custom`
    /// name still fails [`WarGraph::validate`] (and therefore
    /// [`WarEngine::start`]/[`WarEngine::resume`]) before any node executes;
    /// it is never silently treated as "do not retry" at runtime.
    pub fn with_retry_predicate(
        mut self,
        name: impl Into<String>,
        evaluator: Arc<dyn RetryPredicateEvaluator>,
    ) -> Self {
        self.registries.retry_predicates.register(name, evaluator);
        self
    }

    /// Register a named handler for `ErrorHandlerSpec::Custom(name)` policies
    /// (D-13, plan 25-03), shaped like [`WarEngine::with_edge_evaluator`]: no
    /// reserved-name failure mode, infallible. An unregistered `Custom` name
    /// still fails [`WarGraph::validate`] (and therefore
    /// [`WarEngine::start`]/[`WarEngine::resume`]) before any node executes.
    /// Dispatching a resolved handler at run time is plan 25-10/11's job --
    /// this plan owns registration and validation only.
    pub fn with_error_handler(
        mut self,
        name: impl Into<String>,
        handler: Arc<dyn ErrorHandler>,
    ) -> Self {
        self.registries.error_handlers.register(name, handler);
        self
    }

    /// Attach `sink` as this engine's `TraceSink` (ENG-FR-21). Replaces any
    /// previously configured sink; events are forwarded fire-and-forget over
    /// a bounded, drop-oldest queue -- see `engine::hooks::TraceDispatcher`.
    pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
        self.trace_sink = Some(sink);
        self
    }

    /// Store `trace` as this engine's most-recently-constructed dispatcher
    /// (D-03) -- called by every `start`/`resume*` entry point right after
    /// it builds its own fresh, thread-scoped dispatcher (never itself
    /// constructing or replacing the dispatcher a run actually uses), so a
    /// [`WarEngine::trace_emitter`] call can hand a caller a handle onto
    /// the SAME `seq` counter.
    fn remember_trace_dispatcher(&self, trace: &Arc<TraceDispatcher>) {
        *self
            .trace_dispatcher_cell
            .lock()
            .expect("trace dispatcher mutex poisoned") = Some(Arc::clone(trace));
    }

    /// A cheap, clonable [`TraceEmitter`] handle bound to this engine's own
    /// `TraceDispatcher` (D-03): the SAME `seq` counter `start`/`resume*`'s
    /// own `SuperstepStarted`/`NodeFinished`/... records stamp through, so
    /// a below-engine producer composed with this handle (28-06:
    /// `FallbackLlmAdapter::with_trace_emitter`, the middleware chain, the
    /// execution service) lands in that run's own causal `seq` order
    /// rather than starting a competing sequence of its own.
    ///
    /// Reflects the dispatcher of the MOST RECENT `start`/`resume*` call on
    /// this engine (never rebuilt by a later call -- each entry point
    /// always constructs its own fresh dispatcher for `seq` to restart at 1
    /// per run, D-03, then also records it here). Before this engine has
    /// run at all, there is nothing yet to bind to; this lazily constructs
    /// one (stamped with a placeholder `ThreadId`, forwarding this engine's
    /// own `trace_sink`/`trace_capacity`) so an early caller still gets a
    /// working handle rather than `None` -- that SAME instance is then
    /// orphaned (not reused) the moment the next real `start`/`resume*`
    /// call replaces this cell with its own dispatcher, a known limitation
    /// left for 28-06's own wiring work to resolve.
    pub fn trace_emitter(&self) -> Arc<dyn TraceEmitter> {
        let mut cell = self
            .trace_dispatcher_cell
            .lock()
            .expect("trace dispatcher mutex poisoned");
        let dispatcher = cell.get_or_insert_with(|| {
            Arc::new(TraceDispatcher::with_capacity(
                // A hardcoded, whitespace-free, well-under-the-length-limit
                // literal -- `ThreadId::new`'s own validation can never
                // reject it (identical in kind to the many `.unwrap()`
                // call sites already in this crate's own test suite
                // constructing a `ThreadId` from a literal).
                ThreadId::new("pending").expect("static placeholder id is always valid"),
                None,
                self.trace_sink.clone(),
                self.trace_capacity,
            ))
        });
        Arc::clone(dispatcher) as Arc<dyn TraceEmitter>
    }

    /// Override the queue capacity every per-call `TraceDispatcher` is
    /// constructed with (28-06: the facade composition root passes
    /// `TraceConfig::channel_capacity` through here). Defaults to
    /// `TraceDispatcher`'s own default when never called.
    pub fn with_trace_capacity(mut self, capacity: usize) -> Self {
        self.trace_capacity = capacity;
        self
    }

    /// Pre-bind a `TraceDispatcher` for `thread`/`run_id` before this
    /// engine's `start`/`resume*` is ever called (28-06, D-03): a
    /// [`WarEngine::trace_emitter`] call made immediately AFTER this builder
    /// step returns the SAME dispatcher instance the next matching
    /// `start`/`resume*` call then uses for its own emissions, closing the
    /// "orphaned placeholder" gap `trace_emitter`'s own doc comment
    /// describes (28-03's documented known gap).
    ///
    /// The worker composition root knows `run.thread_id`/`run_id` before
    /// constructing the per-run engine: call this right after
    /// `with_trace_sink`, then `trace_emitter()` to get the handle to hand
    /// to the fallback adapter, the middleware chain and the execution
    /// service, THEN call `start`/`resume*` with the SAME `thread`.
    ///
    /// Consumed by the next matching entry-point call only -- calling this
    /// again before that call replaces the pending binding, and a
    /// `start`/`resume*` call whose `thread` does NOT match the pending
    /// binding builds its own fresh dispatcher instead (D-03's per-run
    /// `seq`-restart contract is never compromised either way).
    pub fn with_bound_trace(self, thread: ThreadId, run_id: Option<RunId>) -> Self {
        let dispatcher = Arc::new(TraceDispatcher::with_capacity(
            thread.clone(),
            run_id,
            self.trace_sink.clone(),
            self.trace_capacity,
        ));
        self.with_bound_trace_dispatcher(thread, dispatcher)
    }

    /// As [`WarEngine::with_bound_trace`], but for a caller that must build
    /// the `TraceDispatcher` itself BEFORE this engine exists (28-06): the
    /// worker composition root needs the SAME `Arc<TraceDispatcher>`
    /// instance (coerced to `Arc<dyn TraceEmitter>`) to construct the
    /// per-run `FallbackLlmAdapter`/middleware chain/execution service --
    /// each of which is itself a dependency of this engine's own
    /// `paladin_port` constructor argument, so the dispatcher must exist
    /// before `WarEngine::new` is even called. Binds `dispatcher` under
    /// `thread` exactly as `with_bound_trace` does; consumed the same way
    /// by the next matching `start`/`resume*` call.
    pub fn with_bound_trace_dispatcher(
        self,
        thread: ThreadId,
        dispatcher: Arc<TraceDispatcher>,
    ) -> Self {
        self.remember_trace_dispatcher(&dispatcher);
        *self.bound_trace.lock().expect("bound trace mutex poisoned") = Some((thread, dispatcher));
        self
    }

    /// Returns the dispatcher pre-bound via [`WarEngine::with_bound_trace`]
    /// when its `thread` matches `thread`, consuming the binding; otherwise
    /// builds a fresh dispatcher exactly as every entry point did before
    /// `with_bound_trace` existed (D-03: `seq` restarts at 1 per run either
    /// way -- this only decides which `TraceDispatcher` INSTANCE serves that
    /// contract, not whether it holds).
    fn take_or_build_trace_dispatcher(&self, thread: &ThreadId) -> Arc<TraceDispatcher> {
        if let Ok(mut bound) = self.bound_trace.lock()
            && let Some((bound_thread, dispatcher)) = bound.take()
            && &bound_thread == thread
        {
            return dispatcher;
        }
        // Either nothing was bound, or it was bound for a DIFFERENT thread
        // than the one starting now -- a mismatched binding's dispatcher is
        // simply dropped (its own `RunStarted`/`RunFinished` bracket was
        // never emitted, so nothing observable is lost) and a fresh
        // dispatcher is built for THIS thread, unchanged from before
        // `with_bound_trace` existed.
        Arc::new(TraceDispatcher::with_capacity(
            thread.clone(),
            None,
            self.trace_sink.clone(),
            self.trace_capacity,
        ))
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

    /// Attach a [`CancellationProbe`] this engine consults at every
    /// superstep boundary, BESIDE -- never instead of -- any
    /// `CancellationToken` configured via
    /// [`WarEngine::with_cancellation_token`] (D-14, PLAT-FR-04): either one
    /// answering "cancelled" halts the run identically. A probe that never
    /// reports cancellation produces behavior identical to no probe
    /// configured at all.
    pub fn with_cancellation_probe(mut self, probe: Arc<dyn CancellationProbe>) -> Self {
        self.cancellation_probe = Some(probe);
        self
    }

    /// Set the grace window a mid-superstep cancellation races the
    /// in-flight batch of spawned node tasks against (HITL-04, D-19, D-20):
    /// once the cancellation token fires while nodes are executing, the
    /// engine keeps awaiting them until this deadline, then aborts every
    /// still-outstanding task and records it `Skipped { reason: "shutdown"
    /// }`, re-listed in the Halted Waypoint's vanguard so `resume` re-runs
    /// it exactly once. Defaults to 30 seconds when never called.
    /// `Duration::ZERO` aborts every in-flight node the moment cancellation
    /// is observed.
    ///
    /// A runtime setting only: never part of [`EngineLimits`] and never
    /// hashed into [`WarGraph::fingerprint`] (D-20) -- changing it does not
    /// invalidate a suspended thread's [`crate::engine::graph::WarGraph::fingerprint`]
    /// comparison on resume.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_battalion::engine::WarEngine;
    /// use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
    /// use paladin_core::platform::container::directive::Directive;
    /// use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
    /// use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use paladin_core::platform::container::paladin_error::PaladinError;
    /// use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use async_trait::async_trait;
    ///
    /// struct NoopPort;
    /// #[async_trait]
    /// impl PaladinPort for NoopPort {
    ///     async fn execute(&self, _p: &Paladin, _i: &str) -> Result<PaladinResult, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     async fn execute_stream(&self, _p: &Paladin, _i: &str) -> Result<PaladinStream, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     fn validate(&self, _p: &Paladin) -> Result<(), PaladinError> { Ok(()) }
    /// }
    ///
    /// let engine = WarEngine::new(Arc::new(NoopPort), Arc::new(InMemoryWaypointStore::new()))
    ///     .with_shutdown_grace(Duration::from_secs(5));
    /// ```
    pub fn with_shutdown_grace(mut self, grace: std::time::Duration) -> Self {
        self.shutdown_grace = grace;
        self
    }

    /// Wire `cache` as this engine's per-node result cache backend (Doc 04
    /// FT-FR-18, D-29; plan 25-13). Replaces any previously configured
    /// backend.
    ///
    /// With a backend wired, every node carrying a resolved
    /// `Aegis.cache` [`CachePolicy`](paladin_core::platform::container::aegis::CachePolicy)
    /// is looked up BEFORE its first attempt under a key composed by
    /// [`cache_key`] (graph fingerprint, node id, resolved input, Paladin
    /// configuration fingerprint) -- a hit merges the stored delta with no
    /// execution and records `cache_hit: true`; a miss executes the node
    /// and, on a successful attempt whose `Directive` routes via
    /// `NextStep::Edges`, stores its delta under the policy's TTL. The
    /// cache is best-effort by construction: a `get` failure is a miss and
    /// a `put` failure is logged, never a run failure.
    ///
    /// Without a backend, a `CachePolicy` anywhere in the graph (including
    /// inside a `NodeSpec::Battalion` child) fails
    /// [`WarEngine::start`]/`resume` with
    /// [`EngineError::CachePolicyWithoutCacheBackend`] before any node runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use paladin_battalion::engine::WarEngine;
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use paladin_core::platform::container::paladin_error::PaladinError;
    /// use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
    /// use paladin_storage::node_cache::in_memory::InMemoryNodeCache;
    /// use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    ///
    /// struct NoopPort;
    /// #[async_trait]
    /// impl PaladinPort for NoopPort {
    ///     async fn execute(&self, _p: &Paladin, _i: &str) -> Result<PaladinResult, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     async fn execute_stream(&self, _p: &Paladin, _i: &str) -> Result<PaladinStream, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     fn validate(&self, _p: &Paladin) -> Result<(), PaladinError> { Ok(()) }
    /// }
    ///
    /// let engine = WarEngine::new(Arc::new(NoopPort), Arc::new(InMemoryWaypointStore::new()))
    ///     .with_node_cache(Arc::new(InMemoryNodeCache::new()));
    /// ```
    pub fn with_node_cache(mut self, cache: Arc<dyn NodeCachePort>) -> Self {
        self.node_cache = Some(cache);
        self
    }

    /// Wire `executor` as this engine's structured-output executor (RT-05,
    /// RT-FR-19, D-29; plan 26-18), the shape `WarEngine::with_node_cache`
    /// establishes: a plain `Option<Arc<dyn _>>` field, checked separately
    /// from [`WarGraph::validate`] via
    /// [`WarGraph::validate_structured_executor_backend`] since only the
    /// engine knows whether one is configured.
    ///
    /// Without a backend, an `output_schema` anywhere in the graph
    /// (including inside a `NodeSpec::Battalion` child) fails
    /// [`WarEngine::start`]/`resume` with
    /// [`EngineError::StructuredExecutorMissing`] before any node runs.
    pub fn with_structured_executor(mut self, executor: Arc<dyn StructuredExecutorPort>) -> Self {
        self.structured_executor = Some(executor);
        self
    }

    /// Register `schema` under `name` for `SchemaRef::Registered(name)`
    /// resolution (D-29, RT-FR-19, plan 26-18), shaped like
    /// [`WarEngine::with_retry_predicate`]: no reserved-name failure mode,
    /// infallible, replacing any prior registration under the same name. An
    /// unregistered `Registered` name still fails [`WarGraph::validate`]
    /// (and therefore [`WarEngine::start`]/[`WarEngine::resume`]) before any
    /// node executes -- never a runtime surprise.
    pub fn with_output_schema(
        mut self,
        name: impl Into<String>,
        schema: Arc<dyn StructuredSchema>,
    ) -> Self {
        self.registries.output_schemas.insert(name.into(), schema);
        self
    }

    /// Wire `vault` as this engine's Vault store, granting `base` to every
    /// node of every run on this engine (Doc 05 RT-04, D-21). Replaces any
    /// previously configured grant.
    ///
    /// Cross-thread memory is the point of this method: EVERY node of every
    /// run on this engine receives the SAME `base` grant through
    /// `NodeContext::vault()`, and a `NodeSpec::Paladin` node receives the
    /// same grant through the defaulted `PaladinPort::execute_scoped`
    /// dispatch (`engine::superstep`'s Paladin arm). A per-thread
    /// sub-namespace is never derived automatically -- if a host wants one,
    /// it is the host's own choice to make `base` itself carry a
    /// per-deployment segment; this method promises exactly one grant,
    /// shared by the whole engine, never an implicit per-thread narrowing.
    ///
    /// Without a call to this method, `NodeContext::vault()` is `None` for
    /// every node on this engine -- never a handle silently granted the
    /// root namespace.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    /// use paladin_battalion::engine::WarEngine;
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use paladin_core::platform::container::paladin_error::PaladinError;
    /// use paladin_core::platform::container::vault::Namespace;
    /// use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
    /// use paladin_ports::output::vault_port::VaultPort;
    /// use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
    ///
    /// struct NoopPort;
    /// #[async_trait]
    /// impl PaladinPort for NoopPort {
    ///     async fn execute(&self, _p: &Paladin, _i: &str) -> Result<PaladinResult, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     async fn execute_stream(&self, _p: &Paladin, _i: &str) -> Result<PaladinStream, PaladinError> {
    ///         unreachable!()
    ///     }
    ///     fn validate(&self, _p: &Paladin) -> Result<(), PaladinError> { Ok(()) }
    /// }
    ///
    /// # fn build_vault() -> Arc<dyn VaultPort> { unimplemented!() }
    /// let base = Namespace::parse("app").unwrap();
    /// let engine = WarEngine::new(Arc::new(NoopPort), Arc::new(InMemoryWaypointStore::new()))
    ///     .with_vault(build_vault(), base);
    /// ```
    pub fn with_vault(mut self, vault: Arc<dyn VaultPort>, base: Namespace) -> Self {
        self.vault = Some(ConfinedVault::new(vault, base));
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
        graph.validate(registry, &self.registries)?;
        graph.validate_node_cache_backend(self.node_cache.is_some())?;
        graph.validate_structured_executor_backend(self.structured_executor.is_some())?;

        let battlefield = Battlefield::initialize(graph.schema().clone(), &initial)?;
        battlefield.validate_required()?;

        // D-03: a fresh, thread-scoped dispatcher stamps `seq`/`at` for
        // every trace record this call (and everything it calls) emits.
        // 28-06: reuses a `with_bound_trace`-pre-bound dispatcher for this
        // `thread` if one is pending, so a caller that already pulled
        // `trace_emitter()` before this call shares the SAME dispatcher
        // (D-03) -- otherwise builds a fresh one exactly as before.
        let trace = self.take_or_build_trace_dispatcher(&thread);
        self.remember_trace_dispatcher(&trace);
        trace.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: graph.fingerprint().to_string(),
        });
        // D-02: this call's own wall-clock start, for `RunFinished
        // .duration_ms` below.
        let run_started_at = tokio::time::Instant::now();
        let outcome = superstep::run(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.registries,
            graph,
            thread.clone(),
            battlefield,
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            &self.paladin_port,
            &trace,
            &self.interceptors,
            &self.cancellation_token,
            &self.cancellation_probe,
            Some(Arc::clone(&self.waypoint_port)),
            self.shutdown_grace,
            self.node_cache.clone(),
            self.vault.clone(),
            self.structured_executor.clone(),
        )
        .await;
        // D-02, D-04: `status` from the `RunOutcome`/`Err` this call itself
        // matches on; `total_supersteps`/`total_tokens` from this run's own
        // dispatcher, tallied synchronously as `SuperstepStarted`/
        // `NodeFinished` records were stamped (never racing the async
        // consumer, `TraceDispatcher::superstep_count`/`token_total`'s own
        // doc comments); `trace_dropped_total` is stamped by
        // `TraceDispatcher::emit` itself at enqueue time (D-07).
        trace.emit(TraceEvent::RunFinished {
            status: run_finish_status(&outcome),
            total_supersteps: trace.superstep_count(),
            total_tokens: trace.token_total(),
            duration_ms: run_started_at.elapsed().as_millis() as u64,
            trace_dropped_total: 0,
        });
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
    ///
    /// Mid-muster resume (CF-FR-12, D-14): when the loaded Waypoint carries
    /// `muster_progress: Some(progress)`, this call re-enters that SAME
    /// superstep (`latest.superstep`, never `+ 1`) dispatching only
    /// `progress.unfinished_tasks()` -- the tasks whose `task_key` is absent
    /// from `progress.completed` -- alongside the loaded Waypoint's ordinary
    /// `vanguard`. Every restored task's `worker` is checked against the
    /// (possibly new) graph first: an absent or no-longer-worker-template
    /// `worker` fails with `EngineError::MusterProgressWorkerMissing`,
    /// mirroring `VanguardNodeMissing`'s "never silently skip expected
    /// work" rule. The superstep loop then merges every task's delta --
    /// restored plus newly produced -- in `task_key` order exactly once, so
    /// the resumed run reaches the same final Battlefield the uninterrupted
    /// run would have.
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

        // D-03: a fresh, thread-scoped dispatcher for this call and
        // everything it calls (including the early-return branch below).
        // 28-06: reuses a `with_bound_trace`-pre-bound dispatcher for this
        // `thread` if pending (see `start`'s own comment above).
        let trace = self.take_or_build_trace_dispatcher(&thread);
        self.remember_trace_dispatcher(&trace);
        // D-02: this call's own wall-clock start, for both `RunFinished`
        // sites below (the already-`Completed` early return and the real
        // resumed run) -- one instant, captured once, covers either path.
        let run_started_at = tokio::time::Instant::now();

        if matches!(latest.status, WaypointStatus::Completed) {
            trace.emit(TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: expected.to_string(),
            });
            // D-02, D-04: nothing executed on this call (the thread was
            // already `Completed`), so `total_supersteps`/`total_tokens`
            // are genuinely `0` here -- this dispatcher's own counters
            // agree, since no `SuperstepStarted`/`NodeFinished` was ever
            // stamped through it.
            trace.emit(TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: trace.superstep_count(),
                total_tokens: trace.token_total(),
                duration_ms: run_started_at.elapsed().as_millis() as u64,
                trace_dropped_total: 0,
            });
            return Ok(RunOutcome::Completed {
                final_state: latest.battlefield,
                waypoint: latest.waypoint_id,
            });
        }

        // --- HITL-01, D-11, RESEARCH.md Pitfall 2: an explicit
        // `AwaitingInput` arm, BEFORE the generic vanguard-restore
        // fallthrough below. `AwaitingInput` did not exist as a real,
        // reachable status before this phase (`ParleyNotSupported`
        // prevented it from ever being written) -- so this fallthrough was
        // never wrong until now. Only `WarEngine::resume_with` may advance
        // a suspended thread; a plain `resume`/`resume_with_options` fails
        // closed here, writing no Waypoint, rather than silently re-running
        // the parleying node(s) as ordinary vanguard entries.
        if let WaypointStatus::AwaitingInput { parleys, .. } = &latest.status {
            return Err(EngineError::ThreadAwaitingInput {
                thread: thread.clone(),
                parleys: parleys.clone(),
            });
        }

        // --- HITL-02, D-12: a thread whose latest Waypoint is `Failed`
        // (e.g. a `FailRun` parley expiry) is refused by a plain
        // `resume`/`resume_with_options`, mirroring the `AwaitingInput`
        // guard just above -- a `Failed` Waypoint records a terminal
        // outcome, not "more work pending," so the generic vanguard-
        // restore fallthrough below would otherwise silently attempt to
        // continue a run the engine itself already declared over. Such a
        // thread is advanced only by `replay`/`fork` from an earlier
        // Waypoint (a later plan), never by `resume`/`resume_with` again.
        if let WaypointStatus::Failed {
            error, failed_node, ..
        } = &latest.status
        {
            return Err(EngineError::ThreadAlreadyFailed {
                thread: thread.clone(),
                error: error.clone(),
                failed_node: failed_node.clone(),
            });
        }

        for node in &latest.vanguard {
            if graph.node(node).is_none() {
                return Err(EngineError::VanguardNodeMissing { node: node.clone() });
            }
        }

        // --- CF-FR-12, D-14: a mid-muster progress Waypoint additionally
        // names every unfinished task's `worker` -- checked against the
        // (possibly new) graph before this call decides how to re-enter the
        // superstep loop, mirroring the ordinary-vanguard check just above.
        if let Some(progress) = &latest.muster_progress {
            for task in &progress.tasks {
                match graph.node(&task.worker) {
                    Some(_) if graph.is_worker_template(&task.worker) => {}
                    _ => {
                        return Err(EngineError::MusterProgressWorkerMissing {
                            node: progress.node.clone(),
                            worker: task.worker.clone(),
                        });
                    }
                }
            }
        }

        let registry = self.dispatch_registry.resolver();
        graph.validate(registry, &self.registries)?;
        graph.validate_node_cache_backend(self.node_cache.is_some())?;
        graph.validate_structured_executor_backend(self.structured_executor.is_some())?;

        trace.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: expected.to_string(),
        });
        // --- CF-FR-12, D-14: a mid-muster progress Waypoint re-enters the
        // SAME superstep it was written at (never `+ 1`, unlike an ordinary
        // superstep-complete Waypoint) -- the muster's own dispatch
        // superstep is not yet finished, so continuing it is "resuming
        // superstep N", not "starting superstep N+1".
        let resume_superstep = if latest.muster_progress.is_some() {
            latest.superstep
        } else {
            latest.superstep + 1
        };
        let outcome = superstep::run(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.registries,
            graph,
            thread.clone(),
            latest.battlefield,
            latest.vanguard,
            latest.visit_counts,
            Some(latest.frontier),
            latest.muster_progress,
            Some(latest.waypoint_id),
            resume_superstep,
            &self.paladin_port,
            &trace,
            &self.interceptors,
            &self.cancellation_token,
            &self.cancellation_probe,
            Some(Arc::clone(&self.waypoint_port)),
            self.shutdown_grace,
            self.node_cache.clone(),
            self.vault.clone(),
            self.structured_executor.clone(),
        )
        .await;
        trace.emit(TraceEvent::RunFinished {
            status: run_finish_status(&outcome),
            total_supersteps: trace.superstep_count(),
            total_tokens: trace.token_total(),
            duration_ms: run_started_at.elapsed().as_millis() as u64,
            trace_dropped_total: 0,
        });
        outcome
    }

    /// Resume `thread` from an `AwaitingInput` Waypoint, delivering
    /// `responses` to the paused node(s)' continuation (HITL-02, D-08).
    ///
    /// Loads `latest(thread)` (absent -> `ThreadNotFound`), compares
    /// `graph_fingerprint` against `graph.fingerprint()` (mismatch ->
    /// `GraphMismatch`, ENG-FR-14, mirroring `resume`), and requires the
    /// loaded status to be `AwaitingInput` (else
    /// `EngineError::ThreadNotAwaitingInput { thread, status }`) -- only
    /// `resume_with` may advance a suspended thread (D-11).
    ///
    /// Validation is TOTAL before any state change (D-10): every
    /// OUTSTANDING (not-yet-answered) parley is first checked for expiry,
    /// evaluated lazily against `Utc::now()` (D-12, D-13) -- a `FailRun`
    /// parley past its `expires_at` fails the WHOLE call with a persisted
    /// `Failed` Waypoint and `Err(ParleyExpired)` before any submitted
    /// response is even inspected; a `ResumeWithDefault` parley
    /// substitutes its pre-validated default (`responded_by: None`,
    /// `defaulted: true`), overriding any late submission for the same
    /// `parley_id` (the clock alone decides once a request has expired).
    /// Every submitted (or defaulted) response is then checked against its
    /// own request: `UnknownParleyId` if its `parley_id` is not among this
    /// thread's own outstanding `parleys` (T-24-01, never a global
    /// lookup), `ParleyAlreadyAnswered` if that `parley_id` already has an
    /// accepted response (from the thread's prior history OR an earlier
    /// response in this SAME call -- two responses answering the same
    /// parley in one submission are both rejected), `ResponseShapeInvalid`
    /// if the value fails its `ParleyKind`'s shape rule. Any error here
    /// leaves the thread suspended with no Waypoint written (except the
    /// `Failed` Waypoint a `FailRun` expiry itself persists).
    ///
    /// A valid but PARTIAL submission (D-11) persists a new `AwaitingInput`
    /// Waypoint at the SAME superstep with `responses` extended, and
    /// returns `RunOutcome::AwaitingInput` naming only the still-remaining
    /// parleys -- the thread stays suspended, queryable from a cold store.
    /// Once every outstanding parley has an accepted response, this call
    /// seeds superstep `latest.superstep + 1` with `vanguard` = every
    /// parleying node named on the loaded `AwaitingInput` status (D-08):
    /// exactly the persisted Waypoint's own `vanguard`, discarding
    /// nothing. Each dispatched node's `NodeContext.parley_response` is
    /// populated with its matching response (looked up by the node's own
    /// `NodeId`, via the request that named it) -- from there this is an
    /// ordinary superstep: deltas merge, edges resolve, one Waypoint per
    /// superstep (ENG-FR-11 holds with no clarification). Responses are
    /// durably consumed only when this first post-resume Waypoint
    /// persists (D-08): if the process dies between validation and that
    /// write, the `AwaitingInput` Waypoint just read is still `latest`,
    /// and re-submitting the identical responses is safe.
    pub async fn resume_with(
        &self,
        graph: &WarGraph,
        thread: ThreadId,
        responses: Vec<ParleyResponse>,
    ) -> Result<RunOutcome, EngineError> {
        let latest = self
            .waypoint_port
            .latest(&thread)
            .await
            .map_err(|source| EngineError::WaypointRead { source })?
            .ok_or_else(|| EngineError::ThreadNotFound(thread.clone()))?;

        let expected = graph.fingerprint();
        if latest.graph_fingerprint != expected {
            return Err(EngineError::GraphMismatch {
                expected,
                got: latest.graph_fingerprint,
            });
        }

        let (parleys, existing_responses) = match &latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                (parleys.clone(), responses.clone())
            }
            other => {
                return Err(EngineError::ThreadNotAwaitingInput {
                    thread: thread.clone(),
                    status: format!("{other:?}"),
                });
            }
        };

        let registry = self.dispatch_registry.resolver();
        graph.validate(registry, &self.registries)?;
        graph.validate_node_cache_backend(self.node_cache.is_some())?;
        graph.validate_structured_executor_backend(self.structured_executor.is_some())?;

        // D-03: a fresh, thread-scoped dispatcher for this call and every
        // early-return branch inside it.
        // 28-06: reuses a `with_bound_trace`-pre-bound dispatcher for this
        // `thread` if pending (see `start`'s own comment above).
        let trace = self.take_or_build_trace_dispatcher(&thread);
        self.remember_trace_dispatcher(&trace);
        // D-02: this call's own wall-clock start, for the `RunFinished`
        // site below.
        let run_started_at = tokio::time::Instant::now();

        let now = Utc::now();
        let already_answered: BTreeSet<ParleyId> =
            existing_responses.iter().map(|r| r.parley_id).collect();

        // --- HITL-02, D-12, D-13: lazy expiry, evaluated over every
        // OUTSTANDING (not-yet-answered) request -- independent of
        // whether THIS call's `responses` even names it. Once the clock
        // alone says a request expired, no late submission for it
        // matters: `FailRun` fails the whole call before any submitted
        // response is even inspected (extending D-10's total-validation
        // discipline to expiry); `ResumeWithDefault` substitutes its own
        // pre-validated default (T-24-06), unconditionally overriding
        // whatever this call may have submitted for the same `parley_id`.
        let mut defaulted: Vec<ParleyResponse> = Vec::new();
        for request in &parleys {
            if already_answered.contains(&request.parley_id) {
                continue;
            }
            let Some(expires_at) = request.expires_at else {
                continue;
            };
            if expires_at > now {
                continue;
            }
            match &request.on_expire {
                OnExpire::FailRun => {
                    let reason = format!(
                        "parley {} (node {}) expired at {expires_at} under on_expire: FailRun",
                        request.parley_id, request.node_id
                    );
                    let waypoint = superstep::build_waypoint(
                        &thread,
                        Some(latest.waypoint_id),
                        latest.superstep,
                        graph,
                        &latest.battlefield,
                        latest.vanguard.clone(),
                        Vec::new(),
                        WaypointStatus::Failed {
                            error: reason,
                            failed_node: request.node_id.clone(),
                            // A parley expiry is not an Aegis-governed node
                            // failure (D-08): no structured `NodeError`.
                            node_error: None,
                        },
                        latest.visit_counts.clone(),
                        latest.frontier.clone(),
                        None,
                        latest.checkpoint_ns.clone(),
                        // --- HITL-03, D-14: a FailRun expiry's own `Failed`
                        // Waypoint stays on the SAME branch `latest` was on
                        // -- propagated verbatim, never reset to mainline.
                        latest.fork_of,
                    );
                    superstep::persist_waypoint(
                        self.waypoint_port.as_ref(),
                        self.durability,
                        &waypoint,
                        &trace,
                    )
                    .await?;
                    return Err(EngineError::ParleyExpired {
                        parley_id: request.parley_id,
                        expires_at,
                    });
                }
                OnExpire::ResumeWithDefault(value) => {
                    defaulted.push(ParleyResponse {
                        parley_id: request.parley_id,
                        kind: request.kind.clone(),
                        prompt: request.prompt.clone(),
                        value: value.clone(),
                        responded_by: None,
                        responded_at: now,
                        defaulted: true,
                    });
                }
                // `OnExpire` is `#[non_exhaustive]`: a future policy this
                // engine does not yet recognise fails CLOSED here too,
                // mirroring `graph::validate_parley_value_for_kind`'s own
                // fail-closed catch-all -- never silently treated as
                // still open.
                _ => {
                    return Err(EngineError::ParleyExpired {
                        parley_id: request.parley_id,
                        expires_at,
                    });
                }
            }
        }

        // A default substitution always wins over a late submission for
        // the same `parley_id` (see the loop above's rationale).
        let mut effective_responses: Vec<ParleyResponse> = responses
            .into_iter()
            .filter(|r| !defaulted.iter().any(|d| d.parley_id == r.parley_id))
            .collect();
        effective_responses.extend(defaulted);

        // --- HITL-02, D-10: total validation -- every submitted (or
        // defaulted) response is checked against its own request BEFORE
        // any state changes. Two responses answering the SAME `parley_id`
        // within one call are BOTH rejected (the flagged "review
        // manually" edge probe's planner-resolved reading): the first is
        // accepted into `newly_answered`, so the second fails
        // `ParleyAlreadyAnswered`.
        let mut newly_answered: BTreeSet<ParleyId> = BTreeSet::new();
        for response in &effective_responses {
            let Some(request) = parleys.iter().find(|p| p.parley_id == response.parley_id) else {
                return Err(EngineError::UnknownParleyId {
                    parley_id: response.parley_id,
                });
            };
            if already_answered.contains(&response.parley_id)
                || !newly_answered.insert(response.parley_id)
            {
                return Err(EngineError::ParleyAlreadyAnswered {
                    parley_id: response.parley_id,
                });
            }
            if let Err(reason) = validate_response_shape(graph, request, &response.value) {
                return Err(EngineError::ResponseShapeInvalid {
                    parley_id: response.parley_id,
                    reason,
                });
            }
        }

        // --- Every response is now valid; nothing past this point can
        // fail on account of the CALLER's input, so it is safe to start
        // building persisted state.
        let mut all_responses = existing_responses;
        all_responses.extend(effective_responses);

        let remaining: Vec<ParleyRequest> = parleys
            .iter()
            .filter(|p| !all_responses.iter().any(|r| r.parley_id == p.parley_id))
            .cloned()
            .collect();

        if !remaining.is_empty() {
            // --- HITL-02, D-11: a valid but PARTIAL submission persists
            // a NEW `AwaitingInput` Waypoint at the SAME superstep
            // (mirrors D-14's mid-muster progress-Waypoint precedent):
            // `parleys` unchanged, `responses` extended, `vanguard`
            // unchanged -- the parleying nodes are still the parleying
            // nodes, since nothing has run yet.
            let waypoint = superstep::build_waypoint(
                &thread,
                Some(latest.waypoint_id),
                latest.superstep,
                graph,
                &latest.battlefield,
                latest.vanguard.clone(),
                Vec::new(),
                WaypointStatus::AwaitingInput {
                    parleys: parleys.clone(),
                    responses: all_responses,
                },
                latest.visit_counts.clone(),
                latest.frontier.clone(),
                None,
                latest.checkpoint_ns.clone(),
                // --- HITL-03, D-14: a partial-answer Waypoint stays on the
                // SAME branch `latest` was on -- propagated verbatim.
                latest.fork_of,
            );
            superstep::persist_waypoint(
                self.waypoint_port.as_ref(),
                self.durability,
                &waypoint,
                &trace,
            )
            .await?;
            return Ok(RunOutcome::AwaitingInput {
                parleys: remaining,
                waypoint: waypoint.waypoint_id,
            });
        }

        // --- D-08: `NodeContext.parley_response` is looked up by the
        // executing node's own `NodeId`, never by `parley_id` -- so
        // responses are re-keyed here, through the matching request, once.
        // HITL-01, D-07: `kind`/`prompt` are ALSO stamped onto the response
        // here, from the matching request, regardless of what an external
        // caller supplied for them when constructing this `ParleyResponse`
        // -- mirroring `ParleyRequest.node_id`'s own engine-stamped-
        // regardless contract (24-01). This is what lets the `parley.`
        // `InputMapping` namespace (`InputMapping::render`'s third
        // parameter) resolve `{parley.prompt}`/`{parley.kind}` from this
        // ONE type, with no separate `NodeContext`-only side channel
        // duplicating data already recorded on the request.
        let mut responses_by_node: BTreeMap<NodeId, ParleyResponse> = BTreeMap::new();
        for response in all_responses {
            if let Some(request) = parleys.iter().find(|p| p.parley_id == response.parley_id) {
                let mut response = response;
                response.kind = request.kind.clone();
                response.prompt = request.prompt.clone();
                responses_by_node.insert(request.node_id.clone(), response);
            }
        }

        trace.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: expected.to_string(),
        });
        // --- D-08: the persisted `AwaitingInput` Waypoint's OWN `vanguard`
        // is exactly the parleying nodes -- passed through unchanged as
        // this call's forced vanguard, never recomputed. `run_with_namespace`
        // is called directly (bypassing the public `run` wrapper) since
        // only THIS call site ever has a real parley-responses map to pass
        // -- `run`'s own signature stays unchanged by this plan.
        let outcome = superstep::run_with_namespace(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.registries,
            graph,
            thread.clone(),
            latest.battlefield,
            latest.vanguard,
            latest.visit_counts,
            Some(latest.frontier),
            None,
            Some(latest.waypoint_id),
            latest.superstep + 1,
            &self.paladin_port,
            &trace,
            &self.interceptors,
            &self.cancellation_token,
            &self.cancellation_probe,
            Some(Arc::clone(&self.waypoint_port)),
            None,
            // --- HITL-03, D-14: the resumed run's own Waypoints stay on the
            // SAME branch `latest` (the just-loaded `AwaitingInput`
            // Waypoint) was on -- propagated verbatim, so a suspended
            // branch's resume never silently reverts to mainline.
            latest.fork_of,
            Some(responses_by_node),
            self.shutdown_grace,
            // --- FT-FR-09, D-19: a top-level resume has no parent node to
            // beat -- only a Battalion child dispatch passes `Some`.
            None,
            self.node_cache.clone(),
            self.vault.clone(),
            self.structured_executor.clone(),
        )
        .await;
        trace.emit(TraceEvent::RunFinished {
            status: run_finish_status(&outcome),
            total_supersteps: trace.superstep_count(),
            total_tokens: trace.token_total(),
            duration_ms: run_started_at.elapsed().as_millis() as u64,
            trace_dropped_total: 0,
        });
        outcome
    }

    /// Re-enter the superstep loop from `from`, exactly as it ran the first
    /// time, producing a NEW branch chain while `thread`'s existing
    /// Waypoints stay byte-identical (HITL-03, D-16, D-17).
    ///
    /// Loads `from` through [`WaypointPort::get`] (absent ->
    /// [`EngineError::WaypointNotFound`]), checks `graph.fingerprint()`
    /// against the loaded Waypoint's own (mismatch -> [`EngineError::GraphMismatch`],
    /// ENG-FR-14) -- BEFORE anything else, mirroring `resume_with_options`'s
    /// own guard order -- then re-enters [`superstep::run_with_namespace`]
    /// with `parent_waypoint_id = Some(from)`, `fork_of = Some(from)` and
    /// superstep numbering continuing at `from`'s own `superstep + 1`
    /// (the SAME `from.superstep` when `from` carries a mid-muster
    /// `muster_progress` record, mirroring `resume_with_options`'s own
    /// mid-muster re-entry rule). Neither this call nor [`WarEngine::fork`]
    /// ever mutates, overwrites or deletes an existing Waypoint -- a branch
    /// is always new Waypoints on the SAME thread, distinguished only by
    /// `fork_of`. Calling `replay` twice from the same `from` produces two
    /// independent branches; neither call disturbs the other's Waypoints or
    /// the mainline chain.
    pub async fn replay(
        &self,
        graph: &WarGraph,
        thread: &ThreadId,
        from: WaypointId,
    ) -> Result<RunOutcome, EngineError> {
        self.replay_or_fork(graph, thread, from, None).await
    }

    /// Like [`WarEngine::replay`], but merges `edit` into the starting
    /// Waypoint's Battlefield through the schema's own dispatch rules
    /// BEFORE the first forked superstep runs (HITL-03, D-16), so an edit
    /// that flips a conditional edge's evaluated value routes the branch
    /// down a different path than the original chain took, while the
    /// original chain's own routing is unchanged.
    ///
    /// `edit` is merged as a single synthetic writer (this call names no
    /// real graph node as the edit's author) through
    /// [`Battlefield::merge`](paladin_core::platform::container::battlefield::Battlefield::merge)
    /// -- an edit naming a field the graph's schema does not declare fails
    /// with a typed [`EngineError::Battlefield`] and persists nothing,
    /// exactly like [`WarEngine::replay`]'s own guard clauses.
    pub async fn fork(
        &self,
        graph: &WarGraph,
        thread: &ThreadId,
        from: WaypointId,
        edit: StateDelta,
    ) -> Result<RunOutcome, EngineError> {
        self.replay_or_fork(graph, thread, from, Some(edit)).await
    }

    /// Shared implementation of [`WarEngine::replay`]/[`WarEngine::fork`]
    /// (HITL-03, D-16): `edit` is `None` for a plain replay, `Some(delta)`
    /// for a fork-with-edit.
    async fn replay_or_fork(
        &self,
        graph: &WarGraph,
        thread: &ThreadId,
        from: WaypointId,
        edit: Option<StateDelta>,
    ) -> Result<RunOutcome, EngineError> {
        let waypoint = self
            .waypoint_port
            .get(thread, &from)
            .await
            .map_err(|source| EngineError::WaypointRead { source })?
            .ok_or_else(|| EngineError::WaypointNotFound {
                thread: thread.clone(),
                waypoint: from,
            })?;

        // --- HITL-03, D-16: the fingerprint is checked before anything
        // else, mirroring `resume_with_options`'s own guard order --
        // nothing is persisted by either check above or this one.
        let expected = graph.fingerprint();
        if waypoint.graph_fingerprint != expected {
            return Err(EngineError::GraphMismatch {
                expected,
                got: waypoint.graph_fingerprint,
            });
        }

        let registry = self.dispatch_registry.resolver();
        graph.validate(registry, &self.registries)?;
        graph.validate_node_cache_backend(self.node_cache.is_some())?;
        graph.validate_structured_executor_backend(self.structured_executor.is_some())?;

        // --- HITL-03, D-16: `fork`'s edit is merged through the schema's
        // OWN dispatch rules -- an undeclared field is `EngineError::Battlefield`
        // (`BattlefieldError::UnknownField`), a typed error, and merge is
        // all-or-nothing (`Battlefield::merge`'s own contract), so a
        // rejected edit leaves `battlefield` untouched and persists
        // nothing. This happens BEFORE the first forked superstep runs
        // (D-16, acceptance 4).
        let resume_superstep = if waypoint.muster_progress.is_some() {
            waypoint.superstep
        } else {
            waypoint.superstep + 1
        };
        let mut battlefield = waypoint.battlefield;
        if let Some(edit) = edit {
            battlefield.merge(
                vec![(NodeId::new("__fork_edit__"), edit)],
                resume_superstep,
                registry,
            )?;
        }

        // D-03: a fresh, thread-scoped dispatcher for this call.
        // 28-06: reuses a `with_bound_trace`-pre-bound dispatcher for this
        // `thread` if pending (see `start`'s own comment above).
        let trace = self.take_or_build_trace_dispatcher(thread);
        self.remember_trace_dispatcher(&trace);
        // D-02: this call's own wall-clock start, for the `RunFinished`
        // site below.
        let run_started_at = tokio::time::Instant::now();
        trace.emit(TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: expected.to_string(),
        });
        let outcome = superstep::run_with_namespace(
            self.waypoint_port.as_ref(),
            self.durability,
            self.parallelism,
            registry,
            &self.registries,
            graph,
            thread.clone(),
            battlefield,
            waypoint.vanguard,
            waypoint.visit_counts,
            Some(waypoint.frontier),
            waypoint.muster_progress,
            // --- HITL-03, D-16: the new branch's FIRST Waypoint chains
            // from `from` -- `parent_waypoint_id = Some(from)`.
            Some(from),
            resume_superstep,
            &self.paladin_port,
            &trace,
            &self.interceptors,
            &self.cancellation_token,
            &self.cancellation_probe,
            Some(Arc::clone(&self.waypoint_port)),
            waypoint.checkpoint_ns,
            // --- HITL-03, D-14/D-16: `from` becomes the branch ROOT --
            // every Waypoint this run (and any nested Battalion child run,
            // via `ChildEngineResources::fork_of`) produces carries
            // `fork_of = Some(from)`, propagated verbatim.
            Some(from),
            None,
            self.shutdown_grace,
            // --- FT-FR-09, D-19: a top-level fork has no parent node to
            // beat -- only a Battalion child dispatch passes `Some`.
            None,
            self.node_cache.clone(),
            self.vault.clone(),
            self.structured_executor.clone(),
        )
        .await;
        trace.emit(TraceEvent::RunFinished {
            status: run_finish_status(&outcome),
            total_supersteps: trace.superstep_count(),
            total_tokens: trace.token_total(),
            duration_ms: run_started_at.elapsed().as_millis() as u64,
            trace_dropped_total: 0,
        });
        outcome
    }
}

/// Validate a submitted [`ParleyResponse::value`] against its own
/// `request`'s [`ParleyKind`] (HITL-02, D-10). Delegates the structural,
/// schema-oblivious rules to [`graph::validate_parley_value_for_kind`] --
/// the SAME per-kind validator [`graph::WarGraph::validate`]'s Gate
/// `on_expire` check (24-02) and `DirectiveParser`'s raise-time `on_expire`
/// check (24-03) both call (T-24-06) -- never a second, weaker check for
/// those rules.
///
/// Additionally, for [`ParleyKind::StateEdit`]: `graph`'s schema is checked
/// against the deserialised `StateDelta`'s field names, since ONLY this
/// call site has both a real submitted `StateEdit` value AND a live
/// `WarGraph` to validate it against (`validate_parley_value_for_kind`'s
/// other two call sites check a Gate's/Directive's own AUTHORED default
/// value at author time, when no submitted-response schema check applies).
/// An undeclared field rejects THIS response, never the run and never a
/// partial edit (T-24-13) -- `Battlefield::merge`'s own `UnknownField`
/// error is never allowed to reach this deep; this check runs first.
fn validate_response_shape(
    graph: &WarGraph,
    request: &ParleyRequest,
    value: &serde_json::Value,
) -> Result<(), String> {
    graph::validate_parley_value_for_kind(&request.kind, request.choices.as_deref(), value)?;
    if request.kind == ParleyKind::StateEdit {
        let delta: StateDelta = serde_json::from_value(value.clone())
            .map_err(|e| format!("StateEdit value must deserialize as a StateDelta: {e}"))?;
        let schema = graph.schema();
        let mut unknown: Vec<&str> = delta
            .values
            .keys()
            .filter(|field| schema.field_spec(field).is_none())
            .map(|field| field.as_str())
            .collect();
        if !unknown.is_empty() {
            unknown.sort_unstable();
            return Err(format!(
                "StateEdit value names field(s) not declared in the graph schema: {}",
                unknown.join(", ")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, RetryPredicate};
    use paladin_core::platform::container::battalion::campaign::EdgeCondition;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::parley::{
        OnExpire, ParleyId, ParleyKind, ParleyRequest,
    };
    use paladin_core::platform::container::transience::Transience;
    use paladin_core::platform::container::waypoint::{NodeOutcomeKind, Waypoint};
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream};
    use paladin_ports::output::trace_sink_port::MiddlewareAction;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use crate::engine::graph::{EdgeSpec, GateRequestTemplate};
    use crate::engine::test_support::{
        AttemptObservingNode, CountingFunctionNode, FailThenSucceedNode, FailingFunctionNode,
        FailingPaladinPort, FixedDecisionInterceptor, MusterFailThenSucceedWorker,
        RecordingInterceptor, RecordingPaladinPort, RecordingWaypointStore,
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
        ) -> Result<paladin_core::platform::container::directive::Directive, StateNodeError>
        {
            let mut delta = StateDelta::new();
            delta.set_raw(self.field.clone(), self.value.clone());
            Ok(delta.into())
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
        assert_eq!(
            mapping.render(&battlefield, None, None).unwrap(),
            "hello world!"
        );
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
            NodeSpec::paladin(
                make_paladin("summarizer"),
                InputMapping::new("summarize {topic}"),
                field_name.clone(),
            ),
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
            NodeSpec::paladin(
                make_paladin("first"),
                InputMapping::new("note one"),
                field_name.clone(),
            ),
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::paladin(
                make_paladin("second"),
                InputMapping::new("note two"),
                field_name.clone(),
            ),
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
            NodeSpec::paladin(make_paladin("counter"), InputMapping::new("go"), field_name),
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
            NodeSpec::paladin(make_paladin("failer"), InputMapping::new("go"), field_name),
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
            ..EngineLimits::default()
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
                    NodeSpec::paladin(
                        make_paladin(&format!("n{}", i + 1)),
                        InputMapping::new(format!("{{{input_field}}}")),
                        field_names[i].clone(),
                    ),
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
            TraceEvent::NodeProgress { .. } => "NodeProgress",
            TraceEvent::NodeFinished { .. } => "NodeFinished",
            TraceEvent::EdgeEvaluated { .. } => "EdgeEvaluated",
            TraceEvent::DeltaMerged { .. } => "DeltaMerged",
            TraceEvent::WaypointSaved { .. } => "WaypointSaved",
            TraceEvent::ParleyRaised { .. } => "ParleyRaised",
            TraceEvent::RunFinished { .. } => "RunFinished",
            TraceEvent::FallbackHop { .. } => "FallbackHop",
            TraceEvent::MiddlewareEvent { .. } => "MiddlewareEvent",
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
        let names: Vec<&str> = sink
            .events()
            .await
            .iter()
            .map(|record| trace_event_name(&record.event))
            .collect();
        assert_eq!(
            names,
            vec![
                "RunStarted",
                "SuperstepStarted",
                "NodeStarted",
                "NodeFinished",
                "DeltaMerged",
                // Plan 28-03, D-04: `a`'s one outgoing edge (`a -> b`) is
                // evaluated once `a` has run, between this superstep's
                // `DeltaMerged` and `WaypointSaved` -- `b` (the second
                // superstep's own node) has no outgoing edge of its own, so
                // no second `EdgeEvaluated` follows the second
                // `DeltaMerged` below.
                "EdgeEvaluated",
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
            InterceptDecision::Fail(StateNodeError("interceptor-forced failure".to_string()))
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

    // --- Plan 25-01: the Aegis retry loop -----------------------------

    /// D-09/D-15: the default `RetryPredicate` (`TransientOnly`) never
    /// retries an `Unknown`-classified error -- this plan has no
    /// adapter-sourced classifier yet (plan 25-02), so every
    /// `StateNodeError` classifies `Unknown` until then. Every test below
    /// that wants a retry to actually fire uses this predicate explicitly.
    fn retrying_aegis(max_attempts: u32) -> Aegis {
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts,
                retry_on: RetryPredicate::TransientAndUnknown,
                jitter: false,
                initial_interval: std::time::Duration::from_millis(1),
                ..RetryPolicy::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn transient_function_node_failure_is_retried_and_run_completes() {
        let node_id = NodeId::new("flaky");
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("transient-retry-completes").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "expected Completed, got {outcome:?}"
        );
        assert_eq!(node.run_count(), 2, "node must run exactly twice");

        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 1);
        let record = &waypoints[0].completed[0];
        assert_eq!(record.node_id, node_id);
        assert_eq!(record.attempt, 2, "the succeeding attempt is attempt 2");
    }

    #[tokio::test]
    async fn failed_attempt_delta_never_reaches_the_battlefield() {
        let node_id = NodeId::new("flaky-delta");
        let field_name = FieldName::new("result").unwrap();
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            field_name.clone(),
            serde_json::json!("only-the-winner"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));

        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("failed-attempt-delta-isolated").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&field_name).unwrap(),
                    Some("only-the-winner".to_string()),
                    "the merged Battlefield reflects only the succeeding attempt's value -- a \
                     failing `StateNode::run` returns `Err` with no `Directive` at all, so no \
                     earlier attempt's delta can ever reach the merge on any code path"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn each_attempt_reads_an_identical_battlefield_snapshot() {
        let node_id = NodeId::new("flaky-snapshot");
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));

        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("identical-snapshot-per-attempt").unwrap();
        engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        let snapshots = node.observed_snapshots();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(
            snapshots[0], snapshots[1],
            "every attempt observes an equal Battlefield snapshot"
        );
    }

    #[tokio::test]
    async fn interceptors_run_once_per_attempt_not_once_per_node() {
        let node_id = NodeId::new("flaky-intercepted");
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));

        let recorder = RecordingInterceptor::new();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![Arc::clone(&recorder) as Arc<dyn NodeInterceptor>]);
        let thread = ThreadId::new("interceptors-once-per-attempt").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            recorder.calls(),
            // `before` runs once per attempt (2 attempts here); `after`
            // (hooks.rs's own documented contract: "Never called ... for a
            // node whose own execution returned an error") runs only for
            // the SUCCEEDING attempt -- the failing first attempt produces
            // no `Directive`/delta for `after` to observe at all.
            vec!["before", "before", "after"],
            "before runs once per attempt; after only for the succeeding attempt"
        );
    }

    #[tokio::test]
    async fn interceptor_fail_decision_is_not_retried() {
        let node_id = NodeId::new("intercepted-fail");
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("x"),
            )),
        );
        graph.add_entry(node_id.clone());
        // The DEFAULT `RetryPredicate` (`TransientOnly`) -- not
        // `retrying_aegis`'s `TransientAndUnknown` -- because every
        // `StateNodeError` (including one from an interceptor's own `Fail`
        // decision) classifies `Unknown` until plan 25-02 lands a real
        // classifier (D-05/D-07 stand-in, see the retry loop's own
        // comment in `superstep.rs`): under `TransientOnly`, `Unknown`
        // never retries, so THIS is what proves the "not retried" claim.
        graph.set_aegis(
            node_id.clone(),
            Aegis {
                retry: Some(RetryPolicy::default()),
                ..Default::default()
            },
        );

        let interceptor = FixedDecisionInterceptor::new(|| {
            InterceptDecision::Fail(StateNodeError("intercepted failure".to_string()))
        });
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![Arc::clone(&interceptor) as Arc<dyn NodeInterceptor>]);
        let thread = ThreadId::new("interceptor-fail-not-retried").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Failed { .. }));
        assert_eq!(
            interceptor.before_call_count(),
            1,
            "a Fail decision is exactly one attempt, never retried"
        );
    }

    #[tokio::test]
    async fn interceptor_skip_decision_produces_exactly_one_attempt() {
        let node_id = NodeId::new("intercepted-skip");
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("x"),
            )),
        );
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));

        let interceptor =
            FixedDecisionInterceptor::new(|| InterceptDecision::Skip("skip-once".to_string()));
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_interceptors(vec![Arc::clone(&interceptor) as Arc<dyn NodeInterceptor>]);
        let thread = ThreadId::new("interceptor-skip-not-retried").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "Skip is never an error"
        );
        assert_eq!(
            interceptor.before_call_count(),
            1,
            "a Skip decision is exactly one attempt, never retried"
        );
    }

    #[tokio::test]
    async fn node_without_aegis_behaves_exactly_as_before() {
        let node_id = NodeId::new("no-aegis-failure");
        let node = FailThenSucceedNode::new(
            // Never reaches this attempt: `usize::MAX` fail-until means
            // every run fails.
            usize::MAX,
            "permanent-looking failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("never"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id);
        // Deliberately no `set_aegis`/`with_default_aegis` call.

        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("no-aegis-byte-identical").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Failed { .. }));
        assert_eq!(
            node.run_count(),
            1,
            "no Aegis means exactly one attempt, byte-identical to pre-Phase-25 behavior"
        );
    }

    #[tokio::test]
    async fn set_aegis_per_node_wins_wholesale_over_default_aegis() {
        let node_id = NodeId::new("own-aegis-wins");
        let node = FailThenSucceedNode::new(
            usize::MAX,
            "permanent-looking failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("never"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.with_default_aegis(retrying_aegis(5));
        // The node's own Aegis carries NO retry policy -- must win
        // wholesale over `default_aegis`'s retry policy, never merge
        // field-by-field.
        graph.set_aegis(
            node_id,
            Aegis {
                retry: None,
                ..Default::default()
            },
        );

        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("own-aegis-wholesale-override").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        assert!(matches!(outcome, RunOutcome::Failed { .. }));
        assert_eq!(
            node.run_count(),
            1,
            "the node's own retry:None Aegis wins wholesale over default_aegis's retry policy"
        );
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

    // --- D-14, PLAT-FR-04: CancellationProbe -> Halted, beside the token --
    // Nested in its own `mod` (rather than flat in `tests`) so
    // `cargo test -p paladin-battalion --lib cancellation_probe` selects
    // exactly this group by module-path substring match, mirroring how
    // `vault_tests`/`llm_decision::tests` are already scoped elsewhere in
    // this crate.
    mod cancellation_probe_tests {
        use super::*;

        /// Counts every `is_cancelled` call and answers `true` from the
        /// configured call number onward (never `false` again afterward).
        struct CountingProbe {
            calls: std::sync::atomic::AtomicUsize,
            cancel_at_call: usize,
        }

        #[async_trait]
        impl CancellationProbe for CountingProbe {
            async fn is_cancelled(&self, _thread: &ThreadId) -> bool {
                let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                call >= self.cancel_at_call
            }
        }

        /// A probe that always answers `false` -- proves an attached-but-
        /// never-cancelling probe changes nothing about a run's outcome.
        struct NeverCancellingProbe;

        #[async_trait]
        impl CancellationProbe for NeverCancellingProbe {
            async fn is_cancelled(&self, _thread: &ThreadId) -> bool {
                false
            }
        }

        #[tokio::test]
        async fn probe_cancelling_on_the_second_boundary_halts_after_one_completed_superstep() {
            let (graph, ids) = four_node_chain_graph();
            let probe = Arc::new(CountingProbe {
                calls: std::sync::atomic::AtomicUsize::new(0),
                cancel_at_call: 2,
            });
            let store = Arc::new(RecordingWaypointStore::new());
            let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
                .with_cancellation_probe(probe.clone());
            let thread = ThreadId::new("probe-cancel-second-boundary").unwrap();

            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                engine.start(&graph, thread.clone(), StateDelta::new()),
            )
            .await
            .expect("probe cancellation must not hang the run")
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
                vec![ids[1].clone()],
                "the Halted waypoint's vanguard must be exactly the node that would run next (n2), \
                 identical to the token path"
            );

            // Exactly one superstep's worth of work happened -- n1 ran, n2 never did.
            let all_node_ids: std::collections::HashSet<String> = waypoints
                .iter()
                .flat_map(|w| w.completed.iter().map(|r| r.node_id.as_str().to_string()))
                .collect();
            assert!(all_node_ids.contains(ids[0].as_str()));
            assert!(!all_node_ids.contains(ids[1].as_str()));
        }

        #[tokio::test]
        async fn probe_that_never_cancels_changes_nothing() {
            let (graph_a, _ids_a) = four_node_chain_graph();
            let engine_no_probe = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            );
            let outcome_a = engine_no_probe
                .start(
                    &graph_a,
                    ThreadId::new("no-probe").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap();

            let (graph_b, _ids_b) = four_node_chain_graph();
            let engine_with_probe = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_cancellation_probe(Arc::new(NeverCancellingProbe));
            let outcome_b = engine_with_probe
                .start(
                    &graph_b,
                    ThreadId::new("with-never-cancelling-probe").unwrap(),
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

        #[tokio::test]
        async fn probe_is_consulted_exactly_once_per_superstep_boundary() {
            // A 3-node sequential chain (one node per superstep): n1 -> n2 -> n3.
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                FieldName::new("trace").unwrap(),
                DispatchRule::Append,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let ids: Vec<NodeId> = (1..=3).map(|i| NodeId::new(format!("n{i}"))).collect();
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

            let probe = Arc::new(CountingProbe {
                calls: std::sync::atomic::AtomicUsize::new(0),
                cancel_at_call: usize::MAX, // never actually cancels
            });
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_cancellation_probe(probe.clone());

            let outcome = engine
                .start(
                    &graph,
                    ThreadId::new("probe-call-count").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::Completed { .. }));

            // --- D-14: the boundary check runs at the TOP of the loop, before
            // that iteration's own dispatch. The loop's own inline
            // "next_vanguard empty -> Completed" short-circuit
            // (`engine::superstep::run_with_namespace`) returns WITHOUT looping
            // back to the top for one more boundary check once the LAST
            // superstep's dispatch computes an empty `next_vanguard` -- so a
            // 3-superstep chain consults the probe exactly 3 times (once
            // before n1, once before n2, once before n3), never a 4th time
            // after n3 completes.
            assert_eq!(
                probe.calls.load(std::sync::atomic::Ordering::SeqCst),
                3,
                "the probe must be consulted exactly once per superstep boundary, no more, no less"
            );
        }

        #[tokio::test]
        async fn either_token_or_probe_cancelling_halts_the_run() {
            // The token is attached but never cancelled; only the probe fires --
            // proves either signal alone is sufficient to halt.
            let token = CancellationToken::new();
            let probe = Arc::new(CountingProbe {
                calls: std::sync::atomic::AtomicUsize::new(0),
                cancel_at_call: 2,
            });
            let (graph, ids) = four_node_chain_graph();
            let store = Arc::new(RecordingWaypointStore::new());
            let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
                .with_cancellation_token(token)
                .with_cancellation_probe(probe.clone());
            let thread = ThreadId::new("token-and-probe-either-halts").unwrap();

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
                vec![ids[1].clone()],
                "the never-cancelled token coexists with the cancelling probe; either signal halts"
            );
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

    // --- HITL-01, HITL-02, D-08, D-11: Parley suspend/resume, typed guards
    // (Phase 24 Plan 01) ------------------------------------------------

    fn sample_parley_request(node_id: NodeId, parley_id: ParleyId) -> ParleyRequest {
        ParleyRequest {
            parley_id,
            node_id,
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    /// Test 4 (Task 2): after suspension, `WarEngine::resume_with(&graph,
    /// &thread, vec![response])` delivers the response to the paused
    /// node's continuation via `NodeContext.parley_response()` and the run
    /// reaches `RunOutcome::Completed`.
    #[tokio::test]
    async fn parley_suspends_and_resumes_end_to_end() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = {
            let node_id_for_request = node_id.clone();
            CountingFunctionNode::with_context_directive(move |run, _state, ctx| {
                if run == 0 {
                    Directive {
                        delta: StateDelta::new(),
                        next: NextStep::Parley(sample_parley_request(
                            node_id_for_request.clone(),
                            parley_id,
                        )),
                    }
                } else {
                    let value = ctx
                        .parley_response()
                        .expect("parley_response must be populated on resume")
                        .value
                        .clone();
                    let mut delta = StateDelta::new();
                    delta.set_raw(FieldName::new("result").unwrap(), value);
                    delta.into()
                }
            })
        };
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());

        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("parley-e2e").unwrap();

        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        match suspended {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_id);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        let response = ParleyResponse {
            parley_id,
            // `kind`/`prompt` are stamped over by `resume_with` regardless
            // (mirrors `ParleyRequest.node_id`'s own engine-stamped
            // contract, HITL-01, D-07) -- these placeholder values are
            // never observed.
            kind: ParleyKind::Approval,
            prompt: String::new(),
            // Plan 24-04's `resume_with` validation matrix now enforces
            // `ParleyKind::Approval`'s shape rule (bool or one of
            // true/false/yes/no/approve/deny, case-insensitive) -- this
            // test only exercises pass-through-to-continuation, so the
            // value must be a rule-conforming string, not an arbitrary
            // one.
            value: serde_json::json!("approve"),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };

        let resumed = engine
            .resume_with(&graph, thread, vec![response])
            .await
            .unwrap();
        match resumed {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("result").unwrap())
                        .unwrap(),
                    Some("approve".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// `resume_with` rejects a response naming a `parley_id` this thread
    /// has no outstanding request for (D-10 happy-path guard), writing no
    /// Waypoint.
    #[tokio::test]
    async fn resume_with_unknown_parley_id_fails_and_writes_no_waypoint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(sample_parley_request(NodeId::new(""), parley_id)),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("parley-unknown-id").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let save_count_before = store.save_call_count();

        let wrong_response = ParleyResponse {
            parley_id: ParleyId::new(),
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!(true),
            responded_by: None,
            responded_at: Utc::now(),
            defaulted: false,
        };
        let err = engine
            .resume_with(&graph, thread, vec![wrong_response])
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::UnknownParleyId { .. }));
        assert_eq!(
            store.save_call_count(),
            save_count_before,
            "an invalid resume_with call must write no Waypoint"
        );
    }

    /// Task 3, Test 1: `WarEngine::resume` on a thread whose latest
    /// Waypoint is `AwaitingInput` returns `Err(EngineError::
    /// ThreadAwaitingInput { thread, parleys })`, the parleys list matches
    /// the persisted requests, and no additional Waypoint is written.
    #[tokio::test]
    async fn plain_resume_refuses_awaiting_input_thread() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(sample_parley_request(NodeId::new(""), parley_id)),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("plain-resume-awaiting").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let save_count_before = store.save_call_count();

        let err = engine.resume(&graph, thread).await.unwrap_err();
        match err {
            EngineError::ThreadAwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].node_id, node_id);
            }
            other => panic!("expected ThreadAwaitingInput, got {other:?}"),
        }
        assert_eq!(
            store.save_call_count(),
            save_count_before,
            "plain resume against an AwaitingInput thread must write no Waypoint"
        );
    }

    /// Task 3, Test 2: `WarEngine::resume` on a `Halted` thread still runs
    /// through the generic fallthrough and makes progress (regression
    /// guard on Pitfall 2's "Halted is harmless" claim -- the literal test
    /// name the plan's `<verify>` command runs; the fuller scenario is
    /// `resume_continues_a_halted_thread_to_normal_completion` above).
    #[tokio::test]
    async fn plain_resume_still_continues_a_halted_thread() {
        let token = CancellationToken::new();
        token.cancel();
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
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_cancellation_token(token);
        let thread = ThreadId::new("plain-resume-halted").unwrap();
        let halted = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(halted, RunOutcome::Halted { .. }));

        let resume_engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let resumed = resume_engine.resume(&graph, thread).await.unwrap();
        assert!(matches!(resumed, RunOutcome::Completed { .. }));
    }

    /// Task 3, Test 3: a nested `NodeSpec::Battalion` child that suspends
    /// fails the parent with `EngineError::ParleyInChildUnsupported { node,
    /// child_thread }`, and no `AwaitingInput` Waypoint is written on the
    /// parent thread.
    #[tokio::test]
    async fn parley_in_battalion_child_is_typed_error() {
        let child_schema = one_field_schema();
        let mut child_graph = WarGraph::new(child_schema, EngineLimits::default());
        let child_node_id = NodeId::new("child-asker");
        let parley_id = ParleyId::new();
        let child_node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(sample_parley_request(NodeId::new(""), parley_id)),
        });
        child_graph.add_node(child_node_id.clone(), NodeSpec::Function(child_node));
        child_graph.add_entry(child_node_id);

        let mut parent_graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let battalion_node_id = NodeId::new("battalion");
        parent_graph.add_node(
            battalion_node_id.clone(),
            NodeSpec::Battalion {
                graph: Arc::new(child_graph),
                state_map: crate::engine::graph::StateMap::default(),
                restart_on_resume: false,
            },
        );
        parent_graph.add_entry(battalion_node_id.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("parley-in-child").unwrap();
        let outcome = engine
            .start(&parent_graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::Failed {
                error: EngineError::ParleyInChildUnsupported { node, .. },
                ..
            } => {
                assert_eq!(node, battalion_node_id);
            }
            other => panic!("expected Failed(ParleyInChildUnsupported), got {other:?}"),
        }

        let saved = store.saved_waypoints(&thread).await;
        assert!(
            !saved
                .iter()
                .any(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. })),
            "no AwaitingInput waypoint may be written on the parent thread"
        );
    }

    // --- HITL-02, D-10, D-11, D-12: the resume_with validation matrix,
    // partial-answer persistence and lazy expiry (Phase 24 Plan 04) ------
    //
    // RED-STATE MARKER: every test below references `EngineError` variants
    // (`ParleyAlreadyAnswered`, `ResponseShapeInvalid`, `ParleyExpired`,
    // `ThreadAlreadyFailed`) not yet added to the enum -- the crate does
    // not compile until the GREEN commit lands them alongside the
    // `resume_with` rewrite.

    /// A graph of `n` independent, single-parley-raising `Function` nodes
    /// (each its own entry point, no edges between them): on first visit
    /// each raises its own `Approval` parley (via `sample_parley_request`,
    /// no expiry); on the post-resume visit each writes its delivered
    /// value to its own field (`f0`, `f1`, ...). All `n` parleys are
    /// raised in the SAME superstep, so the suspending Waypoint carries
    /// all `n` requests together.
    fn multi_parley_graph(n: usize) -> (WarGraph, Vec<NodeId>, Vec<ParleyId>) {
        let fields: Vec<FieldName> = (0..n)
            .map(|i| FieldName::new(format!("f{i}")).unwrap())
            .collect();
        let schema = BattlefieldSchema::new(
            fields
                .iter()
                .map(|f| FieldSpec::new(f.clone(), DispatchRule::LastWrite, None, false))
                .collect(),
        );
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_ids: Vec<NodeId> = (0..n).map(|i| NodeId::new(format!("asker{i}"))).collect();
        let parley_ids: Vec<ParleyId> = (0..n).map(|_| ParleyId::new()).collect();
        for i in 0..n {
            let node_id_for_request = node_ids[i].clone();
            let parley_id = parley_ids[i];
            let field = fields[i].clone();
            graph.add_node(
                node_ids[i].clone(),
                NodeSpec::Function(CountingFunctionNode::with_context_directive(
                    move |run, _state, ctx| {
                        if run == 0 {
                            Directive {
                                delta: StateDelta::new(),
                                next: NextStep::Parley(sample_parley_request(
                                    node_id_for_request.clone(),
                                    parley_id,
                                )),
                            }
                        } else {
                            let value = ctx
                                .parley_response()
                                .expect("parley_response must be populated on resume")
                                .value
                                .clone();
                            let mut delta = StateDelta::new();
                            delta.set_raw(field.clone(), value);
                            delta.into()
                        }
                    },
                )),
            );
            graph.add_entry(node_ids[i].clone());
        }
        (graph, node_ids, parley_ids)
    }

    fn approval_response(parley_id: ParleyId, value: bool) -> ParleyResponse {
        ParleyResponse {
            parley_id,
            // Stamped over by `resume_with` regardless -- never observed.
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!(value),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        }
    }

    // --- Task 1: the total per-kind validation matrix -------------------

    /// Test 1: an unknown `parley_id` is rejected and writes no Waypoint.
    #[tokio::test]
    async fn resume_with_rejects_unknown_parley_id() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(sample_parley_request(NodeId::new(""), parley_id)),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-with-rejects-unknown-parley-id").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let save_count_before = store.save_call_count();

        let wrong_response = approval_response(ParleyId::new(), true);
        let err = engine
            .resume_with(&graph, thread, vec![wrong_response])
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::UnknownParleyId { .. }));
        assert_eq!(store.save_call_count(), save_count_before);
    }

    /// Test 2: a `parley_id` already answered (either by the thread's
    /// prior history, or by an earlier response in the SAME call) is
    /// rejected and writes no Waypoint.
    #[tokio::test]
    async fn resume_with_rejects_already_answered_parley() {
        // Cross-call: a second resume_with re-answering an already
        // accepted parley.
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-with-rejects-already-answered").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let outcome = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[0], true)],
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::AwaitingInput { .. }));

        let save_count_before = store.save_call_count();
        let err = engine
            .resume_with(
                &graph,
                thread,
                vec![approval_response(parley_ids[0], false)],
            )
            .await
            .unwrap_err();
        match err {
            EngineError::ParleyAlreadyAnswered { parley_id } => {
                assert_eq!(parley_id, parley_ids[0]);
            }
            other => panic!("expected ParleyAlreadyAnswered, got {other:?}"),
        }
        assert_eq!(
            store.save_call_count(),
            save_count_before,
            "re-answering an already-answered parley must write no Waypoint"
        );

        // Within one call: two responses answering the SAME parley_id are
        // BOTH rejected -- the first is accepted into the working set
        // before the second is checked (the "review manually" edge
        // probe's planner-resolved reading).
        let (graph2, _node_ids2, parley_ids2) = multi_parley_graph(1);
        let store2 = Arc::new(RecordingWaypointStore::new());
        let engine2 = WarEngine::new(Arc::new(UnimplementedPaladinPort), store2.clone());
        let thread2 = ThreadId::new("resume-with-rejects-duplicate-in-one-call").unwrap();
        engine2
            .start(&graph2, thread2.clone(), StateDelta::new())
            .await
            .unwrap();
        let save_count_before2 = store2.save_call_count();
        let err2 = engine2
            .resume_with(
                &graph2,
                thread2,
                vec![
                    approval_response(parley_ids2[0], true),
                    approval_response(parley_ids2[0], false),
                ],
            )
            .await
            .unwrap_err();
        assert!(matches!(err2, EngineError::ParleyAlreadyAnswered { .. }));
        assert_eq!(store2.save_call_count(), save_count_before2);
    }

    /// Test 3: one invalid-shape case per `ParleyKind`, each rejected with
    /// `ResponseShapeInvalid` naming the offending `parley_id`.
    #[tokio::test]
    async fn resume_with_rejects_wrong_shape_per_kind() {
        struct Case {
            kind: ParleyKind,
            choices: Option<Vec<String>>,
            invalid_value: serde_json::Value,
        }
        let cases = vec![
            Case {
                kind: ParleyKind::Approval,
                choices: None,
                invalid_value: serde_json::json!(123),
            },
            Case {
                kind: ParleyKind::Choice,
                choices: Some(vec!["yes".to_string(), "no".to_string()]),
                invalid_value: serde_json::json!("maybe"),
            },
            Case {
                kind: ParleyKind::FreeText,
                choices: None,
                invalid_value: serde_json::json!(42),
            },
            Case {
                kind: ParleyKind::StateEdit,
                choices: None,
                invalid_value: serde_json::json!("not-a-state-delta"),
            },
        ];

        for case in cases {
            let node_id = NodeId::new("asker");
            let parley_id = ParleyId::new();
            let request = ParleyRequest {
                parley_id,
                node_id: node_id.clone(),
                kind: case.kind.clone(),
                prompt: "provide input".to_string(),
                payload: serde_json::json!({}),
                choices: case.choices.clone(),
                expires_at: None,
                created_at: Utc::now(),
                on_expire: OnExpire::FailRun,
            };
            let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
            let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Parley(request.clone()),
            });
            graph.add_node(node_id.clone(), NodeSpec::Function(node));
            graph.add_entry(node_id);

            let store = Arc::new(RecordingWaypointStore::new());
            let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
            let thread = ThreadId::new(format!("resume-with-wrong-shape-{:?}", case.kind)).unwrap();
            engine
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .unwrap();

            let response = ParleyResponse {
                parley_id,
                kind: case.kind.clone(),
                prompt: String::new(),
                value: case.invalid_value.clone(),
                responded_by: Some("tester".to_string()),
                responded_at: Utc::now(),
                defaulted: false,
            };
            let err = engine
                .resume_with(&graph, thread, vec![response])
                .await
                .unwrap_err();
            match err {
                EngineError::ResponseShapeInvalid {
                    parley_id: err_parley_id,
                    ..
                } => assert_eq!(err_parley_id, parley_id, "kind {:?}", case.kind),
                other => panic!(
                    "expected ResponseShapeInvalid for kind {:?}, got {other:?}",
                    case.kind
                ),
            }
        }
    }

    /// Test 4: a `StateEdit` response naming an undeclared schema field
    /// rejects THIS response, leaves the thread `AwaitingInput`, and
    /// applies no partial edit.
    #[tokio::test]
    async fn state_edit_unknown_schema_field_rejects_the_response_not_the_run() {
        let schema = string_field_schema("known", "");
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("editor");
        let parley_id = ParleyId::new();
        let request = ParleyRequest {
            parley_id,
            node_id: node_id.clone(),
            kind: ParleyKind::StateEdit,
            prompt: "edit".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        };
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(request.clone()),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("state-edit-unknown-schema-field").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let response = ParleyResponse {
            parley_id,
            kind: ParleyKind::StateEdit,
            prompt: String::new(),
            value: serde_json::json!({"values": {"undeclared": "x"}}),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let err = engine
            .resume_with(&graph, thread.clone(), vec![response])
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::ResponseShapeInvalid { .. }));

        let latest = store.latest(&thread).await.unwrap().unwrap();
        match latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 1);
                assert!(responses.is_empty(), "no partial edit may be applied");
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 5: a submission of three responses where the third is invalid
    /// writes no Waypoint at all and leaves `latest(thread)` unchanged.
    #[tokio::test]
    async fn resume_with_validation_is_total_before_any_write() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(3);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-with-validation-is-total").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let before = store.latest(&thread).await.unwrap().unwrap();
        let save_count_before = store.save_call_count();

        let mut invalid_response = approval_response(parley_ids[2], true);
        invalid_response.value = serde_json::json!(999);
        let responses = vec![
            approval_response(parley_ids[0], true),
            approval_response(parley_ids[1], false),
            invalid_response,
        ];

        let err = engine
            .resume_with(&graph, thread.clone(), responses)
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::ResponseShapeInvalid { .. }));
        assert_eq!(
            store.save_call_count(),
            save_count_before,
            "a submission with any invalid response must write no Waypoint"
        );
        let after = store.latest(&thread).await.unwrap().unwrap();
        assert_eq!(after, before, "latest(thread) must be byte-identical");
    }

    /// Test 6: a graph fingerprint mismatch is returned before any
    /// response is even inspected.
    #[tokio::test]
    async fn resume_with_checks_graph_fingerprint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(sample_parley_request(NodeId::new(""), parley_id)),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resume-with-checks-fingerprint").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let mut altered_graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        altered_graph.add_node(
            node_id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("noop"),
            )),
        );
        altered_graph.add_node(
            NodeId::new("extra"),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("extra"),
            )),
        );
        altered_graph.add_entry(node_id);
        assert_ne!(graph.fingerprint(), altered_graph.fingerprint());

        // Even an obviously-invalid response (unknown parley id) must not
        // be inspected before the fingerprint check runs.
        let err = engine
            .resume_with(
                &altered_graph,
                thread,
                vec![approval_response(ParleyId::new(), true)],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::GraphMismatch { .. }));
    }

    /// Test 7: a `Running`/`Completed` latest Waypoint returns
    /// `ThreadNotAwaitingInput` carrying the observed status.
    #[tokio::test]
    async fn resume_with_rejects_non_awaiting_input_thread() {
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

        let engine = engine();
        let thread = ThreadId::new("resume-with-rejects-non-awaiting-input").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let err = engine
            .resume_with(&graph, thread, Vec::new())
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::ThreadNotAwaitingInput { .. }));
    }

    /// Test 8: a valid `ParleyId` outstanding on a DIFFERENT thread is
    /// `UnknownParleyId` here -- never a global lookup.
    #[tokio::test]
    async fn resume_with_parley_ids_are_scoped_to_the_requested_thread() {
        let (graph_a, _node_ids_a, parley_ids_a) = multi_parley_graph(1);
        let (graph_b, _node_ids_b, _parley_ids_b) = multi_parley_graph(1);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());

        let thread_a = ThreadId::new("resume-with-scoped-a").unwrap();
        engine
            .start(&graph_a, thread_a.clone(), StateDelta::new())
            .await
            .unwrap();

        let thread_b = ThreadId::new("resume-with-scoped-b").unwrap();
        engine
            .start(&graph_b, thread_b.clone(), StateDelta::new())
            .await
            .unwrap();

        let err = engine
            .resume_with(
                &graph_b,
                thread_b,
                vec![approval_response(parley_ids_a[0], true)],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::UnknownParleyId { .. }));
    }

    // --- Task 2: partial answers and durable response consumption -------

    /// Test 1: with two outstanding parleys, answering one writes a child
    /// Waypoint at the SAME superstep whose status is `AwaitingInput` with
    /// `responses.len() == 1` and `parleys` still listing both requests.
    #[tokio::test]
    async fn partial_answer_persists_new_awaiting_input_at_same_superstep() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("partial-answer-same-superstep").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let original_superstep = match &suspended {
            RunOutcome::AwaitingInput { waypoint, .. } => {
                store
                    .get(&thread, waypoint)
                    .await
                    .unwrap()
                    .unwrap()
                    .superstep
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        };

        let outcome = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[0], true)],
            )
            .await
            .unwrap();
        match outcome {
            RunOutcome::AwaitingInput { parleys, waypoint } => {
                assert_eq!(parleys.len(), 1);
                let wp = store.get(&thread, &waypoint).await.unwrap().unwrap();
                assert_eq!(
                    wp.superstep, original_superstep,
                    "a partial answer must persist at the SAME superstep"
                );
                match wp.status {
                    WaypointStatus::AwaitingInput {
                        parleys: wp_parleys,
                        responses,
                    } => {
                        assert_eq!(wp_parleys.len(), 2);
                        assert_eq!(responses.len(), 1);
                    }
                    other => panic!("expected AwaitingInput, got {other:?}"),
                }
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 2: the returned `RunOutcome::AwaitingInput` lists exactly the
    /// one still-unanswered request.
    #[tokio::test]
    async fn partial_answer_returns_only_remaining_parleys() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let engine = engine();
        let thread = ThreadId::new("partial-answer-remaining").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let outcome = engine
            .resume_with(&graph, thread, vec![approval_response(parley_ids[0], true)])
            .await
            .unwrap();
        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_ids[1]);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 3: answering the second parley proceeds into the resume
    /// superstep rather than writing another `AwaitingInput` Waypoint.
    #[tokio::test]
    async fn answering_the_last_parley_advances_the_run() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let engine = engine();
        let thread = ThreadId::new("answering-last-parley-advances").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let partial = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[0], true)],
            )
            .await
            .unwrap();
        assert!(matches!(partial, RunOutcome::AwaitingInput { .. }));

        let final_outcome = engine
            .resume_with(
                &graph,
                thread,
                vec![approval_response(parley_ids[1], false)],
            )
            .await
            .unwrap();
        match final_outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<bool>(&FieldName::new("f0").unwrap())
                        .unwrap(),
                    Some(true)
                );
                assert_eq!(
                    final_state
                        .get::<bool>(&FieldName::new("f1").unwrap())
                        .unwrap(),
                    Some(false)
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Test 4: loading `latest(thread)` from the store handle directly
    /// (no in-process WarEngine state involved) reports two parleys and
    /// one response.
    #[tokio::test]
    async fn partial_answer_state_is_queryable_from_the_waypoint_alone() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("partial-answer-queryable").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[0], true)],
            )
            .await
            .unwrap();

        let latest = store.latest(&thread).await.unwrap().unwrap();
        match latest.status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 2);
                assert_eq!(responses.len(), 1);
                assert_eq!(responses[0].parley_id, parley_ids[0]);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 5: with `fail_next_save` armed, a `resume_with` whose Waypoint
    /// write fails leaves the previous `AwaitingInput` Waypoint as latest,
    /// and re-submitting the identical response succeeds rather than
    /// returning `ParleyAlreadyAnswered`.
    #[tokio::test]
    async fn resubmitting_responses_after_a_failed_save_is_safe() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(2);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("resubmit-after-failed-save").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let original_waypoint = match suspended {
            RunOutcome::AwaitingInput { waypoint, .. } => waypoint,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };

        store.fail_next_save();
        let response = approval_response(parley_ids[0], true);
        let err = engine
            .resume_with(&graph, thread.clone(), vec![response.clone()])
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::WaypointWrite { .. }));

        let latest_after_failure = store.latest(&thread).await.unwrap().unwrap();
        assert_eq!(
            latest_after_failure.waypoint_id, original_waypoint,
            "a failed save must leave the original AwaitingInput Waypoint as latest"
        );

        let outcome = engine
            .resume_with(&graph, thread, vec![response])
            .await
            .unwrap();
        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_ids[1]);
            }
            other => panic!("expected AwaitingInput (safe resubmission), got {other:?}"),
        }
    }

    /// Test 6: each partial answer's Waypoint carries `parent_waypoint_id`
    /// pointing at the previous one, so the sequence is a chain.
    #[tokio::test]
    async fn chain_of_partial_answers_is_linear() {
        let (graph, _node_ids, parley_ids) = multi_parley_graph(3);
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("chain-of-partial-answers").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let root_waypoint = match suspended {
            RunOutcome::AwaitingInput { waypoint, .. } => waypoint,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };

        let first = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[0], true)],
            )
            .await
            .unwrap();
        let first_waypoint = match first {
            RunOutcome::AwaitingInput { waypoint, .. } => waypoint,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        let first_wp = store.get(&thread, &first_waypoint).await.unwrap().unwrap();
        assert_eq!(first_wp.parent_waypoint_id, Some(root_waypoint));

        let second = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_ids[1], true)],
            )
            .await
            .unwrap();
        let second_waypoint = match second {
            RunOutcome::AwaitingInput { waypoint, .. } => waypoint,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        let second_wp = store.get(&thread, &second_waypoint).await.unwrap().unwrap();
        assert_eq!(second_wp.parent_waypoint_id, Some(first_waypoint));

        let third = engine
            .resume_with(&graph, thread, vec![approval_response(parley_ids[2], true)])
            .await
            .unwrap();
        assert!(matches!(third, RunOutcome::Completed { .. }));
    }

    // --- Task 3: lazy expiry with both `on_expire` policies --------------

    /// Test 1: a request whose `expires_at` is in the past with
    /// `on_expire: FailRun` causes `resume_with` to persist a `Failed`
    /// Waypoint naming the expired parley, and to return
    /// `Err(ParleyExpired)`.
    #[tokio::test]
    async fn expired_parley_with_fail_run_persists_failed_waypoint() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let expires_at = Utc::now() - chrono::Duration::seconds(60);
        let request = ParleyRequest {
            parley_id,
            node_id: node_id.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            on_expire: OnExpire::FailRun,
        };
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(request.clone()),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("expired-parley-fail-run").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let err = engine
            .resume_with(&graph, thread.clone(), Vec::new())
            .await
            .unwrap_err();
        match err {
            EngineError::ParleyExpired {
                parley_id: err_id,
                expires_at: err_expires_at,
            } => {
                assert_eq!(err_id, parley_id);
                assert_eq!(err_expires_at, expires_at);
            }
            other => panic!("expected ParleyExpired, got {other:?}"),
        }

        let latest = store.latest(&thread).await.unwrap().unwrap();
        match latest.status {
            WaypointStatus::Failed {
                error, failed_node, ..
            } => {
                assert!(error.contains(parley_id.to_string().as_str()));
                assert_eq!(failed_node, node_id);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    /// Test 2: after that failure, both `resume` and `resume_with` refuse
    /// the thread.
    #[tokio::test]
    async fn expired_fail_run_thread_is_not_resumable() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let expires_at = Utc::now() - chrono::Duration::seconds(60);
        let request = ParleyRequest {
            parley_id,
            node_id: node_id.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            on_expire: OnExpire::FailRun,
        };
        let node = CountingFunctionNode::with_directive(move |_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(request.clone()),
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("expired-fail-run-not-resumable").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        engine
            .resume_with(&graph, thread.clone(), Vec::new())
            .await
            .unwrap_err();

        let resume_err = engine.resume(&graph, thread.clone()).await.unwrap_err();
        assert!(matches!(
            resume_err,
            EngineError::ThreadAlreadyFailed { .. }
        ));

        let resume_with_err = engine
            .resume_with(&graph, thread, Vec::new())
            .await
            .unwrap_err();
        assert!(matches!(
            resume_with_err,
            EngineError::ThreadNotAwaitingInput { .. }
        ));
    }

    /// Test 3: a request whose `expires_at` is in the past with
    /// `on_expire: ResumeWithDefault(v)` substitutes `v` as the response,
    /// records `responded_by: None` and `defaulted: true`, and lets the
    /// run proceed.
    #[tokio::test]
    async fn expired_parley_with_resume_with_default_substitutes_value() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let expires_at = Utc::now() - chrono::Duration::seconds(60);
        let request = ParleyRequest {
            parley_id,
            node_id: node_id.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            on_expire: OnExpire::ResumeWithDefault(serde_json::json!(true)),
        };
        let node = CountingFunctionNode::with_context_directive(move |run, _state, ctx| {
            if run == 0 {
                Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Parley(request.clone()),
                }
            } else {
                let response = ctx
                    .parley_response()
                    .expect("parley_response must be populated on resume");
                assert_eq!(response.responded_by, None);
                assert!(response.defaulted);
                let mut delta = StateDelta::new();
                delta.set_raw(FieldName::new("result").unwrap(), response.value.clone());
                delta.into()
            }
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("expired-parley-resume-with-default").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let outcome = engine
            .resume_with(&graph, thread, Vec::new())
            .await
            .unwrap();
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<bool>(&FieldName::new("result").unwrap())
                        .unwrap(),
                    Some(true)
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Test 4: a request whose `expires_at` is in the future is not
    /// treated as expired, and an ordinary submitted response completes
    /// the run normally.
    #[tokio::test]
    async fn expiry_is_evaluated_only_at_resume_time() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let expires_at = Utc::now() + chrono::Duration::seconds(3600);
        let request = ParleyRequest {
            parley_id,
            node_id: node_id.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        };
        let node = CountingFunctionNode::with_context_directive(move |run, _state, ctx| {
            if run == 0 {
                Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Parley(request.clone()),
                }
            } else {
                let value = ctx
                    .parley_response()
                    .expect("parley_response must be populated on resume")
                    .value
                    .clone();
                let mut delta = StateDelta::new();
                delta.set_raw(FieldName::new("result").unwrap(), value);
                delta.into()
            }
        });
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id);

        let engine = engine();
        let thread = ThreadId::new("expiry-evaluated-only-at-resume-time").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let outcome = engine
            .resume_with(&graph, thread, vec![approval_response(parley_id, true)])
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    /// Test 5: the substituted response's `defaulted` flag is persisted on
    /// the (partial-answer) `AwaitingInput` Waypoint and survives a serde
    /// round trip, so an audit can see it.
    #[tokio::test]
    async fn defaulted_marker_is_persisted_and_queryable() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("a").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(
                FieldName::new("b").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let node_a = NodeId::new("asker-a");
        let parley_a = ParleyId::new();
        let expires_at = Utc::now() - chrono::Duration::seconds(60);
        let request_a = ParleyRequest {
            parley_id: parley_a,
            node_id: node_a.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed a?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            on_expire: OnExpire::ResumeWithDefault(serde_json::json!(true)),
        };
        let field_a = FieldName::new("a").unwrap();
        graph.add_node(
            node_a.clone(),
            NodeSpec::Function(CountingFunctionNode::with_context_directive(
                move |run, _state, ctx| {
                    if run == 0 {
                        Directive {
                            delta: StateDelta::new(),
                            next: NextStep::Parley(request_a.clone()),
                        }
                    } else {
                        let value = ctx.parley_response().expect("populated").value.clone();
                        let mut delta = StateDelta::new();
                        delta.set_raw(field_a.clone(), value);
                        delta.into()
                    }
                },
            )),
        );
        graph.add_entry(node_a);

        let node_b = NodeId::new("asker-b");
        let parley_b = ParleyId::new();
        let field_b = FieldName::new("b").unwrap();
        let node_b_for_request = node_b.clone();
        graph.add_node(
            node_b.clone(),
            NodeSpec::Function(CountingFunctionNode::with_context_directive(
                move |run, _state, ctx| {
                    if run == 0 {
                        Directive {
                            delta: StateDelta::new(),
                            next: NextStep::Parley(sample_parley_request(
                                node_b_for_request.clone(),
                                parley_b,
                            )),
                        }
                    } else {
                        let value = ctx.parley_response().expect("populated").value.clone();
                        let mut delta = StateDelta::new();
                        delta.set_raw(field_b.clone(), value);
                        delta.into()
                    }
                },
            )),
        );
        graph.add_entry(node_b);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("defaulted-marker-queryable").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        // No responses submitted: parley_a's default substitutes
        // automatically; parley_b stays outstanding, so a partial
        // AwaitingInput Waypoint carrying the defaulted response persists.
        let outcome = engine
            .resume_with(&graph, thread.clone(), Vec::new())
            .await
            .unwrap();
        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_b);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        let latest = store.latest(&thread).await.unwrap().unwrap();
        match &latest.status {
            WaypointStatus::AwaitingInput { responses, .. } => {
                assert_eq!(responses.len(), 1);
                assert_eq!(responses[0].parley_id, parley_a);
                assert!(responses[0].defaulted);
                assert_eq!(responses[0].responded_by, None);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        // The marker survives a serde round trip.
        let json = serde_json::to_string(&latest).unwrap();
        let restored: Waypoint = serde_json::from_str(&json).unwrap();
        match restored.status {
            WaypointStatus::AwaitingInput { responses, .. } => {
                assert!(responses[0].defaulted);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 6: a submission mixing an expired `FailRun` parley (untouched
    /// by this call's own responses) with a valid response for a
    /// different, non-expired parley fails the whole submission with
    /// `ParleyExpired` before the valid response is ever accepted.
    #[tokio::test]
    async fn expired_and_valid_responses_in_one_submission() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("a").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(
                FieldName::new("b").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let node_a = NodeId::new("asker-a");
        let parley_a = ParleyId::new();
        let expires_at = Utc::now() - chrono::Duration::seconds(60);
        let request_a = ParleyRequest {
            parley_id: parley_a,
            node_id: node_a.clone(),
            kind: ParleyKind::Approval,
            prompt: "proceed a?".to_string(),
            payload: serde_json::json!({}),
            choices: None,
            expires_at: Some(expires_at),
            created_at: Utc::now() - chrono::Duration::seconds(120),
            on_expire: OnExpire::FailRun,
        };
        let field_a = FieldName::new("a").unwrap();
        graph.add_node(
            node_a.clone(),
            NodeSpec::Function(CountingFunctionNode::with_context_directive(
                move |run, _state, ctx| {
                    if run == 0 {
                        Directive {
                            delta: StateDelta::new(),
                            next: NextStep::Parley(request_a.clone()),
                        }
                    } else {
                        let value = ctx.parley_response().expect("populated").value.clone();
                        let mut delta = StateDelta::new();
                        delta.set_raw(field_a.clone(), value);
                        delta.into()
                    }
                },
            )),
        );
        graph.add_entry(node_a);

        let node_b = NodeId::new("asker-b");
        let parley_b = ParleyId::new();
        let field_b = FieldName::new("b").unwrap();
        let node_b_for_request = node_b.clone();
        graph.add_node(
            node_b.clone(),
            NodeSpec::Function(CountingFunctionNode::with_context_directive(
                move |run, _state, ctx| {
                    if run == 0 {
                        Directive {
                            delta: StateDelta::new(),
                            next: NextStep::Parley(sample_parley_request(
                                node_b_for_request.clone(),
                                parley_b,
                            )),
                        }
                    } else {
                        let value = ctx.parley_response().expect("populated").value.clone();
                        let mut delta = StateDelta::new();
                        delta.set_raw(field_b.clone(), value);
                        delta.into()
                    }
                },
            )),
        );
        graph.add_entry(node_b);

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("expired-and-valid-in-one-submission").unwrap();
        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let save_count_before = store.save_call_count();

        let err = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval_response(parley_b, true)],
            )
            .await
            .unwrap_err();
        match err {
            EngineError::ParleyExpired { parley_id, .. } => assert_eq!(parley_id, parley_a),
            other => panic!("expected ParleyExpired, got {other:?}"),
        }

        // The FailRun expiry itself persists exactly one Failed Waypoint
        // (the policy's own required write); nothing else is written.
        assert_eq!(store.save_call_count(), save_count_before + 1);
        let latest = store.latest(&thread).await.unwrap().unwrap();
        assert!(matches!(latest.status, WaypointStatus::Failed { .. }));
    }

    // --- HITL-01, D-05/D-06: Gate node dispatch (Phase 24 Plan 02) ------

    fn bool_field_schema(name: &str, default: bool) -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new(name).unwrap(),
            DispatchRule::LastWrite,
            Some(serde_json::json!(default)),
            false,
        )])
    }

    fn string_field_schema(name: &str, default: &str) -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new(name).unwrap(),
            DispatchRule::LastWrite,
            Some(serde_json::json!(default)),
            false,
        )])
    }

    /// Test 1 (Task 2): visiting a Gate suspends the run with an
    /// `AwaitingInput` Waypoint whose single `ParleyRequest` carries the
    /// rendered prompt, the rendered payload, the declared `choices`, the
    /// Gate's `ParleyKind` and `on_expire`.
    #[tokio::test]
    async fn gate_raises_parley_on_first_visit() {
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                FieldName::new("topic").unwrap(),
                DispatchRule::LastWrite,
                Some(serde_json::json!("launch")),
                false,
            ),
            FieldSpec::new(
                FieldName::new("approved").unwrap(),
                DispatchRule::LastWrite,
                Some(serde_json::json!(false)),
                false,
            ),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let request = GateRequestTemplate::new(
            ParleyKind::Approval,
            InputMapping::new("Proceed with {topic}?"),
        )
        .with_payload_template(InputMapping::new("{topic}"));
        graph.add_node(
            NodeId::new("approve"),
            NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
        );
        graph.add_entry(NodeId::new("approve"));

        let engine = engine();
        let thread = ThreadId::new("gate-raises").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                let req = &parleys[0];
                assert_eq!(req.node_id, NodeId::new("approve"));
                assert_eq!(req.kind, ParleyKind::Approval);
                assert_eq!(req.prompt, "Proceed with launch?");
                assert_eq!(req.payload, serde_json::json!("launch"));
                assert_eq!(req.choices, None);
                assert_eq!(req.on_expire, OnExpire::FailRun);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 2 (Task 2): with `expires_in: Some(d)`, the raised request's
    /// `expires_at` is `Some(created_at + d)`; with `None` it is `None`.
    #[tokio::test]
    async fn gate_stamps_expires_at_from_expires_in() {
        let with_expiry_schema = bool_field_schema("approved", false);
        let mut with_expiry_graph = WarGraph::new(with_expiry_schema, EngineLimits::default());
        let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"))
            .with_expires_in(std::time::Duration::from_secs(60));
        with_expiry_graph.add_node(
            NodeId::new("approve"),
            NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
        );
        with_expiry_graph.add_entry(NodeId::new("approve"));

        let engine1 = engine();
        let outcome = engine1
            .start(
                &with_expiry_graph,
                ThreadId::new("gate-expiry-some").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                let req = &parleys[0];
                let expires_at = req.expires_at.expect("expires_at must be Some");
                let expected = req.created_at + chrono::Duration::seconds(60);
                assert_eq!(expires_at, expected);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        let no_expiry_schema = bool_field_schema("approved", false);
        let mut no_expiry_graph = WarGraph::new(no_expiry_schema, EngineLimits::default());
        let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"));
        no_expiry_graph.add_node(
            NodeId::new("approve"),
            NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
        );
        no_expiry_graph.add_entry(NodeId::new("approve"));

        let engine2 = engine();
        let outcome2 = engine2
            .start(
                &no_expiry_graph,
                ThreadId::new("gate-expiry-none").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        match outcome2 {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys[0].expires_at, None);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    /// Test 3 (Task 2): resuming a `kind: Approval` Gate with the JSON
    /// string `"approve"` writes JSON `true` to a Bool `output_field`, and
    /// `"deny"` writes `false`; the accepted set is `true`/`false` and
    /// case-insensitive `yes`/`no`/`approve`/`deny`.
    #[tokio::test]
    async fn gate_writes_normalised_approval_value_on_resume() {
        for (submitted, expected) in [
            (serde_json::json!("approve"), true),
            (serde_json::json!("deny"), false),
            (serde_json::json!("YES"), true),
            (serde_json::json!("No"), false),
            (serde_json::json!(true), true),
            (serde_json::json!(false), false),
        ] {
            let schema = bool_field_schema("approved", false);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"));
            graph.add_node(
                NodeId::new("approve"),
                NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
            );
            graph.add_entry(NodeId::new("approve"));

            let engine = engine();
            let thread = ThreadId::new(format!("gate-approval-{submitted}")).unwrap();
            let suspended = engine
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .unwrap();
            let parley_id = match suspended {
                RunOutcome::AwaitingInput { parleys, .. } => parleys[0].parley_id,
                other => panic!("expected AwaitingInput, got {other:?}"),
            };

            let response = ParleyResponse {
                parley_id,
                // `kind`/`prompt` are stamped over by `resume_with`
                // regardless -- never observed.
                kind: ParleyKind::Approval,
                prompt: String::new(),
                value: submitted.clone(),
                responded_by: Some("tester".to_string()),
                responded_at: Utc::now(),
                defaulted: false,
            };
            let resumed = engine
                .resume_with(&graph, thread, vec![response])
                .await
                .unwrap();
            match resumed {
                RunOutcome::Completed { final_state, .. } => {
                    assert_eq!(
                        final_state
                            .get::<bool>(&FieldName::new("approved").unwrap())
                            .unwrap(),
                        Some(expected),
                        "submitted value {submitted} should normalise to {expected}"
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
        }
    }

    /// Test 4 (Task 2): the same Approval delivery against a String
    /// `output_field` writes `"true"`/`"false"`.
    #[tokio::test]
    async fn gate_writes_string_true_false_for_string_output_field() {
        for (submitted, expected) in [
            (serde_json::json!("approve"), "true"),
            (serde_json::json!("deny"), "false"),
        ] {
            let schema = string_field_schema("approved", "");
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"));
            graph.add_node(
                NodeId::new("approve"),
                NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
            );
            graph.add_entry(NodeId::new("approve"));

            let engine = engine();
            let thread = ThreadId::new(format!("gate-approval-string-{submitted}")).unwrap();
            let suspended = engine
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .unwrap();
            let parley_id = match suspended {
                RunOutcome::AwaitingInput { parleys, .. } => parleys[0].parley_id,
                other => panic!("expected AwaitingInput, got {other:?}"),
            };

            let response = ParleyResponse {
                parley_id,
                // `kind`/`prompt` are stamped over by `resume_with`
                // regardless -- never observed.
                kind: ParleyKind::Approval,
                prompt: String::new(),
                value: submitted.clone(),
                responded_by: Some("tester".to_string()),
                responded_at: Utc::now(),
                defaulted: false,
            };
            let resumed = engine
                .resume_with(&graph, thread, vec![response])
                .await
                .unwrap();
            match resumed {
                RunOutcome::Completed { final_state, .. } => {
                    assert_eq!(
                        final_state
                            .get::<String>(&FieldName::new("approved").unwrap())
                            .unwrap(),
                        Some(expected.to_string())
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
        }
    }

    /// Test 5 (Task 2): a `StateEdit` Gate merges the response's
    /// `StateDelta` and writes no `output_field`.
    #[tokio::test]
    async fn gate_state_edit_returns_delta_and_writes_no_output_field() {
        let schema = string_field_schema("extra", "");
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let request = GateRequestTemplate::new(ParleyKind::StateEdit, InputMapping::new("edit?"));
        graph.add_node(NodeId::new("editor"), NodeSpec::gate(request, None));
        graph.add_entry(NodeId::new("editor"));

        let engine = engine();
        let thread = ThreadId::new("gate-state-edit").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let parley_id = match suspended {
            RunOutcome::AwaitingInput { parleys, .. } => parleys[0].parley_id,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };

        let response = ParleyResponse {
            parley_id,
            // `kind`/`prompt` are stamped over by `resume_with` regardless
            // (mirrors `ParleyRequest.node_id`'s own engine-stamped
            // contract, HITL-01, D-07) -- these placeholder values are
            // never observed.
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!({"values": {"extra": "hello"}}),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let resumed = engine
            .resume_with(&graph, thread, vec![response])
            .await
            .unwrap();
        match resumed {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state
                        .get::<String>(&FieldName::new("extra").unwrap())
                        .unwrap(),
                    Some("hello".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Test 6 (Task 2): an Approval Gate plus a `Contains("true")` edge and
    /// a `Contains("false")` edge routes to the action node on approval and
    /// the cancellation node on denial (the E2E-2 shape).
    ///
    /// The edge needles are the full `"approved":true` / `"approved":false`
    /// key-value pairs, not the bare words `true`/`false`: `Contains`
    /// matches against `serde_json::to_string(&battlefield)`, which embeds
    /// the WHOLE `BattlefieldSchema` alongside the current values --
    /// including every OTHER field's `required: bool` flag (`"required":
    /// false"` for any non-required field, always present regardless of
    /// `approved`'s own value). A bare `Contains("false")` needle would
    /// therefore match on every superstep from the unrelated `"required":
    /// false"` text alone, independent of whether the gate was approved or
    /// denied (confirmed empirically while authoring this test: both edges
    /// fired for an "approve" response, corrupting the run with a
    /// `DispatchConflict`). Anchoring the needle to `"approved":<value>`
    /// disambiguates it from any other boolean-shaped text the serialised
    /// schema happens to carry -- this is a caveat of `Contains`/`Regex`'s
    /// whole-Battlefield-JSON matching strategy generally (pre-dating this
    /// plan), not something specific to `Gate`; a real graph author's edge
    /// condition should be similarly specific.
    #[tokio::test]
    async fn approval_gate_routes_both_branches() {
        async fn run_branch(submitted: serde_json::Value) -> String {
            let schema = BattlefieldSchema::new(vec![
                FieldSpec::new(
                    FieldName::new("approved").unwrap(),
                    DispatchRule::LastWrite,
                    Some(serde_json::json!(false)),
                    false,
                ),
                FieldSpec::new(
                    FieldName::new("path").unwrap(),
                    DispatchRule::LastWrite,
                    None,
                    false,
                ),
            ]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"));
            graph.add_node(
                NodeId::new("approve"),
                NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
            );
            graph.add_node(
                NodeId::new("act"),
                NodeSpec::Function(CountingFunctionNode::fixed(
                    FieldName::new("path").unwrap(),
                    serde_json::json!("act"),
                )),
            );
            graph.add_node(
                NodeId::new("cancel"),
                NodeSpec::Function(CountingFunctionNode::fixed(
                    FieldName::new("path").unwrap(),
                    serde_json::json!("cancel"),
                )),
            );
            graph.add_edge(EdgeSpec {
                from: NodeId::new("approve"),
                to: NodeId::new("act"),
                condition: Some(EdgeCondition::Contains(r#""approved":true"#.to_string())),
            });
            graph.add_edge(EdgeSpec {
                from: NodeId::new("approve"),
                to: NodeId::new("cancel"),
                condition: Some(EdgeCondition::Contains(r#""approved":false"#.to_string())),
            });
            graph.add_entry(NodeId::new("approve"));

            let engine = engine();
            let thread = ThreadId::new(format!("gate-routes-{submitted}")).unwrap();
            let suspended = engine
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .unwrap();
            let parley_id = match suspended {
                RunOutcome::AwaitingInput { parleys, .. } => parleys[0].parley_id,
                other => panic!("expected AwaitingInput, got {other:?}"),
            };
            let response = ParleyResponse {
                parley_id,
                // `kind`/`prompt` are stamped over by `resume_with`
                // regardless -- never observed.
                kind: ParleyKind::Approval,
                prompt: String::new(),
                value: submitted,
                responded_by: Some("tester".to_string()),
                responded_at: Utc::now(),
                defaulted: false,
            };
            let resumed = engine
                .resume_with(&graph, thread, vec![response])
                .await
                .unwrap();
            match resumed {
                RunOutcome::Completed { final_state, .. } => final_state
                    .get::<String>(&FieldName::new("path").unwrap())
                    .unwrap()
                    .expect("path field must be set"),
                other => panic!("expected Completed, got {other:?}"),
            }
        }

        assert_eq!(run_branch(serde_json::json!("approve")).await, "act");
        assert_eq!(run_branch(serde_json::json!("deny")).await, "cancel");
    }

    /// Test 7 (Task 2): a registered `Custom` evaluator on an edge whose
    /// source is a Gate receives the Gate's `output_field` value, not the
    /// whole serialised Battlefield.
    #[tokio::test]
    async fn gate_source_uses_output_field_for_custom_evaluator() {
        struct RecordingEvaluator(std::sync::Arc<std::sync::Mutex<Option<String>>>);

        #[async_trait]
        impl EdgeConditionEvaluator for RecordingEvaluator {
            async fn evaluate(
                &self,
                output: &str,
                _ctx: &crate::edge_evaluator::EdgeContext<'_>,
            ) -> Result<bool, crate::edge_evaluator::EdgeEvaluatorError> {
                *self.0.lock().unwrap() = Some(output.to_string());
                Ok(true)
            }
        }

        let schema = string_field_schema("approved", "");
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let request = GateRequestTemplate::new(ParleyKind::Approval, InputMapping::new("go?"));
        graph.add_node(
            NodeId::new("approve"),
            NodeSpec::gate(request, Some(FieldName::new("approved").unwrap())),
        );
        graph.add_node(
            NodeId::new("target"),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("approved").unwrap(),
                serde_json::json!("unreachable-if-not-fired"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: NodeId::new("approve"),
            to: NodeId::new("target"),
            condition: Some(EdgeCondition::Custom("record".to_string())),
        });
        graph.add_entry(NodeId::new("approve"));

        let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_edge_evaluator("record", Arc::new(RecordingEvaluator(captured.clone())));

        let thread = ThreadId::new("gate-custom-evaluator").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let parley_id = match suspended {
            RunOutcome::AwaitingInput { parleys, .. } => parleys[0].parley_id,
            other => panic!("expected AwaitingInput, got {other:?}"),
        };
        let response = ParleyResponse {
            parley_id,
            // `kind`/`prompt` are stamped over by `resume_with` regardless
            // (mirrors `ParleyRequest.node_id`'s own engine-stamped
            // contract, HITL-01, D-07) -- these placeholder values are
            // never observed.
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!("approve"),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        engine
            .resume_with(&graph, thread, vec![response])
            .await
            .unwrap();

        assert_eq!(captured.lock().unwrap().as_deref(), Some("true"));
    }

    // --- Plan 24-03, Task 2: the structured directive envelope's
    // `next.parley` key (HITL-01, D-07) -- a Paladin node raising a parley
    // through its own raw output, not a declarative `NodeSpec::Gate`.

    #[tokio::test]
    async fn paladin_node_parley_round_trips_to_awaiting_input() {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("approved").unwrap(),
            DispatchRule::LastWrite,
            Some(serde_json::json!(false)),
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("approver");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("approver"),
                InputMapping::new("decide"),
                FieldName::new("approved").unwrap(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(node_id.clone());

        let port = Arc::new(crate::engine::test_support::RecordingPaladinPort::new());
        port.set_output(
            "approver",
            r#"{"delta": {}, "next": {"parley": {"kind": "Approval", "prompt": "Approve this?"}}}"#,
        );
        let engine = engine_with_port(port);

        let thread = ThreadId::new("paladin-parley-round-trip").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();

        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                let request = &parleys[0];
                // The engine's own suspension arm re-stamps `node_id` from
                // the DISPATCHING node regardless of the directive parser's
                // placeholder (24-01) -- proving the round trip actually
                // reached the real suspension path, not a hand-built value.
                assert_eq!(request.node_id, node_id);
                assert_eq!(request.kind, ParleyKind::Approval);
                assert_eq!(request.prompt, "Approve this?");
                assert_eq!(request.on_expire, OnExpire::FailRun);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
    }

    // --- Plan 24-07, Task 1: `WarEngine::replay`/`WarEngine::fork` --
    // Chronicle branch lineage with byte-for-byte mainline immutability
    // (HITL-03, D-16, D-17). RED-STATE MARKER: `WarEngine::replay`,
    // `WarEngine::fork` and `EngineError::WaypointNotFound` do not exist
    // yet at this commit -- this whole block fails to compile until the
    // GREEN commit lands them.

    /// A linear two-node chain (`n1` -> `n2`, unconditional edge) whose
    /// first Waypoint (`n1`'s own, vanguard = `[n2]`) is a convenient
    /// `from` for a plain `replay` with no routing to flip.
    fn linear_two_node_graph() -> (WarGraph, NodeId, NodeId) {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let n1 = NodeId::new("n1");
        let n2 = NodeId::new("n2");
        graph.add_node(
            n1.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("n1"),
            )),
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("n2"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: n1.clone(),
            to: n2.clone(),
            condition: None,
        });
        graph.add_entry(n1.clone());
        (graph, n1, n2)
    }

    /// Read every Waypoint of `thread` back out of `port`, sorted by
    /// `(superstep, created_at)` -- oldest first, mirroring
    /// `subgraph_formation_in_campaign_test.rs`'s own `full_history` helper.
    async fn full_history<W: WaypointPort>(port: &W, thread: &ThreadId) -> Vec<Waypoint> {
        let summaries = port.history(thread, None, None).await.unwrap();
        let mut waypoints = Vec::with_capacity(summaries.len());
        for summary in summaries {
            let wp = port
                .get(thread, &summary.waypoint_id)
                .await
                .unwrap()
                .expect("summary's own waypoint must exist");
            waypoints.push(wp);
        }
        waypoints.sort_by_key(|w| (w.superstep, w.created_at));
        waypoints
    }

    /// Test 1: `replay(graph, thread, from)` produces a new branch Waypoint
    /// with `parent_waypoint_id = Some(from)`, `fork_of = Some(from)` and
    /// superstep numbering starting at `from.superstep + 1`.
    #[tokio::test]
    async fn replay_creates_a_new_branch_from_the_given_waypoint() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("replay-creates-branch").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let mainline = full_history(store.as_ref(), &thread).await;
        assert_eq!(mainline.len(), 2, "n1's superstep, then n2's");
        assert_eq!(mainline[0].fork_of, None);
        let from = mainline[0].waypoint_id;

        let outcome = engine.replay(&graph, &thread, from).await.unwrap();
        match outcome {
            RunOutcome::Completed { .. } => {}
            other => panic!("expected Completed, got {other:?}"),
        }

        let after = full_history(store.as_ref(), &thread).await;
        let branch: Vec<&Waypoint> = after.iter().filter(|w| w.fork_of == Some(from)).collect();
        assert_eq!(
            branch.len(),
            1,
            "replaying the tail of a 2-superstep chain from n1's Waypoint produces exactly one \
             new branch Waypoint"
        );
        assert_eq!(branch[0].parent_waypoint_id, Some(from));
        assert_eq!(branch[0].superstep, mainline[0].superstep + 1);
    }

    /// Test 2 (acceptance 3): every mainline Waypoint is byte-identical,
    /// serialized, before and after `replay`.
    #[tokio::test]
    async fn replay_leaves_the_mainline_byte_identical() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("replay-byte-identical").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let before = full_history(store.as_ref(), &thread).await;
        let before_bytes: Vec<(WaypointId, String)> = before
            .iter()
            .map(|w| (w.waypoint_id, serde_json::to_string(w).unwrap()))
            .collect();
        let from = before[0].waypoint_id;

        engine.replay(&graph, &thread, from).await.unwrap();

        for (id, expected_json) in &before_bytes {
            let after = store
                .get(&thread, id)
                .await
                .unwrap()
                .expect("mainline waypoint must still exist");
            let after_json = serde_json::to_string(&after).unwrap();
            assert_eq!(
                &after_json, expected_json,
                "mainline waypoint {id} must be byte-identical before and after replay"
            );
        }
    }

    /// Test 3: calling `replay` twice from the same Waypoint leaves the
    /// mainline byte-identical after BOTH calls, and neither call mutates
    /// or deletes the other's branch Waypoints.
    #[tokio::test]
    async fn replay_twice_is_safe() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("replay-twice-is-safe").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();

        let before = full_history(store.as_ref(), &thread).await;
        let before_bytes: Vec<(WaypointId, String)> = before
            .iter()
            .map(|w| (w.waypoint_id, serde_json::to_string(w).unwrap()))
            .collect();
        let from = before[0].waypoint_id;

        engine.replay(&graph, &thread, from).await.unwrap();
        let first_branch_ids: std::collections::HashSet<WaypointId> =
            full_history(store.as_ref(), &thread)
                .await
                .into_iter()
                .filter(|w| w.fork_of == Some(from))
                .map(|w| w.waypoint_id)
                .collect();
        assert_eq!(first_branch_ids.len(), 1);

        engine.replay(&graph, &thread, from).await.unwrap();
        let after = full_history(store.as_ref(), &thread).await;

        for (id, expected_json) in &before_bytes {
            let wp = store.get(&thread, id).await.unwrap().unwrap();
            assert_eq!(
                serde_json::to_string(&wp).unwrap(),
                *expected_json,
                "mainline waypoint {id} must stay byte-identical after two replays"
            );
        }

        let second_branch_ids: std::collections::HashSet<WaypointId> = after
            .iter()
            .filter(|w| w.fork_of == Some(from))
            .map(|w| w.waypoint_id)
            .collect();
        assert!(
            first_branch_ids.is_subset(&second_branch_ids),
            "the second replay must not delete or mutate the first branch's own Waypoint"
        );
        assert_eq!(
            second_branch_ids.len(),
            2,
            "two independent replays from the same Waypoint produce two distinct branch Waypoints"
        );
    }

    /// A branching graph (`seed` -> `router` -> `node_a` | `node_b`, decided
    /// by the `route` field's value) whose routing edge is a genuine
    /// `EdgeCondition::Contains` check against the whole post-merge
    /// Battlefield (D-06's "_" arm), not a hand-computed `Directive`.
    fn conditional_route_graph() -> (WarGraph, NodeId, NodeId, NodeId, NodeId) {
        let route = FieldName::new("route").unwrap();
        let a_ran = FieldName::new("a_ran").unwrap();
        let b_ran = FieldName::new("b_ran").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                route.clone(),
                DispatchRule::LastWrite,
                Some(serde_json::json!("a")),
                false,
            ),
            FieldSpec::new(
                a_ran.clone(),
                DispatchRule::LastWrite,
                Some(serde_json::json!(false)),
                false,
            ),
            FieldSpec::new(
                b_ran.clone(),
                DispatchRule::LastWrite,
                Some(serde_json::json!(false)),
                false,
            ),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());

        let seed = NodeId::new("seed");
        let router = NodeId::new("router");
        let node_a = NodeId::new("node_a");
        let node_b = NodeId::new("node_b");

        graph.add_node(
            seed.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        graph.add_node(
            router.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        graph.add_node(
            node_a.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                a_ran.clone(),
                serde_json::json!(true),
            )),
        );
        graph.add_node(
            node_b.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                b_ran.clone(),
                serde_json::json!(true),
            )),
        );

        graph.add_edge(EdgeSpec {
            from: seed.clone(),
            to: router.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: router.clone(),
            to: node_a.clone(),
            condition: Some(EdgeCondition::Contains("\"route\":\"a\"".to_string())),
        });
        graph.add_edge(EdgeSpec {
            from: router.clone(),
            to: node_b.clone(),
            condition: Some(EdgeCondition::Contains("\"route\":\"b\"".to_string())),
        });
        graph.add_entry(seed.clone());

        (graph, seed, router, node_a, node_b)
    }

    /// Test 4: a `fork` whose `StateDelta` edit changes the field a
    /// conditional edge tests routes down the OTHER branch, while the
    /// original chain's own routing is unchanged.
    #[tokio::test]
    async fn fork_with_edit_flips_a_conditional_edge() {
        let (graph, _seed, router, _node_a, _node_b) = conditional_route_graph();
        let a_ran = FieldName::new("a_ran").unwrap();
        let b_ran = FieldName::new("b_ran").unwrap();
        let route = FieldName::new("route").unwrap();

        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("fork-flips-conditional-edge").unwrap();

        let control = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        match control {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(final_state.get::<bool>(&a_ran).unwrap(), Some(true));
                assert_eq!(final_state.get::<bool>(&b_ran).unwrap(), Some(false));
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        let mainline = full_history(store.as_ref(), &thread).await;
        assert_eq!(mainline[0].vanguard, vec![router.clone()]);
        let from = mainline[0].waypoint_id;

        let mut edit = StateDelta::new();
        edit.set(route.clone(), "b").unwrap();

        let outcome = engine.fork(&graph, &thread, from, edit).await.unwrap();
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<bool>(&a_ran).unwrap(),
                    Some(false),
                    "the branch must never run node_a"
                );
                assert_eq!(
                    final_state.get::<bool>(&b_ran).unwrap(),
                    Some(true),
                    "the edit must route the branch down node_b instead"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        // The original chain's own routing (and every mainline Waypoint's
        // bytes) is unchanged.
        let mainline_after: Vec<Waypoint> = full_history(store.as_ref(), &thread)
            .await
            .into_iter()
            .filter(|w| w.fork_of.is_none())
            .collect();
        assert_eq!(mainline_after.len(), mainline.len());
        for (before, after) in mainline.iter().zip(mainline_after.iter()) {
            assert_eq!(
                serde_json::to_string(before).unwrap(),
                serde_json::to_string(after).unwrap()
            );
        }
    }

    /// Test 5: the edit is visible to the FIRST node executed on the
    /// branch -- proven by a node that reads the edited field at its own
    /// run time and echoes it into another field, rather than by routing
    /// alone (Test 4's concern).
    #[tokio::test]
    async fn fork_merges_the_edit_before_the_first_forked_superstep() {
        let value = FieldName::new("value").unwrap();
        let echoed = FieldName::new("echoed").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(
                value.clone(),
                DispatchRule::LastWrite,
                Some(serde_json::json!("orig")),
                false,
            ),
            FieldSpec::new(echoed.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let seed = NodeId::new("seed");
        let reader = NodeId::new("reader");
        graph.add_node(
            seed.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        let value_for_reader = value.clone();
        let echoed_for_reader = echoed.clone();
        graph.add_node(
            reader.clone(),
            NodeSpec::Function(CountingFunctionNode::new(move |_run, state| {
                let seen = state
                    .get::<String>(&value_for_reader)
                    .unwrap()
                    .unwrap_or_default();
                let mut delta = StateDelta::new();
                delta.set(echoed_for_reader.clone(), seen).unwrap();
                delta
            })),
        );
        graph.add_edge(EdgeSpec {
            from: seed.clone(),
            to: reader.clone(),
            condition: None,
        });
        graph.add_entry(seed.clone());

        let store = Arc::new(InMemoryWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("fork-edit-visible-first-node").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let mainline = full_history(store.as_ref(), &thread).await;
        assert_eq!(mainline[0].vanguard, vec![reader.clone()]);
        let from = mainline[0].waypoint_id;

        let mut edit = StateDelta::new();
        edit.set(value.clone(), "edited").unwrap();

        let outcome = engine.fork(&graph, &thread, from, edit).await.unwrap();
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&echoed).unwrap(),
                    Some("edited".to_string()),
                    "the reader node must observe the fork edit at its own dispatch time"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Test 6: an unknown `from` returns `EngineError::WaypointNotFound {
    /// thread, waypoint }` with nothing persisted.
    #[tokio::test]
    async fn replay_rejects_unknown_waypoint() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("replay-unknown-waypoint").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let saves_before = store.save_call_count();

        let unknown = WaypointId::generate();
        let err = engine.replay(&graph, &thread, unknown).await.unwrap_err();
        match err {
            EngineError::WaypointNotFound {
                thread: t,
                waypoint,
            } => {
                assert_eq!(t, thread);
                assert_eq!(waypoint, unknown);
            }
            other => panic!("expected WaypointNotFound, got {other:?}"),
        }
        assert_eq!(
            store.save_call_count(),
            saves_before,
            "an unknown `from` must persist nothing"
        );
    }

    /// Test 7: a graph whose fingerprint differs from `from`'s own returns
    /// `GraphMismatch` with nothing persisted, checked BEFORE the response
    /// (here: the graph itself) is otherwise inspected.
    #[tokio::test]
    async fn replay_rejects_fingerprint_mismatch() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("replay-fingerprint-mismatch").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let mainline = full_history(store.as_ref(), &thread).await;
        let from = mainline[0].waypoint_id;
        let saves_before = store.save_call_count();

        let (mut altered_graph, ..) = linear_two_node_graph();
        altered_graph.add_node(
            NodeId::new("extra"),
            NodeSpec::Function(CountingFunctionNode::fixed(
                FieldName::new("result").unwrap(),
                serde_json::json!("extra"),
            )),
        );
        assert_ne!(graph.fingerprint(), altered_graph.fingerprint());

        let err = engine
            .replay(&altered_graph, &thread, from)
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::GraphMismatch { .. }));
        assert_eq!(
            store.save_call_count(),
            saves_before,
            "a fingerprint mismatch must persist nothing"
        );
    }

    /// Test 8: an edit naming a field the schema does not declare is a
    /// typed error with nothing persisted.
    #[tokio::test]
    async fn fork_rejects_an_edit_the_schema_does_not_accept() {
        let (graph, _n1, _n2) = linear_two_node_graph();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("fork-rejects-bad-edit").unwrap();

        engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        let mainline = full_history(store.as_ref(), &thread).await;
        let from = mainline[0].waypoint_id;
        let saves_before = store.save_call_count();

        let mut edit = StateDelta::new();
        edit.set_raw(
            FieldName::new("not_a_real_field").unwrap(),
            serde_json::json!("x"),
        );

        let err = engine.fork(&graph, &thread, from, edit).await.unwrap_err();
        assert!(matches!(
            err,
            EngineError::Battlefield(BattlefieldError::UnknownField { .. })
        ));
        assert_eq!(
            store.save_call_count(),
            saves_before,
            "a schema-rejected edit must persist nothing"
        );
    }

    // --- Phase 25 Plan 07, Task 1: attempt history, per-attempt trace
    //     events, cache_hit defaults (FT-FR-03, D-16) ---------------------

    #[tokio::test]
    async fn failed_attempts_are_recorded_in_order() {
        let node_id = NodeId::new("flaky-history");
        let node = FailThenSucceedNode::new(
            3,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("attempt-history-in-order").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 3);

        let waypoints = store.saved_waypoints(&thread).await;
        let record = &waypoints[0].completed[0];
        assert_eq!(record.node_id, node_id);
        assert_eq!(record.attempt, 3, "the succeeding attempt is attempt 3");
        assert_eq!(
            record.attempts.len(),
            2,
            "exactly the two FAILED attempts, never the succeeding one"
        );
        let numbers: Vec<u32> = record.attempts.iter().map(|a| a.attempt).collect();
        assert_eq!(numbers, vec![1, 2], "ascending by attempt number");
        for attempt in &record.attempts {
            assert_eq!(attempt.error.node_id, node_id);
            assert_eq!(attempt.error.attempt, attempt.attempt);
            assert!(matches!(
                &attempt.error.source,
                NodeErrorSource::Function { message } if message == "transient failure"
            ));
            assert!(attempt.started_at <= record.started_at);
        }
    }

    #[tokio::test]
    async fn a_node_that_succeeds_first_time_records_an_empty_attempts_list() {
        let node_id = NodeId::new("steady");
        let node = FailThenSucceedNode::new(
            1,
            "never used",
            FieldName::new("result").unwrap(),
            serde_json::json!("first-time"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));

        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("attempt-history-empty").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let waypoints = store.saved_waypoints(&thread).await;
        let record = &waypoints[0].completed[0];
        assert_eq!(record.attempt, 1);
        assert!(record.attempts.is_empty());
    }

    #[tokio::test]
    async fn node_events_are_emitted_once_per_attempt_with_the_attempt_number() {
        let node_id = NodeId::new("flaky-traced");
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));

        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone());
        let thread = ThreadId::new("trace-per-attempt").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        // Give the background trace consumer a chance to drain.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let node_events: Vec<(&'static str, u32)> = sink
            .events()
            .await
            .iter()
            .filter_map(|record| match &record.event {
                TraceEvent::NodeStarted { attempt, .. } => Some(("NodeStarted", *attempt)),
                TraceEvent::NodeFinished { attempt, .. } => Some(("NodeFinished", *attempt)),
                _ => None,
            })
            .collect();
        assert_eq!(
            node_events,
            vec![
                ("NodeStarted", 1),
                ("NodeFinished", 1),
                ("NodeStarted", 2),
                ("NodeFinished", 2),
            ],
            "one NodeStarted/NodeFinished pair per attempt, each carrying its attempt number"
        );
    }

    #[tokio::test]
    async fn cache_hit_defaults_to_false_on_every_record_and_event() {
        let node_id = NodeId::new("flaky-uncached");
        let node = FailThenSucceedNode::new(
            2,
            "transient failure",
            FieldName::new("result").unwrap(),
            serde_json::json!("recovered"),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));

        let sink = RecordingTraceSink::new();
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_trace_sink(sink.clone());
        let thread = ThreadId::new("cache-hit-defaults-false").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let waypoints = store.saved_waypoints(&thread).await;
        assert!(!waypoints.is_empty());
        for record in waypoints.iter().flat_map(|w| w.completed.iter()) {
            assert!(
                !record.cache_hit,
                "no cache is configured, so no record is a hit"
            );
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let finished: Vec<bool> = sink
            .events()
            .await
            .iter()
            .filter_map(|record| match &record.event {
                TraceEvent::NodeFinished { cache_hit, .. } => Some(*cache_hit),
                _ => None,
            })
            .collect();
        assert_eq!(
            finished,
            vec![false, false],
            "one NodeFinished per attempt, none a hit"
        );
    }

    // --- Phase 25 Plan 07, Task 2: the structured failure path (D-08) ----

    /// An always-failing Function node under `retrying_aegis(max_attempts)`,
    /// run to its `Failed` Waypoint. Returns the run outcome and the store.
    async fn run_exhausting_node(
        thread: &str,
        aegis: Option<Aegis>,
    ) -> (NodeId, RunOutcome, Arc<RecordingWaypointStore>) {
        let node_id = NodeId::new("always-down");
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(
            node_id.clone(),
            NodeSpec::Function(FailingFunctionNode::new("always down")),
        );
        graph.add_entry(node_id.clone());
        if let Some(aegis) = aegis {
            graph.set_aegis(node_id.clone(), aegis);
        }
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let outcome = engine
            .start(&graph, ThreadId::new(thread).unwrap(), StateDelta::new())
            .await
            .unwrap();
        (node_id, outcome, store)
    }

    fn failed_status(waypoint: &Waypoint) -> (&str, &NodeId, Option<&NodeError>) {
        match &waypoint.status {
            WaypointStatus::Failed {
                error,
                failed_node,
                node_error,
            } => (error.as_str(), failed_node, node_error.as_ref()),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn exhausted_retry_writes_a_failed_waypoint_carrying_the_structured_error() {
        let (node_id, outcome, store) =
            run_exhausting_node("exhausted-structured", Some(retrying_aegis(2))).await;
        assert!(
            matches!(outcome, RunOutcome::Failed { .. }),
            "got {outcome:?}"
        );
        let thread = ThreadId::new("exhausted-structured").unwrap();
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 1);
        let (_, failed_node, node_error) = failed_status(&waypoints[0]);
        assert_eq!(failed_node, &node_id);
        let ne = node_error.expect("an Aegis-governed exhausted failure carries a NodeError");
        assert_eq!(ne.node_id, node_id);
        assert_eq!(ne.attempt, 2, "the last (exhausted) attempt number");
        // A `StateNodeError` carries no typed transience: `Unknown`, the
        // same classification the retry predicate saw.
        assert_eq!(ne.transience, Transience::Unknown);
        assert!(matches!(
            &ne.source,
            NodeErrorSource::Function { message } if message == "always down"
        ));
        let record = &waypoints[0].completed[0];
        assert_eq!(record.attempt, 2);
        assert_eq!(
            record.attempts.len(),
            1,
            "attempt 1 failed before the exhausted attempt 2"
        );
        assert_eq!(record.attempts[0].attempt, 1);
    }

    #[tokio::test]
    async fn the_display_line_on_a_failed_waypoint_is_unchanged() {
        let legacy_line = EngineError::Node(StateNodeError("always down".to_string())).to_string();

        let (_, outcome, store) =
            run_exhausting_node("display-with-aegis", Some(retrying_aegis(2))).await;
        let thread = ThreadId::new("display-with-aegis").unwrap();
        let with_aegis = store.saved_waypoints(&thread).await;
        let (line, _, node_error) = failed_status(&with_aegis[0]);
        assert_eq!(
            line, legacy_line,
            "the human-readable line is byte-identical"
        );
        assert!(node_error.is_some());
        match &outcome {
            RunOutcome::Failed { error, .. } => assert_eq!(error.to_string(), legacy_line),
            other => panic!("expected Failed, got {other:?}"),
        }

        let (_, _, store) = run_exhausting_node("display-without-aegis", None).await;
        let thread = ThreadId::new("display-without-aegis").unwrap();
        let without_aegis = store.saved_waypoints(&thread).await;
        let (line, _, _) = failed_status(&without_aegis[0]);
        assert_eq!(line, legacy_line);
    }

    #[tokio::test]
    async fn pre_aegis_and_limit_failures_carry_none() {
        // (a) A node failure with no Aegis at all: the pre-Phase-25 path,
        //     byte-identical (D-09) -- generic `EngineError::Node`, no
        //     structured error.
        let (node_id, outcome, store) = run_exhausting_node("pre-aegis-none", None).await;
        assert!(matches!(
            &outcome,
            RunOutcome::Failed {
                error: EngineError::Node(_),
                ..
            }
        ));
        assert!(outcome.node_error().is_none());
        let thread = ThreadId::new("pre-aegis-none").unwrap();
        let waypoints = store.saved_waypoints(&thread).await;
        let (_, failed_node, node_error) = failed_status(&waypoints[0]);
        assert_eq!(failed_node, &node_id);
        assert!(node_error.is_none());

        // (b) An engine-limit failure: a self-loop that never resolves trips
        //     `NodeVisitLimitExceeded`; it is not a node's own failure, so no
        //     structured error either -- even though the node HAS an Aegis.
        let status = FieldName::new("result").unwrap();
        let looping = CountingFunctionNode::new({
            let status = status.clone();
            move |_run, _state| {
                let mut d = StateDelta::new();
                d.set_raw(status.clone(), serde_json::json!("looping"));
                d
            }
        });
        let mut graph = WarGraph::new(
            one_field_schema(),
            EngineLimits {
                max_node_visits: 3,
                ..EngineLimits::default()
            },
        );
        let a = NodeId::new("a");
        graph.add_node(a.clone(), NodeSpec::Function(looping));
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: a.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(a.clone());
        graph.set_aegis(a.clone(), retrying_aegis(3));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("limit-none").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(
            &outcome,
            RunOutcome::Failed {
                error: EngineError::NodeVisitLimitExceeded { .. },
                ..
            }
        ));
        assert!(outcome.node_error().is_none());
        let latest = store
            .saved_waypoints(&thread)
            .await
            .into_iter()
            .next()
            .unwrap();
        let (_, failed_node, node_error) = failed_status(&latest);
        assert_eq!(failed_node, &a);
        assert!(node_error.is_none());
    }

    #[tokio::test]
    async fn run_outcome_failed_exposes_the_same_node_error() {
        let (_, outcome, store) =
            run_exhausting_node("outcome-same-node-error", Some(retrying_aegis(2))).await;
        let thread = ThreadId::new("outcome-same-node-error").unwrap();
        let waypoints = store.saved_waypoints(&thread).await;
        let (_, _, recorded) = failed_status(&waypoints[0]);
        let recorded = recorded.expect("the Waypoint records the NodeError");
        let exposed = outcome
            .node_error()
            .expect("RunOutcome::Failed exposes the NodeError");
        assert_eq!(exposed, recorded, "identical value on both surfaces");
        match &outcome {
            RunOutcome::Failed { error, .. } => {
                assert!(matches!(error, EngineError::NodeFailed(ne) if ne == recorded));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn engine_error_node_failed_maps_to_battalion_error_node() {
        let ne = NodeError {
            node_id: NodeId::new("n"),
            attempt: 3,
            transience: Transience::Permanent,
            source: NodeErrorSource::Function {
                message: "boom".to_string(),
            },
        };
        let mapped = BattalionError::from(EngineError::NodeFailed(ne.clone()));
        assert!(
            matches!(&mapped, BattalionError::Node(inner) if inner == &ne),
            "got {mapped:?}"
        );

        // Every other variant keeps a rendered, non-structured mapping.
        let other = BattalionError::from(EngineError::Node(StateNodeError("x".to_string())));
        assert!(matches!(other, BattalionError::CampaignError(msg) if msg.contains("x")));
    }

    async fn run_failing_paladin_node(
        thread: &str,
        port: Arc<FailingPaladinPort>,
    ) -> (NodeId, RunOutcome, Arc<RecordingWaypointStore>) {
        let field_name = FieldName::new("result").unwrap();
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("summarizer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                make_paladin("summarizer"),
                InputMapping::new("summarize"),
                field_name,
            ),
        );
        graph.add_entry(node_id.clone());
        // An Aegis with a single-attempt retry policy: policy-governed (so
        // the structured error is recorded) without any actual retry.
        graph.set_aegis(node_id.clone(), retrying_aegis(1));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(port, store.clone());
        let outcome = engine
            .start(&graph, ThreadId::new(thread).unwrap(), StateDelta::new())
            .await
            .unwrap();
        (node_id, outcome, store)
    }

    #[tokio::test]
    async fn a_paladin_node_failure_becomes_node_error_source_paladin() {
        // A non-LLM Paladin failure: `Paladin { kind: <variant name>, .. }`.
        let port = FailingPaladinPort::new(|| PaladinError::ExecutionError("paladin down".into()));
        let (node_id, outcome, _) = run_failing_paladin_node("paladin-source", port.clone()).await;
        assert_eq!(port.call_count(), 1);
        let ne = outcome.node_error().expect("structured error").clone();
        assert_eq!(ne.node_id, node_id);
        assert_eq!(
            ne.transience,
            PaladinError::ExecutionError(String::new()).transience()
        );
        match &ne.source {
            NodeErrorSource::Paladin {
                kind,
                status,
                provider,
                message,
            } => {
                assert_eq!(kind, "ExecutionError");
                assert_eq!(*status, None);
                assert_eq!(*provider, None);
                assert!(message.contains("paladin down"));
            }
            other => panic!("expected Paladin, got {other:?}"),
        }

        // An LLM failure underneath: `Llm { .. }` carrying the typed status
        // and provider -- never `Function`, and distinguishable from any
        // other Paladin failure.
        let port = FailingPaladinPort::new(|| PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: Some("openai".to_string()),
            message: "upstream unavailable".to_string(),
        });
        let (_, outcome, store) = run_failing_paladin_node("llm-source", port.clone()).await;
        assert_eq!(port.call_count(), 1, "max_attempts 1: no retry");
        let ne = outcome.node_error().expect("structured error").clone();
        assert_eq!(
            ne.transience,
            Transience::Transient,
            "read from the typed field"
        );
        match &ne.source {
            NodeErrorSource::Llm {
                status, provider, ..
            } => {
                assert_eq!(*status, Some(503));
                assert_eq!(provider.as_deref(), Some("openai"));
            }
            other => panic!("expected Llm, got {other:?}"),
        }
        let thread = ThreadId::new("llm-source").unwrap();
        let waypoints = store.saved_waypoints(&thread).await;
        let (line, _, recorded) = failed_status(&waypoints[0]);
        assert_eq!(recorded, Some(&ne));
        // The display line is the legacy `StateNodeError(e.to_string())` one.
        assert_eq!(
            line,
            EngineError::Node(StateNodeError(
                "LLM error: upstream unavailable".to_string()
            ))
            .to_string()
        );
    }

    // --- Phase 25 Plan 07, Task 3: per-task retry inside a Muster, and the
    //     Waypoint and Parley interactions (D-17, FT-FR-06, FT-FR-07) -----

    fn retrying_aegis_with_interval(max_attempts: u32, interval: std::time::Duration) -> Aegis {
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts,
                retry_on: RetryPredicate::TransientAndUnknown,
                jitter: false,
                initial_interval: interval,
                ..RetryPolicy::default()
            }),
            ..Default::default()
        }
    }

    /// `planner -> Muster(a..e) -> worker` over an `Append` field, the
    /// worker being `worker_node` with `worker_aegis` set on the template.
    fn muster_graph(
        results_field: &FieldName,
        worker_node: Arc<MusterFailThenSucceedWorker>,
        worker_aegis: Aegis,
    ) -> (WarGraph, NodeId) {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            results_field.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(
                    ["a", "b", "c", "d", "e"]
                        .iter()
                        .map(|k| MusterTask {
                            worker: worker.clone(),
                            payload: serde_json::json!(*k),
                            task_key: k.to_string(),
                        })
                        .collect(),
                ),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node));
        graph.set_aegis(worker.clone(), worker_aegis);
        graph.add_entry(planner);
        (graph, worker)
    }

    #[tokio::test]
    async fn one_mustered_task_retries_without_re_running_siblings() {
        let results = FieldName::new("results").unwrap();
        let worker_node = MusterFailThenSucceedWorker::new(results.clone(), [("c", 2)], None);
        let (graph, _worker) = muster_graph(&results, worker_node.clone(), retrying_aegis(3));
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let outcome = engine
            .start(
                &graph,
                ThreadId::new("muster-one-task-retries").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();

        for sibling in ["a", "b", "d", "e"] {
            assert_eq!(
                worker_node.run_count(sibling),
                1,
                "sibling {sibling} ran exactly once"
            );
        }
        assert_eq!(
            worker_node.run_count("c"),
            3,
            "the failing task ran three times"
        );
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let mut aggregated: Vec<String> = final_state.get(&results).unwrap().unwrap();
                aggregated.sort();
                assert_eq!(
                    aggregated,
                    vec!["a", "b", "c", "d", "e"],
                    "five results aggregated"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn sibling_tasks_do_not_wait_for_a_retrying_task_to_finish() {
        let results = FieldName::new("results").unwrap();
        let worker_node = MusterFailThenSucceedWorker::new(results.clone(), [("c", 2)], None);
        let interval = std::time::Duration::from_millis(500);
        let (graph, _worker) = muster_graph(
            &results,
            worker_node.clone(),
            retrying_aegis_with_interval(3, interval),
        );
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let outcome = engine
            .start(
                &graph,
                ThreadId::new("muster-siblings-do-not-wait").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let calls = worker_node.calls();
        let c_final = calls
            .iter()
            .filter(|c| c.task_key == "c")
            .map(|c| c.at)
            .max()
            .expect("c ran");
        let c_first = calls
            .iter()
            .filter(|c| c.task_key == "c")
            .map(|c| c.at)
            .min()
            .expect("c ran");
        assert!(
            c_final - c_first >= interval + interval * 2,
            "c's final attempt waited out the 500ms + 1000ms backoffs on the paused clock"
        );
        for call in calls.iter().filter(|c| c.task_key != "c") {
            assert!(
                call.at < c_final,
                "sibling {} ran at {:?}, before c's final attempt at {:?}",
                call.task_key,
                call.at,
                c_final
            );
            assert!(
                call.at - c_first < interval,
                "sibling {} did not wait for any of c's backoffs",
                call.task_key
            );
        }
    }

    #[tokio::test]
    async fn each_muster_task_has_its_own_attempt_counter() {
        let results = FieldName::new("results").unwrap();
        let worker_node =
            MusterFailThenSucceedWorker::new(results.clone(), [("b", 1), ("d", 1)], None);
        let (graph, worker) = muster_graph(&results, worker_node.clone(), retrying_aegis(3));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("muster-own-attempt-counters").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(worker_node.run_count("b"), 2);
        assert_eq!(worker_node.run_count("d"), 2);
        for other in ["a", "c", "e"] {
            assert_eq!(worker_node.run_count(other), 1);
        }

        // The muster superstep's consolidated Waypoint (no progress payload)
        // carries one record per task: exactly two at `attempt: 2`, each
        // with exactly one failed attempt in its history, and three at
        // `attempt: 1` -- neither retrying task observed the other's count.
        let waypoints = store.saved_waypoints(&thread).await;
        let consolidated = waypoints
            .iter()
            .find(|w| w.superstep == 2 && w.muster_progress.is_none())
            .expect("the muster superstep's consolidated Waypoint");
        let worker_records: Vec<_> = consolidated
            .completed
            .iter()
            .filter(|r| r.node_id == worker)
            .collect();
        assert_eq!(worker_records.len(), 5);
        let mut attempts: Vec<u32> = worker_records.iter().map(|r| r.attempt).collect();
        attempts.sort_unstable();
        assert_eq!(attempts, vec![1, 1, 1, 2, 2]);
        for record in &worker_records {
            assert_eq!(record.attempts.len(), (record.attempt - 1) as usize);
        }
    }

    #[tokio::test]
    async fn no_waypoint_is_written_between_attempts() {
        let node_id = NodeId::new("flaky-observer");
        let store = Arc::new(RecordingWaypointStore::new());
        let node =
            AttemptObservingNode::new(3, FieldName::new("result").unwrap(), store.clone(), None);
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let outcome = engine
            .start(
                &graph,
                ThreadId::new("no-waypoint-between-attempts").unwrap(),
                StateDelta::new(),
            )
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 3);
        let seen = node.saves_seen();
        assert_eq!(seen.len(), 3);
        assert_eq!(
            seen[1] - seen[0],
            0,
            "zero saves between attempt 1's failure and attempt 2's start"
        );
        assert_eq!(seen[2] - seen[1], 0, "and between attempts 2 and 3");
        assert_eq!(
            store.save_call_count(),
            1,
            "the superstep's one Waypoint, after the loop"
        );
    }

    #[tokio::test]
    async fn muster_progress_waypoints_record_only_completed_tasks() {
        let results = FieldName::new("results").unwrap();
        let store = Arc::new(RecordingWaypointStore::new());
        let worker_node =
            MusterFailThenSucceedWorker::new(results.clone(), [("c", 2)], Some(store.clone()));
        let (graph, _worker) = muster_graph(&results, worker_node.clone(), retrying_aegis(3));
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("muster-progress-only-completed").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        // (1) No progress Waypoint was written while `c` was mid-retry: every
        //     one of c's attempts observed the same save count, so nothing
        //     was persisted between its failures and its success.
        let c_saves: Vec<usize> = worker_node
            .calls()
            .iter()
            .filter(|call| call.task_key == "c")
            .map(|call| call.saves_seen)
            .collect();
        assert_eq!(c_saves.len(), 3);
        assert!(c_saves.iter().all(|&n| n == c_saves[0]), "{c_saves:?}");

        // (2) Every progress Waypoint's `completed` map lists only tasks
        //     that had already succeeded, growing in task_key order; `c`
        //     first appears in the third one (after a and b), never before
        //     its succeeding attempt, and the payload shape is Phase 23's.
        let mut progress: Vec<&Waypoint> = Vec::new();
        let waypoints = store.saved_waypoints(&thread).await;
        for w in waypoints.iter().rev() {
            if w.muster_progress.is_some() {
                progress.push(w);
            }
        }
        assert_eq!(
            progress.len(),
            5,
            "one progress Waypoint per completed task"
        );
        for (index, w) in progress.iter().enumerate() {
            let p = w.muster_progress.as_ref().unwrap();
            assert_eq!(p.tasks.len(), 5);
            let keys: Vec<&String> = p.completed.keys().collect();
            let expected: Vec<String> = ["a", "b", "c", "d", "e"][..=index]
                .iter()
                .map(|k| k.to_string())
                .collect();
            assert_eq!(keys, expected.iter().collect::<Vec<_>>());
            assert_eq!(p.completed.contains_key("c"), index >= 2);
        }
    }

    #[tokio::test]
    async fn resume_re_executes_an_interrupted_node_from_attempt_one() {
        let node_id = NodeId::new("interrupted");
        let store = Arc::new(RecordingWaypointStore::new());
        let token = CancellationToken::new();
        // Fails on run 1 (cancelling the run from inside that failure, so
        // the backoff wait observes the cancellation and the run halts
        // mid-retry), succeeds on run 2 -- which must be attempt 1 of the
        // resumed run.
        let node = AttemptObservingNode::new(
            2,
            FieldName::new("result").unwrap(),
            store.clone(),
            Some(token.clone()),
        );
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_cancellation_token(token);
        let thread = ThreadId::new("resume-from-attempt-one").unwrap();
        let halted = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            engine.start(&graph, thread.clone(), StateDelta::new()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            matches!(halted, RunOutcome::Halted { .. }),
            "got {halted:?}"
        );
        assert_eq!(
            node.run_count(),
            1,
            "attempt 1 ran; the interrupted backoff never retried"
        );
        let latest = store
            .saved_waypoints(&thread)
            .await
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(latest.status, WaypointStatus::Halted);
        assert!(
            latest.vanguard.contains(&node_id),
            "the interrupted node is re-listed on the Halted vanguard so resume re-runs it"
        );

        let resume_engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let resumed = resume_engine.resume(&graph, thread.clone()).await.unwrap();
        assert!(
            matches!(resumed, RunOutcome::Completed { .. }),
            "got {resumed:?}"
        );
        assert_eq!(node.run_count(), 2);
        let latest = store
            .saved_waypoints(&thread)
            .await
            .into_iter()
            .next()
            .unwrap();
        let record = latest
            .completed
            .iter()
            .find(|r| r.node_id == node_id)
            .expect("the resumed run recorded the node");
        assert_eq!(
            record.attempt, 1,
            "a resume re-executes from attempt 1 (FT-FR-07)"
        );
        assert!(record.attempts.is_empty());
        assert_eq!(record.outcome, NodeOutcomeKind::Succeeded);
    }

    #[tokio::test]
    async fn a_parley_directive_is_never_retried() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = {
            let node_id = node_id.clone();
            CountingFunctionNode::with_context_directive(move |_run, _state, _ctx| Directive {
                delta: StateDelta::new(),
                next: NextStep::Parley(sample_parley_request(node_id.clone(), parley_id)),
            })
        };
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("parley-never-retried").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(
            matches!(outcome, RunOutcome::AwaitingInput { .. }),
            "got {outcome:?}"
        );
        assert_eq!(
            node.run_count(),
            1,
            "a Parley is a success: exactly one execution"
        );
        let latest = store
            .saved_waypoints(&thread)
            .await
            .into_iter()
            .next()
            .unwrap();
        let record = &latest.completed[0];
        assert_eq!(record.outcome, NodeOutcomeKind::Parleyed);
        assert_eq!(record.attempt, 1);
        assert!(record.attempts.is_empty(), "no retry budget consumed");
    }

    #[tokio::test]
    async fn post_resume_rerun_of_a_parleying_node_starts_at_attempt_one() {
        let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
        let node_id = NodeId::new("asker");
        let parley_id = ParleyId::new();
        let node = {
            let node_id = node_id.clone();
            CountingFunctionNode::with_context_directive(move |run, _state, ctx| {
                if run == 0 {
                    Directive {
                        delta: StateDelta::new(),
                        next: NextStep::Parley(sample_parley_request(node_id.clone(), parley_id)),
                    }
                } else {
                    let value = ctx
                        .parley_response()
                        .expect("parley_response is set on the post-resume re-run")
                        .value
                        .clone();
                    let mut delta = StateDelta::new();
                    delta.set_raw(FieldName::new("result").unwrap(), value);
                    delta.into()
                }
            })
        };
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id.clone(), retrying_aegis(3));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone());
        let thread = ThreadId::new("parley-rerun-attempt-one").unwrap();
        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(suspended, RunOutcome::AwaitingInput { .. }));

        let response = ParleyResponse {
            parley_id,
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value: serde_json::json!("approve"),
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let resumed = engine
            .resume_with(&graph, thread.clone(), vec![response])
            .await
            .unwrap();
        assert!(
            matches!(resumed, RunOutcome::Completed { .. }),
            "got {resumed:?}"
        );
        assert_eq!(node.run_count(), 2);
        let latest = store
            .saved_waypoints(&thread)
            .await
            .into_iter()
            .next()
            .unwrap();
        let record = latest
            .completed
            .iter()
            .find(|r| r.node_id == node_id)
            .expect("the re-run is recorded");
        assert_eq!(
            record.attempt, 1,
            "the post-resume re-run is a fresh attempt 1"
        );
        assert!(record.attempts.is_empty());
        assert_eq!(record.outcome, NodeOutcomeKind::Succeeded);
    }

    // --- Phase 25 Plan 13, Task 1: the fail-closed cache validation
    //     clauses (FT-FR-18, FT-FR-20, D-29) ---------------------------------
    mod node_cache_validation_tests {
        use super::*;
        use crate::engine::graph::StateMap;
        use crate::engine::test_support::RecordingNodeCache;
        use paladin_core::platform::container::aegis::{CacheKeySpec, CachePolicy};
        use paladin_core::platform::container::battlefield::CacheMarker;
        use std::time::Duration;

        fn cache_aegis() -> Aegis {
            Aegis {
                cache: Some(CachePolicy {
                    ttl: Duration::from_secs(60),
                    key: CacheKeySpec::Default,
                }),
                ..Aegis::default()
            }
        }

        fn engine_with_cache(
            port: Arc<dyn PaladinPort>,
        ) -> (WarEngine<InMemoryWaypointStore>, Arc<RecordingNodeCache>) {
            let cache = RecordingNodeCache::new();
            let engine = WarEngine::new(port, Arc::new(InMemoryWaypointStore::new()))
                .with_node_cache(cache.clone());
            (engine, cache)
        }

        fn cached_function_graph(names: &[&str]) -> WarGraph {
            let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
            for name in names {
                let id = NodeId::new(*name);
                graph.add_node(
                    id.clone(),
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        FieldName::new("result").unwrap(),
                        serde_json::json!("v"),
                    )),
                );
                graph.add_entry(id.clone());
                graph.set_aegis(id, cache_aegis());
            }
            graph
        }

        /// Test 2: a `CachePolicy` on a node, run through a `WarEngine` with
        /// no `with_node_cache`, fails validation with a typed error naming
        /// the node -- never a silent no-op.
        #[tokio::test]
        async fn a_cache_policy_without_an_engine_cache_fails_validation() {
            let graph = cached_function_graph(&["cached"]);
            let err = engine()
                .start(
                    &graph,
                    ThreadId::new("no-backend").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap_err();
            match err {
                EngineError::CachePolicyWithoutCacheBackend { nodes, reason } => {
                    assert_eq!(nodes, vec![NodeId::new("cached")]);
                    assert!(reason.contains("cached"));
                    assert!(reason.contains("with_node_cache"));
                }
                other => panic!("expected CachePolicyWithoutCacheBackend, got {other:?}"),
            }
        }

        /// The same graph WITH a backend validates and runs -- the clause is
        /// about the backend's absence, not the policy's presence.
        #[tokio::test]
        async fn a_cache_policy_with_an_engine_cache_validates() {
            let graph = cached_function_graph(&["cached"]);
            let (engine, _cache) = engine_with_cache(Arc::new(UnimplementedPaladinPort));
            let outcome = engine
                .start(
                    &graph,
                    ThreadId::new("with-backend").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::Completed { .. }));
        }

        /// Test 3: a Paladin node whose `output_field` is marked
        /// `CacheMarker::Deny` cannot carry a `CachePolicy`; the typed error
        /// names the field and the node.
        #[tokio::test]
        async fn a_cache_policy_on_a_deny_output_field_fails_validation() {
            let summary = FieldName::new("summary").unwrap();
            let schema = BattlefieldSchema::new(vec![
                FieldSpec::new(summary.clone(), DispatchRule::LastWrite, None, false)
                    .with_cache(CacheMarker::Deny),
            ]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let node_id = NodeId::new("summarise");
            graph.add_node(
                node_id.clone(),
                NodeSpec::paladin(make_paladin("p"), InputMapping::new("go"), summary),
            );
            graph.add_entry(node_id.clone());
            graph.set_aegis(node_id, cache_aegis());

            let (engine, _cache) = engine_with_cache(Arc::new(RecordingPaladinPort::new()));
            let err = engine
                .start(
                    &graph,
                    ThreadId::new("deny-field").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap_err();
            match err {
                EngineError::CachePolicyOnDeniedField { offenders, reason } => {
                    assert_eq!(offenders.len(), 1);
                    assert!(offenders[0].contains("summarise"), "{offenders:?}");
                    assert!(offenders[0].contains("summary"), "{offenders:?}");
                    assert!(reason.contains("Deny"));
                }
                other => panic!("expected CachePolicyOnDeniedField, got {other:?}"),
            }
        }

        /// Test 4: three offending nodes produce ONE error naming all three.
        #[tokio::test]
        async fn validation_lists_every_cache_offender() {
            let graph = cached_function_graph(&["charlie", "alpha", "bravo"]);
            let err = engine()
                .start(
                    &graph,
                    ThreadId::new("three-offenders").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap_err();
            match err {
                EngineError::CachePolicyWithoutCacheBackend { nodes, reason } => {
                    assert_eq!(
                        nodes,
                        vec![
                            NodeId::new("alpha"),
                            NodeId::new("bravo"),
                            NodeId::new("charlie")
                        ],
                        "every offender, sorted"
                    );
                    for name in ["alpha", "bravo", "charlie"] {
                        assert!(reason.contains(name), "{reason}");
                    }
                }
                other => panic!("expected CachePolicyWithoutCacheBackend, got {other:?}"),
            }
        }

        /// Test 6: the marker is opt-in denial, not automatic -- an `Append`
        /// field left at `Allow` validates (the replay hazard is
        /// documentation, FT-FR-20).
        #[tokio::test]
        async fn an_append_dispatch_field_may_still_be_cached_when_marked_allow() {
            let log = FieldName::new("log").unwrap();
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                log.clone(),
                DispatchRule::Append,
                None,
                false,
            )]);
            assert_eq!(schema.fields[0].cache, CacheMarker::Allow);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let node_id = NodeId::new("append-writer");
            graph.add_node(
                node_id.clone(),
                NodeSpec::paladin(make_paladin("p"), InputMapping::new("go"), log.clone()),
            );
            graph.add_entry(node_id.clone());
            graph.set_aegis(node_id, cache_aegis());

            let port = Arc::new(RecordingPaladinPort::new());
            port.set_output("p", "entry");
            let (engine, cache) = engine_with_cache(port);
            let outcome = engine
                .start(
                    &graph,
                    ThreadId::new("append-allow").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap();
            match outcome {
                RunOutcome::Completed { final_state, .. } => {
                    assert_eq!(
                        final_state.get::<Vec<String>>(&log).unwrap(),
                        Some(vec!["entry".to_string()])
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
            assert_eq!(cache.put_count(), 1, "an Allow field IS cached");
        }

        /// A `CachePolicy` inside a `NodeSpec::Battalion` child graph is just
        /// as fail-closed as one on the parent: the child inherits the
        /// engine's cache wholesale, so it inherits the absence too.
        #[tokio::test]
        async fn a_cache_policy_inside_a_battalion_child_fails_without_a_backend() {
            let child = Arc::new(cached_function_graph(&["inner"]));
            let mut parent = WarGraph::new(one_field_schema(), EngineLimits::default());
            let battalion = NodeId::new("outer");
            parent.add_node(
                battalion.clone(),
                NodeSpec::battalion(child, StateMap::default()),
            );
            parent.add_entry(battalion);
            let err = engine()
                .start(
                    &parent,
                    ThreadId::new("child-no-backend").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap_err();
            match err {
                EngineError::CachePolicyWithoutCacheBackend { nodes, .. } => {
                    assert_eq!(nodes, vec![NodeId::new("outer/inner")]);
                }
                other => panic!("expected CachePolicyWithoutCacheBackend, got {other:?}"),
            }
        }

        /// A `CacheKeySpec::Fields` naming a field the schema does not
        /// declare is rejected before any node runs -- a typo must never
        /// silently narrow the key.
        #[tokio::test]
        async fn a_cache_key_spec_naming_an_undeclared_field_fails_validation() {
            let mut graph = cached_function_graph(&["cached"]);
            graph.set_aegis(
                NodeId::new("cached"),
                Aegis {
                    cache: Some(CachePolicy {
                        ttl: Duration::from_secs(60),
                        key: CacheKeySpec::Fields(vec![FieldName::new("nope").unwrap()]),
                    }),
                    ..Aegis::default()
                },
            );
            let (engine, _cache) = engine_with_cache(Arc::new(UnimplementedPaladinPort));
            let err = engine
                .start(
                    &graph,
                    ThreadId::new("undeclared-key-field").unwrap(),
                    StateDelta::new(),
                )
                .await
                .unwrap_err();
            match err {
                EngineError::CacheKeyFieldUndeclared { offenders, .. } => {
                    assert_eq!(offenders.len(), 1);
                    assert!(offenders[0].contains("cached") && offenders[0].contains("nope"));
                }
                other => panic!("expected CacheKeyFieldUndeclared, got {other:?}"),
            }
        }
    }

    /// Plan 26-13, Doc 05 RT-04, D-21: `WarEngine::with_vault` grants a base
    /// namespace to every node of every run on the engine, `NodeContext::
    /// vault()` is the read/write handle a `StateNode` uses, an engine
    /// without `with_vault` gives nodes no handle at all, and a
    /// `NodeSpec::Paladin` node receives the same grant through the
    /// defaulted `execute_scoped` dispatch.
    mod vault_tests {
        use super::*;
        use std::collections::HashMap;
        use std::sync::Mutex as StdMutex;

        use paladin_core::platform::container::run_scope::RunScope;
        use paladin_core::platform::container::vault::Namespace;
        use paladin_ports::output::vault_port::{Page, VaultError, VaultPort, VaultRecord};

        /// A minimal, real `VaultPort` -- exact-namespace storage, no
        /// descendant leakage -- used only by this module's tests. Not
        /// `paladin_memory::vault::InMemoryVault` (plan 26-04), to avoid
        /// adding a `paladin-memory` dev-dependency to `paladin-battalion`
        /// for a handful of tests; `search` keeps the trait's correct
        /// `Unsupported` default, unexercised here.
        #[derive(Default)]
        struct TestVault {
            data: StdMutex<HashMap<String, HashMap<String, serde_json::Value>>>,
        }

        impl TestVault {
            /// Test-only escape hatch bypassing every `ConfinedVault` --
            /// reads the raw backend directly, the way
            /// `concurrent_confined_writes_produce_zero_cross_namespace_records`
            /// proves no write ever landed under a foreign namespace.
            fn raw_records(&self, ns: &Namespace) -> HashMap<String, serde_json::Value> {
                self.data
                    .lock()
                    .unwrap()
                    .get(&ns.to_string())
                    .cloned()
                    .unwrap_or_default()
            }
        }

        #[async_trait]
        impl VaultPort for TestVault {
            async fn put(
                &self,
                ns: &Namespace,
                key: &str,
                value: serde_json::Value,
            ) -> Result<(), VaultError> {
                self.data
                    .lock()
                    .unwrap()
                    .entry(ns.to_string())
                    .or_default()
                    .insert(key.to_string(), value);
                Ok(())
            }

            async fn get(
                &self,
                ns: &Namespace,
                key: &str,
            ) -> Result<Option<VaultRecord>, VaultError> {
                let value = self
                    .data
                    .lock()
                    .unwrap()
                    .get(&ns.to_string())
                    .and_then(|m| m.get(key).cloned());
                match value {
                    Some(v) => Ok(Some(VaultRecord::new(ns.clone(), key, v)?)),
                    None => Ok(None),
                }
            }

            async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
                Ok(self
                    .data
                    .lock()
                    .unwrap()
                    .get_mut(&ns.to_string())
                    .map(|m| m.remove(key).is_some())
                    .unwrap_or(false))
            }

            async fn list(
                &self,
                ns: &Namespace,
                _prefix: Option<&str>,
                _page: Page,
            ) -> Result<Vec<VaultRecord>, VaultError> {
                let data = self.data.lock().unwrap();
                Ok(data
                    .get(&ns.to_string())
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| {
                                VaultRecord::new(ns.clone(), k.clone(), v.clone()).unwrap()
                            })
                            .collect()
                    })
                    .unwrap_or_default())
            }

            // `search` keeps the trait's correct `Unsupported` default.
        }

        fn one_field_schema() -> BattlefieldSchema {
            BattlefieldSchema::new(vec![FieldSpec::new(
                FieldName::new("result").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            )])
        }

        /// A `StateNode` that reads `ctx.vault()` and writes what it
        /// observed into the `result` field: the granted namespace's
        /// `Display` string if `Some`, or the literal `"none"` if `None` --
        /// so a test can assert on `final_state` alone, never on internal
        /// engine state.
        struct VaultProbeNode;

        #[async_trait]
        impl StateNode for VaultProbeNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &NodeContext,
            ) -> Result<Directive, StateNodeError> {
                let mut delta = StateDelta::new();
                let observed = match ctx.vault() {
                    Some(confined) => confined.granted().to_string(),
                    None => "none".to_string(),
                };
                delta
                    .set(FieldName::new("result").unwrap(), observed)
                    .unwrap();
                Ok(delta.into())
            }
        }

        /// A `StateNode` that attempts a write WITHIN its grant (expected
        /// to succeed) and a write to an unrelated namespace (expected to
        /// be denied), recording both outcomes into the `result` field as
        /// `"within_ok=<bool>,outside_denied=<bool>"`.
        struct GrantBoundaryNode;

        #[async_trait]
        impl StateNode for GrantBoundaryNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &NodeContext,
            ) -> Result<Directive, StateNodeError> {
                let confined = ctx
                    .vault()
                    .expect("this test always wires a Vault via with_vault");

                let within = confined.granted().clone();
                let within_ok = confined
                    .put(&within, "n", serde_json::json!(1))
                    .await
                    .is_ok();

                let outside = Namespace::parse("other").unwrap();
                let outside_denied = matches!(
                    confined.put(&outside, "n", serde_json::json!(1)).await,
                    Err(VaultError::NamespaceDenied { .. })
                );

                let mut delta = StateDelta::new();
                delta
                    .set(
                        FieldName::new("result").unwrap(),
                        format!("within_ok={within_ok},outside_denied={outside_denied}"),
                    )
                    .unwrap();
                Ok(delta.into())
            }
        }

        fn probe_graph(node: Arc<dyn StateNode>) -> WarGraph {
            let mut graph = WarGraph::new(one_field_schema(), EngineLimits::default());
            let node_id = NodeId::new("probe");
            graph.add_node(node_id.clone(), NodeSpec::Function(node));
            graph.add_entry(node_id);
            graph
        }

        /// Test 1: `engine_grants_the_base_namespace_to_every_node` -- a
        /// `WarEngine` built with `with_vault(vault, ["app"])` gives every
        /// node's `NodeContext::vault()` a handle granted `["app"]`.
        #[tokio::test]
        async fn engine_grants_the_base_namespace_to_every_node() {
            let vault: Arc<dyn VaultPort> = Arc::new(TestVault::default());
            let base = Namespace::parse("app").unwrap();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_vault(vault, base.clone());

            let graph = probe_graph(Arc::new(VaultProbeNode));
            let thread = ThreadId::new("engine-grants-base").unwrap();
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
                        Some(base.to_string())
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
        }

        /// Test 2: `a_state_node_can_read_and_write_within_its_grant` -- a
        /// `StateNode` writing through `ctx.vault()` under its own grant
        /// succeeds, and a write to an unrelated namespace is denied.
        #[tokio::test]
        async fn a_state_node_can_read_and_write_within_its_grant() {
            let vault: Arc<dyn VaultPort> = Arc::new(TestVault::default());
            let base = Namespace::parse("app/counters").unwrap();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_vault(vault, base);

            let graph = probe_graph(Arc::new(GrantBoundaryNode));
            let thread = ThreadId::new("grant-boundary").unwrap();
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
                        Some("within_ok=true,outside_denied=true".to_string())
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
        }

        /// A `PaladinPort` test double that records the `RunScope` it
        /// receives through `execute_scoped`, so
        /// `a_paladin_node_receives_the_same_grant_through_execute_scoped`
        /// can assert on it directly.
        #[derive(Default)]
        struct ScopeRecordingPaladinPort {
            observed: StdMutex<Option<RunScope>>,
        }

        impl ScopeRecordingPaladinPort {
            fn observed_vault_namespace(&self) -> Option<Namespace> {
                self.observed
                    .lock()
                    .unwrap()
                    .as_ref()
                    .and_then(|scope| scope.vault_namespace.clone())
            }
        }

        #[async_trait]
        impl PaladinPort for ScopeRecordingPaladinPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Ok(PaladinResult {
                    output: "ok".to_string(),
                    ..Default::default()
                })
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

            async fn execute_scoped(
                &self,
                paladin: &Paladin,
                input: &str,
                _heartbeat: &HeartbeatHandle,
                scope: &RunScope,
            ) -> Result<PaladinResult, PaladinError> {
                *self.observed.lock().unwrap() = Some(scope.clone());
                self.execute(paladin, input).await
            }
        }

        /// Test 3: `a_paladin_node_receives_the_same_grant_through_execute_scoped`
        /// -- the Paladin arm calls `execute_scoped` with a `RunScope`
        /// carrying the engine's base namespace.
        #[tokio::test]
        async fn a_paladin_node_receives_the_same_grant_through_execute_scoped() {
            let vault: Arc<dyn VaultPort> = Arc::new(TestVault::default());
            let base = Namespace::parse("app").unwrap();
            let port = Arc::new(ScopeRecordingPaladinPort::default());
            let engine = WarEngine::new(port.clone(), Arc::new(InMemoryWaypointStore::new()))
                .with_vault(vault, base.clone());

            let field_name = FieldName::new("summary").unwrap();
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                field_name.clone(),
                DispatchRule::LastWrite,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let node_id = NodeId::new("p");
            graph.add_node(
                node_id.clone(),
                NodeSpec::paladin(make_paladin("p"), InputMapping::new("go"), field_name),
            );
            graph.add_entry(node_id);

            let thread = ThreadId::new("scope-through-execute-scoped").unwrap();
            engine
                .start(&graph, thread, StateDelta::new())
                .await
                .unwrap();

            assert_eq!(port.observed_vault_namespace(), Some(base));
        }

        /// Test 4: `an_engine_without_with_vault_gives_nodes_no_vault` --
        /// `NodeContext::vault()` is `None` and a node that tries to use it
        /// gets `None`, not a root-granted handle.
        #[tokio::test]
        async fn an_engine_without_with_vault_gives_nodes_no_vault() {
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            );

            let graph = probe_graph(Arc::new(VaultProbeNode));
            let thread = ThreadId::new("no-vault-wired").unwrap();
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
                        Some("none".to_string())
                    );
                }
                other => panic!("expected Completed, got {other:?}"),
            }
        }

        /// A `StateNode` that writes `count` records under exactly its own
        /// grant, each keyed uniquely and valued with `run_index` -- so a
        /// concurrent run of N of these, sharing one backend but each under
        /// its own `WarEngine::with_vault` grant, can be checked for
        /// cross-namespace leakage afterward.
        struct ConcurrentVaultWriterNode {
            run_index: usize,
            count: usize,
        }

        #[async_trait]
        impl StateNode for ConcurrentVaultWriterNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &NodeContext,
            ) -> Result<Directive, StateNodeError> {
                let confined = ctx
                    .vault()
                    .expect("this test always wires a Vault via with_vault");
                let ns = confined.granted().clone();
                for i in 0..self.count {
                    confined
                        .put(&ns, &format!("k{i}"), serde_json::json!(self.run_index))
                        .await
                        .map_err(|e| StateNodeError(e.to_string()))?;
                }
                Ok(StateDelta::new().into())
            }
        }

        /// Test 6 (D-39, X-05): N concurrent runs under distinct grants,
        /// sharing one backend, each writing M records; afterward every
        /// namespace holds exactly M records and every record's value is
        /// that namespace's own run index -- proving zero cross-namespace
        /// records under real concurrency, not merely under the
        /// single-threaded assertions Task 1's `ConfinedVault` tests cover.
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn concurrent_confined_writes_produce_zero_cross_namespace_records() {
            const RUNS: usize = 5;
            const RECORDS_PER_RUN: usize = 20;

            let shared_vault = Arc::new(TestVault::default());
            let mut handles = Vec::new();

            for run_index in 0..RUNS {
                let vault: Arc<dyn VaultPort> = shared_vault.clone();
                let ns = Namespace::parse(&format!("run{run_index}")).unwrap();
                let engine = WarEngine::new(
                    Arc::new(UnimplementedPaladinPort),
                    Arc::new(InMemoryWaypointStore::new()),
                )
                .with_vault(vault, ns.clone());

                let graph = probe_graph(Arc::new(ConcurrentVaultWriterNode {
                    run_index,
                    count: RECORDS_PER_RUN,
                }));
                let thread = ThreadId::new(format!("concurrent-vault-{run_index}")).unwrap();

                handles.push(tokio::spawn(async move {
                    engine.start(&graph, thread, StateDelta::new()).await
                }));
            }

            let results = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                futures::future::join_all(handles),
            )
            .await
            .expect("all runs must finish within the timeout guard");

            for result in results {
                let outcome = result
                    .expect("task must not panic")
                    .expect("run must succeed");
                assert!(matches!(outcome, RunOutcome::Completed { .. }));
            }

            for run_index in 0..RUNS {
                let ns = Namespace::parse(&format!("run{run_index}")).unwrap();
                let records = shared_vault.raw_records(&ns);
                assert_eq!(
                    records.len(),
                    RECORDS_PER_RUN,
                    "namespace run{run_index} must hold exactly {RECORDS_PER_RUN} records"
                );
                for value in records.values() {
                    assert_eq!(
                        value,
                        &serde_json::json!(run_index),
                        "every record under run{run_index} must have been written by run {run_index}, never another run"
                    );
                }
            }
        }
    }

    // --- Plan 28-03: populated `RunFinished`, richer `NodeStarted`/
    //     `NodeFinished`, the `DeltaMerged`-value opt-in, and
    //     `WarEngine::trace_emitter()` (D-02, D-03, D-04, D-05) -----------

    /// A [`StateNode`] that always raises a `NextStep::Parley` -- the
    /// minimal shape `run_finished_reports_failed_and_halted_and_awaiting_input`'s
    /// `AwaitingInput` table row needs.
    struct ParleyingNode;

    #[async_trait]
    impl StateNode for ParleyingNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, StateNodeError> {
            Ok(Directive {
                delta: StateDelta::new(),
                next: NextStep::Parley(ParleyRequest {
                    parley_id: ParleyId::new(),
                    node_id: NodeId::new(""),
                    kind: ParleyKind::Approval,
                    prompt: "need input".to_string(),
                    payload: serde_json::json!({}),
                    choices: None,
                    expires_at: None,
                    created_at: Utc::now(),
                    on_expire: OnExpire::FailRun,
                }),
            })
        }
    }

    /// The first `TraceEvent::RunFinished` record in `sink`'s events so
    /// far, or panics -- every table row below asserts exactly one exists.
    async fn run_finished_status_of(sink: &Arc<RecordingTraceSink>) -> RunFinishStatus {
        sink.events()
            .await
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::RunFinished { status, .. } => Some(*status),
                _ => None,
            })
            .expect("a RunFinished record must exist")
    }

    /// D-02, D-04: a two-superstep successful run's `RunFinished` reports
    /// `status: Completed`, `total_supersteps: 2`, `total_tokens` the sum
    /// of both nodes' `NodeFinished.token_count`, and `duration_ms > 0`.
    #[tokio::test]
    async fn run_finished_reports_completed_with_totals() {
        /// A minimal in-test `PaladinPort` reporting a caller-configured
        /// token count per Paladin name, and sleeping 1ms per call so this
        /// test's `duration_ms > 0` assertion is never a coin flip on fast
        /// hardware.
        struct TokenPaladinPort {
            tokens: std::collections::HashMap<&'static str, u32>,
        }
        #[async_trait]
        impl PaladinPort for TokenPaladinPort {
            async fn execute(
                &self,
                paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                let tokens = self
                    .tokens
                    .get(paladin.node.name.as_str())
                    .copied()
                    .unwrap_or(0);
                Ok(PaladinResult {
                    output: "ok".to_string(),
                    token_count: tokens,
                    ..Default::default()
                })
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

        let out = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let n1 = NodeId::new("first");
        let n2 = NodeId::new("second");
        graph.add_node(
            n1.clone(),
            NodeSpec::paladin(make_paladin("first"), InputMapping::new("go"), out.clone()),
        );
        graph.add_node(
            n2.clone(),
            NodeSpec::paladin(make_paladin("second"), InputMapping::new("go"), out.clone()),
        );
        graph.add_edge(EdgeSpec {
            from: n1.clone(),
            to: n2.clone(),
            condition: None,
        });
        graph.add_entry(n1);

        let mut tokens = std::collections::HashMap::new();
        tokens.insert("first", 5u32);
        tokens.insert("second", 7u32);
        let port: Arc<dyn PaladinPort> = Arc::new(TokenPaladinPort { tokens });
        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(port, Arc::new(InMemoryWaypointStore::new()))
            .with_trace_sink(sink.clone());

        let thread = ThreadId::new("run-finished-totals").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let run_finished = sink
            .events()
            .await
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::RunFinished {
                    status,
                    total_supersteps,
                    total_tokens,
                    duration_ms,
                    trace_dropped_total,
                } => Some((
                    *status,
                    *total_supersteps,
                    *total_tokens,
                    *duration_ms,
                    *trace_dropped_total,
                )),
                _ => None,
            })
            .expect("a RunFinished record must exist");
        assert_eq!(run_finished.0, RunFinishStatus::Completed);
        assert_eq!(run_finished.1, 2, "two supersteps: n1 then n2");
        assert_eq!(
            run_finished.2, 12,
            "5 + 7 tokens across both NodeFinished records"
        );
        assert!(
            run_finished.3 > 0,
            "duration_ms must be > 0: {}",
            run_finished.3
        );
        assert_eq!(run_finished.4, 0);
    }

    /// D-02, D-04: the other three `RunOutcome` shapes (`Failed`, `Halted`,
    /// `AwaitingInput`) each map onto the matching `RunFinishStatus` -- a
    /// table test over four independent runs, `Completed` included for
    /// completeness (closes 27-CONTEXT D-25's correction, T-28-03-04).
    #[tokio::test(flavor = "multi_thread")]
    async fn run_finished_reports_failed_and_halted_and_awaiting_input() {
        // Completed.
        {
            let out = FieldName::new("out").unwrap();
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                out.clone(),
                DispatchRule::LastWrite,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let id = NodeId::new("solo");
            graph.add_node(
                id.clone(),
                NodeSpec::Function(CountingFunctionNode::fixed(out, serde_json::json!("v"))),
            );
            graph.add_entry(id);
            let sink = RecordingTraceSink::new();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_trace_sink(sink.clone());
            let thread = ThreadId::new("table-completed").unwrap();
            let outcome = engine
                .start(&graph, thread, StateDelta::new())
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::Completed { .. }));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            assert_eq!(
                run_finished_status_of(&sink).await,
                RunFinishStatus::Completed
            );
        }

        // Failed.
        {
            let out = FieldName::new("out").unwrap();
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                out,
                DispatchRule::LastWrite,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let id = NodeId::new("failer");
            graph.add_node(
                id.clone(),
                NodeSpec::Function(FailingFunctionNode::new("nope")),
            );
            graph.add_entry(id);
            let sink = RecordingTraceSink::new();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_trace_sink(sink.clone());
            let thread = ThreadId::new("table-failed").unwrap();
            let outcome = engine
                .start(&graph, thread, StateDelta::new())
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::Failed { .. }));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            assert_eq!(run_finished_status_of(&sink).await, RunFinishStatus::Failed);
        }

        // Halted -- a token cancelled BEFORE the first superstep boundary,
        // mirroring `cancellation_before_first_superstep_still_yields_a_halted_waypoint`.
        {
            let (graph, _ids) = four_node_chain_graph();
            let token = CancellationToken::new();
            token.cancel();
            let sink = RecordingTraceSink::new();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_cancellation_token(token)
            .with_trace_sink(sink.clone());
            let thread = ThreadId::new("table-halted").unwrap();
            let outcome = engine
                .start(&graph, thread, StateDelta::new())
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::Halted { .. }));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            assert_eq!(run_finished_status_of(&sink).await, RunFinishStatus::Halted);
        }

        // AwaitingInput.
        {
            let out = FieldName::new("out").unwrap();
            let schema = BattlefieldSchema::new(vec![FieldSpec::new(
                out,
                DispatchRule::LastWrite,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(schema, EngineLimits::default());
            let id = NodeId::new("asker");
            graph.add_node(id.clone(), NodeSpec::Function(Arc::new(ParleyingNode)));
            graph.add_entry(id);
            let sink = RecordingTraceSink::new();
            let engine = WarEngine::new(
                Arc::new(UnimplementedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            )
            .with_trace_sink(sink.clone());
            let thread = ThreadId::new("table-awaiting").unwrap();
            let outcome = engine
                .start(&graph, thread, StateDelta::new())
                .await
                .unwrap();
            assert!(matches!(outcome, RunOutcome::AwaitingInput { .. }));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            assert_eq!(
                run_finished_status_of(&sink).await,
                RunFinishStatus::AwaitingInput
            );
        }
    }

    /// D-02: a retried node's two `NodeFinished` records carry `attempt` 1
    /// (failed) and 2 (succeeded), each with its own `duration_ms`/
    /// `token_count` matching the SAME values the Waypoint's own
    /// `NodeExecutionRecord` carries for that attempt -- trace and
    /// Waypoint never disagree.
    #[tokio::test]
    async fn node_finished_carries_real_outcome_and_cost() {
        let out = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let id = NodeId::new("retrier");
        let node = FailThenSucceedNode::new(2, "transient", out, serde_json::json!("ok"));
        graph.add_node(id.clone(), NodeSpec::Function(node));
        graph.add_entry(id.clone());
        graph.set_aegis(
            id.clone(),
            Aegis {
                retry: Some(RetryPolicy {
                    max_attempts: 3,
                    retry_on: RetryPredicate::TransientAndUnknown,
                    jitter: false,
                    initial_interval: std::time::Duration::from_millis(1),
                    ..RetryPolicy::default()
                }),
                ..Default::default()
            },
        );

        let store = Arc::new(RecordingWaypointStore::new());
        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(Arc::new(UnimplementedPaladinPort), store.clone())
            .with_trace_sink(sink.clone());
        let thread = ThreadId::new("node-finished-cost").unwrap();
        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut finishes: Vec<(u32, NodeOutcomeKind, u64, u64)> = sink
            .events()
            .await
            .iter()
            .filter_map(|r| match &r.event {
                TraceEvent::NodeFinished {
                    node_id,
                    attempt,
                    outcome,
                    duration_ms,
                    token_count,
                    ..
                } if node_id == &id => {
                    Some((*attempt, outcome.clone(), *duration_ms, *token_count))
                }
                _ => None,
            })
            .collect();
        finishes.sort_by_key(|(attempt, ..)| *attempt);
        assert_eq!(finishes.len(), 2, "two attempts: {finishes:?}");
        assert_eq!(finishes[0].0, 1);
        assert_ne!(
            finishes[0].1,
            NodeOutcomeKind::Succeeded,
            "attempt 1 must have failed"
        );
        assert_eq!(finishes[1].0, 2);
        assert_eq!(
            finishes[1].1,
            NodeOutcomeKind::Succeeded,
            "attempt 2 must have succeeded"
        );

        let waypoints = ascending_history(&store, &thread).await;
        let record = waypoints
            .iter()
            .flat_map(|w| w.completed.iter())
            .find(|r| r.node_id == id && r.attempt == 2)
            .expect("a NodeExecutionRecord for the succeeding attempt must exist");
        assert_eq!(
            finishes[1].2, record.duration_ms,
            "trace and Waypoint duration_ms must agree"
        );
        assert_eq!(
            finishes[1].3, record.token_count,
            "trace and Waypoint token_count must agree"
        );
    }

    /// D-02: a Muster worker's `NodeStarted` carries `muster_task_key:
    /// Some(..)`; an ordinary (non-Muster) node's is `None`.
    #[tokio::test]
    async fn node_started_carries_muster_task_key() {
        let results = FieldName::new("results").unwrap();
        let plain_out = FieldName::new("plain").unwrap();
        let schema = BattlefieldSchema::new(vec![
            FieldSpec::new(results.clone(), DispatchRule::Append, None, false),
            FieldSpec::new(plain_out.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let plain = NodeId::new("plain");
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![MusterTask {
                    worker: worker.clone(),
                    payload: serde_json::json!("x"),
                    task_key: "only".to_string(),
                }]),
            })
        };
        let worker_node = {
            let results = results.clone();
            CountingFunctionNode::with_context_directive(move |_run, _state, ctx| {
                let mut delta = StateDelta::new();
                delta.set_raw(
                    results.clone(),
                    ctx.muster_payload().cloned().unwrap_or_default(),
                );
                delta.into()
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node));
        graph.add_node(
            plain.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                plain_out,
                serde_json::json!("p"),
            )),
        );
        graph.add_entry(planner.clone());
        graph.add_entry(plain.clone());

        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone());
        let thread = ThreadId::new("muster-task-key").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let records = sink.events().await;
        let worker_started = records
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::NodeStarted {
                    node_id,
                    muster_task_key,
                    ..
                } if node_id == &worker => Some(muster_task_key.clone()),
                _ => None,
            })
            .expect("worker's NodeStarted must exist");
        assert_eq!(worker_started, Some("only".to_string()));

        let plain_started = records
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::NodeStarted {
                    node_id,
                    muster_task_key,
                    ..
                } if node_id == &plain => Some(muster_task_key.clone()),
                _ => None,
            })
            .expect("plain node's NodeStarted must exist");
        assert_eq!(plain_started, None);
    }

    /// D-05, T-28-03-01: `DeltaMerged.field_changes` carries names/sizes
    /// only by default (`value: None`, `value_bytes` the serialized
    /// length); with value inclusion enabled the value is the redacted,
    /// truncated string, and an API-key-shaped token is redacted BEFORE
    /// truncation is applied (the security-instructions ordering rule).
    /// Drives `superstep::run` directly (`WarEngine` has no public knob
    /// for this yet -- 28-06 wires `TraceConfig::state_values` through
    /// `TraceDispatcher::with_state_values`), so this test constructs its
    /// own dispatcher to exercise the opt-in path.
    #[tokio::test]
    async fn delta_merged_carries_names_not_values_by_default() {
        let out = FieldName::new("secret").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        // A value carrying an API-key-shaped token, long enough that the
        // redacted-and-capped assertion below is meaningfully distinct
        // from "the whole thing fit".
        let secret_value = format!("prefix sk-{} suffix {}", "a".repeat(40), "b".repeat(600));
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let id = NodeId::new("writer");
        graph.add_node(
            id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                out.clone(),
                serde_json::json!(secret_value),
            )),
        );
        graph.add_entry(id);

        let expected_bytes = serde_json::to_string(&serde_json::json!(secret_value))
            .unwrap()
            .len() as u64;

        let store = RecordingWaypointStore::new();
        let port: Arc<dyn PaladinPort> = Arc::new(UnimplementedPaladinPort);

        // -- Default: names and sizes only, no value. --
        let sink = RecordingTraceSink::new();
        let thread = ThreadId::new("delta-merged-default").unwrap();
        let trace = Arc::new(TraceDispatcher::new(
            thread.clone(),
            None,
            Some(sink.clone()),
        ));
        let outcome = crate::engine::superstep::run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EngineRegistries::new(),
            &graph,
            thread,
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            &port,
            &trace,
            &[],
            &None,
            &None,
            None,
            std::time::Duration::from_secs(30),
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let default_change = sink
            .events()
            .await
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::DeltaMerged { field_changes, .. } => {
                    field_changes.iter().find(|fc| fc.field == out).cloned()
                }
                _ => None,
            })
            .expect("a DeltaMerged carrying the changed field must exist");
        assert_eq!(default_change.value, None);
        assert_eq!(default_change.value_bytes, expected_bytes);

        // -- Value inclusion enabled: redacted, then capped. --
        let cap = 64usize;
        let sink2 = RecordingTraceSink::new();
        let thread2 = ThreadId::new("delta-merged-included").unwrap();
        let trace2 = Arc::new(
            TraceDispatcher::new(thread2.clone(), None, Some(sink2.clone())).with_state_values(
                true,
                cap,
                Arc::new(paladin_llm::redaction::redact_secret_patterns),
            ),
        );
        let outcome2 = crate::engine::superstep::run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EngineRegistries::new(),
            &graph,
            thread2,
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            &port,
            &trace2,
            &[],
            &None,
            &None,
            None,
            std::time::Duration::from_secs(30),
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert!(matches!(outcome2, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let included_change = sink2
            .events()
            .await
            .iter()
            .find_map(|r| match &r.event {
                TraceEvent::DeltaMerged { field_changes, .. } => {
                    field_changes.iter().find(|fc| fc.field == out).cloned()
                }
                _ => None,
            })
            .expect("a DeltaMerged carrying the changed field must exist");
        let value = included_change.value.expect("value inclusion was enabled");
        // `redact_secret_patterns` keeps the MARKER ("sk-") visible and
        // replaces only the secret token that follows it (module docs,
        // `redact_token_after`) -- the assertion below is on the SECRET
        // itself being gone, not the marker.
        assert!(
            !value.contains(&"a".repeat(40)),
            "the API-key-shaped token's secret portion must be redacted: {value}"
        );
        assert!(
            value.contains("[REDACTED]"),
            "the redaction placeholder must be present: {value}"
        );
        assert!(
            value.chars().count() <= cap + 64,
            "the value must be truncated to roughly the configured cap (plus the elision \
             marker): {value}"
        );
    }

    /// D-03: records emitted through `engine.trace_emitter().emit(..)`
    /// after a run interleave into the SAME `seq` sequence that run's own
    /// records were stamped with -- no repeats, no gaps.
    #[tokio::test]
    async fn trace_emitter_shares_the_run_counter() {
        let out = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let id = NodeId::new("solo");
        graph.add_node(
            id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(out, serde_json::json!("v"))),
        );
        graph.add_entry(id);

        let sink = RecordingTraceSink::new();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone());
        let thread = ThreadId::new("trace-emitter-shares").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let before = sink.events().await;
        assert!(!before.is_empty(), "the run must have emitted records");
        let max_seq_before = before.iter().map(|r| r.seq).max().unwrap();

        // Chain three more records through the SAME dispatcher the run
        // just used.
        let emitter = engine.trace_emitter();
        for _ in 0..3 {
            emitter.emit(TraceEvent::MiddlewareEvent {
                name: "post-run-probe".to_string(),
                action: MiddlewareAction::Retry,
            });
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let after = sink.events().await;
        let mut seqs: Vec<u64> = after.iter().map(|r| r.seq).collect();
        seqs.sort_unstable();
        for pair in seqs.windows(2) {
            assert_eq!(pair[1], pair[0] + 1, "seq must be gapless: {seqs:?}");
        }
        assert_eq!(seqs.first().copied(), Some(1));
        assert_eq!(after.len(), before.len() + 3);
        assert_eq!(
            after.iter().filter(|r| r.seq > max_seq_before).count(),
            3,
            "the three post-run records must continue the SAME seq sequence, never restart"
        );
    }

    /// 28-06: `with_bound_trace` closes the gap `trace_emitter`'s own doc
    /// comment describes -- a caller that pulls `trace_emitter()` BEFORE
    /// `start()`, after pre-binding via `with_bound_trace`, gets the SAME
    /// dispatcher `start()` itself then uses, so records emitted through
    /// that early handle interleave into the run's own gapless `seq`
    /// sequence rather than being orphaned on a placeholder dispatcher.
    #[tokio::test]
    async fn trace_emitter_before_start_uses_the_same_dispatcher_with_bound_trace() {
        let out = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let id = NodeId::new("solo");
        graph.add_node(
            id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(out, serde_json::json!("v"))),
        );
        graph.add_entry(id);

        let sink = RecordingTraceSink::new();
        let thread = ThreadId::new("trace-emitter-before-start").unwrap();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone())
        .with_bound_trace(thread.clone(), None);

        // Pulled BEFORE `start()` -- must be the SAME dispatcher `start()`
        // itself uses, not an orphaned placeholder.
        let emitter = engine.trace_emitter();
        emitter.emit(TraceEvent::MiddlewareEvent {
            name: "pre-run-probe".to_string(),
            action: MiddlewareAction::Fallback,
        });

        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let events = sink.events().await;

        let probe = events
            .iter()
            .find(|r| matches!(&r.event, TraceEvent::MiddlewareEvent { name, .. } if name == "pre-run-probe"))
            .expect("the pre-start probe record must have reached the run's own sink");
        assert_eq!(
            probe.seq, 1,
            "the pre-start probe is this dispatcher's FIRST record, seq 1"
        );

        let run_started = events
            .iter()
            .find(|r| matches!(r.event, TraceEvent::RunStarted { .. }))
            .expect("RunStarted must have been emitted through the SAME dispatcher");
        assert_eq!(
            run_started.seq, 2,
            "RunStarted continues the SAME seq sequence the pre-start probe started, never restarts"
        );

        let mut seqs: Vec<u64> = events.iter().map(|r| r.seq).collect();
        seqs.sort_unstable();
        for pair in seqs.windows(2) {
            assert_eq!(pair[1], pair[0] + 1, "seq must be gapless: {seqs:?}");
        }
    }

    /// 28-06: a `with_bound_trace` binding for a DIFFERENT thread than the
    /// one `start` is actually called with is never used -- `start` builds
    /// its own fresh dispatcher for its own thread instead, so the run's
    /// `seq` sequence starts at 1 regardless (D-03 is never compromised by
    /// a mismatched binding).
    #[tokio::test]
    async fn mismatched_bound_trace_thread_is_not_used() {
        let out = FieldName::new("out").unwrap();
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let id = NodeId::new("solo");
        graph.add_node(
            id.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(out, serde_json::json!("v"))),
        );
        graph.add_entry(id);

        let sink = RecordingTraceSink::new();
        let bound_thread = ThreadId::new("bound-thread").unwrap();
        let real_thread = ThreadId::new("real-thread").unwrap();
        let engine = WarEngine::new(
            Arc::new(UnimplementedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        )
        .with_trace_sink(sink.clone())
        .with_bound_trace(bound_thread, None);

        let outcome = engine
            .start(&graph, real_thread.clone(), StateDelta::new())
            .await
            .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let events = sink.events().await;
        assert!(
            events.iter().all(|r| r.thread_id == real_thread),
            "every record must carry the REAL thread the run actually executed under"
        );
        let run_started = events
            .iter()
            .find(|r| matches!(r.event, TraceEvent::RunStarted { .. }))
            .expect("RunStarted must have been emitted");
        assert_eq!(
            run_started.seq, 1,
            "a fresh dispatcher for the real thread starts its own seq at 1"
        );
    }
}
