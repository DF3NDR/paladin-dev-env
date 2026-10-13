//! Campaign Pattern Unit Tests
//!
//! Tests for the Campaign domain entity - graph-based Paladin orchestration using DAG.

use paladin::core::base::entity::Entity;
use paladin::core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin::core::platform::container::battalion::{BattalionConfig, BattalionError};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};

/// Helper function to create a test Paladin
fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Entity::new(data, Some(name.to_string()))
}

#[cfg(test)]
mod campaign_construction_tests {
    use super::*;

    #[test]
    fn test_campaign_new_with_valid_config() {
        // Arrange
        let config =
            BattalionConfig::new("test_campaign").with_description("Test campaign description");

        // Act
        let campaign = Campaign::new(config.clone());

        // Assert
        assert_eq!(campaign.config().name, "test_campaign");
        assert_eq!(
            campaign.config().description,
            Some("Test campaign description".to_string())
        );
        assert_eq!(campaign.paladin_count(), 0);
        assert_eq!(campaign.edge_count(), 0);
    }

    #[test]
    fn test_campaign_add_paladin() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let paladin = create_test_paladin("Paladin1");

        // Act
        let paladin_id = campaign.add_paladin(paladin);

        // Assert
        assert_eq!(campaign.paladin_count(), 1);
        assert!(campaign.has_paladin(&paladin_id));
    }

    #[test]
    fn test_campaign_add_multiple_paladins() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);

        // Act
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));
        let id3 = campaign.add_paladin(create_test_paladin("Paladin3"));

        // Assert
        assert_eq!(campaign.paladin_count(), 3);
        assert!(campaign.has_paladin(&id1));
        assert!(campaign.has_paladin(&id2));
        assert!(campaign.has_paladin(&id3));
    }
}

#[cfg(test)]
mod edge_condition_tests {
    use super::*;

    #[test]
    fn test_edge_condition_always() {
        // Arrange
        let condition = EdgeCondition::Always;

        // Assert - Always should be the default and simplest condition
        assert!(matches!(condition, EdgeCondition::Always));
    }

    #[test]
    fn test_edge_condition_contains() {
        // Arrange
        let condition = EdgeCondition::Contains("success".to_string());

        // Assert
        if let EdgeCondition::Contains(pattern) = condition {
            assert_eq!(pattern, "success");
        } else {
            panic!("Expected Contains variant");
        }
    }

    #[test]
    fn test_edge_condition_regex() {
        // Arrange
        let condition = EdgeCondition::Regex("^success.*".to_string());

        // Assert
        if let EdgeCondition::Regex(pattern) = condition {
            assert_eq!(pattern, "^success.*");
        } else {
            panic!("Expected Regex variant");
        }
    }

    #[test]
    fn test_edge_condition_custom() {
        // Arrange
        let condition = EdgeCondition::Custom("custom_logic".to_string());

        // Assert
        if let EdgeCondition::Custom(name) = condition {
            assert_eq!(name, "custom_logic");
        } else {
            panic!("Expected Custom variant");
        }
    }
}

#[cfg(test)]
mod campaign_edge_tests {
    use super::*;

    #[test]
    fn test_campaign_add_edge_with_always_condition() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));

        // Act
        let edge = CampaignEdge::new(id1, id2, EdgeCondition::Always);
        campaign.add_edge(edge).expect("Failed to add edge");

        // Assert
        assert_eq!(campaign.edge_count(), 1);
    }

    #[test]
    fn test_campaign_add_edge_with_contains_condition() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));

        // Act
        let edge = CampaignEdge::new(id1, id2, EdgeCondition::Contains("success".to_string()));
        campaign.add_edge(edge).expect("Failed to add edge");

        // Assert
        assert_eq!(campaign.edge_count(), 1);
    }

    #[test]
    fn test_campaign_add_edge_with_transform() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));

        // Act
        let edge = CampaignEdge::new(id1, id2, EdgeCondition::Always)
            .with_transform("Extract summary".to_string());
        campaign.add_edge(edge).expect("Failed to add edge");

        // Assert
        assert_eq!(campaign.edge_count(), 1);
    }

    #[test]
    fn test_campaign_add_edge_nonexistent_source_fails() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));
        let fake_id = uuid::Uuid::new_v4();

        // Act
        let edge = CampaignEdge::new(fake_id, id2, EdgeCondition::Always);
        let result = campaign.add_edge(edge);

        // Assert
        assert!(result.is_err());
        if let Err(BattalionError::InvalidGraph(msg)) = result {
            assert!(msg.contains("Source"));
        } else {
            panic!("Expected InvalidGraph error for nonexistent source");
        }
    }

    #[test]
    fn test_campaign_add_edge_nonexistent_target_fails() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let fake_id = uuid::Uuid::new_v4();

        // Act
        let edge = CampaignEdge::new(id1, fake_id, EdgeCondition::Always);
        let result = campaign.add_edge(edge);

        // Assert
        assert!(result.is_err());
        if let Err(BattalionError::InvalidGraph(msg)) = result {
            assert!(msg.contains("Target"));
        } else {
            panic!("Expected InvalidGraph error for nonexistent target");
        }
    }
}

#[cfg(test)]
mod campaign_validation_tests {
    use super::*;

