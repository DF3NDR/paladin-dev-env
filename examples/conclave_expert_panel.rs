//! Conclave Expert Panel Example
//!
//! This example demonstrates the Conclave pattern - the Mixture-of-Agents approach
//! where multiple expert Paladins analyze a task in parallel, then an aggregator
//! synthesizes their diverse perspectives into a comprehensive response.
//!
//! # Concepts Demonstrated
//!
//! - **Expert Parallel Execution**: Multiple specialized Paladins process input simultaneously
//! - **Diverse Perspectives**: Each expert brings unique expertise (Technical, Business, Security)
//! - **Synthesis Aggregation**: Aggregator combines expert outputs into coherent response
//! - **Retry Logic**: Automatic retry with exponential backoff for failed experts
//! - **Partial Success Handling**: Continue even if some experts fail
//! - **Observability Levels**: Control logging detail (Minimal/Standard/Verbose)
//!
//! # Use Cases
//!
//! - Multi-perspective analysis requiring technical, business, and security views
//! - Decision-making that benefits from diverse expert opinions
//! - Code review with specialized reviewers (security, performance, maintainability)
//! - Architectural decisions requiring cross-functional input
//! - Product feature evaluation from multiple stakeholder perspectives
//!
//! # Architecture
//!
//! ```text
//!                     ┌──────────────┐
//!                     │   Input      │
//!                     │   Query      │
//!                     └──────┬───────┘
//!                            │
//!          ┌─────────────────┼─────────────────┐
//!          │                 │                 │
//!          ▼                 ▼                 ▼
//!   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
//!   │  Technical  │   │  Business   │   │  Security   │
//!   │   Expert    │   │   Expert    │   │   Expert    │
//!   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
//!          │                 │                 │
//!          └─────────────────┼─────────────────┘
//!                            │
//!                            ▼
//!                     ┌─────────────┐
//!                     │ Aggregator  │
//!                     │  Synthesis  │
//!                     └──────┬──────┘
//!                            │
//!                            ▼
//!                     ┌─────────────┐
//!                     │   Final     │
//!                     │  Response   │
//!                     └─────────────┘
//! ```
//!
//! Run with: `cargo run --example conclave_expert_panel`

use async_trait::async_trait;
use paladin::application::services::battalion::conclave_execution_service::ConclaveExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::BattalionConfig;
use paladin::core::platform::container::battalion::conclave::{
    Conclave, ConclaveConfig, ObservabilityLevel,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin::core::platform::container::token_usage::TokenUsage;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, StopReason};
use std::sync::Arc;

/// Example mock implementation of PaladinPort
/// In a real application, this would call actual LLM APIs
struct ExamplePaladinPort;

