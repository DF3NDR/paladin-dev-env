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
use paladin_core::platform::container::heartbeat::HeartbeatHandle;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run_scope::RunScope;
use paladin_llm::cadence::CadenceWiring;
use paladin_llm::provider_factory::LlmProviderFactory;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_web::{AgentProvisioner, AgentSpec, ProvisionError, ProvisionedAgent};

use crate::application::services::paladin::middleware::limits::TokenBudget;
use crate::application::services::paladin::paladin_execution_service::{
    AgentLoopSettlement, PaladinExecutionService,
};
use crate::config::agent_runtime::TokenBudgetConfig;
use crate::config::agents::AgentDefinition;
use crate::config::settings::Settings;
use crate::config::treasurer::TreasurerConfig;
use crate::infrastructure::cadence::{build_cadence, compose_llm};
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
    treasurer: TreasurerConfig,
    /// The rate-pacing wiring every runtime-provisioned agent shares (PACE-02, D-08). Rebuilt
    /// from `treasurer.cadence` by [`FacadeProvisioner::with_treasurer`]; `None` when pacing is
    /// disabled.
    cadence: Option<CadenceWiring>,
    /// Why `with_treasurer` could not build the wiring (for example `backend: redis` on a binary
    /// built without `redis-cadence`), reported by `provision` instead of silently pacing
    /// in-process. Cleared by [`FacadeProvisioner::with_cadence`].
    cadence_error: Option<String>,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    token_budget: TokenBudgetConfig,
}

impl FacadeProvisioner {
    /// Create a provisioner with an explicit default provider and circuit breaker.
    ///
    /// The treasurer configuration defaults to [`TreasurerConfig::default()`] (empty pricing
    /// table, rate pacing on over a fresh in-process gate) -- use
    /// [`FacadeProvisioner::with_treasurer`] to price runtime-provisioned agents.
    /// No treasury ledger writer is installed by default -- use
    /// [`FacadeProvisioner::with_treasury_ledger`] to settle runtime-provisioned agents' calls.
    pub fn new(default_provider: impl Into<String>, breaker: Arc<CircuitBreaker>) -> Self {
        let treasurer = TreasurerConfig::default();
        // The default `treasurer.cadence` is valid by construction, so this is `Some`.
        let cadence = build_cadence(&treasurer.cadence).ok().flatten();
        Self {
            factory: LlmProviderFactory::new(),
            default_provider: default_provider.into(),
            breaker,
            treasurer,
            cadence,
            cadence_error: None,
            treasury_ledger: None,
            token_budget: TokenBudgetConfig::default(),
        }
    }

    /// Create a provisioner whose defaults match the config-load builder for `settings`,
    /// including its `treasurer.pricing` table (PRICE-01, D-09).
    pub fn from_settings(settings: &Settings) -> Self {
        Self::new(default_provider_name(settings), default_circuit_breaker())
            .with_treasurer(settings.get_treasurer_config())
            .with_token_budget(settings.agent_runtime.token_budget.clone())
    }

    /// Set the operator's `agent_runtime.token_budget` that the one
    /// [`TokenBudget`](crate::application::services::paladin::middleware::limits::TokenBudget)
    /// installed on every runtime-provisioned agent's service enforces (D-11, G12) -- the same
    /// figure a config-defined agent carries. The default is the disabled budget, so a
    /// provisioner built with [`FacadeProvisioner::new`] is unchanged until a derived figure
    /// arrives with a call.
    pub fn with_token_budget(mut self, token_budget: TokenBudgetConfig) -> Self {
        self.token_budget = token_budget;
        self
    }

    /// Set the treasurer (pricing) configuration this provisioner's runtime-provisioned
    /// agents are priced from (D-09). An empty table (the default) installs no pricing
    /// decorator on any agent this provisioner builds.
    ///
    /// The rate-pacing wiring is rebuilt from `config.cadence` (PACE-02, D-08) when that
    /// subtree builds; one that does not (invalid, or `backend: redis` without the
    /// `redis-cadence` feature) keeps the previous wiring and is rejected by `provision`
    /// instead, exactly like an invalid price table -- never a silent in-process fallback. A
    /// server composing several ports should share one wiring across them with
    /// [`FacadeProvisioner::with_cadence`] after this call.
    pub fn with_treasurer(mut self, config: TreasurerConfig) -> Self {
        match build_cadence(&config.cadence) {
            Ok(cadence) => {
                self.cadence = cadence;
                self.cadence_error = None;
            }
            Err(reason) => self.cadence_error = Some(reason),
        }
        self.treasurer = config;
        self
    }

