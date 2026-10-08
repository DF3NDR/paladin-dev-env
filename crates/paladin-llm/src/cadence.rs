//! Cadence decorator -- `CadenceLlmAdapter` (PACE-02, D-01, D-07, D-08; Phase 43)
//!
//! [`CadenceLlmAdapter`] is a stateless `Arc<dyn LlmPort>` decorator that paces calls to one
//! provider and model after that provider answered `429 Too Many Requests`. It holds no pacing
//! state of its own: the gate lives behind the shared
//! [`CadencePort`](paladin_ports::output::cadence_port::CadencePort), so the same provider
//! wrapped in two decorators (the agent host and a fallback hop, say) is paced once, not twice.
//!
//! ## Placement
//!
//! `Pricing(Cadence(provider))`. Pricing stays outermost so it still prices the response that was
//! actually served (ADR-0052); Cadence sits directly on each provider so its key is that
//! provider's own name. A [`FallbackLlmAdapter`](crate::fallback::FallbackLlmAdapter) chain is
//! paced per hop (plan 43-06), so [`with_cadence`] passes a port reporting
//! [`FALLBACK_PROVIDER_NAME`] through unchanged -- gating the chain under `("fallback", model)`
//! would double-gate it.
//!
//! ## Gate, never retry (D-01)
//!
//! The decorator waits on the gate *before* it delegates, and records the outcome *after*. It
//! never re-issues a request and never sleeps on the call that received the 429: that call
//! returns the inner error unchanged, and `RetryPolicy` and `FallbackLlmAdapter` remain the only
//! retry owners, so attempt counts above the decorator are never multiplied. The 429 only slows
//! the *next* call.
//!
//! Waits nest as `max(retry back-off, gate)`, not as a sum: an agent loop's `RetryPolicy` sleeps
//! its own back-off after a failure and then calls again, hitting the gate -- so an attempt-count
//! test must not assume a wall-clock sum.
//!
//! ## Bookkeeping never fails a call
//!
//! A port error is logged under [`CADENCE_LOG_TARGET`] at `warn` (once per call) and the call
//! proceeds: the decorator never fails a request because of its own bookkeeping.
//!
//! Log lines carry the provider name, model and durations only -- never request bodies,
//! credentials or header values.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::Stream;
use paladin_ports::output::cadence_port::{
    CADENCE_LOG_TARGET, CadenceError, CadenceKey, CadencePort,
};
use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

use crate::fallback::FALLBACK_PROVIDER_NAME;

/// The operator-tunable bounds that travel with a [`CadencePort`] (D-10).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use paladin_llm::cadence::CadenceSettings;
///
/// let settings = CadenceSettings::default();
/// assert_eq!(settings.max_wait(), Duration::from_secs(300));
/// assert_eq!(settings.max_backoff(), Duration::from_secs(30));
/// assert_eq!(settings.fallback_pace_budget(), Duration::from_secs(60));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CadenceSettings {
    max_wait: Duration,
    max_backoff: Duration,
    fallback_pace_budget: Duration,
}

impl CadenceSettings {
    /// Build settings.
    pub fn new(max_wait: Duration, max_backoff: Duration, fallback_pace_budget: Duration) -> Self {
        Self {
            max_wait,
            max_backoff,
            fallback_pace_budget,
        }
    }

    /// The longest a single call waits on a gate before surfacing a rate limit (D-06).
    pub fn max_wait(&self) -> Duration {
        self.max_wait
    }

    /// The largest delay-less gate (mirrors the policy's cap).
    pub fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    /// How long a fallback hop is paced before the chain moves on (D-03).
    pub fn fallback_pace_budget(&self) -> Duration {
        self.fallback_pace_budget
    }
}

impl Default for CadenceSettings {
    /// 300 s / 30 s / 60 s (D-10).
    fn default() -> Self {
        Self {
            max_wait: Duration::from_secs(300),
            max_backoff: Duration::from_secs(30),
            fallback_pace_budget: Duration::from_secs(60),
        }
    }
}

