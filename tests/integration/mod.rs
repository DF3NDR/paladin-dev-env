// tests/integration/mod.rs - Common integration test framework
//
// ## The live-API test suite's double gate
//
// `llm_live_api_tests` (below) is the actual skip mechanism for Epic 23 FR-23.4.4 and Epic 24
// US-24.7's "skip gracefully when API keys are missing" requirement — not `require_api_key`'s
// panic inside that module. Two gates apply together:
//
// 1. A `cfg(feature = "live-api-tests")` gate attribute on the `llm_live_api_tests` module
//    below excludes it from compilation entirely unless the `live-api-tests` feature is
//    explicitly enabled.
// 2. Every one of that module's 13 tests additionally carries `#[ignore]`.
//
// Consequence: a default `cargo test --workspace` run — no feature flag, no `--ignored` — never
// compiles `llm_live_api_tests` at all, so it skips the whole live-API suite cleanly without
// ever reaching `require_api_key`. And because the module is absent from the compiled test
// binary in that default configuration, the panic inside `require_api_key` cannot abort or
// corrupt an unrelated test running concurrently in that default parallel run.
//
// The suite runs only when a developer explicitly opts in with
// `cargo test --features live-api-tests -- --ignored`. See the recorded decision at
// `.planning/decisions/0012-live-api-test-key-behaviour.md`.

use std::env;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "llm-anthropic")]
pub mod anthropic_provider_test;
pub mod arsenal_bridge_regression_test;
pub mod arsenal_execution_integration_test;
pub mod arsenal_registry_integration_test;
pub mod autonomous_planning_test;
pub mod battalion;
pub mod battalion_campaign_integration_test;
pub mod battalion_chain_of_command_herald_test;
pub mod battalion_chain_of_command_integration_test;
pub mod battalion_herald_end_to_end_test;
pub mod citadel_integration_test;
#[cfg(feature = "cli")]
pub mod cli_integration_test;
#[cfg(feature = "cli")]
pub mod cli_real_providers_test;
#[cfg(feature = "cli")]
pub mod cli_real_services_test;
pub mod commander_error_paths_test;
pub mod commander_integration_tests;
pub mod context_injection_test;
#[cfg(feature = "llm-deepseek")]
pub mod deepseek_provider_test;
#[cfg(feature = "s3-storage")]
pub mod file_storage_integration_tests;
pub mod herald_integration_test;
pub mod in_memory_sanctum_tests;
#[cfg(feature = "live-api-tests")]
pub mod llm_live_api_tests;
pub mod mcp_stdio_test;
pub mod mcp_streamable_http_live_test;
pub mod mcp_streamable_http_test;
pub mod middleware_under_engine_test;
pub mod notification_system_integration_test;
#[cfg(feature = "llm-openai")]
pub mod openai_content_analysis_integration_test;
#[cfg(feature = "openai-embeddings")]
pub mod openai_embedding_tests;
#[cfg(feature = "llm-openai")]
pub mod openai_provider_test;
pub mod orchestrator_workflow_lifecycle_test;
pub mod paladin_garrison_integration_test;
pub mod paladin_integration_test;
pub mod provider_switching_test;
#[cfg(feature = "qdrant")]
pub mod qdrant_sanctum_tests;
#[cfg(feature = "qdrant")]
pub mod rag_integration_tests;
pub mod reasoning_agent_test;
#[cfg(feature = "redis-queue")]
pub mod redis_queue_integration_test;
pub mod scheduler_integration_test;
pub mod sqlite_garrison_integration_test;
pub mod structured_engine_node_test;
pub mod system_log_integration_test;
pub mod vault_confinement_test;
#[cfg(all(feature = "vision", feature = "llm-openai", feature = "llm-anthropic"))]
pub mod vision_integration_test;

static INIT: Once = Once::new();

/// Test environment configuration
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    pub redis_host: String,
    pub redis_port: u16,
    pub minio_endpoint: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub use_external_services: bool,
}

impl TestEnvironment {
    /// Create test environment based on detection of external services
    pub async fn new() -> Self {
        INIT.call_once(|| {
            env_logger::init();
        });

        // Check if we should use external services (CI/CD or explicit override)
        let use_external = env::var("USE_EXTERNAL_TEST_SERVICES")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or_else(|_| Self::detect_external_services());

        if use_external {
            Self::external_services().await
        } else {
            Self::local_containers().await
        }
    }

