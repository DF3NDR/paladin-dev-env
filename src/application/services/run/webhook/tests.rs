//! `WebhookDeliveryService` end-to-end tests: the bounded-retry schedule
//! under a paused clock, receiver-side signature verification against the
//! raw captured body, and the wire payload's exact key set (27-13 Task 2,
//! D-40..D-43).

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use paladin_core::platform::container::parley::ParleyResponse;
use paladin_core::platform::container::run::{
    AssistantRef, Run, RunEventKind, RunId, RunStatus, WebhookSpec,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_core::platform::container::webhook::{
    WebhookDelivery, WebhookDeliveryId, WebhookDeliveryStatus,
};
use paladin_ports::output::run_repository_port::{
    RunOutcomeRecord, RunPage, RunQuery, RunRepositoryError, RunRepositoryPort,
};
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository;

use super::service::{WebhookDeliveryOptions, WebhookDeliveryService};
use super::signature::WEBHOOK_SIGNATURE_HEADER;
use super::{WebhookPayload, WebhookPayloadAssistant};

/// A fixed, deterministic clock -- mirrors `schedule::tests::AtomicClock`.
#[derive(Clone)]
struct AtomicClock {
    micros_since_epoch: Arc<std::sync::atomic::AtomicI64>,
}

impl AtomicClock {
    fn new(at: DateTime<Utc>) -> Self {
        Self {
            micros_since_epoch: Arc::new(std::sync::atomic::AtomicI64::new(at.timestamp_micros())),
        }
    }

    fn set(&self, at: DateTime<Utc>) {
        self.micros_since_epoch
            .store(at.timestamp_micros(), Ordering::SeqCst);
    }

    fn now(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_micros(self.micros_since_epoch.load(Ordering::SeqCst))
            .unwrap_or_else(Utc::now)
    }

    fn as_now_fn(&self) -> Arc<dyn Fn() -> DateTime<Utc> + Send + Sync> {
        let this = self.clone();
        Arc::new(move || this.now())
    }
}

fn base_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

use chrono::TimeZone;

fn sample_run(webhook: Option<WebhookSpec>) -> Run {
    let mut run = Run::new(
        RunId::new_v7(),
        ThreadId::new(format!("t-{}", uuid::Uuid::now_v7())).unwrap(),
        AssistantRef {
            assistant_id: "a1".to_string(),
            version: 1,
        },
        serde_json::json!({}),
    );
    if let Some(webhook) = webhook {
        run = run.with_webhook(webhook);
    }
    run
}

fn sample_payload_json(run: &Run, event: RunEventKind, attempt: u32) -> String {
    let payload = WebhookPayload {
        run_id: run.run_id.clone(),
        thread_id: run.thread_id.clone(),
        assistant: WebhookPayloadAssistant {
            assistant_id: run.assistant.assistant_id.clone(),
            version: run.assistant.version,
        },
        status: paladin_core::platform::container::run::RunStatus::Completed,
        event,
        timestamp: Utc::now(),
        attempt,
        parleys: None,
    };
    serde_json::to_string(&payload).unwrap()
}

fn delivery_for(
    run: &Run,
    url: &str,
    payload: &str,
    next_attempt_at: DateTime<Utc>,
) -> WebhookDelivery {
    WebhookDelivery::new(
        WebhookDeliveryId::new_v7(),
        run.run_id.clone(),
        run.thread_id.clone(),
        RunEventKind::Completed,
        url,
        payload,
        next_attempt_at,
    )
}

/// A `RunRepositoryPort` double whose `get` always fails with a backend
/// error -- proves WR-01 (`27-REVIEW.md`): a signing-key load failure must
/// reschedule the delivery rather than send it signed with a fallback empty
/// key. Every other method returns the same minimal value
/// `run_repository_port.rs`'s own `MockRepository` returns; none of them
/// are exercised by `webhook_signing_key_load_failure_reschedules_without_sending`.
struct FailingRunRepository;

#[async_trait::async_trait]
impl RunRepositoryPort for FailingRunRepository {
    async fn insert(&self, _run: &Run) -> Result<(), RunRepositoryError> {
        Ok(())
    }

    async fn get(&self, _run_id: &RunId) -> Result<Option<Run>, RunRepositoryError> {
        Err(RunRepositoryError::Backend {
            source: "simulated backend outage".into(),
        })
    }

    async fn update_status(
        &self,
        _run_id: &RunId,
        _from: RunStatus,
        _to: RunStatus,
        _at: DateTime<Utc>,
    ) -> Result<(), RunRepositoryError> {
        Ok(())
    }

    async fn record_outcome(
        &self,
        _run_id: &RunId,
        _outcome: RunOutcomeRecord,
    ) -> Result<(), RunRepositoryError> {
        Ok(())
    }

    async fn list(&self, _query: RunQuery) -> Result<RunPage, RunRepositoryError> {
        Ok(RunPage {
            items: vec![],
            next_cursor: None,
        })
    }

    async fn active_run_for_thread(
        &self,
        _thread_id: &ThreadId,
    ) -> Result<Option<Run>, RunRepositoryError> {
        Ok(None)
    }

    async fn request_cancel(&self, run_id: &RunId) -> Result<RunStatus, RunRepositoryError> {
        Err(RunRepositoryError::NotFound {
            run_id: run_id.clone(),
        })
    }

    async fn is_cancel_requested(&self, _thread_id: &ThreadId) -> Result<bool, RunRepositoryError> {
        Ok(false)
    }

    async fn bump_attempt(&self, _run_id: &RunId) -> Result<u32, RunRepositoryError> {
        Ok(1)
    }

    async fn record_resume(
        &self,
        _run_id: &RunId,
        _responses: Vec<ParleyResponse>,
    ) -> Result<u32, RunRepositoryError> {
        Ok(1)
    }

    async fn clear_pending_responses(&self, _run_id: &RunId) -> Result<(), RunRepositoryError> {
        Ok(())
    }
}

async fn service_with(
    deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
    runs: Arc<dyn RunRepositoryPort>,
    clock: &AtomicClock,
) -> WebhookDeliveryService {
    // `allow_private: true` -- these tests target a mockito server bound
    // to 127.0.0.1, which the SSRF guard's default (`false`) would
    // otherwise reject at send time before the receiver is ever hit.
    // `send_time_ssrf_rejection_dead_letters_immediately` below is the
    // dedicated proof that the guard's default DOES reject a loopback
    // target.
    let options = WebhookDeliveryOptions {
        now: clock.as_now_fn(),
        allow_private: true,
        ..WebhookDeliveryOptions::default()
    };
    WebhookDeliveryService::new(deliveries, runs, options).unwrap()
}

// ── webhook_retry_schedule (D-43, paused clock) ───────────────────────────

/// Three retry-schedule scenarios driven by the injected clock alone (the
/// same deterministic pattern `schedule::tests::AtomicClock` documents:
/// `tokio::time::pause` freezes tokio TIMERS, not `chrono::Utc::now()`, and
/// mixing a real mockito HTTP round trip with `tokio::time::advance`'s
/// instantaneous multi-second jumps was observed to intermittently starve
/// the underlying connection -- the paused-clock proof for the idle POLL
/// path lives in `spawn_idles_under_paused_clock_without_real_delay` below,
/// which never overlaps a live HTTP call with an `advance`):
/// - a `500`-answering endpoint retries at `backoff_for(1..4)` (1s, 2s, 4s,
///   8s) intervals, dead-lettering with `attempt == 5` after the 5th
///   failure;
/// - a `404`-answering endpoint dead-letters after exactly 1 attempt;
/// - a `500`-then-`200` endpoint delivers at attempt 2.
#[tokio::test]
async fn webhook_retry_schedule() {
    let mut server = mockito::Server::new_async().await;
    let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let t0 = base_time();
    let clock = AtomicClock::new(t0);
    let service = service_with(Arc::clone(&deliveries), Arc::clone(&runs), &clock).await;

    // ── Scenario 1: 5x500 -> Dead with attempt == 5 ───────────────────────
    let always_500_hits = Arc::new(AtomicU32::new(0));
    {
        let hits = Arc::clone(&always_500_hits);
        server
            .mock("POST", "/always-500")
            .with_status_code_from_request(move |_req| {
                hits.fetch_add(1, Ordering::SeqCst);
                500
            })
            .create_async()
            .await;
    }
    let run_a = sample_run(None);
    runs.insert(&run_a).await.unwrap();
    let url_a = format!("{}/always-500", server.url());
    let payload_a = sample_payload_json(&run_a, RunEventKind::Completed, 0);
    let delivery_a = delivery_for(&run_a, &url_a, &payload_a, t0);
    let delivery_a_id = delivery_a.delivery_id.clone();
    deliveries.enqueue(delivery_a).await.unwrap();

    let mut now = t0;
    for attempt in 1..=4u32 {
        let claimed = service.run_once(now).await;
        assert_eq!(
            claimed, 1,
            "attempt {attempt} must claim exactly 1 delivery"
        );
        let loaded = deliveries.get(&delivery_a_id).await.unwrap().unwrap();
        assert_eq!(loaded.attempt, attempt);
        assert!(matches!(loaded.status, WebhookDeliveryStatus::Retrying));

        let delay = super::service::backoff_for(attempt);
        now += chrono::Duration::from_std(delay).unwrap();
        clock.set(now);
    }
    // 5th attempt: new_attempt (5) is NOT < max_attempts (5) -> Dead.
    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);
    let loaded = deliveries.get(&delivery_a_id).await.unwrap().unwrap();
    assert_eq!(loaded.attempt, 5);
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Dead));
    assert_eq!(always_500_hits.load(Ordering::SeqCst), 5);

    // ── Scenario 2: 404 once -> Dead after 1 attempt ──────────────────────
    server
        .mock("POST", "/always-404")
        .with_status(404)
        .create_async()
        .await;
    let run_b = sample_run(None);
    runs.insert(&run_b).await.unwrap();
    let url_b = format!("{}/always-404", server.url());
    let payload_b = sample_payload_json(&run_b, RunEventKind::Completed, 0);
    let delivery_b = delivery_for(&run_b, &url_b, &payload_b, now);
    let delivery_b_id = delivery_b.delivery_id.clone();
    deliveries.enqueue(delivery_b).await.unwrap();

    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);
    let loaded = deliveries.get(&delivery_b_id).await.unwrap().unwrap();
    assert_eq!(loaded.attempt, 1);
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Dead));
    assert_eq!(loaded.last_response_status, Some(404));

    // ── Scenario 3: 500 then 200 -> Delivered at attempt 2 ────────────────
    let flaky_hits = Arc::new(AtomicU32::new(0));
    {
        let hits = Arc::clone(&flaky_hits);
        server
            .mock("POST", "/flaky")
            .with_status_code_from_request(move |_req| {
                let n = hits.fetch_add(1, Ordering::SeqCst);
                if n == 0 { 500 } else { 200 }
            })
            .create_async()
            .await;
    }
    let run_c = sample_run(None);
    runs.insert(&run_c).await.unwrap();
    let url_c = format!("{}/flaky", server.url());
    let payload_c = sample_payload_json(&run_c, RunEventKind::Completed, 0);
    let delivery_c = delivery_for(&run_c, &url_c, &payload_c, now);
    let delivery_c_id = delivery_c.delivery_id.clone();
    deliveries.enqueue(delivery_c).await.unwrap();

    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);
    let loaded = deliveries.get(&delivery_c_id).await.unwrap().unwrap();
    assert_eq!(loaded.attempt, 1);
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Retrying));

    let delay = super::service::backoff_for(1);
    now += chrono::Duration::from_std(delay).unwrap();
    clock.set(now);

    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);
    let loaded = deliveries.get(&delivery_c_id).await.unwrap().unwrap();
    assert_eq!(loaded.attempt, 2);
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Delivered));
}

