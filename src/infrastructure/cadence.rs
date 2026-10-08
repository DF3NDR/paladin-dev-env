//! Rate-pacing composition -- `build_cadence` and `compose_llm` (PACE-02, D-07, D-08; Phase 43)
//!
//! The composition root's half of the Cadence: [`build_cadence`] turns the operator's
//! `treasurer.cadence` section into one [`CadenceWiring`] (a shared
//! [`CadencePort`](paladin_ports::output::cadence_port::CadencePort) plus its settings), and
//! [`compose_llm`] layers every resolved provider as `Pricing(Cadence(provider))`:
//!
//! ```text
//! with_pricing( with_cadence( provider ) )
//! ```
//!
//! Pricing stays outermost so it still prices the response that was actually served (ADR-0052);
//! Cadence sits directly on the provider so its key is that provider's own name.
//!
//! ## One wiring per process
//!
//! The in-process backend keeps its gate state inside the port instance, so ONE
//! [`CadenceWiring`] must be shared by every composition root in a process for pacing to be
//! process-wide. Building a wiring per call site would pace each site independently.
//! `paladin-server` builds one at boot and hands clones to the agent host, the facade
//! provisioner and the run engine's wiring.
//!
//! ## On by default
//!
//! A default `treasurer.cadence` section yields `Some(wiring)` over the in-process adapter (D-08);
//! only `enabled: false` yields `None`, and then [`compose_llm`] installs no pacing layer at all.
//!
//! ## Backends
//!
//! | `treasurer.cadence.backend` | Port built | Needs |
//! |-----------------------------|------------|-------|
//! | `in_process` (default) | `InMemoryCadence` | nothing |
//! | `{ redis: { url_env } }` | `ResilientCadence(RedisCadence, InMemoryCadence x degraded_multiplier)` | the `redis-cadence` feature and the named environment variable |
//!
//! The Redis backend is built **without connecting** (`RedisCadence::new` only parses the URL),
//! so a worker boots while Redis is down and starts degraded on first use: the composite
//! serves every call from the in-process fallback, with delays multiplied by
//! `degraded_multiplier`, warns once per outage and recovers on its own (D-05). A binary built
//! without `redis-cadence` refuses `backend: redis` at boot rather than quietly pacing
//! in-process -- a fleet that believes it shares pacing state must never silently not.
//!
//! All Redis keys live under the fixed `paladin:cadence` namespace: two independent fleets
//! pointed at one Redis server share pacing state for the same provider and model. Give each
//! fleet its own Redis server or logical database.
//!
//! The Redis URL may carry a password. It is read from the named environment variable here, at
//! boot, handed straight to `RedisCadence` (whose `Debug` and errors redact it) and never
//! logged, stored on a config type or serialised.

use std::sync::Arc;

use paladin_core::platform::container::cost::PriceTable;
use paladin_llm::cadence::{CadenceWiring, with_cadence};
use paladin_llm::pricing::with_pricing;
use paladin_ports::output::llm_port::LlmPort;
use paladin_storage::cadence::InMemoryCadence;
#[cfg(feature = "redis-cadence")]
use paladin_storage::cadence::{RedisCadence, RedisCadenceConfig, ResilientCadence};

use crate::config::treasurer::{CadenceBackend, CadenceConfig};

