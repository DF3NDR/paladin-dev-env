//! `ExecutionOverlay` -- what actually happened, layered onto a
//! [`GraphShape`] (D-21, OBS-03 / OBS-FR-09): which branch fired, how many
//! times each node ran, and what each visit cost, answerable from either the
//! always-available [`Waypoint`] history or an exact persisted trace.
//!
//! # Two sources, one overlay (D-21)
//!
//! [`ExecutionOverlay::from_waypoints`] builds from `WaypointPort::history` +
//! each [`Waypoint::completed`]/[`Waypoint::vanguard`] -- always available,
//! since every run checkpoints Waypoints, but its fired-edge set is
//! **derived**: a completed node `A` in one Waypoint and a vanguard node `C`
//! in the NEXT Waypoint, where the shape declares an `A -> C` edge, is taken
//! as that edge having fired. No `evaluated_edges` are known from Waypoint
//! history alone (a Waypoint records what ran, not which candidate edges
//! were considered and rejected), so that set stays empty and `source` is
//! [`OverlaySource::Waypoints`].
//!
//! [`ExecutionOverlay::from_trace_records`] builds from persisted
//! [`TraceRecord`]s (28-01/28-03/28-04) -- an upgrade available only when a
//! trace was persisted for the run -- and both edge sets are **exact**,
//! read straight from `TraceEvent::EdgeEvaluated`. `source` is
//! [`OverlaySource::Trace`]. Carrying `source` on the overlay itself (rather
//! than only in `evaluated_edges`' emptiness) means a Waypoints-derived
//! overlay is never mistaken for an exact one downstream (T-28-10-02).
//!
//! # Security (T-28-10-01)
//!
//! [`Visit`] carries superstep, attempt, outcome, duration, tokens and the
//! cache-hit flag only -- never a `Battlefield` field VALUE. `DeltaMerged`
//! trace events (which DO carry opt-in field values, D-05) are not read by
//! either overlay constructor.
//!
//! # No DOT overlay
//!
//! Only [`super::mermaid::to_mermaid_overlay`] exists -- there is no DOT
//! overlay renderer. The PRD names Mermaid for the execution overlay
//! specifically (D-21); this omission is deliberate, not an oversight a
//! future reader should "fix" by adding one without a requirement driving
//! it.

use std::collections::{BTreeMap, BTreeSet};

use paladin_core::platform::container::trace::{TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, Waypoint};

use crate::engine::export::shape::GraphShape;

/// One node's execution during one superstep (D-21): outcome, cost and
/// whether it was served from the node cache -- never a `Battlefield` field
/// value (T-28-10-01).
#[derive(Debug, Clone, PartialEq)]
pub struct Visit {
    /// The superstep this visit belongs to.
    pub superstep: u64,
    /// The 1-indexed attempt this visit's `outcome` belongs to (mirrors
    /// [`NodeExecutionRecord::attempt`](paladin_core::platform::container::waypoint::NodeExecutionRecord::attempt)/
    /// `TraceEvent::NodeFinished::attempt`).
    pub attempt: u32,
    /// This visit's outcome.
    pub outcome: NodeOutcomeKind,
    /// How long this visit took, in milliseconds.
    pub duration_ms: u64,
    /// Tokens consumed by this visit.
    pub tokens: u64,
    /// Whether this visit's outcome was served from the node cache (FT-06)
    /// rather than by executing the node.
    pub cache_hit: bool,
}

/// Where an [`ExecutionOverlay`]'s data came from (D-21, T-28-10-02): carried
/// on the overlay and rendered, so a derived (Waypoints) overlay is never
/// presented as if it were exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySource {
    /// Built from [`Waypoint`] history alone: `fired_edges` is derived,
    /// `evaluated_edges` is always empty.
    Waypoints,
    /// Built from persisted [`TraceRecord`]s: both edge sets are exact.
    Trace,
}

