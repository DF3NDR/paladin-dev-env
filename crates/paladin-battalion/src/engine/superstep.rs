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
    Battlefield, CustomDispatchResolver, FieldName, StateDelta,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId, Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::trace_sink_port::TraceEvent;
use paladin_ports::output::waypoint_port::WaypointPort;

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
        /// The field `PaladinResult.output` is written into as a delta.
        output_field: FieldName,
    },
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
) -> (Option<Uuid>, u64, Result<StateDelta, NodeError>) {
    match dispatch {
        NodeDispatch::Function(node) => {
            let result = node.run(snapshot, ctx).await;
            (None, 0, result)
        }
        NodeDispatch::Paladin {
            paladin,
            input_template,
            output_field,
        } => {
            let paladin_id = Some(paladin.uuid);
            let rendered = match input_template.render(snapshot) {
                Ok(rendered) => rendered,
                Err(e) => return (paladin_id, 0, Err(NodeError(e.to_string()))),
            };
            match paladin_port.execute(&paladin, &rendered).await {
                Ok(result) => {
                    let token_count = u64::from(result.token_count);
                    let mut delta = StateDelta::new();
                    match delta.set(output_field, result.output.clone()) {
                        Ok(()) => (paladin_id, token_count, Ok(delta)),
                        Err(e) => (paladin_id, token_count, Err(NodeError(e.to_string()))),
                    }
                }
                Err(e) => (paladin_id, 0, Err(NodeError(e.to_string()))),
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
    /// `Proceed`ed and the node itself succeeded) and produced this delta,
    /// already passed through every `after` hook in order.
    Succeeded(StateDelta),
    /// A `NodeInterceptor::before` returned `Skip(reason)`: the node never
    /// executed. Contributes no delta to this superstep's merge.
    Skipped(String),
    /// The node failed -- either its own execution returned an error, or a
    /// `NodeInterceptor::before` returned `Fail(error)` before the node
    /// could run.
    Failed(NodeError),
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
    graph: &WarGraph,
    thread: ThreadId,
    mut battlefield: Battlefield,
    mut vanguard: Vec<NodeId>,
    mut visit_counts: BTreeMap<NodeId, u32>,
    mut parent_waypoint_id: Option<WaypointId>,
    mut superstep_number: u64,
    paladin_port: &Arc<dyn PaladinPort>,
    trace: &Arc<TraceDispatcher>,
    interceptors: &[Arc<dyn NodeInterceptor>],
    cancellation: &Option<CancellationToken>,
) -> Result<RunOutcome, EngineError> {
    // The entry-vanguard-empty case: nothing to run, ever. Persist exactly
    // one Completed Waypoint and return immediately (ENG-FR-01 step 7's
    // "Vanguard empty -> Completed" path, reached without executing a
    // superstep at all).
    if vanguard.is_empty() {
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
        );
        persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
        return Ok(RunOutcome::Completed {
            final_state: battlefield,
            waypoint: waypoint.waypoint_id,
        });
    }

    let mut frontier = Frontier::new(graph);

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

        // --- ENG-FR-05/ENG-NFR-02: exactly one Arc-shared read snapshot
        // for the whole superstep, cloned once.
        let snapshot = Arc::new(battlefield.clone());
        let limit = parallelism.unwrap_or(vanguard.len()).max(1);
        let semaphore = Arc::new(Semaphore::new(limit));

        let mut handles = Vec::with_capacity(vanguard.len());
        for node_id in &vanguard {
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
                } => NodeDispatch::Paladin {
                    paladin: paladin.clone(),
                    input_template: input_template.clone(),
                    output_field: output_field.clone(),
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
                    InterceptDecision::Fail(err) => (None, 0u64, NodeRunOutcome::Failed(err)),
                    InterceptDecision::Proceed => {
                        let _permit = sem
                            .acquire_owned()
                            .await
                            .expect("semaphore is never closed");
                        let (paladin_id, token_count, result) =
                            execute_vanguard_node(dispatch, &snap, &ctx, &port).await;
                        match result {
                            Ok(mut delta) => {
                                // --- ENG-FR-22: run every `after` in order,
                                // each observing the previous one's mutation.
                                for interceptor in &node_interceptors {
                                    interceptor.after(&ctx, &mut delta).await;
                                }
                                (paladin_id, token_count, NodeRunOutcome::Succeeded(delta))
                            }
                            Err(e) => (paladin_id, token_count, NodeRunOutcome::Failed(e)),
                        }
                    }
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
        let mut node_failure: Option<(NodeId, NodeError)> = None;
        for handle in handles {
            let (node_id, started_at, duration_ms, paladin_id, token_count, outcome) = handle
                .await
                .map_err(|e| EngineError::Node(NodeError(format!("task join error: {e}"))))?;
            match outcome {
                NodeRunOutcome::Succeeded(delta) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: NodeOutcomeKind::Succeeded,
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
            let error = EngineError::Node(err);
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
                        failed_node: ran.first().cloned().unwrap_or_else(|| vanguard[0].clone()),
                    },
                    visit_counts,
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
            frontier.record_execution(graph, node_id, superstep_number, &battlefield)?;
        }
        frontier.propagate_dead(graph);
        let next_vanguard = compute_next_vanguard(graph, &frontier);
        let status = if next_vanguard.is_empty() {
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
        );
        persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;

        if next_vanguard.is_empty() {
            return Ok(RunOutcome::Completed {
                final_state: battlefield,
                waypoint: waypoint.waypoint_id,
            });
        }

        vanguard = next_vanguard;
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

    /// Record that `node` completed `superstep`, evaluating every one of
    /// its outgoing edges against the POST-merge `battlefield` and storing
    /// each as `Fired`/`NotFiring` at this superstep (ENG-FR-06). Re-running
    /// a node (a cycle or self-loop) overwrites its edges' previous state
    /// with the fresh evaluation.
    fn record_execution(
        &mut self,
        graph: &WarGraph,
        node: &NodeId,
        superstep: u64,
        battlefield: &Battlefield,
    ) -> Result<(), EngineError> {
        self.last_executed.insert(node.clone(), superstep);
        for (idx, edge) in graph.edges().iter().enumerate() {
            if &edge.from != node {
                continue;
            }
            let fires = match &edge.condition {
                None => true,
                Some(condition) => evaluate_edge_condition(condition, battlefield)?,
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

    /// Whether `node` is executable for the NEXT Vanguard (ENG-FR-06): it
    /// has at least one incoming edge, none of them is still pending from a
    /// run-reachable source (a `Pending` edge from a proven-dead source
    /// counts as resolved not-firing, via [`Frontier::edge_resolution`]),
    /// and at least one has fired at or after the superstep `node` last
    /// executed (any fired edge at all, for a node that has never
    /// executed).
    fn is_ready(&self, graph: &WarGraph, node: &NodeId) -> bool {
        let Some(incoming) = self.incoming.get(node) else {
            return false;
        };
        if incoming.is_empty() {
            return false;
        }
        let threshold: i64 = self.last_executed.get(node).map_or(-1, |&s| s as i64);
        let mut any_pending = false;
        let mut any_fresh_fire = false;
        for idx in incoming {
            match self.edge_resolution(graph, *idx) {
                Some((true, resolved_at)) => {
                    if resolved_at as i64 >= threshold {
                        any_fresh_fire = true;
                    }
                }
                Some((false, _)) => {}
                None => any_pending = true,
            }
        }
        !any_pending && any_fresh_fire
    }
}

/// Compute the Vanguard for the superstep after the one `frontier` was just
/// updated for (ENG-FR-06): every non-deferred node the `Frontier` reports
/// executable, in `graph.edges()`'s stable insertion order (ENG-FR-04),
/// de-duplicated. If no non-deferred node is executable, releases every
/// `defer`-marked node the `Frontier` reports executable instead, ordered by
/// this graph's node registration order (`node_order`) rather than
/// `HashMap` order -- the aggregate-after-all-branches case.
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

    let mut deferred_ready = Vec::new();
    for node in graph.node_order() {
        if graph.is_deferred(node) && frontier.is_ready(graph, node) {
            deferred_ready.push(node.clone());
        }
    }
    deferred_ready
}

/// Evaluate an [`EdgeCondition`] against the whole post-merge Battlefield,
/// rendered as its canonical (schema-ordered, `BTreeMap`-backed) JSON
/// string — deterministic by construction, since `Battlefield`'s own
/// `Serialize` impl already guarantees byte-identical output for
/// byte-identical state (ENG-FR-08). This is the typed-state analog of
/// `campaign_service.rs::evaluate_edge_condition`, which matches against a
/// single Paladin's string output; here there is no single canonical
/// "output string" per node; the whole merged state is the sanest, most
/// general substitute available at this phase.
fn evaluate_edge_condition(
    condition: &EdgeCondition,
    battlefield: &Battlefield,
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
        EdgeCondition::Custom(_) => {
            // Engine-level custom edge predicates (mirroring
            // DispatchRule::Custom's registry) are a later plan's
            // expansion; default to firing, matching
            // campaign_service.rs's own placeholder for the same variant.
            Ok(true)
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
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
    };
    use paladin_core::platform::container::waypoint::ThreadId;

    use crate::engine::graph::EdgeSpec;
    use crate::engine::graph::EngineLimits;
    use crate::engine::test_support::{
        ConcurrencyTrackingNode, CountingFunctionNode, FailingFunctionNode, RecordingPaladinPort,
        RecordingWaypointStore, YieldingNode, shuffle_seeded,
    };

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
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
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
        )
        .await
        .unwrap()
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
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
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
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
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
            &graph,
            thread,
            Battlefield::new(graph.schema().clone()),
            graph.entry().to_vec(),
            BTreeMap::new(),
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
