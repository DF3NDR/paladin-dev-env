//! Builds a populated [`AgentRegistry`](paladin_web::AgentRegistry) from configuration
//! for the HTTP service host (Milestone 12, Epic 2).
//!
//! This is composition-root glue: it lives in the facade crate because it wires the
//! application-layer [`PaladinBuilder`] / [`PaladinExecutionService`] and the
//! `paladin-llm` provider factory into the `paladin-web` registry. `paladin-web` itself
//! depends on neither.
//!
//! Agents are **LLM + prompt only** in this epic — no garrison (memory) or arsenal
//! (tools). [`build_agent`] is the single build path shared by config-load
//! ([`build_agent_registry`]) and runtime registration (the
//! [`FacadeProvisioner`](super::facade_provisioner::FacadeProvisioner)).

use std::sync::Arc;
use std::time::Duration;

use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_llm::provider_factory::{LlmProviderFactory, ProviderFactoryError};
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_web::AgentRegistry;

use crate::application::services::paladin::paladin_builder::PaladinBuilder;
use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use crate::config::agents::AgentDefinition;
use crate::config::settings::Settings;
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;

/// Errors raised while building agents from configuration.
#[derive(Debug, thiserror::Error)]
pub enum HostBuildError {
    /// The provider named by an agent could not be created (unknown provider or
    /// missing provider configuration / API key).
    #[error("agent '{id}': provider '{provider}' unavailable: {source}")]
    Provider {
        /// The offending agent id.
        id: String,
        /// The provider name that failed to resolve.
        provider: String,
        /// The underlying factory error.
        source: ProviderFactoryError,
    },

    /// The agent could not be built from its definition.
    #[error("agent '{id}': failed to build: {source}")]
    Build {
        /// The offending agent id.
        id: String,
        /// The underlying builder error.
        source: PaladinError,
    },

    /// Two agents in the configuration share an id.
    #[error("duplicate agent id '{0}' in configuration")]
    DuplicateId(String),

    /// An agent names a provider that is not available in this build.
    #[error("agent '{id}': unknown provider '{provider}' (available: {available:?})")]
    UnknownProvider {
        /// The offending agent id.
        id: String,
        /// The unavailable provider name.
        provider: String,
        /// Providers that are available in this build.
        available: Vec<String>,
    },

    /// An agent definition is structurally invalid (e.g. an empty required field).
    #[error("agent '{id}': {reason}")]
    InvalidAgent {
        /// The offending agent id (may be empty if that is the problem).
        id: String,
        /// Why the definition is invalid.
        reason: String,
    },
}

/// Default circuit-breaker settings shared across config-built agents.
pub(crate) fn default_circuit_breaker() -> Arc<CircuitBreaker> {
    Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)))
}

/// Resolve the provider name for an agent: its explicit `provider`, else the supplied
/// default.
pub(crate) fn resolve_provider(def: &AgentDefinition, default_provider: &str) -> String {
    def.provider
        .clone()
        .unwrap_or_else(|| default_provider.to_string())
}

/// Determine the default provider from settings, falling back to the factory default
/// and finally `"openai"`.
pub(crate) fn default_provider_name(settings: &Settings) -> String {
    settings
        .llm
        .as_ref()
        .and_then(|l| l.default_provider.clone())
        .or_else(LlmProviderFactory::get_default_provider)
        .unwrap_or_else(|| "openai".to_string())
}

/// Build a `(Paladin, executor)` pair from a definition and an already-resolved LLM.
///
/// This is the hermetic core of agent construction — it performs no provider lookup, so
/// it can be exercised in tests with a mock [`LlmPort`].
pub(crate) async fn build_agent_with_llm(
    def: &AgentDefinition,
    llm: Arc<dyn LlmPort>,
    breaker: Arc<CircuitBreaker>,
) -> Result<(Paladin, Arc<dyn PaladinExecutorPort>), HostBuildError> {
    // The executor and the builder share the same LLM port.
    let executor: Arc<dyn PaladinExecutorPort> = Arc::new(PaladinExecutionService::new(
        Arc::clone(&llm),
        breaker,
        None,
        None,
    ));

    let mut builder = PaladinBuilder::new(llm)
        .name(&def.id)
        .system_prompt(&def.system_prompt)
        .model(&def.model);
    if let Some(temperature) = def.temperature {
        builder = builder.temperature(temperature);
    }
    if let Some(max_loops) = def.max_loops {
        builder = builder.max_loops(max_loops);
    }
    for word in &def.stop_words {
        builder = builder.add_stop_word(word.clone());
    }

    let paladin = builder
        .build()
        .await
        .map_err(|source| HostBuildError::Build {
            id: def.id.clone(),
            source,
        })?;

    Ok((paladin, executor))
}

