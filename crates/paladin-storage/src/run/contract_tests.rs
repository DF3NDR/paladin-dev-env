//! Shared `RunRepositoryPort` contract suite (D-04, D-17, D-18).
//!
//! One generic async function per contract clause, each taking `&dyn
//! RunRepositoryPort` (or, for the concurrency stress test, `Arc<dyn
//! RunRepositoryPort>`) and asserting inside. Every backend
//! (`InMemoryRunRepository`, `SqliteRunRepository`, `PostgresRunRepository`)
//! invokes these unchanged from its own `#[tokio::test]`s, so "identical
//! suite across backends" is enforced by construction rather than by
//! convention, mirroring `crate::waypoint::contract_tests`'s house pattern
//! (D-09 precedent). Named per-clause (not a declarative macro) so a failure
//! names the violated contract clause rather than a line number.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use paladin_core::platform::container::assistant::AssistantId;
use paladin_core::platform::container::parley::{ParleyId, ParleyKind, ParleyResponse};
use paladin_core::platform::container::run::{
    AssistantRef, ForkSpec, Run, RunEventKind, RunId, RunStatus, WebhookSpec,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunQuery, RunRepositoryError, RunRepositoryPort,
};

use crate::assistant::contract_tests::sample_new_version as sample_assistant_new_version;

/// Build a `Run` fixture for `thread`/`assistant_id`, stamped with the given
/// `submitted_at`. Every contract function should build fixtures through
/// this function so all backends exercise identical inputs.
pub fn sample_run(thread: &ThreadId, assistant_id: &str, submitted_at: DateTime<Utc>) -> Run {
    let mut run = Run::new(
        RunId::new_v7(),
        thread.clone(),
        AssistantRef {
            assistant_id: assistant_id.to_string(),
            version: 1,
        },
        serde_json::json!({ "key": "value" }),
    );
    run.submitted_at = submitted_at;
    run
}

/// A fixture `ParleyResponse` "answering" a made-up parley, carrying the full
/// field set so round-trip tests exercise every field.
fn sample_parley_response(responded_by: &str) -> ParleyResponse {
    ParleyResponse {
        parley_id: ParleyId::new(),
        kind: ParleyKind::Choice,
        prompt: "which option?".to_string(),
        value: serde_json::json!("a"),
        responded_by: Some(responded_by.to_string()),
        responded_at: Utc::now(),
        defaulted: false,
    }
}

// ── Round trip (D-01) ─────────────────────────────────────────────────────

/// `insert` then `get` round-trips every `Run` field, including `input`,
/// `webhook` (with `secret`), `pending_responses` and `fork_from`.
pub async fn insert_then_get_round_trips_every_field(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-insert-get-round-trip").unwrap();
    let mut run = sample_run(&thread, "assistant-a", Utc::now())
        .with_webhook(WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: Some("shh-secret".to_string()),
            events: vec![RunEventKind::Completed, RunEventKind::Failed],
        })
        .with_fork_from(ForkSpec {
            from_waypoint_id: "wp-parent".to_string(),
            edit: Some(serde_json::json!({ "patched": true })),
        });
    run.pending_responses = vec![sample_parley_response("responder-a")];

    port.insert(&run).await.unwrap();
    let loaded = port.get(&run.run_id).await.unwrap().unwrap();

    assert_eq!(loaded.run_id, run.run_id);
    assert_eq!(loaded.thread_id, run.thread_id);
    assert_eq!(loaded.assistant, run.assistant);
    assert_eq!(loaded.input, run.input);
    assert_eq!(loaded.status, run.status);
    assert_eq!(loaded.submitted_at, run.submitted_at);
    assert_eq!(loaded.attempt, run.attempt);
    assert_eq!(loaded.cancel_requested, run.cancel_requested);
    assert_eq!(
        loaded.webhook.as_ref().map(|w| w.url.clone()),
        run.webhook.as_ref().map(|w| w.url.clone())
    );
    assert_eq!(
        loaded.webhook.as_ref().and_then(|w| w.secret.clone()),
        run.webhook.as_ref().and_then(|w| w.secret.clone())
    );
    assert_eq!(
        loaded.webhook.as_ref().map(|w| w.events.clone()),
        run.webhook.as_ref().map(|w| w.events.clone())
    );
    assert_eq!(loaded.pending_responses, run.pending_responses);
    assert_eq!(loaded.fork_from, run.fork_from);
    assert_eq!(loaded.schema_version, run.schema_version);
}

