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
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::warn;
use regex::Regex;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use paladin_core::platform::container::aegis::{Aegis, CachePolicy, ErrorHandlerSpec};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    BATTLEFIELD_SCHEMA_VERSION, Battlefield, BattlefieldSchema, CacheMarker,
    CustomDispatchResolver, FieldName, StateDelta,
};
use paladin_core::platform::container::directive::{
    Directive, MusterContext, MusterTask, NextStep,
};
use paladin_core::platform::container::node_cache::{CachedDelta, NODE_CACHE_SCHEMA_VERSION};
use paladin_core::platform::container::node_error::{
    AttemptRecord, NodeError, NodeErrorSource, TimeoutKind,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::parley::{
    ParleyId, ParleyKind, ParleyRequest, ParleyResponse,
};
use paladin_core::platform::container::run_scope::RunScope;
use paladin_core::platform::container::structured::SchemaRef;
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::waypoint::{
    FrontierEdgeState, FrontierSnapshot, GraphFingerprint, MusterProgress, NodeExecutionRecord,
    NodeId, NodeOutcomeKind, ThreadId, Waypoint, WaypointId, WaypointStatus,
    canonical_edge_condition,
};
use paladin_ports::output::node_cache_port::{NodeCacheKey, NodeCachePort};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::structured_executor_port::{StructuredExecutorPort, StructuredOptions};
use paladin_ports::output::trace_sink_port::TraceEvent;
use paladin_ports::output::vault_confined::ConfinedVault;
use paladin_ports::output::waypoint_port::WaypointPort;

use crate::edge_evaluator::EdgeEvaluatorRegistry;
use crate::engine::cache_key;
use crate::engine::directive_parser::{DirectiveParseError, DirectiveParser};
use crate::engine::graph::{EngineLimits, GateRequestTemplate, NodeSpec, StateMap, WarGraph};
use crate::engine::heartbeat::HeartbeatHandle;
use crate::engine::hooks::{InterceptDecision, NodeInterceptor, TraceDispatcher};
use crate::engine::input_mapping::InputMapping;
use crate::engine::node::{NodeContext, StateNode, StateNodeError};
use crate::engine::registries::EngineRegistries;
use crate::engine::retry;
use crate::engine::{EngineError, RunOutcome, WaypointDurability};
use crate::llm_failure;

/// Every parent engine resource D-21 requires forwarding into a
/// `NodeSpec::Battalion` node's child run (CF-FR-16): the `WaypointPort`,
/// `WaypointDurability`, the parallelism setting, the dispatch resolver,
/// the whole `EngineRegistries` bundle (edge evaluators, retry predicates
/// and error handlers -- a child inherits the parent's registries
/// wholesale, Phase 23 D-21 / Phase 25 D-13), the trace sink, the
/// interceptor chain and the shared `CancellationToken`. `PaladinPort` is forwarded separately
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
    registries: EngineRegistries,
    trace: Arc<TraceDispatcher>,
    interceptors: Vec<Arc<dyn NodeInterceptor>>,
    cancellation: Option<CancellationToken>,
    /// THIS run's own `checkpoint_ns` (CF-FR-15, D-20) -- captured once
    /// here so a NESTED `NodeSpec::Battalion` dispatch (a grandchild, from
    /// this run's own perspective) can derive the next namespace segment
    /// as `"{this}{grandchild_node_id}/"` without threading an extra
    /// parameter through the whole dispatch/execute call chain.
    checkpoint_ns: Option<String>,
    /// THIS run's own `fork_of` (HITL-03, D-14) -- captured once here so a
    /// NESTED `NodeSpec::Battalion` dispatch propagates the SAME branch
    /// root onto its child run's own Waypoints, verbatim (never
    /// concatenated -- a branch root is a single value shared by the whole
    /// run tree, unlike `checkpoint_ns`'s per-level namespace segments).
    fork_of: Option<WaypointId>,
    /// THIS run's own `shutdown_grace` (HITL-04, D-19, D-20) -- captured
    /// once here so a nested `NodeSpec::Battalion` child run observes the
    /// SAME grace window its parent does when racing its own in-flight
    /// batch against a mid-superstep cancellation (a runtime setting shared
    /// by the whole run tree, exactly like `fork_of` above).
    shutdown_grace: std::time::Duration,
    /// THIS run's node cache backend (FT-FR-18, D-29; plan 25-13) --
    /// inherited by a nested `NodeSpec::Battalion` child run wholesale,
    /// like every other engine resource, so a child graph's own
    /// `CachePolicy` nodes are served by the same backend the parent's are.
    node_cache: Option<Arc<dyn NodeCachePort>>,
    /// THIS engine's confined Vault handle (RT-04, D-21; plan 26-13) --
    /// inherited by a nested `NodeSpec::Battalion` child run wholesale,
    /// like every other engine resource, so a child graph's own nodes
    /// receive the SAME grant the parent's do.
    vault: Option<ConfinedVault>,
    /// THIS engine's structured-output executor (RT-05, RT-FR-19, D-29;
    /// plan 26-18) -- inherited by a nested `NodeSpec::Battalion` child run
    /// wholesale, like every other engine resource, so a child graph's own
    /// `output_schema` nodes dispatch through the SAME executor the
    /// parent's do.
    structured_executor: Option<Arc<dyn StructuredExecutorPort>>,
}

/// One dispatched node's resolved cache binding (Doc 04 FT-FR-18, D-29;
/// plan 25-13): `Some` only when the node's resolved `Aegis` carries a
/// `cache` policy AND this run has a backend -- `WarGraph::
/// validate_node_cache_backend` already rejected the policy-without-backend
/// case before any node ran, so a `None` here always means "no policy".
#[derive(Clone)]
struct NodeCacheBinding {
    policy: CachePolicy,
    cache: Arc<dyn NodeCachePort>,
    graph_fingerprint: GraphFingerprint,
}

/// Compose this dispatch's cache key (D-28, `engine::cache_key`): a
/// `NodeSpec::Paladin` node keys on the SAME rendered input string
/// `execute_vanguard_node` will hand the port (rendered here a second time
/// over the same immutable snapshot -- cheap, and it keeps the key
/// composition a pure function of the dispatch rather than a side channel
/// out of the attempt); a `Function` node keys on the snapshot. `None` when
/// the input cannot be rendered (the attempt will then fail with the same
/// `InputMapping` error, so there is nothing to look up) or for a dispatch
/// kind that never carries a cache policy (`Battalion`, rejected at
/// validation).
fn compose_node_cache_key<W: WaypointPort + 'static>(
    binding: &NodeCacheBinding,
    dispatch: &NodeDispatch<W>,
    snapshot: &Battlefield,
    ctx: &NodeContext,
) -> Option<NodeCacheKey> {
    match dispatch {
        NodeDispatch::Function(_) => Some(cache_key::compose(&cache_key::CacheKeyInputs {
            graph_fingerprint: &binding.graph_fingerprint,
            node_id: &ctx.node_id,
            input: cache_key::InputComponent::Snapshot(snapshot),
            snapshot,
            key_spec: &binding.policy.key,
            muster: ctx.muster.as_ref(),
            paladin: None,
        })),
        NodeDispatch::Paladin {
            paladin,
            input_template,
            ..
        } => {
            let rendered = input_template
                .render(snapshot, ctx.muster.as_ref(), ctx.parley_response())
                .ok()?;
            Some(cache_key::compose(&cache_key::CacheKeyInputs {
                graph_fingerprint: &binding.graph_fingerprint,
                node_id: &ctx.node_id,
                input: cache_key::InputComponent::Rendered(&rendered),
                snapshot,
                key_spec: &binding.policy.key,
                muster: ctx.muster.as_ref(),
                paladin: Some(paladin.as_ref()),
            }))
        }
        NodeDispatch::Battalion { .. } => None,
    }
}

/// The lookup BEFORE attempt 1 (FT-FR-18, D-29). `Some(cached)` only for a
/// live hit: a backend `Err` is a MISS (logged, never a failure -- the
/// cache is best-effort by construction), an entry at or past its
/// `expires_at` is a miss even if the backend served it (the closed TTL
/// boundary, re-checked here so the engine and every backend agree on what
/// `expires_at` means), and an entry authored under a different
/// `CachedDelta`/`StateDelta` schema version is a miss rather than a delta
/// this build might mis-merge.
async fn lookup_node_cache(
    binding: &NodeCacheBinding,
    key: &NodeCacheKey,
    node_id: &NodeId,
) -> Option<CachedDelta> {
    match binding.cache.get(key).await {
        Ok(Some(cached)) => {
            if cached.is_expired_at(Utc::now()) {
                log::debug!("node cache: entry for {node_id} is expired -- miss");
                return None;
            }
            if cached.schema_version != NODE_CACHE_SCHEMA_VERSION
                || cached.delta.schema_version != BATTLEFIELD_SCHEMA_VERSION
            {
                log::debug!(
                    "node cache: entry for {node_id} carries schema versions {}/{} (this build: \
                     {NODE_CACHE_SCHEMA_VERSION}/{BATTLEFIELD_SCHEMA_VERSION}) -- miss",
                    cached.schema_version,
                    cached.delta.schema_version
                );
                return None;
            }
            Some(cached)
        }
        Ok(None) => None,
        Err(err) => {
            warn!("node cache: get failed for {node_id}: {err} -- treated as a miss (D-29)");
            None
        }
    }
}

/// The store AFTER a successful attempt (FT-FR-18, D-29): called only for a
/// genuine `NodeRunOutcome::Succeeded` whose `Directive` routes via
/// `NextStep::Edges` -- never for a failed attempt, a handler-compensated
/// failure, or a `Goto`/`End`/`Parley`/`Muster` directive (a `CachedDelta`
/// stores a delta alone, and replaying only the delta of a routing
/// directive would silently drop its routing). A delta touching a field the
/// schema marks other than `CacheMarker::Allow` is never stored (FT-FR-20:
/// the `Function`-node half of the `Deny` guarantee, since a `StateNode`'s
/// write set is only knowable here). A backend `Err` is logged and never
/// fails the run.
async fn store_node_cache(
    binding: &NodeCacheBinding,
    key: &NodeCacheKey,
    delta: &StateDelta,
    schema: &BattlefieldSchema,
    node_id: &NodeId,
) {
    let denied: Vec<&str> = delta
        .values
        .keys()
        .filter(|field| {
            schema
                .field_spec(field)
                .is_some_and(|spec| !matches!(spec.cache, CacheMarker::Allow))
        })
        .map(FieldName::as_str)
        .collect();
    if !denied.is_empty() {
        log::debug!(
            "node cache: not storing {node_id}'s delta -- it writes cache: Deny field(s) {}",
            denied.join(", ")
        );
        return;
    }
    if let Err(err) = binding.cache.put(key, delta, binding.policy.ttl).await {
        warn!("node cache: put failed for {node_id}: {err} -- ignored, the run continues (D-29)");
    }
}

/// Pairs a spawned node task's own dispatch-order position with its
/// [`tokio::task::JoinHandle`], so a batch of handles can be raced through a
/// [`FuturesUnordered`] (which does not preserve insertion order) while
/// still recovering each result's ORIGINAL `dispatch_entries` index for the
/// existing order-sensitive bookkeeping (`node_failure`'s first-wins guard,
/// `goto_targets` push order, etc. -- D-19, RESEARCH.md Pitfall 1).
/// [`IndexedHandle::abort`] delegates to the wrapped handle so the
/// grace-deadline race can cancel a still-outstanding task without first
/// removing it from the `FuturesUnordered` (`iter()` inspects without
/// polling or removing).
struct IndexedHandle<T> {
    index: usize,
    handle: tokio::task::JoinHandle<T>,
}

impl<T> IndexedHandle<T> {
    /// Abort the wrapped task (`JoinHandle::abort`, cancel-safe to call
    /// while the handle is also registered in a `FuturesUnordered`).
    fn abort(&self) {
        self.handle.abort();
    }
}

impl<T> std::future::Future for IndexedHandle<T> {
    type Output = (usize, Result<T, tokio::task::JoinError>);

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // `tokio::task::JoinHandle<T>` is `Unpin` unconditionally, so
        // `IndexedHandle<T>` (a `usize` plus a `JoinHandle<T>`, nothing
        // self-referential) is auto-`Unpin` too -- no `unsafe` needed to
        // reach `&mut self.handle` through the `Pin`.
        let this = self.get_mut();
        let index = this.index;
        std::pin::Pin::new(&mut this.handle)
            .poll(cx)
            .map(|res| (index, res))
    }
}

/// Resolves to the token's own cancellation the moment it fires, or never
/// resolves at all when `token` is `None` -- lets the mid-superstep grace
/// race (D-19) include "watch for cancellation" as an ordinary
/// `tokio::select!` branch regardless of whether a `CancellationToken` is
/// configured, with no `unwrap()`/`expect()` on the `Option` (house rule).
async fn cancelled_or_pending(token: &Option<CancellationToken>) {
    match token {
        Some(t) => t.cancelled().await,
        None => std::future::pending::<()>().await,
    }
}

/// One attempt's resolved timeout bounds (Doc 04 FT-FR-08/09/10, D-20;
/// plan 25-09): the tightest wall-clock deadline and which bound it belongs
/// to, plus the idle window. Resolved fresh per attempt by
/// [`AttemptBounds::resolve`] and raced against the attempt by
/// [`race_attempt`].
#[derive(Debug, Clone, Copy)]
struct AttemptBounds {
    /// The absolute wall-clock deadline for this attempt and the typed kind
    /// naming which bound it is: `Run` when the node's own
    /// `TimeoutPolicy::run_timeout` is the tightest, `EngineRun` when the
    /// remaining `EngineLimits::run_timeout` budget is. `None` when neither
    /// bound is declared.
    deadline: Option<(tokio::time::Instant, TimeoutKind)>,
    /// The node's `TimeoutPolicy::idle_timeout`, if declared: the attempt
    /// fails `Timeout(Idle)` if no heartbeat arrives within this window.
    idle: Option<std::time::Duration>,
}

impl AttemptBounds {
    /// Resolve the bounds for an attempt starting NOW from the node's
    /// resolved `TimeoutPolicy` (if any) and the run's absolute engine
    /// deadline (if any). The per-attempt deadline is
    /// `min(now + run_timeout, engine_deadline)`, named by whichever was
    /// tightest; on an exact tie the node's own `Run` bound is named (the
    /// policy the node author declared wins the label). Zero durations
    /// never reach here: `WarGraph::validate` rejects them (plan 25-03).
    fn resolve(
        policy: Option<&paladin_core::platform::container::aegis::TimeoutPolicy>,
        engine_deadline: Option<tokio::time::Instant>,
    ) -> Self {
        let now = tokio::time::Instant::now();
        let run = policy
            .and_then(|p| p.run_timeout)
            .map(|d| (now + d, TimeoutKind::Run));
        let engine = engine_deadline.map(|d| (d, TimeoutKind::EngineRun));
        let deadline = match (run, engine) {
            (Some(run), Some(engine)) if engine.0 < run.0 => Some(engine),
            (Some(run), _) => Some(run),
            (None, engine) => engine,
        };
        Self {
            deadline,
            idle: policy.and_then(|p| p.idle_timeout),
        }
    }
}

/// Resolves when `deadline` passes, or never when there is none -- an
/// always-present `tokio::select!` branch with no `unwrap()` on the
/// `Option` (house rule), mirroring [`cancelled_or_pending`].
async fn deadline_or_pending(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending::<()>().await,
    }
}

/// Resolves when no beat has been observed on `heartbeat` for `idle`, or
/// never when there is no idle window (a node without an `idle_timeout`
/// never subscribes -- D-18's "heartbeat is a no-op" truth). Each observed
/// beat restarts the window: the timer AWAITS the handle's `changed()`
/// rather than polling a timestamp, so under `tokio::time::pause` it is
/// driven purely by the virtual clock (RESEARCH.md's `watch` recommendation).
async fn idle_or_pending(heartbeat: &HeartbeatHandle, idle: Option<std::time::Duration>) {
    let Some(idle) = idle else {
        return std::future::pending::<()>().await;
    };
    let mut beats = heartbeat.subscribe();
    loop {
        match tokio::time::timeout(idle, beats.changed()).await {
            // A beat arrived inside the window: progress -- restart it.
            Ok(Ok(())) => continue,
            // The handle was dropped: the attempt itself is gone (finished
            // or cancelled), so there is nothing left to bound.
            Ok(Err(_)) => return std::future::pending::<()>().await,
            // No beat for a whole window: the node has stalled.
            Err(_elapsed) => return,
        }
    }
}

/// Race one attempt's execution against its [`AttemptBounds`] (D-20).
/// `biased` toward the attempt so a result landing on the same virtual
/// tick as a deadline is still a result. On expiry the attempt future is
/// dropped here -- its partial work is discarded exactly as any other
/// failed attempt's is (FT-FR-03, T-25-41) -- and the failure names the
/// bound that fired by typed `TimeoutKind`, never by message text (T-25-42).
async fn race_attempt(
    attempt: impl std::future::Future<Output = NodeDispatchResult>,
    bounds: &AttemptBounds,
    heartbeat: &HeartbeatHandle,
) -> NodeDispatchResult {
    let (deadline, deadline_kind) = match bounds.deadline {
        Some((at, kind)) => (Some(at), kind),
        None => (None, TimeoutKind::Run),
    };
    tokio::select! {
        biased;
        result = attempt => result,
        _ = deadline_or_pending(deadline) => {
            (None, 0, Err(NodeFailure::Timeout(deadline_kind)))
        }
        _ = idle_or_pending(heartbeat, bounds.idle) => {
            (None, 0, Err(NodeFailure::Timeout(TimeoutKind::Idle)))
        }
    }
}

/// The ONE path every engine-limit failure takes (ENG-FR-03, D-20):
/// `RecursionLimitExceeded`, `NodeVisitLimitExceeded` and, from plan 25-09,
/// `RunTimeoutExceeded` all persist a `WaypointStatus::Failed` Waypoint
/// through here and return `RunOutcome::Failed` -- consistency is
/// structural (one function), not three implementations kept in step by
/// hand. `completed` is empty for a boundary-time limit (nothing ran this
/// superstep) and carries the superstep's records for a mid-superstep cut;
/// `node_error` is `None` for a boundary-time limit (exactly what the two
/// pre-existing limits always wrote) and `Some` when the engine budget cut
/// an in-flight attempt, so the typed `Timeout(EngineRun)` survives on the
/// Waypoint.
#[allow(clippy::too_many_arguments)]
async fn persist_limit_failure<W: WaypointPort + 'static>(
    waypoint_port: &W,
    durability: WaypointDurability,
    trace: &Arc<TraceDispatcher>,
    thread: &ThreadId,
    parent_waypoint_id: Option<WaypointId>,
    superstep_number: u64,
    graph: &WarGraph,
    battlefield: &Battlefield,
    vanguard: Vec<NodeId>,
    completed: Vec<NodeExecutionRecord>,
    error: EngineError,
    failed_node: NodeId,
    node_error: Option<NodeError>,
    visit_counts: BTreeMap<NodeId, u32>,
    frontier: FrontierSnapshot,
    checkpoint_ns: Option<String>,
    fork_of: Option<WaypointId>,
) -> Result<RunOutcome, EngineError> {
    let waypoint = build_waypoint(
        thread,
        parent_waypoint_id,
        superstep_number,
        graph,
        battlefield,
        vanguard,
        completed,
        WaypointStatus::Failed {
            error: error.to_string(),
            failed_node,
            node_error,
        },
        visit_counts,
        frontier,
        None,
        checkpoint_ns,
        fork_of,
    );
    persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
    Ok(RunOutcome::Failed {
        error,
        waypoint: Some(waypoint.waypoint_id),
    })
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
        /// This node's `output_schema`, ALREADY RESOLVED to its JSON Schema
        /// value (RT-05, RT-FR-19, D-29, plan 26-18) -- `SchemaRef::Inline`
        /// unwrapped, `SchemaRef::Registered(name)` looked up in
        /// `registries.output_schemas` and rendered via
        /// `StructuredSchema::to_json_schema` -- both resolved ONCE, before
        /// this dispatch entry is spawned (never per-attempt), by the
        /// dispatch-building loop below. `None` for an ordinary node,
        /// unchanged from before this phase.
        output_schema: Option<serde_json::Value>,
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

// Manual `Clone` (rather than `#[derive(Clone)]`) so this impl does NOT pick
// up a spurious `W: Clone` bound -- every field that mentions `W` is already
// behind an `Arc` (`ChildEngineResources<W>`), which is `Clone` regardless
// of whether `W` itself is. D-14's retry loop needs to re-dispatch the SAME
// `NodeDispatch` on every attempt, so this is now load-bearing rather than
// merely convenient.
impl<W: WaypointPort + 'static> Clone for NodeDispatch<W> {
    fn clone(&self) -> Self {
        match self {
            NodeDispatch::Function(node) => NodeDispatch::Function(Arc::clone(node)),
            NodeDispatch::Paladin {
                paladin,
                input_template,
                output_field,
                directive_parser,
                output_schema,
            } => NodeDispatch::Paladin {
                paladin: paladin.clone(),
                input_template: input_template.clone(),
                output_field: output_field.clone(),
                directive_parser: directive_parser.clone(),
                output_schema: output_schema.clone(),
            },
            NodeDispatch::Battalion {
                graph,
                state_map,
                resources,
                restart_on_resume,
            } => NodeDispatch::Battalion {
                graph: Arc::clone(graph),
                state_map: state_map.clone(),
                resources: Arc::clone(resources),
                restart_on_resume: *restart_on_resume,
            },
        }
    }
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
    /// internal engine error -- everything that was `StateNodeError` before this
    /// phase, unchanged.
    Node(StateNodeError),
    /// A `NodeSpec::Paladin` node's `PaladinPort::execute` call failed
    /// (Doc 04 D-07): the live `PaladinError` is kept until the engine
    /// boundary converts it, so its typed `transience()`, `status` and
    /// `provider` reach the structured `NodeError` instead of being erased
    /// to a string one line early. Retry-eligible exactly like `Node`.
    Paladin(PaladinError),
    /// A `NodeSpec::Paladin` node's `DirectiveParser::StructuredDirective`
    /// call under `OnParseError::FailRun` (CF-02, D-11).
    DirectiveParse(DirectiveParseError),
    /// A `NodeSpec::Battalion` node's child run failed (CF-FR-16, D-21) --
    /// already the fully-formed, structured `EngineError::BattalionChildFailed`
    /// (X-06: naming the failing child node and thread, never a bare
    /// interpolated string), built where the child's own thread id is in
    /// scope and passed through here unchanged.
    Battalion(EngineError),
    /// The attempt was cut by a timeout (Doc 04 FT-FR-08/09/10, D-20; plan
    /// 25-09): the per-attempt wall-clock `TimeoutPolicy::run_timeout`
    /// (`Run`), the progress-aware `TimeoutPolicy::idle_timeout` (`Idle`),
    /// or the run-level `EngineLimits::run_timeout` (`EngineRun`). The
    /// attempt's future was DROPPED at expiry, so its partial work never
    /// existed as a `Directive` to merge (T-25-41). Always `Transient`, so
    /// `Run`/`Idle` feed the retry predicate like any other transient
    /// failure; `EngineRun` is never retried (the budget is gone) and ends
    /// the whole run with `EngineError::RunTimeoutExceeded`.
    Timeout(TimeoutKind),
    /// A `NodeSpec::Paladin` node's `output_schema` structured-output
    /// repair loop exhausted (D-29, RT-FR-19, Phase 25 D-05): ALWAYS
    /// classified `Transience::Unknown` in `node_error` below -- NEVER
    /// delegated to `PaladinError::transience()`'s general `Permanent`
    /// verdict for `PaladinError::StructuredOutputInvalid` (the verdict a
    /// non-engine caller of `execute_structured` correctly gets, since it
    /// has no different-graph/different-model retry available). The engine
    /// repair loop already retried internally (`max_repair_attempts`)
    /// before this failure surfaced, so it is not a transient network
    /// condition -- but it is not provably permanent either, and a
    /// `TransientAndUnknown` Aegis may legitimately retry the WHOLE node
    /// with a different prompt/model. Deliberately a DISTINCT variant from
    /// `Paladin(PaladinError)` above rather than a special case inside it,
    /// so this divergence from the general classification is visible at
    /// the type level, not buried inside a conditional.
    StructuredOutputInvalid(PaladinError),
}

impl NodeFailure {
    /// The structured [`NodeError`] this failure converts to at the engine
    /// boundary (Doc 04 D-07), for `node_id`'s 1-indexed `attempt`:
    ///
    /// - `Node(StateNodeError)` -> `NodeErrorSource::Function { message }`
    ///   classified `Transience::Unknown` -- a `StateNode` author returns
    ///   only a message, so nothing typed exists to classify from (the
    ///   retry predicate's `TransientAndUnknown` is how a Function node
    ///   opts into retrying these; `TransientOnly`, the default, does not).
    /// - `Paladin(PaladinError)` -> `NodeErrorSource::Paladin { kind, .. }`,
    ///   or `NodeErrorSource::Llm { status, provider, .. }` for a
    ///   `PaladinError::LlmFailure`, classified by the error's own typed
    ///   `PaladinError::transience()` (D-05) -- via
    ///   `llm_failure::to_node_error_source`, the one conversion beside
    ///   `to_paladin_error` so both read the same typed fields.
    /// - `Timeout(kind)` -> `NodeErrorSource::Timeout(kind)` classified
    ///   `Transience::Transient` (D-20, FT-FR-08): the bound that fired is
    ///   carried by the typed `TimeoutKind`, never inferred from a message.
    /// - `DirectiveParse`/`Battalion` -> `None`: neither is a node-execution
    ///   failure the Aegis governs (each has its own typed `EngineError`
    ///   and is never retried, D-14).
    fn node_error(&self, node_id: &NodeId, attempt: u32) -> Option<NodeError> {
        let (transience, source) = match self {
            NodeFailure::Node(err) => (Transience::Unknown, NodeErrorSource::from(err.clone())),
            NodeFailure::Paladin(err) => (err.transience(), llm_failure::to_node_error_source(err)),
            NodeFailure::Timeout(kind) => (Transience::Transient, NodeErrorSource::Timeout(*kind)),
            // --- D-29, Phase 25 D-05: hardcoded `Unknown`, never
            // `err.transience()` -- see this variant's own rustdoc.
            NodeFailure::StructuredOutputInvalid(err) => {
                (Transience::Unknown, llm_failure::to_node_error_source(err))
            }
            NodeFailure::DirectiveParse(_) | NodeFailure::Battalion(_) => return None,
        };
        Some(NodeError {
            node_id: node_id.clone(),
            attempt,
            transience,
            source,
        })
    }
}

/// Resolve a node's FINAL failure through its `Aegis.on_error` handler
/// (Doc 04 FT-FR-11/12/13, D-21, D-13; plan 25-10), returning the
/// compensating [`Directive`] the dispatch loop honours as if the node had
/// returned it -- or the error the run fails with instead:
///
/// - `Route { to, error_field }`: the structured `NodeError` is serialized
///   to a `serde_json::Value` and written into `error_field` as an ORDINARY
///   delta write (so the schema's declared dispatch applies -- validation
///   already guaranteed the field is declared and not `Sum`), routed via
///   `NextStep::Goto([to])` so the existing Goto machinery places `to` in
///   the next Vanguard and resolves the failed node's static successors
///   `NotFiring` (the routed target REPLACES them, FT-FR-11).
/// - `Absorb { fallback_delta }`: the fallback delta (possibly empty --
///   merges nothing) routed via `NextStep::Edges`, so the node's static
///   edges fire exactly as they would on success (FT-FR-12).
/// - `Custom(name)`: the handler registered under `name` in
///   `registries.error_handlers` (D-13, FT-FR-13) is `await`ed with
///   `(err, state)` and its `Directive` returned verbatim -- `Edges`,
///   `Goto`, `End`, `Parley` and `Muster` are all honoured by the caller
///   exactly as a node's own `NextStep` is. In particular a handler may ask
///   a human (D-23, plan 25-11): `Parley` takes the ONE existing HITL-01
///   suspension path a node-raised parley takes -- one `AwaitingInput`
///   Waypoint, `RunOutcome::AwaitingInput` -- and the post-resume re-run
///   is a fresh attempt 1 with `ctx.parley_response()` set (Phase 24
///   D-07/D-08), having spent no retry budget (a parley is not an attempt
///   failure). Inside a Muster task only `Edges` is honoured (D-22): the
///   caller rejects anything else as `MusterHandlerMustBeDeltaOnly`, so a
///   handler-raised parley never suspends from inside a fan-out.
///   `WarGraph::validate` already
///   rejected an unregistered name before any node ran (plan 25-03), so a
///   miss here is unreachable in practice; library code must still not
///   panic on an invariant it cannot enforce, so it re-fails with the
///   original error -- never a silent fallthrough to `Absorb`/`Edges`.
///
/// Every arm reads `state` as the same immutable pre-superstep snapshot
/// the node's attempts read (T-25-50). `serde_json::to_value` on a
/// `NodeError` cannot fail for a well-formed value (every field is a plain
/// serde value, D-07); should it ever, the run fails with the ORIGINAL
/// error rather than a fabricated one, so nothing is silently dropped.
async fn dispatch_error_handler(
    spec: &ErrorHandlerSpec,
    err: &NodeError,
    state: &Battlefield,
    registries: &EngineRegistries,
) -> Result<Directive, NodeError> {
    match spec {
        ErrorHandlerSpec::Route { to, error_field } => {
            let serialized = serde_json::to_value(err).map_err(|_| err.clone())?;
            let mut delta = StateDelta::new();
            delta.set_raw(error_field.clone(), serialized);
            Ok(Directive {
                delta,
                next: NextStep::Goto(vec![to.clone()]),
            })
        }
        ErrorHandlerSpec::Absorb { fallback_delta } => Ok(Directive {
            delta: fallback_delta.clone(),
            next: NextStep::Edges,
        }),
        ErrorHandlerSpec::Custom(name) => match registries.error_handlers.get(name) {
            Some(handler) => handler.handle(err, state).await,
            None => Err(err.clone()),
        },
        // `ErrorHandlerSpec` is `#[non_exhaustive]`: a variant added later
        // is deliberately a re-fail with the original error, never an
        // `Absorb`-shaped fallthrough that would silently swallow it.
        _ => Err(err.clone()),
    }
}

