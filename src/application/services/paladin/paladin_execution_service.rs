//! Paladin Execution Service
//!
//! This module provides the core execution service for Paladins, handling LLM interactions,
//! retry logic, circuit breaking, timeout enforcement, and execution metadata tracking.
//!
//! # Overview
//!
//! The `PaladinExecutionService` orchestrates the execution of a Paladin's reasoning loop,
//! wrapping LLM calls with resilience patterns including:
//! - Exponential backoff retry logic
//! - Circuit breaker pattern for fault tolerance
//! - Timeout enforcement
//! - Stop word detection
//! - Execution metadata tracking
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
//! use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
//! use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
//! use paladin_ports::output::llm_port::LlmPort;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
//! // Create circuit breaker
//! let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
//!
//! // Create execution service
//! let service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);
//!
//! // Build paladin
//! let paladin = PaladinBuilder::new(llm_port)
//!     .system_prompt("You are a helpful assistant")
//!     .max_loops(5)
//!     .retry_attempts(3)
//!     .timeout_seconds(300)
//!     .build().await?;
//!
//! // Execute
//! let result = service.execute(&paladin, "What is Rust?").await?;
//! println!("Output: {}", result.output);
//! println!("Loops: {}, Tokens: {}", result.loop_count, result.token_count);
//! # Ok(())
//! # }
//! ```

use crate::application::services::arsenal::composite_arsenal::CompositeArsenalPort;
use crate::application::services::arsenal::vault_tools::VaultTools;
use crate::application::services::paladin::error::PaladinError;
use crate::application::services::paladin::handoff_service::HandoffService;
use crate::application::services::paladin::middleware::{
    BeforeOutcome, ExecutionMiddleware, LlmResponseView, ModelCallContext, PromptAssembly,
    ToolCallContext, ToolCallKind, ToolFlow, run_after, run_around_tool, run_before,
};
use crate::application::services::paladin::planning_service::PlanningService;
use crate::application::services::paladin::prompt_generation_service::PromptGenerationService;
use crate::application::services::paladin::vault_confined::ConfinedVault;
use crate::application::services::sanctum::memory_extraction_service::{
    MemoryExtractionService, MemoryExtractionStrategy,
};
use crate::application::services::sanctum::rag_retrieval_service::RagRetrievalService;
use crate::config::agent_runtime::{ToolErrorConfig, ToolErrorMode};
use crate::core::base::entity::node::Node;
use crate::core::platform::container::arsenal::{ArmamentCall, ArsenalError};
use crate::core::platform::container::garrison::{ConversationRole, GarrisonEntry};
use crate::core::platform::container::heartbeat::HeartbeatHandle;
use crate::core::platform::container::herald::Herald;
use crate::core::platform::container::paladin::Paladin;
use crate::core::platform::container::prompt::{
    PromptData, PromptItem, PromptParameters, PromptType, UserPrompt,
};
#[cfg(feature = "vision")]
use crate::core::platform::container::vision::VisionContent;
use crate::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use log::{debug, error, info, warn};
use paladin_battalion::llm_failure::to_paladin_error;
use paladin_core::platform::container::run_scope::RunScope;
use paladin_core::platform::container::structured::render_instruction_block;
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::vault::Namespace;
use paladin_llm::fallback::SERVED_BY_METADATA_KEY;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::{FunctionCall, LlmPort, LlmRequest, ResponseFormat};
use paladin_ports::output::orchestrator_port::OrchestratorPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::paladin_port::{
    PaladinResult, PaladinStream, PaladinStreamChunk, StopReason,
};
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_ports::output::structured_executor_port::{
    Structured, StructuredExecutorPort, StructuredOptions, run_structured,
};
use paladin_ports::output::token_counter_port::TokenCounterPort;
use paladin_ports::output::vault_port::VaultPort;
#[cfg(feature = "vision")]
use paladin_ports::output::vision_port::VisionPort;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep, timeout};

/// Paladin Execution Service
///
/// Orchestrates the execution of Paladin reasoning loops with resilience patterns.
///
/// # Features
///
/// - **Retry Logic**: Exponential backoff (100ms, 200ms, 400ms, etc.)
/// - **Circuit Breaker**: Prevents cascading failures
/// - **Timeout Enforcement**: Respects configured timeout limits
/// - **Stop Word Detection**: Halts execution on detected stop words
/// - **Metadata Tracking**: Records execution time, loops, and token usage
/// - **Memory Management**: Stores conversation history in Garrison when provided
///
/// # Thread Safety
///
/// This service is thread-safe and can be shared across threads using `Arc<PaladinExecutionService>`.
pub struct PaladinExecutionService {
    /// LLM port for model interactions
    llm_port: Arc<dyn LlmPort>,

    /// Circuit breaker for fault tolerance
    circuit_breaker: Arc<CircuitBreaker>,

    /// Optional Garrison for conversation memory
    garrison: Option<Arc<dyn GarrisonPort>>,

    /// Optional Arsenal for tool execution
    arsenal: Option<Arc<dyn ArsenalPort>>,

    /// Optional Herald for output formatting
    herald: Option<Arc<dyn Herald>>,

    /// Tool result formatter for context injection
    formatter: ToolResultFormatter,

    /// Optional RAG retrieval service for context augmentation
    rag_retrieval_service: Option<Arc<RagRetrievalService>>,

    /// Optional memory extraction service for storing important information
    memory_extraction_service: Option<Arc<MemoryExtractionService>>,

    /// Vision adapters registry (provider name → adapter)
    #[cfg(feature = "vision")]
    vision_adapters: HashMap<String, Arc<dyn VisionPort>>,

    /// Optional planning service for autonomous task decomposition (Layer 1)
    planning_service: Option<Arc<PlanningService>>,

    /// Optional prompt generation service for dynamic system prompts (Layer 1)
    prompt_generation_service: Option<Arc<PromptGenerationService>>,

    /// Optional handoff service for agent delegation (Layer 3)
    handoff_service: Option<Arc<HandoffService>>,

    /// Optional Agent → Orchestrator bridge port for scheduling jobs, queuing
    /// items, firing events, and sending notifications from agent execution.
    orchestrator_port: Option<Arc<dyn OrchestratorPort>>,

    /// Ordered `ExecutionMiddleware` chain (Doc 05 RT-01, D-01…D-06): fires
    /// `before_model`/`after_model` around every model call of the
    /// reasoning loop and `around_tool` around every Arsenal and handoff
    /// dispatch. Empty by default -- an empty chain reproduces today's
    /// prompt bytes, port call count and `PaladinResult` exactly (D-02).
    middleware: Vec<Arc<dyn ExecutionMiddleware>>,

    /// Synchronous, infallible token-counting port (Doc 05 RT-FR-10, D-13).
    /// Defaults to `HeuristicTokenCounter`. Budget-shaped built-ins
    /// (`HistoryTrimmer`, and a future `SummarizationMiddleware`) consume
    /// this port rather than each constructing or hard-coding their own,
    /// so a caller assembling the middleware chain reads it from here
    /// (`token_counter()`) instead of duplicating the choice.
    token_counter: Arc<dyn TokenCounterPort>,

    /// Overrides the hard-coded `recall_recent(20)` call once a
    /// `HistoryTrimmer` middleware is installed (D-14). `None` (the
    /// default) keeps today's behavior byte-for-byte; a caller that
    /// installs `HistoryTrimmer` via `with_middleware` is responsible for
    /// also calling `with_recall_limit(config.recall_limit)` so the two
    /// stay consistent (X-03: recall_limit is opt-in, exactly like
    /// installing the trimmer itself).
    recall_limit: Option<u32>,

    /// The Vault store this service confines runs into, if any (Doc 05
    /// RT-04, D-21). Installed via [`PaladinExecutionService::with_vault`].
    /// `None` by default -- a service with no vault store confines no run,
    /// no matter what a `RunScope` carries.
    vault: Option<Arc<dyn VaultPort>>,

    /// The default Vault grant applied when a run's own [`RunScope`]
    /// carries none (D-21). `None` by default. See
    /// [`PaladinExecutionService::confined_vault`] for the full resolution
    /// order.
    default_vault_namespace: Option<Namespace>,

    /// Whether the built-in `vault_get`/`vault_put` Armaments are opted
    /// into every run of this service (Doc 05 RT-04, D-22). `false` by
    /// default -- the PRD requires this to be opt-in, never on by default.
    /// Installed via [`PaladinExecutionService::enable_vault_tools`]. Has
    /// no effect on a run that resolves to no Vault grant (see
    /// [`PaladinExecutionService::confined_vault`]): a run with no grant
    /// never sees the vault tools listed, regardless of this flag.
    vault_tools_enabled: bool,

    /// Governs whether a failed tool (Armament or handoff) call is fed
    /// back to the model or fails the run (D-33, D-34). Defaults to
    /// [`ToolErrorConfig::default`] (`mode: FeedToModel`) -- today's v0.9
    /// behavior, not a new default. Set via
    /// [`PaladinExecutionService::with_tool_error_config`].
    tool_error_config: ToolErrorConfig,
}

/// D-16: the latest-summary-wins effective-history rule.
///
/// `GarrisonPort` has no delete-by-id (`garrison_port.rs:380-491`:
/// `remember`, `recall_recent`, `search`, `forget_all`, `stats`), so an old
/// summary a `SummarizationMiddleware` (plan 26-15) has already compounded
/// past is never removed from the store -- it just sits there, older than
/// the newest summary. This function is how every reader tolerates that:
/// given a recalled window in chronological order (oldest first, per
/// [`paladin_ports::output::garrison_port::GarrisonPort::recall_recent`]'s
/// own contract), it finds the NEWEST entry with `is_summary == true` and
/// returns that entry plus every RAW entry logically newer than it. With no
/// summary in the window, the whole window is the effective history,
/// unchanged.
///
/// # Cutoff is by `summarized_through`, not by the summary's own physical
/// position (a correctness fix over a naive position-based reading)
///
/// `GarrisonPort::remember` always APPENDS at the physical end of the store
/// -- a summary is therefore always physically the newest thing in the
/// store the moment it is written, even though the raw entries
/// `SummarizationMiddleware` chose to keep un-folded (`keep_recent`) were
/// all inserted BEFORE it. A naive rule of "keep everything at a position
/// after the summary's own index" would incorrectly drop those kept-raw
/// entries the very next time this function runs, because they sit BEFORE
/// the summary in insertion order, not after it.
///
/// The fix: the summary's own `metadata["summarized_through"]` names the
/// newest RAW entry it folds. This function finds THAT entry's position
/// among the window's raw (non-summary) entries and keeps every raw entry
/// after it -- regardless of where the summary itself physically sits in
/// the window. Falls back to the summary's own physical position only when
/// `summarized_through` is absent or unparseable, or names an entry that
/// aged out of the recalled window (e.g. a hand-built summary in a test, or
/// one written by a future producer that does not set the key).
///
/// Applied by [`PaladinExecutionService::execute_internal`] at the one place
/// a recalled window becomes a run's history -- so the trimmer, the
/// summarizer and the renderer all agree on what "the history" is. Exposed
/// at `pub(crate)` (rather than a private associated function) so
/// `middleware::summarization`'s tests can build the exact fixture a real
/// second run would see, without re-deriving this rule.
///
/// # Examples
///
/// `pub(crate)`, so this illustrative snippet cannot run as a doctest (a
/// doctest compiles against the crate's PUBLIC API only) -- see
/// `effective_history_is_the_newest_summary_plus_newer_raw` in this file's
/// own `#[cfg(test)]` module for the equivalent, actually-run assertion.
///
/// ```ignore
/// use paladin::application::services::paladin::paladin_execution_service::effective_history;
/// use paladin_core::platform::container::garrison::{ConversationRole, GarrisonEntry};
///
/// let history = vec![
///     GarrisonEntry::new(ConversationRole::User, "old raw, superseded".to_string()),
///     GarrisonEntry::summary("condensed older turns".to_string()),
///     GarrisonEntry::new(ConversationRole::User, "newer raw".to_string()),
/// ];
/// let effective = effective_history(history);
/// assert_eq!(effective.len(), 2);
/// assert!(effective[0].is_summary);
/// assert_eq!(effective[1].content, "newer raw");
/// ```
pub(crate) fn effective_history(history: Vec<GarrisonEntry>) -> Vec<GarrisonEntry> {
    let Some(summary_index) = history.iter().rposition(|entry| entry.is_summary) else {
        return history;
    };
    let summary = history[summary_index].clone();

    let summarized_through_id = summary
        .metadata
        .get("summarized_through")
        .and_then(|value| value.as_str())
        .and_then(|id_str| uuid::Uuid::parse_str(id_str).ok());

    let cutoff_index_among_raw = summarized_through_id.and_then(|id| {
        history
            .iter()
            .position(|entry| !entry.is_summary && entry.id == id)
    });

    let kept_raw: Vec<GarrisonEntry> = match cutoff_index_among_raw {
        Some(cutoff) => history
            .iter()
            .enumerate()
            .filter(|(index, entry)| *index > cutoff && !entry.is_summary)
            .map(|(_, entry)| entry.clone())
            .collect(),
        None => history
            .iter()
            .enumerate()
            .filter(|(index, entry)| *index > summary_index && !entry.is_summary)
            .map(|(_, entry)| entry.clone())
            .collect(),
    };

    let mut effective = Vec::with_capacity(1 + kept_raw.len());
    effective.push(summary);
    effective.extend(kept_raw);
    effective
}

impl PaladinExecutionService {
    /// Creates a new Paladin execution service
    ///
    /// # Arguments
    ///
    /// * `llm_port` - The LLM port implementation to use for model calls
    /// * `circuit_breaker` - Circuit breaker for fault tolerance
    /// * `garrison` - Optional Garrison for conversation memory (None for stateless operations)
    /// * `arsenal` - Optional Arsenal for tool execution (None to disable tool support)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// use paladin_ports::output::llm_port::LlmPort;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    /// let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
    /// # }
    /// ```
    pub fn new(
        llm_port: Arc<dyn LlmPort>,
        circuit_breaker: Arc<CircuitBreaker>,
        garrison: Option<Arc<dyn GarrisonPort>>,
        arsenal: Option<Arc<dyn ArsenalPort>>,
    ) -> Self {
        info!(
            "Creating PaladinExecutionService with garrison: {}, arsenal: {}",
            garrison.is_some(),
            arsenal.is_some()
        );
        Self {
            llm_port,
            circuit_breaker,
            garrison,
            arsenal,
            herald: None,
            formatter: ToolResultFormatter::new(),
            rag_retrieval_service: None,
            memory_extraction_service: None,
            #[cfg(feature = "vision")]
            vision_adapters: HashMap::new(),
            planning_service: None,
            prompt_generation_service: None,
            handoff_service: None,
            orchestrator_port: None,
            middleware: Vec::new(),
            token_counter: Arc::new(paladin_memory::token_counter::HeuristicTokenCounter),
            recall_limit: None,
            vault: None,
            default_vault_namespace: None,
            vault_tools_enabled: false,
            tool_error_config: ToolErrorConfig::default(),
        }
    }

    /// Sets the Agent → Orchestrator bridge port
    ///
    /// Attaches an [`OrchestratorPort`] so that Paladins can schedule jobs,
    /// queue items, fire events, and send notifications during execution. When
    /// not set, the service behaves exactly as before (no orchestration bridge).
    ///
    /// # Arguments
    ///
    /// * `orchestrator_port` - The orchestrator bridge port implementation
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_orchestrator_port(mut self, orchestrator_port: Arc<dyn OrchestratorPort>) -> Self {
        info!("Attaching orchestrator bridge port to PaladinExecutionService");
        self.orchestrator_port = Some(orchestrator_port);
        self
    }

    /// Returns a reference to the attached orchestrator bridge port, if any.
    pub fn orchestrator_port(&self) -> Option<&Arc<dyn OrchestratorPort>> {
        self.orchestrator_port.as_ref()
    }

    /// Sets the RAG retrieval service for context augmentation
    ///
    /// # Arguments
    ///
    /// * `service` - The RAG retrieval service to use for context retrieval
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_rag_retrieval(mut self, service: Arc<RagRetrievalService>) -> Self {
        info!("Attaching RAG retrieval service to PaladinExecutionService");
        self.rag_retrieval_service = Some(service);
        self
    }

    /// Sets the tool-error policy (D-33, D-34): whether a failed tool
    /// (Armament or handoff) call is fed back into the model's context
    /// (`ToolErrorMode::FeedToModel`, the default -- today's v0.9 behavior,
    /// unchanged) or fails the run with a structured
    /// [`PaladinError::ArmamentFailed`] (`ToolErrorMode::FailRun`, the new
    /// opt-in), with an optional per-tool-name override
    /// (`ToolErrorConfig::per_tool`) taking precedence over the global
    /// mode. Does NOT touch `PaladinConfig` -- this is service-level
    /// configuration only (D-10).
    ///
    /// [`PaladinError::ArmamentFailed`]: paladin_core::platform::container::paladin_error::PaladinError::ArmamentFailed
    ///
    /// # Arguments
    ///
    /// * `config` - The tool-error policy this service applies to every
    ///   run's Arsenal and handoff tool-call failures.
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_tool_error_config(mut self, config: ToolErrorConfig) -> Self {
        info!(
            "Setting tool-error policy on PaladinExecutionService: mode={:?}, per_tool_overrides={}",
            config.mode,
            config.per_tool.len()
        );
        self.tool_error_config = config;
        self
    }

    /// Resolves the effective [`ToolErrorMode`] for `tool_name`: a
    /// `per_tool` override if one is configured for that name, else the
    /// global `mode` (D-34).
    fn effective_tool_error_mode(&self, tool_name: &str) -> ToolErrorMode {
        self.tool_error_config
            .per_tool
            .get(tool_name)
            .copied()
            .unwrap_or(self.tool_error_config.mode)
    }