// ── Compare-and-set status transitions (D-04) ────────────────────────────

/// `update_status(Queued, Running, t)` succeeds and sets `started_at = t`; a
/// second call claiming the row is still `Queued` fails with
/// `IllegalTransition` because the row is no longer `Queued` (the CAS check).
pub async fn update_status_queued_to_running_sets_started_at_then_stale_cas_fails(
    port: &dyn RunRepositoryPort,
) {
    let thread = ThreadId::new("contract-run-cas-queued-running").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();

    let at = Utc::now();
    port.update_status(&run.run_id, RunStatus::Queued, RunStatus::Running, at)
        .await
        .unwrap();
    let loaded = port.get(&run.run_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, RunStatus::Running);
    assert_eq!(loaded.started_at, Some(at));

    let err = port
        .update_status(&run.run_id, RunStatus::Queued, RunStatus::Cancelled, at)
        .await
        .unwrap_err();
    assert!(matches!(err, RunRepositoryError::IllegalTransition { .. }));
}

/// `update_status(Running, Completed, t)` sets `finished_at = t`; any
/// transition attempted out of a now-terminal status fails with
/// `IllegalTransition`.
pub async fn update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing(
    port: &dyn RunRepositoryPort,
) {
    let thread = ThreadId::new("contract-run-cas-running-completed").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();
    let started_at = Utc::now();
    port.update_status(
        &run.run_id,
        RunStatus::Queued,
        RunStatus::Running,
        started_at,
    )
    .await
    .unwrap();

    let finished_at = Utc::now();
    port.update_status(
        &run.run_id,
        RunStatus::Running,
        RunStatus::Completed,
        finished_at,
    )
    .await
    .unwrap();
    let loaded = port.get(&run.run_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, RunStatus::Completed);
    assert_eq!(loaded.finished_at, Some(finished_at));

    let err = port
        .update_status(
            &run.run_id,
            RunStatus::Completed,
            RunStatus::Running,
            finished_at,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, RunRepositoryError::IllegalTransition { .. }));
}

/// A self-transition (`Running -> Running`) fails with
/// `IllegalTransition { from: Running, to: Running }`.
pub async fn update_status_self_transition_fails(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-cas-self-transition").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();
    let at = Utc::now();
    port.update_status(&run.run_id, RunStatus::Queued, RunStatus::Running, at)
        .await
        .unwrap();

    let err = port
        .update_status(&run.run_id, RunStatus::Running, RunStatus::Running, at)
        .await
        .unwrap_err();
    match err {
        RunRepositoryError::IllegalTransition { from, to } => {
            assert_eq!(from, RunStatus::Running);
            assert_eq!(to, RunStatus::Running);
        }
        other => {
            panic!("expected IllegalTransition {{ from: Running, to: Running }}, got {other:?}")
        }
    }
}

// ── D-17 / D-18: one-active-run-per-thread ───────────────────────────────

/// A second `insert` for a thread whose latest run is `Queued`, `Running` or
/// `AwaitingInput` fails with `ThreadBusy`; after the first run reaches a
/// terminal status, a new `insert` on that thread succeeds (D-18).
pub async fn insert_rejects_second_active_run_then_succeeds_after_terminal(
    port: &dyn RunRepositoryPort,
) {
    for active in [
        RunStatus::Queued,
        RunStatus::Running,
        RunStatus::AwaitingInput,
    ] {
        let thread =
            ThreadId::new(format!("contract-run-thread-busy-{}", active.as_str())).unwrap();
        let first = sample_run(&thread, "assistant-a", Utc::now());
        port.insert(&first).await.unwrap();

        // Drive the first run to `active`'s status (Queued needs no
        // transition at all).
        match active {
            RunStatus::Queued => {}
            RunStatus::Running => {
                port.update_status(
                    &first.run_id,
                    RunStatus::Queued,
                    RunStatus::Running,
                    Utc::now(),
                )
                .await
                .unwrap();
            }
            RunStatus::AwaitingInput => {
                port.update_status(
                    &first.run_id,
                    RunStatus::Queued,
                    RunStatus::Running,
                    Utc::now(),
                )
                .await
                .unwrap();
                port.update_status(
                    &first.run_id,
                    RunStatus::Running,
                    RunStatus::AwaitingInput,
                    Utc::now(),
                )
                .await
                .unwrap();
            }
            _ => unreachable!("only active statuses are iterated"),
        }

        let second = sample_run(&thread, "assistant-a", Utc::now());
        let err = port.insert(&second).await.unwrap_err();
        assert!(
            matches!(err, RunRepositoryError::ThreadBusy { .. }),
            "expected ThreadBusy while the thread's active run is {active:?}, got {err:?}"
        );

        // Retire the first run to a terminal status (every active status has
        // a legal edge to `Cancelled`, per `RunStatus::try_transition`),
        // then confirm the thread is no longer busy.
        port.update_status(&first.run_id, active, RunStatus::Cancelled, Utc::now())
            .await
            .unwrap();

        let third = sample_run(&thread, "assistant-a", Utc::now());
        port.insert(&third).await.unwrap();
    }
}

