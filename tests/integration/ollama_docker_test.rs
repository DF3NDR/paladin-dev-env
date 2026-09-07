//! Ollama Docker-gated Tier 2 integration suite (17-07, D-15).
//!
//! This is the only place in the workspace where the shared
//! [`paladin_llm::compat::CompatEngine`] meets a real implementation of the
//! OpenAI-compatible protocol without a credential -- every other new
//! provider adapter landed this phase (Kimi, Qwen, Grok, Gemini,
//! OpenAI-compatible) is proven only against `mockito` fixtures (Tier 1),
//! because they all require a vendor API key this workspace does not hold. A
//! divergence between the engine's assumptions and a real server's wire
//! behaviour therefore surfaces here, not in production against a hosted
//! vendor.
//!
//! This test target is `required-features` gated
//! (`["integration-tests", "llm-ollama"]`, see root `Cargo.toml`), so a
//! plain `cargo test` never attempts to reach a Docker service. Bring the
//! `ollama-test` service up before running it:
//!
//! ```sh
//! docker compose -f docker/docker-compose.test.yml up -d ollama-test ollama-test-init
//! cargo test --test ollama_docker --features integration-tests,llm-ollama -- --nocapture
//! ```
//!
//! Even with the required feature enabled, each test independently probes
//! `OLLAMA_TEST_URL` before doing anything else and prints a named reason
//! and returns early if the service is not reachable, rather than panicking
//! or hanging -- the suite is deliberately built so "ran without the
//! service" is a visible skip, never a silent pass. See
//! `.planning/phases/17-additional-llm-provider-adapters/17-07-SUMMARY.md`
//! for the record of what could and could not be verified against a real
//! Ollama instance in the sandbox this suite was authored in (no Docker
//! daemon available there) -- this suite's runtime behaviour against a real
//! server is authored but unverified, and is tracked as verification debt.

use futures::StreamExt;
use paladin::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_llm::ollama::{OllamaAdapter, OllamaConfig};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use std::env;
use std::time::Duration;

/// The model `ollama-test-init` pulls (see `docker/docker-compose.test.yml`
/// for the exact tag and the reasoning behind choosing it).
const OLLAMA_TEST_MODEL: &str = "qwen2.5:0.5b";

/// Default `ollama-test` URL, matching the `11435:11434` port mapping in
/// `docker/docker-compose.test.yml`. Overridable via `OLLAMA_TEST_URL` for a
/// CI runner that maps the service to a different host port.
fn ollama_test_url() -> String {
    env::var("OLLAMA_TEST_URL").unwrap_or_else(|_| "http://localhost:11435/v1".to_string())
}

/// Returns `true` (after printing a reason naming the unreachable URL) if
/// `ollama-test` is not reachable at `base_url`. This is the runtime gate
/// that lets every test in this `required-features`-gated suite skip
/// gracefully -- with a printed reason -- when the operator ran the suite
/// deliberately without first bringing the Docker service up, instead of
/// hanging on a real HTTP timeout or panicking with an opaque connection
/// error. It is also what proves, without a Docker daemon at all, that a
/// stopped/missing service produces a named skip rather than a silent pass
/// (Task 2 acceptance criterion, T-17-39).
async fn skip_if_unreachable(base_url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            eprintln!(
                "SKIP: could not build an HTTP client to probe ollama-test at {base_url} ({err})"
            );
            return true;
        }
    };

    match client.get(format!("{base_url}/models")).send().await {
        Ok(resp) if resp.status().is_success() => false,
        Ok(resp) => {
            eprintln!(
                "SKIP: ollama-test at {base_url} responded with status {} -- is the service \
                 healthy yet? Bring it up with `docker compose -f \
                 docker/docker-compose.test.yml up -d ollama-test ollama-test-init` and wait \
                 for the model pull to finish before running this suite.",
                resp.status()
            );
            true
        }
        Err(err) => {
            eprintln!(
                "SKIP: ollama-test unreachable at {base_url} ({err}). Bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d ollama-test \
                 ollama-test-init` before running this suite -- this is a \
                 required-features-gated Tier 2 test, run deliberately, not part of `cargo \
                 test`'s default set."
            );
            true
        }
    }
}

fn build_adapter() -> OllamaAdapter {
    let config = OllamaConfig::new(ollama_test_url(), OLLAMA_TEST_MODEL.to_string());
    OllamaAdapter::new(config)
        .expect("a base URL from ollama_test_url() and a fixed model name always validate")
}