    #[test]
    fn test_campaign_validate_requires_paladins() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let campaign = Campaign::new(config);

        // Act
        let result = campaign.validate();

        // Assert
        assert!(result.is_err());
        if let Err(BattalionError::InvalidGraph(msg)) = result {
            assert!(msg.contains("at least one Paladin"));
        } else {
            panic!("Expected InvalidGraph error for empty campaign");
        }
    }

    #[test]
    fn test_campaign_validate_requires_entry_point() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        campaign.add_paladin(create_test_paladin("Paladin1"));

        // Act
        let result = campaign.validate();

        // Assert
        assert!(
            result.is_ok(),
            "Campaign with single Paladin should be valid (implicit entry point)"
        );
    }

    #[test]
    fn test_campaign_validate_detects_cycles() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));
        let id3 = campaign.add_paladin(create_test_paladin("Paladin3"));

        // Create a cycle: 1 -> 2 -> 3 -> 1
        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id2, id3, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id3, id1, EdgeCondition::Always))
            .unwrap();

        // Act
        let result = campaign.validate();

        // Assert
        assert!(result.is_err());
        if let Err(BattalionError::InvalidGraph(msg)) = result {
            assert!(msg.contains("cycle") || msg.contains("Cycle"));
        } else {
            panic!("Expected InvalidGraph error for cycle detection");
        }
    }

    #[test]
    fn test_campaign_validate_linear_graph_success() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Paladin1"));
        let id2 = campaign.add_paladin(create_test_paladin("Paladin2"));
        let id3 = campaign.add_paladin(create_test_paladin("Paladin3"));

        // Create linear graph: 1 -> 2 -> 3
        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id2, id3, EdgeCondition::Always))
            .unwrap();

        // Act
        let result = campaign.validate();

        // Assert
        assert!(result.is_ok(), "Linear graph should be valid");
    }

    #[test]
    fn test_campaign_validate_branching_graph_success() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Router"));
        let id2 = campaign.add_paladin(create_test_paladin("Path A"));
        let id3 = campaign.add_paladin(create_test_paladin("Path B"));

        // Create branching: 1 -> 2 and 1 -> 3
        campaign
            .add_edge(CampaignEdge::new(
                id1,
                id2,
                EdgeCondition::Contains("option_a".to_string()),
            ))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(
                id1,
                id3,
                EdgeCondition::Contains("option_b".to_string()),
            ))
            .unwrap();

        // Act
        let result = campaign.validate();

        // Assert
        assert!(result.is_ok(), "Branching graph should be valid");
    }

    #[test]
    fn test_campaign_validate_complex_dag_success() {
        // Arrange - Diamond shape: 1 -> (2, 3) -> 4
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Start"));
        let id2 = campaign.add_paladin(create_test_paladin("Path A"));
        let id3 = campaign.add_paladin(create_test_paladin("Path B"));
        let id4 = campaign.add_paladin(create_test_paladin("Merge"));

        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id1, id3, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id2, id4, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id3, id4, EdgeCondition::Always))
            .unwrap();

        // Act
        let result = campaign.validate();

        // Assert
        assert!(result.is_ok(), "Diamond DAG should be valid");
    }
}

#[cfg(test)]
mod campaign_entry_point_tests {
    use super::*;

    #[test]
    fn test_campaign_set_entry_point() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Entry"));
        let id2 = campaign.add_paladin(create_test_paladin("Worker"));

        campaign
            .add_edge(CampaignEdge::new(id1, id2, EdgeCondition::Always))
            .unwrap();

        // Act
        campaign
            .set_entry_point(id1)
            .expect("Failed to set entry point");

        // Assert
        assert_eq!(campaign.entry_points().len(), 1);
        assert!(campaign.entry_points().contains(&id1));
    }

    #[test]
    fn test_campaign_multiple_entry_points() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let id1 = campaign.add_paladin(create_test_paladin("Entry1"));
        let id2 = campaign.add_paladin(create_test_paladin("Entry2"));
        let id3 = campaign.add_paladin(create_test_paladin("Merge"));

        campaign
            .add_edge(CampaignEdge::new(id1, id3, EdgeCondition::Always))
            .unwrap();
        campaign
            .add_edge(CampaignEdge::new(id2, id3, EdgeCondition::Always))
            .unwrap();

        // Act
        campaign.set_entry_point(id1).unwrap();
        campaign.set_entry_point(id2).unwrap();

        // Assert
        assert_eq!(campaign.entry_points().len(), 2);
        assert!(campaign.entry_points().contains(&id1));
        assert!(campaign.entry_points().contains(&id2));
    }

    #[test]
    fn test_campaign_set_entry_point_nonexistent_fails() {
        // Arrange
        let config = BattalionConfig::new("test_campaign");
        let mut campaign = Campaign::new(config);
        let fake_id = uuid::Uuid::new_v4();

        // Act
        let result = campaign.set_entry_point(fake_id);

        // Assert
        assert!(result.is_err());
        if let Err(BattalionError::InvalidGraph(msg)) = result {
            assert!(msg.contains("entry point") || msg.contains("not found"));
        } else {
            panic!("Expected InvalidGraph error for nonexistent entry point");
        }
    }
}
