//! Treasurer cost arithmetic — `Cost`, `CurrencyCode`, `PriceRow`, `PriceTable`, `cost_of_call`
//!
//! This module is the pure fixed-point cost arithmetic the Treasurer (Milestone 14) is built
//! on. Three properties hold everywhere in this file:
//!
//! - **One operator currency, no FX (D-00b).** A [`Cost`] amount is `i64` nano-units (1e-9) of
//!   a single [`CurrencyCode`]; there is no multi-currency conversion hook anywhere here.
//! - **Prices are nano-units per 1M tokens (D-01).** A [`PriceRow`] axis (e.g. `prompt`) is the
//!   cost, in nano-units of the table's currency, of one million tokens on that axis — the same
//!   scale every provider's published price sheet uses (`"2.50"` per 1M becomes
//!   `2_500_000_000` nano-units per 1M).
//! - **No floating point (D-02).** Every intermediate product is computed in `i128` before the
//!   divide-by-1,000,000 step, a call's cost is rounded exactly once — half-up — after every
//!   axis has been summed (never per axis), and the result saturates into `i64` rather than
//!   overflowing or panicking. There is no `f32`/`f64` anywhere in this module; the one and only
//!   place a nanos figure becomes a float is the display-edge conversion on
//!   [`crate::platform::container::herald::ExecutionMetadataBuilder::cost`] (D-03).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::platform::container::token_usage::TokenUsage;

/// Errors constructing the cost-arithmetic value types in this module.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CostError {
    /// [`CurrencyCode::new`] was given something other than exactly three ASCII uppercase
    /// letters.
    #[error("currency code must be exactly three ASCII uppercase letters (got {0:?})")]
    InvalidCurrencyCode(String),

    /// A [`PriceRow`] constructor or builder was given a negative nano-units-per-1M-tokens
    /// price. Zero is valid (D-07, a free tier); only negative is rejected.
    #[error("{axis} price must not be negative (got {nanos_per_million} nano-units per 1M tokens)")]
    NegativePrice {
        /// Which price axis was negative: `"prompt"`, `"completion"`, `"cache_read"`,
        /// `"cache_write"` or `"reasoning"`.
        axis: &'static str,
        /// The rejected value.
        nanos_per_million: i64,
    },
}

/// An ISO 4217-shaped three-letter currency code (e.g. `"USD"`, `"EUR"`).
///
/// Validated at construction and at deserialization alike (`#[serde(try_from = "String")]`), so
/// a [`Cost`] or [`PriceTable`] can never carry a malformed currency string (D-04, D-09 threat
/// register). Deliberately carries no `Default` — there is no sensible default currency for a
/// value type that exists specifically to prevent silent currency assumptions.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{CostError, CurrencyCode};
///
/// let usd = CurrencyCode::new("USD")?;
/// assert_eq!(usd.as_str(), "USD");
/// assert_eq!(usd.to_string(), "USD");
///
/// assert!(CurrencyCode::new("usd").is_err());
/// assert!(CurrencyCode::new("US").is_err());
/// assert!(CurrencyCode::new("USDX").is_err());
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Validate and construct a currency code: exactly three ASCII uppercase letters.
    ///
    /// # Errors
    ///
    /// [`CostError::InvalidCurrencyCode`] if `code` is not exactly three bytes, each in
    /// `b'A'..=b'Z'`.
    pub fn new(code: &str) -> Result<Self, CostError> {
        let bytes = code.as_bytes();
        if bytes.len() == 3 && bytes.iter().all(|b| b.is_ascii_uppercase()) {
            Ok(Self(code.to_string()))
        } else {
            Err(CostError::InvalidCurrencyCode(code.to_string()))
        }
    }

    /// The three-letter code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = CostError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<CurrencyCode> for String {
    fn from(value: CurrencyCode) -> Self {
        value.0
    }
}

/// A monetary amount in `i64` nano-units (1e-9) of a single [`CurrencyCode`] (D-02).
///
/// The nano-unit integer is the one authoritative figure; a `f64` display value is produced
/// exactly once, at the display edge, by
/// [`crate::platform::container::herald::ExecutionMetadataBuilder::cost`] (D-03) and is never
/// fed back into this type or any comparison/aggregation. Deliberately carries no `Default` — a
/// zero `Cost` with a fabricated currency would contradict the "unpriced is `None`, never zero"
/// rule (D-00c).
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{CostError, Cost, CurrencyCode};
///
/// let usd = CurrencyCode::new("USD")?;
/// let a = Cost::new(1_000, usd.clone());
/// let b = Cost::new(500, usd.clone());
/// assert_eq!(a.checked_add(&b), Some(Cost::new(1_500, usd.clone())));
///
/// let eur = CurrencyCode::new("EUR")?;
/// let c = Cost::new(1, eur);
/// assert_eq!(a.checked_add(&c), None, "mismatched currencies never silently combine");
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    nanos: i64,
    currency: CurrencyCode,
}

