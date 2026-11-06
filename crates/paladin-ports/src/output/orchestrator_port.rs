//! Agent → Orchestrator bridge port.
//!
//! This module defines the [`OrchestratorPort`](crate::output::orchestrator_port::OrchestratorPort) abstraction that lets a Paladin
//! agent, during execution, trigger a narrow, guarded set of orchestration
//! actions: schedule a job, enqueue a content item, fire an event, or send a
//! notification. It completes the *agent → orchestrator* direction of the
//! platform integration (the inverse of the *content → agent* bridge).
//!
//! # Design: port vs. Arsenal tool
//!
//! Two approaches were evaluated for exposing orchestration capabilities to an
//! agent:
//!
//! - **Option A — `OrchestratorPort` (chosen):** a port trait injected into the
//!   execution service alongside `LlmPort`/`ArsenalPort`. It maximizes
//!   decoupling and testability (mockable), and allows centralized safety
//!   enforcement via [`BridgePolicy`](crate::output::orchestrator_port::BridgePolicy) *before* any orchestrator call is made.
//! - **Option B — Arsenal tool (`OrchestratorArmament`):** register the
//!   capabilities as an LLM-discoverable tool. This is self-describing to the
//!   model but couples the capability to the tool-call format and spreads
//!   authorization across tool plumbing.
//!
//! Option A is adopted here. Option B is deferred as a non-breaking follow-up
//! that can wrap an `Arc<dyn OrchestratorPort>` without changing this trait.
//!
//! # Crate boundary
//!
//! This port and all of its value objects live in `paladin-ports` and depend
//! only on `paladin-core` domain types. The concrete adapter that drives the
//! real `Orchestrator` lives in the root crate, because it depends on the
//! root-crate `Orchestrator` (a lower crate cannot depend on the root crate
//! without a circular dependency).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use paladin_core::platform::container::schedule::Schedule;

/// The set of orchestration actions an agent may be permitted to trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BridgeAction {
    /// Schedule a recurring or one-off job.
    ScheduleJob,
    /// Enqueue a content item / payload for asynchronous processing.
    QueueItem,
    /// Fire a domain event through the orchestrator's listener path.
    FireEvent,
    /// Send a notification through a notification delivery channel.
    SendNotification,
}

impl BridgeAction {
    /// Human-readable identifier used in error messages.
    pub fn as_str(self) -> &'static str {
        match self {
            BridgeAction::ScheduleJob => "schedule_job",
            BridgeAction::QueueItem => "queue_item",
            BridgeAction::FireEvent => "fire_event",
            BridgeAction::SendNotification => "send_notification",
        }
    }
}

/// Request to schedule a job for recurring or one-off execution.
///
/// Note: this type holds a [`Schedule`], which is not `serde`-serializable, so
/// this request is not `Serialize`/`Deserialize` (unlike the other requests).
#[derive(Debug, Clone)]
pub struct ScheduleJobRequest {
    /// Human-readable job name.
    pub name: String,
    /// Description of what the job does.
    pub description: String,
    /// When the job should run.
    pub schedule: Schedule,
}

/// Request to enqueue a content item / payload onto a named queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemRequest {
    /// Target queue name.
    pub queue_name: String,
    /// Arbitrary JSON payload describing the item to process.
    pub payload: serde_json::Value,
}

/// Request to fire a domain event through the orchestrator's listener path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FireEventRequest {
    /// Event type/name (e.g., `"critical_finding"`).
    pub event_type: String,
    /// Arbitrary JSON payload carried by the event.
    pub payload: serde_json::Value,
    /// Source identifier for the event (e.g., the agent name).
    pub source: String,
}

/// Request to send a notification through a delivery channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendNotificationRequest {
    /// Delivery channel identifier (e.g., `"email"`, `"slack"`).
    pub channel: String,
    /// Recipient address/identifier appropriate for the channel.
    pub recipient: String,
    /// Notification subject/title.
    pub subject: String,
    /// Notification body.
    pub body: String,
}

