//! `WebhookDeliveryService` -- the claim-then-send drain loop (D-40, D-43).
//!
//! [`WebhookDeliveryService::run_once`] claims due deliveries
//! (`WebhookDeliveryRepositoryPort::claim_due`), re-applies the SSRF guard
//! at send time (D-42 -- a URL can resolve differently between enqueue and
//! send), signs the delivery's own stored `payload` bytes (never
//! re-serialized, D-41), sends the request through the no-redirect client,
//! and records the outcome. Delivery is entirely decoupled from run status:
//! a delivery failure is persisted on the delivery row and never touches
//! the run.
//!
//! # The injected clock
//!
//! Mirrors `schedule::service`'s own documented rationale:
//! `tokio::time::pause` freezes tokio TIMERS, not `chrono::Utc::now()` --
//! every timestamp this service persists or compares against comes from
//! [`WebhookDeliveryOptions::now`], never a direct `Utc::now()` call.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;

use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::webhook::{
    WebhookAttemptOutcome, WebhookAttemptResult, WebhookDelivery,
};
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

use super::client::build_webhook_client;
use super::signature::WEBHOOK_SIGNATURE_HEADER;
use super::ssrf::SsrfGuard;

/// How many due deliveries [`WebhookDeliveryService::run_once`] claims per
/// call.
const DEFAULT_BATCH: usize = 50;

/// The character budget a redacted delivery-failure diagnostic is bounded
/// to before being persisted as `last_error` (D-43).
const LAST_ERROR_CHAR_BUDGET: usize = 256;

/// Construction knobs for [`WebhookDeliveryService`].
pub struct WebhookDeliveryOptions {
    /// The retry ceiling stamped on every delivery this service's own
    /// callers enqueue with the default (`WebhookDelivery::new` itself
    /// defaults to 5) -- read by `record_attempt`'s Retrying-vs-Dead
    /// decision from each delivery's OWN `max_attempts` field, not this
    /// value; kept here for callers that want one place to configure the
    /// default before enqueueing.
    pub max_attempts: u32,
    /// The per-request HTTP timeout.
    pub timeout: Duration,
    /// How many due deliveries are claimed per drain iteration.
    pub batch: usize,
    /// How long an idle drain iteration (nothing claimed) sleeps before
    /// polling again.
    pub poll_interval: Duration,
    /// Whether the send-time SSRF guard allows private/loopback/link-local
    /// addresses (D-42). `false` in production; a deployment opts in only
    /// for internal-network testing.
    pub allow_private: bool,
    /// The clock every persisted/compared timestamp in this service comes
    /// from -- never a direct `Utc::now()` call. Production wiring:
    /// `Arc::new(chrono::Utc::now)`.
    pub now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}

impl Default for WebhookDeliveryOptions {
    /// 5 max attempts, a 10s per-request timeout, a batch of 50, a 5s idle
    /// poll interval, `allow_private: false`, and the real system clock.
    fn default() -> Self {
        Self {
            max_attempts: 5,
            timeout: Duration::from_secs(10),
            batch: DEFAULT_BATCH,
            poll_interval: Duration::from_secs(5),
            allow_private: false,
            now: Arc::new(Utc::now),
        }
    }
}

/// The bounded-retry backoff schedule (D-43): `min(60s, 1s * 2^(attempt -
/// 1))` for the delivery's `attempt` count AFTER this failure was recorded
/// (1-indexed) -- the delay before the NEXT attempt. Pure and unit-tested
/// independent of any clock or repository.
pub fn backoff_for(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(6);
    let seconds = 2u64.saturating_pow(exponent).min(60);
    Duration::from_secs(seconds)
}

/// Drains a durable webhook delivery queue: claim, SSRF-check, sign, send,
/// record (D-40..D-43).
pub struct WebhookDeliveryService {
    deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
    runs: Arc<dyn RunRepositoryPort>,
    guard: SsrfGuard,
    client: reqwest::Client,
    options: WebhookDeliveryOptions,
}

impl WebhookDeliveryService {
    /// Construct a service over the given delivery repository and run
    /// repository (the source of each run's webhook signing value at send
    /// time), building its own no-redirect HTTP client and SSRF guard from
    /// `options`.
    ///
    /// # Errors
    ///
    /// Returns the underlying `reqwest::Error` if the HTTP client fails to
    /// build.
    pub fn new(
        deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
        runs: Arc<dyn RunRepositoryPort>,
        options: WebhookDeliveryOptions,
    ) -> Result<Self, reqwest::Error> {
        let client = build_webhook_client(options.timeout)?;
        let guard = SsrfGuard::new(options.allow_private);
        Ok(Self {
            deliveries,
            runs,
            guard,
            client,
            options,
        })
    }

