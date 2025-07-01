// tests/lib.rs - Test library configuration

//! Integration test library for in4me
//! 
//! This module provides common functionality for integration tests including:
//! - Environment detection (local vs CI/CD)
//! - Service management (Redis, MinIO)
//! - Test utilities and helpers

use std::sync::Once;

// Re-export integration test modules
pub mod integration;

// Initialize logging once for all tests
static INIT: Once = Once::new();

/// Initialize test environment
pub fn init_test_env() {
    INIT.call_once(|| {
        // Set up test logging
        if std::env::var("TEST_LOG").is_ok() {
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("debug")
            ).init();
        }

        unsafe {
            // Set test-specific environment variables
            std::env::set_var("RUST_BACKTRACE", "1");
            
            // Ensure we're in test mode
            std::env::set_var("ENVIRONMENT", "test");
        }
    });
}

/// Common test result type
pub type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Test timing utilities
pub mod timing {
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F, Fut>(
        mut condition: F,
        timeout: Duration,
        check_interval: Duration,
    ) -> bool
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if condition().await {
                return true;
            }
            sleep(check_interval).await;
        }
        
        false
    }

    /// Default timeouts for different operations
    pub const SERVICE_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
    pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(30);
    pub const TEST_OPERATION_TIMEOUT: Duration = Duration::from_secs(10);
    pub const CHECK_INTERVAL: Duration = Duration::from_millis(500);
}

/// Test data generators
pub mod generators {
    use std::path::PathBuf;

    /// Generate test file paths
    pub fn test_file_path(name: &str) -> PathBuf {
        PathBuf::from(format!("test-{}", name))
    }

    /// Generate test content
    pub fn test_content(size: usize) -> String {
        format!("Test content with {} characters: {}", size, "x".repeat(size.saturating_sub(40)))
    }

    /// Generate binary test data
    pub fn test_binary_data(size: usize) -> Vec<u8> {
        (0..size).map(|i| (i % 256) as u8).collect()
    }
}

/// Test assertions and utilities
pub mod assertions {
    use crate::TestResult;

    /// Assert that a future completes within a timeout
    pub async fn assert_timeout<F, T>(
        future: F,
        timeout: std::time::Duration,
        message: &str,
    ) -> TestResult
    where
        F: std::future::Future<Output = T>,
    {
        match tokio::time::timeout(timeout, future).await {
            Ok(_) => Ok(()),
            Err(_) => Err(format!("Timeout: {}", message).into()),
        }
    }

    /// Assert that services are healthy
    pub async fn assert_services_healthy(
        redis_host: &str,
        redis_port: u16,
        minio_endpoint: &str,
    ) -> TestResult {
        // Check Redis
        if !crate::integration::TestEnvironment::check_service_availability(redis_host, redis_port) {
            return Err(format!("Redis not available at {}:{}", redis_host, redis_port).into());
        }

        // Check MinIO
        let (minio_host, minio_port) = crate::integration::TestEnvironment::parse_endpoint(minio_endpoint);
        if !crate::integration::TestEnvironment::check_service_availability(&minio_host, minio_port) {
            return Err(format!("MinIO not available at {}", minio_endpoint).into());
        }

        Ok(())
    }
}

/// Parallel test execution utilities
pub mod parallel {
    use futures::future::try_join_all;
    use crate::TestResult;

    /// Run tests in parallel with controlled concurrency
    pub async fn run_parallel_tests<F, Fut>(
        tests: Vec<(&str, F)>,
        max_concurrency: usize,
    ) -> TestResult
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = TestResult> + Send,
    {
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency));
        
        let tasks: Vec<_> = tests.into_iter().map(|(name, test)| {
            let semaphore = semaphore.clone();
            let name = name.to_string();
            
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                println!("🧪 Running test: {}", name);
                
                match test().await {
                    Ok(_) => {
                        println!("✅ Test passed: {}", name);
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Test failed: {} - {}", name, e);
                        Err(e)
                    }
                }
            })
        }).collect();

        let results: Result<Vec<_>, _> = try_join_all(tasks).await;
        
        match results {
            Ok(test_results) => {
                for result in test_results {
                    result?;
                }
                Ok(())
            }
            Err(e) => Err(format!("Parallel test execution failed: {}", e).into()),
        }
    }
}

/// Performance testing utilities
pub mod performance {
    use std::time::{Duration, Instant};

    /// Simple performance measurement
    pub struct PerfMeasurement {
        start: Instant,
        operation: String,
    }

    impl PerfMeasurement {
        pub fn start(operation: &str) -> Self {
            Self {
                start: Instant::now(),
                operation: operation.to_string(),
            }
        }

        pub fn finish(self) -> Duration {
            let duration = self.start.elapsed();
            println!("⏱️  {}: {:?}", self.operation, duration);
            duration
        }
    }

    /// Benchmark a function
    pub async fn benchmark<F, Fut, T>(
        name: &str,
        iterations: usize,
        mut operation: F,
    ) -> (Duration, Vec<T>)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let mut results = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            results.push(operation().await);
        }

        let total_duration = start.elapsed();
        let avg_duration = total_duration / iterations as u32;

        println!("📊 Benchmark {}: {} iterations, total: {:?}, avg: {:?}",
                name, iterations, total_duration, avg_duration);

        (total_duration, results)
    }
}

// Conditional compilation for different test environments
#[cfg(test)]
mod test_config {
    use chrono::Duration;
    
    /// Test configuration that adapts to environment
    pub struct TestConfig {
        pub use_external_services: bool,
        pub redis_host: String,
        pub redis_port: u16,
        pub minio_endpoint: String,
        pub timeout_multiplier: f32,
    }

    impl TestConfig {
        pub fn from_env() -> Self {
            let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
            let timeout_multiplier = if is_ci { 2.0 } else { 1.0 };

            Self {
                use_external_services: std::env::var("USE_EXTERNAL_TEST_SERVICES")
                    .map(|v| v.to_lowercase() == "true")
                    .unwrap_or(is_ci),
                redis_host: std::env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                redis_port: std::env::var("TEST_REDIS_PORT")
                    .unwrap_or_else(|_| "6380".to_string())
                    .parse()
                    .unwrap_or(6380),
                minio_endpoint: std::env::var("TEST_MINIO_ENDPOINT")
                    .unwrap_or_else(|_| "localhost:9010".to_string()),
                timeout_multiplier,
            }
        }

        pub fn timeout(&self, base: Duration) -> Duration {
            let secs_f = base.num_seconds() as f32 * self.timeout_multiplier;
            let nanos_f = base.num_nanoseconds().unwrap_or(0) as f32 * self.timeout_multiplier;
            // Prefer nanoseconds for higher precision if available
            Duration::nanoseconds(nanos_f.round() as i64)
                .max(Duration::seconds(secs_f.round() as i64))
        }
    }
}

#[cfg(test)]
pub use test_config::TestConfig;