//! Vision configuration types for the paladin-llm crate.
//!
//! These are the application-level config types (with `serde` support for
//! YAML/TOML loading) for the vision feature. They map onto the adapter-level
//! types in `crate::openai::vision` via `config_bridge`.

use serde::{Deserialize, Serialize};

/// Retry configuration for vision API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRetryConfig {
    /// Maximum number of retry attempts for transient errors.
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds.
    pub initial_backoff_ms: u64,
    /// Multiplier for exponential backoff (e.g., 2.0 for doubling).
    pub backoff_multiplier: f64,
}

impl Default for VisionRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }
}

impl VisionRetryConfig {
    /// Validates retry configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.backoff_multiplier <= 0.0 {
            return Err("backoff_multiplier must be greater than 0".to_string());
        }
        if self.backoff_multiplier < 1.0 {
            return Err(
                "backoff_multiplier should be at least 1.0 for effective backoff".to_string(),
            );
        }
        Ok(())
    }
}

/// Configuration for a single vision provider (OpenAI or Anthropic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionProviderConfig {
    /// Maximum tokens to request in vision responses.
    pub max_tokens: usize,
}

impl Default for VisionProviderConfig {
    fn default() -> Self {
        Self { max_tokens: 4096 }
    }
}

impl VisionProviderConfig {
    /// Validates provider configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_tokens == 0 {
            return Err("max_tokens must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Configuration for vision capabilities (multi-modal image analysis).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisionConfig {
    /// Retry configuration for vision API calls.
    pub retry: VisionRetryConfig,
    /// OpenAI vision provider configuration.
    pub openai: VisionProviderConfig,
    /// Anthropic vision provider configuration.
    pub anthropic: VisionProviderConfig,
}

impl VisionConfig {
    /// Validates vision configuration.
    pub fn validate(&self) -> Result<(), String> {
        self.retry.validate()?;
        self.openai.validate()?;
        self.anthropic.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_config_defaults() {
        let config = VisionConfig::default();
        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.retry.initial_backoff_ms, 1000);
        assert_eq!(config.retry.backoff_multiplier, 2.0);
        assert_eq!(config.openai.max_tokens, 4096);
        assert_eq!(config.anthropic.max_tokens, 4096);
    }

    #[test]
    fn test_vision_retry_config_validation_success() {
        let config = VisionRetryConfig {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 2.0,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_vision_retry_config_validation_invalid_multiplier() {
        let config = VisionRetryConfig {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 0.0,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be greater than 0"));
    }

    #[test]
    fn test_vision_retry_config_validation_low_multiplier() {
        let config = VisionRetryConfig {
            max_retries: 3,
            initial_backoff_ms: 1000,
            backoff_multiplier: 0.5,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("should be at least 1.0"));
    }

    #[test]
    fn test_vision_provider_config_validation_success() {
        let config = VisionProviderConfig { max_tokens: 4096 };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_vision_provider_config_validation_zero_tokens() {
        let config = VisionProviderConfig { max_tokens: 0 };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be greater than 0"));
    }

    #[test]
    fn test_vision_config_validation_success() {
        let config = VisionConfig::default();
        assert!(config.validate().is_ok());
    }
}
