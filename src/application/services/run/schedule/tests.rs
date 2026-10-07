//! `ScheduleService` tests (PLAT-05, D-37, D-39, D-52).
//!
//! Every timestamp comes from an injected, fully-deterministic `now`
//! closure -- never real wall-clock time -- so these tests never sleep and
//! never depend on `tokio::time::pause` (which freezes tokio TIMERS, not
//! `chrono::Utc::now()`; see `service.rs`'s own module docs).

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use paladin_core::platform::container::assistant::AssistantSource;
use paladin_core::platform::container::principal::PrincipalRef;
use paladin_core::platform::container::run::{AssistantRef, RunId, WebhookSpec};
use paladin_core::platform::container::run_schedule::{
    OnMissed, RunSchedule, RunScheduleId, RunScheduleUpdate, ThreadStrategy,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{
    CancelOutcome, ForkRun, RunAccepted, RunSubmissionError, RunSubmissionPort, SubmitRun,
};
use paladin_ports::input::schedule_admin_port::{
    CreateRunSchedule, ScheduleAdminError, ScheduleAdminPort,
};
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_storage::run_schedule::in_memory::InMemoryRunScheduleRepository;

use super::super::resolver::{AssistantResolver, ResolveError, ResolvedAssistant, Runnable};
use super::service::{ScheduleService, ScheduleServiceOptions, ScheduleTickOutcome, SkipReason};

/// A [`RunSubmissionPort`] test double: always succeeds unless `busy_thread`
/// names the requested thread, in which case it returns `ThreadBusy`.
/// Records every accepted [`SubmitRun`] for assertions.
#[derive(Default)]
struct RecordingSubmission {
    submitted: Mutex<Vec<SubmitRun>>,
    busy_thread: Mutex<Option<ThreadId>>,
}

impl RecordingSubmission {
    fn submitted_count(&self) -> usize {
        self.submitted.lock().unwrap().len()
    }

    fn set_busy(&self, thread_id: ThreadId) {
        *self.busy_thread.lock().unwrap() = Some(thread_id);
    }
}

#[async_trait]
impl RunSubmissionPort for RecordingSubmission {
    async fn submit(&self, request: SubmitRun) -> Result<RunAccepted, RunSubmissionError> {
        let thread_id = request
            .thread_id
            .clone()
            .unwrap_or_else(|| ThreadId::new(uuid::Uuid::now_v7().to_string()).unwrap());

        if self.busy_thread.lock().unwrap().as_ref() == Some(&thread_id) {
            return Err(RunSubmissionError::ThreadBusy { thread_id });
        }

        self.submitted.lock().unwrap().push(request);
        Ok(RunAccepted {
            run_id: RunId::new_v7(),
            thread_id,
        })
    }

    async fn cancel(
        &self,
        _run_id: &RunId,
        _requested_by: Option<PrincipalRef>,
    ) -> Result<CancelOutcome, RunSubmissionError> {
        Err(RunSubmissionError::NotWired)
    }

    async fn fork(&self, request: ForkRun) -> Result<RunAccepted, RunSubmissionError> {
        Ok(RunAccepted {
            run_id: RunId::new_v7(),
            thread_id: request.thread_id,
        })
    }
}

/// A fixed, deterministic clock: `now()` returns whatever `set` last stored.
/// `ScheduleServiceOptions::now` is `Arc<dyn Fn() -> DateTime<Utc>>`, so a
/// second `Arc<AtomicClock>` clone shares the SAME underlying cell as the
/// closure captured by the service.
#[derive(Clone)]
struct AtomicClock {
    micros_since_epoch: Arc<AtomicI64>,
}

impl AtomicClock {
    fn new(at: DateTime<Utc>) -> Self {
        Self {
            micros_since_epoch: Arc::new(AtomicI64::new(at.timestamp_micros())),
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
    // Minute-aligned so `*/1 * * * *` (every minute) fires at exact minute
    // boundaries -- required for `cron.next_after` to advance in clean,
    // predictable 60s steps.
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

fn options_with(clock: &AtomicClock, tick_interval_secs: u64) -> ScheduleServiceOptions {
    ScheduleServiceOptions {
        tick_interval: Duration::from_secs(tick_interval_secs),
        now: clock.as_now_fn(),
    }
}

// ── schedule_restart_exactly_once (PRD acceptance 5) ──────────────────────

/// A schedule restarted across a tick boundary fires exactly once per tick,
/// never zero and never twice: service A claims and fires the tick at `T`;
/// A is dropped ("restart"); service B over the SAME repository ticks at
/// `T + 30s` (still not due -- 0 runs) then at `T + 60s` (due again -- 1
/// run). Total across A and B: exactly 2.
#[tokio::test(flavor = "multi_thread")]
async fn schedule_restart_exactly_once() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let t0 = base_time();

    let schedule =
        RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "*/1 * * * *").with_next_tick(t0);
    let schedule_id = schedule.schedule_id.clone();
    repo.insert(schedule).await.unwrap();

    // Service A ticks at T, claims, submits (1 run), then is dropped.
    let clock_a = AtomicClock::new(t0);
    let service_a = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock_a, 60),
    ));
    let outcomes_a = service_a.tick_once().await;
    assert_eq!(outcomes_a.len(), 1);
    assert!(matches!(
        &outcomes_a[0],
        ScheduleTickOutcome::Fired { schedule_id: id, .. } if *id == schedule_id
    ));
    drop(service_a);

    // Service B ("restart"), same repository, ticks at T + 30s -- the
    // schedule's next_tick is now T + 60s, so this is not due yet.
    let clock_b = AtomicClock::new(t0 + chrono::Duration::seconds(30));
    let service_b = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock_b, 60),
    ));
    let outcomes_b_early = service_b.tick_once().await;
    assert!(
        outcomes_b_early.is_empty(),
        "not yet due at T + 30s, expected no outcomes, got {outcomes_b_early:?}"
    );

    // B ticks again at T + 60s: now due, fires exactly once.
    clock_b.set(t0 + chrono::Duration::seconds(60));
    let outcomes_b_due = service_b.tick_once().await;
    assert_eq!(outcomes_b_due.len(), 1);
    assert!(matches!(
        &outcomes_b_due[0],
        ScheduleTickOutcome::Fired { schedule_id: id, .. } if *id == schedule_id
    ));

    assert_eq!(
        submission.submitted_count(),
        2,
        "exactly 2 runs total across the restart, never 1 (missed) or 3+ (double-fired)"
    );
}

