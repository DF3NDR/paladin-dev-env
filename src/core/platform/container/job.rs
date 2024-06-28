
use super::task::Task;

pub struct Job {
    tasks: Vec<Task>,
    // Add other fields associated with data orchestration here
}

impl Job {
    pub fn new(tasks: Vec<Task>) -> Job {
        Job { tasks }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }
    
    pub fn remove_task(&mut self, task: Task) {
        self.tasks.retain(|t| t != &task);
    }
    
    pub fn delete_job(&mut self) {
        self.tasks.clear();
    }
}