// ── Listing and pagination ────────────────────────────────────────────────

/// `list(RunQuery { limit: 2, .. })` over five runs with two sharing a
/// `submitted_at` returns pages ordered by `(submitted_at DESC, run_id
/// DESC)` with no overlap and no gap across three pages, and `next_cursor`
/// is `None` exactly when the page ends on the last row.
pub async fn list_paginates_by_submitted_at_and_run_id_with_no_overlap_or_gap(
    port: &dyn RunRepositoryPort,
) {
    // Five DISTINCT threads (D-17/D-18 forbid five active runs on one
    // thread), scoped to this test via a unique `assistant_id` so pagination
    // sees exactly these five rows regardless of what else the backend
    // holds.
    let assistant_id = "contract-run-list-pagination-fixture";
    let base = Utc::now();
    let mut inserted = Vec::new();
    for i in 0..5u32 {
        // Runs 0 and 1 deliberately share a `submitted_at` (both `base`) so
        // the `run_id` tiebreak is exercised; the rest strictly increase.
        let ts = if i <= 1 {
            base
        } else {
            base + chrono::Duration::seconds(i as i64)
        };
        let thread = ThreadId::new(format!("contract-run-list-pagination-{i}")).unwrap();
        let run = sample_run(&thread, assistant_id, ts);
        port.insert(&run).await.unwrap();
        inserted.push(run);
    }

    // Expected order: (submitted_at DESC, run_id DESC).
    let mut expected = inserted.clone();
    expected.sort_by(|a, b| {
        b.submitted_at
            .cmp(&a.submitted_at)
            .then_with(|| b.run_id.as_str().cmp(a.run_id.as_str()))
    });
    let expected_ids: Vec<RunId> = expected.iter().map(|r| r.run_id.clone()).collect();

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..3 {
        let page = port
            .list(RunQuery {
                assistant_id: Some(assistant_id.to_string()),
                limit: 2,
                cursor: cursor.clone(),
                ..Default::default()
            })
            .await
            .unwrap();
        for item in &page.items {
            assert!(
                !seen.contains(&item.run_id),
                "run {} appeared on more than one page (overlap)",
                item.run_id
            );
            seen.push(item.run_id.clone());
        }
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(seen, expected_ids, "no gap: every run seen, in order");
}

/// `list` filtered by `thread_id`, by `assistant_id` and by `status` returns
/// only matching rows.
pub async fn list_filters_by_thread_assistant_and_status(port: &dyn RunRepositoryPort) {
    let thread_a = ThreadId::new("contract-run-list-filter-thread-a").unwrap();
    let thread_b = ThreadId::new("contract-run-list-filter-thread-b").unwrap();

    let run_a = sample_run(&thread_a, "assistant-x", Utc::now());
    let run_b = sample_run(
        &thread_b,
        "assistant-y",
        Utc::now() + chrono::Duration::seconds(1),
    );
    port.insert(&run_a).await.unwrap();
    port.insert(&run_b).await.unwrap();
    port.update_status(
        &run_b.run_id,
        RunStatus::Queued,
        RunStatus::Running,
        Utc::now(),
    )
    .await
    .unwrap();

    let by_thread = port
        .list(RunQuery {
            thread_id: Some(thread_a.clone()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_thread.items.len(), 1);
    assert_eq!(by_thread.items[0].run_id, run_a.run_id);

    let by_assistant = port
        .list(RunQuery {
            assistant_id: Some("assistant-y".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_assistant.items.len(), 1);
    assert_eq!(by_assistant.items[0].run_id, run_b.run_id);

    let by_status = port
        .list(RunQuery {
            status: Some(RunStatus::Running),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_status.items.len(), 1);
    assert_eq!(by_status.items[0].run_id, run_b.run_id);
}

// ── Cancellation ───────────────────────────────────────────────────────────

/// `request_cancel` on a non-terminal run sets `cancel_requested = true`, is
/// idempotent, and returns the current status; on a terminal run it fails
/// with `AlreadyTerminal`.
pub async fn request_cancel_is_idempotent_and_rejects_terminal(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-request-cancel").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();

    let status = port.request_cancel(&run.run_id).await.unwrap();
    assert_eq!(status, RunStatus::Queued);
    let status_again = port.request_cancel(&run.run_id).await.unwrap();
    assert_eq!(status_again, RunStatus::Queued);

    port.update_status(
        &run.run_id,
        RunStatus::Queued,
        RunStatus::Cancelled,
        Utc::now(),
    )
    .await
    .unwrap();

    let err = port.request_cancel(&run.run_id).await.unwrap_err();
    assert!(matches!(err, RunRepositoryError::AlreadyTerminal { .. }));
}

/// `is_cancel_requested(thread)` reflects the flag of the thread's active
/// run and is `false` for an unknown thread.
pub async fn is_cancel_requested_reflects_active_run_flag(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-is-cancel-requested").unwrap();
    let unknown = ThreadId::new("contract-run-is-cancel-requested-unknown").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();

    assert!(!port.is_cancel_requested(&thread).await.unwrap());
    assert!(!port.is_cancel_requested(&unknown).await.unwrap());

    port.request_cancel(&run.run_id).await.unwrap();
    assert!(port.is_cancel_requested(&thread).await.unwrap());
}

// ── Attempt/resume bookkeeping ───────────────────────────────────────────

/// `bump_attempt` returns the incremented value and persists it.
pub async fn bump_attempt_increments_and_persists(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-bump-attempt").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();

    let first = port.bump_attempt(&run.run_id).await.unwrap();
    let second = port.bump_attempt(&run.run_id).await.unwrap();
    assert_eq!(second, first + 1);

    let loaded = port.get(&run.run_id).await.unwrap().unwrap();
    assert_eq!(loaded.attempt, second);
}

/// `record_resume` on an `AwaitingInput` run stores the responses,
/// increments `attempt` and returns it; on any other status it fails with
/// `IllegalTransition { from: <status>, to: AwaitingInput }`.
/// `clear_pending_responses` empties the list.
pub async fn record_resume_on_awaiting_input_then_clear_pending_responses(
    port: &dyn RunRepositoryPort,
) {
    let thread = ThreadId::new("contract-run-record-resume").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();

    // Not AwaitingInput yet: record_resume must fail.
    let err = port
        .record_resume(&run.run_id, vec![sample_parley_response("early")])
        .await
        .unwrap_err();
    assert!(matches!(err, RunRepositoryError::IllegalTransition { .. }));

    port.update_status(
        &run.run_id,
        RunStatus::Queued,
        RunStatus::Running,
        Utc::now(),
    )
    .await
    .unwrap();
    port.update_status(
        &run.run_id,
        RunStatus::Running,
        RunStatus::AwaitingInput,
        Utc::now(),
    )
    .await
    .unwrap();

    let before_attempt = port.get(&run.run_id).await.unwrap().unwrap().attempt;
    let responses = vec![sample_parley_response("resumer")];
    let new_attempt = port
        .record_resume(&run.run_id, responses.clone())
        .await
        .unwrap();
    assert_eq!(new_attempt, before_attempt + 1);

    let loaded = port.get(&run.run_id).await.unwrap().unwrap();
    assert_eq!(loaded.pending_responses, responses);
    assert_eq!(loaded.attempt, new_attempt);

    port.clear_pending_responses(&run.run_id).await.unwrap();
    let cleared = port.get(&run.run_id).await.unwrap().unwrap();
    assert!(cleared.pending_responses.is_empty());
}

// ── Outcome recording ──────────────────────────────────────────────────────

/// `record_outcome` persists `error`, `output` and `final_waypoint_id`
/// without touching `status`.
pub async fn record_outcome_persists_fields_without_touching_status(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-record-outcome").unwrap();
    let run = sample_run(&thread, "assistant-a", Utc::now());
    port.insert(&run).await.unwrap();
    port.update_status(
        &run.run_id,
        RunStatus::Queued,
        RunStatus::Running,
        Utc::now(),
    )
    .await
    .unwrap();

    port.record_outcome(
        &run.run_id,
        RunOutcomeRecord {
            error: Some("boom".to_string()),
            output: Some(serde_json::json!({ "ok": false })),
            final_waypoint_id: Some("wp-1".to_string()),
        },
    )
    .await
    .unwrap();

    let loaded = port.get(&run.run_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, RunStatus::Running);
    assert_eq!(loaded.error.as_deref(), Some("boom"));
    assert_eq!(loaded.output, Some(serde_json::json!({ "ok": false })));
    assert_eq!(loaded.final_waypoint_id.as_deref(), Some("wp-1"));
}

// ── Schema versioning (X-04) ────────────────────────────────────────────

/// A row whose `schema_version` does not match the currently supported
/// version fails `get` with `UnknownSchemaVersion { found }`, never a panic.
pub async fn get_on_unsupported_schema_version_fails(port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-unknown-schema-version").unwrap();
    let mut run = sample_run(&thread, "assistant-a", Utc::now());
    run.schema_version = "v99-from-the-future".to_string();
    port.insert(&run).await.unwrap();

    let err = port.get(&run.run_id).await.unwrap_err();
    match err {
        RunRepositoryError::UnknownSchemaVersion { found } => {
            assert_eq!(found, "v99-from-the-future");
        }
        other => panic!("expected UnknownSchemaVersion, got {other:?}"),
    }
}

// ── Concurrency stress test (D-52, X-05) ──────────────────────────────────

/// Ten concurrent `insert` calls for the same thread against a fresh store
/// yield exactly one `Ok` and nine `ThreadBusy` (PLAT-FR-05 at the storage
/// layer, D-52).
pub async fn ten_concurrent_inserts_one_thread_exactly_one_accepted(
    port: Arc<dyn RunRepositoryPort>,
) {
    let thread = ThreadId::new("contract-run-ten-concurrent-inserts").unwrap();
    let mut handles = Vec::new();
    for _ in 0..10 {
        let port = Arc::clone(&port);
        let run = sample_run(&thread, "assistant-a", Utc::now());
        handles.push(tokio::spawn(async move { port.insert(&run).await }));
    }

    let mut ok_count = 0;
    let mut busy_count = 0;
    tokio::time::timeout(Duration::from_secs(10), async {
        for handle in handles {
            match handle.await.expect("task must not panic") {
                Ok(()) => ok_count += 1,
                Err(RunRepositoryError::ThreadBusy { .. }) => busy_count += 1,
                Err(other) => panic!("unexpected error: {other:?}"),
            }
        }
    })
    .await
    .expect("ten concurrent inserts must not hang");

    assert_eq!(ok_count, 1, "exactly one insert must be accepted");
    assert_eq!(
        busy_count, 9,
        "the other nine must be rejected as ThreadBusy"
    );
}

// ── insert_with_latest / freeze-at-submit (D-30) ─────────────────────────
//
// These clauses require a REAL `insert_with_latest` override (D-30) --
// they are not run against the port's own default (verbatim-insert)
// implementation, which has no concept of an assistant repository and
// therefore cannot demonstrate freeze-at-submit at all.

/// `insert_with_latest` resolves and freezes the assistant's CURRENT
/// `latest` onto the row, ignoring whatever `assistant.version` the caller
/// already set on the `Run`; a later `append_version` is picked up by the
/// next `insert_with_latest` call.
pub async fn insert_with_latest_resolves_current_latest_and_freezes_it(
    run_port: &dyn RunRepositoryPort,
    assistant_port: &dyn AssistantRepositoryPort,
) {
    let assistant_id = AssistantId::new("contract-run-insert-with-latest").unwrap();
    assistant_port
        .create(&assistant_id, sample_assistant_new_version("v1"))
        .await
        .unwrap();

    let thread1 = ThreadId::new("contract-run-insert-with-latest-t1").unwrap();
    let mut run1 = sample_run(&thread1, assistant_id.as_str(), Utc::now());
    run1.assistant.version = 999; // must be ignored -- resolved from `latest`, not the caller
    let resolved1 = run_port.insert_with_latest(&run1).await.unwrap();
    assert_eq!(resolved1, 1);
    let loaded1 = run_port.get(&run1.run_id).await.unwrap().unwrap();
    assert_eq!(loaded1.assistant.version, 1);

    assistant_port
        .append_version(&assistant_id, sample_assistant_new_version("v2"))
        .await
        .unwrap();

    let thread2 = ThreadId::new("contract-run-insert-with-latest-t2").unwrap();
    let run2 = sample_run(&thread2, assistant_id.as_str(), Utc::now());
    let resolved2 = run_port.insert_with_latest(&run2).await.unwrap();
    assert_eq!(resolved2, 2);
    let loaded2 = run_port.get(&run2.run_id).await.unwrap().unwrap();
    assert_eq!(loaded2.assistant.version, 2);
}

/// `insert_with_latest` against an assistant that does not exist fails
/// `UnknownAssistant`.
pub async fn insert_with_latest_unknown_assistant_fails(run_port: &dyn RunRepositoryPort) {
    let thread = ThreadId::new("contract-run-insert-with-latest-unknown").unwrap();
    let run = sample_run(&thread, "definitely-unknown-assistant", Utc::now());
    let err = run_port.insert_with_latest(&run).await.unwrap_err();
    assert!(matches!(err, RunRepositoryError::UnknownAssistant { .. }));
}

/// `insert_with_latest` against a soft-deleted assistant fails
/// `UnknownAssistant` (a deleted assistant cannot resolve a fresh `latest`).
pub async fn insert_with_latest_soft_deleted_assistant_fails(
    run_port: &dyn RunRepositoryPort,
    assistant_port: &dyn AssistantRepositoryPort,
) {
    let assistant_id = AssistantId::new("contract-run-insert-with-latest-deleted").unwrap();
    assistant_port
        .create(&assistant_id, sample_assistant_new_version("v1"))
        .await
        .unwrap();
    assistant_port.soft_delete(&assistant_id).await.unwrap();

    let thread = ThreadId::new("contract-run-insert-with-latest-deleted-thread").unwrap();
    let run = sample_run(&thread, assistant_id.as_str(), Utc::now());
    let err = run_port.insert_with_latest(&run).await.unwrap_err();
    assert!(matches!(err, RunRepositoryError::UnknownAssistant { .. }));
}

/// Twenty alternating `append_version`/`insert_with_latest` calls on
/// distinct threads: every run's resolved version exists and is `<=` the
/// `latest` observed after the loop; no run resolves version `0` or a
/// version greater than `latest` (PLAT-FR-08 as a database property, D-52).
pub async fn assistant_version_freeze_at_submit(
    run_port: Arc<dyn RunRepositoryPort>,
    assistant_port: Arc<dyn AssistantRepositoryPort>,
) {
    let assistant_id = AssistantId::new("contract-run-freeze-at-submit").unwrap();
    assistant_port
        .create(&assistant_id, sample_assistant_new_version("v1"))
        .await
        .unwrap();

    let mut append_handles = Vec::new();
    for i in 0..10 {
        let assistant_port = Arc::clone(&assistant_port);
        let assistant_id = assistant_id.clone();
        append_handles.push(tokio::spawn(async move {
            assistant_port
                .append_version(
                    &assistant_id,
                    sample_assistant_new_version(&format!("a{i}")),
                )
                .await
        }));
    }

    let mut insert_handles = Vec::new();
    for i in 0..10 {
        let run_port = Arc::clone(&run_port);
        let thread = ThreadId::new(format!("contract-run-freeze-thread-{i}")).unwrap();
        let run = sample_run(&thread, assistant_id.as_str(), Utc::now());
        let run_id = run.run_id.clone();
        insert_handles.push(tokio::spawn(async move {
            run_port
                .insert_with_latest(&run)
                .await
                .map(|version| (run_id, version))
        }));
    }

    tokio::time::timeout(Duration::from_secs(20), async {
        for handle in append_handles {
            handle
                .await
                .expect("append_version task must not panic")
                .expect("append_version must succeed");
        }
    })
    .await
    .expect("append_version tasks must not hang");

    let mut resolved = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        for handle in insert_handles {
            resolved.push(
                handle
                    .await
                    .expect("insert_with_latest task must not panic")
                    .expect("insert_with_latest must succeed"),
            );
        }
    })
    .await
    .expect("insert_with_latest tasks must not hang");

    let final_latest = assistant_port
        .get(&assistant_id)
        .await
        .unwrap()
        .unwrap()
        .latest;
    assert_eq!(final_latest, 11, "ten appends onto version 1 must reach 11");

    for (run_id, version) in &resolved {
        assert!(
            *version >= 1 && *version <= final_latest,
            "run {run_id} resolved version {version}, outside [1, {final_latest}]"
        );
        let exists = assistant_port
            .get_version(&assistant_id, *version)
            .await
            .unwrap();
        assert!(
            exists.is_some(),
            "run {run_id}'s resolved version {version} must exist"
        );
    }
}