    /// Detect if external services are available (CI/CD environment)
    fn detect_external_services() -> bool {
        // Check for CI environment variables
        env::var("CI").is_ok()
            || env::var("GITHUB_ACTIONS").is_ok()
            || env::var("GITLAB_CI").is_ok()
            || env::var("JENKINS_URL").is_ok()
            // Check if docker-compose test services are running
            || Self::check_service_availability("localhost", 6380) // Redis test port
            && Self::check_service_availability("localhost", 9010) // MinIO test port
    }

    /// Check if a service is available on host:port
    pub fn check_service_availability(host: &str, port: u16) -> bool {
        std::net::TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().unwrap(),
            Duration::from_secs(1),
        )
        .is_ok()
    }

    /// Configuration for external services (CI/CD)
    async fn external_services() -> Self {
        println!("🔧 Using external test services (CI/CD mode)");

        let config = Self {
            redis_host: env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
            redis_port: env::var("TEST_REDIS_PORT")
                .unwrap_or_else(|_| "6380".to_string())
                .parse()
                .unwrap_or(6380),
            minio_endpoint: env::var("TEST_MINIO_ENDPOINT")
                .unwrap_or_else(|_| "localhost:9010".to_string()),
            minio_access_key: env::var("TEST_MINIO_ACCESS_KEY")
                .unwrap_or_else(|_| "testuser".to_string()),
            minio_secret_key: env::var("TEST_MINIO_SECRET_KEY")
                .unwrap_or_else(|_| "testpass123".to_string()),
            use_external_services: true,
        };

        // Wait for services to be ready
        Self::wait_for_services(&config).await;
        config
    }

    /// Configuration for local containers (development mode)
    async fn local_containers() -> Self {
        println!("🐳 Using local test containers (development mode)");

        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379, // Will be overridden by testcontainers
            minio_endpoint: "localhost:9000".to_string(), // Will be overridden
            minio_access_key: "minioadmin".to_string(),
            minio_secret_key: "minioadmin".to_string(),
            use_external_services: false,
        }
    }

    /// Wait for external services to be ready
    async fn wait_for_services(config: &TestEnvironment) {
        println!("⏳ Waiting for external services to be ready...");

        // Wait for Redis
        for attempt in 1..=30 {
            if Self::check_service_availability(&config.redis_host, config.redis_port) {
                println!(
                    "✅ Redis is ready at {}:{}",
                    config.redis_host, config.redis_port
                );
                break;
            }
            if attempt == 30 {
                panic!("❌ Redis not available after 30 attempts");
            }
            sleep(Duration::from_secs(1)).await;
        }

        // Wait for MinIO
        let (minio_host, minio_port) = Self::parse_endpoint(&config.minio_endpoint);
        for attempt in 1..=30 {
            if Self::check_service_availability(&minio_host, minio_port) {
                println!("✅ MinIO is ready at {}", config.minio_endpoint);
                break;
            }
            if attempt == 30 {
                panic!("❌ MinIO not available after 30 attempts");
            }
            sleep(Duration::from_secs(1)).await;
        }

        // Additional wait for service initialization
        sleep(Duration::from_secs(2)).await;
    }

    pub fn parse_endpoint(endpoint: &str) -> (String, u16) {
        if let Some((host, port_str)) = endpoint.split_once(':') {
            (host.to_string(), port_str.parse().unwrap_or(9000))
        } else {
            (endpoint.to_string(), 9000)
        }
    }

    /// Get bucket name for tests
    pub fn test_bucket(&self) -> String {
        if self.use_external_services {
            "integration-tests".to_string()
        } else {
            "test-bucket".to_string()
        }
    }

    /// Get Redis database for tests
    pub fn test_redis_db(&self) -> u8 {
        if self.use_external_services { 1 } else { 0 }
    }
}

/// Trait for test contexts that can be created with environment detection
#[async_trait::async_trait]
pub trait TestContext: Sized {
    async fn new() -> Result<Self, Box<dyn std::error::Error>>;
    async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// Macro to create integration tests that work in both modes
#[macro_export]
macro_rules! integration_test {
    ($test_name:ident, $test_body:expr) => {
        #[tokio::test]
        #[ignore] // Ignore by default, run with --ignored or specific filter
        async fn $test_name() -> Result<(), Box<dyn std::error::Error>> {
            let _context = $test_body.await?;
            Ok(())
        }
    };
}

/// Helper function to setup test tracing/logging
pub fn setup_test_logging() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    if env::var("TEST_LOG").is_ok() {
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

/// Test result reporting
pub fn report_test_result(test_name: &str, result: &Result<(), Box<dyn std::error::Error>>) {
    match result {
        Ok(_) => println!("✅ {} passed", test_name),
        Err(e) => println!("❌ {} failed: {}", test_name, e),
    }
}
