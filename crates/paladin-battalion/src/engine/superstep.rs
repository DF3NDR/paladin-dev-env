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
use paladin_core::platform::container::directive::{
    Directive, MusterContext, MusterTask, NextStep,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::{
    FrontierEdgeState, FrontierSnapshot, MusterProgress, NodeExecutionRecord, NodeId,
    NodeOutcomeKind, ThreadId, Waypoint, WaypointId, WaypointStatus, canonical_edge_condition,
};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::trace_sink_port::TraceEvent;
use paladin_ports::output::waypoint_port::WaypointPort;

use crate::edge_evaluator::EdgeEvaluatorRegistry;
use crate::engine::directive_parser::{DirectiveParseError, DirectiveParser};
use crate::engine::graph::{EngineLimits, NodeSpec, StateMap, WarGraph};
use crate::engine::hooks::{InterceptDecision, NodeInterceptor, TraceDispatcher};
use crate::engine::input_mapping::InputMapping;
use crate::engine::node::NodeError;
use crate::engine::{EngineError, RunOutcome, WaypointDurability};

/// Every parent engine resource D-21 requires forwarding into a
/// `NodeSpec::Battalion` node's child run (CF-FR-16): the `WaypointPort`,
/// `WaypointDurability`, the parallelism setting, the dispatch resolver,
/// the edge-evaluator registry, the trace sink, the interceptor chain and
/// the shared `CancellationToken`. `PaladinPort` is forwarded separately
/// (already `Arc<dyn PaladinPort>` at every call site, no bundling
/// needed). Gathered ONCE per [`run`] call -- never per-dispatch -- and
/// `Arc`-wrapped so every per-superstep node's `tokio::spawn`'d task can
/// capture a cheap clone of it regardless of whether that dispatch entry
/// even is a `NodeSpec::Battalion` node. A resource silently not present
/// here is a resource the child could never receive (or, for the two
/// registries, a validation the child would then skip) -- this is the
/// single construction site so a future `WarEngine` builder method has
/// exactly one place to be forwarded from.
struct ChildEngineResources<W: WaypointPort + 'static> {
    waypoint_port: Arc<W>,
    durability: WaypointDurability,
    parallelism: Option<usize>,
    registry: CustomDispatchResolver,
    evaluators: EdgeEvaluatorRegistry,
    trace: Arc<TraceDispatcher>,
    interceptors: Vec<Arc<dyn NodeInterceptor>>,
    cancellation: Option<CancellationToken>,
    /// THIS run's own `checkpoint_ns` (CF-FR-15, D-20) -- captured once
    /// here so a NESTED `NodeSpec::Battalion` dispatch (a grandchild, from
    /// this run's own perspective) can derive the next namespace segment
    /// as `"{this}{grandchild_node_id}/"` without threading an extra
    /// parameter through the whole dispatch/execute call chain.
    checkpoint_ns: Option<String>,
}

/// What one vanguard node resolves to for this superstep's execution: either
/// a `Function` node's trait object, or the pieces of a `NodeSpec::Paladin`
/// node needed to render its input and call the port, cloned out of the
/// graph so the spawned task owns everything it touches (`Paladin` is `Box`ed
/// in `NodeSpec`; cloning one `Paladin` per executing node per superstep is
/// the accepted cost of keeping `WarGraph` itself immutable and shareable
/// across concurrently-executing peers).
enum NodeDispatch<W: WaypointPort + 'static> {
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
    /// A `NodeSpec::Battalion` node's execution inputs (CF-FR-14, D-19).
    Battalion {
        /// The embedded child graph.
        graph: Arc<WarGraph>,
        /// The declared parent<->child state channel.
        state_map: StateMap,
        /// Every parent engine resource this child run inherits (D-21),
        /// gathered once per outer [`run`] call.
        resources: Arc<ChildEngineResources<W>>,
        /// Whether a resumed run restarts this node's child from scratch
        /// (CF-FR-15, D-20) rather than resuming it from
        /// `latest(child_thread)` -- see the dispatch arm's own rustdoc for
        /// the abandon-vs-overwrite policy this implements.
        restart_on_resume: bool,
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
    /// A `NodeSpec::Battalion` node's child run failed (CF-FR-16, D-21) --
    /// already the fully-formed, structured `EngineError::BattalionChildFailed`
    /// (X-06: naming the failing child node and thread, never a bare
    /// interpolated string), built where the child's own thread id is in
    /// scope and passed through here unchanged.
    Battalion(EngineError),
}

/// [`execute_vanguard_node`]'s per-node result: `paladin_id`/`token_count`
/// (`None`/`0` for a `Function` or `Battalion` node) plus the resolved
/// `Directive` or [`NodeFailure`].
type NodeDispatchResult = (Option<Uuid>, u64, Result<Directive, NodeFailure>);

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
///
/// Declared as a plain `fn` manually returning a boxed, `dyn`-erased
/// future (rather than `async fn`, which would give it an opaque
/// `impl Future` return type) because its `NodeSpec::Battalion` arm calls
/// [`run`] recursively: two `async fn`s whose bodies call each other
/// create a compiler-level opaque-type inference cycle (E0391) that pure
/// `Box::pin`-at-the-call-site boxing does not resolve on its own -- an
/// explicit, non-opaque signature here is what breaks it, mirroring the
/// SAME `dyn Future + Send` erasure the recursive call site itself uses.
fn execute_vanguard_node<'a, W: WaypointPort + 'static>(
    dispatch: NodeDispatch<W>,
    snapshot: &'a Battlefield,
    ctx: &'a crate::engine::node::NodeContext,
    paladin_port: &'a Arc<dyn PaladinPort>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = NodeDispatchResult> + Send + 'a>> {
    Box::pin(async move {
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
                // --- CF-03, D-15: the executing task's Muster context (`Some`
                // only for a worker-template dispatch), so `{muster.payload}`/
                // `{muster.task_key}` resolve from it, never from the
                // Battlefield.
                let rendered = match input_template.render(snapshot, ctx.muster.as_ref()) {
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
                            Err(e) => {
                                (paladin_id, token_count, Err(NodeFailure::DirectiveParse(e)))
                            }
                        }
                    }
                    Err(e) => (
                        paladin_id,
                        0,
                        Err(NodeFailure::Node(NodeError(e.to_string()))),
                    ),
                }
            }
            NodeDispatch::Battalion {
                graph: child_graph,
                state_map,
                resources,
                restart_on_resume,
            } => {
                // --- CF-FR-14, D-19: seed the child's initial state from
                // `state_map.inputs`, read from the PARENT's superstep
                // snapshot under the parent field name, written under the
                // child field name. A parent field absent from the snapshot
                // (e.g. never yet written) is simply not set here -- the
                // child schema's own default/required-field rules decide
                // whether that is acceptable, exactly as `Battlefield::initialize`
                // already does for an ordinary run's own `initial` delta.
                let mut initial = StateDelta::new();
                for (parent_field, child_field) in &state_map.inputs {
                    if let Some(value) = snapshot.get_raw(parent_field) {
                        initial.set_raw(child_field.clone(), value.clone());
                    }
                }

                // --- CF-FR-15, D-20: the child's thread id is a derived,
                // PROVABLY INJECTIVE encoding of (this run's thread, this
                // node's id) -- `ThreadId::child`, length-prefixed exactly
                // like `graph.rs`'s `push_field` (22.1 CR-01's lesson: a
                // bare delimiter join of `NodeId`s, which accept any
                // non-empty string, is collidable by construction). Fails
                // typed (never silently truncates) if the derived id would
                // itself exceed `ThreadId`'s own limits.
                let child_thread = match ThreadId::child(&ctx.thread_id, &ctx.node_id) {
                    Ok(id) => id,
                    Err(e) => {
                        return (
                            None,
                            0,
                            Err(NodeFailure::Node(NodeError(format!(
                                "battalion node {}: failed to derive child thread id: {e}",
                                ctx.node_id
                            )))),
                        );
                    }
                };

                // --- CF-FR-15, D-20: `checkpoint_ns` is a RECORD of the
                // namespace path for observability/debugging ONLY -- never
                // the isolation mechanism (RESEARCH.md Pitfall 6). Isolation
                // comes entirely from `child_thread` above being a distinct
                // `ThreadId`. Nested paths concatenate: a grandchild's
                // namespace is `"{this child's namespace}{grandchild_node}/"`.
                let child_checkpoint_ns = Some(format!(
                    "{}{}/",
                    resources.checkpoint_ns.as_deref().unwrap_or(""),
                    ctx.node_id.as_str()
                ));

                // --- CF-FR-15, D-20: resume-mid-child. Unless
                // `restart_on_resume` opts out, look up the child thread's
                // own latest Waypoint through the SAME `WaypointPort` this
                // child run addresses (no `WaypointPort` method change) --
                // if one exists and the child has not yet `Completed`, this
                // dispatch RESUMES the child from exactly where it stopped
                // rather than re-seeding it from `state_map.inputs`, so a
                // parent resumed mid-child never repeats the child's already
                // -completed work. A `Completed` prior child is mapped
                // straight to this node's output delta without re-invoking
                // `run()` at all (idempotent re-dispatch, no redundant
                // Waypoint write). `restart_on_resume: true` -- or no prior
                // history at all -- falls through to a fresh child run,
                // exactly as before this plan: the OLD child chain (if any)
                // is deliberately ABANDONED, never deleted -- its own latest
                // Waypoint stays in the store and stays protected by
                // retention's existing per-thread rule exactly as any other
                // thread's would (D-20; `WaypointRetentionService` itself is
                // unchanged), it is simply no longer this thread's `latest`
                // once the fresh run below persists its own first Waypoint.
                let existing_latest = if restart_on_resume {
                    None
                } else {
                    match resources.waypoint_port.latest(&child_thread).await {
                        Ok(w) => w,
                        Err(e) => {
                            return (
                                None,
                                0,
                                Err(NodeFailure::Node(NodeError(format!(
                                    "battalion node {}: failed to read child thread history: {e}",
                                    ctx.node_id
                                )))),
                            );
                        }
                    }
                };

                if let Some(latest) = &existing_latest
                    && matches!(latest.status, WaypointStatus::Completed)
                {
                    // --- the child already finished in a prior attempt
                    // (e.g. the crash landed between the child's own
                    // completion and this node's own delta reaching the
                    // PARENT's next Waypoint) -- map its recorded final
                    // state straight through, never re-running it.
                    let mut delta = StateDelta::new();
                    for (child_field, parent_field) in &state_map.outputs {
                        if let Some(value) = latest.battlefield.get_raw(child_field) {
                            delta.set_raw(parent_field.clone(), value.clone());
                        }
                    }
                    return (None, 0, Ok(delta.into()));
                }

                let (
                    child_battlefield,
                    child_vanguard,
                    child_visit_counts,
                    child_frontier_snapshot,
                    child_muster_progress,
                    child_parent_waypoint_id,
                    child_superstep_number,
                ) = match existing_latest {
                    Some(latest) => {
                        let resume_superstep = if latest.muster_progress.is_some() {
                            latest.superstep
                        } else {
                            latest.superstep + 1
                        };
                        (
                            latest.battlefield,
                            latest.vanguard,
                            latest.visit_counts,
                            Some(latest.frontier),
                            latest.muster_progress,
                            Some(latest.waypoint_id),
                            resume_superstep,
                        )
                    }
                    None => {
                        let fresh_battlefield =
                            match Battlefield::initialize(child_graph.schema().clone(), &initial) {
                                Ok(bf) => bf,
                                Err(e) => {
                                    return (
                                        None,
                                        0,
                                        Err(NodeFailure::Node(NodeError(format!(
                                            "battalion node {}: failed to initialize child \
                                         battlefield: {e}",
                                            ctx.node_id
                                        )))),
                                    );
                                }
                            };
                        if let Err(e) = fresh_battlefield.validate_required() {
                            return (
                                None,
                                0,
                                Err(NodeFailure::Node(NodeError(format!(
                                    "battalion node {}: child battlefield missing required \
                                     field(s): {e}",
                                    ctx.node_id
                                )))),
                            );
                        }
                        (
                            fresh_battlefield,
                            child_graph.entry().to_vec(),
                            BTreeMap::new(),
                            None,
                            None,
                            None,
                            1,
                        )
                    }
                };

                // --- CF-FR-16, D-21: one parent superstep spans the whole
                // child run, however many supersteps the child itself takes,
                // because this whole recursive call is awaited INLINE within
                // this single dispatch entry's own `tokio::spawn`'d task --
                // never spawned as a separate sibling task. Recursion into the
                // SAME `run_with_namespace` requires boxing (Rust cannot size a directly
                // self-referential async fn) -- and explicit `dyn Future +
                // Send` erasure specifically (not merely `Box::pin` over the
                // concrete opaque type), because a self-recursive async fn's
                // auto-trait (`Send`) inference cannot resolve through its own
                // cyclic opaque return type; erasing to a trait object breaks
                // the cycle and is checked, at this one call site, to actually
                // be `Send`.
                let child_fut: std::pin::Pin<
                    Box<
                        dyn std::future::Future<Output = Result<RunOutcome, EngineError>>
                            + Send
                            + '_,
                    >,
                > = Box::pin(run_with_namespace(
                    resources.waypoint_port.as_ref(),
                    resources.durability,
                    resources.parallelism,
                    &resources.registry,
                    &resources.evaluators,
                    child_graph.as_ref(),
                    child_thread.clone(),
                    child_battlefield,
                    child_vanguard,
                    child_visit_counts,
                    child_frontier_snapshot,
                    child_muster_progress,
                    child_parent_waypoint_id,
                    child_superstep_number,
                    paladin_port,
                    &resources.trace,
                    &resources.interceptors,
                    &resources.cancellation,
                    Some(Arc::clone(&resources.waypoint_port)),
                    child_checkpoint_ns,
                ));
                let outcome = child_fut.await;

                match outcome {
                    Ok(RunOutcome::Completed { final_state, .. }) => {
                        // --- CF-FR-14: only `state_map.outputs`-mapped fields
                        // are read out of the child's final state -- no code
                        // path copies the child's whole Battlefield into the
                        // parent, keeping every unmapped child field private.
                        let mut delta = StateDelta::new();
                        for (child_field, parent_field) in &state_map.outputs {
                            if let Some(value) = final_state.get_raw(child_field) {
                                delta.set_raw(parent_field.clone(), value.clone());
                            }
                        }
                        (None, 0, Ok(delta.into()))
                    }
                    Ok(RunOutcome::Halted { .. }) => {
                        // --- D-21: the child observed the shared
                        // `CancellationToken` at its own superstep boundary
                        // and persisted `Halted`. This node contributes an
                        // empty delta (never coerced into a failure); the
                        // PARENT's own top-of-loop cancellation check -- the
                        // SAME token -- halts the parent at its own next
                        // boundary.
                        (None, 0, Ok(StateDelta::new().into()))
                    }
                    Ok(RunOutcome::AwaitingInput { .. }) => (
                        None,
                        0,
                        Err(NodeFailure::Node(NodeError(format!(
                            "battalion node {}: child run paused awaiting input, which this phase \
                         does not support",
                            ctx.node_id
                        )))),
                    ),
                    Ok(RunOutcome::Failed { error, .. }) => (
                        None,
                        0,
                        Err(NodeFailure::Battalion(EngineError::BattalionChildFailed {
                            node: ctx.node_id.clone(),
                            child_thread,
                            source: Box::new(error),
                        })),
                    ),
                    Err(error) => (
                        None,
                        0,
                        Err(NodeFailure::Battalion(EngineError::BattalionChildFailed {
                            node: ctx.node_id.clone(),
                            child_thread,
                            source: Box::new(error),
                        })),
                    ),
                }
            }
        }
    })
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

