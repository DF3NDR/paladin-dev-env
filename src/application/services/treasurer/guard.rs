//! The Treasurer's mid-run boundary guard (ALLOW-03, Phase 42 D-01..D-04, G11; ADR-0057).
//!
//! [`TreasurerSpendGuard`] implements the engine's
//! [`SpendGuard`](paladin_ports::output::spend_guard::SpendGuard) port from the Treasurer's one
//! shared ceiling evaluation (`Treasurer::evaluate`), the same function admission calls. At
//! every superstep boundary it reads the balance for each applicable ceiling and answers `Halt`
//! when one is exhausted -- before the next superstep starts.
//!
//! - **Check only, every boundary** (D-01, D-02): the guard reads the LEDGER and never writes
//!   it, and never caches a `Continue`: other runs sharing the scope are exactly the case that
//!   matters.
//! - **Notices observe, never gate** (D-17, D-18, Pitfall 9): the same boundary read also
//!   claims, through the Phase 41 once-per-window notice store, the warning for a `warn_at`
//!   crossing the run reached mid-run (emitted on the run's own trace emitter and queued for
//!   the operator) and the halt notice for a spend halt (queued for the operator). A claim never
//!   changes the decision, a `ledger_unavailable` halt claims nothing, and an in-run memo skips a
//!   repeat write for a ceiling and window already tried -- the store stays the dedup truth.
//! - **Fail closed** (D-03): a failed read answers `Halt(LedgerUnavailable)`, never a swallowed
//!   `Continue`.
//! - **A halt is sticky** (G11): the guard memoises its first `Halt` in an
//!   `Arc<OnceLock<HaltReason>>` shared with every clone, so a child battalion's halt cannot be
//!   passed by a later parent boundary that re-reads after a window roll.
//! - **Identity only**: the guard holds a [`RunAttribution`] (tenant and key NAME), never a role
//!   and never a key value, so no role bypasses it and no log line can leak a credential.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::allowance::{HaltReason, NoticeKind};
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::trace::TraceEvent;
use paladin_core::platform::container::treasury_ledger::format_cost;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_ports::output::spend_guard::{SpendDecision, SpendGuard};
use paladin_ports::output::trace_sink_port::TraceEmitter;

use super::{Ceiling, Treasurer, collect_crossings};

/// What one notice claim is keyed on in the in-run memo: the ceiling's identity, the window it
/// was read over, the ceiling's size and the notice kind. It mirrors the store's own dedup
/// identity, so the memo skips exactly the writes the store would answer `AlreadyRecorded` to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClaimKey {
    scope_kind: &'static str,
    tenant_id: String,
    api_key_id: Option<String>,
    limit_kind: &'static str,
    window_start: Option<DateTime<Utc>>,
    ceiling_nanos: i64,
    kind: &'static str,
}

impl ClaimKey {
    fn new(ceiling: &Ceiling, window_start: Option<DateTime<Utc>>, kind: NoticeKind) -> Self {
        Self {
            scope_kind: ceiling.scope_kind.as_str(),
            tenant_id: ceiling.tenant_id.clone(),
            api_key_id: ceiling.api_key_id.clone(),
            limit_kind: ceiling.limit_kind.as_str(),
            window_start,
            ceiling_nanos: ceiling.ceiling_nanos,
            kind: kind.as_str(),
        }
    }
}

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
    /// The run's own trace emitter: a mid-run warning this guard wins is emitted here, so it
    /// lands on that run's stream. `None` leaves only the trace leg off (D-17).
    emitter: Option<Arc<dyn TraceEmitter>>,
    /// Every notice claim this guard has already tried (Pitfall 9, T-42-39): a repeat boundary
    /// does not write again. Only skips writes -- the store stays the truth, and a claim never
    /// changes the guard's decision. The lock is held only to test-and-insert, never across an
    /// `.await`. Held behind an `Arc` so the struct keeps the `Freeze` auto trait it published
    /// with (an inline `Mutex` would change the public API surface).
    claimed: Arc<Mutex<HashSet<ClaimKey>>>,
}

impl std::fmt::Debug for TreasurerSpendGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Tenant id and run id only: never a key value, never a balance.
        f.debug_struct("TreasurerSpendGuard")
            .field("tenant", &self.subject.tenant_id)
            .field("run_id", &self.run_id)
            .field("emitter", &self.emitter.is_some())
            .finish_non_exhaustive()
    }
}

impl TreasurerSpendGuard {
    /// Remember `reason` as this run's first halt and return the stored value (the first writer
    /// wins, so every later call answers the same reason).
    fn memoise(&self, reason: HaltReason) -> HaltReason {
        self.halted.get_or_init(|| reason).clone()
    }

