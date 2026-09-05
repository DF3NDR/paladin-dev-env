//! CF-FR-17 / PRD `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` §3
//! acceptance criterion 4: a `from_formation` subgraph embedded as a `NodeSpec::Battalion`
//! node of a BRANCHING parent graph -- not merely a linear wrapper -- runs correctly (the
//! Formation's nodes execute in their sequential order and its mapped output reaches the
//! parent; the branch not taken never fires), and a run killed after the child's FIRST
//! superstep resumes without repeating the child's already-completed work while the final
//! parent Battlefield matches an uninterrupted control run's. A third, lighter test proves
//! `from_phalanx` and `from_campaign` graphs construct into `NodeSpec::Battalion` and
//! validate too, so CF-FR-17's "the legacy bridges embed for free" claim is not scoped to
//! `from_formation` alone.
//!
//! ## Why the "crash" is simulated by re-seeding a fresh `SqliteWaypointStore` file rather
//! than aborting a live `WarEngine::start`/`resume` task
//!
//! Same rationale as `tests/integration/e2e_crash_resume_test.rs`'s own module doc comment:
//! the superstep loop spawns each vanguard node's execution via `tokio::spawn`, a genuinely
//! detached task, so racing a live run against a "stop once N Waypoints exist" signal cannot
//! guarantee no additional Waypoint is written in the background after the signal fires. Both
//! the graph and the mock port here are fully deterministic, so the first Waypoints of an
//! uninterrupted control run are byte-for-byte identical to what a live "run and kill after
//! the child's first superstep" would have produced. Reading those real, durably-persisted
//! Waypoints back out of the control run's own `SqliteWaypointStore` -- for BOTH the parent
//! thread and the child's own derived thread -- and re-saving them into a second, fresh
//! temporary database file is therefore an equivalent, deterministic, CI-safe simulation of
//! "a thread whose latest Waypoint reflects a process that died right after the child's first
//! superstep", without the background-task race.

use std::sync::Arc;
use std::time::Duration;

use paladin_battalion::EdgeEvaluatorRegistry;
use paladin_battalion::engine::graph::StateMap;
use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode, WarEngine,
    WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battalion::BattalionConfig;
use paladin_core::platform::container::battalion::campaign::{Campaign, EdgeCondition};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName, FieldSpec,
    StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::{Paladin, PaladinData};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, Waypoint, WaypointStatus};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this standalone
// [[test]] target only needs `FaultyPaladinPort`, matching the pattern already established
// by `tests/integration/e2e_crash_resume_test.rs` and `golden_bridge_equivalence_test.rs`.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;

fn make_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        name: name.to_string(),
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

fn field(name: &str) -> FieldName {
    FieldName::new(name).expect("valid field name")
}

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "subgraph_formation_in_campaign_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

async fn full_history(store: &SqliteWaypointStore, thread: &ThreadId) -> Vec<Waypoint> {
    let summaries = store
        .history(thread, None, None)
        .await
        .expect("history should succeed");
    let mut waypoints = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let wp = store
            .get(thread, &summary.waypoint_id)
            .await
            .expect("get should succeed")
            .expect("summary's own waypoint must exist");
        waypoints.push(wp);
    }
    waypoints.sort_by_key(|w| w.superstep);
    waypoints
}

/// A no-op routing node: contributes no delta. Its only job is to be the
/// graph's single entry point so the two OUTGOING edges below it (one to
/// `other_arm`, one to the embedded Formation) are a REAL branch decided by
/// the `route` field's schema-DEFAULT value, not a linear wrapper.
struct RouterNode;

#[async_trait::async_trait]
impl StateNode for RouterNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        Ok(StateDelta::new().into())
    }
}

/// The branch NOT taken in every test below: if this ever ran, it would
/// write `other_out` -- a test asserting `other_out` stays unset is
/// asserting this node's incoming edge was proven `NotFiring`, not merely
/// "happened not to run".
struct OtherArmNode {
    field: FieldName,
}

