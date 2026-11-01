//! # Model fallback — `FallbackLlmAdapter` (Doc 04 FT-FR-16/17, D-24…D-26)
//!
//! Composes an ordered chain of plain [`LlmPort`](paladin_ports::output::llm_port::LlmPort) implementors into one
//! [`LlmPort`](paladin_ports::output::llm_port::LlmPort): every call starts at chain element 0 and moves to the next
//! element only when the current one fails with an error whose
//! [`LlmError::transience`](paladin_ports::output::llm_port::LlmError::transience) is `Transient` or `Unknown`. A `Permanent`
//! error short-circuits, so a malformed or rejected prompt is never re-sent
//! to every provider in the chain (T-25-35).
//!
//! The hop rule has exactly one exception, and only when pacing is attached: a
//! provider `429` is paced on the same provider before it can hop. See
//! "Pace first, hop last" below.
//!
//! ## Pace first, hop last (PACE-02, D-03)
//!
//! A silent mid-run switch to a different provider on a rate limit breaks benchmark
//! consistency, tool-call formats, token accounting and the provider's prompt cache, so a chain
//! built with [`FallbackLlmAdapter::with_cadence`](crate::fallback::FallbackLlmAdapter::with_cadence) treats a `429` differently from every other
//! transient error. On a [`LlmError::RateLimitExceeded`](paladin_ports::output::llm_port::LlmError::RateLimitExceeded) from a hop, the chain reads that hop's
//! gate from the shared [`CadencePort`](paladin_ports::output::cadence_port::CadencePort) and, while the time already spent pacing that hop in
//! this call plus the next gate still fits the pace budget
//! ([`CadenceSettings::fallback_pace_budget`](crate::cadence::CadenceSettings::fallback_pace_budget),
//! 60 s by default), retries the SAME provider: its own [`CadenceLlmAdapter`](crate::cadence::CadenceLlmAdapter)
//! sleeps the gate (the provider's own `Retry-After` is the minimum, with back-off and jitter
//! when it gave none) and sends again. Only when the next retry would exceed the budget does the
//! `429` reach the ordinary hop rule below. A budget of exactly the elapsed time plus the next
//! gate still paces; one millisecond more hops.
//!
//! Everything else keeps the old rule: a 5xx, a timeout or a network error hops at once, a
//! `Permanent` error short-circuits, and a gate that cannot be read, is already clear, or is
//! longer than `max_wait` (the decorator refuses those without sleeping) is not paced -- with no
//! wait to honour there is nothing to pace against, and retrying would only thrash. A chain
//! without [`FallbackLlmAdapter::with_cadence`](crate::fallback::FallbackLlmAdapter::with_cadence) has no gate to read and hops on a `429` exactly
//! as before. The decorator still never retries (D-01); it is this chain, which already owns
//! hops, that re-enters a hop, and only for a `429` with pacing attached.
//!
//! Paced retries are attempts: each one is listed in [`LlmError::AllProvidersFailed`](paladin_ports::output::llm_port::LlmError::AllProvidersFailed)'s
//! `attempts` in order and logged at `debug` under
//! [`CADENCE_LOG_TARGET`](paladin_ports::output::cadence_port::CADENCE_LOG_TARGET) (provider, model and durations only). A
//! [`TraceEvent::FallbackHop`](paladin_core::platform::container::trace::TraceEvent::FallbackHop) is emitted only when a hop really happens.
//!
//! ## One model per hop (D-15, PARITY-06, handoff B-3)
//!
//! [`FallbackLlmAdapter::new`](crate::fallback::FallbackLlmAdapter::new) sends every hop the request's own model, which is only right when
//! every hop serves that model. A chain across vendors (Anthropic, then Kimi) must not send
//! `claude-*` to Moonshot, so [`FallbackLlmAdapter::from_steps`](crate::fallback::FallbackLlmAdapter::from_steps) takes [`FallbackStep`](crate::fallback::FallbackStep)s, each a
//! port and the model it is sent. Before each hop the request's `model` is rewritten to the
//! step's, on `generate` and `generate_stream`; the hop's own default model is never
//! substituted, and the first hop's model is never reused on a later hop. That model also keys
//! the pacing (Cadence keys on `(provider, model)`), so a 429 on hop 1 gates hop 1's model only.
//! Every served response records the hop's provider under [`SERVED_BY_METADATA_KEY`](crate::fallback::SERVED_BY_METADATA_KEY) and the
//! model it was sent under [`SERVED_MODEL_METADATA_KEY`](crate::fallback::SERVED_MODEL_METADATA_KEY), for both constructors. A streamed
//! response has no metadata to carry them; the stream's chunks are the hop's own.
//!
//! ## A refusal stops the chain by default (D-10)
//!
//! [`LlmError::Refusal`](paladin_ports::output::llm_port::LlmError::Refusal) is `Permanent`, so it short-circuits like any other: an automatic hop
//! would hide the refusal from the consumer and spend budget on every hop.
//! [`FallbackLlmAdapter::hop_on_refusal`](crate::fallback::FallbackLlmAdapter::hop_on_refusal) is the explicit opt-in to trying the next step on a
//! refusal; it never widens to any other `Permanent` error.
//!
//! ## A hopped-past bill is carried, not dropped (G-50-5a remainder (a))
//!
//! A refusal the vendor billed carries its usage ([`LlmError::billed_usage`](paladin_ports::output::llm_port::LlmError::billed_usage)). When the chain
//! hops past such an error -- and only then -- it records the bill through
//! [`record_hopped_bill`](paladin_ports::output::spend_scope::record_hopped_bill) into the call's `HOPPED_BILLS` collector
//! (`paladin_ports::output::spend_scope`), which `PaladinExecutionService` installs around every
//! model call. The bill is recorded at the one hop decision point and priced there, against the
//! hop's own model (D-15), by the pricer the `PricingLlmAdapter` outside the chain installed. It
//! is counted into the attempt's `ATTEMPT_SPEND` scope at once, so it is charged even when the
//! attempt is cut off while a later hop is in flight; the execution service then settles it in
//! the call's ledger row. An exhausted chain's last hop is not recorded:
//! [`LlmError::AllProvidersFailed`](paladin_ports::output::llm_port::LlmError::AllProvidersFailed) carries that bill itself. A billed hop with no collector
//! installed (a direct [`LlmPort`](paladin_ports::output::llm_port::LlmPort) caller) is not silently lost: the chain logs one `warn!` per
//! (provider, model) under the `paladin::usage` log target, with token counts only. The chain's
//! hop rules do not change: `hop_on_refusal` stays opt-in (D-10).
//!
//! ## Composition
//!
//! `Pricing(Fallback(Cadence(hop1), Cadence(hop2)))`: pricing stays outermost so it prices the
//! response that was actually served (ADR-0052), and each hop is gated under its own provider
//! name, never under `"fallback"`. Pricing also prices every billed hop the chain moved past and
//! recorded for settlement, each against its own model (G-50-5a).
//!
//! ## No trait change, no policy type (D-24, RT-FR-09)
//!
//! The adapter adds nothing to [`LlmPort`](paladin_ports::output::llm_port::LlmPort): provider names come from the
//! existing [`LlmPort::get_provider_name`](paladin_ports::output::llm_port::LlmPort::get_provider_name), the adapter itself reports
//! `"fallback"`, [`LlmPort::get_capabilities`](paladin_ports::output::llm_port::LlmPort::get_capabilities) answers with the first
//! element's, and [`LlmPort::validate_model`](paladin_ports::output::llm_port::LlmPort::validate_model) / [`LlmPort::get_available_models`](paladin_ports::output::llm_port::LlmPort::get_available_models)
//! answer with the first element that returns `Ok`. It is a plain port so
//! Phase 26's retry/fallback middleware can delegate to it unchanged.
//!
//! ## The streaming first-chunk rule (D-25, T-25-33)
//!
//! [`LlmPort::generate_stream`](paladin_ports::output::llm_port::LlmPort::generate_stream) falls through only when the call itself
//! returns `Err`, or when the stream's **first** item is `Err` — the adapter
//! peeks that first item before handing the stream to the caller and still
//! delivers it. After any `Ok` chunk has been observed an error propagates
//! unchanged: a partial answer is never silently completed by a different
//! model.
//!
//! ## Per-hop observability (D-25, T-25-34, 28-06 D-03)
//!
//! Each hop emits [`TraceEvent::FallbackHop`](paladin_core::platform::container::trace::TraceEvent::FallbackHop) (`node_id: None` — a port
//! below the superstep engine cannot know which node it serves) through
//! whichever [`TraceEmitter`](paladin_ports::output::trace_sink_port::TraceEmitter) handle is available, plus a `log::warn!`
//! naming both providers. Two sources, checked in order:
//!
//! 1. An explicit handle set via [`FallbackLlmAdapter::with_trace_emitter`](crate::fallback::FallbackLlmAdapter::with_trace_emitter)
//!    — takes precedence, for a caller (typically a unit test, or any
//!    standalone construction) that has no ambient per-run context.
//! 2. `paladin_ports::output::trace_sink_port::current_trace_emitter` — the
//!    run's own [`TraceEmitter`](paladin_ports::output::trace_sink_port::TraceEmitter), set for the duration of one engine
//!    dispatch by the worker's own composition root
//!    (`RUN_TRACE_EMITTER.scope`). This is how a REAL run reaches this
//!    adapter: `FallbackLlmAdapter` is composed once, at boot, into a
//!    deeply shared `Arc<dyn PaladinPort>` singleton every concurrent run
//!    calls through, so there is no per-run FIELD to set — the task-local
//!    is what lets one hop stamp into THIS run's own `seq` sequence without
//!    racing a different concurrent run sharing the same singleton.
//!
//! With neither source available, a hop still logs its `warn!` line but
//! emits no `TraceEvent` — exactly like today's "sink not attached" case
//! (X-03).
//!
//! A successful response is stamped with the serving provider under
//! [`SERVED_BY_METADATA_KEY`](crate::fallback::SERVED_BY_METADATA_KEY) in the existing `LlmResponse.metadata` map
//! (D-26); `PaladinExecutionService` copies it into
//! `PaladinResult.served_by`.
//!
//! ## Circuit breakers sit ABOVE this adapter (D-24)
//!
//! The facade's `CircuitBreaker` (`src/infrastructure/resilience/circuit_breaker.rs`)
//! yields `PaladinError::CircuitBreakerOpen` *above* the port boundary and is
//! invisible to this adapter, so PRD 04's "or an open circuit breaker" clause
//! is satisfied by composition, not by a new variant: a breaker wrapping an
//! individual [`LlmPort`](paladin_ports::output::llm_port::LlmPort) must surface an [`LlmError`](paladin_ports::output::llm_port::LlmError) the chain classifies
//! `Transient` (a [`LlmError::NetworkError`](paladin_ports::output::llm_port::LlmError::NetworkError) or a 503
//! [`LlmError::ProviderError`](paladin_ports::output::llm_port::LlmError::ProviderError), for instance) for the chain to hop past it.
//!
//! ## No cross-call state of its own (T-25-36)
//!
//! The adapter holds only its immutable chain, sink and pacing handle. Nothing in the adapter
//! records which provider served the previous call, so one call's hop never changes where the
//! next call — or a concurrent one — starts. Pacing state lives behind the shared
//! [`CadencePort`](paladin_ports::output::cadence_port::CadencePort) its hops are wrapped with, keyed by provider and model, and is never read to
//! choose a starting provider.
//!
//! Not feature-gated (ADR-0046): it composes whichever provider adapters are
//! compiled in and needs none of them itself.

use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, Stream, StreamExt};
use paladin_core::platform::container::reasoning::{ReasoningEffort, ReasoningFallback};
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::cadence_port::{CADENCE_LOG_TARGET, CadenceKey, CadencePort};
use paladin_ports::output::llm_port::{
    BilledUsage, LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ProviderCapabilities,
    StreamingResponse,
};
use paladin_ports::output::model_profile::EffortSupport;
use paladin_ports::output::spend_scope::record_hopped_bill;
use paladin_ports::output::trace_sink_port::{TraceEmitter, TraceEvent, current_trace_emitter};
use thiserror::Error;
use tokio::time::Instant;

use crate::cadence::{CadenceWiring, with_cadence as wrap_with_cadence};

/// The `LlmResponse.metadata` key under which the adapter records the
/// `get_provider_name()` of the provider that actually served a response
/// (D-26). A plain single-provider adapter never sets it.
pub const SERVED_BY_METADATA_KEY: &str = "paladin.served_by";

/// The `LlmResponse.metadata` key under which the chain records the model the serving hop was
/// sent (PARITY-06, D-15): the step's model for a [`FallbackLlmAdapter::from_steps`] chain, the
/// request's model for a [`FallbackLlmAdapter::new`] chain.
///
/// It is the model the hop was *sent*, not the vendor-echoed `LlmResponse.model`, which may be a
/// dated alias of it. Both are visible to the caller.
pub const SERVED_MODEL_METADATA_KEY: &str = "paladin.served_model";

/// The provider name the adapter itself reports from
/// [`LlmPort::get_provider_name`].
pub const FALLBACK_PROVIDER_NAME: &str = "fallback";

/// The log target of the one-per-(provider, model) warning a billed hop with no hopped-bill
/// collector installed produces (G-50-5a remainder (a)): the usage-accounting target the usage
/// reconciliation warnings already use.
const HOP_BILL_LOG_TARGET: &str = "paladin::usage";

/// How many distinct (provider, model) pairs this process warns about for an uncollected hop
/// bill before it stops, bounding memory against an attacker-chosen model string (the
/// `UNPRICED_MODEL_WARNING_CAPACITY` precedent, T-38-07).
const HOP_BILL_WARNING_CAPACITY: usize = 256;

/// A bounded warn-once set keyed on (provider, model): [`Self::should_warn`] answers `true`
/// exactly once per pair, then `false`, and `false` for any new pair once `capacity` pairs are
/// held. A poisoned lock is recovered, never propagated.
struct HopBillWarnings {
    seen: Mutex<HashSet<(String, String)>>,
    capacity: usize,
}

impl HopBillWarnings {
    fn new(capacity: usize) -> Self {
        Self {
            seen: Mutex::new(HashSet::new()),
            capacity,
        }
    }

    fn should_warn(&self, provider: &str, model: &str) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (provider.to_string(), model.to_string());
        if seen.contains(&key) || seen.len() >= self.capacity {
            return false;
        }
        seen.insert(key)
    }
}

/// The process-wide instance every [`FallbackLlmAdapter`] consults.
static HOP_BILL_WARNINGS: LazyLock<HopBillWarnings> =
    LazyLock::new(|| HopBillWarnings::new(HOP_BILL_WARNING_CAPACITY));

/// Carry the bill of a hop the chain is moving past (G-50-5a remainder (a), WR-03): record it into
/// the call's collector, which prices it with the pricer the decorator outside the chain
/// installed and counts it into the attempt's spend scope at once (so it is charged even if the
/// attempt is cut off while a later hop is in flight), or -- with no collector installed -- warn
/// once per (provider, model) under `HOP_BILL_LOG_TARGET` that it will not be settled. The line names the provider, the model
/// and the token counts only: never refusal text, content or a credential (T-50-30-04).
/// Returns `true` when the bill was recorded.
fn carry_hopped_bill(warnings: &HopBillWarnings, from: &str, billed: &BilledUsage) -> bool {
    if record_hopped_bill(billed.clone()) {
        return true;
    }
    if warnings.should_warn(from, &billed.model) {
        log::warn!(
            target: HOP_BILL_LOG_TARGET,
            "provider '{from}' model '{model}': the fallback chain moved past a billed call \
             ({prompt} prompt and {completion} completion tokens) with no hopped-bill collector \
             installed, so this bill is not settled; route the call through \
             PaladinExecutionService or install paladin_ports::output::spend_scope::HOPPED_BILLS \
             (G-50-5a)",
            model = billed.model,
            prompt = billed.usage.prompt_tokens,
            completion = billed.usage.completion_tokens,
        );
    }
    false
}

