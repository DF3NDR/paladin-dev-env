//! Planning Domain Types
//!
//! This module defines domain types for autonomous task planning and decomposition.
//! These types support the autonomous planning mode where a Paladin uses an LLM
//! to break down complex tasks into manageable subtasks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete execution plan for a complex task
///
/// Contains the original task description and a list of subtasks
/// that the planner has determined are needed to complete the task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskPlan {
    /// Original task description provided by the user
    pub original_task: String,

    /// List of subtasks to execute in order
    pub subtasks: Vec<Subtask>,

    /// Dependencies between subtasks (subtask_id -> list of dependency ids)
    pub dependencies: HashMap<String, Vec<String>>,

    /// Maximum number of subtasks allowed in this plan
    pub max_subtasks: u32,
}

impl TaskPlan {
    /// Creates a new empty task plan
    ///
    /// # Arguments
    ///
    /// * `original_task` - The original user task description
    /// * `max_subtasks` - Maximum number of subtasks allowed
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_core::platform::container::planning::TaskPlan;
    ///
    /// let plan = TaskPlan::new("Create a marketing strategy".to_string(), 10);
    /// assert_eq!(plan.subtasks.len(), 0);
    /// ```
    pub fn new(original_task: String, max_subtasks: u32) -> Self {
        Self {
            original_task,
            subtasks: Vec::new(),
            dependencies: HashMap::new(),
            max_subtasks,
        }
    }

    /// Adds a subtask to the plan
    ///
    /// Returns an error if adding would exceed max_subtasks
    pub fn add_subtask(&mut self, subtask: Subtask) -> Result<(), String> {
        if self.subtasks.len() >= self.max_subtasks as usize {
            return Err(format!(
                "Cannot add subtask: max_subtasks limit of {} reached",
                self.max_subtasks
            ));
        }
        self.subtasks.push(subtask);
        Ok(())
    }

    /// Returns the number of subtasks in the plan
    pub fn subtask_count(&self) -> usize {
        self.subtasks.len()
    }

    /// Validates the plan structure
    ///
    /// Checks for:
    /// - Non-empty subtasks
    /// - Valid dependencies (no circular refs, all referenced tasks exist)
    /// - Subtask count within limits
    pub fn validate(&self) -> Result<(), String> {
        if self.subtasks.is_empty() {
            return Err("Task plan must have at least one subtask".to_string());
        }

        if self.subtasks.len() > self.max_subtasks as usize {
            return Err(format!(
                "Subtask count {} exceeds maximum {}",
                self.subtasks.len(),
                self.max_subtasks
            ));
        }

        // Validate all dependency references exist
        for (subtask_id, deps) in &self.dependencies {
            if !self.subtasks.iter().any(|st| st.id == *subtask_id) {
                return Err(format!(
                    "Dependency references non-existent subtask: {}",
                    subtask_id
                ));
            }
            for dep_id in deps {
                if !self.subtasks.iter().any(|st| st.id == *dep_id) {
                    return Err(format!(
                        "Dependency {} references non-existent subtask: {}",
                        subtask_id, dep_id
                    ));
                }
            }
        }

        Ok(())
    }
}

/// A single subtask within a larger task plan
///
/// Represents one atomic unit of work that can be executed independently
/// or with dependencies on other subtasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subtask {
    /// Unique identifier for this subtask
    pub id: String,

    /// Description of what this subtask accomplishes
    pub description: String,

    /// Expected output or result from executing this subtask; empty when none was specified
    /// (`PlanningService` leaves it empty, since its plan schema carries none)
    pub expected_output: String,

    /// Actual result after execution (None if not yet executed)
    pub result: Option<String>,

    /// Whether this subtask has been completed
    pub completed: bool,
}

impl Subtask {
    /// Creates a new subtask
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `description` - What the subtask does
    /// * `expected_output` - What result is expected
    ///
    /// # Example
    ///
    /// ```
    /// use paladin_core::platform::container::planning::Subtask;
    ///
    /// let subtask = Subtask::new(
    ///     "st-1".to_string(),
    ///     "Analyze market trends".to_string(),
    ///     "Market analysis report".to_string()
    /// );
    /// assert!(!subtask.completed);
    /// ```
    pub fn new(id: String, description: String, expected_output: String) -> Self {
        Self {
            id,
            description,
            expected_output,
            result: None,
            completed: false,
        }
    }

