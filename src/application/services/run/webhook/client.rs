//! Webhook HTTP client construction (D-42): the house no-redirect pattern
//! (`crates/paladin-llm/src/openai/adapter.rs`) applied to outbound webhook
//! delivery -- a `3xx` from wherever the URL resolves to can never carry
//! the `X-Paladin-Signature` credential-shaped header to a different,
//! attacker-influenced host, because the client never follows it.

use std::time::Duration;

/// The cap, in bytes, on how much of a webhook receiver's response body is
/// ever read into this process's memory for one delivery attempt (CR-01,
/// `27-REVIEW.md`).
///
/// The receiver's response body is attacker-influenced: the target URL is
/// caller-chosen (only its address class is constrained, by the SSRF guard
/// in `ssrf.rs`), a `5xx` response is retried up to `max_attempts`, and the
/// caller can trigger unlimited deliveries simply by submitting more runs.
/// Without a bound applied DURING the read, an authenticated `User`-role
/// principal could point a webhook at a host they control, answer every
/// attempt with a `5xx` and a multi-gigabyte body, and repeat it
/// indefinitely -- a repeatable memory-exhaustion vector. The bound is
/// therefore enforced as bytes are drained off the wire, never after the
/// whole body has already been buffered.
///
/// `Content-Length` is deliberately NOT trusted as the bound: it is a
/// header the receiver controls and can simply lie about (or omit, for a
/// chunked response), so relying on it to decide how much to read would
/// let the same attacker bypass the cap entirely.
pub(crate) const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

/// Drain `response`'s body one chunk at a time, stopping the moment the
/// accumulated length reaches `cap` bytes -- the bytes beyond `cap` are
/// never buffered, regardless of what the response claims via
/// `Content-Length` or how large the receiver actually makes the body
/// (CR-01, see [`MAX_ERROR_BODY_BYTES`]'s docs for the threat this closes).
///
/// The last chunk read is truncated to only the prefix that still fits
/// under `cap`, so the accumulated buffer never exceeds `cap` bytes even
/// when a single chunk would otherwise overshoot it. The result is decoded
/// with [`String::from_utf8_lossy`], so invalid UTF-8 -- including a
/// multi-byte character split in half by the cap boundary -- yields a lossy
/// replacement rather than a panic or an error.
///
/// A transport error partway through the read (a dropped connection, a
/// timeout mid-body) yields whatever was accumulated so far rather than
/// discarding it: a partial diagnostic is still useful, and this function
/// must never panic or propagate the error itself.
pub(crate) async fn read_bounded_body(mut response: reqwest::Response, cap: usize) -> String {
    let mut buffer: Vec<u8> = Vec::with_capacity(cap.min(8 * 1024));

    loop {
        if buffer.len() >= cap {
            break;
        }
        match response.chunk().await {
            Ok(Some(chunk)) => {
                let remaining = cap - buffer.len();
                if chunk.len() <= remaining {
                    buffer.extend_from_slice(&chunk);
                } else {
                    buffer.extend_from_slice(&chunk[..remaining]);
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    String::from_utf8_lossy(&buffer).into_owned()
}

/// Build the `reqwest::Client` every webhook delivery attempt is sent
/// through: `timeout`-bounded, no redirects, no pooled idle connections.
///
/// `pool_max_idle_per_host(0)`: each delivery targets a caller-chosen,
/// typically low-frequency host, and successive attempts to the SAME URL
/// can be minutes to an hour apart (D-43's own backoff schedule) -- keeping
/// an idle connection open that long buys little and, worse, a pooled
/// connection spanning that gap is exactly the shape a paused-then-advanced
/// test clock (`webhook_retry_schedule`) can observe as unexpectedly stale.
/// A fresh connection per attempt is simpler and avoids that whole class of
/// staleness bug in both production and tests.
pub fn build_webhook_client(timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("paladin-webhooks/0.10")
        .pool_max_idle_per_host(0)
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

    /// A body an order of magnitude larger than a small test cap is stopped
    /// at exactly the cap -- the remainder is never buffered (CR-01).
    #[tokio::test]
    async fn bounded_body_stops_at_the_cap() {
        let mut server = mockito::Server::new_async().await;
        let cap = 1024usize;
        let big_body = "a".repeat(cap * 10);
        server
            .mock("POST", "/big")
            .with_status(500)
            .with_body(&big_body)
            .create_async()
            .await;

        let client = build_webhook_client(Duration::from_secs(5)).unwrap();
        let response = client
            .post(format!("{}/big", server.url()))
            .send()
            .await
            .unwrap();

        let bounded = read_bounded_body(response, cap).await;
        assert_eq!(bounded.len(), cap);
    }

    /// A body smaller than the cap is returned whole and unchanged.
    #[tokio::test]
    async fn bounded_body_returns_a_small_body_whole() {
        let mut server = mockito::Server::new_async().await;
        let cap = 1024usize;
        let small_body = "small error body";
        server
            .mock("POST", "/small")
            .with_status(500)
            .with_body(small_body)
            .create_async()
            .await;

        let client = build_webhook_client(Duration::from_secs(5)).unwrap();
        let response = client
            .post(format!("{}/small", server.url()))
            .send()
            .await
            .unwrap();

        let bounded = read_bounded_body(response, cap).await;
        assert_eq!(bounded, small_body);
    }

    /// A body whose byte length is EXACTLY the cap is returned whole, with
    /// no off-by-one truncation and no error.
    #[tokio::test]
    async fn bounded_body_at_exactly_the_cap_is_not_truncated() {
        let mut server = mockito::Server::new_async().await;
        let cap = 256usize;
        let exact_body = "b".repeat(cap);
        server
            .mock("POST", "/exact")
            .with_status(500)
            .with_body(&exact_body)
            .create_async()
            .await;

        let client = build_webhook_client(Duration::from_secs(5)).unwrap();
        let response = client
            .post(format!("{}/exact", server.url()))
            .send()
            .await
            .unwrap();

        let bounded = read_bounded_body(response, cap).await;
        assert_eq!(bounded, exact_body);
        assert_eq!(bounded.len(), cap);
    }
}
