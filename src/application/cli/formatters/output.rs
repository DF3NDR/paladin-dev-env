//! Core output formatting with colors, boxes, and headers
//!
//! Provides utilities for formatted terminal output that respects NO_COLOR
//! environment variable and supports quiet/verbose modes.

use colored::*;
use std::env;

/// Output styling options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStyle {
    /// Success message (green)
    Success,
    /// Error message (red)
    Error,
    /// Warning message (yellow)
    Warning,
    /// Informational message (blue)
    Info,
    /// Link or reference (cyan)
    Link,
    /// Default styling
    Default,
}

/// Output formatter with color and style support
#[derive(Debug, Clone)]
pub struct OutputFormatter {
    /// Whether colors are enabled
    colors_enabled: bool,
    /// Quiet mode (minimal output)
    quiet: bool,
    /// Verbose mode (detailed output)
    verbose: bool,
}

impl OutputFormatter {
    /// Create a new output formatter
    pub fn new() -> Self {
        Self {
            colors_enabled: !Self::no_color_requested(),
            quiet: false,
            verbose: false,
        }
    }

    /// Create a formatter with quiet mode enabled
    pub fn quiet() -> Self {
        Self {
            colors_enabled: !Self::no_color_requested(),
            quiet: true,
            verbose: false,
        }
    }

    /// Create a formatter with verbose mode enabled
    pub fn with_verbose() -> Self {
        Self {
            colors_enabled: !Self::no_color_requested(),
            quiet: false,
            verbose: true,
        }
    }

    /// Check if NO_COLOR environment variable is set
    fn no_color_requested() -> bool {
        env::var("NO_COLOR").is_ok()
    }

