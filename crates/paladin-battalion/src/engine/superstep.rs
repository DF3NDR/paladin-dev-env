//! The superstep loop (ENG-FR-01): snapshot isolation, bounded concurrency,
//! one automatic Waypoint per superstep, and the two engine limits.
//!
//! `WarEngine::start` and (from Plan 22-08) `WarEngine::resume` both reduce
//! to a call into [`run`], which implements ENG-FR-01's seven steps in
//! order: take the current Vanguard; take ONE `Arc<Battlefield>` read
//! snapshot for the whole superstep; execute the Vanguard's nodes
//! concurrently; collect one `(NodeId, StateDelta)` per node; merge them
//! through `Battlefield::merge`; compute the next Vanguard; persist exactly
//! one Waypoint; then decide whether to continue.
//!
//! Next-Vanguard computation (`Frontier`, ENG-FR-06) resolves every incoming
//! edge of every node to `Fired`, `NotFiring`, or `Pending`, persisting that
//! resolution across supersteps: a node becomes executable once no incoming
//! edge from a run-reachable source is still pending and at least one has
//! fired (freshly, for a node re-entering the Vanguard after an earlier
//! execution — the cycle/self-loop case). A node whose every incoming edge
//! resolves not-firing, including transitively via a source itself proven
//! dead, is propagated to a fixpoint as dead, so a false branch can never
//! strand a downstream join waiting on it. `defer`-marked nodes that are
//! otherwise executable are held back until the computed Vanguard would
//! otherwise contain no non-deferred executable node.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use log::warn;
use regex::Regex;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, CustomDispatchResolver, FieldName,
};
use paladin_core::platform::container::directive::{
    Directive, MusterContext, MusterTask, NextStep,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    FrontierEdgeState, FrontierSnapshot, NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId,
    Waypoint, WaypointId, WaypointStatus, canonical_edge_condition,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::trace_sink_port::TraceEvent;
use paladin_ports::output::waypoint_port::WaypointPort;

use crate::edge_evaluator::EdgeEvaluatorRegistry;
use crate::engine::directive_parser::{DirectiveParseError, DirectiveParser};
use crate::engine::graph::{NodeSpec, WarGraph};
use crate::engine::hooks::{InterceptDecision, NodeInterceptor, TraceDispatcher};
use crate::engine::input_mapping::InputMapping;
use crate::engine::node::NodeError;
use crate::engine::{EngineError, RunOutcome, WaypointDurability};

/// What one vanguard node resolves to for this superstep's execution: either
/// a `Function` node's trait object, or the pieces of a `NodeSpec::Paladin`
/// node needed to render its input and call the port, cloned out of the
/// graph so the spawned task owns everything it touches (`Paladin` is `Box`ed
/// in `NodeSpec`; cloning one `Paladin` per executing node per superstep is
/// the accepted cost of keeping `WarGraph` itself immutable and shareable
/// across concurrently-executing peers).
enum NodeDispatch {
    /// A pure `Function` node.
    Function(Arc<dyn crate::engine::node::StateNode>),
    /// A `NodeSpec::Paladin` node's execution inputs.
    Paladin {
        /// The Paladin to execute.
        paladin: Box<Paladin>,
        /// Renders the Paladin's string input from the superstep snapshot.
        input_template: InputMapping,
        /// The field `PaladinResult.output` is written into as a delta
        /// under `DirectiveParser::PlainOutput`, or the fallback target
        /// under `OnParseError::FallbackPlain` (CF-02, D-11).
        output_field: FieldName,
        /// How this node's raw output becomes a routing `Directive`.
        directive_parser: DirectiveParser,
    },
}

/// A vanguard node's failure, distinguishing a `DirectiveParser` parse
/// failure (CF-02, D-11) -- which the per-node accumulation loop below
/// converts to the typed `EngineError::DirectiveParseFailed` naming this
/// node -- from every other node-execution failure, which converts to the
/// existing generic `EngineError::Node` exactly as before this phase (X-06:
/// no bare-`String` variant added for the new failure mode; the existing
/// generic path is untouched for every other case).
enum NodeFailure {
    /// A `Function` node's own error, a `NodeSpec::Paladin` node's
    /// `InputMapping::render`/`PaladinPort::execute` failure, or an
    /// internal engine error -- everything that was `NodeError` before this
    /// phase, unchanged.
    Node(NodeError),
    /// A `NodeSpec::Paladin` node's `DirectiveParser::StructuredDirective`
    /// call under `OnParseError::FailRun` (CF-02, D-11).
    DirectiveParse(DirectiveParseError),
}

/// Execute one vanguard node's dispatch against `snapshot`.
///
/// Returns `(paladin_id, token_count, result)`: `paladin_id`/`token_count`
/// are `None`/`0` for a `Function` node (it never carries either), and are
/// populated from the executed `Paladin` and its `PaladinResult` for a
/// `NodeSpec::Paladin` node. An `InputMapping::render` failure (an
/// undeclared field, or a declared field with no value and no default) and a
/// `PaladinPort::execute` error both become a `NodeError` here, so a Paladin
/// node's failure reaches the exact same node-failure path (and the same
/// `WaypointStatus::Failed { failed_node, .. }` reporting) a `Function`
/// node's own error already does — no special-cased Paladin failure path.
async fn execute_vanguard_node(
    dispatch: NodeDispatch,
    snapshot: &Battlefield,
    ctx: &crate::engine::node::NodeContext,
    paladin_port: &Arc<dyn PaladinPort>,
) -> (Option<Uuid>, u64, Result<Directive, NodeFailure>) {
    match dispatch {
        NodeDispatch::Function(node) => {
            let result = node.run(snapshot, ctx).await;
            (None, 0, result.map_err(NodeFailure::Node))
        }
        NodeDispatch::Paladin {
            paladin,
            input_template,
            output_field,
            directive_parser,
        } => {
            let paladin_id = Some(paladin.uuid);
            let rendered = match input_template.render(snapshot) {
                Ok(rendered) => rendered,
                Err(e) => {
                    return (
                        paladin_id,
                        0,
                        Err(NodeFailure::Node(NodeError(e.to_string()))),
                    );
                }
            };
            match paladin_port.execute(&paladin, &rendered).await {
                Ok(result) => {
                    let token_count = u64::from(result.token_count);
                    // --- CF-02, D-11: the `DirectiveParser` call replacing
                    // the prior unconditional `delta.set(output_field,
                    // result.output.clone())` write. `PlainOutput`
                    // reproduces that write verbatim; `StructuredDirective`
                    // parses D-11's envelope and applies only its `delta`.
                    match directive_parser.parse(&result.output, &output_field) {
                        Ok(directive) => (paladin_id, token_count, Ok(directive)),
                        Err(e) => (paladin_id, token_count, Err(NodeFailure::DirectiveParse(e))),
                    }
                }
                Err(e) => (
                    paladin_id,
                    0,
                    Err(NodeFailure::Node(NodeError(e.to_string()))),
                ),
            }
        }
    }
}

/// What one vanguard node's per-superstep processing (its `NodeInterceptor`
/// `before` chain, its dispatch if `Proceed`d, and its `after` chain)
/// resolved to (ENG-FR-22), replacing the plain `Result<StateDelta,
/// NodeError>` `execute_vanguard_node` alone would produce: a `Skip`
/// decision is neither a success nor a failure, so it needs its own
/// variant rather than being folded into one of the other two.
enum NodeRunOutcome {
    /// The node executed (or an interceptor's `before` chain unanimously
    /// `Proceed`ed and the node itself succeeded) and produced this
    /// `Directive`, whose `delta` has already passed through every `after`
    /// hook in order (`next` is untouched by any interceptor, per D-08's
    /// leave-ENG-07-untouched discretion).
    Succeeded(Directive),
    /// A `NodeInterceptor::before` returned `Skip(reason)`: the node never
    /// executed. Contributes no delta to this superstep's merge.
    Skipped(String),
    /// The node failed -- either its own execution returned an error, a
    /// `NodeSpec::Paladin` node's `DirectiveParser` failed to parse under
    /// `OnParseError::FailRun` (CF-02, D-11), or a `NodeInterceptor::before`
    /// returned `Fail(error)` before the node could run.
    Failed(NodeFailure),
}

/// Accept an incoming `NextStep::Muster(tasks)` at the Directive-receipt
/// point (CF-03, D-13) -- the SAME per-node accumulation loop where a
/// `Goto` target is validated -- and BEFORE any task is dispatched. This
/// tracer slice carries every task forward unconditionally; Plan 23-05's
/// Task 2 extends this function with the malformed-Muster rejection clauses
/// (empty list, duplicate `task_key`, `max_muster_tasks` breach, unknown or
/// non-worker-template `worker`). Returns `tasks` sorted by `task_key`
/// (`String` byte order) -- the ordering the deterministic task_key-order
/// merge (D-13) relies on, since every accepted task then reaches [`run`]'s
/// dispatch-building loop in this order and the existing
/// sequential-await-per-handle + stable `deltas.sort_by(NodeId)` machinery
/// preserves it into the final merge without any bespoke reordering.
fn validate_muster_tasks(mut tasks: Vec<MusterTask>) -> Vec<MusterTask> {
    tasks.sort_by(|a, b| a.task_key.cmp(&b.task_key));
    tasks
}

/// Run the superstep loop starting from `vanguard` at `superstep_number`,
/// over `battlefield`, persisting through `waypoint_port` under
/// `durability`, bounding per-superstep concurrency at `parallelism` (or the
/// Vanguard's own size when `None`, per D-12).
///
/// `parent_waypoint_id` chains the first Waypoint this call writes to the
/// caller-supplied lineage (`None` for a fresh `start`, `Some(id)` when a
/// later plan re-enters this loop from `resume`).
///
/// `frontier_snapshot` (BUG-04 / ENG-FR-12a) seeds the `Frontier` this call
/// builds: `None` for a fresh `start` (`Frontier::new`, every edge
/// `Pending`), `Some(snapshot)` for a `resume` (`Frontier::from_snapshot`),
/// restoring the per-edge resolutions and per-node last-executed supersteps
/// recorded before an earlier interruption, so a resumed run schedules the
/// same nodes in the same supersteps as an uninterrupted one.
///
/// `trace` receives every `TraceEvent` this loop's own steps produce
/// (`SuperstepStarted`, `NodeStarted`/`NodeFinished`, `DeltaMerged`,
/// `WaypointSaved`) -- `RunStarted`/`RunFinished` bracket the call from
/// `WarEngine::start`/`resume_with_options` instead, since a "run" is a
/// caller-level concept this loop itself has no opinion about (ENG-FR-21).
/// `interceptors` wraps each vanguard node's dispatch in an ordered
/// `NodeInterceptor` chain, empty by default (ENG-FR-22). `cancellation`,
/// checked at the top of the loop (i.e. at every superstep BOUNDARY,
/// including before the very first superstep), turns a cancelled token into
/// a `RunOutcome::Halted` carrying a `Waypoint` whose vanguard is exactly the
/// nodes that would have run next (ENG-FR-23) -- the in-flight superstep
/// that was already executing when cancellation fired always finishes and
/// merges first, since the check only ever happens between iterations.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run<W: WaypointPort>(
    waypoint_port: &W,
    durability: WaypointDurability,
    parallelism: Option<usize>,
    registry: &CustomDispatchResolver,
    evaluators: &EdgeEvaluatorRegistry,
    graph: &WarGraph,
    thread: ThreadId,
    mut battlefield: Battlefield,
    mut vanguard: Vec<NodeId>,
    mut visit_counts: BTreeMap<NodeId, u32>,
    frontier_snapshot: Option<FrontierSnapshot>,
    mut parent_waypoint_id: Option<WaypointId>,
    mut superstep_number: u64,
    paladin_port: &Arc<dyn PaladinPort>,
    trace: &Arc<TraceDispatcher>,
    interceptors: &[Arc<dyn NodeInterceptor>],
    cancellation: &Option<CancellationToken>,
) -> Result<RunOutcome, EngineError> {
    // The entry-vanguard-empty case: nothing to run, ever. Persist exactly
    // one Waypoint and return immediately (ENG-FR-01 step 7's "Vanguard
    // empty -> Completed" path, reached without executing a superstep at
    // all) -- UNLESS D-04's run-end truthful-outcome check finds a node
    // still holding an unconsumed fired incoming edge on a freshly built
    // `Frontier`, in which case this is the OTHER decision site
    // `starved_at_completion` guards (an empty entry Vanguard over a graph
    // whose Frontier disagrees is the same invariant violation as the
    // mid-loop site, just caught before any superstep ever ran).
    if vanguard.is_empty() {
        let entry_frontier = Frontier::for_run(graph, &frontier_snapshot);
        let starved = starved_at_completion(graph, &entry_frontier);
        if !starved.is_empty() {
            let names = starved
                .iter()
                .map(NodeId::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            let error = EngineError::StarvedNodeAtCompletion {
                nodes: starved.clone(),
                reason: format!(
                    "the entry Vanguard was empty but the eligible set still holds an \
                     unconsumed fired incoming edge on: {names} -- a node in the eligible set \
                     held an unconsumed fired incoming edge while the Vanguard was empty \
                     (ENG-FR-06a)"
                ),
            };
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                0,
                graph,
                &battlefield,
                Vec::new(),
                Vec::new(),
                WaypointStatus::Failed {
                    error: error.to_string(),
                    failed_node: starved[0].clone(),
                },
                visit_counts,
                entry_frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        let waypoint = build_waypoint(
            &thread,
            parent_waypoint_id,
            0,
            graph,
            &battlefield,
            Vec::new(),
            Vec::new(),
            WaypointStatus::Completed,
            visit_counts,
            entry_frontier.snapshot(graph),
        );
        persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
        return Ok(RunOutcome::Completed {
            final_state: battlefield,
            waypoint: waypoint.waypoint_id,
        });
    }

    let mut frontier = Frontier::for_run(graph, &frontier_snapshot);

    // --- CF-03: a validated `NextStep::Muster(tasks)` accepted in
    // superstep N is carried here, purely as a loop-local value (never
    // persisted -- Plan 23-06 owns mid-muster crash survival, D-14), and
    // dispatched as synthetic vanguard entries at the top of superstep
    // N+1's iteration below.
    let mut pending_muster: Option<Vec<MusterTask>> = None;

    loop {
        // --- ENG-FR-23: cancellation is observed only at a superstep
        // BOUNDARY -- here, at the top of the loop -- never mid-superstep.
        // `vanguard` at this point is exactly the set of nodes that would
        // run next (the graph's entry set on the very first iteration, or
        // the previous iteration's freshly computed next-Vanguard
        // otherwise), so persisting it verbatim on a `Halted` Waypoint is
        // what makes `resume` able to continue from exactly where this run
        // was asked to stop.
        if cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                Vec::new(),
                WaypointStatus::Halted,
                visit_counts,
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Halted {
                waypoint: waypoint.waypoint_id,
            });
        }

        // --- ENG-FR-03: bounded iteration, checked at the top of the loop
        // so a run stops at exactly `max_supersteps` rather than one over.
        if superstep_number >= graph.limits().max_supersteps {
            let error = EngineError::RecursionLimitExceeded {
                limit: graph.limits().max_supersteps,
                thread_id: thread.clone(),
            };
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                Vec::new(),
                WaypointStatus::Failed {
                    error: error.to_string(),
                    failed_node: vanguard[0].clone(),
                },
                visit_counts,
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        // --- ENG-FR-03: per-node visit bound, checked before a node is
        // placed into this superstep's execution set.
        let mut candidate_counts = visit_counts.clone();
        let mut tripped: Option<NodeId> = None;
        for node_id in &vanguard {
            let count = candidate_counts.entry(node_id.clone()).or_insert(0);
            *count += 1;
            if *count >= graph.limits().max_node_visits {
                tripped = Some(node_id.clone());
                break;
            }
        }
        if let Some(node) = tripped {
            let error = EngineError::NodeVisitLimitExceeded {
                node: node.clone(),
                limit: graph.limits().max_node_visits,
            };
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                Vec::new(),
                WaypointStatus::Failed {
                    error: error.to_string(),
                    failed_node: node,
                },
                visit_counts,
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }
        visit_counts = candidate_counts;

        trace.emit(TraceEvent::SuperstepStarted {
            thread_id: thread.clone(),
            superstep: superstep_number,
        });

        // --- CF-03: this superstep's dispatch entries = every ordinary
        // `vanguard` node (`muster: None`) PLUS every task from a Muster
        // accepted in the PREVIOUS superstep (`pending_muster`, taken here
        // so it dispatches exactly once), each a synthetic entry sharing
        // its `worker` template's `NodeId` with `NodeContext.muster` set
        // (RESEARCH.md Pitfall 3: the SAME snapshot/spawn/semaphore
        // machinery ordinary vanguard nodes use, never a bespoke "run these
        // N tasks" loop). `pending_muster`'s tasks already arrive sorted by
        // `task_key` (`validate_muster_tasks`); pushed in that order here,
        // the existing sequential-await-per-handle plus the stable
        // `deltas.sort_by(NodeId)` below preserve that order into the
        // final merge with no bespoke reordering. Muster dispatch entries
        // are NOT subject to `visit_counts`/`max_node_visits` (that bound
        // governs a node's own re-entry into the vanguard across
        // supersteps, e.g. a Goto refine loop; a Muster's fan-out width is
        // bounded separately by `EngineLimits::max_muster_tasks`).
        let muster_dispatch: Vec<(NodeId, Option<MusterContext>)> = pending_muster
            .take()
            .into_iter()
            .flatten()
            .map(|task| {
                (
                    task.worker,
                    Some(MusterContext {
                        payload: task.payload,
                        task_key: task.task_key,
                    }),
                )
            })
            .collect();
        let dispatch_entries: Vec<(NodeId, Option<MusterContext>)> = vanguard
            .iter()
            .map(|id| (id.clone(), None))
            .chain(muster_dispatch)
            .collect();

        // --- ENG-FR-05/ENG-NFR-02: exactly one Arc-shared read snapshot
        // for the whole superstep, cloned once.
        let snapshot = Arc::new(battlefield.clone());
        let limit = parallelism.unwrap_or(dispatch_entries.len()).max(1);
        let semaphore = Arc::new(Semaphore::new(limit));

        let mut handles = Vec::with_capacity(dispatch_entries.len());
        for (node_id, muster_ctx) in &dispatch_entries {
            let spec = graph.node(node_id).ok_or_else(|| {
                EngineError::Node(NodeError(format!(
                    "vanguard node {node_id} not found in graph"
                )))
            })?;
            let dispatch = match spec {
                NodeSpec::Function(node) => NodeDispatch::Function(Arc::clone(node)),
                NodeSpec::Paladin {
                    paladin,
                    input_template,
                    output_field,
                    directive_parser,
                } => NodeDispatch::Paladin {
                    paladin: paladin.clone(),
                    input_template: input_template.clone(),
                    output_field: output_field.clone(),
                    directive_parser: directive_parser.clone(),
                },
            };
            let snap = Arc::clone(&snapshot);
            let sem = Arc::clone(&semaphore);
            let port = Arc::clone(paladin_port);
            let node_trace = Arc::clone(trace);
            let node_interceptors = interceptors.to_vec();
            let ctx = crate::engine::node::NodeContext {
                node_id: node_id.clone(),
                thread_id: thread.clone(),
                superstep: superstep_number,
                muster: muster_ctx.clone(),
            };
            let nid = node_id.clone();
            handles.push(tokio::spawn(async move {
                node_trace.emit(TraceEvent::NodeStarted {
                    thread_id: ctx.thread_id.clone(),
                    superstep: ctx.superstep,
                    node_id: nid.clone(),
                });
                let started_at = Utc::now();

                // --- ENG-FR-22: run every `before` in order, short-
                // circuiting on the first non-`Proceed` decision.
                let mut decision = InterceptDecision::Proceed;
                for interceptor in &node_interceptors {
                    decision = interceptor.before(&ctx, &snap).await;
                    if !matches!(decision, InterceptDecision::Proceed) {
                        break;
                    }
                }

                let (paladin_id, token_count, outcome) = match decision {
                    InterceptDecision::Skip(reason) => {
                        (None, 0u64, NodeRunOutcome::Skipped(reason))
                    }
                    InterceptDecision::Fail(err) => {
                        (None, 0u64, NodeRunOutcome::Failed(NodeFailure::Node(err)))
                    }
                    InterceptDecision::Proceed => match sem.acquire_owned().await {
                        Ok(_permit) => {
                            let (paladin_id, token_count, result) =
                                execute_vanguard_node(dispatch, &snap, &ctx, &port).await;
                            match result {
                                Ok(mut directive) => {
                                    // --- ENG-FR-22: run every `after` in
                                    // order, each observing the previous
                                    // one's mutation. `after` still takes
                                    // `&mut StateDelta` only -- the ENG-07
                                    // hook signature is unchanged by CF-02;
                                    // `directive.next` is not visible to any
                                    // interceptor this phase.
                                    for interceptor in &node_interceptors {
                                        interceptor.after(&ctx, &mut directive.delta).await;
                                    }
                                    (
                                        paladin_id,
                                        token_count,
                                        NodeRunOutcome::Succeeded(directive),
                                    )
                                }
                                Err(e) => (paladin_id, token_count, NodeRunOutcome::Failed(e)),
                            }
                        }
                        // Semaphore is never `.close()`d anywhere in this
                        // engine today, so this arm is unreachable in
                        // practice -- but library code must not `.expect()`
                        // an invariant it cannot enforce (WR-01, Phase
                        // 22.1). Report it the same way a node's own
                        // execution error is reported, through the existing
                        // NodeRunOutcome/NodeError plumbing, rather than
                        // panicking inside a detached `tokio::spawn`ed task.
                        Err(_) => (
                            None,
                            0u64,
                            NodeRunOutcome::Failed(NodeFailure::Node(NodeError(
                                "internal error: superstep semaphore closed unexpectedly"
                                    .to_string(),
                            ))),
                        ),
                    },
                };
                let duration_ms = (Utc::now() - started_at).num_milliseconds().max(0) as u64;
                node_trace.emit(TraceEvent::NodeFinished {
                    thread_id: ctx.thread_id.clone(),
                    superstep: ctx.superstep,
                    node_id: nid.clone(),
                });
                (
                    nid,
                    started_at,
                    duration_ms,
                    paladin_id,
                    token_count,
                    outcome,
                )
            }));
        }

        let mut deltas = Vec::with_capacity(handles.len());
        let mut completed_records = Vec::with_capacity(handles.len());
        let mut node_failure: Option<(NodeId, NodeFailure)> = None;
        // --- CF-02: per-superstep runtime values derived from this
        // superstep's `Directive`s, NOT `Frontier` state (RESEARCH.md
        // Pattern 3) -- rebuilt fresh every superstep, never persisted.
        // `goto_targets` is unioned into `next_vanguard` after
        // `compute_next_vanguard` returns; `notfiring_nodes` marks every
        // node whose `Directive.next` was not `Edges`, so
        // `Frontier::record_execution` resolves its static outgoing edges
        // `NotFiring` directly instead of evaluating them (D-08c, serves
        // Goto/Muster/End/Parley alike); `end_requested` is the first node
        // this superstep to return `NextStep::End` (D-09); `mustered` is a
        // validated `NextStep::Muster` task list (CF-03, D-13), threaded
        // into `pending_muster` for the NEXT superstep's dispatch below. A
        // Goto target validation failure, an invalid Muster, or a returned
        // Parley all fail the run before any of this bookkeeping is acted
        // on further (checked together with `node_failure`, before the
        // merge).
        let mut goto_targets: Vec<NodeId> = Vec::new();
        let mut notfiring_nodes: HashSet<NodeId> = HashSet::new();
        let mut end_requested: Option<NodeId> = None;
        let mut routing_failure: Option<(NodeId, EngineError)> = None;
        let mut mustered: Option<Vec<MusterTask>> = None;
        for handle in handles {
            let (node_id, started_at, duration_ms, paladin_id, token_count, outcome) = handle
                .await
                .map_err(|e| EngineError::Node(NodeError(format!("task join error: {e}"))))?;
            match outcome {
                NodeRunOutcome::Succeeded(directive) => {
                    let Directive { delta, next } = directive;
                    let outcome_kind = match &next {
                        NextStep::Edges => NodeOutcomeKind::Succeeded,
                        NextStep::Goto(targets) => {
                            notfiring_nodes.insert(node_id.clone());
                            for target in targets {
                                if graph.node(target).is_none() {
                                    if routing_failure.is_none() {
                                        routing_failure = Some((
                                            node_id.clone(),
                                            EngineError::GotoUnknownNode {
                                                from: node_id.clone(),
                                                to: target.clone(),
                                            },
                                        ));
                                    }
                                } else {
                                    goto_targets.push(target.clone());
                                }
                            }
                            NodeOutcomeKind::Succeeded
                        }
                        NextStep::Muster(tasks) => {
                            // CF-03, D-13: accepted here, at
                            // Directive-receipt time -- the SAME per-node
                            // accumulation loop Goto validates in -- and
                            // carried into `pending_muster` for dispatch
                            // next superstep, never inside the
                            // worker-dispatch loop itself. Malformed-Muster
                            // rejection (empty/duplicate/limit/unknown
                            // worker) is Plan 23-05's Task 2.
                            notfiring_nodes.insert(node_id.clone());
                            if mustered.is_none() {
                                mustered = Some(validate_muster_tasks(tasks.clone()));
                            }
                            NodeOutcomeKind::Succeeded
                        }
                        NextStep::End => {
                            notfiring_nodes.insert(node_id.clone());
                            if end_requested.is_none() {
                                end_requested = Some(node_id.clone());
                            }
                            NodeOutcomeKind::Ended
                        }
                        NextStep::Parley(_) => {
                            // D-10: never coerced to `Edges`; still marked
                            // NotFiring per D-08c even though the run is
                            // about to fail, for the same "no NextStep
                            // variant leaves an edge Pending" uniformity.
                            notfiring_nodes.insert(node_id.clone());
                            if routing_failure.is_none() {
                                routing_failure = Some((
                                    node_id.clone(),
                                    EngineError::ParleyNotSupported {
                                        node: node_id.clone(),
                                    },
                                ));
                            }
                            NodeOutcomeKind::Succeeded
                        }
                    };
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: outcome_kind,
                        attempt: 1,
                    });
                    deltas.push((node_id, delta));
                }
                NodeRunOutcome::Skipped(reason) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: NodeOutcomeKind::Skipped { reason },
                        attempt: 1,
                    });
                }
                NodeRunOutcome::Failed(e) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: NodeOutcomeKind::Failed,
                        attempt: 1,
                    });
                    if node_failure.is_none() {
                        node_failure = Some((node_id, e));
                    }
                }
            }
        }
        completed_records.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        if let Some((node_id, err)) = node_failure {
            // --- CF-02, D-11: a `DirectiveParser` parse failure gets its
            // own typed `EngineError` naming the node (X-06), rather than
            // routing through the generic `EngineError::Node` every other
            // node-execution failure uses.
            let error = match err {
                NodeFailure::Node(e) => EngineError::Node(e),
                NodeFailure::DirectiveParse(e) => EngineError::DirectiveParseFailed {
                    node: node_id.clone(),
                    reason: e.reason,
                },
            };
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                completed_records,
                WaypointStatus::Failed {
                    error: error.to_string(),
                    failed_node: node_id,
                },
                visit_counts,
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        // --- CF-02: a `Goto` target that names an undeclared node, or a
        // returned `Parley` (D-10), both fail the run here -- before the
        // merge, mirroring `node_failure`'s ordering -- so neither
        // `goto_targets` nor `notfiring_nodes` ever reaches `Frontier`
        // state (D-08a: validated the moment the Directive is received,
        // before any routing state changes).
        if let Some((failed_node, error)) = routing_failure {
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                completed_records,
                WaypointStatus::Failed {
                    error: error.to_string(),
                    failed_node,
                },
                visit_counts,
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        // --- Merge, only after every node in this superstep has completed
        // (ENG-FR-05: no node observes a peer's delta this superstep).
        deltas.sort_by(|a, b| a.0.cmp(&b.0));
        let ran: Vec<NodeId> = deltas.iter().map(|(id, _)| id.clone()).collect();
        let merge_report = match battlefield.merge(deltas, superstep_number, registry) {
            Ok(report) => report,
            Err(e) => {
                let error = EngineError::Battlefield(e);
                let waypoint = build_waypoint(
                    &thread,
                    parent_waypoint_id,
                    superstep_number,
                    graph,
                    &battlefield,
                    vanguard.clone(),
                    completed_records,
                    WaypointStatus::Failed {
                        error: error.to_string(),
                        // CF-03: `vanguard` alone may be empty in a
                        // muster-only superstep, so the fallback reads from
                        // `dispatch_entries` (ordinary nodes + muster
                        // tasks) instead -- `ran`/`deltas` are only
                        // non-empty when at least one dispatch entry
                        // succeeded, so this branch is unreachable in
                        // practice, but must not panic if it ever is.
                        failed_node: ran
                            .first()
                            .cloned()
                            .unwrap_or_else(|| dispatch_entries[0].0.clone()),
                    },
                    visit_counts,
                    frontier.snapshot(graph),
                );
                persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
                return Ok(RunOutcome::Failed {
                    error,
                    waypoint: Some(waypoint.waypoint_id),
                });
            }
        };
        trace.emit(TraceEvent::DeltaMerged {
            thread_id: thread.clone(),
            superstep: superstep_number,
            field_changes: merge_report.changed_fields,
        });

        for node_id in &ran {
            frontier
                .record_execution(
                    graph,
                    node_id,
                    superstep_number,
                    &battlefield,
                    evaluators,
                    &thread,
                    notfiring_nodes.contains(node_id),
                )
                .await?;
        }
        frontier.propagate_dead(graph);
        let mut next_vanguard = compute_next_vanguard(graph, &frontier);

        // --- CF-02 / D-08b: union this superstep's validated `Goto`
        // targets into `next_vanguard`, bypassing `Frontier::is_ready`
        // entirely -- a Goto target is admitted unconditionally, not
        // because it satisfied the normal readiness test. De-duplicated
        // against nodes `compute_next_vanguard` already selected (a node
        // that is both a Goto target AND ordinarily tier-1-ready this
        // superstep is scheduled, and therefore executes, exactly once).
        // `compute_next_vanguard` itself stays pure over `Frontier` state
        // (RESEARCH.md Open Question 2) -- this union happens here, one
        // level up, over its result.
        if !goto_targets.is_empty() {
            let mut seen: HashSet<NodeId> = next_vanguard.iter().cloned().collect();
            for target in goto_targets {
                if seen.insert(target.clone()) {
                    next_vanguard.push(target);
                }
            }
        }

        // --- CF-03, D-13: a validated Muster accepted this superstep
        // dispatches next superstep regardless of what `next_vanguard`
        // (static-edge-derived) computed -- a worker template has no
        // static incoming edge (D-12), so `compute_next_vanguard` can never
        // select it on its own. `has_pending_muster` therefore stands in
        // for "there is more work next superstep" everywhere below that
        // would otherwise treat an empty `next_vanguard` as the run being
        // truly done.
        let has_pending_muster = mustered.is_some();

        // --- D-09 / CF-FR-08: `End` completes the run after this
        // superstep's merge -- which already happened above, so every
        // peer's delta is reflected in `battlefield` -- regardless of what
        // `compute_next_vanguard` and the `Goto` union just produced
        // (`End` beats `Goto` in the same superstep). This bypasses the
        // `StarvedNodeAtCompletion` check entirely rather than gating it on
        // `next_vanguard.is_empty()`: the check's job is to catch the
        // scheduler silently walking away from ready work it never
        // dispatched, and an explicit, node-authored `End` is not that --
        // it is deliberate, observable termination (recorded via
        // `NodeOutcomeKind::Ended` above), never a scheduler lie. The
        // suppression is scoped to exactly this fact (`end_requested`),
        // never to the general emptiness of `next_vanguard`: a run with no
        // `End` and a genuine starvation-invariant violation still reaches
        // the check below and fails loudly.
        if end_requested.is_some() {
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                Vec::new(),
                completed_records,
                WaypointStatus::Completed,
                visit_counts.clone(),
                frontier.snapshot(graph),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Completed {
                final_state: battlefield,
                waypoint: waypoint.waypoint_id,
            });
        }

        // D-04's run-end truthful-outcome check: an independent net over
        // the SAME `frontier` `compute_next_vanguard` just consumed, run
        // only when that computation says there is nothing left to
        // schedule -- AND there is no pending Muster (CF-03): a worker
        // template legitimately has no static incoming edge, so an empty
        // `next_vanguard` alongside a pending Muster is "waiting to
        // dispatch next superstep," not the scheduler silently walking
        // away from ready work. A non-empty result here means the
        // scheduler's own invariant broke -- some node in the eligible set
        // still holds an unconsumed fired incoming edge -- so `Completed`
        // is refused in favor of a typed, checkpointed failure naming every
        // such node.
        if next_vanguard.is_empty() && !has_pending_muster {
            let starved = starved_at_completion(graph, &frontier);
            if !starved.is_empty() {
                let names = starved
                    .iter()
                    .map(NodeId::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                let error = EngineError::StarvedNodeAtCompletion {
                    nodes: starved.clone(),
                    reason: format!(
                        "the computed next Vanguard was empty but the eligible set still \
                         holds an unconsumed fired incoming edge on: {names} -- a node in the \
                         eligible set held an unconsumed fired incoming edge while the \
                         Vanguard was empty (ENG-FR-06a)"
                    ),
                };
                let waypoint = build_waypoint(
                    &thread,
                    parent_waypoint_id,
                    superstep_number,
                    graph,
                    &battlefield,
                    next_vanguard.clone(),
                    completed_records,
                    WaypointStatus::Failed {
                        error: error.to_string(),
                        failed_node: starved[0].clone(),
                    },
                    visit_counts.clone(),
                    frontier.snapshot(graph),
                );
                persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
                return Ok(RunOutcome::Failed {
                    error,
                    waypoint: Some(waypoint.waypoint_id),
                });
            }
        }

        let status = if next_vanguard.is_empty() && !has_pending_muster {
            WaypointStatus::Completed
        } else {
            WaypointStatus::Running
        };

        let waypoint = build_waypoint(
            &thread,
            parent_waypoint_id,
            superstep_number,
            graph,
            &battlefield,
            next_vanguard.clone(),
            completed_records,
            status,
            visit_counts.clone(),
            frontier.snapshot(graph),
        );
        persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;

        if next_vanguard.is_empty() && !has_pending_muster {
            return Ok(RunOutcome::Completed {
                final_state: battlefield,
                waypoint: waypoint.waypoint_id,
            });
        }

        vanguard = next_vanguard;
        // --- CF-03: carry this superstep's validated Muster (if any) into
        // the next iteration's dispatch-entry build above.
        pending_muster = mustered;
        parent_waypoint_id = Some(waypoint.waypoint_id);
        superstep_number += 1;
    }
}

