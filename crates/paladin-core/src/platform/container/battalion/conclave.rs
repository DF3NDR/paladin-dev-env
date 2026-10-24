//! Conclave Domain Entity
//!
//! Conclave implements the MixtureOfAgents (MoA) orchestration pattern where multiple
//! expert Paladins process the same task in parallel from different perspectives, and
//! a designated aggregator Paladin synthesizes their outputs into a single,
//! high-quality response.
//!
//! This pattern is particularly useful for:
//! - Complex decision-making requiring multiple expert perspectives
//! - Code review with security, performance, and maintainability analysis
//! - Strategy evaluation from technical, business, and risk viewpoints
//! - Research synthesis combining multiple analytical approaches
//!
//! # Example
//!
//! ```ignore
//! use paladin_core::platform::container::battalion::conclave::{Conclave, ConclaveConfig};
//! use paladin_core::platform::container::battalion::BattalionConfig;
//!
//! let battalion_config = BattalionConfig::new("expert_panel");
//! let conclave_config = ConclaveConfig::new("TechnicalAnalysis", battalion_config);
//!
//! let experts = vec![technical_expert, business_expert, security_expert];
//! let aggregator = chief_analyst;
//!
//! let conclave = Conclave::new(
//!     experts,
//!     aggregator,
//!     conclave_config,
//! ).expect("Valid conclave");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{BattalionConfig, BattalionError};
use crate::platform::container::execution_result::PaladinResult;
use crate::platform::container::paladin::Paladin;

/// Observability level for Conclave execution
///
/// Controls the level of detail in logging and metrics collection during
/// Conclave execution. Higher levels provide more detailed information but
/// may impact performance.
///
/// # Levels
///
/// - **Minimal**: Only final aggregated result and overall status
/// - **Standard**: Execution time per expert, total time, retry counts, success/failure status
/// - **Verbose**: Full expert outputs, token usage, LLM provider details, timestamps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ObservabilityLevel {
    /// Minimal observability - only final result and status
    Minimal,

    /// Standard observability - execution metrics and status per expert
    #[default]
    Standard,

    /// Verbose observability - full details including outputs and token usage
    Verbose,
}

/// Configuration for Conclave orchestration
///
/// Defines the behavior and settings for a Conclave execution including the retry count,
/// synthesis prompt customization, and observability level. The per-attempt timeout and the
/// backoff shape come from `battalion_config.aegis`: each expert and aggregator attempt is
/// bounded by `aegis.timeout`, and `aegis.retry` (or its default) shapes the wait between
/// retries while `retry_attempts` stays the retry count.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
/// use paladin_core::platform::container::battalion::conclave::{ConclaveConfig, ObservabilityLevel};
/// use paladin_core::platform::container::battalion::BattalionConfig;
/// use std::time::Duration;
///
/// // Each expert and aggregator attempt may run for at most ten minutes.
/// let battalion_config = BattalionConfig::new("expert_panel").with_aegis(Aegis {
///     timeout: Some(TimeoutPolicy {
///         run_timeout: Some(Duration::from_secs(600)),
///         idle_timeout: None,
///     }),
///     ..Aegis::default()
/// });
/// let mut config = ConclaveConfig::new("Analysis", battalion_config);
///
/// config = config
///     .with_retry_attempts(3)
///     .with_observability(ObservabilityLevel::Verbose)
///     .with_expert_names(true);
///
/// assert_eq!(config.retry_attempts, 3);
/// assert!(config.battalion_config.validate_aegis().is_ok());
/// ```
///
/// # Removed v0.10 key
///
/// A stored document that still carries the v0.10 whole-run timeout key is refused at
/// load: deserialization fails with an error that names the key, its replacement
/// (`battalion_config.aegis.timeout.run_timeout`) and `MIGRATION.md` row `M-B-06`, and
/// never echoes the value. A removed key nested inside `battalion_config` is refused
/// through [`BattalionConfig`] the same way. Unrelated unknown keys are still ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "super::legacy_keys::ConclaveConfigWire")]
pub struct ConclaveConfig {
    /// Name of the Conclave
    pub name: String,

