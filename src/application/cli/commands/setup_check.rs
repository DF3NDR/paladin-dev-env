//! Setup check command
//!
//! Validates the Paladin environment setup including:
//! - CLI version and Rust toolchain
//! - Environment variables and .env file
//! - LLM provider connectivity (OpenAI, Anthropic, DeepSeek)
//! - Optional service connectivity (Redis, Qdrant)

use super::provider_probe::{validate_anthropic_key, validate_deepseek_key, validate_openai_key};
use crate::application::cli::error::{CliError, CliResult};
use crate::application::cli::formatters::output::OutputFormatter;
use crate::application::cli::formatters::progress::Spinner;
use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Validation result for a specific check
#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckStatus {
    Pass,
    Warn(String),
    Fail(String),
    Skip(String),
}

/// Category of check being performed
#[derive(Debug, Clone, Copy)]
enum CheckCategory {
    System,
    Environment,
    Provider,
    Service,
}

impl CheckCategory {
    fn label(&self) -> &str {
        match self {
            CheckCategory::System => "System",
            CheckCategory::Environment => "Environment",
            CheckCategory::Provider => "LLM Provider",
            CheckCategory::Service => "Service",
        }
    }
}

/// Result of a single validation check
struct CheckResult {
    category: CheckCategory,
    name: String,
    status: CheckStatus,
}

impl CheckResult {
    fn new(category: CheckCategory, name: impl Into<String>, status: CheckStatus) -> Self {
        Self {
            category,
            name: name.into(),
            status,
        }
    }
}

// API key validation lives in the shared `provider_probe` module (DF-19).

/// Check Redis connectivity
async fn check_redis_connectivity(redis_url: &str) -> CliResult<()> {
    // For now, just validate the URL format
    // Full redis connectivity check would require redis client dependency
    if redis_url.starts_with("redis://") || redis_url.starts_with("rediss://") {
        Ok(())
    } else {
        // WR-03: the value is never echoed. A malformed URL may still carry a
        // password (`redis:/:<pw>@host`), and this text reaches the terminal and CI logs.
        Err(CliError::configuration(
            "Invalid Redis URL format: expected a redis:// or rediss:// URL (value not shown)",
        ))
    }
}

/// Check Qdrant connectivity
async fn check_qdrant_connectivity(qdrant_url: &str) -> CliResult<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| CliError::execution(format!("Failed to create HTTP client: {}", e)))?;

    // Try to get collections list
    let url = format!("{}/collections", qdrant_url.trim_end_matches('/'));
    let response = client
        .get(&url)
        .send()
        .await
        // WR-03: reqwest appends the request URL to its error text, and a credential
        // in the query string would reach the terminal and CI logs with it.
        .map_err(|e| {
            CliError::execution(format!("Qdrant connection failed: {}", e.without_url()))
        })?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(CliError::execution(format!(
            "Qdrant API check failed ({})",
            response.status()
        )))
    }
}

// Check Functions

/// Check Paladin CLI version
fn check_cli_version() -> CheckResult {
    let version = env!("CARGO_PKG_VERSION");
    CheckResult::new(
        CheckCategory::System,
        format!("Paladin CLI v{}", version),
        CheckStatus::Pass,
    )
}

/// Check Rust toolchain version
fn check_rust_version() -> CheckResult {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    match output {
        Some(version) => CheckResult::new(
            CheckCategory::System,
            format!("Rust Toolchain ({})", version),
            CheckStatus::Pass,
        ),
        None => CheckResult::new(
            CheckCategory::System,
            "Rust Toolchain",
            CheckStatus::Fail("Could not detect Rust toolchain".into()),
        ),
    }
}

/// Check for .env file
fn check_env_file() -> CheckResult {
    let env_path = Path::new(".env");
    if env_path.exists() {
        CheckResult::new(CheckCategory::Environment, ".env file", CheckStatus::Pass)
    } else {
        CheckResult::new(
            CheckCategory::Environment,
            ".env file",
            CheckStatus::Warn("Not found - run 'paladin onboarding' to create".into()),
        )
    }
}

/// Check OpenAI configuration
async fn check_openai() -> CheckResult {
    match env::var("OPENAI_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => match validate_openai_key(&api_key).await {
            Ok(_) => CheckResult::new(CheckCategory::Provider, "OpenAI", CheckStatus::Pass),
            Err(e) => CheckResult::new(
                CheckCategory::Provider,
                "OpenAI",
                CheckStatus::Fail(format!("API validation failed: {}", e)),
            ),
        },
        _ => CheckResult::new(
            CheckCategory::Provider,
            "OpenAI",
            CheckStatus::Skip("OPENAI_API_KEY not configured".into()),
        ),
    }
}

