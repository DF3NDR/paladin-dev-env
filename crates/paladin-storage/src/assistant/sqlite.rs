/*
SQLite Assistant Repository

Concrete `AssistantRepositoryPort` implementation over SQLite (PLAT-04,
D-28, D-29, D-30). `create` inserts the `assistants` row and its first
`assistant_versions` row inside one transaction (never partially visible).
`append_version` is insert-then-CAS (`append_version_once`): `INSERT INTO
assistant_versions (.., version = ?n+1)` -- a PRIMARY KEY violation on
`(assistant_id, version)` maps to `VersionConflict` via
`is_unique_violation()` BEFORE the generic error wrap (27-RESEARCH.md
Pattern 2, mirroring `run/sqlite.rs`) -- then `UPDATE assistants SET latest
= ?n+1 WHERE assistant_id = ? AND latest = ?n` inside the SAME transaction;
the public `append_version` retries `append_version_once` on
`VersionConflict` up to 20 times. Every statement uses bound parameters or
`sqlx::QueryBuilder`'s `push_bind`; no query string is ever built by
formatting a caller-supplied value into it. Migrations follow the
versioned-file convention at `crates/paladin-storage/migrations/sqlite/`,
embedded at compile time via `sqlx::migrate!` and applied automatically on
construction.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, sqlite::SqliteRow};
use std::str::FromStr;

use paladin_core::platform::container::assistant::{
    ASSISTANT_SCHEMA_VERSION, Assistant, AssistantDefinition, AssistantId, AssistantKind,
    AssistantSource, AssistantVersion, NewAssistantVersion,
};
use paladin_ports::output::assistant_repository_port::{
    AssistantPage, AssistantRepositoryError, AssistantRepositoryPort, AssistantVersionPage,
};

use crate::waypoint::redact::redact_database_url_password;

/// The number of times the public [`AssistantRepositoryPort::append_version`]
/// retries [`SqliteAssistantRepository::append_version_once`] on a
/// [`AssistantRepositoryError::VersionConflict`] before giving up.
const MAX_APPEND_RETRIES: u32 = 20;

const INSERT_ASSISTANT: &str = "INSERT INTO assistants \
     (assistant_id, latest, source, created_at, deleted_at, schema_version) \
     VALUES (?, ?, ?, ?, NULL, ?)";

const INSERT_VERSION: &str = "INSERT INTO assistant_versions \
     (assistant_id, version, kind, body, created_at, created_by, note, schema_version) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

const UPDATE_LATEST_CAS: &str =
    "UPDATE assistants SET latest = ? WHERE assistant_id = ? AND latest = ?";

const SELECT_ASSISTANT_BY_ID: &str = "SELECT assistant_id, latest, source, created_at, \
     deleted_at, schema_version FROM assistants WHERE assistant_id = ?";

const SELECT_VERSION: &str = "SELECT assistant_id, version, kind, body, created_at, \
     created_by, note, schema_version FROM assistant_versions \
     WHERE assistant_id = ? AND version = ?";

const LIST_ASSISTANTS_PREFIX: &str = "SELECT assistant_id, latest, source, created_at, \
     deleted_at, schema_version FROM assistants WHERE 1 = 1";

// NOTE: no literal `?` here -- unlike the fixed-shape queries above, this is
// fed into a `QueryBuilder` whose OWN `push_bind` calls generate every
// placeholder; mixing a literal `?` into a `QueryBuilder` seed and then
// also calling `push_bind` produces a mismatched placeholder/binding count
// (a real bug this comment exists to prevent regressing).
const LIST_VERSIONS_PREFIX: &str = "SELECT assistant_id, version, kind, body, created_at, \
     created_by, note, schema_version FROM assistant_versions WHERE assistant_id = ";

const UPDATE_SOFT_DELETE: &str = "UPDATE assistants SET deleted_at = ? WHERE assistant_id = ?";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/sqlite");

fn kind_to_str(kind: AssistantKind) -> &'static str {
    match kind {
        AssistantKind::Agent => "agent",
        AssistantKind::Workflow => "workflow",
    }
}

fn kind_from_str(s: &str) -> Result<AssistantKind, AssistantRepositoryError> {
    match s {
        "agent" => Ok(AssistantKind::Agent),
        "workflow" => Ok(AssistantKind::Workflow),
        other => Err(AssistantRepositoryError::Serialization {
            message: format!("unknown assistant kind: {other:?}"),
        }),
    }
}

fn source_to_str(source: AssistantSource) -> &'static str {
    match source {
        AssistantSource::Stored => "stored",
        AssistantSource::Code => "code",
    }
}

fn source_from_str(s: &str) -> Result<AssistantSource, AssistantRepositoryError> {
    match s {
        "stored" => Ok(AssistantSource::Stored),
        "code" => Ok(AssistantSource::Code),
        other => Err(AssistantRepositoryError::Serialization {
            message: format!("unknown assistant source: {other:?}"),
        }),
    }
}

/// SQLite `AssistantRepositoryPort` implementation (PLAT-04, Tier 1: always
/// exercised in CI, no external service required).
#[derive(Debug)]
pub struct SqliteAssistantRepository {
    pool: SqlitePool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password, mirroring `SqliteRunRepository`'s
    /// rationale (T-22-18).
    database_url: String,
}

impl SqliteAssistantRepository {
    /// Connect to `database_url`, creating the database file if missing, and
    /// apply the versioned migration. Safe to call more than once against
    /// the same database file: the migration is idempotent and
    /// `sqlx::migrate::Migrator` itself tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, AssistantRepositoryError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        Ok(Self {
            pool,
            database_url: database_url.to_string(),
        })
    }

    /// Connect over a shared on-disk file with WAL journaling, mirroring
    /// `SqliteRunRepository::new_shared_file` exactly -- used only by
    /// `run/sqlite.rs`'s `insert_with_latest`/freeze-at-submit tests, which
    /// need a `SqliteAssistantRepository` and a `SqliteRunRepository`
    /// reading and writing the SAME on-disk database (the migrator embeds
    /// every file under `migrations/sqlite/`, so either constructor alone
    /// already creates both this crate's `runs` AND `assistants` tables on
    /// that shared file). `pub(crate)` (not `pub`): a cross-module,
    /// test-only construction path, not a production API surface.
    #[cfg(test)]
    pub(crate) async fn new_shared_file(
        database_url: &str,
    ) -> Result<Self, AssistantRepositoryError> {
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| Self::wrap(database_url, e))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| Self::wrap(database_url, e))?;

        Ok(Self {
            pool,
            database_url: database_url.to_string(),
        })
    }

    fn wrap(database_url: &str, err: impl std::error::Error) -> AssistantRepositoryError {
        let redacted = redact_database_url_password(&err.to_string(), database_url);
        AssistantRepositoryError::Backend {
            source: redacted.into(),
        }
    }

    fn wrap_error(&self, err: sqlx::Error) -> AssistantRepositoryError {
        Self::wrap(&self.database_url, err)
    }

    /// Map an `assistants` insert failure: a PRIMARY KEY violation on
    /// `assistant_id` is `AlreadyExists` -- checked BEFORE the generic wrap
    /// (27-RESEARCH.md Pattern 2). Every other `sqlx::Error` falls through
    /// to the generic `Backend` wrap.
    fn map_assistant_insert_error(
        &self,
        err: sqlx::Error,
        assistant_id: &AssistantId,
    ) -> AssistantRepositoryError {
        match &err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AssistantRepositoryError::AlreadyExists {
                    assistant_id: assistant_id.clone(),
                }
            }
            _ => self.wrap_error(err),
        }
    }

    /// Map an `assistant_versions` insert failure: a PRIMARY KEY violation
    /// on `(assistant_id, version)` is `VersionConflict` -- checked BEFORE
    /// the generic wrap (D-17's identical pattern, 27-RESEARCH.md Pattern
    /// 2). Every other `sqlx::Error` falls through to the generic `Backend`
    /// wrap.
    fn map_version_insert_error(
        &self,
        err: sqlx::Error,
        assistant_id: &AssistantId,
        version: u32,
    ) -> AssistantRepositoryError {
        match &err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AssistantRepositoryError::VersionConflict {
                    assistant_id: assistant_id.clone(),
                    version,
                }
            }
            _ => self.wrap_error(err),
        }
    }

    fn row_to_assistant(row: &SqliteRow) -> Result<Assistant, AssistantRepositoryError> {
        let backend_err = |e: sqlx::Error| AssistantRepositoryError::Backend { source: e.into() };

        let assistant_id_str: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_id = AssistantId::new(assistant_id_str).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: format!("invalid assistant_id: {e}"),
            }
        })?;
        let latest: i64 = row.try_get("latest").map_err(backend_err)?;
        let source_str: String = row.try_get("source").map_err(backend_err)?;
        let source = source_from_str(&source_str)?;
        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(backend_err)?;
        let deleted_at: Option<DateTime<Utc>> = row.try_get("deleted_at").map_err(backend_err)?;
        let schema_version: String = row.try_get("schema_version").map_err(backend_err)?;

        if schema_version != ASSISTANT_SCHEMA_VERSION {
            return Err(AssistantRepositoryError::UnknownSchemaVersion {
                found: schema_version,
            });
        }

        let mut assistant = Assistant::new(assistant_id, latest as u32, source);
        assistant.created_at = created_at;
        assistant.deleted_at = deleted_at;
        assistant.schema_version = schema_version;
        Ok(assistant)
    }

    fn row_to_version(row: &SqliteRow) -> Result<AssistantVersion, AssistantRepositoryError> {
        let backend_err = |e: sqlx::Error| AssistantRepositoryError::Backend { source: e.into() };
        let ser_err = |e: serde_json::Error| AssistantRepositoryError::Serialization {
            message: e.to_string(),
        };

        let assistant_id_str: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_id = AssistantId::new(assistant_id_str).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: format!("invalid assistant_id: {e}"),
            }
        })?;
        let version: i64 = row.try_get("version").map_err(backend_err)?;
        let kind_str: String = row.try_get("kind").map_err(backend_err)?;
        let kind = kind_from_str(&kind_str)?;
        let body_str: String = row.try_get("body").map_err(backend_err)?;
        let body: serde_json::Value = serde_json::from_str(&body_str).map_err(ser_err)?;
        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(backend_err)?;
        let created_by: Option<String> = row.try_get("created_by").map_err(backend_err)?;
        let note: Option<String> = row.try_get("note").map_err(backend_err)?;
        let schema_version: String = row.try_get("schema_version").map_err(backend_err)?;

        if schema_version != ASSISTANT_SCHEMA_VERSION {
            return Err(AssistantRepositoryError::UnknownSchemaVersion {
                found: schema_version,
            });
        }

        let mut assistant_version = AssistantVersion::new(
            assistant_id,
            version as u32,
            AssistantDefinition { kind, body },
        );
        assistant_version.created_at = created_at;
        assistant_version.created_by = created_by;
        assistant_version.note = note;
        assistant_version.schema_version = schema_version;
        Ok(assistant_version)
    }

    /// Attempt to append exactly one version, with NO retry: insert
    /// `(assistant_id, expected_latest + 1)` into `assistant_versions`, then
    /// CAS `assistants.latest` from `expected_latest` to `expected_latest +
    /// 1` -- both inside one transaction. A PRIMARY KEY violation on the
    /// version insert means another writer already claimed that version
    /// number and this call returns `VersionConflict` without retrying.
    async fn append_version_once(
        &self,
        assistant_id: &AssistantId,
        expected_latest: u32,
        new: &NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError> {
        let next_version = expected_latest + 1;
        let body = serde_json::to_string(&new.definition.body).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;

        let mut version =
            AssistantVersion::new(assistant_id.clone(), next_version, new.definition.clone());
        version.created_by = new.created_by.clone();
        version.note = new.note.clone();

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        sqlx::query(INSERT_VERSION)
            .bind(assistant_id.as_str())
            .bind(next_version as i64)
            .bind(kind_to_str(new.definition.kind))
            .bind(&body)
            .bind(version.created_at)
            .bind(&version.created_by)
            .bind(&version.note)
            .bind(&version.schema_version)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.map_version_insert_error(e, assistant_id, next_version))?;

        let result = sqlx::query(UPDATE_LATEST_CAS)
            .bind(next_version as i64)
            .bind(assistant_id.as_str())
            .bind(expected_latest as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            // Should not happen given this design's invariants: every
            // `latest` advance is paired with the version insert that just
            // succeeded above, inside the SAME transaction, under the
            // caller-supplied `expected_latest` CAS predicate. Surfacing
            // this as a distinct backend error (rather than silently
            // swallowing it) makes an actual violation of that invariant
            // loud rather than a quietly wrong `latest`.
            return Err(AssistantRepositoryError::Backend {
                source: format!(
                    "assistant_versions row for ({assistant_id}, {next_version}) was inserted \
                     but assistants.latest could not be advanced from {expected_latest}"
                )
                .into(),
            });
        }

        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(version)
    }
}

#[async_trait]
impl AssistantRepositoryPort for SqliteAssistantRepository {
    async fn create(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError> {
        let mut version = AssistantVersion::new(assistant_id.clone(), 1, new.definition.clone());
        version.created_by = new.created_by;
        version.note = new.note;

        let body = serde_json::to_string(&new.definition.body).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: e.to_string(),
            }
        })?;

        let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

        sqlx::query(INSERT_ASSISTANT)
            .bind(assistant_id.as_str())
            .bind(1i64)
            .bind(source_to_str(AssistantSource::Stored))
            .bind(version.created_at)
            .bind(ASSISTANT_SCHEMA_VERSION)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.map_assistant_insert_error(e, assistant_id))?;

        sqlx::query(INSERT_VERSION)
            .bind(assistant_id.as_str())
            .bind(1i64)
            .bind(kind_to_str(new.definition.kind))
            .bind(&body)
            .bind(version.created_at)
            .bind(&version.created_by)
            .bind(&version.note)
            .bind(&version.schema_version)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        tx.commit().await.map_err(|e| self.wrap_error(e))?;
        Ok(version)
    }

    async fn append_version(
        &self,
        assistant_id: &AssistantId,
        new: NewAssistantVersion,
    ) -> Result<AssistantVersion, AssistantRepositoryError> {
        let mut last_attempted_version = 0u32;
        for _ in 0..MAX_APPEND_RETRIES {
            let assistant = self.get(assistant_id).await?.ok_or_else(|| {
                AssistantRepositoryError::NotFound {
                    assistant_id: assistant_id.clone(),
                }
            })?;
            if assistant.deleted_at.is_some() {
                return Err(AssistantRepositoryError::NotFound {
                    assistant_id: assistant_id.clone(),
                });
            }
            last_attempted_version = assistant.latest + 1;
            match self
                .append_version_once(assistant_id, assistant.latest, &new)
                .await
            {
                Ok(version) => return Ok(version),
                Err(AssistantRepositoryError::VersionConflict { .. }) => continue,
                Err(other) => return Err(other),
            }
        }
        Err(AssistantRepositoryError::VersionConflict {
            assistant_id: assistant_id.clone(),
            version: last_attempted_version,
        })
    }

    async fn get(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<Option<Assistant>, AssistantRepositoryError> {
        let row = sqlx::query(SELECT_ASSISTANT_BY_ID)
            .bind(assistant_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        row.as_ref().map(Self::row_to_assistant).transpose()
    }

    async fn get_version(
        &self,
        assistant_id: &AssistantId,
        version: u32,
    ) -> Result<Option<AssistantVersion>, AssistantRepositoryError> {
        if version == 0 {
            return Ok(None);
        }
        let row = sqlx::query(SELECT_VERSION)
            .bind(assistant_id.as_str())
            .bind(version as i64)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        row.as_ref().map(Self::row_to_version).transpose()
    }

    async fn list(
        &self,
        limit: u32,
        cursor: Option<AssistantId>,
        include_deleted: bool,
    ) -> Result<AssistantPage, AssistantRepositoryError> {
        let unlimited = limit == 0;
        let fetch_limit: i64 = if unlimited {
            i64::MAX
        } else {
            limit as i64 + 1
        };

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_ASSISTANTS_PREFIX);
        if !include_deleted {
            builder.push(" AND deleted_at IS NULL");
        }
        if let Some(cursor) = &cursor {
            builder.push(" AND assistant_id > ");
            builder.push_bind(cursor.as_str().to_string());
        }
        builder.push(" ORDER BY assistant_id ASC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut items: Vec<Assistant> = rows
            .iter()
            .map(Self::row_to_assistant)
            .collect::<Result<_, _>>()?;

        let next_cursor = if !unlimited && items.len() > limit as usize {
            items.pop();
            items.last().map(|last| last.assistant_id.clone())
        } else {
            None
        };

        Ok(AssistantPage { items, next_cursor })
    }

    async fn list_versions(
        &self,
        assistant_id: &AssistantId,
        limit: u32,
        cursor: Option<u32>,
    ) -> Result<AssistantVersionPage, AssistantRepositoryError> {
        let unlimited = limit == 0;
        let fetch_limit: i64 = if unlimited {
            i64::MAX
        } else {
            limit as i64 + 1
        };

        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_VERSIONS_PREFIX);
        builder.push_bind(assistant_id.as_str().to_string());
        if let Some(cursor) = cursor {
            builder.push(" AND version > ");
            builder.push_bind(cursor as i64);
        }
        builder.push(" ORDER BY version ASC LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        let mut items: Vec<AssistantVersion> = rows
            .iter()
            .map(Self::row_to_version)
            .collect::<Result<_, _>>()?;

        let next_cursor = if !unlimited && items.len() > limit as usize {
            items.pop();
            items.last().map(|last| last.version)
        } else {
            None
        };

        Ok(AssistantVersionPage { items, next_cursor })
    }

    async fn soft_delete(
        &self,
        assistant_id: &AssistantId,
    ) -> Result<(), AssistantRepositoryError> {
        let result = sqlx::query(UPDATE_SOFT_DELETE)
            .bind(Utc::now())
            .bind(assistant_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            return Err(AssistantRepositoryError::NotFound {
                assistant_id: assistant_id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::contract_tests;
    use std::sync::Arc;

    // One #[tokio::test] per shared contract function (D-09 precedent),
    // each against a fresh in-memory database, so a failure names the
    // violated contract clause. See `contract_tests` for the assertions
    // themselves -- this file only wires `SqliteAssistantRepository` into
    // them, unchanged, plus adapter-specific tests below (the retry loop,
    // the no-retry primitive, and password redaction).

    async fn fresh_store() -> SqliteAssistantRepository {
        SqliteAssistantRepository::new("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn create_creates_version_one_and_rejects_duplicate() {
        contract_tests::create_creates_version_one_and_rejects_duplicate(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn append_version_advances_latest_and_rejects_unknown_or_deleted() {
        contract_tests::append_version_advances_latest_and_rejects_unknown_or_deleted(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn get_version_out_of_range_returns_none() {
        contract_tests::get_version_out_of_range_returns_none(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn list_versions_paginates_ascending_by_version() {
        contract_tests::list_versions_paginates_ascending_by_version(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn list_assistants_paginates_ascending_by_assistant_id() {
        contract_tests::list_assistants_paginates_ascending_by_assistant_id(&fresh_store().await)
            .await;
    }

    #[tokio::test]
    async fn list_excludes_soft_deleted_unless_requested() {
        contract_tests::list_excludes_soft_deleted_unless_requested(&fresh_store().await).await;
    }

    #[tokio::test]
    async fn soft_delete_then_versions_still_readable_but_append_fails() {
        contract_tests::soft_delete_then_versions_still_readable_but_append_fails(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn publishing_identical_body_twice_creates_distinct_versions() {
        contract_tests::publishing_identical_body_twice_creates_distinct_versions(
            &fresh_store().await,
        )
        .await;
    }

    #[tokio::test]
    async fn get_version_on_unsupported_schema_version_fails() {
        contract_tests::get_version_on_unsupported_schema_version_fails(&fresh_store().await).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_append_admits_exactly_one_per_version() {
        let repo: Arc<dyn AssistantRepositoryPort> = Arc::new(fresh_store().await);
        contract_tests::concurrent_append_admits_exactly_one_per_version(repo).await;
    }

    // ── Adapter-specific: the no-retry primitive genuinely conflicts ─────

    #[tokio::test]
    async fn append_version_once_returns_version_conflict_when_two_callers_precompute_same_n_plus_1()
     {
        let store = fresh_store().await;
        let id = AssistantId::new("sqlite-append-once-conflict").unwrap();
        store
            .create(
                &id,
                NewAssistantVersion {
                    definition: AssistantDefinition {
                        kind: AssistantKind::Agent,
                        body: serde_json::json!({}),
                    },
                    created_by: None,
                    note: None,
                },
            )
            .await
            .unwrap();

        let new_version = NewAssistantVersion {
            definition: AssistantDefinition {
                kind: AssistantKind::Agent,
                body: serde_json::json!({ "attempt": 1 }),
            },
            created_by: None,
            note: None,
        };

        // Both callers pre-compute next = 2 from the SAME expected_latest
        // (1) -- the first call succeeds, the second must genuinely
        // conflict since `append_version_once` performs NO retry.
        let first = store.append_version_once(&id, 1, &new_version).await;
        assert!(first.is_ok());

        let second = store.append_version_once(&id, 1, &new_version).await;
        assert!(matches!(
            second,
            Err(AssistantRepositoryError::VersionConflict { .. })
        ));
    }

    #[tokio::test]
    async fn connection_error_redacts_password_from_database_url() {
        let url = "sqlite://user:hunter2-secret@/nonexistent/path/that/does/not/exist.db";
        let err = SqliteAssistantRepository::new(url).await.unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("hunter2-secret"),
            "connection error leaked the password: {message}"
        );
    }
}