    /// Override the SSRF guard -- tests inject a stubbed resolver so the
    /// send-time check does not depend on real DNS.
    pub fn with_guard(mut self, guard: SsrfGuard) -> Self {
        self.guard = guard;
        self
    }

    /// Claim and process up to `options.batch` due deliveries. Returns how
    /// many were claimed (0 means the queue had nothing due).
    pub async fn run_once(&self, now: DateTime<Utc>) -> usize {
        let claimed = match self
            .deliveries
            .claim_due(now, self.options.batch as u32)
            .await
        {
            Ok(claimed) => claimed,
            Err(error) => {
                log::warn!("webhook delivery service: claim_due failed: {error}");
                return 0;
            }
        };
        let count = claimed.len();
        for delivery in claimed {
            self.process(delivery).await;
        }
        count
    }

    async fn process(&self, delivery: WebhookDelivery) {
        let delivery_id = delivery.delivery_id.clone();

        // D-42: the send-time half of the SSRF guard. A URL that passed
        // the write-time check can still resolve differently by send time
        // (DNS change) -- re-checked here, against the SAME guard
        // configuration, before any bytes leave this process.
        if let Err(rejection) = self.guard.check_url(&delivery.url).await {
            self.finish(
                &delivery_id,
                WebhookAttemptResult {
                    outcome: WebhookAttemptOutcome::Dead,
                    response_status: None,
                    error: Some(bounded_error(&format!("SSRF check failed: {rejection}"))),
                },
            )
            .await;
            return;
        }

        // The signing value lives on the RUN's own WebhookSpec, never on
        // the delivery row (prohibition P1) -- read it fresh at send time.
        let signing_key = match self.runs.get(&delivery.run_id).await {
            Ok(Some(run)) => run
                .webhook
                .as_ref()
                .and_then(|webhook| webhook.secret.clone())
                .unwrap_or_default(),
            Ok(None) => String::new(),
            Err(error) => {
                log::warn!(
                    "webhook delivery service: failed to load run {} for delivery {delivery_id}: {error}",
                    delivery.run_id
                );
                String::new()
            }
        };

        let signature = super::signature::sign_webhook_body(
            signing_key.as_bytes(),
            delivery.payload.as_bytes(),
        );
        let new_attempt = delivery.attempt + 1;

        let send_result = self
            .client
            .post(&delivery.url)
            .header("Content-Type", "application/json")
            .header("X-Paladin-Event", event_wire_name(delivery.event))
            .header("X-Paladin-Delivery", delivery.delivery_id.as_str())
            .header(WEBHOOK_SIGNATURE_HEADER, signature)
            .body(delivery.payload.clone())
            .send()
            .await;

        let result = match send_result {
            Ok(response) => {
                let status = response.status().as_u16();
                if (200..300).contains(&status) {
                    WebhookAttemptResult {
                        outcome: WebhookAttemptOutcome::Delivered,
                        response_status: Some(status),
                        error: None,
                    }
                } else {
                    let body = response.text().await.unwrap_or_default();
                    let error = Some(bounded_error(&format!("http {status}: {body}")));
                    if (500..600).contains(&status) {
                        self.retry_or_dead(&delivery, new_attempt, Some(status), error)
                    } else {
                        // 3xx (never followed -- Policy::none()) and 4xx
                        // dead-letter immediately (D-43).
                        WebhookAttemptResult {
                            outcome: WebhookAttemptOutcome::Dead,
                            response_status: Some(status),
                            error,
                        }
                    }
                }
            }
            Err(error) => {
                // Timeout or connect error -- no response was ever
                // received, so no response_status.
                let text = Some(bounded_error(&error.to_string()));
                self.retry_or_dead(&delivery, new_attempt, None, text)
            }
        };

        self.finish(&delivery_id, result).await;
    }

    /// Decide Retrying-vs-Dead for a `5xx`/timeout/connect-error attempt
    /// (D-43): retry while `new_attempt < delivery.max_attempts`, using
    /// [`backoff_for`] against THIS service's own injected clock; dead-
    /// letter once the budget is exhausted.
    fn retry_or_dead(
        &self,
        delivery: &WebhookDelivery,
        new_attempt: u32,
        response_status: Option<u16>,
        error: Option<String>,
    ) -> WebhookAttemptResult {
        if new_attempt < delivery.max_attempts {
            let delay = chrono::Duration::from_std(backoff_for(new_attempt))
                .unwrap_or_else(|_| chrono::Duration::zero());
            WebhookAttemptResult {
                outcome: WebhookAttemptOutcome::Retrying {
                    next_attempt_at: (self.options.now)() + delay,
                },
                response_status,
                error,
            }
        } else {
            WebhookAttemptResult {
                outcome: WebhookAttemptOutcome::Dead,
                response_status,
                error,
            }
        }
    }

