//! Examples for `docs/src/user-guides/agent-runtime.md` (Phase 26, D-35,
//! D-38).
//!
//! The `// ANCHOR:` region below is pulled into the Agent Runtime user
//! guide via mdBook `{{#include}}`, so the guide's example cannot drift
//! from the landed API: `cargo check -p paladin-doc-examples` compiles it.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;

use paladin::InProcessArsenal;
use paladin::presets::reasoning_agent;
use paladin_core::platform::container::arsenal::Armament;
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::output::paladin_port::StopReason;
use serde_json::json;

/// The one `add` tool the example registers -- kept OUTSIDE the anchored
/// region so the reader-facing body stays within RT-FR-23's 15-line limit.
fn add_armament() -> Armament {
    Armament {
        name: "add".to_string(),
        description: "Adds two integers".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}},
            "required": ["a", "b"]
        }),
        required_params: vec![],
    }
}

// ANCHOR: reasoning_agent
pub async fn reasoning_agent_example() -> Result<(), Box<dyn std::error::Error>> {
    let llm = MockLlmAdapter::new().with_responses(vec![
        r#"{"tool":"add","arguments":{"a":2,"b":2}}"#.to_string(),
        "The answer is 4".to_string(),
    ]);
    let arsenal = InProcessArsenal::new()
        .with_tool(add_armament(), |_args| async { Ok(json!({ "sum": 4 })) });
    let agent = reasoning_agent(Arc::new(llm), Arc::new(arsenal), Default::default())?;
    let result = agent.run("What is 2+2?").await?;
    assert!(result.output.contains('4'));
    assert_eq!(result.loop_count, 2);
    assert_eq!(result.stop_reason, StopReason::Completed);
    Ok(())
}
// ANCHOR_END: reasoning_agent
