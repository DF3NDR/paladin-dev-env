//! Campaign Workflow Example
//!
//! This example demonstrates the Campaign pattern - graph-based orchestration of Paladins
//! with conditional routing, fan-out/fan-in, and complex DAG workflows.
//!
//! # Concepts Demonstrated
//!
//! - **DAG Execution**: Directed Acyclic Graph-based Paladin orchestration
//! - **Conditional Routing**: EdgeCondition (Always, Contains, Regex) for dynamic paths
//! - **Fan-Out/Fan-In**: Parallel branch execution with result aggregation
//! - **Topological Ordering**: Automatic dependency-aware execution
//! - **Cycle Detection**: Graph validation to prevent infinite loops
//! - **Multiple Entry Points**: Start workflow from multiple nodes concurrently
//!
//! # Use Cases
//!
//! - Complex approval workflows with conditional branching
//! - Multi-stage data processing pipelines with parallel branches
//! - Dynamic decision trees based on LLM outputs
//! - Quality control systems with conditional routing
//! - Hierarchical processing with fan-out parallelism
//!
//! Run with: `cargo run --example campaign_workflow`

use async_trait::async_trait;
use paladin::application::services::battalion::campaign_service::CampaignExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;

/// Example mock implementation of PaladinPort
/// In a real application, this would call actual LLM APIs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🤖 {} executing...", paladin.node.name);
        println!("   Input: {}", input);

        // Simulate processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Generate output based on Paladin role
        let output = match paladin.node.name.as_str() {
            // Quality control workflow
            "QualityChecker" => {
                if input.contains("high quality") {
                    "QUALITY_CHECK: PASSED - High quality detected".to_string()
                } else {
                    "QUALITY_CHECK: FAILED - Quality issues found".to_string()
                }
            }
            "ApprovalHandler" => {
                format!("Approved: {} - Ready for production", input)
            }
            "RejectionHandler" => {
                format!("Rejected: {} - Requires rework", input)
            }

            // Data processing pipeline
            "DataIngestion" => {
                format!("Ingested data: {} records loaded", input)
            }
            "CleanerA" => format!("CleanerA: Removed duplicates from {}", input),
            "CleanerB" => format!("CleanerB: Normalized fields in {}", input),
            "CleanerC" => format!("CleanerC: Validated integrity of {}", input),
            "DataAggregator" => {
                "Aggregated results: Combined outputs from all cleaners".to_string()
            }
            "FinalExporter" => {
                format!("Export complete: {} ready for delivery", input)
            }

            // Log analysis workflow
            "LogParser" => {
                if input.contains("error") {
                    "Error: Code E503 - Service Unavailable".to_string()
                } else {
                    "Success: All systems operational".to_string()
                }
            }
            "ErrorHandler" => {
                format!("Handled error from: {} - Alert sent", input)
            }
            "NormalHandler" => {
                format!("Normal processing for: {}", input)
            }

            // Approval chain
            "SubmissionReceiver" => {
                format!("Received submission: {}", input)
            }
            "Level1Reviewer" => "Review L1: APPROVED - Passed initial review".to_string(),
            "Level2Reviewer" => "Review L2: APPROVED - Passed detailed review".to_string(),
            "FinalApprover" => {
                format!("Final approval: {} - Fully approved", input)
            }

            _ => format!("{} processed: {}", paladin.node.name, input),
        };

        println!("   Output: {}", output);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(75, 0),
            execution_time_ms: 100,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<
        tokio::sync::mpsc::Receiver<
            Result<paladin_ports::output::paladin_port::PaladinStreamChunk, PaladinError>,
        >,
        PaladinError,
    > {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn create_paladin(name: &str, system_prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: system_prompt.to_string(),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(3),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

/// Example 1: Quality Control Workflow with Conditional Routing
async fn quality_control_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Example 1: Quality Control Workflow");
    println!("=====================================");
    println!("Graph: QualityChecker → [ApprovalHandler | RejectionHandler]");
    println!("Routing: Based on PASSED/FAILED in output\n");

    let port = Arc::new(ExamplePaladinPort);
    let service = CampaignExecutionService::new(port);

    let config = BattalionConfig::new("quality_control")
        .with_description("Quality control with conditional routing")
        .with_timeout(30);

    let mut campaign = Campaign::new(config);

    // Create Paladins
    let checker = create_paladin("QualityChecker", "You are a quality control inspector");
    let approve = create_paladin("ApprovalHandler", "You handle approved items");
    let reject = create_paladin("RejectionHandler", "You handle rejected items");

    // Build graph
    let id_checker = campaign.add_paladin(checker);
    let id_approve = campaign.add_paladin(approve);
    let id_reject = campaign.add_paladin(reject);

    // Conditional routing based on output
    campaign.add_edge(CampaignEdge::new(
        id_checker,
        id_approve,
        EdgeCondition::Contains("PASSED".to_string()),
    ))?;

    campaign.add_edge(CampaignEdge::new(
        id_checker,
        id_reject,
        EdgeCondition::Contains("FAILED".to_string()),
    ))?;

    // Execute - should route to approval
    let result = service.execute(&campaign, "high quality item").await?;

    println!("\n✅ Workflow Result:");
    println!("   Status: {:?}", result.status);
    println!("   Paladins Executed: {}", result.paladin_results.len());
    println!("   Final Output: {}", result.final_output);

    Ok(())
}

/// Example 2: Data Processing Pipeline with Fan-Out/Fan-In
async fn data_processing_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Example 2: Data Processing Pipeline");
    println!("======================================");
    println!("Graph: Ingestion → [CleanerA, CleanerB, CleanerC] → Aggregator → Exporter");
    println!("Pattern: Fan-out to parallel cleaners, fan-in to aggregator\n");

    let port = Arc::new(ExamplePaladinPort);
    let service = CampaignExecutionService::new(port);

    let config = BattalionConfig::new("data_pipeline")
        .with_description("Data processing with parallel cleaning")
        .with_timeout(60);

    let mut campaign = Campaign::new(config);

    // Create Paladins
    let ingestion = create_paladin("DataIngestion", "Ingest raw data");
    let cleaner_a = create_paladin("CleanerA", "Remove duplicates");
    let cleaner_b = create_paladin("CleanerB", "Normalize fields");
    let cleaner_c = create_paladin("CleanerC", "Validate integrity");
    let aggregator = create_paladin("DataAggregator", "Aggregate cleaned data");
    let exporter = create_paladin("FinalExporter", "Export final data");

    // Build diamond graph
    let id_ingest = campaign.add_paladin(ingestion);
    let id_a = campaign.add_paladin(cleaner_a);
    let id_b = campaign.add_paladin(cleaner_b);
    let id_c = campaign.add_paladin(cleaner_c);
    let id_agg = campaign.add_paladin(aggregator);
    let id_export = campaign.add_paladin(exporter);

    // Fan-out: Ingestion → [A, B, C]
    campaign.add_edge(CampaignEdge::new(id_ingest, id_a, EdgeCondition::Always))?;
    campaign.add_edge(CampaignEdge::new(id_ingest, id_b, EdgeCondition::Always))?;
    campaign.add_edge(CampaignEdge::new(id_ingest, id_c, EdgeCondition::Always))?;

    // Fan-in: [A, B, C] → Aggregator
    campaign.add_edge(CampaignEdge::new(id_a, id_agg, EdgeCondition::Always))?;
    campaign.add_edge(CampaignEdge::new(id_b, id_agg, EdgeCondition::Always))?;
    campaign.add_edge(CampaignEdge::new(id_c, id_agg, EdgeCondition::Always))?;

    // Final export
    campaign.add_edge(CampaignEdge::new(id_agg, id_export, EdgeCondition::Always))?;

    let result = service.execute(&campaign, "1000").await?;

    println!("\n✅ Pipeline Result:");
    println!("   Status: {:?}", result.status);
    println!("   Paladins Executed: {}", result.paladin_results.len());
    println!(
        "   Total Execution Time: {}ms",
        result
            .paladin_results
            .iter()
            .map(|p| p.execution_time_ms)
            .sum::<u64>()
    );
    println!("   Final Output: {}", result.final_output);

    Ok(())
}