impl Cost {
    /// Construct a `Cost` from its raw nano-unit amount and currency.
    pub fn new(nanos: i64, currency: CurrencyCode) -> Self {
        Self { nanos, currency }
    }

    /// The raw nano-unit (1e-9) amount.
    pub fn nanos(&self) -> i64 {
        self.nanos
    }

    /// The currency this amount is denominated in.
    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    /// Add two costs, refusing a currency mismatch.
    ///
    /// Returns `None` when `self.currency() != other.currency()`; otherwise the saturating sum
    /// of the two nano-unit amounts in `self`'s currency. This is the safe way to combine two
    /// `Cost`s across calls — prefer this, `CostTally` (Task 2) or the [`std::iter::Sum`] impl
    /// over the unconditional [`std::ops::Add`] impl when the two operands' currencies are not
    /// already known to match.
    pub fn checked_add(&self, other: &Cost) -> Option<Cost> {
        if self.currency != other.currency {
            return None;
        }
        Some(Cost {
            nanos: self.nanos.saturating_add(other.nanos),
            currency: self.currency.clone(),
        })
    }
}

impl std::ops::Add for Cost {
    type Output = Cost;

    /// Saturating add that keeps the LEFT operand's currency unconditionally — this impl exists
    /// for the common case where both operands are already known to share a currency (e.g.
    /// accumulating a single price table's calls). Prefer [`Cost::checked_add`] or the
    /// [`std::iter::Sum`] impl when the currencies are not already known to match, since this
    /// impl does not detect a mismatch.
    fn add(self, rhs: Self) -> Self::Output {
        Cost {
            nanos: self.nanos.saturating_add(rhs.nanos),
            currency: self.currency,
        }
    }
}

impl std::ops::AddAssign for Cost {
    fn add_assign(&mut self, rhs: Self) {
        self.nanos = self.nanos.saturating_add(rhs.nanos);
    }
}

impl std::iter::Sum<Cost> for Option<Cost> {
    /// Sum an iterator of [`Cost`]: an empty iterator or any currency mismatch yields `None`;
    /// otherwise `Some` of the saturating sum.
    fn sum<I: Iterator<Item = Cost>>(mut iter: I) -> Self {
        let first = iter.next()?;
        iter.try_fold(first, |acc, x| acc.checked_add(&x))
    }
}

/// One model's per-1M-token prices, in nano-units of a [`PriceTable`]'s currency (D-06).
///
/// `prompt` and `completion` are required; `cache_read`, `cache_write` and `reasoning` are
/// optional — an omitted cache axis bills at the `prompt` price and an omitted `reasoning`
/// axis bills at the `completion` price, because [`TokenUsage`] already counts cache tokens
/// inside `prompt_tokens` and reasoning tokens inside `completion_tokens`. Zero is a valid
/// price (D-07, a free tier); only a negative price is rejected.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{CostError, PriceRow};
///
/// let row = PriceRow::new(2_500_000_000, 10_000_000_000)?
///     .with_cache_read(300_000_000)?;
/// assert_eq!(row.prompt(), 2_500_000_000);
/// assert_eq!(row.cache_read(), Some(300_000_000));
/// assert_eq!(row.reasoning(), None);
///
/// assert!(matches!(
///     PriceRow::new(-1, 0),
///     Err(CostError::NegativePrice { axis: "prompt", .. })
/// ));
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    prompt: i64,
    completion: i64,
    cache_read: Option<i64>,
    cache_write: Option<i64>,
    reasoning: Option<i64>,
}

