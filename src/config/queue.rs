//! Configuration for the Redis queue system.

use crate::config::env_utils::EnvOverridable;
use crate::config::redaction::RedactedSecret;
use serde::{Deserialize, Serialize};

/// Configuration for Redis queue
///
/// `Debug` prints `redis_password` as `<redacted>` (DF-07).
#[derive(Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: Option<String>,
    pub redis_db: u8,
    pub connection_timeout: Option<u64>,
    pub key_prefix: Option<String>,
    pub max_retries: Option<u32>,
    pub enable_priority_queues: Option<bool>,
}

impl std::fmt::Debug for QueueConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueConfig")
            .field("redis_host", &self.redis_host)
            .field("redis_port", &self.redis_port)
            .field(
                "redis_password",
                &self.redis_password.as_deref().map(RedactedSecret),
            )
            .field("redis_db", &self.redis_db)
            .field("connection_timeout", &self.connection_timeout)
            .field("key_prefix", &self.key_prefix)
            .field("max_retries", &self.max_retries)
            .field("enable_priority_queues", &self.enable_priority_queues)
            .finish()
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            connection_timeout: Some(30),
            key_prefix: Some("paladin:queue".to_string()),
            max_retries: Some(3),
            enable_priority_queues: Some(true),
        }
    }
}

impl EnvOverridable for QueueConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("APP_REDIS_HOST") {
            self.redis_host = v;
        }
        if let Ok(v) = std::env::var("APP_REDIS_PORT")
            && let Ok(p) = v.parse::<u16>()
        {
            self.redis_port = p;
        }
        if let Ok(v) = std::env::var("APP_REDIS_PASSWORD") {
            self.redis_password = Some(v);
        }
        if let Ok(v) = std::env::var("APP_REDIS_DB")
            && let Ok(d) = v.parse::<u8>()
        {
            self.redis_db = d;
        }
        if let Ok(v) = std::env::var("APP_REDIS_CONNECTION_TIMEOUT")
            && let Ok(t) = v.parse::<u64>()
        {
            self.connection_timeout = Some(t);
        }
        if let Ok(v) = std::env::var("APP_REDIS_KEY_PREFIX") {
            self.key_prefix = Some(v);
        }
        if let Ok(v) = std::env::var("APP_REDIS_MAX_RETRIES")
            && let Ok(r) = v.parse::<u32>()
        {
            self.max_retries = Some(r);
        }
        if let Ok(v) = std::env::var("APP_REDIS_ENABLE_PRIORITY_QUEUES")
            && let Ok(b) = v.parse::<bool>()
        {
            self.enable_priority_queues = Some(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_config_compatibility() {
        let config = QueueConfig::default();
        assert_eq!(config.redis_host, "localhost");
        assert_eq!(config.redis_port, 6379);
        assert_eq!(config.redis_db, 0);
    }

    #[test]
    fn queue_config_debug_redacts_the_redis_password() {
        let sentinel = "test-key-not-a-secret-df07-redis-password";
        let config = QueueConfig {
            redis_password: Some(sentinel.to_string()),
            ..QueueConfig::default()
        };
        // Non-vacuous: the field really holds the sentinel.
        assert_eq!(config.redis_password.as_deref(), Some(sentinel));
        let text = format!("{config:?}");
        assert!(
            !text.contains(sentinel),
            "QueueConfig Debug leaked the redis password"
        );
        assert!(
            text.contains("redis_password: Some(<redacted>)"),
            "no marker in: {text}"
        );
        assert!(
            text.contains("redis_host: \"localhost\""),
            "host missing: {text}"
        );

        let unset = format!("{:?}", QueueConfig::default());
        assert!(
            unset.contains("redis_password: None"),
            "None not shown: {unset}"
        );
    }
}
