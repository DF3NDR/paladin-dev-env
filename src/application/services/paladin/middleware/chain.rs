//! The onion-ordering chain driver: free functions over `&[Arc<dyn
//! ExecutionMiddleware>]`, called from
//! [`crate::application::services::paladin::paladin_execution_service::PaladinExecutionService`]'s
//! reasoning loop.
//!
//! `run_before` iterates first-to-last, recording how far it got.
//! `run_after` iterates that *reached prefix* last-to-first -- the onion
//! shape: the first middleware to see the request is the last to see the
//! response. `Finish` from index `i` stops the `before` pass, does **not**
//! run index `i`'s own `after`, and runs `after` for indices `0..i` in
//! reverse (D-06, PRD 05 §3.1) -- encoded here as the driver's documented
//! contract, not as a caller convention. `Fail(e)` aborts immediately and
//! propagates `e` unchanged; no `catch_unwind` anywhere. The empty slice is
//! not a special case -- the same loop runs for zero, one and many
//! middleware.
//!
//! # `MiddlewareEvent` emission (28-06, D-03, D-04)
//!
//! Every one of the six [`MiddlewareAction`] values is emitted as one
//! `TraceEvent::MiddlewareEvent` naming the middleware that took it, at
//! the point the driver OBSERVES that action:
//!
//! - [`MiddlewareAction::Finish`]/[`MiddlewareAction::Fail`] — observed
//!   directly from the returned [`MiddlewareFlow`], both in `run_before`
//!   and `run_after`.
//! - [`MiddlewareAction::Deny`]/[`MiddlewareAction::Redact`] — observed
//!   directly from the returned [`ToolFlow`] in `run_around_tool`
//!   (`Deny`/`Rewrite` respectively — a rewritten call IS a redaction of
//!   the request).
//! - [`MiddlewareAction::Retry`]/[`MiddlewareAction::Fallback`] — WR-01
//!   (28-REVIEW) moved these OFF the generic hint mechanism below:
//!   `ModelRetryMiddleware`/`ModelFallbackMiddleware::before_model` only
//!   INSTALL a capability (`cx.retry_policy`/`cx.llm_override`) that the
//!   call site may or may not end up using, so setting the hint
//!   unconditionally in `before_model` made every call -- retried or not,
//!   hopped or not -- emit a misleading event. `MiddlewareAction::Retry` is
//!   now emitted directly by
//!   `PaladinExecutionService::execute_with_retry_and_temperature`'s own
//!   retry arm, at the moment a retry is actually about to happen;
//!   `MiddlewareAction::Fallback` was dropped entirely in favor of
//!   `FallbackLlmAdapter`'s own `TraceEvent::FallbackHop`, which already
//!   fires only on a real hop. Neither goes through `emit_pending_hint`
//!   below anymore.
//! - The [`ModelCallContext::middleware_action_hint`] mechanism itself
//!   remains general-purpose: any future middleware whose action the
//!   driver cannot structurally observe from the returned `MiddlewareFlow`
//!   (e.g. `Guardrail`'s in-place redaction) can still set it before
//!   returning `Continue`, and `emit_pending_hint` reads (and clears) it
//!   immediately after EVERY `before_model`/`after_model` call.

use std::sync::Arc;

use paladin_ports::output::trace_sink_port::MiddlewareAction;

use crate::application::services::paladin::error::PaladinError;

use super::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    ToolCallContext, ToolFlow,
};

/// Emit `cx`'s pending [`ModelCallContext::middleware_action_hint`] (if
/// any) as a `TraceEvent::MiddlewareEvent` for `middleware_name`, then
/// clear it -- called after EVERY `before_model`/`after_model` invocation,
/// regardless of the `MiddlewareFlow` it returned, so a hint set alongside
/// a `Continue` (the common case: `ModelRetryMiddleware`/
/// `ModelFallbackMiddleware`, `Guardrail`'s in-place redaction) is never
/// lost, and never leaks onto the NEXT middleware's own call.
fn emit_pending_hint(cx: &mut ModelCallContext<'_>, middleware_name: &str) {
    if let Some(action) = cx.middleware_action_hint.take() {
        cx.emit_middleware_event(middleware_name, action);
    }
}

