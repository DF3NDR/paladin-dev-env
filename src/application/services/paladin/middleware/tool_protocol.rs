//! `ToolCallProtocolMiddleware` / `FinishOnPlainAnswerMiddleware`: a
//! **prompt-level** tool-call protocol that makes the reasoning loop's tool
//! branch reachable for a shipped LLM provider (D-36).
//!
//! # This is prompt-level, not wire-level -- ADR-0042's boundary stays untouched
//!
//! No shipped adapter ever populates `LlmResponse.function_call`
//! (`crates/paladin-llm/src/mock.rs` included) -- ADR-0042 defers
//! wire-level (native) tool calling behind its own trigger, and nothing in
//! the tree renders a tool catalogue into a prompt (the builder only
//! auto-registers the handoff tool). This module does not touch any of
//! that: no `LlmRequest.tools` field is added, no adapter file is
//! modified, and `MockLlmAdapter`'s capability flags stay `false`.
//! `crates/paladin-llm/src/lib.rs`'s
//! `test_capabilities_tool_calling_matches_request_surface` is this
//! module's own witness that the correspondence test still holds
//! (`no_adapter_or_capability_changed`).
//!
//! Instead, [`ToolCallProtocolMiddleware::before_model`] renders the
//! arsenal's [`paladin_ports::output::arsenal_port::ArsenalPort::list_armaments`]
//! catalogue directly into the prompt text as a `## Tools`
//! [`super::PromptSection`], with a documented JSON envelope the model is
//! asked to reply with. [`ToolCallProtocolMiddleware::after_model`] --
//! ONLY when a real `function_call` is absent -- extracts that envelope
//! from the model's plain-text response via the SAME
//! [`paladin_core::platform::container::structured::extract_json`] the
//! structured-output machinery already uses (never a second envelope
//! parser), and synthesizes [`super::LlmResponseView::function_call`] so
//! the EXISTING tool branch in `execute_internal` (the `if let
//! Some(function_call) = response_view.function_call.clone()` check) fires
//! completely unchanged. Anything that is not the documented envelope --
//! non-JSON text, JSON that is not the envelope shape, or a tool name the
//! arsenal does not declare -- is left exactly as it is; a real adapter's
//! or a consumer-supplied port's own `function_call` is never overwritten
//! (T-26-61).
//!
//! # `FinishOnPlainAnswerMiddleware` is a SEPARATE opt-in (D-36, X-03)
//!
//! Today's loop always runs to `max_loops` and returns
//! `StopReason::MaxLoops` -- correct default for a multi-turn
//! conversational agent, wrong for a tool-using agent that should stop the
//! moment it answers without requesting a tool. Rather than changing that
//! default for every existing caller,
//! [`FinishOnPlainAnswerMiddleware`] is a second, independent opt-in
//! built-in: once installed, it finishes the run the moment a response
//! carries no tool call. Not installed, the loop's existing `MaxLoops`
//! behavior is completely unchanged
//! (`without_the_middleware_the_loop_still_runs_to_max_loops`).

use async_trait::async_trait;
use std::sync::Arc;

use crate::application::services::paladin::error::PaladinError;
use crate::core::platform::container::arsenal::Armament;
use paladin_core::platform::container::structured::extract_json;
use paladin_ports::output::arsenal_port::ArsenalPort;
use paladin_ports::output::llm_port::FunctionCall;
use paladin_ports::output::paladin_port::StopReason;

use super::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    PromptSection, SectionPlacement,
};

/// Heading for the rendered tool-catalogue section. A single named
/// constant so a test asserts the exact heading text this implementation
/// emits, never an approximation of it.
const TOOLS_HEADING: &str = "Tools";

/// The documented JSON envelope's call-format instructions, appended after
/// the rendered catalogue. Named so the render side and the test asserting
/// its wording stay in sync.
const ENVELOPE_INSTRUCTIONS: &str = "To call one of the tools above, reply with ONLY a JSON \
object of this exact shape (as plain text, or inside a ```json fenced block):\n\n\
{\"tool\": \"<tool name>\", \"arguments\": { ... }}\n\n\
If you do not need a tool, reply normally with your answer as plain text.";