/// `OnMissed::Skip` variant of the restart scenario: no service ticks
/// between `T` and `T + 5min` (a long gap simulating downtime). B's FIRST
/// tick, at `T + 5min`, discovers the tick is missed (`> 2 * tick_interval`
/// late) and -- because `on_missed == Skip` -- submits nothing, only
/// recomputing `next_tick` from NOW (not from the stale original tick).
#[tokio::test(flavor = "multi_thread")]
async fn schedule_restart_exactly_once_on_missed_skip() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let t0 = base_time();

    let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *")
        .with_next_tick(t0)
        .with_on_missed(OnMissed::Skip);
    let schedule_id = schedule.schedule_id.clone();
    repo.insert(schedule).await.unwrap();

    let now = t0 + chrono::Duration::minutes(5);
    let clock = AtomicClock::new(now);
    let service = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    ));

    let outcomes = service.tick_once().await;
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        &outcomes[0],
        ScheduleTickOutcome::Skipped { schedule_id: id, reason: SkipReason::Missed } if *id == schedule_id
    ));
    assert_eq!(submission.submitted_count(), 0, "Skip must submit nothing");

    let loaded = repo.get(&schedule_id).await.unwrap().unwrap();
    let expected_next = loaded.next_tick.expect("next_tick must be set");
    assert!(
        expected_next > now,
        "next_tick must be recomputed strictly after NOW ({now}), got {expected_next}"
    );
}

/// `OnMissed::RunOnce` variant: the SAME missed-by-5-minutes scenario, but
/// with `on_missed == RunOnce` -- exactly ONE run fires for the missed
/// window, then `next_tick` recomputes from NOW.
#[tokio::test(flavor = "multi_thread")]
async fn schedule_restart_exactly_once_on_missed_run_once() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let t0 = base_time();

    let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *")
        .with_next_tick(t0)
        .with_on_missed(OnMissed::RunOnce);
    let schedule_id = schedule.schedule_id.clone();
    repo.insert(schedule).await.unwrap();

    let now = t0 + chrono::Duration::minutes(5);
    let clock = AtomicClock::new(now);
    let service = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    ));

    let outcomes = service.tick_once().await;
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        &outcomes[0],
        ScheduleTickOutcome::Fired { schedule_id: id, .. } if *id == schedule_id
    ));
    assert_eq!(
        submission.submitted_count(),
        1,
        "RunOnce must submit exactly one run for the missed window"
    );

    let loaded = repo.get(&schedule_id).await.unwrap().unwrap();
    let expected_next = loaded.next_tick.expect("next_tick must be set");
    assert!(
        expected_next > now,
        "next_tick must be recomputed strictly after NOW ({now}), got {expected_next}"
    );
}

