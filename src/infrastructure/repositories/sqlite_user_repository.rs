/*
SQLite User Repository Adapter

Concrete implementation of UserRepositoryPort using SQLite database.
This adapter handles the actual database operations for user persistence.
*/

use crate::application::storage::user_store::UserRepositoryPort;
use crate::core::platform::container::user::{User, UserData, Email, UserProfile, UserError};
use crate::config::application_settings::Settings;
use async_trait::async_trait;
use sqlx::{SqlitePool, Row, sqlite::SqlitePoolOptions};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::str::FromStr;

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    /// Create a new SQLite user repository
    pub async fn new(settings: &Settings) -> Result<Self, UserError> {
        let database_url = &settings.server.database_url;
        
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Failed to connect to database: {}", e)))?;

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
        let uuid_str: String = row.try_get("uuid")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get uuid: {}", e)))?;
        let uuid = Uuid::from_str(&uuid_str)
            .map_err(|e| UserError::RepositoryError(format!("Invalid UUID: {}", e)))?;

        let email_str: String = row.try_get("email")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get email: {}", e)))?;
        let email = Email::new(email_str)?;

        let created_str: String = row.try_get("created_at")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get created_at: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| UserError::RepositoryError(format!("Invalid created_at: {}", e)))?
            .with_timezone(&Utc);

        let modified_str: String = row.try_get("modified_at")
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
            username: row.try_get("username")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get username: {}", e)))?,
            email,
            password_hash: row.try_get("password_hash")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get password_hash: {}", e)))?,
            is_active: row.try_get("is_active")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get is_active: {}", e)))?,
            is_verified: row.try_get("is_verified")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get is_verified: {}", e)))?,
            profile,
        };

        let mut user = User {
            uuid,
            version: row.try_get("version")
                .map_err(|e| UserError::RepositoryError(format!("Failed to get version: {}", e)))?,
            created: created_at,
            modified: modified_at,
            name: row.try_get("username").ok(), // or use another field if appropriate
            node: user_data,
        };

        Ok(user)
    }

    /// Convert User to database parameters
    fn user_to_params(&self, user: &User) -> Vec<(&str, Box<dyn sqlx::Encode<'_, sqlx::Sqlite> + Send>)> {
        vec![
            ("uuid", Box::new(user.uuid.to_string())),
            ("version", Box::new(user.version as i64)),
            ("created_at", Box::new(user.created.to_rfc3339())),
            ("modified_at", Box::new(user.modified.to_rfc3339())),
            ("title", Box::new(user.title.clone())),
            ("username", Box::new(user.node.username.clone())),
            ("email", Box::new(user.node.email.value().to_string())),
            ("password_hash", Box::new(user.node.password_hash.clone())),
            ("is_active", Box::new(user.node.is_active)),
            ("is_verified", Box::new(user.node.is_verified)),
            ("first_name", Box::new(user.node.profile.first_name.clone())),
            ("last_name", Box::new(user.node.profile.last_name.clone())),
            ("bio", Box::new(user.node.profile.bio.clone())),
            ("avatar_url", Box::new(user.node.profile.avatar_url.clone())),
            ("timezone", Box::new(user.node.profile.timezone.clone())),
            ("locale", Box::new(user.node.profile.locale.clone())),
        ]
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
                first_name, last_name, bio, avatar_url, timezone, locale
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.uuid.to_string()) // Use UUID as primary key
        .bind(user.uuid.to_string())
        .bind(user.version as i64)
        .bind(user.created.to_rfc3339())
        .bind(user.modified.to_rfc3339())
        .bind(&user.title)
        .bind(&user.node.username)
        .bind(user.node.email.value())
        .bind(&user.node.password_hash)
        .bind(user.node.is_active)
        .bind(user.node.is_verified)
        .bind(&user.node.profile.first_name)
        .bind(&user.node.profile.last_name)
        .bind(&user.node.profile.bio)
        .bind(&user.node.profile.avatar_url)
        .bind(&user.node.profile.timezone)
        .bind(&user.node.profile.locale)
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
                first_name = ?, last_name = ?, bio = ?,
                avatar_url = ?, timezone = ?, locale = ?
            WHERE uuid = ?
            "#,
        )
        .bind(user.version as i64)
        .bind(user.modified.to_rfc3339())
        .bind(&user.title)
        .bind(&user.node.username)
        .bind(user.node.email.value())
        .bind(&user.node.password_hash)
        .bind(user.node.is_active)
        .bind(user.node.is_verified)
        .bind(&user.node.profile.first_name)
        .bind(&user.node.profile.last_name)
        .bind(&user.node.profile.bio)
        .bind(&user.node.profile.avatar_url)
        .bind(&user.node.profile.timezone)
        .bind(&user.node.profile.locale)
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

        let count: i64 = row.try_get("count")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get count: {}", e)))?;

        Ok(count > 0)
    }

    async fn username_exists(&self, username: &str) -> Result<bool, UserError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| UserError::RepositoryError(format!("Database query failed: {}", e)))?;

        let count: i64 = row.try_get("count")
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

        let count: i64 = row.try_get("count")
            .map_err(|e| UserError::RepositoryError(format!("Failed to get count: {}", e)))?;

        Ok(count as u64)
    }
}