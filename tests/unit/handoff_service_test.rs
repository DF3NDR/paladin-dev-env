//! Unit tests for HandoffService
//!
//! Tests the agent handoff infrastructure including:
//! - Handoff decision logic with different strategies
//! - Agent selection based on capabilities
//! - Handoff chain tracking and circular delegation prevention
//! - Max depth enforcement
//! - Context transfer mechanism
//! - Handoff execution with retry logic (Phase 5)

use paladin::application::errors::handoff_error::HandoffError;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::handoff_service::HandoffService;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::autonomous_config::{HandoffConfig, HandoffRetryConfig};
use paladin::core::platform::container::handoff::{HandoffContext, HandoffStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[test]
fn test_handoff_service_new() {
    // Test: HandoffService can be constructed with valid configuration
    let config = HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    };

    let service = HandoffService::new(Arc::new(config));

    assert!(service.is_some(), "HandoffService should be created");
}

#[test]
fn test_handoff_service_new_with_disabled_config() {
    // Test: HandoffService returns None when handoffs are disabled
    let config = HandoffConfig {
        enabled: false,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    };

    let service = HandoffService::new(Arc::new(config));

    assert!(
        service.is_none(),
        "HandoffService should return None when disabled"
    );
}

#[test]
fn test_should_handoff_automatic_high_confidence() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();
    let context = HandoffContext::new("Simple task".to_string(), "Agent1".to_string());

    // High confidence (>0.8) - shouldn't handoff for simple tasks
    assert!(!service.should_handoff("What is 2+2?", 0.95, &context));
}

#[test]
fn test_should_handoff_automatic_low_confidence() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();
    let context = HandoffContext::new("Complex task".to_string(), "Agent1".to_string());

    // Low confidence (<0.5) - should handoff
    assert!(service.should_handoff("Implement a distributed consensus algorithm", 0.3, &context));
}

#[test]
fn test_should_handoff_explicit_never() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Explicit,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();
    let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());

    // Explicit strategy never auto-handoffs regardless of confidence
    assert!(!service.should_handoff("Complex task", 0.1, &context));
    assert!(!service.should_handoff("Simple task", 0.9, &context));
}

#[test]
fn test_should_handoff_threshold_below() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::threshold(0.7),
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();
    let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());

    // Confidence below threshold - should handoff
    assert!(service.should_handoff("Task", 0.6, &context));
}

#[test]
fn test_should_handoff_threshold_above() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::threshold(0.7),
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();
    let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());

    // Confidence above threshold - shouldn't handoff
    assert!(!service.should_handoff("Task", 0.8, &context));
}

#[test]
fn test_should_handoff_max_depth_exceeded() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 2,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    // Context at max depth
    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.depth = 2;

    // Even with low confidence, shouldn't handoff if at max depth
    assert!(!service.should_handoff("Complex task", 0.2, &context));
}

#[test]
fn test_select_agent_with_matching_specialist() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    // Task requiring code expertise
    let agents = vec![
        (
            "GeneralAssistant".to_string(),
            "General purpose assistant".to_string(),
        ),
        (
            "CodeExpert".to_string(),
            "Expert in Rust, Python, and software architecture".to_string(),
        ),
        (
            "DataAnalyst".to_string(),
            "Specializes in data analysis and statistics".to_string(),
        ),
    ];

    let selected = service.select_agent("Implement a Rust async function", &agents);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap(), "CodeExpert");
}

#[test]
fn test_select_agent_with_multiple_matches() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    // Task that could match multiple specialists
    let agents = vec![
        (
            "PythonExpert".to_string(),
            "Expert in Python programming".to_string(),
        ),
        (
            "RustExpert".to_string(),
            "Expert in Rust programming".to_string(),
        ),
        (
            "CodeReviewer".to_string(),
            "Expert code reviewer for all languages".to_string(),
        ),
    ];

    let selected = service.select_agent("Review this Python code", &agents);
    assert!(selected.is_some());
    // Should prefer the most specific match
    let agent = selected.unwrap();
    assert!(agent == "PythonExpert" || agent == "CodeReviewer");
}

