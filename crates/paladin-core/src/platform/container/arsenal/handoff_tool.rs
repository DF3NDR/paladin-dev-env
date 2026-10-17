use crate::platform::container::arsenal::handoff_error::HandoffError;
use crate::platform::container::paladin::Paladin;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool for mid-execution handoff to specialist agents.
///
/// This tool allows a Paladin to delegate tasks to specialist agents
/// during execution. The tool schema dynamically includes available
/// specialist agent names as an enum for the agent_name parameter.
///
/// # Example
///
/// ```rust,no_run
/// use paladin_core::platform::container::arsenal::handoff_tool::HandoffTool;
/// use std::sync::Arc;
///
/// // Create tool with no specialists (simplified example)
/// let tool = HandoffTool::new(vec![]);
/// let schema = tool.get_schema();
/// assert_eq!(schema["type"], "function");
/// ```
#[derive(Debug, Clone)]
pub struct HandoffTool {
    /// Available specialist agents
    specialists: Vec<Arc<Paladin>>,
}

impl HandoffTool {
    /// Create a new handoff tool with available specialists.
    ///
    /// # Arguments
    ///
    /// * `specialists` - Vector of specialist Paladin agents available for handoff
    ///
    /// # Returns
    ///
    /// A new `HandoffTool` instance
    pub fn new(specialists: Vec<Arc<Paladin>>) -> Self {
        Self { specialists }
    }

    /// Get the tool schema with dynamic agent_name enum.
    ///
    /// The schema includes an enum of available specialist agent names,
    /// allowing the LLM to choose from valid options.
    ///
    /// # Returns
    ///
    /// JSON schema for the handoff tool
    pub fn get_schema(&self) -> Value {
        // Extract agent names for enum
        let agent_names: Vec<Value> = self
            .specialists
            .iter()
            .map(|p| json!(p.data.name.clone()))
            .collect();

        json!({
            "type": "function",
            "function": {
                "name": "handoff_to_agent",
                "description": "Delegate the current task to a specialist agent",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "agent_name": {
                            "type": "string",
                            "description": "Name of the specialist agent to delegate to",
                            "enum": agent_names
                        },
                        "message": {
                            "type": "string",
                            "description": "Task description and context to pass to the specialist agent"
                        }
                    },
                    "required": ["agent_name", "message"]
                }
            }
        })
    }

    /// Validate tool parameters.
    ///
    /// Checks that:
    /// - `agent_name` parameter exists and is a valid specialist
    /// - `message` parameter exists and is non-empty
    ///
    /// # Arguments
    ///
    /// * `params` - Map of parameter names to values
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Parameters are valid
    /// * `Err(HandoffError)` - Validation failed with specific error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin_core::platform::container::arsenal::handoff_tool::HandoffTool;
    /// # use std::collections::HashMap;
    /// # use serde_json::json;
    /// # let tool = HandoffTool::new(vec![]);
    /// let mut params = HashMap::new();
    /// params.insert("agent_name".to_string(), json!("code_expert"));
    /// params.insert("message".to_string(), json!("Analyze this code"));
    ///
    /// tool.validate_parameters(&params).unwrap();
    /// ```
    pub fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), HandoffError> {
        // Check agent_name exists
        let agent_name = params
            .get("agent_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HandoffError::ConfigurationError(
                    "Missing or invalid agent_name parameter".to_string(),
                )
            })?;

        // Check agent exists in specialists
        let agent_exists = self.specialists.iter().any(|p| p.data.name == agent_name);
        if !agent_exists {
            return Err(HandoffError::InvalidAgent {
                agent_name: agent_name.to_string(),
            });
        }

        // Check message exists and is non-empty
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HandoffError::ConfigurationError("Missing or invalid message parameter".to_string())
            })?;

        if message.trim().is_empty() {
            return Err(HandoffError::ConfigurationError(
                "Message parameter cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Get the list of available specialist agents.
    ///
    /// # Returns
    ///
    /// Reference to the vector of specialist agents
    pub fn specialists(&self) -> &Vec<Arc<Paladin>> {
        &self.specialists
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_handoff_tool() {
        let tool = HandoffTool::new(vec![]);
        assert_eq!(tool.specialists().len(), 0);
    }

    #[test]
    fn test_specialists_accessor() {
        let tool = HandoffTool::new(vec![]);
        let specialists = tool.specialists();
        assert_eq!(specialists.len(), 0);
    }
}
