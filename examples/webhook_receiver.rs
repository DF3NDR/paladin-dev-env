//! Runnable example: a webhook RECEIVER -- the other side of Paladin's outbound webhook
//! delivery (Phase 27, plan 27-13) -- demonstrating correct signature verification and
//! the private-address SSRF override from the RECEIVING end.
//!
//! Hermetic -- no provider key, no external service:
//! `cargo run --example webhook_receiver --features "web-server"`.
//!
//! ## What this program demonstrates
//!
//! 1. Stands up a receiver handler on an ephemeral loopback port that captures the raw
//!    request body bytes BEFORE any deserialization, and recomputes the keyed hash over
//!    exactly those bytes -- using [`sign_webhook_body`], the SAME function and the SAME
//!    `hmac`/`sha2` crates the shipped [`WebhookDeliveryService`] signs with, never a
//!    hand-rolled construction (D-41: the signature is computed ONCE over the exact byte
//!    buffer stored on the delivery row and sent verbatim, so a receiver's own
//!    recomputation over its own raw capture always matches).
//! 2. Verifies a genuine delivery (EX-96): sends a delivery whose `X-Paladin-Signature:
//!    sha256=<hex>` header (the [`WEBHOOK_SIGNATURE_HEADER`] constant -- the same wire
//!    header name the shipped delivery service sends) carries the hex digest computed
//!    over the same byte buffer, and prints that the recomputed digest matched.
//! 3. Rejects a tampered body: sends the SAME signature with a mutated body and prints
//!    the rejection, using [`hmac::Mac::verify_slice`]'s constant-time comparison (never
//!    a plain `==` on the digest bytes) for the check.
//! 4. Names the private-address override (EX-97): prints the current
//!    `APP_WEBHOOKS_ALLOW_PRIVATE` value, one sentence on what it relaxes (loopback and
//!    private-range destinations, for local development) and one sentence on what it does
//!    NOT relax (the cloud metadata address is rejected regardless), plus the guard's own
//!    documented DNS-rebinding limitation -- so this example does not read as a stronger
//!    guarantee than the tree provides.
//!
//! ## Credential rules (D-29, security.instructions.md, binding)
//!
//! The shared signing secret is generated in-process from 32 cryptographically random
//! bytes, is NEVER printed, logged, or placed in a header comment or fenced command --
//! only whether verification succeeded or failed is printed, never the digest input or
//! the secret itself.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Router, body::Bytes};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

use paladin::application::services::run::webhook::{
    SsrfGuard, WEBHOOK_SIGNATURE_HEADER, sign_webhook_body,
};
use paladin::config::env_utils::EnvOverridable;
use paladin::config::webhooks::WebhooksConfig;

/// Shared receiver state: only the signing secret, held behind an `Arc` so the handler
/// never clones it out to anywhere a print statement could reach.
struct ReceiverState {
    secret: Vec<u8>,
}