/// An incoming edge's resolution state, persisted across supersteps
/// (ENG-FR-06). `Pending` until the edge's source completes; `Fired`/
/// `NotFiring` from then on, stamped with the superstep the source
/// completed at so a re-entrant target (cycle/self-loop) can tell a fresh
/// firing from a stale one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeState {
    Pending,
    Fired(u64),
    NotFiring(u64),
}

/// Tracks, for the whole run, which incoming edges of which nodes have
/// resolved (ENG-FR-06): the precise join/defer/not-firing frontier that
/// replaces the "an edge whose source ran this superstep" heuristic. A
/// diamond join waits for every incoming edge to resolve rather than firing
/// once per satisfied edge; a false branch is proven `NotFiring` rather than
/// leaving its downstream join pending forever; a `defer`-marked node is
/// held back until no non-deferred node is executable.
struct Frontier {
    /// Per-edge state, indexed identically to `graph.edges()`.
    edge_state: Vec<EdgeState>,
    /// Nodes proven to never execute in this run: a non-entry node with no
    /// incoming edges, or a node all of whose incoming edges resolve
    /// not-firing (directly, or transitively via a dead source) with none
    /// firing.
    dead: HashSet<NodeId>,
    /// The superstep at which a node last executed, if ever.
    last_executed: HashMap<NodeId, u64>,
    /// Incoming edge indices per target node, in `graph.edges()`'s
    /// insertion order.
    incoming: HashMap<NodeId, Vec<usize>>,
}

