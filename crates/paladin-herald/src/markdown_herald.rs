//! Markdown Herald formatter implementation
//!
//! The MarkdownHerald provides Markdown formatting for Paladin and Battalion execution results.
//! It supports headings, status badges, code blocks, and optional ANSI color output for terminals.
//!
//! # Examples
//!
//! ```rust,ignore
//! use paladin_herald::MarkdownHerald;
//! use paladin::core::platform::container::herald::Herald;
//!
//! let herald = MarkdownHerald::new();
//! let formatted = herald.format_paladin_result(&result)?;
//! println!("{}", formatted);
//! ```

#[cfg(feature = "color")]
use colored::*;
use paladin_core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, PaladinError, PaladinResult,
    StreamChunk,
};
use serde::{Deserialize, Serialize};

/// Configuration for Markdown Herald formatter
#[doc(hidden)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownHeraldConfig {
    /// Enable ANSI color codes for terminal output
    pub include_colors: bool,
    /// Heading level for main sections (1-6)
    pub heading_level: u8,
}

impl Default for MarkdownHeraldConfig {
    fn default() -> Self {
        Self {
            include_colors: Self::default_include_colors(),
            heading_level: 2,
        }
    }
}

impl MarkdownHeraldConfig {
    /// Detect if the terminal supports color output
    ///
    /// Returns true if:
    /// - TERM environment variable is set and not "dumb"
    /// - NO_COLOR environment variable is not set
    /// - Running in a TTY
    pub fn supports_color() -> bool {
        // Check NO_COLOR environment variable (universal way to disable colors)
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        // Check TERM environment variable
        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
            // Common color-supporting terminals
            if term.contains("color")
                || term.contains("xterm")
                || term.contains("screen")
                || term.contains("tmux")
            {
                return true;
            }
        }

        // Default to no colors if we can't detect
        false
    }

    /// Default `include_colors` value: auto-detected terminal support when the `color`
    /// feature is enabled, `false` (the existing plain-text path) when it is not.
    #[cfg(feature = "color")]
    fn default_include_colors() -> bool {
        Self::supports_color()
    }

    /// Default `include_colors` value when the `color` feature is not enabled.
    #[cfg(not(feature = "color"))]
    fn default_include_colors() -> bool {
        false
    }
}

/// Markdown formatter for Paladin execution results
///
/// The MarkdownHerald converts Paladin and Battalion results into Markdown format,
/// making them suitable for documentation, CLI output, and human-readable displays.
///
/// # Thread Safety
///
/// MarkdownHerald is thread-safe and can be shared across threads using `Arc`.
///
/// # Configuration
///
/// - **include_colors**: Enable ANSI color codes for terminal output
/// - **heading_level**: Markdown heading level (1-6) for main sections
///
/// # Examples
///
/// ```rust,ignore
/// // Create with default configuration (auto-detect colors, heading level 2)
/// let herald = MarkdownHerald::new();
///
/// // Create with custom configuration
/// let config = MarkdownHeraldConfig {
///     include_colors: false,
///     heading_level: 3,
/// };
/// let herald = MarkdownHerald::with_config(config);
/// ```
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct MarkdownHerald {
    config: MarkdownHeraldConfig,
}

impl MarkdownHerald {
    /// Create a new MarkdownHerald with default configuration
    ///
    /// Default configuration:
    /// - include_colors: auto-detected based on terminal support
    /// - heading_level: 2
    pub fn new() -> Self {
        Self {
            config: MarkdownHeraldConfig::default(),
        }
    }

