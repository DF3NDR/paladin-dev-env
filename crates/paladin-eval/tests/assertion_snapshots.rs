//! Freezes every OBS-FR-12 assertion kind's failure rendering with `insta`
//! (D-29, PRD 07 §3 acceptance 5). Twelve deliberately failing evaluations
//! against one fixed, branching, retrying run -- a `NodeStarted` sequence
//! that branches (`planner` -> `worker`, `planner` -> `reviewer` evaluated
//! but not fired), retries (`worker` attempted twice), raises a Parley, and
//! finishes -- prove the exact wording of `AssertionFailure::render_failure`
//! is part of the contract, not an accident of the implementation. A change
//! to that wording is a visible diff here.
//!
//! Fixture values that would vary between machines or runs (timestamps,
//! `ParleyId`, `WaypointId`) are never rendered by `render_failure`, so the
//! snapshots stay reproducible without an `insta` filter.

use chrono::Utc;
use paladin_battalion::engine::RunOutcome;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, CustomDispatchResolver, DispatchRule, FieldName, FieldSpec,
    StateDelta,
};
use paladin_core::platform::container::parley::{ParleyId, ParleyKind};
use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, ThreadId, WaypointId};
use paladin_eval::scenario::{Assertion, RunStatusValue, Times};
use paladin_eval::{AssertionContext, AssertionOutcome, evaluate};

fn thread_id() -> ThreadId {
    ThreadId::new("eval-fixture").expect("valid thread id")
}

fn record(seq: u64, event: TraceEvent) -> TraceRecord {
    TraceRecord {
        thread_id: thread_id(),
        run_id: None,
        seq,
        at: Utc::now(),
        event,
    }
}

/// A fixed, branching, retrying run: `planner` starts and finishes, its
/// `worker` edge fires and its `reviewer` edge is evaluated but does not
/// fire, `worker` is attempted twice (fails, then succeeds), a Parley is
/// raised, and the run finishes -- one fixture exercising every OBS-FR-12
/// evaluator's evidence shape.
fn fixture_records() -> Vec<TraceRecord> {
    vec![
        record(
            1,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".to_string(),
            },
        ),
        record(
            2,
            TraceEvent::SuperstepStarted {
                superstep: 1,
                vanguard: vec![NodeId::new("planner")],
            },
        ),
        record(
            3,
            TraceEvent::NodeStarted {
                superstep: 1,
                node_id: NodeId::new("planner"),
                attempt: 1,
                muster_task_key: None,
            },
        ),
        record(
            4,
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("planner"),
                to: NodeId::new("worker"),
                condition_kind: "always".to_string(),
                fired: true,
            },
        ),
        record(
            5,
            TraceEvent::EdgeEvaluated {
                from: NodeId::new("planner"),
                to: NodeId::new("reviewer"),
                condition_kind: "contains".to_string(),
                fired: false,
            },
        ),
        record(
            6,
            TraceEvent::NodeFinished {
                superstep: 1,
                node_id: NodeId::new("planner"),
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                token_count: 10,
                cache_hit: false,
            },
        ),
        record(
            7,
            TraceEvent::SuperstepStarted {
                superstep: 2,
                vanguard: vec![NodeId::new("worker")],
            },
        ),
        record(
            8,
            TraceEvent::NodeStarted {
                superstep: 2,
                node_id: NodeId::new("worker"),
                attempt: 1,
                muster_task_key: None,
            },
        ),
        record(
            9,
            TraceEvent::NodeFinished {
                superstep: 2,
                node_id: NodeId::new("worker"),
                attempt: 1,
                outcome: NodeOutcomeKind::Failed,
                duration_ms: 8,
                token_count: 5,
                cache_hit: false,
            },
        ),
        record(
            10,
            TraceEvent::NodeStarted {
                superstep: 2,
                node_id: NodeId::new("worker"),
                attempt: 2,
                muster_task_key: None,
            },
        ),
        record(
            11,
            TraceEvent::NodeFinished {
                superstep: 2,
                node_id: NodeId::new("worker"),
                attempt: 2,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 6,
                token_count: 7,
                cache_hit: false,
            },
        ),
        record(
            12,
            TraceEvent::ParleyRaised {
                parley_id: ParleyId::new(),
                node_id: NodeId::new("worker"),
                parley_kind: ParleyKind::Approval,
            },
        ),
        record(
            13,
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 2,
                total_tokens: 22,
                duration_ms: 20,
                trace_dropped_total: 0,
            },
        ),
    ]
}

fn fixture_battlefield() -> Battlefield {
    let status = FieldName::new("status").expect("valid field name");
    let count = FieldName::new("count").expect("valid field name");
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(status.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(count.clone(), DispatchRule::LastWrite, None, false),
    ]);
    let mut battlefield = Battlefield::new(schema);
    let mut delta = StateDelta::new();
    delta.set(status, "in_progress").expect("sets status");
    delta.set(count, 3_i64).expect("sets count");
    battlefield
        .merge(
            vec![(NodeId::new("worker"), delta)],
            2,
            &CustomDispatchResolver::new(),
        )
        .expect("merge succeeds");
    battlefield
}

