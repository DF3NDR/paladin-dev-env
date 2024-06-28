

pub struct Task {
    // Define the fields for a task here
    index: usize,
    name: String,
    service: Box<dyn TaskService>,
}

impl Task {
    pub fn new(index: usize, name: String, service: Box<dyn TaskService>) -> Task {
        Task { index, name, service }
    }

    pub fn execute(&self) {
        self.service.execute();
    }
}

pub trait TaskService {
    fn execute(&self);
}