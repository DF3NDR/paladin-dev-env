//! Shared OpenAI-compatible protocol engine (D-05, Phase 17).
//!
//! One engine owns the request/response types, streaming chunk assembly,
//! retry and error mapping shared by every OpenAI-compatible provider
//! preset (Kimi, and future presets under this feature gate). Presets supply
//! only `base_url`, credential, default model, curated model-list fallback
//! and a capabilities block.

/// Generalized OpenAI-compatible wire types (request/response/stream shapes).
pub mod types;

/// The shared engine: request building, HTTP transport, retry, error
/// mapping, credential redaction and memoized model-list resolution.
pub mod engine;

pub use engine::{CompatCapabilities, CompatEngine, CompatEngineConfig, CompatRequestParameters};

/// Test-only plumbing shared by the five presets' wire tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Arc, Mutex};

    use mockito::{Server, ServerGuard};
    use serde_json::{Value, json};

    /// Register a `/chat/completions` mock on `server` that records the JSON body of the request
    /// it receives and answers with a minimal well-formed completion. Returns the capture slot.
    pub(crate) async fn mock_completion(server: &mut ServerGuard) -> Arc<Mutex<Option<Value>>> {
        let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&captured);
        server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body_from_request(move |req| {
                let text = req.utf8_lossy_body().unwrap_or_default().into_owned();
                *slot.lock().unwrap() = serde_json::from_str(&text).ok();
                json!({
                    "id": "cmpl-1",
                    "model": "test-model",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": "ok"},
                        "finish_reason": "stop"
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
                })
                .to_string()
                .into_bytes()
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

    /// How many times `port` hits a mock endpoint that answers every request with HTTP 500,
    /// for one `generate` call. Asserts nothing about the error beyond it being one.
    pub(crate) async fn attempts_against_a_failing_endpoint(
        mut server: ServerGuard,
        port: &dyn paladin_ports::output::llm_port::LlmPort,
        model: &str,
        expected_attempts: usize,
    ) {
        use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
        use paladin_ports::output::llm_port::LlmRequest;

        let mock = server
            .mock("POST", "/chat/completions")
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
}
