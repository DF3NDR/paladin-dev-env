//! SQLite-based persistent garrison implementation.
//!
//! Provides durable storage for conversation history with:
//! - Connection pooling for concurrent access
//! - Automatic migrations
//! - Full-text search
//! - Optional vector embeddings support
//! - Eviction strategies with persistence

use async_trait::async_trait;
use paladin_core::platform::container::garrison::{
    ConversationRole, GarrisonConfig, GarrisonEntry,
};
use paladin_ports::output::garrison_port::{GarrisonError, GarrisonPort, GarrisonStats};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

/// SQLite-backed Garrison adapter with persistent conversation history.
///
/// Stores conversation entries in a SQLite database with support for:
/// - Full-text search using FTS5
/// - Automatic eviction based on the configured [`GarrisonConfig`] strategy
/// - Connection pooling for concurrent performance
///
/// Prefer [`crate::garrison::InMemoryGarrison`] for ephemeral/test use cases
/// where persistence is not required.  Use `SqliteGarrison` when conversation
/// history must survive process restarts.
///
/// # Construction
///
/// Use [`SqliteGarrison::connect`] — it creates the database file if it does
/// not already exist and runs pending migrations automatically.
///
/// # Examples
///
/// ```no_run
/// use paladin_memory::garrison::SqliteGarrison;
/// use paladin_core::platform::container::garrison::GarrisonConfig;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let garrison = SqliteGarrison::connect(
///     "./garrison.db",
///     GarrisonConfig::default(),
///     "paladin-001"
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SqliteGarrison {
    pool: SqlitePool,
    config: GarrisonConfig,
    paladin_id: String,
}

