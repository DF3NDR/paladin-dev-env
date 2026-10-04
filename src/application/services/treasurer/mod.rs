//! The Treasurer's admission slice (ALLOW-01, ALLOW-02, Phase 41).
//!
//! [`Treasurer`] is the facade service that answers [`AllowanceAdmissionPort`]: before a run (or
//! an agent call) is accepted it reads the ledger's balance for each allowance ceiling that
//! applies to the caller and refuses when a balance has reached its ceiling.
//!
//! - **A check, never a hold** (D-05): admission writes nothing to the ledger.
//! - **One clock** (ALLOW-01, D-01): window boundaries and `Retry-After` come only from the
//!   ledger's `store_now`, read once per admission and truncated to whole seconds -- never a
//!   process's own clock.
//! - **Fail closed** (D-10): a principal with a configured ceiling whose balance cannot be read
//!   is not admitted. A principal with no configured allowance never reaches the ledger.
//! - **Identity only** (D-09): the Treasurer takes a [`RunAttribution`] -- tenant and API key
//!   name, never a role -- so no role bypasses an allowance.
//! - **A notice observes, never gates** (ALLOW-04, D-15, D-16): on the admitted path, every
//!   ceiling whose pre-admission balance has reached its `warn_at` percent claims one durable
//!   once-per-window notice from the [`TreasuryNoticePort`] (when one is attached with
//!   [`Treasurer::with_notices`]). The claim is store-enforced across replicas; a notice-store
//!   failure is logged and the run is still admitted. A claim is made BEFORE the run row is
//!   inserted, so `abandon` gives an admitted-but-never-persisted run's notices back (a crash in
//!   between can lose a window's notice but never duplicate it).
//!
//! This module imports `paladin_core` and `paladin_ports` only, never a storage adapter
//! (hexagonal, D-06).

mod policy;
mod window;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::allowance::{
    Admission, AllowanceNotice, AllowanceRefusal, AllowanceWarning, NoticeOutcome, NoticeRecord,
    crosses_warn_threshold,
};
use paladin_core::platform::container::cost::Cost;
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::{BalanceQuery, format_cost};
use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;

pub use policy::{AllowancePolicy, Ceiling, ScopeAllowance};
pub use window::window_for;

/// The Treasurer facade service: an [`AllowancePolicy`] evaluated over a [`TreasuryLedgerPort`].
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use paladin::application::services::treasurer::{AllowancePolicy, Treasurer};
/// use paladin_core::platform::container::cost::CurrencyCode;
/// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
/// use paladin_ports::input::allowance_admission_port::AllowanceAdmissionPort;
/// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let policy = AllowancePolicy::new(CurrencyCode::new("USD")?, 80);
/// let treasurer = Treasurer::new(policy, Arc::new(InMemoryTreasuryLedger::new()));
/// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
/// // No allowance configured: admitted without a ledger read.
/// assert!(treasurer.admit(&subject, None).await?.is_empty());
/// # Ok(())
/// # }
/// ```
pub struct Treasurer {
    policy: AllowancePolicy,
    ledger: Arc<dyn TreasuryLedgerPort>,
    /// The once-per-window notice store (ALLOW-04). `None` leaves the warn leg off.
    notices: Option<Arc<dyn TreasuryNoticePort>>,
}

impl std::fmt::Debug for Treasurer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Treasurer")
            .field("policy_set", &!self.policy.is_empty())
            .field("notices", &self.notices.is_some())
            .finish_non_exhaustive()
    }
}

impl Treasurer {
    /// Build a Treasurer over `policy` and the ledger it reads balances from.
    pub fn new(policy: AllowancePolicy, ledger: Arc<dyn TreasuryLedgerPort>) -> Self {
        Self {
            policy,
            ledger,
            notices: None,
        }
    }

    /// Attach the once-per-window notice store (ALLOW-04, D-16).
    ///
    /// Without a notice store the warn leg is off: crossings are never evaluated into notices
    /// and `abandon` has nothing to discard. Production always attaches one beside the ledger
    /// (`build_treasury_notices`).
    pub fn with_notices(mut self, notices: Arc<dyn TreasuryNoticePort>) -> Self {
        self.notices = Some(notices);
        self
    }

    /// Claim the notice for one warn crossing and, when this admission won it, return it.
    ///
    /// `AlreadyRecorded` yields `None` (another admission owns the window's notice). A store
    /// error is logged naming the scope kind and tenant id and also yields `None`: a notice
    /// observes the run, it never gates it (D-15).
    async fn claim_notice(
        &self,
        notices: &dyn TreasuryNoticePort,
        crossing: Crossing<'_>,
        run_id: Option<&RunId>,
        recorded_at: DateTime<Utc>,
    ) -> Option<AllowanceNotice> {
        let ceiling = crossing.ceiling;
        let currency = self.policy.currency().clone();
        let record = NoticeRecord {
            notice_id: uuid::Uuid::now_v7().to_string(),
            tenant_id: ceiling.tenant_id.clone(),
            api_key_id: ceiling.api_key_id.clone(),
            warning: AllowanceWarning {
                scope_kind: ceiling.scope_kind,
                limit_kind: ceiling.limit_kind,
                balance: crossing.balance,
                ceiling: Cost::new(ceiling.ceiling_nanos, currency),
                window_start: crossing.window.map(|(start, _)| start),
                window_end: crossing.window.map(|(_, end)| end),
                warn_at: ceiling.warn_at,
            },
            run_id: run_id.cloned(),
            recorded_at,
        };
        match notices.record(&record).await {
            Ok(NoticeOutcome::Recorded) => Some(AllowanceNotice::from(&record)),
            Ok(NoticeOutcome::AlreadyRecorded) => None,
            Err(error) => {
                log::error!(
                    "allowance notice claim failed (the run is still admitted): scope={} \
                     tenant={} error={error}",
                    ceiling.scope_kind.as_str(),
                    ceiling.tenant_id,
                );
                None
            }
        }
    }
}

