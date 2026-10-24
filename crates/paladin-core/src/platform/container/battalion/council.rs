//! Council Pattern - Conversational Multi-Agent Collaboration
//!
//! This module implements the Council pattern for multi-agent orchestration where
//! multiple Paladins engage in turn-based conversations to collaboratively solve
//! complex problems through structured dialogue.
//!
//! # Overview
//!
//! A Council coordinates multiple Paladins in a discussion format, allowing them to:
//! - Exchange ideas and perspectives
//! - Build upon each other's contributions
//! - Reach consensus through dialogue
//! - Leverage collective intelligence
//!
//! # Example
//!
//! ```ignore
//! use paladin_core::platform::container::battalion::council::{
//!     CouncilBuilder, TurnStrategy, TerminationCondition
//! };
//!
//! let council = CouncilBuilder::new()
//!     .name("Security Review Panel")
//!     .add_participant("security_expert")
//!     .add_participant("compliance_officer")
//!     .add_participant("dev_lead")
//!     .max_rounds(5)
//!     .turn_strategy(TurnStrategy::RoundRobin)
//!     .termination_condition(TerminationCondition::MaxRounds)
//!     .build()?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BattalionError, CouncilError};

/// Turn-taking strategy for Council discussions
///
/// Determines how the Council decides which Paladin speaks next during
/// the conversation.
///
/// # Strategies
///
/// - **RoundRobin**: Participants take turns in sequence (simple, fair)
/// - **ModeratorDirected**: Moderator decides who speaks next (flexible, controlled)
/// - **Random**: Random selection of next speaker (unpredictable, diverse)
/// - **VoluntaryWithTimeout**: Paladins volunteer to speak, timeout if none volunteer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnStrategy {
    /// Participants take turns in sequence, cycling through the list
    RoundRobin,

    /// Moderator Paladin selects who speaks next by including the name in response
    ModeratorDirected,

    /// Random selection from available participants
    Random,

    /// Paladins volunteer to speak, with a timeout if no one volunteers
    ///
    /// # Fields
    ///
    /// * `timeout_ms` - Maximum time to wait for a volunteer (milliseconds)
    VoluntaryWithTimeout {
        /// Timeout in milliseconds
        timeout_ms: u64,
    },
}

impl Default for TurnStrategy {
    /// Default turn strategy is RoundRobin
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Termination condition for Council discussions
///
/// Determines when the Council discussion should conclude.
///
/// # Conditions
///
/// - **MaxRounds**: Stop after a fixed number of rounds
/// - **Consensus**: Detect consensus through keyword analysis
/// - **ModeratorDecision**: Moderator explicitly ends the discussion
/// - **Keyword**: Custom keyword triggers termination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationCondition {
    /// Stop after reaching maximum number of rounds
    MaxRounds,

    /// Detect consensus through keyword matching (e.g., "I agree", "consensus reached")
    Consensus,

    /// Moderator decides when to end (e.g., says "discussion concluded")
    ModeratorDecision,

    /// Custom keyword triggers termination
    Keyword(String),
}

impl Default for TerminationCondition {
    /// Default termination is MaxRounds
    fn default() -> Self {
        Self::MaxRounds
    }
}

/// A single message in a Council conversation
///
/// Represents one turn in the discussion, capturing who said what and when.
///
/// # Fields
///
/// * `speaker` - Paladin ID/name of who spoke
/// * `content` - The message content
/// * `round` - Which discussion round this occurred in
/// * `timestamp` - When this message was created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilMessage {
    /// Paladin ID or name of the speaker
    pub speaker: String,

    /// Content of the message
    pub content: String,

    /// Round number (1-indexed)
    pub round: u32,

    /// When this message was created
    pub timestamp: DateTime<Utc>,
}

impl CouncilMessage {
    /// Create a new CouncilMessage
    ///
    /// # Arguments
    ///
    /// * `speaker` - Paladin ID or name
    /// * `content` - Message content
    /// * `round` - Round number
    ///
    /// # Example
    ///
    /// ```ignore
    /// let message = CouncilMessage::new("expert_1", "I believe we should...", 1);
    /// ```
    pub fn new(speaker: impl Into<String>, content: impl Into<String>, round: u32) -> Self {
        Self {
            speaker: speaker.into(),
            content: content.into(),
            round,
            timestamp: Utc::now(),
        }
    }

