//! Grove Routing Example
//!
//! Demonstrates the Grove pattern for routing tasks to specialized agent trees
//! based on keyword matching.
//!
//! This example showcases:
//! - Creating a Grove with multiple expert trees
//! - Keyword-based routing strategy
//! - Specialized agents with specific expertise
//! - Routing decision visibility and confidence scoring
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example grove_routing
//! ```

use paladin::core::platform::container::battalion::grove::{
    GroveBuilder, GroveConfig, RoutingStrategy, Tree, TreeAgent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                Grove Routing: Task Distribution                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Create Security Experts Tree
    println!("🔐 Creating Security Experts Tree...");
    let security_tree = Tree::new("Security Experts")
        .add_agent(TreeAgent::new("SecurityAuditor").with_keywords(vec![
            "security",
            "vulnerability",
            "authentication",
            "encryption",
            "penetration",
            "audit",
        ]))
        .add_agent(TreeAgent::new("CryptoExpert").with_keywords(vec![
            "cryptography",
            "encryption",
            "keys",
            "certificates",
            "tls",
            "ssl",
        ]))
        .add_agent(
            TreeAgent::new("AccessControlSpecialist").with_keywords(vec![
                "authorization",
                "permissions",
                "rbac",
                "access",
                "identity",
            ]),
        );

    println!("   ✓ Added 3 security specialists");
    println!();

    // Create Performance Experts Tree
    println!("⚡ Creating Performance Experts Tree...");
    let performance_tree = Tree::new("Performance Experts")
        .add_agent(TreeAgent::new("DatabaseOptimizer").with_keywords(vec![
            "database",
            "query",
            "index",
            "sql",
            "performance",
            "optimization",
        ]))
        .add_agent(TreeAgent::new("CachingExpert").with_keywords(vec![
            "cache",
            "redis",
            "memcached",
            "caching",
            "latency",
        ]))
        .add_agent(
            TreeAgent::new("LoadBalancingSpecialist").with_keywords(vec![
                "scaling",
                "load",
                "throughput",
                "capacity",
                "horizontal",
            ]),
        );

    println!("   ✓ Added 3 performance specialists");
    println!();

    // Configure Grove with KeywordMatch routing
    let config = GroveConfig {
        routing_strategy: RoutingStrategy::KeywordMatch,
        routing_model: None,
        fallback_tree: Some("Security Experts".to_string()),
        similarity_threshold: 0.7,
        routing_fallback: "keyword".to_string(),
        min_confidence: 0.5,
    };

    // Build the Grove
    let grove = GroveBuilder::new()
        .name("Expert Routing Grove")
        .add_tree(security_tree)
        .add_tree(performance_tree)
        .config(config)
        .build()?;

    println!("🌳 Grove Configuration:");
    println!("   • Name: {}", grove.data.name);
    println!("   • Trees: {}", grove.data.trees.len());
    println!(
        "   • Routing Strategy: {:?}",
        grove.data.config.routing_strategy
    );
    println!("   • Fallback Tree: {:?}", grove.data.config.fallback_tree);
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Example Task 1: Security Review
    println!("📋 Task 1: Security Review");
    println!("   Input: \"Review authentication implementation for vulnerabilities\"");
    println!();
    println!("   🎯 Expected Routing:");
    println!("      • Tree: Security Experts");
    println!("      • Agent: SecurityAuditor");
    println!("      • Reason: Keywords 'authentication' and 'vulnerabilities' match");
    println!("      • Confidence: High");
    println!();

    // Example Task 2: Performance Optimization
    println!("📋 Task 2: Performance Optimization");
    println!("   Input: \"Optimize database query performance and add proper indexes\"");
    println!();
    println!("   🎯 Expected Routing:");
    println!("      • Tree: Performance Experts");
    println!("      • Agent: DatabaseOptimizer");
    println!("      • Reason: Keywords 'database', 'query', 'performance', 'indexes' match");
    println!("      • Confidence: High");
    println!();

    // Example Task 3: Encryption Implementation
    println!("📋 Task 3: Encryption Implementation");
    println!("   Input: \"Implement TLS encryption with proper certificate management\"");
    println!();
    println!("   🎯 Expected Routing:");
    println!("      • Tree: Security Experts");
    println!("      • Agent: CryptoExpert");
    println!("      • Reason: Keywords 'encryption', 'tls', 'certificates' match");
    println!("      • Confidence: High");
    println!();

    // Example Task 4: Caching Strategy
    println!("📋 Task 4: Caching Strategy");
    println!("   Input: \"Design Redis caching strategy to reduce latency\"");
    println!();
    println!("   🎯 Expected Routing:");
    println!("      • Tree: Performance Experts");
    println!("      • Agent: CachingExpert");
    println!("      • Reason: Keywords 'redis', 'caching', 'latency' match");
    println!("      • Confidence: High");
    println!();

    // Example Task 5: Ambiguous (Fallback)
    println!("📋 Task 5: Ambiguous Task (Fallback Test)");
    println!("   Input: \"Review the overall system architecture\"");
    println!();
    println!("   🎯 Expected Routing:");
    println!("      • Tree: Security Experts (Fallback)");
    println!("      • Agent: First available");
    println!("      • Reason: No strong keyword matches, using fallback tree");
    println!("      • Confidence: Low");
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    println!("📊 Grove Statistics:");
    println!("   • Total Trees: {}", grove.data.trees.len());

    let total_agents: usize = grove.data.trees.iter().map(|tree| tree.agents.len()).sum();
    println!("   • Total Agents: {}", total_agents);

    let total_keywords: usize = grove
        .data
        .trees
        .iter()
        .flat_map(|tree| tree.agents.iter())
        .map(|agent| agent.expertise_keywords.len())
        .sum();
    println!("   • Total Keywords: {}", total_keywords);

    println!();
    println!("✅ Grove routing example completed successfully!");
    println!();
    println!("💡 Key Takeaways:");
    println!("   • Grove pattern enables intelligent task routing to specialists");
    println!("   • KeywordMatch provides fast, deterministic routing");
    println!("   • Fallback trees ensure tasks are never left unrouted");
    println!("   • Multiple agents per tree enable fine-grained specialization");
    println!();
    println!("🔄 Alternative Routing Strategies:");
    println!("   • SemanticSimilarity: Uses embeddings for contextual matching");
    println!("   • LlmRouting: Let an LLM analyze and route tasks intelligently");
    println!();
    println!("📚 For production use:");
    println!("   • Integrate with GroveExecutionService for actual routing");
    println!("   • Add embedding service for SemanticSimilarity routing");
    println!("   • Configure LLM adapter for LlmRouting strategy");

    Ok(())
}
