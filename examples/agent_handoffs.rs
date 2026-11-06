// examples/agent_handoffs.rs
//
// Agent Handoffs Example
//
// This example demonstrates intelligent task delegation between specialized agents.
// When enabled, agents can:
// 1. Recognize when a subtask requires specialized expertise
// 2. Delegate to a more suitable specialist agent
// 3. Receive results and integrate them into the final output
//
// To run this example:
// ```bash
// cargo run --example agent_handoffs
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::autonomous_config::HandoffConfig;
use paladin::core::platform::container::handoff::HandoffStrategy;
use paladin::core::platform::container::herald::Herald;
use paladin::infrastructure::adapters::herald::MarkdownHerald;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤝 Agent Handoffs Example\n");
    println!("This example demonstrates intelligent task delegation between specialist agents\n");

    // Create specialized agents

    // 1. Database Specialist
    println!("👤 Creating Specialist Agents...\n");

    let db_llm = Arc::new(
        MockLlmAdapter::new().with_response(
            "I recommend the following database schema:\n\
             - users table: id (PK), email (unique), created_at\n\
             - posts table: id (PK), user_id (FK), title, content, published_at\n\
             - Indexes: users.email, posts.user_id, posts.published_at\n\
             - Consider partitioning posts by published_at for scalability"
                .to_string(),
        ),
    );

    let db_specialist = PaladinBuilder::new(db_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a database architect specializing in schema design, \
             optimization, and scalability. Focus on normalization, indexing, \
             and performance considerations.",
        )
        .name("DatabaseArchitect")
        .model("gpt-4o-mini")
        .temperature(0.3) // Low temp for technical precision
        .max_loops(1)
        .build()
        .await?;

    println!("   ✅ DatabaseArchitect created");

    // 2. Security Specialist
    let security_llm = Arc::new(
        MockLlmAdapter::new().with_response(
            "Security recommendations:\n\
         1. Authentication: Implement JWT with refresh tokens (15min access, 7d refresh)\n\
         2. Authorization: Role-based access control (RBAC) with user/admin/moderator roles\n\
         3. Input Validation: Use parameterized queries, sanitize all inputs\n\
         4. Password Security: bcrypt with cost factor 12, enforce 12+ char passwords\n\
         5. Rate Limiting: 100 req/min per IP, 1000 req/hour per user\n\
         6. HTTPS: Enforce TLS 1.3, HSTS headers\n\
         7. CSRF Protection: SameSite cookies, CSRF tokens for state-changing ops"
                .to_string(),
        ),
    );

    let security_specialist = PaladinBuilder::new(security_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a cybersecurity expert specializing in web application security, \
             authentication, authorization, and threat mitigation. Provide comprehensive \
             security recommendations following OWASP guidelines.",
        )
        .name("SecuritySpecialist")
        .model("gpt-4o-mini")
        .temperature(0.2) // Very low for security precision
        .max_loops(1)
        .build()
        .await?;

    println!("   ✅ SecuritySpecialist created");

    // 3. API Designer
    let api_llm = Arc::new(
        MockLlmAdapter::new().with_response(
            "RESTful API Design:\n\n\
             POST   /api/users         - Create user\n\
             GET    /api/users/:id     - Get user profile\n\
             PUT    /api/users/:id     - Update user\n\
             DELETE /api/users/:id     - Delete user\n\n\
             POST   /api/posts         - Create post\n\
             GET    /api/posts         - List posts (paginated)\n\
             GET    /api/posts/:id     - Get single post\n\
             PUT    /api/posts/:id     - Update post\n\
             DELETE /api/posts/:id     - Delete post\n\n\
             All endpoints return JSON, use standard HTTP status codes (200, 201, 400, 401, 404, 500)"
                .to_string(),
        ),
    );

    let api_specialist = PaladinBuilder::new(api_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are an API design expert focusing on RESTful design principles, \
             versioning strategies, and developer experience. Design clean, intuitive APIs.",
        )
        .name("ApiDesigner")
        .model("gpt-4o-mini")
        .temperature(0.4)
        .max_loops(1)
        .build()
        .await?;

    println!("   ✅ ApiDesigner created");
    println!();

    // 4. Coordinator Agent (with handoff capability)
    println!("👤 Creating Coordinator Agent with Handoff Capability...\n");

    let coordinator_llm = Arc::new(
        MockLlmAdapter::new()
            // Initial analysis
            .with_response(
                "I'll coordinate this project design by delegating to specialists:\n\
                 1. Database schema → DatabaseArchitect\n\
                 2. Security requirements → SecuritySpecialist\n\
                 3. API design → ApiDesigner"
                    .to_string(),
            )
            // After handoffs (simulated final synthesis)
            .with_response(
                "Project design complete! I've coordinated with specialists:\n\n\
                 📊 Database: Schema designed with users/posts tables and optimizations\n\
                 🔒 Security: Comprehensive security measures including JWT, RBAC, rate limiting\n\
                 🔌 API: RESTful endpoints designed following best practices\n\n\
                 All components are integrated and ready for implementation."
                    .to_string(),
            ),
    );

    // Configure handoffs
    let handoff_config = HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    };

    // Note: Specialist pool would be managed by HandoffService
    // For this example, we manually demonstrate delegation

    // Note: In actual implementation, config would enable handoffs
    // For this mock example, we demonstrate the configuration structure
    let coordinator = PaladinBuilder::new(coordinator_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt(
            "You are a project coordinator who delegates technical tasks to \
             appropriate specialists. Analyze tasks, identify specialist needs, \
             and synthesize results into cohesive solutions.",
        )
        .name("ProjectCoordinator")
        .model("gpt-4o-mini")
        .temperature(0.6)
        .max_loops(2)
        .build()
        .await?;

    println!("   ✅ ProjectCoordinator created");
    println!("   📋 Handoff Configuration:");
    println!("      Strategy: {:?}", handoff_config.strategy);
    println!("      Max Depth: {}", handoff_config.max_depth);
    println!("      Note: Specialist pool managed separately");
    println!();

    // Execute complex project requiring multiple specialists
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)));
    let herald: Arc<dyn Herald> = Arc::new(MarkdownHerald::new());

    let coordinator_service =
        PaladinExecutionService::new(coordinator_llm, circuit_breaker, None, None)
            .with_herald(herald);

    println!("🚀 Executing Complex Project...\n");

    let project_task = "Design a blog platform with user authentication, posts, and comments. \
                       I need database schema, security measures, and API endpoints.";

    println!("📝 Project Requirements:");
    println!("   {}", project_task);
    println!();

    println!("🔄 Delegation Flow:");
    println!();
    println!("   ProjectCoordinator (analyzing)");
    println!("   ├─→ DatabaseArchitect (delegated schema design)");
    println!("   ├─→ SecuritySpecialist (delegated security requirements)");
    println!("   └─→ ApiDesigner (delegated API design)");
    println!();
    println!("   ProjectCoordinator (synthesizing results)");
    println!();

    // Simulate handoffs by executing specialists manually
    // (In real implementation, HandoffService would automate this)

    println!("📊 Executing Handoff #1: Database Schema...");
    let db_service = PaladinExecutionService::new(
        db_llm,
        Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30))),
        None,
        None,
    )
    .with_herald(Arc::new(MarkdownHerald::new()));

    let db_result = db_service
        .execute(&db_specialist, "Design database schema for blog platform")
        .await?;
    println!("   ✅ DatabaseArchitect completed");
    println!();

    println!("🔒 Executing Handoff #2: Security Requirements...");
    let security_service = PaladinExecutionService::new(
        security_llm,
        Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30))),
        None,
        None,
    )
    .with_herald(Arc::new(MarkdownHerald::new()));

    let security_result = security_service
        .execute(
            &security_specialist,
            "Define security requirements for blog platform",
        )
        .await?;
    println!("   ✅ SecuritySpecialist completed");
    println!();

    println!("🔌 Executing Handoff #3: API Design...");
    let api_service = PaladinExecutionService::new(
        api_llm,
        Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30))),
        None,
        None,
    )
    .with_herald(Arc::new(MarkdownHerald::new()));

    let api_result = api_service
        .execute(&api_specialist, "Design RESTful API for blog platform")
        .await?;
    println!("   ✅ ApiDesigner completed");
    println!();

    println!("🔄 Coordinator synthesizing results...");
    let final_result = coordinator_service
        .execute(&coordinator, project_task)
        .await?;
    println!();

    // Display results
    println!("✨ Project Design Complete!\n");
    println!("{}", "=".repeat(70));
    println!("\n📊 Database Schema (from DatabaseArchitect):\n");
    println!("{}", db_result.output);
    println!();

    println!("{}", "=".repeat(70));
    println!("\n🔒 Security Requirements (from SecuritySpecialist):\n");
    println!("{}", security_result.output);
    println!();

    println!("{}", "=".repeat(70));
    println!("\n🔌 API Design (from ApiDesigner):\n");
    println!("{}", api_result.output);
    println!();

    println!("{}", "=".repeat(70));
    println!("\n🎯 Final Synthesis (from ProjectCoordinator):\n");
    println!("{}", final_result.output);
    println!();

    // Summary
    println!("{}", "=".repeat(70));
    println!("\n💡 Key Takeaways:\n");
    println!("   ✓ Coordinator delegated 3 specialized subtasks");
    println!("   ✓ Each specialist focused on their domain expertise");
    println!("   ✓ Results were synthesized into cohesive solution");
    println!("   ✓ Automatic strategy identifies best specialist per task");
    println!("   ✓ Max depth prevents infinite delegation loops");
    println!();

    println!("🔧 Handoff Strategies:\n");
    println!("   • Automatic: Agent decides when to delegate");
    println!("   • Manual: Explicit delegation with specialist name");
    println!("   • Hybrid: Combine automatic detection + manual overrides");
    println!();

    println!("📋 Configuration:\n");
    println!("   YAML:");
    println!("   ```yaml");
    println!("   autonomous:");
    println!("     handoffs:");
    println!("       enabled: true");
    println!("       strategy: automatic");
    println!("       max_depth: 3");
    println!("   ```");
    println!();

    println!("   Builder API:");
    println!("   ```rust");
    println!("   .with_config(PaladinConfig::builder()");
    println!("       .autonomous(AutonomousConfig {{");
    println!("           handoffs: HandoffConfig {{");
    println!("               enabled: true,");
    println!("               strategy: HandoffStrategy::Automatic,");
    println!("               max_depth: 3,");
    println!("           }},");
    println!("           ..Default::default()");
    println!("       }})");
    println!("       .build()?");
    println!("   ```");
    println!("   Note: Specialist pool managed by HandoffService");
    println!();

    println!("📚 Learn More:");
    println!("   • See docs/AUTONOMOUS.md §4 for detailed handoff documentation");
    println!("   • Combine with planning for complex multi-agent workflows");
    println!("   • Use specialist pools to control delegation scope");

    Ok(())
}