    /// Installs `vault` as this service's Vault store, with `default_namespace`
    /// as the grant applied when a run's own [`RunScope`] carries none (Doc
    /// 05 RT-04, D-21).
    ///
    /// # Arguments
    ///
    /// * `vault` - The Vault backend every confined run's calls ultimately
    ///   reach, once a grant admits them.
    /// * `default_namespace` - The namespace granted to a run whose
    ///   `RunScope::vault_namespace` is `None`. `None` here means this
    ///   service has no default grant of its own -- a run with neither a
    ///   scope namespace nor a service default gets **no grant at all**
    ///   (see [`PaladinExecutionService::confined_vault`]).
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn with_vault(
        mut self,
        vault: Arc<dyn VaultPort>,
        default_namespace: Option<Namespace>,
    ) -> Self {
        info!(
            "Attaching Vault store to PaladinExecutionService (default namespace: {})",
            default_namespace
                .as_ref()
                .map(|ns| ns.to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        self.vault = Some(vault);
        self.default_vault_namespace = default_namespace;
        self
    }

    /// Resolves the [`ConfinedVault`] a run under `scope` should observe
    /// (Doc 05 RT-04, D-21), in a fixed order:
    ///
    /// 1. `scope.vault_namespace`, if set.
    /// 2. Else this service's own [`PaladinExecutionService::with_vault`]
    ///    default namespace, if set.
    /// 3. Else **no grant at all** -- returns `None`.
    ///
    /// # No grant means denied, never root
    ///
    /// When neither the scope nor the service default names a namespace,
    /// this method returns `None` rather than falling back to some
    /// "root"-like namespace. A `None` here means the run has no Vault
    /// handle whatsoever: no `ConfinedVault` is ever constructed, so no
    /// call can reach the store, and (once a later plan wires vault tools)
    /// no vault tool is even listed for the run. A bug that instead
    /// defaulted to a root namespace here would silently hand every agent
    /// the whole store -- this method's contract exists specifically to
    /// rule that out.
    ///
    /// This method also returns `None` when no Vault store was ever
    /// installed via `with_vault`, regardless of what `scope` carries --
    /// there is nothing to confine access to.
    pub fn confined_vault(&self, scope: &RunScope) -> Option<ConfinedVault> {
        let vault = self.vault.clone()?;
        let namespace = scope
            .vault_namespace
            .clone()
            .or_else(|| self.default_vault_namespace.clone())?;
        Some(ConfinedVault::new(vault, namespace))
    }

    /// Opts every run of this service into the built-in `vault_get`/
    /// `vault_put` Armaments (Doc 05 RT-04, D-22) -- **off by default**,
    /// matching the PRD's opt-in requirement (`VaultToolsConfig.enabled` is
    /// `false` out of the box too; a config-driven caller should call this
    /// method when that flag is `true`).
    ///
    /// When a run resolves to a Vault grant (see
    /// [`PaladinExecutionService::confined_vault`]), the effective arsenal
    /// for that run becomes a [`CompositeArsenalPort`] of this service's
    /// configured arsenal (if any, via [`PaladinExecutionService::new`])
    /// and a fresh [`VaultTools`] built from that run's own
    /// [`ConfinedVault`] -- so the tools reach only that run's own granted
    /// subtree, never another run's. When a run resolves to **no** grant,
    /// this flag has no effect at all: the vault tools are not listed and
    /// not reachable for that run, exactly as if this method were never
    /// called (D-21's "no grant means denied, never root", extended to
    /// tool visibility itself).
    ///
    /// # HTTP-served agents never reach this (ADR-0039)
    ///
    /// Agents served through the HTTP service-host topology
    /// (`crates/paladin-web/src/agent_registry.rs`,
    /// `src/bin/paladin-server.rs`) carry no Arsenal at all -- ADR-0039
    /// records this as a permanent property of that topology, not a gap to
    /// close. Enabling vault tools on a service has no observable effect
    /// over `/v1/agents/*`: there is no Arsenal for a composite to be
    /// installed into. This is stated here, not worked around -- no
    /// HTTP-specific code path is added to give an HTTP-served agent an
    /// Arsenal it otherwise would not have.
    ///
    /// # Returns
    ///
    /// Returns self for method chaining.
    pub fn enable_vault_tools(mut self) -> Self {
        info!("Enabling built-in vault_get/vault_put Armaments (opt-in, D-22)");
        self.vault_tools_enabled = true;
        self
    }

    /// Resolves the [`ArsenalPort`] a run should actually dispatch tool
    /// calls through, applying [`PaladinExecutionService::enable_vault_tools`]'s
    /// opt-in composition on top of this service's configured arsenal.
    ///
    /// - Vault tools disabled, or the run has no grant: returns this
    ///   service's configured arsenal unchanged (`None` if none was
    ///   configured). A run with no grant sees no vault tools even when
    ///   [`Self::enable_vault_tools`] was called (D-21).
    /// - Vault tools enabled AND the run has a grant: returns a
    ///   [`CompositeArsenalPort`] of the configured arsenal (if any) plus a
    ///   fresh [`VaultTools`] scoped to that run's own [`ConfinedVault`].
    fn effective_arsenal(
        &self,
        confined_vault: Option<&ConfinedVault>,
    ) -> Option<Arc<dyn ArsenalPort>> {
        if !self.vault_tools_enabled {
            return self.arsenal.clone();
        }

        let Some(confined) = confined_vault else {
            // No grant -- the flag has no effect for this run (D-21).
            return self.arsenal.clone();
        };

        let vault_tools: Arc<dyn ArsenalPort> = Arc::new(VaultTools::new(confined.clone()));
        match &self.arsenal {
            Some(existing) => Some(Arc::new(CompositeArsenalPort::new(vec![
                existing.clone(),
                vault_tools,
            ]))),
            None => Some(vault_tools),
        }
    }

    /// Sets the memory extraction service for storing important information
    ///
    /// # Arguments
    ///
    /// * `service` - The memory extraction service to use
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_memory_extraction(mut self, service: Arc<MemoryExtractionService>) -> Self {
        info!("Attaching memory extraction service to PaladinExecutionService");
        self.memory_extraction_service = Some(service);
        self
    }

    /// Sets the Herald formatter for this service
    ///
    /// # Arguments
    ///
    /// * `herald` - The Herald implementation to use for formatting execution results
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// use paladin::infrastructure::adapters::herald::JsonHerald;
    /// use std::sync::Arc;
    /// # use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::time::Duration;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// # let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    /// let herald = Arc::new(JsonHerald::default());
    /// let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
    ///     .with_herald(herald);
    /// # }
    /// ```
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Registers a vision adapter for a specific provider
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider name (e.g., "openai", "anthropic")
    /// * `adapter` - The vision adapter implementation
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// use paladin::infrastructure::adapters::llm::openai_adapter::OpenAIAdapter;
    /// use std::sync::Arc;
    /// # use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::time::Duration;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>, openai: Arc<OpenAIAdapter>) {
    /// # let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    /// let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
    ///     .with_vision_adapter("openai".to_string(), openai);
    /// # }
    /// ```
    #[cfg(feature = "vision")]
    pub fn with_vision_adapter(mut self, provider: String, adapter: Arc<dyn VisionPort>) -> Self {
        info!("Registering vision adapter for provider: {}", provider);
        self.vision_adapters.insert(provider, adapter);
        self
    }

    /// Sets the planning service for autonomous task decomposition (Layer 1)
    ///
    /// # Arguments
    ///
    /// * `service` - The planning service to use for task decomposition
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// use paladin::application::services::paladin::planning_service::PlanningService;
    /// use std::sync::Arc;
    /// # use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::time::Duration;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// # let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    /// let planning_service = Arc::new(PlanningService::new(llm_port.clone()));
    /// let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
    ///     .with_planning_service(planning_service);
    /// # }
    /// ```
    pub fn with_planning_service(mut self, service: Arc<PlanningService>) -> Self {
        info!("Attaching planning service to PaladinExecutionService");
        self.planning_service = Some(service);
        self
    }

    /// Sets the prompt generation service for dynamic system prompts (Layer 1)
    ///
    /// # Arguments
    ///
    /// * `service` - The prompt generation service to use
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// use paladin::application::services::paladin::prompt_generation_service::PromptGenerationService;
    /// use std::sync::Arc;
    /// # use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::time::Duration;
    ///
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// # let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    /// let prompt_service = Arc::new(PromptGenerationService::new(llm_port.clone()));
    /// let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
    ///     .with_prompt_generation_service(prompt_service);
    /// # }
    /// ```
    pub fn with_prompt_generation_service(mut self, service: Arc<PromptGenerationService>) -> Self {
        info!("Attaching prompt generation service to PaladinExecutionService");
        self.prompt_generation_service = Some(service);
        self
    }

    /// Sets the handoff service for agent delegation (Layer 3)
    ///
    /// When configured, the execution service can delegate tasks to specialist
    /// Paladins when a `handoff_to_specialist` tool call is detected.
    ///
    /// # Arguments
    ///
    /// * `service` - The HandoffService for managing delegations
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_handoff_service(mut self, service: Arc<HandoffService>) -> Self {
        info!("Attaching handoff service to PaladinExecutionService (Layer 3)");
        self.handoff_service = Some(service);
        self
    }

    /// Appends `middleware` to the end of the `ExecutionMiddleware` chain
    /// (Doc 05 RT-01, D-06).
    ///
    /// Middleware run in attachment order for `before_model`/`around_tool`
    /// and in reverse order for `after_model` (the onion shape: the first
    /// middleware to see the request is the last to see the response).
    ///
    /// # Arguments
    ///
    /// * `middleware` - The middleware to append
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_middleware(mut self, middleware: Arc<dyn ExecutionMiddleware>) -> Self {
        info!(
            "Attaching execution middleware to PaladinExecutionService: {}",
            middleware.name()
        );
        self.middleware.push(middleware);
        self
    }

    /// Replaces the whole `ExecutionMiddleware` chain with `chain` (Doc 05
    /// RT-01, D-06).
    ///
    /// # Arguments
    ///
    /// * `chain` - The middleware chain to install, in attachment order
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_middleware_chain(mut self, chain: Vec<Arc<dyn ExecutionMiddleware>>) -> Self {
        info!(
            "Replacing PaladinExecutionService middleware chain: {} middleware",
            chain.len()
        );
        self.middleware = chain;
        self
    }

    /// Sets the token-counting port budget-shaped built-ins consume
    /// (Doc 05 RT-FR-10, D-13). Defaults to `HeuristicTokenCounter` -- a
    /// caller assembling a `HistoryTrimmer` (or a future
    /// `SummarizationMiddleware`) reads this service's counter via
    /// [`Self::token_counter`] rather than constructing its own, so every
    /// budget feature this phase adds shares the ONE counting path.
    ///
    /// # Arguments
    ///
    /// * `counter` - The token-counting adapter to use
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounterPort>) -> Self {
        info!(
            "Setting PaladinExecutionService token counter: {}",
            counter.name()
        );
        self.token_counter = counter;
        self
    }

    /// Returns the configured `TokenCounterPort` (defaults to
    /// `HeuristicTokenCounter`).
    pub fn token_counter(&self) -> &Arc<dyn TokenCounterPort> {
        &self.token_counter
    }

    /// Overrides the hard-coded `recall_recent(20)` call with `limit`
    /// (D-14). Install this alongside a `HistoryTrimmer` middleware
    /// carrying the same `HistoryTrimmerConfig.recall_limit` value -- with
    /// no call to this method the service recalls 20 entries exactly as it
    /// does today (X-03).
    ///
    /// # Arguments
    ///
    /// * `limit` - The number of recent Garrison entries to recall
    ///
    /// # Returns
    ///
    /// Returns self for method chaining
    pub fn with_recall_limit(mut self, limit: u32) -> Self {
        info!("Setting PaladinExecutionService Garrison recall_limit: {limit}");
        self.recall_limit = Some(limit);
        self
    }

    /// Formats a Paladin execution result using the configured Herald
    ///
    /// If no Herald is configured, returns None. This allows for optional formatting
    /// based on runtime configuration or user preferences.
    ///
    /// # Arguments
    ///
    /// * `result` - The Paladin execution result to format
    /// * `paladin` - The Paladin that produced this result (for name/ID)
    ///
    /// # Returns
    ///
    /// Returns `Some(formatted_output)` if a Herald is configured and formatting succeeds,
    /// `None` if no Herald is configured.
    ///
    /// # Errors
    ///
    /// Returns `PaladinError::ExecutionError` if formatting fails.
    pub fn format_result(
        &self,
        result: &PaladinResult,
        _paladin: &Paladin,
    ) -> Result<Option<String>, PaladinError> {
        if let Some(ref herald) = self.herald {
            // Herald now uses actual PaladinResult directly - no conversion needed!
            let formatted = herald.format_paladin_result(result).map_err(|e| {
                PaladinError::ExecutionError(format!("Herald formatting failed: {}", e))
            })?;
            Ok(Some(formatted))
        } else {
            Ok(None)
        }
    }

    /// Executes a Paladin with the given input
    ///
    /// This is the main entry point for Paladin execution. It orchestrates the entire
    /// execution lifecycle including timeout enforcement, retry logic, and metadata tracking.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin to execute
    /// * `input` - The input text to process
    ///
    /// # Returns
    ///
    /// - `Ok(PaladinResult)` - Execution succeeded with result metadata
    /// - `Err(PaladinError)` - Execution failed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # use std::time::Duration;
    /// # async fn example(llm_port: Arc<dyn LlmPort>, service: PaladinExecutionService) -> Result<(), Box<dyn std::error::Error>> {
    /// # let paladin = PaladinBuilder::new(llm_port).system_prompt("test").build().await?;
    /// let result = service.execute(&paladin, "Explain quantum computing").await?;
    /// println!("Result: {}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.execute_scoped(paladin, input, None, &RunScope::default())
            .await
    }

