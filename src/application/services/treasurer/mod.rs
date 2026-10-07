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
//! - **The operator webhook rides the durable queue** (ALLOW-04, D-17): when an
//!   [`OperatorNoticeTarget`] is attached ([`Treasurer::with_operator_webhook`]), `confirm`
//!   enqueues one `webhook_deliveries` row (event `allowance_warning`) per notice the
//!   admission won, delivered later by the existing signed, SSRF-guarded, no-redirect
//!   `WebhookDeliveryService`. This module never builds an HTTP client and never holds the
//!   signing secret -- the delivery service does (C3).
//!
//! - **One evaluation, two callers** (ALLOW-03, Phase 42 D-17): admission and the per-boundary
//!   [`TreasurerSpendGuard`] share one ceiling evaluation (`Treasurer::evaluate`): the same
//!   ceiling order, the same single truncated store-clock read, the same `balance >= ceiling`
//!   predicate, and the same stop at the first exhausted ceiling. The guard is how an allowance
//!   exhausted mid-run halts the run at its next superstep boundary.
//!
//! - **A derived budget for the agent loop** (ALLOW-05, Phase 42 D-09, D-10): with a price table
//!   attached ([`Treasurer::with_pricing`]), [`Treasurer::derive_budget`] and the model-aware
//!   admission ([`AllowanceAdmissionPort::admit_for_model`]) turn the remaining allowance into a
//!   per-run token budget at the model's dearest price, refuse an unpriced model
//!   ([`AdmissionError::ModelUnpriced`]) and a zero budget, and read through the same shared
//!   evaluation. See `derive.rs`.
//!
//! This module imports `paladin_core` and `paladin_ports` only, never a storage adapter
//! (hexagonal, D-06).

mod derive;
mod evaluate;
mod guard;
mod policy;
mod window;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use paladin_core::platform::container::allowance::{
    Admission, AllowanceNotice, AllowanceWarning, NoticeKind, NoticeOutcome, NoticeRecord,
    crosses_warn_threshold,
};
use paladin_core::platform::container::cost::{Cost, PriceTable};
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::run::{RunEventKind, RunId};
use paladin_core::platform::container::treasury_ledger::format_cost;
use paladin_core::platform::container::waypoint::ThreadId;
use paladin_core::platform::container::webhook::{WebhookDelivery, WebhookDeliveryId};
use paladin_ports::input::allowance_admission_port::{AdmissionError, AllowanceAdmissionPort};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

use crate::application::services::run::webhook::AllowanceWarningPayload;

pub use guard::TreasurerSpendGuard;
pub use policy::{AllowancePolicy, Ceiling, ScopeAllowance};
pub use window::window_for;

/// The thread id every operator allowance delivery row carries (C3 Option A). No run lives on
/// this thread, which is one of the two reasons an operator notice never appears under a
/// caller's `GET /v1/runs/{id}/webhook-deliveries` (the other is the correlation run id).
pub const OPERATOR_NOTICE_THREAD_ID: &str = "treasurer-notices";

/// Where an operator allowance notice is delivered (D-17): the operator's webhook URL and the
/// durable delivery queue the notice is enqueued onto.
///
/// Carries no signing secret: the secret lives on `WebhookDeliveryService`
/// (`with_operator_notice_secret`) and never touches a delivery row. `Debug` prints the URL
/// only.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use paladin::application::services::treasurer::OperatorNoticeTarget;
/// use paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository;
///
/// let target = OperatorNoticeTarget::new(
///     "https://ops.example.com/allowance",
///     Arc::new(InMemoryWebhookDeliveryRepository::new()),
/// );
/// assert!(format!("{target:?}").contains("ops.example.com"));
/// ```
#[derive(Clone)]
pub struct OperatorNoticeTarget {
    url: String,
    deliveries: Arc<dyn WebhookDeliveryRepositoryPort>,
}

impl OperatorNoticeTarget {
    /// Build a target delivering to `url` through the `deliveries` queue.
    pub fn new(url: impl Into<String>, deliveries: Arc<dyn WebhookDeliveryRepositoryPort>) -> Self {
        Self {
            url: url.into(),
            deliveries,
        }
    }
}

