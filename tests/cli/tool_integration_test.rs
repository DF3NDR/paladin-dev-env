//! Tool integration tests for Arsenal tool call flows
//!
//! Tests the end-to-end tool call loop: LLM requests tool → Arsenal invokes tool →
//! result is formatted and fed back to LLM for next iteration. Uses in-process
//! mocks (MockLlmAdapter + MockArsenalPort) for CI-friendly testing without
//! external dependencies.

use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::base::entity::Entity;
use paladin::core::platform::container::arsenal::ArmamentResult;
use paladin::core::platform::container::garrison::{ConversationRole, GarrisonConfig};
use paladin::core::platform::container::paladin::{MaxLoops, PaladinData, PaladinStatus};
use paladin::infrastructure::adapters::garrison::InMemoryGarrison;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::LlmPort;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

// Test helper imports
use crate::helpers::{MockArsenalPort, MockLlmAdapter};

/// Create a test Paladin with minimal configuration for tool integration tests
fn create_test_paladin(max_loops: MaxLoops) -> Entity<PaladinData> {
    let paladin_data = PaladinData {
        system_prompt: "You are a helpful AI assistant with access to tools.".to_string(),
        name: "ToolTestPaladin".to_string(),
        user_name: "TestUser".to_string(),
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_loops,
        stop_words: vec![],
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };
    Entity::new(paladin_data, None)
}

/// Create PaladinExecutionService with CircuitBreaker wired up
fn create_service(
    llm: Arc<dyn LlmPort>,
    arsenal: Option<Arc<dyn ArsenalPort>>,
) -> PaladinExecutionService {
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    PaladinExecutionService::new(
        llm,
        circuit_breaker,
        None, // No garrison for these basic tests
        arsenal,
    )
}

