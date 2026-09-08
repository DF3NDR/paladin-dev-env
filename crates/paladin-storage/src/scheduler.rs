//! `TokioCronSchedulerAdapter` — a concrete [`SchedulerPort`] implementation
//! backed by [`tokio_cron_scheduler`].
//!
//! Until this adapter, the only `SchedulerPort` implementation shipped in the
//! framework was `MockScheduler` in `doc-examples` — a test double, not a
//! runnable adapter — even though `paladin-web`'s content deliverer already
//! consumes `Arc<dyn SchedulerPort>` in earnest and the workspace already
//! declares a `tokio-cron-scheduler` dependency. This module fills that gap
//! so an application can schedule real cron jobs against the port without
//! writing its own engine adapter.
//!
//! # Why the adapter tracks its own job state
//!
//! `tokio-cron-scheduler` 0.13's `JobScheduler` offers no status query, no
//! job-info query, and no job listing — its public surface is essentially
//! `add`/`remove`/`start`/`shutdown`/`time_till_next_job`/`next_tick_for_job`/
//! `context`. Every field the [`JobInfo`] contract promises — `status`,
//! `created_at`, `last_run`, `next_run`, `run_count`, `failure_count` — must
//! therefore be tracked by this adapter itself, in [`JobRegistration`]. Only
//! `next_run` can be answered by the engine (via `next_tick_for_job`).
//!
//! # The identity rule
//!
//! The engine assigns a **fresh** `Uuid` on every `add()` call — there is no
//! way to ask it to reuse a previously known job id, so a `JobId` handed out
//! by this adapter is process-lifetime-only. Callers that need a stable
//! cross-restart identity for a scheduled job must key it on their own domain
//! identifier (carried in [`JobSpec::metadata`]) and re-register on startup;
//! they must not persist this adapter's `JobId`.
//!
//! # The result-less tick closure
//!
//! `tokio_cron_scheduler::Job::new_async`'s closure returns
//! `Pin<Box<dyn Future<Output = ()>>>`, not a `Result` — a tick's error or
//! panic is invisible to the engine, and there is no failure counter inside
//! it to propagate into. The only way the counters that
//! [`SchedulerPort::get_job_status`]/[`SchedulerPort::get_job_info`] report
//! can stay correct is for the tick closure to call
//! [`TokioCronSchedulerAdapter::record_job_run`] itself, on every path, before
//! returning. That method is this adapter's own bookkeeping seam — deliberately
//! not on the `SchedulerPort` trait, because it is recovery from a gap in the
//! engine, not a capability the port promises.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex as AsyncMutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use paladin_ports::output::scheduler_port::{
    JobId, JobInfo, JobSpec, JobStatus, SchedulerError, SchedulerPort,
};

/// The number of whitespace-separated fields a `tokio-cron-scheduler` cron
/// expression must have: `sec min hour day month weekday`. Re-exported (as
/// [`crate::cron::CRON_FIELD_COUNT`]) so this module and `cron.rs` share one
/// named constant rather than two independently-defined literals.
const CRON_FIELD_COUNT: usize = crate::cron::CRON_FIELD_COUNT;

