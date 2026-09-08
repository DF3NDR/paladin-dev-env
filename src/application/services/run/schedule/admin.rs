//! `ScheduleAdminPort` implementation for `ScheduleService` (PLAT-05, D-42, D-46).
//!
//! Validate-then-persist: [`ScheduleService::create`]/[`ScheduleService::patch`] validate
//! the cron (5/6 field via `paladin_storage::cron::parse_run_cron`), the IANA timezone
//! (same call), the assistant reference (`AssistantResolver::resolve`, if a resolver is
//! wired via [`ScheduleService::with_resolver`]) and the webhook URL (the shared write-time
//! SSRF guard, D-42) — nothing is ever persisted until every check passes.
//!
//! # The write-time SSRF guard (D-42, collapsed onto 27-13's shared guard in 27-15)
//!
//! 27-14 (this module's own plan) built an independent, standalone `SsrfGuard` copy here
//! because 27-13's real guard (`src/application/services/run/webhook/ssrf.rs`) was not in
//! that plan's parallel-wave worktree base. Both waves have since landed, so this module now
//! routes through [`super::super::webhook::SsrfGuard`] directly — ONE guard applied at both
//! write time (here, on schedule create/patch) and send time (27-13's webhook client, on
//! every delivery attempt), never two independently-maintained copies of the same
//! security-relevant classification table (D-42 wave_context, 27-15).
use async_trait::async_trait;

use paladin_core::platform::container::run_schedule::{
    RunSchedule, RunScheduleId, RunScheduleUpdate, ThreadStrategy,
};
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::input::assistant_admin_port::ValidationViolation;
use paladin_ports::input::schedule_admin_port::{
    CreateRunSchedule, ScheduleAdminError, ScheduleAdminPort,
};
use paladin_ports::output::run_schedule_repository_port::{
    RunSchedulePage, RunScheduleRepositoryError,
};
use paladin_storage::cron::{CronParseError, parse_run_cron};

use super::super::resolver::ResolveError;
use super::service::ScheduleService;

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Map a [`CronParseError`] (other than [`CronParseError::UnknownTimezone`], handled
/// separately at `/timezone`) onto a stable `/cron` violation code.
fn cron_violation_code(error: &CronParseError) -> &'static str {
    match error {
        CronParseError::FieldCount { .. } => "invalid_field_count",
        CronParseError::Invalid { .. } => "invalid_cron",
        CronParseError::NoNextOccurrence => "no_next_occurrence",
        CronParseError::UnknownTimezone { .. } => "unknown_timezone",
        _ => "invalid_cron",
    }
}

/// Re-validate a [`ThreadStrategy::FixedThread`]'s [`ThreadId`] (`/thread_strategy`).
///
/// [`ThreadId`] derives `Deserialize` `#[serde(transparent)]`, which wraps the raw JSON
/// string directly WITHOUT running [`ThreadId::new`]'s own non-empty/length/no-whitespace
/// checks — a wire-supplied `FixedThread` can therefore carry a `ThreadId` [`ThreadId::new`]
/// itself would have rejected. Re-running the same constructor here is what actually
/// enforces those invariants for every caller of this port (HTTP DTO deserialization
/// included), not just direct-Rust callers that went through `ThreadId::new` themselves.
fn validate_thread_strategy(strategy: &ThreadStrategy) -> Option<ValidationViolation> {
    let ThreadStrategy::FixedThread(thread_id) = strategy else {
        return None;
    };
    match ThreadId::new(thread_id.as_str()) {
        Ok(_) => None,
        Err(error) => Some(ValidationViolation::new(
            "/thread_strategy",
            "invalid_thread_id",
            error.to_string(),
        )),
    }
}

fn map_repo_error(error: RunScheduleRepositoryError) -> ScheduleAdminError {
    match error {
        RunScheduleRepositoryError::NotFound { schedule_id } => {
            ScheduleAdminError::NotFound { schedule_id }
        }
        other => ScheduleAdminError::Backend {
            source: Box::new(other),
        },
    }
}

