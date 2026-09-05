//! # Model fallback — `FallbackLlmAdapter` (Doc 04 FT-FR-16/17, D-24…D-26)
//!
//! Composes an ordered chain of plain [`LlmPort`] implementors into one
//! [`LlmPort`]: every call starts at chain element 0 and moves to the next
//! element only when the current one fails with an error whose
//! [`LlmError::transience`] is `Transient` or `Unknown`. A `Permanent`
//! error short-circuits, so a malformed or rejected prompt is never re-sent
//! to every provider in the chain (T-25-35).
//!
//! ## No trait change, no policy type (D-24, RT-FR-09)
//!
//! The adapter adds nothing to [`LlmPort`]: provider names come from the
//! existing [`LlmPort::get_provider_name`], the adapter itself reports
//! `"fallback"`, [`LlmPort::get_capabilities`] answers with the first
//! element's, and [`LlmPort::validate_model`] / [`LlmPort::get_available_models`]
//! answer with the first element that returns `Ok`. It is a plain port so
//! Phase 26's retry/fallback middleware can delegate to it unchanged.
//!
//! ## The streaming first-chunk rule (D-25, T-25-33)
//!
//! [`LlmPort::generate_stream`] falls through only when the call itself
//! returns `Err`, or when the stream's **first** item is `Err` — the adapter
//! peeks that first item before handing the stream to the caller and still
//! delivers it. After any `Ok` chunk has been observed an error propagates
//! unchanged: a partial answer is never silently completed by a different
//! model.
//!
//! ## Per-hop observability (D-25, T-25-34)
//!
//! Each hop emits [`TraceEvent::FallbackHop`] (`node_id: None` — a port
//! below the superstep engine cannot know which node it serves) through the
//! optional [`FallbackLlmAdapter::with_trace_sink`] sink, plus a
//! `log::warn!` naming both providers. A successful response is stamped
//! with the serving provider under [`SERVED_BY_METADATA_KEY`] in the
//! existing `LlmResponse.metadata` map (D-26); `PaladinExecutionService`
//! copies it into `PaladinResult.served_by`.
//!
//! ## Circuit breakers sit ABOVE this adapter (D-24)
//!
//! The facade's `CircuitBreaker` (`src/infrastructure/resilience/circuit_breaker.rs`)
//! yields `PaladinError::CircuitBreakerOpen` *above* the port boundary and is
//! invisible to this adapter, so PRD 04's "or an open circuit breaker" clause
//! is satisfied by composition, not by a new variant: a breaker wrapping an
//! individual [`LlmPort`] must surface an [`LlmError`] the chain classifies
//! `Transient` (a [`LlmError::NetworkError`] or a 503
//! [`LlmError::ProviderError`], for instance) for the chain to hop past it.
//!
//! ## No cross-call state (T-25-36)
//!
//! The adapter holds only its immutable chain and sink. Nothing records
//! which provider served the previous call, so one call's hop never changes
//! where the next call — or a concurrent one — starts.
//!
//! Not feature-gated (ADR-0046): it composes whichever provider adapters are
//! compiled in and needs none of them itself.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::{self, Stream, StreamExt};
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::llm_port::{
    LlmError, LlmPort, LlmRequest, LlmResponse, ProviderCapabilities, StreamingResponse,
};
use paladin_ports::output::trace_sink_port::{TraceEvent, TraceSink};
use thiserror::Error;

/// The `LlmResponse.metadata` key under which the adapter records the
/// `get_provider_name()` of the provider that actually served a response
/// (D-26). A plain single-provider adapter never sets it.
pub const SERVED_BY_METADATA_KEY: &str = "paladin.served_by";

/// The provider name the adapter itself reports from
/// [`LlmPort::get_provider_name`].
pub const FALLBACK_PROVIDER_NAME: &str = "fallback";

/// Construction-time errors for [`FallbackLlmAdapter`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FallbackChainError {
    /// [`FallbackLlmAdapter::new`] was given no providers. Rejected at
    /// construction so the misconfiguration surfaces where it was made,
    /// not on the first request.
    #[error("a fallback chain needs at least one provider")]
    EmptyChain,
}

/// The item type every [`LlmPort::generate_stream`] yields.
type StreamItem = Result<StreamingResponse, LlmError>;

