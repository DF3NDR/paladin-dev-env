/*
PostgreSQL Assistant Repository

Concrete `AssistantRepositoryPort` implementation over PostgreSQL, behind
the `postgres` feature (PLAT-04, D-28, D-29, D-30, Tier 2: Docker-gated, see
`docker/docker-compose.test.yml`'s `postgres-test` service and `make
test-integration-docker`). Mirrors `sqlite.rs` exactly -- same seven
methods, same transactional create/append-then-CAS shape, same
is_unique_violation()-before-generic-wrap mapping to
`AlreadyExists`/`VersionConflict` (27-RESEARCH.md Pattern 2) --
substituting `$1, $2, ...` placeholders for `?`, `JSONB` (via an explicit
`::jsonb` cast on write, the `body` precedent in `run/postgres.rs`) for the
TEXT JSON `body` column, and native `TIMESTAMPTZ` for the timestamp
columns. Every statement uses bound parameters; no query string is ever
built by formatting a caller-supplied value into it.
*/

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};

use paladin_core::platform::container::assistant::{
    ASSISTANT_SCHEMA_VERSION, Assistant, AssistantDefinition, AssistantId, AssistantKind,
    AssistantSource, AssistantVersion, NewAssistantVersion,
};
use paladin_ports::output::assistant_repository_port::{
    AssistantPage, AssistantRepositoryError, AssistantRepositoryPort, AssistantVersionPage,
};

use crate::waypoint::redact::redact_database_url_password;

/// The number of times the public [`AssistantRepositoryPort::append_version`]
/// retries [`PostgresAssistantRepository::append_version_once`] on a
/// [`AssistantRepositoryError::VersionConflict`] before giving up.
const MAX_APPEND_RETRIES: u32 = 20;

const INSERT_ASSISTANT: &str = "INSERT INTO assistants \
     (assistant_id, latest, source, created_at, deleted_at, schema_version) \
     VALUES ($1, $2, $3, $4, NULL, $5)";

const INSERT_VERSION: &str = "INSERT INTO assistant_versions \
     (assistant_id, version, kind, body, created_at, created_by, note, schema_version) \
     VALUES ($1, $2, $3, $4::jsonb, $5, $6, $7, $8)";

const UPDATE_LATEST_CAS: &str =
    "UPDATE assistants SET latest = $1 WHERE assistant_id = $2 AND latest = $3";

const SELECT_ASSISTANT_BY_ID: &str = "SELECT assistant_id, latest, source, created_at, \
     deleted_at, schema_version FROM assistants WHERE assistant_id = $1";

const SELECT_VERSION: &str = "SELECT assistant_id, version, kind, body, created_at, \
     created_by, note, schema_version FROM assistant_versions \
     WHERE assistant_id = $1 AND version = $2";

const LIST_ASSISTANTS_PREFIX: &str = "SELECT assistant_id, latest, source, created_at, \
     deleted_at, schema_version FROM assistants WHERE 1 = 1";

// NOTE: no literal `$1` here -- fed into a `QueryBuilder` whose own
// `push_bind` calls generate every placeholder (mirrors the identical fix
// in `sqlite.rs`'s `LIST_VERSIONS_PREFIX`: mixing a hand-written
// placeholder into a `QueryBuilder` seed produces a mismatched
// placeholder/binding count).
const LIST_VERSIONS_PREFIX: &str = "SELECT assistant_id, version, kind, body, created_at, \
     created_by, note, schema_version FROM assistant_versions WHERE assistant_id = ";

const UPDATE_SOFT_DELETE: &str = "UPDATE assistants SET deleted_at = $1 WHERE assistant_id = $2";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations/postgres");

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

/// PostgreSQL `AssistantRepositoryPort` implementation, behind the
/// `postgres` feature (PLAT-04, D-28, Tier 2).
#[derive(Debug)]
pub struct PostgresAssistantRepository {
    pool: PgPool,
    /// Kept so every error this store returns can be redacted of the
    /// connection URL's password (mirrors `PostgresRunRepository`'s
    /// rationale, T-22-18).
    database_url: String,
}

impl PostgresAssistantRepository {
    /// Connect to `database_url` and apply the versioned migration. Safe to
    /// call more than once against the same database: the migration is
    /// idempotent and `sqlx::migrate::Migrator` tracks applied versions.
    pub async fn new(database_url: &str) -> Result<Self, AssistantRepositoryError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            // Mirrors `PostgresRunRepository::new`'s rationale: a genuinely
            // unreachable server (this Tier 2 suite's local-skip case)
            // surfaces as a fast, clearly-diagnosed error rather than a
            // slow hang absorbing sqlx's default 30s acquire timeout.
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(database_url)
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