    /// Execute a Paladin while reporting progress on `heartbeat` (Doc 04
    /// FT-FR-09, D-19; plan 25-09).
    ///
    /// Runs EXACTLY the path [`PaladinExecutionService::execute`] runs --
    /// same reasoning loop, same timeout wrapper, identical result for
    /// identical inputs -- and additionally beats `heartbeat` at each
    /// progress event: after every completed LLM call and after every
    /// Armament invocation (a streamed chunk is the third event, reported
    /// by [`PaladinExecutionService::execute_stream_observed`]). Each beat
    /// resets the superstep engine's per-attempt `idle_timeout` timer,
    /// which is how a node that is merely slow is told apart from one that
    /// has stalled.
    ///
    /// This is the service's implementation of the defaulted
    /// `PaladinPort::execute_observed` contract: the service itself
    /// implements `PaladinExecutorPort` (not `PaladinPort`), so the
    /// progress-reporting entry point is exposed as an inherent method the
    /// `PaladinPort` adapter over this service delegates to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_core::platform::container::heartbeat::HeartbeatHandle;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>, service: PaladinExecutionService) -> Result<(), Box<dyn std::error::Error>> {
    /// # let paladin = PaladinBuilder::new(llm_port).system_prompt("test").build().await?;
    /// let heartbeat = HeartbeatHandle::new();
    /// let result = service.execute_observed(&paladin, "Summarise", &heartbeat).await?;
    /// assert!(heartbeat.beats() >= 1, "at least one LLM call completed");
    /// println!("{}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_observed(
        &self,
        paladin: &Paladin,
        input: &str,
        heartbeat: &HeartbeatHandle,
    ) -> Result<PaladinResult, PaladinError> {
        self.execute_scoped(paladin, input, Some(heartbeat), &RunScope::default())
            .await
    }

    /// Execute a Paladin under a host-issued [`RunScope`] (Doc 05 RT-04,
    /// D-21) -- the real entry point `execute` and `execute_observed` both
    /// are, each with `RunScope::default()` (one implementation, three
    /// doors).
    ///
    /// Runs EXACTLY the path [`PaladinExecutionService::execute`] runs --
    /// same reasoning loop, same timeout wrapper -- resolving `scope`'s
    /// Vault grant via [`PaladinExecutionService::confined_vault`] before
    /// dispatching. `heartbeat` is `Some` only on the observed path,
    /// exactly like [`Self::execute_bounded`]'s own contract.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_core::platform::container::run_scope::RunScope;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>, service: PaladinExecutionService) -> Result<(), Box<dyn std::error::Error>> {
    /// # let paladin = PaladinBuilder::new(llm_port).system_prompt("test").build().await?;
    /// let result = service
    ///     .execute_scoped(&paladin, "Summarise", None, &RunScope::default())
    ///     .await?;
    /// println!("{}", result.output);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_scoped(
        &self,
        paladin: &Paladin,
        input: &str,
        heartbeat: Option<&HeartbeatHandle>,
        scope: &RunScope,
    ) -> Result<PaladinResult, PaladinError> {
        let execution_id = uuid::Uuid::new_v4();
        // --- D-21/D-25: resolved here (before dispatch) so the run's
        // `ModelCallContext` (built inside `execute_internal`) carries the
        // SAME `ConfinedVault` handle `VaultRecallMiddleware` reads --
        // exactly one resolution point, per `confined_vault`'s own contract.
        let confined_vault = self.confined_vault(scope);
        info!(
            "Starting scoped Paladin execution: id={}, name={}, input_len={}, vault_namespace={}",
            execution_id,
            paladin.node.name,
            input.len(),
            confined_vault
                .as_ref()
                .map(|v| v.granted().to_string())
                .unwrap_or_else(|| "none".to_string())
        );
        self.execute_bounded(paladin, input, execution_id, heartbeat, confined_vault)
            .await
    }

    /// The shared body of `execute`, `execute_observed` and `execute_scoped`:
    /// the same per-execution timeout wrapper around
    /// [`Self::execute_internal`], differing only in whether a heartbeat is
    /// threaded through.
    async fn execute_bounded(
        &self,
        paladin: &Paladin,
        input: &str,
        execution_id: uuid::Uuid,
        heartbeat: Option<&HeartbeatHandle>,
        confined_vault: Option<ConfinedVault>,
    ) -> Result<PaladinResult, PaladinError> {
        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(paladin.node.max_loops.as_u32() as u64 * 60);

        // Wrap execution with timeout
        let execution_future =
            self.execute_internal(paladin, input, execution_id, heartbeat, confined_vault);

        match timeout(timeout_duration, execution_future).await {
            Ok(result) => {
                let elapsed = start_time.elapsed();
                info!(
                    "Paladin execution completed: id={}, duration_ms={}, success={}",
                    execution_id,
                    elapsed.as_millis(),
                    result.is_ok()
                );
                result
            }
            Err(_) => {
                let elapsed = start_time.elapsed();
                error!(
                    "Paladin execution timed out: id={}, duration_ms={}",
                    execution_id,
                    elapsed.as_millis()
                );
                Err(PaladinError::Timeout(elapsed.as_secs()))
            }
        }
    }

    /// Execute a Paladin with vision capabilities
    ///
    /// Validates that the Paladin has vision enabled and the LLM provider supports vision.
    /// Executes vision analysis using the registered vision adapters.
    #[cfg(feature = "vision")]
    pub async fn execute_with_vision(
        &self,
        paladin: &Paladin,
        task: &str,
        images: Vec<VisionContent>,
    ) -> Result<PaladinResult, PaladinError> {
        // Step 1: Validate vision is enabled on the Paladin
        if !paladin.node.vision_enabled {
            return Err(PaladinError::ConfigurationError(
                "Vision execution requires vision_enabled=true. Use PaladinBuilder::enable_vision(true)".to_string(),
            ));
        }

        // Step 2: Check if LLM provider supports vision
        let capabilities = self.llm_port.get_capabilities();
        if !capabilities.supports_vision {
            return Err(PaladinError::ConfigurationError(format!(
                "LLM provider '{}' does not support vision capabilities. Use a vision-capable model like gpt-4o or claude-3-opus",
                self.llm_port.get_provider_name()
            )));
        }

        // Step 3: Extract provider from model name
        let model = paladin.node.model.as_str();
        let provider = self.extract_provider_from_model(model)?;

        // Step 4: Get vision adapter for provider
        let vision_adapter = self.vision_adapters.get(&provider).ok_or_else(|| {
            PaladinError::ExecutionError(format!(
                "No vision adapter registered for provider: {}. Available: {:?}",
                provider,
                self.vision_adapters.keys().collect::<Vec<_>>()
            ))
        })?;

        // Step 5: Determine timeout from paladin config
        let timeout_secs = paladin.node.max_loops.as_u32() as u64 * 60;
        let timeout_duration = Duration::from_secs(timeout_secs);

        // Step 6: Execute vision analysis with timeout
        let vision_result = timeout(timeout_duration, async {
            vision_adapter
                .analyze_image(
                    task,
                    images.clone(),
                    model,
                    Some(4000), // Default max tokens for vision
                )
                .await
                .map_err(|e| match e {
                    crate::core::platform::container::vision::VisionError::InvalidRequest(msg) => {
                        PaladinError::ExecutionError(msg)
                    }
                    crate::core::platform::container::vision::VisionError::Timeout(secs) => {
                        PaladinError::ExecutionError(format!(
                            "Vision request timed out after {} seconds",
                            secs
                        ))
                    }
                    crate::core::platform::container::vision::VisionError::MaxRetriesExceeded(
                        attempts,
                    ) => PaladinError::ExecutionError(format!(
                        "Max retries exceeded: {} attempts",
                        attempts
                    )),
                    _ => PaladinError::ExecutionError(e.to_string()),
                })
        })
        .await
        .map_err(|_| {
            PaladinError::ExecutionError(format!(
                "Vision execution timed out after {} seconds",
                timeout_secs
            ))
        })??;

        // Step 7: Check for stop words
        for stop_word in &paladin.node.stop_words {
            if vision_result.content.contains(stop_word) {
                return Err(PaladinError::ExecutionError(format!(
                    "Stop word detected: {}",
                    stop_word
                )));
            }
        }

        // Step 8: Store in garrison if configured
        if let Some(ref garrison) = self.garrison {
            // Store user message with task
            let user_entry = GarrisonEntry::new(ConversationRole::User, task.to_string());
            garrison.remember(user_entry).await.map_err(|e| {
                warn!("Failed to store user vision message in garrison: {}", e);
                PaladinError::ExecutionError(format!("Garrison storage failed: {}", e))
            })?;

            // Store assistant response
            let assistant_entry =
                GarrisonEntry::new(ConversationRole::Assistant, vision_result.content.clone());
            garrison.remember(assistant_entry).await.map_err(|e| {
                warn!(
                    "Failed to store assistant vision response in garrison: {}",
                    e
                );
                PaladinError::ExecutionError(format!("Garrison storage failed: {}", e))
            })?;
        }

        // Step 9: Build and return PaladinResult
        Ok(PaladinResult {
            output: vision_result.content,
            loop_count: 1, // Vision is single-shot
            token_count: vision_result.token_usage.total_tokens,
            stop_reason: StopReason::Completed,
            execution_time_ms: 0, // Will be set by caller if needed
            ..Default::default()
        })
    }

    /// Extract provider name from model string
    ///
    /// Examples:
    /// - "gpt-4o" → "openai"
    /// - "claude-3-opus" → "anthropic"
    #[cfg(feature = "vision")]
    fn extract_provider_from_model(&self, model: &str) -> Result<String, PaladinError> {
        if model.starts_with("gpt-") || model.starts_with("o1-") {
            Ok("openai".to_string())
        } else if model.starts_with("claude-") {
            Ok("anthropic".to_string())
        } else {
            Err(PaladinError::ConfigurationError(format!(
                "Cannot determine provider from model: {}. Use gpt-* or claude-* models",
                model
            )))
        }
    }

    /// Internal execution logic without timeout wrapper.
    ///
    /// `heartbeat` is `Some` only on the `execute_observed` path (D-19):
    /// it is beaten after every completed LLM call and after every
    /// Armament invocation, and never consulted otherwise, so the
    /// unobserved `execute` path is byte-identical to before plan 25-09.
    async fn execute_internal(
        &self,
        paladin: &Paladin,
        input: &str,
        execution_id: uuid::Uuid,
        heartbeat: Option<&HeartbeatHandle>,
        confined_vault: Option<ConfinedVault>,
    ) -> Result<PaladinResult, PaladinError> {
        let start_time = Instant::now();
        let mut total_tokens = 0u32;
        let mut accumulated_output = String::new();
        let mut _retrieval_latency_ms = 0u64;
        let mut _memories_retrieved_count = 0usize;
        let mut _extraction_triggered = false;
        let mut handoff_history = Vec::new();
        // FT-FR-17 (D-26): the provider that served the latest loop, copied
        // from the `paladin.served_by` metadata key a `FallbackLlmAdapter`
        // stamps on its response. A plain adapter stamps nothing, so this
        // stays `None` and the serialised result is byte-identical to before.
        let mut served_by: Option<String> = None;

        // =======================================================================
        // LAYER 1: Autonomous Planning & Prompt Generation (Optional, Pre-Exec)
        // =======================================================================

        // Apply Layer 1a: Planning (if enabled)
        let task_plan = self
            .apply_layer1_planning(paladin, input, execution_id)
            .await;

        // Apply Layer 1b: Prompt Generation (if enabled)
        let generated_prompt = self
            .apply_layer1_prompt_generation(paladin, execution_id)
            .await;

        // Use generated prompt if available, otherwise use configured prompt
        let effective_system_prompt = generated_prompt
            .as_ref()
            .unwrap_or(&paladin.node.system_prompt);

        // =======================================================================
        // CORE LAYER 0: Standard Execution (Always Runs)
        // =======================================================================

        // Step 1: Retrieve relevant context from Sanctum if RAG is configured
        let retrieved_context = if self.check_sanctum_configured() {
            debug!(
                "Sanctum configured, retrieving context: execution_id={}",
                execution_id
            );
            let retrieval_start = Instant::now();

            match self
                .retrieve_context_with_timeout(paladin, input, execution_id)
                .await
            {
                Ok(results) => {
                    _retrieval_latency_ms = retrieval_start.elapsed().as_millis() as u64;
                    _memories_retrieved_count = results.len();

                    // Format results into context string
                    let context = if results.is_empty() {
                        String::new()
                    } else {
                        self.format_retrieved_context(&results)
                    };

                    info!(
                        "RAG retrieval succeeded: execution_id={}, memories={}, latency_ms={}",
                        execution_id, _memories_retrieved_count, _retrieval_latency_ms
                    );
                    Some(context)
                }
                Err(e) => {
                    _retrieval_latency_ms = retrieval_start.elapsed().as_millis() as u64;
                    warn!(
                        "RAG retrieval failed: execution_id={}, error={}, latency_ms={}",
                        execution_id, e, _retrieval_latency_ms
                    );
                    None
                }
            }
        } else {
            None
        };

        // Store user input in garrison if available
        if let Some(garrison) = &self.garrison {
            let user_entry = GarrisonEntry::new(ConversationRole::User, input.to_string());
            garrison.remember(user_entry).await?;
            debug!(
                "Stored user input in garrison: execution_id={}",
                execution_id
            );
        }

        // Retrieve conversation history if garrison is available. D-14:
        // `recall_limit` replaces the hard-coded 20 only once a caller has
        // installed it (alongside a `HistoryTrimmer`); with no override this
        // is byte-for-byte today's behavior (X-03).
        let conversation_history = if let Some(garrison) = &self.garrison {
            let recall_limit = self.recall_limit.unwrap_or(20) as usize;
            let history = garrison.recall_recent(recall_limit).await?;
            let recalled_count = history.len();
            // D-16: latest-summary-wins is applied HERE, at the one place a
            // recalled Garrison window becomes a run's history, so the
            // trimmer, the summarizer and the renderer all read the same
            // "effective history" -- see `effective_history`'s own doc for
            // why old summaries are skipped rather than deleted.
            let history = effective_history(history);
            debug!(
                "Retrieved {} messages from garrison (effective history: {} after D-16's \
                 latest-summary-wins rule): execution_id={}",
                recalled_count,
                history.len(),
                execution_id
            );
            history
        } else {
            vec![]
        };

        // The ExecutionMiddleware chain's per-run context (Doc 05 D-03):
        // constructed once for the whole run so `scratch` and the typed
        // state bag live across every loop iteration. The assembly is
        // replaced every iteration below.
        let mut middleware_cx = ModelCallContext::new(
            execution_id,
            paladin,
            PromptAssembly::new(effective_system_prompt.clone(), input, "", vec![], None),
        );
        // D-22: resolved BEFORE `confined_vault` is moved into
        // `middleware_cx.vault` below, from a clone of the same handle --
        // so the tool-call branch and `VaultRecallMiddleware` observe the
        // exact same grant for this run.
        let effective_arsenal = self.effective_arsenal(confined_vault.as_ref());

        // D-21/D-25: the run's Vault grant, resolved once by
        // `execute_scoped` before dispatch, is set once here and read (never
        // mutated) by `VaultRecallMiddleware` on loop 0.
        middleware_cx.vault = confined_vault;

        // Execute reasoning loop
        for loop_num in 1..=paladin.node.max_loops.as_u32() {
            debug!(
                "Paladin loop iteration: id={}, loop={}/{}",
                execution_id, loop_num, paladin.node.max_loops
            );

            // =======================================================================
            // LAYER 2: Dynamic Temperature (Optional, Per-Loop)
            // =======================================================================
            let effective_temperature = self.apply_layer2_dynamic_temperature(paladin, loop_num);

            // Build the prompt assembly for this iteration with conversation
            // history and RAG context (D-02). Use effective_system_prompt
            // from Layer 1 (generated or original).
            middleware_cx.assembly = PromptAssembly::new(
                effective_system_prompt.clone(),
                input,
                accumulated_output.as_str(),
                conversation_history.clone(),
                retrieved_context.clone(),
            );
            middleware_cx.loop_index = loop_num - 1;

            // --- D-04: before_model fires once per iteration, after the
            // assembly is built and before the model call.
            let before_outcome = run_before(&self.middleware, &mut middleware_cx).await?;
            let reached = match before_outcome {
                BeforeOutcome::Continue { reached } => reached,
                BeforeOutcome::Finish { result, reached } => {
                    // A middleware finished the run before any model call
                    // this iteration -- still run `after_model` over the
                    // reached prefix (D-06), on a synthetic response view
                    // built from the FinalResult, then return without
                    // calling the LLM.
                    let mut synthetic_view = LlmResponseView {
                        content: result.output.clone(),
                        usage: paladin_core::platform::container::token_usage::TokenUsage::default(
                        ),
                        finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                        function_call: None,
                    };
                    // CR-02 (26-REVIEW.md): if a middleware in the reached
                    // prefix itself requests a `Finish` from `after_model`,
                    // that hook's own `FinalResult` (output AND
                    // stop_reason) wins over the original `before_model`
                    // `Finish`'s result -- mirroring the sibling
                    // post-model-call path below, which threads
                    // `final_result.stop_reason` through unconditionally.
                    let effective_result = run_after(
                        &self.middleware,
                        &mut middleware_cx,
                        &mut synthetic_view,
                        reached,
                    )
                    .await?
                    .unwrap_or(result);
                    accumulated_output = synthetic_view.content;

                    if let Some(garrison) = &self.garrison {
                        let assistant_entry = GarrisonEntry::new(
                            ConversationRole::Assistant,
                            accumulated_output.clone(),
                        );
                        garrison.remember(assistant_entry).await?;
                    }
                    if self.should_extract_memories(MemoryExtractionStrategy::OnCompletion) {
                        _extraction_triggered = true;
                        self.extract_memories_async(paladin, &conversation_history, execution_id);
                    }

                    return Ok(PaladinResult {
                        output: accumulated_output,
                        token_count: total_tokens,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        loop_count: loop_num,
                        stop_reason: effective_result.stop_reason,
                        plan: task_plan,
                        handoff_history,
                        served_by,
                    });
                }
            };

            let prompt = middleware_cx.assembly.render();

            // Execute with retry and circuit breaker (using effective temperature)
            let response = self
                .execute_with_retry_and_temperature(
                    paladin,
                    &prompt,
                    effective_temperature,
                    execution_id,
                    loop_num,
                    &middleware_cx,
                )
                .await?;

            // --- FT-FR-09, D-19: an LLM call completed -- progress.
            if let Some(heartbeat) = heartbeat {
                heartbeat.beat();
            }

            // Update accumulated token count -- BEFORE after_model, so a
            // built-in like TokenBudget reads the run's true running sum
            // (D-08).
            total_tokens += response.usage.total_tokens;
            middleware_cx.cumulative_tokens = total_tokens;
            if let Some(provider) = response.metadata.get(SERVED_BY_METADATA_KEY) {
                served_by = Some(provider.clone());
            }

            // --- D-04: after_model fires on the FINAL response for this
            // iteration -- after the service's own buffered retry and
            // circuit breaker, never per attempt.
            let mut response_view = LlmResponseView::from_response(&response);
            if let Some(final_result) = run_after(
                &self.middleware,
                &mut middleware_cx,
                &mut response_view,
                reached,
            )
            .await?
            {
                accumulated_output = response_view.content;

                if let Some(garrison) = &self.garrison {
                    let assistant_entry =
                        GarrisonEntry::new(ConversationRole::Assistant, accumulated_output.clone());
                    garrison.remember(assistant_entry).await?;
                }
                if self.should_extract_memories(MemoryExtractionStrategy::OnCompletion) {
                    _extraction_triggered = true;
                    self.extract_memories_async(paladin, &conversation_history, execution_id);
                }

                return Ok(PaladinResult {
                    output: accumulated_output,
                    token_count: total_tokens,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    loop_count: loop_num,
                    stop_reason: final_result.stop_reason,
                    plan: task_plan,
                    handoff_history,
                    served_by,
                });
            }

            accumulated_output = response_view.content;

            // =======================================================================
            // LAYER 3: Handoff Detection & Execution (Optional, Post-LLM)
            // =======================================================================

            // Check for tool calls and execute them if arsenal is available
            if let Some(function_call) = response_view.function_call.clone() {
                // Check if this is a handoff tool call (Layer 3)
                if self.is_handoff_tool_call(&function_call) {
                    info!(
                        "Handoff tool call detected: id={}, tool={}, loop={}",
                        execution_id, function_call.name, loop_num
                    );

                    // --- D-04: around_tool wraps the handoff branch too --
                    // a handoff is a tool call the model made.
                    let call = Self::function_call_to_armament_call(&function_call);
                    let mut tool_cx = ToolCallContext {
                        call: call.clone(),
                        kind: ToolCallKind::Handoff,
                        loop_index: middleware_cx.loop_index,
                        run_id: execution_id,
                        scratch: middleware_cx.scratch.clone(),
                    };
                    let flow = run_around_tool(&self.middleware, &mut tool_cx).await?;
                    // D-08: a stateful around_tool hook (e.g. ToolCallLimit)
                    // may have written to its working copy -- persist it
                    // back onto the run's own scratch so the next dispatch
                    // sees it.
                    middleware_cx.scratch = tool_cx.scratch;

                    match flow {
                        ToolFlow::Deny { reason } => {
                            let error_message = format!(
                                "\n\n🤝 Handoff Execution: {}\nResult: FAILED\nError: {}\n",
                                function_call.name, reason
                            );
                            accumulated_output.push_str(&error_message);
                        }
                        _ => {
                            let effective_call = match flow {
                                ToolFlow::Rewrite(rewritten) => rewritten,
                                _ => call,
                            };
                            let effective_function_call =
                                Self::armament_call_to_function_call(&effective_call);

                            // Execute handoff via HandoffService with retry logic
                            match self
                                .execute_handoff(
                                    &effective_function_call,
                                    paladin,
                                    execution_id,
                                    &mut handoff_history,
                                )
                                .await
                            {
                                Ok(handoff_result) => {
                                    accumulated_output.push_str("\n\n");
                                    accumulated_output.push_str(&handoff_result);

                                    // Store handoff result in garrison if available
                                    if let Some(garrison) = &self.garrison {
                                        let tool_entry = GarrisonEntry::new(
                                            ConversationRole::Tool,
                                            handoff_result,
                                        );
                                        garrison.remember(tool_entry).await?;
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Handoff execution failed: id={}, error={}",
                                        execution_id, e
                                    );
                                    // D-33/D-34: route through the shared
                                    // formatter, branching on the effective
                                    // tool-error policy for this tool name.
                                    match self
                                        .effective_tool_error_mode(&effective_function_call.name)
                                    {
                                        ToolErrorMode::FeedToModel => {
                                            let error_message = self
                                                .formatter
                                                .format_error(&effective_call, &e.to_string());
                                            accumulated_output.push_str("\n\n");
                                            accumulated_output.push_str(&error_message);
                                        }
                                        ToolErrorMode::FailRun => {
                                            return Err(PaladinError::ArmamentFailed {
                                                tool: effective_function_call.name.clone(),
                                                reason: e.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(ref arsenal) = effective_arsenal {
                    // Regular tool execution (not a handoff)
                    debug!(
                        "Tool call detected: id={}, tool={}, loop={}",
                        execution_id, function_call.name, loop_num
                    );

                    match Self::parse_armament_call(&function_call) {
                        Err(e) => {
                            warn!(
                                "Tool execution failed: id={}, tool={}, error={}",
                                execution_id, function_call.name, e
                            );
                            let error_message = format!(
                                "\n\n🔧 Tool Execution: {}\nResult: FAILED\nError: {}\n",
                                function_call.name, e
                            );
                            accumulated_output.push_str(&error_message);
                        }
                        Ok(call) => {
                            // --- D-04: around_tool wraps the Arsenal branch.
                            let mut tool_cx = ToolCallContext {
                                call: call.clone(),
                                kind: ToolCallKind::Armament,
                                loop_index: middleware_cx.loop_index,
                                run_id: execution_id,
                                scratch: middleware_cx.scratch.clone(),
                            };
                            let flow = run_around_tool(&self.middleware, &mut tool_cx).await?;
                            // D-08: persist a stateful around_tool hook's
                            // working-copy mutations back onto the run's
                            // scratch (see the handoff branch above).
                            middleware_cx.scratch = tool_cx.scratch;

                            match flow {
                                ToolFlow::Deny { reason } => {
                                    let error_message = format!(
                                        "\n\n🔧 Tool Execution: {}\nResult: FAILED\nError: {}\n",
                                        function_call.name, reason
                                    );
                                    accumulated_output.push_str(&error_message);
                                }
                                _ => {
                                    let effective_call = match flow {
                                        ToolFlow::Rewrite(rewritten) => rewritten,
                                        _ => call,
                                    };

                                    let tool_outcome = self
                                        .handle_tool_call(
                                            effective_call.clone(),
                                            arsenal.as_ref(),
                                            execution_id,
                                        )
                                        .await;
                                    // --- FT-FR-09, D-19: an Armament invocation resolved
                                    // (succeeded OR failed -- either way the node made
                                    // observable progress, not a stall).
                                    if let Some(heartbeat) = heartbeat {
                                        heartbeat.beat();
                                    }

                                    match tool_outcome {
                                        Ok(formatted_result) => {
                                            debug!(
                                                "Tool execution succeeded: id={}, tool={}",
                                                execution_id, effective_call.tool_name
                                            );
                                            // Inject tool result into accumulated output for next iteration
                                            accumulated_output.push_str("\n\n");
                                            accumulated_output.push_str(&formatted_result);

                                            // Store tool result in garrison if available
                                            if let Some(garrison) = &self.garrison {
                                                let tool_entry = GarrisonEntry::new(
                                                    ConversationRole::Tool,
                                                    formatted_result,
                                                );
                                                garrison.remember(tool_entry).await?;
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Tool execution failed: id={}, tool={}, error={}",
                                                execution_id, effective_call.tool_name, e
                                            );
                                            // D-33/D-34: route through the
                                            // shared formatter, branching on
                                            // the effective tool-error
                                            // policy for this tool name.
                                            match self.effective_tool_error_mode(
                                                &effective_call.tool_name,
                                            ) {
                                                ToolErrorMode::FeedToModel => {
                                                    let error_message =
                                                        self.formatter.format_error(
                                                            &effective_call,
                                                            &e.to_string(),
                                                        );
                                                    accumulated_output.push_str("\n\n");
                                                    accumulated_output.push_str(&error_message);
                                                }
                                                ToolErrorMode::FailRun => {
                                                    return Err(PaladinError::ArmamentFailed {
                                                        tool: effective_call.tool_name.clone(),
                                                        reason: e.to_string(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    warn!(
                        "Tool call requested but no arsenal available: id={}, tool={}",
                        execution_id, function_call.name
                    );
                }
            }

            // Check for stop words
            if let Some(stop_word) = self.check_stop_words(paladin, &accumulated_output) {
                warn!(
                    "Stop word detected: id={}, word={}, loop={}",
                    execution_id, stop_word, loop_num
                );
                return Err(PaladinError::StopWordDetected(stop_word));
            }

            // Check if we've reached max loops
            if loop_num == paladin.node.max_loops.as_u32() {
                debug!(
                    "Reached max loops: id={}, loops={}",
                    execution_id, paladin.node.max_loops
                );

                // Store assistant response in garrison if available
                if let Some(garrison) = &self.garrison {
                    let assistant_entry =
                        GarrisonEntry::new(ConversationRole::Assistant, accumulated_output.clone());
                    garrison.remember(assistant_entry).await?;
                    debug!(
                        "Stored assistant response in garrison: execution_id={}",
                        execution_id
                    );
                }

                // Trigger memory extraction on completion if configured
                if self.should_extract_memories(MemoryExtractionStrategy::OnCompletion) {
                    _extraction_triggered = true;
                    self.extract_memories_async(paladin, &conversation_history, execution_id);
                }

                // Return result with autonomous metadata (Phase 2 enhancement)
                return Ok(PaladinResult {
                    output: accumulated_output,
                    token_count: total_tokens,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    loop_count: loop_num,
                    stop_reason: StopReason::MaxLoops,
                    plan: task_plan, // Layer 1 metadata
                    handoff_history, // Layer 3 metadata
                    served_by,       // FT-FR-17: serving provider (fallback chains only)
                });
            }
        }

        // This should not be reached due to the loop logic, but provide a fallback
        // Store response in garrison before returning
        if let Some(garrison) = &self.garrison {
            let assistant_entry =
                GarrisonEntry::new(ConversationRole::Assistant, accumulated_output.clone());
            garrison.remember(assistant_entry).await?;
        }

        // Trigger memory extraction on completion if configured
        if self.should_extract_memories(MemoryExtractionStrategy::OnCompletion) {
            _extraction_triggered = true;
            self.extract_memories_async(paladin, &conversation_history, execution_id);
        }

        Ok(PaladinResult {
            output: accumulated_output,
            token_count: total_tokens,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            loop_count: paladin.node.max_loops.as_u32(),
            stop_reason: StopReason::Completed,
            plan: task_plan, // Layer 1 metadata
            handoff_history, // Layer 3 metadata
            served_by,       // FT-FR-17: serving provider (fallback chains only)
        })
    }

    /// Builds the prompt for an LLM call
    ///
    /// Combines the system prompt, RAG context, conversation history from Garrison,
    /// user input, and accumulated output from previous loops.
    // Exercised by unit tests; not on the default execution path.
    #[allow(dead_code)]
    fn build_prompt_with_history_and_rag(
        &self,
        paladin: &Paladin,
        input: &str,
        accumulated_output: &str,
        conversation_history: &[GarrisonEntry],
        rag_context: Option<&str>,
    ) -> String {
        let mut prompt = format!("{}\n\n", paladin.node.system_prompt);

        // Inject RAG context if available
        if let Some(context) = rag_context
            && !context.is_empty()
        {
            prompt.push_str("## Relevant Context from Memory\n");
            prompt.push_str(context);
            prompt.push_str("\n\n");
        }

        // Add conversation history if available
        if !conversation_history.is_empty() {
            prompt.push_str("Previous conversation:\n");
            for entry in conversation_history.iter().rev().take(10).rev() {
                // Most recent 10 entries
                let role_str = match entry.role {
                    ConversationRole::System => "System",
                    ConversationRole::User => "User",
                    ConversationRole::Assistant => "Assistant",
                    ConversationRole::Tool => "Tool",
                };
                prompt.push_str(&format!("{}: {}\n", role_str, entry.content));
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!("User: {}\n", input));

        if !accumulated_output.is_empty() {
            prompt.push_str(&format!("Previous output: {}\n", accumulated_output));
        }

        prompt
    }

    // `build_prompt_with_custom_system` (the Layer-1-aware prompt builder
    // the reasoning loop used to call directly) is replaced by
    // `middleware::PromptAssembly::new` + `PromptAssembly::render` (D-02):
    // the loop now builds a `PromptAssembly` from exactly the same inputs,
    // runs the `before_model` chain over it, and renders it through the
    // same code path this method used to contain directly. `render()`'s
    // own doc test pins the byte-identical output.

    /// Checks if Sanctum (RAG) is configured and ready
    fn check_sanctum_configured(&self) -> bool {
        self.rag_retrieval_service.is_some()
    }

    /// Retrieves context from Sanctum with timeout
    ///
    /// Wraps RAG retrieval in a 5-second timeout to prevent blocking execution.
    async fn retrieve_context_with_timeout(
        &self,
        paladin: &Paladin,
        query: &str,
        execution_id: uuid::Uuid,
    ) -> Result<Vec<paladin_ports::output::sanctum_port::SanctumSearchResult>, PaladinError> {
        if let Some(ref rag_service) = self.rag_retrieval_service {
            let paladin_id = paladin.uuid.to_string();

            match timeout(
                Duration::from_secs(5),
                rag_service.retrieve_context(&paladin_id, query),
            )
            .await
            {
                Ok(Ok(results)) => {
                    debug!(
                        "RAG retrieval completed: execution_id={}, results={}",
                        execution_id,
                        results.len()
                    );
                    Ok(results)
                }
                Ok(Err(e)) => {
                    warn!(
                        "RAG retrieval failed: execution_id={}, error={}",
                        execution_id, e
                    );
                    Err(PaladinError::ExecutionError(format!(
                        "RAG retrieval failed: {}",
                        e
                    )))
                }
                Err(_) => {
                    warn!("RAG retrieval timed out: execution_id={}", execution_id);
                    Err(PaladinError::Timeout(5))
                }
            }
        } else {
            Err(PaladinError::ConfigurationError(
                "RAG retrieval service not configured".to_string(),
            ))
        }
    }

    /// Formats retrieved search results into a context string for injection
    fn format_retrieved_context(
        &self,
        results: &[paladin_ports::output::sanctum_port::SanctumSearchResult],
    ) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut context = String::new();
        for (i, result) in results.iter().enumerate() {
            context.push_str(&format!(
                "{}. [Score: {:.2}] {}\n",
                i + 1,
                result.score,
                result.entry.memory.content
            ));
        }
        context
    }

    /// Checks if memories should be extracted based on the strategy
    fn should_extract_memories(&self, strategy: MemoryExtractionStrategy) -> bool {
        self.memory_extraction_service.is_some()
            && matches!(strategy, MemoryExtractionStrategy::OnCompletion)
    }

    /// Spawns an async task to extract memories in the background
    ///
    /// This runs asynchronously so it doesn't block Paladin execution completion.
    fn extract_memories_async(
        &self,
        paladin: &Paladin,
        conversation_history: &[GarrisonEntry],
        execution_id: uuid::Uuid,
    ) {
        if let Some(ref extraction_service) = self.memory_extraction_service {
            let paladin_id = paladin.uuid.to_string();
            let conversation = conversation_history.to_vec();
            let service = Arc::clone(extraction_service);

            // Spawn background task
            tokio::spawn(async move {
                let start = Instant::now();
                debug!(
                    "Starting background memory extraction: execution_id={}, conversations={}",
                    execution_id,
                    conversation.len()
                );

                match service.extract_memories(&paladin_id, &conversation).await {
                    Ok(extracted_entries) => {
                        let elapsed = start.elapsed();
                        info!(
                            "Memory extraction completed: execution_id={}, memories_extracted={}, duration_ms={}",
                            execution_id,
                            extracted_entries.len(),
                            elapsed.as_millis()
                        );
                    }
                    Err(e) => {
                        let elapsed = start.elapsed();
                        warn!(
                            "Memory extraction failed: execution_id={}, error={}, duration_ms={}",
                            execution_id,
                            e,
                            elapsed.as_millis()
                        );
                    }
                }
            });
        }
    }

    /// Checks if any stop words are present in the output
    ///
    /// Performs case-insensitive exact word matching.
    ///
    /// # Returns
    ///
    /// - `Some(stop_word)` if a stop word is found
    /// - `None` if no stop words are found
    fn check_stop_words(&self, paladin: &Paladin, output: &str) -> Option<String> {
        let output_lower = output.to_lowercase();

        for stop_word in paladin.node.stop_words.iter() {
            let stop_word_lower = stop_word.to_lowercase();

            // Check for exact word match (case-insensitive)
            if output_lower.contains(&stop_word_lower) {
                return Some(stop_word.clone());
            }
        }

        None
    }

    //
    // ==================== AUTONOMOUS ORCHESTRATION LAYERS ====================
    // Phase 4 (Epic 21): Layered autonomous feature execution
    // Layer 0: Core execution (always runs)
    // Layer 1: Planning & Prompts (optional, pre-execution)
    // Layer 2: Dynamic Temperature (optional, per-loop)
    // Layer 3: Handoff Detection (optional, post-LLM)
    // =========================================================================
    //

    /// Layer 1: Apply planning if autonomous_planning is enabled
    ///
    /// Generates a task plan before execution begins. If planning fails,
    /// logs a warning and continues with core execution (graceful degradation).
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin configuration
    /// * `input` - User input/task description
    /// * `execution_id` - Unique execution ID for logging
    ///
    /// # Returns
    ///
    /// `Some(TaskPlan)` if planning succeeds, `None` if disabled or failed
    async fn apply_layer1_planning(
        &self,
        paladin: &Paladin,
        input: &str,
        execution_id: uuid::Uuid,
    ) -> Option<crate::core::platform::container::planning::TaskPlan> {
        // Check if planning is enabled
        if !paladin.node.autonomous_planning {
            debug!(
                "Planning disabled: execution_id={}, autonomous_planning=false",
                execution_id
            );
            return None;
        }

        // Check if planning service is available
        let planning_service = match &self.planning_service {
            Some(service) => service,
            None => {
                warn!(
                    "Planning enabled but no planning service configured: execution_id={}",
                    execution_id
                );
                return None;
            }
        };

        info!(
            "Generating task plan: execution_id={}, input_len={}",
            execution_id,
            input.len()
        );

        // Use model from paladin config (Phase 1 enhancement)
        let model = paladin.node.model.as_str();

        // Attempt to generate plan with graceful degradation
        match planning_service.create_plan(input, 10, model).await {
            Ok(plan) => {
                info!(
                    "Planning succeeded: execution_id={}, subtasks={}",
                    execution_id,
                    plan.subtasks.len()
                );
                Some(plan)
            }
            Err(e) => {
                warn!(
                    "Planning failed, continuing with core execution: execution_id={}, error={}",
                    execution_id, e
                );
                None
            }
        }
    }

    /// Layer 1: Apply prompt generation if autonomous_prompts is enabled
    ///
    /// Generates a dynamic system prompt based on agent description. If generation
    /// fails, logs a warning and uses the existing system prompt (graceful degradation).
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin configuration (may be mutated if prompt generation succeeds)
    /// * `execution_id` - Unique execution ID for logging
    ///
    /// # Returns
    ///
    /// `Some(String)` with the generated prompt if successful, `None` if disabled or failed
    async fn apply_layer1_prompt_generation(
        &self,
        paladin: &Paladin,
        execution_id: uuid::Uuid,
    ) -> Option<String> {
        // Check if prompt generation is enabled
        if !paladin.node.autonomous_prompts {
            debug!(
                "Prompt generation disabled: execution_id={}, autonomous_prompts=false",
                execution_id
            );
            return None;
        }

        // Check if prompt generation service is available
        let prompt_service = match &self.prompt_generation_service {
            Some(service) => service,
            None => {
                warn!(
                    "Prompt generation enabled but no prompt service configured: execution_id={}",
                    execution_id
                );
                return None;
            }
        };

        // Check if agent has a description (required for prompt generation)
        if paladin.node.agent_description.is_empty() {
            warn!(
                "Prompt generation enabled but agent_description is empty: execution_id={}",
                execution_id
            );
            return None;
        }

        info!(
            "Generating system prompt: execution_id={}, agent={}",
            execution_id, paladin.node.name
        );

        // Use model from paladin config (Phase 1 enhancement)
        let model = paladin.node.model.as_str();

        // Attempt to generate prompt with graceful degradation
        match prompt_service
            .generate_prompt(&paladin.node.name, &paladin.node.agent_description, model)
            .await
        {
            Ok(generated_prompt) => {
                info!(
                    "Prompt generation succeeded: execution_id={}, prompt_len={}",
                    execution_id,
                    generated_prompt.len()
                );
                Some(generated_prompt)
            }
            Err(e) => {
                warn!(
                    "Prompt generation failed, using original prompt: execution_id={}, error={}",
                    execution_id, e
                );
                None
            }
        }
    }

    /// Layer 2: Calculate dynamic temperature for current loop iteration
    ///
    /// Applies temperature adjustment based on loop progress if dynamic_temperature is enabled.
    /// Temperature increases linearly from configured base to 1.0 over max_loops.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin configuration
    /// * `loop_num` - Current loop iteration (1-indexed)
    ///
    /// # Returns
    ///
    /// Adjusted temperature value (base_temp + progress * (1.0 - base_temp))
    fn apply_layer2_dynamic_temperature(&self, paladin: &Paladin, loop_num: u32) -> f32 {
        // If dynamic temperature is disabled, return configured temperature
        if !paladin.node.dynamic_temperature {
            return paladin.node.temperature;
        }

        let base_temp = paladin.node.temperature;
        let max_loops = paladin.node.max_loops.as_u32() as f32;
        let current_loop = loop_num as f32;

        // Linear interpolation: temp = base + progress * (1.0 - base)
        // Loop 1: base_temp, Loop max: 1.0
        let progress = (current_loop - 1.0) / (max_loops - 1.0).max(1.0);
        let adjusted_temp = base_temp + progress * (1.0 - base_temp);

        debug!(
            "Dynamic temperature: loop={}/{}, base={}, adjusted={}",
            loop_num, paladin.node.max_loops, base_temp, adjusted_temp
        );

        adjusted_temp.clamp(0.0, 1.0)
    }

    /// Layer 3: Check if response contains handoff tool call
    ///
    /// Examines the LLM function call to determine if it's a handoff request.
    /// Handoff execution will be implemented in Phase 5.
    ///
    /// # Arguments
    ///
    /// * `function_call` - The function call from LLM response
    ///
    /// # Returns
    ///
    /// `true` if this is a handoff tool call, `false` otherwise
    fn is_handoff_tool_call(&self, function_call: &FunctionCall) -> bool {
        function_call.name == "handoff_to_specialist"
    }

    /// Layer 3: Execute a handoff to a specialist Paladin
    ///
    /// Parses the handoff tool call arguments, validates the specialist,
    /// and delegates execution via `HandoffService`. If no `HandoffService`
    /// is configured, returns a placeholder message.
    ///
    /// # Arguments
    ///
    /// * `function_call` - The handoff tool call from LLM
    /// * `paladin` - The current Paladin (coordinator)
    /// * `execution_id` - Unique execution ID for logging
    /// * `handoff_history` - Mutable reference to accumulate handoff records
    ///
    /// # Returns
    ///
    /// Formatted handoff result to inject into conversation
    async fn execute_handoff(
        &self,
        function_call: &FunctionCall,
        paladin: &Paladin,
        execution_id: uuid::Uuid,
        handoff_history: &mut Vec<crate::core::platform::container::handoff::HandoffRecord>,
    ) -> Result<String, PaladinError> {
        // Parse specialist name and task from function call arguments
        let args: Value = serde_json::from_str(&function_call.arguments).unwrap_or_default();
        let specialist_name = args["specialist_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let task_description = args["task_description"].as_str().unwrap_or("").to_string();

        info!(
            "Handoff execution: id={}, specialist={}, task_len={}",
            execution_id,
            specialist_name,
            task_description.len()
        );

        // Check if HandoffService is configured
        let handoff_service = match &self.handoff_service {
            Some(service) => service,
            None => {
                warn!(
                    "Handoff detected but no HandoffService configured: id={}",
                    execution_id
                );
                return Ok(format!(
                    "\n\n\u{1f91d} Handoff to '{}': No HandoffService configured. \
                     Configure with_handoff_service() on PaladinExecutionService.\n",
                    specialist_name
                ));
            }
        };

        // Create handoff context from the coordinator
        let context = crate::core::platform::container::handoff::HandoffContext::new(
            task_description.clone(),
            paladin.node.name.clone(),
        );

        // Execute the handoff via HandoffService with retry logic
        // Note: In a full implementation, specialist Paladins would be looked up
        // from a registry. For Phase 5, we use the coordinator as a stand-in.
        match handoff_service
            .execute_handoff(&specialist_name, &task_description, &context, paladin, self)
            .await
        {
            Ok((result, record)) => {
                info!(
                    "Handoff completed: id={}, specialist={}, result_len={}",
                    execution_id,
                    specialist_name,
                    result.len()
                );
                handoff_history.push(record);
                Ok(format!(
                    "\n\n\u{1f91d} Handoff to '{}':\n{}\n",
                    specialist_name, result
                ))
            }
            Err(handoff_err) => {
                warn!(
                    "Handoff failed: id={}, specialist={}, error={}",
                    execution_id, specialist_name, handoff_err
                );
                Err(PaladinError::ExecutionError(format!(
                    "Handoff to '{}' failed: {}",
                    specialist_name, handoff_err
                )))
            }
        }
    }

    //
    // ==================== END AUTONOMOUS LAYERS ====================
    //

    /// Executes an LLM call with retry logic, circuit breaker, and custom temperature
    ///
    /// This variant supports Layer 2 (dynamic temperature) by accepting a temperature
    /// parameter instead of using paladin.node.temperature.
    ///
    /// With no retry policy set on `cx`, implements today's exponential
    /// backoff unchanged: 100ms, 200ms, 400ms, etc. (D-11, X-03).
    ///
    /// # The single port/policy resolution point (D-11, assumption-delta)
    ///
    /// This is the ONE site in the service that resolves which
    /// [`paladin_ports::output::llm_port::LlmPort`] a model call reaches
    /// and which retry policy governs it: `cx.effective_llm(&self.llm_port)`
    /// (the override, if `ModelFallbackMiddleware` set one, else the
    /// service's own port, seeded as the default) and `cx.retry_policy`
    /// (set by `ModelRetryMiddleware`, else `None` -- today's shape). No
    /// other call site in this service chooses a port or a retry policy.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin configuration
    /// * `prompt` - The prompt to send to the LLM
    /// * `temperature` - The temperature value to use for this call
    /// * `execution_id` - Unique ID for this execution (for logging)
    /// * `loop_num` - Current loop iteration number
    /// * `cx` - The run's `ModelCallContext`, read (never mutated) for its
    ///   `llm_override` and `retry_policy` (D-11)
    ///
    /// # Returns
    ///
    /// LLM response on success
    ///
    /// # Errors
    ///
    /// Returns `PaladinError` if:
    /// - Circuit breaker is open
    /// - The failure is [`Transience::Permanent`](paladin_core::platform::container::transience::Transience::Permanent)
    ///   (e.g. a rejected credential or a malformed prompt) -- returned
    ///   immediately on the attempt that first observes it, never retried
    ///   (WR-02, `25-REVIEW.md`), whether or not a retry policy is active
    /// - An active retry policy's `RetryPredicate::admits` declines this
    ///   failure's transience (D-11) -- returned immediately, unretried
    /// - All retry attempts are exhausted
    /// - LLM call fails with a non-retryable error
    async fn execute_with_retry_and_temperature(
        &self,
        paladin: &Paladin,
        prompt: &str,
        temperature: f32,
        execution_id: uuid::Uuid,
        loop_num: u32,
        cx: &ModelCallContext<'_>,
    ) -> Result<paladin_ports::output::llm_port::LlmResponse, PaladinError> {
        // D-11 / the assumption-delta promote (see `ModelCallContext::effective_llm`'s
        // rustdoc): the effective port for THIS call is resolved here,
        // exactly once. The service's own `llm_port` is the seeded default;
        // `cx.llm_override` (set by `ModelFallbackMiddleware::before_model`)
        // is the override.
        let llm_port = cx.effective_llm(&self.llm_port);
        // D-11: a `ModelRetryMiddleware`-supplied policy drives attempts,
        // delays and the retry predicate below; with `None` the loop keeps
        // today's shape byte-for-byte (X-03).
        let retry_policy = cx.retry_policy.clone();
        let mut attempt = 0;
        let max_attempts = retry_policy
            .as_ref()
            .map(|policy| policy.max_attempts)
            .unwrap_or_else(|| paladin.node.max_loops.as_u32().min(10)); // Cap retries at 10 with no policy

        loop {
            attempt += 1;

            debug!(
                "LLM call attempt: id={}, loop={}, attempt={}/{}, temperature={}",
                execution_id, loop_num, attempt, max_attempts, temperature
            );

            // Create prompt item with custom temperature
            let prompt_data = PromptData {
                prompt_type: PromptType::User(UserPrompt {
                    query: prompt.to_string(),
                    context: None,
                }),
                content_attachments: vec![],
                parameters: PromptParameters {
                    max_tokens: None,
                    temperature: Some(temperature), // Use provided temperature
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    stop_sequences: if paladin.node.stop_words.is_empty() {
                        None
                    } else {
                        Some(paladin.node.stop_words.clone())
                    },
                },
                context: None,
                expected_output: None,
                tags: None,
                category: None,
                author: None,
                metadata: BTreeMap::new(),
            };

            let prompt_item = PromptItem {
                node: Node::new(prompt_data, Some(format!("execution-{}", execution_id))),
            };

            // Create LLM request. `cx.response_format` (RT-05, D-27, D-28) is
            // set exactly once, by a structured run's `execute_json_schema` --
            // an ordinary run's context never sets it, so this call is a
            // no-op for every existing caller (X-03).
            let mut request = LlmRequest::new(paladin.node.model.clone(), prompt_item);
            if let Some(response_format) = cx.response_format.clone() {
                request = request.with_response_format(response_format);
            }

            // Wrap LLM call with circuit breaker (async version). Uses the
            // resolved `llm_port` (D-11's single resolution point), not
            // `self.llm_port` directly.
            let port = Arc::clone(&llm_port);
            let result = self
                .circuit_breaker
                .call_async(async move {
                    match port.generate(request).await {
                        Ok(response) => Ok(response),
                        Err(e) => Err(to_paladin_error(&e)),
                    }
                })
                .await;

            match result {
                Ok(response) => {
                    debug!(
                        "LLM call succeeded: id={}, loop={}, attempt={}",
                        execution_id, loop_num, attempt
                    );
                    return Ok(response);
                }
                Err(PaladinError::CircuitBreakerOpen) => {
                    // Circuit breaker is open, fail fast
                    error!(
                        "Circuit breaker open: id={}, loop={}",
                        execution_id, loop_num
                    );
                    return Err(PaladinError::CircuitBreakerOpen);
                }
                // WR-02 (`25-REVIEW.md`), D-11: a `Permanent` failure (e.g. a
                // rejected credential or a malformed prompt) needs operator
                // intervention, not a retry -- the identical request would
                // fail identically on every subsequent attempt, whether or
                // not a retry policy is active. Checked BEFORE any
                // policy/attempt-budget arm so it fails fast on the FIRST
                // attempt rather than burning the full retry budget first.
                // Returns the underlying typed error (usually
                // `PaladinError::LlmFailure`) rather than
                // `MaxRetriesExceeded`, so the operator sees the real
                // cause -- every adapter's own retry loop in this crate
                // (`anthropic::execute_with_retry`,
                // `deepseek::call_api_with_retry`,
                // `compat::CompatEngine::call_api_with_retry`) already
                // excludes its permanent-failure set for the same reason.
                Err(e) if e.transience() == Transience::Permanent => {
                    error!(
                        "LLM call failed permanently, not retrying: id={}, loop={}, attempt={}, error={}",
                        execution_id, loop_num, attempt, e
                    );
                    return Err(e);
                }
                // D-11: with an active retry policy, `RetryPredicate::admits`
                // -- the SAME pure function
                // `paladin_battalion::engine::retry::should_retry` calls --
                // decides whether this transience is retried at all. With no
                // policy this arm never matches (`unwrap_or(false)`), so
                // every non-Permanent failure remains retryable, exactly as
                // today.
                Err(e)
                    if retry_policy
                        .as_ref()
                        .map(|policy| !policy.retry_on.admits(e.transience()))
                        .unwrap_or(false) =>
                {
                    warn!(
                        "LLM call failed with a policy-declined transience, not retrying: id={}, loop={}, attempt={}, error={}",
                        execution_id, loop_num, attempt, e
                    );
                    return Err(e);
                }
                Err(_e) if attempt >= max_attempts => {
                    // Exhausted retries
                    error!(
                        "Max retries exhausted: id={}, loop={}, attempts={}",
                        execution_id, loop_num, attempt
                    );
                    return Err(PaladinError::MaxRetriesExceeded(attempt));
                }
                Err(e) => {
                    // D-11: the delay before the NEXT attempt comes from
                    // `paladin_battalion::engine::retry::backoff_delay` when
                    // a policy is active; with none, today's
                    // `100ms * 2^(attempt-1)` shape is unchanged (X-03).
                    let backoff = match &retry_policy {
                        Some(policy) => {
                            paladin_battalion::engine::retry::backoff_delay(policy, attempt + 1)
                        }
                        None => Duration::from_millis(100 * 2u64.pow(attempt - 1)), // 100ms, 200ms, 400ms, ...
                    };
                    warn!(
                        "LLM call failed, retrying: id={}, loop={}, attempt={}, backoff_ms={}, error={}",
                        execution_id,
                        loop_num,
                        attempt,
                        backoff.as_millis(),
                        e
                    );
                    sleep(backoff).await;
                }
            }
        }
    }

    /// Executes an LLM call with retry logic and circuit breaker
    ///
    /// Implements exponential backoff: 100ms, 200ms, 400ms, etc.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin configuration
    /// * `prompt` - The prompt to send to the LLM
    /// * `execution_id` - Unique ID for this execution (for logging)
    /// * `loop_num` - Current loop iteration number
    ///
    /// # Returns
    ///
    /// The LLM response or an error after exhausting retries
    // Retry + circuit-breaker call path; retained pending integration into the execution loop.
    #[allow(dead_code)]
    async fn execute_with_retry(
        &self,
        paladin: &Paladin,
        prompt: &str,
        execution_id: uuid::Uuid,
        loop_num: u32,
    ) -> Result<paladin_ports::output::llm_port::LlmResponse, PaladinError> {
        let mut attempt = 0;
        let max_attempts = paladin.node.max_loops.as_u32().min(10); // Cap retries at 10

        loop {
            attempt += 1;

            debug!(
                "LLM call attempt: id={}, loop={}, attempt={}/{}",
                execution_id, loop_num, attempt, max_attempts
            );

            // Create prompt item
            let prompt_data = PromptData {
                prompt_type: PromptType::User(UserPrompt {
                    query: prompt.to_string(),
                    context: None,
                }),
                content_attachments: vec![],
                parameters: PromptParameters {
                    max_tokens: None,
                    temperature: Some(paladin.node.temperature),
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    stop_sequences: if paladin.node.stop_words.is_empty() {
                        None
                    } else {
                        Some(paladin.node.stop_words.clone())
                    },
                },
                context: None,
                expected_output: None,
                tags: None,
                category: None,
                author: None,
                metadata: BTreeMap::new(),
            };

            let prompt_item = PromptItem {
                node: Node::new(prompt_data, Some(format!("execution-{}", execution_id))),
            };

            // Create LLM request
            let request = LlmRequest::new(paladin.node.model.clone(), prompt_item);

            // Wrap LLM call with circuit breaker (async version)
            let llm_port = Arc::clone(&self.llm_port);
            let result = self
                .circuit_breaker
                .call_async(async move {
                    match llm_port.generate(request).await {
                        Ok(response) => Ok(response),
                        Err(e) => Err(to_paladin_error(&e)),
                    }
                })
                .await;

            match result {
                Ok(response) => {
                    debug!(
                        "LLM call succeeded: id={}, loop={}, attempt={}",
                        execution_id, loop_num, attempt
                    );
                    return Ok(response);
                }
                Err(PaladinError::CircuitBreakerOpen) => {
                    // Circuit breaker is open, fail fast
                    error!(
                        "Circuit breaker open: id={}, loop={}",
                        execution_id, loop_num
                    );
                    return Err(PaladinError::CircuitBreakerOpen);
                }
                // WR-02 (`25-REVIEW.md`): see the identical arm in
                // `execute_with_retry_and_temperature` for the full
                // rationale -- a `Permanent` failure needs operator
                // intervention, not a retry, and is checked before the
                // `attempt >= max_attempts` arm so it fails fast on the
                // FIRST attempt.
                Err(e) if e.transience() == Transience::Permanent => {
                    error!(
                        "LLM call failed permanently, not retrying: id={}, loop={}, attempt={}, error={}",
                        execution_id, loop_num, attempt, e
                    );
                    return Err(e);
                }
                Err(_e) if attempt >= max_attempts => {
                    // Exhausted retries
                    error!(
                        "Max retries exhausted: id={}, loop={}, attempts={}",
                        execution_id, loop_num, attempt
                    );
                    return Err(PaladinError::MaxRetriesExceeded(attempt));
                }
                Err(e) => {
                    // Retry with exponential backoff
                    let backoff_ms = 100 * 2u64.pow(attempt - 1);
                    warn!(
                        "LLM call failed, retrying: id={}, loop={}, attempt={}, backoff_ms={}, error={}",
                        execution_id, loop_num, attempt, backoff_ms, e
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }

    /// Handles tool call execution and formatting
    ///
    /// Parses the function call, invokes the tool via Arsenal, and formats
    /// the result for injection into the conversation context.
    ///
    /// # Arguments
    ///
    /// * `function_call` - The function call details from the LLM
    /// * `arsenal` - The Arsenal port for tool execution
    /// * `execution_id` - Unique ID for this execution (for logging)
    ///
    /// # Returns
    ///
    /// Formatted tool result as a string, or error if tool execution fails
    async fn handle_tool_call(
        &self,
        call: ArmamentCall,
        arsenal: &dyn ArsenalPort,
        execution_id: uuid::Uuid,
    ) -> Result<String, ArsenalError> {
        debug!(
            "Invoking tool: id={}, tool={}, call_id={}",
            execution_id, call.tool_name, call.call_id
        );

        // Invoke the tool (clone because invoke takes ownership)
        let result = arsenal.invoke(call.clone()).await?;

        debug!(
            "Tool invocation completed: id={}, tool={}, success={}, time_ms={}",
            execution_id, call.tool_name, result.success, result.execution_time_ms
        );

        // Format result for LLM context
        let formatted = self.formatter.format_result(&call, &result);

        Ok(formatted)
    }

    /// Parse a [`FunctionCall`]'s JSON arguments into an [`ArmamentCall`]
    /// for Arsenal dispatch (Doc 05 D-04: the `around_tool` hook needs a
    /// concrete `ArmamentCall` before dispatch, so this parse -- previously
    /// inline in `handle_tool_call` -- now happens before the hook fires).
    ///
    /// # Errors
    ///
    /// Returns `ArsenalError::InvalidArguments` if `function_call.arguments`
    /// is not valid JSON -- exactly the error `handle_tool_call` used to
    /// surface, at exactly the same point (before any Arsenal dispatch).
    fn parse_armament_call(function_call: &FunctionCall) -> Result<ArmamentCall, ArsenalError> {
        let arguments: HashMap<String, Value> = serde_json::from_str(&function_call.arguments)
            .map_err(|e| {
                ArsenalError::InvalidArguments(format!("Failed to parse arguments JSON: {}", e))
            })?;
        Ok(ArmamentCall::new(&function_call.name, arguments))
    }

    /// Build an [`ArmamentCall`] representing a handoff tool call, for the
    /// `around_tool` hook (Doc 05 D-04: a handoff is a tool call the model
    /// made). Malformed JSON silently defaults to an empty argument map --
    /// the same fallback `execute_handoff`'s own parsing already applies,
    /// so this introduces no new failure mode on the handoff path.
    fn function_call_to_armament_call(function_call: &FunctionCall) -> ArmamentCall {
        let arguments: HashMap<String, Value> =
            serde_json::from_str(&function_call.arguments).unwrap_or_default();
        ArmamentCall::new(&function_call.name, arguments)
    }

    /// The inverse of [`Self::function_call_to_armament_call`]: rebuild a
    /// [`FunctionCall`] from a (possibly `ToolFlow::Rewrite`-replaced)
    /// [`ArmamentCall`] so `execute_handoff` (which takes a `FunctionCall`)
    /// can dispatch it.
    fn armament_call_to_function_call(call: &ArmamentCall) -> FunctionCall {
        FunctionCall {
            name: call.tool_name.clone(),
            arguments: serde_json::to_string(&call.arguments).unwrap_or_default(),
        }
    }
}

/// Implementation of `PaladinExecutorPort` for `PaladinExecutionService`
///
/// This enables `HandoffService` to delegate specialist execution back to
/// `PaladinExecutionService` without a circular compile-time dependency.
/// The `HandoffService` depends on `Arc<dyn PaladinExecutorPort>`, while
/// `PaladinExecutionService` provides the concrete implementation.
#[async_trait::async_trait]
impl PaladinExecutorPort for PaladinExecutionService {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Delegate to the existing public execute method
        self.execute(paladin, input).await
    }
}

/// Streaming execution over [`LlmPort::generate_stream`].
///
/// This is a single-pass streaming path (LLM + prompt only — no tool loop, garrison, or
/// RAG): it composes the prompt the same way the buffered path does for the no-history
/// case, opens the provider stream, and forwards each delta as a [`PaladinStreamChunk`]
/// over an `mpsc` channel. A provider that does not support streaming surfaces an error
/// up front (before any chunk). Dropping the returned receiver (client disconnect or a
/// timeout) cancels the producer task on its next send.
///
/// # `ExecutionMiddleware` coverage (Doc 05 D-04)
///
/// This path runs `before_model` exactly once (there is one model call and
/// no tool loop) so a middleware can still observe/mutate the prompt
/// assembly before it renders. **`after_model` and `around_tool` are never
/// invoked here** — there is no discrete final response to hand
/// `after_model` (the provider streams deltas, not one `LlmResponse`) and
/// no tool-call loop for `around_tool` to wrap. Response-screening
/// middleware over a stream is a deferred idea.
#[async_trait::async_trait]
impl StreamingExecutorPort for PaladinExecutionService {
    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        self.execute_stream_inner(paladin, input, None).await
    }
}

/// `execute_structured<T>` becomes real (Doc 05 RT-05, RT-FR-17…19, D-26,
/// D-27): `PaladinExecutionService` implements the object-safe,
/// JSON-schema-level half of structured output natively.
///
/// For **every** model call this drives -- the first attempt and every
/// repair re-prompt alike -- [`Self::execute_structured_call`] sets both
/// mechanisms, belt and braces (D-27, D-28):
///
/// 1. `LlmRequest.response_format` is set to
///    [`ResponseFormat::JsonSchema`], so the four wired adapter paths
///    (plan 26-06: OpenAI, the compat engine, Gemini, DeepSeek) use the
///    provider's own native constrained-JSON mode; and
/// 2. [`render_instruction_block`] is appended to the prompt, so a
///    provider with no native mode at all (Anthropic) -- or one that
///    deliberately degrades the schema to a plain JSON-object request
///    (the compat engine, DeepSeek) -- is still constrained by the prompt
///    itself.
///
/// Correctness never depends on the provider's native mode: the prompt-level
/// instruction is the floor, the native `response_format` hint is the
/// (optional) improvement.
///
/// The bounded repair loop itself is **not** re-implemented here --
/// [`execute_json_schema`](StructuredExecutorPort::execute_json_schema)
/// supplies only the `execute_fn` closure
/// [`run_structured`](paladin_ports::output::structured_executor_port::run_structured)
/// (plan 26-12) drives; `execute_json_schema_observed` is left to the
/// trait's own defaulted body (the same D-19 pattern `PaladinPort::
/// execute_observed`/`execute_scoped` already use elsewhere in this
/// crate), since this service has no heartbeat plumbing to add for the
/// structured path.
///
/// `PaladinPort` gains nothing here (PRD 05 §2.4's explicit instruction,
/// X-10.4's stated reason `StructuredExecutorPort` exists at all) -- this
/// impl block is entirely additive to `PaladinExecutionService`'s own
/// inherent surface.
#[async_trait::async_trait]
impl StructuredExecutorPort for PaladinExecutionService {
    async fn execute_json_schema(
        &self,
        paladin: &Paladin,
        input: &str,
        schema: &Value,
        opts: &StructuredOptions,
    ) -> Result<Structured<Value>, PaladinError> {
        let execution_id = uuid::Uuid::new_v4();
        let schema_owned = schema.clone();
        let response_format = ResponseFormat::JsonSchema {
            name: "structured_output".to_string(),
            schema: schema.clone(),
            strict: true,
        };
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let execute_fn = move |current_input: String| {
            let schema = schema_owned.clone();
            let response_format = response_format.clone();
            let call_count = Arc::clone(&call_count);
            async move {
                let call_num = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                // D-27: belt and braces, on EVERY call -- regardless of
                // whether `current_input` (as built by `run_structured`,
                // either the first attempt's own instruction-block-appended
                // input or a later repair re-prompt) already carries schema
                // text in some form, this call's prompt is guaranteed to
                // contain `render_instruction_block`'s literal output too.
                let prompt = format!("{current_input}{}", render_instruction_block(&schema));
                self.execute_structured_call(
                    paladin,
                    &prompt,
                    execution_id,
                    call_num,
                    response_format,
                )
                .await
            }
        };

        run_structured(execute_fn, input, schema, opts).await
    }
}

impl PaladinExecutionService {
    /// One structured-output model call (RT-05, D-27): builds a scratch
    /// [`ModelCallContext`] carrying `response_format`, then dispatches
    /// through the SAME retry/circuit-breaker call site
    /// ([`Self::execute_with_retry_and_temperature`]) every ordinary
    /// reasoning-loop iteration uses -- not a second call path.
    ///
    /// Deliberately does **not** run the full multi-loop `execute_internal`
    /// reasoning loop: a structured request is one deterministic model call
    /// per [`run_structured`](paladin_ports::output::structured_executor_port::run_structured)
    /// attempt, so `loop_count` on the returned [`PaladinResult`] is
    /// `call_num` -- the count of model calls made so far in this
    /// structured run, across every repair attempt -- not the Paladin's own
    /// `max_loops`.
    async fn execute_structured_call(
        &self,
        paladin: &Paladin,
        prompt: &str,
        execution_id: uuid::Uuid,
        call_num: u32,
        response_format: ResponseFormat,
    ) -> Result<PaladinResult, PaladinError> {
        let assembly = PromptAssembly::new(String::new(), "", "", Vec::new(), None);
        let mut cx = ModelCallContext::new(execution_id, paladin, assembly);
        cx.response_format = Some(response_format);

        let response = self
            .execute_with_retry_and_temperature(
                paladin,
                prompt,
                paladin.node.temperature,
                execution_id,
                call_num,
                &cx,
            )
            .await?;

        Ok(PaladinResult {
            output: response.content,
            token_count: response.usage.total_tokens,
            loop_count: call_num,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }
}

impl PaladinExecutionService {
    /// Streaming execution that beats `heartbeat` once per forwarded chunk
    /// (Doc 04 FT-FR-09, D-19; plan 25-09).
    ///
    /// Identical to [`StreamingExecutorPort::execute_stream`] -- same prompt
    /// composition, same provider stream, same chunk forwarding -- with one
    /// addition: every chunk sent to the returned receiver first beats
    /// `heartbeat`, so a stream that emits a chunk every 100 ms keeps its
    /// node's `idle_timeout` timer reset while one that stalls past the idle
    /// window does not. The beat happens BEFORE the send, so a slow
    /// consumer never delays the progress report.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_core::platform::container::heartbeat::HeartbeatHandle;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>, service: PaladinExecutionService) -> Result<(), Box<dyn std::error::Error>> {
    /// # let paladin = PaladinBuilder::new(llm_port).system_prompt("test").build().await?;
    /// let heartbeat = HeartbeatHandle::new();
    /// let mut stream = service.execute_stream_observed(&paladin, "Report", &heartbeat).await?;
    /// while let Some(chunk) = stream.recv().await {
    ///     let chunk = chunk?;
    ///     if chunk.is_final {
    ///         break;
    ///     }
    /// }
    /// assert!(heartbeat.beats() >= 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_stream_observed(
        &self,
        paladin: &Paladin,
        input: &str,
        heartbeat: &HeartbeatHandle,
    ) -> Result<PaladinStream, PaladinError> {
        self.execute_stream_inner(paladin, input, Some(heartbeat.clone()))
            .await
    }

    /// The shared body of `execute_stream` and `execute_stream_observed`:
    /// `heartbeat` is `Some` only on the observed path and is beaten once
    /// per forwarded chunk; the unobserved path is byte-identical to before
    /// plan 25-09.
    async fn execute_stream_inner(
        &self,
        paladin: &Paladin,
        input: &str,
        heartbeat: Option<HeartbeatHandle>,
    ) -> Result<PaladinStream, PaladinError> {
        // --- D-04: before_model fires exactly once on this path (no loop,
        // no tool dispatch); after_model/around_tool are never invoked
        // here (see the `execute_stream` rustdoc). An assembly built from
        // exactly the no-history inputs renders byte-identically to the
        // old hardcoded `format!("{}\n\nUser: {}\n", ..)` (D-02).
        let run_id = uuid::Uuid::new_v4();
        let assembly =
            PromptAssembly::new(paladin.node.system_prompt.clone(), input, "", vec![], None);
        let mut middleware_cx = ModelCallContext::new(run_id, paladin, assembly);
        let before_outcome = run_before(&self.middleware, &mut middleware_cx).await?;

        let prompt = match before_outcome {
            BeforeOutcome::Continue { .. } => middleware_cx.assembly.render(),
            BeforeOutcome::Finish { result, .. } => {
                // No model call at all: emit the finished output as the
                // sole, final chunk.
                let (tx, rx) = mpsc::channel::<Result<PaladinStreamChunk, PaladinError>>(1);
                let _ = tx
                    .send(Ok(PaladinStreamChunk {
                        text: result.output,
                        is_final: true,
                        metadata: None,
                    }))
                    .await;
                return Ok(rx);
            }
        };

        let prompt_data = PromptData {
            prompt_type: PromptType::User(UserPrompt {
                query: prompt,
                context: None,
            }),
            content_attachments: vec![],
            parameters: PromptParameters {
                max_tokens: None,
                temperature: Some(paladin.node.temperature),
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                stop_sequences: if paladin.node.stop_words.is_empty() {
                    None
                } else {
                    Some(paladin.node.stop_words.clone())
                },
            },
            context: None,
            expected_output: None,
            tags: None,
            category: None,
            author: None,
            metadata: BTreeMap::new(),
        };

        let request = LlmRequest::new(
            paladin.node.model.clone(),
            PromptItem {
                node: Node::new(prompt_data, Some("stream".to_string())),
            },
        )
        .with_stream(true);

        // Open the provider stream eagerly so an unsupported provider errors here
        // (before the caller starts an SSE response).
        let provider_stream = self
            .llm_port
            .generate_stream(request)
            .await
            .map_err(|e| to_paladin_error(&e))?;

        let (tx, rx) = mpsc::channel::<Result<PaladinStreamChunk, PaladinError>>(64);

        tokio::spawn(async move {
            use futures::StreamExt;
            let mut stream = Box::into_pin(provider_stream);
            while let Some(item) = stream.next().await {
                match item {
                    Ok(resp) => {
                        let is_final = resp.finish_reason.is_some();
                        let chunk = PaladinStreamChunk {
                            text: resp.delta,
                            is_final,
                            metadata: None,
                        };
                        // --- FT-FR-09, D-19: a chunk arrived -- progress.
                        // Beaten BEFORE the send so a slow consumer never
                        // delays the report.
                        if let Some(heartbeat) = &heartbeat {
                            heartbeat.beat();
                        }
                        // A send error means the receiver was dropped — stop producing.
                        if tx.send(Ok(chunk)).await.is_err() {
                            return;
                        }
                        if is_final {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(to_paladin_error(&e))).await;
                        return;
                    }
                }
            }
            // Provider stream ended without an explicit final marker — emit one.
            let _ = tx
                .send(Ok(PaladinStreamChunk {
                    text: String::new(),
                    is_final: true,
                    metadata: None,
                }))
                .await;
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::sanctum::MemoryExtractionStrategy;
    use crate::core::base::entity::node::Node;
    #[cfg(feature = "vision")]
    use crate::core::platform::container::vision::{ImageDetail, VisionContent};
    use crate::core::platform::container::{
        paladin::PaladinData,
        sanctum::{Memory, MemoryType, SanctumEntry},
    };
    use async_trait::async_trait;
    use paladin_ports::output::llm_port::{
        LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
    };
    use paladin_ports::output::sanctum_port::SanctumSearchResult;
    use uuid::Uuid;

    // Mock LlmPort for testing
    struct MockLlmPort;

    #[async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            unimplemented!()
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "Mock"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn create_test_paladin() -> Paladin {
        let data = PaladinData {
            system_prompt: "You are a helpful assistant".to_string(),
            ..Default::default()
        };

        Node::new(data, Some("TestPaladin".to_string()))
    }

    // --- Plan 26-13, D-21: RunScope / execute_scoped / with_vault --------

    fn test_circuit_breaker() -> Arc<CircuitBreaker> {
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)))
    }

    /// An `LlmPort` that counts `generate` calls and always answers a
    /// plain, non-tool-calling completion.
    #[derive(Default)]
    struct CountingLlmPort {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl CountingLlmPort {
        fn call_count(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LlmPort for CountingLlmPort {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
                content: "the answer".to_string(),
                finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                usage: crate::core::platform::container::token_usage::TokenUsage::new(1, 1),
                created_at: chrono::Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
            })
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            unimplemented!("not exercised")
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "Counting"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    /// Test 2: a run through `execute` produces a `PaladinResult` equal to
    /// the same run through `execute_scoped(.., &RunScope::default())`, and
    /// both make the same number of port calls.
    #[tokio::test]
    async fn execute_is_execute_scoped_with_default_scope() {
        let llm = Arc::new(CountingLlmPort::default());
        let service = PaladinExecutionService::new(llm.clone(), test_circuit_breaker(), None, None);
        let paladin = create_test_paladin();

        let direct = service
            .execute(&paladin, "hello")
            .await
            .expect("execute succeeds");
        let calls_after_direct = llm.call_count();

        let scoped = service
            .execute_scoped(&paladin, "hello", None, &RunScope::default())
            .await
            .expect("execute_scoped succeeds");
        let calls_after_scoped = llm.call_count() - calls_after_direct;

        assert_eq!(scoped.output, direct.output);
        assert_eq!(scoped.loop_count, direct.loop_count);
        assert_eq!(scoped.stop_reason, direct.stop_reason);
        assert_eq!(
            calls_after_scoped, calls_after_direct,
            "execute and execute_scoped(default) must make the same number of port calls"
        );
    }

    /// Test 3: with `with_vault(vault, None)` and a scope carrying
    /// `["user","alice"]`, the run's confined vault has that grant.
    #[tokio::test]
    async fn scope_namespace_becomes_the_run_grant() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, None);

        let alice = Namespace::parse("user/alice").unwrap();
        let scope = RunScope::default().with_vault_namespace(alice.clone());

        let confined = service
            .confined_vault(&scope)
            .expect("a scope namespace must produce a grant");
        assert_eq!(confined.granted(), &alice);
    }

    /// Test 4: with `with_vault(vault, Some(["team"]))` and
    /// `RunScope::default()`, the run's grant is `["team"]`.
    #[tokio::test]
    async fn service_default_namespace_applies_when_the_scope_has_none() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let team = Namespace::parse("team").unwrap();
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, Some(team.clone()));

        let confined = service
            .confined_vault(&RunScope::default())
            .expect("the service default namespace must apply when the scope has none");
        assert_eq!(confined.granted(), &team);
    }

    /// A scope namespace takes precedence over the service's own default
    /// (D-21's resolution order: scope first, service default second).
    #[tokio::test]
    async fn scope_namespace_overrides_the_service_default() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let team = Namespace::parse("team").unwrap();
        let alice = Namespace::parse("user/alice").unwrap();
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, Some(team));

        let scope = RunScope::default().with_vault_namespace(alice.clone());
        let confined = service
            .confined_vault(&scope)
            .expect("a scope namespace must produce a grant");
        assert_eq!(confined.granted(), &alice);
    }

    /// Test 5: with `with_vault(vault, None)` and `RunScope::default()`,
    /// the run has NO grant: `confined_vault` returns `None` -- never a
    /// handle silently granted the root namespace. This is the explicit
    /// denial: with no `ConfinedVault` constructed at all, no vault call
    /// can ever reach the store for this run.
    #[tokio::test]
    async fn no_grant_means_denied_not_root() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, None);

        let confined = service.confined_vault(&RunScope::default());
        assert!(
            confined.is_none(),
            "a run with no grant must have NO ConfinedVault at all -- never one \
             silently granted the root namespace"
        );
    }

    /// A service with no Vault store confines nothing, regardless of what
    /// the scope carries.
    #[tokio::test]
    async fn no_vault_store_means_no_confined_vault_regardless_of_scope() {
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        );
        let alice = Namespace::parse("user/alice").unwrap();
        let scope = RunScope::default().with_vault_namespace(alice);
        assert!(service.confined_vault(&scope).is_none());
    }

    // --- Plan 26-16, D-22: enable_vault_tools / effective_arsenal ---------

    /// Test 6 (service level): a run with no grant sees no vault tools at
    /// all, even with `enable_vault_tools()` called -- the flag has no
    /// effect for a run that resolves to no grant (D-21).
    #[tokio::test]
    async fn vault_tools_are_not_listed_without_a_grant() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, None)
        .enable_vault_tools();

        let confined = service.confined_vault(&RunScope::default());
        assert!(
            confined.is_none(),
            "no service default and no scope namespace must resolve to no grant"
        );

        let effective = service.effective_arsenal(confined.as_ref());
        assert!(
            effective.is_none(),
            "with no grant, the vault tools must not be listed even though enable_vault_tools() was called"
        );
    }

    /// Test 7: without `enable_vault_tools()`, the tools are absent even
    /// when a vault and a grant are both configured.
    #[tokio::test]
    async fn vault_tools_are_opt_in() {
        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let alice = Namespace::parse("user/alice").unwrap();
        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            None,
        )
        .with_vault(vault, Some(alice));
        // Deliberately NOT calling enable_vault_tools().

        let confined = service
            .confined_vault(&RunScope::default())
            .expect("the service default namespace must resolve to a grant");

        let effective = service.effective_arsenal(Some(&confined));
        assert!(
            effective.is_none(),
            "vault tools must be absent without enable_vault_tools(), even with a vault \
             and a grant both configured"
        );
    }

    /// Test 8: with an existing arsenal already configured, enabling vault
    /// tools produces a composite listing both sets, and the pre-existing
    /// tool still invokes correctly through the composite.
    #[tokio::test]
    async fn enable_vault_tools_composes_with_an_existing_arsenal() {
        use crate::application::services::arsenal::in_process_arsenal::InProcessArsenal;
        use crate::core::platform::container::arsenal::Armament;

        let vault: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());
        let alice = Namespace::parse("user/alice").unwrap();

        let existing: Arc<dyn ArsenalPort> = Arc::new(InProcessArsenal::new().with_tool(
            Armament {
                name: "web_search".to_string(),
                description: "Searches the web".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_params: vec![],
            },
            |_args| async move { Ok(serde_json::json!({"results": []})) },
        ));

        let service = PaladinExecutionService::new(
            Arc::new(CountingLlmPort::default()),
            test_circuit_breaker(),
            None,
            Some(existing),
        )
        .with_vault(vault, Some(alice))
        .enable_vault_tools();

        let confined = service
            .confined_vault(&RunScope::default())
            .expect("the service default namespace must resolve to a grant");
        let effective = service
            .effective_arsenal(Some(&confined))
            .expect("vault tools enabled + a grant must produce a composite arsenal");

        let names: std::collections::HashSet<String> = effective
            .list_armaments()
            .await
            .into_iter()
            .map(|a| a.name)
            .collect();
        assert!(
            names.contains("web_search"),
            "the pre-existing tool must still be listed"
        );
        assert!(
            names.contains("vault_get"),
            "vault_get must be listed once enabled"
        );
        assert!(
            names.contains("vault_put"),
            "vault_put must be listed once enabled"
        );

        let result = effective
            .invoke(ArmamentCall::new("web_search", HashMap::new()))
            .await
            .expect("the pre-existing tool must still invoke correctly through the composite");
        assert_eq!(result.output, Some(serde_json::json!({"results": []})));
    }

    // --- Plan 26-15, D-16: effective_history (latest-summary-wins) -------

    fn history_entry(content: &str) -> GarrisonEntry {
        GarrisonEntry::new(ConversationRole::User, content.to_string())
    }

    /// Test 1: a recalled window containing raw entries, a summary, then
    /// more raw entries yields an effective history of [that summary] +
    /// [the raw entries newer than it]; the older raw entries are skipped,
    /// and nothing is deleted from the source `Vec` (this function is a
    /// pure transform, not a store mutation).
    #[test]
    fn effective_history_is_the_newest_summary_plus_newer_raw() {
        let history = vec![
            history_entry("older raw 1"),
            history_entry("older raw 2"),
            GarrisonEntry::summary("condensed older turns".to_string()),
            history_entry("newer raw 1"),
            history_entry("newer raw 2"),
        ];

        let effective = effective_history(history);

        let contents: Vec<&str> = effective.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(
            contents,
            vec!["condensed older turns", "newer raw 1", "newer raw 2"]
        );
        assert!(effective[0].is_summary);
        assert!(!effective[1].is_summary);
        assert!(!effective[2].is_summary);
    }

    /// With no summary anywhere in the window, the whole window is the
    /// effective history, unchanged.
    #[test]
    fn effective_history_with_no_summary_is_the_whole_window() {
        let history = vec![history_entry("a"), history_entry("b"), history_entry("c")];
        let effective = effective_history(history.clone());
        let original: Vec<&str> = history.iter().map(|e| e.content.as_str()).collect();
        let after: Vec<&str> = effective.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(original, after);
    }

    /// An OLDER stale summary in the window (one `SummarizationMiddleware`
    /// has already compounded past) is skipped entirely -- only the NEWEST
    /// summary, plus raw entries newer than it, survives. `GarrisonPort` has
    /// no delete-by-id, so this stale-summary shape is expected, not a bug.
    #[test]
    fn effective_history_skips_a_stale_older_summary() {
        let history = vec![
            GarrisonEntry::summary("stale, superseded summary".to_string()),
            history_entry("raw between the two summaries"),
            GarrisonEntry::summary("the newest summary".to_string()),
            history_entry("newest raw"),
        ];

        let effective = effective_history(history);

        let contents: Vec<&str> = effective.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, vec!["the newest summary", "newest raw"]);
    }

    /// EDGE(RT-03/empty): an empty history is a no-op.
    #[test]
    fn effective_history_of_an_empty_window_is_empty() {
        assert!(effective_history(Vec::new()).is_empty());
    }

    /// A bare `ModelCallContext` with no `llm_override`/`retry_policy` set
    /// -- the D-11 "no resilience middleware" shape, for tests that call
    /// `execute_with_retry_and_temperature` directly.
    fn bare_cx(paladin: &Paladin) -> ModelCallContext<'_> {
        ModelCallContext::new(
            Uuid::new_v4(),
            paladin,
            PromptAssembly::new("system", "input", "", vec![], None),
        )
    }

    // --- Plan 25-09, D-19: the three beat points ------------------------

    /// An `LlmPort` whose `generate` always requests the `lookup` tool (so
    /// the Arsenal branch of the reasoning loop runs) and whose
    /// `generate_stream` yields `chunks` deltas then a final marker.
    struct ToolCallingLlmPort {
        chunks: usize,
    }

    #[async_trait]
    impl LlmPort for ToolCallingLlmPort {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
                content: "calling lookup".to_string(),
                finish_reason: paladin_ports::output::llm_port::FinishReason::FunctionCall,
                usage: crate::core::platform::container::token_usage::TokenUsage::new(1, 1),
                created_at: chrono::Utc::now(),
                metadata: HashMap::new(),
                function_call: Some(FunctionCall {
                    name: "lookup".to_string(),
                    arguments: r#"{"q":"x"}"#.to_string(),
                }),
            })
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            let mut items: Vec<Result<StreamingResponse, LlmError>> = (0..self.chunks)
                .map(|i| {
                    Ok(StreamingResponse {
                        id: Uuid::new_v4(),
                        delta: format!("c{i}"),
                        finish_reason: None,
                    })
                })
                .collect();
            items.push(Ok(StreamingResponse {
                id: Uuid::new_v4(),
                delta: String::new(),
                finish_reason: Some(paladin_ports::output::llm_port::FinishReason::Stop),
            }));
            Ok(Box::new(futures::stream::iter(items)))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "ToolCalling"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    /// An `ArsenalPort` that counts `invoke` calls and always succeeds.
    #[derive(Default)]
    struct CountingArsenal {
        invocations: std::sync::atomic::AtomicUsize,
    }

    #[async_trait]
    impl ArsenalPort for CountingArsenal {
        async fn list_armaments(&self) -> Vec<crate::core::platform::container::arsenal::Armament> {
            Vec::new()
        }

        async fn invoke(
            &self,
            call: ArmamentCall,
        ) -> Result<crate::core::platform::container::arsenal::ArmamentResult, ArsenalError>
        {
            self.invocations
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(
                crate::core::platform::container::arsenal::ArmamentResult::success(
                    call.call_id,
                    serde_json::json!("found"),
                    0,
                ),
            )
        }

        fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
            Ok(())
        }
    }

    /// D-19 / FT-FR-09: `execute_observed` beats on every completed LLM
    /// call and every Armament invocation, and `execute_stream_observed`
    /// beats once per streamed chunk -- a recording `HeartbeatHandle`
    /// observes at least one beat per event, and the buffered result is
    /// identical to `execute`'s for identical inputs.
    #[tokio::test]
    async fn paladin_execution_service_beats_on_llm_completion_stream_chunk_and_armament() {
        use paladin_core::platform::container::heartbeat::HeartbeatHandle;
        use paladin_core::platform::container::paladin::MaxLoops;

        let arsenal = Arc::new(CountingArsenal::default());
        let llm: Arc<dyn LlmPort> = Arc::new(ToolCallingLlmPort { chunks: 3 });
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(
            llm,
            circuit_breaker,
            None,
            Some(arsenal.clone() as Arc<dyn ArsenalPort>),
        );
        let mut paladin = create_test_paladin();
        paladin.node.max_loops = MaxLoops::Fixed(2);

        // --- buffered path: 2 loops x (1 LLM completion + 1 Armament call)
        let heartbeat = HeartbeatHandle::new();
        let observed = service
            .execute_observed(&paladin, "hello", &heartbeat)
            .await
            .expect("observed execution succeeds");
        assert_eq!(
            arsenal
                .invocations
                .load(std::sync::atomic::Ordering::SeqCst),
            2,
            "one Armament invocation per loop"
        );
        assert!(
            heartbeat.beats() >= 4,
            "at least one beat per LLM completion (2) and per Armament invocation (2), got {}",
            heartbeat.beats()
        );

        // --- identical result to the unobserved path
        let direct = service
            .execute(&paladin, "hello")
            .await
            .expect("direct execution succeeds");
        assert_eq!(observed.output, direct.output);
        assert_eq!(observed.loop_count, direct.loop_count);
        assert_eq!(observed.stop_reason, direct.stop_reason);

        // --- streaming path: one beat per chunk (3 content chunks + final)
        let stream_heartbeat = HeartbeatHandle::new();
        let mut stream = service
            .execute_stream_observed(&paladin, "hello", &stream_heartbeat)
            .await
            .expect("stream opens");
        let mut chunks = 0usize;
        while let Some(item) = stream.recv().await {
            let chunk = item.expect("chunk is Ok");
            chunks += 1;
            if chunk.is_final {
                break;
            }
        }
        assert_eq!(chunks, 4, "3 deltas plus the final marker");
        assert!(
            stream_heartbeat.beats() >= 3,
            "at least one beat per streamed content chunk, got {}",
            stream_heartbeat.beats()
        );
    }

    /// Where a [`FailingLlmPort`] surfaces its `LlmError`.
    #[derive(Clone, Copy)]
    enum FailAt {
        /// `generate` and `generate_stream` both return `Err` up front.
        Open,
        /// `generate_stream` opens, then the stream's first item is `Err`.
        MidStream,
    }

    /// An `LlmPort` that fails with a caller-chosen real `LlmError`, so the
    /// sites migrated by plan 25-06 (D-02) can be observed converting it
    /// through `llm_failure::to_paladin_error`.
    struct FailingLlmPort {
        make: fn() -> LlmError,
        fail_at: FailAt,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl FailingLlmPort {
        fn new(make: fn() -> LlmError, fail_at: FailAt) -> Arc<Self> {
            Arc::new(Self {
                make,
                fail_at,
                calls: std::sync::atomic::AtomicUsize::new(0),
            })
        }

        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LlmPort for FailingLlmPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err((self.make)())
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
            LlmError,
        > {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match self.fail_at {
                FailAt::Open => Err((self.make)()),
                FailAt::MidStream => Ok(Box::new(futures::stream::iter(vec![Err((self.make)())]))),
            }
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "failing"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn provider_503() -> LlmError {
        LlmError::ProviderError {
            provider: "openai".to_string(),
            status: 503,
            message: "upstream unavailable".to_string(),
        }
    }

    fn auth_failure() -> LlmError {
        LlmError::AuthenticationError("invalid API key".to_string())
    }

    fn failing_service(
        make: fn() -> LlmError,
        fail_at: FailAt,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> (Arc<FailingLlmPort>, PaladinExecutionService) {
        let port = FailingLlmPort::new(make, fail_at);
        let llm: Arc<dyn LlmPort> = port.clone();
        (
            port,
            PaladinExecutionService::new(llm, circuit_breaker, None, None),
        )
    }

    fn default_breaker() -> Arc<CircuitBreaker> {
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)))
    }

    /// Plan 25-06 Test 1 (D-02): a provider 503 reaching `execute_stream`'s
    /// open-stream site surfaces as the structured `LlmFailure`, carrying the
    /// typed transience/status/provider -- not the stringly `LlmError(String)`.
    #[tokio::test]
    async fn paladin_execution_service_surfaces_structured_llm_failure() {
        use paladin_core::platform::container::transience::Transience;

        let (_, service) = failing_service(provider_503, FailAt::Open, default_breaker());
        let paladin = create_test_paladin();

        let err = match service.execute_stream(&paladin, "hello").await {
            Err(err) => err,
            Ok(_) => panic!("a provider 503 must fail the stream open"),
        };

        match err {
            PaladinError::LlmFailure {
                transience,
                status,
                provider,
                ..
            } => {
                assert_eq!(transience, Transience::Transient);
                assert_eq!(status, Some(503));
                assert_eq!(provider.as_deref(), Some("openai"));
            }
            other => panic!("expected PaladinError::LlmFailure, got {other:?}"),
        }
    }

    /// Plan 25-06 Test 2 (D-02): a permanent provider failure surfaces as
    /// `Permanent`, both at stream open and from inside a live stream (the
    /// channel-send site).
    #[tokio::test]
    async fn permanent_provider_failure_surfaces_as_permanent() {
        use paladin_core::platform::container::transience::Transience;

        let paladin = create_test_paladin();

        // Stream-open site.
        let (_, service) = failing_service(auth_failure, FailAt::Open, default_breaker());
        match service.execute_stream(&paladin, "hello").await {
            Err(PaladinError::LlmFailure {
                transience,
                status,
                provider,
                ..
            }) => {
                assert_eq!(transience, Transience::Permanent);
                assert_eq!(status, None);
                assert_eq!(provider, None);
            }
            other => panic!("expected Err(LlmFailure), got {other:?}"),
        }

        // Mid-stream site: the error rides the mpsc channel.
        let (_, service) = failing_service(auth_failure, FailAt::MidStream, default_breaker());
        let mut stream = service
            .execute_stream(&paladin, "hello")
            .await
            .expect("stream opens before the first item fails");
        match stream.recv().await {
            Some(Err(PaladinError::LlmFailure { transience, .. })) => {
                assert_eq!(transience, Transience::Permanent);
            }
            other => panic!("expected Some(Err(LlmFailure)), got {other:?}"),
        }
        assert!(
            stream.recv().await.is_none(),
            "the producer stops after forwarding the failure"
        );
    }

    /// Plan 25-06 (T-25-25): the two buffered retry-loop sites feed the
    /// converted failure to the circuit breaker, whose `is_retryable()`
    /// accounting must not drift -- `LlmFailure` counts as a failure exactly
    /// as the legacy `LlmError(_)` did. With a threshold of one failure, the
    /// first attempt trips the breaker and the second attempt fails fast with
    /// `CircuitBreakerOpen`, so the loop's control flow is observably unchanged
    /// (one provider call, not two).
    #[tokio::test]
    async fn buffered_retry_sites_trip_the_circuit_breaker_like_the_legacy_variant() {
        use crate::core::platform::container::paladin::MaxLoops;

        let mut paladin = create_test_paladin();
        paladin.node.max_loops = MaxLoops::Fixed(2);

        // Site: execute_with_retry_and_temperature.
        let trips_after_one = Arc::new(CircuitBreaker::new(1, 1, Duration::from_secs(60)));
        let (port, service) = failing_service(provider_503, FailAt::Open, trips_after_one);
        let result = service
            .execute_with_retry_and_temperature(
                &paladin,
                "hello",
                0.5,
                Uuid::new_v4(),
                1,
                &bare_cx(&paladin),
            )
            .await;
        assert!(
            matches!(result, Err(PaladinError::CircuitBreakerOpen)),
            "expected CircuitBreakerOpen on the second attempt, got {result:?}"
        );
        assert_eq!(
            port.calls(),
            1,
            "the breaker must reject the second attempt"
        );

        // Site: execute_with_retry.
        let trips_after_one = Arc::new(CircuitBreaker::new(1, 1, Duration::from_secs(60)));
        let (port, service) = failing_service(provider_503, FailAt::Open, trips_after_one);
        let result = service
            .execute_with_retry(&paladin, "hello", Uuid::new_v4(), 1)
            .await;
        assert!(
            matches!(result, Err(PaladinError::CircuitBreakerOpen)),
            "expected CircuitBreakerOpen on the second attempt, got {result:?}"
        );
        assert_eq!(
            port.calls(),
            1,
            "the breaker must reject the second attempt"
        );
    }

    /// WR-02 (`25-REVIEW.md`) regression: a `Permanent`-classified failure
    /// (a rejected credential) must be returned immediately, on the FIRST
    /// attempt, rather than retried up to `max_attempts` with exponential
    /// backoff. Uses a generous breaker (threshold well above 1) so a
    /// `CircuitBreakerOpen` on a later attempt cannot be mistaken for the
    /// fail-fast behaviour under test -- `port.calls() == 1` is the only
    /// way this test can pass.
    #[tokio::test]
    async fn permanent_failure_is_not_retried_by_buffered_retry_sites() {
        use crate::core::platform::container::paladin::MaxLoops;

        let mut paladin = create_test_paladin();
        paladin.node.max_loops = MaxLoops::Fixed(5);

        // Site: execute_with_retry_and_temperature.
        let (port, service) = failing_service(auth_failure, FailAt::Open, default_breaker());
        let result = service
            .execute_with_retry_and_temperature(
                &paladin,
                "hello",
                0.5,
                Uuid::new_v4(),
                1,
                &bare_cx(&paladin),
            )
            .await;
        match result {
            Err(PaladinError::LlmFailure { transience, .. }) => {
                assert_eq!(transience, Transience::Permanent);
            }
            other => panic!("expected Err(LlmFailure {{ Permanent }}), got {other:?}"),
        }
        assert_eq!(
            port.calls(),
            1,
            "a Permanent failure must return on the first attempt, never retried"
        );

        // Site: execute_with_retry.
        let (port, service) = failing_service(auth_failure, FailAt::Open, default_breaker());
        let result = service
            .execute_with_retry(&paladin, "hello", Uuid::new_v4(), 1)
            .await;
        match result {
            Err(PaladinError::LlmFailure { transience, .. }) => {
                assert_eq!(transience, Transience::Permanent);
            }
            other => panic!("expected Err(LlmFailure {{ Permanent }}), got {other:?}"),
        }
        assert_eq!(
            port.calls(),
            1,
            "a Permanent failure must return on the first attempt, never retried"
        );
    }

    /// WR-02 (`25-REVIEW.md`) regression, the complementary case: a
    /// `Transient` failure (unchanged) and an `Unknown`-classified failure
    /// (`ProcessingError`, which has no typed field to tell it apart from a
    /// permanent one) must both still be retried up to `max_attempts`,
    /// exactly as before this fix -- only `Permanent` short-circuits.
    #[tokio::test(start_paused = true)]
    async fn transient_and_unknown_failures_still_retry_until_max_attempts() {
        use crate::core::platform::container::paladin::MaxLoops;

        fn processing_error() -> LlmError {
            LlmError::ProcessingError("transient decode noise".to_string())
        }

        let mut paladin = create_test_paladin();
        paladin.node.max_loops = MaxLoops::Fixed(2);

        for make in [provider_503, processing_error] {
            let (port, service) = failing_service(make, FailAt::Open, default_breaker());
            let result = service
                .execute_with_retry_and_temperature(
                    &paladin,
                    "hello",
                    0.5,
                    Uuid::new_v4(),
                    1,
                    &bare_cx(&paladin),
                )
                .await;
            assert!(
                matches!(result, Err(PaladinError::MaxRetriesExceeded(2))),
                "expected MaxRetriesExceeded(2), got {result:?}"
            );
            assert_eq!(
                port.calls(),
                2,
                "both attempts must be spent before surfacing MaxRetriesExceeded"
            );

            let (port, service) = failing_service(make, FailAt::Open, default_breaker());
            let result = service
                .execute_with_retry(&paladin, "hello", Uuid::new_v4(), 1)
                .await;
            assert!(
                matches!(result, Err(PaladinError::MaxRetriesExceeded(2))),
                "expected MaxRetriesExceeded(2), got {result:?}"
            );
            assert_eq!(
                port.calls(),
                2,
                "both attempts must be spent before surfacing MaxRetriesExceeded"
            );
        }
    }

    /// Plan 25-06 Test 5 (X-03, T-25-27): for a fixed `LlmError`, what each
    /// migrated site in this file returns renders exactly what the legacy
    /// `PaladinError::LlmError(e.to_string())` rendered -- `LLM error: {e}`.
    ///
    /// The two buffered retry-loop sites never return the converted error to
    /// a caller (they log it and end in `MaxRetriesExceeded` or
    /// `CircuitBreakerOpen`, unchanged); their rendering is pinned by the same
    /// helper's own every-variant test in `paladin_battalion::llm_failure`.
    #[tokio::test]
    async fn rendered_error_text_at_every_migrated_site_is_unchanged() {
        let expected = format!("LLM error: {}", provider_503());
        let paladin = create_test_paladin();

        // Stream-open site.
        let (_, service) = failing_service(provider_503, FailAt::Open, default_breaker());
        let err = service
            .execute_stream(&paladin, "hello")
            .await
            .expect_err("stream open fails");
        assert_eq!(err.to_string(), expected);

        // Mid-stream channel-send site.
        let (_, service) = failing_service(provider_503, FailAt::MidStream, default_breaker());
        let mut stream = service
            .execute_stream(&paladin, "hello")
            .await
            .expect("stream opens");
        let err = match stream.recv().await {
            Some(Err(err)) => err,
            other => panic!("expected Some(Err(_)), got {other:?}"),
        };
        assert_eq!(err.to_string(), expected);
    }

    #[tokio::test]
    async fn execute_stream_assembles_chunks_into_full_output() {
        use paladin_llm::mock::MockLlmAdapter;

        // MockLlmAdapter::generate_stream emits the content delta then a final marker.
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm, circuit_breaker, None, None);
        let paladin = create_test_paladin();

        let mut stream = service
            .execute_stream(&paladin, "hello")
            .await
            .expect("stream starts");

        let mut assembled = String::new();
        let mut saw_final = false;
        while let Some(item) = stream.recv().await {
            let chunk = item.expect("chunk should be Ok");
            if chunk.is_final {
                saw_final = true;
                break;
            }
            assembled.push_str(&chunk.text);
        }

        assert!(saw_final, "stream must end with a final chunk");
        assert_eq!(assembled, "Mock LLM response");
    }

    fn create_mock_search_result(content: &str, score: f32) -> SanctumSearchResult {
        let memory = Memory {
            id: Uuid::new_v4(),
            paladin_id: "test".to_string(),
            content: content.to_string(),
            memory_type: MemoryType::Episodic,
            importance: 0.5,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };

        let entry = SanctumEntry::new(memory, vec![0.1; 384]).expect("Failed to create test entry");

        SanctumSearchResult { entry, score }
    }

    #[tokio::test]
    async fn test_format_retrieved_context() {
        // Arrange
        let results = vec![
            create_mock_search_result("First memory", 0.95),
            create_mock_search_result("Second memory", 0.85),
        ];

        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Act
        let formatted = service.format_retrieved_context(&results);

        // Assert
        assert!(formatted.contains("1. [Score: 0.95] First memory"));
        assert!(formatted.contains("2. [Score: 0.85] Second memory"));
    }

    #[tokio::test]
    async fn test_format_retrieved_context_empty() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Act
        let formatted = service.format_retrieved_context(&[]);

        // Assert
        assert!(formatted.is_empty());
    }

    #[tokio::test]
    async fn test_check_sanctum_configured() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        // Act & Assert: Without RAG service
        let service_without = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
        assert!(!service_without.check_sanctum_configured());
    }

    #[tokio::test]
    async fn test_should_extract_memories() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        // Act & Assert: Without extraction service
        let service_without = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
        assert!(!service_without.should_extract_memories(MemoryExtractionStrategy::OnCompletion));
    }

    #[tokio::test]
    async fn test_rag_context_injection() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
        let paladin = create_test_paladin();

        // Act: Build prompt with RAG context
        let retrieved_context = "1. [Score: 0.95] Previous conversation about Rust\n";
        let prompt = service.build_prompt_with_history_and_rag(
            &paladin,
            "What is Rust?",
            "",
            &[],
            Some(retrieved_context),
        );

        // Assert: Context should be injected
        assert!(prompt.contains("## Relevant Context from Memory"));
        assert!(prompt.contains("Previous conversation about Rust"));
    }

    #[tokio::test]
    async fn test_rag_context_injection_empty() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
        let paladin = create_test_paladin();

        // Act: Build prompt without RAG context
        let prompt =
            service.build_prompt_with_history_and_rag(&paladin, "What is Rust?", "", &[], None);

        // Assert: No RAG section should be present
        assert!(!prompt.contains("## Relevant Context from Memory"));
    }

    // Mock OrchestratorPort for wiring tests
    struct MockOrchestratorPort;

    #[async_trait]
    impl OrchestratorPort for MockOrchestratorPort {
        async fn schedule_job(
            &self,
            _request: paladin_ports::output::orchestrator_port::ScheduleJobRequest,
        ) -> Result<Uuid, paladin_ports::output::orchestrator_port::OrchestratorBridgeError>
        {
            Ok(Uuid::new_v4())
        }

        async fn queue_item(
            &self,
            _request: paladin_ports::output::orchestrator_port::QueueItemRequest,
        ) -> Result<Uuid, paladin_ports::output::orchestrator_port::OrchestratorBridgeError>
        {
            Ok(Uuid::new_v4())
        }

        async fn fire_event(
            &self,
            _request: paladin_ports::output::orchestrator_port::FireEventRequest,
        ) -> Result<
            paladin_ports::output::orchestrator_port::EventDispatchResult,
            paladin_ports::output::orchestrator_port::OrchestratorBridgeError,
        > {
            Ok(paladin_ports::output::orchestrator_port::EventDispatchResult::default())
        }

        async fn send_notification(
            &self,
            _request: paladin_ports::output::orchestrator_port::SendNotificationRequest,
        ) -> Result<Uuid, paladin_ports::output::orchestrator_port::OrchestratorBridgeError>
        {
            Ok(Uuid::new_v4())
        }
    }

    #[tokio::test]
    async fn test_orchestrator_port_wiring() {
        // Arrange
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        // Default: no orchestrator port attached
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);
        assert!(service.orchestrator_port().is_none());

        // After wiring: orchestrator port is attached
        let orchestrator_port: Arc<dyn OrchestratorPort> = Arc::new(MockOrchestratorPort);
        let service = service.with_orchestrator_port(orchestrator_port);
        assert!(service.orchestrator_port().is_some());
    }

    #[test]
    fn test_build_prompt_basic() {
        // Basic test without async context
        let prompt = "Test system prompt";
        let input = "Test input";
        let _accumulated = "";

        let expected = format!("{}\n\nUser: {}\n", prompt, input);
        assert!(expected.contains(prompt));
        assert!(expected.contains(input));
    }

    #[test]
    fn test_check_stop_words_case_insensitive() {
        // Test stop word detection logic
        let output = "This response contains STOP keyword";
        let stop_words = ["stop".to_string()];

        let output_lower = output.to_lowercase();
        let found = stop_words
            .iter()
            .any(|word| output_lower.contains(&word.to_lowercase()));

        assert!(found, "Should detect stop word case-insensitively");
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_vision_capability_check() {
        // Test that vision capability is checked
        // This is a placeholder until we implement the run_with_vision method
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let _service = PaladinExecutionService::new(llm_port.clone(), circuit_breaker, None, None);

        // Create a paladin with vision_enabled
        let data = PaladinData {
            vision_enabled: true,
            ..Default::default()
        };
        let paladin = Node::new(data, Some("VisionPaladin".to_string()));

        // Verify that the MockLlmPort doesn't support vision
        let caps = llm_port.get_capabilities();
        assert!(
            !caps.supports_vision,
            "MockLlmPort should not support vision"
        );

        // Verify paladin has vision_enabled
        assert!(
            paladin.node.vision_enabled,
            "Paladin should have vision enabled"
        );
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_execute_with_vision_not_enabled() {
        // Test that execute_with_vision fails when vision is not enabled
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Create a paladin WITHOUT vision_enabled
        let data = PaladinData {
            vision_enabled: false,
            ..Default::default()
        };
        let paladin = Node::new(data, Some("NormalPaladin".to_string()));

        // Try to execute with vision - should fail
        let images = vec![VisionContent::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
            detail: ImageDetail::Auto,
        }];

        let result = service
            .execute_with_vision(&paladin, "What's in this image?", images)
            .await;

        assert!(result.is_err(), "Should fail when vision not enabled");
        match result {
            Err(PaladinError::ConfigurationError(msg)) => {
                assert!(
                    msg.contains("vision_enabled=true"),
                    "Error should mention vision_enabled"
                );
            }
            _ => panic!("Should return ConfigurationError"),
        }
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_execute_with_vision_unsupported_provider() {
        // Test that execute_with_vision fails when LLM provider doesn't support vision
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Create a paladin with vision_enabled but MockLlmPort doesn't support vision
        let data = PaladinData {
            vision_enabled: true,
            ..Default::default()
        };
        let paladin = Node::new(data, Some("VisionPaladin".to_string()));

        let images = vec![VisionContent::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
            detail: ImageDetail::Auto,
        }];

        let result = service
            .execute_with_vision(&paladin, "What's in this image?", images)
            .await;

        assert!(
            result.is_err(),
            "Should fail when provider doesn't support vision"
        );
        match result {
            Err(PaladinError::ConfigurationError(msg)) => {
                assert!(
                    msg.contains("does not support vision"),
                    "Error should mention lack of vision support"
                );
            }
            _ => panic!("Should return ConfigurationError"),
        }
    }

    // Mock VisionPort for testing
    #[cfg(feature = "vision")]
    struct MockVisionPort {
        provider: String,
        should_fail: bool,
        response_content: String,
    }

    #[cfg(feature = "vision")]
    impl MockVisionPort {
        fn new(provider: &str) -> Self {
            Self {
                provider: provider.to_string(),
                should_fail: false,
                response_content: "Mock vision analysis result".to_string(),
            }
        }

        // Test mock builder; retained for failure-path coverage.
        #[allow(dead_code)]
        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_response(mut self, content: String) -> Self {
            self.response_content = content;
            self
        }
    }

    #[cfg(feature = "vision")]
    #[async_trait]
    impl paladin_ports::output::vision_port::VisionPort for MockVisionPort {
        async fn analyze_image(
            &self,
            _prompt: &str,
            _images: Vec<VisionContent>,
            _model: &str,
            _max_tokens: Option<u32>,
        ) -> Result<
            paladin_ports::output::vision_port::VisionResult,
            crate::core::platform::container::vision::VisionError,
        > {
            if self.should_fail {
                return Err(
                    crate::core::platform::container::vision::VisionError::InvalidRequest(
                        "Mock failure".to_string(),
                    ),
                );
            }

            Ok(paladin_ports::output::vision_port::VisionResult {
                content: self.response_content.clone(),
                model: "mock-model".to_string(),
                token_usage: paladin_ports::output::vision_port::VisionTokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 50,
                    total_tokens: 150,
                },
                metadata: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now(),
            })
        }

        fn is_vision_model(&self, _model: &str) -> bool {
            true
        }

        fn provider_name(&self) -> &str {
            &self.provider
        }
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_extract_provider_from_openai_model() {
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Test OpenAI models
        assert_eq!(
            service.extract_provider_from_model("gpt-4o").unwrap(),
            "openai"
        );
        assert_eq!(
            service.extract_provider_from_model("gpt-4-turbo").unwrap(),
            "openai"
        );
        assert_eq!(
            service
                .extract_provider_from_model("gpt-3.5-turbo")
                .unwrap(),
            "openai"
        );
        assert_eq!(
            service.extract_provider_from_model("o1-preview").unwrap(),
            "openai"
        );
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_extract_provider_from_anthropic_model() {
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Test Anthropic models
        assert_eq!(
            service
                .extract_provider_from_model("claude-3-opus-20240229")
                .unwrap(),
            "anthropic"
        );
        assert_eq!(
            service
                .extract_provider_from_model("claude-3-sonnet")
                .unwrap(),
            "anthropic"
        );
        assert_eq!(
            service
                .extract_provider_from_model("claude-3-5-sonnet")
                .unwrap(),
            "anthropic"
        );
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_extract_provider_from_unsupported_model() {
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Test unsupported model
        let result = service.extract_provider_from_model("llama-2-70b");
        assert!(result.is_err());
        match result {
            Err(PaladinError::ConfigurationError(msg)) => {
                assert!(msg.contains("Cannot determine provider"));
            }
            _ => panic!("Should return ConfigurationError"),
        }
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_with_vision_adapter() {
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        let mock_vision = Arc::new(MockVisionPort::new("openai"));

        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
            .with_vision_adapter("openai".to_string(), mock_vision.clone());

        // Verify adapter was registered
        assert!(service.vision_adapters.contains_key("openai"));
        assert_eq!(service.vision_adapters.len(), 1);
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_vision_execution_with_stop_word() {
        // Create mock LLM port with vision support
        struct VisionCapableMockLlmPort;

        #[async_trait]
        impl LlmPort for VisionCapableMockLlmPort {
            async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
                unimplemented!()
            }

            async fn generate_stream(
                &self,
                _request: LlmRequest,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
                LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
                Ok(true)
            }

            async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
                Ok(vec![])
            }

            fn get_provider_name(&self) -> &'static str {
                "MockWithVision"
            }

            fn get_capabilities(&self) -> ProviderCapabilities {
                ProviderCapabilities {
                    supports_streaming: false,
                    supports_function_calling: false,
                    supports_tool_calling: false,
                    supports_vision: true,
                    supports_embeddings: false,
                    max_context_tokens: Some(4096),
                    supports_system_messages: true,
                    temperature_range: None,
                }
            }
        }

        let llm_port: Arc<dyn LlmPort> = Arc::new(VisionCapableMockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        // Create mock vision adapter that returns content with stop word
        let mock_vision = Arc::new(
            MockVisionPort::new("openai").with_response("This is a STOP word test".to_string()),
        );

        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
            .with_vision_adapter("openai".to_string(), mock_vision);

        // Create paladin with vision enabled and stop word
        let data = PaladinData {
            vision_enabled: true,
            model: "gpt-4o".to_string(),
            stop_words: vec!["STOP".to_string()],
            ..Default::default()
        };
        let paladin = Node::new(data, Some("VisionPaladin".to_string()));

        let images = vec![VisionContent::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
            detail: ImageDetail::Auto,
        }];

        let result = service
            .execute_with_vision(&paladin, "What's in this image?", images)
            .await;

        // Should detect stop word and return error
        assert!(result.is_err());
        match result {
            Err(PaladinError::ExecutionError(msg)) => {
                assert!(
                    msg.contains("Stop word detected") || msg.contains("STOP"),
                    "Error should mention stop word: {}",
                    msg
                );
            }
            _ => panic!("Should return ExecutionError with stop word message"),
        }
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_vision_execution_missing_adapter() {
        // Create mock LLM port with vision support
        struct VisionCapableMockLlmPort;

        #[async_trait]
        impl LlmPort for VisionCapableMockLlmPort {
            async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
                unimplemented!()
            }

            async fn generate_stream(
                &self,
                _request: LlmRequest,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
                LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
                Ok(true)
            }

            async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
                Ok(vec![])
            }

            fn get_provider_name(&self) -> &'static str {
                "MockWithVision"
            }

            fn get_capabilities(&self) -> ProviderCapabilities {
                ProviderCapabilities {
                    supports_streaming: false,
                    supports_function_calling: false,
                    supports_tool_calling: false,
                    supports_vision: true,
                    supports_embeddings: false,
                    max_context_tokens: Some(4096),
                    supports_system_messages: true,
                    temperature_range: None,
                }
            }
        }

        let llm_port: Arc<dyn LlmPort> = Arc::new(VisionCapableMockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        // Create service WITHOUT registering vision adapter
        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None);

        // Create paladin with OpenAI model (requires openai adapter)
        let data = PaladinData {
            vision_enabled: true,
            model: "gpt-4o".to_string(),
            ..Default::default()
        };
        let paladin = Node::new(data, Some("VisionPaladin".to_string()));

        let images = vec![VisionContent::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
            detail: ImageDetail::Auto,
        }];

        let result = service
            .execute_with_vision(&paladin, "What's in this image?", images)
            .await;

        // Should fail because no vision adapter registered
        assert!(result.is_err());
        match result {
            Err(PaladinError::ExecutionError(msg)) => {
                assert!(
                    msg.contains("No vision adapter registered"),
                    "Error should mention missing adapter: {}",
                    msg
                );
            }
            _ => panic!("Should return ExecutionError about missing adapter"),
        }
    }

    #[tokio::test]
    #[cfg(feature = "vision")]
    async fn test_vision_execution_successful() {
        // Create mock LLM port with vision support
        struct VisionCapableMockLlmPort;

        #[async_trait]
        impl LlmPort for VisionCapableMockLlmPort {
            async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
                unimplemented!()
            }

            async fn generate_stream(
                &self,
                _request: LlmRequest,
            ) -> Result<
                Box<dyn futures::Stream<Item = Result<StreamingResponse, LlmError>> + Send>,
                LlmError,
            > {
                unimplemented!()
            }

            async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
                Ok(true)
            }

            async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
                Ok(vec![])
            }

            fn get_provider_name(&self) -> &'static str {
                "MockWithVision"
            }

            fn get_capabilities(&self) -> ProviderCapabilities {
                ProviderCapabilities {
                    supports_streaming: false,
                    supports_function_calling: false,
                    supports_tool_calling: false,
                    supports_vision: true,
                    supports_embeddings: false,
                    max_context_tokens: Some(4096),
                    supports_system_messages: true,
                    temperature_range: None,
                }
            }
        }

        let llm_port: Arc<dyn LlmPort> = Arc::new(VisionCapableMockLlmPort);
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));

        let mock_vision = Arc::new(MockVisionPort::new("openai"));

        let service = PaladinExecutionService::new(llm_port, circuit_breaker, None, None)
            .with_vision_adapter("openai".to_string(), mock_vision);

        // Create paladin with vision enabled
        let data = PaladinData {
            vision_enabled: true,
            model: "gpt-4o".to_string(),
            ..Default::default()
        };
        let paladin = Node::new(data, Some("VisionPaladin".to_string()));

        let images = vec![VisionContent::ImageUrl {
            url: "https://example.com/image.jpg".to_string(),
            detail: ImageDetail::Auto,
        }];

        let result = service
            .execute_with_vision(&paladin, "What's in this image?", images)
            .await;

        // Should succeed
        assert!(result.is_ok());
        let paladin_result = result.unwrap();
        assert_eq!(paladin_result.output, "Mock vision analysis result");
        assert_eq!(paladin_result.token_count, 150);
        assert_eq!(paladin_result.loop_count, 1);
        assert_eq!(paladin_result.stop_reason, StopReason::Completed);
    }
}

