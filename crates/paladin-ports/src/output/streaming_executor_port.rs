//! Streaming Executor Port
//!
//! Defines the abstraction for executing a Paladin agent with **streamed** output.
//! It is the streaming counterpart to
//! [`PaladinExecutorPort`](crate::output::paladin_executor_port::PaladinExecutorPort)
//! (which is buffered) and is kept as a separate, focused trait so that buffered-only
//! callers and registries are unaffected — an executor may implement one or both.
//!
//! Implementors produce a [`PaladinStream`] (an `mpsc` receiver of
//! [`PaladinStreamChunk`](crate::output::paladin_port::PaladinStreamChunk)s), forwarding
//! incremental output as it is generated and a final chunk when execution completes.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐     ┌──────────────────────────────┐
//! │  SSE HTTP handler   │────▶│   StreamingExecutorPort      │
//! │ (paladin-web)       │     │   (trait / abstraction)      │
//! └─────────────────────┘     └──────────────┬───────────────┘
//!                                            │ implements
//!                                            ▼
//!                              ┌──────────────────────────────┐
//!                              │  PaladinExecutionService      │
//!                              │  (drives LlmPort::            │
//!                              │   generate_stream)            │
//!                              └──────────────────────────────┘
//! ```

use async_trait::async_trait;

use crate::output::paladin_port::PaladinStream;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run_scope::RunScope;

/// Port trait for executing a Paladin agent with streamed output.
///
/// Separate from [`PaladinExecutorPort`](crate::output::paladin_executor_port::PaladinExecutorPort)
/// so the buffered execution path and any registry that stores buffered executors stay
/// unchanged; streaming is an *optional* capability layered alongside it.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to be shared across async tasks.
///
/// # Examples
///
/// ```rust,no_run
/// use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
/// use paladin_core::platform::container::paladin::Paladin;
/// use paladin_core::platform::container::paladin_error::PaladinError;
///
/// async fn first_chunk(
///     executor: &dyn StreamingExecutorPort,
///     agent: &Paladin,
///     input: &str,
/// ) -> Result<Option<String>, PaladinError> {
///     let mut stream = executor.execute_stream(agent, input).await?;
///     match stream.recv().await {
///         Some(Ok(chunk)) => Ok(Some(chunk.text)),
///         Some(Err(e)) => Err(e),
///         None => Ok(None),
///     }
/// }
/// ```
#[async_trait]
pub trait StreamingExecutorPort: Send + Sync {
    /// Execute a Paladin with the given input, streaming output chunks.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin agent to execute
    /// * `input` - The input/task to process
    ///
    /// # Returns
    ///
    /// * `Ok(PaladinStream)` - A receiver yielding `Ok(PaladinStreamChunk)` per chunk
    ///   (the last with `is_final = true`), or `Err(PaladinError)` if a chunk fails.
    /// * `Err(PaladinError)` - If the stream could not be started (e.g. the provider
    ///   does not support streaming).
    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError>;

    /// Execute a Paladin with streamed output under a caller-resolved [`RunScope`]
    /// (Phase 40 D-16).
    ///
    /// The default body **ignores `scope`** and delegates to [`Self::execute_stream`]: a
    /// correct claim of no scoped capability (X-10.4 -- every existing implementor compiles
    /// and behaves unchanged; the `PaladinPort::execute_scoped` precedent). Overriding is
    /// how an implementor acts on the scope: `PaladinExecutionService` overrides it so the
    /// streamed call's priced terminal chunk settles under `scope.ledger_scope` -- the
    /// calling principal's tenant and API key id -- instead of the unattributed sentinel.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin agent to execute
    /// * `input` - The input/task to process
    /// * `scope` - The run scope this execution runs under; ignored by the default body
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use paladin_core::platform::container::paladin_error::PaladinError;
    /// use paladin_core::platform::container::run_scope::RunScope;
    /// use paladin_core::platform::container::treasury_ledger::LedgerScope;
    /// use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
    ///
    /// async fn first_chunk_for_tenant(
    ///     executor: &dyn StreamingExecutorPort,
    ///     agent: &Paladin,
    ///     input: &str,
    /// ) -> Result<Option<String>, PaladinError> {
    ///     let scope = RunScope::default().with_ledger_scope(LedgerScope::new("acme", "svc-a"));
    ///     let mut stream = executor.execute_stream_scoped(agent, input, &scope).await?;
    ///     match stream.recv().await {
    ///         Some(Ok(chunk)) => Ok(Some(chunk.text)),
    ///         Some(Err(e)) => Err(e),
    ///         None => Ok(None),
    ///     }
    /// }
    /// ```
    async fn execute_stream_scoped(
        &self,
        paladin: &Paladin,
        input: &str,
        scope: &RunScope,
    ) -> Result<PaladinStream, PaladinError> {
        let _ = scope;
        self.execute_stream(paladin, input).await
    }
}