/// Build the shared pacing wiring from `config`.
///
/// Returns `Ok(None)` when `config.enabled` is `false` (nothing is installed), and otherwise a
/// wiring over the backend `config.backend` names: `InMemoryCadence` for `in_process`, and for
/// `redis { url_env }` a `ResilientCadence` over a `RedisCadence` with an
/// `InMemoryCadence` fallback at `degraded_multiplier`. The Redis backend does not connect here:
/// an unreachable server degrades pacing on first use, it does not fail boot (D-05).
///
/// # Errors
///
/// A `String` naming the offending `treasurer.cadence.*` key when `config` fails
/// [`CadenceConfig::validate`] (which includes a `backend.redis.url_env` that is empty or names
/// an unset variable), or when `backend: redis` is configured on a binary built without the
/// `redis-cadence` feature -- the message names the feature and `backend: in_process` as the
/// alternative. The Redis URL never appears in an error.
///
/// # Examples
///
/// ```
/// use paladin::config::treasurer::CadenceConfig;
/// use paladin::infrastructure::cadence::build_cadence;
///
/// // On by default: an omitted `treasurer.cadence` section paces.
/// assert!(build_cadence(&CadenceConfig::default())?.is_some());
///
/// // `enabled: false` installs nothing.
/// let off = CadenceConfig {
///     enabled: false,
///     ..CadenceConfig::default()
/// };
/// assert!(build_cadence(&off)?.is_none());
/// # Ok::<(), String>(())
/// ```
pub fn build_cadence(config: &CadenceConfig) -> Result<Option<CadenceWiring>, String> {
    config.validate()?;
    if !config.enabled {
        return Ok(None);
    }
    match &config.backend {
        CadenceBackend::InProcess => {
            let port = InMemoryCadence::new(config.policy()?);
            Ok(Some(CadenceWiring::new(Arc::new(port), config.settings())))
        }
        CadenceBackend::Redis { url_env } => build_redis_wiring(config, url_env).map(Some),
    }
}

/// The `redis` backend: `ResilientCadence(RedisCadence, InMemoryCadence x degraded_multiplier)`.
#[cfg(feature = "redis-cadence")]
fn build_redis_wiring(config: &CadenceConfig, url_env: &str) -> Result<CadenceWiring, String> {
    // Only the variable NAME is ever put in a message; the value is the URL.
    let url = std::env::var(url_env).map_err(|_| {
        format!(
            "treasurer.cadence.backend.redis.url_env names env var '{url_env}', which is not set \
             (or is not valid unicode)"
        )
    })?;
    let redis = RedisCadence::new(RedisCadenceConfig::new(url), config.policy()?)
        .map_err(|e| format!("treasurer.cadence.backend.redis: {e}"))?;
    let fallback =
        InMemoryCadence::new(config.policy()?).with_multiplier(config.degraded_multiplier);
    Ok(CadenceWiring::new(
        Arc::new(ResilientCadence::new(Arc::new(redis), fallback)),
        config.settings(),
    ))
}

/// The `redis` backend on a binary built without `redis-cadence`: a boot error, never a silent
/// in-process fallback.
#[cfg(not(feature = "redis-cadence"))]
fn build_redis_wiring(_config: &CadenceConfig, _url_env: &str) -> Result<CadenceWiring, String> {
    Err(
        "treasurer.cadence.backend is `redis`, but this binary was built without the \
         `redis-cadence` feature; rebuild with `--features redis-cadence`, or set \
         `treasurer.cadence.backend: in_process`"
            .to_string(),
    )
}

