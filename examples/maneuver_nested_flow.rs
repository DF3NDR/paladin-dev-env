//! Complex Nested Flow Example
//!
//! This example demonstrates advanced Maneuver patterns with deeply nested
//! sequential and parallel flows.
//!
//! # Concepts Demonstrated
//!
//! - **Deep Nesting**: Multiple levels of nested flow expressions
//! - **Complex Patterns**: Mixing sequential and parallel at multiple levels
//! - **Error Handling**: ContinueOnError strategy for resilient workflows
//! - **Output Aggregation**: JSON format for structured results
//! - **Real-World Workflow**: Multi-stage document review pipeline
//!
//! # Flow Structure
//!
//! ```text
//! Flow: "intake -> (technical -> (code_review, security_scan), business, legal) -> synthesis -> approval"
//!
//! Execution:
//! Input → intake → ┌─ technical → ┌─ code_review ─┐
//!                  │              └─ security_scan ─┘
//!                  ├─ business
//!                  └─ legal
//!                  → synthesis → approval → Output
//! ```
//!
//! # Use Cases
//!
//! - Enterprise approval workflows with multiple review stages
//! - Complex document processing with specialized validators
//! - Multi-level quality assurance pipelines
//! - Hierarchical decision-making workflows
//!
//! Run with: `cargo run --example maneuver_nested_flow`

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

