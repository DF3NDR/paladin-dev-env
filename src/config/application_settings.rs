use serde::{Deserialize, Serialize};
use config::{Config, ConfigError, File, Environment};
use std::fs;
use std::time::Duration;
use crate::infrastructure::adapters::file_storage::minio::MinioConfig;
use crate::infrastructure::adapters::notifications::{EmailAdapterConfig, SystemAdapterConfig};

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

/// Configuration for MinIO file storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    pub minio_endpoint: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub minio_bucket: String,
    pub minio_region: Option<String>,
    pub minio_secure: Option<bool>,
    pub minio_path_style: Option<bool>,
    pub connection_timeout: Option<u64>,
    pub request_timeout: Option<u64>,
    pub max_idle_conns: Option<u32>,
    pub max_file_size: Option<u64>,
    pub allowed_extensions: Option<Vec<String>>,
}

impl Default for FileStorageConfig {
    fn default() -> Self {
        Self {
            minio_endpoint: "localhost:9000".to_string(),
            minio_access_key: "minioadmin".to_string(),
            minio_secret_key: "minioadmin".to_string(),
            minio_bucket: "in4me-files".to_string(),
            minio_region: None,
            minio_secure: Some(false),
            minio_path_style: Some(true),
            connection_timeout: Some(30),
            request_timeout: Some(300),
            max_idle_conns: Some(10),
            max_file_size: Some(100 * 1024 * 1024), // 100MB
            allowed_extensions: Some(vec![
                "txt".to_string(), "md".to_string(), "json".to_string(),
                "pdf".to_string(), "doc".to_string(), "docx".to_string(),
                "jpg".to_string(), "png".to_string(), "gif".to_string(),
                "rs".to_string(), "py".to_string(), "js".to_string(),
                "html".to_string(), "css".to_string(), "xml".to_string(),
            ]),
        }
    }
}

