//! `RunInspectorService` -- the facade [`RunInspectorPort`] implementation
//! (D-24, OBS-03 / OBS-FR-10): builds an [`InspectorView`] on top of the
//! 28-10 execution overlay, Waypoint history, and (when persisted) exact
//! trace records.
//!
//! Lives in the facade, never in `paladin-web`, for the same reason
//! `events.rs`/`worker.rs` do: resolving a `WarGraph` and rendering a
//! `GraphShape`/`ExecutionOverlay` needs `paladin-battalion` (ADR-0031).
//! `paladin-web` sees only [`RunInspectorService`] through the
//! [`RunInspectorPort`] trait object -- it never names `WarGraph`,
//! `GraphShape`, `ExecutionOverlay`, or `TraceEvent`.
//!
//! # Graph resolution (D-22, restricted to this plan's scope)
//!
//! Unlike `paladin-cli graph export`'s `--graph <FILE>` flag (28-13,
//! out of scope here), this service has exactly two buckets: the thread's
//! latest [`Run`]'s assistant version, resolved through
//! [`AssistantResolver`] to a [`WarGraph`] and rendered via
//! [`GraphShape::from_graph`]; or, when no run exists, resolution fails, or
//! the assistant is `Agent`-kind (no `WarGraph` at all),
//! [`GraphShape::observed`] -- the same D-22 fallback 28-10 established,
//! with `observed_only: true` carried through to [`InspectorView`].
//!
//! # Edge source (D-21)
//!
//! When a [`RunTracePort`] is wired AND the thread has persisted rows, the
//! overlay is built from those records
//! ([`ExecutionOverlay::from_trace_records`]) -- `fired_edges` and
//! `evaluated_edges` are exact. Otherwise the overlay is derived from
//! Waypoint history ([`ExecutionOverlay::from_waypoints`]) -- `fired_edges`
//! is derived and `evaluated_edges` stays empty. Either way,
//! [`InspectorView::supersteps`] is always built from the thread's
//! Waypoint history: a Waypoint is the durability truth (28-04), and a
//! trace is a replay convenience layered on top of it, never a
//! replacement for it.
//!
//! # `field_changes` are names only (T-28-14-01)
//!
//! When the trace source is available, `field_changes` comes straight
//! from `TraceEvent::DeltaMerged`'s own `field_changes: Vec<FieldChange>`
//! (already names-and-bookkeeping-only, D-05) -- this service never reads
//! `FieldChange::value`. Otherwise `field_changes` is derived by diffing
//! two consecutive Waypoints' `Battlefield`s BY FIELD NAME (comparing
//! `Battlefield::get_raw` for every declared field): a field whose raw
//! JSON value differs between the two snapshots is named, never the value
//! itself.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;

use paladin_battalion::engine::{ExecutionOverlay, GraphShape, OverlaySource, to_mermaid_overlay};
use paladin_core::platform::container::battlefield::{Battlefield, FieldName};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, Waypoint, WaypointStatus};
use paladin_ports::input::run_inspector_port::{
    CompletedRow, InspectorError, InspectorSource, InspectorView, RunInspectorPort, SuperstepRow,
    SuperstepStatus, VisitSummary,
};
use paladin_ports::output::run_repository_port::{RunQuery, RunRepositoryPort};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceRecord};
use paladin_ports::output::waypoint_port::WaypointPort;

use super::resolver::{AssistantResolver, Runnable};

/// Records fetched per [`RunTracePort::read`] call while paginating a
/// thread's trace history -- mirrors `events.rs`'s `REPLAY_PAGE_LIMIT`
/// (T-28-11-03's "never load an unbounded run in one shot" precedent).
const TRACE_PAGE_LIMIT: u32 = 256;

/// The facade [`RunInspectorPort`] implementation (D-24): builds an
/// [`InspectorView`] on top of Waypoint history, an optional persisted
/// trace, the run repository, and the assistant resolver.
pub struct RunInspectorService {
    waypoint_port: Arc<dyn WaypointPort>,
    run_repo: Arc<dyn RunRepositoryPort>,
    assistant_resolver: Arc<dyn AssistantResolver>,
    run_trace_port: Option<Arc<dyn RunTracePort>>,
}

impl RunInspectorService {
    /// Construct a service with no trace port wired -- every thread is
    /// inspected from Waypoint history alone. See [`Self::with_run_trace_port`]
    /// to add the exact-edge upgrade.
    pub fn new(
        waypoint_port: Arc<dyn WaypointPort>,
        run_repo: Arc<dyn RunRepositoryPort>,
        assistant_resolver: Arc<dyn AssistantResolver>,
    ) -> Self {
        Self {
            waypoint_port,
            run_repo,
            assistant_resolver,
            run_trace_port: None,
        }
    }

