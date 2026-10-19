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
//! unchanged. `generate` prices `response.model` -- the model that actually SERVED the call,
//! which may differ from the requested model after a `FallbackLlmAdapter` hop (D-05); a
//! stream prices the REQUEST's model, since [`StreamingResponse`] carries no served-model
//! string of its own. Both paths share one `price_or_warn` so the pricing/warn-once rule
//! cannot diverge between them.
//!
//! ## Billed errors (G-50-5a)
//!
//! A call the vendor billed but that ended as an `EmptyCompletion` or a `Refusal` carries its
//! usage on the error (`LlmError::billed_usage`). Both paths price that bill as they price a
//! success, through one shared helper: `generate` against the bill's served model, a stream's
//! error item against the request's model. An unpriced model keeps `cost: None` (D-00c).
//!
//! ## Hopped-past bills (G-50-5a remainder (a), plan 50-30)
//!
//! Before it delegates, this decorator installs its pricer on the call's
//! `paladin_ports::output::spend_scope::HOPPED_BILLS` collector (`HoppedBills::install_pricer`).
//! A `FallbackLlmAdapter` inside this decorator records the bill of every billed hop it moves
//! past (a refusal under `hop_on_refusal`) into that collector; the pricer prices the bill
//! against the entry's own model, through the same `price_or_warn`, and the bill is counted into
//! the attempt's spend scope at that hop. So each hop is charged at its own model's rate the
//! moment the chain hops, even if the attempt is cut off while a later hop is in flight. A model
//! with no price row stays `cost: None`. No collector installed: nothing to price.
//!
//! The passes after the inner call are the idempotent safety net: once the inner call returns,
//! `Ok` or `Err`, this decorator prices any entry still at `cost: None` (an entry added with
//! `push`). A stream prices the collector it captured when `generate_stream` was called, once
//! that call returns and again when the stream yields its terminal chunk or an error item
//! (only unpriced entries).

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use paladin_core::platform::container::cost::{Cost, PriceTable};
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ProviderCapabilities,
    StreamingResponse,
};
use paladin_ports::output::spend_scope::{HOPPED_BILLS, HoppedBills};

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

    /// Price `usage` for `model` against `table`, emitting the once-per-model-per-process
    /// `warn!` line (D-08) when unpriced -- never a fabricated zero (D-00c). Shared by
    /// `generate` and the `generate_stream` terminal-chunk mapper so the pricing/warn-once
    /// rule cannot diverge between the two run paths (D-09).
    fn price_or_warn(table: &PriceTable, model: &str, usage: &TokenUsage) -> Option<Cost> {
        match table.price(model, usage) {
            Some(cost) => Some(cost),
            None => {
                if UNPRICED_MODEL_WARNINGS.should_warn(model) {
                    log::warn!(
                        target: PRICING_LOG_TARGET,
                        "no price configured for model {model:?}; its cost is reported as \
                         unknown until a treasurer.pricing entry is added for it"
                    );
                }
                None
            }
        }
    }

    /// Price a billed-but-failed call's error (G-50-5a) exactly as a success is priced.
    ///
    /// When `err` carries a bill ([`LlmError::billed_usage`]) with no cost yet, its usage is
    /// priced through [`Self::price_or_warn`] against `model` (`None`: the bill's own served
    /// model) -- the same warn-once rule and `cost: None` for an unpriced model (D-00c) -- and
    /// the priced bill is set back on the error ([`LlmError::with_billed_usage`]). Any other
    /// error, or a bill that already carries a cost, is returned unchanged. Shared by `generate`
    /// (priced against the bill's served model, as a success is priced against
    /// `response.model`) and the `generate_stream` error items (also priced against the bill's
    /// model; the request model only when the bill names none), so the two run paths cannot
    /// diverge (D-09).
    fn price_billed_error(table: &PriceTable, model: Option<&str>, err: LlmError) -> LlmError {
        let priced = match err.billed_usage() {
            Some(billed) if billed.cost.is_none() => {
                let model = model.unwrap_or(billed.model.as_str());
                Self::price_or_warn(table, model, &billed.usage)
                    .map(|cost| billed.clone().with_cost(cost))
            }
            _ => None,
        };
        match priced {
            Some(billed) => err.with_billed_usage(billed),
            None => err,
        }
    }

    /// Price every hopped-past bill in `bills` still at `cost: None` (G-50-5a remainder (a),
    /// plan 50-30) against the entry's OWN model through [`Self::price_or_warn`] -- never the
    /// served model's rate (PARITY-06) -- with the same warn-once rule and `cost: None` for an
    /// unpriced model (D-00c). Idempotent: an entry already priced is left as it is.
    fn price_hopped_bills(table: &PriceTable, bills: &HoppedBills) {
        bills.price_unpriced(|bill| Self::price_or_warn(table, &bill.model, &bill.usage));
    }

    /// [`Self::price_hopped_bills`] on the active call's [`HOPPED_BILLS`] collector; a no-op
    /// when none is installed. The after-call safety net (A2-50-32): an idempotent pass over
    /// entries the installed pricer did not price, such as ones added with `push`.
    fn price_active_hopped_bills(table: &PriceTable) {
        let _ = HOPPED_BILLS.try_with(|bills| Self::price_hopped_bills(table, bills));
    }

    /// Install this decorator's hop-bill pricer on `bills` BEFORE the call is delegated (WR-03,
    /// plan 50-32). The pricer prices an entry against the entry's OWN model through
    /// [`Self::price_or_warn`] (PARITY-06, D-15): the same warn-once rule, and `None` for an
    /// unpriced model. With it installed, a bill the chain records at its hop is priced and then
    /// counted into the attempt scope at that moment, rather than only after the whole call
    /// returns. The first pricer installed on a collector wins (the outermost decorator's table).
    fn install_hop_pricer(table: &Arc<PriceTable>, bills: &HoppedBills) {
        let table = Arc::clone(table);
        bills.install_pricer(move |bill| Self::price_or_warn(&table, &bill.model, &bill.usage));
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
    /// Prices the RESERVED response's own `model` and `usage` (D-05, D-09) -- the model that
    /// actually SERVED the call, which may differ from the requested model after a
    /// `FallbackLlmAdapter` hop composed inside this decorator
    /// (`Pricing(Fallback(..))`). `None` when no price row exists for that model (D-00c).
    ///
    /// Billed errors are priced like successes; other errors pass through untouched. An
    /// `EmptyCompletion` or `Refusal` the vendor billed ([`LlmError::billed_usage`], G-50-5a)
    /// gets its bill's cost from the bill's own served model, through the same warn-once rule,
    /// so an unpriced model keeps `cost: None`.
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // WR-03, plan 50-32: the pricer goes onto the call's collector BEFORE the chain runs, so
        // every billed hop it moves past is priced against its own model and counted into the
        // attempt scope at the hop, even when a later hop is cut off by the engine.
        let _ = HOPPED_BILLS.try_with(|bills| Self::install_hop_pricer(&self.table, bills));
        let outcome = self.inner.generate(request).await;
        // G-50-5a remainder (a), plan 50-30: the idempotent safety net -- by now the chain
        // inside has recorded every billed hop it moved past, whichever way the call ended.
        Self::price_active_hopped_bills(&self.table);
        let mut response =
            outcome.map_err(|err| Self::price_billed_error(&self.table, None, err))?;
        response.cost = Self::price_or_warn(&self.table, &response.model, &response.usage);
        Ok(response)
    }

    /// Prices the terminal chunk of the delegated stream from ITS OWN usage against the
    /// request's model (D-09, Phase 31 contract -- `StreamingResponse` carries no served-model
    /// string of its own): a chunk whose `finish_reason.is_some()` and whose `usage` is `Some`
    /// gets [`StreamingResponse::with_cost`] when `price_or_warn` answers `Some`; when
    /// it answers `None`, the model is unpriced and gets exactly one `warn!` line per distinct
    /// model name per process (D-08) -- never a fabricated zero. A terminal chunk with no usage
    /// at all gets no cost and no pricing warning (the execution service already warns about
    /// the missing usage, Phase 31 D-17). Every non-terminal chunk passes through unchanged.
    ///
    /// An `Err` item that carries a bill ([`LlmError::billed_usage`], G-50-5a: a stream that
    /// ended empty or refused) is priced against the BILL's own model -- the hop that billed it,
    /// which a billed error names unlike an `Ok` terminal chunk -- through the helper `generate`
    /// uses for billed errors (the request's model only when the bill names none); an unpriced
    /// model keeps `cost: None` and warns once. Any other `Err` item passes through unchanged.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError> {
        let model = request.model.clone();
        let table = Arc::clone(&self.table);
        // G-50-5a remainder (a), plan 50-30: the call's collector, captured here because the
        // stream is polled later, possibly from another task.
        let hopped = HOPPED_BILLS.try_with(Clone::clone).ok();
        // WR-03, plan 50-32: install the hop pricer before the chain opens (see `generate`).
        if let Some(bills) = &hopped {
            Self::install_hop_pricer(&table, bills);
        }
        let opened = self.inner.generate_stream(request).await;
        if let Some(bills) = &hopped {
            Self::price_hopped_bills(&table, bills);
        }
        // A billed error the chain returns before any chunk (`AllProvidersFailed` after billed
        // refusals) is priced against its bill's own model -- the hop that answered last, not
        // the request's model.
        let raw = opened.map_err(|err| Self::price_billed_error(&table, None, err))?;
        let priced = Box::into_pin(raw).map(move |item| match item {
            Ok(chunk) => {
                if chunk.finish_reason.is_none() {
                    return Ok(chunk);
                }
                if let Some(bills) = &hopped {
                    Self::price_hopped_bills(&table, bills);
                }
                let Some(usage) = chunk.usage.clone() else {
                    return Ok(chunk);
                };
                Ok(match Self::price_or_warn(&table, &model, &usage) {
                    Some(cost) => chunk.with_cost(cost),
                    None => chunk,
                })
            }
            // G-50-5a / WR-01: a billed error item names the hop that billed it
            // (`BilledUsage::model`), unlike an `Ok` terminal chunk, so it is priced against that
            // model through the helper `generate` uses (D-09) -- after a fallback hop rewrote the
            // request model the request's row would be the wrong rate. Only a bill that names no
            // model falls back to the request's model.
            Err(err) => {
                if let Some(bills) = &hopped {
                    Self::price_hopped_bills(&table, bills);
                }
                let unnamed = err
                    .billed_usage()
                    .filter(|billed| billed.model.is_empty())
                    .map(|_| model.as_str());
                Err(Self::price_billed_error(&table, unnamed, err))
            }
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

    /// Unchanged -- delegates to `inner`, so the per-model parameter policy (ADR-0061) is not
    /// silently disabled by this decorator.
    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        self.inner.model_profile(model)
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::fallback::FallbackLlmAdapter;
    use crate::mock::MockLlmAdapter;
    use chrono::Utc;
    use paladin_core::platform::container::cost::{CurrencyCode, PriceRow};
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_core::platform::container::token_usage::TokenUsage;
    use paladin_ports::output::llm_port::FinishReason;
    use std::collections::HashMap;
    use uuid::Uuid;

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

    /// A stub `LlmPort` that always serves a FIXED model name and usage, ignoring the
    /// request's own model entirely -- `MockLlmAdapter` echoes `request.model`, so it cannot
    /// express a fallback hop that ends up served by a DIFFERENTLY-named model (D-05).
    struct NamedModelPort {
        model: &'static str,
        usage: TokenUsage,
    }

    #[async_trait]
    impl LlmPort for NamedModelPort {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                id: Uuid::new_v4(),
                request_id: request.id,
                model: self.model.to_string(),
                content: "hi from the backup".to_string(),
                finish_reason: FinishReason::Stop,
                usage: self.usage.clone(),
                cost: None,
                created_at: Utc::now(),
                metadata: HashMap::new(),
                function_call: None,
                reasoning: None,
            })
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
        {
            Err(LlmError::ProcessingError(
                "NamedModelPort does not support streaming".to_string(),
            ))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![self.model.to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            "named-model-stub"
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    /// D-09: `generate` prices the RESPONSE's own served model, not the request's.
    #[tokio::test]
    async fn generate_prices_the_response_model() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let response = adapter.generate(request("gpt-4")).await.unwrap();
        let cost = response.cost.expect("priced model must carry a cost");
        assert_eq!(cost.nanos(), 22_500_000);
        assert_eq!(cost.currency().as_str(), "USD");
    }

    /// D-00c/D-08: an unpriced model's non-streaming call gets `cost: None`, never a
    /// fabricated zero; a fresh `UnpricedModelWarnings` instance still warns exactly once per
    /// distinct model name (the same dedup rule the stream path already proves).
    #[tokio::test]
    async fn generate_unpriced_model_reports_none() {
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_response("hi")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let adapter = PricingLlmAdapter::new(mock, usd_table());

        let response = adapter
            .generate(request("p38-unpriced-generate"))
            .await
            .unwrap();
        assert!(
            response.cost.is_none(),
            "an unpriced model must never carry a cost"
        );

        let warnings = UnpricedModelWarnings::new(10);
        assert!(warnings.should_warn("p38-unpriced-generate"));
        assert!(
            !warnings.should_warn("p38-unpriced-generate"),
            "the same unpriced model warns only once"
        );
    }

    /// D-09/RESEARCH Open Question 2: composed as `Pricing(Fallback(primary, backup))`, a
    /// call that hops from a failing primary to a backup serving a DIFFERENTLY-named model is
    /// priced against the backup's SERVED model -- or `None` when that model has no row --
    /// never against the originally requested model.
    #[tokio::test]
    async fn prices_the_served_model_after_fallback_hop() {
        let primary: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_error(LlmError::rate_limited(None)));
        let backup: Arc<dyn LlmPort> = Arc::new(NamedModelPort {
            model: "backup-model",
            usage: TokenUsage::new(100, 0),
        });
        let fallback =
            Arc::new(FallbackLlmAdapter::new(vec![primary, backup]).expect("non-empty chain"));

        let priced_table = Arc::new(
            PriceTable::new(CurrencyCode::new("USD").unwrap())
                .with_row("backup-model", PriceRow::new(1_000_000_000, 0).unwrap()),
        );
        let priced_adapter = PricingLlmAdapter::new(fallback.clone(), priced_table);
        let response = priced_adapter
            .generate(request("originally-requested-model"))
            .await
            .unwrap();
        assert_eq!(response.model, "backup-model");
        let cost = response
            .cost
            .expect("the served model's row must price the call");
        assert_eq!(cost.nanos(), 100_000);
        assert_eq!(cost.currency().as_str(), "USD");

        let unpriced_table = Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()));
        let unpriced_adapter = PricingLlmAdapter::new(fallback, unpriced_table);
        let response = unpriced_adapter
            .generate(request("originally-requested-model"))
            .await
            .unwrap();
        assert_eq!(response.model, "backup-model");
        assert!(
            response.cost.is_none(),
            "a served model with no price row must never carry a cost"
        );
    }

    /// The Kimi served model the billed tracer prices, and its row: 2.5 USD per million input
    /// tokens, 10 USD per million output tokens.
    const BILLED_SERVED_MODEL: &str = "kimi-served-test";

    fn billed_table() -> Arc<PriceTable> {
        Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            BILLED_SERVED_MODEL,
            PriceRow::new(2_500_000_000, 10_000_000_000).unwrap(),
        ))
    }

    /// G-50-5a tracer: a Kimi (compat) call that the vendor billed but answered with no text --
    /// `finish_reason: length`, usage 1_000 / 62_000 / 63_000 and no
    /// `completion_tokens_details`, so the figures hold under either usage semantics -- comes
    /// back through `PricingLlmAdapter::generate` as `Err(EmptyCompletion)` whose bill carries
    /// the Treasurer price of the served model's row, exactly as a success would be priced.
    #[cfg(feature = "kimi")]
    #[tokio::test]
    async fn priced_billed_empty_completion_carries_its_cost() {
        use crate::kimi::adapter::KIMI_DEFAULT_MODEL;
        use crate::kimi::{KimiAdapter, KimiConfig};
        use serde_json::json;

        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "cmpl-1",
                    "model": BILLED_SERVED_MODEL,
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": ""},
                        "finish_reason": "length"
                    }],
                    "usage": {
                        "prompt_tokens": 1_000,
                        "completion_tokens": 62_000,
                        "total_tokens": 63_000
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;
        let kimi: Arc<dyn LlmPort> = Arc::new(
            KimiAdapter::new(KimiConfig::new(
                "test-key".to_string(),
                server.url(),
                KIMI_DEFAULT_MODEL.to_string(),
            ))
            .unwrap(),
        );

        let table = billed_table();
        let adapter = PricingLlmAdapter::new(Arc::clone(&kimi), Arc::clone(&table));
        let err = adapter
            .generate(request(KIMI_DEFAULT_MODEL))
            .await
            .expect_err("an empty length completion is an error");
        assert!(
            matches!(err, LlmError::EmptyCompletion { .. }),
            "expected EmptyCompletion, got {err:?}"
        );
        let billed = err
            .billed_usage()
            .expect("a billed empty completion carries its usage");
        assert_eq!(billed.model, BILLED_SERVED_MODEL);
        assert_eq!(billed.usage.total_tokens, 63_000);
        let expected = table
            .price(BILLED_SERVED_MODEL, &billed.usage)
            .expect("the served model has a row");
        // 1_000 * 2.5 USD/M + 62_000 * 10 USD/M = 0.0025 + 0.62 USD.
        assert_eq!(expected.nanos(), 622_500_000);
        assert_eq!(billed.cost.as_ref(), Some(&expected));

        // An unpriced served model: the same error and bill, with `cost: None` (D-00c).
        let unpriced = PricingLlmAdapter::new(
            kimi,
            Arc::new(
                PriceTable::new(CurrencyCode::new("USD").unwrap())
                    .with_row("some-other-model", PriceRow::new(1, 1).unwrap()),
            ),
        );
        let err = unpriced
            .generate(request(KIMI_DEFAULT_MODEL))
            .await
            .expect_err("an empty length completion is an error");
        let billed = err
            .billed_usage()
            .expect("an unpriced bill still carries its usage");
        assert_eq!(billed.usage.total_tokens, 63_000);
        assert_eq!(
            billed.cost, None,
            "an unpriced model must never carry a cost"
        );
    }

    /// G-50-5a: an error the vendor did not bill passes through `generate` unchanged, and a
    /// billed error on an unpriced model keeps `cost: None`.
    #[tokio::test]
    async fn unbilled_and_unpriced_errors_pass_through_generate() {
        let mock = Arc::new(MockLlmAdapter::new().with_error(LlmError::empty_completion("x")));
        let adapter = PricingLlmAdapter::new(mock, billed_table());
        let err = adapter.generate(request("gpt-4")).await.unwrap_err();
        assert!(
            matches!(&err, LlmError::EmptyCompletion { detail, .. } if detail == "x"),
            "got {err:?}"
        );
        assert_eq!(err.billed_usage(), None, "nothing billed, nothing priced");

        let mock = Arc::new(MockLlmAdapter::new().with_error(LlmError::rate_limited(None)));
        let adapter = PricingLlmAdapter::new(mock, billed_table());
        let err = adapter.generate(request("gpt-4")).await.unwrap_err();
        assert!(
            matches!(err, LlmError::RateLimitExceeded { .. }),
            "got {err:?}"
        );

        let bill = paladin_ports::output::llm_port::BilledUsage::new(
            "p50-27-unpriced-billed",
            TokenUsage::new(10, 20),
        );
        let billed = LlmError::empty_completion("x").with_billed_usage(bill.clone());
        let mock = Arc::new(MockLlmAdapter::new().with_error(billed));
        let adapter = PricingLlmAdapter::new(mock, billed_table());
        let err = adapter.generate(request("gpt-4")).await.unwrap_err();
        assert_eq!(
            err.billed_usage(),
            Some(&bill),
            "an unpriced bill is returned exactly as it came"
        );
    }

    /// G-50-5a / D-09 / WR-01: a stream's billed `Err` item is priced against the BILL's own
    /// model (the hop that billed it, as `generate` prices it), not the request's model, so a
    /// fallback-served stream is not mispriced; an empty bill model falls back to the request's
    /// model; an unpriced bill model leaves `cost: None`; deltas already yielded and unbilled
    /// errors are untouched.
    #[tokio::test]
    async fn priced_billed_stream_error_carries_its_cost() {
        use paladin_ports::output::llm_port::BilledUsage;

        let billed = |model: &str| {
            LlmError::empty_completion("finish_reason=stop with empty content")
                .with_billed_usage(BilledUsage::new(model, TokenUsage::new(1_000, 2_000)))
        };
        let items = |adapter: PricingLlmAdapter, model: &'static str| async move {
            let stream = adapter.generate_stream(request(model)).await.unwrap();
            Box::into_pin(stream).collect::<Vec<_>>().await
        };

        // The bill names `gpt-4`; the request names a model with no row. The bill's own model
        // is what is priced.
        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_stream_items(vec![Ok("partial".to_string()), Err(billed("gpt-4"))]),
        );
        let got = items(
            PricingLlmAdapter::new(mock, usd_table()),
            "p50-request-model-not-in-table",
        )
        .await;
        assert_eq!(got.len(), 2);
        let first = got[0].as_ref().expect("the delta is yielded unchanged");
        assert_eq!(first.delta, "partial");
        assert!(first.cost.is_none());
        let err = got[1].as_ref().expect_err("the terminal item is the error");
        let bill = err.billed_usage().expect("the bill survives pricing");
        assert_eq!(bill.usage.total_tokens, 3_000);
        let cost = bill
            .cost
            .as_ref()
            .expect("priced against the bill model's row");
        assert_eq!(cost.nanos(), 22_500_000);
        assert_eq!(cost.currency().as_str(), "USD");

        // The request's model has a row, the bill's model does not: the bill stays unpriced
        // rather than being charged at the request model's rate (WR-01).
        let mock =
            Arc::new(MockLlmAdapter::new().with_stream_items(vec![Err(billed("bill-model-x"))]));
        let got = items(PricingLlmAdapter::new(mock, usd_table()), "gpt-4").await;
        let err = got[0].as_ref().expect_err("the item is the error");
        assert_eq!(
            err.billed_usage().map(|bill| bill.cost.clone()),
            Some(None),
            "an unpriced bill model must never carry a cost"
        );

        // A bill that names no model is priced at the request's model.
        let mock = Arc::new(MockLlmAdapter::new().with_stream_items(vec![Err(billed(""))]));
        let got = items(PricingLlmAdapter::new(mock, usd_table()), "gpt-4").await;
        let err = got[0].as_ref().expect_err("the item is the error");
        assert_eq!(
            err.billed_usage()
                .and_then(|bill| bill.cost.as_ref())
                .map(Cost::nanos),
            Some(22_500_000),
            "an empty bill model falls back to the request model"
        );

        let mock = Arc::new(
            MockLlmAdapter::new()
                .with_stream_items(vec![Err(LlmError::NetworkError("reset".to_string()))]),
        );
        let got = items(PricingLlmAdapter::new(mock, usd_table()), "gpt-4").await;
        assert!(
            matches!(&got[0], Err(LlmError::NetworkError(msg)) if msg == "reset"),
            "an unbilled error passes through unchanged, got {:?}",
            got[0]
        );
    }

    /// D-09: identity methods are unchanged pass-throughs to the inner port.
    #[tokio::test]
    async fn pricing_is_transparent() {
        let mock = Arc::new(MockLlmAdapter::new());
        let adapter = PricingLlmAdapter::new(Arc::clone(&mock) as Arc<dyn LlmPort>, usd_table());

        assert_eq!(adapter.get_provider_name(), mock.get_provider_name());
        assert_eq!(
            adapter.get_capabilities().supports_streaming,
            mock.get_capabilities().supports_streaming
        );
        assert_eq!(
            adapter.validate_model("gpt-4").await.unwrap(),
            mock.validate_model("gpt-4").await.unwrap()
        );
        assert_eq!(
            adapter.get_available_models().await.unwrap(),
            mock.get_available_models().await.unwrap()
        );
    }

    use crate::fallback::test_support::ProfiledStubPort;
    use paladin_ports::output::llm_port::ParamSupport;

    fn sample_profile() -> ModelProfile {
        ModelProfile::new()
            .with_temperature(ParamSupport::Fixed(1.0))
            .with_context_window(128_000)
            .with_verified_on("2026-10-15")
    }

    /// ADR-0061 / Pitfall 4: the pricing decorator forwards `model_profile` to its inner port and
    /// passes the model string through unchanged.
    #[test]
    fn pricing_forwards_model_profile() {
        let stub = Arc::new(ProfiledStubPort::new("stub", "m", Some(sample_profile())));
        let adapter = PricingLlmAdapter::new(stub.clone(), usd_table());

        assert_eq!(adapter.model_profile("m"), Some(sample_profile()));
        assert_eq!(adapter.model_profile("other"), None);
        assert_eq!(stub.asked(), vec!["m".to_string(), "other".to_string()]);
    }

    /// ADR-0061 / Pitfall 4: the production composition `Pricing(Cadence(provider))` returns the
    /// provider's own profile, so the policy cannot be silently disabled by the decorators.
    #[test]
    fn pricing_over_cadence_forwards_model_profile() {
        use crate::cadence::{CadenceSettings, CadenceWiring, with_cadence};
        use paladin_ports::output::cadence_port::CadencePolicy;
        use paladin_storage::cadence::InMemoryCadence;

        let stub: Arc<dyn LlmPort> =
            Arc::new(ProfiledStubPort::new("stub", "m", Some(sample_profile())));
        let wiring = CadenceWiring::new(
            Arc::new(InMemoryCadence::new(CadencePolicy::default())),
            CadenceSettings::default(),
        );
        let cadence = with_cadence(stub, Some(&wiring));
        let composed = with_pricing(cadence, &usd_table());

        assert_eq!(composed.model_profile("m"), Some(sample_profile()));
        assert_eq!(composed.model_profile("unknown"), None);
    }

    // ---- G-50-5a remainder (a), plan 50-30 ----

    /// `m1` and `m2` priced at different rates, so a hop priced at the served model's rate
    /// would show.
    fn two_model_table() -> Arc<PriceTable> {
        Arc::new(
            PriceTable::new(CurrencyCode::new("USD").unwrap())
                .with_row("m1", PriceRow::new(3_000_000_000, 15_000_000_000).unwrap())
                .with_row("m2", PriceRow::new(2_500_000_000, 10_000_000_000).unwrap()),
        )
    }

    /// `Pricing(Fallback[m1 refuses after billing, m2 answers].hop_on_refusal(true))`.
    fn hop_chain(second: MockLlmAdapter, table: Arc<PriceTable>) -> PricingLlmAdapter {
        use crate::fallback::FallbackStep;
        use paladin_ports::output::llm_port::BilledUsage;

        let first = MockLlmAdapter::new().with_provider_name("hop1").with_error(
            LlmError::refusal("hop1", "m1", "I can't help with that.")
                .with_billed_usage(BilledUsage::new("m1", TokenUsage::new(1_000, 40))),
        );
        let chain = FallbackLlmAdapter::from_steps(vec![
            FallbackStep::new(Arc::new(first) as Arc<dyn LlmPort>, "m1"),
            FallbackStep::new(Arc::new(second) as Arc<dyn LlmPort>, "m2"),
        ])
        .unwrap()
        .hop_on_refusal(true);
        PricingLlmAdapter::new(Arc::new(chain), table)
    }

    /// Each hopped-past bill is priced against its OWN model (`m1`), not the served model
    /// (`m2`), on `generate` and on `generate_stream`; an unpriced model keeps `cost: None`.
    #[tokio::test]
    async fn hopped_bills_are_priced_against_their_own_model() {
        use paladin_ports::output::spend_scope::{HOPPED_BILLS, HoppedBills};

        let table = two_model_table();
        let own = table.price("m1", &TokenUsage::new(1_000, 40)).unwrap();
        let at_served_rate = table.price("m2", &TokenUsage::new(1_000, 40)).unwrap();
        assert_ne!(own, at_served_rate, "the two rates differ");

        let answer = || {
            MockLlmAdapter::new()
                .with_provider_name("hop2")
                .with_response("served")
                .with_token_usage_struct(TokenUsage::new(10, 20))
        };

        let bills = HoppedBills::new();
        let adapter = hop_chain(answer(), Arc::clone(&table));
        let response = HOPPED_BILLS
            .scope(bills.clone(), adapter.generate(request("m2")))
            .await
            .unwrap();
        assert_eq!(response.model, "m2");
        let taken = bills.take();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].model, "m1");
        assert_eq!(
            taken[0].cost,
            Some(own.clone()),
            "generate: priced at m1's rate"
        );

        let bills = HoppedBills::new();
        let adapter = hop_chain(answer(), Arc::clone(&table));
        let stream = HOPPED_BILLS
            .scope(bills.clone(), adapter.generate_stream(request("m2")))
            .await
            .unwrap();
        let items: Vec<_> = Box::into_pin(stream).collect().await;
        assert!(items.iter().all(Result::is_ok));
        let taken = bills.take();
        assert_eq!(taken.len(), 1);
        assert_eq!(
            taken[0].cost,
            Some(own),
            "generate_stream: priced at m1's rate"
        );

        // An unpriced hop model keeps `cost: None` (D-00c).
        let only_m2 = Arc::new(
            PriceTable::new(CurrencyCode::new("USD").unwrap())
                .with_row("m2", PriceRow::new(2_500_000_000, 10_000_000_000).unwrap()),
        );
        let bills = HoppedBills::new();
        let adapter = hop_chain(answer(), only_m2);
        HOPPED_BILLS
            .scope(bills.clone(), adapter.generate(request("m2")))
            .await
            .unwrap();
        let taken = bills.take();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].cost, None, "m1 is unpriced here");
    }

    /// WR-03 (plan 50-32): a hopped-past bill is priced against its own model and counted into
    /// the active attempt scope AT THE HOP. `generate`: hop 2 never answers, the attempt is cut
    /// by a deadline, and the scope already holds hop 1's bill priced at `m1`'s rate.
    /// `generate_stream`: the bill reaches the scope priced, not unpriced.
    #[tokio::test(start_paused = true)]
    async fn hop_bills_are_priced_and_counted_at_the_hop() {
        use paladin_ports::output::spend_scope::{ATTEMPT_SPEND, SpendScope};
        use std::time::Duration;

        let table = two_model_table();
        let u1 = TokenUsage::new(1_000, 40);
        let c1 = table.price("m1", &u1).unwrap();

        let stalled = MockLlmAdapter::new()
            .with_provider_name("hop2")
            .with_response("never served")
            .with_delay(Duration::from_secs(3_600));
        let adapter = hop_chain(stalled, Arc::clone(&table));
        let scope = SpendScope::new();
        let cut = tokio::time::timeout(
            Duration::from_millis(500),
            ATTEMPT_SPEND.scope(
                scope.clone(),
                HOPPED_BILLS.scope(HoppedBills::new(), adapter.generate(request("m2"))),
            ),
        )
        .await;
        assert!(cut.is_err(), "the attempt was dropped mid-call");
        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 1, "hop 1's bill is already counted");
        assert_eq!(spend.usage(), &u1);
        assert_eq!(spend.cost(), Some(c1.clone()), "priced at m1's own rate");

        let answer = MockLlmAdapter::new()
            .with_provider_name("hop2")
            .with_response("served")
            .with_token_usage_struct(TokenUsage::new(10, 20));
        let adapter = hop_chain(answer, Arc::clone(&table));
        let scope = SpendScope::new();
        let stream = ATTEMPT_SPEND
            .scope(
                scope.clone(),
                HOPPED_BILLS.scope(HoppedBills::new(), adapter.generate_stream(request("m2"))),
            )
            .await
            .unwrap();
        let items: Vec<_> = Box::into_pin(stream).collect().await;
        assert!(items.iter().all(Result::is_ok));
        let spend = scope.snapshot();
        assert_eq!(spend.calls(), 1, "the hop bill was counted when recorded");
        assert_eq!(spend.usage(), &u1);
        assert_eq!(
            spend.cost(),
            Some(c1),
            "and it was priced when it was recorded"
        );
    }
}