    /// Format the message for display
    pub fn format(&self) -> String {
        format!("[Round {}] {}: {}", self.round, self.speaker, self.content)
    }
}

/// Configuration for Council execution
///
/// Controls how the Council discussion proceeds, including turn-taking,
/// termination, and conversation history management.
///
/// # Example
///
/// ```
/// use paladin_core::platform::container::battalion::council::{
///     CouncilConfig, TurnStrategy, TerminationCondition
/// };
///
/// let config = CouncilConfig {
///     max_rounds: 5,
///     turn_strategy: TurnStrategy::RoundRobin,
///     termination_condition: TerminationCondition::MaxRounds,
///     include_history: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilConfig {
    /// Maximum number of rounds before forced termination
    pub max_rounds: u32,

    /// Strategy for determining who speaks next
    pub turn_strategy: TurnStrategy,

    /// Condition that triggers discussion termination
    pub termination_condition: TerminationCondition,

    /// Whether to include conversation history in each Paladin's context
    pub include_history: bool,
}

impl Default for CouncilConfig {
    /// Create CouncilConfig with sensible defaults
    ///
    /// # Default Values
    ///
    /// - `max_rounds`: 10
    /// - `turn_strategy`: RoundRobin
    /// - `termination_condition`: MaxRounds
    /// - `include_history`: true
    fn default() -> Self {
        Self {
            max_rounds: 10,
            turn_strategy: TurnStrategy::default(),
            termination_condition: TerminationCondition::default(),
            include_history: true,
        }
    }
}

/// Council domain entity data
///
/// Contains the core data for a Council without the Entity wrapper.
/// This follows the pattern used by other domain entities in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouncilData {
    /// Name of the Council
    pub name: String,

    /// IDs of participant Paladins
    pub participant_ids: Vec<String>,

    /// Optional moderator Paladin ID
    pub moderator_id: Option<String>,

    /// Council configuration
    pub config: CouncilConfig,
}

/// Council - Conversational multi-agent collaboration pattern
///
/// A Council coordinates multiple Paladins in structured discussions where they
/// take turns contributing to a conversation, building on each other's ideas
/// to solve complex problems or reach consensus.
///
/// # Architecture
///
/// Council uses the `Entity<CouncilData>` pattern for consistency with other
/// domain entities. The actual execution logic is in the application layer.
///
/// # Validation Rules
///
/// - At least 2 participants required
/// - Moderator required if using ModeratorDirected strategy
/// - max_rounds must be > 0
///
/// # Example
///
/// ```ignore
/// use paladin_core::platform::container::battalion::council::CouncilBuilder;
///
/// let council = CouncilBuilder::new()
///     .name("Expert Panel")
///     .add_participant("expert_1")
///     .add_participant("expert_2")
///     .add_participant("expert_3")
///     .max_rounds(5)
///     .build()?;
/// ```
pub type Council = crate::base::entity::Entity<CouncilData>;

/// Builder for creating Council instances
///
/// Provides a fluent interface for constructing and validating Council
/// configurations before use.
///
/// # Examples
///
/// ```ignore
/// let council = CouncilBuilder::new()
///     .name("Security Review")
///     .add_participant("security_expert")
///     .add_participant("compliance_officer")
///     .moderator("facilitator")
///     .max_rounds(5)
///     .turn_strategy(TurnStrategy::ModeratorDirected)
///     .build()?;
/// ```
#[derive(Debug)]
pub struct CouncilBuilder {
    name: Option<String>,
    participant_ids: Vec<String>,
    moderator_id: Option<String>,
    config: CouncilConfig,
}

