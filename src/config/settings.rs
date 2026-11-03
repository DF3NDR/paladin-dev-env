//! Top-level application [`Settings`] struct and all per-domain
//! `get_*_config()` accessor methods.

use crate::config::agent_runtime::AgentRuntimeConfig;
use crate::config::env_utils::EnvOverridable;
use crate::config::redaction::RedactedSecret;
use crate::config::trace::TraceConfig;
use crate::config::treasurer::TreasurerConfig;
#[cfg(feature = "s3-storage")]
use crate::infrastructure::adapters::file_storage::minio::MinioConfig;
use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
#[cfg(feature = "s3-storage")]
use std::time::Duration;

#[cfg(feature = "notifications")]
use super::NotificationConfig;
use super::{
    AgentDefinition, AgentTimeoutsConfig, ArsenalConfig, CitadelConfig, FileStorageConfig,
    GarrisonSettings, HeraldConfig, LlmConfig, MemoryExtractionConfig, MessageServiceSettings,
    QueueConfig, RagConfig, SanctumAdapterType, SanctumConfig, SchedulerConfig, ServerConfig,
    SourceConfig, VisionConfig, WebHttpConfig, WebServerConfig,
};

/// Top-level application configuration struct.
///
/// Load from YAML/TOML config files and environment variable overrides via
/// [`Settings::new`] or [`Settings::load_from_file`].
///
/// `Debug` prints `llm_api_key` as `<redacted>` (DF-07); the nested `ApiKeyConfig`,
/// `QueueConfig` and `FileStorageConfig` redact their own secrets. URL-shaped fields are printed
/// as written (an operator-embedded userinfo would show), so prefer a summary over `{:?}` in logs.
#[allow(missing_docs)]
#[derive(Serialize, Deserialize, Clone)]
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
    /// Execution timeout policy for the HTTP service host (`default`/`max` seconds).
    #[serde(default)]
    pub timeouts: Option<AgentTimeoutsConfig>,
    /// Cross-cutting HTTP layer config for the service host (CORS, body limit, rate limit).
    #[serde(default)]
    pub http: Option<WebHttpConfig>,
    /// Agent runtime execution middleware configuration (X-09, D-10). Every
    /// section defaults to inert, so an absent `agent_runtime:` key
    /// resolves to [`AgentRuntimeConfig::default()`] and boots with
    /// identical behavior to a v0.9 configuration.
    #[serde(default)]
    pub agent_runtime: AgentRuntimeConfig,
    /// Runtime trace pipeline configuration (X-09, D-36). Every field
    /// defaults to today's behavior, so an absent `trace:` key resolves to
    /// [`TraceConfig::default()`] and boots identically to a v0.9
    /// configuration.
    #[serde(default)]
    pub trace: TraceConfig,
    /// Web-server-scoped configuration for concerns outside core HTTP
    /// bind/port settings -- currently the developer inspector UI's
    /// Mermaid source (D-26, D-36, X-09). An absent `web_server:` key
    /// resolves to [`WebServerConfig::default()`].
    #[serde(default)]
    pub web_server: WebServerConfig,
    /// Operator-configured per-model price table (PRICE-01, D-07). Every field defaults to
    /// inert (empty table, `"USD"`), so an absent `treasurer:` key resolves to
    /// [`TreasurerConfig::default()`] and boots with identical behavior to a config file with
    /// no `treasurer:` section.
    #[serde(default)]
    pub treasurer: TreasurerConfig,
}

