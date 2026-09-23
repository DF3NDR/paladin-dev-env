# Phase 22: Battlefield State & Superstep Engine - Pattern Map

**Mapped:** 2026-09-01
**Files analyzed:** 15 (core types, port, engine, storage adapters, config, CI)
**Analogs found:** 13 / 15

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/src/platform/container/battlefield.rs` | model | CRUD (typed field merge) | `crates/paladin-core/src/platform/container/token_usage.rs` (value-type shape) + `citadel.rs` (module doc/versioning style) | role-match |
| `crates/paladin-core/src/platform/container/battlefield_error.rs` | model (error enum) | — | `crates/paladin-core/src/platform/container/citadel_error.rs` | exact |
| `crates/paladin-core/src/platform/container/waypoint.rs` | model | event-driven (status machine) | `citadel.rs` (`PaladinState`/`StateSummary`/schema_version pattern) | role-match |
| `crates/paladin-ports/src/output/waypoint_port.rs` | service (port trait) | CRUD + request-response | `crates/paladin-ports/src/output/citadel_port.rs` | exact (explicitly modeled as sibling-but-separate) |
| `crates/paladin-battalion/src/engine/mod.rs` (`WarEngine`, `WarGraph`) | service (orchestrator) | event-driven / graph traversal | `crates/paladin-battalion/src/campaign_service.rs` (graph execution) | role-match |
| `crates/paladin-battalion/src/engine/superstep.rs` | service (concurrent step executor) | streaming/batch (fan-out + join) | `crates/paladin-battalion/src/phalanx_service.rs` (`execute_collect_all`) | exact for concurrency shape |
| `crates/paladin-battalion/src/engine/bridges.rs` | transform/adapter (legacy→engine) | transform | `campaign_service.rs` fan-in join (`"\n\n---\n\n"`) at line 373 (verified separately in RESEARCH.md) | role-match |
| `crates/paladin-battalion/src/engine/dispatch_registry.rs` | service (registry) | CRUD | no close analog — new concept | none |
| `crates/paladin-battalion/src/engine/input_mapping.rs` | utility (template resolution) | transform | no close analog — new concept | none |
| `crates/paladin-storage/src/waypoint/in_memory.rs` | model/service (in-memory store) | CRUD | `crates/paladin-ports/src/output/citadel_port.rs` doc-test `InMemoryCitadel` (lines 415-469, `Arc<RwLock<HashMap<...>>>`) | role-match |
| `crates/paladin-storage/src/waypoint/sqlite.rs` | storage adapter | CRUD (file-I/O via sqlx) | `crates/paladin-storage/src/sqlite_workflow_repository.rs` | exact |
| `crates/paladin-storage/src/waypoint/postgres.rs` | storage adapter | CRUD (file-I/O via sqlx) | `crates/paladin-storage/src/sqlite_workflow_repository.rs` (same query shape, `?`→`$N` placeholders, JSONB instead of TEXT) | role-match |
| `crates/paladin-storage/migrations/00X_create_waypoints_table.sql` | migration | — | `crates/paladin-memory/migrations/001_create_garrison_tables.sql` | exact |
| `src/config/engine.rs` / `src/config/waypoint_retention.rs` | config | CRUD (struct + validate) | `src/config/citadel.rs` + `src/config/env_utils.rs` | exact |
| `.github/workflows/ci.yml` (`semver`, `msrv` jobs) | config (CI) | — | existing `api-surface`/`test`/`publish-dry-run` jobs in same file (not re-read here; RESEARCH.md already enumerates job names) | role-match |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/battlefield.rs` (model, CRUD)

**Analogs:** `token_usage.rs` (simple value type shape) and `citadel.rs` (module-doc + schema_version + `#[derive(..., Serialize, Deserialize)]` convention)

**Module doc + derive pattern** (`token_usage.rs` lines 1-20):
```rust
//! Token Usage Tracking
//!
//! This module defines [`TokenUsage`], a pure domain value type for tracking
//! LLM token consumption. The `application` layer re-exports it from here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```
Apply the same shape to `Battlefield`/`StateDelta`/`FieldSpec`: plain `Debug, Clone, PartialEq, Serialize, Deserialize` value structs, constructor associated functions (`new`, `from_total`-style helpers), no interior mutability, tests colocated in `#[cfg(test)] mod tests` in the same file.