// ── webhook_signature_verifies_on_receiver (D-41) ─────────────────────────

/// A receiver recomputes the HMAC over the RAW captured body bytes with the
/// run's own signing value and it matches the `X-Paladin-Signature` header
/// exactly.
#[tokio::test]
async fn webhook_signature_verifies_on_receiver() {
    let mut server = mockito::Server::new_async().await;
    type Captured = Arc<Mutex<Option<(Vec<u8>, String)>>>;
    let captured: Captured = Arc::new(Mutex::new(None));

    {
        let captured = Arc::clone(&captured);
        server
            .mock("POST", "/hook")
            .with_status_code_from_request(move |req| {
                let body = req.body().cloned().unwrap_or_default();
                let signature = req
                    .header(WEBHOOK_SIGNATURE_HEADER)
                    .first()
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                *captured.lock().unwrap() = Some((body, signature));
                200
            })
            .create_async()
            .await;
    }

    let key = "webhook-signing-value-0123456789";
    let run = sample_run(Some(WebhookSpec {
        url: format!("{}/hook", server.url()),
        secret: Some(key.to_string()),
        events: vec![RunEventKind::Completed],
    }));
    let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    runs.insert(&run).await.unwrap();

    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let payload = sample_payload_json(&run, RunEventKind::Completed, 1);
    let url = format!("{}/hook", server.url());
    let now = Utc::now();
    let delivery = delivery_for(&run, &url, &payload, now);
    deliveries.enqueue(delivery).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = service_with(Arc::clone(&deliveries), Arc::clone(&runs), &clock).await;
    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);

    let (raw_body, header_signature) = captured.lock().unwrap().clone().expect("receiver was hit");
    assert_eq!(raw_body, payload.as_bytes());

    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(&raw_body);
    let expected = format!(
        "sha256={}",
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(header_signature, expected);
}