    /// Pace every runtime-provisioned agent through the supplied `cadence` wiring (PACE-02,
    /// D-08), replacing the wiring [`FacadeProvisioner::new`] or
    /// [`FacadeProvisioner::with_treasurer`] built. `None` installs no pacing.
    ///
    /// Pass a clone of the ONE [`CadenceWiring`] the process built at boot -- the one handed to
    /// the agent registry and the run engine's port -- so a 429 seen through any of them gates
    /// the others. Call this after [`FacadeProvisioner::with_treasurer`], which would otherwise
    /// rebuild a private wiring.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use paladin::config::settings::Settings;
    /// use paladin::infrastructure::cadence::build_cadence;
    /// use paladin::infrastructure::web::facade_provisioner::FacadeProvisioner;
    ///
    /// # fn demo(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    /// let cadence = build_cadence(&settings.get_treasurer_config().cadence)?;
    /// let provisioner = FacadeProvisioner::from_settings(settings).with_cadence(cadence.clone());
    /// # let _ = provisioner;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_cadence(mut self, cadence: Option<CadenceWiring>) -> Self {
        self.cadence = cadence;
        self.cadence_error = None;
        self
    }

    /// Installs `ledger` as the treasury ledger writer runtime-provisioned agents settle
    /// under (D-08, 39-05): every priced call this provisioner's agents make settles under
    /// [`AgentLoopSettlement::EveryCall`] (`build_agent_with_llm`'s own contract), exactly
    /// like a config-defined agent installed via
    /// [`build_agent_registry_with_ledger`](super::agent_host::build_agent_registry_with_ledger).
    /// `None` (the default) installs no ledger call at all.
    pub fn with_treasury_ledger(mut self, ledger: Arc<dyn TreasuryLedgerPort>) -> Self {
        self.treasury_ledger = Some(ledger);
        self
    }
}

/// Adapts a [`PaladinExecutionService`] to the engine-facing [`PaladinPort`] seam
/// `WarEngine::new` expects (Phase 27, PLAT-01/02): the same shape
/// `src/application/services/run/tracer_e2e.rs`'s own `PaladinPortAdapter` establishes for
/// tests, promoted here as the production adapter since none existed outside that test
/// module before this phase.
pub(crate) struct EngineExecutionPort(Arc<PaladinExecutionService>);

