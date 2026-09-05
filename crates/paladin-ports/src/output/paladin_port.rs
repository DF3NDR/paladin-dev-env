//! Paladin Port - AI Agent Execution Abstraction
//!
//! This module defines the output port (interface) for Paladin agent execution following
//! Hexagonal Architecture principles. The `PaladinPort` trait provides a clean abstraction
//! that allows the application layer to execute Paladin agents without being coupled to
//! specific LLM providers or execution strategies.
//!
//! # Purpose
//!
//! The Paladin port enables the orchestration layer (Battalions, Commanders, etc.) to execute
//! individual Paladin agents while maintaining a clean separation between the domain logic
//! (what a Paladin is) and the execution logic (how it runs). This allows you to:
//!
//! - Execute Paladin agents with different LLM providers (OpenAI, DeepSeek, Anthropic)
//! - Switch between synchronous and streaming execution modes
//! - Test agent logic without real LLM calls
//! - Implement retry logic, caching, or other middleware consistently
//! - Track execution metadata (tokens, time, loops, stop reasons)
//! - Support autonomous planning and agent handoffs
//!
//! # Hexagonal Architecture (Ports & Adapters)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    Application Layer                          │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  Battalion Orchestration                                │  │
//! │  │  - Formation (sequential execution)                     │  │
//! │  │  - Phalanx (parallel execution)                         │  │
//! │  │  - Campaign (graph/DAG execution)                       │  │
//! │  │  - Chain of Command (hierarchical delegation)           │  │
//! │  └─────────────────────┬────────────────────────────────────┘  │
//! │                        │                                       │
//! │                        ↓                                       │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  PaladinPort (trait)                                    │  │
//! │  │  - execute()                                            │  │
//! │  │  - execute_stream()                                     │  │
//! │  │  - validate()                                           │  │
//! │  └────────────────────┬───────────────────────────────────┘  │
//! └─────────────────────────┼────────────────────────────────────┘
//!                          │
//!                          ↓
//!   ┌──────────────────────────────────────────────────────────┐
//!   │  PaladinExecutionService (adapter)                        │
//!   │  - LlmPort integration                                    │
//!   │  - GarrisonPort (memory)                                  │
//!   │  - ArsenalPort (tools)                                    │
//!   │  - Autonomous planning & handoff logic                    │
//!   └──────────────────────────────────────────────────────────┘
//! ```
//!
//! # Paladin Execution Model
//!
//! A Paladin executes in a reasoning loop:
//!
//! 1. **Build Prompt**: System prompt + Garrison context + User input
//! 2. **LLM Call**: Generate response via LlmPort
//! 3. **Check Stop Conditions**: Stop words, max loops, timeout
//! 4. **Tool Execution**: If response contains tool calls, execute via Arsenal
//! 5. **Update Context**: Store results in Garrison
//! 6. **Repeat**: Loop back to step 1 until stop condition
//!
//! ## Execution Modes
//!
//! - **Standard Mode**: Fixed number of loops (e.g., `MaxLoops::Fixed(3)`)
//! - **Autonomous Mode**: Auto-planned subtasks (`MaxLoops::Auto`)
//! - **Streaming**: Real-time output chunks via `execute_stream()`
//! - **Batch**: Collect full output via `execute()`
//!
//! # Common Use Cases
//!
//! ## 1. Execute Paladin Agent
//!
//! ```ignore
//! use paladin_ports::output::paladin_port::{
//!     PaladinPort, PaladinResult, StopReason
//! };
//! use paladin_core::platform::container::paladin::Paladin;
//! use std::sync::Arc;
//!
//! async fn run_agent(
//!     paladin_port: Arc<dyn PaladinPort>,
//!     paladin: &Paladin,
//! ) -> Result<String, Box<dyn std::error::Error>> {
//!     // Validate configuration
//!     paladin_port.validate(paladin)?;
//!
//!     // Execute agent
//!     let result = paladin_port
//!         .execute(paladin, "Analyze the attached data and summarize findings")
//!         .await?;
//!
//!     println!("Output: {}", result.output);
//!     println!("Tokens: {}", result.token_count);
//!     println!("Time: {}ms", result.execution_time_ms);
//!     println!("Loops: {}", result.loop_count);
//!     println!("Stop reason: {:?}", result.stop_reason);
//!
//!     if result.stop_reason.is_successful() {
//!         Ok(result.output)
//!     } else {
//!         Err(format!("Execution failed: {:?}", result.stop_reason).into())
//!     }
//! }
//! ```
//!
//! ## 2. Streaming Execution for Real-Time UI
//!
//! ```ignore
//! use paladin_ports::output::paladin_port::{
//!     PaladinPort, PaladinStreamChunk
//! };
//! use paladin_core::platform::container::paladin::Paladin;
//! use std::sync::Arc;
//!
//! async fn stream_agent_output(
//!     paladin_port: Arc<dyn PaladinPort>,
//!     paladin: &Paladin,
//! ) {
//!     let mut stream = paladin_port
//!         .execute_stream(paladin, "Write a detailed report")
//!         .await
//!         .unwrap();
//!
//!     // Stream chunks as they arrive
//!     while let Some(chunk_result) = stream.recv().await {
//!         match chunk_result {
//!             Ok(chunk) => {
//!                 print!("{}", chunk.text); // Print incrementally
//!                 std::io::Write::flush(&mut std::io::stdout()).unwrap();
//!
//!                 if chunk.is_final {
//!                     println!("\n[Execution complete]");
//!                     break;
//!                 }
//!             }
//!             Err(e) => {
//!                 eprintln!("Error: {}", e);
//!                 break;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## 3. Check Autonomous Planning Metadata
//!
//! ```ignore
//! use paladin_ports::output::paladin_port::PaladinResult;
//!
//! fn analyze_execution(result: &PaladinResult) {
//!     // Check if autonomous planning was used
//!     if result.has_plan() {
//!         let plan = result.plan.as_ref().unwrap();
//!         println!("Task was decomposed into {} subtasks:", plan.subtask_count());
//!         for (i, subtask) in plan.subtasks.iter().enumerate() {
//!             println!(
//!                 "  {}. {} -> {}",
//!                 i + 1,
//!                 subtask.description,
//!                 subtask.actual_result.as_ref().unwrap_or(&"Pending".to_string())
//!             );
//!         }
//!     }
//!
//!     // Check if agent handoffs occurred
//!     if result.has_handoffs() {
//!         println!("Agent delegated {} tasks:", result.handoff_count());
//!         for handoff in &result.handoff_history {
//!             println!(
//!                 "  {} → {}: {}",
//!                 handoff.from_agent, handoff.to_agent, handoff.task
//!             );
//!         }
//!     }
//! }
//! ```
//!
//! ## 4. Retry on Failure
//!
//! ```ignore
//! use paladin_ports::output::paladin_port::{
//!     PaladinPort, PaladinResult, StopReason
//! };
//! use paladin_core::platform::container::paladin_error::PaladinError;
//! use std::sync::Arc;
//! use std::time::Duration;
//! use tokio::time::sleep;
//!
//! async fn execute_with_retry(
//!     paladin_port: Arc<dyn PaladinPort>,
//!     paladin: &paladin_core::platform::container::paladin::Paladin,
//!     input: &str,
//!     max_retries: u32,
//! ) -> Result<PaladinResult, PaladinError> {
//!     let mut attempts = 0;
//!     let mut backoff = Duration::from_millis(100);
//!
//!     loop {
//!         match paladin_port.execute(paladin, input).await {
//!             Ok(result) if result.stop_reason.is_successful() => return Ok(result),
//!             Ok(result) if result.stop_reason == StopReason::Timeout && attempts < max_retries => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!                 attempts += 1;
//!             }
//!             Ok(result) => return Ok(result), // Accept partial results after retries
//!             Err(e) if attempts >= max_retries => return Err(e),
//!             Err(PaladinError::LlmError(_)) => {
//!                 sleep(backoff).await;
//!                 backoff *= 2;
//!                 attempts += 1;
//!             }
//!             Err(e) => return Err(e), // Non-retryable error
//!         }
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! PaladinPort methods return `PaladinError` which includes:
//!
//! | Error | Retryable? | Recovery Strategy |
//! |-------|------------|-------------------|
//! | ConfigurationError | No | Fix Paladin configuration (system prompt, model, etc.) |
//! | ExecutionError | Maybe | Check error message, retry if transient |
//! | Timeout | Yes | Increase timeout or reduce max_loops |
//! | LlmError | Yes | Retry with exponential backoff |
//! | StopWordDetected | No | This is success, check stop word in result |
//!
//! ## Stop Reasons
//!
//! The `StopReason` enum indicates why execution terminated:
//!
//! - `Completed`: Natural completion (success)
//! - `StopWord(String)`: Configured stop word detected (success)
//! - `MaxLoops`: Loop limit reached (partial success)
//! - `Timeout`: Execution time limit exceeded (partial success)
//!
//! Use `StopReason::is_successful()` to check if the result is acceptable.
//!
//! # Thread Safety
//!
//! PaladinPort is `Send + Sync`, allowing safe use across async task boundaries.
//! This is critical for Battalion orchestration where multiple Paladins may execute
//! concurrently (Phalanx pattern).
//!
//! # Implementation Notes
//!
//! ## Adapter Implementation Checklist
//!
//! When implementing a Paladin adapter:
//!
//! 1. **Validation**: Implement `validate()` to check system prompt, model, max_loops, timeout
//! 2. **Reasoning Loop**: Build prompt, call LLM, check stop conditions, repeat
//! 3. **Garrison Integration**: Fetch conversation history, store results
//! 4. **Arsenal Integration**: Detect tool calls in LLM response, execute tools
//! 5. **Stop Word Detection**: Check output for configured stop words
//! 6. **Timeout Handling**: Enforce timeout configuration
//! 7. **Token Counting**: Track prompt + completion tokens for cost estimation
//! 8. **Execution Timing**: Measure total execution time
//! 9. **Streaming Support**: Yield chunks incrementally for `execute_stream()`
//! 10. **Autonomous Planning**: Support `MaxLoops::Auto` for task decomposition
//! 11. **Handoff Support**: Track agent-to-agent delegation
//! 12. **Error Mapping**: Map LLM provider errors to `PaladinError`
//!
//! ## Performance Considerations
//!
//! - **Garrison Caching**: Cache Garrison context for multiple executions
//! - **Prompt Optimization**: Minimize prompt size to reduce token usage
//! - **Streaming for Long Tasks**: Use `execute_stream()` for tasks > 30s
//! - **Concurrent Execution**: Execute multiple Paladins in parallel (Phalanx)
//! - **Tool Call Batching**: Batch multiple tool calls when possible
//! - **Timeout Configuration**: Set reasonable timeouts (30-300s typical)
//!
//! ## Testing Strategy
//!
//! ```ignore
//! use paladin_ports::output::paladin_port::{
//!     PaladinPort, PaladinResult, PaladinStream, StopReason
//! };
//! use paladin_core::platform::container::paladin_error::PaladinError;
//! use paladin_core::platform::container::paladin::Paladin;
//! use async_trait::async_trait;
//!
//! /// Mock Paladin port for testing
//! struct MockPaladinPort {
//!     output: String,
//!     should_fail: bool,
//! }
//!
//! #[async_trait]
//! impl PaladinPort for MockPaladinPort {
//!     async fn execute(
//!         &self,
//!         paladin: &Paladin,
//!         input: &str,
//!     ) -> Result<PaladinResult, PaladinError> {
//!         if self.should_fail {
//!             return Err(PaladinError::ExecutionError("Mock failure".into()));
//!         }
//!
//!         Ok(PaladinResult {
//!             output: self.output.clone(),
//!             token_count: 100,
//!             execution_time_ms: 500,
//!             loop_count: 1,
//!             stop_reason: StopReason::Completed,
//!             ..Default::default()
//!         })
//!     }
//!
//!     async fn execute_stream(
//!         &self,
//!         paladin: &Paladin,
//!         input: &str,
//!     ) -> Result<PaladinStream, PaladinError> {
//!         let (mut tx, rx) = tokio::sync::mpsc::channel(10);
//!         let output = self.output.clone();
//!
//!         tokio::spawn(async move {
//!             // Send chunks
//!             for chunk in output.split_whitespace() {
//!                 let _ = tx
//!                     .send(Ok(PaladinStreamChunk {
//!                         text: format!("{} ", chunk),
//!                         is_final: false,
//!                         metadata: None,
//!                     }))
//!                     .await;
//!             }
//!
//!             // Send final chunk
//!             let _ = tx
//!                 .send(Ok(PaladinStreamChunk {
//!                     text: String::new(),
//!                     is_final: true,
//!                     metadata: None,
//!                 }))
//!                 .await;
//!         });
//!
//!         Ok(rx)
//!     }
//!
//!     fn validate(&self, paladin: &Paladin) -> Result<(), PaladinError> {
//!         if paladin.system_prompt.is_empty() {
//!             Err(PaladinError::ConfigurationError(
//!                 "System prompt cannot be empty".into(),
//!             ))
//!         } else {
//!             Ok(())
//!         }
//!     }
//! }
//! ```
//!
//! # Common Pitfalls
//!
//! 1. **Not Calling validate()**: Always validate before execute() to catch config errors early
//! 2. **Ignoring Stop Reason**: Check `stop_reason` to determine if output is complete
//! 3. **Token Counting**: Don't forget to include both prompt and completion tokens
//! 4. **Streaming Cleanup**: Ensure stream channel is properly closed on error
//! 5. **Timeout Too Short**: Allow enough time for LLM calls and tool executions
//! 6. **Missing Tool Support**: If Arsenal is configured, handle tool calls in loop
//! 7. **No Garrison Context**: Include conversation history for context-aware responses
//! 8. **Blocking Execution**: All operations must be async to avoid blocking
//!
//! # Related Modules
//!
//! - [`paladin_core::platform::container::paladin`] - Paladin domain entity
//! - [`paladin_battalion::paladin`] - Paladin execution service (adapter)
//! - [`paladin_ports::output::llm_port`] - LLM provider integration
//! - [`paladin_ports::output::garrison_port`] - Conversation memory
//! - [`paladin_ports::output::arsenal_port`] - Tool execution
//! - [`paladin_battalion::battalion`] - Multi-agent orchestration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;

