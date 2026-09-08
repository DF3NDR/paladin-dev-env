//! `ScheduleService` — claim-then-submit tick loop (PLAT-05, D-37, D-39).
//!
//! # Claim before submit (D-37)
//!
//! [`ScheduleService::tick_once`]'s ordering is the whole restart/replica
//! safety story: for each due schedule, [`RunScheduleRepositoryPort::claim_tick`]
//! is called FIRST; only a `true` result (this instance won the conditional
//! update) leads to [`RunSubmissionPort::submit`] being called at all. Two
//! `ScheduleService` instances racing the same tick therefore submit at most
//! one run between them, and a restarted instance re-reads `next_tick` from
//! the repository rather than any in-process state, so it neither
//! double-fires nor misses-then-double-fires.
//!
//! # The injected clock (D-37, test note)
//! `tokio::time::pause` freezes tokio TIMERS (the interval loop inside
//! [`ScheduleService::spawn`]) but NOT `chrono::Utc::now()` -- so every
//! timestamp this service persists or compares against comes from
//! `ScheduleServiceOptions::now`, an injected closure, never a direct
//! `Utc::now()` call. Production wiring passes `Arc::new(Utc::now)`; tests
//! pass a fabricated, controllable clock.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::run_schedule::{OnMissed, RunScheduleId, ThreadStrategy};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::run_submission_port::{RunSubmissionError, RunSubmissionPort, SubmitRun};
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_storage::cron::parse_run_cron;

use super::super::resolver::AssistantResolver;
use super::super::webhook::SsrfGuard;

/// How many due schedules [`ScheduleService::tick_once`] processes per call.
/// Generous enough that a normal deployment's schedule count fits in one
/// tick; a deployment with more due schedules than this simply catches the
/// rest on the NEXT tick (no schedule is ever lost, only delayed).
const DUE_BATCH_LIMIT: u32 = 100;

/// Generate a fresh [`ThreadId`] from a UUIDv7 string.
///
/// A UUIDv7 string is always non-empty and free of whitespace, so
/// `ThreadId::new` can never reject it here -- looped rather than
/// `.expect()`ed so this file stays free of `unwrap`/`expect` (CLAUDE.md)
/// while remaining provably total (the loop body always returns on its
/// first iteration). Mirrors `run::submission::generate_thread_id` exactly
/// (that file is sibling-owned in this wave, so this is a small, deliberate
/// duplication rather than a cross-worktree edit).
fn generate_thread_id() -> ThreadId {
    loop {
        if let Ok(id) = ThreadId::new(uuid::Uuid::now_v7().to_string()) {
            return id;
        }
    }
}

/// Why a due schedule's tick was skipped rather than fired.
///
/// `#[non_exhaustive]`: a future policy (e.g. a rate limit) may add a
/// variant without breaking an exhaustive `match` downstream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipReason {
    /// The tick was discovered more than roughly one tick interval after it
    /// was due, and `on_missed == Skip` -- nothing was submitted for the
    /// missed window (D-39).
    Missed,
    /// `ThreadStrategy::FixedThread` landed on a thread that already has an
    /// active run; `skipped_ticks` was incremented on the row (D-39).
    ThreadBusy,
    /// The claimed tick's submission failed for a reason OTHER than
    /// `ThreadBusy` (e.g. the assistant was deleted between claim and
    /// submit). The tick is still recorded as claimed -- `next_tick` has
    /// already advanced -- so this schedule is not retried until its next
    /// natural due time.
    SubmissionError,
}

/// The outcome of processing one due schedule during a
/// [`ScheduleService::tick_once`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleTickOutcome {
    /// This instance claimed the tick and successfully submitted a run.
    Fired {
        /// The schedule that fired.
        schedule_id: RunScheduleId,
        /// The submitted run's identity.
        run_id: RunId,
    },
    /// This instance claimed the tick but did not submit a run (see
    /// [`SkipReason`]).
    Skipped {
        /// The schedule that skipped.
        schedule_id: RunScheduleId,
        /// Why.
        reason: SkipReason,
    },
    /// This instance's `claim_tick` call returned `false`: another replica
    /// (or another call on this same instance) already claimed this tick
    /// first. Not an error -- the D-37 race has exactly one winner and this
    /// was not it.
    LostRace {
        /// The schedule whose tick was already claimed elsewhere.
        schedule_id: RunScheduleId,
    },
}