/// Construction-time errors for [`FallbackLlmAdapter`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FallbackChainError {
    /// [`FallbackLlmAdapter::new`] was given no providers. Rejected at
    /// construction so the misconfiguration surfaces where it was made,
    /// not on the first request.
    #[error("a fallback chain needs at least one provider")]
    EmptyChain,
    /// [`FallbackLlmAdapter::from_steps`] was given a step whose model is empty (or only
    /// whitespace). A step states the model it sends; an empty one would send nothing, or tempt
    /// the hop's adapter to substitute its own default (D-15's rejected alternative).
    #[error("fallback step {index} has an empty model; every step must name the model it sends")]
    EmptyStepModel {
        /// Zero-based position of the offending step.
        index: usize,
    },
}

/// One hop of a [`FallbackLlmAdapter::from_steps`] chain: the port to call and the model to send
/// it (D-15).
///
/// Before the hop is tried, the request's `model` is rewritten to [`FallbackStep::model`], so an
/// Anthropic -> Kimi chain sends `claude-*` to Anthropic and the Kimi model to Moonshot rather
/// than the first model to both (handoff B-3). The model is also what the hop is paced under
/// (D-00h): Cadence keys on `(provider, model)`.
///
/// A plain struct with public fields, so a caller can also build one without [`FallbackStep::new`].
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "mock")]
/// # {
/// use std::sync::Arc;
/// use paladin_llm::fallback::FallbackStep;
/// use paladin_llm::mock::MockLlmAdapter;
///
/// let step = FallbackStep::new(
///     Arc::new(MockLlmAdapter::new().with_provider_name("kimi")),
///     "kimi-k2",
/// );
/// assert_eq!(step.model, "kimi-k2");
/// assert!(format!("{step:?}").contains("kimi"));
/// # }
/// ```
#[derive(Clone)]
pub struct FallbackStep {
    /// The port this hop calls.
    pub port: Arc<dyn LlmPort>,
    /// The model this hop is sent, in place of the request's own.
    pub model: String,
}

impl FallbackStep {
    /// A step that sends `model` to `port`.
    pub fn new(port: Arc<dyn LlmPort>, model: impl Into<String>) -> Self {
        Self {
            port,
            model: model.into(),
        }
    }
}

impl fmt::Debug for FallbackStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackStep")
            .field("provider", &self.port.get_provider_name())
            .field("model", &self.model)
            .finish()
    }
}

/// The item type every [`LlmPort::generate_stream`] yields.
type StreamItem = Result<StreamingResponse, LlmError>;

/// An ordered chain of [`LlmPort`]s that fails over on `Transient` or
/// `Unknown` errors only (FT-FR-16). See the module documentation for the
/// hop rule, the streaming first-chunk rule and the observability contract.
///
/// # Reasoning effort pre-flight (D-08)
///
/// Every hop is judged by its **own** [`LlmPort::check_reasoning`] -- against the step's model
/// (else the request's), under the request's effort and fallback and then that hop's own
/// provider-level `reasoning_effort` / `reasoning_fallback` -- before the first call. The first
/// hop, in chain order, that cannot honour the effort (or substitute it under the fallback in
/// force) fails the whole request with `LlmError::UnsupportedParameter` naming that hop's
/// provider and model, its reason prefixed `fallback hop <index>: `; no hop is called. A chain
/// therefore never discovers an incompatible hop only during an outage of an earlier one, and a
/// hop whose provider default would refuse when reached refuses the chain up front even when the
/// request sets no effort. A hop that is reached re-admits the request itself and sends its own
/// resolution, recorded as `paladin.reasoning_requested` / `paladin.reasoning_effective` on the
/// served response beside `paladin.served_by`. The chain's own `check_reasoning` is the same
/// call, so `PaladinBuilder::build` over a chain and the pre-flight are one verdict. A request
/// with no effort in force on any hop is never refused.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "mock")]
/// # {
/// use std::sync::Arc;
/// use paladin_llm::fallback::FallbackLlmAdapter;
/// use paladin_llm::mock::MockLlmAdapter;
/// use paladin_ports::output::llm_port::LlmPort;
///
/// let primary = Arc::new(MockLlmAdapter::new().with_provider_name("openai"));
/// let backup = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
/// let chain = FallbackLlmAdapter::new(vec![primary, backup])?;
/// assert_eq!(chain.get_provider_name(), "fallback");
/// assert_eq!(chain.chain_len(), 2);
/// # }
/// # Ok::<(), paladin_llm::fallback::FallbackChainError>(())
/// ```
#[derive(Clone)]
pub struct FallbackLlmAdapter {
    chain: Vec<Arc<dyn LlmPort>>,
    /// The model each hop is sent, parallel to `chain`: `Some` for a
    /// [`FallbackLlmAdapter::from_steps`] chain, `None` for every hop of a
    /// [`FallbackLlmAdapter::new`] chain (the request's own model is sent).
    models: Vec<Option<String>>,
    trace_emitter: Option<Arc<dyn TraceEmitter>>,
    pacing: Option<FallbackPacing>,
    /// Whether a [`LlmError::Refusal`] hops to the next step (D-10). Off by default.
    hop_on_refusal: bool,
}

/// What the chain needs to decide whether a `429` is paced on the same hop (D-03): the shared
/// port to read each hop's gate from, and the bounds that travel with it.
#[derive(Clone)]
struct FallbackPacing {
    port: Arc<dyn CadencePort>,
    /// The longest a hop is paced, per call.
    budget: Duration,
    /// The longest gate the hops' decorators will sleep; a longer one is refused, not waited.
    max_wait: Duration,
}

impl fmt::Debug for FallbackLlmAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackLlmAdapter")
            .field(
                "chain",
                &self
                    .chain
                    .iter()
                    .map(|provider| provider.get_provider_name())
                    .collect::<Vec<_>>(),
            )
            .field("models", &self.models)
            .field("trace_emitter", &self.trace_emitter.is_some())
            .field("paced", &self.pacing.is_some())
            .field("hop_on_refusal", &self.hop_on_refusal)
            .finish()
    }
}

impl FallbackLlmAdapter {
    /// Build a chain from `chain[0]` (tried first) to `chain[n-1]` (tried
    /// last).
    ///
    /// # Errors
    ///
    /// [`FallbackChainError::EmptyChain`] if `chain` is empty.
    pub fn new(chain: Vec<Arc<dyn LlmPort>>) -> Result<Self, FallbackChainError> {
        if chain.is_empty() {
            return Err(FallbackChainError::EmptyChain);
        }
        let models = vec![None; chain.len()];
        Ok(Self {
            chain,
            models,
            trace_emitter: None,
            pacing: None,
            hop_on_refusal: false,
        })
    }

    /// Build a chain whose every hop is sent its own model (D-15, PARITY-06): `steps[0]` is
    /// tried first, `steps[n-1]` last, and before each hop the request's `model` is rewritten to
    /// that step's model, on [`LlmPort::generate`] and [`LlmPort::generate_stream`] alike.
    ///
    /// Two consecutive steps may share a port and differ in model; they are two hops, each sent
    /// its own model. The same model on two steps is still two hops.
    ///
    /// # Errors
    ///
    /// [`FallbackChainError::EmptyChain`] if `steps` is empty;
    /// [`FallbackChainError::EmptyStepModel`] naming the first step whose model is empty after
    /// trimming.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "mock")]
    /// # {
    /// use std::sync::Arc;
    /// use paladin_llm::fallback::{FallbackLlmAdapter, FallbackStep};
    /// use paladin_llm::mock::MockLlmAdapter;
    ///
    /// let anthropic = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
    /// let kimi = Arc::new(MockLlmAdapter::new().with_provider_name("kimi"));
    /// let chain = FallbackLlmAdapter::from_steps(vec![
    ///     FallbackStep::new(anthropic, "claude-sonnet-4"),
    ///     FallbackStep::new(kimi, "kimi-k2"),
    /// ])?;
    /// assert_eq!(chain.chain_len(), 2);
    /// # }
    /// # Ok::<(), paladin_llm::fallback::FallbackChainError>(())
    /// ```
    pub fn from_steps(steps: Vec<FallbackStep>) -> Result<Self, FallbackChainError> {
        if steps.is_empty() {
            return Err(FallbackChainError::EmptyChain);
        }
        if let Some(index) = steps.iter().position(|step| step.model.trim().is_empty()) {
            return Err(FallbackChainError::EmptyStepModel { index });
        }
        let (chain, models): (Vec<_>, Vec<_>) = steps
            .into_iter()
            .map(|step| (step.port, Some(step.model)))
            .unzip();
        Ok(Self {
            chain,
            models,
            trace_emitter: None,
            pacing: None,
            hop_on_refusal: false,
        })
    }

    /// Whether a [`LlmError::Refusal`] hops to the next step (D-10). Default `false`.
    ///
    /// A refusal is `Permanent`, so by default it short-circuits the chain and reaches the
    /// consumer: an automatic hop would hide the refusal and spend budget on every hop. With
    /// `true`, a `Refusal` -- and only a `Refusal` -- tries the next step; every other
    /// `Permanent` error (a bad key, a malformed request) still short-circuits. When the last
    /// step refuses too, the chain ends in [`LlmError::AllProvidersFailed`], whose `attempts`
    /// list every refusal.
    ///
    /// A refusal the vendor billed is not lost when the chain hops past it (G-50-5a): its bill
    /// is recorded for settlement into the call's hopped-bill collector, priced by the pricing
    /// decorator outside the chain against its own model and settled with the call by
    /// `PaladinExecutionService` (see the module docs).
    #[must_use]
    pub fn hop_on_refusal(mut self, enabled: bool) -> Self {
        self.hop_on_refusal = enabled;
        self
    }

    /// Pace every hop (PACE-02, D-03): each hop is wrapped in the Cadence decorator, so it is
    /// gated under its own provider name, and a `429` on a hop is paced on that same provider
    /// until the pace budget is spent before the chain hops. See the module docs' "Pace first,
    /// hop last" section.
    ///
    /// A chain is paced once: calling this on a chain that already has pacing is a no-op, since
    /// wrapping the hops twice would gate them twice. A hop that is itself a fallback chain
    /// (reports [`FALLBACK_PROVIDER_NAME`]) is left as it is, as it paces its own hops.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "mock")]
    /// # {
    /// use std::sync::Arc;
    /// use paladin_llm::cadence::{CadenceSettings, CadenceWiring};
    /// use paladin_llm::fallback::FallbackLlmAdapter;
    /// use paladin_llm::mock::MockLlmAdapter;
    /// use paladin_ports::output::cadence_port::CadencePolicy;
    /// use paladin_storage::cadence::InMemoryCadence;
    ///
    /// let wiring = CadenceWiring::new(
    ///     Arc::new(InMemoryCadence::new(CadencePolicy::default())),
    ///     CadenceSettings::default(),
    /// );
    /// let primary = Arc::new(MockLlmAdapter::new().with_provider_name("openai"));
    /// let backup = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
    /// let chain = FallbackLlmAdapter::new(vec![primary, backup])?.with_cadence(&wiring);
    /// assert!(format!("{chain:?}").contains("paced: true"));
    /// # }
    /// # Ok::<(), paladin_llm::fallback::FallbackChainError>(())
    /// ```
    #[must_use]
    pub fn with_cadence(mut self, wiring: &CadenceWiring) -> Self {
        if self.pacing.is_some() {
            return self;
        }
        self.chain = self
            .chain
            .into_iter()
            .map(|hop| wrap_with_cadence(hop, Some(wiring)))
            .collect();
        self.pacing = Some(FallbackPacing {
            port: Arc::clone(wiring.port()),
            budget: wiring.settings().fallback_pace_budget(),
            max_wait: wiring.settings().max_wait(),
        });
        self
    }

    /// Attach an explicit [`TraceEmitter`] handle that receives one
    /// [`TraceEvent::FallbackHop`] per hop (28-06, D-03), taking precedence
    /// over the ambient `current_trace_emitter()` a real run supplies via
    /// `RUN_TRACE_EMITTER` -- see the module docs' "Per-hop observability"
    /// section for the full precedence rule. Typically used by a caller
    /// with no ambient per-run context (a unit test, or a standalone
    /// construction outside any run).
    pub fn with_trace_emitter(mut self, emitter: Arc<dyn TraceEmitter>) -> Self {
        self.trace_emitter = Some(emitter);
        self
    }

    /// Number of providers in the chain (always at least one).
    pub fn chain_len(&self) -> usize {
        self.chain.len()
    }

    /// The provider tried first. `new` guarantees the chain is non-empty,
    /// so the fallback arm is unreachable in practice but keeps this free of
    /// any panicking path.
    fn first(&self) -> Option<&Arc<dyn LlmPort>> {
        self.chain.first()
    }

    /// Record one hop from `from` to `to`: a `warn!` naming both providers
    /// plus a [`TraceEvent::FallbackHop`] through whichever [`TraceEmitter`]
    /// is available (see the module docs' "Per-hop observability" section
    /// for the explicit-field-then-task-local precedence rule). `emit`
    /// never awaits and never fails visibly (D-03's own contract) -- with
    /// neither source available, only the `warn!` line is produced.
    async fn record_hop(&self, from: &'static str, to: &'static str, err: &LlmError) {
        log::warn!(
            "Fallback chain hopping from provider '{from}' to '{to}' after {transience:?} error: {err}",
            transience = err.transience()
        );
        let Some(emitter) = self.trace_emitter.clone().or_else(current_trace_emitter) else {
            return;
        };
        emitter.emit(TraceEvent::FallbackHop {
            node_id: None,
            from_provider: from.to_string(),
            to_provider: to.to_string(),
        });
    }

    /// Decide what happens after chain element `index` (named `from`)
    /// failed with `err`: `Ok(())` means "hop to the next element", `Err`
    /// is the error the caller must return — the `Permanent` error itself
    /// (short-circuit), or [`LlmError::AllProvidersFailed`] once the chain
    /// is exhausted.
    async fn after_failure(
        &self,
        index: usize,
        from: &'static str,
        err: LlmError,
        attempts: &mut Vec<(String, String)>,
    ) -> Result<(), LlmError> {
        attempts.push((from.to_string(), err.to_string()));
        // A refusal is `Permanent`, so it short-circuits unless the operator opted in (D-10).
        // The opt-in covers a `Refusal` and nothing else: a rejected key or a malformed request
        // is no better at the next provider.
        let hops_past = self.hop_on_refusal && matches!(err, LlmError::Refusal { .. });
        if err.transience() == Transience::Permanent && !hops_past {
            return Err(err);
        }
        match self.chain.get(index + 1) {
            Some(next) => {
                // G-50-5a remainder (a), plan 50-30: the one point where a hop past a billed
                // error is decided. Its bill goes to the call's `HOPPED_BILLS` collector so it
                // is priced (outside the chain) and settled with the call, not dropped with
                // the error. An exhausted chain records nothing here: `AllProvidersFailed`
                // keeps the last hop's own bill (`LlmError::billed_usage` looks through it).
                if let Some(billed) = err.billed_usage() {
                    carry_hopped_bill(&HOP_BILL_WARNINGS, from, billed);
                }
                self.record_hop(from, next.get_provider_name(), &err).await;
                Ok(())
            }
            None => Err(LlmError::AllProvidersFailed {
                attempts: std::mem::take(attempts),
                last: Box::new(err),
            }),
        }
    }