// Re-export pure domain result types from core
pub use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};

/// Streaming chunk from Paladin execution
///
/// Represents a single chunk of output during streaming execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaladinStreamChunk {
    /// The text chunk
    pub text: String,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Optional metadata for this chunk
    pub metadata: Option<ChunkMetadata>,
}

/// Metadata for a streaming chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Tokens in this chunk
    pub tokens: Option<u32>,

    /// Current loop iteration
    pub loop_count: Option<u32>,
}

/// Type alias for Paladin streaming receiver
///
/// Receives chunks of output as they are generated.
pub type PaladinStream = mpsc::Receiver<Result<PaladinStreamChunk, PaladinError>>;

/// Port abstraction for Paladin execution
///
/// This trait defines the interface that any Paladin execution implementation
/// must satisfy. It follows the hexagonal architecture pattern, allowing the
/// core domain to remain independent of execution details.
///
/// # Capabilities
///
/// - **Synchronous Execution**: Execute and collect full output (`execute()`)
/// - **Streaming Execution**: Stream output chunks in real-time (`execute_stream()`)
/// - **Configuration Validation**: Verify Paladin configuration before execution (`validate()`)
/// - **Stop Condition Handling**: Detect stop words, max loops, timeouts
/// - **Metadata Tracking**: Return execution stats (tokens, time, loops, stop reason)
/// - **Autonomous Planning**: Support auto-planned task decomposition
/// - **Agent Handoffs**: Track delegation to specialist agents
///
/// # Requirements
///
/// Implementations must:
/// - Be `Send + Sync` for safe concurrent use across async tasks
/// - Integrate with LlmPort for LLM provider calls
/// - Integrate with GarrisonPort for conversation memory (optional)
/// - Integrate with ArsenalPort for tool execution (optional)
/// - Enforce timeout configuration
/// - Track token usage for cost estimation
/// - Measure execution time
/// - Handle errors gracefully with detailed error context
///
/// # Examples
///
/// ## Basic Implementation Pattern
///
/// ```ignore
/// use async_trait::async_trait;
/// use paladin_ports::output::paladin_port::{
///     PaladinPort, PaladinResult, PaladinStream, StopReason
/// };
/// use paladin_core::platform::container::paladin::Paladin;
/// use paladin_core::platform::container::paladin_error::PaladinError;
/// use paladin_ports::output::llm_port::LlmPort;
/// use std::sync::Arc;
///
/// struct MyPaladinExecutor {
///     llm_port: Arc<dyn LlmPort>,
/// }
///
/// #[async_trait]
/// impl PaladinPort for MyPaladinExecutor {
///     async fn execute(
///         &self,
///         paladin: &Paladin,
///         input: &str,
///     ) -> Result<PaladinResult, PaladinError> {
///         // 1. Validate configuration
///         self.validate(paladin)?;
///
///         // 2. Build prompt (system + user input)
///         let prompt = format!("{}\n\nUser: {}", paladin.system_prompt, input);
///
///         // 3. Execute reasoning loop
///         let mut output = String::new();
///         let mut loop_count = 0;
///         let start_time = std::time::Instant::now();
///
///         while loop_count < paladin.max_loops {
///             // Call LLM
///             let response = self.llm_port.generate(&prompt).await?;
///             output.push_str(&response.content);
///             loop_count += 1;
///
///             // Check stop conditions
///             if response.content.contains(&paladin.stop_words[0]) {
///                 break;
///             }
///         }
///
///         // 4. Return result
///         Ok(PaladinResult {
///             output,
///             token_count: 500, // Track actual usage
///             execution_time_ms: start_time.elapsed().as_millis() as u64,
///             loop_count,
///             stop_reason: StopReason::Completed,
///             ..Default::default()
///         })
///     }
///
///     async fn execute_stream(
///         &self,
///         paladin: &Paladin,
///         input: &str,
///     ) -> Result<PaladinStream, PaladinError> {
///         // Streaming implementation (simplified)
///         let (_tx, rx) = tokio::sync::mpsc::channel(10);
///         Ok(rx)
///     }
///
///     fn validate(&self, paladin: &Paladin) -> Result<(), PaladinError> {
///         if paladin.system_prompt.is_empty() {
///             return Err(PaladinError::ConfigurationError(
///                 "System prompt is required".into()
///             ));
///         }
///         if paladin.max_loops == 0 {
///             return Err(PaladinError::ConfigurationError(
///                 "Max loops must be > 0".into()
///             ));
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Reasoning Loop Pattern
///
/// The core execution pattern is a reasoning loop:
///
/// ```ignore
/// loop {
///     // 1. Build prompt with context
///     let prompt = build_prompt(system_prompt, garrison_context, user_input);
///
///     // 2. Call LLM
///     let response = llm_port.generate(&prompt).await?;
///
///     // 3. Check stop words
///     for stop_word in &paladin.stop_words {
///         if response.content.contains(stop_word) {
///             return Ok(PaladinResult { stop_reason: StopReason::StopWord(stop_word.clone()), ... });
///         }
///     }
///
///     // 4. Execute tool calls (if Arsenal configured)
///     if let Some(tool_call) = extract_tool_call(&response) {
///         let tool_result = arsenal_port.execute(&tool_call).await?;
///         garrison_port.store(&tool_result).await?;
///     }
///
///     // 5. Update garrison
///     garrison_port.store_message(&response).await?;
///
///     // 6. Check loop limit
///     loop_count += 1;
///     if loop_count >= paladin.max_loops {
///         return Ok(PaladinResult { stop_reason: StopReason::MaxLoops, ... });
///     }
///
///     // 7. Check timeout
///     if start_time.elapsed() > timeout {
///         return Ok(PaladinResult { stop_reason: StopReason::Timeout, ... });
///     }
/// }
/// ```
///
/// ## Streaming Implementation
///
/// For `execute_stream()`, yield chunks as they arrive:
///
/// ```ignore
/// async fn execute_stream(&self, paladin: &Paladin, input: &str) -> Result<PaladinStream, PaladinError> {
///     let (tx, rx) = tokio::sync::mpsc::channel(100);
///
///     let llm_port = self.llm_port.clone();
///     let paladin = paladin.clone();
///     let input = input.to_string();
///
///     tokio::spawn(async move {
///         // Execute reasoning loop, sending chunks
///         match llm_port.generate_stream(&prompt).await {
///             Ok(mut stream) => {
///                 while let Some(chunk) = stream.recv().await {
///                     let _ = tx.send(Ok(PaladinStreamChunk {
///                         text: chunk.content,
///                         is_final: false,
///                         metadata: Some(ChunkMetadata {
///                             tokens: Some(chunk.tokens),
///                             loop_count: Some(current_loop),
///                         }),
///                     })).await;
///                 }
///
///                 // Send final chunk
///                 let _ = tx.send(Ok(PaladinStreamChunk {
///                     text: String::new(),
///                     is_final: true,
///                     metadata: None,
///                 })).await;
///             }
///             Err(e) => {
///                 let _ = tx.send(Err(PaladinError::from(e))).await;
///             }
///         }
///     });
///
///     Ok(rx)
/// }
/// ```
///
/// # Performance Tips
///
/// 1. **Cache Garrison Context**: Fetch conversation history once per execution
/// 2. **Batch Tool Calls**: Execute multiple tool calls concurrently when possible
/// 3. **Streaming for Long Tasks**: Use `execute_stream()` for tasks > 30 seconds
/// 4. **Timeout Configuration**: Set reasonable timeouts (30-300s) based on task complexity
/// 5. **Prompt Optimization**: Keep system prompts concise to reduce token usage
/// 6. **LLM Provider Choice**: Use faster models (GPT-4o-mini, DeepSeek) for simpler tasks
#[async_trait]
pub trait PaladinPort: Send + Sync {
    /// Execute a Paladin with the given input
    ///
    /// Runs the Paladin's reasoning loop until completion, timeout, or stop word detection.
    /// This method collects the full output and returns it along with execution metadata.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin entity to execute
    /// * `input` - The user input/task to process
    ///
    /// # Returns
    ///
    /// Returns a `PaladinResult` containing:
    /// - `output`: The generated text response
    /// - `token_count`: Total tokens used (prompt + completion)
    /// - `execution_time_ms`: Total execution time in milliseconds
    /// - `loop_count`: Number of reasoning loops executed
    /// - `stop_reason`: Why execution terminated
    /// - `plan`: Optional task plan (for autonomous mode)
    /// - `handoff_history`: Optional delegation records
    ///
    /// # Errors
    ///
    /// - `PaladinError::ConfigurationError` - Invalid Paladin configuration
    /// - `PaladinError::ExecutionError` - Error during execution
    /// - `PaladinError::Timeout` - Execution exceeded timeout
    /// - `PaladinError::LlmError` - LLM provider error
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::paladin_port::PaladinPort;
    /// use std::sync::Arc;
    ///
    /// async fn run_paladin(
    ///     port: Arc<dyn PaladinPort>,
    ///     paladin: &paladin_core::platform::container::paladin::Paladin,
    /// ) {
    ///     let result = port
    ///         .execute(paladin, "Analyze sales data and provide insights")
    ///         .await
    ///         .unwrap();
    ///
    ///     println!("Output: {}", result.output);
    ///     println!("Used {} tokens in {}ms", result.token_count, result.execution_time_ms);
    ///
    ///     if !result.stop_reason.is_successful() {
    ///         eprintln!("Warning: Execution stopped due to: {:?}", result.stop_reason);
    ///     }
    /// }
    /// ```
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>;

