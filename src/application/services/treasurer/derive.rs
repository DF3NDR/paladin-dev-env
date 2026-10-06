//! The agent loop's derived token budget (ALLOW-05, Phase 42 D-09, D-10, ADR-0057 group e).
//!
//! The agent loop cannot ask the ledger anything mid-flight, so the Treasurer converts what
//! remains of a principal's allowance into a number the loop already understands: a token count
//! for the one `TokenBudget` middleware. The conversion is deliberately pessimistic and uses
//! integer arithmetic only:
//!
//! - **Dearest axis** (D-09): a call's real price depends on how its tokens split across the
//!   five [`PriceRow`] axes, which is not known in advance, so every token is priced at the
//!   largest axis (`cost_of_call`'s defaults applied: cache read and cache write default to the
//!   prompt price, reasoning to the completion price). Sub-counts are contained in the prompt
//!   and completion counts, so total tokens at the dearest price is a valid upper bound.
//! - **Floor division in `i128`**: `floor(remaining * 1_000_000 / dearest)`. Products of an
//!   `i64` nano figure and the million scale cannot overflow `i128`, floor never rounds up, and
//!   the saturating `u32` conversion never wraps. `cost_of_call` at exactly the derived count,
//!   all tokens billed at the dearest axis, therefore never exceeds the remaining nanos.
//! - **The tightest ceiling binds**: the budget is derived from the smallest `ceiling -
//!   balance` among the principal's applicable ceilings, so no ceiling is overshot by the budget
//!   itself. The only overshoot is the one response the cutoff observes (the cutoff compares
//!   after a response completes, strictly `>`).
//! - **A free model gets no budget** (dearest price 0): nothing it does can spend an allowance.
//!   **A zero figure is a refusal**, not a zero-token run.
//!
//! Derivation reads through the same [`Treasurer::evaluate`] admission uses, so it can never
//! disagree with admission about the ceiling order, the store clock or the exhausted predicate.
//! It writes nothing.

use paladin_core::platform::container::allowance::{AllowanceRefusal, DerivedTokenBudget};
use paladin_core::platform::container::cost::{Cost, PriceRow};
use paladin_core::platform::container::principal::RunAttribution;
use paladin_ports::input::allowance_admission_port::AdmissionError;

use super::evaluate::Evaluation;
use super::{Treasurer, backend};