#[async_trait::async_trait]
impl StateNode for OtherArmNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<Directive, NodeError> {
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), serde_json::json!("other-ran"));
        Ok(delta.into())
    }
}

fn input_field() -> FieldName {
    field("input")
}

fn output_field() -> FieldName {
    field("output")
}

fn initial_delta(seed_value: &str) -> StateDelta {
    let mut delta = StateDelta::new();
    delta.set(field("seed"), seed_value).unwrap();
    delta
}

/// Build the CF-FR-17 fixture: a branching parent graph (`router` ->
/// `other_arm` | `sub`, `route`'s schema default always selects the `sub`
/// arm) whose `sub` node embeds `from_formation(paladins)` unchanged
/// (`Arc::new(...)`, D-22 -- the bridge is never modified), mapping the
/// parent's `seed` field into the child's `input` and the child's `output`
/// back out as the parent's `parent_out`.
///
/// Returns the graph and the three node ids callers need to reference
/// (`router`, `other_arm`, `sub`) -- most usefully `sub`, whose `NodeId` is
/// exactly what `ThreadId::child` needs to compute the SAME derived child
/// thread id production uses.
fn build_branching_parent(paladins: Vec<Paladin>) -> (WarGraph, NodeId, NodeId, NodeId) {
    let route = field("route");
    let seed = field("seed");
    let other_out = field("other_out");
    let parent_out = field("parent_out");

    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(
            route.clone(),
            DispatchRule::LastWrite,
            Some(serde_json::json!("formation")),
            false,
        ),
        FieldSpec::new(seed.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(other_out.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(parent_out.clone(), DispatchRule::LastWrite, None, false),
    ]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());

    let router = NodeId::new("router");
    let other_arm = NodeId::new("other_arm");
    let sub = NodeId::new("sub");

    graph.add_node(router.clone(), NodeSpec::Function(Arc::new(RouterNode)));
    graph.add_node(
        other_arm.clone(),
        NodeSpec::Function(Arc::new(OtherArmNode {
            field: other_out.clone(),
        })),
    );

    // D-22: `from_formation` embeds unchanged via `Arc::new(...)`.
    let formation_graph = Arc::new(WarGraph::from_formation(paladins));
    let state_map = StateMap::new()
        .with_input(seed.clone(), input_field())
        .with_output(output_field(), parent_out.clone());
    graph.add_node(sub.clone(), NodeSpec::battalion(formation_graph, state_map));

    graph.add_edge(EdgeSpec {
        from: router.clone(),
        to: other_arm.clone(),
        condition: Some(EdgeCondition::Contains("\"route\":\"other\"".to_string())),
    });
    graph.add_edge(EdgeSpec {
        from: router.clone(),
        to: sub.clone(),
        condition: Some(EdgeCondition::Contains(
            "\"route\":\"formation\"".to_string(),
        )),
    });
    graph.add_entry(router.clone());

    (graph, router, other_arm, sub)
}

