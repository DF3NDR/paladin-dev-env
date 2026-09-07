//! Provider-switching integration test (Epic 6 task 7.10).
//!
//! Proves that two different `LlmPort` implementations can be selected at
//! runtime behind the same `Arc<dyn LlmPort>` binding, that switching between
//! them does not change the request/response contract the caller sees, and
//! that requesting an unknown provider name returns a typed error rather
//! than panicking or silently falling back to a default.
//!
//! Runs entirely offline. Provider A is `paladin_llm::mock::MockLlmAdapter`
//! (in-process, no network at all). Provider B is a real `DeepSeekAdapter`
//! pointed at a local `mockito` server, so the "second provider" genuinely
//! exercises a distinct request-building/response-parsing implementation
//! without a live API key or network egress. No provider feature flag is
//! required: the root `Cargo.toml`'s dependency declaration always builds
//! `paladin-llm` with `openai`, `anthropic`, `deepseek` and `mock` enabled
//! (see `crates/paladin-llm/src/{deepseek,mock}.rs`'s always-on
//! availability, used the same way by `tests/unit/llm/`), so this test runs
//! under the default feature set with no `#[cfg(feature = ...)]` guard.

use mockito::Server;
use paladin::core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_llm::deepseek::{DeepSeekAdapter, DeepSeekConfig};
use paladin_llm::mock::MockLlmAdapter;
use paladin_llm::provider_factory::{LlmProviderFactory, ProviderFactoryError};
use paladin_ports::output::llm_port::{LlmPort, LlmRequest};
use std::sync::Arc;

/// Build a fresh request each call so the same logical request can be sent
/// through more than one provider without fighting move semantics.
fn make_request(content: &str) -> LlmRequest {
    let prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: content.to_string(),
        context: None,
    }))
    .expect("fixed, valid prompt construction cannot fail");

    LlmRequest::new("test-model", prompt)
}

#[tokio::test]
async fn test_provider_switch_preserves_request_contract() {
    // Provider A: the in-crate mock adapter -- zero network, deterministic
    // response content.
    let provider_a: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_response("mock provider response"));

    // Provider B: a real DeepSeekAdapter pointed at a local mockito server --
    // a genuinely different LlmPort implementation (its own request/response
    // wire format), but still fully offline and keyless.
    let mut server = Server::new_async().await;
    let mock_body = serde_json::json!({
        "id": "chatcmpl-switch-test",
        "object": "chat.completion",
        "created": 1_700_000_000,
        "model": "deepseek-chat",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "deepseek provider response" },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8 }
    })
    .to_string();
    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_body)
        .create_async()
        .await;

    let deepseek_config = DeepSeekConfig {
        api_key: "test-key".to_string(),
        base_url: server.url(),
        model: "deepseek-chat".to_string(),
        timeout_seconds: 5,
    };
    let provider_b: Arc<dyn LlmPort> =
        Arc::new(DeepSeekAdapter::new(deepseek_config).expect("valid DeepSeek config"));

    // The same caller-constructed request travels through each provider in
    // turn behind the identical `Arc<dyn LlmPort>` binding type -- the
    // "switch" is choosing which adapter serves the request; nothing about
    // the request the caller supplies or the response fields it reads
    // changes shape between them.
    for provider in [&provider_a, &provider_b] {
        let response = provider
            .generate(make_request("Hello"))
            .await
            .expect("every switched-to provider must serve the same request successfully");

        assert!(
            !response.content.is_empty(),
            "switched-to provider must return a well-formed, non-empty response"
        );
        assert!(
            response.usage.total_tokens > 0,
            "switched-to provider must report non-zero token usage"
        );
    }

    // Prove the switch actually took effect: each provider returns its own
    // content, not the same adapter answering twice.
    let response_a = provider_a.generate(make_request("Hello")).await.unwrap();
    let response_b = provider_b.generate(make_request("Hello")).await.unwrap();
    assert_ne!(
        response_a.content, response_b.content,
        "the two providers must be genuinely distinct, not the same adapter resolved twice"
    );

    // The capability surface differs too -- this is what proves the switch
    // is meaningful rather than cosmetic. temperature_range is the natural
    // discriminator now that ADR-0004/D-15 have landed (plan 02-02):
    // DeepSeek declares its real 0.0-2.0 range; MockLlmAdapter declares none.
    let caps_a = provider_a.get_capabilities();
    let caps_b = provider_b.get_capabilities();
    assert_ne!(
        caps_a.temperature_range, caps_b.temperature_range,
        "capabilities must differ in at least one field, proving the switch took effect \
         rather than resolving to the same adapter twice"
    );
    assert_eq!(caps_a.temperature_range, None);
    assert_eq!(caps_b.temperature_range, Some((0.0, 2.0)));
}

#[tokio::test]
async fn test_provider_switch_rejects_unknown_provider() {
    // The unknown-provider path needs no configuration and reads no
    // environment variable at all -- it fails before any provider-specific
    // code runs.
    let factory = LlmProviderFactory::new();
    let result = factory.create("not-a-real-provider");

    match result {
        Err(ProviderFactoryError::UnknownProvider(name)) => {
            assert_eq!(name, "not-a-real-provider");
        }
        Err(other) => panic!("expected UnknownProvider, got a different typed error: {other}"),
        Ok(_) => panic!("expected UnknownProvider, factory silently returned an adapter"),
    }
}
