//! Battalion Port - Multi-Agent Orchestration Abstraction
//!
//! This module defines the output port (interface) for Battalion execution following
//! Hexagonal Architecture principles. The `BattalionPort` trait provides a clean abstraction
//! that allows the application layer to orchestrate multiple Paladin agents without being
//! coupled to specific orchestration strategies or execution backends.
//!
//! # Purpose
//!
//! The Battalion port enables coordinated execution of multiple Paladin agents using various
//! orchestration patterns while maintaining a clean separation between the domain logic
//! (what a Battalion is) and the execution logic (how it orchestrates). This allows you to:
//!
//! - Orchestrate multiple Paladins with different patterns (Formation, Phalanx, Campaign, Chain of Command)
//! - Execute Paladins sequentially, in parallel, or in complex graph workflows
//! - Monitor long-running Battalion operations asynchronously
//! - Cancel in-flight executions gracefully
//! - Track execution metadata (timing, status, results from each Paladin)
//! - Implement retry logic, checkpointing, or recovery consistently
//! - Test orchestration logic without real agent execution
//!
//! # Hexagonal Architecture (Ports & Adapters)
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    Application Layer                          │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  Commander (Strategic Router)                           │  │
//! │  │  - Analyzes tasks                                       │  │
//! │  │  - Selects optimal Battalion pattern                    │  │
//! │  │  - Routes to appropriate orchestration service          │  │
//! │  └─────────────────────┬────────────────────────────────────┘  │
//! │                        │                                       │
//! │                        ↓                                       │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │  BattalionPort (trait)                                  │  │
//! │  │  - execute()                                            │  │
//! │  │  - status()                                             │  │
//! │  │  - cancel()                                             │  │
//! │  └────────────────────┬───────────────────────────────────┘  │
//! └─────────────────────────┼────────────────────────────────────┘
//!                          │
//!                          ↓
//!   ┌──────────────────────────────────────────────────────────┐
//!   │  Battalion Execution Services (adapters)                  │
//!   │  - FormationService (sequential)                          │
//!   │  - PhalanxService (parallel)                              │
//!   │  - CampaignService (graph/DAG)                            │
//!   │  - ChainOfCommandService (hierarchical)                   │
//!   │  - Each uses PaladinPort to execute individual agents     │
//!   └──────────────────────────────────────────────────────────┘
//! ```
//!
//! # Battalion Orchestration Patterns
//!
//! ## Formation (Sequential Execution)
//!
//! Execute Paladins one after another, passing output as input to the next:
//!
//! ```text
//! Paladin A → Result A → Paladin B → Result B → Paladin C → Final Result
//! ```
//!
//! **Use Cases**: Multi-stage pipelines, data transformation chains, sequential analysis
//!
//! ## Phalanx (Parallel Execution)
//!
//! Execute Paladins concurrently and aggregate results:
//!
//! ```text
//! Paladin A ─┐
//! Paladin B ─┼→ Aggregate → Final Result
//! Paladin C ─┘
//! ```
//!
//! **Use Cases**: Parallel analysis, consensus building, concurrent data processing
//!
//! ## Campaign (Graph/DAG Workflow)
//!
//! Execute Paladins in a directed acyclic graph with conditional branches:
//!
//! ```text
//!         ┌→ Paladin B ─┐
//! Start ─→│              ├→ Paladin D → End
//!         └→ Paladin C ─┘
//! ```
//!
//! **Use Cases**: Complex workflows, conditional logic, decision trees
//!
//! ## Chain of Command (Hierarchical Delegation)
//!
//! A commander Paladin delegates subtasks to specialist Paladins:
//!
//! ```text
//! Commander → [Subtask 1 → Specialist A]
//!          → [Subtask 2 → Specialist B]
//!          → [Subtask 3 → Specialist C]
//!          → Aggregate Results
//! ```
//!
//! **Use Cases**: Task decomposition, specialist routing, hierarchical processing
//!
//! # Common Use Cases
//!
//! ## 1. Execute Battalion with Monitoring
//!
//! ```ignore
//! use paladin_ports::output::battalion_port::{
//!     BattalionPort, BattalionStatus
//! };
//! use uuid::Uuid;
//! use std::sync::Arc;
//! use tokio::time::{sleep, Duration};
//!
//! async fn run_battalion_with_monitoring(
//!     battalion_port: Arc<dyn BattalionPort>,
//!     battalion_id: Uuid,
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Start execution in background
//!     let port_clone = battalion_port.clone();
//!     let execute_handle = tokio::spawn(async move {
//!         port_clone.execute(battalion_id).await
//!     });
//!
//!     // Monitor progress
//!     loop {
//!         let status = battalion_port.status(battalion_id).await?;
//!
//!         match status {
//!             BattalionStatus::Pending => println!("Waiting to start..."),
//!             BattalionStatus::Running => println!("Executing..."),
//!             BattalionStatus::Completed => {
//!                 println!("Battalion completed!");
//!                 break;
//!             }
//!             BattalionStatus::Failed(err) => {
//!                 eprintln!("Battalion failed: {}", err);
//!                 break;
//!             }
//!             BattalionStatus::Cancelled => {
//!                 println!("Battalion was cancelled");
//!                 break;
//!             }
//!         }
//!
//!         sleep(Duration::from_secs(2)).await;
//!     }
//!
//!     // Get final result
//!     let result = execute_handle.await??;
//!     println!("Final output: {}", result.final_output);
//!     println!("Total time: {}ms", result.total_execution_time_ms);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 2. Execute with Timeout and Cancellation
//!
//! ```ignore
//! use paladin_ports::output::battalion_port::BattalionPort;
//! use uuid::Uuid;
//! use std::sync::Arc;
//! use tokio::time::{timeout, Duration};
//!
//! async fn execute_with_timeout(
//!     battalion_port: Arc<dyn BattalionPort>,
//!     battalion_id: Uuid,
//!     timeout_seconds: u64,
//! ) -> Result<String, Box<dyn std::error::Error>> {
//!     let port_clone = battalion_port.clone();
//!
//!     match timeout(
//!         Duration::from_secs(timeout_seconds),
//!         port_clone.execute(battalion_id),
//!     )
//!     .await
//!     {
//!         Ok(Ok(result)) => {
//!             println!("Completed in time");
//!             Ok(result.final_output)
//!         }
//!         Ok(Err(e)) => {
//!             eprintln!("Execution failed: {}", e);
//!             Err(e.into())
//!         }
//!         Err(_) => {
//!             println!("Timeout reached, cancelling...");
//!             battalion_port.cancel(battalion_id).await?;
//!             Err("Battalion execution timed out".into())
//!         }
//!     }
//! }
//! ```
//!
//! ## 3. Retry Failed Battalions
//!
//! ```ignore
//! use paladin_ports::output::battalion_port::BattalionPort;
//! use paladin_core::platform::container::battalion::BattalionError;
//! use uuid::Uuid;
//! use std::sync::Arc;
//! use tokio::time::{sleep, Duration};
//!
//! async fn execute_with_retry(
//!     battalion_port: Arc<dyn BattalionPort>,
//!     battalion_id: Uuid,
//!     max_retries: u32,
//! ) -> Result<String, BattalionError> {
//!     let mut attempts = 0;
//!     let mut backoff = Duration::from_secs(1);
//!
//!     loop {
//!         match battalion_port.execute(battalion_id).await {
//!             Ok(result) => return Ok(result.final_output),
//!             Err(e) if attempts >= max_retries => return Err(e),
//!             Err(BattalionError::PaladinError(_)) => {
//!                 println!("Attempt {} failed, retrying...", attempts + 1);
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
//! ## 4. Multi-Battalion Orchestration
//!
//! ```ignore
//! use paladin_ports::output::battalion_port::BattalionPort;
//! use uuid::Uuid;
//! use std::sync::Arc;
//!
//! async fn execute_multiple_battalions(
//!     battalion_port: Arc<dyn BattalionPort>,
//!     battalion_ids: Vec<Uuid>,
//! ) -> Vec<Result<String, Box<dyn std::error::Error>>> {
//!     let handles: Vec<_> = battalion_ids
//!         .into_iter()
//!         .map(|id| {
//!             let port = battalion_port.clone();
//!             tokio::spawn(async move {
//!                 let result = port.execute(id).await?;
//!                 Ok::<String, Box<dyn std::error::Error>>(result.final_output)
//!             })
//!         })
//!         .collect();
//!
//!     // Wait for all to complete
//!     let mut results = Vec::new();
//!     for handle in handles {
//!         results.push(handle.await.unwrap());
//!     }
//!
//!     results
//! }
//! ```
//!
//! # Error Handling
//!
//! BattalionPort methods return `BattalionError` which includes:
//!
//! | Error | Retryable? | Recovery Strategy |
//! |-------|------------|-------------------|
//! | ValidationError | No | Fix Battalion configuration (invalid pattern, missing Paladins) |
//! | PaladinError | Maybe | Retry individual Paladin or entire Battalion |
//! | ExecutionError | Maybe | Check error message, retry if transient |
//! | NotFound | No | Ensure Battalion was registered before execution |
//! | AlreadyRunning | No | Wait for completion or cancel existing execution |
//! | Cancelled | No | This is success if cancellation was intentional |
//!
//! ## Status Enum
//!
//! The `BattalionStatus` enum indicates current execution state:
//!
//! - `Pending`: Battalion registered but not started
//! - `Running`: Currently executing Paladins
//! - `Completed`: All Paladins executed successfully
//! - `Failed(String)`: Execution failed with error message
//! - `Cancelled`: Execution was cancelled via `cancel()`
//!
//! # Thread Safety
//!
//! BattalionPort is `Send + Sync`, allowing safe use across async task boundaries.
//! This is critical for:
//! - Concurrent Battalion execution
//! - Status monitoring from separate tasks
//! - Timeout/cancellation from watchdog tasks
//!
//! # Implementation Notes
//!
//! ## Adapter Implementation Checklist
//!
//! When implementing a Battalion adapter:
//!
//! 1. **Pattern Support**: Implement at least one pattern (Formation, Phalanx, Campaign, Chain of Command)
//! 2. **State Management**: Track Battalion status (Pending → Running → Completed/Failed/Cancelled)
//! 3. **Paladin Execution**: Use PaladinPort to execute individual agents
//! 4. **Result Aggregation**: Combine outputs from multiple Paladins
//! 5. **Error Propagation**: Handle Paladin failures according to pattern (fail-fast vs continue)
//! 6. **Cancellation Support**: Implement graceful cancellation of running Paladins
//! 7. **Concurrency Management**: Handle parallel execution safely (Phalanx, Campaign)
//! 8. **Status Updates**: Provide accurate real-time status via `status()`
//! 9. **Timing Metadata**: Track total execution time and per-Paladin timing
//! 10. **Idempotency**: Handle duplicate `execute()` calls gracefully
//! 11. **Cleanup**: Release resources on completion, failure, or cancellation
//!
//! ## Performance Considerations
//!
//! - **Paladin Concurrency**: Execute independent Paladins in parallel (Phalanx, Campaign)
//! - **Resource Limits**: Limit concurrent Paladin count to avoid resource exhaustion
//! - **Checkpointing**: Save intermediate results for long-running Campaigns
//! - **Status Caching**: Cache status to reduce overhead of frequent polling
//! - **Async Execution**: Use non-blocking operations throughout
//! - **Error Fast-Fail**: For Formation, fail immediately on first error to save resources
//!
//! ## Testing Strategy
//!
//! ```ignore
//! use paladin_ports::output::battalion_port::{
//!     BattalionPort, BattalionStatus
//! };
//! use paladin_core::platform::container::battalion::{
//!     BattalionResult, BattalionError
//! };
//! use async_trait::async_trait;
//! use uuid::Uuid;
//!
//! /// Mock Battalion port for testing
//! struct MockBattalionPort {
//!     should_fail: bool,
//!     execution_time_ms: u64,
//! }
//!
//! #[async_trait]
//! impl BattalionPort for MockBattalionPort {
//!     async fn execute(&self, _battalion_id: Uuid) -> Result<BattalionResult, BattalionError> {
//!         if self.should_fail {
//!             return Err(BattalionError::PaladinError("Mock failure".into()));
//!         }
//!
//!         Ok(BattalionResult {
//!             final_output: "Mock result".into(),
//!             paladin_results: Vec::new(),
//!             total_execution_time_ms: self.execution_time_ms,
//!             pattern_used: "Formation".into(),
//!         })
//!     }
//!
//!     async fn status(&self, _battalion_id: Uuid) -> Result<BattalionStatus, BattalionError> {
//!         if self.should_fail {
//!             Ok(BattalionStatus::Failed("Mock error".into()))
//!         } else {
//!             Ok(BattalionStatus::Completed)
//!         }
//!     }
//!
//!     async fn cancel(&self, _battalion_id: Uuid) -> Result<(), BattalionError> {
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Common Pitfalls
//!
//! 1. **Not Checking Status**: Always check status after long operations to detect failures
//! 2. **Missing Cancellation**: Implement proper cancellation to avoid zombie executions
//! 3. **Resource Leaks**: Ensure cleanup happens on all exit paths (success, error, cancel)
//! 4. **Race Conditions**: Protect shared state in concurrent patterns (Phalanx, Campaign)
//! 5. **Error Swallowing**: Don't silently ignore Paladin errors, propagate appropriately
//! 6. **Blocking Operations**: All operations must be async to avoid blocking
//! 7. **Status Polling**: Don't poll status too frequently, use reasonable intervals (1-5s)
//! 8. **Result Ordering**: Maintain Paladin execution order in results for debugging
//!
//! # Related Modules
//!
//! - [`paladin_core::platform::container::battalion`] - Battalion domain entities
//! - `paladin_battalion::battalion` - Battalion execution services (adapters)
//! - [`paladin_ports::output::paladin_port`](crate::output::paladin_port) - Individual agent execution
//! - `paladin_battalion::commander` - Strategic pattern routing
//! - [`paladin_core::platform::container::citadel`] - Checkpointing and recovery