    /// Map an `assistants` insert failure: a unique-constraint violation on
    /// `assistant_id` is `AlreadyExists` -- checked BEFORE the generic wrap
    /// (27-RESEARCH.md Pattern 2, SQLSTATE 23505). Every other
    /// `sqlx::Error` falls through to the generic `Backend` wrap.
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

    /// Map an `assistant_versions` insert failure: a unique-constraint
    /// violation on `(assistant_id, version)` is `VersionConflict` --
    /// checked BEFORE the generic wrap. Every other `sqlx::Error` falls
    /// through to the generic `Backend` wrap.
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

    fn row_to_assistant(row: &PgRow) -> Result<Assistant, AssistantRepositoryError> {
        let backend_err = |e: sqlx::Error| AssistantRepositoryError::Backend { source: e.into() };

        let assistant_id_str: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_id = AssistantId::new(assistant_id_str).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: format!("invalid assistant_id: {e}"),
            }
        })?;
        let latest: i32 = row.try_get("latest").map_err(backend_err)?;
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

    fn row_to_version(row: &PgRow) -> Result<AssistantVersion, AssistantRepositoryError> {
        let backend_err = |e: sqlx::Error| AssistantRepositoryError::Backend { source: e.into() };

        let assistant_id_str: String = row.try_get("assistant_id").map_err(backend_err)?;
        let assistant_id = AssistantId::new(assistant_id_str).map_err(|e| {
            AssistantRepositoryError::Serialization {
                message: format!("invalid assistant_id: {e}"),
            }
        })?;
        let version: i32 = row.try_get("version").map_err(backend_err)?;
        let kind_str: String = row.try_get("kind").map_err(backend_err)?;
        let kind = kind_from_str(&kind_str)?;
        let body: serde_json::Value = row.try_get("body").map_err(backend_err)?;
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

    /// Attempt to append exactly one version, with NO retry -- see
    /// `sqlite.rs`'s identical method for the full rationale.
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
            .bind(next_version as i32)
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
            .bind(next_version as i32)
            .bind(assistant_id.as_str())
            .bind(expected_latest as i32)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.wrap_error(e))?;

        if result.rows_affected() == 0 {
            // Should not happen given this design's invariants -- see
            // `sqlite.rs`'s identical branch for the full rationale.
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
impl AssistantRepositoryPort for PostgresAssistantRepository {
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
            .bind(1i32)
            .bind(source_to_str(AssistantSource::Stored))
            .bind(version.created_at)
            .bind(ASSISTANT_SCHEMA_VERSION)
            .execute(&mut *tx)
            .await
            .map_err(|e| self.map_assistant_insert_error(e, assistant_id))?;

        sqlx::query(INSERT_VERSION)
            .bind(assistant_id.as_str())
            .bind(1i32)
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
            .bind(version as i32)
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
            sqlx::QueryBuilder::new(LIST_ASSISTANTS_PREFIX);
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

        let mut builder: sqlx::QueryBuilder<sqlx::Postgres> =
            sqlx::QueryBuilder::new(LIST_VERSIONS_PREFIX);
        builder.push_bind(assistant_id.as_str().to_string());
        if let Some(cursor) = cursor {
            builder.push(" AND version > ");
            builder.push_bind(cursor as i32);
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

    // Docker-gated Tier 2 suite (D-51): every test independently probes the
    // shared Postgres service and prints a named `SKIP:` reason then
    // returns early -- never panics or hangs -- when it is not reachable,
    // mirroring `run::postgres`'s `store_or_skip` gate exactly. This whole
    // module is ALSO compile-time gated behind the `postgres` feature (see
    // `assistant/mod.rs`), which is not in any default feature set.
    //
    // `STORAGE_POSTGRES_TEST_URL` is the SAME storage-wide env var name
    // `run::postgres` reads (27-02's own naming decision) -- one CI export
    // covers both suites.
    //
    // Bring the service up before running this suite:
    // ```sh
    // docker compose -f docker/docker-compose.test.yml up -d postgres-test
    // STORAGE_POSTGRES_TEST_URL=postgres://... \
    //   cargo test -p paladin-storage --features postgres --lib assistant::postgres
    // ```

    fn postgres_test_url() -> String {
        std::env::var("STORAGE_POSTGRES_TEST_URL").unwrap_or_else(|_| {
            "postgres://paladin:paladin@localhost:5433/paladin_run_test".to_string()
        })
    }

    /// A cheap, short-timeout TCP reachability probe, tried BEFORE handing
    /// `url` to `sqlx`'s pool -- mirrors `run::postgres`'s identical helper.
    fn postgres_reachable(url: &str) -> bool {
        use std::net::ToSocketAddrs;

        let Ok(parsed) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let port = parsed.port().unwrap_or(5432);

        (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .is_some_and(|addr| {
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(750))
                    .is_ok()
            })
    }

    /// Returns a connected, migrated store, or `None` (after printing a
    /// named reason) if `postgres-test` is not reachable.
    async fn store_or_skip() -> Option<PostgresAssistantRepository> {
        let url = postgres_test_url();
        if !postgres_reachable(&url) {
            println!(
                "SKIP: postgres-test not reachable at {url} -- bring it up with \
                 `docker compose -f docker/docker-compose.test.yml up -d postgres-test`"
            );
            return None;
        }

        match PostgresAssistantRepository::new(&url).await {
            Ok(store) => Some(store),
            Err(e) => {
                println!("SKIP: postgres-test connection failed at {url} ({e})");
                None
            }
        }
    }

    // One #[tokio::test] per shared contract function (D-09 precedent),
    // written out explicitly (not via a macro) so each names the violated
    // contract clause on failure, mirroring `sqlite.rs`'s test module and
    // `run::postgres`'s skip-gate exactly.

    #[tokio::test]
    async fn create_creates_version_one_and_rejects_duplicate() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::create_creates_version_one_and_rejects_duplicate(&store).await;
    }

    #[tokio::test]
    async fn append_version_advances_latest_and_rejects_unknown_or_deleted() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::append_version_advances_latest_and_rejects_unknown_or_deleted(&store).await;
    }

    #[tokio::test]
    async fn get_version_out_of_range_returns_none() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_version_out_of_range_returns_none(&store).await;
    }

    #[tokio::test]
    async fn list_versions_paginates_ascending_by_version() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_versions_paginates_ascending_by_version(&store).await;
    }

    #[tokio::test]
    async fn list_assistants_paginates_ascending_by_assistant_id() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_assistants_paginates_ascending_by_assistant_id(&store).await;
    }

    #[tokio::test]
    async fn list_excludes_soft_deleted_unless_requested() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::list_excludes_soft_deleted_unless_requested(&store).await;
    }

    #[tokio::test]
    async fn soft_delete_then_versions_still_readable_but_append_fails() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::soft_delete_then_versions_still_readable_but_append_fails(&store).await;
    }

    #[tokio::test]
    async fn publishing_identical_body_twice_creates_distinct_versions() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::publishing_identical_body_twice_creates_distinct_versions(&store).await;
    }

    #[tokio::test]
    async fn get_version_on_unsupported_schema_version_fails() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        contract_tests::get_version_on_unsupported_schema_version_fails(&store).await;
    }

    // Plain `#[tokio::test]` (current-thread flavor), not `multi_thread`:
    // this module's declared-test count is asserted equal to
    // `contract_tests`'s `pub async fn` count by regex on the literal
    // `#[tokio::test]` attribute (this plan's own acceptance criterion),
    // mirroring `run::postgres`'s identical choice -- the assertion under
    // test is a database-level invariant (the PRIMARY KEY on
    // `(assistant_id, version)`), not one that depends on OS-thread
    // parallelism.
    #[tokio::test]
    async fn concurrent_append_admits_exactly_one_per_version() {
        let Some(store) = store_or_skip().await else {
            return;
        };
        let store: Arc<dyn AssistantRepositoryPort> = Arc::new(store);
        contract_tests::concurrent_append_admits_exactly_one_per_version(store).await;
    }

    // No extra, non-contract `#[tokio::test]`s in this module by design:
    // CI asserts this module's `#[tokio::test]` count equals
    // `contract_tests`'s `pub async fn` count exactly (this plan's own
    // acceptance criterion), so an `append_version_once`/password-redaction
    // smoke test analogous to `sqlite.rs`'s would break that count. Those
    // are proven once, on SQLite, where the identical logic runs Tier 1
    // (no Docker) on every CI run.
}
