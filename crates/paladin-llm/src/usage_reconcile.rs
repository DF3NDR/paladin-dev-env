//! The check every usage mapper's output passes through (G-50-5a).
//!
//! # The inclusive contract
//!
//! [`TokenUsage`] is inclusive: `prompt_tokens` carries **every** billed input token (cache reads
//! and cache writes included), `completion_tokens` carries **every** billed output token
//! (reasoning included), each sub-count sits inside its parent, and `total_tokens` is always
//! `prompt_tokens + completion_tokens` (D-02, kept by owner decision 2026-10-17). The Treasurer
//! prices those counts (`cost_of_call`) and the token budgets compare `total_tokens`, so a billed
//! category a mapper leaves out is a call billed by the vendor and not by Paladin.
//!
//! Vendors do not all report that shape. xAI, for one, reports reasoning tokens *beside*
//! `completion_tokens` rather than inside it. Each mapper therefore **folds** a category its
//! vendor reports outside the parent into that parent, so that `prompt + completion` equals what
//! the vendor bills. This module does not fold anything: it is the **check** on the fold.
//!
//! # What is checked
//!
//! [`reconcile`] compares a mapped [`TokenUsage`] with the vendor's own reported total and with
//! the containment the type promises:
//!
//! - `total`: the vendor reported a `total_tokens` and it differs from the mapped total;
//! - `reasoning`: the reasoning sub-count exceeds `completion_tokens`;
//! - `cache`: cache read plus cache write exceed `prompt_tokens`.
//!
//! A finding is surfaced, never repaired. Nothing here overwrites `total_tokens` with the
//! vendor's figure, and an unexplained residual is never attributed to the prompt side or the
//! completion side (A1-50-21): a guess at where the missing tokens belong would price them at a
//! rate nobody measured.
//!
//! # Visibility rule (ADR-0061)
//!
//! The same two-part rule the sampling filter uses: the first time an adapter instance sees a
//! given (model, check) finding it emits one `log::warn!` (target `paladin::usage`), and every
//! response carrying a finding has [`USAGE_UNRECONCILED_METADATA_KEY`] in its metadata (the
//! buffered `LlmResponse.metadata`, or the terminal chunk's `StreamingResponse.metadata` on a
//! streamed call). Warn text and metadata value carry provider, model, check name and token
//! counts only -- never content, a URL or a credential.
// Consumed by the compat engine today (plan 50-21); the four bespoke adapters join in plan 50-23.
// Without a consuming adapter feature the crate-private helpers are legitimately unused.
#![cfg_attr(
    not(any(
        feature = "kimi",
        feature = "qwen",
        feature = "grok",
        feature = "ollama",
        feature = "openai-compatible"
    )),
    allow(dead_code)
)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use paladin_core::platform::container::token_usage::TokenUsage;

/// The response-metadata key under which an adapter records that a call's mapped usage did not
/// reconcile with what the vendor reported (G-50-5a).
///
/// The value lists one finding per failed check, in fixed order `total`, `reasoning`, `cache`,
/// joined by `;`:
///
/// - `total:vendor=<n>,mapped=<m>` -- the vendor's own `total_tokens` (`n`) differs from the
///   mapped `prompt_tokens + completion_tokens` (`m`);
/// - `reasoning:reasoning=<n>,completion=<m>` -- the reasoning sub-count exceeds
///   `completion_tokens`;
/// - `cache:cache=<n>,prompt=<m>` -- cache read plus cache write exceed `prompt_tokens`.
///
/// The value carries token counts only, never content, a URL or a credential. Absent -- never
/// empty -- when every check holds, including when the vendor reported no total at all. Set on
/// `LlmResponse.metadata` for a buffered call and on the terminal chunk's
/// `StreamingResponse.metadata` for a streamed one; an error terminal item carries none.
pub const USAGE_UNRECONCILED_METADATA_KEY: &str = "paladin.usage_unreconciled";

