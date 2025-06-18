use super::task::{Task, TaskError};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub tasks: Vec<Task>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub parallel_execution: bool,
}

impl Job {
    pub fn new(name: String, tasks: Vec<Task>) -> Job {
        Job {
            id: Uuid::new_v4(),
            name,
            tasks,
            created_at: Utc::now(),
            last_run: None,
            run_count: 0,
            parallel_execution: false,
        }
    }

    pub fn with_parallel_execution(mut self, parallel: bool) -> Self {
        self.parallel_execution = parallel;
        self
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }
    
    pub fn remove_task(&mut self, task_id: Uuid) -> bool {
        let original_len = self.tasks.len();
        self.tasks.retain(|t| t.id() != task_id);
        self.tasks.len() != original_len
    }
    
    pub fn clear_tasks(&mut self) {
        self.tasks.clear();
    }

    pub async fn execute(&mut self) -> Result<Vec<Result<(), TaskError>>, JobError> {
        if self.tasks.is_empty() {
            return Err(JobError::NoTasks);
        }

        println!("Executing job: {} with {} tasks", self.name, self.tasks.len());
        
        let results = if self.parallel_execution {
            self.execute_parallel().await
        } else {
            self.execute_sequential().await
        };

        self.last_run = Some(Utc::now());
        self.run_count += 1;

        Ok(results)
    }

    async fn execute_sequential(&mut self) -> Vec<Result<(), TaskError>> {
        let mut results = Vec::new();
        
        for task in &mut self.tasks {
            let result = task.execute().await;
            results.push(result);
        }
        
        results
    }

    async fn execute_parallel(&mut self) -> Vec<Result<(), TaskError>> {
        use futures::future::join_all;
        
        let mut futures = Vec::new();
        
        for task in &mut self.tasks {
            futures.push(task.execute());
        }
        
        join_all(futures).await
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Debug, Clone)]
pub enum JobError {
    NoTasks,
    ExecutionFailed(String),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::NoTasks => write!(f, "Job has no tasks to execute"),
            JobError::ExecutionFailed(msg) => write!(f, "Job execution failed: {}", msg),
        }
    }
}

impl std::error::Error for JobError {}