    async fn finish(
        &self,
        delivery_id: &paladin_core::platform::container::webhook::WebhookDeliveryId,
        result: WebhookAttemptResult,
    ) {
        if let Err(error) = self.deliveries.record_attempt(delivery_id, result).await {
            log::warn!(
                "webhook delivery service: record_attempt failed for {delivery_id}: {error}"
            );
        }
    }

    /// Spawn a background task calling [`WebhookDeliveryService::run_once`]
    /// repeatedly, registered with `coordinator` (D-13's draining
    /// precedent): on shutdown the task stops once its current iteration
    /// returns and drops its `RunGuard`. Idles at `options.poll_interval`
    /// whenever a claim comes back empty, mirroring `RunWorkerPool::spawn`.
    pub fn spawn(self: Arc<Self>, coordinator: &ShutdownCoordinator) -> JoinHandle<()> {
        let (child_token, guard) = coordinator.register();
        let poll_interval = self.options.poll_interval;

        tokio::spawn(async move {
            let _guard = guard;
            loop {
                if child_token.is_cancelled() {
                    break;
                }
                let now = (self.options.now)();
                let processed = self.run_once(now).await;
                if processed == 0 {
                    tokio::select! {
                        _ = tokio::time::sleep(poll_interval) => {}
                        _ = child_token.cancelled() => break,
                    }
                }
            }
        })
    }
}

/// `RunEventKind` (paladin-core) carries no `as_str` of its own -- this
/// mirrors `paladin-storage`'s `event_to_str` (`webhook/sqlite.rs`), the
/// wire value for the `X-Paladin-Event` header.
fn event_wire_name(event: paladin_core::platform::container::run::RunEventKind) -> &'static str {
    use paladin_core::platform::container::run::RunEventKind;
    match event {
        RunEventKind::AwaitingInput => "awaiting_input",
        RunEventKind::Completed => "completed",
        RunEventKind::Failed => "failed",
        RunEventKind::Halted => "halted",
        RunEventKind::Cancelled => "cancelled",
    }
}

/// Redact-then-truncate a delivery-failure diagnostic before it is
/// persisted as `last_error` (D-43, `security.instructions.md`'s
/// redact-before-truncate rule): a captured response body or error message
/// may itself carry a credential (an upstream proxy's own header echoed
/// back, a leaked key in an error page) that must never survive into a
/// persisted row.
fn bounded_error(text: &str) -> String {
    let redacted = paladin_llm::redaction::redact_secret_patterns(text);
    paladin_llm::redaction::bounded_excerpt(&redacted, LAST_ERROR_CHAR_BUDGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_for_doubles_and_caps_at_sixty_seconds() {
        assert_eq!(backoff_for(1), Duration::from_secs(1));
        assert_eq!(backoff_for(2), Duration::from_secs(2));
        assert_eq!(backoff_for(3), Duration::from_secs(4));
        assert_eq!(backoff_for(4), Duration::from_secs(8));
        assert_eq!(backoff_for(5), Duration::from_secs(16));
        for attempt in 1..=5 {
            assert!(backoff_for(attempt) <= Duration::from_secs(60));
        }
        // Large attempts saturate at the 60s cap, never overflow or panic.
        assert_eq!(backoff_for(100), Duration::from_secs(60));
    }

    #[test]
    fn webhook_delivery_options_default_matches_documented_values() {
        let options = WebhookDeliveryOptions::default();
        assert_eq!(options.max_attempts, 5);
        assert_eq!(options.timeout, Duration::from_secs(10));
        assert_eq!(options.batch, 50);
        assert!(!options.allow_private);
    }

    #[test]
    fn bounded_error_redacts_and_truncates() {
        let long = "x".repeat(1000);
        let text = format!("Bearer sk-livekey-abcdef0123456789 {long}");
        let excerpt = bounded_error(&text);
        assert!(!excerpt.contains("abcdef0123456789"));
        assert!(excerpt.chars().count() <= LAST_ERROR_CHAR_BUDGET + 40);
    }
}
