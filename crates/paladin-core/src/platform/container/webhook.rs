//! Webhook delivery identity and the `WebhookDelivery` aggregate
//! (PLAT-FR-14/15, D-40..D-43).
//!
//! This module defines the persisted `webhook_deliveries` row shape (plan
//! 27-13): a delivery is a durable QUEUE entry, drained by a bounded-retry
//! service (`src/application/services/run/webhook/service.rs`), never a
//! spawned fire-and-forget task -- a restart between enqueue and send loses
//! nothing (D-40).
//!
//! # No signing key on the row (prohibition P1)
//!
//! `WebhookDelivery` deliberately carries no signing key field: the HMAC
//! signing value lives on the run's own `WebhookSpec` and is read from
//! there at send time, never copied onto a delivery row, a trace event or
//! an error body (`.github/instructions/security.instructions.md`).
//!
//! # Schema versioning (X-04)
//!
//! Every persisted [`WebhookDelivery`] carries
//! [`WEBHOOK_DELIVERY_SCHEMA_VERSION`] in its own `schema_version` field,
//! mirroring the `Run`/`RunSchedule`/`Waypoint` precedent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::platform::container::run::{RunEventKind, RunId};
use crate::platform::container::waypoint::ThreadId;

/// Schema version stamped on every persisted [`WebhookDelivery`] (X-04).
pub const WEBHOOK_DELIVERY_SCHEMA_VERSION: &str = "v1";

fn default_webhook_delivery_schema_version() -> String {
    WEBHOOK_DELIVERY_SCHEMA_VERSION.to_string()
}

/// The default bounded-retry ceiling (D-43): `4xx`/`3xx` dead-letter
/// immediately (never consult this), `5xx`/timeout/connect-error retry up
/// to this many attempts total before dead-lettering.
fn default_max_attempts() -> u32 {
    5
}

/// Identity of a webhook delivery: a UUIDv7 (time-ordered) value, mirroring
/// [`crate::platform::container::run::RunId`]'s own convention.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WebhookDeliveryId(String);

/// Error returned by [`WebhookDeliveryId::parse`] when the supplied string
/// is not a valid UUID.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WebhookDeliveryIdError {
    /// The supplied delivery id was empty.
    #[error("webhook delivery id must not be empty")]
    Empty,
    /// The supplied delivery id was not a valid UUID.
    #[error("webhook delivery id {value:?} is not a valid UUID")]
    InvalidUuid {
        /// The rejected value.
        value: String,
    },
}

impl WebhookDeliveryId {
    /// Generate a fresh, time-ordered `WebhookDeliveryId` (UUIDv7).
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// Parse a `WebhookDeliveryId` from a caller-supplied string,
    /// validating it is a well-formed UUID.
    ///
    /// # Errors
    ///
    /// Returns [`WebhookDeliveryIdError::Empty`] for an empty string, or
    /// [`WebhookDeliveryIdError::InvalidUuid`] if the string is not a valid
    /// UUID.
    pub fn parse(id: impl Into<String>) -> Result<Self, WebhookDeliveryIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(WebhookDeliveryIdError::Empty);
        }
        Uuid::parse_str(&id)
            .map_err(|_| WebhookDeliveryIdError::InvalidUuid { value: id.clone() })?;
        Ok(Self(id))
    }

    /// Borrow the delivery id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WebhookDeliveryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The lifecycle status of a [`WebhookDelivery`] (D-40, D-43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookDeliveryStatus {
    /// Enqueued, not yet claimed by the drain service.
    Pending,
    /// Claimed by a drain-service instance and currently being sent.
    InFlight,
    /// Delivered with a `2xx` response. Terminal.
    Delivered,
    /// A `5xx`/timeout/connect-error attempt failed and another attempt is
    /// scheduled at `next_attempt_at`.
    Retrying,
    /// Dead-lettered: either a `3xx`/`4xx` response, or the retry budget
    /// was exhausted. Terminal.
    Dead,
}

impl WebhookDeliveryStatus {
    /// The canonical snake_case string this status stores as (SQL column
    /// value and wire representation).
    pub fn as_str(&self) -> &'static str {
        match self {
            WebhookDeliveryStatus::Pending => "pending",
            WebhookDeliveryStatus::InFlight => "in_flight",
            WebhookDeliveryStatus::Delivered => "delivered",
            WebhookDeliveryStatus::Retrying => "retrying",
            WebhookDeliveryStatus::Dead => "dead",
        }
    }
}