/// Compose a resolved provider as `Pricing(Cadence(provider))`.
///
/// `cadence: None` installs no pacing layer, and an empty `price_table` installs no pricing
/// layer, so with both off `llm` comes back unchanged (`Arc::ptr_eq` holds).
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
///
/// use paladin::config::treasurer::CadenceConfig;
/// use paladin::infrastructure::cadence::{build_cadence, compose_llm};
/// use paladin_core::platform::container::cost::{CurrencyCode, PriceTable};
/// use paladin_llm::mock::MockLlmAdapter;
/// use paladin_ports::output::llm_port::LlmPort;
///
/// let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
/// let empty_prices = Arc::new(PriceTable::new(CurrencyCode::new("USD")?));
///
/// // Nothing to install: the very same port comes back.
/// assert!(Arc::ptr_eq(&compose_llm(llm.clone(), &empty_prices, None), &llm));
///
/// // With pacing on, the provider is wrapped (and still reports its own name).
/// let wiring = build_cadence(&CadenceConfig::default())?;
/// let paced = compose_llm(llm.clone(), &empty_prices, wiring.as_ref());
/// assert!(!Arc::ptr_eq(&paced, &llm));
/// assert_eq!(paced.get_provider_name(), llm.get_provider_name());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn compose_llm(
    llm: Arc<dyn LlmPort>,
    price_table: &Arc<PriceTable>,
    cadence: Option<&CadenceWiring>,
) -> Arc<dyn LlmPort> {
    with_pricing(with_cadence(llm, cadence), price_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::treasurer::TreasurerConfig;
    use paladin_core::platform::container::cost::CurrencyCode;

    fn empty_price_table() -> Result<Arc<PriceTable>, Box<dyn std::error::Error>> {
        Ok(Arc::new(PriceTable::new(CurrencyCode::new("USD")?)))
    }

    #[test]
    fn cadence_defaults_on_when_section_omitted() -> Result<(), Box<dyn std::error::Error>> {
        let from_default = build_cadence(&TreasurerConfig::default().cadence)?;
        assert!(from_default.is_some(), "pacing is on by default (D-08)");

        let from_empty_json: TreasurerConfig = serde_json::from_str("{}")?;
        assert!(build_cadence(&from_empty_json.cadence)?.is_some());
        Ok(())
    }

    #[test]
    fn cadence_disabled_installs_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let config: TreasurerConfig = serde_json::from_str(r#"{"cadence":{"enabled":false}}"#)?;
        let wiring = build_cadence(&config.cadence)?;
        assert!(wiring.is_none());

        let llm: Arc<dyn LlmPort> = Arc::new(paladin_llm::mock::MockLlmAdapter::new());
        let composed = compose_llm(llm.clone(), &empty_price_table()?, wiring.as_ref());
        assert!(Arc::ptr_eq(&composed, &llm), "nothing installed");
        Ok(())
    }

    /// Wiring-building tests for `backend: redis`. They use a private variable name so they never
    /// race the config tests' variables, and a URL on `127.0.0.1:1` (nothing listens there).
    const REDIS_URL_VAR: &str = "PALADIN_TEST_CADENCE_BUILD_REDIS_URL";

    fn redis_backend_config() -> CadenceConfig {
        CadenceConfig {
            backend: CadenceBackend::Redis {
                url_env: REDIS_URL_VAR.to_string(),
            },
            ..CadenceConfig::default()
        }
    }

    #[cfg(feature = "redis-cadence")]
    #[tokio::test]
    #[serial_test::serial]
    async fn build_cadence_redis_backend_builds_without_connecting()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{Duration, Instant};

        unsafe {
            std::env::set_var(REDIS_URL_VAR, "redis://:hunter2@127.0.0.1:1/0");
        }
        let started = Instant::now();
        let wiring = build_cadence(&redis_backend_config());
        unsafe {
            std::env::remove_var(REDIS_URL_VAR);
        }
        let wiring = wiring?.expect("redis backend builds a wiring");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "construction must not wait on the (dead) server"
        );

        // A worker that boots while Redis is down still gets a working, degraded gate: the first
        // use latches the fallback and nothing errors.
        let key = paladin_ports::output::cadence_port::CadenceKey::new("openai", "gpt-4o");
        let reading = wiring.port().record_rate_limited(&key, None).await?;
        assert!(reading.wait() > Duration::ZERO, "the fallback paces");
        Ok(())
    }

    #[cfg(not(feature = "redis-cadence"))]
    #[test]
    #[serial_test::serial]
    fn build_cadence_redis_backend_without_the_feature_names_the_feature() {
        unsafe {
            std::env::set_var(REDIS_URL_VAR, "redis://:hunter2@127.0.0.1:1/0");
        }
        let result = build_cadence(&redis_backend_config());
        unsafe {
            std::env::remove_var(REDIS_URL_VAR);
        }
        let err = result.expect_err("never a silent in-process fallback");
        assert!(err.contains("redis-cadence"), "{err}");
        assert!(err.contains("in_process"), "{err}");
        assert!(!err.contains("hunter2"), "{err}");
    }

    #[test]
    #[serial_test::serial]
    fn build_cadence_redis_backend_with_an_unset_variable_names_the_variable() {
        unsafe {
            std::env::remove_var(REDIS_URL_VAR);
        }
        let err = build_cadence(&redis_backend_config()).expect_err("unset variable");
        assert!(err.contains(REDIS_URL_VAR), "{err}");
        assert!(
            err.contains("treasurer.cadence.backend.redis.url_env"),
            "{err}"
        );
    }

    /// D-08 / T-43-36: ONE wiring shared by two composed ports (the agent host's and the run
    /// engine's, in the server) paces them together -- a 429 seen through the first delays the
    /// second's next call to the same provider and model by at least the base back-off.
    #[tokio::test(start_paused = true)]
    async fn one_wiring_paces_the_agent_host_and_the_run_engine_together()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;

        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
        use paladin_ports::output::llm_port::{LlmError, LlmRequest};
        use tokio::time::Instant;

        let config = CadenceConfig::default();
        let base = Duration::from_millis(config.base_backoff_ms);
        let wiring = build_cadence(&config)?.expect("pacing is on by default");

        // Two ports over two providers that both report the name "openai".
        let agent_host_provider = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Error(LlmError::rate_limited(None))]),
        );
        let run_engine_provider = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Text("ok".to_string())]),
        );
        let agent_host = compose_llm(
            agent_host_provider.clone(),
            &empty_price_table()?,
            Some(&wiring),
        );
        let run_engine = compose_llm(
            run_engine_provider.clone(),
            &empty_price_table()?,
            Some(&wiring.clone()),
        );

        let request = || -> Result<LlmRequest, Box<dyn std::error::Error>> {
            Ok(LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))?,
            ))
        };

        let first = agent_host.generate(request()?).await;
        let recorded_at = Instant::now();
        assert!(
            matches!(first, Err(LlmError::RateLimitExceeded { .. })),
            "the 429 reaches the agent host's caller unchanged, got {first:?}"
        );

        let second = run_engine.generate(request()?).await?;
        let gap = recorded_at.elapsed();
        assert_eq!(second.content, "ok");
        assert!(
            gap >= base,
            "the run engine's call reached its provider only {gap:?} after the agent host's \
             429; one shared wiring must hold it for at least {base:?}"
        );
        Ok(())
    }

    /// The control for the test above: wirings built separately do NOT share state, which is
    /// exactly why the server builds one and hands clones to every composition root.
    #[tokio::test(start_paused = true)]
    async fn separate_wirings_do_not_share_gate_state() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;

        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
        use paladin_ports::output::llm_port::{LlmError, LlmRequest};
        use tokio::time::Instant;

        let config = CadenceConfig::default();
        let wiring_a = build_cadence(&config)?.expect("on");
        let wiring_b = build_cadence(&config)?.expect("on");
        let provider_a = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Error(LlmError::rate_limited(None))]),
        );
        let provider_b = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Text("ok".to_string())]),
        );
        let port_a = compose_llm(provider_a, &empty_price_table()?, Some(&wiring_a));
        let port_b = compose_llm(provider_b, &empty_price_table()?, Some(&wiring_b));

        let request = || -> Result<LlmRequest, Box<dyn std::error::Error>> {
            Ok(LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))?,
            ))
        };
        let _ = port_a.generate(request()?).await;
        let recorded_at = Instant::now();
        port_b.generate(request()?).await?;
        assert!(
            recorded_at.elapsed() < Duration::from_millis(50),
            "an unshared wiring must not be gated by another wiring's 429"
        );
        Ok(())
    }

    #[test]
    fn an_invalid_cadence_section_names_its_key() {
        let config = CadenceConfig {
            base_backoff_ms: 0,
            ..CadenceConfig::default()
        };
        let err = build_cadence(&config).expect_err("zero base");
        assert!(err.contains("treasurer.cadence.base_backoff_ms"), "{err}");
    }

    /// PACE-05 / ROADMAP success criterion 5, end to end: with the shared backend down, a call
    /// after a provider 429 is still delayed -- by the in-process fallback, at the stricter
    /// degraded multiplier -- and nothing the outage does fails an LLM call. Needs no Redis
    /// server: the "dead Redis" is a test-local port that errors on every operation.
    #[tokio::test(start_paused = true)]
    async fn cadence_with_redis_down_still_paces() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;

        use async_trait::async_trait;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_llm::cadence::CadenceSettings;
        use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
        use paladin_ports::output::cadence_port::{
            CadenceError, CadenceKey, CadencePolicy, CadencePort, FencingToken, GateReading,
            LockKey,
        };
        use paladin_ports::output::llm_port::{LlmError, LlmRequest};
        use paladin_storage::cadence::ResilientCadence;
        use tokio::time::Instant;

        /// A shared pacing backend that is unreachable.
        struct RedisDown;

        fn down() -> CadenceError {
            CadenceError::Backend {
                message: "connection refused".to_string(),
            }
        }

        #[async_trait]
        impl CadencePort for RedisDown {
            async fn gate(&self, _: &CadenceKey) -> Result<GateReading, CadenceError> {
                Err(down())
            }
            async fn record_rate_limited(
                &self,
                _: &CadenceKey,
                _: Option<Duration>,
            ) -> Result<GateReading, CadenceError> {
                Err(down())
            }
            async fn record_success(&self, _: &CadenceKey) -> Result<(), CadenceError> {
                Err(down())
            }
            async fn try_lock(
                &self,
                _: &LockKey,
                _: Duration,
            ) -> Result<Option<FencingToken>, CadenceError> {
                Err(down())
            }
            async fn unlock(&self, _: &LockKey, _: &FencingToken) -> Result<bool, CadenceError> {
                Err(down())
            }
        }

        let policy = CadencePolicy::default();
        let base = policy.base_backoff();
        let resilient = Arc::new(ResilientCadence::new(
            Arc::new(RedisDown),
            InMemoryCadence::new(policy).with_multiplier(2.0),
        ));
        let wiring = CadenceWiring::new(
            Arc::clone(&resilient) as Arc<dyn CadencePort>,
            CadenceSettings::default(),
        );

        // The provider answers a delay-less 429, then succeeds.
        let provider = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![
                    MockScriptEntry::Error(LlmError::rate_limited(None)),
                    MockScriptEntry::Text("ok".to_string()),
                ]),
        );
        let llm = compose_llm(provider.clone(), &empty_price_table()?, Some(&wiring));

        let request = || -> Result<LlmRequest, Box<dyn std::error::Error>> {
            Ok(LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))?,
            ))
        };

        // Call 1: the provider's 429 reaches the caller unchanged, outage or not.
        let first = llm.generate(request()?).await;
        let first_returned = Instant::now();
        assert!(
            matches!(first, Err(LlmError::RateLimitExceeded { .. })),
            "the outage must not change the call's outcome, got {first:?}"
        );
        assert!(
            resilient.is_degraded(),
            "the dead backend latched degraded mode"
        );
        assert_eq!(provider.call_count(), 1);

        // Call 2: still gated -- at the degraded 2 x base, from the in-process fallback.
        let second = llm.generate(request()?).await?;
        let gap = first_returned.elapsed();
        assert_eq!(second.content, "ok");
        assert!(
            gap >= base * 2,
            "call 2 reached the provider only {gap:?} after the 429; a run must never be \
             unpaced, and degraded delays are 2 x base = {:?}",
            base * 2
        );
        assert!(
            gap < base * 2 + Duration::from_millis(150),
            "the wait should be the degraded gate plus a small spread, got {gap:?}"
        );
        assert_eq!(provider.call_count(), 2);
        Ok(())
    }

    /// The Redis server the fleet test talks to: `CADENCE_REDIS_TEST_URL`, defaulting to the
    /// compose `redis-test` service on logical database 2 (the database the storage suite uses).
    #[cfg(feature = "redis-cadence")]
    fn cadence_redis_test_url() -> String {
        std::env::var("CADENCE_REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6380/2".to_string())
    }

    /// A short-timeout TCP probe, so a missing server is a fast, clean skip.
    #[cfg(feature = "redis-cadence")]
    fn redis_reachable(url: &str) -> bool {
        use std::net::ToSocketAddrs;

        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let port = parsed.port().unwrap_or(6379);
        (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .is_some_and(|addr| {
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(750))
                    .is_ok()
            })
    }

    /// PACE-03 / ROADMAP success criterion 3, end to end across workers: two workers, each with
    /// its own `RedisCadence` (hence its own Redis connection) over one server and one key
    /// prefix. A 429 on worker A measurably slows worker B's next call to the same provider and
    /// model, while another model on worker B is not delayed.
    #[cfg(feature = "redis-cadence")]
    #[tokio::test]
    async fn cadence_fleet_429_on_one_worker_slows_the_other()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{Duration, Instant};

        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_llm::cadence::CadenceSettings;
        use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
        use paladin_ports::output::cadence_port::CadencePolicy;
        use paladin_ports::output::llm_port::{LlmError, LlmRequest};
        use paladin_storage::cadence::redis::{RedisCadence, RedisCadenceConfig};

        let url = cadence_redis_test_url();
        if !redis_reachable(&url) {
            println!(
                "SKIP: redis-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d redis-test` \
                 (or point CADENCE_REDIS_TEST_URL at a reachable server)"
            );
            return Ok(());
        }

        // One unique prefix: the two workers share state, no other test does.
        let prefix = format!("test-cadence-fleet-{}", uuid::Uuid::new_v4());
        let worker_wiring = || -> Result<CadenceWiring, Box<dyn std::error::Error>> {
            let port = RedisCadence::new(
                RedisCadenceConfig::new(url.as_str()).with_key_prefix(prefix.as_str()),
                CadencePolicy::default(),
            )?;
            Ok(CadenceWiring::new(
                Arc::new(port),
                CadenceSettings::default(),
            ))
        };
        let wiring_a = worker_wiring()?;
        let wiring_b = worker_wiring()?;

        // Both workers talk to "openai"; worker A's provider answers one 429 with a 1 s delay.
        let provider_a = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Error(LlmError::rate_limited(Some(
                    Duration::from_secs(1),
                )))]),
        );
        let provider_b = Arc::new(
            MockLlmAdapter::new()
                .with_provider_name("openai")
                .with_script(vec![MockScriptEntry::Text("ok".to_string())]),
        );
        let worker_a = compose_llm(provider_a.clone(), &empty_price_table()?, Some(&wiring_a));
        let worker_b = compose_llm(provider_b.clone(), &empty_price_table()?, Some(&wiring_b));

        let request = |model: &str| -> Result<LlmRequest, Box<dyn std::error::Error>> {
            Ok(LlmRequest::new(
                model,
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))?,
            ))
        };

        // Worker A gets the 429, unchanged, with exactly one provider hit.
        let first = worker_a.generate(request("gpt-4o")?).await;
        let recorded_at = Instant::now();
        assert!(
            matches!(first, Err(LlmError::RateLimitExceeded { .. })),
            "expected the unchanged rate limit, got {first:?}"
        );
        assert_eq!(provider_a.call_count(), 1);

        // Another model on worker B is not delayed by a gate on gpt-4o.
        let unrelated_started = Instant::now();
        let unrelated = worker_b.generate(request("gpt-4o-mini")?).await?;
        let unrelated_took = unrelated_started.elapsed();
        assert_eq!(unrelated.content, "ok");
        assert!(
            unrelated_took < Duration::from_millis(200),
            "a different model must not be gated by worker A's 429, took {unrelated_took:?}"
        );

        // Worker B's call to the same provider and model is held by the gate worker A opened.
        let paced = worker_b.generate(request("gpt-4o")?).await?;
        let gap = recorded_at.elapsed();
        assert_eq!(paced.content, "ok", "the script cycles, so B answers again");
        assert!(
            gap >= Duration::from_millis(900),
            "worker B reached its provider only {gap:?} after worker A's 429; the shared gate \
             must hold it for about 1 s"
        );
        assert_eq!(provider_b.call_count(), 2);
        Ok(())
    }

    /// D-10 end to end on a live server: two wirings built by `build_cadence` from the SAME
    /// `backend: redis` config (two workers) share state, so a 429 recorded through one gates
    /// the other. The default key prefix is the only one `build_cadence` can build, so the test
    /// uses a unique provider name instead of a unique prefix.
    #[cfg(feature = "redis-cadence")]
    #[tokio::test]
    #[serial_test::serial]
    async fn build_cadence_redis_backend_shares_state_between_two_workers()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;

        use paladin_ports::output::cadence_port::CadenceKey;

        let url = cadence_redis_test_url();
        if !redis_reachable(&url) {
            println!("SKIP: redis-test not reachable at {url}");
            return Ok(());
        }
        unsafe {
            std::env::set_var(REDIS_URL_VAR, url.as_str());
        }
        let config = redis_backend_config();
        let worker_a = build_cadence(&config);
        let worker_b = build_cadence(&config);
        unsafe {
            std::env::remove_var(REDIS_URL_VAR);
        }
        let worker_a = worker_a?.expect("redis backend builds a wiring");
        let worker_b = worker_b?.expect("redis backend builds a wiring");

        let key = CadenceKey::new(&format!("build-cadence-{}", uuid::Uuid::new_v4()), "m");
        let recorded = worker_a
            .port()
            .record_rate_limited(&key, Some(Duration::from_secs(5)))
            .await?;
        assert!(recorded.wait() >= Duration::from_secs(4), "{recorded:?}");
        let seen = worker_b.port().gate(&key).await?;
        assert!(
            seen.wait() >= Duration::from_secs(3),
            "worker B must see worker A's gate through Redis, got {seen:?}"
        );
        Ok(())
    }

    /// Phase 43's tracer: a provider 429 slows the NEXT call, end to end -- config ->
    /// `build_cadence` -> `compose_llm` -> decorator -> port -> in-process adapter -> a real
    /// `OpenAIAdapter` (with `max_retries: 3`) -> an HTTP mock.
    #[cfg(feature = "llm-openai")]
    #[tokio::test]
    async fn cadence_tracer_paces_a_real_openai_429_end_to_end()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{Duration, Instant};

        use mockito::Server;
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};
        use paladin_ports::output::llm_port::{LlmError, LlmRequest};

        let config: TreasurerConfig =
            serde_json::from_str(r#"{"cadence":{"base_backoff_ms":200,"max_backoff_ms":400}}"#)?;
        let wiring = build_cadence(&config.cadence)?;
        assert!(wiring.is_some(), "pacing is on");

        let mut server = Server::new_async().await;
        let openai = OpenAIAdapter::new(OpenAIConfig {
            api_key: "test-key".to_string(),
            base_url: server.url(),
            organization: None,
            timeout_seconds: 5,
            max_retries: 3,
        })?;
        let llm = compose_llm(Arc::new(openai), &empty_price_table()?, wiring.as_ref());

        let request = || -> Result<LlmRequest, Box<dyn std::error::Error>> {
            Ok(LlmRequest::new(
                "gpt-4o",
                PromptItem::new(PromptType::User(UserPrompt {
                    query: "Hello".to_string(),
                    context: None,
                }))?,
            ))
        };

        // Call 1: a 429 is surfaced on the first attempt (the adapter retries nothing) ...
        let rate_limited = server
            .mock("POST", "/chat/completions")
            .with_status(429)
            .with_body(r#"{"error":{"message":"slow down"}}"#)
            .expect(1)
            .create_async()
            .await;
        let first = llm.generate(request()?).await;
        let first_returned = Instant::now();
        assert!(
            matches!(first, Err(LlmError::RateLimitExceeded { .. })),
            "expected the unchanged rate limit, got {first:?}"
        );
        rate_limited.assert_async().await;
        rate_limited.remove_async().await;

        // ... and call 2 is held back by the gate it opened, then succeeds.
        let served = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "id": "cmpl-1",
                    "model": "gpt-4o",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;
        let second = llm.generate(request()?).await?;
        let gap = first_returned.elapsed();
        assert_eq!(second.content, "ok");
        assert!(
            gap >= Duration::from_millis(200),
            "call 2 returned only {gap:?} after call 1; the gate must hold it for >= 200 ms"
        );
        served.assert_async().await;
        Ok(())
    }
}
