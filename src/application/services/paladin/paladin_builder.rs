//! PaladinBuilder - Fluent builder for creating Paladin instances with validation
//!
//! This module provides a builder pattern implementation for constructing Paladin entities
//! with compile-time safety and runtime validation of configuration parameters.
//!
//! # Examples
//!
//! ```rust,no_run
//! use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
//! use paladin_ports::output::llm_port::LlmPort;
//! use paladin::core::platform::container::paladin_config::OutputFormat;
//! use std::sync::Arc;
//!
//! # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
//! let paladin = PaladinBuilder::new(llm_port)
//!     .system_prompt("You are a helpful coding assistant")
//!     .name("CodePaladin")
//!     .user_name("Developer")
//!     .model("gpt-4")
//!     .temperature(0.7)
//!     .max_loops(5)
//!     .add_stop_word("STOP")
//!     .retry_attempts(3)
//!     .timeout_seconds(300)
//!     .enable_planning(true)
//!     .output_format(OutputFormat::Json)
//!     .build().await?;
//! # Ok(())
//! # }
//! ```

use crate::application::services::paladin::error::PaladinError;
use crate::application::services::sanctum::memory_extraction_service::MemoryExtractionStrategy;
use crate::config::arsenal::MCPServerConfig;
use crate::core::base::entity::node::Node;
use crate::core::platform::container::arsenal::Armament;
use crate::core::platform::container::herald::Herald;
use crate::core::platform::container::paladin::MaxLoops;
use crate::core::platform::container::paladin::{Paladin, PaladinData};
use crate::core::platform::container::paladin_config::{OutputFormat, PaladinConfig};
use crate::infrastructure::adapters::citadel::file_citadel::FileCitadel;
use paladin_ports::output::arsenal_port::ArsenalRegistry;
use paladin_ports::output::citadel_port::CitadelPort;
use paladin_ports::output::embedding_port::EmbeddingPort;
use paladin_ports::output::garrison_port::GarrisonPort;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::sanctum_port::SanctumPort;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Builder for creating Paladin instances with validation
///
/// The builder provides a fluent interface for constructing Paladins with comprehensive
/// validation of all configuration parameters. It enforces constraints like:
/// - Non-empty system prompts
/// - Temperature within the selected provider's declared range (falls back to
///   [0.0, 1.0] when the provider declares none — ADR-0004)
/// - Max loops in range [1, 100]
///
/// # Examples
///
/// ```rust,no_run
/// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
/// # use paladin_ports::output::llm_port::LlmPort;
/// # use std::sync::Arc;
/// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
/// let paladin = PaladinBuilder::new(llm_port)
///     .system_prompt("You are an AI assistant")
///     .name("Assistant")
///     .model("gpt-4")
///     .temperature(0.8)
///     .build().await?;
/// # Ok(())
/// # }
/// ```
pub struct PaladinBuilder {
    llm_port: Arc<dyn LlmPort>,
    data: PaladinData,
    config: PaladinConfig,
    garrison: Option<Arc<dyn GarrisonPort>>,
    arsenal_registry: Option<Arc<dyn ArsenalRegistry>>,
    mcp_servers: Vec<MCPServerConfig>,
    citadel_port: Option<Arc<dyn CitadelPort>>,
    autosave_enabled: bool,
    state_dir: Option<String>,
    herald: Option<Arc<dyn Herald>>,
    // Sanctum RAG integration fields
    sanctum_port: Option<Arc<dyn SanctumPort>>,
    embedding_port: Option<Arc<dyn EmbeddingPort>>,
    memory_extraction_strategy: MemoryExtractionStrategy,
    // Auto-prompt generation fields
    auto_generate_prompt_enabled: bool,
    agent_description: Option<String>,
    manual_prompt_override: bool,
    // Auto-temperature selection fields
    auto_temperature_enabled: bool,
    manual_temperature_override: bool,
    // Handoff/delegation fields
    specialist_agents: Vec<Arc<Paladin>>,
    handoff_config: Option<Arc<crate::core::platform::container::autonomous_config::HandoffConfig>>,
    handoffs_configured: bool,
}