use async_trait::async_trait;
use uuid::Uuid;

use paladin_core::platform::container::battalion::{
    BattalionError, BattalionResult, BattalionStatus,
};

/// Port abstraction for Battalion execution
///
/// This trait defines the interface that any Battalion orchestration implementation
/// must satisfy. It follows the hexagonal architecture pattern, allowing the
/// core domain to remain independent of orchestration details.
///
/// # Capabilities
///
/// - **Orchestration Patterns**: Support for Formation, Phalanx, Campaign, Chain of Command
/// - **Asynchronous Execution**: Non-blocking execution of multi-agent workflows
/// - **Status Monitoring**: Query current execution state without blocking
/// - **Cancellation**: Gracefully cancel in-flight executions
/// - **Result Aggregation**: Combine outputs from multiple Paladins
/// - **Error Handling**: Propagate errors according to pattern (fail-fast vs continue)
///
/// # Requirements
///
/// Implementations must:
/// - Be `Send + Sync` for safe concurrent use across async tasks
/// - Integrate with PaladinPort for individual agent execution
/// - Track Battalion state (Pending → Running → Completed/Failed/Cancelled)
/// - Support at least one orchestration pattern
/// - Handle concurrent Paladin execution safely
/// - Implement graceful cancellation
/// - Aggregate results from multiple Paladins
/// - Provide accurate real-time status updates
///
/// # Examples
///
/// ## Basic Implementation Pattern
///
/// ```ignore
/// use async_trait::async_trait;
/// use paladin_ports::output::battalion_port::{
///     BattalionPort
/// };
/// use paladin_ports::output::paladin_port::PaladinPort;
/// use paladin_core::platform::container::battalion::{
///     BattalionResult, BattalionStatus, BattalionError
/// };
/// use uuid::Uuid;
/// use std::sync::Arc;
/// use std::collections::HashMap;
/// use tokio::sync::RwLock;
///
/// struct SimpleBattalionExecutor {
///     paladin_port: Arc<dyn PaladinPort>,
///     status_map: Arc<RwLock<HashMap<Uuid, BattalionStatus>>>,
/// }
///
/// #[async_trait]
/// impl BattalionPort for SimpleBattalionExecutor {
///     async fn execute(&self, battalion_id: Uuid) -> Result<BattalionResult, BattalionError> {
///         // 1. Update status to Running
///         {
///             let mut status = self.status_map.write().await;
///             status.insert(battalion_id, BattalionStatus::Running);
///         }
///
///         // 2. Execute Paladins (simplified Formation pattern)
///         let start_time = std::time::Instant::now();
///         let mut outputs = Vec::new();
///
///         // Execute each Paladin sequentially
///         // In real implementation, load Paladins from Battalion configuration
///         // for paladin in paladins {
///         //     let result = self.paladin_port.execute(&paladin, input).await?;
///         //     outputs.push(result);
///         //     input = result.output; // Pass to next
///         // }
///
///         // 3. Update status to Completed
///         {
///             let mut status = self.status_map.write().await;
///             status.insert(battalion_id, BattalionStatus::Completed);
///         }
///
///         // 4. Return result
///         Ok(BattalionResult {
///             final_output: "Combined output".into(),
///             paladin_results: outputs,
///             total_execution_time_ms: start_time.elapsed().as_millis() as u64,
///             pattern_used: "Formation".into(),
///         })
///     }
///
///     async fn status(&self, battalion_id: Uuid) -> Result<BattalionStatus, BattalionError> {
///         let status = self.status_map.read().await;
///         status
///             .get(&battalion_id)
///             .cloned()
///             .ok_or_else(|| BattalionError::NotFound(format!("Battalion {} not found", battalion_id)))
///     }
///
///     async fn cancel(&self, battalion_id: Uuid) -> Result<(), BattalionError> {
///         let mut status = self.status_map.write().await;
///         if let Some(current_status) = status.get(&battalion_id) {
///             match current_status {
///                 BattalionStatus::Running => {
///                     // Signal cancellation (in real impl, stop Paladins)
///                     status.insert(battalion_id, BattalionStatus::Cancelled);
///                     Ok(())
///                 }
///                 BattalionStatus::Completed | BattalionStatus::Failed(_) => {
///                     Err(BattalionError::ValidationError(
///                         "Cannot cancel completed Battalion".into()
///                     ))
///                 }
///                 _ => Ok(()),
///             }
///         } else {
///             Err(BattalionError::NotFound(format!("Battalion {} not found", battalion_id)))
///         }
///     }
/// }
/// ```
///
/// # Implementation Notes
///
/// ## Orchestration Patterns
///
/// ### Formation (Sequential)
///
/// ```ignore
/// async fn execute_formation(
///     &self,
///     paladins: Vec<Paladin>,
///     initial_input: &str,
/// ) -> Result<String, BattalionError> {
///     let mut current_input = initial_input.to_string();
///
///     for paladin in paladins {
///         let result = self.paladin_port.execute(&paladin, &current_input).await?;
///         current_input = result.output; // Chain output to next
///     }
///
///     Ok(current_input)
/// }
/// ```
///
/// ### Phalanx (Parallel)
///
/// ```ignore
/// async fn execute_phalanx(
///     &self,
///     paladins: Vec<Paladin>,
///     input: &str,
/// ) -> Result<Vec<String>, BattalionError> {
///     let handles: Vec<_> = paladins
///         .into_iter()
///         .map(|paladin| {
///             let port = self.paladin_port.clone();
///             let input = input.to_string();
///             tokio::spawn(async move {
///                 port.execute(&paladin, &input).await
///             })
///         })
///         .collect();
///
///     let mut results = Vec::new();
///     for handle in handles {
///         let result = handle.await??;
///         results.push(result.output);
///     }
///
///     Ok(results)
/// }
/// ```
///
/// ### Campaign (Graph/DAG)
///
/// ```ignore
/// async fn execute_campaign(
///     &self,
///     graph: &BattalionGraph,
///     initial_input: &str,
/// ) -> Result<String, BattalionError> {
///     let mut completed = HashMap::new();
///     let mut queue = vec![graph.start_node()];
///
///     while let Some(node) = queue.pop() {
///         // Check if dependencies completed
///         if !node.dependencies().iter().all(|d| completed.contains_key(d)) {
///             continue;
///         }
///
///         // Execute Paladin for this node
///         let input = build_input_from_dependencies(&completed, node.dependencies());
///         let result = self.paladin_port.execute(&node.paladin, &input).await?;
///         completed.insert(node.id, result.output);
///
///         // Add next nodes to queue
///         queue.extend(graph.next_nodes(node.id));
///     }
///
///     Ok(completed.get(&graph.end_node()).unwrap().clone())
/// }
/// ```
///
/// ## Cancellation Implementation
///
/// ```ignore
/// async fn cancel(&self, battalion_id: Uuid) -> Result<(), BattalionError> {
///     // 1. Set cancellation flag
///     self.cancellation_tokens.write().await.insert(battalion_id, true);
///
///     // 2. Stop running Paladins
///     if let Some(running_paladins) = self.active_paladins.read().await.get(&battalion_id) {
///         for paladin_id in running_paladins {
///             // Signal Paladin to stop (implementation-specific)
///             self.stop_paladin(*paladin_id).await?;
///         }
///     }
///
///     // 3. Update status
///     self.status_map.write().await.insert(battalion_id, BattalionStatus::Cancelled);
///
///     Ok(())
/// }
/// ```
///
/// # Performance Tips
///
/// 1. **Parallel Execution**: Use tokio::spawn for Phalanx and independent Campaign nodes
/// 2. **Resource Limits**: Limit concurrent Paladins with semaphore (e.g., `Semaphore::new(10)`)
/// 3. **Status Caching**: Cache status for short period to reduce lock contention
/// 4. **Batch Operations**: Group multiple status checks into single query
/// 5. **Early Cancellation Check**: Check cancellation flag between Paladin executions
/// 6. **Result Streaming**: Stream partial results as they complete (advanced)
#[async_trait]
pub trait BattalionPort: Send + Sync {
    /// Execute a Battalion and return the final result
    ///
    /// This method orchestrates the execution of all Paladins in the Battalion
    /// according to the configured pattern (Formation, Phalanx, Campaign, or
    /// Chain of Command).
    ///
    /// # Arguments
    ///
    /// * `battalion_id` - Unique identifier for this Battalion execution
    ///
    /// # Returns
    ///
    /// Returns a `BattalionResult` containing:
    /// - `final_output`: The aggregated final output from all Paladins
    /// - `paladin_results`: Individual results from each Paladin
    /// - `total_execution_time_ms`: Total execution time in milliseconds
    /// - `pattern_used`: The orchestration pattern that was executed
    ///
    /// # Errors
    ///
    /// - `BattalionError::ValidationError` - Invalid Battalion configuration
    /// - `BattalionError::PaladinError` - One or more Paladins failed
    /// - `BattalionError::ExecutionError` - Orchestration logic error
    /// - `BattalionError::NotFound` - Battalion ID not registered
    /// - `BattalionError::AlreadyRunning` - Battalion is already executing
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::battalion_port::BattalionPort;
    /// use uuid::Uuid;
    /// use std::sync::Arc;
    ///
    /// async fn run_battalion(
    ///     port: Arc<dyn BattalionPort>,
    ///     battalion_id: Uuid,
    /// ) {
    ///     match port.execute(battalion_id).await {
    ///         Ok(result) => {
    ///             println!("Battalion completed!");
    ///             println!("Final output: {}", result.final_output);
    ///             println!("Total time: {}ms", result.total_execution_time_ms);
    ///             println!("Pattern: {}", result.pattern_used);
    ///             println!("Paladin count: {}", result.paladin_results.len());
    ///         }
    ///         Err(e) => {
    ///             eprintln!("Battalion failed: {}", e);
    ///         }
    ///     }
    /// }
    /// ```
    async fn execute(&self, battalion_id: Uuid) -> Result<BattalionResult, BattalionError>;

