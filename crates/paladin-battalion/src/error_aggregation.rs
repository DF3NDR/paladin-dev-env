//! Error Aggregation Utility
//!
//! Provides utilities for collecting and aggregating errors during Battalion execution,
//! particularly for patterns that continue past a failed Paladin under
//! `aegis.on_error = Absorb`.

use paladin_core::platform::container::battalion::BattalionError;

/// Aggregated error information
///
/// Collects multiple errors that occurred during Battalion execution
/// when a pattern continues past a failed Paladin (`aegis.on_error = Absorb`).
#[derive(Debug, Clone)]
pub struct AggregatedError {
    /// Individual errors collected
    pub errors: Vec<BattalionError>,

    /// Total number of operations attempted
    pub total_operations: usize,

    /// Number of successful operations
    pub successful_operations: usize,
}

impl AggregatedError {
    /// Create a new AggregatedError
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    ///
    /// let agg = AggregatedError::new(10);
    /// assert_eq!(agg.total_operations, 10);
    /// assert_eq!(agg.successful_operations, 0);
    /// assert!(agg.errors.is_empty());
    /// ```
    pub fn new(total_operations: usize) -> Self {
        Self {
            errors: Vec::new(),
            total_operations,
            successful_operations: 0,
        }
    }

    /// Add an error to the collection
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    /// use paladin_core::platform::container::battalion::BattalionError;
    ///
    /// let mut agg = AggregatedError::new(5);
    /// agg.add_error(BattalionError::PaladinError("Test error".to_string()));
    /// assert_eq!(agg.errors.len(), 1);
    /// ```
    pub fn add_error(&mut self, error: BattalionError) {
        self.errors.push(error);
    }

    /// Record a successful operation
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    ///
    /// let mut agg = AggregatedError::new(5);
    /// agg.record_success();
    /// agg.record_success();
    /// assert_eq!(agg.successful_operations, 2);
    /// ```
    pub fn record_success(&mut self) {
        self.successful_operations += 1;
    }

    /// Check if any errors were collected
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    /// use paladin_core::platform::container::battalion::BattalionError;
    ///
    /// let mut agg = AggregatedError::new(5);
    /// assert!(!agg.has_errors());
    ///
    /// agg.add_error(BattalionError::PaladinError("Test".to_string()));
    /// assert!(agg.has_errors());
    /// ```
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the failure rate as a percentage
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    /// use paladin_core::platform::container::battalion::BattalionError;
    ///
    /// let mut agg = AggregatedError::new(10);
    /// agg.record_success();
    /// agg.record_success();
    /// agg.record_success();
    /// agg.add_error(BattalionError::PaladinError("Test".to_string()));
    ///
    /// // 3 success, 1 error = 4 total, 1 failed
    /// assert_eq!(agg.failure_rate(), 25.0); // 1/4 = 25%
    /// ```
    pub fn failure_rate(&self) -> f64 {
        let total_executed = self.successful_operations + self.errors.len();
        if total_executed == 0 {
            0.0
        } else {
            (self.errors.len() as f64 / total_executed as f64) * 100.0
        }
    }