**Persisted-type versioning pattern** (`citadel.rs` lines 97-110, doc comments):
```rust
/// - `schema_version`: Version identifier for state schema compatibility
```
Apply: every persisted `Battlefield`/`Waypoint` type carries `schema_version: String` per X-04, exactly as `PaladinState`/`BattalionState` already do in `citadel.rs`.

---

### `crates/paladin-core/src/platform/container/battlefield_error.rs` (model/error, N/A)

**Analog:** `crates/paladin-core/src/platform/container/citadel_error.rs` (full file, 162 lines — read in full)

**Error enum + constructor + `#[from]` pattern** (lines 24-92):
```rust
#[derive(Debug, Error)]
pub enum CitadelError {
    #[error("State not found: {0}")]
    StateNotFound(Uuid),

    #[error("Incompatible state version: expected {expected}, found {found}")]
    IncompatibleVersion { expected: String, found: String },

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    // ...
}

impl CitadelError {
    pub fn corrupted(message: impl Into<String>) -> Self {
        Self::CorruptedState(message.into())
    }
    // ... one constructor helper per non-trivial variant
}
```
Apply directly to `BattlefieldError` (`UnknownField`, `MissingRequiredField`, `TypeMismatch`, `SchemaVersionUnsupported`, dispatch conflicts) and `EngineError`/`WaypointError`: named struct variants for multi-field errors, tuple variants for single-value errors, `#[from]` at I/O/serde boundaries, doc-tested constructor helpers, `#[cfg(test)] mod tests` asserting `to_string()` contains expected substrings (see lines 94-161 for the exact assertion style: `assert!(error_msg.contains(...))`).

---

### `crates/paladin-core/src/platform/container/waypoint.rs` (model, event-driven)

**Analog:** `citadel.rs` (`PaladinState`, `StateSummary`, `PaladinStatus` enum — lines 88-96, 97-...)

**Status enum pattern** (lines 88-95):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaladinStatus {
    Idle,
    Executing,
    Completed,
    Failed(String),
}
```
Apply the same shape to `WaypointStatus` (`Running`, `Completed`, `Halted`, `AwaitingInput(ParleyRequest)` stub, `Failed(String)`-style variant). Consider `#[non_exhaustive]` here per RESEARCH.md Pitfall 3 (no existing precedent in the tree, but this enum is known to grow in Docs 02-04).

---

### `crates/paladin-ports/src/output/waypoint_port.rs` (service/port trait, CRUD + request-response)

**Analog:** `crates/paladin-ports/src/output/citadel_port.rs` (full file, 663 lines — read in full; excerpts below are the load-bearing sections)

**Trait shape** (lines 566-586):
```rust
#[async_trait]
pub trait CitadelPort: Send + Sync {
    async fn save_paladin(&self, state: &PaladinState) -> Result<(), CitadelError>;
    async fn load_paladin(&self, id: Uuid) -> Result<Option<PaladinState>, CitadelError>;
    async fn save_battalion(&self, state: &BattalionState) -> Result<(), CitadelError>;
    async fn load_battalion(&self, id: Uuid) -> Result<Option<BattalionState>, CitadelError>;
    async fn list_saved(&self) -> Result<Vec<StateSummary>, CitadelError>;
}
```
Apply directly to `WaypointPort`: `save(&self, waypoint: &Waypoint) -> Result<(), WaypointError>`, `load_latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError>`, `list_for_thread`, plus D-01's retention-cleanup method. Same `Send + Sync`/`#[async_trait]` bound, same "`None` for missing, not an error" contract (documented at lines 305-306).

**Object-safety + mock test pattern** (lines 588-662):
```rust
struct MockCitadel;

#[async_trait]
impl CitadelPort for MockCitadel {
    async fn save_paladin(&self, _state: &PaladinState) -> Result<(), CitadelError> { Ok(()) }
    // ...
}

#[test]
fn test_trait_is_object_safe() {
    let _: Option<Box<dyn CitadelPort>> = None;
}
```
Reuse verbatim for `WaypointPort`'s own trait-object-safety test.

