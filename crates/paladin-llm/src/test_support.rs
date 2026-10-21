//! Test-only mock-endpoint plumbing shared by every adapter's wire tests: a mock server, a
//! capturing mock for the JSON body a request sends, and helpers that drive one request or count
//! the attempts made against a failing endpoint. Compiled only under `cfg(test)` and not gated
//! on any adapter feature, so any one adapter's tests can use it alone.

// Each adapter's tests use a different subset of these helpers, and a single-feature test build
// compiles only that adapter's tests, so any helper may be unused in some feature set.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use mockito::{Server, ServerGuard};
use serde_json::{Value, json};

/// Register a `/chat/completions` mock on `server` that records the JSON body of the request
/// it receives and answers with a minimal well-formed completion. Returns the capture slot.
pub(crate) async fn mock_completion(server: &mut ServerGuard) -> Arc<Mutex<Option<Value>>> {
    mock_capture(
        server,
        "/chat/completions",
        json!({
            "id": "cmpl-1",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        }),
    )
    .await
}

/// Register a `POST path` mock on `server` that records the JSON body of the request it
/// receives and answers HTTP 200 with `response`. Returns the capture slot.
///
/// The shape every wire's knob test uses: the compat presets and OpenAI/DeepSeek answer at
/// `/chat/completions`, Anthropic at `/messages`, Gemini at `/models/<model>:generateContent`.
pub(crate) async fn mock_capture(
    server: &mut ServerGuard,
    path: &str,
    response: Value,
) -> Arc<Mutex<Option<Value>>> {
    let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&captured);
    let body = response.to_string().into_bytes();
    server
        .mock("POST", path)
        .with_status(200)
        .with_body_from_request(move |req| {
            let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
            *slot.lock().unwrap() = serde_json::from_str(&text).ok();
            body.clone()
        })
        .create_async()
        .await;
    captured
}

/// A fresh mock server.
pub(crate) async fn server() -> ServerGuard {
    Server::new_async().await
}

/// Send one plain user request for `model` through `port`, with `max_tokens` as the
/// request's own value (every other sampling parameter unset), and return the JSON body the
/// mock endpoint captured in `captured`.
///
/// # Errors
///
/// Whatever `port.generate` returns, so a test can assert a refusal before any HTTP call.
pub(crate) async fn send_with_max_tokens(
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    captured: &Arc<Mutex<Option<Value>>>,
    model: &str,
    max_tokens: Option<u32>,
) -> Result<Value, paladin_ports::output::llm_port::LlmError> {
    use paladin_core::platform::container::prompt::{
        PromptItem, PromptParameters, PromptType, UserPrompt,
    };
    use paladin_ports::output::llm_port::LlmRequest;

    let mut request = LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    );
    request.prompt.entity.data.parameters = PromptParameters {
        max_tokens,
        temperature: None,
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop_sequences: None,
    };
    *captured.lock().unwrap() = None;
    port.generate(request).await?;
    Ok(captured
        .lock()
        .unwrap()
        .take()
        .expect("the mock endpoint received a request"))
}

/// A plain one-line user request for `model`, with no effort and no sampling parameter set
/// (Phase 51: the base the reasoning-effort tests add `with_reasoning` to).
pub(crate) fn plain_request(model: &str) -> paladin_ports::output::llm_port::LlmRequest {
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};

    paladin_ports::output::llm_port::LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    )
}

/// Send `request` through `port` and return the JSON body the mock endpoint captured in
/// `captured` together with the response (`None` for the body when no HTTP call was made).
///
/// # Errors
///
/// Whatever `port.generate` returns, so a test can assert a refusal before any HTTP call.
pub(crate) async fn send_capturing(
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    captured: &Arc<Mutex<Option<Value>>>,
    request: paladin_ports::output::llm_port::LlmRequest,
) -> Result<
    (Value, paladin_ports::output::llm_port::LlmResponse),
    paladin_ports::output::llm_port::LlmError,
> {
    *captured.lock().unwrap() = None;
    let response = port.generate(request).await?;
    let body = captured
        .lock()
        .unwrap()
        .take()
        .expect("the mock endpoint received a request");
    Ok((body, response))
}

/// How many times `port` hits a mock endpoint that answers every request with HTTP 500,
/// for one `generate` call. Asserts nothing about the error beyond it being one.
pub(crate) async fn attempts_against_a_failing_endpoint(
    server: ServerGuard,
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    model: &str,
    expected_attempts: usize,
) {
    attempts_against_a_failing_endpoint_at(
        server,
        "/chat/completions",
        port,
        model,
        expected_attempts,
    )
    .await;
}

/// [`attempts_against_a_failing_endpoint`] for a wire that answers at `path`.
pub(crate) async fn attempts_against_a_failing_endpoint_at(
    mut server: ServerGuard,
    path: &str,
    port: &dyn paladin_ports::output::llm_port::LlmPort,
    model: &str,
    expected_attempts: usize,
) {
    use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
    use paladin_ports::output::llm_port::LlmRequest;

    let mock = server
        .mock("POST", path)
        .with_status(500)
        .with_body(r#"{"error":"boom"}"#)
        .expect(expected_attempts)
        .create_async()
        .await;
    let request = LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: "Hello".to_string(),
            context: None,
        }))
        .unwrap(),
    );
    assert!(port.generate(request).await.is_err());
    mock.assert_async().await;
}
