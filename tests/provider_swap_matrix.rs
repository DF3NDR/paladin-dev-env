//! Phase 50 provider-swap matrix (PARITY-01, SC-1): one agent definition, nine providers, the
//! provider switched only through an `LlmProviderConfig` block (built with
//! `LlmProviderFactory::create_from_config` from one `LlmConfig`), every assertion made on the wire
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
use paladin::application::services::paladin::middleware::{ModelCallContext, PromptAssembly};
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::{
    AgentLoopSettlement, PaladinExecutionService,
};
use paladin::config::agent_runtime::{
    AgentRuntimeConfig, AgentRuntimeDeps, FallbackStepConfig, ModelFallbackConfig,
};
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::{LlmPort, PaladinError};
use paladin::{LlmConfig, LlmProviderConfig, LlmProviderFactory};
use paladin_core::platform::container::cost::{Cost, CurrencyCode, PriceRow, PriceTable};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_core::platform::container::token_usage::TokenUsage;
use paladin_core::platform::container::transience::Transience;
use paladin_core::platform::container::treasury_ledger::{SpendGroupBy, SpendQuery};
use paladin_llm::fallback::{FallbackLlmAdapter, FallbackStep};
use paladin_llm::pricing::with_pricing;
use paladin_ports::output::llm_port::{LlmRequest, LlmResponse, StopSupport};
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_storage::treasury::in_memory::InMemoryTreasuryLedger;
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
    LlmProviderConfig::new("matrix-test-key")
        .with_base_url(base_url.to_string())
        .with_default_model(model.to_string())
        .with_timeout_seconds(5)
        .with_max_retries(0)
}

/// Build the named provider's port the way a deployment does: one `LlmConfig` holding that
/// provider's block, handed to `LlmProviderFactory::create_from_config` with the block's model
/// (D-12). The only per-provider code is the choice of which `llm.<provider>` field carries the
/// block; construction is the one public path, so a case switches provider by configuration
/// alone. Everything here comes from the facade (`paladin::...`), proving it is enough.
fn port_from_config(provider: &str, block: &LlmProviderConfig) -> Arc<dyn LlmPort> {
    let block = Some(block.clone());
    let mut config = LlmConfig::default();
    match provider {
        "openai" => config.openai = block,
        "anthropic" => config.anthropic = block,
        "deepseek" => config.deepseek = block,
        "kimi" => config.kimi = block,
        "qwen" => config.qwen = block,
        "grok" => config.grok = block,
        "gemini" => config.gemini = block,
        "ollama" => config.ollama = block,
        "openai-compatible" => config.openai_compatible = block,
        other => panic!("matrix has no block field for provider '{other}'"),
    }
    let model = config
        .get_provider_config(provider)
        .and_then(|block| block.default_model.clone());
    LlmProviderFactory::new()
        .create_from_config(&config, provider, model.as_deref())
        .unwrap_or_else(|error| panic!("the matrix block must build an adapter: {error}"))
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

/// `body` served under `model` and billed `prompt` / `completion` tokens, in the wire's own
/// usage shape (G-50-5a): the billed model and the usage are what the adapter reports and the
/// Treasurer prices, so a case that checks the ledger names both explicitly.
fn served_and_billed(
    wire: Wire,
    mut body: Value,
    model: &str,
    prompt: u32,
    completion: u32,
) -> Value {
    match wire {
        Wire::OpenAiChat => {
            body["model"] = json!(model);
            body["usage"] = json!({
                "prompt_tokens": prompt,
                "completion_tokens": completion,
                "total_tokens": prompt + completion
            });
        }
        Wire::Anthropic => {
            body["model"] = json!(model);
            body["usage"] = json!({ "input_tokens": prompt, "output_tokens": completion });
        }
        Wire::Gemini => {
            body["modelVersion"] = json!(model);
            body["usageMetadata"] = json!({
                "promptTokenCount": prompt,
                "candidatesTokenCount": completion,
                "totalTokenCount": prompt + completion
            });
        }
    }
    body
}

/// An OpenAI-chat body whose model refused through the wire's typed `refusal` field
/// (`content: null`), billed `prompt` / `completion` tokens on `model`.
fn openai_refusal_body(model: &str, prompt: u32, completion: u32) -> Value {
    let body = json!({
        "id": "chatcmpl-matrix",
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": null, "refusal": "I can't help with that." },
            "finish_reason": "stop"
        }]
    });
    served_and_billed(Wire::OpenAiChat, body, model, prompt, completion)
}