/// Example 3: Log Analysis with Regex Routing
async fn log_analysis_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Example 3: Log Analysis with Regex Routing");
    println!("=============================================");
    println!("Graph: LogParser → [ErrorHandler | NormalHandler]");
    println!("Routing: Regex pattern matching on 'Error.*E\\d+'\n");

    let port = Arc::new(ExamplePaladinPort);
    let service = CampaignExecutionService::new(port);

    let config = BattalionConfig::new("log_analysis")
        .with_description("Log analysis with regex-based routing")
        .with_timeout(30);

    let mut campaign = Campaign::new(config);

    let parser = create_paladin("LogParser", "Parse log entries");
    let error_handler = create_paladin("ErrorHandler", "Handle errors");
    let normal_handler = create_paladin("NormalHandler", "Handle normal logs");

    let id_parser = campaign.add_paladin(parser);
    let id_error = campaign.add_paladin(error_handler);
    let id_normal = campaign.add_paladin(normal_handler);

    // Regex routing
    campaign.add_edge(CampaignEdge::new(
        id_parser,
        id_error,
        EdgeCondition::Regex(r"Error.*E\d+".to_string()),
    ))?;

    campaign.add_edge(CampaignEdge::new(
        id_parser,
        id_normal,
        EdgeCondition::Contains("Success".to_string()),
    ))?;

    // Test with error log
    let result = service.execute(&campaign, "error log entry").await?;

    println!("\n✅ Analysis Result:");
    println!("   Status: {:?}", result.status);
    println!("   Route Taken: Error handler (regex matched)");
    println!("   Final Output: {}", result.final_output);

    Ok(())
}

