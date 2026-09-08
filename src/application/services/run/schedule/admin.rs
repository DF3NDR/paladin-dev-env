//! `ScheduleAdminPort` implementation for `ScheduleService` (PLAT-05, D-42, D-46).
//!
//! Validate-then-persist: [`ScheduleService::create`]/[`ScheduleService::patch`] validate
//! the cron (5/6 field via `paladin_storage::cron::parse_run_cron`), the IANA timezone
//! (same call), the assistant reference (`AssistantResolver::resolve`, if a resolver is
//! wired via [`ScheduleService::with_resolver`]) and the webhook URL (a write-time SSRF
//! guard, D-42) — nothing is ever persisted until every check passes.
//!
//! # The write-time SSRF guard (D-42)
//!
//! [`SsrfGuard::check_url`] is this plan's own standalone implementation of D-42's table:
//! reject non-`http(s)` schemes and any literal-IP host that classifies as loopback,
//! link-local (`169.254.0.0/16` — which already covers the metadata address
//! `169.254.169.254` — and `fe80::/10`), RFC1918, unique-local (`fc00::/7`) or unspecified,
//! overridable only by `allow_private`. 27-13's sibling plan builds the SAME check as
//! `src/application/services/run/webhook/ssrf.rs`'s `SsrfGuard::check_url`, applied at BOTH
//! write time (here, on schedule create/patch) and send time (27-13's webhook client, on
//! every delivery attempt) — this module's copy is written so a later plan (27-15) can
//! delete it and route both call sites through 27-13's shared guard with no behavior
//! change, just a `use` update (the `check_url` name and signature are chosen to match).
//!
//! A non-IP-literal hostname is accepted by this WRITE-time check without a live DNS
//! resolution dependency (this facade has no reason to hold a resolver just to validate a
//! schedule at create/patch time); a hostname that only later resolves to a private address
//! is still caught at SEND time by 27-13's guard, which resolves immediately before every
//! delivery attempt. This is a documented, intentional scope boundary, not an oversight —
//! see this crate's `security.instructions.md` precedent of naming a gap rather than
//! claiming coverage that does not exist.

use std::net::{Ipv4Addr, Ipv6Addr};

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

/// Write-time (and, in 27-13, send-time) guard against SSRF via a schedule's webhook URL
/// (D-42). `allow_private` defaults to `false` — the X-09 safe default; a future
/// `webhooks.allow_private` config knob (owned outside this plan's file scope) is what would
/// flip it.
#[derive(Debug, Clone, Copy, Default)]
pub struct SsrfGuard {
    /// Overrides every host-classification rejection when `true` (D-42,
    /// `webhooks.allow_private`). Defaults to `false`.
    pub allow_private: bool,
}

impl SsrfGuard {
    /// Construct a guard with `allow_private` set explicitly.
    pub fn new(allow_private: bool) -> Self {
        Self { allow_private }
    }

    /// Reject a webhook URL per D-42's table. `Ok(())` when the URL is acceptable to send an
    /// unattended, credentialed request to.
    ///
    /// # Errors
    ///
    /// Returns a human-readable rejection reason (never persisted verbatim to a client
    /// without being wrapped in a [`ValidationViolation`] by the caller).
    pub fn check_url(&self, raw_url: &str) -> Result<(), String> {
        let parsed = url::Url::parse(raw_url).map_err(|e| format!("invalid webhook URL: {e}"))?;
        match parsed.scheme() {
            "http" | "https" => {}
            other => {
                return Err(format!(
                    "webhook URL scheme must be http or https, got {other:?}"
                ));
            }
        }
        if self.allow_private {
            return Ok(());
        }
        let Some(host) = parsed.host() else {
            return Err("webhook URL must have a host".to_string());
        };
        match host {
            url::Host::Domain(domain) => {
                if domain.eq_ignore_ascii_case("localhost") {
                    return Err("webhook URL host must not be localhost".to_string());
                }
                Ok(())
            }
            url::Host::Ipv4(ip) => classify_ipv4(ip),
            url::Host::Ipv6(ip) => classify_ipv6(ip),
        }
    }
}

fn classify_ipv4(ip: Ipv4Addr) -> Result<(), String> {
    if ip.is_loopback() || ip.is_link_local() || ip.is_private() || ip.is_unspecified() {
        return Err(format!(
            "webhook URL host {ip} resolves to a disallowed loopback/link-local/private/unspecified address"
        ));
    }
    Ok(())
}

fn classify_ipv6(ip: Ipv6Addr) -> Result<(), String> {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return classify_ipv4(mapped);
    }
    let segments = ip.segments();
    let is_link_local = (segments[0] & 0xffc0) == 0xfe80; // fe80::/10
    let is_unique_local = (segments[0] & 0xfe00) == 0xfc00; // fc00::/7
    if ip.is_loopback() || ip.is_unspecified() || is_link_local || is_unique_local {
        return Err(format!(
            "webhook URL host {ip} resolves to a disallowed loopback/link-local/unique-local/unspecified address"
        ));
    }
    Ok(())
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
            && let Err(message) = self.ssrf_guard.check_url(&webhook.url)
        {
            violations.push(ValidationViolation::new(
                "/webhook/url",
                "webhook_url_rejected",
                message,
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
            && let Err(message) = self.ssrf_guard.check_url(&webhook.url)
        {
            violations.push(ValidationViolation::new(
                "/webhook/url",
                "webhook_url_rejected",
                message,
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

#[cfg(test)]
mod ssrf_guard_tests {
    use super::SsrfGuard;

    #[test]
    fn rejects_non_http_scheme() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("ftp://example.com/hook").is_err());
    }

    #[test]
    fn rejects_loopback_ipv4() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("http://127.0.0.1/hook").is_err());
    }

    #[test]
    fn rejects_metadata_link_local_ipv4() {
        let guard = SsrfGuard::default();
        assert!(
            guard
                .check_url("http://169.254.169.254/latest/meta-data")
                .is_err()
        );
    }

    #[test]
    fn rejects_rfc1918_ipv4() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("http://10.0.0.5/hook").is_err());
        assert!(guard.check_url("http://192.168.1.1/hook").is_err());
    }

    #[test]
    fn rejects_unspecified_ipv4() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("http://0.0.0.0/hook").is_err());
    }

    #[test]
    fn rejects_loopback_and_unique_local_ipv6() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("http://[::1]/hook").is_err());
        assert!(guard.check_url("http://[fc00::1]/hook").is_err());
        assert!(guard.check_url("http://[fe80::1]/hook").is_err());
    }

    #[test]
    fn rejects_localhost_hostname() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("http://localhost/hook").is_err());
    }

    #[test]
    fn accepts_public_https_hostname() {
        let guard = SsrfGuard::default();
        assert!(guard.check_url("https://example.com/hook").is_ok());
    }

    #[test]
    fn allow_private_overrides_every_rejection() {
        let guard = SsrfGuard::new(true);
        assert!(guard.check_url("http://127.0.0.1/hook").is_ok());
        assert!(guard.check_url("http://169.254.169.254/latest").is_ok());
    }
}