    /// Base Battalion configuration
    pub battalion_config: BattalionConfig,

    /// Number of retry attempts for failed experts (0-5)
    pub retry_attempts: u32,

    /// Custom synthesis prompt template for aggregator
    ///
    /// If None, a default template will be used. The template can include
    /// placeholders that will be replaced with actual values:
    /// - `{original_task}`: The original input task
    /// - `{expert_outputs}`: Formatted expert outputs
    pub synthesis_prompt: Option<String>,

    /// Include expert names in aggregator prompt
    pub include_expert_names: bool,

    /// Maximum tokens per expert output (prevents excessive context)
    pub max_expert_output_tokens: Option<usize>,

    /// Observability level for logging and metrics
    pub observability_level: ObservabilityLevel,
}

impl ConclaveConfig {
    /// Create a new ConclaveConfig with default settings
    ///
    /// # Arguments
    ///
    /// * `name` - Name identifier for this Conclave
    /// * `battalion_config` - Base Battalion configuration
    ///
    /// # Default Values
    ///
    /// - `retry_attempts`: 2
    /// - `synthesis_prompt`: None (uses default template)
    /// - `include_expert_names`: true
    /// - `max_expert_output_tokens`: None (no limit)
    /// - `observability_level`: Standard
    pub fn new(name: impl Into<String>, battalion_config: BattalionConfig) -> Self {
        Self {
            name: name.into(),
            battalion_config,
            retry_attempts: 2,
            synthesis_prompt: None,
            include_expert_names: true,
            max_expert_output_tokens: None,
            observability_level: ObservabilityLevel::default(),
        }
    }

    /// Set retry attempts (builder pattern)
    ///
    /// Valid range: 0-5 attempts. Values outside this range will be clamped.
    pub fn with_retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts.min(5);
        self
    }

    /// Set custom synthesis prompt template (builder pattern)
    pub fn with_synthesis_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.synthesis_prompt = Some(prompt.into());
        self
    }

    /// Set whether to include expert names in aggregator prompt (builder pattern)
    pub fn with_expert_names(mut self, include: bool) -> Self {
        self.include_expert_names = include;
        self
    }

    /// Set maximum expert output tokens (builder pattern)
    pub fn with_max_expert_tokens(mut self, max_tokens: usize) -> Self {
        self.max_expert_output_tokens = Some(max_tokens);
        self
    }

    /// Set observability level (builder pattern)
    pub fn with_observability(mut self, level: ObservabilityLevel) -> Self {
        self.observability_level = level;
        self
    }
}

/// Conclave orchestration pattern
///
/// A Conclave executes multiple expert Paladins in parallel on the same task,
/// then synthesizes their outputs through an aggregator Paladin. This implements
/// the MixtureOfAgents pattern for high-quality response generation through
/// expert consensus.
///
/// # Minimum Requirements
///
/// - At least 2 expert Paladins
/// - Exactly 1 aggregator Paladin
/// - All Paladins must have unique names
///
/// # Example
///
/// ```ignore
/// use paladin_core::platform::container::battalion::conclave::Conclave;
///
/// let conclave = Conclave::new(
///     vec![expert1, expert2, expert3],
///     aggregator,
///     config,
/// )?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conclave {
    /// Expert Paladins that analyze the task in parallel
    pub experts: Vec<Paladin>,

    /// Aggregator Paladin that synthesizes expert outputs
    pub aggregator: Paladin,

    /// Configuration for this Conclave
    pub config: ConclaveConfig,
}