/// The receiver handler: captures the raw body BEFORE deserialization (`Bytes`, not
/// `Json<...>`), recomputes the signature over exactly those bytes, and verifies in
/// constant time via [`hmac::Mac::verify_slice`].
async fn webhook_handler(
    State(state): State<Arc<ReceiverState>>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    let Some(header_value) = headers
        .get(WEBHOOK_SIGNATURE_HEADER)
        .and_then(|v| v.to_str().ok())
    else {
        println!("  receiver: rejected (missing {WEBHOOK_SIGNATURE_HEADER} header)");
        return (StatusCode::UNAUTHORIZED, "missing signature header");
    };

    let Some(hex_digest) = header_value.strip_prefix("sha256=") else {
        println!("  receiver: rejected (signature header not sha256=<hex>)");
        return (StatusCode::UNAUTHORIZED, "malformed signature header");
    };

    let Ok(received_digest) = hex_decode(hex_digest) else {
        println!("  receiver: rejected (signature header is not valid hex)");
        return (StatusCode::UNAUTHORIZED, "malformed signature header");
    };

    // Constant-time verification (`Mac::verify_slice`, never a plain `==` on the digest
    // bytes): recompute the HMAC over the EXACT raw bytes this handler captured, using
    // the SAME key and body order the shipped delivery service signs with.
    let verified = match <Hmac<Sha256> as Mac>::new_from_slice(&state.secret) {
        Ok(mut mac) => {
            mac.update(&body);
            mac.verify_slice(&received_digest).is_ok()
        }
        Err(_) => false,
    };

    if verified {
        println!("  receiver: signature verified (recomputed digest matched)");
        (StatusCode::OK, "valid")
    } else {
        println!("  receiver: rejected (recomputed digest did not match)");
        (StatusCode::UNAUTHORIZED, "invalid")
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if !s.len().is_multiple_of(2) {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A fresh, in-process, never-printed signing secret (32 cryptographically random
    // bytes) -- shared only between this program's own sender half and the receiver
    // handler's `Arc<ReceiverState>`, never logged or echoed (D-29).
    let mut secret = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);
    let state = Arc::new(ReceiverState {
        secret: secret.clone(),
    });

    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    let base = format!("http://{addr}");
    println!("webhook_receiver (example) listening on {base}/webhook");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // 1/2. A genuine delivery (EX-96): the signature is computed ONCE, here, over the
    // EXACT byte buffer this program is about to send -- exactly as `WebhookDeliveryService`
    // signs the exact buffer it stored on the delivery row, and sends it verbatim (D-41).
    // A receiver's own recomputation over its own raw capture always matches for exactly
    // this reason: neither side ever re-serializes the JSON in between.
    let genuine_body = br#"{"event":"run.completed","run_id":"01demo-run-id"}"#.to_vec();
    let genuine_signature = sign_webhook_body(&secret, &genuine_body);
    println!("sending a genuine delivery (signature computed once, over these exact bytes)...");
    let genuine_response = client
        .post(format!("{base}/webhook"))
        .header(WEBHOOK_SIGNATURE_HEADER, &genuine_signature)
        .body(genuine_body.clone())
        .send()
        .await?;
    println!(
        "genuine delivery -> {} {}",
        genuine_response.status(),
        genuine_response.text().await?
    );

    // 3. A tampered body under the SAME signature -- must be rejected.
    let mut tampered_body = genuine_body.clone();
    tampered_body.extend_from_slice(b" tampered");
    println!("sending the SAME signature over a tampered (mutated) body...");
    let tampered_response = client
        .post(format!("{base}/webhook"))
        .header(WEBHOOK_SIGNATURE_HEADER, &genuine_signature)
        .body(tampered_body)
        .send()
        .await?;
    println!(
        "tampered delivery -> {} {}",
        tampered_response.status(),
        tampered_response.text().await?
    );

    // 4. The private-address override (EX-97): print its current value and what it does
    // and does not relax, plus the guard's own documented DNS-rebinding limitation.
    let mut webhooks_config = WebhooksConfig::default();
    webhooks_config.apply_env_overrides();
    println!(
        "APP_WEBHOOKS_ALLOW_PRIVATE = {} (unset defaults to false)",
        webhooks_config.allow_private
    );
    println!(
        "  relaxes: loopback and private-range (RFC1918/link-local/unique-local) delivery \
         destinations, for local development and internal-network testing only."
    );
    println!(
        "  does NOT relax: the cloud metadata address 169.254.169.254 is rejected \
         regardless of this override -- it is never a legitimate webhook receiver."
    );
    println!(
        "  known, documented limitation (not an oversight): neither the write-time nor the \
         send-time SSRF check pins the resolved address between the check and the actual \
         connection, so DNS rebinding (a hostname that answers a public address at check \
         time and a private/metadata address at connect time) is not caught by this guard \
         alone."
    );

    // A concrete instance of the override in action: this receiver's own address is
    // loopback, so the SAME guard the write-time and send-time checks use accepts it only
    // when the override is set -- demonstrated against the receiver's own address, never
    // an address chosen by user input.
    let guard = SsrfGuard::new(webhooks_config.allow_private);
    let receiver_url = format!("{base}/webhook");
    match guard.check_url(&receiver_url).await {
        Ok(_) => {
            println!("  SsrfGuard::check_url({receiver_url}) -> accepted (allow_private is set)")
        }
        Err(rejection) => println!(
            "  SsrfGuard::check_url({receiver_url}) -> rejected ({rejection}) -- this \
             program's own loopback receiver is exactly the kind of destination the guard \
             rejects by default; set APP_WEBHOOKS_ALLOW_PRIVATE=true to allow it"
        ),
    }
    // The metadata address is rejected unconditionally -- shown without ever contacting it.
    let metadata_rejection = guard.check_addrs(&[IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))]);
    println!(
        "  SsrfGuard::check_addrs([169.254.169.254]) -> {} (checked regardless of \
         allow_private, never dispatched over the network)",
        match metadata_rejection {
            Ok(()) => "accepted".to_string(),
            Err(rejection) => format!("rejected ({rejection})"),
        }
    );

    let _ = shutdown_tx.send(());
    let _ = server.await;
    Ok(())
}
