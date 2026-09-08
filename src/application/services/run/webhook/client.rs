//! Webhook HTTP client construction (D-42): the house no-redirect pattern
//! (`crates/paladin-llm/src/openai/adapter.rs`) applied to outbound webhook
//! delivery -- a `3xx` from wherever the URL resolves to can never carry
//! the `X-Paladin-Signature` credential-shaped header to a different,
//! attacker-influenced host, because the client never follows it.

use std::time::Duration;

/// Build the `reqwest::Client` every webhook delivery attempt is sent
/// through: `timeout`-bounded, no redirects.
pub fn build_webhook_client(timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("paladin-webhooks/0.10")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_webhook_client_succeeds() {
        let client = build_webhook_client(Duration::from_secs(5));
        assert!(client.is_ok());
    }

    /// A mockito 302 with a `Location` header -- the no-redirect client
    /// returns the 302 response ITSELF; the redirect target mock records
    /// zero hits.
    #[tokio::test]
    async fn webhook_client_no_redirects() {
        let mut server = mockito::Server::new_async().await;
        let target = server
            .mock("GET", "/target")
            .with_status(200)
            .create_async()
            .await;
        let redirecting = server
            .mock("GET", "/redirecting")
            .with_status(302)
            .with_header("Location", &format!("{}/target", server.url()))
            .create_async()
            .await;

        let client = build_webhook_client(Duration::from_secs(5)).unwrap();
        let response = client
            .get(format!("{}/redirecting", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), 302);
        redirecting.assert_async().await;
        target.expect(0).assert_async().await;
    }
}
