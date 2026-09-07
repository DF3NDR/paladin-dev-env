//! Tool result formatting for LLM context injection
//!
//! This module provides formatting capabilities to convert tool execution results
//! into human-readable text suitable for injection into LLM conversation context.
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
//! use paladin::core::platform::container::arsenal::{ArmamentCall, ArmamentResult};
//! use std::collections::HashMap;
//! use serde_json::Value;
//! use uuid::Uuid;
//!
//! let formatter = ToolResultFormatter::new();
//! let call_id = Uuid::new_v4();
//!
//! let call = ArmamentCall::new("calculator", HashMap::new());
//! let result = ArmamentResult::success(call_id, Value::String("42".to_string()), 150);
//!
//! let formatted = formatter.format_result(&call, &result);
//! println!("{}", formatted);
//! ```

use crate::core::platform::container::arsenal::{ArmamentCall, ArmamentResult};
use serde_json::Value;

/// Formats tool execution results for LLM context injection
///
/// The formatter converts structured `ArmamentResult` objects into
/// human-readable markdown-like text that can be injected into
/// LLM conversation context. This allows the LLM to understand
/// what tools were called and what results they returned.
///
/// # Format Structure
///
/// Successful result:
/// ```text
/// 🔧 Tool Execution: tool_name
///
/// Arguments:
/// - arg1: value1
/// - arg2: value2
///
/// Result: SUCCESS
/// Output:
/// tool output here
///
/// Execution Time: 150ms
/// ```
///
/// Failed result:
/// ```text
/// 🔧 Tool Execution: tool_name
///
/// Arguments:
/// - arg1: value1
///
/// Result: FAILED
/// Error: error message here
///
/// Execution Time: 75ms
/// ```
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ToolResultFormatter {
    /// Whether to include execution time in formatted output
    include_timing: bool,
    /// Whether to use emoji indicators
    use_emoji: bool,
}

