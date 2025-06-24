use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::application_settings::Settings;
use crate::infrastructure::repositories::sqlite_content_repository::SqliteStore;
use crate::application::storage::sql_store::MigrationManager;
use crate::core::platform::manager::scheduler::Scheduler;
use crate::core::base::service::message_service::{MessageService, MessageServiceConfig};
use crate::core::platform::manager::event_manager::EventService;
use tokio::task::JoinHandle;
use tokio::signal;
use std::env;

pub struct ServiceRunner {
    scheduler: Arc<RwLock<Scheduler>>,
    scheduler_handle: Option<JoinHandle<()>>,
    message_service: Option<Arc<MessageService>>,
    event_service: Option<Arc<EventService>>,
    database: Option<SqliteStore>,
}

impl ServiceRunner {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(Scheduler::new())),
            scheduler_handle: None,
            message_service: None,
            event_service: None,
            database: None,
        }
    }

    pub async fn run_services(&mut self, config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting services...");

        // Initialize the database
        self.database = Some(Self::init_database().await?);
        println!("Database initialized successfully");

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
        self.event_service = Some(event_service);

        // Start the scheduler
        let scheduler_clone = Arc::clone(&self.scheduler);
        self.scheduler_handle = Some(tokio::spawn(async move {
            let mut scheduler = scheduler_clone.write().await;
            scheduler.start().await;
        }));

        println!("Scheduler started successfully");

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        Ok(())
    }

    async fn init_database() -> Result<SqliteStore, Box<dyn std::error::Error>> {
        let db = SqliteStore::new("sqlite:database.db").await?;
        db.migrate().await?;
        Ok(db)
    }

    fn create_message_service_config(config: &Settings) -> MessageServiceConfig {
        // Helper function to get value from config, env, or default for usize
        fn get_usize_config_value(
            config_value: Option<usize>,
            env_key: &str,
            default_value: usize,
        ) -> usize {
            // First try config value
            if let Some(value) = config_value {
                return value;
            }
            
            // Then try environment variable
            if let Ok(env_value) = env::var(env_key) {
                if let Ok(parsed) = env_value.parse::<usize>() {
                    return parsed;
                }
            }
            
            // Finally use default
            default_value
        }

        // Helper function to get value from config, env, or default for u32
        fn get_u32_config_value(
            config_value: Option<u32>,
            env_key: &str,
            default_value: u32,
        ) -> u32 {
            // First try config value
            if let Some(value) = config_value {
                return value;
            }
            
            // Then try environment variable
            if let Ok(env_value) = env::var(env_key) {
                if let Ok(parsed) = env_value.parse::<u32>() {
                    return parsed;
                }
            }
            
            // Finally use default
            default_value
        }

        // Helper function to get value from config, env, or default for u64
        fn get_i64_config_value(
            config_value: Option<i64>,
            env_key: &str,
            default_value: i64,
        ) -> i64 {
            // First try config value
            if let Some(value) = config_value {
                return value;
            }
            
            // Then try environment variable
            if let Ok(env_value) = env::var(env_key) {
                if let Ok(parsed) = env_value.parse::<i64>() {
                    return parsed;
                }
            }
            
            // Finally use default
            default_value
        }
        
                // Helper function to get value from config, env, or default for u64
        fn get_u64_config_value(
            config_value: Option<u64>,
            env_key: &str,
            default_value: u64,
        ) -> u64 {
            // First try config value
            if let Some(value) = config_value {
                return value;
            }
            
            // Then try environment variable
            if let Ok(env_value) = env::var(env_key) {
                if let Ok(parsed) = env_value.parse::<u64>() {
                    return parsed;
                }
            }
            
            // Finally use default
            default_value
        }

        // Helper for boolean values
        fn get_bool_config_value(
            config_value: Option<bool>,
            env_key: &str,
            default_value: bool
        ) -> bool {
            // First try config value
            if let Some(value) = config_value {
                return value;
            }
            
            // Then try environment variable
            if let Ok(env_value) = env::var(env_key) {
                match env_value.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => return true,
                    "false" | "0" | "no" | "off" => return false,
                    _ => {}
                }
            }
            
            // Finally use default
            default_value
        }

        // Extract message service configuration from Settings if available
        let message_config = config.message_service.as_ref();

        MessageServiceConfig {
            max_queue_size: get_usize_config_value(
                message_config.and_then(|c| c.max_queue_size),
                "MESSAGE_SERVICE_MAX_QUEUE_SIZE",
                10000,
            ),
            default_ttl_seconds: get_i64_config_value(
                message_config.and_then(|c| c.default_ttl_seconds),
                "MESSAGE_SERVICE_DEFAULT_TTL_SECONDS",
                3600, // 1 hour
            ),
            enable_persistence: get_bool_config_value(
                message_config.and_then(|c| c.enable_persistence),
                "MESSAGE_SERVICE_ENABLE_PERSISTENCE",
                false
            ),
            worker_threads: get_usize_config_value(
                message_config.and_then(|c| c.worker_threads),
                "MESSAGE_SERVICE_WORKER_THREADS",
                4,
            ),
            retry_attempts: get_u32_config_value(
                message_config.and_then(|c| c.retry_attempts),
                "MESSAGE_SERVICE_RETRY_ATTEMPTS",
                3,
            ),
            retry_delay_ms: get_u64_config_value(
                message_config.and_then(|c| c.retry_delay_ms),
                "MESSAGE_SERVICE_RETRY_DELAY_MS",
                1000,
            ),
        }
    }

    async fn wait_for_shutdown(&mut self) {
        println!("Services running. Press Ctrl+C to shutdown...");
        
        // Wait for Ctrl+C
        match signal::ctrl_c().await {
            Ok(()) => {
                println!("Received shutdown signal, stopping services...");
                self.shutdown().await;
            },
            Err(err) => {
                eprintln!("Unable to listen for shutdown signal: {}", err);
            },
        }
    }

    pub async fn shutdown(&mut self) {
        println!("Shutting down services...");

        // Stop the scheduler
        {
            let mut scheduler = self.scheduler.write().await;
            scheduler.stop();
        }

        // Wait for scheduler task to complete
        if let Some(handle) = self.scheduler_handle.take() {
            let _ = handle.await;
        }

        // Stop message service
        if let Some(message_service) = &self.message_service {
            if let Err(e) = message_service.stop().await {
                eprintln!("Error stopping message service: {}", e);
            } else {
                println!("Message service stopped");
            }
        }

        // Event service doesn't have explicit stop method as it relies on message service

        // Close database connection
        self.database = None;

        // Clear service references
        self.message_service = None;
        self.event_service = None;

        println!("All services stopped");
    }

    pub async fn get_scheduler_status(&self) -> Vec<SchedulerJobStatus> {
        let scheduler = self.scheduler.read().await;
        scheduler.list_jobs().into_iter()
            .map(|job_info| SchedulerJobStatus {
                id: job_info.id,
                name: job_info.name,
                enabled: job_info.enabled,
                next_run: job_info.next_run,
                last_run: job_info.last_run,
                run_count: job_info.run_count,
                task_count: job_info.task_count,
                status: job_info.status,
            })
            .collect()
    }

    pub fn get_database(&self) -> Option<&SqliteStore> {
        self.database.as_ref()
    }

    pub fn get_message_service(&self) -> Option<Arc<MessageService>> {
        self.message_service.clone()
    }

    pub fn get_event_service(&self) -> Option<Arc<EventService>> {
        self.event_service.clone()
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

        health
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
    }

    #[tokio::test]
    async fn test_message_service_config_creation() {
        let settings = Settings::default();
        let config = ServiceRunner::create_message_service_config(&settings);
        assert_eq!(config.max_queue_size, 10000);
        assert_eq!(config.worker_threads, 4);
    }

    #[tokio::test]
    async fn test_message_service_config_with_env_vars() {
        // Set environment variables using unsafe blocks
        unsafe {
            env::set_var("MESSAGE_SERVICE_MAX_QUEUE_SIZE", "5000");
            env::set_var("MESSAGE_SERVICE_WORKER_THREADS", "8");
            env::set_var("MESSAGE_SERVICE_ENABLE_PERSISTENCE", "true");
        }
        
        let mut settings = Settings::default();
        settings.message_service = None;
        let config = ServiceRunner::create_message_service_config(&settings);
        
        // Should use environment variables
        assert_eq!(config.max_queue_size, 5000);
        assert_eq!(config.worker_threads, 8);
        assert_eq!(config.enable_persistence, true);
        
        // Clean up
        unsafe {
            env::remove_var("MESSAGE_SERVICE_MAX_QUEUE_SIZE");
            env::remove_var("MESSAGE_SERVICE_WORKER_THREADS");
            env::remove_var("MESSAGE_SERVICE_ENABLE_PERSISTENCE");
        }
    }

    #[tokio::test]
    async fn test_service_health_status() {
        let runner = ServiceRunner::new();
        let health = runner.get_service_health().await;
        
        assert!(!health.database_connected);
        assert!(!health.message_service_healthy);
        assert!(!health.event_service_initialized);
        assert_eq!(health.total_jobs, 0);
        assert_eq!(health.enabled_jobs, 0);
    }

    #[test]
    fn test_config_precedence() {
        // Test that config values take precedence over env vars
        unsafe {
            env::set_var("MESSAGE_SERVICE_MAX_QUEUE_SIZE", "9999");
        }
        
        // Create a settings with message service config
        let settings = Settings::default();
        // Note: You'll need to adjust this based on your actual Settings structure
        // This is just an example of how config values should take precedence
        
        let config = ServiceRunner::create_message_service_config(&settings);
        
        // If Settings had a value, it should take precedence over env var
        // Since Settings::default() provides Some values, config should use those
        assert_eq!(config.max_queue_size, 10000); // Should use config value, not env var
        
        unsafe {
            env::remove_var("MESSAGE_SERVICE_MAX_QUEUE_SIZE");
        }
    }
}