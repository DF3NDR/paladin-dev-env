//! Formation Domain Entity
//!
//! Formation represents a sequential execution pattern for Battalion orchestration
//! where Paladins execute one after another, with output from one feeding into the next.

use serde::{Deserialize, Serialize};

use super::{BattalionConfig, BattalionError};
use crate::platform::container::paladin::Paladin;

/// Formation orchestration pattern
///
/// A Formation executes Paladins sequentially, where the output of each Paladin
/// becomes the input for the next Paladin in the sequence. This is useful for
/// building processing pipelines where each step refines or transforms the output
/// of the previous step.
///
/// # Minimum Requirements
///
/// - At least 1 Paladin is required for a Formation
/// - Paladins execute in the order they are provided
///
/// # Example
///
/// ```ignore
/// use paladin_core::platform::container::battalion::formation::Formation;
/// use paladin_core::platform::container::battalion::BattalionConfig;
///
/// let config = BattalionConfig::new("research_pipeline");
/// let paladins = vec![researcher, analyst, summarizer];
///
/// let formation = Formation::new(paladins, config)
///     .unwrap()
///     .with_shared_context("Research topic: AI Safety".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formation {
    /// Paladins to execute in sequence
    pub paladins: Vec<Paladin>,

    /// Configuration for this Formation
    pub config: BattalionConfig,

    /// Optional shared context provided to all Paladins
    pub shared_context: Option<String>,
}

impl Formation {
    /// Create a new Formation with the given Paladins and configuration
    ///
    /// # Arguments
    ///
    /// * `paladins` - Vector of Paladins to execute sequentially (minimum 1)
    /// * `config` - Battalion configuration
    ///
    /// # Returns
    ///
    /// * `Ok(Formation)` - Successfully created Formation
    /// * `Err(BattalionError::ValidationError)` - If validation fails (< 1 Paladin)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let paladins = vec![paladin1, paladin2, paladin3];
    /// let config = BattalionConfig::new("my_formation");
    /// let formation = Formation::new(paladins, config)?;
    /// ```
    pub fn new(paladins: Vec<Paladin>, config: BattalionConfig) -> Result<Self, BattalionError> {
        let formation = Self {
            paladins,
            config,
            shared_context: None,
        };

        formation.validate()?;
        Ok(formation)
    }

    /// Set shared context for all Paladins
    ///
    /// The shared context is prepended to each Paladin's input, allowing you to
    /// provide global information or instructions that apply to all steps in the Formation.
    ///
    /// # Arguments
    ///
    /// * `context` - Context string to share with all Paladins
    ///
    /// # Example
    ///
    /// ```ignore
    /// let formation = Formation::new(paladins, config)?
    ///     .with_shared_context("Goal: Generate a comprehensive report".to_string());
    /// ```
    pub fn with_shared_context(mut self, context: String) -> Self {
        self.shared_context = Some(context);
        self
    }

    /// Validate Formation requirements
    ///
    /// Ensures:
    /// - at least 1 Paladin is present
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Validation passed
    /// * `Err(BattalionError::ValidationError)` - Validation failed
    fn validate(&self) -> Result<(), BattalionError> {
        if self.paladins.is_empty() {
            return Err(BattalionError::ValidationError(format!(
                "Formation requires at least 1 Paladin, got {}",
                self.paladins.len()
            )));
        }

        Ok(())
    }

    /// Get the number of Paladins in this Formation
    pub fn paladin_count(&self) -> usize {
        self.paladins.len()
    }

    /// Get a reference to the Paladins
    pub fn paladins(&self) -> &[Paladin] {
        &self.paladins
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &BattalionConfig {
        &self.config
    }

    /// Get a reference to the shared context
    pub fn shared_context(&self) -> Option<&str> {
        self.shared_context.as_deref()
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
    fn test_formation_creation_valid() {
        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");
        let config = BattalionConfig::new("test");

        let result = Formation::new(vec![p1, p2], config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_formation_validation_minimum_paladins() {
        let p1 = create_test_paladin("P1");
        let config = BattalionConfig::new("test");

        let result = Formation::new(vec![p1], config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_formation_rejects_zero_paladins() {
        let config = BattalionConfig::new("test");

        let result = Formation::new(vec![], config);
        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::ValidationError(msg) => {
                assert!(msg.contains('0'));
            }
            other => panic!("Expected ValidationError, got {other:?}"),
        }
    }

    #[test]
    fn test_formation_accepts_single_paladin() {
        let p1 = create_test_paladin("P1");
        let config = BattalionConfig::new("test");

        let result = Formation::new(vec![p1], config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_formation_paladin_count() {
        let paladins: Vec<_> = (0..5)
            .map(|i| create_test_paladin(&format!("P{}", i)))
            .collect();
        let config = BattalionConfig::new("test");

        let formation = Formation::new(paladins, config).unwrap();
        assert_eq!(formation.paladin_count(), 5);
    }

    #[test]
    fn test_formation_with_shared_context() {
        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");
        let config = BattalionConfig::new("test");

        let formation = Formation::new(vec![p1, p2], config)
            .unwrap()
            .with_shared_context("Test context".to_string());

        assert_eq!(formation.shared_context(), Some("Test context"));
    }

    #[test]
    fn test_formation_accessors() {
        let p1 = create_test_paladin("P1");
        let p2 = create_test_paladin("P2");
        let config = BattalionConfig::new("test");

        let formation = Formation::new(vec![p1, p2], config).unwrap();

        assert_eq!(formation.paladins().len(), 2);
        assert_eq!(formation.config().name, "test");
        assert_eq!(formation.shared_context(), None);
    }
}
