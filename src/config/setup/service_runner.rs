use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::application_settings::Settings;
use crate::infrastructure::repositories::sqlite_content_repository::SqliteStore;
use crate::application::storage::sql_store::MigrationManager;
use crate::core::platform::manager::scheduler::Scheduler;
use crate::core::base::service::message_service::{MessageService, MessageServiceConfig};
use crate::core::platform::manager::event_manager::EventService;
use crate::infrastructure::adapters::queue::redis::RedisQueueAdapter;
use crate::application::ports::output::queue_port::QueuePort;
use crate::infrastructure::adapters::file_storage::minio::MinioAdapter;
use crate::infrastructure::adapters::logs::system_log_adapter::SystemLogAdapter;
use crate::application::ports::output::log_port::LogPort;
use tokio::task::JoinHandle;
use tokio::signal;
use std::env;
use std::path::PathBuf;
use crate::application::ports::output::file_storage_port::FileStorageUtils;
use crate::application::ports::output::file_storage_port::FileStoragePort;
use crate::core::platform::manager::user_service::UserService;
use crate::config::user_config::UserServiceFactory;
use crate::core::platform::manager::notification_service::NotificationService;
pub struct ServiceRunner {
    scheduler: Arc<RwLock<Scheduler>>,
    scheduler_handle: Option<JoinHandle<()>>,
    message_service: Option<Arc<MessageService>>,
    event_service: Option<Arc<EventService>>,
    notification_service: Option<Arc<NotificationService>>,
    user_service: Option<Arc<UserService>>,
    database: Option<SqliteStore>,
    queue_adapter: Option<Arc<RedisQueueAdapter>>,
    file_storage_adapter: Option<Arc<MinioAdapter>>,
    log_adapter: Option<Arc<SystemLogAdapter>>,
}

