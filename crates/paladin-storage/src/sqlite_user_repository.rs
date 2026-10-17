/*
SQLite User Repository Adapter

Concrete implementation of UserRepositoryPort using SQLite database.
This adapter handles the actual database operations for user persistence.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use paladin_core::platform::container::user::{
    Email, User, UserData, UserError, UserProfile, UserRole,
};
use paladin_ports::output::user_repository_port::UserRepositoryPort;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::str::FromStr;
use uuid::Uuid;

#[doc(hidden)]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    /// Create a new SQLite user repository.
    pub async fn new(database_url: &str) -> Result<Self, UserError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| {
                UserError::RepositoryError(format!("Failed to connect to database: {}", e))
            })?;

        let repository = Self { pool };
        repository.migrate().await?;

        Ok(repository)
    }

    /// Create a new SQLite user repository with custom database URL (for testing)
    pub async fn new_with_url(database_url: &str) -> Result<Self, UserError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                UserError::RepositoryError(format!("Failed to connect to database: {}", e))
            })?;

        let repository = Self { pool };
        repository.migrate().await?;

        Ok(repository)
    }

    /// Run database migrations
    async fn migrate(&self) -> Result<(), UserError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                uuid TEXT UNIQUE NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                title TEXT,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                is_verified BOOLEAN NOT NULL DEFAULT 0,
                role TEXT NOT NULL DEFAULT 'user',
                first_name TEXT,
                last_name TEXT,
                bio TEXT,
                avatar_url TEXT,
                timezone TEXT,
                locale TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::RepositoryError(format!("Migration failed: {}", e)))?;

        // Idempotent migration: add the `role` column to pre-existing tables.
        // SQLite has no `ADD COLUMN IF NOT EXISTS`, so ignore the duplicate-column error.
        if let Err(e) =
            sqlx::query("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'")
                .execute(&self.pool)
                .await
        {
            let msg = e.to_string().to_lowercase();
            if !msg.contains("duplicate column") {
                return Err(UserError::RepositoryError(format!(
                    "Role column migration failed: {}",
                    e
                )));
            }
        }

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Index creation failed: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Index creation failed: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_uuid ON users(uuid)")
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Index creation failed: {}", e)))?;

        Ok(())
    }

    /// Convert database row to User
    fn row_to_user(&self, row: &sqlx::sqlite::SqliteRow) -> Result<User, UserError> {
        let uuid_str: String = row
            .try_get("uuid")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get uuid: {}", e)))?;
        let uuid = Uuid::from_str(&uuid_str)
            .map_err(|e| UserError::RepositoryError(format!("Invalid UUID: {}", e)))?;

        let email_str: String = row
            .try_get("email")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get email: {}", e)))?;
        let email = Email::new(email_str)?;

        let created_str: String = row
            .try_get("created_at")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get created_at: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| UserError::RepositoryError(format!("Invalid created_at: {}", e)))?
            .with_timezone(&Utc);

        let modified_str: String = row
            .try_get("modified_at")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get modified_at: {}", e)))?;
        let modified_at = DateTime::parse_from_rfc3339(&modified_str)
            .map_err(|e| UserError::RepositoryError(format!("Invalid modified_at: {}", e)))?
            .with_timezone(&Utc);

        let profile = UserProfile {
            first_name: row.try_get("first_name").ok(),
            last_name: row.try_get("last_name").ok(),
            bio: row.try_get("bio").ok(),
            avatar_url: row.try_get("avatar_url").ok(),
            timezone: row.try_get("timezone").ok(),
            locale: row.try_get("locale").ok(),
        };

        let user_data = UserData {
            username: row.try_get("username").map_err(|e| {
                UserError::RepositoryError(format!("Failed to get username: {}", e))
            })?,
            email,
            password_hash: row.try_get("password_hash").map_err(|e| {
                UserError::RepositoryError(format!("Failed to get password_hash: {}", e))
            })?,
            is_active: row.try_get("is_active").map_err(|e| {
                UserError::RepositoryError(format!("Failed to get is_active: {}", e))
            })?,
            is_verified: row.try_get("is_verified").map_err(|e| {
                UserError::RepositoryError(format!("Failed to get is_verified: {}", e))
            })?,
            role: row
                .try_get::<String, _>("role")
                .map(|r| UserRole::from_str_lossy(&r))
                .unwrap_or_default(),
            profile,
        };

        let user = User {
            uuid,
            version: row
                .try_get("version")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get version: {}", e)))?,
            created: created_at,
            modified: modified_at,
            name: row.try_get("username").ok(), // or use another field if appropriate
            data: user_data,
        };

        Ok(user)
    }
}

#[async_trait]
impl UserRepositoryPort for SqliteUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, UserError> {
        let row = sqlx::query("SELECT * FROM users WHERE uuid = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
        let row = sqlx::query("SELECT * FROM users WHERE email = ?")
            .bind(email.to_lowercase())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, UserError> {
        let row = sqlx::query("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn save(&self, user: User) -> Result<User, UserError> {
        sqlx::query(
            r#"
            INSERT INTO users (
                id, uuid, version, created_at, modified_at, title,
                username, email, password_hash, is_active, is_verified,
                role, first_name, last_name, bio, avatar_url, timezone, locale
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.uuid.to_string()) // Use UUID as primary key
        .bind(user.uuid.to_string())
        .bind(user.version as i64)
        .bind(user.created.to_rfc3339())
        .bind(user.modified.to_rfc3339())
        .bind(&user.name)
        .bind(&user.data.username)
        .bind(user.data.email.value())
        .bind(&user.data.password_hash)
        .bind(user.data.is_active)
        .bind(user.data.is_verified)
        .bind(user.data.role.as_str())
        .bind(&user.data.profile.first_name)
        .bind(&user.data.profile.last_name)
        .bind(&user.data.profile.bio)
        .bind(&user.data.profile.avatar_url)
        .bind(&user.data.profile.timezone)
        .bind(&user.data.profile.locale)
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::RepositoryError(format!("Failed to save user: {}", e)))?;

        Ok(user)
    }

    async fn update(&self, user: User) -> Result<User, UserError> {
        sqlx::query(
            r#"
            UPDATE users SET
                version = ?, modified_at = ?, title = ?,
                username = ?, email = ?, password_hash = ?,
                is_active = ?, is_verified = ?,
                role = ?, first_name = ?, last_name = ?, bio = ?,
                avatar_url = ?, timezone = ?, locale = ?
            WHERE uuid = ?
            "#,
        )
        .bind(user.version as i64)
        .bind(user.modified.to_rfc3339())
        .bind(&user.name)
        .bind(&user.data.username)
        .bind(user.data.email.value())
        .bind(&user.data.password_hash)
        .bind(user.data.is_active)
        .bind(user.data.is_verified)
        .bind(user.data.role.as_str())
        .bind(&user.data.profile.first_name)
        .bind(&user.data.profile.last_name)
        .bind(&user.data.profile.bio)
        .bind(&user.data.profile.avatar_url)
        .bind(&user.data.profile.timezone)
        .bind(&user.data.profile.locale)
        .bind(user.uuid.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::RepositoryError(format!("Failed to update user: {}", e)))?;

        Ok(user)
    }

    async fn delete(&self, id: Uuid) -> Result<(), UserError> {
        sqlx::query("DELETE FROM users WHERE uuid = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Failed to delete user: {}", e)))?;

        Ok(())
    }

    async fn email_exists(&self, email: &str) -> Result<bool, UserError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE email = ?")
            .bind(email.to_lowercase())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let count: i64 = row
            .try_get("count")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get count: {}", e)))?;

        Ok(count > 0)
    }

    async fn username_exists(&self, username: &str) -> Result<bool, UserError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let count: i64 = row
            .try_get("count")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get count: {}", e)))?;

        Ok(count > 0)
    }

    async fn find_by_active_status(&self, is_active: bool) -> Result<Vec<User>, UserError> {
        let rows = sqlx::query("SELECT * FROM users WHERE is_active = ?")
            .bind(is_active)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(&row)?);
        }

        Ok(users)
    }

    async fn find_by_verification_status(&self, is_verified: bool) -> Result<Vec<User>, UserError> {
        let rows = sqlx::query("SELECT * FROM users WHERE is_verified = ?")
            .bind(is_verified)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let mut users = Vec::new();
        for row in rows {
            users.push(self.row_to_user(&row)?);
        }

        Ok(users)
    }

    async fn count_users(&self) -> Result<u64, UserError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let count: i64 = row
            .try_get("count")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get count: {}", e)))?;

        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test repository with a temporary database
    async fn create_test_repository() -> Result<SqliteUserRepository, UserError> {
        // Use in-memory SQLite database for testing
        let database_url = "sqlite::memory:";

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| {
                UserError::RepositoryError(format!("Failed to connect to database: {}", e))
            })?;

        let repo = SqliteUserRepository { pool };
        repo.migrate().await?;

        Ok(repo)
    }

    // Helper function to create a test user
    fn create_test_user() -> User {
        let email = Email::new("test@example.com".to_string()).unwrap();
        User::new_user(
            "testuser".to_string(),
            email,
            "hashed_password".to_string(),
            Some(UserProfile {
                first_name: Some("Test".to_string()),
                last_name: Some("User".to_string()),
                bio: Some("Test bio".to_string()),
                avatar_url: Some("https://example.com/avatar.jpg".to_string()),
                timezone: Some("UTC".to_string()),
                locale: Some("en-US".to_string()),
            }),
        )
    }

    #[tokio::test]
    async fn test_repository_creation() {
        let result = create_test_repository().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Save the user
        let saved_user = repo.save(user.clone()).await.unwrap();
        assert_eq!(saved_user.uuid, user.uuid);

        // Find the user by ID
        let found_user = repo.find_by_id(user.uuid).await.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.uuid, user.uuid);
        assert_eq!(found_user.username(), user.username());
        assert_eq!(found_user.email().value(), user.email().value());
    }

    #[tokio::test]
    async fn test_role_persisted_and_read_back() {
        let repo = create_test_repository().await.unwrap();
        let mut user = create_test_user();
        user.set_role(UserRole::Admin);

        repo.save(user.clone()).await.unwrap();

        let found = repo.find_by_id(user.uuid).await.unwrap().unwrap();
        assert_eq!(found.role(), UserRole::Admin);

        // Default role round-trips as `User` as well.
        let mut default_user = create_test_user();
        default_user
            .update_email(Email::new("default@example.com".to_string()).unwrap())
            .unwrap();
        default_user
            .update_username("defaultuser".to_string())
            .unwrap();
        repo.save(default_user.clone()).await.unwrap();
        let found_default = repo.find_by_id(default_user.uuid).await.unwrap().unwrap();
        assert_eq!(found_default.role(), UserRole::User);
    }

    #[tokio::test]
    async fn test_find_by_email() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Save the user
        repo.save(user.clone()).await.unwrap();

        // Find by email (case insensitive)
        let found_user = repo.find_by_email("TEST@EXAMPLE.COM").await.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.uuid, user.uuid);
    }

    #[tokio::test]
    async fn test_find_by_username() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Save the user
        repo.save(user.clone()).await.unwrap();

        // Find by username
        let found_user = repo.find_by_username("testuser").await.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.uuid, user.uuid);
    }

    #[tokio::test]
    async fn test_find_nonexistent_user() {
        let repo = create_test_repository().await.unwrap();
        let nonexistent_id = Uuid::new_v4();

        let result = repo.find_by_id(nonexistent_id).await.unwrap();
        assert!(result.is_none());

        let result = repo.find_by_email("nonexistent@example.com").await.unwrap();
        assert!(result.is_none());

        let result = repo.find_by_username("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_user() {
        let repo = create_test_repository().await.unwrap();
        let mut user = create_test_user();

        // Save the user
        repo.save(user.clone()).await.unwrap();

        // Update the user
        user.update_username("updateduser".to_string()).unwrap();
        let updated_user = repo.update(user.clone()).await.unwrap();
        assert_eq!(updated_user.username(), "updateduser");

        // Verify the update persisted
        let found_user = repo.find_by_id(user.uuid).await.unwrap().unwrap();
        assert_eq!(found_user.username(), "updateduser");
    }

    #[tokio::test]
    async fn test_delete_user() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Save the user
        repo.save(user.clone()).await.unwrap();

        // Verify user exists
        let found_user = repo.find_by_id(user.uuid).await.unwrap();
        assert!(found_user.is_some());

        // Delete the user
        repo.delete(user.uuid).await.unwrap();

        // Verify user is gone
        let found_user = repo.find_by_id(user.uuid).await.unwrap();
        assert!(found_user.is_none());
    }

    #[tokio::test]
    async fn test_email_exists() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Email should not exist initially
        let exists = repo.email_exists("test@example.com").await.unwrap();
        assert!(!exists);

        // Save the user
        repo.save(user).await.unwrap();

        // Email should exist now
        let exists = repo.email_exists("test@example.com").await.unwrap();
        assert!(exists);

        // Case insensitive check
        let exists = repo.email_exists("TEST@EXAMPLE.COM").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_username_exists() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Username should not exist initially
        let exists = repo.username_exists("testuser").await.unwrap();
        assert!(!exists);

        // Save the user
        repo.save(user).await.unwrap();

        // Username should exist now
        let exists = repo.username_exists("testuser").await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_find_by_active_status() {
        let repo = create_test_repository().await.unwrap();
        let active_user = create_test_user();
        let mut inactive_user = User::new_user(
            "inactiveuser".to_string(),
            Email::new("inactive@example.com".to_string()).unwrap(),
            "hashed_password".to_string(),
            None,
        );
        inactive_user.data.is_active = false;

        // Save users
        repo.save(active_user.clone()).await.unwrap();
        repo.save(inactive_user.clone()).await.unwrap();

        // Find active users
        let active_users = repo.find_by_active_status(true).await.unwrap();
        assert_eq!(active_users.len(), 1);
        assert_eq!(active_users[0].uuid, active_user.uuid);

        // Find inactive users
        let inactive_users = repo.find_by_active_status(false).await.unwrap();
        assert_eq!(inactive_users.len(), 1);
        assert_eq!(inactive_users[0].uuid, inactive_user.uuid);
    }

    #[tokio::test]
    async fn test_find_by_verification_status() {
        let repo = create_test_repository().await.unwrap();
        let mut verified_user = User::new_user(
            "verifieduser".to_string(),
            Email::new("verified@example.com".to_string()).unwrap(),
            "hashed_password".to_string(),
            None,
        );
        verified_user.data.is_verified = true; // Make this user verified

        let unverified_user = User::new_user(
            "unverifieduser".to_string(),
            Email::new("unverified@example.com".to_string()).unwrap(),
            "hashed_password".to_string(),
            None,
        );
        // unverified_user is not verified by default

        // Save users
        repo.save(verified_user.clone()).await.unwrap();
        repo.save(unverified_user.clone()).await.unwrap();

        // Find verified users
        let verified_users = repo.find_by_verification_status(true).await.unwrap();
        assert_eq!(verified_users.len(), 1);
        assert_eq!(verified_users[0].uuid, verified_user.uuid);

        // Find unverified users
        let unverified_users = repo.find_by_verification_status(false).await.unwrap();
        assert_eq!(unverified_users.len(), 1);
        assert_eq!(unverified_users[0].uuid, unverified_user.uuid);
    }

    #[tokio::test]
    async fn test_count_users() {
        let repo = create_test_repository().await.unwrap();

        // Initially no users
        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 0);

        // Add some users
        let user1 = create_test_user();
        let user2 = User::new_user(
            "user2".to_string(),
            Email::new("user2@example.com".to_string()).unwrap(),
            "hashed_password".to_string(),
            None,
        );

        repo.save(user1).await.unwrap();
        repo.save(user2).await.unwrap();

        // Count should be 2
        let count = repo.count_users().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_migration_creates_tables() {
        let repo = create_test_repository().await.unwrap();

        // Try to query the users table - if migration worked, this should succeed
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
                .fetch_one(&repo.pool)
                .await;

        assert!(result.is_ok());
        let row = result.unwrap();
        let table_name: String = row.try_get("name").unwrap();
        assert_eq!(table_name, "users");
    }

    #[tokio::test]
    async fn test_row_to_user_conversion() {
        let repo = create_test_repository().await.unwrap();
        let user = create_test_user();

        // Save user to get a row
        repo.save(user.clone()).await.unwrap();

        // Query the row back
        let row = sqlx::query("SELECT * FROM users WHERE uuid = ?")
            .bind(user.uuid.to_string())
            .fetch_one(&repo.pool)
            .await
            .unwrap();

        // Convert row to user
        let converted_user = repo.row_to_user(&row).unwrap();

        // Verify conversion
        assert_eq!(converted_user.uuid, user.uuid);
        assert_eq!(converted_user.username(), user.username());
        assert_eq!(converted_user.email().value(), user.email().value());
        assert_eq!(converted_user.password_hash(), user.password_hash());
        assert_eq!(converted_user.is_active(), user.is_active());
        assert_eq!(converted_user.is_verified(), user.is_verified());
    }
}