#[tokio::test]
async fn formation_subgraph_runs_as_a_node_of_a_branching_parent_graph() {
    let (graph, _router, _other_arm, _sub) = build_branching_parent(vec![
        make_paladin("form-a"),
        make_paladin("form-b"),
        make_paladin("form-c"),
    ]);

    let port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(port.clone(), Arc::new(InMemoryWaypointStore::new()));
    let thread = ThreadId::new("cf-fr-17-branching-formation").unwrap();
    let outcome = engine
        .start(&graph, thread.clone(), initial_delta("seed-value"))
        .await
        .unwrap();

    let expected_a = "FaultyPaladinPort: form-a processed seed-value".to_string();
    let expected_b = format!("FaultyPaladinPort: form-b processed {expected_a}");
    let expected_c = format!("FaultyPaladinPort: form-c processed {expected_b}");

    match outcome {
        RunOutcome::Completed { final_state, .. } => {
            assert_eq!(
                final_state.get::<String>(&field("parent_out")).unwrap(),
                Some(expected_c),
                "the Formation's final output must reach the parent through the mapped \
                 state_map.outputs field"
            );
            assert_eq!(
                final_state.get::<String>(&field("other_out")).unwrap(),
                None,
                "the branch not taken (other_arm) must never run -- its incoming edge is \
                 proven NotFiring, not merely skipped"
            );
        }
        other => panic!("expected Completed, got {other:?}"),
    }

    // The Formation's three nodes ran in their sequential order, each
    // receiving exactly the previous node's output (ENG-FR-19's Formation
    // semantics, unchanged by embedding).
    assert_eq!(
        port.execution_log(),
        vec![
            "form-a: seed-value".to_string(),
            format!("form-b: {expected_a}"),
            format!("form-c: {expected_b}"),
        ]
    );
}