/// Pre-validates that a cron string has the six fields
/// (`sec min hour day month weekday`) `tokio-cron-scheduler` requires, mapping
/// the common five-field (standard crontab) mistake to a
/// [`SchedulerError::InvalidCronExpression`] whose `reason` names the expected
/// form.
///
/// This runs **before** the engine's own parser is ever reached. The engine's
/// parse error is a unit variant carrying no field-level detail, so a caller
/// who passes a five-field crontab pattern would otherwise get an
/// unactionable message; naming the expected six-field form here is the whole
/// value. Deeper semantic validity (a field out of range, an unparseable
/// token) is still caught when the [`Job`] is constructed in
/// [`TokioCronSchedulerAdapter::schedule_job_with`] — that error also maps to
/// `InvalidCronExpression`, though with the engine's own (less specific)
/// message.
///
/// The field-COUNTING primitive is shared with `crate::cron::cron_field_count`
/// (D-38) — this adapter's own six-field ACCEPTANCE requirement is unchanged
/// (X-03): `cron.rs`'s run-schedule parser accepts 5 OR 6 fields, but this
/// function still requires exactly 6, so every existing caller of this
/// adapter keeps its current behavior verbatim.
///
/// Extracted as a free function so it is unit-testable without constructing
/// the adapter or the engine.
fn validate_cron_field_count(schedule: &str) -> Result<(), SchedulerError> {
    let field_count = crate::cron::cron_field_count(schedule);
    if field_count == CRON_FIELD_COUNT {
        Ok(())
    } else {
        Err(SchedulerError::InvalidCronExpression {
            expression: schedule.to_string(),
            reason: format!(
                "expected a 6-field cron expression (sec min hour day month weekday), \
                 found {field_count} field(s)"
            ),
        })
    }
}

/// This adapter's own bookkeeping for one registered job — everything the
/// [`JobInfo`] shape promises that the engine itself cannot answer (see module
/// docs). Constructed fresh on every schedule call and mutated only by
/// [`TokioCronSchedulerAdapter::record_job_run`] and
/// [`TokioCronSchedulerAdapter::cancel_job`].
#[derive(Debug, Clone)]
struct JobRegistration {
    spec: JobSpec,
    created_at: DateTime<Utc>,
    status: JobStatus,
    last_run: Option<DateTime<Utc>>,
    run_count: u32,
    failure_count: u32,
}

impl JobRegistration {
    /// A freshly scheduled job: `Scheduled`, zeroed counters, no run yet.
    fn new(spec: JobSpec, created_at: DateTime<Utc>) -> Self {
        Self {
            spec,
            created_at,
            status: JobStatus::Scheduled,
            last_run: None,
            run_count: 0,
            failure_count: 0,
        }
    }

    /// The pure transition [`TokioCronSchedulerAdapter::record_job_run`]
    /// applies. Deliberately a plain, engine-free method so it is
    /// unit-testable without constructing the real scheduling engine.
    ///
    /// Success: `run_count` increments, `last_run` is set, `failure_count`
    /// resets to 0, `status` becomes `Completed`. Failure: `run_count` and
    /// `failure_count` both increment, `status` becomes `Failed`.
    fn record_run(&mut self, succeeded: bool, at: DateTime<Utc>) {
        self.run_count = self.run_count.saturating_add(1);
        self.last_run = Some(at);
        if succeeded {
            self.failure_count = 0;
            self.status = JobStatus::Completed;
        } else {
            self.failure_count = self.failure_count.saturating_add(1);
            self.status = JobStatus::Failed("scheduled job's tick reported failure".to_string());
        }
    }
}

/// A concrete [`SchedulerPort`] adapter over `tokio-cron-scheduler` 0.13's
/// [`JobScheduler`].
///
/// The engine is held behind a [`tokio::sync::Mutex`] so its `&mut self`
/// methods (`shutdown`, `time_till_next_job`, `next_tick_for_job`) remain
/// reachable from behind an `Arc<dyn SchedulerPort>`. The registration map is
/// a second, independent mutex (never the same lock as the engine, so a
/// registration read never blocks on an in-flight engine call and vice versa).
/// `running` is a plain [`AtomicBool`] because [`SchedulerPort::is_running`]
/// is a synchronous trait method that cannot await either lock.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use paladin_storage::scheduler::TokioCronSchedulerAdapter;
/// use paladin_ports::output::scheduler_port::{JobSpec, SchedulerPort};
///
/// let adapter = TokioCronSchedulerAdapter::try_new().await?;
///
/// // Attach real work with `schedule_job_with`; the tick closure MUST call
/// // `adapter.record_job_run(&job_id, succeeded, when)` before it returns so
/// // the job's status/counters stay accurate.
/// let job_id = adapter
///     .schedule_job_with(JobSpec::new("nightly", "0 0 0 * * *"), |_uuid, _sched| {
///         Box::pin(async { /* do work */ })
///     })
///     .await?;
///
/// adapter.start().await?;
/// # let _ = job_id;
/// # Ok(())
/// # }
/// ```
pub struct TokioCronSchedulerAdapter {
    engine: AsyncMutex<JobScheduler>,
    registrations: AsyncMutex<HashMap<JobId, JobRegistration>>,
    running: AtomicBool,
}

