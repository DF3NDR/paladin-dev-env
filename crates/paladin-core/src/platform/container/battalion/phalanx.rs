//! Phalanx Pattern - Concurrent Execution
//!
//! Phalanx executes multiple Paladins concurrently (in parallel) and aggregates
//! their results according to a specified strategy.

use serde::{Deserialize, Serialize};

use crate::platform::container::battalion::{BattalionConfig, BattalionError};
use crate::platform::container::paladin::Paladin;

/// Aggregation strategy for combining concurrent Paladin results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AggregationStrategy {
    /// Collect all results (wait for all Paladins to complete)
    #[default]
    CollectAll,

    /// Return first successful result (early termination)
    FirstSuccess,

    /// Require majority consensus (≥50% agreement, requires ≥3 Paladins)
    Majority,

    /// Custom aggregation logic (function name for future extensibility)
    Custom(String),
}

/// Phalanx - Concurrent Paladin execution pattern
///
/// Executes multiple Paladins in parallel and aggregates their results
/// according to the specified aggregation strategy.
///
/// # Example
///
/// ```ignore
/// let phalanx = Phalanx::new(
///     vec![paladin1, paladin2, paladin3],
///     BattalionConfig::new("concurrent_analysis")
/// )?
/// .with_aggregation(AggregationStrategy::Majority)
/// .with_max_concurrency(10);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phalanx {
    /// Paladins to execute concurrently
    paladins: Vec<Paladin>,

    /// Battalion configuration
    config: BattalionConfig,

    /// Strategy for aggregating results
    aggregation_strategy: AggregationStrategy,

    /// Maximum concurrent Paladins (None = unlimited)
    max_concurrency: Option<usize>,
}

impl Phalanx {
    /// Create a new Phalanx with the specified Paladins and configuration
    ///
    /// # Validation
    /// - Requires at least 2 Paladins
    ///
    /// # Example
    ///
    /// ```ignore
    /// let phalanx = Phalanx::new(
    ///     vec![paladin1, paladin2],
    ///     BattalionConfig::new("parallel_processing")
    /// )?;
    /// ```
    pub fn new(paladins: Vec<Paladin>, config: BattalionConfig) -> Result<Self, BattalionError> {
        let phalanx = Self {
            paladins,
            config,
            aggregation_strategy: AggregationStrategy::default(),
            max_concurrency: None,
        };

        phalanx.validate()?;
        Ok(phalanx)
    }

    /// Set the aggregation strategy
    ///
    /// # Example
    ///
    /// ```ignore
    /// let phalanx = phalanx.with_aggregation(AggregationStrategy::FirstSuccess);
    /// ```
    pub fn with_aggregation(mut self, strategy: AggregationStrategy) -> Self {
        self.aggregation_strategy = strategy;
        self
    }

    /// Set maximum concurrent Paladins (for concurrency limiting)
    ///
    /// `max` must be at least 1: the Phalanx execution service rejects a limit
    /// of 0 with a validation error before any Paladin runs, because a
    /// zero-permit limiter would block every Paladin forever. It also rejects a
    /// limit above tokio's `Semaphore::MAX_PERMITS`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let phalanx = phalanx.with_max_concurrency(5);
    /// ```
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = Some(max);
        self
    }

    /// Get the number of Paladins in this Phalanx
    pub fn paladin_count(&self) -> usize {
        self.paladins.len()
    }

    /// Get reference to the Paladins
    pub fn paladins(&self) -> &[Paladin] {
        &self.paladins
    }

    /// Get reference to the configuration
    pub fn config(&self) -> &BattalionConfig {
        &self.config
    }

    /// Get reference to the aggregation strategy
    pub fn aggregation_strategy(&self) -> &AggregationStrategy {
        &self.aggregation_strategy
    }

    /// Get the maximum concurrency limit
    pub fn max_concurrency(&self) -> Option<usize> {
        self.max_concurrency
    }

    /// Validate Phalanx configuration
    fn validate(&self) -> Result<(), BattalionError> {
        if self.paladins.len() < 2 {
            return Err(BattalionError::ValidationError(
                "Phalanx requires at least 2 Paladins".to_string(),
            ));
        }

        // Validate Majority strategy requirements
        if matches!(self.aggregation_strategy, AggregationStrategy::Majority)
            && self.paladins.len() < 3
        {
            return Err(BattalionError::ValidationError(
                "Majority aggregation requires at least 3 Paladins".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::entity::Entity;
    use crate::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};

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
    fn test_phalanx_requires_minimum_two_paladins() {
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
    fn test_phalanx_with_aggregation_strategy() {
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
    fn test_majority_strategy_validation() {
        let p1 = create_test_paladin("Agent1");
        let p2 = create_test_paladin("Agent2");
        let config = BattalionConfig::new("test_phalanx");

        // Creating with 2 Paladins should succeed
        let result = Phalanx::new(vec![p1, p2], config);
        assert!(result.is_ok());

        // But setting Majority should fail validation when executed
        // (we'll test this in the service layer)
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
        assert!(matches!(
            phalanx.aggregation_strategy(),
            &AggregationStrategy::CollectAll
        ));
        assert_eq!(phalanx.max_concurrency(), None);
    }
}
