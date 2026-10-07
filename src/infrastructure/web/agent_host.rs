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

use paladin_core::platform::container::cost::PriceTable;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::user::UserRole;
use paladin_llm::pricing::with_pricing;
use paladin_llm::provider_factory::{LlmProviderFactory, ProviderFactoryError};
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_web::{AgentEntry, AgentRegistry};

use crate::application::services::paladin::middleware::limits::TokenBudget;
use crate::application::services::paladin::paladin_builder::PaladinBuilder;
use crate::application::services::paladin::paladin_execution_service::{
    AgentLoopSettlement, PaladinExecutionService,
};
use crate::config::agent_runtime::TokenBudgetConfig;
use crate::config::agents::AgentDefinition;
use crate::config::settings::Settings;
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;

/// A built agent: the agent plus its buffered and (optional) streaming executors.
pub(crate) type BuiltAgent = (
    Paladin,
    Arc<dyn PaladinExecutorPort>,
    Option<Arc<dyn StreamingExecutorPort>>,
);

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
///
/// `treasury_ledger`, when `Some`, installs [`AgentLoopSettlement::EveryCall`] (D-08, 39-05)
/// on the ONE shared [`PaladinExecutionService`] BEFORE it is split into the buffered and
/// streaming handles below -- so every priced call this agent makes, buffered or streamed,
/// settles under this same ledger, regardless of which handle a caller reaches it through.
///
/// `token_budget` is the operator's `agent_runtime.token_budget` (D-11, G12, ALLOW-05). It
/// installs the ONE [`TokenBudget`] on the same shared service, so the agent loop of every
/// config-defined and runtime-provisioned agent enforces the tighter of the operator's figure
/// and the Treasurer's per-run derived figure (tightest wins, a tie goes to the allowance). The
/// middleware is inert unless the operator enabled it or a call carries a derived figure, so a
/// deployment with neither is unchanged. The operator's figure therefore now takes effect on
/// the HTTP agent routes -- before Phase 42 only the preset path honoured it. The shared run
/// engine's service is built elsewhere and runs Treasurer-only (plan 42-09).
pub(crate) async fn build_agent_with_llm(
    def: &AgentDefinition,
    llm: Arc<dyn LlmPort>,
    breaker: Arc<CircuitBreaker>,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    token_budget: TokenBudgetConfig,
) -> Result<BuiltAgent, HostBuildError> {
    // One execution service backs both the buffered and streaming handles
    // (`PaladinExecutionService` implements both `PaladinExecutorPort` and
    // `StreamingExecutorPort`).
    let mut service = PaladinExecutionService::new(Arc::clone(&llm), breaker, None, None);
    if let Some(ledger) = treasury_ledger {
        service = service.with_treasury_ledger(ledger, AgentLoopSettlement::EveryCall);
    }
    // The one cutoff, installed before the service is split into its two handles so the
    // buffered and streaming paths share it (inert without an operator or derived figure).
    service = service.with_middleware(Arc::new(TokenBudget::new(token_budget)));
    let service = Arc::new(service);
    let executor: Arc<dyn PaladinExecutorPort> = service.clone();
    let streamer: Arc<dyn StreamingExecutorPort> = service;

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

    Ok((paladin, executor, Some(streamer)))
}

/// Build a `(Paladin, executor)` pair from a definition, resolving the provider via the
/// factory. Shared by config load and runtime provisioning.
///
/// `price_table` wraps the resolved provider with [`with_pricing`] (D-09, ADR-0052) BEFORE
/// `build_agent_with_llm` composes the execution service, outside any fallback composition --
/// an empty table installs no extra layer at all. `treasury_ledger` is threaded straight
/// through to `build_agent_with_llm` (D-08, 39-05), as is `token_budget` -- the operator's
/// `agent_runtime.token_budget`, from which the one [`TokenBudget`] is installed on the agent's
/// service (D-11, G12).
pub(crate) async fn build_agent(
    def: &AgentDefinition,
    factory: &LlmProviderFactory,
    default_provider: &str,
    breaker: Arc<CircuitBreaker>,
    price_table: &Arc<PriceTable>,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
    token_budget: TokenBudgetConfig,
) -> Result<BuiltAgent, HostBuildError> {
    let provider = resolve_provider(def, default_provider);
    let llm = factory
        .create(&provider)
        .map_err(|source| HostBuildError::Provider {
            id: def.id.clone(),
            provider,
            source,
        })?;
    let llm = with_pricing(llm, price_table);
    build_agent_with_llm(def, llm, breaker, treasury_ledger, token_budget).await
}

