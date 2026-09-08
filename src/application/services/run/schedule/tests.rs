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

use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::run_schedule::{
    OnMissed, RunSchedule, RunScheduleId, ThreadStrategy,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{
    CancelOutcome, RunAccepted, RunSubmissionError, RunSubmissionPort, SubmitRun,
};
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_storage::run_schedule::in_memory::InMemoryRunScheduleRepository;

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

    async fn cancel(&self, _run_id: &RunId) -> Result<CancelOutcome, RunSubmissionError> {
        Err(RunSubmissionError::NotWired)
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