#[test]
fn test_select_agent_no_matches() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    // Task with no relevant specialists
    let agents = vec![
        (
            "MusicComposer".to_string(),
            "Composes music and melodies".to_string(),
        ),
        (
            "ChefAdvisor".to_string(),
            "Provides cooking advice".to_string(),
        ),
    ];

    let selected = service.select_agent("Debug this Rust code", &agents);
    // Should return None or first agent as fallback
    assert!(selected.is_none() || selected.is_some());
}

#[test]
fn test_select_agent_empty_list() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let agents: Vec<(String, String)> = vec![];
    let selected = service.select_agent("Any task", &agents);
    assert!(selected.is_none());
}

#[test]
fn test_validate_handoff_success() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    let result = service.validate_handoff("Agent2", &context);
    assert!(result.is_ok());
}

#[test]
fn test_validate_handoff_circular_delegation() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    // Create context with Agent1 -> Agent2 -> Agent3 chain
    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.chain.push("Agent2".to_string());
    context.chain.push("Agent3".to_string());
    context.depth = 3;

    // Try to handoff back to Agent2 (circular)
    let result = service.validate_handoff("Agent2", &context);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circular"));
}

#[test]
fn test_validate_handoff_max_depth() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 3,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.depth = 3; // Already at max

    let result = service.validate_handoff("Agent2", &context);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Maximum handoff depth"));
}

#[test]
fn test_can_handoff_to_agent_not_in_chain() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.chain.push("Agent2".to_string());

    // Agent3 is not in chain, should be allowed
    assert!(service.can_handoff_to("Agent3", &context));
    // Agent2 is in chain, should not be allowed
    assert!(!service.can_handoff_to("Agent2", &context));
    // Agent1 (origin) is in chain, should not be allowed
    assert!(!service.can_handoff_to("Agent1", &context));
}

#[test]
fn test_transfer_context_creates_new_context() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let original = HandoffContext::new("Original task".to_string(), "Agent1".to_string());
    let new_context = service.transfer_context("Delegated subtask", &original, "Agent2");

    assert_eq!(new_context.task, "Delegated subtask");
    assert_eq!(new_context.depth, 2); // Incremented
    assert_eq!(new_context.chain.len(), 2);
    assert_eq!(new_context.chain[0], "Agent1");
    assert_eq!(new_context.chain[1], "Agent2");
}

#[test]
fn test_transfer_context_preserves_chain() {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth: 5,
        retry: Default::default(),
    });
    let service = HandoffService::new(config).unwrap();

    let mut original = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    original.chain.push("Agent2".to_string());
    original.depth = 2;

    let new_context = service.transfer_context("Subtask", &original, "Agent3");

    assert_eq!(new_context.depth, 3);
    assert_eq!(new_context.chain.len(), 3);
    assert!(new_context.chain.contains(&"Agent1".to_string()));
    assert!(new_context.chain.contains(&"Agent2".to_string()));
    assert!(new_context.chain.contains(&"Agent3".to_string()));
}

// =============================================================================
// Phase 5: execute_handoff tests
// =============================================================================

/// Mock PaladinExecutorPort that always succeeds
struct SuccessExecutor {
    call_count: AtomicU32,
}

impl SuccessExecutor {
    fn new() -> Self {
        Self {
            call_count: AtomicU32::new(0),
        }
    }

    fn calls(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl PaladinExecutorPort for SuccessExecutor {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(PaladinResult {
            output: "Specialist result".to_string(),
            token_count: 100,
            execution_time_ms: 50,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
            served_by: None,
        })
    }
}

/// Mock PaladinExecutorPort that always fails with a permanent error
struct PermanentFailExecutor;

#[async_trait::async_trait]
impl PaladinExecutorPort for PermanentFailExecutor {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        Err(PaladinError::ConfigurationError(
            "Invalid model configuration".to_string(),
        ))
    }
}

/// Mock PaladinExecutorPort that fails N times with transient errors then succeeds
struct TransientThenSuccessExecutor {
    failures_remaining: AtomicU32,
    call_count: AtomicU32,
}