/// Per-node edge-resolution facts shared by [`Frontier::is_ready`] and the
/// starvation-release fallback pass in `compute_next_vanguard` (ENG-FR-06a):
/// whether any incoming edge has fired at or after `node`'s `last_executed`
/// threshold (a "fresh" fire -- any fired edge at all, for a node that has
/// never executed), and the indices of any incoming edges still unresolved
/// (`Pending`) from a live, not-yet-proven-dead source
/// ([`Frontier::edge_resolution`] already resolves a `Pending` edge from a
/// dead source to not-firing, so every index collected here is genuinely
/// still pending). Returned by [`Frontier::node_edge_summary`].
struct NodeEdgeSummary {
    /// At least one incoming edge fired at or after `node`'s
    /// `last_executed` threshold.
    any_fresh_fire: bool,
    /// Incoming edge indices still unresolved from a live source.
    pending_from_live: Vec<usize>,
}

impl Frontier {
    /// Build the initial frontier for `graph`: every edge `Pending`, then
    /// propagate structural deadness (non-entry nodes with no incoming
    /// edges, and anything only reachable through them) to a fixpoint.
    fn new(graph: &WarGraph) -> Self {
        let mut incoming: HashMap<NodeId, Vec<usize>> = HashMap::new();
        for (idx, edge) in graph.edges().iter().enumerate() {
            incoming.entry(edge.to.clone()).or_default().push(idx);
        }
        let edge_state = vec![EdgeState::Pending; graph.edges().len()];
        let mut frontier = Self {
            edge_state,
            dead: HashSet::new(),
            last_executed: HashMap::new(),
            incoming,
        };
        frontier.propagate_dead(graph);
        frontier
    }

    /// Build the `Frontier` a `run` call starts from: fresh
    /// ([`Frontier::new`]) when `frontier_snapshot` is `None` (a fresh
    /// `start`), restored ([`Frontier::from_snapshot`]) when it is `Some`
    /// (a `resume`, BUG-04 / ENG-FR-12a). The one call site for both of
    /// `run`'s Frontier constructions (the early empty-Vanguard return and
    /// the main loop), so `Frontier::new` has exactly one call site in this
    /// module -- the fresh-start case -- and the resume path always reaches
    /// `Frontier::from_snapshot`.
    fn for_run(graph: &WarGraph, frontier_snapshot: &Option<FrontierSnapshot>) -> Self {
        match frontier_snapshot {
            Some(snapshot) => Self::from_snapshot(graph, snapshot),
            None => Self::new(graph),
        }
    }

    /// Build a `Frontier` for `graph` restored from a persisted
    /// [`FrontierSnapshot`] (BUG-04 / ENG-FR-12a): every graph edge whose
    /// identity (`from`, `to`, [`canonical_edge_condition`]) matches a
    /// snapshot entry is set `Fired`/`NotFiring` at the snapshot's
    /// `resolved_at`; every graph edge with no matching snapshot entry
    /// starts `Pending`, exactly as [`Frontier::new`] would build it -- a
    /// snapshot edge with no matching graph edge is silently dropped (D-22:
    /// under `ResumeOptions::allow_graph_change`, an edge the new graph no
    /// longer declares must not resurrect). `last_executed` entries naming a
    /// node absent from `graph` are dropped the same way. Structural
    /// deadness is then re-propagated exactly as `new` does, so the restored
    /// frontier is indistinguishable in shape from one built fresh and then
    /// driven to the same edge states by replaying every recorded
    /// execution.
    fn from_snapshot(graph: &WarGraph, snapshot: &FrontierSnapshot) -> Self {
        let mut incoming: HashMap<NodeId, Vec<usize>> = HashMap::new();
        for (idx, edge) in graph.edges().iter().enumerate() {
            incoming.entry(edge.to.clone()).or_default().push(idx);
        }

        let mut by_identity: HashMap<(&str, &str, &str), &FrontierEdgeState> = HashMap::new();
        for entry in &snapshot.edges {
            by_identity.insert(
                (
                    entry.from.as_str(),
                    entry.to.as_str(),
                    entry.condition.as_str(),
                ),
                entry,
            );
        }

        let mut edge_state = vec![EdgeState::Pending; graph.edges().len()];
        for (idx, edge) in graph.edges().iter().enumerate() {
            let condition = canonical_edge_condition(&edge.condition);
            let key = (edge.from.as_str(), edge.to.as_str(), condition.as_str());
            if let Some(entry) = by_identity.get(&key) {
                edge_state[idx] = if entry.fired {
                    EdgeState::Fired(entry.resolved_at)
                } else {
                    EdgeState::NotFiring(entry.resolved_at)
                };
            }
        }

        let last_executed: HashMap<NodeId, u64> = snapshot
            .last_executed
            .iter()
            .filter(|(node, _)| graph.node(node).is_some())
            .map(|(node, superstep)| (node.clone(), *superstep))
            .collect();

        let mut frontier = Self {
            edge_state,
            dead: HashSet::new(),
            last_executed,
            incoming,
        };
        frontier.propagate_dead(graph);
        frontier
    }

    /// Record that `node` completed `superstep`. When `force_notfiring` is
    /// `false` (the ordinary case), evaluates every one of `node`'s
    /// outgoing edges against the POST-merge `battlefield` and stores each
    /// as `Fired`/`NotFiring` at this superstep (ENG-FR-06). When `true` --
    /// `node`'s `Directive.next` was not `NextStep::Edges` (CF-02, D-08c) --
    /// every outgoing edge is set `NotFiring` directly, skipping
    /// `evaluate_edge_condition` entirely: a node that authored its own
    /// routing (`Goto`/`Muster`/`End`/`Parley`) never also fires its static
    /// outgoing edges for the same execution, so no `NextStep` variant can
    /// leave one `Pending` and strand a downstream join. Re-running a node
    /// (a cycle or self-loop) overwrites its edges' previous state with the
    /// fresh evaluation either way.
    #[allow(clippy::too_many_arguments)]
    async fn record_execution(
        &mut self,
        graph: &WarGraph,
        node: &NodeId,
        superstep: u64,
        battlefield: &Battlefield,
        evaluators: &EdgeEvaluatorRegistry,
        thread: &ThreadId,
        force_notfiring: bool,
    ) -> Result<(), EngineError> {
        self.last_executed.insert(node.clone(), superstep);
        for (idx, edge) in graph.edges().iter().enumerate() {
            if &edge.from != node {
                continue;
            }
            let fires = if force_notfiring {
                false
            } else {
                match &edge.condition {
                    None => true,
                    Some(condition) => {
                        evaluate_edge_condition(
                            condition,
                            battlefield,
                            graph,
                            evaluators,
                            node,
                            &edge.to,
                            thread,
                            superstep,
                        )
                        .await?
                    }
                }
            };
            self.edge_state[idx] = if fires {
                EdgeState::Fired(superstep)
            } else {
                EdgeState::NotFiring(superstep)
            };
        }
        Ok(())
    }

    /// This edge's resolution as `(fired, resolved_at)`, or `None` while
    /// still pending from a source that is not (yet) proven dead. A
    /// `Pending` edge whose source is dead is treated as resolved
    /// not-firing at superstep 0 -- the source will never run, so the edge
    /// will never fire (the "provably not-firing" half of ENG-FR-06).
    fn edge_resolution(&self, graph: &WarGraph, idx: usize) -> Option<(bool, u64)> {
        match self.edge_state[idx] {
            EdgeState::Fired(s) => Some((true, s)),
            EdgeState::NotFiring(s) => Some((false, s)),
            EdgeState::Pending => {
                let source = &graph.edges()[idx].from;
                if self.dead.contains(source) {
                    Some((false, 0))
                } else {
                    None
                }
            }
        }
    }

