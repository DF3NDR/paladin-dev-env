//! Concrete [`AgentProvisioner`] for runtime agent registration (Milestone 12, Epic 2).
//!
//! The HTTP API's `POST /agents` route (in `paladin-web`) delegates to an injected
//! [`AgentProvisioner`] to turn a request [`AgentSpec`] into a `(Paladin, executor)`
//! pair. [`FacadeProvisioner`] is that implementation: it reuses the same
//! [`build_agent`](super::agent_host::build_agent) path as config load, so
//! config-defined and runtime-registered agents are built identically.

use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_llm::provider_factory::LlmProviderFactory;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_web::{AgentProvisioner, AgentSpec, ProvisionError, ProvisionedAgent};

use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
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

/// Adapts a [`PaladinExecutionService`] to the engine-facing [`PaladinPort`] seam
/// `WarEngine::new` expects (Phase 27, PLAT-01/02): the same shape
/// `src/application/services/run/tracer_e2e.rs`'s own `PaladinPortAdapter` establishes for
/// tests, promoted here as the production adapter since none existed outside that test
/// module before this phase.
struct EngineExecutionPort(Arc<PaladinExecutionService>);

#[async_trait]
impl PaladinPort for EngineExecutionPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.0.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        self.0.execute_stream(paladin, input).await
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Build the run engine's real [`PaladinPort`] from `settings` (Phase 27, PLAT-01/02),
/// using the SAME default-provider resolution [`FacadeProvisioner`]/`build_agent` use: no
/// per-node provider hint exists on [`paladin_core::platform::container::paladin::PaladinData`]
/// (a `WarGraphDoc`-defined `Paladin` node carries only a `model` string, never a
/// `provider`), so the single resolved default provider backs every `NodeSpec::Paladin` node
/// the run engine dispatches, mirroring `spec_to_definition`'s own `provider: None` choice
/// for a runtime-provisioned agent.
///
/// Constructed once at boot and shared by the whole run engine's lifetime -- see
/// `src/infrastructure/web/run_api_wiring.rs::build_run_api`, the sole production caller.
///
/// # Errors
///
/// Returns a [`HostBuildError::Provider`] if the resolved default provider cannot be
/// constructed (an unknown provider name, or a missing API key).
pub fn paladin_port_from_settings(
    settings: &Settings,
) -> Result<Arc<dyn PaladinPort>, HostBuildError> {
    let factory = LlmProviderFactory::new();
    let provider = default_provider_name(settings);
    let llm = factory
        .create(&provider)
        .map_err(|source| HostBuildError::Provider {
            id: "run-engine".to_string(),
            provider,
            source,
        })?;
    let service = Arc::new(PaladinExecutionService::new(
        llm,
        default_circuit_breaker(),
        None,
        None,
    ));
    Ok(Arc::new(EngineExecutionPort(service)))
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
        // The per-agent timeout is applied by the web layer (registry entry), not the
        // build path, so it is not mapped onto the definition here.
        timeout_seconds: None,
        // Authorization is enforced by the web layer from `AgentSpec.allowed_roles`; the
        // build path is role-agnostic.
        allowed_roles: Vec::new(),
    }
}

#[async_trait]
impl AgentProvisioner for FacadeProvisioner {
    async fn provision(&self, spec: &AgentSpec) -> Result<ProvisionedAgent, ProvisionError> {
        let def = spec_to_definition(spec);
        let (paladin, executor, streamer) = build_agent(
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
        })?;

        Ok(ProvisionedAgent {
            paladin,
            executor,
            streamer,
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
            timeout_seconds: None,
            allowed_roles: vec![],
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

    #[test]
    fn paladin_port_from_settings_unknown_provider_errors() {
        // Force resolution to fail at the provider factory (hermetic — no API keys, no
        // network): the same failure shape `FacadeProvisioner::provision` maps to
        // `ProvisionError::Failed`, surfaced here as `HostBuildError::Provider`.
        let settings = Settings {
            llm: Some(paladin_llm::config::llm::LlmConfig {
                default_provider: Some("no-such-provider".to_string()),
                ..Default::default()
            }),
            ..Settings::default()
        };

        let err = paladin_port_from_settings(&settings)
            .err()
            .expect("unknown provider must error");
        assert!(
            matches!(err, HostBuildError::Provider { .. }),
            "unknown provider must map to HostBuildError::Provider, got {err:?}"
        );
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