**Rationale-for-separation doc requirement:** CONTEXT.md D-decisions require `waypoint_port.rs`'s module doc to explicitly state why it is NOT `CitadelPort` (high-frequency/append-mostly/thread-addressed vs coarse entity snapshots) — model the doc structure (Purpose / Hexagonal Architecture Context / Thread Safety / Error Handling headers) on `citadel_port.rs` lines 1-68, but write new rationale prose, don't copy the Citadel-specific reasoning.

---

### `crates/paladin-battalion/src/engine/mod.rs` (`WarEngine`/`WarGraph`, service, event-driven graph traversal)

**Analog:** `crates/paladin-battalion/src/campaign_service.rs` (`execute_internal`, lines 223-352 read in full)

**Graph traversal skeleton to adapt (NOT toposort-reuse)** (lines 236-326):
```rust
// campaign_service.rs — DO NOT reuse toposort; this is the exact mechanism ENG-FR-02 forbids
let sorted_nodes = toposort(campaign.graph(), None).map_err(|cycle| {
    BattalionError::InvalidGraph(format!("Cycle detected in campaign graph at node {:?}", cycle.node_id()))
})?;

let mut node_outputs: HashMap<Uuid, String> = HashMap::new();
let mut all_results: Vec<PaladinResult> = Vec::new();
let mut ready_nodes: HashSet<Uuid> = entry_points.clone();
let mut executed_nodes: HashSet<Uuid> = HashSet::new();

for node_index in sorted_nodes {
    // ... execute, then evaluate outgoing edges to compute the next ready set
}
```
The WarEngine's superstep loop should follow the same "frontier set → execute → compute next frontier from edges" shape, but replace `toposort` with a bounded iteration counter (`max_supersteps`) and a plain `HashMap<NodeId, NodeSpec>` + `Vec<EdgeSpec>` per RESEARCH.md's explicit recommendation — self-loops/cycles must be legal.

**Service struct/builder shape** (`campaign_service.rs` lines 58-100, same shape in `phalanx_service.rs` lines 41-99):
```rust
pub struct CampaignExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
    herald: Option<Arc<dyn Herald>>,
}

impl CampaignExecutionService {
    pub fn new(paladin_port: Arc<dyn PaladinPort>) -> Self { /* ... */ }
    pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
        self.herald = Some(herald);
        self
    }
}
```
`WarEngine::new(paladin_port, waypoint_port)` / `.with_trace_sink(...)` / `.with_interceptors(...)` should follow this exact fluent-builder-via-`Arc<dyn Trait>`-field shape.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (service, concurrent fan-out/join)

**Analog:** `crates/paladin-battalion/src/phalanx_service.rs::execute_collect_all` (lines 323-376, read in full)

**Concurrent execution + join pattern**:
```rust
let semaphore = phalanx.max_concurrency().map(|max| Arc::new(Semaphore::new(max)));
let mut tasks = Vec::new();

for paladin in phalanx.paladins() {
    let paladin_clone = paladin.clone();
    let input_clone = input.to_string();
    let port = self.paladin_port.clone();
    let semaphore_clone = semaphore.clone();

    let task: tokio::task::JoinHandle<Result<PaladinResult, PaladinError>> =
        tokio::spawn(async move {
            let _permit = if let Some(sem) = &semaphore_clone {
                Some(sem.acquire().await.unwrap())
            } else {
                None
            };
            port.execute(&paladin_clone, &input_clone).await
        });
    tasks.push(task);
}

let mut results = Vec::new();
let mut errors = Vec::new();
for (i, task) in tasks.into_iter().enumerate() {
    match task.await {
        Ok(Ok(result)) => results.push(result),
        Ok(Err(e)) => errors.push(format!("{}: {}", paladin_name, e)),
        Err(e) => errors.push(format!("{}: Task join error: {}", paladin_name, e)),
    }
}
```
Apply this exact `tokio::spawn` + `Semaphore` + "collect handles, then await-join sequentially" shape for executing a superstep's Vanguard concurrently. Per D-12/Pattern 2 in RESEARCH.md, wrap the pre-superstep read snapshot in `Arc<Battlefield>` (cloned once per superstep, not per node) instead of `Arc<dyn PaladinPort>`-only capture — collect `(NodeId, StateDelta)` pairs the same way this collects `(index, PaladinResult)`, then merge only after the `for` loop completes (never mutate the shared battlefield inside the spawned tasks).