/// Construction knobs for [`ScheduleService`].
#[derive(Clone)]
pub struct ScheduleServiceOptions {
    /// How often [`ScheduleService::spawn`]'s background loop calls
    /// [`ScheduleService::tick_once`]. Also the unit `tick_once` uses to
    /// decide whether a due tick counts as "missed" (more than `2 *
    /// tick_interval` late).
    pub tick_interval: Duration,
    /// The clock every persisted/compared timestamp in this service comes
    /// from -- never a direct `Utc::now()` call (see this module's own
    /// docs). Production wiring: `Arc::new(chrono::Utc::now)`.
    pub now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}

impl Default for ScheduleServiceOptions {
    /// A 60-second tick interval and the real system clock.
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(60),
            now: Arc::new(Utc::now),
        }
    }
}

/// Drives due [`RunSchedule`]s through a claim-then-submit tick loop
/// (PLAT-05, D-37, D-39), and -- via `admin.rs`'s `ScheduleAdminPort` impl --
/// validates and persists them in the first place (D-42, D-46). Fields are
/// `pub(super)` rather than private so `schedule::admin`'s
/// `impl ScheduleAdminPort for ScheduleService` (a sibling module, not this
/// one) can read them directly.
pub struct ScheduleService {
    pub(super) repo: Arc<dyn RunScheduleRepositoryPort>,
    pub(super) submission: Arc<dyn RunSubmissionPort>,
    pub(super) options: ScheduleServiceOptions,
    /// Resolves an `(assistant_id, version)` reference at schedule
    /// create-time (`ScheduleAdminPort::create`'s `unknown_assistant`/
    /// `unknown_version` violations) -- `None` skips that one check (e.g. a
    /// bare `ScheduleService` under test that only exercises the tick
    /// loop).
    pub(super) resolver: Option<Arc<dyn AssistantResolver>>,
    /// The write-time SSRF guard (D-42) `ScheduleAdminPort::create`/`patch`
    /// run a schedule's `webhook.url` through before persisting.
    pub(super) ssrf_guard: SsrfGuard,
}

impl ScheduleService {
    /// Construct a `ScheduleService` over the given repository and
    /// submission port. No assistant resolver is wired and the SSRF guard
    /// defaults to `allow_private: false` -- see [`Self::with_resolver`]
    /// and [`Self::with_ssrf_guard`].
    pub fn new(
        repo: Arc<dyn RunScheduleRepositoryPort>,
        submission: Arc<dyn RunSubmissionPort>,
        options: ScheduleServiceOptions,
    ) -> Self {
        Self {
            repo,
            submission,
            options,
            resolver: None,
            ssrf_guard: SsrfGuard::new(false),
        }
    }

