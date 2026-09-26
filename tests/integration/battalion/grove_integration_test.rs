//! Integration tests for Grove pattern
//!
//! Tests end-to-end Grove execution with intelligent agent routing

use async_trait::async_trait;
use paladin::application::services::battalion::grove_service::GroveExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::grove::{
    GroveBuilder, RoutingStrategy, Tree, TreeAgent,
};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_battalion::in_memory_registry::HashMapPaladinRegistry;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream, StopReason};
use paladin_ports::output::paladin_registry::PaladinRegistry;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Mock PaladinPort for Grove integration testing
#[derive(Clone)]
struct GroveMockPaladinPort {
    execution_log: Arc<Mutex<Vec<String>>>,
}

impl GroveMockPaladinPort {
    fn new() -> Self {
        Self {
            execution_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    fn get_execution_log(&self) -> Vec<String> {
        self.execution_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl PaladinPort for GroveMockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Log execution
        let log_entry = format!("Agent '{}' handling task", paladin.node.name);
        self.execution_log.lock().unwrap().push(log_entry);

        // Simulate processing delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(PaladinResult {
            output: format!("[{}]: Handled task: {}", paladin.node.name, input),
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
            execution_time_ms: 10,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Helper function to create a specialized Paladin
fn create_specialist_paladin(name: &str, expertise: &str, _keywords: Vec<&str>) -> Paladin {
    let data = PaladinData {
        system_prompt: format!(
            "You are a {} specialist with expertise in {}",
            name, expertise
        ),
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

/// Helper to create TreeAgent
fn create_tree_agent(paladin_id: &str, keywords: Vec<&str>) -> TreeAgent {
    TreeAgent::new(paladin_id).with_keywords(keywords.iter().map(|s| s.to_string()).collect())
}

#[tokio::test]
async fn test_grove_keyword_match_routing() {
    // Task 8.9: Grove with KeywordMatch routing
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    // Create Paladins (using agent_N format to match TreeAgent IDs)
    let paladins = vec![
        create_specialist_paladin("agent_0", "authentication", vec!["auth"]),
        create_specialist_paladin("agent_1", "encryption", vec!["crypto"]),
        create_specialist_paladin("agent_2", "caching", vec!["cache"]),
        create_specialist_paladin("agent_3", "database", vec!["database"]),
    ];

    // Create security experts tree
    let security_tree = Tree::new("Security Experts")
        .add_agent(create_tree_agent(
            "agent_0",
            vec!["auth", "authentication", "login", "oauth"],
        ))
        .add_agent(create_tree_agent(
            "agent_1",
            vec!["encryption", "crypto", "tls", "ssl"],
        ));

    // Create performance experts tree
    let performance_tree = Tree::new("Performance Experts")
        .add_agent(create_tree_agent(
            "agent_2",
            vec!["cache", "redis", "memcached"],
        ))
        .add_agent(create_tree_agent(
            "agent_3",
            vec!["database", "query", "index", "sql"],
        ));

    let grove = GroveBuilder::new()
        .name("KeywordGrove")
        .add_tree(security_tree)
        .add_tree(performance_tree)
        .routing_strategy(RoutingStrategy::KeywordMatch)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port.clone(), None, None, Arc::new(registry));

    // Test routing to security tree
    let security_task = "Review authentication implementation for vulnerabilities";
    let result = service
        .execute(&grove, security_task)
        .await
        .expect("Security task should succeed");

    assert!(!result.execution_result.is_empty());
    assert!(
        result.routing_decision.selected_agent == "agent_0"
            || result.routing_decision.selected_agent == "agent_1",
        "Should route to security expert"
    );

    // Test routing to performance tree
    let performance_task = "Optimize database queries for better performance";
    let result2 = service
        .execute(&grove, performance_task)
        .await
        .expect("Performance task should succeed");

    assert!(
        result2.routing_decision.selected_agent == "agent_2"
            || result2.routing_decision.selected_agent == "agent_3",
        "Should route to performance expert"
    );
}

#[tokio::test]
async fn test_grove_semantic_similarity_routing() {
    // Task 8.10: Grove with SemanticSimilarity routing (mock embeddings)
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    // Create Paladins using agent_N format
    let paladins = vec![
        create_specialist_paladin("agent_0", "React", vec!["react"]),
        create_specialist_paladin("agent_1", "CSS", vec!["css"]),
        create_specialist_paladin("agent_2", "REST APIs", vec!["api"]),
        create_specialist_paladin("agent_3", "Databases", vec!["database"]),
    ];

    // Create trees
    let frontend_tree = Tree::new("Frontend Experts")
        .add_agent(create_tree_agent(
            "agent_0",
            vec!["react", "jsx", "frontend", "ui"],
        ))
        .add_agent(create_tree_agent("agent_1", vec!["css", "styling", "ui"]));

    let backend_tree = Tree::new("Backend Experts")
        .add_agent(create_tree_agent("agent_2", vec!["api", "rest", "backend"]))
        .add_agent(create_tree_agent(
            "agent_3",
            vec!["database", "sql", "backend"],
        ));

    let grove = GroveBuilder::new()
        .name("SemanticGrove")
        .add_tree(frontend_tree)
        .add_tree(backend_tree)
        .routing_strategy(RoutingStrategy::SemanticSimilarity)
        .similarity_threshold(0.5)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port.clone(), None, None, Arc::new(registry));

    // Note: In a real test with embeddings, we'd use an actual embedding service
    // For this integration test, we're just verifying the routing mechanism works
    let task = "Design a user interface for the login page";
    let result = service.execute(&grove, task).await;

    // Should succeed (will use keyword fallback if embeddings not available)
    assert!(result.is_ok(), "Execution should succeed");
}

#[tokio::test]
async fn test_grove_llm_routing() {
    // Task 8.11: Grove with LlmRouting
    // Note: This would require a mock LLM that returns routing decisions
    // For now, we test that the routing strategy can be configured

    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    let paladins = vec![
        create_specialist_paladin("agent_0", "features", vec!["feature"]),
        create_specialist_paladin("agent_1", "bugs", vec!["bug"]),
    ];

    let tree1 =
        Tree::new("Team A").add_agent(create_tree_agent("agent_0", vec!["feature", "development"]));

    let tree2 = Tree::new("Team B").add_agent(create_tree_agent("agent_1", vec!["bug", "fix"]));

    let grove = GroveBuilder::new()
        .name("LlmRoutingGrove")
        .add_tree(tree1)
        .add_tree(tree2)
        .routing_strategy(RoutingStrategy::LlmRouting)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port, None, None, Arc::new(registry));

    // A Grove selecting LLM routing without a routing model is a configuration error, not a
    // thing to guess at: it surfaces from `execute()` with no fallback of any kind (D-02). This
    // test is the direct inversion of the counter-example `06-VERIFICATION.md` cited — it used
    // to assert `result.is_ok()` for exactly this misconfiguration.
    let result = service.execute(&grove, "Fix the login bug").await;

    match result {
        Err(paladin::core::platform::container::battalion::BattalionError::RoutingError(msg)) => {
            assert!(
                msg.contains("routing_model"),
                "error message should name routing_model, got: {}",
                msg
            );
        }
        other => panic!(
            "Expected RoutingError naming routing_model, got {:?}",
            other
        ),
    }
}

/// Recording mock `LlmPort` used to prove that a Grove misconfigured for `LlmRouting` (no
/// `routing_model`) never reaches the LLM through `GroveExecutionService::execute()` — the
/// public entry point. A green result on a test using this mock while recording zero calls
/// means the D-02 no-fallback guarantee held; any recorded call is a direct proof it did not.
struct RecordingRoutingLlmMock {
    recorded_models: Arc<Mutex<Vec<String>>>,
}

impl RecordingRoutingLlmMock {
    fn new() -> Self {
        Self {
            recorded_models: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn recorded_models(&self) -> Vec<String> {
        self.recorded_models.lock().unwrap().clone()
    }
}

#[async_trait]
impl paladin_ports::output::llm_port::LlmPort for RecordingRoutingLlmMock {
    async fn generate(
        &self,
        request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        paladin_ports::output::llm_port::LlmResponse,
        paladin_ports::output::llm_port::LlmError,
    > {
        self.recorded_models
            .lock()
            .unwrap()
            .push(request.model.clone());

        let response_json = r#"{
            "tree_name": "Team A",
            "agent_id": "agent_0",
            "confidence": 0.9,
            "reasoning": "Recording mock always selects agent_0"
        }"#;

        Ok(paladin_ports::output::llm_port::LlmResponse {
            id: uuid::Uuid::new_v4(),
            request_id: request.id,
            model: request.model,
            content: response_json.to_string(),
            finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 50),
            cost: None,
            created_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
            function_call: None,
        })
    }

    async fn generate_stream(
        &self,
        _request: paladin_ports::output::llm_port::LlmRequest,
    ) -> Result<
        Box<
            dyn futures::Stream<
                    Item = Result<
                        paladin_ports::output::llm_port::StreamingResponse,
                        paladin_ports::output::llm_port::LlmError,
                    >,
                > + Send,
        >,
        paladin_ports::output::llm_port::LlmError,
    > {
        unimplemented!("Mock not needed for these tests")
    }

    async fn validate_model(
        &self,
        _model: &str,
    ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
        Ok(true)
    }

    async fn get_available_models(
        &self,
    ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn get_provider_name(&self) -> &'static str {
        "recording-routing-mock"
    }

    fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
        paladin_ports::output::llm_port::ProviderCapabilities::default()
    }
}

#[tokio::test]
async fn test_grove_llm_routing_errors_when_routing_model_absent_through_execute() {
    // 06-VERIFICATION.md missing item (b): a Grove built for LlmRouting, with two trees and
    // registered agents, a **configured** llm_port, and no routing_model, driven through
    // GroveExecutionService::execute() — the only public entry point — must return
    // Err(BattalionError::RoutingError(..)) naming routing_model, with zero LLM calls made.
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    let paladins = vec![
        create_specialist_paladin("agent_0", "features", vec!["feature"]),
        create_specialist_paladin("agent_1", "bugs", vec!["bug"]),
    ];

    let tree1 =
        Tree::new("Team A").add_agent(create_tree_agent("agent_0", vec!["feature", "development"]));
    let tree2 = Tree::new("Team B").add_agent(create_tree_agent("agent_1", vec!["bug", "fix"]));

    let grove = GroveBuilder::new()
        .name("LlmRoutingGroveNoModel")
        .add_tree(tree1)
        .add_tree(tree2)
        .routing_strategy(RoutingStrategy::LlmRouting)
        // no routing_model set
        .build()
        .expect("Grove build should succeed");

    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let llm_mock = Arc::new(RecordingRoutingLlmMock::new());
    let service = GroveExecutionService::new(
        paladin_port,
        None,
        Some(llm_mock.clone()),
        Arc::new(registry),
    );

    let result = service.execute(&grove, "Fix the login bug").await;

    match result {
        Err(paladin::core::platform::container::battalion::BattalionError::RoutingError(msg)) => {
            assert!(
                msg.contains("routing_model"),
                "error message should name routing_model, got: {}",
                msg
            );
        }
        other => panic!(
            "Expected RoutingError naming routing_model, got {:?}",
            other
        ),
    }
    assert!(
        llm_mock.recorded_models().is_empty(),
        "no LLM call should have been made for a misconfigured Grove"
    );
}

/// Scope negative control (planner-recorded assumption, 06-08-PLAN.md
/// `<assumption_delta_decision>`): the D-02 hard error is scoped strictly to the missing-
/// `routing_model` configuration case. An absent `llm_port` under LLM routing is a *different*
/// failure mode and deliberately keeps its pre-existing fallback behaviour through `execute()`
/// — no recorded decision covers breaking it, and generalizing the hard error to every
/// LLM-routing configuration error was explicitly not taken.
#[tokio::test]
async fn test_grove_llm_routing_falls_back_when_llm_port_absent_but_routing_model_set() {
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    let paladins = vec![
        create_specialist_paladin("agent_0", "features", vec!["feature"]),
        create_specialist_paladin("agent_1", "bugs", vec!["bug"]),
    ];

    let tree1 =
        Tree::new("Team A").add_agent(create_tree_agent("agent_0", vec!["feature", "development"]));
    let tree2 = Tree::new("Team B").add_agent(create_tree_agent("agent_1", vec!["bug", "fix"]));

    let grove = GroveBuilder::new()
        .name("LlmRoutingGroveNoPort")
        .add_tree(tree1)
        .add_tree(tree2)
        .routing_strategy(RoutingStrategy::LlmRouting)
        .routing_model("deepseek-chat")
        .build()
        .expect("Grove build should succeed");

    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    // llm_port passed as None: this is a different, pre-existing failure mode from the
    // missing-routing_model configuration error, and it deliberately keeps its fallback.
    let service = GroveExecutionService::new(paladin_port, None, None, Arc::new(registry));

    let result = service.execute(&grove, "Fix the login bug").await;

    assert!(
        result.is_ok(),
        "an absent llm_port under LlmRouting with a configured routing_model must still fall \
         back successfully through execute() — this is out of D-02's scope"
    );
}

#[tokio::test]
async fn test_grove_fallback_behavior() {
    // Task 8.12: Grove fallback behavior when no match
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    let paladins = vec![
        create_specialist_paladin("agent_0", "general", vec!["general"]),
        create_specialist_paladin("agent_1", "general", vec!["general"]),
        create_specialist_paladin("agent_2", "Rust", vec!["rust"]),
    ];

    let fallback_tree = Tree::new("Generalists")
        .add_agent(create_tree_agent("agent_0", vec!["general"]))
        .add_agent(create_tree_agent("agent_1", vec!["general"]));

    let specialist_tree =
        Tree::new("Specialists").add_agent(create_tree_agent("agent_2", vec!["rust", "cargo"]));

    let grove = GroveBuilder::new()
        .name("FallbackGrove")
        .add_tree(fallback_tree)
        .add_tree(specialist_tree)
        .routing_strategy(RoutingStrategy::KeywordMatch)
        .fallback_tree("Generalists")
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port.clone(), None, None, Arc::new(registry));

    // Task that doesn't match any specialist keywords
    let task = "What's the weather like today?";
    let result = service
        .execute(&grove, task)
        .await
        .expect("Should succeed using fallback");

    assert!(
        result.routing_decision.selected_agent == "agent_0"
            || result.routing_decision.selected_agent == "agent_1",
        "Should route to fallback tree"
    );
    assert_eq!(result.routing_decision.selected_tree, "Generalists");
}

#[tokio::test]
async fn test_grove_no_fallback_default_behavior() {
    // Test default fallback when no fallback_tree configured
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    let paladins = vec![create_specialist_paladin(
        "agent_0",
        "specific domain",
        vec!["specific"],
    )];

    let tree1 = Tree::new("Specialists").add_agent(create_tree_agent("agent_0", vec!["specific"]));

    let grove = GroveBuilder::new()
        .name("NoFallbackGrove")
        .add_tree(tree1)
        .routing_strategy(RoutingStrategy::KeywordMatch)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port, None, None, Arc::new(registry));

    // Task that doesn't match keywords
    let task = "Completely unrelated task";
    let result = service.execute(&grove, task).await;

    // Should still succeed by routing to first tree as default
    assert!(result.is_ok(), "Should succeed with default fallback");
}

#[tokio::test]
async fn test_grove_multiple_trees() {
    // Task 8.13: Grove with multiple trees
    let paladin_port = Arc::new(GroveMockPaladinPort::new());

    // Create Paladins
    let paladins = vec![
        create_specialist_paladin("agent_0", "security", vec!["security"]),
        create_specialist_paladin("agent_1", "performance", vec!["performance"]),
        create_specialist_paladin("agent_2", "data", vec!["data"]),
        create_specialist_paladin("agent_3", "infrastructure", vec!["infra"]),
    ];

    // Create 4 different expert trees
    let security_tree = Tree::new("Security").add_agent(create_tree_agent(
        "agent_0",
        vec!["security", "vulnerability"],
    ));

    let performance_tree = Tree::new("Performance").add_agent(create_tree_agent(
        "agent_1",
        vec!["performance", "optimization"],
    ));

    let data_tree =
        Tree::new("Data").add_agent(create_tree_agent("agent_2", vec!["data", "analytics"]));

    let infrastructure_tree = Tree::new("Infrastructure").add_agent(create_tree_agent(
        "agent_3",
        vec!["infrastructure", "deployment", "kubernetes"],
    ));

    let grove = GroveBuilder::new()
        .name("MultiTreeGrove")
        .add_tree(security_tree)
        .add_tree(performance_tree)
        .add_tree(data_tree)
        .add_tree(infrastructure_tree)
        .routing_strategy(RoutingStrategy::KeywordMatch)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port.clone(), None, None, Arc::new(registry));

