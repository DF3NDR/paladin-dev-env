//! Durable webhook delivery (PLAT-FR-14/15, D-40..D-43): an SSRF guard
//! applied at write AND send time, HMAC-SHA256 signing over the exact bytes
//! sent, a no-redirect HTTP client, and a bounded-retry drain service
//! (`WebhookDeliveryService`) under an injectable clock -- the most
//! security-relevant slice of Phase 27: an outbound HTTP client carrying a
//! credential-shaped header to a caller-chosen URL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::allowance::{
    AllowanceLimitKind, AllowanceNotice, AllowanceScopeKind,
};
use paladin_core::platform::container::parley::ParleyRequest;
use paladin_core::platform::container::run::{RunEventKind, RunId, RunStatus};
use paladin_core::platform::container::treasury_ledger::format_cost;
use paladin_core::platform::container::waypoint::ThreadId;

/// The SSRF guard applied at write time and send time (D-42).
pub mod ssrf;

/// HMAC-SHA256 body signing over the exact bytes sent (D-41).
pub mod signature;

/// The no-redirect webhook HTTP client (D-42).
pub mod client;

/// `WebhookDeliveryService` -- the claim-then-send drain loop (D-40, D-43).
pub mod service;

pub use client::build_webhook_client;
pub use service::{WebhookDeliveryOptions, WebhookDeliveryService, backoff_for};
pub use signature::{WEBHOOK_SIGNATURE_HEADER, sign_webhook_body};
pub use ssrf::{SsrfGuard, SsrfRejection};

/// The assistant reference a [`WebhookPayload`] carries -- a minimal,
/// wire-stable subset of `AssistantRef` (PRD 06 PLAT-FR-14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayloadAssistant {
    /// The assistant's identity.
    pub assistant_id: String,
    /// The specific version resolved at submit time.
    pub version: u32,
}

/// The webhook wire payload (PRD 06 PLAT-FR-14; this phase adds `attempt`,
/// D-23, and freezes the set): `{ run_id, thread_id, assistant, status,
/// event, timestamp, attempt, parleys?, halt_reason? }`. Serialized ONCE per
/// delivery, stored verbatim on the `WebhookDelivery` row, and signed/sent
/// from that SAME buffer -- never re-serialized (D-41).
///
/// This exact key set is a prohibition boundary (T-27-13-03): no run
/// input, no Battlefield state, and no HMAC signing value ever appear here.
/// `halt_reason` (Phase 42, D-19) is an additive optional key under that
/// discipline: it is omitted from every event but a `halted` one whose run
/// halted with a typed reason, so every other payload is byte-identical to
/// what it was before the key existed.
///
/// The payload is produced for stored-workflow and code-registered-agent
/// runs alike (PLAT-08).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// The run this event reports on.
    pub run_id: RunId,
    /// The thread the run executed against.
    pub thread_id: ThreadId,
    /// The assistant reference, frozen at submit time.
    pub assistant: WebhookPayloadAssistant,
    /// The run's status at the time this event was recorded.
    pub status: RunStatus,
    /// Which lifecycle event this delivery reports.
    pub event: RunEventKind,
    /// When this event was recorded.
    pub timestamp: DateTime<Utc>,
    /// This delivery's attempt number (D-23).
    pub attempt: u32,
    /// The pending parley requests, present only for an `AwaitingInput`
    /// event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parleys: Option<Vec<ParleyRequest>>,
    /// Why the run halted, present only on a `halted` event whose run halted with a typed
    /// reason (Phase 42, D-19, ALLOW-03). Equal to `HaltReason::wire_json()` -- the same
    /// object `GET /v1/runs/{id}` serves and, for an exhausted allowance, the figures of the
    /// `429 allowance_exhausted` `details` -- so the reason (`reason`, plus the halted
    /// ceiling's own `scope`, `kind`, `balance`, `ceiling`, `window_start`, `window_end`) can
    /// never drift between surfaces. Carries no run input, no API key value and no signing
    /// value; signing is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halt_reason: Option<serde_json::Value>,
}

