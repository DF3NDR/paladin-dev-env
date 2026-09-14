//! Shared `WebhookDeliveryRepositoryPort` contract suite (D-40, D-43).
//!
//! One generic async function per contract clause, each taking `&dyn
//! WebhookDeliveryRepositoryPort` (or, for the concurrency stress test,
//! `Arc<dyn WebhookDeliveryRepositoryPort>`) and asserting inside. Every
//! backend (`InMemoryWebhookDeliveryRepository`,
//! `SqliteWebhookDeliveryRepository`, `PostgresWebhookDeliveryRepository`)
//! invokes these unchanged from its own `#[tokio::test]`s, mirroring
//! `crate::run_schedule::contract_tests`'s house pattern.

use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};

use paladin_core::platform::container::run::{RunEventKind, RunId};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_core::platform::container::webhook::{
    WebhookAttemptOutcome, WebhookAttemptResult, WebhookDelivery, WebhookDeliveryId,
    WebhookDeliveryStatus,
};
use paladin_ports::output::webhook_delivery_port::{
    WebhookDeliveryRepositoryError, WebhookDeliveryRepositoryPort,
};

/// Build a `WebhookDelivery` fixture, tagging `run_id` with `marker` so
/// distinct calls are distinguishable. Eligible for claim immediately
/// (`next_attempt_at == now`, `status: Pending`) unless overridden.
pub fn sample_delivery(marker: &str, next_attempt_at: chrono::DateTime<Utc>) -> WebhookDelivery {
    WebhookDelivery::new(
        WebhookDeliveryId::new_v7(),
        RunId::new_v7(),
        ThreadId::new(format!("thread-{marker}")).unwrap(),
        RunEventKind::Completed,
        format!("https://example.com/hook-{marker}"),
        format!(r#"{{"marker":"{marker}"}}"#),
        next_attempt_at,
    )
}

// ── enqueue + get ────────────────────────────────────────────────────────

/// `enqueue` persists a delivery readable by `get`, `Pending`, attempt `0`.
pub async fn enqueue_creates_and_is_readable(port: &dyn WebhookDeliveryRepositoryPort) {
    let delivery = sample_delivery("enqueue", Utc::now());
    let id = delivery.delivery_id.clone();
    let run_id = delivery.run_id.clone();
    port.enqueue(delivery).await.unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert_eq!(loaded.delivery_id, id);
    assert_eq!(loaded.run_id, run_id);
    assert_eq!(loaded.attempt, 0);
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Pending));
}

/// `get` on an unknown id returns `Ok(None)`, never an error.
pub async fn get_returns_none_for_unknown(port: &dyn WebhookDeliveryRepositoryPort) {
    let unknown = WebhookDeliveryId::new_v7();
    assert!(port.get(&unknown).await.unwrap().is_none());
}

// ── claim_due (D-40) ─────────────────────────────────────────────────────

/// `claim_due` claims only `Pending`/`Retrying` rows with
/// `next_attempt_at <= now`, transitioning each to `InFlight`; a
/// not-yet-due row and a terminal (`Delivered`/`Dead`) row are excluded.
pub async fn claim_due_transitions_eligible_rows_to_in_flight(
    port: &dyn WebhookDeliveryRepositoryPort,
) {
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();

    let due_pending = sample_delivery("due-pending", now - chrono::Duration::minutes(1));
    let due_retrying =
        sample_delivery("due-retrying", now).with_status(WebhookDeliveryStatus::Retrying);
    let not_due = sample_delivery("not-due", now + chrono::Duration::hours(1));
    let delivered = sample_delivery("delivered", now).with_status(WebhookDeliveryStatus::Delivered);

    let due_pending_id = due_pending.delivery_id.clone();
    let due_retrying_id = due_retrying.delivery_id.clone();
    let not_due_id = not_due.delivery_id.clone();
    let delivered_id = delivered.delivery_id.clone();

    port.enqueue(due_pending).await.unwrap();
    port.enqueue(due_retrying).await.unwrap();
    port.enqueue(not_due).await.unwrap();
    port.enqueue(delivered).await.unwrap();

    let claimed = port.claim_due(now, 100).await.unwrap();
    let claimed_ids: Vec<_> = claimed.iter().map(|d| d.delivery_id.clone()).collect();

    assert!(claimed_ids.contains(&due_pending_id));
    assert!(claimed_ids.contains(&due_retrying_id));
    assert!(!claimed_ids.contains(&not_due_id));
    assert!(!claimed_ids.contains(&delivered_id));

    for claimed_delivery in &claimed {
        assert!(matches!(
            claimed_delivery.status,
            WebhookDeliveryStatus::InFlight
        ));
    }

    // The claim is persisted: re-reading the row shows InFlight too.
    let reloaded = port.get(&due_pending_id).await.unwrap().unwrap();
    assert!(matches!(reloaded.status, WebhookDeliveryStatus::InFlight));
}

/// `claim_due` respects `limit`.
pub async fn claim_due_respects_limit(port: &dyn WebhookDeliveryRepositoryPort) {
    let now = Utc::now();
    for i in 0..5 {
        port.enqueue(sample_delivery(&format!("limit-{i}"), now))
            .await
            .unwrap();
    }
    let claimed = port.claim_due(now, 2).await.unwrap();
    assert_eq!(claimed.len(), 2);
}

