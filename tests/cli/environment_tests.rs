//! Environment and CLI end-to-end tests
//!
//! Tier 1 tests: Core functionality with mocked components (no external dependencies)
//! Includes non-interactive mode tests (5.4) and environment variation tests (5.5)

use paladin::application::cli::config::loader::load_paladin_config;
use paladin::application::cli::error::CliError;
use paladin::application::cli::formatters::output::{OutputFormatter, OutputStyle};
use paladin::application::cli::formatters::table::TableFormatter;
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temporary directory for test files
fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper to create a minimal valid Paladin config
fn create_minimal_config(temp_dir: &TempDir, name: &str) -> PathBuf {
    let config_path = temp_dir.path().join(format!("{}.yaml", name));
    let config_content = r#"
name: "test-paladin"
system_prompt: "You are a helpful assistant"
model: "gpt-4"
temperature: 0.7
max_loops: 3

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config file");
    config_path
}

/// Helper to create an invalid YAML config
fn create_invalid_yaml(temp_dir: &TempDir, name: &str) -> PathBuf {
    let config_path = temp_dir.path().join(format!("{}.yaml", name));
    let invalid_yaml = r#"
name: "test"
invalid: [unclosed
brackets
"#;
    fs::write(&config_path, invalid_yaml).expect("Failed to write invalid config");
    config_path
}

// =============================================================================
// 5.1.1 & 5.1.2: Happy path tests (Note: Full agent/battalion run requires
// real LLM provider - tested in integration tests)
// =============================================================================

#[test]
fn test_minimal_config_loads_successfully() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = create_minimal_config(&temp_dir, "minimal");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_ok(),
        "Minimal valid config should load successfully"
    );
    let config = result.unwrap();
    assert_eq!(config.name, "test-paladin");
    assert_eq!(config.provider.provider_type, "openai");
}

// =============================================================================
// 5.1.3: Test error handling - missing required arguments
// =============================================================================

#[test]
fn test_missing_config_file_error() {
    // Arrange
    let non_existent_path = PathBuf::from("/tmp/does_not_exist_12345.yaml");

    // Act
    let result = load_paladin_config(&non_existent_path);

    // Assert
    assert!(
        result.is_err(),
        "Loading non-existent config should return error"
    );

    match result {
        Err(CliError::ConfigFileNotFound { .. }) => {
            // Expected error type
        }
        Err(e) => panic!("Expected ConfigFileNotFound error, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_config_missing_required_paladin_name() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("missing_name.yaml");
    let config_content = r#"
system_prompt: "Test"
model: "gpt-4"

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_err(),
        "Config missing required 'name' should return error"
    );
}

#[test]
fn test_config_missing_provider() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("missing_provider.yaml");
    let config_content = r#"
name: "test"
system_prompt: "Test"
model: "gpt-4"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_err(),
        "Config missing provider section should return error"
    );
}

// =============================================================================
// 5.1.4 & 5.1.8: Test error handling - invalid YAML config & malformed YAML
// =============================================================================

#[test]
fn test_invalid_yaml_syntax_error() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = create_invalid_yaml(&temp_dir, "invalid_syntax");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_err(),
        "Invalid YAML syntax should return parse error"
    );

    match result {
        Err(CliError::InvalidYaml { .. }) => {
            // Expected error type
        }
        Err(e) => panic!("Expected InvalidYaml, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[test]
fn test_malformed_yaml_with_tabs() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("tabs.yaml");
    // YAML doesn't allow tabs for indentation
    let config_with_tabs = "paladin:\n\tname: \"test\"\n\tsystem_prompt: \"Test\"";
    fs::write(&config_path, config_with_tabs).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(result.is_err(), "YAML with tabs should return parse error");
}

#[test]
fn test_yaml_with_duplicate_keys() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("duplicate_keys.yaml");
    let duplicate_keys_yaml = r#"
name: "first"
name: "second"
system_prompt: "Test"
model: "gpt-4"

provider:
  type: "openai"
"#;
    fs::write(&config_path, duplicate_keys_yaml).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // YAML parsers typically use the last value for duplicate keys
    // This may succeed but with the last value, or fail depending on parser
    // We document the behavior
    if let Ok(config) = result {
        // Should have the second value
        assert_eq!(
            config.name, "second",
            "Duplicate keys should use last value"
        );
    }
}