    /// Execute a Paladin with streaming output
    ///
    /// Similar to `execute()` but returns a stream of chunks as they are generated.
    /// This is useful for real-time user interfaces, long-running tasks, or when you
    /// want to display progress incrementally.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin entity to execute
    /// * `input` - The user input/task to process
    ///
    /// # Returns
    ///
    /// Returns a `PaladinStream` (mpsc::Receiver) that yields `PaladinStreamChunk` results.
    /// Each chunk contains:
    /// - `text`: Incremental text output
    /// - `is_final`: Whether this is the last chunk
    /// - `metadata`: Optional chunk metadata (tokens, loop count)
    ///
    /// # Errors
    ///
    /// Errors are delivered through the stream as `Err(PaladinError)`. Same error
    /// variants as `execute()`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::paladin_port::{
    ///     PaladinPort, PaladinStreamChunk
    /// };
    /// use std::sync::Arc;
    ///
    /// async fn stream_paladin(
    ///     port: Arc<dyn PaladinPort>,
    ///     paladin: &paladin_core::platform::container::paladin::Paladin,
    /// ) {
    ///     let mut stream = port
    ///         .execute_stream(paladin, "Write a detailed report on market trends")
    ///         .await
    ///         .unwrap();
    ///
    ///     let mut full_output = String::new();
    ///
    ///     while let Some(chunk_result) = stream.recv().await {
    ///         match chunk_result {
    ///             Ok(chunk) => {
    ///                 print!("{}", chunk.text); // Display incrementally
    ///                 full_output.push_str(&chunk.text);
    ///
    ///                 if chunk.is_final {
    ///                     println!("\n[Complete]");
    ///                     break;
    ///                 }
    ///             }
    ///             Err(e) => {
    ///                 eprintln!("Stream error: {}", e);
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Implementation Note
    ///
    /// The stream should be closed (drop sender) when execution completes or encounters
    /// an error. Ensure proper cleanup to avoid resource leaks.
    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError>;

