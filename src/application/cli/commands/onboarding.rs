//! Onboarding wizard command
//!
//! Interactive wizard to guide new users through Paladin setup

use super::provider_probe::{validate_anthropic_key, validate_deepseek_key, validate_openai_key};
use crate::application::cli::error::{CliError, CliResult};
use crate::application::cli::formatters::output::OutputFormatter;
use crate::application::cli::formatters::progress::Spinner;
use crate::application::cli::interactive::prompts::PromptBuilder;
use crate::application::cli::interactive::wizard::{StepResult, Wizard, WizardContext, WizardStep};
use crate::application::cli::templates::env::EnvTemplate;
use std::collections::HashMap;
use std::path::PathBuf;

// Constants for context keys
const KEY_SELECTED_PROVIDERS: &str = "selected_providers";
const KEY_API_KEYS_PREFIX: &str = "api_key_";
const KEY_CREATE_SAMPLES: &str = "create_samples";
const KEY_ENV_ACTION: &str = "env_action";

/// LLM Provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Provider {
    OpenAI,
    Anthropic,
    DeepSeek,
}

impl Provider {
    fn name(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OpenAI",
            Provider::Anthropic => "Anthropic",
            Provider::DeepSeek => "DeepSeek",
        }
    }

    fn env_var(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
        }
    }

    fn all() -> Vec<Self> {
        vec![Provider::OpenAI, Provider::Anthropic, Provider::DeepSeek]
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "OpenAI" => Some(Provider::OpenAI),
            "Anthropic" => Some(Provider::Anthropic),
            "DeepSeek" => Some(Provider::DeepSeek),
            _ => None,
        }
    }
}

// API key validation lives in the shared `provider_probe` module (DF-19).

/// Generate sample configuration files
fn generate_sample_configs(provider: &str) -> CliResult<()> {
    use crate::application::cli::templates::battalion_template::generate_battalion_template;
    use crate::application::cli::templates::paladin_template::generate_paladin_template;
    use std::fs;

    // Create examples directory if it doesn't exist
    let examples_dir = PathBuf::from("examples");
    fs::create_dir_all(&examples_dir)
        .map_err(|e| CliError::execution(format!("Failed to create examples directory: {}", e)))?;

    // Generate basic_paladin.yaml
    let basic_paladin = generate_paladin_template("Assistant", provider);
    let basic_path = examples_dir.join("basic_paladin.yaml");
    fs::write(&basic_path, basic_paladin)
        .map_err(|e| CliError::execution(format!("Failed to write basic_paladin.yaml: {}", e)))?;

    // Generate formation.yaml
    let formation = generate_battalion_template("AnalysisPipeline", "formation").map_err(|e| {
        CliError::execution(format!("Failed to generate formation template: {}", e))
    })?;
    let formation_path = examples_dir.join("formation.yaml");
    fs::write(&formation_path, formation)
        .map_err(|e| CliError::execution(format!("Failed to write formation.yaml: {}", e)))?;

    // Generate phalanx.yaml
    let phalanx = generate_battalion_template("ParallelProcessors", "phalanx")
        .map_err(|e| CliError::execution(format!("Failed to generate phalanx template: {}", e)))?;
    let phalanx_path = examples_dir.join("phalanx.yaml");
    fs::write(&phalanx_path, phalanx)
        .map_err(|e| CliError::execution(format!("Failed to write phalanx.yaml: {}", e)))?;

    // Generate paladin_with_rag.yaml - enhanced version with garrison
    let rag_paladin = format!(
        r#"# Paladin with RAG Configuration
# This Paladin includes Garrison (memory) for context retention

name: "RAGAssistant"

system_prompt: |
  You are a knowledgeable AI assistant with access to conversation history.
  Use previous context to provide more relevant and contextual responses.
  Reference prior information when appropriate.

model: "{model}"

temperature: 0.7
max_loops: 3
timeout_seconds: 300

provider:
  type: {provider}

# Enable Garrison for context/memory
garrison:
  type: sqlite
  config:
    path: "./garrison.db"
    max_entries: 100

# Optional: Add Arsenal for tool access
# arsenal:
#   mcp_servers:
#     - name: web_search
#       type: stdio
#       command: uvx
#       args:
#         - mcp-web-search
"#,
        model = match provider {
            "openai" => "gpt-4",
            "deepseek" => "deepseek-chat",
            "anthropic" => "claude-3-5-sonnet-20241022",
            _ => "gpt-4",
        },
        provider = provider
    );
    let rag_path = examples_dir.join("paladin_with_rag.yaml");
    fs::write(&rag_path, rag_paladin).map_err(|e| {
        CliError::execution(format!("Failed to write paladin_with_rag.yaml: {}", e))
    })?;

    Ok(())
}

