//! The Treasurer's mid-run boundary guard (ALLOW-03, Phase 42 D-01..D-04, G11; ADR-0057).
//!
//! [`TreasurerSpendGuard`] implements the engine's
//! [`SpendGuard`](paladin_ports::output::spend_guard::SpendGuard) port from the Treasurer's one
//! shared ceiling evaluation (`Treasurer::evaluate`), the same function admission calls. At
//! every superstep boundary it reads the balance for each applicable ceiling and answers `Halt`
//! when one is exhausted -- before the next superstep starts.
//!
//! - **Check only, every boundary** (D-01, D-02): the guard reads and never writes, and never
//!   caches a `Continue`: other runs sharing the scope are exactly the case that matters.
//! - **Fail closed** (D-03): a failed read answers `Halt(LedgerUnavailable)`, never a swallowed
//!   `Continue`.
//! - **A halt is sticky** (G11): the guard memoises its first `Halt` in an
//!   `Arc<OnceLock<HaltReason>>` shared with every clone, so a child battalion's halt cannot be
//!   passed by a later parent boundary that re-reads after a window roll.
//! - **Identity only**: the guard holds a [`RunAttribution`] (tenant and key NAME), never a role
//!   and never a key value, so no role bypasses it and no log line can leak a credential.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;

use paladin_core::platform::container::allowance::HaltReason;
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::format_cost;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};

use super::Treasurer;

/// A per-run [`SpendGuard`] answering from the Treasurer's shared ceiling evaluation.
///
/// Build one per run with [`Treasurer::spend_guard`]; clones (a child battalion run inherits the
/// guard) share the memoised first halt.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use paladin::application::services::treasurer::{AllowancePolicy, Treasurer};
/// use paladin_core::platform::container::cost::CurrencyCode;
/// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
/// use paladin_core::platform::container::run::RunId;
/// use paladin_core::platform::container::waypoint::ThreadId;
/// use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};
/// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = AllowancePolicy::new(CurrencyCode::new("USD")?, 80);
/// let treasurer = Arc::new(Treasurer::new(policy, Arc::new(InMemoryTreasuryLedger::new())));
/// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
/// let guard = treasurer.spend_guard(subject, RunId::new_v7());
/// // No allowance configured for the principal: the guard continues without a ledger read.
/// let thread = ThreadId::new("11111111-1111-7111-8111-111111111111")?;
/// assert_eq!(guard.check(&thread).await, SpendDecision::Continue);
/// # Ok(())
/// # }
/// ```
pub struct TreasurerSpendGuard {
    treasurer: Arc<Treasurer>,
    subject: RunAttribution,
    run_id: RunId,
    /// The first halt this run's guard answered (G11). Shared by every clone.
    halted: Arc<OnceLock<HaltReason>>,
}

impl std::fmt::Debug for TreasurerSpendGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Tenant id and run id only: never a key value, never a balance.
        f.debug_struct("TreasurerSpendGuard")
            .field("tenant", &self.subject.tenant_id)
            .field("run_id", &self.run_id)
            .finish_non_exhaustive()
    }
}

impl TreasurerSpendGuard {
    /// Remember `reason` as this run's first halt and return the stored value (the first writer
    /// wins, so every later call answers the same reason).
    fn memoise(&self, reason: HaltReason) -> HaltReason {
        self.halted.get_or_init(|| reason).clone()
    }
}

#[async_trait]
impl SpendGuard for TreasurerSpendGuard {
    async fn check(&self, _thread: &ThreadId) -> SpendDecision {
        // G11: a halt already answered is final -- no ledger read, same reason.
        if let Some(reason) = self.halted.get() {
            return SpendDecision::Halt(reason.clone());
        }

        match self.treasurer.evaluate(&self.subject).await {
            // No ceiling applies to this principal: nothing to guard (D-03, D-10).
            Ok(None) => SpendDecision::Continue,
            Ok(Some(evaluation)) => match evaluation.exhausted {
                Some(refusal) => {
                    log::warn!(
                        "run halted at a superstep boundary: run={} scope={} limit={} tenant={} \
                         balance={} ceiling={}",
                        self.run_id,
                        refusal.scope_kind.as_str(),
                        refusal.limit_kind.as_str(),
                        self.subject.tenant_id,
                        format_cost(&refusal.balance),
                        format_cost(&refusal.ceiling),
                    );
                    SpendDecision::Halt(self.memoise(HaltReason::AllowanceExhausted(refusal)))
                }
                // Headroom everywhere: continue, and cache nothing (D-02).
                None => SpendDecision::Continue,
            },
            // D-03: fail closed. The error text names no key value (the ledger's own
            // `wrap` already redacts connection URLs).
            Err(error) => {
                let scopes = self.treasurer.policy.ceilings_for(&self.subject);
                log::error!(
                    "{}",
                    fail_closed_message(
                        &self.run_id,
                        scopes.iter().map(|ceiling| ceiling.scope_kind.as_str()),
                        &self.subject.tenant_id,
                        &error,
                    )
                );
                SpendDecision::Halt(self.memoise(HaltReason::LedgerUnavailable))
            }
        }
    }
}

/// The one `error`-level line a fail-closed boundary check writes (D-03, T-42-18).
///
/// Names the run, the scope kinds of the ceilings that could not be evaluated (a scope kind is a
/// label such as `api_key`, never a key value), the tenant id and the backend error. It takes no
/// key value, so none can reach the log.
pub(super) fn fail_closed_message<'a>(
    run_id: &RunId,
    scope_kinds: impl IntoIterator<Item = &'a str>,
    tenant_id: &impl std::fmt::Display,
    error: &impl std::fmt::Display,
) -> String {
    let mut kinds: Vec<&str> = Vec::new();
    for kind in scope_kinds {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    format!(
        "allowance boundary check failed closed: run={run_id} scope={} tenant={tenant_id} \
         error={error}",
        kinds.join(","),
    )
}

impl Treasurer {
    /// Build the per-run [`SpendGuard`] for `subject` running as `run_id` (ALLOW-03, D-04).
    ///
    /// One guard per run: the worker attaches it with `WarEngine::with_spend_guard`, a child
    /// battalion run inherits the same instance, and every clone shares the memoised first
    /// halt (G11). The guard takes identity only (a [`RunAttribution`]), never a role.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use paladin::application::services::treasurer::{AllowancePolicy, Treasurer};
    /// use paladin_core::platform::container::cost::CurrencyCode;
    /// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    /// use paladin_core::platform::container::run::RunId;
    /// use paladin_ports::output::spend_guard::SpendGuard;
    /// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let policy = AllowancePolicy::new(CurrencyCode::new("USD")?, 80);
    /// let treasurer = Arc::new(Treasurer::new(policy, Arc::new(InMemoryTreasuryLedger::new())));
    /// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
    /// let _guard: Arc<dyn SpendGuard> = treasurer.spend_guard(subject, RunId::new_v7());
    /// # Ok(())
    /// # }
    /// ```
    pub fn spend_guard(
        self: &Arc<Self>,
        subject: RunAttribution,
        run_id: RunId,
    ) -> Arc<dyn SpendGuard> {
        Arc::new(TreasurerSpendGuard {
            treasurer: Arc::clone(self),
            subject,
            run_id,
            halted: Arc::new(OnceLock::new()),
        })
    }
}
