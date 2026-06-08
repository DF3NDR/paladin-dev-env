//! Top-level application [`Settings`] struct and all per-domain
//! `get_*_config()` accessor methods.

use crate::config::env_utils::EnvOverridable;
#[cfg(feature = "s3-storage")]
use crate::infrastructure::adapters::file_storage::minio::MinioConfig;
use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
#[cfg(feature = "s3-storage")]
use std::time::Duration;

#[cfg(feature = "notifications")]
use super::NotificationConfig;
use super::{
    AgentDefinition, ArsenalConfig, CitadelConfig, FileStorageConfig, GarrisonSettings,
    HeraldConfig, LlmConfig, MemoryExtractionConfig, MessageServiceSettings, QueueConfig,
    RagConfig, SanctumAdapterType, SanctumConfig, SchedulerConfig, ServerConfig, SourceConfig,
    VisionConfig,
};

/// Top-level application configuration struct.
///
/// Load from YAML/TOML config files and environment variable overrides via
/// [`Settings::new`] or [`Settings::load_from_file`].
#[allow(missing_docs)]
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
    #[cfg(feature = "notifications")]
    pub notifications: Option<NotificationConfig>,
    pub garrison: Option<GarrisonSettings>,
    pub sanctum: Option<SanctumConfig>,
    pub rag: Option<RagConfig>,
    pub memory_extraction: Option<MemoryExtractionConfig>,
    pub arsenal: Option<ArsenalConfig>,
    pub citadel: Option<CitadelConfig>,
    pub llm: Option<LlmConfig>,
    pub herald: Option<HeraldConfig>,
    pub vision: Option<VisionConfig>,
    pub scheduler: Option<SchedulerConfig>,
    /// Agents to load into the HTTP service host (`paladin-server`). Defaults to empty
    /// when the `agents:` key is absent, so non-server configs are unaffected.
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
}

impl Settings {
    /// Load settings from config files and `APP_*` environment variables.
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .add_source(File::with_name("config").required(true))
            .add_source(Environment::with_prefix("APP"));

        if let Ok(env) = std::env::var("APP_ENV") {
            builder =
                builder.add_source(File::with_name(&format!("config.{}", env)).required(false));
        }

