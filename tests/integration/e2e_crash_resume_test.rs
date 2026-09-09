//! Program acceptance scenario E2E-1 (crash-resume), `.project/v0.10.0/00-program-overview.md`
//! section 6: a 6-node cyclic workflow (one loop with a max-iteration bound) run against a
//! mock LLM with a durable Waypoint backend, interrupted after superstep 3 and resumed by a
//! fresh engine, must (a) never re-execute an already-completed node, (b) reach the same final
//! Battlefield as an uninterrupted control run, and (c) leave exactly one Waypoint per completed
//! superstep. A fourth assertion checks the loop node ran the same number of times in both runs.
//!
//! ## Why the "crash" is simulated by re-seeding a fresh `SqliteWaypointStore` file rather than
//! aborting a live `WarEngine::start` task
//!
//! The superstep loop (`paladin_battalion::engine::superstep::run`) spawns each vanguard node's
//! execution via `tokio::spawn` -- a genuinely detached task. Racing `engine.start(..)` against a
//! "stop once 3 Waypoints exist" signal via `tokio::select!` would drop the `start` future, but
//! any node tasks already spawned for the superstep in flight at that instant are NOT
//! transitively cancelled (they are independent, unlinked tasks) and could keep running in the
//! background, potentially writing a stray Waypoint that races with this test's own "resume"
//! call against the same database file. Since the graph and the mock port are both fully
//! deterministic, the first 3 Waypoints of an uninterrupted control run are byte-for-byte
//! identical to what a live "run and kill after superstep 3" would have produced. Reading those
//! 3 real, durably-persisted Waypoints back out of the control run's own `SqliteWaypointStore`
//! and re-saving them into a second, fresh temporary database file is therefore an equivalent --
//! and deterministic, CI-safe -- way to construct "a thread whose latest Waypoint reflects a
//! process that died right after superstep 3", without the background-task race above.

use std::sync::Arc;
use std::time::Duration;

use paladin_battalion::engine::{RunOutcome, WarEngine};
use paladin_core::platform::container::battlefield::StateDelta;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, Waypoint, WaypointStatus};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this
// standalone [[test]] target needs `FaultyPaladinPort` and the shared
// `e2e_fixtures` graph builders (plan 28-16, D-34) -- the rest of the module
// tree is unused here, allowed rather than pruned.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;
use helpers::e2e_fixtures;

// The E2E-1 fixture: 6 nodes (5 Paladin, 1 Function), one bounded
// self-loop.
//
// `loop_gate` (self-loop, bounded, GRAPH ENTRY) `-> researcher -> writer ->
// reviewer -> finalizer -> archiver`. The loop is deliberately the graph's
// entry point rather than fed by an upstream node: the engine's
// join-readiness rule (Phase 22 Plan 07 `Frontier::is_ready`) requires
// EVERY incoming edge of a node to be resolved (not `Pending`) before that
// node is placed in the next Vanguard. A self-loop edge is Pending until
// its OWN node has executed at least once -- so a node that is BOTH
// self-looping AND fed by a separate upstream edge could never execute at
// all (its self-edge blocks its first run, and its first run is what would
// resolve the self-edge). `loop_gate` itself has no separate upstream feed
// -- it is a standalone self-loop, the simplest instance of the same
// bootstrap problem: with no OTHER incoming edge either, it could never
// take its first turn unless seeded directly into the initial Vanguard as a
// graph entry.
//
// (Phase 22 Plan 16 audit, `22-deferred-items.md`: `loop_gate`'s
// standalone self-loop-as-entry arrangement is the same readiness-dodge
// pattern the Plan 16 audit enumerated by direct reading, not assumed,
// across the fixtures that actually EXECUTE their looping node. One
// exception exists --
// `engine::graph::tests::validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge`
// constructs the harder self-loop-plus-upstream-edge shape this comment
// describes, but only calls `validate`, never runs the graph -- so it never
// reaches `is_ready` and needs no entry-point workaround. The general
// "self-looping AND fed by a separate upstream edge" case this comment
// warns about is BUG-03's cycle-bootstrap starvation defect, registered
// and fixed in Phase 22.1 by `Frontier::starved_release`
// (`engine::superstep`) -- see the now-passing regression tests
// `engine::superstep::tests::self_looping_node_fed_by_upstream_edge_can_never_take_first_turn`
// and
// `engine::superstep::tests::cycle_node_fed_from_outside_the_cycle_takes_its_first_turn`,
// not an ignored reproduction.)
//
// An uninterrupted run takes exactly `LOOP_BOUND + 5` supersteps:
// `loop_gate` x `LOOP_BOUND` (supersteps 1..=LOOP_BOUND), then researcher,
// writer, reviewer, finalizer, archiver (one superstep each).
//
// The graph itself now lives in `tests/helpers/e2e_fixtures.rs` as
// `build_crash_resume_graph`, shared verbatim with the eval harness (plan
// 28-16, D-34). This comment stays here since it documents THIS test's own
// crash-simulation technique, not the graph shape.

