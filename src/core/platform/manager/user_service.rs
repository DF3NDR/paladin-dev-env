/*
User Service

Business logic layer for user operations including registration, authentication,
and profile management. This service coordinates between domain entities and 
infrastructure adapters.
*/

use crate::core::platform::container::user::{User, Email, UserProfile, UserError};
use crate::application::storage::user_store::UserRepositoryPort;
use crate::application::ports::output::log_port::LogPort;
use crate::core::platform::manager::notification_service::NotificationService;
use crate::core::platform::container::log::{LogLevel, LogEntryBuilder, LogDestination};
use crate::core::base::entity::message::Location;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use std::sync::Arc;
use uuid::Uuid;
use async_trait::async_trait;

/// User registration request
#[derive(Debug, Clone)]
pub struct UserRegistrationRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub profile: Option<UserProfile>,
}

/// User login request
#[derive(Debug, Clone)]
pub struct UserLoginRequest {
    pub email: String,
    pub password: String,
}

/// User profile update request
#[derive(Debug, Clone)]
pub struct UserProfileUpdateRequest {
    pub user_id: Uuid,
    pub username: Option<String>,
    pub email: Option<String>,
    pub profile: Option<UserProfile>,
}

/// User login response
#[derive(Debug, Clone)]
pub struct UserAuthenticationResult {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
    pub success: bool,
}

/// User Service trait defining business operations
#[async_trait]
pub trait UserServiceTrait: Send + Sync {
    /// Register a new user
    async fn register_user(&self, request: UserRegistrationRequest) -> Result<User, UserError>;
    
    /// Authenticate a user login
    async fn login_user(&self, request: UserLoginRequest) -> Result<UserAuthenticationResult, UserError>;
    
    /// Update user profile
    async fn update_user_profile(&self, request: UserProfileUpdateRequest) -> Result<User, UserError>;
    
    /// Get user by ID
    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, UserError>;
    
    /// Get user by email
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserError>;
    
    /// Activate user account
    async fn activate_user(&self, user_id: Uuid) -> Result<(), UserError>;
    
    /// Deactivate user account
    async fn deactivate_user(&self, user_id: Uuid) -> Result<(), UserError>;
    
    /// Verify user email
    async fn verify_user(&self, user_id: Uuid) -> Result<(), UserError>;
    
    /// CLI support methods
    
    /// Find users by active status
    async fn find_by_active_status(&self, is_active: bool) -> Result<Vec<User>, UserError>;
    
    /// Find users by verification status  
    async fn find_by_verification_status(&self, is_verified: bool) -> Result<Vec<User>, UserError>;
    
    /// Count total users
    async fn count_users(&self) -> Result<u64, UserError>;
}

/// Concrete implementation of UserService
pub struct UserService {
    user_repository: Arc<dyn UserRepositoryPort>,
    log_port: Arc<dyn LogPort>,
    notification_service: Arc<NotificationService>,
    argon2: Argon2<'static>,
}

impl UserService {
    /// Creates a new UserService instance
    pub fn new(
        user_repository: Arc<dyn UserRepositoryPort>,
        log_port: Arc<dyn LogPort>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            user_repository,
            log_port,
            notification_service,
            argon2: Argon2::default(),
        }
    }

    /// Hash password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String, UserError> {
        if password.len() < 8 {
            return Err(UserError::InvalidPassword("Password must be at least 8 characters".to_string()));
        }
        if password.len() > 128 {
            return Err(UserError::InvalidPassword("Password cannot exceed 128 characters".to_string()));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| UserError::HashError(e.to_string()))?;
        
        Ok(password_hash.to_string())
    }

    /// Verify password against hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, UserError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| UserError::HashError(e.to_string()))?;
        
        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Validate username
    fn validate_username(&self, username: &str) -> Result<(), UserError> {
        if username.trim().is_empty() {
            return Err(UserError::InvalidUsername("Username cannot be empty".to_string()));
        }
        if username.len() < 3 {
            return Err(UserError::InvalidUsername("Username must be at least 3 characters".to_string()));
        }
        if username.len() > 50 {
            return Err(UserError::InvalidUsername("Username cannot exceed 50 characters".to_string()));
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(UserError::InvalidUsername("Username can only contain alphanumeric characters, underscores, and hyphens".to_string()));
        }
        Ok(())
    }

    /// Send welcome notification
    async fn send_welcome_notification(&self, user: &User) -> Result<(), UserError> {
        use crate::core::platform::container::notification::{
            NotificationRecipient, NotificationContent, 
            NotificationChannel, NotificationPriority
        };

        let recipient = NotificationRecipient::Email(user.email().value().to_string());
        let content = NotificationContent {
            title: "Welcome to in4me!".to_string(),
            body: format!("Hello {}, welcome to our platform!", user.username()),
            category: "welcome".to_string(),
            action_url: None,
            attachments: Vec::new(),
            template_id: Some("user_welcome".to_string()),
            template_variables: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        };

        let notification = self.notification_service
            .create_notification(
                recipient,
                content,
                NotificationChannel::Email,
                NotificationPriority::Normal,
            )
            .await
            .map_err(|e| UserError::RepositoryError(format!("Failed to create welcome notification: {}", e)))?;

        self.notification_service
            .send_notification(notification.id)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Failed to send welcome notification: {}", e)))?;

        Ok(())
    }

    /// Log user action
    async fn log_action(&self, level: LogLevel, message: String, user_id: Option<Uuid>) {
        let enhanced_message = match user_id {
            Some(id) => format!("[User: {}] {}", id, message),
            None => message,
        };
        
        // Create a proper log entry using LogEntryBuilder
        let log_entry = LogEntryBuilder::new_entry(
            Location::service("user-service"),
            LogDestination::System,
            level,
            enhanced_message,
        );
        
        // Use write_entry instead of log
        if let Err(e) = self.log_port.write_entry(log_entry).await {
            eprintln!("Failed to log user action: {}", e);
        }
    }
}