/// The bare arm name of a [`NextStep`], for
/// [`EngineError::MusterHandlerMustBeDeltaOnly`]'s `returned` field (D-22).
fn next_step_arm_name(next: &NextStep) -> &'static str {
    match next {
        NextStep::Edges => "Edges",
        NextStep::Goto(_) => "Goto",
        NextStep::Muster(_) => "Muster",
        NextStep::End => "End",
        NextStep::Parley(_) => "Parley",
    }
}

/// [`execute_vanguard_node`]'s per-node result: `paladin_id`/`token_count`
/// (`None`/`0` for a `Function` or `Battalion` node) plus the resolved
/// `Directive` or [`NodeFailure`].
type NodeDispatchResult = (Option<Uuid>, u64, Result<Directive, NodeFailure>);

/// Executes a [`NodeSpec::Gate`] node's no-`run`-body-of-its-own contract
/// (HITL-01, D-05), dispatched exactly like a `Function` node
/// ([`NodeDispatch::Function`]) so it reuses every existing
/// dispatch/interceptor/trace code path with no changes to any of them.
///
/// On the node's first visit (`ctx.parley_response()` is `None`) renders
/// `request.prompt_template`/`payload_template` from `state` and returns
/// `NextStep::Parley`, entering the suspension path plan 24-01 landed. On
/// the post-resume visit (`ctx.parley_response()` is `Some`) writes the
/// normalised delivered value to `output_field` -- or, for
/// `ParleyKind::StateEdit`, returns the response's `StateDelta` as this
/// node's own delta -- and routes via `NextStep::Edges` like any other
/// node (D-06).
struct GateDispatchNode {
    request: GateRequestTemplate,
    output_field: Option<FieldName>,
}

#[async_trait::async_trait]
impl StateNode for GateDispatchNode {
    async fn run(
        &self,
        state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        match ctx.parley_response() {
            // --- First visit: render and raise. Never merges anything
            // beyond an empty delta -- a Gate's own contribution to the
            // Battlefield happens only on the post-resume visit below.
            None => {
                // --- HITL-01, D-07: this is the RAISING visit -- no parley
                // context is in scope yet (that only exists on the
                // post-resume visit below), so both templates render with
                // `parley: None`, exactly like every other first-visit
                // render call site (Task 1's own call-site audit,
                // 24-03-SUMMARY.md).
                let prompt = self
                    .request
                    .prompt_template
                    .render(state, ctx.muster.as_ref(), None)
                    .map_err(|e| StateNodeError(format!("gate {}: {e}", ctx.node_id)))?;
                let payload = match &self.request.payload_template {
                    Some(template) => {
                        let rendered = template
                            .render(state, ctx.muster.as_ref(), None)
                            .map_err(|e| StateNodeError(format!("gate {}: {e}", ctx.node_id)))?;
                        // A payload template commonly renders a JSON shape
                        // (e.g. `{"amount": {amount}}`) through ordinary
                        // `InputMapping` field substitution -- parsed back
                        // into structured JSON when it is valid JSON, and
                        // carried as a plain JSON string otherwise (never
                        // an error: a payload is author-supplied context,
                        // not a validated contract, HITL-FR-03).
                        serde_json::from_str(&rendered)
                            .unwrap_or(serde_json::Value::String(rendered))
                    }
                    None => serde_json::json!({}),
                };
                let created_at = Utc::now();
                let expires_at = self.request.expires_in.map(|d| {
                    created_at
                        + chrono::Duration::from_std(d).unwrap_or_else(|_| chrono::Duration::zero())
                });
                let request = ParleyRequest {
                    parley_id: ParleyId::new(),
                    // Stamped onto the request regardless -- the engine's
                    // suspension arm (plan 24-01) re-stamps it from the
                    // dispatching `node_id` anyway, but setting it
                    // correctly here keeps this type's own invariant
                    // honest independent of that belt-and-braces rewrite.
                    node_id: ctx.node_id.clone(),
                    kind: self.request.kind.clone(),
                    prompt,
                    payload,
                    choices: self.request.choices.clone(),
                    expires_at,
                    created_at,
                    on_expire: self.request.on_expire.clone(),
                };
                Ok(Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Parley(request),
                })
            }
            // --- Post-resume visit: deliver and route via static edges.
            Some(response) => {
                let mut delta = StateDelta::new();
                match &self.request.kind {
                    ParleyKind::StateEdit => {
                        let state_delta: StateDelta =
                            serde_json::from_value(response.value.clone()).map_err(|e| {
                                StateNodeError(format!(
                                    "gate {}: StateEdit response value is not a valid \
                                     StateDelta: {e}",
                                    ctx.node_id
                                ))
                            })?;
                        return Ok(state_delta.into());
                    }
                    ParleyKind::Approval => {
                        let approved =
                            crate::engine::graph::normalize_approval_value(&response.value)
                                .ok_or_else(|| {
                                    StateNodeError(format!(
                                        "gate {}: response value {} is not a valid Approval value",
                                        ctx.node_id, response.value
                                    ))
                                })?;
                        if let Some(field) = &self.output_field {
                            // D-06: a Bool output_field receives the JSON
                            // boolean; a String output_field receives the
                            // strings "true"/"false" -- inferred from the
                            // field's schema default, the same signal
                            // `WarGraph::validate` used to accept this
                            // wiring in the first place.
                            let is_bool_field = state
                                .schema()
                                .field_spec(field)
                                .and_then(|spec| spec.default.as_ref())
                                .is_some_and(serde_json::Value::is_boolean);
                            let value = if is_bool_field {
                                serde_json::json!(approved)
                            } else {
                                serde_json::json!(if approved { "true" } else { "false" })
                            };
                            delta.set_raw(field.clone(), value);
                        }
                    }
                    ParleyKind::Choice | ParleyKind::FreeText => {
                        if let Some(field) = &self.output_field {
                            delta.set_raw(field.clone(), response.value.clone());
                        }
                    }
                    // `ParleyKind` is `#[non_exhaustive]`: a future kind
                    // reaching here (never raised by this phase's own
                    // `GateRequestTemplate`) writes no output field rather
                    // than panicking.
                    _ => {}
                }
                Ok(delta.into())
            }
        }
    }
}