fn initial_delta() -> StateDelta {
    let mut delta = StateDelta::new();
    delta
        .set(e2e_fixtures::field("topic"), "rust workflows")
        .unwrap();
    delta
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

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "e2e_crash_resume_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

#[tokio::test]
async fn e2e_1_crash_resume_matches_control_run_with_no_reexecution() {
    tokio::time::timeout(Duration::from_secs(30), async {
        // --- Control run: uninterrupted, start to finish. -----------------
        let control_url = temp_db_url("control");
        let control_store = Arc::new(
            SqliteWaypointStore::new(&control_url)
                .await
                .expect("control store should connect"),
        );
        let control_port = Arc::new(FaultyPaladinPort::new());
        let control_graph = e2e_fixtures::build_crash_resume_graph();
        let control_thread = ThreadId::new("e2e-1-control").expect("valid thread id");
        let control_engine = WarEngine::new(control_port.clone(), control_store.clone());

        let control_outcome = control_engine
            .start(&control_graph, control_thread.clone(), initial_delta())
            .await
            .expect("control run should succeed");
        let control_final = match control_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected control run to complete, got {other:?}"),
        };

        let total_supersteps = (e2e_fixtures::LOOP_BOUND as usize) + 5;
        let control_waypoints = full_history(&control_store, &control_thread).await;
        assert_eq!(
            control_waypoints.len(),
            total_supersteps,
            "the 6-node fixture (LOOP_BOUND loop iterations + 5 straight-line nodes) takes \
             LOOP_BOUND + 5 supersteps"
        );
        let control_loop_runs = control_waypoints
            .iter()
            .flat_map(|wp| wp.completed.iter())
            .filter(|r| r.node_id == NodeId::new("loop_gate"))
            .count();
        assert_eq!(
            control_loop_runs,
            e2e_fixtures::LOOP_BOUND as usize,
            "loop_gate must run exactly LOOP_BOUND times"
        );

        // --- Interrupted run: seed a fresh backend with only the first 3 --
        // Waypoints (see the module doc comment for why this, rather than a
        // live task abort, is the correct simulation here).
        let crashed_url = temp_db_url("crashed");
        {
            let seed_store = SqliteWaypointStore::new(&crashed_url)
                .await
                .expect("seed store should connect");
            for wp in &control_waypoints[0..3] {
                seed_store.save(wp).await.expect("seed save should succeed");
            }
            // seed_store (and its pool) dropped here -- "the engine and its
            // port handles" are gone before the next line runs.
        }

        let already_completed_before_drop: std::collections::HashSet<NodeId> = control_waypoints
            [0..3]
            .iter()
            .flat_map(|wp| wp.completed.iter().map(|r| r.node_id.clone()))
            .collect();
        assert_eq!(
            already_completed_before_drop,
            std::collections::HashSet::from([NodeId::new("loop_gate")]),
            "sanity check: with LOOP_BOUND=5, only loop_gate (mid-loop) has completed as of \
             superstep 3 -- the drop lands inside the loop, not after it"
        );

        // A brand new WarEngine and a brand new SqliteWaypointStore, against
        // the SAME database file the seed store just wrote to.
        let resumed_store = Arc::new(
            SqliteWaypointStore::new(&crashed_url)
                .await
                .expect("resumed store should reconnect to the same file"),
        );
        let resumed_port = Arc::new(FaultyPaladinPort::new());
        let resumed_graph = e2e_fixtures::build_crash_resume_graph();
        let resumed_engine = WarEngine::new(resumed_port.clone(), resumed_store.clone());

        let resumed_outcome = resumed_engine
            .resume(&resumed_graph, control_thread.clone())
            .await
            .expect("resume should succeed");
        let resumed_final = match resumed_outcome {
            RunOutcome::Completed { final_state, .. } => final_state,
            other => panic!("expected resumed run to complete, got {other:?}"),
        };

        // --- (a): no node completed before the drop appears again. --------
        // Every Paladin node's name equals its NodeId in this fixture, so a
        // resumed-run port call whose name matches an already-completed
        // NodeId would be a re-execution. (With LOOP_BOUND=5 the only node
        // completed as of superstep 3 is `loop_gate`, a Function node that
        // never reaches this port at all -- this check is still exercised
        // meaningfully by clause (d)'s exact loop-run-count comparison,
        // which WOULD fail if resume restarted the loop from iteration 1.)
        let resumed_log = resumed_port.execution_log();
        for entry in &resumed_log {
            for node in &already_completed_before_drop {
                assert!(
                    !entry.starts_with(&format!("{}:", node.as_str())),
                    "clause (a) violated: {node} completed before the drop but was called \
                     again post-resume (log entry: {entry})"
                );
            }
        }

        // --- (b): resumed final Battlefield equals the control run's. -----
        assert_eq!(
            resumed_final, control_final,
            "clause (b) violated: resumed final Battlefield must equal the control run's"
        );

        // --- (c): exactly one Waypoint per completed superstep, unbroken --
        // parent chain, across the WHOLE thread (3 seeded + however many
        // resume adds).
        let final_waypoints = full_history(&resumed_store, &control_thread).await;
        assert_eq!(
            final_waypoints.len(),
            total_supersteps,
            "clause (c) violated: expected exactly one Waypoint per completed superstep \
             ({total_supersteps} total: 3 seeded + {} from resume)",
            total_supersteps - 3
        );
        let mut supersteps: Vec<u64> = final_waypoints.iter().map(|w| w.superstep).collect();
        supersteps.sort_unstable();
        assert_eq!(
            supersteps,
            (1..=total_supersteps as u64).collect::<Vec<_>>(),
            "clause (c) violated: superstep numbers must be contiguous with no gap or duplicate"
        );
        for pair in final_waypoints.windows(2) {
            assert_eq!(
                pair[1].parent_waypoint_id,
                Some(pair[0].waypoint_id),
                "clause (c) violated: parent chain must be unbroken between superstep {} and {}",
                pair[0].superstep,
                pair[1].superstep
            );
        }
        assert_eq!(
            final_waypoints.last().unwrap().status,
            WaypointStatus::Completed
        );

        // --- (d): the loop node ran the same number of times in both runs.
        let resumed_loop_runs = final_waypoints
            .iter()
            .flat_map(|wp| wp.completed.iter())
            .filter(|r| r.node_id == NodeId::new("loop_gate"))
            .count();
        assert_eq!(
            resumed_loop_runs, control_loop_runs,
            "clause (d) violated: the loop node must run the same number of times in both runs"
        );
    })
    .await
    .expect("E2E-1 scenario must complete within its 30s timeout guard");
}