/// Execution history layered onto a [`GraphShape`] (D-21): which nodes ran,
/// how many times, what each run cost, and which edges fired -- the data the
/// OBS-03 acceptance question ("which branch fired, and why did node X run 3
/// times") is answered from.
///
/// Every collection is ordered (`BTreeMap`/`BTreeSet`, never `HashMap`/
/// `HashSet`) so [`super::mermaid::to_mermaid_overlay`]'s output stays
/// byte-deterministic across repeated calls, mirroring [`GraphShape`]'s own
/// determinism contract.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionOverlay {
    /// Every visited node's [`Visit`] history, in the order each node was
    /// visited (ascending superstep).
    pub visits: BTreeMap<NodeId, Vec<Visit>>,
    /// Edges that fired -- derived (Waypoints source) or exact (Trace
    /// source).
    pub fired_edges: BTreeSet<(NodeId, NodeId)>,
    /// Edges that were evaluated but did NOT fire. Always empty for a
    /// [`OverlaySource::Waypoints`] overlay (D-21) -- Waypoint history
    /// carries no record of a rejected candidate edge.
    pub evaluated_edges: BTreeSet<(NodeId, NodeId)>,
    /// Where this overlay's data came from.
    pub source: OverlaySource,
    /// Whether the [`GraphShape`] this overlay was rendered onto was built
    /// by [`GraphShape::observed`](super::shape::GraphShape::observed) --
    /// i.e. no static graph document was available for the thread (D-22).
    pub observed_only: bool,
}

impl ExecutionOverlay {
    /// Build an `ExecutionOverlay` from a thread's Waypoint history (D-21):
    /// always available, since every run checkpoints Waypoints, but its
    /// fired-edge set is **derived**, not exact.
    ///
    /// `waypoints` is sorted by ascending `superstep` internally before any
    /// derivation runs -- `WaypointPort::history` returns newest-first
    /// (descending `created_at`), so this constructor never assumes its
    /// caller already reordered the slice.
    ///
    /// Visits: every [`Waypoint::completed`] record becomes one [`Visit`],
    /// appended to its node's list in ascending-superstep order.
    ///
    /// Fired edges: [`Waypoint::vanguard`]'s own contract is "nodes ready
    /// for the NEXT superstep" -- the checkpoint the engine writes after
    /// superstep `n` (`engine::superstep::build_waypoint`) carries BOTH
    /// `completed` (what ran AT superstep `n`) and `vanguard` (what is
    /// dispatched at superstep `n + 1`) on that SAME `Waypoint`, so the
    /// `n -> n+1` transition this derivation needs is already co-located:
    /// for every `Waypoint`, every node in its OWN `completed` set crossed
    /// with every node in its OWN `vanguard` -- never a different
    /// `Waypoint`'s `vanguard` field. A candidate `(a, c)` pair is added to
    /// `fired_edges` only when the shape actually declares an `a -> c`
    /// edge, so an unrelated coincidence of "X completed this superstep, Y
    /// is vanguard next superstep" is never mistaken for a fired edge.
    ///
    /// `evaluated_edges` is always empty and `source` is always
    /// [`OverlaySource::Waypoints`].
    pub fn from_waypoints(waypoints: &[Waypoint], shape: &GraphShape) -> Self {
        let mut ordered: Vec<&Waypoint> = waypoints.iter().collect();
        ordered.sort_by_key(|wp| wp.superstep);

        let mut visits: BTreeMap<NodeId, Vec<Visit>> = BTreeMap::new();
        for wp in &ordered {
            for record in &wp.completed {
                visits
                    .entry(record.node_id.clone())
                    .or_default()
                    .push(Visit {
                        superstep: wp.superstep,
                        attempt: record.attempt,
                        outcome: record.outcome.clone(),
                        duration_ms: record.duration_ms,
                        tokens: record.token_count,
                        cache_hit: record.cache_hit,
                    });
            }
        }

        let shape_edges = collect_shape_edges(shape);
        let mut fired_edges: BTreeSet<(NodeId, NodeId)> = BTreeSet::new();
        // `Waypoint::vanguard`'s own contract ("nodes ready for the NEXT
        // superstep") means the transition this derivation needs is
        // already co-located on ONE `Waypoint`: the checkpoint superstep
        // engine writes after superstep `n` carries `completed` (what ran
        // AT superstep `n`) and `vanguard` (what's dispatched at superstep
        // `n + 1`) together (`engine::superstep::build_waypoint`'s
        // `next_vanguard` argument) -- so `A` (completed this waypoint) and
        // `C` (this SAME waypoint's own vanguard) are the `n -> n+1`
        // transition, never a different waypoint's `vanguard` field.
        for wp in &ordered {
            for completed in &wp.completed {
                for vanguard_node in &wp.vanguard {
                    let candidate = (completed.node_id.clone(), vanguard_node.clone());
                    if shape_edges.contains(&candidate) {
                        fired_edges.insert(candidate);
                    }
                }
            }
        }

        ExecutionOverlay {
            visits,
            fired_edges,
            evaluated_edges: BTreeSet::new(),
            source: OverlaySource::Waypoints,
            observed_only: false,
        }
    }

