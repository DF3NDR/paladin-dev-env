/*
User Controller

REST API endpoints for user operations. This handles HTTP requests and responses,
delegating business logic to the UserService.
*/

use crate::core::platform::manager::user_service::{
    UserService, UserRegistrationRequest, UserLoginRequest, 
    UserProfileUpdateRequest
};
use crate::core::platform::container::user::{UserError, UserProfile};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// User registration request DTO
#[derive(Debug, Deserialize)]
pub struct RegisterUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}

/// User login request DTO
#[derive(Debug, Deserialize)]
pub struct LoginUserRequest {
    pub email: String,
    pub password: String,
}

/// User profile update request DTO
#[derive(Debug, Deserialize)]
pub struct UpdateUserProfileRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}

/// User response DTO
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub profile: UserProfileResponse,
    pub created_at: String,
    pub modified_at: String,
}

/// User profile response DTO
#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}

/// Login response DTO
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub is_verified: bool,
    pub success: bool,
}

/// Error response DTO
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// API Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorResponse>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String, code: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ErrorResponse { error, code }),
        }
    }
}

/// Convert UserError to HTTP status code and error response
fn user_error_to_response(error: UserError) -> (StatusCode, Json<ApiResponse<()>>) {
    let (status, code, message) = match error {
        UserError::InvalidEmail(_) => (StatusCode::BAD_REQUEST, "INVALID_EMAIL", error.to_string()),
        UserError::InvalidUsername(_) => (StatusCode::BAD_REQUEST, "INVALID_USERNAME", error.to_string()),
        UserError::InvalidPassword(_) => (StatusCode::BAD_REQUEST, "INVALID_PASSWORD", error.to_string()),
        UserError::EmailAlreadyExists(_) => (StatusCode::CONFLICT, "EMAIL_EXISTS", error.to_string()),
        UserError::UsernameAlreadyExists(_) => (StatusCode::CONFLICT, "USERNAME_EXISTS", error.to_string()),
        UserError::UserNotFound(_) => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", error.to_string()),
        UserError::UserNotFoundByEmail(_) => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", error.to_string()),
        UserError::AuthenticationFailed => (StatusCode::UNAUTHORIZED, "AUTH_FAILED", "Invalid credentials".to_string()),
        UserError::UserNotActive => (StatusCode::FORBIDDEN, "USER_INACTIVE", "User account is not active".to_string()),
        UserError::UserNotVerified => (StatusCode::FORBIDDEN, "USER_NOT_VERIFIED", "User email is not verified".to_string()),
        UserError::RepositoryError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal server error".to_string()),
        UserError::HashError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal server error".to_string()),
    };

    (status, Json(ApiResponse::error(message, code.to_string())))
}

/// Convert User to UserResponse
fn user_to_response(user: &crate::core::platform::container::user::User) -> UserResponse {
    UserResponse {
        id: user.uuid,
        username: user.username().to_string(),
        email: user.email().value().to_string(),
        is_active: user.is_active(),
        is_verified: user.is_verified(),
        profile: UserProfileResponse {
            first_name: user.profile().first_name.clone(),
            last_name: user.profile().last_name.clone(),
            bio: user.profile().bio.clone(),
            avatar_url: user.profile().avatar_url.clone(),
            timezone: user.profile().timezone.clone(),
            locale: user.profile().locale.clone(),
        },
        created_at: user.created.to_rfc3339(),
        modified_at: user.modified.to_rfc3339(),
    }
}

