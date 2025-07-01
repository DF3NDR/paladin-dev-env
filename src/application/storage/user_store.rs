/*
User Repository Port

This port defines the contract for user data persistence operations.
Infrastructure adapters must implement this trait to provide actual
database interaction capabilities.
*/

use crate::core::platform::container::user::{User, UserError};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepositoryPort: Send + Sync {
    /// Find user by ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserError>;
    
    /// Find user by email address
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError>;
    
    /// Find user by username
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, UserError>;
    
    /// Save a new user
    async fn save(&self, user: User) -> Result<User, UserError>;
    
    /// Update an existing user
    async fn update(&self, user: User) -> Result<User, UserError>;
    
    /// Delete a user by ID
    async fn delete(&self, id: Uuid) -> Result<(), UserError>;
    
    /// Check if email exists
    async fn email_exists(&self, email: &str) -> Result<bool, UserError>;
    
    /// Check if username exists
    async fn username_exists(&self, username: &str) -> Result<bool, UserError>;
    
    /// Get users by active status
    async fn find_by_active_status(&self, is_active: bool) -> Result<Vec<User>, UserError>;
    
    /// Get users by verification status
    async fn find_by_verification_status(&self, is_verified: bool) -> Result<Vec<User>, UserError>;
    
    /// Count total users
    async fn count_users(&self) -> Result<u64, UserError>;
}