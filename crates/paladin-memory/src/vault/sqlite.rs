//! SQLite-backed `VaultPort` implementation (D-18, D-23).
//!
//! Rides the SAME shared embedded migrator plan 26-07 introduced for
//! `SqliteGarrison` (`crate::migrations::MIGRATOR`) -- one static, one
//! `migrations/` directory, one numbering sequence across every table this
//! crate persists, regardless of which adapter owns that table. `003` (this
//! module's schema) is numbered after `002` (the Garrison's `is_summary`
//! column) in that one shared sequence.
//!
//! Pointing a `SqliteVault` and a `SqliteGarrison` at the SAME database file
//! is safe: each adapter's `initialize`/`new` call runs the whole embedded
//! migrator, so the file simply gains both adapters' tables regardless of
//! which one constructed first. Pointing them at separate files is also
//! safe -- each file just gains a few empty tables it never uses. Both
//! outcomes are harmless because every `CREATE TABLE`/`CREATE INDEX` in this
//! crate's migrations is `IF NOT EXISTS` (D-23).
//!
//! `search` is not implemented here and falls back to `VaultPort::search`'s
//! default `Unsupported` response -- reach for `crate::vault::SemanticVault`
//! when search is actually needed.

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use paladin_core::platform::container::vault::{DEFAULT_MAX_VALUE_BYTES, Namespace, VaultRecord};
use paladin_ports::output::vault_port::{Page, VaultError, VaultPort};
use sqlx::Row;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteRow,
};

use crate::vault::redact::redact_and_bound;

/// SQLite-backed `VaultPort` implementation, behind the crate's existing
/// `sqlite` feature (no new cargo feature, D-18).
///
/// # Construction
///
/// Use [`SqliteVault::new`] -- it creates the database file if it does not
/// already exist and runs any pending migrations automatically, safe to
/// call more than once against the same file (the embedded migrator tracks
/// applied versions itself; the `SqliteGarrison` / `SqliteWaypointStore`
/// "constructs twice idempotently" precedent).
///
/// # Namespace column
///
/// `ns` is stored as `namespace.to_string()`, the `/`-joined segment path.
/// This is unambiguous **because** a [`Namespace`] segment can never
/// contain `/` (validated at construction), so the joined form round-trips
/// to the exact same segment list -- two distinct namespaces can never
/// collide on their joined string.
///
/// # Examples
///
/// ```no_run
/// use paladin_memory::vault::SqliteVault;
/// use paladin_core::platform::container::vault::Namespace;
/// use paladin_ports::output::vault_port::VaultPort;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let vault = SqliteVault::new("./vault.db").await?;
/// let ns = Namespace::parse("user/alice")?;
/// vault.put(&ns, "favorite_color", json!("blue")).await?;
/// # Ok(())
/// # }
/// ```
pub struct SqliteVault {
    pool: SqlitePool,
    max_value_bytes: usize,
}

