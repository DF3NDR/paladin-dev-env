//! `CompositeArsenalPort` — unions several [`ArsenalPort`]s into one (Doc 05
//! RT-04, D-22).
//!
//! Follows the same chain-of-ports-as-one-port shape as
//! [`paladin_llm::fallback::FallbackLlmAdapter`] (26-PATTERNS.md's named
//! analog): a `Vec<Arc<dyn ArsenalPort>>` presented behind the single
//! `ArsenalPort` trait, so a caller that composes several tool sources
//! (e.g. an MCP-backed [`ArsenalExecutionService`](
//! super::arsenal_execution_service::ArsenalExecutionService) plus the
//! built-in [`VaultTools`](super::vault_tools::VaultTools)) never has to
//! special-case which one actually serves a given call.
//!
//! # First-registration-of-a-name-wins
//!
//! When two members declare a tool with the same name, the **first**
//! member's definition is the one [`list_armaments`](ArsenalPort::list_armaments),
//! [`invoke`](ArsenalPort::invoke) and [`validate_call`](ArsenalPort::validate_call)
//! all resolve to — the duplicate from a later member is never reachable
//! through this composite. Every duplicate is logged at `warn` naming both
//! the tool and its position in the member list, so a shadowed tool is
//! diagnosable rather than silently dropped.
//!
//! # An empty composite denies everything
//!
//! `CompositeArsenalPort::new(vec![])` lists no tools and returns a typed
//! [`ArsenalError::ToolNotFound`] for any [`invoke`](ArsenalPort::invoke) or
//! [`validate_call`](ArsenalPort::validate_call) call — it does not silently
//! succeed with an empty result, which would make a genuinely mis-wired
//! composite (no members configured at all) indistinguishable from a
//! healthy one with nothing to do.

use std::sync::Arc;

use async_trait::async_trait;
use log::warn;

use paladin_core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};
use paladin_ports::output::arsenal_port::ArsenalPort;

/// Unions the [`Armament`]s and dispatch of several [`ArsenalPort`]
/// implementors behind one `ArsenalPort`, resolving name collisions by
/// first-registration-wins (module documentation above).
pub struct CompositeArsenalPort {
    members: Vec<Arc<dyn ArsenalPort>>,
}

impl CompositeArsenalPort {
    /// Builds a composite over `members`, in priority order: `members[0]`'s
    /// tool definitions and dispatch win any name collision with a later
    /// member.
    pub fn new(members: Vec<Arc<dyn ArsenalPort>>) -> Self {
        Self { members }
    }
}

#[async_trait]
impl ArsenalPort for CompositeArsenalPort {
    async fn list_armaments(&self) -> Vec<Armament> {
        let mut seen = std::collections::HashSet::new();
        let mut union = Vec::new();

        for (index, member) in self.members.iter().enumerate() {
            for armament in member.list_armaments().await {
                if seen.insert(armament.name.clone()) {
                    union.push(armament);
                } else {
                    warn!(
                        "CompositeArsenalPort: tool '{}' from member #{index} is a duplicate \
                         of an earlier member's registration -- the earlier definition wins \
                         and this one is unreachable",
                        armament.name
                    );
                }
            }
        }

        union
    }

    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
        for member in &self.members {
            match member.validate_call(&call) {
                Err(ArsenalError::ToolNotFound(_)) => continue,
                Err(e) => return Err(e),
                Ok(()) => return member.invoke(call).await,
            }
        }
        Err(ArsenalError::ToolNotFound(call.tool_name.clone()))
    }

    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
        for member in &self.members {
            match member.validate_call(call) {
                Err(ArsenalError::ToolNotFound(_)) => continue,
                other => return other,
            }
        }
        Err(ArsenalError::ToolNotFound(call.tool_name.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::arsenal::in_process_arsenal::InProcessArsenal;
    use serde_json::json;
    use std::collections::HashMap;

    fn search_armament(description: &str) -> Armament {
        Armament {
            name: "search".to_string(),
            description: description.to_string(),
            parameters: json!({"type": "object"}),
            required_params: vec![],
        }
    }

    fn only_armament(name: &str) -> Armament {
        Armament {
            name: name.to_string(),
            description: format!("{name} description"),
            parameters: json!({"type": "object"}),
            required_params: vec![],
        }
    }

    #[tokio::test]
    async fn composite_unions_list_armaments_first_registration_wins() {
        let first: Arc<dyn ArsenalPort> = Arc::new(
            InProcessArsenal::new()
                .with_tool(search_armament("first arsenal's search"), |_| async move {
                    Ok(json!("first"))
                }),
        );
        let second: Arc<dyn ArsenalPort> = Arc::new(
            InProcessArsenal::new()
                .with_tool(search_armament("second arsenal's search"), |_| async move {
                    Ok(json!("second"))
                }),
        );

        let composite = CompositeArsenalPort::new(vec![first, second]);
        let listed = composite.list_armaments().await;

        assert_eq!(
            listed.len(),
            1,
            "the duplicate name must union to exactly one entry"
        );
        assert_eq!(
            listed[0].description, "first arsenal's search",
            "the FIRST registration wins"
        );
    }

    #[tokio::test]
    async fn composite_routes_invoke_and_validate_by_name() {
        let first: Arc<dyn ArsenalPort> = Arc::new(
            InProcessArsenal::new()
                .with_tool(search_armament("first"), |_| async move {
                    Ok(json!("from first"))
                })
                .with_tool(only_armament("only_in_first"), |_| async move {
                    Ok(json!("only-first"))
                }),
        );
        let second: Arc<dyn ArsenalPort> = Arc::new(
            InProcessArsenal::new()
                .with_tool(search_armament("second"), |_| async move {
                    Ok(json!("from second"))
                })
                .with_tool(only_armament("only_in_second"), |_| async move {
                    Ok(json!("only-second"))
                }),
        );

        let composite = CompositeArsenalPort::new(vec![first, second]);

        // Shared name -> reaches the FIRST arsenal.
        let shared_result = composite
            .invoke(ArmamentCall::new("search", HashMap::new()))
            .await
            .expect("shared-name invoke should succeed");
        assert_eq!(shared_result.output, Some(json!("from first")));

        // A name only the SECOND arsenal declares -> reaches the second.
        let second_only_result = composite
            .invoke(ArmamentCall::new("only_in_second", HashMap::new()))
            .await
            .expect("second-only invoke should succeed");
        assert_eq!(second_only_result.output, Some(json!("only-second")));

        assert!(
            composite
                .validate_call(&ArmamentCall::new("only_in_second", HashMap::new()))
                .is_ok()
        );
    }

    #[tokio::test]
    async fn composite_of_zero_arsenals_lists_nothing_and_denies_everything() {
        let composite = CompositeArsenalPort::new(vec![]);

        assert!(composite.list_armaments().await.is_empty());

        let result = composite
            .invoke(ArmamentCall::new("anything", HashMap::new()))
            .await;
        assert!(matches!(result, Err(ArsenalError::ToolNotFound(_))));

        let validation = composite.validate_call(&ArmamentCall::new("anything", HashMap::new()));
        assert!(matches!(validation, Err(ArsenalError::ToolNotFound(_))));
    }
}