    /// Validate a Paladin's configuration
    ///
    /// Checks that the Paladin is properly configured before execution.
    /// This should be called before `execute()` or `execute_stream()` to catch
    /// configuration errors early.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin entity to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `PaladinError::ConfigurationError` with specific details.
    ///
    /// # Validation Checks
    ///
    /// Implementations should validate:
    /// - System prompt is not empty
    /// - Max loops > 0
    /// - Timeout > 0 (if configured)
    /// - Model name is valid
    /// - Temperature in range [0, 2]
    /// - Stop words are not empty (if configured)
    ///
    /// # Errors
    ///
    /// - `PaladinError::ConfigurationError` - Invalid configuration with specific details
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::paladin_port::PaladinPort;
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use std::sync::Arc;
    ///
    /// async fn validate_before_execution(
    ///     port: Arc<dyn PaladinPort>,
    ///     paladin: &Paladin,
    /// ) {
    ///     // Validate first
    ///     match port.validate(paladin) {
    ///         Ok(_) => println!("Configuration valid, proceeding..."),
    ///         Err(e) => {
    ///             eprintln!("Invalid configuration: {}", e);
    ///             return;
    ///         }
    ///     }
    ///
    ///     // Safe to execute
    ///     let result = port.execute(paladin, "Task input").await.unwrap();
    /// }
    /// ```
    fn validate(&self, paladin: &Paladin) -> Result<(), PaladinError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plan 25-09, D-19 / X-10.4: `execute_observed` is a DEFAULTED method
    /// whose default body delegates to `execute` and beats nothing. A port
    /// that never overrides it produces exactly what `execute` produces,
    /// and the handle records zero beats -- the default correctly claims
    /// no progress, so an `idle_timeout` over such a port degrades to a
    /// per-attempt wall clock rather than to no bound at all.
    #[tokio::test]
    async fn execute_observed_defaults_to_execute() {
        use paladin_core::platform::container::heartbeat::HeartbeatHandle;
        use paladin_core::platform::container::paladin::PaladinData;

        struct ExecuteOnlyPort;

        #[async_trait]
        impl PaladinPort for ExecuteOnlyPort {
            async fn execute(
                &self,
                _paladin: &Paladin,
                input: &str,
            ) -> Result<PaladinResult, PaladinError> {
                Ok(PaladinResult {
                    output: format!("echo:{input}"),
                    token_count: 7,
                    loop_count: 1,
                    stop_reason: StopReason::Completed,
                    ..Default::default()
                })
            }

            async fn execute_stream(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<PaladinStream, PaladinError> {
                unimplemented!("not exercised")
            }

            fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
                Ok(())
            }
        }

        let port = ExecuteOnlyPort;
        let paladin: Paladin = paladin_core::base::entity::node::Node::new(
            PaladinData::default(),
            Some("p".to_string()),
        );
        let heartbeat = HeartbeatHandle::new();

        let direct = port.execute(&paladin, "hi").await.unwrap();
        let observed = port
            .execute_observed(&paladin, "hi", &heartbeat)
            .await
            .unwrap();

        assert_eq!(observed.output, direct.output);
        assert_eq!(observed.token_count, direct.token_count);
        assert_eq!(observed.loop_count, direct.loop_count);
        assert_eq!(observed.stop_reason, direct.stop_reason);
        assert_eq!(heartbeat.beats(), 0, "the default body emits no beat");
    }

