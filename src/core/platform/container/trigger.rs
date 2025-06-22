/*
Trigger Container

A Trigger is a container that represents a request to execute an Action in response to an Event.
Triggers are created by Listener services when they detect Events that match their criteria.

The Trigger acts as a bridge between Events and Actions, containing:
- The Event that caused the trigger
- The Action to be executed
- Trigger-specific configuration and metadata
- Condition matching and filtering logic

This container is the base for more complex Trigger types that can be built in the
Application Layer for specific use cases.
*/

use crate::core::base::component::event::Event;
use crate::core::base::component::action::{Action, ActionStatus};
use crate::core::base::entity::message::MessagePriority;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Datelike, Timelike};
use std::collections::HashMap;

/// Trigger execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerStatus {
    /// Trigger has been created and is ready to be processed
    Pending,
    /// Trigger is being processed
    Processing,
    /// Trigger has been successfully processed (action executed)
    Completed,
    /// Trigger processing failed
    Failed,
    /// Trigger was cancelled
    Cancelled,
    /// Trigger was skipped due to conditions not being met
    Skipped,
    /// Trigger expired before processing
    Expired,
}

impl Default for TriggerStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Trigger condition for determining when to fire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    /// Event type pattern to match (supports wildcards)
    pub event_type_pattern: String,
    /// Source pattern to match
    pub source_pattern: Option<String>,
    /// Payload conditions (JSONPath expressions)
    pub payload_conditions: Vec<String>,
    /// Minimum priority level to trigger
    pub min_priority: Option<MessagePriority>,
    /// Time-based conditions
    pub time_conditions: Option<TimeCondition>,
}

/// Time-based trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCondition {
    /// Only trigger during these hours (24-hour format)
    pub active_hours: Option<(u8, u8)>,
    /// Days of week to be active (0 = Sunday)
    pub active_days: Option<Vec<u8>>,
    /// Cooldown period in seconds between triggers
    pub cooldown_seconds: Option<u64>,
}

/// Trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Maximum number of retries if action fails
    pub max_retries: u32,
    /// Timeout for action execution in seconds
    pub timeout_seconds: u64,
    /// Whether to preserve the trigger after completion
    pub preserve_after_completion: bool,
    /// Time-to-live for the trigger in seconds
    pub ttl_seconds: u64,
    /// Priority for trigger processing
    pub processing_priority: MessagePriority,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_seconds: 300, // 5 minutes
            preserve_after_completion: true,
            ttl_seconds: 3600, // 1 hour
            processing_priority: MessagePriority::Normal,
        }
    }
}

/// Main Trigger container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Unique identifier for the trigger
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Description of what this trigger does
    pub description: String,
    /// Source listener service that created this trigger
    pub source: String,
    /// Target service that will process the trigger
    pub target: String,
    /// The event that caused this trigger
    pub triggering_event: Event,
    /// The action to be executed
    pub action: Action,
    /// Trigger condition that was matched
    pub condition: TriggerCondition,
    /// Configuration for trigger processing
    pub config: TriggerConfig,
    /// Current processing status
    pub status: TriggerStatus,
    /// When the trigger was created
    pub created_at: DateTime<Utc>,
    /// When the trigger was last updated
    pub updated_at: DateTime<Utc>,
    /// When the trigger was processed
    pub processed_at: Option<DateTime<Utc>>,
    /// Number of processing attempts
    pub attempt_count: u32,
    /// Processing worker identifier
    pub worker_id: Option<String>,
    /// Additional trigger-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Correlation chain for tracking related triggers
    pub correlation_chain: Vec<Uuid>,
}