impl PriceRow {
    /// Construct a row from its required prompt/completion prices.
    ///
    /// # Errors
    ///
    /// [`CostError::NegativePrice`] naming `"prompt"` or `"completion"` if either is negative.
    pub fn new(prompt: i64, completion: i64) -> Result<Self, CostError> {
        if prompt < 0 {
            return Err(CostError::NegativePrice {
                axis: "prompt",
                nanos_per_million: prompt,
            });
        }
        if completion < 0 {
            return Err(CostError::NegativePrice {
                axis: "completion",
                nanos_per_million: completion,
            });
        }
        Ok(Self {
            prompt,
            completion,
            cache_read: None,
            cache_write: None,
            reasoning: None,
        })
    }

    /// Set the cache-read price. Omitted, a cache-read token bills at the `prompt` price.
    ///
    /// # Errors
    ///
    /// [`CostError::NegativePrice`] naming `"cache_read"` if `nanos_per_million` is negative.
    pub fn with_cache_read(mut self, nanos_per_million: i64) -> Result<Self, CostError> {
        if nanos_per_million < 0 {
            return Err(CostError::NegativePrice {
                axis: "cache_read",
                nanos_per_million,
            });
        }
        self.cache_read = Some(nanos_per_million);
        Ok(self)
    }

    /// Set the cache-write price. Omitted, a cache-write token bills at the `prompt` price.
    ///
    /// # Errors
    ///
    /// [`CostError::NegativePrice`] naming `"cache_write"` if `nanos_per_million` is negative.
    pub fn with_cache_write(mut self, nanos_per_million: i64) -> Result<Self, CostError> {
        if nanos_per_million < 0 {
            return Err(CostError::NegativePrice {
                axis: "cache_write",
                nanos_per_million,
            });
        }
        self.cache_write = Some(nanos_per_million);
        Ok(self)
    }

    /// Set the reasoning price. Omitted, a reasoning token bills at the `completion` price.
    ///
    /// # Errors
    ///
    /// [`CostError::NegativePrice`] naming `"reasoning"` if `nanos_per_million` is negative.
    pub fn with_reasoning(mut self, nanos_per_million: i64) -> Result<Self, CostError> {
        if nanos_per_million < 0 {
            return Err(CostError::NegativePrice {
                axis: "reasoning",
                nanos_per_million,
            });
        }
        self.reasoning = Some(nanos_per_million);
        Ok(self)
    }

    /// The required prompt price, nano-units per 1M tokens.
    pub fn prompt(&self) -> i64 {
        self.prompt
    }

    /// The required completion price, nano-units per 1M tokens.
    pub fn completion(&self) -> i64 {
        self.completion
    }

    /// The cache-read price, if set (else the `prompt` price applies).
    pub fn cache_read(&self) -> Option<i64> {
        self.cache_read
    }

    /// The cache-write price, if set (else the `prompt` price applies).
    pub fn cache_write(&self) -> Option<i64> {
        self.cache_write
    }

    /// The reasoning price, if set (else the `completion` price applies).
    pub fn reasoning(&self) -> Option<i64> {
        self.reasoning
    }
}