    #[test]
    fn test_paladin_result_creation() {
        let result = PaladinResult {
            output: "Test output".to_string(),
            token_count: 100,
            execution_time_ms: 500,
            loop_count: 2,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
            served_by: None,
        };

        assert_eq!(result.output, "Test output");
        assert_eq!(result.token_count, 100);
        assert_eq!(result.loop_count, 2);
        assert!(!result.has_plan());
        assert!(!result.has_handoffs());
    }

    #[test]
    fn test_stop_reason_is_successful() {
        assert!(StopReason::Completed.is_successful());
        assert!(StopReason::StopWord("DONE".to_string()).is_successful());
        assert!(!StopReason::MaxLoops.is_successful());
        assert!(!StopReason::Timeout.is_successful());
    }

    #[test]
    fn test_stop_reason_is_limit() {
        assert!(StopReason::MaxLoops.is_limit());
        assert!(StopReason::Timeout.is_limit());
        assert!(!StopReason::Completed.is_limit());
        assert!(!StopReason::StopWord("DONE".to_string()).is_limit());
    }

    #[test]
    fn test_stop_reason_equality() {
        assert_eq!(StopReason::Completed, StopReason::Completed);
        assert_eq!(StopReason::MaxLoops, StopReason::MaxLoops);
        assert_eq!(
            StopReason::StopWord("STOP".to_string()),
            StopReason::StopWord("STOP".to_string())
        );
        assert_ne!(StopReason::Completed, StopReason::MaxLoops);
    }