impl TokioCronSchedulerAdapter {
    /// Constructs the adapter, initialising the real `tokio-cron-scheduler`
    /// engine. The engine's own construction error is mapped through
    /// [`SchedulerError::Internal`].
    pub async fn try_new() -> Result<Self, SchedulerError> {
        let engine = JobScheduler::new()
            .await
            .map_err(|e| SchedulerError::Internal(e.to_string()))?;
        Ok(Self {
            engine: AsyncMutex::new(engine),
            registrations: AsyncMutex::new(HashMap::new()),
            running: AtomicBool::new(false),
        })
    }

    /// Registers a job with a real tick closure attached.
    ///
    /// [`SchedulerPort::schedule_job`] cannot carry a closure (its signature
    /// takes only a [`JobSpec`]), so this inherent method is the real
    /// registration path callers use to attach work; the trait's own
    /// `schedule_job` is implemented in terms of it with a no-op closure that
    /// logs a warning.
    pub async fn schedule_job_with<F>(&self, spec: JobSpec, run: F) -> Result<JobId, SchedulerError>
    where
        F: FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync
            + 'static,
    {
        // Field-count pre-check before the engine is ever touched. Deeper
        // parse errors are caught by `Job::new_async` below.
        validate_cron_field_count(&spec.schedule)?;

        let job = Job::new_async(spec.schedule.as_str(), run).map_err(|e| {
            SchedulerError::InvalidCronExpression {
                expression: spec.schedule.clone(),
                reason: e.to_string(),
            }
        })?;

        let uuid = {
            let engine = self.engine.lock().await;
            engine
                .add(job)
                .await
                .map_err(|e| SchedulerError::Internal(e.to_string()))?
        };
        let job_id = JobId::from_uuid(uuid);

        let registration = JobRegistration::new(spec, Utc::now());
        self.registrations
            .lock()
            .await
            .insert(job_id.clone(), registration);

        Ok(job_id)
    }

    /// This adapter's own bookkeeping seam — not on [`SchedulerPort`], because
    /// it is recovery from a gap in the engine, not a capability the port
    /// promises.
    ///
    /// Because the tick closure returns `()` rather than a `Result`, a tick's
    /// error is invisible to the engine and there is no failure counter inside
    /// it to propagate into. The counters
    /// [`SchedulerPort::get_job_status`]/[`SchedulerPort::get_job_info`] report
    /// can only stay correct if the tick closure calls this method itself, on
    /// every path, before returning.
    ///
    /// A call against an unknown `job_id` (e.g. a job cancelled between
    /// tick-fire and tick-completion) is a silent no-op rather than an error —
    /// there is no registration left to update, and the caller (a tick closure
    /// with no `Result` return path) has nowhere to send an error anyway.
    pub async fn record_job_run(&self, job_id: &JobId, succeeded: bool, at: DateTime<Utc>) {
        let mut registrations = self.registrations.lock().await;
        if let Some(registration) = registrations.get_mut(job_id) {
            registration.record_run(succeeded, at);
        } else {
            eprintln!(
                "paladin-storage scheduler: record_job_run called for job id {job_id} with no \
                 tracked registration -- likely cancelled between tick-fire and tick-completion"
            );
        }
    }
}

