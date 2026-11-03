//! The one provider-key probe module for `setup-check` and `onboarding`.
//!
//! DF-19: both commands used to carry their own copy of each key validator. The Anthropic copy
//! posted a retired, hard-coded claude-3 model id, Anthropic answered 404, and a valid key was
//! reported as broken in two places. The copies had also drifted (setup-check dropped the
//! provider's reply, onboarding echoed it raw), so a fix in one file would not have reached the
//! other. All three probes live here so they cannot drift apart again.

use crate::application::cli::error::{CliError, CliResult};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Anthropic Messages endpoint used by the key probe.
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
/// Stand-in for the Anthropic adapter's seeded default in builds without `llm-anthropic`.
///
/// `anthropic_probe_model_is_the_adapters_seeded_default` pins it equal to
/// `ANTHROPIC_DEFAULT_MODEL`, so the two cannot drift silently.
#[cfg_attr(feature = "llm-anthropic", allow(dead_code))]
const ANTHROPIC_PROBE_MODEL_MIRROR: &str = "claude-haiku-5-5";

/// The model the Anthropic key probe posts: the Anthropic adapter's seeded default.
///
/// paladin-llm's `anthropic_default_model_is_seeded` pins `ANTHROPIC_DEFAULT_MODEL` to a seeded
/// registry row, so a default bump moves this probe with it. Builds without `llm-anthropic` have
/// no adapter constant and use [`ANTHROPIC_PROBE_MODEL_MIRROR`], which
/// `anthropic_probe_model_is_the_adapters_seeded_default` ties to the constant.
fn anthropic_probe_model() -> &'static str {
    #[cfg(feature = "llm-anthropic")]
    {
        paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL
    }
    #[cfg(not(feature = "llm-anthropic"))]
    {
        ANTHROPIC_PROBE_MODEL_MIRROR
    }
}

/// The one HTTP client every key probe uses.
///
/// 10 s timeout and `redirect::Policy::none()`: reqwest strips `Authorization` on a cross-host
/// redirect but not the custom `x-api-key` header, so a probe must never follow a 3xx. A 3xx is
/// reported as a failure instead.
fn probe_client() -> CliResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| CliError::execution(format!("Failed to create HTTP client: {}", e)))
}

/// Build the failure for a non-2xx probe reply.
///
/// The reply is always passed through `diagnostic_excerpt`, which redacts the key and credential
/// patterns and only then bounds the length (bounding first could slice a secret across the
/// boundary). `model` is `Some` for probes that send one, so the message says what was probed.
fn probe_failure(
    provider: &str,
    model: Option<&str>,
    status: reqwest::StatusCode,
    body: &str,
    api_key: &str,
) -> CliError {
    let excerpt = paladin_llm::redaction::diagnostic_excerpt(body, api_key);
    let mut message = match model {
        Some(model) => format!(
            "{} API validation failed for model '{}' ({})",
            provider, model, status
        ),
        None => format!("{} API validation failed ({})", provider, status),
    };
    if !excerpt.is_empty() {
        message.push_str(": ");
        message.push_str(&excerpt);
    }
    CliError::execution(message)
}

/// Validate an Anthropic key against `messages_url` by posting a minimal request for `model`.
async fn validate_anthropic_key_at(
    messages_url: &str,
    api_key: &str,
    model: &str,
) -> CliResult<()> {
    let client = probe_client()?;

    let body = json!({
        "model": model,
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "test"}]
    });

    let response = client
        .post(messages_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| CliError::execution(format!("API request failed: {}", e)))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let reply = response.text().await.unwrap_or_default();
        Err(probe_failure(
            "Anthropic",
            Some(model),
            status,
            &reply,
            api_key,
        ))
    }
}

