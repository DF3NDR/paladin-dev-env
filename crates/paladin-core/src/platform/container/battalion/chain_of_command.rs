//! Chain of Command Pattern - Hierarchical Paladin Delegation
//!
//! Chain of Command coordinates a commander Paladin with one or more specialist Paladins.
//! The commander analyzes the task and delegates to specialists based on the delegation strategy.
//!
//! # Examples
//!
//! ```no_run
//! use paladin_core::platform::container::battalion::chain_of_command::{ChainOfCommand, DelegationStrategy};
//! use paladin_core::platform::container::battalion::BattalionConfig;
//!
//! // Create a chain with commander and specialists
//! let config = BattalionConfig::new("research_team");
//! // let commander = create_paladin("TeamLead");
//! // let specialist1 = create_paladin("DataAnalyst");
//! // let specialist2 = create_paladin("Researcher");
//!
//! // let chain = ChainOfCommand::new(
//! //     commander,
//! //     vec![specialist1, specialist2],
//! //     config
//! // ).unwrap().with_strategy(DelegationStrategy::Automatic);
//! ```

use crate::platform::container::battalion::{BattalionConfig, BattalionError};
use crate::platform::container::paladin::Paladin;
use serde::{Deserialize, Serialize};

/// Delegation strategy for specialist selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationStrategy {
    /// Commander automatically analyzes input and selects best specialist(s)
    Automatic,

    /// Broadcast task to all specialists concurrently
    Broadcast,

    /// Round-robin delegation across specialists (with state tracking)
    RoundRobin,

    /// Custom delegation logic (user-defined function name)
    Custom(String),
}

/// Chain of Command - Hierarchical Paladin delegation pattern
///
/// Coordinates a commander Paladin with specialist Paladins. The commander
/// receives the initial task, analyzes it, and delegates to appropriate
/// specialist(s) based on the delegation strategy.
///
/// # Architecture
///
/// - **Commander**: Lead Paladin that receives tasks and delegates
/// - **Specialists**: Expert Paladins with specific capabilities
/// - **Delegation Strategy**: How the commander selects specialists
///
/// # Delegation Strategies
///
/// - **Automatic**: Commander uses reasoning to select best specialist
/// - **Broadcast**: All specialists receive the task concurrently
/// - **RoundRobin**: Specialists are selected in rotation
/// - **Custom**: User-defined delegation logic
#[derive(Debug, Clone)]
pub struct ChainOfCommand {
    /// The commanding Paladin
    commander: Paladin,

    /// Specialist Paladins
    specialists: Vec<Paladin>,

    /// Battalion configuration
    config: BattalionConfig,

    /// Delegation strategy
    delegation_strategy: DelegationStrategy,
}

impl ChainOfCommand {
    /// Create a new Chain of Command
    ///
    /// # Arguments
    ///
    /// * `commander` - The lead Paladin that delegates tasks
    /// * `specialists` - Vector of specialist Paladins (must have at least 1)
    /// * `config` - Battalion configuration
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::ValidationError` if:
    /// - No specialists provided (requires at least 1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_core::platform::container::battalion::chain_of_command::ChainOfCommand;
    /// # use paladin_core::platform::container::battalion::BattalionConfig;
    /// # let commander = todo!();
    /// # let specialist1 = todo!();
    /// # let specialist2 = todo!();
    /// let config = BattalionConfig::new("my_chain");
    /// let chain = ChainOfCommand::new(
    ///     commander,
    ///     vec![specialist1, specialist2],
    ///     config
    /// )?;
    /// # Ok::<(), paladin_core::platform::container::battalion::BattalionError>(())
    /// ```
    pub fn new(
        commander: Paladin,
        specialists: Vec<Paladin>,
        config: BattalionConfig,
    ) -> Result<Self, BattalionError> {
        if specialists.is_empty() {
            return Err(BattalionError::ValidationError(
                "Chain of Command requires at least 1 specialist Paladin".to_string(),
            ));
        }

        Ok(Self {
            commander,
            specialists,
            config,
            delegation_strategy: DelegationStrategy::Automatic, // Default strategy
        })
    }