/// Register a new user
async fn register_user(
    State(user_service): State<Arc<UserService>>,
    Json(request): Json<RegisterUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let profile = if request.first_name.is_some() || request.last_name.is_some() || 
                     request.bio.is_some() || request.timezone.is_some() || request.locale.is_some() {
        Some(UserProfile {
            first_name: request.first_name,
            last_name: request.last_name,
            bio: request.bio,
            avatar_url: None,
            timezone: request.timezone.or_else(|| Some("UTC".to_string())),
            locale: request.locale.or_else(|| Some("en-US".to_string())),
        })
    } else {
        None
    };

    let registration_request = UserRegistrationRequest {
        username: request.username,
        email: request.email,
        password: request.password,
        profile,
    };

    match user_service.register_user(registration_request).await {
        Ok(user) => Ok(Json(ApiResponse::success(user_to_response(&user)))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// User login
async fn login_user(
    State(user_service): State<Arc<UserService>>,
    Json(request): Json<LoginUserRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let login_request = UserLoginRequest {
        email: request.email,
        password: request.password,
    };

    match user_service.login_user(login_request).await {
        Ok(result) => Ok(Json(ApiResponse::success(LoginResponse {
            user_id: result.user_id,
            username: result.username,
            email: result.email,
            is_verified: result.is_verified,
            success: result.success,
        }))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Get user by ID
async fn get_user(
    State(user_service): State<Arc<UserService>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match user_service.get_user_by_id(user_id).await {
        Ok(Some(user)) => Ok(Json(ApiResponse::success(user_to_response(&user)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("User not found".to_string(), "USER_NOT_FOUND".to_string()))
        )),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Update user profile
async fn update_user_profile(
    State(user_service): State<Arc<UserService>>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserProfileRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let profile = if request.first_name.is_some() || request.last_name.is_some() || 
                     request.bio.is_some() || request.avatar_url.is_some() ||
                     request.timezone.is_some() || request.locale.is_some() {
        // First, get current user to preserve existing profile values
        let current_user = match user_service.get_user_by_id(user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("User not found".to_string(), "USER_NOT_FOUND".to_string()))
            )),
            Err(error) => return Err(user_error_to_response(error)),
        };

        Some(UserProfile {
            first_name: request.first_name.or(current_user.profile().first_name.clone()),
            last_name: request.last_name.or(current_user.profile().last_name.clone()),
            bio: request.bio.or(current_user.profile().bio.clone()),
            avatar_url: request.avatar_url.or(current_user.profile().avatar_url.clone()),
            timezone: request.timezone.or(current_user.profile().timezone.clone()),
            locale: request.locale.or(current_user.profile().locale.clone()),
        })
    } else {
        None
    };

    let update_request = UserProfileUpdateRequest {
        user_id,
        username: request.username,
        email: request.email,
        profile,
    };

    match user_service.update_user_profile(update_request).await {
        Ok(user) => Ok(Json(ApiResponse::success(user_to_response(&user)))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Activate user account
async fn activate_user(
    State(user_service): State<Arc<UserService>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match user_service.activate_user(user_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Deactivate user account
async fn deactivate_user(
    State(user_service): State<Arc<UserService>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match user_service.deactivate_user(user_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Verify user email
async fn verify_user(
    State(user_service): State<Arc<UserService>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match user_service.verify_user(user_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(()))),
        Err(error) => Err(user_error_to_response(error)),
    }
}

/// Create the user routes
pub fn create_user_routes(user_service: Arc<UserService>) -> Router {
    Router::new()
        .route("/users/register", post(register_user))
        .route("/users/login", post(login_user))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user_profile))
        .route("/users/:id/activate", post(activate_user))
        .route("/users/:id/deactivate", post(deactivate_user))
        .route("/users/:id/verify", post(verify_user))
        .with_state(user_service)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::user::Email;
    use crate::core::platform::manager::user_service::UserAuthenticationResult;
    use crate::core::platform::container::user::User;

    #[test]
    fn test_email_validation() {
        // Valid emails
        assert!(Email::new("test@example.com".to_string()).is_ok());
        assert!(Email::new("user.name+tag@domain.co.uk".to_string()).is_ok());
        assert!(Email::new("123@test.org".to_string()).is_ok());

        // Invalid emails
        assert!(Email::new("invalid-email".to_string()).is_err());
        assert!(Email::new("@domain.com".to_string()).is_err());
        assert!(Email::new("user@".to_string()).is_err());
        assert!(Email::new("".to_string()).is_err());
    }

    #[test]
    fn test_email_methods() {
        let email = Email::new("Test.User@Example.COM".to_string()).unwrap();
        
        // Email should be normalized to lowercase
        assert_eq!(email.value(), "test.user@example.com");
        assert_eq!(email.domain(), Some("example.com"));
        assert_eq!(email.local_part(), Some("test.user"));
        assert_eq!(email.to_string(), "test.user@example.com");
    }

    #[test]
    fn test_user_creation() {
        let email = Email::new("user@example.com".to_string()).unwrap();
        let user = User::new_user(
            "testuser".to_string(),
            email.clone(),
            "password_hash".to_string(),
            None,
        );

        assert_eq!(user.username(), "testuser");
        assert_eq!(user.email(), &email);
        assert_eq!(user.password_hash(), "password_hash");
        assert!(user.is_active());
        assert!(!user.is_verified());
    }

    #[test]
    fn test_user_updates() {
        let email = Email::new("user@example.com".to_string()).unwrap();
        let mut user = User::new_user(
            "testuser".to_string(),
            email,
            "password_hash".to_string(),
            None,
        );

        // Test username update
        assert!(user.update_username("newusername".to_string()).is_ok());
        assert_eq!(user.username(), "newusername");

        // Test invalid username
        assert!(user.update_username("a".to_string()).is_err());
        assert!(user.update_username("".to_string()).is_err());

        // Test email update
        let new_email = Email::new("new@example.com".to_string()).unwrap();
        assert!(user.update_email(new_email.clone()).is_ok());
        assert_eq!(user.email(), &new_email);
        assert!(!user.is_verified()); // Should reset verification

        // Test activation/deactivation
        user.deactivate();
        assert!(!user.is_active());
        user.activate();
        assert!(user.is_active());

        // Test verification
        user.verify();
        assert!(user.is_verified());
    }

    #[tokio::test]
    async fn test_user_service_password_hashing() {
        use crate::application::storage::user_store::UserRepositoryPort;
        use crate::application::ports::output::log_port::LogPort;
        use crate::application::ports::output::notification_port::NotificationPublisherPort;
        use std::sync::Arc;
        use async_trait::async_trait;

        // Mock implementations for testing
        struct MockUserRepository;
        struct MockLogPort;
        struct MockNotificationPublisher;

        #[async_trait]
        impl UserRepositoryPort for MockUserRepository {
            async fn find_by_id(&self, _id: Uuid) -> Result<Option<User>, UserError> { Ok(None) }
            async fn find_by_email(&self, _email: &str) -> Result<Option<User>, UserError> { Ok(None) }
            async fn find_by_username(&self, _username: &str) -> Result<Option<User>, UserError> { Ok(None) }
            async fn save(&self, user: User) -> Result<User, UserError> { Ok(user) }
            async fn update(&self, user: User) -> Result<User, UserError> { Ok(user) }
            async fn delete(&self, _id: Uuid) -> Result<(), UserError> { Ok(()) }
            async fn email_exists(&self, _email: &str) -> Result<bool, UserError> { Ok(false) }
            async fn username_exists(&self, _username: &str) -> Result<bool, UserError> { Ok(false) }
            async fn find_by_active_status(&self, _is_active: bool) -> Result<Vec<User>, UserError> { Ok(vec![]) }
            async fn find_by_verification_status(&self, _is_verified: bool) -> Result<Vec<User>, UserError> { Ok(vec![]) }
            async fn count_users(&self) -> Result<u64, UserError> { Ok(0) }
        }

        #[async_trait]
        impl LogPort for MockLogPort {
            async fn log(&self, _level: LogLevel, _message: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }
        }

        #[async_trait]
        impl NotificationPublisherPort for MockNotificationPublisher {
            async fn publish(&self, _notification: crate::core::platform::manager::notification_publisher::NotificationRequest) -> Result<crate::core::platform::manager::notification_publisher::NotificationResponse, Box<dyn std::error::Error + Send + Sync>> {
                use uuid::Uuid;
                Ok(crate::core::platform::manager::notification_publisher::NotificationResponse {
                    id: Uuid::new_v4(),
                    status: crate::core::platform::manager::notification_publisher::NotificationStatus::Delivered,
                    delivered_at: Some(chrono::Utc::now()),
                    failure_reason: None,
                    retry_count: 0,
                    metadata: None,
                })
            }
        }

        let user_service = UserService::new(
            Arc::new(MockUserRepository),
            Arc::new(MockLogPort),
            Arc::new(MockNotificationPublisher),
        );

        // Test password hashing
        let password = "test_password123";
        let hash = user_service.hash_password(password).unwrap();
        
        assert_ne!(hash, password);
        assert!(hash.starts_with("$argon2"));
        
        // Test password verification
        assert!(user_service.verify_password(password, &hash).unwrap());
        assert!(!user_service.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_username_validation() {
        let user_service = UserService::new(
            Arc::new(MockUserRepository),
            Arc::new(MockLogPort),
            Arc::new(MockNotificationPublisher),
        );

        // Valid usernames
        assert!(user_service.validate_username("validuser").is_ok());
    }
}