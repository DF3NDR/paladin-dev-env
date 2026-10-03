//! Shared `RunScheduleRepositoryPort` contract suite (D-36, D-37, D-39).
//!
//! One generic async function per contract clause, each taking `&dyn
//! RunScheduleRepositoryPort` (or, for the concurrency stress test, `Arc<dyn
//! RunScheduleRepositoryPort>`) and asserting inside. Every backend
//! (`InMemoryRunScheduleRepository`, `SqliteRunScheduleRepository`,
//! `PostgresRunScheduleRepository`) invokes these unchanged from its own
//! `#[tokio::test]`s, mirroring `crate::assistant::contract_tests`'s house
//! pattern.

use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};

use paladin_core::platform::container::principal::{RunAttribution, TenantId};
use paladin_core::platform::container::run::WebhookSpec;
use paladin_core::platform::container::run_schedule::{
    OnMissed, RunSchedule, RunScheduleId, RunScheduleUpdate, ThreadStrategy,
};
use paladin_ports::output::run_schedule_repository_port::{
    RunScheduleRepositoryError, RunScheduleRepositoryPort,
};

/// Build a `RunSchedule` fixture, tagging `assistant_id` with `marker` so
/// distinct calls are distinguishable.
pub fn sample_schedule(marker: &str) -> RunSchedule {
    RunSchedule::new(
        RunScheduleId::new_v7(),
        format!("assistant-{marker}"),
        "*/5 * * * *",
    )
}

// ── Insert + AlreadyExists ──────────────────────────────────────────────

/// `insert` persists a schedule readable by `get`; a second `insert` for the
/// same id fails with `AlreadyExists`.
pub async fn insert_creates_and_rejects_duplicate(port: &dyn RunScheduleRepositoryPort) {
    let schedule = sample_schedule("insert");
    let id = schedule.schedule_id.clone();
    port.insert(schedule.clone()).await.unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.schedule_id, id);
    assert_eq!(loaded.assistant_id, schedule.assistant_id);

    let err = port.insert(schedule).await.unwrap_err();
    assert!(matches!(
        err,
        RunScheduleRepositoryError::AlreadyExists { .. }
    ));
}

/// `get` on an unknown id returns `Ok(None)`, never an error.
pub async fn get_returns_none_for_unknown(port: &dyn RunScheduleRepositoryPort) {
    let unknown = RunScheduleId::new_v7();
    assert!(port.get(&unknown).await.unwrap().is_none());
}

// ── Listing and pagination ────────────────────────────────────────────────

/// `list` returns schedules in ascending `schedule_id` order across pages,
/// tolerant of interleaved rows created by other tests sharing the same
/// backend (Tier 2 Postgres): filters to the ids THIS clause created,
/// asserting no overlap and no gap among those.
pub async fn list_paginates_ascending_by_schedule_id(port: &dyn RunScheduleRepositoryPort) {
    let mut expected: Vec<RunScheduleId> = Vec::new();
    for i in 0..5 {
        let schedule = sample_schedule(&format!("list-page-{i}"));
        expected.push(schedule.schedule_id.clone());
        port.insert(schedule).await.unwrap();
    }
    expected.sort();

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..1000 {
        let page = port.list(2, cursor.clone()).await.unwrap();
        if page.items.is_empty() && page.next_cursor.is_none() {
            break;
        }
        for item in &page.items {
            if expected.contains(&item.schedule_id) {
                assert!(
                    !seen.contains(&item.schedule_id),
                    "schedule {} appeared on more than one page (overlap)",
                    item.schedule_id
                );
                seen.push(item.schedule_id.clone());
            }
        }
        cursor = page.next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }
    seen.sort();
    assert_eq!(
        seen, expected,
        "no gap: every created schedule seen, in order"
    );
}

// ── Update ───────────────────────────────────────────────────────────────

