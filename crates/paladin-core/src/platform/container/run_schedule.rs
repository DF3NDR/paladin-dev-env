//! Run schedule identity and the `RunSchedule` aggregate (PLAT-05, D-36..D-39).
//!
//! This module defines the core types the Platform API's schedule surface
//! submits, persists and publishes over HTTP: [`RunScheduleId`] addresses one
//! schedule, [`RunSchedule`] is the persisted `run_schedules` row shape (plan
//! 27-11), and [`RunScheduleUpdate`] carries a partial update.
//!
//! # Not `tokio-cron-scheduler` (D-36)
//!
//! The existing `TokioCronSchedulerAdapter`
//! (`crates/paladin-storage/src/scheduler.rs`) is in-memory, offers no
//! persistence and no status query -- exactly what PLAT-FR-13 forbids. It
//! stays unchanged for its own `SchedulerPort` consumers (X-03). Run
//! schedules get their own repository port
//! (`paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort`)
//! and a facade `ScheduleService` driving them.
//!
//! # Restart- and replica-safety without leader election (D-37)
//!
//! A tick is CLAIMED by a conditional update: `UPDATE run_schedules SET
//! last_tick = ?next, next_tick = ?after WHERE schedule_id = ? AND next_tick
//! = ?next` -- exactly one replica's update affects a row, and only that
//! replica submits the run. `last_tick`/`next_tick` persist this claim, so a
//! restart neither double-fires (the claim already advanced `next_tick`) nor
//! missed-then-double-fires (`on_missed: Skip` recomputes from now;
//! `RunOnce` fires once, then recomputes).
//!
//! # Schema versioning (X-04)
//!
//! Every persisted [`RunSchedule`] carries [`RUN_SCHEDULE_SCHEMA_VERSION`] in
//! its own `schema_version` field, mirroring the `Run`/`Assistant`/`Waypoint`
//! precedent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::run::WebhookSpec;
use crate::platform::container::waypoint::ThreadId;

/// Schema version stamped on every persisted [`RunSchedule`] (X-04).
pub const RUN_SCHEDULE_SCHEMA_VERSION: &str = "v1";

fn default_run_schedule_schema_version() -> String {
    RUN_SCHEDULE_SCHEMA_VERSION.to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_enabled() -> bool {
    true
}

/// Identity of a run schedule: a UUIDv7 (time-ordered) value, mirroring
/// [`crate::platform::container::run::RunId`]'s own convention.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunScheduleId(String);

/// Error returned by [`RunScheduleId::parse`] when the supplied string is not
/// a valid UUID.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunScheduleIdError {
    /// The supplied schedule id was empty.
    #[error("run schedule id must not be empty")]
    Empty,
    /// The supplied schedule id was not a valid UUID.
    #[error("run schedule id {value:?} is not a valid UUID")]
    InvalidUuid {
        /// The rejected value.
        value: String,
    },
}

impl RunScheduleId {
    /// Generate a fresh, time-ordered `RunScheduleId` (UUIDv7).
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Parse a `RunScheduleId` from a caller-supplied string, validating it
    /// is a well-formed UUID.
    ///
    /// # Errors
    ///
    /// Returns [`RunScheduleIdError::Empty`] for an empty string, or
    /// [`RunScheduleIdError::InvalidUuid`] if the string is not a valid
    /// UUID.
    pub fn parse(id: impl Into<String>) -> Result<Self, RunScheduleIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(RunScheduleIdError::Empty);
        }
        Uuid::parse_str(&id).map_err(|_| RunScheduleIdError::InvalidUuid { value: id.clone() })?;
        Ok(Self(id))
    }

    /// Borrow the schedule id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunScheduleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Which thread a tick's submitted run executes against (D-39). Serializes
/// as the default externally-tagged `serde` enum representation with
/// `rename_all = "snake_case"`: the unit variant is the bare string
/// `"new_thread_per_tick"`, the tuple variant is `{ "fixed_thread": "<id>" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStrategy {
    /// Each tick submits a run against a fresh thread (the default).
    NewThreadPerTick,
    /// Every tick submits a run against the SAME thread. A tick that lands
    /// while that thread is busy skips (and increments `skipped_ticks` --
    /// D-39).
    FixedThread(ThreadId),
}

impl Default for ThreadStrategy {
    /// PLAT-05's default: each tick gets a fresh thread (D-39).
    fn default() -> Self {
        ThreadStrategy::NewThreadPerTick
    }
}

/// What happens when a tick is discovered LATE -- more than roughly one tick
/// interval after `next_tick` was due (D-39, PRD acceptance 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnMissed {
    /// Recompute `next_tick` from now and submit nothing for the missed
    /// window (the default).
    Skip,
    /// Submit exactly one run for the missed window, then recompute
    /// `next_tick` from now.
    RunOnce,
}

impl Default for OnMissed {
    /// PLAT-05's default: a missed tick is skipped, not caught up (D-39).
    fn default() -> Self {
        OnMissed::Skip
    }
}