// ── two_services_one_tick_exactly_one_fire (D-52 concurrency stress) ──────

/// Two `ScheduleService` instances over ONE `InMemoryRunScheduleRepository`
/// call `tick_once` CONCURRENTLY against 50 simultaneously-due schedules --
/// exactly 50 runs are submitted in total (one per schedule), never 100.
#[tokio::test(flavor = "multi_thread")]
async fn two_services_one_tick_exactly_one_fire() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let now = base_time();

    for i in 0..50 {
        let schedule = RunSchedule::new(
            RunScheduleId::new_v7(),
            format!("assistant-{i}"),
            "*/1 * * * *",
        )
        .with_next_tick(now);
        repo.insert(schedule).await.unwrap();
    }

    let clock = AtomicClock::new(now);
    let service_a = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    ));
    let service_b = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    ));

    let outcomes = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(service_a.tick_once(), service_b.tick_once())
    })
    .await
    .expect("two concurrent tick_once calls over 50 schedules must not hang");

    let fired_a = outcomes
        .0
        .iter()
        .filter(|o| matches!(o, ScheduleTickOutcome::Fired { .. }))
        .count();
    let fired_b = outcomes
        .1
        .iter()
        .filter(|o| matches!(o, ScheduleTickOutcome::Fired { .. }))
        .count();

    assert_eq!(
        fired_a + fired_b,
        50,
        "exactly 50 fires total across both services, never fewer and never 100"
    );
    assert_eq!(
        submission.submitted_count(),
        50,
        "exactly 50 runs actually submitted"
    );
}

// ── fixed_thread_busy_increments_skipped (D-39) ────────────────────────────

/// A `FixedThread` schedule whose thread already has an active run skips
/// (does not submit) and increments `skipped_ticks` on the row.
#[tokio::test(flavor = "multi_thread")]
async fn fixed_thread_busy_increments_skipped() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let now = base_time();

    let fixed_thread = ThreadId::new("fixed-busy-thread").unwrap();
    submission.set_busy(fixed_thread.clone());

    let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *")
        .with_next_tick(now)
        .with_thread_strategy(ThreadStrategy::FixedThread(fixed_thread));
    let schedule_id = schedule.schedule_id.clone();
    repo.insert(schedule).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    ));

    let outcomes = service.tick_once().await;
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        &outcomes[0],
        ScheduleTickOutcome::Skipped { schedule_id: id, reason: SkipReason::ThreadBusy } if *id == schedule_id
    ));
    assert_eq!(
        submission.submitted_count(),
        0,
        "no second run was submitted"
    );

    let loaded = repo.get(&schedule_id).await.unwrap().unwrap();
    assert_eq!(loaded.skipped_ticks, 1);

    // The tick was still CLAIMED (the row advanced) even though nothing was
    // submitted -- a busy-thread skip is not a lost race, so a second call
    // at the SAME now should see nothing due (already claimed).
    let second_pass = service.tick_once().await;
    assert!(second_pass.is_empty());
}

// ── Additional coverage ─────────────────────────────────────────────────

/// A schedule that is not yet due produces no outcome.
#[tokio::test]
async fn not_yet_due_schedule_produces_no_outcome() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let now = base_time();

    let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *")
        .with_next_tick(now + chrono::Duration::hours(1));
    repo.insert(schedule).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    );

    let outcomes = service.tick_once().await;
    assert!(outcomes.is_empty());
}

/// A disabled schedule, even if its `next_tick` is due, is never returned by
/// `due()` and therefore produces no outcome.
#[tokio::test]
async fn disabled_schedule_never_ticks() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let now = base_time();

    let mut schedule =
        RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *").with_next_tick(now);
    schedule.enabled = false;
    repo.insert(schedule).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    );

    let outcomes = service.tick_once().await;
    assert!(outcomes.is_empty());
    assert_eq!(submission.submitted_count(), 0);
}