/// The result of running a chain's `before_model` pass.
#[derive(Debug)]
pub enum BeforeOutcome {
    /// Every middleware in the chain returned `Continue`. `reached` equals
    /// the chain's length -- the whole chain participates in the
    /// corresponding `run_after` call.
    Continue {
        /// Number of middleware whose `before_model` ran (== chain length).
        reached: usize,
    },
    /// A middleware requested an early finish. `reached` is the number of
    /// middleware BEFORE it whose own `before_model` returned `Continue` --
    /// exactly the prefix `run_after` must still visit in reverse.
    Finish {
        /// The final result to return for the run.
        result: FinalResult,
        /// The prefix of the chain whose `after_model` must still run.
        reached: usize,
    },
}

/// Run `chain`'s `before_model` hooks first-to-last over `cx`.
///
/// See the module doc for the exact `Finish`/`Fail` short-circuit contract.
pub async fn run_before(
    chain: &[Arc<dyn ExecutionMiddleware>],
    cx: &mut ModelCallContext<'_>,
) -> Result<BeforeOutcome, PaladinError> {
    for (index, middleware) in chain.iter().enumerate() {
        cx.middleware_action_hint = None;
        let flow = middleware.before_model(cx).await?;
        emit_pending_hint(cx, middleware.name());
        match flow {
            MiddlewareFlow::Continue => {}
            MiddlewareFlow::Finish(result) => {
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Finish);
                return Ok(BeforeOutcome::Finish {
                    result,
                    reached: index,
                });
            }
            MiddlewareFlow::Fail(error) => {
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Fail);
                return Err(error);
            }
        }
    }
    Ok(BeforeOutcome::Continue {
        reached: chain.len(),
    })
}

/// Run `chain`'s `after_model` hooks over the `reached` prefix, in reverse
/// (last-to-first).
///
/// Returns `Ok(Some(result))` if some middleware in the prefix requested an
/// early finish via its own `after_model`; the reverse walk stops
/// immediately at that point (its own `Finish`/`Fail` is terminal for the
/// `after` pass, mirroring `before_model`'s short-circuit contract).
pub async fn run_after(
    chain: &[Arc<dyn ExecutionMiddleware>],
    cx: &mut ModelCallContext<'_>,
    resp: &mut LlmResponseView,
    reached: usize,
) -> Result<Option<FinalResult>, PaladinError> {
    for middleware in chain[..reached].iter().rev() {
        cx.middleware_action_hint = None;
        let flow = middleware.after_model(cx, resp).await?;
        emit_pending_hint(cx, middleware.name());
        match flow {
            MiddlewareFlow::Continue => {}
            MiddlewareFlow::Finish(result) => {
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Finish);
                return Ok(Some(result));
            }
            MiddlewareFlow::Fail(error) => {
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Fail);
                return Err(error);
            }
        }
    }
    Ok(None)
}