#[async_trait]
impl UserServiceTrait for UserService {
    async fn register_user(&self, request: UserRegistrationRequest) -> Result<User, UserError> {
        // Validate input
        self.validate_username(&request.username)?;
        let email = Email::new(request.email)?;
        
        // Check if user already exists
        if let Some(_) = self.user_repository.find_by_email(email.value()).await? {
            return Err(UserError::EmailAlreadyExists(email.value().to_string()));
        }

        // Hash password
        let password_hash = self.hash_password(&request.password)?;

        // Create user
        let user = User::new_user(
            request.username.clone(),
            email,
            password_hash,
            request.profile,
        );

        // Save user
        let saved_user = self.user_repository.save(user).await?;

        // Log successful registration
        self.log_action(
            LogLevel::Info,
            format!("User registered successfully: {}", request.username),
            Some(saved_user.uuid),
        ).await;

        // Send welcome notification
        if let Err(e) = self.send_welcome_notification(&saved_user).await {
            self.log_action(
                LogLevel::Warn,
                format!("Failed to send welcome notification: {}", e),
                Some(saved_user.uuid),
            ).await;
        }

        Ok(saved_user)
    }

    async fn login_user(&self, request: UserLoginRequest) -> Result<UserAuthenticationResult, UserError> {
        // Find user by email
        let user = self.user_repository.find_by_email(&request.email).await?
            .ok_or(UserError::AuthenticationFailed)?;

        // Check if user is active
        if !user.is_active() {
            self.log_action(
                LogLevel::Warn,
                format!("Login attempt for inactive user: {}", request.email),
                Some(user.uuid),
            ).await;
            return Err(UserError::UserNotActive);
        }

        // Verify password
        if !self.verify_password(&request.password, user.password_hash())? {
            self.log_action(
                LogLevel::Warn,
                format!("Failed login attempt for user: {}", request.email),
                Some(user.uuid),
            ).await;
            return Err(UserError::AuthenticationFailed);
        }

        // Log successful login
        self.log_action(
            LogLevel::Info,
            format!("User logged in successfully: {}", request.email),
            Some(user.uuid),
        ).await;

        Ok(UserAuthenticationResult {
            user_id: user.uuid,
            username: user.username().to_string(),
            email: user.email().value().to_string(),
            is_verified: user.is_verified(),
            success: true,
        })
    }

    async fn update_user_profile(&self, request: UserProfileUpdateRequest) -> Result<User, UserError> {
        // Get existing user
        let mut user = self.user_repository.find_by_id(request.user_id).await?
            .ok_or(UserError::UserNotFound(request.user_id))?;

        // Update username if provided
        if let Some(new_username) = request.username {
            user.update_username(new_username)?;
        }

        // Update email if provided
        if let Some(new_email) = request.email {
            let email = Email::new(new_email)?;
            
            // Check if email is already taken by another user
            if let Some(existing_user) = self.user_repository.find_by_email(email.value()).await? {
                if existing_user.uuid != user.uuid {
                    return Err(UserError::EmailAlreadyExists(email.value().to_string()));
                }
            }
            
            user.update_email(email)?;
        }

        // Update profile if provided
        if let Some(new_profile) = request.profile {
            user.update_profile(new_profile);
        }

        // Save updated user
        let updated_user = self.user_repository.update(user).await?;

        // Log update
        self.log_action(
            LogLevel::Info,
            "User profile updated successfully".to_string(),
            Some(updated_user.uuid),
        ).await;

        Ok(updated_user)
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, UserError> {
        self.user_repository.find_by_id(user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
        self.user_repository.find_by_email(email).await
    }

    async fn activate_user(&self, user_id: Uuid) -> Result<(), UserError> {
        let mut user = self.user_repository.find_by_id(user_id).await?
            .ok_or(UserError::UserNotFound(user_id))?;

        user.activate();
        self.user_repository.update(user).await?;

        self.log_action(
            LogLevel::Info,
            "User account activated".to_string(),
            Some(user_id),
        ).await;

        Ok(())
    }

    async fn deactivate_user(&self, user_id: Uuid) -> Result<(), UserError> {
        let mut user = self.user_repository.find_by_id(user_id).await?
            .ok_or(UserError::UserNotFound(user_id))?;

        user.deactivate();
        self.user_repository.update(user).await?;

        self.log_action(
            LogLevel::Info,
            "User account deactivated".to_string(),
            Some(user_id),
        ).await;

        Ok(())
    }

    async fn verify_user(&self, user_id: Uuid) -> Result<(), UserError> {
        let mut user = self.user_repository.find_by_id(user_id).await?
            .ok_or(UserError::UserNotFound(user_id))?;

        user.verify();
        self.user_repository.update(user).await?;

        self.log_action(
            LogLevel::Info,
            "User email verified".to_string(),
            Some(user_id),
        ).await;

        Ok(())
    }

    /// CLI support methods
    
    /// Find users by active status
    async fn find_by_active_status(&self, is_active: bool) -> Result<Vec<User>, UserError> {
        self.user_repository.find_by_active_status(is_active).await
    }

    /// Find users by verification status  
    async fn find_by_verification_status(&self, is_verified: bool) -> Result<Vec<User>, UserError> {
        self.user_repository.find_by_verification_status(is_verified).await
    }

    /// Count total users
    async fn count_users(&self) -> Result<u64, UserError> {
        self.user_repository.count_users().await
    }
}