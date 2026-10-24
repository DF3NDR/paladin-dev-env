//! Commander Strategy Router
//!
//! Provides unified interface for selecting and executing Battalion orchestration patterns.
//! Supports both manual strategy selection and Auto mode with rule-based heuristics.

use log::{debug, info, warn};
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

use crate::campaign_service::CampaignExecutionService;
use crate::chain_of_command_service::ChainOfCommandExecutionService;
use crate::conclave_execution_service::ConclaveExecutionService;
use crate::council_service::CouncilExecutionService;
use crate::formation_service::FormationExecutionService;
use crate::grove_service::GroveExecutionService;
use crate::in_memory_registry::HashMapPaladinRegistry;
use crate::llm_decision::llm_error_class;
use crate::maneuver::service::ManeuverExecutionService;
use crate::phalanx_service::PhalanxExecutionService;
use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
use paladin_core::platform::container::battalion::conclave::ConclaveConfig;
use paladin_core::platform::container::battalion::{
    BattalionConfig, BattalionError, BattalionResult, BattalionStrategy,
};
use paladin_core::platform::container::herald::Herald;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use paladin_ports::output::paladin_port::PaladinPort;
use paladin_ports::output::paladin_registry::PaladinRegistry;

/// How [`Commander`] resolves `BattalionStrategy::Auto` (CF-05, D-25).
///
/// Off by default: a `Commander` built without calling
/// [`CommanderBuilder::strategy_selection`] uses [`StrategySelection::Heuristic`]
/// -- today's keyword-based crate-private `Commander::analyze_and_select`
/// routine, unchanged. No
/// `APP_*` environment variable, cargo feature, or config-struct field can
/// reach [`StrategySelection::Semantic`] (D-26); a workflow author reaches it
/// only by constructing one in code.
///
/// # Examples
///
/// ```
/// use paladin_battalion::commander::StrategySelection;
///
/// assert!(matches!(StrategySelection::default(), StrategySelection::Heuristic));
/// ```
#[derive(Clone, Default)]
pub enum StrategySelection {
    /// Today's keyword-based heuristic (the crate-private
    /// `Commander::analyze_and_select` routine), unchanged. The default.
    #[default]
    Heuristic,
    /// Prompt `llm` with the strategy catalog and the run's input, parse the
    /// answer as a strategy name (exact-after-trim, case-insensitive). Any
    /// LLM error, or an answer that names no catalog strategy, falls back to
    /// [`StrategySelection::Heuristic`] deterministically, recording the
    /// fallback and its cause in `BattalionResult::strategy_selection_reasoning`.
    Semantic {
        /// The provider to prompt for a strategy name.
        llm: Arc<dyn LlmPort>,
        /// The model identifier to request.
        model: String,
    },
}

// `Arc<dyn LlmPort>` is not `Debug`, so this impl is manual -- and
// deliberately prints the variant name and model string only, never the
// port itself (T-23-10: no credential the port may hold can leak through a
// `{:?}` log line).
impl std::fmt::Debug for StrategySelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategySelection::Heuristic => f.write_str("Heuristic"),
            StrategySelection::Semantic { model, .. } => f
                .debug_struct("Semantic")
                .field("llm", &"<dyn LlmPort>")
                .field("model", model)
                .finish(),
        }
    }
}

/// Commander for routing Battalion execution to appropriate strategies.
///
/// The Commander is the primary interface for orchestrating multiple Paladins in coordinated
/// workflows. It provides intelligent strategy selection and unified execution across all
/// Battalion patterns: Formation, Phalanx, Campaign, and ChainOfCommand.
///
/// # Features
///
/// - **Auto Mode**: Automatically selects optimal strategy based on input analysis and Paladin count
/// - **Explicit Strategy**: Manually select Formation, Phalanx, Campaign, ChainOfCommand, Conclave, Council, or Grove
/// - **Per-attempt Aegis policy**: `config.aegis` bounds and retries each Paladin attempt of
///   Formation, Phalanx, Campaign and the Conclave (`aegis.timeout` is a per-attempt bound, not a
///   whole-run clock; `aegis.retry` counts total attempts). The Commander adds no bound of its
///   own: ChainOfCommand and Grove do not read `aegis`, and a Maneuver reads only
///   `aegis.timeout.run_timeout`, as one whole-flow clock (a set `aegis.on_error` or
///   `aegis.retry` is ignored, with a one-time warning per execution; an explicit
///   `CommanderBuilder::maneuver_config` replaces the aegis-derived config, timeout included,
///   and reads no `aegis` at all, so it draws no warning)
/// - **Error Handling**: fail fast by default; `aegis.on_error = Absorb` lets a Formation continue
///   past a failed Paladin
/// - **Telemetry**: Comprehensive execution metadata including timing and success/failure counts
///
/// # Auto Mode Heuristics
///
/// When using `BattalionStrategy::Auto`, the Commander applies the following rules:
///
/// 1. **Conclave** (Expert Synthesis)
///    - 3+ Paladins with diverse expertise
///    - Keywords: "synthesize", "compare", "expert panel", "perspectives", "consensus"
///
/// 2. **Council** (Collaborative Discussion)
///    - 2+ Paladins for turn-based dialogue
///    - Keywords: "discuss", "debate", "collaborate", "consensus", "brainstorm"
///
/// 3. **Grove** (Intelligent Routing)
///    - 2+ Paladins with specialized capabilities
///    - Keywords: "route", "best agent", "expertise", "most qualified"
///
/// 4. **Formation** (Sequential)
///    - 1-3 Paladins by default
///    - Keywords: "sequential", "pipeline", "step by step", "one after", "first then"
///
/// 5. **Phalanx** (Parallel)
///    - 4+ Paladins with independent tasks
///    - Keywords: "parallel", "concurrent", "all at once", "simultaneously"
///
/// 6. **Campaign** (Graph/Workflow)
///    - Complex multi-stage workflows
///    - Keywords: "workflow", "graph", "conditional", "if-then", "depends on"
///
/// 7. **ChainOfCommand** (Hierarchical)
///    - Specialist delegation patterns
///    - Keywords: "delegate", "hierarchy", "specialist", "expert", "assign to"
///
/// # Examples
///
/// ## Basic Usage with Explicit Strategy
///
/// ```ignore
/// use paladin_battalion::commander::CommanderBuilder;
/// use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
/// use paladin_core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// // Create Commander with Formation strategy
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Formation)
///     .paladins(vec![analyzer, enhancer, reviewer])
///     .config(BattalionConfig::new("sequential_pipeline").with_aegis(Aegis {
///         // Each Paladin attempt may run for at most 60 seconds.
///         timeout: Some(TimeoutPolicy {
///             run_timeout: Some(Duration::from_secs(60)),
///             idle_timeout: None,
///         }),
///         ..Aegis::default()
///     }))
///     .build()?;
///
/// // Execute with input
/// let result = commander.execute("Process this data").await?;
/// println!("Final output: {}", result.final_output);
/// ```
///
/// ## Auto Mode with Telemetry
///
/// ```ignore
/// use paladin_core::platform::container::battalion::BattalionStrategy;
///
/// // Auto mode will select the best strategy
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Auto)
///     .paladins(vec![worker1, worker2, worker3, worker4, worker5])
///     .build()?; // Uses default config
///
/// let result = commander.execute("Process batch in parallel").await?;
///
/// // Check what strategy was selected
/// println!("Selected: {:?}", result.strategy_used);
/// if let Some(reasoning) = &result.strategy_selection_reasoning {
///     println!("Because: {}", reasoning);
/// }
/// println!("Selection took: {}ms", result.strategy_selection_time_ms);
/// ```
///
/// ## Production Configuration
///
/// ```ignore
/// use paladin_core::platform::container::aegis::{
///     Aegis, ErrorHandlerSpec, RetryPolicy, RetryPredicate, TimeoutPolicy,
/// };
/// use paladin_core::platform::container::battalion::BattalionConfig;
/// use paladin_core::platform::container::battlefield::StateDelta;
/// use std::time::Duration;
///
/// let config = BattalionConfig::new("production_battalion").with_aegis(Aegis {
///     timeout: Some(TimeoutPolicy {
///         run_timeout: Some(Duration::from_secs(300)),
///         idle_timeout: None,
///     }),
///     // `max_attempts` counts TOTAL attempts, including the first.
///     retry: Some(RetryPolicy {
///         max_attempts: 3,
///         retry_on: RetryPredicate::TransientAndUnknown,
///         ..RetryPolicy::default()
///     }),
///     // Absorb lets a Formation continue past a failed Paladin.
///     on_error: Some(ErrorHandlerSpec::Absorb {
///         fallback_delta: StateDelta::new(),
///     }),
///     ..Aegis::default()
/// });
///
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Formation)
///     .paladins(paladins)
///     .config(config)
///     .build()?;
///
/// match commander.execute("Critical task").await {
///     Ok(result) => {
///         println!("Success: {} succeeded, {} failed",
///             result.paladin_success_count,
///             result.paladin_failure_count);
///     }
///     Err(e) => eprintln!("Battalion failed: {}", e),
/// }
/// ```
///
/// # See Also
///
/// - [`CommanderBuilder`] - Fluent builder for creating Commander instances
/// - [`BattalionStrategy`] - Available orchestration patterns
/// - [`BattalionConfig`] - Configuration options for execution
/// - [`BattalionResult`] - Result type with execution metadata
pub struct Commander {
    /// Unique identifier for this Commander instance
    pub id: Uuid,

    /// Selected orchestration strategy
    pub strategy: BattalionStrategy,

    /// Paladins to orchestrate
    pub paladins: Vec<Paladin>,

    /// Battalion configuration
    pub config: BattalionConfig,

    /// Optional aggregator Paladin for Conclave strategy
    pub aggregator: Option<Paladin>,

    /// Optional flow expression for Maneuver strategy
    pub flow_expression: Option<String>,

    /// Optional Maneuver configuration
    pub maneuver_config: Option<crate::maneuver::ManeuverConfig>,

    /// Paladin execution port
    paladin_port: Arc<dyn PaladinPort>,

    /// Optional Herald for formatting Battalion results
    herald: Option<Arc<dyn Herald>>,

    /// How `BattalionStrategy::Auto` is resolved (CF-05, D-25). Private --
    /// not part of the public constructor signature; set only through
    /// [`CommanderBuilder::strategy_selection`]. Defaults to
    /// [`StrategySelection::Heuristic`].
    strategy_selection: StrategySelection,

    /// `true` when `CommanderBuilder::build` materialised `maneuver_config` from the
    /// aegis-derived Maneuver config plus an `error_strategy(..)` override, rather than from an
    /// explicit `maneuver_config(..)`. Such a config still follows `aegis.timeout.run_timeout`,
    /// so the one-time ignored-policy warning still applies to it. Private: it is not part of
    /// the public constructor signature.
    maneuver_config_from_aegis: bool,
}

impl std::fmt::Debug for Commander {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commander")
            .field("id", &self.id)
            .field("strategy", &self.strategy)
            .field("paladins", &self.paladins)
            .field("config", &self.config)
            .field("aggregator", &self.aggregator)
            .field("flow_expression", &self.flow_expression)
            .field("maneuver_config", &self.maneuver_config)
            .field("paladin_port", &"<dyn PaladinPort>")
            .field(
                "herald",
                &self
                    .herald
                    .as_ref()
                    .map(|_| "<dyn Herald>")
                    .unwrap_or("None"),
            )
            .field("strategy_selection", &self.strategy_selection)
            .finish()
    }
}

/// The `aegis` policies a Maneuver run cannot honour, in a fixed order (D-24, WR-04).
///
/// A Maneuver reads only `aegis.timeout.run_timeout`, as one whole-flow clock rather than per
/// attempt (see `Commander::default_maneuver_config`). It has no retry loop and no continue
/// mode, so a set `aegis.on_error` or `aegis.retry` is dropped. This returns the dropped
/// fields, `aegis.on_error` first and `aegis.retry` second, and an empty vector when neither is
/// set (a timeout alone is honoured, so it is never named). The function is pure so the
/// decision is unit-tested directly: the repository does not capture log output.
fn maneuver_ignored_aegis_policies(aegis: &Aegis) -> Vec<&'static str> {
    let mut ignored = Vec::new();
    if aegis.on_error.is_some() {
        ignored.push("aegis.on_error");
    }
    if aegis.retry.is_some() {
        ignored.push("aegis.retry");
    }
    ignored
}

impl Commander {
    /// Create a new Commander instance
    ///
    /// # Arguments
    ///
    /// * `strategy` - Orchestration strategy to use
    /// * `paladins` - Vector of Paladins to orchestrate
    /// * `config` - Battalion configuration
    /// * `aggregator` - Optional aggregator Paladin for Conclave strategy
    /// * `paladin_port` - Port for executing Paladins
    ///
    /// # Returns
    ///
    /// A new Commander instance with generated UUID and creation timestamp
    pub fn new(
        strategy: BattalionStrategy,
        paladins: Vec<Paladin>,
        config: BattalionConfig,
        aggregator: Option<Paladin>,
        paladin_port: Arc<dyn PaladinPort>,
    ) -> Self {
        let id = Uuid::new_v4();
        info!(
            "Creating Commander {} with strategy {:?} and {} Paladins",
            id,
            strategy,
            paladins.len()
        );

        Self {
            id,
            strategy,
            paladins,
            config,
            aggregator,
            flow_expression: None,
            maneuver_config: None,
            paladin_port,
            herald: None,
            strategy_selection: StrategySelection::default(),
            maneuver_config_from_aegis: false,
        }
    }

    /// Set the Herald for formatting results
    ///
    /// This allows runtime override of the default Herald. If set, this Herald
    /// will be used to format Battalion results.
    ///
    /// # Arguments
    ///
    /// * `herald` - The Herald to use for formatting
    ///
    /// # Example
    ///
    /// ```ignore
    /// let commander = Commander::new(strategy, paladins, config, None, paladin_port)
    ///     .with_herald(Arc::new(JsonHerald::new()));
    /// ```
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Format a Battalion result using the configured Herald
    ///
    /// Converts the Battalion result into the Herald's output format. If no Herald
    /// is configured, returns None.
    ///
    /// # Arguments
    ///
    /// * `result` - The Battalion result to format
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - Formatted output if Herald is configured
    /// * `Ok(None)` - If no Herald is configured
    /// * `Err(BattalionError)` - If formatting fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let formatted = commander.format_result(&result)?;
    /// if let Some(output) = formatted {
    ///     println!("{}", output);
    /// }
    /// ```
    pub fn format_result(
        &self,
        result: &BattalionResult,
    ) -> Result<Option<String>, BattalionError> {
        match &self.herald {
            Some(herald) => herald
                .format_battalion_result(result)
                .map(Some)
                .map_err(|e| {
                    BattalionError::CommanderValidation(format!("Herald formatting error: {}", e))
                }),
            None => Ok(None),
        }
    }

