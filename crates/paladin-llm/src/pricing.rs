//! Pricing decorator — `PricingLlmAdapter` (D-08, D-09; ADR-0052)
//!
//! [`PricingLlmAdapter`] is a stateless `Arc<dyn LlmPort>` decorator apart from its immutable
//! `Arc<PriceTable>` and one process-wide warn-once set — the sibling shape to
//! [`crate::fallback::FallbackLlmAdapter`]. Per ADR-0052, cost is metered at the `LlmPort`
//! boundary on both run paths, so this decorator composes OUTSIDE any `FallbackLlmAdapter`
//! (`Pricing(Fallback(..))`): it prices whatever response was actually served, from that
//! response's own `model` and `usage`, so it prices identically for a single provider, a
//! fallback hop, or a mixed-model graph.
//!
//! A streaming call is priced from the terminal chunk's usage only (Phase 31 contract) against
//! the REQUEST's model string, since [`StreamingResponse`] carries no model of its own.
//!
//! ## No cross-call state (D-09)
//!
//! The adapter holds only its immutable price table and delegates every identity method
//! (`validate_model`, `get_available_models`, `get_provider_name`, `get_capabilities`)
//! unchanged. `generate` is unchanged in this task -- 38-04 adds non-streaming pricing with
//! `LlmResponse.cost`.

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use paladin_core::platform::container::cost::PriceTable;
use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};

/// The log/trace target every unpriced-model warning is emitted under (D-08).
pub const PRICING_LOG_TARGET: &str = "paladin::pricing";

/// The number of distinct unpriced model names this process will warn about before it starts
/// suppressing further warnings -- bounds memory against an attacker-chosen model string
/// (T-38-07).
const UNPRICED_MODEL_WARNING_CAPACITY: usize = 256;

/// A process-wide, per-model-name warn-once set (D-08): [`UnpricedModelWarnings::should_warn`]
/// answers `true` exactly once per distinct model name, then `false` forever after -- so an
/// unpriced model logs exactly one `warn!` line no matter how many calls follow. Once
/// `capacity` distinct names have been recorded, further NEW names also answer `false`, and one
/// additional "warnings suppressed" line is logged the first time the cap is hit.
struct UnpricedModelWarnings {
    seen: Mutex<HashSet<String>>,
    capacity: usize,
    cap_announced: AtomicBool,
}

impl UnpricedModelWarnings {
    fn new(capacity: usize) -> Self {
        Self {
            seen: Mutex::new(HashSet::new()),
            capacity,
            cap_announced: AtomicBool::new(false),
        }
    }

    /// `true` the first time `model` is seen; `false` on every subsequent call for the same
    /// name, and `false` for any new name once `capacity` distinct names are already held.
    /// Lock poisoning is recovered (`PoisonError::into_inner`) rather than propagated -- this is
    /// library code and must never panic.
    fn should_warn(&self, model: &str) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if seen.contains(model) {
            return false;
        }
        if seen.len() >= self.capacity {
            if !self.cap_announced.swap(true, Ordering::SeqCst) {
                log::warn!(
                    target: PRICING_LOG_TARGET,
                    "unpriced-model warning capacity ({}) reached; further unpriced-model \
                     warnings are suppressed for the remainder of this process",
                    self.capacity
                );
            }
            return false;
        }
        seen.insert(model.to_string());
        true
    }
}

/// The process-wide instance every [`PricingLlmAdapter`] consults (D-08's dedup unit is per
/// process, not per adapter instance).
static UNPRICED_MODEL_WARNINGS: LazyLock<UnpricedModelWarnings> =
    LazyLock::new(|| UnpricedModelWarnings::new(UNPRICED_MODEL_WARNING_CAPACITY));

/// Decorates an `Arc<dyn LlmPort>` with per-call cost pricing (D-09).
///
/// Construct through [`PricingLlmAdapter::new`] or, preferably,
/// [`with_pricing`], which skips installing the decorator entirely when the price table is
/// empty. `Clone` is cheap: both fields are `Arc`s.
#[derive(Clone)]
pub struct PricingLlmAdapter {
    inner: Arc<dyn LlmPort>,
    table: Arc<PriceTable>,
}

impl fmt::Debug for PricingLlmAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PricingLlmAdapter")
            .field("inner_provider", &self.inner.get_provider_name())
            .field("currency", &self.table.currency().as_str())
            .field("rows", &self.table.len())
            .finish()
    }
}

impl PricingLlmAdapter {
    /// Wrap `inner` with per-call pricing from `table`.
    pub fn new(inner: Arc<dyn LlmPort>, table: Arc<PriceTable>) -> Self {
        Self { inner, table }
    }
}

/// Wrap `inner` with pricing from `table`, UNLESS `table` is empty -- an operator who never
/// configured `treasurer.pricing` gets `inner` back unchanged (`Arc::ptr_eq` holds), so an empty
/// table installs no extra layer at all.
pub fn with_pricing(inner: Arc<dyn LlmPort>, table: &Arc<PriceTable>) -> Arc<dyn LlmPort> {
    if table.is_empty() {
        return inner;
    }
    Arc::new(PricingLlmAdapter::new(inner, Arc::clone(table)))
}