    #[test]
    fn test_paladin_stream_chunk() {
        let chunk = PaladinStreamChunk {
            text: "Hello ".to_string(),
            is_final: false,
            metadata: Some(ChunkMetadata {
                tokens: Some(2),
                loop_count: Some(1),
            }),
        };

        assert_eq!(chunk.text, "Hello ");
        assert!(!chunk.is_final);
        assert!(chunk.metadata.is_some());
    }

    #[test]
    fn test_chunk_metadata() {
        let metadata = ChunkMetadata {
            tokens: Some(10),
            loop_count: Some(3),
        };

        assert_eq!(metadata.tokens, Some(10));
        assert_eq!(metadata.loop_count, Some(3));
    }

    #[test]
    fn test_paladin_result_serialization() {
        let result = PaladinResult {
            output: "Test".to_string(),
            token_count: 50,
            execution_time_ms: 250,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: Vec::new(),
            served_by: None,
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        let deserialized: PaladinResult =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(result.output, deserialized.output);
        assert_eq!(result.token_count, deserialized.token_count);
        assert_eq!(result.stop_reason, deserialized.stop_reason);
    }

    #[test]
    fn test_paladin_result_default_values() {
        let result = PaladinResult::default();

        assert_eq!(result.output, "");
        assert_eq!(result.token_count, 0);
        assert_eq!(result.execution_time_ms, 0);
        assert_eq!(result.loop_count, 0);
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert!(result.plan.is_none());
        assert!(result.handoff_history.is_empty());
        assert!(!result.has_plan());
        assert!(!result.has_handoffs());
    }

    #[test]
    fn test_paladin_result_with_plan_metadata() {
        use paladin_core::platform::container::planning::{Subtask, TaskPlan};

        let mut plan = TaskPlan::new("Test task".to_string(), 5);
        plan.add_subtask(Subtask::new(
            "st-1".to_string(),
            "First subtask".to_string(),
            "Expected output".to_string(),
        ))
        .expect("Failed to add subtask");

        let result = PaladinResult {
            output: "Task completed".to_string(),
            token_count: 200,
            execution_time_ms: 1000,
            loop_count: 3,
            stop_reason: StopReason::Completed,
            plan: Some(plan.clone()),
            handoff_history: Vec::new(),
            served_by: None,
        };

        assert!(result.has_plan());
        assert_eq!(result.plan.as_ref().unwrap().original_task, "Test task");
        assert_eq!(result.plan.as_ref().unwrap().subtask_count(), 1);
        assert!(!result.has_handoffs());
    }

    #[test]
    fn test_paladin_result_with_handoff_history() {
        use paladin_core::platform::container::handoff::HandoffRecord;

        let mut record1 = HandoffRecord::new(
            "Coordinator".to_string(),
            "RustExpert".to_string(),
            "Debug Rust code".to_string(),
            1,
        );
        record1.set_result("Code debugged successfully".to_string());

        let record2 = HandoffRecord::new(
            "RustExpert".to_string(),
            "TestExpert".to_string(),
            "Write unit tests".to_string(),
            2,
        );

        let result = PaladinResult {
            output: "All tasks completed".to_string(),
            token_count: 500,
            execution_time_ms: 3000,
            loop_count: 5,
            stop_reason: StopReason::Completed,
            plan: None,
            handoff_history: vec![record1, record2],
            served_by: None,
        };

        assert!(!result.has_plan());
        assert!(result.has_handoffs());
        assert_eq!(result.handoff_count(), 2);
        assert_eq!(result.handoff_history[0].from_agent, "Coordinator");
        assert_eq!(result.handoff_history[0].to_agent, "RustExpert");
        assert_eq!(
            result.handoff_history[0].result.as_ref().unwrap(),
            "Code debugged successfully"
        );
        assert_eq!(result.handoff_history[1].depth, 2);
    }

    #[test]
    fn test_paladin_result_serialization_with_new_fields() {
        use paladin_core::platform::container::handoff::HandoffRecord;
        use paladin_core::platform::container::planning::{Subtask, TaskPlan};

        let mut plan = TaskPlan::new("Complex task".to_string(), 3);
        plan.add_subtask(Subtask::new(
            "st-1".to_string(),
            "Step 1".to_string(),
            "Output 1".to_string(),
        ))
        .expect("Failed to add subtask");

        let record = HandoffRecord::new(
            "Agent1".to_string(),
            "Agent2".to_string(),
            "Subtask".to_string(),
            1,
        );

        let result = PaladinResult {
            output: "Final output".to_string(),
            token_count: 300,
            execution_time_ms: 2000,
            loop_count: 4,
            stop_reason: StopReason::Completed,
            plan: Some(plan),
            handoff_history: vec![record],
            served_by: None,
        };

        // Serialize
        let json = serde_json::to_string(&result).expect("Failed to serialize");
        assert!(json.contains("\"plan\""));
        assert!(json.contains("\"handoff_history\""));
        assert!(json.contains("Complex task"));
        assert!(json.contains("Agent1"));

        // Deserialize
        let deserialized: PaladinResult =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.output, "Final output");
        assert!(deserialized.has_plan());
        assert!(deserialized.has_handoffs());
        assert_eq!(deserialized.handoff_count(), 1);
    }