#[tokio::test]
async fn killing_after_the_childs_first_superstep_and_resuming_does_not_repeat_child_work() {
    tokio::time::timeout(Duration::from_secs(30), async {
        // --- Control run: uninterrupted, start to finish. -----------------
        let control_url = temp_db_url("control");
        let control_store = Arc::new(
            SqliteWaypointStore::new(&control_url)
                .await
                .expect("control store should connect"),
        );
        let control_port = Arc::new(FaultyPaladinPort::new());
        let (control_graph, _router, _other_arm, sub) = build_branching_parent(vec![
            make_paladin("kf-a"),
            make_paladin("kf-b"),
            make_paladin("kf-c"),
        ]);
        let control_thread = ThreadId::new("cf-fr-17-kill-resume").unwrap();
        let control_engine = WarEngine::new(control_port.clone(), control_store.clone());

        let control_outcome = control_engine
            .start(
                &control_graph,
                control_thread.clone(),
                initial_delta("kill-seed"),
            )
            .await
            .expect("control run should succeed");
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        // The child's real thread id -- the SAME derivation
        // `execute_vanguard_node`'s `NodeSpec::Battalion` dispatch arm uses
        // in production.
        let child_thread = ThreadId::child(&control_thread, &sub).expect("valid child thread id");

        let parent_waypoints = full_history(&control_store, &control_thread).await;
        let child_waypoints = full_history(&control_store, &child_thread).await;

        // Sanity: the parent takes exactly 2 supersteps (router, then the
        // whole embedded child run inside `sub`'s single dispatch); the
        // 3-Paladin Formation child takes exactly 3 of its OWN supersteps.
        assert_eq!(
            parent_waypoints.len(),
            2,
            "parent: router's superstep, then sub's superstep"
        );
        assert_eq!(
            child_waypoints.len(),
            3,
            "child: one superstep per Formation paladin"
        );
        assert!(matches!(
            parent_waypoints[0].status,
            WaypointStatus::Running
        ));
        assert!(matches!(child_waypoints[0].status, WaypointStatus::Running));

        // --- Interrupted run: seed a fresh backend with ONLY the parent's
        // FIRST superstep Waypoint (router ran, vanguard = [sub]) and the
        // child's FIRST superstep Waypoint (kf-a ran, vanguard = [f0001]).
        // The parent's OWN superstep-2 Waypoint (which would mark the run
        // Completed) is deliberately NOT seeded -- it is never written
        // until the WHOLE recursive child run inside `sub`'s dispatch
        // returns, so a crash mid-child never produces it either.
        let crashed_url = temp_db_url("crashed");
        {
            let seed_store = SqliteWaypointStore::new(&crashed_url)
                .await
                .expect("seed store should connect");
            seed_store
                .save(&parent_waypoints[0])
                .await
                .expect("seed parent waypoint save should succeed");
            seed_store
                .save(&child_waypoints[0])
                .await
                .expect("seed child waypoint save should succeed");
            // seed_store (and its pool) dropped here.
        }

        // A brand new WarEngine and a brand new SqliteWaypointStore, against
        // the SAME database file the seed store just wrote to.
        let resumed_store = Arc::new(
            SqliteWaypointStore::new(&crashed_url)
                .await
                .expect("resumed store should reconnect to the same file"),
        );
        let resumed_port = Arc::new(FaultyPaladinPort::new());
        let (resumed_graph, ..) = build_branching_parent(vec![
            make_paladin("kf-a"),
            make_paladin("kf-b"),
            make_paladin("kf-c"),
        ]);
        let resumed_engine = WarEngine::new(resumed_port.clone(), resumed_store.clone());

        let resumed_outcome = resumed_engine
            .resume(&resumed_graph, control_thread.clone())
            .await
            .expect("resume should succeed");
        let resumed_final = match resumed_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected resumed run to complete, got {other:?}"),
        };

        // (a): the already-completed child node (kf-a) does not re-execute
        // -- checked against a FRESH port whose own log starts empty, so
        // any re-execution would show up as a new entry.
        let resumed_log = resumed_port.execution_log();
        assert!(
            !resumed_log.iter().any(|e| e.starts_with("kf-a:")),
            "clause (a) violated: kf-a completed before the drop but was called again \
             post-resume (log: {resumed_log:?})"
        );

        // (b): the remaining child nodes DO run.
        assert!(
            resumed_log.iter().any(|e| e.starts_with("kf-b:")),
            "clause (b) violated: kf-b must run post-resume (log: {resumed_log:?})"
        );
        assert!(
            resumed_log.iter().any(|e| e.starts_with("kf-c:")),
            "clause (b) violated: kf-c must run post-resume (log: {resumed_log:?})"
        );

        // (c): the resumed final Battlefield equals the uninterrupted
        // control run's.
        assert_eq!(
            resumed_final, control_final,
            "clause (c) violated: resumed final Battlefield must equal the control run's"
        );
    })
    .await
    .expect("CF-FR-17 kill/resume scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn phalanx_and_campaign_bridges_also_embed() {
    // D-22: `from_phalanx` embeds unchanged, exactly like `from_formation`.
    let phalanx_graph = Arc::new(WarGraph::from_phalanx(vec![
        make_paladin("phal-embed-1"),
        make_paladin("phal-embed-2"),
    ]));
    let mut parent_phalanx = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let sub_phalanx = NodeId::new("phalanx_sub");
    parent_phalanx.add_node(
        sub_phalanx.clone(),
        NodeSpec::battalion(phalanx_graph, StateMap::new()),
    );
    parent_phalanx.add_entry(sub_phalanx);
    parent_phalanx
        .validate(
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
        )
        .expect("from_phalanx must embed as a validating NodeSpec::Battalion child");

    // D-22: `from_campaign` embeds unchanged too -- a minimal one-node
    // Campaign is enough to prove construction and validation; the deep
    // exercise (branching, resume-mid-child) is `from_formation`'s job
    // above.
    let mut campaign = Campaign::new(BattalionConfig::new("cf-fr-17-embed-campaign"));
    let entry = campaign.add_paladin(make_paladin("camp-embed"));
    campaign.set_entry_point(entry).unwrap();
    let campaign_graph = Arc::new(WarGraph::from_campaign(&campaign));
    let mut parent_campaign =
        WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let sub_campaign = NodeId::new("campaign_sub");
    parent_campaign.add_node(
        sub_campaign.clone(),
        NodeSpec::battalion(campaign_graph, StateMap::new()),
    );
    parent_campaign.add_entry(sub_campaign);
    parent_campaign
        .validate(
            &CustomDispatchResolver::new(),
            &EdgeEvaluatorRegistry::new(),
        )
        .expect("from_campaign must embed as a validating NodeSpec::Battalion child");
}

