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
//! Every case that tree failed carries an `#[ignore]` whose reason begins "red until plan" and
//! names the plan that fixes it, and the failing output of that run is recorded in
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
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{LlmPort, PaladinError};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::transience::Transience;
use paladin_llm::config::llm::LlmProviderConfig;
use paladin_llm::fallback::FallbackLlmAdapter;
use paladin_ports::output::llm_port::{LlmRequest, LlmResponse};
use serde_json::{Value, json};

/// The query every case sends: multibyte on purpose, so a normalising or truncating adapter fails.
const MATRIX_QUERY: &str = "Grüße — 你好 🚀 Name the capital of France in one word.";

/// The system prompt of the one agent definition every case runs.
const MATRIX_SYSTEM_PROMPT: &str = "You are a concise assistant.";

/// The run-metadata key under which an adapter records the parameters it did not send
/// (introduced by plan 50-03).
const PARAMS_OMITTED_KEY: &str = "paladin.params_omitted";

/// The three request/response shapes the nine providers speak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wire {
    /// OpenAI chat completions: OpenAI, DeepSeek, Kimi, Qwen, Grok, Ollama, OpenAI-compatible.
    OpenAiChat,
    /// Anthropic messages.
    Anthropic,
    /// Gemini `generateContent`.
    Gemini,
}

/// One row of the matrix: a provider, the model its block names, and the wire it speaks.
#[derive(Clone, Copy, Debug)]
struct Provider {
    /// The name the provider is registered under (`llm.<name>`).
    name: &'static str,
    /// The model the matrix agent names for this provider.
    model: &'static str,
    /// The wire shape its mock speaks.
    wire: Wire,
}

/// The nine providers, in registry order.
const PROVIDERS: [&str; 9] = [
    "openai",
    "anthropic",
    "deepseek",
    "kimi",
    "qwen",
    "grok",
    "gemini",
    "ollama",
    "openai-compatible",
];

/// Look up a provider's matrix row.
fn provider(name: &str) -> Provider {
    use paladin_llm::gemini::adapter::GEMINI_DEFAULT_MODEL;
    use paladin_llm::grok::adapter::GROK_DEFAULT_MODEL;
    use paladin_llm::kimi::adapter::KIMI_DEFAULT_MODEL;
    use paladin_llm::ollama::adapter::OLLAMA_DEFAULT_MODEL;
    use paladin_llm::qwen::adapter::QWEN_DEFAULT_MODEL;

    let (name, model, wire) = match name {
        "openai" => ("openai", "matrix-openai-model", Wire::OpenAiChat),
        "anthropic" => ("anthropic", "matrix-anthropic-model", Wire::Anthropic),
        "deepseek" => ("deepseek", "matrix-deepseek-model", Wire::OpenAiChat),
        "kimi" => ("kimi", KIMI_DEFAULT_MODEL, Wire::OpenAiChat),
        "qwen" => ("qwen", QWEN_DEFAULT_MODEL, Wire::OpenAiChat),
        "grok" => ("grok", GROK_DEFAULT_MODEL, Wire::OpenAiChat),
        "gemini" => ("gemini", GEMINI_DEFAULT_MODEL, Wire::Gemini),
        "ollama" => ("ollama", OLLAMA_DEFAULT_MODEL, Wire::OpenAiChat),
        "openai-compatible" => (
            "openai-compatible",
            "matrix-compatible-model",
            Wire::OpenAiChat,
        ),
        other => panic!("matrix has no row for provider '{other}'"),
    };
    Provider { name, model, wire }
}

/// A mock vendor endpoint: answers every POST on any path with one caller-supplied status and
/// body and records every request it receives (path and JSON body), in order.
struct MockVendor {
    server: ServerGuard,
    requests: Arc<Mutex<Vec<(String, Value)>>>,
}