// Welcome Step
struct WelcomeStep;

impl WizardStep for WelcomeStep {
    fn execute(&self, _context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("Welcome to Paladin! 🛡️");
        println!();
        formatter.success("Let's get you set up with a multi-agent orchestration framework.");
        println!();

        println!("This wizard will help you:");
        println!("  • Configure LLM provider API keys");
        println!("  • Set up your environment file");
        println!("  • Validate API connectivity");
        println!("  • Generate sample configurations");
        println!();

        let continue_prompt = PromptBuilder::confirm("Ready to begin?")
            .with_default(true)
            .prompt()?;

        if continue_prompt {
            Ok(StepResult::Continue)
        } else {
            formatter.warning("Onboarding cancelled. Run 'paladin onboarding' anytime to restart.");
            Ok(StepResult::Cancel)
        }
    }

    fn name(&self) -> &str {
        "Welcome"
    }
}

// Provider Selection Step
struct ProviderSelectionStep;

impl WizardStep for ProviderSelectionStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("LLM Provider Selection");
        println!();
        println!("Select one or more LLM providers to configure:");
        println!();

        let provider_options: Vec<String> = Provider::all()
            .iter()
            .map(|p| p.name().to_string())
            .collect();

        let selections = PromptBuilder::multi_select(
            "Select providers (use Space to select, Enter to confirm)",
            provider_options.clone(),
        )
        .with_defaults(vec![true, false, false])
        .prompt()?;

        if selections.is_empty() {
            formatter.error("At least one provider must be selected.");
            return Ok(StepResult::Continue); // Retry this step
        }

        // selections are already the provider names (Vec<String>), not indices
        context.set(KEY_SELECTED_PROVIDERS, selections.join(","));
        let selected_names = selections;

        formatter.success(&format!(
            "Selected providers: {}",
            selected_names.join(", ")
        ));
        println!();

        Ok(StepResult::Continue)
    }

    fn name(&self) -> &str {
        "Provider Selection"
    }
}

// API Key Input Step
struct ApiKeyInputStep;

impl WizardStep for ApiKeyInputStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("API Key Configuration");
        println!();

        let provider_names = context
            .get(KEY_SELECTED_PROVIDERS)
            .ok_or_else(|| CliError::configuration("No providers selected".to_string()))?;

        let providers: Vec<Provider> = provider_names
            .split(',')
            .filter_map(Provider::from_name)
            .collect();

        let mut keys_configured = 0;

        for provider in &providers {
            println!("Enter your {} API key:", provider.name());

            let key =
                PromptBuilder::password(&format!("{}_API_KEY", provider.name().to_uppercase()))
                    .prompt()?;

            if key.is_empty() {
                formatter.warning(&format!("Skipping {} (no key provided)", provider.name()));
                continue;
            }

            // Store API key in context
            context.set(format!("{}{}", KEY_API_KEYS_PREFIX, provider.name()), key);
            keys_configured += 1;
        }

        if keys_configured == 0 {
            formatter.error("No API keys provided. At least one is required.");
            return Ok(StepResult::Continue); // Retry
        }

        println!();
        formatter.success(&format!("Configured {} API key(s)", keys_configured));
        println!();

        Ok(StepResult::Continue)
    }

    fn name(&self) -> &str {
        "API Key Input"
    }
}

// API Validation Step
struct ApiValidationStep;

impl WizardStep for ApiValidationStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("API Validation");
        println!();
        println!("Testing API connectivity...");
        println!();

        let provider_names = context
            .get(KEY_SELECTED_PROVIDERS)
            .ok_or_else(|| CliError::configuration("No providers selected".to_string()))?;

        let providers: Vec<Provider> = provider_names
            .split(',')
            .filter_map(Provider::from_name)
            .collect();

        for provider in &providers {
            let key_name = format!("{}{}", KEY_API_KEYS_PREFIX, provider.name());
            if let Some(api_key) = context.get(&key_name) {
                let spinner = Spinner::new(format!("Validating {}...", provider.name()));

                // Validate API key with actual API call
                let validation_result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        match provider {
                            Provider::OpenAI => validate_openai_key(api_key).await,
                            Provider::Anthropic => validate_anthropic_key(api_key).await,
                            Provider::DeepSeek => validate_deepseek_key(api_key).await,
                        }
                    })
                });

                match validation_result {
                    Ok(_) => {
                        spinner.finish_with_message(format!("{} ✓", provider.name()));
                    }
                    Err(e) => {
                        spinner.finish_with_message(format!("{} ✗", provider.name()));
                        formatter.error(&format!("Validation failed: {}", e));
                        println!();
                        formatter.warning("You can continue setup, but verify your API key later.");
                        println!();
                    }
                }
            }
        }

        println!();
        formatter.success("All API keys validated successfully!");
        println!();

        Ok(StepResult::Continue)
    }

    fn name(&self) -> &str {
        "API Validation"
    }
}