/// What a `WebhookDeliveryService` attempt resolved to, and the fields to
/// persist alongside it (D-43).
#[derive(Debug, Clone, PartialEq)]
pub enum WebhookAttemptOutcome {
    /// A `2xx` response. Terminal.
    Delivered,
    /// A `5xx`/timeout/connect-error response with retry budget remaining;
    /// the next attempt is scheduled at `next_attempt_at`.
    Retrying {
        /// When the next attempt is scheduled.
        next_attempt_at: DateTime<Utc>,
    },
    /// A `3xx`/`4xx` response, or the retry budget was exhausted. Terminal.
    Dead,
}

/// The result of one delivery attempt, passed to
/// `WebhookDeliveryRepositoryPort::record_attempt` (`crates/paladin-ports`).
/// The repository increments the persisted `attempt` counter by one and
/// applies `outcome`/`response_status`/`error` atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct WebhookAttemptResult {
    /// What this attempt resolved to.
    pub outcome: WebhookAttemptOutcome,
    /// The HTTP status this attempt observed, if any (absent for a
    /// connect/timeout error that never received a response).
    pub response_status: Option<u16>,
    /// A redacted, bounded diagnostic of the failure, if any.
    pub error: Option<String>,
}

/// A durably-queued webhook delivery for one run lifecycle event (PLAT-FR-14,
/// D-40..D-43).
///
/// `WebhookDelivery` is simultaneously the persisted `webhook_deliveries` SQL
/// row (plan 27-13) and the shape a later plan's
/// `GET /runs/{id}/webhook-deliveries` publishes. Construct through
/// [`WebhookDelivery::new`] plus the `with_*` builder methods;
/// `#[non_exhaustive]` keeps future field additions non-breaking (X-10.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookDelivery {
    /// This delivery's identity.
    pub delivery_id: WebhookDeliveryId,
    /// The run this delivery reports on.
    pub run_id: RunId,
    /// The thread the run executed against.
    pub thread_id: ThreadId,
    /// Which lifecycle event this delivery carries.
    pub event: RunEventKind,
    /// The delivery URL (copied from the run's `WebhookSpec` at enqueue
    /// time).
    pub url: String,
    /// The exact JSON bytes signed and sent -- never re-serialized at send
    /// time (D-41).
    pub payload: String,
    /// How many attempts have been made so far.
    #[serde(default)]
    pub attempt: u32,
    /// The maximum number of attempts before dead-lettering (D-43).
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// This delivery's current status.
    pub status: WebhookDeliveryStatus,
    /// When this delivery is next eligible to be claimed
    /// (`WebhookDeliveryRepositoryPort::claim_due`).
    pub next_attempt_at: DateTime<Utc>,
    /// The HTTP status the most recent attempt observed, if any.
    #[serde(default)]
    pub last_response_status: Option<u16>,
    /// A redacted, bounded diagnostic of the most recent attempt's failure,
    /// if any.
    #[serde(default)]
    pub last_error: Option<String>,
    /// When this delivery was enqueued.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// When this delivery was last updated.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version this row was persisted under (X-04).
    #[serde(default = "default_webhook_delivery_schema_version")]
    pub schema_version: String,
}