/// A persisted cron schedule that submits a run on every tick (PLAT-05,
/// D-36..D-39).
///
/// `RunSchedule` is simultaneously the persisted `run_schedules` SQL row
/// (plan 27-11) and the published schedule HTTP response body (the HTTP
/// surface lands in plan 27-14). Construct through [`RunSchedule::new`] plus
/// the `with_*` builder methods; `#[non_exhaustive]` keeps future field
/// additions non-breaking (X-10.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunSchedule {
    /// This schedule's identity.
    pub schedule_id: RunScheduleId,
    /// The assistant this schedule submits runs against.
    pub assistant_id: String,
    /// A specific version, or `None` to resolve `latest` at each tick
    /// (mirrors `SubmitRun::version`).
    #[serde(default)]
    pub version: Option<u32>,
    /// The cron expression (5- or 6-field, `crates/paladin-storage/src/cron.rs`
    /// parses either form -- D-38).
    pub cron: String,
    /// The IANA timezone the cron expression is evaluated in. Defaults to
    /// `"UTC"`.
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// The input every tick's submitted run receives.
    #[serde(default)]
    pub input: serde_json::Value,
    /// Whether this schedule is currently active. A disabled schedule never
    /// appears in `RunScheduleRepositoryPort::due`.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Which thread each tick's run executes against (D-39).
    #[serde(default)]
    pub thread_strategy: ThreadStrategy,
    /// What happens when a tick is discovered late (D-39).
    #[serde(default)]
    pub on_missed: OnMissed,
    /// An optional webhook delivery target for each tick's submitted run.
    #[serde(default)]
    pub webhook: Option<WebhookSpec>,
    /// The last tick this schedule fired (or was claimed) at, if any.
    #[serde(default)]
    pub last_tick: Option<DateTime<Utc>>,
    /// The next tick this schedule is due at, if scheduled.
    #[serde(default)]
    pub next_tick: Option<DateTime<Utc>>,
    /// How many ticks have been skipped (a `FixedThread` busy-thread skip,
    /// or an `OnMissed::Skip` missed window) -- the PLAT-FR-13 counted
    /// metric (D-39).
    #[serde(default)]
    pub skipped_ticks: u64,
    /// When this schedule was created.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// When this schedule was last updated.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version this row was persisted under (X-04).
    #[serde(default = "default_run_schedule_schema_version")]
    pub schema_version: String,
}