#[async_trait]
impl SchedulerPort for TokioCronSchedulerAdapter {
    async fn start(&self) -> Result<(), SchedulerError> {
        let engine = self.engine.lock().await;
        engine
            .start()
            .await
            .map_err(|e| SchedulerError::Internal(e.to_string()))?;
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), SchedulerError> {
        // shutdown() takes &mut self on the engine -- reachable only because
        // the Mutex hands out an exclusive guard.
        let mut engine = self.engine.lock().await;
        engine
            .shutdown()
            .await
            .map_err(|e| SchedulerError::Internal(e.to_string()))?;
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn schedule_job(&self, spec: JobSpec) -> Result<JobId, SchedulerError> {
        // A job registered through the bare trait method has nothing to run --
        // the trait signature carries no closure parameter. Saying so loudly
        // at every fire beats silently registering a job that does nothing.
        let label = spec.label.clone();
        self.schedule_job_with(spec, move |_uuid, _scheduler| {
            let label = label.clone();
            Box::pin(async move {
                eprintln!(
                    "paladin-storage scheduler: job '{label}' was registered through the bare \
                     SchedulerPort::schedule_job trait method with no tick handler attached and \
                     will do nothing when it fires -- use \
                     TokioCronSchedulerAdapter::schedule_job_with to attach real work"
                );
            })
        })
        .await
    }

    async fn cancel_job(&self, job_id: &JobId) -> Result<(), SchedulerError> {
        let mut registrations = self.registrations.lock().await;
        if !registrations.contains_key(job_id) {
            return Err(SchedulerError::JobNotFound(job_id.clone()));
        }

        let engine = self.engine.lock().await;
        engine
            .remove(job_id.as_uuid())
            .await
            .map_err(|e| SchedulerError::Internal(e.to_string()))?;

        registrations.remove(job_id);
        Ok(())
    }

    async fn get_job_status(&self, job_id: &JobId) -> Result<JobStatus, SchedulerError> {
        let registrations = self.registrations.lock().await;
        registrations
            .get(job_id)
            .map(|r| r.status.clone())
            .ok_or_else(|| SchedulerError::JobNotFound(job_id.clone()))
    }

    async fn get_job_info(&self, job_id: &JobId) -> Result<JobInfo, SchedulerError> {
        let registration = {
            let registrations = self.registrations.lock().await;
            registrations
                .get(job_id)
                .cloned()
                .ok_or_else(|| SchedulerError::JobNotFound(job_id.clone()))?
        };

        // next_run is the one JobInfo field the engine itself can answer. A
        // lookup failure (e.g. the engine has no record yet for a job
        // registered immediately before its first tick) degrades to None
        // rather than failing the whole call -- every other field is still
        // accurate.
        let next_run = {
            let mut engine = self.engine.lock().await;
            engine
                .next_tick_for_job(*job_id.as_uuid())
                .await
                .unwrap_or(None)
        };

        Ok(JobInfo {
            id: job_id.clone(),
            spec: registration.spec,
            status: registration.status,
            created_at: registration.created_at,
            last_run: registration.last_run,
            next_run,
            run_count: registration.run_count,
            failure_count: registration.failure_count,
        })
    }

    async fn list_jobs(&self) -> Result<Vec<JobInfo>, SchedulerError> {
        // Served entirely from the registration map (the engine has no
        // list-jobs equivalent). next_run stays None here rather than paying
        // for one engine call per job; get_job_info is the place to ask the
        // engine for a single job's next tick.
        let registrations = self.registrations.lock().await;
        let mut entries: Vec<(JobId, JobRegistration)> = registrations
            .iter()
            .map(|(id, r)| (id.clone(), r.clone()))
            .collect();
        entries.sort_by(|a, b| a.1.spec.label.cmp(&b.1.spec.label));

        Ok(entries
            .into_iter()
            .map(|(id, r)| JobInfo {
                id,
                spec: r.spec,
                status: r.status,
                created_at: r.created_at,
                last_run: r.last_run,
                next_run: None,
                run_count: r.run_count,
                failure_count: r.failure_count,
            })
            .collect())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Pure, engine-free tests -- the default-suite coverage. None of these
    // construct TokioCronSchedulerAdapter or JobScheduler.
    // ------------------------------------------------------------------

    #[test]
    fn validate_cron_field_count_accepts_valid_six_field_input() {
        assert!(validate_cron_field_count("0 0 9 * * *").is_ok());
    }

    #[test]
    fn validate_cron_field_count_rejects_five_field_input_naming_the_expected_form() {
        let err = validate_cron_field_count("0 9 * * *").unwrap_err();
        match err {
            SchedulerError::InvalidCronExpression { expression, reason } => {
                assert_eq!(expression, "0 9 * * *");
                assert!(
                    reason.contains("sec min hour day month weekday"),
                    "reason should name the expected 6-field form, got: {reason}"
                );
                assert!(
                    reason.contains("5 field"),
                    "reason should report the actual field count, got: {reason}"
                );
            }
            other => panic!("expected InvalidCronExpression, got {other:?}"),
        }
    }

    #[test]
    fn validate_cron_field_count_ignores_irregular_whitespace() {
        // Tabs and repeated spaces still yield six fields.
        assert!(validate_cron_field_count("0\t0   9 * * *").is_ok());
    }

    #[test]
    fn new_registration_starts_scheduled_with_zeroed_counters_and_no_last_run() {
        let spec = JobSpec::new("acme-corp", "0 0 9 * * *");
        let created_at = Utc::now();
        let registration = JobRegistration::new(spec.clone(), created_at);

        assert_eq!(registration.status, JobStatus::Scheduled);
        assert_eq!(registration.run_count, 0);
        assert_eq!(registration.failure_count, 0);
        assert_eq!(registration.last_run, None);
        assert_eq!(registration.created_at, created_at);
        assert_eq!(registration.spec.label, spec.label);
    }

    #[test]
    fn record_run_success_increments_run_count_sets_last_run_resets_failures_and_completes() {
        let mut registration =
            JobRegistration::new(JobSpec::new("acme-corp", "0 0 9 * * *"), Utc::now());
        registration.failure_count = 3; // simulate a prior failure streak
        let at = Utc::now();

        registration.record_run(true, at);

        assert_eq!(registration.run_count, 1);
        assert_eq!(registration.last_run, Some(at));
        assert_eq!(registration.failure_count, 0);
        assert_eq!(registration.status, JobStatus::Completed);
    }

    #[test]
    fn record_run_failure_increments_both_counters_and_sets_failed() {
        let mut registration =
            JobRegistration::new(JobSpec::new("acme-corp", "0 0 9 * * *"), Utc::now());
        let at = Utc::now();

        registration.record_run(false, at);

        assert_eq!(registration.run_count, 1);
        assert_eq!(registration.failure_count, 1);
        assert_eq!(registration.last_run, Some(at));
        assert!(matches!(registration.status, JobStatus::Failed(_)));
    }

    #[test]
    fn record_run_accumulates_across_repeated_failures() {
        let mut registration =
            JobRegistration::new(JobSpec::new("acme-corp", "0 0 9 * * *"), Utc::now());

        registration.record_run(false, Utc::now());
        registration.record_run(false, Utc::now());
        registration.record_run(false, Utc::now());

        assert_eq!(registration.run_count, 3);
        assert_eq!(registration.failure_count, 3);
    }

    // ------------------------------------------------------------------
    // Engine-constructing tests -- opt-in only. Constructing
    // TokioCronSchedulerAdapter constructs the real tokio-cron-scheduler
    // engine, which needs a multi_thread runtime (the engine's own README
    // states it hangs on `scheduler.add()` under a single-threaded runtime).
    // Every test in this block is therefore #[ignore]d and never runs in the
    // default `cargo test` suite; run them with `--ignored`.
    // ------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn schedule_job_with_valid_cron_returns_job_id_and_scheduled_registration() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let spec = JobSpec::new("acme-corp", "0 0 9 * * *");
        let job_id = adapter
            .schedule_job_with(spec, |_uuid, _scheduler| Box::pin(async {}))
            .await
            .expect("valid cron should schedule");

        assert_eq!(
            adapter.get_job_status(&job_id).await.unwrap(),
            JobStatus::Scheduled
        );
        let info = adapter.get_job_info(&job_id).await.unwrap();
        assert_eq!(info.run_count, 0);
        assert_eq!(info.failure_count, 0);
        assert_eq!(info.last_run, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn schedule_job_with_five_field_cron_is_rejected_before_the_engine() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let err = adapter
            .schedule_job_with(
                JobSpec::new("acme-corp", "0 9 * * *"),
                |_uuid, _scheduler| Box::pin(async {}),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, SchedulerError::InvalidCronExpression { .. }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn get_job_status_for_unknown_id_returns_job_not_found() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let err = adapter.get_job_status(&JobId::new()).await.unwrap_err();
        assert!(matches!(err, SchedulerError::JobNotFound(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn list_jobs_returns_one_entry_per_job_ordered_by_label() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        adapter
            .schedule_job_with(JobSpec::new("zeta", "0 0 9 * * *"), |_uuid, _scheduler| {
                Box::pin(async {})
            })
            .await
            .expect("schedule should succeed");
        adapter
            .schedule_job_with(
                JobSpec::new("alpha", "0 0 10 * * *"),
                |_uuid, _scheduler| Box::pin(async {}),
            )
            .await
            .expect("schedule should succeed");

        let jobs = adapter.list_jobs().await.expect("list should succeed");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].spec.label, "alpha");
        assert_eq!(jobs[1].spec.label, "zeta");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn cancel_job_removes_registration_and_a_subsequent_lookup_is_not_found() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let job_id = adapter
            .schedule_job_with(
                JobSpec::new("acme-corp", "0 0 9 * * *"),
                |_uuid, _scheduler| Box::pin(async {}),
            )
            .await
            .expect("schedule should succeed");

        adapter
            .cancel_job(&job_id)
            .await
            .expect("cancel should succeed");

        let err = adapter.get_job_status(&job_id).await.unwrap_err();
        assert!(matches!(err, SchedulerError::JobNotFound(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn cancel_job_for_unknown_id_returns_job_not_found() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let err = adapter.cancel_job(&JobId::new()).await.unwrap_err();
        assert!(matches!(err, SchedulerError::JobNotFound(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn record_job_run_success_and_failure_are_reflected_through_get_job_status() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let job_id = adapter
            .schedule_job_with(
                JobSpec::new("acme-corp", "0 0 9 * * *"),
                |_uuid, _scheduler| Box::pin(async {}),
            )
            .await
            .expect("schedule should succeed");

        adapter.record_job_run(&job_id, true, Utc::now()).await;
        assert_eq!(
            adapter.get_job_status(&job_id).await.unwrap(),
            JobStatus::Completed
        );

        adapter.record_job_run(&job_id, false, Utc::now()).await;
        assert!(matches!(
            adapter.get_job_status(&job_id).await.unwrap(),
            JobStatus::Failed(_)
        ));

        let info = adapter.get_job_info(&job_id).await.unwrap();
        assert_eq!(info.run_count, 2);
        assert_eq!(info.failure_count, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn is_running_is_false_before_start_and_true_after() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        assert!(!adapter.is_running());
        adapter.start().await.expect("start should succeed");
        assert!(adapter.is_running());

        adapter.shutdown().await.expect("shutdown should succeed");
        assert!(!adapter.is_running());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "constructs the real tokio-cron-scheduler engine (needs multi_thread) -- \
                run explicitly with `cargo test -p paladin-storage --features scheduler -- --ignored`"]
    async fn two_schedule_calls_for_the_same_label_produce_distinct_job_ids() {
        let adapter = TokioCronSchedulerAdapter::try_new()
            .await
            .expect("adapter should construct");

        let spec = JobSpec::new("acme-corp", "0 0 9 * * *");
        let first = adapter
            .schedule_job(spec.clone())
            .await
            .expect("first should succeed");
        let second = adapter
            .schedule_job(spec)
            .await
            .expect("second should succeed");

        assert_ne!(first, second);
    }
}