/// Validate an Anthropic API key with the adapter's seeded default model.
pub(super) async fn validate_anthropic_key(api_key: &str) -> CliResult<()> {
    validate_anthropic_key_at(ANTHROPIC_MESSAGES_URL, api_key, anthropic_probe_model()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    const ANTHROPIC_SENTINEL: &str = "test-key-not-a-secret-df19-anthropic";

    /// The model the request body must carry: the adapter's seeded default.
    fn expected_model() -> &'static str {
        #[cfg(feature = "llm-anthropic")]
        {
            paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL
        }
        #[cfg(not(feature = "llm-anthropic"))]
        {
            ANTHROPIC_PROBE_MODEL_MIRROR
        }
    }

    #[cfg(feature = "llm-anthropic")]
    #[test]
    fn anthropic_probe_model_is_the_adapters_seeded_default() {
        assert_eq!(
            anthropic_probe_model(),
            paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL
        );
        assert_eq!(
            ANTHROPIC_PROBE_MODEL_MIRROR,
            paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL
        );
    }

    #[tokio::test]
    async fn anthropic_probe_posts_the_probe_model_and_passes_on_200() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/messages")
            .match_header("x-api-key", ANTHROPIC_SENTINEL)
            .match_body(Matcher::PartialJson(json!({"model": expected_model()})))
            .with_status(200)
            .with_body(r#"{"type":"message"}"#)
            .expect(1)
            .create_async()
            .await;

        let url = format!("{}/v1/messages", server.url());
        let result =
            validate_anthropic_key_at(&url, ANTHROPIC_SENTINEL, anthropic_probe_model()).await;

        assert!(result.is_ok(), "expected Ok, got {result:?}");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn anthropic_probe_failure_names_the_model_and_the_provider_reply() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("POST", "/v1/messages")
            .with_status(404)
            .with_body(
                r#"{"type":"error","error":{"type":"not_found_error","message":"model: x"}}"#,
            )
            .create_async()
            .await;

        let url = format!("{}/v1/messages", server.url());
        let err = validate_anthropic_key_at(&url, ANTHROPIC_SENTINEL, anthropic_probe_model())
            .await
            .expect_err("a 404 must fail");
        let text = err.to_string();

        assert!(
            text.contains(&format!("for model '{}'", anthropic_probe_model())),
            "{text}"
        );
        assert!(text.contains("404"), "{text}");
        assert!(text.contains("not_found_error"), "{text}");
    }

    #[tokio::test]
    async fn anthropic_probe_failure_redacts_the_key_from_the_reply() {
        let mut server = mockito::Server::new_async().await;
        let body = format!(
            r#"{{"type":"error","error":{{"type":"authentication_error","message":"invalid x-api-key {ANTHROPIC_SENTINEL}"}}}}"#
        );
        let _mock = server
            .mock("POST", "/v1/messages")
            .with_status(401)
            .with_body(body)
            .create_async()
            .await;

        let url = format!("{}/v1/messages", server.url());
        let err = validate_anthropic_key_at(&url, ANTHROPIC_SENTINEL, anthropic_probe_model())
            .await
            .expect_err("a 401 must fail");
        let text = err.to_string();

        assert!(!text.contains(ANTHROPIC_SENTINEL), "key leaked: {text}");
        assert!(text.contains("[REDACTED]"), "{text}");
    }

    #[tokio::test]
    async fn anthropic_probe_does_not_follow_a_redirect() {
        let mut target = mockito::Server::new_async().await;
        let target_mock = target
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_body(r#"{"type":"message"}"#)
            .expect(0)
            .create_async()
            .await;

        let mut origin = mockito::Server::new_async().await;
        let _redirect = origin
            .mock("POST", "/v1/messages")
            .with_status(307)
            .with_header("location", &format!("{}/v1/messages", target.url()))
            .create_async()
            .await;

        let url = format!("{}/v1/messages", origin.url());
        let err = validate_anthropic_key_at(&url, ANTHROPIC_SENTINEL, anthropic_probe_model())
            .await
            .expect_err("a 3xx must be reported as a failure");

        assert!(err.to_string().contains("307"), "{err}");
        target_mock.assert_async().await;
    }
}
