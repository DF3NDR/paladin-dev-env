//! Unit tests for Chain of Command domain entity
//!
//! Tests the ChainOfCommand struct and its behavior following TDD methodology.

use paladin::core::base::entity::Entity;
use paladin::core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};

/// Helper function to create a test Paladin
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

#[cfg(test)]
mod chain_of_command_construction_tests {
    use super::*;

    #[test]
    fn test_chain_of_command_new_with_valid_setup() {
        let commander = create_test_paladin("Commander");
        let specialist1 = create_test_paladin("Specialist1");
        let specialist2 = create_test_paladin("Specialist2");

        let specialists = vec![specialist1, specialist2];
        let config = BattalionConfig::new("test_chain");

        let result = ChainOfCommand::new(commander, specialists, config);
        assert!(result.is_ok());

        let chain = result.unwrap();
        assert_eq!(chain.specialists().len(), 2);
        assert_eq!(chain.config().name, "test_chain");
        assert_eq!(chain.commander().data.name, "Commander");
    }

    #[test]
    fn test_chain_of_command_new_with_no_specialists_fails() {
        let commander = create_test_paladin("Commander");
        let specialists: Vec<Paladin> = vec![];
        let config = BattalionConfig::new("test_chain");

        let result = ChainOfCommand::new(commander, specialists, config);
        assert!(result.is_err());

        match result.unwrap_err() {
            BattalionError::ValidationError(msg) => {
                assert!(msg.contains("at least 1 specialist"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_chain_of_command_with_single_specialist_succeeds() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let specialists = vec![specialist];
        let config = BattalionConfig::new("test_chain");

        let result = ChainOfCommand::new(commander, specialists, config);
        assert!(result.is_ok());

        let chain = result.unwrap();
        assert_eq!(chain.specialists().len(), 1);
    }
}

#[cfg(test)]
mod delegation_strategy_tests {
    use super::*;

    #[test]
    fn test_delegation_strategy_automatic() {
        let strategy = DelegationStrategy::Automatic;
        assert!(matches!(strategy, DelegationStrategy::Automatic));
    }

    #[test]
    fn test_delegation_strategy_broadcast() {
        let strategy = DelegationStrategy::Broadcast;
        assert!(matches!(strategy, DelegationStrategy::Broadcast));
    }

    #[test]
    fn test_delegation_strategy_round_robin() {
        let strategy = DelegationStrategy::RoundRobin;
        assert!(matches!(strategy, DelegationStrategy::RoundRobin));
    }

    #[test]
    fn test_delegation_strategy_custom() {
        let strategy = DelegationStrategy::Custom("my_custom_delegator".to_string());
        match strategy {
            DelegationStrategy::Custom(name) => {
                assert_eq!(name, "my_custom_delegator");
            }
            _ => panic!("Expected Custom strategy"),
        }
    }

    #[test]
    fn test_delegation_strategy_clone() {
        let strategy1 = DelegationStrategy::Automatic;
        let strategy2 = strategy1.clone();
        assert!(matches!(strategy2, DelegationStrategy::Automatic));
    }

    #[test]
    fn test_delegation_strategy_debug() {
        let strategy = DelegationStrategy::Broadcast;
        let debug_str = format!("{:?}", strategy);
        assert!(debug_str.contains("Broadcast"));
    }
}

#[cfg(test)]
mod chain_of_command_builder_tests {
    use super::*;

    #[test]
    fn test_with_strategy_automatic() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .unwrap()
            .with_strategy(DelegationStrategy::Automatic);

        assert!(matches!(
            chain.delegation_strategy(),
            &DelegationStrategy::Automatic
        ));
    }

    #[test]
    fn test_with_strategy_broadcast() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .unwrap()
            .with_strategy(DelegationStrategy::Broadcast);

        assert!(matches!(
            chain.delegation_strategy(),
            &DelegationStrategy::Broadcast
        ));
    }

    #[test]
    fn test_with_strategy_round_robin() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .unwrap()
            .with_strategy(DelegationStrategy::RoundRobin);

        assert!(matches!(
            chain.delegation_strategy(),
            &DelegationStrategy::RoundRobin
        ));
    }

    #[test]
    fn test_with_strategy_custom() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config)
            .unwrap()
            .with_strategy(DelegationStrategy::Custom("expert_selector".to_string()));

        match chain.delegation_strategy() {
            DelegationStrategy::Custom(name) => {
                assert_eq!(name, "expert_selector");
            }
            _ => panic!("Expected Custom strategy"),
        }
    }
}

#[cfg(test)]
mod chain_of_command_validation_tests {
    use super::*;

    #[test]
    fn test_validate_with_valid_chain_succeeds() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();
        let result = chain.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_multiple_specialists_succeeds() {
        let commander = create_test_paladin("Commander");
        let specialist1 = create_test_paladin("Specialist1");
        let specialist2 = create_test_paladin("Specialist2");
        let specialist3 = create_test_paladin("Specialist3");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(
            commander,
            vec![specialist1, specialist2, specialist3],
            config,
        )
        .unwrap();
        let result = chain.validate();
        assert!(result.is_ok());
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
    fn test_commander_access() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();
        assert_eq!(chain.commander().data.name, "Commander");
    }

    #[test]
    fn test_specialists_access() {
        let commander = create_test_paladin("Commander");
        let specialist1 = create_test_paladin("Specialist1");
        let specialist2 = create_test_paladin("Specialist2");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist1, specialist2], config).unwrap();
        let specialists = chain.specialists();
        assert_eq!(specialists.len(), 2);
        assert_eq!(specialists[0].data.name, "Specialist1");
        assert_eq!(specialists[1].data.name, "Specialist2");
    }
}

#[cfg(test)]
mod chain_of_command_default_strategy_tests {
    use super::*;

    #[test]
    fn test_default_strategy_is_automatic() {
        let commander = create_test_paladin("Commander");
        let specialist = create_test_paladin("Specialist1");
        let config = BattalionConfig::new("test_chain");

        let chain = ChainOfCommand::new(commander, vec![specialist], config).unwrap();
        assert!(matches!(
            chain.delegation_strategy(),
            &DelegationStrategy::Automatic
        ));
    }
}