/// Mock PaladinPort with realistic review outputs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        println!("\n🔍 {} reviewing...", paladin.node.name);

        // Variable processing times for realistic simulation
        let delay = match paladin.node.name.as_str() {
            "intake" => 50,
            "technical" => 150,
            "code_review" => 200,
            "security_scan" => 250,
            "business" => 100,
            "legal" => 180,
            "synthesis" => 120,
            "approval" => 80,
            _ => 100,
        };
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

        // Generate role-specific output
        let output = match paladin.node.name.as_str() {
            "intake" => {
                format!(
                    "✓ Intake Complete\n\
                     Document ID: DOC-2026-001\n\
                     Type: Technical Proposal\n\
                     Pages: 24\n\
                     Submitted: {}\n\
                     Status: Ready for Review",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                )
            }
            "technical" => {
                format!(
                    "🔧 Technical Assessment\n\
                     Architecture: Microservices-based\n\
                     Technology Stack: Rust, PostgreSQL, Redis\n\
                     Scalability: High (supports 10K+ RPS)\n\
                     Complexity: Medium\n\
                     Recommendation: Proceed to detailed code review\n\
                     Input Context: {}",
                    input.chars().take(100).collect::<String>()
                )
            }
            "code_review" => "💻 Code Review Results\n\
                 Files Reviewed: 47\n\
                 Issues Found: 3 minor\n\
                 Test Coverage: 87%\n\
                 Code Quality: A-\n\
                 Rust Idioms: Followed\n\
                 ✅ APPROVED with minor recommendations"
                .to_string(),
            "security_scan" => "🔒 Security Scan Report\n\
                 Vulnerabilities: 0 critical, 1 low\n\
                 Dependencies: All up-to-date\n\
                 Authentication: OAuth2 + JWT\n\
                 Encryption: TLS 1.3\n\
                 Compliance: SOC2, GDPR ready\n\
                 ✅ PASSED security review"
                .to_string(),
            "business" => "💼 Business Analysis\n\
                 Market Fit: Excellent\n\
                 ROI Estimate: 250% over 18 months\n\
                 Cost: $180K development + $20K/mo ops\n\
                 Timeline: 4 months\n\
                 Risk Level: Low-Medium\n\
                 ✅ Business case approved"
                .to_string(),
            "legal" => "⚖️  Legal Review\n\
                 Contract Terms: Standard approved\n\
                 IP Rights: Clear ownership\n\
                 Liability: Covered by insurance\n\
                 Compliance: All requirements met\n\
                 Data Privacy: GDPR compliant\n\
                 ✅ Legal approval granted"
                .to_string(),
            "synthesis" => {
                format!(
                    "📊 Comprehensive Synthesis\n\
                     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                     Technical: APPROVED (code + security passed)\n\
                     Business: APPROVED (strong ROI, low risk)\n\
                     Legal: APPROVED (compliant, clear terms)\n\
                     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
                     Overall Recommendation: PROCEED\n\
                     Next Steps: Final executive approval\n\
                     Combined Context: {}\n",
                    input.chars().take(150).collect::<String>()
                )
            }
            "approval" => {
                format!(
                    "🎯 FINAL APPROVAL\n\
                     ═══════════════════════════════════\n\
                     Authorized By: Executive Committee\n\
                     Date: {}\n\
                     Decision: APPROVED\n\
                     Budget Allocated: $200,000\n\
                     Start Date: Immediate\n\
                     Project Code: PROJ-2026-QTM\n\
                     ═══════════════════════════════════\n\
                     ✅ PROJECT GREENLIT - Proceed to implementation",
                    chrono::Utc::now().format("%Y-%m-%d")
                )
            }
            _ => format!("{} processed input", paladin.node.name),
        };

        println!("   ✓ Complete ({}ms)", delay);

        Ok(PaladinResult {
            output,
            usage: paladin_ports::output::llm_port::TokenUsage::new(150, 0),
            execution_time_ms: delay,
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
        user_name: "ReviewSystem".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.5, // Lower temperature for more consistent reviews
        max_loops: MaxLoops::Fixed(2),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏢 Complex Nested Flow Example: Enterprise Review Pipeline");
    println!("{}", "=".repeat(70));

    // Step 1: Define complex nested flow
    println!("\n📝 Step 1: Define Nested Flow Expression");
    let flow_expression = "intake -> (technical -> (code_review, security_scan), business, legal) -> synthesis -> approval";
    println!("Flow DSL:");
    println!("  {}", flow_expression);

    let flow = FlowParser::parse(flow_expression)?;
    println!("✅ Complex flow parsed successfully");

    // Step 2: Visualize nested structure
    println!("\n📊 Step 2: Visualize Nested Flow Structure");
    println!("{}", "-".repeat(70));
    let ascii_viz = FlowVisualizer::visualize(&flow, VisualizationFormat::Ascii);
    println!("{}", ascii_viz);
    println!("{}", "-".repeat(70));

    // Step 3: Create specialized Paladins
    println!("\n🏗️  Step 3: Create Specialized Review Paladins");
    let mut agents = HashMap::new();

    let agent_definitions = vec![
        ("intake", "Initial document intake and validation"),
        ("technical", "High-level technical feasibility assessment"),
        (
            "code_review",
            "Detailed code quality and architecture review",
        ),
        (
            "security_scan",
            "Security vulnerability and compliance analysis",
        ),
        ("business", "Business case and ROI analysis"),
        ("legal", "Legal compliance and contract review"),
        (
            "synthesis",
            "Synthesize all review outputs into recommendation",
        ),
        ("approval", "Executive approval decision"),
    ];

    for (name, role) in agent_definitions {
        agents.insert(name.to_string(), create_paladin(name, role));
        println!("   ✓ Created '{}' paladin - {}", name, role);
    }

    // Step 4: Configure with error resilience
    println!("\n⚙️  Step 4: Configure Maneuver for Production");
    let config = ManeuverConfig::new()
        .with_error_strategy(ErrorStrategy::IgnoreErrors) // Resilient workflow
        .with_output_format(OutputFormat::JsonArray) // Structured output
        .with_timing_metrics(true)
        .with_detailed_observability(true);

    println!("   ✓ Error strategy: ContinueOnError (resilient)");
    println!("   ✓ Output format: JSON (structured)");
    println!("   ✓ Observability: Detailed");

    // Step 5: Create Maneuver
    println!("\n🎖️  Step 5: Create Enterprise Review Maneuver");
    let maneuver = Maneuver::new("enterprise-review-pipeline", agents, flow, config)?;
    println!("   ✓ Maneuver '{}' created with 8 agents", maneuver.name);

    // Step 6: Execute workflow
    println!("\n🚀 Step 6: Execute Review Workflow");
    println!("{}", "=".repeat(70));

    let paladin_port = Arc::new(ExamplePaladinPort);
    let service = ManeuverExecutionService::new(paladin_port);

    let proposal = "Proposal: Implement quantum-resistant cryptography for secure communications";
    println!("\n📥 Input Proposal:");
    println!("   {}", proposal);

    let start = std::time::Instant::now();
    let result = service.execute(&maneuver, proposal).await?;
    let elapsed = start.elapsed();

    // Step 7: Analyze Results
    println!("\n{}", "=".repeat(70));
    println!("✅ Review Pipeline Complete");
    println!("{}", "=".repeat(70));

    println!("\n📋 Execution Flow:");
    for (idx, agent_name) in result.execution_order.iter().enumerate() {
        println!("   {:2}. {}", idx + 1, agent_name);
    }

    println!("\n⏱️  Performance Metrics:");
    println!("   Total Time: {}ms", elapsed.as_millis());
    if let Some(metrics) = &result.timing_metrics {
        let mut sorted_metrics: Vec<_> = metrics.iter().collect();
        sorted_metrics.sort_by_key(|(_, duration)| *duration);

        for (agent, duration) in sorted_metrics {
            println!("   {} - {}ms", agent, duration.as_millis());
        }

        let total_agent_time: u128 = metrics.values().map(|d| d.as_millis()).sum();
        let parallelization_efficiency =
            (total_agent_time as f64) / (elapsed.as_millis() as f64) * 100.0;
        println!(
            "\n   Parallelization Efficiency: {:.1}%",
            parallelization_efficiency
        );
    }

    println!("\n📊 Review Results:");
    println!("{}", "-".repeat(70));

    // Parse and display each agent's output
    for (agent_name, output) in result.step_outputs {
        println!("\n{}", agent_name.to_uppercase());
        println!("{}", output);
        println!("{}", "-".repeat(70));
    }

    println!("\n💡 Key Insights:");
    println!("   • Complex nested flows enable hierarchical workflows");
    println!("   • Parallel reviews (technical, business, legal) reduce latency");
    println!("   • Sequential stages ensure proper information flow");
    println!("   • Error resilience maintains workflow even if sub-tasks fail");
    println!("   • JSON output enables programmatic result processing");

    println!("\n📈 Workflow Statistics:");
    println!("   Total Stages: {}", result.execution_order.len());
    println!("   Parallel Branches: 3 (technical path, business, legal)");
    println!("   Nested Levels: 3 (intake → reviews → synthesis → approval)");
    println!(
        "   Time Savings: ~{:.0}% vs pure sequential",
        (1.0 - (elapsed.as_millis() as f64) / 1130.0) * 100.0
    );

    Ok(())
}