    /// Marks the subtask as completed with a result
    pub fn complete(&mut self, result: String) {
        self.result = Some(result);
        self.completed = true;
    }

    /// Returns true if this subtask is completed
    pub fn is_completed(&self) -> bool {
        self.completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_plan_new() {
        let plan = TaskPlan::new("Test task".to_string(), 5);
        assert_eq!(plan.original_task, "Test task");
        assert_eq!(plan.subtasks.len(), 0);
        assert_eq!(plan.max_subtasks, 5);
    }

    #[test]
    fn test_task_plan_add_subtask() {
        let mut plan = TaskPlan::new("Test task".to_string(), 3);
        let subtask = Subtask::new(
            "st-1".to_string(),
            "First subtask".to_string(),
            "Result 1".to_string(),
        );

        assert!(plan.add_subtask(subtask.clone()).is_ok());
        assert_eq!(plan.subtask_count(), 1);
    }

    #[test]
    fn test_task_plan_max_subtasks_limit() {
        let mut plan = TaskPlan::new("Test task".to_string(), 2);
        let st1 = Subtask::new("st-1".to_string(), "First".to_string(), "R1".to_string());
        let st2 = Subtask::new("st-2".to_string(), "Second".to_string(), "R2".to_string());
        let st3 = Subtask::new("st-3".to_string(), "Third".to_string(), "R3".to_string());

        assert!(plan.add_subtask(st1).is_ok());
        assert!(plan.add_subtask(st2).is_ok());

        let result = plan.add_subtask(st3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_subtasks limit"));
    }

    #[test]
    fn test_task_plan_validate_empty() {
        let plan = TaskPlan::new("Test".to_string(), 5);
        let result = plan.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one subtask"));
    }

    #[test]
    fn test_task_plan_validate_valid() {
        let mut plan = TaskPlan::new("Test".to_string(), 5);
        let subtask = Subtask::new("st-1".to_string(), "Task".to_string(), "Output".to_string());
        plan.add_subtask(subtask).unwrap();

        assert!(plan.validate().is_ok());
    }

    #[test]
    fn test_task_plan_validate_dependencies() {
        let mut plan = TaskPlan::new("Test".to_string(), 5);
        let st1 = Subtask::new("st-1".to_string(), "First".to_string(), "R1".to_string());
        let st2 = Subtask::new("st-2".to_string(), "Second".to_string(), "R2".to_string());
        plan.add_subtask(st1).unwrap();
        plan.add_subtask(st2).unwrap();

        // Add valid dependency
        plan.dependencies
            .insert("st-2".to_string(), vec!["st-1".to_string()]);
        assert!(plan.validate().is_ok());

        // Add invalid dependency
        plan.dependencies
            .insert("st-2".to_string(), vec!["st-999".to_string()]);
        let result = plan.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_subtask_new() {
        let subtask = Subtask::new(
            "st-1".to_string(),
            "Analyze data".to_string(),
            "Analysis report".to_string(),
        );

        assert_eq!(subtask.id, "st-1");
        assert_eq!(subtask.description, "Analyze data");
        assert_eq!(subtask.expected_output, "Analysis report");
        assert!(!subtask.completed);
        assert!(subtask.result.is_none());
    }

    #[test]
    fn test_subtask_complete() {
        let mut subtask =
            Subtask::new("st-1".to_string(), "Task".to_string(), "Output".to_string());

        assert!(!subtask.is_completed());

        subtask.complete("Actual result".to_string());

        assert!(subtask.is_completed());
        assert_eq!(subtask.result, Some("Actual result".to_string()));
    }

    #[test]
    fn test_subtask_serialization() {
        let subtask = Subtask::new(
            "st-1".to_string(),
            "Test task".to_string(),
            "Test output".to_string(),
        );

        let json = serde_json::to_string(&subtask).unwrap();
        let deserialized: Subtask = serde_json::from_str(&json).unwrap();

        assert_eq!(subtask, deserialized);
    }

    #[test]
    fn test_task_plan_serialization() {
        let mut plan = TaskPlan::new("Main task".to_string(), 5);
        let subtask = Subtask::new("st-1".to_string(), "Sub".to_string(), "Out".to_string());
        plan.add_subtask(subtask).unwrap();

        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: TaskPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(plan, deserialized);
    }
}