/// Wrap the ONE shared run-engine service as the engine-facing [`PaladinPort`], installing the
/// [`TokenBudget`] in **Treasurer-only mode** (Phase 42, D-11, G12, ADR-0052).
///
/// The shared service backs the worker's agent-kind path *and* every engine node. The operator's
/// `agent_runtime.token_budget` is therefore forced off here (`enabled: false`): the middleware
/// then acts only when a call's [`RunScope`] carries a derived figure, and only the worker's
/// agent-kind dispatch ever sets one. An enabled operator budget on this service would be the
/// hazard ADR-0052 rejected -- it would cap an engine node mid-flight and hand the successful
/// partial result to the Battlefield as if it were a finished answer. An engine node's scope
/// never carries a derived figure, so it is never capped.
///
/// The remaining `token_budget` fields are carried through unchanged so a reader of the
/// installed config sees the operator's own values with only the switch forced off.
pub(crate) fn shared_engine_execution_port(
    service: PaladinExecutionService,
    token_budget: &TokenBudgetConfig,
) -> Arc<dyn PaladinPort> {
    let service = service.with_middleware(Arc::new(TokenBudget::new(TokenBudgetConfig {
        enabled: false,
        ..token_budget.clone()
    })));
    Arc::new(EngineExecutionPort(Arc::new(service)))
}

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

    /// Forwards `scope` to [`PaladinExecutionService::execute_scoped`] (D-07, 39-05) so a
    /// worker-supplied `RunScope` run id (a Platform API run of an agent-kind assistant,
    /// 39-07) reaches this shared service's agent-loop settle writer. `heartbeat` is
    /// forwarded as `None`, exactly the trait's own default behavior -- this override
    /// changes nothing about heartbeat handling, only that `scope` now actually reaches the
    /// service instead of being silently discarded by the trait's default body.
    async fn execute_scoped(
        &self,
        paladin: &Paladin,
        input: &str,
        _heartbeat: &HeartbeatHandle,
        scope: &RunScope,
    ) -> Result<PaladinResult, PaladinError> {
        self.0.execute_scoped(paladin, input, None, scope).await
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
/// Returns a [`HostBuildError::Build`] naming `treasurer.pricing` if the operator's
/// `treasurer.pricing` table is invalid, or naming `treasurer.cadence` if that subtree is
/// invalid (both checked FIRST, before any provider is resolved, so they fail hermetically).
/// Returns a [`HostBuildError::Provider`] if the resolved default provider cannot be
/// constructed (an unknown provider name, or a missing API key).
///
/// Installs no treasury ledger writer -- see [`paladin_port_from_settings_with_ledger`] for
/// the variant that does.
pub fn paladin_port_from_settings(
    settings: &Settings,
) -> Result<Arc<dyn PaladinPort>, HostBuildError> {
    paladin_port_from_settings_with_ledger(settings, None)
}

/// Build the run engine's real [`PaladinPort`] from `settings`, installing `treasury_ledger`
/// (D-08, 39-05) under [`AgentLoopSettlement::PlatformRunsOnly`] when `Some`.
///
/// Identical to [`paladin_port_from_settings`] otherwise. `PlatformRunsOnly` is the mode this
/// SHARED service must use: engine nodes are already settled once per superstep by
/// `WarEngine::with_treasury_ledger` (39-04), so this service settles only calls whose
/// [`RunScope`] names a Platform run id -- an agent-kind assistant's run, dispatched by the
/// worker (39-07) through the internal `EngineExecutionPort::execute_scoped`'s forwarded scope. An
/// engine node's own dispatch (no run id in its scope) never settles here, so engine spend is
/// never double-charged.
///
/// Rate pacing (PACE-02, D-08): the provider is composed as `Pricing(Cadence(provider))` over a
/// wiring built here from `settings.treasurer.cadence`. A server that also paces other ports
/// should build ONE wiring and call [`paladin_port_from_settings_with_cadence`] so this port
/// shares gate state with them.
///
/// # Errors
///
/// Returns a [`HostBuildError::Build`] naming `treasurer.pricing` if the operator's
/// `treasurer.pricing` table is invalid, or naming `treasurer.cadence` if that subtree is
/// invalid or `backend: redis` is configured on a binary built without the `redis-cadence`
/// feature (both checked FIRST, before any provider is resolved, so they fail hermetically).
/// Returns a [`HostBuildError::Provider`] if the resolved default provider cannot be
/// constructed (an unknown provider name, or a missing API key).
pub fn paladin_port_from_settings_with_ledger(
    settings: &Settings,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
) -> Result<Arc<dyn PaladinPort>, HostBuildError> {
    let cadence = build_cadence(&settings.get_treasurer_config().cadence).map_err(|reason| {
        HostBuildError::Build {
            id: "run-engine".to_string(),
            source: PaladinError::ConfigurationError(reason),
        }
    })?;
    paladin_port_from_settings_with_cadence(settings, treasury_ledger, cadence)
}

/// Build the run engine's real [`PaladinPort`] from `settings`, pacing its provider through the
/// supplied `cadence` wiring (PACE-02, D-08).
///
/// Identical to [`paladin_port_from_settings_with_ledger`] otherwise: the provider is composed
/// through [`compose_llm`] as `Pricing(Cadence(provider))`. The caller owns the
/// [`CadenceWiring`]; pass a clone of the ONE wiring the process built at boot (the one given
/// to the agent registry and the provisioner) so a 429 seen through any of those ports gates the
/// run engine's next call to the same provider and model, or `None` to install no pacing.
///
/// # Errors
///
/// As [`paladin_port_from_settings_with_ledger`], except that `settings.treasurer.cadence` is
/// still validated here (hermetically, before any provider is resolved) even though the wiring
/// is supplied, so the port never runs under a configuration the rest of the server rejects.
///
/// # Examples
///
/// ```no_run
/// use paladin::config::settings::Settings;
/// use paladin::infrastructure::cadence::build_cadence;
/// use paladin::infrastructure::web::facade_provisioner::paladin_port_from_settings_with_cadence;
///
/// # fn demo(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
/// let cadence = build_cadence(&settings.get_treasurer_config().cadence)?;
/// let port = paladin_port_from_settings_with_cadence(settings, None, cadence.clone())?;
/// # let _ = port;
/// # Ok(())
/// # }
/// ```
pub fn paladin_port_from_settings_with_cadence(
    settings: &Settings,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    cadence: Option<CadenceWiring>,
) -> Result<Arc<dyn PaladinPort>, HostBuildError> {
    let treasurer = settings.get_treasurer_config();
    let price_table =
        Arc::new(
            treasurer
                .price_table()
                .map_err(|reason| HostBuildError::Build {
                    id: "run-engine".to_string(),
                    source: PaladinError::ConfigurationError(reason),
                })?,
        );
    treasurer
        .cadence
        .validate()
        .map_err(|reason| HostBuildError::Build {
            id: "run-engine".to_string(),
            source: PaladinError::ConfigurationError(reason),
        })?;

    let factory = LlmProviderFactory::new();
    let provider = default_provider_name(settings);
    let llm = factory
        .create(&provider)
        .map_err(|source| HostBuildError::Provider {
            id: "run-engine".to_string(),
            provider,
            source,
        })?;
    let llm = compose_llm(llm, &price_table, cadence.as_ref());
    let mut service = PaladinExecutionService::new(llm, default_circuit_breaker(), None, None);
    if let Some(ledger) = treasury_ledger {
        service = service.with_treasury_ledger(ledger, AgentLoopSettlement::PlatformRunsOnly);
    }
    Ok(shared_engine_execution_port(
        service,
        &settings.agent_runtime.token_budget,
    ))
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
        let price_table = Arc::new(self.treasurer.price_table().map_err(|reason| {
            ProvisionError::Failed(format!("invalid treasurer configuration: {reason}"))
        })?);
        self.treasurer.cadence.validate().map_err(|reason| {
            ProvisionError::Failed(format!("invalid treasurer configuration: {reason}"))
        })?;
        if let Some(reason) = &self.cadence_error {
            return Err(ProvisionError::Failed(format!(
                "invalid treasurer configuration: {reason}"
            )));
        }
        let def = spec_to_definition(spec);
        let (paladin, executor, streamer) = build_agent(
            &def,
            &self.factory,
            &self.default_provider,
            Arc::clone(&self.breaker),
            &price_table,
            self.treasury_ledger.clone(),
            self.token_budget.clone(),
            self.cadence.as_ref(),
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

    fn invalid_treasurer() -> TreasurerConfig {
        let mut treasurer = TreasurerConfig::default();
        treasurer.pricing.insert(
            "gpt-4".to_string(),
            crate::config::PriceRowConfig {
                prompt: "-1".to_string(),
                completion: "1.00".to_string(),
                cache_read: None,
                cache_write: None,
                reasoning: None,
            },
        );
        treasurer
    }

    #[test]
    fn paladin_port_from_settings_rejects_invalid_treasurer_price() {
        // Checked BEFORE any provider is resolved, so this fails hermetically even with no
        // API keys and no default provider configured beyond Settings::default().
        let settings = Settings {
            treasurer: invalid_treasurer(),
            ..Settings::default()
        };

        let err = paladin_port_from_settings(&settings)
            .err()
            .expect("invalid treasurer price must error");
        assert!(matches!(err, HostBuildError::Build { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("treasurer.pricing.gpt-4.prompt"),
            "error must name the offending config path: {err}"
        );
    }

    #[tokio::test]
    async fn provisioner_rejects_invalid_treasurer_price() {
        let provisioner = FacadeProvisioner::new("openai", default_circuit_breaker())
            .with_treasurer(invalid_treasurer());

        // `ProvisionedAgent` is not `Debug` (it carries a `Paladin`/`Arc<dyn ...>`), so match
        // on the result rather than `expect`/`unwrap` the whole `Result`.
        let result = provisioner.provision(&sample_spec("x")).await;
        assert!(
            matches!(result, Err(ProvisionError::Failed(_))),
            "invalid treasurer price must map to ProvisionError::Failed"
        );
        if let Err(ProvisionError::Failed(msg)) = result {
            assert!(
                msg.contains("invalid treasurer configuration"),
                "got {msg:?}"
            );
        }
    }

    #[tokio::test]
    async fn provisioner_rejects_invalid_cadence_naming_its_key() {
        let mut treasurer = TreasurerConfig::default();
        treasurer.cadence.max_wait_secs = 0;
        let provisioner =
            FacadeProvisioner::new("openai", default_circuit_breaker()).with_treasurer(treasurer);

        let result = provisioner.provision(&sample_spec("x")).await;
        assert!(
            matches!(result, Err(ProvisionError::Failed(_))),
            "invalid treasurer.cadence must map to ProvisionError::Failed"
        );
        if let Err(ProvisionError::Failed(msg)) = result {
            assert!(
                msg.contains("treasurer.cadence.max_wait_secs"),
                "got {msg:?}"
            );
        }
    }

    #[test]
    fn provisioner_paces_by_default_and_follows_the_treasurer_cadence() {
        let default = FacadeProvisioner::new("openai", default_circuit_breaker());
        assert!(default.cadence.is_some(), "pacing is on by default (D-08)");

        let mut off = TreasurerConfig::default();
        off.cadence.enabled = false;
        let off = default.with_treasurer(off);
        assert!(off.cadence.is_none(), "enabled: false installs nothing");

        let mut invalid = TreasurerConfig::default();
        invalid.cadence.lock_ttl_secs = 0;
        let kept =
            FacadeProvisioner::new("openai", default_circuit_breaker()).with_treasurer(invalid);
        assert!(
            kept.cadence.is_some(),
            "an invalid subtree keeps the previous wiring; provision rejects it instead"
        );
    }

    #[test]
    fn provisioner_with_cadence_uses_the_supplied_wiring() {
        let shared = build_cadence(&TreasurerConfig::default().cadence)
            .expect("default cadence builds")
            .expect("pacing is on by default");

        // The supplied wiring replaces the private one `new` built: same port instance.
        let provisioner = FacadeProvisioner::new("openai", default_circuit_breaker())
            .with_cadence(Some(shared.clone()));
        let installed = provisioner.cadence.as_ref().expect("wiring installed");
        assert!(Arc::ptr_eq(installed.port(), shared.port()));

        // It also wins over the wiring `with_treasurer` rebuilds, when called after it.
        let provisioner = FacadeProvisioner::new("openai", default_circuit_breaker())
            .with_treasurer(TreasurerConfig::default())
            .with_cadence(Some(shared.clone()));
        let installed = provisioner.cadence.as_ref().expect("wiring installed");
        assert!(Arc::ptr_eq(installed.port(), shared.port()));

        // `None` installs nothing.
        let provisioner =
            FacadeProvisioner::new("openai", default_circuit_breaker()).with_cadence(None);
        assert!(provisioner.cadence.is_none());
    }

    /// An invalid `treasurer.cadence` is rejected hermetically, before any provider is resolved,
    /// by BOTH engine-port entry points -- naming the key, exactly like an invalid price.
    #[test]
    fn paladin_port_from_settings_with_ledger_rejects_an_invalid_cadence_config_before_resolving_a_provider()
     {
        let mut settings = Settings::default();
        settings.treasurer.cadence.max_wait_secs = 0;

        let err = paladin_port_from_settings_with_ledger(&settings, None)
            .err()
            .expect("an invalid treasurer.cadence must error");
        assert!(matches!(err, HostBuildError::Build { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("treasurer.cadence.max_wait_secs"),
            "error must name the offending config path: {err}"
        );

        // The supplied-wiring variant re-validates, so it never runs under a rejected config.
        let err = paladin_port_from_settings_with_cadence(&settings, None, None)
            .err()
            .expect("an invalid treasurer.cadence must error");
        assert!(matches!(err, HostBuildError::Build { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("treasurer.cadence.max_wait_secs"),
            "got {err}"
        );
    }

    /// `None` (pacing disabled) is a valid wiring: the build proceeds to resolving the provider,
    /// which is where this hermetic settings fails -- proving the disabled path is not rejected
    /// earlier and composes nothing but pricing.
    #[test]
    fn disabled_cadence_composes_pricing_only_at_every_site() {
        let settings = Settings {
            llm: Some(paladin_llm::config::llm::LlmConfig {
                default_provider: Some("no-such-provider".to_string()),
                ..Default::default()
            }),
            ..Settings::default()
        };
        let err = paladin_port_from_settings_with_cadence(&settings, None, None)
            .err()
            .expect("unknown provider must error");
        assert!(
            matches!(err, HostBuildError::Provider { .. }),
            "got {err:?}"
        );

        let mut off = TreasurerConfig::default();
        off.cadence.enabled = false;
        assert!(
            build_cadence(&off.cadence)
                .expect("disabled builds")
                .is_none()
        );
        let provisioner = FacadeProvisioner::new("openai", default_circuit_breaker())
            .with_treasurer(off)
            .with_cadence(None);
        assert!(provisioner.cadence.is_none());

        // The composition itself: no pricing rows and no wiring leaves the provider untouched.
        let llm: Arc<dyn paladin_ports::output::llm_port::LlmPort> =
            Arc::new(paladin_llm::mock::MockLlmAdapter::new());
        let empty = Arc::new(paladin_core::platform::container::cost::PriceTable::new(
            paladin_core::platform::container::cost::CurrencyCode::new("USD").expect("USD"),
        ));
        assert!(Arc::ptr_eq(&compose_llm(llm.clone(), &empty, None), &llm));
    }

    /// `backend: redis` on a binary without `redis-cadence` is never a silent in-process
    /// fallback -- the provisioner (which cannot fail in `with_treasurer`) rejects every
    /// `provision` instead.
    #[cfg(not(feature = "redis-cadence"))]
    #[tokio::test]
    #[serial_test::serial]
    async fn provisioner_without_the_feature_rejects_a_redis_backend_instead_of_pacing_in_process()
    {
        const VAR: &str = "PALADIN_TEST_PROVISIONER_CADENCE_URL";
        unsafe {
            std::env::set_var(VAR, "redis://:hunter2@127.0.0.1:1/0");
        }
        let mut treasurer = TreasurerConfig::default();
        treasurer.cadence.backend = crate::config::treasurer::CadenceBackend::Redis {
            url_env: VAR.to_string(),
        };
        let provisioner =
            FacadeProvisioner::new("openai", default_circuit_breaker()).with_treasurer(treasurer);
        let result = provisioner.provision(&sample_spec("x")).await;
        unsafe {
            std::env::remove_var(VAR);
        }
        match result {
            Err(ProvisionError::Failed(msg)) => {
                assert!(msg.contains("redis-cadence"), "got {msg:?}");
                assert!(!msg.contains("hunter2"), "got {msg:?}");
            }
            Err(_) => panic!("expected ProvisionError::Failed"),
            Ok(_) => panic!("a redis backend without the feature must not provision"),
        }
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

    fn make_engine_paladin() -> Paladin {
        use paladin_core::base::entity::node::Node;
        use paladin_core::platform::container::paladin::{MaxLoops, PaladinData};

        let data = PaladinData {
            system_prompt: "system".to_string(),
            model: "gpt-4".to_string(),
            max_loops: MaxLoops::Fixed(1),
            ..Default::default()
        };
        Node::new(data, None)
    }

    /// D-07 (39-05): `EngineExecutionPort::execute_scoped` forwards `scope` to the wrapped
    /// `PaladinExecutionService::execute_scoped` -- a `PlatformRunsOnly` service settles a
    /// call whose `RunScope` names a run id, and settles nothing for a plain,
    /// unscoped `execute` call.
    #[tokio::test]
    async fn engine_execution_port_forwards_the_run_scope_to_the_ledger() {
        use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
        use paladin_core::platform::container::run::RunId;
        use paladin_core::platform::container::token_usage::TokenUsage;
        use paladin_core::platform::container::treasury_ledger::SpendQuery;
        use paladin_llm::mock::MockLlmAdapter;
        use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

        let table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ));
        let mock: Arc<dyn paladin_ports::output::llm_port::LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi there")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let llm = compose_llm(mock, &table, None);
        let ledger = Arc::new(InMemoryTreasuryLedger::new());
        let service = PaladinExecutionService::new(llm, default_circuit_breaker(), None, None)
            .with_treasury_ledger(ledger.clone(), AgentLoopSettlement::PlatformRunsOnly);
        let engine_port = EngineExecutionPort(Arc::new(service));
        let paladin = make_engine_paladin();

        // A plain, unscoped call never settles under PlatformRunsOnly.
        engine_port
            .execute(&paladin, "hi")
            .await
            .expect("plain execute succeeds");
        let empty = ledger
            .spend(SpendQuery::default())
            .await
            .expect("spend query succeeds");
        assert!(
            empty.is_empty(),
            "an engine node's own dispatch (no scope run id) must never settle here"
        );

        // A scoped call whose RunScope names a run id settles under it.
        let run_id = RunId::new_v7();
        let scope = RunScope::default().with_run_id(run_id.clone());
        let heartbeat = HeartbeatHandle::new();
        engine_port
            .execute_scoped(&paladin, "hi", &heartbeat, &scope)
            .await
            .expect("scoped execute succeeds");
        let rows = ledger
            .spend(SpendQuery {
                group_by: paladin_core::platform::container::treasury_ledger::SpendGroupBy::Run,
                run_ids: vec![run_id],
                ..Default::default()
            })
            .await
            .expect("spend by run succeeds");
        assert_eq!(
            rows.len(),
            1,
            "the scoped call must settle under its run id"
        );
    }
    fn looping_engine_paladin() -> Paladin {
        use paladin_core::base::entity::node::Node;
        use paladin_core::platform::container::paladin::{MaxLoops, PaladinData};

        Node::new(
            PaladinData {
                system_prompt: "system".to_string(),
                model: "gpt-4".to_string(),
                max_loops: MaxLoops::Fixed(10),
                ..Default::default()
            },
            None,
        )
    }

    /// A derived budget of `max_tokens` over a fixed binding ceiling.
    fn derived_scope(max_tokens: u32) -> RunScope {
        use chrono::{TimeZone, Utc};
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind, DerivedTokenBudget,
        };
        use paladin_core::platform::container::cost::{Cost, CurrencyCode};
        let usd = CurrencyCode::new("USD").expect("USD is valid");
        let figures = AllowanceRefusal {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Lifetime,
            balance: Cost::new(5_000, usd.clone()),
            ceiling: Cost::new(5_000, usd),
            window: None,
            evaluated_at: Utc
                .with_ymd_and_hms(2026, 10, 6, 12, 0, 0)
                .single()
                .expect("valid instant"),
        };
        RunScope::default().with_derived_token_budget(DerivedTokenBudget::new(max_tokens, figures))
    }

    /// G12 (ADR-0052): the shared service built as production builds it never caps an engine
    /// node. The operator's budget is enabled with a tiny figure, yet a call whose scope carries
    /// no derived figure runs to `max_loops` (a cut would hand a partial result to a
    /// Battlefield); the control shows the same installed middleware does cut a scope that
    /// carries a derived figure, so the zero effect is not a missing installation.
    #[tokio::test]
    async fn shared_service_never_caps_an_engine_node() {
        use paladin_core::platform::container::execution_result::StopReason;
        use paladin_llm::mock::MockLlmAdapter;

        let operator = TokenBudgetConfig {
            enabled: true,
            max_tokens: 1,
        };
        let build = |llm: Arc<MockLlmAdapter>| {
            let service = PaladinExecutionService::new(llm, default_circuit_breaker(), None, None);
            shared_engine_execution_port(service, &operator)
        };
        let scripted = || {
            Arc::new(
                MockLlmAdapter::new()
                    .with_response("chunk")
                    .with_token_usage(0, 100, 100),
            )
        };
        let paladin = looping_engine_paladin();

        // An engine node: no derived figure on its scope, operator figure enabled at 1 token.
        let llm = scripted();
        let port = build(llm.clone());
        let result = port
            .execute_scoped(
                &paladin,
                "hi",
                &HeartbeatHandle::new(),
                &RunScope::default(),
            )
            .await
            .expect("an engine node's call succeeds");
        assert_eq!(llm.call_count(), 10, "only max_loops ends an engine node");
        assert!(
            !matches!(
                result.stop_reason,
                StopReason::TokenBudget | StopReason::AllowanceHalted(_)
            ),
            "an engine node is never capped: {:?}",
            result.stop_reason
        );
        assert!(
            !result.output.contains("budget"),
            "no truncation notice reaches an engine node's output"
        );

        // Control: the same installed middleware cuts a scope carrying a derived figure.
        let llm = scripted();
        let port = build(llm.clone());
        let result = port
            .execute_scoped(&paladin, "hi", &HeartbeatHandle::new(), &derived_scope(150))
            .await
            .expect("an agent-kind call succeeds");
        assert_eq!(llm.call_count(), 2);
        assert!(matches!(result.stop_reason, StopReason::AllowanceHalted(_)));
    }
}
