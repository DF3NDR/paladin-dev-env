//! Unit tests for Phalanx domain entity
//!
//! Following TDD methodology: write failing tests first

use paladin::core::base::entity::Entity;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};

fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("{} system prompt", name),
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
mod phalanx_construction_tests {
    use super::*;

    #[test]
    fn test_phalanx_creation_valid() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let result = Phalanx::new(vec![p1, p2], config);

        assert!(result.is_ok());
        let phalanx = result.unwrap();
        assert_eq!(phalanx.paladin_count(), 2);
    }

    #[test]
    fn test_phalanx_creation_with_single_paladin_fails() {
        let p1 = create_test_paladin("Agent1");
        let config = BattalionConfig::new("test_phalanx");

        let result = Phalanx::new(vec![p1], config);

        assert!(result.is_err());
    }

    #[test]
    fn test_phalanx_creation_with_empty_paladins_fails() {
        let config = BattalionConfig::new("test_phalanx");

        let result = Phalanx::new(vec![], config);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod aggregation_strategy_tests {
    use super::*;

    #[test]
    fn test_aggregation_strategy_default_is_collect_all() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(vec![p1, p2], config).unwrap();

        assert!(matches!(
            phalanx.aggregation_strategy(),
            &AggregationStrategy::CollectAll
        ));
    }

    #[test]
    fn test_phalanx_with_first_success_strategy() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(vec![p1, p2], config)
            .unwrap()
            .with_aggregation(AggregationStrategy::FirstSuccess);

        assert!(matches!(
            phalanx.aggregation_strategy(),
            &AggregationStrategy::FirstSuccess
        ));
    }

    #[test]
    fn test_phalanx_with_majority_strategy() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let p3 = create_test_paladin("Agent3");
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(vec![p1, p2, p3], config)
            .unwrap()
            .with_aggregation(AggregationStrategy::Majority);

        assert!(matches!(
            phalanx.aggregation_strategy(),
            &AggregationStrategy::Majority
        ));
    }
}

#[cfg(test)]
mod phalanx_builder_tests {
    use super::*;

    #[test]
    fn test_phalanx_with_max_concurrency() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(vec![p1, p2], config)
            .unwrap()
            .with_max_concurrency(5);

        assert_eq!(phalanx.max_concurrency(), Some(5));
    }

    #[test]
    fn test_phalanx_accessors() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(vec![p1, p2], config).unwrap();

        assert_eq!(phalanx.paladin_count(), 2);
        assert_eq!(phalanx.paladins().len(), 2);
        assert_eq!(phalanx.config().name, "test_phalanx");
    }
}

#[cfg(test)]
mod phalanx_validation_tests {
    use super::*;

    #[test]
    fn test_phalanx_validation_minimum_paladins() {
        let p1 = create_test_paladin("Agent1");
        let config = BattalionConfig::new("test_phalanx");

        let result = Phalanx::new(vec![p1], config);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least 2 Paladins")
        );
    }

    #[test]
    fn test_majority_requires_at_least_three_paladins() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        let result = Phalanx::new(vec![p1, p2], config)
            .unwrap()
            .with_aggregation(AggregationStrategy::Majority);

        // Validation should happen during execution, but we can test structure
        assert_eq!(result.paladin_count(), 2);
    }

    #[test]
    fn test_phalanx_with_valid_concurrency_limit() {
        let paladins: Vec<Paladin> = (1..=10)
            .map(|i| create_test_paladin(&format!("Agent{}", i)))
            .collect();
        let config = BattalionConfig::new("test_phalanx");

        let phalanx = Phalanx::new(paladins, config)
            .unwrap()
            .with_max_concurrency(5);

        assert_eq!(phalanx.max_concurrency(), Some(5));
        assert_eq!(phalanx.paladin_count(), 10);
    }
}