    /// Execute the Commander's Battalion with the given input.
    ///
    /// This is the primary entry point for Battalion execution. It handles strategy
    /// resolution, service delegation, and result enrichment.
    ///
    /// # Execution Flow
    ///
    /// 1. **Strategy Resolution**: If using Auto mode, analyzes input and Paladin characteristics
    ///    to select the optimal strategy. Otherwise, uses the explicitly configured strategy.
    ///
    /// 2. **Service Delegation**: Routes to the appropriate Battalion service:
    ///    - Formation -> Sequential pipeline execution
    ///    - Phalanx -> Concurrent parallel execution  
    ///    - Campaign -> Graph-based workflow execution
    ///    - ChainOfCommand -> Hierarchical delegation execution
    ///
    /// 3. **Result Enrichment**: Enhances the result with Commander-specific metadata:
    ///    - `strategy_used`: The actual strategy executed (resolved from Auto if applicable)
    ///    - `strategy_selection_reasoning`: Explanation of why the strategy was chosen
    ///    - `strategy_selection_time_ms`: Time spent selecting strategy
    ///    - Timing and success/failure counts
    ///
    /// # Arguments
    ///
    /// * `input` - The initial input string to provide to the Battalion. For:
    ///   - **Formation**: Provided to the first Paladin; subsequent Paladins receive prior output
    ///   - **Phalanx**: Provided identically to all Paladins in parallel
    ///   - **Campaign**: Provided to entry point Paladin(s) in the graph
    ///   - **ChainOfCommand**: Provided to the commander Paladin for delegation decisions
    ///
    /// # Returns
    ///
    /// * `Ok(BattalionResult)` - Successful execution with:
    ///   - `final_output`: The final result from the Battalion
    ///   - `status`: BattalionStatus::Completed
    ///   - `strategy_used`: The strategy that was actually executed
    ///   - `strategy_selection_reasoning`: Auto mode explanation (if applicable)
    ///   - `paladin_success_count` / `paladin_failure_count`: Execution statistics
    ///   - `started_at` / `completed_at`: Timestamp metadata
    ///   - Additional telemetry fields
    ///
    /// # Timeouts
    ///
    /// The Commander adds no wall clock of its own. Each Paladin attempt of Formation,
    /// Phalanx, Campaign and the Conclave is bounded by `config.aegis.timeout` -- **per
    /// attempt**, never for the whole run. ChainOfCommand, Grove and Council are not bounded by
    /// `aegis`. A caller that needs a whole-run bound wraps `execute()` in its own
    /// `tokio::time::timeout`.
    ///
    /// # Errors
    ///
    /// * `BattalionError::Node` - A Paladin attempt exceeded `aegis.timeout`
    ///   (`NodeErrorSource::Timeout`)
    /// * `BattalionError::ExecutionError` - One or more Paladins failed (if using FailFast)
    /// * `BattalionError::ValidationError` - Invalid Battalion configuration or state
    /// * `BattalionError::PaladinError` - Underlying Paladin execution failure
    /// * Other strategy-specific errors from delegated services
    ///
    /// # Examples
    ///
    /// ## Basic Execution
    ///
    /// ```ignore
    /// let result = commander.execute("Analyze this customer feedback").await?;
    /// println!("Analysis result: {}", result.final_output);
    /// ```
    ///
    /// ## With Error Handling
    ///
    /// ```ignore
    /// match commander.execute("Process data").await {
    ///     Ok(result) => {
    ///         println!("✅ Success: {}", result.final_output);
    ///         println!("   Strategy: {:?}", result.strategy_used);
    ///         println!("   Duration: {}ms",
    ///             result.completed_at.signed_duration_since(result.started_at).num_milliseconds());
    ///     }
    ///     Err(BattalionError::Node(node_error)) => {
    ///         eprintln!("❌ Paladin attempt failed: {:?}", node_error.source);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("❌ Execution failed: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// ## Analyzing Auto Mode Selection
    ///
    /// ```ignore
    /// let result = commander.execute("Run these tasks in parallel").await?;
    ///
    /// // Auto mode provides reasoning
    /// if let Some(reasoning) = result.strategy_selection_reasoning {
    ///     println!("Auto mode selected {:?}", result.strategy_used);
    ///     println!("Reasoning: {}", reasoning);
    ///     println!("Selection time: {}ms", result.strategy_selection_time_ms);
    /// }
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - Auto mode adds 0-5ms overhead for strategy analysis
    /// - Timeouts are enforced per Paladin attempt by the delegated services, not by the Commander
    /// - Telemetry collection adds minimal overhead (<1ms typically)
    ///
    /// # See Also
    ///
    /// - [`BattalionStrategy`] - Available orchestration patterns
    /// - [`BattalionResult`] - Detailed result structure
    /// - [`BattalionConfig`] - Configuration options affecting execution
    pub async fn execute(&self, input: &str) -> Result<BattalionResult, BattalionError> {
        self.execute_internal(input).await
    }

    /// Bridge the Battalion's [`Aegis`] policy onto a [`ConclaveConfig`] (D-04).
    ///
    /// The per-attempt timeout and the backoff shape travel inside the cloned
    /// `battalion_config.aegis`, which the Conclave reads directly; this only
    /// derives the retry **count**. `aegis.retry.max_attempts` counts total
    /// attempts while `retry_attempts` counts retries after the first, so the
    /// bridge sets `max_attempts - 1` (clamped to 5 by
    /// [`ConclaveConfig::with_retry_attempts`]). With no `aegis.retry` the
    /// `ConclaveConfig` default of 2 applies.
    fn conclave_config_from_battalion(&self) -> ConclaveConfig {
        let config = ConclaveConfig::new(&self.config.name, self.config.clone());
        match &self.config.aegis.retry {
            Some(policy) => config.with_retry_attempts(policy.max_attempts.saturating_sub(1)),
            None => config,
        }
    }

    /// The Maneuver configuration used when none was set explicitly (D-04).
    ///
    /// The error strategy is the Maneuver default (`FailFast`) whatever
    /// `aegis.on_error` says (`CommanderBuilder::error_strategy` overrides just that field on
    /// top of this config, keeping the aegis-derived timeout); the Maneuver's own workflow timeout is
    /// `aegis.timeout.run_timeout` (`None` when no timeout is set). Note the change of
    /// meaning: on Formation, Phalanx and Campaign `run_timeout` bounds each attempt, while here
    /// it becomes one whole-flow clock for the entire Maneuver. `aegis.on_error` and
    /// `aegis.retry` are not read at all; the Commander warns once per execution when either is
    /// set and this derived config is in use (`maneuver_warned_aegis_policies`).
    fn default_maneuver_config(&self) -> crate::maneuver::ManeuverConfig {
        crate::maneuver::ManeuverConfig {
            error_strategy: crate::maneuver::ErrorStrategy::default(),
            output_format: crate::maneuver::OutputFormat::Concatenate,
            pass_output_as_input: true,
            timeout: self
                .config
                .aegis
                .timeout
                .as_ref()
                .and_then(|policy| policy.run_timeout),
            collect_timing_metrics: true,
            detailed_observability: false,
        }
    }

