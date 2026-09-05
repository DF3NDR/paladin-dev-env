//! Battalion Orchestration Base Types
//!
//! This module provides the core domain types for multi-Paladin orchestration.
//! Battalions coordinate multiple Paladins using various execution patterns.

pub mod campaign;
pub mod chain_of_command;
pub mod conclave;
pub mod council;
pub mod formation;
pub mod grove;
pub mod phalanx;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::platform::container::execution_result::PaladinResult;
use crate::platform::container::paladin_error::PaladinError;
use crate::platform::container::registry_error::RegistryError;

/// Configuration for Battalion operations
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
///
/// let config = BattalionConfig::new("research_battalion")
///     .with_timeout(300)
///     .with_error_strategy(ErrorStrategy::FailFast);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattalionConfig {
    /// Name of the Battalion
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Maximum execution time in seconds
    pub timeout_seconds: u64,

    /// Retry policy for failed operations
    pub retry_policy: RetryPolicy,

    /// Strategy for handling errors
    pub error_strategy: ErrorStrategy,

    /// Directory for saving metadata output
    pub metadata_output_dir: Option<PathBuf>,
}

impl BattalionConfig {
    /// Create a new BattalionConfig with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name identifier for this Battalion
    ///
    /// Uses default values for all other configuration options.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            timeout_seconds: 300,
            retry_policy: RetryPolicy::default(),
            error_strategy: ErrorStrategy::default(),
            metadata_output_dir: None,
        }
    }

    /// Set the description (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `description` - Human-readable description of the Battalion's purpose
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the timeout in seconds (builder pattern).
    ///
    /// # Arguments
    ///
    /// * `seconds` - Maximum execution time before timing out
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set the retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Set the error strategy
    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    /// Set the metadata output directory
    pub fn with_metadata_dir(mut self, dir: PathBuf) -> Self {
        self.metadata_output_dir = Some(dir);
        self
    }

    /// Validate the metadata output directory, if configured
    ///
    /// Checks that the directory exists and is writable. If `metadata_output_dir`
    /// is `None`, validation passes trivially.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the directory is valid or not configured
    /// * `Err(String)` with a description of the validation failure
    pub fn validate_metadata_dir(&self) -> Result<(), String> {
        if let Some(dir) = &self.metadata_output_dir {
            if !dir.exists() {
                // Attempt to create the directory
                std::fs::create_dir_all(dir).map_err(|e| {
                    format!(
                        "Metadata output directory '{}' does not exist and could not be created: {}",
                        dir.display(),
                        e
                    )
                })?;
            }
            if !dir.is_dir() {
                return Err(format!(
                    "Metadata output path '{}' is not a directory",
                    dir.display()
                ));
            }
            // Check writability by attempting to create a temp file
            let test_path = dir.join(".paladin_write_test");
            std::fs::write(&test_path, b"test").map_err(|e| {
                format!(
                    "Metadata output directory '{}' is not writable: {}",
                    dir.display(),
                    e
                )
            })?;
            let _ = std::fs::remove_file(&test_path);
        }
        Ok(())
    }
}

impl Default for BattalionConfig {
    /// Creates a BattalionConfig with default values.
    ///
    /// # Default Values
    /// - `name`: "default_battalion"
    /// - `timeout_seconds`: 300 (5 minutes)
    /// - `retry_policy`: RetryPolicy::default()
    /// - `error_strategy`: ErrorStrategy::FailFast
    fn default() -> Self {
        Self::new("default_battalion")
    }
}

/// Retry policy configuration for Battalion operations.
///
/// Defines how failed operations should be retried with exponential backoff
/// and jitter to prevent thundering herd problems.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::battalion::RetryPolicy;
/// use std::time::Duration;
///
/// let policy = RetryPolicy {
///     max_attempts: 5,
///     base_delay: Duration::from_millis(200),
///     max_delay: Duration::from_secs(30),
///     exponential_backoff: true,
///     jitter: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Base delay between retries
    pub base_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Whether to use exponential backoff
    pub exponential_backoff: bool,

    /// Whether to add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryPolicy {
    /// Creates a RetryPolicy with sensible defaults.
    ///
    /// # Default Values
    /// - `max_attempts`: 3
    /// - `base_delay`: 100ms
    /// - `max_delay`: 10s
    /// - `exponential_backoff`: true
    /// - `jitter`: true
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_backoff: true,
            jitter: true,
        }
    }
}

/// Strategy for handling errors during Battalion execution.
///
/// Determines how the Battalion should respond when individual Paladin executions fail.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::battalion::ErrorStrategy;
///
/// let fail_fast = ErrorStrategy::FailFast; // Stop on first error
/// let continue_on_error = ErrorStrategy::ContinueOnError; // Collect all errors
/// let retry = ErrorStrategy::RetryThenContinue; // Retry then proceed
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ErrorStrategy {
    /// Stop immediately on first error
    #[default]
    FailFast,

    /// Continue execution despite errors, collect all at end
    ContinueOnError,

    /// Retry failed operations, then continue
    RetryThenContinue,
}

