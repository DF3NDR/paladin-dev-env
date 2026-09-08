//! Run identity, the pure run status machine, and the `Run` aggregate.
//!
//! This module defines the core types the Platform API (PRD 06) submits,
//! persists and publishes over HTTP: [`RunId`] addresses one run, [`Run`]
//! is both the persisted `runs` row shape (plan 27-02) and the published
//! `GET /v1/runs/{run_id}` response body, and [`RunStatus::try_transition`]
//! is the single pure function every layer above this one routes a status
//! change through (D-01, D-02).
//!
//! # One-way door (D-01)
//!
//! `Run`'s field set and `RunStatus`'s seven-value vocabulary are frozen for
//! v0.10.0: changing either after release needs a data migration for every
//! deployed store plus a wire break for every generated SDK client. `Run` is
//! `#[non_exhaustive]` and every field beyond the identity trio
//! (`run_id`/`thread_id`/`assistant`) carries `#[serde(default)]`, so adding
//! a field later stays additive in both directions (X-10.3) even though the
//! *existing* shape itself is a one-way door.
//!
//! # Schema versioning (X-04)
//!
//! Every persisted `Run` carries [`RUN_SCHEMA_VERSION`] in its own
//! `schema_version` field, mirroring the `Waypoint`/`Battlefield` precedent
//! in [`crate::platform::container::waypoint`].
//!
//! # No status-history type (D-05)
//!
//! This module deliberately does not define a `run_status_history` type: the
//! Waypoint chain (`GET /threads/{id}/history`) is already the execution
//! audit trail PLAT-01 needs, and `Run`'s own `status` plus
//! `submitted_at`/`started_at`/`finished_at` are enough for the monotonic
//! status machine itself.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::parley::ParleyResponse;
use crate::platform::container::waypoint::ThreadId;

/// Schema version stamped on every persisted [`Run`] (X-04).
pub const RUN_SCHEMA_VERSION: &str = "v1";

fn default_run_schema_version() -> String {
    RUN_SCHEMA_VERSION.to_string()
}

/// Identity of a run: a UUIDv7 (time-ordered) value, so runs within a thread
/// sort chronologically by id alone — the same convention
/// [`crate::platform::container::waypoint::WaypointId`] already established.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

/// Error returned by [`RunId::parse`] when the supplied string is not a
/// valid UUID.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunIdError {
    /// The supplied run id was empty.
    #[error("run id must not be empty")]
    Empty,
    /// The supplied run id was not a valid UUID.
    #[error("run id {value:?} is not a valid UUID")]
    InvalidUuid {
        /// The rejected value.
        value: String,
    },
}

impl RunId {
    /// Generate a fresh, time-ordered `RunId` (UUIDv7).
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Parse a `RunId` from a caller-supplied string, validating it is a
    /// well-formed UUID.
    ///
    /// # Errors
    ///
    /// Returns [`RunIdError::Empty`] for an empty string, or
    /// [`RunIdError::InvalidUuid`] if the string is not a valid UUID.
    pub fn parse(id: impl Into<String>) -> Result<Self, RunIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(RunIdError::Empty);
        }
        Uuid::parse_str(&id).map_err(|_| RunIdError::InvalidUuid { value: id.clone() })?;
        Ok(Self(id))
    }

    /// Borrow the run id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The status of a [`Run`], as a monotonic state machine (D-02).
///
/// The four terminal statuses (`Completed`, `Failed`, `Halted`, `Cancelled`)
/// are absorbing: no [`RunStatus::try_transition`] call with a terminal
/// `from` ever returns `Ok`. Serializes to the snake_case strings the SQL
/// `runs.status` column stores (`"queued"`, `"awaiting_input"`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Persisted and enqueued, not yet picked up by a worker.
    Queued,
    /// A worker is actively driving the engine for this run.
    Running,
    /// Suspended awaiting a parley response (HITL).
    AwaitingInput,
    /// Finished normally. Terminal.
    Completed,
    /// Finished with an error. Terminal.
    Failed,
    /// Gracefully halted (cancellation or shutdown drain). Terminal.
    Halted,
    /// Cancelled before or during execution. Terminal.
    Cancelled,
}

impl Default for RunStatus {
    /// A freshly constructed [`Run`] starts `Queued`; also the fallback for
    /// a `Run` payload written before this field existed (X-10.3).
    fn default() -> Self {
        RunStatus::Queued
    }
}