// ── webhook_payload_has_no_secret_or_input (T-27-13-03) ───────────────────

/// The wire body's JSON has EXACTLY the documented payload keys -- no
/// `secret`, `input`, or any other field ever appears on the wire.
#[tokio::test]
async fn webhook_payload_has_no_secret_or_input() {
    let mut server = mockito::Server::new_async().await;
    let captured: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    {
        let captured = Arc::clone(&captured);
        server
            .mock("POST", "/hook")
            .with_status_code_from_request(move |req| {
                *captured.lock().unwrap() = req.body().ok().cloned();
                200
            })
            .create_async()
            .await;
    }

    let run = sample_run(None);
    let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    runs.insert(&run).await.unwrap();

    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let payload = sample_payload_json(&run, RunEventKind::Completed, 1);
    let url = format!("{}/hook", server.url());
    let now = Utc::now();
    let delivery = delivery_for(&run, &url, &payload, now);
    deliveries.enqueue(delivery).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = service_with(Arc::clone(&deliveries), Arc::clone(&runs), &clock).await;
    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);

    let raw_body = captured.lock().unwrap().clone().expect("receiver was hit");
    let value: serde_json::Value = serde_json::from_slice(&raw_body).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "assistant",
            "attempt",
            "event",
            "run_id",
            "status",
            "thread_id",
            "timestamp",
        ]
    );
    let raw_str = String::from_utf8(raw_body).unwrap();
    assert!(!raw_str.to_lowercase().contains("secret"));
    assert!(!raw_str.contains("\"input\""));
}