/// A [`CadencePort`] together with its [`CadenceSettings`] -- the one handle every composition
/// root shares so in-process pacing is process-wide.
#[derive(Clone)]
pub struct CadenceWiring {
    port: Arc<dyn CadencePort>,
    settings: CadenceSettings,
}

impl CadenceWiring {
    /// Bundle a port with its settings.
    pub fn new(port: Arc<dyn CadencePort>, settings: CadenceSettings) -> Self {
        Self { port, settings }
    }

    /// The shared pacing port.
    pub fn port(&self) -> &Arc<dyn CadencePort> {
        &self.port
    }

    /// The settings that travel with the port.
    pub fn settings(&self) -> &CadenceSettings {
        &self.settings
    }
}

impl fmt::Debug for CadenceWiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CadenceWiring")
            .field("settings", &self.settings)
            .finish()
    }
}

/// Decorates an `Arc<dyn LlmPort>` with cross-call rate pacing (D-01).
///
/// Construct through [`with_cadence`], which installs nothing when pacing is off or when the
/// inner port is a fallback chain. `Clone` is cheap: both fields are `Arc`-backed.
#[derive(Clone)]
pub struct CadenceLlmAdapter {
    inner: Arc<dyn LlmPort>,
    wiring: CadenceWiring,
}

impl fmt::Debug for CadenceLlmAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CadenceLlmAdapter")
            .field("inner_provider", &self.inner.get_provider_name())
            .finish()
    }
}

impl CadenceLlmAdapter {
    /// Wait until the key's gate is clear, re-checking after every sleep because a concurrent
    /// 429 may have extended it. Returns the streak read at the moment the gate cleared (zero
    /// when the port failed), and whether the port misbehaved.
    async fn wait_for_gate(&self, key: &CadenceKey, warned: &mut bool) -> u32 {
        loop {
            match self.wiring.port().gate(key).await {
                Ok(reading) if reading.is_clear() => return reading.streak(),
                Ok(reading) => {
                    log::trace!(
                        target: CADENCE_LOG_TARGET,
                        "provider {} model {} is gated; waiting {:?}",
                        key.provider(),
                        key.model(),
                        reading.wait()
                    );
                    tokio::time::sleep(reading.wait()).await;
                }
                Err(err) => {
                    Self::warn_port_error("gate", key, &err, warned);
                    return 0;
                }
            }
        }
    }

    /// Record the outcome of the delegated call. Never fails the call.
    async fn note_outcome<T>(
        &self,
        key: &CadenceKey,
        pre_send_streak: u32,
        outcome: &Result<T, LlmError>,
        warned: &mut bool,
    ) {
        match outcome {
            Ok(_) if pre_send_streak > 0 => {
                if let Err(err) = self.wiring.port().record_success(key).await {
                    Self::warn_port_error("record_success", key, &err, warned);
                }
            }
            Err(LlmError::RateLimitExceeded) => {
                if let Err(err) = self.wiring.port().record_rate_limited(key, None).await {
                    Self::warn_port_error("record_rate_limited", key, &err, warned);
                }
            }
            _ => {}
        }
    }

    /// Log a port failure at most once per call.
    fn warn_port_error(operation: &str, key: &CadenceKey, err: &CadenceError, warned: &mut bool) {
        if std::mem::replace(warned, true) {
            return;
        }
        log::warn!(
            target: CADENCE_LOG_TARGET,
            "cadence port {operation} failed for provider {} model {}: {err}; \
             continuing without pacing for this call",
            key.provider(),
            key.model()
        );
    }
}