/// The operator-level allowance notice wire payload (Phase 41, D-17, amended at the
/// 41-01 design checkpoint to carry `tenant_id` and `api_key_id`): `{ event, scope, kind,
/// balance, ceiling, window_start, window_end, warn_at, run_id, timestamp, tenant_id,
/// api_key_id }` -- twelve keys, always all present (`null` where a value does not apply).
/// Serialized ONCE per notice, stored verbatim on the `WebhookDelivery` row and signed and
/// sent from that SAME buffer, like [`WebhookPayload`] (D-41).
///
/// This exact key set is a prohibition boundary: no run input, no API key value and no
/// signing secret ever appear here. `api_key_id` is the key's configured NAME, never its
/// value (D-00g classes tenant ids and key names as log-safe). `balance` and `ceiling` are
/// `format_cost` strings (`"0.8000 USD"`); `run_id` is the admitting run, or `null` on the
/// HTTP agent execute path where no run row exists; `window_start` and `window_end` are
/// `null` for a lifetime ceiling.
///
/// # Examples
///
/// ```
/// use chrono::{TimeZone, Utc};
/// use paladin::application::services::run::webhook::AllowanceWarningPayload;
/// use paladin_core::platform::container::allowance::{
///     AllowanceLimitKind, AllowanceNotice, AllowanceScopeKind, AllowanceWarning,
/// };
/// use paladin_core::platform::container::cost::{Cost, CurrencyCode};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let usd = CurrencyCode::new("USD")?;
/// let at = Utc.with_ymd_and_hms(2026, 10, 3, 9, 0, 0).single().ok_or("bad instant")?;
/// let notice = AllowanceNotice {
///     notice_id: "n1".into(),
///     tenant_id: "acme".into(),
///     api_key_id: Some("svc-w".into()),
///     warning: AllowanceWarning {
///         scope_kind: AllowanceScopeKind::ApiKey,
///         limit_kind: AllowanceLimitKind::Lifetime,
///         balance: Cost::new(800_000_000, usd.clone()),
///         ceiling: Cost::new(1_000_000_000, usd),
///         window_start: None,
///         window_end: None,
///         warn_at: 80,
///     },
///     run_id: None,
///     recorded_at: at,
/// };
/// let payload = AllowanceWarningPayload::from_notice(&notice);
/// assert_eq!(payload.balance, "0.8000 USD");
/// assert_eq!(payload.tenant_id, "acme");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowanceWarningPayload {
    /// Always [`RunEventKind::AllowanceWarning`].
    pub event: RunEventKind,
    /// Which identity the ceiling is held against.
    pub scope: AllowanceScopeKind,
    /// Which kind of limit the ceiling is.
    pub kind: AllowanceLimitKind,
    /// The balance read when the warning fired, as a `format_cost` string.
    pub balance: String,
    /// The ceiling the balance is approaching, as a `format_cost` string.
    pub ceiling: String,
    /// The window's start (inclusive); `null` for a lifetime ceiling.
    pub window_start: Option<DateTime<Utc>>,
    /// The window's end (exclusive); `null` for a lifetime ceiling.
    pub window_end: Option<DateTime<Utc>>,
    /// The configured warn threshold, in whole percent of the ceiling.
    pub warn_at: u8,
    /// The admitting run; `null` on the HTTP agent execute path.
    pub run_id: Option<RunId>,
    /// The store instant the notice was recorded at.
    pub timestamp: DateTime<Utc>,
    /// The tenant the ceiling belongs to.
    pub tenant_id: String,
    /// The API key NAME for an API-key-scope notice (never a key value); `null` for a
    /// tenant-scope notice.
    pub api_key_id: Option<String>,
}

impl AllowanceWarningPayload {
    /// Build the payload for a won notice. `timestamp` is `notice.recorded_at`, the store
    /// instant, so a payload re-built from the same notice is identical.
    pub fn from_notice(notice: &AllowanceNotice) -> Self {
        let warning = &notice.warning;
        Self {
            event: RunEventKind::AllowanceWarning,
            scope: warning.scope_kind,
            kind: warning.limit_kind,
            balance: format_cost(&warning.balance),
            ceiling: format_cost(&warning.ceiling),
            window_start: warning.window_start,
            window_end: warning.window_end,
            warn_at: warning.warn_at,
            run_id: notice.run_id.clone(),
            timestamp: notice.recorded_at,
            tenant_id: notice.tenant_id.clone(),
            api_key_id: notice.api_key_id.clone(),
        }
    }
}

/// `webhook_retry_schedule`, `webhook_signature_verifies_on_receiver`,
/// `webhook_payload_has_no_secret_or_input` and the write-time submission
/// guard proofs (27-13 Task 2, D-40..D-43).
#[cfg(test)]
mod tests;

#[cfg(test)]
mod webhook_payload_tests {
    use super::*;