// Hand-written so `llm_api_key` is redacted (DF-07). One `.field` per struct field, in
// declaration order; `settings_debug_names_every_serialized_field` fails if a serialised field
// is ever missing here.
impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Settings");
        s.field("llm_type", &self.llm_type);
        s.field("llm_url", &self.llm_url);
        s.field("llm_api_key", &RedactedSecret(&self.llm_api_key));
        s.field("server", &self.server);
        s.field("sources", &self.sources);
        s.field("max_file_size", &self.max_file_size);
        s.field("message_service", &self.message_service);
        s.field("queue", &self.queue);
        s.field("file_storage", &self.file_storage);
        #[cfg(feature = "notifications")]
        s.field("notifications", &self.notifications);
        s.field("garrison", &self.garrison);
        s.field("sanctum", &self.sanctum);
        s.field("rag", &self.rag);
        s.field("memory_extraction", &self.memory_extraction);
        s.field("arsenal", &self.arsenal);
        s.field("citadel", &self.citadel);
        s.field("llm", &self.llm);
        s.field("herald", &self.herald);
        s.field("vision", &self.vision);
        s.field("scheduler", &self.scheduler);
        s.field("agents", &self.agents);
        s.field("timeouts", &self.timeouts);
        s.field("http", &self.http);
        s.field("agent_runtime", &self.agent_runtime);
        s.field("trace", &self.trace);
        s.field("web_server", &self.web_server);
        s.field("treasurer", &self.treasurer);
        s.finish()
    }
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

    /// Validates this configuration's own cross-cutting invariants.
    ///
    /// Validates [`AgentRuntimeConfig`], [`TraceConfig`], [`WebServerConfig`] (X-09) and
    /// [`TreasurerConfig`] (PRICE-01, applying `APP_TREASURER_CURRENCY` first so an invalid
    /// env override is caught here too); other domain configs perform their own validation
    /// through their individual `get_*_config()` accessors.
    pub fn validate(&self) -> Result<(), String> {
        self.agent_runtime.validate()?;
        self.trace.validate()?;
        self.web_server.validate()?;
        self.get_treasurer_config().validate()?;
        Ok(())
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

    /// Get treasurer (pricing) configuration with environment variable overrides.
    pub fn get_treasurer_config(&self) -> TreasurerConfig {
        let mut cfg = self.treasurer.clone();
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
        #[cfg(feature = "cli")]
        use crate::infrastructure::adapters::herald::TableHerald;
        use crate::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald};
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
            #[cfg(feature = "cli")]
            "table" => {
                let table_config =
                    crate::infrastructure::adapters::herald::table_herald::TableHeraldConfig {
                        max_column_width: config.table.max_column_width,
                        border_style: config.table.border_style.clone(),
                    };
                let herald = TableHerald::new(table_config);
                Ok(Arc::new(herald))
            }
            other => {
                #[cfg(feature = "cli")]
                let valid_options = "json, markdown, table";
                #[cfg(not(feature = "cli"))]
                let valid_options = "json, markdown";
                Err(format!(
                    "Unknown formatter '{}'. Valid options: {}",
                    other, valid_options
                ))
            }
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
            timeouts: Some(AgentTimeoutsConfig::default()),
            http: Some(WebHttpConfig::default()),
            agent_runtime: AgentRuntimeConfig::default(),
            trace: TraceConfig::default(),
            web_server: WebServerConfig::default(),
            treasurer: TreasurerConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ApiKeyConfig, WebHttpConfig};
    use paladin_core::platform::container::user::UserRole;

    const LLM_KEY: &str = "test-key-not-a-secret-df07-llm-api-key";
    const API_KEY: &str = "test-key-not-a-secret-df07-api-key";
    const REDIS_PASSWORD: &str = "test-key-not-a-secret-df07-redis-password";
    const MINIO_ACCESS: &str = "test-key-not-a-secret-df07-minio-access-key";
    const MINIO_SECRET: &str = "test-key-not-a-secret-df07-minio-secret-key";

    #[test]
    fn settings_debug_redacts_every_credential_it_reaches() {
        let mut http = WebHttpConfig::default();
        http.auth.api_keys.push(ApiKeyConfig {
            key: API_KEY.to_string(),
            name: "df07-probe".to_string(),
            role: UserRole::User,
            tenant: "Acme-Prod".to_string(),
        });
        let settings = Settings {
            llm_api_key: LLM_KEY.to_string(),
            http: Some(http),
            queue: Some(QueueConfig {
                redis_password: Some(REDIS_PASSWORD.to_string()),
                ..QueueConfig::default()
            }),
            file_storage: Some(FileStorageConfig {
                minio_access_key: MINIO_ACCESS.to_string(),
                minio_secret_key: MINIO_SECRET.to_string(),
                ..FileStorageConfig::default()
            }),
            ..Settings::default()
        };

        // Non-vacuous: every sentinel really sits in the config we format.
        assert_eq!(settings.llm_api_key, LLM_KEY);
        assert_eq!(
            settings
                .http
                .as_ref()
                .map(|h| h.auth.api_keys[0].key.as_str()),
            Some(API_KEY)
        );
        assert_eq!(
            settings
                .queue
                .as_ref()
                .and_then(|q| q.redis_password.as_deref()),
            Some(REDIS_PASSWORD)
        );
        assert_eq!(
            settings
                .file_storage
                .as_ref()
                .map(|f| f.minio_access_key.as_str()),
            Some(MINIO_ACCESS)
        );
        assert_eq!(
            settings
                .file_storage
                .as_ref()
                .map(|f| f.minio_secret_key.as_str()),
            Some(MINIO_SECRET)
        );

        let text = format!("{settings:?}");
        // Name the leaking field, never the ~6 KB rendered text.
        for (field, value) in [
            ("llm_api_key", LLM_KEY),
            ("http.auth.api_keys[].key", API_KEY),
            ("queue.redis_password", REDIS_PASSWORD),
            ("file_storage.minio_access_key", MINIO_ACCESS),
            ("file_storage.minio_secret_key", MINIO_SECRET),
        ] {
            assert!(!text.contains(value), "Settings Debug leaked {field}");
        }
        assert!(
            text.contains("llm_api_key: <redacted>"),
            "llm_api_key marker missing"
        );
    }

    #[test]
    fn settings_debug_names_every_serialized_field() {
        let value = serde_json::to_value(Settings::default()).expect("Settings serialises");
        let object = value.as_object().expect("Settings serialises as an object");
        assert!(!object.is_empty(), "no serialised fields to compare");
        // Pretty Debug puts each top-level field on its own line at four spaces of
        // indent, so nested field names cannot satisfy this by accident.
        let text = format!("{:#?}", Settings::default());
        for key in object.keys() {
            assert!(
                text.contains(&format!("\n    {key}: ")),
                "Settings Debug does not name the serialised field `{key}`"
            );
        }
    }
}
