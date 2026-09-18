//! Battalion command implementations for multi-agent orchestration
//!
//! This module provides CLI commands for creating and running Battalion workflows:
//! - **Formation**: Sequential execution (output N → input N+1)
//! - **Phalanx**: Concurrent execution with result aggregation
//! - **Campaign**: Graph/DAG-based conditional routing
//! - **Chain of Command**: Hierarchical delegation with commander
//!
//! # Examples
//!
//! ```bash
//! # Create a Formation (sequential pipeline)
//! paladin battalion new my-pipeline --type formation --output pipeline.yaml
//!
//! # Create a Phalanx (parallel execution)
//! paladin battalion new my-phalanx --type phalanx --output phalanx.yaml
//!
//! # Run a Battalion workflow
//! paladin battalion run --config pipeline.yaml --type formation
//! ```

use crate::application::cli::error::CliError;
use crate::application::cli::templates::battalion_template::generate_battalion_template;
use clap::Subcommand;
use colored::Colorize;
use std::path::PathBuf;
use std::sync::Arc;

/// Battalion subcommands for multi-agent orchestration
#[derive(Debug, Subcommand)]
pub enum BattalionCommands {
    /// Create a new Battalion configuration template
    New(BattalionNewArgs),
    /// Run a Battalion workflow
    Run(BattalionRunArgs),
}

/// Arguments for creating a new Battalion template
#[derive(Debug, clap::Args)]
pub struct BattalionNewArgs {
    /// Name for the Battalion
    #[arg(short, long)]
    pub name: String,

    /// Battalion type (formation, phalanx, campaign, chain-of-command)
    #[arg(short, long)]
    pub r#type: String,

    /// Output path for the template file
    #[arg(short, long)]
    pub output: PathBuf,
}

/// Arguments for executing a Battalion workflow
#[derive(Debug, clap::Args)]
pub struct BattalionRunArgs {
    /// Path to Battalion YAML configuration file
    #[arg(short, long)]
    pub config: PathBuf,

    /// Battalion type (formation, phalanx, campaign, chain-of-command)
    #[arg(short, long)]
    pub r#type: String,