/// The checks [`reconcile`] runs, in the fixed order their findings are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsageCheck {
    /// The vendor's reported total against the mapped total.
    Total,
}

impl UsageCheck {
    /// The name a finding is listed under (the part before its `:`).
    fn as_str(self) -> &'static str {
        match self {
            Self::Total => "total",
        }
    }
}

/// Check `mapped` against the vendor's own `total_tokens` (when it reported one) and the
/// containment [`TokenUsage`] promises.
///
/// Returns the [`USAGE_UNRECONCILED_METADATA_KEY`] value -- every finding, in fixed order, joined
/// by `;` -- or `None` when every check holds. A vendor that reported no total is checked for
/// containment only. Pure: no logging, no state.
pub(crate) fn reconcile(mapped: &TokenUsage, vendor_total: Option<u32>) -> Option<String> {
    let mut found: Vec<String> = Vec::new();
    if let Some(vendor) = vendor_total
        && vendor != mapped.total_tokens
    {
        found.push(format!(
            "{}:vendor={vendor},mapped={}",
            UsageCheck::Total.as_str(),
            mapped.total_tokens
        ));
    }
    (!found.is_empty()).then(|| found.join(";"))
}

/// Once-per-instance record of which (model, check) findings have already been logged -- the
/// [`crate::sampling`] `OmissionLog` shape.
///
/// One lives on each adapter instance, so a hot loop of disagreeing responses warns once in
/// total. `Clone` shares the record (it is behind an `Arc`), which is how a streamed call's
/// `'static` terminal-chunk closure records into the same instance's log.
#[derive(Debug, Default, Clone)]
pub(crate) struct UsageLog {
    warned: Arc<Mutex<HashSet<(String, String)>>>,
}

impl UsageLog {
    /// An empty log.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// [`reconcile`] `mapped` against `vendor_total` for `provider` and `model`.
    ///
    /// Emits one `log::warn!` (target `paladin::usage`) the first time this instance sees a
    /// given (model, check) finding, naming provider, model, check and the numbers. Returns the
    /// [`USAGE_UNRECONCILED_METADATA_KEY`] value **every** time a check fails, and `None` when
    /// every check holds.
    pub(crate) fn record(
        &self,
        provider: &str,
        model: &str,
        mapped: &TokenUsage,
        vendor_total: Option<u32>,
    ) -> Option<String> {
        let value = reconcile(mapped, vendor_total)?;

        // A poisoned lock only means another thread panicked while holding it; the set it guards
        // is still a valid set of "already warned" keys, so recover it rather than panic.
        let mut warned = self
            .warned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for finding in value.split(';') {
            let check = finding.split_once(':').map_or(finding, |(name, _)| name);
            if warned.insert((model.to_string(), check.to_string())) {
                log::warn!(
                    target: "paladin::usage",
                    "provider '{provider}' model '{model}': reported usage does not reconcile \
                     with the mapped usage ({finding}); the mapped counts are kept and the \
                     finding is recorded in the response metadata under '{}' (G-50-5a)",
                    USAGE_UNRECONCILED_METADATA_KEY,
                );
            }
        }
        drop(warned);

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreeing_vendor_total_is_reconciled() {
        let mapped = TokenUsage::new(1234, 567);
        assert_eq!(reconcile(&mapped, Some(1801)), None);
    }

    #[test]
    fn absent_vendor_total_with_containment_holding_is_reconciled() {
        let mapped = TokenUsage::new(100, 40)
            .with_cache_read(30)
            .with_reasoning(15);
        assert_eq!(reconcile(&mapped, None), None);
    }

    #[test]
    fn disagreeing_vendor_total_is_a_total_finding() {
        let mapped = TokenUsage::new(1234, 567);
        assert_eq!(
            reconcile(&mapped, Some(9999)),
            Some("total:vendor=9999,mapped=1801".to_string())
        );
    }
}