/// `ScheduleService::spawn` drives `tick_once` on its own interval and stops
/// cleanly when the coordinator is cancelled.
#[tokio::test(flavor = "multi_thread")]
async fn spawn_ticks_on_interval_and_stops_on_shutdown() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let submission = Arc::new(RecordingSubmission::default());
    let now = base_time();

    let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "* * * * *")
        // 1ms late is "due" but well under the 20ms-tick_interval's 40ms
        // "missed" threshold below -- this test is about the interval loop
        // firing/stopping, not the missed-tick policy (covered separately).
        .with_next_tick(now - chrono::Duration::milliseconds(1));
    repo.insert(schedule).await.unwrap();

    let clock = AtomicClock::new(now);
    let service = Arc::new(ScheduleService::new(
        Arc::clone(&repo),
        Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
        ScheduleServiceOptions {
            tick_interval: Duration::from_millis(20),
            now: clock.as_now_fn(),
        },
    ));

    let coordinator = paladin_battalion::engine::shutdown::ShutdownCoordinator::new();
    let handle = service.spawn(&coordinator);

    tokio::time::sleep(Duration::from_millis(100)).await;
    coordinator.cancel_and_wait(Duration::from_secs(5)).await;
    handle.await.unwrap();

    assert_eq!(
        submission.submitted_count(),
        1,
        "the one due schedule fires exactly once even across multiple interval ticks"
    );
}

// ── `ScheduleAdminPort` tests (Task 1, PLAT-05, D-42, D-46) ────────────────
//
// `admin.rs`'s `impl ScheduleAdminPort for ScheduleService` -- create/get/
// list/patch/delete, cron+timezone+assistant+webhook validation, and the
// write-time SSRF guard. `admin.rs`'s own `ssrf_guard_tests` module covers
// `SsrfGuard::check_url`'s classification table directly; these tests cover
// the port's validate-then-persist orchestration around it.

/// A minimal [`AssistantResolver`] test double: resolves any id present in
/// `known` (up to its recorded `latest` version), rejects everything else.
struct MockResolver {
    known: Vec<(String, u32)>,
}

#[async_trait]
impl AssistantResolver for MockResolver {
    async fn resolve(
        &self,
        assistant_id: &str,
        version: Option<u32>,
    ) -> Result<ResolvedAssistant, ResolveError> {
        let Some((_, latest)) = self.known.iter().find(|(id, _)| id == assistant_id) else {
            return Err(ResolveError::UnknownAssistant {
                assistant_id: assistant_id.to_string(),
            });
        };
        let resolved_version = version.unwrap_or(*latest);
        if resolved_version > *latest {
            return Err(ResolveError::UnknownVersion {
                assistant_id: assistant_id.to_string(),
                version: resolved_version,
            });
        }
        Ok(ResolvedAssistant {
            reference: AssistantRef {
                assistant_id: assistant_id.to_string(),
                version: resolved_version,
            },
            runnable: Runnable::Agent(Arc::new(paladin_core::base::entity::node::Node::new(
                paladin_core::platform::container::paladin::PaladinData::default(),
                Some(assistant_id.to_string()),
            ))),
            allowed_roles: vec![],
            source: AssistantSource::Stored,
        })
    }
}

/// Build a `ScheduleService` wired with a [`MockResolver`] that only knows
/// `"assistant-1"` (latest version `3`) -- used by every admin-port test
/// below.
fn admin_service(repo: Arc<dyn RunScheduleRepositoryPort>, clock: &AtomicClock) -> ScheduleService {
    let submission = Arc::new(RecordingSubmission::default());
    let resolver: Arc<dyn AssistantResolver> = Arc::new(MockResolver {
        known: vec![("assistant-1".to_string(), 3)],
    });
    ScheduleService::new(
        repo,
        submission as Arc<dyn RunSubmissionPort>,
        options_with(clock, 60),
    )
    .with_resolver(resolver)
}

fn create_request(assistant_id: &str, cron: &str) -> CreateRunSchedule {
    CreateRunSchedule {
        assistant_id: assistant_id.to_string(),
        version: None,
        cron: cron.to_string(),
        timezone: None,
        input: serde_json::Value::Null,
        enabled: true,
        thread_strategy: None,
        on_missed: None,
        webhook: None,
        created_by: None,
    }
}