impl Conclave {
    /// Create a new Conclave with the given experts, aggregator, and configuration
    ///
    /// # Arguments
    ///
    /// * `experts` - Vector of expert Paladins (minimum 2)
    /// * `aggregator` - Aggregator Paladin for synthesis
    /// * `config` - Conclave configuration
    ///
    /// # Returns
    ///
    /// * `Ok(Conclave)` - Successfully created Conclave
    /// * `Err(BattalionError::ValidationError)` - If validation fails
    ///
    /// # Validation Rules
    ///
    /// - At least 2 expert Paladins required
    /// - All expert names must be unique
    /// - Aggregator name must be unique from all experts
    ///
    /// # Example
    ///
    /// ```ignore
    /// let conclave = Conclave::new(
    ///     vec![technical_expert, business_expert],
    ///     synthesizer,
    ///     config,
    /// )?;
    /// ```
    pub fn new(
        experts: Vec<Paladin>,
        aggregator: Paladin,
        config: ConclaveConfig,
    ) -> Result<Self, BattalionError> {
        let conclave = Self {
            experts,
            aggregator,
            config,
        };

        conclave.validate()?;
        Ok(conclave)
    }

    /// Validate the Conclave configuration
    ///
    /// Ensures:
    /// - At least 2 expert Paladins
    /// - All Paladin names are unique (including aggregator)
    /// - `retry_attempts` is within 0-5
    /// - The Battalion's Aegis policy is valid (`BattalionConfig::validate_aegis`)
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::ValidationError` if any validation rule fails.
    pub fn validate(&self) -> Result<(), BattalionError> {
        // Check minimum number of experts
        if self.experts.len() < 2 {
            return Err(BattalionError::ValidationError(format!(
                "Conclave requires at least 2 experts, found {}",
                self.experts.len()
            )));
        }

        // Check for duplicate names among experts
        let mut expert_names = std::collections::HashSet::new();
        for expert in &self.experts {
            let name = &expert.data.name;
            if !expert_names.insert(name.clone()) {
                return Err(BattalionError::ValidationError(format!(
                    "Duplicate expert name: '{}'",
                    name
                )));
            }
        }

        // Check aggregator name doesn't conflict with expert names
        let aggregator_name = &self.aggregator.data.name;
        if expert_names.contains(aggregator_name) {
            return Err(BattalionError::ValidationError(format!(
                "Aggregator name '{}' conflicts with an expert name",
                aggregator_name
            )));
        }

        // Validate retry attempts is within acceptable range
        if self.config.retry_attempts > 5 {
            return Err(BattalionError::ValidationError(format!(
                "Retry attempts must be 0-5, found {}",
                self.config.retry_attempts
            )));
        }

        // The per-attempt timeout and backoff policy live in `aegis`.
        self.config.battalion_config.validate_aegis()?;

        Ok(())
    }

    /// Get the name of this Conclave from configuration
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get the number of experts in this Conclave
    pub fn expert_count(&self) -> usize {
        self.experts.len()
    }
}

/// Status of Conclave execution
///
/// Indicates the overall outcome of a Conclave execution, taking into account
/// both expert execution results and aggregator success.
///
/// # Status Values
///
/// - **Success**: All experts succeeded and aggregator produced output
/// - **PartialSuccess**: Some experts failed but aggregator synthesized available outputs
/// - **Failed**: Critical failure (all experts failed or aggregator failed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConclaveStatus {
    /// All experts succeeded and aggregation completed successfully
    Success,

    /// Some experts failed but enough succeeded for aggregation to proceed
    PartialSuccess,

    /// Critical failure - either all experts failed or aggregator failed
    Failed,
}

/// Result of Conclave execution
///
/// Contains the outputs from all experts, the aggregated result, execution metrics,
/// and status information. This provides complete visibility into the Conclave execution
/// for analysis and debugging.
///
/// # Example
///
/// ```ignore
/// let result: ConclaveResult = conclave_service.execute(&conclave, input).await?;
///
/// println!("Status: {:?}", result.status);
/// println!("Total time: {}ms", result.execution_time_ms);
/// println!("Successful experts: {}/{}", result.successful_expert_count(), result.expert_outputs.len());
///
/// for (name, output) in &result.expert_outputs {
///     println!("Expert '{}': {}", name, output.content);
/// }
///
/// println!("Aggregated: {}", result.aggregated_output.content);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConclaveResult {
    /// Outputs from each expert, keyed by expert name
    pub expert_outputs: HashMap<String, PaladinResult>,