/// Battalion orchestration strategy
///
/// Defines the pattern used to coordinate multiple Paladins.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::battalion::BattalionStrategy;
///
/// let strategy = BattalionStrategy::Formation;
/// assert_eq!(strategy, BattalionStrategy::Formation);
///
/// let auto = BattalionStrategy::Auto;
/// // Auto will be resolved to a specific strategy at runtime
///
/// // Explicit strategy selection
/// let strategy = BattalionStrategy::Formation;
///
/// // Auto mode for intelligent selection
/// let auto_strategy = BattalionStrategy::Auto;
///
/// // Explicit strategy for known patterns
/// let formation = BattalionStrategy::Formation;
/// let phalanx = BattalionStrategy::Phalanx;
///
/// // Pattern matching
/// match formation {
///     BattalionStrategy::Formation => println!("Sequential pipeline"),
///     BattalionStrategy::Phalanx => println!("Parallel execution"),
///     BattalionStrategy::Campaign => println!("Graph workflow"),
///     BattalionStrategy::ChainOfCommand => println!("Hierarchical delegation"),
///     BattalionStrategy::Conclave => println!("Multi-expert synthesis"),
///     BattalionStrategy::Council => println!("Discussion orchestration"),
///     BattalionStrategy::Grove => println!("Intelligent routing"),
///     BattalionStrategy::Maneuver => println!("Flow DSL execution"),
///     BattalionStrategy::Auto => println!("Automatic selection"),
/// }
/// ```
///
/// # Strategy Descriptions
///
/// ## Formation
///
/// Sequential execution where each Paladin output becomes input for the next Paladin.
/// Ideal for pipelines and multi-stage transformations.
///
/// Use When:
/// - Tasks must be performed in a specific order
/// - Each step depends on the previous step output
/// - Data flows through a linear transformation pipeline
///
/// Example Use Cases:
/// - Research -> Analysis -> Summary workflow
/// - Data extraction -> Transformation -> Loading (ETL)
/// - Draft -> Edit -> Review document workflow
///
/// ## Phalanx
///
/// Concurrent parallel execution where all Paladins receive the same input and execute
/// simultaneously. Results are aggregated at the end.
/// Ideal for independent parallel tasks.
///
/// Use When:
/// - Tasks can run independently without dependencies
/// - All tasks need the same input data
/// - Want to maximize throughput via parallelism
///
/// Example Use Cases:
/// - Analyzing same data with different models
/// - Batch processing independent items
/// - Multi-perspective analysis (technical, business, legal review in parallel)
///
/// ## Campaign
///
/// Graph/DAG-based execution with conditional branching and complex dependencies.
/// Paladins are organized in a directed graph with conditional edges.
/// Ideal for complex workflows with branching logic.
///
/// Use When:
/// - Workflow has conditional branching (if-then-else)
/// - Dependencies form a complex graph (not just linear)
/// - Need dynamic routing based on intermediate results
///
/// Example Use Cases:
/// - Approval workflows with escalation paths
/// - Multi-stage decision trees
/// - Error handling with fallback paths
///
/// ## ChainOfCommand
///
/// Hierarchical delegation where a commander Paladin analyzes the task and delegates
/// to specialist Paladins based on expertise matching.
/// Ideal for dynamic task routing to specialists.
///
/// Use When:
/// - Have specialized Paladins with different expertise
/// - Task requires intelligent routing to the right specialist
/// - Need hierarchical decision-making
///
/// Example Use Cases:
/// - Customer support routing to specialized agents
/// - Code review routing to domain experts
/// - Medical triage routing to specialists
///
/// ## Auto
///
/// Automatic strategy selection based on intelligent heuristics analyzing:
/// - Input keywords such as parallel, sequential, workflow, delegate
/// - Number of Paladins (1-3 uses Formation, 4+ considers parallelism)
/// - Task characteristics
///
/// Use When:
/// - Want the framework to select the optimal pattern
/// - Building general-purpose orchestration APIs
/// - Prototyping or exploring different patterns
///
/// Selection Rules:
/// - Formation: Keywords like sequential, step-by-step, pipeline; or 1-3 Paladins
/// - Phalanx: Keywords like parallel, concurrent, simultaneously; or 4+ similar tasks
/// - Campaign: Keywords like workflow, conditional, if-then, depends-on
/// - ChainOfCommand: Keywords like delegate, specialist, expert, route-to
/// - Conclave: Keywords like synthesize, compare, expert panel, consensus; or 3+ diverse experts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattalionStrategy {
    /// Sequential execution with output chaining (Paladin N output -> Paladin N+1 input)
    ///
    /// Best for linear pipelines where each stage depends on the previous stage's output.
    Formation,

    /// Concurrent parallel execution (all Paladins receive same input, results aggregated)
    ///
    /// Best for independent tasks that can run simultaneously to maximize throughput.
    Phalanx,

    /// Graph/DAG-based orchestration (conditional branching and complex workflows)
    ///
    /// Best for complex workflows with conditional logic and multi-path dependencies.
    Campaign,

    /// Hierarchical delegation pattern (commander delegates to specialist Paladins)
    ///
    /// Best for dynamic routing to specialists based on task characteristics.
    ChainOfCommand,

    /// Mixture of Agents pattern (multiple expert Paladins analyze in parallel, aggregator synthesizes)
    ///
    /// Best for complex analytical tasks requiring diverse expert perspectives with synthesis.
    /// All expert Paladins process the input independently in parallel with retry logic,
    /// then an aggregator Paladin synthesizes their outputs into a comprehensive response.
    ///
    /// Particularly effective for:
    /// - Multi-perspective analysis (legal + technical + business review)
    /// - Comparative evaluations (pros/cons from different viewpoints)
    /// - Expert consensus building
    /// - Complex decision-making requiring diverse expertise
    Conclave,

    /// Conversational multi-agent pattern (Paladins engage in turn-based discussion)
    ///
    /// Best for collaborative problem-solving through structured dialogue with flexible turn-taking.
    /// Paladins take turns contributing to a shared conversation using configurable turn strategies
    /// (RoundRobin, ModeratorDirected, Random, VoluntaryWithTimeout) until a termination condition
    /// is met (MaxRounds, Consensus, ModeratorDecision, Keyword detection).
    ///
    /// Particularly effective for:
    /// - Collaborative brainstorming and idea refinement
    /// - Debate and deliberation with opposing viewpoints
    /// - Consensus-building discussions
    /// - Iterative problem-solving with feedback
    Council,

    /// Tree-based intelligent routing pattern (routes tasks to specialized agents by expertise)
    ///
    /// Best for dynamically routing tasks to the most qualified agent based on expertise matching.
    /// Uses configurable routing strategies (KeywordMatch, SemanticSimilarity, LlmRouting) to
    /// analyze task requirements and select the optimal agent from a hierarchical tree structure.
    ///
    /// Particularly effective for:
    /// - Customer support routing to specialized departments
    /// - Task assignment based on domain expertise
    /// - Dynamic workload distribution by capability
    /// - Intelligent delegation in complex organizations
    Grove,

    /// Flow DSL-based orchestration (declarative flow expressions with sequential/parallel composition)
    ///
    /// Best for dynamic agent workflows defined using a declarative flow language.
    /// Supports flow expressions like "agent1 -> agent2 -> (agent3 | agent4)" for sequential
    /// chaining and parallel branching with flexible error handling strategies.
    ///
    /// Particularly effective for:
    /// - Dynamic workflow configuration from external definitions
    /// - Mixed sequential/parallel patterns in a single workflow
    /// - Complex branching logic with nested parallel sections
    /// - Runtime-defined agent orchestration patterns
    Maneuver,

    /// Automatic strategy selection based on heuristics
    ///
    /// Analyzes input and Paladin characteristics to intelligently select Formation,
    /// Phalanx, Campaign, ChainOfCommand, Conclave, Council, Grove, or Maneuver. Provides reasoning for transparency.
    Auto,
}