    /// Create a new MarkdownHerald with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the Markdown formatter
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = MarkdownHeraldConfig {
    ///     include_colors: true,
    ///     heading_level: 1,
    /// };
    /// let herald = MarkdownHerald::with_config(config);
    /// ```
    pub fn with_config(config: MarkdownHeraldConfig) -> Self {
        Self { config }
    }

    /// Generate heading markup
    fn heading(&self, level: u8, text: &str) -> String {
        let level = level.clamp(1, 6);
        format!("{} {}\n\n", "#".repeat(level as usize), text)
    }

    /// Generate status badge with emoji
    #[cfg(feature = "color")]
    fn status_badge(&self, status: &str) -> String {
        let (emoji, color_fn): (&str, fn(&str) -> ColoredString) =
            match status.to_lowercase().as_str() {
                "success" | "completed" | "ok" => ("✅", |s| s.green()),
                "failed" | "error" => ("❌", |s| s.red()),
                "timeout" | "warning" => ("⏱️", |s| s.yellow()),
                "running" | "in_progress" => ("🔄", |s| s.blue()),
                "pending" | "queued" => ("⏳", |s| s.cyan()),
                _ => ("ℹ️", |s| s.normal()),
            };

        if self.config.include_colors {
            format!("{} {}", emoji, color_fn(status))
        } else {
            format!("{} {}", emoji, status)
        }
    }

    /// Generate status badge with emoji (plain-text path; `color` feature not enabled)
    #[cfg(not(feature = "color"))]
    fn status_badge(&self, status: &str) -> String {
        let emoji = match status.to_lowercase().as_str() {
            "success" | "completed" | "ok" => "✅",
            "failed" | "error" => "❌",
            "timeout" | "warning" => "⏱️",
            "running" | "in_progress" => "🔄",
            "pending" | "queued" => "⏳",
            _ => "ℹ️",
        };
        format!("{} {}", emoji, status)
    }

    /// Format a key-value pair as bold key
    #[cfg(feature = "color")]
    fn format_field(&self, key: &str, value: &str) -> String {
        if self.config.include_colors {
            format!("**{}:** {}\n", key.bold(), value)
        } else {
            format!("**{}:** {}\n", key, value)
        }
    }

    /// Format a key-value pair as bold key (plain-text path; `color` feature not enabled)
    #[cfg(not(feature = "color"))]
    fn format_field(&self, key: &str, value: &str) -> String {
        format!("**{}:** {}\n", key, value)
    }

    /// Format code block
    fn code_block(&self, content: &str, language: &str) -> String {
        format!("```{}\n{}\n```\n\n", language, content)
    }

    /// Render the "Error" heading used by [`Herald::format_error`]
    #[cfg(feature = "color")]
    fn error_heading(&self) -> String {
        if self.config.include_colors {
            format!("**{}**\n\n", "Error".red().bold())
        } else {
            "**Error**\n\n".to_string()
        }
    }

    /// Render the "Error" heading (plain-text path; `color` feature not enabled)
    #[cfg(not(feature = "color"))]
    fn error_heading(&self) -> String {
        "**Error**\n\n".to_string()
    }
}

impl Default for MarkdownHerald {
    fn default() -> Self {
        Self::new()
    }
}

impl Herald for MarkdownHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let mut output = String::new();

        // Main heading
        output.push_str(&self.heading(self.config.heading_level, "Paladin Result"));

        // Status badge
        let status_str = format!("{:?}", result.stop_reason);
        output.push_str(&self.status_badge(&status_str));
        output.push_str("\n\n");

        // Output section
        output.push_str(&self.heading(self.config.heading_level + 1, "Output"));
        output.push_str(&result.output);
        output.push_str("\n\n");

        // Metadata section
        output.push_str(&self.heading(self.config.heading_level + 1, "Metadata"));
        output.push_str(&self.format_field("Token Count", &result.token_count.to_string()));
        output.push_str(
            &self.format_field("Execution Time (ms)", &result.execution_time_ms.to_string()),
        );
        output.push_str(&self.format_field("Loop Count", &result.loop_count.to_string()));
        output.push_str(&self.format_field("Stop Reason", &format!("{:?}", result.stop_reason)));
        output.push_str(&self.format_field("Timestamp", &chrono::Utc::now().to_rfc3339()));

        Ok(output)
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        let mut output = String::new();

        // Main heading
        output.push_str(&self.heading(
            self.config.heading_level,
            &format!("Battalion: {}", result.battalion_name),
        ));

        // Status badge
        let status_str = format!("{:?}", result.status);
        output.push_str(&self.status_badge(&status_str));
        output.push_str("\n\n");

        // Summary
        output.push_str(&self.format_field("Battalion ID", &result.battalion_id.to_string()));
        output.push_str(&self.format_field("Strategy", &format!("{:?}", result.strategy_used)));
        output.push_str(
            &self.format_field("Total Paladins", &result.paladin_results.len().to_string()),
        );
        output.push_str(
            &self.format_field("Success Count", &result.paladin_success_count.to_string()),
        );
        output.push_str(
            &self.format_field("Failure Count", &result.paladin_failure_count.to_string()),
        );
        output.push_str(&self.format_field("Total Tokens", &result.total_tokens.to_string()));
        output.push('\n');

        // Individual Paladin results
        output.push_str(&self.heading(self.config.heading_level + 1, "Paladin Results"));

        for (idx, paladin_result) in result.paladin_results.iter().enumerate() {
            output.push_str(&self.heading(
                self.config.heading_level + 2,
                &format!("Paladin {}", idx + 1),
            ));
            let stop_str = format!("{:?}", paladin_result.stop_reason);
            output.push_str(&self.status_badge(&stop_str));
            output.push_str("\n\n");
            output.push_str(&paladin_result.output);
            output.push_str("\n\n");
        }