impl std::fmt::Debug for OperatorNoticeTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperatorNoticeTarget")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

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
    /// The operator webhook target (D-17). `None` disables only the webhook leg.
    operator_webhook: Option<OperatorNoticeTarget>,
    /// The per-model price table the derived agent budget is computed from (ALLOW-05, D-09).
    /// `None` means no model-aware derivation can be made.
    pricing: Option<Arc<PriceTable>>,
}

impl std::fmt::Debug for Treasurer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Treasurer")
            .field("policy_set", &!self.policy.is_empty())
            .field("notices", &self.notices.is_some())
            .field("operator_webhook", &self.operator_webhook)
            .field("pricing", &self.pricing.is_some())
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
            operator_webhook: None,
            pricing: None,
        }
    }

    /// Attach the per-model price table the agent loop's derived token budget is computed from
    /// (ALLOW-05, Phase 42 D-09).
    ///
    /// Without a table, a principal that has a configured ceiling is refused with
    /// [`AdmissionError::ModelUnpriced`] by [`AllowanceAdmissionPort::admit_for_model`] (an
    /// allowance cannot meter an unpriced model); a principal with no ceiling is unaffected. The
    /// table's currency must match the allowance policy's, or derivation reports a backend error
    /// rather than converting.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use paladin::application::services::treasurer::{AllowancePolicy, Treasurer};
    /// use paladin_core::platform::container::cost::{CurrencyCode, PriceRow, PriceTable};
    /// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    /// use paladin_ports::input::allowance_admission_port::AllowanceAdmissionPort;
    /// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let usd = CurrencyCode::new("USD")?;
    /// let prices = PriceTable::new(usd.clone())
    ///     .with_row("gpt-4", PriceRow::new(2_500_000_000, 10_000_000_000)?);
    /// let treasurer = Treasurer::new(
    ///     AllowancePolicy::new(usd, 80),
    ///     Arc::new(InMemoryTreasuryLedger::new()),
    /// )
    /// .with_pricing(Arc::new(prices));
    /// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
    /// // No allowance configured for this principal: admitted with no derived budget.
    /// let admission = treasurer.admit_for_model(&subject, None, "gpt-4").await?;
    /// assert!(admission.derived_budget().is_none());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_pricing(mut self, table: Arc<PriceTable>) -> Self {
        self.pricing = Some(table);
        self
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

    /// Attach the operator webhook target (ALLOW-04, D-17).
    ///
    /// Every notice an admission won is enqueued onto the target's delivery queue by
    /// [`AllowanceAdmissionPort::confirm`]. Without a target only the webhook leg is off: the
    /// notice row, the trace event and the herald line still occur.
    #[must_use]
    pub fn with_operator_webhook(mut self, target: OperatorNoticeTarget) -> Self {
        self.operator_webhook = Some(target);
        self
    }

    /// Enqueue one operator delivery for a won notice. Best-effort: every failure is logged
    /// at `error` (scope kind, tenant id, error -- never a key value or the secret) and
    /// swallowed, because the run is already admitted and the notice row already exists.
    async fn enqueue_operator_delivery(
        &self,
        target: &OperatorNoticeTarget,
        notice: &AllowanceNotice,
    ) {
        let scope = notice.warning.scope_kind.as_str();
        let payload = match serde_json::to_string(&AllowanceWarningPayload::from_notice(notice)) {
            Ok(payload) => payload,
            Err(error) => {
                log::error!(
                    "operator allowance notice not enqueued ({scope} scope, tenant {}): \
                     payload serialization failed: {error}",
                    notice.tenant_id
                );
                return;
            }
        };
        let thread_id = match ThreadId::new(OPERATOR_NOTICE_THREAD_ID) {
            Ok(thread_id) => thread_id,
            Err(error) => {
                log::error!(
                    "operator allowance notice not enqueued ({scope} scope, tenant {}): \
                     invalid notice thread id: {error}",
                    notice.tenant_id
                );
                return;
            }
        };
        // A fresh correlation run id that no run owns (C3 Option A): the row can never be
        // listed under the admitting run, and the delivery service signs it without a run
        // lookup. The REAL admitting run id travels in the payload.
        let delivery = WebhookDelivery::new(
            WebhookDeliveryId::new_v7(),
            RunId::new_v7(),
            thread_id,
            RunEventKind::AllowanceWarning,
            target.url.clone(),
            payload,
            notice.recorded_at,
        );
        if let Err(error) = target.deliveries.enqueue(delivery).await {
            log::error!(
                "operator allowance notice not enqueued ({scope} scope, tenant {}): {error}",
                notice.tenant_id
            );
        }
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
            kind: NoticeKind::Warning,
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

impl Treasurer {
    /// The one admission body behind both [`AllowanceAdmissionPort::admit`] (`model: None`) and
    /// [`AllowanceAdmissionPort::admit_for_model`] (`model: Some`). The derivation sits after the
    /// exhausted check and before any notice claim, so a refused derivation (an unpriced model, a
    /// zero budget) claims nothing.
    async fn admit_inner(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
        model: Option<&str>,
    ) -> Result<Admission, AdmissionError> {
        // The one shared evaluation (Phase 42 D-17): the same ceiling order, one truncated
        // store-clock read and `balance >= ceiling` predicate the mid-run boundary guard uses.
        let Some(evaluation) = self.evaluate(subject).await? else {
            // D-03, D-10: no entry means no ledger read and no way to fail closed.
            return Ok(Admission::none());
        };
        let evaluated_at = evaluation.evaluated_at;

        if let Some(refusal) = evaluation.exhausted {
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

        // ALLOW-05 (D-09, D-10): with the model known, convert the remaining allowance into the
        // run's token budget. A refusal here (zero budget, unpriced model) precedes every notice
        // claim, so nothing needs giving back.
        let derived = match model {
            Some(model) => self.derive_from(&evaluation, model)?,
            None => None,
        };

        // Ceilings whose balance has reached its warn threshold; claimed only because every
        // ceiling admits (a refused request notifies nothing -- the refusal is its own signal).
        // D-15: integer-only crossing test on the PRE-admission balance. A balance at the
        // ceiling was refused above, so `warn_at: 100` can never reach here.
        let crossings: Vec<Crossing<'_>> = evaluation
            .readings
            .iter()
            .filter(|reading| {
                crosses_warn_threshold(
                    reading.balance.nanos(),
                    reading.ceiling.ceiling_nanos,
                    reading.ceiling.warn_at,
                )
            })
            .map(|reading| Crossing {
                ceiling: &reading.ceiling,
                balance: reading.balance.clone(),
                window: reading.window,
            })
            .collect();

        // The admitted path: claim every crossing's once-per-window notice (D-16). With no
        // notice store attached the warn leg is off.
        let mut admission = Admission::none();
        if let Some(budget) = derived {
            admission = admission.with_derived_budget(budget);
        }
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
}

#[async_trait]
impl AllowanceAdmissionPort for Treasurer {
    async fn admit(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
    ) -> Result<Admission, AdmissionError> {
        self.admit_inner(subject, run_id, None).await
    }

    /// The model-aware admission (ALLOW-05, D-09, D-10): [`AllowanceAdmissionPort::admit`] plus
    /// the derived per-run token budget on the returned [`Admission`].
    ///
    /// A principal with no ceiling is admitted with no budget and no ledger read. A principal
    /// with a ceiling and an unpriced `model` is refused with [`AdmissionError::ModelUnpriced`]
    /// before any notice claim; a derived budget of zero tokens is refused with
    /// [`AdmissionError::Refused`] carrying the binding ceiling's figures.
    async fn admit_for_model(
        &self,
        subject: &RunAttribution,
        run_id: Option<&RunId>,
        model: &str,
    ) -> Result<Admission, AdmissionError> {
        self.admit_inner(subject, run_id, Some(model)).await
    }

    /// Deliver the operator notices this admission won (D-17): when an
    /// [`OperatorNoticeTarget`] is attached, enqueue exactly one `allowance_warning` delivery
    /// per won notice through the durable webhook queue. The durable notice row already exists
    /// from `admit`; this adds only the webhook leg.
    ///
    /// Best-effort: a serialization, thread-id or enqueue failure is logged at `error` and not
    /// retried, and `confirm` never fails or blocks the run. With no target attached nothing
    /// is enqueued.
    async fn confirm(&self, admission: &Admission) {
        let Some(target) = &self.operator_webhook else {
            return;
        };
        for notice in admission.notices() {
            self.enqueue_operator_delivery(target, notice).await;
        }
    }

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