impl MockVendor {
    /// Start a mock that answers every POST with `status` and `body`.
    async fn start(status: usize, body: Value) -> Self {
        let mut server = mockito::Server::new_async().await;
        let requests: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&requests);
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
                    guard.push((request.path().to_string(), parsed));
                }
                response.clone()
            })
            .create_async()
            .await;
        Self { server, requests }
    }

    /// The server's base URL (no trailing slash).
    fn url(&self) -> String {
        self.server.url()
    }

    /// Every request body received so far, in arrival order.
    fn bodies(&self) -> Vec<Value> {
        self.requests
            .lock()
            .map(|guard| guard.iter().map(|(_, body)| body.clone()).collect())
            .unwrap_or_default()
    }

    /// Every request path received so far, in arrival order.
    fn paths(&self) -> Vec<String> {
        self.requests
            .lock()
            .map(|guard| guard.iter().map(|(path, _)| path.clone()).collect())
            .unwrap_or_default()
    }

    /// The most recent request body; the case fails if the vendor was never called.
    fn last_body(&self, provider: &str) -> Value {
        self.bodies()
            .pop()
            .unwrap_or_else(|| panic!("{provider}: the mock vendor received no request"))
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
/// `From<&LlmProviderConfig>` bridge into its `*Config` and then `*Adapter::new` -- the
/// config-only switch available today.
///
/// Plan 50-10 replaces this body with `LlmProviderFactory::create_from_config` without changing
/// any case.
fn port_from_config(provider: &str, block: &LlmProviderConfig) -> Arc<dyn LlmPort> {
    const BUILD: &str = "the matrix block must build an adapter";
    match provider {
        "openai" => Arc::new(
            paladin_llm::openai::OpenAIAdapter::new(paladin_llm::openai::OpenAIConfig::from(block))
                .expect(BUILD),
        ),
        "anthropic" => Arc::new(
            paladin_llm::anthropic::AnthropicAdapter::new(
                paladin_llm::anthropic::AnthropicConfig::from(block),
            )
            .expect(BUILD),
        ),
        "deepseek" => Arc::new(
            paladin_llm::deepseek::DeepSeekAdapter::new(
                paladin_llm::deepseek::DeepSeekConfig::from(block),
            )
            .expect(BUILD),
        ),
        "kimi" => Arc::new(
            paladin_llm::kimi::KimiAdapter::new(paladin_llm::kimi::KimiConfig::from(block))
                .expect(BUILD),
        ),
        "qwen" => Arc::new(
            paladin_llm::qwen::QwenAdapter::new(paladin_llm::qwen::QwenConfig::from(block))
                .expect(BUILD),
        ),
        "grok" => Arc::new(
            paladin_llm::grok::GrokAdapter::new(paladin_llm::grok::GrokConfig::from(block))
                .expect(BUILD),
        ),
        "gemini" => Arc::new(
            paladin_llm::gemini::GeminiAdapter::new(paladin_llm::gemini::GeminiConfig::from(block))
                .expect(BUILD),
        ),
        "ollama" => Arc::new(
            paladin_llm::ollama::OllamaAdapter::new(paladin_llm::ollama::OllamaConfig::from(block))
                .expect(BUILD),
        ),
        // `OpenAiCompatibleConfig` is a `TryFrom`: the generic provider has no vendor to default
        // `base_url` and `model` from, so the block must name both (the matrix block does).
        "openai-compatible" => Arc::new(
            paladin_llm::openai_compatible::OpenAiCompatibleAdapter::new(
                paladin_llm::openai_compatible::OpenAiCompatibleConfig::try_from(block)
                    .expect(BUILD),
            )
            .expect(BUILD),
        ),
        other => panic!("matrix has no constructor for provider '{other}'"),
    }
}

/// A mock vendor for `provider` answering `body`, and the provider's port built from the block
/// that points at it.
async fn vendor_and_port(
    provider: &Provider,
    status: usize,
    body: Value,
) -> (MockVendor, Arc<dyn LlmPort>) {
    let vendor = MockVendor::start(status, body).await;
    let block = matrix_block(&vendor.url(), provider.model);
    let port = port_from_config(provider.name, &block);
    (vendor, port)
}

/// The builder every case starts from, mirroring `build_agent_with_llm` in
/// `src/infrastructure/web/agent_host.rs`: name, system prompt, model and `max_loops` first (one
/// loop, so one request per run); the caller adds temperature, stop words and the rest the
/// definition sets.
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

/// A direct (agent-less) request for `generate`, as the execution service would shape it.
fn llm_request(
    model: &str,
    query: &str,
    context: Option<&str>,
    temperature: Option<f32>,
) -> LlmRequest {
    let mut request = LlmRequest::new(
        model,
        PromptItem::new(PromptType::User(UserPrompt {
            query: query.to_string(),
            context: context.map(str::to_string),
        }))
        .expect("a user prompt must build"),
    );
    request.prompt.entity.data.parameters.temperature = temperature;
    request
}

// ---- wire bodies -------------------------------------------------------------------------

/// A normal-stop success body in the provider's wire shape.
fn success_body(wire: Wire, text: &str) -> Value {
    match wire {
        Wire::OpenAiChat => json!({
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
        }),
        Wire::Anthropic => json!({
            "id": "msg_matrix",
            "type": "message",
            "role": "assistant",
            "model": "matrix",
            "content": [{ "type": "text", "text": text }],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 12, "output_tokens": 3 }
        }),
        Wire::Gemini => json!({
            "candidates": [{
                "content": { "role": "model", "parts": [{ "text": text }] },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 3,
                "totalTokenCount": 15
            }
        }),
    }
}

/// The wire's normal stop finish with an empty completion: what a model that emitted nothing
/// (or only a stop sequence) returns.
fn empty_stop_body(wire: Wire) -> Value {
    match wire {
        Wire::OpenAiChat => success_body(wire, ""),
        Wire::Anthropic => json!({
            "id": "msg_matrix",
            "type": "message",
            "role": "assistant",
            "model": "matrix",
            "content": [],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": { "input_tokens": 12, "output_tokens": 0 }
        }),
        Wire::Gemini => success_body(wire, ""),
    }
}

// ---- reading the wire --------------------------------------------------------------------

/// The text of the last user turn in a request body, in the provider's wire shape.
fn last_user_text(wire: Wire, body: &Value) -> Option<String> {
    match wire {
        Wire::OpenAiChat | Wire::Anthropic => body
            .get("messages")?
            .as_array()?
            .iter()
            .rev()
            .find(|message| message.get("role").and_then(Value::as_str) == Some("user"))?
            .get("content")?
            .as_str()
            .map(str::to_string),
        Wire::Gemini => {
            let turn = body
                .get("contents")?
                .as_array()?
                .iter()
                .rev()
                .find(|content| content.get("role").and_then(Value::as_str) == Some("user"))?;
            let text: String = turn
                .get("parts")?
                .as_array()?
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect();
            Some(text)
        }
    }
}

/// The temperature a request body carries, if any.
fn sent_temperature(wire: Wire, body: &Value) -> Option<f64> {
    match wire {
        Wire::OpenAiChat | Wire::Anthropic => body.get("temperature")?.as_f64(),
        Wire::Gemini => body.get("generationConfig")?.get("temperature")?.as_f64(),
    }
}

/// The stop list a request body carries, if any, in the vendor's own field.
fn sent_stop(wire: Wire, body: &Value) -> Option<Vec<String>> {
    let field = match wire {
        Wire::OpenAiChat => body.get("stop")?,
        Wire::Anthropic => body.get("stop_sequences")?,
        Wire::Gemini => body.get("generationConfig")?.get("stopSequences")?,
    };
    match field {
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
        ),
        Value::String(single) => Some(vec![single.clone()]),
        _ => None,
    }
}