impl CouncilBuilder {
    /// Create a new CouncilBuilder with default configuration
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_core::platform::container::battalion::council::CouncilBuilder;
    ///
    /// let builder = CouncilBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            name: None,
            participant_ids: Vec::new(),
            moderator_id: None,
            config: CouncilConfig::default(),
        }
    }

    /// Set the Council name
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the Council
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a participant Paladin by ID
    ///
    /// # Arguments
    ///
    /// * `paladin_id` - Unique identifier for the Paladin
    pub fn add_participant(mut self, paladin_id: impl Into<String>) -> Self {
        self.participant_ids.push(paladin_id.into());
        self
    }

    /// Set the moderator Paladin ID
    ///
    /// # Arguments
    ///
    /// * `paladin_id` - Unique identifier for the moderator Paladin
    pub fn moderator(mut self, paladin_id: impl Into<String>) -> Self {
        self.moderator_id = Some(paladin_id.into());
        self
    }

    /// Set the complete configuration
    ///
    /// # Arguments
    ///
    /// * `config` - CouncilConfig instance
    pub fn config(mut self, config: CouncilConfig) -> Self {
        self.config = config;
        self
    }

    /// Set maximum rounds
    ///
    /// # Arguments
    ///
    /// * `rounds` - Maximum number of discussion rounds
    pub fn max_rounds(mut self, rounds: u32) -> Self {
        self.config.max_rounds = rounds;
        self
    }

    /// Set turn-taking strategy
    ///
    /// # Arguments
    ///
    /// * `strategy` - TurnStrategy to use
    pub fn turn_strategy(mut self, strategy: TurnStrategy) -> Self {
        self.config.turn_strategy = strategy;
        self
    }

    /// Set termination condition
    ///
    /// # Arguments
    ///
    /// * `condition` - TerminationCondition to use
    pub fn termination_condition(mut self, condition: TerminationCondition) -> Self {
        self.config.termination_condition = condition;
        self
    }

    /// Set whether to include conversation history
    ///
    /// # Arguments
    ///
    /// * `include` - true to include history in each Paladin's context
    pub fn include_history(mut self, include: bool) -> Self {
        self.config.include_history = include;
        self
    }

    /// Build and validate the Council
    ///
    /// # Returns
    ///
    /// * `Ok(Council)` - Successfully created and validated Council
    /// * `Err(BattalionError)` - Validation failed
    ///
    /// # Validation Rules
    ///
    /// 1. Name must be provided
    /// 2. At least 2 participants required
    /// 3. Moderator required if using ModeratorDirected strategy
    /// 4. max_rounds must be > 0
    ///
    /// # Example
    ///
    /// ```ignore
    /// let council = CouncilBuilder::new()
    ///     .name("Panel")
    ///     .add_participant("expert_1")
    ///     .add_participant("expert_2")
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<Council, BattalionError> {
        // Validation
        let name = self.name.ok_or_else(|| {
            BattalionError::ValidationError("Council name is required".to_string())
        })?;

        if self.participant_ids.is_empty() {
            return Err(CouncilError::NoParticipants.into());
        }

        if self.participant_ids.len() < 2 {
            return Err(BattalionError::ValidationError(
                "Council requires at least 2 participants for meaningful discussion".to_string(),
            ));
        }

        // Validate moderator requirement for ModeratorDirected strategy
        if matches!(self.config.turn_strategy, TurnStrategy::ModeratorDirected)
            && self.moderator_id.is_none()
        {
            return Err(CouncilError::ModeratorRequired.into());
        }

        if self.config.max_rounds == 0 {
            return Err(CouncilError::InvalidMaxRounds.into());
        }

        // Create CouncilData
        let data = CouncilData {
            name: name.clone(),
            participant_ids: self.participant_ids,
            moderator_id: self.moderator_id,
            config: self.config,
        };

        // Create the Council entity
        Ok(crate::base::entity::Entity::new(data, Some(name)))
    }
}