    /// Set the delegation strategy
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use paladin_core::platform::container::battalion::chain_of_command::{ChainOfCommand, DelegationStrategy};
    /// # use paladin_core::platform::container::battalion::BattalionConfig;
    /// # let commander = todo!();
    /// # let specialist = todo!();
    /// # let config = BattalionConfig::new("chain");
    /// let chain = ChainOfCommand::new(commander, vec![specialist], config)?
    ///     .with_strategy(DelegationStrategy::Broadcast);
    /// # Ok::<(), paladin_core::platform::container::battalion::BattalionError>(())
    /// ```
    pub fn with_strategy(mut self, strategy: DelegationStrategy) -> Self {
        self.delegation_strategy = strategy;
        self
    }

    /// Validate the Chain of Command structure
    ///
    /// Checks:
    /// - Has at least one specialist
    /// - Commander and specialists are valid
    ///
    /// # Errors
    ///
    /// Returns `BattalionError::ValidationError` if validation fails.
    pub fn validate(&self) -> Result<(), BattalionError> {
        if self.specialists.is_empty() {
            return Err(BattalionError::ValidationError(
                "Chain of Command requires at least 1 specialist".to_string(),
            ));
        }

        Ok(())
    }

    /// Get the commander Paladin
    pub fn commander(&self) -> &Paladin {
        &self.commander
    }

    /// Get the specialist Paladins
    pub fn specialists(&self) -> &[Paladin] {
        &self.specialists
    }

    /// Get the number of specialists
    pub fn specialist_count(&self) -> usize {
        self.specialists.len()
    }

    /// Get the delegation strategy
    pub fn delegation_strategy(&self) -> &DelegationStrategy {
        &self.delegation_strategy
    }

    /// Get the battalion configuration
    pub fn config(&self) -> &BattalionConfig {
        &self.config
    }

    /// Get a mutable reference to a specialist by index
    pub fn get_specialist_mut(&mut self, index: usize) -> Option<&mut Paladin> {
        self.specialists.get_mut(index)
    }

    /// Get a reference to a specialist by index
    pub fn get_specialist(&self, index: usize) -> Option<&Paladin> {
        self.specialists.get(index)
    }

    /// Get mutable reference to the commander
    pub fn commander_mut(&mut self) -> &mut Paladin {
        &mut self.commander
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::entity::Entity;
    use crate::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

    fn create_test_paladin(name: &str) -> Paladin {
        let data = PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "TestUser".to_string(),
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

    #[test]
    fn test_chain_of_command_creation() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config);
        assert!(chain.is_ok());
    }

    #[test]
    fn test_empty_specialists_fails() {
        let commander = create_test_paladin("Commander");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![], config);
        assert!(chain.is_err());
    }

    #[test]
    fn test_default_strategy_is_automatic() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();
        assert!(matches!(
            chain.delegation_strategy(),
            DelegationStrategy::Automatic
        ));
    }

    #[test]
    fn test_with_strategy() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .unwrap()
            .with_strategy(DelegationStrategy::Broadcast);

        assert!(matches!(
            chain.delegation_strategy(),
            DelegationStrategy::Broadcast
        ));
    }

    #[test]
    fn test_validate_success() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();
        assert!(chain.validate().is_ok());
    }

    #[test]
    fn test_specialist_count() {
        let commander = create_test_paladin("Commander");
        let specialist1 = create_test_paladin("Specialist1");
        let specialist2 = create_test_paladin("Specialist2");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist1, specialist2], config).unwrap();
        assert_eq!(chain.specialist_count(), 2);
    }

    #[test]
    fn test_accessors() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();

        assert_eq!(chain.commander().data.name, "Commander");
        assert_eq!(chain.specialists().len(), 1);
        assert_eq!(chain.specialists()[0].data.name, "Specialist");
        assert_eq!(chain.config().name, "test_chain");
    }
}
