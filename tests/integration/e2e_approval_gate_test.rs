//! Program acceptance scenario E2E-2 (approval gate), `.project/v0.10.0/00-program-overview.md`
//! section 6: an approval gate expressed as one `NodeSpec::Gate` plus a `Contains("true")` edge
//! and a `Contains("false")` edge must route down the correct branch on both approval and
//! denial, across a process drop and recreate over one shared, on-disk `SqliteWaypointStore`
//! file -- the only channel a suspended thread crosses (HITL-01, HITL-02).
//!
//! ## Why the "process drop" is simulated by dropping the engine/store `Arc`s rather than
//! aborting a live task
//!
//! Mirrors `tests/integration/e2e_crash_resume_test.rs`'s own documented technique: engine
//! instance A is constructed, run to suspension, and dropped (its `Arc<SqliteWaypointStore>`
//! goes out of scope) before engine instance B is constructed over the SAME database file path
//! and used to `resume_with`. No in-memory state crosses the drop -- the persisted `AwaitingInput`
//! Waypoint is the only channel, which is exactly the HITL-01 claim this scenario proves.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use paladin_battalion::engine::{NodeSpec, RunOutcome, WarEngine};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{FieldName, StateDelta};
use paladin_core::platform::container::parley::{ParleyKind, ParleyResponse};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, WaypointStatus};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

// `tests/helpers/` is shared across many integration test binaries; this
// standalone [[test]] target needs `FaultyPaladinPort` and the shared
// `e2e_fixtures` graph builders (plan 28-16, D-34) -- the rest of the module
// tree is unused here, allowed rather than pruned, following
// `e2e_crash_resume_test.rs`'s own precedent for this exact situation.
#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;
use helpers::e2e_fixtures;

// The E2E-2 fixture -- one `NodeSpec::Gate` (`Approval`, `output_field: "approved"`) plus a
// `Contains("true")` edge to `act` and a `Contains("false")` edge to `cancel`, the exact
// "three lines of graph" shape the PRD promises -- now lives in
// `tests/helpers/e2e_fixtures.rs` as `build_approval_gate_graph`, shared verbatim with the
// eval harness (plan 28-16, D-34).

fn temp_db_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "e2e_approval_gate_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    format!("sqlite://{}", path.display())
}

/// Runs the E2E-2 scenario end to end: engine instance A suspends at the gate over a fresh
/// on-disk `SqliteWaypointStore`, is dropped, and a brand new engine instance B -- constructed
/// over the SAME database file -- delivers `approved_value` and drives the run to completion.
/// Returns the final `path` field's value (`"act"` or `"cancel"`).
async fn run_gate_scenario(label: &str, approved_value: serde_json::Value) -> String {
    let db_url = temp_db_url(label);
    let graph = e2e_fixtures::build_approval_gate_graph();
    let thread = ThreadId::new(format!("e2e-2-{label}")).expect("valid thread id");

    // --- Instance A: start, suspend, then drop (simulated process death). --
    let parley_id = {
        let store_a = Arc::new(
            SqliteWaypointStore::new(&db_url)
                .await
                .expect("store A should connect"),
        );
        let port_a = Arc::new(FaultyPaladinPort::new());
        let engine_a = WarEngine::new(port_a, store_a);

        let outcome = engine_a
            .start(&graph, thread.clone(), StateDelta::new())
            .await
            .expect("start should suspend at the gate");
        match outcome {
            RunOutcome::AwaitingInput { parleys, .. } => {
                assert_eq!(parleys.len(), 1, "exactly one Gate node parleys");
                parleys[0].parley_id
            }
            other => panic!("expected AwaitingInput, got {other:?}"),
        }
        // engine_a and its Arc<SqliteWaypointStore> are dropped here --
        // instance A's process is simulated as gone before instance B is
        // constructed below, over the same file.
    };

    // --- Instance B: a brand new engine AND a brand new store, same file. --
    let store_b = Arc::new(
        SqliteWaypointStore::new(&db_url)
            .await
            .expect("store B should reconnect to the same file"),
    );
    let port_b = Arc::new(FaultyPaladinPort::new());
    let engine_b = WarEngine::new(port_b, store_b);

    let response = ParleyResponse {
        parley_id,
        // `kind`/`prompt` are stamped over by `resume_with` regardless of
        // what is submitted here -- never observed.
        kind: ParleyKind::Approval,
        prompt: String::new(),
        value: approved_value,
        responded_by: Some("tester".to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    };

    let resumed = engine_b
        .resume_with(&graph, thread, vec![response])
        .await
        .expect("resume_with should complete the run");
    match resumed {
        RunOutcome::Completed { final_state, .. } => final_state
            .get::<String>(&e2e_fixtures::field("path"))
            .expect("path field should read")
            .expect("path field must be set by the branch that fired"),
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn e2e2_approval_branch_survives_process_drop() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let path = run_gate_scenario("approval", serde_json::json!(true)).await;
        assert_eq!(
            path, "act",
            "an approved gate must route to the action branch"
        );
    })
    .await
    .expect("E2E-2 approval scenario must complete within its 30s timeout guard");
}