/// One ceiling whose pre-admission balance reached its warn threshold on an admitted request.
struct Crossing<'a> {
    ceiling: &'a Ceiling,
    balance: Cost,
    window: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

fn backend(message: impl Into<String>) -> AdmissionError {
    AdmissionError::Backend {
        message: message.into(),
    }
}

#[async_trait]
impl AllowanceAdmissionPort for Treasurer {
    async fn admit(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
    ) -> Result<Admission, AdmissionError> {
        let ceilings = self.policy.ceilings_for(subject);
        if ceilings.is_empty() {
            // D-03, D-10: no entry means no ledger read and no way to fail closed.
            return Ok(Admission::none());
        }

        // One store-clock read per admission, truncated to whole seconds, shared by every
        // ceiling and by Retry-After (ALLOW-01, C8).
        let now = self
            .ledger
            .store_now()
            .await
            .map_err(|e| backend(format!("store clock unavailable: {e}")))?;
        let evaluated_at = DateTime::<Utc>::from_timestamp(now.timestamp(), 0)
            .ok_or_else(|| backend("store clock is outside the representable range"))?;

        // Ceilings whose balance has reached its warn threshold; claimed only if every ceiling
        // admits (a refused request notifies nothing -- the refusal is its own signal).
        let mut crossings: Vec<Crossing<'_>> = Vec::new();

        for ceiling in &ceilings {
            let window = match ceiling.period_secs {
                Some(period) => Some(window_for(evaluated_at, period).ok_or_else(|| {
                    backend(format!(
                        "allowance period of {period}s has no representable window"
                    ))
                })?),
                None => None,
            };
            let query = BalanceQuery {
                tenant_id: ceiling.tenant_id.clone(),
                api_key_id: ceiling.api_key_id.clone(),
                currency: self.policy.currency().clone(),
                since: window.map(|(start, _)| start),
                until: window.map(|(_, end)| end),
            };
            let balance = self
                .ledger
                .balance(query)
                .await
                .map_err(|e| backend(format!("balance unavailable: {e}")))?;

            // D-05: a balance exactly at the ceiling is exhausted. Integer comparison only.
            if balance.nanos() >= ceiling.ceiling_nanos {
                let refusal = AllowanceRefusal {
                    scope_kind: ceiling.scope_kind,
                    limit_kind: ceiling.limit_kind,
                    ceiling: Cost::new(ceiling.ceiling_nanos, self.policy.currency().clone()),
                    balance,
                    window,
                    evaluated_at,
                };
                log::warn!(
                    "allowance refused: scope={} limit={} tenant={} balance={} ceiling={}",
                    refusal.scope_kind.as_str(),
                    refusal.limit_kind.as_str(),
                    subject.tenant_id,
                    format_cost(&refusal.balance),
                    format_cost(&refusal.ceiling),
                );
                return Err(AdmissionError::Refused(refusal));
            }

            // D-15: integer-only crossing test on the PRE-admission balance. A balance at the
            // ceiling was refused above, so `warn_at: 100` can never reach here.
            if crosses_warn_threshold(balance.nanos(), ceiling.ceiling_nanos, ceiling.warn_at) {
                crossings.push(Crossing {
                    ceiling,
                    balance,
                    window,
                });
            }
        }

        // The admitted path: claim every crossing's once-per-window notice (D-16). With no
        // notice store attached the warn leg is off.
        let mut admission = Admission::none();
        if let Some(notices) = &self.notices {
            for crossing in crossings {
                if let Some(notice) = self
                    .claim_notice(notices.as_ref(), crossing, run_id, evaluated_at)
                    .await
                {
                    admission = admission.with_notice(notice);
                }
            }
        }
        Ok(admission)
    }

    /// Nothing to confirm in this plan: the durable notice already exists, written at `admit`
    /// time. The operator webhook leg attaches here in 41-08.
    async fn confirm(&self, _admission: &Admission) {}

    /// Give back the notices this admission won (RESEARCH Pattern 3): the run was admitted but
    /// never persisted or enqueued, so the next admission in the same window must win them
    /// again. A discard failure is logged at `error` and never propagated.
    async fn abandon(&self, admission: &Admission) {
        let Some(notices) = &self.notices else {
            return;
        };
        if admission.is_empty() {
            return;
        }
        let ids: Vec<String> = admission
            .notices()
            .iter()
            .map(|notice| notice.notice_id.clone())
            .collect();
        if let Err(error) = notices.discard(&ids).await {
            log::error!(
                "allowance notice discard failed for an abandoned admission ({} notice(s)): {error}",
                ids.len()
            );
        }
    }
}

#[cfg(test)]
mod tests;
