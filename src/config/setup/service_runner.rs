use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::application_settings::Settings;
use crate::infrastructure::repositories::sqlite_content_repository::SqliteStore;
use crate::application::storage::sql_store::MigrationManager; // Import the trait
use crate::core::platform::manager::scheduler::Scheduler;
use tokio::task::JoinHandle;
use tokio::signal;

pub struct ServiceRunner {
    scheduler: Arc<RwLock<Scheduler>>,
    scheduler_handle: Option<JoinHandle<()>>,
    database: Option<SqliteStore>,
}

impl ServiceRunner {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(Scheduler::new())),
            scheduler_handle: None,
            database: None,
        }
    }

    pub async fn run_services(&mut self, _config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting services...");

        // Initialize the database
        self.database = Some(Self::init_database().await?);
        println!("Database initialized successfully");

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
        db.migrate()?; // Now this will work because MigrationManager is imported
        Ok(db)
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

        // Close database connection
        self.database = None;

        println!("All services stopped");
    }

    pub async fn get_scheduler_status(&self) -> Vec<(uuid::Uuid, String, bool, Option<chrono::DateTime<chrono::Utc>>)> {
        let scheduler = self.scheduler.read().await;
        scheduler.list_jobs().into_iter()
            .map(|(id, name, enabled, next_run)| (id, name.to_string(), enabled, next_run))
            .collect()
    }

    pub fn get_database(&self) -> Option<&SqliteStore> {
        self.database.as_ref()
    }
}

// Convenience function for backward compatibility
pub async fn run_services(config: Arc<Settings>) -> Result<(), Box<dyn std::error::Error>> {
    let mut service_runner = ServiceRunner::new();
    service_runner.run_services(config).await
}