#[async_trait]
impl ScheduleAdminPort for ScheduleService {
    async fn create(&self, create: CreateRunSchedule) -> Result<RunSchedule, ScheduleAdminError> {
        let mut violations = Vec::new();
        let timezone = create.timezone.clone().unwrap_or_else(default_timezone);

        let cron = match parse_run_cron(&create.cron, &timezone) {
            Ok(cron) => Some(cron),
            Err(CronParseError::UnknownTimezone { name }) => {
                violations.push(ValidationViolation::new(
                    "/timezone",
                    "unknown_timezone",
                    format!("unknown IANA timezone: {name:?}"),
                ));
                None
            }
            Err(error) => {
                violations.push(ValidationViolation::new(
                    "/cron",
                    cron_violation_code(&error),
                    error.to_string(),
                ));
                None
            }
        };

        if let Some(resolver) = self.resolver.as_ref()
            && let Err(error) = resolver.resolve(&create.assistant_id, create.version).await
        {
            match error {
                ResolveError::UnknownAssistant { .. } => violations.push(ValidationViolation::new(
                    "/assistant_id",
                    "unknown_assistant",
                    error.to_string(),
                )),
                ResolveError::UnknownVersion { .. } => violations.push(ValidationViolation::new(
                    "/version",
                    "unknown_version",
                    error.to_string(),
                )),
            }
        }

        if let Some(strategy) = &create.thread_strategy
            && let Some(violation) = validate_thread_strategy(strategy)
        {
            violations.push(violation);
        }

        if let Some(webhook) = &create.webhook
            && let Err(rejection) = self.ssrf_guard.check_url(&webhook.url).await
        {
            violations.push(ValidationViolation::new(
                "/webhook/url",
                "webhook_url_rejected",
                rejection.to_string(),
            ));
        }

        let Some(cron) = cron else {
            return Err(ScheduleAdminError::Invalid { violations });
        };
        if !violations.is_empty() {
            return Err(ScheduleAdminError::Invalid { violations });
        }

        let now = (self.options.now)();
        let next_tick = cron
            .next_after(now)
            .map_err(|error| ScheduleAdminError::Invalid {
                violations: vec![ValidationViolation::new(
                    "/cron",
                    "no_next_occurrence",
                    error.to_string(),
                )],
            })?;

        let schedule_id = RunScheduleId::new_v7();
        let mut schedule = RunSchedule::new(schedule_id, create.assistant_id, create.cron)
            .with_timezone(timezone)
            .with_input(create.input)
            .with_next_tick(next_tick);
        if let Some(version) = create.version {
            schedule = schedule.with_version(version);
        }
        if let Some(strategy) = create.thread_strategy {
            schedule = schedule.with_thread_strategy(strategy);
        }
        if let Some(on_missed) = create.on_missed {
            schedule = schedule.with_on_missed(on_missed);
        }
        if let Some(webhook) = create.webhook {
            schedule = schedule.with_webhook(webhook);
        }
        if !create.enabled {
            schedule = schedule.disabled();
        }

        self.repo
            .insert(schedule.clone())
            .await
            .map_err(map_repo_error)?;
        Ok(schedule)
    }

    async fn get(
        &self,
        schedule_id: &RunScheduleId,
    ) -> Result<Option<RunSchedule>, ScheduleAdminError> {
        self.repo.get(schedule_id).await.map_err(map_repo_error)
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<RunScheduleId>,
    ) -> Result<RunSchedulePage, ScheduleAdminError> {
        self.repo.list(limit, cursor).await.map_err(map_repo_error)
    }

    async fn patch(
        &self,
        schedule_id: &RunScheduleId,
        mut update: RunScheduleUpdate,
    ) -> Result<RunSchedule, ScheduleAdminError> {
        let existing = self
            .repo
            .get(schedule_id)
            .await
            .map_err(map_repo_error)?
            .ok_or_else(|| ScheduleAdminError::NotFound {
                schedule_id: schedule_id.clone(),
            })?;

        let mut violations = Vec::new();
        let cron_or_timezone_changed = update.cron.is_some() || update.timezone.is_some();
        let effective_cron = update.cron.clone().unwrap_or_else(|| existing.cron.clone());
        let effective_timezone = update
            .timezone
            .clone()
            .unwrap_or_else(|| existing.timezone.clone());

        let mut recomputed_cron = None;
        if cron_or_timezone_changed {
            match parse_run_cron(&effective_cron, &effective_timezone) {
                Ok(cron) => recomputed_cron = Some(cron),
                Err(CronParseError::UnknownTimezone { name }) => {
                    violations.push(ValidationViolation::new(
                        "/timezone",
                        "unknown_timezone",
                        format!("unknown IANA timezone: {name:?}"),
                    ));
                }
                Err(error) => {
                    violations.push(ValidationViolation::new(
                        "/cron",
                        cron_violation_code(&error),
                        error.to_string(),
                    ));
                }
            }
        }

        if let Some(strategy) = &update.thread_strategy
            && let Some(violation) = validate_thread_strategy(strategy)
        {
            violations.push(violation);
        }

        if let Some(webhook) = &update.webhook
            && let Err(rejection) = self.ssrf_guard.check_url(&webhook.url).await
        {
            violations.push(ValidationViolation::new(
                "/webhook/url",
                "webhook_url_rejected",
                rejection.to_string(),
            ));
        }

        if !violations.is_empty() {
            return Err(ScheduleAdminError::Invalid { violations });
        }

        // `recomputed_cron` is `Some` exactly when `cron_or_timezone_changed` is true AND
        // parsing succeeded (a parse failure would already have returned above via the
        // `violations` check) -- `enabled: false` with no cron/timezone change never
        // touches `next_tick` here, a disabled schedule is simply skipped by `due()` (D-39).
        if let Some(cron) = recomputed_cron {
            let now = (self.options.now)();
            match cron.next_after(now) {
                Ok(next) => update.next_tick = Some(next),
                Err(error) => {
                    return Err(ScheduleAdminError::Invalid {
                        violations: vec![ValidationViolation::new(
                            "/cron",
                            "no_next_occurrence",
                            error.to_string(),
                        )],
                    });
                }
            }
        }

        self.repo
            .update(schedule_id, update)
            .await
            .map_err(map_repo_error)?;
        self.repo
            .get(schedule_id)
            .await
            .map_err(map_repo_error)?
            .ok_or_else(|| ScheduleAdminError::NotFound {
                schedule_id: schedule_id.clone(),
            })
    }

    async fn delete(&self, schedule_id: &RunScheduleId) -> Result<(), ScheduleAdminError> {
        self.repo.delete(schedule_id).await.map_err(map_repo_error)
    }
}