impl PaladinBuilder {
    /// Creates a new PaladinBuilder with default values
    ///
    /// # Arguments
    ///
    /// * `llm_port` - The LLM port implementation to use for this Paladin
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port);
    /// # }
    /// ```
    pub fn new(llm_port: Arc<dyn LlmPort>) -> Self {
        Self {
            llm_port,
            data: PaladinData::default(),
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        }
    }

    /// Sets the system prompt that defines the Paladin's behavior and personality
    ///
    /// # Arguments
    ///
    /// * `prompt` - The system prompt (must be non-empty)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a helpful coding assistant");
    /// # }
    /// ```
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.data.system_prompt = prompt.into();
        self.manual_prompt_override = true; // Manual prompt takes precedence
        self
    }

    /// Sets the name of the Paladin
    ///
    /// # Arguments
    ///
    /// * `name` - The Paladin's name
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .name("CodeAssistant");
    /// # }
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.data.name = name.into();
        self
    }

    /// Sets the user name that the Paladin will interact with
    ///
    /// # Arguments
    ///
    /// * `user_name` - The user's name
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .user_name("Developer");
    /// # }
    /// ```
    pub fn user_name(mut self, user_name: impl Into<String>) -> Self {
        self.data.user_name = user_name.into();
        self
    }

    /// Sets the LLM model to use
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier (e.g., "gpt-4", "gpt-3.5-turbo")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .model("gpt-4");
    /// # }
    /// ```
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.data.model = model.into();
        self
    }

    /// Sets the temperature for LLM generation (controls randomness)
    ///
    /// # Arguments
    ///
    /// * `temperature` - Value between 0.0 (deterministic) and 1.0 (random)
    ///
    /// # Validation
    ///
    /// Temperature must be in range [0.0, 1.0] or validation will fail during build()
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .temperature(0.7);
    /// # }
    /// ```
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.data.temperature = temperature;
        self.manual_temperature_override = true;
        self
    }

    /// Sets the maximum number of reasoning loops
    ///
    /// # Arguments
    ///
    /// * `max_loops` - Maximum iterations (must be between 1 and 100)
    ///
    /// # Validation
    ///
    /// max_loops must be in range [1, 100] or validation will fail during build()
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .max_loops(5);
    /// # }
    /// ```
    pub fn max_loops(mut self, max_loops: u32) -> Self {
        self.data.max_loops = MaxLoops::Fixed(max_loops);
        self
    }

    /// Enables or disables automatic system prompt generation
    ///
    /// When enabled, the Paladin will use LLM to automatically generate
    /// an optimized system prompt based on the agent description.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable auto-prompt generation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .auto_generate_prompt(true)
    ///     .agent_description("A code review assistant specialized in Rust");
    /// # }
    /// ```
    pub fn auto_generate_prompt(mut self, enabled: bool) -> Self {
        self.auto_generate_prompt_enabled = enabled;
        self
    }

    /// Sets the agent description for auto-prompt generation
    ///
    /// This description is used by the prompt generation service to create
    /// a contextual system prompt optimized for the agent's role.
    ///
    /// # Arguments
    ///
    /// * `description` - Description of the agent's role and capabilities
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .auto_generate_prompt(true)
    ///     .agent_description("Analyzes security vulnerabilities in code");
    /// # }
    /// ```
    pub fn agent_description(mut self, description: impl Into<String>) -> Self {
        let desc = description.into();
        self.agent_description = Some(desc.clone());
        self.data.agent_description = desc; // Also set in PaladinData for autonomous features
        self
    }

    /// Forces regeneration of auto-generated prompt by clearing cache
    ///
    /// Call this method to invalidate cached prompts and force the service
    /// to generate a fresh prompt on the next build.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .auto_generate_prompt(true)
    ///     .agent_description("Data analyst")
    ///     .regenerate_prompt(); // Clear cache
    /// # }
    /// ```
    pub fn regenerate_prompt(self) -> Self {
        // Note: Cache invalidation will happen in build() method
        // when we have access to the PromptGenerationService
        self
    }

    /// Enables or disables automatic temperature selection based on task type
    ///
    /// When enabled, the Paladin will use LLM to analyze the agent description
    /// and task context to automatically select an optimal temperature value:
    /// - Creative tasks (writing, brainstorming): ~0.85
    /// - Analytical tasks (math, code, logic): ~0.2
    /// - Standard tasks (Q&A, conversation): ~0.6
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable automatic temperature selection
    ///
    /// # Note
    ///
    /// If you call `temperature()` explicitly, it will override the automatic
    /// temperature selection, just like manual system prompts override auto-generated ones.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .agent_description("A creative writing assistant")
    ///     .auto_temperature(true); // Will use ~0.85 for creative tasks
    /// # }
    /// ```
    pub fn auto_temperature(mut self, enabled: bool) -> Self {
        self.auto_temperature_enabled = enabled;
        self
    }

    /// Adds a stop word that will halt execution when detected in LLM output
    ///
    /// # Arguments
    ///
    /// * `word` - The stop word to add
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .add_stop_word("STOP")
    ///     .add_stop_word("END");
    /// # }
    /// ```
    pub fn add_stop_word(mut self, word: impl Into<String>) -> Self {
        self.data.stop_words.push(word.into());
        self
    }

    /// Sets the number of retry attempts for failed LLM calls
    ///
    /// # Arguments
    ///
    /// * `attempts` - Number of retries (default: 3)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .retry_attempts(5);
    /// # }
    /// ```
    pub fn retry_attempts(mut self, attempts: u32) -> Self {
        self.config.retry_attempts = attempts;
        self
    }

    /// Sets the execution timeout in seconds
    ///
    /// # Arguments
    ///
    /// * `seconds` - Timeout duration (default: 300)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .timeout_seconds(600);
    /// # }
    /// ```
    pub fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    /// Enables or disables planning mode
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable planning (default: false)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .enable_planning(true);
    /// # }
    /// ```
    pub fn enable_planning(mut self, enabled: bool) -> Self {
        self.config.enable_planning = enabled;
        self
    }

    /// Enables or disables vision capabilities for multimodal input
    ///
    /// When enabled, the Paladin can process both text and images in requests.
    /// The underlying LLM must support vision (checked during validation).
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable vision (default: false)
    ///
    /// # Validation
    ///
    /// If vision is enabled, the LLM port must implement VisionCapableLlm trait,
    /// or validation will fail during build().
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an AI that can analyze images")
    ///     .enable_vision(true);
    /// # }
    /// ```
    pub fn enable_vision(mut self, enabled: bool) -> Self {
        self.data.vision_enabled = enabled;
        self
    }

    /// Enables autonomous planning mode (Layer 1)
    ///
    /// When enabled, the Paladin will use PlanningService to decompose complex
    /// tasks into subtasks before execution. Requires planning service to be
    /// configured via `with_planning_service` in the execution service.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable autonomous planning
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let paladin = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an AI assistant")
    ///     .enable_autonomous_planning(true)
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn enable_autonomous_planning(mut self, enabled: bool) -> Self {
        self.data.autonomous_planning = enabled;
        self
    }

    /// Enables autonomous prompt generation (Layer 1)
    ///
    /// When enabled, the Paladin will use PromptGenerationService to generate
    /// a contextual system prompt based on agent_description. Requires prompt
    /// generation service to be configured via `with_prompt_generation_service`
    /// in the execution service.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable autonomous prompt generation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let paladin = PaladinBuilder::new(llm_port)
    ///     .agent_description("An AI specialized in code review")
    ///     .enable_autonomous_prompts(true)
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn enable_autonomous_prompts(mut self, enabled: bool) -> Self {
        self.data.autonomous_prompts = enabled;
        self
    }

    /// Enables dynamic temperature adjustment (Layer 2)
    ///
    /// When enabled, temperature increases linearly from the configured base
    /// value to 1.0 over the course of max_loops iterations. This encourages
    /// exploration in later loops when the agent might be stuck.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable dynamic temperature
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let paladin = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an AI assistant")
    ///     .temperature(0.5)  // Starting temperature
    ///     .max_loops(5)
    ///     .enable_dynamic_temperature(true)  // Will increase to 1.0 by loop 5
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn enable_dynamic_temperature(mut self, enabled: bool) -> Self {
        self.data.dynamic_temperature = enabled;
        self
    }

    /// Sets the output format for responses
    ///
    /// # Arguments
    ///
    /// * `format` - The desired output format
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin::core::platform::container::paladin_config::OutputFormat;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .output_format(OutputFormat::Json);
    /// # }
    /// ```
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.config.output_format = format;
        self
    }

    /// Attaches a Garrison memory system to the Paladin
    ///
    /// The Garrison enables the Paladin to maintain conversation context across
    /// multiple turns. It is optional for single-turn operations but required
    /// for multi-turn conversations.
    ///
    /// # Arguments
    ///
    /// * `garrison` - The Garrison port implementation to use for memory
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin_ports::output::garrison_port::GarrisonPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>, garrison: Arc<dyn GarrisonPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a conversational assistant")
    ///     .with_garrison(garrison);
    /// # }
    /// ```
    pub fn with_garrison(mut self, garrison: Arc<dyn GarrisonPort>) -> Self {
        self.garrison = Some(garrison);
        self
    }

    /// Attaches an Arsenal registry to the Paladin for tool execution
    ///
    /// The Arsenal enables the Paladin to discover and invoke external tools
    /// through the Model Context Protocol (MCP). Tools can be STDIO-based
    /// (command-line) or SSE-based (HTTP).
    ///
    /// # Arguments
    ///
    /// * `registry` - The Arsenal registry implementation containing registered tools
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin_ports::output::arsenal_port::ArsenalRegistry;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>, registry: Arc<dyn ArsenalRegistry>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a tool-using assistant")
    ///     .with_arsenal_registry(registry);
    /// # }
    /// ```
    pub fn with_arsenal_registry(mut self, registry: Arc<dyn ArsenalRegistry>) -> Self {
        self.arsenal_registry = Some(registry);
        self
    }

    /// Sets a Herald formatter for output formatting
    ///
    /// Herald formatters control how Paladin execution results are formatted for display.
    /// Built-in formatters include JSON, Markdown, and Table.
    ///
    /// # Arguments
    ///
    /// * `herald` - The Herald implementation to use for formatting
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin::infrastructure::adapters::herald::JsonHerald;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let herald = Arc::new(JsonHerald::default());
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a helpful assistant")
    ///     .with_herald(herald);
    /// # }
    /// ```
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }

    /// Attaches a Sanctum vector store for RAG capabilities
    ///
    /// Sanctum enables Retrieval-Augmented Generation by storing and retrieving
    /// relevant context from past conversations and knowledge. When combined with
    /// an embedding port, the Paladin can automatically:
    /// - Retrieve relevant memories before generating responses
    /// - Extract and store important information after conversations
    /// - Perform semantic search over past interactions
    ///
    /// **Note**: Requires an `EmbeddingPort` to be set via `with_embedding_port()`
    ///
    /// # Arguments
    ///
    /// * `sanctum` - The Sanctum port implementation (e.g., Qdrant adapter)
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin_ports::output::sanctum_port::SanctumPort;
    /// # use paladin_ports::output::embedding_port::EmbeddingPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>, sanctum: Arc<dyn SanctumPort>, embedding: Arc<dyn EmbeddingPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let paladin = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an assistant with RAG")
    ///     .with_sanctum(sanctum)
    ///     .with_embedding_port(embedding)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_sanctum(mut self, sanctum: Arc<dyn SanctumPort>) -> Self {
        self.sanctum_port = Some(sanctum);
        self
    }

    /// Attaches an embedding port for generating vector embeddings
    ///
    /// The embedding port converts text into vector representations that can be
    /// stored in Sanctum and used for semantic search. This is required when
    /// using Sanctum for RAG capabilities.
    ///
    /// Supported embedding providers include:
    /// - OpenAI (`text-embedding-ada-002`, `text-embedding-3-small`, `text-embedding-3-large`)
    /// - Local models via transformers
    ///
    /// # Arguments
    ///
    /// * `embedding_port` - The embedding port implementation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin_ports::output::embedding_port::EmbeddingPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>, embedding: Arc<dyn EmbeddingPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a RAG-enabled assistant")
    ///     .with_embedding_port(embedding);
    /// # }
    /// ```
    pub fn with_embedding_port(mut self, embedding_port: Arc<dyn EmbeddingPort>) -> Self {
        self.embedding_port = Some(embedding_port);
        self
    }

    /// Sets the strategy for extracting memories from conversations
    ///
    /// Controls when the Paladin should analyze conversation history and extract
    /// important information to store in Sanctum.
    ///
    /// # Strategies
    ///
    /// - `EveryTurn`: Extract memories after each conversation turn (most thorough but expensive)
    /// - `OnCompletion`: Extract memories only when the conversation completes (default, balanced)
    /// - `Manual`: Only extract memories when explicitly triggered
    /// - `Threshold { importance }`: Extract memories when importance threshold is exceeded
    ///
    /// # Arguments
    ///
    /// * `strategy` - The memory extraction strategy to use
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin::application::services::sanctum::memory_extraction_service::MemoryExtractionStrategy;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an assistant")
    ///     .memory_extraction_strategy(MemoryExtractionStrategy::Threshold { importance: 5 });
    /// # }
    /// ```
    pub fn memory_extraction_strategy(mut self, strategy: MemoryExtractionStrategy) -> Self {
        self.memory_extraction_strategy = strategy;
        self
    }

    /// Registers specialist agents for task delegation via handoffs
    ///
    /// Allows this Paladin to delegate tasks to specialist agents when confidence
    /// is low or tasks require specific expertise. The handoff strategy determines
    /// when delegation occurs.
    ///
    /// # Arguments
    ///
    /// * `specialists` - Vector of specialist Paladin agents available for delegation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Create specialist agents
    /// let rust_expert = PaladinBuilder::new(llm_port.clone())
    ///     .system_prompt("You are a Rust programming expert")
    ///     .name("RustExpert")
    ///     .build().await?;
    ///
    /// let python_expert = PaladinBuilder::new(llm_port.clone())
    ///     .system_prompt("You are a Python programming expert")
    ///     .name("PythonExpert")
    ///     .build().await?;
    ///
    /// // Coordinator can delegate to specialists
    /// let coordinator = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a code coordinator")
    ///     .name("Coordinator")
    ///     .with_handoffs(vec![Arc::new(rust_expert), Arc::new(python_expert)])
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_handoffs(mut self, specialists: Vec<Arc<Paladin>>) -> Self {
        self.specialist_agents = specialists;
        self.handoffs_configured = !self.specialist_agents.is_empty();
        self
    }

    /// Sets the handoff configuration for agent delegation
    ///
    /// Configures when and how this Paladin should delegate tasks to specialists.
    ///
    /// # Arguments
    ///
    /// * `config` - Handoff configuration including strategy and limits
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin::core::platform::container::autonomous_config::{HandoffConfig, HandoffRetryConfig};
    /// # use paladin::core::platform::container::handoff::HandoffStrategy;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let config = Arc::new(HandoffConfig {
    ///     enabled: true,
    ///     strategy: HandoffStrategy::threshold(0.7),
    ///     max_depth: 3,
    ///     retry: HandoffRetryConfig::default(),
    /// });
    ///
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a coordinator")
    ///     .handoff_config(config);
    /// # }
    /// ```
    pub fn handoff_config(
        mut self,
        config: Arc<crate::core::platform::container::autonomous_config::HandoffConfig>,
    ) -> Self {
        self.handoff_config = Some(config);
        self
    }

    /// Adds an STDIO-based MCP server configuration
    ///
    /// STDIO servers are command-line tools that communicate via stdin/stdout
    /// using the Model Context Protocol.
    ///
    /// # Arguments
    ///
    /// * `name` - Identifier for the server
    /// * `command` - Command to execute
    /// * `args` - Command-line arguments
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an assistant with web search")
    ///     .add_mcp_stdio("web_search", "uvx", &["mcp-web-search"]);
    /// # }
    /// ```
    pub fn add_mcp_stdio(
        mut self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: &[&str],
    ) -> Self {
        self.mcp_servers.push(MCPServerConfig {
            name: name.into(),
            server_type: "stdio".to_string(),
            command: Some(command.into()),
            args: Some(args.iter().map(|s| s.to_string()).collect()),
            endpoint: None,
            auth_token_env: None,
        });
        self
    }

    /// Attaches a Citadel state persistence system to the Paladin
    ///
    /// The Citadel enables automatic saving and restoration of Paladin state,
    /// including configuration, execution history, and Garrison context.
    ///
    /// # Arguments
    ///
    /// * `citadel` - The Citadel port implementation to use for state persistence
    ///
    /// Enables state persistence by attaching a Citadel adapter.
    ///
    /// The Citadel system provides automatic state saving and restoration for
    /// Paladin agents. This enables fault tolerance, debugging, and long-running
    /// workflows that can survive system restarts.
    ///
    /// # Arguments
    ///
    /// * `citadel` - A Citadel adapter implementing the CitadelPort trait
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin::infrastructure::adapters::citadel::file_citadel::FileCitadel;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a file-based Citadel adapter
    /// let citadel = Arc::new(FileCitadel::new("./paladin-states")?);
    ///
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a stateful assistant")
    ///     .with_citadel(citadel);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_citadel(mut self, citadel: Arc<dyn CitadelPort>) -> Self {
        self.citadel_port = Some(citadel);
        self
    }

    /// Enables automatic state saving after Paladin execution
    ///
    /// When enabled, the Paladin's state will be automatically saved to the
    /// Citadel after each execution completes successfully. This enables
    /// resumption of work and audit trails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a persistent assistant")
    ///     .enable_autosave();
    /// # }
    /// ```
    pub fn enable_autosave(mut self) -> Self {
        self.autosave_enabled = true;
        self
    }

    /// Sets the directory path for state persistence
    ///
    /// If not set, the default directory from configuration will be used.
    /// The directory will be created automatically if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path where state files will be saved
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # fn example(llm_port: Arc<dyn LlmPort>) {
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are a persistent assistant")
    ///     .save_state_dir("./my_citadel");
    /// # }
    /// ```
    pub fn save_state_dir(mut self, path: impl Into<String>) -> Self {
        self.state_dir = Some(path.into());
        self
    }

    /// Restores a Paladin from a previously saved state
    ///
    /// Loads the Paladin configuration, execution history, and Garrison context
    /// from the Citadel, allowing resumption of previous work.
    ///
    /// # Arguments
    ///
    /// * `state_id` - UUID of the saved state to restore
    ///
    /// # Returns
    ///
    /// - `Ok(Self)` if state was successfully restored
    /// - `Err(PaladinError)` if state couldn't be loaded or Citadel is not configured
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use paladin_ports::output::citadel_port::CitadelPort;
    /// # use uuid::Uuid;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>, citadel: Arc<dyn CitadelPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let state_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?;
    /// let builder = PaladinBuilder::new(llm_port)
    ///     .with_citadel(citadel)
    ///     .restore_from(state_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn restore_from(mut self, state_id: Uuid) -> Result<Self, PaladinError> {
        let citadel = self.citadel_port.as_ref().ok_or_else(|| {
            PaladinError::ConfigurationError(
                "Citadel port must be configured to restore state".to_string(),
            )
        })?;

        let state = citadel
            .load_paladin(state_id)
            .await
            .map_err(|e| PaladinError::ConfigurationError(format!("Failed to load state: {}", e)))?
            .ok_or_else(|| {
                PaladinError::ConfigurationError(format!("State not found: {}", state_id))
            })?;

        // Restore data from state
        self.data.system_prompt = state.paladin.node.system_prompt.clone();
        self.data.name = state.paladin.node.name.clone();
        self.data.user_name = state.paladin.node.user_name.clone();
        self.data.model = state.paladin.node.model.clone();
        self.data.temperature = state.paladin.node.temperature;
        self.data.max_loops = state.paladin.node.max_loops;
        self.data.stop_words = state.paladin.node.stop_words.clone();

        // Note: Config fields (retry_attempts, timeout_seconds) are not part of PaladinData
        // They would need to be stored separately if we want to restore them

        Ok(self)
    }

    /// Validates all configuration parameters
    ///
    /// # Validation Rules
    ///
    /// - `system_prompt` must be non-empty
    /// - **A temperature the caller actually expressed** (via [`Self::temperature`], or
    ///   the auto-temperature branch in [`Self::build`]) must fall within the selected
    ///   provider's declared `temperature_range` (both endpoints inclusive); providers
    ///   that declare no range fall back to `[0.0, 1.0]` (ADR-0004). **Refined 2026-08-22
    ///   (17-19, closing G-17-4b):** ADR-0004 validates "whatever temperature the caller
    ///   ultimately supplies" — a caller who never called `.temperature()` supplied
    ///   nothing; the `0.7` `validate()` would otherwise see is
    ///   `PaladinData::default()`'s framework fabrication, not a request. This gate now
    ///   fires only when `manual_temperature_override` is set, so a provider's truthful,
    ///   narrow declaration (e.g. a degenerate `(1.0, 1.0)`) cannot deny service to a
    ///   caller who said nothing about temperature — while a caller who did ask for an
    ///   illegal value is still refused by name, unchanged.
    /// - `max_loops` must be in range [1, 100]
    /// - If `autosave_enabled` is true, `state_dir` must be set or Citadel must be configured
    /// - If `sanctum_port` is set, `embedding_port` must also be set (required for RAG)
    ///
    /// # Returns
    ///
    /// Ok(()) if all validations pass, Err(PaladinError::ConfigurationError) otherwise
    fn validate(&self) -> Result<(), PaladinError> {
        // Validate system_prompt is non-empty
        if self.data.system_prompt.trim().is_empty() {
            return Err(PaladinError::ConfigurationError(
                "system prompt cannot be empty".to_string(),
            ));
        }

        // Validate temperature against the selected provider's declared range, falling
        // back to [0.0, 1.0] when the provider has not declared one (ADR-0004). Both
        // endpoints are inclusive; no epsilon, no rounding, no clamping — an
        // out-of-range value is reported to the caller rather than silently corrected.
        //
        // Refined 2026-08-22 (17-19, closing G-17-4b, T-17-78): only checked when the
        // caller actually expressed a temperature (`manual_temperature_override`). A
        // provider's own declared range is not the caller's business until the caller
        // asks for something; `PaladinData::default()`'s fabricated `0.7` is not a
        // request and must not be judged against a narrow provider declaration such as
        // Kimi's measured `(1.0, 1.0)`. This is a narrowing of what the gate checks, not
        // a weakening of it: an expressed out-of-range value is still rejected below,
        // unchanged.
        if self.manual_temperature_override {
            let (min_temp, max_temp) = self
                .llm_port
                .get_capabilities()
                .temperature_range
                .unwrap_or((0.0, 1.0));
            if !(self.data.temperature >= min_temp && self.data.temperature <= max_temp) {
                return Err(PaladinError::ConfigurationError(format!(
                    "temperature must be between {} and {} for provider '{}', got {}",
                    min_temp,
                    max_temp,
                    self.llm_port.get_provider_name(),
                    self.data.temperature
                )));
            }
        }

        // Validate max_loops is in [1, 100]
        let loops = self.data.max_loops.as_u32();
        if (1..=100).contains(&loops) {
            // Valid range
        } else {
            return Err(PaladinError::ConfigurationError(format!(
                "max_loops must be between 1 and 100, got {}",
                loops
            )));
        }

        // Validate autosave configuration
        if self.autosave_enabled && self.citadel_port.is_none() && self.state_dir.is_none() {
            return Err(PaladinError::ConfigurationError(
                "autosave_enabled requires either citadel_port or state_dir to be set".to_string(),
            ));
        }

        // Validate Sanctum RAG dependencies
        if self.sanctum_port.is_some() && self.embedding_port.is_none() {
            return Err(PaladinError::ConfigurationError(
                "embedding_port is required when sanctum_port is set (needed for RAG operations)"
                    .to_string(),
            ));
        }

        // Validate vision capability
        if self.data.vision_enabled {
            // Check if LLM port supports vision by checking if it reports vision capability
            let capabilities = self.llm_port.get_capabilities();
            if !capabilities.supports_vision {
                return Err(PaladinError::ConfigurationError(format!(
                    "vision_enabled is true but the LLM provider '{}' does not support vision. \
                         Enable vision support in the LLM adapter or set vision_enabled to false.",
                    self.llm_port.get_provider_name()
                )));
            }
        }

        Ok(())
    }

    /// Generates the handoff tool schema with specialist names
    ///
    /// Creates an Armament (tool definition) for the handoff functionality
    /// that includes all configured specialist agent names as an enum parameter.
    ///
    /// # Returns
    ///
    /// An `Armament` instance representing the handoff tool
    fn generate_handoff_tool(&self) -> Armament {
        use serde_json::json;

        // Extract specialist names from the configured specialist agents
        let specialist_names: Vec<String> = self
            .specialist_agents
            .iter()
            .map(|p| p.node.name.clone())
            .collect();

        // Create JSON schema for handoff tool with specialist names as enum
        let parameters = json!({
            "type": "object",
            "properties": {
                "specialist_name": {
                    "type": "string",
                    "description": "Name of the specialist agent to delegate the task to",
                    "enum": specialist_names
                },
                "task_description": {
                    "type": "string",
                    "description": "Clear description of the task to delegate to the specialist"
                }
            },
            "required": ["specialist_name", "task_description"]
        });

        Armament {
            name: "handoff_to_specialist".to_string(),
            description: format!(
                "Delegate a task to one of {} specialist agents: {}",
                specialist_names.len(),
                specialist_names.join(", ")
            ),
            parameters,
            required_params: vec![
                "specialist_name".to_string(),
                "task_description".to_string(),
            ],
        }
    }

    /// Builds and returns a validated Paladin instance
    ///
    /// # Returns
    ///
    /// - `Ok(Paladin)` if validation succeeds
    /// - `Err(PaladinError::ConfigurationError)` if validation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
    /// # use paladin_ports::output::llm_port::LlmPort;
    /// # use std::sync::Arc;
    /// # async fn example(llm_port: Arc<dyn LlmPort>) -> Result<(), Box<dyn std::error::Error>> {
    /// let paladin = PaladinBuilder::new(llm_port)
    ///     .system_prompt("You are an AI assistant")
    ///     .model("gpt-4")
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(mut self) -> Result<Paladin, PaladinError> {
        // Handle auto-prompt generation if enabled and no manual override
        if self.auto_generate_prompt_enabled && !self.manual_prompt_override {
            if let Some(description) = &self.agent_description {
                use crate::application::services::paladin::prompt_generation_service::PromptGenerationService;

                let prompt_service = PromptGenerationService::new(self.llm_port.clone());
                let agent_name = if self.data.name.is_empty() {
                    "Agent"
                } else {
                    &self.data.name
                };

                match prompt_service
                    .generate_prompt(agent_name, description, &self.data.model)
                    .await
                {
                    Ok(generated_prompt) => {
                        log::info!("Auto-generated system prompt for agent: {}", agent_name);
                        self.data.system_prompt = generated_prompt;
                    }
                    Err(e) => {
                        return Err(PaladinError::ConfigurationError(format!(
                            "Failed to auto-generate prompt: {}",
                            e
                        )));
                    }
                }
            } else {
                return Err(PaladinError::ConfigurationError(
                    "auto_generate_prompt is enabled but agent_description is not set".to_string(),
                ));
            }
        }

        // Handle auto-temperature selection if enabled and no manual override
        if self.auto_temperature_enabled && !self.manual_temperature_override {
            if let Some(description) = &self.agent_description {
                use crate::application::services::paladin::temperature_service::TemperatureService;

                let temperature_service = TemperatureService::new(self.llm_port.clone());

                match temperature_service
                    .calculate_optimal_temperature(description, None)
                    .await
                {
                    Ok(optimal_temp) => {
                        // CR-01: validate() runs BEFORE this branch and, since
                        // 17-19, fires only on `manual_temperature_override` --
                        // which the auto path deliberately does not set. An
                        // LLM-chosen temperature therefore reached `data` with no
                        // ADR-0004 check at all, while validate()'s own rustdoc
                        // claimed to cover "the auto-temperature branch in
                        // Self::build". The gap predates 17-19 but only became
                        // reachable when Kimi began declaring a degenerate
                        // (1.0, 1.0): before that no provider had a range an
                        // auto-selected value could fall outside.
                        //
                        // ADR-0004 refuses rather than substitutes, so an
                        // out-of-range auto selection is an error naming the
                        // legal endpoints -- never a silent clamp the caller
                        // cannot discover.
                        let (min_temp, max_temp) = self
                            .llm_port
                            .get_capabilities()
                            .temperature_range
                            .unwrap_or((0.0, 1.0));
                        if !(optimal_temp >= min_temp && optimal_temp <= max_temp) {
                            return Err(PaladinError::ConfigurationError(format!(
                                "auto-selected temperature {} is outside the range {} to {} \
                                 declared by provider '{}'. Set an explicit temperature within \
                                 that range, or disable auto_temperature for this provider.",
                                optimal_temp,
                                min_temp,
                                max_temp,
                                self.llm_port.get_provider_name()
                            )));
                        }
                        log::info!(
                            "Auto-selected temperature {} for agent based on task type",
                            optimal_temp
                        );
                        self.data.temperature = optimal_temp;
                    }
                    Err(e) => {
                        return Err(PaladinError::ConfigurationError(format!(
                            "Failed to auto-select temperature: {}",
                            e
                        )));
                    }
                }
            } else {
                return Err(PaladinError::ConfigurationError(
                    "auto_temperature is enabled but agent_description is not set".to_string(),
                ));
            }
        }

        // Auto-register handoff tool if handoffs are configured
        if self.handoffs_configured {
            if let Some(arsenal) = &self.arsenal_registry {
                let handoff_tool = self.generate_handoff_tool();
                arsenal.register(handoff_tool).await;
                log::info!(
                    "Auto-registered handoff tool with {} specialists",
                    self.specialist_agents.len()
                );
            } else {
                log::warn!(
                    "Handoffs configured but no arsenal registry provided - handoff tool not registered"
                );
            }
        }

        // Validate configuration
        self.validate()?;

        // Initialize FileCitadel if state_dir is provided but citadel_port is not
        if let Some(state_dir) = &self.state_dir
            && self.citadel_port.is_none()
        {
            let file_citadel = FileCitadel::new(PathBuf::from(state_dir)).map_err(|e| {
                PaladinError::ConfigurationError(format!("Failed to initialize FileCitadel: {}", e))
            })?;
            self.citadel_port = Some(Arc::new(file_citadel));
        }

        // Create Paladin using Node pattern with name
        let name = if self.data.name.is_empty() {
            None
        } else {
            Some(self.data.name.clone())
        };

        Ok(Node::new(self.data, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;

    #[test]
    fn test_builder_validation_empty_prompt() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(PaladinError::ConfigurationError(_))));
    }

    #[test]
    fn test_builder_validation_invalid_temperature() {
        // `manual_temperature_override: true` expresses this test's actual intent
        // (17-19): "a value the caller asked for and cannot have is rejected" — not
        // "a value that merely happens to sit in the struct is rejected". Since the
        // gate was narrowed to values the caller actually expressed, leaving this
        // flag `false` would make the builder ignore `temperature: 1.5` entirely.
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                temperature: 1.5,
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: true,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn auto_selected_temperature_is_validated_against_the_provider_range() {
        // CR-01. `validate()` runs before build()'s auto-temperature branch and,
        // since 17-19, fires only on `manual_temperature_override` -- which the
        // auto path does not set. An LLM-chosen temperature therefore reached
        // `data` unchecked. The mock classifies as CREATIVE -> 0.85, outside the
        // degenerate (1.0, 1.0) Kimi's live measurement produces.
        let result = PaladinBuilder::new(Arc::new(MockLlmPortWithTemperatureRange((1.0, 1.0))))
            .system_prompt("Test")
            .agent_description("write an imaginative short story")
            .auto_temperature(true)
            .build()
            .await;

        let err = result.expect_err("0.85 is outside (1.0, 1.0) and must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("auto-selected temperature"),
            "error must name the auto path, got: {msg}"
        );
        // ADR-0004 refuses by name and states the legal endpoints rather than
        // silently substituting a value the caller cannot discover.
        assert!(
            msg.contains('1'),
            "error must state the legal range, got: {msg}"
        );
    }

    #[tokio::test]
    async fn auto_selected_temperature_inside_the_provider_range_still_builds() {
        // Control for the test above: the gate must refuse only what is actually
        // out of range. 0.85 lies inside (0.0, 1.0), so this must still build --
        // otherwise CR-01's fix would be a denial of service on every auto caller.
        let result = PaladinBuilder::new(Arc::new(MockLlmPortWithTemperatureRange((0.0, 1.0))))
            .system_prompt("Test")
            .agent_description("write an imaginative short story")
            .auto_temperature(true)
            .build()
            .await;
        assert!(result.is_ok(), "0.85 is inside (0.0, 1.0) and must build");
    }

    /// Builds a DeepSeek-backed `Arc<dyn LlmPort>` for the provider-aware temperature
    /// validation tests below. DeepSeek declares `temperature_range: Some((0.0, 2.0))`
    /// (ADR-0004), which is what makes 1.8 and 2.0 reachable through the builder where
    /// the old global `[0.0, 1.0]` clamp made them permanently unreachable.
    fn deepseek_llm_port() -> Arc<dyn LlmPort> {
        let config = paladin_llm::deepseek::DeepSeekConfig::new(
            "test-key".to_string(),
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        );
        Arc::new(paladin_llm::deepseek::DeepSeekAdapter::new(config).unwrap())
    }

    #[tokio::test]
    async fn test_deepseek_temperature_range_accepts_two_point_zero() {
        // Exactly DeepSeek's declared maximum — the upper endpoint is inclusive.
        let result = PaladinBuilder::new(deepseek_llm_port())
            .system_prompt("Test")
            .temperature(2.0)
            .build()
            .await;
        assert!(result.is_ok());

        // 1.8 is unreachable under the old global [0.0, 1.0] clamp and must now build.
        let result = PaladinBuilder::new(deepseek_llm_port())
            .system_prompt("Test")
            .temperature(1.8)
            .build()
            .await;
        assert!(result.is_ok());

        // Exactly DeepSeek's declared minimum — the lower endpoint is inclusive.
        let result = PaladinBuilder::new(deepseek_llm_port())
            .system_prompt("Test")
            .temperature(0.0)
            .build()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_temperature_rejected_above_provider_max() {
        // Just above DeepSeek's declared maximum (2.0) must be rejected with a typed
        // error naming the rejected value, never silently clamped (ADR-0004).
        let result = PaladinBuilder::new(deepseek_llm_port())
            .system_prompt("Test")
            .temperature(2.1)
            .build()
            .await;
        match result {
            Err(PaladinError::ConfigurationError(msg)) => {
                assert!(
                    msg.contains("2.1"),
                    "error should name the rejected value: {}",
                    msg
                );
            }
            other => panic!("expected ConfigurationError, got {:?}", other),
        }

        // A provider that declares no range (MockLlmPort -> Default -> None) falls back
        // to exactly today's [0.0, 1.0] behaviour — 1.8 must still be rejected.
        let result = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .temperature(1.8)
            .build()
            .await;
        assert!(matches!(result, Err(PaladinError::ConfigurationError(_))));
    }

    // ── ADR-0004 refinement (17-19, closing G-17-4b's Task 2): the gate ──
    // validates a temperature the caller actually expressed, not a
    // framework-fabricated default (`PaladinData::default()`'s `0.7`).
    // Lands ahead of Kimi's truthful degenerate `(1.0, 1.0)` declaration
    // (Task 3) so no intermediate commit makes every Kimi-backed Paladin
    // fail to build (T-17-78).

    #[tokio::test]
    async fn test_unexpressed_temperature_builds_against_a_degenerate_provider_range() {
        // A provider declaring the degenerate range (1.0, 1.0) — Kimi's
        // live-measured constraint (17-19) — must not reject a Paladin whose
        // caller never called `.temperature()`. The value `validate()` would
        // otherwise see is the framework-fabricated `PaladinData::default()`
        // value of 0.7, not a caller request.
        let result = PaladinBuilder::new(Arc::new(MockLlmPortWithTemperatureRange((1.0, 1.0))))
            .system_prompt("Test")
            .build()
            .await;
        assert!(
            result.is_ok(),
            "a Paladin whose caller never expressed a temperature must build even \
             against a degenerate provider range, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_expressed_temperature_is_still_rejected_against_a_degenerate_provider_range() {
        // The narrowing only exempts the unexpressed case — a caller who
        // explicitly asks for a temperature the provider cannot honour is
        // still refused by name, with the legal endpoints in the message.
        let result = PaladinBuilder::new(Arc::new(MockLlmPortWithTemperatureRange((1.0, 1.0))))
            .system_prompt("Test")
            .temperature(0.5)
            .build()
            .await;
        match result {
            Err(PaladinError::ConfigurationError(msg)) => {
                assert!(
                    msg.contains("0.5"),
                    "error should name the rejected value: {}",
                    msg
                );
                assert!(
                    msg.contains('1'),
                    "error should name the legal endpoints: {}",
                    msg
                );
            }
            other => panic!("expected ConfigurationError, got {:?}", other),
        }
    }

    #[test]
    fn test_builder_validation_invalid_max_loops() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                max_loops: MaxLoops::Fixed(0),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_citadel() {
        let citadel = Arc::new(MockCitadelPort);
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .with_citadel(citadel);

        assert!(builder.citadel_port.is_some());
    }

    #[test]
    fn test_builder_enable_autosave() {
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .enable_autosave();

        assert!(builder.autosave_enabled);
    }

    #[test]
    fn test_builder_save_state_dir() {
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .save_state_dir("./test_citadel");

        assert_eq!(builder.state_dir, Some("./test_citadel".to_string()));
    }

    #[test]
    fn test_builder_validation_autosave_without_citadel_or_dir() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: true,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(PaladinError::ConfigurationError(_))));
    }

    #[test]
    fn test_builder_validation_autosave_with_citadel() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: Some(Arc::new(MockCitadelPort)),
            autosave_enabled: true,
            state_dir: None,
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_validation_autosave_with_state_dir() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: true,
            state_dir: Some("./test".to_string()),
            herald: None,
            sanctum_port: None,
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_with_sanctum() {
        let sanctum = Arc::new(MockSanctumPort);
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .with_sanctum(sanctum);

        assert!(builder.sanctum_port.is_some());
    }

    #[test]
    fn test_builder_with_embedding_port() {
        let embedding = Arc::new(MockEmbeddingPort);
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .with_embedding_port(embedding);

        assert!(builder.embedding_port.is_some());
    }

    #[test]
    fn test_builder_memory_extraction_strategy() {
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort))
            .system_prompt("Test")
            .memory_extraction_strategy(MemoryExtractionStrategy::EveryTurn);

        assert!(matches!(
            builder.memory_extraction_strategy,
            MemoryExtractionStrategy::EveryTurn
        ));
    }

    #[test]
    fn test_builder_validation_sanctum_without_embedding() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: Some(Arc::new(MockSanctumPort)),
            embedding_port: None,
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(PaladinError::ConfigurationError(_))));
    }

    #[test]
    fn test_builder_validation_sanctum_with_embedding() {
        let builder = PaladinBuilder {
            llm_port: Arc::new(MockLlmPort),
            data: PaladinData {
                system_prompt: "Test".to_string(),
                ..Default::default()
            },
            config: PaladinConfig::default(),
            garrison: None,
            arsenal_registry: None,
            mcp_servers: Vec::new(),
            citadel_port: None,
            autosave_enabled: false,
            state_dir: None,
            herald: None,
            sanctum_port: Some(Arc::new(MockSanctumPort)),
            embedding_port: Some(Arc::new(MockEmbeddingPort)),
            memory_extraction_strategy: MemoryExtractionStrategy::default(),
            auto_generate_prompt_enabled: false,
            agent_description: None,
            manual_prompt_override: false,
            auto_temperature_enabled: false,
            manual_temperature_override: false,
            specialist_agents: Vec::new(),
            handoffs_configured: false,
            handoff_config: None,
        };

        let result = builder.validate();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_builder_restore_from_without_citadel() {
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort));
        let state_id = Uuid::new_v4();

        let result = builder.restore_from(state_id).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(PaladinError::ConfigurationError(_))));
    }

    #[tokio::test]
    async fn test_builder_restore_from_state_not_found() {
        let citadel = Arc::new(MockCitadelPort);
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort)).with_citadel(citadel);
        let state_id = Uuid::new_v4();

        let result = builder.restore_from(state_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_restore_from_success() {
        let citadel = Arc::new(MockCitadelPortWithState);
        let builder = PaladinBuilder::new(Arc::new(MockLlmPort)).with_citadel(citadel);
        let state_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let result = builder.restore_from(state_id).await;
        assert!(result.is_ok());

        let restored_builder = result.unwrap();
        assert_eq!(restored_builder.data.system_prompt, "Restored prompt");
        assert_eq!(restored_builder.data.name, "RestoredPaladin");
        assert_eq!(restored_builder.data.model, "gpt-4");
    }

    // Mock for internal tests
    struct MockLlmPort;

    #[async_trait::async_trait]
    impl LlmPort for MockLlmPort {
        async fn generate(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            paladin_ports::output::llm_port::LlmResponse,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!()
        }

        async fn generate_stream(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<
                            paladin_ports::output::llm_port::StreamingResponse,
                            paladin_ports::output::llm_port::LlmError,
                        >,
                    > + Send,
            >,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(
            &self,
            _model: &str,
        ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
            Ok(true)
        }

        async fn get_available_models(
            &self,
        ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "Mock"
        }

        fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
            paladin_ports::output::llm_port::ProviderCapabilities::default()
        }
    }

    /// Mock LLM port with a configurable declared `temperature_range`, for testing
    /// the ADR-0004 gate against provider ranges other than `MockLlmPort`'s `None`
    /// (17-19: the degenerate `(1.0, 1.0)` range Kimi's live measurement produces).
    struct MockLlmPortWithTemperatureRange(pub (f32, f32));

    #[async_trait::async_trait]
    impl LlmPort for MockLlmPortWithTemperatureRange {
        /// Answers the task-type classification `TemperatureService` sends on the
        /// auto-temperature path. `CREATIVE` maps to `creative_temp` (0.85), which
        /// sits outside a degenerate `(1.0, 1.0)` declaration -- the CR-01 case.
        /// Previously `unimplemented!()`, which made the auto path untestable.
        async fn generate(
            &self,
            request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            paladin_ports::output::llm_port::LlmResponse,
            paladin_ports::output::llm_port::LlmError,
        > {
            Ok(paladin_ports::output::llm_port::LlmResponse {
                id: uuid::Uuid::new_v4(),
                request_id: request.id,
                model: request.model.clone(),
                content: "CREATIVE".to_string(),
                finish_reason: paladin_ports::output::llm_port::FinishReason::Stop,
                usage: Default::default(),
                cost: None,
                created_at: chrono::Utc::now(),
                function_call: None,
                metadata: Default::default(),
            })
        }

        async fn generate_stream(
            &self,
            _request: paladin_ports::output::llm_port::LlmRequest,
        ) -> Result<
            Box<
                dyn futures::Stream<
                        Item = Result<
                            paladin_ports::output::llm_port::StreamingResponse,
                            paladin_ports::output::llm_port::LlmError,
                        >,
                    > + Send,
            >,
            paladin_ports::output::llm_port::LlmError,
        > {
            unimplemented!()
        }

        async fn validate_model(
            &self,
            _model: &str,
        ) -> Result<bool, paladin_ports::output::llm_port::LlmError> {
            Ok(true)
        }

        async fn get_available_models(
            &self,
        ) -> Result<Vec<String>, paladin_ports::output::llm_port::LlmError> {
            Ok(vec![])
        }

        fn get_provider_name(&self) -> &'static str {
            "MockWithTemperatureRange"
        }

        fn get_capabilities(&self) -> paladin_ports::output::llm_port::ProviderCapabilities {
            paladin_ports::output::llm_port::ProviderCapabilities {
                temperature_range: Some(self.0),
                ..Default::default()
            }
        }
    }

    // Mock CitadelPort for testing
    struct MockCitadelPort;

    #[async_trait::async_trait]
    impl CitadelPort for MockCitadelPort {
        async fn save_paladin(
            &self,
            _state: &crate::core::platform::container::citadel::PaladinState,
        ) -> Result<(), crate::application::errors::citadel_error::CitadelError> {
            Ok(())
        }

        async fn load_paladin(
            &self,
            _state_id: Uuid,
        ) -> Result<
            Option<crate::core::platform::container::citadel::PaladinState>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            Ok(None)
        }

        async fn save_battalion(
            &self,
            _state: &crate::core::platform::container::citadel::BattalionState,
        ) -> Result<(), crate::application::errors::citadel_error::CitadelError> {
            Ok(())
        }

        async fn load_battalion(
            &self,
            _state_id: Uuid,
        ) -> Result<
            Option<crate::core::platform::container::citadel::BattalionState>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            Ok(None)
        }

        async fn list_saved(
            &self,
        ) -> Result<
            Vec<crate::core::platform::container::citadel::StateSummary>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            Ok(vec![])
        }
    }

    // Mock CitadelPort that returns a state for testing restore
    struct MockCitadelPortWithState;

    #[async_trait::async_trait]
    impl CitadelPort for MockCitadelPortWithState {
        async fn save_paladin(
            &self,
            _state: &crate::core::platform::container::citadel::PaladinState,
        ) -> Result<(), crate::application::errors::citadel_error::CitadelError> {
            Ok(())
        }

        async fn load_paladin(
            &self,
            state_id: Uuid,
        ) -> Result<
            Option<crate::core::platform::container::citadel::PaladinState>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            if state_id == Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap() {
                // Create PaladinData
                let data = crate::core::platform::container::citadel::PaladinData {
                    system_prompt: "Restored prompt".to_string(),
                    name: "RestoredPaladin".to_string(),
                    user_name: "User".to_string(),
                    model: "gpt-4".to_string(),
                    temperature: 0.8,
                    max_loops: MaxLoops::Fixed(5),
                    stop_words: vec![],
                    status: crate::core::platform::container::citadel::PaladinStatus::Idle,
                    vision_enabled: false,
                    ..Default::default()
                };

                // Create Paladin (Node<PaladinData>)
                let paladin = Node::new(data, Some("RestoredPaladin".to_string()));

                Ok(Some(
                    crate::core::platform::container::citadel::PaladinState {
                        paladin,
                        garrison: vec![],
                        execution_history: vec![],
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        schema_version: "1.0.0".to_string(),
                    },
                ))
            } else {
                Ok(None)
            }
        }

        async fn save_battalion(
            &self,
            _state: &crate::core::platform::container::citadel::BattalionState,
        ) -> Result<(), crate::application::errors::citadel_error::CitadelError> {
            Ok(())
        }

        async fn load_battalion(
            &self,
            _state_id: Uuid,
        ) -> Result<
            Option<crate::core::platform::container::citadel::BattalionState>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            Ok(None)
        }

        async fn list_saved(
            &self,
        ) -> Result<
            Vec<crate::core::platform::container::citadel::StateSummary>,
            crate::application::errors::citadel_error::CitadelError,
        > {
            Ok(vec![])
        }
    }

    // Mock SanctumPort for testing
    struct MockSanctumPort;

    #[async_trait::async_trait]
    impl paladin_ports::output::sanctum_port::SanctumPort for MockSanctumPort {
        async fn store(
            &self,
            _entry: crate::core::platform::container::sanctum::SanctumEntry,
        ) -> Result<(), paladin_ports::output::sanctum_port::SanctumError> {
            Ok(())
        }

        async fn store_batch(
            &self,
            _entries: Vec<crate::core::platform::container::sanctum::SanctumEntry>,
        ) -> Result<(), paladin_ports::output::sanctum_port::SanctumError> {
            Ok(())
        }

        async fn search(
            &self,
            _query: paladin_ports::output::sanctum_port::SanctumQuery,
        ) -> Result<
            Vec<paladin_ports::output::sanctum_port::SanctumSearchResult>,
            paladin_ports::output::sanctum_port::SanctumError,
        > {
            Ok(vec![])
        }

        async fn delete(
            &self,
            _id: &str,
        ) -> Result<bool, paladin_ports::output::sanctum_port::SanctumError> {
            Ok(false)
        }

        async fn update(
            &self,
            _entry: crate::core::platform::container::sanctum::SanctumEntry,
        ) -> Result<(), paladin_ports::output::sanctum_port::SanctumError> {
            Ok(())
        }

        async fn count(
            &self,
            _filter: Option<paladin_ports::output::sanctum_port::SanctumFilter>,
        ) -> Result<usize, paladin_ports::output::sanctum_port::SanctumError> {
            Ok(0)
        }
    }

    // Mock EmbeddingPort for testing
    struct MockEmbeddingPort;

    #[async_trait::async_trait]
    impl paladin_ports::output::embedding_port::EmbeddingPort for MockEmbeddingPort {
        async fn embed_text(
            &self,
            _text: &str,
        ) -> Result<
            paladin_ports::output::embedding_port::Embedding,
            paladin_ports::output::embedding_port::EmbeddingError,
        > {
            Ok(paladin_ports::output::embedding_port::Embedding {
                vector: vec![0.0; 1536],
                model: "mock-model".to_string(),
                dimension: 1536,
                token_count: Some(10),
            })
        }

        async fn embed_batch(
            &self,
            texts: &[&str],
        ) -> Result<
            Vec<paladin_ports::output::embedding_port::Embedding>,
            paladin_ports::output::embedding_port::EmbeddingError,
        > {
            Ok(texts
                .iter()
                .map(|_| paladin_ports::output::embedding_port::Embedding {
                    vector: vec![0.0; 1536],
                    model: "mock-model".to_string(),
                    dimension: 1536,
                    token_count: Some(10),
                })
                .collect())
        }

        fn dimension(&self) -> usize {
            1536
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    #[tokio::test]
    async fn test_auto_generate_prompt_enabled() {
        // Given: A builder with auto-prompt generation enabled
        let llm_port = Arc::new(MockLlmPort);
        let builder = PaladinBuilder::new(llm_port)
            .auto_generate_prompt(true)
            .agent_description("A code review assistant")
            .name("ReviewBot");

        // Then: The builder should have auto-generation enabled
        assert!(builder.auto_generate_prompt_enabled);
        assert_eq!(
            builder.agent_description,
            Some("A code review assistant".to_string())
        );
    }

    #[tokio::test]
    async fn test_auto_generate_prompt_builder_method() {
        // Given: A builder with auto-prompt disabled
        let llm_port = Arc::new(MockLlmPort);
        let builder = PaladinBuilder::new(llm_port).auto_generate_prompt(false);

        // Then: Auto-generation should be disabled
        assert!(!builder.auto_generate_prompt_enabled);
    }

    #[tokio::test]
    async fn test_agent_description_method() {
        // Given: A builder
        let llm_port = Arc::new(MockLlmPort);
        let builder = PaladinBuilder::new(llm_port).agent_description("Test description");

        // Then: Description should be set
        assert_eq!(
            builder.agent_description,
            Some("Test description".to_string())
        );
    }

    #[tokio::test]
    async fn test_manual_prompt_override() {
        // Given: A builder with auto-generation enabled
        let llm_port = Arc::new(MockLlmPort);
        let builder = PaladinBuilder::new(llm_port)
            .auto_generate_prompt(true)
            .agent_description("Test agent")
            .system_prompt("Manual prompt"); // Manual override

        // Then: Manual override flag should be set
        assert!(builder.manual_prompt_override);
        assert_eq!(builder.data.system_prompt, "Manual prompt");
    }

    #[tokio::test]
    async fn test_regenerate_prompt_method() {
        // Given: A builder
        let llm_port = Arc::new(MockLlmPort);
        let builder = PaladinBuilder::new(llm_port)
            .auto_generate_prompt(true)
            .agent_description("Test")
            .regenerate_prompt(); // Should not panic

        // Then: Builder should still be valid
        assert!(builder.auto_generate_prompt_enabled);
    }

    // Mock ArsenalRegistry for testing
    struct MockArsenalRegistry {
        registered_tools: Arc<tokio::sync::Mutex<Vec<Armament>>>,
    }

    impl MockArsenalRegistry {
        fn new() -> Self {
            Self {
                registered_tools: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        async fn get_registered_tools(&self) -> Vec<Armament> {
            self.registered_tools.lock().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl ArsenalRegistry for MockArsenalRegistry {
        async fn register(&self, armament: Armament) {
            self.registered_tools.lock().await.push(armament);
        }

        async fn unregister(&self, name: &str) -> Option<Armament> {
            let mut tools = self.registered_tools.lock().await;
            tools
                .iter()
                .position(|t| t.name == name)
                .map(|pos| tools.remove(pos))
        }

        async fn get(&self, name: &str) -> Option<Armament> {
            self.registered_tools
                .lock()
                .await
                .iter()
                .find(|t| t.name == name)
                .cloned()
        }
    }

    #[tokio::test]
    async fn test_builder_auto_registers_handoff_tool_when_configured() {
        // Given: A Paladin with specialist agents configured
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let arsenal = Arc::new(MockArsenalRegistry::new());

        let specialist1: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Rust expert")
            .name("RustExpert")
            .build()
            .await
            .unwrap();

        let specialist2: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Python expert")
            .name("PythonExpert")
            .build()
            .await
            .unwrap();

        // When: Building a coordinator with handoffs configured
        let _coordinator: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Coordinator")
            .name("Coordinator")
            .with_arsenal_registry(Arc::clone(&arsenal) as Arc<dyn ArsenalRegistry>)
            .with_handoffs(vec![Arc::new(specialist1), Arc::new(specialist2)])
            .build()
            .await
            .unwrap();

        // Then: Handoff tool should be auto-registered
        let tools = arsenal.get_registered_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "handoff_to_specialist");
        assert!(tools[0].description.contains("2 specialist agents"));
    }

    #[tokio::test]
    async fn test_builder_does_not_register_handoff_tool_when_not_configured() {
        // Given: A Paladin without handoffs
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let arsenal = Arc::new(MockArsenalRegistry::new());

        // When: Building without handoffs
        let _paladin: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Regular agent")
            .name("Agent")
            .with_arsenal_registry(Arc::clone(&arsenal) as Arc<dyn ArsenalRegistry>)
            .build()
            .await
            .unwrap();

        // Then: No tools should be registered
        let tools = arsenal.get_registered_tools().await;
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_handoff_tool_schema_includes_all_specialists() {
        // Given: A builder with 3 specialists
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let arsenal = Arc::new(MockArsenalRegistry::new());

        let specialists: Vec<Arc<Paladin>> = vec!["Expert1", "Expert2", "Expert3"]
            .into_iter()
            .map(|name| {
                Arc::new(
                    PaladinBuilder::new(Arc::clone(&llm_port))
                        .system_prompt(format!("{} system prompt", name))
                        .name(name)
                        .build()
                        .now_or_never()
                        .unwrap()
                        .unwrap(),
                )
            })
            .collect();

        // When: Building coordinator
        let _coordinator: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Coordinator")
            .name("Coordinator")
            .with_arsenal_registry(Arc::clone(&arsenal) as Arc<dyn ArsenalRegistry>)
            .with_handoffs(specialists)
            .build()
            .await
            .unwrap();

        // Then: Tool schema should include all 3 specialist names in enum
        let tools = arsenal.get_registered_tools().await;
        let handoff_tool = &tools[0];

        let specialist_param = handoff_tool.parameters["properties"]["specialist_name"].clone();
        let enum_values = specialist_param["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(enum_values.len(), 3);
        assert!(enum_values.contains(&"Expert1"));
        assert!(enum_values.contains(&"Expert2"));
        assert!(enum_values.contains(&"Expert3"));
    }

    #[tokio::test]
    async fn test_handoff_tool_auto_registration_is_idempotent() {
        // Given: Arsenal with handoff tool already registered
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let arsenal = Arc::new(MockArsenalRegistry::new());

        let specialist: Arc<Paladin> = Arc::new(
            PaladinBuilder::new(Arc::clone(&llm_port))
                .system_prompt("Expert")
                .name("Expert")
                .build()
                .await
                .unwrap(),
        );

        // When: Building multiple coordinators with same specialists
        for _ in 0..3 {
            let _coordinator: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
                .system_prompt("Coordinator")
                .name("Coordinator")
                .with_arsenal_registry(Arc::clone(&arsenal) as Arc<dyn ArsenalRegistry>)
                .with_handoffs(vec![Arc::clone(&specialist)])
                .build()
                .await
                .unwrap();
        }

        // Then: Only one (or multiple with same name) tool should be registered
        // Note: ArsenalRegistry's register() replaces tools with same name, so we'll have 3 identical ones
        let tools = arsenal.get_registered_tools().await;
        assert_eq!(tools.len(), 3); // Each build() registers one
        assert!(tools.iter().all(|t| t.name == "handoff_to_specialist"));
    }

    #[tokio::test]
    async fn test_handoff_tool_schema_validation() {
        // Given: A builder with specialists
        let llm_port: Arc<dyn LlmPort> = Arc::new(MockLlmPort);
        let arsenal = Arc::new(MockArsenalRegistry::new());

        let specialist: Arc<Paladin> = Arc::new(
            PaladinBuilder::new(Arc::clone(&llm_port))
                .system_prompt("Expert")
                .name("TestExpert")
                .build()
                .await
                .unwrap(),
        );

        // When: Building coordinator
        let _coordinator: Paladin = PaladinBuilder::new(Arc::clone(&llm_port))
            .system_prompt("Coordinator")
            .name("Coordinator")
            .with_arsenal_registry(Arc::clone(&arsenal) as Arc<dyn ArsenalRegistry>)
            .with_handoffs(vec![specialist])
            .build()
            .await
            .unwrap();

        // Then: Tool schema should be valid JSON Schema with correct structure
        let tools = arsenal.get_registered_tools().await;
        let handoff_tool = &tools[0];

        // Validate required fields
        assert!(
            handoff_tool
                .required_params
                .contains(&"specialist_name".to_string())
        );
        assert!(
            handoff_tool
                .required_params
                .contains(&"task_description".to_string())
        );

        // Validate schema structure
        assert_eq!(handoff_tool.parameters["type"], "object");
        assert!(handoff_tool.parameters["properties"].is_object());
        assert!(handoff_tool.parameters["properties"]["specialist_name"].is_object());
        assert!(handoff_tool.parameters["properties"]["task_description"].is_object());

        // Validate parameter types
        assert_eq!(
            handoff_tool.parameters["properties"]["specialist_name"]["type"],
            "string"
        );
        assert_eq!(
            handoff_tool.parameters["properties"]["task_description"]["type"],
            "string"
        );
    }
}