/// The `tasks.len() > limits.max_muster_tasks` comparison (D-13's
/// `precision` edge truth), factored out of [`validate_muster_tasks`] so it
/// is independently unit-testable without allocating a multi-billion-
/// element `Vec`: `limit` is always widened to `usize` here, `count`
/// (already a `usize`) is never narrowed with `as u32` -- a task list
/// longer than `u32::MAX` cannot wrap into a passing count.
fn muster_task_count_exceeds_limit(count: usize, limit: u32) -> bool {
    count > limit as usize
}

/// Validate an incoming `NextStep::Muster(tasks)` at the Directive-receipt
/// point (CF-03, D-13) -- the SAME per-node accumulation loop where a
/// `Goto` target is validated -- and BEFORE any task is dispatched: an
/// empty task list, a duplicate `task_key`, a task count exceeding
/// `limits.max_muster_tasks`, and a task naming an unknown or
/// non-worker-template `worker` are all rejected here, never inside the
/// worker-dispatch loop where a partial launch would be unrecoverable
/// (RESEARCH.md anti-pattern 3). On success, returns `tasks` sorted by
/// `task_key` (`String` byte order) -- the ordering the deterministic
/// task_key-order merge (D-13) relies on, since every accepted task then
/// reaches [`run`]'s dispatch-building loop in this order and the existing
/// sequential-await-per-handle + stable `deltas.sort_by(NodeId)` machinery
/// preserves it into the final merge without any bespoke reordering.
///
/// The count check widens `limits.max_muster_tasks` (`u32`) to `usize`
/// rather than narrowing `tasks.len()` with `as u32`, so a task list longer
/// than `u32::MAX` cannot wrap into a passing count (the `precision` edge
/// truth).
fn validate_muster_tasks(
    graph: &WarGraph,
    node: &NodeId,
    limits: &EngineLimits,
    mut tasks: Vec<MusterTask>,
) -> Result<Vec<MusterTask>, EngineError> {
    if tasks.is_empty() {
        return Err(EngineError::EmptyMuster { node: node.clone() });
    }

    if muster_task_count_exceeds_limit(tasks.len(), limits.max_muster_tasks) {
        return Err(EngineError::MusterTaskLimitExceeded {
            node: node.clone(),
            requested: tasks.len(),
            limit: limits.max_muster_tasks,
        });
    }

    let mut seen_keys: HashSet<&str> = HashSet::new();
    for task in &tasks {
        if !seen_keys.insert(task.task_key.as_str()) {
            return Err(EngineError::DuplicateMusterTaskKey {
                node: node.clone(),
                task_key: task.task_key.clone(),
            });
        }
    }

    for task in &tasks {
        match graph.node(&task.worker) {
            None => {
                return Err(EngineError::MusterUnknownWorker {
                    node: node.clone(),
                    worker: task.worker.clone(),
                });
            }
            Some(_) if !graph.is_worker_template(&task.worker) => {
                return Err(EngineError::MusterWorkerNotATemplate {
                    node: node.clone(),
                    worker: task.worker.clone(),
                });
            }
            Some(_) => {}
        }
    }

    tasks.sort_by(|a, b| a.task_key.cmp(&b.task_key));
    Ok(tasks)
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
/// `initial_muster_progress` (CF-FR-12, D-14) seeds a mid-muster resume:
/// `None` for a fresh `start` or an ordinary (non-muster) `resume`,
/// `Some(progress)` when the loaded Waypoint carried a `muster_progress`
/// record. When `Some`, this call re-enters the SAME superstep the record
/// was written at (never `superstep_number + 1`) and dispatches only
/// `progress.unfinished_tasks()` -- the caller (`WarEngine::resume_with_options`)
/// is responsible for passing `superstep_number` equal to the loaded
/// Waypoint's own `superstep`, not one past it, to match.
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
///
/// `waypoint_port_arc` (CF-FR-16, D-21) is the single seam a
/// `NodeSpec::Battalion` node's child run is constructed from: `Some(Arc)`
/// from every real `WarEngine::start`/`resume_with_options` call (which
/// already hold `Arc<W>`), `None` from a test helper whose graph never
/// embeds a Battalion node. A Battalion dispatch entry with no Arc
/// available fails closed with a `NodeError` naming the node, rather than
/// silently skipping the child -- this seam existing at all is what lets
/// [`ChildEngineResources`] be gathered exactly once per `run()` call and
/// `Arc`-cloned into each dispatching node's `tokio::spawn`'d task, the
/// single construction site D-21's own rustdoc note calls for.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run<W: WaypointPort + 'static>(
    waypoint_port: &W,
    durability: WaypointDurability,
    parallelism: Option<usize>,
    registry: &CustomDispatchResolver,
    evaluators: &EdgeEvaluatorRegistry,
    graph: &WarGraph,
    thread: ThreadId,
    battlefield: Battlefield,
    vanguard: Vec<NodeId>,
    visit_counts: BTreeMap<NodeId, u32>,
    frontier_snapshot: Option<FrontierSnapshot>,
    initial_muster_progress: Option<MusterProgress>,
    parent_waypoint_id: Option<WaypointId>,
    superstep_number: u64,
    paladin_port: &Arc<dyn PaladinPort>,
    trace: &Arc<TraceDispatcher>,
    interceptors: &[Arc<dyn NodeInterceptor>],
    cancellation: &Option<CancellationToken>,
    waypoint_port_arc: Option<Arc<W>>,
) -> Result<RunOutcome, EngineError> {
    // --- CF-FR-15, D-20: a top-level call through this public entry point
    // (`WarEngine::start`/`resume_with_options`, and every existing test
    // call site predating this plan) is never itself a Battalion child, so
    // its own Waypoints carry no namespace. This function's SIGNATURE is
    // deliberately left unchanged by this plan -- it is called from outside
    // this module (`engine::mod`, and `engine::graph`'s own tests) -- and
    // instead forwards, unconditionally, to [`run_with_namespace`], the
    // real implementation, with `checkpoint_ns: None`. A `NodeSpec::Battalion`
    // dispatch (`execute_vanguard_node`, below) calls
    // [`run_with_namespace`] directly instead of this wrapper, since ONLY
    // that call site ever has a `Some` namespace to pass.
    run_with_namespace(
        waypoint_port,
        durability,
        parallelism,
        registry,
        evaluators,
        graph,
        thread,
        battlefield,
        vanguard,
        visit_counts,
        frontier_snapshot,
        initial_muster_progress,
        parent_waypoint_id,
        superstep_number,
        paladin_port,
        trace,
        interceptors,
        cancellation,
        waypoint_port_arc,
        None,
    )
    .await
}