// ── Send-time SSRF rejection dead-letters immediately ─────────────────────

#[tokio::test]
async fn send_time_ssrf_rejection_dead_letters_immediately() {
    let run = sample_run(None);
    let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    runs.insert(&run).await.unwrap();

    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let payload = sample_payload_json(&run, RunEventKind::Completed, 1);
    let now = Utc::now();
    // A loopback URL -- rejected by the default (allow_private: false)
    // guard at send time even though it was never blocked at write time in
    // this direct-enqueue test fixture.
    let delivery = delivery_for(&run, "http://127.0.0.1:1/hook", &payload, now);
    let delivery_id = delivery.delivery_id.clone();
    deliveries.enqueue(delivery).await.unwrap();

    // Deliberately NOT `service_with` (which sets `allow_private: true` for
    // the mockito-targeting tests above) -- this test proves the guard's
    // real default (`allow_private: false`).
    let options = WebhookDeliveryOptions {
        now: AtomicClock::new(now).as_now_fn(),
        ..WebhookDeliveryOptions::default()
    };
    let service =
        WebhookDeliveryService::new(Arc::clone(&deliveries), Arc::clone(&runs), options).unwrap();
    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);

    let loaded = deliveries.get(&delivery_id).await.unwrap().unwrap();
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Dead));
    assert_eq!(loaded.attempt, 1);
    assert!(loaded.last_error.is_some());
}