/// Price a single LLM call against one [`PriceRow`] (D-06).
///
/// Sub-counts are first clamped so billed tokens never exceed the reported parent count — a
/// `cache_read`/`cache_write`/`reasoning` that violates [`TokenUsage`]'s own containment
/// invariant (e.g. a malformed provider response) is billed at the parent's count rather than
/// panicking or overcounting. The five axis products (`base_prompt × prompt`,
/// `cache_read × cache_read_price`, `cache_write × cache_write_price`,
/// `base_completion × completion`, `reasoning × reasoning_price`) are each computed in `i128`,
/// summed, and rounded to the nearest nano-unit — half-up, exactly once for the whole call, not
/// once per axis (D-02) — before saturating into the `i64` this function returns.
///
/// This function is infallible and total: every `TokenUsage`/`PriceRow` pair, including
/// `TokenUsage::default()`, produces a `Cost` (never a `Result` or `Option`) — the caller
/// (typically [`PriceTable::price`]) decides `None` only when no row exists for a model at all.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{CostError, CurrencyCode, PriceRow, cost_of_call};
/// use paladin_core::platform::container::token_usage::TokenUsage;
///
/// let usage = TokenUsage::new(1_000, 2_000);
/// let row = PriceRow::new(2_500_000_000, 10_000_000_000)?;
/// let usd = CurrencyCode::new("USD")?;
///
/// let cost = cost_of_call(&usage, &row, &usd);
/// assert_eq!(cost.nanos(), 22_500_000);
/// # Ok::<(), CostError>(())
/// ```
pub fn cost_of_call(usage: &TokenUsage, row: &PriceRow, currency: &CurrencyCode) -> Cost {
    let prompt_tokens = usage.prompt_tokens;
    let completion_tokens = usage.completion_tokens;

    // Clamp first so billed sub-counts never exceed their parent (T-38-05).
    let cache_read = usage.cache_read_tokens.unwrap_or(0).min(prompt_tokens);
    let remaining_after_read = prompt_tokens.saturating_sub(cache_read);
    let cache_write = usage
        .cache_write_tokens
        .unwrap_or(0)
        .min(remaining_after_read);
    let reasoning = usage.reasoning_tokens.unwrap_or(0).min(completion_tokens);

    let base_prompt = prompt_tokens - cache_read - cache_write;
    let base_completion = completion_tokens - reasoning;

    let cache_read_price = row.cache_read.unwrap_or(row.prompt);
    let cache_write_price = row.cache_write.unwrap_or(row.prompt);
    let reasoning_price = row.reasoning.unwrap_or(row.completion);

    let product = |tokens: u32, price_nanos_per_million: i64| -> i128 {
        i128::from(tokens) * i128::from(price_nanos_per_million)
    };

    let sum: i128 = product(base_prompt, row.prompt)
        + product(cache_read, cache_read_price)
        + product(cache_write, cache_write_price)
        + product(base_completion, row.completion)
        + product(reasoning, reasoning_price);

    // Half-up rounding, once, over the summed products (D-02). `sum` is always
    // non-negative: token counts are `u32` and every price axis was validated
    // non-negative at `PriceRow` construction.
    let rounded = (sum + 500_000) / 1_000_000;
    let nanos = i64::try_from(rounded).unwrap_or(i64::MAX);

    Cost::new(nanos, currency.clone())
}

/// A per-model set of [`PriceRow`]s sharing one [`CurrencyCode`] — an operator's price table
/// (D-07).
///
/// Keyed by the bare model name, matched exactly and case-sensitively against
/// `LlmResponse.model` (D-05) — no normalization, no `provider/model` composite key.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::cost::{CostError, CurrencyCode, PriceRow, PriceTable};
/// use paladin_core::platform::container::token_usage::TokenUsage;
///
/// let table = PriceTable::new(CurrencyCode::new("USD")?)
///     .with_row("gpt-4", PriceRow::new(2_500_000_000, 10_000_000_000)?);
///
/// assert!(table.price("gpt-4", &TokenUsage::new(1_000, 2_000)).is_some());
/// assert!(table.price("GPT-4", &TokenUsage::new(1_000, 2_000)).is_none());
/// assert!(table.price("claude-3", &TokenUsage::new(1_000, 2_000)).is_none());
/// # Ok::<(), CostError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PriceTable {
    currency: CurrencyCode,
    rows: BTreeMap<String, PriceRow>,
}

impl PriceTable {
    /// Construct an empty table in the given currency.
    pub fn new(currency: CurrencyCode) -> Self {
        Self {
            currency,
            rows: BTreeMap::new(),
        }
    }

    /// Add (or replace) one model's price row.
    pub fn with_row(mut self, model: impl Into<String>, row: PriceRow) -> Self {
        self.rows.insert(model.into(), row);
        self
    }

    /// This table's currency.
    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    /// The price row for `model`, if one is configured (exact, case-sensitive match, D-05).
    pub fn row(&self, model: &str) -> Option<&PriceRow> {
        self.rows.get(model)
    }

    /// `true` when the table has no rows configured (an operator who omitted `treasurer:`
    /// entirely, D-07).
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The number of configured rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Price a call against this table: `Some(cost)` when `model` has a configured row (D-06 —
    /// a present row always prices, even a zero-token call), `None` when it does not (D-00c —
    /// never a fabricated zero).
    pub fn price(&self, model: &str, usage: &TokenUsage) -> Option<Cost> {
        self.row(model)
            .map(|row| cost_of_call(usage, row, &self.currency))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpt4_row_prices_1000_prompt_2000_completion_at_22_500_000_nanos() {
        let usage = TokenUsage::new(1_000, 2_000);
        let row = PriceRow::new(2_500_000_000, 10_000_000_000).unwrap();
        let usd = CurrencyCode::new("USD").unwrap();

        let cost = cost_of_call(&usage, &row, &usd);

        assert_eq!(cost.nanos(), 22_500_000);
        assert_eq!(cost.currency(), &usd);
    }
}