impl RunStatus {
    /// The canonical snake_case string this status stores as (SQL column
    /// value and wire representation).
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::AwaitingInput => "awaiting_input",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Halted => "halted",
            RunStatus::Cancelled => "cancelled",
        }
    }

    /// Whether this status is one of the four absorbing terminal statuses.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Halted | RunStatus::Cancelled
        )
    }

    /// Whether this status counts as an active (busy) run for the
    /// one-active-run-per-thread invariant (D-18): `Queued`, `Running` and
    /// `AwaitingInput` all count as busy.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            RunStatus::Queued | RunStatus::Running | RunStatus::AwaitingInput
        )
    }

    /// Attempt a transition from `from` to `to`, returning the new status on
    /// success or a structured [`IllegalTransition`] naming both endpoints
    /// on failure (X-06 — never a bare `bool`).
    ///
    /// Implements D-02's edge table exactly:
    /// - `Queued -> {Running, Cancelled}`
    /// - `Running -> {Completed, Failed, Halted, Cancelled, AwaitingInput}`
    /// - `AwaitingInput -> {Running, Cancelled, Failed}`
    ///
    /// Every other ordered pair — including every self-transition and every
    /// edge leaving a terminal status — is illegal.
    ///
    /// # Errors
    ///
    /// Returns [`IllegalTransition`] for any pair not in the table above.
    ///
    /// ```
    /// use paladin_core::platform::container::run::{IllegalTransition, RunStatus};
    ///
    /// // A legal edge: Queued -> Running.
    /// assert_eq!(
    ///     RunStatus::try_transition(RunStatus::Queued, RunStatus::Running),
    ///     Ok(RunStatus::Running)
    /// );
    ///
    /// // An illegal edge: Completed is terminal, so nothing leaves it.
    /// assert_eq!(
    ///     RunStatus::try_transition(RunStatus::Completed, RunStatus::Running),
    ///     Err(IllegalTransition {
    ///         from: RunStatus::Completed,
    ///         to: RunStatus::Running
    ///     })
    /// );
    /// ```
    pub fn try_transition(from: RunStatus, to: RunStatus) -> Result<RunStatus, IllegalTransition> {
        let legal = matches!(
            (from, to),
            (RunStatus::Queued, RunStatus::Running)
                | (RunStatus::Queued, RunStatus::Cancelled)
                | (RunStatus::Running, RunStatus::Completed)
                | (RunStatus::Running, RunStatus::Failed)
                | (RunStatus::Running, RunStatus::Halted)
                | (RunStatus::Running, RunStatus::Cancelled)
                | (RunStatus::Running, RunStatus::AwaitingInput)
                | (RunStatus::AwaitingInput, RunStatus::Running)
                | (RunStatus::AwaitingInput, RunStatus::Cancelled)
                | (RunStatus::AwaitingInput, RunStatus::Failed)
        );
        if legal {
            Ok(to)
        } else {
            Err(IllegalTransition { from, to })
        }
    }
}

/// Error returned by [`RunStatus::from_str`] for an unrecognized status
/// string.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown run status: {0:?}")]
pub struct RunStatusParseError(pub String);

impl FromStr for RunStatus {
    type Err = RunStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(RunStatus::Queued),
            "running" => Ok(RunStatus::Running),
            "awaiting_input" => Ok(RunStatus::AwaitingInput),
            "completed" => Ok(RunStatus::Completed),
            "failed" => Ok(RunStatus::Failed),
            "halted" => Ok(RunStatus::Halted),
            "cancelled" => Ok(RunStatus::Cancelled),
            other => Err(RunStatusParseError(other.to_string())),
        }
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A structured, typed error naming both endpoints of a rejected
/// [`RunStatus::try_transition`] call (X-06 — never a bare `bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("illegal run status transition from {from} to {to}")]
pub struct IllegalTransition {
    /// The status the transition was attempted from.
    pub from: RunStatus,
    /// The status the transition was attempted to.
    pub to: RunStatus,
}

/// A reference to the assistant (workflow or agent) a [`Run`] was submitted
/// against, resolved and frozen at submit time (D-30).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantRef {
    /// The assistant's identity.
    pub assistant_id: String,
    /// The specific, immutable version resolved at submit time.
    pub version: u32,
}

/// The run lifecycle events a [`WebhookSpec`] may subscribe to: the four
/// terminal statuses plus `AwaitingInput` (HITL suspension is a notable
/// event even though the run is not yet finished).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEventKind {
    /// The run's `AwaitingInput` suspension.
    AwaitingInput,
    /// The run reached `Completed`.
    Completed,
    /// The run reached `Failed`.
    Failed,
    /// The run reached `Halted`.
    Halted,
    /// The run reached `Cancelled`.
    Cancelled,
}

