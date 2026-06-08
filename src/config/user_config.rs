/*
User Configuration

Configuration setup for user-related services, including dependency injection
and service initialization.
*/

use crate::application::services::notification_orchestrator::NotificationService;
use crate::config::Settings;
use crate::core::platform::manager::user_service::UserService;
use crate::infrastructure::repositories::sqlite_user_repository::SqliteUserRepository;
use paladin_ports::output::log_port::LogPort;
use std::sync::Arc;

/// User service configuration and factory
pub struct UserServiceFactory;

impl UserServiceFactory {
    /// Create a new UserService with all dependencies
    pub async fn create_user_service(
        _settings: &Settings,
        log_port: Arc<dyn LogPort>,
        notification_service: Arc<NotificationService>,
    ) -> Result<Arc<UserService>, Box<dyn std::error::Error>> {
        // Create user repository
        let user_repository = Arc::new(SqliteUserRepository::new("sqlite:database.db").await?);

        // Create user service with dependencies
        let user_service = Arc::new(UserService::new(
            user_repository,
            log_port,
            notification_service,
        ));

        Ok(user_service)
    }

    /// Create a new UserService with custom database URL (for testing)
    #[cfg(test)]
    pub async fn create_user_service_with_db_url(
        _settings: &Settings,
        database_url: &str,
        log_port: Arc<dyn LogPort>,
        notification_service: Arc<NotificationService>,
    ) -> Result<Arc<UserService>, Box<dyn std::error::Error>> {
        // Create user repository with custom database URL
        let user_repository = Arc::new(SqliteUserRepository::new_with_url(database_url).await?);

        // Create user service with dependencies
        let user_service = Arc::new(UserService::new(
            user_repository,
            log_port,
            notification_service,
        ));

        Ok(user_service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::base::service::message_service::MessageService;
    use crate::core::platform::container::log::{LogDestination, LogEntry};
    use async_trait::async_trait;
    use chrono::DateTime;
    use paladin_core::platform::container::notification::NotificationServiceConfig;
    use paladin_ports::output::log_port::{
        LogDestinationConfig, LogFormat, LogHealthCheck, LogQuery, LogResult, LogStats,
    };
    use std::sync::Mutex;

    // Mock LogPort for testing
    struct MockLogPort {
        call_count: Mutex<usize>,
        should_fail: Mutex<bool>,
    }

    impl MockLogPort {
        fn new() -> Self {
            Self {
                call_count: Mutex::new(0),
                should_fail: Mutex::new(false),
            }
        }

        // Test mock setter; retained for failure-injection scenarios.
        #[allow(dead_code)]
        fn set_should_fail(&self, fail: bool) {
            *self.should_fail.lock().unwrap() = fail;
        }

        // Test mock accessor; retained for call-count assertions.
        #[allow(dead_code)]
        fn get_call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl LogPort for MockLogPort {
        async fn write_entry(&self, _entry: LogEntry) -> LogResult<()> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;

            if *self.should_fail.lock().unwrap() {
                Err(paladin_ports::output::log_port::LogError::IoError(
                    "Mock log error".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        async fn write_entries(&self, entries: Vec<LogEntry>) -> LogResult<()> {
            let mut count = self.call_count.lock().unwrap();
            *count += entries.len();

            if *self.should_fail.lock().unwrap() {
                Err(paladin_ports::output::log_port::LogError::IoError(
                    "Mock log error".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        async fn batch_write(
            &self,
            _request: paladin_ports::output::log_port::BatchWriteRequest,
        ) -> LogResult<()> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;

            if *self.should_fail.lock().unwrap() {
                Err(paladin_ports::output::log_port::LogError::IoError(
                    "Mock log error".to_string(),
                ))
            } else {
                Ok(())
            }
        }

        async fn read_entries(&self, _query: LogQuery) -> LogResult<Vec<LogEntry>> {
            Ok(vec![])
        }

        async fn count_entries(&self, _query: LogQuery) -> LogResult<u64> {
            Ok(0)
        }

        async fn configure_destination(&self, _config: LogDestinationConfig) -> LogResult<()> {
            Ok(())
        }

        async fn remove_destination(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn list_destinations(&self) -> LogResult<Vec<LogDestination>> {
            Ok(vec![])
        }

        async fn flush(&self) -> LogResult<()> {
            Ok(())
        }

        async fn flush_destination(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn rotate_logs(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn get_stats(&self) -> LogResult<LogStats> {
            Ok(LogStats {
                entries_written: 0,
                entries_by_level: std::collections::HashMap::new(),
                errors: 0,
                bytes_written: 0,
                last_write: None,
            })
        }

        async fn get_destination_stats(&self, _destination: LogDestination) -> LogResult<LogStats> {
            Ok(LogStats {
                entries_written: 0,
                entries_by_level: std::collections::HashMap::new(),
                errors: 0,
                bytes_written: 0,
                last_write: None,
            })
        }

        async fn clear_logs(&self, _destination: LogDestination) -> LogResult<()> {
            Ok(())
        }

        async fn clear_logs_before(
            &self,
            _destination: LogDestination,
            _before: DateTime<chrono::Utc>,
        ) -> LogResult<u64> {
            Ok(0)
        }

        async fn health_check(&self) -> LogResult<Vec<LogHealthCheck>> {
            Ok(vec![])
        }

        async fn health_check_destination(
            &self,
            _destination: LogDestination,
        ) -> LogResult<LogHealthCheck> {
            Ok(LogHealthCheck {
                destination: LogDestination::System,
                healthy: true,
                last_write: Some(chrono::Utc::now()),
                error_message: None,
                response_time: Some(std::time::Duration::from_millis(10)),
            })
        }

        fn get_provider_name(&self) -> &'static str {
            "MockLogPort"
        }

        async fn test_connection(&self) -> LogResult<()> {
            Ok(())
        }

        async fn archive_logs(
            &self,
            _destination: LogDestination,
            _before: DateTime<chrono::Utc>,
        ) -> LogResult<String> {
            Ok("mock_archive_path".to_string())
        }

        fn supported_formats(&self) -> Vec<LogFormat> {
            vec![LogFormat::Json]
        }
    }

    fn create_test_settings() -> Settings {
        Settings {
            llm_type: "openai".to_string(),
            llm_url: "https://api.openai.com/v1".to_string(),
            llm_api_key: "test_key".to_string(),
            server: crate::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            sources: vec![],
            max_file_size: 100 * 1024 * 1024, // 100MB
            message_service: Some(crate::config::MessageServiceSettings {
                max_queue_size: Some(1000),
                default_ttl_seconds: Some(3600),
                enable_persistence: Some(false),
                worker_threads: Some(4),
                retry_attempts: Some(3),
                retry_delay_ms: Some(1000),
            }),
            queue: Some(crate::config::QueueConfig::default()),
            file_storage: Some(crate::config::FileStorageConfig::default()),
            #[cfg(feature = "notifications")]
            notifications: Some(crate::config::NotificationConfig::default()),
            garrison: Some(crate::config::GarrisonSettings::default()),
            sanctum: Some(crate::config::SanctumConfig::default()),
            rag: Some(crate::config::RagConfig::default()),
            memory_extraction: Some(crate::config::MemoryExtractionConfig::default()),
            arsenal: Some(crate::config::ArsenalConfig::default()),
            citadel: Some(crate::config::CitadelConfig::default()),
            llm: Some(crate::config::LlmConfig::default()),
            herald: Some(crate::config::HeraldConfig::default()),
            vision: Some(crate::config::VisionConfig::default()),
            scheduler: Some(crate::config::SchedulerConfig::default()),
            agents: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_create_user_service_success() {
        // Arrange
        let settings = create_test_settings();
        let log_port = Arc::new(MockLogPort::new());

        let message_service = Arc::new(MessageService::new(
            crate::core::base::service::message_service::MessageServiceConfig::default(),
        ));
        let notification_config = NotificationServiceConfig::default();
        let notification_service = Arc::new(NotificationService::new(
            notification_config,
            message_service,
        ));

        // Act
        let result = UserServiceFactory::create_user_service_with_db_url(
            &settings,
            "sqlite::memory:",
            log_port.clone(),
            notification_service,
        )
        .await;

        // Assert
        match result {
            Ok(user_service) => {
                assert!(
                    Arc::strong_count(&user_service) >= 1,
                    "UserService should be properly wrapped in Arc"
                );
            }
            Err(e) => {
                panic!("UserService creation failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_create_user_service_with_invalid_database_url() {
        // Arrange - create settings with invalid SQLite URL
        let settings = create_test_settings();
        let log_port = Arc::new(MockLogPort::new());

        let message_service = Arc::new(MessageService::new(
            crate::core::base::service::message_service::MessageServiceConfig::default(),
        ));
        let notification_config = NotificationServiceConfig::default();
        let notification_service = Arc::new(NotificationService::new(
            notification_config,
            message_service,
        ));

        // Act
        let result = UserServiceFactory::create_user_service_with_db_url(
            &settings,
            "invalid://url",
            log_port.clone(),
            notification_service,
        )
        .await;

        // Assert - this should fail with invalid database URL
        assert!(
            result.is_err(),
            "UserService creation should fail with invalid database URL"
        );
    }

    #[tokio::test]
    async fn test_user_service_factory_structure() {
        // Test that UserServiceFactory can be instantiated
        let _factory = UserServiceFactory;
        // This is mainly a compilation test to ensure the struct is properly defined
    }
}