    // Test routing to each tree
    let tasks = vec![
        ("Fix security vulnerability in auth", "agent_0"),
        ("Optimize query performance", "agent_1"),
        ("Analyze user data trends", "agent_2"),
        ("Deploy to Kubernetes cluster", "agent_3"),
    ];

    for (task, expected_expert) in tasks {
        let result = service
            .execute(&grove, task)
            .await
            .expect("Task should succeed");

        assert_eq!(
            result.routing_decision.selected_agent, expected_expert,
            "Task '{}' should route to {}",
            task, expected_expert
        );
    }
}

#[tokio::test]
async fn test_grove_error_handling() {
    // Test error handling when agent fails
    struct FailingMockPort;

    #[async_trait]
    impl PaladinPort for FailingMockPort {
        async fn execute(
            &self,
            paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            if paladin.node.name == "agent_1" {
                return Err(PaladinError::ExecutionError(
                    "Simulated failure".to_string(),
                ));
            }

            Ok(PaladinResult {
                output: format!("[{}]: Success", paladin.node.name),
                usage: paladin_ports::output::llm_port::TokenUsage::new(50, 0),
                execution_time_ms: 10,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                ..Default::default()
            })
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    let paladin_port = Arc::new(FailingMockPort);

    let paladins = vec![
        create_specialist_paladin("agent_0", "good", vec!["good"]),
        create_specialist_paladin("agent_1", "failing", vec!["failing"]),
    ];

    let tree = Tree::new("Team")
        .add_agent(create_tree_agent("agent_0", vec!["good", "team"]))
        .add_agent(create_tree_agent("agent_1", vec!["failing", "team"]));

    let grove = GroveBuilder::new()
        .name("ErrorTestGrove")
        .add_tree(tree)
        .routing_strategy(RoutingStrategy::KeywordMatch)
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service = GroveExecutionService::new(paladin_port, None, None, Arc::new(registry));

    // Task with "failing" keyword should route to agent_1 which will fail
    let result = service.execute(&grove, "failing team task").await;

    // Should return an error since agent_1 fails
    assert!(result.is_err(), "Should propagate execution error");
}

#[tokio::test]
async fn test_grove_llm_routing_end_to_end() {
    use paladin_ports::output::llm_port::{
        FinishReason, LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, TokenUsage,
    };
    use uuid::Uuid;

    /// Mock LLM port that returns routing decisions
    struct RoutingLlmMock;

    #[async_trait]
    impl LlmPort for RoutingLlmMock {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            // Extract the prompt to determine which agent to route to
            let prompt_text = match &request.prompt.node.node.prompt_type {
                paladin::core::platform::container::prompt::PromptType::User(user_prompt) => {
                    &user_prompt.query
                }
                _ => "",
            };

            // Extract just the task line from the prompt
            let task_line = prompt_text
                .lines()
                .find(|line| line.starts_with("Task:"))
                .unwrap_or("");

            // Route based on task keywords
            let response_json = if task_line.to_lowercase().contains("rust")
                || task_line.to_lowercase().contains("backend")
            {
                r#"{
                    "tree_name": "engineering",
                    "agent_id": "backend_expert",
                    "confidence": 0.95,
                    "reasoning": "Task mentions rust and backend development, backend_expert has strong expertise in these areas"
                }"#
            } else if task_line.to_lowercase().contains("react")
                || task_line.to_lowercase().contains("dashboard")
                || task_line.to_lowercase().contains("ui")
            {
                r#"{
                    "tree_name": "engineering",
                    "agent_id": "frontend_expert",
                    "confidence": 0.90,
                    "reasoning": "Task is about react and UI, frontend_expert is the best match"
                }"#
            } else {
                r#"{
                    "tree_name": "engineering",
                    "agent_id": "backend_expert",
                    "confidence": 0.60,
                    "reasoning": "Default routing to backend_expert for unclear tasks"
                }"#
            };

            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: request.model,
                content: response_json.to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage::new(150, 80),
                cost: None,
                created_at: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
                function_call: None,
            })
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<paladin_ports::output::llm_port::StreamingResponse, LlmError>,
                    > + Send,
            >,
            LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec!["gpt-4".to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "mock-routing"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    // Setup
    let paladin_port = Arc::new(GroveMockPaladinPort::new());
    let llm_port = Arc::new(RoutingLlmMock);

    let paladins = vec![
        create_specialist_paladin(
            "backend_expert",
            "Rust backend development",
            vec!["rust", "backend", "api"],
        ),
        create_specialist_paladin(
            "frontend_expert",
            "React frontend development",
            vec!["react", "frontend", "ui"],
        ),
    ];

    let tree = Tree::new("engineering")
        .add_agent(create_tree_agent(
            "backend_expert",
            vec!["rust", "backend", "api"],
        ))
        .add_agent(create_tree_agent(
            "frontend_expert",
            vec!["react", "frontend", "ui"],
        ));

    let grove = GroveBuilder::new()
        .name("LLMRoutingGrove")
        .add_tree(tree)
        .routing_strategy(RoutingStrategy::LlmRouting)
        .routing_model("gpt-4")
        .min_confidence(0.5)
        .routing_fallback("error")
        .build()
        .expect("Grove build should succeed");

    // Create registry and register paladins
    let registry = HashMapPaladinRegistry::new();
    for paladin in &paladins {
        registry
            .register(paladin.node.name.clone(), Arc::new(paladin.clone()))
            .expect("Registry should accept paladin");
    }

    let service =
        GroveExecutionService::new(paladin_port, None, Some(llm_port), Arc::new(registry));

    // Test 1: Backend task should route to backend_expert
    let result = service
        .execute(&grove, "Build a rust backend API service")
        .await;

    assert!(result.is_ok(), "LLM routing should succeed");
    let grove_result = result.unwrap();
    assert_eq!(
        grove_result.routing_decision.selected_agent, "backend_expert",
        "Should route to backend_expert for rust task"
    );
    assert!(
        grove_result.routing_decision.confidence >= 0.9,
        "Should have high confidence for clear task"
    );
    assert!(grove_result.execution_result.contains("backend_expert"));

    // Test 2: Frontend task should route to frontend_expert
    let result = service.execute(&grove, "Create a react dashboard UI").await;

    assert!(result.is_ok(), "LLM routing should succeed");
    let grove_result = result.unwrap();
    assert_eq!(
        grove_result.routing_decision.selected_agent, "frontend_expert",
        "Should route to frontend_expert for react task"
    );
    assert!(
        grove_result.routing_decision.confidence >= 0.9,
        "Should have high confidence for clear task"
    );
    assert!(grove_result.execution_result.contains("frontend_expert"));

    // Test 3: Ambiguous task gets lower confidence but still routes
    let result = service
        .execute(&grove, "Do something with the system")
        .await;

    assert!(
        result.is_ok(),
        "LLM routing should succeed even with low confidence"
    );
    let grove_result = result.unwrap();
    // Should route to backend_expert as default
    assert_eq!(
        grove_result.routing_decision.selected_agent,
        "backend_expert"
    );
    assert!(
        grove_result.routing_decision.confidence >= 0.5,
        "Should meet minimum confidence threshold"
    );
}