impl Trigger {
    /// Create a new trigger
    pub fn new(
        name: String,
        description: String,
        source: String,
        target: String,
        triggering_event: Event,
        action: Action,
        condition: TriggerCondition,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            source,
            target,
            triggering_event,
            action,
            condition,
            config: TriggerConfig::default(),
            status: TriggerStatus::Pending,
            created_at: now,
            updated_at: now,
            processed_at: None,
            attempt_count: 0,
            worker_id: None,
            metadata: HashMap::new(),
            correlation_chain: Vec::new(),
        }
    }

    /// Create a trigger with custom configuration
    pub fn with_config(
        name: String,
        description: String,
        source: String,
        target: String,
        triggering_event: Event,
        action: Action,
        condition: TriggerCondition,
        config: TriggerConfig,
    ) -> Self {
        let mut trigger = Self::new(name, description, source, target, triggering_event, action, condition);
        trigger.config = config;
        trigger
    }

    /// Check if the trigger matches the given event
    pub fn matches_event(&self, event: &Event) -> bool {
        // Check event type pattern
        if !self.matches_pattern(&self.condition.event_type_pattern, &event.event_type) {
            return false;
        }

        // Check source pattern
        if let Some(source_pattern) = &self.condition.source_pattern {
            if !self.matches_pattern(source_pattern, &event.source) {
                return false;
            }
        }

        // Check time conditions
        if let Some(time_cond) = &self.condition.time_conditions {
            if !self.matches_time_condition(time_cond) {
                return false;
            }
        }

        // TODO: Implement payload condition matching (JSONPath)
        // This would require a JSONPath library for complex payload filtering

        true
    }

    /// Check if a string matches a pattern (supports simple wildcards)
    fn matches_pattern(&self, pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return value.starts_with(prefix) && value.ends_with(suffix);
            }
        }
        
        pattern == value
    }

    /// Check if current time matches time conditions
    fn matches_time_condition(&self, time_cond: &TimeCondition) -> bool {
        let now = Utc::now();
        
        // Check active hours
        if let Some((start_hour, end_hour)) = time_cond.active_hours {
            let current_hour = now.hour() as u8;
            if current_hour < start_hour || current_hour > end_hour {
                return false;
            }
        }
        
        // Check active days
        if let Some(active_days) = &time_cond.active_days {
            let current_day = now.weekday().num_days_from_sunday() as u8;
            if !active_days.contains(&current_day) {
                return false;
            }
        }
        
        // TODO: Check cooldown period
        // This would require tracking the last execution time
        
        true
    }

    /// Check if the trigger can be processed
    pub fn can_process(&self) -> bool {
        match self.status {
            TriggerStatus::Pending => !self.is_expired(),
            _ => false,
        }
    }

    /// Check if the trigger has expired
    pub fn is_expired(&self) -> bool {
        let age_seconds = Utc::now().timestamp() - self.created_at.timestamp();
        age_seconds > self.config.ttl_seconds as i64
    }

    /// Check if retry attempts are exhausted
    pub fn is_retry_exhausted(&self) -> bool {
        self.attempt_count > self.config.max_retries
    }

    /// Start processing the trigger
    pub fn start_processing(&mut self, worker_id: String) -> Result<(), String> {
        if !self.can_process() {
            return Err(format!("Trigger cannot be processed, current status: {:?}", self.status));
        }

        if self.is_expired() {
            self.status = TriggerStatus::Expired;
            return Err("Trigger has expired".to_string());
        }

        self.status = TriggerStatus::Processing;
        self.worker_id = Some(worker_id);
        self.attempt_count += 1;
        self.updated_at = Utc::now();
        
        // Start the underlying action
        self.action.start_execution();
        
        Ok(())
    }

    /// Complete processing successfully
    pub fn complete_processing(&mut self, result_data: Option<serde_json::Value>) {
        self.status = TriggerStatus::Completed;
        self.processed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.worker_id = None;
        
        // Complete the underlying action
        let action_result = crate::core::base::component::action::ActionResult {
            success: true,
            duration_ms: self.processing_duration_ms(),
            data: result_data,
            error: None,
            metadata: self.metadata.clone(),
        };
        
        self.action.complete_execution(action_result);
    }

    /// Fail processing with an error
    pub fn fail_processing(&mut self, error: String) -> bool {
        let duration_ms = self.processing_duration_ms();
        let can_retry = self.action.fail_execution(error.clone(), duration_ms) && !self.is_retry_exhausted();
        
        if can_retry {
            self.status = TriggerStatus::Pending; // Reset to pending for retry
        } else {
            self.status = TriggerStatus::Failed;
            self.processed_at = Some(Utc::now());
        }
        
        self.updated_at = Utc::now();
        self.worker_id = None;
        
        can_retry
    }

    /// Cancel the trigger
    pub fn cancel(&mut self) {
        self.status = TriggerStatus::Cancelled;
        self.updated_at = Utc::now();
        self.worker_id = None;
        self.action.cancel();
    }

    /// Skip the trigger (conditions not met after initial match)
    pub fn skip(&mut self, reason: String) {
        self.status = TriggerStatus::Skipped;
        self.updated_at = Utc::now();
        self.add_metadata("skip_reason".to_string(), serde_json::Value::String(reason));
    }

    /// Get processing duration in milliseconds
    fn processing_duration_ms(&self) -> u64 {
        if let Some(processed_at) = self.processed_at {
            let duration = processed_at.timestamp_millis() - self.created_at.timestamp_millis();
            duration.max(0) as u64
        } else {
            0
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Add to correlation chain
    pub fn add_correlation(&mut self, correlation_id: Uuid) {
        if !self.correlation_chain.contains(&correlation_id) {
            self.correlation_chain.push(correlation_id);
            self.updated_at = Utc::now();
        }
    }

    /// Create a summary for monitoring
    pub fn summary(&self) -> TriggerSummary {
        TriggerSummary {
            id: self.id,
            name: self.name.clone(),
            source: self.source.clone(),
            target: self.target.clone(),
            status: self.status.clone(),
            event_type: self.triggering_event.event_type.clone(),
            event_source: self.triggering_event.source.clone(),
            action_name: self.action.name.clone(),
            action_status: self.action.status.clone(),
            attempt_count: self.attempt_count,
            age_seconds: Utc::now().timestamp() - self.created_at.timestamp(),
            processing_duration_ms: if self.status == TriggerStatus::Processing {
                Some(self.processing_duration_ms())
            } else {
                None
            },
        }
    }
}

/// Summary information for trigger monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSummary {
    pub id: Uuid,
    pub name: String,
    pub source: String,
    pub target: String,
    pub status: TriggerStatus,
    pub event_type: String,
    pub event_source: String,
    pub action_name: String,
    pub action_status: ActionStatus,
    pub attempt_count: u32,
    pub age_seconds: i64,
    pub processing_duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_trigger_creation() {
        let event = Event::new(
            "test_event".to_string(),
            json!({"test": "data"}),
            "test_source".to_string(),
        );
        
        let action = Action::new(
            "Test Action".to_string(),
            "Test action".to_string(),
            "test_source".to_string(),
            "test_service".to_string(),
        );
        
        let condition = TriggerCondition {
            event_type_pattern: "test_*".to_string(),
            source_pattern: None,
            payload_conditions: vec![],
            min_priority: None,
            time_conditions: None,
        };
        
        let trigger = Trigger::new(
            "Test Trigger".to_string(),
            "Test trigger".to_string(),
            "listener_service".to_string(),
            "action_service".to_string(),
            event,
            action,
            condition,
        );
        
        assert_eq!(trigger.name, "Test Trigger");
        assert_eq!(trigger.status, TriggerStatus::Pending);
        assert!(trigger.can_process());
    }

    #[test]
    fn test_trigger_event_matching() {
        let event = Event::new(
            "user_created".to_string(),
            json!({"user_id": "123"}),
            "user_service".to_string(),
        );
        
        let action = Action::default();
        
        let condition = TriggerCondition {
            event_type_pattern: "user_*".to_string(),
            source_pattern: Some("user_service".to_string()),
            payload_conditions: vec![],
            min_priority: None,
            time_conditions: None,
        };
        
        let trigger = Trigger::new(
            "User Event Trigger".to_string(),
            "Triggers on user events".to_string(),
            "listener".to_string(),
            "processor".to_string(),
            event.clone(),
            action,
            condition,
        );
        
        // Should match the same event
        assert!(trigger.matches_event(&event));
        
        // Should not match different event type
        let different_event = Event::new(
            "order_created".to_string(),
            json!({}),
            "user_service".to_string(),
        );
        assert!(!trigger.matches_event(&different_event));
    }

    #[test]
    fn test_trigger_processing_lifecycle() {
        let event = Event::default();
        let action = Action::default();
        let condition = TriggerCondition {
            event_type_pattern: "*".to_string(),
            source_pattern: None,
            payload_conditions: vec![],
            min_priority: None,
            time_conditions: None,
        };
        
        let mut trigger = Trigger::new(
            "Test".to_string(),
            "Test".to_string(),
            "listener".to_string(),
            "processor".to_string(),
            event,
            action,
            condition,
        );
        
        // Start processing
        assert!(trigger.start_processing("worker-1".to_string()).is_ok());
        assert_eq!(trigger.status, TriggerStatus::Processing);
        assert_eq!(trigger.attempt_count, 1);
        
        // Complete processing
        trigger.complete_processing(Some(json!({"result": "success"})));
        assert_eq!(trigger.status, TriggerStatus::Completed);
        assert!(trigger.processed_at.is_some());
    }

    #[test]
    fn test_trigger_pattern_matching() {
        let event = Event::default();
        let action = Action::default();
        let condition = TriggerCondition {
            event_type_pattern: "*".to_string(),
            source_pattern: None,
            payload_conditions: vec![],
            min_priority: None,
            time_conditions: None,
        };
        
        let trigger = Trigger::new(
            "Test".to_string(),
            "Test".to_string(),
            "listener".to_string(),
            "processor".to_string(),
            event,
            action,
            condition,
        );
        
        // Test wildcard matching
        assert!(trigger.matches_pattern("*", "anything"));
        assert!(trigger.matches_pattern("user_*", "user_created"));
        assert!(trigger.matches_pattern("*_event", "test_event"));
        assert!(!trigger.matches_pattern("user_*", "order_created"));
        
        // Test exact matching
        assert!(trigger.matches_pattern("exact_match", "exact_match"));
        assert!(!trigger.matches_pattern("exact_match", "different"));
    }
}