/// Build a `(Paladin, executor)` pair from a definition, resolving the provider via the
/// factory. Shared by config load and runtime provisioning.
pub(crate) async fn build_agent(
    def: &AgentDefinition,
    factory: &LlmProviderFactory,
    default_provider: &str,
    breaker: Arc<CircuitBreaker>,
) -> Result<(Paladin, Arc<dyn PaladinExecutorPort>), HostBuildError> {
    let provider = resolve_provider(def, default_provider);
    let llm = factory
        .create(&provider)
        .map_err(|source| HostBuildError::Provider {
            id: def.id.clone(),
            provider,
            source,
        })?;
    build_agent_with_llm(def, llm, breaker).await
}

/// Insert a built agent into the registry, rejecting a duplicate id.
pub(crate) fn register_built(
    registry: &AgentRegistry,
    id: &str,
    paladin: Paladin,
    executor: Arc<dyn PaladinExecutorPort>,
) -> Result<(), HostBuildError> {
    if registry.insert(id.to_string(), Arc::new(paladin), executor) {
        Ok(())
    } else {
        Err(HostBuildError::DuplicateId(id.to_string()))
    }
}

/// The TCP bind address derived from the `server` section (`host:port`).
pub fn bind_address(settings: &Settings) -> String {
    format!("{}:{}", settings.server.host, settings.server.port)
}

/// Validate the `agents` configuration *before* building anything.
///
/// This is a fast, key-free pre-flight check so misconfiguration fails at startup with a
/// specific message rather than mid-build. It verifies, for every agent: non-empty `id`,
/// `model`, and `system_prompt`; no duplicate ids; and that the resolved provider is one
/// of the providers available in this build. It does **not** verify API keys — those are
/// checked when the provider is actually created in [`build_agent`].
///
/// # Errors
///
/// Returns the first [`HostBuildError`] encountered.
pub fn validate_config(settings: &Settings) -> Result<(), HostBuildError> {
    let default_provider = default_provider_name(settings);
    let available = LlmProviderFactory::list_available_providers();
    let mut seen = std::collections::HashSet::new();

    for def in &settings.agents {
        if def.id.trim().is_empty() {
            return Err(HostBuildError::InvalidAgent {
                id: def.id.clone(),
                reason: "id must not be empty".to_string(),
            });
        }
        for (field, value) in [("model", &def.model), ("system_prompt", &def.system_prompt)] {
            if value.trim().is_empty() {
                return Err(HostBuildError::InvalidAgent {
                    id: def.id.clone(),
                    reason: format!("{field} must not be empty"),
                });
            }
        }
        if !seen.insert(def.id.clone()) {
            return Err(HostBuildError::DuplicateId(def.id.clone()));
        }
        let provider = resolve_provider(def, &default_provider);
        if !available.iter().any(|p| p == &provider) {
            return Err(HostBuildError::UnknownProvider {
                id: def.id.clone(),
                provider,
                available: available.clone(),
            });
        }
    }
    Ok(())
}