// ── spawn's idle poll under a paused clock ────────────────────────────────

/// [`WebhookDeliveryService::spawn`]'s idle poll-interval sleep resolves
/// instantly under a paused clock rather than costing real test wall-clock
/// time -- proves the drain loop's shutdown-registration and idle-sleep
/// path without ever overlapping a live HTTP call with a `tokio::time`
/// jump (see `webhook_retry_schedule`'s own doc comment for why that
/// combination is avoided there).
#[tokio::test(start_paused = true)]
async fn spawn_idles_under_paused_clock_without_real_delay() {
    let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());
    let clock = AtomicClock::new(Utc::now());
    let options = WebhookDeliveryOptions {
        now: clock.as_now_fn(),
        poll_interval: std::time::Duration::from_secs(30),
        ..WebhookDeliveryOptions::default()
    };
    let service = Arc::new(WebhookDeliveryService::new(deliveries, runs, options).unwrap());

    let coordinator = paladin_battalion::engine::shutdown::ShutdownCoordinator::new();
    let handle = service.spawn(&coordinator);

    // Nothing is due, so the loop's first iteration enters its
    // `poll_interval` sleep -- under `start_paused`, jumping the virtual
    // clock past it resolves instantly rather than costing 30 real seconds.
    tokio::time::advance(std::time::Duration::from_secs(31)).await;

    let outcome = coordinator
        .cancel_and_wait(std::time::Duration::from_secs(5))
        .await;
    assert!(outcome.drained(), "the idle poll loop must drain cleanly");
    handle.await.unwrap();
}

// ── webhook_signing_key_load_failure_reschedules_without_sending (WR-01) ──

/// When the run lookup that supplies the signing secret fails with a
/// backend error, NO HTTP request is issued -- proven by a mock with
/// `expect(0)` -- and the delivery ends `Retrying` with a `next_attempt_at`
/// strictly after the clock's current value (`27-REVIEW.md` WR-01).
#[tokio::test]
async fn webhook_signing_key_load_failure_reschedules_without_sending() {
    let mut server = mockito::Server::new_async().await;
    let target = server
        .mock("POST", "/hook")
        .with_status(200)
        .expect(0)
        .create_async()
        .await;

    let runs: Arc<dyn RunRepositoryPort> = Arc::new(FailingRunRepository);
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(InMemoryWebhookDeliveryRepository::new());

    // The delivery references a run id `FailingRunRepository` never looks
    // up successfully -- its `get` always errs regardless of the id.
    let run_id = RunId::new_v7();
    let thread_id = ThreadId::new(format!("t-{}", uuid::Uuid::now_v7())).unwrap();
    let payload = serde_json::to_string(&WebhookPayload {
        run_id: run_id.clone(),
        thread_id: thread_id.clone(),
        assistant: WebhookPayloadAssistant {
            assistant_id: "a1".to_string(),
            version: 1,
        },
        status: paladin_core::platform::container::run::RunStatus::Completed,
        event: RunEventKind::Completed,
        timestamp: Utc::now(),
        attempt: 0,
        parleys: None,
    })
    .unwrap();

    let now = base_time();
    let url = format!("{}/hook", server.url());
    let delivery = WebhookDelivery::new(
        WebhookDeliveryId::new_v7(),
        run_id,
        thread_id,
        RunEventKind::Completed,
        &url,
        &payload,
        now,
    );
    let delivery_id = delivery.delivery_id.clone();
    deliveries.enqueue(delivery).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = service_with(Arc::clone(&deliveries), Arc::clone(&runs), &clock).await;
    let claimed = service.run_once(now).await;
    assert_eq!(claimed, 1);

    target.expect(0).assert_async().await;

    let loaded = deliveries.get(&delivery_id).await.unwrap().unwrap();
    assert!(matches!(loaded.status, WebhookDeliveryStatus::Retrying));
    assert!(
        loaded.next_attempt_at > now,
        "next_attempt_at must be strictly after the clock's current value, got {:?}",
        loaded.next_attempt_at
    );
    assert!(loaded.last_error.is_some());
    assert!(loaded.last_response_status.is_none());
}