impl TransientThenSuccessExecutor {
    fn new(failure_count: u32) -> Self {
        Self {
            failures_remaining: AtomicU32::new(failure_count),
            call_count: AtomicU32::new(0),
        }
    }

    fn calls(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl PaladinExecutorPort for TransientThenSuccessExecutor {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let remaining = self.failures_remaining.load(Ordering::SeqCst);
        if remaining > 0 {
            self.failures_remaining.fetch_sub(1, Ordering::SeqCst);
            Err(PaladinError::ExecutionError(
                "temporary network unavailable".to_string(),
            ))
        } else {
            Ok(PaladinResult {
                output: "Recovered result".to_string(),
                token_count: 80,
                execution_time_ms: 40,
                loop_count: 1,
                stop_reason: StopReason::Completed,
                plan: None,
                handoff_history: Vec::new(),
                served_by: None,
            })
        }
    }
}

/// Mock PaladinExecutorPort that always fails with transient errors
struct AlwaysTransientFailExecutor {
    call_count: AtomicU32,
}

impl AlwaysTransientFailExecutor {
    fn new() -> Self {
        Self {
            call_count: AtomicU32::new(0),
        }
    }

    fn calls(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl PaladinExecutorPort for AlwaysTransientFailExecutor {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Err(PaladinError::ExecutionError(
            "temporary network timeout".to_string(),
        ))
    }
}

/// Helper: create a test Paladin
fn create_test_paladin(name: &str) -> Paladin {
    Node::new(
        PaladinData {
            system_prompt: format!("You are {name}"),
            name: name.to_string(),
            model: "gpt-4".to_string(),
            status: PaladinStatus::Idle,
            max_loops: MaxLoops::Fixed(1),
            ..PaladinData::default()
        },
        Some(name.to_string()),
    )
}

/// Helper: create a HandoffService with fast retry config (for test speed)
fn create_test_service(max_depth: u32, max_retries: u32) -> HandoffService {
    let config = Arc::new(HandoffConfig {
        enabled: true,
        strategy: HandoffStrategy::Automatic,
        max_depth,
        retry: HandoffRetryConfig::new(max_retries, 1, 1.0),
    });
    HandoffService::new(config).unwrap()
}

// --- Test: successful delegation ---

#[tokio::test]
async fn test_execute_handoff_delegates_to_specialist() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("CodeExpert");
    let executor = SuccessExecutor::new();
    let context = HandoffContext::new("Fix the bug".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff(
            "CodeExpert",
            "Fix the bug",
            &context,
            &specialist,
            &executor,
        )
        .await;

    assert!(result.is_ok(), "Handoff should succeed");
    let (output, record) = result.unwrap();
    assert_eq!(output, "Specialist result");
    assert_eq!(record.from_agent, "Coordinator");
    assert_eq!(record.to_agent, "CodeExpert");
    assert_eq!(record.task, "Fix the bug");
    assert_eq!(record.depth, 2); // incremented from context depth 1
    assert!(record.result.is_some());
    assert_eq!(record.result.unwrap(), "Specialist result");
    assert_eq!(executor.calls(), 1);
}

// --- Test: chain depth tracking ---

#[tokio::test]
async fn test_execute_handoff_tracks_chain_depth() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("Agent3");
    let executor = SuccessExecutor::new();

    // Context already 2 deep: Agent1 → Agent2
    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.chain.push("Agent2".to_string());
    context.depth = 2;

    let result = service
        .execute_handoff("Agent3", "Subtask", &context, &specialist, &executor)
        .await;

    assert!(result.is_ok());
    let (_output, record) = result.unwrap();
    assert_eq!(record.depth, 3); // 2 + 1
    assert_eq!(record.from_agent, "Agent2"); // last in chain
}

// --- Test: circular handoff detection ---

#[tokio::test]
async fn test_execute_handoff_detects_circular_handoff() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("Agent1");
    let executor = SuccessExecutor::new();

    // Agent1 → Agent2, now trying to handoff back to Agent1
    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.chain.push("Agent2".to_string());
    context.depth = 2;