    /// Synthesized output from the aggregator
    pub aggregated_output: PaladinResult,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Execution time per expert in milliseconds
    pub expert_execution_times: HashMap<String, u64>,

    /// Number of retry attempts per expert
    pub retry_counts: HashMap<String, u32>,

    /// Overall execution status
    pub status: ConclaveStatus,
}

impl ConclaveResult {
    /// Count of experts that succeeded
    pub fn successful_expert_count(&self) -> usize {
        self.expert_outputs.len()
    }

    /// Check if all experts succeeded
    pub fn all_experts_succeeded(&self) -> bool {
        self.status == ConclaveStatus::Success
    }

    /// Check if execution completed (even with partial success)
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            ConclaveStatus::Success | ConclaveStatus::PartialSuccess
        )
    }
}

/// Error types specific to Conclave operations
///
/// These errors extend the base `BattalionError` with Conclave-specific failure modes.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConclaveError {
    /// All expert Paladins failed after retry attempts
    #[error("All experts failed after retries")]
    AllExpertsFailed,

    /// Aggregator Paladin failed to synthesize outputs
    #[error("Aggregator failed: {0}")]
    AggregatorFailed(String),

    /// Configuration error specific to Conclave
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Specific expert failed
    #[error("Expert '{0}' failed: {1}")]
    ExpertError(String, String),
}

