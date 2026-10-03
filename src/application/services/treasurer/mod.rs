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
//!
//! This module imports `paladin_core` and `paladin_ports` only, never a storage adapter
//! (hexagonal, D-06).

mod policy;
mod window;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::allowance::{Admission, AllowanceRefusal};
use paladin_core::platform::container::cost::Cost;
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::RunId;
use paladin_core::platform::container::treasury_ledger::{BalanceQuery, format_cost};
use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;

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
}

impl std::fmt::Debug for Treasurer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Treasurer")
            .field("policy_set", &!self.policy.is_empty())
            .finish_non_exhaustive()
    }
}

impl Treasurer {
    /// Build a Treasurer over `policy` and the ledger it reads balances from.
    pub fn new(policy: AllowancePolicy, ledger: Arc<dyn TreasuryLedgerPort>) -> Self {
        Self { policy, ledger }
    }
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
        _run_id: Option<&RunId>,
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
        }

        Ok(Admission::none())
    }

    /// Nothing to confirm yet: this Treasurer holds no notice store (41-06 gives it one).
    async fn confirm(&self, _admission: &Admission) {}

    /// Nothing to abandon yet: this Treasurer holds no notice store (41-06 gives it one).
    async fn abandon(&self, _admission: &Admission) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use chrono::TimeZone;
    use paladin_core::platform::container::allowance::{AllowanceLimitKind, AllowanceScopeKind};
    use paladin_core::platform::container::cost::CurrencyCode;
    use paladin_core::platform::container::principal::TenantId;
    use paladin_core::platform::container::treasury_ledger::{
        ReservationId, ReserveRequest, SettleOutcome, SettleRequest, SpendQuery, SpendRow,
    };
    use paladin_ports::output::treasury_ledger_port::TreasuryLedgerError;

    /// A ledger double scripting `store_now` and `balance` and recording every balance query.
    /// The four required methods are never reached by admission.
    struct ScriptedLedger {
        now: DateTime<Utc>,
        balance_nanos: i64,
        queries: Mutex<Vec<BalanceQuery>>,
        clock_reads: Mutex<u32>,
    }

    impl ScriptedLedger {
        fn new(now: DateTime<Utc>, balance_nanos: i64) -> Arc<Self> {
            Arc::new(Self {
                now,
                balance_nanos,
                queries: Mutex::new(Vec::new()),
                clock_reads: Mutex::new(0),
            })
        }
    }

    fn unused() -> TreasuryLedgerError {
        TreasuryLedgerError::InvalidRequest {
            message: "not scripted".to_string(),
        }
    }

    #[async_trait]
    impl TreasuryLedgerPort for ScriptedLedger {
        async fn reserve(&self, _r: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError> {
            Err(unused())
        }
        async fn release(&self, _r: ReservationId) -> Result<(), TreasuryLedgerError> {
            Err(unused())
        }
        async fn settle(&self, _r: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError> {
            Err(unused())
        }
        async fn spend(&self, _q: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError> {
            Err(unused())
        }
        async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError> {
            *self.clock_reads.lock().unwrap() += 1;
            Ok(self.now)
        }
        async fn balance(&self, query: BalanceQuery) -> Result<Cost, TreasuryLedgerError> {
            let currency = query.currency.clone();
            self.queries.lock().unwrap().push(query);
            Ok(Cost::new(self.balance_nanos, currency))
        }
    }

    fn usd() -> CurrencyCode {
        CurrencyCode::new("USD").expect("USD is valid")
    }

    fn subject(tenant: &str, key: &str) -> RunAttribution {
        RunAttribution::new(TenantId::new(tenant).expect("valid tenant"), key)
    }

    fn noon() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 3, 12, 0, 0)
            .single()
            .expect("valid instant")
    }

    fn policy() -> AllowancePolicy {
        AllowancePolicy::new(usd(), 80).with_api_key("svc-a", ScopeAllowance::new(86_400, 100))
    }

    #[tokio::test]
    async fn principal_without_an_entry_is_admitted_without_a_ledger_read() {
        let ledger = ScriptedLedger::new(noon(), 1_000_000);
        let treasurer = Treasurer::new(policy(), ledger.clone());

        let admission = treasurer
            .admit(&subject("acme", "svc-b"), None)
            .await
            .expect("no entry is admitted");

        assert!(admission.is_empty());
        assert_eq!(*ledger.clock_reads.lock().unwrap(), 0);
        assert!(ledger.queries.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn balance_at_the_ceiling_is_refused_with_the_window_figures() {
        let ledger = ScriptedLedger::new(noon(), 100);
        let treasurer = Treasurer::new(policy(), ledger.clone());

        let err = treasurer
            .admit(&subject("acme", "svc-a"), None)
            .await
            .expect_err("a balance at the ceiling is exhausted");

        let AdmissionError::Refused(refusal) = err else {
            panic!("expected a refusal, got {err:?}");
        };
        assert_eq!(refusal.scope_kind, AllowanceScopeKind::ApiKey);
        assert_eq!(refusal.limit_kind, AllowanceLimitKind::Window);
        assert_eq!(refusal.balance.nanos(), 100);
        assert_eq!(refusal.ceiling.nanos(), 100);
        let (start, end) = refusal.window.expect("a window refusal carries its window");
        assert_eq!(
            start,
            Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single().unwrap()
        );
        assert_eq!(
            end,
            Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).single().unwrap()
        );
        assert_eq!(refusal.evaluated_at, noon());
        assert_eq!(refusal.retry_after_secs(), Some(12 * 3_600));

        let queries = ledger.queries.lock().unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].tenant_id, "acme");
        assert_eq!(queries[0].api_key_id.as_deref(), Some("svc-a"));
        assert_eq!(queries[0].since, Some(start));
        assert_eq!(queries[0].until, Some(end));
    }

    #[tokio::test]
    async fn balance_below_the_ceiling_is_admitted() {
        let ledger = ScriptedLedger::new(noon(), 99);
        let treasurer = Treasurer::new(policy(), ledger.clone());

        let admission = treasurer
            .admit(&subject("acme", "svc-a"), None)
            .await
            .expect("below the ceiling is admitted");

        assert!(admission.is_empty());
        assert_eq!(*ledger.clock_reads.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn the_store_clock_is_truncated_to_whole_seconds() {
        let fractional = noon() + chrono::Duration::milliseconds(750);
        let ledger = ScriptedLedger::new(fractional, 100);
        let treasurer = Treasurer::new(policy(), ledger);

        let err = treasurer
            .admit(&subject("acme", "svc-a"), None)
            .await
            .expect_err("refused");
        let AdmissionError::Refused(refusal) = err else {
            panic!("expected a refusal");
        };
        assert_eq!(refusal.evaluated_at, noon());
    }
}