    #[test]
    fn webhook_payload_serializes_with_exactly_the_documented_keys() {
        let payload = WebhookPayload {
            run_id: RunId::new_v7(),
            thread_id: ThreadId::new("t1").unwrap(),
            assistant: WebhookPayloadAssistant {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            status: RunStatus::Completed,
            event: RunEventKind::Completed,
            timestamp: Utc::now(),
            attempt: 1,
            parleys: None,
            halt_reason: None,
        };
        let value = serde_json::to_value(&payload).unwrap();
        let object = value.as_object().unwrap();
        let mut keys: Vec<&str> = object.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "assistant",
                "attempt",
                "event",
                "run_id",
                "status",
                "thread_id",
                "timestamp",
            ]
        );
    }

    #[test]
    fn webhook_payload_includes_halt_reason_only_when_present() {
        use paladin_core::platform::container::allowance::HaltReason;

        let mut payload = WebhookPayload {
            run_id: RunId::new_v7(),
            thread_id: ThreadId::new("t1").unwrap(),
            assistant: WebhookPayloadAssistant {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            status: RunStatus::Halted,
            event: RunEventKind::Halted,
            timestamp: Utc::now(),
            attempt: 1,
            parleys: None,
            halt_reason: Some(HaltReason::LedgerUnavailable.wire_json()),
        };
        let value = serde_json::to_value(&payload).unwrap();
        assert_eq!(
            value["halt_reason"],
            serde_json::json!({"reason": "ledger_unavailable"})
        );

        // Absent, the key is omitted entirely: every other event's bytes are unchanged.
        payload.halt_reason = None;
        let value = serde_json::to_value(&payload).unwrap();
        assert!(!value.as_object().unwrap().contains_key("halt_reason"));
    }

    #[test]
    fn webhook_payload_includes_parleys_only_when_present() {
        let mut payload = WebhookPayload {
            run_id: RunId::new_v7(),
            thread_id: ThreadId::new("t1").unwrap(),
            assistant: WebhookPayloadAssistant {
                assistant_id: "a1".to_string(),
                version: 1,
            },
            status: RunStatus::AwaitingInput,
            event: RunEventKind::AwaitingInput,
            timestamp: Utc::now(),
            attempt: 1,
            parleys: Some(vec![]),
            halt_reason: None,
        };
        let value = serde_json::to_value(&payload).unwrap();
        assert!(value.as_object().unwrap().contains_key("parleys"));

        payload.parleys = None;
        let value = serde_json::to_value(&payload).unwrap();
        assert!(!value.as_object().unwrap().contains_key("parleys"));
    }
}

#[cfg(test)]
mod allowance_warning_payload_tests {
    use super::*;
    use paladin_core::platform::container::allowance::AllowanceWarning;
    use paladin_core::platform::container::cost::{Cost, CurrencyCode};

    fn notice(run_id: Option<RunId>, api_key_id: Option<&str>) -> AllowanceNotice {
        let usd = CurrencyCode::new("USD").unwrap();
        AllowanceNotice {
            notice_id: "n1".to_string(),
            tenant_id: "acme".to_string(),
            api_key_id: api_key_id.map(str::to_string),
            warning: AllowanceWarning {
                scope_kind: if api_key_id.is_some() {
                    AllowanceScopeKind::ApiKey
                } else {
                    AllowanceScopeKind::Tenant
                },
                limit_kind: AllowanceLimitKind::Window,
                balance: Cost::new(800_000_000, usd.clone()),
                ceiling: Cost::new(1_000_000_000, usd),
                window_start: Some(Utc::now()),
                window_end: Some(Utc::now()),
                warn_at: 80,
            },
            run_id,
            recorded_at: Utc::now(),
        }
    }

    #[test]
    fn allowance_warning_payload_serializes_with_exactly_the_documented_keys() {
        // Twelve keys: the D-17 ten plus `tenant_id` and `api_key_id` (the 41-01
        // checkpoint's option-b amendment). All are always present -- `null` where a
        // value does not apply.
        for payload in [
            AllowanceWarningPayload::from_notice(&notice(Some(RunId::new_v7()), Some("svc-w"))),
            AllowanceWarningPayload::from_notice(&notice(None, None)),
        ] {
            let value = serde_json::to_value(&payload).unwrap();
            let mut keys: Vec<&str> = value
                .as_object()
                .unwrap()
                .keys()
                .map(|k| k.as_str())
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                vec![
                    "api_key_id",
                    "balance",
                    "ceiling",
                    "event",
                    "kind",
                    "run_id",
                    "scope",
                    "tenant_id",
                    "timestamp",
                    "warn_at",
                    "window_end",
                    "window_start",
                ]
            );
            assert_eq!(value["event"], "allowance_warning");
            assert_eq!(value["balance"], "0.8000 USD");
            assert_eq!(value["ceiling"], "1.0000 USD");
        }
    }

    #[test]
    fn allowance_warning_payload_carries_the_admitting_run_or_null() {
        let run_id = RunId::new_v7();
        let with_run = AllowanceWarningPayload::from_notice(&notice(Some(run_id.clone()), None));
        assert_eq!(with_run.run_id, Some(run_id));
        let value = serde_json::to_value(AllowanceWarningPayload::from_notice(&notice(
            None,
            Some("svc-w"),
        )))
        .unwrap();
        assert!(value["run_id"].is_null());
        assert_eq!(value["api_key_id"], "svc-w");
        assert_eq!(value["scope"], "api_key");
        assert_eq!(value["kind"], "window");
    }

    #[test]
    fn allowance_warning_payload_has_no_secret_or_input() {
        let json = serde_json::to_string(&AllowanceWarningPayload::from_notice(&notice(
            Some(RunId::new_v7()),
            Some("svc-w"),
        )))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        for key in value.as_object().unwrap().keys() {
            for forbidden in [
                "input",
                "secret",
                "key",
                "token",
                "password",
                "authorization",
            ] {
                // `api_key_id` is the key's NAME and is the one documented exception.
                if key == "api_key_id" {
                    continue;
                }
                assert!(
                    !key.contains(forbidden),
                    "payload key {key:?} must not name {forbidden:?}"
                );
            }
        }
    }
}