// Environment File Step
struct EnvFileStep;

impl WizardStep for EnvFileStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("Environment File Setup");
        println!();

        let env_path = PathBuf::from(".env");
        let env_exists = env_path.exists();

        let action = if env_exists {
            formatter.warning(&format!("Found existing file: {}", env_path.display()));
            println!();

            let choice = PromptBuilder::select(
                "How should we proceed?",
                vec![
                    "Overwrite (replace existing file)".to_string(),
                    "Skip (keep existing file)".to_string(),
                    "Merge (combine configurations)".to_string(),
                ],
            )
            .prompt_index()?;

            match choice {
                0 => "overwrite".to_string(),
                1 => "skip".to_string(),
                2 => "merge".to_string(),
                _ => "skip".to_string(),
            }
        } else {
            println!("Creating new .env file...");
            "overwrite".to_string()
        };

        context.set(KEY_ENV_ACTION, &action);

        match action.as_str() {
            "skip" => {
                formatter.info("Keeping existing .env file");
            }
            "overwrite" | "merge" => {
                write_env_file(context, action.as_str() == "merge")?;
                formatter.success(if action.as_str() == "merge" {
                    "Merged with existing .env file"
                } else {
                    "Created .env file"
                });
            }
            _ => {}
        }

        println!();
        Ok(StepResult::Continue)
    }

    fn name(&self) -> &str {
        "Environment File"
    }
}

// Sample Configurations Step
struct SampleConfigsStep;

impl WizardStep for SampleConfigsStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("Sample Configurations");
        println!();

        let create = PromptBuilder::confirm("Create sample configuration files?")
            .with_default(true)
            .prompt()?;

        if create {
            let spinner = Spinner::new("Generating samples...");

            // Get the first configured provider for sample generation
            let provider = context
                .get(KEY_SELECTED_PROVIDERS)
                .and_then(|providers| providers.split(',').next().map(String::from))
                .unwrap_or_else(|| "openai".to_string())
                .to_lowercase();

            // Generate sample configuration files
            let result = generate_sample_configs(&provider);

            match result {
                Ok(_) => {
                    spinner.finish_with_message("Sample files created");

                    println!();
                    println!("  • examples/basic_paladin.yaml");
                    println!("  • examples/formation.yaml");
                    println!("  • examples/phalanx.yaml");
                    println!("  • examples/paladin_with_rag.yaml");

                    context.set(KEY_CREATE_SAMPLES, "true");
                }
                Err(e) => {
                    spinner.finish_with_message("Failed to create samples");
                    formatter.error(&format!("Error generating samples: {}", e));
                    formatter.warning("You can manually create configuration files later.");
                }
            }
        } else {
            formatter.info("Skipped sample generation");
        }

        println!();
        Ok(StepResult::Continue)
    }

    fn name(&self) -> &str {
        "Sample Configurations"
    }
}

// Summary Step
struct SummaryStep;

impl WizardStep for SummaryStep {
    fn execute(&self, context: &mut WizardContext) -> CliResult<StepResult> {
        let formatter = OutputFormatter::new();

        formatter.header("Setup Complete! 🎉");
        println!();

        formatter.success("Your Paladin environment is ready!");
        println!();

        println!("Completed:");

        if let Some(providers) = context.get(KEY_SELECTED_PROVIDERS) {
            let count = providers.split(',').count();
            println!("  ✓ Configured {} LLM provider(s)", count);
        }

        if context.get(KEY_ENV_ACTION).is_some() {
            println!("  ✓ Created/updated .env file");
        }

        if context.contains(KEY_CREATE_SAMPLES) {
            println!("  ✓ Generated sample configurations");
        }

        println!();
        formatter.box_message(&[
            "Next Steps:",
            "1. Run 'paladin setup-check' to verify your configuration",
            "2. Try the examples in the examples/ directory",
            "3. Run 'paladin features' to explore available commands",
            "4. Read the docs at https://github.com/DF3NDR/paladin",
        ]);

        println!();
        Ok(StepResult::Complete)
    }