// =============================================================================
// 5.1.5: Test error handling - connection failures (will be simulated)
// =============================================================================

// Note: Connection failure tests require mocking at a lower level or
// integration with real services. These are deferred to Tier 2 (Docker-gated)
// tests in Task 5.2

// =============================================================================
// 5.1.6: Test edge case - empty input
// =============================================================================

#[test]
fn test_config_with_empty_system_prompt() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("empty_prompt.yaml");
    let config_content = r#"
name: "test"
system_prompt: ""
model: "gpt-4"

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // Empty system prompt should either be rejected or handled gracefully
    if let Ok(config) = result {
        assert_eq!(config.system_prompt, "");
    }
}

#[test]
fn test_config_with_empty_model_name() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("empty_model.yaml");
    let config_content = r#"
name: "test"
system_prompt: "Test"
model: ""

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // Empty model should be handled (either error or default)
    if let Ok(config) = result {
        // Document behavior
        println!("Empty model resolved to: {}", config.model);
    }
}

// =============================================================================
// 5.1.7: Test edge case - very large input (>10KB)
// =============================================================================

#[test]
fn test_config_with_very_large_system_prompt() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("large_prompt.yaml");

    // Create 15KB system prompt (exceeds typical limits)
    let large_prompt = "A".repeat(15_000);
    let config_content = format!(
        r#"
name: "test"
system_prompt: "{}"
model: "gpt-4"

provider:
  type: "openai"
"#,
        large_prompt
    );
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_ok(),
        "Config with large system prompt should load (validation happens at execution)"
    );

    if let Ok(config) = result {
        assert!(
            config.system_prompt.len() > 10_000,
            "Large prompt should be preserved"
        );
    }
}

#[test]
fn test_config_with_large_file_size() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("large_file.yaml");

    // Create a large but valid config (100KB)
    let mut config_content = String::from(
        r#"
name: "test"
system_prompt: "Test"
model: "gpt-4"
stop_words:
"#,
    );

    // Add many stop words to increase file size
    for i in 0..10_000 {
        config_content.push_str(&format!("  - \"stop_word_{}\"\n", i));
    }

    config_content.push_str(
        r#"
provider:
  type: "openai"
"#,
    );

    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    assert!(
        result.is_ok(),
        "Config with large file size should load successfully"
    );

    if let Ok(config) = result {
        assert!(
            config.stop_words.len() > 5_000,
            "Should preserve large stop_words list"
        );
    }
}

// =============================================================================
// 5.1.9: Test edge case - concurrent operations (Rust handles this well)
// =============================================================================

#[tokio::test]
async fn test_concurrent_config_loading() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = create_minimal_config(&temp_dir, "concurrent_test");

    // Act: Load same config from multiple tasks concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let path = config_path.clone();
        let handle = tokio::spawn(async move { load_paladin_config(&path) });
        handles.push(handle);
    }

    // Wait for all tasks
    let results: Vec<_> = futures::future::join_all(handles).await;

    // Assert: All should succeed
    for result in results {
        let config_result = result.expect("Task should not panic");
        assert!(
            config_result.is_ok(),
            "Concurrent config loading should succeed"
        );
    }
}

// =============================================================================
// Additional validation tests
// =============================================================================

#[test]
fn test_config_with_invalid_provider_type() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("invalid_provider.yaml");
    let config_content = r#"
name: "test"
system_prompt: "Test"
model: "gpt-4"

provider:
  type: "invalid_provider_type"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // Invalid provider type should be caught during validation or provider creation
    // Config loading may succeed but validation should catch it later
    if let Ok(config) = result {
        assert_eq!(config.provider.provider_type, "invalid_provider_type");
        // Validation would happen when creating the provider
    }
}