/// Phase 26 Plan 01 (RT-01, D-01…D-06): the `ExecutionMiddleware` chain's
/// end-to-end wiring into the reasoning loop, proven against a real
/// [`paladin_llm::mock::MockLlmAdapter`] rather than the hand-rolled
/// `LlmPort` doubles `mod tests` uses.
#[cfg(test)]
mod middleware_wiring_tests {
    use super::*;
    use crate::application::services::paladin::middleware::{
        ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
        PromptSection, SectionPlacement, ToolCallContext, ToolCallKind, ToolFlow,
    };
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, PaladinData};
    use async_trait::async_trait;
    use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
    use std::sync::Mutex;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "You are a helpful assistant".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, Some("TestPaladin".to_string()))
    }

    fn make_service(llm: Arc<MockLlmAdapter>) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
    }

    fn make_service_with_arsenal(
        llm: Arc<MockLlmAdapter>,
        arsenal: Arc<dyn ArsenalPort>,
    ) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            Some(arsenal),
        )
    }

    /// An `ArsenalPort` that records every call it receives and always
    /// succeeds.
    #[derive(Default)]
    struct RecordingArsenal {
        calls: Mutex<Vec<ArmamentCall>>,
    }

    #[async_trait]
    impl ArsenalPort for RecordingArsenal {
        async fn list_armaments(&self) -> Vec<crate::core::platform::container::arsenal::Armament> {
            Vec::new()
        }

        async fn invoke(
            &self,
            call: ArmamentCall,
        ) -> Result<crate::core::platform::container::arsenal::ArmamentResult, ArsenalError>
        {
            self.calls.lock().unwrap().push(call.clone());
            Ok(
                crate::core::platform::container::arsenal::ArmamentResult::success(
                    call.call_id,
                    serde_json::json!("ok"),
                    0,
                ),
            )
        }

        fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
            Ok(())
        }
    }

    /// Records `"{name}.before({loop_index})"` / `"{name}.after({loop_index})"`.
    struct RecordingMiddleware {
        name: &'static str,
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ExecutionMiddleware for RecordingMiddleware {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.before({})", self.name, cx.loop_index));
            Ok(MiddlewareFlow::Continue)
        }

        async fn after_model(
            &self,
            cx: &mut ModelCallContext<'_>,
            _resp: &mut LlmResponseView,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.after({})", self.name, cx.loop_index));
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    /// Records every `ToolCallContext::kind` it observes and allows the
    /// call through unchanged.
    struct RecordingAroundTool {
        kinds: Arc<Mutex<Vec<ToolCallKind>>>,
    }

    #[async_trait]
    impl ExecutionMiddleware for RecordingAroundTool {
        async fn around_tool(&self, cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
            self.kinds.lock().unwrap().push(cx.kind);
            Ok(ToolFlow::Allow)
        }

        fn name(&self) -> &str {
            "recording-around-tool"
        }
    }

    /// Denies every tool call with a fixed reason.
    struct DenyingMiddleware {
        reason: &'static str,
    }

    #[async_trait]
    impl ExecutionMiddleware for DenyingMiddleware {
        async fn around_tool(&self, _cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
            Ok(ToolFlow::Deny {
                reason: self.reason.to_string(),
            })
        }

        fn name(&self) -> &str {
            "denying"
        }
    }

    /// Rewrites every tool call to a fixed `ArmamentCall`.
    struct RewritingMiddleware {
        rewritten_name: &'static str,
    }

    #[async_trait]
    impl ExecutionMiddleware for RewritingMiddleware {
        async fn around_tool(&self, cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
            Ok(ToolFlow::Rewrite(ArmamentCall::new(
                self.rewritten_name,
                cx.call.arguments.clone(),
            )))
        }

        fn name(&self) -> &str {
            "rewriting"
        }
    }

    /// Pushes a fixed `PromptSection` into the assembly on every
    /// `before_model`.
    struct SectionPushingMiddleware;

    #[async_trait]
    impl ExecutionMiddleware for SectionPushingMiddleware {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            cx.assembly.push_section(PromptSection::new(
                "Injected Section",
                "injected body text",
                SectionPlacement::End,
            ));
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            "section-pushing"
        }
    }

    /// Finishes the run from `before_model` with a fixed `FinalResult`,
    /// mirroring a real `ModelCallLimit` finishing early (CR-02, 26-REVIEW.md).
    struct FinishBeforeMiddleware;

    #[async_trait]
    impl ExecutionMiddleware for FinishBeforeMiddleware {
        async fn before_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            Ok(MiddlewareFlow::Finish(FinalResult::new(
                "before-finish-output",
                StopReason::CallLimit,
            )))
        }

        fn name(&self) -> &str {
            "finish-before"
        }
    }

    /// Finishes the run from `after_model` with a fixed `FinalResult`,
    /// mirroring a real `Guardrail` rule with `on_match: Finish` (CR-02,
    /// 26-REVIEW.md). Like `Guardrail::apply_to_field`, it mutates
    /// `resp.content` in place -- the service's `after_model` call site
    /// reads the final output from `resp.content`, not `FinalResult::output`
    /// (see `guardrail.rs`'s own `apply_to_field` doc comment).
    struct FinishAfterMiddleware;

    #[async_trait]
    impl ExecutionMiddleware for FinishAfterMiddleware {
        async fn after_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
            resp: &mut LlmResponseView,
        ) -> Result<MiddlewareFlow, PaladinError> {
            resp.content = "after-finish-output".to_string();
            Ok(MiddlewareFlow::Finish(FinalResult::new(
                "after-finish-output",
                StopReason::Completed,
            )))
        }

        fn name(&self) -> &str {
            "finish-after"
        }
    }

    /// D-02's locked invariant: with no middleware attached, the rendered
    /// prompt, the `LlmPort` call count and the returned `PaladinResult` are
    /// byte-identical to a run with no chain -- the golden equivalence
    /// baseline every other test in this module is measured against.
    #[tokio::test]
    async fn empty_chain_renders_byte_identical_prompt() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("hello"));
        let service = make_service(llm.clone());
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        let expected_prompt = "You are a helpful assistant\n\nUser: hi\n";
        assert_eq!(llm.last_prompt().unwrap(), expected_prompt);
        assert_eq!(llm.call_count(), 1);
        assert_eq!(result.output, "hello");
        assert_eq!(result.stop_reason, StopReason::MaxLoops);
        assert_eq!(result.loop_count, 1);
    }

    /// `before_model`/`after_model` fire once per loop iteration, in strict
    /// alternation -- never two `before`s in a row and never a hook per
    /// buffered retry attempt.
    #[tokio::test]
    async fn recording_middleware_observes_before_model_then_after_model_per_iteration() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let log = Arc::new(Mutex::new(Vec::new()));
        let service = make_service(llm).with_middleware(Arc::new(RecordingMiddleware {
            name: "R",
            log: log.clone(),
        }));
        let paladin = make_paladin(2);

        service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "R.before(0)".to_string(),
                "R.after(0)".to_string(),
                "R.before(1)".to_string(),
                "R.after(1)".to_string(),
            ]
        );
    }

    /// CR-02 (26-REVIEW.md): when an earlier-index middleware's
    /// `before_model` finishes the run early, `run_after` still runs
    /// `after_model` over the reached prefix (D-06) -- and if one of THOSE
    /// hooks itself requests a `Finish`, that hook's own `FinalResult`
    /// (output AND stop_reason) must win over the original `before_model`
    /// `Finish`'s result, mirroring the sibling post-model-call path just
    /// below in the source.
    #[tokio::test]
    async fn after_model_finish_over_a_synthetic_view_overrides_the_before_model_finish() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("unused"));
        let service = make_service(llm).with_middleware_chain(vec![
            Arc::new(FinishAfterMiddleware),
            Arc::new(FinishBeforeMiddleware),
        ]);
        let paladin = make_paladin(3);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(result.output, "after-finish-output");
        assert_eq!(result.stop_reason, StopReason::Completed);
    }

    /// `around_tool` fires for BOTH the Arsenal branch and the handoff
    /// branch, in dispatch order, with the correct `ToolCallKind` each time.
    #[tokio::test]
    async fn around_tool_fires_for_both_arsenal_and_handoff_dispatch() {
        let llm = Arc::new(MockLlmAdapter::new().with_script(vec![
            MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            },
            MockScriptEntry::ToolCall {
                name: "handoff_to_specialist".to_string(),
                arguments: r#"{"specialist_name":"x","task_description":"y"}"#.to_string(),
            },
        ]));
        let arsenal = Arc::new(RecordingArsenal::default());
        let kinds = Arc::new(Mutex::new(Vec::new()));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(Arc::new(RecordingAroundTool {
                kinds: kinds.clone(),
            }));
        let paladin = make_paladin(2);

        service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(
            *kinds.lock().unwrap(),
            vec![ToolCallKind::Armament, ToolCallKind::Handoff]
        );
        assert_eq!(arsenal.calls.lock().unwrap().len(), 1);
    }

    /// `ToolFlow::Deny` injects `reason` exactly where the existing
    /// tool-error arm writes to, the Arsenal is never invoked, and the run
    /// continues.
    #[tokio::test]
    async fn tool_flow_deny_injects_the_reason_where_a_tool_error_is_injected_today() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(RecordingArsenal::default());
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(Arc::new(DenyingMiddleware {
                reason: "denied for test",
            }));
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(arsenal.calls.lock().unwrap().len(), 0);
        assert!(
            result.output.contains(
                "\n\n🔧 Tool Execution: lookup\nResult: FAILED\nError: denied for test\n"
            ),
            "unexpected output: {}",
            result.output
        );
    }

    /// `ToolFlow::Rewrite` causes the Arsenal to receive the rewritten
    /// `ArmamentCall`, not the model's original.
    #[tokio::test]
    async fn tool_flow_rewrite_replaces_the_call_before_dispatch() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(RecordingArsenal::default());
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_middleware(Arc::new(RewritingMiddleware {
                rewritten_name: "rewritten_tool",
            }));
        let paladin = make_paladin(1);

        service.execute(&paladin, "hi").await.unwrap();

        let calls = arsenal.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "rewritten_tool");
    }

    /// A middleware that pushes a `PromptSection` in `before_model` causes
    /// that section's text to appear in the rendered prompt the mock
    /// received.
    #[tokio::test]
    async fn middleware_mutates_the_assembly_and_the_mutation_reaches_the_rendered_prompt() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let service = make_service(llm.clone()).with_middleware(Arc::new(SectionPushingMiddleware));
        let paladin = make_paladin(1);

        service.execute(&paladin, "hi").await.unwrap();

        let prompt = llm.last_prompt().unwrap();
        assert!(
            prompt.contains("## Injected Section\ninjected body text\n"),
            "prompt did not contain the pushed section: {prompt}"
        );
    }

    /// `execute_stream` runs `before_model` exactly once and never
    /// `after_model`/`around_tool`.
    #[tokio::test]
    async fn streaming_path_runs_before_model_only() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("streamed"));
        let log = Arc::new(Mutex::new(Vec::new()));
        let service = make_service(llm).with_middleware(Arc::new(RecordingMiddleware {
            name: "S",
            log: log.clone(),
        }));
        let paladin = make_paladin(1);

        let mut stream = service.execute_stream(&paladin, "hi").await.unwrap();
        while let Some(chunk) = stream.recv().await {
            if chunk.unwrap().is_final {
                break;
            }
        }

        let recorded = log.lock().unwrap();
        let before_count = recorded.iter().filter(|s| s.contains(".before")).count();
        let after_count = recorded.iter().filter(|s| s.contains(".after")).count();
        assert_eq!(before_count, 1, "expected exactly one before_model");
        assert_eq!(after_count, 0, "expected zero after_model calls");
    }

    /// Across a two-iteration run, `loop_index` is `0, 1`, `run_id` is
    /// stable across both calls, and `cumulative_tokens` after the second
    /// iteration equals the sum of both responses' `total_tokens`.
    #[tokio::test]
    async fn context_carries_run_id_loop_index_and_cumulative_tokens() {
        struct ContextObserver {
            loop_indices: Arc<Mutex<Vec<u32>>>,
            run_ids: Arc<Mutex<Vec<uuid::Uuid>>>,
            cumulative_after: Arc<Mutex<Vec<u32>>>,
        }

        #[async_trait]
        impl ExecutionMiddleware for ContextObserver {
            async fn before_model(
                &self,
                cx: &mut ModelCallContext<'_>,
            ) -> Result<MiddlewareFlow, PaladinError> {
                self.loop_indices.lock().unwrap().push(cx.loop_index);
                self.run_ids.lock().unwrap().push(cx.run_id);
                Ok(MiddlewareFlow::Continue)
            }

            async fn after_model(
                &self,
                cx: &mut ModelCallContext<'_>,
                _resp: &mut LlmResponseView,
            ) -> Result<MiddlewareFlow, PaladinError> {
                self.cumulative_after
                    .lock()
                    .unwrap()
                    .push(cx.cumulative_tokens);
                Ok(MiddlewareFlow::Continue)
            }

            fn name(&self) -> &str {
                "context-observer"
            }
        }

        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let loop_indices = Arc::new(Mutex::new(Vec::new()));
        let run_ids = Arc::new(Mutex::new(Vec::new()));
        let cumulative_after = Arc::new(Mutex::new(Vec::new()));
        let service = make_service(llm).with_middleware(Arc::new(ContextObserver {
            loop_indices: loop_indices.clone(),
            run_ids: run_ids.clone(),
            cumulative_after: cumulative_after.clone(),
        }));
        let paladin = make_paladin(2);

        service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(*loop_indices.lock().unwrap(), vec![0, 1]);
        let ids = run_ids.lock().unwrap();
        assert_eq!(ids[0], ids[1], "run_id must be stable across iterations");

        let cumulative = cumulative_after.lock().unwrap();
        assert_eq!(cumulative.len(), 2);
        assert_eq!(
            cumulative[1],
            cumulative[0] + 30,
            "cumulative_tokens after iteration 2 must equal both responses' total_tokens summed \
             (30 each, the MockLlmAdapter default)"
        );
    }

    // ── D-33/D-34: tool-error policy (Task 2, Plan 26-19) ────────────────

    /// An `ArsenalPort` whose `invoke` always fails with a fixed
    /// `ArsenalError::TransportError` message, counting how many times it
    /// was called.
    struct FailingArsenal {
        message: String,
        calls: Mutex<u32>,
    }

    impl FailingArsenal {
        fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
                calls: Mutex::new(0),
            }
        }

        fn call_count(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    #[async_trait]
    impl ArsenalPort for FailingArsenal {
        async fn list_armaments(&self) -> Vec<crate::core::platform::container::arsenal::Armament> {
            Vec::new()
        }

        async fn invoke(
            &self,
            _call: ArmamentCall,
        ) -> Result<crate::core::platform::container::arsenal::ArmamentResult, ArsenalError>
        {
            *self.calls.lock().unwrap() += 1;
            Err(ArsenalError::TransportError(self.message.clone()))
        }

        fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
            Ok(())
        }
    }

    /// D-33: `ToolErrorConfig::default()` (`mode: FeedToModel`) is v0.9's
    /// unnamed behavior, unchanged -- a failing tool's error is appended to
    /// the accumulated output and the run continues to completion.
    #[tokio::test]
    async fn feed_to_model_is_the_default_and_matches_v0_9() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(FailingArsenal::new("transport blip"));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(arsenal.call_count(), 1);
        assert!(
            result.output.contains("🔧 Tool Execution: lookup"),
            "got {}",
            result.output
        );
        assert!(
            result.output.contains("Result: FAILED"),
            "got {}",
            result.output
        );
        assert!(
            result.output.contains("transport blip"),
            "got {}",
            result.output
        );
        assert!(
            result
                .output
                .contains("You may retry with corrected arguments or proceed without it."),
            "got {}",
            result.output
        );
    }

    /// D-33: `ToolErrorMode::FailRun` is the new opt-in -- a failing tool
    /// fails the run with a structured `PaladinError::ArmamentFailed`
    /// naming the tool, rather than feeding anything back.
    #[tokio::test]
    async fn fail_run_produces_a_structured_error() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(FailingArsenal::new("boom"));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>)
            .with_tool_error_config(ToolErrorConfig {
                mode: ToolErrorMode::FailRun,
                per_tool: HashMap::new(),
            });
        let paladin = make_paladin(1);

        let err = service.execute(&paladin, "hi").await.unwrap_err();

        match err {
            PaladinError::ArmamentFailed { tool, reason } => {
                assert_eq!(tool, "lookup");
                assert!(reason.contains("boom"), "got {reason}");
            }
            other => panic!("expected PaladinError::ArmamentFailed, got {other:?}"),
        }
    }

    /// D-34: a `per_tool` override beats the global mode -- the named
    /// tool's failure fails the run while another tool's failure (under
    /// the same service, same global default) is still fed back.
    #[tokio::test]
    async fn per_tool_override_beats_the_global_mode() {
        let config = ToolErrorConfig {
            mode: ToolErrorMode::FeedToModel,
            per_tool: HashMap::from([("danger".to_string(), ToolErrorMode::FailRun)]),
        };
        let paladin = make_paladin(1);

        // The overridden tool fails the run.
        let llm_danger =
            Arc::new(
                MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                    name: "danger".to_string(),
                    arguments: "{}".to_string(),
                }]),
            );
        let arsenal_danger = Arc::new(FailingArsenal::new("boom"));
        let service_danger =
            make_service_with_arsenal(llm_danger, arsenal_danger as Arc<dyn ArsenalPort>)
                .with_tool_error_config(config.clone());

        let err = service_danger.execute(&paladin, "hi").await.unwrap_err();
        assert!(
            matches!(err, PaladinError::ArmamentFailed { ref tool, .. } if tool == "danger"),
            "expected ArmamentFailed naming 'danger', got {err:?}"
        );

        // A different tool's failure, under the same config, is fed back
        // per the global default.
        let llm_other =
            Arc::new(
                MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                    name: "lookup".to_string(),
                    arguments: "{}".to_string(),
                }]),
            );
        let arsenal_other = Arc::new(FailingArsenal::new("boom"));
        let service_other =
            make_service_with_arsenal(llm_other, arsenal_other as Arc<dyn ArsenalPort>)
                .with_tool_error_config(config);

        let result = service_other.execute(&paladin, "hi").await.unwrap();
        assert!(
            result.output.contains("🔧 Tool Execution: lookup"),
            "got {}",
            result.output
        );
        assert!(
            result.output.contains("Result: FAILED"),
            "got {}",
            result.output
        );
    }

    /// T-26-03: a credential embedded in a failing tool's error text must
    /// never reach the model -- redacted before the fed-back text is ever
    /// appended to the accumulated output.
    #[tokio::test]
    async fn a_secret_in_a_tool_error_never_reaches_the_model() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "lookup".to_string(),
                arguments: "{}".to_string(),
            }]),
        );
        let arsenal = Arc::new(FailingArsenal::new(
            "upstream rejected: Authorization: Bearer sk-live-abcdef0123456789",
        ));
        let service = make_service_with_arsenal(llm, arsenal.clone() as Arc<dyn ArsenalPort>);
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert!(
            !result.output.contains("abcdef0123456789"),
            "got {}",
            result.output
        );
        // `paladin_llm::redaction::CREDENTIAL_PLACEHOLDER` is `pub(crate)`
        // to that crate; assert the literal it is defined as.
        assert!(
            result.output.contains("[REDACTED]"),
            "got {}",
            result.output
        );
    }

    /// D-34: the handoff arm routes through the SAME tool-error policy and
    /// the same shared formatter as the Arsenal arm -- proven end to end by
    /// forcing a handoff to fail via `HandoffConfig.max_depth = 0` (an
    /// immediate, deterministic `HandoffError::MaxDepthExceeded` with no
    /// executor call needed).
    #[tokio::test]
    async fn handoff_arm_routes_through_the_same_tool_error_policy() {
        use crate::application::services::paladin::handoff_service::HandoffService;
        use crate::core::platform::container::autonomous_config::HandoffConfig;
        use crate::core::platform::container::handoff::HandoffStrategy;

        let handoff_config = Arc::new(HandoffConfig {
            enabled: true,
            strategy: HandoffStrategy::Automatic,
            max_depth: 0,
            retry: Default::default(),
        });
        let handoff_service = HandoffService::new(handoff_config).unwrap();

        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "handoff_to_specialist".to_string(),
                arguments: r#"{"specialist_name":"x","task_description":"y"}"#.to_string(),
            }]),
        );
        let service = make_service(llm).with_handoff_service(Arc::new(handoff_service));
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        // Same shape the Arsenal arm produces -- "🔧 Tool Execution", not
        // the pre-existing "🤝 Handoff Execution" text (D-34's explicit
        // unification onto one shared formatter).
        assert!(
            result
                .output
                .contains("🔧 Tool Execution: handoff_to_specialist"),
            "got {}",
            result.output
        );
        assert!(
            result.output.contains("Result: FAILED"),
            "got {}",
            result.output
        );
    }
}

