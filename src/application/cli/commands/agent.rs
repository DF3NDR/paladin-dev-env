//! Agent command implementations for creating and running Paladin agents
//!
//! This module provides CLI commands for:
//! - Creating Paladin configuration templates
//! - Executing Paladins from YAML configuration files
//!
//! # Examples
//!
//! ```bash
//! # Create a new agent template
//! paladin agent new my-assistant --output agent.yaml --provider openai
//!
//! # Run an agent from config
//! paladin agent run --config agent.yaml --input "What is Rust?"
//! ```

use crate::application::cli::config::loader::{instantiate_arsenal, instantiate_garrison};
use crate::application::cli::config::paladin_config::PaladinYamlConfig;
use crate::application::cli::error::CliError;
use crate::application::cli::templates::paladin_template::generate_paladin_template;
use crate::application::services::paladin::paladin_builder::PaladinBuilder;
use crate::core::platform::container::autonomous_config::HandoffConfig;
use clap::Subcommand;
use colored::Colorize;
use std::path::PathBuf;
use std::sync::Arc;

/// Agent subcommands for Paladin management
#[derive(Debug, Subcommand)]
pub enum AgentCommands {
    /// Create a new Paladin configuration template
    New(AgentNewArgs),
    /// Run a Paladin from configuration
    Run(AgentRunArgs),
}

/// Arguments for creating a new Paladin agent template
#[derive(Debug, clap::Args)]
pub struct AgentNewArgs {
    /// Name for the Paladin
    #[arg(short, long)]
    pub name: String,

    /// Output path for the template file
    #[arg(short, long)]
    pub output: PathBuf,

    /// LLM provider (openai, deepseek, anthropic)
    #[arg(short, long)]
    pub provider: Option<String>,
}

/// Arguments for executing a Paladin agent
#[derive(Debug, clap::Args)]
pub struct AgentRunArgs {
    /// Path to Paladin YAML configuration file
    #[arg(short, long)]
    pub config: PathBuf,

    /// Input text for the Paladin (prompts if not provided)
    #[arg(short, long)]
    pub input: Option<String>,

    /// Path to save output file (prints to stdout if not provided)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to image file(s) for vision processing (repeatable)
    #[cfg(feature = "vision")]
    #[arg(long = "image")]
    pub images: Vec<PathBuf>,

    /// Path to document file for processing (PDF, TXT, MD)
    #[arg(long = "document")]
    pub document: Option<PathBuf>,

    // Autonomous feature flags: additive overrides layered on top of the
    // YAML `autonomous` section (D-05, D-06, D-07). Each flag can only force
    // its feature ON. None of the four can turn OFF a feature the
    // configuration file already enabled -- there are no `--no-*`
    // counterparts.
    /// Force autonomous planning mode on, regardless of the configuration
    /// file's `autonomous.planning.enabled`. Cannot turn planning off if the
    /// configuration file already enabled it. Note: this does not set
    /// `MaxLoops::Auto` -- that mode is controlled independently by
    /// `max_loops` in the configuration file.
    #[arg(long = "auto-plan")]
    pub auto_plan: bool,

    /// Force automatic prompt generation on, regardless of the
    /// configuration file's `autonomous.prompt_generation.enabled`. Cannot
    /// turn prompt generation off if the configuration file already enabled
    /// it.
    #[arg(long = "auto-prompt")]
    pub auto_prompt: bool,

    /// Force dynamic temperature adjustment on, regardless of the
    /// configuration file's `autonomous.dynamic_temperature.enabled`.
    /// Cannot turn dynamic temperature off if the configuration file
    /// already enabled it.
    #[arg(long = "dynamic-temp")]
    pub dynamic_temp: bool,

    /// Force agent handoff capabilities on, regardless of the configuration
    /// file's `autonomous.handoffs.enabled`. Cannot turn handoffs off if the
    /// configuration file already enabled them. This flag only enables the
    /// handoff *configuration*; the specialist agents to hand off to are
    /// wired through the library's `PaladinBuilder::with_handoffs` surface,
    /// which this CLI does not expose.
    #[arg(long = "enable-handoffs")]
    pub enable_handoffs: bool,
}