/// Build a populated [`AgentRegistry`] from the `agents` section of `settings`.
///
/// Runs [`validate_config`] first (fail-fast), then constructs each agent via
/// [`build_agent`]. A validation failure, an unresolvable provider, or a build failure
/// aborts with a descriptive [`HostBuildError`] naming the agent.
///
/// # Errors
///
/// Returns [`HostBuildError`] on the first problem encountered.
pub async fn build_agent_registry(settings: &Settings) -> Result<AgentRegistry, HostBuildError> {
    validate_config(settings)?;

    let factory = LlmProviderFactory::new();
    let default_provider = default_provider_name(settings);
    let breaker = default_circuit_breaker();

    let registry = AgentRegistry::new();
    for def in &settings.agents {
        let (paladin, executor) =
            build_agent(def, &factory, &default_provider, Arc::clone(&breaker)).await?;
        register_built(&registry, &def.id, paladin, executor)?;
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_llm::mock::MockLlmAdapter;

    fn base(id: &str) -> AgentDefinition {
        AgentDefinition {
            id: id.to_string(),
            model: "gpt-4".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            provider: None,
            temperature: None,
            max_loops: None,
            stop_words: vec![],
        }
    }

    fn mock_llm() -> Arc<dyn LlmPort> {
        Arc::new(MockLlmAdapter::new())
    }

    #[test]
    fn resolve_provider_prefers_explicit_then_default() {
        let mut def = base("a");
        def.provider = Some("anthropic".to_string());
        assert_eq!(resolve_provider(&def, "openai"), "anthropic");

        def.provider = None;
        assert_eq!(resolve_provider(&def, "openai"), "openai");
    }

    #[tokio::test]
    async fn build_agent_with_llm_applies_definition_fields() {
        let mut def = base("researcher");
        def.model = "gpt-4o".to_string();
        def.temperature = Some(0.5);
        def.max_loops = Some(2);

        let (paladin, _executor) =
            build_agent_with_llm(&def, mock_llm(), default_circuit_breaker())
                .await
                .expect("builds");

        assert_eq!(paladin.node.name, "researcher");
        assert_eq!(paladin.node.model, "gpt-4o");
    }

    #[tokio::test]
    async fn build_agent_unknown_provider_errors() {
        let mut def = base("x");
        def.provider = Some("no-such-provider".to_string());
        let factory = LlmProviderFactory::new();

        // Note: `(Paladin, Arc<dyn PaladinExecutorPort>)` is not `Debug`, so we match on
        // the result rather than using `expect_err`.
        let result = build_agent(&def, &factory, "openai", default_circuit_breaker()).await;
        assert!(
            matches!(result, Err(HostBuildError::Provider { .. })),
            "unknown provider must yield a Provider error"
        );
    }

    #[tokio::test]
    async fn register_built_rejects_duplicate_id() {
        let registry = AgentRegistry::new();

        let (p1, e1) = build_agent_with_llm(&base("dup"), mock_llm(), default_circuit_breaker())
            .await
            .unwrap();
        register_built(&registry, "dup", p1, e1).expect("first insert ok");

        let (p2, e2) = build_agent_with_llm(&base("dup"), mock_llm(), default_circuit_breaker())
            .await
            .unwrap();
        let err = register_built(&registry, "dup", p2, e2).expect_err("duplicate must error");
        assert!(matches!(err, HostBuildError::DuplicateId(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn build_agent_registry_empty_when_no_agents() {
        let settings = Settings::default(); // agents is empty
        let registry = build_agent_registry(&settings).await.expect("builds");
        assert!(registry.is_empty());
    }

    fn settings_with(agents: Vec<AgentDefinition>) -> Settings {
        Settings {
            agents,
            ..Settings::default()
        }
    }

    #[test]
    fn bind_address_uses_server_host_and_port() {
        let mut settings = Settings::default();
        settings.server.host = "0.0.0.0".to_string();
        settings.server.port = 3000;
        assert_eq!(bind_address(&settings), "0.0.0.0:3000");
    }

    #[test]
    fn validate_passes_for_empty_agents() {
        assert!(validate_config(&Settings::default()).is_ok());
    }

    #[test]
    fn validate_rejects_empty_required_field() {
        let mut def = base("ok");
        def.system_prompt = "  ".to_string();
        let err = validate_config(&settings_with(vec![def])).expect_err("must reject");
        assert!(
            matches!(err, HostBuildError::InvalidAgent { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let settings = settings_with(vec![base("dup"), base("dup")]);
        let err = validate_config(&settings).expect_err("must reject");
        assert!(matches!(err, HostBuildError::DuplicateId(_)), "got {err:?}");
    }

    #[test]
    fn validate_rejects_unknown_provider() {
        let mut def = base("x");
        def.provider = Some("no-such-provider".to_string());
        let err = validate_config(&settings_with(vec![def])).expect_err("must reject");
        assert!(
            matches!(err, HostBuildError::UnknownProvider { .. }),
            "got {err:?}"
        );
    }
}