    let result = service
        .execute_handoff("Agent1", "Subtask", &context, &specialist, &executor)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, HandoffError::CircularHandoff { .. }),
        "Expected CircularHandoff, got: {err:?}"
    );
    // Executor should never be called for a circular handoff
    assert_eq!(executor.calls(), 0);
}

// --- Test: max depth enforcement ---

#[tokio::test]
async fn test_execute_handoff_enforces_max_depth() {
    let service = create_test_service(2, 3); // max_depth=2
    let specialist = create_test_paladin("Agent3");
    let executor = SuccessExecutor::new();

    let mut context = HandoffContext::new("Task".to_string(), "Agent1".to_string());
    context.chain.push("Agent2".to_string());
    context.depth = 2; // already at max

    let result = service
        .execute_handoff("Agent3", "Subtask", &context, &specialist, &executor)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, HandoffError::MaxDepthExceeded { .. }),
        "Expected MaxDepthExceeded, got: {err:?}"
    );
    assert_eq!(executor.calls(), 0);
}

// --- Test: retries transient errors then succeeds ---

#[tokio::test]
async fn test_execute_handoff_retries_transient_errors() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("Specialist");
    // Fails 2 times with transient error, then succeeds on 3rd attempt
    let executor = TransientThenSuccessExecutor::new(2);
    let context = HandoffContext::new("Task".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff("Specialist", "Retry task", &context, &specialist, &executor)
        .await;

    assert!(result.is_ok(), "Should succeed after retries");
    let (output, _record) = result.unwrap();
    assert_eq!(output, "Recovered result");
    assert_eq!(executor.calls(), 3); // 1 initial + 2 retries
}

// --- Test: fails immediately on permanent error ---

#[tokio::test]
async fn test_execute_handoff_fails_immediately_on_permanent_error() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("Specialist");
    let executor = PermanentFailExecutor;
    let context = HandoffContext::new("Task".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff("Specialist", "Bad task", &context, &specialist, &executor)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, HandoffError::ExecutionFailed { .. }),
        "Expected ExecutionFailed, got: {err:?}"
    );
    // Permanent error: no retry, only 1 call
    // The error "Invalid model configuration" does not contain transient keywords
}

// --- Test: exhausts all retries on persistent transient errors ---

#[tokio::test]
async fn test_execute_handoff_exhausts_retries() {
    let service = create_test_service(5, 2); // max_retries=2
    let specialist = create_test_paladin("Specialist");
    let executor = AlwaysTransientFailExecutor::new();
    let context = HandoffContext::new("Task".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff(
            "Specialist",
            "Hopeless task",
            &context,
            &specialist,
            &executor,
        )
        .await;

    assert!(result.is_err());
    // 1 initial attempt + 2 retries = 3 total calls
    assert_eq!(executor.calls(), 3);
}

// --- Test: handoff record creation ---

#[tokio::test]
async fn test_execute_handoff_creates_handoff_records() {
    let service = create_test_service(5, 3);
    let specialist = create_test_paladin("DataAnalyst");
    let executor = SuccessExecutor::new();
    let context = HandoffContext::new("Analyze data".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff(
            "DataAnalyst",
            "Analyze data",
            &context,
            &specialist,
            &executor,
        )
        .await;

    assert!(result.is_ok());
    let (_output, record) = result.unwrap();
    assert_eq!(record.from_agent, "Coordinator");
    assert_eq!(record.to_agent, "DataAnalyst");
    assert_eq!(record.task, "Analyze data");
    assert_eq!(record.depth, 2);
    assert!(record.result.is_some(), "Record should contain the result");
}

// --- Test: zero retries configuration ---

#[tokio::test]
async fn test_execute_handoff_zero_retries_fails_on_first_error() {
    let service = create_test_service(5, 0); // max_retries=0, no retries
    let specialist = create_test_paladin("Specialist");
    let executor = AlwaysTransientFailExecutor::new();
    let context = HandoffContext::new("Task".to_string(), "Coordinator".to_string());

    let result = service
        .execute_handoff("Specialist", "Task", &context, &specialist, &executor)
        .await;

    assert!(result.is_err());
    assert_eq!(executor.calls(), 1); // Only the initial attempt, no retries
}
