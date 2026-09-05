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
pub mod directive_parser;
pub mod dispatch_registry;
pub mod graph;
pub mod hooks;
pub mod input_mapping;
pub mod node;
mod superstep;
#[cfg(test)]
pub(crate) mod test_support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeEvaluatorRegistry};
#[cfg(test)]
use paladin_core::platform::container::battlefield::CustomDispatchResolver;
use paladin_core::platform::container::battlefield::{Battlefield, FieldName, StateDelta};
use paladin_core::platform::container::battlefield_error::BattlefieldError;
#[cfg(test)]
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::parley::{
    OnExpire, ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};
use paladin_core::platform::container::waypoint::{
    GraphFingerprint, NodeId, ThreadId, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceSink};
use paladin_ports::output::waypoint_port::{WaypointError, WaypointPort};

pub use bridges::{CAMPAIGN_FAN_IN_SEPARATOR, campaign_node_ids, dedicated_output_field};
pub use directive_parser::{DirectiveParseError, DirectiveParser, OnParseError};
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
            None,
            1,
            &self.paladin_port,
            &self.trace_dispatcher,
            &self.interceptors,
            &self.cancellation_token,
            Some(Arc::clone(&self.waypoint_port)),
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
        if let WaypointStatus::Failed { error, failed_node } = &latest.status {
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
        graph.validate(registry, &self.edge_evaluators)?;

        self.trace_dispatcher.emit(TraceEvent::RunStarted {
            thread_id: thread.clone(),
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
            &self.edge_evaluators,
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
            &self.trace_dispatcher,
            &self.interceptors,
            &self.cancellation_token,
            Some(Arc::clone(&self.waypoint_port)),
        )
        .await;
        self.trace_dispatcher
            .emit(TraceEvent::RunFinished { thread_id: thread });
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
        graph.validate(registry, &self.edge_evaluators)?;

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
                        &self.trace_dispatcher,
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
                &self.trace_dispatcher,
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

        self.trace_dispatcher.emit(TraceEvent::RunStarted {
            thread_id: thread.clone(),
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
            &self.edge_evaluators,
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
            &self.trace_dispatcher,
            &self.interceptors,
            &self.cancellation_token,
            Some(Arc::clone(&self.waypoint_port)),
            None,
            // --- HITL-03, D-14: the resumed run's own Waypoints stay on the
            // SAME branch `latest` (the just-loaded `AwaitingInput`
            // Waypoint) was on -- propagated verbatim, so a suspended
            // branch's resume never silently reverts to mainline.
            latest.fork_of,
            Some(responses_by_node),
        )
        .await;
        self.trace_dispatcher
            .emit(TraceEvent::RunFinished { thread_id: thread });
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
    use paladin_core::platform::container::battalion::campaign::EdgeCondition;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::directive::{Directive, NextStep};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::parley::{OnExpire, ParleyKind};
    use paladin_core::platform::container::waypoint::{NodeOutcomeKind, Waypoint};
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream};
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use crate::engine::graph::{EdgeSpec, GateRequestTemplate};
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
        ) -> Result<paladin_core::platform::container::directive::Directive, NodeError> {
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
            WaypointStatus::Failed { error, failed_node } => {
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
}
