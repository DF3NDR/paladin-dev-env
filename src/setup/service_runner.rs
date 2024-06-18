use std::sync::Arc;
use crate::config::Settings;
use crate::infrastructure::repositories::sql_content_repository::SqlContentRepository;
use crate::infrastructure::notifications::email_notification_service::EmailNotificationService;
use tokio::task;
use crate::setup::use_case_initializer::UseCases;
use crate::adapters::primary::{scheduler::start_scheduler, queue::start_analyzer};

pub async fn run_services(config: Arc<Settings>, use_cases: UseCases) {
    // Initialize the database
    let _db = SqlContentRepository::init_db().expect("Failed to initialize database");

    // Initialize the email notification service
    let _email_service = EmailNotificationService::new();

    // Start the scheduler
    task::spawn(async move {
        start_scheduler().await;
    });

    // Start the analyzer
    task::spawn(async move {
        start_analyzer(config.clone()).await;
    });
}