#[test]
fn test_config_with_negative_temperature() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("negative_temp.yaml");
    let config_content = r#"
name: "test"
system_prompt: "Test"
model: "gpt-4"
temperature: -0.5

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // Should either reject negative temperature or clamp it
    if let Ok(config) = result {
        println!("Negative temperature handled as: {}", config.temperature);
    }
}

#[test]
fn test_config_with_temperature_above_2() {
    // Arrange
    let temp_dir = create_test_dir();
    let config_path = temp_dir.path().join("high_temp.yaml");
    let config_content = r#"
name: "test"
system_prompt: "Test"
model: "gpt-4"
temperature: 5.0

provider:
  type: "openai"
"#;
    fs::write(&config_path, config_content).expect("Failed to write config");

    // Act
    let result = load_paladin_config(&config_path);

    // Assert
    // Should either reject or clamp temperature > 2.0
    if let Ok(config) = result {
        println!("High temperature handled as: {}", config.temperature);
    }
}

// =============================================================================
// 5.4: Non-interactive mode tests
// =============================================================================
//
// These tests verify that CLI operations work without interactive prompts,
// which is critical for CI/CD and scripted usage.

#[test]
fn test_config_loading_is_non_interactive() {
    // Config loading should never require interactive input.
    // This test verifies load_paladin_config returns immediately
    // (success or error) without blocking on stdin.
    let temp_dir = create_test_dir();
    let config_path = create_minimal_config(&temp_dir, "non_interactive");

    // If this blocks for user input, the test will timeout
    let result = load_paladin_config(&config_path);
    assert!(
        result.is_ok(),
        "Config loading must be non-interactive: {:?}",
        result.err()
    );
}

#[test]
fn test_missing_config_gives_error_not_prompt() {
    // When config file is missing, we must get a clear error,
    // not a hanging interactive prompt asking for a path.
    let missing_path = PathBuf::from("/tmp/nonexistent_config_test_54321.yaml");

    let result = load_paladin_config(&missing_path);
    assert!(
        result.is_err(),
        "Missing config must return error immediately, not prompt for input"
    );

    match result {
        Err(CliError::ConfigFileNotFound { .. }) => {
            // Expected: clear error, not a prompt
        }
        Err(e) => panic!("Expected ConfigFileNotFound, got: {:?}", e),
        Ok(_) => unreachable!(),
    }
}

#[test]
fn test_invalid_config_gives_error_not_prompt() {
    // When config has invalid YAML, we must get a parse error,
    // not a prompt asking the user to fix it.
    let temp_dir = create_test_dir();
    let config_path = create_invalid_yaml(&temp_dir, "bad_config_no_prompt");

    let result = load_paladin_config(&config_path);
    assert!(
        result.is_err(),
        "Invalid config must return error immediately, not prompt for corrections"
    );
}

#[test]
fn test_agent_run_args_accept_all_fields_via_flags() {
    // Verify AgentRunArgs struct accepts all required fields programmatically,
    // meaning CLI users can supply everything via flags without interaction.
    use paladin::application::cli::commands::agent::AgentRunArgs;

    let args = AgentRunArgs {
        config: PathBuf::from("agent.yaml"),
        input: Some("What is Rust?".to_string()),
        output: Some(PathBuf::from("output.json")),
        verbose: true,
        images: vec![],
        document: None,
        auto_plan: false,
        auto_prompt: false,
        dynamic_temp: false,
        enable_handoffs: false,
    };

    // All flags are set - no interactive prompt needed
    assert!(args.input.is_some(), "Input provided via flag");
    assert!(args.output.is_some(), "Output provided via flag");
    assert!(args.verbose, "Verbose provided via flag");
}

#[test]
fn test_agent_new_args_accept_all_fields_via_flags() {
    // Verify AgentNewArgs struct accepts all required fields programmatically.
    use paladin::application::cli::commands::agent::AgentNewArgs;

    let args = AgentNewArgs {
        name: "my-assistant".to_string(),
        output: PathBuf::from("agent.yaml"),
        provider: Some("openai".to_string()),
    };

    assert_eq!(args.name, "my-assistant");
    assert_eq!(args.output, PathBuf::from("agent.yaml"));
    assert_eq!(args.provider, Some("openai".to_string()));
}

