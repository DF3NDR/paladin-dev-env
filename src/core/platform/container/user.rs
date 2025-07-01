/*
User Container

User type built on the core base entity Node type to utilize the versioning system 
through composition. It represents a user entity in the system with all needed values
including username, email, and password hash.

This follows Domain-Driven Design principles and leverages the existing Node infrastructure.
*/

use crate::core::base::entity::node::Node;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Email value object that encapsulates email validation logic
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Email {
    value: String,
}

impl Email {
    /// Creates a new Email value object with validation
    pub fn new(email: String) -> Result<Self, UserError> {
        if Self::is_valid(&email) {
            Ok(Self { value: email.to_lowercase() })
        } else {
            Err(UserError::InvalidEmail(email))
        }
    }

    /// Validates email format using a comprehensive regex
    fn is_valid(email: &str) -> bool {
        use regex::Regex;
        
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).unwrap();
        
        email_regex.is_match(email) && email.len() <= 254
    }

    /// Returns the email value as a string
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the domain part of the email
    pub fn domain(&self) -> Option<&str> {
        self.value.split('@').nth(1)
    }

    /// Returns the local part of the email
    pub fn local_part(&self) -> Option<&str> {
        self.value.split('@').next()
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// User data structure that will be wrapped by Node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserData {
    pub username: String,
    pub email: Email,
    pub password_hash: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub profile: UserProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            first_name: None,
            last_name: None,
            bio: None,
            avatar_url: None,
            timezone: Some("UTC".to_string()),
            locale: Some("en-US".to_string()),
        }
    }
}

impl Hash for UserData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.username.hash(state);
        self.email.hash(state);
        self.password_hash.hash(state);
        self.is_active.hash(state);
        self.is_verified.hash(state);
    }
}

/// User type built on Node for versioning and consistency
pub type User = Node<UserData>;

impl User {
    /// Creates a new User with UserData
    pub fn new_user(
        username: String,
        email: Email,
        password_hash: String,
        profile: Option<UserProfile>,
    ) -> Self {
        let user_data = UserData {
            username,
            email,
            password_hash,
            is_active: true,
            is_verified: false,
            profile: profile.unwrap_or_default(),
        };
        
        Node::new(user_data, Some(format!("User: {}", user_data.username)))
    }

    /// Gets the username
    pub fn username(&self) -> &str {
        &self.node.username
    }

    /// Gets the email
    pub fn email(&self) -> &Email {
        &self.node.email
    }

    /// Gets the password hash
    pub fn password_hash(&self) -> &str {
        &self.node.password_hash
    }

    /// Checks if user is active
    pub fn is_active(&self) -> bool {
        self.node.is_active
    }

    /// Checks if user is verified
    pub fn is_verified(&self) -> bool {
        self.node.is_verified
    }

    /// Gets the user profile
    pub fn profile(&self) -> &UserProfile {
        &self.node.profile
    }

    /// Updates username
    pub fn update_username(&mut self, new_username: String) -> Result<(), UserError> {
        if new_username.trim().is_empty() {
            return Err(UserError::InvalidUsername("Username cannot be empty".to_string()));
        }
        if new_username.len() < 3 {
            return Err(UserError::InvalidUsername("Username must be at least 3 characters".to_string()));
        }
        if new_username.len() > 50 {
            return Err(UserError::InvalidUsername("Username cannot exceed 50 characters".to_string()));
        }
        
        self.node.username = new_username;
        self.touch(); // Update modified timestamp
        Ok(())
    }

    /// Updates email
    pub fn update_email(&mut self, new_email: Email) -> Result<(), UserError> {
        self.node.email = new_email;
        self.node.is_verified = false; // Reset verification on email change
        self.touch();
        Ok(())
    }

    /// Updates password hash
    pub fn update_password_hash(&mut self, new_password_hash: String) {
        self.node.password_hash = new_password_hash;
        self.touch();
    }

    /// Activates the user
    pub fn activate(&mut self) {
        self.node.is_active = true;
        self.touch();
    }

    /// Deactivates the user
    pub fn deactivate(&mut self) {
        self.node.is_active = false;
        self.touch();
    }

    /// Verifies the user
    pub fn verify(&mut self) {
        self.node.is_verified = true;
        self.touch();
    }

    /// Updates the user profile
    pub fn update_profile(&mut self, profile: UserProfile) {
        self.node.profile = profile;
        self.touch();
    }
}

/// User-specific error types
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Invalid email format: {0}")]
    InvalidEmail(String),
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("User not found with ID: {0}")]
    UserNotFound(Uuid),
    #[error("User not found with email: {0}")]
    UserNotFoundByEmail(String),
    #[error("Email already exists: {0}")]
    EmailAlreadyExists(String),
    #[error("Username already exists: {0}")]
    UsernameAlreadyExists(String),
    #[error("Invalid password: {0}")]
    InvalidPassword(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("User is not active")]
    UserNotActive,
    #[error("User is not verified")]
    UserNotVerified,
    #[error("Repository error: {0}")]
    RepositoryError(String),
    #[error("Hash error: {0}")]
    HashError(String),
}