    /// The ignored `aegis` policies the Maneuver warning must name (empty: no warning).
    ///
    /// The warning is only true when the Maneuver config is derived from `aegis`
    /// (`default_maneuver_config`): with an explicit `ManeuverConfig` the Maneuver reads none
    /// of `aegis`, so the message ("Maneuver reads only aegis.timeout.run_timeout") would be
    /// false for that run. A config built by `CommanderBuilder::error_strategy` alone still
    /// follows `aegis.timeout.run_timeout`, so it counts as derived. Pure, so the gating is
    /// unit-tested directly.
    fn maneuver_warned_aegis_policies(&self) -> Vec<&'static str> {
        if self.maneuver_config.is_some() && !self.maneuver_config_from_aegis {
            return Vec::new();
        }
        maneuver_ignored_aegis_policies(&self.config.aegis)
    }

    /// Internal execution logic (strategy resolution, delegation, enrichment)
    async fn execute_internal(&self, input: &str) -> Result<BattalionResult, BattalionError> {
        let start_time = std::time::Instant::now();
        let started_at = chrono::Utc::now();

        // Resolve strategy (Auto mode uses the configured StrategySelection --
        // today's Heuristic by default, or Semantic if the builder set one).
        let (effective_strategy, selection_reason) = match &self.strategy {
            BattalionStrategy::Auto => {
                let (selected, reason) = self.select_strategy(input).await;
                info!(
                    "Commander {} Auto mode selected {:?}: {}",
                    self.id, selected, reason
                );
                (selected, Some(reason))
            }
            explicit_strategy => {
                debug!(
                    "Commander {} using explicit strategy {:?}",
                    self.id, explicit_strategy
                );
                (explicit_strategy.clone(), None)
            }
        };

        let selection_time_ms = start_time.elapsed().as_millis() as u64;
        debug!(
            "Strategy selection took {}ms for Commander {}",
            selection_time_ms, self.id
        );

        // Log execution details
        info!(
            "Commander {} executing {} Paladins with {:?} strategy",
            self.id,
            self.paladins.len(),
            effective_strategy
        );

        // Delegate to appropriate service
        let mut result = match effective_strategy {
            BattalionStrategy::Formation => {
                debug!("Delegating to FormationExecutionService");
                let formation =
                    paladin_core::platform::container::battalion::formation::Formation::new(
                        self.paladins.clone(),
                        self.config.clone(),
                    )?;
                let service = FormationExecutionService::new(Arc::clone(&self.paladin_port));
                service.execute(&formation, input).await?
            }
            BattalionStrategy::Phalanx => {
                debug!("Delegating to PhalanxExecutionService");
                let phalanx = paladin_core::platform::container::battalion::phalanx::Phalanx::new(
                    self.paladins.clone(),
                    self.config.clone(),
                )?;
                let service = PhalanxExecutionService::new(Arc::clone(&self.paladin_port));
                service.execute(&phalanx, input).await?
            }
            BattalionStrategy::Campaign => {
                debug!("Delegating to CampaignExecutionService");
                // For Campaign, create a simple linear graph from paladins
                let mut campaign =
                    paladin_core::platform::container::battalion::campaign::Campaign::new(
                        self.config.clone(),
                    );

                // Add all Paladins to the campaign
                let mut paladin_ids: Vec<uuid::Uuid> = Vec::new();
                for paladin in &self.paladins {
                    let paladin_clone: paladin_core::platform::container::paladin::Paladin =
                        paladin.clone();
                    let id = campaign.add_paladin(paladin_clone);
                    paladin_ids.push(id);
                }

                // Create linear edges: paladin_0 -> paladin_1 -> paladin_2 -> ...
                for i in 0..paladin_ids.len().saturating_sub(1) {
                    let edge = paladin_core::platform::container::battalion::campaign::CampaignEdge::new(
                        paladin_ids[i],
                        paladin_ids[i + 1],
                        paladin_core::platform::container::battalion::campaign::EdgeCondition::Always,
                    );
                    campaign.add_edge(edge)?;
                }

                // Set first Paladin as entry point
                if !paladin_ids.is_empty() {
                    campaign.set_entry_point(paladin_ids[0])?;
                }

                let service = CampaignExecutionService::new(Arc::clone(&self.paladin_port));
                service.execute(&campaign, input).await?
            }
            BattalionStrategy::ChainOfCommand => {
                debug!("Delegating to ChainOfCommandExecutionService");
                // For ChainOfCommand, use first Paladin as commander, rest as specialists
                if self.paladins.is_empty() {
                    return Err(BattalionError::ValidationError(
                        "ChainOfCommand requires at least 1 Paladin".to_string(),
                    ));
                }
                let commander = self.paladins[0].clone();
                let specialists = if self.paladins.len() > 1 {
                    self.paladins[1..].to_vec()
                } else {
                    // If only 1 Paladin, use it as both commander and specialist
                    vec![self.paladins[0].clone()]
                };
                let chain = paladin_core::platform::container::battalion::chain_of_command::ChainOfCommand::new(
                    commander,
                    specialists,
                    self.config.clone(),
                )?;
                let service = ChainOfCommandExecutionService::new(Arc::clone(&self.paladin_port));
                let delegation_result = service.execute(&chain, input).await?;

                // Convert DelegationResult to BattalionResult via the service's single shared
                // conversion method (see `ChainOfCommandExecutionService`), rather than
                // constructing a second inline copy here. Keeping this the only call site
                // means the Commander and the service can never report divergent results for
                // the same execution.
                service.to_battalion_result(&chain, &delegation_result, started_at)
            }
            BattalionStrategy::Conclave => {
                debug!("Delegating to ConclaveExecutionService");

                // Determine experts and aggregator
                let aggregator = self.aggregator.as_ref().ok_or_else(|| {
                    BattalionError::ValidationError(
                        "Conclave strategy requires an aggregator Paladin".to_string(),
                    )
                })?;

                // All paladins become experts
                let experts = self.paladins.clone();

                if experts.len() < 2 {
                    return Err(BattalionError::ValidationError(
                        "Conclave requires at least 2 experts".to_string(),
                    ));
                }

                // Create ConclaveConfig from BattalionConfig (D-04)
                let conclave_config = self.conclave_config_from_battalion();

                // Create Conclave instance
                let conclave =
                    paladin_core::platform::container::battalion::conclave::Conclave::new(
                        experts,
                        aggregator.clone(),
                        conclave_config,
                    )?;

                // Execute Conclave
                let service = ConclaveExecutionService::new(Arc::clone(&self.paladin_port));
                let conclave_result = service.execute(&conclave, input).await?;

                // Convert ConclaveResult to BattalionResult
                let total_experts = conclave.expert_count();
                let successful_experts = conclave_result.successful_expert_count();
                let failed_experts = total_experts.saturating_sub(successful_experts);

                BattalionResult {
                    battalion_id: Uuid::new_v4(),
                    battalion_name: self.config.name.clone(),
                    started_at,
                    completed_at: chrono::Utc::now(),
                    final_output: conclave_result.aggregated_output.output.clone(),
                    paladin_results: vec![], // Conclave handles this internally
                    status:
                        paladin_core::platform::container::battalion::BattalionStatus::Completed,
                    strategy_used: BattalionStrategy::Conclave,
                    strategy_selection_reasoning: None,
                    strategy_selection_time_ms: 0,
                    per_paladin_times: std::collections::HashMap::new(),
                    per_paladin_tokens: std::collections::HashMap::new(),
                    total_tokens: 0,
                    paladin_success_count: successful_experts,
                    paladin_failure_count: failed_experts,
                    node_errors: Vec::new(),
                }
            }
            BattalionStrategy::Council => {
                debug!("Delegating to CouncilExecutionService");

                // Validation: Council requires at least 2 Paladins for meaningful discussion
                if self.paladins.len() < 2 {
                    return Err(BattalionError::ValidationError(
                        "Council requires at least 2 Paladins for discussion".to_string(),
                    ));
                }

                // Build Council using builder pattern
                let mut council_builder =
                    paladin_core::platform::container::battalion::council::CouncilBuilder::new()
                        .name(self.config.name.clone())
                        .max_rounds(3); // Limit to 3 rounds for reasonable execution time

                // Add all Paladins as participants using their actual names as IDs
                for paladin in &self.paladins {
                    council_builder = council_builder.add_participant(paladin.data.name.clone());
                }

                let council = council_builder.build()?;

                // Create temporary registry from paladins for Council execution
                use crate::in_memory_registry::HashMapPaladinRegistry;
                use paladin_ports::output::paladin_registry::PaladinRegistry;
                let registry = HashMapPaladinRegistry::new();
                for paladin in &self.paladins {
                    // Use paladin name as ID
                    registry.register(paladin.data.name.clone(), Arc::new(paladin.clone()))?;
                }

                // Execute Council (pass None for garrison_port - Commander doesn't have one)
                let service = CouncilExecutionService::new(
                    Arc::clone(&self.paladin_port),
                    None,
                    Arc::new(registry),
                );
                let council_result = service.convene(&council, input).await?;

                // Convert council result to BattalionResult
                // Final output is the complete conversation history
                let final_output = council_result
                    .transcript
                    .iter()
                    .map(|msg| format!("{}: {}", msg.speaker, msg.content))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let total_participants = self.paladins.len();

                BattalionResult {
                    battalion_id: Uuid::new_v4(),
                    battalion_name: self.config.name.clone(),
                    started_at,
                    completed_at: chrono::Utc::now(),
                    final_output,
                    paladin_results: vec![], // Council handles this internally
                    status:
                        paladin_core::platform::container::battalion::BattalionStatus::Completed,
                    strategy_used: BattalionStrategy::Council,
                    strategy_selection_reasoning: None,
                    strategy_selection_time_ms: 0,
                    per_paladin_times: std::collections::HashMap::new(),
                    per_paladin_tokens: std::collections::HashMap::new(),
                    total_tokens: 0,
                    paladin_success_count: total_participants,
                    paladin_failure_count: 0,
                    node_errors: Vec::new(),
                }
            }
            BattalionStrategy::Grove => {
                debug!("Delegating to GroveExecutionService");

                // Validation: Grove requires at least 2 agents for routing
                if self.paladins.len() < 2 {
                    return Err(BattalionError::ValidationError(
                        "Grove requires at least 2 Paladins for routing".to_string(),
                    ));
                }

                // Create a temporary registry from self.paladins
                let registry = HashMapPaladinRegistry::new();

                // Build Grove instance using builder pattern
                // Create a Tree with all Paladins as agents
                let mut tree =
                    paladin_core::platform::container::battalion::grove::Tree::new("main");

                // Convert Paladins to TreeAgents using Paladin names as IDs
                for paladin in &self.paladins {
                    // Register paladin in registry
                    registry
                        .register(paladin.data.name.clone(), Arc::new(paladin.clone()))
                        .map_err(|e| {
                            BattalionError::ExecutionError(format!(
                                "Failed to register paladin '{}': {}",
                                paladin.data.name, e
                            ))
                        })?;

                    // Create TreeAgent with paladin ID matching registry
                    let tree_agent =
                        paladin_core::platform::container::battalion::grove::TreeAgent::new(
                            paladin.data.name.clone(),
                        );
                    tree = tree.add_agent(tree_agent);
                }

                let grove = paladin_core::platform::container::battalion::grove::GroveBuilder::new()
                    .name(self.config.name.clone())
                    .routing_strategy(
                        paladin_core::platform::container::battalion::grove::RoutingStrategy::KeywordMatch,
                    )
                    .add_tree(tree)
                    .build()?;

                // Execute Grove with registry (no longer passes paladins directly)
                let service = GroveExecutionService::new(
                    Arc::clone(&self.paladin_port),
                    None, // embedding_port
                    None, // llm_port
                    Arc::new(registry),
                );
                let grove_result = service.execute(&grove, input).await?;

                // Convert grove result to BattalionResult
                BattalionResult {
                    battalion_id: Uuid::new_v4(),
                    battalion_name: self.config.name.clone(),
                    started_at,
                    completed_at: chrono::Utc::now(),
                    final_output: grove_result.execution_result.clone(),
                    paladin_results: vec![], // Grove handles routing internally
                    status:
                        paladin_core::platform::container::battalion::BattalionStatus::Completed,
                    strategy_used: BattalionStrategy::Grove,
                    strategy_selection_reasoning: None,
                    strategy_selection_time_ms: 0,
                    per_paladin_times: std::collections::HashMap::new(),
                    per_paladin_tokens: std::collections::HashMap::new(),
                    total_tokens: 0,
                    paladin_success_count: 1,
                    paladin_failure_count: 0,
                    node_errors: Vec::new(),
                }
            }
            BattalionStrategy::Maneuver => {
                debug!("Delegating to ManeuverExecutionService");

                // Validation: Maneuver requires at least 1 Paladin
                if self.paladins.is_empty() {
                    return Err(BattalionError::ValidationError(
                        "Maneuver requires at least 1 Paladin".to_string(),
                    ));
                }

                // Use flow expression from Commander (set via builder)
                // If not set, default to sequential flow for backwards compatibility
                let flow_expr = self.flow_expression.as_deref().unwrap_or_else(|| {
                    // Generate default sequential flow
                    if self.paladins.len() == 1 {
                        self.paladins[0].name.as_deref().unwrap_or("agent0")
                    } else {
                        // This fallback is not ideal but maintains backwards compatibility
                        // In practice, flow_expression should always be set via builder
                        debug!(
                            "Warning: No flow expression set, generating default sequential flow"
                        );
                        "" // Will be handled below
                    }
                });

                // If empty flow_expr from fallback, generate sequential
                let flow_expr = if flow_expr.is_empty() {
                    self.paladins
                        .iter()
                        .enumerate()
                        .map(|(i, p)| p.name.as_ref().unwrap_or(&format!("agent{}", i)).clone())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                } else {
                    flow_expr.to_string()
                };

                // Parse the flow expression
                let flow = crate::maneuver::parser::FlowParser::parse(&flow_expr).map_err(|e| {
                    BattalionError::ValidationError(format!("Flow parse error: {}", e))
                })?;

                // Build agent name -> Paladin mapping
                let mut agents = std::collections::HashMap::new();
                for (i, paladin) in self.paladins.iter().enumerate() {
                    let agent_name = paladin
                        .name
                        .as_ref()
                        .unwrap_or(&format!("agent{}", i))
                        .clone();
                    agents.insert(agent_name, paladin.clone());
                }

                // On the aegis-derived path Maneuver reads only `aegis.timeout.run_timeout`; a
                // set `on_error` or `retry` is dropped, so say so once per execution (the
                // Campaign message's shape). An explicit `ManeuverConfig` reads no `aegis`.
                let ignored = self.maneuver_warned_aegis_policies();
                if !ignored.is_empty() {
                    warn!(
                        "Maneuver {} ignores {}: Maneuver reads only aegis.timeout.run_timeout, \
                         as one whole-flow clock",
                        self.config.name,
                        ignored.join(" and ")
                    );
                }

                // Use ManeuverConfig from Commander if set, otherwise create from BattalionConfig
                let maneuver_config = self
                    .maneuver_config
                    .clone()
                    .unwrap_or_else(|| self.default_maneuver_config());

                // Create Maneuver instance
                let maneuver = crate::maneuver::Maneuver::new(
                    &self.config.name,
                    agents,
                    flow,
                    maneuver_config,
                )
                .map_err(|e| {
                    BattalionError::ValidationError(format!("Maneuver creation failed: {}", e))
                })?;

                // Execute Maneuver
                let service = ManeuverExecutionService::new(Arc::clone(&self.paladin_port));
                let maneuver_result = service.execute(&maneuver, input).await.map_err(|e| {
                    BattalionError::ExecutionError(format!("Maneuver execution failed: {}", e))
                })?;

                // Convert ManeuverResult to BattalionResult
                let successful_agents = maneuver_result.execution_order.len();

                // Convert timing metrics to HashMap<String, u64> keyed by Paladin name
                let per_paladin_times: std::collections::HashMap<String, u64> = maneuver_result
                    .timing_metrics
                    .as_ref()
                    .map(|metrics| {
                        metrics
                            .iter()
                            .map(|(name, d)| (name.clone(), d.as_millis() as u64))
                            .collect()
                    })
                    .unwrap_or_default();

                BattalionResult {
                    battalion_id: Uuid::new_v4(),
                    battalion_name: self.config.name.clone(),
                    started_at,
                    completed_at: chrono::Utc::now(),
                    final_output: maneuver_result.final_output.clone(),
                    paladin_results: vec![], // Maneuver handles this internally
                    status: match maneuver_result.status {
                        crate::maneuver::ExecutionStatus::Success => {
                            paladin_core::platform::container::battalion::BattalionStatus::Completed
                        }
                        crate::maneuver::ExecutionStatus::PartialSuccess => {
                            paladin_core::platform::container::battalion::BattalionStatus::Completed
                        }
                        crate::maneuver::ExecutionStatus::Failed => {
                            paladin_core::platform::container::battalion::BattalionStatus::Failed
                        }
                    },
                    strategy_used: BattalionStrategy::Maneuver,
                    strategy_selection_reasoning: None,
                    strategy_selection_time_ms: 0,
                    per_paladin_times,
                    per_paladin_tokens: std::collections::HashMap::new(),
                    total_tokens: 0,
                    paladin_success_count: successful_agents,
                    paladin_failure_count: 0,
                    node_errors: Vec::new(),
                }
            }
            BattalionStrategy::Auto => {
                // This should never happen as Auto is resolved above
                return Err(BattalionError::StrategySelection(
                    "Auto strategy was not resolved".to_string(),
                ));
            }
        };

        // Enrich result with Commander-specific metadata
        result.strategy_used = effective_strategy.clone();
        result.strategy_selection_reasoning = selection_reason.clone();
        result.strategy_selection_time_ms = selection_time_ms;

        let total_time_ms = start_time.elapsed().as_millis() as u64;
        info!(
            "Commander {} completed in {}ms (selection: {}ms, execution: {}ms)",
            self.id,
            total_time_ms,
            selection_time_ms,
            total_time_ms - selection_time_ms
        );

        if let Some(reason) = selection_reason {
            debug!("Auto-selection reasoning: {}", reason);
        }

        // Export metadata to file if configured
        self.export_metadata(&result);

        Ok(result)
    }

    /// Export execution metadata to a JSON file, if `metadata_output_dir` is configured.
    ///
    /// This method is non-fatal: errors are logged as warnings but do not cause
    /// the overall execution to fail.
    ///
    /// # File naming convention
    ///
    /// `{strategy}_{timestamp}_{uuid_short}.json`
    ///
    /// For example: `formation_20250715_143022_a1b2c3d4.json`
    fn export_metadata(&self, result: &BattalionResult) {
        let Some(dir) = &self.config.metadata_output_dir else {
            return;
        };

        let strategy_name = format!("{:?}", result.strategy_used).to_lowercase();
        let timestamp = result.started_at.format("%Y%m%d_%H%M%S");
        let uuid_short = &result.battalion_id.to_string()[..8];
        let filename = format!("{}_{timestamp}_{uuid_short}.json", strategy_name);
        let path = dir.join(&filename);

        // Ensure directory exists
        if let Err(e) = std::fs::create_dir_all(dir) {
            warn!(
                "Failed to create metadata output directory '{}': {}",
                dir.display(),
                e
            );
            return;
        }

        match serde_json::to_string_pretty(result) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, &json) {
                    warn!("Failed to write metadata to '{}': {}", path.display(), e);
                } else {
                    info!("Metadata exported to {}", path.display());
                }
            }
            Err(e) => {
                warn!("Failed to serialize metadata: {}", e);
            }
        }
    }

    /// Resolve `BattalionStrategy::Auto` per the configured
    /// [`StrategySelection`] (CF-05, D-25): today's heuristic, or -- if
    /// `Semantic` is configured -- a model-named strategy with a
    /// deterministic fallback to the heuristic on any failure.
    async fn select_strategy(&self, input: &str) -> (BattalionStrategy, String) {
        match &self.strategy_selection {
            StrategySelection::Heuristic => self.analyze_and_select(input),
            StrategySelection::Semantic { llm, model } => {
                self.select_strategy_semantic(llm, model, input).await
            }
        }
    }

    /// The catalog of strategies [`StrategySelection::Semantic`] may name --
    /// exactly the strategies [`Commander::analyze_and_select`] can return.
    /// `Maneuver` is deliberately excluded: it is explicit-only, never
    /// selected by Auto mode (see `analyze_and_select`'s own note).
    const SEMANTIC_STRATEGY_CATALOG: &'static [&'static str] = &[
        "Formation",
        "Phalanx",
        "Campaign",
        "ChainOfCommand",
        "Conclave",
        "Council",
        "Grove",
    ];

    /// Match a model's answer against [`Self::SEMANTIC_STRATEGY_CATALOG`],
    /// exact-after-trim, case-insensitive (D-25).
    fn strategy_from_name(name: &str) -> Option<BattalionStrategy> {
        let trimmed = name.trim();
        Self::SEMANTIC_STRATEGY_CATALOG
            .iter()
            .find(|catalog_name| catalog_name.eq_ignore_ascii_case(trimmed))
            .map(|catalog_name| match *catalog_name {
                "Formation" => BattalionStrategy::Formation,
                "Phalanx" => BattalionStrategy::Phalanx,
                "Campaign" => BattalionStrategy::Campaign,
                "ChainOfCommand" => BattalionStrategy::ChainOfCommand,
                "Conclave" => BattalionStrategy::Conclave,
                "Council" => BattalionStrategy::Council,
                "Grove" => BattalionStrategy::Grove,
                _ => unreachable!("catalog name not covered by this match"),
            })
    }

    /// Prompt `llm` with the strategy catalog and `input`, parse the answer
    /// as a strategy name. Any [`paladin_ports::output::llm_port::LlmError`]
    /// or an answer naming no catalog strategy falls back to
    /// [`Self::analyze_and_select`] deterministically -- the fallback and its
    /// cause class are recorded in the returned reasoning string, never the
    /// model's raw answer or a provider's raw error text (the privacy
    /// prohibition: this reasoning string is echoed back to callers via
    /// `BattalionResult::strategy_selection_reasoning`).
    async fn select_strategy_semantic(
        &self,
        llm: &Arc<dyn LlmPort>,
        model: &str,
        input: &str,
    ) -> (BattalionStrategy, String) {
        let catalog = Self::SEMANTIC_STRATEGY_CATALOG.join(", ");
        let prompt_text = format!(
            "Choose exactly one Battalion orchestration strategy for the following task, from \
             this catalog: {catalog}. Reply with only the strategy name, nothing else.\n\nTask: {input}"
        );

        let prompt = match PromptItem::new(PromptType::User(UserPrompt {
            query: prompt_text,
            context: None,
        })) {
            Ok(p) => p,
            Err(_) => return self.fall_back_to_heuristic(input, "prompt construction failed"),
        };

        let request = LlmRequest::new(model.to_string(), prompt);

        let response = match llm.generate(request).await {
            Ok(response) => response,
            Err(e) => return self.fall_back_to_heuristic(input, llm_error_class(&e)),
        };

        match Self::strategy_from_name(&response.content) {
            Some(strategy) => {
                let reason = format!(
                    "Semantic strategy selection: model chose {strategy:?} from the strategy catalog"
                );
                (strategy, reason)
            }
            None => self.fall_back_to_heuristic(input, "model answer matched no catalog strategy"),
        }
    }

    /// Fall back to [`Self::analyze_and_select`], recording both the fact of
    /// the fallback and `cause` (a short, fixed class -- never the model's
    /// raw answer or a provider's raw error text) in the reasoning string.
    fn fall_back_to_heuristic(&self, input: &str, cause: &str) -> (BattalionStrategy, String) {
        let (strategy, heuristic_reason) = self.analyze_and_select(input);
        (
            strategy,
            format!(
                "Semantic strategy selection fell back to the heuristic ({cause}): {heuristic_reason}"
            ),
        )
    }

    /// Analyze input and Paladins to select optimal strategy
    ///
    /// Applies rule-based heuristics to determine the best orchestration
    /// pattern based on input keywords and Paladin characteristics.
    ///
    /// # Arguments
    ///
    /// * `input` - The user's input query/task
    ///
    /// # Returns
    ///
    /// A tuple of (selected strategy, reasoning for the selection)
    ///
    /// # Strategy Selection Rules
    ///
    /// 1. **Conclave** - Mixture of Agents synthesis
    ///    - Keywords: "synthesize", "compare", "expert panel", "perspectives", "consensus", "combine"
    ///    - 3+ Paladins with diverse expertise
    ///
    /// # Strategy Selection Rules
    ///
    /// 1. **Conclave** - Mixture of Agents synthesis
    ///    - Keywords: "synthesize", "compare", "expert panel", "perspectives", "consensus", "combine"
    ///    - 3+ Paladins with diverse expertise
    ///
    /// 2. **Council** - Conversational multi-agent collaboration
    ///    - Keywords: "discuss", "debate", "collaborate", "consensus", "brainstorm", "dialogue"
    ///    - 2+ Paladins for turn-based discussion
    ///
    /// 3. **Grove** - Intelligent routing to specialists
    ///    - Keywords: "route", "best agent", "expertise", "most qualified", "match to"
    ///    - 2+ Paladins with specialized capabilities
    ///
    /// 4. **Formation** - Sequential execution
    ///    - Keywords: "sequential", "pipeline", "chain", "step by step", "one after", "in order"
    ///    - 1-3 Paladins (default for small teams)
    ///
    /// 5. **Phalanx** - Parallel execution
    ///    - Keywords: "parallel", "concurrent", "all at once", "simultaneously", "together"
    ///    - 4+ Paladins with similar capabilities
    ///
    /// 6. **Campaign** - Graph/DAG orchestration
    ///    - Keywords: "workflow", "graph", "conditional", "if-then", "depends on", "after"
    ///    - Complex multi-stage tasks
    ///
    /// 7. **ChainOfCommand** - Hierarchical delegation
    ///    - Keywords: "delegate", "hierarchy", "specialist", "expert", "coordinator", "manager"
    ///    - Tasks requiring specialized expertise
    ///
    /// # Default
    ///
    /// Falls back to Formation if no clear indicators are found.
    fn analyze_and_select(&self, input: &str) -> (BattalionStrategy, String) {
        let input_lower = input.to_lowercase();

        // Check for Conclave indicators (synthesis/multi-perspective analysis)
        // Check this FIRST as it's most specific and should take precedence
        let conclave_keywords = [
            "synthesize",
            "synthesis",
            "compare",
            "expert panel",
            "perspectives",
            "consensus",
            "combine",
            "aggregate",
            "merge",
            "integrate views",
            "diverse opinions",
            "multiple experts",
            "comprehensive analysis",
        ];
        if conclave_keywords.iter().any(|kw| input_lower.contains(kw)) && self.paladins.len() >= 3 {
            return (
                BattalionStrategy::Conclave,
                format!(
                    "Input contains synthesis/multi-perspective keywords with {} Paladins, using Conclave for expert synthesis",
                    self.paladins.len()
                ),
            );
        }

        // Check for Council indicators (conversational collaboration)
        // Check this SECOND as it's also very specific
        let council_keywords = [
            "discuss",
            "discussion",
            "debate",
            "deliberate",
            "collaborate",
            "conversation",
            "dialogue",
            "consensus",
            "brainstorm",
            "round table",
            "panel discussion",
            "town hall",
            "collaborate on",
            "talk through",
        ];
        if council_keywords.iter().any(|kw| input_lower.contains(kw)) && self.paladins.len() >= 2 {
            return (
                BattalionStrategy::Council,
                format!(
                    "Input contains discussion/collaboration keywords with {} Paladins, using Council for turn-based dialogue",
                    self.paladins.len()
                ),
            );
        }

        // Check for Grove indicators (intelligent routing to specialists)
        // Check this THIRD before other routing patterns
        let grove_keywords = [
            "route",
            "routing",
            "best agent",
            "expertise",
            "expert for",
            "most qualified",
            "match to",
            "assign based on",
            "specialized in",
            "skilled in",
            "capability match",
            "dynamic routing",
            "intelligent assignment",
        ];
        if grove_keywords.iter().any(|kw| input_lower.contains(kw)) && self.paladins.len() >= 2 {
            return (
                BattalionStrategy::Grove,
                format!(
                    "Input contains routing/expertise keywords with {} Paladins, using Grove for intelligent agent selection",
                    self.paladins.len()
                ),
            );
        }

        // NOTE: Maneuver strategy is EXPLICIT-ONLY and NOT selected by Auto mode.
        // Flow DSL patterns like "->" and "|" are now checked AFTER Campaign to avoid
        // conflicting with natural language usage of arrows and pipes.
        // To use Maneuver, explicitly set BattalionStrategy::Maneuver via CommanderBuilder
        // and provide a flow expression using .flow() method.

        // Check for Campaign indicators (workflow/graph orchestration)
        // Only check if no flow syntax was found (since Campaign is conceptual, not syntax-based)
        let campaign_keywords = [
            "workflow",
            "graph",
            "conditional",
            "if-then", // Multi-word phrase checked as a whole
            "depends on",
            "after",
            "before",
            "when",
            "complex",
            "multi-stage",
        ];
        if campaign_keywords.iter().any(|kw| input_lower.contains(kw)) {
            return (
                BattalionStrategy::Campaign,
                format!(
                    "Input contains workflow/conditional keywords, using Campaign for {} Paladins",
                    self.paladins.len()
                ),
            );
        }

        // NOTE: Maneuver keyword detection removed - Maneuver is explicit-only.
        // Keywords like "flow", "branch", "nested" will now be handled by other strategies
        // (Campaign for workflow/branching, Formation for nested sequences).

        // Check for Formation indicators (sequential execution)
        let formation_keywords = [
            "sequential",
            "pipeline",
            "chain",
            "step by step",
            "one after",
            "in order",
            "first",
            "next",
        ];
        if formation_keywords.iter().any(|kw| input_lower.contains(kw)) {
            return (
                BattalionStrategy::Formation,
                format!(
                    "Input contains sequential keywords, using Formation for {} Paladins",
                    self.paladins.len()
                ),
            );
        }

        // Check for Phalanx indicators (parallel execution)
        let phalanx_keywords = [
            "parallel",
            "concurrent",
            "all at once",
            "simultaneously",
            "together",
            "at the same time",
            "in parallel",
        ];
        if phalanx_keywords.iter().any(|kw| input_lower.contains(kw)) {
            return (
                BattalionStrategy::Phalanx,
                format!(
                    "Input contains parallel keywords, using Phalanx for {} Paladins",
                    self.paladins.len()
                ),
            );
        }

        // Check for ChainOfCommand indicators (hierarchical delegation)
        let chain_keywords = [
            "delegate",
            "hierarchy",
            "specialist",
            "expert",
            "coordinator",
            "manager",
            "lead",
            "senior",
            "specialized",
        ];
        if chain_keywords.iter().any(|kw| input_lower.contains(kw)) {
            return (
                BattalionStrategy::ChainOfCommand,
                format!(
                    "Input contains delegation/hierarchy keywords, using ChainOfCommand for {} Paladins",
                    self.paladins.len()
                ),
            );
        }

        // Heuristics based on Paladin count (only if no keywords matched)
        match self.paladins.len() {
            1 => (
                BattalionStrategy::Formation,
                "Single Paladin detected, using Formation (sequential)".to_string(),
            ),
            2..=3 => (
                BattalionStrategy::Formation,
                format!(
                    "Small team ({} Paladins), using Formation (sequential)",
                    self.paladins.len()
                ),
            ),
            _ => {
                // Default fallback for larger teams
                (
                    BattalionStrategy::Formation,
                    format!(
                        "No clear strategy indicators, defaulting to Formation for {} Paladins",
                        self.paladins.len()
                    ),
                )
            }
        }
    }
}