    /// Wire a [`RunTracePort`]: a thread with persisted rows is inspected
    /// from the exact trace source instead of derived Waypoint history.
    /// Additive -- a service constructed via [`Self::new`] alone never
    /// attempts a trace read.
    pub fn with_run_trace_port(mut self, port: Arc<dyn RunTracePort>) -> Self {
        self.run_trace_port = Some(port);
        self
    }

    /// The thread's most recently submitted [`Run`](paladin_core::platform::container::run::Run),
    /// if any (D-24: a thread may have Waypoint history with no associated
    /// run row at all).
    async fn latest_run(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<paladin_core::platform::container::run::Run>, InspectorError> {
        let page = self
            .run_repo
            .list(RunQuery {
                thread_id: Some(thread.clone()),
                limit: 1,
                ..Default::default()
            })
            .await
            .map_err(|error| InspectorError::Backend {
                message: error.to_string(),
            })?;
        Ok(page.items.into_iter().next())
    }

    /// Every full [`Waypoint`] for `thread`, ascending by superstep --
    /// `WaypointPort::history` returns lightweight summaries, so each is
    /// resolved to its full record via [`WaypointPort::get`].
    async fn load_waypoints_ascending(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<Waypoint>, InspectorError> {
        let summaries = self
            .waypoint_port
            .history(thread, None, None)
            .await
            .map_err(|error| InspectorError::Backend {
                message: error.to_string(),
            })?;

        let mut waypoints = Vec::with_capacity(summaries.len());
        for summary in &summaries {
            if let Some(wp) = self
                .waypoint_port
                .get(thread, &summary.waypoint_id)
                .await
                .map_err(|error| InspectorError::Backend {
                    message: error.to_string(),
                })?
            {
                waypoints.push(wp);
            }
        }
        waypoints.sort_by_key(|wp| wp.superstep);
        Ok(waypoints)
    }

    /// Every persisted trace record for `thread`, paginated (see
    /// [`TRACE_PAGE_LIMIT`]). Returns an empty `Vec` when no
    /// [`RunTracePort`] is wired -- not an error.
    async fn load_trace_records(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<TraceRecord>, InspectorError> {
        let Some(port) = &self.run_trace_port else {
            return Ok(Vec::new());
        };
        let mut records = Vec::new();
        let mut after_seq = 0u64;
        loop {
            let page = port
                .read(thread, after_seq, TRACE_PAGE_LIMIT)
                .await
                .map_err(|error| InspectorError::Backend {
                    message: error.to_string(),
                })?;
            if page.is_empty() {
                break;
            }
            after_seq = page.last().map(|r| r.seq).unwrap_or(after_seq);
            let page_len = page.len();
            records.extend(page);
            if page_len < TRACE_PAGE_LIMIT as usize {
                break;
            }
        }
        Ok(records)
    }

    /// Resolve `run`'s assistant to a [`GraphShape`], per this module's
    /// "Graph resolution" doc: `Some` only for a `Workflow`-kind assistant
    /// that resolves cleanly, `None` otherwise (no run, an `Agent`-kind
    /// assistant, or a resolution failure -- all fall to the D-22
    /// observed-only bucket).
    async fn resolve_graph_shape(
        &self,
        run: Option<&paladin_core::platform::container::run::Run>,
    ) -> Option<GraphShape> {
        let run = run?;
        let resolved = self
            .assistant_resolver
            .resolve(&run.assistant.assistant_id, Some(run.assistant.version))
            .await
            .ok()?;
        match resolved.runnable {
            Runnable::Workflow(graph) => Some(GraphShape::from_graph(&graph)),
            Runnable::Agent(_) => None,
        }
    }
}

#[async_trait]
impl RunInspectorPort for RunInspectorService {
    async fn inspect(&self, thread: &ThreadId) -> Result<InspectorView, InspectorError> {
        let run = self.latest_run(thread).await?;
        let waypoints = self.load_waypoints_ascending(thread).await?;
        let trace_records = self.load_trace_records(thread).await?;

        if run.is_none() && waypoints.is_empty() && trace_records.is_empty() {
            return Err(InspectorError::ThreadNotFound {
                thread_id: thread.clone(),
            });
        }

        let graph_shape = self.resolve_graph_shape(run.as_ref()).await;
        let observed_only = graph_shape.is_none();

        let mut overlay = if !trace_records.is_empty() {
            ExecutionOverlay::from_trace_records(&trace_records)
        } else {
            let shape_for_derivation = graph_shape.clone().unwrap_or(GraphShape {
                nodes: Vec::new(),
                edges: Vec::new(),
                entry: Vec::new(),
            });
            ExecutionOverlay::from_waypoints(&waypoints, &shape_for_derivation)
        };
        overlay.observed_only = observed_only;

        let render_shape = graph_shape.unwrap_or_else(|| GraphShape::observed(&overlay));
        let mermaid = to_mermaid_overlay(&render_shape, &overlay);

        let source = match overlay.source {
            OverlaySource::Waypoints => InspectorSource::Waypoints,
            OverlaySource::Trace => InspectorSource::Trace,
        };

        let supersteps = build_supersteps(&waypoints, &overlay, &trace_records);
        let visits = build_visits(&overlay);

        Ok(InspectorView {
            thread_id: thread.clone(),
            run_id: run.as_ref().map(|r| r.run_id.clone()),
            status: run.as_ref().map(|r| r.status),
            mermaid,
            observed_only,
            source,
            supersteps,
            visits,
        })
    }
}

/// Build [`SuperstepRow`]s from `waypoints` (ascending), attributing each
/// of `overlay`'s `fired_edges` to the row whose `completed` set contains
/// the edge's source node (the module doc's "Edge source" section).
fn build_supersteps(
    waypoints: &[Waypoint],
    overlay: &ExecutionOverlay,
    trace_records: &[TraceRecord],
) -> Vec<SuperstepRow> {
    let field_changes_by_superstep = if overlay.source == OverlaySource::Trace {
        Some(delta_merged_field_names_by_superstep(trace_records))
    } else {
        None
    };

    let mut rows = Vec::with_capacity(waypoints.len());
    for (index, wp) in waypoints.iter().enumerate() {
        let completed: Vec<CompletedRow> = wp
            .completed
            .iter()
            .map(|record| CompletedRow {
                node_id: record.node_id.clone(),
                attempt: record.attempt,
                outcome: record.outcome.clone(),
                // A cache-served attempt's stored duration/token figures
                // are not a meaningful execution measurement -- `None`
                // rather than a possibly-stale or possibly-zero number
                // (28-UI-SPEC.md E2 "partial").
                duration_ms: (!record.cache_hit).then_some(record.duration_ms),
                token_count: (!record.cache_hit).then_some(record.token_count),
                cache_hit: record.cache_hit,
            })
            .collect();

        let completed_ids: BTreeSet<NodeId> =
            wp.completed.iter().map(|r| r.node_id.clone()).collect();

        let fired_edges: Vec<(NodeId, NodeId)> = overlay
            .fired_edges
            .iter()
            .filter(|(from, _)| completed_ids.contains(from))
            .cloned()
            .collect();

        let evaluated_edges: Vec<(NodeId, NodeId)> = overlay
            .evaluated_edges
            .iter()
            .filter(|(from, _)| completed_ids.contains(from))
            .cloned()
            .collect();

        let field_changes = match &field_changes_by_superstep {
            Some(map) => map.get(&wp.superstep).cloned().unwrap_or_default(),
            None => {
                let previous = if index == 0 {
                    Battlefield::new(wp.battlefield.schema().clone())
                } else {
                    waypoints[index - 1].battlefield.clone()
                };
                diff_battlefield_field_names(&previous, &wp.battlefield)
            }
        };

        rows.push(SuperstepRow {
            superstep: wp.superstep,
            waypoint_id: wp.waypoint_id,
            vanguard: wp.vanguard.clone(),
            completed,
            field_changes,
            fired_edges,
            evaluated_edges,
            status: superstep_status(&wp.status),
        });
    }
    rows
}

/// Map a Waypoint's own [`WaypointStatus`] to the view's payload-free
/// [`SuperstepStatus`] (28-UI-SPEC.md E4 "partial": an awaiting-input
/// superstep is labelable, not a silently blank row).
fn superstep_status(status: &WaypointStatus) -> SuperstepStatus {
    match status {
        WaypointStatus::Running => SuperstepStatus::Running,
        WaypointStatus::Completed => SuperstepStatus::Completed,
        WaypointStatus::Failed { .. } => SuperstepStatus::Failed,
        WaypointStatus::AwaitingInput { .. } => SuperstepStatus::AwaitingInput,
        WaypointStatus::Halted => SuperstepStatus::Halted,
        // `WaypointStatus` is `#[non_exhaustive]`: a future variant this
        // view does not yet understand is treated as `Running` rather
        // than panicking or failing the whole inspect call.
        _ => SuperstepStatus::Running,
    }
}

/// Every `TraceEvent::DeltaMerged` record's field names (T-28-14-01: names
/// only, `FieldChange::value` is never read), keyed by superstep. Two
/// `DeltaMerged` records for the same superstep (should not happen in
/// practice, but not assumed) union their field names rather than
/// overwriting.
fn delta_merged_field_names_by_superstep(
    trace_records: &[TraceRecord],
) -> HashMap<u64, Vec<FieldName>> {
    let mut map: HashMap<u64, Vec<FieldName>> = HashMap::new();
    for record in trace_records {
        if let TraceEvent::DeltaMerged {
            superstep,
            field_changes,
        } = &record.event
        {
            let entry = map.entry(*superstep).or_default();
            for change in field_changes {
                if !entry.contains(&change.field) {
                    entry.push(change.field.clone());
                }
            }
        }
    }
    map
}

/// The field names whose raw JSON value differs between `previous` and
/// `current`, in `current`'s own schema declaration order. Compares only
/// [`Battlefield::get_raw`] -- never deserializes to a typed value, so no
/// field VALUE is ever held in memory by this diff, only its presence/
/// absence and byte-equality.
fn diff_battlefield_field_names(previous: &Battlefield, current: &Battlefield) -> Vec<FieldName> {
    current
        .schema()
        .fields
        .iter()
        .filter_map(|field_spec| {
            let name = &field_spec.name;
            if previous.get_raw(name) != current.get_raw(name) {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Build [`VisitSummary`]s from `overlay.visits`, ordered by each node's
/// first-visited superstep ascending (matches the order the inspector
/// page's own visits panel renders in, 28-UI-SPEC.md E2 "populated").
fn build_visits(overlay: &ExecutionOverlay) -> Vec<VisitSummary> {
    let mut visits: Vec<VisitSummary> = overlay
        .visits
        .iter()
        .map(|(node_id, visit_list)| VisitSummary {
            node_id: node_id.clone(),
            count: visit_list.len() as u32,
            supersteps: visit_list.iter().map(|v| v.superstep).collect(),
        })
        .collect();
    visits.sort_by_key(|v| v.supersteps.first().copied().unwrap_or(u64::MAX));
    visits
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use paladin_battalion::engine::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::run::{AssistantRef, Run, RunId};
    use paladin_core::platform::container::waypoint::{
        FrontierSnapshot, GraphFingerprint, NodeExecutionRecord, NodeOutcomeKind, WaypointStatus,
    };
    use paladin_storage::run::in_memory::InMemoryRunRepository;
    use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use crate::application::services::run::resolver::CodeWorkflowResolver;

    fn thread(label: &str) -> ThreadId {
        ThreadId::new(format!("inspector-test-{label}")).unwrap()
    }

    fn fingerprint() -> GraphFingerprint {
        GraphFingerprint::from_canonical_bytes(b"inspector-test-graph")
    }

    fn record(node_id: &str, attempt: u32, outcome: NodeOutcomeKind) -> NodeExecutionRecord {
        record_with_cost(node_id, attempt, outcome, 10, 5, false)
    }

    fn record_with_cost(
        node_id: &str,
        attempt: u32,
        outcome: NodeOutcomeKind,
        duration_ms: u64,
        token_count: u64,
        cache_hit: bool,
    ) -> NodeExecutionRecord {
        NodeExecutionRecord {
            node_id: NodeId::new(node_id),
            paladin_id: None,
            started_at: chrono::Utc::now(),
            duration_ms,
            token_count,
            outcome,
            attempt,
            attempts: Vec::new(),
            cache_hit,
        }
    }

    fn waypoint(
        thread_id: &ThreadId,
        superstep: u64,
        vanguard: Vec<&str>,
        completed: Vec<NodeExecutionRecord>,
        status: WaypointStatus,
        battlefield: Battlefield,
    ) -> Waypoint {
        Waypoint::new_root(
            thread_id.clone(),
            superstep,
            fingerprint(),
            battlefield,
            vanguard.into_iter().map(NodeId::new).collect(),
            completed,
            status,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        )
    }

    fn empty_battlefield() -> Battlefield {
        Battlefield::new(BattlefieldSchema::new(Vec::new()))
    }

    fn empty_graph() -> Arc<WarGraph> {
        Arc::new(WarGraph::new(
            BattlefieldSchema::new(Vec::new()),
            EngineLimits::default(),
        ))
    }

    /// A `check -> retry`, `check -> done` graph, matching 28-10's overlay
    /// fixture -- registered under `assistant_id` so `resolve_graph_shape`
    /// resolves a real `GraphShape` rather than falling to observed-only.
    fn check_retry_graph() -> Arc<WarGraph> {
        struct NoopNode;
        #[async_trait]
        impl paladin_battalion::engine::StateNode for NoopNode {
            async fn run(
                &self,
                _state: &Battlefield,
                _ctx: &paladin_battalion::engine::NodeContext,
            ) -> Result<
                paladin_core::platform::container::directive::Directive,
                paladin_battalion::engine::StateNodeError,
            > {
                Ok(StateDelta::new().into())
            }
        }

        let mut graph = WarGraph::new(BattlefieldSchema::new(Vec::new()), EngineLimits::default());
        for id in ["check", "retry", "done"] {
            graph.add_node(NodeId::new(id), NodeSpec::Function(Arc::new(NoopNode)));
        }
        graph.add_edge(EdgeSpec {
            from: NodeId::new("check"),
            to: NodeId::new("retry"),
            condition: None,
        });
        graph.add_edge(EdgeSpec {
            from: NodeId::new("check"),
            to: NodeId::new("done"),
            condition: None,
        });
        graph.add_entry(NodeId::new("check"));
        Arc::new(graph)
    }

    /// A single-field `LastWrite` schema, plus a helper to merge one value
    /// into a fresh `Battlefield` under that schema.
    fn single_field_schema(field: &FieldName) -> BattlefieldSchema {
        BattlefieldSchema::new(vec![FieldSpec::new(
            field.clone(),
            DispatchRule::LastWrite,
            None,
            false,
        )])
    }

    fn merged_battlefield(
        schema: BattlefieldSchema,
        field: &FieldName,
        value: &str,
    ) -> Battlefield {
        let mut battlefield = Battlefield::new(schema);
        let mut delta = StateDelta::new();
        delta.set(field.clone(), value).unwrap();
        battlefield
            .merge(
                vec![(NodeId::new("writer"), delta)],
                0,
                &CustomDispatchResolver::new(),
            )
            .unwrap();
        battlefield
    }

    async fn insert_run(
        repo: &Arc<dyn RunRepositoryPort>,
        thread_id: &ThreadId,
        assistant_id: &str,
    ) -> RunId {
        let run_id = RunId::new_v7();
        let run = Run::new(
            run_id.clone(),
            thread_id.clone(),
            AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        repo.insert(&run).await.unwrap();
        run_id
    }

    fn service(
        waypoint_port: Arc<dyn WaypointPort>,
        run_repo: Arc<dyn RunRepositoryPort>,
        resolver: Arc<dyn AssistantResolver>,
    ) -> RunInspectorService {
        RunInspectorService::new(waypoint_port, run_repo, resolver)
    }

    /// Test: a thread with three Waypoints yields an `InspectorView` with
    /// three `SuperstepRow`s in superstep order, each carrying its
    /// waypoint id, vanguard, completed rows and field-change names, and a
    /// non-empty `mermaid` string.
    #[tokio::test]
    async fn inspect_returns_a_view_for_a_thread_with_waypoints() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", check_retry_graph()));

        let thread_id = thread("three-waypoints");
        insert_run(&run_repo, &thread_id, "wf").await;

        let field = FieldName::new("x").unwrap();
        let schema = single_field_schema(&field);
        let bf1 = merged_battlefield(schema.clone(), &field, "v1");
        let bf2 = merged_battlefield(schema.clone(), &field, "v2");
        let bf3 = merged_battlefield(schema, &field, "v2");

        let wp1 = waypoint(
            &thread_id,
            1,
            vec!["check"],
            vec![],
            WaypointStatus::Running,
            bf1,
        );
        let wp2 = waypoint(
            &thread_id,
            2,
            vec!["retry"],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Running,
            bf2,
        );
        let wp3 = waypoint(
            &thread_id,
            3,
            vec![],
            vec![record("retry", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Completed,
            bf3,
        );
        for wp in [&wp1, &wp2, &wp3] {
            waypoint_port.save(wp).await.unwrap();
        }

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        assert_eq!(view.supersteps.len(), 3);
        assert_eq!(
            view.supersteps
                .iter()
                .map(|r| r.superstep)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(view.supersteps[1].waypoint_id, wp2.waypoint_id);
        assert_eq!(view.supersteps[1].vanguard, vec![NodeId::new("retry")]);
        assert_eq!(view.supersteps[1].completed.len(), 1);
        assert_eq!(
            view.supersteps[1].completed[0].node_id,
            NodeId::new("check")
        );
        // The field changed value from v1 to v2 between waypoints 1 and 2.
        assert_eq!(view.supersteps[1].field_changes, vec![field.clone()]);
        // The field held the same value between waypoints 2 and 3.
        assert!(view.supersteps[2].field_changes.is_empty());
        assert!(!view.mermaid.is_empty());
        assert!(!view.observed_only);
    }

    /// Test: a loop node visited in supersteps 2, 4 and 6 has `count == 3`
    /// and `supersteps == [2, 4, 6]`, and each of those rows names the
    /// edge that fired into it.
    #[tokio::test]
    async fn visit_summary_answers_the_acceptance_question() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());

        struct NoopNode;
        #[async_trait]
        impl paladin_battalion::engine::StateNode for NoopNode {
            async fn run(
                &self,
                _state: &Battlefield,
                _ctx: &paladin_battalion::engine::NodeContext,
            ) -> Result<
                paladin_core::platform::container::directive::Directive,
                paladin_battalion::engine::StateNodeError,
            > {
                Ok(StateDelta::new().into())
            }
        }
        let mut graph = WarGraph::new(BattlefieldSchema::new(Vec::new()), EngineLimits::default());
        graph.add_node(NodeId::new("loop"), NodeSpec::Function(Arc::new(NoopNode)));
        graph.add_edge(EdgeSpec {
            from: NodeId::new("loop"),
            to: NodeId::new("loop"),
            condition: None,
        });
        graph.add_entry(NodeId::new("loop"));
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", Arc::new(graph)));

        let thread_id = thread("loop");
        insert_run(&run_repo, &thread_id, "wf").await;

        let waypoints = vec![
            waypoint(
                &thread_id,
                2,
                vec!["loop"],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
                WaypointStatus::Running,
                empty_battlefield(),
            ),
            waypoint(
                &thread_id,
                4,
                vec!["loop"],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
                WaypointStatus::Running,
                empty_battlefield(),
            ),
            waypoint(
                &thread_id,
                6,
                vec![],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
                WaypointStatus::Completed,
                empty_battlefield(),
            ),
        ];
        for wp in &waypoints {
            waypoint_port.save(wp).await.unwrap();
        }

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        assert_eq!(view.visits.len(), 1);
        let loop_visits = &view.visits[0];
        assert_eq!(loop_visits.node_id, NodeId::new("loop"));
        assert_eq!(loop_visits.count, 3);
        assert_eq!(loop_visits.supersteps, vec![2, 4, 6]);

        for row in &view.supersteps {
            assert!(
                row.fired_edges
                    .contains(&(NodeId::new("loop"), NodeId::new("loop"))),
                "superstep {} must name the self-loop edge: {:?}",
                row.superstep,
                row.fired_edges
            );
        }
    }

    /// Test: a superstep whose Battlefield changed a field holding a long
    /// string yields that field's NAME in `field_changes` and no value
    /// anywhere in the view -- asserted via `Debug` formatting (Task 1;
    /// Task 2 repeats this over real JSON serialization).
    #[tokio::test]
    async fn field_changes_are_names_only() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", empty_graph()));

        let thread_id = thread("field-names-only");
        insert_run(&run_repo, &thread_id, "wf").await;

        let field = FieldName::new("secret_field").unwrap();
        let secret_value = "AAAA_SENSITIVE_INSPECTOR_VALUE_AAAA";
        let schema = single_field_schema(&field);
        let bf1 = Battlefield::new(schema.clone());
        let bf2 = merged_battlefield(schema, &field, secret_value);

        let wp1 = waypoint(&thread_id, 1, vec![], vec![], WaypointStatus::Running, bf1);
        let wp2 = waypoint(
            &thread_id,
            2,
            vec![],
            vec![],
            WaypointStatus::Completed,
            bf2,
        );
        waypoint_port.save(&wp1).await.unwrap();
        waypoint_port.save(&wp2).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        assert_eq!(view.supersteps[1].field_changes, vec![field]);
        let debug_output = format!("{view:?}");
        assert!(
            !debug_output.contains(secret_value),
            "the secret value must never appear in the view: {debug_output}"
        );
    }

    /// Test: with `run_traces` rows present, `source` is the trace source
    /// and `fired_edges` come from `EdgeEvaluated` records rather than
    /// derivation.
    #[tokio::test]
    async fn trace_source_populates_exact_edges() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", check_retry_graph()));
        let trace_port: Arc<dyn RunTracePort> = Arc::new(InMemoryRunTraceStore::new());

        let thread_id = thread("trace-source");
        insert_run(&run_repo, &thread_id, "wf").await;

        let wp = waypoint(
            &thread_id,
            1,
            vec![],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Completed,
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let records = vec![
            trace_record(
                &thread_id,
                1,
                TraceEvent::NodeFinished {
                    superstep: 1,
                    node_id: NodeId::new("check"),
                    attempt: 1,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms: 10,
                    token_count: 5,
                    cache_hit: false,
                },
            ),
            trace_record(
                &thread_id,
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "always".to_string(),
                    fired: true,
                },
            ),
            trace_record(
                &thread_id,
                3,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("done"),
                    condition_kind: "always".to_string(),
                    fired: false,
                },
            ),
        ];
        trace_port.append(&records).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver).with_run_trace_port(trace_port);
        let view = svc.inspect(&thread_id).await.unwrap();

        assert_eq!(view.source, InspectorSource::Trace);
        assert!(
            view.supersteps[0]
                .fired_edges
                .contains(&(NodeId::new("check"), NodeId::new("retry")))
        );
    }

    /// Test: an unknown thread yields `InspectorError::ThreadNotFound`.
    #[tokio::test]
    async fn unknown_thread_is_thread_not_found() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> = Arc::new(CodeWorkflowResolver::new());

        let svc = service(waypoint_port, run_repo, resolver);
        let thread_id = thread("unknown");
        let err = svc.inspect(&thread_id).await.unwrap_err();
        assert!(matches!(err, InspectorError::ThreadNotFound { .. }));
    }

    /// Test: a known thread (a `Run` row exists) with no Waypoints and no
    /// trace rows yields `Ok` with empty `supersteps` and `visits`, not an
    /// error.
    #[tokio::test]
    async fn thread_with_no_history_is_an_empty_view() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", empty_graph()));

        let thread_id = thread("no-history");
        insert_run(&run_repo, &thread_id, "wf").await;

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        assert!(view.supersteps.is_empty());
        assert!(view.visits.is_empty());
    }

    fn trace_record(thread_id: &ThreadId, seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id: thread_id.clone(),
            run_id: None,
            seq,
            at: chrono::Utc::now(),
            event,
        }
    }

    /// Test: the view exposes both the fired-edge set and, when the
    /// source is a trace, the evaluated-but-not-fired set on the SAME row
    /// -- so the page can label the missing half when it is a
    /// Waypoints-only source rather than silently omitting it.
    #[tokio::test]
    async fn view_distinguishes_exact_from_derived_edges() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", check_retry_graph()));
        let trace_port: Arc<dyn RunTracePort> = Arc::new(InMemoryRunTraceStore::new());

        let thread_id = thread("distinguishes-edges");
        insert_run(&run_repo, &thread_id, "wf").await;

        let wp = waypoint(
            &thread_id,
            1,
            vec![],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Completed,
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let records = vec![
            trace_record(
                &thread_id,
                1,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "always".to_string(),
                    fired: true,
                },
            ),
            trace_record(
                &thread_id,
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("done"),
                    condition_kind: "always".to_string(),
                    fired: false,
                },
            ),
        ];
        trace_port.append(&records).await.unwrap();