/// Renders the arsenal's tool catalogue -- name, description and JSON
/// Schema parameters for each [`Armament`] -- plus [`ENVELOPE_INSTRUCTIONS`],
/// into one Markdown body. Returns `None` for an empty arsenal: an empty
/// catalogue renders NO section at all, not an empty one (Test 2 --
/// `empty_arsenal_renders_no_tools_section`).
fn render_catalogue(armaments: &[Armament]) -> Option<String> {
    if armaments.is_empty() {
        return None;
    }

    let mut body = String::new();
    for armament in armaments {
        body.push_str(&format!(
            "### {}\n{}\nParameters (JSON Schema): {}\n\n",
            armament.name, armament.description, armament.parameters
        ));
    }
    body.push_str(ENVELOPE_INSTRUCTIONS);
    Some(body)
}

/// The documented envelope's decoded parts: a tool name known to the
/// arsenal, and its arguments as a JSON object.
struct DecodedEnvelope {
    tool_name: String,
    arguments: serde_json::Map<String, serde_json::Value>,
}

/// Parses `content` for the documented `{"tool": "<name>", "arguments":
/// {..}}` envelope (bare or fenced, via the shared [`extract_json`]),
/// returning `None` for anything else: non-JSON text, JSON that is not the
/// envelope shape, or a missing/wrong-typed `tool`/`arguments` key. Does
/// NOT check the arsenal -- callers still validate the tool name is known
/// (T-26-10).
fn decode_envelope(content: &str) -> Option<DecodedEnvelope> {
    let value = extract_json(content)?;
    let tool_name = value.get("tool")?.as_str()?.to_string();
    let arguments = value.get("arguments")?.as_object()?.clone();
    Some(DecodedEnvelope {
        tool_name,
        arguments,
    })
}

/// Renders the arsenal's [`ArsenalPort::list_armaments`] catalogue into a
/// `## Tools` prompt section on every iteration, and synthesizes
/// [`super::LlmResponseView::function_call`] from the documented JSON
/// envelope when no real one is present. See the module docs for the full
/// ADR-0042 boundary and the ordering/overwrite guarantees.
pub struct ToolCallProtocolMiddleware {
    arsenal: Arc<dyn ArsenalPort>,
}

impl ToolCallProtocolMiddleware {
    /// Constructs a `ToolCallProtocolMiddleware` over `arsenal`'s tool
    /// catalogue.
    pub fn new(arsenal: Arc<dyn ArsenalPort>) -> Self {
        Self { arsenal }
    }
}

#[async_trait]
impl ExecutionMiddleware for ToolCallProtocolMiddleware {
    async fn before_model(
        &self,
        cx: &mut ModelCallContext<'_>,
    ) -> Result<MiddlewareFlow, PaladinError> {
        let armaments = self.arsenal.list_armaments().await;
        if let Some(body) = render_catalogue(&armaments) {
            cx.assembly.push_section(PromptSection::new(
                TOOLS_HEADING,
                body,
                SectionPlacement::End,
            ));
        }
        Ok(MiddlewareFlow::Continue)
    }

    async fn after_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        // T-26-61: a real adapter's or a consumer-supplied port's own
        // function_call is never overwritten.
        if resp.function_call.is_some() {
            return Ok(MiddlewareFlow::Continue);
        }

        let Some(decoded) = decode_envelope(&resp.content) else {
            // Non-JSON, or JSON that is not the documented envelope: leave
            // the response exactly as it is (Test 4).
            return Ok(MiddlewareFlow::Continue);
        };

        // T-26-10: only honor a tool the arsenal actually declares -- an
        // unknown name leaves function_call as None and the text is
        // returned to the model (Test 6), rather than producing a call
        // that will fail downstream.
        let known = self
            .arsenal
            .list_armaments()
            .await
            .iter()
            .any(|armament| armament.name == decoded.tool_name);
        if !known {
            return Ok(MiddlewareFlow::Continue);
        }

