//! Examples for `docs/src/user-guides/arsenal-tools.md` (Phase 35, MB-19).
//!
//! Every `// ANCHOR:` region below is pulled into the Arsenal Tools user guide via
//! mdBook `{{#include}}`, so a sample in the guide cannot drift from the landed API:
//! `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::llm_port::LlmPort;

// ANCHOR: custom_armament
use async_trait::async_trait;
use paladin_core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin_ports::output::arsenal_port::ArsenalPort;

/// Implement `ArsenalPort` to expose any Rust function as a tool.
pub struct CalculatorTool;

#[async_trait]
impl ArsenalPort for CalculatorTool {
    async fn list_armaments(&self) -> Vec<Armament> {
        vec![Armament {
            name: "calculate".to_string(),
            description: "Evaluate a mathematical expression".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string" }
                },
                "required": ["expression"]
            }),
            required_params: vec!["expression".to_string()],
        }]
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        // Arguments live on the `arguments` map, not an `args` field.
        let expr = call
            .arguments
            .get("expression")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        // ... evaluate `expr` ...
        Ok(ArmamentResult {
            call_id: call.call_id,
            success: true,
            output: Some(serde_json::json!(42)),
            error: None,
            execution_time_ms: 1,
        })
    }

    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
        if call.arguments.contains_key("expression") {
            Ok(())
        } else {
            Err(ArsenalError::InvalidArguments(
                "expression is required".into(),
            ))
        }
    }
}
// ANCHOR_END: custom_armament

// ANCHOR: handoffs
/// Register specialist agents on the builder so the built-in handoff Armament can
/// delegate to them at runtime — `with_handoffs` takes the whole specialist list at
/// once, there is no per-call chainable registration method.
pub async fn build_coordinator_with_handoffs() -> Result<(), Box<dyn std::error::Error>> {
    let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());

    let code_paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You review code changes.")
        .name("CodeReviewer")
        .build()
        .await?;
    let test_paladin = PaladinBuilder::new(llm_port.clone())
        .system_prompt("You write and run tests.")
        .name("TestEngineer")
        .build()
        .await?;

    let coordinator = PaladinBuilder::new(llm_port)
        .system_prompt("You are a coordinator. Delegate to specialists when needed.")
        .with_handoffs(vec![Arc::new(code_paladin), Arc::new(test_paladin)])
        .build()
        .await?;

    let _ = coordinator;
    Ok(())
}
// ANCHOR_END: handoffs
