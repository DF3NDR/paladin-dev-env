//! Configuration for agents served by the HTTP service host (Milestone 12, Epic 2).
//!
//! The top-level `agents:` key in `config.yml` is a list of [`AgentDefinition`]s. The
//! `paladin-server` binary turns each definition into a resident agent in the
//! `paladin_web::AgentRegistry` (see the facade `infrastructure::web` builder).
//!
//! Secrets (API keys) are **never** read from these definitions — they come from the
//! `llm:` provider configuration and the corresponding environment variables.

use serde::{Deserialize, Serialize};

/// Declarative definition of one agent to load into the HTTP service host.
///
/// `id`, `model`, and `system_prompt` are required; everything else is optional and
/// falls back to a provider/builder default. Optional fields use `#[serde(default)]`
/// so new fields can be added without breaking existing configs.
///
/// # Example (YAML)
///
/// ```yaml
/// agents:
///   - id: "researcher"
///     provider: "openai"      # optional; defaults to llm.default_provider
///     model: "gpt-4"
///     system_prompt: "You research topics thoroughly."
///     temperature: 0.7
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique registry id — the `{id}` path segment in `/agents/{id}/…`.
    pub id: String,

    /// LLM model identifier (e.g. `"gpt-4"`).
    pub model: String,

    /// System prompt defining the agent's behavior.
    pub system_prompt: String,

    /// Provider name (e.g. `"openai"`, `"anthropic"`, `"deepseek"`).
    ///
    /// When absent, the server falls back to `llm.default_provider`.
    #[serde(default)]
    pub provider: Option<String>,

    /// Response randomness (`0.0`–`1.0`). When absent, the builder default applies.
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Maximum reasoning loops. When absent, the builder default applies.
    #[serde(default)]
    pub max_loops: Option<u32>,

    /// Tokens that signal the agent to stop processing.
    #[serde(default)]
    pub stop_words: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_full_definition() {
        let json = serde_json::json!({
            "id": "researcher",
            "model": "gpt-4",
            "system_prompt": "You research topics.",
            "provider": "openai",
            "temperature": 0.7,
            "max_loops": 5,
            "stop_words": ["STOP"]
        });
        let def: AgentDefinition = serde_json::from_value(json).expect("parses");
        assert_eq!(def.id, "researcher");
        assert_eq!(def.model, "gpt-4");
        assert_eq!(def.system_prompt, "You research topics.");
        assert_eq!(def.provider.as_deref(), Some("openai"));
        assert_eq!(def.temperature, Some(0.7));
        assert_eq!(def.max_loops, Some(5));
        assert_eq!(def.stop_words, vec!["STOP".to_string()]);
    }

    #[test]
    fn deserializes_minimal_definition_with_defaults() {
        // Only the three required fields are present.
        let json = serde_json::json!({
            "id": "summarizer",
            "model": "gpt-4",
            "system_prompt": "You summarize."
        });
        let def: AgentDefinition = serde_json::from_value(json).expect("parses");
        assert_eq!(def.id, "summarizer");
        assert!(def.provider.is_none());
        assert!(def.temperature.is_none());
        assert!(def.max_loops.is_none());
        assert!(def.stop_words.is_empty());
    }

    #[test]
    fn missing_required_field_fails() {
        // No `system_prompt` → must not deserialize.
        let json = serde_json::json!({ "id": "x", "model": "gpt-4" });
        let result: Result<AgentDefinition, _> = serde_json::from_value(json);
        assert!(result.is_err(), "missing required field must fail");
    }
}