/// Handle the `paladin agent new` command
///
/// Creates a new Paladin configuration template file with documented options
pub fn handle_agent_new(args: AgentNewArgs) -> Result<(), CliError> {
    use crate::application::cli::interactive::confirm;

    // Validate and normalize provider
    let provider = args.provider.as_deref().unwrap_or("openai");
    let valid_providers = ["openai", "deepseek", "anthropic"];

    if !valid_providers.contains(&provider) {
        return Err(CliError::InvalidFieldValue {
            field: "provider".to_string(),
            message: format!(
                "must be one of: {}. Got: {}",
                valid_providers.join(", "),
                provider
            ),
        });
    }

    // Check if output file already exists and prompt for confirmation
    if args.output.exists() {
        let should_overwrite = confirm(
            &format!(
                "File '{}' already exists. Overwrite?",
                args.output.display()
            ),
            false,
        )?;

        if !should_overwrite {
            return Err(CliError::Cancelled);
        }
    }

    // Generate template
    let template = generate_paladin_template(&args.name, provider);

    // Write to file
    std::fs::write(&args.output, template)?;

    // Print success message with colored output
    println!(
        "{} Created Paladin template: {}",
        "✓".green().bold(),
        args.output.display()
    );

    Ok(())
}

/// Handle the `paladin agent run` command
///
/// Loads a Paladin configuration and executes it with the given input
pub async fn handle_agent_run(args: AgentRunArgs) -> Result<(), CliError> {
    use crate::application::cli::config::loader::load_paladin_config;
    use crate::application::cli::interactive::prompt_for_input;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    #[cfg(feature = "vision")]
    use crate::core::platform::container::vision::{ImageDetail, VisionContent};
    #[cfg(feature = "content-processing")]
    use crate::infrastructure::adapters::document::DocumentAdapter;
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_llm::provider_factory::LlmProviderFactory;
    #[cfg(feature = "content-processing")]
    use paladin_ports::input::document_port::{DocumentPort, DocumentSource};
    use paladin_ports::output::llm_port::LlmPort;
    use std::time::Duration;

    // Validate image paths if provided
    #[cfg(feature = "vision")]
    if !args.images.is_empty() {
        for image_path in &args.images {
            if !image_path.exists() {
                return Err(CliError::InvalidFilePath {
                    path: image_path.display().to_string(),
                    message: "Image file does not exist".to_string(),
                });
            }

            // Validate image format
            let extension = image_path
                .extension()
                .and_then(|e| e.to_str())
                .ok_or_else(|| CliError::InvalidFilePath {
                    path: image_path.display().to_string(),
                    message: "No file extension found".to_string(),
                })?;

            let valid_image_formats = ["png", "jpg", "jpeg", "gif", "webp"];
            if !valid_image_formats.contains(&extension.to_lowercase().as_str()) {
                return Err(CliError::UnsupportedFormat {
                    format: extension.to_string(),
                    supported: "png, jpg, jpeg, gif, webp".to_string(),
                });
            }
        }

        if args.verbose {
            println!(
                "{} {} image(s) provided",
                "→".cyan().bold(),
                args.images.len()
            );
        }
    }

    // Validate document path if provided
    if let Some(doc_path) = &args.document {
        if !doc_path.exists() {
            return Err(CliError::InvalidFilePath {
                path: doc_path.display().to_string(),
                message: "Document file does not exist".to_string(),
            });
        }

        // Validate document format
        let extension = doc_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| CliError::InvalidFilePath {
                path: doc_path.display().to_string(),
                message: "No file extension found".to_string(),
            })?;

        let valid_doc_formats = ["pdf", "txt", "md", "markdown"];
        if !valid_doc_formats.contains(&extension.to_lowercase().as_str()) {
            return Err(CliError::UnsupportedFormat {
                format: extension.to_string(),
                supported: "pdf, txt, md, markdown".to_string(),
            });
        }

        if args.verbose {
            println!(
                "{} Document provided: {}",
                "→".cyan().bold(),
                doc_path.display()
            );
        }
    }

    // Load configuration
    let config = load_paladin_config(&args.config)?;

    // Get input - prompt interactively if not provided per FR-6.
    // Cloned rather than moved out of `args.input`: `args` is borrowed again
    // below by `apply_autonomous_config` for the autonomous flag overrides.
    let input = if let Some(input_text) = args.input.clone() {
        input_text
    } else {
        prompt_for_input("Enter input for Paladin")?
    };

    // Load API key from environment variable based on provider
    let env_var_name = match config.provider.provider_type.as_str() {
        "openai" => "OPENAI_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => {
            return Err(CliError::InvalidFieldValue {
                field: "provider.type".to_string(),
                message: format!(
                    "Unknown provider: {}. Supported: openai, deepseek, anthropic",
                    config.provider.provider_type
                ),
            });
        }
    };

    // Check if API key is set
    if std::env::var(env_var_name).is_err() {
        return Err(CliError::MissingApiKey {
            provider: config.provider.provider_type.clone(),
            env_var: env_var_name.to_string(),
        });
    }

    if args.verbose {
        println!(
            "{} Using provider: {}",
            "→".cyan().bold(),
            config.provider.provider_type
        );
        println!("{} Model: {}", "→".cyan().bold(), config.model);
    }

    // Create LLM port adapter using provider factory
    let factory = LlmProviderFactory::new();
    let llm_port: Arc<dyn LlmPort> =
        factory
            .create(&config.provider.provider_type)
            .map_err(|e| CliError::LlmProviderError {
                message: e.to_string(),
            })?;

    // Create circuit breaker for fault tolerance
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        3,                       // failure_threshold
        2,                       // success_threshold
        Duration::from_secs(30), // timeout_duration
    ));

    // Configure garrison if specified in config (Task 5.8)
    let garrison = instantiate_garrison(&config.garrison, &config.name).await?;

    // Configure arsenal/MCP servers if specified in config (Task 5.9)
    let arsenal = instantiate_arsenal(&config.arsenal).await?;

    // Create Paladin execution service
    let service =
        PaladinExecutionService::new(llm_port.clone(), circuit_breaker, garrison, arsenal);

    // Build Paladin from configuration using PaladinBuilder
    let mut builder = PaladinBuilder::new(llm_port)
        .system_prompt(&config.system_prompt)
        .name(&config.name)
        .model(&config.model)
        .temperature(config.temperature)
        .max_loops(config.max_loops.as_u32())
        .timeout_seconds(config.timeout_seconds);

    // Add stop words
    for word in &config.stop_words {
        builder = builder.add_stop_word(word);
    }

    // Apply the YAML `autonomous` baseline, then layer the four CLI flags on
    // top as additive-only overrides: the configuration file supplies the
    // baseline and a present flag forces its feature on, never off (D-05,
    // D-07).
    builder = apply_autonomous_config(builder, &config, &args);
    if args.verbose {
        let enabled_features = autonomous_feature_summary(&config, &args);
        if !enabled_features.is_empty() {
            println!(
                "{} Autonomous features enabled: {}",
                "→".cyan().bold(),
                enabled_features.join(", ")
            );
        }
    }

    // Enable vision if images are provided
    #[cfg(feature = "vision")]
    if !args.images.is_empty() {
        builder = builder.enable_vision(true);
        if args.verbose {
            println!("{} Vision mode enabled", "→".cyan().bold());
        }
    }

    let paladin = builder.build().await?;

    // Process document if provided
    #[allow(unused_mut)]
    let mut combined_input = input.clone();
    #[cfg(feature = "content-processing")]
    {
        if let Some(doc_path) = &args.document {
            if args.verbose {
                println!(
                    "{} Processing document: {}",
                    "→".cyan().bold(),
                    doc_path.display()
                );
            }

            let doc_adapter = DocumentAdapter::new();
            let document = doc_adapter
                .ingest(DocumentSource::File(doc_path.clone()))
                .await
                .map_err(|e| CliError::DocumentProcessingError {
                    message: e.to_string(),
                })?;

            // Extract text from all pages
            let doc_text: String = document
                .pages
                .iter()
                .map(|p| p.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            if args.verbose {
                println!(
                    "{} Document processed: {} pages, {} words",
                    "✓".green().bold(),
                    document.page_count(),
                    document.word_count()
                );
            }

            // Combine input with document text
            combined_input = format!("{}\n\nDocument content:\n{}\n", input, doc_text);
        }
    }
    #[cfg(not(feature = "content-processing"))]
    {
        if args.document.is_some() {
            return Err(CliError::DocumentProcessingError {
                message: "Document processing requires the 'content-processing' feature flag"
                    .to_string(),
            });
        }
    }

    if args.verbose {
        println!("{} Executing Paladin: {}", "→".cyan().bold(), config.name);
        #[cfg(feature = "vision")]
        if !args.images.is_empty() {
            println!(
                "{} Input: {} (with {} image(s))",
                "→".cyan().bold(),
                input,
                args.images.len()
            );
        } else {
            println!("{} Input: {}", "→".cyan().bold(), input);
        }
    }

    // Execute Paladin with vision support if images provided
    let start = std::time::Instant::now();
    let result = {
        #[cfg(feature = "vision")]
        {
            if !args.images.is_empty() {
                // Load images and convert to VisionContent
                let mut vision_contents = Vec::new();
                for image_path in &args.images {
                    if args.verbose {
                        println!("Loading image: {}", image_path.display());
                    }

                    // Create VisionContent from file path
                    let vision_content = VisionContent::ImageFile {
                        path: image_path.clone(),
                        detail: ImageDetail::Auto,
                    };

                    // Validate format
                    vision_content.validate_format().map_err(|e| {
                        CliError::VisionProcessingError {
                            message: e.to_string(),
                        }
                    })?;

                    vision_contents.push(vision_content);
                }

                // Execute with vision
                service
                    .execute_with_vision(&paladin, &combined_input, vision_contents)
                    .await
                    .map_err(|e| CliError::ExecutionError {
                        message: e.to_string(),
                    })?
            } else {
                // Standard execution without vision
                service
                    .execute(&paladin, &combined_input)
                    .await
                    .map_err(|e| CliError::ExecutionError {
                        message: e.to_string(),
                    })?
            }
        }
        #[cfg(not(feature = "vision"))]
        {
            // Standard execution without vision
            service
                .execute(&paladin, &combined_input)
                .await
                .map_err(|e| CliError::ExecutionError {
                    message: e.to_string(),
                })?
        }
    };
    let duration = start.elapsed();

    if args.verbose {
        println!(
            "{} Execution completed in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
        println!(
            "{} Loops: {}, {}",
            "→".cyan().bold(),
            result.loop_count,
            crate::application::cli::formatters::output::format_token_usage_summary(&result.usage)
        );
        println!(
            "{} Stop reason: {:?}",
            "→".cyan().bold(),
            result.stop_reason
        );
    }

    // Handle output
    if let Some(output_path) = args.output {
        // Write JSON to file
        let json_output =
            serde_json::to_string_pretty(&result).map_err(|e| CliError::SerializationError {
                message: e.to_string(),
            })?;
        std::fs::write(&output_path, json_output)?;
        println!(
            "{} Output written to: {}",
            "✓".green().bold(),
            output_path.display()
        );
    } else {
        // Print human-readable output to stdout
        println!("\n{}", "─".repeat(60));
        println!("{}", result.output);
        println!("{}", "─".repeat(60));
    }

    Ok(())
}

/// Applies the YAML `autonomous` section to `builder` as a baseline, then
/// layers the CLI flag overrides on top, additive only (D-05, D-07): the
/// configuration file supplies the baseline and a present flag forces its
/// feature on, never off.
///
/// Order is part of the contract -- baseline first, flags second -- even
/// though a force-on-only override makes the result order-independent for
/// any single feature; keeping the order explicit is what makes the D-07
/// semantics readable at the call site. `handle_agent_run` and this module's
/// tests both drive Paladin construction through this same function so the
/// tests exercise the exact composition the CLI uses.
fn apply_autonomous_config(
    mut builder: PaladinBuilder,
    config: &PaladinYamlConfig,
    args: &AgentRunArgs,
) -> PaladinBuilder {
    // Baseline: the YAML `autonomous` section, if present.
    if let Some(autonomous) = &config.autonomous {
        if autonomous.planning.enabled {
            builder = builder.enable_autonomous_planning(true);
        }
        if autonomous.prompt_generation.enabled {
            builder = builder.enable_autonomous_prompts(true);
        }
        if autonomous.dynamic_temperature.enabled {
            builder = builder.enable_dynamic_temperature(true);
        }
        if let Some(handoffs) = effective_handoffs(Some(&autonomous.handoffs), false) {
            builder = builder.handoff_config(Arc::new(handoffs));
        }
    }

    // Override: a present flag forces its feature on, additive only -- it
    // never resets a feature the configuration file already enabled.
    if args.auto_plan {
        builder = builder.enable_autonomous_planning(true);
    }
    if args.auto_prompt {
        builder = builder.enable_autonomous_prompts(true);
    }
    if args.dynamic_temp {
        builder = builder.enable_dynamic_temperature(true);
    }
    if args.enable_handoffs {
        let yaml_handoffs = config.autonomous.as_ref().map(|a| &a.handoffs);
        if let Some(handoffs) = effective_handoffs(yaml_handoffs, true) {
            builder = builder.handoff_config(Arc::new(handoffs));
        }
    }

    builder
}

/// Computes the effective `HandoffConfig` to hand to the builder's handoff
/// setter, given an optional YAML-sourced baseline and whether
/// `force_enabled` (the `--enable-handoffs` flag) forces the feature on.
///
/// `force_enabled == false` returns the YAML baseline only when the operator
/// enabled it there (or `None` when there is nothing to apply).
/// `force_enabled == true` always returns a config with `enabled` forced
/// `true`, preserving every other field the YAML block set (`strategy`,
/// `max_depth`, `retry`) -- or `HandoffConfig::default()`'s fields when no
/// YAML block exists. The flag never resets a field the operator configured
/// (D-07) -- this is what "additive only" means for the one non-boolean
/// autonomous feature.
fn effective_handoffs(
    yaml_handoffs: Option<&HandoffConfig>,
    force_enabled: bool,
) -> Option<HandoffConfig> {
    if force_enabled {
        let mut handoffs = yaml_handoffs.cloned().unwrap_or_default();
        handoffs.enabled = true;
        Some(handoffs)
    } else {
        yaml_handoffs.filter(|handoffs| handoffs.enabled).cloned()
    }
}

/// Names the autonomous features that end up enabled once the YAML baseline
/// and the CLI flag overrides are both accounted for, for `--verbose`
/// reporting. Mirrors `apply_autonomous_config`'s "YAML OR flag" logic
/// without touching the builder.
fn autonomous_feature_summary(
    config: &PaladinYamlConfig,
    args: &AgentRunArgs,
) -> Vec<&'static str> {
    let autonomous = config.autonomous.as_ref();
    let mut enabled = Vec::new();

    if autonomous.is_some_and(|a| a.planning.enabled) || args.auto_plan {
        enabled.push("planning");
    }
    if autonomous.is_some_and(|a| a.prompt_generation.enabled) || args.auto_prompt {
        enabled.push("prompt-generation");
    }
    if autonomous.is_some_and(|a| a.dynamic_temperature.enabled) || args.dynamic_temp {
        enabled.push("dynamic-temperature");
    }
    if autonomous.is_some_and(|a| a.handoffs.enabled) || args.enable_handoffs {
        enabled.push("handoffs");
    }

    enabled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::cli::config::paladin_config::ProviderConfig;
    use crate::core::platform::container::autonomous_config::{
        AutonomousConfig, PlanningConfig, PromptConfig, TemperatureConfig,
    };
    use crate::core::platform::container::paladin::{MaxLoops, PaladinData};
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_ports::output::llm_port::LlmPort;
    use std::fs;
    use tempfile::TempDir;

    /// Builds a minimal, otherwise-valid `PaladinYamlConfig` fixture, with
    /// `autonomous` set to the given value. Every other field is the
    /// smallest valid value so tests can focus on the `autonomous` wiring
    /// alone.
    fn make_yaml_config(autonomous: Option<AutonomousConfig>) -> PaladinYamlConfig {
        PaladinYamlConfig {
            name: "test-paladin".to_string(),
            system_prompt: "You are a helpful assistant".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(3),
            timeout_seconds: 300,
            stop_words: vec![],
            provider: ProviderConfig {
                provider_type: "openai".to_string(),
            },
            garrison: None,
            arsenal: None,
            autonomous,
            vision_enabled: false,
            images: vec![],
            documents: vec![],
        }
    }

    /// Builds an `AgentRunArgs` fixture with only the four autonomous flags
    /// varying; every other field is a harmless default.
    fn make_args(
        auto_plan: bool,
        auto_prompt: bool,
        dynamic_temp: bool,
        enable_handoffs: bool,
    ) -> AgentRunArgs {
        AgentRunArgs {
            config: PathBuf::from("config.yaml"),
            input: None,
            output: None,
            verbose: false,
            #[cfg(feature = "vision")]
            images: vec![],
            document: None,
            auto_plan,
            auto_prompt,
            dynamic_temp,
            enable_handoffs,
        }
    }

    /// Drives Paladin construction through the exact same `apply_autonomous_config`
    /// composition `handle_agent_run` uses, against a `MockLlmAdapter`, and
    /// returns the resulting `PaladinData` for field assertions.
    async fn build_paladin_with(
        autonomous: Option<AutonomousConfig>,
        args: AgentRunArgs,
    ) -> PaladinData {
        let config = make_yaml_config(autonomous);
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let builder = PaladinBuilder::new(llm_port).system_prompt(&config.system_prompt);
        let builder = apply_autonomous_config(builder, &config, &args);
        builder
            .build()
            .await
            .expect("builder composition under test should always succeed")
            .node
    }

    #[test]
    fn test_agent_new_args_creation() {
        let args = AgentNewArgs {
            name: "test-agent".to_string(),
            output: PathBuf::from("test.yaml"),
            provider: Some("openai".to_string()),
        };

        assert_eq!(args.name, "test-agent");
        assert_eq!(args.output, PathBuf::from("test.yaml"));
        assert_eq!(args.provider, Some("openai".to_string()));
    }

    #[test]
    fn test_agent_run_args_creation() {
        let args = AgentRunArgs {
            config: PathBuf::from("config.yaml"),
            input: Some("test input".to_string()),
            output: Some(PathBuf::from("output.json")),
            verbose: true,
            #[cfg(feature = "vision")]
            images: vec![],
            document: None,
            auto_plan: false,
            auto_prompt: false,
            dynamic_temp: false,
            enable_handoffs: false,
        };

        assert_eq!(args.config, PathBuf::from("config.yaml"));
        assert_eq!(args.input, Some("test input".to_string()));
        assert_eq!(args.output, Some(PathBuf::from("output.json")));
        assert!(args.verbose);
        #[cfg(feature = "vision")]
        assert!(args.images.is_empty());
        assert!(args.document.is_none());
    }

    #[test]
    #[cfg(feature = "vision")]
    fn test_agent_run_args_with_images() {
        let args = AgentRunArgs {
            config: PathBuf::from("config.yaml"),
            input: Some("analyze these".to_string()),
            output: None,
            verbose: false,
            #[cfg(feature = "vision")]
            images: vec![PathBuf::from("image1.png"), PathBuf::from("image2.jpg")],
            document: None,
            auto_plan: false,
            auto_prompt: false,
            dynamic_temp: false,
            enable_handoffs: false,
        };

        assert_eq!(args.images.len(), 2);
        assert_eq!(args.images[0], PathBuf::from("image1.png"));
        assert_eq!(args.images[1], PathBuf::from("image2.jpg"));
    }

    #[test]
    fn test_agent_run_args_with_document() {
        let args = AgentRunArgs {
            config: PathBuf::from("config.yaml"),
            input: Some("summarize this".to_string()),
            output: None,
            verbose: false,
            #[cfg(feature = "vision")]
            images: vec![],
            document: Some(PathBuf::from("document.pdf")),
            auto_plan: false,
            auto_prompt: false,
            dynamic_temp: false,
            enable_handoffs: false,
        };

        assert_eq!(args.document, Some(PathBuf::from("document.pdf")));
    }

    #[test]
    fn test_handle_agent_new_success() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("agent.yaml");

        let args = AgentNewArgs {
            name: "test-paladin".to_string(),
            output: output_path.clone(),
            provider: Some("openai".to_string()),
        };

        let result = handle_agent_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify file content contains expected elements
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("test-paladin"));
        assert!(content.contains("openai"));
    }

    #[test]
    fn test_handle_agent_new_default_provider() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("agent.yaml");

        let args = AgentNewArgs {
            name: "test-paladin".to_string(),
            output: output_path.clone(),
            provider: None, // Default to openai
        };

        let result = handle_agent_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("openai"));
    }

    #[test]
    fn test_handle_agent_new_invalid_provider() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("agent.yaml");

        let args = AgentNewArgs {
            name: "test-paladin".to_string(),
            output: output_path.clone(),
            provider: Some("invalid_provider".to_string()),
        };

        let result = handle_agent_new(args);
        assert!(result.is_err());

        match result {
            Err(CliError::InvalidFieldValue { field, message }) => {
                assert_eq!(field, "provider");
                assert!(message.contains("invalid_provider"));
            }
            _ => panic!("Expected InvalidFieldValue error"),
        }
    }

    #[test]
    fn test_handle_agent_new_deepseek_provider() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("agent.yaml");

        let args = AgentNewArgs {
            name: "deepseek-paladin".to_string(),
            output: output_path.clone(),
            provider: Some("deepseek".to_string()),
        };

        let result = handle_agent_new(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("deepseek"));
    }

    #[test]
    fn test_handle_agent_new_anthropic_provider() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("agent.yaml");

        let args = AgentNewArgs {
            name: "anthropic-paladin".to_string(),
            output: output_path.clone(),
            provider: Some("anthropic".to_string()),
        };

        let result = handle_agent_new(args);
        assert!(result.is_ok());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("anthropic"));
    }

    #[test]
    fn test_handle_agent_new_file_write_error() {
        // Try to write to an invalid path (directory that doesn't exist)
        let invalid_path = PathBuf::from("/nonexistent/directory/agent.yaml");

        let args = AgentNewArgs {
            name: "test-paladin".to_string(),
            output: invalid_path,
            provider: Some("openai".to_string()),
        };

        let result = handle_agent_new(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_commands_enum_new_variant() {
        let new_args = AgentNewArgs {
            name: "test".to_string(),
            output: PathBuf::from("test.yaml"),
            provider: None,
        };
        let command = AgentCommands::New(new_args);

        match command {
            AgentCommands::New(args) => {
                assert_eq!(args.name, "test");
            }
            _ => panic!("Expected New variant"),
        }
    }

    #[test]
    fn test_agent_commands_enum_run_variant() {
        let run_args = AgentRunArgs {
            config: PathBuf::from("config.yaml"),
            input: None,
            output: None,
            verbose: false,
            #[cfg(feature = "vision")]
            images: vec![],
            document: None,
            auto_plan: false,
            auto_prompt: false,
            dynamic_temp: false,
            enable_handoffs: false,
        };
        let command = AgentCommands::Run(run_args);

        match command {
            AgentCommands::Run(args) => {
                assert_eq!(args.config, PathBuf::from("config.yaml"));
            }
            _ => panic!("Expected Run variant"),
        }
    }

    // ------------------------------------------------------------------
    // D-05/D-06/D-07: autonomous YAML section + CLI flag override wiring
    // (CLOSE-02, Epic 14 cluster 8.0)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_autonomous_planning_from_yaml_reaches_paladin_data() {
        // A config whose `autonomous.planning.enabled` is true, with
        // `auto_plan` false, yields `PaladinData.autonomous_planning == true`
        // -- the YAML baseline alone reaches the built Paladin.
        let autonomous = Some(AutonomousConfig {
            planning: PlanningConfig {
                enabled: true,
                max_subtasks: 10,
            },
            ..Default::default()
        });
        let args = make_args(false, false, false, false);

        let data = build_paladin_with(autonomous, args).await;

        assert!(data.autonomous_planning);
    }

    #[tokio::test]
    async fn test_auto_plan_flag_forces_planning_on() {
        // A bare `--auto-plan`, with no `autonomous` section at all, yields
        // `PaladinData.autonomous_planning == true` -- the flag alone
        // reaches the built Paladin.
        let args = make_args(true, false, false, false);

        let data = build_paladin_with(None, args).await;

        assert!(data.autonomous_planning);
    }

    /// Edge row CLOSE-02/adjacency: every YAML-value x flag-presence
    /// combination for one feature, in the order `must_haves.truths`
    /// enumerates them.
    fn adjacency_matrix() -> [(Option<bool>, bool, bool); 6] {
        [
            (Some(true), false, true),   // YAML true, flag absent -> stays on
            (Some(true), true, true),    // YAML true, flag present -> stays on
            (Some(false), true, true),   // YAML false, flag present -> turns on
            (Some(false), false, false), // YAML false, flag absent -> stays off
            (None, true, true),          // YAML section absent, flag present -> turns on
            (None, false, false),        // YAML section absent, flag absent -> stays off
        ]
    }

    #[tokio::test]
    async fn test_autonomous_prompts_yaml_and_flag() {
        for (yaml_enabled, flag, expected) in adjacency_matrix() {
            let autonomous = yaml_enabled.map(|enabled| AutonomousConfig {
                prompt_generation: PromptConfig {
                    enabled,
                    description: None,
                },
                ..Default::default()
            });
            let args = make_args(false, flag, false, false);

            let data = build_paladin_with(autonomous, args).await;

            assert_eq!(
                data.autonomous_prompts, expected,
                "yaml_enabled={yaml_enabled:?} flag={flag} expected={expected}"
            );
        }
    }

    #[tokio::test]
    async fn test_dynamic_temperature_yaml_and_flag() {
        for (yaml_enabled, flag, expected) in adjacency_matrix() {
            let autonomous = yaml_enabled.map(|enabled| AutonomousConfig {
                dynamic_temperature: TemperatureConfig {
                    enabled,
                    ..Default::default()
                },
                ..Default::default()
            });
            let args = make_args(false, false, flag, false);

            let data = build_paladin_with(autonomous, args).await;

            assert_eq!(
                data.dynamic_temperature, expected,
                "yaml_enabled={yaml_enabled:?} flag={flag} expected={expected}"
            );
        }
    }

    #[test]
    fn test_handoffs_yaml_and_flag() {
        // `PaladinData` carries no handoff field (the domain-level surface
        // stops at `PaladinBuilder`, which exposes no getter for its private
        // handoff-configuration field -- D-08 forbids touching that file).
        // This test therefore exercises `effective_handoffs` directly: the
        // pure function `apply_autonomous_config` calls to compute the exact
        // value it hands to the builder's handoff setter in both the
        // baseline and override blocks.

        // A YAML `autonomous.handoffs.enabled: true` with a non-default
        // `max_depth` is preserved as the baseline (no flag).
        let yaml_handoffs = HandoffConfig {
            enabled: true,
            max_depth: 9,
            ..HandoffConfig::default()
        };
        assert_eq!(
            effective_handoffs(Some(&yaml_handoffs), false),
            Some(yaml_handoffs.clone())
        );

        // `--enable-handoffs` with no YAML `autonomous` section produces a
        // `HandoffConfig` with `enabled` true and every other field at its
        // `Default`.
        assert_eq!(
            effective_handoffs(None, true),
            Some(HandoffConfig {
                enabled: true,
                ..HandoffConfig::default()
            })
        );

        // `--enable-handoffs` on top of a YAML `handoffs` block preserves
        // that block's non-`enabled` fields (max_depth stays 9) and forces
        // `enabled` true.
        let disabled_yaml_handoffs = HandoffConfig {
            enabled: false,
            max_depth: 9,
            ..HandoffConfig::default()
        };
        assert_eq!(
            effective_handoffs(Some(&disabled_yaml_handoffs), true),
            Some(HandoffConfig {
                enabled: true,
                max_depth: 9,
                ..HandoffConfig::default()
            })
        );
    }

    /// Edge row CLOSE-02/ordering: applying a flag twice is idempotent, and
    /// setting any subset of the four flags leaves the other three
    /// features' `PaladinData` fields untouched.
    #[tokio::test]
    async fn test_autonomous_flag_application_is_idempotent_and_independent() {
        let config = make_yaml_config(None);
        let args = make_args(true, false, false, false);
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());

        let builder_once =
            PaladinBuilder::new(llm_port.clone()).system_prompt(&config.system_prompt);
        let builder_once = apply_autonomous_config(builder_once, &config, &args);
        let data_once = builder_once.build().await.expect("build once").node;

        let builder_twice = PaladinBuilder::new(llm_port).system_prompt(&config.system_prompt);
        let builder_twice = apply_autonomous_config(builder_twice, &config, &args);
        let builder_twice = apply_autonomous_config(builder_twice, &config, &args);
        let data_twice = builder_twice.build().await.expect("build twice").node;

        assert_eq!(
            data_once.autonomous_planning,
            data_twice.autonomous_planning
        );
        assert_eq!(data_once.autonomous_prompts, data_twice.autonomous_prompts);
        assert_eq!(
            data_once.dynamic_temperature,
            data_twice.dynamic_temperature
        );

        // Planning-only leaves prompts/dynamic-temperature untouched.
        let data = build_paladin_with(None, make_args(true, false, false, false)).await;
        assert!(data.autonomous_planning);
        assert!(!data.autonomous_prompts);
        assert!(!data.dynamic_temperature);

        // Prompts-only leaves planning/dynamic-temperature untouched.
        let data = build_paladin_with(None, make_args(false, true, false, false)).await;
        assert!(!data.autonomous_planning);
        assert!(data.autonomous_prompts);
        assert!(!data.dynamic_temperature);

        // Dynamic-temperature-only leaves planning/prompts untouched.
        let data = build_paladin_with(None, make_args(false, false, true, false)).await;
        assert!(!data.autonomous_planning);
        assert!(!data.autonomous_prompts);
        assert!(data.dynamic_temperature);
    }

    #[tokio::test]
    async fn test_no_autonomous_section_and_no_flags_is_a_no_op() {
        // A config with `autonomous` `None` and all four flags false yields
        // every `PaladinData` autonomous field false, and no handoff
        // configuration.
        let data = build_paladin_with(None, make_args(false, false, false, false)).await;

        assert!(!data.autonomous_planning);
        assert!(!data.autonomous_prompts);
        assert!(!data.dynamic_temperature);
        assert_eq!(effective_handoffs(None, false), None);
    }

    #[tokio::test]
    async fn test_yaml_enabled_feature_cannot_be_disabled_from_cli() {
        // With `autonomous.planning.enabled: true` in YAML, no combination
        // of the four flags produces `PaladinData.autonomous_planning ==
        // false` -- the stated, deliberate consequence of D-07.
        let autonomous = Some(AutonomousConfig {
            planning: PlanningConfig {
                enabled: true,
                max_subtasks: 10,
            },
            ..Default::default()
        });

        for auto_plan in [false, true] {
            for auto_prompt in [false, true] {
                for dynamic_temp in [false, true] {
                    for enable_handoffs in [false, true] {
                        let args = make_args(auto_plan, auto_prompt, dynamic_temp, enable_handoffs);
                        let data = build_paladin_with(autonomous.clone(), args).await;
                        assert!(
                            data.autonomous_planning,
                            "flags ({auto_plan}, {auto_prompt}, {dynamic_temp}, \
                             {enable_handoffs}) must not disable a YAML-enabled feature"
                        );
                    }
                }
            }
        }
    }
}