// --- Plan 24-07, Task 2: subgraph forks never share child Waypoints
// (HITL-03, D-18). RED-STATE MARKER: at this commit, the
// `NodeSpec::Battalion` dispatch arm in `superstep.rs` always derives the
// child thread via `ThreadId::child`, ignoring a run's own `fork_of` --
// these tests fail (assertion failures, not compile errors, since
// `WarEngine::fork`/`ThreadId::child_on_branch` already exist from earlier
// plans) until the GREEN commit wires `ThreadId::child_on_branch` in for a
// run entered from a branch.

/// Read every Waypoint of `thread` back out of a `&dyn WaypointPort`,
/// oldest first -- a `dyn`-erased sibling of this file's own `full_history`
/// (which is typed for `&SqliteWaypointStore`) so these `InMemoryWaypointStore`-
/// backed tests can read an arbitrary derived thread id (a mainline child,
/// or a branch-scoped one) through the same port reference.
async fn full_history_dyn(port: &dyn WaypointPort, thread: &ThreadId) -> Vec<Waypoint> {
    let summaries = port
        .history(thread, None, None)
        .await
        .expect("history should succeed");
    let mut waypoints = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let wp = port
            .get(thread, &summary.waypoint_id)
            .await
            .expect("get should succeed")
            .expect("summary's own waypoint must exist");
        waypoints.push(wp);
    }
    waypoints.sort_by_key(|w| w.superstep);
    waypoints
}

#[tokio::test]
async fn fork_child_thread_differs_from_mainline_child_thread() {
    let (graph, _router, _other_arm, sub) = build_branching_parent(vec![
        make_paladin("fork-a"),
        make_paladin("fork-b"),
        make_paladin("fork-c"),
    ]);
    let store = Arc::new(InMemoryWaypointStore::new());
    let port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(port.clone(), store.clone());
    let thread = ThreadId::new("cf-fr-17-fork-child-thread-differs").unwrap();

    engine
        .start(&graph, thread.clone(), initial_delta("fork-seed"))
        .await
        .expect("control run should succeed");

    // router's own Waypoint: vanguard = [sub], not yet run.
    let parent_waypoints = full_history_dyn(store.as_ref(), &thread).await;
    let from = parent_waypoints[0].waypoint_id;

    let outcome = engine
        .fork(&graph, &thread, from, StateDelta::new())
        .await
        .expect("fork should succeed");
    match outcome {
        RunOutcome::Completed { .. } => {}
        other => panic!("expected Completed, got {other:?}"),
    }

    let mainline_child_thread = ThreadId::child(&thread, &sub).expect("valid child thread id");
    let branch_child_thread =
        ThreadId::child_on_branch(&thread, &from, &sub).expect("valid branch child thread id");
    assert_ne!(
        mainline_child_thread.as_str(),
        branch_child_thread.as_str(),
        "a fork's subgraph child must run under a distinct, branch-scoped thread id"
    );

    let branch_child_waypoints = full_history_dyn(store.as_ref(), &branch_child_thread).await;
    assert_eq!(
        branch_child_waypoints.len(),
        3,
        "the branch's own child (a 3-Paladin Formation) must have run, under its own \
         branch-scoped thread, fully fresh"
    );
}

