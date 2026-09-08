//! `ScheduleService` — the facade tick loop driving `RunSchedule`s (PLAT-05,
//! D-36..D-39).
//!
//! Lives beside `run::worker`/`run::submission`: this is the ONE service
//! that reads `RunScheduleRepositoryPort`, parses the schedule's cron
//! (`paladin_storage::cron::parse_run_cron`), claims a tick (D-37), and
//! submits through the SAME `RunSubmissionPort` `RunSubmissionService`
//! implements -- a fired schedule tick is indistinguishable, downstream,
//! from an HTTP-submitted run.

/// `impl ScheduleAdminPort for ScheduleService` (PLAT-05, D-42, D-46) --
/// validate-then-persist create/get/list/patch/delete, plus `SsrfGuard`,
/// this plan's own write-time SSRF check (D-42).
pub mod admin;
/// `ScheduleService`, `ScheduleServiceOptions`, `ScheduleTickOutcome`,
/// `SkipReason` -- the claim-then-submit tick loop itself.
pub mod service;

pub use admin::SsrfGuard;
pub use service::{ScheduleService, ScheduleServiceOptions, ScheduleTickOutcome, SkipReason};

/// `schedule_restart_exactly_once`, `two_services_one_tick_exactly_one_fire`,
/// `fixed_thread_busy_increments_skipped` and the rest of this plan's own
/// test suite. A `#[cfg(test)]` module rather than a `tests/` target so it
/// counts toward `cargo llvm-cov` (D-54), mirroring every sibling `run::*`
/// test module's convention.
#[cfg(test)]
mod tests;