/// The largest of a row's five price axes, nanos per 1M tokens, with `cost_of_call`'s defaults:
/// cache read and cache write default to the prompt price, reasoning to the completion price.
pub(crate) fn dearest_price_per_million(row: &PriceRow) -> i64 {
    [
        row.prompt(),
        row.completion(),
        row.cache_read().unwrap_or_else(|| row.prompt()),
        row.cache_write().unwrap_or_else(|| row.prompt()),
        row.reasoning().unwrap_or_else(|| row.completion()),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

/// `floor(remaining_nanos * 1_000_000 / price_per_million)`, saturated to `u32`.
///
/// `None` when the price is not positive (a free model has no budget). A negative remaining
/// figure is treated as 0. Integer arithmetic only, in `i128`, so it cannot overflow.
pub(crate) fn derive_max_tokens(remaining_nanos: i64, price_per_million: i64) -> Option<u32> {
    if price_per_million <= 0 {
        return None;
    }
    let tokens = i128::from(remaining_nanos.max(0)) * 1_000_000 / i128::from(price_per_million);
    Some(u32::try_from(tokens).unwrap_or(u32::MAX))
}

impl Treasurer {
    /// Derive the per-run token budget for `subject` calling `model` (ALLOW-05, D-09).
    ///
    /// Returns `Ok(None)` when no budget applies: the principal has no configured ceiling (no
    /// ledger read is made) or the model is free (dearest price 0). Otherwise the budget is
    /// `floor(min over applicable ceilings of (ceiling - balance) * 1_000_000 / dearest price)`
    /// tokens, with `halt_figures` reporting the binding ceiling at balance equal to the ceiling
    /// (a conservative bound the loop cannot refine, ADR-0057 A5). Derivation reads through the
    /// same evaluation admission uses and writes nothing: deriving twice over an unchanged ledger
    /// and clock yields equal budgets.
    ///
    /// # Errors
    ///
    /// - [`AdmissionError::Refused`] when a ceiling is already exhausted, or the derived figure
    ///   is zero tokens (the refusal carries the binding ceiling's real figures).
    /// - [`AdmissionError::ModelUnpriced`] when a ceiling applies and `model` has no price row
    ///   (or no price table is attached with [`Treasurer::with_pricing`]).
    /// - [`AdmissionError::Backend`] when the ledger cannot be read, or the price table's
    ///   currency differs from the policy's (never converted).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use paladin::application::services::treasurer::{AllowancePolicy, Treasurer};
    /// use paladin_core::platform::container::cost::CurrencyCode;
    /// use paladin_core::platform::container::principal::{RunAttribution, TenantId};
    /// use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let policy = AllowancePolicy::new(CurrencyCode::new("USD")?, 80);
    /// let treasurer = Treasurer::new(policy, Arc::new(InMemoryTreasuryLedger::new()));
    /// let subject = RunAttribution::new(TenantId::new("acme")?, "svc-a");
    /// // No allowance configured: no ledger read and no budget.
    /// assert!(treasurer.derive_budget(&subject, "gpt-4").await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn derive_budget(
        &self,
        subject: &RunAttribution,
        model: &str,
    ) -> Result<Option<DerivedTokenBudget>, AdmissionError> {
        let Some(evaluation) = self.evaluate(subject).await? else {
            return Ok(None);
        };
        if let Some(refusal) = evaluation.exhausted {
            return Err(AdmissionError::Refused(refusal));
        }
        self.derive_from(&evaluation, model)
    }

    /// Derive from an evaluation whose ceilings all have headroom. Shared by
    /// [`Treasurer::derive_budget`] and the model-aware admission so the two cannot drift.
    pub(crate) fn derive_from(
        &self,
        evaluation: &Evaluation,
        model: &str,
    ) -> Result<Option<DerivedTokenBudget>, AdmissionError> {
        let row = self
            .pricing
            .as_ref()
            .and_then(|table| table.row(model))
            .ok_or_else(|| AdmissionError::ModelUnpriced {
                model: model.to_string(),
            })?;
        if let Some(table) = &self.pricing
            && table.currency() != self.policy.currency()
        {
            // D-00h: never converted. The pricing table and the allowance policy must agree.
            return Err(backend(format!(
                "price table currency {} differs from allowance policy currency {}",
                table.currency().as_str(),
                self.policy.currency().as_str(),
            )));
        }

        let dearest = dearest_price_per_million(row);
        if dearest <= 0 {
            // A free model cannot spend an allowance: no derived budget.
            return Ok(None);
        }

        // The binding ceiling is the one with the least headroom; a tie goes to the first in
        // policy order. Saturating, because a ledger balance is not trusted to be non-negative.
        let Some((reading, remaining)) = evaluation
            .readings
            .iter()
            .map(|reading| {
                (
                    reading,
                    reading
                        .ceiling
                        .ceiling_nanos
                        .saturating_sub(reading.balance.nanos()),
                )
            })
            .min_by_key(|(_, remaining)| *remaining)
        else {
            return Ok(None);
        };

        let currency = self.policy.currency().clone();
        let ceiling = Cost::new(reading.ceiling.ceiling_nanos, currency);
        let figures = |balance: Cost| AllowanceRefusal {
            scope_kind: reading.ceiling.scope_kind,
            limit_kind: reading.ceiling.limit_kind,
            balance,
            ceiling: ceiling.clone(),
            window: reading.window,
            evaluated_at: evaluation.evaluated_at,
        };

        match derive_max_tokens(remaining, dearest) {
            None => Ok(None),
            // D-09: a zero figure is a refusal carrying the binding ceiling's real figures.
            Some(0) => Err(AdmissionError::Refused(figures(reading.balance.clone()))),
            // A5: the loop cannot know the post-spend balance, so the cutoff reports the ceiling
            // as the balance -- a conservative bound.
            Some(max_tokens) => Ok(Some(DerivedTokenBudget::new(
                max_tokens,
                figures(ceiling.clone()),
            ))),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use paladin_core::platform::container::cost::{CurrencyCode, cost_of_call};
    use paladin_core::platform::container::token_usage::TokenUsage;

    #[test]
    fn derive_max_tokens_table() {
        assert_eq!(
            derive_max_tokens(1_000_000_000, 10_000_000_000),
            Some(100_000)
        );
        // Floor: 5 nanos at 10 USD per 1M cannot buy a whole token.
        assert_eq!(derive_max_tokens(5, 10_000_000_000), Some(0));
        // Saturates, never wraps.
        assert_eq!(derive_max_tokens(i64::MAX, 1), Some(u32::MAX));
        // A free (or nonsensical) price has no budget.
        assert_eq!(derive_max_tokens(100, 0), None);
        assert_eq!(derive_max_tokens(100, -5), None);
        // A negative remaining figure is treated as zero.
        assert_eq!(derive_max_tokens(-100, 7), Some(0));
        // Floor, not round: 7 / 3 tokens-per-nano scale.
        assert_eq!(derive_max_tokens(1, 3_000_000), Some(0));
        assert_eq!(derive_max_tokens(10, 3_000_000), Some(3));
    }

    #[test]
    fn dearest_takes_the_largest_of_the_five_axes_with_cost_of_call_defaults() {
        let plain = PriceRow::new(2, 7).unwrap();
        assert_eq!(dearest_price_per_million(&plain), 7);

        // Cache read and write default to prompt, so a high prompt wins over completion.
        let prompt_heavy = PriceRow::new(9, 3).unwrap();
        assert_eq!(dearest_price_per_million(&prompt_heavy), 9);

        // A reasoning price above completion wins.
        let reasoning = PriceRow::new(2, 7).unwrap().with_reasoning(40).unwrap();
        assert_eq!(dearest_price_per_million(&reasoning), 40);

        // Cache axes above both win.
        let cached = PriceRow::new(2, 7)
            .unwrap()
            .with_cache_read(11)
            .unwrap()
            .with_cache_write(13)
            .unwrap();
        assert_eq!(dearest_price_per_million(&cached), 13);

        // A reasoning price BELOW completion does not lower the maximum.
        let cheap_reasoning = PriceRow::new(2, 7).unwrap().with_reasoning(1).unwrap();
        assert_eq!(dearest_price_per_million(&cheap_reasoning), 7);

        // A free row is free.
        let free = PriceRow::new(0, 0).unwrap();
        assert_eq!(dearest_price_per_million(&free), 0);
    }

    /// The derived budget, billed entirely at the dearest axis, never costs more than the
    /// remaining allowance (T-42-29), including non-divisible pairs and rounding-edge cases.
    #[test]
    fn cost_at_the_derived_count_never_exceeds_the_remaining_nanos() {
        let usd = CurrencyCode::new("USD").unwrap();
        let remainders = [
            0_i64,
            1,
            2,
            5,
            999,
            1_000,
            333_333_333,
            1_000_000_000,
            2_500_000_001,
            i64::MAX,
        ];
        let prices = [
            1_i64,
            3,
            7,
            999_999,
            1_000_000,
            2_500_000_000,
            10_000_000_000,
        ];
        for &remaining in &remainders {
            for &price in &prices {
                let Some(n) = derive_max_tokens(remaining, price) else {
                    panic!("positive price always derives");
                };
                // Bill every token on the dearest axis: all completion tokens on a row whose
                // completion price is `price` and whose other axes are no dearer.
                let row = PriceRow::new(price, price).unwrap();
                let cost = cost_of_call(&TokenUsage::new(n, 0), &row, &usd);
                assert!(
                    cost.nanos() <= remaining,
                    "{n} tokens at {price} cost {} > remaining {remaining}",
                    cost.nanos()
                );
            }
        }
    }
}