#[tokio::test]
async fn e2e2_denial_branch_survives_process_drop() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let path = run_gate_scenario("denial", serde_json::json!(false)).await;
        assert_eq!(
            path, "cancel",
            "a denied gate must route to the cancellation branch"
        );
    })
    .await
    .expect("E2E-2 denial scenario must complete within its 30s timeout guard");
}

/// Between suspension and resume, no task, timer or connection belonging to the run is
/// retained by instance A before it is dropped: the run's own returned outcome is
/// `RunOutcome::AwaitingInput`, and no further Waypoint is written to the shared file while the
/// thread sits suspended, even after giving any stray background work a chance to run.
#[tokio::test]
async fn e2e2_suspended_thread_holds_no_engine_resources() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let db_url = temp_db_url("no-resources");
        let graph = e2e_fixtures::build_approval_gate_graph();
        let thread = ThreadId::new("e2e-2-no-resources").expect("valid thread id");

        {
            let store_a = Arc::new(
                SqliteWaypointStore::new(&db_url)
                    .await
                    .expect("store A should connect"),
            );
            let port_a = Arc::new(FaultyPaladinPort::new());
            let engine_a = WarEngine::new(port_a, Arc::clone(&store_a));

            let outcome = engine_a
                .start(&graph, thread.clone(), StateDelta::new())
                .await
                .expect("start should suspend at the gate");
            assert!(
                matches!(outcome, RunOutcome::AwaitingInput { .. }),
                "the run's own returned outcome must be AwaitingInput, got {outcome:?}"
            );

            let history_at_suspension = store_a
                .history(&thread, None, None)
                .await
                .expect("history should succeed");
            assert_eq!(
                history_at_suspension.len(),
                1,
                "exactly one Waypoint persisted at suspension"
            );

            // Give any stray background task/timer a chance to run before
            // instance A is dropped.
            tokio::time::sleep(Duration::from_millis(200)).await;

            let history_after_wait = store_a
                .history(&thread, None, None)
                .await
                .expect("history should succeed");
            assert_eq!(
                history_after_wait.len(),
                1,
                "no further Waypoint may be written while the thread sits suspended -- a \
                 second write here would mean the engine retained a task/timer for this run"
            );

            // engine_a and store_a are dropped here.
        }

        // A brand new store handle on the SAME file sees exactly the same
        // single Waypoint -- the suspension state lives entirely on disk,
        // not in any resource instance A retained.
        let store_fresh = SqliteWaypointStore::new(&db_url)
            .await
            .expect("fresh store should reconnect to the same file");
        let history = store_fresh
            .history(&thread, None, None)
            .await
            .expect("history should succeed");
        assert_eq!(history.len(), 1);
        let latest = store_fresh
            .latest(&thread)
            .await
            .expect("latest should succeed")
            .expect("latest waypoint must exist");
        assert!(
            matches!(latest.status, WaypointStatus::AwaitingInput { .. }),
            "the persisted Waypoint alone must still show AwaitingInput"
        );
    })
    .await
    .expect("no-engine-resources scenario must complete within its 30s timeout guard");
}

/// The approval gate under test is expressed as exactly one `NodeSpec::Gate` plus a
/// `Contains("true")` edge and a `Contains("false")` edge -- the shape the PRD promises.
#[test]
fn e2e2_graph_is_three_lines_of_graph() {
    let graph = e2e_fixtures::build_approval_gate_graph();

    match graph.node(&NodeId::new("approve")) {
        Some(NodeSpec::Gate {
            request,
            output_field,
        }) => {
            assert_eq!(request.kind, ParleyKind::Approval);
            assert_eq!(
                output_field.as_ref().map(FieldName::as_str),
                Some("approved")
            );
        }
        other => panic!("expected NodeSpec::Gate for \"approve\", got {other:?}"),
    }

    let edges = graph.edges();
    assert_eq!(
        edges.len(),
        2,
        "an approval gate needs exactly two edges: one per branch"
    );
    assert!(
        edges.iter().any(|e| {
            e.from == NodeId::new("approve")
                && e.to == NodeId::new("act")
                && e.condition == Some(EdgeCondition::Contains(r#""approved":true"#.to_string()))
        }),
        "missing the Contains(\"true\") edge to the action branch"
    );
    assert!(
        edges.iter().any(|e| {
            e.from == NodeId::new("approve")
                && e.to == NodeId::new("cancel")
                && e.condition == Some(EdgeCondition::Contains(r#""approved":false"#.to_string()))
        }),
        "missing the Contains(\"false\") edge to the cancellation branch"
    );
}
