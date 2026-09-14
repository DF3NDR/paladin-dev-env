//! End-to-end tests for `paladin::presets::reasoning_agent` (Doc 05 RT-07,
//! RT-FR-23/24, D-35, D-36) -- the one-liner that composes every built-in
//! this phase shipped in isolation into a runnable, tool-using agent.
//!
//! `defaults_match_the_documented_options` (plan 26-20 Test 2) lives beside
//! `reasoning_agent` in `src/presets/mod.rs` instead: it asserts against
//! `ReasoningAgentOptions`' private fields, which only a same-module test
//! can reach.

use std::sync::Arc;

use paladin::InProcessArsenal;
use paladin::presets::{ReasoningAgentOptions, reasoning_agent};
use paladin_core::platform::container::arsenal::Armament;
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::paladin_port::StopReason;
use serde::Deserialize;
use serde_json::json;

fn add_armament() -> Armament {
    Armament {
        name: "add".to_string(),
        description: "Adds two integers".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
            "required": ["a", "b"]
        }),
        required_params: vec!["a".to_string(), "b".to_string()],
    }
}

fn add_arsenal() -> Arc<dyn ArsenalPort> {
    Arc::new(
        InProcessArsenal::new().with_tool(add_armament(), |args| async move {
            let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(json!({ "sum": a + b }))
        }),
    )
}

/// Test 1: a tool-call envelope followed by a plain answer produces an
/// output containing the answer, `loop_count == 2` and
/// `StopReason::Completed`.
#[tokio::test]
async fn reasoning_agent_runs_a_tool_and_answers() {
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"tool":"add","arguments":{"a":2,"b":2}}"#.to_string(),
        "The answer is 4".to_string(),
    ]));
    let agent = reasoning_agent(llm, add_arsenal(), ReasoningAgentOptions::default())
        .expect("valid options must build");

    let result = agent.run("What is 2+2?").await.expect("run must succeed");

    assert!(result.output.contains('4'), "{}", result.output);
    assert_eq!(result.loop_count, 2);
    assert_eq!(result.stop_reason, StopReason::Completed);
}

/// Test 3: a preset built with an EMPTY arsenal still runs -- no `##
/// Tools` section is rendered and the agent answers in one loop.
#[tokio::test]
async fn an_empty_arsenal_still_runs() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("Hello there!"));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(InProcessArsenal::new());

    let agent = reasoning_agent(llm.clone(), arsenal, ReasoningAgentOptions::default())
        .expect("valid options must build");
    let result = agent.run("hi").await.expect("run must succeed");

    assert_eq!(result.output, "Hello there!");
    assert_eq!(result.loop_count, 1);
    assert_eq!(result.stop_reason, StopReason::Completed);
    let prompt = llm.last_prompt().expect("a prompt must have been sent");
    assert!(!prompt.contains("## Tools"), "{prompt}");
}

/// Test 4: with `max_tool_calls: 1` and a model that tries two tool calls,
/// the second is denied and the run still completes.
#[tokio::test]
async fn max_tool_calls_is_enforced() {
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"tool":"add","arguments":{"a":1,"b":1}}"#.to_string(),
        r#"{"tool":"add","arguments":{"a":2,"b":2}}"#.to_string(),
        "Done".to_string(),
    ]));
    let opts = ReasoningAgentOptions::default().max_tool_calls(1);
    let agent = reasoning_agent(llm, add_arsenal(), opts).expect("valid options must build");

    let result = agent.run("add things").await.expect("run must complete");
    assert_eq!(result.stop_reason, StopReason::Completed);
}

/// Test 5: a closure returning an error produces fed-back text the model
/// sees on the next iteration, and the run completes (the default
/// `FeedToModel` tool-error policy).
#[tokio::test]
async fn tool_failure_is_fed_back_by_default() {
    let failing_arsenal: Arc<dyn ArsenalPort> = Arc::new(
        InProcessArsenal::new().with_tool(add_armament(), |_args| async move {
            Err("deliberate failure".to_string())
        }),
    );
    let llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"tool":"add","arguments":{"a":1,"b":1}}"#.to_string(),
        "I could not complete the calculation.".to_string(),
    ]));

    let agent = reasoning_agent(llm, failing_arsenal, ReasoningAgentOptions::default())
        .expect("valid options must build");
    let result = agent.run("add things").await.expect("run must complete");

    assert_eq!(result.stop_reason, StopReason::Completed);
    assert_eq!(result.loop_count, 2);
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct Weather {
    city: String,
}

/// Test 6: `run_structured` delegates to the same `StructuredExecutorExt`
/// machinery `execute_structured` uses.
#[tokio::test]
async fn run_structured_delegates_to_the_extension() {
    let llm = Arc::new(MockLlmAdapter::new().with_response(r#"{"city": "Oslo"}"#));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(InProcessArsenal::new());
    let agent = reasoning_agent(llm, arsenal, ReasoningAgentOptions::default())
        .expect("valid options must build");

    let weather: Weather = agent
        .run_structured::<Weather>("What is the weather?")
        .await
        .expect("structured run must succeed")
        .value;

    assert_eq!(weather.city, "Oslo");
}

/// Test 7: with a `garrison` supplied via the options, the run's history
/// is recalled from and written to it.
#[tokio::test]
async fn a_garrison_is_used_when_supplied() {
    let garrison: Arc<dyn GarrisonPort> = Arc::new(
        paladin_memory::garrison::in_memory_garrison::InMemoryGarrison::new(
            paladin_core::platform::container::garrison::GarrisonConfig::default(),
        ),
    );
    let llm = Arc::new(MockLlmAdapter::new().with_response("Nice to meet you!"));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(InProcessArsenal::new());
    let opts = ReasoningAgentOptions::default().garrison(garrison.clone());

    let agent = reasoning_agent(llm, arsenal, opts).expect("valid options must build");
    agent.run("My name is Ada").await.expect("run must succeed");

    let history = garrison
        .recall_recent(10)
        .await
        .expect("recall must succeed");
    assert!(
        history
            .iter()
            .any(|entry| entry.content.contains("My name is Ada")),
        "{history:?}"
    );
}

/// Test 8: `reasoning_agent` never calls `with_vault` or
/// `enable_vault_tools` on the service it builds -- the rendered prompt
/// never lists `vault_get`/`vault_put`, and there is no way to pass a
/// vault or a vault-tools flag through `ReasoningAgentOptions` at all.
/// Vault access stays an explicit, separate opt-in (T-26-63, D-21, D-22).
#[tokio::test]
async fn the_preset_does_not_enable_vault_tools() {
    let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
    let arsenal: Arc<dyn ArsenalPort> = Arc::new(InProcessArsenal::new());
    let agent = reasoning_agent(llm.clone(), arsenal, ReasoningAgentOptions::default())
        .expect("valid options must build");

    agent.run("hi").await.expect("run must succeed");
    let prompt = llm.last_prompt().expect("a prompt must have been sent");
    assert!(!prompt.contains("vault_get"), "{prompt}");
    assert!(!prompt.contains("vault_put"), "{prompt}");
}