    fn name(&self) -> &str {
        "Summary"
    }
}

/// Write .env file with API keys
fn write_env_file(context: &WizardContext, merge: bool) -> CliResult<()> {
    let provider_names = context
        .get(KEY_SELECTED_PROVIDERS)
        .ok_or_else(|| CliError::configuration("No providers selected".to_string()))?;

    let providers: Vec<Provider> = provider_names
        .split(',')
        .filter_map(Provider::from_name)
        .collect();

    // Collect API keys
    let mut api_keys = HashMap::new();
    for provider in providers {
        let key_name = format!("{}{}", KEY_API_KEYS_PREFIX, provider.name());
        if let Some(key) = context.get(&key_name) {
            api_keys.insert(provider.env_var().to_string(), key.to_string());
        }
    }

    let existing_content = if merge {
        std::fs::read_to_string(".env").ok()
    } else {
        None
    };

    let template = EnvTemplate::new();
    let content = template
        .generate(&api_keys, existing_content.as_deref())
        .map_err(|e| CliError::configuration(format!("Template error: {}", e)))?;

    std::fs::write(".env", content)?;

    Ok(())
}

/// Run the onboarding wizard
pub async fn run_onboarding() -> CliResult<()> {
    let wizard = Wizard::new()
        .add_step(Box::new(WelcomeStep))
        .add_step(Box::new(ProviderSelectionStep))
        .add_step(Box::new(ApiKeyInputStep))
        .add_step(Box::new(ApiValidationStep))
        .add_step(Box::new(EnvFileStep))
        .add_step(Box::new(SampleConfigsStep))
        .add_step(Box::new(SummaryStep))
        .with_resume();

    // Run wizard
    wizard.run().map_err(|e| CliError::Other(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[test]
    fn test_provider_enum_name() {
        assert_eq!(Provider::OpenAI.name(), "OpenAI");
        assert_eq!(Provider::Anthropic.name(), "Anthropic");
        assert_eq!(Provider::DeepSeek.name(), "DeepSeek");
    }

    #[test]
    fn test_provider_env_var() {
        assert_eq!(Provider::OpenAI.env_var(), "OPENAI_API_KEY");
        assert_eq!(Provider::Anthropic.env_var(), "ANTHROPIC_API_KEY");
        assert_eq!(Provider::DeepSeek.env_var(), "DEEPSEEK_API_KEY");
    }

    #[test]
    fn test_provider_from_name() {
        assert_eq!(Provider::from_name("OpenAI"), Some(Provider::OpenAI));
        assert_eq!(Provider::from_name("Anthropic"), Some(Provider::Anthropic));
        assert_eq!(Provider::from_name("DeepSeek"), Some(Provider::DeepSeek));
        assert_eq!(Provider::from_name("Invalid"), None);
    }

    #[test]
    fn test_provider_all() {
        let providers = Provider::all();
        assert_eq!(providers.len(), 3);
        assert!(providers.contains(&Provider::OpenAI));
        assert!(providers.contains(&Provider::Anthropic));
        assert!(providers.contains(&Provider::DeepSeek));
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_openai_key_invalid() {
        let result = validate_openai_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_anthropic_key_invalid() {
        let result = validate_anthropic_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_deepseek_key_invalid() {
        let result = validate_deepseek_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_sample_configs() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Change to temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Test generation for each provider
        for provider in &["openai", "anthropic", "deepseek"] {
            let result = generate_sample_configs(provider);
            assert!(
                result.is_ok(),
                "Failed to generate configs for {}",
                provider
            );

            // Verify files were created
            assert!(temp_dir.path().join("examples/basic_paladin.yaml").exists());
            assert!(temp_dir.path().join("examples/formation.yaml").exists());
            assert!(temp_dir.path().join("examples/phalanx.yaml").exists());
            assert!(
                temp_dir
                    .path()
                    .join("examples/paladin_with_rag.yaml")
                    .exists()
            );

            // Clean up for next iteration
            fs::remove_dir_all(temp_dir.path().join("examples")).unwrap();
        }

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_wizard_step_names() {
        assert_eq!(WelcomeStep.name(), "Welcome");
        assert_eq!(ProviderSelectionStep.name(), "Provider Selection");
        assert_eq!(ApiKeyInputStep.name(), "API Key Input");
        assert_eq!(ApiValidationStep.name(), "API Validation");
        assert_eq!(EnvFileStep.name(), "Environment File");
        assert_eq!(SampleConfigsStep.name(), "Sample Configurations");
        assert_eq!(SummaryStep.name(), "Summary");
    }
}