    /// Decide whether a failure of hop `hop` is paced on the SAME hop instead of reaching the
    /// hop rule (D-03). `true` means "retry this hop": the attempt has been recorded and the
    /// hop's own Cadence decorator will sleep its gate before the next send.
    ///
    /// Only a [`LlmError::RateLimitExceeded`] on a chain with pacing attached can be paced, and
    /// only while `time already spent pacing this hop + the hop's current gate` still fits the
    /// budget (equal still paces). The clock starts at the hop's first `429` of this call.
    /// A gate that cannot be read, is already clear, or is longer than `max_wait` is not paced:
    /// with no wait to honour there is nothing to pace against, and retrying would be the
    /// immediate-retry thrash this feature exists to prevent.
    async fn pace_hop(
        &self,
        hop: &Arc<dyn LlmPort>,
        model: &str,
        first_429: &mut Option<Instant>,
        err: &LlmError,
        attempts: &mut Vec<(String, String)>,
    ) -> bool {
        let Some(pacing) = &self.pacing else {
            return false;
        };
        if !matches!(err, LlmError::RateLimitExceeded { .. }) {
            return false;
        }
        let name = hop.get_provider_name();
        let elapsed = first_429.get_or_insert_with(Instant::now).elapsed();
        let wait = match pacing.port.gate(&CadenceKey::new(name, model)).await {
            Ok(reading) => reading.wait(),
            Err(port_error) => {
                log::debug!(
                    target: CADENCE_LOG_TARGET,
                    "provider {name} model {model}: cannot read the gate ({port_error}); \
                     not pacing this 429"
                );
                return false;
            }
        };
        if wait.is_zero() || wait > pacing.max_wait || elapsed.saturating_add(wait) > pacing.budget
        {
            return false;
        }
        attempts.push((name.to_string(), err.to_string()));
        log::debug!(
            target: CADENCE_LOG_TARGET,
            "provider {name} model {model} answered 429; pacing the same provider for {wait:?} \
             ({elapsed:?} of the {budget:?} budget already spent)",
            budget = pacing.budget
        );
        true
    }

    /// The request as hop `index` is sent it: the caller's request with `model` replaced by the
    /// step's model on a [`FallbackLlmAdapter::from_steps`] chain, unchanged on a
    /// [`FallbackLlmAdapter::new`] chain (D-15). The model is never taken from the hop's adapter
    /// default: each step states the model it sends.
    fn request_for_hop(&self, index: usize, request: &LlmRequest) -> LlmRequest {
        let mut hop_request = request.clone();
        if let Some(model) = self.models.get(index).and_then(Option::as_deref) {
            hop_request.model = model.to_string();
        }
        hop_request
    }

    /// D-08: judge the request's reasoning effort against **every** hop before the first call, so
    /// a chain never discovers an incompatible hop only during an outage of an earlier one
    /// (REASON-04).
    ///
    /// This is [`LlmPort::check_reasoning`] on the request's own model, effort and fallback: each
    /// hop is visited in chain order with the model it will be sent (the step's model, else the
    /// request's) and judges the request under its own provider-level `reasoning_effort` /
    /// `reasoning_fallback`. The lowest-index refusing hop is named, its reason prefixed
    /// `fallback hop <index>: `, and no hop is called. A hop that is later reached re-admits the
    /// request itself, so each hop sends its own resolution.
    ///
    /// # Errors
    ///
    /// [`LlmError::UnsupportedParameter`] for the first hop that cannot honour the effort.
    fn preflight_reasoning(&self, request: &LlmRequest) -> Result<(), LlmError> {
        self.check_reasoning(
            &request.model,
            request.reasoning,
            request.reasoning_fallback,
        )
    }

    /// The error returned when every element of the chain has been asked
    /// and none answered — only reachable if the chain were empty, which
    /// `new` forbids.
    fn chain_exhausted() -> LlmError {
        LlmError::ProcessingError("fallback chain has no providers".to_string())
    }
}

#[async_trait]
impl LlmPort for FallbackLlmAdapter {
    /// Try each provider from chain element 0, hopping on `Transient` or
    /// `Unknown` errors only. The served response is stamped with the
    /// provider's name under [`SERVED_BY_METADATA_KEY`].
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // D-08: refuse up front, naming the hop, before any hop is called.
        self.preflight_reasoning(&request)?;
        let mut attempts: Vec<(String, String)> = Vec::new();
        for (index, provider) in self.chain.iter().enumerate() {
            let name = provider.get_provider_name();
            // The request as this hop is sent it (D-15); its model keys the pacing (D-00h).
            let hop_request = self.request_for_hop(index, &request);
            // Per hop, per call: when this hop first answered 429 (D-03).
            let mut first_429 = None;
            loop {
                match provider.generate(hop_request.clone()).await {
                    Ok(mut response) => {
                        response
                            .metadata
                            .insert(SERVED_BY_METADATA_KEY.to_string(), name.to_string());
                        response.metadata.insert(
                            SERVED_MODEL_METADATA_KEY.to_string(),
                            hop_request.model.clone(),
                        );
                        return Ok(response);
                    }
                    Err(err) => {
                        let paced = self
                            .pace_hop(
                                provider,
                                &hop_request.model,
                                &mut first_429,
                                &err,
                                &mut attempts,
                            )
                            .await;
                        if paced {
                            continue;
                        }
                        self.after_failure(index, name, err, &mut attempts).await?;
                        break;
                    }
                }
            }
        }
        Err(Self::chain_exhausted())
    }

    /// Fall through only when the call itself fails or the stream's FIRST
    /// item is `Err`; once any `Ok` chunk has been observed the stream is
    /// handed to the caller and later errors propagate (D-25). The serving hop is
    /// named on the terminal chunk's [`StreamingResponse::metadata`]
    /// ([`SERVED_BY_METADATA_KEY`], [`SERVED_MODEL_METADATA_KEY`]), the streamed
    /// counterpart of what [`generate`](Self::generate) stamps on its response.
    async fn generate_stream(
        &self,
        request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = StreamItem> + Send>, LlmError> {
        // D-08: the same up-front refusal as `generate`, before any hop is called.
        self.preflight_reasoning(&request)?;
        let mut attempts: Vec<(String, String)> = Vec::new();
        for (index, provider) in self.chain.iter().enumerate() {
            let name = provider.get_provider_name();
            // The request as this hop is sent it (D-15); its model keys the pacing (D-00h).
            let hop_request = self.request_for_hop(index, &request);
            // Per hop, per call: when this hop first answered 429 (D-03).
            let mut first_429 = None;
            loop {
                let first_item = match provider.generate_stream(hop_request.clone()).await {
                    Ok(raw) => {
                        // Peek exactly one item. `Box<dyn Stream>` is not
                        // `Unpin`, so pin it once here and keep the pinned
                        // handle — it is what the caller receives.
                        let mut rest = Box::into_pin(raw);
                        match rest.next().await {
                            Some(Err(err)) => Err(err),
                            peeked => Ok((peeked, rest)),
                        }
                    }
                    Err(err) => Err(err),
                };
                match first_item {
                    Ok((peeked, rest)) => {
                        // Re-attach the peeked chunk (or nothing, if the stream
                        // was already exhausted) in front of the remainder so
                        // the consumer sees every item the provider produced.
                        //
                        // ADR-0061 (h), owner reading R-1 (option-b): the terminal chunk --
                        // and only it -- names the hop that served this stream, with the same
                        // two keys `generate` stamps on its response.
                        let served_by = name.to_string();
                        let served_model = hop_request.model.clone();
                        let replayed = stream::iter(peeked).chain(rest).map(move |item| {
                            item.map(|chunk| {
                                if chunk.finish_reason.is_none() {
                                    return chunk;
                                }
                                chunk
                                    .with_metadata(SERVED_BY_METADATA_KEY, served_by.as_str())
                                    .with_metadata(SERVED_MODEL_METADATA_KEY, served_model.as_str())
                            })
                        });
                        return Ok(Box::new(replayed));
                    }
                    Err(err) => {
                        let paced = self
                            .pace_hop(
                                provider,
                                &hop_request.model,
                                &mut first_429,
                                &err,
                                &mut attempts,
                            )
                            .await;
                        if paced {
                            continue;
                        }
                        self.after_failure(index, name, err, &mut attempts).await?;
                        break;
                    }
                }
            }
        }
        Err(Self::chain_exhausted())
    }

    /// The first element that answers `Ok`; the last element's error if
    /// none does (D-24).
    async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
        let mut last_err = None;
        for provider in &self.chain {
            match provider.validate_model(model).await {
                Ok(valid) => return Ok(valid),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(Self::chain_exhausted))
    }

    /// The first element that answers `Ok`; the last element's error if
    /// none does (D-24).
    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        let mut last_err = None;
        for provider in &self.chain {
            match provider.get_available_models().await {
                Ok(models) => return Ok(models),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.unwrap_or_else(Self::chain_exhausted))
    }

    /// Always [`FALLBACK_PROVIDER_NAME`]; the provider that actually served
    /// a response is in its metadata under [`SERVED_BY_METADATA_KEY`].
    fn get_provider_name(&self) -> &'static str {
        FALLBACK_PROVIDER_NAME
    }

    /// The first element's capabilities (D-24).
    fn get_capabilities(&self) -> ProviderCapabilities {
        self.first()
            .map(|provider| provider.get_capabilities())
            .unwrap_or_default()
    }

    /// The strictest sampling profile any hop would accept (ADR-0061): the
    /// [`ModelProfile::intersect`] of every hop's `Some` profile for `model`, so the builder
    /// refuses a sampling value any hop could not honour. `None` when no hop declares a profile.
    ///
    /// For sampling, a hop that returns `None` (a third-party adapter with no profile) is skipped:
    /// each hop's own request builder applies its own limits. For effort the chain's profile is
    /// [`EffortSupport::Unmeasured`] (with no `effort_verified_on`) whenever any hop has none, so
    /// no reader can mistake a chain with an unprofiled hop for one that can take an explicit
    /// effort. The authority for an effort verdict is
    /// [`check_reasoning`](LlmPort::check_reasoning), which judges every hop against its own
    /// profile and defaults; this profile is not consulted for it. The intersection is
    /// order-independent, so the order the hops are listed in does not change the answer.
    ///
    /// Each hop is asked about the model it will be sent (D-15): a
    /// [`FallbackLlmAdapter::from_steps`] chain asks every step about that step's own model and
    /// ignores `model`; a [`FallbackLlmAdapter::new`] chain asks every hop about `model`, which
    /// is what each hop is sent.
    fn model_profile(&self, model: &str) -> Option<ModelProfile> {
        let mut any_hop_without_profile = false;
        let mut combined: Option<ModelProfile> = None;
        for (provider, step_model) in self.chain.iter().zip(&self.models) {
            match provider.model_profile(step_model.as_deref().unwrap_or(model)) {
                Some(next) => {
                    combined = Some(match combined {
                        Some(so_far) => so_far.intersect(&next),
                        None => next,
                    });
                }
                None => any_hop_without_profile = true,
            }
        }
        if any_hop_without_profile && let Some(profile) = combined.as_mut() {
            profile.effort = EffortSupport::Unmeasured;
            profile.effort_verified_on = None;
        }
        combined
    }

    /// The AND over hops, in chain order, of each hop's own
    /// [`check_reasoning`](LlmPort::check_reasoning) (D-08, D-09): hop `index` is asked about the
    /// model it will be sent (the step's model, else `model`) with the same effort and fallback,
    /// and judges them under its own provider-level defaults. The first refusal is returned as
    /// [`LlmError::UnsupportedParameter`] with that hop's provider and model and its reason
    /// prefixed `fallback hop <index>: `; any other error is returned unchanged. No hop is called
    /// over the network, and no profile is read for a hop with no effort in force.
    ///
    /// The chain's [`model_profile`](LlmPort::model_profile) is an intersection, which skips
    /// sampling for a hop with no profile and over-refuses level sets that only touch; this
    /// per-hop verdict is the authority for effort, at build, pre-flight and send alike.
    fn check_reasoning(
        &self,
        model: &str,
        effort: Option<ReasoningEffort>,
        fallback: Option<ReasoningFallback>,
    ) -> Result<(), LlmError> {
        for (index, port) in self.chain.iter().enumerate() {
            let hop_model = self
                .models
                .get(index)
                .and_then(Option::as_deref)
                .unwrap_or(model);
            match port.check_reasoning(hop_model, effort, fallback) {
                Ok(()) => {}
                Err(LlmError::UnsupportedParameter {
                    param,
                    value,
                    provider,
                    model,
                    reason,
                    ..
                }) => {
                    return Err(LlmError::unsupported_parameter(
                        param,
                        value,
                        provider,
                        model,
                        format!("fallback hop {index}: {reason}"),
                    ));
                }
                Err(other) => return Err(other),
            }
        }
        Ok(())
    }
}

/// A scripted [`LlmPort`] that reports a fixed [`ModelProfile`] for one model, used by the
/// decorator tests in this crate to prove `model_profile` is forwarded (ADR-0061, Pitfall 4).
///
/// Compiled with the `mock` feature only: its users (the `cadence`, `fallback` and `pricing`
/// decorator tests) are all mock-driven, so a build without `mock` would carry it unused.
#[cfg(all(test, feature = "mock"))]
pub(crate) mod test_support {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::stream::Stream;
    use paladin_ports::output::llm_port::{
        LlmError, LlmPort, LlmRequest, LlmResponse, ModelProfile, ProviderCapabilities,
        StreamingResponse,
    };

    /// Reports `profile` for `model` and `None` for any other model, recording every model it is
    /// asked about so a test can check the model string passes through unchanged.
    pub(crate) struct ProfiledStubPort {
        name: &'static str,
        model: &'static str,
        profile: Option<ModelProfile>,
        asked: Mutex<Vec<String>>,
    }

    impl ProfiledStubPort {
        pub(crate) fn new(
            name: &'static str,
            model: &'static str,
            profile: Option<ModelProfile>,
        ) -> Self {
            Self {
                name,
                model,
                profile,
                asked: Mutex::new(Vec::new()),
            }
        }

        /// The models `model_profile` has been called with, in order.
        pub(crate) fn asked(&self) -> Vec<String> {
            self.asked
                .lock()
                .map(|asked| asked.clone())
                .unwrap_or_default()
        }
    }