/// Whether a refusal message names the provider (by matrix name or by the name the port itself
/// reports), the model, and the parameter.
fn refusal_names(
    message: &str,
    provider: &Provider,
    port: &dyn LlmPort,
    parameters: &[&str],
) -> bool {
    let message = message.to_lowercase();
    let names_provider = message.contains(provider.name)
        || message.contains(&port.get_provider_name().to_lowercase());
    names_provider
        && message.contains(&provider.model.to_lowercase())
        && parameters
            .iter()
            .any(|parameter| message.contains(parameter))
}

// ---- cases -------------------------------------------------------------------------------

/// (a) The query text reaches the wire: the agent runs through the execution service and the
/// last user turn on the wire carries the multibyte query byte for byte.
///
/// The execution service folds the system prompt and the query into the one prompt string it
/// hands the adapter as `UserPrompt::query`, so the query is carried inside the user turn rather
/// than being the whole of it.
async fn case_prompt_reaches_the_wire(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let paladin = agent_builder(Arc::clone(&port), p.model)
        .build()
        .await
        .unwrap_or_else(|error| panic!("{name}: the matrix agent must build: {error}"));
    execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the matrix agent must execute: {error}"));

    let last = vendor.last_body(name);
    let content = last_user_text(p.wire, &last).unwrap_or_default();
    assert!(
        content.contains(MATRIX_QUERY),
        "{name}: the query must reach the wire inside the last user turn; body was {last}"
    );
}