#[async_trait]
impl LlmPort for PricingLlmAdapter {
    /// Unchanged in this task -- delegates to `inner`. 38-04 adds pricing here
    /// (`LlmResponse.cost`, D-10).
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.inner.generate(request).await
    }

    /// Prices the terminal chunk of the delegated stream from ITS OWN usage against the
    /// request's model (D-09): a chunk whose `finish_reason.is_some()` and whose `usage` is
    /// `Some` gets [`StreamingResponse::with_cost`] when [`PriceTable::price`] answers `Some`;
    /// when it answers `None`, the model is unpriced and gets exactly one `warn!` line per
    /// distinct model name per process (D-08) -- never a fabricated zero. A terminal chunk with
    /// no usage at all gets no cost and no pricing warning (the execution service already warns
    /// about the missing usage, Phase 31 D-17). Every non-terminal chunk passes through
    /// unchanged.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let model = request.model.clone();
        let table = Arc::clone(&self.table);
        let raw = self.inner.generate_stream(request).await?;
        let priced = Box::into_pin(raw).map(move |item| {
            item.map(|chunk| {
                if chunk.finish_reason.is_none() {
                    return chunk;
                }
                let Some(usage) = chunk.usage.clone() else {
                    return chunk;
                };
                match table.price(&model, &usage) {
                    Some(cost) => chunk.with_cost(cost),
                    None => {
                        if UNPRICED_MODEL_WARNINGS.should_warn(&model) {
                            log::warn!(
                                target: PRICING_LOG_TARGET,
                                "no price configured for model {model:?}; its cost is reported \
                                 as unknown until a treasurer.pricing entry is added for it"
                            );
                        }
                        chunk
                    }
                }
            })
        });
        Ok(Box::new(priced))
    }

    /// Unchanged -- delegates to `inner`.
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        self.inner.validate_model(model).await
    }

    /// Unchanged -- delegates to `inner`.
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.get_available_models().await
    }

    /// Unchanged -- delegates to `inner`.
    fn get_provider_name(&self) -> &'static str {
        self.inner.get_provider_name()
    }

    /// Unchanged -- delegates to `inner`.
    fn get_capabilities(&self) -> ProviderCapabilities {
        self.inner.get_capabilities()
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::mock::MockLlmAdapter;
    use paladin_core::platform::container::cost::{CurrencyCode, PriceRow};
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_core::platform::container::token_usage::TokenUsage;

    fn request(model: &str) -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "quest".to_string(),
            context: None,
        }))
        .unwrap();
        LlmRequest::new(model, prompt)
    }

    fn usd_table() -> Arc<PriceTable> {
        Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ))
    }

    async fn collect_stream(adapter: &PricingLlmAdapter, model: &str) -> Vec<StreamingResponse> {
        let stream = adapter.generate_stream(request(model)).await.unwrap();
        Box::into_pin(stream)
            .map(|item| item.unwrap())
            .collect()
            .await
    }

    #[tokio::test]
    async fn priced_terminal_chunk_carries_the_cost() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let chunks = collect_stream(&adapter, "gpt-4").await;
        let terminal = chunks
            .iter()
            .find(|c| c.finish_reason.is_some())
            .expect("a terminal chunk must be produced");
        let cost = terminal
            .cost
            .as_ref()
            .expect("priced model must carry a cost");
        assert_eq!(cost.nanos(), 22_500_000);
        assert_eq!(cost.currency().as_str(), "USD");
    }

    #[tokio::test]
    async fn non_final_chunks_carry_no_cost() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let chunks = collect_stream(&adapter, "gpt-4").await;
        for chunk in chunks.iter().filter(|c| c.finish_reason.is_none()) {
            assert!(
                chunk.cost.is_none(),
                "a non-final chunk must never carry a cost"
            );
        }
    }

    #[tokio::test]
    async fn unpriced_model_leaves_cost_none() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let chunks = collect_stream(&adapter, "unpriced-model-pricing-test").await;
        let terminal = chunks
            .iter()
            .find(|c| c.finish_reason.is_some())
            .expect("a terminal chunk must be produced");
        assert!(
            terminal.cost.is_none(),
            "an unpriced model must never carry a cost"
        );
    }

    #[tokio::test]
    async fn terminal_chunk_with_no_usage_gets_no_cost() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_no_streamed_usage(),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let chunks = collect_stream(&adapter, "gpt-4").await;
        let terminal = chunks
            .iter()
            .find(|c| c.finish_reason.is_some())
            .expect("a terminal chunk must be produced");
        assert!(terminal.usage.is_none());
        assert!(
            terminal.cost.is_none(),
            "a terminal chunk with no usage must never carry a cost"
        );
    }

    #[test]
    fn with_pricing_on_an_empty_table_returns_the_same_arc() {
        let mock: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let empty = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()));

        let wrapped = with_pricing(Arc::clone(&mock), &empty);

        assert!(
            Arc::ptr_eq(&mock, &wrapped),
            "an empty table must install no extra layer"
        );
    }

    #[test]
    fn with_pricing_on_a_non_empty_table_wraps_the_provider() {
        let mock: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
        let table = usd_table();

        let wrapped = with_pricing(Arc::clone(&mock), &table);

        assert!(
            !Arc::ptr_eq(&mock, &wrapped),
            "a non-empty table must install the decorator"
        );
    }

    #[test]
    fn unpriced_model_warnings_warns_once_per_model_and_stops_at_capacity() {
        let warnings = UnpricedModelWarnings::new(2);

        assert!(warnings.should_warn("model-a"));
        assert!(
            !warnings.should_warn("model-a"),
            "the same model warns only once"
        );
        assert!(warnings.should_warn("model-b"));
        assert!(
            !warnings.should_warn("model-c"),
            "a third distinct model must be suppressed once capacity (2) is reached"
        );
        assert!(
            !warnings.should_warn("model-a"),
            "an already-seen model stays suppressed after the cap is hit"
        );
    }
}