    #[async_trait]
    impl LlmPort for ProfiledStubPort {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::ProcessingError(
                "ProfiledStubPort does not generate".to_string(),
            ))
        }

        async fn generate_stream(
            &self,
            _request: LlmRequest,
        ) -> Result<Box<dyn Stream<Item = Result<StreamingResponse, LlmError>> + Send>, LlmError>
        {
            Err(LlmError::ProcessingError(
                "ProfiledStubPort does not stream".to_string(),
            ))
        }

        async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
            Ok(true)
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            Ok(vec![self.model.to_string()])
        }

        fn get_provider_name(&self) -> &'static str {
            self.name
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn model_profile(&self, model: &str) -> Option<ModelProfile> {
            if let Ok(mut asked) = self.asked.lock() {
                asked.push(model.to_string());
            }
            if model == self.model {
                self.profile.clone()
            } else {
                None
            }
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::mock::MockLlmAdapter;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_core::platform::container::waypoint::ThreadId;
    use paladin_ports::output::trace_sink_port::{
        StandaloneEmitter, TraceRecord, TraceSink, TraceSinkError,
    };
    use std::time::Duration;

    /// A [`TraceSink`] that records every record it receives, in order.
    #[derive(Default)]
    struct RecordingSink {
        events: tokio::sync::Mutex<Vec<TraceRecord>>,
    }

    impl RecordingSink {
        async fn events(&self) -> Vec<TraceRecord> {
            self.events.lock().await.clone()
        }

        /// As [`RecordingSink::events`], but polls (up to a 2s guard)
        /// until at least `expected_len` records have arrived --
        /// `StandaloneEmitter`'s background consumer task drains
        /// asynchronously (28-06), so a test reading straight after
        /// `adapter.generate(..).await` returns needs to wait for that
        /// task to run at least once rather than racing it.
        async fn events_eventually(&self, expected_len: usize) -> Vec<TraceRecord> {
            for _ in 0..200 {
                let events = self.events().await;
                if events.len() >= expected_len {
                    return events;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            self.events().await
        }

        async fn hops_eventually(
            &self,
            expected_len: usize,
        ) -> Vec<(Option<String>, String, String)> {
            self.events_eventually(expected_len)
                .await
                .into_iter()
                .filter_map(|record| match record.event {
                    TraceEvent::FallbackHop {
                        node_id,
                        from_provider,
                        to_provider,
                    } => Some((node_id.map(|id| id.to_string()), from_provider, to_provider)),
                    _ => None,
                })
                .collect()
        }
    }

    #[async_trait]
    impl TraceSink for RecordingSink {
        async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
            self.events.lock().await.push(record);
            Ok(())
        }
    }

    /// Wrap `sink` behind a [`StandaloneEmitter`] (28-06) so it can be
    /// attached via [`FallbackLlmAdapter::with_trace_emitter`] -- the test
    /// double's shape (a bare [`TraceSink`] recording every record) needs
    /// no change of its own; only the attachment mechanism changed from a
    /// raw sink to an emitter handle.
    fn emitter_over(sink: Arc<RecordingSink>) -> Arc<dyn TraceEmitter> {
        Arc::new(StandaloneEmitter::new(
            ThreadId::new("fallback-test").unwrap(),
            None,
            Some(sink),
        ))
    }

    fn request() -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "quest".to_string(),
            context: None,
        }))
        .unwrap();
        LlmRequest::new("mock-model", prompt)
    }

    fn transient(provider: &str, status: u16) -> LlmError {
        LlmError::ProviderError {
            provider: provider.to_string(),
            status,
            message: "upstream unavailable".to_string(),
        }
    }

    fn provider(name: &'static str) -> MockLlmAdapter {
        MockLlmAdapter::new()
            .with_provider_name(name)
            .with_response(format!("answer from {name}"))
    }

    fn failing(name: &'static str, error: LlmError) -> MockLlmAdapter {
        MockLlmAdapter::new()
            .with_provider_name(name)
            .with_error(error)
    }

    fn chain(providers: &[MockLlmAdapter]) -> FallbackLlmAdapter {
        FallbackLlmAdapter::new(
            providers
                .iter()
                .map(|p| Arc::new(p.clone()) as Arc<dyn LlmPort>)
                .collect(),
        )
        .unwrap()
    }

    async fn collect(
        stream: Box<dyn Stream<Item = StreamItem> + Send>,
    ) -> Vec<Result<String, LlmError>> {
        Box::into_pin(stream)
            .map(|item| item.map(|chunk| chunk.delta))
            .collect()
            .await
    }

    #[test]
    fn empty_chain_is_rejected_at_construction() {
        let result = FallbackLlmAdapter::new(Vec::new());
        assert!(matches!(result, Err(FallbackChainError::EmptyChain)));
    }

    #[test]
    fn debug_lists_provider_names_without_exposing_providers() {
        let adapter = chain(&[provider("openai"), provider("anthropic")]);
        let rendered = format!("{adapter:?}");
        assert!(rendered.contains("openai"), "{rendered}");
        assert!(rendered.contains("anthropic"), "{rendered}");
    }

    /// Test 1: providers 1 and 2 return 503, provider 3 serves; the response
    /// names provider 3 and exactly two hop events were emitted.
    #[tokio::test]
    async fn three_provider_chain_falls_through_two_transient_failures() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = failing("anthropic", transient("anthropic", 503));
        let p3 = provider("deepseek");
        let sink = Arc::new(RecordingSink::default());
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()])
            .with_trace_emitter(emitter_over(sink.clone()));

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(response.content, "answer from deepseek");
        assert_eq!(
            response
                .metadata
                .get(SERVED_BY_METADATA_KEY)
                .map(String::as_str),
            Some("deepseek")
        );
        assert_eq!(
            (p1.call_count(), p2.call_count(), p3.call_count()),
            (1, 1, 1)
        );
        let hops = sink.hops_eventually(2).await;
        assert_eq!(hops.len(), 2, "{hops:?}");
        assert_eq!(
            hops[0],
            (None, "openai".to_string(), "anthropic".to_string())
        );
        assert_eq!(
            hops[1],
            (None, "anthropic".to_string(), "deepseek".to_string())
        );
    }

    /// Test 2: a Permanent error short-circuits — providers 2 and 3 are never
    /// called and no hop event is emitted (T-25-35).
    #[tokio::test]
    async fn permanent_error_short_circuits_after_one_call() {
        let p1 = failing(
            "openai",
            LlmError::AuthenticationError("bad key".to_string()),
        );
        let p2 = provider("anthropic");
        let p3 = provider("deepseek");
        let sink = Arc::new(RecordingSink::default());
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()])
            .with_trace_emitter(emitter_over(sink.clone()));

        let result = adapter.generate(request()).await;

        assert!(
            matches!(result, Err(LlmError::AuthenticationError(_))),
            "{result:?}"
        );
        assert_eq!(p1.call_count(), 1);
        assert_eq!(p2.call_count(), 0, "provider 2 must never be called");
        assert_eq!(p3.call_count(), 0, "provider 3 must never be called");
        assert!(sink.events().await.is_empty());
    }

    /// Test 3: an Unknown error (`ProcessingError`) hops.
    #[tokio::test]
    async fn unknown_error_hops() {
        let p1 = failing(
            "openai",
            LlmError::ProcessingError("something odd".to_string()),
        );
        let p2 = provider("anthropic");
        let adapter = chain(&[p1.clone(), p2.clone()]);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(response.content, "answer from anthropic");
        assert_eq!((p1.call_count(), p2.call_count()), (1, 1));
    }

    /// Test 4: when every provider fails transiently the error is
    /// `AllProvidersFailed`, its attempts follow chain order with one entry
    /// per provider, and its transience is the LAST error's.
    #[tokio::test]
    async fn exhaustion_returns_all_providers_failed_in_chain_order() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = failing("anthropic", transient("anthropic", 429));
        let p3 = failing(
            "deepseek",
            LlmError::ProcessingError("unclassifiable".to_string()),
        );
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()]);

        let err = adapter.generate(request()).await.unwrap_err();

        let LlmError::AllProvidersFailed { attempts, last } = &err else {
            panic!("expected AllProvidersFailed, got {err:?}");
        };
        let names: Vec<&str> = attempts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["openai", "anthropic", "deepseek"]);
        assert!(attempts[0].1.contains("503"), "{:?}", attempts[0]);
        assert!(matches!(**last, LlmError::ProcessingError(_)));
        assert_eq!(err.transience(), Transience::Unknown);
        assert_eq!(err.transience(), last.transience());
        assert_eq!(
            (p1.call_count(), p2.call_count(), p3.call_count()),
            (1, 1, 1)
        );
    }

    /// Test 5: after a call that hopped to provider 3, the next call still
    /// starts at provider 1 — no sticky provider state.
    #[tokio::test]
    async fn every_call_starts_at_the_first_provider() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = failing("anthropic", transient("anthropic", 503));
        let p3 = provider("deepseek");
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()]);

        adapter.generate(request()).await.unwrap();
        adapter.generate(request()).await.unwrap();

        assert_eq!(
            (p1.call_count(), p2.call_count(), p3.call_count()),
            (2, 2, 2)
        );
    }

    /// Test 6: concurrent calls share no hop state — exact per-provider call
    /// counts under a multi-thread runtime, with a timeout guard (T-25-36).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_calls_share_no_hop_state() {
        const CALLS: usize = 16;
        let p1 = failing("openai", transient("openai", 503)).with_delay(Duration::from_millis(2));
        let p2 = provider("anthropic").with_delay(Duration::from_millis(2));
        let p3 = provider("deepseek");
        let adapter = Arc::new(chain(&[p1.clone(), p2.clone(), p3.clone()]));

        let tasks: Vec<_> = (0..CALLS)
            .map(|_| {
                let adapter = Arc::clone(&adapter);
                tokio::spawn(async move { adapter.generate(request()).await })
            })
            .collect();
        let outcomes =
            tokio::time::timeout(Duration::from_secs(10), futures::future::join_all(tasks))
                .await
                .expect("concurrent fallback calls must finish within the guard");

        for outcome in outcomes {
            let response = outcome.unwrap().unwrap();
            assert_eq!(response.content, "answer from anthropic");
        }
        assert_eq!(p1.call_count(), CALLS);
        assert_eq!(p2.call_count(), CALLS);
        assert_eq!(p3.call_count(), 0);
    }

    /// Test 7: a stream whose FIRST item is `Err` falls through; provider 2's
    /// stream is delivered intact.
    #[tokio::test]
    async fn streaming_error_before_the_first_chunk_falls_through() {
        let p1 = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_stream_items(vec![Err(transient("openai", 503))]);
        let p2 = MockLlmAdapter::new()
            .with_provider_name("anthropic")
            .with_stream_items(vec![Ok("Hello".to_string()), Ok(" world".to_string())]);
        let sink = Arc::new(RecordingSink::default());
        let adapter =
            chain(&[p1.clone(), p2.clone()]).with_trace_emitter(emitter_over(sink.clone()));

        let stream = adapter.generate_stream(request()).await.unwrap();
        let items = collect(stream).await;

        let deltas: Vec<&str> = items
            .iter()
            .map(|item| item.as_ref().map(String::as_str).unwrap_or("<err>"))
            .collect();
        assert_eq!(deltas, ["Hello", " world"]);
        assert_eq!((p1.call_count(), p2.call_count()), (1, 1));
        assert_eq!(sink.hops_eventually(1).await.len(), 1);
    }

    /// Test 7b: `generate_stream` itself returning `Err` also falls through.
    #[tokio::test]
    async fn streaming_call_error_falls_through() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = MockLlmAdapter::new()
            .with_provider_name("anthropic")
            .with_stream_items(vec![Ok("Hello".to_string())]);
        let adapter = chain(&[p1.clone(), p2.clone()]);

        let stream = adapter.generate_stream(request()).await.unwrap();
        let items = collect(stream).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].as_deref().unwrap(), "Hello");
        assert_eq!((p1.call_count(), p2.call_count()), (1, 1));
    }

    /// Test 8: after one `Ok` chunk an error propagates with the prefix
    /// intact and provider 2 is never called (T-25-33).
    #[tokio::test]
    async fn streaming_error_after_the_first_chunk_propagates_with_the_prefix() {
        let p1 = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_stream_items(vec![
                Ok("partial".to_string()),
                Err(transient("openai", 503)),
            ]);
        let p2 = MockLlmAdapter::new()
            .with_provider_name("anthropic")
            .with_stream_items(vec![Ok("never".to_string())]);
        let sink = Arc::new(RecordingSink::default());
        let adapter =
            chain(&[p1.clone(), p2.clone()]).with_trace_emitter(emitter_over(sink.clone()));

        let stream = adapter.generate_stream(request()).await.unwrap();
        let items = collect(stream).await;

        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(items[0].as_deref().unwrap(), "partial");
        assert!(
            matches!(items[1], Err(LlmError::ProviderError { status: 503, .. })),
            "{:?}",
            items[1]
        );
        assert_eq!(p2.call_count(), 0, "no provider switch after a chunk");
        assert!(sink.events().await.is_empty());
    }

    /// Test 8b: exhaustion on the streaming path reports `AllProvidersFailed`
    /// exactly like the request-response path.
    #[tokio::test]
    async fn streaming_exhaustion_returns_all_providers_failed() {
        let p1 = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_stream_items(vec![Err(transient("openai", 503))]);
        let p2 = failing("anthropic", transient("anthropic", 502));
        let adapter = chain(&[p1.clone(), p2.clone()]);

        let err = adapter.generate_stream(request()).await.err().unwrap();

        let LlmError::AllProvidersFailed { attempts, .. } = &err else {
            panic!("expected AllProvidersFailed, got {err:?}");
        };
        let names: Vec<&str> = attempts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["openai", "anthropic"]);
        assert_eq!(err.transience(), Transience::Transient);
    }

    /// Test 9: identity methods follow the chain rules (D-24).
    #[tokio::test]
    async fn adapter_identity_methods_follow_the_chain_rules() {
        let p1 = provider("openai")
            .with_available_models(vec!["gpt".to_string()])
            .with_model_query_error(LlmError::NetworkError("down".to_string()));
        let p2 = provider("anthropic").with_available_models(vec!["claude".to_string()]);
        let adapter = chain(&[p1.clone(), p2.clone()]);

        assert_eq!(adapter.get_provider_name(), "fallback");
        assert_eq!(adapter.get_capabilities(), p1.get_capabilities());
        assert!(adapter.validate_model("claude").await.unwrap());
        assert!(!adapter.validate_model("gpt").await.unwrap());
        assert_eq!(
            adapter.get_available_models().await.unwrap(),
            vec!["claude".to_string()]
        );
    }

    /// Test 9b: when no element answers `Ok`, the LAST element's error is
    /// returned.
    #[tokio::test]
    async fn identity_methods_return_the_last_error_when_no_element_answers() {
        let p1 =
            provider("openai").with_model_query_error(LlmError::NetworkError("one".to_string()));
        let p2 = provider("anthropic")
            .with_model_query_error(LlmError::ProcessingError("two".to_string()));
        let adapter = chain(&[p1, p2]);

        let err = adapter.get_available_models().await.unwrap_err();
        assert!(
            matches!(err, LlmError::ProcessingError(ref m) if m == "two"),
            "{err:?}"
        );
        let err = adapter.validate_model("x").await.unwrap_err();
        assert!(
            matches!(err, LlmError::ProcessingError(ref m) if m == "two"),
            "{err:?}"
        );
    }

    /// Test 10: each hop's event carries `node_id: None` and the correct
    /// `from_provider` / `to_provider`; the `warn!` line is emitted on the
    /// same path (`record_hop`).
    #[tokio::test]
    async fn each_hop_emits_a_trace_event_and_a_warning() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = failing("anthropic", LlmError::Timeout("1s".to_string()));
        let p3 = provider("deepseek");
        let sink = Arc::new(RecordingSink::default());
        let adapter = chain(&[p1, p2, p3]).with_trace_emitter(emitter_over(sink.clone()));

        adapter.generate(request()).await.unwrap();

        let events = sink.events_eventually(2).await;
        assert_eq!(events.len(), 2);
        for (event, (from, to)) in events
            .iter()
            .zip([("openai", "anthropic"), ("anthropic", "deepseek")])
        {
            match &event.event {
                TraceEvent::FallbackHop {
                    node_id,
                    from_provider,
                    to_provider,
                } => {
                    assert!(node_id.is_none(), "the adapter cannot know the node");
                    assert_eq!(from_provider, from);
                    assert_eq!(to_provider, to);
                }
                other => panic!("unexpected event {other:?}"),
            }
        }
    }

    /// A single-provider chain that fails transiently still reports
    /// exhaustion through `AllProvidersFailed` with one attempt.
    #[tokio::test]
    async fn single_provider_chain_reports_exhaustion_with_one_attempt() {
        let p1 = failing("openai", transient("openai", 500));
        let adapter = chain(&[p1]);

        let err = adapter.generate(request()).await.unwrap_err();
        assert!(
            matches!(&err, LlmError::AllProvidersFailed { attempts, .. } if attempts.len() == 1),
            "{err:?}"
        );
    }

    /// D-03 (28-06): a `FallbackHop` emitted while running inside a
    /// `RUN_TRACE_EMITTER` scope (the ambient per-run handle a real worker
    /// dispatch sets, `paladin_ports::output::trace_sink_port`) lands in
    /// that run's OWN `seq` sequence -- between the surrounding
    /// `NodeStarted`/`NodeFinished` records the "run" itself emits, not a
    /// competing sequence of its own. The adapter carries no explicit
    /// `with_trace_emitter` handle of its own here: it must fall back to
    /// the ambient one.
    #[tokio::test]
    async fn fallback_hop_lands_in_the_run_sequence() {
        use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind};
        use paladin_ports::output::trace_sink_port::RUN_TRACE_EMITTER;

        let sink = Arc::new(RecordingSink::default());
        let run_emitter: Arc<dyn TraceEmitter> = emitter_over(sink.clone());

        let p1 = failing("openai", transient("openai", 503));
        let p2 = provider("anthropic");
        let adapter = chain(&[p1, p2]);

        RUN_TRACE_EMITTER
            .scope(run_emitter.clone(), async {
                run_emitter.emit(TraceEvent::NodeStarted {
                    superstep: 1,
                    node_id: NodeId::new("n1"),
                    attempt: 1,
                    muster_task_key: None,
                });
                adapter.generate(request()).await.unwrap();
                run_emitter.emit(TraceEvent::NodeFinished {
                    superstep: 1,
                    node_id: NodeId::new("n1"),
                    attempt: 1,
                    outcome: NodeOutcomeKind::Succeeded,
                    duration_ms: 0,
                    usage: paladin_core::platform::container::token_usage::TokenUsage::default(),
                    cost: None,
                    reasoning_requested: None,
                    reasoning_effective: None,
                    cache_hit: false,
                });
            })
            .await;

        let events = sink.events_eventually(3).await;
        assert_eq!(events.len(), 3, "{events:?}");

        let mut seqs: Vec<u64> = events.iter().map(|record| record.seq).collect();
        seqs.sort_unstable();
        assert_eq!(seqs, vec![1, 2, 3], "one gapless sequence, no restart");

        let started_seq = events
            .iter()
            .find(|r| matches!(r.event, TraceEvent::NodeStarted { .. }))
            .expect("NodeStarted must have been recorded")
            .seq;
        let hop_seq = events
            .iter()
            .find(|r| matches!(r.event, TraceEvent::FallbackHop { .. }))
            .expect("the hop must have been recorded")
            .seq;
        let finished_seq = events
            .iter()
            .find(|r| matches!(r.event, TraceEvent::NodeFinished { .. }))
            .expect("NodeFinished must have been recorded")
            .seq;
        assert!(
            started_seq < hop_seq && hop_seq < finished_seq,
            "the hop must land BETWEEN NodeStarted and NodeFinished: \
             started={started_seq} hop={hop_seq} finished={finished_seq}"
        );
    }

    /// A bare `FallbackLlmAdapter` under unit test, with NO ambient
    /// `RUN_TRACE_EMITTER` scope and NO explicit `with_trace_emitter`
    /// handle, records no `TraceEvent` at all -- `record_hop`'s own
    /// `let Some(emitter) = ... else { return; }` short-circuit, proven
    /// directly rather than merely by absence of a panic.
    #[tokio::test]
    async fn no_emitter_available_records_nothing() {
        let p1 = failing("openai", transient("openai", 503));
        let p2 = provider("anthropic");
        let adapter = chain(&[p1, p2]);

        let response = adapter.generate(request()).await.unwrap();
        assert_eq!(response.content, "answer from anthropic");
        // No sink was ever attached and no scope was ever entered -- there
        // is nothing to assert beyond "this did not panic and the call
        // still succeeded", which is the whole point: observability is
        // never load-bearing.
    }

    /// A bare `FallbackLlmAdapter` in a unit test, wired with a
    /// `StandaloneEmitter` (28-06) -- no engine, no worker, no ambient
    /// `RUN_TRACE_EMITTER` scope -- still produces well-formed records
    /// under its OWN gapless `seq` counter.
    #[tokio::test]
    async fn standalone_emitter_is_used_when_no_engine_is_present() {
        let sink = Arc::new(RecordingSink::default());
        let emitter = emitter_over(sink.clone());

        let p1 = failing("openai", transient("openai", 503));
        let p2 = failing("anthropic", transient("anthropic", 503));
        let p3 = provider("deepseek");
        let adapter = chain(&[p1, p2, p3]).with_trace_emitter(emitter);

        let response = adapter.generate(request()).await.unwrap();
        assert_eq!(response.content, "answer from deepseek");

        let events = sink.events_eventually(2).await;
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert!(
            events
                .iter()
                .all(|r| matches!(r.event, TraceEvent::FallbackHop { .. })),
            "{events:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Pace first, hop last (PACE-02, D-03; Phase 43 plan 06)
    // -----------------------------------------------------------------------

    use crate::cadence::{CadenceSettings, CadenceWiring, pin_waiter_spread};
    use crate::mock::MockScriptEntry;
    use paladin_ports::output::cadence_port::{
        CadenceError, CadenceKey, CadencePolicy, CadencePort, FencingToken, GateReading, LockKey,
    };
    use paladin_storage::cadence::InMemoryCadence;
    use std::sync::Mutex;
    use tokio::time::Instant;

    /// Tokio rounds a timer deadline up to the next millisecond.
    const TIMER_ROUNDING: Duration = Duration::from_millis(3);
    const BASE: Duration = Duration::from_millis(500);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    /// A provider that records the paused-clock instant of every `generate` and `generate_stream`
    /// it receives, then
    /// delegates to a mock. Clones share the record.
    #[derive(Clone)]
    struct TimedProvider {
        inner: MockLlmAdapter,
        sends: Arc<Mutex<Vec<Instant>>>,
    }

    impl TimedProvider {
        fn new(inner: MockLlmAdapter) -> Self {
            Self {
                inner,
                sends: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn call_count(&self) -> usize {
            self.inner.call_count()
        }

        /// Offsets of every send from the first one.
        fn offsets(&self) -> Vec<Duration> {
            let sends = self.sends.lock().unwrap();
            let Some(first) = sends.first().copied() else {
                return Vec::new();
            };
            sends.iter().map(|at| at.duration_since(first)).collect()
        }

        /// Time between consecutive sends.
        fn gaps(&self) -> Vec<Duration> {
            let sends = self.sends.lock().unwrap();
            sends
                .windows(2)
                .map(|pair| pair[1].duration_since(pair[0]))
                .collect()
        }
    }

    #[async_trait]
    impl LlmPort for TimedProvider {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            self.sends.lock().unwrap().push(Instant::now());
            self.inner.generate(request).await
        }

        async fn generate_stream(
            &self,
            request: LlmRequest,
        ) -> Result<Box<dyn Stream<Item = StreamItem> + Send>, LlmError> {
            self.sends.lock().unwrap().push(Instant::now());
            self.inner.generate_stream(request).await
        }

        async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
            self.inner.validate_model(model).await
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            self.inner.get_available_models().await
        }

        fn get_provider_name(&self) -> &'static str {
            self.inner.get_provider_name()
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            self.inner.get_capabilities()
        }
    }

    /// `name` answers `script` in order, cycling.
    fn scripted(name: &'static str, script: Vec<MockScriptEntry>) -> TimedProvider {
        TimedProvider::new(
            MockLlmAdapter::new()
                .with_provider_name(name)
                .with_script(script),
        )
    }

    /// `name` answers `error` on every call.
    fn always(name: &'static str, error: LlmError) -> TimedProvider {
        TimedProvider::new(failing(name, error))
    }

    fn answer(name: &str) -> MockScriptEntry {
        MockScriptEntry::Text(format!("answer from {name}"))
    }

    fn explicit_429(delay: Duration) -> LlmError {
        LlmError::rate_limited(Some(delay))
    }

    /// A wiring over an in-process port with the given jitter source and pace budget. The port
    /// is returned so a test can read the gates the chain left behind.
    fn pacing(jitter: fn() -> f64, budget: Duration) -> (CadenceWiring, Arc<InMemoryCadence>) {
        let port = Arc::new(
            InMemoryCadence::new(CadencePolicy::new(BASE, MAX_BACKOFF).expect("policy"))
                .with_jitter(jitter),
        );
        let settings = CadenceSettings::new(Duration::from_secs(300), MAX_BACKOFF, budget);
        (CadenceWiring::new(port.clone(), settings), port)
    }

    fn paced_chain(hops: &[&TimedProvider], wiring: &CadenceWiring) -> FallbackLlmAdapter {
        FallbackLlmAdapter::new(
            hops.iter()
                .map(|hop| Arc::new((*hop).clone()) as Arc<dyn LlmPort>)
                .collect(),
        )
        .unwrap()
        .with_cadence(wiring)
    }

    fn served_by(response: &LlmResponse) -> Option<&str> {
        response
            .metadata
            .get(SERVED_BY_METADATA_KEY)
            .map(String::as_str)
    }

    fn assert_close(actual: Duration, expected: Duration, what: &str) {
        let (lo, hi) = (
            expected.saturating_sub(TIMER_ROUNDING),
            expected + TIMER_ROUNDING,
        );
        assert!(
            (lo..=hi).contains(&actual),
            "{what}: expected about {expected:?}, got {actual:?}"
        );
    }

    /// ROADMAP success criterion 2: a mocked 429 against a fallback hop honours the retry delay
    /// as a minimum and stays on the provider instead of hopping or thrashing.
    #[tokio::test(start_paused = true)]
    async fn mocked_429_on_a_fallback_hop_backs_off_and_stays_on_the_provider() {
        let primary = scripted(
            "openai",
            vec![
                MockScriptEntry::Error(explicit_429(Duration::from_secs(7))),
                answer("openai"),
            ],
        );
        let backup = always("anthropic", transient("anthropic", 503));
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("openai"));
        assert_eq!(response.content, "answer from openai");
        assert_eq!(primary.call_count(), 2, "the same provider is retried once");
        assert_eq!(backup.call_count(), 0, "the backup is never touched");
        let gaps = primary.gaps();
        assert_eq!(gaps.len(), 1);
        assert!(
            gaps[0] >= Duration::from_secs(7),
            "the retry delay is a minimum, got {:?}",
            gaps[0]
        );
        // The waiter spread only ever adds, and at most min(wait / 10, 1 s).
        assert!(
            gaps[0] <= Duration::from_millis(7_700) + TIMER_ROUNDING,
            "got {:?}",
            gaps[0]
        );
    }

    /// Run one chain over three delay-less 429s and return the three gaps between sends.
    async fn delayless_gaps(jitter: fn() -> f64) -> Vec<Duration> {
        let primary = scripted(
            "openai",
            vec![
                MockScriptEntry::Error(LlmError::rate_limited(None)),
                MockScriptEntry::Error(LlmError::rate_limited(None)),
                MockScriptEntry::Error(LlmError::rate_limited(None)),
                answer("openai"),
            ],
        );
        let backup = provider_timed("anthropic");
        let (wiring, _port) = pacing(jitter, Duration::from_secs(60));
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("openai"));
        assert_eq!(primary.call_count(), 4);
        assert_eq!(backup.call_count(), 0);
        primary.gaps()
    }

    fn provider_timed(name: &'static str) -> TimedProvider {
        TimedProvider::new(provider(name))
    }

    /// Back-off with jitter on a hop: with jitter pinned near 1.0 and no spread, the gaps double
    /// from the base; with the real random jitter and spread, every gap stays inside its bounds
    /// and the gaps are not all identical.
    #[tokio::test(start_paused = true)]
    async fn delayless_429s_on_a_hop_back_off_with_jitter() {
        // Pinned: gate_n = max(base, floor(base * 2^(n-1) * 0.999)).
        let _no_spread = pin_waiter_spread(0.0);
        let gaps = delayless_gaps(|| 0.999).await;
        assert_eq!(gaps.len(), 3);
        assert_close(gaps[0], BASE, "first gap is the base");
        assert_close(gaps[1], Duration::from_millis(999), "second gap doubles");
        assert_close(
            gaps[2],
            Duration::from_millis(1_998),
            "third gap doubles again",
        );
        drop(_no_spread);

        // Real jitter and real spread, 20 repetitions.
        let mut third_gaps = Vec::new();
        for repetition in 0..20 {
            let gaps = delayless_gaps(rand::random::<f64>).await;
            assert_eq!(gaps.len(), 3, "repetition {repetition}");
            for (n, gap) in gaps.iter().enumerate() {
                let ceiling = (BASE * (1u32 << n)).min(MAX_BACKOFF);
                let spread = (ceiling / 10).min(Duration::from_secs(1));
                assert!(
                    *gap >= BASE,
                    "repetition {repetition} gap {n}: {gap:?} is below the base"
                );
                assert!(
                    *gap <= ceiling + spread + TIMER_ROUNDING,
                    "repetition {repetition} gap {n}: {gap:?} is above {ceiling:?} plus spread"
                );
            }
            third_gaps.push(gaps[2]);
        }
        third_gaps.sort();
        third_gaps.dedup();
        assert!(
            third_gaps.len() > 1,
            "jitter must spread the gaps, not repeat one value"
        );
    }

    /// D-03 boundary: paced while `elapsed + next gate <= budget`, hop once it would exceed it,
    /// and one `FallbackHop` event for the hop, not one per paced retry.
    #[tokio::test(start_paused = true)]
    async fn pace_budget_boundary_paces_at_the_budget_and_hops_beyond_it() {
        let _no_spread = pin_waiter_spread(0.0);
        let primary = always("openai", explicit_429(Duration::from_secs(30)));
        let backup = provider_timed("anthropic");
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let sink = Arc::new(RecordingSink::default());
        let adapter = paced_chain(&[&primary, &backup], &wiring)
            .with_trace_emitter(emitter_over(sink.clone()));

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(primary.call_count(), 3, "paced at 0 s, 30 s and 60 s");
        let offsets = primary.offsets();
        assert_close(offsets[0], Duration::ZERO, "first send");
        assert_close(offsets[1], Duration::from_secs(30), "second send");
        assert_close(
            offsets[2],
            Duration::from_secs(60),
            "third send, at the budget",
        );
        assert_eq!(backup.call_count(), 1);

        let hops = sink.hops_eventually(1).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let hops_after = sink.hops_eventually(1).await;
        assert_eq!(hops, hops_after, "no further hop events arrive");
        assert_eq!(
            hops,
            vec![(None, "openai".to_string(), "anthropic".to_string())],
            "exactly one hop event, at the hop and not per paced retry"
        );
    }

    /// D-03: a 5xx still hops at once with cadence attached.
    #[tokio::test(start_paused = true)]
    async fn a_5xx_still_hops_immediately_with_cadence_attached() {
        let primary = always("openai", transient("openai", 503));
        let backup = provider_timed("anthropic");
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let start = Instant::now();
        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(primary.call_count(), 1, "a 503 is never retried on the hop");
        assert_eq!(backup.call_count(), 1);
        assert_eq!(start.elapsed(), Duration::ZERO, "the hop is immediate");
    }

    /// The `generate_stream` mirror of the success-criterion-2 test.
    #[tokio::test(start_paused = true)]
    async fn streaming_429_on_a_hop_paces_before_hopping() {
        let primary = scripted(
            "openai",
            vec![
                MockScriptEntry::Error(explicit_429(Duration::from_secs(7))),
                answer("openai"),
            ],
        );
        let backup = provider_timed("anthropic");
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let stream = adapter.generate_stream(request()).await.unwrap();
        let items = collect(stream).await;

        assert_eq!(items[0].as_ref().unwrap(), "answer from openai");
        assert_eq!(primary.call_count(), 2);
        assert_eq!(backup.call_count(), 0);
        let gaps = primary.gaps();
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0] >= Duration::from_secs(7), "got {:?}", gaps[0]);
    }

    /// `AllProvidersFailed.attempts` lists every attempt in order, paced retries included.
    #[tokio::test(start_paused = true)]
    async fn all_providers_failed_lists_paced_attempts() {
        let _no_spread = pin_waiter_spread(0.0);
        let primary = always("openai", explicit_429(Duration::from_secs(30)));
        let backup = always("anthropic", explicit_429(Duration::from_secs(30)));
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let result = adapter.generate(request()).await;

        let Err(LlmError::AllProvidersFailed { attempts, last }) = result else {
            panic!("expected AllProvidersFailed, got {result:?}");
        };
        let providers: Vec<&str> = attempts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            providers,
            [
                "openai",
                "openai",
                "openai",
                "anthropic",
                "anthropic",
                "anthropic"
            ],
            "paced retries are attempts too, in order"
        );
        assert!(matches!(*last, LlmError::RateLimitExceeded { .. }));
        assert_eq!((primary.call_count(), backup.call_count()), (3, 3));
    }

    /// A chain with no cadence attached keeps the legacy rule: a 429 hops at once.
    #[tokio::test(start_paused = true)]
    async fn chain_without_cadence_still_hops_on_a_429() {
        let primary = always("openai", explicit_429(Duration::from_secs(7)));
        let backup = provider_timed("anthropic");
        let adapter = FallbackLlmAdapter::new(vec![
            Arc::new(primary.clone()) as Arc<dyn LlmPort>,
            Arc::new(backup.clone()) as Arc<dyn LlmPort>,
        ])
        .unwrap();

        let start = Instant::now();
        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(primary.call_count(), 1);
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// `with_cadence` wraps every hop under its own provider name: a 429 on hop 2 gates
    /// `(hop2, model)`, not `(hop1, model)` and not `("fallback", model)`.
    #[tokio::test(start_paused = true)]
    async fn with_cadence_wraps_every_hop_under_its_own_provider_name() {
        let first = always("openai", transient("openai", 503));
        let second = always("anthropic", explicit_429(Duration::from_secs(120)));
        let (wiring, port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = paced_chain(&[&first, &second], &wiring);

        let result = adapter.generate(request()).await;

        assert!(
            matches!(result, Err(LlmError::AllProvidersFailed { .. })),
            "{result:?}"
        );
        let gate = |provider: &'static str| {
            let port = port.clone();
            async move {
                port.gate(&CadenceKey::new(provider, "mock-model"))
                    .await
                    .unwrap()
            }
        };
        assert_eq!(gate("anthropic").await.wait(), Duration::from_secs(120));
        assert!(gate("openai").await.is_clear(), "hop 1 was never gated");
        assert!(
            gate("fallback").await.is_clear(),
            "the chain is never gated"
        );
    }

    /// A gate that cannot be read never turns into a hot retry loop: with nothing to pace
    /// against, the 429 reaches the hop rule.
    #[tokio::test(start_paused = true)]
    async fn a_broken_gate_hops_instead_of_retrying_hot() {
        struct BrokenPort;

        #[async_trait]
        impl CadencePort for BrokenPort {
            async fn gate(&self, _key: &CadenceKey) -> Result<GateReading, CadenceError> {
                Err(CadenceError::Backend {
                    message: "gate unreadable".to_string(),
                })
            }

            async fn record_rate_limited(
                &self,
                _key: &CadenceKey,
                _retry_after: Option<Duration>,
            ) -> Result<GateReading, CadenceError> {
                Err(CadenceError::Backend {
                    message: "write refused".to_string(),
                })
            }

            async fn record_success(&self, _key: &CadenceKey) -> Result<(), CadenceError> {
                Ok(())
            }

            async fn try_lock(
                &self,
                _key: &LockKey,
                _ttl: Duration,
            ) -> Result<Option<FencingToken>, CadenceError> {
                Ok(None)
            }

            async fn unlock(
                &self,
                _key: &LockKey,
                _token: &FencingToken,
            ) -> Result<bool, CadenceError> {
                Ok(false)
            }
        }

        let primary = always("openai", LlmError::rate_limited(None));
        let backup = provider_timed("anthropic");
        let wiring = CadenceWiring::new(Arc::new(BrokenPort), CadenceSettings::default());
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let start = Instant::now();
        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(primary.call_count(), 1);
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    /// A gate beyond `max_wait` is refused by the decorator without sleeping; the chain must not
    /// mistake that refusal for something to pace against (it would spin without time passing).
    #[tokio::test(start_paused = true)]
    async fn a_gate_beyond_max_wait_hops_instead_of_spinning() {
        let primary = always("openai", explicit_429(Duration::from_secs(100)));
        let backup = provider_timed("anthropic");
        let port = Arc::new(InMemoryCadence::new(CadencePolicy::default()));
        // A budget far above `max_wait`: only the max_wait guard keeps the chain from retrying.
        let settings = CadenceSettings::new(
            Duration::from_secs(10),
            MAX_BACKOFF,
            Duration::from_secs(3_600),
        );
        let wiring = CadenceWiring::new(port, settings);
        let adapter = paced_chain(&[&primary, &backup], &wiring);

        let start = Instant::now();
        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(primary.call_count(), 1);
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    use super::test_support::ProfiledStubPort;
    use paladin_ports::output::llm_port::{ModelProfile, ParamSupport, StopSupport};

    fn hop(name: &'static str, profile: Option<ModelProfile>) -> Arc<dyn LlmPort> {
        Arc::new(ProfiledStubPort::new(name, "m", profile))
    }

    /// ADR-0061 (g): the chain's profile is the intersection of its hops' profiles; a hop with no
    /// profile is skipped; a chain where no hop declares one has none; and the order the hops are
    /// listed in does not change the answer.
    #[test]
    fn fallback_model_profile_is_the_intersection_of_hops() {
        let p1 = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            .with_stop_sequences(StopSupport::Supported { max: Some(16) })
            .with_context_window(200_000)
            .with_json_object(true);
        let p2 = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_stop_sequences(StopSupport::Supported { max: Some(4) })
            .with_context_window(128_000)
            .with_json_object(true);
        let expected = p1.intersect(&p2);
        assert_eq!(expected.temperature, ParamSupport::Range(0.0, 1.0));

        let chain = FallbackLlmAdapter::new(vec![
            hop("openai", Some(p1.clone())),
            hop("anthropic", Some(p2.clone())),
        ])
        .expect("chain");
        assert_eq!(chain.model_profile("m"), Some(expected.clone()));

        // Order independent.
        let reversed = FallbackLlmAdapter::new(vec![
            hop("anthropic", Some(p2.clone())),
            hop("openai", Some(p1.clone())),
        ])
        .expect("chain");
        assert_eq!(reversed.model_profile("m"), Some(expected.clone()));

        // A hop with no profile is skipped, not treated as "nothing sendable".
        let with_blank = FallbackLlmAdapter::new(vec![
            hop("openai", Some(p1)),
            hop("third-party", None),
            hop("anthropic", Some(p2)),
        ])
        .expect("chain");
        assert_eq!(with_blank.model_profile("m"), Some(expected));

        // No hop declares one: no profile.
        let none = FallbackLlmAdapter::new(vec![hop("a", None), hop("b", None)]).expect("chain");
        assert_eq!(none.model_profile("m"), None);

        // A single-hop chain returns that hop's profile unchanged.
        let single_profile = ModelProfile::new().with_temperature(ParamSupport::Fixed(1.0));
        let single = FallbackLlmAdapter::new(vec![hop("only", Some(single_profile.clone()))])
            .expect("chain");
        assert_eq!(single.model_profile("m"), Some(single_profile));
    }

    /// G-2 item 3: a hop that reports no profile can never be said to take an explicit effort, so
    /// the chain's profile reports `Unmeasured` effort (and no verification date) as soon as any
    /// hop answers `None`, instead of advertising the reporting hops' intersection.
    #[test]
    fn a_chain_profile_is_effort_unmeasured_when_any_hop_reports_none() {
        use paladin_core::platform::container::reasoning::ReasoningEffort;
        use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};

        let measured = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            .with_effort(EffortSupport::Supported(EffortLevels::new([
                ReasoningEffort::Low,
                ReasoningEffort::High,
            ])))
            .with_effort_verified_on("2026-10-21");

        // Every hop reports: today's intersection, effort and its date included.
        let all = FallbackLlmAdapter::new(vec![
            hop("a", Some(measured.clone())),
            hop("b", Some(measured.clone())),
        ])
        .expect("chain");
        let profile = all.model_profile("m").expect("both hops report");
        assert!(matches!(profile.effort, EffortSupport::Supported(_)));
        assert_eq!(profile.effort, measured.clone().intersect(&measured).effort);

        // One hop reports nothing: effort fails closed.
        let some = FallbackLlmAdapter::new(vec![
            hop("a", Some(measured.clone())),
            hop("third-party", None),
        ])
        .expect("chain");
        let profile = some.model_profile("m").expect("one hop reports");
        assert_eq!(profile.effort, EffortSupport::Unmeasured);
        assert_eq!(profile.effort_verified_on, None);

        // The same whichever side the profile-less hop sits on, and for a single reporting hop.
        let reversed = FallbackLlmAdapter::new(vec![
            hop("third-party", None),
            hop("a", Some(measured.clone())),
        ])
        .expect("chain");
        let profile = reversed.model_profile("m").expect("one hop reports");
        assert_eq!(profile.effort, EffortSupport::Unmeasured);
        assert_eq!(profile.effort_verified_on, None);
    }

    /// The sampling half of the chain profile is unchanged by the effort fail-closed rule: a hop
    /// with no profile is still skipped for sampling (each hop's own request builder applies its
    /// own), so the reporting hops' intersection still bounds temperature and the rest.
    #[test]
    fn a_chain_profile_keeps_sampling_from_the_hops_that_report_one() {
        let p1 = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 2.0))
            .with_stop_sequences(StopSupport::Supported { max: Some(16) })
            .with_context_window(200_000);
        let p2 = ModelProfile::new()
            .with_temperature(ParamSupport::Range(0.0, 1.0))
            .with_stop_sequences(StopSupport::Supported { max: Some(4) })
            .with_context_window(128_000);
        let expected = p1.intersect(&p2);

        let chain = FallbackLlmAdapter::new(vec![
            hop("openai", Some(p1)),
            hop("third-party", None),
            hop("anthropic", Some(p2)),
        ])
        .expect("chain");
        let profile = chain.model_profile("m").expect("two hops report");

        assert_eq!(profile.temperature, expected.temperature);
        assert_eq!(profile.stop_sequences, expected.stop_sequences);
        assert_eq!(profile.context_window, expected.context_window);
        assert_eq!(profile.temperature, ParamSupport::Range(0.0, 1.0));
    }

    // -----------------------------------------------------------------------
    // Per-step models, the served model and the refusal opt-in (plan 50-14:
    // D-15, D-10, D-00h, PARITY-06)
    // -----------------------------------------------------------------------

    use super::test_support::ProfiledStubPort as StubProfile;

    fn step(port: &MockLlmAdapter, model: &str) -> FallbackStep {
        FallbackStep::new(Arc::new(port.clone()) as Arc<dyn LlmPort>, model)
    }

    fn timed_step(port: &TimedProvider, model: &str) -> FallbackStep {
        FallbackStep::new(Arc::new(port.clone()) as Arc<dyn LlmPort>, model)
    }

    fn served_model(response: &LlmResponse) -> Option<&str> {
        response
            .metadata
            .get(SERVED_MODEL_METADATA_KEY)
            .map(String::as_str)
    }

    fn sent_models(port: &MockLlmAdapter) -> Vec<String> {
        port.requests()
            .into_iter()
            .map(|request| request.model)
            .collect()
    }

    fn refusal(provider: &str, model: &str) -> LlmError {
        LlmError::refusal(provider, model, "I can't help with that.")
    }

    #[test]
    fn the_served_model_key_is_namespaced() {
        assert_eq!(SERVED_MODEL_METADATA_KEY, "paladin.served_model");
    }

    /// D-15 / PARITY-06: hop 2 is sent its own model, and the response names the hop that served
    /// and the model it was sent.
    #[tokio::test]
    async fn from_steps_sends_each_hop_its_own_model() {
        let anthropic = failing("anthropic", transient("anthropic", 503));
        let kimi = provider("kimi");
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&anthropic, "m1"), step(&kimi, "m2")])
                .unwrap();

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(sent_models(&anthropic), ["m1"]);
        assert_eq!(sent_models(&kimi), ["m2"]);
        assert_eq!(response.content, "answer from kimi");
        assert_eq!(
            response
                .metadata
                .get(SERVED_BY_METADATA_KEY)
                .map(String::as_str),
            Some("kimi")
        );
        assert_eq!(served_model(&response), Some("m2"));
    }

    #[tokio::test]
    async fn from_steps_sends_each_hop_its_own_model_on_the_stream() {
        let anthropic = failing("anthropic", transient("anthropic", 503));
        let kimi = MockLlmAdapter::new()
            .with_provider_name("kimi")
            .with_stream_items(vec![Ok("Hello".to_string())]);
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&anthropic, "m1"), step(&kimi, "m2")])
                .unwrap();

        let items = collect(adapter.generate_stream(request()).await.unwrap()).await;

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].as_deref().unwrap(), "Hello");
        assert_eq!(sent_models(&anthropic), ["m1"]);
        assert_eq!(sent_models(&kimi), ["m2"]);
    }

    /// Phase 50, owner reading R-1 (option-b): a streamed fallback call names the hop that
    /// served it on the terminal chunk -- `paladin.served_by` and `paladin.served_model`, the
    /// same two keys `generate` stamps -- and stamps no content chunk.
    #[tokio::test]
    async fn the_fallback_stream_stamps_served_by_and_served_model_on_its_terminal_chunk() {
        use futures::StreamExt;
        let anthropic = failing("anthropic", transient("anthropic", 503));
        let kimi = MockLlmAdapter::new()
            .with_provider_name("kimi")
            .with_response("Hello");
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&anthropic, "m1"), step(&kimi, "m2")])
                .unwrap();

        let stream = adapter.generate_stream(request()).await.unwrap();
        let chunks: Vec<StreamingResponse> = Box::into_pin(stream)
            .map(|item| item.expect("the scripted stream has no error item"))
            .collect()
            .await;

        let terminals: Vec<&StreamingResponse> = chunks
            .iter()
            .filter(|chunk| chunk.finish_reason.is_some())
            .collect();
        assert_eq!(terminals.len(), 1, "{chunks:?}");
        let terminal = terminals[0];
        assert_eq!(
            terminal
                .metadata
                .get(SERVED_BY_METADATA_KEY)
                .map(String::as_str),
            Some("kimi")
        );
        assert_eq!(
            terminal
                .metadata
                .get(SERVED_MODEL_METADATA_KEY)
                .map(String::as_str),
            Some("m2")
        );
        for chunk in chunks.iter().filter(|c| c.finish_reason.is_none()) {
            assert!(chunk.metadata.is_empty(), "content chunk: {chunk:?}");
        }
    }

    /// A `new` chain is unchanged: every hop is sent the request's own model, and the served
    /// model names it.
    #[tokio::test]
    async fn a_new_chain_keeps_sending_every_hop_the_requests_model() {
        let first = failing("openai", transient("openai", 503));
        let second = provider("anthropic");
        let adapter = chain(&[first.clone(), second.clone()]);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(sent_models(&first), ["mock-model"]);
        assert_eq!(sent_models(&second), ["mock-model"]);
        assert_eq!(served_model(&response), Some("mock-model"));
        assert_eq!(served_by(&response), Some("anthropic"));
    }

    /// Two consecutive steps on one port with different models are two hops; the same model on
    /// two steps is still two hops.
    #[tokio::test]
    async fn same_port_different_models_are_two_hops() {
        let port = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_script(vec![
                MockScriptEntry::Error(transient("openai", 503)),
                MockScriptEntry::Text("second model answered".to_string()),
            ]);
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&port, "gpt-a"), step(&port, "gpt-b")])
                .unwrap();

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(sent_models(&port), ["gpt-a", "gpt-b"]);
        assert_eq!(response.content, "second model answered");
        assert_eq!(served_model(&response), Some("gpt-b"));

        let twice = MockLlmAdapter::new()
            .with_provider_name("openai")
            .with_script(vec![
                MockScriptEntry::Error(transient("openai", 503)),
                MockScriptEntry::Text("again".to_string()),
            ]);
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&twice, "gpt-a"), step(&twice, "gpt-a")])
                .unwrap();
        let response = adapter.generate(request()).await.unwrap();
        assert_eq!(sent_models(&twice), ["gpt-a", "gpt-a"], "two hops, not one");
        assert_eq!(served_model(&response), Some("gpt-a"));
    }

    /// Steps are tried strictly in declared order; the served metadata names the hop that
    /// succeeded, not the first or the last configured.
    #[tokio::test]
    async fn steps_are_tried_in_order_and_the_server_is_recorded() {
        let first = failing("alpha", transient("alpha", 503));
        let second = failing("bravo", LlmError::Timeout("1s".to_string()));
        let third = provider("charlie");
        let fourth = provider("delta");
        let adapter = FallbackLlmAdapter::from_steps(vec![
            step(&first, "m-a"),
            step(&second, "m-b"),
            step(&third, "m-c"),
            step(&fourth, "m-d"),
        ])
        .unwrap();

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(first.call_count(), 1);
        assert_eq!(second.call_count(), 1);
        assert_eq!(third.call_count(), 1);
        assert_eq!(fourth.call_count(), 0, "the later step is never reached");
        assert_eq!(served_by(&response), Some("charlie"));
        assert_eq!(served_model(&response), Some("m-c"));
    }

    #[test]
    fn from_steps_rejects_an_empty_chain_and_an_empty_model() {
        assert_eq!(
            FallbackLlmAdapter::from_steps(Vec::new()).unwrap_err(),
            FallbackChainError::EmptyChain
        );

        let good = provider("openai");
        let bad = provider("kimi");
        let err = FallbackLlmAdapter::from_steps(vec![step(&bad, "  ")]).unwrap_err();
        assert_eq!(err, FallbackChainError::EmptyStepModel { index: 0 });
        assert!(err.to_string().contains('0'), "{err}");

        let err =
            FallbackLlmAdapter::from_steps(vec![step(&good, "m1"), step(&bad, "")]).unwrap_err();
        assert_eq!(err, FallbackChainError::EmptyStepModel { index: 1 });
    }

    #[test]
    fn a_step_debug_names_the_provider_and_the_model() {
        let kimi = provider("kimi");
        let rendered = format!("{:?}", step(&kimi, "kimi-k2"));
        assert!(rendered.contains("kimi"), "{rendered}");
        assert!(rendered.contains("kimi-k2"), "{rendered}");
    }

    /// D-10: a refusal short-circuits by default; hop 2 is never called.
    #[tokio::test]
    async fn refusal_stops_by_default() {
        let first = failing("anthropic", refusal("anthropic", "m1"));
        let second = provider("kimi");
        let adapter =
            FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")]).unwrap();

        let err = adapter.generate(request()).await.unwrap_err();

        assert!(matches!(err, LlmError::Refusal { .. }), "{err:?}");
        assert_eq!(second.call_count(), 0, "a refusal must not hop by default");

        let stream_err = adapter.generate_stream(request()).await.err().unwrap();
        assert!(
            matches!(stream_err, LlmError::Refusal { .. }),
            "{stream_err:?}"
        );
        assert_eq!(second.call_count(), 0);
    }

    /// D-10: with the opt-in, a refusal (and only a refusal) tries the next step.
    #[tokio::test]
    async fn hop_on_refusal_tries_the_next_step() {
        let first = failing("anthropic", refusal("anthropic", "m1"));
        let second = provider("kimi");
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(second.call_count(), 1);
        assert_eq!(served_by(&response), Some("kimi"));
        assert_eq!(served_model(&response), Some("m2"));

        // Setting it back off restores the default.
        let off = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true)
            .hop_on_refusal(false);
        assert!(matches!(
            off.generate(request()).await,
            Err(LlmError::Refusal { .. })
        ));
    }

    /// With the opt-in on, every other `Permanent` error still short-circuits.
    #[tokio::test]
    async fn hop_on_refusal_does_not_hop_past_other_permanent_errors() {
        let first = failing(
            "anthropic",
            LlmError::AuthenticationError("bad key".to_string()),
        );
        let second = provider("kimi");
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let err = adapter.generate(request()).await.unwrap_err();

        assert!(matches!(err, LlmError::AuthenticationError(_)), "{err:?}");
        assert_eq!(second.call_count(), 0);
    }

    /// With the opt-in, an exhausted chain lists every refusal and ends in `AllProvidersFailed`.
    #[tokio::test]
    async fn hop_on_refusal_exhaustion_lists_every_refusal() {
        let first = failing("anthropic", refusal("anthropic", "m1"));
        let second = failing("kimi", refusal("kimi", "m2"));
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let err = adapter.generate(request()).await.unwrap_err();

        let LlmError::AllProvidersFailed { attempts, last } = &err else {
            panic!("expected AllProvidersFailed, got {err:?}");
        };
        let names: Vec<&str> = attempts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["anthropic", "kimi"]);
        assert!(matches!(**last, LlmError::Refusal { .. }));
    }

    /// The streaming path honours the opt-in the same way.
    #[tokio::test]
    async fn hop_on_refusal_applies_to_the_stream() {
        let first = failing("anthropic", refusal("anthropic", "m1"));
        let second = MockLlmAdapter::new()
            .with_provider_name("kimi")
            .with_stream_items(vec![Ok("ok".to_string())]);
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let items = collect(adapter.generate_stream(request()).await.unwrap()).await;

        assert_eq!(items.len(), 1);
        assert_eq!(sent_models(&second), ["m2"]);
    }

    /// D-00h: a 429 on hop 1 gates `(hop-1 provider, hop-1 model)` and nothing else: not the
    /// request's model, not hop 2's model, not hop 2's provider.
    #[tokio::test(start_paused = true)]
    async fn cadence_is_keyed_on_each_steps_model() {
        let first = always("openai", explicit_429(Duration::from_secs(120)));
        let second = provider_timed("anthropic");
        let (wiring, port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = FallbackLlmAdapter::from_steps(vec![
            timed_step(&first, "m1"),
            timed_step(&second, "m2"),
        ])
        .unwrap()
        .with_cadence(&wiring);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("anthropic"));
        assert_eq!(served_model(&response), Some("m2"));
        let gate = |provider: &'static str, model: &'static str| {
            let port = port.clone();
            async move { port.gate(&CadenceKey::new(provider, model)).await.unwrap() }
        };
        assert_eq!(gate("openai", "m1").await.wait(), Duration::from_secs(120));
        assert!(
            gate("openai", "mock-model").await.is_clear(),
            "the request's model must not key hop 1"
        );
        assert!(gate("openai", "m2").await.is_clear());
        assert!(gate("anthropic", "m2").await.is_clear());
        assert!(gate("anthropic", "m1").await.is_clear());
    }

    /// D-00h: `pace_hop` reads the hop's own gate. Hop 1's 429 gate is `(openai, m1)`; had the
    /// chain read `(openai, request-model)` it would see a clear gate and hop instead of pacing.
    #[tokio::test(start_paused = true)]
    async fn a_429_is_paced_on_the_hops_own_model() {
        let first = scripted(
            "openai",
            vec![
                MockScriptEntry::Error(explicit_429(Duration::from_secs(7))),
                answer("openai"),
            ],
        );
        let second = provider_timed("anthropic");
        let (wiring, _port) = pacing(|| 0.999, Duration::from_secs(60));
        let adapter = FallbackLlmAdapter::from_steps(vec![
            timed_step(&first, "m1"),
            timed_step(&second, "m2"),
        ])
        .unwrap()
        .with_cadence(&wiring);

        let response = adapter.generate(request()).await.unwrap();

        assert_eq!(served_by(&response), Some("openai"));
        assert_eq!(served_model(&response), Some("m1"));
        assert_eq!(first.call_count(), 2, "paced on the same hop, not hopped");
        assert_eq!(second.call_count(), 0);
        assert_eq!(
            first
                .inner
                .requests()
                .iter()
                .map(|request| request.model.as_str())
                .collect::<Vec<_>>(),
            ["m1", "m1"]
        );
    }

    /// The chain's profile is the intersection of each step's profile for that step's model.
    #[test]
    fn from_steps_model_profile_uses_each_steps_own_model() {
        let m1 = ModelProfile::new().with_temperature(ParamSupport::Range(0.0, 2.0));
        let m2 = ModelProfile::new().with_temperature(ParamSupport::Fixed(1.0));
        let first = Arc::new(StubProfile::new("openai", "m1", Some(m1)));
        let second = Arc::new(StubProfile::new("anthropic", "m2", Some(m2)));
        let adapter = FallbackLlmAdapter::from_steps(vec![
            FallbackStep::new(first.clone(), "m1"),
            FallbackStep::new(second.clone(), "m2"),
        ])
        .unwrap();

        // The argument is the request's model; a steps chain ignores it.
        let profile = adapter.model_profile("whatever-the-request-says").unwrap();

        assert_eq!(profile.temperature, ParamSupport::Fixed(1.0));
        assert_eq!(first.asked(), ["m1"]);
        assert_eq!(second.asked(), ["m2"]);
    }

    // ---- G-50-5a remainder (a), plan 50-30: hopped-past bills ----

    use paladin_ports::output::llm_port::{BilledUsage, TokenUsage};
    use paladin_ports::output::spend_scope::{HOPPED_BILLS, HoppedBills};

    fn billed_refusal(provider: &str, model: &str, usage: TokenUsage) -> LlmError {
        refusal(provider, model).with_billed_usage(BilledUsage::new(model, usage))
    }

    /// A billed refusal the chain hops past is recorded, unpriced, into the call's collector;
    /// the served response is unchanged.
    #[tokio::test]
    async fn a_billed_hop_is_recorded_for_settlement() {
        let first = failing(
            "anthropic",
            billed_refusal("anthropic", "m1", TokenUsage::new(40, 2)),
        );
        let second = provider("kimi");
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let bills = HoppedBills::new();
        let response = HOPPED_BILLS
            .scope(bills.clone(), adapter.generate(request()))
            .await
            .unwrap();

        assert_eq!(served_model(&response), Some("m2"));
        assert_eq!(
            bills.take(),
            vec![BilledUsage::new("m1", TokenUsage::new(40, 2))]
        );
    }

    /// An unbilled hop (a 5xx, an unbilled refusal) records nothing.
    #[tokio::test]
    async fn an_unbilled_hop_records_nothing() {
        let first = failing("openai", transient("openai", 503));
        let second = failing("anthropic", refusal("anthropic", "m2"));
        let third = provider("kimi");
        let adapter = FallbackLlmAdapter::from_steps(vec![
            step(&first, "m1"),
            step(&second, "m2"),
            step(&third, "m3"),
        ])
        .unwrap()
        .hop_on_refusal(true);

        let bills = HoppedBills::new();
        HOPPED_BILLS
            .scope(bills.clone(), adapter.generate(request()))
            .await
            .unwrap();

        assert!(bills.take().is_empty());
    }

    /// On exhaustion every hop but the last is recorded; the last hop's bill rides on
    /// `AllProvidersFailed` itself, so nothing is counted twice.
    #[tokio::test]
    async fn exhaustion_records_every_hop_but_the_last() {
        let first = failing(
            "anthropic",
            billed_refusal("anthropic", "m1", TokenUsage::new(40, 2)),
        );
        let second = failing("kimi", billed_refusal("kimi", "m2", TokenUsage::new(30, 3)));
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let bills = HoppedBills::new();
        let err = HOPPED_BILLS
            .scope(bills.clone(), adapter.generate(request()))
            .await
            .unwrap_err();

        assert!(
            matches!(err, LlmError::AllProvidersFailed { .. }),
            "{err:?}"
        );
        assert_eq!(
            err.billed_usage().map(|bill| bill.model.as_str()),
            Some("m2")
        );
        assert_eq!(
            bills.take(),
            vec![BilledUsage::new("m1", TokenUsage::new(40, 2))]
        );
    }

    /// The streamed path records a billed hop the same way.
    #[tokio::test]
    async fn a_billed_hop_on_the_stream_is_recorded() {
        let first = failing(
            "anthropic",
            billed_refusal("anthropic", "m1", TokenUsage::new(40, 2)),
        );
        let second = MockLlmAdapter::new()
            .with_provider_name("kimi")
            .with_stream_items(vec![Ok("ok".to_string())]);
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);

        let bills = HoppedBills::new();
        let stream = HOPPED_BILLS
            .scope(bills.clone(), adapter.generate_stream(request()))
            .await
            .unwrap();
        assert_eq!(collect(stream).await.len(), 1);
        assert_eq!(
            bills.take(),
            vec![BilledUsage::new("m1", TokenUsage::new(40, 2))]
        );
    }

    /// With no collector installed the chain behaves exactly as before, and the lost bill is
    /// made visible: one warning per (provider, model), never a second.
    #[tokio::test]
    async fn a_billed_hop_with_no_collector_warns_once_per_provider_and_model() {
        let first = failing(
            "anthropic",
            billed_refusal("anthropic", "m1", TokenUsage::new(40, 2)),
        );
        let second = provider("kimi");
        let adapter = FallbackLlmAdapter::from_steps(vec![step(&first, "m1"), step(&second, "m2")])
            .unwrap()
            .hop_on_refusal(true);
        let response = adapter.generate(request()).await.unwrap();
        assert_eq!(
            served_model(&response),
            Some("m2"),
            "the chain is unchanged"
        );

        let warnings = HopBillWarnings::new(3);
        let bill = BilledUsage::new("m1", TokenUsage::new(40, 2));
        assert!(
            !carry_hopped_bill(&warnings, "anthropic", &bill),
            "nothing recorded"
        );
        assert!(
            !warnings.should_warn("anthropic", "m1"),
            "the first call warned"
        );
        assert!(
            warnings.should_warn("anthropic", "m9"),
            "another model warns once"
        );
        assert!(
            warnings.should_warn("kimi", "m1"),
            "another provider warns once"
        );
        assert!(
            !warnings.should_warn("openai", "m1"),
            "past the capacity nothing more is held"
        );

        // Inside a collector the bill is recorded and nothing is warned.
        let bills = HoppedBills::new();
        let fresh = HopBillWarnings::new(2);
        let recorded = HOPPED_BILLS.sync_scope(bills.clone(), || {
            carry_hopped_bill(&fresh, "anthropic", &bill)
        });
        assert!(recorded);
        assert!(fresh.should_warn("anthropic", "m1"), "no warning was spent");
        assert_eq!(bills.take(), vec![bill]);
    }
}

