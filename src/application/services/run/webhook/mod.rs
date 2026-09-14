//! Durable webhook delivery (PLAT-FR-14/15, D-40..D-43): an SSRF guard
//! applied at write AND send time, HMAC-SHA256 signing over the exact bytes
//! sent, a no-redirect HTTP client, and a bounded-retry drain service
//! (`WebhookDeliveryService`) under an injectable clock -- the most
//! security-relevant slice of Phase 27: an outbound HTTP client carrying a
//! credential-shaped header to a caller-chosen URL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use paladin_core::platform::container::parley::ParleyRequest;
use paladin_core::platform::container::run::{RunEventKind, RunId, RunStatus};
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
/// event, timestamp, attempt, parleys? }`. Serialized ONCE per delivery,
/// stored verbatim on the `WebhookDelivery` row, and signed/sent from that
/// SAME buffer -- never re-serialized (D-41).
///
/// This exact key set is a prohibition boundary (T-27-13-03): no run
/// input, no Battlefield state, and no HMAC signing value ever appear here.
///
/// **(WR-02) Documented carve-out: a legacy `Runnable::Agent` run never
/// produces this payload.** `RunWorkerPool::run_agent` (`worker.rs`)
/// completes or fails a code-registered agent run without ever calling
/// `webhook_deliveries.enqueue`, so PLAT-FR-14's delivery rule above
/// applies only to `Runnable::Workflow` runs (stored `WarGraphDoc`
/// assistants). Tracked as ledger row 31 and pinned by
/// `agent_kind_run_with_a_webhook_enqueues_no_delivery` in
/// `worker_tests.rs`.
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
        };
        let value = serde_json::to_value(&payload).unwrap();
        assert!(value.as_object().unwrap().contains_key("parleys"));

        payload.parleys = None;
        let value = serde_json::to_value(&payload).unwrap();
        assert!(!value.as_object().unwrap().contains_key("parleys"));
    }
}