    /// `true` the first time this guard sees `key`, `false` for every repeat. A poisoned lock
    /// is recovered: the memo only skips writes, so a stale set is harmless.
    fn first_attempt(&self, key: ClaimKey) -> bool {
        self.claimed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key)
    }

    /// Claim the once-per-window warning for every ceiling this boundary read found at or past
    /// its `warn_at`, skipping any this guard already tried (D-17). A won claim is emitted as one
    /// [`TraceEvent::AllowanceWarning`] on the run's own emitter and queued for the operator --
    /// the same legs admission produces. A lost claim does neither.
    async fn claim_mid_run_warnings(&self, evaluation: &super::evaluate::Evaluation) {
        let Some(notices) = self.treasurer.notices.as_deref() else {
            return; // no notice store attached: the warn leg is off
        };
        for crossing in collect_crossings(&evaluation.readings) {
            let key = ClaimKey::new(
                crossing.ceiling,
                crossing.window.map(|(start, _)| start),
                NoticeKind::Warning,
            );
            if !self.first_attempt(key) {
                continue;
            }
            let Some(notice) = self
                .treasurer
                .claim_notice(
                    notices,
                    crossing,
                    Some(&self.run_id),
                    evaluation.evaluated_at,
                )
                .await
            else {
                continue;
            };
            if let Some(emitter) = &self.emitter {
                emitter.emit(TraceEvent::from(notice.warning.clone()));
            }
            self.treasurer.notify_operator(&notice).await;
        }
    }

    /// Claim the once-per-window halt notice for the ceiling this guard just halted on and, when
    /// this run won it, queue one `allowance_halted` delivery for the operator (D-18).
    async fn claim_spend_halt(&self, exhausted: &super::evaluate::Exhausted) {
        let key = ClaimKey::new(
            &exhausted.ceiling,
            exhausted.refusal.window.map(|(start, _)| start),
            NoticeKind::Halt,
        );
        if !self.first_attempt(key) {
            return;
        }
        if let Some(notice) = self
            .treasurer
            .claim_halt_notice(&exhausted.ceiling, &exhausted.refusal, &self.run_id)
            .await
        {
            self.treasurer.notify_operator(&notice).await;
        }
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
            Ok(Some(evaluation)) => match &evaluation.exhausted {
                Some(exhausted) => {
                    let refusal = &exhausted.refusal;
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
                    // D-18: the halt notice is claimed once per window across runs; the claim
                    // observes the halt and never changes it.
                    self.claim_spend_halt(exhausted).await;
                    SpendDecision::Halt(
                        self.memoise(HaltReason::AllowanceExhausted(refusal.clone())),
                    )
                }
                // Headroom everywhere: continue, and cache no decision (D-02). A ceiling this
                // run has crossed `warn_at` on claims its once-per-window warning (D-17).
                None => {
                    self.claim_mid_run_warnings(&evaluation).await;
                    SpendDecision::Continue
                }
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
    /// Build the per-run [`SpendGuard`] for `subject` running as `run_id` (ALLOW-03, D-04),
    /// with no trace emitter.
    ///
    /// One guard per run: the worker attaches it with `WarEngine::with_spend_guard`, a child
    /// battalion run inherits the same instance, and every clone shares the memoised first
    /// halt (G11). The guard takes identity only (a [`RunAttribution`]), never a role.
    ///
    /// A guard built here still claims its mid-run warning and halt notices and queues the
    /// operator deliveries; only the run-stream warning event is left off. The worker uses the
    /// crate-private emitter-carrying constructor so a mid-run warning lands on the run's own
    /// stream.
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
        self.spend_guard_with_emitter(subject, run_id, None)
    }

    /// Build the per-run [`SpendGuard`] carrying the run's own trace `emitter` (D-17).
    ///
    /// The same guard as [`Treasurer::spend_guard`], plus the emitter a won mid-run warning is
    /// emitted on. Crate-private: the worker is the only caller that holds a run's emitter.
    pub(crate) fn spend_guard_with_emitter(
        self: &Arc<Self>,
        subject: RunAttribution,
        run_id: RunId,
        emitter: Option<Arc<dyn TraceEmitter>>,
    ) -> Arc<dyn SpendGuard> {
        Arc::new(TreasurerSpendGuard {
            treasurer: Arc::clone(self),
            subject,
            run_id,
            halted: Arc::new(OnceLock::new()),
            emitter,
            claimed: Arc::new(Mutex::new(HashSet::new())),
        })
    }
}