fn fixture_outcome(battlefield: &Battlefield) -> RunOutcome {
    RunOutcome::Completed {
        final_state: battlefield.clone(),
        waypoint: WaypointId::generate(),
    }
}

/// Evaluate `assertion` against `ctx`, snapshot its `render_failure()` under
/// `name`, and fail the test loudly (not silently) if the fixture was built
/// wrong and the assertion unexpectedly passed.
fn assert_failure_snapshot(name: &str, assertion: &Assertion, ctx: &AssertionContext<'_>) {
    match evaluate(assertion, ctx) {
        AssertionOutcome::Failed(failure) => {
            insta::assert_snapshot!(name, failure.render_failure());
        }
        AssertionOutcome::Passed => {
            panic!("expected assertion {name} to fail against the fixture, but it passed");
        }
    }
}

#[test]
fn final_state_field_equals_failure_renders_expected_and_observed() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "final_state_field_equals",
        &Assertion::FinalStateFieldEquals {
            field: "status".to_string(),
            value: serde_json::json!("done"),
        },
        &ctx,
    );
}

#[test]
fn final_state_field_matches_failure_renders_the_pattern() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "final_state_field_matches",
        &Assertion::FinalStateFieldMatches {
            field: "status".to_string(),
            pattern: "^done$".to_string(),
        },
        &ctx,
    );
}

#[test]
fn field_json_path_equals_failure_renders_the_path() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "field_json_path_equals",
        &Assertion::FieldJsonPathEquals {
            path: "/count".to_string(),
            value: serde_json::json!(99),
        },
        &ctx,
    );
}

/// The `node_executed` snapshot shows the visit table.
#[test]
fn node_executed_failure_renders_the_visit_table() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "node_executed",
        &Assertion::NodeExecuted {
            node: "worker".to_string(),
            times: Times::Exact(5),
        },
        &ctx,
    );
}

#[test]
fn node_not_executed_failure_renders_the_visit_table() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "node_not_executed",
        &Assertion::NodeNotExecuted {
            node: "planner".to_string(),
        },
        &ctx,
    );
}

/// The `edge_fired` snapshot shows the evaluated-but-not-fired wording.
#[test]
fn edge_fired_failure_renders_evaluated_but_not_fired() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "edge_fired",
        &Assertion::EdgeFired {
            from: "planner".to_string(),
            to: "reviewer".to_string(),
        },
        &ctx,
    );
}

/// The `route_taken` snapshot shows the observed route.
#[test]
fn route_taken_failure_renders_the_observed_route() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "route_taken",
        &Assertion::RouteTaken(vec!["worker".to_string(), "planner".to_string()]),
        &ctx,
    );
}

#[test]
fn run_status_failure_renders_expected_and_observed() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "run_status",
        &Assertion::RunStatus(RunStatusValue::Failed),
        &ctx,
    );
}

#[test]
fn total_tokens_max_failure_renders_the_bound_and_total() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot("total_tokens_max", &Assertion::TotalTokensMax(10), &ctx);
}

#[test]
fn supersteps_max_failure_renders_the_bound_and_total() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot("supersteps_max", &Assertion::SuperstepsMax(1), &ctx);
}

#[test]
fn parley_raised_failure_renders_the_observed_parleys() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);
    let ctx = AssertionContext::new(&records, &battlefield, &outcome);

    assert_failure_snapshot(
        "parley_raised",
        &Assertion::ParleyRaised {
            kind: "choice".to_string(),
            node: "worker".to_string(),
        },
        &ctx,
    );
}

/// The `final_state_snapshot` snapshot shows the first differing path,
/// comparing against a blessed file whose content is deliberately
/// mismatched at `/status`. The temp file's content is fixed, but its own
/// path is never rendered by `final_state_snapshot`'s failure message, so
/// this snapshot carries no absolute, machine-dependent path.
#[test]
fn final_state_snapshot_failure_renders_the_first_differing_path() {
    let records = fixture_records();
    let battlefield = fixture_battlefield();
    let outcome = fixture_outcome(&battlefield);

    let path = std::env::temp_dir().join(format!(
        "paladin-eval-assertion-snapshot-test-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, r#"{"count":3,"status":"done"}"#).expect("writes blessed snapshot");

    let ctx =
        AssertionContext::new(&records, &battlefield, &outcome).with_snapshot_path(path.clone());
    assert_failure_snapshot("final_state_snapshot", &Assertion::FinalStateSnapshot, &ctx);

    let _ = std::fs::remove_file(&path);
}