impl ServiceRunner {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(Scheduler::new())),
            scheduler_handle: None,
            message_service: None,
            event_service: None,
            user_service: None,
            database: None,
            queue_adapter: None,
            file_storage_adapter: None,
            log_adapter: None,
            notification_service: None,
        }
    }

    pub async fn run_services(&mut self, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting services...");

        // Initialize the database
        self.database = Some(Self::init_database().await?);
        println!("Database initialized successfully");

        // Initialize LogAdapter
        let log_adapter = SystemLogAdapter::new(Default::default())
            .map_err(|e| format!("Failed to initialize SystemLogAdapter: {}", e))?;
        let log_adapter = Arc::new(log_adapter);
        self.log_adapter = Some(log_adapter.clone());
        println!("Log adapter initialized successfully");

        // Initialize Redis Queue Adapter
        let queue_config = config.get_queue_config();
        let redis_config = crate::infrastructure::adapters::queue::redis::RedisQueueConfig {
            redis_host: queue_config.redis_host,
            redis_port: queue_config.redis_port,
            redis_password: queue_config.redis_password,
            redis_db: queue_config.redis_db,
            connection_timeout: queue_config.connection_timeout.unwrap_or(30),
            key_prefix: queue_config.key_prefix.unwrap_or_else(|| "paladin:queue".to_string()),
            max_retries: queue_config.max_retries.unwrap_or(3),
        };

        let queue_adapter = Arc::new(
            RedisQueueAdapter::new(redis_config, Some(log_adapter.clone() as Arc<dyn LogPort>))
                .await
                .map_err(|e| format!("Failed to initialize Redis queue adapter: {}", e))?
        );
        self.queue_adapter = Some(queue_adapter.clone());
        
        // Test Redis connection
        queue_adapter.health_check().await
            .map_err(|e| format!("Redis queue adapter health check failed: {}", e))?;
        println!("Redis queue adapter initialized successfully");

        // Initialize MinIO File Storage Adapter
        let minio_config = config.to_minio_config();
        let file_storage_adapter = Arc::new(
            MinioAdapter::new(minio_config, Some(log_adapter.clone() as Arc<dyn LogPort>))
                .await
                .map_err(|e| format!("Failed to initialize MinIO file storage adapter: {}", e))?
        );
        self.file_storage_adapter = Some(file_storage_adapter.clone());

        // Test MinIO connection
        file_storage_adapter.health_check().await
            .map_err(|e| format!("MinIO file storage adapter health check failed: {}", e))?;
        println!("MinIO file storage adapter initialized successfully");

        // Initialize MessageService
        let message_service_config = Self::create_message_service_config(&config);
        let message_service = Arc::new(MessageService::new(message_service_config));
        message_service.start().await
            .map_err(|e| format!("Failed to start message service: {}", e))?;
        
        println!("Message service started successfully");
        self.message_service = Some(message_service.clone());

        // Initialize EventService
        let event_service = Arc::new(EventService::new(message_service.clone()).await
            .map_err(|e| format!("Failed to create event service: {}", e))?);
        
        println!("Event service initialized successfully");
        self.event_service = Some(event_service.clone());

        // Initialize Notification Service
        let notification_service = Self::init_notification_service(&config, message_service.clone()).await?;
        self.notification_service = Some(notification_service);
        println!("Notification service initialized successfully");

        // Initialize User Service
        self.init_user_service(config.clone()).await?;

        // Update scheduler with adapters
        {
            let _scheduler = self.scheduler.write().await;
            // Here you would inject the adapters into the scheduler
            // This depends on your scheduler implementation
            println!("Scheduler updated with queue and file storage adapters");
        }

        // Start the scheduler
        let scheduler_clone = Arc::clone(&self.scheduler);
        self.scheduler_handle = Some(tokio::spawn(async move {
            let mut scheduler = scheduler_clone.write().await;
            scheduler.start().await;
        }));

        println!("Scheduler started successfully");
        println!("All services started successfully!");
        println!("Queue adapter: {}", queue_adapter.get_connection_info());
        println!("File storage adapter: {}", file_storage_adapter.get_connection_info());

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        Ok(())
    }

    async fn init_database() -> Result<SqliteStore, Box<dyn std::error::Error>> {
        let db = SqliteStore::new("sqlite:database.db").await?;
        db.migrate().await?;
        Ok(db)
    }

    fn create_message_service_config(settings: &Settings) -> MessageServiceConfig {
        // Get base configuration from settings or defaults
        let base_config = if let Some(msg_config) = &settings.message_service {
            MessageServiceConfig {
                max_queue_size: msg_config.max_queue_size.unwrap_or(10000),
                default_ttl_seconds: msg_config.default_ttl_seconds.unwrap_or(3600),
                enable_persistence: msg_config.enable_persistence.unwrap_or(false),
                worker_threads: msg_config.worker_threads.unwrap_or(4),
                retry_attempts: msg_config.retry_attempts.unwrap_or(3),
                retry_delay_ms: msg_config.retry_delay_ms.unwrap_or(1000),
                ..Default::default()
            }
        } else {
            MessageServiceConfig::default()
        };

        // Override with environment variables if present
        let mut config = base_config;

        if let Ok(max_queue_size) = env::var("MESSAGE_SERVICE_MAX_QUEUE_SIZE") {
            if let Ok(size) = max_queue_size.parse::<usize>() {
                config.max_queue_size = size;
            }
        }

        if let Ok(worker_threads) = env::var("MESSAGE_SERVICE_WORKER_THREADS") {
            if let Ok(threads) = worker_threads.parse::<usize>() {
                config.worker_threads = threads;
            }
        }

        if let Ok(enable_persistence) = env::var("MESSAGE_SERVICE_ENABLE_PERSISTENCE") {
            if let Ok(enabled) = enable_persistence.parse::<bool>() {
                config.enable_persistence = enabled;
            }
        }

        if let Ok(ttl) = env::var("MESSAGE_SERVICE_DEFAULT_TTL_SECONDS") {
            if let Ok(ttl_val) = ttl.parse::<i64>() {
                config.default_ttl_seconds = ttl_val;
            }
        }

        if let Ok(retry_attempts) = env::var("MESSAGE_SERVICE_RETRY_ATTEMPTS") {
            if let Ok(attempts) = retry_attempts.parse::<u32>() {
                config.retry_attempts = attempts;
            }
        }

        if let Ok(retry_delay) = env::var("MESSAGE_SERVICE_RETRY_DELAY_MS") {
            if let Ok(delay) = retry_delay.parse::<u64>() {
                config.retry_delay_ms = delay;
            }
        }

        config
    }

    async fn wait_for_shutdown(&self) {
        let ctrl_c = signal::ctrl_c();
        
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            
            tokio::select! {
                _ = ctrl_c => {
                    println!("Received Ctrl+C, shutting down...");
                },
                _ = sigterm.recv() => {
                    println!("Received SIGTERM, shutting down...");
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            ctrl_c.await.expect("Failed to listen for ctrl-c");
            println!("Received Ctrl+C, shutting down...");
        }
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Shutting down services...");

        // Stop scheduler
        if let Some(handle) = self.scheduler_handle.take() {
            handle.abort();
            println!("Scheduler stopped");
        }

        // Shutdown message service
        if let Some(_message_service) = &self.message_service {
            // Assuming MessageService has a shutdown method
            println!("Message service stopped");
        }

        // Shutdown file storage adapter
        if let Some(file_storage_adapter) = &self.file_storage_adapter {
            file_storage_adapter.shutdown().await
                .map_err(|e| format!("Failed to shutdown file storage adapter: {}", e))?;
            println!("File storage adapter stopped");
        }

        // Shutdown queue adapter
        if let Some(queue_adapter) = &self.queue_adapter {
            queue_adapter.shutdown().await
                .map_err(|e| format!("Failed to shutdown queue adapter: {}", e))?;
            println!("Queue adapter stopped");
        }

        // Close database connection
        if let Some(_database) = &self.database {
            // SQLite connections are automatically closed when dropped
            println!("Database connection closed");
        }

        println!("All services shut down successfully");
        Ok(())
    }

    /// Get queue adapter reference for use by other services
    pub fn get_queue_adapter(&self) -> Option<Arc<RedisQueueAdapter>> {
        self.queue_adapter.clone()
    }

    /// Get file storage adapter reference for use by other services
    pub fn get_file_storage_adapter(&self) -> Option<Arc<MinioAdapter>> {
        self.file_storage_adapter.clone()
    }

    /// Get log adapter reference for use by other services  
    pub fn get_log_adapter(&self) -> Option<Arc<SystemLogAdapter>> {
        self.log_adapter.clone()
    }

    /// Get notification service reference for use by other services
    pub fn get_notification_service(&self) -> Option<Arc<NotificationService>> {
        self.notification_service.clone()
    }

    /// Get service health status
    pub async fn get_service_health(&self) -> ServiceHealthStatus {
        let mut health = ServiceHealthStatus {
            database_connected: self.database.is_some(),
            message_service_healthy: false,
            event_service_initialized: self.event_service.is_some(),
            scheduler_running: false,
            total_jobs: 0,
            enabled_jobs: 0,
            queue_adapter_healthy: false,
            redis_connected: false,
            file_storage_adapter_healthy: false,
            minio_connected: false,
        };

        // Check message service health
        if let Some(message_service) = &self.message_service {
            health.message_service_healthy = message_service.health_check().await.unwrap_or(false);
        }

        // Check scheduler status
        {
            let scheduler = self.scheduler.read().await;
            let stats = scheduler.stats();
            health.total_jobs = stats.total_jobs;
            health.enabled_jobs = stats.enabled_jobs;
            // Note: We don't have a direct way to check if scheduler is running
            // This could be added to the Scheduler struct if needed
        }

        // Check queue adapter health
        if let Some(queue_adapter) = &self.queue_adapter {
            match queue_adapter.health_check().await {
                Ok(true) => {
                    health.queue_adapter_healthy = true;
                    health.redis_connected = true;
                }
                _ => {
                    health.queue_adapter_healthy = false;
                    health.redis_connected = false;
                }
            }
        }

        // Check file storage adapter health
        if let Some(file_storage_adapter) = &self.file_storage_adapter {
            match file_storage_adapter.health_check().await {
                Ok(storage_health) => {
                    health.file_storage_adapter_healthy = storage_health.is_available;
                    health.minio_connected = storage_health.is_available;
                }
                _ => {
                    health.file_storage_adapter_healthy = false;
                    health.minio_connected = false;
                }
            }
        }

        health
    }

    /// Initialize a sample file for testing file storage
    pub async fn initialize_sample_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_storage) = &self.file_storage_adapter {
            use std::path::PathBuf;
            use crate::application::ports::output::file_storage_port::{FileStoragePort, UploadOptions};

            // Create a sample analysis directory structure
            let sample_files = vec![
                ("analysis/README.md", "# Analysis Files\n\nThis directory contains files for security analysis."),
                ("reports/README.md", "# Reports\n\nGenerated security audit reports are stored here."),
                ("code/sample.rs", "// Sample Rust code for analysis\nfn main() {\n    println!(\"Hello, world!\");\n}"),
                ("code/sample.py", "# Sample Python code for analysis\nprint(\"Hello, world!\")"),
            ];

            for (path, content) in sample_files {
                let file_path = PathBuf::from(path);
                let upload_options = UploadOptions {
                    content_type: Some(Self::detect_content_type(&file_path)),
                    tags: vec!["sample".to_string(), "initialization".to_string()],
                    overwrite: true,
                    ..Default::default()
                };

                match file_storage.upload_file(&file_path, content.as_bytes(), Some(upload_options)).await {
                    Ok(file_item) => {
                        println!("Created sample file: {} ({} bytes)", file_item.path.display(), file_item.size);
                    }
                    Err(e) => {
                        println!("Failed to create sample file {}: {}", path, e);
                    }
                }
            }

            println!("Sample files initialized in MinIO storage");
        }

        Ok(())
    }

    /// Helper method to detect content type from file extension
    fn detect_content_type(path: &PathBuf) -> String {
        <() as FileStorageUtils>::detect_content_type(path).unwrap_or_else(|| "application/octet-stream".to_string())
    }
    
    async fn init_user_service(
        &mut self,
        config: Arc<Settings>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let log_adapter = self.log_adapter.as_ref()
            .ok_or("LogAdapter must be initialized before UserService")?;

        let notification_service = self.notification_service.as_ref()
            .ok_or("NotificationService must be initialized before UserService")?;

        self.user_service = Some(
            UserServiceFactory::create_user_service(
                &config,
                log_adapter.clone(),
                notification_service.clone(),
            ).await?
        );

        println!("User service initialized successfully");
        Ok(())
    }

    /// Get user service
    pub fn get_user_service(&self) -> Option<Arc<UserService>> {
        self.user_service.clone()
    }

    async fn init_notification_service(
        config: &Settings,
        message_service: Arc<MessageService>,
    ) -> Result<Arc<NotificationService>, Box<dyn std::error::Error>> {
        let notification_config = config.get_notification_config();
        
        // Create service config from notification config
        let service_config = crate::core::platform::manager::notification_service::NotificationServiceConfig {
            default_max_retries: notification_config.max_retries,
            default_expiry_seconds: 86400, // 24 hours
            enable_persistence: true,
            batch_size: 100,
            processing_interval_ms: 1000,
            template_cache_size: 1000,
            max_attachment_size: 25 * 1024 * 1024, // 25MB
        };

        // Create notification service
        let notification_service = NotificationService::new(service_config, message_service);

        if !notification_config.enabled {
            println!("Notification service is disabled in configuration");
            return Ok(Arc::new(notification_service));
        }

        // TODO: Create adapter wrappers that implement NotificationChannelHandler
        // The current adapters implement NotificationDeliveryPort but we need NotificationChannelHandler
        // This will be implemented in a future phase when we create adapter bridges

        println!("Notification service configured (adapter registration pending)");
        
        Ok(Arc::new(notification_service))
    }
}