    #[test]
    fn test_paladin_result_deserialization_backward_compatibility() {
        // Old JSON format without plan and handoff_history fields
        let old_json = r#"{
            "output": "Old result",
            "token_count": 150,
            "execution_time_ms": 800,
            "loop_count": 2,
            "stop_reason": "Completed"
        }"#;

        // Should deserialize successfully with default values for new fields
        let result: PaladinResult =
            serde_json::from_str(old_json).expect("Failed to deserialize old format");

        assert_eq!(result.output, "Old result");
        assert_eq!(result.token_count, 150);
        assert_eq!(result.execution_time_ms, 800);
        assert_eq!(result.loop_count, 2);
        assert_eq!(result.stop_reason, StopReason::Completed);

        // New fields should have default values
        assert!(result.plan.is_none());
        assert!(result.handoff_history.is_empty());
        assert!(!result.has_plan());
        assert!(!result.has_handoffs());
    }

    #[test]
    fn test_paladin_result_new_constructor() {
        let result = PaladinResult::new(
            "Constructor test".to_string(),
            250,
            1500,
            3,
            StopReason::MaxLoops,
        );

        assert_eq!(result.output, "Constructor test");
        assert_eq!(result.token_count, 250);
        assert_eq!(result.execution_time_ms, 1500);
        assert_eq!(result.loop_count, 3);
        assert_eq!(result.stop_reason, StopReason::MaxLoops);
        assert!(result.plan.is_none());
        assert!(result.handoff_history.is_empty());
    }
}