// Conversion from ConclaveError to BattalionError for error propagation
impl From<ConclaveError> for BattalionError {
    fn from(error: ConclaveError) -> Self {
        match error {
            ConclaveError::AllExpertsFailed => {
                BattalionError::ValidationError("All experts failed".to_string())
            }
            ConclaveError::AggregatorFailed(msg) => BattalionError::AggregationError(msg),
            ConclaveError::ConfigurationError(msg) => BattalionError::ConfigurationError(msg),
            ConclaveError::ExpertError(name, msg) => {
                BattalionError::PaladinError(format!("Expert '{}': {}", name, msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::entity::Entity;
    use crate::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    fn create_test_paladin(name: &str) -> Paladin {
        let paladin_data = PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "TestUser".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        };

        Entity::new(paladin_data, Some(name.to_string()))
    }

    fn create_test_config() -> ConclaveConfig {
        let battalion_config = BattalionConfig::new("test_conclave");
        ConclaveConfig::new("TestConclave", battalion_config)
    }

    #[test]
    fn test_conclave_creation_success() {
        let experts = vec![
            create_test_paladin("Expert1"),
            create_test_paladin("Expert2"),
            create_test_paladin("Expert3"),
        ];
        let aggregator = create_test_paladin("Aggregator");
        let config = create_test_config();

        let result = Conclave::new(experts, aggregator, config);
        assert!(result.is_ok());

        let conclave = result.unwrap();
        assert_eq!(conclave.expert_count(), 3);
        assert_eq!(conclave.name(), "TestConclave");
    }

    #[test]
    fn test_conclave_insufficient_experts() {
        let experts = vec![create_test_paladin("Expert1")];
        let aggregator = create_test_paladin("Aggregator");
        let config = create_test_config();

        let result = Conclave::new(experts, aggregator, config);
        assert!(result.is_err());

        if let Err(BattalionError::ValidationError(msg)) = result {
            assert!(msg.contains("at least 2 experts"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_conclave_duplicate_expert_names() {
        let experts = vec![
            create_test_paladin("Expert1"),
            create_test_paladin("Expert1"), // Duplicate
            create_test_paladin("Expert3"),
        ];
        let aggregator = create_test_paladin("Aggregator");
        let config = create_test_config();

        let result = Conclave::new(experts, aggregator, config);
        assert!(result.is_err());

        if let Err(BattalionError::ValidationError(msg)) = result {
            assert!(msg.contains("Duplicate expert name"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_conclave_aggregator_name_conflicts() {
        let experts = vec![
            create_test_paladin("Expert1"),
            create_test_paladin("Expert2"),
        ];
        let aggregator = create_test_paladin("Expert1"); // Same name as expert
        let config = create_test_config();

        let result = Conclave::new(experts, aggregator, config);
        assert!(result.is_err());

        if let Err(BattalionError::ValidationError(msg)) = result {
            assert!(msg.contains("conflicts with an expert name"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_conclave_config_builder() {
        let battalion_config = BattalionConfig::new("test");
        let config = ConclaveConfig::new("TestConclave", battalion_config)
            .with_retry_attempts(3)
            .with_expert_names(false)
            .with_observability(ObservabilityLevel::Verbose)
            .with_synthesis_prompt("Custom prompt");

        assert_eq!(config.retry_attempts, 3);
        assert!(!config.include_expert_names);
        assert_eq!(config.observability_level, ObservabilityLevel::Verbose);
        assert!(config.synthesis_prompt.is_some());
    }

    #[test]
    fn test_conclave_config_retry_attempts_clamping() {
        let battalion_config = BattalionConfig::new("test");
        let config = ConclaveConfig::new("TestConclave", battalion_config).with_retry_attempts(10); // Exceeds maximum

        assert_eq!(config.retry_attempts, 5); // Should be clamped to 5
    }

    #[test]
    fn conclave_validate_delegates_to_validate_aegis() {
        use crate::platform::container::aegis::{Aegis, ErrorHandlerSpec};
        use crate::platform::container::battlefield::FieldName;
        use crate::platform::container::waypoint::NodeId;

        let experts = vec![
            create_test_paladin("Expert1"),
            create_test_paladin("Expert2"),
        ];
        let aggregator = create_test_paladin("Aggregator");

        // A Route handler is not supported on the Battalion patterns.
        let battalion_config = BattalionConfig::new("test").with_aegis(Aegis {
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: FieldName::new("failure").expect("valid field name"),
            }),
            ..Aegis::default()
        });
        let config = ConclaveConfig::new("TestConclave", battalion_config);
        match Conclave::new(experts.clone(), aggregator.clone(), config) {
            Err(BattalionError::ValidationError(msg)) => assert!(msg.contains("Route"), "{msg}"),
            other => panic!("Expected ValidationError, got {:?}", other.map(|_| ())),
        }

        // The retry_attempts <= 5 rule is kept.
        let mut config = create_test_config();
        config.retry_attempts = 6;
        assert!(matches!(
            Conclave::new(experts.clone(), aggregator.clone(), config),
            Err(BattalionError::ValidationError(_))
        ));

        // A valid policy is accepted.
        assert!(Conclave::new(experts, aggregator, create_test_config()).is_ok());
    }

    #[test]
    fn test_observability_level_default() {
        assert_eq!(ObservabilityLevel::default(), ObservabilityLevel::Standard);
    }

    #[test]
    fn test_conclave_status_equality() {
        assert_eq!(ConclaveStatus::Success, ConclaveStatus::Success);
        assert_ne!(ConclaveStatus::Success, ConclaveStatus::Failed);
        assert_ne!(ConclaveStatus::Success, ConclaveStatus::PartialSuccess);
    }

    #[test]
    fn test_conclave_error_conversion() {
        let conclave_error = ConclaveError::AllExpertsFailed;
        let battalion_error: BattalionError = conclave_error.into();

        match battalion_error {
            BattalionError::ValidationError(msg) => {
                assert!(msg.contains("All experts failed"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}