/// A caller-supplied webhook delivery target for a [`Run`]'s lifecycle
/// events.
///
/// `secret` is given a manual [`std::fmt::Debug`] impl that prints a
/// redaction placeholder rather than the value — this type must never leak
/// its secret into a log line, a trace event or an error body (threat
/// T-27-02, `security.instructions.md`).
#[derive(Clone, Serialize, Deserialize)]
pub struct WebhookSpec {
    /// The delivery URL.
    pub url: String,
    /// The HMAC signing secret, if any. Never rendered by `Debug`.
    pub secret: Option<String>,
    /// The events this webhook subscribes to.
    pub events: Vec<RunEventKind>,
}

impl std::fmt::Debug for WebhookSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookSpec")
            .field("url", &self.url)
            .field(
                "secret",
                &self.secret.as_ref().map(|_| "[redacted]").unwrap_or(""),
            )
            .field("events", &self.events)
            .finish()
    }
}

/// A request to fork a run from a specific Waypoint, optionally editing
/// state at the fork point. Consumed by the fork route (plan 27-15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForkSpec {
    /// The Waypoint id to fork from.
    pub from_waypoint_id: String,
    /// An optional state edit applied at the fork point.
    pub edit: Option<serde_json::Value>,
}

/// The keyset cursor `RunRepositoryPort::list` (plan 27-02) pages on,
/// ordered `(submitted_at DESC, run_id DESC)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCursor {
    /// The `submitted_at` of the last item on the previous page.
    pub submitted_at: DateTime<Utc>,
    /// The `run_id` of the last item on the previous page (tiebreaker).
    pub run_id: RunId,
}

/// A single execution of an assistant against a thread.
///
/// `Run` is simultaneously the persisted `runs` SQL row (plan 27-02) and the
/// published `GET /v1/runs/{run_id}` response body (D-01) — see this
/// module's own docs for why that makes its shape a one-way door. Construct
/// through [`Run::new`] plus the `with_*` builder methods; `#[non_exhaustive]`
/// means a struct literal from outside this crate will not compile, which is
/// what keeps future field additions non-breaking (X-10.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Run {
    /// This run's identity.
    pub run_id: RunId,
    /// The thread this run executes against.
    pub thread_id: ThreadId,
    /// The assistant reference, resolved and frozen at submit time.
    pub assistant: AssistantRef,
    /// The caller-supplied input. Submit-time validation checks only the
    /// assistant reference and body shape — an empty object or omitted
    /// input is always accepted at submit; a schema violation surfaces
    /// later as a `Failed` run, never as a 400.
    #[serde(default)]
    pub input: serde_json::Value,
    /// This run's current status.
    #[serde(default)]
    pub status: RunStatus,
    /// When this run was submitted.
    #[serde(default = "Utc::now")]
    pub submitted_at: DateTime<Utc>,
    /// When this run entered `Running`, if it has.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// When this run reached a terminal status, if it has.
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    /// An optional webhook delivery target for this run's lifecycle events.
    #[serde(default)]
    pub webhook: Option<WebhookSpec>,
    /// The attempt counter, shared by redelivery and resume (D-23).
    #[serde(default)]
    pub attempt: u32,
    /// Whether a cancel has been requested (durable flag, D-16).
    #[serde(default)]
    pub cancel_requested: bool,
    /// The engine's error, if this run is `Failed`.
    #[serde(default)]
    pub error: Option<String>,
    /// Responses parked on this row for the worker to consume on resume
    /// (D-07, D-09; filled by plan 27-08).
    #[serde(default)]
    pub pending_responses: Vec<ParleyResponse>,
    /// The fork this run was created from, if any.
    #[serde(default)]
    pub fork_from: Option<ForkSpec>,
    /// The final output, for an `Agent`-kind assistant.
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    /// The final Waypoint id this run reached, if any.
    #[serde(default)]
    pub final_waypoint_id: Option<String>,
    /// Schema version this row was persisted under (X-04).
    #[serde(default = "default_run_schema_version")]
    pub schema_version: String,
}

