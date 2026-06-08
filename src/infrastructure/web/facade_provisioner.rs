//! Concrete [`AgentProvisioner`] for runtime agent registration (Milestone 12, Epic 2).
//!
//! The HTTP API's `POST /agents` route (in `paladin-web`) delegates to an injected
//! [`AgentProvisioner`] to turn a request [`AgentSpec`] into a `(Paladin, executor)`
//! pair. [`FacadeProvisioner`] is that implementation: it reuses the same
//! [`build_agent`](super::agent_host::build_agent) path as config load, so
//! config-defined and runtime-registered agents are built identically.

use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::paladin::Paladin;
use paladin_llm::provider_factory::LlmProviderFactory;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_web::{AgentProvisioner, AgentSpec, ProvisionError};

use crate::config::agents::AgentDefinition;
use crate::config::settings::Settings;
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use crate::infrastructure::web::agent_host::{
    HostBuildError, build_agent, default_circuit_breaker, default_provider_name,
};

/// Builds agents at runtime from `POST /agents` request specs, using the facade's
/// LLM provider factory and the shared agent-build path.
pub struct FacadeProvisioner {
    factory: LlmProviderFactory,
    default_provider: String,
    breaker: Arc<CircuitBreaker>,
}

impl FacadeProvisioner {
    /// Create a provisioner with an explicit default provider and circuit breaker.
    pub fn new(default_provider: impl Into<String>, breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            factory: LlmProviderFactory::new(),
            default_provider: default_provider.into(),
            breaker,
        }
    }

    /// Create a provisioner whose defaults match the config-load builder for `settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        Self::new(default_provider_name(settings), default_circuit_breaker())
    }
}

/// Map a runtime [`AgentSpec`] onto the config-shaped [`AgentDefinition`] so both paths
/// share one build implementation.
///
/// `AgentSpec` carries no `provider` or `max_loops`, so the provisioner's default
/// provider applies and the builder default loop count is used.
fn spec_to_definition(spec: &AgentSpec) -> AgentDefinition {
    AgentDefinition {
        id: spec.id.clone(),
        model: spec.model.clone(),
        system_prompt: spec.system_prompt.clone(),
        provider: None,
        temperature: spec.temperature,
        max_loops: None,
        stop_words: spec.stop_words.clone(),
    }
}

#[async_trait]
impl AgentProvisioner for FacadeProvisioner {
    async fn provision(
        &self,
        spec: &AgentSpec,
    ) -> Result<(Paladin, Arc<dyn PaladinExecutorPort>), ProvisionError> {
        let def = spec_to_definition(spec);
        build_agent(
            &def,
            &self.factory,
            &self.default_provider,
            Arc::clone(&self.breaker),
        )
        .await
        .map_err(|err| match &err {
            // A build failure usually means the spec itself is unusable
            // (e.g. an empty prompt rejected by the builder).
            HostBuildError::Build { .. } => ProvisionError::InvalidSpec(err.to_string()),
            // Provider/registration failures are environment/runtime failures.
            _ => ProvisionError::Failed(err.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec(id: &str) -> AgentSpec {
        AgentSpec {
            id: id.to_string(),
            name: "Researcher".to_string(),
            model: "gpt-4".to_string(),
            system_prompt: "You research topics.".to_string(),
            temperature: Some(0.5),
            stop_words: vec!["STOP".to_string()],
        }
    }

    #[test]
    fn spec_maps_onto_definition() {
        let def = spec_to_definition(&sample_spec("researcher"));
        assert_eq!(def.id, "researcher");
        assert_eq!(def.model, "gpt-4");
        assert_eq!(def.system_prompt, "You research topics.");
        assert_eq!(def.temperature, Some(0.5));
        assert_eq!(def.stop_words, vec!["STOP".to_string()]);
        // Spec carries neither; defaults apply.
        assert!(def.provider.is_none());
        assert!(def.max_loops.is_none());
    }

    #[tokio::test]
    async fn provision_unknown_provider_maps_to_provision_error() {
        // Force the build to fail at provider resolution (hermetic — no API keys).
        let provisioner = FacadeProvisioner::new("no-such-provider", default_circuit_breaker());

        // `(Paladin, Arc<dyn PaladinExecutorPort>)` is not `Debug`, so match the result.
        let result = provisioner.provision(&sample_spec("x")).await;
        assert!(
            matches!(result, Err(ProvisionError::Failed(_))),
            "unknown provider must map to ProvisionError::Failed"
        );
    }
}