#[tokio::test]
async fn test_tool_call_basic_flow() {
    // Arrange: Set up MockLlmAdapter with tool call + final response
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("calculator", r#"{"operation":"add","a":1,"b":2}"#);
    mock_llm.add_success("The result is 3");

    // Arrange: Set up MockArsenalPort with calculator tool
    let mut mock_arsenal = MockArsenalPort::new();
    mock_arsenal.add_tool("calculator", "A basic calculator");
    let mock_arsenal = Arc::new(mock_arsenal);

    let call_id = Uuid::new_v4();
    mock_arsenal.set_response(
        "calculator",
        ArmamentResult::success(call_id, json!("3"), 100),
    );

    // Arrange: Create Paladin and execution service
    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act: Execute Paladin with tool-requiring query
    let result = service.execute(&paladin, "What is 1 + 2?").await;

    // Assert: Execution succeeds
    assert!(
        result.is_ok(),
        "Execution should succeed: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // Assert: Output contains tool result
    assert!(
        output.output.contains("3") || output.output.contains("calculator"),
        "Output should contain tool result or tool name. Got: {}",
        output.output
    );

    // Assert: LLM called twice (tool call + final)
    assert_eq!(
        mock_llm.call_count(),
        2,
        "LLM should be called twice: tool call then final response"
    );

    // Assert: Arsenal invoked once
    assert_eq!(
        mock_arsenal.call_count(),
        1,
        "Arsenal should be invoked once for calculator"
    );

    println!("✓ Basic tool call flow test passed");
    println!("  Output: {}", output.output);
}

#[tokio::test]
async fn test_tool_call_result_fed_back_to_llm() {
    // Arrange: Same setup as basic flow
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("calculator", r#"{"operation":"add","a":5,"b":7}"#);
    mock_llm.add_success("The calculation shows the answer is 12");

    let mut mock_arsenal = MockArsenalPort::new();
    mock_arsenal.add_tool("calculator", "A calculator");
    let mock_arsenal = Arc::new(mock_arsenal);

    let call_id = Uuid::new_v4();
    mock_arsenal.set_response(
        "calculator",
        ArmamentResult::success(call_id, json!("12"), 50),
    );

    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act: Execute
    let result = service.execute(&paladin, "What is 5 + 7?").await;

    // Assert: Execution succeeds
    assert!(result.is_ok(), "Execution should succeed");

    // Assert: Inspect invocations to verify tool result was fed back
    let invocations = mock_llm.invocations();
    assert_eq!(
        invocations.len(),
        2,
        "Should have exactly 2 LLM invocations"
    );

    // The second invocation's prompt should contain the formatted tool result
    let second_prompt = &invocations[1].prompt;

    // Check for formatted tool result markers (from ToolResultFormatter)
    let has_tool_execution = second_prompt.contains("Tool Execution")
        || second_prompt.contains("calculator")
        || second_prompt.contains("🔧");
    let has_result_marker = second_prompt.contains("Result:") || second_prompt.contains("SUCCESS");
    let has_output = second_prompt.contains("12") || second_prompt.contains("Output:");

    assert!(
        has_tool_execution,
        "Second LLM call should contain tool execution marker. Prompt: {}",
        second_prompt
    );
    assert!(
        has_result_marker || has_output,
        "Second LLM call should contain result marker or output. Prompt: {}",
        second_prompt
    );

    println!("✓ Tool result feedback test passed");
    println!(
        "  Second invocation prompt length: {} chars",
        second_prompt.len()
    );
    println!(
        "  Contains 'calculator': {}",
        second_prompt.contains("calculator")
    );
    println!("  Contains '12': {}", second_prompt.contains("12"));
}

// ========================================================================================
// Error Handling Tests (Task 3.0 - FR-2.4 to FR-2.7)
// ========================================================================================

/// FR-2.4: Test tool call when no arsenal is available
#[tokio::test]
async fn test_tool_call_no_arsenal_available() {
    println!("\n▶ Testing tool call with no arsenal available...");

    // Arrange: Mock LLM that tries to call a tool
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call(
        "calculator",
        r#"{"operation":"add","a":1,"b":2}"#.to_string(),
    );
    mock_llm.add_success("Done without tool".to_string());

    // Create Paladin and service WITHOUT arsenal
    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(mock_llm.clone() as Arc<dyn LlmPort>, None);

    // Act: Execute
    let result = service.execute(&paladin, "What is 1 + 2?").await;

    // Assert: Should not crash, execution completes gracefully
    assert!(
        result.is_ok(),
        "Execution should succeed even without arsenal"
    );

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // Should complete both loops (tool call attempt + final response)
    assert_eq!(
        mock_llm.call_count(),
        2,
        "Should have 2 LLM calls (tool call + final)"
    );

    println!("✓ No arsenal available test passed");
}

/// FR-2.5: Test tool call with unknown/unregistered tool
#[tokio::test]
async fn test_tool_call_unknown_tool() {
    println!("\n▶ Testing tool call with unknown tool...");

    // Arrange: Mock LLM calling a nonexistent tool
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("nonexistent_tool", r#"{"arg":"value"}"#.to_string());
    mock_llm.add_success("Proceeding anyway".to_string());

    // Mock Arsenal with NO tools registered (invoke will return ToolNotFound)
    let mock_arsenal = Arc::new(MockArsenalPort::new());
    // Don't add any tools - leaving it empty

    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act
    let result = service.execute(&paladin, "Use the nonexistent tool").await;

    // Assert: Execution succeeds with graceful error handling
    assert!(result.is_ok(), "Should handle unknown tool gracefully");

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // Graceful degradation: execution continues without crashing
    // The error may or may not be visible in the output, depending on LLM response
    // Key is that the system doesn't crash

    assert_eq!(mock_llm.call_count(), 2, "Should have 2 LLM calls");
    assert_eq!(
        mock_arsenal.call_count(),
        1,
        "Should have attempted 1 tool invocation"
    );

    println!("✓ Unknown tool test passed");
}

/// FR-2.6: Test tool call with invalid arguments (malformed JSON)
#[tokio::test]
async fn test_tool_call_invalid_arguments() {
    println!("\n▶ Testing tool call with invalid arguments...");

    // Arrange: Mock LLM providing invalid JSON arguments
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("calculator", "not valid json{{{".to_string());
    mock_llm.add_success("Recovered from error".to_string());

    // Mock Arsenal with calculator tool
    let mut mock_arsenal_raw = MockArsenalPort::new();
    mock_arsenal_raw.add_tool("calculator", "A calculator tool");
    let mock_arsenal = Arc::new(mock_arsenal_raw);
    mock_arsenal.set_response(
        "calculator",
        ArmamentResult::success(Uuid::new_v4(), json!("Should not reach here"), 10),
    );

    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act
    let result = service.execute(&paladin, "Calculate with bad args").await;

    // Assert: Graceful degradation
    assert!(result.is_ok(), "Should handle invalid arguments gracefully");

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // The tool invocation might fail early due to JSON parsing, or succeed but return error
    // Either way, execution should complete
    assert!(
        mock_llm.call_count() >= 1,
        "Should have at least 1 LLM call"
    );

    println!("✓ Invalid arguments test passed");
}

/// FR-2.7: Test tool execution error
#[tokio::test]
async fn test_tool_call_execution_error() {
    println!("\n▶ Testing tool call execution error...");

    // Arrange: Mock LLM calling a tool that will fail
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("failing_tool", r#"{"input":"test"}"#.to_string());
    mock_llm.add_success("Handled the error".to_string());

    // Mock Arsenal configured to return an error for this tool
    let mut mock_arsenal_raw = MockArsenalPort::new();
    mock_arsenal_raw.add_tool("failing_tool", "A tool that fails");
    let mock_arsenal = Arc::new(mock_arsenal_raw);
    mock_arsenal.set_error("failing_tool", "Simulated execution failure");

    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act
    let result = service.execute(&paladin, "Use the failing tool").await;

    // Assert: Execution succeeds, error is handled gracefully
    assert!(result.is_ok(), "Should handle tool execution error");

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // Output should contain error information
    let has_error = output.output.contains("FAILED")
        || output.output.contains("error")
        || output.output.contains("Error")
        || output.output.contains("failure");

    assert!(
        has_error,
        "Output should contain error message. Got: {}",
        output.output
    );

    assert_eq!(mock_llm.call_count(), 2, "Should have 2 LLM calls");
    assert_eq!(
        mock_arsenal.call_count(),
        1,
        "Should have attempted tool execution"
    );

    println!("✓ Execution error test passed");
}

// ========================================================================================
// Advanced Tests (Task 4.0 - FR-2.8 to FR-2.9)
// ========================================================================================

/// Helper function to create service with garrison enabled
fn create_service_with_garrison(
    llm: Arc<dyn LlmPort>,
    arsenal: Option<Arc<dyn ArsenalPort>>,
    garrison: Arc<dyn GarrisonPort>,
) -> PaladinExecutionService {
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    PaladinExecutionService::new(llm, circuit_breaker, Some(garrison), arsenal)
}

/// FR-2.8: Test multiple sequential tool calls
#[tokio::test]
async fn test_multiple_sequential_tool_calls() {
    println!("\n▶ Testing multiple sequential tool calls...");

    // Arrange: Mock LLM that calls two tools then gives final answer
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call("tool_a", r#"{"param":"value_a"}"#.to_string());
    mock_llm.add_tool_call("tool_b", r#"{"param":"value_b"}"#.to_string());
    mock_llm.add_success("Final answer after using both tools".to_string());

    // Mock Arsenal with both tools
    let mut mock_arsenal_raw = MockArsenalPort::new();
    mock_arsenal_raw.add_tool("tool_a", "First tool");
    mock_arsenal_raw.add_tool("tool_b", "Second tool");
    let mock_arsenal = Arc::new(mock_arsenal_raw);

    mock_arsenal.set_response(
        "tool_a",
        ArmamentResult::success(Uuid::new_v4(), json!("Result from tool A"), 10),
    );
    mock_arsenal.set_response(
        "tool_b",
        ArmamentResult::success(Uuid::new_v4(), json!("Result from tool B"), 10),
    );

    // Paladin with enough loops for 3 LLM calls (tool_a, tool_b, final)
    let paladin = create_test_paladin(MaxLoops::Fixed(3));
    let service = create_service(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
    );

    // Act
    let result = service.execute(&paladin, "Use both tools").await;

    // Assert
    assert!(result.is_ok(), "Should handle multiple tool calls");

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // Should have 3 LLM invocations: tool_a call, tool_b call, final response
    assert_eq!(
        mock_llm.call_count(),
        3,
        "Should have 3 LLM calls (tool_a + tool_b + final)"
    );

    // Should have 2 arsenal invocations
    assert_eq!(
        mock_arsenal.call_count(),
        2,
        "Should have 2 tool invocations"
    );

    // Verify execution completed successfully with both tools used
    assert!(
        output.output.contains("Final answer"),
        "Output should contain final answer. Got: {}",
        output.output
    );

    println!("✓ Multiple sequential tool calls test passed");
}

/// FR-2.9: Test tool call with garrison (memory) integration
#[tokio::test]
async fn test_tool_call_with_garrison() {
    println!("\n▶ Testing tool call with garrison integration...");

    // Arrange: Create garrison
    let garrison =
        Arc::new(InMemoryGarrison::new(GarrisonConfig::default())) as Arc<dyn GarrisonPort>;

    // Mock LLM calling a tool
    let mock_llm = Arc::new(MockLlmAdapter::new());
    mock_llm.add_tool_call(
        "calculator",
        r#"{"operation":"multiply","a":6,"b":7}"#.to_string(),
    );
    mock_llm.add_success("The calculation is complete".to_string());

    // Mock Arsenal with calculator
    let mut mock_arsenal_raw = MockArsenalPort::new();
    mock_arsenal_raw.add_tool("calculator", "A calculator tool");
    let mock_arsenal = Arc::new(mock_arsenal_raw);

    mock_arsenal.set_response(
        "calculator",
        ArmamentResult::success(Uuid::new_v4(), json!(42), 5),
    );

    let paladin = create_test_paladin(MaxLoops::Fixed(2));
    let service = create_service_with_garrison(
        mock_llm.clone() as Arc<dyn LlmPort>,
        Some(mock_arsenal.clone()),
        garrison.clone(),
    );

    // Act
    let result = service.execute(&paladin, "What is 6 times 7?").await;

    // Assert: Execution succeeds
    assert!(result.is_ok(), "Should execute successfully with garrison");

    let output = result.unwrap();
    println!("  Output: {}", output.output);

    // Verify garrison contains tool entry
    let entries = garrison
        .recall_recent(100)
        .await
        .expect("Failed to get entries from garrison");
    println!("  Garrison has {} entries", entries.len());

    // Should have at least one Tool role entry (for the tool result)
    let tool_entries: Vec<_> = entries
        .iter()
        .filter(|e| matches!(e.role, ConversationRole::Tool))
        .collect();

    assert!(
        !tool_entries.is_empty(),
        "Garrison should contain at least one Tool role entry"
    );

    println!("  Tool entries in garrison: {}", tool_entries.len());

    // The tool entry should contain information about the calculator call
    let tool_entry_text = &tool_entries[0].content;
    let has_calculator_ref = tool_entry_text.contains("calculator")
        || tool_entry_text.contains("42")
        || tool_entry_text.contains("Tool Execution");

    assert!(
        has_calculator_ref,
        "Tool entry should reference the calculator or its result. Got: {}",
        tool_entry_text
    );

    println!("✓ Garrison integration test passed");
}