/// Check Anthropic configuration
async fn check_anthropic() -> CheckResult {
    match env::var("ANTHROPIC_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => match validate_anthropic_key(&api_key).await {
            Ok(_) => CheckResult::new(CheckCategory::Provider, "Anthropic", CheckStatus::Pass),
            Err(e) => CheckResult::new(
                CheckCategory::Provider,
                "Anthropic",
                CheckStatus::Fail(format!("API validation failed: {}", e)),
            ),
        },
        _ => CheckResult::new(
            CheckCategory::Provider,
            "Anthropic",
            CheckStatus::Skip("ANTHROPIC_API_KEY not configured".into()),
        ),
    }
}

/// Check DeepSeek configuration
async fn check_deepseek() -> CheckResult {
    match env::var("DEEPSEEK_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => match validate_deepseek_key(&api_key).await {
            Ok(_) => CheckResult::new(CheckCategory::Provider, "DeepSeek", CheckStatus::Pass),
            Err(e) => CheckResult::new(
                CheckCategory::Provider,
                "DeepSeek",
                CheckStatus::Fail(format!("API validation failed: {}", e)),
            ),
        },
        _ => CheckResult::new(
            CheckCategory::Provider,
            "DeepSeek",
            CheckStatus::Skip("DEEPSEEK_API_KEY not configured".into()),
        ),
    }
}

/// Check Redis configuration and connectivity
async fn check_redis() -> CheckResult {
    match env::var("REDIS_URL") {
        Ok(redis_url) if !redis_url.is_empty() => {
            match check_redis_connectivity(&redis_url).await {
                Ok(_) => CheckResult::new(CheckCategory::Service, "Redis", CheckStatus::Pass),
                Err(e) => CheckResult::new(
                    CheckCategory::Service,
                    "Redis",
                    CheckStatus::Fail(format!("Connection failed: {}", e)),
                ),
            }
        }
        _ => CheckResult::new(
            CheckCategory::Service,
            "Redis",
            CheckStatus::Skip("REDIS_URL not configured".into()),
        ),
    }
}

/// Check Qdrant configuration and connectivity
async fn check_qdrant() -> CheckResult {
    match env::var("QDRANT_URL") {
        Ok(qdrant_url) if !qdrant_url.is_empty() => {
            match check_qdrant_connectivity(&qdrant_url).await {
                Ok(_) => CheckResult::new(CheckCategory::Service, "Qdrant", CheckStatus::Pass),
                Err(e) => CheckResult::new(
                    CheckCategory::Service,
                    "Qdrant",
                    CheckStatus::Fail(format!("Connection failed: {}", e)),
                ),
            }
        }
        _ => CheckResult::new(
            CheckCategory::Service,
            "Qdrant",
            CheckStatus::Skip("QDRANT_URL not configured".into()),
        ),
    }
}

