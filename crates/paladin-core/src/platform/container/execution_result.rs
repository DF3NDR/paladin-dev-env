//! Paladin Execution Result Types
//!
//! This module defines the result types returned by Paladin execution:
//! [`PaladinResult`] and [`StopReason`].
//!
//! These are pure domain value types with no infrastructure dependencies.
//! The `application` layer re-exports them from here.

use serde::{Deserialize, Serialize};

use crate::platform::container::handoff::HandoffRecord;
use crate::platform::container::planning::TaskPlan;

/// Result of a Paladin execution
///
/// Contains the output, metadata about execution, and the reason for completion.
/// Optionally includes autonomous execution metadata like task plans and handoff history.
///
/// # Example
///
/// ```
/// use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};
///
/// let result = PaladinResult {
///     output: "The answer is 42".to_string(),
///     token_count: 150,
///     execution_time_ms: 1250,
///     loop_count: 1,
///     stop_reason: StopReason::Completed,
///     plan: None,
///     handoff_history: Vec::new(),
/// };
///
/// assert_eq!(result.loop_count, 1);
/// assert!(result.stop_reason.is_successful());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaladinResult {
    /// The generated output text
    pub output: String,

    /// Total number of tokens used (prompt + completion)
    pub token_count: u32,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Number of reasoning loops executed
    pub loop_count: u32,

    /// Reason why execution stopped
    pub stop_reason: StopReason,

    /// Task plan generated during autonomous planning mode
    ///
    /// When a Paladin runs in autonomous planning mode (MaxLoops::Auto),
    /// this field contains the decomposed task plan with subtasks and their results.
    /// This provides transparency into how the Paladin broke down the task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<TaskPlan>,

    /// History of agent handoffs during execution
    ///
    /// When a Paladin delegates tasks to specialist agents, each handoff
    /// is recorded here for transparency and debugging. The records include
    /// which agent handled what task and at what depth in the delegation chain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handoff_history: Vec<HandoffRecord>,
}

/// Reason why Paladin execution stopped
///
/// Indicates whether the Paladin completed successfully or was terminated
/// by a limit or external factor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// Maximum loop iterations reached
    MaxLoops,

    /// A configured stop word was detected
    StopWord(String),

    /// Execution completed naturally
    Completed,

    /// Execution exceeded timeout
    Timeout,
}

impl StopReason {
    /// Check if this represents successful completion
    pub fn is_successful(&self) -> bool {
        matches!(self, StopReason::Completed | StopReason::StopWord(_))
    }

    /// Check if this represents a limit being reached
    pub fn is_limit(&self) -> bool {
        matches!(self, StopReason::MaxLoops | StopReason::Timeout)
    }
}

impl Default for PaladinResult {
    /// Creates a PaladinResult with default values
    ///
    /// Useful for testing and as a base for builder patterns.
    fn default() -> Self {
        Self {
            output: String::new(),
            token_count: 0,
            execution_time_ms: 0,
            loop_count: 0,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
        }
    }
}

impl PaladinResult {
    /// Creates a new PaladinResult with required fields
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};
    ///
    /// let result = PaladinResult::new(
    ///     "Response text".to_string(),
    ///     100,
    ///     500,
    ///     1,
    ///     StopReason::Completed
    /// );
    /// ```
    pub fn new(
        output: String,
        token_count: u32,
        execution_time_ms: u64,
        loop_count: u32,
        stop_reason: StopReason,
    ) -> Self {
        Self {
            output,
            token_count,
            execution_time_ms,
            loop_count,
            stop_reason,
            plan: None,
            handoff_history: Vec::new(),
        }
    }

    /// Checks if this result includes autonomous planning metadata
    pub fn has_plan(&self) -> bool {
        self.plan.is_some()
    }

    /// Checks if this result includes handoff history
    pub fn has_handoffs(&self) -> bool {
        !self.handoff_history.is_empty()
    }

    /// Returns the number of handoffs in the history
    pub fn handoff_count(&self) -> usize {
        self.handoff_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D-26: `served_by: None` leaves the serialised JSON byte-identical to
    /// what the same value produced before the field existed.
    #[test]
    fn served_by_is_absent_from_legacy_json() {
        let result = PaladinResult {
            output: "answer".to_string(),
            token_count: 3,
            execution_time_ms: 7,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
            served_by: None,
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(!json.contains("served_by"), "{json}");
        assert_eq!(
            json,
            r#"{"output":"answer","token_count":3,"execution_time_ms":7,"loop_count":1,"stop_reason":"Completed"}"#
        );
    }

    /// D-26: a payload written before the field existed deserialises with
    /// `served_by: None`.
    #[test]
    fn legacy_json_deserialises_with_served_by_none() {
        let legacy = r#"{"output":"answer","token_count":3,"execution_time_ms":7,"loop_count":1,"stop_reason":"Completed"}"#;

        let result: PaladinResult = serde_json::from_str(legacy).unwrap();

        assert_eq!(result.output, "answer");
        assert!(result.served_by.is_none());
    }

    /// A fallback-served result round-trips its serving provider.
    #[test]
    fn served_by_round_trips_when_present() {
        let result = PaladinResult {
            served_by: Some("anthropic".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""served_by":"anthropic""#), "{json}");
        let back: PaladinResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.served_by.as_deref(), Some("anthropic"));
    }

    /// FT-FR-17: `Default` and `new()` both still work and leave `served_by`
    /// unset.
    #[test]
    fn default_still_constructs() {
        let by_default = PaladinResult::default();
        let by_new = PaladinResult::new("text".to_string(), 10, 20, 1, StopReason::Completed);

        assert!(by_default.served_by.is_none());
        assert!(by_new.served_by.is_none());
        assert_eq!(by_new.output, "text");
        assert_eq!(by_new.token_count, 10);
    }
}
