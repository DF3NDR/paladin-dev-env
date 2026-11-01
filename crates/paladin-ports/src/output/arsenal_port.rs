//! # Arsenal Port - External Tool Integration Interface
//!
//! This module defines the port interfaces for the Arsenal tool system,
//! enabling Paladins to interact with external tools and services through
//! the Model Context Protocol (MCP).
//!
//! ## Purpose
//!
//! The Arsenal ports provide a standardized interface for:
//! - **Tool Discovery**: Listing available tools from MCP servers
//! - **Tool Registration**: Managing the tool registry lifecycle
//! - **Tool Invocation**: Executing tools with validated parameters
//! - **Tool Validation**: Checking parameter schemas before execution
//!
//! Following hexagonal architecture, these traits abstract tool operations
//! from their implementations (STDIO MCP, Streamable-HTTP MCP, direct
//! integrations).
//!
//! ## Hexagonal Architecture Context
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │               Application Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  PaladinExecutionService                      │  │
//! │  │    - Uses ArsenalPort to execute tools       │  │
//! │  │    - Validates calls before invocation       │  │
//! │  └──────────────────────────────────────────────┘  │
//! │                         │                            │
//! │                         ▼                            │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  ArsenalPort & ArsenalRegistry (this module) │  │
//! │  │    - Tool execution interface                 │  │
//! │  │    - Tool registry interface                  │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────┐
//! │            Infrastructure Layer                      │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │  MCPStdioAdapter (STDIN/STDOUT MCP servers)  │  │
//! │  │  MCPStreamableHttpAdapter (remote MCP, WIP)  │  │
//! │  │  DirectToolAdapter (Native Rust tools)       │  │
//! │  └──────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync`:
//! - **Send**: Tools may be invoked from different threads
//! - **Sync**: Multiple Paladins may access the Arsenal concurrently
//! - Implementations must handle concurrent tool invocations safely
//!
//! ## Error Handling
//!
//! Tool operations can fail for several reasons:
//! - **Tool Not Found**: Requested tool doesn't exist in registry
//! - **Invalid Arguments**: Parameters don't match JSON schema
//! - **Timeout**: Tool execution exceeded time limit
//! - **Protocol Error**: MCP communication failure
//! - **Transport Error**: Network/process communication failure
//!
//! All errors are represented via [`ArsenalError`]
//! with context for debugging and recovery strategies.
//!
//! ## Common Use Cases
//!
//! ### 1. Web Search Tool
//!
//! ```rust,no_run
//! use paladin_ports::output::arsenal_port::{ArsenalPort, ArsenalRegistry};
//! use paladin_core::platform::container::arsenal::{Armament, ArmamentCall};
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! async fn search_web(
//!     arsenal: &dyn ArsenalPort,
//!     query: &str,
//! ) -> Result<String, Box<dyn std::error::Error>> {
//!     let mut args = HashMap::new();
//!     args.insert("query".to_string(), json!(query));
//!
//!     let call = ArmamentCall::new("web_search", args);
//!     arsenal.validate_call(&call)?;
//!
//!     let result = arsenal.invoke(call).await?;
//!
//!     if result.success {
//!         Ok(result.output.unwrap().to_string())
//!     } else {
//!         Err(result.error.unwrap().into())
//!     }
//! }
//! ```
//!
//! ### 2. File System Operations
//!
//! ```rust,no_run
//! use paladin_ports::output::arsenal_port::{ArsenalPort, ArsenalRegistry};
//! use paladin_core::platform::container::arsenal::ArmamentCall;
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! async fn read_file_content(
//!     arsenal: &dyn ArsenalPort,
//!     file_path: &str,
//! ) -> Result<String, Box<dyn std::error::Error>> {
//!     let mut args = HashMap::new();
//!     args.insert("path".to_string(), json!(file_path));
//!
//!     let call = ArmamentCall::new("read_file", args);
//!     let result = arsenal.invoke(call).await?;
//!
//!     Ok(serde_json::from_value(result.output.unwrap())?)
//! }
//! ```
//!
//! ### 3. MCP Tool Discovery and Registration
//!
//! ```rust,no_run
//! use paladin_ports::output::arsenal_port::ArsenalRegistry;
//! use paladin_core::platform::container::arsenal::Armament;
//! use serde_json::json;
//!
//! async fn discover_and_register_tools(
//!     registry: &dyn ArsenalRegistry,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Simulate MCP tool discovery
//!     let calculator = Armament {
//!         name: "calculator".to_string(),
//!         description: "Arithmetic operations".to_string(),
//!         parameters: json!({
//!             "type": "object",
//!             "properties": {
//!                 "operation": {"type": "string", "enum": ["add", "subtract"]},
//!                 "x": {"type": "number"},
//!                 "y": {"type": "number"}
//!             }
//!         }),
//!         required_params: vec!["operation".to_string(), "x".to_string(), "y".to_string()],
//!     };
//!
//!     registry.register(calculator).await;
//!
//!     // Verify registration
//!     if let Some(tool) = registry.get("calculator").await {
//!         println!("Registered: {} - {}", tool.name, tool.description);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### 4. Tool Listing and Discovery
//!
//! ```rust,no_run
//! use paladin_ports::output::arsenal_port::ArsenalPort;
//!
//! async fn list_available_tools(arsenal: &dyn ArsenalPort) {
//!     let tools = arsenal.list_armaments().await;
//!
//!     println!("Available tools: {}", tools.len());
//!     for tool in tools {
//!         println!("  • {} - {}", tool.name, tool.description);
//!         println!("    Required params: {:?}", tool.required_params);
//!     }
//! }
//! ```
//!
//! ## Implementation Notes
//!
//! ### MCP Protocol Integration
//!
//! The Model Context Protocol (MCP) enables standardized tool communication:
//!
//! ```rust,ignore
//! // STDIO-based MCP (command-line tools) — connect performs the full
//! // rmcp-backed handshake internally (D-01/D-04)
//! let client = MCPClient::connect_stdio("uvx", &["mcp-web-search".to_string()]).await?;
//!
//! // Streamable-HTTP-based MCP (remote, authenticated servers) lands in a
//! // follow-up plan of Phase 12.1 — the previous "SSE" adapter (plain HTTP
//! // POST, no auth) has been retired.
//!
//! // Discover tools from MCP server
//! let tools = client.discover_tools().await?;
//!
//! // Register discovered tools
//! for tool in tools {
//!     registry.register(tool).await;
//! }
//! ```
//!
//! ### Tool Validation Strategy
//!
//! Always validate tool calls before invocation to catch errors early:
//!
//! ```rust,ignore
//! // Validate before invoking
//! if let Err(e) = arsenal.validate_call(&call) {
//!     return Err(format!("Invalid tool call: {}", e));
//! }
//!
//! // Validation checks:
//! // 1. Tool exists in registry
//! // 2. All required parameters provided
//! // 3. Parameter types match JSON schema
//! // 4. Additional custom constraints (if any)
//! ```
//!
//! ### Performance Considerations
//!
//! 1. **Connection Pooling**: Reuse MCP connections across invocations
//! 2. **Timeout Configuration**: Set appropriate timeouts (5-30s typical)
//! 3. **Caching**: Cache tool metadata to avoid repeated discovery
//! 4. **Parallel Execution**: Multiple tools can run concurrently
//!
//! ### Best Practices
//!
//! 1. **Validate Early**: Use `validate_call()` before `invoke()`
//! 2. **Handle Timeouts**: Set reasonable timeouts for long-running tools
//! 3. **Log Tool Calls**: Track tool usage for debugging and cost monitoring
//! 4. **Error Recovery**: Implement retry logic for transient failures
//! 5. **Schema Versioning**: Version tool schemas for backward compatibility
//!
//! ## Common Pitfalls
//!
//! - Not validating tool calls before invocation (wasted resources)
//! - Missing required parameters in argument maps
//! - Not handling tool timeouts (blocked Paladin execution)
//! - Not checking `success` field in `ArmamentResult`
//! - Mixing JSON types (passing string when number expected)
//!
//! ## Related Modules
//!
//! - [`Armament`] - Tool metadata
//! - [`ArmamentCall`] - Tool invocation request
//! - [`ArmamentResult`] - Tool execution result
//! - [`ArsenalError`] - Error types
//! - [`LlmPort`](crate::output::llm_port::LlmPort) - LLM integration (generates tool calls)
//!
//! ## See Also
//!
//! - [ARSENAL.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/docs/ARSENAL.md) - Comprehensive Arsenal guide
//! - [MCP Specification](https://modelcontextprotocol.io) - Model Context Protocol details
//! - `examples/arsenal_stdio_tools.rs` - STDIO MCP example

use async_trait::async_trait;
use paladin_core::platform::container::arsenal::{
    Armament, ArmamentCall, ArmamentResult, ArsenalError,
};

/// Port trait for executing external tools via the Arsenal system.
///
/// Provides the interface for tool invocation, validation, and discovery.
/// Implementations handle MCP protocol communication, tool execution,
/// timeout management, and result formatting.
///
/// # Capabilities
///
/// - **Tool Discovery**: List all available tools with [`list_armaments`](Self::list_armaments)
/// - **Tool Invocation**: Execute tools with validated parameters via [`invoke`](Self::invoke)
/// - **Call Validation**: Verify tool calls before execution with [`validate_call`](Self::validate_call)
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support:
/// - Concurrent tool invocations from multiple Paladins
/// - Async execution across thread boundaries
/// - Shared access to MCP connections and tool state
///
/// # Implementation Requirements
///
/// Implementations should:
/// 1. Validate tool calls before execution (required parameters, type checking)
/// 2. Handle tool timeouts gracefully (5-30s typical, configurable)
/// 3. Manage MCP connection lifecycle (connect, reconnect, cleanup)
/// 4. Return detailed error context for debugging
/// 5. Track execution metrics (time, success rate, errors)
///
/// # Examples
///
/// ## Tool Invocation with Validation
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalPort;
/// use paladin_core::platform::container::arsenal::ArmamentCall;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// async fn execute_calculator(
///     arsenal: &dyn ArsenalPort,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let mut args = HashMap::new();
///     args.insert("operation".to_string(), json!("add"));
///     args.insert("x".to_string(), json!(10));
///     args.insert("y".to_string(), json!(5));
///
///     let call = ArmamentCall::new("calculator", args);
///
///     // Validate before invoking
///     arsenal.validate_call(&call)?;
///
///     // Execute tool
///     let result = arsenal.invoke(call).await?;
///
///     if result.success {
///         println!("Result: {:?}", result.output);
///     } else {
///         eprintln!("Tool failed: {:?}", result.error);
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Error Handling with Retry
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalPort;
/// use paladin_core::platform::container::arsenal::{ArmamentCall, ArsenalError};
///
/// async fn invoke_with_retry(
///     arsenal: &dyn ArsenalPort,
///     call: ArmamentCall,
///     max_retries: u32,
/// ) -> Result<String, ArsenalError> {
///     let mut attempts = 0;
///
///     loop {
///         match arsenal.invoke(call.clone()).await {
///             Ok(result) if result.success => {
///                 return Ok(result.output.unwrap().to_string());
///             }
///             Ok(result) => {
///                 return Err(ArsenalError::ProtocolError(
///                     result.error.unwrap_or_else(|| "Unknown error".to_string())
///                 ));
///             }
///             Err(ArsenalError::Timeout(_)) if attempts < max_retries => {
///                 attempts += 1;
///                 tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
///                 continue;
///             }
///             Err(e) => return Err(e),
///         }
///     }
/// }
/// ```
///
/// ## Listing and Discovering Tools
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalPort;
///
/// async fn discover_tools(arsenal: &dyn ArsenalPort) {
///     let tools = arsenal.list_armaments().await;
///
///     println!("Available tools: {}", tools.len());
///     for tool in tools {
///         println!("  • {} - {}", tool.name, tool.description);
///         println!("    Required: {:?}", tool.required_params);
///         println!("    Schema: {}", tool.parameters);
///     }
/// }
/// ```
///
/// ## Custom Implementation Example
///
/// ```rust
/// use paladin_ports::output::arsenal_port::ArsenalPort;
/// use paladin_core::platform::container::arsenal::{
///     Armament, ArmamentCall, ArmamentResult, ArsenalError
/// };
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use std::sync::{Arc, RwLock};
/// use serde_json::json;
///
/// struct MockArsenal {
///     tools: Arc<RwLock<HashMap<String, Armament>>>,
/// }
///
/// #[async_trait]
/// impl ArsenalPort for MockArsenal {
///     async fn list_armaments(&self) -> Vec<Armament> {
///         self.tools.read().unwrap().values().cloned().collect()
///     }
///
///     async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
///         // Simulate tool execution
///         if !self.tools.read().unwrap().contains_key(&call.tool_name) {
///             return Err(ArsenalError::ToolNotFound(call.tool_name));
///         }
///
///         Ok(ArmamentResult::success(
///             call.call_id,
///             json!({"result": "mock success"}),
///             100, // execution time ms
///         ))
///     }
///
///     fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
///         if !self.tools.read().unwrap().contains_key(&call.tool_name) {
///             return Err(ArsenalError::ToolNotFound(call.tool_name.clone()));
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## MCP Protocol Support
///
/// Implementations typically support multiple MCP transports:
///
/// - **STDIO**: Subprocess communication (Python, Node.js, CLI tools)
/// - **Streamable-HTTP**: Remote, authenticated MCP servers (lands in a
///   follow-up plan of Phase 12.1; the previous "SSE" adapter — plain HTTP
///   POST, no auth — has been retired)
/// - **Direct**: Native Rust tool implementations
///
/// ```rust,ignore
/// // STDIO example — connect performs the full rmcp handshake internally
/// let client = MCPClient::connect_stdio("python3", &["mcp_server.py".to_string()]).await?;
/// let arsenal = ArsenalExecutionService::new(client);
/// ```
///
/// ## Timeout Management
///
/// Tool execution should have configurable timeouts:
///
/// - **Short-running**: 5-10s (API calls, simple calculations)
/// - **Medium**: 10-30s (web scraping, data processing)
/// - **Long-running**: 30-120s (code generation, analysis)
///
/// ## Performance Optimization
///
/// 1. **Connection Pooling**: Reuse MCP connections
/// 2. **Parallel Execution**: Multiple tools can run concurrently
/// 3. **Caching**: Cache tool metadata from discovery
/// 4. **Lazy Loading**: Connect to MCP servers on first use
///
/// ## Error Recovery Strategies
///
/// - **Timeout**: Retry with increased timeout or fail fast
/// - **Transport Error**: Reconnect and retry once
/// - **Protocol Error**: Check MCP version compatibility
/// - **Tool Not Found**: Refresh tool registry from MCP server
///
/// # Common Pitfalls
///
/// - Not validating calls before invocation (wasted execution)
/// - Missing timeout configuration (hung Paladin execution)
/// - Not checking `success` field in result (silent failures)
/// - Blocking on synchronous tool execution (use async)
/// - Not handling MCP reconnection (fragile connections)
///
/// # See Also
///
/// - [`ArsenalRegistry`] - Tool registration and lifecycle management
/// - [`Armament`](paladin_core::platform::container::arsenal::Armament) - Tool metadata structure
/// - [`ArmamentCall`](paladin_core::platform::container::arsenal::ArmamentCall) - Invocation request
/// - [`ArmamentResult`] - Execution result
/// - [`ArsenalError`] - Error types
#[async_trait]
pub trait ArsenalPort: Send + Sync {
    /// Lists all available tools in the Arsenal.
    ///
    /// Returns a vector of all registered tools with their metadata,
    /// including name, description, parameters, and requirements.
    ///
    /// # Returns
    ///
    /// A vector of `Armament` instances representing available tools.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tools = arsenal.list_armaments().await;
    /// for tool in tools {
    ///     println!("Available: {} - {}", tool.name, tool.description);
    /// }
    /// ```
    async fn list_armaments(&self) -> Vec<Armament>;

    /// Invokes a tool with the provided arguments.
    ///
    /// Executes the specified tool with the given arguments and returns
    /// the result. This method handles tool execution, error recovery,
    /// and result formatting.
    ///
    /// # Arguments
    ///
    /// * `call` - The tool invocation request containing tool name and arguments
    ///
    /// # Returns
    ///
    /// An `ArmamentResult` containing the execution outcome, output data,
    /// and execution metrics.
    ///
    /// # Errors
    ///
    /// Returns `ArsenalError` if:
    /// - Tool is not found
    /// - Arguments are invalid
    /// - Execution times out
    /// - Communication with tool fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let call = ArmamentCall::new("calculator", args);
    /// let result = arsenal.invoke(call).await?;
    /// if result.success {
    ///     println!("Output: {:?}", result.output);
    /// }
    /// ```
    async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError>;

    /// Validates a tool call before execution.
    ///
    /// Checks that the tool exists, all required parameters are provided,
    /// and arguments match the expected JSON schema.
    ///
    /// # Arguments
    ///
    /// * `call` - The tool invocation request to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the call is valid.
    ///
    /// # Errors
    ///
    /// Returns `ArsenalError` if:
    /// - Tool does not exist
    /// - Required parameters are missing
    /// - Parameter types don't match schema
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if arsenal.validate_call(&call).is_ok() {
    ///     let result = arsenal.invoke(call).await?;
    /// }
    /// ```
    fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError>;
}

/// Port trait for managing the Arsenal tool registry.
///
/// Provides the interface for registering, unregistering, and retrieving
/// tool metadata. Implementations handle tool storage, lifecycle management,
/// and thread-safe access to the tool collection.
///
/// # Capabilities
///
/// - **Registration**: Add tools to the registry with [`register`](Self::register)
/// - **Unregistration**: Remove tools by name with [`unregister`](Self::unregister)
/// - **Lookup**: Retrieve tool metadata with [`get`](Self::get)
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support:
/// - Concurrent registration from multiple sources
/// - Safe lookup from multiple Paladins
/// - MCP server discovery updates during runtime
///
/// # Implementation Requirements
///
/// Implementations should:
/// 1. Handle concurrent access safely (use Arc<RwLock<>> or similar)
/// 2. Support idempotent registration (replace existing tools)
/// 3. Return cloned tool metadata (avoid holding locks during I/O)
/// 4. Validate tool metadata on registration (non-empty names, valid schemas)
/// 5. Track registration history for debugging (optional)
///
/// # Examples
///
/// ## Tool Registration and Lookup
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalRegistry;
/// use paladin_core::platform::container::arsenal::Armament;
/// use serde_json::json;
///
/// async fn register_calculator(
///     registry: &dyn ArsenalRegistry,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let calculator = Armament {
///         name: "calculator".to_string(),
///         description: "Performs arithmetic operations".to_string(),
///         parameters: json!({
///             "type": "object",
///             "properties": {
///                 "operation": {"type": "string", "enum": ["add", "subtract"]},
///                 "x": {"type": "number"},
///                 "y": {"type": "number"}
///             }
///         }),
///         required_params: vec!["operation".to_string(), "x".to_string(), "y".to_string()],
///     };
///
///     registry.register(calculator).await;
///
///     // Verify registration
///     if let Some(tool) = registry.get("calculator").await {
///         println!("Registered: {} - {}", tool.name, tool.description);
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## Batch Registration from MCP Discovery
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalRegistry;
/// use paladin_core::platform::container::arsenal::Armament;
///
/// async fn register_discovered_tools(
///     registry: &dyn ArsenalRegistry,
///     tools: Vec<Armament>,
/// ) {
///     println!("Registering {} tools from MCP discovery", tools.len());
///
///     for tool in tools {
///         println!("  • Registering: {}", tool.name);
///         registry.register(tool).await;
///     }
///
///     println!("Registration complete");
/// }
/// ```
///
/// ## Tool Lifecycle Management
///
/// ```rust,no_run
/// use paladin_ports::output::arsenal_port::ArsenalRegistry;
///
/// async fn replace_tool(
///     registry: &dyn ArsenalRegistry,
///     tool_name: &str,
///     new_tool: paladin_core::platform::container::arsenal::Armament,
/// ) {
///     // Unregister old version
///     if let Some(old) = registry.unregister(tool_name).await {
///         println!("Unregistered old version: {}", old.description);
///     }
///
///     // Register new version
///     registry.register(new_tool).await;
///     println!("Registered new version");
/// }
/// ```
///
/// ## Custom Implementation Example
///
/// ```rust
/// use paladin_ports::output::arsenal_port::ArsenalRegistry;
/// use paladin_core::platform::container::arsenal::Armament;
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use std::sync::{Arc, RwLock};
///
/// struct InMemoryRegistry {
///     tools: Arc<RwLock<HashMap<String, Armament>>>,
/// }
///
/// impl InMemoryRegistry {
///     pub fn new() -> Self {
///         Self {
///             tools: Arc::new(RwLock::new(HashMap::new())),
///         }
///     }
/// }
///
/// #[async_trait]
/// impl ArsenalRegistry for InMemoryRegistry {
///     async fn register(&self, armament: Armament) {
///         let mut tools = self.tools.write().unwrap();
///         tools.insert(armament.name.clone(), armament);
///     }
///
///     async fn unregister(&self, name: &str) -> Option<Armament> {
///         let mut tools = self.tools.write().unwrap();
///         tools.remove(name)
///     }
///
///     async fn get(&self, name: &str) -> Option<Armament> {
///         let tools = self.tools.read().unwrap();
///         tools.get(name).cloned()
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Storage Backend
///
/// Implementations can use different storage backends:
///
/// - **In-Memory**: `HashMap` with `Arc<RwLock<>>` (default, fast)
/// - **Persistent**: SQLite, PostgreSQL (survives restarts)
/// - **Distributed**: Redis, Consul (multi-instance deployments)
///
/// ```rust,ignore
/// // In-memory registry
/// let registry = InMemoryArsenalRegistry::new();
///
/// // Persistent registry
/// let registry = SqliteArsenalRegistry::new("arsenal.db").await?;
///
/// // Distributed registry
/// let registry = RedisArsenalRegistry::new("redis://localhost").await?;
/// ```
///
/// ## Concurrency Patterns
///
/// For thread-safe access:
///
/// ```rust,ignore
/// use std::sync::{Arc, RwLock};
/// use std::collections::HashMap;
///
/// // Multiple readers, single writer
/// let tools: Arc<RwLock<HashMap<String, Armament>>> = Arc::new(RwLock::new(HashMap::new()));
///
/// // Read (shared lock)
/// let tool = tools.read().unwrap().get("calculator").cloned();
///
/// // Write (exclusive lock)
/// tools.write().unwrap().insert("calculator".to_string(), tool);
/// ```
///
/// ## Tool Versioning Strategy
///
/// When replacing tools, consider versioning:
///
/// ```rust,ignore
/// // Option 1: Version in tool name
/// registry.register(Armament {
///     name: "calculator_v2".to_string(),
///     // ...
/// });
///
/// // Option 2: Metadata versioning
/// registry.register(Armament {
///     name: "calculator".to_string(),
///     description: "Calculator v2.0 - Supports complex numbers".to_string(),
///     // ...
/// });
/// ```
///
/// ## Best Practices
///
/// 1. **Idempotent Registration**: Allow re-registering the same tool
/// 2. **Clone on Return**: Don't expose internal storage directly
/// 3. **Validate on Register**: Check name, schema format before storing
/// 4. **Log Changes**: Track registration/unregistration for auditing
/// 5. **Support Bulk Operations**: Optimize for MCP batch discovery
///
/// ## Performance Considerations
///
/// - **Read-Heavy**: Optimize for fast lookups (RwLock, caching)
/// - **Write-Heavy**: Use channels for async registration queues
/// - **Large Registry**: Consider indexing by tags/categories
/// - **Distributed**: Cache locally, sync periodically
///
/// # Common Pitfalls
///
/// - Holding locks during async operations (deadlock risk)
/// - Not cloning on return (borrowing internal state)
/// - Missing validation on registration (corrupt registry)
/// - No cleanup on unregister (memory leaks if tools hold resources)
/// - Not handling concurrent replacement (lost updates)
///
/// # See Also
///
/// - [`ArsenalPort`] - Tool execution interface
/// - [`Armament`](paladin_core::platform::container::arsenal::Armament) - Tool metadata structure
/// - [ARSENAL.md](https://github.com/DF3NDR/paladin-dev-env/blob/main/docs/ARSENAL.md) - Comprehensive guide
#[async_trait]
pub trait ArsenalRegistry: Send + Sync {
    /// Registers a new tool in the Arsenal.
    ///
    /// Adds a tool to the registry, making it available for invocation.
    /// If a tool with the same name already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `armament` - The tool metadata to register
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tool = Armament {
    ///     name: "calculator".to_string(),
    ///     description: "Basic math operations".to_string(),
    ///     parameters: json!({"type": "object"}),
    ///     required_params: vec!["operation".to_string()],
    /// };
    /// registry.register(tool).await;
    /// ```
    async fn register(&self, armament: Armament);

    /// Unregisters a tool from the Arsenal.
    ///
    /// Removes a tool from the registry by name. Returns the removed
    /// tool metadata if it existed.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to remove
    ///
    /// # Returns
    ///
    /// `Some(Armament)` if the tool was found and removed, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(removed) = registry.unregister("calculator").await {
    ///     println!("Removed tool: {}", removed.name);
    /// }
    /// ```
    async fn unregister(&self, name: &str) -> Option<Armament>;

    /// Retrieves a tool by name.
    ///
    /// Looks up a tool in the registry and returns its metadata if found.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the tool to retrieve
    ///
    /// # Returns
    ///
    /// `Some(Armament)` if the tool exists, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(tool) = registry.get("calculator").await {
    ///     println!("Found tool: {}", tool.description);
    /// }
    /// ```
    async fn get(&self, name: &str) -> Option<Armament>;

    /// Lists all tools currently registered.
    ///
    /// Additive default method (Phase 12.1 OQ1) — returns an empty `Vec` by
    /// default so pre-existing `ArsenalRegistry` implementors (the
    /// [`InMemoryRegistry`](self) doc-example above and any test double
    /// predating this method) keep compiling without modification. Real
    /// implementations backed by actual storage (e.g.
    /// `ArsenalRegistryService`) should override this to return every
    /// registered [`Armament`].
    ///
    /// # Returns
    ///
    /// A vector of all registered tool metadata. Order is not guaranteed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let all_tools = registry.list().await;
    /// println!("{} tools registered", all_tools.len());
    /// ```
    async fn list(&self) -> Vec<Armament> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// The Phase 3 entry measurement (`.planning/phases/03-verification-depth/
// 03-coverage-measurement.md`) records `arsenal_port.rs` at 0.00% -- 2 of 2
// counted lines missed. Every other line in this file is either a doc
// comment or a trait-method signature with no body; the crate's only
// executable statements are the two lines of `ArsenalRegistry::list`'s
// default body directly above. This module gives that body its first
// caller by deliberately NOT overriding `list()` on a minimal implementor.
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Minimal in-memory `ArsenalRegistry` implementor that overrides only
    /// `register`/`unregister`/`get` and deliberately leaves `list()` on the
    /// trait's default body, so that default is what these tests exercise.
    struct MinimalRegistry {
        tools: RwLock<HashMap<String, Armament>>,
    }

    impl MinimalRegistry {
        fn new() -> Self {
            Self {
                tools: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl ArsenalRegistry for MinimalRegistry {
        async fn register(&self, armament: Armament) {
            self.tools
                .write()
                .unwrap()
                .insert(armament.name.clone(), armament);
        }

        async fn unregister(&self, name: &str) -> Option<Armament> {
            self.tools.write().unwrap().remove(name)
        }

        async fn get(&self, name: &str) -> Option<Armament> {
            self.tools.read().unwrap().get(name).cloned()
        }

        // `list` is intentionally not overridden here -- see module doc above.
    }

    fn test_armament(name: &str) -> Armament {
        Armament {
            name: name.to_string(),
            description: "test tool".to_string(),
            parameters: serde_json::json!({}),
            required_params: vec![],
        }
    }

    #[tokio::test]
    async fn list_default_body_returns_empty_vec_on_empty_registry() {
        let registry = MinimalRegistry::new();
        let tools = registry.list().await;
        assert!(
            tools.is_empty(),
            "default list() should return an empty Vec, got {tools:?}"
        );
    }

    #[tokio::test]
    async fn list_default_body_stays_empty_regardless_of_registered_tools() {
        // The default body always returns `Vec::new()` -- it does not
        // consult the implementor's storage at all. This pins that exact
        // behavior (rather than "empty because nothing was registered")
        // against an implementor whose `get` proves the tool really is
        // stored, so a future accidental wiring of `list()` to storage
        // would be a visible, intentional change rather than a silent one.
        let registry = MinimalRegistry::new();
        registry.register(test_armament("calculator")).await;

        assert!(registry.get("calculator").await.is_some());
        assert!(registry.list().await.is_empty());
    }
}