/// Run `chain`'s `around_tool` hooks first-to-last over `cx`, returning the
/// FIRST non-`Allow` flow, or `Allow` if every middleware allowed the call.
///
/// `cx` is `&mut` so a stateful middleware (`ToolCallLimit`, D-08) can read
/// and write `cx.scratch` across the chain; the caller is responsible for
/// copying it back into the run's `ModelCallContext::scratch` afterward.
pub async fn run_around_tool(
    chain: &[Arc<dyn ExecutionMiddleware>],
    cx: &mut ToolCallContext,
) -> Result<ToolFlow, PaladinError> {
    for middleware in chain {
        let flow = middleware.around_tool(cx).await?;
        match &flow {
            ToolFlow::Allow => {}
            ToolFlow::Deny { .. } => {
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Deny);
                return Ok(flow);
            }
            ToolFlow::Rewrite(_) => {
                // A rewritten call IS a redaction of the request -- the
                // model's original arguments never reach the Arsenal/
                // handoff dispatch unchanged.
                cx.emit_middleware_event(middleware.name(), MiddlewareAction::Redact);
                return Ok(flow);
            }
        }
    }
    Ok(ToolFlow::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::middleware::PromptAssembly;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::paladin::PaladinData;
    use paladin_ports::output::llm_port::{FinishReason, LlmResponse, TokenUsage};
    use std::sync::Mutex;
    use uuid::Uuid;

    fn make_paladin() -> crate::core::platform::container::paladin::Paladin {
        Node::new(PaladinData::default(), None)
    }

    fn make_context(
        paladin: &crate::core::platform::container::paladin::Paladin,
    ) -> ModelCallContext<'_> {
        let assembly = PromptAssembly::new("system", "input", "", vec![], None);
        ModelCallContext::new(Uuid::new_v4(), paladin, assembly)
    }

    fn make_response_view() -> LlmResponseView {
        LlmResponseView::from_response(&LlmResponse {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            model: "mock".to_string(),
            content: "response".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::default(),
            created_at: chrono::Utc::now(),
            metadata: Default::default(),
            function_call: None,
        })
    }

    /// Records `"{name}.before"` / `"{name}.after"` into a shared log.
    struct RecordingMiddleware {
        name: &'static str,
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl ExecutionMiddleware for RecordingMiddleware {
        async fn before_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.before", self.name));
            Ok(MiddlewareFlow::Continue)
        }

        async fn after_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
            _resp: &mut LlmResponseView,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.after", self.name));
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    /// Like [`RecordingMiddleware`], but its `before_model` returns
    /// `Finish` on a specific 1-indexed call number.
    struct FinishOnCall {
        name: &'static str,
        log: Arc<Mutex<Vec<String>>>,
        at: u32,
        calls: std::sync::atomic::AtomicU32,
    }

    #[async_trait::async_trait]
    impl ExecutionMiddleware for FinishOnCall {
        async fn before_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.before", self.name));
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if call == self.at {
                return Ok(MiddlewareFlow::Finish(FinalResult::new(
                    "finished early",
                    paladin_ports::output::paladin_port::StopReason::Completed,
                )));
            }
            Ok(MiddlewareFlow::Continue)
        }

        async fn after_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
            _resp: &mut LlmResponseView,
        ) -> Result<MiddlewareFlow, PaladinError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{}.after", self.name));
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    /// A middleware whose `before_model` always fails.
    struct FailingMiddleware;

    #[async_trait::async_trait]
    impl ExecutionMiddleware for FailingMiddleware {
        async fn before_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            Ok(MiddlewareFlow::Fail(PaladinError::ExecutionError(
                "deliberate failure".to_string(),
            )))
        }

        fn name(&self) -> &str {
            "failing"
        }
    }

    #[tokio::test]
    async fn onion_ordering_finish_from_second_middleware() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let a: Arc<dyn ExecutionMiddleware> = Arc::new(RecordingMiddleware {
            name: "A",
            log: log.clone(),
        });
        let b: Arc<dyn ExecutionMiddleware> = Arc::new(FinishOnCall {
            name: "B",
            log: log.clone(),
            at: 1,
            calls: std::sync::atomic::AtomicU32::new(0),
        });
        let c: Arc<dyn ExecutionMiddleware> = Arc::new(RecordingMiddleware {
            name: "C",
            log: log.clone(),
        });
        let chain = vec![a, b, c];

        let paladin = make_paladin();
        let mut cx = make_context(&paladin);
        let outcome = run_before(&chain, &mut cx).await.unwrap();

        let (result, reached) = match outcome {
            BeforeOutcome::Finish { result, reached } => (result, reached),
            BeforeOutcome::Continue { .. } => panic!("expected Finish"),
        };
        assert_eq!(
            result.stop_reason,
            paladin_ports::output::paladin_port::StopReason::Completed
        );

        let mut view = make_response_view();
        let after_outcome = run_after(&chain, &mut cx, &mut view, reached)
            .await
            .unwrap();
        assert!(after_outcome.is_none());

        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "A.before".to_string(),
                "B.before".to_string(),
                "A.after".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn onion_ordering_full_pass_runs_after_in_reverse() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let a: Arc<dyn ExecutionMiddleware> = Arc::new(RecordingMiddleware {
            name: "A",
            log: log.clone(),
        });
        let b: Arc<dyn ExecutionMiddleware> = Arc::new(RecordingMiddleware {
            name: "B",
            log: log.clone(),
        });
        let c: Arc<dyn ExecutionMiddleware> = Arc::new(RecordingMiddleware {
            name: "C",
            log: log.clone(),
        });
        let chain = vec![a, b, c];

        let paladin = make_paladin();
        let mut cx = make_context(&paladin);
        let outcome = run_before(&chain, &mut cx).await.unwrap();
        let reached = match outcome {
            BeforeOutcome::Continue { reached } => reached,
            BeforeOutcome::Finish { .. } => panic!("expected Continue"),
        };
        assert_eq!(reached, 3);

        let mut view = make_response_view();
        run_after(&chain, &mut cx, &mut view, reached)
            .await
            .unwrap();

        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "A.before".to_string(),
                "B.before".to_string(),
                "C.before".to_string(),
                "C.after".to_string(),
                "B.after".to_string(),
                "A.after".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn fail_from_before_model_propagates_unchanged_and_is_not_retried() {
        let chain: Vec<Arc<dyn ExecutionMiddleware>> = vec![Arc::new(FailingMiddleware)];
        let paladin = make_paladin();
        let mut cx = make_context(&paladin);

        let result = run_before(&chain, &mut cx).await;
        match result {
            Err(PaladinError::ExecutionError(msg)) => {
                assert_eq!(msg, "deliberate failure");
            }
            other => panic!("expected ExecutionError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_chain_before_after_are_no_ops() {
        let chain: Vec<Arc<dyn ExecutionMiddleware>> = vec![];
        let paladin = make_paladin();
        let mut cx = make_context(&paladin);
        let outcome = run_before(&chain, &mut cx).await.unwrap();
        let reached = match outcome {
            BeforeOutcome::Continue { reached } => reached,
            BeforeOutcome::Finish { .. } => panic!("expected Continue"),
        };
        assert_eq!(reached, 0);

        let mut view = make_response_view();
        let after_outcome = run_after(&chain, &mut cx, &mut view, reached)
            .await
            .unwrap();
        assert!(after_outcome.is_none());
    }

    // ── 28-06: MiddlewareEvent emission (D-03, D-04) ─────────────────────

    /// A middleware that sets `cx.middleware_action_hint` to a fixed
    /// [`MiddlewareAction`] and returns `Continue` -- the general mechanism
    /// `ModelFallbackMiddleware`/`ModelRetryMiddleware`/`Guardrail`'s
    /// in-place redaction all use for an action the driver cannot observe
    /// structurally from the returned `MiddlewareFlow` (see this module's
    /// own docs).
    struct HintMiddleware {
        name: &'static str,
        action: MiddlewareAction,
    }

    #[async_trait::async_trait]
    impl ExecutionMiddleware for HintMiddleware {
        async fn before_model(
            &self,
            cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            cx.middleware_action_hint = Some(self.action);
            Ok(MiddlewareFlow::Continue)
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    /// A middleware whose `before_model` always finishes the run.
    struct AlwaysFinishMiddleware(&'static str);
    #[async_trait::async_trait]
    impl ExecutionMiddleware for AlwaysFinishMiddleware {
        async fn before_model(
            &self,
            _cx: &mut ModelCallContext<'_>,
        ) -> Result<MiddlewareFlow, PaladinError> {
            Ok(MiddlewareFlow::Finish(FinalResult::new(
                "done",
                paladin_ports::output::paladin_port::StopReason::Completed,
            )))
        }
        fn name(&self) -> &str {
            self.0
        }
    }

    /// A middleware whose `around_tool` always denies the call.
    struct AlwaysDenyMiddleware(&'static str);
    #[async_trait::async_trait]
    impl ExecutionMiddleware for AlwaysDenyMiddleware {
        async fn around_tool(&self, _cx: &mut ToolCallContext) -> Result<ToolFlow, PaladinError> {
            Ok(ToolFlow::Deny {
                reason: "denied".to_string(),
            })
        }
        fn name(&self) -> &str {
            self.0
        }
    }

    fn make_tool_context(
        run_id: Uuid,
        trace_emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter>,
    ) -> ToolCallContext {
        use crate::core::platform::container::arsenal::ArmamentCall;
        use std::collections::HashMap;

        ToolCallContext {
            call: ArmamentCall::new("noop", HashMap::new()),
            kind: super::super::ToolCallKind::Armament,
            loop_index: 0,
            run_id,
            scratch: HashMap::new(),
            trace_emitter: Some(trace_emitter),
        }
    }

    /// D-04: a table test driving the chain through each of the six
    /// `MiddlewareAction` values produces exactly one `MiddlewareEvent` per
    /// action, carrying the acting middleware's own `name()` and the
    /// matching `MiddlewareAction`.
    #[tokio::test]
    async fn middleware_emits_one_event_per_action() {
        use paladin_core::platform::container::waypoint::ThreadId;
        use paladin_ports::output::trace_sink_port::{
            StandaloneEmitter, TraceEvent, TraceRecord, TraceSink, TraceSinkError,
        };

        #[derive(Default)]
        struct RecordingSink {
            events: Mutex<Vec<TraceRecord>>,
        }
        #[async_trait::async_trait]
        impl TraceSink for RecordingSink {
            async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
                self.events.lock().unwrap().push(record);
                Ok(())
            }
        }

        async fn events_eventually(sink: &RecordingSink, expected_len: usize) -> Vec<TraceRecord> {
            for _ in 0..200 {
                let events = sink.events.lock().unwrap().clone();
                if events.len() >= expected_len {
                    return events;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            sink.events.lock().unwrap().clone()
        }

        let sink = Arc::new(RecordingSink::default());
        let emitter: Arc<dyn paladin_ports::output::trace_sink_port::TraceEmitter> =
            Arc::new(StandaloneEmitter::new(
                ThreadId::new("chain-test").unwrap(),
                None,
                Some(sink.clone() as Arc<dyn TraceSink>),
            ));

        // Finish, Fail: observed directly from `run_before`'s own
        // `MiddlewareFlow`.
        let paladin = make_paladin();
        let mut cx = make_context(&paladin);
        cx.trace_emitter = Some(emitter.clone());
        let finish_chain: Vec<Arc<dyn ExecutionMiddleware>> =
            vec![Arc::new(AlwaysFinishMiddleware("finisher"))];
        let _ = run_before(&finish_chain, &mut cx).await.unwrap();

        let mut cx = make_context(&paladin);
        cx.trace_emitter = Some(emitter.clone());
        let fail_chain: Vec<Arc<dyn ExecutionMiddleware>> = vec![Arc::new(FailingMiddleware)];
        let _ = run_before(&fail_chain, &mut cx).await;

        // Deny: observed directly from `run_around_tool`'s own `ToolFlow`.
        let mut tool_cx = make_tool_context(cx.run_id, emitter.clone());
        let deny_chain: Vec<Arc<dyn ExecutionMiddleware>> =
            vec![Arc::new(AlwaysDenyMiddleware("denier"))];
        let _ = run_around_tool(&deny_chain, &mut tool_cx).await.unwrap();

        // Redact, Retry, Fallback: observed via `cx.middleware_action_hint`
        // (the mechanism `Guardrail`/`ModelFallbackMiddleware`/
        // `ModelRetryMiddleware` each use in production).
        for (name, action) in [
            ("redactor", MiddlewareAction::Redact),
            ("retrier", MiddlewareAction::Retry),
            ("hopper", MiddlewareAction::Fallback),
        ] {
            let mut cx = make_context(&paladin);
            cx.trace_emitter = Some(emitter.clone());
            let hint_chain: Vec<Arc<dyn ExecutionMiddleware>> =
                vec![Arc::new(HintMiddleware { name, action })];
            let outcome = run_before(&hint_chain, &mut cx).await.unwrap();
            assert!(matches!(outcome, BeforeOutcome::Continue { .. }));
        }

        let events = events_eventually(&sink, 6).await;
        assert_eq!(events.len(), 6, "{events:?}");

        let observed: Vec<(String, MiddlewareAction)> = events
            .iter()
            .map(|record| match &record.event {
                TraceEvent::MiddlewareEvent { name, action } => (name.clone(), *action),
                other => panic!("unexpected non-MiddlewareEvent record: {other:?}"),
            })
            .collect();

        let expect_exactly_one = |name: &str, action: MiddlewareAction| {
            let count = observed
                .iter()
                .filter(|(n, a)| n == name && *a == action)
                .count();
            assert_eq!(
                count, 1,
                "expected exactly one {action:?} event named {name:?}, observed: {observed:?}"
            );
        };
        expect_exactly_one("finisher", MiddlewareAction::Finish);
        expect_exactly_one("failing", MiddlewareAction::Fail);
        expect_exactly_one("denier", MiddlewareAction::Deny);
        expect_exactly_one("redactor", MiddlewareAction::Redact);
        expect_exactly_one("retrier", MiddlewareAction::Retry);
        expect_exactly_one("hopper", MiddlewareAction::Fallback);
    }
}
