/*
User Configuration

Configuration setup for user-related services, including dependency injection
and service initialization.
*/

use crate::core::platform::manager::user_service::UserService;
use crate::infrastructure::repositories::sqlite_user_repository::SqliteUserRepository;
use crate::application::ports::output::log_port::LogPort;
use crate::core::platform::manager::notification_service::NotificationService;
use crate::config::application_settings::Settings;
use std::sync::Arc;

/// User service configuration and factory
pub struct UserServiceFactory;

impl UserServiceFactory {
    /// Create a new UserService with all dependencies
    pub async fn create_user_service(
        settings: &Settings,
        log_port: Arc<dyn LogPort>,
        notification_service: Arc<NotificationService>,
    ) -> Result<Arc<UserService>, Box<dyn std::error::Error>> {
        // Create user repository
        let user_repository = Arc::new(
            SqliteUserRepository::new(settings).await?
        );

        // Create user service with dependencies
        let user_service = Arc::new(UserService::new(
            user_repository,
            log_port,
            notification_service,
        ));

        Ok(user_service)
    }
}