    /// Build an `ExecutionOverlay` from persisted [`TraceRecord`]s (D-21):
    /// available only when a trace was persisted for the run, but both edge
    /// sets are **exact**.
    ///
    /// Visits: one per `TraceEvent::NodeFinished` (one per attempt, D-16) --
    /// the paired `NodeStarted` establishes when the attempt began, but
    /// every value a [`Visit`] needs (`outcome`, `duration_ms`,
    /// `token_count`, `cache_hit`) is already carried on `NodeFinished`
    /// itself.
    ///
    /// Edges: every `TraceEvent::EdgeEvaluated` is added to `evaluated_edges`
    /// unconditionally, and to `fired_edges` too when `fired` is `true` --
    /// no derivation, read straight from the record.
    ///
    /// `source` is always [`OverlaySource::Trace`]. Records are consumed in
    /// their given order; a caller supplying `run_trace_port::read`'s own
    /// `seq`-ordered result needs no re-sorting.
    // RED (TDD): always returns an empty overlay -- deliberately wrong, so
    // this plan's Task 2 `<behavior>` tests fail for the right reason
    // before the GREEN commit implements the real body.
    pub fn from_trace_records(_records: &[TraceRecord]) -> Self {
        ExecutionOverlay {
            visits: BTreeMap::new(),
            fired_edges: BTreeSet::new(),
            evaluated_edges: BTreeSet::new(),
            source: OverlaySource::Trace,
            observed_only: false,
        }
    }
}

/// Every edge in `shape`, recursively including nested `Workflow` subgraphs,
/// as a lookup set for [`ExecutionOverlay::from_waypoints`]' fired-edge
/// derivation.
fn collect_shape_edges(shape: &GraphShape) -> BTreeSet<(NodeId, NodeId)> {
    let mut set = BTreeSet::new();
    collect_shape_edges_into(shape, &mut set);
    set
}