/// Phase 51 (plan 51-07, D-08, Pitfall 11): the chain resolves a request's reasoning effort
/// against every hop's own profile before the first call, and the serving hop's effort keys
/// survive the chain's own stamping.
#[cfg(all(test, feature = "mock"))]
mod reasoning_preflight_tests {
    use super::*;
    use crate::mock::MockLlmAdapter;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_core::platform::container::reasoning::{
        REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY, ReasoningEffort,
        ReasoningFallback,
    };
    use paladin_ports::output::model_profile::{EffortLevels, EffortSupport};

    /// A hop that reports a scripted effort profile for every model, counts its calls through
    /// the wrapped mock, and -- like a real adapter -- stamps the requested and effective effort
    /// on what it serves.
    struct EffortHop {
        inner: MockLlmAdapter,
        profile: Option<ModelProfile>,
    }

    impl EffortHop {
        fn new(inner: MockLlmAdapter, support: Option<EffortSupport>) -> Arc<Self> {
            Arc::new(Self {
                inner,
                profile: support.map(|support| ModelProfile::new().with_effort(support)),
            })
        }

        fn stamps(&self, request: &LlmRequest) -> Vec<(&'static str, String)> {
            let (Some(requested), Some(profile)) = (request.reasoning, &self.profile) else {
                return Vec::new();
            };
            let fallback = request.reasoning_fallback.unwrap_or_default();
            match profile.effort.resolve(requested, fallback) {
                Ok(resolved) => vec![
                    (
                        REASONING_REQUESTED_METADATA_KEY,
                        resolved.requested.to_string(),
                    ),
                    (
                        REASONING_EFFECTIVE_METADATA_KEY,
                        resolved.effective.to_string(),
                    ),
                ],
                Err(_) => Vec::new(),
            }
        }
    }