/// `update` applies only the fields set to `Some`; an unknown id fails
/// `NotFound`.
pub async fn update_applies_partial_changes_and_rejects_unknown(
    port: &dyn RunScheduleRepositoryPort,
) {
    let schedule = sample_schedule("update");
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    port.update(
        &id,
        RunScheduleUpdate {
            cron: Some("0 0 * * *".to_string()),
            enabled: Some(false),
            on_missed: Some(OnMissed::RunOnce),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.cron, "0 0 * * *");
    assert!(!loaded.enabled);
    assert_eq!(loaded.on_missed, OnMissed::RunOnce);
    // Untouched fields keep their original values.
    assert_eq!(loaded.timezone, "UTC");

    let unknown = RunScheduleId::new_v7();
    let err = port
        .update(&unknown, RunScheduleUpdate::default())
        .await
        .unwrap_err();
    assert!(matches!(err, RunScheduleRepositoryError::NotFound { .. }));
}

// ── Delete ───────────────────────────────────────────────────────────────

/// `delete` removes the schedule (`get` returns `None` afterward); deleting
/// an unknown id fails `NotFound`.
pub async fn delete_removes_and_rejects_unknown(port: &dyn RunScheduleRepositoryPort) {
    let schedule = sample_schedule("delete");
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    port.delete(&id).await.unwrap();
    assert!(port.get(&id).await.unwrap().is_none());

    let err = port.delete(&id).await.unwrap_err();
    assert!(matches!(err, RunScheduleRepositoryError::NotFound { .. }));
}

// ── due ──────────────────────────────────────────────────────────────────

/// `due` returns only enabled schedules whose `next_tick <= now`, ordered
/// ascending by `next_tick`; a disabled schedule and a not-yet-due schedule
/// are both excluded.
pub async fn due_filters_enabled_and_next_tick_le_now_ordered_ascending(
    port: &dyn RunScheduleRepositoryPort,
) {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();

    let due_early = sample_schedule("due-early").with_next_tick(now - chrono::Duration::hours(1));
    let due_late = sample_schedule("due-late").with_next_tick(now);
    let not_due = sample_schedule("not-due").with_next_tick(now + chrono::Duration::hours(1));
    let mut disabled_but_due = sample_schedule("disabled-due").with_next_tick(now);
    disabled_but_due.enabled = false;

    let due_early_id = due_early.schedule_id.clone();
    let due_late_id = due_late.schedule_id.clone();
    let not_due_id = not_due.schedule_id.clone();
    let disabled_id = disabled_but_due.schedule_id.clone();

    port.insert(due_early).await.unwrap();
    port.insert(due_late).await.unwrap();
    port.insert(not_due).await.unwrap();
    port.insert(disabled_but_due).await.unwrap();

    let due = port.due(now, 100).await.unwrap();
    let due_ids: Vec<_> = due.iter().map(|s| s.schedule_id.clone()).collect();

    assert!(due_ids.contains(&due_early_id));
    assert!(due_ids.contains(&due_late_id));
    assert!(!due_ids.contains(&not_due_id));
    assert!(!due_ids.contains(&disabled_id));

    let early_pos = due_ids.iter().position(|id| id == &due_early_id).unwrap();
    let late_pos = due_ids.iter().position(|id| id == &due_late_id).unwrap();
    assert!(
        early_pos < late_pos,
        "due() must order ascending by next_tick"
    );
}

// ── claim_tick (D-37) ───────────────────────────────────────────────────

/// `claim_tick` succeeds (`true`) exactly once for the current `next_tick`,
/// advancing `last_tick`/`next_tick`; a second call with the SAME
/// (now-stale) `expected_next` fails (`false`) without changing the row.
pub async fn claim_tick_succeeds_once_then_fails_on_stale_expected(
    port: &dyn RunScheduleRepositoryPort,
) {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let t1 = t0 + chrono::Duration::minutes(5);
    let t2 = t1 + chrono::Duration::minutes(5);

    let schedule = sample_schedule("claim").with_next_tick(t1);
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    let claimed = port.claim_tick(&id, t1, t1, t2).await.unwrap();
    assert!(claimed, "first claim at the current next_tick must succeed");

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.last_tick, Some(t1));
    assert_eq!(loaded.next_tick, Some(t2));

    // A second claim against the SAME (now-stale) expected_next must fail.
    let stale = port.claim_tick(&id, t1, t1, t2).await.unwrap();
    assert!(!stale, "a stale expected_next must not claim");

    // The row is unchanged by the failed claim.
    let loaded_again = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded_again.next_tick, Some(t2));
}

/// `claim_tick` against an unknown schedule id returns `false`, never an
/// error.
pub async fn claim_tick_on_unknown_schedule_returns_false(port: &dyn RunScheduleRepositoryPort) {
    let unknown = RunScheduleId::new_v7();
    let now = Utc::now();
    let claimed = port
        .claim_tick(&unknown, now, now, now + chrono::Duration::minutes(1))
        .await
        .unwrap();
    assert!(!claimed);
}