    /// Get a summary message of all errors
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    /// use paladin_core::platform::container::battalion::BattalionError;
    ///
    /// let mut agg = AggregatedError::new(5);
    /// agg.add_error(BattalionError::PaladinError("Error 1".to_string()));
    /// agg.add_error(BattalionError::PaladinError("Error 2".to_string()));
    /// agg.record_success();
    ///
    /// let summary = agg.summary();
    /// assert!(summary.contains("2 errors"));
    /// assert!(summary.contains("1 successful"));
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Battalion execution completed with {} errors out of {} operations ({} successful, {:.1}% failure rate)",
            self.errors.len(),
            self.total_operations,
            self.successful_operations,
            self.failure_rate()
        )
    }

    /// Convert to a BattalionError if there are errors
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_battalion::error_aggregation::AggregatedError;
    /// use paladin_core::platform::container::battalion::BattalionError;
    ///
    /// let mut agg = AggregatedError::new(5);
    /// agg.add_error(BattalionError::PaladinError("Test".to_string()));
    ///
    /// let result = agg.into_result();
    /// assert!(result.is_err());
    /// ```
    pub fn into_result(self) -> Result<(), BattalionError> {
        if self.has_errors() {
            Err(BattalionError::AggregationError(self.summary()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battalion::BattalionError;

    #[test]
    fn test_aggregated_error_new() {
        let agg = AggregatedError::new(10);
        assert_eq!(agg.total_operations, 10);
        assert_eq!(agg.successful_operations, 0);
        assert_eq!(agg.errors.len(), 0);
        assert!(!agg.has_errors());
    }

    #[test]
    fn test_add_error() {
        let mut agg = AggregatedError::new(5);

        agg.add_error(BattalionError::PaladinError("Error 1".to_string()));
        assert_eq!(agg.errors.len(), 1);
        assert!(agg.has_errors());

        agg.add_error(BattalionError::FormationError("Error 2".to_string()));
        assert_eq!(agg.errors.len(), 2);
    }

    #[test]
    fn test_record_success() {
        let mut agg = AggregatedError::new(5);

        agg.record_success();
        assert_eq!(agg.successful_operations, 1);

        agg.record_success();
        agg.record_success();
        assert_eq!(agg.successful_operations, 3);
    }

    #[test]
    fn test_failure_rate_no_operations() {
        let agg = AggregatedError::new(10);
        assert_eq!(agg.failure_rate(), 0.0);
    }

    #[test]
    fn test_failure_rate_all_success() {
        let mut agg = AggregatedError::new(10);
        for _ in 0..10 {
            agg.record_success();
        }
        assert_eq!(agg.failure_rate(), 0.0);
    }

    #[test]
    fn test_failure_rate_all_failures() {
        let mut agg = AggregatedError::new(5);
        for i in 0..5 {
            agg.add_error(BattalionError::PaladinError(format!("Error {}", i)));
        }
        assert_eq!(agg.failure_rate(), 100.0);
    }

    #[test]
    fn test_failure_rate_mixed() {
        let mut agg = AggregatedError::new(10);

        // 3 successful
        agg.record_success();
        agg.record_success();
        agg.record_success();

        // 1 failure
        agg.add_error(BattalionError::PaladinError("Error".to_string()));

        // Total: 4 executed, 1 failed = 25%
        assert_eq!(agg.failure_rate(), 25.0);
    }

    #[test]
    fn test_summary() {
        let mut agg = AggregatedError::new(10);

        agg.record_success();
        agg.record_success();
        agg.add_error(BattalionError::PaladinError("Error 1".to_string()));
        agg.add_error(BattalionError::FormationError("Error 2".to_string()));

        let summary = agg.summary();
        assert!(summary.contains("2 errors"));
        assert!(summary.contains("10 operations"));
        assert!(summary.contains("2 successful"));
        assert!(summary.contains("50.0%")); // 2 failed out of 4 executed
    }

    #[test]
    fn test_into_result_with_errors() {
        let mut agg = AggregatedError::new(5);
        agg.add_error(BattalionError::PaladinError("Test".to_string()));

        let result = agg.into_result();
        assert!(result.is_err());

        match result.unwrap_err() {
            BattalionError::AggregationError(msg) => {
                assert!(msg.contains("1 errors"));
            }
            _ => panic!("Expected AggregationError"),
        }
    }

    #[test]
    fn test_into_result_no_errors() {
        let mut agg = AggregatedError::new(5);
        agg.record_success();
        agg.record_success();

        let result = agg.into_result();
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_errors() {
        let mut agg = AggregatedError::new(5);
        assert!(!agg.has_errors());

        agg.record_success();
        assert!(!agg.has_errors());

        agg.add_error(BattalionError::ExecutionError(
            "Paladin never answered".to_string(),
        ));
        assert!(agg.has_errors());
    }

    #[test]
    fn test_multiple_error_types() {
        let mut agg = AggregatedError::new(10);

        agg.add_error(BattalionError::PaladinError("Paladin failed".to_string()));
        agg.add_error(BattalionError::FormationError(
            "Formation issue".to_string(),
        ));
        agg.add_error(BattalionError::ExecutionError(
            "Paladin never answered".to_string(),
        ));
        agg.add_error(BattalionError::ValidationError(
            "Invalid config".to_string(),
        ));

        assert_eq!(agg.errors.len(), 4);
        assert_eq!(agg.failure_rate(), 100.0); // No successes recorded
    }

    #[test]
    fn test_realistic_scenario() {
        let mut agg = AggregatedError::new(10);

        // Simulate 10 operations: 7 success, 3 failures
        for _ in 0..7 {
            agg.record_success();
        }

        agg.add_error(BattalionError::PaladinError("Paladin 3 failed".to_string()));
        agg.add_error(BattalionError::PaladinError("Paladin 5 failed".to_string()));
        agg.add_error(BattalionError::ExecutionError(
            "Paladin never answered".to_string(),
        ));

        assert_eq!(agg.successful_operations, 7);
        assert_eq!(agg.errors.len(), 3);
        assert_eq!(agg.failure_rate(), 30.0); // 3/10
        assert!(agg.has_errors());

        let summary = agg.summary();
        assert!(summary.contains("3 errors"));
        assert!(summary.contains("7 successful"));
    }
}