/// [`run`]'s real implementation (CF-FR-15, D-20): identical to [`run`] in
/// every respect except the trailing `checkpoint_ns` parameter, which
/// [`run`] always passes as `None` and [`execute_vanguard_node`]'s
/// `NodeSpec::Battalion` dispatch arm passes as `Some(namespace)` when
/// recursing into a child run. Kept as a SEPARATE function (rather than
/// adding the parameter to [`run`] directly) so `run`'s own public
/// signature -- called from `engine::mod` and from `engine::graph`'s own
/// tests -- stays unchanged by this plan.
#[allow(clippy::too_many_arguments)]
async fn run_with_namespace<W: WaypointPort + 'static>(
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
    initial_muster_progress: Option<MusterProgress>,
    mut parent_waypoint_id: Option<WaypointId>,
    mut superstep_number: u64,
    paladin_port: &Arc<dyn PaladinPort>,
    trace: &Arc<TraceDispatcher>,
    interceptors: &[Arc<dyn NodeInterceptor>],
    cancellation: &Option<CancellationToken>,
    waypoint_port_arc: Option<Arc<W>>,
    // --- CF-FR-15, D-20: the namespace path THIS run's own Waypoints are
    // stamped with (`Waypoint.checkpoint_ns`) -- `None` for a top-level
    // run (`WarEngine::start`/`resume_with_options`), `Some(ns)` for a
    // `NodeSpec::Battalion` node's child run, where `ns` was computed by
    // the PARENT dispatch (this run's own caller) as
    // `"{parent's checkpoint_ns}{battalion_node_id}/"` -- concatenating
    // one more namespace segment per nesting level. Stamped, verbatim,
    // onto every Waypoint [`build_waypoint`] produces in this call (never
    // mutated mid-run); a nested Battalion dispatch below derives the NEXT
    // level's namespace from it via `ChildEngineResources::checkpoint_ns`.
    checkpoint_ns: Option<String>,
) -> Result<RunOutcome, EngineError> {
    // --- CF-FR-16, D-21: gathered ONCE per `run()` call, never per
    // dispatch -- see `ChildEngineResources`'s own rustdoc for why a
    // single construction site matters. `None` when this call has no
    // `Arc<W>` available (a non-Battalion test helper); harmless unless a
    // Battalion node is actually dispatched, in which case the dispatch
    // loop below fails that one node closed rather than silently running
    // the child with a missing resource.
    let child_resources: Option<Arc<ChildEngineResources<W>>> =
        waypoint_port_arc.map(|waypoint_port| {
            Arc::new(ChildEngineResources {
                waypoint_port,
                durability,
                parallelism,
                registry: registry.clone(),
                evaluators: evaluators.clone(),
                trace: Arc::clone(trace),
                interceptors: interceptors.to_vec(),
                cancellation: cancellation.clone(),
                checkpoint_ns: checkpoint_ns.clone(),
            })
        });

    // --- CF-FR-12, D-14: seed a mid-muster resume. `pending_muster` (the
    // FULL validated task list plus the mustering node) and
    // `muster_carryover` (completed tasks' unmerged deltas, restored from
    // the loaded progress Waypoint) both come from `initial_muster_progress`
    // when it is `Some`; both stay empty for a fresh `start` or an ordinary
    // (non-muster) `resume`, exactly as before this field existed.
    let mut pending_muster: Option<(NodeId, Vec<MusterTask>)> = initial_muster_progress
        .as_ref()
        .map(|progress| (progress.node.clone(), progress.tasks.clone()));
    let mut muster_carryover: Option<BTreeMap<String, StateDelta>> =
        initial_muster_progress.map(|progress| progress.completed);

    // The entry-vanguard-empty case: nothing to run, ever. Persist exactly
    // one Waypoint and return immediately (ENG-FR-01 step 7's "Vanguard
    // empty -> Completed" path, reached without executing a superstep at
    // all) -- UNLESS D-04's run-end truthful-outcome check finds a node
    // still holding an unconsumed fired incoming edge on a freshly built
    // `Frontier`, in which case this is the OTHER decision site
    // `starved_at_completion` guards (an empty entry Vanguard over a graph
    // whose Frontier disagrees is the same invariant violation as the
    // mid-loop site, just caught before any superstep ever ran).
    //
    // CF-03/CF-FR-12: a pending Muster (fresh from the previous superstep,
    // or restored from a mid-muster resume) always means there IS more work
    // -- a worker template has no static incoming edge, so `vanguard` alone
    // being empty here never means the run is done while a Muster is
    // pending.
    if vanguard.is_empty() && pending_muster.is_none() {
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
                None,
                checkpoint_ns.clone(),
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
            None,
            checkpoint_ns.clone(),
        );
        persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
        return Ok(RunOutcome::Completed {
            final_state: battlefield,
            waypoint: waypoint.waypoint_id,
        });
    }

    let mut frontier = Frontier::for_run(graph, &frontier_snapshot);

    // --- CF-03 / CF-FR-12: a validated `NextStep::Muster(tasks)` accepted
    // in superstep N is carried in `pending_muster` (declared above, before
    // the entry-vanguard-empty check, so a mid-muster resume's restored
    // value survives that check too), purely as a loop-local value never
    // itself persisted -- what IS persisted, incrementally, is each
    // completed task's delta into a progress Waypoint's own
    // `MusterProgress` (D-14). Dispatched as synthetic vanguard entries at
    // the top of this (for a resume) or the next (for a fresh Muster
    // acceptance) iteration below. `muster_carryover` pairs with
    // `pending_muster`: `Some(completed)` ONLY on the one iteration that
    // dispatches a mid-muster resume's restored task set (`.take()`n so it
    // is never mistakenly reapplied to a later, unrelated Muster).

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
                None,
                checkpoint_ns.clone(),
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
                    // --- CR-01 (23-REVIEW.md): `vanguard` alone may be
                    // empty on a muster-only round -- this phase's Muster
                    // feature can re-enter this loop with `vanguard` empty
                    // and `pending_muster` carrying the next dispatch
                    // (`has_pending_muster` a few hundred lines below
                    // stands in for "there is more work" in exactly this
                    // situation). Mirrors the `Battlefield::merge` failure
                    // fallback's `dispatch_entries.first()` pattern a few
                    // hundred lines below, using `pending_muster` instead
                    // since `dispatch_entries` is not built yet at this
                    // point in the loop. The loop's own Completed-return
                    // checks guarantee at least one of `vanguard`/
                    // `pending_muster` is non-empty whenever this branch is
                    // reached, so the final placeholder is unreachable by
                    // construction -- but must not panic if that invariant
                    // is ever violated (mirrors `MusterProgress::default`'s
                    // own placeholder `NodeId::new(String::new())`).
                    failed_node: vanguard
                        .first()
                        .cloned()
                        .or_else(|| pending_muster.as_ref().map(|(node, _)| node.clone()))
                        .unwrap_or_else(|| NodeId::new(String::new())),
                },
                visit_counts,
                frontier.snapshot(graph),
                None,
                checkpoint_ns.clone(),
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
                None,
                checkpoint_ns.clone(),
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

        // --- CF-03 / CF-FR-12: this superstep's dispatch entries = every
        // ordinary `vanguard` node (`muster: None`) PLUS every UNFINISHED
        // task from a Muster accepted in the PREVIOUS superstep, or
        // restored from a mid-muster resume (`pending_muster`, taken here
        // so it dispatches exactly once), each a synthetic entry sharing
        // its `worker` template's `NodeId` with `NodeContext.muster` set
        // (RESEARCH.md Pitfall 3: the SAME snapshot/spawn/semaphore
        // machinery ordinary vanguard nodes use, never a bespoke "run these
        // N tasks" loop). `muster_tasks` (the FULL task list) already
        // arrives sorted by `task_key` (`validate_muster_tasks`, or a
        // restored `MusterProgress.tasks`); `muster_carryover_this_round`
        // (D-14) removes any task already completed before an interruption
        // -- non-empty only for the one round that dispatches a mid-muster
        // resume's restored task set -- so `dispatch_tasks` never re-runs an
        // already-completed task. Filtering preserves `muster_tasks`'
        // relative task_key order; pushed in that order here, the existing
        // sequential-await-per-handle plus the stable `deltas.sort_by(NodeId)`
        // below preserve it into the final merge with no bespoke reordering.
        // Muster dispatch entries are NOT subject to
        // `visit_counts`/`max_node_visits` (that bound governs a node's own
        // re-entry into the vanguard across supersteps, e.g. a Goto refine
        // loop; a Muster's fan-out width is bounded separately by
        // `EngineLimits::max_muster_tasks`).
        let (muster_node, muster_tasks): (Option<NodeId>, Vec<MusterTask>) =
            match pending_muster.take() {
                Some((node, tasks)) => (Some(node), tasks),
                None => (None, Vec::new()),
            };
        let muster_carryover_this_round: BTreeMap<String, StateDelta> =
            muster_carryover.take().unwrap_or_default();
        let dispatch_tasks: Vec<MusterTask> = muster_tasks
            .iter()
            .filter(|task| !muster_carryover_this_round.contains_key(&task.task_key))
            .cloned()
            .collect();
        let muster_dispatch: Vec<(NodeId, Option<MusterContext>)> = dispatch_tasks
            .iter()
            .map(|task| {
                (
                    task.worker.clone(),
                    Some(MusterContext {
                        payload: task.payload.clone(),
                        task_key: task.task_key.clone(),
                    }),
                )
            })
            .collect();
        let dispatch_entries: Vec<(NodeId, Option<MusterContext>)> = vanguard
            .iter()
            .map(|id| (id.clone(), None))
            .chain(muster_dispatch)
            .collect();
        // --- CF-FR-12, D-14: running accumulator of this muster's
        // completed-but-unmerged task deltas -- seeded with the restored
        // carryover (if any), grown as each dispatched task in
        // `dispatch_tasks` succeeds below, and written onto every progress
        // Waypoint this round produces.
        let mut muster_completed_so_far: BTreeMap<String, StateDelta> = muster_carryover_this_round;

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
                NodeSpec::Battalion {
                    graph: child_graph,
                    state_map,
                    restart_on_resume,
                } => {
                    // --- CF-FR-16, D-21: fails this one node closed
                    // (never silently skips the child) when this `run()`
                    // call has no `Arc<W>` available -- see `run`'s own
                    // rustdoc note on `waypoint_port_arc`.
                    let resources = child_resources.clone().ok_or_else(|| {
                        EngineError::Node(NodeError(format!(
                            "battalion node {node_id}: no child-engine resources available for \
                             this run"
                        )))
                    })?;
                    NodeDispatch::Battalion {
                        graph: Arc::clone(child_graph),
                        state_map: state_map.clone(),
                        resources,
                        restart_on_resume: *restart_on_resume,
                    }
                }
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
        let mut mustered: Option<(NodeId, Vec<MusterTask>)> = None;
        for (entry, handle) in dispatch_entries.iter().zip(handles) {
            let (_entry_node_id, entry_muster_ctx) = entry;
            let is_muster_task = entry_muster_ctx.is_some();
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
                            // CF-03, D-13: validated here, at
                            // Directive-receipt time, before any task is
                            // dispatched -- the SAME per-node accumulation
                            // loop Goto validates in, never inside the
                            // worker-dispatch loop a later superstep runs.
                            notfiring_nodes.insert(node_id.clone());
                            match validate_muster_tasks(
                                graph,
                                &node_id,
                                graph.limits(),
                                tasks.clone(),
                            ) {
                                Ok(sorted_tasks) => {
                                    if mustered.is_none() {
                                        mustered = Some((node_id.clone(), sorted_tasks));
                                    }
                                }
                                Err(err) => {
                                    if routing_failure.is_none() {
                                        routing_failure = Some((node_id.clone(), err));
                                    }
                                }
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

                    if is_muster_task {
                        // --- CF-FR-12, D-14: a mustered worker task's own
                        // completion. Its delta is NOT pushed into `deltas`
                        // here -- that would merge it into `battlefield`
                        // before every sibling task has resolved, breaking
                        // ENG-FR-05 snapshot isolation. Instead: record it
                        // into the running `muster_completed_so_far` map and
                        // persist a progress Waypoint AT THIS SUPERSTEP's
                        // index, `status: Running`, whose `battlefield` is
                        // still the unmerged superstep-start snapshot --
                        // one progress Waypoint per completed task, bounded
                        // by `max_muster_tasks` (`validate_muster_tasks`
                        // already bounded `dispatch_tasks`' length before
                        // any task started). The consolidated,
                        // task_key-ordered fold into `deltas` happens once,
                        // after this whole loop (see below).
                        if let Some(task_key) =
                            entry_muster_ctx.as_ref().map(|ctx| ctx.task_key.clone())
                        {
                            muster_completed_so_far.insert(task_key, delta);
                        }
                        if let Some(node) = &muster_node {
                            let progress = MusterProgress {
                                node: node.clone(),
                                tasks: muster_tasks.clone(),
                                completed: muster_completed_so_far.clone(),
                            };
                            let progress_waypoint = build_waypoint(
                                &thread,
                                parent_waypoint_id,
                                superstep_number,
                                graph,
                                &battlefield,
                                vanguard.clone(),
                                completed_records.clone(),
                                WaypointStatus::Running,
                                visit_counts.clone(),
                                frontier.snapshot(graph),
                                Some(progress),
                                checkpoint_ns.clone(),
                            );
                            persist_waypoint(waypoint_port, durability, &progress_waypoint, trace)
                                .await?;
                            parent_waypoint_id = Some(progress_waypoint.waypoint_id);
                        }
                    } else {
                        deltas.push((node_id, delta));
                    }
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
                // --- CF-FR-16, D-21: already the fully-formed, structured
                // `EngineError::BattalionChildFailed` built in
                // `execute_vanguard_node`, where the child thread id was
                // in scope -- passed through unchanged.
                NodeFailure::Battalion(e) => e,
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
                None,
                checkpoint_ns.clone(),
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
                None,
                checkpoint_ns.clone(),
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        // --- CF-FR-12, D-14: fold this muster's completed-task deltas
        // (restored carryover + everything newly dispatched this round)
        // into `deltas`, in `muster_tasks`' own (task_key-sorted) order --
        // NOT in per-handle completion/await order -- so that siblings
        // sharing one worker template's `NodeId` retain their correct
        // relative task_key order after `deltas.sort_by(NodeId)`'s stable
        // sort below (CF-FR-11). This is the ONE point a Muster's deltas
        // enter the merge: never incrementally as each task completes
        // (that would break snapshot isolation for still-running siblings
        // and make a resumed run double-merge), exactly once here, whether
        // every task ran fresh this round or some were restored from a
        // mid-muster resume.
        // `muster_tasks` is non-empty only when `muster_node` is `Some`
        // (they are always constructed together above); iterating it
        // unconditionally is a no-op when no Muster was in play this round.
        for task in &muster_tasks {
            if let Some(delta) = muster_completed_so_far.get(&task.task_key) {
                deltas.push((task.worker.clone(), delta.clone()));
            }
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
                    None,
                    checkpoint_ns.clone(),
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
                None,
                checkpoint_ns.clone(),
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
                    None,
                    checkpoint_ns.clone(),
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
            None,
            checkpoint_ns.clone(),
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
    muster_progress: Option<MusterProgress>,
    // --- CF-FR-15, D-20: the namespace path this `run()` call's OWN
    // Waypoints are stamped with -- `None` for every top-level (non-child)
    // run, `Some("parent_node_id/")` (nested paths concatenating) for a
    // Battalion node's child run. A RECORD for observability only; carries
    // no isolation meaning (RESEARCH.md Pitfall 6) -- isolation comes
    // entirely from `thread` already being the child's own derived
    // `ThreadId` by the time this function is called.
    checkpoint_ns: Option<String>,
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
        muster_progress,
        checkpoint_ns,
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
    use std::sync::Mutex;

    use crate::engine::directive_parser::{DirectiveParser, OnParseError};
    use crate::engine::graph::EdgeSpec;
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
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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
            None,
            1,
            port,
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
        )
        .await
        .unwrap()
    }

    /// Like `run_default`, but seeding a mid-muster resume: `battlefield`,
    /// `vanguard`, `visit_counts`, `frontier_snapshot`, `muster_progress`
    /// and `superstep_number` all come from a caller-loaded Waypoint,
    /// exactly as `WarEngine::resume_with_options` (`engine::mod`) would
    /// compute them for a `muster_progress: Some(..)` Waypoint (CF-FR-12,
    /// D-14): `superstep_number` equal to that Waypoint's own `superstep`
    /// (never `+ 1`).
    #[allow(clippy::too_many_arguments)]
    async fn run_resumed_mid_muster(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
        battlefield: Battlefield,
        vanguard: Vec<NodeId>,
        visit_counts: BTreeMap<NodeId, u32>,
        frontier_snapshot: FrontierSnapshot,
        muster_progress: MusterProgress,
        superstep_number: u64,
    ) -> RunOutcome {
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            graph,
            thread,
            battlefield,
            vanguard,
            visit_counts,
            Some(frontier_snapshot),
            Some(muster_progress),
            None,
            superstep_number,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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
            None,
            2,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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

    // --- CF-03, D-13: malformed-Muster rejection, before any task starts
    // (Plan 23-05, Task 2).

    #[tokio::test]
    async fn duplicate_task_key_fails_before_any_task_starts() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let worker_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![
                    muster_task(&worker, serde_json::json!("a"), "dup"),
                    muster_task(&worker, serde_json::json!("b"), "dup"),
                ]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node.clone()));
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-dup-key").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::DuplicateMusterTaskKey { node, task_key },
                ..
            } => {
                assert_eq!(node, planner);
                assert_eq!(task_key, "dup");
            }
            other => panic!("expected Failed(DuplicateMusterTaskKey), got {other:?}"),
        }
        assert_eq!(worker_node.run_count(), 0, "no task may start");
    }

    #[tokio::test]
    async fn muster_exceeding_the_limit_fails_before_any_task_starts() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(
            s,
            EngineLimits {
                max_muster_tasks: 2,
                ..EngineLimits::default()
            },
        );
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let worker_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
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
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-limit-exceeded").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error:
                    EngineError::MusterTaskLimitExceeded {
                        node,
                        requested,
                        limit,
                    },
                ..
            } => {
                assert_eq!(node, planner);
                assert_eq!(requested, 3);
                assert_eq!(limit, 2);
            }
            other => panic!("expected Failed(MusterTaskLimitExceeded), got {other:?}"),
        }
        assert_eq!(worker_node.run_count(), 0, "no task may start");
    }

    #[tokio::test]
    async fn muster_of_exactly_the_limit_runs() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(
            s,
            EngineLimits {
                max_muster_tasks: 3,
                ..EngineLimits::default()
            },
        );
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let worker_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
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
        graph.add_entry(planner);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-exactly-limit").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(worker_node.run_count(), 3);
    }

    // ── CF-FR-12, D-14: mid-muster crash survival (Plan 23-06) ───────────

    /// A `planner -> Muster(five tasks: a,b,c,d,e) -> worker template`
    /// fixture over an `Append` field keyed by `ctx.task_key()`: the
    /// planner runs at superstep 1, the muster dispatches at superstep 2 --
    /// the shape every test below shares. Each worker execution is also
    /// recorded, in order, into `executed_keys`, so a resume test can
    /// assert precisely which task_keys ran on a given `run()` call.
    fn five_task_muster_graph(
        results_field: &FieldName,
        executed_keys: Arc<Mutex<Vec<String>>>,
    ) -> (WarGraph, NodeId) {
        struct KeyRecordingWorkerNode {
            field: FieldName,
            executed_keys: Arc<Mutex<Vec<String>>>,
        }
        #[async_trait::async_trait]
        impl StateNode for KeyRecordingWorkerNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &crate::engine::node::NodeContext,
            ) -> Result<Directive, NodeError> {
                let key = ctx.task_key().unwrap_or_default().to_string();
                self.executed_keys.lock().unwrap().push(key.clone());
                let mut delta = StateDelta::new();
                delta.set_raw(self.field.clone(), serde_json::json!(key));
                Ok(delta.into())
            }
        }

        let s = schema(vec![FieldSpec::new(
            results_field.clone(),
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
                next: NextStep::Muster(
                    ["a", "b", "c", "d", "e"]
                        .iter()
                        .map(|k| muster_task(&worker, serde_json::json!(*k), k))
                        .collect(),
                ),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::Function(std::sync::Arc::new(KeyRecordingWorkerNode {
                field: results_field.clone(),
                executed_keys,
            })),
        );
        graph.add_entry(planner);
        (graph, worker)
    }

    /// Waypoints saved for `thread`, oldest-first (chronological) --
    /// [`RecordingWaypointStore::saved_waypoints`] returns newest-first;
    /// this just reverses that for tests that want to walk a run's history
    /// forward.
    async fn ascending_saved_waypoints(
        store: &RecordingWaypointStore,
        thread: &ThreadId,
    ) -> Vec<Waypoint> {
        let mut waypoints = store.saved_waypoints(thread).await;
        waypoints.reverse();
        waypoints
    }

    #[tokio::test]
    async fn progress_waypoint_battlefield_equals_the_superstep_start_snapshot() {
        let results_field = field("results");
        let (graph, _worker) =
            five_task_muster_graph(&results_field, Arc::new(Mutex::new(Vec::new())));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-progress-unmerged-battlefield").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let ascending = ascending_saved_waypoints(&store, &thread).await;
        let progress_waypoints: Vec<&Waypoint> = ascending
            .iter()
            .filter(|w| w.superstep == 2 && w.muster_progress.is_some())
            .collect();
        assert_eq!(
            progress_waypoints.len(),
            5,
            "one progress Waypoint per task"
        );
        for wp in &progress_waypoints {
            let results = wp.battlefield.get::<Vec<String>>(&results_field).unwrap();
            assert!(
                results.is_none(),
                "a progress Waypoint's battlefield must still be the unmerged \
                 superstep-start snapshot, got {results:?}"
            );
        }

        let complete_waypoint = ascending
            .iter()
            .find(|w| w.superstep == 2 && w.muster_progress.is_none())
            .expect("the superstep-complete waypoint for superstep 2");
        let merged = complete_waypoint
            .battlefield
            .get::<Vec<String>>(&results_field)
            .unwrap()
            .unwrap();
        assert_eq!(
            merged,
            vec!["a", "b", "c", "d", "e"],
            "the merge happens exactly once, after every task resolves"
        );
    }

    #[tokio::test]
    async fn progress_waypoints_are_written_at_the_same_superstep_index_with_status_running() {
        let results_field = field("results");
        let (graph, _worker) =
            five_task_muster_graph(&results_field, Arc::new(Mutex::new(Vec::new())));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-progress-superstep-status").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let ascending = ascending_saved_waypoints(&store, &thread).await;
        let progress_waypoints: Vec<&Waypoint> = ascending
            .iter()
            .filter(|w| w.muster_progress.is_some())
            .collect();
        assert_eq!(progress_waypoints.len(), 5);
        for wp in &progress_waypoints {
            assert_eq!(
                wp.superstep, 2,
                "every progress Waypoint shares the muster's own superstep"
            );
            assert_eq!(wp.status, WaypointStatus::Running);
        }

        let complete_waypoints: Vec<&Waypoint> = ascending
            .iter()
            .filter(|w| w.superstep == 2 && w.muster_progress.is_none())
            .collect();
        assert_eq!(
            complete_waypoints.len(),
            1,
            "exactly one superstep-complete Waypoint follows the progress Waypoints"
        );
        assert_eq!(complete_waypoints[0].status, WaypointStatus::Completed);
    }

    #[tokio::test]
    async fn one_progress_waypoint_per_completed_task() {
        let results_field = field("results");
        let (graph, worker) =
            five_task_muster_graph(&results_field, Arc::new(Mutex::new(Vec::new())));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-progress-cadence").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let ascending = ascending_saved_waypoints(&store, &thread).await;
        let progress_records_for_worker: usize = ascending
            .iter()
            .filter_map(|w| w.muster_progress.as_ref())
            .filter(|p| p.node == NodeId::new("planner"))
            .count();
        assert_eq!(progress_records_for_worker, 5);
        let _ = worker;
    }

    #[tokio::test]
    async fn resume_mid_muster_runs_exactly_the_unfinished_tasks() {
        let results_field = field("results");
        let control_keys = Arc::new(Mutex::new(Vec::new()));
        let (control_graph, _worker) = five_task_muster_graph(&results_field, control_keys);

        let control_store = RecordingWaypointStore::new();
        let control_thread = ThreadId::new("muster-resume-control").unwrap();
        let control_outcome =
            run_default(&control_graph, control_thread.clone(), &control_store).await;
        assert!(matches!(control_outcome, RunOutcome::Completed { .. }));

        // Drop the engine after two of five tasks: copy only the planner's
        // own waypoint plus the first two progress Waypoints (tasks "a" and
        // "b") into a fresh store, simulating a crash before task "c"'s
        // progress Waypoint was ever written.
        let ascending = ascending_saved_waypoints(&control_store, &control_thread).await;
        let progress_waypoints: Vec<&Waypoint> = ascending
            .iter()
            .filter(|w| w.superstep == 2 && w.muster_progress.is_some())
            .collect();
        assert_eq!(progress_waypoints.len(), 5);

        let truncated_store = RecordingWaypointStore::new();
        // Planner's own superstep-1 waypoint, then the first two progress
        // Waypoints (2 of 5 tasks done).
        truncated_store.save(&ascending[0]).await.unwrap();
        truncated_store.save(progress_waypoints[0]).await.unwrap();
        truncated_store.save(progress_waypoints[1]).await.unwrap();

        let latest = truncated_store
            .latest(&control_thread)
            .await
            .unwrap()
            .expect("a waypoint was saved");
        assert_eq!(
            latest.waypoint_id, progress_waypoints[1].waypoint_id,
            "latest() must return the most recently written progress Waypoint"
        );
        let progress = latest
            .muster_progress
            .clone()
            .expect("the latest waypoint is a mid-muster progress record");
        assert_eq!(progress.completed.len(), 2);

        let resumed_keys = Arc::new(Mutex::new(Vec::new()));
        let (resume_graph, _worker) = five_task_muster_graph(&results_field, resumed_keys.clone());
        let resume_store = RecordingWaypointStore::new();
        let resumed_outcome = run_resumed_mid_muster(
            &resume_graph,
            control_thread.clone(),
            &resume_store,
            latest.battlefield.clone(),
            latest.vanguard.clone(),
            latest.visit_counts.clone(),
            latest.frontier.clone(),
            progress,
            latest.superstep,
        )
        .await;
        assert!(matches!(resumed_outcome, RunOutcome::Completed { .. }));

        let executed = resumed_keys.lock().unwrap().clone();
        assert_eq!(
            executed,
            vec!["c".to_string(), "d".to_string(), "e".to_string()],
            "exactly the three unfinished tasks must run, none of the two already-completed"
        );
    }

    #[tokio::test]
    async fn resumed_muster_final_battlefield_equals_the_uninterrupted_run() {
        let results_field = field("results");
        let control_keys = Arc::new(Mutex::new(Vec::new()));
        let (control_graph, _worker) = five_task_muster_graph(&results_field, control_keys);

        let control_store = RecordingWaypointStore::new();
        let control_thread = ThreadId::new("muster-resume-equality-control").unwrap();
        let control_outcome =
            run_default(&control_graph, control_thread.clone(), &control_store).await;
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        let ascending = ascending_saved_waypoints(&control_store, &control_thread).await;
        let progress_waypoints: Vec<&Waypoint> = ascending
            .iter()
            .filter(|w| w.superstep == 2 && w.muster_progress.is_some())
            .collect();

        let truncated_store = RecordingWaypointStore::new();
        truncated_store.save(&ascending[0]).await.unwrap();
        truncated_store.save(progress_waypoints[0]).await.unwrap();
        truncated_store.save(progress_waypoints[1]).await.unwrap();
        let latest = truncated_store
            .latest(&control_thread)
            .await
            .unwrap()
            .unwrap();
        let progress = latest.muster_progress.clone().unwrap();

        let resumed_keys = Arc::new(Mutex::new(Vec::new()));
        let (resume_graph, _worker) = five_task_muster_graph(&results_field, resumed_keys);
        let resume_store = RecordingWaypointStore::new();
        let resumed_outcome = run_resumed_mid_muster(
            &resume_graph,
            control_thread.clone(),
            &resume_store,
            latest.battlefield.clone(),
            latest.vanguard.clone(),
            latest.visit_counts.clone(),
            latest.frontier.clone(),
            progress,
            latest.superstep,
        )
        .await;
        let resumed_final = match resumed_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected resumed run to complete, got {other:?}"),
        };

        assert_eq!(
            serde_json::to_string(&resumed_final).unwrap(),
            serde_json::to_string(&control_final).unwrap(),
            "a resumed mid-muster run must reach the uninterrupted run's final Battlefield"
        );
    }

    #[tokio::test]
    async fn strict_durability_failure_on_a_progress_write_fails_the_run() {
        let results_field = field("results");
        let (graph, _worker) =
            five_task_muster_graph(&results_field, Arc::new(Mutex::new(Vec::new())));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-progress-strict-durability").unwrap();
        // Call #1 is the planner's own superstep-1 waypoint; calls #2..#6
        // are the five progress Waypoints (task "a" is call #2, "b" is call
        // #3, ...). Fail call #3: task "a"'s checkpoint saves successfully,
        // task "b"'s checkpoint write fails.
        store.fail_nth_save(3);

        let result = run(
            &store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread.clone(),
            Battlefield::initialize(
                graph.schema().clone(),
                &paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
        )
        .await;

        assert!(matches!(result, Err(EngineError::WaypointWrite { .. })));

        // The one progress Waypoint that saved successfully before the
        // failure (task "a"'s) is still durably persisted -- a future
        // resume can still recover from it.
        let saved = store.saved_waypoints(&thread).await;
        let progress_count = saved.iter().filter(|w| w.muster_progress.is_some()).count();
        assert_eq!(progress_count, 1);
    }

    #[tokio::test]
    async fn best_effort_durability_failure_on_a_progress_write_continues() {
        let results_field = field("results");
        let (graph, worker) =
            five_task_muster_graph(&results_field, Arc::new(Mutex::new(Vec::new())));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-progress-best-effort-durability").unwrap();
        // Same target call as the Strict test above (task "b"'s checkpoint
        // write, call #3) -- but under BestEffort the run must continue
        // past it and still complete, with all five tasks having run.
        store.fail_nth_save(3);

        let result = run(
            &store,
            WaypointDurability::BestEffort,
            None,
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &graph,
            thread.clone(),
            Battlefield::initialize(
                graph.schema().clone(),
                &paladin_core::platform::container::battlefield::StateDelta::new(),
            )
            .unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
        )
        .await
        .unwrap();

        match result {
            RunOutcome::Completed { final_state, .. } => {
                let merged = final_state
                    .get::<Vec<String>>(&results_field)
                    .unwrap()
                    .unwrap();
                assert_eq!(merged, vec!["a", "b", "c", "d", "e"]);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        let _ = worker;

        // Four progress Waypoints saved successfully (the failed one was
        // swallowed under BestEffort), plus the final superstep-complete
        // Waypoint.
        let saved = store.saved_waypoints(&thread).await;
        let progress_count = saved.iter().filter(|w| w.muster_progress.is_some()).count();
        assert_eq!(progress_count, 4);
    }

    #[tokio::test]
    async fn empty_muster_fails_with_a_typed_error() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let planner_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Muster(vec![]),
        });
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-empty").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::EmptyMuster { node },
                ..
            } => {
                assert_eq!(node, planner);
            }
            other => panic!("expected Failed(EmptyMuster), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn muster_naming_an_unknown_worker_fails() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let ghost = NodeId::new("ghost");
        let planner_node = {
            let ghost = ghost.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![muster_task(&ghost, serde_json::json!("a"), "a")]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-unknown-worker").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::MusterUnknownWorker { node, worker },
                ..
            } => {
                assert_eq!(node, planner);
                assert_eq!(worker, ghost);
            }
            other => panic!("expected Failed(MusterUnknownWorker), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn muster_naming_a_non_template_node_fails() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let not_a_template = NodeId::new("not-a-template");
        let not_a_template_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let planner_node = {
            let not_a_template = not_a_template.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![muster_task(
                    &not_a_template,
                    serde_json::json!("a"),
                    "a",
                )]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_node(
            not_a_template.clone(),
            NodeSpec::Function(not_a_template_node.clone()),
        );
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-not-a-template").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Failed {
                error: EngineError::MusterWorkerNotATemplate { node, worker },
                ..
            } => {
                assert_eq!(node, planner);
                assert_eq!(worker, not_a_template);
            }
            other => panic!("expected Failed(MusterWorkerNotATemplate), got {other:?}"),
        }
        assert_eq!(not_a_template_node.run_count(), 0, "no task may start");
    }

    #[test]
    fn task_count_check_does_not_narrow_the_length() {
        // If the comparison narrowed `count` with `as u32`, a count of
        // `u32::MAX as usize + 1` would wrap to 0 and (incorrectly) NOT
        // exceed even a limit of 1. The widening comparison correctly
        // reports it as exceeding.
        let count = u32::MAX as usize + 1;
        assert!(muster_task_count_exceeds_limit(count, 1));
        assert!(!muster_task_count_exceeds_limit(5, 10));
        assert!(muster_task_count_exceeds_limit(10, 9));
        assert!(!muster_task_count_exceeds_limit(10, 10));
    }

    // --- CF-03, D-15: the `muster.` InputMapping namespace, and CF-FR-11's
    // ≥20-iteration determinism repeat test (Plan 23-05, Task 3).

    #[tokio::test]
    async fn worker_input_template_resolves_the_muster_payload_placeholder() {
        let out_field = field("out");
        let s = schema(vec![FieldSpec::new(
            out_field.clone(),
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
                next: NextStep::Muster(vec![muster_task(
                    &worker,
                    serde_json::json!("widget-1"),
                    "only",
                )]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::paladin(
                make_paladin("worker"),
                InputMapping::new("process {muster.payload}"),
                out_field,
            ),
        );
        graph.add_entry(planner);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("worker", "done");
        let port: Arc<dyn PaladinPort> = recording.clone();

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-payload-placeholder").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let call_log = recording.call_log();
        assert_eq!(call_log.len(), 1);
        assert_eq!(call_log[0].1, "process widget-1");
    }

    #[tokio::test]
    async fn worker_input_template_resolves_the_task_key_placeholder() {
        let out_field = field("out");
        let s = schema(vec![FieldSpec::new(
            out_field.clone(),
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
                next: NextStep::Muster(vec![muster_task(
                    &worker,
                    serde_json::json!("ignored"),
                    "task-99",
                )]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(
            worker.clone(),
            NodeSpec::paladin(
                make_paladin("worker"),
                InputMapping::new("key={muster.task_key}"),
                out_field,
            ),
        );
        graph.add_entry(planner);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("worker", "done");
        let port: Arc<dyn PaladinPort> = recording.clone();

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-task-key-placeholder").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let call_log = recording.call_log();
        assert_eq!(call_log.len(), 1);
        assert_eq!(call_log[0].1, "key=task-99");
    }

    #[tokio::test]
    async fn muster_placeholders_never_resolve_from_the_battlefield() {
        // An ordinary (non-Muster) Paladin node whose InputMapping
        // references {muster.payload} must fail typed -- no muster context
        // is present, so the placeholder is never satisfied by a
        // same-named Battlefield field or any other silent fallback.
        let out_field = field("out");
        let s = schema(vec![FieldSpec::new(
            out_field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let solo = NodeId::new("solo");
        graph.add_node(
            solo.clone(),
            NodeSpec::paladin(
                make_paladin("solo"),
                InputMapping::new("{muster.payload}"),
                out_field,
            ),
        );
        graph.add_entry(solo.clone());

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("solo", "done");
        let port: Arc<dyn PaladinPort> = recording.clone();

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-placeholder-no-context").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;
        match outcome {
            // `execute_vanguard_node`'s Paladin arm wraps an
            // `InputMapping::render` failure as the same generic
            // `EngineError::Node` every other node-execution failure uses
            // (not a dedicated `EngineError::InputMapping` -- that variant
            // wraps a DIFFERENT, unrelated call path); the message names
            // the unresolved placeholder.
            RunOutcome::Failed {
                error: EngineError::Node(NodeError(message)),
                ..
            } => {
                assert!(
                    message.contains("muster.payload"),
                    "error must name the unresolved placeholder, got: {message}"
                );
            }
            other => panic!("expected Failed(Node(..)), got {other:?}"),
        }
        assert_eq!(
            recording.call_count(),
            0,
            "the Paladin must never be called with an unresolved placeholder"
        );
    }

    #[tokio::test]
    async fn task_key_order_is_stable_across_twenty_shuffled_runs() {
        // CF-FR-11: at least 20 iterations through the seeded-shuffle
        // determinism harness (Phase 22 D-11), each worker's completion
        // order perturbed by a per-iteration seeded delay assignment, every
        // iteration's final Battlefield asserted byte-identical to the
        // lexicographic-key reference.
        struct SeededDelayWorkerNode {
            field: FieldName,
            delays_by_key: std::collections::HashMap<String, u64>,
        }
        #[async_trait::async_trait]
        impl StateNode for SeededDelayWorkerNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &crate::engine::node::NodeContext,
            ) -> Result<Directive, NodeError> {
                let key = ctx.task_key().unwrap_or_default().to_string();
                let delay_ms = self.delays_by_key.get(&key).copied().unwrap_or(0);
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                let mut delta = StateDelta::new();
                delta.set_raw(self.field.clone(), serde_json::json!(key));
                Ok(delta.into())
            }
        }

        let keys = ["a", "b", "c", "d", "e"];
        let order_field = field("order");
        let expected: Vec<String> = keys.iter().map(|k| k.to_string()).collect();

        for seed in 0..20u64 {
            let mut delay_values: Vec<u64> = (0..keys.len() as u64).collect();
            shuffle_seeded(&mut delay_values, seed);
            let delays_by_key: std::collections::HashMap<String, u64> = keys
                .iter()
                .zip(delay_values.iter())
                .map(|(k, d)| (k.to_string(), *d * 5))
                .collect();

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
                    next: NextStep::Muster(
                        keys.iter()
                            .map(|k| muster_task(&worker, serde_json::json!(k), k))
                            .collect(),
                    ),
                })
            };
            graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
            graph.add_worker_template(
                worker.clone(),
                NodeSpec::Function(std::sync::Arc::new(SeededDelayWorkerNode {
                    field: order_field.clone(),
                    delays_by_key,
                })),
            );
            graph.add_entry(planner);

            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("muster-determinism-{seed}")).unwrap();
            let outcome = run_default(&graph, thread, &store).await;
            match outcome {
                RunOutcome::Completed { final_state, .. } => {
                    assert_eq!(
                        final_state.get::<Vec<String>>(&order_field).unwrap(),
                        Some(expected.clone()),
                        "seed {seed}: final Battlefield must equal the reference \
                         lexicographic-key run"
                    );
                }
                other => panic!("seed {seed}: expected Completed, got {other:?}"),
            }
        }
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
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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
            None,
            1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
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
    async fn muster_only_round_at_recursion_limit_fails_without_panicking() {
        // CR-01 regression (23-REVIEW.md): a muster-only round (the
        // mustering node's only arm is a worker template, which has no
        // static incoming edge per D-12) leaves `vanguard` empty for the
        // next superstep while `pending_muster` carries the dispatch
        // forward. If `max_supersteps` is tight enough that the DISPATCH
        // superstep itself trips the recursion limit, the
        // `RecursionLimitExceeded` branch used to index `vanguard[0]`
        // unconditionally and panic on the empty Vec. It must instead fail
        // closed with a typed `EngineError`, never panic.
        let s = schema(vec![]);
        let mut graph = WarGraph::new(
            s,
            EngineLimits {
                max_supersteps: 2,
                ..EngineLimits::default()
            },
        );
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let worker_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(vec![muster_task(&worker, serde_json::json!("a"), "a")]),
            })
        };
        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node.clone()));
        graph.add_entry(planner.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("muster-only-recursion-limit").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Failed { error, waypoint } => {
                assert!(matches!(
                    error,
                    EngineError::RecursionLimitExceeded { limit: 2, .. }
                ));
                assert!(waypoint.is_some());
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert_eq!(
            worker_node.run_count(),
            0,
            "the muster dispatch superstep must never run -- the limit trips before dispatch"
        );
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

    // --- Plan 23-08: NodeSpec::Battalion (subgraph composition) ----------

    /// Like `run_default`, but threading a REAL `Arc<W>` for the
    /// `waypoint_port_arc` seam (CF-FR-16, D-21) so a `NodeSpec::Battalion`
    /// node's child run can actually construct its `ChildEngineResources`
    /// -- `run_default`'s own bare `&RecordingWaypointStore` has no owning
    /// `Arc` to hand over. `store` drives both the borrowed and the
    /// `Arc`-cloned parameter, so a Battalion child's own persisted
    /// Waypoints land in the SAME store a test then inspects.
    #[allow(clippy::too_many_arguments)]
    async fn run_with_children(
        graph: &WarGraph,
        thread: ThreadId,
        initial: StateDelta,
        store: &Arc<RecordingWaypointStore>,
        port: &Arc<dyn PaladinPort>,
        registry: &CustomDispatchResolver,
        evaluators: &EdgeEvaluatorRegistry,
        cancellation: &Option<CancellationToken>,
    ) -> Result<RunOutcome, EngineError> {
        run(
            store.as_ref(),
            WaypointDurability::Strict,
            None,
            registry,
            evaluators,
            graph,
            thread,
            Battlefield::initialize(graph.schema().clone(), &initial).unwrap(),
            graph.entry().to_vec(),
            BTreeMap::new(),
            None,
            None,
            None,
            1,
            port,
            &no_trace(),
            &no_interceptors(),
            cancellation,
            Some(Arc::clone(store)),
        )
        .await
    }

    /// The SAME injective, length-prefixed derivation `execute_vanguard_node`'s
    /// `NodeSpec::Battalion` arm uses in production (`ThreadId::child`), so
    /// every test asserting on a child's derived thread id exercises the
    /// real derivation rather than a parallel test-only encoding that could
    /// silently drift from it.
    fn child_thread_id(parent: &ThreadId, node: &str) -> ThreadId {
        ThreadId::child(parent, &NodeId::new(node)).unwrap()
    }

    #[tokio::test]
    async fn battalion_node_runs_its_child_graph_to_completion() {
        let child_result = field("child_result");
        let child_schema = schema(vec![FieldSpec::new(
            child_result.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let c1 = NodeId::new("c1");
        let c2 = NodeId::new("c2");
        let c1_ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let c2_ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let flag = Arc::clone(&c1_ran);
            child.add_node(
                c1.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    StateDelta::new()
                })),
            );
        }
        {
            let flag = Arc::clone(&c2_ran);
            let result_field = child_result.clone();
            child.add_node(
                c2.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    let mut delta = StateDelta::new();
                    delta.set_raw(result_field.clone(), serde_json::json!("done"));
                    delta
                })),
            );
        }
        child.add_edge(EdgeSpec {
            from: c1.clone(),
            to: c2.clone(),
            condition: Some(EdgeCondition::Always),
        });
        child.add_entry(c1);

        let sub = NodeId::new("sub");
        let mut parent = WarGraph::new(schema(vec![]), EngineLimits::default());
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-basic").unwrap();
        let outcome = run_with_children(
            &parent,
            thread,
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "expected Completed, got {outcome:?}"
        );
        assert!(
            c1_ran.load(std::sync::atomic::Ordering::SeqCst),
            "child entry node must have run"
        );
        assert!(
            c2_ran.load(std::sync::atomic::Ordering::SeqCst),
            "child second node must have run"
        );
    }

    #[tokio::test]
    async fn state_map_inputs_seed_the_child_schema() {
        let parent_topic = field("parent_topic");
        let observed = field("observed");
        let child_topic = field("child_topic");
        let child_out = field("child_out");

        let child_schema = schema(vec![
            FieldSpec::new(child_topic.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(child_out.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let reader = NodeId::new("reader");
        {
            let read_field = child_topic.clone();
            let write_field = child_out.clone();
            child.add_node(
                reader.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, state| {
                    let value: Option<String> = state.get(&read_field).unwrap();
                    let mut delta = StateDelta::new();
                    delta.set_raw(write_field.clone(), serde_json::json!(value));
                    delta
                })),
            );
        }
        child.add_entry(reader);

        let parent_schema = schema(vec![
            FieldSpec::new(parent_topic.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(observed.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new()
            .with_input(parent_topic.clone(), child_topic)
            .with_output(child_out, observed.clone());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-inputs").unwrap();
        let mut initial = StateDelta::new();
        initial.set(parent_topic, "rust").unwrap();

        let outcome = run_with_children(
            &parent,
            thread,
            initial,
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&observed).unwrap(),
                    Some("rust".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn state_map_outputs_return_as_the_parent_nodes_delta() {
        let child_note = field("child_note");
        let child_schema = schema(vec![FieldSpec::new(
            child_note.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let writer = NodeId::new("writer");
        child.add_node(
            writer.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                child_note.clone(),
                serde_json::json!("from-child"),
            )),
        );
        child.add_entry(writer);

        let notes = field("notes");
        let parent_schema = schema(vec![FieldSpec::new(
            notes.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(child_note, notes.clone());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-outputs-append").unwrap();
        let mut initial = StateDelta::new();
        initial
            .set(notes.clone(), vec!["existing".to_string()])
            .unwrap();

        let outcome = run_with_children(
            &parent,
            thread,
            initial,
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let values: Vec<String> = final_state.get(&notes).unwrap().unwrap();
                assert_eq!(
                    values,
                    vec!["existing".to_string(), "from-child".to_string()],
                    "the child's output must merge through the PARENT's Append dispatch rule"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unmapped_child_fields_stay_private() {
        let secret = field("secret");
        let visible = field("visible");
        let child_schema = schema(vec![
            FieldSpec::new(secret.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(visible.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let writer = NodeId::new("writer");
        child.add_node(
            writer.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| {
                let mut delta = StateDelta::new();
                delta.set_raw(
                    FieldName::new("secret").unwrap(),
                    serde_json::json!("TOP_SECRET_VALUE"),
                );
                delta.set_raw(
                    FieldName::new("visible").unwrap(),
                    serde_json::json!("public"),
                );
                delta
            })),
        );
        child.add_entry(writer);

        let out = field("out");
        let parent_schema = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(visible, out);
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-privacy").unwrap();
        let outcome = run_with_children(
            &parent,
            thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        let final_state = match outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };
        let serialized = serde_json::to_string(&final_state).unwrap();
        assert!(
            !serialized.contains("secret"),
            "unmapped child field name must not appear in the parent Battlefield: {serialized}"
        );
        assert!(
            !serialized.contains("TOP_SECRET_VALUE"),
            "unmapped child field value must not appear in the parent Battlefield: {serialized}"
        );

        let saved = store.saved_waypoints(&thread).await;
        assert!(!saved.is_empty());
        for wp in &saved {
            let wp_json = serde_json::to_string(&wp.battlefield).unwrap();
            assert!(
                !wp_json.contains("TOP_SECRET_VALUE"),
                "unmapped child field value must not appear in any parent Waypoint payload: \
                 {wp_json}"
            );
        }
    }

    #[tokio::test]
    async fn one_parent_superstep_spans_the_whole_child_run() {
        let child_out = field("child_out");
        let child_schema = schema(vec![FieldSpec::new(
            child_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let c = NodeId::new("c");
        child.add_node(
            a.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_r, _s| StateDelta::new())),
        );
        child.add_node(
            b.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_r, _s| StateDelta::new())),
        );
        child.add_node(
            c.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                child_out.clone(),
                serde_json::json!("done"),
            )),
        );
        child.add_edge(EdgeSpec {
            from: a.clone(),
            to: b.clone(),
            condition: Some(EdgeCondition::Always),
        });
        child.add_edge(EdgeSpec {
            from: b.clone(),
            to: c.clone(),
            condition: Some(EdgeCondition::Always),
        });
        child.add_entry(a);

        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let sub = NodeId::new("sub");
        let state_map = StateMap::new().with_output(child_out, parent_out);
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-one-superstep").unwrap();
        let outcome = run_with_children(
            &parent,
            thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        assert_eq!(
            saved.len(),
            1,
            "exactly one parent Waypoint regardless of the child's own 3 supersteps"
        );
        assert_eq!(
            saved[0].superstep, 1,
            "the parent's superstep index must advance by exactly one"
        );
    }

    #[tokio::test]
    async fn child_inherits_every_parent_engine_resource() {
        let out_field = field("out");
        let score_field = field("score");
        let child_schema = schema(vec![
            FieldSpec::new(out_field.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(
                score_field.clone(),
                DispatchRule::Custom("double".to_string()),
                Some(serde_json::json!(0)),
                false,
            ),
        ]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let paladin_node = NodeId::new("child_paladin");
        child.add_node(
            paladin_node.clone(),
            NodeSpec::paladin(
                make_paladin("child_paladin"),
                InputMapping::new("go"),
                out_field.clone(),
            ),
        );
        let scorer = NodeId::new("scorer");
        child.add_node(
            scorer.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                score_field.clone(),
                serde_json::json!(21),
            )),
        );
        child.add_entry(paladin_node);
        child.add_entry(scorer);

        let sub = NodeId::new("sub");
        let parent_out = field("parent_out");
        let parent_score = field("parent_score");
        let parent_schema = schema(vec![
            FieldSpec::new(parent_out.clone(), DispatchRule::LastWrite, None, false),
            FieldSpec::new(parent_score.clone(), DispatchRule::LastWrite, None, false),
        ]);
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        let state_map = StateMap::new()
            .with_output(out_field, parent_out.clone())
            .with_output(score_field, parent_score.clone());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let port = Arc::new(RecordingPaladinPort::new());
        port.set_output("child_paladin", "child output");
        let port_dyn: Arc<dyn PaladinPort> = port.clone();
        let mut registry = CustomDispatchResolver::new();
        registry.insert(
            "double".to_string(),
            Arc::new(|_c: &serde_json::Value, d: &serde_json::Value| {
                Ok(serde_json::json!(d.as_i64().unwrap_or(0) * 2))
            }),
        );

        let thread = ThreadId::new("battalion-resources").unwrap();
        let outcome = run_with_children(
            &parent,
            thread.clone(),
            StateDelta::new(),
            &store,
            &port_dyn,
            &registry,
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&parent_out).unwrap(),
                    Some("child output".to_string())
                );
                // --- the dispatch resolver: `double` only exists on the
                // PARENT's registry; a resolved score of 42 (21 * 2) proves
                // the CHILD's own merge used it, not a default fallback.
                assert_eq!(final_state.get::<i64>(&parent_score).unwrap(), Some(42));
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        // --- the PaladinPort: the child's own Paladin executed through the
        // PARENT's port instance.
        assert_eq!(
            port.call_log(),
            vec![("child_paladin".to_string(), "go".to_string())]
        );

        // --- the WaypointPort: the child's own run persisted through the
        // PARENT's store, under the deterministic child thread id.
        let child_thread = child_thread_id(&thread, "sub");
        let child_waypoints = store.saved_waypoints(&child_thread).await;
        assert!(
            !child_waypoints.is_empty(),
            "child run must persist through the parent's WaypointPort"
        );
    }

    #[tokio::test]
    async fn child_uses_its_own_engine_limits() {
        let looper = NodeId::new("looper");
        let mut child = WarGraph::new(
            schema(vec![]),
            EngineLimits {
                max_supersteps: 2,
                ..EngineLimits::default()
            },
        );
        {
            let looper = looper.clone();
            child.add_node(
                looper.clone(),
                NodeSpec::Function(CountingFunctionNode::with_directive(move |_run, _state| {
                    Directive {
                        delta: StateDelta::new(),
                        next: NextStep::Goto(vec![looper.clone()]),
                    }
                })),
            );
        }
        child.add_entry(looper);

        let mut parent = WarGraph::new(schema(vec![]), EngineLimits::default());
        let sub = NodeId::new("sub");
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_entry(sub);

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-own-limits").unwrap();
        let outcome = run_with_children(
            &parent,
            thread,
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => match error {
                EngineError::BattalionChildFailed { source, .. } => {
                    assert!(
                        matches!(
                            *source,
                            EngineError::RecursionLimitExceeded { limit: 2, .. }
                        ),
                        "expected the CHILD's own max_supersteps (2) to trip, got {source:?}"
                    );
                }
                other => panic!("expected BattalionChildFailed, got {other:?}"),
            },
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn child_failure_surfaces_as_a_structured_node_error() {
        let mut child = WarGraph::new(schema(vec![]), EngineLimits::default());
        let failing = NodeId::new("failing");
        child.add_node(
            failing.clone(),
            NodeSpec::Function(FailingFunctionNode::new("boom")),
        );
        child.add_entry(failing);

        let mut parent = WarGraph::new(schema(vec![]), EngineLimits::default());
        let sub = NodeId::new("sub");
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_entry(sub.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-child-fails").unwrap();
        let outcome = run_with_children(
            &parent,
            thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        match outcome {
            RunOutcome::Failed { error, .. } => match error {
                EngineError::BattalionChildFailed {
                    node,
                    child_thread,
                    source,
                } => {
                    assert_eq!(node, sub);
                    assert_eq!(child_thread, child_thread_id(&thread, "sub"));
                    assert!(matches!(*source, EngineError::Node(_)));
                }
                other => panic!("expected BattalionChildFailed, got {other:?}"),
            },
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cancellation_is_observed_at_the_child_superstep_boundary() {
        let token = CancellationToken::new();

        let child_a = NodeId::new("child_a");
        let child_b = NodeId::new("child_b");
        let mut child = WarGraph::new(schema(vec![]), EngineLimits::default());
        {
            let token = token.clone();
            child.add_node(
                child_a.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                    // --- deterministically place cancellation mid-child-run:
                    // observed only at the CHILD's own next superstep
                    // boundary, before `child_b` ever runs.
                    token.cancel();
                    StateDelta::new()
                })),
            );
        }
        child.add_node(
            child_b.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        child.add_edge(EdgeSpec {
            from: child_a.clone(),
            to: child_b.clone(),
            condition: Some(EdgeCondition::Always),
        });
        child.add_entry(child_a);

        // The Battalion node has a static successor so the parent's run
        // does NOT short-circuit through "vanguard empty -> Completed"
        // before its own next top-of-loop cancellation check.
        let sub = NodeId::new("sub");
        let after = NodeId::new("after");
        let mut parent = WarGraph::new(schema(vec![]), EngineLimits::default());
        parent.add_node(
            sub.clone(),
            NodeSpec::battalion(Arc::new(child), StateMap::new()),
        );
        parent.add_node(
            after.clone(),
            NodeSpec::Function(CountingFunctionNode::new(|_run, _state| StateDelta::new())),
        );
        parent.add_edge(EdgeSpec {
            from: sub.clone(),
            to: after,
            condition: Some(EdgeCondition::Always),
        });
        parent.add_entry(sub.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let thread = ThreadId::new("battalion-cancel").unwrap();
        let outcome = run_with_children(
            &parent,
            thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &Some(token),
        )
        .await
        .unwrap();

        assert!(
            matches!(outcome, RunOutcome::Halted { .. }),
            "expected the parent to halt at its own boundary, got {outcome:?}"
        );

        let child_thread = child_thread_id(&thread, "sub");
        let child_waypoints = store.saved_waypoints(&child_thread).await;
        let last = child_waypoints
            .first()
            .expect("child must have persisted at least one waypoint");
        assert!(
            matches!(last.status, WaypointStatus::Halted),
            "expected the child's own latest waypoint to be Halted, got {:?}",
            last.status
        );
    }

    // --- Plan 23-09: child ThreadId identity, checkpoint_ns, resume-mid-child ---

    /// A two-node sequential child (`c1 -> c2`, `Always`) whose entry node
    /// (`c1`) contributes no delta and whose second node (`c2`) writes
    /// `child_out`. Each node's call count is tracked via the caller-owned
    /// `Arc<AtomicUsize>` counters, so a test can assert on exactly which
    /// nodes re-executed (or did not) across a resume without needing a
    /// `PaladinPort` execution log.
    fn build_two_node_child(
        child_out: FieldName,
        c1_calls: Arc<std::sync::atomic::AtomicUsize>,
        c2_calls: Arc<std::sync::atomic::AtomicUsize>,
    ) -> (WarGraph, NodeId, NodeId) {
        let child_schema = schema(vec![FieldSpec::new(
            child_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let c1 = NodeId::new("c1");
        let c2 = NodeId::new("c2");
        {
            let counter = Arc::clone(&c1_calls);
            child.add_node(
                c1.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    StateDelta::new()
                })),
            );
        }
        {
            let counter = Arc::clone(&c2_calls);
            let out = child_out.clone();
            child.add_node(
                c2.clone(),
                NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let mut delta = StateDelta::new();
                    delta.set_raw(out.clone(), serde_json::json!("child-done"));
                    delta
                })),
            );
        }
        child.add_edge(EdgeSpec {
            from: c1.clone(),
            to: c2.clone(),
            condition: Some(EdgeCondition::Always),
        });
        child.add_entry(c1.clone());
        (child, c1, c2)
    }

    #[tokio::test]
    async fn latest_on_the_child_thread_returns_the_childs_own_waypoint() {
        let child_out = field("child_out");
        let (child, _c1, _c2) = build_two_node_child(
            child_out.clone(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        );
        let sub = NodeId::new("sub");
        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let state_map = StateMap::new().with_output(child_out.clone(), parent_out.clone());
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let parent_thread = ThreadId::new("battalion-latest-isolation").unwrap();
        let outcome = run_with_children(
            &parent,
            parent_thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let child_thread = child_thread_id(&parent_thread, "sub");
        let parent_latest = store
            .latest(&parent_thread)
            .await
            .unwrap()
            .expect("parent has a latest waypoint");
        let child_latest = store
            .latest(&child_thread)
            .await
            .unwrap()
            .expect("child has a latest waypoint");

        assert_ne!(parent_latest.thread_id, child_latest.thread_id);
        assert_ne!(parent_latest.waypoint_id, child_latest.waypoint_id);
        assert!(matches!(parent_latest.status, WaypointStatus::Completed));
        assert!(matches!(child_latest.status, WaypointStatus::Completed));
    }

    #[tokio::test]
    async fn resume_of_a_parent_mid_child_resumes_the_child_where_it_stopped() {
        let child_out = field("child_out");

        // --- Step 1: produce a REAL first-superstep child Waypoint by
        // running the child graph directly (not embedded in a parent), then
        // keep only its superstep-1 Waypoint -- the same "seed from a real
        // run" technique `e2e_crash_resume_test.rs` uses, so the seeded
        // Waypoint's `frontier`/`vanguard` are exactly what production
        // would have produced, rather than hand-constructed and possibly
        // wrong.
        let seed_c1_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seed_c2_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (seed_child, _, _) =
            build_two_node_child(child_out.clone(), seed_c1_calls, seed_c2_calls);
        let seed_store = RecordingWaypointStore::new();
        let seed_thread = ThreadId::new("scratch-seed-child").unwrap();
        let _ = run_default(&seed_child, seed_thread.clone(), &seed_store).await;
        let seed_waypoints = seed_store.saved_waypoints(&seed_thread).await;
        let first_superstep_waypoint = seed_waypoints
            .iter()
            .find(|w| w.superstep == 1)
            .expect("child's first superstep waypoint must exist")
            .clone();
        assert!(
            matches!(first_superstep_waypoint.status, WaypointStatus::Running),
            "sanity: the child's first superstep must still be Running (c2 not yet reached)"
        );

        // --- Step 2: the REAL parent+child this test asserts against, with
        // FRESH call counters (the seed run's counters, above, are
        // discarded).
        let c1_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (child, _c1, _c2) = build_two_node_child(
            child_out.clone(),
            Arc::clone(&c1_calls),
            Arc::clone(&c2_calls),
        );

        let sub = NodeId::new("sub");
        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let state_map = StateMap::new().with_output(child_out.clone(), parent_out.clone());
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub.clone());

        let parent_thread = ThreadId::new("battalion-resume-mid-child").unwrap();
        let child_thread = child_thread_id(&parent_thread, "sub");

        // --- Step 3: seed a FRESH store (the one the resumed run actually
        // uses) with ONLY the child's real first-superstep Waypoint,
        // re-addressed under the PRODUCTION-derived child thread id. The
        // PARENT thread has NO seeded Waypoint at all -- simulating a crash
        // that landed right after the child's own first superstep
        // persisted but before the recursive Battalion dispatch (still
        // awaiting `c2`) ever returned to the parent's own superstep loop,
        // so the parent's own Waypoint for this superstep was never
        // written. A "resume" here is driven entirely by the Battalion
        // dispatch's own `latest(child_thread)` check, not by
        // `WarEngine::resume`.
        let mut seeded = first_superstep_waypoint;
        seeded.thread_id = child_thread.clone();
        let resumed_store = Arc::new(RecordingWaypointStore::new());
        resumed_store.save(&seeded).await.unwrap();

        let outcome = run_with_children(
            &parent,
            parent_thread.clone(),
            StateDelta::new(),
            &resumed_store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        assert_eq!(
            c1_calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "c1 must NOT re-execute: its completion is already recorded in the seeded child \
             Waypoint"
        );
        assert_eq!(
            c2_calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "c2 must execute exactly once to finish the resumed child"
        );

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&parent_out).unwrap(),
                    Some("child-done".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn restart_on_resume_true_runs_the_child_fresh() {
        let child_out = field("child_out");

        // Seed a "prior" child Waypoint exactly as
        // `resume_of_a_parent_mid_child_resumes_the_child_where_it_stopped`
        // does -- `restart_on_resume: true` must ignore it entirely.
        let seed_c1_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seed_c2_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (seed_child, _, _) =
            build_two_node_child(child_out.clone(), seed_c1_calls, seed_c2_calls);
        let seed_store = RecordingWaypointStore::new();
        let seed_thread = ThreadId::new("scratch-seed-child-restart").unwrap();
        let _ = run_default(&seed_child, seed_thread.clone(), &seed_store).await;
        let seed_waypoints = seed_store.saved_waypoints(&seed_thread).await;
        let first_superstep_waypoint = seed_waypoints
            .iter()
            .find(|w| w.superstep == 1)
            .expect("child's first superstep waypoint must exist")
            .clone();

        let c1_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (child, _c1, _c2) = build_two_node_child(
            child_out.clone(),
            Arc::clone(&c1_calls),
            Arc::clone(&c2_calls),
        );

        let sub = NodeId::new("sub");
        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let state_map = StateMap::new().with_output(child_out.clone(), parent_out.clone());
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        // `NodeSpec::Battalion` constructed directly (rather than through
        // `NodeSpec::battalion`, which always defaults `restart_on_resume`
        // to `false`) -- allowed from within this crate despite
        // `#[non_exhaustive]`, which restricts only OTHER crates.
        parent.add_node(
            sub.clone(),
            NodeSpec::Battalion {
                graph: Arc::new(child),
                state_map,
                restart_on_resume: true,
            },
        );
        parent.add_entry(sub.clone());

        let parent_thread = ThreadId::new("battalion-restart-on-resume").unwrap();
        let child_thread = child_thread_id(&parent_thread, "sub");

        let mut seeded = first_superstep_waypoint;
        seeded.thread_id = child_thread.clone();
        let resumed_store = Arc::new(RecordingWaypointStore::new());
        resumed_store.save(&seeded).await.unwrap();

        let outcome = run_with_children(
            &parent,
            parent_thread.clone(),
            StateDelta::new(),
            &resumed_store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        assert_eq!(
            c1_calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "restart_on_resume: true must run the child's entry node fresh, ignoring the \
             seeded prior Waypoint"
        );
        assert_eq!(c2_calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get::<String>(&parent_out).unwrap(),
                    Some("child-done".to_string())
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn checkpoint_ns_records_the_namespace_path() {
        // Innermost grandchild: one Function node.
        let grand_out = field("grand_out");
        let grand_schema = schema(vec![FieldSpec::new(
            grand_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut grandchild = WarGraph::new(grand_schema, EngineLimits::default());
        let g1 = NodeId::new("g1");
        grandchild.add_node(
            g1.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                grand_out.clone(),
                serde_json::json!("grand-done"),
            )),
        );
        grandchild.add_entry(g1);

        // Middle child: a single Battalion node wrapping the grandchild.
        let child_out = field("child_out");
        let child_schema = schema(vec![FieldSpec::new(
            child_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let inner_sub = NodeId::new("inner");
        let mut child = WarGraph::new(child_schema, EngineLimits::default());
        let inner_state_map = StateMap::new().with_output(grand_out.clone(), child_out.clone());
        child.add_node(
            inner_sub.clone(),
            NodeSpec::battalion(Arc::new(grandchild), inner_state_map),
        );
        child.add_entry(inner_sub.clone());

        // Outer parent: a single Battalion node wrapping the child.
        let outer_sub = NodeId::new("outer");
        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let outer_state_map = StateMap::new().with_output(child_out.clone(), parent_out.clone());
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        parent.add_node(
            outer_sub.clone(),
            NodeSpec::battalion(Arc::new(child), outer_state_map),
        );
        parent.add_entry(outer_sub.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let parent_thread = ThreadId::new("battalion-checkpoint-ns-nesting").unwrap();
        let outcome = run_with_children(
            &parent,
            parent_thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let child_thread = child_thread_id(&parent_thread, "outer");
        let grandchild_thread = child_thread_id(&child_thread, "inner");

        let child_latest = store
            .latest(&child_thread)
            .await
            .unwrap()
            .expect("child waypoint");
        let grandchild_latest = store
            .latest(&grandchild_thread)
            .await
            .unwrap()
            .expect("grandchild waypoint");

        assert_eq!(child_latest.checkpoint_ns, Some("outer/".to_string()));
        assert_eq!(
            grandchild_latest.checkpoint_ns,
            Some("outer/inner/".to_string())
        );

        // The parent's own Waypoints carry no namespace at all.
        let parent_latest = store
            .latest(&parent_thread)
            .await
            .unwrap()
            .expect("parent waypoint");
        assert_eq!(parent_latest.checkpoint_ns, None);
    }

    #[tokio::test]
    async fn child_threads_are_ordinary_threads_for_retention() {
        let child_out = field("child_out");
        let (child, _c1, _c2) = build_two_node_child(
            child_out.clone(),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        );
        let sub = NodeId::new("sub");
        let parent_out = field("parent_out");
        let parent_schema = schema(vec![FieldSpec::new(
            parent_out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let state_map = StateMap::new().with_output(child_out.clone(), parent_out.clone());
        let mut parent = WarGraph::new(parent_schema, EngineLimits::default());
        parent.add_node(sub.clone(), NodeSpec::battalion(Arc::new(child), state_map));
        parent.add_entry(sub.clone());

        let store = Arc::new(RecordingWaypointStore::new());
        let parent_thread = ThreadId::new("battalion-retention-ordinary").unwrap();
        run_with_children(
            &parent,
            parent_thread.clone(),
            StateDelta::new(),
            &store,
            &no_paladin_port(),
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
            &None,
        )
        .await
        .unwrap();

        let child_thread = child_thread_id(&parent_thread, "sub");

        // A child thread shows up in `list_threads` exactly like any other
        // thread -- the SAME `WaypointPort` API `WaypointRetentionService`
        // drives its pruning decisions through (D-20: no change to that
        // service, proven here by construction rather than by inspection).
        let threads = store.list_threads(None, None).await.unwrap();
        let ids: std::collections::HashSet<_> =
            threads.iter().map(|t| t.thread_id.clone()).collect();
        assert!(
            ids.contains(&parent_thread),
            "parent thread must be visible via list_threads"
        );
        assert!(
            ids.contains(&child_thread),
            "child thread must be visible via list_threads exactly like an ordinary thread"
        );

        // `history` on the child thread returns only the child's OWN
        // Waypoints -- the same per-thread scoping retention relies on for
        // every thread.
        let child_history = store.history(&child_thread, None, None).await.unwrap();
        assert!(!child_history.is_empty());
        for summary in &child_history {
            let wp = store
                .get(&child_thread, &summary.waypoint_id)
                .await
                .unwrap()
                .expect("summary's own waypoint must exist");
            assert_eq!(wp.thread_id, child_thread);
        }
    }

    // --- X-05 stress test: a 50-task muster under real multi-thread
    // contention (PRD 02 §4 item 8, `.project/v0.10.0/00-program-overview.md`
    // X-05). Every muster property up to this point is proven at 3-5 tasks
    // on the default (single-threaded, per-test) `#[tokio::test]` runtime --
    // this is the ONE place in the phase that exercises the muster dispatch
    // path under GENUINE OS-thread contention, following
    // `src/application/services/orchestration/listener.rs`'s house pattern
    // for exact-assertion, timeout-guarded, `multi_thread` concurrency
    // coverage: `#[tokio::test(flavor = "multi_thread")]`, an explicit
    // `tokio::time::timeout` around the run so a deadlock in the muster
    // dispatch/semaphore path fails loudly instead of hanging the suite, and
    // exact-count assertions rather than a lower bound (a dropped or
    // duplicated task must fail the test, not silently pass a `>=` check).
    //
    // 50 sits comfortably inside `EngineLimits::max_muster_tasks`'s default
    // of 100 (`engine::graph::EngineLimits`), so this exercises real
    // concurrency, never the limit-rejection path Plan 23-05 already owns.
    // Workers are lightweight `CountingFunctionNode`s, not mock-Paladin
    // round trips -- this test module is the phase's per-task sampling
    // command, so a slow test here would degrade the whole feedback loop.

    /// Builds a fresh 50-task muster fixture: `planner` (Function, entry,
    /// one-shot `Muster` of 50 tasks keyed `"000"`..`"049"`, already in
    /// lexicographic order so a passing "aggregated order == sorted key
    /// order" assertion cannot be satisfied by accident) `-> worker`
    /// (Function worker template, appends its own `task_key` into
    /// `results`) `-> aggregator` (Function, `defer: true`, asserted to run
    /// exactly once). Returns the graph plus both nodes so the caller can
    /// read `run_count()` after the run.
    fn fifty_task_muster_graph() -> (
        WarGraph,
        Arc<CountingFunctionNode>,
        Arc<CountingFunctionNode>,
    ) {
        const TASK_COUNT: usize = 50;
        let results_field = field("results");
        let s = schema(vec![FieldSpec::new(
            results_field.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let aggregator = NodeId::new("aggregator");

        let planner_node = {
            let worker = worker.clone();
            CountingFunctionNode::with_directive(move |_run, _state| Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(
                    (0..TASK_COUNT)
                        .map(|i| muster_task(&worker, serde_json::json!(i), &format!("{i:03}")))
                        .collect(),
                ),
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
        let aggregator_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());

        graph.add_node(planner.clone(), NodeSpec::Function(planner_node));
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node.clone()));
        graph.add_deferred_node(
            aggregator.clone(),
            NodeSpec::Function(aggregator_node.clone()),
        );
        graph.add_edge(EdgeSpec {
            from: worker,
            to: aggregator,
            condition: None,
        });
        graph.add_entry(planner);

        (graph, worker_node, aggregator_node)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fifty_task_muster_runs_to_completion_under_multi_thread() {
        const TASK_COUNT: usize = 50;
        let (graph, worker_node, aggregator_node) = fifty_task_muster_graph();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("fifty-task-muster").unwrap();

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            run_default(&graph, thread, &store),
        )
        .await
        .expect(
            "a 50-task muster must complete inside the timeout -- a deadlock in the muster \
             dispatch/semaphore path under real contention would hang here instead of failing",
        );

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                let results = final_state
                    .get::<Vec<String>>(&field("results"))
                    .unwrap()
                    .unwrap_or_default();
                assert_eq!(
                    results.len(),
                    TASK_COUNT,
                    "exactly 50 entries in the aggregated field, not a lower bound"
                );
                let expected: Vec<String> = (0..TASK_COUNT).map(|i| format!("{i:03}")).collect();
                assert_eq!(
                    results, expected,
                    "the aggregated order must equal the sorted task_key order, proven under \
                     real multi-thread contention"
                );
                let distinct: std::collections::HashSet<&String> = results.iter().collect();
                assert_eq!(
                    distinct.len(),
                    TASK_COUNT,
                    "all 50 task_keys must be distinct -- a duplicate would collapse this count"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        assert_eq!(
            worker_node.run_count(),
            TASK_COUNT,
            "exactly 50 worker executions, no more, no fewer"
        );
        assert_eq!(
            aggregator_node.run_count(),
            1,
            "the deferred aggregator must run exactly once"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fifty_task_muster_is_deterministic_across_repeats() {
        const REPEATS: usize = 3;
        let mut final_states = Vec::with_capacity(REPEATS);
        for i in 0..REPEATS {
            let (graph, worker_node, aggregator_node) = fifty_task_muster_graph();
            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("fifty-task-muster-repeat-{i}")).unwrap();

            let outcome = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                run_default(&graph, thread, &store),
            )
            .await
            .expect(
                "a 50-task muster must complete inside the timeout on every repeat -- a \
                 deadlock under real contention would hang here instead of failing",
            );

            match outcome {
                RunOutcome::Completed { final_state, .. } => {
                    assert_eq!(
                        worker_node.run_count(),
                        50,
                        "repeat {i}: exactly 50 worker executions"
                    );
                    assert_eq!(
                        aggregator_node.run_count(),
                        1,
                        "repeat {i}: the deferred aggregator must run exactly once"
                    );
                    final_states.push(final_state);
                }
                other => panic!("repeat {i}: expected Completed, got {other:?}"),
            }
        }

        for (i, state) in final_states.iter().enumerate().skip(1) {
            assert_eq!(
                state, &final_states[0],
                "repeat {i}: the final Battlefield must be byte-identical to repeat 0's -- a \
                 merge-order defect under real thread interleaving would surface as a mismatch \
                 here"
            );
        }
    }
}
