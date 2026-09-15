//! Dynamic Flow Generation Example
//!
//! This example demonstrates runtime flow generation based on configuration,
//! user input, or system state - enabling adaptive workflow orchestration.
//!
//! # Concepts Demonstrated
//!
//! - **Runtime Flow Generation**: Create flows dynamically based on conditions
//! - **Conditional Logic**: Choose different flows based on input characteristics
//! - **Configuration-Driven**: Load workflows from external config
//! - **Adaptive Workflows**: Adjust execution patterns based on context
//! - **Flow Validation**: Validate dynamically generated flows before execution
//!
//! # Use Cases
//!
//! - User-configurable workflows
//! - Adaptive processing based on input complexity
//! - A/B testing different orchestration strategies
//! - Multi-tenant systems with custom flows per tenant
//! - Dynamic resource allocation based on load
//!
//! Run with: `cargo run --example maneuver_dynamic_flow`

use async_trait::async_trait;
use paladin::application::services::battalion::flow_visualizer::{
    FlowVisualizer, VisualizationFormat,
};
use paladin::application::services::battalion::maneuver_service::ManeuverExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::maneuver::{
    ErrorStrategy, Maneuver, ManeuverConfig, OutputFormat,
};
use paladin::core::platform::container::battalion::parser::FlowParser;
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock PaladinPort
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("   🤖 {} processing...", paladin.node.name);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let output = match paladin.node.name.as_str() {
            "simple_processor" => format!("[Simple] Processed: {}", input),
            "quick_check" => "[Quick] Basic validation passed".to_string(),
            "validator" => "[Validator] Input validated and sanitized".to_string(),
            "analyzer" => "[Analyzer] Deep analysis completed".to_string(),
            "transformer" => "[Transformer] Data transformed".to_string(),
            "enricher" => "[Enricher] Additional context added".to_string(),
            "expert_reviewer" => "[Expert] Comprehensive review completed".to_string(),
            "quality_check" => "[QA] Quality standards verified".to_string(),
            "optimizer" => "[Optimizer] Output optimized".to_string(),
            "finalizer" => "[Finalizer] Final output prepared".to_string(),
            _ => format!("{} completed", paladin.node.name),
        };

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
            execution_time_ms: 50,
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
        user_name: "System".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(2),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

/// Analyze input to determine appropriate workflow complexity
#[derive(Debug, Clone, Copy)]
enum WorkflowComplexity {
    Simple,
    Medium,
    Complex,
}

fn analyze_input(input: &str) -> WorkflowComplexity {
    let word_count = input.split_whitespace().count();
    let has_special_requirements = input.to_lowercase().contains("detailed")
        || input.to_lowercase().contains("comprehensive")
        || input.to_lowercase().contains("expert");

    if word_count < 10 && !has_special_requirements {
        WorkflowComplexity::Simple
    } else if word_count < 30 && !has_special_requirements {
        WorkflowComplexity::Medium
    } else {
        WorkflowComplexity::Complex
    }
}

/// Generate flow expression based on complexity
fn generate_flow_for_complexity(complexity: WorkflowComplexity) -> String {
    match complexity {
        WorkflowComplexity::Simple => {
            // Simple: quick validation only
            "quick_check -> simple_processor".to_string()
        }
        WorkflowComplexity::Medium => {
            // Medium: validation, parallel processing, finalization
            "validator -> (analyzer, transformer) -> finalizer".to_string()
        }
        WorkflowComplexity::Complex => {
            // Complex: full pipeline with parallel experts
            "validator -> (analyzer, enricher) -> (expert_reviewer, quality_check) -> optimizer -> finalizer".to_string()
        }
    }
}

/// Create agent pool (lazy initialization - only create agents actually used)
fn create_agent_pool() -> HashMap<String, Paladin> {
    let mut agents = HashMap::new();

    // Simple workflow agents
    agents.insert(
        "simple_processor".to_string(),
        create_paladin("simple_processor", "Simple processing"),
    );
    agents.insert(
        "quick_check".to_string(),
        create_paladin("quick_check", "Quick validation"),
    );

    // Medium workflow agents
    agents.insert(
        "validator".to_string(),
        create_paladin("validator", "Input validation"),
    );
    agents.insert(
        "analyzer".to_string(),
        create_paladin("analyzer", "Deep analysis"),
    );
    agents.insert(
        "transformer".to_string(),
        create_paladin("transformer", "Data transformation"),
    );
    agents.insert(
        "finalizer".to_string(),
        create_paladin("finalizer", "Final output preparation"),
    );

    // Complex workflow agents
    agents.insert(
        "enricher".to_string(),
        create_paladin("enricher", "Context enrichment"),
    );
    agents.insert(
        "expert_reviewer".to_string(),
        create_paladin("expert_reviewer", "Expert review"),
    );
    agents.insert(
        "quality_check".to_string(),
        create_paladin("quality_check", "Quality assurance"),
    );
    agents.insert(
        "optimizer".to_string(),
        create_paladin("optimizer", "Output optimization"),
    );

    agents
}