    /// Propagate dead-node status to a fixpoint (ENG-FR-06): a non-entry
    /// node with no incoming edges never executes; a node all of whose
    /// incoming edges are resolved not-firing (directly or via a dead
    /// source) with none firing is itself dead. Runs until no further node
    /// changes state, so a chain of unreachable nodes resolves in one call.
    /// Iterates `graph.node_order()`, never raw `HashMap` iteration
    /// (ENG-FR-04).
    fn propagate_dead(&mut self, graph: &WarGraph) {
        loop {
            let mut changed = false;
            for node in graph.node_order() {
                if self.dead.contains(node) || self.last_executed.contains_key(node) {
                    continue;
                }
                if graph.entry().contains(node) {
                    // Entry nodes are scheduled directly regardless of
                    // incoming-edge state; never mark one dead before it
                    // has had its guaranteed first execution.
                    continue;
                }
                let incoming = self.incoming.get(node).cloned().unwrap_or_default();
                if incoming.is_empty() {
                    self.dead.insert(node.clone());
                    changed = true;
                    continue;
                }
                let mut any_pending = false;
                let mut any_fired = false;
                for idx in &incoming {
                    match self.edge_resolution(graph, *idx) {
                        Some((true, _)) => any_fired = true,
                        Some((false, _)) => {}
                        None => any_pending = true,
                    }
                }
                if !any_pending && !any_fired {
                    self.dead.insert(node.clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Compute [`NodeEdgeSummary`] for `node`, or `None` if `node` has no
    /// declared incoming edges at all (never executable). Uses the same
    /// `u64 -> i64` threshold convention as the caller that used to inline
    /// this loop: `node`'s `last_executed` absent maps to `-1`, so a node
    /// that has never executed treats any fired edge as fresh.
    fn node_edge_summary(&self, graph: &WarGraph, node: &NodeId) -> Option<NodeEdgeSummary> {
        let incoming = self.incoming.get(node)?;
        if incoming.is_empty() {
            return None;
        }
        let threshold: i64 = self.last_executed.get(node).map_or(-1, |&s| s as i64);
        let mut any_fresh_fire = false;
        let mut pending_from_live = Vec::new();
        for &idx in incoming {
            match self.edge_resolution(graph, idx) {
                Some((true, resolved_at)) => {
                    if resolved_at as i64 >= threshold {
                        any_fresh_fire = true;
                    }
                }
                Some((false, _)) => {}
                None => pending_from_live.push(idx),
            }
        }
        Some(NodeEdgeSummary {
            any_fresh_fire,
            pending_from_live,
        })
    }

    /// Whether `node` is executable for the NEXT Vanguard (ENG-FR-06): it
    /// has at least one incoming edge, none of them is still pending from a
    /// run-reachable source (a `Pending` edge from a proven-dead source
    /// counts as resolved not-firing, via [`Frontier::edge_resolution`]),
    /// and at least one has fired at or after the superstep `node` last
    /// executed (any fired edge at all, for a node that has never
    /// executed). Delegates to [`Frontier::node_edge_summary`], the same
    /// per-node edge-resolution helper the ENG-FR-06a starvation-release
    /// pass uses.
    fn is_ready(&self, graph: &WarGraph, node: &NodeId) -> bool {
        let Some(summary) = self.node_edge_summary(graph, node) else {
            return false;
        };
        summary.pending_from_live.is_empty() && summary.any_fresh_fire
    }

    /// Snapshot this `Frontier` as of RIGHT NOW (BUG-04 / ENG-FR-12a): one
    /// [`FrontierEdgeState`] per edge whose `edge_state` is `Fired` or
    /// `NotFiring` (never for `Pending`, and never for the derived
    /// dead-source resolution [`Frontier::edge_resolution`] computes on the
    /// fly for a `Pending` edge from a proven-dead source -- that is
    /// re-derived by [`Frontier::propagate_dead`] on restore, not persisted
    /// here), de-duplicated by identity and sorted by `(from, to,
    /// condition)`, plus `last_executed` as a `BTreeMap` -- both collections
    /// keyed/ordered so two byte-identical runs produce byte-identical
    /// `Waypoint` payloads (ENG-FR-04/08, RESEARCH.md Pitfall 5). A
    /// duplicate-identity edge pair always resolves identically (both are
    /// evaluated from the same `record_execution` call against the same
    /// post-merge `Battlefield`), so collapsing them to one entry loses no
    /// information.
    fn snapshot(&self, graph: &WarGraph) -> FrontierSnapshot {
        let mut edges: BTreeMap<(String, String, String), FrontierEdgeState> = BTreeMap::new();
        for (idx, edge) in graph.edges().iter().enumerate() {
            let (fired, resolved_at) = match self.edge_state[idx] {
                EdgeState::Fired(s) => (true, s),
                EdgeState::NotFiring(s) => (false, s),
                EdgeState::Pending => continue,
            };
            let condition = canonical_edge_condition(&edge.condition);
            let key = (
                edge.from.as_str().to_string(),
                edge.to.as_str().to_string(),
                condition.clone(),
            );
            edges.insert(
                key,
                FrontierEdgeState {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    condition,
                    fired,
                    resolved_at,
                },
            );
        }

        FrontierSnapshot {
            edges: edges.into_values().collect(),
            last_executed: self
                .last_executed
                .iter()
                .map(|(node, superstep)| (node.clone(), *superstep))
                .collect(),
        }
    }
}

/// Compute the Vanguard for the superstep after the one `frontier` was just
/// updated for (ENG-FR-06). Four tiers, each engaged only when every prior
/// tier returned empty:
///
/// 1. **Normal-ready**: every non-deferred node the `Frontier` reports
///    executable, in `graph.edges()`'s stable insertion order (ENG-FR-04),
///    de-duplicated.
/// 2. **Starvation release** ([`starved_release`], ENG-FR-06a, BUG-03):
///    releases a non-deferred node that is starved rather than genuinely
///    blocked -- it already holds at least one fresh fired incoming edge,
///    and every other unresolved incoming edge is `Pending` from a live
///    source that has NEVER executed. Without this tier, a cycle whose only
///    path back to one of its own members is that member's own
///    not-yet-resolved incoming edge can never bootstrap its first
///    execution, and the run reports `Completed` over a node that never
///    ran -- the same truthful-outcome violation BUG-02 fixed by a
///    different mechanism.
/// 3. **Defer release**: every `defer`-marked node the `Frontier` reports
///    executable, ordered by this graph's node registration order
///    (`node_order`) rather than `HashMap` order -- the
///    aggregate-after-all-branches case.
/// 4. **Deferred starvation release** ([`starved_deferred_release`], D-02a):
///    the SAME starvation rule as tier 2, applied to `defer`-marked nodes
///    instead of excluding them. Without this tier, a `defer`-marked
///    aggregator caught in the exact starvation shape tier 2 exists to fix
///    would never be released -- tier 2 deliberately skips deferred nodes
///    so an aggregator still waits for a released cycle node to run first
///    (tiers 2 and 3 are strictly ordered for that reason), but a deferred
///    node starved by its OWN cycle-bootstrap back-edge, not by a sibling
///    it is aggregating after, needs the same rescue tier 2 gives every
///    other node.
///
/// Each tier engages ONLY when every earlier tier is empty, so a diamond
/// join still waits for every incoming edge from a live source that HAS
/// already executed (that node is legitimately waiting, not starved), and a
/// `defer`-marked aggregator still waits for a released cycle node before
/// firing.
fn compute_next_vanguard(graph: &WarGraph, frontier: &Frontier) -> Vec<NodeId> {
    let mut ready = Vec::new();
    let mut seen = HashSet::new();
    for edge in graph.edges() {
        let target = &edge.to;
        if graph.is_deferred(target) || seen.contains(target) {
            continue;
        }
        if frontier.is_ready(graph, target) {
            seen.insert(target.clone());
            ready.push(target.clone());
        }
    }
    if !ready.is_empty() {
        return ready;
    }

    let starved = starved_release(graph, frontier);
    if !starved.is_empty() {
        return starved;
    }

    let mut deferred_ready = Vec::new();
    for node in graph.node_order() {
        if graph.is_deferred(node) && frontier.is_ready(graph, node) {
            deferred_ready.push(node.clone());
        }
    }
    if !deferred_ready.is_empty() {
        return deferred_ready;
    }

    starved_deferred_release(graph, frontier)
}

/// Shared starvation-classification loop behind [`starved_release`] and
/// [`starved_deferred_release`]: a node is released here when
/// `!frontier.dead.contains(node)`, `graph.is_deferred(node) == deferred`
/// (selecting either the non-deferred or the deferred population),
/// [`Frontier::node_edge_summary`] reports at least one fresh fired
/// incoming edge, and every remaining unresolved incoming edge is `Pending`
/// from a live source that has never executed (no entry in
/// `frontier.last_executed`). A node blocked by an unresolved edge from a
/// live source that HAS already executed is not starved -- it is
/// legitimately waiting on that source's NEXT firing, and releasing it
/// would violate join semantics (ENG-FR-06). Iterates `graph.node_order()`,
/// never raw `HashMap`/`HashSet` order (ENG-FR-04), so a simultaneous
/// starvation release is deterministically ordered.
///
/// Introduces no new persisted state: every fact used here is derived from
/// `frontier.edge_state`, `frontier.dead` and `frontier.last_executed`, all
/// rebuilt fresh within a run (D-03).
fn starved_nodes(graph: &WarGraph, frontier: &Frontier, deferred: bool) -> Vec<NodeId> {
    let mut starved = Vec::new();
    for node in graph.node_order() {
        if frontier.dead.contains(node) || graph.is_deferred(node) != deferred {
            continue;
        }
        let Some(summary) = frontier.node_edge_summary(graph, node) else {
            continue;
        };
        if !summary.any_fresh_fire {
            continue;
        }
        let only_never_executed_sources = summary.pending_from_live.iter().all(|&idx| {
            let source = &graph.edges()[idx].from;
            !frontier.last_executed.contains_key(source)
        });
        if only_never_executed_sources {
            starved.push(node.clone());
        }
    }
    starved
}

/// The ENG-FR-06a starvation-release fallback (BUG-03), called by
/// [`compute_next_vanguard`] only when both the normal-ready pass and the
/// defer-release pass return empty. Releases NON-deferred starved nodes --
/// see [`starved_nodes`] for the shared classification rule.
fn starved_release(graph: &WarGraph, frontier: &Frontier) -> Vec<NodeId> {
    starved_nodes(graph, frontier, false)
}

/// D-02a's deferred-node starvation-release tier, called by
/// [`compute_next_vanguard`] only when the normal-ready pass, the
/// non-deferred starvation release, AND the ordinary defer release all
/// return empty. Releases DEFERRED starved nodes -- see [`starved_nodes`]
/// for the shared classification rule. Exists so a `defer`-marked
/// aggregator caught in the same cycle-bootstrap starvation shape
/// [`starved_release`] fixes for ordinary nodes is released too, rather
/// than leaving `superstep::starved_at_completion`'s D-04 check to (rightly)
/// fail the run over a legitimately-declared aggregator.
fn starved_deferred_release(graph: &WarGraph, frontier: &Frontier) -> Vec<NodeId> {
    starved_nodes(graph, frontier, true)
}

/// D-04's run-end truthful-outcome check, called from [`run`] at BOTH
/// places it is about to report `RunOutcome::Completed` -- the mid-loop
/// branch where `compute_next_vanguard` returned empty, and the early
/// return for a Vanguard empty at entry. Returns, in `graph.node_order()`
/// order (ENG-FR-04, never `HashMap`/`HashSet` order), every declared node
/// that is not `frontier.dead` and whose [`Frontier::node_edge_summary`]
/// reports at least one fresh fired incoming edge: work the scheduler was
/// about to walk away from without ever dispatching it.
///
/// Deliberately independent of `compute_next_vanguard` and the
/// `starved_release` fallback it calls (D-04): this re-derives its answer
/// from the SAME `Frontier` state those passes already updated for this
/// superstep, rather than re-invoking their scheduling logic, so a future
/// regression in the release mechanism cannot silently satisfy both the
/// release and this check at once. An empty result is what makes
/// `RunOutcome::Completed` truthful: every declared, non-dead node's
/// incoming edges are either resolved not-firing, resolved fired-and-then-
/// consumed by that node's own subsequent execution, or genuinely still
/// pending from a source that has never run and never will (which is
/// exactly `frontier.dead`'s job to have already caught).
fn starved_at_completion(graph: &WarGraph, frontier: &Frontier) -> Vec<NodeId> {
    let mut starved = Vec::new();
    for node in graph.node_order() {
        if frontier.dead.contains(node) {
            continue;
        }
        if let Some(summary) = frontier.node_edge_summary(graph, node)
            && summary.any_fresh_fire
        {
            starved.push(node.clone());
        }
    }
    starved
}

/// Evaluate an [`EdgeCondition`] for the edge `source -> target`, whose
/// source node completed at `superstep` on `thread` (BUG-01, CF-01).
///
/// `Always`/`Contains`/`Regex` are evaluated against the whole post-merge
/// Battlefield, rendered as its canonical (schema-ordered, `BTreeMap`-backed)
/// JSON string — deterministic by construction, since `Battlefield`'s own
/// `Serialize` impl already guarantees byte-identical output for
/// byte-identical state (ENG-FR-08). `Custom(name)` looks `name` up in
/// `evaluators` (a miss here is unreachable in practice --
/// `WarGraph::validate` already rejected any unregistered `Custom` name
/// before any node executed -- but is still resolved as a fail-closed
/// internal error rather than any default branch, should that invariant
/// ever be violated) and awaits its verdict, passing (D-02): the string
/// value of `source`'s `output_field` (empty string if unset) when `source`
/// is a `NodeSpec::Paladin` node, else the same canonical Battlefield JSON
/// the `Contains`/`Regex` arms render. This is the typed-state analog of
/// `campaign_service.rs::evaluate_edge_condition`, which matches against a
/// single Paladin's string output; here there is no single canonical
/// "output string" per node in the general case, so the Paladin
/// `output_field` value is used when one exists and the whole merged state
/// is the sanest, most general substitute otherwise.
#[allow(clippy::too_many_arguments)]
async fn evaluate_edge_condition(
    condition: &EdgeCondition,
    battlefield: &Battlefield,
    graph: &WarGraph,
    evaluators: &EdgeEvaluatorRegistry,
    source: &NodeId,
    target: &NodeId,
    thread: &ThreadId,
    superstep: u64,
) -> Result<bool, EngineError> {
    match condition {
        EdgeCondition::Always => Ok(true),
        EdgeCondition::Contains(needle) => {
            let rendered = serde_json::to_string(battlefield).unwrap_or_default();
            Ok(rendered.contains(needle.as_str()))
        }
        EdgeCondition::Regex(pattern) => {
            let rendered = serde_json::to_string(battlefield).unwrap_or_default();
            let regex = Regex::new(pattern).map_err(|e| EngineError::InvalidEdgeCondition {
                reason: e.to_string(),
            })?;
            Ok(regex.is_match(&rendered))
        }
        EdgeCondition::Custom(name) => {
            let evaluator = evaluators.get(name).cloned().ok_or_else(|| {
                EngineError::Node(NodeError(format!(
                    "internal error: edge evaluator '{name}' missing after graph validation"
                )))
            })?;
            let output = match graph.node(source) {
                Some(NodeSpec::Paladin { output_field, .. }) => battlefield
                    .get::<String>(output_field)
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                _ => serde_json::to_string(battlefield).unwrap_or_default(),
            };
            let ctx = crate::edge_evaluator::EdgeContext {
                source,
                target,
                battlefield: Some(battlefield),
                thread: Some(thread),
                superstep: Some(superstep),
            };
            evaluator.evaluate(&output, &ctx).await.map_err(|err| {
                EngineError::EdgeEvaluatorFailed {
                    from: source.clone(),
                    to: target.clone(),
                    evaluator: name.clone(),
                    source: err,
                }
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_waypoint(
    thread: &ThreadId,
    parent_waypoint_id: Option<WaypointId>,
    superstep: u64,
    graph: &WarGraph,
    battlefield: &Battlefield,
    vanguard: Vec<NodeId>,
    completed: Vec<NodeExecutionRecord>,
    status: WaypointStatus,
    visit_counts: BTreeMap<NodeId, u32>,
    frontier: FrontierSnapshot,
) -> Waypoint {
    Waypoint {
        thread_id: thread.clone(),
        waypoint_id: WaypointId::generate(),
        parent_waypoint_id,
        superstep,
        graph_fingerprint: graph.fingerprint(),
        battlefield: battlefield.clone(),
        vanguard,
        completed,
        status,
        created_at: Utc::now(),
        schema_version: Waypoint::current_schema_version(),
        visit_counts,
        frontier,
    }
}

/// Persist `waypoint`, honouring `durability`: under `Strict` (the
/// default), a save failure fails the run immediately with
/// `EngineError::WaypointWrite`; under `BestEffort`, it is logged as a
/// warning and the caller proceeds as if the save had succeeded. Emits
/// `TraceEvent::WaypointSaved` (ENG-FR-21) exactly when the save actually
/// succeeded -- a `BestEffort`-swallowed failure is not reported as saved.
async fn persist_waypoint<W: WaypointPort>(
    waypoint_port: &W,
    durability: WaypointDurability,
    waypoint: &Waypoint,
    trace: &Arc<TraceDispatcher>,
) -> Result<(), EngineError> {
    match waypoint_port.save(waypoint).await {
        Ok(()) => {
            trace.emit(TraceEvent::WaypointSaved {
                thread_id: waypoint.thread_id.clone(),
                waypoint_id: waypoint.waypoint_id,
            });
        }
        Err(source) => match durability {
            WaypointDurability::Strict => return Err(EngineError::WaypointWrite { source }),
            WaypointDurability::BestEffort => {
                warn!(
                    "waypoint save failed under BestEffort durability for thread {}: {source}",
                    waypoint.thread_id
                );
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::battlefield_error::BattlefieldError;
    use paladin_core::platform::container::paladin::Paladin;
    use paladin_core::platform::container::waypoint::{ParleyRequest, ThreadId};

    use crate::engine::directive_parser::{DirectiveParser, OnParseError};
    use crate::engine::graph::EdgeSpec;
    use crate::engine::graph::EngineLimits;
    use crate::engine::node::StateNode;
    use crate::engine::test_support::{
        ConcurrencyTrackingNode, CountingFunctionNode, FailingFunctionNode, RecordingPaladinPort,
        RecordingWaypointStore, YieldingNode, shuffle_seeded,
    };

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    fn make_paladin(name: &str) -> Paladin {
        let data = paladin_core::platform::container::paladin::PaladinData {
            name: name.to_string(),
            ..Default::default()
        };
        paladin_core::base::entity::node::Node::new(data, Some(name.to_string()))
    }

    fn schema(fields: Vec<FieldSpec>) -> BattlefieldSchema {
        BattlefieldSchema::new(fields)
    }

    fn no_paladin_port() -> Arc<dyn PaladinPort> {
        Arc::new(RecordingPaladinPort::new())
    }

    fn no_trace() -> Arc<TraceDispatcher> {
        Arc::new(TraceDispatcher::new(None))
    }

    fn no_interceptors() -> Vec<Arc<dyn NodeInterceptor>> {
        Vec::new()
    }

    async fn run_default(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
    ) -> RunOutcome {
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            graph,
            thread,
            Battlefield::initialize(
                graph.schema().clone(),
                &paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap()
    }

    /// Like `run_default`, but over a caller-supplied `PaladinPort` --
    /// `run_default`'s own `no_paladin_port()` scripts no output, so a
    /// `NodeSpec::Paladin` test needs this instead (CF-02, D-11).
    async fn run_with_port(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
        port: &Arc<dyn PaladinPort>,
    ) -> RunOutcome {
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            graph,
            thread,
            Battlefield::initialize(
                graph.schema().clone(),
                &paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            port,
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap()
    }

    // --- CF-02, D-11: DirectiveParser wired into NodeSpec::Paladin dispatch

    #[tokio::test]
    async fn structured_directive_parses_a_bare_json_object_output() {
        let verdict_field = field("verdict");
        let raw_field = field("raw");
        let s = schema(vec![
            FieldSpec::new(verdict_field.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(raw_field.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("judge");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("judge"),
                InputMapping::new("judge"),
                raw_field,
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(node_id);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output(
            "judge",
            r#"{"delta": {"verdict": "approved"}, "next": "edges"}"#,
        );
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("structured-bare-json").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&verdict_field).unwrap(),
                    Some("approved".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn structured_directive_does_not_write_the_output_field() {
        let verdict_field = field("verdict");
        let raw_field = field("raw");
        let s = schema(vec![
            FieldSpec::new(verdict_field, DispatchRule::LastWrite, None, false),
            FieldSpec::new(raw_field.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("judge");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("judge"),
                InputMapping::new("judge"),
                raw_field.clone(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(node_id);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output(
            "judge",
            r#"{"delta": {"verdict": "approved"}, "next": "edges"}"#,
        );
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("structured-no-output-field").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&raw_field).unwrap(),
                    None,
                    "StructuredDirective performs no implicit output_field write"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn structured_directive_goto_routes_the_run() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let router = NodeId::new("router");
        let target = NodeId::new("target");
        graph.add_node(
            router.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("router"),
                InputMapping::new("route"),
                field("raw"),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        let target_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(target.clone(), NodeSpec::Function(target_node.clone()));
        graph.add_entry(router.clone());
        graph.mark_dynamic_target(target.clone());

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("router", r#"{"delta": {}, "next": {"goto": ["target"]}}"#);
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("structured-goto").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(target_node.run_count(), 1, "the Goto target must run");
    }

    #[tokio::test]
    async fn envelope_delta_naming_an_unknown_field_fails_the_run() {
        let verdict_field = field("verdict");
        let s = schema(vec![FieldSpec::new(
            verdict_field,
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("judge");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("judge"),
                InputMapping::new("judge"),
                field("raw"),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(node_id);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output(
            "judge",
            r#"{"delta": {"not_a_real_field": "x"}, "next": "edges"}"#,
        );
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("unknown-delta-field").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Failed {
                error: EngineError::Battlefield(BattlefieldError::UnknownField { field }),
                ..
            } => {
                assert_eq!(field.as_str(), "not_a_real_field");
            }
            other => panic!("expected Failed(Battlefield(UnknownField)), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_output_under_fail_run_fails_the_run() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("judge");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("judge"),
                InputMapping::new("judge"),
                field("raw"),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(node_id.clone());

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("judge", "not json at all");
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("malformed-fail-run").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Failed {
                error: EngineError::DirectiveParseFailed { node, .. },
                ..
            } => {
                assert_eq!(node, node_id);
            }
            other => panic!("expected Failed(DirectiveParseFailed), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_output_under_fallback_plain_writes_the_raw_output() {
        let raw_field = field("raw");
        let s = schema(vec![FieldSpec::new(
            raw_field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("judge");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("judge"),
                InputMapping::new("judge"),
                raw_field.clone(),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FallbackPlain,
                },
            ),
        );
        graph.add_entry(node_id);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("judge", "not json at all");
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("malformed-fallback-plain").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&raw_field).unwrap(),
                    Some("not json at all".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    // --- Task 2 (red/green): the FailRun/FallbackPlain end-to-end proof and
    // the no-partial-merge guarantee, asserted in full through superstep::run.

    #[tokio::test]
    async fn structured_directive_parse_failure_does_not_merge_a_partial_delta() {
        let ok_field = field("ok_field");
        let s = schema(vec![FieldSpec::new(
            ok_field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let succeeding = NodeId::new("succeeding");
        let failing = NodeId::new("failing");
        graph.add_node(
            succeeding.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("succeeding"),
                InputMapping::new("go"),
                field("unused_out"),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_node(
            failing.clone(),
            NodeSpec::paladin_with_directive_parser(
                make_paladin("failing"),
                InputMapping::new("go"),
                field("unused_out2"),
                DirectiveParser::StructuredDirective {
                    on_parse_error: OnParseError::FailRun,
                },
            ),
        );
        graph.add_entry(succeeding);
        graph.add_entry(failing);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output(
            "succeeding",
            r#"{"delta": {"ok_field": "should-not-appear"}, "next": "edges"}"#,
        );
        recording.set_output("failing", "definitely not json");
        let port: Arc<dyn PaladinPort> = recording;

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("partial-delta-no-merge").unwrap();
        let outcome = run_with_port(&graph, thread.clone(), &store, &port).await;

        assert!(
            matches!(
                outcome,
                RunOutcome::Failed {
                    error: EngineError::DirectiveParseFailed { .. },
                    ..
                }
            ),
            "expected Failed(DirectiveParseFailed), got {outcome:?}"
        );

        let waypoints = store.saved_waypoints(&thread).await;
        let failed_waypoint = waypoints
            .iter()
            .find(|w| matches!(w.status, WaypointStatus::Failed { .. }))
            .expect("a Failed waypoint was persisted");
        assert_eq!(
            failed_waypoint
                .battlefield
                .get::<String>(&ok_field)
                .unwrap(),
            None,
            "no delta may be merged when a sibling node's directive fails to parse -- the \
             whole superstep's deltas are discarded together, before merge"
        );
    }

    // --- CF-02: Directive-driven Goto -----------------------------------

    #[tokio::test]
    async fn function_node_goto_sends_control_to_the_named_node_next_superstep() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        let a_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("c")]),
        });
        let b_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let c_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        graph.add_node(b.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_node(c.clone(), NodeSpec::Function(c_node.clone()));
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_entry(a.clone());
        graph.mark_dynamic_target(c.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-basic").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        assert_eq!(c_node.run_count(), 1, "the Goto target must run");
        assert_eq!(
            b_node.run_count(),
            0,
            "the node's own static outgoing edge must not also fire"
        );

        let saved = store.saved_waypoints(&thread).await;
        let first = saved
            .iter()
            .find(|w| w.superstep == 1)
            .expect("superstep 1 waypoint");
        let edge_state = first
            .frontier
            .edges
            .iter()
            .find(|e| e.from == a && e.to == b)
            .expect("a -> b edge state recorded");
        assert!(
            !edge_state.fired,
            "a -> b must resolve NotFiring when a routes via Goto (D-08c)"
        );
    }

    #[tokio::test]
    async fn goto_to_an_undeclared_node_fails_the_run() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let ghost = NodeId::new("ghost");
        let a_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("ghost")]),
        });
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        graph.add_entry(a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-unknown").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::GotoUnknownNode { from, to },
                ..
            } => {
                assert_eq!(from, a);
                assert_eq!(to, ghost);
            }
            other => panic!("expected Failed(GotoUnknownNode), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn goto_only_target_must_be_declared_dynamic() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let c = NodeId::new("c");
        let a_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("c")]),
        });
        let c_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        graph.add_node(c.clone(), NodeSpec::Function(c_node.clone()));
        graph.add_entry(a.clone());

        let err = graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .expect_err("c is reachable only via Goto and not marked dynamic_target");
        assert!(matches!(err, EngineError::UnreachableNode { .. }));

        graph.mark_dynamic_target(c.clone());
        graph
            .validate(
                &CustomDispatchResolver::new(),
                &EdgeEvaluatorRegistry::new(),
            )
            .expect("c is now a declared dynamic target");

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-dynamic-target").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(c_node.run_count(), 1);
    }

    #[tokio::test]
    async fn goto_refine_loop_terminates_on_the_reviewer_verdict() {
        // writer -> reviewer, reviewer Goto(writer)s for its first two runs,
        // then routes via Edges (reviewer has no outgoing edge, so the run
        // completes once it stops looping) -- PRD acceptance 3.
        const REFINE_ROUNDS: usize = 2;
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let writer = NodeId::new("writer");
        let reviewer = NodeId::new("reviewer");
        let writer_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let reviewer_node = CountingFunctionNode::with_directive(|run, _state| {
            let next = if run < REFINE_ROUNDS {
                NextStep::Goto(vec![NodeId::new("writer")])
            } else {
                NextStep::Edges
            };
            Directive {
                delta: StateDelta::new(),
                next,
            }
        });
        graph.add_node(writer.clone(), NodeSpec::Function(writer_node.clone()));
        graph.add_node(reviewer.clone(), NodeSpec::Function(reviewer_node));
        graph.add_edge(EdgeSpec {
            from: writer.clone(),
            to: reviewer.clone(),
            condition: None,
        });
        graph.add_entry(writer);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-refine-loop").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert!(
            writer_node.run_count() > 1,
            "the writer must re-run at least once via Goto, got {}",
            writer_node.run_count()
        );
    }

    #[tokio::test]
    async fn unbounded_goto_loop_trips_the_node_visit_limit() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let a_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("a")]),
        });
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        graph.add_entry(a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-unbounded").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::NodeVisitLimitExceeded { node, limit },
                ..
            } => {
                assert_eq!(node, a);
                assert_eq!(limit, EngineLimits::default().max_node_visits);
            }
            other => panic!("expected Failed(NodeVisitLimitExceeded), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn goto_target_that_is_also_tier_one_ready_is_scheduled_exactly_once() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        let a_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let b_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("c")]),
        });
        let c_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        graph.add_node(b.clone(), NodeSpec::Function(b_node));
        graph.add_node(c.clone(), NodeSpec::Function(c_node.clone()));
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: c.clone(),
            condition: None,
        });
        graph.add_entry(a);
        graph.add_entry(b);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("goto-tier1-both").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            c_node.run_count(),
            1,
            "a node that is both tier-1-ready and a Goto target this superstep must run \
             exactly once"
        );
    }

    // --- CF-02: End semantics, End-over-Goto precedence, typed Parley ---

    #[tokio::test]
    async fn end_completes_the_run_after_the_emitting_superstep_merges() {
        let s = schema(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let ender = NodeId::new("ender");
        let peer = NodeId::new("peer");
        let ender_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::End,
        });
        let peer_node = CountingFunctionNode::fixed(field("result"), serde_json::json!("peer-ran"));
        graph.add_node(ender.clone(), NodeSpec::Function(ender_node));
        graph.add_node(peer.clone(), NodeSpec::Function(peer_node.clone()));
        graph.add_entry(ender);
        graph.add_entry(peer);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("end-basic").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&field("result")).unwrap(),
                    Some("peer-ran".to_string()),
                    "the peer's delta must merge before End completes the run"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(peer_node.run_count(), 1);

        let saved = store.saved_waypoints(&thread).await;
        assert_eq!(
            saved.len(),
            1,
            "no superstep after the one End fired in must run"
        );
    }

    #[tokio::test]
    async fn end_beats_goto_in_the_same_superstep() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let ender = NodeId::new("ender");
        let gotoer = NodeId::new("gotoer");
        let c = NodeId::new("c");
        let ender_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::End,
        });
        let gotoer_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("c")]),
        });
        let c_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(ender.clone(), NodeSpec::Function(ender_node));
        graph.add_node(gotoer.clone(), NodeSpec::Function(gotoer_node));
        graph.add_node(c.clone(), NodeSpec::Function(c_node.clone()));
        graph.add_entry(ender);
        graph.add_entry(gotoer);
        graph.mark_dynamic_target(c.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("end-beats-goto").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(
            c_node.run_count(),
            0,
            "End must win over a peer's Goto in the same superstep"
        );
    }

    #[tokio::test]
    async fn end_terminated_run_does_not_trip_the_starvation_completion_check() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let x = NodeId::new("x");
        let d = NodeId::new("d");
        let ender = NodeId::new("ender");
        let x_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let d_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let ender_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::End,
        });
        graph.add_node(x.clone(), NodeSpec::Function(x_node));
        graph.add_node(d.clone(), NodeSpec::Function(d_node.clone()));
        graph.add_node(ender.clone(), NodeSpec::Function(ender_node));
        graph.add_edge(EdgeSpec {
            from: x.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_entry(x);
        graph.add_entry(ender);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("end-suppresses-starvation").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "End must complete the run even though d's fired incoming edge from x is never \
             consumed: got {outcome:?}"
        );
        assert_eq!(
            d_node.run_count(),
            0,
            "d must never run -- End short-circuits before its superstep"
        );
    }

    #[tokio::test]
    async fn starvation_completion_check_still_fires_when_no_node_ended_the_run() {
        // The entry-vanguard-empty variant of D-04's check (superstep.rs's
        // top-of-`run` branch): seed a FrontierSnapshot whose entry -> d
        // edge is already fired but never consumed (d never executed), pass
        // an EMPTY vanguard, and confirm the check still fails the run when
        // no node in this call emits `NextStep::End` at all (nothing runs
        // here -- the vanguard is empty from the start).
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let entry = NodeId::new("entry");
        let d = NodeId::new("d");
        graph.add_node(
            entry.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        graph.add_node(
            d.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        graph.add_edge(EdgeSpec {
            from: entry.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_entry(entry.clone());

        let snapshot = FrontierSnapshot {
            edges: vec![FrontierEdgeState {
                from: entry.clone(),
                to: d.clone(),
                condition: canonical_edge_condition(&None),
                fired: true,
                resolved_at: 1,
            }],
            last_executed: BTreeMap::from([(entry, 1)]),
        };

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("starvation-still-fires").unwrap();
        let outcome = run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread,
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
            Vec::new(),
            BTreeMap::new(),
            Some(snapshot),
            None,
            2,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Failed {
                error: EngineError::StarvedNodeAtCompletion { nodes, .. },
                ..
            } => {
                assert_eq!(nodes, vec![d]);
            }
            other => panic!("expected Failed(StarvedNodeAtCompletion), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn which_node_ended_the_run_is_observable_from_the_waypoint() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let ender = NodeId::new("ender");
        let ender_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::End,
        });
        graph.add_node(ender.clone(), NodeSpec::Function(ender_node));
        graph.add_entry(ender.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("end-observable").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        let wp = saved
            .iter()
            .find(|w| w.superstep == 1)
            .expect("superstep 1 waypoint");
        let record = wp
            .completed
            .iter()
            .find(|r| r.node_id == ender)
            .expect("ender's execution record");
        assert_eq!(record.outcome, NodeOutcomeKind::Ended);
    }

    #[tokio::test]
    async fn parley_returned_this_phase_fails_the_run() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let asker = NodeId::new("asker");
        let asker_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(ParleyRequest {
                prompt: "need input".to_string(),
            }),
        });
        graph.add_node(asker.clone(), NodeSpec::Function(asker_node));
        graph.add_entry(asker.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("parley-unsupported").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::ParleyNotSupported { node },
                ..
            } => {
                assert_eq!(node, asker);
            }
            other => panic!("expected Failed(ParleyNotSupported), got {other:?}"),
        }

        let saved = store.saved_waypoints(&thread).await;
        assert!(
            saved
                .iter()
                .all(|w| !matches!(w.status, WaypointStatus::AwaitingInput { .. })),
            "no AwaitingInput waypoint may be written for an unsupported Parley"
        );
        assert!(
            !saved
                .iter()
                .any(|w| matches!(w.status, WaypointStatus::Completed)),
            "the run must not report Completed"
        );
    }

    // --- CF-03: Muster dynamic fan-out (Plan 23-05).

    fn muster_task(worker: &NodeId, payload: serde_json::Value, task_key: &str) -> MusterTask {
        MusterTask {
            worker: worker.clone(),
            payload,
            task_key: task_key.to_string(),
        }
    }

    #[tokio::test]
    async fn planner_musters_three_workers_that_all_run_in_one_superstep() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let decoy = NodeId::new("decoy");
        let worker_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let decoy_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!("a"), "a"),
                    muster_task(&worker, serde_json::json!("b"), "b"),
                    muster_task(&worker, serde_json::json!("c"), "c"),
                ]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node.clone()));
        graph.add_node(decoy.clone(), NodeSpec::Function(decoy_node.clone()));
        graph.add_edge(EdgeSpec {
            from: planner.clone(),
            to: decoy.clone(),
            condition: None,
        });
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-basic").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        assert_eq!(worker_node.run_count(), 3, "all three tasks must run");
        assert_eq!(
            decoy_node.run_count(),
            0,
            "the planner's own static outgoing edge must not also fire (D-08c)"
        );

