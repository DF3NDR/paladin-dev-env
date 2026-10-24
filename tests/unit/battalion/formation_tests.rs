//! Unit tests for Formation domain entity
//!
//! Tests the Formation struct and its behavior following TDD methodology.

use paladin::core::base::entity::Entity;
use paladin::core::platform::container::battalion::formation::Formation;
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
mod formation_construction_tests {
    use super::*;

    #[test]
    fn test_formation_new_with_valid_paladins() {
        let paladin1 = create_test_paladin("Paladin1");
        let paladin2 = create_test_paladin("Paladin2");

        let paladins = vec![paladin1, paladin2];
        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_ok());

        let formation = result.unwrap();
        assert_eq!(formation.paladins.len(), 2);
        assert_eq!(formation.config.name, "test_formation");
    }

    #[test]
    fn test_formation_new_with_single_paladin_succeeds() {
        let paladin = create_test_paladin("Paladin1");
        let paladins = vec![paladin];
        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_ok());

        let formation = result.unwrap();
        assert_eq!(formation.paladins.len(), 1);
    }

    #[test]
    fn test_formation_new_with_empty_paladins_fails() {
        let paladins: Vec<Paladin> = vec![];
        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_err());

        match result.unwrap_err() {
            BattalionError::ValidationError(msg) => {
                assert!(msg.contains("at least 1 Paladin"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}

#[cfg(test)]
mod formation_builder_tests {
    use super::*;

    #[test]
    fn test_formation_with_config() {
        let paladin1 = create_test_paladin("Paladin1");
        let paladin2 = create_test_paladin("Paladin2");
        let paladins = vec![paladin1, paladin2];

        let custom_config = BattalionConfig::new("custom_formation");

        let formation = Formation::new(paladins, custom_config).unwrap();
        assert_eq!(formation.config.name, "custom_formation");
    }

    #[test]
    fn test_formation_with_shared_context() {
        let paladin1 = create_test_paladin("Paladin1");
        let paladin2 = create_test_paladin("Paladin2");
        let paladins = vec![paladin1, paladin2];
        let config = BattalionConfig::new("test_formation");

        let mut formation = Formation::new(paladins, config).unwrap();

        formation = formation.with_shared_context("Shared context information".to_string());
        assert!(formation.shared_context.is_some());
        assert_eq!(
            formation.shared_context.unwrap(),
            "Shared context information"
        );
    }

    #[test]
    fn test_formation_builder_chain() {
        let paladin1 = create_test_paladin("Paladin1");
        let paladin2 = create_test_paladin("Paladin2");
        let paladin3 = create_test_paladin("Paladin3");
        let paladins = vec![paladin1, paladin2, paladin3];

        let config = BattalionConfig::new("chained_formation")
            .with_description("Test formation with chained builders");

        let formation = Formation::new(paladins, config)
            .unwrap()
            .with_shared_context("Global context for all Paladins".to_string());

        assert_eq!(formation.paladins.len(), 3);
        assert!(formation.shared_context.is_some());
    }
}

#[cfg(test)]
mod formation_validation_tests {
    use super::*;

    #[test]
    fn test_validate_requires_minimum_paladins() {
        let paladins: Vec<Paladin> = vec![];
        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_accepts_two_paladins() {
        let paladin1 = create_test_paladin("Paladin1");
        let paladin2 = create_test_paladin("Paladin2");
        let paladins = vec![paladin1, paladin2];
        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_accepts_many_paladins() {
        let paladins: Vec<Paladin> = (1..=10)
            .map(|i| create_test_paladin(&format!("Paladin{}", i)))
            .collect();

        let config = BattalionConfig::new("test_formation");

        let result = Formation::new(paladins, config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().paladins.len(), 10);
    }
}