/// Builder for creating Commander instances with validation
///
/// Provides a fluent interface for constructing Commanders with proper
/// validation of required fields.
///
/// # Example
///
/// ```ignore
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Formation)
///     .paladins(vec![paladin1, paladin2])
///     .config(config)
///     .build()?;
/// ```
///
/// # Builder Pattern
///
/// The CommanderBuilder follows the fluent builder pattern, allowing method chaining
/// for readable and flexible Commander construction.
///
/// ## Required Fields
///
/// - **paladin_port**: Must be provided to `new()` - adapter for executing Paladins
/// - **strategy**: Must be set via `strategy()` - the orchestration pattern to use
/// - **paladins**: Must be set via `paladins()` - the Paladins to orchestrate (cannot be empty)
///
/// ## Optional Fields
///
/// - **config**: Can be set via `config()` - if omitted, uses sensible defaults:
///   - Name: "default_commander_battalion"
///   - Aegis timeout: `run_timeout` of 300 seconds (5 minutes), applied **per attempt**
///   - No Aegis retry and no `on_error` handler (fail fast)
///
/// The Commander adds no bound of its own, so that 300 second default is not a general bound:
/// ChainOfCommand and Grove do not read `aegis`, so it does not bound them, and a Maneuver uses
/// `run_timeout` as one whole-flow clock instead of per attempt. That holds while the Maneuver
/// config is derived from `aegis` (including via `error_strategy`); a config supplied through
/// `maneuver_config` carries its own `timeout` and ignores `aegis` entirely.
///
/// # Validation
///
/// The `build()` method performs comprehensive validation:
///
/// - Ensures strategy is set
/// - Ensures at least one Paladin is provided
/// - Validates the config's Aegis policy through `BattalionConfig::validate_aegis`
///
/// # Examples
///
/// ## Minimal Configuration (with defaults)
///
/// ```ignore
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Formation)
///     .paladins(vec![paladin1, paladin2])
///     .build()?; // Uses default config
/// ```
///
/// ## Full Configuration
///
/// ```ignore
/// use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, TimeoutPolicy};
/// use paladin_core::platform::container::battalion::{BattalionConfig, BattalionStrategy};
/// use std::path::PathBuf;
/// use std::time::Duration;
///
/// let config = BattalionConfig::new("custom_battalion")
///     .with_description("Customer data processing pipeline")
///     .with_aegis(Aegis {
///         // 10 minutes per Paladin attempt
///         timeout: Some(TimeoutPolicy {
///             run_timeout: Some(Duration::from_secs(600)),
///             idle_timeout: None,
///         }),
///         // `max_attempts` counts total attempts, including the first.
///         retry: Some(RetryPolicy {
///             max_attempts: 5,
///             ..RetryPolicy::default()
///         }),
///         ..Aegis::default()
///     })
///     .with_metadata_dir(PathBuf::from("./checkpoints"));
///
/// let commander = CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Auto)
///     .paladins(paladins)
///     .config(config)
///     .build()?;
/// ```
///
/// ## Error Handling
///
/// ```ignore
/// use paladin_core::platform::container::battalion::BattalionError;
///
/// match CommanderBuilder::new(paladin_port)
///     .strategy(BattalionStrategy::Formation)
///     .paladins(vec![])
///     .build()
/// {
///     Ok(commander) => { /* use commander */ }
///     Err(BattalionError::CommanderValidation(msg)) => {
///         eprintln!("Validation failed: {}", msg);
///     }
///     Err(e) => eprintln!("Build error: {}", e),
/// }
/// ```
pub struct CommanderBuilder {
    strategy: Option<BattalionStrategy>,
    paladins: Option<Vec<Paladin>>,
    config: Option<BattalionConfig>,
    aggregator: Option<Paladin>,
    flow_expression: Option<String>,
    maneuver_config: Option<crate::maneuver::ManeuverConfig>,
    /// Error strategy to lay over the Maneuver config at `build()` (see `error_strategy`).
    error_strategy: Option<crate::maneuver::ErrorStrategy>,
    paladin_port: Arc<dyn PaladinPort>,
    strategy_selection: Option<StrategySelection>,
}

impl CommanderBuilder {
    /// Create a new CommanderBuilder
    ///
    /// # Arguments
    ///
    /// * `paladin_port` - Port for executing Paladins (required)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = CommanderBuilder::new(paladin_port);
    /// ```
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self {
        Self {
            strategy: None,
            paladins: None,
            config: None,
            aggregator: None,
            flow_expression: None,
            maneuver_config: None,
            error_strategy: None,
            paladin_port,
            strategy_selection: None,
        }
    }

    /// Configure how `BattalionStrategy::Auto` resolves (CF-05, D-25).
    ///
    /// Additive: omitting this call leaves [`StrategySelection::Heuristic`]
    /// in effect, today's unchanged keyword heuristic. `Commander::new`'s
    /// signature is unaffected -- this is the only way to reach
    /// [`StrategySelection::Semantic`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.strategy_selection(StrategySelection::Semantic { llm, model })
    /// ```
    pub fn strategy_selection(mut self, selection: StrategySelection) -> Self {
        self.strategy_selection = Some(selection);
        self
    }

