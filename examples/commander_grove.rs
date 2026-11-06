//! Commander with Grove Strategy Example
//!
//! Demonstrates using Commander to orchestrate Grove routing with all three
//! routing strategies: KeywordMatch, SemanticSimilarity, and LlmRouting.
//!
//! This example showcases:
//! - Commander automatically selecting Grove strategy
//! - KeywordMatch routing (fast, deterministic)
//! - SemanticSimilarity routing (contextual, embedding-based)
//! - LlmRouting (intelligent, LLM-powered)
//! - Fallback behavior and confidence scoring
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example commander_grove
//! ```

use paladin::core::platform::container::battalion::grove::{
    GroveBuilder, GroveConfig, RoutingStrategy, Tree, TreeAgent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          Commander with Grove Strategy Examples                ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    println!("This example demonstrates the Commander pattern orchestrating");
    println!("Grove routing with different routing strategies.");
    println!();
    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Create common expert trees
    let backend_tree = Tree::new("Backend Specialists")
        .add_agent(
            TreeAgent::new("DatabaseExpert")
                .with_keywords(vec!["database", "sql", "query", "index", "schema"]),
        )
        .add_agent(
            TreeAgent::new("ApiExpert")
                .with_keywords(vec!["api", "rest", "graphql", "endpoint", "route"]),
        )
        .add_agent(TreeAgent::new("MicroservicesExpert").with_keywords(vec![
            "microservices",
            "service",
            "distributed",
            "rpc",
        ]));

    let frontend_tree = Tree::new("Frontend Specialists")
        .add_agent(TreeAgent::new("ReactExpert").with_keywords(vec![
            "react",
            "jsx",
            "hooks",
            "component",
            "state",
        ]))
        .add_agent(TreeAgent::new("CssExpert").with_keywords(vec![
            "css",
            "styling",
            "layout",
            "responsive",
            "design",
        ]))
        .add_agent(TreeAgent::new("PerformanceExpert").with_keywords(vec![
            "performance",
            "optimization",
            "bundle",
            "lazy",
        ]));

    let devops_tree = Tree::new("DevOps Specialists")
        .add_agent(TreeAgent::new("KubernetesExpert").with_keywords(vec![
            "kubernetes",
            "k8s",
            "pod",
            "deployment",
            "cluster",
        ]))
        .add_agent(TreeAgent::new("CicdExpert").with_keywords(vec![
            "ci",
            "cd",
            "pipeline",
            "deployment",
            "automation",
        ]))
        .add_agent(TreeAgent::new("MonitoringExpert").with_keywords(vec![
            "monitoring",
            "observability",
            "metrics",
            "logs",
        ]));

    println!("🌳 Created 3 Expert Trees:");
    println!("   • Backend Specialists (3 agents)");
    println!("   • Frontend Specialists (3 agents)");
    println!("   • DevOps Specialists (3 agents)");
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Example 1: KeywordMatch Routing
    println!("📋 Example 1: KeywordMatch Routing Strategy");
    println!();
    println!("Configuration:");
    println!("   • Strategy: Grove (automatic detection from 'route to' keyword)");
    println!("   • Routing: KeywordMatch");
    println!("   • Trees: 3");
    println!("   • Fallback: Backend Specialists");
    println!();

    let _grove1 = GroveBuilder::new()
        .name("KeywordMatch Grove")
        .add_tree(backend_tree.clone())
        .add_tree(frontend_tree.clone())
        .add_tree(devops_tree.clone())
        .config(GroveConfig {
            routing_strategy: RoutingStrategy::KeywordMatch,
            routing_model: None,
            fallback_tree: Some("Backend Specialists".to_string()),
            similarity_threshold: 0.7,
            routing_fallback: "keyword".to_string(),
            min_confidence: 0.5,
        })
        .build()?;

    println!("   🎯 Example Tasks:");
    println!();
    println!("   Task 1: \"Optimize database query performance with proper indexing\"");
    println!("      → Route: Backend Specialists → DatabaseExpert");
    println!("      → Matches: database, query, index (3 keywords)");
    println!("      → Confidence: High (95%)");
    println!();
    println!("   Task 2: \"Fix React component state management issues\"");
    println!("      → Route: Frontend Specialists → ReactExpert");
    println!("      → Matches: react, component, state (3 keywords)");
    println!("      → Confidence: High (90%)");
    println!();
    println!("   Task 3: \"Set up Kubernetes deployment pipeline\"");
    println!("      → Route: DevOps Specialists → KubernetesExpert");
    println!("      → Matches: kubernetes, deployment, pipeline (3 keywords)");
    println!("      → Confidence: High (92%)");
    println!();
    println!("   Advantages:");
    println!("      • Fast: O(n) keyword matching");
    println!("      • Deterministic: Same keywords → same route");
    println!("      • Transparent: Clear matching logic");
    println!("      • No external dependencies");
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Example 2: SemanticSimilarity Routing
    println!("📋 Example 2: SemanticSimilarity Routing Strategy");
    println!();
    println!("Configuration:");
    println!("   • Strategy: Grove");
    println!("   • Routing: SemanticSimilarity");
    println!("   • Embedding Model: text-embedding-ada-002 (OpenAI)");
    println!("   • Similarity Threshold: 0.75");
    println!("   • Fallback: Backend Specialists");
    println!();

    let _grove2 = GroveBuilder::new()
        .name("Semantic Grove")
        .add_tree(backend_tree.clone())
        .add_tree(frontend_tree.clone())
        .add_tree(devops_tree.clone())
        .config(GroveConfig {
            routing_strategy: RoutingStrategy::SemanticSimilarity,
            routing_model: None,
            fallback_tree: Some("Backend Specialists".to_string()),
            similarity_threshold: 0.75,
            routing_fallback: "keyword".to_string(),
            min_confidence: 0.5,
        })
        .build()?;

    println!("   🎯 Example Tasks:");
    println!();
    println!("   Task 1: \"Our users are experiencing slow page loads\"");
    println!("      → Route: Frontend Specialists → PerformanceExpert");
    println!("      → Semantic match: 'slow page loads' → performance optimization");
    println!("      → Similarity: 0.87");
    println!("      → Confidence: High (87%)");
    println!();
    println!("   Task 2: \"How do we scale our system to handle 10x traffic?\"");
    println!("      → Route: DevOps Specialists → KubernetesExpert");
    println!("      → Semantic match: 'scale system' → deployment/cluster");
    println!("      → Similarity: 0.82");
    println!("      → Confidence: High (82%)");
    println!();
    println!("   Task 3: \"Need to store user preferences efficiently\"");
    println!("      → Route: Backend Specialists → DatabaseExpert");
    println!("      → Semantic match: 'store data' → database/schema");
    println!("      → Similarity: 0.79");
    println!("      → Confidence: High (79%)");
    println!();
    println!("   Advantages:");
    println!("      • Contextual: Understands meaning, not just keywords");
    println!("      • Flexible: Works with paraphrased queries");
    println!("      • Robust: Handles synonyms and related concepts");
    println!("      • Quality: Better matches for ambiguous tasks");
    println!();
    println!("   Requirements:");
    println!("      • Embedding service (OpenAI, local model)");
    println!("      • Pre-computed agent embeddings");
    println!("      • Slightly higher latency (~50-100ms)");
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    // Example 3: LlmRouting
    println!("📋 Example 3: LlmRouting Strategy");
    println!();
    println!("Configuration:");
    println!("   • Strategy: Grove");
    println!("   • Routing: LlmRouting");
    println!("   • LLM Model: gpt-4o-mini");
    println!("   • Temperature: 0.3 (more deterministic)");
    println!("   • Fallback: Backend Specialists");
    println!();

    let _grove3 = GroveBuilder::new()
        .name("LLM-Routed Grove")
        .add_tree(backend_tree.clone())
        .add_tree(frontend_tree.clone())
        .add_tree(devops_tree.clone())
        .config(GroveConfig {
            routing_strategy: RoutingStrategy::LlmRouting,
            // Required under LlmRouting (CLOSE-01/D-01/D-02): routing hard-errors if this is
            // absent. Matches the "LLM Model: gpt-4o-mini" configuration printed above.
            routing_model: Some("gpt-4o-mini".to_string()),
            fallback_tree: Some("Backend Specialists".to_string()),
            similarity_threshold: 0.7,
            routing_fallback: "keyword".to_string(),
            min_confidence: 0.7,
        })
        .build()?;

    println!("   🎯 Example Tasks:");
    println!();
    println!("   Task 1: \"Users report seeing old data after updates\"");
    println!("      → LLM Analysis:");
    println!("         - Identifies caching/state management issue");
    println!("         - Considers both frontend state and backend cache");
    println!("         - Chooses based on context clues");
    println!("      → Route: Frontend Specialists → ReactExpert");
    println!("      → Reasoning: 'seeing old data' suggests UI state problem");
    println!("      → Confidence: High (88%)");
    println!();
    println!("   Task 2: \"Need to reduce API response times by 50%\"");
    println!("      → LLM Analysis:");
    println!("         - Multi-faceted problem: DB queries, API design, caching");
    println!("         - Primary bottleneck likely in backend");
    println!("         - Could involve multiple specialists");
    println!("      → Route: Backend Specialists → DatabaseExpert");
    println!("      → Reasoning: API performance often database-bound");
    println!("      → Confidence: Medium (72%)");
    println!();
    println!("   Task 3: \"Deploy new version without downtime\"");
    println!("      → LLM Analysis:");
    println!("         - Clearly a deployment/infrastructure problem");
    println!("         - Requires knowledge of rolling updates");
    println!("         - DevOps domain");
    println!("      → Route: DevOps Specialists → KubernetesExpert");
    println!("      → Reasoning: Zero-downtime deployment is k8s specialty");
    println!("      → Confidence: Very High (94%)");
    println!();
    println!("   Advantages:");
    println!("      • Intelligent: Deep understanding of task context");
    println!("      • Explainable: Provides reasoning for routing decision");
    println!("      • Adaptive: Handles novel or complex scenarios");
    println!("      • Multi-factor: Considers multiple aspects simultaneously");
    println!();
    println!("   Requirements:");
    println!("      • LLM service (OpenAI, Anthropic, DeepSeek)");
    println!("      • Higher latency (~200-500ms)");
    println!("      • Higher cost per routing decision");
    println!("      • Requires well-structured prompts");
    println!();

    println!("────────────────────────────────────────────────────────────────");
    println!();

    println!("📊 Routing Strategy Comparison:");
    println!();
    println!("╔════════════════════╤════════════╤═══════════╤════════════╤═══════════╗");
    println!("║ Strategy           │ Latency    │ Cost      │ Accuracy   │ Use Case  ║");
    println!("╠════════════════════╪════════════╪═══════════╪════════════╪═══════════╣");
    println!("║ KeywordMatch       │ <10ms      │ Free      │ Good       │ Clear     ║");
    println!("║                    │            │           │            │ keywords  ║");
    println!("╠════════════════════╪════════════╪═══════════╪════════════╪═══════════╣");
    println!("║ SemanticSimilarity │ 50-100ms   │ Low       │ Better     │ Contextual║");
    println!("║                    │            │ (~$0.0001)│            │ matching  ║");
    println!("╠════════════════════╪════════════╪═══════════╪════════════╪═══════════╣");
    println!("║ LlmRouting         │ 200-500ms  │ Medium    │ Best       │ Complex   ║");
    println!("║                    │            │ (~$0.001) │            │ scenarios ║");
    println!("╚════════════════════╧════════════╧═══════════╧════════════╧═══════════╝");
    println!();

    println!("✅ Commander Grove examples completed!");
    println!();
    println!("💡 Key Takeaways:");
    println!("   • Commander can automatically detect Grove strategy");
    println!("   • Choose routing strategy based on requirements:");
    println!("     - KeywordMatch: Speed and determinism");
    println!("     - SemanticSimilarity: Balance of speed and accuracy");
    println!("     - LlmRouting: Maximum intelligence and flexibility");
    println!("   • Always configure fallback tree for unmatched tasks");
    println!("   • Similarity threshold controls routing confidence");
    println!();
    println!("🔧 Production Recommendations:");
    println!("   • Start with KeywordMatch for well-defined domains");
    println!("   • Upgrade to SemanticSimilarity for better UX");
    println!("   • Use LlmRouting for complex or critical decisions");
    println!("   • Monitor routing accuracy and adjust thresholds");
    println!("   • Cache embeddings to improve performance");

    Ok(())
}