    /// Path to save output file (prints to stdout if not provided)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Handle the `paladin battalion new` command
///
/// Creates a new Battalion configuration template file with documented options
pub fn handle_battalion_new(args: BattalionNewArgs) -> Result<(), CliError> {
    use crate::application::cli::interactive::confirm;

    // Validate battalion type
    let valid_types = [
        "formation",
        "phalanx",
        "campaign",
        "chain-of-command",
        "conclave",
        "maneuver",
    ];
    if !valid_types.contains(&args.r#type.as_str()) {
        return Err(CliError::InvalidFieldValue {
            field: "type".to_string(),
            message: format!(
                "must be one of: {}. Got: {}",
                valid_types.join(", "),
                args.r#type
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
    let template = generate_battalion_template(&args.name, &args.r#type)?;

    // Write to file
    std::fs::write(&args.output, template)?;

    // Print success message with colored output
    println!(
        "{} Created Battalion template: {}",
        "✓".green().bold(),
        args.output.display()
    );

    Ok(())
}

/// Handle the `paladin battalion run` command
///
/// Loads a Battalion configuration and executes it with appropriate orchestration
pub async fn handle_battalion_run(args: BattalionRunArgs) -> Result<(), CliError> {
    use crate::application::cli::config::battalion_config::BattalionYamlConfig;
    use crate::application::cli::config::loader::load_battalion_config;

    // Load battalion configuration
    let battalion_config = load_battalion_config(&args.config)?;

    // Validate config type matches the type argument
    let config_type = match &battalion_config {
        BattalionYamlConfig::Formation(_) => "formation",
        BattalionYamlConfig::Phalanx(_) => "phalanx",
        BattalionYamlConfig::Campaign(_) => "campaign",
        BattalionYamlConfig::ChainOfCommand(_) => "chain-of-command",
        BattalionYamlConfig::Conclave(_) => "conclave",
        BattalionYamlConfig::Maneuver(_) => "maneuver",
    };

    if config_type != args.r#type {
        return Err(CliError::ValidationError {
            message: format!(
                "Configuration type mismatch: expected {}, but config file contains {}",
                args.r#type, config_type
            ),
        });
    }

    if args.verbose {
        println!(
            "{} Loading {} battalion: {}",
            "→".cyan().bold(),
            config_type,
            args.config.display()
        );
    }

    // Execute based on battalion type
    match battalion_config {
        BattalionYamlConfig::Formation(config) => {
            execute_formation(config, args.verbose, args.output).await
        }
        BattalionYamlConfig::Phalanx(config) => {
            execute_phalanx(config, args.verbose, args.output).await
        }
        BattalionYamlConfig::Campaign(_config) => {
            // Campaign execution - to be implemented
            Err(CliError::Other(
                "Campaign execution not yet implemented in CLI. See Task 7.0".to_string(),
            ))
        }
        BattalionYamlConfig::ChainOfCommand(_config) => {
            // Chain of Command execution - to be implemented
            Err(CliError::Other(
                "Chain of Command execution not yet implemented in CLI. See Task 7.0".to_string(),
            ))
        }
        BattalionYamlConfig::Conclave(config) => {
            execute_conclave(config, args.verbose, args.output).await
        }
        BattalionYamlConfig::Maneuver(config) => {
            execute_maneuver(config, args.verbose, args.output).await
        }
    }
}

/// Simple adapter to make PaladinExecutionService compatible with PaladinPort
struct PaladinExecutionAdapter {
    service: Arc<
        crate::application::services::paladin::paladin_execution_service::PaladinExecutionService,
    >,
}

#[async_trait::async_trait]
impl paladin_ports::output::paladin_port::PaladinPort for PaladinExecutionAdapter {
    async fn execute(
        &self,
        paladin: &crate::core::platform::container::paladin::Paladin,
        input: &str,
    ) -> Result<
        paladin_ports::output::paladin_port::PaladinResult,
        crate::application::services::paladin::error::PaladinError,
    > {
        self.service.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        _paladin: &crate::core::platform::container::paladin::Paladin,
        _input: &str,
    ) -> Result<
        paladin_ports::output::paladin_port::PaladinStream,
        crate::application::services::paladin::error::PaladinError,
    > {
        Err(
            crate::application::services::paladin::error::PaladinError::ExecutionError(
                "Streaming not supported in CLI adapter".to_string(),
            ),
        )
    }

    fn validate(
        &self,
        _paladin: &crate::core::platform::container::paladin::Paladin,
    ) -> Result<(), crate::application::services::paladin::error::PaladinError> {
        Ok(())
    }
}

/// Execute a Formation battalion
async fn execute_formation(
    config: crate::application::cli::config::battalion_config::FormationConfig,
    verbose: bool,
    output: Option<PathBuf>,
) -> Result<(), CliError> {
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::platform::container::battalion::BattalionConfig;
    use crate::core::platform::container::battalion::formation::Formation;
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_ports::output::paladin_port::PaladinPort;
    use std::sync::Arc;
    use std::time::Duration;

    if verbose {
        println!("{} Building Formation: {}", "→".cyan().bold(), config.name);
        println!("{} Paladins: {}", "→".cyan().bold(), config.paladins.len());
    }

    // Build all Paladins
    let mut paladins = Vec::new();
    for (idx, paladin_ref) in config.paladins.iter().enumerate() {
        let paladin = build_paladin_from_reference(paladin_ref, verbose, idx + 1).await?;
        paladins.push(paladin);
    }

    // Create Battalion configuration
    let battalion_config = BattalionConfig::new(&config.name)
        .with_timeout(600) // Default 10 minute timeout
        .with_description(format!("Formation with {} Paladins", paladins.len()));

    // Create Formation
    let formation =
        Formation::new(paladins, battalion_config).map_err(|e| CliError::BattalionError {
            message: e.to_string(),
        })?;

    if verbose {
        println!("{} Formation built successfully", "✓".green().bold());
    }

    // Create Paladin execution service with circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    // Create a dummy LLM port (each Paladin already has its own configured)
    use paladin_llm::provider_factory::LlmProviderFactory;
    let factory = LlmProviderFactory::new();
    let dummy_llm_port = factory
        .create("openai")
        .map_err(|e| CliError::LlmProviderError {
            message: format!("Failed to create LLM port: {}", e),
        })?;

    let paladin_service = Arc::new(PaladinExecutionService::new(
        dummy_llm_port,
        circuit_breaker,
        None,
        None,
    ));

    // Wrap in adapter
    let paladin_port = Arc::new(PaladinExecutionAdapter {
        service: paladin_service,
    });

    // Create Formation execution service
    let formation_service =
        crate::application::services::battalion::formation_service::FormationExecutionService::new(
            paladin_port as Arc<dyn PaladinPort>,
        );

    // Get input from user
    println!("\n{} Enter input for Formation:", "?".cyan().bold());
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let input = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| CliError::Other("No input provided".to_string()))?
        .map_err(|e| CliError::IoError {
            message: "Failed to read input".to_string(),
            source: e,
        })?;

    if verbose {
        println!(
            "{} Executing Formation with input: {}",
            "→".cyan().bold(),
            input
        );
    }

    // Execute Formation
    let start = std::time::Instant::now();
    let result = formation_service
        .execute(&formation, &input)
        .await
        .map_err(|e| CliError::BattalionError {
            message: e.to_string(),
        })?;
    let duration = start.elapsed();

    if verbose {
        println!(
            "{} Formation completed in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
        println!(
            "{} Paladin executions: {}",
            "→".cyan().bold(),
            result.paladin_results.len()
        );
    }

    // Handle output
    if let Some(output_path) = output {
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
        println!("\n{}", "═".repeat(80));
        println!("{} Formation Result", "📊".cyan());
        println!("{}", "═".repeat(80));
        println!("\n{} Final Output:", "→".cyan().bold());
        println!("{}\n", result.final_output);

        if verbose {
            println!("{} Individual Paladin Outputs:", "→".cyan().bold());
            for (idx, paladin_result) in result.paladin_results.iter().enumerate() {
                println!(
                    "\n  {}. Paladin {} ({} loops, {}):",
                    idx + 1,
                    idx + 1,
                    paladin_result.loop_count,
                    crate::application::cli::formatters::output::format_token_usage_summary(
                        &paladin_result.usage
                    )
                );
                println!(
                    "     {}",
                    paladin_result.output.lines().next().unwrap_or("")
                );
            }
        }

        println!("\n{}", "═".repeat(80));
    }

    Ok(())
}

/// Execute a Phalanx battalion
async fn execute_phalanx(
    config: crate::application::cli::config::battalion_config::PhalanxConfig,
    verbose: bool,
    output: Option<PathBuf>,
) -> Result<(), CliError> {
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::platform::container::battalion::BattalionConfig;
    use crate::core::platform::container::battalion::phalanx::Phalanx;
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_ports::output::paladin_port::PaladinPort;
    use std::sync::Arc;
    use std::time::Duration;

    if verbose {
        println!("{} Building Phalanx: {}", "→".cyan().bold(), config.name);
        println!(
            "{} Paladins: {} (parallel)",
            "→".cyan().bold(),
            config.paladins.len()
        );
    }

    // Build all Paladins
    let mut paladins = Vec::new();
    for (idx, paladin_ref) in config.paladins.iter().enumerate() {
        let paladin = build_paladin_from_reference(paladin_ref, verbose, idx + 1).await?;
        paladins.push(paladin);
    }

    // Create Battalion configuration
    let battalion_config = BattalionConfig::new(&config.name)
        .with_timeout(600)
        .with_description(format!("Phalanx with {} parallel Paladins", paladins.len()));

    // Create Phalanx
    let phalanx =
        Phalanx::new(paladins, battalion_config).map_err(|e| CliError::BattalionError {
            message: e.to_string(),
        })?;

    if verbose {
        println!("{} Phalanx built successfully", "✓".green().bold());
    }

    // Create Paladin execution service
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
    use paladin_llm::provider_factory::LlmProviderFactory;
    let factory = LlmProviderFactory::new();
    let dummy_llm_port = factory
        .create("openai")
        .map_err(|e| CliError::LlmProviderError {
            message: format!("Failed to create LLM port: {}", e),
        })?;

    let paladin_service = Arc::new(PaladinExecutionService::new(
        dummy_llm_port,
        circuit_breaker,
        None,
        None,
    ));

    // Wrap in adapter
    let paladin_port = Arc::new(PaladinExecutionAdapter {
        service: paladin_service,
    });

    // Create Phalanx execution service
    let phalanx_service =
        crate::application::services::battalion::phalanx_service::PhalanxExecutionService::new(
            paladin_port as Arc<dyn PaladinPort>,
        );

    // Get input from user
    println!("\n{} Enter input for Phalanx:", "?".cyan().bold());
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let input = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| CliError::Other("No input provided".to_string()))?
        .map_err(|e| CliError::IoError {
            message: "Failed to read input".to_string(),
            source: e,
        })?;

    if verbose {
        println!(
            "{} Executing Phalanx with {} parallel Paladins",
            "→".cyan().bold(),
            phalanx.paladins().len()
        );
    }

    // Execute Phalanx
    let start = std::time::Instant::now();
    let result = phalanx_service
        .execute(&phalanx, &input)
        .await
        .map_err(|e| CliError::BattalionError {
            message: e.to_string(),
        })?;
    let duration = start.elapsed();

    if verbose {
        println!(
            "{} Phalanx completed in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
    }

    // Handle output
    if let Some(output_path) = output {
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
        println!("\n{}", "═".repeat(80));
        println!("{} Phalanx Results (Parallel Execution)", "📊".cyan());
        println!("{}", "═".repeat(80));

        for (idx, paladin_result) in result.paladin_results.iter().enumerate() {
            println!(
                "\n{} Paladin {}:",
                format!("{}.", idx + 1).cyan().bold(),
                idx + 1
            );
            println!(
                "   Loops: {}, {}",
                paladin_result.loop_count,
                crate::application::cli::formatters::output::format_token_usage_summary(
                    &paladin_result.usage
                )
            );
            println!("   {}\n", "─".repeat(76));
            println!("   {}", paladin_result.output);
        }

        println!("\n{}", "═".repeat(80));
    }

    Ok(())
}

/// Execute a Conclave battalion (Mixture of Agents pattern)
async fn execute_conclave(
    config: crate::application::cli::config::battalion_config::ConclaveConfig,
    verbose: bool,
    output: Option<PathBuf>,
) -> Result<(), CliError> {
    use crate::application::services::battalion::conclave_execution_service::ConclaveExecutionService;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::platform::container::battalion::BattalionConfig;
    use crate::core::platform::container::battalion::conclave::{
        Conclave, ConclaveConfig as DomainConclaveConfig, ObservabilityLevel,
    };
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_ports::output::paladin_port::PaladinPort;
    use std::sync::Arc;
    use std::time::Duration;

    if verbose {
        println!("{} Building Conclave: {}", "→".cyan().bold(), config.name);
        println!("{} Experts: {}", "→".cyan().bold(), config.experts.len());
        println!("{} Aggregator: configured", "→".cyan().bold());
    }

    // Build expert Paladins
    let mut experts = Vec::new();
    for (idx, expert_ref) in config.experts.iter().enumerate() {
        let expert = build_paladin_from_reference(expert_ref, verbose, idx + 1).await?;
        experts.push(expert);
    }

    // Build aggregator Paladin
    let aggregator = build_paladin_from_reference(&config.aggregator, verbose, 0).await?;

    if verbose {
        println!("{} All Paladins built successfully", "✓".green().bold());
    }

    // Create Battalion configuration
    let battalion_config = BattalionConfig::new(&config.name)
        .with_timeout(config.timeout_seconds.unwrap_or(300))
        .with_description(format!("Conclave with {} experts", experts.len()));

    // Parse observability level
    let observability = match config.observability_level.as_deref() {
        Some("minimal") => ObservabilityLevel::Minimal,
        Some("verbose") => ObservabilityLevel::Verbose,
        _ => ObservabilityLevel::Standard,
    };

    // Create Conclave domain configuration
    let mut conclave_config = DomainConclaveConfig::new(&config.name, battalion_config)
        .with_timeout(config.timeout_seconds.unwrap_or(300))
        .with_retry_attempts(config.retry_attempts.unwrap_or(2))
        .with_observability(observability);

    if let Some(ref synthesis_prompt) = config.synthesis_prompt {
        conclave_config = conclave_config.with_synthesis_prompt(synthesis_prompt);
    }

    if let Some(include_names) = config.include_expert_names {
        conclave_config = conclave_config.with_expert_names(include_names);
    }

    if let Some(max_tokens) = config.max_expert_output_tokens {
        conclave_config = conclave_config.with_max_expert_tokens(max_tokens);
    }

    // Create Conclave
    let conclave = Conclave::new(experts, aggregator, conclave_config).map_err(|e| {
        CliError::BattalionError {
            message: e.to_string(),
        }
    })?;

    if verbose {
        println!("{} Conclave built successfully", "✓".green().bold());
    }

    // Create Paladin execution service with circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    use paladin_llm::provider_factory::LlmProviderFactory;
    let factory = LlmProviderFactory::new();
    let dummy_llm_port = factory
        .create("openai")
        .map_err(|e| CliError::LlmProviderError {
            message: format!("Failed to create LLM port: {}", e),
        })?;

    let paladin_service = Arc::new(PaladinExecutionService::new(
        dummy_llm_port,
        circuit_breaker,
        None,
        None,
    ));

    let paladin_port = Arc::new(PaladinExecutionAdapter {
        service: paladin_service,
    });

    // Create Conclave execution service
    let conclave_service = ConclaveExecutionService::new(paladin_port as Arc<dyn PaladinPort>);

    // Get input from user
    println!("\n{} Enter task for expert analysis:", "?".cyan().bold());
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let input = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| CliError::Other("No input provided".to_string()))?
        .map_err(|e| CliError::IoError {
            message: "Failed to read input".to_string(),
            source: e,
        })?;

    if verbose {
        println!(
            "{} Executing Conclave with input: {}",
            "→".cyan().bold(),
            input
        );
    }

    // Execute Conclave
    let start = std::time::Instant::now();
    let result = conclave_service
        .execute(&conclave, &input)
        .await
        .map_err(|e| CliError::BattalionError {
            message: e.to_string(),
        })?;
    let duration = start.elapsed();

    if verbose {
        println!(
            "{} Conclave completed in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
        println!(
            "{} Successful experts: {}/{}",
            "→".cyan().bold(),
            result.successful_expert_count(),
            conclave.expert_count()
        );
    }

    // Handle output
    if let Some(output_path) = output {
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
        println!("\n{}", "═".repeat(80));
        println!("{} Conclave Result - Synthesized Analysis", "📊".cyan());
        println!("{}", "═".repeat(80));
        println!("\n{} Aggregated Output:", "→".cyan().bold());
        println!("{}\n", result.aggregated_output.output);

        if verbose {
            println!("{} Individual Expert Outputs:", "→".cyan().bold());
            for (expert_name, expert_result) in result.expert_outputs.iter() {
                println!(
                    "\n  {} {} ({} loops, {}):",
                    "→".cyan(),
                    expert_name,
                    expert_result.loop_count,
                    crate::application::cli::formatters::output::format_token_usage_summary(
                        &expert_result.usage
                    )
                );
                println!("   {}", "─".repeat(76));
                // Print first few lines
                for (idx, line) in expert_result.output.lines().take(5).enumerate() {
                    println!("   {}", line);
                    if idx == 4 && expert_result.output.lines().count() > 5 {
                        println!("   ... (truncated)");
                    }
                }
            }

            println!("\n{} Execution Summary:", "→".cyan().bold());
            println!("   Status: {:?}", result.status);
            println!("   Total time: {}ms", result.execution_time_ms);
            println!(
                "   Expert success rate: {}/{}",
                result.successful_expert_count(),
                conclave.expert_count()
            );
        }

        println!("\n{}", "═".repeat(80));
    }

    Ok(())
}

/// Execute a Maneuver battalion (Flow DSL orchestration)
async fn execute_maneuver(
    config: crate::application::cli::config::battalion_config::ManeuverConfig,
    verbose: bool,
    output: Option<PathBuf>,
) -> Result<(), CliError> {
    use crate::application::services::battalion::flow_visualizer::{
        FlowVisualizer, VisualizationFormat,
    };
    use crate::application::services::battalion::maneuver_service::ManeuverExecutionService;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::platform::container::battalion::BattalionConfig;
    use crate::core::platform::container::battalion::maneuver::Maneuver;
    use crate::core::platform::container::battalion::parser::FlowParser;
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use std::sync::Arc;
    use std::time::Duration;

    if verbose {
        println!("{} Building Maneuver: {}", "→".cyan().bold(), config.name);
        println!("{} Flow: {}", "→".cyan().bold(), config.flow);
        println!("{} Paladins: {}", "→".cyan().bold(), config.paladins.len());
    }

    // Parse flow expression
    let flow = FlowParser::parse(&config.flow).map_err(|e| CliError::ValidationError {
        message: format!("Invalid flow expression: {}", e),
    })?;

    // Show visualization if requested
    if let Some(viz_format) = &config.visualize {
        let visualization = match viz_format.as_str() {
            "ascii" => FlowVisualizer::visualize(&flow, VisualizationFormat::Ascii),
            "mermaid" => FlowVisualizer::visualize(&flow, VisualizationFormat::Mermaid),
            _ => {
                return Err(CliError::InvalidFieldValue {
                    field: "visualize".to_string(),
                    message: format!("must be 'ascii' or 'mermaid', got: {}", viz_format),
                });
            }
        };

        println!("\n{} Flow Visualization:", "📊".cyan().bold());
        println!("{}", "─".repeat(80));
        println!("{}", visualization);
        println!("{}", "─".repeat(80));
    }

    // Build Paladins
    let mut paladins_map = std::collections::HashMap::new();
    for (idx, paladin_ref) in config.paladins.iter().enumerate() {
        let paladin = build_paladin_from_reference(paladin_ref, verbose, idx + 1).await?;
        let name = paladin.node.name.clone();
        paladins_map.insert(name, paladin);
    }

    if verbose {
        println!("{} All Paladins built successfully", "✓".green().bold());
    }

    // Create Battalion configuration (not currently used but kept for consistency)
    let _battalion_config = BattalionConfig::new(&config.name)
        .with_timeout(300)
        .with_description(format!("Maneuver with flow: {}", config.flow));

    // Create Maneuver config
    let maneuver_config =
        crate::core::platform::container::battalion::maneuver::ManeuverConfig::new();

    // Create Maneuver
    let maneuver = Maneuver::new(
        config.name.clone(),
        paladins_map.clone(),
        flow,
        maneuver_config,
    )
    .map_err(|e| CliError::BattalionError {
        message: format!("Failed to create Maneuver: {}", e),
    })?;

    if verbose {
        println!("{} Maneuver built successfully", "✓".green().bold());
    }

    // Create Paladin execution service with circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));

    use paladin_llm::provider_factory::LlmProviderFactory;
    let factory = LlmProviderFactory::new();
    let dummy_llm_port = factory
        .create("openai")
        .map_err(|e| CliError::LlmProviderError {
            message: format!("Failed to create LLM port: {}", e),
        })?;

    let paladin_service = Arc::new(PaladinExecutionService::new(
        dummy_llm_port,
        circuit_breaker,
        None,
        None,
    ));

    let paladin_port = Arc::new(PaladinExecutionAdapter {
        service: paladin_service,
    });

    // Create Maneuver execution service
    let maneuver_service = ManeuverExecutionService::new(paladin_port);

    // Prompt for input
    use crate::application::cli::interactive::prompt_for_input;
    let user_input = prompt_for_input("Enter input for maneuver")?;

    if verbose {
        println!(
            "\n{} Executing Maneuver with flow: {}",
            "→".cyan().bold(),
            config.flow
        );
        println!("{}", "─".repeat(80));
    }

    // Execute the Maneuver
    let result = maneuver_service
        .execute(&maneuver, &user_input)
        .await
        .map_err(|e| CliError::BattalionError {
            message: format!("Maneuver execution failed: {}", e),
        })?;

    // Handle output
    if let Some(output_path) = output {
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
        println!("\n{}", "═".repeat(80));
        println!("{} Maneuver Result - Flow Execution", "📊".cyan());
        println!("{}", "═".repeat(80));
        println!("\n{} Final Output:", "→".cyan().bold());
        println!("{}\n", result.final_output);

        if verbose {
            println!("{} Execution Summary:", "→".cyan().bold());
            println!("   Status: {:?}", result.status);
            println!("   Agents executed: {}", result.execution_order.len());

            println!("\n{} Execution Order:", "→".cyan().bold());
            for (idx, agent_name) in result.execution_order.iter().enumerate() {
                let timing = result
                    .timing_metrics
                    .as_ref()
                    .and_then(|metrics| metrics.get(agent_name))
                    .map(|d| format!("{}ms", d.as_millis()))
                    .unwrap_or_else(|| "N/A".to_string());

                println!("   {}. {} ({})", idx + 1, agent_name, timing);

                if let Some(output) = result.step_outputs.get(agent_name) {
                    let preview = output.chars().take(100).collect::<String>();
                    println!("      Output: {}", preview);
                    if output.len() > 100 {
                        println!("      ... (truncated)");
                    }
                }
            }
        }

        println!("\n{}", "═".repeat(80));
    }

    Ok(())
}

/// Build a Paladin from a PaladinReference (inline or file)
async fn build_paladin_from_reference(
    reference: &crate::application::cli::config::battalion_config::PaladinReference,
    verbose: bool,
    index: usize,
) -> Result<crate::core::platform::container::paladin::Paladin, CliError> {
    use crate::application::cli::config::battalion_config::PaladinReference;
    use crate::application::cli::config::loader::load_paladin_config;
    use crate::application::services::paladin::paladin_builder::PaladinBuilder;
    use paladin_llm::provider_factory::LlmProviderFactory;

    let paladin_config = match reference {
        PaladinReference::File { file } => {
            if verbose {
                println!("  {} Loading Paladin {} from: {}", "→".cyan(), index, file);
            }
            load_paladin_config(&std::path::PathBuf::from(file))?
        }
        PaladinReference::Inline(inline) => {
            if verbose {
                println!(
                    "  {} Building inline Paladin {}: {}",
                    "→".cyan(),
                    index,
                    inline.name
                );
            }
            (**inline).clone()
        }
    };

    // Check API key
    let env_var_name = match paladin_config.provider.provider_type.as_str() {
        "openai" => "OPENAI_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => {
            return Err(CliError::InvalidFieldValue {
                field: "provider.type".to_string(),
                message: format!(
                    "Unknown provider: {}",
                    paladin_config.provider.provider_type
                ),
            });
        }
    };

    if std::env::var(env_var_name).is_err() {
        return Err(CliError::MissingApiKey {
            provider: paladin_config.provider.provider_type.clone(),
            env_var: env_var_name.to_string(),
        });
    }

    // Create LLM port
    let factory = LlmProviderFactory::new();
    let llm_port = factory
        .create(&paladin_config.provider.provider_type)
        .map_err(|e| CliError::LlmProviderError {
            message: e.to_string(),
        })?;

    // Build Paladin
    let mut builder = PaladinBuilder::new(llm_port)
        .system_prompt(&paladin_config.system_prompt)
        .name(&paladin_config.name)
        .model(&paladin_config.model)
        .temperature(paladin_config.temperature)
        .max_loops(paladin_config.max_loops.as_u32())
        .timeout_seconds(paladin_config.timeout_seconds);

    // Add stop words
    for word in &paladin_config.stop_words {
        builder = builder.add_stop_word(word);
    }

    let paladin = builder.build().await?;

    if verbose {
        println!(
            "    {} {} ready ({})",
            "✓".green(),
            paladin_config.name,
            paladin_config.model
        );
    }

    Ok(paladin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_battalion_new_args_creation() {
        let args = BattalionNewArgs {
            name: "test-battalion".to_string(),
            r#type: "formation".to_string(),
            output: PathBuf::from("battalion.yaml"),
        };

        assert_eq!(args.name, "test-battalion");
        assert_eq!(args.r#type, "formation");
        assert_eq!(args.output, PathBuf::from("battalion.yaml"));
    }

    #[test]
    fn test_battalion_run_args_creation() {
        let args = BattalionRunArgs {
            config: PathBuf::from("config.yaml"),
            r#type: "phalanx".to_string(),
            output: Some(PathBuf::from("output.json")),
            verbose: true,
        };

        assert_eq!(args.config, PathBuf::from("config.yaml"));
        assert_eq!(args.r#type, "phalanx");
        assert_eq!(args.output, Some(PathBuf::from("output.json")));
        assert!(args.verbose);
    }

    #[test]
    fn test_handle_battalion_new_formation() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("formation.yaml");

        let args = BattalionNewArgs {
            name: "test-formation".to_string(),
            r#type: "formation".to_string(),
            output: output_path.clone(),
        };

        let result = handle_battalion_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("test-formation"));
        assert!(content.contains("formation"));
    }

    #[test]
    fn test_handle_battalion_new_phalanx() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("phalanx.yaml");

        let args = BattalionNewArgs {
            name: "test-phalanx".to_string(),
            r#type: "phalanx".to_string(),
            output: output_path.clone(),
        };

        let result = handle_battalion_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("phalanx"));
    }

    #[test]
    fn test_handle_battalion_new_campaign() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("campaign.yaml");

        let args = BattalionNewArgs {
            name: "test-campaign".to_string(),
            r#type: "campaign".to_string(),
            output: output_path.clone(),
        };

        let result = handle_battalion_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("campaign"));
    }

    #[test]
    fn test_handle_battalion_new_chain_of_command() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("chain.yaml");

        let args = BattalionNewArgs {
            name: "test-chain".to_string(),
            r#type: "chain-of-command".to_string(),
            output: output_path.clone(),
        };

        let result = handle_battalion_new(args);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("chain-of-command") || content.contains("chain_of_command"));
    }

    #[test]
    fn test_handle_battalion_new_invalid_type() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("invalid.yaml");

        let args = BattalionNewArgs {
            name: "test-battalion".to_string(),
            r#type: "invalid_type".to_string(),
            output: output_path,
        };

        let result = handle_battalion_new(args);
        assert!(result.is_err());

        match result {
            Err(CliError::InvalidFieldValue { field, message }) => {
                assert_eq!(field, "type");
                assert!(message.contains("invalid_type"));
                assert!(message.contains("formation"));
            }
            _ => panic!("Expected InvalidFieldValue error"),
        }
    }

    #[test]
    fn test_handle_battalion_new_file_write_error() {
        let invalid_path = PathBuf::from("/nonexistent/directory/battalion.yaml");

        let args = BattalionNewArgs {
            name: "test-battalion".to_string(),
            r#type: "formation".to_string(),
            output: invalid_path,
        };

        let result = handle_battalion_new(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_battalion_commands_enum_new_variant() {
        let new_args = BattalionNewArgs {
            name: "test".to_string(),
            r#type: "formation".to_string(),
            output: PathBuf::from("test.yaml"),
        };
        let command = BattalionCommands::New(new_args);

        match command {
            BattalionCommands::New(args) => {
                assert_eq!(args.name, "test");
                assert_eq!(args.r#type, "formation");
            }
            _ => panic!("Expected New variant"),
        }
    }

    #[test]
    fn test_battalion_commands_enum_run_variant() {
        let run_args = BattalionRunArgs {
            config: PathBuf::from("config.yaml"),
            r#type: "phalanx".to_string(),
            output: None,
            verbose: false,
        };
        let command = BattalionCommands::Run(run_args);

        match command {
            BattalionCommands::Run(args) => {
                assert_eq!(args.config, PathBuf::from("config.yaml"));
                assert_eq!(args.r#type, "phalanx");
            }
            _ => panic!("Expected Run variant"),
        }
    }

    #[test]
    fn test_all_valid_battalion_types() {
        let temp_dir = TempDir::new().unwrap();
        let valid_types = ["formation", "phalanx", "campaign", "chain-of-command"];

        for battalion_type in &valid_types {
            let output_path = temp_dir.path().join(format!("{}.yaml", battalion_type));
            let args = BattalionNewArgs {
                name: format!("test-{}", battalion_type),
                r#type: battalion_type.to_string(),
                output: output_path.clone(),
            };

            let result = handle_battalion_new(args);
            assert!(
                result.is_ok(),
                "Failed to create battalion of type: {}",
                battalion_type
            );
            assert!(
                output_path.exists(),
                "Output file not created for type: {}",
                battalion_type
            );
        }
    }

    #[test]
    fn test_battalion_run_args_without_output() {
        let args = BattalionRunArgs {
            config: PathBuf::from("config.yaml"),
            r#type: "formation".to_string(),
            output: None,
            verbose: false,
        };

        assert_eq!(args.output, None);
    }

    #[test]
    fn test_battalion_run_args_with_verbose() {
        let args = BattalionRunArgs {
            config: PathBuf::from("config.yaml"),
            r#type: "formation".to_string(),
            output: None,
            verbose: true,
        };

        assert!(args.verbose);
    }
}
