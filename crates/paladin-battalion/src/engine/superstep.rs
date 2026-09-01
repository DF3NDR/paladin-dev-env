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
//! Next-Vanguard computation here is the simple "dependencies satisfied"
//! heuristic (an edge whose source ran this superstep and whose condition
//! evaluated true adds its target) — sufficient for linear chains,
//! branching fan-out and self-loops/cycles, which is everything this plan's
//! own tests exercise. Precise join/defer/not-firing semantics for
//! multi-incoming-edge nodes are Plan 22-07's expansion (ENG-FR-06); that
//! plan replaces `compute_next_vanguard` without changing this loop's shape.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use log::warn;
use regex::Regex;
use tokio::sync::Semaphore;

use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{Battlefield, CustomDispatchResolver};
use paladin_core::platform::container::waypoint::{
    NodeExecutionRecord, NodeId, NodeOutcomeKind, ThreadId, Waypoint, WaypointId, WaypointStatus,
};
use paladin_ports::output::waypoint_port::WaypointPort;

use crate::engine::graph::{NodeSpec, WarGraph};
use crate::engine::node::NodeError;
use crate::engine::{EngineError, RunOutcome, WaypointDurability};

/// Run the superstep loop starting from `vanguard` at `superstep_number`,
/// over `battlefield`, persisting through `waypoint_port` under
/// `durability`, bounding per-superstep concurrency at `parallelism` (or the
/// Vanguard's own size when `None`, per D-12).
///
/// `parent_waypoint_id` chains the first Waypoint this call writes to the
/// caller-supplied lineage (`None` for a fresh `start`, `Some(id)` when a
/// later plan re-enters this loop from `resume`).
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
        persist_waypoint(waypoint_port, durability, &waypoint).await?;
        return Ok(RunOutcome::Completed {
            final_state: battlefield,
            waypoint: waypoint.waypoint_id,
        });
    }

    loop {
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
            persist_waypoint(waypoint_port, durability, &waypoint).await?;
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
            persist_waypoint(waypoint_port, durability, &waypoint).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }
        visit_counts = candidate_counts;

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
            let node = match spec {
                NodeSpec::Function(node) => Arc::clone(node),
                NodeSpec::Paladin { .. } => {
                    return Err(EngineError::Node(NodeError(
                        "the superstep loop (Phase 22 Plan 05) only supports Function nodes; \
                         NodeSpec::Paladin execution is Plan 22-08's expansion"
                            .to_string(),
                    )));
                }
            };
            let snap = Arc::clone(&snapshot);
            let sem = Arc::clone(&semaphore);
            let ctx = crate::engine::node::NodeContext {
                node_id: node_id.clone(),
                thread_id: thread.clone(),
                superstep: superstep_number,
            };
            let nid = node_id.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem
                    .acquire_owned()
                    .await
                    .expect("semaphore is never closed");
                let started_at = Utc::now();
                let result = node.run(&snap, &ctx).await;
                let duration_ms = (Utc::now() - started_at).num_milliseconds().max(0) as u64;
                (nid, started_at, duration_ms, result)
            }));
        }

        let mut deltas = Vec::with_capacity(handles.len());
        let mut completed_records = Vec::with_capacity(handles.len());
        let mut node_failure: Option<(NodeId, NodeError)> = None;
        for handle in handles {
            let (node_id, started_at, duration_ms, result) = handle
                .await
                .map_err(|e| EngineError::Node(NodeError(format!("task join error: {e}"))))?;
            match result {
                Ok(delta) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id: None,
                        started_at,
                        duration_ms,
                        token_count: 0,
                        outcome: NodeOutcomeKind::Succeeded,
                        attempt: 1,
                    });
                    deltas.push((node_id, delta));
                }
                Err(e) => {
                    completed_records.push(NodeExecutionRecord {
                        node_id: node_id.clone(),
                        paladin_id: None,
                        started_at,
                        duration_ms,
                        token_count: 0,
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
            persist_waypoint(waypoint_port, durability, &waypoint).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        // --- Merge, only after every node in this superstep has completed
        // (ENG-FR-05: no node observes a peer's delta this superstep).
        deltas.sort_by(|a, b| a.0.cmp(&b.0));
        let ran: Vec<NodeId> = deltas.iter().map(|(id, _)| id.clone()).collect();
        if let Err(e) = battlefield.merge(deltas, superstep_number, registry) {
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
            persist_waypoint(waypoint_port, durability, &waypoint).await?;
            return Ok(RunOutcome::Failed {
                error,
                waypoint: Some(waypoint.waypoint_id),
            });
        }

        let next_vanguard = compute_next_vanguard(graph, &ran, &battlefield)?;
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
        persist_waypoint(waypoint_port, durability, &waypoint).await?;

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

/// Compute the Vanguard for the superstep after the one that just ran
/// `ran` (the `NodeId`s that executed successfully this superstep),
/// evaluating each outgoing edge's condition against the POST-merge
/// `battlefield`. An edge with no condition always fires. Iterates
/// `graph.edges()` in the graph's stable insertion order (ENG-FR-04) and
/// de-duplicates targets reached by more than one firing edge.
fn compute_next_vanguard(
    graph: &WarGraph,
    ran: &[NodeId],
    battlefield: &Battlefield,
) -> Result<Vec<NodeId>, EngineError> {
    let mut next = Vec::new();
    let mut seen = HashSet::new();
    for edge in graph.edges() {
        if !ran.contains(&edge.from) {
            continue;
        }
        let fires = match &edge.condition {
            None => true,
            Some(condition) => evaluate_edge_condition(condition, battlefield)?,
        };
        if fires && seen.insert(edge.to.clone()) {
            next.push(edge.to.clone());
        }
    }
    Ok(next)
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
/// warning and the caller proceeds as if the save had succeeded.
async fn persist_waypoint<W: WaypointPort>(
    waypoint_port: &W,
    durability: WaypointDurability,
    waypoint: &Waypoint,
) -> Result<(), EngineError> {
    if let Err(source) = waypoint_port.save(waypoint).await {
        match durability {
            WaypointDurability::Strict => return Err(EngineError::WaypointWrite { source }),
            WaypointDurability::BestEffort => {
                warn!(
                    "waypoint save failed under BestEffort durability for thread {}: {source}",
                    waypoint.thread_id
                );
            }
        }
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
        ConcurrencyTrackingNode, CountingFunctionNode, FailingFunctionNode, RecordingWaypointStore,
    };

    fn field(name: &str) -> FieldName {
        FieldName::new(name).unwrap()
    }

    fn schema(fields: Vec<FieldSpec>) -> BattlefieldSchema {
        BattlefieldSchema::new(fields)
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
}