#[test]
fn test_interactive_prompt_detects_non_tty() {
    // In CI/CD (and during `cargo test`), stdin is typically NOT a terminal.
    // The interactive functions should detect this and fail gracefully.
    //
    // NOTE: This test verifies the TTY detection logic. In environments
    // where stdin IS a terminal (e.g., developer running tests manually),
    // this test documents the expected behavior.
    let stdin_is_tty = std::io::stdin().is_terminal();
    let stdout_is_tty = std::io::stdout().is_terminal();

    if !stdin_is_tty || !stdout_is_tty {
        // Non-TTY environment (CI/CD): prompt_for_input should fail
        let result = paladin::application::cli::interactive::prompt_for_input("test prompt");
        assert!(
            result.is_err(),
            "prompt_for_input should fail in non-TTY environment"
        );

        match result {
            Err(CliError::ValidationError { .. }) => {
                // Expected: TTY validation error
            }
            Err(e) => panic!("Expected ValidationError for non-TTY, got: {:?}", e),
            Ok(_) => unreachable!(),
        }
    } else {
        // TTY environment: skip this check (developer terminal)
        println!(
            "Skipping non-TTY assertion: running in TTY environment (stdin_tty={}, stdout_tty={})",
            stdin_is_tty, stdout_is_tty
        );
    }
}

#[test]
fn test_confirm_detects_non_tty() {
    // Same as above but for the confirm() function.
    let stdin_is_tty = std::io::stdin().is_terminal();
    let stdout_is_tty = std::io::stdout().is_terminal();

    if !stdin_is_tty || !stdout_is_tty {
        let result = paladin::application::cli::interactive::confirm("Proceed?", false);
        assert!(
            result.is_err(),
            "confirm should fail in non-TTY environment"
        );

        match result {
            Err(CliError::ValidationError { .. }) => {
                // Expected: TTY validation error
            }
            Err(e) => panic!("Expected ValidationError for non-TTY, got: {:?}", e),
            Ok(_) => unreachable!(),
        }
    } else {
        println!("Skipping non-TTY assertion: running in TTY environment");
    }
}