/// An ordered chain of [`LlmPort`]s that fails over on `Transient` or
/// `Unknown` errors only (FT-FR-16). See the module documentation for the
/// hop rule, the streaming first-chunk rule and the observability contract.
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
    trace_sink: Option<Arc<dyn TraceSink>>,
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
            .field("trace_sink", &self.trace_sink.is_some())
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
        Ok(Self {
            chain,
            trace_sink: None,
        })
    }

    /// Attach a [`TraceSink`] that receives one [`TraceEvent::FallbackHop`]
    /// per hop. The sink is awaited inline and its `Result` is discarded —
    /// a failing sink can never fail a request.
    pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
        self.trace_sink = Some(sink);
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
    /// plus a [`TraceEvent::FallbackHop`] if a sink is attached.
    async fn record_hop(&self, from: &'static str, to: &'static str, err: &LlmError) {
        log::warn!(
            "Fallback chain hopping from provider '{from}' to '{to}' after {transience:?} error: {err}",
            transience = err.transience()
        );
        if let Some(sink) = &self.trace_sink {
            if let Err(sink_err) = sink
                .on_event(TraceEvent::FallbackHop {
                    node_id: None,
                    from_provider: from.to_string(),
                    to_provider: to.to_string(),
                })
                .await
            {
                log::debug!("trace sink rejected FallbackHop event: {sink_err}");
            }
        }
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
        if err.transience() == Transience::Permanent {
            return Err(err);
        }
        match self.chain.get(index + 1) {
            Some(next) => {
                self.record_hop(from, next.get_provider_name(), &err).await;
                Ok(())
            }
            None => Err(LlmError::AllProvidersFailed {
                attempts: std::mem::take(attempts),
                last: Box::new(err),
            }),
        }
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
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::ProcessingError(
            "FallbackLlmAdapter::generate is not implemented yet".to_string(),
        ))
    }

    async fn generate_stream(
        &self,
        _request: LlmRequest,
    ) -> Result<Box<dyn Stream<Item = StreamItem> + Send>, LlmError> {
        let _ = stream::iter(Vec::<StreamItem>::new()).boxed();
        Err(LlmError::ProcessingError(
            "FallbackLlmAdapter::generate_stream is not implemented yet".to_string(),
        ))
    }

    async fn validate_model(&self, _model: &str) -> Result<bool, LlmError> {
        let _ = self.first();
        Err(Self::chain_exhausted())
    }

    async fn get_available_models(&self) -> Result<Vec<String>, LlmError> {
        Err(Self::chain_exhausted())
    }

    fn get_provider_name(&self) -> &'static str {
        FALLBACK_PROVIDER_NAME
    }

    fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::mock::MockLlmAdapter;
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::trace_sink_port::TraceSinkError;
    use std::collections::HashMap;
    use std::time::Duration;
    use uuid::Uuid;

    /// A [`TraceSink`] that records every event it receives, in order.
    #[derive(Default)]
    struct RecordingSink {
        events: tokio::sync::Mutex<Vec<TraceEvent>>,
    }

    impl RecordingSink {
        async fn events(&self) -> Vec<TraceEvent> {
            self.events.lock().await.clone()
        }

        async fn hops(&self) -> Vec<(Option<String>, String, String)> {
            self.events()
                .await
                .into_iter()
                .filter_map(|event| match event {
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
        async fn on_event(&self, event: TraceEvent) -> Result<(), TraceSinkError> {
            self.events.lock().await.push(event);
            Ok(())
        }
    }

    fn request() -> LlmRequest {
        let prompt = PromptItem::new(PromptType::User(UserPrompt {
            query: "quest".to_string(),
            context: None,
        }))
        .unwrap();
        LlmRequest {
            id: Uuid::new_v4(),
            model: "mock-model".to_string(),
            prompt,
            attachments: vec![],
            stream: false,
            metadata: HashMap::new(),
        }
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
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()]).with_trace_sink(sink.clone());

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
        let hops = sink.hops().await;
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
        let adapter = chain(&[p1.clone(), p2.clone(), p3.clone()]).with_trace_sink(sink.clone());

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
        let adapter = chain(&[p1.clone(), p2.clone()]).with_trace_sink(sink.clone());

        let stream = adapter.generate_stream(request()).await.unwrap();
        let items = collect(stream).await;

        let deltas: Vec<&str> = items
            .iter()
            .map(|item| item.as_ref().map(String::as_str).unwrap_or("<err>"))
            .collect();
        assert_eq!(deltas, ["Hello", " world"]);
        assert_eq!((p1.call_count(), p2.call_count()), (1, 1));
        assert_eq!(sink.hops().await.len(), 1);
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
        let adapter = chain(&[p1.clone(), p2.clone()]).with_trace_sink(sink.clone());

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
        assert_eq!(adapter.validate_model("claude").await.unwrap(), true);
        assert_eq!(adapter.validate_model("gpt").await.unwrap(), false);
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
        let adapter = chain(&[p1, p2, p3]).with_trace_sink(sink.clone());

        adapter.generate(request()).await.unwrap();

        let events = sink.events().await;
        assert_eq!(events.len(), 2);
        for (event, (from, to)) in events
            .iter()
            .zip([("openai", "anthropic"), ("anthropic", "deepseek")])
        {
            match event {
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
}
