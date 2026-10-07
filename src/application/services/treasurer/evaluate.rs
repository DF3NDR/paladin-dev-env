//! The one ceiling evaluation every Treasurer decision shares (ALLOW-03, Phase 42 D-17).
//!
//! [`Treasurer::evaluate`] is the single implementation of "read the balance for each applicable
//! ceiling and say which, if any, is exhausted". Admission ([`Treasurer::admit`] through
//! `AllowanceAdmissionPort`) and the per-boundary [`TreasurerSpendGuard`](super::TreasurerSpendGuard)
//! both call it, so the two can never disagree about the ceiling order, the clock or the
//! `balance >= ceiling` predicate:
//!
//! - **Ceiling order** is the policy's own (`AllowancePolicy::ceilings_for`: key-window,
//!   key-lifetime, tenant-window, tenant-lifetime).
//! - **One clock** (ALLOW-01): one `store_now()` read per evaluation, truncated to whole seconds
//!   and shared by every ceiling -- never a process's own clock, never a floating-point value
//!   (D-00h).
//! - **Exhausted means `>=`** (D-05): a balance exactly at the ceiling is exhausted; integer
//!   nano-unit comparison only.
//! - **Short circuit**: the first exhausted ceiling stops the reading. Later ceilings are not
//!   read, so a refused request or a halting run costs no more balance reads than it needs.
//! - **Check only** (D-01): evaluation reads and never writes the ledger.

use chrono::{DateTime, Utc};

use paladin_core::platform::container::allowance::AllowanceRefusal;
use paladin_core::platform::container::cost::Cost;
use paladin_core::platform::container::principal::RunAttribution;
use paladin_core::platform::container::treasury_ledger::BalanceQuery;
use paladin_ports::input::allowance_admission_port::AdmissionError;

use super::{Ceiling, Treasurer, backend, window_for};

/// One ceiling and the balance read for it (and the window the read covered).
#[derive(Debug, Clone)]
pub(crate) struct CeilingReading {
    /// The ceiling that was evaluated.
    pub(crate) ceiling: Ceiling,
    /// The ledger balance read for the ceiling's scope and window.
    pub(crate) balance: Cost,
    /// The half-open `[start, end)` window the balance covers; `None` for a lifetime ceiling.
    pub(crate) window: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// The first exhausted ceiling of an evaluation: the ceiling itself (its identity and `warn_at`
/// key the halt notice) and the refusal carrying the figures read for it.
#[derive(Debug, Clone)]
pub(crate) struct Exhausted {
    /// The ceiling whose balance has reached it.
    pub(crate) ceiling: Ceiling,
    /// The refusal figures: balance, ceiling, window and the evaluation instant.
    pub(crate) refusal: AllowanceRefusal,
}

/// The outcome of evaluating every applicable ceiling for one principal.
#[derive(Debug, Clone)]
pub(crate) struct Evaluation {
    /// The store instant (whole seconds) every window was computed from.
    pub(crate) evaluated_at: DateTime<Utc>,
    /// Every ceiling read, in policy order, up to and including the first exhausted one.
    pub(crate) readings: Vec<CeilingReading>,
    /// The first exhausted ceiling and its refusal; `None` when every ceiling has headroom.
    pub(crate) exhausted: Option<Exhausted>,
}

impl Treasurer {
    /// Evaluate every ceiling that applies to `subject`.
    ///
    /// `Ok(None)` means no ceiling applies: no ledger call of any kind was made (D-03, D-10).
    /// Otherwise one store-clock read is made, then each ceiling's balance in policy order, and
    /// reading stops at the first ceiling whose balance has reached it.
    ///
    /// # Errors
    ///
    /// [`AdmissionError::Backend`] when the store clock, a window computation or a balance read
    /// fails -- the caller fails closed (D-03, D-10).
    pub(crate) async fn evaluate(
        &self,
        subject: &RunAttribution,
    ) -> Result<Option<Evaluation>, AdmissionError> {
        let ceilings = self.policy.ceilings_for(subject);
        if ceilings.is_empty() {
            // D-03, D-10: no entry means no ledger read and no way to fail closed.
            return Ok(None);
        }

        // One store-clock read per evaluation, truncated to whole seconds, shared by every
        // ceiling and by Retry-After (ALLOW-01, C8).
        let now = self
            .ledger
            .store_now()
            .await
            .map_err(|e| backend(format!("store clock unavailable: {e}")))?;
        let evaluated_at = DateTime::<Utc>::from_timestamp(now.timestamp(), 0)
            .ok_or_else(|| backend("store clock is outside the representable range"))?;

        let mut readings = Vec::with_capacity(ceilings.len());
        for ceiling in ceilings {
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
                    balance: balance.clone(),
                    window,
                    evaluated_at,
                };
                readings.push(CeilingReading {
                    ceiling: ceiling.clone(),
                    balance,
                    window,
                });
                return Ok(Some(Evaluation {
                    evaluated_at,
                    readings,
                    exhausted: Some(Exhausted { ceiling, refusal }),
                }));
            }
            readings.push(CeilingReading {
                ceiling,
                balance,
                window,
            });
        }
        Ok(Some(Evaluation {
            evaluated_at,
            readings,
            exhausted: None,
        }))
    }
}