/// The agent's `.model(..)` reaches the wire (Gemini carries the model in the request path).
async fn case_model_reaches_the_wire(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let paladin = agent_builder(Arc::clone(&port), p.model)
        .build()
        .await
        .unwrap_or_else(|error| panic!("{name}: the matrix agent must build: {error}"));
    execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the matrix agent must execute: {error}"));

    let last = vendor.last_body(name);
    let on_the_wire = match p.wire {
        Wire::Gemini => vendor
            .paths()
            .last()
            .is_some_and(|path| path.contains(p.model)),
        Wire::OpenAiChat | Wire::Anthropic => {
            last.get("model").and_then(Value::as_str) == Some(p.model)
        }
    };
    assert!(
        on_the_wire,
        "{name}: model '{}' must reach the wire; paths {:?}, body {last}",
        p.model,
        vendor.paths()
    );
}

/// A present `UserPrompt::context` is appended to the query by every provider: query, one blank
/// line, context (PARITY-08, the shared `user_turn_text` rule).
async fn case_context_reaches_the_wire(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "4")).await;

    port.generate(llm_request(
        p.model,
        "What is 2+2?",
        Some("Extra context."),
        None,
    ))
    .await
    .unwrap_or_else(|error| panic!("{name}: the direct call must succeed: {error}"));

    let last = vendor.last_body(name);
    assert_eq!(
        last_user_text(p.wire, &last).as_deref(),
        Some("What is 2+2?\n\nExtra context."),
        "{name}: the user turn must be the query, one blank line, then the context; body was {last}"
    );
}

/// (b) The same agent definition -- temperature `0.5`, stop words `END` and `STOP` -- is either
/// refused at build by name (provider, model and parameter) or carries both values on the wire
/// in the vendor's own fields. Nothing else passes: a built agent whose value is missing or
/// changed on the wire is a silent divergence.
async fn case_same_agent_is_equivalent_or_refused_by_name(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let built = agent_builder(Arc::clone(&port), p.model)
        .temperature(0.5)
        .add_stop_word("END")
        .add_stop_word("STOP")
        .build()
        .await;

    let paladin = match built {
        Err(PaladinError::ConfigurationError(message)) => {
            assert!(
                refusal_names(&message, &p, port.as_ref(), &["temperature", "stop"]),
                "{name}: a refusal must name the provider, the model '{}' and the parameter \
                 (temperature or stop); it said: {message}",
                p.model
            );
            return;
        }
        Err(other) => {
            panic!("{name}: the build failed with something other than a named refusal: {other}")
        }
        Ok(paladin) => paladin,
    };

    execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the built agent must execute: {error}"));

    let last = vendor.last_body(name);
    let temperature = sent_temperature(p.wire, &last);
    assert_eq!(
        temperature,
        Some(0.5),
        "{name}: the agent was built with temperature 0.5 but the wire carries {temperature:?}; body was {last}"
    );
    let stop = sent_stop(p.wire, &last);
    assert_eq!(
        stop,
        Some(vec!["END".to_string(), "STOP".to_string()]),
        "{name}: the agent was built with stop words [END, STOP] but the wire carries {stop:?}; body was {last}"
    );
}