/// An OpenAI-chat body that hit the length cap with no text (all the budget spent thinking),
/// billed `prompt` / `completion` tokens on `model`.
fn openai_empty_length_body(model: &str, prompt: u32, completion: u32) -> Value {
    let mut body = success_body(Wire::OpenAiChat, "");
    body["choices"][0]["finish_reason"] = json!("length");
    served_and_billed(Wire::OpenAiChat, body, model, prompt, completion)
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
///
/// One declared exception (WR-01): a model whose profile takes no wire `stop`
/// (`StopSupport::NotSent`) does not refuse the build, because the execution service enforces
/// stop words client-side (`StopWordDetected`). Its wire must then carry no `stop` at all -- the
/// sampler omits the unsupported parameter -- while the temperature is still carried.
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

    let wire_takes_stop = port
        .model_profile(p.model)
        .is_none_or(|profile| !matches!(profile.stop_sequences, StopSupport::NotSent));
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
    if wire_takes_stop {
        assert_eq!(
            stop,
            Some(vec!["END".to_string(), "STOP".to_string()]),
            "{name}: the agent was built with stop words [END, STOP] but the wire carries {stop:?}; body was {last}"
        );
    } else {
        assert_eq!(
            stop, None,
            "{name}: the model takes no wire stop, so the stop words are enforced client-side and \
             none may be sent; the wire carries {stop:?}; body was {last}"
        );
    }
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

/// (c) Kimi's default model (`kimi-k2.6`) has a seeded entry from the operator's live probe of
/// 2026-10-15 (plan 50-06): every probed explicit temperature (`1.0`, `0.5`, just above `1.0`)
/// was rejected by the vendor ("only 0.6 is allowed for this model"), and the allowed value
/// depends on the thinking mode the probes disabled, so the entry states `NotSent` rather than
/// pinning one value. Every explicit temperature is therefore refused at build, naming provider,
/// model and parameter. (Before the seed this case asserted that an explicit `1.0` builds and is
/// sent, which was the adapter-wide fallback's `Fixed(1.0)` from the 2026-08-22 measurement.)
async fn case_fixed_temperature_explicit(name: &'static str) {
    let p = provider(name);
    let (_vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    let just_above_one = f32::from_bits(1.0f32.to_bits() + 1);
    for refused in [1.0_f32, 0.5_f32, just_above_one] {
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
                "{name}: the seeded entry for '{}' admits no temperature but temperature {refused} built",
                p.model
            ),
        }
    }
}