impl SqliteVault {
    /// Connects to (creating if missing) the SQLite database at `path` and
    /// runs any pending migrations, using [`DEFAULT_MAX_VALUE_BYTES`] as the
    /// value-size bound.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Storage`] if the connection or migration
    /// fails.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, VaultError> {
        Self::new_with_max_value_bytes(path, DEFAULT_MAX_VALUE_BYTES).await
    }

    /// Like [`SqliteVault::new`], but validates against an explicit
    /// `max_value_bytes` bound rather than [`DEFAULT_MAX_VALUE_BYTES`].
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::Storage`] if the connection or migration
    /// fails.
    pub async fn new_with_max_value_bytes(
        path: impl AsRef<Path>,
        max_value_bytes: usize,
    ) -> Result<Self, VaultError> {
        let path_str = path.as_ref().to_str().ok_or_else(|| VaultError::Storage {
            message: "invalid database path (not valid UTF-8)".to_string(),
        })?;

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{path_str}"))
            .map_err(|e| VaultError::Storage {
                message: redact_and_bound(&e.to_string()),
            })?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| VaultError::Storage {
                message: redact_and_bound(&e.to_string()),
            })?;

        // Run migrations via the crate's one shared, compile-time-embedded
        // migrator (D-17, D-23) -- the same static SqliteGarrison calls.
        crate::migrations::run_migrations(&pool)
            .await
            .map_err(|e| VaultError::Storage {
                message: redact_and_bound(&e.to_string()),
            })?;

        Ok(Self {
            pool,
            max_value_bytes,
        })
    }

    /// Reconstructs a [`VaultRecord`] from a fetched row plus the `ns`/`key`
    /// already known to the caller (the row itself may or may not carry a
    /// `key` column, so it is passed explicitly rather than re-read).
    ///
    /// This deserializes through `VaultRecord`'s `serde` implementation
    /// (private fields, `#[derive(Serialize, Deserialize)]`) rather than
    /// through a public constructor, because every public constructor
    /// stamps `created_at`/`updated_at` to `Utc::now()` -- there is no
    /// public way to rebuild a record with its actual, previously persisted
    /// timestamps. The value has already passed every invariant check at
    /// `put` time, so re-validating here would be redundant.
    fn row_to_record(
        &self,
        ns: &Namespace,
        key: &str,
        row: &SqliteRow,
    ) -> Result<VaultRecord, VaultError> {
        let value_json: String = row.try_get("value").map_err(|e| VaultError::Storage {
            message: redact_and_bound(&e.to_string()),
        })?;
        let value: serde_json::Value =
            serde_json::from_str(&value_json).map_err(|e| VaultError::Serialization {
                message: redact_and_bound(&e.to_string()),
            })?;
        let created_at: String = row.try_get("created_at").map_err(|e| VaultError::Storage {
            message: redact_and_bound(&e.to_string()),
        })?;
        let updated_at: String = row.try_get("updated_at").map_err(|e| VaultError::Storage {
            message: redact_and_bound(&e.to_string()),
        })?;

        let record_json = serde_json::json!({
            "namespace": ns.segments(),
            "key": key,
            "value": value,
            "created_at": created_at,
            "updated_at": updated_at,
        });

        serde_json::from_value(record_json).map_err(|e| VaultError::Serialization {
            message: redact_and_bound(&e.to_string()),
        })
    }
}

