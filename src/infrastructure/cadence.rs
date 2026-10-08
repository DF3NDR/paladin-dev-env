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
//! process-wide. Building a wiring per call site would pace each site independently. Plan 43-09
//! threads a single wiring through the whole server.
//!
//! ## On by default
//!
//! A default `treasurer.cadence` section yields `Some(wiring)` over the in-process adapter (D-08);
//! only `enabled: false` yields `None`, and then [`compose_llm`] installs no pacing layer at all.

use std::sync::Arc;

use paladin_core::platform::container::cost::PriceTable;
use paladin_llm::cadence::{CadenceWiring, with_cadence};
use paladin_llm::pricing::with_pricing;
use paladin_ports::output::llm_port::LlmPort;
use paladin_storage::cadence::InMemoryCadence;

use crate::config::treasurer::{CadenceBackend, CadenceConfig};

/// Build the shared pacing wiring from `config`.
///
/// Returns `Ok(None)` when `config.enabled` is `false` (nothing is installed), and otherwise a
/// wiring over the backend `config.backend` names.
///
/// # Errors
///
/// A `String` naming the offending `treasurer.cadence.*` key when `config` fails
/// [`CadenceConfig::validate`].
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
    match config.backend {
        CadenceBackend::InProcess => {
            let port = InMemoryCadence::new(config.policy()?);
            Ok(Some(CadenceWiring::new(Arc::new(port), config.settings())))
        }
    }
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

    #[test]
    fn an_invalid_cadence_section_names_its_key() {
        let config = CadenceConfig {
            base_backoff_ms: 0,
            ..CadenceConfig::default()
        };
        let err = build_cadence(&config).expect_err("zero base");
        assert!(err.contains("treasurer.cadence.base_backoff_ms"), "{err}");
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