    /// Wire an [`AssistantResolver`] so [`ScheduleAdminPort::create`](
    /// paladin_ports::input::schedule_admin_port::ScheduleAdminPort::create)
    /// validates the `(assistant_id, version)` reference before persisting.
    pub fn with_resolver(mut self, resolver: Arc<dyn AssistantResolver>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Override the default (`allow_private: false`) write-time SSRF guard
    /// (D-42).
    pub fn with_ssrf_guard(mut self, ssrf_guard: SsrfGuard) -> Self {
        self.ssrf_guard = ssrf_guard;
        self
    }

    /// Process every currently-due schedule once: for each, compute the next
    /// occurrence, CLAIM the current tick (D-37 -- this is the whole
    /// restart/replica-safety mechanism), and only on a WINNING claim
    /// (non-missed-and-`Skip` path) call [`RunSubmissionPort::submit`].
    /// Returns one [`ScheduleTickOutcome`] per due schedule that was
    /// successfully read and processed; a repository failure reading the
    /// due set is logged and yields an empty result (a transient backend
    /// error is not this method's caller's problem to handle per-schedule).
    pub async fn tick_once(&self) -> Vec<ScheduleTickOutcome> {
        let now = (self.options.now)();

        let due = match self.repo.due(now, DUE_BATCH_LIMIT).await {
            Ok(due) => due,
            Err(error) => {
                log::warn!("schedule service: due() failed: {error}");
                return Vec::new();
            }
        };

        let mut outcomes = Vec::with_capacity(due.len());
        for schedule in due {
            let schedule_id = schedule.schedule_id.clone();

            // `due()` only returns rows with `next_tick <= now`, so
            // `next_tick` is always `Some` here -- defensive rather than
            // assumed, since a future adapter bug should degrade to "skip
            // this row, log it" rather than panic.
            let Some(next_tick) = schedule.next_tick else {
                log::warn!("schedule {schedule_id}: due() returned a row with no next_tick");
                continue;
            };

            let cron = match parse_run_cron(&schedule.cron, &schedule.timezone) {
                Ok(cron) => cron,
                Err(error) => {
                    log::warn!("schedule {schedule_id}: cron parse failed: {error}");
                    continue;
                }
            };
            let after = match cron.next_after(now) {
                Ok(after) => after,
                Err(error) => {
                    log::warn!(
                        "schedule {schedule_id}: could not compute next occurrence: {error}"
                    );
                    continue;
                }
            };

            let interval = chrono::Duration::from_std(self.options.tick_interval)
                .unwrap_or_else(|_| chrono::Duration::zero());
            let missed = (now - next_tick) > interval * 2;

            // D-37: CLAIM before anything else observes this tick as ours.
            let claimed = self
                .repo
                .claim_tick(&schedule_id, next_tick, now, after)
                .await;

            if missed && schedule.on_missed == OnMissed::Skip {
                // A late tick under the Skip policy claims (to advance
                // next_tick past the missed window) but never submits.
                let outcome = match claimed {
                    Ok(true) => ScheduleTickOutcome::Skipped {
                        schedule_id,
                        reason: SkipReason::Missed,
                    },
                    Ok(false) => ScheduleTickOutcome::LostRace { schedule_id },
                    Err(error) => {
                        log::warn!("schedule {schedule_id}: claim_tick failed: {error}");
                        continue;
                    }
                };
                outcomes.push(outcome);
                continue;
            }

            let outcome = match claimed {
                Ok(true) => {
                    // This instance won the claim -- resolve the target
                    // thread per D-39's strategy and submit ONE run.
                    let thread_id = match &schedule.thread_strategy {
                        ThreadStrategy::NewThreadPerTick => generate_thread_id(),
                        ThreadStrategy::FixedThread(thread_id) => thread_id.clone(),
                    };
                    let request = SubmitRun {
                        assistant_id: schedule.assistant_id.clone(),
                        version: schedule.version,
                        thread_id: Some(thread_id),
                        input: schedule.input.clone(),
                        webhook: schedule.webhook.clone(),
                        requested_by: None,
                    };

                    match self.submission.submit(request).await {
                        Ok(accepted) => ScheduleTickOutcome::Fired {
                            schedule_id,
                            run_id: accepted.run_id,
                        },
                        Err(RunSubmissionError::ThreadBusy { .. }) => {
                            if let Err(error) = self.repo.increment_skipped(&schedule_id).await {
                                log::warn!(
                                    "schedule {schedule_id}: increment_skipped failed: {error}"
                                );
                            }
                            ScheduleTickOutcome::Skipped {
                                schedule_id,
                                reason: SkipReason::ThreadBusy,
                            }
                        }
                        Err(error) => {
                            log::warn!("schedule {schedule_id}: run submission failed: {error}");
                            ScheduleTickOutcome::Skipped {
                                schedule_id,
                                reason: SkipReason::SubmissionError,
                            }
                        }
                    }
                }
                Ok(false) => ScheduleTickOutcome::LostRace { schedule_id },
                Err(error) => {
                    log::warn!("schedule {schedule_id}: claim_tick failed: {error}");
                    continue;
                }
            };
            outcomes.push(outcome);
        }
        outcomes
    }

    /// Spawn a background task calling [`ScheduleService::tick_once`] every
    /// `options.tick_interval`, registered with `coordinator` (D-13's
    /// draining precedent): on shutdown the task stops ticking once its
    /// current `tick_once` call returns and drops its `RunGuard`.
    pub fn spawn(self: Arc<Self>, coordinator: &ShutdownCoordinator) -> JoinHandle<()> {
        let (child_token, guard) = coordinator.register();
        let interval_duration = self.options.tick_interval;

        tokio::spawn(async move {
            let _guard = guard;
            let mut ticker = tokio::time::interval(interval_duration);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        self.tick_once().await;
                    }
                    _ = child_token.cancelled() => break,
                }
            }
        })
    }
}