#[tokio::test]
async fn create_stamps_the_creators_attribution() {
    use paladin_core::platform::container::principal::{RunAttribution, TenantId};

    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);
    let creator = RunAttribution::new(TenantId::new("acme").unwrap(), "ops");

    let mut request = create_request("assistant-1", "*/1 * * * *");
    request.created_by = Some(creator.clone());
    let created = service.create(request).await.unwrap();

    assert_eq!(created.created_by, Some(creator.clone()));
    let stored = repo.get(&created.schedule_id).await.unwrap().unwrap();
    assert_eq!(stored.created_by, Some(creator.clone()));

    // A patch never re-assigns the creator (D-08).
    service
        .patch(
            &created.schedule_id,
            RunScheduleUpdate {
                cron: Some("0 * * * *".to_string()),
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let patched = repo.get(&created.schedule_id).await.unwrap().unwrap();
    assert_eq!(patched.created_by, Some(creator));
}

#[tokio::test]
async fn create_without_a_creator_stores_none() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();

    assert!(created.created_by.is_none());
    let stored = repo.get(&created.schedule_id).await.unwrap().unwrap();
    assert!(stored.created_by.is_none());
}

#[tokio::test]
async fn create_sets_first_next_tick() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let now = base_time();
    let clock = AtomicClock::new(now);
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();

    let next_tick = created.next_tick.expect("next_tick must be set on create");
    assert!(next_tick > now);
}

#[tokio::test]
async fn create_rejects_bad_cron() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let err = service
        .create(create_request("assistant-1", "not a cron"))
        .await
        .unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(violations.iter().any(|v| v.path == "/cron"));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[tokio::test]
async fn create_rejects_unknown_timezone() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let mut request = create_request("assistant-1", "*/1 * * * *");
    request.timezone = Some("Not/AZone".to_string());
    let err = service.create(request).await.unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(violations.iter().any(|v| v.path == "/timezone"));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[tokio::test]
async fn create_rejects_unknown_assistant() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let err = service
        .create(create_request("nope", "*/1 * * * *"))
        .await
        .unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(violations.iter().any(|v| v.path == "/assistant_id"));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }

    // Nothing was persisted: the repository's page stays empty.
    let page = repo.list(10, None).await.unwrap();
    assert!(page.items.is_empty());
}

#[tokio::test]
async fn create_rejects_unknown_version() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let mut request = create_request("assistant-1", "*/1 * * * *");
    request.version = Some(99);
    let err = service.create(request).await.unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(violations.iter().any(|v| v.path == "/version"));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[tokio::test]
async fn create_rejects_webhook_url_via_ssrf_guard() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let mut request = create_request("assistant-1", "*/1 * * * *");
    request.webhook = Some(WebhookSpec {
        url: "http://127.0.0.1/hook".to_string(),
        secret: None,
        events: vec![],
    });
    let err = service.create(request).await.unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(
                violations
                    .iter()
                    .any(|v| v.path == "/webhook/url" && v.code == "webhook_url_rejected")
            );
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[tokio::test]
async fn patch_cron_recomputes_next_tick() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let now = base_time();
    let clock = AtomicClock::new(now);
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();
    let original_next = created.next_tick.expect("next_tick set on create");

    clock.set(now + chrono::Duration::seconds(30));
    let update = RunScheduleUpdate {
        cron: Some("*/5 * * * *".to_string()),
        ..Default::default()
    };
    let patched = service.patch(&created.schedule_id, update).await.unwrap();

    assert_eq!(patched.cron, "*/5 * * * *");
    let new_next = patched.next_tick.expect("next_tick recomputed on patch");
    assert_ne!(new_next, original_next);
    assert!(new_next > now + chrono::Duration::seconds(30));
}

#[tokio::test]
async fn patch_webhook_rejected_by_guard() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();

    let update = RunScheduleUpdate {
        webhook: Some(WebhookSpec {
            url: "http://169.254.169.254/latest/meta-data".to_string(),
            secret: None,
            events: vec![],
        }),
        ..Default::default()
    };
    let err = service
        .patch(&created.schedule_id, update)
        .await
        .unwrap_err();
    match err {
        ScheduleAdminError::Invalid { violations } => {
            assert!(violations.iter().any(|v| v.path == "/webhook/url"));
        }
        other => panic!("expected Invalid, got {other:?}"),
    }

    // Nothing persisted -- the schedule's webhook is still unset.
    let reloaded = repo.get(&created.schedule_id).await.unwrap().unwrap();
    assert!(reloaded.webhook.is_none());
}

#[tokio::test]
async fn patch_enabled_false_leaves_next_tick_in_place() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();
    let original_next = created.next_tick;

    let update = RunScheduleUpdate {
        enabled: Some(false),
        ..Default::default()
    };
    let patched = service.patch(&created.schedule_id, update).await.unwrap();

    assert!(!patched.enabled);
    assert_eq!(patched.next_tick, original_next);
}