        // Failure detail, rendered only when there is something to report.
        if !result.node_errors.is_empty() {
            output.push_str(&self.heading(self.config.heading_level + 1, "Failures"));
            for node_error in &result.node_errors {
                output.push_str(&self.format_field(&node_error.node_name, &node_error.error));
            }
            output.push('\n');
        }

        Ok(output)
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        // For Markdown streaming, emit content progressively
        // No special formatting needed for chunks, just pass through
        Ok(Some(chunk.content.clone()))
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        let mut output = String::new();

        output.push_str("\n\n");
        output.push_str(&self.heading(self.config.heading_level + 1, "Execution Metadata"));
        output.push_str(&self.format_field("Model", &metadata.model_used));
        if let Some(duration) = metadata.duration_ms {
            output.push_str(&self.format_field("Duration", &format!("{}ms", duration)));
        }
        output.push_str(&self.format_field(
            "Total Tokens",
            &metadata.token_usage.total_tokens.to_string(),
        ));
        if let Some(cost) = metadata.cost_estimate {
            output.push_str(&self.format_field("Cost", &format!("${:.4}", cost)));
        }

        Ok(output)
    }

    fn format_error(&self, error: &PaladinError) -> String {
        let mut output = String::new();

        output.push_str(&self.error_heading());
        output.push_str(&self.code_block(&error.to_string(), ""));

        output
    }

    fn name(&self) -> &str {
        "markdown"
    }

    fn mime_type(&self) -> &str {
        "text/markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::battalion::{BattalionStatus, BattalionStrategy};
    use paladin_ports::output::paladin_port::StopReason;
    use uuid::Uuid;

    fn create_test_paladin_result() -> PaladinResult {
        PaladinResult {
            output: "Test output content".to_string(),
            token_count: 100,
            execution_time_ms: 1500,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        }
    }

    fn create_test_battalion_result() -> BattalionResult {
        BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "TestBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: "Combined output".to_string(),
            paladin_results: vec![
                create_test_paladin_result(),
                PaladinResult {
                    output: "Second output".to_string(),
                    token_count: 150,
                    execution_time_ms: 2000,
                    loop_count: 2,
                    stop_reason: StopReason::MaxLoops,
                    ..Default::default()
                },
            ],
            status: BattalionStatus::Completed,
            strategy_used: BattalionStrategy::Formation,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: [
                ("paladin_1".to_string(), 1500u64),
                ("paladin_2".to_string(), 2000u64),
            ]
            .into_iter()
            .collect(),
            per_paladin_tokens: std::collections::HashMap::new(),
            total_tokens: 0,
            paladin_success_count: 1,
            paladin_failure_count: 1,
            node_errors: Vec::new(),
        }
    }

    #[test]
    fn test_new_creates_default_config() {
        let herald = MarkdownHerald::new();
        assert_eq!(herald.config.heading_level, 2);
    }

    #[test]
    fn test_with_config_uses_custom_config() {
        let config = MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 3,
        };
        let herald = MarkdownHerald::with_config(config);
        assert!(!herald.config.include_colors);
        assert_eq!(herald.config.heading_level, 3);
    }

    #[test]
    fn test_heading_generation() {
        let herald = MarkdownHerald::new();
        let heading = herald.heading(2, "Test Heading");
        assert_eq!(heading, "## Test Heading\n\n");
    }

    #[test]
    fn test_heading_clamps_level() {
        let herald = MarkdownHerald::new();
        let heading_too_low = herald.heading(0, "Test");
        let heading_too_high = herald.heading(10, "Test");
        assert!(heading_too_low.starts_with('#'));
        assert!(heading_too_high.starts_with("######"));
    }

    #[test]
    fn test_status_badge_success() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let badge = herald.status_badge("success");
        assert!(badge.contains("✅"));
        assert!(badge.contains("success"));
    }

    #[test]
    fn test_status_badge_failed() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let badge = herald.status_badge("failed");
        assert!(badge.contains("❌"));
        assert!(badge.contains("failed"));
    }

    #[test]
    fn test_status_badge_timeout() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let badge = herald.status_badge("timeout");
        assert!(badge.contains("⏱️"));
    }

    #[test]
    fn test_format_field() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let field = herald.format_field("Key", "Value");
        assert!(field.contains("**Key:**"));
        assert!(field.contains("Value"));
    }

    #[test]
    fn test_code_block() {
        let herald = MarkdownHerald::new();
        let code = herald.code_block("println!(\"test\");", "rust");
        assert!(code.starts_with("```rust"));
        assert!(code.contains("println!(\"test\");"));
        assert!(code.ends_with("```\n\n"));
    }

    #[test]
    fn test_format_paladin_result_structure() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let result = create_test_paladin_result();

        let formatted = herald.format_paladin_result(&result).unwrap();

        assert!(formatted.contains("## Paladin Result"));
        assert!(formatted.contains("✅")); // Success badge for Completed
        assert!(formatted.contains("### Output"));
        assert!(formatted.contains("Test output content"));
        assert!(formatted.contains("### Metadata"));
        assert!(formatted.contains("**Token Count:**"));
    }

    #[test]
    fn test_format_battalion_result_structure() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let result = create_test_battalion_result();

        let formatted = herald.format_battalion_result(&result).unwrap();

        assert!(formatted.contains("## Battalion: TestBattalion"));
        assert!(formatted.contains("### Paladin Results"));
        assert!(formatted.contains("#### Paladin 1")); // Heading for first result
        assert!(formatted.contains("#### Paladin 2")); // Heading for second result
        assert!(formatted.contains("✅")); // Success badge
        // Strategy and aggregate token fields, added alongside the pre-existing summary fields.
        assert!(formatted.contains("**Strategy:**"));
        assert!(formatted.contains("Formation"));
        assert!(formatted.contains("**Total Tokens:**"));
    }

    #[test]
    fn test_markdown_herald_battalion_includes_strategy_and_total_tokens() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });

        let mut per_paladin_tokens = std::collections::HashMap::new();
        per_paladin_tokens.insert(
            "Scout".to_string(),
            paladin_core::platform::container::battalion::TokenUsage::from_total(137),
        );
        per_paladin_tokens.insert(
            "Sentinel".to_string(),
            paladin_core::platform::container::battalion::TokenUsage::from_total(263),
        );

        let result = BattalionResult {
            battalion_id: Uuid::new_v4(),
            battalion_name: "TokenBattalion".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            final_output: "Combined output".to_string(),
            paladin_results: vec![
                PaladinResult {
                    output: "Scout output".to_string(),
                    token_count: 137,
                    execution_time_ms: 1500,
                    loop_count: 1,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                },
                PaladinResult {
                    output: "Sentinel output".to_string(),
                    token_count: 263,
                    execution_time_ms: 2000,
                    loop_count: 2,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                },
            ],
            status: BattalionStatus::Completed,
            strategy_used: paladin_core::platform::container::battalion::BattalionStrategy::Phalanx,
            strategy_selection_reasoning: None,
            strategy_selection_time_ms: 0,
            per_paladin_times: std::collections::HashMap::new(),
            per_paladin_tokens,
            total_tokens: 400,
            paladin_success_count: 2,
            paladin_failure_count: 1,
            node_errors: vec![paladin_core::platform::container::battalion::NodeError {
                node_name: "Herald".to_string(),
                error: "connection refused".to_string(),
            }],
        };

        let formatted = herald.format_battalion_result(&result).unwrap();

        assert!(formatted.contains("**Strategy:**"));
        assert!(formatted.contains("Phalanx"));
        assert!(formatted.contains("**Total Tokens:**"));
        assert!(formatted.contains("400"));

        // Failure detail: each node error's name and text rendered.
        assert!(formatted.contains("**Herald:**"));
        assert!(formatted.contains("connection refused"));

        // paladin_results order is preserved.
        let scout_pos = formatted.find("Scout output").unwrap();
        let sentinel_pos = formatted.find("Sentinel output").unwrap();
        assert!(scout_pos < sentinel_pos);
    }

    #[test]
    fn test_markdown_herald_battalion_no_failures_section_when_no_node_errors() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let result = create_test_battalion_result();
        assert!(result.node_errors.is_empty());

        let formatted = herald.format_battalion_result(&result).unwrap();

        assert!(!formatted.contains("### Failures"));
    }

    #[test]
    fn test_format_stream_chunk() {
        let herald = MarkdownHerald::new();
        let chunk = StreamChunk::builder()
            .chunk_id(uuid::Uuid::new_v4())
            .sequence_number(0)
            .timestamp(chrono::Utc::now())
            .content("Streaming content".to_string())
            .is_final(false)
            .build()
            .unwrap();

        let formatted = herald.format_stream_chunk(&chunk).unwrap();
        assert!(formatted.is_some());
        assert_eq!(formatted.unwrap(), "Streaming content");
    }

    #[test]
    fn test_finalize_stream() {
        use paladin_ports::output::llm_port::TokenUsage;
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let metadata = ExecutionMetadata::builder()
            .execution_id(uuid::Uuid::new_v4())
            .start_time(chrono::Utc::now())
            .model_used("gpt-4".to_string())
            .token_usage(TokenUsage::new(300, 200))
            .duration_ms(1234)
            .build()
            .unwrap();

        let formatted = herald.finalize_stream(&metadata).unwrap();
        assert!(formatted.contains("### Execution Metadata"));
        assert!(formatted.contains("1234ms"));
        assert!(formatted.contains("500"));
    }

    #[test]
    fn test_format_error() {
        let herald = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let error = PaladinError::ExecutionError("Something went wrong".to_string());

        let formatted = herald.format_error(&error);
        assert!(formatted.contains("**Error**"));
        assert!(formatted.contains("Something went wrong"));
        assert!(formatted.contains("```"));
    }

    #[test]
    fn test_name() {
        let herald = MarkdownHerald::new();
        assert_eq!(herald.name(), "markdown");
    }

    #[test]
    fn test_mime_type() {
        let herald = MarkdownHerald::new();
        assert_eq!(herald.mime_type(), "text/markdown");
    }

    #[test]
    fn test_color_support_detection() {
        // This test verifies the function runs without panicking
        // Actual color support depends on environment
        let _supports_color = MarkdownHeraldConfig::supports_color();
    }

    #[test]
    fn test_with_and_without_colors() {
        let result = create_test_paladin_result();

        let herald_no_color = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let formatted_no_color = herald_no_color.format_paladin_result(&result).unwrap();

        let herald_with_color = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: true,
            heading_level: 2,
        });
        let formatted_with_color = herald_with_color.format_paladin_result(&result).unwrap();

        // Both should contain the same content
        assert!(formatted_no_color.contains("Test output content"));
        assert!(formatted_with_color.contains("Test output content"));
        // Note: length comparison is omitted — the timestamp inside format_paladin_result
        // uses chrono::Utc::now() which can produce variable-length rfc3339 strings
        // (sub-second precision), making length comparisons non-deterministic in CI.
    }

    #[test]
    fn test_herald_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MarkdownHerald>();
    }

    #[test]
    fn test_default_trait() {
        let herald = MarkdownHerald::default();
        assert_eq!(herald.config.heading_level, 2);
    }

    #[test]
    fn test_streaming_output_consistency() {
        let herald = MarkdownHerald::new();

        // Simulate streaming chunks
        let chunks = vec![
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(0)
                .timestamp(chrono::Utc::now())
                .content("First chunk of text. ".to_string())
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(1)
                .timestamp(chrono::Utc::now())
                .content("Second chunk of text. ".to_string())
                .is_final(false)
                .build()
                .unwrap(),
            StreamChunk::builder()
                .chunk_id(uuid::Uuid::new_v4())
                .sequence_number(2)
                .timestamp(chrono::Utc::now())
                .content("Final chunk.".to_string())
                .is_final(true)
                .build()
                .unwrap(),
        ];

        // Collect streamed output
        let mut streamed_output = String::new();
        for chunk in &chunks {
            if let Some(formatted) = herald.format_stream_chunk(chunk).unwrap() {
                streamed_output.push_str(&formatted);
            }
        }

        // Add metadata
        use paladin_ports::output::llm_port::TokenUsage;
        let metadata = ExecutionMetadata::builder()
            .execution_id(uuid::Uuid::new_v4())
            .start_time(chrono::Utc::now())
            .model_used("gpt-4".to_string())
            .token_usage(TokenUsage::new(180, 120))
            .duration_ms(1500)
            .build()
            .unwrap();
        let metadata_output = herald.finalize_stream(&metadata).unwrap();
        streamed_output.push_str(&metadata_output);

        // Verify the concatenated content
        assert!(streamed_output.contains("First chunk of text."));
        assert!(streamed_output.contains("Second chunk of text."));
        assert!(streamed_output.contains("Final chunk."));

        // Verify metadata section is present
        assert!(
            streamed_output.contains("Execution Metadata") || streamed_output.contains("execution")
        );
        assert!(streamed_output.contains("1500ms") || streamed_output.contains("1500"));
        assert!(streamed_output.contains("300"));

        // Verify progressive nature - chunks should appear in order
        let first_pos = streamed_output.find("First chunk").unwrap();
        let second_pos = streamed_output.find("Second chunk").unwrap();
        let final_pos = streamed_output.find("Final chunk").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < final_pos);
    }
}