#[tokio::test]
async fn fork_does_not_touch_mainline_child_waypoints() {
    let (graph, _router, _other_arm, sub) = build_branching_parent(vec![
        make_paladin("keep-a"),
        make_paladin("keep-b"),
        make_paladin("keep-c"),
    ]);
    let store = Arc::new(InMemoryWaypointStore::new());
    let port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(port.clone(), store.clone());
    let thread = ThreadId::new("cf-fr-17-fork-keeps-mainline-child").unwrap();

    engine
        .start(&graph, thread.clone(), initial_delta("keep-seed"))
        .await
        .expect("control run should succeed");

    let mainline_child_thread = ThreadId::child(&thread, &sub).expect("valid child thread id");
    let before = full_history_dyn(store.as_ref(), &mainline_child_thread).await;
    let before_bytes: Vec<String> = before
        .iter()
        .map(|w| serde_json::to_string(w).unwrap())
        .collect();
    assert_eq!(before_bytes.len(), 3);

    let parent_waypoints = full_history_dyn(store.as_ref(), &thread).await;
    let from = parent_waypoints[0].waypoint_id;
    engine
        .fork(&graph, &thread, from, StateDelta::new())
        .await
        .expect("fork should succeed");

    let after = full_history_dyn(store.as_ref(), &mainline_child_thread).await;
    let after_bytes: Vec<String> = after
        .iter()
        .map(|w| serde_json::to_string(w).unwrap())
        .collect();
    assert_eq!(
        before_bytes, after_bytes,
        "the mainline child's Waypoints must be unchanged in count and in serialised bytes \
         after a fork"
    );
}

#[tokio::test]
async fn latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child() {
    let (graph, _router, _other_arm, sub) = build_branching_parent(vec![
        make_paladin("latest-a"),
        make_paladin("latest-b"),
        make_paladin("latest-c"),
    ]);
    let store = Arc::new(InMemoryWaypointStore::new());
    let port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(port.clone(), store.clone());
    let thread = ThreadId::new("cf-fr-17-fork-latest-child").unwrap();

    engine
        .start(&graph, thread.clone(), initial_delta("latest-seed"))
        .await
        .expect("control run should succeed");

    let mainline_child_thread = ThreadId::child(&thread, &sub).expect("valid child thread id");
    let mainline_latest = store
        .latest(&mainline_child_thread)
        .await
        .expect("latest should succeed")
        .expect("mainline child must have a latest Waypoint");

    let parent_waypoints = full_history_dyn(store.as_ref(), &thread).await;
    let from = parent_waypoints[0].waypoint_id;
    engine
        .fork(&graph, &thread, from, StateDelta::new())
        .await
        .expect("fork should succeed");

    let branch_child_thread =
        ThreadId::child_on_branch(&thread, &from, &sub).expect("valid branch child thread id");
    let branch_latest = store
        .latest(&branch_child_thread)
        .await
        .expect("latest should succeed")
        .expect("branch child must have a latest Waypoint");

    assert_ne!(
        branch_latest.waypoint_id, mainline_latest.waypoint_id,
        "latest on the branch's own child thread must never resolve the mainline child's"
    );

    let mainline_latest_after = store
        .latest(&mainline_child_thread)
        .await
        .expect("latest should succeed")
        .expect("mainline child must still have a latest Waypoint");
    assert_eq!(
        mainline_latest_after.waypoint_id, mainline_latest.waypoint_id,
        "the mainline child's own latest must be unaffected by the fork"
    );
}

#[tokio::test]
async fn mainline_runs_keep_using_child() {
    let (graph, _router, _other_arm, sub) = build_branching_parent(vec![
        make_paladin("mainline-a"),
        make_paladin("mainline-b"),
        make_paladin("mainline-c"),
    ]);
    let store = Arc::new(InMemoryWaypointStore::new());
    let port = Arc::new(FaultyPaladinPort::new());
    let engine = WarEngine::new(port.clone(), store.clone());
    let thread = ThreadId::new("cf-fr-17-mainline-keeps-child").unwrap();

    engine
        .start(&graph, thread.clone(), initial_delta("mainline-seed"))
        .await
        .expect("run should succeed");

    let mainline_child_thread = ThreadId::child(&thread, &sub).expect("valid child thread id");
    let waypoints = full_history_dyn(store.as_ref(), &mainline_child_thread).await;
    assert_eq!(
        waypoints.len(),
        3,
        "a plain (non-forked) run's subgraph child must still run under the unchanged \
         ThreadId::child derivation (regression guard, D-18)"
    );
}