#[tokio::test]
async fn patch_unknown_schedule_is_not_found() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let err = service
        .patch(&RunScheduleId::new_v7(), RunScheduleUpdate::default())
        .await
        .unwrap_err();
    assert!(matches!(err, ScheduleAdminError::NotFound { .. }));
}

#[tokio::test]
async fn delete_then_get_is_none() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let created = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();

    service.delete(&created.schedule_id).await.unwrap();
    let after = service.get(&created.schedule_id).await.unwrap();
    assert!(after.is_none());
}

#[tokio::test]
async fn delete_unknown_schedule_is_not_found() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let err = service.delete(&RunScheduleId::new_v7()).await.unwrap_err();
    assert!(matches!(err, ScheduleAdminError::NotFound { .. }));
}

#[tokio::test]
async fn list_pages_created_schedules() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let clock = AtomicClock::new(base_time());
    let service = admin_service(Arc::clone(&repo), &clock);

    let _ = service
        .create(create_request("assistant-1", "*/1 * * * *"))
        .await
        .unwrap();
    let _ = service
        .create(create_request("assistant-1", "*/2 * * * *"))
        .await
        .unwrap();

    let page = service.list(10, None).await.unwrap();
    assert_eq!(page.items.len(), 2);
}

// ── Phase 41 (41-05, D-08): schedule-fired runs are attributed and admitted ──

mod allowance {
    use super::*;