/// Several tasks race `claim_due` against the SAME small set of due rows --
/// the union of every claimed batch admits each row exactly once (D-40).
pub async fn claim_due_race_admits_each_row_once(port: Arc<dyn WebhookDeliveryRepositoryPort>) {
    let now = Utc::now();
    let mut expected_ids = Vec::new();
    for i in 0..6 {
        let delivery = sample_delivery(&format!("race-{i}"), now);
        expected_ids.push(delivery.delivery_id.clone());
        port.enqueue(delivery).await.unwrap();
    }

    let mut handles = Vec::new();
    for _ in 0..8 {
        let port = Arc::clone(&port);
        handles.push(tokio::spawn(async move { port.claim_due(now, 6).await }));
    }

    let mut claimed_ids: Vec<WebhookDeliveryId> = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), async {
        for handle in handles {
            let batch = handle.await.expect("task must not panic").unwrap();
            for delivery in batch {
                claimed_ids.push(delivery.delivery_id);
            }
        }
    })
    .await
    .expect("eight concurrent claim_due calls must not hang");

    claimed_ids.sort();
    claimed_ids.dedup();
    let mut expected_sorted = expected_ids.clone();
    expected_sorted.sort();
    assert_eq!(
        claimed_ids, expected_sorted,
        "every due row must be claimed exactly once across all racing callers"
    );
}

// ── record_attempt (D-43) ───────────────────────────────────────────────

/// `record_attempt` with `Delivered` sets the terminal status and
/// increments `attempt`.
pub async fn record_attempt_delivered_sets_terminal_status(
    port: &dyn WebhookDeliveryRepositoryPort,
) {
    let delivery = sample_delivery("delivered-outcome", Utc::now());
    let id = delivery.delivery_id.clone();
    port.enqueue(delivery).await.unwrap();

    port.record_attempt(
        &id,
        WebhookAttemptResult {
            outcome: WebhookAttemptOutcome::Delivered,
            response_status: Some(200),
            error: None,
        },
    )
    .await
    .unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Delivered));
    assert_eq!(loaded.attempt, 1);
    assert_eq!(loaded.last_response_status, Some(200));
}

/// `record_attempt` with `Retrying` sets `next_attempt_at` and increments
/// `attempt`, leaving the row eligible for a future `claim_due`.
pub async fn record_attempt_retrying_sets_next_attempt_at(
    port: &dyn WebhookDeliveryRepositoryPort,
) {
    let delivery = sample_delivery("retrying-outcome", Utc::now());
    let id = delivery.delivery_id.clone();
    port.enqueue(delivery).await.unwrap();

    let next = Utc::now() + chrono::Duration::seconds(30);
    port.record_attempt(
        &id,
        WebhookAttemptResult {
            outcome: WebhookAttemptOutcome::Retrying {
                next_attempt_at: next,
            },
            response_status: Some(500),
            error: Some("server error".to_string()),
        },
    )
    .await
    .unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Retrying));
    assert_eq!(loaded.attempt, 1);
    assert_eq!(loaded.next_attempt_at.timestamp(), next.timestamp());
    assert_eq!(loaded.last_error.as_deref(), Some("server error"));
}

/// `record_attempt` with `Dead` sets the terminal status.
pub async fn record_attempt_dead_sets_terminal_status(port: &dyn WebhookDeliveryRepositoryPort) {
    let delivery = sample_delivery("dead-outcome", Utc::now());
    let id = delivery.delivery_id.clone();
    port.enqueue(delivery).await.unwrap();

    port.record_attempt(
        &id,
        WebhookAttemptResult {
            outcome: WebhookAttemptOutcome::Dead,
            response_status: Some(404),
            error: Some("not found".to_string()),
        },
    )
    .await
    .unwrap();

    let loaded = port.get(&id).await.unwrap().unwrap();
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Dead));
    assert_eq!(loaded.attempt, 1);
}

/// `record_attempt` on an unknown id fails `NotFound`.
pub async fn record_attempt_on_unknown_returns_not_found(port: &dyn WebhookDeliveryRepositoryPort) {
    let unknown = WebhookDeliveryId::new_v7();
    let err = port
        .record_attempt(
            &unknown,
            WebhookAttemptResult {
                outcome: WebhookAttemptOutcome::Delivered,
                response_status: Some(200),
                error: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        WebhookDeliveryRepositoryError::NotFound { .. }
    ));
}

// ── list_for_run ─────────────────────────────────────────────────────────

/// `list_for_run` returns only the target run's deliveries, ordered
/// descending by `created_at`.
pub async fn list_for_run_orders_descending_by_created_at(
    port: &dyn WebhookDeliveryRepositoryPort,
) {
    let run_id = RunId::new_v7();
    let thread_id = ThreadId::new("thread-list").unwrap();
    let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    let mut expected_ids = Vec::new();
    for i in 0..3 {
        let mut delivery = WebhookDelivery::new(
            WebhookDeliveryId::new_v7(),
            run_id.clone(),
            thread_id.clone(),
            RunEventKind::Completed,
            "https://example.com/hook",
            "{}",
            base,
        );
        delivery.created_at = base + chrono::Duration::minutes(i);
        expected_ids.push(delivery.delivery_id.clone());
        port.enqueue(delivery).await.unwrap();
    }
    // A delivery for a DIFFERENT run must never appear.
    port.enqueue(sample_delivery("other-run", base))
        .await
        .unwrap();

    let page = port.list_for_run(&run_id, 100, None).await.unwrap();
    let ids: Vec<_> = page.items.iter().map(|d| d.delivery_id.clone()).collect();

    expected_ids.reverse();
    assert_eq!(ids, expected_ids, "must be ordered DESC by created_at");
}