        let saved = store.saved_waypoints(&thread).await;
        let muster_superstep = saved
            .iter()
            .find(|w| w.superstep == 2)
            .expect("superstep 2 (the muster superstep) waypoint");
        let worker_records = muster_superstep
            .completed
            .iter()
            .filter(|r| r.node_id == worker)
            .count();
        assert_eq!(
            worker_records, 3,
            "all three worker tasks must be recorded as having run in the same superstep"
        );
    }

    #[tokio::test]
    async fn worker_deltas_merge_in_task_key_order_not_completion_order() {
        // Each worker sleeps for a duration INVERSELY related to its
        // task_key, so real completion order is c, b, a -- the opposite of
        // lexicographic task_key order -- yet the merged "order" field must
        // still read ["a", "b", "c"].
        struct DelayedWorkerNode {
            field: FieldName,
        }
        #[async_trait::async_trait]
        impl StateNode for DelayedWorkerNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &crate::engine::node::NodeContext,
            ) -> Result<Directive, NodeError> {
                let key = ctx.task_key().unwrap_or_default().to_string();
                let delay_ms = match key.as_str() {
                    "a" => 30,
                    "b" => 15,
                    _ => 0,
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                let mut delta = StateDelta::new();
                delta.set_raw(self.field.clone(), serde_json::json!(key));
                Ok(delta.into())
            }
        }

        let order_field = field("order");
        let s = schema(vec![FieldSpec::new(
            order_field.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!("a"), "a"),
                    muster_task(&worker, serde_json::json!("b"), "b"),
                    muster_task(&worker, serde_json::json!("c"), "c"),
                ]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::Function(std::sync::Arc::new(DelayedWorkerNode {
                field: order_field.clone(),
            })),
        );
        graph.add_entry(planner);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-order").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<Vec<String>>(&order_field).unwrap(),
                    Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
                    "deltas must merge in task_key order regardless of completion order"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn each_worker_sees_only_its_own_payload() {
        let seen_field = field("seen");
        let s = schema(vec![FieldSpec::new(
            seen_field.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!("payload-a"), "a"),
                    muster_task(&worker, serde_json::json!("payload-b"), "b"),
                    muster_task(&worker, serde_json::json!("payload-c"), "c"),
                ]),
            })
        };
        let worker_node = {
            let seen_field = seen_field.clone();
            CountingFunctionNode::with_context_directive(move |_run, _state, ctx| {
                let mut delta = StateDelta::new();
                delta.set_raw(
                    seen_field.clone(),
                    serde_json::json!({
                        "task_key": ctx.task_key(),
                        "payload": ctx.muster_payload(),
                    }),
                );
                delta.into()
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node));
        graph.add_entry(planner);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-payload-isolation").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let seen = final_state
                    .get::<Vec<serde_json::Value>>(&seen_field)
                    .unwrap()
                    .unwrap();
                assert_eq!(seen.len(), 3);
                for entry in &seen {
                    let task_key = entry["task_key"].as_str().unwrap();
                    let expected_payload = format!("payload-{task_key}");
                    assert_eq!(
                        entry["payload"].as_str().unwrap(),
                        expected_payload,
                        "each worker must see only its own payload, never a sibling's"
                    );
                }
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn muster_payload_never_enters_the_battlefield() {
        let ran_field = field("ran");
        let s = schema(vec![FieldSpec::new(
            ran_field.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        const MARKERS: [&str; 3] = [
            "SECRET_PAYLOAD_MARKER_A",
            "SECRET_PAYLOAD_MARKER_B",
            "SECRET_PAYLOAD_MARKER_C",
        ];
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!(MARKERS[0]), "a"),
                    muster_task(&worker, serde_json::json!(MARKERS[1]), "b"),
                    muster_task(&worker, serde_json::json!(MARKERS[2]), "c"),
                ]),
            })
        };
        let worker_node = {
            let ran_field = ran_field.clone();
            // Deliberately never writes the payload anywhere -- only its
            // task_key -- so the marker strings can appear ONLY if the
            // engine itself leaked the payload into the Battlefield.
            CountingFunctionNode::with_context_directive(move |_run, _state, ctx| {
                let mut delta = StateDelta::new();
                delta.set_raw(ran_field.clone(), serde_json::json!(ctx.task_key()));
                delta.into()
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node));
        graph.add_entry(planner);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-no-leak").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let serialized = serde_json::to_string(&final_state).unwrap();
                for marker in MARKERS {
                    assert!(
                        !serialized.contains(marker),
                        "payload marker {marker} must never reach the Battlefield"
                    );
                }
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn deferred_aggregator_runs_once_after_every_task_resolves() {
        let results_field = field("results");
        let aggregated_field = field("aggregated");
        let s = schema(vec![
            FieldSpec::new(results_field.clone(), DispatchRule::Append, None, false),
            FieldSpec::new(
                aggregated_field.clone(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let aggregator = NodeId::new("aggregator");
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!("a"), "a"),
                    muster_task(&worker, serde_json::json!("b"), "b"),
                    muster_task(&worker, serde_json::json!("c"), "c"),
                ]),
            })
        };
        let worker_node = {
            let results_field = results_field.clone();
            CountingFunctionNode::with_context_directive(move |_run, _state, ctx| {
                let mut delta = StateDelta::new();
                delta.set_raw(results_field.clone(), serde_json::json!(ctx.task_key()));
                delta.into()
            })
        };
        let aggregator_node = {
            let results_field = results_field.clone();
            let aggregated_field = aggregated_field.clone();
            CountingFunctionNode::new(move |_run, state| {
                let results = state
                    .get::<Vec<String>>(&results_field)
                    .unwrap()
                    .unwrap_or_default();
                let mut delta = StateDelta::new();
                delta.set_raw(aggregated_field.clone(), serde_json::json!(results));
                delta
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node));
        graph.add_deferred_node(
            aggregator.clone(),
            NodeSpec::Function(aggregator_node.clone()),
        );
        graph.add_edge(EdgeSpec {
            from: worker.clone(),
            to: aggregator.clone(),
            condition: None,
        });
        graph.add_entry(planner);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-defer-aggregate").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<Vec<String>>(&aggregated_field).unwrap(),
                    Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
                    "the aggregator must see exactly three results in task_key order"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(
            aggregator_node.run_count(),
            1,
            "the deferred aggregator must run exactly once"
        );

        let saved = store.saved_waypoints(&thread).await;
        let worker_superstep = saved
            .iter()
            .filter(|w| w.completed.iter().any(|r| r.node_id == worker))
            .map(|w| w.superstep)
            .max()
            .expect("a superstep in which the worker ran");
        let aggregator_superstep = saved
            .iter()
            .find(|w| w.completed.iter().any(|r| r.node_id == aggregator))
            .map(|w| w.superstep)
            .expect("a superstep in which the aggregator ran");
        assert!(
            aggregator_superstep > worker_superstep,
            "the aggregator ({aggregator_superstep}) must run strictly after the workers \
             ({worker_superstep})"
        );
    }

    #[tokio::test]
    async fn empty_entry_vanguard_completes_immediately_with_one_waypoint() {
        let graph = WarGraph::new(schema(vec![]), EngineLimits::default());
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("empty-entry").unwrap();

        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].status, WaypointStatus::Completed);
        assert!(saved[0].completed.is_empty());
        assert!(saved[0].vanguard.is_empty());
    }

    #[tokio::test]
    async fn three_superstep_linear_run_persists_three_waypoints_with_parent_chain() {
        let s = schema(vec![FieldSpec::new(
            field("result"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        graph.add_node(
            a.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("result"),
                serde_json::json!("a"),
            )),
        );
        graph.add_node(
            b.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("result"),
                serde_json::json!("b"),
            )),
        );
        graph.add_node(
            c.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("result"),
                serde_json::json!("c"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: b.clone(),
            to: c.clone(),
            condition: None,
        });
        graph.add_entry(a);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("linear-3").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        assert_eq!(saved.len(), 3);
        let mut by_superstep = saved.clone();
        by_superstep.sort_by_key(|w| w.superstep);
        assert_eq!(
            by_superstep.iter().map(|w| w.superstep).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(by_superstep[0].parent_waypoint_id, None);
        assert_eq!(
            by_superstep[1].parent_waypoint_id,
            Some(by_superstep[0].waypoint_id)
        );
        assert_eq!(
            by_superstep[2].parent_waypoint_id,
            Some(by_superstep[1].waypoint_id)
        );
        assert_eq!(by_superstep[2].status, WaypointStatus::Completed);
        assert!(by_superstep[2].vanguard.is_empty());
    }

    #[tokio::test]
    async fn peer_node_observes_pre_superstep_value_not_siblings_write() {
        let s = schema(vec![
            FieldSpec::new(
                field("x"),
                DispatchRule::LastWrite,
                Some(serde_json::json!("orig")),
                false,
            ),
            FieldSpec::new(field("y"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let writer = NodeId::new("writer");
        let reader = NodeId::new("reader");
        graph.add_node(
            writer.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("x"),
                serde_json::json!("new"),
            )),
        );
        graph.add_node(
            reader.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_n, state| {
                let observed = state
                    .get::<String>(&field("x"))
                    .unwrap()
                    .unwrap_or_default();
                let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
                d.set(field("y"), observed).unwrap();
                d
            })),
        );
        graph.add_entry(writer);
        graph.add_entry(reader);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("isolation").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&field("x")).unwrap(),
                    Some("new".to_string())
                );
                assert_eq!(
                    final_state.get::<String>(&field("y")).unwrap(),
                    Some("orig".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn battlefield_cloned_once_per_superstep_arc_ptr_eq() {
        // Two entry nodes writing DISTINCT fields, so the merge never hits
        // LastWrite's two-distinct-writer DispatchConflict path -- this test
        // is purely a vehicle for running two nodes in one superstep and
        // comparing the raw pointer address of the Battlefield snapshot
        // each one observed.
        let s = schema(vec![
            FieldSpec::new(field("x"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("y"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let node_a = CountingFunctionNode::fixed(field("x"), serde_json::json!("a"));
        let node_b = CountingFunctionNode::fixed(field("y"), serde_json::json!("b"));
        graph.add_node(a.clone(), NodeSpec::Function(node_a.clone()));
        graph.add_node(b.clone(), NodeSpec::Function(node_b.clone()));
        graph.add_entry(a);
        graph.add_entry(b);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("ptr-eq").unwrap();
        let _ = run_default(&graph, thread, &store).await;

        let a_ptrs = node_a.observed_ptrs();
        let b_ptrs = node_b.observed_ptrs();
        assert_eq!(a_ptrs.len(), 1);
        assert_eq!(b_ptrs.len(), 1);
        assert_eq!(
            a_ptrs[0], b_ptrs[0],
            "both nodes must observe the same Arc-shared snapshot"
        );
    }

    #[tokio::test]
    async fn strict_durability_fails_run_on_save_failure() {
        let graph = WarGraph::new(schema(vec![]), EngineLimits::default());
        let store = RecordingWaypointStore::new();
        store.fail_next_save();
        let thread = ThreadId::new("strict-fail").unwrap();

        let result = run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await;

        assert!(matches!(result, Err(EngineError::WaypointWrite { .. })));
    }

    #[tokio::test]
    async fn best_effort_durability_continues_past_save_failure() {
        let graph = WarGraph::new(schema(vec![]), EngineLimits::default());
        let store = RecordingWaypointStore::new();
        store.fail_next_save();
        let thread = ThreadId::new("best-effort").unwrap();

        let result = run(
            &store,
            WaypointDurability::BestEffort,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap();

        assert!(matches!(result, RunOutcome::Completed { .. }));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parallelism_limit_bounds_in_flight_execution() {
        // Append, not LastWrite: four concurrent entry nodes all touch this
        // field, and LastWrite hard-conflicts on 2+ distinct writers.
        let s = schema(vec![FieldSpec::new(
            field("x"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut entries = Vec::new();
        for i in 0..4 {
            let id = NodeId::new(format!("n{i}"));
            graph.add_node(
                id.clone(),
                NodeSpec::Function(ConcurrencyTrackingNode::new(
                    field("x"),
                    serde_json::json!(i),
                    in_flight.clone(),
                    max_seen.clone(),
                    std::time::Duration::from_millis(30),
                )),
            );
            graph.add_entry(id.clone());
            entries.push(id);
        }

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("parallelism").unwrap();
        let outcome = run(
            &store,
            WaypointDurability::Strict,
            Some(2),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert!(
            max_seen.load(std::sync::atomic::Ordering::SeqCst) <= 2,
            "no more than the parallelism limit should run concurrently"
        );
    }

    // --- Task 3: bounded iteration -----------------------------------

    /// `a` is this graph's only node, self-looping and declared entry.
    /// Readiness dodge, not a strandedness workaround (Phase 22 Plan 16
    /// audit, `22-deferred-items.md`): with no other node to be fed by, `a`'s
    /// self-loop is its sole incoming edge, and [`Frontier::is_ready`] leaves
    /// a self-loop edge `Pending` until the node has run once -- a non-entry
    /// `a` could never take its first turn regardless of reachability.
    /// Declaring it entry is what bootstraps it; `a` would satisfy BUG-02's
    /// eligible-set check either way, since entry nodes are always eligible.
    /// BUG-03's starvation-release fix ([`starved_release`]) does not apply
    /// to this shape either: it releases a cycle node that already holds a
    /// fresh fired edge from OUTSIDE the cycle, and `a` has no such edge --
    /// its only incoming edge is its own self-loop.
    fn self_loop_graph(node: Arc<CountingFunctionNode>, limits: EngineLimits) -> WarGraph {
        let s = schema(vec![FieldSpec::new(
            field("status"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, limits);
        let a = NodeId::new("a");
        graph.add_node(a.clone(), NodeSpec::Function(node));
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: a.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(a);
        graph
    }

    #[tokio::test]
    async fn self_loop_runs_exactly_three_times_when_approved_on_third_visit() {
        let node = CountingFunctionNode::new(|run_index, _state| {
            let status = if run_index == 2 {
                "approved"
            } else {
                "looping"
            };
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), status).unwrap();
            d
        });
        let graph = self_loop_graph(
            node.clone(),
            EngineLimits {
                max_node_visits: 5,
                ..EngineLimits::default()
            },
        );
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("self-loop-approved").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 3);
    }

    #[tokio::test]
    async fn self_loop_never_approved_trips_node_visit_limit_at_five() {
        let node = CountingFunctionNode::new(|_run_index, _state| {
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), "looping").unwrap();
            d
        });
        let graph = self_loop_graph(
            node.clone(),
            EngineLimits {
                max_node_visits: 5,
                ..EngineLimits::default()
            },
        );
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("self-loop-never-approved").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(
                    error,
                    EngineError::NodeVisitLimitExceeded { limit: 5, .. }
                ));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        // Only max_node_visits - 1 = 4 actual executions are allowed; the
        // 5th attempt trips before the node runs again.
        assert_eq!(node.run_count(), 4);

        let saved = store.saved_waypoints(&thread).await;
        let failed = saved.first().unwrap();
        assert!(matches!(failed.status, WaypointStatus::Failed { .. }));
        assert_eq!(failed.visit_counts.get(&NodeId::new("a")), Some(&4));
    }

    #[tokio::test]
    async fn self_loop_at_four_visits_does_not_trip() {
        // The mirror of the "at limit" trip above: a node approved on
        // exactly its 4th visit (max_node_visits - 1) must complete
        // normally, never tripping the limit.
        let node = CountingFunctionNode::new(|run_index, _state| {
            let status = if run_index == 3 {
                "approved"
            } else {
                "looping"
            };
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), status).unwrap();
            d
        });
        let graph = self_loop_graph(
            node.clone(),
            EngineLimits {
                max_node_visits: 5,
                ..EngineLimits::default()
            },
        );
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("self-loop-four-visits").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 4);
    }

    /// **BUG-03** (found during the Phase 22 Plan 16 fixture audit,
    /// `22-deferred-items.md`): a node that is BOTH self-looping AND fed by
    /// a separate upstream edge can never take its first turn, and the run
    /// still reports `Completed` regardless.
    ///
    /// Mechanism: `Frontier::is_ready` requires every incoming edge of a
    /// node to be resolved (not `Pending`) before that node is scheduled.
    /// A node's own self-edge is `Pending` until the node has executed at
    /// least once. So `b` here -- reachable from `entry` by a normal edge,
    /// AND self-looping -- has two incoming edges: `entry -> b` (which
    /// fires once `entry` runs) and `b -> b` (which stays `Pending`
    /// forever, since nothing but `b`'s own first run could resolve it).
    /// `is_ready` requires ALL incoming edges resolved, so `b` can never be
    /// placed in a Vanguard: its self-edge blocks the very first execution
    /// that would resolve it.
    ///
    /// This is the SAME truthful-outcome violation as BUG-02 (a
    /// `RunOutcome::Completed` reported over a node that never ran) reached
    /// by a DIFFERENT mechanism. Plan 22-15's eligible-set reachability
    /// check (`WarGraph::validate`) does NOT and CANNOT catch it: `b` is
    /// statically reachable from `entry` over a declared edge, so
    /// `validate` accepts this graph cleanly (asserted below) -- the defect
    /// is a property of `Frontier::is_ready`'s RUNTIME readiness
    /// computation, not of static reachability.
    ///
    /// Fixed by ENG-FR-06a's starvation-release fallback pass in
    /// `compute_next_vanguard` (D-03): when neither the normal-ready pass
    /// nor the defer-release pass has anything to schedule, a node blocked
    /// only by its own not-yet-resolved incoming edges from live,
    /// never-executed sources is released anyway, rather than the run
    /// silently reporting `Completed` over it. The assertions below
    /// describe correct behaviour and, with the fix landed, now pass.
    #[tokio::test]
    async fn self_looping_node_fed_by_upstream_edge_can_never_take_first_turn() {
        let s = schema(vec![
            FieldSpec::new(field("entry_ran"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("status"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let entry_id = NodeId::new("entry");
        let b_id = NodeId::new("b");

        let entry_node = CountingFunctionNode::fixed(field("entry_ran"), serde_json::json!(true));
        // Bounded so a correct engine would terminate: "looping" on its
        // first run, "done" from its second run on -- if `b` could ever
        // take a first turn, this self-loop would resolve after two visits.
        let b_node = CountingFunctionNode::new(|run_index, _state| {
            let status = if run_index == 0 { "looping" } else { "done" };
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), status).unwrap();
            d
        });

        graph.add_node(entry_id.clone(), NodeSpec::Function(entry_node));
        graph.add_node(b_id.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_edge(EdgeSpec {
            from: entry_id.clone(),
            to: b_id.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: b_id.clone(),
            to: b_id.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(entry_id);

        // The defect survives Plan 22-15's fix: `b` is statically reachable
        // from `entry` over a declared edge, so eligible-set validation
        // accepts this graph. This is NOT a reachability problem.
        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok(),
            "b is reachable from entry over a static edge, so validate() must accept this \
             graph -- the defect this test reproduces is a runtime readiness problem, not a \
             reachability one"
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("readiness-defect-repro").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        // Correct behaviour: `b` must execute at least once before the run
        // can complete, since it is legitimately reachable and its self-loop
        // is bounded to resolve after two visits.
        assert!(
            b_node.run_count() >= 1,
            "b must execute at least once -- it is reachable from entry and its self-loop is \
             bounded to terminate, so a correct engine schedules it; got run_count() == 0, \
             meaning Frontier::is_ready never placed it in any Vanguard"
        );
        // Correct behaviour: the run must never report Completed while a
        // reachable, never-executed node's visit count is zero.
        assert!(
            !(matches!(outcome, RunOutcome::Completed { .. }) && b_node.run_count() == 0),
            "the run must not report Completed while b's run_count() is 0 -- that is the exact \
             truthful-outcome violation BUG-02 fixed by a different mechanism; got outcome = \
             {outcome:?}, b.run_count() = {}",
            b_node.run_count()
        );
    }

    /// **BUG-03**, general (non-self-loop) shape: `entry -> a`, `a -> b`,
    /// `b -> a`. `a` is fed both from outside the cycle (`entry`) and from
    /// inside it (`b`'s back-edge) -- the same starvation shape as the
    /// self-loop reproduction above, but with two distinct nodes forming
    /// the cycle rather than one node looping to itself. `a`'s incoming
    /// edges are `entry -> a` (fires once `entry` runs) and `b -> a`
    /// (stays `Pending` until `b` runs -- but `b` cannot run until `a`
    /// runs first, since `a -> b` is `a`'s only outgoing edge into the
    /// cycle). Before the fix, `is_ready(a)` requires both edges resolved,
    /// so `a` never runs and neither does `b`; the run still reports
    /// `Completed`.
    ///
    /// Only `entry` is a declared entry node -- `a` and `b` are ordinary
    /// non-entry nodes, reachable only through the cycle's own edges.
    #[tokio::test]
    async fn cycle_node_fed_from_outside_the_cycle_takes_its_first_turn() {
        let s = schema(vec![
            FieldSpec::new(field("entry_ran"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("status"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let entry_id = NodeId::new("entry");
        let a_id = NodeId::new("a");
        let b_id = NodeId::new("b");

        let entry_node = CountingFunctionNode::fixed(field("entry_ran"), serde_json::json!(true));
        let a_node = CountingFunctionNode::fixed(field("status"), serde_json::json!("a-ran"));
        // Bounded so a correct engine terminates: "looping" on b's first
        // run, "done" from its second run on -- exactly the self-loop
        // reproduction's bounding style above, applied to the back-edge
        // `b -> a` instead of a self-edge.
        let b_node = CountingFunctionNode::new(|run_index, _state| {
            let status = if run_index == 0 { "looping" } else { "done" };
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), status).unwrap();
            d
        });

        graph.add_node(entry_id.clone(), NodeSpec::Function(entry_node));
        graph.add_node(a_id.clone(), NodeSpec::Function(a_node.clone()));
        graph.add_node(b_id.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_edge(EdgeSpec {
            from: entry_id.clone(),
            to: a_id.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: a_id.clone(),
            to: b_id.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: b_id.clone(),
            to: a_id.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(entry_id);

        // The shape is statically legal: `a` is reachable from `entry` over
        // a declared edge (`validate_accepts_two_node_cycle` in `graph.rs`
        // already proves the two-node-cycle topology validates on its own).
        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok(),
            "a is reachable from entry over a static edge, so validate() must accept this \
             graph -- the defect this test reproduces is a runtime readiness problem, not a \
             reachability one"
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("cycle-bootstrap-general-repro").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        // Correct behaviour: both `a` and `b` must execute at least once
        // before the run can complete -- both are legitimately reachable
        // and the cycle is bounded to resolve after two round trips.
        assert!(
            a_node.run_count() >= 1,
            "a must execute at least once -- it is reachable from entry and the cycle it \
             anchors is bounded to terminate, so a correct engine schedules it; got \
             run_count() == 0, meaning Frontier::is_ready never placed it in any Vanguard"
        );
        assert!(
            b_node.run_count() >= 1,
            "b must execute at least once -- it is reachable from a and the cycle is bounded \
             to terminate, so a correct engine schedules it; got run_count() == 0"
        );
        // Correct behaviour: the run must never report Completed while a
        // reachable, never-executed node's visit count is zero.
        assert!(
            !(matches!(outcome, RunOutcome::Completed { .. })
                && (a_node.run_count() == 0 || b_node.run_count() == 0)),
            "the run must not report Completed while a or b's run_count() is 0 -- that is the \
             exact truthful-outcome violation BUG-02 fixed by a different mechanism; got \
             outcome = {outcome:?}, a.run_count() = {}, b.run_count() = {}",
            a_node.run_count(),
            b_node.run_count()
        );
    }

    // --- D-04 / D-02a: run-end truthful-outcome check and the deferred-
    // node starvation tier (Phase 22.1 Plan 02).

    #[test]
    fn completion_check_names_every_node_holding_an_unconsumed_fired_edge() {
        // Hand-constructed Frontier state, exercising `starved_at_completion`
        // directly rather than through a full run -- entry -> b (idx 0) and
        // entry -> c (idx 1), with both edges marked Fired but neither
        // target ever executed. Both non-entry, non-dead nodes must be
        // named, in node_order order.
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let entry = NodeId::new("entry");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        graph.add_node(
            entry.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("entry"),
            )),
        );
        graph.add_node(
            b.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("b"),
            )),
        );
        graph.add_node(
            c.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("c"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: entry.clone(),
            to: b.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: entry.clone(),
            to: c.clone(),
            condition: None,
        });
        graph.add_entry(entry.clone());

        let mut frontier = Frontier::new(&graph);
        frontier.edge_state[0] = EdgeState::Fired(1);
        frontier.edge_state[1] = EdgeState::Fired(1);

        assert_eq!(
            starved_at_completion(&graph, &frontier),
            vec![b.clone(), c.clone()],
            "every node holding an unconsumed fired incoming edge must be named, in \
             node_order order"
        );

        // The state at the end of a normal completed run: both targets
        // executed, consuming their fired edges' freshness (their
        // `last_executed` now postdates the superstep the edge fired at).
        frontier.last_executed.insert(b.clone(), 2);
        frontier.last_executed.insert(c.clone(), 2);
        assert!(
            starved_at_completion(&graph, &frontier).is_empty(),
            "a normally completed run's final frontier must report no starved nodes"
        );
    }

    #[tokio::test]
    async fn deferred_aggregator_starved_by_a_cycle_is_still_released() {
        // The exact self-loop cycle-bootstrap starvation shape
        // `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`
        // reproduces above, but with the cycle node registered `defer`
        // instead of plain: `starved_release` (tier 2) deliberately skips
        // deferred nodes, so without the D-02a deferred-starvation tier
        // (tier 4), this aggregator would never be released and the run
        // would fail with `StarvedNodeAtCompletion` instead of completing.
        let s = schema(vec![
            FieldSpec::new(field("entry_ran"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("status"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let entry_id = NodeId::new("entry");
        let agg_id = NodeId::new("agg");

        let entry_node = CountingFunctionNode::fixed(field("entry_ran"), serde_json::json!(true));
        // Bounded so a correct engine terminates: "looping" on its first
        // run, "done" from its second run on.
        let agg_node = CountingFunctionNode::new(|run_index, _state| {
            let status = if run_index == 0 { "looping" } else { "done" };
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(field("status"), status).unwrap();
            d
        });

        graph.add_node(entry_id.clone(), NodeSpec::Function(entry_node));
        graph.add_deferred_node(agg_id.clone(), NodeSpec::Function(agg_node.clone()));
        graph.add_edge(EdgeSpec {
            from: entry_id.clone(),
            to: agg_id.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: agg_id.clone(),
            to: agg_id.clone(),
            condition: Some(EdgeCondition::Contains("looping".to_string())),
        });
        graph.add_entry(entry_id);

        assert!(
            graph
                .validate(
                    &CustomDispatchResolver::new(),
                    &EdgeEvaluatorRegistry::new()
                )
                .is_ok(),
            "agg is reachable from entry over a static edge, so validate() must accept this \
             graph"
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("deferred-starvation").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("a starved deferred aggregator must not deadlock");

        assert!(
            agg_node.run_count() >= 1,
            "the deferred aggregator must execute at least once despite being starved by its \
             own cycle-bootstrap back-edge; got run_count() == 0"
        );
        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "a starved deferred aggregator that IS released by the D-02a tier must complete \
             normally, not report StarvedNodeAtCompletion; got outcome = {outcome:?}"
        );
    }

    fn linear_chain_graph(length: usize, limits: EngineLimits) -> WarGraph {
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, limits);
        let ids: Vec<NodeId> = (0..length).map(|i| NodeId::new(format!("n{i}"))).collect();
        for id in &ids {
            graph.add_node(
                id.clone(),
                NodeSpec::Function(CountingFunctionNode::fixed(
                    field("log"),
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
        graph
    }

    #[tokio::test]
    async fn chain_needing_max_supersteps_minus_one_completes_normally() {
        // max_supersteps = 3 allows exactly 2 (= 3 - 1) supersteps; a
        // 2-node chain needs exactly 2.
        let graph = linear_chain_graph(
            2,
            EngineLimits {
                max_supersteps: 3,
                ..EngineLimits::default()
            },
        );
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("chain-limit-minus-one").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
    }

    #[tokio::test]
    async fn chain_needing_max_supersteps_trips_recursion_limit() {
        // max_supersteps = 3 allows exactly 2; a 3-node chain needs 3 and
        // must trip at exactly superstep 3.
        let graph = linear_chain_graph(
            3,
            EngineLimits {
                max_supersteps: 3,
                ..EngineLimits::default()
            },
        );
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("chain-trips").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(
                    error,
                    EngineError::RecursionLimitExceeded { limit: 3, .. }
                ));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }

        let saved = store.saved_waypoints(&thread).await;
        let failed = saved.first().unwrap();
        assert!(matches!(failed.status, WaypointStatus::Failed { .. }));
        // Only n0 and n1 (superstep 1 and 2) ever ran; n2 (which would need
        // superstep 3) never got the chance to be visited.
        assert_eq!(failed.visit_counts.get(&NodeId::new("n0")), Some(&1));
        assert_eq!(failed.visit_counts.get(&NodeId::new("n1")), Some(&1));
        assert_eq!(failed.visit_counts.get(&NodeId::new("n2")), None);
    }

    #[tokio::test]
    async fn node_execution_error_fails_run_with_failed_waypoint() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        graph.add_node(
            a.clone(),
            NodeSpec::Function(FailingFunctionNode::new("boom")),
        );
        graph.add_entry(a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("node-failure").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(error, EngineError::Node(_)));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(store.save_call_count(), 1);
        let saved = store.saved_waypoints(&thread).await;
        assert!(matches!(saved[0].status, WaypointStatus::Failed { .. }));
    }

    // --- Task 1: join, defer and not-firing frontier semantics ----------

    fn diamond_graph(
        a: Arc<CountingFunctionNode>,
        b: Arc<CountingFunctionNode>,
        c: Arc<CountingFunctionNode>,
        d: Arc<CountingFunctionNode>,
        a_to_c_condition: Option<EdgeCondition>,
    ) -> (WarGraph, NodeId, NodeId, NodeId, NodeId) {
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let na = NodeId::new("a");
        let nb = NodeId::new("b");
        let nc = NodeId::new("c");
        let nd = NodeId::new("d");
        graph.add_node(na.clone(), NodeSpec::Function(a));
        graph.add_node(nb.clone(), NodeSpec::Function(b));
        graph.add_node(nc.clone(), NodeSpec::Function(c));
        graph.add_node(nd.clone(), NodeSpec::Function(d));
        graph.add_edge(EdgeSpec {
            from: na.clone(),
            to: nb.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: na.clone(),
            to: nc.clone(),
            condition: a_to_c_condition,
        });
        graph.add_edge(EdgeSpec {
            from: nb.clone(),
            to: nd.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: nc.clone(),
            to: nd.clone(),
            condition: None,
        });
        graph.add_entry(na.clone());
        (graph, na, nb, nc, nd)
    }

    #[tokio::test]
    async fn diamond_join_executes_target_exactly_once() {
        let a = CountingFunctionNode::fixed(field("log"), serde_json::json!("a"));
        let b = CountingFunctionNode::fixed(field("log"), serde_json::json!("b"));
        let c = CountingFunctionNode::fixed(field("log"), serde_json::json!("c"));
        let d = CountingFunctionNode::fixed(field("log"), serde_json::json!("d"));
        let (graph, _, _, c_id, d_id) =
            diamond_graph(a.clone(), b.clone(), c.clone(), d.clone(), None);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("diamond-join").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("diamond join must not deadlock");

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(d.run_count(), 1, "join target must execute exactly once");
        assert_eq!(c.run_count(), 1);
        let _ = c_id;
        let _ = d_id;
    }

    #[tokio::test]
    async fn false_branch_is_proven_not_firing_and_join_still_runs_once() {
        // A-to-C's condition can never match A's own output, so C is
        // proven not-firing rather than merely never scheduled -- and D
        // (which also depends on B) must still execute exactly once rather
        // than waiting forever on C. Explicit timeout: a regression here is
        // a deadlock, not a wrong value, so a hang must fail loudly.
        let a = CountingFunctionNode::fixed(field("log"), serde_json::json!("a"));
        let b = CountingFunctionNode::fixed(field("log"), serde_json::json!("b"));
        let c = CountingFunctionNode::fixed(field("log"), serde_json::json!("c"));
        let d = CountingFunctionNode::fixed(field("log"), serde_json::json!("d"));
        let (graph, _, _, _, _) = diamond_graph(
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
            Some(EdgeCondition::Contains(
                "UNREACHABLE_MARKER_XYZ".to_string(),
            )),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("diamond-not-firing").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("a not-firing branch must not strand the downstream join");

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(c.run_count(), 0, "the false branch never executes");
        assert_eq!(
            d.run_count(),
            1,
            "the join still executes exactly once despite the not-firing branch"
        );
    }

    #[tokio::test]
    async fn node_fed_only_by_an_unreachable_source_never_runs_and_does_not_stall_its_join() {
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let ghost = NodeId::new("ghost");
        let u = NodeId::new("u");
        let d = NodeId::new("d");
        let a_node = CountingFunctionNode::fixed(field("log"), serde_json::json!("a"));
        let ghost_node = CountingFunctionNode::fixed(field("log"), serde_json::json!("ghost"));
        let u_node = CountingFunctionNode::fixed(field("log"), serde_json::json!("u"));
        let d_node = CountingFunctionNode::fixed(field("log"), serde_json::json!("d"));
        graph.add_node(a.clone(), NodeSpec::Function(a_node));
        // `ghost` is declared but is neither an entry point nor the target
        // of any edge: it is structurally unreachable from the start.
        graph.add_node(ghost.clone(), NodeSpec::Function(ghost_node.clone()));
        graph.add_node(u.clone(), NodeSpec::Function(u_node.clone()));
        graph.add_node(d.clone(), NodeSpec::Function(d_node.clone()));
        graph.add_edge(EdgeSpec {
            from: ghost.clone(),
            to: u.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: u.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_entry(a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("unreachable-source").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("a dead upstream node must not stall its own downstream join");

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(ghost_node.run_count(), 0);
        assert_eq!(u_node.run_count(), 0, "u's only source never runs");
        assert_eq!(d_node.run_count(), 1, "d's join still resolves once");
    }

    #[tokio::test]
    async fn deferred_node_aggregates_only_after_no_other_node_is_executable() {
        let log: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let x = NodeId::new("x");
        let y = NodeId::new("y");
        let e = NodeId::new("e");
        let d = NodeId::new("d");

        let mk = |log: Arc<std::sync::Mutex<Vec<String>>>, id: &'static str| {
            CountingFunctionNode::new(move |_run, _state| {
                log.lock().unwrap().push(id.to_string());
                let mut delta = paladin_core::platform::container::battlefield::StateDelta::new();
                delta.set_raw(field("log"), serde_json::json!(id));
                delta
            })
        };

        graph.add_node(x.clone(), NodeSpec::Function(mk(log.clone(), "x")));
        graph.add_node(y.clone(), NodeSpec::Function(mk(log.clone(), "y")));
        graph.add_node(e.clone(), NodeSpec::Function(mk(log.clone(), "e")));
        graph.add_deferred_node(d.clone(), NodeSpec::Function(mk(log.clone(), "d")));
        graph.add_edge(EdgeSpec {
            from: x.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: x.clone(),
            to: e.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: y.clone(),
            to: d.clone(),
            condition: None,
        });
        graph.add_entry(x.clone());
        graph.add_entry(y.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("defer-aggregate").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("defer aggregation must not deadlock");

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        let order = log.lock().unwrap().clone();
        assert_eq!(
            order.last(),
            Some(&"d".to_string()),
            "the deferred node must run last, after its non-deferred sibling"
        );
        assert!(
            order.iter().filter(|id| id.as_str() == "d").count() == 1,
            "the deferred node executes exactly once"
        );
        assert!(order.contains(&"e".to_string()));
    }

    #[tokio::test]
    async fn two_deferred_nodes_resolve_in_node_registration_order() {
        let s = schema(vec![FieldSpec::new(
            field("log"),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let x = NodeId::new("x");
        let d2 = NodeId::new("d2");
        let d1 = NodeId::new("d1");

        graph.add_node(
            x.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("x"),
            )),
        );
        // Registered in the order d2, then d1 -- deliberately the reverse
        // of the edge-insertion order below, so a pass that (incorrectly)
        // orders deferred releases by edge order rather than node
        // registration order would produce [d1, d2] instead.
        graph.add_deferred_node(
            d2.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("d2"),
            )),
        );
        graph.add_deferred_node(
            d1.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("log"),
                serde_json::json!("d1"),
            )),
        );
        graph.add_edge(EdgeSpec {
            from: x.clone(),
            to: d1.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: x.clone(),
            to: d2.clone(),
            condition: None,
        });
        graph.add_entry(x.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("defer-order").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread.clone(), &store),
        )
        .await
        .expect("defer release must not deadlock");
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        let mut by_superstep = saved.clone();
        by_superstep.sort_by_key(|w| w.superstep);
        // The first waypoint's `vanguard` is the Vanguard computed for
        // superstep 2, i.e. the released deferred nodes.
        assert_eq!(by_superstep[0].vanguard, vec![d2.clone(), d1.clone()]);
    }

    #[tokio::test]
    async fn insertion_order_does_not_change_the_vanguard_sequence() {
        async fn run_diamond_and_collect_vanguards(
            build: impl FnOnce(&mut WarGraph, NodeId, NodeId, NodeId, NodeId),
            thread_name: &str,
        ) -> Vec<Vec<NodeId>> {
            let s = schema(vec![FieldSpec::new(
                field("log"),
                DispatchRule::Append,
                None,
                false,
            )]);
            let mut graph = WarGraph::new(s, EngineLimits::default());
            let a = NodeId::new("a");
            let b = NodeId::new("b");
            let c = NodeId::new("c");
            let d = NodeId::new("d");
            build(&mut graph, a.clone(), b.clone(), c.clone(), d.clone());
            graph.add_edge(EdgeSpec {
                from: a.clone(),
                to: b.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: a.clone(),
                to: c.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: b.clone(),
                to: d.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: c.clone(),
                to: d.clone(),
                condition: None,
            });
            graph.add_entry(a);

            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(thread_name).unwrap();
            let outcome = run_default(&graph, thread.clone(), &store).await;
            assert!(matches!(outcome, RunOutcome::Completed { .. }));

            let saved = store.saved_waypoints(&thread).await;
            let mut by_superstep = saved;
            by_superstep.sort_by_key(|w| w.superstep);
            by_superstep.into_iter().map(|w| w.vanguard).collect()
        }

        let forward = run_diamond_and_collect_vanguards(
            |graph, a, b, c, d| {
                graph.add_node(
                    a,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("a"),
                    )),
                );
                graph.add_node(
                    b,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("b"),
                    )),
                );
                graph.add_node(
                    c,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("c"),
                    )),
                );
                graph.add_node(
                    d,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("d"),
                    )),
                );
            },
            "insertion-order-forward",
        )
        .await;

        let reversed = run_diamond_and_collect_vanguards(
            |graph, a, b, c, d| {
                graph.add_node(
                    d,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("d"),
                    )),
                );
                graph.add_node(
                    c,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("c"),
                    )),
                );
                graph.add_node(
                    b,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("b"),
                    )),
                );
                graph.add_node(
                    a,
                    NodeSpec::Function(CountingFunctionNode::fixed(
                        field("log"),
                        serde_json::json!("a"),
                    )),
                );
            },
            "insertion-order-reversed",
        )
        .await;

        assert_eq!(forward, reversed);
    }

    #[tokio::test]
    async fn two_node_cycle_terminates_on_edge_condition_not_a_limit() {
        let s = schema(vec![FieldSpec::new(
            field("status"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        // `a` continues for its first two executions, then stops; `b` only
        // ever relays back to `a`, so the cycle's length is driven entirely
        // by `a`'s own condition, never by a visit/superstep limit.
        let a_node = CountingFunctionNode::new(|run_index, _state| {
            let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
            d.set(
                field("status"),
                if run_index < 2 { "continue" } else { "stop" },
            )
            .unwrap();
            d
        });
        let b_node = CountingFunctionNode::fixed(field("status"), serde_json::json!("relayed"));
        graph.add_node(a.clone(), NodeSpec::Function(a_node.clone()));
        graph.add_node(b.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: Some(EdgeCondition::Contains("continue".to_string())),
        });
        graph.add_edge(EdgeSpec {
            from: b.clone(),
            to: a.clone(),
            condition: None,
        });
        graph.add_entry(a);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("two-node-cycle").unwrap();
        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_default(&graph, thread, &store),
        )
        .await
        .expect("the cycle must terminate on its own condition, not hang");

        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(a_node.run_count(), 3);
        assert_eq!(b_node.run_count(), 2);
        assert!(3 < EngineLimits::default().max_supersteps);
    }

    // --- Task 3: determinism under randomized scheduling, and the X-05
    // 100-iteration multi-thread stress test ------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn eng_fr_08_determinism_over_twenty_randomized_scheduling_iterations() {
        let mut reference: Option<(String, Vec<Vec<NodeId>>)> = None;

        for seed in 0..20u64 {
            let s = schema(vec![
                FieldSpec::new(field("log"), DispatchRule::Append, None, false),
                FieldSpec::new(field("total"), DispatchRule::Sum, None, false),
            ]);
            let mut graph = WarGraph::new(s, EngineLimits::default());

            let mut entry_ids = Vec::new();
            for i in 0..4 {
                let id = NodeId::new(format!("append{i}"));
                let base =
                    CountingFunctionNode::fixed(field("log"), serde_json::json!(format!("v{i}")));
                let node = YieldingNode::new(base, (seed as usize + i) % 3);
                graph.add_node(id.clone(), NodeSpec::Function(node));
                entry_ids.push(id);
            }
            let sum_id = NodeId::new("summer");
            let sum_base = CountingFunctionNode::fixed(field("total"), serde_json::json!(1));
            let sum_node = YieldingNode::new(sum_base, seed as usize % 2);
            graph.add_node(sum_id.clone(), NodeSpec::Function(sum_node));
            entry_ids.push(sum_id);

            shuffle_seeded(&mut entry_ids, seed);
            for id in &entry_ids {
                graph.add_entry(id.clone());
            }

            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("determinism-{seed}")).unwrap();
            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                run_default(&graph, thread.clone(), &store),
            )
            .await
            .unwrap_or_else(|_| panic!("seed {seed} must not hang"));
            let final_state = match outcome {
                RunOutcome::Completed { final_state, .. } => final_state,
                other => panic!("seed {seed}: expected Completed, got {other:?}"),
            };
            let serialized = serde_json::to_string(&final_state).unwrap();

            let saved = store.saved_waypoints(&thread).await;
            let mut by_superstep = saved;
            by_superstep.sort_by_key(|w| w.superstep);
            let vanguard_sequence: Vec<Vec<NodeId>> =
                by_superstep.into_iter().map(|w| w.vanguard).collect();

            match &reference {
                None => reference = Some((serialized, vanguard_sequence)),
                Some((ref_state, ref_sequence)) => {
                    assert_eq!(
                        &serialized, ref_state,
                        "seed {seed} produced a non-byte-identical final Battlefield"
                    );
                    assert_eq!(
                        &vanguard_sequence, ref_sequence,
                        "seed {seed} produced a different Vanguard sequence"
                    );
                }
            }
        }
    }

    /// D-02c: the SAME 20-seed determinism harness as
    /// `eng_fr_08_determinism_over_twenty_randomized_scheduling_iterations`
    /// above (extended, not rebuilt), applied to the two cycle-bootstrap
    /// shapes Plan 22.1-01 fixed -- the self-loop shape
    /// (`self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`)
    /// and the general `entry -> a -> b -> a` shape
    /// (`cycle_node_fed_from_outside_the_cycle_takes_its_first_turn`). Each
    /// shape gets its own 20-iteration loop with its own reference, since
    /// the two are structurally different graphs; `YieldingNode`-backed
    /// nodes with a seed-dependent yield count perturb real async
    /// completion interleaving exactly as the pre-existing test's nodes do,
    /// so a byte-identical result across seeds is not true only by
    /// accident of incidental single-threaded ordering.
    #[tokio::test(flavor = "multi_thread")]
    async fn eng_fr_08_determinism_over_twenty_randomized_iterations_for_cycle_bootstrap_shapes() {
        type Reference = (String, Vec<usize>, std::mem::Discriminant<RunOutcome>);

        // --- Shape 1: self-loop fed from upstream (entry -> b, b -> b). --
        let mut self_loop_reference: Option<Reference> = None;
        for seed in 0..20u64 {
            let s = schema(vec![
                FieldSpec::new(field("entry_ran"), DispatchRule::LastWrite, None, false),
                FieldSpec::new(field("status"), DispatchRule::LastWrite, None, false),
            ]);
            let mut graph = WarGraph::new(s, EngineLimits::default());
            let entry_id = NodeId::new("entry");
            let b_id = NodeId::new("b");

            let entry_base =
                CountingFunctionNode::fixed(field("entry_ran"), serde_json::json!(true));
            let entry_node = YieldingNode::new(entry_base.clone(), seed as usize % 3);
            let b_base = CountingFunctionNode::new(|run_index, _state| {
                let status = if run_index == 0 { "looping" } else { "done" };
                let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
                d.set(field("status"), status).unwrap();
                d
            });
            let b_node = YieldingNode::new(b_base.clone(), (seed as usize + 1) % 3);

            graph.add_node(entry_id.clone(), NodeSpec::Function(entry_node));
            graph.add_node(b_id.clone(), NodeSpec::Function(b_node));
            graph.add_edge(EdgeSpec {
                from: entry_id.clone(),
                to: b_id.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: b_id.clone(),
                to: b_id.clone(),
                condition: Some(EdgeCondition::Contains("looping".to_string())),
            });

            let mut entry_ids = vec![entry_id.clone()];
            shuffle_seeded(&mut entry_ids, seed);
            for id in &entry_ids {
                graph.add_entry(id.clone());
            }

            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("determinism-selfloop-{seed}")).unwrap();
            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                run_default(&graph, thread, &store),
            )
            .await
            .unwrap_or_else(|_| panic!("self-loop shape seed {seed} must not hang"));

            let discriminant = std::mem::discriminant(&outcome);
            let final_state = match outcome {
                RunOutcome::Completed { final_state, .. } => final_state,
                other => panic!("self-loop shape seed {seed}: expected Completed, got {other:?}"),
            };
            let serialized = serde_json::to_string(&final_state).unwrap();
            let run_counts = vec![entry_base.run_count(), b_base.run_count()];

            match &self_loop_reference {
                None => self_loop_reference = Some((serialized, run_counts, discriminant)),
                Some((ref_state, ref_counts, ref_discriminant)) => {
                    assert_eq!(
                        &serialized, ref_state,
                        "self-loop shape seed {seed} produced a non-byte-identical final \
                         Battlefield"
                    );
                    assert_eq!(
                        &run_counts, ref_counts,
                        "self-loop shape seed {seed} produced different per-node run counts"
                    );
                    assert_eq!(
                        &discriminant, ref_discriminant,
                        "self-loop shape seed {seed} produced a different RunOutcome \
                         discriminant"
                    );
                }
            }
        }

        // --- Shape 2: general cycle, entry -> a -> b -> a. ---------------
        let mut cycle_reference: Option<Reference> = None;
        for seed in 0..20u64 {
            let s = schema(vec![
                FieldSpec::new(field("entry_ran"), DispatchRule::LastWrite, None, false),
                FieldSpec::new(field("status"), DispatchRule::LastWrite, None, false),
            ]);
            let mut graph = WarGraph::new(s, EngineLimits::default());
            let entry_id = NodeId::new("entry");
            let a_id = NodeId::new("a");
            let b_id = NodeId::new("b");

            let entry_base =
                CountingFunctionNode::fixed(field("entry_ran"), serde_json::json!(true));
            let entry_node = YieldingNode::new(entry_base.clone(), seed as usize % 3);
            let a_base = CountingFunctionNode::fixed(field("status"), serde_json::json!("a-ran"));
            let a_node = YieldingNode::new(a_base.clone(), (seed as usize + 1) % 3);
            let b_base = CountingFunctionNode::new(|run_index, _state| {
                let status = if run_index == 0 { "looping" } else { "done" };
                let mut d = paladin_core::platform::container::battlefield::StateDelta::new();
                d.set(field("status"), status).unwrap();
                d
            });
            let b_node = YieldingNode::new(b_base.clone(), (seed as usize + 2) % 3);

            graph.add_node(entry_id.clone(), NodeSpec::Function(entry_node));
            graph.add_node(a_id.clone(), NodeSpec::Function(a_node));
            graph.add_node(b_id.clone(), NodeSpec::Function(b_node));
            graph.add_edge(EdgeSpec {
                from: entry_id.clone(),
                to: a_id.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: a_id.clone(),
                to: b_id.clone(),
                condition: None,
            });
            graph.add_edge(EdgeSpec {
                from: b_id.clone(),
                to: a_id.clone(),
                condition: Some(EdgeCondition::Contains("looping".to_string())),
            });

            let mut entry_ids = vec![entry_id.clone()];
            shuffle_seeded(&mut entry_ids, seed);
            for id in &entry_ids {
                graph.add_entry(id.clone());
            }

            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("determinism-cycle-{seed}")).unwrap();
            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                run_default(&graph, thread, &store),
            )
            .await
            .unwrap_or_else(|_| panic!("general cycle shape seed {seed} must not hang"));

            let discriminant = std::mem::discriminant(&outcome);
            let final_state = match outcome {
                RunOutcome::Completed { final_state, .. } => final_state,
                other => {
                    panic!("general cycle shape seed {seed}: expected Completed, got {other:?}")
                }
            };
            let serialized = serde_json::to_string(&final_state).unwrap();
            let run_counts = vec![
                entry_base.run_count(),
                a_base.run_count(),
                b_base.run_count(),
            ];

            match &cycle_reference {
                None => cycle_reference = Some((serialized, run_counts, discriminant)),
                Some((ref_state, ref_counts, ref_discriminant)) => {
                    assert_eq!(
                        &serialized, ref_state,
                        "general cycle shape seed {seed} produced a non-byte-identical final \
                         Battlefield"
                    );
                    assert_eq!(
                        &run_counts, ref_counts,
                        "general cycle shape seed {seed} produced different per-node run counts"
                    );
                    assert_eq!(
                        &discriminant, ref_discriminant,
                        "general cycle shape seed {seed} produced a different RunOutcome \
                         discriminant"
                    );
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn x05_eight_node_parallel_stress_100_iterations_exact_counts() {
        const NODES: usize = 8;
        const ITERATIONS: usize = 100;
        let executions = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut total_saves = 0usize;

        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            for iter in 0..ITERATIONS {
                let s = schema(vec![FieldSpec::new(
                    field("log"),
                    DispatchRule::Append,
                    None,
                    false,
                )]);
                let mut graph = WarGraph::new(s, EngineLimits::default());

                let mut entry_ids = Vec::new();
                for i in 0..NODES {
                    let id = NodeId::new(format!("n{i}"));
                    let exec = executions.clone();
                    let base = CountingFunctionNode::new(move |_run, _state| {
                        exec.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let mut d =
                            paladin_core::platform::container::battlefield::StateDelta::new();
                        d.set_raw(field("log"), serde_json::json!(i));
                        d
                    });
                    let node = YieldingNode::new(base, (iter + i) % 3);
                    graph.add_node(id.clone(), NodeSpec::Function(node));
                    entry_ids.push(id);
                }
                shuffle_seeded(&mut entry_ids, iter as u64);
                for id in &entry_ids {
                    graph.add_entry(id.clone());
                }

                let store = RecordingWaypointStore::new();
                let thread = ThreadId::new(format!("x05-{iter}")).unwrap();
                let outcome = run_default(&graph, thread, &store).await;
                assert!(
                    matches!(outcome, RunOutcome::Completed { .. }),
                    "iteration {iter} did not complete"
                );
                total_saves += store.save_call_count();
            }
        })
        .await
        .expect(
            "the 100-iteration 8-node all-parallel stress run must complete inside the \
             timeout -- a deadlock or livelock must fail loudly, not hang the suite",
        );

        // Exact equality, not a lower bound (X-05): a lost or duplicated
        // node execution or Waypoint save would show as a count below or
        // above these products, which a `>=`/`<=` assertion would tolerate.
        assert_eq!(
            executions.load(std::sync::atomic::Ordering::SeqCst),
            NODES * ITERATIONS,
            "exact node-execution count across all iterations"
        );
        assert_eq!(
            total_saves, ITERATIONS,
            "exactly one Waypoint save per single-superstep iteration"
        );
    }
}
