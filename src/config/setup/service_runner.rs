use std::sync::Arc;
use crate::config::application_settings::Settings;
use crate::infrastructure::adapters::output::sql_content_repository::SqlContentRepository;
use tokio::task;
use crate::core::platform::manager::scheduler::start_scheduler;

pub async fn run_services(config: Arc<Settings>) {
    // Initialize the database
    // Our Configuration for the SQL database is stored in the Settings struct
    let _db = SqlContentRepository::init_db().expect("Failed to initialize database");

    // Start the scheduler
    task::spawn(async move {
        start_scheduler().await;
    });

}