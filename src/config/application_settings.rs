use serde::{Deserialize, Serialize};
use config::{Config, ConfigError, File, Environment};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceConfig {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub prompt: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageServiceSettings {
    pub max_queue_size: Option<usize>,
    pub default_ttl_seconds: Option<i64>,
    pub enable_persistence: Option<bool>,
    pub worker_threads: Option<usize>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

/// Configuration for Redis queue
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_password: None,
            redis_db: 0,
            connection_timeout: Some(30),
            key_prefix: Some("in4me:queue".to_string()),
            max_retries: Some(3),
            enable_priority_queues: Some(true),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub llm_type: String,
    pub llm_url: String,
    pub llm_api_key: String,
    pub server: ServerConfig,
    pub sources: Vec<SourceConfig>,
    pub max_file_size: u64,
    pub message_service: Option<MessageServiceSettings>,
    pub queue: Option<QueueConfig>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .add_source(File::with_name("config").required(true))
            .add_source(Environment::with_prefix("APP"));

        if let Ok(env) = std::env::var("APP_ENV") {
            builder = builder.add_source(File::with_name(&format!("config.{}", env)).required(false));
        }

        builder.build()?.try_deserialize()
    }

    pub fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(filename)?;
        let config: Settings = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get queue configuration with environment variable overrides
    pub fn get_queue_config(&self) -> QueueConfig {
        let mut config = self.queue.clone().unwrap_or_default();

        // Override with environment variables if present
        if let Ok(host) = std::env::var("APP_REDIS_HOST") {
            config.redis_host = host;
        }

        if let Ok(port_str) = std::env::var("APP_REDIS_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                config.redis_port = port;
            }
        }

        if let Ok(password) = std::env::var("APP_REDIS_PASSWORD") {
            config.redis_password = Some(password);
        }

        if let Ok(db_str) = std::env::var("APP_REDIS_DB") {
            if let Ok(db) = db_str.parse::<u8>() {
                config.redis_db = db;
            }
        }

        if let Ok(timeout_str) = std::env::var("APP_REDIS_CONNECTION_TIMEOUT") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                config.connection_timeout = Some(timeout);
            }
        }

        if let Ok(prefix) = std::env::var("APP_REDIS_KEY_PREFIX") {
            config.key_prefix = Some(prefix);
        }

        if let Ok(retries_str) = std::env::var("APP_REDIS_MAX_RETRIES") {
            if let Ok(retries) = retries_str.parse::<u32>() {
                config.max_retries = Some(retries);
            }
        }

        if let Ok(priority_str) = std::env::var("APP_REDIS_ENABLE_PRIORITY_QUEUES") {
            if let Ok(enable) = priority_str.parse::<bool>() {
                config.enable_priority_queues = Some(enable);
            }
        }

        config
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm_type: "openai".to_string(),
            llm_url: "https://api.openai.com/v1".to_string(),
            llm_api_key: "".to_string(),
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            sources: Vec::new(),
            max_file_size: 10 * 1024 * 1024, // 10MB
            message_service: Some(MessageServiceSettings {
                max_queue_size: Some(10000),
                default_ttl_seconds: Some(3600),
                enable_persistence: Some(false),
                worker_threads: Some(4),
                retry_attempts: Some(3),
                retry_delay_ms: Some(1000),
            }),
            queue: Some(QueueConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_queue_config() {
        let config = QueueConfig::default();
        assert_eq!(config.redis_host, "localhost");
        assert_eq!(config.redis_port, 6379);
        assert_eq!(config.redis_db, 0);
        assert!(config.redis_password.is_none());
        assert_eq!(config.connection_timeout, Some(30));
        assert_eq!(config.key_prefix, Some("in4me:queue".to_string()));
        assert_eq!(config.max_retries, Some(3));
        assert_eq!(config.enable_priority_queues, Some(true));
    }

    #[test]
    fn test_queue_config_env_override() {
        // Set environment variables
        unsafe {
            env::set_var("APP_REDIS_HOST", "redis-server");
            env::set_var("APP_REDIS_PORT", "6380");
            env::set_var("APP_REDIS_PASSWORD", "secret");
            env::set_var("APP_REDIS_DB", "1");
            env::set_var("APP_REDIS_CONNECTION_TIMEOUT", "60");
            env::set_var("APP_REDIS_KEY_PREFIX", "myapp:queue");
            env::set_var("APP_REDIS_MAX_RETRIES", "5");
            env::set_var("APP_REDIS_ENABLE_PRIORITY_QUEUES", "false");
        }
        let settings = Settings::default();
        let config = settings.get_queue_config();

        assert_eq!(config.redis_host, "redis-server");
        assert_eq!(config.redis_port, 6380);
        assert_eq!(config.redis_password, Some("secret".to_string()));
        assert_eq!(config.redis_db, 1);
        assert_eq!(config.connection_timeout, Some(60));
        assert_eq!(config.key_prefix, Some("myapp:queue".to_string()));
        assert_eq!(config.max_retries, Some(5));
        assert_eq!(config.enable_priority_queues, Some(false));

        // Clean up
        unsafe {
            env::remove_var("APP_REDIS_HOST");
            env::remove_var("APP_REDIS_PORT");
            env::remove_var("APP_REDIS_PASSWORD");
            env::remove_var("APP_REDIS_DB");
            env::remove_var("APP_REDIS_CONNECTION_TIMEOUT");
            env::remove_var("APP_REDIS_KEY_PREFIX");
            env::remove_var("APP_REDIS_MAX_RETRIES");
            env::remove_var("APP_REDIS_ENABLE_PRIORITY_QUEUES");
        }
    }

    #[test]
    fn test_settings_with_queue_config() {
        let settings = Settings {
            queue: Some(QueueConfig {
                redis_host: "custom-host".to_string(),
                redis_port: 6000,
                redis_password: Some("password".to_string()),
                redis_db: 2,
                connection_timeout: Some(45),
                key_prefix: Some("custom:prefix".to_string()),
                max_retries: Some(10),
                enable_priority_queues: Some(false),
            }),
            ..Default::default()
        };

        let config = settings.get_queue_config();
        assert_eq!(config.redis_host, "custom-host");
        assert_eq!(config.redis_port, 6000);
        assert_eq!(config.redis_password, Some("password".to_string()));
        assert_eq!(config.redis_db, 2);
    }
}