        builder.build()?.try_deserialize()
    }

    /// Load settings from a file at the given path.
    ///
    /// Files with `.yml` or `.yaml` extension are parsed as YAML; all others
    /// are assumed to be TOML.
    pub fn load_from_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let format = if filename.ends_with(".yml") || filename.ends_with(".yaml") {
            FileFormat::Yaml
        } else {
            FileFormat::Toml
        };
        let cfg = Config::builder()
            .add_source(File::new(filename, format))
            .build()?;
        Ok(cfg.try_deserialize()?)
    }

    /// Get queue configuration with environment variable overrides.
    pub fn get_queue_config(&self) -> QueueConfig {
        let mut cfg = self.queue.clone().unwrap_or_default();
        cfg.apply_env_overrides();
        cfg
    }

    /// Get file storage configuration with environment variable overrides.
    pub fn get_file_storage_config(&self) -> FileStorageConfig {
        let mut cfg = self.file_storage.clone().unwrap_or_default();
        cfg.apply_env_overrides();
        cfg
    }

    /// Get notification configuration with environment variable overrides.
    #[cfg(feature = "notifications")]
    pub fn get_notification_config(&self) -> NotificationConfig {
        let mut cfg = self.notifications.clone().unwrap_or_default();
        cfg.apply_env_overrides();
        cfg
    }

    /// Get garrison configuration with environment variable overrides.
    pub fn get_garrison_config(&self) -> GarrisonSettings {
        let mut config = self.garrison.clone().unwrap_or_default();

        if let Ok(garrison_type) = std::env::var("APP_GARRISON_TYPE") {
            config.garrison_type = garrison_type;
        }
        if let Ok(path) = std::env::var("APP_GARRISON_PATH") {
            config.path = Some(path);
        }
        if let Ok(v) = std::env::var("APP_GARRISON_MAX_ENTRIES")
            && let Ok(max_entries) = v.parse::<usize>()
        {
            config.max_entries = max_entries;
        }
        if let Ok(v) = std::env::var("APP_GARRISON_MAX_TOKENS")
            && let Ok(max_tokens) = v.parse::<u32>()
        {
            config.max_tokens = Some(max_tokens);
        }
        if let Ok(tokenizer) = std::env::var("APP_GARRISON_TOKENIZER") {
            config.tokenizer = tokenizer;
        }
        if let Ok(eviction_strategy) = std::env::var("APP_GARRISON_EVICTION_STRATEGY") {
            config.eviction_strategy = eviction_strategy;
        }
        if let Ok(v) = std::env::var("APP_GARRISON_PRESERVE_RECENT_COUNT")
            && let Ok(preserve_recent) = v.parse::<usize>()
        {
            config.preserve_recent_count = preserve_recent;
        }

        config
    }

    /// Get sanctum configuration with environment variable overrides.
    pub fn get_sanctum_config(&self) -> SanctumConfig {
        let mut config = self.sanctum.clone().unwrap_or_default();

        if let Ok(v) = std::env::var("APP_SANCTUM_ENABLED")
            && let Ok(enabled) = v.parse::<bool>()
        {
            config.enabled = enabled;
        }
        if let Ok(adapter_type) = std::env::var("APP_SANCTUM_ADAPTER_TYPE") {
            match adapter_type.to_lowercase().as_str() {
                "in_memory" => config.adapter_type = SanctumAdapterType::InMemory,
                "qdrant" => config.adapter_type = SanctumAdapterType::Qdrant,
                _ => {
                    log::warn!(
                        "Invalid APP_SANCTUM_ADAPTER_TYPE '{}', using default",
                        adapter_type
                    );
                }
            }
        }
        if let Ok(url) = std::env::var("APP_SANCTUM_QDRANT_URL") {
            let mut qdrant = config.qdrant.unwrap_or_default();
            qdrant.url = url;
            config.qdrant = Some(qdrant);
        }
        if let Ok(collection) = std::env::var("APP_SANCTUM_QDRANT_COLLECTION_NAME") {
            let mut qdrant = config.qdrant.unwrap_or_default();
            qdrant.collection_name = collection;
            config.qdrant = Some(qdrant);
        }
        if let Ok(v) = std::env::var("APP_SANCTUM_QDRANT_VECTOR_DIMENSION")
            && let Ok(dimension) = v.parse::<usize>()
        {
            let mut qdrant = config.qdrant.unwrap_or_default();
            qdrant.vector_dimension = dimension;
            config.qdrant = Some(qdrant);
        }

        config
    }

    /// Get citadel configuration with environment variable overrides.
    pub fn get_citadel_config(&self) -> CitadelConfig {
        let mut cfg = self.citadel.clone().unwrap_or_default();
        cfg.apply_env_overrides();
        cfg
    }

    /// Get herald configuration with environment variable overrides.
    pub fn get_herald_config(&self) -> HeraldConfig {
        let mut cfg = self.herald.clone().unwrap_or_default();
        cfg.apply_env_overrides();
        cfg
    }

    /// Create a default [`Herald`](crate::core::platform::container::herald::Herald) instance
    /// from the current herald configuration.
    ///
    /// Reads `herald.default_formatter` and creates the appropriate Herald
    /// implementation.  Returns an error string if the formatter name is
    /// unrecognised.
    pub fn create_default_herald(
        &self,
    ) -> Result<std::sync::Arc<dyn crate::core::platform::container::herald::Herald>, String> {
        use crate::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald};
        use std::sync::Arc;

        let config = self.get_herald_config();

        match config.default_formatter.as_str() {
            "json" => {
                let json_config =
                    crate::infrastructure::adapters::herald::json_herald::JsonHeraldConfig {
                        pretty: config.json.pretty,
                        include_metadata: config.json.include_metadata,
                    };
                let herald = JsonHerald::with_config(json_config);
                Ok(Arc::new(herald))
            }
            "markdown" => {
                let markdown_config =
                    crate::infrastructure::adapters::herald::markdown_herald::MarkdownHeraldConfig {
                        include_colors: config.markdown.include_colors,
                        heading_level: config.markdown.heading_level,
                    };
                let herald = MarkdownHerald::with_config(markdown_config);
                Ok(Arc::new(herald))
            }
            "table" => {
                let table_config =
                    crate::infrastructure::adapters::herald::table_herald::TableHeraldConfig {
                        max_column_width: config.table.max_column_width,
                        border_style: config.table.border_style.clone(),
                    };
                let herald = TableHerald::new(table_config);
                Ok(Arc::new(herald))
            }
            other => Err(format!(
                "Unknown formatter '{}'. Valid options: json, markdown, table",
                other
            )),
        }
    }

    /// Get vision configuration with environment variable overrides.
    pub fn get_vision_config(&self) -> VisionConfig {
        let mut config = self.vision.clone().unwrap_or_default();

        if let Ok(v) = std::env::var("APP_VISION_RETRY_MAX_RETRIES")
            && let Ok(max_retries) = v.parse::<u32>()
        {
            config.retry.max_retries = max_retries;
        }
        if let Ok(v) = std::env::var("APP_VISION_RETRY_INITIAL_BACKOFF_MS")
            && let Ok(initial_backoff) = v.parse::<u64>()
        {
            config.retry.initial_backoff_ms = initial_backoff;
        }
        if let Ok(v) = std::env::var("APP_VISION_RETRY_BACKOFF_MULTIPLIER")
            && let Ok(backoff_multiplier) = v.parse::<f64>()
        {
            config.retry.backoff_multiplier = backoff_multiplier;
        }
        if let Ok(v) = std::env::var("APP_VISION_OPENAI_MAX_TOKENS")
            && let Ok(max_tokens) = v.parse::<usize>()
        {
            config.openai.max_tokens = max_tokens;
        }
        if let Ok(v) = std::env::var("APP_VISION_ANTHROPIC_MAX_TOKENS")
            && let Ok(max_tokens) = v.parse::<usize>()
        {
            config.anthropic.max_tokens = max_tokens;
        }

        config
    }

    /// Convert the file-storage config to a [`MinioConfig`].
    #[cfg(feature = "s3-storage")]
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
            #[cfg(feature = "notifications")]
            notifications: Some(NotificationConfig::default()),
            garrison: Some(GarrisonSettings::default()),
            sanctum: Some(SanctumConfig::default()),
            rag: Some(RagConfig::default()),
            memory_extraction: Some(MemoryExtractionConfig::default()),
            arsenal: Some(ArsenalConfig::default()),
            citadel: Some(CitadelConfig::default()),
            llm: Some(LlmConfig::default()),
            herald: Some(HeraldConfig::default()),
            vision: Some(VisionConfig::default()),
            scheduler: Some(SchedulerConfig::default()),
            agents: Vec::new(),
        }
    }
}