    /// Get the current status of a Battalion execution
    ///
    /// This method returns the current execution status without blocking.
    /// Useful for monitoring long-running Battalion operations from a separate task.
    ///
    /// # Arguments
    ///
    /// * `battalion_id` - Unique identifier for the Battalion execution
    ///
    /// # Returns
    ///
    /// Returns the current `BattalionStatus`:
    /// - `Pending`: Battalion registered but not started
    /// - `Running`: Currently executing Paladins
    /// - `Completed`: All Paladins executed successfully
    /// - `Failed(String)`: Execution failed with error message
    /// - `Cancelled`: Execution was cancelled
    ///
    /// # Errors
    ///
    /// - `BattalionError::NotFound` - Battalion ID not found
    /// - `BattalionError::ExecutionError` - Error retrieving status
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::battalion_port::{
    ///     BattalionPort, BattalionStatus
    /// };
    /// use uuid::Uuid;
    /// use std::sync::Arc;
    /// use tokio::time::{sleep, Duration};
    ///
    /// async fn monitor_battalion(
    ///     port: Arc<dyn BattalionPort>,
    ///     battalion_id: Uuid,
    /// ) {
    ///     loop {
    ///         match port.status(battalion_id).await {
    ///             Ok(BattalionStatus::Running) => {
    ///                 println!("Still executing...");
    ///                 sleep(Duration::from_secs(2)).await;
    ///             }
    ///             Ok(BattalionStatus::Completed) => {
    ///                 println!("Execution complete!");
    ///                 break;
    ///             }
    ///             Ok(BattalionStatus::Failed(err)) => {
    ///                 eprintln!("Execution failed: {}", err);
    ///                 break;
    ///             }
    ///             Ok(status) => {
    ///                 println!("Status: {:?}", status);
    ///                 break;
    ///             }
    ///             Err(e) => {
    ///                 eprintln!("Error checking status: {}", e);
    ///                 break;
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    async fn status(&self, battalion_id: Uuid) -> Result<BattalionStatus, BattalionError>;

    /// Cancel a running Battalion execution
    ///
    /// This method attempts to gracefully cancel a Battalion that is currently
    /// executing. Running Paladins will be signaled to stop, and the Battalion
    /// status will be set to `Cancelled`.
    ///
    /// # Arguments
    ///
    /// * `battalion_id` - Unique identifier for the Battalion execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Cancellation was successful or Battalion already stopped
    /// * `Err(BattalionError)` - Error during cancellation
    ///
    /// # Errors
    ///
    /// - `BattalionError::NotFound` - Battalion ID not found
    /// - `BattalionError::ValidationError` - Battalion already completed (cannot cancel)
    ///
    /// # Behavior
    ///
    /// - If Battalion is `Running`: Signal cancellation and update status to `Cancelled`
    /// - If Battalion is `Pending`: Cancel before execution starts
    /// - If Battalion is `Completed` or `Failed`: Return error (already finished)
    /// - If Battalion is `Cancelled`: Return Ok (idempotent)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use paladin_ports::output::battalion_port::BattalionPort;
    /// use uuid::Uuid;
    /// use std::sync::Arc;
    /// use tokio::time::{timeout, Duration};
    ///
    /// async fn execute_with_timeout(
    ///     port: Arc<dyn BattalionPort>,
    ///     battalion_id: Uuid,
    /// ) {
    ///     let port_clone = port.clone();
    ///
    ///     match timeout(
    ///         Duration::from_secs(60),
    ///         port_clone.execute(battalion_id),
    ///     )
    ///     .await
    ///     {
    ///         Ok(Ok(result)) => {
    ///             println!("Completed: {}", result.final_output);
    ///         }
    ///         Ok(Err(e)) => {
    ///             eprintln!("Failed: {}", e);
    ///         }
    ///         Err(_) => {
    ///             println!("Timeout! Cancelling...");
    ///             if let Err(e) = port.cancel(battalion_id).await {
    ///                 eprintln!("Cancel failed: {}", e);
    ///             } else {
    ///                 println!("Successfully cancelled");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    async fn cancel(&self, battalion_id: Uuid) -> Result<(), BattalionError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::battalion::BattalionStatus;

    // Mock implementation for testing
    struct MockBattalionPort {
        should_fail: bool,
    }

    #[async_trait]
    impl BattalionPort for MockBattalionPort {
        async fn execute(&self, _battalion_id: Uuid) -> Result<BattalionResult, BattalionError> {
            if self.should_fail {
                Err(BattalionError::PaladinError("Mock failure".to_string()))
            } else {
                // This will fail until BattalionResult::new is available
                unimplemented!("BattalionResult construction not yet implemented")
            }
        }

        async fn status(&self, _battalion_id: Uuid) -> Result<BattalionStatus, BattalionError> {
            if self.should_fail {
                Err(BattalionError::ValidationError(
                    "Battalion not found".to_string(),
                ))
            } else {
                Ok(BattalionStatus::Running)
            }
        }

        async fn cancel(&self, _battalion_id: Uuid) -> Result<(), BattalionError> {
            if self.should_fail {
                Err(BattalionError::ValidationError(
                    "Battalion not found".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_battalion_port_status_success() {
        let port = MockBattalionPort { should_fail: false };
        let battalion_id = Uuid::new_v4();

        let result = port.status(battalion_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BattalionStatus::Running);
    }

    #[tokio::test]
    async fn test_battalion_port_status_failure() {
        let port = MockBattalionPort { should_fail: true };
        let battalion_id = Uuid::new_v4();

        let result = port.status(battalion_id).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::ValidationError(msg) => {
                assert_eq!(msg, "Battalion not found");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_battalion_port_cancel_success() {
        let port = MockBattalionPort { should_fail: false };
        let battalion_id = Uuid::new_v4();

        let result = port.cancel(battalion_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_battalion_port_cancel_failure() {
        let port = MockBattalionPort { should_fail: true };
        let battalion_id = Uuid::new_v4();

        let result = port.cancel(battalion_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_battalion_port_execute_failure() {
        let port = MockBattalionPort { should_fail: true };
        let battalion_id = Uuid::new_v4();

        let result = port.execute(battalion_id).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            BattalionError::PaladinError(msg) => {
                assert_eq!(msg, "Mock failure");
            }
            _ => panic!("Expected PaladinError"),
        }
    }

    #[test]
    fn test_battalion_port_is_send_sync() {
        // Verify trait is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn BattalionPort>>();
    }
}