/// (e) A normal-stop completion with no text is a typed, permanent `Empty completion` failure,
/// never an empty successful answer.
async fn case_empty_stop_completion_is_typed(name: &'static str) {
    let p = provider(name);
    let (_vendor, port) = vendor_and_port(&p, 200, empty_stop_body(p.wire)).await;

    let paladin = agent_builder(Arc::clone(&port), p.model)
        .build()
        .await
        .unwrap_or_else(|error| panic!("{name}: the matrix agent must build: {error}"));
    let outcome = execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await;

    match outcome {
        Err(PaladinError::LlmFailure {
            transience,
            message,
            ..
        }) => {
            assert_eq!(
                transience,
                Transience::Permanent,
                "{name}: an empty stop completion must be permanent; message was: {message}"
            );
            assert!(
                message.contains("Empty completion"),
                "{name}: the failure must be the typed empty completion; message was: {message}"
            );
        }
        Err(other) => {
            panic!("{name}: expected a typed LlmFailure for an empty stop completion, got {other}")
        }
        Ok(result) => panic!(
            "{name}: an empty stop completion was returned as a successful answer: {:?}",
            result.output
        ),
    }
}

/// (c) Kimi accepts only temperature `1.0` (a `(1, 1)` range): an explicit `1.0` builds and is
/// sent; anything else is refused at build, naming provider, model and parameter.
async fn case_fixed_temperature_explicit(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let accepted = agent_builder(Arc::clone(&port), p.model)
        .temperature(1.0)
        .build()
        .await
        .unwrap_or_else(|error| panic!("{name}: an explicit temperature 1.0 must build: {error}"));
    execution_service(Arc::clone(&port))
        .execute(&accepted, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the temperature-1.0 agent must execute: {error}"));
    let last = vendor.last_body(name);
    assert_eq!(
        sent_temperature(p.wire, &last),
        Some(1.0),
        "{name}: an explicit temperature 1.0 must be on the wire; body was {last}"
    );

    let just_above_one = f32::from_bits(1.0f32.to_bits() + 1);
    for refused in [0.5_f32, just_above_one] {
        match agent_builder(Arc::clone(&port), p.model)
            .temperature(refused)
            .build()
            .await
        {
            Err(PaladinError::ConfigurationError(message)) => assert!(
                refusal_names(&message, &p, port.as_ref(), &["temperature"]),
                "{name}: the refusal of temperature {refused} must name the provider, the model '{}' \
                 and temperature; it said: {message}",
                p.model
            ),
            Err(other) => panic!(
                "{name}: temperature {refused} failed with something other than a named refusal: {other}"
            ),
            Ok(_) => panic!(
                "{name}: temperature {refused} is outside the provider's range but the agent built"
            ),
        }
    }
}

/// (d) With auto temperature on a fixed-temperature provider, no request body carries a
/// temperature, and a direct call that asks for one gets the omission recorded in the response
/// metadata.
async fn case_fixed_temperature_auto(name: &'static str) {
    let p = provider(name);
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let paladin = agent_builder(Arc::clone(&port), p.model)
        .auto_temperature(true)
        .agent_description("Write a short creative story about a lighthouse.")
        .build()
        .await
        .unwrap_or_else(|error| panic!("{name}: an auto-temperature agent must build: {error}"));
    execution_service(Arc::clone(&port))
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the auto-temperature agent must execute: {error}"));

    // The auto-temperature service may issue its own classification call through the same
    // port, so every recorded body is checked, not only the last.
    for body in vendor.bodies() {
        assert!(
            sent_temperature(p.wire, &body).is_none() && body.get("temperature").is_none(),
            "{name}: a fixed-temperature provider must never be sent a temperature; body was {body}"
        );
    }

    let response: LlmResponse = port
        .generate(llm_request(p.model, "Name a colour.", None, Some(0.85)))
        .await
        .unwrap_or_else(|error| panic!("{name}: the direct call must succeed: {error}"));
    let omitted = response.metadata.get(PARAMS_OMITTED_KEY);
    assert!(
        omitted.is_some_and(|value| value.contains("temperature")),
        "{name}: the response metadata under '{PARAMS_OMITTED_KEY}' must list temperature; metadata was {:?}",
        response.metadata
    );
}

/// (f) A fallback hop is sent its own model: a request for the primary's model that falls over
/// to the next provider reaches it under that provider's model, not the primary's.
async fn case_fallback_hop_gets_its_own_model() {
    let primary = provider("anthropic");
    let secondary = provider("kimi");

    let (_primary_vendor, primary_port) = vendor_and_port(
        &primary,
        503,
        json!({ "error": { "message": "overloaded", "type": "overloaded_error" } }),
    )
    .await;
    let (secondary_vendor, secondary_port) =
        vendor_and_port(&secondary, 200, success_body(secondary.wire, "Paris")).await;

    let chain = FallbackLlmAdapter::new(vec![primary_port, secondary_port])
        .expect("a two-element chain must build");
    chain
        .generate(llm_request(primary.model, "Name a colour.", None, None))
        .await
        .unwrap_or_else(|error| panic!("the chain must answer from the second provider: {error}"));

    let last = secondary_vendor.last_body("kimi");
    assert_eq!(
        last.get("model").and_then(Value::as_str),
        Some(secondary.model),
        "the second hop must be sent its own model '{}', not the primary's; body was {last}",
        secondary.model
    );
}

// ---- the matrix --------------------------------------------------------------------------

/// Emit one `#[tokio::test]` per (provider, case). Each test owns its own `MockVendor`, so cases
/// share no state and run in any order. Attributes (the `#[ignore]` markers that name the fixing
/// plan) go before the test name.
macro_rules! matrix_case {
    ($(#[$meta:meta])* $test:ident => $case:ident($provider:literal)) => {
        $(#[$meta])*
        #[tokio::test]
        async fn $test() {
            $case($provider).await
        }
    };
}

// (a) the prompt on the wire -- all nine
matrix_case!(openai_prompt_reaches_the_wire => case_prompt_reaches_the_wire("openai"));
matrix_case!(anthropic_prompt_reaches_the_wire => case_prompt_reaches_the_wire("anthropic"));
matrix_case!(deepseek_prompt_reaches_the_wire => case_prompt_reaches_the_wire("deepseek"));
matrix_case!(kimi_prompt_reaches_the_wire => case_prompt_reaches_the_wire("kimi"));
matrix_case!(qwen_prompt_reaches_the_wire => case_prompt_reaches_the_wire("qwen"));
matrix_case!(grok_prompt_reaches_the_wire => case_prompt_reaches_the_wire("grok"));
matrix_case!(gemini_prompt_reaches_the_wire => case_prompt_reaches_the_wire("gemini"));
matrix_case!(ollama_prompt_reaches_the_wire => case_prompt_reaches_the_wire("ollama"));
matrix_case!(openai_compatible_prompt_reaches_the_wire => case_prompt_reaches_the_wire("openai-compatible"));

// the agent's model on the wire -- all nine
matrix_case!(openai_model_reaches_the_wire => case_model_reaches_the_wire("openai"));
matrix_case!(anthropic_model_reaches_the_wire => case_model_reaches_the_wire("anthropic"));
matrix_case!(deepseek_model_reaches_the_wire => case_model_reaches_the_wire("deepseek"));
matrix_case!(kimi_model_reaches_the_wire => case_model_reaches_the_wire("kimi"));
matrix_case!(qwen_model_reaches_the_wire => case_model_reaches_the_wire("qwen"));
matrix_case!(grok_model_reaches_the_wire => case_model_reaches_the_wire("grok"));
matrix_case!(gemini_model_reaches_the_wire => case_model_reaches_the_wire("gemini"));
matrix_case!(ollama_model_reaches_the_wire => case_model_reaches_the_wire("ollama"));
matrix_case!(openai_compatible_model_reaches_the_wire => case_model_reaches_the_wire("openai-compatible"));

// a present context on the wire -- all nine (PARITY-08)
matrix_case!(openai_context_reaches_the_wire => case_context_reaches_the_wire("openai"));
matrix_case!(anthropic_context_reaches_the_wire => case_context_reaches_the_wire("anthropic"));
matrix_case!(deepseek_context_reaches_the_wire => case_context_reaches_the_wire("deepseek"));
matrix_case!(kimi_context_reaches_the_wire => case_context_reaches_the_wire("kimi"));
matrix_case!(qwen_context_reaches_the_wire => case_context_reaches_the_wire("qwen"));
matrix_case!(grok_context_reaches_the_wire => case_context_reaches_the_wire("grok"));
matrix_case!(gemini_context_reaches_the_wire => case_context_reaches_the_wire("gemini"));
matrix_case!(ollama_context_reaches_the_wire => case_context_reaches_the_wire("ollama"));
matrix_case!(openai_compatible_context_reaches_the_wire => case_context_reaches_the_wire("openai-compatible"));

// (b) the same agent is equivalent or refused by name -- all nine
matrix_case!(#[ignore = "red until plan 50-05: the stop words are not sent on the wire"] openai_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("openai"));
matrix_case!(#[ignore = "red until plan 50-05: neither the temperature nor the stop words are sent on the wire"] anthropic_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("anthropic"));
matrix_case!(#[ignore = "red until plan 50-05: the stop words are not sent on the wire"] deepseek_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("deepseek"));
matrix_case!(#[ignore = "red until plan 50-03: the (1,1) temperature refusal names the provider but not the model"] kimi_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("kimi"));
matrix_case!(#[ignore = "red until plan 50-03: the compat presets neither send the stop words nor refuse them by name"] qwen_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("qwen"));
matrix_case!(#[ignore = "red until plan 50-03: the compat presets neither send the stop words nor refuse them by name"] grok_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("grok"));
matrix_case!(gemini_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("gemini"));
matrix_case!(#[ignore = "red until plan 50-03: the compat presets neither send the stop words nor refuse them by name"] ollama_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("ollama"));
matrix_case!(#[ignore = "red until plan 50-03: the compat presets neither send the stop words nor refuse them by name"] openai_compatible_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("openai-compatible"));

// (c) and (d) the fixed-temperature provider
matrix_case!(#[ignore = "red until plan 50-03: an explicit 1.0 is not sent, and the refusal of other values does not name the model"] kimi_fixed_temperature_explicit => case_fixed_temperature_explicit("kimi"));
matrix_case!(#[ignore = "red until plan 50-03: an auto-selected temperature is refused at build instead of being dropped and recorded"] kimi_fixed_temperature_auto => case_fixed_temperature_auto("kimi"));

// (e) an empty stop completion is typed -- all nine
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] openai_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("openai"));
matrix_case!(anthropic_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("anthropic"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] deepseek_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("deepseek"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] kimi_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("kimi"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] qwen_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("qwen"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] grok_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("grok"));
matrix_case!(#[ignore = "red until plan 50-13: an empty stop completion is returned as a successful empty answer"] gemini_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("gemini"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] ollama_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("ollama"));
matrix_case!(#[ignore = "red until plan 50-12: an empty stop completion is returned as a successful empty answer"] openai_compatible_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("openai-compatible"));

// (f) a fallback hop is sent its own model
#[ignore = "red until plan 50-14: the fallback hop is sent the primary's model, not its own"]
#[tokio::test]
async fn fallback_hop_gets_its_own_model() {
    case_fallback_hop_gets_its_own_model().await
}

/// The provider list the macro invocations above cover, so a provider added to the registry but
/// not to the matrix fails here rather than passing unmeasured.
#[test]
fn matrix_covers_every_provider() {
    assert_eq!(PROVIDERS.len(), 9);
    for name in PROVIDERS {
        let row = provider(name);
        assert_eq!(row.name, name);
        assert!(
            !row.model.is_empty(),
            "{name}: the matrix model must be named"
        );
    }
}