    /// Set the orchestration strategy
    ///
    /// # Arguments
    ///
    /// * `strategy` - BattalionStrategy to use
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.strategy(BattalionStrategy::Auto)
    /// ```
    pub fn strategy(mut self, strategy: BattalionStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Set the Paladins to orchestrate
    ///
    /// # Arguments
    ///
    /// * `paladins` - Vector of Paladin instances
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.paladins(vec![paladin1, paladin2, paladin3])
    /// ```
    pub fn paladins(mut self, paladins: Vec<Paladin>) -> Self {
        self.paladins = Some(paladins);
        self
    }

    /// Set the Battalion configuration
    ///
    /// # Arguments
    ///
    /// * `config` - BattalionConfig instance
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.config(config)
    /// ```
    pub fn config(mut self, config: BattalionConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the aggregator Paladin for Conclave strategy
    ///
    /// The aggregator is responsible for synthesizing expert outputs in Conclave.
    /// If not set and using Conclave strategy, the last Paladin in the list will
    /// be used as the aggregator.
    ///
    /// # Arguments
    ///
    /// * `paladin` - Paladin to use as aggregator
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.aggregator(synthesis_paladin)
    /// ```
    pub fn aggregator(mut self, paladin: Paladin) -> Self {
        self.aggregator = Some(paladin);
        self
    }

    /// Set the flow expression for Maneuver strategy
    ///
    /// The flow expression defines the execution pattern using Flow DSL syntax:
    /// - `agent1 -> agent2` - Sequential execution
    /// - `agent1, agent2` - Parallel execution
    /// - `(agent1 -> agent2), agent3` - Nested patterns
    ///
    /// Required when using BattalionStrategy::Maneuver.
    ///
    /// # Arguments
    ///
    /// * `expression` - Flow DSL expression string
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.flow("analyzer -> enhancer -> (reviewer, validator)")
    /// ```
    pub fn flow(mut self, expression: impl Into<String>) -> Self {
        self.flow_expression = Some(expression.into());
        self
    }

    /// Set the error strategy for Maneuver execution
    ///
    /// Configures how errors should be handled during Maneuver execution. Only the error
    /// strategy changes: when no explicit [`maneuver_config`](Self::maneuver_config) is set, the
    /// rest of the Maneuver config stays the one derived from `aegis` at `build()`, so a
    /// configured `aegis.timeout.run_timeout` still bounds the Maneuver (and an unset one stays
    /// unbounded). When an explicit `maneuver_config` is set, the strategy is laid over it.
    /// Calling `maneuver_config` after this method replaces the whole config, strategy included.
    ///
    /// # Arguments
    ///
    /// * `strategy` - ManeuverErrorStrategy to use
    ///
    /// # Example
    ///
    /// ```ignore
    /// use paladin_core::platform::container::battalion::maneuver::ErrorStrategy;
    /// builder.error_strategy(ErrorStrategy::ContinueParallel)
    /// ```
    pub fn error_strategy(mut self, strategy: crate::maneuver::ErrorStrategy) -> Self {
        self.error_strategy = Some(strategy);
        self
    }

    /// Set the complete Maneuver configuration
    ///
    /// Provides fine-grained control over Maneuver execution behavior including
    /// error handling, output formatting, timing metrics, and timeouts.
    ///
    /// The config replaces the aegis-derived Maneuver config outright, `timeout` included
    /// (`ManeuverConfig::default()` carries a 300 second timeout, whatever `aegis.timeout`
    /// says), and the Maneuver then reads no `aegis` field. Set `timeout` explicitly. A
    /// `error_strategy` call made before this one is discarded; one made after it is laid
    /// over this config.
    ///
    /// # Arguments
    ///
    /// * `config` - ManeuverConfig instance
    ///
    /// # Example
    ///
    /// ```ignore
    /// use paladin_core::platform::container::battalion::maneuver::ManeuverConfig;
    /// let config = ManeuverConfig::default()
    ///     .with_timeout(Duration::from_secs(60))
    ///     .with_timing_metrics(true);
    /// builder.maneuver_config(config)
    /// ```
    pub fn maneuver_config(mut self, config: crate::maneuver::ManeuverConfig) -> Self {
        self.maneuver_config = Some(config);
        // An earlier `error_strategy` must not override the config just supplied.
        self.error_strategy = None;
        self
    }

    /// Build the Commander instance with validation
    ///
    /// Validates that all required fields are present and returns a configured
    /// Commander ready for execution.
    ///
    /// If no config is provided, generates a default configuration with:
    /// - Name: "default_commander_battalion"
    /// - `aegis.timeout`: `run_timeout` of 300 seconds, applied **per attempt**
    /// - `aegis.retry` and `aegis.on_error`: none (fail fast)
    ///
    /// The Commander adds no bound of its own: ChainOfCommand and Grove do not read `aegis`, so
    /// the 300 second default does not bound them, and a Maneuver uses `run_timeout` as one
    /// whole-flow clock rather than per attempt (unless an explicit `maneuver_config` was set,
    /// which carries its own `timeout`).
    ///
    /// # Returns
    ///
    /// * `Ok(Commander)` - Successfully built Commander
    /// * `Err(BattalionError::CommanderValidation)` - If validation fails
    ///
    /// # Errors
    ///
    /// Returns `CommanderValidation` error if:
    /// - Strategy is not set
    /// - Paladins vector is not set or is empty
    /// - The config's Aegis policy is invalid (`BattalionConfig::validate_aegis`, for example a
    ///   zero `run_timeout`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let commander = builder.build()?;
    /// ```
    pub fn build(self) -> Result<Commander, BattalionError> {
        let strategy = self.strategy.ok_or_else(|| {
            BattalionError::CommanderValidation("Strategy is required".to_string())
        })?;

        let paladins = self.paladins.ok_or_else(|| {
            BattalionError::CommanderValidation("Paladins are required".to_string())
        })?;

        if paladins.is_empty() {
            return Err(BattalionError::CommanderValidation(
                "At least one Paladin is required".to_string(),
            ));
        }

        // Validate Conclave strategy requirements and handle aggregator
        let aggregator = if strategy == BattalionStrategy::Conclave {
            if paladins.len() < 2 {
                return Err(BattalionError::CommanderValidation(
                    "Conclave requires at least 2 Paladins (for experts)".to_string(),
                ));
            }
            // If no aggregator specified, use the last Paladin as aggregator
            let agg = self.aggregator.unwrap_or_else(|| {
                debug!("No aggregator specified for Conclave, using last Paladin as aggregator");
                paladins.last().cloned().unwrap()
            });
            Some(agg)
        } else {
            self.aggregator // Keep aggregator if explicitly set for other strategies
        };

        // Validate Maneuver strategy requirements
        if strategy == BattalionStrategy::Maneuver {
            if self.flow_expression.is_none() {
                return Err(BattalionError::CommanderValidation(
                    "Maneuver strategy requires a flow expression. Use .flow() to set it."
                        .to_string(),
                ));
            }

            // Validate flow expression can be parsed
            let flow_expr = self.flow_expression.as_ref().unwrap();
            crate::maneuver::parser::FlowParser::parse(flow_expr).map_err(|e| {
                BattalionError::CommanderValidation(format!("Invalid flow expression: {}", e))
            })?;

            // Validate all agents referenced in flow exist in paladins
            // This will be done at execution time since we need the parsed expression
        }

        // Generate default config if none provided
        let config = self.config.unwrap_or_else(|| {
            debug!("No config provided, generating default configuration");
            // A bare builder keeps a documented 300 s bound -- per attempt, not per run.
            BattalionConfig::new("default_commander_battalion").with_aegis(Aegis {
                timeout: Some(TimeoutPolicy {
                    run_timeout: Some(Duration::from_secs(300)),
                    idle_timeout: None,
                }),
                ..Aegis::default()
            })
        });

        // Validate the Aegis policy (zero durations, zero max_attempts, unsupported handlers)
        config.validate_aegis().map_err(|e| match e {
            BattalionError::ValidationError(msg) => BattalionError::CommanderValidation(msg),
            other => other,
        })?;

        // Validate metadata output directory if configured
        config.validate_metadata_dir().map_err(|e| {
            BattalionError::CommanderValidation(format!("Metadata directory error: {}", e))
        })?;

        let mut commander =
            Commander::new(strategy, paladins, config, aggregator, self.paladin_port);

        // Set optional Maneuver fields
        commander.flow_expression = self.flow_expression;
        commander.maneuver_config = self.maneuver_config;
        if let Some(strategy) = self.error_strategy {
            match commander.maneuver_config.as_mut() {
                Some(explicit) => explicit.error_strategy = strategy,
                None => {
                    // Start from the aegis-derived config so the override does not swap a
                    // configured (or absent) `run_timeout` for `ManeuverConfig::default()`'s.
                    let mut derived = commander.default_maneuver_config();
                    derived.error_strategy = strategy;
                    commander.maneuver_config = Some(derived);
                    commander.maneuver_config_from_aegis = true;
                }
            }
        }
        if let Some(selection) = self.strategy_selection {
            commander.strategy_selection = selection;
        }

        Ok(commander)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use paladin_core::base::entity::Entity;
    use paladin_core::platform::container::aegis::{
        Aegis, ErrorHandlerSpec, RetryPolicy as AegisRetryPolicy, RetryPredicate,
    };
    use paladin_core::platform::container::battalion::{BattalionStatus, TokenUsage};
    use paladin_core::platform::container::battlefield::StateDelta;
    use paladin_core::platform::container::node_error::{NodeErrorSource, TimeoutKind};
    use paladin_core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_ports::output::llm_port::LlmError;
    use paladin_ports::output::paladin_port::{PaladinResult, PaladinStream, StopReason};
    use std::collections::HashMap;

    /// Mock PaladinPort for testing
    struct MockPaladinPort;

    #[async_trait]
    impl PaladinPort for MockPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            Ok(PaladinResult {
                output: "test output".to_string(),
                usage: TokenUsage::new(100, 0),
                execution_time_ms: 100,
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
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// Mock PaladinPort for ChainOfCommand testing  
    /// Returns proper SELECT/REASON format when executed on commander
    struct MockChainOfCommandPort;

    #[async_trait]
    impl PaladinPort for MockChainOfCommandPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            // Commander returns specialist selection, others return normal output
            let output = if paladin.data.name == "Commander" {
                "SELECT: Specialist_1, Specialist_2\nREASON: Both specialists are needed for this task".to_string()
            } else {
                format!("{} completed the task", paladin.data.name)
            };

            Ok(PaladinResult {
                output,
                usage: TokenUsage::new(100, 0),
                execution_time_ms: 100,
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
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    fn create_test_paladin() -> Paladin {
        let data = PaladinData {
            system_prompt: "Test prompt".to_string(),
            name: "TestPaladin".to_string(),
            user_name: "User".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(3),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        Entity::new(data, Some("TestPaladin".to_string()))
    }

    fn create_test_paladin_with_name(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("{} prompt", name),
            name: name.to_string(),
            user_name: "User".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(3),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };
        Entity::new(data, Some(name.to_string()))
    }

    fn create_test_config() -> BattalionConfig {
        BattalionConfig::new("TestBattalion")
    }

    /// An `Aegis` carrying only a per-attempt `run_timeout`.
    fn run_timeout_aegis(run_timeout: Duration) -> Aegis {
        Aegis {
            timeout: Some(TimeoutPolicy {
                run_timeout: Some(run_timeout),
                idle_timeout: None,
            }),
            ..Aegis::default()
        }
    }

    #[test]
    fn test_commander_builder_success() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin])
            .config(config)
            .build();

        assert!(commander.is_ok());
        let commander = commander.unwrap();
        assert_eq!(commander.strategy, BattalionStrategy::Formation);
        assert_eq!(commander.paladins.len(), 1);
        assert_eq!(commander.config.name, "TestBattalion");
    }

    #[test]
    fn test_commander_builder_missing_strategy() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        let result = CommanderBuilder::new(paladin_port)
            .paladins(vec![paladin])
            .config(config)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert_eq!(msg, "Strategy is required");
            }
            _ => panic!("Expected CommanderValidation error"),
        }
    }

    #[test]
    fn test_commander_builder_missing_paladins() {
        let paladin_port = Arc::new(MockPaladinPort);
        let config = create_test_config();

        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Phalanx)
            .config(config)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert_eq!(msg, "Paladins are required");
            }
            _ => panic!("Expected CommanderValidation error"),
        }
    }

    #[test]
    fn test_commander_builder_empty_paladins() {
        let paladin_port = Arc::new(MockPaladinPort);
        let config = create_test_config();

        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Campaign)
            .paladins(vec![])
            .config(config)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert_eq!(msg, "At least one Paladin is required");
            }
            _ => panic!("Expected CommanderValidation error"),
        }
    }

    #[test]
    fn commander_builder_rejects_zero_run_timeout() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();

        // Test with zero run_timeout (invalid)
        let invalid_config =
            BattalionConfig::new("test").with_aegis(run_timeout_aegis(Duration::ZERO));

        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin])
            .config(invalid_config)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert!(msg.contains("run_timeout"), "{msg}");
            }
            _ => panic!("Expected CommanderValidation error"),
        }
    }

    #[test]
    fn test_commander_all_strategies() {
        let strategies = vec![
            BattalionStrategy::Formation,
            BattalionStrategy::Phalanx,
            BattalionStrategy::Campaign,
            BattalionStrategy::ChainOfCommand,
            BattalionStrategy::Auto,
        ];

        for strategy in strategies {
            let paladin_port = Arc::new(MockPaladinPort);
            let paladin = create_test_paladin();
            let config = create_test_config();

            let commander = CommanderBuilder::new(paladin_port)
                .strategy(strategy.clone())
                .paladins(vec![paladin.clone()])
                .config(config.clone())
                .build();

            assert!(commander.is_ok());
            assert_eq!(commander.unwrap().strategy, strategy);
        }
    }

    #[test]
    fn test_auto_selects_formation_for_sequential_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Process this step by step");
        assert_eq!(strategy, BattalionStrategy::Formation);
        assert!(reason.contains("sequential"));

        let (strategy2, _) = commander.analyze_and_select("Run these in a pipeline");
        assert_eq!(strategy2, BattalionStrategy::Formation);

        let (strategy3, _) = commander.analyze_and_select("Chain these together");
        assert_eq!(strategy3, BattalionStrategy::Formation);
    }

    #[test]
    fn test_auto_selects_phalanx_for_parallel_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 4];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Run these in parallel");
        assert_eq!(strategy, BattalionStrategy::Phalanx);
        assert!(reason.contains("parallel"));

        let (strategy2, _) = commander.analyze_and_select("Execute all at once");
        assert_eq!(strategy2, BattalionStrategy::Phalanx);

        let (strategy3, _) = commander.analyze_and_select("Process simultaneously");
        assert_eq!(strategy3, BattalionStrategy::Phalanx);
    }

    #[test]
    fn test_auto_selects_campaign_for_workflow_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Build a workflow for this task");
        assert_eq!(strategy, BattalionStrategy::Campaign);
        assert!(reason.contains("workflow"));

        let (strategy2, _) = commander.analyze_and_select("If-then conditional logic");
        assert_eq!(strategy2, BattalionStrategy::Campaign);

        let (strategy3, _) = commander.analyze_and_select("This is a complex multi-stage process");
        assert_eq!(strategy3, BattalionStrategy::Campaign);
    }

    #[test]
    fn test_auto_selects_chain_for_delegate_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Delegate to specialist");
        assert_eq!(strategy, BattalionStrategy::ChainOfCommand);
        assert!(reason.contains("delegation"));

        let (strategy2, _) = commander.analyze_and_select("Use a hierarchy of experts");
        assert_eq!(strategy2, BattalionStrategy::ChainOfCommand);

        let (strategy3, _) = commander.analyze_and_select("Coordinator should manage this");
        assert_eq!(strategy3, BattalionStrategy::ChainOfCommand);
    }

    #[test]
    fn test_auto_selects_formation_for_single_paladin() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Do something");
        assert_eq!(strategy, BattalionStrategy::Formation);
        assert!(reason.contains("Single Paladin"));
    }

    #[test]
    fn test_auto_defaults_to_formation_when_uncertain() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 5];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Analyze this data");
        assert_eq!(strategy, BattalionStrategy::Formation);
        assert!(reason.contains("defaulting"));
    }

    // -----------------------------------------------------------------
    // StrategySelection::Semantic (CF-05, D-25)
    // -----------------------------------------------------------------

    #[tokio::test]
    async fn default_strategy_selection_is_heuristic_and_unchanged() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let input = "Process this step by step";
        let (heuristic_strategy, heuristic_reason) = commander.analyze_and_select(input);
        let (selected_strategy, selected_reason) = commander.select_strategy(input).await;

        assert_eq!(selected_strategy, heuristic_strategy);
        assert_eq!(selected_reason, heuristic_reason);
    }

    #[tokio::test]
    async fn semantic_mode_selects_the_strategy_the_model_names() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = create_test_config();
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("Phalanx"));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .strategy_selection(StrategySelection::Semantic {
                llm,
                model: "mock-model".to_string(),
            })
            .build()
            .unwrap();

        let (strategy, reason) = commander.select_strategy("irrelevant input").await;
        assert_eq!(strategy, BattalionStrategy::Phalanx);
        assert!(reason.contains("Phalanx"));
    }

    #[tokio::test]
    async fn semantic_matching_is_exact_after_trim_and_case_insensitive() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = create_test_config();
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("  Formation \n"));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .strategy_selection(StrategySelection::Semantic {
                llm,
                model: "mock-model".to_string(),
            })
            .build()
            .unwrap();

        let (strategy, _) = commander.select_strategy("irrelevant input").await;
        assert_eq!(strategy, BattalionStrategy::Formation);
    }

    #[tokio::test]
    async fn semantic_falls_back_to_heuristic_on_llm_error() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = create_test_config();
        let llm: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_error(LlmError::rate_limited(None)));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .strategy_selection(StrategySelection::Semantic {
                llm,
                model: "mock-model".to_string(),
            })
            .build()
            .unwrap();

        let input = "Process this step by step";
        let (heuristic_strategy, _) = commander.analyze_and_select(input);
        let (strategy, reason) = commander.select_strategy(input).await;

        assert_eq!(strategy, heuristic_strategy);
        assert!(reason.contains("fell back"));
        assert!(reason.contains("rate limit exceeded"));
    }

    #[tokio::test]
    async fn semantic_falls_back_to_heuristic_on_unrecognized_answer() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = create_test_config();
        let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_response("Battalion"));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .strategy_selection(StrategySelection::Semantic {
                llm,
                model: "mock-model".to_string(),
            })
            .build()
            .unwrap();

        let input = "Process this step by step";
        let (heuristic_strategy, _) = commander.analyze_and_select(input);
        let (strategy, reason) = commander.select_strategy(input).await;

        assert_eq!(strategy, heuristic_strategy);
        assert!(reason.contains("fell back"));
        assert!(!reason.contains("Battalion"));
    }

    #[test]
    fn test_auto_selection_is_case_insensitive() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 2];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy1, _) = commander.analyze_and_select("Run these in PARALLEL");
        assert_eq!(strategy1, BattalionStrategy::Phalanx);

        let (strategy2, _) = commander.analyze_and_select("Execute STEP BY STEP");
        assert_eq!(strategy2, BattalionStrategy::Formation);

        let (strategy3, _) = commander.analyze_and_select("Create a WORKFLOW");
        assert_eq!(strategy3, BattalionStrategy::Campaign);
    }

    #[test]
    fn test_auto_prioritizes_keywords_over_count() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        // Even with 1 Paladin, "parallel" keyword should select Phalanx
        let (strategy, _) = commander.analyze_and_select("Run this in parallel");
        assert_eq!(strategy, BattalionStrategy::Phalanx);
    }

    #[tokio::test]
    async fn test_execute_routes_to_phalanx_service() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Phalanx)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("Test input").await;
        assert!(result.is_ok(), "Phalanx execution should succeed");
    }

    #[tokio::test]
    async fn test_execute_routes_to_campaign_service() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![
            create_test_paladin_with_name("Agent_A"),
            create_test_paladin_with_name("Agent_B"),
            create_test_paladin_with_name("Agent_C"),
            create_test_paladin_with_name("Agent_D"),
        ];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Campaign)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("Test input").await;
        if let Err(ref e) = result {
            eprintln!("Campaign execution error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Campaign execution should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_execute_routes_to_chain_service() {
        let paladin_port = Arc::new(MockChainOfCommandPort);
        let paladins = vec![
            create_test_paladin_with_name("Commander"),
            create_test_paladin_with_name("Specialist_1"),
            create_test_paladin_with_name("Specialist_2"),
        ];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::ChainOfCommand)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("Test input").await;
        if let Err(ref e) = result {
            eprintln!("ChainOfCommand execution error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "ChainOfCommand execution should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_execute_resolves_auto_strategy() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 4];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        // Test with parallel keyword - should select Phalanx and execute
        let result = commander.execute("Run these in parallel").await;
        assert!(
            result.is_ok(),
            "Auto mode with parallel keyword should succeed"
        );

        // Test with sequential keyword - should select Formation and execute
        let result2 = commander.execute("Run these step by step").await;
        assert!(
            result2.is_ok(),
            "Auto mode with sequential keyword should succeed"
        );
    }

    #[tokio::test]
    async fn test_result_contains_strategy_used() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 2];
        let config = create_test_config();

        // Test explicit strategy
        let commander = CommanderBuilder::new(paladin_port.clone())
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins.clone())
            .config(config.clone())
            .build()
            .unwrap();

        let result = commander.execute("Test input").await.unwrap();
        assert_eq!(result.strategy_used, BattalionStrategy::Formation);

        // Test Auto mode resolves to actual strategy (not Auto)
        let commander_auto = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result_auto = commander_auto.execute("Test input").await.unwrap();
        assert_ne!(result_auto.strategy_used, BattalionStrategy::Auto);
        assert_eq!(result_auto.strategy_used, BattalionStrategy::Formation);
    }

    #[tokio::test]
    async fn test_result_contains_selection_reasoning() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        // Test explicit strategy - should have no reasoning
        let commander = CommanderBuilder::new(paladin_port.clone())
            .strategy(BattalionStrategy::Phalanx)
            .paladins(paladins.clone())
            .config(config.clone())
            .build()
            .unwrap();

        let result = commander.execute("Test input").await.unwrap();
        assert!(result.strategy_selection_reasoning.is_none());

        // Test Auto mode - should have reasoning
        let commander_auto = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result_auto = commander_auto
            .execute("Run these in parallel")
            .await
            .unwrap();
        assert!(result_auto.strategy_selection_reasoning.is_some());
        let reasoning = result_auto.strategy_selection_reasoning.unwrap();
        assert!(reasoning.contains("parallel") || reasoning.contains("Phalanx"));
    }

    #[tokio::test]
    async fn test_result_contains_telemetry_metadata() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 2];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("Test input").await.unwrap();

        // Verify all metadata fields are present
        // strategy_selection_time_ms is u64 so always >= 0
        assert!(!result.battalion_id.is_nil());
        assert!(!result.battalion_name.is_empty());
        assert_eq!(result.strategy_used, BattalionStrategy::Formation);
    }

    // Commander error-path tests (fail-fast, continue-on-error, retry-then-continue,
    // partial-results) now live in tests/integration/commander_error_paths_test.rs,
    // driven by the shared FaultyPaladinPort mock in tests/helpers/mock_paladin_port.rs.

    #[tokio::test]
    async fn test_config_passthrough_to_services() {
        let paladin_port = Arc::new(MockPaladinPort);

        // Create paladin for testing
        let paladin = create_test_paladin();

        // Create config with specific values to verify passthrough
        let aegis = Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..run_timeout_aegis(Duration::from_secs(600))
        };
        let config = BattalionConfig::new("test_battalion").with_aegis(aegis.clone());

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin])
            .config(config.clone())
            .build()
            .unwrap();

        // Verify config is properly stored
        assert_eq!(commander.config.name, "test_battalion");
        assert_eq!(commander.config.aegis, aegis);
    }

    /// A port whose every call takes `delay` before succeeding.
    struct SlowMockPaladinPort {
        delay: Duration,
    }

    #[async_trait]
    impl PaladinPort for SlowMockPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            tokio::time::sleep(self.delay).await;
            Ok(PaladinResult {
                output: "slow output".to_string(),
                usage: TokenUsage::new(100, 0),
                execution_time_ms: self.delay.as_millis() as u64,
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
            unimplemented!()
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// D-03: the Commander adds no clock of its own; the bound is Aegis's, per attempt.
    #[tokio::test(start_paused = true)]
    async fn commander_formation_timeout_is_per_attempt() {
        let paladin_port = Arc::new(SlowMockPaladinPort {
            delay: Duration::from_secs(2),
        });
        let config = BattalionConfig::new("timeout_test")
            .with_aegis(run_timeout_aegis(Duration::from_secs(1)));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![create_test_paladin(), create_test_paladin()])
            .config(config)
            .build()
            .unwrap();

        let started = tokio::time::Instant::now();
        let result = commander.execute("Test input").await;

        match result {
            Err(BattalionError::Node(node_error)) => {
                assert_eq!(
                    node_error.source,
                    NodeErrorSource::Timeout(TimeoutKind::Run)
                );
            }
            other => panic!("Expected a Node timeout error, got {:?}", other),
        }
        // The first Paladin's attempt is cut at 1 s and the run fails fast.
        assert_eq!(started.elapsed(), Duration::from_secs(1));
    }

    /// Each Paladin gets its own bound: two 0.8 s Paladins under a 1 s bound finish in 1.6 s.
    #[tokio::test(start_paused = true)]
    async fn commander_bound_is_not_a_whole_run_clock() {
        let paladin_port = Arc::new(SlowMockPaladinPort {
            delay: Duration::from_millis(800),
        });
        let config = BattalionConfig::new("per_attempt")
            .with_aegis(run_timeout_aegis(Duration::from_secs(1)));

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![create_test_paladin(), create_test_paladin()])
            .config(config)
            .build()
            .unwrap();

        let started = tokio::time::Instant::now();
        let result = commander.execute("Test input").await;

        assert!(result.is_ok(), "{:?}", result.err());
        assert_eq!(started.elapsed(), Duration::from_millis(1600));
    }

    #[tokio::test]
    async fn commander_builder_default_bounds_each_attempt() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();

        // Build Commander without providing config
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin])
            // Intentionally NOT calling .config()
            .build()
            .unwrap();

        // Verify default config was generated
        assert_eq!(commander.config.name, "default_commander_battalion");
        // A bare builder keeps a documented 300 s bound, applied per attempt (Open Question 1).
        let aegis = &commander.config.aegis;
        assert_eq!(
            aegis.timeout,
            Some(TimeoutPolicy {
                run_timeout: Some(Duration::from_secs(300)),
                idle_timeout: None,
            })
        );
        assert_eq!(aegis.retry, None);
        assert_eq!(aegis.on_error, None);
    }

    #[test]
    fn test_auto_selects_council_for_discussion_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Let's discuss this problem");
        assert_eq!(strategy, BattalionStrategy::Council);
        assert!(reason.contains("discussion") || reason.contains("Council"));

        let (strategy2, _) = commander.analyze_and_select("Debate the best approach");
        assert_eq!(strategy2, BattalionStrategy::Council);

        let (strategy3, _) = commander.analyze_and_select("Collaborate on a solution");
        assert_eq!(strategy3, BattalionStrategy::Council);

        let (strategy4, _) = commander.analyze_and_select("Have a dialogue about this");
        assert_eq!(strategy4, BattalionStrategy::Council);

        let (strategy5, _) = commander.analyze_and_select("Round table discussion needed");
        assert_eq!(strategy5, BattalionStrategy::Council);
    }

    #[test]
    fn test_auto_selects_grove_for_routing_keywords() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let (strategy, reason) = commander.analyze_and_select("Route this to the best agent");
        assert_eq!(strategy, BattalionStrategy::Grove);
        assert!(reason.contains("routing") || reason.contains("Grove"));

        let (strategy2, _) = commander.analyze_and_select("Find the expert for this task");
        assert_eq!(strategy2, BattalionStrategy::Grove);

        let (strategy3, _) = commander.analyze_and_select("Match to the most qualified agent");
        assert_eq!(strategy3, BattalionStrategy::Grove);

        let (strategy4, _) = commander.analyze_and_select("Who is skilled in this area?");
        assert_eq!(strategy4, BattalionStrategy::Grove);

        let (strategy5, _) = commander.analyze_and_select("Dynamic routing based on expertise");
        assert_eq!(strategy5, BattalionStrategy::Grove);
    }

    #[test]
    fn test_council_requires_multiple_paladins() {
        let paladin_port = Arc::new(MockPaladinPort);
        let single_paladin = vec![create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(single_paladin)
            .config(config)
            .build()
            .unwrap();

        // With only 1 Paladin, "discuss" keyword should NOT select Council
        // Should fall back to Formation instead
        let (strategy, _) = commander.analyze_and_select("Let's discuss this");
        assert_ne!(strategy, BattalionStrategy::Council);
        assert_eq!(strategy, BattalionStrategy::Formation);
    }

    #[test]
    fn test_grove_requires_multiple_paladins() {
        let paladin_port = Arc::new(MockPaladinPort);
        let single_paladin = vec![create_test_paladin()];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(single_paladin)
            .config(config)
            .build()
            .unwrap();

        // With only 1 Paladin, "route" keyword should NOT select Grove
        // Should fall back to Formation instead
        let (strategy, _) = commander.analyze_and_select("Route to the best agent");
        assert_ne!(strategy, BattalionStrategy::Grove);
        assert_eq!(strategy, BattalionStrategy::Formation);
    }

    #[test]
    fn test_council_and_grove_keywords_are_case_insensitive() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(); 3];
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        // Test Council with uppercase
        let (strategy1, _) = commander.analyze_and_select("Let's DISCUSS this");
        assert_eq!(strategy1, BattalionStrategy::Council);

        // Test Grove with uppercase
        let (strategy2, _) = commander.analyze_and_select("ROUTE to the best EXPERT");
        assert_eq!(strategy2, BattalionStrategy::Grove);

        // Test mixed case
        let (strategy3, _) = commander.analyze_and_select("Collaborate ON this problem");
        assert_eq!(strategy3, BattalionStrategy::Council);
    }

    #[tokio::test]
    async fn test_maneuver_strategy_explicit() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..3 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        // Test explicit Maneuver strategy with flow expression
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0 -> agent1 -> agent2")
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("Process this workflow").await.unwrap();

        // Verify strategy was Maneuver
        assert_eq!(result.strategy_used, BattalionStrategy::Maneuver);
        assert_eq!(
            result.status,
            paladin_core::platform::container::battalion::BattalionStatus::Completed
        );
        assert!(!result.final_output.is_empty());
    }

    // NOTE: These tests are REMOVED because Maneuver is now explicit-only per Task 4.4
    // Maneuver should NOT be selected by Auto mode. To use Maneuver, explicitly set
    // BattalionStrategy::Maneuver via CommanderBuilder.strategy() and provide flow expression.
    //
    // Previous behavior (now removed):
    // - Auto mode would select Maneuver for "flow", "branch", "nested" keywords
    // - Auto mode would select Maneuver for "->" or "|" patterns in input
    //
    // New behavior:
    // - Auto mode will NEVER select Maneuver
    // - Keywords like "flow" and "branch" now route to Campaign or other strategies
    // - Patterns like "->" in natural language don't trigger Maneuver

    #[test]
    fn test_maneuver_requires_at_least_one_paladin() {
        let paladin_port = Arc::new(MockPaladinPort);
        let config = create_test_config();

        // Without paladins, Maneuver strategy should return error
        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent1")
            .paladins(vec![]) // Empty paladins vector
            .config(config)
            .build();

        // Should fail during build validation
        assert!(result.is_err());
    }

    // Task 4.5: Commander integration tests for Maneuver strategy

    #[test]
    fn test_commander_builder_with_flow_expression() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..3 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        // Test CommanderBuilder with flow expression
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0 -> agent1 -> agent2")
            .paladins(paladins)
            .config(config)
            .build();

        assert!(commander.is_ok());
        let commander = commander.unwrap();
        assert_eq!(commander.strategy, BattalionStrategy::Maneuver);
        assert!(commander.flow_expression.is_some());
        assert_eq!(
            commander.flow_expression.unwrap(),
            "agent0 -> agent1 -> agent2"
        );
    }

    #[test]
    fn test_maneuver_without_flow_expression_fails() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        // Maneuver strategy requires flow expression
        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .paladins(vec![paladin])
            .config(config)
            // Intentionally NOT calling .flow()
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert!(msg.contains("flow expression"));
            }
            _ => panic!("Expected CommanderValidation error for missing flow"),
        }
    }

    #[test]
    fn test_maneuver_with_invalid_flow_expression_fails() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        // Invalid flow expression (empty parentheses)
        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent1 -> ()")
            .paladins(vec![paladin])
            .config(config)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::CommanderValidation(msg) => {
                assert!(msg.contains("Invalid flow expression"));
            }
            _ => panic!("Expected CommanderValidation error for invalid flow"),
        }
    }

    #[test]
    fn test_commander_builder_with_error_strategy() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        // Test setting error strategy via builder
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0")
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .paladins(vec![paladin])
            .config(config)
            .build();

        assert!(commander.is_ok());
        let commander = commander.unwrap();
        assert!(commander.maneuver_config.is_some());
        assert_eq!(
            commander.maneuver_config.unwrap().error_strategy,
            crate::maneuver::ErrorStrategy::ContinueParallel
        );
    }

    /// A Commander over `config` for bridge tests (the strategy is irrelevant to the helpers).
    fn commander_with(config: BattalionConfig) -> Commander {
        CommanderBuilder::new(Arc::new(MockPaladinPort))
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![create_test_paladin()])
            .config(config)
            .build()
            .unwrap()
    }

    fn retrying_config(max_attempts: u32) -> BattalionConfig {
        BattalionConfig::new("bridge").with_aegis(Aegis {
            retry: Some(AegisRetryPolicy {
                max_attempts,
                ..AegisRetryPolicy::default()
            }),
            ..Aegis::default()
        })
    }

    /// D-24 / WR-04: the decision behind the Commander's one-time Maneuver warning is a pure
    /// function (the repository does not capture log output), naming exactly the `aegis`
    /// fields Maneuver drops, in a fixed order, and never the timeout it honours.
    #[test]
    fn maneuver_ignored_aegis_policies_names_only_the_fields_maneuver_drops() {
        let absorb = || {
            Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            })
        };
        let retry = || {
            Some(AegisRetryPolicy {
                max_attempts: 3,
                ..AegisRetryPolicy::default()
            })
        };

        assert!(maneuver_ignored_aegis_policies(&Aegis::default()).is_empty());
        assert_eq!(
            maneuver_ignored_aegis_policies(&Aegis {
                on_error: absorb(),
                ..Aegis::default()
            }),
            vec!["aegis.on_error"]
        );
        assert_eq!(
            maneuver_ignored_aegis_policies(&Aegis {
                retry: retry(),
                ..Aegis::default()
            }),
            vec!["aegis.retry"]
        );
        assert_eq!(
            maneuver_ignored_aegis_policies(&Aegis {
                on_error: absorb(),
                retry: retry(),
                ..Aegis::default()
            }),
            vec!["aegis.on_error", "aegis.retry"]
        );
        // Maneuver honours `timeout.run_timeout`, so a timeout alone is not ignored.
        assert!(
            maneuver_ignored_aegis_policies(&run_timeout_aegis(Duration::from_secs(45))).is_empty()
        );
    }

    /// A builder over an `on_error`-absorbing config with a 45 second `run_timeout`, so the
    /// Maneuver warning has something to name and the timeout has a value to preserve.
    fn absorbing_builder() -> CommanderBuilder {
        let config = BattalionConfig::new("gate").with_aegis(Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..run_timeout_aegis(Duration::from_secs(45))
        });
        CommanderBuilder::new(Arc::new(MockPaladinPort))
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![create_test_paladin()])
            .config(config)
    }

    /// WR-01: the Maneuver warning fires only on the aegis-derived path. An explicit
    /// `ManeuverConfig` reads no `aegis`, so the message would be false for that run.
    #[test]
    fn maneuver_warning_fires_only_on_the_derived_config_path() {
        let derived = absorbing_builder().build().unwrap();
        assert!(derived.maneuver_config.is_none());
        assert_eq!(
            derived.maneuver_warned_aegis_policies(),
            vec!["aegis.on_error"]
        );

        let explicit = absorbing_builder()
            .maneuver_config(crate::maneuver::ManeuverConfig::default())
            .build()
            .unwrap();
        assert!(explicit.maneuver_warned_aegis_policies().is_empty());

        // `error_strategy` alone still follows `aegis.timeout.run_timeout`: still derived.
        let strategy_only = absorbing_builder()
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .build()
            .unwrap();
        assert_eq!(
            strategy_only.maneuver_warned_aegis_policies(),
            vec!["aegis.on_error"]
        );

        // Nothing ignored, nothing to warn about, on either path.
        let quiet = commander_with(BattalionConfig::new("quiet"));
        assert!(quiet.maneuver_warned_aegis_policies().is_empty());
    }

    /// WR-01: the documented M-B-11 remediation, `error_strategy(ContinueParallel)`, must not
    /// swap the aegis-derived timeout for `ManeuverConfig::default()`'s 300 seconds.
    #[test]
    fn error_strategy_keeps_the_aegis_derived_maneuver_timeout() {
        let configured = absorbing_builder()
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .build()
            .unwrap();
        let cfg = configured.maneuver_config.unwrap();
        assert_eq!(
            cfg.error_strategy,
            crate::maneuver::ErrorStrategy::ContinueParallel
        );
        assert_eq!(cfg.timeout, Some(Duration::from_secs(45)));

        // No `aegis` timeout: the Maneuver stays unbounded rather than gaining 300 seconds.
        let unbounded = CommanderBuilder::new(Arc::new(MockPaladinPort))
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![create_test_paladin()])
            .config(BattalionConfig::new("unbounded"))
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .build()
            .unwrap();
        assert_eq!(unbounded.maneuver_config.unwrap().timeout, None);
    }

    /// WR-01: `error_strategy` and `maneuver_config` compose in call order -- a later
    /// `error_strategy` is laid over the explicit config, a later `maneuver_config` wins
    /// outright (the pre-fix behaviour of both orders).
    #[test]
    fn error_strategy_and_maneuver_config_compose_in_call_order() {
        let explicit =
            || crate::maneuver::ManeuverConfig::default().with_timeout(Duration::from_secs(60));

        let strategy_last = absorbing_builder()
            .maneuver_config(explicit())
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .build()
            .unwrap();
        let cfg = strategy_last.maneuver_config.unwrap();
        assert_eq!(
            cfg.error_strategy,
            crate::maneuver::ErrorStrategy::ContinueParallel
        );
        assert_eq!(cfg.timeout, Some(Duration::from_secs(60)));

        let config_last = absorbing_builder()
            .error_strategy(crate::maneuver::ErrorStrategy::ContinueParallel)
            .maneuver_config(explicit())
            .build()
            .unwrap();
        let cfg = config_last.maneuver_config.unwrap();
        assert_eq!(cfg.error_strategy, crate::maneuver::ErrorStrategy::FailFast);
        assert_eq!(cfg.timeout, Some(Duration::from_secs(60)));
    }

    /// D-04: with no explicit `ManeuverConfig` the bridge fails fast whatever `on_error` says,
    /// and the Maneuver timeout is `aegis.timeout.run_timeout`.
    #[test]
    fn commander_maneuver_bridge_defaults_to_fail_fast() {
        let absorbing = BattalionConfig::new("bridge").with_aegis(Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..run_timeout_aegis(Duration::from_secs(45))
        });
        let bridged = commander_with(absorbing).default_maneuver_config();
        assert_eq!(
            bridged.error_strategy,
            crate::maneuver::ErrorStrategy::FailFast
        );
        assert_eq!(bridged.timeout, Some(Duration::from_secs(45)));

        let unbounded = commander_with(BattalionConfig::new("bridge")).default_maneuver_config();
        assert_eq!(
            unbounded.error_strategy,
            crate::maneuver::ErrorStrategy::FailFast
        );
        assert_eq!(unbounded.timeout, None);
    }

    /// D-04 as refined by ADR-0059 (e): `retry_attempts = max_attempts - 1`, clamped to 5; the
    /// `ConclaveConfig` default of 2 applies when there is no `aegis.retry`.
    #[test]
    fn commander_conclave_bridge_derives_retry_attempts() {
        let four = commander_with(retrying_config(4)).conclave_config_from_battalion();
        assert_eq!(four.retry_attempts, 3);

        let clamped = commander_with(retrying_config(9)).conclave_config_from_battalion();
        assert_eq!(clamped.retry_attempts, 5);

        let single = commander_with(retrying_config(1)).conclave_config_from_battalion();
        assert_eq!(single.retry_attempts, 0);

        let none = commander_with(BattalionConfig::new("bridge")).conclave_config_from_battalion();
        assert_eq!(none.retry_attempts, 2);
        assert_eq!(none.battalion_config.name, "bridge");
    }

    #[test]
    fn test_commander_builder_with_maneuver_config() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        // Test setting complete ManeuverConfig
        let maneuver_config = crate::maneuver::ManeuverConfig::default()
            .with_timeout(std::time::Duration::from_secs(60))
            .with_timing_metrics(false);

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0")
            .maneuver_config(maneuver_config.clone())
            .paladins(vec![paladin])
            .config(config)
            .build();

        assert!(commander.is_ok());
        let commander = commander.unwrap();
        assert!(commander.maneuver_config.is_some());
        let stored_config = commander.maneuver_config.unwrap();
        assert_eq!(
            stored_config.timeout,
            Some(std::time::Duration::from_secs(60))
        );
        assert!(!stored_config.collect_timing_metrics);
    }

    #[tokio::test]
    async fn test_maneuver_execution_through_commander() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..3 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        // Test execution through Commander with Maneuver
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0 -> agent1 -> agent2")
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;
        assert!(result.is_ok(), "Maneuver execution should succeed");

        let result = result.unwrap();
        assert_eq!(result.strategy_used, BattalionStrategy::Maneuver);
        assert!(!result.final_output.is_empty());
    }

    #[test]
    fn test_auto_strategy_does_not_select_maneuver() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..3 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Auto)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        // Test various inputs that should NOT select Maneuver
        // Maneuver is explicit-only per Task 4.4

        // Input with arrow (could be confused with flow DSL)
        let (strategy1, _) = commander.analyze_and_select("Process step1 -> step2 -> step3");
        assert_ne!(
            strategy1,
            BattalionStrategy::Maneuver,
            "Auto should not select Maneuver even with -> in input"
        );

        // Input with "flow" keyword
        let (strategy2, _) = commander.analyze_and_select("Create a flow for this task");
        assert_ne!(
            strategy2,
            BattalionStrategy::Maneuver,
            "Auto should not select Maneuver for 'flow' keyword"
        );

        // Input with "branch" keyword
        let (strategy3, _) = commander.analyze_and_select("Branch execution based on results");
        assert_ne!(
            strategy3,
            BattalionStrategy::Maneuver,
            "Auto should not select Maneuver for 'branch' keyword"
        );

        // Input with pipe character
        let (strategy4, _) = commander.analyze_and_select("Run agent1 | agent2 | agent3");
        assert_ne!(
            strategy4,
            BattalionStrategy::Maneuver,
            "Auto should not select Maneuver even with | in input"
        );
    }

    #[tokio::test]
    async fn test_maneuver_with_parallel_pattern() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..3 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        // Test with parallel pattern
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0, agent1, agent2")
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;
        assert!(
            result.is_ok(),
            "Maneuver with parallel pattern should succeed"
        );
    }

    #[tokio::test]
    async fn test_maneuver_with_nested_pattern() {
        let paladin_port = Arc::new(MockPaladinPort);
        let mut paladins = vec![];
        for i in 0..4 {
            let data = PaladinData {
                system_prompt: format!("Agent {}", i),
                name: format!("agent{}", i),
                user_name: "test".to_string(),
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_loops: MaxLoops::Fixed(3),
                stop_words: vec![],
                status: PaladinStatus::Idle,
                vision_enabled: false,
                ..Default::default()
            };
            paladins.push(Entity::new(data, Some(format!("agent{}", i))));
        }
        let config = create_test_config();

        // Test with nested pattern: agent0 -> (agent1 -> agent2)
        // This creates Sequential(agent0, Sequential(agent1, agent2))
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Maneuver)
            .flow("agent0 -> (agent1 -> agent2)")
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Maneuver with nested sequential pattern should succeed: {:?}",
            result.err()
        );
    }

    // ── Task 8.0: Commander metadata export configuration tests ──

    #[tokio::test]
    async fn test_commander_build_with_valid_metadata_dir() {
        let dir = std::env::temp_dir().join("paladin_cmd_meta_valid_8_0");
        let _ = std::fs::remove_dir_all(&dir);

        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = BattalionConfig::new("meta_test").with_metadata_dir(dir.clone());

        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build();

        assert!(
            result.is_ok(),
            "Build should succeed with valid metadata dir"
        );
        assert!(dir.exists(), "Metadata dir should be auto-created");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_commander_build_without_metadata_dir() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin()];
        let config = BattalionConfig::new("no_meta_test");

        let result = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build();

        assert!(result.is_ok(), "Build should succeed without metadata dir");
    }

    // ── Task 9.0: Commander metadata export logic tests ──

    #[tokio::test]
    async fn test_metadata_export_creates_file() {
        let dir = std::env::temp_dir().join("paladin_meta_export_9_0");
        let _ = std::fs::remove_dir_all(&dir);

        let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = BattalionConfig::new("export_test").with_metadata_dir(dir.clone());

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;
        assert!(result.is_ok(), "Execute should succeed");

        // Verify a JSON file was created in the metadata dir
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("metadata dir should exist")
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1, "Exactly one metadata file should exist");
        assert!(
            entries[0]
                .path()
                .extension()
                .is_some_and(|ext| ext == "json"),
            "File should have .json extension"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_metadata_export_correct_naming() {
        let dir = std::env::temp_dir().join("paladin_meta_naming_9_0");
        let _ = std::fs::remove_dir_all(&dir);

        let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = BattalionConfig::new("naming_test").with_metadata_dir(dir.clone());

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let _ = commander.execute("test input").await;

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("metadata dir should exist")
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let filename = entries[0].file_name().to_string_lossy().to_string();
        // Format: {strategy}_{timestamp}_{uuid_short}.json
        assert!(
            filename.starts_with("formation_"),
            "Filename should start with strategy name, got: {}",
            filename
        );
        assert!(
            filename.ends_with(".json"),
            "Filename should end with .json, got: {}",
            filename
        );
        // Check timestamp portion (YYYYMMDD_HHMMSS)
        let parts: Vec<&str> = filename.trim_end_matches(".json").splitn(3, '_').collect();
        assert!(
            parts.len() >= 3,
            "Filename should have strategy_date_time_uuid parts"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_metadata_export_json_structure() {
        let dir = std::env::temp_dir().join("paladin_meta_json_9_0");
        let _ = std::fs::remove_dir_all(&dir);

        let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = BattalionConfig::new("json_test").with_metadata_dir(dir.clone());

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let _ = commander.execute("test input").await;

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("metadata dir should exist")
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let content = std::fs::read_to_string(entries[0].path()).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("Metadata file should be valid JSON");

        // Verify key fields are present
        assert!(
            parsed.get("battalion_id").is_some(),
            "Should have battalion_id"
        );
        assert!(
            parsed.get("battalion_name").is_some(),
            "Should have battalion_name"
        );
        assert!(parsed.get("started_at").is_some(), "Should have started_at");
        assert!(
            parsed.get("completed_at").is_some(),
            "Should have completed_at"
        );
        assert!(
            parsed.get("final_output").is_some(),
            "Should have final_output"
        );
        assert!(
            parsed.get("strategy_used").is_some(),
            "Should have strategy_used"
        );
        assert!(
            parsed.get("per_paladin_times").is_some(),
            "Should have per_paladin_times"
        );
        assert!(
            parsed.get("per_paladin_tokens").is_some(),
            "Should have per_paladin_tokens"
        );
        assert!(
            parsed.get("total_tokens").is_some(),
            "Should have total_tokens"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_metadata_export_no_dir_configured() {
        // When no metadata_output_dir is configured, execute should still succeed
        // and no files should be written
        let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockPaladinPort);
        let paladins = vec![create_test_paladin(), create_test_paladin()];
        let config = BattalionConfig::new("no_export_test");

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(paladins)
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;
        assert!(
            result.is_ok(),
            "Execute should succeed without metadata dir: {:?}",
            result.err()
        );
    }

    /// Test that fail-fast error strategy stops execution on first failure
    #[tokio::test]
    async fn test_error_handling_fail_fast() {
        // Create 3 paladins
        let paladin1 = create_test_paladin();
        let paladin2 = create_test_paladin();
        let paladin3 = create_test_paladin();

        // Fail fast is the default Aegis policy (`on_error: None`)
        let config = BattalionConfig::new("test-fail-fast");

        let paladin_port = Arc::new(MockPaladinPort) as Arc<dyn PaladinPort>;
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin1, paladin2, paladin3])
            .config(config)
            .build()
            .unwrap();

        // Execute - should stop after second paladin (simulated failure)
        // Note: For this test, we'll verify execution completes. Actual failure simulation
        // would require MockPaladinPort to return errors, which we'll implement in follow-up
        let result = commander.execute("test input").await;

        // For now, just verify the commander executes formation strategy successfully
        assert!(
            result.is_ok(),
            "Fail-fast strategy should execute: {:?}",
            result.err()
        );

        // TODO: Enhance MockPaladinPort to support error injection for proper failure testing
    }

    /// Test that continue-on-error collects all errors and continues execution
    #[tokio::test]
    async fn test_error_handling_continue_on_error() {
        // Create 3 paladins
        let paladin1 = create_test_paladin();
        let paladin2 = create_test_paladin();
        let paladin3 = create_test_paladin();

        // Continue past failures: the Aegis Absorb handler
        let config = BattalionConfig::new("test-continue-on-error").with_aegis(Aegis {
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..Aegis::default()
        });

        let paladin_port = Arc::new(MockPaladinPort) as Arc<dyn PaladinPort>;
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin1, paladin2, paladin3])
            .config(config)
            .build()
            .unwrap();

        // Execute - should continue despite failures and collect all results
        let result = commander.execute("test input").await;

        assert!(
            result.is_ok(),
            "Continue-on-error strategy should complete: {:?}",
            result.err()
        );

        // TODO: Verify all paladins executed and partial results collected
    }

    /// Test retry-then-continue strategy performs retries before continuing
    #[tokio::test]
    async fn test_error_handling_retry_then_continue() {
        // Create paladins
        let paladin1 = create_test_paladin();
        let paladin2 = create_test_paladin();

        // v0.10 `RetryPolicy { max_attempts: 3 }` is `max_attempts: 4` under
        // Aegis (the count now includes the first attempt). `TransientAndUnknown`
        // keeps the v0.10 "retry everything" meaning: the default `TransientOnly`
        // would not retry an Unknown failure.
        let retry = AegisRetryPolicy {
            max_attempts: 4,
            initial_interval: Duration::from_millis(10),
            backoff_factor: 2.0,
            max_interval: Duration::from_millis(100),
            jitter: false,
            retry_on: RetryPredicate::TransientAndUnknown,
        };

        let config = BattalionConfig::new("test-retry-continue").with_aegis(Aegis {
            retry: Some(retry),
            on_error: Some(ErrorHandlerSpec::Absorb {
                fallback_delta: StateDelta::new(),
            }),
            ..Aegis::default()
        });

        let paladin_port = Arc::new(MockPaladinPort) as Arc<dyn PaladinPort>;
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin1, paladin2])
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;

        assert!(
            result.is_ok(),
            "Retry-then-continue should complete: {:?}",
            result.err()
        );

        // TODO: Verify retry attempts were made (requires enhanced MockPaladinPort)
    }

    /// Test partial failure handling in parallel execution (Phalanx)
    #[tokio::test]
    async fn test_partial_failure_handling() {
        // Create 4 paladins for parallel execution
        let paladin1 = create_test_paladin();
        let paladin2 = create_test_paladin();
        let paladin3 = create_test_paladin();
        let paladin4 = create_test_paladin();

        // Configure to continue on error for partial failure handling
        let config = BattalionConfig::new("test-partial-failure")
            .with_aegis(run_timeout_aegis(Duration::from_secs(60)));

        let paladin_port = Arc::new(MockPaladinPort) as Arc<dyn PaladinPort>;
        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Phalanx)
            .paladins(vec![paladin1, paladin2, paladin3, paladin4])
            .config(config)
            .build()
            .unwrap();

        let result = commander.execute("test input").await;

        assert!(
            result.is_ok(),
            "Phalanx with partial failures should complete: {:?}",
            result.err()
        );

        // Verify result metadata
        let result = result.unwrap();
        assert_eq!(
            result.status,
            BattalionStatus::Completed,
            "Execution should complete"
        );

        // TODO: When MockPaladinPort supports error injection:
        // - Verify success_count and failure_count in metadata
        // - Verify successful results are preserved
        // - Verify failure details are captured
    }

    /// Mock Herald for `format_result` tests, mirroring `herald.rs`'s own `MockHerald` test
    /// fixture shape.
    struct MockHerald;

    impl paladin_core::platform::container::herald::Herald for MockHerald {
        fn format_paladin_result(
            &self,
            result: &paladin_core::platform::container::herald::PaladinResult,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(format!("MOCK PALADIN: {}", result.output))
        }

        fn format_battalion_result(
            &self,
            result: &BattalionResult,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(format!("MOCK BATTALION: {}", result.battalion_name))
        }

        fn format_stream_chunk(
            &self,
            chunk: &paladin_core::platform::container::herald::StreamChunk,
        ) -> Result<Option<String>, paladin_core::platform::container::herald::HeraldError>
        {
            Ok(Some(chunk.content.clone()))
        }

        fn finalize_stream(
            &self,
            _metadata: &paladin_core::platform::container::herald::ExecutionMetadata,
        ) -> Result<String, paladin_core::platform::container::herald::HeraldError> {
            Ok(String::new())
        }

        fn format_error(
            &self,
            error: &paladin_core::platform::container::herald::PaladinError,
        ) -> String {
            format!("ERROR: {}", error)
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn mime_type(&self) -> &str {
            "text/plain"
        }
    }

    #[tokio::test]
    async fn test_commander_with_herald_formats_result() {
        let paladin_port = Arc::new(MockPaladinPort);
        let paladin = create_test_paladin();
        let config = create_test_config();

        let commander = CommanderBuilder::new(paladin_port)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin])
            .config(config)
            .build()
            .expect("commander should build")
            .with_herald(Arc::new(MockHerald));

        let result = commander
            .execute("Test input")
            .await
            .expect("execution should succeed");

        let formatted = commander
            .format_result(&result)
            .expect("format_result should succeed with a Herald configured");
        assert_eq!(
            formatted,
            Some(format!("MOCK BATTALION: {}", result.battalion_name))
        );

        // A Commander without a configured Herald returns Ok(None).
        let paladin_port_no_herald = Arc::new(MockPaladinPort);
        let paladin_no_herald = create_test_paladin();
        let config_no_herald = create_test_config();
        let commander_no_herald = CommanderBuilder::new(paladin_port_no_herald)
            .strategy(BattalionStrategy::Formation)
            .paladins(vec![paladin_no_herald])
            .config(config_no_herald)
            .build()
            .expect("commander should build");

        let result_no_herald = commander_no_herald
            .execute("Test input")
            .await
            .expect("execution should succeed");
        let unformatted = commander_no_herald
            .format_result(&result_no_herald)
            .expect("format_result should succeed without a Herald configured");
        assert_eq!(unformatted, None);
    }

    #[tokio::test]
    async fn test_commander_chain_of_command_uses_shared_conversion() {
        let paladin_port: Arc<dyn PaladinPort> = Arc::new(MockChainOfCommandPort);
        let paladins = vec![
            create_test_paladin_with_name("Commander"),
            create_test_paladin_with_name("Specialist_1"),
            create_test_paladin_with_name("Specialist_2"),
        ];
        let config = create_test_config();

        let commander = CommanderBuilder::new(Arc::clone(&paladin_port))
            .strategy(BattalionStrategy::ChainOfCommand)
            .paladins(paladins.clone())
            .config(config.clone())
            .build()
            .expect("commander should build");

        let commander_result = commander
            .execute("Test input")
            .await
            .expect("ChainOfCommand execution should succeed");

        // Independently drive the exact same inputs through the Chain of Command service
        // directly (not through the Commander), then compare the Commander's result against
        // values derived straight from that real `DelegationResult` -- not against a second
        // hand-built `BattalionResult` literal, which would silently recreate the exact
        // drift risk D-14 removes. If the Commander's branch ever stopped delegating to the
        // service's shared conversion, these values would no longer agree.
        let chain_service = crate::chain_of_command_service::ChainOfCommandExecutionService::new(
            Arc::clone(&paladin_port),
        );
        let chain =
            paladin_core::platform::container::battalion::chain_of_command::ChainOfCommand::new(
                paladins[0].clone(),
                paladins[1..].to_vec(),
                config.clone(),
            )
            .expect("chain construction should succeed");

        let delegation_result = chain_service
            .execute(&chain, "Test input")
            .await
            .expect("direct execution should succeed");

        assert_eq!(
            commander_result.strategy_used,
            BattalionStrategy::ChainOfCommand
        );
        assert_eq!(commander_result.status, BattalionStatus::Completed);
        assert_eq!(commander_result.battalion_name, config.name);
        assert_eq!(
            commander_result.final_output,
            delegation_result.outputs.join("\n")
        );
    }

    /// Tracer (Phase 44 D-01, D-02, LEGACY-02): `BattalionConfig.aegis` bounds
    /// each Paladin attempt on Campaign through the Commander -- never the run.
    #[tokio::test(start_paused = true)]
    async fn commander_campaign_honours_aegis_per_attempt_timeout_end_to_end() {
        use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
        use paladin_core::platform::container::node_error::{NodeErrorSource, TimeoutKind};
        use paladin_core::platform::container::transience::Transience;

        /// Sleeps a per-Paladin duration (keyed by Paladin name), then succeeds.
        struct SleepyPaladinPort {
            delays: HashMap<String, Duration>,
        }

        #[async_trait]
        impl PaladinPort for SleepyPaladinPort {
            async fn execute(
                &self,
                paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                let delay = self
                    .delays
                    .get(&paladin.data.name)
                    .copied()
                    .unwrap_or_default();
                tokio::time::sleep(delay).await;
                Ok(PaladinResult {
                    output: format!("{} done", paladin.data.name),
                    usage: TokenUsage::new(10, 0),
                    execution_time_ms: delay.as_millis() as u64,
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
                Err(PaladinError::ExecutionError("not used".to_string()))
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let bounded_config = || {
            BattalionConfig::new("aegis_tracer").with_aegis(Aegis {
                timeout: Some(TimeoutPolicy {
                    run_timeout: Some(Duration::from_secs(1)),
                    idle_timeout: None,
                }),
                ..Aegis::default()
            })
        };

        // Case 1: three 600 ms Paladins finish in 1.8 s -- longer than the
        // 1 s bound, so the bound is per attempt, not per run.
        let names = ["scout_a", "scout_b", "scout_c"];
        let port = Arc::new(SleepyPaladinPort {
            delays: names
                .iter()
                .map(|n| (n.to_string(), Duration::from_millis(600)))
                .collect(),
        });
        let commander = CommanderBuilder::new(port)
            .strategy(BattalionStrategy::Campaign)
            .paladins(
                names
                    .iter()
                    .map(|n| create_test_paladin_with_name(n))
                    .collect(),
            )
            .config(bounded_config())
            .build()
            .expect("commander builds");
        let started = tokio::time::Instant::now();
        let result = commander
            .execute("go")
            .await
            .expect("each 600 ms attempt is inside its 1 s bound");
        assert_eq!(result.status, BattalionStatus::Completed);
        assert_eq!(result.paladin_results.len(), 3);
        assert!(started.elapsed() >= Duration::from_millis(1800));

        // Case 2: one 1.5 s Paladin exceeds the 1 s per-attempt bound.
        let port = Arc::new(SleepyPaladinPort {
            delays: HashMap::from([("laggard".to_string(), Duration::from_millis(1500))]),
        });
        let commander = CommanderBuilder::new(port)
            .strategy(BattalionStrategy::Campaign)
            .paladins(vec![create_test_paladin_with_name("laggard")])
            .config(bounded_config())
            .build()
            .expect("commander builds");
        let started = tokio::time::Instant::now();
        let error = commander
            .execute("go")
            .await
            .expect_err("a 1.5 s attempt exceeds the 1 s bound");
        let elapsed = started.elapsed();
        match error {
            BattalionError::Node(e) => {
                assert_eq!(e.node_id.as_str(), "laggard");
                assert_eq!(e.attempt, 1);
                assert_eq!(e.transience, Transience::Transient);
                assert_eq!(e.source, NodeErrorSource::Timeout(TimeoutKind::Run));
            }
            other => panic!("expected BattalionError::Node, got {other:?}"),
        }
        assert!(elapsed >= Duration::from_secs(1));
        assert!(elapsed < Duration::from_millis(1500));
    }
}