**Error handling shape:** note this file does NOT propagate individual task errors as a whole-call `Result::Err` — it collects per-node errors into a `Vec<String>` and returns `(results, errors)` for the caller to reason about partial failure. For the engine's Strict-durability semantics (a Waypoint write failure fails the whole run), this shape needs modification: propagate the Waypoint-persist error immediately rather than collecting it, but keep the node-execution collect-then-report shape for individual node failures within a superstep.

---

### `crates/paladin-storage/src/waypoint/sqlite.rs` (storage adapter, CRUD/file-I/O)

**Analog:** `crates/paladin-storage/src/sqlite_workflow_repository.rs` (full file read, 260+ lines)

**Struct + constructor + migration pattern** (lines 19-69):
```rust
pub struct SqliteWorkflowRepository {
    pool: SqlitePool,
}

impl SqliteWorkflowRepository {
    pub async fn new(database_url: &str) -> Result<Self, WorkflowRepositoryError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| WorkflowRepositoryError::RepositoryError(format!("Failed to connect to database: {e}")))?;

        let repository = Self { pool };
        repository.migrate().await?;
        Ok(repository)
    }

    async fn migrate(&self) -> Result<(), WorkflowRepositoryError> {
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS workflow_state (...)"#)
            .execute(&self.pool).await
            .map_err(|e| WorkflowRepositoryError::RepositoryError(format!("Migration failed: {e}")))?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_workflow_state_status ON workflow_state(status)")
            .execute(&self.pool).await
            .map_err(|e| WorkflowRepositoryError::RepositoryError(format!("Index creation failed: {e}")))?;
        Ok(())
    }
}
```
`SqliteWaypointStore::new(database_url)` should follow this exact shape. Note: per D-02, use `CREATE TABLE IF NOT EXISTS waypoints (... payload TEXT NOT NULL ...)` and `CREATE INDEX IF NOT EXISTS idx_waypoints_thread_created ON waypoints(thread_id, created_at DESC)` per ENG-FR-16's named access pattern — but the project's actual migration convention (per D-02) is a versioned SQL file under `crates/paladin-storage/migrations/`, not an in-code `migrate()` call; verify which convention this crate currently uses (`sqlite_workflow_repository.rs` uses in-code `migrate()`, but D-02 explicitly says migrations follow `crates/paladin-memory/migrations/001_...` file convention) — prefer the file-based migration convention since D-02 names it explicitly.