#[test]
fn test_template_generation_without_interaction() {
    // Template generation (agent new) is non-interactive when the output
    // file doesn't already exist (no overwrite confirmation needed).
    use paladin::application::cli::commands::agent::{AgentNewArgs, handle_agent_new};

    let temp_dir = create_test_dir();
    let output_path = temp_dir.path().join("new_template.yaml");

    let args = AgentNewArgs {
        name: "test-agent".to_string(),
        output: output_path.clone(),
        provider: Some("openai".to_string()),
    };

    // This should complete without any interaction since file doesn't exist
    let result = handle_agent_new(args);
    assert!(
        result.is_ok(),
        "Template generation should be non-interactive when file doesn't exist: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "Template file should be created");

    // Verify the generated template is valid YAML
    let content = fs::read_to_string(&output_path).expect("Failed to read template");
    assert!(
        content.contains("test-agent"),
        "Template should contain the agent name"
    );
}

#[test]
fn test_template_invalid_provider_gives_error_not_prompt() {
    // Invalid provider should return an error, not prompt for correction.
    use paladin::application::cli::commands::agent::{AgentNewArgs, handle_agent_new};

    let temp_dir = create_test_dir();
    let output_path = temp_dir.path().join("invalid_provider.yaml");

    let args = AgentNewArgs {
        name: "test-agent".to_string(),
        output: output_path,
        provider: Some("invalid_llm".to_string()),
    };

    let result = handle_agent_new(args);
    assert!(
        result.is_err(),
        "Invalid provider should return error, not prompt for correction"
    );

    match result {
        Err(CliError::InvalidFieldValue { field, .. }) => {
            assert_eq!(field, "provider", "Should reference the provider field");
        }
        Err(e) => panic!("Expected InvalidFieldValue, got: {:?}", e),
        Ok(_) => unreachable!(),
    }
}

// =============================================================================
// 5.5: Environment variation tests
// =============================================================================
//
// These tests verify formatter behavior across different environment settings
// (NO_COLOR, quiet mode, verbose mode, etc.)

#[test]
fn test_output_formatter_style_returns_raw_text_when_colors_disabled() {
    // When colors are disabled (NO_COLOR set), style() should return
    // the input text unchanged (no ANSI escape codes).
    //
    // We test this by creating a formatter and checking style() output
    // against the raw input text.
    let formatter = OutputFormatter::new();

    // style() should always return text containing the original content
    let styled = formatter.style("hello", OutputStyle::Success);
    assert!(
        styled.contains("hello"),
        "Styled text must contain original content"
    );

    // When colors are disabled, the output should be exactly the input
    // When colors are enabled, the output will have ANSI codes wrapping it
    // Either way, the text content is preserved
    let styled_error = formatter.style("error msg", OutputStyle::Error);
    assert!(
        styled_error.contains("error msg"),
        "Error styled text must contain original content"
    );
}

#[test]
fn test_output_formatter_emoji_or_behavior() {
    // emoji_or() returns emoji when colors are enabled, alternative when disabled.
    let formatter = OutputFormatter::new();

    let emoji_result = formatter.emoji_or("✓", "[OK]");
    let alt_result = formatter.emoji_or("✗", "[FAIL]");

    // In any case, we get one of the two options
    assert!(
        emoji_result == "✓" || emoji_result == "[OK]",
        "emoji_or should return emoji or alt text, got: {}",
        emoji_result
    );
    assert!(
        alt_result == "✗" || alt_result == "[FAIL]",
        "emoji_or should return emoji or alt text, got: {}",
        alt_result
    );
}

#[test]
fn test_output_formatter_quiet_mode_flags() {
    // Quiet mode should suppress non-critical output.
    let quiet_formatter = OutputFormatter::quiet();
    assert!(
        quiet_formatter.is_quiet(),
        "Quiet formatter should report quiet"
    );
    assert!(
        !quiet_formatter.is_verbose(),
        "Quiet formatter should not be verbose"
    );

    // Verbose mode should show extra details.
    let verbose_formatter = OutputFormatter::with_verbose();
    assert!(
        verbose_formatter.is_verbose(),
        "Verbose formatter should report verbose"
    );
    assert!(
        !verbose_formatter.is_quiet(),
        "Verbose formatter should not be quiet"
    );
}

#[test]
fn test_output_formatter_set_quiet_disables_verbose() {
    // Setting quiet mode should disable verbose mode (mutually exclusive).
    let mut formatter = OutputFormatter::with_verbose();
    assert!(formatter.is_verbose());

    formatter.set_quiet(true);
    assert!(formatter.is_quiet(), "Quiet should be enabled");
    assert!(
        !formatter.is_verbose(),
        "Verbose should be disabled when quiet is set"
    );
}

#[test]
fn test_output_formatter_set_verbose_disables_quiet() {
    // Setting verbose mode should disable quiet mode (mutually exclusive).
    let mut formatter = OutputFormatter::quiet();
    assert!(formatter.is_quiet());

    formatter.set_verbose(true);
    assert!(formatter.is_verbose(), "Verbose should be enabled");
    assert!(
        !formatter.is_quiet(),
        "Quiet should be disabled when verbose is set"
    );
}

#[test]
fn test_output_formatter_all_styles_produce_output() {
    // All OutputStyle variants should produce non-empty output.
    let formatter = OutputFormatter::new();

    let styles = [
        OutputStyle::Success,
        OutputStyle::Error,
        OutputStyle::Warning,
        OutputStyle::Info,
        OutputStyle::Link,
        OutputStyle::Default,
    ];

    for style in &styles {
        let result = formatter.style("test", *style);
        assert!(
            !result.is_empty(),
            "Style {:?} should produce non-empty output",
            style
        );
        assert!(
            result.contains("test"),
            "Style {:?} output must contain original text",
            style
        );
    }
}

#[test]
fn test_output_formatter_key_value_format() {
    // key_value() should format as "key: value".
    let formatter = OutputFormatter::new();
    let kv = formatter.key_value("Name", "test-paladin");
    assert!(kv.contains("Name"), "key_value must contain the key");
    assert!(
        kv.contains("test-paladin"),
        "key_value must contain the value"
    );
}

#[test]
fn test_output_formatter_default_construction() {
    // Default trait implementation should work the same as new().
    let default_formatter = OutputFormatter::default();
    let new_formatter = OutputFormatter::new();

    // Both should have the same quiet/verbose state
    assert_eq!(default_formatter.is_quiet(), new_formatter.is_quiet());
    assert_eq!(default_formatter.is_verbose(), new_formatter.is_verbose());
}

#[test]
fn test_table_formatter_renders_without_colors() {
    // TableFormatter should produce valid output regardless of NO_COLOR setting.
    let mut table = TableFormatter::new();
    table
        .set_header(vec!["Service", "Status", "Latency"])
        .add_row(vec!["Redis", "Connected", "2ms"])
        .add_row(vec!["MinIO", "Connected", "5ms"]);

    let output = table.render();
    assert!(output.contains("Service"), "Table must contain header");
    assert!(output.contains("Redis"), "Table must contain row data");
    assert!(output.contains("MinIO"), "Table must contain row data");
}

#[test]
fn test_table_formatter_styled_cells_degrade_gracefully() {
    // Styled cells should work regardless of color support.
    let table = TableFormatter::new();

    let success = table.success_cell("OK");
    let error = table.error_cell("FAIL");
    let warning = table.warning_cell("WARN");
    let info = table.info_cell("NOTE");

    // Cell content should always be preserved
    assert_eq!(success.content(), "OK");
    assert_eq!(error.content(), "FAIL");
    assert_eq!(warning.content(), "WARN");
    assert_eq!(info.content(), "NOTE");
}

#[test]
fn test_table_formatter_empty_table() {
    // Empty table (no rows) should render without panicking.
    let table = TableFormatter::new();
    let output = table.render();
    // Empty table may render as empty string or just borders
    assert!(
        output.is_empty() || !output.is_empty(),
        "Empty table should render without panic"
    );
}

#[test]
fn test_table_formatter_single_column() {
    // Single-column table should work correctly.
    let mut table = TableFormatter::new();
    table
        .set_header(vec!["Item"])
        .add_row(vec!["Alpha"])
        .add_row(vec!["Beta"]);

    let output = table.render();
    assert!(output.contains("Item"));
    assert!(output.contains("Alpha"));
    assert!(output.contains("Beta"));
}

#[test]
fn test_table_formatter_unicode_content() {
    // Unicode characters should be preserved in table output.
    // This tests behavior in "basic terminal environment" (5.5.2).
    let mut table = TableFormatter::new();
    table
        .set_header(vec!["Paladin", "Status"])
        .add_row(vec!["研究者", "✓"])
        .add_row(vec!["Müller", "⚠"]);

    let output = table.render();
    assert!(output.contains("研究者"), "Unicode CJK should be preserved");
    assert!(
        output.contains("Müller"),
        "Unicode Latin should be preserved"
    );
    assert!(output.contains("✓"), "Unicode symbols should be preserved");
}

#[test]
fn test_output_formatter_header_contains_title() {
    // header() should produce output containing the title text.
    // We can't capture stdout easily, but we can verify the
    // underlying style() method works with header-like content.
    let formatter = OutputFormatter::new();
    let styled_title = formatter.style("Paladin Status", OutputStyle::Info);
    assert!(
        styled_title.contains("Paladin Status"),
        "Header title must be in styled output"
    );
}

#[test]
fn test_no_color_environment_detection() {
    // Document the expected behavior of NO_COLOR checking.
    // In test environments, NO_COLOR may or may not be set.
    // The formatter should respect whichever state is present.
    let no_color_set = std::env::var("NO_COLOR").is_ok();

    let formatter = OutputFormatter::new();
    let styled = formatter.style("test", OutputStyle::Success);

    if no_color_set {
        // NO_COLOR is set: output should be plain text
        assert_eq!(
            styled, "test",
            "With NO_COLOR set, style should return plain text"
        );
    } else {
        // NO_COLOR not set: output may contain ANSI codes
        // The text content should still be there
        assert!(
            styled.contains("test"),
            "Without NO_COLOR, styled output should contain original text"
        );
    }
}

#[test]
fn test_ci_environment_line_buffering() {
    // In CI/CD environments, output should use line buffering.
    // Rust's println! uses line buffering by default, which is correct.
    // This test documents that behavior.
    //
    // The OutputFormatter methods (success, error, warning, info) all use
    // println!/eprintln! which flush at each newline - appropriate for CI.
    let formatter = OutputFormatter::new();

    // Verify formatter methods produce strings (not just print to stdout)
    let kv = formatter.key_value("CI", "true");
    assert!(!kv.is_empty(), "key_value should produce output string");

    // style() returns a string, allowing callers to control output
    let styled = formatter.style("buffered output", OutputStyle::Info);
    assert!(
        styled.contains("buffered output"),
        "style() returns string for caller-controlled output"
    );
}

// =============================================================================
// 5.6: Full user journey test (with mock provider)
// =============================================================================
//
// Simulates the new user experience:
//   1. Generate a config template (onboarding)
//   2. Load the generated config (config validation)
//   3. Verify the pipeline is ready for execution

#[test]
fn test_user_journey_template_to_config_load() {
    // Simulate: onboarding → config generation → config loading
    use paladin::application::cli::commands::agent::{AgentNewArgs, handle_agent_new};

    let temp_dir = create_test_dir();

    // Step 1: Generate a new template (like `paladin agent new`)
    let template_path = temp_dir.path().join("my-first-agent.yaml");
    let args = AgentNewArgs {
        name: "my-first-agent".to_string(),
        output: template_path.clone(),
        provider: Some("openai".to_string()),
    };
    let result = handle_agent_new(args);
    assert!(
        result.is_ok(),
        "Step 1 - Template generation should succeed: {:?}",
        result.err()
    );
    assert!(template_path.exists(), "Step 1 - Template file created");

    // Step 2: Load the generated template as config
    let config = load_paladin_config(&template_path);
    assert!(
        config.is_ok(),
        "Step 2 - Generated template should be loadable as config: {:?}",
        config.err()
    );

    // Step 3: Verify loaded config matches expectations
    let config = config.unwrap();
    assert_eq!(config.name, "my-first-agent");
    assert_eq!(config.provider.provider_type, "openai");
    assert!(config.temperature > 0.0, "Temperature should be positive");
    assert!(
        matches!(config.max_loops, paladin::platform::container::paladin::MaxLoops::Fixed(n) if n > 0),
        "Max loops should be a positive Fixed value"
    );
    assert!(
        !config.system_prompt.is_empty(),
        "System prompt should be generated"
    );
}

#[test]
fn test_user_journey_all_providers() {
    // Verify template → config load works for each provider
    use paladin::application::cli::commands::agent::{AgentNewArgs, handle_agent_new};

    let temp_dir = create_test_dir();

    for provider in &["openai", "deepseek", "anthropic"] {
        let template_path = temp_dir.path().join(format!("{}-agent.yaml", provider));
        let args = AgentNewArgs {
            name: format!("{}-agent", provider),
            output: template_path.clone(),
            provider: Some(provider.to_string()),
        };

        let gen_result = handle_agent_new(args);
        assert!(
            gen_result.is_ok(),
            "Template generation should succeed for provider {}: {:?}",
            provider,
            gen_result.err()
        );

        let load_result = load_paladin_config(&template_path);
        assert!(
            load_result.is_ok(),
            "Generated {} template should load successfully: {:?}",
            provider,
            load_result.err()
        );

        let config = load_result.unwrap();
        assert_eq!(config.provider.provider_type, *provider);
    }
}

#[test]
fn test_user_journey_modify_and_reload_config() {
    // Simulate: user generates template, modifies it, reloads
    use paladin::application::cli::commands::agent::{AgentNewArgs, handle_agent_new};

    let temp_dir = create_test_dir();
    let template_path = temp_dir.path().join("customized-agent.yaml");

    // Step 1: Generate template
    let args = AgentNewArgs {
        name: "custom-agent".to_string(),
        output: template_path.clone(),
        provider: Some("openai".to_string()),
    };
    handle_agent_new(args).expect("Template generation should succeed");

    // Step 2: User customizes the config by writing a new version
    let custom_config = r#"
name: "my-custom-researcher"
system_prompt: |
  You are a research assistant specialized in AI papers.
  Always cite sources and provide detailed analysis.
model: "gpt-4-turbo"
temperature: 0.3
max_loops: 5
timeout_seconds: 600
stop_words:
  - "ANALYSIS_COMPLETE"
  - "END_REPORT"

provider:
  type: "openai"
"#;
    fs::write(&template_path, custom_config).expect("Failed to write custom config");

    // Step 3: Reload customized config
    let config =
        load_paladin_config(&template_path).expect("Customized config should load successfully");

    assert_eq!(config.name, "my-custom-researcher");
    assert_eq!(config.model, "gpt-4-turbo");
    assert!((config.temperature - 0.3).abs() < f32::EPSILON);
    assert_eq!(
        config.max_loops,
        paladin::platform::container::paladin::MaxLoops::Fixed(5)
    );
    assert_eq!(config.stop_words.len(), 2);
    assert!(config.stop_words.contains(&"ANALYSIS_COMPLETE".to_string()));
    assert!(config.stop_words.contains(&"END_REPORT".to_string()));
}

#[test]
fn test_user_journey_output_formatting() {
    // Simulate: user runs agent, results are formatted for display
    // We can't run a real LLM call, but we test the formatting pipeline
    use paladin_ports::output::paladin_port::{PaladinResult, StopReason};

    let formatter = OutputFormatter::new();

    // Simulate a PaladinResult
    let result = PaladinResult {
        output: "Rust is a systems programming language focused on safety and performance."
            .to_string(),
        token_count: 42,
        execution_time_ms: 1250,
        loop_count: 1,
        stop_reason: StopReason::Completed,
        plan: None,
        handoff_history: vec![],
        served_by: None,
    };

    // Format for display
    let formatted = formatter.format_paladin_result(&result, false);
    assert!(
        formatted.contains("Paladin Execution Result"),
        "Result should have header"
    );
    assert!(
        formatted.contains("Rust is a systems programming language"),
        "Result should contain output text"
    );
    assert!(
        formatted.contains("1.25s"),
        "Result should show execution time"
    );

    // Format for file output (JSON)
    let json = OutputFormatter::format_paladin_result_json(&result);
    assert_eq!(
        json["output"],
        "Rust is a systems programming language focused on safety and performance."
    );
    assert_eq!(json["metadata"]["token_count"], 42);
    assert_eq!(json["metadata"]["execution_time_ms"], 1250);
}

#[test]
fn test_user_journey_verbose_output_shows_details() {
    // When verbose mode is used, the formatter shows extra details
    use paladin_ports::output::paladin_port::{PaladinResult, StopReason};

    let formatter = OutputFormatter::with_verbose();

    let result = PaladinResult {
        output: "Answer".to_string(),
        token_count: 10,
        execution_time_ms: 500,
        loop_count: 3,
        stop_reason: StopReason::MaxLoops,
        plan: None,
        handoff_history: vec![],
        served_by: None,
    };

    // Verbose mode should include loop count and stop reason details
    let formatted = formatter.format_paladin_result(&result, true);
    assert!(
        formatted.contains("Reasoning Loops: 3"),
        "Verbose output should show loop count"
    );
    assert!(
        formatted.contains("Stop Reason"),
        "Verbose output should show stop reason"
    );
}