impl SqliteGarrison {
    /// Connect to a SQLite database at the specified path.
    ///
    /// Creates the database file if it does not already exist, then runs any
    /// pending migrations and initialises per-Paladin metadata.
    ///
    /// # Arguments
    ///
    /// * `path` - Filesystem path for the SQLite database file
    /// * `config` - Garrison configuration for entry limits and eviction
    /// * `paladin_id` - Unique identifier for this Paladin's conversation scope
    ///
    /// # Errors
    ///
    /// Returns [`GarrisonError::StorageError`] if the connection or migration fails.
    pub async fn connect(
        path: impl AsRef<Path>,
        config: GarrisonConfig,
        paladin_id: impl Into<String>,
    ) -> Result<Self, GarrisonError> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| GarrisonError::ConfigurationError("Invalid database path".into()))?;

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path_str))
            .map_err(|e| GarrisonError::StorageError(format!("Connection options error: {}", e)))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(30));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| GarrisonError::StorageError(format!("Connection failed: {}", e)))?;

        let garrison = Self {
            pool,
            config,
            paladin_id: paladin_id.into(),
        };

        garrison.initialize().await?;
        Ok(garrison)
    }

    /// Initialize the database schema and metadata
    async fn initialize(&self) -> Result<(), GarrisonError> {
        // Run migrations via the crate's one shared, compile-time-embedded
        // migrator (D-17, D-23) -- no longer relative to the process CWD.
        crate::migrations::run_migrations(&self.pool)
            .await
            .map_err(|e| GarrisonError::StorageError(format!("Migration failed: {}", e)))?;

        // Initialize metadata for this paladin if not exists
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO garrison_metadata
            (paladin_id, max_entries, max_tokens, eviction_strategy, preserve_recent_count)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&self.paladin_id)
        .bind(self.config.max_entries as i64)
        .bind(self.config.max_tokens.map(|t| t as i64))
        .bind(format!("{:?}", self.config.eviction_strategy))
        .bind(self.config.preserve_recent_count as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Metadata init failed: {}", e)))?;

        Ok(())
    }

    /// Apply eviction strategy to maintain configured limits
    async fn apply_eviction(&self) -> Result<(), GarrisonError> {
        let stats = self.stats().await?;

        // Check if eviction is needed
        let needs_eviction = if stats.entry_count > self.config.max_entries {
            true
        } else if let Some(max_tokens) = self.config.max_tokens {
            stats.total_tokens > max_tokens
        } else {
            false
        };

        if !needs_eviction {
            return Ok(());
        }

        // Calculate how many entries to keep (not remove)
        // Use the smaller of max_entries and preserve_recent_count to ensure we don't exceed limits
        let target_count =
            std::cmp::min(self.config.max_entries, self.config.preserve_recent_count);

        match self.config.eviction_strategy {
            paladin_core::platform::container::garrison::EvictionStrategy::FIFO => {
                // Remove oldest entries (FIFO)
                sqlx::query(
                    r#"
                    DELETE FROM garrison_entries
                    WHERE paladin_id = ?
                    AND id NOT IN (
                        SELECT id FROM garrison_entries
                        WHERE paladin_id = ?
                        ORDER BY timestamp DESC
                        LIMIT ?
                    )
                    "#,
                )
                .bind(&self.paladin_id)
                .bind(&self.paladin_id)
                .bind(target_count as i64)
                .execute(&self.pool)
                .await
                .map_err(|e| GarrisonError::StorageError(format!("FIFO eviction failed: {}", e)))?;
            }
            paladin_core::platform::container::garrison::EvictionStrategy::ImportanceBased => {
                // Preserve system messages and recent entries
                sqlx::query(
                    r#"
                    DELETE FROM garrison_entries
                    WHERE paladin_id = ?
                    AND role != 'system'
                    AND id NOT IN (
                        SELECT id FROM garrison_entries
                        WHERE paladin_id = ?
                        ORDER BY timestamp DESC
                        LIMIT ?
                    )
                    "#,
                )
                .bind(&self.paladin_id)
                .bind(&self.paladin_id)
                .bind(target_count as i64)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    GarrisonError::StorageError(format!("Importance eviction failed: {}", e))
                })?;
            }
            paladin_core::platform::container::garrison::EvictionStrategy::SlidingWindow => {
                // Keep only the most recent entries
                sqlx::query(
                    r#"
                    DELETE FROM garrison_entries
                    WHERE paladin_id = ?
                    AND id NOT IN (
                        SELECT id FROM garrison_entries
                        WHERE paladin_id = ?
                        ORDER BY timestamp DESC
                        LIMIT ?
                    )
                    "#,
                )
                .bind(&self.paladin_id)
                .bind(&self.paladin_id)
                .bind(target_count as i64)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    GarrisonError::StorageError(format!("Sliding window eviction failed: {}", e))
                })?;
            }
        }

        // Update metadata
        self.update_metadata().await?;

        Ok(())
    }

    /// Update garrison metadata after changes
    async fn update_metadata(&self) -> Result<(), GarrisonError> {
        // Calculate stats inline to avoid circular dependency
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as entry_count,
                COALESCE(SUM(token_count), 0) as total_tokens
            FROM garrison_entries
            WHERE paladin_id = ?
            "#,
        )
        .bind(&self.paladin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Stats query failed: {}", e)))?;

        let entry_count: i64 = row
            .try_get("entry_count")
            .map_err(|e| GarrisonError::SerializationError(format!("Entry count error: {}", e)))?;
        let total_tokens: i64 = row
            .try_get("total_tokens")
            .map_err(|e| GarrisonError::SerializationError(format!("Total tokens error: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE garrison_metadata
            SET total_entries = ?,
                total_tokens = ?,
                last_eviction = datetime('now'),
                updated_at = datetime('now')
            WHERE paladin_id = ?
            "#,
        )
        .bind(entry_count)
        .bind(total_tokens)
        .bind(&self.paladin_id)
        .execute(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Metadata update failed: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl GarrisonPort for SqliteGarrison {
    async fn remember(&self, entry: GarrisonEntry) -> Result<(), GarrisonError> {
        // Validate entry
        entry
            .validate()
            .map_err(GarrisonError::ConfigurationError)?;

        // Insert entry
        sqlx::query(
            r#"
            INSERT INTO garrison_entries
            (id, paladin_id, role, content, timestamp, token_count, metadata, is_summary, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
        )
        .bind(entry.id.to_string())
        .bind(&self.paladin_id)
        .bind(format!("{:?}", entry.role).to_lowercase())
        .bind(&entry.content)
        .bind(entry.timestamp.to_rfc3339())
        .bind(entry.token_count.map(|t| t as i64))
        .bind(serde_json::to_string(&entry.metadata).ok())
        .bind(entry.is_summary)
        .execute(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Insert failed: {}", e)))?;

        // Apply eviction if needed
        self.apply_eviction().await?;

        Ok(())
    }

    async fn recall_recent(&self, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        let rows = sqlx::query(
            r#"
            SELECT id, role, content, timestamp, token_count, metadata, is_summary
            FROM garrison_entries
            WHERE paladin_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&self.paladin_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Recall failed: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let role_str: String = row.try_get("role").map_err(|e| {
                GarrisonError::SerializationError(format!("Role parse error: {}", e))
            })?;
            let role = match role_str.as_str() {
                "system" => ConversationRole::System,
                "user" => ConversationRole::User,
                "assistant" => ConversationRole::Assistant,
                "tool" => ConversationRole::Tool,
                _ => ConversationRole::User,
            };

            let id: String = row
                .try_get("id")
                .map_err(|e| GarrisonError::SerializationError(format!("ID parse error: {}", e)))?;
            let content: String = row.try_get("content").map_err(|e| {
                GarrisonError::SerializationError(format!("Content parse error: {}", e))
            })?;
            let timestamp_str: String = row.try_get("timestamp").map_err(|e| {
                GarrisonError::SerializationError(format!("Timestamp parse error: {}", e))
            })?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| {
                    GarrisonError::SerializationError(format!("Timestamp conversion error: {}", e))
                })?
                .with_timezone(&chrono::Utc);

            let token_count: Option<i64> = row.try_get("token_count").ok();
            let metadata_str: Option<String> = row.try_get("metadata").ok();
            let metadata = metadata_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let is_summary: i64 = row.try_get("is_summary").unwrap_or(0);

            let mut entry = GarrisonEntry::new(role, content);
            entry.id = uuid::Uuid::parse_str(&id)
                .map_err(|e| GarrisonError::SerializationError(format!("UUID parse: {}", e)))?;
            entry.timestamp = timestamp;
            entry.token_count = token_count.map(|t| t as u32);
            entry.metadata = metadata;
            entry.is_summary = is_summary != 0;

            entries.push(entry);
        }

        // Reverse to maintain chronological order (oldest first)
        entries.reverse();

        Ok(entries)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<GarrisonEntry>, GarrisonError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT e.id, e.role, e.content, e.timestamp, e.token_count, e.metadata, e.is_summary
            FROM garrison_entries e
            JOIN garrison_search s ON e.rowid = s.rowid
            WHERE e.paladin_id = ? AND garrison_search MATCH ?
            ORDER BY e.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(&self.paladin_id)
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Search failed: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            let role_str: String = row.try_get("role").map_err(|e| {
                GarrisonError::SerializationError(format!("Role parse error: {}", e))
            })?;
            let role = match role_str.as_str() {
                "system" => ConversationRole::System,
                "user" => ConversationRole::User,
                "assistant" => ConversationRole::Assistant,
                "tool" => ConversationRole::Tool,
                _ => ConversationRole::User,
            };

            let id: String = row
                .try_get("id")
                .map_err(|e| GarrisonError::SerializationError(format!("ID parse error: {}", e)))?;
            let content: String = row.try_get("content").map_err(|e| {
                GarrisonError::SerializationError(format!("Content parse error: {}", e))
            })?;
            let timestamp_str: String = row.try_get("timestamp").map_err(|e| {
                GarrisonError::SerializationError(format!("Timestamp parse error: {}", e))
            })?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| {
                    GarrisonError::SerializationError(format!("Timestamp conversion error: {}", e))
                })?
                .with_timezone(&chrono::Utc);

            let token_count: Option<i64> = row.try_get("token_count").ok();
            let metadata_str: Option<String> = row.try_get("metadata").ok();
            let metadata = metadata_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let is_summary: i64 = row.try_get("is_summary").unwrap_or(0);

            let mut entry = GarrisonEntry::new(role, content);
            entry.id = uuid::Uuid::parse_str(&id)
                .map_err(|e| GarrisonError::SerializationError(format!("UUID parse: {}", e)))?;
            entry.timestamp = timestamp;
            entry.token_count = token_count.map(|t| t as u32);
            entry.metadata = metadata;
            entry.is_summary = is_summary != 0;

            entries.push(entry);
        }

        Ok(entries)
    }

    async fn forget_all(&self) -> Result<(), GarrisonError> {
        sqlx::query("DELETE FROM garrison_entries WHERE paladin_id = ?")
            .bind(&self.paladin_id)
            .execute(&self.pool)
            .await
            .map_err(|e| GarrisonError::StorageError(format!("Delete all failed: {}", e)))?;

        self.update_metadata().await?;

        Ok(())
    }

    async fn stats(&self) -> Result<GarrisonStats, GarrisonError> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as entry_count,
                COALESCE(SUM(token_count), 0) as total_tokens
            FROM garrison_entries
            WHERE paladin_id = ?
            "#,
        )
        .bind(&self.paladin_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Stats query failed: {}", e)))?;

        let entry_count: i64 = row
            .try_get("entry_count")
            .map_err(|e| GarrisonError::SerializationError(format!("Entry count error: {}", e)))?;
        let total_tokens: i64 = row
            .try_get("total_tokens")
            .map_err(|e| GarrisonError::SerializationError(format!("Total tokens error: {}", e)))?;

        // Get database file size (approximate)
        let size_row = sqlx::query(
            "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| GarrisonError::StorageError(format!("Size query failed: {}", e)))?;

        let size_bytes = size_row
            .and_then(|r| r.try_get::<i64, _>("size").ok())
            .map(|s| s as u64);

        Ok(GarrisonStats {
            entry_count: entry_count as usize,
            total_tokens: total_tokens as u32,
            size_bytes: size_bytes.map(|s| s as usize),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite_garrison_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();

        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        let stats = garrison.stats().await.unwrap();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_tokens, 0);
    }

    #[tokio::test]
    async fn test_sqlite_remember_and_recall() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        let entry = GarrisonEntry::new(ConversationRole::User, "Test message".to_string());
        garrison.remember(entry).await.unwrap();

        let entries = garrison.recall_recent(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Test message");
    }

    #[tokio::test]
    async fn test_sqlite_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();

        // First connection - add entry
        {
            let garrison =
                SqliteGarrison::connect(temp_file.path(), config.clone(), "test-paladin")
                    .await
                    .unwrap();

            let entry =
                GarrisonEntry::new(ConversationRole::User, "Persistent message".to_string());
            garrison.remember(entry).await.unwrap();
        }

        // Second connection - verify persistence
        {
            let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
                .await
                .unwrap();

            let entries = garrison.recall_recent(10).await.unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].content, "Persistent message");
        }
    }

    #[tokio::test]
    async fn test_sqlite_search() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Hello world".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Goodbye world".to_string(),
            ))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Random message".to_string(),
            ))
            .await
            .unwrap();

        let results = garrison.search("world", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_sqlite_eviction() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::new(3, None);
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        for i in 0..5 {
            garrison
                .remember(GarrisonEntry::new(
                    ConversationRole::User,
                    format!("Message {}", i),
                ))
                .await
                .unwrap();
        }

        let stats = garrison.stats().await.unwrap();
        assert!(stats.entry_count <= 3);
    }

    // RT-03 / D-17: one embedded migrator, the `002` column migration, and the
    // Garrison entry's `is_summary` round trip.

    #[tokio::test]
    async fn fresh_sqlite_garrison_has_the_is_summary_column() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        let rows = sqlx::query("PRAGMA table_info(garrison_entries)")
            .fetch_all(&garrison.pool)
            .await
            .unwrap();

        let is_summary_column = rows.iter().find(|row| {
            row.try_get::<String, _>("name")
                .map(|n| n == "is_summary")
                .unwrap_or(false)
        });
        assert!(
            is_summary_column.is_some(),
            "garrison_entries is missing the is_summary column"
        );

        let default_value: Option<String> = is_summary_column.unwrap().try_get("dflt_value").ok();
        assert_eq!(default_value.as_deref(), Some("0"));
    }

    #[tokio::test]
    async fn existing_v0_9_database_migrates_forward() {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_file_path = temp_file.path().to_path_buf();
        let url = format!("sqlite://{}", temp_file_path.display());

        let options = SqliteConnectOptions::from_str(&url)
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // Build a v0.9-shaped database: apply exactly `001`'s raw SQL directly (no
        // migrator involved, so no `_sqlx_migrations` bookkeeping exists yet) --
        // exactly the schema a pre-`is_summary` deployment would have had.
        let v0_9_schema = include_str!("../../migrations/001_create_garrison_tables.sql");
        sqlx::raw_sql(v0_9_schema).execute(&pool).await.unwrap();

        // Insert a pre-existing row the way v0.9 code would have (no `is_summary`
        // column exists yet at this point).
        sqlx::query(
            r#"
            INSERT INTO garrison_entries
            (id, paladin_id, role, content, timestamp, token_count, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            "#,
        )
        .bind("11111111-1111-1111-1111-111111111111")
        .bind("test-paladin")
        .bind("user")
        .bind("pre-existing v0.9 row")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(Option::<i64>::None)
        .bind(Option::<String>::None)
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        // Construct SqliteGarrison against the SAME file: `002` must apply and the
        // pre-existing row must survive with `is_summary == false`.
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(&temp_file_path, config, "test-paladin")
            .await
            .unwrap();

        let entries = garrison.recall_recent(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "pre-existing v0.9 row");
        assert!(
            !entries[0].is_summary,
            "pre-existing row must read is_summary == false"
        );
    }

    #[tokio::test]
    async fn sqlite_garrison_constructs_twice_idempotently() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();

        let first = SqliteGarrison::connect(temp_file.path(), config.clone(), "test-paladin")
            .await
            .unwrap();
        drop(first);

        let second = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&second.pool)
            .await
            .unwrap();
        let expected = crate::migrations::MIGRATOR.iter().count() as i64;
        assert_eq!(
            row.0, expected,
            "expected exactly one _sqlx_migrations row per embedded migration ({expected}), never a duplicate from the second construction"
        );
    }

    #[tokio::test]
    async fn is_summary_round_trips_through_sqlite() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        garrison
            .remember(GarrisonEntry::summary("condensed history".to_string()))
            .await
            .unwrap();
        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "raw entry".to_string(),
            ))
            .await
            .unwrap();

        let entries = garrison.recall_recent(10).await.unwrap();
        assert_eq!(entries.len(), 2);

        let summary_entry = entries
            .iter()
            .find(|e| e.content == "condensed history")
            .unwrap();
        assert!(summary_entry.is_summary);

        let raw_entry = entries.iter().find(|e| e.content == "raw entry").unwrap();
        assert!(!raw_entry.is_summary);
    }

    #[tokio::test]
    async fn test_sqlite_forget_all() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = GarrisonConfig::default();
        let garrison = SqliteGarrison::connect(temp_file.path(), config, "test-paladin")
            .await
            .unwrap();

        garrison
            .remember(GarrisonEntry::new(
                ConversationRole::User,
                "Test".to_string(),
            ))
            .await
            .unwrap();

        garrison.forget_all().await.unwrap();

        let stats = garrison.stats().await.unwrap();
        assert_eq!(stats.entry_count, 0);
    }
}