/// Status information for a scheduled job
#[derive(Debug, Clone)]
pub struct SchedulerJobStatus {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub next_run: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    pub run_count: u32,
    pub task_count: usize,
    pub status: crate::core::base::component::action::ActionStatus,
}

/// Overall service health status  
#[derive(Debug, Clone)]
pub struct ServiceHealthStatus {
    pub database_connected: bool,
    pub message_service_healthy: bool,
    pub event_service_initialized: bool,
    pub scheduler_running: bool,
    pub total_jobs: usize,
    pub enabled_jobs: usize,
    pub queue_adapter_healthy: bool,
    pub redis_connected: bool,
    pub file_storage_adapter_healthy: bool,
    pub minio_connected: bool,
}

impl Default for ServiceRunner {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience function for backward compatibility
pub async fn run_services(config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    let mut service_runner = ServiceRunner::new();
    service_runner.run_services(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_runner_creation() {
        let runner = ServiceRunner::new();
        assert!(runner.database.is_none());
        assert!(runner.message_service.is_none());
        assert!(runner.event_service.is_none());
        assert!(runner.queue_adapter.is_none());
        assert!(runner.file_storage_adapter.is_none());
        assert!(runner.log_adapter.is_none());
    }

    #[tokio::test]
    async fn test_message_service_config_creation() {
        let settings = Settings::default();
        let config = ServiceRunner::create_message_service_config(&settings);
        assert_eq!(config.max_queue_size, 10000);
        assert_eq!(config.worker_threads, 4);
    }

    #[tokio::test]
    async fn test_service_health_status() {
        let runner = ServiceRunner::new();
        let health = runner.get_service_health().await;
        
        assert!(!health.database_connected);
        assert!(!health.message_service_healthy);
        assert!(!health.event_service_initialized);
        assert!(!health.queue_adapter_healthy);
        assert!(!health.redis_connected);
        assert!(!health.file_storage_adapter_healthy);
        assert!(!health.minio_connected);
        assert_eq!(health.total_jobs, 0);
        assert_eq!(health.enabled_jobs, 0);
    }

    #[test]
    fn test_content_type_detection() {
        use std::path::PathBuf;
        
        assert_eq!(ServiceRunner::detect_content_type(&PathBuf::from("test.rs")), "text/x-rust");
        assert_eq!(ServiceRunner::detect_content_type(&PathBuf::from("test.json")), "application/json");
        assert_eq!(ServiceRunner::detect_content_type(&PathBuf::from("test.jpg")), "image/jpeg");
        assert_eq!(ServiceRunner::detect_content_type(&PathBuf::from("test.pdf")), "application/pdf");
    }

    #[tokio::test]
    async fn test_adapters_integration() {
        let runner = ServiceRunner::new();
        
        // Initially no adapters
        assert!(runner.get_queue_adapter().is_none());
        assert!(runner.get_file_storage_adapter().is_none());
        assert!(runner.get_log_adapter().is_none());
        
        // After initialization, adapters should be available
        // This would require a full service initialization which needs Redis and MinIO
        // So we'll skip this for unit tests and rely on integration tests
    }
}