/// Result of dispatching an event through the listener path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventDispatchResult {
    /// Number of triggers created in response to the event.
    pub triggered_count: usize,
    /// Identifiers of the triggers created.
    pub trigger_ids: Vec<Uuid>,
}

/// Errors returned by the agent → orchestrator bridge.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OrchestratorBridgeError {
    /// The requested action is not in the policy allow-list.
    #[error("Action not allowed by bridge policy: {0}")]
    ActionNotAllowed(String),

    /// A per-execution quantitative cap for the action was exceeded.
    #[error("Quota exceeded for action '{action}' (limit {limit})")]
    QuotaExceeded {
        /// The action whose cap was exceeded.
        action: String,
        /// The configured cap for that action.
        limit: u32,
    },

    /// The request value object failed validation.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// The underlying orchestrator/notification call failed. Stringified at the
    /// adapter boundary to avoid leaking root-crate error types into this crate.
    #[error("Orchestrator error: {0}")]
    OrchestratorError(String),
}

/// Guardrail policy controlling which orchestration actions an agent may take
/// and how many times per execution.
///
/// Enforcement is performed by the adapter *before* any underlying orchestrator
/// call is made: a disallowed action yields
/// [`OrchestratorBridgeError::ActionNotAllowed`] and a cap-exceeding action
/// yields [`OrchestratorBridgeError::QuotaExceeded`].
///
/// This is intentionally minimal — an allow-list plus simple caps — not a full
/// RBAC policy engine.
#[derive(Debug, Clone)]
pub struct BridgePolicy {
    allowed: HashSet<BridgeAction>,
    max_jobs_scheduled: u32,
    max_items_queued: u32,
    max_events_fired: u32,
    max_notifications_sent: u32,
}

impl BridgePolicy {
    /// Create a policy allowing the given actions with the given per-execution caps.
    pub fn new(
        allowed: HashSet<BridgeAction>,
        max_jobs_scheduled: u32,
        max_items_queued: u32,
        max_events_fired: u32,
        max_notifications_sent: u32,
    ) -> Self {
        Self {
            allowed,
            max_jobs_scheduled,
            max_items_queued,
            max_events_fired,
            max_notifications_sent,
        }
    }

    /// Returns `true` if the action is permitted by this policy.
    pub fn is_allowed(&self, action: BridgeAction) -> bool {
        self.allowed.contains(&action)
    }

    /// Returns the per-execution cap configured for the given action.
    pub fn cap_for(&self, action: BridgeAction) -> u32 {
        match action {
            BridgeAction::ScheduleJob => self.max_jobs_scheduled,
            BridgeAction::QueueItem => self.max_items_queued,
            BridgeAction::FireEvent => self.max_events_fired,
            BridgeAction::SendNotification => self.max_notifications_sent,
        }
    }

    /// Allow an additional action (builder-style).
    pub fn allow(mut self, action: BridgeAction) -> Self {
        self.allowed.insert(action);
        self
    }
}

impl Default for BridgePolicy {
    /// Conservative default: all four actions allowed, with a small per-execution
    /// cap of 3 for each.
    fn default() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(BridgeAction::ScheduleJob);
        allowed.insert(BridgeAction::QueueItem);
        allowed.insert(BridgeAction::FireEvent);
        allowed.insert(BridgeAction::SendNotification);
        Self {
            allowed,
            max_jobs_scheduled: 3,
            max_items_queued: 3,
            max_events_fired: 3,
            max_notifications_sent: 3,
        }
    }
}