/// Run environment setup checks
pub async fn run_setup_check(verbose: bool) -> Result<i32, CliError> {
    let formatter = OutputFormatter::new();

    formatter.header("Paladin Setup Check");
    println!();

    // Collect all checks
    let mut results = Vec::new();

    // System checks
    results.push(check_cli_version());
    results.push(check_rust_version());

    // Environment checks
    results.push(check_env_file());

    // Provider checks (with async validation)
    let spinner = Spinner::new("Checking LLM providers...");
    results.push(check_openai().await);
    results.push(check_anthropic().await);
    results.push(check_deepseek().await);
    spinner.finish_with_message("LLM provider checks complete");
    println!();

    // Service checks
    let spinner = Spinner::new("Checking optional services...");
    results.push(check_redis().await);
    results.push(check_qdrant().await);
    spinner.finish_with_message("Service checks complete");
    println!();

    // Group results by category
    let mut categorized: HashMap<String, Vec<&CheckResult>> = HashMap::new();
    for result in &results {
        categorized
            .entry(result.category.label().to_string())
            .or_default()
            .push(result);
    }

    // Display results by category
    let mut has_failures = false;
    let mut has_warnings = false;

    for (category, checks) in &categorized {
        println!("{}:", category);
        for check in checks {
            match &check.status {
                CheckStatus::Pass => {
                    println!("  ✓ {} ", check.name);
                    if verbose {
                        formatter.info("  All checks passed");
                    }
                }
                CheckStatus::Warn(msg) => {
                    has_warnings = true;
                    println!("  ⚠ {} ", check.name);
                    formatter.warning(&format!("  {}", msg));
                }
                CheckStatus::Fail(msg) => {
                    has_failures = true;
                    println!("  ✗ {} ", check.name);
                    formatter.error(&format!("  {}", msg));
                }
                CheckStatus::Skip(msg) => {
                    if verbose {
                        println!("  ○ {} ", check.name);
                        formatter.info(&format!("  {}", msg));
                    }
                }
            }
        }
        println!();
    }

    // Summary
    let pass_count = results
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Pass))
        .count();
    let warn_count = results
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Warn(_)))
        .count();
    let fail_count = results
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Fail(_)))
        .count();
    let skip_count = results
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Skip(_)))
        .count();

    formatter.header("Summary");
    println!("  Passed: {}", pass_count);
    if warn_count > 0 {
        println!("  Warnings: {}", warn_count);
    }
    if fail_count > 0 {
        println!("  Failed: {}", fail_count);
    }
    if verbose && skip_count > 0 {
        println!("  Skipped: {}", skip_count);
    }
    println!();

    // Exit code
    let exit_code = if has_failures {
        formatter.error("Setup check failed - please address the errors above");
        1
    } else if has_warnings {
        formatter.warning("Setup check completed with warnings");
        2
    } else {
        formatter.success("All checks passed! ✨");
        0
    };

    println!();
    if exit_code > 0 {
        formatter.box_message(&[
            "Next Steps:",
            "1. Run 'paladin onboarding' to configure missing providers",
            "2. Verify environment variables in your .env file",
            "3. Run 'paladin setup-check' again to verify",
        ]);
    }

    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_cli_version() {
        let result = check_cli_version();
        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn test_check_rust_version() {
        let result = check_rust_version();
        // Should either pass or fail, never skip
        assert!(matches!(
            result.status,
            CheckStatus::Pass | CheckStatus::Fail(_)
        ));
    }

    #[test]
    fn test_check_env_file() {
        let result = check_env_file();
        // Should either pass or warn, never fail
        assert!(matches!(
            result.status,
            CheckStatus::Pass | CheckStatus::Warn(_)
        ));
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_openai_key_invalid() {
        let result = validate_openai_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_anthropic_key_invalid() {
        let result = validate_anthropic_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_validate_deepseek_key_invalid() {
        let result = validate_deepseek_key("invalid-key-12345").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_openai_no_env() {
        // Temporarily remove env var
        unsafe {
            env::remove_var("OPENAI_API_KEY");
        }
        let result = check_openai().await;
        assert!(matches!(result.status, CheckStatus::Skip(_)));
    }

    #[tokio::test]
    async fn test_check_redis_invalid_url() {
        let result = check_redis_connectivity("invalid-url").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_redis_valid_url_format() {
        let result = check_redis_connectivity("redis://localhost:6379").await;
        assert!(result.is_ok());
    }

    /// WR-03 (56-REVIEW): the malformed-scheme error must carry none of the URL.
    #[tokio::test]
    async fn check_redis_format_error_does_not_echo_the_value() {
        const PW: &str = "test-key-not-a-secret-wr03-redis";
        for url in [
            format!("redis:/:{PW}@host"),
            format!("valkey://:{PW}@host"),
            format!("http://u:{PW}@host"),
        ] {
            let err = check_redis_connectivity(&url)
                .await
                .expect_err("a malformed-scheme URL must be rejected");
            let display = err.to_string();
            let debug = format!("{err:?}");
            assert!(
                !display.contains(PW),
                "the Redis format error Display must not carry the URL password"
            );
            assert!(
                !debug.contains(PW),
                "the Redis format error Debug must not carry the URL password"
            );
            assert!(
                display.contains(
                    "Invalid Redis URL format: expected a redis:// or rediss:// URL (value not shown)"
                ),
                "the Redis format error must use the fixed value-free text"
            );
        }
    }

    /// WR-03 (56-REVIEW): a failed Qdrant request must not print the URL credentials.
    ///
    /// reqwest strips URL userinfo from its own error text, but it still echoes a
    /// credential carried in the query string, so both forms are pinned here.
    #[tokio::test]
    async fn check_qdrant_failure_does_not_echo_the_url_credentials() {
        const PW: &str = "test-key-not-a-secret-wr03-qdrant";
        // Loopback port 1 refuses the connection, so no external network is used.
        for url in [
            format!("http://user:{PW}@127.0.0.1:1"),
            format!("http://127.0.0.1:1?api-key={PW}"),
        ] {
            let err = check_qdrant_connectivity(&url)
                .await
                .expect_err("a refused loopback connection must be an error");
            let display = err.to_string();
            let debug = format!("{err:?}");
            assert!(
                !display.contains(PW),
                "the Qdrant failure Display must not carry the URL credentials"
            );
            assert!(
                !debug.contains(PW),
                "the Qdrant failure Debug must not carry the URL credentials"
            );
        }
    }
}
