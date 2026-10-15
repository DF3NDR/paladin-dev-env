//! Phase 50 provider-swap matrix (PARITY-01, SC-1): one agent definition, nine providers, the
//! provider switched only through an `LlmProviderConfig` block, every assertion made on the wire
//! body a mock transport recorded or on the outcome the execution service returned.
//!
//! What it proves: a deployment that changes `llm.<provider>` in its configuration -- and nothing
//! else -- gets the same agent behaviour from every provider, or a refusal that names the
//! provider, the model and the parameter. It is the layer-crossing check for the whole phase: the
//! config block, the adapter built from it, `PaladinBuilder`, `PaladinExecutionService`, the
//! adapter's request builder and the wire.
//!
//! Red-first rule: this file was written against the tree as it stood before the Phase 50 fixes.
//! Every case that tree failed carries `#[ignore = "red until plan 50-NN: ..."]` naming the plan
//! that fixes it, and the failing output of that run is recorded in
//! `.planning/phases/50-provider-parity-config-driven-model-swap/50-MATRIX-RED-EVIDENCE.md`. A
//! case is un-ignored only by the plan that fixes the behaviour it names; its assertion is never
//! weakened, narrowed to fewer providers, or deleted.
//!
//! This file is the regression net for Phase 50.
//!
//! ```bash
//! cargo test --test provider_swap_matrix --features llm-all
//! cargo test --test provider_swap_matrix --features llm-all -- --ignored   # the red cases
//! ```
//!
//! Credentials: the only key is the literal `matrix-test-key`, sent to a local mock server. No
//! environment variable is read and nothing leaves the process.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use mockito::{Matcher, ServerGuard};
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::core::platform::container::paladin::Paladin;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{LlmPort, PaladinError};
use paladin_llm::config::llm::LlmProviderConfig;
use serde_json::{Value, json};

/// The query every case sends: multibyte on purpose, so a normalising or truncating adapter fails.
const MATRIX_QUERY: &str = "Grüße — 你好 🚀 Name the capital of France in one word.";

/// The system prompt of the one agent definition every case runs.
const MATRIX_SYSTEM_PROMPT: &str = "You are a concise assistant.";

/// A mock vendor endpoint: answers every POST on any path with one caller-supplied body and
/// records every request body it receives, in order.
struct MockVendor {
    server: ServerGuard,
    bodies: Arc<Mutex<Vec<Value>>>,
}

impl MockVendor {
    /// Start a mock that answers every POST with `status` and `body`.
    async fn start(status: usize, body: Value) -> Self {
        let mut server = mockito::Server::new_async().await;
        let bodies: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&bodies);
        let response = body.to_string().into_bytes();
        server
            .mock("POST", Matcher::Any)
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |request| {
                let parsed = request
                    .body()
                    .ok()
                    .and_then(|raw| serde_json::from_slice::<Value>(raw).ok())
                    .unwrap_or(Value::Null);
                if let Ok(mut guard) = recorder.lock() {
                    guard.push(parsed);
                }
                response.clone()
            })
            .create_async()
            .await;
        Self { server, bodies }
    }

    /// The server's base URL (no trailing slash).
    fn url(&self) -> String {
        self.server.url()
    }

    /// Every request body received so far, in arrival order.
    fn bodies(&self) -> Vec<Value> {
        self.bodies
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

/// The `llm.<provider>` block a deployment would write: only the endpoint and the model change
/// between providers.
fn matrix_block(base_url: &str, model: &str) -> LlmProviderConfig {
    LlmProviderConfig {
        api_key: "matrix-test-key".to_string(),
        base_url: Some(base_url.to_string()),
        default_model: Some(model.to_string()),
        default_temperature: None,
        timeout_seconds: Some(5),
        max_retries: Some(0),
    }
}

/// Build the named provider's port from a config block, through that provider's existing
/// `From<&LlmProviderConfig>` bridge -- the config-only switch available today.
///
/// Plan 50-10 replaces this body with `LlmProviderFactory::create_from_config` without changing
/// any case.
fn port_from_config(provider: &str, block: &LlmProviderConfig) -> Arc<dyn LlmPort> {
    match provider {
        "openai" => {
            let config = paladin_llm::openai::OpenAIConfig::from(block);
            Arc::new(
                paladin_llm::openai::OpenAIAdapter::new(config)
                    .expect("the matrix block must build an OpenAI adapter"),
            )
        }
        other => panic!("matrix has no constructor for provider '{other}'"),
    }
}

/// The builder every case starts from, mirroring `build_agent_with_llm` in
/// `src/infrastructure/web/agent_host.rs`: name, system prompt and model first; the caller adds
/// temperature, stop words and the rest the definition sets.
fn agent_builder(port: Arc<dyn LlmPort>, model: &str) -> PaladinBuilder {
    PaladinBuilder::new(port)
        .name("matrix-agent")
        .system_prompt(MATRIX_SYSTEM_PROMPT)
        .model(model)
        .max_loops(1)
}

/// The execution service every case runs the agent through.
fn execution_service(port: Arc<dyn LlmPort>) -> PaladinExecutionService {
    PaladinExecutionService::new(
        port,
        Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30))),
        None,
        None,
    )
}

/// An OpenAI-chat success body.
fn openai_chat_success(text: &str) -> Value {
    json!({
        "id": "chatcmpl-matrix",
        "object": "chat.completion",
        "created": 0,
        "model": "matrix",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15 }
    })
}

/// The content of the last `messages[]` entry whose role is `user`, if the body has one.
fn last_user_content(body: &Value) -> Option<&str> {
    body.get("messages")?
        .as_array()?
        .iter()
        .rev()
        .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))?
        .get("content")?
        .as_str()
}

/// Tracer (PARITY-08): an OpenAI agent built from a config block and run through the execution
/// service puts the query text on the wire as the last user message.
#[tokio::test]
async fn openai_prompt_reaches_the_wire() {
    let vendor = MockVendor::start(200, openai_chat_success("Paris")).await;
    let block = matrix_block(&vendor.url(), "matrix-openai-model");
    let port = port_from_config("openai", &block);

    let paladin: Paladin = agent_builder(Arc::clone(&port), "matrix-openai-model")
        .build()
        .await
        .expect("the matrix agent must build");
    let outcome: Result<_, PaladinError> = execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await;
    outcome.expect("the matrix agent must execute against the mock");

    let bodies = vendor.bodies();
    let last = bodies
        .last()
        .expect("the mock must have received a request");
    // The execution service folds the system prompt and the query into the one prompt string it
    // hands the adapter as `UserPrompt::query`, so the query text is carried inside the user
    // message -- byte for byte -- rather than being the whole of it.
    let content = last_user_content(last).unwrap_or_default();
    assert!(
        content.contains(MATRIX_QUERY),
        "openai: the query must reach the wire inside the last user message; body was {last}"
    );
}