/// Wrap `inner` with rate pacing from `cadence`, UNLESS pacing is off (`None`) or `inner` is a
/// fallback chain (it reports [`FALLBACK_PROVIDER_NAME`]; each hop is paced instead, plan 43-06)
/// -- in both cases `inner` comes back unchanged (`Arc::ptr_eq` holds), so nothing is installed.
pub fn with_cadence(inner: Arc<dyn LlmPort>, cadence: Option<&CadenceWiring>) -> Arc<dyn LlmPort> {
    match cadence {
        Some(wiring) if inner.get_provider_name() != FALLBACK_PROVIDER_NAME => {
            Arc::new(CadenceLlmAdapter {
                inner,
                wiring: wiring.clone(),
            })
        }
        _ => inner,
    }
}

#[async_trait]
impl LlmPort for CadenceLlmAdapter {
    /// Waits out the key's gate, delegates exactly once, then records the outcome. A 429 is
    /// recorded and returned UNCHANGED, with no sleep and no second attempt (D-01); any other
    /// error passes through untouched.
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let key = CadenceKey::new(self.inner.get_provider_name(), &request.model);
        let mut warned = false;
        let streak = self.wait_for_gate(&key, &mut warned).await;
        let outcome = self.inner.generate(request).await;
        self.note_outcome(&key, streak, &outcome, &mut warned).await;
        outcome
    }

    /// Delegates unpaced in this plan; stream gating arrives in plan 43-05.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        self.inner.generate_stream(request).await
    }

    /// Unchanged -- delegates to `inner`.
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        self.inner.validate_model(model).await
    }

    /// Unchanged -- delegates to `inner`.
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.get_available_models().await
    }

    /// Unchanged -- delegates to `inner`.
    fn get_provider_name(&self) -> &'static str {
        self.inner.get_provider_name()
    }

    /// Unchanged -- delegates to `inner`.
    fn get_capabilities(&self) -> ProviderCapabilities {
        self.inner.get_capabilities()
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::fallback::FallbackLlmAdapter;
    use crate::mock::{MockLlmAdapter, MockScriptEntry};
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::cadence_port::{CadencePolicy, GateReading};
    use paladin_storage::cadence::InMemoryCadence;

    const BASE: Duration = Duration::from_millis(500);

    fn request(model: &str) -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "quest".to_string(),
            context: None,
        }))
        .expect("prompt");
        LlmRequest::new(model, prompt)
    }

    fn wiring() -> CadenceWiring {
        let policy = CadencePolicy::new(BASE, Duration::from_secs(30)).expect("policy");
        let port = InMemoryCadence::new(policy).with_jitter(|| 0.999);
        CadenceWiring::new(Arc::new(port), CadenceSettings::default())
    }

    fn rate_limited_then_ok() -> Arc<MockLlmAdapter> {
        Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![
                    MockScriptEntry::Error(LlmError::RateLimitExceeded),
                    MockScriptEntry::Text("ok".to_string()),
                ]),
        )
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_returns_unchanged_with_no_wait_and_one_inner_call() {
        let inner = rate_limited_then_ok();
        let paced = with_cadence(inner.clone(), Some(&wiring()));

        let start = tokio::time::Instant::now();
        let result = paced.generate(request("gpt-x")).await;

        assert!(matches!(result, Err(LlmError::RateLimitExceeded)));
        assert_eq!(
            start.elapsed(),
            Duration::ZERO,
            "the failing call never sleeps"
        );
        assert_eq!(inner.call_count(), 1, "the decorator never re-issues");
    }

    #[tokio::test(start_paused = true)]
    async fn the_next_call_waits_out_the_gate_then_succeeds() {
        let inner = rate_limited_then_ok();
        let paced = with_cadence(inner.clone(), Some(&wiring()));

        let _ = paced.generate(request("gpt-x")).await;
        let second_start = tokio::time::Instant::now();
        let response = paced.generate(request("gpt-x")).await.expect("second call");

        assert_eq!(response.content, "ok");
        assert!(
            second_start.elapsed() >= BASE,
            "second call waited only {:?}",
            second_start.elapsed()
        );
        assert_eq!(inner.call_count(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_different_model_is_not_gated() {
        let inner = rate_limited_then_ok();
        let paced = with_cadence(inner.clone(), Some(&wiring()));
        let _ = paced.generate(request("gpt-x")).await;

        let start = tokio::time::Instant::now();
        let response = paced.generate(request("gpt-y")).await.expect("other model");
        assert_eq!(response.content, "ok");
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn a_non_rate_limit_error_passes_through_and_opens_no_gate() {
        let inner = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![
                    MockScriptEntry::Error(LlmError::NetworkError("down".to_string())),
                    MockScriptEntry::Text("ok".to_string()),
                ]),
        );
        let paced = with_cadence(inner.clone(), Some(&wiring()));

        assert!(matches!(
            paced.generate(request("gpt-x")).await,
            Err(LlmError::NetworkError(_))
        ));
        let start = tokio::time::Instant::now();
        paced.generate(request("gpt-x")).await.expect("second");
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn a_success_after_a_gate_resets_the_streak() {
        let wiring = wiring();
        let inner = rate_limited_then_ok();
        let paced = with_cadence(inner, Some(&wiring));
        let key = CadenceKey::new("openai", "gpt-x");

        let _ = paced.generate(request("gpt-x")).await;
        assert_eq!(wiring.port().gate(&key).await.expect("gate").streak(), 1);
        paced.generate(request("gpt-x")).await.expect("second");
        assert_eq!(wiring.port().gate(&key).await.expect("gate").streak(), 0);
    }

    #[test]
    fn with_cadence_installs_nothing_when_off_or_for_a_fallback_chain() {
        let plain: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        assert!(Arc::ptr_eq(&with_cadence(plain.clone(), None), &plain));

        let chain: Arc<dyn LlmPort> = Arc::new(
            FallbackLlmAdapter::new(vec![
                Arc::new(MockLlmAdapter::new().with_provider_name("openai")) as Arc<dyn LlmPort>,
                Arc::new(MockLlmAdapter::new().with_provider_name("anthropic")),
            ])
            .expect("chain"),
        );
        assert_eq!(chain.get_provider_name(), FALLBACK_PROVIDER_NAME);
        assert!(Arc::ptr_eq(
            &with_cadence(chain.clone(), Some(&wiring())),
            &chain
        ));

        let paced = with_cadence(plain.clone(), Some(&wiring()));
        assert!(!Arc::ptr_eq(&paced, &plain), "a plain provider is wrapped");
        assert_eq!(paced.get_provider_name(), plain.get_provider_name());
    }

    /// A port that always fails, to prove bookkeeping never fails a call.
    struct BrokenPort;

    #[async_trait]
    impl CadencePort for BrokenPort {
        async fn gate(&self, _key: &CadenceKey) -> Result<GateReading, CadenceError> {
            Err(CadenceError::Backend {
                message: "down".to_string(),
            })
        }
        async fn record_rate_limited(
            &self,
            _key: &CadenceKey,
            _retry_after: Option<Duration>,
        ) -> Result<GateReading, CadenceError> {
            Err(CadenceError::Backend {
                message: "down".to_string(),
            })
        }
        async fn record_success(&self, _key: &CadenceKey) -> Result<(), CadenceError> {
            Err(CadenceError::Backend {
                message: "down".to_string(),
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_failing_port_never_fails_or_alters_the_call() {
        let wiring = CadenceWiring::new(Arc::new(BrokenPort), CadenceSettings::default());
        let inner = rate_limited_then_ok();
        let paced = with_cadence(inner.clone(), Some(&wiring));

        assert!(matches!(
            paced.generate(request("m")).await,
            Err(LlmError::RateLimitExceeded)
        ));
        paced
            .generate(request("m"))
            .await
            .expect("second call succeeds");
        assert_eq!(inner.call_count(), 2);
    }

    #[test]
    fn settings_default_matches_d10_and_debug_names_no_port() {
        let w = wiring();
        assert_eq!(w.settings(), &CadenceSettings::default());
        assert!(format!("{w:?}").contains("CadenceWiring"));
    }
}