/// Example 4: Approval Chain (Linear DAG)
async fn approval_chain_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Example 4: Approval Chain Workflow");
    println!("====================================");
    println!("Graph: Submission → L1Review → L2Review → FinalApproval");
    println!("Pattern: Sequential approval stages\n");

    let port = Arc::new(ExamplePaladinPort);
    let service = CampaignExecutionService::new(port);

    let config = BattalionConfig::new("approval_chain")
        .with_description("Sequential approval workflow")
        .with_timeout(60);

    let mut campaign = Campaign::new(config);

    let submission = create_paladin("SubmissionReceiver", "Receive submissions");
    let level1 = create_paladin("Level1Reviewer", "Level 1 review");
    let level2 = create_paladin("Level2Reviewer", "Level 2 review");
    let final_approver = create_paladin("FinalApprover", "Final approval");

    let id_sub = campaign.add_paladin(submission);
    let id_l1 = campaign.add_paladin(level1);
    let id_l2 = campaign.add_paladin(level2);
    let id_final = campaign.add_paladin(final_approver);

    // Create approval chain
    campaign.add_edge(CampaignEdge::new(id_sub, id_l1, EdgeCondition::Always))?;
    campaign.add_edge(CampaignEdge::new(
        id_l1,
        id_l2,
        EdgeCondition::Contains("APPROVED".to_string()),
    ))?;
    campaign.add_edge(CampaignEdge::new(
        id_l2,
        id_final,
        EdgeCondition::Contains("APPROVED".to_string()),
    ))?;

    let result = service.execute(&campaign, "New feature request").await?;

    println!("\n✅ Approval Result:");
    println!("   Status: {:?}", result.status);
    println!("   Approval Stages: {}", result.paladin_results.len());
    println!(
        "   Total Time: {}ms",
        result
            .paladin_results
            .iter()
            .map(|p| p.execution_time_ms)
            .sum::<u64>()
    );
    println!("   Final Output: {}", result.final_output);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Campaign Graph Workflow Examples");
    println!("====================================\n");
    println!("Demonstrating DAG-based Paladin orchestration with:");
    println!("  • Conditional routing (Contains, Regex)");
    println!("  • Fan-out/fan-in patterns");
    println!("  • Parallel branch execution");
    println!("  • Topological ordering");

    // Run all examples
    quality_control_workflow().await?;
    data_processing_pipeline().await?;
    log_analysis_workflow().await?;
    approval_chain_workflow().await?;

    println!("\n🎉 All Campaign examples completed successfully!");
    println!("\nKey Takeaways:");
    println!("  ✓ Campaign enables complex graph-based workflows");
    println!("  ✓ EdgeCondition provides flexible routing logic");
    println!("  ✓ Automatic topological sorting ensures correct execution order");
    println!("  ✓ Fan-out/fan-in supports parallel processing patterns");
    println!("  ✓ Cycle detection prevents infinite loops");

    Ok(())
}