/// Phase 26 Plan 11 (RT-03, D-13, D-14): `PaladinExecutionService`'s own
/// `TokenCounterPort` field and its `recall_limit` guard -- the pieces
/// `HistoryTrimmer` needs from the service, wired independently of the
/// middleware itself (D-14, X-03: `recall_limit` only takes effect once a
/// caller explicitly installs it, mirroring how installing `HistoryTrimmer`
/// itself is opt-in).
#[cfg(test)]
mod token_counter_and_recall_limit_tests {
    use super::*;
    use crate::application::services::paladin::middleware::{
        ExecutionMiddleware, MiddlewareFlow, ModelCallContext,
    };
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, PaladinData};
    use async_trait::async_trait;
    use paladin_core::platform::container::garrison::GarrisonConfig;
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_memory::garrison::InMemoryGarrison;
    use paladin_ports::output::token_counter_port::TokenCounterPort;
    use std::sync::Mutex;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    /// Records `cx.assembly.history.len()` observed on the FIRST loop
    /// iteration of each run.
    struct HistoryLenRecorder {
        observed: Mutex<Vec<usize>>,
    }

    #[async_trait]
    impl ExecutionMiddleware for HistoryLenRecorder {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.observed
                .lock()
                .unwrap()
                .push(cx.assembly.history.len());
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            "history-len-recorder"
        }
    }

    async fn seed_garrison(n: usize) -> Arc<dyn GarrisonPort> {
        let garrison: Arc<dyn GarrisonPort> =
            Arc::new(InMemoryGarrison::new(GarrisonConfig::default()));
        for i in 0..n {
            garrison
                .remember(GarrisonEntry::new(
                    ConversationRole::User,
                    format!("seed-{i}"),
                ))
                .await
                .unwrap();
        }
        garrison
    }

    /// Test 11: with no `recall_limit` set, the service recalls 20 entries
    /// exactly as it does today; with one set (as if a `HistoryTrimmer` had
    /// been installed alongside it), it recalls that many instead.
    #[tokio::test]
    async fn recall_limit_applies_only_when_the_trimmer_is_installed() {
        let garrison = seed_garrison(25).await;
        let recorder = Arc::new(HistoryLenRecorder {
            observed: Mutex::new(Vec::new()),
        });
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            Some(garrison),
            None,
        )
        .with_middleware(recorder.clone());

        service.execute(&make_paladin(1), "hi").await.unwrap();

        assert_eq!(
            recorder.observed.lock().unwrap()[0],
            20,
            "no recall_limit set -- today's hard-coded 20"
        );

        let garrison2 = seed_garrison(25).await;
        let recorder2 = Arc::new(HistoryLenRecorder {
            observed: Mutex::new(Vec::new()),
        });
        let llm2 = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let service2 = PaladinExecutionService::new(
            llm2,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            Some(garrison2),
            None,
        )
        .with_recall_limit(5)
        .with_middleware(recorder2.clone());

        service2.execute(&make_paladin(1), "hi").await.unwrap();

        assert_eq!(
            recorder2.observed.lock().unwrap()[0],
            5,
            "recall_limit installed -- honoured instead of the hard-coded 20"
        );
    }

    /// A counter test double distinct from `HeuristicTokenCounter`, so an
    /// override is provably distinguishable from the default.
    struct AlwaysOneCounter;

    impl TokenCounterPort for AlwaysOneCounter {
        fn count(&self, _text: &str, _model: &str) -> u32 {
            1
        }

        fn name(&self) -> &str {
            "always-one-test-counter"
        }
    }

    /// `token_counter()` defaults to `HeuristicTokenCounter`;
    /// `with_token_counter` overrides it.
    #[test]
    fn token_counter_defaults_to_heuristic_and_with_token_counter_overrides_it() {
        let llm = Arc::new(MockLlmAdapter::new());
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        );
        assert_eq!(service.token_counter().name(), "heuristic");

        let service = service.with_token_counter(Arc::new(AlwaysOneCounter));
        assert_eq!(service.token_counter().name(), "always-one-test-counter");
    }
}