#[async_trait]
impl PaladinPort for ExamplePaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Simulate API latency
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Generate expert-specific analysis based on Paladin role
        let output = match paladin.node.name.as_str() {
            "TechnicalExpert" => format!(
                "TECHNICAL ANALYSIS:\n\
                 \n\
                 Architecture Assessment:\n\
                 - The proposed {} requires careful consideration of scalability\n\
                 - Recommend microservices architecture with API gateway pattern\n\
                 - Database sharding will be critical for horizontal scaling\n\
                 - Implement circuit breakers for resilience\n\
                 \n\
                 Performance Considerations:\n\
                 - Expect 3x increase in API latency during migration\n\
                 - Cache layer (Redis) essential for read-heavy workloads\n\
                 - Consider eventual consistency for non-critical data\n\
                 \n\
                 Technology Stack:\n\
                 - Rust for high-performance services\n\
                 - Kubernetes for orchestration\n\
                 - PostgreSQL with read replicas",
                input
            ),
            "BusinessExpert" => format!(
                "BUSINESS ANALYSIS:\n\
                 \n\
                 Market Opportunity:\n\
                 - The {} aligns with current market trends\n\
                 - Potential 25% increase in customer retention\n\
                 - Estimated 6-8 month ROI timeline\n\
                 \n\
                 Cost-Benefit Analysis:\n\
                 - Development cost: $200K-300K\n\
                 - Ongoing operational cost: $5K/month\n\
                 - Expected revenue lift: $100K/year\n\
                 \n\
                 Risk Assessment:\n\
                 - Medium implementation risk\n\
                 - Low market adoption risk\n\
                 - Competitive advantage window: 12-18 months\n\
                 \n\
                 Stakeholder Impact:\n\
                 - Engineering team needs 2 additional hires\n\
                 - Sales team requires new training materials\n\
                 - Customer success needs updated documentation",
                input
            ),
            "SecurityExpert" => format!(
                "SECURITY ANALYSIS:\n\
                 \n\
                 Threat Model:\n\
                 - {} introduces new attack surface\n\
                 - Primary threats: DDoS, data exfiltration, injection attacks\n\
                 - Threat level: Medium-High\n\
                 \n\
                 Recommended Controls:\n\
                 - Implement OAuth 2.0 + OIDC for authentication\n\
                 - Use RBAC with principle of least privilege\n\
                 - Encrypt data at rest (AES-256) and in transit (TLS 1.3)\n\
                 - Deploy WAF (Web Application Firewall)\n\
                 - Implement rate limiting and DDoS protection\n\
                 \n\
                 Compliance Requirements:\n\
                 - GDPR: Data residency and right to deletion\n\
                 - SOC 2 Type II: Audit logging required\n\
                 - HIPAA: If handling health data\n\
                 \n\
                 Security Testing:\n\
                 - Penetration testing before launch\n\
                 - Quarterly security audits\n\
                 - Continuous vulnerability scanning",
                input
            ),
            "SynthesisAggregator" => {
                // In real implementation, aggregator receives all expert outputs
                // Here we simulate the synthesis
                format!(
                    "SYNTHESIZED RECOMMENDATION:\n\
                     \n\
                     Executive Summary:\n\
                     Based on comprehensive analysis from technical, business, and security experts, \
                     the proposed {} is RECOMMENDED with specific conditions.\n\
                     \n\
                     Key Findings:\n\
                     1. Technical Feasibility: STRONG - Architecture is sound with proper planning\n\
                     2. Business Value: POSITIVE - Clear ROI with manageable risk\n\
                     3. Security Posture: ACCEPTABLE - With required controls implemented\n\
                     \n\
                     Critical Success Factors:\n\
                     • Allocate $250K budget and 6-month timeline\n\
                     • Hire 2 senior engineers with microservices experience\n\
                     • Implement security controls BEFORE launch (not after)\n\
                     • Plan for 3x API latency increase during migration\n\
                     • Establish quarterly security audit schedule\n\
                     \n\
                     Phased Implementation:\n\
                     Phase 1 (Months 1-2): Architecture design + security framework\n\
                     Phase 2 (Months 3-4): Core service development + testing\n\
                     Phase 3 (Months 5-6): Integration, security testing, launch\n\
                     \n\
                     Risk Mitigation:\n\
                     • Technical: Use proven patterns, incremental rollout\n\
                     • Business: Monitor KPIs weekly, adjust quickly\n\
                     • Security: Penetration testing, audit before launch\n\
                     \n\
                     Final Verdict: PROCEED with conditions above",
                    input
                )
            }
            _ => format!("{} analysis of: {}", paladin.node.name, input),
        };

        let token_estimate = (output.len() / 4) as u32; // Calculate before move

        Ok(PaladinResult {
            output,
            usage: TokenUsage::new(token_estimate, 0),
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

/// Helper function to create a Paladin with specified configuration
fn create_expert(name: &str, system_prompt: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: system_prompt.to_string(),
        name: name.to_string(),
        user_name: "User".to_string(),
        model: "gpt-4o".to_string(),
        temperature: 0.7,
        max_loops: MaxLoops::Fixed(1),
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Node::new(data, Some(name.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️  Conclave Expert Panel Example - Mixture of Agents\n");
    println!("========================================================\n");

    // Example 1: Basic Expert Panel with 3 Experts
    example_1_basic_expert_panel().await?;

    // Example 2: Custom Configuration Options
    example_2_custom_configuration().await?;

    // Example 3: Handling Partial Failures
    example_3_partial_failures().await?;

    println!("\n✅ All Conclave examples completed successfully!");
    Ok(())
}

/// Example 1: Basic Expert Panel
///
/// Demonstrates the simplest Conclave usage with 3 experts and an aggregator.
/// This is the recommended starting point for most use cases.
async fn example_1_basic_expert_panel() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Example 1: Basic Expert Panel");
    println!("----------------------------------");
    println!("Scenario: Multi-perspective analysis of a technical initiative\n");

    // Create the PaladinPort adapter for executing Paladins
    let paladin_port: Arc<dyn PaladinPort> = Arc::new(ExamplePaladinPort);

    // Step 1: Create Expert Paladins
    // Each expert has a specialized role and perspective
    println!("Creating expert Paladins...");

    let technical_expert = create_expert(
        "TechnicalExpert",
        "You are a senior technical architect with expertise in distributed systems, \
         microservices, and cloud infrastructure. Analyze the input from a technical \
         perspective, focusing on architecture, scalability, performance, and implementation.",
    );

    let business_expert = create_expert(
        "BusinessExpert",
        "You are a business strategist and product manager. Analyze the input from a \
         business perspective, focusing on market fit, ROI, cost-benefit analysis, \
         stakeholder impact, and competitive advantages.",
    );

    let security_expert = create_expert(
        "SecurityExpert",
        "You are a security expert specializing in application security, threat modeling, \
         and compliance. Analyze the input from a security perspective, identifying risks, \
         required controls, compliance requirements, and security testing needs.",
    );

    // Step 2: Create Aggregator Paladin
    // The aggregator synthesizes expert outputs into a coherent final response
    println!("Creating aggregator Paladin...");

    let aggregator = create_expert(
        "SynthesisAggregator",
        "You are a synthesis expert who combines multiple perspectives into a coherent, \
         comprehensive analysis. You will receive technical, business, and security analyses. \
         Your role is to:\n\
         1. Identify common themes and agreements\n\
         2. Highlight unique insights from each expert\n\
         3. Resolve contradictions by weighing evidence\n\
         4. Synthesize a balanced recommendation\n\
         5. Provide clear action items and risk mitigation strategies",
    );

    // Step 3: Configure the Conclave
    // Set up Battalion configuration and Conclave-specific options
    println!("Configuring Conclave...");

    let battalion_config = BattalionConfig::new("expert_panel_conclave")
        .with_description("Multi-expert analysis with synthesis")
        .with_timeout(300); // 5 minute timeout

    let conclave_config = ConclaveConfig::new("expert_panel", battalion_config)
        .with_timeout(300)
        .with_retry_attempts(2) // Retry failed experts up to 2 times
        .with_observability(ObservabilityLevel::Standard) // Standard logging
        .with_expert_names(true); // Include expert names in aggregator input

    // Step 4: Create the Conclave
    // Combine experts, aggregator, and configuration
    println!("Building Conclave...");

    let conclave = Conclave::new(
        vec![technical_expert, business_expert, security_expert],
        aggregator,
        conclave_config,
    )?;

    println!(
        "✅ Conclave created with {} experts\n",
        conclave.expert_count()
    );

    // Step 5: Execute the Conclave
    // Create execution service and run the analysis
    println!("Executing Conclave analysis...");

    let conclave_service = ConclaveExecutionService::new(paladin_port);

    let input = "migrate our monolithic application to a microservices architecture";
    println!("Input: '{}'\n", input);

    let start = std::time::Instant::now();
    let result = conclave_service.execute(&conclave, input).await?;
    let duration = start.elapsed();

    // Step 6: Display Results
    println!("⏱️  Execution completed in {:.2}s", duration.as_secs_f64());
    println!("📊 Status: {:?}", result.status);
    println!(
        "👥 Expert Success Rate: {}/{}\n",
        result.successful_expert_count(),
        conclave.expert_count()
    );

    // Show individual expert outputs (truncated for readability)
    println!("📝 Expert Outputs:");
    println!("{}", "─".repeat(80));
    for (expert_name, expert_result) in result.expert_outputs.iter() {
        println!("\n🔹 {}:", expert_name);
        let lines: Vec<&str> = expert_result.output.lines().collect();
        for line in lines.iter().take(5) {
            println!("  {}", line);
        }
        if lines.len() > 5 {
            println!("  ... ({} more lines)", lines.len() - 5);
        }
        println!(
            "  ⏱️  {}ms | 🔄 {} loops | 📊 {} tokens",
            expert_result.execution_time_ms,
            expert_result.loop_count,
            expert_result.usage.total_tokens
        );
    }

    // Show final synthesized output
    println!("\n{}", "═".repeat(80));
    println!("🎯 SYNTHESIZED RECOMMENDATION:");
    println!("{}", "═".repeat(80));
    println!("{}", result.aggregated_output.output);
    println!("{}", "═".repeat(80));
    println!(
        "📊 Aggregator: {}ms | {} tokens\n",
        result.aggregated_output.execution_time_ms, result.aggregated_output.usage.total_tokens
    );

    Ok(())
}

/// Example 2: Custom Configuration
///
/// Demonstrates advanced Conclave configuration options including custom synthesis prompts,
/// output truncation, and observability settings.
async fn example_2_custom_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Example 2: Custom Configuration");
    println!("------------------------------------");
    println!("Demonstrates: Custom synthesis prompt, output limits, verbose logging\n");

    let paladin_port: Arc<dyn PaladinPort> = Arc::new(ExamplePaladinPort);

    // Create experts
    let experts = vec![
        create_expert("TechnicalExpert", "Technical analysis expert"),
        create_expert("BusinessExpert", "Business analysis expert"),
        create_expert("SecurityExpert", "Security analysis expert"),
    ];

    let aggregator = create_expert("SynthesisAggregator", "Synthesis expert");

    // Custom configuration with advanced options
    let battalion_config = BattalionConfig::new("custom_conclave").with_timeout(600);

    let conclave_config = ConclaveConfig::new("custom", battalion_config)
        .with_timeout(600)
        .with_retry_attempts(3) // More aggressive retry
        .with_observability(ObservabilityLevel::Verbose) // Detailed logging
        .with_expert_names(true)
        .with_max_expert_tokens(500) // Limit expert output size
        .with_synthesis_prompt(
            "Combine the expert analyses focusing ONLY on technical feasibility. \
             Ignore business and security concerns for this analysis. \
             Provide a YES/NO recommendation with brief justification.",
        );

    let conclave = Conclave::new(experts, aggregator, conclave_config)?;

    println!("✅ Conclave configured with:");
    println!("   • Retry attempts: 3");
    println!("   • Max tokens per expert: 500");
    println!("   • Observability: Verbose");
    println!("   • Custom synthesis prompt: Focused on technical feasibility\n");

    let conclave_service = ConclaveExecutionService::new(paladin_port);
    let result = conclave_service
        .execute(&conclave, "implement GraphQL API layer")
        .await?;

    println!("✅ Execution completed");
    println!(
        "📊 Expert participation: {}/{}",
        result.successful_expert_count(),
        conclave.expert_count()
    );
    println!("\n🎯 Synthesized Output (with custom prompt):");
    println!("{}", result.aggregated_output.output);
    println!();

    Ok(())
}

/// Example 3: Handling Partial Failures
///
/// Demonstrates how Conclave handles scenarios where some experts fail but
/// aggregation can still proceed with available expert outputs.
async fn example_3_partial_failures() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Example 3: Partial Failure Handling");
    println!("----------------------------------------");
    println!("Demonstrates: Resilience when some experts fail\n");

    let paladin_port: Arc<dyn PaladinPort> = Arc::new(ExamplePaladinPort);

    // Create experts (in real scenarios, some might timeout or fail)
    let experts = vec![
        create_expert("TechnicalExpert", "Technical expert - may be slow"),
        create_expert("BusinessExpert", "Business expert - usually reliable"),
        create_expert("SecurityExpert", "Security expert - may have issues"),
    ];

    let aggregator = create_expert("SynthesisAggregator", "Aggregator with fallback");

    let battalion_config = BattalionConfig::new("resilient_conclave").with_timeout(300);

    let conclave_config = ConclaveConfig::new("resilient", battalion_config)
        .with_timeout(300)
        .with_retry_attempts(2)
        .with_observability(ObservabilityLevel::Standard);

    let conclave = Conclave::new(experts, aggregator, conclave_config)?;

    println!("📝 Note: Conclave will proceed with aggregation even if some experts fail");
    println!("   Minimum 1 successful expert required for aggregation\n");

    let conclave_service = ConclaveExecutionService::new(paladin_port);
    let result = conclave_service
        .execute(&conclave, "evaluate cloud provider options")
        .await?;

    // In this mock example, all succeed, but the code shows how to handle partial success
    let success_rate = result.successful_expert_count() as f64 / conclave.expert_count() as f64;

    println!("✅ Execution completed with partial success");
    println!(
        "📊 Success rate: {:.1}% ({}/{})",
        success_rate * 100.0,
        result.successful_expert_count(),
        conclave.expert_count()
    );

    if result.successful_expert_count() < conclave.expert_count() {
        println!("⚠️  Some experts failed, but aggregation proceeded with available outputs");
    } else {
        println!("✅ All experts succeeded");
    }

    println!("\n🎯 Aggregated Result:");
    println!("{}", result.aggregated_output.output);
    println!();

    Ok(())
}