fn build_request(query: &str) -> LlmRequest {
    let prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: query.to_string(),
        context: None,
    }))
    .expect("fixed, valid prompt construction cannot fail");

    LlmRequest::new(OLLAMA_TEST_MODEL, prompt)
}

/// A real non-streaming `generate()` round trip against the pulled model
/// returns non-empty text and a `TokenUsage` whose fields the real server
/// populated -- a mock fixture can assert this shape but cannot prove a
/// real server actually produces it.
#[tokio::test]
async fn generate_round_trip_returns_nonempty_content_and_real_token_usage() {
    let base_url = ollama_test_url();
    if skip_if_unreachable(&base_url).await {
        return;
    }

    let adapter = build_adapter();
    let response = adapter
        .generate(build_request("Say hello in exactly one word."))
        .await
        .expect("ollama-test was confirmed reachable above; generate() must succeed");

    assert!(
        !response.content.is_empty(),
        "a real Ollama completion must return non-empty text"
    );
    assert!(
        response.usage.prompt_tokens > 0,
        "the real server must populate prompt_tokens, got {}",
        response.usage.prompt_tokens
    );
    assert!(
        response.usage.completion_tokens > 0,
        "the real server must populate completion_tokens, got {}",
        response.usage.completion_tokens
    );
    assert_eq!(
        response.usage.total_tokens,
        response.usage.prompt_tokens + response.usage.completion_tokens,
        "total_tokens must equal the sum of prompt and completion tokens"
    );
}

/// A real `generate_stream()` produces more than one chunk and their
/// concatenation is non-empty -- proving the engine's SSE line-splitting
/// and `[DONE]` handling match a real emitter, not just a hand-written
/// fixture (see `crates/paladin-llm/src/ollama/adapter.rs`'s mockito-based
/// streaming test for the fixture-only counterpart of this assertion).
#[tokio::test]
async fn generate_stream_produces_multiple_chunks_with_nonempty_concatenation() {
    let base_url = ollama_test_url();
    if skip_if_unreachable(&base_url).await {
        return;
    }

    let adapter = build_adapter();
    let mut request = build_request("Count from one to five, one number per line.");
    request.stream = true;

    let stream = adapter
        .generate_stream(request)
        .await
        .expect("ollama-test was confirmed reachable above; generate_stream() must succeed");
    let mut stream = Box::into_pin(stream);

    let mut chunk_count = 0usize;
    let mut assembled = String::new();
    while let Some(item) = stream.next().await {
        let chunk = item.expect("a real Ollama stream must not error mid-stream");
        chunk_count += 1;
        assembled.push_str(&chunk.delta);
    }

    assert!(
        chunk_count > 1,
        "a real streaming completion must arrive as more than one SSE chunk, got {chunk_count}"
    );
    assert!(
        !assembled.is_empty(),
        "the concatenated stream content must be non-empty"
    );
}

/// `get_available_models()` against the real `/v1/models` returns the
/// pulled model, proving the live-fetch path (D-13) parses a real response
/// shape rather than the mock's.
#[tokio::test]
async fn get_available_models_returns_the_pulled_model() {
    let base_url = ollama_test_url();
    if skip_if_unreachable(&base_url).await {
        return;
    }

    let adapter = build_adapter();
    let models = adapter
        .get_available_models()
        .await
        .expect("ollama-test was confirmed reachable above; get_available_models() must succeed");

    assert!(
        models.iter().any(|m| m == OLLAMA_TEST_MODEL),
        "the live /v1/models catalog must include the model pulled by ollama-test-init \
         ({OLLAMA_TEST_MODEL}); got {models:?}"
    );
}

/// `validate_model()` returns `true` for the pulled model and `false` for a
/// model that was not pulled -- the operator-catalog behaviour that is the
/// whole reason D-13 exists for Ollama.
#[tokio::test]
async fn validate_model_distinguishes_pulled_from_unpulled() {
    let base_url = ollama_test_url();
    if skip_if_unreachable(&base_url).await {
        return;
    }

    let adapter = build_adapter();

    let pulled = adapter
        .validate_model(OLLAMA_TEST_MODEL)
        .await
        .expect("ollama-test was confirmed reachable above; validate_model() must succeed");
    assert!(
        pulled,
        "the model pulled by ollama-test-init ({OLLAMA_TEST_MODEL}) must validate true"
    );

    let not_pulled = adapter
        .validate_model("definitely-not-a-real-model-name:latest")
        .await
        .expect("validate_model() must succeed even for a model that was never pulled");
    assert!(
        !not_pulled,
        "a model never pulled onto ollama-test must validate false"
    );
}