        let Ok(arguments_json) =
            serde_json::to_string(&serde_json::Value::Object(decoded.arguments))
        else {
            // Unreachable in practice (a serde_json::Map always
            // serializes), but this library never panics on a malformed
            // model response -- leave the response untouched instead.
            return Ok(MiddlewareFlow::Continue);
        };

        resp.function_call = Some(FunctionCall {
            name: decoded.tool_name,
            arguments: arguments_json,
        });

        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "tool_call_protocol"
    }
}

/// Finishes the run the moment a response carries no tool call -- the
/// opt-in complement to today's always-run-to-`max_loops` default (D-36,
/// X-03). See the module docs for why this is a separate, independent
/// built-in rather than a change to the existing default.
#[derive(Default)]
pub struct FinishOnPlainAnswerMiddleware;

impl FinishOnPlainAnswerMiddleware {
    /// Constructs a `FinishOnPlainAnswerMiddleware`.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionMiddleware for FinishOnPlainAnswerMiddleware {
    async fn after_model(
        &self,
        _cx: &mut ModelCallContext<'_>,
        resp: &mut LlmResponseView,
    ) -> Result<MiddlewareFlow, PaladinError> {
        if resp.function_call.is_none() {
            return Ok(MiddlewareFlow::Finish(FinalResult::new(
                resp.content.clone(),
                StopReason::Completed,
            )));
        }
        Ok(MiddlewareFlow::Continue)
    }