/// RT-05 / D-27: `PaladinExecutionService::execute_json_schema` (belt and
/// braces -- `response_format` AND the instruction block, on every model
/// call), the shared `run_structured` driver, and `PaladinPort` unchanged.
#[cfg(test)]
mod structured_output_tests {
    use super::*;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::{MaxLoops, PaladinData};
    use paladin_core::platform::container::heartbeat::HeartbeatHandle;
    use paladin_core::platform::container::prompt::PromptType;
    use paladin_llm::mock::MockLlmAdapter;

    fn make_paladin() -> Paladin {
        let data = PaladinData {
            system_prompt: "system".to_string(),
            max_loops: MaxLoops::Fixed(3),
            ..Default::default()
        };
        Node::new(data, None)
    }

    fn weather_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "required": ["city", "temp_c"],
            "properties": {
                "city": {"type": "string"},
                "temp_c": {"type": "number"}
            }
        })
    }

    fn service_with(llm: Arc<MockLlmAdapter>) -> PaladinExecutionService {
        PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(50, 25, Duration::from_secs(60))),
            None,
            None,
        )
    }

    fn prompt_text(request: &LlmRequest) -> String {
        match request.prompt.prompt_type() {
            PromptType::User(user) => user.query.clone(),
            PromptType::System(system) => system.instructions.clone(),
            _ => String::new(),
        }
    }

    /// Test 1: a structured run sets `response_format` on the FIRST call
    /// and on the repair attempt.
    #[tokio::test]
    async fn structured_run_sets_response_format_on_every_model_call() {
        let mock = Arc::new(MockLlmAdapter::new().with_responses(vec![
            r#"{"city": "Oslo"}"#.to_string(),
            r#"{"city": "Oslo", "temp_c": 4.5}"#.to_string(),
        ]));
        let service = service_with(mock.clone());
        let paladin = make_paladin();

        let result = service
            .execute_json_schema(
                &paladin,
                "what is the weather in Oslo",
                &weather_schema(),
                &StructuredOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            result.value,
            serde_json::json!({"city": "Oslo", "temp_c": 4.5})
        );

        let requests = mock.requests();
        assert_eq!(requests.len(), 2, "one initial call plus one repair");
        assert!(
            requests.iter().all(|r| r.response_format.is_some()),
            "response_format must be set on EVERY model call, including the repair attempt"
        );
    }

    /// Test 2: both prompts (first attempt and repair) contain
    /// `render_instruction_block`'s literal output -- belt and braces.
    #[tokio::test]
    async fn structured_run_also_appends_the_instruction_block() {
        let mock = Arc::new(MockLlmAdapter::new().with_responses(vec![
            r#"{"city": "Oslo"}"#.to_string(),
            r#"{"city": "Oslo", "temp_c": 4.5}"#.to_string(),
        ]));
        let service = service_with(mock.clone());
        let paladin = make_paladin();

        service
            .execute_json_schema(
                &paladin,
                "what is the weather in Oslo",
                &weather_schema(),
                &StructuredOptions::default(),
            )
            .await
            .unwrap();

        let requests = mock.requests();
        assert_eq!(requests.len(), 2);
        let instruction = render_instruction_block(&weather_schema());
        for request in &requests {
            assert!(
                prompt_text(request).contains(&instruction),
                "every model call's prompt must contain render_instruction_block's \
                 literal output: {}",
                prompt_text(request)
            );
        }
    }

    /// Test 3: a conforming first response returns `Structured { value, raw }`
    /// with `raw.loop_count` reflecting exactly one call.
    #[tokio::test]
    async fn happy_path_returns_a_parsed_value_after_one_call() {
        let mock =
            Arc::new(MockLlmAdapter::new().with_response(r#"{"city": "Oslo", "temp_c": 4.5}"#));
        let service = service_with(mock.clone());
        let paladin = make_paladin();

        let result = service
            .execute_json_schema(
                &paladin,
                "what is the weather in Oslo",
                &weather_schema(),
                &StructuredOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            result.value,
            serde_json::json!({"city": "Oslo", "temp_c": 4.5})
        );
        assert_eq!(result.raw.loop_count, 1);
        assert_eq!(mock.requests().len(), 1);
    }

    /// Test 4: repair succeeds on attempt two; `raw.loop_count` reflects
    /// BOTH calls.
    #[tokio::test]
    async fn repair_succeeds_on_attempt_two() {
        let mock = Arc::new(MockLlmAdapter::new().with_responses(vec![
            r#"{"city": "Oslo"}"#.to_string(),
            r#"{"city":"Oslo","temp_c":4.5}"#.to_string(),
        ]));
        let service = service_with(mock.clone());
        let paladin = make_paladin();

        let result = service
            .execute_json_schema(
                &paladin,
                "what is the weather in Oslo",
                &weather_schema(),
                &StructuredOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            result.value,
            serde_json::json!({"city": "Oslo", "temp_c": 4.5})
        );
        assert_eq!(result.raw.loop_count, 2);
        assert_eq!(mock.requests().len(), 2);
    }

    /// Test 5: with `max_repair_attempts: 1` and two non-conforming
    /// responses, exhaustion returns `StructuredOutputInvalid { attempts: 2,
    /// raw_output }` where `raw_output` is the SECOND response verbatim.
    #[tokio::test]
    async fn exhaustion_preserves_the_raw_output() {
        let mock = Arc::new(MockLlmAdapter::new().with_responses(vec![
            r#"{"nope": 1}"#.to_string(),
            r#"{"nope": 2}"#.to_string(),
        ]));
        let service = service_with(mock.clone());
        let paladin = make_paladin();

        let err = service
            .execute_json_schema(
                &paladin,
                "what is the weather in Oslo",
                &weather_schema(),
                &StructuredOptions::new(1),
            )
            .await
            .unwrap_err();

        match err {
            PaladinError::StructuredOutputInvalid {
                attempts,
                raw_output,
                ..
            } => {
                assert_eq!(attempts, 2);
                assert_eq!(raw_output, r#"{"nope": 2}"#);
            }
            other => panic!("expected StructuredOutputInvalid, got {other:?}"),
        }
    }

    /// Test 6: a caller using `execute_json_schema_observed` on a service
    /// that has not overridden it gets the same result as
    /// `execute_json_schema` (the D-19 defaulted-method pattern).
    #[tokio::test]
    async fn the_observed_variant_defaults_to_the_plain_one() {
        let paladin = make_paladin();
        let schema = weather_schema();

        let mock1 =
            Arc::new(MockLlmAdapter::new().with_response(r#"{"city": "Oslo", "temp_c": 4.5}"#));
        let service1 = service_with(mock1);
        let plain = service1
            .execute_json_schema(&paladin, "weather", &schema, &StructuredOptions::default())
            .await
            .unwrap();

        let mock2 =
            Arc::new(MockLlmAdapter::new().with_response(r#"{"city": "Oslo", "temp_c": 4.5}"#));
        let service2 = service_with(mock2);
        let heartbeat = HeartbeatHandle::new();
        let observed = service2
            .execute_json_schema_observed(
                &paladin,
                "weather",
                &schema,
                &StructuredOptions::default(),
                &heartbeat,
            )
            .await
            .unwrap();

        assert_eq!(plain.value, observed.value);
    }

    /// Test 7: a compile-visible assertion that `PaladinPort` has no
    /// structured method -- a minimal implementor with only the
    /// pre-existing methods compiles.
    #[test]
    fn paladin_port_gained_nothing() {
        use paladin_ports::output::paladin_port::PaladinPort;

        struct ExecuteOnlyPort;

        #[async_trait::async_trait]
        impl PaladinPort for ExecuteOnlyPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Ok(PaladinResult {
                    output: format!("echo:{input}"),
                    loop_count: 1,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                })
            }

            async fn execute_stream(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinStream, PaladinError> {
                unimplemented!("not exercised")
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let _port: Arc<dyn PaladinPort> = Arc::new(ExecuteOnlyPort);
    }

    /// Test 8: a source-level assertion that the facade contains no second
    /// attempt loop -- `execute_json_schema` supplies only an `execute_fn`
    /// to the shared `run_structured` driver (paladin-ports).
    #[test]
    fn the_repair_loop_is_the_shared_driver() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/application/services/paladin/paladin_execution_service.rs");
        let source = std::fs::read_to_string(&path).expect("read this file's own source");

        let start = source
            .find("impl StructuredExecutorPort for PaladinExecutionService")
            .expect("the StructuredExecutorPort impl block must exist");
        let from_start = &source[start..];
        let end = from_start
            .find("\nimpl PaladinExecutionService {")
            .expect("the impl block must be followed by the next impl block");
        let block = &from_start[..end];

        assert_eq!(
            block.matches("run_structured(").count(),
            1,
            "execute_json_schema must call run_structured exactly once, not \
             re-implement a repair loop"
        );
        assert_eq!(
            block.matches("loop {").count(),
            0,
            "the facade's StructuredExecutorPort impl must contain no manual \
             repair loop -- run_structured (paladin-ports) owns the only \
             bounded attempt loop"
        );
    }
}
