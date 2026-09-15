//! Examples for `docs/src/user-guides/agent-orchestrator-bridge.md`.
#![allow(unused_variables, unused_imports, dead_code)]

use std::collections::HashSet;
use std::sync::Arc;

use crate::support::{create_paladin, mock_executor, mock_orchestrator};

// ANCHOR: agent_triggers
use paladin_ports::output::orchestrator_port::{
    BridgeAction, BridgePolicy, FireEventRequest, OrchestratorBridgeError, OrchestratorPort,
};

/// An agent fires a domain event through the policy-guarded bridge.
pub async fn agent_triggers_orchestration() -> Result<(), Box<dyn std::error::Error>> {
    // Grant ONLY the actions this agent should perform, with explicit caps.
    let mut allowed = HashSet::new();
    allowed.insert(BridgeAction::FireEvent);
    let policy = BridgePolicy::new(allowed, 0, 0, 5, 0); // up to 5 events, nothing else

    // In production this is an `OrchestratorBridgeAdapter`; here a mock stands in.
    let bridge: Arc<dyn OrchestratorPort> = mock_orchestrator();
    let _ = &policy; // the real adapter is constructed as `::new(orchestrator, policy)`

    match bridge
        .fire_event(FireEventRequest {
            event_type: "critical_finding".to_string(),
            payload: serde_json::json!({ "severity": "high" }),
            source: "security-agent".to_string(),
        })
        .await
    {
        Ok(result) => println!("fired; {} trigger(s) matched", result.triggered_count),
        Err(OrchestratorBridgeError::ActionNotAllowed(_)) => {
            eprintln!("policy forbids this action")
        }
        Err(OrchestratorBridgeError::QuotaExceeded { .. }) => {
            eprintln!("per-execution cap reached")
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
// ANCHOR_END: agent_triggers

// ANCHOR: orchestration_invokes
use paladin_core::platform::container::paladin::Paladin;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;

/// A workflow step runs a single Paladin, passing context via the input string.
pub async fn orchestration_invokes_agent(
    analyst: &Paladin,
) -> Result<(), Box<dyn std::error::Error>> {
    let executor: Arc<dyn PaladinExecutorPort> = mock_executor();

    let upstream = "Q3 revenue rose 12% QoQ; churn fell to 2.1%.";
    let input = format!("Summarize the key risks given this context:\n{upstream}");

    let result = executor.execute(analyst, &input).await?;

    println!("agent said: {}", result.output);
    println!(
        "tokens: {}, stop reason: {:?}",
        result.usage.total_tokens, result.stop_reason
    );
    Ok(())
}
// ANCHOR_END: orchestration_invokes

// ANCHOR: bridge_policy
/// Build least-privilege and default bridge policies.
pub fn configure_bridge() {
    use paladin_ports::output::orchestrator_port::{BridgeAction, BridgePolicy};

    // Explicit, least-privilege: allow scheduling + notifications only,
    // with caps of (jobs=2, queue=0, events=0, notifications=5).
    let mut allowed = HashSet::new();
    allowed.insert(BridgeAction::ScheduleJob);
    allowed.insert(BridgeAction::SendNotification);
    let policy = BridgePolicy::new(allowed, 2, 0, 0, 5);

    // Builder-style: start from caps and add actions.
    let policy = BridgePolicy::new(HashSet::new(), 1, 1, 1, 1)
        .allow(BridgeAction::FireEvent)
        .allow(BridgeAction::QueueItem);

    // Conservative-but-usable default: all four actions, cap 3 each.
    let policy = BridgePolicy::default();
}
// ANCHOR_END: bridge_policy

// ANCHOR: recipe_news
use paladin_ports::output::orchestrator_port::SendNotificationRequest;

/// Recipe: notify the result of an AI summary through the bridge.
pub async fn recipe_news_notification(
    bridge: &Arc<dyn OrchestratorPort>,
    summary: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    bridge
        .send_notification(SendNotificationRequest {
            channel: "email".to_string(),
            recipient: "ops@example.com".to_string(),
            subject: "Daily news digest".to_string(),
            body: summary.to_string(),
        })
        .await?;
    Ok(())
}
// ANCHOR_END: recipe_news

// ANCHOR: recipe_schedule
use paladin_core::platform::container::schedule::Schedule;
use paladin_ports::output::orchestrator_port::{QueueItemRequest, ScheduleJobRequest};

/// Recipe: schedule a recurring batch job and enqueue an item.
pub async fn recipe_scheduled_batch(
    bridge: &Arc<dyn OrchestratorPort>,
    content_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    bridge
        .schedule_job(ScheduleJobRequest {
            name: "nightly-enrichment".to_string(),
            description: "Enrich the day's content with AI tags".to_string(),
            schedule: Schedule::Daily(2, 0), // 02:00 daily
        })
        .await?;

    bridge
        .queue_item(QueueItemRequest {
            queue_name: "enrichment".to_string(),
            payload: serde_json::json!({ "content_id": content_id }),
        })
        .await?;
    Ok(())
}
// ANCHOR_END: recipe_schedule

// ANCHOR: recipe_trigger
/// Recipe: an agent fires an event that a Trigger turns into a Paladin run.
pub async fn recipe_trigger_initiated(
    bridge: &Arc<dyn OrchestratorPort>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dispatch = bridge
        .fire_event(FireEventRequest {
            event_type: "anomaly_detected".to_string(),
            payload: serde_json::json!({ "metric": "latency_p99", "value": 920 }),
            source: "monitor-agent".to_string(),
        })
        .await?;

    println!("{} trigger(s) initiated", dispatch.triggered_count);
    Ok(())
}
// ANCHOR_END: recipe_trigger