/// Eight tasks race `claim_tick` on the SAME schedule with the SAME
/// `expected_next` — exactly one succeeds (D-37, PRD acceptance 5's
/// underlying primitive).
pub async fn claim_tick_race_admits_exactly_one(port: Arc<dyn RunScheduleRepositoryPort>) {
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let t1 = t0 + chrono::Duration::minutes(1);
    let t2 = t1 + chrono::Duration::minutes(1);

    let schedule = sample_schedule("claim-race").with_next_tick(t1);
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let port = Arc::clone(&port);
        let id = id.clone();
        handles.push(tokio::spawn(async move {
            port.claim_tick(&id, t1, t1, t2).await
        }));
    }

    let mut successes = 0;
    tokio::time::timeout(Duration::from_secs(20), async {
        for handle in handles {
            let claimed = handle.await.expect("task must not panic").unwrap();
            if claimed {
                successes += 1;
            }
        }
    })
    .await
    .expect("eight concurrent claim_tick calls must not hang");

    assert_eq!(
        successes, 1,
        "exactly one of eight concurrent claim_tick calls must succeed"
    );

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.next_tick, Some(t2));
}

// ── increment_skipped (D-39) ─────────────────────────────────────────────

/// `increment_skipped` increments and returns the new count, persisted on
/// the row.
pub async fn increment_skipped_increments_and_persists(port: &dyn RunScheduleRepositoryPort) {
    let schedule = sample_schedule("skip");
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    let first = port.increment_skipped(&id).await.unwrap();
    assert_eq!(first, 1);
    let second = port.increment_skipped(&id).await.unwrap();
    assert_eq!(second, 2);

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.skipped_ticks, 2);
}

// ── created_by (Phase 41, D-08) ───────────────────────────────────────────

/// Build the attribution fixture the `created_by` clauses stamp.
fn creator_fixture() -> RunAttribution {
    RunAttribution::new(
        TenantId::new("acme").expect("fixture tenant is valid"),
        "ops",
    )
}

/// A schedule inserted with `created_by` reads the same attribution back from
/// `get`, `list` and `due` (D-08).
pub async fn created_by_round_trips(port: &dyn RunScheduleRepositoryPort) {
    let past = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let schedule = sample_schedule("created-by")
        .with_created_by(creator_fixture())
        .with_next_tick(past);
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    let via_get = port.get(&id).await.unwrap().unwrap();
    assert_eq!(via_get.created_by, Some(creator_fixture()));

    // `list` pages ascending by id; walk the pages until this clause's row
    // is found (tolerant of rows other clauses left in a shared backend).
    let mut cursor = None;
    let mut via_list = None;
    for _ in 0..1000 {
        let page = port.list(50, cursor.clone()).await.unwrap();
        if let Some(found) = page.items.into_iter().find(|s| s.schedule_id == id) {
            via_list = Some(found);
            break;
        }
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    let via_list = via_list.expect("list must return the inserted schedule");
    assert_eq!(via_list.created_by, Some(creator_fixture()));

    let now = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let due = port.due(now, 0).await.unwrap();
    let via_due = due
        .into_iter()
        .find(|s| s.schedule_id == id)
        .expect("due must return the inserted schedule");
    assert_eq!(via_due.created_by, Some(creator_fixture()));
}

/// A schedule with no creator reads back `None` (stored as NULL in both
/// columns, D-08).
pub async fn null_created_by_reads_back_none(port: &dyn RunScheduleRepositoryPort) {
    let schedule = sample_schedule("no-creator");
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert!(loaded.created_by.is_none());
}

/// `update` with every patchable field changed, plus `increment_skipped` and
/// `claim_tick`, leaves `created_by` unchanged (D-08: `PATCH` never
/// re-assigns the creator).
pub async fn update_never_changes_created_by(port: &dyn RunScheduleRepositoryPort) {
    let t1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 5, 0).unwrap();
    let t2 = t1 + chrono::Duration::minutes(5);
    let schedule = sample_schedule("keep-creator")
        .with_created_by(creator_fixture())
        .with_next_tick(t1);
    let id = schedule.schedule_id.clone();
    port.insert(schedule).await.unwrap();

    port.update(
        &id,
        RunScheduleUpdate {
            cron: Some("0 0 * * *".to_string()),
            timezone: Some("Europe/Berlin".to_string()),
            input: Some(serde_json::json!({"changed": true})),
            enabled: Some(false),
            thread_strategy: Some(ThreadStrategy::NewThreadPerTick),
            on_missed: Some(OnMissed::RunOnce),
            webhook: Some(WebhookSpec {
                url: "https://example.com/hook".to_string(),
                secret: None,
                events: vec![],
            }),
            next_tick: Some(t2),
        },
    )
    .await
    .unwrap();
    port.increment_skipped(&id).await.unwrap();
    assert!(port.claim_tick(&id, t2, t2, t2).await.unwrap());

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.cron, "0 0 * * *");
    assert_eq!(loaded.created_by, Some(creator_fixture()));
}