**Bound-parameter INSERT/UPSERT pattern** (lines 126-150):
```rust
sqlx::query(
    r#"
    INSERT INTO workflow_state (workflow_id, status, completed_job_ids, definition, updated_at)
    VALUES (?, ?, ?, ?, ?)
    ON CONFLICT(workflow_id) DO UPDATE SET
        status = excluded.status,
        completed_job_ids = excluded.completed_job_ids,
        definition = excluded.definition,
        updated_at = excluded.updated_at
    "#,
)
.bind(record.workflow_id.to_string())
.bind(record.status.as_str())
.bind(completed_json)
.bind(definition_json)
.bind(record.updated_at.to_rfc3339())
.execute(&self.pool)
.await
.map_err(|e| WorkflowRepositoryError::RepositoryError(format!("Failed to save workflow: {e}")))?;
```
Apply verbatim for `WaypointPort::save` — always bound parameters (`.bind(...)`), never string-interpolated SQL (this is also the ASVS/security mitigation called out in RESEARCH.md's Security Domain table).

**Row deserialization pattern** (lines 72-115):
```rust
fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> Result<PersistedWorkflow, WorkflowRepositoryError> {
    let workflow_id_str: String = row.try_get("workflow_id").map_err(|e| { ... })?;
    let workflow_id = Uuid::parse_str(&workflow_id_str).map_err(|e| { ... })?;
    // ... one try_get + typed-parse per column, explicit error mapping at each step
}
```
Apply the same per-column `try_get` + explicit `map_err` shape when deserializing `waypoints` rows into `Waypoint` (including the JSON-in-TEXT `payload` column deserialization D-02 specifies).

---

### `crates/paladin-storage/src/waypoint/postgres.rs` (storage adapter, CRUD/file-I/O, `postgres` feature-gated)

**Analog:** Same `sqlite_workflow_repository.rs` file as above — swap `SqlitePool`/`SqlitePoolOptions` for `PgPool`/`PgPoolOptions`, `?` placeholders for `$1, $2, ...`, and the `payload` column type from `TEXT` to `JSONB` per D-02. No Postgres adapter currently exists in the tree to copy from directly (confirmed: `ls crates/paladin-storage/src/` shows only `mysql_content_repository.rs`, `sqlite_*` files, no `postgres_*`) — this is a new-pattern file whose closest analog is still the SQLite sibling; gate the whole file behind `#[cfg(feature = "postgres")]` per D-01, and add the facade passthrough feature per RESEARCH.md Pitfall 4.

---

### `crates/paladin-storage/migrations/00X_create_waypoints_table.sql` (migration)

**Analog:** `crates/paladin-memory/migrations/001_create_garrison_tables.sql`

**Full pattern to follow:**
```sql
CREATE TABLE IF NOT EXISTS garrison_entries (
    id TEXT PRIMARY KEY NOT NULL,
    paladin_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    token_count INTEGER,
    metadata TEXT, -- JSON blob for flexible metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_paladin_timestamp ON garrison_entries(paladin_id, timestamp DESC);
```
Apply directly to `waypoints`: `TEXT PRIMARY KEY` id column (`waypoint_id`, UUIDv7 string form), `thread_id TEXT NOT NULL`, `status TEXT NOT NULL`, `payload TEXT NOT NULL` (JSON per D-02), `superstep INTEGER NOT NULL`, `created_at TEXT NOT NULL DEFAULT (datetime('now'))`, plus `CREATE INDEX IF NOT EXISTS idx_waypoints_thread_created ON waypoints(thread_id, created_at DESC)` for ENG-FR-16's access pattern.

---

### `src/config/engine.rs` / `src/config/waypoint_retention.rs` (config)

**Analog:** `src/config/citadel.rs` (full file, 214 lines, read in full) + `src/config/env_utils.rs` (full file, 227 lines, read in full)

**Struct + Default + validate() pattern** (`citadel.rs` lines 6-56):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitadelConfig {
    pub enabled: bool,
    pub state_dir: String,
    pub autosave_enabled: bool,
    pub cleanup_enabled: bool,
    pub max_state_age_days: Option<u32>,
}

impl Default for CitadelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            state_dir: "./paladin-states".to_string(),
            autosave_enabled: false,
            cleanup_enabled: false,
            max_state_age_days: Some(30),
        }
    }
}

impl CitadelConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.state_dir.trim().is_empty() {
            return Err("state_dir cannot be empty when citadel is enabled".to_string());
        }
        // ... field-by-field checks, plain String errors
        Ok(())
    }
}
```
Apply directly to `EngineConfig` (durability, `max_supersteps: u32 = 50`, `max_node_visits: u32 = 25`, `run_timeout: Option<Duration>`) and `WaypointRetentionConfig` (must never delete latest/`AwaitingInput` Waypoint per RESEARCH.md's Security Domain table).

**EnvOverridable pattern** (`citadel.rs` lines 59-85):
```rust
impl EnvOverridable for CitadelConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("APP_CITADEL_ENABLED")
            && let Ok(b) = v.parse::<bool>()
        {
            self.enabled = b;
        }
        // ... one block per field
    }
}
```
Or, preferably, use the newer `read_env` helper from `env_utils.rs` (lines 33-54) which is the more DRY variant already documented there:
```rust
pub trait EnvOverridable {
    fn apply_env_overrides(&mut self);
}

pub fn read_env<T: std::str::FromStr>(var_name: &str) -> Option<T> {
    std::env::var(var_name).ok()?.parse::<T>().ok()
}
```
Use `read_env::<u32>("APP_ENGINE_MAX_SUPERSTEPS")` etc. — shorter than `citadel.rs`'s inline `std::env::var(...).parse()` per-field blocks, and is the trait these new config structs must implement regardless of which internal style is chosen.

**Test pattern:** mirror `citadel.rs` lines 87-214 exactly — `#[test] fn test_*_config_default()`, `test_*_config_validate_valid()`, one test per validation failure branch, and a `#[test] #[serial] fn test_*_config_env_override()` using `serial_test::serial` + `unsafe { env::set_var(...) }` / `env::remove_var(...)` pairs (env var tests must be `#[serial]` to avoid cross-test interference — this is a hard convention, not optional).

---

