use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub service: Box<dyn TaskService>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
}

impl Task {
    pub fn new(name: String, service: Box<dyn TaskService>) -> Task {
        Task {
            id: Uuid::new_v4(),
            name,
            service,
            created_at: Utc::now(),
            last_run: None,
            run_count: 0,
        }
    }

    pub async fn execute(&mut self) -> Result<(), TaskError> {
        println!("Executing task: {}", self.name);
        
        match self.service.execute().await {
            Ok(_) => {
                self.last_run = Some(Utc::now());
                self.run_count += 1;
                println!("Task '{}' completed successfully", self.name);
                Ok(())
            },
            Err(e) => {
                println!("Task '{}' failed: {}", self.name, e);
                Err(e)
            }
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub enum TaskError {
    ExecutionFailed(String),
    ServiceUnavailable,
    Timeout,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::ExecutionFailed(msg) => write!(f, "Task execution failed: {}", msg),
            TaskError::ServiceUnavailable => write!(f, "Service unavailable"),
            TaskError::Timeout => write!(f, "Task execution timed out"),
        }
    }
}

impl std::error::Error for TaskError {}

#[async_trait::async_trait]
pub trait TaskService: Debug + Send + Sync {
    async fn execute(&self) -> Result<(), TaskError>;
    fn clone_box(&self) -> Box<dyn TaskService>;
    fn name(&self) -> &str;
}

impl Clone for Box<dyn TaskService> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// Example task services
#[derive(Debug, Clone)]
pub struct DataBackupService {
    pub backup_path: String,
}

#[async_trait::async_trait]
impl TaskService for DataBackupService {
    async fn execute(&self) -> Result<(), TaskError> {
        println!("Running data backup to: {}", self.backup_path);
        // Simulate backup work
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        println!("Data backup completed");
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn TaskService> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "DataBackupService"
    }
}

#[derive(Debug, Clone)]
pub struct ContentIndexingService;

#[async_trait::async_trait]
impl TaskService for ContentIndexingService {
    async fn execute(&self) -> Result<(), TaskError> {
        println!("Running content indexing...");
        // Simulate indexing work
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        println!("Content indexing completed");
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn TaskService> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "ContentIndexingService"
    }
}