    use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};
    use paladin_ports::output::run_queue_port::RunQueuePort;
    use paladin_ports::output::run_repository_port::{RunQuery, RunRepositoryPort};
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
    use paladin_storage::run::in_memory::InMemoryRunRepository;
    use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
    use paladin_storage::treasury::contract_tests::{settle_request, usd};
    use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

    use crate::application::services::run::submission::RunSubmissionService;
    use crate::application::services::treasurer::Treasurer;
    use crate::config::treasurer::TreasurerConfig;

    fn attribution(tenant: &str, key: &str) -> RunAttribution {
        RunAttribution::new(TenantId::new(tenant).unwrap(), key)
    }

    /// A real `RunSubmissionService` over in-memory stores with a real `Treasurer` built from
    /// `allowance` (the `treasurer.allowance` JSON an operator would write).
    struct Harness {
        submission: Arc<dyn RunSubmissionPort>,
        runs: Arc<dyn RunRepositoryPort>,
        queue: Arc<dyn RunQueuePort>,
        ledger: Arc<dyn TreasuryLedgerPort>,
        repo: Arc<dyn RunScheduleRepositoryPort>,
    }

    impl Harness {
        fn new(allowance: serde_json::Value) -> Self {
            let runs: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
            let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
            let ledger: Arc<dyn TreasuryLedgerPort> = Arc::new(InMemoryTreasuryLedger::new());
            let resolver: Arc<dyn AssistantResolver> = Arc::new(MockResolver {
                known: vec![("assistant-1".to_string(), 3)],
            });
            // The scheduled assistant is agent-kind (`MockResolver`), and an agent-kind
            // submission under a ceiling is admitted WITH its model (Phase 42 D-10): the
            // model must carry a `treasurer.pricing` row or the tick is refused.
            let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
                "currency": "USD",
                "pricing": { "gpt-4": { "prompt": "10.00", "completion": "10.00" } },
                "allowance": allowance,
            }))
            .unwrap();
            let treasurer = Treasurer::new(config.allowance_policy().unwrap(), Arc::clone(&ledger))
                .with_pricing(Arc::new(config.price_table().unwrap()));
            let submission: Arc<dyn RunSubmissionPort> = Arc::new(
                RunSubmissionService::new(Arc::clone(&runs), Arc::clone(&queue), resolver)
                    .with_treasurer(Arc::new(treasurer)),
            );
            Self {
                submission,
                runs,
                queue,
                ledger,
                repo: Arc::new(InMemoryRunScheduleRepository::new()),
            }
        }

        /// Spend `2.50` USD under `(tenant, key)` -- the whole ceiling the fixtures grant.
        async fn spend_everything(&self, tenant: &str, key: &str) {
            self.ledger
                .settle(settle_request(
                    LedgerScope::new(tenant, key),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    2_500_000_000,
                    usd(),
                    "gpt-4",
                ))
                .await
                .unwrap();
        }

        /// Insert a due schedule created by `created_by` and tick once.
        async fn tick(
            &self,
            created_by: Option<RunAttribution>,
        ) -> (RunScheduleId, Vec<ScheduleTickOutcome>) {
            let t0 = base_time();
            let mut schedule =
                RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "*/1 * * * *")
                    .with_next_tick(t0);
            if let Some(creator) = created_by {
                schedule = schedule.with_created_by(creator);
            }
            let schedule_id = schedule.schedule_id.clone();
            self.repo.insert(schedule).await.unwrap();

            let clock = AtomicClock::new(t0);
            let service = ScheduleService::new(
                Arc::clone(&self.repo),
                Arc::clone(&self.submission),
                options_with(&clock, 60),
            );
            (schedule_id, service.tick_once().await)
        }

        async fn run_count(&self) -> usize {
            self.runs
                .list(RunQuery::default())
                .await
                .unwrap()
                .items
                .len()
        }
    }

    fn api_key_ceiling(key: &str) -> serde_json::Value {
        serde_json::json!({ "api_keys": { key: {
            "period": "1d", "amount": "2.50", "lifetime": "2.50"
        } } })
    }

    fn tenant_ceiling(tenant: &str) -> serde_json::Value {
        serde_json::json!({ "tenants": { tenant: {
            "period": "1d", "amount": "2.50", "lifetime": "2.50"
        } } })
    }

    #[tokio::test]
    async fn tick_for_an_exhausted_creator_is_skipped_and_counted() {
        let harness = Harness::new(api_key_ceiling("ops"));
        harness.spend_everything("acme", "ops").await;

        let (schedule_id, outcomes) = harness.tick(Some(attribution("acme", "ops"))).await;

        assert_eq!(
            outcomes,
            vec![ScheduleTickOutcome::Skipped {
                schedule_id: schedule_id.clone(),
                reason: SkipReason::AllowanceExhausted,
            }]
        );
        let reloaded = harness.repo.get(&schedule_id).await.unwrap().unwrap();
        assert_eq!(reloaded.skipped_ticks, 1, "the skip must be counted");
        assert_eq!(harness.run_count().await, 0, "no run row may be written");
        assert_eq!(
            harness.queue.depth().await.unwrap(),
            0,
            "nothing may be queued"
        );
    }

    #[tokio::test]
    async fn tick_for_a_schedule_without_a_creator_fires_unattributed_and_ungated() {
        // Even with the only ceiling in the policy fully spent, a schedule with no recorded
        // creator behaves exactly as before Phase 41: unattributed and never gated.
        let harness = Harness::new(api_key_ceiling("ops"));
        harness.spend_everything("acme", "ops").await;

        let (schedule_id, outcomes) = harness.tick(None).await;

        let [ScheduleTickOutcome::Fired { run_id, .. }] = outcomes.as_slice() else {
            panic!("expected one Fired outcome, got {outcomes:?}");
        };
        let run = harness.runs.get(run_id).await.unwrap().unwrap();
        assert!(
            run.submitted_by.is_none(),
            "a legacy schedule fires unattributed"
        );
        let reloaded = harness.repo.get(&schedule_id).await.unwrap().unwrap();
        assert_eq!(reloaded.skipped_ticks, 0);
    }

    #[tokio::test]
    async fn tick_fires_attributed_to_the_creator() {
        // The fire site hands the creator over as identity only: `requested_by` stays `None`.
        let repo: Arc<dyn RunScheduleRepositoryPort> =
            Arc::new(InMemoryRunScheduleRepository::new());
        let submission = Arc::new(RecordingSubmission::default());
        let t0 = base_time();
        let creator = attribution("acme", "ops");
        let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "*/1 * * * *")
            .with_next_tick(t0)
            .with_created_by(creator.clone());
        repo.insert(schedule).await.unwrap();

        let clock = AtomicClock::new(t0);
        let service = ScheduleService::new(
            Arc::clone(&repo),
            Arc::clone(&submission) as Arc<dyn RunSubmissionPort>,
            options_with(&clock, 60),
        );
        let outcomes = service.tick_once().await;
        assert!(matches!(
            outcomes.as_slice(),
            [ScheduleTickOutcome::Fired { .. }]
        ));

        {
            let submitted = submission.submitted.lock().unwrap();
            assert_eq!(submitted.len(), 1);
            assert!(submitted[0].requested_by.is_none());
            assert_eq!(submitted[0].attributed_to, Some(creator));
        }

        // Through the real service the run carries the creator, so its spend settles under it.
        let harness = Harness::new(api_key_ceiling("ops"));
        let (_, outcomes) = harness.tick(Some(attribution("acme", "ops"))).await;
        let [ScheduleTickOutcome::Fired { run_id, .. }] = outcomes.as_slice() else {
            panic!("expected one Fired outcome, got {outcomes:?}");
        };
        let run = harness.runs.get(run_id).await.unwrap().unwrap();
        assert_eq!(run.submitted_by, Some(attribution("acme", "ops")));
    }

    #[tokio::test]
    async fn removed_creator_key_is_gated_by_its_tenant_allowance_only() {
        // The policy has only a tenant entry: the creator's key (`gone-key`) was removed from
        // `treasurer.allowance.api_keys`. The schedule stays attributed by its persisted names.
        let exhausted = Harness::new(tenant_ceiling("acme"));
        exhausted.spend_everything("acme", "someone-else").await;
        let (schedule_id, outcomes) = exhausted.tick(Some(attribution("acme", "gone-key"))).await;
        assert_eq!(
            outcomes,
            vec![ScheduleTickOutcome::Skipped {
                schedule_id,
                reason: SkipReason::AllowanceExhausted,
            }],
            "an exhausted tenant allowance still gates the removed key's schedule"
        );
        assert_eq!(exhausted.run_count().await, 0);

        let headroom = Harness::new(tenant_ceiling("acme"));
        let (_, outcomes) = headroom.tick(Some(attribution("acme", "gone-key"))).await;
        let [ScheduleTickOutcome::Fired { run_id, .. }] = outcomes.as_slice() else {
            panic!("expected one Fired outcome, got {outcomes:?}");
        };
        let run = headroom.runs.get(run_id).await.unwrap().unwrap();
        assert_eq!(
            run.submitted_by,
            Some(attribution("acme", "gone-key")),
            "the run is attributed by the persisted names"
        );
    }
}