        let svc = service(waypoint_port.clone(), run_repo.clone(), resolver.clone())
            .with_run_trace_port(trace_port);
        let trace_view = svc.inspect(&thread_id).await.unwrap();
        assert_eq!(trace_view.source, InspectorSource::Trace);
        assert!(
            trace_view.supersteps[0]
                .evaluated_edges
                .contains(&(NodeId::new("check"), NodeId::new("done")))
        );

        // Waypoints-only source (no trace port wired): `evaluated_edges`
        // must stay empty on every row -- the page's own cue that the
        // missing half is unavailable, not that nothing was evaluated.
        let waypoints_svc = service(waypoint_port, run_repo, resolver);
        let waypoints_view = waypoints_svc.inspect(&thread_id).await.unwrap();
        assert_eq!(waypoints_view.source, InspectorSource::Waypoints);
        for row in &waypoints_view.supersteps {
            assert!(row.evaluated_edges.is_empty());
        }
    }

    /// Test: `observed_only` and `source` are both present and correct --
    /// for a thread with a resolvable graph (`observed_only: false`) and
    /// for one with none (`observed_only: true`, no run at all).
    #[tokio::test]
    async fn view_carries_observed_only_and_source() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", check_retry_graph()));

        let resolved_thread = thread("observed-only-resolved");
        insert_run(&run_repo, &resolved_thread, "wf").await;
        let wp = waypoint(
            &resolved_thread,
            1,
            vec![],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Completed,
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let svc = service(waypoint_port.clone(), run_repo.clone(), resolver.clone());
        let resolved_view = svc.inspect(&resolved_thread).await.unwrap();
        assert!(!resolved_view.observed_only);
        assert_eq!(resolved_view.source, InspectorSource::Waypoints);

        // A thread known only via Waypoint history (no `Run` row at all)
        // has no assistant to resolve a graph from -- always observed-only.
        let unresolved_thread = thread("observed-only-unresolved");
        let wp2 = waypoint(
            &unresolved_thread,
            1,
            vec![],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Completed,
            empty_battlefield(),
        );
        waypoint_port.save(&wp2).await.unwrap();
        let view = svc.inspect(&unresolved_thread).await.unwrap();
        assert!(view.observed_only);
    }

    /// Test: a cache-hit attempt yields `None` in `duration_ms`/
    /// `token_count` rather than a (possibly stale, possibly zero) number
    /// -- the page renders a dash and the row is never omitted. A
    /// non-cache-hit attempt still carries `Some` real figures.
    #[tokio::test]
    async fn completed_row_partial_values_are_representable() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", empty_graph()));

        let thread_id = thread("partial-values");
        insert_run(&run_repo, &thread_id, "wf").await;

        let wp = waypoint(
            &thread_id,
            1,
            vec![],
            vec![
                record_with_cost("cached", 1, NodeOutcomeKind::Succeeded, 999, 999, true),
                record_with_cost("executed", 1, NodeOutcomeKind::Succeeded, 42, 7, false),
            ],
            WaypointStatus::Completed,
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        let row = &view.supersteps[0];
        let cached = row
            .completed
            .iter()
            .find(|c| c.node_id == NodeId::new("cached"))
            .unwrap();
        assert!(cached.cache_hit);
        assert_eq!(cached.duration_ms, None);
        assert_eq!(cached.token_count, None);

        let executed = row
            .completed
            .iter()
            .find(|c| c.node_id == NodeId::new("executed"))
            .unwrap();
        assert!(!executed.cache_hit);
        assert_eq!(executed.duration_ms, Some(42));
        assert_eq!(executed.token_count, Some(7));
    }

    /// Test: a Gate suspension yields a superstep row with an empty
    /// `completed` list and a `status` the page can label
    /// (`SuperstepStatus::AwaitingInput`), not a missing row.
    #[tokio::test]
    async fn superstep_awaiting_input_has_an_empty_completed_list() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", empty_graph()));

        let thread_id = thread("awaiting-input");
        insert_run(&run_repo, &thread_id, "wf").await;

        let parley = paladin_core::platform::container::parley::ParleyRequest {
            parley_id: paladin_core::platform::container::parley::ParleyId::new(),
            node_id: NodeId::new("gate"),
            kind: paladin_core::platform::container::parley::ParleyKind::Approval,
            prompt: "Proceed?".to_string(),
            payload: serde_json::Value::Null,
            choices: None,
            expires_at: None,
            created_at: chrono::Utc::now(),
            on_expire: paladin_core::platform::container::parley::OnExpire::FailRun,
        };
        let wp = waypoint(
            &thread_id,
            1,
            vec![],
            vec![],
            WaypointStatus::AwaitingInput {
                parleys: vec![parley],
                responses: vec![],
            },
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        let row = &view.supersteps[0];
        assert!(row.completed.is_empty());
        assert_eq!(row.status, SuperstepStatus::AwaitingInput);
    }

    /// Test: the whole view serializes to JSON and deserializes back to an
    /// equal value -- the page embeds exactly this payload.
    #[tokio::test]
    async fn view_is_serializable_and_round_trips() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", check_retry_graph()));

        let thread_id = thread("round-trips");
        insert_run(&run_repo, &thread_id, "wf").await;
        let wp = waypoint(
            &thread_id,
            1,
            vec!["retry"],
            vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            WaypointStatus::Running,
            empty_battlefield(),
        );
        waypoint_port.save(&wp).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        let json = serde_json::to_string(&view).unwrap();
        let round_tripped: InspectorView = serde_json::from_str(&json).unwrap();
        assert_eq!(view, round_tripped);
    }

    /// Test: serializing a view for a run whose Battlefield held a
    /// distinctive secret-shaped string yields JSON that does not contain
    /// it -- the type-level guarantee (T-28-14-01) proven over the ACTUAL
    /// wire payload, not just `Debug` output (see `field_changes_are_names_only`
    /// for the Task 1 analog).
    #[tokio::test]
    async fn serialized_view_contains_no_field_values() {
        let waypoint_port: Arc<dyn WaypointPort> = Arc::new(InMemoryWaypointStore::new());
        let run_repo: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("wf", empty_graph()));

        let thread_id = thread("serialized-no-values");
        insert_run(&run_repo, &thread_id, "wf").await;

        let field = FieldName::new("secret_field").unwrap();
        let secret_value = "AAAA_SENSITIVE_SERIALIZED_VALUE_AAAA";
        let schema = single_field_schema(&field);
        let bf1 = Battlefield::new(schema.clone());
        let bf2 = merged_battlefield(schema, &field, secret_value);

        let wp1 = waypoint(&thread_id, 1, vec![], vec![], WaypointStatus::Running, bf1);
        let wp2 = waypoint(
            &thread_id,
            2,
            vec![],
            vec![],
            WaypointStatus::Completed,
            bf2,
        );
        waypoint_port.save(&wp1).await.unwrap();
        waypoint_port.save(&wp2).await.unwrap();

        let svc = service(waypoint_port, run_repo, resolver);
        let view = svc.inspect(&thread_id).await.unwrap();

        let json = serde_json::to_string(&view).unwrap();
        assert!(
            !json.contains(secret_value),
            "the secret value must never appear in the serialized view: {json}"
        );
        assert!(json.contains("secret_field"));
    }
}