/// Port through which an agent triggers a narrow set of orchestration actions.
///
/// Implementations are concrete adapters (in the root crate) that map each
/// method onto real `Orchestrator` capabilities, enforcing a [`BridgePolicy`]
/// before performing the underlying call.
///
/// # Examples
///
/// ```rust
/// use paladin_ports::output::orchestrator_port::{
///     OrchestratorPort, QueueItemRequest, OrchestratorBridgeError,
/// };
/// use uuid::Uuid;
///
/// async fn hand_off_to_queue(
///     bridge: &dyn OrchestratorPort,
///     payload: serde_json::Value,
/// ) -> Result<Uuid, OrchestratorBridgeError> {
///     let request = QueueItemRequest {
///         queue_name: "agent-followups".to_string(),
///         payload,
///     };
///
///     bridge.queue_item(request).await
/// }
/// ```
#[async_trait]
pub trait OrchestratorPort: Send + Sync {
    /// Schedule a job for recurring or one-off execution.
    async fn schedule_job(
        &self,
        request: ScheduleJobRequest,
    ) -> Result<Uuid, OrchestratorBridgeError>;

    /// Enqueue a content item / payload onto a named queue.
    async fn queue_item(&self, request: QueueItemRequest) -> Result<Uuid, OrchestratorBridgeError>;

    /// Fire a domain event through the orchestrator's listener path.
    async fn fire_event(
        &self,
        request: FireEventRequest,
    ) -> Result<EventDispatchResult, OrchestratorBridgeError>;

    /// Send a notification through a delivery channel.
    async fn send_notification(
        &self,
        request: SendNotificationRequest,
    ) -> Result<Uuid, OrchestratorBridgeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_allows_all_actions() {
        let policy = BridgePolicy::default();
        assert!(policy.is_allowed(BridgeAction::ScheduleJob));
        assert!(policy.is_allowed(BridgeAction::QueueItem));
        assert!(policy.is_allowed(BridgeAction::FireEvent));
        assert!(policy.is_allowed(BridgeAction::SendNotification));
    }

    #[test]
    fn default_policy_has_small_caps() {
        let policy = BridgePolicy::default();
        assert_eq!(policy.cap_for(BridgeAction::ScheduleJob), 3);
        assert_eq!(policy.cap_for(BridgeAction::QueueItem), 3);
        assert_eq!(policy.cap_for(BridgeAction::FireEvent), 3);
        assert_eq!(policy.cap_for(BridgeAction::SendNotification), 3);
    }

    #[test]
    fn empty_policy_disallows_actions() {
        let policy = BridgePolicy::new(HashSet::new(), 0, 0, 0, 0);
        assert!(!policy.is_allowed(BridgeAction::ScheduleJob));
        assert!(!policy.is_allowed(BridgeAction::FireEvent));
    }

    #[test]
    fn allow_builder_adds_action() {
        let policy = BridgePolicy::new(HashSet::new(), 1, 1, 1, 1).allow(BridgeAction::ScheduleJob);
        assert!(policy.is_allowed(BridgeAction::ScheduleJob));
        assert!(!policy.is_allowed(BridgeAction::QueueItem));
    }

    #[test]
    fn bridge_action_as_str_matches() {
        assert_eq!(BridgeAction::ScheduleJob.as_str(), "schedule_job");
        assert_eq!(BridgeAction::QueueItem.as_str(), "queue_item");
        assert_eq!(BridgeAction::FireEvent.as_str(), "fire_event");
        assert_eq!(BridgeAction::SendNotification.as_str(), "send_notification");
    }

    #[test]
    fn custom_caps_are_reported() {
        let mut allowed = HashSet::new();
        allowed.insert(BridgeAction::ScheduleJob);
        let policy = BridgePolicy::new(allowed, 5, 7, 9, 11);
        assert_eq!(policy.cap_for(BridgeAction::ScheduleJob), 5);
        assert_eq!(policy.cap_for(BridgeAction::QueueItem), 7);
        assert_eq!(policy.cap_for(BridgeAction::FireEvent), 9);
        assert_eq!(policy.cap_for(BridgeAction::SendNotification), 11);
    }
}