/// Current status of a Battalion execution.
///
/// Tracks the lifecycle state of a Battalion from creation through completion.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::battalion::BattalionStatus;
///
/// let mut status = BattalionStatus::Idle;
/// status = BattalionStatus::Running;
/// // ... execution happens ...
/// status = BattalionStatus::Completed;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BattalionStatus {
    /// Battalion is idle, not yet started
    #[default]
    Idle,

    /// Battalion is currently executing
    Running,

    /// Battalion execution is paused
    Paused,

    /// Battalion completed successfully
    Completed,

    /// Battalion failed with errors
    Failed,

    /// Battalion was cancelled
    Cancelled,
}

// Token usage metrics for a single Paladin execution — re-exported from the
// canonical definition (ADR-0016 / DEBT-05) so this path keeps resolving.
pub use crate::platform::container::token_usage::TokenUsage;

/// Per-node failure detail for a Battalion execution.
///
/// Carries the failed Paladin's name and its error text, so an operator can
/// diagnose WHICH node failed and WHY, instead of only an aggregate
/// success/failure count. Populated for any strategy that continues past a
/// failure (`ContinueOnError`, `RetryThenContinue`) — see
/// `FormationExecutionService::execute_internal` and
/// `PhalanxExecutionService::execute_internal`; empty for a fully-successful
/// run and for strategies that do not collect it.
///
/// A new plain-data struct (mirroring [`TokenUsage`]'s shape) rather than a
/// reuse of [`BattalionError`], since `BattalionError` does not derive
/// `Serialize`/`Deserialize` and `BattalionResult` does.
///
/// **Not the same type as [`crate::platform::container::node_error::NodeError`]
/// (D-06):** this is the pre-existing v0.9 plain-data summary carried on
/// [`BattalionResult`] (name + error text only) for the legacy
/// Formation/Phalanx/Campaign patterns. The structured, engine-execution
/// `NodeError` landed by Phase 25 for the superstep engine's per-node retry
/// loop is a different, unrelated type in a different module -- it is never
/// aliased here and this summary is never reshaped to match it (X-03).
/// [`BattalionError::Node`] wraps the STRUCTURED type, path-qualified, never
/// this summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeError {
    /// Name of the Paladin node that failed
    pub node_name: String,
    /// The error text produced by that node's failure
    pub error: String,
}