#[async_trait]
impl VaultPort for SqliteVault {
    // RED stub (plan 26-09 Task 1, TDD): these four methods do not yet touch
    // `vault_records` at all -- every shared contract clause that exercises a
    // real write/read round trip must fail against this stub before the GREEN
    // commit that follows immediately replaces these bodies with the real
    // parameter-bound SQL.
    async fn put(&self, ns: &Namespace, key: &str, value: serde_json::Value) -> Result<(), VaultError> {
        let _ = (ns, key, value);
        Ok(())
    }

    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
        let _ = (ns, key);
        Ok(None)
    }

    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
        let _ = (ns, key);
        Ok(false)
    }

    async fn list(&self, ns: &Namespace, prefix: Option<&str>, page: Page) -> Result<Vec<VaultRecord>, VaultError> {
        let _ = (ns, prefix, page);
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::contract_tests;
    use serde_json::json;
    use tempfile::NamedTempFile;

    async fn new_test_vault() -> (SqliteVault, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let vault = SqliteVault::new(temp_file.path()).await.unwrap();
        (vault, temp_file)
    }

    #[tokio::test]
    async fn put_then_get_returns_the_record() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::put_then_get_returns_the_record(&vault).await;
    }

    #[tokio::test]
    async fn put_overwrites_and_preserves_created_at() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::put_overwrites_and_preserves_created_at(&vault).await;
    }

    #[tokio::test]
    async fn delete_returns_true_then_false() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::delete_returns_true_then_false(&vault).await;
    }

    #[tokio::test]
    async fn list_returns_only_this_namespace_ordered_by_key() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::list_returns_only_this_namespace_ordered_by_key(&vault).await;
    }

    /// Named to match the plan's own `<verify>`/Behavior wording verbatim
    /// (Test 4: `list_scopes_to_exactly_the_namespace`) -- same shared
    /// contract clause as the test above.
    #[tokio::test]
    async fn list_scopes_to_exactly_the_namespace() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::list_returns_only_this_namespace_ordered_by_key(&vault).await;
    }

    #[tokio::test]
    async fn list_filters_by_key_prefix() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::list_filters_by_key_prefix(&vault).await;
    }

    #[tokio::test]
    async fn list_paginates_by_opaque_after_cursor() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::list_paginates_by_opaque_after_cursor(&vault).await;
    }

    #[tokio::test]
    async fn list_on_an_empty_namespace_is_an_empty_page_not_an_error() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::list_on_an_empty_namespace_is_an_empty_page_not_an_error(&vault).await;
    }

    #[tokio::test]
    async fn namespaces_are_isolated() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::namespaces_are_isolated(&vault).await;
    }

    #[tokio::test]
    async fn value_larger_than_the_bound_is_rejected() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::value_larger_than_the_bound_is_rejected(&vault).await;
    }

    #[tokio::test]
    async fn search_is_unsupported_on_sqlite() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::assert_search_unsupported(&vault).await;
    }

    #[tokio::test]
    async fn run_all_shared_clauses_smoke_aggregate() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::run_all_shared_clauses(&vault).await;
    }

    #[tokio::test]
    async fn sqlite_vault_passes_the_shared_contract_suite() {
        let (vault, _f) = new_test_vault().await;
        contract_tests::run_all_shared_clauses(&vault).await;
        contract_tests::assert_search_unsupported(&vault).await;
    }

    #[tokio::test]
    async fn sqlite_vault_constructs_twice_idempotently() {
        let temp_file = NamedTempFile::new().unwrap();

        let first = SqliteVault::new(temp_file.path()).await.unwrap();
        drop(first);

        let second = SqliteVault::new(temp_file.path()).await.unwrap();

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&second.pool)
            .await
            .unwrap();
        assert_eq!(
            row.0, 3,
            "expected exactly one row per migration (001, 002, 003)"
        );
    }

    #[tokio::test]
    async fn vault_and_garrison_share_one_migrator() {
        use crate::garrison::SqliteGarrison;
        use paladin_core::platform::container::garrison::GarrisonConfig;

        // A database constructed by SqliteVault::new gains garrison_entries too.
        {
            let (vault, _f) = new_test_vault().await;
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'garrison_entries'",
            )
            .fetch_optional(&vault.pool)
            .await
            .unwrap();
            assert!(
                row.is_some(),
                "garrison_entries must exist even though only SqliteVault was constructed"
            );
        }

        // And vice versa: a database constructed by SqliteGarrison::connect gains
        // vault_records too. `SqliteGarrison`'s pool is private to its own module,
        // so this opens a fresh connection to the same file to check.
        let temp_file_2 = NamedTempFile::new().unwrap();
        {
            let garrison = SqliteGarrison::connect(
                temp_file_2.path(),
                GarrisonConfig::default(),
                "test-paladin",
            )
            .await
            .unwrap();
            drop(garrison);
        }

        let check_pool = SqlitePool::connect(&format!("sqlite://{}", temp_file_2.path().display()))
            .await
            .unwrap();
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'vault_records'",
        )
        .fetch_optional(&check_pool)
        .await
        .unwrap();
        assert!(
            row.is_some(),
            "vault_records must exist even though only SqliteGarrison was constructed"
        );
    }

    #[tokio::test]
    async fn storage_errors_are_typed_and_redacted() {
        // The redaction half: any credential-shaped token in adapter-boundary
        // error text must be gone before the text is bounded (D-34).
        let raw = "connection failed: Bearer sk-should-never-appear-1234567890 rejected";
        let redacted = redact_and_bound(raw);
        assert!(!redacted.contains("sk-should-never-appear-1234567890"));
        assert!(redacted.contains("[REDACTED]"));

        // The typed half: a forced backend failure surfaces as a typed
        // VaultError::Storage, never a panic or an untyped error.
        let (vault, _f) = new_test_vault().await;
        vault.pool.close().await;

        let ns = Namespace::parse("contract-forced-failure/ns").unwrap();
        let err = vault.put(&ns, "k", json!(1)).await.unwrap_err();
        assert!(matches!(err, VaultError::Storage { .. }));
    }
}