## Shared Patterns

### Error enum construction
**Source:** `crates/paladin-core/src/platform/container/citadel_error.rs` (full file)
**Apply to:** `BattlefieldError`, `WaypointError` (ports layer), `EngineError`
```rust
#[derive(Debug, Error)]
pub enum CitadelError {
    #[error("State not found: {0}")]
    StateNotFound(Uuid),
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
```

### Config struct convention (X-09)
**Source:** `src/config/citadel.rs` + `src/config/env_utils.rs`
**Apply to:** `EngineConfig`, `WaypointRetentionConfig`
- `Default` impl with the PRD's literal defaults (50/25/Strict/vanguard-size)
- `validate(&self) -> Result<(), String>` with plain-string field checks
- `impl EnvOverridable` using `read_env::<T>("APP_...")`
- Colocated `#[cfg(test)] mod tests` with a `#[serial]` env-override test

### Async trait port shape
**Source:** `crates/paladin-ports/src/output/citadel_port.rs` lines 566-586
**Apply to:** `WaypointPort`
```rust
#[async_trait]
pub trait WaypointPort: Send + Sync {
    async fn save(&self, waypoint: &Waypoint) -> Result<(), WaypointError>;
    async fn load_latest(&self, thread_id: &ThreadId) -> Result<Option<Waypoint>, WaypointError>;
    // "None for missing, not an error" — same contract as CitadelPort
}
```

### SQL adapter shape (bound parameters, never string interpolation)
**Source:** `crates/paladin-storage/src/sqlite_workflow_repository.rs` (full file)
**Apply to:** `SqliteWaypointStore`, `PostgresWaypointStore`
- Struct wraps a `SqlitePool`/`PgPool`
- `new(database_url)` connects then runs migrations
- Every write uses `.bind(...)` — never `format!()` into a query string (ASVS V5/SQL-injection mitigation, see RESEARCH.md Security Domain)
- Row → domain-type deserialization via explicit `try_get` + `map_err` per column

### Concurrent fan-out/join
**Source:** `crates/paladin-battalion/src/phalanx_service.rs::execute_collect_all` lines 323-376
**Apply to:** superstep Vanguard execution in `engine/superstep.rs`
- `tokio::spawn` per node with a cloned `Arc<dyn PaladinPort>`/read-only snapshot handle
- Optional `Semaphore` for concurrency bound (here: parallelism defaults to vanguard size, so likely unbounded/no semaphore needed unless `EngineLimits` adds a cap)
- Collect `JoinHandle`s first, `.await` them in a second loop — never block the spawn loop on a running task

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/paladin-battalion/src/engine/dispatch_registry.rs` | service (registry) | CRUD | No existing "custom rule/strategy registry" pattern in the tree; nearest sibling (`ArsenalRegistry`, referenced in CLAUDE.md but not read this session) may be worth a follow-up look during implementation, but was out of this pass's 3-5-analog budget |
| `crates/paladin-battalion/src/engine/input_mapping.rs` | utility (template resolution) | transform | No `{field}`-placeholder template resolver exists elsewhere in the tree; this is genuinely new logic per X-03's bridge requirement — implement from the PRD's §3 type shapes directly |
| `crates/paladin-storage/src/waypoint/postgres.rs` | storage adapter | CRUD/file-I/O | No Postgres adapter exists anywhere in the tree yet (`paladin-storage` only has `mysql_content_repository.rs` + `sqlite_*`); use the SQLite sibling as the structural analog (already captured above) but expect more original work on `PgPoolOptions`/`$N` bind syntax than a true "copy" |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`, `crates/paladin-ports/src/output/`, `crates/paladin-battalion/src/`, `crates/paladin-storage/src/`, `crates/paladin-memory/migrations/`, `src/config/`, `src/application/services/orchestration/`
**Files read in full this session:** `citadel.rs` (partial, 120/672 lines — sufficient for pattern extraction), `citadel_error.rs` (162/162), `citadel_port.rs` (663/663), `token_usage.rs` (75/75), `src/config/citadel.rs` (214/214), `src/config/env_utils.rs` (227/227), `campaign_service.rs` (223-352 of 618), `phalanx_service.rs` (323-376 of 914), `sqlite_workflow_repository.rs` (1-190 of ~260)
**Pattern extraction date:** 2026-09-01