/// A [`RunSubmissionPort`] whose `submit` always refuses with `ModelUnpriced` (Phase 42 D-10):
/// the scheduled assistant is agent-kind, its creator has a ceiling and its model lost its row.
struct UnpricedModelSubmission;

#[async_trait]
impl RunSubmissionPort for UnpricedModelSubmission {
    async fn submit(&self, _request: SubmitRun) -> Result<RunAccepted, RunSubmissionError> {
        Err(RunSubmissionError::ModelUnpriced {
            model: "mystery-model".to_string(),
        })
    }

    async fn cancel(
        &self,
        _run_id: &RunId,
        _requested_by: Option<PrincipalRef>,
    ) -> Result<CancelOutcome, RunSubmissionError> {
        Err(RunSubmissionError::NotWired)
    }

    async fn fork(&self, _request: ForkRun) -> Result<RunAccepted, RunSubmissionError> {
        Err(RunSubmissionError::NotWired)
    }
}

/// 42-09: a schedule-fired agent-kind run refused as `ModelUnpriced` is a submission error --
/// logged and left claimed like the schedule's other non-allowance failures -- and is never
/// reported or counted as an allowance skip.
#[tokio::test]
async fn tick_with_an_unpriced_agent_model_is_a_submission_error_not_an_allowance_skip() {
    let repo: Arc<dyn RunScheduleRepositoryPort> = Arc::new(InMemoryRunScheduleRepository::new());
    let t0 = base_time();
    let schedule =
        RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "*/1 * * * *").with_next_tick(t0);
    let schedule_id = schedule.schedule_id.clone();
    repo.insert(schedule).await.unwrap();

    let clock = AtomicClock::new(t0);
    let service = ScheduleService::new(
        Arc::clone(&repo),
        Arc::new(UnpricedModelSubmission) as Arc<dyn RunSubmissionPort>,
        options_with(&clock, 60),
    );
    let outcomes = service.tick_once().await;

    assert_eq!(
        outcomes,
        vec![ScheduleTickOutcome::Skipped {
            schedule_id: schedule_id.clone(),
            reason: SkipReason::SubmissionError,
        }],
        "never SkipReason::AllowanceExhausted"
    );
    let reloaded = repo.get(&schedule_id).await.unwrap().unwrap();
    assert_eq!(
        reloaded.skipped_ticks, 0,
        "counted like every other non-allowance submission failure, not as an allowance skip"
    );
    assert!(
        reloaded.next_tick.is_some_and(|next| next > t0),
        "the tick stays claimed: the schedule is next tried at its next natural due time"
    );
}