/// Insert a built agent into the registry, rejecting a duplicate id.
#[allow(clippy::too_many_arguments)]
pub(crate) fn register_built(
    registry: &AgentRegistry,
    id: &str,
    paladin: Paladin,
    executor: Arc<dyn PaladinExecutorPort>,
    streamer: Option<Arc<dyn StreamingExecutorPort>>,
    timeout_secs: Option<u64>,
    allowed_roles: Vec<UserRole>,
) -> Result<(), HostBuildError> {
    let inserted = registry.insert_entry(
        id.to_string(),
        AgentEntry {
            paladin: Arc::new(paladin),
            executor,
            streamer,
            timeout_secs,
            allowed_roles,
        },
    );
    if inserted {
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
/// checked when the provider is actually created in `build_agent`.
///
/// # Errors
///
/// Returns the first [`HostBuildError`] encountered.
pub fn validate_config(settings: &Settings) -> Result<(), HostBuildError> {
    // Pass 1 — structural checks (key-independent): non-empty required fields and
    // unique ids. These are checked first so a duplicate/empty-field error is reported
    // before any provider/environment concern, and deterministically regardless of which
    // API keys happen to be set.
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
    }

    // Pass 2 — provider availability. `list_available_providers` is gated on the
    // presence of each provider's API key, so this also catches a missing key for the
    // configured provider (the message lists what is available).
    let default_provider = default_provider_name(settings);
    let available = LlmProviderFactory::list_available_providers();
    for def in &settings.agents {
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
/// Runs [`validate_config`] first (fail-fast), then builds the operator's `treasurer.pricing`
/// table (PRICE-01) -- an invalid price aborts here, before any agent or provider is built --
/// and constructs each agent via `build_agent`, priced by that table. A validation failure, an
/// invalid treasurer configuration, an unresolvable provider, or a build failure aborts with a
/// descriptive [`HostBuildError`] naming the agent (or `"treasurer"` for a pricing failure).
///
/// Installs no treasury ledger writer -- see
/// [`build_agent_registry_with_ledger`] for the variant that does.
///
/// # Errors
///
/// Returns [`HostBuildError`] on the first problem encountered.
pub async fn build_agent_registry(settings: &Settings) -> Result<AgentRegistry, HostBuildError> {
    build_agent_registry_with_ledger(settings, None).await
}

/// Build a populated [`AgentRegistry`] from the `agents` section of `settings`, installing
/// `treasury_ledger` (D-08, 39-05) on every configured agent's execution service.
///
/// Identical to [`build_agent_registry`] otherwise -- same validation, same
/// `treasurer.pricing` table build, same per-agent construction via `build_agent` -- except
/// every agent's [`PaladinExecutionService`] settles under
/// [`AgentLoopSettlement::EveryCall`](crate::application::services::paladin::paladin_execution_service::AgentLoopSettlement::EveryCall)
/// when `treasury_ledger` is `Some`. The ledger is observational (D-08, LEDGR-03): a settle
/// failure never fails an agent's execution. Every priced call of every configured agent
/// settles under its own execution id and, for a call made through the HTTP agent routes
/// (`POST /v1/agents/{id}/execute`, its `/stream` variant and `/jobs`), under the calling
/// principal's tenant and API key id -- the handlers pass `principal.ledger_scope()` through
/// `PaladinExecutorPort::execute_scoped` / `StreamingExecutorPort::execute_stream_scoped`
/// (Phase 40 D-16). The [`LedgerScope::unattributed`](
/// paladin_core::platform::container::treasury_ledger::LedgerScope::unattributed) sentinel
/// appears only for a caller that passes no scope at all (embedded library use of the
/// plain `execute`/`execute_stream` methods, D-10).
///
/// # Errors
///
/// Returns [`HostBuildError`] on the first problem encountered.
pub async fn build_agent_registry_with_ledger(
    settings: &Settings,
    treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>,
) -> Result<AgentRegistry, HostBuildError> {
    validate_config(settings)?;

    let factory = LlmProviderFactory::new();
    let default_provider = default_provider_name(settings);
    let breaker = default_circuit_breaker();
    let price_table = Arc::new(settings.get_treasurer_config().price_table().map_err(
        |reason| HostBuildError::Build {
            id: "treasurer".to_string(),
            source: PaladinError::ConfigurationError(reason),
        },
    )?);

    let registry = AgentRegistry::new();
    for def in &settings.agents {
        let (paladin, executor, streamer) = build_agent(
            def,
            &factory,
            &default_provider,
            Arc::clone(&breaker),
            &price_table,
            treasury_ledger.clone(),
            settings.agent_runtime.token_budget.clone(),
        )
        .await?;
        register_built(
            &registry,
            &def.id,
            paladin,
            executor,
            streamer,
            def.timeout_seconds,
            def.allowed_roles.clone(),
        )?;
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::cost::{CurrencyCode, PriceRow};
    use paladin_core::platform::container::token_usage::TokenUsage;
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;

    fn empty_price_table() -> Arc<PriceTable> {
        Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()))
    }

    fn base(id: &str) -> AgentDefinition {
        AgentDefinition {
            id: id.to_string(),
            model: "gpt-4".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            provider: None,
            temperature: None,
            max_loops: None,
            stop_words: vec![],
            timeout_seconds: None,
            allowed_roles: vec![],
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

        let (paladin, _executor, streamer) = build_agent_with_llm(
            &def,
            mock_llm(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");

        assert_eq!(paladin.node.name, "researcher");
        assert_eq!(paladin.node.model, "gpt-4o");
        assert!(streamer.is_some(), "execution service is streaming-capable");
    }

    #[tokio::test]
    async fn build_agent_unknown_provider_errors() {
        let mut def = base("x");
        def.provider = Some("no-such-provider".to_string());
        let factory = LlmProviderFactory::new();

        // Note: `(Paladin, Arc<dyn PaladinExecutorPort>)` is not `Debug`, so we match on
        // the result rather than using `expect_err`.
        let result = build_agent(
            &def,
            &factory,
            "openai",
            default_circuit_breaker(),
            &empty_price_table(),
            None,
            TokenBudgetConfig::default(),
        )
        .await;
        assert!(
            matches!(result, Err(HostBuildError::Provider { .. })),
            "unknown provider must yield a Provider error"
        );
    }

    #[tokio::test]
    async fn build_agent_registry_rejects_invalid_treasurer_price() {
        let mut settings = Settings::default(); // agents is empty
        settings.treasurer.pricing.insert(
            "gpt-4".to_string(),
            crate::config::PriceRowConfig {
                prompt: "-1".to_string(),
                completion: "1.00".to_string(),
                cache_read: None,
                cache_write: None,
                reasoning: None,
            },
        );

        let err = build_agent_registry(&settings)
            .await
            .err()
            .expect("an invalid treasurer price must abort registry build");
        assert!(matches!(err, HostBuildError::Build { .. }), "got {err:?}");
        assert!(
            err.to_string().contains("treasurer.pricing.gpt-4.prompt"),
            "error must name the offending config path: {err}"
        );
    }

    #[tokio::test]
    async fn priced_agent_stream_reports_cost() {
        let table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ));
        let mock: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("streamed")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let priced = with_pricing(mock, &table);

        let (paladin, _executor, streamer) = build_agent_with_llm(
            &base("gpt-4"),
            priced,
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");
        let streamer = streamer.expect("execution service is streaming-capable");

        let mut stream = streamer
            .execute_stream(&paladin, "hi")
            .await
            .expect("stream starts");
        let metadata = loop {
            let item = stream.recv().await.expect("stream must emit a final chunk");
            let chunk = item.expect("chunk must not error");
            if chunk.is_final {
                break chunk
                    .metadata
                    .expect("the final chunk must carry ChunkMetadata");
            }
        };

        let cost = metadata
            .cost
            .as_ref()
            .expect("the exact composition build_agent performs must carry a cost");
        assert_eq!(cost.nanos(), 22_500_000);
        assert_eq!(cost.currency().as_str(), "USD");
    }

    #[tokio::test]
    async fn register_built_rejects_duplicate_id() {
        let registry = AgentRegistry::new();

        let (p1, e1, s1) = build_agent_with_llm(
            &base("dup"),
            mock_llm(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .unwrap();
        register_built(&registry, "dup", p1, e1, s1, None, Vec::new()).expect("first insert ok");

        let (p2, e2, s2) = build_agent_with_llm(
            &base("dup"),
            mock_llm(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .unwrap();
        let err = register_built(&registry, "dup", p2, e2, s2, None, Vec::new())
            .expect_err("duplicate must error");
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

    /// D-08 (39-05): a ledger installed via `build_agent_with_llm`'s `treasury_ledger`
    /// parameter settles a priced call made through the returned executor.
    #[tokio::test]
    async fn build_agent_with_llm_settles_priced_calls_when_a_ledger_is_installed() {
        let table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ));
        let mock: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi there")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let priced = with_pricing(mock, &table);
        let ledger = Arc::new(InMemoryTreasuryLedger::new());

        let (paladin, executor, _streamer) = build_agent_with_llm(
            &base("gpt-4"),
            priced,
            default_circuit_breaker(),
            Some(ledger.clone()),
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");

        executor
            .execute(&paladin, "hi")
            .await
            .expect("execution succeeds");

        let rows = ledger
            .spend(paladin_core::platform::container::treasury_ledger::SpendQuery::default())
            .await
            .expect("spend query succeeds");
        assert_eq!(rows.len(), 1, "the priced call must have settled");
    }

    /// The converse: with no `treasury_ledger` argument, an otherwise-identical priced call
    /// performs no ledger call at all.
    #[tokio::test]
    async fn build_agent_with_llm_without_a_ledger_settles_nothing() {
        let table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ));
        let mock: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi there")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let priced = with_pricing(mock, &table);

        let (paladin, executor, _streamer) = build_agent_with_llm(
            &base("gpt-4"),
            priced,
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");

        let result = executor
            .execute(&paladin, "hi")
            .await
            .expect("execution succeeds");
        assert!(
            result.cost.is_some(),
            "pricing is still installed even with no ledger"
        );
        // No ledger was installed at all -- nothing to assert against a store; this test's
        // purpose is documented by its name and the absence of any ledger construction here.
    }

    // -- The one cutoff on every per-agent service (D-11, G12, ALLOW-05) ----------------------

    /// A looping agent over a mock that reports 100 tokens per response, so cumulative usage
    /// is 100, 200, 300 ... and nothing but a budget or `max_loops` ends the run.
    fn looping_agent_def(id: &str) -> AgentDefinition {
        AgentDefinition {
            max_loops: Some(10),
            ..base(id)
        }
    }

    fn hundred_token_llm() -> Arc<MockLlmAdapter> {
        Arc::new(
            MockLlmAdapter::new()
                .with_response("chunk")
                .with_token_usage(0, 100, 100),
        )
    }

    fn derived_scope(max_tokens: u32) -> paladin_core::platform::container::run_scope::RunScope {
        use chrono::{TimeZone, Utc};
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind, DerivedTokenBudget,
        };
        use paladin_core::platform::container::cost::Cost;
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
        paladin_core::platform::container::run_scope::RunScope::default()
            .with_derived_token_budget(DerivedTokenBudget::new(max_tokens, figures))
    }

    /// With the default (disabled) operator budget and no derived figure on the call, the
    /// installed middleware never cuts: the run goes past any token count.
    #[tokio::test]
    async fn built_agent_without_any_budget_runs_as_before() {
        use paladin_core::platform::container::execution_result::StopReason;
        let llm = hundred_token_llm();
        let (paladin, executor, _streamer) = build_agent_with_llm(
            &looping_agent_def("plain"),
            llm.clone(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");

        let result = executor
            .execute(&paladin, "hi")
            .await
            .expect("execution succeeds");

        assert_eq!(llm.call_count(), 10, "only max_loops ends the run");
        assert_eq!(result.usage.total_tokens, 1_000);
        assert!(!matches!(
            result.stop_reason,
            StopReason::TokenBudget | StopReason::AllowanceHalted(_)
        ));
    }

    /// A call carrying a derived figure is cut after the crossing response by the one
    /// middleware `build_agent_with_llm` installed -- with the operator budget disabled.
    #[tokio::test]
    async fn built_agent_cuts_a_derived_budget() {
        use paladin_core::platform::container::execution_result::StopReason;
        let llm = hundred_token_llm();
        let (paladin, executor, _streamer) = build_agent_with_llm(
            &looping_agent_def("derived"),
            llm.clone(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig::default(),
        )
        .await
        .expect("builds");

        let result = executor
            .execute_scoped(&paladin, "hi", &derived_scope(150))
            .await
            .expect("execution succeeds");

        assert_eq!(
            llm.call_count(),
            2,
            "cumulative 100 continues, 200 crosses 150"
        );
        assert!(matches!(result.stop_reason, StopReason::AllowanceHalted(_)));
        assert!(!result.stop_reason.is_successful());
    }

    /// The operator's `agent_runtime.token_budget` now takes effect on a built agent: below the
    /// scripted usage, with no derived figure on the call, the run ends with `TokenBudget`.
    #[tokio::test]
    async fn built_agent_honours_the_operator_budget() {
        use paladin_core::platform::container::execution_result::StopReason;
        let llm = hundred_token_llm();
        let (paladin, executor, _streamer) = build_agent_with_llm(
            &looping_agent_def("operator"),
            llm.clone(),
            default_circuit_breaker(),
            None,
            TokenBudgetConfig {
                enabled: true,
                max_tokens: 150,
            },
        )
        .await
        .expect("builds");

        let result = executor
            .execute(&paladin, "hi")
            .await
            .expect("execution succeeds");

        assert_eq!(llm.call_count(), 2);
        assert_eq!(result.stop_reason, StopReason::TokenBudget);
    }
    // -- End to end: the real agent router, the real agent service, a priced Treasurer ---------

    /// The mock-priced table the end-to-end test derives from: `gpt-4` costs 30 USD per 1M
    /// tokens on its dearest (completion) axis; `mystery` has no row.
    fn e2e_prices() -> Arc<PriceTable> {
        Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(10_000_000_000, 30_000_000_000).unwrap(),
        ))
    }

    /// `x-api-key` map for the principals the end-to-end test calls as.
    fn e2e_auth(keys: &[(&str, &str)]) -> paladin_web::AgentAuthConfig {
        let api_keys = keys
            .iter()
            .map(|(secret, name)| {
                (
                    (*secret).to_string(),
                    paladin_web::Principal::new(
                        *name,
                        paladin_core::platform::container::user::UserRole::User,
                        paladin_core::platform::container::principal::TenantId::new("acme")
                            .expect("tenant id"),
                    ),
                )
            })
            .collect();
        paladin_web::AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        }
    }

    async fn post_json(
        app: &axum::Router,
        uri: &str,
        key: &str,
    ) -> (axum::http::StatusCode, serde_json::Value, String) {
        use tower::ServiceExt;
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .header("x-api-key", key)
                    .body(axum::body::Body::from(r#"{"input":"hello"}"#))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let raw = String::from_utf8(bytes.to_vec()).expect("utf8 body");
        let json = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
        (status, json, raw)
    }

    /// ALLOW-05 over HTTP, end to end (D-09 to D-13): a real `build_agent_with_llm` agent over a
    /// scripted model that reports 100 tokens per response, behind the real agent router and a
    /// real Treasurer with a price table and an in-memory ledger.
    ///
    /// - `svc-a` has a 4_500_000-nano ceiling: at 30 USD per 1M tokens that derives 150 tokens,
    ///   so the run is cut on response 2 (cumulative 200 > 150) with `allowance_halted`, the
    ///   partial output and the truncation notice;
    /// - the same allowance with a smaller operator budget stops as `token_budget`, no
    ///   `halt_reason`;
    /// - `svc-z`'s 29_999-nano ceiling derives 0 tokens: `429 allowance_exhausted`;
    /// - an unpriced model under a ceiling is `422 model_unpriced`;
    /// - `svc-free` has no ceiling and is unaffected;
    /// - `jobs` records the same halt on the job result.
    #[tokio::test]
    async fn agent_execute_halts_on_the_derived_budget() {
        use crate::application::services::treasurer::{AllowancePolicy, ScopeAllowance, Treasurer};
        use paladin_web::{AgentApiState, agent_router};
        use tower::ServiceExt;

        let usd = CurrencyCode::new("USD").unwrap();
        let policy = AllowancePolicy::new(usd, 80)
            .with_api_key("svc-a", ScopeAllowance::new(86_400, 4_500_000))
            .with_api_key("svc-z", ScopeAllowance::new(86_400, 29_999));
        let ledger: Arc<dyn TreasuryLedgerPort> = Arc::new(InMemoryTreasuryLedger::new());
        let treasurer = Arc::new(Treasurer::new(policy, ledger).with_pricing(e2e_prices()));

        let registry = AgentRegistry::new();
        let agents: [(&str, &str, TokenBudgetConfig); 3] = [
            ("scripted", "gpt-4", TokenBudgetConfig::default()),
            (
                "operator-tight",
                "gpt-4",
                TokenBudgetConfig {
                    enabled: true,
                    max_tokens: 120,
                },
            ),
            ("unpriced", "mystery", TokenBudgetConfig::default()),
        ];
        for (id, model, token_budget) in agents {
            let def = AgentDefinition {
                model: model.to_string(),
                max_loops: Some(10),
                ..base(id)
            };
            let (paladin, executor, streamer) = build_agent_with_llm(
                &def,
                hundred_token_llm(),
                default_circuit_breaker(),
                None,
                token_budget,
            )
            .await
            .expect("builds");
            register_built(&registry, id, paladin, executor, streamer, None, Vec::new())
                .expect("registers");
        }
        let state = AgentApiState::new(Arc::new(registry))
            .with_auth(e2e_auth(&[
                ("key-a", "svc-a"),
                ("key-z", "svc-z"),
                ("key-free", "svc-free"),
            ]))
            .with_treasurer(treasurer);
        let jobs = Arc::clone(&state.jobs);
        let app = agent_router(state);

        // 1. The derived budget cuts the loop: 200 tokens > 150, partial output kept.
        let (status, body, raw) = post_json(&app, "/v1/agents/scripted/execute", "key-a").await;
        assert_eq!(status, axum::http::StatusCode::OK, "{raw}");
        assert_eq!(body["stop_reason"], "allowance_halted", "{raw}");
        assert_eq!(
            body["halt_reason"]["reason"], "allowance_exhausted",
            "{raw}"
        );
        assert_eq!(body["halt_reason"]["ceiling"], "0.0045 USD", "{raw}");
        assert_eq!(body["halt_reason"]["scope"], "api_key", "{raw}");
        assert_eq!(body["usage"]["total_tokens"], 200, "{raw}");
        let output = body["output"].as_str().expect("output text");
        assert!(output.contains("chunk"), "partial output is kept: {raw}");
        assert!(
            output.contains("[budget] Allowance reached"),
            "the allowance-halt notice is on the output: {raw}"
        );
        assert!(!raw.contains("key-a"), "no key value in the body: {raw}");

        // 2. A smaller operator budget wins: today's label, no halt_reason key at all.
        let (status, body, raw) =
            post_json(&app, "/v1/agents/operator-tight/execute", "key-a").await;
        assert_eq!(status, axum::http::StatusCode::OK, "{raw}");
        assert_eq!(body["stop_reason"], "token_budget", "{raw}");
        assert!(body.get("halt_reason").is_none(), "{raw}");

        // 3. A derived budget of zero is refused with the binding ceiling's figures.
        let (status, body, raw) = post_json(&app, "/v1/agents/scripted/execute", "key-z").await;
        assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{raw}");
        assert_eq!(body["error"]["code"], "allowance_exhausted", "{raw}");
        assert_eq!(body["error"]["details"]["scope"], "api_key", "{raw}");
        assert_eq!(body["error"]["details"]["kind"], "window", "{raw}");
        assert!(body["error"]["details"]["window_end"].is_string(), "{raw}");

        // 4. An unpriced model under a ceiling is refused before the agent runs, on all three
        //    routes, naming the model and carrying no Retry-After.
        for uri in [
            "/v1/agents/unpriced/execute",
            "/v1/agents/unpriced/execute/stream",
            "/v1/agents/unpriced/jobs",
        ] {
            let response = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(uri)
                        .header("content-type", "application/json")
                        .header("x-api-key", "key-a")
                        .body(axum::body::Body::from(r#"{"input":"hello"}"#))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(
                response.status(),
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                "{uri}"
            );
            assert!(response.headers().get("retry-after").is_none(), "{uri}");
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body reads");
            let raw = String::from_utf8(bytes.to_vec()).expect("utf8 body");
            let body: serde_json::Value = serde_json::from_str(&raw).expect("json body");
            assert_eq!(body["error"]["code"], "model_unpriced", "{uri}: {raw}");
            assert_eq!(body["error"]["details"]["model"], "mystery", "{uri}: {raw}");
            assert!(
                !raw.contains("key-a") && !raw.contains("svc-a"),
                "{uri}: {raw}"
            );
        }

        // 5. A principal with no ceiling is unaffected: nothing is derived, the run is bounded
        //    only by max_loops, and even the unpriced model runs.
        let (status, body, raw) = post_json(&app, "/v1/agents/scripted/execute", "key-free").await;
        assert_eq!(status, axum::http::StatusCode::OK, "{raw}");
        assert_ne!(body["stop_reason"], "allowance_halted", "{raw}");
        assert!(body.get("halt_reason").is_none(), "{raw}");
        assert_eq!(body["usage"]["total_tokens"], 1_000, "{raw}");
        let (status, _, raw) = post_json(&app, "/v1/agents/unpriced/execute", "key-free").await;
        assert_eq!(status, axum::http::StatusCode::OK, "{raw}");

        // 6. jobs: the same halt lands on the job result.
        let (status, body, raw) = post_json(&app, "/v1/agents/scripted/jobs", "key-a").await;
        assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{raw}");
        let job_id = body["job_id"].as_str().expect("job id").to_string();
        let mut result = None;
        for _ in 0..200 {
            let record = serde_json::to_value(jobs.get(&job_id).expect("job exists"))
                .expect("job serializes");
            if record["result"].is_object() {
                result = Some(record["result"].clone());
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let result = result.expect("the job completes");
        assert_eq!(result["stop_reason"], "allowance_halted");
        assert_eq!(result["halt_reason"]["reason"], "allowance_exhausted");
    }
}
