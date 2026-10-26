//! Examples for `docs/src/user-guides/orchestration.md`.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use crate::support::{create_paladin, mock_orchestrator, mock_paladin_port, mock_scheduler};

// ANCHOR: formation
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_core::platform::container::aegis::{Aegis, TimeoutPolicy};
use paladin_core::platform::container::battalion::BattalionConfig;
use paladin_core::platform::container::battalion::formation::Formation;
use std::time::Duration;

/// Run three Paladins in sequence; each one's output feeds the next.
pub async fn run_formation() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();
    let extractor = create_paladin("Extractor");
    let analyzer = create_paladin("Analyzer");
    let writer = create_paladin("Writer");

    // No on_error handler: the first failure aborts the chain; each Paladin
    // attempt is bounded at 60 s.
    let config = BattalionConfig::new("earnings_pipeline").with_aegis(Aegis {
        timeout: Some(TimeoutPolicy {
            run_timeout: Some(Duration::from_secs(60)),
            idle_timeout: None,
        }),
        ..Aegis::default()
    });
    let formation = Formation::new(vec![extractor, analyzer, writer], config)?;

    let service = FormationExecutionService::new(paladin_port);
    let result = service
        .execute(&formation, "Raw Q3 earnings data...")
        .await?;

    println!("Final output: {}", result.final_output);
    Ok(())
}
// ANCHOR_END: formation

// ANCHOR: phalanx
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin_core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};

/// Fan the same input out to several Paladins concurrently, then aggregate.
pub async fn run_phalanx() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();
    let security = create_paladin("SecurityAuditor");
    let perf = create_paladin("PerformanceAnalyst");
    let style = create_paladin("StyleChecker");

    let phalanx = Phalanx::new(vec![security, perf, style], BattalionConfig::default())?
        .with_aggregation(AggregationStrategy::CollectAll)
        .with_max_concurrency(4); // cap concurrent Paladins

    let service = PhalanxExecutionService::new(paladin_port);
    let result = service
        .execute(&phalanx, "Review this Rust module...")
        .await?;

    println!("Aggregated: {}", result.final_output);
    Ok(())
}
// ANCHOR_END: phalanx

// ANCHOR: campaign
use paladin_battalion::campaign_service::CampaignExecutionService;
use paladin_core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};

/// Arrange Paladins as a DAG: `ingest → analyze → report`.
pub async fn run_campaign() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();

    let mut campaign = Campaign::new(BattalionConfig::default());
    let ingest = campaign.add_paladin(create_paladin("Ingest"));
    let analyze = campaign.add_paladin(create_paladin("Analyze"));
    let report = campaign.add_paladin(create_paladin("Report"));

    // Edges define dependencies; `EdgeCondition::Always` is unconditional.
    campaign.add_edge(CampaignEdge::new(ingest, analyze, EdgeCondition::Always))?;
    // A conditional edge only traverses when the upstream output matches:
    campaign.add_edge(CampaignEdge::new(
        analyze,
        report,
        EdgeCondition::Contains("ready".to_string()),
    ))?;
    campaign.set_entry_point(ingest)?;

    let service = CampaignExecutionService::new(paladin_port);
    let result = service.execute(&campaign, "Start").await?;

    println!("Campaign output: {}", result.final_output);
    Ok(())
}
// ANCHOR_END: campaign

// ANCHOR: chain_of_command
use paladin_battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin_core::platform::container::battalion::chain_of_command::ChainOfCommand;

/// A commander Paladin delegates to specialists and synthesizes their work.
pub async fn run_chain_of_command() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();
    let commander = create_paladin("Commander");
    let specialists = vec![
        create_paladin("BackendDev"),
        create_paladin("FrontendDev"),
        create_paladin("QaEngineer"),
    ];

    let chain = ChainOfCommand::new(commander, specialists, BattalionConfig::default())?;

    let service = ChainOfCommandExecutionService::new(paladin_port);
    let result = service.execute(&chain, "Build a login feature").await?;

    println!("Selected specialists: {:?}", result.selected_specialists);
    println!("Reasoning: {}", result.reasoning);
    for output in &result.outputs {
        println!("- {output}");
    }
    Ok(())
}
// ANCHOR_END: chain_of_command

// ANCHOR: commander
use paladin_battalion::commander::CommanderBuilder;
use paladin_core::platform::container::battalion::BattalionStrategy;

/// Let the Commander auto-select the best pattern for the input.
pub async fn run_commander_auto() -> Result<(), Box<dyn std::error::Error>> {
    let paladin_port = mock_paladin_port();

    let commander = CommanderBuilder::new(paladin_port)
        .strategy(BattalionStrategy::Auto)
        .paladins(vec![
            create_paladin("Analyzer"),
            create_paladin("Processor"),
            create_paladin("Synthesizer"),
        ])
        .build()?;

    let result = commander
        .execute("Analyze and summarize this report")
        .await?;

    println!("Strategy selected: {:?}", result.strategy_used);
    if let Some(reason) = &result.strategy_selection_reasoning {
        println!("Reasoning: {reason}");
    }
    println!("Output: {}", result.final_output);
    Ok(())
}
// ANCHOR_END: commander

// ANCHOR: scheduling
use paladin_ports::output::scheduler_port::{JobSpec, JobStatus, SchedulerPort};

/// Schedule a recurring job with a 6-field cron expression.
pub async fn run_scheduling() -> Result<(), Box<dyn std::error::Error>> {
    let scheduler: Arc<dyn SchedulerPort> = mock_scheduler();
    scheduler.start().await?;

    // 6-field cron: sec min hour day month weekday
    let spec = JobSpec::new("daily-digest", "0 0 9 * * *") // every day at 09:00:00
        .with_metadata("workflow", "news-digest");

    let job_id = scheduler.schedule_job(spec).await?;

    let status: JobStatus = scheduler.get_job_status(&job_id).await?;
    println!("job {job_id:?} is {status:?}");

    // Later: scheduler.cancel_job(&job_id).await?;
    Ok(())
}
// ANCHOR_END: scheduling

// ANCHOR: events
use paladin_core::base::entity::message::MessagePriority;
use paladin_core::platform::container::trigger::{TimeCondition, TriggerCondition, TriggerConfig};
use paladin_ports::output::orchestrator_port::{FireEventRequest, OrchestratorPort};

/// Build a trigger condition and fire a matching event.
pub async fn run_events() -> Result<(), Box<dyn std::error::Error>> {
    let condition = TriggerCondition {
        event_type_pattern: "critical_finding".to_string(),
        source_pattern: Some("security-*".to_string()),
        payload_conditions: vec![],
        min_priority: Some(MessagePriority::High),
        time_conditions: Some(TimeCondition {
            active_hours: Some((9, 17)),            // only 09:00–17:00
            active_days: Some(vec![1, 2, 3, 4, 5]), // Mon–Fri
            cooldown_seconds: Some(300),            // at most once per 5 min
        }),
    };

    let config = TriggerConfig {
        max_retries: 3,
        timeout_seconds: 60,
        preserve_after_completion: false,
        ttl_seconds: 3600,
        processing_priority: MessagePriority::High,
    };

    // Fire an event through the orchestrator bridge.
    let orchestrator = mock_orchestrator();
    let result = orchestrator
        .fire_event(FireEventRequest {
            event_type: "critical_finding".to_string(),
            payload: serde_json::json!({ "severity": "high", "cve": "CVE-2025-0001" }),
            source: "security-scanner".to_string(),
        })
        .await?;

    println!(
        "{} trigger(s) fired: {:?}",
        result.triggered_count, result.trigger_ids
    );
    Ok(())
}
// ANCHOR_END: events