/// Execute one vanguard node's dispatch against `snapshot`.
///
/// Returns `(paladin_id, token_count, result)`: `paladin_id`/`token_count`
/// are `None`/`0` for a `Function` node (it never carries either), and are
/// populated from the executed `Paladin` and its `PaladinResult` for a
/// `NodeSpec::Paladin` node. An `InputMapping::render` failure (an
/// undeclared field, or a declared field with no value and no default) and a
/// `PaladinPort::execute` error both become a `StateNodeError` here, so a Paladin
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
    // --- RT-05, RT-FR-19, D-29 (plan 26-18): this run's structured-output
    // executor, if any -- consulted ONLY when this dispatch's
    // `output_schema` is `Some` (an ordinary node ignores this entirely,
    // exactly as before this phase).
    structured_executor: &'a Option<Arc<dyn StructuredExecutorPort>>,
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
                output_schema,
            } => {
                let paladin_id = Some(paladin.uuid);
                // --- CF-03, D-15: the executing task's Muster context (`Some`
                // only for a worker-template dispatch), so `{muster.payload}`/
                // `{muster.task_key}` resolve from it, never from the
                // Battlefield. --- HITL-01, D-07: `ctx.parley_response()` is
                // `Some` ONLY on the post-resume re-run of a parleying
                // Paladin node, so `{parley.value}`/`{parley.prompt}`/
                // `{parley.kind}`/`{parley.responded_by}` resolve from it on
                // that re-run (Task 1's own call-site audit,
                // 24-03-SUMMARY.md: this is the ONE call site that threads a
                // real parley response, since it is the same dispatch path a
                // Paladin node's first-raising visit AND its post-resume
                // re-run both go through).
                let rendered = match input_template.render(
                    snapshot,
                    ctx.muster.as_ref(),
                    ctx.parley_response(),
                ) {
                    Ok(rendered) => rendered,
                    Err(e) => {
                        return (
                            paladin_id,
                            0,
                            Err(NodeFailure::Node(StateNodeError(e.to_string()))),
                        );
                    }
                };
                // --- FT-FR-09, D-19: ALWAYS `execute_scoped`, never
                // `execute`/`execute_observed` directly, passing this
                // attempt's own handle -- a port that overrides the
                // defaulted `execute_observed` beats it on every LLM
                // completion / stream chunk / Armament call, and each beat
                // resets this node's `idle_timeout` timer. The trait's
                // default `execute_observed` delegates to `execute` and
                // beats nothing, so a non-observing port's `idle_timeout`
                // degrades to a per-attempt wall clock rather than to no
                // bound at all.
                //
                // --- RT-05, RT-FR-19, D-29: when `output_schema` is `Some`,
                // dispatch through the engine's structured executor instead
                // of the plain `PaladinPort` path below -- the PARSED JSON
                // value, never a string, is written to `output_field`.
                // `WarGraph::validate` (`validate_output_schemas`,
                // `directive_parser` must be `PlainOutput`) and
                // `WarGraph::validate_structured_executor_backend` have
                // already guaranteed, before any node ran, that
                // `structured_executor` is `Some` whenever `output_schema`
                // is `Some` -- this `let Some(..) else` unwraps that
                // invariant defensively rather than by `.expect()` (library
                // code must not panic on an invariant it cannot enforce),
                // failing just this node closed should it somehow not hold.
                if let Some(schema_json) = output_schema {
                    let Some(executor) = structured_executor else {
                        return (
                            paladin_id,
                            0,
                            Err(NodeFailure::Node(StateNodeError(format!(
                                "node has output_schema but no structured executor is wired -- \
                                 WarGraph::validate_structured_executor_backend should have \
                                 rejected this before any node ran (node output_field: {})",
                                output_field.as_str()
                            )))),
                        );
                    };
                    return match executor
                        .execute_json_schema_observed(
                            &paladin,
                            &rendered,
                            &schema_json,
                            &StructuredOptions::default(),
                            &ctx.heartbeat,
                        )
                        .await
                    {
                        Ok(structured) => {
                            let token_count = u64::from(structured.raw.token_count);
                            let mut delta = StateDelta::new();
                            delta.set_raw(output_field, structured.value);
                            (paladin_id, token_count, Ok(delta.into()))
                        }
                        // --- D-29, Phase 25 D-05: exhaustion is ALWAYS
                        // `Transience::Unknown` here -- never delegated to
                        // `PaladinError::transience()`'s general `Permanent`
                        // verdict for `StructuredOutputInvalid` (that
                        // verdict is correct for a non-engine caller of
                        // `execute_structured`, who has no different-graph
                        // retry available) -- see `NodeFailure::
                        // StructuredOutputInvalid`'s own rustdoc for why.
                        Err(err @ PaladinError::StructuredOutputInvalid { .. }) => (
                            paladin_id,
                            0,
                            Err(NodeFailure::StructuredOutputInvalid(err)),
                        ),
                        // Any OTHER underlying failure (e.g. an LLM call
                        // failure inside the repair loop) keeps its own
                        // natural classification via the ordinary
                        // `NodeFailure::Paladin` path.
                        Err(other) => (paladin_id, 0, Err(NodeFailure::Paladin(other))),
                    };
                }

                // --- RT-04, D-21: the `RunScope` carries this run's own
                // Vault grant (`ctx.vault`'s granted namespace, if any) so
                // a port that overrides `execute_scoped` can act on it. The
                // trait's default `execute_scoped` delegates to
                // `execute_observed` and ignores the scope entirely, so a
                // non-scoped port's behavior is completely unchanged by
                // this call switching from `execute_observed`.
                let scope = ctx
                    .vault
                    .as_ref()
                    .map(|confined| {
                        RunScope::default().with_vault_namespace(confined.granted().clone())
                    })
                    .unwrap_or_default();
                match paladin_port
                    .execute_scoped(&paladin, &rendered, &ctx.heartbeat, &scope)
                    .await
                {
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
                    Err(e) => (paladin_id, 0, Err(NodeFailure::Paladin(e))),
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

                // --- CF-FR-15, D-20, HITL-03, D-18: the child's thread id
                // is a derived, PROVABLY INJECTIVE encoding of (this run's
                // thread, this node's id) -- `ThreadId::child`,
                // length-prefixed exactly like `graph.rs`'s `push_field`
                // (22.1 CR-01's lesson: a bare delimiter join of `NodeId`s,
                // which accept any non-empty string, is collidable by
                // construction). When this run is itself executing on a
                // branch (`resources.fork_of` is `Some(branch_root)`), the
                // child instead derives under `ThreadId::child_on_branch`,
                // extended with the SAME branch root -- so a fork's
                // subgraph child can never resolve to the mainline child's
                // thread, and `latest(child_thread)` on the branch never
                // sees the mainline child's history (D-18, HITL-FR-12).
                // Mainline runs (`resources.fork_of: None`) keep deriving
                // via `ThreadId::child`, byte-for-byte as before this plan.
                // A fork's subgraph child therefore always starts fresh
                // (D-18's `restart_on_resume` resolution): the derived id
                // has no prior history for `existing_latest` (below) to
                // find, which follows by construction from this distinct
                // id rather than a separate flag. Fails typed (never
                // silently truncates) if the derived id would itself
                // exceed `ThreadId`'s own limits.
                let child_thread = match &resources.fork_of {
                    Some(branch_root) => {
                        ThreadId::child_on_branch(&ctx.thread_id, branch_root, &ctx.node_id)
                    }
                    None => ThreadId::child(&ctx.thread_id, &ctx.node_id),
                };
                let child_thread = match child_thread {
                    Ok(id) => id,
                    Err(e) => {
                        return (
                            None,
                            0,
                            Err(NodeFailure::Node(StateNodeError(format!(
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
                                Err(NodeFailure::Node(StateNodeError(format!(
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
                                        Err(NodeFailure::Node(StateNodeError(format!(
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
                                Err(NodeFailure::Node(StateNodeError(format!(
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
                    &resources.registries,
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
                    // --- HITL-03, D-14: the child run inherits the SAME
                    // branch root as the parent, verbatim -- a branch is a
                    // property of the whole run tree, not re-derived or
                    // concatenated per nesting level like `checkpoint_ns`.
                    resources.fork_of,
                    // --- D-04: a child run never inherits the parent's
                    // resume-time parley responses -- those are keyed to
                    // the PARENT thread's own vanguard, and a suspended
                    // child is unsupported this phase (see the
                    // `RunOutcome::AwaitingInput` arm below).
                    None,
                    // --- HITL-04, D-19, D-20: the child run races its OWN
                    // mid-superstep batch against the SAME grace window the
                    // parent was configured with (`ChildEngineResources`'s
                    // own rustdoc) -- a runtime setting shared by the whole
                    // run tree, exactly like `fork_of` above.
                    resources.shutdown_grace,
                    // --- FT-FR-09, D-19: a child Battalion beats the
                    // PARENT node's own attempt handle once per child
                    // superstep, so a parent `idle_timeout` over a
                    // Battalion node measures child-superstep progress.
                    Some(ctx.heartbeat.clone()),
                    // --- FT-FR-18, D-29: the child run serves its own
                    // `CachePolicy` nodes from the SAME backend.
                    resources.node_cache.clone(),
                    // --- RT-04, D-21: the child run's nodes receive the
                    // SAME Vault grant the parent's do.
                    resources.vault.clone(),
                    // --- RT-05, RT-FR-19, D-29: the child run's own
                    // `output_schema` nodes dispatch through the SAME
                    // structured executor the parent's do.
                    resources.structured_executor.clone(),
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
                        Err(NodeFailure::Battalion(
                            EngineError::ParleyInChildUnsupported {
                                node: ctx.node_id.clone(),
                                child_thread,
                            },
                        )),
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
/// StateNodeError>` `execute_vanguard_node` alone would produce: a `Skip`
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
    /// The run was cancelled while this node was waiting out a retry
    /// backoff (D-15, FT-FR-07): its last attempt failed, no further
    /// attempt ran, and nothing of it merges. Recorded exactly like a
    /// grace-deadline abort -- `Skipped { reason: "shutdown" }` on the
    /// record AND re-listed on the Halted Waypoint's vanguard (or, for a
    /// Muster task, left unfinished in the preserved `MusterProgress`) --
    /// so `resume` re-executes the node from attempt 1 rather than
    /// silently dropping it as an ordinary skip would.
    Interrupted,
}

/// What one spawned node-dispatch task (the `tokio::spawn`'d async block in
/// the dispatch loop below) resolves to: the node's own `NodeId` (so the
/// grace-race join phase, which re-indexes by `dispatch_entries` position
/// rather than relying on completion order, can still cross-check it),
/// when its FINAL (succeeding or exhausted) attempt started, how long that
/// attempt ran, its Paladin identity if any, its token count, its
/// [`NodeRunOutcome`], the 1-indexed attempt number that produced it
/// (D-15: populates `NodeExecutionRecord.attempt`, `1` for a node with no
/// `Aegis` retry policy, exactly as before this phase), and the history of
/// every failed attempt before it (populates `NodeExecutionRecord.attempts`).
struct NodeTaskOutput {
    node_id: NodeId,
    started_at: chrono::DateTime<chrono::Utc>,
    duration_ms: u64,
    paladin_id: Option<Uuid>,
    token_count: u64,
    outcome: NodeRunOutcome,
    attempt: u32,
    /// Every FAILED attempt before the final one, ascending by attempt
    /// number (FT-FR-03, D-16) -- built in order by the retry loop, so it
    /// is asserted (never sorted) at the record construction site.
    failed_attempts: Vec<AttemptRecord>,
    /// The structured error of the FINAL attempt when it failed and the
    /// node has a resolved Aegis (D-08) -- what the exhausted-failure path
    /// surfaces as `EngineError::NodeFailed` and records on the `Failed`
    /// Waypoint. `None` for a success, a skip, a no-Aegis node's failure
    /// (the byte-identical pre-Phase-25 path, D-09), and a
    /// `DirectiveParse`/`Battalion` failure.
    node_error: Option<NodeError>,
    /// Whether `outcome` was served from the node cache (FT-FR-18, D-29)
    /// rather than by executing the node -- `true` only for a
    /// `Succeeded` outcome on `attempt: 1` with no `failed_attempts`.
    cache_hit: bool,
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
/// available fails closed with a `StateNodeError` naming the node, rather than
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
    registries: &EngineRegistries,
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
    // --- HITL-04, D-19, D-20: unlike `checkpoint_ns`/`fork_of`/
    // `initial_parley_responses` below (each fixed to a top-level-only
    // `None` and never a real value this wrapper's own callers supply),
    // `shutdown_grace` IS a real, always-present value every top-level
    // caller configures via `WarEngine::with_shutdown_grace` -- so, unlike
    // those three, it is a genuine new parameter on this wrapper's own
    // signature (forwarded verbatim to [`run_with_namespace`]), not one
    // this plan can fix to a constant.
    shutdown_grace: std::time::Duration,
    // --- FT-FR-18, D-29 (plan 25-13): the engine's node cache backend,
    // like `shutdown_grace` a real, always-present engine setting every
    // top-level caller forwards (`None` when no backend is wired --
    // `WarGraph::validate_node_cache_backend` has then already rejected
    // any `CachePolicy` in the graph).
    node_cache: Option<Arc<dyn NodeCachePort>>,
    // --- RT-04, D-21 (plan 26-13): the engine's confined Vault handle, if
    // any -- like `node_cache`, a real, always-present engine setting every
    // top-level caller forwards (`None` when `WarEngine::with_vault` was
    // never called).
    vault: Option<ConfinedVault>,
    // --- RT-05, RT-FR-19, D-29 (plan 26-18): the engine's structured-output
    // executor, if any -- like `node_cache`/`vault`, a real, always-present
    // engine setting every top-level caller forwards (`None` when
    // `WarEngine::with_structured_executor` was never called --
    // `WarGraph::validate_structured_executor_backend` has then already
    // rejected any `output_schema` in the graph).
    structured_executor: Option<Arc<dyn StructuredExecutorPort>>,
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
        registries,
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
        // --- HITL-03, D-14: a top-level `start`/`resume_with_options` call
        // is never itself entered from a branch -- only a later plan's
        // `WarEngine::fork` first produces a `Some` value, and it calls
        // `run_with_namespace` directly (bypassing this wrapper), exactly
        // like `checkpoint_ns` above.
        None,
        // --- HITL-01, D-08: a top-level `start`/`resume_with_options` call
        // never carries a resume-superstep's parley responses -- only
        // `WarEngine::resume_with` does, and it calls `run_with_namespace`
        // directly (bypassing this wrapper) so this function's SIGNATURE
        // stays unchanged by this plan, exactly like `checkpoint_ns` above.
        None,
        shutdown_grace,
        // --- FT-FR-09, D-19: a top-level run has no parent node whose
        // idle timer it could feed -- only a `NodeSpec::Battalion` child
        // dispatch ever passes `Some` here.
        None,
        node_cache,
        vault,
        structured_executor,
    )
    .await
}

/// [`run`]'s real implementation (CF-FR-15, D-20): identical to [`run`] in
/// every respect except the trailing `checkpoint_ns` and
/// `initial_parley_responses` parameters, which [`run`] always passes as
/// `None` and [`execute_vanguard_node`]'s `NodeSpec::Battalion` dispatch arm
/// passes as `Some(namespace)`/`None` respectively when recursing into a
/// child run. Kept as a SEPARATE function (rather than adding the
/// parameters to [`run`] directly) so `run`'s own public signature --
/// called from `engine::mod` and from `engine::graph`'s own tests -- stays
/// unchanged by this plan. `pub(crate)` (rather than private) so
/// `WarEngine::resume_with` (`engine::mod`, HITL-02) can call it directly --
/// the ONE caller that ever has a real `Some` `initial_parley_responses` to
/// pass.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_with_namespace<W: WaypointPort + 'static>(
    waypoint_port: &W,
    durability: WaypointDurability,
    parallelism: Option<usize>,
    registry: &CustomDispatchResolver,
    registries: &EngineRegistries,
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
    // --- HITL-03, D-14: the branch root THIS run's own Waypoints are
    // stamped with (`Waypoint.fork_of`) -- `None` for every mainline run
    // (`WarEngine::start`/`resume_with_options`/`resume_with`, until a
    // later plan's `WarEngine::fork` first produces a `Some` value), and
    // `Some(root)` for a run entered from a branch. Stamped, verbatim
    // (never re-derived or concatenated), onto every Waypoint
    // [`build_waypoint`] produces in this call; a nested `NodeSpec::Battalion`
    // dispatch propagates the SAME value onto its child run via
    // `ChildEngineResources::fork_of`.
    fork_of: Option<WaypointId>,
    // --- HITL-01, D-08: `Some(responses_by_node)` ONLY on a
    // `WarEngine::resume_with` re-entry, keyed by the PARLEYING node's own
    // `NodeId` (never a `parley_id`, since `NodeContext.parley_response`
    // must be looked up by the executing node, not the request that raised
    // it). Consumed EXACTLY ONCE -- on the very first iteration of the loop
    // below, via `.take()` -- so a resumed run's second superstep (and
    // every ordinary `start`/`resume_with_options` call, which always
    // passes `None`) never re-delivers a stale response to a node that
    // re-enters the vanguard later in the same run (e.g. a Goto/cycle
    // revisit).
    initial_parley_responses: Option<BTreeMap<NodeId, ParleyResponse>>,
    // --- HITL-04, D-19, D-20: the grace window this run's OWN mid-superstep
    // cancellation race (below) shares across the whole in-flight batch,
    // computed once as `cancel_observed_at + shutdown_grace` -- never a
    // per-handle timeout (RESEARCH.md Pitfall 1). A runtime setting, never
    // hashed into the graph fingerprint and never part of `EngineLimits`
    // (D-20); every real `WarEngine::start`/`resume`/`resume_with`/
    // `replay`/`fork` call forwards its own configured
    // `WarEngine::with_shutdown_grace` value here, and a nested
    // `NodeSpec::Battalion` child run inherits the SAME value via
    // `ChildEngineResources::shutdown_grace`.
    shutdown_grace: std::time::Duration,
    // --- FT-FR-09, D-19: `Some(handle)` ONLY when this call is a
    // `NodeSpec::Battalion` child run -- the PARENT node's own attempt
    // handle, beaten once at the top of every child superstep below so a
    // parent `idle_timeout` over a Battalion node observes child progress.
    // `None` for every top-level `start`/`resume`/`fork` call.
    parent_heartbeat: Option<HeartbeatHandle>,
    // --- FT-FR-18, D-29 (plan 25-13): this run's node cache backend, if
    // any; a nested `NodeSpec::Battalion` child run inherits the SAME
    // backend via `ChildEngineResources::node_cache`.
    node_cache: Option<Arc<dyn NodeCachePort>>,
    // --- RT-04, D-21 (plan 26-13): this engine's confined Vault handle, if
    // any -- granted to every `NodeContext` this run builds, and inherited
    // wholesale by a nested `NodeSpec::Battalion` child run via
    // `ChildEngineResources::vault`, like every other engine resource.
    vault: Option<ConfinedVault>,
    // --- RT-05, RT-FR-19, D-29 (plan 26-18): this engine's structured-output
    // executor, if any -- consulted by a `NodeSpec::Paladin` node whose
    // `output_schema` is `Some`, and inherited wholesale by a nested
    // `NodeSpec::Battalion` child run via
    // `ChildEngineResources::structured_executor`, like every other engine
    // resource.
    structured_executor: Option<Arc<dyn StructuredExecutorPort>>,
) -> Result<RunOutcome, EngineError> {
    // --- FT-FR-20, D-28: the graph fingerprint every cache key composed in
    // this run starts with -- computed ONCE per run (never per dispatch),
    // and only when a backend is wired at all.
    let cache_graph_fingerprint: Option<GraphFingerprint> =
        node_cache.as_ref().map(|_| graph.fingerprint());
    // --- FT-FR-10, D-20, ENG-FR-03: the run-level budget. Measured from
    // the moment THIS call starts (a resumed run's budget restarts with the
    // resume; a Battalion child run measures its OWN budget against its
    // OWN `EngineLimits`, per `child_uses_its_own_engine_limits`). The
    // absolute deadline is threaded into every attempt so the per-attempt
    // bound can be `min(attempt run_timeout, remaining budget)` and the
    // top-of-loop check below can end a run whose budget is already gone.
    // `tokio::time::Instant` (not `std`), so the paused clock drives it.
    let run_started_at = tokio::time::Instant::now();
    let engine_deadline: Option<tokio::time::Instant> = graph
        .limits()
        .run_timeout
        .map(|limit| run_started_at + limit);

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
                registries: registries.clone(),
                trace: Arc::clone(trace),
                interceptors: interceptors.to_vec(),
                cancellation: cancellation.clone(),
                checkpoint_ns: checkpoint_ns.clone(),
                fork_of,
                shutdown_grace,
                node_cache: node_cache.clone(),
                vault: vault.clone(),
                structured_executor: structured_executor.clone(),
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

    // --- HITL-01, D-08: `Some` ONLY on a `WarEngine::resume_with`
    // re-entry's very first iteration below (`.take()`n there so it can
    // never re-apply to a later superstep of the same run).
    let mut initial_parley_responses = initial_parley_responses;

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
                    node_error: None,
                },
                visit_counts,
                entry_frontier.snapshot(graph),
                None,
                checkpoint_ns.clone(),
                fork_of,
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
            fork_of,
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
        // --- FT-FR-09, D-19: a child Battalion run reports progress to its
        // parent node once per child superstep -- here, at the boundary,
        // before any of this superstep's work begins.
        if let Some(handle) = &parent_heartbeat {
            handle.beat();
        }

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
                fork_of,
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Halted {
                waypoint: waypoint.waypoint_id,
            });
        }

        // --- ENG-FR-03: bounded iteration, checked at the top of the loop
        // so a run stops at exactly `max_supersteps` rather than one over.
        // --- CR-01 (23-REVIEW.md): `vanguard` alone may be empty on a
        // muster-only round -- this phase's Muster feature can re-enter
        // this loop with `vanguard` empty and `pending_muster` carrying the
        // next dispatch (`has_pending_muster` a few hundred lines below
        // stands in for "there is more work" in exactly this situation).
        // Mirrors the `Battlefield::merge` failure fallback's
        // `dispatch_entries.first()` pattern a few hundred lines below,
        // using `pending_muster` instead since `dispatch_entries` is not
        // built yet at this point in the loop. The loop's own
        // Completed-return checks guarantee at least one of `vanguard`/
        // `pending_muster` is non-empty whenever a boundary-time limit
        // fires, so the final placeholder is unreachable by construction --
        // but must not panic if that invariant is ever violated (mirrors
        // `MusterProgress::default`'s own placeholder
        // `NodeId::new(String::new())`). Shared by every boundary-time
        // limit below (recursion, run timeout).
        let boundary_failed_node = || {
            vanguard
                .first()
                .cloned()
                .or_else(|| pending_muster.as_ref().map(|(node, _)| node.clone()))
                .unwrap_or_else(|| NodeId::new(String::new()))
        };
        if superstep_number >= graph.limits().max_supersteps {
            let error = EngineError::RecursionLimitExceeded {
                limit: graph.limits().max_supersteps,
                thread_id: thread.clone(),
            };
            let failed_node = boundary_failed_node();
            return persist_limit_failure(
                waypoint_port,
                durability,
                trace,
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                Vec::new(),
                error,
                failed_node,
                None,
                visit_counts,
                frontier.snapshot(graph),
                checkpoint_ns.clone(),
                fork_of,
            )
            .await;
        }

        // --- FT-FR-10, D-20, ENG-FR-03: the run-level wall-clock budget,
        // checked at the SAME boundary as the recursion limit so a run
        // whose budget expired during (or exactly at the end of) the
        // previous superstep stops here rather than starting another one.
        // A budget that expires MID-superstep is caught by the per-attempt
        // race instead (`AttemptBounds::resolve` names it `EngineRun`) and
        // surfaced through the same helper after the bookkeeping loop.
        if let Some(limit) = graph.limits().run_timeout {
            let elapsed = run_started_at.elapsed();
            if elapsed >= limit {
                let error = EngineError::RunTimeoutExceeded { elapsed, limit };
                let failed_node = boundary_failed_node();
                return persist_limit_failure(
                    waypoint_port,
                    durability,
                    trace,
                    &thread,
                    parent_waypoint_id,
                    superstep_number,
                    graph,
                    &battlefield,
                    vanguard.clone(),
                    Vec::new(),
                    error,
                    failed_node,
                    None,
                    visit_counts,
                    frontier.snapshot(graph),
                    checkpoint_ns.clone(),
                    fork_of,
                )
                .await;
            }
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
            return persist_limit_failure(
                waypoint_port,
                durability,
                trace,
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                Vec::new(),
                error,
                node,
                None,
                visit_counts,
                frontier.snapshot(graph),
                checkpoint_ns.clone(),
                fork_of,
            )
            .await;
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
        // --- HITL-01, D-08: taken exactly once, on the round that
        // dispatches a `WarEngine::resume_with` re-entry's forced vanguard
        // (every ordinary superstep, and every later round of the SAME
        // resumed run, sees `None` here).
        let parley_responses_this_round: BTreeMap<NodeId, ParleyResponse> =
            initial_parley_responses.take().unwrap_or_default();
        // --- WR-02 (23-REVIEW.md): reuses `MusterProgress::unfinished_tasks`
        // -- the method the module's own rustdoc says resume relies on --
        // instead of hand-rolling an equivalent `task_key` filter a second
        // time, so the two can never silently diverge. `node` is a required
        // `MusterProgress` field but is never read by `unfinished_tasks`
        // itself; when no Muster is pending this round `muster_tasks` is
        // empty and `unfinished_tasks()` trivially returns an empty `Vec`
        // regardless of `node`, so the same documented placeholder
        // `MusterProgress::default` uses (`NodeId::new(String::new())`) is
        // reused here rather than inventing a second one.
        let dispatch_tasks: Vec<MusterTask> = MusterProgress {
            node: muster_node
                .clone()
                .unwrap_or_else(|| NodeId::new(String::new())),
            tasks: muster_tasks.clone(),
            completed: muster_carryover_this_round.clone(),
        }
        .unfinished_tasks();
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
        for (dispatch_index, (node_id, muster_ctx)) in dispatch_entries.iter().enumerate() {
            let spec = graph.node(node_id).ok_or_else(|| {
                EngineError::Node(StateNodeError(format!(
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
                    output_schema,
                } => {
                    // --- RT-05, RT-FR-19, D-29: resolve `output_schema`
                    // ONCE here, before this dispatch entry is spawned --
                    // `SchemaRef::Inline` unwraps directly; `SchemaRef::
                    // Registered(name)` looks the name up in
                    // `registries.output_schemas`, already proven present
                    // by `WarGraph::validate` (`validate_output_schemas`)
                    // before any node ran, so this lookup is infallible in
                    // practice -- but library code must not `.expect()` an
                    // invariant it cannot enforce (mirroring the Battalion
                    // arm's own `resources` lookup below), so a defensive
                    // miss still fails the run with a typed error rather
                    // than panicking.
                    let resolved_schema = match output_schema {
                        None => None,
                        Some(SchemaRef::Inline(value)) => Some(value.clone()),
                        Some(SchemaRef::Registered(name)) => {
                            let schema = registries.output_schemas.get(name).ok_or_else(|| {
                                EngineError::Node(StateNodeError(format!(
                                    "node {node_id}: output_schema registered name \
                                         '{name}' not found in registries at dispatch time -- \
                                         WarGraph::validate should have rejected this before \
                                         any node ran"
                                )))
                            })?;
                            Some(schema.to_json_schema())
                        }
                        // `SchemaRef` is `#[non_exhaustive]` from this
                        // crate's point of view -- a future variant fails
                        // closed here (a typed error naming the node)
                        // rather than silently falling through to "no
                        // schema".
                        Some(_) => {
                            return Err(EngineError::Node(StateNodeError(format!(
                                "node {node_id}: output_schema uses a SchemaRef variant this \
                                 engine does not yet resolve"
                            ))));
                        }
                    };
                    NodeDispatch::Paladin {
                        paladin: paladin.clone(),
                        input_template: input_template.clone(),
                        output_field: output_field.clone(),
                        directive_parser: directive_parser.clone(),
                        output_schema: resolved_schema,
                    }
                }
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
                        EngineError::Node(StateNodeError(format!(
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
                // --- HITL-01, D-05: a Gate has no `run` body of its own --
                // dispatched as an ordinary `Function` node wrapping a
                // fresh `GateDispatchNode`, reusing every existing
                // dispatch/interceptor/trace/NextStep::Parley-suspension
                // code path below with no changes to any of it.
                NodeSpec::Gate {
                    request,
                    output_field,
                } => NodeDispatch::Function(Arc::new(GateDispatchNode {
                    request: request.clone(),
                    output_field: output_field.clone(),
                })),
            };
            let snap = Arc::clone(&snapshot);
            let sem = Arc::clone(&semaphore);
            let port = Arc::clone(paladin_port);
            // --- RT-05, RT-FR-19, D-29: this run's structured-output
            // executor, cloned out of the outer scope so the spawned task
            // owns everything it touches, mirroring `port` immediately
            // above.
            let node_structured_executor = structured_executor.clone();
            let node_trace = Arc::clone(trace);
            let node_interceptors = interceptors.to_vec();
            // --- D-18: the attempt-INVARIANT part of this dispatch's
            // context. `attempt` and `heartbeat` are per attempt (the retry
            // loop below rebuilds `ctx` from this base on every iteration
            // with the current attempt number and a FRESH handle), so the
            // values placed here are placeholders overwritten before any
            // node, interceptor or trace ever sees the context.
            let base_ctx = crate::engine::node::NodeContext {
                node_id: node_id.clone(),
                thread_id: thread.clone(),
                superstep: superstep_number,
                muster: muster_ctx.clone(),
                parley_response: parley_responses_this_round.get(node_id).cloned(),
                attempt: 0,
                heartbeat: HeartbeatHandle::new(),
                // --- RT-04, D-21: the SAME grant for every node of this
                // run -- `None` when the engine has no Vault store wired.
                vault: vault.clone(),
            };
            let nid = node_id.clone();
            // --- D-09, D-10, D-14: this node's resolved Aegis (its own
            // `set_aegis` entry, else the graph's `default_aegis`, else
            // `None`), and this run's cancellation token, both cloned out
            // of the graph/outer scope so the spawned task owns everything
            // it touches, mirroring every other per-dispatch clone above.
            let node_aegis: Option<Aegis> = graph.aegis_for(node_id).cloned();
            let node_cancellation = cancellation.clone();
            // --- FT-FR-18, D-29: this node's cache binding -- `Some` only
            // with BOTH a resolved `cache` policy and a wired backend.
            let node_cache_binding: Option<NodeCacheBinding> = match (
                node_aegis.as_ref().and_then(|a| a.cache.as_ref()),
                node_cache.as_ref(),
                cache_graph_fingerprint.as_ref(),
            ) {
                (Some(policy), Some(cache), Some(fingerprint)) => Some(NodeCacheBinding {
                    policy: policy.clone(),
                    cache: Arc::clone(cache),
                    graph_fingerprint: fingerprint.clone(),
                }),
                _ => None,
            };
            handles.push(IndexedHandle {
                index: dispatch_index,
                handle: tokio::spawn(async move {
                    // --- FT-FR-18, D-29, D-14: the cache lookup happens
                    // BEFORE attempt 1 and OUTSIDE the interceptor chain
                    // (the cache is part of the Aegis, which wraps the whole
                    // per-attempt sequence, so a hit runs no `before`/
                    // `after` interceptor -- nothing executes). A hit merges
                    // the stored delta as a `Succeeded` outcome on attempt 1
                    // with `cache_hit: true`, emits exactly one
                    // `NodeStarted`/`NodeFinished { cache_hit: true }` pair,
                    // consumes no retry budget and calls no port. A `get`
                    // error is a miss (`lookup_node_cache`), never a failure.
                    let cache_key: Option<NodeCacheKey> =
                        node_cache_binding.as_ref().and_then(|binding| {
                            compose_node_cache_key(binding, &dispatch, &snap, &base_ctx)
                        });
                    if let (Some(binding), Some(key)) = (&node_cache_binding, &cache_key)
                        && let Some(cached) = lookup_node_cache(binding, key, &nid).await
                    {
                        let started_at = Utc::now();
                        node_trace.emit(TraceEvent::NodeStarted {
                            thread_id: base_ctx.thread_id.clone(),
                            superstep: base_ctx.superstep,
                            node_id: nid.clone(),
                            attempt: 1,
                        });
                        let paladin_id = match &dispatch {
                            NodeDispatch::Paladin { paladin, .. } => Some(paladin.uuid),
                            _ => None,
                        };
                        let duration_ms =
                            (Utc::now() - started_at).num_milliseconds().max(0) as u64;
                        node_trace.emit(TraceEvent::NodeFinished {
                            thread_id: base_ctx.thread_id.clone(),
                            superstep: base_ctx.superstep,
                            node_id: nid.clone(),
                            attempt: 1,
                            cache_hit: true,
                        });
                        return NodeTaskOutput {
                            node_id: nid,
                            started_at,
                            duration_ms,
                            paladin_id,
                            token_count: 0,
                            outcome: NodeRunOutcome::Succeeded(Directive {
                                delta: cached.delta,
                                next: NextStep::Edges,
                            }),
                            attempt: 1,
                            failed_attempts: Vec::new(),
                            node_error: None,
                            cache_hit: true,
                        };
                    }
                    // --- D-14: the retry loop wraps the ENTIRE per-attempt
                    // sequence below -- the `NodeStarted` emit, the whole
                    // `before` interceptor chain, `execute_vanguard_node`,
                    // the whole `after` interceptor chain, and the
                    // `NodeFinished` emit -- so every attempt runs through
                    // interceptors afresh (`hooks.rs`'s own contract), never
                    // just the dispatch call in isolation. A node with no
                    // resolved Aegis (or no `retry` policy on it) runs
                    // exactly once, byte-identical to pre-Phase-25 behavior
                    // (D-09's "no policy, no change" truth).
                    let mut attempt: u32 = 0;
                    // --- FT-FR-03, D-16: one `AttemptRecord` per FAILED
                    // attempt, pushed in attempt order as each retry is
                    // decided, so the vector is ascending by construction.
                    let mut failed_attempts: Vec<AttemptRecord> = Vec::new();
                    loop {
                        attempt += 1;
                        // --- D-18: rebuilt per attempt with the CURRENT
                        // attempt number and a FRESH `HeartbeatHandle`, so a
                        // straggling task from a cancelled (timed-out)
                        // previous attempt can never reset THIS attempt's
                        // idle timer, and a `StateNode` never observes a
                        // stale `ctx.attempt`.
                        let ctx = crate::engine::node::NodeContext {
                            attempt,
                            heartbeat: HeartbeatHandle::new(),
                            ..base_ctx.clone()
                        };
                        // --- FT-FR-08/09/10, D-20: THIS attempt's bounds,
                        // armed fresh per attempt from the node's resolved
                        // `TimeoutPolicy` and the remaining engine budget.
                        // A policy with both fields `None` (or no policy at
                        // all) and no engine bound arms nothing, so the
                        // attempt runs exactly as before this plan.
                        let attempt_bounds = AttemptBounds::resolve(
                            node_aegis.as_ref().and_then(|a| a.timeout.as_ref()),
                            engine_deadline,
                        );
                        node_trace.emit(TraceEvent::NodeStarted {
                            thread_id: ctx.thread_id.clone(),
                            superstep: ctx.superstep,
                            node_id: nid.clone(),
                            attempt,
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
                            InterceptDecision::Proceed => match Arc::clone(&sem)
                                .acquire_owned()
                                .await
                            {
                                Ok(_permit) => {
                                    // --- FT-FR-08/09/10, D-20: race the
                                    // attempt against its resolved bounds.
                                    // On expiry the attempt future is
                                    // DROPPED (its partial work never becomes
                                    // a Directive, T-25-41) and the failure
                                    // names the bound by typed `TimeoutKind`.
                                    let (paladin_id, token_count, result) = race_attempt(
                                        execute_vanguard_node(
                                            dispatch.clone(),
                                            &snap,
                                            &ctx,
                                            &port,
                                            &node_structured_executor,
                                        ),
                                        &attempt_bounds,
                                        &ctx.heartbeat,
                                    )
                                    .await;
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
                                            // --- FT-FR-18, D-29: `put` ONLY here --
                                            // after a SUCCESSFUL attempt, once the
                                            // `after` chain has produced the delta
                                            // that will actually merge, and only
                                            // for an `Edges`-routed directive
                                            // (`store_node_cache`'s own contract).
                                            // No failed attempt, handler outcome or
                                            // error ever reaches this call.
                                            if let (Some(binding), Some(key)) =
                                                (&node_cache_binding, &cache_key)
                                                && matches!(directive.next, NextStep::Edges)
                                            {
                                                store_node_cache(
                                                    binding,
                                                    key,
                                                    &directive.delta,
                                                    snap.schema(),
                                                    &nid,
                                                )
                                                .await;
                                            }
                                            (
                                                paladin_id,
                                                token_count,
                                                NodeRunOutcome::Succeeded(directive),
                                            )
                                        }
                                        Err(e) => {
                                            (paladin_id, token_count, NodeRunOutcome::Failed(e))
                                        }
                                    }
                                }
                                // Semaphore is never `.close()`d anywhere in this
                                // engine today, so this arm is unreachable in
                                // practice -- but library code must not `.expect()`
                                // an invariant it cannot enforce (WR-01, Phase
                                // 22.1). Report it the same way a node's own
                                // execution error is reported, through the existing
                                // NodeRunOutcome/StateNodeError plumbing, rather than
                                // panicking inside a detached `tokio::spawn`ed task.
                                Err(_) => (
                                    None,
                                    0u64,
                                    NodeRunOutcome::Failed(NodeFailure::Node(StateNodeError(
                                        "internal error: superstep semaphore closed unexpectedly"
                                            .to_string(),
                                    ))),
                                ),
                            },
                        };
                        let duration_ms =
                            (Utc::now() - started_at).num_milliseconds().max(0) as u64;
                        node_trace.emit(TraceEvent::NodeFinished {
                            thread_id: ctx.thread_id.clone(),
                            superstep: ctx.superstep,
                            node_id: nid.clone(),
                            attempt,
                            // Plan 25-13 is the only plan that sets this
                            // `true` (a served-from-cache outcome).
                            cache_hit: false,
                        });

                        // --- D-14, D-15, D-07: only a `NodeFailure::Node`
                        // (a `Function` node's own error, an `InputMapping`
                        // failure, or an `InterceptDecision::Fail` decision
                        // -- everything carrying a `StateNodeError`) or a
                        // `NodeFailure::Paladin` (a `PaladinPort::execute`
                        // error, classified by its own typed transience) is
                        // retry-eligible -- exactly the failures
                        // `NodeFailure::node_error` converts. A
                        // `DirectiveParse`/`Battalion` failure, a `Skipped`
                        // outcome, a `Succeeded` outcome (including a
                        // `NextStep::Parley` Directive, which is a success
                        // that leaves this loop at once and consumes no
                        // retry budget, D-17) never enters this block.
                        //
                        // FT-FR-07: NO Waypoint is written anywhere inside
                        // this loop -- a retry is never a durable checkpoint
                        // boundary, so a run interrupted between attempts
                        // resumes by re-executing this node from attempt 1.
                        //
                        // D-20: an attempt cut by the ENGINE's run budget
                        // (`Timeout(EngineRun)`) is never retried -- the
                        // budget is exhausted, so a fresh attempt would be
                        // cut at once; the run ends `RunTimeoutExceeded`
                        // instead (bookkeeping loop below). `Run`/`Idle`
                        // cuts ARE retried like any other transient failure.
                        if let NodeRunOutcome::Failed(ref failure) = outcome
                            && !matches!(failure, NodeFailure::Timeout(TimeoutKind::EngineRun))
                            && let Some(retry_policy) =
                                node_aegis.as_ref().and_then(|a| a.retry.as_ref())
                            && attempt < retry_policy.max_attempts
                            && let Some(node_error) = failure.node_error(&nid, attempt)
                            && retry::should_retry(retry_policy, &node_error, attempt)
                        {
                            failed_attempts.push(AttemptRecord {
                                attempt,
                                started_at,
                                duration_ms,
                                error: node_error,
                            });
                            let delay = retry::backoff_delay(retry_policy, attempt + 1);
                            if retry::wait_backoff(delay, &node_cancellation).await {
                                continue;
                            }
                            // --- D-15, RESEARCH.md Pitfall 7: the run is
                            // shutting down mid-backoff -- stop retrying and
                            // report this dispatch entry `Interrupted`, which
                            // the bookkeeping loop records `Skipped { reason:
                            // "shutdown" }` and re-lists for resume, exactly
                            // like the grace-race abort path below (FT-FR-07:
                            // a resume re-executes it from attempt 1).
                            break NodeTaskOutput {
                                node_id: nid,
                                started_at,
                                duration_ms,
                                paladin_id,
                                token_count,
                                outcome: NodeRunOutcome::Interrupted,
                                attempt,
                                failed_attempts,
                                node_error: None,
                                cache_hit: false,
                            };
                        }
                        // --- D-08: the structured error travels with a
                        // FINAL failed attempt only when this node has a
                        // resolved Aegis; a no-Aegis node's failure stays on
                        // the byte-identical pre-Phase-25 path (D-09).
                        // --- D-20: a timeout is the one exception -- it is
                        // always structured, whether or not the node has an
                        // Aegis, because the ENGINE's own run budget
                        // (`Timeout(EngineRun)`) can cut a node that declared
                        // no policy at all, and the bound that fired must
                        // still be a typed `TimeoutKind` on the record.
                        let node_error = match (&outcome, node_aegis.as_ref()) {
                            (NodeRunOutcome::Failed(failure), Some(_)) => {
                                failure.node_error(&nid, attempt)
                            }
                            (NodeRunOutcome::Failed(failure @ NodeFailure::Timeout(_)), None) => {
                                failure.node_error(&nid, attempt)
                            }
                            _ => None,
                        };
                        break NodeTaskOutput {
                            node_id: nid,
                            started_at,
                            duration_ms,
                            paladin_id,
                            token_count,
                            outcome,
                            attempt,
                            failed_attempts,
                            node_error,
                            cache_hit: false,
                        };
                    }
                }),
            });
        }

        let mut deltas = Vec::with_capacity(handles.len());
        let mut completed_records = Vec::with_capacity(handles.len());
        let mut node_failure: Option<(NodeId, NodeFailure, Option<NodeError>)> = None;
        // --- FT-FR-10, D-20: the first (dispatch-order) attempt this
        // superstep that the ENGINE budget cut. Checked ahead of
        // `node_failure` below: when the budget is what expired, the run
        // ends `RunTimeoutExceeded` rather than merely failing that node.
        let mut engine_budget_cut: Option<(NodeId, NodeError)> = None;
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
        // Goto target validation failure or an invalid Muster fails the
        // run before any of this bookkeeping is acted on further (checked
        // together with `node_failure`, before the merge). `parley_requests`
        // (HITL-01, D-02, D-03) is NOT a failure path: every `ParleyRequest`
        // raised anywhere this superstep is collected here and, after the
        // merge below, suspends the whole run (checked ahead of
        // `end_requested`).
        let mut goto_targets: Vec<NodeId> = Vec::new();
        let mut notfiring_nodes: HashSet<NodeId> = HashSet::new();
        let mut end_requested: Option<NodeId> = None;
        let mut routing_failure: Option<(NodeId, EngineError)> = None;
        let mut mustered: Option<(NodeId, Vec<MusterTask>)> = None;
        let mut parley_requests: Vec<ParleyRequest> = Vec::new();

        // --- HITL-04, D-19, RESEARCH.md Pitfall 1: race the WHOLE batch of
        // spawned node tasks against ONE shared grace deadline, never a
        // per-handle timeout. `results` is indexed by `dispatch_entries`
        // position (`FuturesUnordered` does not preserve insertion order),
        // so the bookkeeping loop just below still runs in dispatch order,
        // exactly as it did when this was a plain sequential `for` loop.
        // `cancel_observed_at` starts `Some` the instant the token is ALREADY
        // cancelled (e.g. observed at the top-of-loop boundary check just
        // barely before dispatch), and otherwise flips to `Some` the first
        // time the watch branch below fires -- either way, the deadline is
        // computed exactly once, from that one moment.
        let handle_count = dispatch_entries.len();
        let mut results: Vec<Option<NodeTaskOutput>> = (0..handle_count).map(|_| None).collect();
        let mut aborted_node_ids: Vec<NodeId> = Vec::new();
        // --- CR-02 (24-REVIEW.md): a Muster worker task aborted at the
        // grace deadline must NOT re-enter as an ordinary vanguard node on
        // resume (it would lose its `task_key`/`payload` `MusterContext`,
        // per `node.rs`'s worker-template contract). Tracked separately
        // from `aborted_node_ids` -- which stays exactly "ordinary vanguard
        // nodes aborted this superstep" -- so the Halted-branch below can
        // preserve the round's `MusterProgress` instead of folding a
        // worker's `NodeId` into `halted_vanguard`.
        let mut muster_task_aborted = false;
        let mut remaining: FuturesUnordered<IndexedHandle<NodeTaskOutput>> =
            handles.into_iter().collect();
        let mut cancel_observed_at: Option<tokio::time::Instant> = cancellation
            .as_ref()
            .filter(|token| token.is_cancelled())
            .map(|_| tokio::time::Instant::now());

        while !remaining.is_empty() {
            match cancel_observed_at {
                None => {
                    tokio::select! {
                        biased;
                        Some((idx, res)) = remaining.next() => {
                            match res {
                                Ok(output) => results[idx] = Some(output),
                                Err(e) => {
                                    return Err(EngineError::Node(StateNodeError(format!(
                                        "task join error: {e}"
                                    ))));
                                }
                            }
                        }
                        _ = cancelled_or_pending(cancellation) => {
                            cancel_observed_at = Some(tokio::time::Instant::now());
                        }
                    }
                }
                Some(observed_at) => {
                    let deadline = observed_at + shutdown_grace;
                    tokio::select! {
                        biased;
                        Some((idx, res)) = remaining.next() => {
                            match res {
                                Ok(output) => results[idx] = Some(output),
                                Err(e) => {
                                    return Err(EngineError::Node(StateNodeError(format!(
                                        "task join error: {e}"
                                    ))));
                                }
                            }
                        }
                        _ = tokio::time::sleep_until(deadline) => {
                            // --- D-19: every handle still outstanding at the
                            // deadline is aborted; its delta is discarded
                            // (never awaited, never merged) and its own
                            // dispatch-order id is recorded so the
                            // bookkeeping loop below can produce a
                            // `Skipped { reason: "shutdown" }` record for it
                            // without needing the (never-arriving) real
                            // outcome. `iter()` inspects without polling or
                            // removing -- `abort()` is safe to call on a
                            // handle still registered in `remaining`.
                            for handle in remaining.iter() {
                                handle.abort();
                                let (aborted_node_id, aborted_muster_ctx) =
                                    &dispatch_entries[handle.index];
                                if aborted_muster_ctx.is_some() {
                                    // A Muster worker's own dispatch-order id
                                    // is recovered on resume via
                                    // `MusterProgress::unfinished_tasks`, not
                                    // by re-adding it to the plain vanguard.
                                    muster_task_aborted = true;
                                } else {
                                    aborted_node_ids.push(aborted_node_id.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        for (dispatch_index, entry) in dispatch_entries.iter().enumerate() {
            let (entry_node_id, entry_muster_ctx) = entry;
            let is_muster_task = entry_muster_ctx.is_some();
            let Some(NodeTaskOutput {
                node_id,
                started_at,
                duration_ms,
                paladin_id,
                token_count,
                outcome,
                attempt,
                failed_attempts,
                node_error,
                cache_hit,
            }) = results[dispatch_index].take()
            else {
                // --- D-19: aborted past the shared grace deadline. Recorded
                // `Skipped { reason: "shutdown" }`, exactly like an
                // interceptor `Skip` decision above -- never pushed to
                // `deltas`/`ran`, so `frontier.record_execution` is never
                // called for it and its outgoing edges stay `Pending`
                // (acceptance 5), and never both merged and skipped.
                completed_records.push(NodeExecutionRecord {
                    node_id: entry_node_id.clone(),
                    paladin_id: None,
                    started_at: Utc::now(),
                    duration_ms: 0,
                    token_count: 0,
                    outcome: NodeOutcomeKind::Skipped {
                        reason: "shutdown".to_string(),
                    },
                    attempt: 1,
                    attempts: Vec::new(),
                    cache_hit: false,
                });
                continue;
            };
            // --- FT-FR-03, D-16: built in attempt order by the retry loop
            // (asserted, never sorted, so a regression there is loud).
            debug_assert!(
                failed_attempts
                    .windows(2)
                    .all(|w| w[0].attempt < w[1].attempt),
                "attempt history must ascend by attempt number"
            );
            // --- Plan 25-10, D-21, FT-FR-11/12/13: a FINAL failure --
            // retries exhausted, or the predicate refused a non-retryable
            // error; the retry loop above never yields `Failed` while
            // attempts remain (FT-FR-05, T-25-48) -- with a resolved
            // `on_error` handler is handed to that handler HERE, in dispatch
            // order, over the SAME pre-superstep `battlefield` every attempt
            // read (T-25-50: the merge below has not happened yet, so a
            // handler never observes partially merged state). The handler's
            // `Directive` is then honoured through the `Succeeded` arm below
            // exactly as a node's own would be -- `Edges` merges the delta
            // and fires static edges, `Goto` places its target through the
            // existing Goto machinery (`goto_targets` -> `next_vanguard`, so
            // a routed visit is counted by the SAME `visit_counts` bound as
            // any other, T-25-47), `End` completes, `Parley` suspends,
            // `Muster` fans out -- with ONE difference: the record still
            // reads `NodeOutcomeKind::Failed`, because the node DID fail;
            // the handler compensated. A handler returning `Err` fails the
            // run carrying the HANDLER's error (D-13). An attempt cut by
            // the ENGINE budget (`Timeout(EngineRun)`) is never handed to a
            // handler: the budget is gone, and the run ends
            // `RunTimeoutExceeded` below (D-20). A failure with no
            // structured error (a `DirectiveParse`/`Battalion` failure, or a
            // no-Aegis node) has no handler by construction (D-09, D-14).
            let mut node_error = node_error;
            let mut handled_failure = false;
            let outcome = match outcome {
                NodeRunOutcome::Failed(failure)
                    if !matches!(failure, NodeFailure::Timeout(TimeoutKind::EngineRun)) =>
                {
                    let handler = graph.aegis_for(&node_id).and_then(|a| a.on_error.as_ref());
                    match (handler, node_error.as_ref()) {
                        (Some(spec), Some(err)) => {
                            match dispatch_error_handler(spec, err, &battlefield, registries).await
                            {
                                Ok(directive) => {
                                    handled_failure = true;
                                    NodeRunOutcome::Succeeded(directive)
                                }
                                Err(handler_error) => {
                                    node_error = Some(handler_error);
                                    NodeRunOutcome::Failed(failure)
                                }
                            }
                        }
                        _ => NodeRunOutcome::Failed(failure),
                    }
                }
                other => other,
            };
            match outcome {
                NodeRunOutcome::Succeeded(directive) => {
                    let Directive { delta, next } = directive;
                    // --- Plan 25-11, D-22: a handler inside a Muster is
                    // delta-only. A mustered task's result is exactly ONE
                    // contribution to its Muster's aggregation; `Goto`,
                    // `End`, `Parley` and `Muster` each change control flow
                    // for the WHOLE run from inside one of many concurrent
                    // tasks, and the aggregation's semantics for that are
                    // undefined -- so the case is rejected with a typed
                    // error naming the template AND the task key, BEFORE
                    // any routing side effect (`notfiring_nodes`,
                    // `goto_targets`, `parley_requests`, `mustered`) is
                    // touched, through the same `routing_failure` path an
                    // unknown `Goto` target takes. `Route` on a template is
                    // already a validation error, so its `Goto` never
                    // reaches here in practice; the guard still covers it.
                    // A permitted `Edges` delta falls through to the
                    // `is_muster_task` branch below and lands in
                    // `muster_completed_so_far` exactly like a successful
                    // sibling's -- the aggregation sees a full task count.
                    if is_muster_task && handled_failure && !matches!(next, NextStep::Edges) {
                        completed_records.push(NodeExecutionRecord {
                            node_id: node_id.clone(),
                            paladin_id,
                            started_at,
                            duration_ms,
                            token_count,
                            outcome: NodeOutcomeKind::Failed,
                            attempt,
                            attempts: failed_attempts,
                            cache_hit: false,
                        });
                        if routing_failure.is_none() {
                            let task_key = entry_muster_ctx
                                .as_ref()
                                .map(|ctx| ctx.task_key.clone())
                                .unwrap_or_default();
                            routing_failure = Some((
                                node_id.clone(),
                                EngineError::MusterHandlerMustBeDeltaOnly {
                                    node: node_id,
                                    task_key,
                                    returned: next_step_arm_name(&next).to_string(),
                                },
                            ));
                        }
                        continue;
                    }
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
                        NextStep::Parley(request) => {
                            // HITL-01, D-02, D-03: never coerced to `Edges`
                            // -- this node's static outgoing edges resolve
                            // NotFiring for this superstep, exactly like
                            // Goto/Muster/End (D-08c uniformity: no
                            // `NextStep` variant ever leaves an edge
                            // Pending). Its own `StateDelta` still merges
                            // normally below (`deltas.push` a few lines
                            // down, unconditional on `next`) -- it already
                            // emitted it. `node_id` is stamped onto the
                            // request regardless of what the raising code
                            // supplied, so the persisted `parleys` list is
                            // always accurate.
                            notfiring_nodes.insert(node_id.clone());
                            let mut request = request.clone();
                            request.node_id = node_id.clone();
                            parley_requests.push(request);
                            NodeOutcomeKind::Parleyed
                        }
                    };
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        // Plan 25-10: a handler-compensated failure is
                        // still recorded as the failure it was (D-21).
                        outcome: if handled_failure {
                            NodeOutcomeKind::Failed
                        } else {
                            outcome_kind
                        },
                        attempt,
                        attempts: failed_attempts,
                        // Plan 25-13: `true` only for a served-from-cache
                        // outcome (`attempt: 1`, no failed attempts).
                        cache_hit,
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
                                fork_of,
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
                        attempt,
                        attempts: failed_attempts,
                        cache_hit: false,
                    });
                }
                NodeRunOutcome::Interrupted => {
                    // --- D-15, FT-FR-07: cancelled mid-backoff. Recorded
                    // like a grace-deadline abort (`Skipped { reason:
                    // "shutdown" }`, never merged, edges left `Pending`) and
                    // re-listed the same way, so the Halted Waypoint's
                    // vanguard (or its preserved `MusterProgress`) brings
                    // the node back at attempt 1 on resume instead of
                    // dropping it as an ordinary skip would.
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: NodeOutcomeKind::Skipped {
                            reason: "shutdown".to_string(),
                        },
                        attempt,
                        attempts: failed_attempts,
                        cache_hit: false,
                    });
                    if is_muster_task {
                        muster_task_aborted = true;
                    } else {
                        aborted_node_ids.push(node_id);
                    }
                }
                NodeRunOutcome::Failed(e) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id,
                        started_at,
                        duration_ms,
                        token_count,
                        outcome: NodeOutcomeKind::Failed,
                        attempt,
                        attempts: failed_attempts,
                        cache_hit: false,
                    });
                    if matches!(e, NodeFailure::Timeout(TimeoutKind::EngineRun))
                        && engine_budget_cut.is_none()
                        && let Some(cut) = node_error.clone()
                    {
                        engine_budget_cut = Some((node_id.clone(), cut));
                    }
                    if node_failure.is_none() {
                        node_failure = Some((node_id, e, node_error));
                    }
                }
            }
        }
        completed_records.sort_by(|a, b| a.node_id.cmp(&b.node_id));

        // --- FT-FR-10, D-20, ENG-FR-03: the engine budget expired
        // MID-superstep and cut an in-flight attempt. The run ends with the
        // typed `RunTimeoutExceeded` through the SAME limit-failure helper
        // the boundary-time limits use, carrying this superstep's records
        // and the cut attempt's `Timeout(EngineRun)` NodeError on the
        // Waypoint. Checked ahead of `node_failure` so the budget, not an
        // incidental sibling failure, names the outcome.
        if let Some((failed_node, cut)) = engine_budget_cut
            && let Some(limit) = graph.limits().run_timeout
        {
            let error = EngineError::RunTimeoutExceeded {
                elapsed: run_started_at.elapsed(),
                limit,
            };
            return persist_limit_failure(
                waypoint_port,
                durability,
                trace,
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                vanguard.clone(),
                completed_records,
                error,
                failed_node,
                Some(cut),
                visit_counts,
                frontier.snapshot(graph),
                checkpoint_ns.clone(),
                fork_of,
            )
            .await;
        }

        if let Some((node_id, err, node_error)) = node_failure {
            // --- D-08, X-06: an Aegis-governed node's exhausted (or
            // non-retryable) failure surfaces as the structured
            // `EngineError::NodeFailed`, rendering the SAME display line
            // the generic `EngineError::Node` rendered for it before -- so
            // `WaypointStatus::Failed.error` is unchanged while
            // `node_error` beside it carries the structure. A no-Aegis
            // node's failure keeps the generic path byte-identically
            // (D-09); a `PaladinError` on that path is erased to the exact
            // `StateNodeError(e.to_string())` it always was.
            // --- CF-02, D-11: a `DirectiveParser` parse failure gets its
            // own typed `EngineError` naming the node (X-06), rather than
            // routing through the generic `EngineError::Node` every other
            // node-execution failure uses.
            let error = match (err, node_error) {
                (
                    NodeFailure::Node(_)
                    | NodeFailure::Paladin(_)
                    | NodeFailure::StructuredOutputInvalid(_)
                    | NodeFailure::Timeout(_),
                    Some(node_error),
                ) => EngineError::NodeFailed(node_error),
                (NodeFailure::Node(e), None) => EngineError::Node(e),
                (NodeFailure::Paladin(e), None) => EngineError::Node(StateNodeError(e.to_string())),
                // --- D-29: mirrors the `Paladin(e), None` fallback
                // immediately above -- unreachable in practice (this
                // variant's own `node_error` always returns `Some`), but
                // library code must not `unreachable!()` an invariant it
                // cannot enforce.
                (NodeFailure::StructuredOutputInvalid(e), None) => {
                    EngineError::Node(StateNodeError(e.to_string()))
                }
                // --- D-20: unreachable in practice -- a timeout's
                // `node_error` is always `Some` (see the retry loop) -- but
                // library code must not `unreachable!()` an invariant it
                // cannot enforce; render through the generic path instead.
                (NodeFailure::Timeout(kind), None) => {
                    EngineError::Node(StateNodeError(format!("{kind} timeout")))
                }
                (NodeFailure::DirectiveParse(e), _) => EngineError::DirectiveParseFailed {
                    node: node_id.clone(),
                    reason: e.reason,
                },
                // --- CF-FR-16, D-21: already the fully-formed, structured
                // `EngineError::BattalionChildFailed` built in
                // `execute_vanguard_node`, where the child thread id was
                // in scope -- passed through unchanged.
                (NodeFailure::Battalion(e), _) => e,
            };
            let node_error = error.node_error().cloned();
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
                    node_error,
                },
                visit_counts,
                frontier.snapshot(graph),
                None,
                checkpoint_ns.clone(),
                fork_of,
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
                    node_error: None,
                },
                visit_counts,
                frontier.snapshot(graph),
                None,
                checkpoint_ns.clone(),
                fork_of,
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
        //
        // --- CR-02 (24-REVIEW.md): SKIPPED entirely when a sibling task was
        // aborted this round (`muster_task_aborted`). Folding-and-merging
        // the tasks that DID complete here, then ALSO persisting them
        // unmerged inside the Halted Waypoint's own `MusterProgress.completed`
        // (below) so a resumed run can re-fold them, would double-merge:
        // the resumed round re-runs this exact fold once it (genuinely)
        // finishes the whole cohort, over a battlefield that must still be
        // the pre-this-round snapshot for that fold to be correct -- exactly
        // how a mid-muster crash/resume already works (the per-task progress
        // Waypoint the loop above persists is deliberately never merged,
        // for the same reason). Ordinary (non-muster) peers that completed
        // this same aborted round are unaffected: their deltas were pushed
        // to `deltas` directly, above, and merge normally regardless.
        if !muster_task_aborted {
            for task in &muster_tasks {
                if let Some(delta) = muster_completed_so_far.get(&task.task_key) {
                    deltas.push((task.worker.clone(), delta.clone()));
                }
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
                        node_error: None,
                    },
                    visit_counts,
                    frontier.snapshot(graph),
                    None,
                    checkpoint_ns.clone(),
                    fork_of,
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
                    &registries.edge_evaluators,
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

        // --- HITL-04, D-19: one or more nodes were aborted past the shared
        // grace deadline this superstep. Every peer that finished in time
        // already merged normally above and its edges already resolved
        // (the loop just above only ever calls `frontier.record_execution`
        // for `ran`, which never includes an aborted node); this Halted
        // Waypoint's own vanguard re-lists each aborted node's id
        // ALONGSIDE the normally computed `next_vanguard` (D-19 acceptance
        // 5), so `resume` re-runs exactly those nodes exactly once, same as
        // the boundary cancellation check's own Halted Waypoint. Checked
        // BEFORE the Parley/End/starvation logic below: the run is going
        // away, so none of those normal-completion paths apply once a node
        // has actually been aborted (a mere `cancel_observed_at: Some` with
        // NO aborted node -- Test 1's case -- changes nothing here; the
        // very next loop iteration's existing top-of-loop boundary check
        // Halts before the next superstep ever dispatches).
        if !aborted_node_ids.is_empty() || muster_task_aborted {
            let mut halted_vanguard = next_vanguard.clone();
            let mut seen: HashSet<NodeId> = halted_vanguard.iter().cloned().collect();
            for node in aborted_node_ids {
                if seen.insert(node.clone()) {
                    halted_vanguard.push(node);
                }
            }
            // --- CR-02 (24-REVIEW.md): preserve this round's in-flight
            // `MusterProgress` instead of hard-coding `None` -- otherwise
            // resume forgets every sibling task `muster_completed_so_far`
            // already recorded, and re-dispatches the aborted worker as an
            // ordinary node with `NodeContext.muster == None`. Built ONLY
            // when a Muster task was actually aborted (`muster_node` may be
            // `Some` from an EARLIER superstep's now-fully-drained round
            // that produced no new tasks this superstep).
            let aborted_muster_progress = if muster_task_aborted {
                muster_node.as_ref().map(|node| MusterProgress {
                    node: node.clone(),
                    tasks: muster_tasks.clone(),
                    completed: muster_completed_so_far.clone(),
                })
            } else {
                None
            };
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                halted_vanguard,
                completed_records,
                WaypointStatus::Halted,
                visit_counts,
                frontier.snapshot(graph),
                aborted_muster_progress,
                checkpoint_ns.clone(),
                fork_of,
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::Halted {
                waypoint: waypoint.waypoint_id,
            });
        }

        // --- HITL-01, D-02, D-03, D-08: a `NextStep::Parley` raised
        // anywhere this superstep suspends the WHOLE run -- checked before
        // `end_requested` (a human's answer takes precedence over any
        // conflicting same-superstep `End`). Every peer already merged
        // normally above; the persisted `Waypoint`'s `vanguard` is
        // deliberately overridden to be EXACTLY the parleying nodes
        // (D-02), discarding whatever `compute_next_vanguard`/the `Goto`
        // union just produced -- `resume_with` alone re-seeds from this
        // exact list (D-08), so a downstream node made ready by a peer's
        // delta this superstep is picked up on the FIRST post-resume
        // superstep's own `compute_next_vanguard` call instead, once the
        // parleying node(s) have re-run. This bypasses
        // `StarvedNodeAtCompletion` entirely, mirroring `End`'s own
        // rationale just below: a pending Parley is deliberate, observable
        // suspension (recorded via `NodeOutcomeKind::Parleyed` above),
        // never a scheduler silently walking away from ready work.
        if !parley_requests.is_empty() {
            parley_requests.sort_by(|a, b| a.node_id.cmp(&b.node_id));
            let parleying_nodes: Vec<NodeId> =
                parley_requests.iter().map(|r| r.node_id.clone()).collect();
            let waypoint = build_waypoint(
                &thread,
                parent_waypoint_id,
                superstep_number,
                graph,
                &battlefield,
                parleying_nodes,
                completed_records,
                WaypointStatus::AwaitingInput {
                    parleys: parley_requests.clone(),
                    responses: Vec::new(),
                },
                visit_counts.clone(),
                frontier.snapshot(graph),
                None,
                checkpoint_ns.clone(),
                fork_of,
            );
            persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
            return Ok(RunOutcome::AwaitingInput {
                parleys: parley_requests,
                waypoint: waypoint.waypoint_id,
            });
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
                fork_of,
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
                        node_error: None,
                    },
                    visit_counts.clone(),
                    frontier.snapshot(graph),
                    None,
                    checkpoint_ns.clone(),
                    fork_of,
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
            fork_of,
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
                EngineError::Node(StateNodeError(format!(
                    "internal error: edge evaluator '{name}' missing after graph validation"
                )))
            })?;
            let output = match graph.node(source) {
                // D-06: a Gate source's `Custom`-evaluator output is its
                // `output_field` value, exactly like a Paladin node's --
                // a single added pattern on this arm, never a separate
                // match arm (`Contains`/`Regex` above need no Gate-specific
                // code at all: they already read the whole rendered
                // Battlefield JSON, and a Gate's `output_field` is an
                // ordinary schema field merged into it on the post-resume
                // superstep).
                Some(NodeSpec::Paladin { output_field, .. })
                | Some(NodeSpec::Gate {
                    output_field: Some(output_field),
                    ..
                }) => battlefield
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
pub(crate) fn build_waypoint(
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
    // --- HITL-03, D-14: the branch root this `run()` call's OWN Waypoints
    // are stamped with -- `None` for every mainline run, `Some(root)` for a
    // run entered from a branch. Propagated verbatim (never re-derived) --
    // every Waypoint a single `run_with_namespace` call produces carries the
    // SAME value.
    fork_of: Option<WaypointId>,
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
        fork_of,
    }
}

/// Persist `waypoint`, honouring `durability`: under `Strict` (the
/// default), a save failure fails the run immediately with
/// `EngineError::WaypointWrite`; under `BestEffort`, it is logged as a
/// warning and the caller proceeds as if the save had succeeded. Emits
/// `TraceEvent::WaypointSaved` (ENG-FR-21) exactly when the save actually
/// succeeded -- a `BestEffort`-swallowed failure is not reported as saved.
pub(crate) async fn persist_waypoint<W: WaypointPort>(
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
    use paladin_core::platform::container::parley::{OnExpire, ParleyId, ParleyKind};
    use paladin_core::platform::container::waypoint::ThreadId;
    use std::sync::Mutex;

    use crate::engine::directive_parser::{DirectiveParser, OnParseError};
    use crate::engine::graph::EdgeSpec;
    use crate::engine::node::StateNode;
    use crate::engine::test_support::{
        ConcurrencyTrackingNode, CountingFunctionNode, FailingFunctionNode, RecordingPaladinPort,
        RecordingWaypointStore, SlowFunctionNode, YieldingNode, shuffle_seeded,
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

    /// A grace window generous enough that no pre-existing (non-shutdown)
    /// test in this module ever observes an abort -- every helper below
    /// that does not itself test HITL-04's grace race passes this fixed
    /// value, mirroring `no_trace()`/`no_interceptors()`'s "harmless
    /// default" convention.
    fn default_shutdown_grace() -> std::time::Duration {
        std::time::Duration::from_secs(30)
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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

    /// Test 7 (plan 26-18, D-29, RT-FR-19): a plain `NodeSpec::Paladin`
    /// node (no `output_schema`) behaves EXACTLY as before -- same
    /// dispatch (`PaladinPort::execute_scoped`, never the structured
    /// executor), same raw string written to `output_field`. Run with NO
    /// structured executor wired at all (`run()`'s own `structured_executor`
    /// argument is `None`), proving the ordinary path needs none.
    #[tokio::test]
    async fn a_node_without_output_schema_is_unchanged() {
        let raw_field = field("weather");
        let s = schema(vec![FieldSpec::new(
            raw_field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("worker");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                make_paladin("worker"),
                InputMapping::new("what is the weather"),
                raw_field.clone(),
            ),
        );
        graph.add_entry(node_id);

        let recording = Arc::new(RecordingPaladinPort::new());
        recording.set_output("worker", "just a plain string, not JSON");
        let port: Arc<dyn PaladinPort> = recording.clone();

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("no-output-schema-unchanged-lib").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&raw_field),
                    Some(&serde_json::Value::String(
                        "just a plain string, not JSON".to_string()
                    )),
                    "a node with no output_schema must write the raw string verbatim"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(
            recording.call_count(),
            1,
            "dispatched through the ordinary PaladinPort path exactly once, never through a \
             structured executor (none is even wired)"
        );
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
            .validate(&CustomDispatchResolver::new(), &EngineRegistries::default())
            .expect_err("c is reachable only via Goto and not marked dynamic_target");
        assert!(matches!(err, EngineError::UnreachableNode { .. }));

        graph.mark_dynamic_target(c.clone());
        graph
            .validate(&CustomDispatchResolver::new(), &EngineRegistries::default())
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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

    // --- HITL-01, D-01/D-02/D-03: Parley suspension (Phase 24 Plan 01) ---

    /// Test 1: a `StateNode` returning `NextStep::Parley` suspends the run
    /// instead of failing it -- exactly one `AwaitingInput` Waypoint is
    /// persisted, `parleys.len() == 1`, `responses.is_empty()`, and the
    /// run returns `RunOutcome::AwaitingInput` (not `Failed`).
    #[tokio::test]
    async fn parley_suspends_run_and_persists_awaiting_input() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let asker = NodeId::new("asker");
        let asker_node = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(ParleyRequest {
                parley_id: ParleyId::new(),
                node_id: NodeId::new("wrong-on-purpose"),
                kind: ParleyKind::Approval,
                prompt: "need input".to_string(),
                payload: serde_json::json!({}),
                choices: None,
                expires_at: None,
                created_at: Utc::now(),
                on_expire: OnExpire::FailRun,
            }),
        });
        graph.add_node(asker.clone(), NodeSpec::Function(asker_node));
        graph.add_entry(asker.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("parley-suspends").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match &outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                // The engine stamps the real raising node's id, overriding
                // whatever the node itself supplied.
                assert_eq!(parleys[0].node_id, asker);
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        let saved = store.saved_waypoints(&thread).await;
        let awaiting: Vec<&Waypoint> = saved
            .iter()
            .filter(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. }))
            .collect();
        assert_eq!(
            awaiting.len(),
            1,
            "exactly one AwaitingInput waypoint must be persisted"
        );
        match &awaiting[0].status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 1);
                assert!(responses.is_empty());
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
        assert!(
            !saved
                .iter()
                .any(|w| matches!(w.status, WaypointStatus::Completed)),
            "the run must not report Completed"
        );
        assert!(
            !saved
                .iter()
                .any(|w| matches!(w.status, WaypointStatus::Failed { .. })),
            "the run must not fail"
        );
    }

    /// Test 2: a superstep with one parleying node and one ordinary peer
    /// node records `NodeOutcomeKind::Parleyed` for the parleying node,
    /// `Succeeded` for the peer, and the merged Battlefield carries both
    /// nodes' deltas.
    #[tokio::test]
    async fn parley_waypoint_records_parleyed_outcome_and_merges_peer_deltas() {
        let s = schema(vec![
            FieldSpec::new(
                FieldName::new("asker_field").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
            FieldSpec::new(
                FieldName::new("peer_field").unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            ),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let asker = NodeId::new("asker");
        let peer = NodeId::new("peer");
        let asker_field = FieldName::new("asker_field").unwrap();
        let peer_field = FieldName::new("peer_field").unwrap();
        let asker_node = {
            let asker_field = asker_field.clone();
            CountingFunctionNode::with_directive(move |_run, _state| {
                let mut delta = StateDelta::new();
                delta.set_raw(asker_field.clone(), serde_json::json!("asked"));
                Directive {
                    delta,
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
                }
            })
        };
        let peer_node = CountingFunctionNode::fixed(peer_field.clone(), serde_json::json!("ran"));
        graph.add_node(asker.clone(), NodeSpec::Function(asker_node));
        graph.add_node(peer.clone(), NodeSpec::Function(peer_node));
        graph.add_entry(asker.clone());
        graph.add_entry(peer.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("parley-peer-merge").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::AwaitingInput { .. }));

        let saved = store.saved_waypoints(&thread).await;
        let awaiting = saved
            .iter()
            .find(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. }))
            .expect("an AwaitingInput waypoint must exist");

        let asker_record = awaiting
            .completed
            .iter()
            .find(|r| r.node_id == asker)
            .expect("asker's execution record");
        assert_eq!(asker_record.outcome, NodeOutcomeKind::Parleyed);
        let peer_record = awaiting
            .completed
            .iter()
            .find(|r| r.node_id == peer)
            .expect("peer's execution record");
        assert_eq!(peer_record.outcome, NodeOutcomeKind::Succeeded);

        assert_eq!(
            awaiting.battlefield.get::<String>(&asker_field).unwrap(),
            Some("asked".to_string()),
            "the parleying node's own delta merges at raise time (D-03)"
        );
        assert_eq!(
            awaiting.battlefield.get::<String>(&peer_field).unwrap(),
            Some("ran".to_string()),
            "the peer's delta merges normally"
        );
    }

    /// Test 3: the persisted Waypoint's `vanguard` equals the set of
    /// parleying node ids -- with two parleys raised by different nodes in
    /// the same superstep, the persisted list carries both, in `node_id`
    /// order (D-02).
    #[tokio::test]
    async fn awaiting_input_vanguard_is_exactly_the_parleying_nodes() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let asker_b = NodeId::new("b-asker");
        let asker_a = NodeId::new("a-asker");
        let node_b = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(ParleyRequest {
                parley_id: ParleyId::new(),
                node_id: NodeId::new(""),
                kind: ParleyKind::Approval,
                prompt: "b?".to_string(),
                payload: serde_json::json!({}),
                choices: None,
                expires_at: None,
                created_at: Utc::now(),
                on_expire: OnExpire::FailRun,
            }),
        });
        let node_a = CountingFunctionNode::with_directive(|_run, _state| Directive {
            delta: StateDelta::new(),
            next: NextStep::Parley(ParleyRequest {
                parley_id: ParleyId::new(),
                node_id: NodeId::new(""),
                kind: ParleyKind::Approval,
                prompt: "a?".to_string(),
                payload: serde_json::json!({}),
                choices: None,
                expires_at: None,
                created_at: Utc::now(),
                on_expire: OnExpire::FailRun,
            }),
        });
        graph.add_node(asker_b.clone(), NodeSpec::Function(node_b));
        graph.add_node(asker_a.clone(), NodeSpec::Function(node_a));
        graph.add_entry(asker_b.clone());
        graph.add_entry(asker_a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("parley-two-nodes").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 2);
                let ids: Vec<NodeId> = parleys.iter().map(|p| p.node_id.clone()).collect();
                assert_eq!(
                    ids,
                    vec![asker_a.clone(), asker_b.clone()],
                    "parleys must be ordered by node_id"
                );
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }

        let saved = store.saved_waypoints(&thread).await;
        let awaiting = saved
            .iter()
            .find(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. }))
            .expect("an AwaitingInput waypoint must exist");
        assert_eq!(awaiting.vanguard, vec![asker_a, asker_b]);
    }

    // --- HITL-03 / D-14: fork_of propagation (Phase 24 Plan 06) --------

    /// Test 3: an ordinary mainline run's Waypoints all carry
    /// `fork_of: None` -- no fork/replay entry point produces a `Some`
    /// value yet (a later plan's `WarEngine::fork` is the first producer),
    /// so every Waypoint the top-level `run()`/`run_default` path writes
    /// must still show `None`.
    #[tokio::test]
    async fn mainline_waypoints_carry_no_fork_of() {
        let s = schema(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_a = NodeId::new("a");
        let node_b = NodeId::new("b");
        let result_field = field("result");
        let a_node = CountingFunctionNode::new(move |_run, _state| {
            let mut delta = StateDelta::new();
            delta.set_raw(result_field.clone(), serde_json::json!("a"));
            delta
        });
        let b_node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        graph.add_node(node_a.clone(), NodeSpec::Function(a_node));
        graph.add_node(node_b.clone(), NodeSpec::Function(b_node));
        graph.add_edge(EdgeSpec {
            from: node_a.clone(),
            to: node_b.clone(),
            condition: Some(EdgeCondition::Always),
        });
        graph.add_entry(node_a.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("mainline-no-fork-of").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let saved = store.saved_waypoints(&thread).await;
        assert!(
            !saved.is_empty(),
            "the run must have persisted at least one waypoint"
        );
        assert!(
            saved.iter().all(|w| w.fork_of.is_none()),
            "every mainline waypoint must carry fork_of: None"
        );
    }

    /// Test 4: given a Waypoint whose `fork_of` is `Some(root)`, the next
    /// Waypoint `build_waypoint` constructs for that same run also carries
    /// `Some(root)` -- the branch root propagates verbatim onto every
    /// Waypoint of the run, never re-derived per call.
    #[test]
    fn branch_waypoints_inherit_fork_of_from_the_branch_root() {
        let s = schema(vec![]);
        let graph = WarGraph::new(s.clone(), EngineLimits::default());
        let thread = ThreadId::new("branch-inherits-fork-of").unwrap();
        let battlefield = Battlefield::new(s);
        let root = WaypointId::new();

        let first = build_waypoint(
            &thread,
            None,
            1,
            &graph,
            &battlefield,
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
            None,
            None,
            Some(root),
        );
        assert_eq!(first.fork_of, Some(root));

        // The next Waypoint of the SAME (branch) run propagates the SAME
        // root verbatim, exactly as `resume_with` forwards `latest.fork_of`.
        let second = build_waypoint(
            &thread,
            Some(first.waypoint_id),
            2,
            &graph,
            &battlefield,
            vec![],
            vec![],
            WaypointStatus::Completed,
            BTreeMap::new(),
            FrontierSnapshot::default(),
            None,
            None,
            first.fork_of,
        );
        assert_eq!(second.fork_of, Some(root));
    }

    /// Test 5: forking again from a Waypoint whose own `fork_of` is already
    /// `Some(a)` yields Waypoints carrying `Some(b)` where `b` is the NEWER
    /// branch point, not `Some(a)`.
    #[test]
    fn fork_of_a_fork_carries_the_newer_root() {
        let s = schema(vec![]);
        let graph = WarGraph::new(s.clone(), EngineLimits::default());
        let thread = ThreadId::new("fork-of-a-fork").unwrap();
        let battlefield = Battlefield::new(s);
        let root_a = WaypointId::new();
        let root_b = WaypointId::new();

        // A Waypoint already on branch `a`.
        let on_branch_a = build_waypoint(
            &thread,
            None,
            1,
            &graph,
            &battlefield,
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
            None,
            None,
            Some(root_a),
        );
        assert_eq!(on_branch_a.fork_of, Some(root_a));

        // Forking AGAIN from it carries the NEWER root, `root_b`, never the
        // older `root_a` it was itself forked from.
        let on_branch_b = build_waypoint(
            &thread,
            Some(on_branch_a.waypoint_id),
            2,
            &graph,
            &battlefield,
            vec![],
            vec![],
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
            None,
            None,
            Some(root_b),
        );
        assert_eq!(on_branch_b.fork_of, Some(root_b));
        assert_ne!(on_branch_b.fork_of, on_branch_a.fork_of);
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
            ) -> Result<Directive, StateNodeError> {
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
            ) -> Result<Directive, StateNodeError> {
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

    /// A five-task Muster worker where one designated `task_key` cancels the
    /// shared shutdown token the instant it starts (before its own `.await`
    /// point, mirroring `SlowFunctionNode::cancelling`'s determinism), then
    /// sleeps well past the test's grace window; every other task_key
    /// completes immediately. Used to deterministically abort exactly one
    /// Muster task mid-round (CR-02, 24-REVIEW.md).
    fn five_task_muster_graph_with_one_slow_task(
        results_field: &FieldName,
        executed_keys: Arc<Mutex<Vec<String>>>,
        slow_key: &'static str,
        slow_hold: std::time::Duration,
        token: CancellationToken,
    ) -> (WarGraph, NodeId) {
        struct SlowKeyRecordingWorkerNode {
            field: FieldName,
            executed_keys: Arc<Mutex<Vec<String>>>,
            slow_key: &'static str,
            slow_hold: std::time::Duration,
            token: CancellationToken,
        }
        #[async_trait::async_trait]
        impl StateNode for SlowKeyRecordingWorkerNode {
            async fn run(
                &self,
                _state: &Battlefield,
                ctx: &crate::engine::node::NodeContext,
            ) -> Result<Directive, StateNodeError> {
                let key = ctx.task_key().unwrap_or_default().to_string();
                if key == self.slow_key {
                    self.token.cancel();
                    tokio::time::sleep(self.slow_hold).await;
                }
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
            NodeSpec::Function(std::sync::Arc::new(SlowKeyRecordingWorkerNode {
                field: results_field.clone(),
                executed_keys,
                slow_key,
                slow_hold,
                token,
            })),
        );
        graph.add_entry(planner);
        (graph, worker)
    }

    /// CR-02 (24-REVIEW.md) regression: a shutdown-grace abort landing
    /// mid-Muster must preserve the round's `MusterProgress` on the Halted
    /// Waypoint (not `None`), must NOT re-list the aborted worker's
    /// `NodeId` in the plain `vanguard`, and resume must re-dispatch
    /// exactly the unfinished task(s) -- with a populated `MusterContext`,
    /// not as an ordinary vanguard node.
    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_grace_abort_mid_muster_preserves_progress_for_resume() {
        let results_field = field("results");
        let token = CancellationToken::new();
        let executed_keys = Arc::new(Mutex::new(Vec::new()));
        let (graph, worker) = five_task_muster_graph_with_one_slow_task(
            &results_field,
            executed_keys.clone(),
            "c",
            std::time::Duration::from_secs(2),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-abort-mid-muster").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(50),
        )
        .await;
        assert!(
            matches!(outcome, RunOutcome::Halted { .. }),
            "aborting one Muster task past the grace deadline must Halt the run, got {outcome:?}"
        );

        // Exactly the four fast siblings ran before the abort; "c" never
        // reached its own push (aborted mid-sleep).
        let mut completed_before_abort = executed_keys.lock().unwrap().clone();
        completed_before_abort.sort();
        assert_eq!(completed_before_abort, vec!["a", "b", "d", "e"]);

        let saved = store.saved_waypoints(&thread).await;
        let halted = saved.first().expect("a Halted waypoint was saved");
        assert_eq!(halted.status, WaypointStatus::Halted);
        assert!(
            !halted.vanguard.contains(&worker),
            "the aborted Muster worker's NodeId must NOT be re-listed in the plain \
             vanguard -- it is recovered via MusterProgress::unfinished_tasks() instead, \
             got vanguard {:?}",
            halted.vanguard
        );

        let progress = halted
            .muster_progress
            .clone()
            .expect("the Halted waypoint must preserve the in-flight MusterProgress");
        assert_eq!(progress.node, NodeId::new("planner"));
        assert_eq!(progress.tasks.len(), 5, "the full task list is preserved");
        let mut completed_keys: Vec<&String> = progress.completed.keys().collect();
        completed_keys.sort();
        assert_eq!(
            completed_keys,
            vec!["a", "b", "d", "e"],
            "every sibling task completed before the abort must be recorded"
        );
        let unfinished = progress.unfinished_tasks();
        assert_eq!(
            unfinished.len(),
            1,
            "exactly the one aborted task must be unfinished"
        );
        assert_eq!(unfinished[0].task_key, "c");
        assert_eq!(unfinished[0].worker, worker);

        // Resume: mirrors `WarEngine::resume_with_options`'s own mid-muster
        // re-entry (SAME superstep, never `+ 1`), dispatching only
        // `progress.unfinished_tasks()` with its `MusterContext` populated.
        let resumed_keys = Arc::new(Mutex::new(Vec::new()));
        let (resume_graph, _worker) = five_task_muster_graph(&results_field, resumed_keys.clone());
        let resume_store = RecordingWaypointStore::new();
        let resumed_outcome = run_resumed_mid_muster(
            &resume_graph,
            thread.clone(),
            &resume_store,
            halted.battlefield.clone(),
            halted.vanguard.clone(),
            halted.visit_counts.clone(),
            halted.frontier.clone(),
            progress,
            halted.superstep,
        )
        .await;
        assert!(
            matches!(resumed_outcome, RunOutcome::Completed { .. }),
            "resume must complete the interrupted round, got {resumed_outcome:?}"
        );
        assert_eq!(
            resumed_keys.lock().unwrap().clone(),
            vec!["c".to_string()],
            "resume must re-dispatch EXACTLY the one unfinished task, with its \
             MusterContext populated (a bare, muster_ctx-less re-entry would either \
             fail to render {{muster.task_key}} or run every task again)"
        );

        let final_state = match resumed_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        };
        let mut merged = final_state
            .get::<Vec<String>>(&results_field)
            .unwrap()
            .unwrap();
        merged.sort();
        assert_eq!(
            merged,
            vec!["a", "b", "c", "d", "e"],
            "the resumed run's final Battlefield must reflect all five tasks, the four \
             restored via MusterProgress.completed plus the one re-run"
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
                error: EngineError::Node(StateNodeError(message)),
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
            ) -> Result<Directive, StateNodeError> {
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
            &EngineRegistries::default(),
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
            default_shutdown_grace(),
            None,
            None,
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
                .validate(&CustomDispatchResolver::new(), &EngineRegistries::default())
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
                .validate(&CustomDispatchResolver::new(), &EngineRegistries::default())
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
                .validate(&CustomDispatchResolver::new(), &EngineRegistries::default())
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
        registries: &EngineRegistries,
        cancellation: &Option<CancellationToken>,
    ) -> Result<RunOutcome, EngineError> {
        run(
            store.as_ref(),
            WaypointDurability::Strict,
            None,
            registry,
            registries,
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
            default_shutdown_grace(),
            None,
            None,
            None,
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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
            &EngineRegistries::default(),
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

    // --- Phase 24 Plan 08: mid-superstep shutdown-grace race (HITL-04, D-19,
    // RESEARCH.md Pitfall 1) ------------------------------------------------

    /// Race `graph`'s dispatch through the real join loop under `cancellation`
    /// and `shutdown_grace`, exposing both directly (unlike every helper
    /// above, which always passes `&None`/`default_shutdown_grace()`).
    async fn run_with_shutdown_grace(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
        cancellation: &Option<CancellationToken>,
        shutdown_grace: std::time::Duration,
    ) -> RunOutcome {
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EngineRegistries::default(),
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
            cancellation,
            None,
            shutdown_grace,
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    /// Resume `thread` from its latest (`Halted`) Waypoint through the SAME
    /// low-level `run()` entry point, restoring vanguard/battlefield/
    /// visit_counts/frontier exactly as `WarEngine::resume` would (D-19
    /// acceptance 5).
    async fn resume_after_halt(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
        shutdown_grace: std::time::Duration,
    ) -> RunOutcome {
        let latest = store
            .saved_waypoints(&thread)
            .await
            .last()
            .cloned()
            .expect("at least one Waypoint must already be saved for this thread");
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            &EngineRegistries::default(),
            graph,
            thread,
            latest.battlefield,
            latest.vanguard,
            latest.visit_counts,
            Some(latest.frontier),
            None,
            Some(latest.waypoint_id),
            latest.superstep + 1,
            &no_paladin_port(),
            &no_trace(),
            &no_interceptors(),
            &None,
            None,
            shutdown_grace,
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    /// A graph with exactly one entry node -- a [`SlowFunctionNode`] that
    /// cancels `token` the instant it starts, then sleeps `hold` -- so
    /// cancellation is always observed mid-flight, deterministically,
    /// without racing a background poller against real time.
    fn single_slow_entry_graph(
        hold: std::time::Duration,
        run_count: Arc<std::sync::atomic::AtomicUsize>,
        token: CancellationToken,
    ) -> (WarGraph, NodeId, FieldName) {
        let f = field("x");
        let s = schema(vec![FieldSpec::new(
            f.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let id = NodeId::new("slow");
        graph.add_node(
            id.clone(),
            NodeSpec::Function(SlowFunctionNode::cancelling(
                f.clone(),
                serde_json::json!("done"),
                hold,
                run_count,
                token,
            )),
        );
        graph.add_entry(id.clone());
        (graph, id, f)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn in_flight_nodes_finishing_inside_grace_merge_normally() {
        let run_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let token = CancellationToken::new();
        let (graph, _id, f) = single_slow_entry_graph(
            std::time::Duration::from_millis(30),
            run_count.clone(),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-merge-in-time").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_secs(2),
        )
        .await;

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "a node finishing inside the grace window must merge normally and complete the run, \
             got {outcome:?}"
        );
        assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert_eq!(
            first.battlefield.get_raw(&f),
            Some(&serde_json::json!("done"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn over_grace_node_is_aborted_and_recorded_skipped() {
        let run_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let token = CancellationToken::new();
        let (graph, id, f) = single_slow_entry_graph(
            std::time::Duration::from_secs(2),
            run_count.clone(),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-over-abort").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(50),
        )
        .await;

        assert!(
            matches!(outcome, RunOutcome::Halted { .. }),
            "an over-grace node must Halt the run, got {outcome:?}"
        );
        assert_eq!(
            run_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the node must have started exactly once before being aborted"
        );
        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert_eq!(
            first.battlefield.get_raw(&f),
            None,
            "an aborted node's delta must never reach the Battlefield"
        );
        let record = first
            .completed
            .iter()
            .find(|r| r.node_id == id)
            .expect("the aborted node must still have a completed record");
        assert_eq!(
            record.outcome,
            NodeOutcomeKind::Skipped {
                reason: "shutdown".to_string()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn over_grace_node_is_relisted_in_the_halted_vanguard() {
        let run_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let token = CancellationToken::new();
        let (graph, id, _f) = single_slow_entry_graph(
            std::time::Duration::from_secs(2),
            run_count.clone(),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-relisted").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(50),
        )
        .await;
        assert!(matches!(outcome, RunOutcome::Halted { .. }));

        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert!(
            first.vanguard.contains(&id),
            "the aborted node's id must be re-listed in the Halted Waypoint's vanguard, got {:?}",
            first.vanguard
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resume_reruns_the_skipped_node_exactly_once() {
        let run_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let token = CancellationToken::new();
        let (graph, _id, f) = single_slow_entry_graph(
            std::time::Duration::from_millis(150),
            run_count.clone(),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-resume-exactly-once").unwrap();
        let halted = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(20),
        )
        .await;
        assert!(matches!(halted, RunOutcome::Halted { .. }));
        assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        let resumed = resume_after_halt(
            &graph,
            thread.clone(),
            &store,
            std::time::Duration::from_secs(5),
        )
        .await;
        assert!(
            matches!(resumed, RunOutcome::Completed { .. }),
            "resuming a Halted run must re-run the skipped node and complete, got {resumed:?}"
        );
        assert_eq!(
            run_count.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "exactly one aborted attempt plus one completed attempt across the whole scenario"
        );
        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert_eq!(
            first.battlefield.get_raw(&f),
            Some(&serde_json::json!("done"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn two_slow_nodes_share_one_deadline() {
        let field_m = field("m");
        let s = schema(vec![FieldSpec::new(
            field_m.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let token = CancellationToken::new();
        let run_count_a = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let run_count_b = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let id_a = NodeId::new("a_finishes_in_time");
        let id_b = NodeId::new("b_over_grace");
        graph.add_node(
            id_a.clone(),
            NodeSpec::Function(SlowFunctionNode::cancelling(
                field_m.clone(),
                serde_json::json!("a-done"),
                std::time::Duration::from_millis(30),
                run_count_a.clone(),
                token.clone(),
            )),
        );
        graph.add_node(
            id_b.clone(),
            NodeSpec::Function(SlowFunctionNode::new(
                field_m.clone(),
                serde_json::json!("b-done"),
                std::time::Duration::from_millis(400),
                run_count_b.clone(),
            )),
        );
        graph.add_entry(id_a.clone());
        graph.add_entry(id_b.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-shared-deadline").unwrap();
        let started = std::time::Instant::now();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(100),
        )
        .await;
        let elapsed = started.elapsed();

        assert!(matches!(outcome, RunOutcome::Halted { .. }));
        assert_eq!(run_count_a.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(run_count_b.load(std::sync::atomic::Ordering::SeqCst), 1);
        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert_eq!(
            first.battlefield.get_raw(&field_m),
            Some(&serde_json::json!("a-done")),
            "the node finishing inside the shared grace window must merge normally"
        );
        assert!(
            first.vanguard.contains(&id_b) && !first.vanguard.contains(&id_a),
            "only the node still running past the shared deadline is re-listed for resume, got \
             {:?}",
            first.vanguard
        );
        // The deadline is shared, computed once -- not a fresh per-handle
        // budget. A per-handle-timeout bug would need roughly a's own
        // window PLUS b's own freshly-started window once the loop reaches
        // it; the correct, shared-deadline behavior finishes close to the
        // single 100ms window regardless of a's own completion time.
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "the shared deadline must not be penalised by node a's own completion time \
             (elapsed: {elapsed:?}) -- this is the regression signature of a per-handle timeout"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn zero_grace_aborts_immediately() {
        let run_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let token = CancellationToken::new();
        let (graph, id, _f) = single_slow_entry_graph(
            std::time::Duration::from_secs(2),
            run_count.clone(),
            token.clone(),
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-zero").unwrap();
        let started = std::time::Instant::now();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::ZERO,
        )
        .await;
        let elapsed = started.elapsed();

        assert!(matches!(outcome, RunOutcome::Halted { .. }));
        assert_eq!(run_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Duration::ZERO must abort in-flight nodes immediately, not wait for their own \
             completion (elapsed: {elapsed:?})"
        );
        let saved = store.saved_waypoints(&thread).await;
        assert!(saved.first().unwrap().vanguard.contains(&id));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn completed_records_stay_sorted_by_node_id_after_the_race() {
        let field_m = field("m");
        let s = schema(vec![FieldSpec::new(
            field_m.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let token = CancellationToken::new();
        let run_count_fast = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let run_count_slow = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        // Deliberately dispatched/declared out of NodeId order: "z_fast"
        // completes quickly (inside grace), "a_slow" is aborted -- the
        // post-race sort must still land them in NodeId order regardless.
        let id_fast = NodeId::new("z_fast");
        let id_slow = NodeId::new("a_slow");
        graph.add_node(
            id_fast.clone(),
            NodeSpec::Function(SlowFunctionNode::cancelling(
                field_m.clone(),
                serde_json::json!("fast-done"),
                std::time::Duration::from_millis(10),
                run_count_fast.clone(),
                token.clone(),
            )),
        );
        graph.add_node(
            id_slow.clone(),
            NodeSpec::Function(SlowFunctionNode::new(
                field_m.clone(),
                serde_json::json!("never"),
                std::time::Duration::from_millis(500),
                run_count_slow.clone(),
            )),
        );
        graph.add_entry(id_fast.clone());
        graph.add_entry(id_slow.clone());

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("shutdown-grace-sorted-records").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_millis(60),
        )
        .await;
        assert!(matches!(outcome, RunOutcome::Halted { .. }));

        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        let ids: Vec<&NodeId> = first.completed.iter().map(|r| &r.node_id).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        assert_eq!(
            ids, sorted_ids,
            "completed records must stay sorted by node_id regardless of completion/abort order"
        );
    }

    /// A [`StateNode`] test double that sleeps for `delay` before failing
    /// with a fixed message -- lets a test control REAL completion order
    /// independently of dispatch order (Test 8, D-19's re-indexing
    /// requirement).
    struct DelayedFailingNode {
        message: String,
        delay: std::time::Duration,
    }

    #[async_trait::async_trait]
    impl StateNode for DelayedFailingNode {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, StateNodeError> {
            tokio::time::sleep(self.delay).await;
            Err(StateNodeError(self.message.clone()))
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn first_failure_wins_is_dispatch_order_not_completion_order() {
        let s = schema(vec![]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        // Dispatch position 0 ("first") completes LATER than dispatch
        // position 1 ("second") in wall-clock time -- proving the reported
        // failure is picked by re-indexed DISPATCH order, not by whichever
        // FuturesUnordered handle happens to resolve first.
        let first = NodeId::new("first_dispatched_slow_to_fail");
        let second = NodeId::new("second_dispatched_fast_to_fail");
        graph.add_node(
            first.clone(),
            NodeSpec::Function(Arc::new(DelayedFailingNode {
                message: "first".to_string(),
                delay: std::time::Duration::from_millis(100),
            })),
        );
        graph.add_node(
            second.clone(),
            NodeSpec::Function(Arc::new(DelayedFailingNode {
                message: "second".to_string(),
                delay: std::time::Duration::ZERO,
            })),
        );
        graph.add_entry(first);
        graph.add_entry(second);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("dispatch-order-first-wins").unwrap();
        let outcome =
            run_with_shutdown_grace(&graph, thread, &store, &None, default_shutdown_grace()).await;

        match outcome {
            RunOutcome::Failed {
                error: EngineError::Node(StateNodeError(msg)),
                ..
            } => {
                assert_eq!(
                    msg, "first",
                    "the reported failure must be the earlier-DISPATCHED node's error, even \
                     though it completed later in wall-clock time"
                );
            }
            other => panic!("expected Failed(Node(\"first\")), got {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn boundary_cancellation_behaviour_is_unchanged() {
        // A regression guard on the EXISTING, untouched superstep-boundary
        // cancellation check (top of the loop): node1 cancels the token as
        // part of its own execution; superstep 1 (node1 alone) still
        // finishes and merges normally, and the boundary check for
        // superstep 2 -- not the new mid-superstep grace race -- is what
        // Halts the run before node2 is ever dispatched.
        let schema_field = field("trace");
        let closure_field = schema_field.clone();
        let s = schema(vec![FieldSpec::new(
            schema_field,
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let token = CancellationToken::new();
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");
        let node2_runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let node2_runs_clone = node2_runs.clone();
        let token_clone = token.clone();
        graph.add_node(
            node1.clone(),
            NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                token_clone.cancel();
                let mut d = StateDelta::new();
                d.set_raw(closure_field.clone(), serde_json::json!("node1"));
                d
            })),
        );
        graph.add_node(
            node2.clone(),
            NodeSpec::Function(CountingFunctionNode::new(move |_run, _state| {
                node2_runs_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                StateDelta::new()
            })),
        );
        graph.add_edge(EdgeSpec {
            from: node1.clone(),
            to: node2.clone(),
            condition: None,
        });
        graph.add_entry(node1);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("boundary-cancellation-unchanged").unwrap();
        let outcome = run_with_shutdown_grace(
            &graph,
            thread.clone(),
            &store,
            &Some(token),
            std::time::Duration::from_secs(2),
        )
        .await;

        assert!(matches!(outcome, RunOutcome::Halted { .. }));
        assert_eq!(
            node2_runs.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "node2 must never be dispatched -- the boundary check halts before superstep 2 starts"
        );
        let saved = store.saved_waypoints(&thread).await;
        let first = saved.first().unwrap();
        assert!(
            first.vanguard.contains(&node2),
            "the Halted Waypoint's vanguard must be exactly the nodes that would have run next"
        );
    }

    // --- Plan 25-09 Task 1: HeartbeatHandle, NodeContext.attempt/heartbeat(),
    // and the defaulted PaladinPort::execute_observed (D-18, D-19) ----------

    use crate::engine::test_support::{HeartbeatingNode, ObservedCallRecordingPort};

    /// A retry policy that retries `Unknown`-classified `StateNodeError`s
    /// (the default `TransientOnly` never would) with a 1 ms, jitter-free
    /// backoff -- `engine::mod`'s `retrying_aegis` shape, restated here for
    /// this module's own retry-aware tests.
    fn retrying_aegis(max_attempts: u32) -> Aegis {
        Aegis {
            retry: Some(paladin_core::platform::container::aegis::RetryPolicy {
                max_attempts,
                retry_on:
                    paladin_core::platform::container::aegis::RetryPredicate::TransientAndUnknown,
                jitter: false,
                initial_interval: std::time::Duration::from_millis(1),
                ..paladin_core::platform::container::aegis::RetryPolicy::default()
            }),
            ..Default::default()
        }
    }

    /// D-18: `ctx.heartbeat()` on a node with NO `idle_timeout` (no Aegis at
    /// all here) neither panics nor changes the run -- the handle exists
    /// but nothing is watching it, and the run completes normally.
    #[tokio::test]
    async fn heartbeat_is_a_no_op_without_an_idle_timeout() {
        let out = field("out");
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("beater");
        let node = HeartbeatingNode::new(1, 50, out.clone());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("heartbeat-no-op").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(final_state.get_raw(&out), Some(&serde_json::json!("done")));
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(node.observed_attempts(), vec![1]);
    }

    /// D-18: on a node's second attempt, `ctx.attempt` is `2` -- the
    /// context is rebuilt per attempt with the retry loop's own counter.
    #[tokio::test]
    async fn node_context_exposes_the_current_attempt() {
        let out = field("out");
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("second-try");
        let node = HeartbeatingNode::new(2, 0, out.clone());
        graph.add_node(node_id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(node_id.clone());
        graph.set_aegis(node_id, retrying_aegis(3));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("ctx-attempt").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "expected Completed, got {outcome:?}"
        );
        assert_eq!(
            node.observed_attempts(),
            vec![1, 2],
            "attempt 1 fails, attempt 2 sees ctx.attempt == 2 and succeeds"
        );
    }

    /// D-19, D-21 (plan 26-13): the engine dispatches EVERY `NodeSpec::Paladin`
    /// node through `PaladinPort::execute_scoped`, never `execute` directly.
    /// `ObservedCallRecordingPort` overrides `execute_observed`, not
    /// `execute_scoped`, so this still proves the chain: `execute_scoped`'s
    /// default body delegates to `execute_observed`, which the port
    /// overrides and records -- exactly one observed call and zero direct
    /// calls.
    #[tokio::test]
    async fn the_engine_always_calls_execute_observed() {
        let out = field("out");
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("scribe");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                make_paladin("scribe"),
                InputMapping::new("scribe"),
                out.clone(),
            ),
        );
        graph.add_entry(node_id);

        let recording = ObservedCallRecordingPort::new();
        let port: Arc<dyn PaladinPort> = recording.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("always-observed").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&out),
                    Some(&serde_json::json!("observed")),
                    "the output written is the one execute_observed produced"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(recording.observed_calls(), 1);
        assert_eq!(
            recording.direct_calls(),
            0,
            "the engine must never call execute() directly for a Paladin node"
        );
    }

    // --- Plan 25-09 Task 2: per-attempt run_timeout / idle_timeout, named
    // by the typed TimeoutKind (FT-FR-08, FT-FR-09, D-20). Every test here
    // uses the paused-clock idiom `engine/retry.rs` established
    // (`#[tokio::test(start_paused = true)]`): no wall-clock sleeps. ------

    use crate::engine::test_support::{BeatingPaladinPort, TimedFunctionNode};
    use paladin_core::platform::container::aegis::TimeoutPolicy;
    use paladin_core::platform::container::node_error::TimeoutKind;
    use std::time::Duration;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn timeout_aegis(run: Option<Duration>, idle: Option<Duration>) -> Aegis {
        Aegis {
            timeout: Some(TimeoutPolicy {
                run_timeout: run,
                idle_timeout: idle,
            }),
            ..Default::default()
        }
    }

    /// A single-`Paladin`-node graph whose node writes to `out`.
    fn one_paladin_graph(out: &FieldName) -> (WarGraph, NodeId) {
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let node_id = NodeId::new("scout");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(make_paladin("scout"), InputMapping::new("go"), out.clone()),
        );
        graph.add_entry(node_id.clone());
        (graph, node_id)
    }

    /// A single-`Function`-node graph over `node`, writing to `out`, under
    /// the default `EngineLimits`.
    fn one_function_graph(out: &FieldName, node: Arc<dyn StateNode>) -> (WarGraph, NodeId) {
        one_function_graph_with_limits(out, node, EngineLimits::default())
    }

    /// As [`one_function_graph`], under caller-supplied `limits`.
    fn one_function_graph_with_limits(
        out: &FieldName,
        node: Arc<dyn StateNode>,
        limits: EngineLimits,
    ) -> (WarGraph, NodeId) {
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, limits);
        let node_id = NodeId::new("worker");
        graph.add_node(node_id.clone(), NodeSpec::Function(node));
        graph.add_entry(node_id.clone());
        (graph, node_id)
    }

    /// FT-FR-09: a stream emitting a chunk every 100 ms is HEALTHY -- under
    /// `idle_timeout: 250 ms` and `run_timeout: 10 s`, a port that beats
    /// every 100 ms for 2 s completes, and neither bound fires.
    #[tokio::test(start_paused = true)]
    async fn a_port_beating_every_100ms_survives_a_250ms_idle_timeout() {
        let out = field("out");
        let (mut graph, node_id) = one_paladin_graph(&out);
        graph.set_aegis(node_id, timeout_aegis(Some(ms(10_000)), Some(ms(250))));

        let port_impl = BeatingPaladinPort::new(ms(100), 20, ms(0), "scouted");
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("idle-survives").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&out),
                    Some(&serde_json::json!("scouted"))
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(
            port_impl.call_count(),
            1,
            "exactly one attempt, no timeout fired"
        );
    }

    /// FT-FR-09: the SAME policy, but a port that beats then goes silent
    /// for 300 ms, fails with `Timeout(Idle)` classified `Transient`.
    #[tokio::test(start_paused = true)]
    async fn a_port_that_stalls_300ms_fails_with_timeout_idle() {
        let out = field("out");
        let (mut graph, node_id) = one_paladin_graph(&out);
        graph.set_aegis(
            node_id.clone(),
            timeout_aegis(Some(ms(10_000)), Some(ms(250))),
        );

        let port_impl = BeatingPaladinPort::new(ms(100), 2, ms(300), "never");
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("idle-fires").unwrap();
        let outcome = run_with_port(&graph, thread, &store, &port).await;

        let node_error = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        assert_eq!(node_error.node_id, node_id);
        assert_eq!(node_error.attempt, 1);
        assert_eq!(node_error.transience, Transience::Transient);
        assert_eq!(
            node_error.source,
            NodeErrorSource::Timeout(TimeoutKind::Idle),
            "the idle bound fired, named by its typed kind"
        );
        assert_eq!(port_impl.call_count(), 1, "no retry policy: one attempt");
    }

    /// FT-FR-08: a node that keeps beating is still cut by the wall-clock
    /// `run_timeout`, and the failure names `Run` (not `Idle`) -- progress
    /// cannot extend the hard cap (T-25-39).
    #[tokio::test(start_paused = true)]
    async fn a_slow_but_progressing_node_fails_on_run_timeout_not_idle() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(1_000), serde_json::json!("too late"))],
            Some(ms(100)),
        );
        let (mut graph, node_id) = one_function_graph(&out, node.clone());
        graph.set_aegis(node_id.clone(), timeout_aegis(Some(ms(500)), Some(ms(250))));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("run-beats-idle").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        let node_error = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        assert_eq!(node_error.node_id, node_id);
        assert_eq!(node_error.transience, Transience::Transient);
        assert_eq!(
            node_error.source,
            NodeErrorSource::Timeout(TimeoutKind::Run),
            "the wall-clock bound fired even though the node kept beating"
        );
        assert_eq!(node.observed_attempts(), vec![1]);
    }

    /// FT-FR-08, D-20: a `Timeout(Run)` is `Transient`, so under the
    /// default `TransientOnly` retry predicate the attempt is retried and a
    /// faster later attempt completes the run; the attempt history records
    /// the timed-out attempt with its typed kind.
    #[tokio::test(start_paused = true)]
    async fn a_timed_out_attempt_is_retried_as_transient() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![
                (ms(1_000), serde_json::json!("partial")),
                (ms(10), serde_json::json!("final")),
            ],
            None,
        );
        let (mut graph, node_id) = one_function_graph(&out, node.clone());
        graph.set_aegis(
            node_id.clone(),
            Aegis {
                retry: Some(paladin_core::platform::container::aegis::RetryPolicy {
                    max_attempts: 3,
                    jitter: false,
                    initial_interval: ms(1),
                    ..paladin_core::platform::container::aegis::RetryPolicy::default()
                }),
                timeout: Some(TimeoutPolicy {
                    run_timeout: Some(ms(200)),
                    idle_timeout: None,
                }),
                ..Default::default()
            },
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("timeout-retried").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        assert!(
            matches!(outcome, RunOutcome::Completed { .. }),
            "expected Completed, got {outcome:?}"
        );
        assert_eq!(node.observed_attempts(), vec![1, 2]);
        let waypoints = store.saved_waypoints(&thread).await;
        let record = &waypoints[0].completed[0];
        assert_eq!(record.node_id, node_id);
        assert_eq!(record.attempt, 2, "the succeeding attempt is attempt 2");
        assert_eq!(record.attempts.len(), 1);
        assert_eq!(record.attempts[0].attempt, 1);
        assert_eq!(
            record.attempts[0].error.source,
            NodeErrorSource::Timeout(TimeoutKind::Run)
        );
        assert_eq!(record.attempts[0].error.transience, Transience::Transient);
    }

    /// FT-FR-03/FT-FR-08 (T-25-41): a timed-out attempt's partial delta is
    /// discarded exactly like any other failed attempt's -- only the
    /// succeeding attempt's value reaches the merged Battlefield.
    #[tokio::test(start_paused = true)]
    async fn a_timed_out_attempts_partial_work_is_discarded() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![
                (ms(1_000), serde_json::json!("partial")),
                (ms(10), serde_json::json!("final")),
            ],
            None,
        );
        let (mut graph, node_id) = one_function_graph(&out, node.clone());
        graph.set_aegis(
            node_id,
            Aegis {
                retry: Some(paladin_core::platform::container::aegis::RetryPolicy {
                    max_attempts: 2,
                    jitter: false,
                    initial_interval: ms(1),
                    ..paladin_core::platform::container::aegis::RetryPolicy::default()
                }),
                timeout: Some(TimeoutPolicy {
                    run_timeout: Some(ms(200)),
                    idle_timeout: None,
                }),
                ..Default::default()
            },
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("partial-discarded").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&out),
                    Some(&serde_json::json!("final")),
                    "only the succeeding attempt's delta merges"
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(
            waypoints.len(),
            1,
            "no Waypoint is written between attempts"
        );
        assert_ne!(
            waypoints[0].battlefield.get_raw(&out),
            Some(&serde_json::json!("partial"))
        );
    }

    /// `TimeoutPolicy { run_timeout: None, idle_timeout: None }` arms
    /// nothing: a node under it behaves exactly as one with no policy --
    /// a 5 s hold completes with one attempt and no timer ever fires.
    #[tokio::test(start_paused = true)]
    async fn timeout_policy_with_both_fields_none_is_a_no_op() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(5_000), serde_json::json!("eventually"))],
            None,
        );
        let (mut graph, node_id) = one_function_graph(&out, node.clone());
        graph.set_aegis(node_id.clone(), timeout_aegis(None, None));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("no-op-policy").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&out),
                    Some(&serde_json::json!("eventually"))
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
        let waypoints = store.saved_waypoints(&thread).await;
        let record = &waypoints[0].completed[0];
        assert_eq!(record.node_id, node_id);
        assert_eq!(record.attempt, 1);
        assert!(record.attempts.is_empty());
    }

    /// T-25-42: which bound fired is read from the typed `TimeoutKind`
    /// field by value -- this test never renders the error to a string and
    /// never inspects a message.
    #[tokio::test(start_paused = true)]
    async fn the_fired_bound_is_read_from_the_typed_kind() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(1_000), serde_json::json!("silent"))],
            None,
        );
        let (mut graph, node_id) = one_function_graph(&out, node);
        graph.set_aegis(node_id, timeout_aegis(Some(ms(10_000)), Some(ms(250))));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("typed-kind").unwrap();
        let outcome = run_default(&graph, thread, &store).await;

        let node_error = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        // Typed, by value: a `match` on the variant, not a substring search.
        let fired: TimeoutKind = match node_error.source {
            NodeErrorSource::Timeout(kind) => kind,
            other => panic!("expected a Timeout source, got {other:?}"),
        };
        assert_eq!(fired, TimeoutKind::Idle);
        assert_ne!(fired, TimeoutKind::Run);
    }

    // --- Plan 25-09 Task 3: the run-level bound -- EngineLimits.run_timeout
    // enforced, nested with the per-attempt bound, and named (D-20,
    // FT-FR-10, ENG-FR-03). ------------------------------------------------

    /// A three-node chain `a -> b -> c` of `TimedFunctionNode`s each holding
    /// `hold` and writing `out`, under `limits`.
    fn three_step_chain(out: &FieldName, hold: Duration, limits: EngineLimits) -> WarGraph {
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, limits);
        let ids: Vec<NodeId> = ["a", "b", "c"].into_iter().map(NodeId::new).collect();
        for id in &ids {
            let node = TimedFunctionNode::new(
                out.clone(),
                vec![(hold, serde_json::json!(id.as_str()))],
                None,
            );
            graph.add_node(id.clone(), NodeSpec::Function(node));
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

    /// ENG-FR-03, D-20: a run whose total wall clock exhausts
    /// `EngineLimits.run_timeout` at a superstep BOUNDARY ends with the
    /// typed `EngineError::RunTimeoutExceeded`, and writes the SAME kind of
    /// `Failed` Waypoint `NodeVisitLimitExceeded`/`RecursionLimitExceeded`
    /// write today: `node_error: None`, `failed_node` = the first node that
    /// would have run next, the pending vanguard recorded verbatim.
    ///
    /// Shape: two 100 ms supersteps under a 200 ms budget -- superstep 2's
    /// node completes on the SAME virtual tick the budget expires (the race
    /// is biased toward the result, so it merges), and the top-of-loop
    /// check then ends the run before superstep 3 starts.
    #[tokio::test(start_paused = true)]
    async fn engine_run_timeout_ends_the_run_with_a_typed_error() {
        let out = field("out");
        let graph = three_step_chain(
            &out,
            ms(100),
            EngineLimits {
                run_timeout: Some(ms(200)),
                ..EngineLimits::default()
            },
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("engine-run-timeout-boundary").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        let waypoint_id = match outcome {
            RunOutcome::Failed {
                error: EngineError::RunTimeoutExceeded { elapsed, limit },
                waypoint: Some(waypoint_id),
            } => {
                assert_eq!(limit, ms(200));
                assert!(elapsed >= limit, "elapsed {elapsed:?} >= limit {limit:?}");
                waypoint_id
            }
            other => panic!("expected Failed(RunTimeoutExceeded), got {other:?}"),
        };

        let saved = store.saved_waypoints(&thread).await;
        let failed = saved
            .iter()
            .find(|w| w.waypoint_id == waypoint_id)
            .expect("the failure Waypoint was persisted");
        match &failed.status {
            WaypointStatus::Failed {
                failed_node,
                node_error,
                ..
            } => {
                assert_eq!(
                    failed_node,
                    &NodeId::new("c"),
                    "the first node that would run next"
                );
                assert!(
                    node_error.is_none(),
                    "a limit failure carries no NodeError, like NodeVisitLimitExceeded"
                );
            }
            other => panic!("expected a Failed Waypoint, got {other:?}"),
        }
        assert_eq!(failed.vanguard, vec![NodeId::new("c")]);
        assert_eq!(
            failed.battlefield.get_raw(&out),
            Some(&serde_json::json!("b")),
            "superstep 2 merged before the boundary check ended the run"
        );
    }

    /// FT-FR-10, D-20: an attempt cut MID-SUPERSTEP by the engine budget
    /// records `Timeout(EngineRun)` -- not `Run`, not `Idle` -- on the
    /// failure Waypoint, and the run ends `RunTimeoutExceeded` rather than
    /// merely failing that node.
    #[tokio::test(start_paused = true)]
    async fn an_attempt_cut_by_the_engine_bound_records_timeout_enginerun() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(1_000), serde_json::json!("never"))],
            None,
        );
        let (graph, node_id) = one_function_graph_with_limits(
            &out,
            node.clone(),
            EngineLimits {
                run_timeout: Some(ms(150)),
                ..EngineLimits::default()
            },
        );

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("engine-run-cuts-attempt").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;

        let waypoint_id = match outcome {
            RunOutcome::Failed {
                error: EngineError::RunTimeoutExceeded { limit, .. },
                waypoint: Some(waypoint_id),
            } => {
                assert_eq!(limit, ms(150));
                waypoint_id
            }
            other => panic!("expected Failed(RunTimeoutExceeded), got {other:?}"),
        };

        let saved = store.saved_waypoints(&thread).await;
        let failed = saved
            .iter()
            .find(|w| w.waypoint_id == waypoint_id)
            .expect("the failure Waypoint was persisted");
        match &failed.status {
            WaypointStatus::Failed {
                failed_node,
                node_error: Some(node_error),
                ..
            } => {
                assert_eq!(failed_node, &node_id);
                assert_eq!(node_error.node_id, node_id);
                assert_eq!(node_error.attempt, 1);
                assert_eq!(node_error.transience, Transience::Transient);
                assert_eq!(
                    node_error.source,
                    NodeErrorSource::Timeout(TimeoutKind::EngineRun)
                );
            }
            other => panic!("expected Failed with a NodeError, got {other:?}"),
        }
        let record = failed
            .completed
            .iter()
            .find(|r| r.node_id == node_id)
            .expect("the cut node's record is on the Waypoint");
        assert_eq!(record.attempt, 1, "never retried: the budget is gone");
        assert!(matches!(record.outcome, NodeOutcomeKind::Failed));
        assert_eq!(node.observed_attempts(), vec![1]);
    }

    /// D-20: the per-attempt deadline is `min(attempt run_timeout,
    /// remaining engine budget)` and the fired kind names whichever was
    /// tightest -- both directions, by typed kind.
    #[tokio::test(start_paused = true)]
    async fn the_tightest_bound_fires() {
        // --- (a) attempt bound 10 s, engine budget 200 ms -> EngineRun.
        let out = field("out");
        let slow = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(1_000), serde_json::json!("never"))],
            None,
        );
        let (mut graph, node_id) = one_function_graph_with_limits(
            &out,
            slow,
            EngineLimits {
                run_timeout: Some(ms(200)),
                ..EngineLimits::default()
            },
        );
        graph.set_aegis(node_id.clone(), timeout_aegis(Some(ms(10_000)), None));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("tightest-engine").unwrap();
        let outcome = run_default(&graph, thread.clone(), &store).await;
        assert!(
            matches!(
                outcome,
                RunOutcome::Failed {
                    error: EngineError::RunTimeoutExceeded { .. },
                    ..
                }
            ),
            "expected RunTimeoutExceeded, got {outcome:?}"
        );
        let saved = store.saved_waypoints(&thread).await;
        let engine_kind = match &saved[0].status {
            WaypointStatus::Failed {
                node_error: Some(node_error),
                ..
            } => match node_error.source {
                NodeErrorSource::Timeout(kind) => kind,
                ref other => panic!("expected Timeout, got {other:?}"),
            },
            other => panic!("expected Failed with a NodeError, got {other:?}"),
        };
        assert_eq!(engine_kind, TimeoutKind::EngineRun);

        // --- (b) attempt bound 200 ms, engine budget 10 s -> Run.
        let slow = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(1_000), serde_json::json!("never"))],
            None,
        );
        let (mut graph, node_id) = one_function_graph_with_limits(
            &out,
            slow,
            EngineLimits {
                run_timeout: Some(ms(10_000)),
                ..EngineLimits::default()
            },
        );
        graph.set_aegis(node_id, timeout_aegis(Some(ms(200)), None));

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("tightest-attempt").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        let node_error = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        let attempt_kind = match node_error.source {
            NodeErrorSource::Timeout(kind) => kind,
            other => panic!("expected Timeout, got {other:?}"),
        };
        assert_eq!(attempt_kind, TimeoutKind::Run);
    }

    /// `EngineLimits { run_timeout: None, .. }` (the default) arms no
    /// run-level bound: a 5 s node under the default limits completes.
    #[tokio::test(start_paused = true)]
    async fn no_engine_run_timeout_means_no_run_level_bound() {
        let out = field("out");
        let node = TimedFunctionNode::new(
            out.clone(),
            vec![(ms(5_000), serde_json::json!("eventually"))],
            None,
        );
        let (graph, _) = one_function_graph(&out, node);
        assert_eq!(graph.limits().run_timeout, None);

        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("no-engine-bound").unwrap();
        let outcome = run_default(&graph, thread, &store).await;
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                assert_eq!(
                    final_state.get_raw(&out),
                    Some(&serde_json::json!("eventually"))
                );
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Phase 23 D-18 regression guard: `EngineLimits.run_timeout` stays
    /// EXCLUDED from `WarGraph::fingerprint()` like every other limit, so
    /// loosening or tightening the run budget never looks like a graph
    /// change to `resume`'s `GraphMismatch` check.
    #[test]
    fn run_timeout_is_not_hashed_into_the_fingerprint() {
        let out = field("out");
        let s = schema(vec![FieldSpec::new(
            out.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let a = WarGraph::new(
            s.clone(),
            EngineLimits {
                run_timeout: None,
                ..EngineLimits::default()
            },
        );
        let b = WarGraph::new(
            s,
            EngineLimits {
                run_timeout: Some(Duration::from_secs(90)),
                ..EngineLimits::default()
            },
        );
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    /// D-20, X-03: the legacy-bridge `WarGraph`s carry NO legacy Battalion
    /// timeout into `EngineLimits` -- `run_timeout` comes from `EngineConfig`
    /// only -- so PRD 04 FT-FR-10's "any legacy Battalion timeout" clause is
    /// satisfied vacuously. Asserted both behaviourally (every bridge yields
    /// `run_timeout: None`) and as a source tripwire on `bridges.rs`.
    #[test]
    fn bridges_carry_no_legacy_battalion_timeout() {
        assert_eq!(
            WarGraph::from_formation(Vec::new()).limits().run_timeout,
            None
        );
        assert_eq!(
            WarGraph::from_phalanx(Vec::new()).limits().run_timeout,
            None
        );
        let bridges_source = include_str!("bridges.rs");
        assert!(
            !bridges_source.contains("run_timeout"),
            "bridges.rs must never set EngineLimits.run_timeout from a legacy Battalion timeout"
        );
        assert!(
            bridges_source.contains("EngineLimits::default()"),
            "every bridge builds its EngineLimits from the default (run_timeout: None)"
        );
    }

    // --- Plan 25-10 Task 2: Route / Absorb / no-handler dispatch after
    // retries exhaust (D-21, D-08, FT-FR-05, FT-FR-11, FT-FR-12, FT-FR-14) --

    use crate::engine::test_support::{FailingPaladinPort, PermanentlyFailingNode, RecoveryNode};
    use paladin_core::platform::container::aegis::{ErrorHandlerSpec, RetryPolicy};
    use paladin_core::platform::container::node_error::NodeErrorSource;
    use paladin_core::platform::container::transience::Transience;

    /// `result` (the ordinary output field), `booking_error` (a Route's
    /// `error_field`) and `recovered` (what a recovery node or an Absorb
    /// fallback writes) -- all `LastWrite`.
    fn handler_schema() -> BattlefieldSchema {
        schema(
            ["result", "booking_error", "recovered"]
                .into_iter()
                .map(|name| FieldSpec::new(field(name), DispatchRule::LastWrite, None, false))
                .collect(),
        )
    }

    fn route_aegis(to: &NodeId, error_field: &FieldName) -> Aegis {
        Aegis {
            on_error: Some(ErrorHandlerSpec::Route {
                to: to.clone(),
                error_field: error_field.clone(),
            }),
            ..Aegis::default()
        }
    }

    fn absorb_aegis(fallback_delta: StateDelta) -> Aegis {
        Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb { fallback_delta }),
            ..Aegis::default()
        }
    }

    /// `TransientOnly` (the default predicate), jitter-free, 1 ms initial
    /// interval -- so a Permanent failure gets exactly one attempt and a
    /// Transient one retries up to `max_attempts` under the paused clock.
    fn transient_only_retry(max_attempts: u32) -> RetryPolicy {
        RetryPolicy {
            max_attempts,
            jitter: false,
            initial_interval: Duration::from_millis(1),
            ..RetryPolicy::default()
        }
    }

    /// A port whose every call fails `Permanent` (a `ConfigurationError`
    /// classifies Permanent by `PaladinError::transience`, D-05).
    fn permanent_port() -> Arc<FailingPaladinPort> {
        FailingPaladinPort::new(|| PaladinError::ConfigurationError("card declined".to_string()))
    }

    /// A port whose every call fails with a 503-classified `Transient`
    /// `LlmFailure`.
    fn transient_port() -> Arc<FailingPaladinPort> {
        FailingPaladinPort::new(|| PaladinError::LlmFailure {
            transience: Transience::Transient,
            status: Some(503),
            provider: Some("mock".to_string()),
            message: "service unavailable".to_string(),
        })
    }

    fn paladin_node(name: &str, out: &FieldName) -> NodeSpec {
        NodeSpec::paladin(make_paladin(name), InputMapping::new("go"), out.clone())
    }

    /// Run `graph` from its entry through the real superstep loop over
    /// `port`, with `registries` (so a `Custom` handler resolves).
    async fn run_handled(
        graph: &WarGraph,
        thread: ThreadId,
        store: &RecordingWaypointStore,
        port: &Arc<dyn PaladinPort>,
        registries: &EngineRegistries,
    ) -> RunOutcome {
        run(
            store,
            WaypointDurability::Strict,
            None,
            &CustomDispatchResolver::new(),
            registries,
            graph,
            thread,
            Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap(),
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
            default_shutdown_grace(),
            None,
            None,
            None,
        )
        .await
        .unwrap()
    }

    /// The `NodeExecutionRecord` for `node` across every saved Waypoint of
    /// `thread` (oldest superstep first).
    async fn records_for(
        store: &RecordingWaypointStore,
        thread: &ThreadId,
        node: &NodeId,
    ) -> Vec<NodeExecutionRecord> {
        let mut waypoints = store.saved_waypoints(thread).await;
        waypoints.reverse();
        waypoints
            .iter()
            .flat_map(|wp| wp.completed.iter().filter(|r| &r.node_id == node).cloned())
            .collect()
    }

    fn completed_state(outcome: RunOutcome) -> Battlefield {
        match outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// `book` (a Paladin node under `aegis`) with `cancel` (a
    /// [`RecoveryNode`] writing `recovered = "cancelled"`) declared but NOT
    /// statically wired -- reachable only by routing.
    fn book_cancel_graph(aegis: Aegis) -> (WarGraph, NodeId, NodeId, Arc<RecoveryNode>) {
        let mut graph = WarGraph::new(handler_schema(), EngineLimits::default());
        let book = NodeId::new("book");
        let cancel = NodeId::new("cancel");
        let recovery = RecoveryNode::new(field("recovered"), serde_json::json!("cancelled"));
        graph.add_node(book.clone(), paladin_node("book", &field("result")));
        graph.add_node(cancel.clone(), NodeSpec::Function(recovery.clone()));
        graph.add_entry(book.clone());
        graph.set_aegis(book.clone(), aegis);
        (graph, book, cancel, recovery)
    }

    /// FT-FR-11, D-21: the compensation-chain shape from CONTEXT.md --
    /// `book` fails permanently, routes to `cancel`, the run Completes,
    /// `booking_error` holds the NodeError JSON, `book`'s record reads
    /// `outcome: Failed`. Compared as PARSED JSON fields, never bytes.
    #[tokio::test]
    async fn route_writes_the_structured_error_and_places_the_target() {
        let error_field = field("booking_error");
        let (graph, book, cancel, recovery) =
            book_cancel_graph(route_aegis(&NodeId::new("cancel"), &error_field));
        let port_impl = permanent_port();
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("route-structured").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        let written = final_state
            .get_raw(&error_field)
            .expect("booking_error holds the routed NodeError");
        assert_eq!(written["node_id"], serde_json::json!("book"));
        assert_eq!(written["attempt"], serde_json::json!(1));
        assert_eq!(written["transience"], serde_json::json!("Permanent"));
        let message = written["source"]["Paladin"]["message"]
            .as_str()
            .expect("source.Paladin.message is a string");
        assert!(
            message.contains("card declined"),
            "the source message matches the failure: {message}"
        );
        // And it round-trips into the typed value.
        let parsed: NodeError =
            serde_json::from_value(written.clone()).expect("error_field parses as a NodeError");
        assert_eq!(parsed.node_id, book);
        assert_eq!(parsed.transience, Transience::Permanent);
        assert!(matches!(parsed.source, NodeErrorSource::Paladin { .. }));

        assert_eq!(port_impl.call_count(), 1, "no retry policy: one attempt");
        assert!(recovery.ran(), "the Route target ran");
        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("cancelled"))
        );
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records.len(), 1);
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
        assert_eq!(book_records[0].attempt, 1);
        let cancel_records = records_for(&store, &thread, &cancel).await;
        assert_eq!(cancel_records.len(), 1);
        assert_eq!(cancel_records[0].outcome, NodeOutcomeKind::Succeeded);
    }

    /// D-21: the routed target REPLACES the failed node's static
    /// successors -- `confirm` (book's ordinary successor) never runs.
    #[tokio::test]
    async fn route_replaces_the_failed_nodes_static_successors() {
        let error_field = field("booking_error");
        let (mut graph, book, _cancel, recovery) =
            book_cancel_graph(route_aegis(&NodeId::new("cancel"), &error_field));
        let confirm = NodeId::new("confirm");
        let confirm_node = RecoveryNode::new(field("result"), serde_json::json!("confirmed"));
        graph.add_node(confirm.clone(), NodeSpec::Function(confirm_node.clone()));
        graph.add_edge(EdgeSpec {
            from: book.clone(),
            to: confirm.clone(),
            condition: None,
        });
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("route-replaces").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert!(recovery.ran(), "only the routed target ran");
        assert!(
            !confirm_node.ran(),
            "the failed node's static successor must NOT run"
        );
        assert!(final_state.get_raw(&field("result")).is_none());
        assert!(records_for(&store, &thread, &confirm).await.is_empty());
    }

    /// D-21: routing to a terminal recovery node (no outgoing edges) ends
    /// the run `Completed` -- the final Waypoint's status says so, and no
    /// starvation failure is raised for the unrun static successors.
    #[tokio::test]
    async fn a_route_target_with_no_outgoing_edges_completes_the_run_normally() {
        let error_field = field("booking_error");
        let (graph, _book, cancel, recovery) =
            book_cancel_graph(route_aegis(&NodeId::new("cancel"), &error_field));
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("route-terminal").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert!(recovery.ran());
        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("cancelled"))
        );
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 2, "one superstep for book, one for cancel");
        assert_eq!(waypoints[0].status, WaypointStatus::Completed);
        assert!(waypoints[0].vanguard.is_empty());
        // The first Waypoint's vanguard placed exactly the Route target.
        assert_eq!(waypoints[1].vanguard, vec![cancel]);
        assert_eq!(waypoints[1].status, WaypointStatus::Running);
    }

    /// FT-FR-12, D-21: under `Absorb`, the record reads `Failed`, the
    /// fallback delta merges, and the node's static edges fire as on
    /// success.
    #[tokio::test]
    async fn absorb_merges_its_delta_and_fires_static_edges() {
        let mut graph = WarGraph::new(handler_schema(), EngineLimits::default());
        let book = NodeId::new("book");
        let confirm = NodeId::new("confirm");
        let failing = PermanentlyFailingNode::new("booking service down");
        let confirm_node = RecoveryNode::new(field("result"), serde_json::json!("confirmed"));
        graph.add_node(book.clone(), NodeSpec::Function(failing.clone()));
        graph.add_node(confirm.clone(), NodeSpec::Function(confirm_node.clone()));
        graph.add_edge(EdgeSpec {
            from: book.clone(),
            to: confirm.clone(),
            condition: None,
        });
        graph.add_entry(book.clone());
        let mut fallback = StateDelta::new();
        fallback.set_raw(field("recovered"), serde_json::json!("fallback"));
        graph.set_aegis(book.clone(), absorb_aegis(fallback));
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("absorb-merges").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert_eq!(failing.run_count(), 1);
        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("fallback")),
            "the fallback delta merged"
        );
        assert!(confirm_node.ran(), "the static edge fired as on success");
        assert_eq!(
            final_state.get_raw(&field("result")),
            Some(&serde_json::json!("confirmed"))
        );
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records.len(), 1);
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
        // The absorbed node's own delta never existed; only the fallback
        // and confirm's write are in the final state.
        assert!(final_state.get_raw(&field("booking_error")).is_none());
    }

    /// FT-FR-12: an EMPTY fallback delta merges nothing -- the Battlefield
    /// is unchanged by the absorbed node -- yet its static edges still
    /// fire and the record still reads `Failed`.
    #[tokio::test]
    async fn absorb_with_an_empty_fallback_delta_merges_nothing_and_continues() {
        let mut graph = WarGraph::new(handler_schema(), EngineLimits::default());
        let book = NodeId::new("book");
        let confirm = NodeId::new("confirm");
        let failing = PermanentlyFailingNode::new("booking service down");
        let confirm_node = RecoveryNode::new(field("result"), serde_json::json!("confirmed"));
        graph.add_node(book.clone(), NodeSpec::Function(failing.clone()));
        graph.add_node(confirm.clone(), NodeSpec::Function(confirm_node.clone()));
        graph.add_edge(EdgeSpec {
            from: book.clone(),
            to: confirm.clone(),
            condition: None,
        });
        graph.add_entry(book.clone());
        graph.set_aegis(book.clone(), absorb_aegis(StateDelta::new()));
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("absorb-empty").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        // The Battlefield `confirm` observed (the post-absorb merge) is
        // byte-identical to the initial state: nothing merged for `book`.
        let observed = confirm_node.observed();
        assert_eq!(observed.len(), 1);
        let initial = Battlefield::initialize(graph.schema().clone(), &StateDelta::new()).unwrap();
        assert_eq!(observed[0], initial, "the absorbed node merged nothing");
        assert!(confirm_node.ran(), "static edges still fire");
        assert_eq!(
            final_state.get_raw(&field("result")),
            Some(&serde_json::json!("confirmed"))
        );
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records.len(), 1);
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
    }

    /// FT-FR-14, D-08: with no `on_error`, an exhausted failure writes a
    /// `Failed` Waypoint whose `node_error` is `Some(..)` and returns
    /// `RunOutcome::Failed` carrying the same value -- never a bare string.
    #[tokio::test]
    async fn no_handler_fails_the_run_with_the_structured_error() {
        let (graph, book, _cancel, recovery) = book_cancel_graph(Aegis {
            retry: Some(transient_only_retry(3)),
            ..Aegis::default()
        });
        // `cancel` is declared but, with no Route, never reached -- it is
        // the control proving nothing was routed.
        let port_impl = permanent_port();
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("no-handler").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;

        let exposed = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        assert_eq!(exposed.node_id, book);
        assert_eq!(
            exposed.attempt, 1,
            "Permanent under TransientOnly: one attempt"
        );
        assert_eq!(exposed.transience, Transience::Permanent);
        assert!(matches!(
            &outcome,
            RunOutcome::Failed {
                error: EngineError::NodeFailed(_),
                ..
            }
        ));
        assert!(!recovery.ran(), "no handler: nothing routed");
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 1);
        match &waypoints[0].status {
            WaypointStatus::Failed {
                failed_node,
                node_error,
                ..
            } => {
                assert_eq!(failed_node, &book);
                assert_eq!(node_error.as_ref(), Some(&exposed));
            }
            other => panic!("expected a Failed Waypoint, got {other:?}"),
        }
    }

    /// FT-FR-05, T-25-48: a Transient failure under `max_attempts: 3`
    /// reaches the handler only after the THIRD attempt fails -- the
    /// recovery node runs exactly once, after three port calls, and the
    /// routed error records `attempt: 3`.
    #[tokio::test(start_paused = true)]
    async fn a_handler_does_not_run_while_retries_remain() {
        let error_field = field("booking_error");
        let (graph, _book, _cancel, recovery) = book_cancel_graph(Aegis {
            retry: Some(transient_only_retry(3)),
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: error_field.clone(),
            }),
            ..Aegis::default()
        });
        let port_impl = transient_port();
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("handler-after-retries").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert_eq!(port_impl.call_count(), 3, "every attempt ran first");
        assert_eq!(
            recovery.run_count(),
            1,
            "exactly one handler invocation, after exhaustion"
        );
        let written = final_state.get_raw(&error_field).expect("routed error");
        assert_eq!(written["attempt"], serde_json::json!(3));
        assert_eq!(written["transience"], serde_json::json!("Transient"));
        assert_eq!(written["source"]["Llm"]["status"], serde_json::json!(503));
        let book_records = records_for(&store, &thread, &NodeId::new("book")).await;
        assert_eq!(
            book_records.len(),
            1,
            "one record, no Waypoint between attempts"
        );
        assert_eq!(book_records[0].attempt, 3);
        assert_eq!(
            book_records[0].attempts.len(),
            2,
            "two failed attempts before the final"
        );
    }

    /// FT-FR-05: a Permanent failure under `TransientOnly` invokes the
    /// handler after exactly one attempt -- the retry predicate refused it,
    /// and the handler is entered at once.
    #[tokio::test]
    async fn a_non_retryable_error_reaches_the_handler_immediately() {
        let error_field = field("booking_error");
        let (graph, _book, _cancel, recovery) = book_cancel_graph(Aegis {
            retry: Some(transient_only_retry(3)),
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: error_field.clone(),
            }),
            ..Aegis::default()
        });
        let port_impl = permanent_port();
        let port: Arc<dyn PaladinPort> = port_impl.clone();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("handler-immediate").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert_eq!(port_impl.call_count(), 1, "exactly one attempt");
        assert_eq!(recovery.run_count(), 1);
        let written = final_state.get_raw(&error_field).expect("routed error");
        assert_eq!(written["attempt"], serde_json::json!(1));
        assert_eq!(written["transience"], serde_json::json!("Permanent"));
        let book_records = records_for(&store, &thread, &NodeId::new("book")).await;
        assert_eq!(book_records[0].attempt, 1);
        assert!(book_records[0].attempts.is_empty());
    }

    // --- Plan 25-10 Task 3: Custom handler dispatch and the
    // max_node_visits loop bound (D-13, D-21, FT-FR-13, FT-FR-15) ----------

    use crate::engine::test_support::RecordingErrorHandler;

    fn custom_aegis(name: &str) -> Aegis {
        Aegis {
            on_error: Some(ErrorHandlerSpec::Custom(name.to_string())),
            ..Aegis::default()
        }
    }

    fn registries_with(name: &str, handler: Arc<RecordingErrorHandler>) -> EngineRegistries {
        let mut registries = EngineRegistries::default();
        registries.error_handlers.register(name, handler);
        registries
    }

    fn delta_with(name: &str, value: serde_json::Value) -> StateDelta {
        let mut delta = StateDelta::new();
        delta.set_raw(field(name), value);
        delta
    }

    /// `prep` (writes `result = "pre"`) -> `book` (Paladin, fails Permanent,
    /// `Custom("compensate")`) -> `confirm` (a [`RecoveryNode`] writing
    /// `result = "confirmed"`), plus an unwired `cancel` recovery node.
    #[allow(clippy::type_complexity)]
    fn compensation_graph() -> (WarGraph, NodeId, Arc<RecoveryNode>, Arc<RecoveryNode>) {
        let mut graph = WarGraph::new(handler_schema(), EngineLimits::default());
        let prep = NodeId::new("prep");
        let book = NodeId::new("book");
        let confirm = NodeId::new("confirm");
        let cancel = NodeId::new("cancel");
        let confirm_node = RecoveryNode::new(field("result"), serde_json::json!("confirmed"));
        let cancel_node = RecoveryNode::new(field("recovered"), serde_json::json!("cancelled"));
        graph.add_node(
            prep.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("result"),
                serde_json::json!("pre"),
            )),
        );
        graph.add_node(book.clone(), paladin_node("book", &field("result")));
        graph.add_node(confirm.clone(), NodeSpec::Function(confirm_node.clone()));
        graph.add_node(cancel.clone(), NodeSpec::Function(cancel_node.clone()));
        graph.add_edge(EdgeSpec {
            from: prep,
            to: book.clone(),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: book.clone(),
            to: confirm,
            condition: None,
        });
        graph.add_entry(NodeId::new("prep"));
        graph.set_aegis(book.clone(), custom_aegis("compensate"));
        (graph, book, confirm_node, cancel_node)
    }

    /// D-13, T-25-50: a registered handler observes the structured
    /// `NodeError` (node_id / attempt / transience of the failure) and the
    /// pre-failure Battlefield -- the state `book` itself saw, holding
    /// `prep`'s merged write.
    #[tokio::test]
    async fn a_custom_handler_receives_the_structured_error_and_the_battlefield() {
        let (graph, book, _confirm, _cancel) = compensation_graph();
        let handler = RecordingErrorHandler::replying(StateDelta::new().into());
        let registries = registries_with("compensate", handler.clone());
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("custom-receives").unwrap();

        let outcome = run_handled(&graph, thread, &store, &port, &registries).await;
        completed_state(outcome);

        assert_eq!(handler.invocation_count(), 1);
        let seen = handler.seen();
        let (err, state) = &seen[0];
        assert_eq!(err.node_id, book);
        assert_eq!(err.attempt, 1);
        assert_eq!(err.transience, Transience::Permanent);
        assert!(
            matches!(&err.source, NodeErrorSource::Paladin { message, .. } if message.contains("card declined"))
        );
        assert_eq!(
            state.get_raw(&field("result")),
            Some(&serde_json::json!("pre")),
            "the handler sees the pre-failure snapshot, prep's write included"
        );
        assert!(
            state.get_raw(&field("recovered")).is_none(),
            "and nothing from this superstep's merge"
        );
    }

    /// FT-FR-13: `NextStep::Edges` from a handler merges its delta and
    /// continues on the failed node's static edges.
    #[tokio::test]
    async fn a_custom_handler_returning_edges_contributes_its_delta() {
        let (graph, book, confirm, cancel) = compensation_graph();
        let handler = RecordingErrorHandler::replying(
            delta_with("recovered", serde_json::json!("compensated")).into(),
        );
        let registries = registries_with("compensate", handler.clone());
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("custom-edges").unwrap();

        let outcome = run_handled(&graph, thread.clone(), &store, &port, &registries).await;
        let final_state = completed_state(outcome);

        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("compensated"))
        );
        assert!(confirm.ran(), "static edges fired");
        assert!(!cancel.ran());
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
    }

    /// FT-FR-13: `NextStep::Goto(target)` from a handler places `target` in
    /// the next Vanguard, replacing the failed node's static successors.
    #[tokio::test]
    async fn a_custom_handler_returning_goto_places_its_target() {
        let (graph, book, confirm, cancel) = compensation_graph();
        let handler = RecordingErrorHandler::replying(Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("cancel")]),
        });
        let registries = registries_with("compensate", handler.clone());
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("custom-goto").unwrap();

        let outcome = run_handled(&graph, thread.clone(), &store, &port, &registries).await;
        let final_state = completed_state(outcome);

        assert!(cancel.ran(), "the Goto target ran");
        assert!(!confirm.ran(), "the static successor was replaced");
        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("cancelled"))
        );
        let mut waypoints = store.saved_waypoints(&thread).await;
        waypoints.reverse();
        // superstep 1: prep; superstep 2: book (failed, handled); 3: cancel
        assert_eq!(waypoints[1].vanguard, vec![NodeId::new("cancel")]);
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
    }

    /// FT-FR-13: `NextStep::End` from a handler completes the run after
    /// this superstep's merge, with the handler's delta merged.
    #[tokio::test]
    async fn a_custom_handler_returning_end_completes_the_run() {
        let (graph, _book, confirm, cancel) = compensation_graph();
        let handler = RecordingErrorHandler::replying(Directive {
            delta: delta_with("recovered", serde_json::json!("ended")),
            next: NextStep::End,
        });
        let registries = registries_with("compensate", handler.clone());
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("custom-end").unwrap();

        let outcome = run_handled(&graph, thread.clone(), &store, &port, &registries).await;
        let final_state = completed_state(outcome);

        assert_eq!(
            final_state.get_raw(&field("recovered")),
            Some(&serde_json::json!("ended"))
        );
        assert!(!confirm.ran(), "End: nothing after this superstep");
        assert!(!cancel.ran());
        let waypoints = store.saved_waypoints(&thread).await;
        assert_eq!(waypoints.len(), 2, "prep, then the ending superstep");
        assert_eq!(waypoints[0].status, WaypointStatus::Completed);
    }

    /// D-13: `Err(NodeError)` from a handler fails the run carrying the
    /// HANDLER's error, not the original -- on both surfaces.
    #[tokio::test]
    async fn a_custom_handler_returning_err_fails_the_run_with_that_error() {
        let (graph, book, confirm, cancel) = compensation_graph();
        let handler_error = NodeError {
            node_id: book.clone(),
            attempt: 1,
            transience: Transience::Permanent,
            source: NodeErrorSource::Function {
                message: "compensation ledger unreachable".to_string(),
            },
        };
        let handler = RecordingErrorHandler::erroring(handler_error.clone());
        let registries = registries_with("compensate", handler.clone());
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("custom-err").unwrap();

        let outcome = run_handled(&graph, thread.clone(), &store, &port, &registries).await;

        assert_eq!(handler.invocation_count(), 1);
        let exposed = outcome
            .node_error()
            .cloned()
            .unwrap_or_else(|| panic!("expected Failed(NodeFailed), got {outcome:?}"));
        assert_eq!(
            exposed, handler_error,
            "the handler's own error, not the original"
        );
        assert!(matches!(exposed.source, NodeErrorSource::Function { .. }));
        assert!(!confirm.ran());
        assert!(!cancel.ran());
        let waypoints = store.saved_waypoints(&thread).await;
        match &waypoints[0].status {
            WaypointStatus::Failed {
                failed_node,
                node_error,
                ..
            } => {
                assert_eq!(failed_node, &book);
                assert_eq!(node_error.as_ref(), Some(&handler_error));
            }
            other => panic!("expected a Failed Waypoint, got {other:?}"),
        }
        let book_records = records_for(&store, &thread, &book).await;
        assert_eq!(book_records[0].outcome, NodeOutcomeKind::Failed);
    }

    /// FT-FR-15, T-25-47: a node reached by routing increments the SAME
    /// `visit_counts` an ordinary visit does -- no second counter, no
    /// exemption -- observable on the persisted Waypoint.
    #[tokio::test]
    async fn a_handler_routed_visit_counts_against_max_node_visits() {
        let error_field = field("booking_error");
        let (graph, book, cancel, recovery) =
            book_cancel_graph(route_aegis(&NodeId::new("cancel"), &error_field));
        let port: Arc<dyn PaladinPort> = permanent_port();
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("routed-visit-counts").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &port,
            &EngineRegistries::default(),
        )
        .await;
        completed_state(outcome);

        assert!(recovery.ran());
        let waypoints = store.saved_waypoints(&thread).await;
        let final_counts = &waypoints[0].visit_counts;
        assert_eq!(final_counts.get(&book), Some(&1));
        assert_eq!(
            final_counts.get(&cancel),
            Some(&1),
            "the routed target's visit is in the same counter: {final_counts:?}"
        );
    }

    /// FT-FR-15, D-21 (CONTEXT.md loop bound): `a` routes to `b` on
    /// failure, `b` routes to `a`, both always fail, `max_node_visits = 3`
    /// -- the run ends `NodeVisitLimitExceeded` within a bounded number of
    /// supersteps. The `tokio::time::timeout` guard turns a regression
    /// into a loud failure instead of a hung CI job.
    #[tokio::test]
    async fn a_compensation_cycle_terminates_with_the_visit_limit() {
        let mut graph = WarGraph::new(
            handler_schema(),
            EngineLimits {
                max_node_visits: 3,
                ..EngineLimits::default()
            },
        );
        let a = NodeId::new("a");
        let b = NodeId::new("b");
        let a_node = PermanentlyFailingNode::new("a always fails");
        let b_node = PermanentlyFailingNode::new("b always fails");
        graph.add_node(a.clone(), NodeSpec::Function(a_node.clone()));
        graph.add_node(b.clone(), NodeSpec::Function(b_node.clone()));
        graph.add_entry(a.clone());
        graph.set_aegis(a.clone(), route_aegis(&b, &field("booking_error")));
        graph.set_aegis(b.clone(), route_aegis(&a, &field("booking_error")));
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("compensation-cycle").unwrap();

        let outcome = tokio::time::timeout(
            Duration::from_secs(10),
            run_handled(
                &graph,
                thread.clone(),
                &store,
                &no_paladin_port(),
                &EngineRegistries::default(),
            ),
        )
        .await
        .expect("a compensation cycle must terminate, never spin");

        match &outcome {
            RunOutcome::Failed {
                error: EngineError::NodeVisitLimitExceeded { limit, .. },
                ..
            } => assert_eq!(*limit, 3),
            other => panic!("expected Failed(NodeVisitLimitExceeded), got {other:?}"),
        }
        // a(1) b(1) a(2) b(2) a(3 -> trips): at most five supersteps ran.
        let waypoints = store.saved_waypoints(&thread).await;
        assert!(
            waypoints.len() <= 6,
            "bounded: {} waypoints",
            waypoints.len()
        );
        assert!(a_node.run_count() + b_node.run_count() <= 5);
        assert!(matches!(waypoints[0].status, WaypointStatus::Failed { .. }));
    }

    // --- Plan 25-11 Task 1: handlers inside a Muster are delta-only (D-22) --

    use crate::engine::test_support::MusterFailThenSucceedWorker;

    /// `planner -> Muster(a, b, c) -> worker` over an `Append` `results`
    /// field. Task `b` ALWAYS fails (every attempt); `a` and `c` append
    /// their own key at once. `worker_aegis` is set on the template, so the
    /// failing task's final failure is dispatched to that handler.
    fn failing_muster_graph(
        worker_aegis: Aegis,
    ) -> (WarGraph, NodeId, Arc<MusterFailThenSucceedWorker>) {
        let results = field("results");
        let s = schema(vec![FieldSpec::new(
            results.clone(),
            DispatchRule::Append,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        let worker_node =
            MusterFailThenSucceedWorker::new(results.clone(), [("b", usize::MAX)], None);
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
        graph.set_aegis(worker.clone(), worker_aegis);
        graph.add_entry(planner);
        (graph, worker, worker_node)
    }

    /// The merged `results` list, sorted.
    fn sorted_results(state: &Battlefield) -> Vec<String> {
        let mut out: Vec<String> = state
            .get(&field("results"))
            .expect("results reads")
            .unwrap_or_default();
        out.sort();
        out
    }

    /// The muster superstep's worker records (the superstep whose
    /// completed list carries `worker` entries), in record order.
    async fn worker_records(
        store: &RecordingWaypointStore,
        thread: &ThreadId,
        worker: &NodeId,
    ) -> Vec<NodeExecutionRecord> {
        let waypoints = store.saved_waypoints(thread).await;
        waypoints
            .iter()
            .filter(|w| w.muster_progress.is_none())
            .map(|w| {
                w.completed
                    .iter()
                    .filter(|r| &r.node_id == worker)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .find(|records| !records.is_empty())
            .unwrap_or_default()
    }

    /// D-22: a failing mustered task whose `Custom` handler returns
    /// `NextStep::Edges` with a delta contributes THAT delta as its
    /// aggregation entry -- indistinguishable in shape from a successful
    /// sibling's contribution -- and the aggregation sees the full task
    /// count. The task's own record still reads `Failed` (D-21).
    #[tokio::test]
    async fn a_worker_handler_returning_edges_contributes_its_delta_to_the_aggregation() {
        let (graph, worker, worker_node) = failing_muster_graph(custom_aegis("compensate"));
        let handler = RecordingErrorHandler::replying(Directive {
            delta: delta_with("results", serde_json::json!("b-fallback")),
            next: NextStep::Edges,
        });
        let registries = registries_with("compensate", handler.clone());
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("worker-handler-edges").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &registries,
        )
        .await;
        let final_state = completed_state(outcome);

        assert_eq!(
            handler.invocation_count(),
            1,
            "one failed task, one dispatch"
        );
        assert_eq!(
            worker_node.run_count("b"),
            1,
            "no retry policy: one attempt"
        );
        assert_eq!(
            sorted_results(&final_state),
            vec!["a", "b-fallback", "c"],
            "the handler's delta is task b's contribution; the aggregation sees all three"
        );
        let records = worker_records(&store, &thread, &worker).await;
        assert_eq!(
            records.len(),
            3,
            "the aggregation's task count is unchanged"
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.outcome == NodeOutcomeKind::Failed)
                .count(),
            1,
            "exactly one task (b) records the failure it was"
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.outcome == NodeOutcomeKind::Succeeded)
                .count(),
            2
        );
    }

    /// D-22: a `Custom` handler returning `Goto` from inside a Muster task
    /// is `EngineError::MusterHandlerMustBeDeltaOnly`, naming the worker
    /// template AND the task key so the failing task is identifiable in a
    /// wide fan-out; the run fails rather than guessing an aggregation.
    #[tokio::test]
    async fn a_worker_handler_returning_goto_fails_the_run_with_a_typed_error() {
        let (graph, worker, _worker_node) = failing_muster_graph(custom_aegis("compensate"));
        let handler = RecordingErrorHandler::replying(Directive {
            delta: StateDelta::new(),
            next: NextStep::Goto(vec![NodeId::new("planner")]),
        });
        let registries = registries_with("compensate", handler.clone());
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("worker-handler-goto").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &registries,
        )
        .await;

        match &outcome {
            RunOutcome::Failed {
                error:
                    error @ EngineError::MusterHandlerMustBeDeltaOnly {
                        node,
                        task_key,
                        returned,
                    },
                ..
            } => {
                assert_eq!(node, &worker);
                assert_eq!(task_key, "b");
                assert_eq!(returned, "Goto");
                let text = error.to_string();
                assert!(
                    text.contains("worker") && text.contains("`b`") && text.contains("aggregator"),
                    "names the template, the task key and the alternative: {text}"
                );
            }
            other => panic!("expected Failed(MusterHandlerMustBeDeltaOnly), got {other:?}"),
        }
        let waypoints = store.saved_waypoints(&thread).await;
        assert!(
            matches!(waypoints[0].status, WaypointStatus::Failed { .. }),
            "the failure is durable"
        );
    }

    /// D-22: `End`, `Parley` and `Muster` from a worker-task handler are the
    /// same typed error as `Goto` -- every non-`Edges` arm is control flow
    /// out of a single task.
    #[tokio::test]
    async fn a_worker_handler_returning_end_or_parley_or_muster_is_the_same_typed_error() {
        let worker_id = NodeId::new("worker");
        let arms: Vec<(&str, NextStep)> = vec![
            ("End", NextStep::End),
            (
                "Parley",
                NextStep::Parley(ParleyRequest {
                    parley_id: ParleyId::new(),
                    node_id: worker_id.clone(),
                    kind: ParleyKind::Approval,
                    prompt: "retry task b?".to_string(),
                    payload: serde_json::json!({}),
                    choices: None,
                    expires_at: None,
                    created_at: Utc::now(),
                    on_expire: OnExpire::FailRun,
                }),
            ),
            (
                "Muster",
                NextStep::Muster(vec![muster_task(&worker_id, serde_json::json!("b2"), "b2")]),
            ),
        ];
        for (name, next) in arms {
            let (graph, worker, _worker_node) = failing_muster_graph(custom_aegis("compensate"));
            let handler = RecordingErrorHandler::replying(Directive {
                delta: StateDelta::new(),
                next,
            });
            let registries = registries_with("compensate", handler.clone());
            let store = RecordingWaypointStore::new();
            let thread = ThreadId::new(format!("worker-handler-{}", name.to_lowercase())).unwrap();

            let outcome = run_handled(
                &graph,
                thread.clone(),
                &store,
                &no_paladin_port(),
                &registries,
            )
            .await;

            match &outcome {
                RunOutcome::Failed {
                    error:
                        EngineError::MusterHandlerMustBeDeltaOnly {
                            node,
                            task_key,
                            returned,
                        },
                    ..
                } => {
                    assert_eq!(node, &worker, "{name}");
                    assert_eq!(task_key, "b", "{name}");
                    assert_eq!(returned, name);
                }
                other => {
                    panic!("{name}: expected Failed(MusterHandlerMustBeDeltaOnly), got {other:?}")
                }
            }
            // A handler-raised Parley inside a Muster never suspends: no
            // AwaitingInput Waypoint is written for it (D-22 over D-23).
            let waypoints = store.saved_waypoints(&thread).await;
            assert!(
                waypoints
                    .iter()
                    .all(|w| !matches!(w.status, WaypointStatus::AwaitingInput { .. })),
                "{name}: no suspension"
            );
        }
    }

    /// D-22: an `Absorb`ed mustered task contributes its fallback delta and
    /// the aggregation's task count is unchanged.
    #[tokio::test]
    async fn an_absorbed_worker_task_still_appears_in_the_aggregation() {
        let (graph, worker, worker_node) = failing_muster_graph(absorb_aegis(delta_with(
            "results",
            serde_json::json!("b-absorbed"),
        )));
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("worker-absorb").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &EngineRegistries::default(),
        )
        .await;
        let final_state = completed_state(outcome);

        assert_eq!(worker_node.run_count("b"), 1);
        assert_eq!(
            sorted_results(&final_state),
            vec!["a", "b-absorbed", "c"],
            "the fallback delta is task b's contribution"
        );
        let records = worker_records(&store, &thread, &worker).await;
        assert_eq!(
            records.len(),
            3,
            "the aggregation's task count is unchanged"
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.outcome == NodeOutcomeKind::Failed)
                .count(),
            1
        );
    }

    // --- Plan 25-11 Task 2: a Custom handler may Parley (D-23) -------------

    use crate::engine::WarEngine;
    use crate::engine::test_support::ParleyObservingNode;
    use paladin_core::platform::container::parley::ParleyResponse;

    /// The scripted request the `ask-human` handler raises: `node_id` is
    /// deliberately left blank to prove the engine re-stamps it from the
    /// failed node (HITL-01), exactly as for a node-raised parley.
    fn approval_request(parley_id: ParleyId) -> ParleyRequest {
        ParleyRequest {
            parley_id,
            node_id: NodeId::new(""),
            kind: ParleyKind::Approval,
            prompt: "payment failed -- approve a manual retry?".to_string(),
            payload: serde_json::json!({ "amount": 42 }),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    fn approval(parley_id: ParleyId, value: serde_json::Value) -> ParleyResponse {
        ParleyResponse {
            parley_id,
            kind: ParleyKind::Approval,
            prompt: String::new(),
            value,
            responded_by: Some("tester".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        }
    }

    /// `payment` (a [`ParleyObservingNode`], entry, `aegis` with
    /// `Custom("ask-human")`) beside `peer` (entry, writes `peer_field =
    /// "ran"`), over `result`/`peer_field` (`LastWrite`).
    fn payment_graph(
        payment_node: Arc<ParleyObservingNode>,
        aegis: Aegis,
    ) -> (WarGraph, NodeId, NodeId) {
        let s = schema(vec![
            FieldSpec::new(field("result"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(field("peer_field"), DispatchRule::LastWrite, None, false),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let payment = NodeId::new("payment");
        let peer = NodeId::new("peer");
        graph.add_node(payment.clone(), NodeSpec::Function(payment_node));
        graph.add_node(
            peer.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("peer_field"),
                serde_json::json!("ran"),
            )),
        );
        graph.add_entry(payment.clone());
        graph.add_entry(peer.clone());
        graph.set_aegis(payment.clone(), aegis);
        (graph, payment, peer)
    }

    /// A real `WarEngine` over `store` with the `ask-human` handler
    /// registered -- the tests below need `resume_with`, which only the
    /// engine exposes.
    fn parley_engine(
        store: Arc<RecordingWaypointStore>,
        handler: Arc<RecordingErrorHandler>,
    ) -> WarEngine<RecordingWaypointStore> {
        WarEngine::new(no_paladin_port(), store).with_error_handler("ask-human", handler)
    }

    fn custom_aegis_with_retry(name: &str, max_attempts: u32) -> Aegis {
        Aegis {
            on_error: Some(ErrorHandlerSpec::Custom(name.to_string())),
            ..retrying_aegis(max_attempts)
        }
    }

    /// D-23, T-25-54: a handler returning `NextStep::Parley` suspends the
    /// run through the EXISTING HITL-01 path -- `RunOutcome::AwaitingInput`
    /// carrying the handler's request (re-stamped with the failed node's
    /// id), exactly one persisted `AwaitingInput` Waypoint for the
    /// superstep whose `vanguard` is exactly the failed node, and the
    /// node's record reading `Failed` (it DID fail; the handler asked).
    #[tokio::test]
    async fn a_handler_raised_parley_suspends_the_run() {
        let parley_id = ParleyId::new();
        let node = ParleyObservingNode::new(field("result"), "card declined");
        let (graph, payment, _peer) = payment_graph(node.clone(), custom_aegis("ask-human"));
        let handler = RecordingErrorHandler::parleying(approval_request(parley_id));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = parley_engine(store.clone(), handler.clone());
        let thread = ThreadId::new("handler-parley-suspends").unwrap();

        let outcome = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start suspends, never errors");

        match &outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_id, "the handler's own request");
                assert_eq!(
                    parleys[0].node_id, payment,
                    "re-stamped with the failed node"
                );
                assert_eq!(
                    parleys[0].prompt,
                    "payment failed -- approve a manual retry?"
                );
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
        assert_eq!(handler.invocation_count(), 1);
        assert_eq!(
            node.runs().len(),
            1,
            "no retry policy: one attempt raised the parley"
        );

        let waypoints = store.saved_waypoints(&thread).await;
        let awaiting: Vec<&Waypoint> = waypoints
            .iter()
            .filter(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. }))
            .collect();
        assert_eq!(
            awaiting.len(),
            1,
            "exactly one AwaitingInput Waypoint is persisted"
        );
        match &awaiting[0].status {
            WaypointStatus::AwaitingInput { parleys, responses } => {
                assert_eq!(parleys.len(), 1);
                assert_eq!(parleys[0].parley_id, parley_id);
                assert_eq!(parleys[0].node_id, payment);
                assert!(responses.is_empty());
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
        assert_eq!(
            awaiting[0].vanguard,
            vec![payment.clone()],
            "the persisted vanguard is exactly the parleying node (D-02)"
        );
        let record = awaiting[0]
            .completed
            .iter()
            .find(|r| r.node_id == payment)
            .expect("payment's record");
        assert_eq!(
            record.outcome,
            NodeOutcomeKind::Failed,
            "a handler-compensated failure records the failure it was (D-21)"
        );
    }

    /// D-23 + Phase 24 D-07/D-08: after `resume_with`, the failed node
    /// re-runs as a FRESH attempt 1 with `ctx.parley_response()` carrying
    /// the submitted value, and the run completes with that value written.
    #[tokio::test]
    async fn the_post_resume_rerun_is_a_fresh_attempt_one() {
        let parley_id = ParleyId::new();
        let node = ParleyObservingNode::new(field("result"), "card declined");
        let (graph, _payment, _peer) =
            payment_graph(node.clone(), custom_aegis_with_retry("ask-human", 3));
        let handler = RecordingErrorHandler::parleying(approval_request(parley_id));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = parley_engine(store.clone(), handler.clone());
        let thread = ThreadId::new("handler-parley-fresh-attempt").unwrap();

        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start suspends");
        assert!(matches!(suspended, RunOutcome::AwaitingInput { .. }));
        assert_eq!(
            node.observed_attempts(),
            vec![1, 2, 3],
            "the retry budget was spent BEFORE the handler asked"
        );

        let resumed = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval(parley_id, serde_json::json!(true))],
            )
            .await
            .expect("resume completes the run");
        let final_state = completed_state(resumed);

        let runs = node.runs();
        assert_eq!(
            runs.len(),
            4,
            "three raising attempts, then ONE answered re-run"
        );
        assert_eq!(runs[3].0, 1, "the post-resume re-run is a fresh attempt 1");
        assert_eq!(
            runs[3].1,
            Some(serde_json::json!(true)),
            "ctx.parley_response() carries the submitted value"
        );
        assert!(
            runs[..3].iter().all(|(_, answer)| answer.is_none()),
            "no raising attempt ever saw a response"
        );
        assert_eq!(
            final_state.get_raw(&field("result")),
            Some(&serde_json::json!(true))
        );
        assert_eq!(
            handler.invocation_count(),
            1,
            "the answered re-run succeeded"
        );
    }

    /// D-23, T-25-55, FT-FR-05: a `Parley` is not an attempt failure. The
    /// suspension advances no retry counter: a node that fails AGAIN after
    /// resume has its full `max_attempts` budget, reaching the handler a
    /// second time only after spending all of it.
    #[tokio::test]
    async fn a_handler_raised_parley_does_not_consume_the_retry_budget() {
        let parley_id = ParleyId::new();
        let node = ParleyObservingNode::always_failing(field("result"), "card declined");
        let (graph, payment, _peer) =
            payment_graph(node.clone(), custom_aegis_with_retry("ask-human", 3));
        let handler = RecordingErrorHandler::parleying(approval_request(parley_id));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = parley_engine(store.clone(), handler.clone());
        let thread = ThreadId::new("handler-parley-budget").unwrap();

        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start suspends");
        assert!(matches!(suspended, RunOutcome::AwaitingInput { .. }));
        assert_eq!(node.observed_attempts(), vec![1, 2, 3]);

        let resumed = engine
            .resume_with(
                &graph,
                thread.clone(),
                vec![approval(parley_id, serde_json::json!(true))],
            )
            .await
            .expect("resume suspends again, never errors");
        assert!(
            matches!(resumed, RunOutcome::AwaitingInput { .. }),
            "still failing: the handler asks again, got {resumed:?}"
        );
        assert_eq!(
            node.observed_attempts(),
            vec![1, 2, 3, 1, 2, 3],
            "the post-resume run has its FULL attempt budget again"
        );
        assert_eq!(
            handler.invocation_count(),
            2,
            "one dispatch per exhausted budget"
        );

        // The second AwaitingInput Waypoint's record shows a full budget
        // spent this time too: attempt 3 final, two failed attempts before.
        let waypoints = store.saved_waypoints(&thread).await;
        let latest = &waypoints[0];
        assert!(matches!(
            latest.status,
            WaypointStatus::AwaitingInput { .. }
        ));
        let record = latest
            .completed
            .iter()
            .find(|r| r.node_id == payment)
            .expect("payment's record");
        assert_eq!(record.attempt, 3);
        assert_eq!(record.attempts.len(), 2);
    }

    /// D-23: peers of the suspending superstep merge their deltas exactly as
    /// they do for a node-raised parley -- the persisted Battlefield carries
    /// the peer's write, the peer's record reads `Succeeded`, and the
    /// vanguard is still exactly the parleying node.
    #[tokio::test]
    async fn the_suspending_supersteps_peers_merge_normally() {
        let parley_id = ParleyId::new();
        let node = ParleyObservingNode::new(field("result"), "card declined");
        let (graph, payment, peer) = payment_graph(node.clone(), custom_aegis("ask-human"));
        let handler = RecordingErrorHandler::parleying(approval_request(parley_id));
        let store = Arc::new(RecordingWaypointStore::new());
        let engine = parley_engine(store.clone(), handler.clone());
        let thread = ThreadId::new("handler-parley-peers").unwrap();

        let suspended = engine
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start suspends");
        assert!(matches!(suspended, RunOutcome::AwaitingInput { .. }));

        let waypoints = store.saved_waypoints(&thread).await;
        let awaiting = waypoints
            .iter()
            .find(|w| matches!(w.status, WaypointStatus::AwaitingInput { .. }))
            .expect("an AwaitingInput waypoint must exist");
        assert_eq!(
            awaiting
                .battlefield
                .get::<String>(&field("peer_field"))
                .unwrap(),
            Some("ran".to_string()),
            "the peer's delta merges normally"
        );
        let peer_record = awaiting
            .completed
            .iter()
            .find(|r| r.node_id == peer)
            .expect("peer's record");
        assert_eq!(peer_record.outcome, NodeOutcomeKind::Succeeded);
        assert_eq!(awaiting.vanguard, vec![payment.clone()]);

        // And the resumed run completes with both writes intact.
        let resumed = engine
            .resume_with(
                &graph,
                thread,
                vec![approval(parley_id, serde_json::json!(true))],
            )
            .await
            .expect("resume completes");
        let final_state = completed_state(resumed);
        assert_eq!(
            final_state.get_raw(&field("peer_field")),
            Some(&serde_json::json!("ran"))
        );
        assert_eq!(
            final_state.get_raw(&field("result")),
            Some(&serde_json::json!(true))
        );
    }

    /// D-22 over D-23: inside a Muster the delta-only rule wins -- a
    /// worker-template handler returning `Parley` is
    /// `MusterHandlerMustBeDeltaOnly`, never a suspension.
    #[tokio::test]
    async fn a_handler_raised_parley_inside_a_muster_is_rejected() {
        let (graph, worker, _worker_node) = failing_muster_graph(custom_aegis("ask-human"));
        let handler = RecordingErrorHandler::parleying(approval_request(ParleyId::new()));
        let registries = registries_with("ask-human", handler.clone());
        let store = RecordingWaypointStore::new();
        let thread = ThreadId::new("worker-handler-parley").unwrap();

        let outcome = run_handled(
            &graph,
            thread.clone(),
            &store,
            &no_paladin_port(),
            &registries,
        )
        .await;

        match &outcome {
            RunOutcome::Failed {
                error:
                    EngineError::MusterHandlerMustBeDeltaOnly {
                        node,
                        task_key,
                        returned,
                    },
                ..
            } => {
                assert_eq!(node, &worker);
                assert_eq!(task_key, "b");
                assert_eq!(returned, "Parley");
            }
            other => panic!("expected Failed(MusterHandlerMustBeDeltaOnly), got {other:?}"),
        }
        assert_eq!(handler.invocation_count(), 1);
        let waypoints = store.saved_waypoints(&thread).await;
        assert!(
            waypoints
                .iter()
                .all(|w| !matches!(w.status, WaypointStatus::AwaitingInput { .. })),
            "a rejected handler parley never suspends"
        );
        assert!(matches!(waypoints[0].status, WaypointStatus::Failed { .. }));
    }

    // --- Plan 25-13 Tasks 2/3: the node cache's hit/miss path and its
    //     correctness under time, change and backend failure (FT-FR-18,
    //     FT-FR-20, D-28, D-29) ---------------------------------------------

    use crate::engine::test_support::{
        RecordingInterceptor, RecordingNodeCache, RecordingTraceSink,
    };
    use paladin_core::platform::container::aegis::{CacheKeySpec, CachePolicy};

    fn cache_aegis(ttl: Duration) -> Aegis {
        Aegis {
            cache: Some(CachePolicy {
                ttl,
                key: CacheKeySpec::Default,
            }),
            ..Aegis::default()
        }
    }

    /// A cache policy AND a retry policy on the same node, so a hit's
    /// zero-attempt accounting is observable against a budget that exists.
    fn cached_retrying_aegis(ttl: Duration, max_attempts: u32) -> Aegis {
        Aegis {
            cache: Some(CachePolicy {
                ttl,
                key: CacheKeySpec::Default,
            }),
            retry: Some(RetryPolicy {
                max_attempts,
                initial_interval: Duration::from_millis(1),
                jitter: false,
                retry_on:
                    paladin_core::platform::container::aegis::RetryPredicate::TransientAndUnknown,
                ..RetryPolicy::default()
            }),
            ..Aegis::default()
        }
    }

    /// The one-node graph every cache test starts from: `node` under
    /// `aegis`, entry, writing into the `result` field.
    fn cached_graph(node: Arc<dyn StateNode>, aegis: Aegis) -> (WarGraph, NodeId) {
        let s = schema(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let id = NodeId::new("cached");
        graph.add_node(id.clone(), NodeSpec::Function(node));
        graph.add_entry(id.clone());
        graph.set_aegis(id.clone(), aegis);
        (graph, id)
    }

    fn cached_engine(
        port: Arc<dyn PaladinPort>,
        store: Arc<RecordingWaypointStore>,
        cache: Arc<RecordingNodeCache>,
    ) -> WarEngine<RecordingWaypointStore> {
        WarEngine::new(port, store).with_node_cache(cache)
    }

    async fn start_thread(
        engine: &WarEngine<RecordingWaypointStore>,
        graph: &WarGraph,
        thread: &str,
    ) -> RunOutcome {
        engine
            .start(graph, ThreadId::new(thread).unwrap(), StateDelta::new())
            .await
            .unwrap()
    }

    fn completed_result(outcome: &RunOutcome) -> Option<String> {
        match outcome {
            RunOutcome::Completed { final_state, .. } => {
                final_state.get::<String>(&field("result")).unwrap()
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    /// Every `NodeExecutionRecord` for `node` on `thread`, across every
    /// persisted Waypoint (newest first) -- a `&str`-thread twin of the
    /// module's own `records_for`, since every cache test names its threads
    /// inline.
    async fn cache_records_for(
        store: &RecordingWaypointStore,
        thread: &str,
        node: &NodeId,
    ) -> Vec<NodeExecutionRecord> {
        store
            .saved_waypoints(&ThreadId::new(thread).unwrap())
            .await
            .iter()
            .flat_map(|w| w.completed.iter())
            .filter(|r| &r.node_id == node)
            .cloned()
            .collect()
    }

    /// Task 2, Test 7: a pre-populated cache (by a prior run of the same
    /// graph) serves the second run with zero executions, the stored delta
    /// merged, and a record reading Succeeded / attempt 1 / cache_hit true.
    #[tokio::test]
    async fn a_hit_merges_the_stored_delta_with_no_execution() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("computed"));
        let (graph, id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        let first = start_thread(&engine, &graph, "hit-1").await;
        assert_eq!(completed_result(&first).as_deref(), Some("computed"));
        assert_eq!(node.run_count(), 1);
        assert_eq!(cache.put_count(), 1);
        let first_record = &cache_records_for(&store, "hit-1", &id).await[0];
        assert!(!first_record.cache_hit, "the populating run is a miss");

        let second = start_thread(&engine, &graph, "hit-2").await;
        assert_eq!(
            completed_result(&second).as_deref(),
            Some("computed"),
            "the stored delta is merged"
        );
        assert_eq!(node.run_count(), 1, "a hit executes nothing");
        assert_eq!(cache.get_count(), 2);
        assert_eq!(cache.put_count(), 1, "a hit stores nothing");
        let records = cache_records_for(&store, "hit-2", &id).await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].outcome, NodeOutcomeKind::Succeeded);
        assert_eq!(records[0].attempt, 1);
        assert!(records[0].cache_hit);
        assert_eq!(records[0].token_count, 0);
    }

    /// Task 2, Test 8: a hit emits exactly `NodeStarted { attempt: 1 }` and
    /// `NodeFinished { attempt: 1, cache_hit: true }` for the node -- and no
    /// other node event.
    #[tokio::test]
    async fn a_hit_emits_node_started_and_finished_with_cache_hit_true() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, id) = cached_graph(node, cache_aegis(Duration::from_secs(60)));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let sink = RecordingTraceSink::new();
        let engine = cached_engine(no_paladin_port(), store, cache).with_trace_sink(sink.clone());

        start_thread(&engine, &graph, "trace-1").await;
        start_thread(&engine, &graph, "trace-2").await;

        // Give the background trace consumer a chance to drain.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let hit_thread = ThreadId::new("trace-2").unwrap();
        let node_events: Vec<(&'static str, u32, Option<bool>)> = sink
            .events()
            .await
            .iter()
            .filter_map(|event| match event {
                TraceEvent::NodeStarted {
                    thread_id,
                    node_id,
                    attempt,
                    ..
                } if thread_id == &hit_thread && node_id == &id => {
                    Some(("NodeStarted", *attempt, None))
                }
                TraceEvent::NodeFinished {
                    thread_id,
                    node_id,
                    attempt,
                    cache_hit,
                    ..
                } if thread_id == &hit_thread && node_id == &id => {
                    Some(("NodeFinished", *attempt, Some(*cache_hit)))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            node_events,
            vec![("NodeStarted", 1, None), ("NodeFinished", 1, Some(true))]
        );
        // And the populating run's own pair was NOT a hit.
        let miss_thread = ThreadId::new("trace-1").unwrap();
        let miss_finished: Vec<bool> = sink
            .events()
            .await
            .iter()
            .filter_map(|event| match event {
                TraceEvent::NodeFinished {
                    thread_id,
                    cache_hit,
                    ..
                } if thread_id == &miss_thread => Some(*cache_hit),
                _ => None,
            })
            .collect();
        assert_eq!(miss_finished, vec![false]);
    }

    /// Task 2, Test 9: a miss executes exactly once and issues exactly one
    /// `put` carrying the `CachePolicy`'s TTL and the merged delta.
    #[tokio::test]
    async fn a_miss_executes_and_stores_the_successful_delta_with_the_ttl() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("stored"));
        let ttl = Duration::from_secs(1234);
        let (graph, _id) = cached_graph(node.clone(), cache_aegis(ttl));
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            cache.clone(),
        );

        let outcome = start_thread(&engine, &graph, "miss").await;
        assert_eq!(completed_result(&outcome).as_deref(), Some("stored"));
        assert_eq!(node.run_count(), 1);
        assert_eq!(cache.get_count(), 1, "exactly one lookup, before attempt 1");
        let puts = cache.puts();
        assert_eq!(puts.len(), 1, "exactly one put");
        let (key, delta, put_ttl) = &puts[0];
        assert_eq!(*put_ttl, ttl, "the put carries the policy's TTL");
        assert_eq!(
            delta.values.get(&field("result")),
            Some(&serde_json::json!("stored"))
        );
        assert!(key.starts_with(&crate::engine::cache_key::node_prefix(
            &graph.fingerprint(),
            &NodeId::new("cached")
        )));
    }

    /// Task 2, Test 10: a node that fails every attempt issues zero `put`
    /// calls -- no error outcome is ever cached.
    #[tokio::test]
    async fn a_failed_attempt_is_never_stored() {
        let (graph, id) = cached_graph(
            FailingFunctionNode::new("boom"),
            cached_retrying_aegis(Duration::from_secs(60), 3),
        );
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        let outcome = start_thread(&engine, &graph, "always-fails").await;
        assert!(matches!(outcome, RunOutcome::Failed { .. }));
        assert_eq!(cache.get_count(), 1, "looked up once, before attempt 1");
        assert_eq!(cache.put_count(), 0, "a failed attempt is never stored");
        assert!(cache.keys().is_empty());
        let records = cache_records_for(&store, "always-fails", &id).await;
        assert_eq!(records[0].attempt, 3, "every retry ran");
        assert!(!records[0].cache_hit);
    }

    /// Task 2, Test 11: a stored EMPTY delta is a hit that merges nothing --
    /// `cache_hit: true`, no execution, Battlefield unchanged (FT-06 edge
    /// assumption, engine side).
    #[tokio::test]
    async fn an_empty_cached_delta_is_a_hit_that_merges_nothing() {
        let node = CountingFunctionNode::new(|_run, _state| StateDelta::new());
        let (graph, id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        start_thread(&engine, &graph, "empty-1").await;
        assert_eq!(cache.put_count(), 1, "an empty delta IS stored");
        assert!(cache.puts()[0].1.values.is_empty());

        let second = start_thread(&engine, &graph, "empty-2").await;
        assert_eq!(completed_result(&second), None, "nothing merged");
        assert_eq!(node.run_count(), 1, "served from cache, not re-executed");
        let records = cache_records_for(&store, "empty-2", &id).await;
        assert!(records[0].cache_hit);
        assert_eq!(records[0].outcome, NodeOutcomeKind::Succeeded);
    }

    /// FT-FR-20's Function-node half of the `Deny` guarantee: a `StateNode`'s
    /// write set is only knowable from its delta, so a delta touching a
    /// `cache: Deny` field is never stored -- and the next run re-executes.
    #[tokio::test]
    async fn a_function_delta_touching_a_deny_field_is_never_stored() {
        let log = field("log");
        let s = schema(vec![
            FieldSpec::new(field("result"), DispatchRule::LastWrite, None, false),
            FieldSpec::new(log.clone(), DispatchRule::Append, None, false)
                .with_cache(CacheMarker::Deny),
        ]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let id = NodeId::new("appender");
        let node = CountingFunctionNode::fixed(log.clone(), serde_json::json!("line"));
        graph.add_node(id.clone(), NodeSpec::Function(node.clone()));
        graph.add_entry(id.clone());
        graph.set_aegis(id, cache_aegis(Duration::from_secs(60)));
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            cache.clone(),
        );

        let first = start_thread(&engine, &graph, "deny-1").await;
        assert!(matches!(first, RunOutcome::Completed { .. }));
        assert_eq!(
            cache.put_count(),
            0,
            "a Deny-touching delta is never stored"
        );
        start_thread(&engine, &graph, "deny-2").await;
        assert_eq!(node.run_count(), 2, "nothing to hit, so it re-executes");
    }

    /// D-14: the cache is part of the Aegis, which wraps OUTSIDE the
    /// interceptor chain -- so a hit runs no `before`/`after` interceptor
    /// (nothing executes), while the populating miss ran both.
    #[tokio::test]
    async fn a_hit_bypasses_the_interceptor_chain() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, _id) = cached_graph(node, cache_aegis(Duration::from_secs(60)));
        let interceptor = RecordingInterceptor::new();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            RecordingNodeCache::new(),
        )
        .with_interceptors(vec![interceptor.clone()]);

        start_thread(&engine, &graph, "intercept-1").await;
        assert_eq!(interceptor.calls(), vec!["before", "after"]);
        start_thread(&engine, &graph, "intercept-2").await;
        assert_eq!(
            interceptor.calls(),
            vec!["before", "after"],
            "the hit ran no interceptor"
        );
    }

    /// A `Goto`-routed directive is never stored: a `CachedDelta` carries
    /// only a delta, and replaying just the delta would drop the routing.
    #[tokio::test]
    async fn a_routing_directive_is_never_stored() {
        let s = schema(vec![FieldSpec::new(
            field("result"),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(s, EngineLimits::default());
        let router = NodeId::new("router");
        let target = NodeId::new("target");
        let target_id = target.clone();
        graph.add_node(
            router.clone(),
            NodeSpec::Function(CountingFunctionNode::with_directive(move |_run, _state| {
                Directive {
                    delta: StateDelta::new(),
                    next: NextStep::Goto(vec![target_id.clone()]),
                }
            })),
        );
        graph.add_node(
            target.clone(),
            NodeSpec::Function(CountingFunctionNode::fixed(
                field("result"),
                serde_json::json!("routed"),
            )),
        );
        graph.mark_dynamic_target(target);
        graph.add_entry(router.clone());
        graph.set_aegis(router, cache_aegis(Duration::from_secs(60)));
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            cache.clone(),
        );
        let outcome = start_thread(&engine, &graph, "goto").await;
        assert_eq!(completed_result(&outcome).as_deref(), Some("routed"));
        assert_eq!(cache.put_count(), 0, "a Goto directive is never cached");
    }

    // --- Task 3: correctness under time, change and backend failure -------

    /// Task 3, Test 1: under a paused clock, a cached entry whose TTL has
    /// elapsed is a miss and the node executes again -- the call-count node
    /// proves it, with no wall-clock sleep anywhere.
    #[tokio::test(start_paused = true)]
    async fn ttl_expiry_re_executes_the_node() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let ttl = Duration::from_secs(10);
        let (graph, id) = cached_graph(node.clone(), cache_aegis(ttl));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        start_thread(&engine, &graph, "ttl-1").await;
        assert_eq!(node.run_count(), 1);
        // Still live: well inside the TTL, a hit.
        tokio::time::advance(ttl / 2).await;
        start_thread(&engine, &graph, "ttl-2").await;
        assert_eq!(node.run_count(), 1, "inside the TTL is a hit");
        // Past the TTL: a miss, re-executed and re-stored.
        tokio::time::advance(ttl).await;
        start_thread(&engine, &graph, "ttl-3").await;
        assert_eq!(node.run_count(), 2, "an elapsed TTL re-executes");
        assert_eq!(cache.put_count(), 2, "the re-execution re-populates");
        let records = cache_records_for(&store, "ttl-3", &id).await;
        assert!(!records[0].cache_hit);
    }

    /// Task 3, Test 2: an entry read at EXACTLY its `expires_at` is a miss
    /// at the engine layer too -- the backend still serves it (the double's
    /// own clock is untouched), so only the engine's closed-boundary
    /// `is_expired_at` check can produce the re-execution. This pins the
    /// boundary at both layers (plan 25-04's contract case is the backend
    /// half), so the two can never disagree about what `expires_at` means.
    #[tokio::test]
    async fn an_entry_at_exactly_its_expiry_is_a_miss() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(3600)));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        start_thread(&engine, &graph, "boundary-1").await;
        assert_eq!(node.run_count(), 1);
        // Move the stored entry's `expires_at` to NOW: by the time the engine
        // evaluates `is_expired_at(Utc::now())`, `now >= expires_at` holds --
        // the closed boundary -- even though the backend still returns it.
        cache.set_expires_at_for_all(Utc::now());
        start_thread(&engine, &graph, "boundary-2").await;
        assert_eq!(cache.get_count(), 2);
        assert_eq!(node.run_count(), 2, "at exactly expires_at is a miss");
        let records = cache_records_for(&store, "boundary-2", &id).await;
        assert!(!records[0].cache_hit);
        // The control: the same entry one hour out is a hit.
        cache.set_expires_at_for_all(Utc::now() + chrono::Duration::hours(1));
        start_thread(&engine, &graph, "boundary-3").await;
        assert_eq!(node.run_count(), 2, "a live entry is a hit");
    }

    /// Task 3, Test 3: the same graph and input with a changed system
    /// prompt misses and executes -- the graph fingerprint does NOT cover
    /// the prompt (a legitimate operator tuning, ENG-FR-14), so this is the
    /// Paladin-config fingerprint's own guarantee (FT-FR-20).
    #[tokio::test]
    async fn changing_the_system_prompt_re_executes() {
        fn prompt_graph(prompt: &str) -> WarGraph {
            let s = schema(vec![
                FieldSpec::new(
                    field("topic"),
                    DispatchRule::LastWrite,
                    Some(serde_json::json!("rust")),
                    false,
                ),
                FieldSpec::new(field("result"), DispatchRule::LastWrite, None, false),
            ]);
            let mut graph = WarGraph::new(s, EngineLimits::default());
            let id = NodeId::new("writer");
            let data = paladin_core::platform::container::paladin::PaladinData {
                name: "writer".to_string(),
                system_prompt: prompt.to_string(),
                ..Default::default()
            };
            let paladin =
                paladin_core::base::entity::node::Node::new(data, Some("writer".to_string()));
            graph.add_node(
                id.clone(),
                NodeSpec::paladin(
                    paladin,
                    InputMapping::new("write about {topic}"),
                    field("result"),
                ),
            );
            graph.add_entry(id.clone());
            graph.set_aegis(id, cache_aegis(Duration::from_secs(60)));
            graph
        }
        let port = Arc::new(RecordingPaladinPort::new());
        port.set_output("writer", "an essay");
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(port.clone(), Arc::new(RecordingWaypointStore::new()), cache);

        let terse = prompt_graph("be terse");
        let outcome = start_thread(&engine, &terse, "prompt-1").await;
        assert_eq!(completed_result(&outcome).as_deref(), Some("an essay"));
        assert_eq!(port.call_count(), 1);
        start_thread(&engine, &terse, "prompt-2").await;
        assert_eq!(port.call_count(), 1, "same prompt, same input: a hit");

        let verbose = prompt_graph("be verbose");
        assert_eq!(
            terse.fingerprint(),
            verbose.fingerprint(),
            "the prompt is deliberately outside the graph fingerprint"
        );
        let outcome = start_thread(&engine, &verbose, "prompt-3").await;
        assert_eq!(completed_result(&outcome).as_deref(), Some("an essay"));
        assert_eq!(
            port.call_count(),
            2,
            "a changed prompt misses and re-executes"
        );
    }

    /// Task 3, Test 4: a structural graph edit changes the fingerprint and
    /// therefore the key, so the same node executes again.
    #[tokio::test]
    async fn changing_the_graph_re_executes() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, _id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            cache.clone(),
        );
        start_thread(&engine, &graph, "graph-1").await;
        start_thread(&engine, &graph, "graph-2").await;
        assert_eq!(node.run_count(), 1, "the unchanged graph hits");

        // The SAME node instance in a structurally different graph.
        let (mut edited, _) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        edited.add_node(
            NodeId::new("extra"),
            NodeSpec::Function(CountingFunctionNode::new(|_, _| StateDelta::new())),
        );
        edited.add_entry(NodeId::new("extra"));
        assert_ne!(graph.fingerprint(), edited.fingerprint());
        start_thread(&engine, &edited, "graph-3").await;
        assert_eq!(node.run_count(), 2, "a graph edit invalidates naturally");
        let keys = cache.keys();
        assert_eq!(keys.len(), 2, "two graphs, two entries -- never one shared");
        assert!(keys[0].as_str() != keys[1].as_str());
    }

    /// Task 3, Test 5: a backend whose `put` always errors leaves the run
    /// `Completed` with the node's own result intact -- the failure is
    /// logged only (D-29).
    #[tokio::test]
    async fn a_put_failure_leaves_the_run_completed() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("intact"));
        let (graph, id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        cache.fail_puts();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        let outcome = start_thread(&engine, &graph, "put-fails").await;
        assert!(matches!(outcome, RunOutcome::Completed { .. }));
        assert_eq!(completed_result(&outcome).as_deref(), Some("intact"));
        assert_eq!(cache.put_count(), 1, "the put was attempted");
        assert!(cache.keys().is_empty(), "and stored nothing");
        let records = cache_records_for(&store, "put-fails", &id).await;
        assert_eq!(records[0].outcome, NodeOutcomeKind::Succeeded);
        assert!(!records[0].cache_hit);
    }

    /// Task 3, Test 6: a backend whose `get` always errors is a MISS -- the
    /// node executes and the run completes, every time.
    #[tokio::test]
    async fn a_get_failure_is_a_miss_not_an_error() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, _id) = cached_graph(node.clone(), cache_aegis(Duration::from_secs(60)));
        let cache = RecordingNodeCache::new();
        cache.fail_gets();
        let engine = cached_engine(
            no_paladin_port(),
            Arc::new(RecordingWaypointStore::new()),
            cache.clone(),
        );

        let first = start_thread(&engine, &graph, "get-fails-1").await;
        assert!(matches!(first, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 1);
        assert_eq!(cache.get_count(), 1);
        assert_eq!(cache.put_count(), 1, "the successful attempt still stores");
        let second = start_thread(&engine, &graph, "get-fails-2").await;
        assert!(matches!(second, RunOutcome::Completed { .. }));
        assert_eq!(node.run_count(), 2, "every failed get is a miss");
    }

    /// Task 3, Test 7: two mustered tasks over the same worker template with
    /// different payloads do not share a cache entry -- and a later run over
    /// the same fan-out serves BOTH from cache.
    #[tokio::test]
    async fn a_cached_node_inside_a_muster_keys_per_task() {
        let results = field("results");
        let s = schema(vec![FieldSpec::new(
            results.clone(),
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
                ]),
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
        graph.add_worker_template(worker.clone(), NodeSpec::Function(worker_node.clone()));
        graph.set_aegis(worker.clone(), cache_aegis(Duration::from_secs(60)));
        graph.add_entry(planner);
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        let first = start_thread(&engine, &graph, "muster-1").await;
        match &first {
            RunOutcome::Completed { final_state, .. } => assert_eq!(
                final_state.get::<Vec<String>>(&results).unwrap(),
                Some(vec!["a".to_string(), "b".to_string()])
            ),
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(worker_node.run_count(), 2);
        let keys = cache.keys();
        assert_eq!(keys.len(), 2, "one entry per task, never one shared entry");
        let prefix = crate::engine::cache_key::node_prefix(&graph.fingerprint(), &worker);
        assert!(keys.iter().all(|k| k.starts_with(&prefix)));

        let second = start_thread(&engine, &graph, "muster-2").await;
        match &second {
            RunOutcome::Completed { final_state, .. } => assert_eq!(
                final_state.get::<Vec<String>>(&results).unwrap(),
                Some(vec!["a".to_string(), "b".to_string()]),
                "both tasks' deltas served from cache still aggregate in task_key order"
            ),
            other => panic!("expected Completed, got {other:?}"),
        }
        assert_eq!(worker_node.run_count(), 2, "both tasks hit");
        let worker_records = cache_records_for(&store, "muster-2", &worker).await;
        assert!(
            !worker_records.is_empty() && worker_records.iter().all(|r| r.cache_hit),
            "every worker record on the hit run is a hit: {worker_records:?}"
        );
    }

    /// Task 3, Test 8: a hit records `attempt: 1` with an empty `attempts`
    /// list, and no retry machinery runs -- even under a retry policy.
    #[tokio::test(start_paused = true)]
    async fn a_cache_hit_consumes_no_retry_budget() {
        let node = CountingFunctionNode::fixed(field("result"), serde_json::json!("v"));
        let (graph, id) = cached_graph(
            node.clone(),
            cached_retrying_aegis(Duration::from_secs(60), 3),
        );
        let store = Arc::new(RecordingWaypointStore::new());
        let cache = RecordingNodeCache::new();
        let engine = cached_engine(no_paladin_port(), store.clone(), cache.clone());

        start_thread(&engine, &graph, "budget-1").await;
        let before = tokio::time::Instant::now();
        start_thread(&engine, &graph, "budget-2").await;
        assert_eq!(
            before.elapsed(),
            Duration::ZERO,
            "no backoff wait ran: the paused clock never advanced"
        );
        assert_eq!(node.run_count(), 1);
        let records = cache_records_for(&store, "budget-2", &id).await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].attempt, 1);
        assert!(
            records[0].attempts.is_empty(),
            "no failed attempts on a hit"
        );
        assert!(records[0].cache_hit);
    }
}