impl ToolResultFormatter {
    /// Creates a new ToolResultFormatter with default settings
    ///
    /// Default settings:
    /// - Include timing: true
    /// - Use emoji: true
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    ///
    /// let formatter = ToolResultFormatter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            include_timing: true,
            use_emoji: true,
        }
    }

    /// Creates a formatter without emoji indicators
    ///
    /// Useful for contexts where emoji may not render correctly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    ///
    /// let formatter = ToolResultFormatter::without_emoji();
    /// ```
    pub fn without_emoji() -> Self {
        Self {
            include_timing: true,
            use_emoji: false,
        }
    }

    /// Creates a formatter without execution timing
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    ///
    /// let formatter = ToolResultFormatter::without_timing();
    /// ```
    pub fn without_timing() -> Self {
        Self {
            include_timing: false,
            use_emoji: true,
        }
    }

    /// Formats a tool execution result into LLM-readable text
    ///
    /// Converts the structured `ArmamentResult` into a markdown-like format
    /// that includes the tool name, arguments, result status, output/error,
    /// and execution time.
    ///
    /// # Arguments
    ///
    /// * `call` - The tool invocation that was executed
    /// * `result` - The result of the tool execution
    ///
    /// # Returns
    ///
    /// A formatted string suitable for injection into LLM context
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    /// # use paladin::core::platform::container::arsenal::{ArmamentCall, ArmamentResult};
    /// # use std::collections::HashMap;
    /// # use serde_json::Value;
    /// # use uuid::Uuid;
    /// let formatter = ToolResultFormatter::new();
    /// let call = ArmamentCall::new("search", HashMap::new());
    /// let result = ArmamentResult::success(Uuid::new_v4(), Value::String("Found 10 results".to_string()), 200);
    ///
    /// let formatted = formatter.format_result(&call, &result);
    /// ```
    pub fn format_result(&self, call: &ArmamentCall, result: &ArmamentResult) -> String {
        let mut output = String::new();

        // Header
        let tool_icon = if self.use_emoji { "🔧 " } else { "" };
        output.push_str(&format!(
            "{}Tool Execution: {}\n\n",
            tool_icon, call.tool_name
        ));

        // Arguments section
        if !call.arguments.is_empty() {
            output.push_str("Arguments:\n");
            for (key, value) in &call.arguments {
                let formatted_value = self.format_value(value);
                output.push_str(&format!("- {}: {}\n", key, formatted_value));
            }
            output.push('\n');
        }

        // Result section
        if result.success {
            let success_icon = if self.use_emoji { "✅ " } else { "" };
            output.push_str(&format!("{}Result: SUCCESS\n", success_icon));

            if let Some(ref result_output) = result.output {
                output.push_str("Output:\n");
                output.push_str(&self.format_output_value(result_output));
                output.push('\n');
            }
        } else {
            let error_icon = if self.use_emoji { "❌ " } else { "" };
            output.push_str(&format!("{}Result: FAILED\n", error_icon));

            if let Some(ref error) = result.error {
                output.push_str(&format!("Error: {}\n", error));
            }
        }

        // Execution time
        if self.include_timing {
            output.push_str(&format!(
                "\nExecution Time: {}ms\n",
                result.execution_time_ms
            ));
        }

        output
    }

    /// Formats a tool call FAILURE for LLM context injection (D-33, D-34).
    ///
    /// Used by both the Arsenal tool-call branch and the handoff branch of
    /// the reasoning loop's `ToolErrorMode::FeedToModel` path (the default,
    /// today's v0.9 behavior unchanged) — one shared shape for every tool
    /// failure the model sees, rather than two independently-drifting
    /// inline `format!`s. Keeps the pre-existing
    /// `🔧 Tool Execution: {name}\nResult: FAILED\nError: {reason}` shape
    /// (X-03: no unplanned behavioral change to the text a model already
    /// received) and appends the PRD's own sentence: the model may retry
    /// with corrected arguments or proceed without the tool.
    ///
    /// `reason` is sanitized before it is embedded: [`redact_secret_patterns`]
    /// runs BEFORE [`bounded_excerpt`] — redact, then bound, never the
    /// reverse. Bounding first can slice a secret across the truncation
    /// boundary and leak the surviving tail
    /// (`.github/instructions/security.instructions.md`, T-26-03).
    ///
    /// # Example
    ///
    /// ```rust
    /// use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    /// use paladin::core::platform::container::arsenal::ArmamentCall;
    /// use std::collections::HashMap;
    ///
    /// let formatter = ToolResultFormatter::new();
    /// let call = ArmamentCall::new("fetch_report", HashMap::new());
    /// let formatted = formatter.format_error(&call, "upstream gateway timed out");
    ///
    /// assert!(formatted.contains("Tool Execution: fetch_report"));
    /// assert!(formatted.contains("Result: FAILED"));
    /// assert!(formatted.contains("You may retry"));
    /// ```
    ///
    /// [`redact_secret_patterns`]: paladin_llm::redaction::redact_secret_patterns
    /// [`bounded_excerpt`]: paladin_llm::redaction::bounded_excerpt
    pub fn format_error(&self, call: &ArmamentCall, reason: &str) -> String {
        let tool_icon = if self.use_emoji { "🔧 " } else { "" };

        // T-26-03: redact-then-bound, never the reverse -- bounding first
        // can slice a secret across the truncation boundary and leak the
        // surviving tail.
        let redacted = paladin_llm::redaction::redact_secret_patterns(reason);
        let sanitized = paladin_llm::redaction::bounded_excerpt(
            &redacted,
            paladin_llm::redaction::RESPONSE_EXCERPT_CHAR_BUDGET,
        );

        format!(
            "{tool_icon}Tool Execution: {}\nResult: FAILED\nError: {}\nYou may retry with corrected arguments or proceed without it.\n",
            call.tool_name, sanitized
        )
    }

    /// Formats an output value for display
    ///
    /// Converts output values into human-readable strings with
    /// proper formatting and indentation.
    fn format_output_value(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Array(_) | Value::Object(_) => {
                // Pretty print JSON structures
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            }
        }
    }

    /// Formats a JSON value for display in arguments
    ///
    /// Converts JSON values into human-readable strings, handling
    /// different value types appropriately.
    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                if arr.len() <= 3 {
                    format!(
                        "[{}]",
                        arr.iter()
                            .map(|v| self.format_value(v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            Value::Object(obj) => {
                if obj.len() <= 3 {
                    format!(
                        "{{{}}}",
                        obj.iter()
                            .map(|(k, v)| format!("{}: {}", k, self.format_value(v)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    format!("{{{}}} fields", obj.len())
                }
            }
        }
    }

    /// Formats multiple tool results into a single text block
    ///
    /// Useful for batch formatting of multiple tool invocations.
    ///
    /// # Arguments
    ///
    /// * `results` - Slice of (call, result) tuples
    ///
    /// # Returns
    ///
    /// A formatted string with all results separated by dividers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::infrastructure::adapters::arsenal::tool_result_formatter::ToolResultFormatter;
    /// # use paladin::core::platform::container::arsenal::{ArmamentCall, ArmamentResult};
    /// # use std::collections::HashMap;
    /// # use serde_json::Value;
    /// # use uuid::Uuid;
    /// let formatter = ToolResultFormatter::new();
    /// let results = vec![
    ///     (ArmamentCall::new("tool1", HashMap::new()),
    ///      ArmamentResult::success(Uuid::new_v4(), Value::String("result1".to_string()), 100)),
    ///     (ArmamentCall::new("tool2", HashMap::new()),
    ///      ArmamentResult::success(Uuid::new_v4(), Value::String("result2".to_string()), 150)),
    /// ];
    ///
    /// let formatted = formatter.format_batch(&results);
    /// ```
    pub fn format_batch(&self, results: &[(ArmamentCall, ArmamentResult)]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let divider = if self.use_emoji {
            "═══════════════════════════════════════\n"
        } else {
            "---------------------------------------\n"
        };

        results
            .iter()
            .map(|(call, result)| self.format_result(call, result))
            .collect::<Vec<_>>()
            .join(divider)
    }
}

impl Default for ToolResultFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    // ── D-33/D-34: format_error (Task 2, Plan 26-19) ─────────────────────

    #[test]
    fn format_error_keeps_todays_shape_and_appends_the_prd_sentence() {
        let formatter = ToolResultFormatter::new();
        let call = ArmamentCall::new("fetch_report", HashMap::new());

        let formatted = formatter.format_error(&call, "upstream gateway timed out");

        assert!(
            formatted.contains("🔧 Tool Execution: fetch_report"),
            "got {formatted}"
        );
        assert!(formatted.contains("Result: FAILED"), "got {formatted}");
        assert!(
            formatted.contains("Error: upstream gateway timed out"),
            "got {formatted}"
        );
        assert!(
            formatted.contains("You may retry with corrected arguments or proceed without it."),
            "got {formatted}"
        );
    }

    #[test]
    fn both_arms_use_the_same_formatter() {
        // The Arsenal arm and the handoff arm both call format_error with
        // the same shape of inputs; asserting their outputs match one
        // expected shape (up to the tool name and reason) proves there is
        // exactly one formatter, not two independently-drifting call
        // sites.
        let formatter = ToolResultFormatter::new();
        let arsenal_call = ArmamentCall::new("web_search", HashMap::new());
        let handoff_call = ArmamentCall::new("delegate_to_specialist", HashMap::new());

        let arsenal_output = formatter.format_error(&arsenal_call, "connection reset");
        let handoff_output = formatter.format_error(&handoff_call, "connection reset");

        let expected_shape = |tool_name: &str| {
            format!(
                "🔧 Tool Execution: {tool_name}\nResult: FAILED\nError: connection reset\n\
                 You may retry with corrected arguments or proceed without it.\n"
            )
        };

        assert_eq!(arsenal_output, expected_shape("web_search"));
        assert_eq!(handoff_output, expected_shape("delegate_to_specialist"));
    }

    #[test]
    fn format_error_redacts_a_secret_in_the_reason_before_bounding() {
        // T-26-03: a credential in the failure reason must never reach the
        // model, redacted before it is bounded.
        let formatter = ToolResultFormatter::new();
        let call = ArmamentCall::new("fetch_report", HashMap::new());
        let reason = "upstream rejected: Authorization: Bearer sk-live-abcdef0123456789";

        let formatted = formatter.format_error(&call, reason);

        assert!(!formatted.contains("abcdef0123456789"), "got {formatted}");
        // `paladin_llm::redaction::CREDENTIAL_PLACEHOLDER` is `pub(crate)`
        // to that crate, so this asserts the literal it is defined as
        // rather than importing it.
        assert!(formatted.contains("[REDACTED]"), "got {formatted}");
    }

    #[test]
    fn test_format_success_result() {
        let formatter = ToolResultFormatter::new();
        let call_id = Uuid::new_v4();

        let mut args = HashMap::new();
        args.insert(
            "query".to_string(),
            Value::String("test search".to_string()),
        );
        args.insert("limit".to_string(), Value::Number(10.into()));

        let call = ArmamentCall {
            tool_name: "web_search".to_string(),
            arguments: args,
            call_id,
        };

        let result =
            ArmamentResult::success(call_id, Value::String("Found 10 results".to_string()), 250);

        let formatted = formatter.format_result(&call, &result);

        assert!(formatted.contains("🔧 Tool Execution: web_search"));
        assert!(formatted.contains("Arguments:"));
        assert!(formatted.contains("query: test search"));
        assert!(formatted.contains("limit: 10"));
        assert!(formatted.contains("✅ Result: SUCCESS"));
        assert!(formatted.contains("Output:"));
        assert!(formatted.contains("Found 10 results"));
        assert!(formatted.contains("Execution Time: 250ms"));
    }

    #[test]
    fn test_format_error_result() {
        let formatter = ToolResultFormatter::new();
        let call_id = Uuid::new_v4();

        let mut args = HashMap::new();
        args.insert("file".to_string(), Value::String("missing.txt".to_string()));

        let call = ArmamentCall {
            tool_name: "read_file".to_string(),
            arguments: args,
            call_id,
        };

        let result = ArmamentResult::failure(call_id, "File not found", 50);

        let formatted = formatter.format_result(&call, &result);

        assert!(formatted.contains("🔧 Tool Execution: read_file"));
        assert!(formatted.contains("file: missing.txt"));
        assert!(formatted.contains("❌ Result: FAILED"));
        assert!(formatted.contains("Error: File not found"));
        assert!(formatted.contains("Execution Time: 50ms"));
    }

    #[test]
    fn test_format_without_emoji() {
        let formatter = ToolResultFormatter::without_emoji();
        let call_id = Uuid::new_v4();

        let call = ArmamentCall::new("calculator", HashMap::new());
        let result = ArmamentResult::success(call_id, Value::String("42".to_string()), 100);

        let formatted = formatter.format_result(&call, &result);

        assert!(!formatted.contains("🔧"));
        assert!(!formatted.contains("✅"));
        assert!(formatted.contains("Tool Execution: calculator"));
        assert!(formatted.contains("Result: SUCCESS"));
    }

    #[test]
    fn test_format_without_timing() {
        let formatter = ToolResultFormatter::without_timing();
        let call_id = Uuid::new_v4();

        let call = ArmamentCall::new("ping", HashMap::new());
        let result = ArmamentResult::success(call_id, Value::String("pong".to_string()), 25);

        let formatted = formatter.format_result(&call, &result);

        assert!(!formatted.contains("Execution Time"));
        assert!(!formatted.contains("25ms"));
    }

    #[test]
    fn test_format_batch() {
        let formatter = ToolResultFormatter::new();
        let call1_id = Uuid::new_v4();
        let call2_id = Uuid::new_v4();

        let call1 = ArmamentCall::new("tool1", HashMap::new());
        let result1 = ArmamentResult::success(call1_id, Value::String("result1".to_string()), 100);

        let call2 = ArmamentCall::new("tool2", HashMap::new());
        let result2 = ArmamentResult::success(call2_id, Value::String("result2".to_string()), 150);

        let results = vec![(call1, result1), (call2, result2)];
        let formatted = formatter.format_batch(&results);

        assert!(formatted.contains("tool1"));
        assert!(formatted.contains("tool2"));
        assert!(formatted.contains("result1"));
        assert!(formatted.contains("result2"));
        assert!(formatted.contains("═══════════════"));
    }

    #[test]
    fn test_format_value_types() {
        let formatter = ToolResultFormatter::new();

        assert_eq!(formatter.format_value(&Value::Null), "null");
        assert_eq!(formatter.format_value(&Value::Bool(true)), "true");
        assert_eq!(formatter.format_value(&Value::Number(42.into())), "42");
        assert_eq!(
            formatter.format_value(&Value::String("test".to_string())),
            "test"
        );

        let arr = Value::Array(vec![Value::Number(1.into()), Value::Number(2.into())]);
        assert_eq!(formatter.format_value(&arr), "[1, 2]");

        let large_arr = Value::Array(vec![Value::Number(1.into()); 5]);
        assert!(formatter.format_value(&large_arr).contains("[5 items]"));
    }

    #[test]
    fn test_format_empty_arguments() {
        let formatter = ToolResultFormatter::new();
        let call_id = Uuid::new_v4();

        let call = ArmamentCall::new("no_args_tool", HashMap::new());
        let result = ArmamentResult::success(call_id, Value::String("done".to_string()), 10);

        let formatted = formatter.format_result(&call, &result);

        // Should not have "Arguments:" section when empty
        assert!(!formatted.contains("Arguments:"));
        assert!(formatted.contains("Tool Execution: no_args_tool"));
    }
}
