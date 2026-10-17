//! CLI Integration Tests - Tier 2: Docker-Gated Service Tests
//!
//! These tests require Docker services running (Redis, RustFS).
//! Run with: `cargo test --test lib integration::cli_real_services_test -- --ignored`
//! Start services with: `make services-up`
//!
//! All tests are marked `#[ignore]` to prevent running in standard CI.

use std::env;
use std::net::TcpStream;
use std::time::Duration;

/// Check if a service is available on the given host:port
fn service_available(host: &str, port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("{}:{}", host, port).parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Get Redis URL from environment or default
fn redis_url() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string())
}

/// Get the object-store (RustFS) endpoint from environment or default
fn minio_endpoint() -> String {
    env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9010".to_string())
}

// =============================================================================
// 5.2.1: Test gating - skip if Docker services not available
// =============================================================================

#[test]
#[ignore = "Requires Docker services: run 'make services-up'"]
fn test_redis_service_reachable() {
    // Extract host/port from REDIS_URL
    let url = redis_url();
    let port: u16 = url
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6380);

    assert!(
        service_available("localhost", port),
        "Redis should be reachable on port {}. Run 'make services-up' to start Docker services.",
        port
    );
}

// =============================================================================
// 5.2.2: Test setup-check with real Redis connection
// =============================================================================

#[tokio::test]
#[ignore = "Requires Docker services: run 'make services-up'"]
async fn test_setup_check_redis_connection() {
    let url = redis_url();
    let port: u16 = url
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6380);

    if !service_available("localhost", port) {
        println!("SKIPPED: Redis not available on port {}", port);
        return;
    }

    // Verify Redis accepts connections
    let client = redis::Client::open(url.as_str());
    assert!(
        client.is_ok(),
        "Redis client should be created successfully"
    );

    let client = client.unwrap();
    let conn: Result<redis::aio::MultiplexedConnection, redis::RedisError> =
        client.get_multiplexed_async_connection().await;
    assert!(
        conn.is_ok(),
        "Redis connection should be established. Error: {:?}",
        conn.err()
    );
}

// =============================================================================
// 5.2.4: Test setup-check with real object-store (RustFS) connection
// =============================================================================

#[tokio::test]
#[ignore = "Requires Docker services: run 'make services-up'"]
async fn test_setup_check_minio_connection() {
    let endpoint = minio_endpoint();

    // Check the RustFS health endpoint
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    let health_url = format!("{}/health/ready", endpoint);
    let result = client.get(&health_url).send().await;

    assert!(
        result.is_ok(),
        "RustFS health check should succeed. Is RustFS running at {}?",
        endpoint
    );

    let response = result.unwrap();
    assert!(
        response.status().is_success(),
        "RustFS health check should return success status"
    );
}

// =============================================================================
// 5.2.5: Test service health validation
// =============================================================================

#[test]
#[ignore = "Requires Docker services: run 'make services-up'"]
fn test_all_services_health_check() {
    let redis_port: u16 = redis_url()
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6380);

    let minio_port: u16 = 9010;

    // Report status of all services
    let redis_ok = service_available("localhost", redis_port);
    let minio_ok = service_available("localhost", minio_port);

    println!("Service Health Check:");
    println!(
        "  Redis (port {}): {}",
        redis_port,
        if redis_ok { "✅ UP" } else { "❌ DOWN" }
    );
    println!(
        "  RustFS (port {}): {}",
        minio_port,
        if minio_ok { "✅ UP" } else { "❌ DOWN" }
    );

    assert!(
        redis_ok && minio_ok,
        "All Docker services should be running. Run 'make services-up' to start them."
    );
}

// =============================================================================
// 5.2.6: Test connection error handling when service unavailable
// =============================================================================

#[tokio::test]
#[ignore = "Requires Docker services: run 'make services-up'"]
async fn test_connection_error_on_wrong_port() {
    // Try connecting to a port that definitely doesn't have Redis
    let bad_url = "redis://localhost:59999";
    let client = redis::Client::open(bad_url);
    assert!(
        client.is_ok(),
        "Client creation should succeed even with bad URL"
    );

    let client = client.unwrap();
    let conn: Result<redis::aio::MultiplexedConnection, redis::RedisError> =
        client.get_multiplexed_async_connection().await;
    assert!(
        conn.is_err(),
        "Connection to non-existent service should fail gracefully"
    );
}

#[test]
#[ignore = "Requires Docker services: run 'make services-up'"]
fn test_service_unavailable_detection() {
    // Port 59998 should not have any service
    let available = service_available("localhost", 59998);
    assert!(!available, "Port 59998 should not have any service running");
}