/// Result of a Battalion execution
///
/// Contains the final output, individual Paladin results, and execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattalionResult {
    /// Unique identifier for this execution
    pub battalion_id: Uuid,

    /// Name of the Battalion
    pub battalion_name: String,

    /// When execution started
    pub started_at: DateTime<Utc>,

    /// When execution completed
    pub completed_at: DateTime<Utc>,

    /// Final aggregated output
    pub final_output: String,

    /// Individual Paladin results
    pub paladin_results: Vec<PaladinResult>,

    /// Final status
    pub status: BattalionStatus,

    /// The orchestration strategy that was used for execution
    /// For Auto mode, this contains the resolved strategy, not Auto
    pub strategy_used: BattalionStrategy,

    /// Reasoning for strategy selection (only present for Auto mode)
    pub strategy_selection_reasoning: Option<String>,

    /// Time spent on strategy selection in milliseconds
    pub strategy_selection_time_ms: u64,

    /// Execution time for each Paladin in milliseconds, keyed by Paladin name
    pub per_paladin_times: HashMap<String, u64>,

    /// Token usage metrics for each Paladin, keyed by Paladin name
    pub per_paladin_tokens: HashMap<String, TokenUsage>,

    /// Total tokens consumed across all Paladin executions
    pub total_tokens: u64,

    /// Count of Paladins that completed successfully
    pub paladin_success_count: usize,

    /// Count of Paladins that failed
    pub paladin_failure_count: usize,

    /// Per-node failure detail (name + error text) for any strategy that
    /// continues past a failure. Empty for a fully-successful run and for
    /// strategies that do not collect it.
    #[serde(default)]
    pub node_errors: Vec<NodeError>,
}

impl BattalionResult {
    /// Create a new BattalionResult
    pub fn new(
        battalion_id: Uuid,
        battalion_name: String,
        started_at: DateTime<Utc>,
        final_output: String,
        paladin_results: Vec<PaladinResult>,
    ) -> Self {
        // Count successes and failures based on stop_reason
        let paladin_success_count = paladin_results
            .iter()
            .filter(|r| {
                matches!(
                    r.stop_reason,
                    crate::platform::container::execution_result::StopReason::Completed
                )
            })
            .count();
        let paladin_failure_count = paladin_results.len() - paladin_success_count;

        Self {
            battalion_id,
            battalion_name,
            started_at,
            completed_at: Utc::now(),
            final_output,
            paladin_results,
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Formation, // Default to Formation
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: HashMap::new(),
            per_paladin_tokens: HashMap::new(),
            total_tokens: 0,
            paladin_success_count,
            paladin_failure_count,
            node_errors: Vec::new(),
        }
    }

    /// Create from a list of Paladin results (sequential execution)
    pub fn from_paladin_results(
        battalion_id: Uuid,
        battalion_name: String,
        started_at: DateTime<Utc>,
        results: Vec<PaladinResult>,
    ) -> Self {
        let final_output = results.last().map(|r| r.output.clone()).unwrap_or_default();

        Self::new(
            battalion_id,
            battalion_name,
            started_at,
            final_output,
            results,
        )
    }

    /// Get execution duration
    pub fn duration(&self) -> Duration {
        (self.completed_at - self.started_at)
            .to_std()
            .unwrap_or_default()
    }

    /// Set the strategy used for this execution
    pub fn with_strategy(mut self, strategy: BattalionStrategy) -> Self {
        self.strategy_used = strategy;
        self
    }

    /// Set the strategy selection reasoning (for Auto mode)
    pub fn with_selection_reasoning(mut self, reasoning: String) -> Self {
        self.strategy_selection_reasoning = Some(reasoning);
        self
    }

    /// Set the strategy selection time in milliseconds
    pub fn with_selection_time_ms(mut self, time_ms: u64) -> Self {
        self.strategy_selection_time_ms = time_ms;
        self
    }

    /// Set the per-Paladin execution times (keyed by Paladin name, values in milliseconds)
    pub fn with_paladin_times(mut self, times: HashMap<String, u64>) -> Self {
        self.per_paladin_times = times;
        self
    }

    /// Set the per-Paladin token usage metrics (keyed by Paladin name)
    pub fn with_paladin_tokens(mut self, tokens: HashMap<String, TokenUsage>) -> Self {
        self.per_paladin_tokens = tokens;
        self
    }

    /// Set the total token count across all Paladin executions
    pub fn with_total_tokens(mut self, total_tokens: u64) -> Self {
        self.total_tokens = total_tokens;
        self
    }
}

/// Errors specific to Council pattern operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum CouncilError {
    /// No participants configured in the Council
    #[error("No participants configured")]
    NoParticipants,

    /// Moderator required for ModeratorDirected strategy
    #[error("Moderator required for ModeratorDirected strategy")]
    ModeratorRequired,

    /// Participant execution failed
    #[error("Participant execution failed: {0}")]
    ParticipantError(String),

    /// Invalid turn strategy configuration
    #[error("Invalid turn strategy configuration: {0}")]
    InvalidStrategy(String),

    /// Maximum rounds must be greater than zero
    #[error("Maximum rounds must be greater than zero")]
    InvalidMaxRounds,
}

/// Errors specific to Grove pattern operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum GroveError {
    /// No trees configured in the Grove
    #[error("No trees configured")]
    NoTrees,

    /// No agents available in the Grove
    #[error("No agents in grove")]
    NoAgents,

    /// Routing operation failed
    #[error("Routing failed: {0}")]
    RoutingFailed(String),

    /// No agent meets the similarity threshold
    #[error("No agent meets similarity threshold {0}")]
    NoMatchingAgent(f32),

    /// Embeddings required for SemanticSimilarity strategy
    #[error("Embeddings required for SemanticSimilarity strategy")]
    EmbeddingsRequired,

    /// Invalid similarity threshold (must be 0.0-1.0)
    #[error("Invalid similarity threshold: {0} (must be between 0.0 and 1.0)")]
    InvalidSimilarityThreshold(f32),
}