/// Configuration for notification system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable/disable notification system
    pub enabled: bool,
    /// Email notification configuration
    pub email: Option<EmailAdapterConfig>,
    /// System notification configuration  
    pub system: Option<SystemAdapterConfig>,
    /// Global notification settings
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
    pub enable_delivery_tracking: bool,
    /// Rate limiting settings
    pub global_rate_limit_per_minute: Option<u32>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            email: Some(EmailAdapterConfig::default()),
            system: Some(SystemAdapterConfig::default()),
            max_retries: 3,
            retry_delay_seconds: 60,
            enable_delivery_tracking: true,
            global_rate_limit_per_minute: Some(100),
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
    pub file_storage: Option<FileStorageConfig>,
    pub notifications: Option<NotificationConfig>,
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

    /// Get file storage configuration with environment variable overrides
    pub fn get_file_storage_config(&self) -> FileStorageConfig {
        let mut config = self.file_storage.clone().unwrap_or_default();

        // Override with environment variables if present
        if let Ok(endpoint) = std::env::var("APP_MINIO_ENDPOINT") {
            config.minio_endpoint = endpoint;
        }

        if let Ok(access_key) = std::env::var("APP_MINIO_ACCESS_KEY") {
            config.minio_access_key = access_key;
        }

        if let Ok(secret_key) = std::env::var("APP_MINIO_SECRET_KEY") {
            config.minio_secret_key = secret_key;
        }

        if let Ok(bucket) = std::env::var("APP_MINIO_BUCKET") {
            config.minio_bucket = bucket;
        }

        if let Ok(region) = std::env::var("APP_MINIO_REGION") {
            config.minio_region = Some(region);
        }

        if let Ok(secure_str) = std::env::var("APP_MINIO_SECURE") {
            if let Ok(secure) = secure_str.parse::<bool>() {
                config.minio_secure = Some(secure);
            }
        }

        if let Ok(path_style_str) = std::env::var("APP_MINIO_PATH_STYLE") {
            if let Ok(path_style) = path_style_str.parse::<bool>() {
                config.minio_path_style = Some(path_style);
            }
        }

        if let Ok(timeout_str) = std::env::var("APP_MINIO_CONNECTION_TIMEOUT") {
            if let Ok(timeout) = timeout_str.parse::<u64>() {
                config.connection_timeout = Some(timeout);
            }
        }

        if let Ok(request_timeout_str) = std::env::var("APP_MINIO_REQUEST_TIMEOUT") {
            if let Ok(timeout) = request_timeout_str.parse::<u64>() {
                config.request_timeout = Some(timeout);
            }
        }

        if let Ok(max_conns_str) = std::env::var("APP_MINIO_MAX_IDLE_CONNS") {
            if let Ok(max_conns) = max_conns_str.parse::<u32>() {
                config.max_idle_conns = Some(max_conns);
            }
        }

        if let Ok(max_size_str) = std::env::var("APP_MINIO_MAX_FILE_SIZE") {
            if let Ok(max_size) = max_size_str.parse::<u64>() {
                config.max_file_size = Some(max_size);
            }
        }

        if let Ok(extensions_str) = std::env::var("APP_MINIO_ALLOWED_EXTENSIONS") {
            let extensions: Vec<String> = extensions_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect();
            if !extensions.is_empty() {
                config.allowed_extensions = Some(extensions);
            }
        }

        config
    }

    /// Get notification configuration with environment variable overrides
    pub fn get_notification_config(&self) -> NotificationConfig {
        let mut config = self.notifications.clone().unwrap_or_default();

        // Override with environment variables if present
        if let Ok(enabled_str) = std::env::var("APP_NOTIFICATIONS_ENABLED") {
            if let Ok(enabled) = enabled_str.parse::<bool>() {
                config.enabled = enabled;
            }
        }

        // Email configuration overrides
        if let Some(ref mut email_config) = config.email {
            if let Ok(smtp_host) = std::env::var("APP_EMAIL_SMTP_HOST") {
                email_config.smtp_host = smtp_host;
            }
            if let Ok(smtp_port_str) = std::env::var("APP_EMAIL_SMTP_PORT") {
                if let Ok(smtp_port) = smtp_port_str.parse::<u16>() {
                    email_config.smtp_port = smtp_port;
                }
            }
            if let Ok(username) = std::env::var("APP_EMAIL_USERNAME") {
                email_config.username = username;
            }
            if let Ok(password) = std::env::var("APP_EMAIL_PASSWORD") {
                email_config.password = password;
            }
            if let Ok(from_address) = std::env::var("APP_EMAIL_FROM_ADDRESS") {
                email_config.from_address = from_address;
            }
            if let Ok(from_name) = std::env::var("APP_EMAIL_FROM_NAME") {
                email_config.from_name = Some(from_name);
            }
            if let Ok(use_tls_str) = std::env::var("APP_EMAIL_USE_TLS") {
                if let Ok(use_tls) = use_tls_str.parse::<bool>() {
                    email_config.use_tls = use_tls;
                }
            }
        }

        config
    }

    /// Convert FileStorageConfig to MinioConfig
    pub fn to_minio_config(&self) -> MinioConfig {
        let fs_config = self.get_file_storage_config();
        
        MinioConfig {
            endpoint: fs_config.minio_endpoint,
            access_key: fs_config.minio_access_key,
            secret_key: fs_config.minio_secret_key,
            bucket: fs_config.minio_bucket,
            region: fs_config.minio_region,
            secure: fs_config.minio_secure.unwrap_or(false),
            path_style: fs_config.minio_path_style.unwrap_or(true),
            connection_timeout: Duration::from_secs(fs_config.connection_timeout.unwrap_or(30)),
            request_timeout: Duration::from_secs(fs_config.request_timeout.unwrap_or(300)),
            max_idle_conns: fs_config.max_idle_conns.unwrap_or(10),
            max_retries: fs_config.max_idle_conns.unwrap_or(3),
        }
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
            file_storage: Some(FileStorageConfig::default()),
            notifications: Some(NotificationConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use serial_test::serial;

    #[test]
    fn test_default_file_storage_config() {
        let config = FileStorageConfig::default();
        assert_eq!(config.minio_endpoint, "localhost:9000");
        assert_eq!(config.minio_access_key, "minioadmin");
        assert_eq!(config.minio_secret_key, "minioadmin");
        assert_eq!(config.minio_bucket, "in4me-files");
        assert_eq!(config.minio_secure, Some(false));
        assert_eq!(config.minio_path_style, Some(true));
        assert_eq!(config.connection_timeout, Some(30));
        assert_eq!(config.request_timeout, Some(300));
        assert_eq!(config.max_idle_conns, Some(10));
        assert_eq!(config.max_file_size, Some(100 * 1024 * 1024));
        assert!(config.allowed_extensions.is_some());
    }

    #[test]
    #[serial]
    fn test_file_storage_config_env_override() {
        // Set environment variables
        unsafe {
            env::set_var("APP_MINIO_ENDPOINT", "minio-server:9000");
            env::set_var("APP_MINIO_ACCESS_KEY", "testuser");
            env::set_var("APP_MINIO_SECRET_KEY", "testpass");
            env::set_var("APP_MINIO_BUCKET", "test-bucket");
            env::set_var("APP_MINIO_REGION", "us-east-1");
            env::set_var("APP_MINIO_SECURE", "true");
            env::set_var("APP_MINIO_PATH_STYLE", "false");
            env::set_var("APP_MINIO_CONNECTION_TIMEOUT", "60");
            env::set_var("APP_MINIO_REQUEST_TIMEOUT", "600");
            env::set_var("APP_MINIO_MAX_IDLE_CONNS", "20");
            env::set_var("APP_MINIO_MAX_FILE_SIZE", "209715200"); // 200MB
            env::set_var("APP_MINIO_ALLOWED_EXTENSIONS", "pdf,doc,docx,jpg,png");
        }
        let settings = Settings::default();
        let config = settings.get_file_storage_config();

        assert_eq!(config.minio_endpoint, "minio-server:9000");
        assert_eq!(config.minio_access_key, "testuser");
        assert_eq!(config.minio_secret_key, "testpass");
        assert_eq!(config.minio_bucket, "test-bucket");
        assert_eq!(config.minio_region, Some("us-east-1".to_string()));
        assert_eq!(config.minio_secure, Some(true));
        assert_eq!(config.minio_path_style, Some(false));
        assert_eq!(config.connection_timeout, Some(60));
        assert_eq!(config.request_timeout, Some(600));
        assert_eq!(config.max_idle_conns, Some(20));
        assert_eq!(config.max_file_size, Some(209715200));
        assert_eq!(config.allowed_extensions, Some(vec![
            "pdf".to_string(), "doc".to_string(), "docx".to_string(),
            "jpg".to_string(), "png".to_string()
        ]));

        // Clean up
        unsafe {
            env::remove_var("APP_MINIO_ENDPOINT");
            env::remove_var("APP_MINIO_ACCESS_KEY");
            env::remove_var("APP_MINIO_SECRET_KEY");
            env::remove_var("APP_MINIO_BUCKET");
            env::remove_var("APP_MINIO_REGION");
            env::remove_var("APP_MINIO_SECURE");
            env::remove_var("APP_MINIO_PATH_STYLE");
            env::remove_var("APP_MINIO_CONNECTION_TIMEOUT");
            env::remove_var("APP_MINIO_REQUEST_TIMEOUT");
            env::remove_var("APP_MINIO_MAX_IDLE_CONNS");
            env::remove_var("APP_MINIO_MAX_FILE_SIZE");
            env::remove_var("APP_MINIO_ALLOWED_EXTENSIONS");
        }
    }

    #[test]
    fn test_settings_with_file_storage_config() {
        let settings = Settings {
            file_storage: Some(FileStorageConfig {
                minio_endpoint: "custom-minio:9000".to_string(),
                minio_access_key: "custom-access".to_string(),
                minio_secret_key: "custom-secret".to_string(),
                minio_bucket: "custom-bucket".to_string(),
                minio_region: Some("eu-west-1".to_string()),
                minio_secure: Some(true),
                minio_path_style: Some(false),
                connection_timeout: Some(45),
                request_timeout: Some(450),
                max_idle_conns: Some(15),
                max_file_size: Some(50 * 1024 * 1024), // 50MB
                allowed_extensions: Some(vec!["rs".to_string(), "toml".to_string()]),
            }),
            ..Default::default()
        };

        let config = settings.get_file_storage_config();
        assert_eq!(config.minio_endpoint, "custom-minio:9000");
        assert_eq!(config.minio_access_key, "custom-access");
        assert_eq!(config.minio_secret_key, "custom-secret");
        assert_eq!(config.minio_bucket, "custom-bucket");
        assert_eq!(config.minio_region, Some("eu-west-1".to_string()));
    }

    #[test]
    fn test_to_minio_config_conversion() {
        let settings = Settings::default();
        let minio_config = settings.to_minio_config();

        assert_eq!(minio_config.endpoint, "localhost:9000");
        assert_eq!(minio_config.access_key, "minioadmin");
        assert_eq!(minio_config.secret_key, "minioadmin");
        assert_eq!(minio_config.bucket, "in4me-files");
        assert!(!minio_config.secure);
        assert!(minio_config.path_style);
        assert_eq!(minio_config.connection_timeout, Duration::from_secs(30));
        assert_eq!(minio_config.request_timeout, Duration::from_secs(300));
        assert_eq!(minio_config.max_idle_conns, 10);
    }

    #[test]
    fn test_queue_config_compatibility() {
        // Ensure existing queue config functionality still works
        let settings = Settings::default();
        let queue_config = settings.get_queue_config();
        
        assert_eq!(queue_config.redis_host, "localhost");
        assert_eq!(queue_config.redis_port, 6379);
        assert_eq!(queue_config.redis_db, 0);
    }
}