//! Paladin Executor Port
//!
//! Defines the abstraction for executing a Paladin agent. This port breaks the
//! circular dependency between `HandoffService` and `PaladinExecutionService`:
//!
//! - `HandoffService` depends on `Arc<dyn PaladinExecutorPort>` to execute specialists
//! - `PaladinExecutionService` implements `PaladinExecutorPort`
//!
//! This follows the Dependency Inversion Principle: both high-level (HandoffService)
//! and low-level (PaladinExecutionService) modules depend on the abstraction.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐     ┌──────────────────────────┐
//! │   HandoffService    │────▶│  PaladinExecutorPort     │
//! │ (uses trait to      │     │  (trait / abstraction)   │
//! │  execute specialist)│     └──────────┬───────────────┘
//! └─────────────────────┘                │ implements
//!                                        ▼
//!                              ┌──────────────────────────┐
//!                              │ PaladinExecutionService   │
//!                              │ (concrete implementation) │
//!                              └──────────────────────────┘
//! ```

use async_trait::async_trait;

use crate::output::paladin_port::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::run_scope::RunScope;

/// Port trait for executing a Paladin agent
///
/// This abstraction allows services like `HandoffService` to delegate execution
/// to a Paladin without directly depending on `PaladinExecutionService`, thus
/// avoiding circular dependencies in the dependency graph.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow sharing across async tasks.
///
/// # Examples
///
/// ```rust,no_run
/// use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
/// use paladin_ports::output::paladin_port::PaladinResult;
/// use paladin_core::platform::container::paladin::Paladin;
/// use std::sync::Arc;
///
/// async fn delegate_to_specialist(
///     executor: &dyn PaladinExecutorPort,
///     specialist: &Paladin,
///     task: &str,
/// ) -> Result<PaladinResult, paladin_core::platform::container::paladin_error::PaladinError> {
///     executor.execute(specialist, task).await
/// }
/// ```
#[async_trait]
pub trait PaladinExecutorPort: Send + Sync {
    /// Execute a Paladin with the given input
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin agent to execute
    /// * `input` - The input/task to process
    ///
    /// # Returns
    ///
    /// * `Ok(PaladinResult)` - The execution result including output, tokens, etc.
    /// * `Err(PaladinError)` - If execution fails
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError>;

    /// Execute a Paladin under a caller-resolved [`RunScope`] (Phase 40 D-16).
    ///
    /// The default body **ignores `scope`** and delegates to [`Self::execute`]: a
    /// correct claim of no scoped capability, the same shape as `PaladinPort::execute_scoped`
    /// and `RunRepositoryPort::insert_with_latest` (X-10.4 -- a defaulted method added to a
    /// published trait keeps every existing implementor compiling and behaving unchanged).
    /// Overriding is how an implementor acts on the scope: `PaladinExecutionService`
    /// overrides it so priced calls settle under `scope.ledger_scope` -- the calling
    /// principal's tenant and API key id, set by the HTTP agent handlers -- instead of the
    /// unattributed sentinel.
    ///
    /// # Arguments
    ///
    /// * `paladin` - The Paladin agent to execute
    /// * `input` - The input/task to process
    /// * `scope` - The run scope (Vault grant, Platform run id, ledger scope) this
    ///   execution runs under; ignored by the default body
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use paladin_core::platform::container::paladin::Paladin;
    /// use paladin_core::platform::container::paladin_error::PaladinError;
    /// use paladin_core::platform::container::run_scope::RunScope;
    /// use paladin_core::platform::container::treasury_ledger::LedgerScope;
    /// use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
    /// use paladin_ports::output::paladin_port::PaladinResult;
    ///
    /// async fn execute_for_tenant(
    ///     executor: &dyn PaladinExecutorPort,
    ///     agent: &Paladin,
    ///     input: &str,
    /// ) -> Result<PaladinResult, PaladinError> {
    ///     let scope = RunScope::default().with_ledger_scope(LedgerScope::new("acme", "svc-a"));
    ///     executor.execute_scoped(agent, input, &scope).await
    /// }
    /// ```
    async fn execute_scoped(
        &self,
        paladin: &Paladin,
        input: &str,
        scope: &RunScope,
    ) -> Result<PaladinResult, PaladinError> {
        let _ = scope;
        self.execute(paladin, input).await
    }
}