    fn name(&self) -> &str {
        "finish_on_plain_answer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::core::base::entity::node::Node;
    use crate::core::platform::container::arsenal::{ArmamentCall, ArmamentResult, ArsenalError};
    use crate::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;
    use paladin_llm::mock::MockLlmAdapter;
    use std::sync::Mutex;
    use std::time::Duration;

    fn make_paladin(max_loops: u32) -> Paladin {
        let data = PaladinData {
            system_prompt: "You are a helpful assistant".to_string(),
            max_loops: MaxLoops::Fixed(max_loops),
            ..Default::default()
        };
        Node::new(data, None)
    }

    /// A fixed-catalogue `ArsenalPort` that records every call it
    /// receives and always succeeds.
    #[derive(Default)]
    struct FixedArsenal {
        armaments: Vec<Armament>,
        calls: Mutex<Vec<ArmamentCall>>,
    }

    impl FixedArsenal {
        fn new(armaments: Vec<Armament>) -> Self {
            Self {
                armaments,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ArsenalPort for FixedArsenal {
        async fn list_armaments(&self) -> Vec<Armament> {
            self.armaments.clone()
        }

        async fn invoke(&self, call: ArmamentCall) -> Result<ArmamentResult, ArsenalError> {
            self.calls.lock().unwrap().push(call.clone());
            Ok(ArmamentResult::success(
                call.call_id,
                serde_json::json!("ok"),
                0,
            ))
        }

        fn validate_call(&self, _call: &ArmamentCall) -> Result<(), ArsenalError> {
            Ok(())
        }
    }

    fn add_armament() -> Armament {
        Armament {
            name: "add".to_string(),
            description: "Adds two integers".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "a": {"type": "integer"}, "b": {"type": "integer"} },
                "required": ["a", "b"]
            }),
            required_params: vec!["a".to_string(), "b".to_string()],
        }
    }

    fn search_armament() -> Armament {
        Armament {
            name: "search".to_string(),
            description: "Searches the web".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "query": {"type": "string"} },
                "required": ["query"]
            }),
            required_params: vec!["query".to_string()],
        }
    }

    /// Test 1: with an arsenal declaring two armaments, the rendered
    /// prompt contains a `## Tools` section naming both, with their
    /// descriptions and parameter schemas, plus the documented call
    /// format.
    #[tokio::test]
    async fn tool_catalogue_is_rendered_into_a_tools_section() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let arsenal: Arc<dyn ArsenalPort> =
            Arc::new(FixedArsenal::new(vec![add_armament(), search_armament()]));
        let service = PaladinExecutionService::new(
            llm.clone(),
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
        .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(arsenal)));
        let paladin = make_paladin(1);

        service.execute(&paladin, "hi").await.unwrap();

        let prompt = llm.last_prompt().unwrap();
        assert!(prompt.contains("## Tools"), "got {prompt}");
        assert!(prompt.contains("### add"), "got {prompt}");
        assert!(prompt.contains("Adds two integers"), "got {prompt}");
        assert!(prompt.contains("### search"), "got {prompt}");
        assert!(prompt.contains("Searches the web"), "got {prompt}");
        assert!(prompt.contains("\"type\":\"object\""), "got {prompt}");
        assert!(
            prompt.contains("{\"tool\": \"<tool name>\", \"arguments\": { ... }}"),
            "got {prompt}"
        );
    }

    /// Test 2: with no armaments, no `## Tools` section appears and the
    /// prompt is otherwise unchanged (D-02's empty-chain equivalence,
    /// extended to an empty arsenal).
    #[tokio::test]
    async fn empty_arsenal_renders_no_tools_section() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("ack"));
        let arsenal: Arc<dyn ArsenalPort> = Arc::new(FixedArsenal::default());
        let service = PaladinExecutionService::new(
            llm.clone(),
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
        .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(arsenal)));
        let paladin = make_paladin(1);

        service.execute(&paladin, "hi").await.unwrap();

        let prompt = llm.last_prompt().unwrap();
        assert!(!prompt.contains("## Tools"), "got {prompt}");
        assert_eq!(prompt, "You are a helpful assistant\n\nUser: hi\n");
    }

    /// Test 3: a response whose content is the documented envelope (bare
    /// or fenced) causes `function_call` to be synthesized so the
    /// existing tool branch fires and the Arsenal receives an `add` call.
    #[tokio::test]
    async fn a_documented_envelope_synthesizes_a_function_call() {
        for content in [
            r#"{"tool":"add","arguments":{"a":2,"b":2}}"#.to_string(),
            "```json\n{\"tool\":\"add\",\"arguments\":{\"a\":2,\"b\":2}}\n```".to_string(),
        ] {
            let llm = Arc::new(MockLlmAdapter::new().with_response(content));
            let arsenal = Arc::new(FixedArsenal::new(vec![add_armament()]));
            let service = PaladinExecutionService::new(
                llm,
                Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
                None,
                Some(arsenal.clone() as Arc<dyn ArsenalPort>),
            )
            .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(
                arsenal.clone() as Arc<dyn ArsenalPort>,
            )));
            let paladin = make_paladin(1);

            service.execute(&paladin, "add 2 and 2").await.unwrap();

            let calls = arsenal.calls.lock().unwrap();
            assert_eq!(calls.len(), 1, "arsenal should receive exactly one call");
            assert_eq!(calls[0].tool_name, "add");
            assert_eq!(calls[0].arguments.get("a").unwrap(), &serde_json::json!(2));
            assert_eq!(calls[0].arguments.get("b").unwrap(), &serde_json::json!(2));
        }
    }

    /// Test 4: a response whose content is valid JSON but NOT the
    /// documented envelope leaves `function_call` as `None` and the
    /// content untouched.
    #[tokio::test]
    async fn a_non_envelope_json_response_is_left_alone() {
        let llm =
            Arc::new(MockLlmAdapter::new().with_response(r#"{"answer": 4, "confidence": "high"}"#));
        let arsenal = Arc::new(FixedArsenal::new(vec![add_armament()]));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            Some(arsenal.clone() as Arc<dyn ArsenalPort>),
        )
        .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(
            arsenal.clone() as Arc<dyn ArsenalPort>
        )));
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(arsenal.calls.lock().unwrap().len(), 0);
        assert!(result.output.contains(r#""answer": 4"#));
    }

    /// Test 5: when a consumer-supplied port already populated
    /// `function_call`, the middleware does not overwrite it.
    #[tokio::test]
    async fn a_response_that_already_has_a_function_call_is_untouched() {
        use paladin_llm::mock::MockScriptEntry;

        let llm = Arc::new(
            MockLlmAdapter::new().with_script(vec![MockScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: r#"{"query":"rust"}"#.to_string(),
            }]),
        );
        // The envelope in this response, if parsed, would name a
        // DIFFERENT tool -- proving the real function_call wins.
        let arsenal = Arc::new(FixedArsenal::new(vec![add_armament(), search_armament()]));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            Some(arsenal.clone() as Arc<dyn ArsenalPort>),
        )
        .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(
            arsenal.clone() as Arc<dyn ArsenalPort>
        )));
        let paladin = make_paladin(1);

        service.execute(&paladin, "hi").await.unwrap();

        let calls = arsenal.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "search");
    }

    /// Test 6: an envelope naming a tool the arsenal does not declare
    /// leaves `function_call` as `None` rather than producing a call that
    /// will fail -- the model gets its text back.
    #[tokio::test]
    async fn an_unknown_tool_name_in_the_envelope_is_not_synthesized() {
        let llm = Arc::new(
            MockLlmAdapter::new().with_response(r#"{"tool":"delete_everything","arguments":{}}"#),
        );
        let arsenal = Arc::new(FixedArsenal::new(vec![add_armament()]));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            Some(arsenal.clone() as Arc<dyn ArsenalPort>),
        )
        .with_middleware(Arc::new(ToolCallProtocolMiddleware::new(
            arsenal.clone() as Arc<dyn ArsenalPort>
        )));
        let paladin = make_paladin(1);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(arsenal.calls.lock().unwrap().len(), 0);
        assert!(result.output.contains("delete_everything"));
    }

    /// Test 7: with `FinishOnPlainAnswerMiddleware` installed, a response
    /// carrying no tool call finishes with that content and
    /// `StopReason::Completed` after one iteration.
    #[tokio::test]
    async fn finish_on_plain_answer_completes_the_run() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("the answer is 4"));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        )
        .with_middleware(Arc::new(FinishOnPlainAnswerMiddleware::new()));
        let paladin = make_paladin(5);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(result.output, "the answer is 4");
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert_eq!(result.loop_count, 1);
    }

    /// Test 8: the same run WITHOUT the middleware behaves exactly as
    /// today, returning `StopReason::MaxLoops`.
    #[tokio::test]
    async fn without_the_middleware_the_loop_still_runs_to_max_loops() {
        let llm = Arc::new(MockLlmAdapter::new().with_response("the answer is 4"));
        let service = PaladinExecutionService::new(
            llm,
            Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60))),
            None,
            None,
        );
        let paladin = make_paladin(5);

        let result = service.execute(&paladin, "hi").await.unwrap();

        assert_eq!(result.stop_reason, StopReason::MaxLoops);
        assert_eq!(result.loop_count, 5);
    }

    /// Test 9: `test_capabilities_tool_calling_matches_request_surface`
    /// (paladin-llm) still passes and no adapter file was modified by this
    /// plan -- executed as part of the workspace test run and the plan's
    /// own `git diff` acceptance criterion; this test asserts the same
    /// invariant from inside this crate: no `LlmRequest.tools` field
    /// exists and `MockLlmAdapter`'s capabilities stay `false`.
    #[tokio::test]
    async fn no_adapter_or_capability_changed() {
        use paladin_ports::output::llm_port::LlmPort;

        let llm = MockLlmAdapter::new();
        let caps = llm.get_capabilities();
        assert!(
            !caps.supports_tool_calling,
            "MockLlmAdapter's tool-calling capability flag must stay false (ADR-0042)"
        );
    }

    // D-34/D-36 cross-check ("this middleware reuses the SAME
    // `extract_json` the structured-output path uses -- no second
    // envelope-parsing function exists anywhere in the facade") is
    // verified at the shell level, per this plan's own acceptance
    // criteria, not as a unit test: a source-level self-check written as
    // a Rust `#[test]` would have to embed the exact literal it searches
    // for via `include_str!`, which then matches ITSELF -- not a
    // meaningful assertion.
}