impl WebhookDelivery {
    /// Construct a fresh `WebhookDelivery`: `Pending`, attempt `0`, the
    /// default (5) `max_attempts`, eligible for claim at `next_attempt_at`
    /// (typically "now").
    pub fn new(
        delivery_id: WebhookDeliveryId,
        run_id: RunId,
        thread_id: ThreadId,
        event: RunEventKind,
        url: impl Into<String>,
        payload: impl Into<String>,
        next_attempt_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            delivery_id,
            run_id,
            thread_id,
            event,
            url: url.into(),
            payload: payload.into(),
            attempt: 0,
            max_attempts: default_max_attempts(),
            status: WebhookDeliveryStatus::Pending,
            next_attempt_at,
            last_response_status: None,
            last_error: None,
            created_at: now,
            updated_at: now,
            schema_version: WEBHOOK_DELIVERY_SCHEMA_VERSION.to_string(),
        }
    }

    /// Override the default (5) retry ceiling (D-43).
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Override the initial status (test/fixture convenience -- production
    /// code should route status changes through
    /// `WebhookDeliveryRepositoryPort::record_attempt`).
    pub fn with_status(mut self, status: WebhookDeliveryStatus) -> Self {
        self.status = status;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_delivery() -> WebhookDelivery {
        WebhookDelivery::new(
            WebhookDeliveryId::new_v7(),
            RunId::new_v7(),
            ThreadId::new("thread-1").unwrap(),
            RunEventKind::Completed,
            "https://example.com/hook",
            r#"{"run_id":"r1"}"#,
            Utc::now(),
        )
    }

    #[test]
    fn webhook_delivery_id_new_v7_round_trips_through_parse() {
        let id = WebhookDeliveryId::new_v7();
        let parsed = WebhookDeliveryId::parse(id.as_str().to_string()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn webhook_delivery_id_parse_rejects_empty_and_invalid() {
        assert_eq!(
            WebhookDeliveryId::parse(""),
            Err(WebhookDeliveryIdError::Empty)
        );
        assert!(matches!(
            WebhookDeliveryId::parse("not-a-uuid"),
            Err(WebhookDeliveryIdError::InvalidUuid { .. })
        ));
    }

    #[test]
    fn webhook_delivery_status_serde_uses_snake_case() {
        let cases = [
            (WebhookDeliveryStatus::Pending, "pending"),
            (WebhookDeliveryStatus::InFlight, "in_flight"),
            (WebhookDeliveryStatus::Delivered, "delivered"),
            (WebhookDeliveryStatus::Retrying, "retrying"),
            (WebhookDeliveryStatus::Dead, "dead"),
        ];
        for (status, expected) in cases {
            assert_eq!(status.as_str(), expected);
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let restored: WebhookDeliveryStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.as_str(), expected);
        }
    }

    #[test]
    fn webhook_delivery_new_defaults_pending_zero_attempts_five_max() {
        let delivery = sample_delivery();
        assert_eq!(delivery.attempt, 0);
        assert_eq!(delivery.max_attempts, 5);
        assert!(matches!(delivery.status, WebhookDeliveryStatus::Pending));
        assert_eq!(delivery.schema_version, WEBHOOK_DELIVERY_SCHEMA_VERSION);
        assert!(delivery.last_response_status.is_none());
        assert!(delivery.last_error.is_none());
    }

    #[test]
    fn webhook_delivery_round_trips_through_serde_json() {
        let delivery = sample_delivery();
        let json = serde_json::to_string(&delivery).unwrap();
        let restored: WebhookDelivery = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.delivery_id, delivery.delivery_id);
        assert_eq!(restored.run_id, delivery.run_id);
        assert_eq!(restored.url, delivery.url);
        assert_eq!(restored.payload, delivery.payload);
        assert_eq!(restored.schema_version, WEBHOOK_DELIVERY_SCHEMA_VERSION);
    }

    #[test]
    fn webhook_delivery_builder_with_methods_set_expected_fields() {
        let delivery = sample_delivery()
            .with_max_attempts(3)
            .with_status(WebhookDeliveryStatus::Retrying);
        assert_eq!(delivery.max_attempts, 3);
        assert!(matches!(delivery.status, WebhookDeliveryStatus::Retrying));
    }

    #[test]
    fn webhook_attempt_outcome_variants_construct() {
        let delivered = WebhookAttemptOutcome::Delivered;
        let retrying = WebhookAttemptOutcome::Retrying {
            next_attempt_at: Utc::now(),
        };
        let dead = WebhookAttemptOutcome::Dead;
        assert_eq!(delivered, WebhookAttemptOutcome::Delivered);
        assert_ne!(retrying, dead);
    }

    #[test]
    fn webhook_attempt_result_carries_response_status_and_error() {
        let result = WebhookAttemptResult {
            outcome: WebhookAttemptOutcome::Dead,
            response_status: Some(404),
            error: Some("not found".to_string()),
        };
        assert_eq!(result.response_status, Some(404));
        assert_eq!(result.error.as_deref(), Some("not found"));
    }
}