    /// Set quiet mode
    pub fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
        if quiet {
            self.verbose = false;
        }
    }

    /// Set verbose mode
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
        if verbose {
            self.quiet = false;
        }
    }

    /// Check if quiet mode is enabled
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Format text with the given style
    pub fn style(&self, text: &str, style: OutputStyle) -> String {
        if !self.colors_enabled {
            return text.to_string();
        }

        match style {
            OutputStyle::Success => text.green().to_string(),
            OutputStyle::Error => text.red().to_string(),
            OutputStyle::Warning => text.yellow().to_string(),
            OutputStyle::Info => text.blue().to_string(),
            OutputStyle::Link => text.cyan().to_string(),
            OutputStyle::Default => text.to_string(),
        }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", self.style("✓", OutputStyle::Success), message);
        }
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        eprintln!("{} {}", self.style("✗", OutputStyle::Error), message);
    }

    /// Print a warning message
    pub fn warning(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", self.style("⚠", OutputStyle::Warning), message);
        }
    }

    /// Print an info message
    pub fn info(&self, message: &str) {
        if !self.quiet {
            println!("{} {}", self.style("ℹ", OutputStyle::Info), message);
        }
    }

    /// Print a verbose message (only in verbose mode)
    pub fn verbose(&self, message: &str) {
        if self.verbose {
            println!("{}", self.style(message, OutputStyle::Default));
        }
    }

    /// Print a header with box drawing
    pub fn header(&self, title: &str) {
        if self.quiet {
            return;
        }

        let width = title.len() + 4;
        let border = "═".repeat(width);

        println!("┌{}┐", border);
        println!("│ {} │", self.style(title, OutputStyle::Info));
        println!("└{}┘", border);
    }

    /// Print a section header
    pub fn section(&self, title: &str) {
        if self.quiet {
            return;
        }

        println!(
            "\n{}",
            self.style(&format!("━━ {} ━━", title), OutputStyle::Info)
        );
    }

    /// Print a box with content
    pub fn box_message(&self, content: &[&str]) {
        if self.quiet {
            return;
        }

        let max_width = content.iter().map(|s| s.len()).max().unwrap_or(0);
        let border = "─".repeat(max_width + 2);

        println!("┌{}┐", border);
        for line in content {
            println!("│ {:<width$} │", line, width = max_width);
        }
        println!("└{}┘", border);
    }

    /// Format a key-value pair
    pub fn key_value(&self, key: &str, value: &str) -> String {
        format!("{}: {}", self.style(key, OutputStyle::Info), value)
    }

    /// Print an emoji if colors are enabled, otherwise print alternative text
    pub fn emoji_or<'a>(&self, emoji: &'a str, alt: &'a str) -> &'a str {
        if self.colors_enabled { emoji } else { alt }
    }

    /// Print a separator line
    pub fn separator(&self) {
        if !self.quiet {
            println!("{}", "═".repeat(64));
        }
    }

    /// Print a blank line (unless in quiet mode)
    pub fn blank_line(&self) {
        if !self.quiet {
            println!();
        }
    }

    /// Format a Paladin result for human-readable output
    ///
    /// # Arguments
    ///
    /// * `result` - The Paladin execution result
    /// * `verbose` - Whether to include detailed information (loops, timing, etc.)
    ///
    /// # Example Output (Normal Mode)
    ///
    /// ```text
    /// ════════════════════════════════════════════════════════════════════
    /// 📊 Paladin Execution Result
    /// ════════════════════════════════════════════════════════════════════
    ///
    /// → Output:
    /// The analysis shows that...
    ///
    /// → Statistics:
    ///   • Execution Time: 1.25s
    ///   • Tokens Used: 150
    ///   • Status: Completed ✓
    /// ════════════════════════════════════════════════════════════════════
    /// ```
    pub fn format_paladin_result(
        &self,
        result: &paladin_ports::output::paladin_port::PaladinResult,
        verbose: bool,
    ) -> String {
        use paladin_ports::output::paladin_port::StopReason;

        let mut output = String::new();

        output.push_str(&"═".repeat(80));
        output.push_str(&format!(
            "\n{} Paladin Execution Result\n",
            self.style("📊", OutputStyle::Info)
        ));
        output.push_str(&"═".repeat(80));
        output.push('\n');

        // Main output
        output.push_str(&format!(
            "\n{} Output:\n",
            self.style("→", OutputStyle::Info)
        ));
        output.push_str(&format!("{}\n", result.output));

        // Statistics section
        output.push_str(&format!(
            "\n{} Statistics:\n",
            self.style("→", OutputStyle::Info)
        ));
        output.push_str(&format!(
            "  {} Execution Time: {:.2}s\n",
            self.style("•", OutputStyle::Info),
            result.execution_time_ms as f64 / 1000.0
        ));
        output.push_str(&format!(
            "  {} Tokens Used: {}\n",
            self.style("•", OutputStyle::Info),
            result.token_count
        ));

        // Status with color coding
        let status_str = match &result.stop_reason {
            StopReason::Completed => format!(
                "{} {}",
                self.style("Completed", OutputStyle::Success),
                self.style("✓", OutputStyle::Success)
            ),
            StopReason::StopWord(word) => format!(
                "{} {} ({})",
                self.style("Stopped", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning),
                word
            ),
            StopReason::MaxLoops => format!(
                "{} {}",
                self.style("Max Loops Reached", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning)
            ),
            StopReason::Timeout => format!(
                "{} {}",
                self.style("Timeout", OutputStyle::Error),
                self.style("✗", OutputStyle::Error)
            ),
            StopReason::CallLimit => format!(
                "{} {}",
                self.style("Call Limit Reached", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning)
            ),
            StopReason::TokenBudget => format!(
                "{} {}",
                self.style("Token Budget Reached", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning)
            ),
            _ => format!(
                "{} {}",
                self.style("Stopped", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning)
            ),
        };
        output.push_str(&format!(
            "  {} Status: {}\n",
            self.style("•", OutputStyle::Info),
            status_str
        ));

        // Verbose mode: additional details
        if verbose {
            output.push_str(&format!(
                "  {} Reasoning Loops: {}\n",
                self.style("•", OutputStyle::Info),
                result.loop_count
            ));
            output.push_str(&format!(
                "  {} Stop Reason: {:?}\n",
                self.style("•", OutputStyle::Info),
                result.stop_reason
            ));
        }

        output.push_str(&"═".repeat(80));
        output.push('\n');

        output
    }

    /// Format a Paladin result as JSON for file output
    ///
    /// Includes comprehensive metadata, timing information, and execution details.
    pub fn format_paladin_result_json(
        result: &paladin_ports::output::paladin_port::PaladinResult,
    ) -> serde_json::Value {
        use serde_json::json;

        json!({
            "output": result.output,
            "metadata": {
                "token_count": result.token_count,
                "execution_time_ms": result.execution_time_ms,
                "execution_time_seconds": result.execution_time_ms as f64 / 1000.0,
                "loop_count": result.loop_count,
                "stop_reason": format!("{:?}", result.stop_reason),
                "is_successful": result.stop_reason.is_successful(),
                "is_limit_reached": result.stop_reason.is_limit(),
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Format a Battalion result for human-readable output
    ///
    /// # Arguments
    ///
    /// * `result` - The Battalion execution result
    /// * `verbose` - Whether to include detailed per-Paladin information
    pub fn format_battalion_result(
        &self,
        result: &crate::core::platform::container::battalion::BattalionResult,
        verbose: bool,
    ) -> String {
        use crate::core::platform::container::battalion::BattalionStatus;
        use paladin_ports::output::paladin_port::StopReason;

        let mut output = String::new();

        output.push_str(&"═".repeat(80));
        output.push_str(&format!(
            "\n{} Battalion Execution Result: {}\n",
            self.style("🏰", OutputStyle::Info),
            self.style(&result.battalion_name, OutputStyle::Info)
        ));
        output.push_str(&"═".repeat(80));
        output.push('\n');

        // Final aggregated output
        output.push_str(&format!(
            "\n{} Final Output:\n",
            self.style("→", OutputStyle::Info)
        ));
        output.push_str(&format!("{}\n", result.final_output));

        // Statistics section
        output.push_str(&format!(
            "\n{} Statistics:\n",
            self.style("→", OutputStyle::Info)
        ));
        output.push_str(&format!(
            "  {} Total Paladins: {}\n",
            self.style("•", OutputStyle::Info),
            result.paladin_results.len()
        ));

        let success_style = if result.paladin_success_count == result.paladin_results.len() {
            OutputStyle::Success
        } else {
            OutputStyle::Warning
        };
        output.push_str(&format!(
            "  {} Successful: {} {}\n",
            self.style("•", OutputStyle::Info),
            result.paladin_success_count,
            self.style(
                if success_style == OutputStyle::Success {
                    "✓"
                } else {
                    "⚠"
                },
                success_style
            )
        ));

        if result.paladin_failure_count > 0 {
            output.push_str(&format!(
                "  {} Failed: {} {}\n",
                self.style("•", OutputStyle::Info),
                result.paladin_failure_count,
                self.style("✗", OutputStyle::Error)
            ));
        }

        let total_time = (result.completed_at - result.started_at)
            .num_milliseconds()
            .max(0) as u64;
        output.push_str(&format!(
            "  {} Total Time: {:.2}s\n",
            self.style("•", OutputStyle::Info),
            total_time as f64 / 1000.0
        ));

        output.push_str(&format!(
            "  {} Strategy: {:?}\n",
            self.style("•", OutputStyle::Info),
            result.strategy_used
        ));

        // Strategy selection reasoning (Auto mode)
        if let Some(reasoning) = &result.strategy_selection_reasoning {
            output.push_str(&format!(
                "  {} Strategy Selection: {}\n",
                self.style("•", OutputStyle::Info),
                reasoning
            ));
        }

        // Status
        let status_str = match result.status {
            BattalionStatus::Completed => format!(
                "{} {}",
                self.style("Completed", OutputStyle::Success),
                self.style("✓", OutputStyle::Success)
            ),
            BattalionStatus::Failed => format!(
                "{} {}",
                self.style("Failed", OutputStyle::Error),
                self.style("✗", OutputStyle::Error)
            ),
            BattalionStatus::Cancelled => format!(
                "{} {}",
                self.style("Cancelled", OutputStyle::Warning),
                self.style("⚠", OutputStyle::Warning)
            ),
            _ => format!("{:?}", result.status),
        };
        output.push_str(&format!(
            "  {} Status: {}\n",
            self.style("•", OutputStyle::Info),
            status_str
        ));

        // Verbose mode: show individual Paladin results
        if verbose && !result.paladin_results.is_empty() {
            output.push_str(&format!(
                "\n{} Individual Paladin Results:\n",
                self.style("→", OutputStyle::Info)
            ));
            output.push_str(&"─".repeat(80));
            output.push('\n');

            for (idx, paladin_result) in result.paladin_results.iter().enumerate() {
                // Try to find timing from per_paladin_times HashMap by index-based name,
                // fall back to the PaladinResult's own execution_time_ms
                let timing = result
                    .per_paladin_times
                    .get(&format!("paladin_{}", idx))
                    .copied()
                    .unwrap_or(paladin_result.execution_time_ms);

                output.push_str(&format!(
                    "\n{} Paladin {} - {} loops, {:.2}s, {} tokens\n",
                    self.style(&format!("{}.", idx + 1), OutputStyle::Info),
                    idx + 1,
                    paladin_result.loop_count,
                    timing as f64 / 1000.0,
                    paladin_result.token_count
                ));

                let (status_emoji, status_style) = match &paladin_result.stop_reason {
                    StopReason::Completed => ("✓", OutputStyle::Success),
                    StopReason::StopWord(_) => ("⚠", OutputStyle::Warning),
                    StopReason::MaxLoops => ("⚠", OutputStyle::Warning),
                    StopReason::Timeout => ("✗", OutputStyle::Error),
                    StopReason::CallLimit => ("⚠", OutputStyle::Warning),
                    StopReason::TokenBudget => ("⚠", OutputStyle::Warning),
                    _ => ("⚠", OutputStyle::Warning),
                };
                output.push_str(&format!(
                    "   Status: {} {:?}\n",
                    self.style(status_emoji, status_style),
                    paladin_result.stop_reason
                ));

                // Show first 200 chars of output
                let preview = if paladin_result.output.len() > 200 {
                    format!("{}...", &paladin_result.output[..200])
                } else {
                    paladin_result.output.clone()
                };
                output.push_str(&format!("   Output: {}\n", preview));
            }

            output.push_str(&"─".repeat(80));
            output.push('\n');
        }

        output.push_str(&"═".repeat(80));
        output.push('\n');

        output
    }

    /// Format a Battalion result as JSON for file output
    ///
    /// Includes comprehensive metadata, all individual Paladin results, and timing data.
    pub fn format_battalion_result_json(
        result: &crate::core::platform::container::battalion::BattalionResult,
    ) -> serde_json::Value {
        use serde_json::json;

        json!({
            "battalion_id": result.battalion_id,
            "battalion_name": result.battalion_name,
            "started_at": result.started_at.to_rfc3339(),
            "completed_at": result.completed_at.to_rfc3339(),
            "total_time_ms": (result.completed_at - result.started_at).num_milliseconds().max(0),
            "final_output": result.final_output,
            "status": format!("{:?}", result.status),
            "strategy_used": format!("{:?}", result.strategy_used),
            "strategy_selection_reasoning": result.strategy_selection_reasoning,
            "strategy_selection_time_ms": result.strategy_selection_time_ms,
            "paladin_results": result.paladin_results.iter().enumerate().map(|(idx, r)| {
                let timing = result.per_paladin_times.get(&format!("paladin_{}", idx)).copied().unwrap_or(r.execution_time_ms);
                json!({
                    "index": idx,
                    "output": r.output,
                    "token_count": r.token_count,
                    "execution_time_ms": timing,
                    "loop_count": r.loop_count,
                    "stop_reason": format!("{:?}", r.stop_reason),
                    "is_successful": r.stop_reason.is_successful(),
                })
            }).collect::<Vec<_>>(),
            "summary": {
                "total_paladins": result.paladin_results.len(),
                "successful": result.paladin_success_count,
                "failed": result.paladin_failure_count,
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new()
    }
}