impl Run {
    /// Construct a fresh `Run` in `Queued` status, submitted now, with
    /// attempt `1` and every optional field unset.
    pub fn new(
        run_id: RunId,
        thread_id: ThreadId,
        assistant: AssistantRef,
        input: serde_json::Value,
    ) -> Self {
        Self {
            run_id,
            thread_id,
            assistant,
            input,
            status: RunStatus::Queued,
            submitted_at: Utc::now(),
            started_at: None,
            finished_at: None,
            webhook: None,
            attempt: 1,
            cancel_requested: false,
            error: None,
            pending_responses: Vec::new(),
            fork_from: None,
            output: None,
            final_waypoint_id: None,
            schema_version: RUN_SCHEMA_VERSION.to_string(),
        }
    }

    /// Attach a webhook delivery target.
    pub fn with_webhook(mut self, webhook: WebhookSpec) -> Self {
        self.webhook = Some(webhook);
        self
    }

    /// Record that this run was created from a fork.
    pub fn with_fork_from(mut self, fork: ForkSpec) -> Self {
        self.fork_from = Some(fork);
        self
    }

    /// Override the initial status (test/fixture convenience — production
    /// code should route status changes through [`RunStatus::try_transition`]).
    pub fn with_status(mut self, status: RunStatus) -> Self {
        self.status = status;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STATUSES: [RunStatus; 7] = [
        RunStatus::Queued,
        RunStatus::Running,
        RunStatus::AwaitingInput,
        RunStatus::Completed,
        RunStatus::Failed,
        RunStatus::Halted,
        RunStatus::Cancelled,
    ];

    fn is_legal_edge(from: RunStatus, to: RunStatus) -> bool {
        matches!(
            (from, to),
            (RunStatus::Queued, RunStatus::Running)
                | (RunStatus::Queued, RunStatus::Cancelled)
                | (RunStatus::Running, RunStatus::Completed)
                | (RunStatus::Running, RunStatus::Failed)
                | (RunStatus::Running, RunStatus::Halted)
                | (RunStatus::Running, RunStatus::Cancelled)
                | (RunStatus::Running, RunStatus::AwaitingInput)
                | (RunStatus::AwaitingInput, RunStatus::Running)
                | (RunStatus::AwaitingInput, RunStatus::Cancelled)
                | (RunStatus::AwaitingInput, RunStatus::Failed)
        )
    }

    /// PRD 06 §5 item 1: the exhaustive cross-product over every ordered
    /// pair of statuses, asserted against an independently written legality
    /// table rather than a hand-listed sample.
    #[test]
    fn try_transition_matches_the_full_cross_product_table() {
        for &from in &ALL_STATUSES {
            for &to in &ALL_STATUSES {
                let result = RunStatus::try_transition(from, to);
                if is_legal_edge(from, to) {
                    assert_eq!(result, Ok(to), "expected {from:?} -> {to:?} to be legal");
                } else {
                    assert_eq!(
                        result,
                        Err(IllegalTransition { from, to }),
                        "expected {from:?} -> {to:?} to be illegal"
                    );
                }
            }
        }
    }

    #[test]
    fn try_transition_rejects_every_self_transition() {
        for &status in &ALL_STATUSES {
            assert_eq!(
                RunStatus::try_transition(status, status),
                Err(IllegalTransition {
                    from: status,
                    to: status
                }),
                "self-transition on {status:?} must be illegal"
            );
        }
    }

    #[test]
    fn every_terminal_status_is_absorbing() {
        let terminals = [
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Halted,
            RunStatus::Cancelled,
        ];
        for &terminal in &terminals {
            assert!(terminal.is_terminal());
            for &to in &ALL_STATUSES {
                assert!(
                    RunStatus::try_transition(terminal, to).is_err(),
                    "no edge should leave terminal status {terminal:?}, but {terminal:?} -> {to:?} succeeded"
                );
            }
        }
    }

    #[test]
    fn queued_running_and_awaiting_input_are_active_others_are_not() {
        assert!(RunStatus::Queued.is_active());
        assert!(RunStatus::Running.is_active());
        assert!(RunStatus::AwaitingInput.is_active());
        assert!(!RunStatus::Completed.is_active());
        assert!(!RunStatus::Failed.is_active());
        assert!(!RunStatus::Halted.is_active());
        assert!(!RunStatus::Cancelled.is_active());
    }

    #[test]
    fn as_str_and_from_str_round_trip_for_all_seven_statuses() {
        for &status in &ALL_STATUSES {
            let s = status.as_str();
            let parsed: RunStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn as_str_uses_expected_snake_case_strings() {
        assert_eq!(RunStatus::Queued.as_str(), "queued");
        assert_eq!(RunStatus::Running.as_str(), "running");
        assert_eq!(RunStatus::AwaitingInput.as_str(), "awaiting_input");
        assert_eq!(RunStatus::Completed.as_str(), "completed");
        assert_eq!(RunStatus::Failed.as_str(), "failed");
        assert_eq!(RunStatus::Halted.as_str(), "halted");
        assert_eq!(RunStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn from_str_rejects_unknown_status() {
        assert_eq!(
            "bogus".parse::<RunStatus>(),
            Err(RunStatusParseError("bogus".to_string()))
        );
    }

    #[test]
    fn run_status_serde_uses_snake_case() {
        for &status in &ALL_STATUSES {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
            let restored: RunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, status);
        }
    }

    #[test]
    fn run_id_new_v7_round_trips_through_parse() {
        let id = RunId::new_v7();
        let parsed = RunId::parse(id.as_str().to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn run_id_parse_rejects_empty_and_invalid() {
        assert_eq!(RunId::parse(""), Err(RunIdError::Empty));
        assert!(matches!(
            RunId::parse("not-a-uuid"),
            Err(RunIdError::InvalidUuid { .. })
        ));
    }

    #[test]
    fn webhook_spec_debug_redacts_secret() {
        let spec = WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: Some("super-secret-value".to_string()),
            events: vec![RunEventKind::Completed],
        };
        let debug = format!("{spec:?}");
        assert!(!debug.contains("super-secret-value"));
        assert!(debug.contains("[redacted]"));
    }

    #[test]
    fn webhook_spec_debug_with_no_secret_does_not_claim_redaction() {
        let spec = WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: None,
            events: vec![],
        };
        let debug = format!("{spec:?}");
        assert!(!debug.contains("[redacted]"));
    }

    #[test]
    fn run_round_trips_through_serde_json() {
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("thread-1").unwrap(),
            AssistantRef {
                assistant_id: "assistant-1".to_string(),
                version: 1,
            },
            serde_json::json!({}),
        );
        let json = serde_json::to_string(&run).unwrap();
        let restored: Run = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.run_id, run.run_id);
        assert_eq!(restored.thread_id, run.thread_id);
        assert_eq!(restored.status, RunStatus::Queued);
        assert_eq!(restored.schema_version, RUN_SCHEMA_VERSION);
    }

    #[test]
    fn run_deserializes_with_only_identity_trio_and_required_fields_present() {
        // A JSON document missing every field added after the initial set
        // still deserializes (X-10.3), defaulting status to Queued and
        // schema_version to the current constant.
        let minimal = serde_json::json!({
            "run_id": RunId::new_v7().as_str(),
            "thread_id": "thread-minimal",
            "assistant": { "assistant_id": "a1", "version": 1 },
        });
        let run: Run = serde_json::from_str(&minimal.to_string()).unwrap();
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(run.schema_version, RUN_SCHEMA_VERSION);
        assert_eq!(run.attempt, 0);
        assert!(run.pending_responses.is_empty());
        assert!(run.webhook.is_none());
    }

    #[test]
    fn run_builder_with_methods_set_expected_fields() {
        let webhook = WebhookSpec {
            url: "https://example.com/hook".to_string(),
            secret: None,
            events: vec![RunEventKind::Failed],
        };
        let fork = ForkSpec {
            from_waypoint_id: "wp-1".to_string(),
            edit: None,
        };
        let run = Run::new(
            RunId::new_v7(),
            ThreadId::new("thread-2").unwrap(),
            AssistantRef {
                assistant_id: "a2".to_string(),
                version: 3,
            },
            serde_json::json!({"key": "value"}),
        )
        .with_webhook(webhook)
        .with_fork_from(fork)
        .with_status(RunStatus::Running);

        assert!(run.webhook.is_some());
        assert!(run.fork_from.is_some());
        assert_eq!(run.status, RunStatus::Running);
    }

    #[test]
    fn assistant_ref_round_trips() {
        let reference = AssistantRef {
            assistant_id: "assistant-1".to_string(),
            version: 5,
        };
        let json = serde_json::to_string(&reference).unwrap();
        let restored: AssistantRef = serde_json::from_str(&json).unwrap();
        assert_eq!(reference, restored);
    }

    #[test]
    fn run_cursor_round_trips() {
        let cursor = RunCursor {
            submitted_at: Utc::now(),
            run_id: RunId::new_v7(),
        };
        let json = serde_json::to_string(&cursor).unwrap();
        let restored: RunCursor = serde_json::from_str(&json).unwrap();
        assert_eq!(cursor, restored);
    }
}