fn collect_shape_edges_into(shape: &GraphShape, set: &mut BTreeSet<(NodeId, NodeId)>) {
    for edge in &shape.edges {
        set.insert((edge.from.clone(), edge.to.clone()));
    }
    for node in &shape.nodes {
        if let Some(sub) = &node.subgraph {
            collect_shape_edges_into(sub, set);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;
    use paladin_core::platform::container::battlefield::{Battlefield, BattlefieldSchema};
    use paladin_core::platform::container::waypoint::{
        FrontierSnapshot, GraphFingerprint, NodeExecutionRecord, ThreadId, WaypointStatus,
    };

    use super::*;
    use crate::engine::export::mermaid::to_mermaid_overlay;
    use crate::engine::export::shape::{ShapeEdge, ShapeKind, ShapeNode};

    fn thread() -> ThreadId {
        ThreadId::new("overlay-test-thread").expect("valid thread id")
    }

    /// A minimal root `Waypoint` carrying only what the overlay reads:
    /// `superstep`, `vanguard`, `completed`. Every other field is a
    /// harmless placeholder -- `from_waypoints` never inspects them.
    fn waypoint(
        superstep: u64,
        vanguard: Vec<&str>,
        completed: Vec<NodeExecutionRecord>,
    ) -> Waypoint {
        Waypoint::new_root(
            thread(),
            superstep,
            GraphFingerprint::from_canonical_bytes(b"overlay-test-graph"),
            Battlefield::new(BattlefieldSchema::new(Vec::new())),
            vanguard.into_iter().map(NodeId::new).collect(),
            completed,
            WaypointStatus::Running,
            BTreeMap::new(),
            FrontierSnapshot::default(),
        )
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
            started_at: Utc::now(),
            duration_ms,
            token_count,
            outcome,
            attempt,
            attempts: Vec::new(),
            cache_hit,
        }
    }

    fn shape_node(id: &str) -> ShapeNode {
        ShapeNode {
            id: NodeId::new(id),
            kind: ShapeKind::Paladin,
            deferred: false,
            worker_template: false,
            subgraph: None,
        }
    }

    fn shape_edge(from: &str, to: &str) -> ShapeEdge {
        ShapeEdge {
            from: NodeId::new(from),
            to: NodeId::new(to),
            condition: None,
        }
    }

    /// The `check -> retry` branching shape the Task 1 `<behavior>` example
    /// names: `check`, `retry`, `done`, with `check` able to route to either.
    fn check_retry_shape() -> GraphShape {
        GraphShape {
            nodes: vec![shape_node("check"), shape_node("retry"), shape_node("done")],
            edges: vec![shape_edge("check", "retry"), shape_edge("check", "done")],
            entry: vec![NodeId::new("check")],
        }
    }

    #[test]
    fn overlay_from_waypoints_derives_fired_edges() {
        let shape = check_retry_shape();
        // Superstep 2 completes `check`; superstep 3's vanguard is `retry`
        // -- the shape declares `check -> retry`, so that pair is derived
        // as fired. `check -> done` is never even a candidate this run.
        let waypoints = vec![
            waypoint(
                2,
                vec!["retry"],
                vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            ),
            waypoint(
                3,
                vec![],
                vec![record("retry", 1, NodeOutcomeKind::Succeeded)],
            ),
        ];

        let overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        assert_eq!(overlay.source, OverlaySource::Waypoints);
        assert!(overlay.evaluated_edges.is_empty());
        assert_eq!(overlay.fired_edges.len(), 1);
        assert!(
            overlay
                .fired_edges
                .contains(&(NodeId::new("check"), NodeId::new("retry")))
        );
        assert!(
            !overlay
                .fired_edges
                .contains(&(NodeId::new("check"), NodeId::new("done")))
        );
    }

    #[test]
    fn overlay_visits_are_ordered_per_node() {
        let shape = GraphShape {
            nodes: vec![shape_node("loop")],
            edges: vec![],
            entry: vec![NodeId::new("loop")],
        };
        // A loop node visited at supersteps 2, 4 and 6 -- passed out of
        // order (as `WaypointPort::history`'s newest-first ordering would
        // hand them) to prove `from_waypoints` sorts before deriving.
        let waypoints = vec![
            waypoint(
                6,
                vec![],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
            ),
            waypoint(
                2,
                vec!["loop"],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
            ),
            waypoint(
                4,
                vec!["loop"],
                vec![record("loop", 1, NodeOutcomeKind::Succeeded)],
            ),
        ];

        let overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        let visits = overlay
            .visits
            .get(&NodeId::new("loop"))
            .expect("loop visited");
        assert_eq!(visits.len(), 3);
        assert_eq!(
            visits.iter().map(|v| v.superstep).collect::<Vec<_>>(),
            vec![2, 4, 6]
        );
    }

    #[test]
    fn overlay_mermaid_colours_by_last_outcome() {
        let shape = GraphShape {
            nodes: vec![shape_node("a"), shape_node("b")],
            edges: vec![],
            entry: vec![NodeId::new("a")],
        };
        let waypoints = vec![
            waypoint(1, vec![], vec![record("a", 1, NodeOutcomeKind::Failed)]),
            waypoint(2, vec![], vec![record("b", 1, NodeOutcomeKind::Succeeded)]),
        ];
        let overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        let rendered = to_mermaid_overlay(&shape, &overlay);
        assert!(rendered.contains("classDef outcomeFailed"));
        assert!(rendered.contains("classDef outcomeSuccess"));

        let a_id = "n0";
        let b_id = "n1";
        assert!(rendered.contains(&format!("class {a_id} outcomeFailed")));
        assert!(rendered.contains(&format!("class {b_id} outcomeSuccess")));
    }

    #[test]
    fn overlay_mermaid_annotates_repeat_visits_and_cost() {
        let shape = GraphShape {
            nodes: vec![shape_node("loop")],
            edges: vec![],
            entry: vec![NodeId::new("loop")],
        };
        let waypoints = vec![
            waypoint(
                2,
                vec!["loop"],
                vec![record_with_cost(
                    "loop",
                    1,
                    NodeOutcomeKind::Succeeded,
                    100,
                    20,
                    false,
                )],
            ),
            waypoint(
                4,
                vec!["loop"],
                vec![record_with_cost(
                    "loop",
                    1,
                    NodeOutcomeKind::Succeeded,
                    150,
                    30,
                    false,
                )],
            ),
            waypoint(
                6,
                vec![],
                vec![record_with_cost(
                    "loop",
                    1,
                    NodeOutcomeKind::Succeeded,
                    200,
                    40,
                    false,
                )],
            ),
        ];
        let overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        let rendered = to_mermaid_overlay(&shape, &overlay);
        assert!(
            rendered.contains("×3"),
            "expected a ×3 visit badge: {rendered}"
        );
        // Every visited node's label carries duration and token figures
        // for its LAST visit.
        assert!(
            rendered.contains("200ms"),
            "expected the last visit's duration: {rendered}"
        );
        assert!(
            rendered.contains("40tok"),
            "expected the last visit's tokens: {rendered}"
        );
    }

    #[test]
    fn overlay_mermaid_renders_fired_edges_bold() {
        let shape = check_retry_shape();
        let waypoints = vec![
            waypoint(
                2,
                vec!["retry"],
                vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            ),
            waypoint(
                3,
                vec![],
                vec![record("retry", 1, NodeOutcomeKind::Succeeded)],
            ),
        ];
        let overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        let rendered = to_mermaid_overlay(&shape, &overlay);
        // check -> retry fired: bold link.
        assert!(
            rendered.contains("==>"),
            "expected a bold fired-edge arrow: {rendered}"
        );
        // check -> done was never evaluated (Waypoints source carries no
        // evaluated_edges at all) -- no dotted arrow appears.
        assert!(
            !rendered.contains("-.->"),
            "no dotted arrow with a Waypoints source: {rendered}"
        );
    }

    #[test]
    fn overlay_from_trace_is_exact() {
        let records = vec![
            trace_record(
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
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "contains".to_string(),
                    fired: true,
                },
            ),
            trace_record(
                3,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("done"),
                    condition_kind: "contains".to_string(),
                    fired: false,
                },
            ),
        ];

        let overlay = ExecutionOverlay::from_trace_records(&records);

        assert_eq!(overlay.source, OverlaySource::Trace);
        assert_eq!(overlay.evaluated_edges.len(), 2);
        assert_eq!(overlay.fired_edges.len(), 1);
        assert!(
            overlay
                .fired_edges
                .contains(&(NodeId::new("check"), NodeId::new("retry")))
        );
        assert!(
            overlay
                .evaluated_edges
                .contains(&(NodeId::new("check"), NodeId::new("done")))
        );
    }

    #[test]
    fn trace_overlay_shows_evaluated_but_not_fired() {
        let shape = check_retry_shape();
        let records = vec![
            trace_record(
                1,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "contains".to_string(),
                    fired: true,
                },
            ),
            trace_record(
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("done"),
                    condition_kind: "contains".to_string(),
                    fired: false,
                },
            ),
        ];
        let overlay = ExecutionOverlay::from_trace_records(&records);

        assert!(
            overlay
                .evaluated_edges
                .contains(&(NodeId::new("check"), NodeId::new("done")))
        );
        assert!(
            !overlay
                .fired_edges
                .contains(&(NodeId::new("check"), NodeId::new("done")))
        );

        let rendered = to_mermaid_overlay(&shape, &overlay);
        assert!(
            rendered.contains("-.->"),
            "evaluated-not-fired must render dotted: {rendered}"
        );
    }

    #[test]
    fn observed_shape_is_built_when_no_graph_is_available() {
        let records = vec![
            trace_record(
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
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "contains".to_string(),
                    fired: true,
                },
            ),
        ];
        let mut overlay = ExecutionOverlay::from_trace_records(&records);
        overlay.observed_only = true;

        let observed = GraphShape::observed(&overlay);
        assert!(observed.nodes.iter().any(|n| n.id == NodeId::new("check")));
        assert!(observed.nodes.iter().any(|n| n.id == NodeId::new("retry")));
        assert!(
            observed
                .edges
                .iter()
                .any(|e| e.from == NodeId::new("check") && e.to == NodeId::new("retry"))
        );

        let rendered = to_mermaid_overlay(&observed, &overlay);
        let title_line = rendered
            .lines()
            .find(|line| line.contains("observed nodes only"))
            .expect("observed-only title line present");
        assert_eq!(
            title_line,
            "title: (observed nodes only — no graph document available)"
        );
    }

    #[test]
    fn waypoint_and_trace_overlays_of_the_same_run_differ_only_in_evaluated_edges() {
        let shape = check_retry_shape();
        let waypoints = vec![
            waypoint(
                2,
                vec!["retry"],
                vec![record("check", 1, NodeOutcomeKind::Succeeded)],
            ),
            waypoint(
                3,
                vec![],
                vec![record("retry", 1, NodeOutcomeKind::Succeeded)],
            ),
        ];
        let waypoint_overlay = ExecutionOverlay::from_waypoints(&waypoints, &shape);

        let trace_records = vec![
            trace_record(
                1,
                TraceEvent::NodeFinished {
                    superstep: 2,
                    node_id: NodeId::new("check"),
                    attempt: 1,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms: 10,
                    token_count: 5,
                    cache_hit: false,
                },
            ),
            trace_record(
                2,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("retry"),
                    condition_kind: "contains".to_string(),
                    fired: true,
                },
            ),
            trace_record(
                3,
                TraceEvent::EdgeEvaluated {
                    from: NodeId::new("check"),
                    to: NodeId::new("done"),
                    condition_kind: "contains".to_string(),
                    fired: false,
                },
            ),
            trace_record(
                4,
                TraceEvent::NodeFinished {
                    superstep: 3,
                    node_id: NodeId::new("retry"),
                    attempt: 1,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms: 10,
                    token_count: 5,
                    cache_hit: false,
                },
            ),
        ];
        let trace_overlay = ExecutionOverlay::from_trace_records(&trace_records);

        assert_eq!(waypoint_overlay.fired_edges, trace_overlay.fired_edges);
        assert!(waypoint_overlay.evaluated_edges.is_empty());
        assert!(!trace_overlay.evaluated_edges.is_empty());
        assert_ne!(waypoint_overlay.source, trace_overlay.source);
    }

    fn trace_record(seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            thread_id: thread(),
            run_id: None,
            seq,
            at: Utc::now(),
            event,
        }
    }
}