/// (c2) A Kimi model id with no seeded entry runs on the adapter-wide fallback, which declares
/// the same `NotSent` sampling parameters as the seeded rows (plan 50-20, G-50-5c). An explicit
/// temperature through the `llm.kimi` block path is refused at build naming provider, model and
/// parameter, while the same agent without one builds and sends no temperature.
async fn case_unseeded_kimi_refuses_an_explicit_temperature(name: &'static str) {
    let p = Provider {
        model: "kimi-unseeded-matrix-model",
        ..provider(name)
    };
    let (vendor, port) = vendor_and_port(&p, 200, success_body(p.wire, "Paris")).await;

    match agent_builder(Arc::clone(&port), p.model)
        .temperature(0.6)
        .build()
        .await
    {
        Err(PaladinError::ConfigurationError(message)) => assert!(
            refusal_names(&message, &p, port.as_ref(), &["temperature"]),
            "{name}: the refusal must name the provider, the model '{}' and temperature; \
             it said: {message}",
            p.model
        ),
        Err(other) => panic!(
            "{name}: an explicit temperature failed with something other than a named refusal: {other}"
        ),
        Ok(_) => panic!(
            "{name}: the fallback admits no temperature but an explicit one built for '{}'",
            p.model
        ),
    }

    let paladin = agent_builder(Arc::clone(&port), p.model)
        .build()
        .await
        .unwrap_or_else(|error| {
            panic!("{name}: the agent without a temperature must build: {error}")
        });
    execution_service(port)
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("{name}: the agent must execute: {error}"));
    for body in vendor.bodies() {
        assert!(
            body.get("temperature").is_none(),
            "{name}: an unseeded Kimi model must never be sent a temperature; body was {body}"
        );
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

    let chain = FallbackLlmAdapter::from_steps(vec![
        FallbackStep::new(primary_port, primary.model),
        FallbackStep::new(secondary_port, secondary.model),
    ])
    .expect("a two-step chain must build");
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

/// (f2) The same, through the configuration path (RESEARCH Open Question 7): `agent_runtime.
/// model_fallback.steps` built by `AgentRuntimeConfig::build_chain` from the operator's `llm:`
/// blocks, run through the middleware's own `before_model`, sends the second hop its own model on
/// the wire and names the hop and model that served.
async fn case_config_fallback_steps_send_each_hop_its_own_model() {
    let primary = provider("anthropic");
    let secondary = provider("kimi");
    let primary_vendor = MockVendor::start(
        503,
        json!({ "error": { "message": "overloaded", "type": "overloaded_error" } }),
    )
    .await;
    let secondary_vendor = MockVendor::start(200, success_body(secondary.wire, "Paris")).await;

    let llm = LlmConfig {
        anthropic: Some(matrix_block(
            &primary_vendor.url(),
            "block-default-anthropic",
        )),
        kimi: Some(matrix_block(&secondary_vendor.url(), "block-default-kimi")),
        ..LlmConfig::default()
    };
    let config = AgentRuntimeConfig {
        model_fallback: ModelFallbackConfig {
            enabled: true,
            steps: vec![
                FallbackStepConfig {
                    provider: "anthropic".to_string(),
                    model: primary.model.to_string(),
                },
                FallbackStepConfig {
                    provider: "kimi".to_string(),
                    model: secondary.model.to_string(),
                },
            ],
            ..ModelFallbackConfig::default()
        },
        ..AgentRuntimeConfig::default()
    };
    config.validate().expect("the steps configuration is valid");
    let chain = config
        .build_chain(&AgentRuntimeDeps {
            llm_config: Some(llm),
            ..AgentRuntimeDeps::default()
        })
        .unwrap_or_else(|error| panic!("the steps chain must build from the llm blocks: {error}"));
    assert_eq!(chain.len(), 1);

    let service_default: Arc<dyn LlmPort> = Arc::new(paladin::MockLlmAdapter::new());
    let paladin = agent_builder(Arc::clone(&service_default), primary.model)
        .build()
        .await
        .expect("the matrix agent must build");
    let mut cx = ModelCallContext::new(
        uuid::Uuid::new_v4(),
        &paladin,
        PromptAssembly::new(MATRIX_SYSTEM_PROMPT, "Name a colour.", "", vec![], None),
    );
    chain[0]
        .before_model(&mut cx)
        .await
        .expect("the fallback middleware installs its chain");
    let response = cx
        .effective_llm(&service_default)
        .generate(llm_request(primary.model, "Name a colour.", None, None))
        .await
        .unwrap_or_else(|error| panic!("the chain must answer from the second step: {error}"));

    let first = primary_vendor.last_body("anthropic");
    assert_eq!(
        first.get("model").and_then(Value::as_str),
        Some(primary.model),
        "the first step is sent its own model; body was {first}"
    );
    let last = secondary_vendor.last_body("kimi");
    assert_eq!(
        last.get("model").and_then(Value::as_str),
        Some(secondary.model),
        "the second step must be sent its own model '{}', not the first's; body was {last}",
        secondary.model
    );
    assert_eq!(
        response
            .metadata
            .get(paladin::SERVED_BY_METADATA_KEY)
            .map(String::as_str),
        Some("kimi")
    );
    assert_eq!(
        response
            .metadata
            .get(paladin::SERVED_MODEL_METADATA_KEY)
            .map(String::as_str),
        Some(secondary.model)
    );
}

/// The price table the ledger case charges against: a distinct rate per model, so a part
/// priced against the wrong model shows up as a wrong amount.
fn ledger_price_table(models: &[(&str, i64, i64)]) -> Arc<PriceTable> {
    let usd = CurrencyCode::new("USD").expect("USD is a currency code");
    let table = models.iter().fold(
        PriceTable::new(usd),
        |table, (model, prompt, completion)| {
            table.with_row(
                *model,
                PriceRow::new(*prompt, *completion).expect("a non-negative price row"),
            )
        },
    );
    Arc::new(table)
}

/// The ledger's settled spend grouped by `group_by`: group -> (nanos, settlement rows).
async fn ledger_spend(
    ledger: &InMemoryTreasuryLedger,
    group_by: SpendGroupBy,
) -> std::collections::BTreeMap<String, (i64, u64)> {
    ledger
        .spend(SpendQuery {
            group_by,
            ..SpendQuery::default()
        })
        .await
        .expect("the in-memory ledger answers a spend query")
        .into_iter()
        .map(|row| (row.group, (row.amount.nanos(), row.settlements)))
        .collect()
}

/// (g) G-50-5a end to end, through real adapters over mock HTTP, `PricingLlmAdapter`,
/// `FallbackLlmAdapter` and `PaladinExecutionService` with an `InMemoryTreasuryLedger`
/// (`AgentLoopSettlement::EveryCall`): every call a vendor billed is charged exactly once, priced
/// against the model that billed it (plans 50-26 to 50-30 together).
///
/// 1. A chain `[openai -> anthropic]` with `hop_on_refusal(true)`: OpenAI refuses through its
///    typed `refusal` field after billing 1_000 / 40 tokens; Anthropic answers with 1_000 /
///    2_000. The run succeeds, its usage and cost include the refused hop, and the ledger holds
///    ONE row for the call whose per-model breakdown names both models at their own prices.
/// 2. A single provider (Kimi, compat engine) answers an empty `length` completion after billing
///    1_000 / 62_000 tokens. The run fails with a permanent `LlmFailure`, and the ledger holds
///    one row carrying that call's price.
async fn case_billed_failures_and_hopped_refusals_reach_the_ledger() {
    // ---- 1. a hopped-past billed refusal is settled with the served call ----
    let refusing = provider("openai");
    let serving = provider("anthropic");
    let table = ledger_price_table(&[
        (refusing.model, 3_000_000_000, 15_000_000_000),
        (serving.model, 2_500_000_000, 10_000_000_000),
    ]);
    let (refusing_vendor, refusing_port) = vendor_and_port(
        &refusing,
        200,
        openai_refusal_body(refusing.model, 1_000, 40),
    )
    .await;
    let (serving_vendor, serving_port) = vendor_and_port(
        &serving,
        200,
        served_and_billed(
            serving.wire,
            success_body(serving.wire, "Paris"),
            serving.model,
            1_000,
            2_000,
        ),
    )
    .await;
    let chain = FallbackLlmAdapter::from_steps(vec![
        FallbackStep::new(refusing_port, refusing.model),
        FallbackStep::new(serving_port, serving.model),
    ])
    .expect("a two-step chain must build")
    .hop_on_refusal(true);
    let llm = with_pricing(Arc::new(chain), &table);

    let refused_usage = TokenUsage::new(1_000, 40);
    let served_usage = TokenUsage::new(1_000, 2_000);
    let c1 = table
        .price(refusing.model, &refused_usage)
        .expect("the refusing model is priced");
    let c2 = table
        .price(serving.model, &served_usage)
        .expect("the serving model is priced");
    let total: Cost = c1.checked_add(&c2).expect("one currency");

    let ledger = Arc::new(InMemoryTreasuryLedger::new());
    let paladin = agent_builder(Arc::clone(&llm), refusing.model)
        .build()
        .await
        .unwrap_or_else(|error| panic!("the chain agent must build: {error}"));
    let result = execution_service(llm)
        .with_treasury_ledger(ledger.clone(), AgentLoopSettlement::EveryCall)
        .execute(&paladin, MATRIX_QUERY)
        .await
        .unwrap_or_else(|error| panic!("the second hop must serve the run: {error}"));

    assert_eq!(
        refusing_vendor.bodies().len(),
        1,
        "the refusing vendor is called once"
    );
    assert_eq!(
        serving_vendor.bodies().len(),
        1,
        "the serving vendor is called once"
    );
    assert_eq!(result.output, "Paris");
    assert_eq!(
        (
            result.usage.prompt_tokens,
            result.usage.completion_tokens,
            result.usage.total_tokens
        ),
        (2_000, 2_040, 4_040),
        "the run's usage includes the refused hop's billed tokens"
    );
    assert_eq!(
        result.cost,
        Some(total.clone()),
        "and the run's cost its price"
    );
    let by_run = ledger_spend(&ledger, SpendGroupBy::Run).await;
    assert_eq!(
        by_run.values().copied().collect::<Vec<_>>(),
        vec![(total.nanos(), 1)],
        "one run, one row for the call (not one per hop), carrying both parts"
    );
    assert_eq!(
        ledger_spend(&ledger, SpendGroupBy::Model).await,
        std::collections::BTreeMap::from([
            (refusing.model.to_string(), (c1.nanos(), 1)),
            (serving.model.to_string(), (c2.nanos(), 1)),
        ]),
        "the row's breakdown names each model at its own price"
    );

    // ---- 2. a billed empty completion is settled although the run fails ----
    let kimi = provider("kimi");
    let table = ledger_price_table(&[(kimi.model, 600_000_000, 2_500_000_000)]);
    let (_vendor, port) = vendor_and_port(
        &kimi,
        200,
        openai_empty_length_body(kimi.model, 1_000, 62_000),
    )
    .await;
    let llm = with_pricing(port, &table);
    let billed = table
        .price(kimi.model, &TokenUsage::new(1_000, 62_000))
        .expect("the Kimi model is priced");

    let ledger = Arc::new(InMemoryTreasuryLedger::new());
    let paladin = agent_builder(Arc::clone(&llm), kimi.model)
        .build()
        .await
        .unwrap_or_else(|error| panic!("the Kimi agent must build: {error}"));
    let outcome = execution_service(llm)
        .with_treasury_ledger(ledger.clone(), AgentLoopSettlement::EveryCall)
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
                "a billed empty completion is permanent; message was: {message}"
            );
            assert!(
                message.contains("Empty completion"),
                "the failure is the typed empty completion; message was: {message}"
            );
        }
        Err(other) => panic!("expected a typed LlmFailure, got {other}"),
        Ok(result) => panic!(
            "an empty length completion was returned as an answer: {:?}",
            result.output
        ),
    }
    assert_eq!(
        ledger_spend(&ledger, SpendGroupBy::Model).await,
        std::collections::BTreeMap::from([(kimi.model.to_string(), (billed.nanos(), 1))]),
        "the failed call is settled once, at its own model's price"
    );
    assert_eq!(
        ledger_spend(&ledger, SpendGroupBy::Run).await.len(),
        1,
        "under the execution's own run"
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
matrix_case!(openai_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("openai"));
matrix_case!(anthropic_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("anthropic"));
matrix_case!(deepseek_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("deepseek"));
matrix_case!(kimi_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("kimi"));
matrix_case!(qwen_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("qwen"));
matrix_case!(grok_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("grok"));
matrix_case!(gemini_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("gemini"));
matrix_case!(ollama_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("ollama"));
matrix_case!(openai_compatible_same_agent_is_equivalent_or_refused_by_name => case_same_agent_is_equivalent_or_refused_by_name("openai-compatible"));

// (c) and (d) the fixed-temperature provider
matrix_case!(kimi_fixed_temperature_explicit => case_fixed_temperature_explicit("kimi"));
matrix_case!(kimi_fixed_temperature_auto => case_fixed_temperature_auto("kimi"));
matrix_case!(kimi_unseeded_model_refuses_an_explicit_temperature => case_unseeded_kimi_refuses_an_explicit_temperature("kimi"));

// (e) an empty stop completion is typed -- all nine
matrix_case!(openai_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("openai"));
matrix_case!(anthropic_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("anthropic"));
matrix_case!(deepseek_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("deepseek"));
matrix_case!(kimi_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("kimi"));
matrix_case!(qwen_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("qwen"));
matrix_case!(grok_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("grok"));
matrix_case!(gemini_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("gemini"));
matrix_case!(ollama_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("ollama"));
matrix_case!(openai_compatible_empty_stop_completion_is_typed => case_empty_stop_completion_is_typed("openai-compatible"));

// (f) a fallback hop is sent its own model
#[tokio::test]
async fn fallback_hop_gets_its_own_model() {
    case_fallback_hop_gets_its_own_model().await
}

// (f2) the same through `agent_runtime.model_fallback.steps`
#[tokio::test]
async fn config_fallback_steps_send_each_hop_its_own_model() {
    case_config_fallback_steps_send_each_hop_its_own_model().await
}

// (g) billed failures and hopped refusals reach the treasury ledger (G-50-5a)
#[tokio::test]
async fn billed_failures_and_hopped_refusals_reach_the_ledger() {
    case_billed_failures_and_hopped_refusals_reach_the_ledger().await
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