    #[async_trait]
    impl LlmPort for EffortHop {
        async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
            let stamps = self.stamps(&request);
            let mut response = self.inner.generate(request).await?;
            for (key, value) in stamps {
                response.metadata.insert(key.to_string(), value);
            }
            Ok(response)
        }

        async fn generate_stream(
            &self,
            request: LlmRequest,
        ) -> Result<Box<dyn Stream<Item = StreamItem> + Send>, LlmError> {
            let stamps = self.stamps(&request);
            let raw = self.inner.generate_stream(request).await?;
            Ok(Box::new(Box::into_pin(raw).map(move |item| {
                item.map(|chunk| {
                    if chunk.finish_reason.is_none() {
                        return chunk;
                    }
                    stamps.iter().fold(chunk, |chunk, (key, value)| {
                        chunk.with_metadata(*key, value.as_str())
                    })
                })
            })))
        }

        async fn validate_model(&self, model: &str) -> Result<bool, LlmError> {
            self.inner.validate_model(model).await
        }

        async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
            self.inner.get_available_models().await
        }

        fn get_provider_name(&self) -> &'static str {
            self.inner.get_provider_name()
        }

        fn get_capabilities(&self) -> ProviderCapabilities {
            self.inner.get_capabilities()
        }

        fn model_profile(&self, _model: &str) -> Option<ModelProfile> {
            self.profile.clone()
        }
    }

    fn levels(listed: impl IntoIterator<Item = ReasoningEffort>) -> Option<EffortSupport> {
        Some(EffortSupport::Supported(EffortLevels::new(listed)))
    }

    fn serving(name: &'static str) -> MockLlmAdapter {
        MockLlmAdapter::new()
            .with_provider_name(name)
            .with_response(format!("answer from {name}"))
    }

    fn down(name: &'static str) -> MockLlmAdapter {
        MockLlmAdapter::new()
            .with_provider_name(name)
            .with_error(LlmError::ProviderError {
                provider: name.to_string(),
                status: 503,
                message: "upstream unavailable".to_string(),
            })
    }

    fn chain(hops: Vec<Arc<EffortHop>>) -> FallbackLlmAdapter {
        FallbackLlmAdapter::new(hops.into_iter().map(|h| h as Arc<dyn LlmPort>).collect())
            .expect("a non-empty chain")
    }

    fn request(effort: Option<ReasoningEffort>) -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "quest".to_string(),
            context: None,
        }))
        .unwrap();
        let request = LlmRequest::new("mock-model", prompt);
        match effort {
            Some(effort) => request.with_reasoning(effort),
            None => request,
        }
    }

    /// (param, value, provider, "model|reason") of an `UnsupportedParameter`.
    fn refusal(error: LlmError) -> (String, String, String, String) {
        match error {
            LlmError::UnsupportedParameter {
                param,
                value,
                provider,
                model,
                reason,
                ..
            } => (param, value, provider, format!("{model}|{reason}")),
            other => panic!("expected UnsupportedParameter, got {other:?}"),
        }
    }

    async fn collect_terminal(
        stream: Box<dyn Stream<Item = StreamItem> + Send>,
    ) -> StreamingResponse {
        let mut stream = Box::into_pin(stream);
        while let Some(item) = stream.next().await {
            let chunk = item.expect("a healthy stream");
            if chunk.finish_reason.is_some() {
                return chunk;
            }
        }
        panic!("the stream ended without a terminal chunk");
    }

    #[tokio::test]
    async fn an_unmeasured_hop_refuses_the_whole_request_before_any_call() {
        let first = serving("hop-a");
        let second = serving("hop-b");
        let adapter = chain(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::High])),
            EffortHop::new(second.clone(), Some(EffortSupport::Unmeasured)),
        ]);

        let error = adapter
            .generate(request(Some(ReasoningEffort::High)))
            .await
            .expect_err("hop-b cannot honour the effort");
        let (param, value, provider, rest) = refusal(error);
        assert_eq!(param, "reasoning_effort");
        assert_eq!(value, "high");
        assert_eq!(provider, "hop-b");
        assert!(rest.starts_with("mock-model|"), "{rest}");
        assert!(rest.contains("fallback hop 1"), "{rest}");
        assert_eq!((first.call_count(), second.call_count()), (0, 0));

        let error = match adapter
            .generate_stream(request(Some(ReasoningEffort::High)))
            .await
        {
            Err(error) => error,
            Ok(_) => panic!("the streaming path must refuse too"),
        };
        assert_eq!(refusal(error).2, "hop-b");
        assert_eq!((first.call_count(), second.call_count()), (0, 0));
    }

    #[tokio::test]
    async fn a_hop_with_no_profile_at_all_refuses_an_explicit_effort() {
        let first = serving("hop-a");
        let second = serving("hop-b");
        let adapter = chain(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::High])),
            EffortHop::new(second.clone(), None),
        ]);

        let error = adapter
            .generate(request(Some(ReasoningEffort::High)))
            .await
            .expect_err("nothing was measured for hop-b");
        assert_eq!(refusal(error).2, "hop-b");
        assert_eq!((first.call_count(), second.call_count()), (0, 0));
    }

    #[tokio::test]
    async fn the_lowest_index_refusing_hop_is_named() {
        let adapter = chain(vec![
            EffortHop::new(serving("hop-a"), levels([ReasoningEffort::Low])),
            EffortHop::new(serving("hop-b"), levels([ReasoningEffort::Low])),
        ]);
        let error = adapter
            .generate(request(Some(ReasoningEffort::High)))
            .await
            .expect_err("neither hop lists high");
        let (_, _, provider, rest) = refusal(error);
        assert_eq!(provider, "hop-a");
        assert!(rest.contains("fallback hop 0"), "{rest}");
    }

    #[tokio::test]
    async fn a_later_hop_that_cannot_honour_the_effort_refuses_even_when_the_first_would_serve() {
        let first = serving("hop-a");
        let adapter = chain(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::High])),
            EffortHop::new(serving("hop-b"), levels([ReasoningEffort::Low])),
        ]);
        let error = adapter
            .generate(request(Some(ReasoningEffort::High)))
            .await
            .expect_err("hop-b lacks high");
        assert_eq!(refusal(error).2, "hop-b");
        assert_eq!(first.call_count(), 0, "the first hop must never be called");
    }

    #[tokio::test]
    async fn a_nearest_lower_request_passes_and_each_reached_hop_sends_its_own_resolution() {
        let first = down("hop-a");
        let second = serving("hop-b");
        let adapter = chain(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::High])),
            EffortHop::new(second.clone(), levels([ReasoningEffort::Medium])),
        ]);
        let high = request(Some(ReasoningEffort::High))
            .with_reasoning_fallback(ReasoningFallback::NearestLower);

        let response = adapter.generate(high).await.expect("hop-b serves");

        assert_eq!(first.call_count(), 1);
        assert_eq!(second.call_count(), 1);
        let meta = &response.metadata;
        assert_eq!(
            meta.get(SERVED_BY_METADATA_KEY).map(String::as_str),
            Some("hop-b")
        );
        assert_eq!(
            meta.get(SERVED_MODEL_METADATA_KEY).map(String::as_str),
            Some("mock-model")
        );
        assert_eq!(
            meta.get(REASONING_REQUESTED_METADATA_KEY)
                .map(String::as_str),
            Some("high")
        );
        assert_eq!(
            meta.get(REASONING_EFFECTIVE_METADATA_KEY)
                .map(String::as_str),
            Some("medium")
        );
        // The chain forwards the caller's request untouched; each hop re-admits it.
        assert_eq!(
            second.last_request().unwrap().reasoning,
            Some(ReasoningEffort::High)
        );
    }

    #[tokio::test]
    async fn the_streamed_terminal_chunk_keeps_the_effort_keys_beside_the_served_keys() {
        let adapter = chain(vec![
            EffortHop::new(down("hop-a"), levels([ReasoningEffort::High])),
            EffortHop::new(serving("hop-b"), levels([ReasoningEffort::Medium])),
        ]);
        let high = request(Some(ReasoningEffort::High))
            .with_reasoning_fallback(ReasoningFallback::NearestLower);

        let stream = adapter.generate_stream(high).await.expect("hop-b streams");
        let terminal = collect_terminal(stream).await;

        let meta = &terminal.metadata;
        assert_eq!(
            meta.get(SERVED_BY_METADATA_KEY).map(String::as_str),
            Some("hop-b")
        );
        assert!(meta.contains_key(SERVED_MODEL_METADATA_KEY));
        assert_eq!(
            meta.get(REASONING_REQUESTED_METADATA_KEY)
                .map(String::as_str),
            Some("high")
        );
        assert_eq!(
            meta.get(REASONING_EFFECTIVE_METADATA_KEY)
                .map(String::as_str),
            Some("medium")
        );
    }

    /// A DeepSeek hop over the unreachable `127.0.0.1:9`, whose `mock-model` lists exactly
    /// `[low, high]`, configured with the given provider-level defaults
    /// (`llm.deepseek.reasoning_effort` / `reasoning_fallback`). The pre-flight must settle every
    /// test using it before any request could leave the process.
    #[cfg(feature = "deepseek")]
    fn deepseek_hop(
        default_effort: Option<ReasoningEffort>,
        default_fallback: Option<ReasoningFallback>,
    ) -> Arc<dyn LlmPort> {
        use crate::deepseek::{DeepSeekAdapter, DeepSeekConfig};
        use crate::registry::ModelOverride;
        use std::collections::BTreeMap;

        let over = ModelOverride {
            effort: levels([ReasoningEffort::Low, ReasoningEffort::High]),
            ..ModelOverride::default()
        };
        let mut config = DeepSeekConfig::new("test-key".to_string(), "deepseek-flash".to_string())
            .with_base_url("http://127.0.0.1:9")
            .with_model_overrides(BTreeMap::from([("mock-model".to_string(), over)]));
        if let Some(effort) = default_effort {
            config = config.with_reasoning_effort(effort);
        }
        if let Some(fallback) = default_fallback {
            config = config.with_reasoning_fallback(fallback);
        }
        Arc::new(DeepSeekAdapter::new(config).expect("a valid override builds"))
    }

    /// WR-01, chain half: a hop's own `llm.<provider>.reasoning_fallback` is honoured before the
    /// first call. Request `medium` with no fallback: hop 0 lists it, hop 1 would substitute
    /// `low`, so the chain is admitted and hop 0 serves.
    #[cfg(feature = "deepseek")]
    #[tokio::test]
    async fn a_hops_own_provider_fallback_is_honoured_by_the_pre_flight() {
        let first = serving("hop-a");
        let adapter = FallbackLlmAdapter::new(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::Medium])) as Arc<dyn LlmPort>,
            deepseek_hop(None, Some(ReasoningFallback::NearestLower)),
        ])
        .expect("a chain");

        let response = adapter
            .generate(request(Some(ReasoningEffort::Medium)))
            .await
            .expect("hop 1 substitutes medium -> low, so the chain is admitted");

        assert_eq!(first.call_count(), 1);
        assert_eq!(
            response
                .metadata
                .get(SERVED_BY_METADATA_KEY)
                .map(String::as_str),
            Some("hop-a")
        );
    }

    /// A hop's own provider-default effort is pre-flighted even when the request sets none, so a
    /// hop that would refuse when reached refuses the chain before hop 0 is called.
    #[cfg(feature = "deepseek")]
    #[tokio::test]
    async fn a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none() {
        let first = serving("hop-a");
        let adapter = FallbackLlmAdapter::new(vec![
            EffortHop::new(first.clone(), levels([ReasoningEffort::Medium])) as Arc<dyn LlmPort>,
            deepseek_hop(Some(ReasoningEffort::Medium), None),
        ])
        .expect("a chain");

        let error = adapter
            .generate(request(None))
            .await
            .expect_err("hop 1 defaults to medium, which its model lacks");

        let (param, value, provider, rest) = refusal(error);
        assert_eq!(param, "reasoning_effort");
        assert_eq!(value, "medium");
        assert_eq!(provider, "deepseek");
        assert!(rest.contains("fallback hop 1: "), "{rest}");
        assert!(
            rest.ends_with("(set by `llm.deepseek.reasoning_effort`)"),
            "{rest}"
        );
        assert_eq!(first.call_count(), 0, "no hop may be called");

        // The streaming path refuses on the same verdict.
        assert!(
            adapter
                .generate_stream(request(None))
                .await
                .is_err_and(|e| matches!(e, LlmError::UnsupportedParameter { .. }))
        );
        assert_eq!(first.call_count(), 0);
    }

    /// D-08 / D-09 contract: for a grid of hop level sets, fallbacks and efforts, the chain's
    /// `check_reasoning` is `Ok` iff every hop's own `check_reasoning` is `Ok`, and a refusal
    /// names the lowest refusing index with that hop's own provider, model and reason.
    #[test]
    fn a_chain_check_reasoning_is_the_and_of_every_hops_check() {
        use ReasoningEffort::{High, Low, Max, Medium};

        let supports = || -> Vec<Option<EffortSupport>> {
            vec![
                levels([Low, High]),
                levels([Medium, High]),
                levels([High, Max]),
                Some(EffortSupport::Unmeasured),
                None,
            ]
        };
        let efforts = [None, Some(Low), Some(Medium), Some(High), Some(Max)];
        let fallbacks = [
            None,
            Some(ReasoningFallback::Refuse),
            Some(ReasoningFallback::NearestLower),
            Some(ReasoningFallback::NearestHigher),
        ];
        let (mut cases, mut refusals) = (0_u32, 0_u32);
        for first in supports() {
            for second in supports() {
                let hops = vec![
                    EffortHop::new(serving("hop-a"), first.clone()),
                    EffortHop::new(serving("hop-b"), second.clone()),
                ];
                let adapter = chain(hops.clone());
                for effort in efforts {
                    for fallback in fallbacks {
                        cases += 1;
                        let label = format!("{first:?} / {second:?}, {effort:?}/{fallback:?}");
                        let own: Vec<Result<(), LlmError>> = hops
                            .iter()
                            .map(|hop| hop.check_reasoning("mock-model", effort, fallback))
                            .collect();
                        let verdict = adapter.check_reasoning("mock-model", effort, fallback);
                        match own.iter().position(Result::is_err) {
                            None => assert!(verdict.is_ok(), "{label}: {verdict:?}"),
                            Some(index) => {
                                refusals += 1;
                                let (param, value, provider, rest) =
                                    refusal(verdict.expect_err(&label));
                                let (own_param, own_value, own_provider, own_rest) =
                                    refusal(own[index].clone().expect_err(&label));
                                assert_eq!(
                                    (param, value, provider),
                                    (own_param, own_value, own_provider),
                                    "{label}"
                                );
                                let (model, reason) = rest.split_once('|').expect("model|reason");
                                let (own_model, own_reason) =
                                    own_rest.split_once('|').expect("model|reason");
                                assert_eq!(model, own_model, "{label}");
                                assert_eq!(
                                    reason,
                                    format!("fallback hop {index}: {own_reason}"),
                                    "{label}"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 5 * 5 * 5 * 4);
        assert!(
            refusals > 0 && refusals < cases,
            "the grid must cover both verdicts"
        );
    }

    /// With no effort in force on any hop -- the request sets none and no hop has a provider
    /// default -- the pre-flight judges nothing: no profile is read and nothing is refused,
    /// whatever the hops report.
    #[tokio::test]
    async fn a_request_with_no_effort_in_force_on_any_hop_is_never_refused() {
        let first = serving("hop-a");
        let adapter = chain(vec![
            EffortHop::new(first.clone(), None),
            EffortHop::new(serving("hop-b"), Some(EffortSupport::Unmeasured)),
        ]);
        let response = adapter.generate(request(None)).await.expect("served");
        assert_eq!(
            response
                .metadata
                .get(SERVED_BY_METADATA_KEY)
                .map(String::as_str),
            Some("hop-a")
        );
        assert!(
            !response
                .metadata
                .contains_key(REASONING_REQUESTED_METADATA_KEY)
        );
        assert_eq!(first.call_count(), 1);
    }

    #[tokio::test]
    async fn a_step_chain_resolves_each_hop_against_its_own_step_model() {
        // The profile is looked up through the step's model, not the request's.
        let first = serving("hop-a");
        let adapter = FallbackLlmAdapter::from_steps(vec![
            FallbackStep::new(
                EffortHop::new(first.clone(), levels([ReasoningEffort::High])),
                "step-model-a",
            ),
            FallbackStep::new(
                EffortHop::new(serving("hop-b"), Some(EffortSupport::Unmeasured)),
                "step-model-b",
            ),
        ])
        .expect("steps");
        let error = adapter
            .generate(request(Some(ReasoningEffort::High)))
            .await
            .expect_err("hop-b is unmeasured");
        let (_, _, provider, rest) = refusal(error);
        assert_eq!(provider, "hop-b");
        assert!(rest.starts_with("step-model-b|"), "{rest}");
        assert_eq!(first.call_count(), 0);
    }
}