/// Error types for Battalion operations
///
/// # Non-exhaustive (X-10.2, D-04)
///
/// Marked `#[non_exhaustive]` so a future variant can be added without a
/// semver-major bump. Registered as deliberate-breaking in `MIGRATION.md`
/// §9.2 and `.cargo/semver-checks-allowlist.toml` (the `enum_marked_non_exhaustive`
/// lint) in the same commit that added this attribute. Every downstream
/// exhaustive match gained a wildcard arm at that commit.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum BattalionError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Paladin execution error
    #[error("Paladin error: {0}")]
    PaladinError(String),

    /// Formation-specific error
    #[error("Formation error: {0}")]
    FormationError(String),

    /// Phalanx-specific error
    #[error("Phalanx error: {0}")]
    PhalanxError(String),

    /// Campaign-specific error
    #[error("Campaign error: {0}")]
    CampaignError(String),

    /// Invalid graph structure
    #[error("Invalid graph: {0}")]
    InvalidGraph(String),

    /// Chain of Command error
    #[error("Chain of Command error: {0}")]
    ChainOfCommandError(String),

    /// Commander validation error
    #[error("Commander validation error: {0}")]
    CommanderValidation(String),

    /// Strategy selection error
    #[error("Strategy selection failed: {0}")]
    StrategySelection(String),

    /// Council pattern error
    #[error("Council error: {0}")]
    CouncilError(#[from] CouncilError),

    /// Grove pattern error
    #[error("Grove error: {0}")]
    GroveError(#[from] GroveError),

    /// Routing error (Grove pattern) - kept for backward compatibility
    #[error("Routing error: {0}")]
    RoutingError(String),

    /// Paladin not found in registry
    #[error("Paladin not found in registry: {0}")]
    PaladinNotFound(String),

    /// Grove routing failed
    #[error("Grove routing failed: {0}")]
    GroveRoutingFailed(String),

    /// Metadata export failed (non-fatal)
    #[error("Metadata export failed: {0}")]
    MetadataExportFailed(String),

    /// Timeout error
    #[error("Battalion execution timed out after {0} seconds")]
    Timeout(u64),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Aggregation error
    #[error("Aggregation error: {0}")]
    AggregationError(String),

    /// Battalion execution cancelled
    #[error("Battalion execution was cancelled")]
    Cancelled,

    /// General execution error
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// A structured node-execution failure from the superstep engine
    /// (Doc 04 FT-FR-02, D-06). Path-qualified to the NEW structured
    /// `paladin_core::platform::container::node_error::NodeError` -- never
    /// this module's legacy `NodeError { node_name, error }` summary, which
    /// is a different, pre-existing v0.9 type left untouched (see its own
    /// rustdoc cross-link above).
    #[error("Node error: {0}")]
    Node(crate::platform::container::node_error::NodeError),
}

/// Convert RegistryError to BattalionError
///
/// This conversion allows registry errors to be propagated as Battalion errors,
/// particularly useful for Council and Grove operations that need to resolve
/// Paladin IDs from the registry.
impl From<RegistryError> for BattalionError {
    fn from(error: RegistryError) -> Self {
        match error {
            RegistryError::DuplicateId(id) => {
                BattalionError::ConfigurationError(format!("Duplicate Paladin ID: {}", id))
            }
            RegistryError::InvalidId(id) => {
                BattalionError::ValidationError(format!("Invalid Paladin ID: {}", id))
            }
            RegistryError::AccessFailed(msg) => {
                BattalionError::ExecutionError(format!("Registry access failed: {}", msg))
            }
        }
    }
}

/// Convert PaladinError to BattalionError
///
/// Allows Paladin execution errors to be propagated as Battalion errors
/// in multi-agent orchestration patterns such as Chain of Command.
impl From<PaladinError> for BattalionError {
    fn from(err: PaladinError) -> Self {
        BattalionError::PaladinError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battalion_config_default() {
        let config = BattalionConfig::default();
        assert_eq!(config.name, "default_battalion");
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.error_strategy, ErrorStrategy::FailFast);
    }

    #[test]
    fn test_battalion_config_builder() {
        let config = BattalionConfig::new("test_battalion")
            .with_description("Test description")
            .with_timeout(600)
            .with_error_strategy(ErrorStrategy::ContinueOnError);

        assert_eq!(config.name, "test_battalion");
        assert_eq!(config.description, Some("Test description".to_string()));
        assert_eq!(config.timeout_seconds, 600);
        assert_eq!(config.error_strategy, ErrorStrategy::ContinueOnError);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay, Duration::from_millis(100));
        assert_eq!(policy.max_delay, Duration::from_secs(10));
        assert!(policy.exponential_backoff);
        assert!(policy.jitter);
    }

    #[test]
    fn test_error_strategy_variants() {
        let fail_fast = ErrorStrategy::FailFast;
        let continue_on_error = ErrorStrategy::ContinueOnError;
        let retry_then_continue = ErrorStrategy::RetryThenContinue;

        assert_eq!(fail_fast, ErrorStrategy::FailFast);
        assert_eq!(continue_on_error, ErrorStrategy::ContinueOnError);
        assert_eq!(retry_then_continue, ErrorStrategy::RetryThenContinue);
        assert_ne!(fail_fast, continue_on_error);
    }

    #[test]
    fn test_error_strategy_default() {
        let strategy = ErrorStrategy::default();
        assert_eq!(strategy, ErrorStrategy::FailFast);
    }

    #[test]
    fn test_battalion_status_variants() {
        assert_eq!(BattalionStatus::default(), BattalionStatus::Idle);

        let statuses = vec![
            BattalionStatus::Idle,
            BattalionStatus::Running,
            BattalionStatus::Paused,
            BattalionStatus::Completed,
            BattalionStatus::Failed,
            BattalionStatus::Cancelled,
        ];

        for status in statuses {
            // Verify each status can be created
            let _ = status;
        }
    }

    #[test]
    fn test_battalion_result_new() {
        let battalion_id = Uuid::new_v4();
        let started_at = Utc::now();

        let result = BattalionResult::new(
            battalion_id,
            "test_battalion".to_string(),
            started_at,
            "final output".to_string(),
            vec![],
        );

        assert_eq!(result.battalion_id, battalion_id);
        assert_eq!(result.battalion_name, "test_battalion");
        assert_eq!(result.final_output, "final output");
        assert_eq!(result.status, BattalionStatus::Completed);
        assert!(result.paladin_results.is_empty());
    }

    #[test]
    fn test_battalion_result_duration() {
        let battalion_id = Uuid::new_v4();
        let started_at = Utc::now();

        let mut result = BattalionResult::new(
            battalion_id,
            "test_battalion".to_string(),
            started_at,
            "output".to_string(),
            vec![],
        );

        // Set completed_at to 2 seconds after started_at
        result.completed_at = started_at + chrono::Duration::seconds(2);

        let duration = result.duration();
        assert_eq!(duration.as_secs(), 2);
    }

    #[test]
    fn test_battalion_config_serialization() {
        let config = BattalionConfig::new("test").with_timeout(120);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: BattalionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.timeout_seconds, deserialized.timeout_seconds);
    }

    #[test]
    fn test_retry_policy_serialization() {
        let policy = RetryPolicy::default();

        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: RetryPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(policy.max_attempts, deserialized.max_attempts);
        assert_eq!(policy.exponential_backoff, deserialized.exponential_backoff);
    }

    #[test]
    fn test_error_strategy_serialization() {
        let strategy = ErrorStrategy::ContinueOnError;

        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: ErrorStrategy = serde_json::from_str(&json).unwrap();

        assert_eq!(strategy, deserialized);
    }

    #[test]
    fn test_battalion_status_serialization() {
        let status = BattalionStatus::Running;

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: BattalionStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_battalion_result_serialization() {
        let result = BattalionResult::new(
            Uuid::new_v4(),
            "test".to_string(),
            Utc::now(),
            "output".to_string(),
            vec![],
        );

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BattalionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.battalion_id, deserialized.battalion_id);
        assert_eq!(result.battalion_name, deserialized.battalion_name);
        assert_eq!(result.final_output, deserialized.final_output);
    }

    #[test]
    fn test_battalion_error_variants() {
        let errors = vec![
            BattalionError::ConfigurationError("test".to_string()),
            BattalionError::PaladinError("test".to_string()),
            BattalionError::FormationError("test".to_string()),
            BattalionError::PhalanxError("test".to_string()),
            BattalionError::CampaignError("test".to_string()),
            BattalionError::InvalidGraph("test".to_string()),
            BattalionError::ChainOfCommandError("test".to_string()),
            BattalionError::Timeout(300),
            BattalionError::ValidationError("test".to_string()),
            BattalionError::AggregationError("test".to_string()),
            BattalionError::PaladinNotFound("paladin123".to_string()),
            BattalionError::GroveRoutingFailed("no matching agent".to_string()),
            BattalionError::MetadataExportFailed("disk full".to_string()),
        ];

        // Verify error messages
        for error in errors {
            let msg = error.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_new_error_messages_format() {
        // Test PaladinNotFound error message
        let error = BattalionError::PaladinNotFound("test_paladin".to_string());
        assert_eq!(
            error.to_string(),
            "Paladin not found in registry: test_paladin"
        );

        // Test GroveRoutingFailed error message
        let error = BattalionError::GroveRoutingFailed("confidence too low".to_string());
        assert_eq!(
            error.to_string(),
            "Grove routing failed: confidence too low"
        );

        // Test MetadataExportFailed error message
        let error = BattalionError::MetadataExportFailed("permission denied".to_string());
        assert_eq!(
            error.to_string(),
            "Metadata export failed: permission denied"
        );
    }

    #[test]
    fn test_registry_error_conversion() {
        use crate::platform::container::registry_error::RegistryError;

        // Test DuplicateId conversion
        let registry_error = RegistryError::DuplicateId("duplicate_id".to_string());
        let battalion_error: BattalionError = registry_error.into();
        assert!(matches!(
            battalion_error,
            BattalionError::ConfigurationError(_)
        ));
        assert!(
            battalion_error
                .to_string()
                .contains("Duplicate Paladin ID: duplicate_id")
        );

        // Test InvalidId conversion
        let registry_error = RegistryError::InvalidId("".to_string());
        let battalion_error: BattalionError = registry_error.into();
        assert!(matches!(
            battalion_error,
            BattalionError::ValidationError(_)
        ));
        assert!(battalion_error.to_string().contains("Invalid Paladin ID"));

        // Test AccessFailed conversion
        let registry_error = RegistryError::AccessFailed("lock poisoned".to_string());
        let battalion_error: BattalionError = registry_error.into();
        assert!(matches!(battalion_error, BattalionError::ExecutionError(_)));
        assert!(
            battalion_error
                .to_string()
                .contains("Registry access failed")
        );
    }

    #[test]
    fn test_token_usage_new() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_token_usage_from_total() {
        let usage = TokenUsage::from_total(200);
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 200);
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage::new(100, 50);
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: TokenUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage, deserialized);
    }

    #[test]
    fn test_battalion_metadata_serialization() {
        let mut per_paladin_times = HashMap::new();
        per_paladin_times.insert("analyst".to_string(), 1500u64);
        per_paladin_times.insert("reviewer".to_string(), 2300u64);

        let mut per_paladin_tokens = HashMap::new();
        per_paladin_tokens.insert("analyst".to_string(), TokenUsage::new(500, 200));
        per_paladin_tokens.insert("reviewer".to_string(), TokenUsage::new(300, 150));

        let result = BattalionResult::new(
            Uuid::new_v4(),
            "metadata_test".to_string(),
            Utc::now(),
            "output".to_string(),
            vec![],
        )
        .with_paladin_times(per_paladin_times.clone())
        .with_paladin_tokens(per_paladin_tokens.clone())
        .with_total_tokens(1150);

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BattalionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.per_paladin_times.len(), 2);
        assert_eq!(
            deserialized.per_paladin_times.get("analyst"),
            Some(&1500u64)
        );
        assert_eq!(
            deserialized.per_paladin_times.get("reviewer"),
            Some(&2300u64)
        );
        assert_eq!(deserialized.per_paladin_tokens.len(), 2);
        assert_eq!(
            deserialized.per_paladin_tokens.get("analyst"),
            Some(&TokenUsage::new(500, 200))
        );
        assert_eq!(deserialized.total_tokens, 1150);
    }

    #[test]
    fn test_token_usage_aggregation_calculation() {
        let mut per_paladin_tokens = HashMap::new();
        per_paladin_tokens.insert("agent_a".to_string(), TokenUsage::new(500, 200));
        per_paladin_tokens.insert("agent_b".to_string(), TokenUsage::new(300, 150));
        per_paladin_tokens.insert("agent_c".to_string(), TokenUsage::from_total(100));

        // Calculate total from individual token usages
        let total_tokens: u64 = per_paladin_tokens
            .values()
            .map(|t| u64::from(t.total_tokens))
            .sum();

        assert_eq!(total_tokens, 1250); // 700 + 450 + 100

        let result = BattalionResult::new(
            Uuid::new_v4(),
            "aggregation_test".to_string(),
            Utc::now(),
            "output".to_string(),
            vec![],
        )
        .with_paladin_tokens(per_paladin_tokens)
        .with_total_tokens(total_tokens);

        assert_eq!(result.total_tokens, 1250);
        assert_eq!(result.per_paladin_tokens.len(), 3);

        // Verify individual breakdowns
        let agent_a = result.per_paladin_tokens.get("agent_a").unwrap();
        assert_eq!(agent_a.prompt_tokens, 500);
        assert_eq!(agent_a.completion_tokens, 200);
        assert_eq!(agent_a.total_tokens, 700);

        let agent_b = result.per_paladin_tokens.get("agent_b").unwrap();
        assert_eq!(agent_b.prompt_tokens, 300);
        assert_eq!(agent_b.completion_tokens, 150);
        assert_eq!(agent_b.total_tokens, 450);
    }

    #[test]
    fn test_battalion_result_new_initializes_empty_metrics() {
        let result = BattalionResult::new(
            Uuid::new_v4(),
            "test".to_string(),
            Utc::now(),
            "output".to_string(),
            vec![],
        );

        assert!(result.per_paladin_times.is_empty());
        assert!(result.per_paladin_tokens.is_empty());
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn test_battalion_result_builder_methods() {
        let mut times = HashMap::new();
        times.insert("paladin_1".to_string(), 1000u64);

        let mut tokens = HashMap::new();
        tokens.insert("paladin_1".to_string(), TokenUsage::new(100, 50));

        let result = BattalionResult::new(
            Uuid::new_v4(),
            "builder_test".to_string(),
            Utc::now(),
            "output".to_string(),
            vec![],
        )
        .with_paladin_times(times)
        .with_paladin_tokens(tokens)
        .with_total_tokens(150);

        assert_eq!(result.per_paladin_times.len(), 1);
        assert_eq!(result.per_paladin_times.get("paladin_1"), Some(&1000u64));
        assert_eq!(result.per_paladin_tokens.len(), 1);
        assert_eq!(result.total_tokens, 150);
    }
}