/// Execute workflow with dynamically generated flow
async fn execute_dynamic_workflow(
    input: &str,
    service: &ManeuverExecutionService,
    agent_pool: &HashMap<String, Paladin>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(70));
    println!("📥 Input: {}", input);

    // Step 1: Analyze input
    let complexity = analyze_input(input);
    println!("🔍 Detected Complexity: {:?}", complexity);

    // Step 2: Generate appropriate flow
    let flow_expression = generate_flow_for_complexity(complexity);
    println!("📝 Generated Flow: {}", flow_expression);

    // Step 3: Parse and validate flow
    let flow = FlowParser::parse(&flow_expression)?;
    println!("✅ Flow parsed successfully");

    // Step 4: Visualize
    println!("\n📊 Flow Structure:");
    let ascii_viz = FlowVisualizer::visualize(&flow, VisualizationFormat::Ascii);
    println!("{}", ascii_viz);

    // Step 5: Extract required agents from flow
    let required_agents = flow.agent_names();
    println!("\n🏗️  Required Agents: {:?}", required_agents);

    // Filter agent pool to only required agents
    let mut workflow_agents = HashMap::new();
    for agent_name in required_agents {
        if let Some(agent) = agent_pool.get(&agent_name) {
            workflow_agents.insert(agent_name.clone(), agent.clone());
        }
    }
    println!("✓ Loaded {} agents from pool", workflow_agents.len());

    // Step 6: Configure based on complexity
    let config = match complexity {
        WorkflowComplexity::Simple => ManeuverConfig::new()
            .with_error_strategy(ErrorStrategy::FailFast)
            .with_output_format(OutputFormat::Concatenate),
        WorkflowComplexity::Medium => ManeuverConfig::new()
            .with_error_strategy(ErrorStrategy::IgnoreErrors)
            .with_output_format(OutputFormat::Concatenate)
            .with_timing_metrics(true),
        WorkflowComplexity::Complex => ManeuverConfig::new()
            .with_error_strategy(ErrorStrategy::IgnoreErrors)
            .with_output_format(OutputFormat::JsonArray)
            .with_timing_metrics(true)
            .with_detailed_observability(true),
    };

    // Step 7: Create and execute Maneuver
    let maneuver = Maneuver::new("dynamic-workflow", workflow_agents, flow, config)?;

    let start = std::time::Instant::now();
    let result = service.execute(&maneuver, input).await?;
    let elapsed = start.elapsed();

    // Step 8: Display results
    println!("\n✅ Execution Complete ({}ms)", elapsed.as_millis());
    println!("\n📋 Execution Order: {:?}", result.execution_order);

    if let Some(metrics) = result.timing_metrics {
        println!("\n⏱️  Agent Timings:");
        for (agent, duration) in metrics {
            println!("   {} - {}ms", agent, duration.as_millis());
        }
    }

    println!("\n📤 Final Output:");
    println!("{}", result.final_output);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Dynamic Flow Generation Example");
    println!("{}", "=".repeat(70));
    println!("\nThis example demonstrates adaptive workflow orchestration");
    println!("where the flow expression is generated at runtime based on");
    println!("input characteristics and requirements.\n");

    // Create shared agent pool
    println!("🏗️  Creating Agent Pool...");
    let agent_pool = create_agent_pool();
    println!("✓ Initialized pool with {} agents", agent_pool.len());

    // Create service
    let paladin_port = Arc::new(ExamplePaladinPort);
    let service = ManeuverExecutionService::new(paladin_port);

    // Test Case 1: Simple input → Simple workflow
    println!("\n{}", "█".repeat(70));
    println!("TEST CASE 1: Simple Input → Lightweight Workflow");
    println!("{}", "█".repeat(70));

    execute_dynamic_workflow("Hello world", &service, &agent_pool).await?;

    // Test Case 2: Medium input → Medium complexity workflow
    println!("\n{}", "█".repeat(70));
    println!("TEST CASE 2: Medium Input → Balanced Workflow");
    println!("{}", "█".repeat(70));

    execute_dynamic_workflow(
        "Analyze the performance characteristics of the new database indexing strategy",
        &service,
        &agent_pool,
    )
    .await?;

    // Test Case 3: Complex input → Full pipeline
    println!("\n{}", "█".repeat(70));
    println!("TEST CASE 3: Complex Input → Comprehensive Workflow");
    println!("{}", "█".repeat(70));

    execute_dynamic_workflow(
        "Provide a detailed and comprehensive expert analysis of the proposed \
         microservices architecture migration strategy, including technical feasibility, \
         business impact, and risk assessment",
        &service,
        &agent_pool,
    )
    .await?;

    // Summary
    println!("\n{}", "=".repeat(70));
    println!("📊 SUMMARY: Dynamic Flow Generation");
    println!("{}", "=".repeat(70));

    println!("\n💡 Key Concepts:");
    println!("   1. Flow expressions generated at runtime");
    println!("   2. Workflow complexity adapts to input requirements");
    println!("   3. Agent pool enables efficient resource utilization");
    println!("   4. Configuration adjusts to match workflow complexity");
    println!("   5. Same codebase handles simple to complex scenarios");

    println!("\n🎯 Real-World Applications:");
    println!("   • Multi-tenant SaaS with custom workflows per customer");
    println!("   • Content processing pipelines adapting to document type");
    println!("   • API rate limiting by dynamically adjusting parallelism");
    println!("   • A/B testing different orchestration strategies");
    println!("   • Auto-scaling based on system load and input complexity");

    println!("\n📈 Benefits:");
    println!("   ✓ Resource efficiency - only use what's needed");
    println!("   ✓ User experience - fast responses for simple queries");
    println!("   ✓ Maintainability - single codebase for all cases");
    println!("   ✓ Flexibility - easy to add new complexity tiers");
    println!("   ✓ Cost optimization - lower compute for simple workflows");

    Ok(())
}