impl RunSchedule {
    /// Construct a fresh `RunSchedule`: enabled, `"UTC"` timezone, no
    /// version pin, `NewThreadPerTick`/`Skip` defaults, no tick recorded
    /// yet.
    pub fn new(
        schedule_id: RunScheduleId,
        assistant_id: impl Into<String>,
        cron: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            schedule_id,
            assistant_id: assistant_id.into(),
            version: None,
            cron: cron.into(),
            timezone: default_timezone(),
            input: serde_json::Value::Null,
            enabled: true,
            thread_strategy: ThreadStrategy::default(),
            on_missed: OnMissed::default(),
            webhook: None,
            last_tick: None,
            next_tick: None,
            skipped_ticks: 0,
            created_at: now,
            updated_at: now,
            schema_version: RUN_SCHEDULE_SCHEMA_VERSION.to_string(),
        }
    }

    /// Pin a specific assistant version.
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    /// Override the default `"UTC"` timezone.
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = timezone.into();
        self
    }

    /// Set the input every tick's submitted run receives.
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    /// Override the default `NewThreadPerTick` strategy.
    pub fn with_thread_strategy(mut self, strategy: ThreadStrategy) -> Self {
        self.thread_strategy = strategy;
        self
    }

    /// Override the default `Skip` missed-tick policy.
    pub fn with_on_missed(mut self, on_missed: OnMissed) -> Self {
        self.on_missed = on_missed;
        self
    }

    /// Attach a webhook delivery target.
    pub fn with_webhook(mut self, webhook: WebhookSpec) -> Self {
        self.webhook = Some(webhook);
        self
    }

    /// Set the initial `next_tick` (test/fixture convenience -- production
    /// code computes this from `cron::parse_run_cron`).
    pub fn with_next_tick(mut self, next_tick: DateTime<Utc>) -> Self {
        self.next_tick = Some(next_tick);
        self
    }

    /// Construct disabled (test/fixture convenience).
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// A partial update to a [`RunSchedule`] (every field optional -- only
/// fields set to `Some` are changed).
#[derive(Debug, Clone, Default)]
pub struct RunScheduleUpdate {
    /// A new cron expression, if changing it.
    pub cron: Option<String>,
    /// A new timezone, if changing it.
    pub timezone: Option<String>,
    /// A new input, if changing it.
    pub input: Option<serde_json::Value>,
    /// A new enabled flag, if changing it.
    pub enabled: Option<bool>,
    /// A new thread strategy, if changing it.
    pub thread_strategy: Option<ThreadStrategy>,
    /// A new missed-tick policy, if changing it.
    pub on_missed: Option<OnMissed>,
    /// A new webhook target, if changing it.
    pub webhook: Option<WebhookSpec>,
    /// A new `next_tick`, if changing it (e.g. recomputing after an
    /// `enabled` flip).
    pub next_tick: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_schedule_id_new_v7_round_trips_through_parse() {
        let id = RunScheduleId::new_v7();
        let parsed = RunScheduleId::parse(id.as_str().to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn run_schedule_id_parse_rejects_empty_and_invalid() {
        assert_eq!(RunScheduleId::parse(""), Err(RunScheduleIdError::Empty));
        assert!(matches!(
            RunScheduleId::parse("not-a-uuid"),
            Err(RunScheduleIdError::InvalidUuid { .. })
        ));
    }

    #[test]
    fn thread_strategy_default_is_new_thread_per_tick() {
        assert_eq!(ThreadStrategy::default(), ThreadStrategy::NewThreadPerTick);
    }

    #[test]
    fn on_missed_default_is_skip() {
        assert_eq!(OnMissed::default(), OnMissed::Skip);
    }

    #[test]
    fn thread_strategy_new_thread_per_tick_serializes_as_bare_string() {
        let json = serde_json::to_string(&ThreadStrategy::NewThreadPerTick).unwrap();
        assert_eq!(json, "\"new_thread_per_tick\"");
    }

    #[test]
    fn thread_strategy_fixed_thread_serializes_as_tagged_object() {
        let thread = ThreadId::new("t1").unwrap();
        let json = serde_json::to_string(&ThreadStrategy::FixedThread(thread.clone())).unwrap();
        assert_eq!(json, format!("{{\"fixed_thread\":\"{thread}\"}}"));
        let restored: ThreadStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, ThreadStrategy::FixedThread(thread));
    }

    #[test]
    fn on_missed_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&OnMissed::Skip).unwrap(), "\"skip\"");
        assert_eq!(
            serde_json::to_string(&OnMissed::RunOnce).unwrap(),
            "\"run_once\""
        );
    }

    #[test]
    fn run_schedule_round_trips_through_serde_json() {
        let schedule = RunSchedule::new(RunScheduleId::new_v7(), "assistant-1", "*/5 * * * *");
        let json = serde_json::to_string(&schedule).unwrap();
        let restored: RunSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.schedule_id, schedule.schedule_id);
        assert_eq!(restored.assistant_id, schedule.assistant_id);
        assert_eq!(restored.cron, schedule.cron);
        assert_eq!(restored.timezone, "UTC");
        assert!(restored.enabled);
        assert_eq!(restored.thread_strategy, ThreadStrategy::NewThreadPerTick);
        assert_eq!(restored.on_missed, OnMissed::Skip);
        assert_eq!(restored.schema_version, RUN_SCHEDULE_SCHEMA_VERSION);
    }

    #[test]
    fn run_schedule_deserializes_with_only_required_fields_present() {
        let minimal = serde_json::json!({
            "schedule_id": RunScheduleId::new_v7().as_str(),
            "assistant_id": "a1",
            "cron": "* * * * *",
        });
        let schedule: RunSchedule = serde_json::from_str(&minimal.to_string()).unwrap();
        assert_eq!(schedule.timezone, "UTC");
        assert!(schedule.enabled);
        assert_eq!(schedule.thread_strategy, ThreadStrategy::NewThreadPerTick);
        assert_eq!(schedule.on_missed, OnMissed::Skip);
        assert_eq!(schedule.skipped_ticks, 0);
        assert_eq!(schedule.schema_version, RUN_SCHEDULE_SCHEMA_VERSION);
    }

    #[test]
    fn run_schedule_builder_with_methods_set_expected_fields() {
        let thread = ThreadId::new("fixed-thread").unwrap();
        let schedule = RunSchedule::new(RunScheduleId::new_v7(), "a1", "0 9 * * *")
            .with_version(3)
            .with_timezone("Europe/Berlin")
            .with_input(serde_json::json!({"key": "value"}))
            .with_thread_strategy(ThreadStrategy::FixedThread(thread.clone()))
            .with_on_missed(OnMissed::RunOnce)
            .disabled();

        assert_eq!(schedule.version, Some(3));
        assert_eq!(schedule.timezone, "Europe/Berlin");
        assert_eq!(schedule.input, serde_json::json!({"key": "value"}));
        assert_eq!(
            schedule.thread_strategy,
            ThreadStrategy::FixedThread(thread)
        );
        assert_eq!(schedule.on_missed, OnMissed::RunOnce);
        assert!(!schedule.enabled);
    }

    #[test]
    fn run_schedule_update_default_is_all_none() {
        let update = RunScheduleUpdate::default();
        assert!(update.cron.is_none());
        assert!(update.timezone.is_none());
        assert!(update.input.is_none());
        assert!(update.enabled.is_none());
        assert!(update.thread_strategy.is_none());
        assert!(update.on_missed.is_none());
        assert!(update.webhook.is_none());
        assert!(update.next_tick.is_none());
    }
}