#[test]
fn test_battalion_strategy_creation() {
    let formation = BattalionStrategy::Formation;
    let phalanx = BattalionStrategy::Phalanx;
    let campaign = BattalionStrategy::Campaign;
    let chain = BattalionStrategy::ChainOfCommand;
    let auto = BattalionStrategy::Auto;

    assert_eq!(formation, BattalionStrategy::Formation);
    assert_eq!(phalanx, BattalionStrategy::Phalanx);
    assert_eq!(campaign, BattalionStrategy::Campaign);
    assert_eq!(chain, BattalionStrategy::ChainOfCommand);
    assert_eq!(auto, BattalionStrategy::Auto);
    assert_ne!(formation, phalanx);
}

#[test]
fn test_battalion_strategy_serialization() {
    let strategy = BattalionStrategy::Formation;
    let serialized = serde_json::to_string(&strategy).unwrap();
    let deserialized: BattalionStrategy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(strategy, deserialized);

    let auto = BattalionStrategy::Auto;
    let serialized = serde_json::to_string(&auto).unwrap();
    let deserialized: BattalionStrategy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(auto, deserialized);
}

// ── Task 8.0: Commander metadata export configuration tests ──

#[test]
fn test_battalion_config_with_metadata_dir() {
    let dir = std::env::temp_dir().join("paladin_test_metadata_8_0");
    let config = BattalionConfig::new("meta_test")
        .with_timeout(120)
        .with_metadata_dir(dir.clone());

    assert_eq!(config.metadata_output_dir, Some(dir.clone()));
    assert!(config.validate_metadata_dir().is_ok());

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_battalion_config_without_metadata_dir() {
    let config = BattalionConfig::new("no_meta_test").with_timeout(120);

    assert_eq!(config.metadata_output_dir, None);
    assert!(config.validate_metadata_dir().is_ok());
}

#[test]
fn test_battalion_config_metadata_dir_not_a_directory() {
    let file_path = std::env::temp_dir().join("paladin_test_not_a_dir_8_0");
    // Create a file (not a directory)
    std::fs::write(&file_path, b"not a directory").unwrap();

    let config = BattalionConfig::new("not_dir_test")
        .with_timeout(120)
        .with_metadata_dir(file_path.clone());

    let result = config.validate_metadata_dir();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("is not a directory"));

    // Cleanup
    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn test_battalion_config_metadata_dir_auto_creates() {
    let dir = std::env::temp_dir().join("paladin_test_auto_create_8_0");
    // Ensure it doesn't exist
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!dir.exists());

    let config = BattalionConfig::new("auto_create_test")
        .with_timeout(120)
        .with_metadata_dir(dir.clone());

    // validate_metadata_dir should auto-create
    assert!(config.validate_metadata_dir().is_ok());
    assert!(dir.exists());
    assert!(dir.is_dir());

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn battalion_error_node_carries_the_structured_node_error() {
    use crate::platform::container::node_error::{
        NodeError as StructuredNodeError, NodeErrorSource,
    };
    use crate::platform::container::transience::Transience;
    use crate::platform::container::waypoint::NodeId;

    let node_error = StructuredNodeError {
        node_id: NodeId::new("summarize"),
        attempt: 2,
        transience: Transience::Transient,
        source: NodeErrorSource::Function {
            message: "connection reset".to_string(),
        },
    };
    let battalion_error = BattalionError::Node(node_error.clone());

    // Constructible and Clone.
    let cloned = battalion_error.clone();
    assert!(matches!(cloned, BattalionError::Node(_)));

    // Display names the node id and the source summary.
    let rendered = battalion_error.to_string();
    assert!(rendered.contains("summarize"));
    assert!(rendered.contains("connection reset"));
}

#[test]
fn battalion_error_node_uses_the_core_node_error_not_the_legacy_summary() {
    use crate::platform::container::node_error::{
        NodeError as StructuredNodeError, NodeErrorSource,
    };
    use crate::platform::container::transience::Transience;
    use crate::platform::container::waypoint::NodeId;

    let node_error = StructuredNodeError {
        node_id: NodeId::new("summarize"),
        attempt: 1,
        transience: Transience::Unknown,
        source: NodeErrorSource::Cancelled,
    };
    let battalion_error = BattalionError::Node(node_error);

    // A typed binding proves the payload type is the structured
    // `node_error::NodeError`, never this module's legacy summary struct
    // (which has no `node_id`/`attempt`/`transience`/`source` fields).
    if let BattalionError::Node(payload) = battalion_error {
        let _typed: StructuredNodeError = payload;
        assert_eq!(_typed.node_id, NodeId::new("summarize"));
        assert_eq!(_typed.attempt, 1);
    } else {
        panic!("expected BattalionError::Node");
    }
}

#[test]
fn legacy_battalion_node_error_summary_is_unchanged() {
    let legacy = NodeError {
        node_name: "researcher".to_string(),
        error: "timed out".to_string(),
    };
    assert_eq!(legacy.node_name, "researcher");
    assert_eq!(legacy.error, "timed out");

    let json = serde_json::to_string(&legacy).expect("serialize");
    assert!(json.contains("\"node_name\":\"researcher\""));
    assert!(json.contains("\"error\":\"timed out\""));

    let round_tripped: NodeError = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(round_tripped.node_name, legacy.node_name);
    assert_eq!(round_tripped.error, legacy.error);
}