impl Default for CouncilBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_strategy_default() {
        let strategy = TurnStrategy::default();
        assert_eq!(strategy, TurnStrategy::RoundRobin);
    }

    #[test]
    fn test_termination_condition_default() {
        let condition = TerminationCondition::default();
        assert_eq!(condition, TerminationCondition::MaxRounds);
    }

    #[test]
    fn test_council_config_default() {
        let config = CouncilConfig::default();
        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.turn_strategy, TurnStrategy::RoundRobin);
        assert_eq!(
            config.termination_condition,
            TerminationCondition::MaxRounds
        );
        assert!(config.include_history);
    }

    #[test]
    fn test_council_message_creation() {
        let message = CouncilMessage::new("expert_1", "Hello", 1);
        assert_eq!(message.speaker, "expert_1");
        assert_eq!(message.content, "Hello");
        assert_eq!(message.round, 1);
    }

    #[test]
    fn test_council_message_format() {
        let message = CouncilMessage::new("expert_1", "Test message", 2);
        let formatted = message.format();
        assert!(formatted.contains("Round 2"));
        assert!(formatted.contains("expert_1"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_council_builder_basic() {
        let result = CouncilBuilder::new()
            .name("Test Council")
            .add_participant("p1")
            .add_participant("p2")
            .build();

        assert!(result.is_ok());
        let council = result.unwrap();
        assert_eq!(council.data.name, "Test Council");
        assert_eq!(council.data.participant_ids.len(), 2);
    }

    #[test]
    fn test_council_builder_validation_no_name() {
        let result = CouncilBuilder::new()
            .add_participant("p1")
            .add_participant("p2")
            .build();

        assert!(result.is_err());
        match result {
            Err(BattalionError::ValidationError(msg)) => {
                assert!(msg.contains("name is required"));
            }
            _ => panic!("Expected ValidationError for missing name"),
        }
    }

    #[test]
    fn test_council_builder_validation_too_few_participants() {
        let result = CouncilBuilder::new()
            .name("Test")
            .add_participant("p1")
            .build();

        assert!(result.is_err());
        match result {
            Err(BattalionError::ValidationError(msg)) => {
                assert!(msg.contains("at least 2 participants"));
            }
            _ => panic!("Expected ValidationError for too few participants"),
        }
    }

    #[test]
    fn test_council_builder_validation_moderator_required() {
        let result = CouncilBuilder::new()
            .name("Test")
            .add_participant("p1")
            .add_participant("p2")
            .turn_strategy(TurnStrategy::ModeratorDirected)
            .build();

        assert!(result.is_err());
        match result {
            Err(BattalionError::CouncilError(CouncilError::ModeratorRequired)) => {
                // Expected error type
            }
            other => panic!("Expected CouncilError::ModeratorRequired, got {:?}", other),
        }
    }

    #[test]
    fn test_council_builder_with_moderator() {
        let result = CouncilBuilder::new()
            .name("Test")
            .add_participant("p1")
            .add_participant("p2")
            .moderator("mod1")
            .turn_strategy(TurnStrategy::ModeratorDirected)
            .build();

        assert!(result.is_ok());
        let council = result.unwrap();
        assert_eq!(council.data.moderator_id, Some("mod1".to_string()));
    }

    #[test]
    fn test_council_builder_fluent_interface() {
        let result = CouncilBuilder::new()
            .name("Expert Panel")
            .add_participant("expert_1")
            .add_participant("expert_2")
            .add_participant("expert_3")
            .max_rounds(5)
            .turn_strategy(TurnStrategy::RoundRobin)
            .termination_condition(TerminationCondition::Consensus)
            .include_history(false)
            .build();

        assert!(result.is_ok());
        let council = result.unwrap();
        assert_eq!(council.data.name, "Expert Panel");
        assert_eq!(council.data.participant_ids.len(), 3);
        assert_eq!(council.data.config.max_rounds, 5);
        assert_eq!(council.data.config.turn_strategy, TurnStrategy::RoundRobin);
        assert_eq!(
            council.data.config.termination_condition,
            TerminationCondition::Consensus
        );
        assert!(!council.data.config.include_history);
    }

    #[test]
    fn test_turn_strategy_serialization() {
        let strategy = TurnStrategy::VoluntaryWithTimeout { timeout_ms: 5000 };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: TurnStrategy = serde_json::from_str(&json).unwrap();

        match deserialized {
            TurnStrategy::VoluntaryWithTimeout { timeout_ms } => {
                assert_eq!(timeout_ms, 5000);
            }
            _ => panic!("Expected VoluntaryWithTimeout"),
        }
    }

    #[test]
    fn test_termination_condition_serialization() {
        let condition = TerminationCondition::Keyword("DONE".to_string());
        let json = serde_json::to_string(&condition).unwrap();
        let deserialized: TerminationCondition = serde_json::from_str(&json).unwrap();

        match deserialized {
            TerminationCondition::Keyword(keyword) => {
                assert_eq!(keyword, "DONE");
            }
            _ => panic!("Expected Keyword termination"),
        }
    }
}
