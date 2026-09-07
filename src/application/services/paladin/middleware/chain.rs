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

use std::sync::Arc;

use crate::application::services::paladin::error::PaladinError;

use super::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    ToolCallContext, ToolFlow,
};

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
        match middleware.before_model(cx).await? {
            MiddlewareFlow::Continue => {}
            MiddlewareFlow::Finish(result) => {
                return Ok(BeforeOutcome::Finish {
                    result,
                    reached: index,
                });
            }
            MiddlewareFlow::Fail(error) => return Err(error),
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
        match middleware.after_model(cx, resp).await? {
            MiddlewareFlow::Continue => {}
            MiddlewareFlow::Finish(result) => return Ok(Some(result)),
            MiddlewareFlow::Fail(error) => return Err(error),
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
        match middleware.around_tool(cx).await? {
            ToolFlow::Allow => {}
            other => return Ok(other),
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
}
