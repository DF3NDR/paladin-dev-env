# Phase 40: Tenant Identity & Run-Read Scoping - Pattern Map

**Mapped:** 2026-09-28
**Files analyzed:** 17 (new/modified)
**Analogs found:** 17 / 17 — this phase is a pure in-tree extension of already-cited code; RESEARCH.md
already contains exact file:line excerpts for every analog, verified against the live tree this
session. This PATTERNS.md organizes those excerpts by target file for the planner; it does not
re-derive them.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/src/platform/container/principal.rs` (NEW: `TenantId`, `PrincipalRef`, `RunAttribution`, `RunReadScope`) | model (value type) | transform | `crates/paladin-core/src/platform/container/waypoint.rs` (`ThreadId`) | exact |
| `crates/paladin-core/src/platform/container/run.rs` (`submitted_by` field) | model | CRUD | same file, existing `Run` builder (`with_*`) | exact |
| `crates/paladin-core/src/platform/container/run_scope.rs` (`ledger_scope` field) | model | transform | same file, existing `RunScope` builders (`with_run_id`, `with_vault_namespace`) | exact |
| `crates/paladin-core/src/platform/container/treasury_ledger.rs` (`LedgerScope::from_attribution`) | utility | transform | same file, existing `LedgerScope::unattributed()` | exact |
| `crates/paladin-web/src/agent_auth.rs` (`Principal.tenant_id`, `Principal::read_scope()`, `AgentAuthConfig` bearer tenant) | middleware / auth | request-response | same file, `authenticate()`, `authorize_invoke`, `require_admin` | exact |
| `crates/paladin-web/src/run_controller.rs` (`load_visible_run`, route wiring, `RunResponse.submitted_by`) | controller | request-response | same file, `get_run` current 404 pattern | exact |
| `crates/paladin-ports/src/input/run_submission_port.rs` (`SubmitRun`/`ForkRun`/`cancel` → `PrincipalRef`) | route (port trait) | request-response | same file, current `(String, UserRole)` tuple usage | exact |
| `crates/paladin-ports/src/output/run_repository_port.rs` (`RunQuery.scope`) | route (port trait) | CRUD | same file, existing `RunQuery` fields | exact |
| `crates/paladin-ports/src/output/paladin_executor_port.rs` (`execute_scoped` defaulted method) | route (port trait) | request-response | `crates/paladin-ports/src/output/paladin_port.rs` (`execute_scoped` precedent) + `run_repository_port.rs` `insert_with_latest` (X-10.4 defaulted-method precedent) | exact |
| `crates/paladin-storage/migrations/sqlite/008_add_run_attribution_columns.sql` | migration | batch | `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` | exact |
| `crates/paladin-storage/migrations/postgres/008_add_run_attribution_columns.sql` | migration | batch | `crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql` (header style) | role-match |
| `crates/paladin-storage/src/run/sqlite.rs` (INSERT/SELECT/list/row_to_run) | service (adapter) | CRUD | same file, current `list()` `QueryBuilder` chain | exact |
| `crates/paladin-storage/src/run/postgres.rs` (mirror of sqlite.rs) | service (adapter) | CRUD | `crates/paladin-storage/src/run/sqlite.rs` | exact |
| `crates/paladin-storage/src/run/in_memory.rs` (`.filter(query.scope.permits)`) | service (adapter) | CRUD | same file, existing `thread_id`/`assistant_id`/`status` filters | exact |
| `crates/paladin-storage/src/run/contract_tests.rs` (scoped-list + attribution round-trip) | test | CRUD | same file, `list_filters_by_thread_assistant_and_status` | exact |
| `src/config/agents.rs` (`ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `AuthConfig::validate()`) | config | transform | `src/config/run_store.rs` (`validate()` fail-closed precedent) | role-match |
| `src/bin/paladin-server.rs` (`build_auth_config`) | config / bootstrap | request-response | same file, current `api_keys` → `Principal` map construction | exact |
| `src/application/services/run/submission.rs` (`authorize_invocation`, `submit`, `fork`) | service | request-response | same file, current tuple-based `requested_by` handling | exact |
| `src/application/services/run/worker.rs` (`SettlementContext.scope`, `RunScope.ledger_scope`) | service | event-driven | same file, current hard-coded `LedgerScope::unattributed()` | exact |
| `src/application/services/paladin/paladin_execution_service.rs` (`settle_agent_loop_call`, `settle_model_call`) | service | event-driven | same file, current hard-coded `LedgerScope::unattributed()` | exact |
| `crates/paladin-web/src/agent_controller.rs` (`execute_agent`/`execute_agent_stream` → `execute_scoped`) | controller | request-response | same file, current `entry.executor.execute(paladin, input)` call | exact |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/principal.rs` (model, transform) — NEW FILE

**Analog:** `crates/paladin-core/src/platform/container/waypoint.rs` lines 44-77 (`ThreadId`)

**Identifier-newtype pattern to mirror exactly:**
```rust
pub struct ThreadId(String);

pub enum ThreadIdError {
    Empty,
    TooLong { len: usize },
    ContainsWhitespace,
}

impl ThreadId {
    pub fn new(id: impl Into<String>) -> Result<Self, ThreadIdError> {
        let id = id.into();
        if id.is_empty() { return Err(ThreadIdError::Empty); }
        if id.len() > THREAD_ID_MAX_LEN { return Err(ThreadIdError::TooLong { len: id.len() }); }
        if id.chars().any(char::is_whitespace) { return Err(ThreadIdError::ContainsWhitespace); }
        Ok(Self(id))
    }
}
```
`THREAD_ID_MAX_LEN = 256` (`waypoint.rs:35`). Apply the identical three-variant error enum shape to
`TenantId::new(impl Into<String>) -> Result<Self, TenantIdError>` (bound is planner's discretion per
CONTEXT D-01, but must mirror this structure, not invent a new one). Also give `TenantId` `as_str()`,
`Display`, `Serialize`/`Deserialize`, `Hash`/`Eq` (`ThreadId` already has these; read the rest of
`waypoint.rs` for the derive/impl block to copy if needed).

**`RunReadScope` core logic (write fresh, no analog needed — it's new domain logic; the "one shared
function" requirement is PLAT-07's whole point):**
```rust
pub enum RunReadScope {
    All,
    Tenant(TenantId),
}
impl RunReadScope {
    pub fn for_principal(role: UserRole, tenant_id: &TenantId) -> Self {
        if role == UserRole::Admin { Self::All } else { Self::Tenant(tenant_id.clone()) }
    }
    pub fn permits(&self, run: &Run) -> bool {
        match self {
            Self::All => true,
            Self::Tenant(t) => run.submitted_by.as_ref().is_some_and(|a| &a.tenant_id == t),
        }
    }
}
impl Default for RunReadScope {
    fn default() -> Self { Self::All } // internal callers unchanged — D-12
}
```

**`PrincipalRef` and `RunAttribution`:** small value structs, no analog needed beyond standard
`#[derive(Debug, Clone, PartialEq, Eq)]` house style seen throughout `paladin-core`.

---

### `crates/paladin-core/src/platform/container/run.rs` (model, CRUD)

**Analog:** same file, `Run`'s existing `#[non_exhaustive]` + builder pattern (lines 355-460)

**Addition:**
```rust
#[serde(default)]
pub submitted_by: Option<RunAttribution>,
```
plus `Run::with_submitted_by(mut self, attribution: RunAttribution) -> Self`.

**Schema-version rule — do NOT bump.** Governing precedent (`waypoint.rs:1805-1811`, passing test):
```rust
fn battlefield_schema_version_is_unchanged() {
    // Every Phase 25 Waypoint addition is `#[serde(default)]`, so the
    // schema version pinned on `main` before this phase must still hold
    // (X-04 does not require a bump for a purely additive change).
    assert_eq!(BATTLEFIELD_SCHEMA_VERSION, "1.0.0");
}
```
`RUN_SCHEMA_VERSION` stays `"v1"`. The SQLite adapter's strict-equality read guard
(`crates/paladin-storage/src/run/sqlite.rs:285-289`) would reject every pre-bump row if bumped —
confirmed anti-pattern, do not do this.

---

### `crates/paladin-core/src/platform/container/run_scope.rs` (model, transform)

**Analog:** same file, existing `with_run_id`/`with_vault_namespace` builders (lines 95-119)

**Addition:** `ledger_scope: Option<LedgerScope>` field (additive, `None` default) +
`with_ledger_scope(mut self, scope: LedgerScope) -> Self` builder, same shape as the existing
builders in this file.

---

### `crates/paladin-core/src/platform/container/treasury_ledger.rs` (utility, transform)

**Analog:** same file, existing `LedgerScope::unattributed()`

```rust
impl LedgerScope {
    pub fn from_attribution(attribution: Option<&RunAttribution>) -> Self {
        match attribution {
            Some(a) => Self::new(a.tenant_id.as_str(), &a.api_key_id),
            None => Self::unattributed(),
        }
    }
}
```
**Do not confuse with `RunAttribution = None` on the `runs` row** — `NULL` on the row is never the
literal string `"unattributed"`; that sentinel is ledger-row-only (`LedgerScope::UNATTRIBUTED`,
`treasury_ledger.rs:63`), produced only by `from_attribution(None)`.

---

### `crates/paladin-web/src/agent_auth.rs` (middleware/auth, request-response)

**Analog:** same file — `Principal` struct (lines 37-42), `authenticate()` (lines 144-151, 261),
`Principal::open_access()` (lines 44-51), neighborhood of `authorize_invoke`/`require_admin`
(lines 195-220) for the new `read_scope()` method.

**Current `Principal` (before):**
```rust
#[derive(Debug, Clone)]
pub struct Principal {
    pub id: String,
    pub role: UserRole,
}
```
Add required `tenant_id: TenantId` (no `Option` — D-01).

**Every production construction site to update (confirmed via `grep -rn "Principal {"`):**
- `agent_auth.rs:147` — bearer-token branch of `authenticate()`
- `agent_auth.rs:261` — API-key path (value in map built by `build_auth_config`)
- `agent_auth.rs:44-51` — `Principal::open_access()` body: use `TenantId::OPEN_ACCESS` sentinel + `role: Admin`
- `src/bin/paladin-server.rs:372` — `build_auth_config`'s `api_keys` map construction

**Test-only construction sites needing `tenant_id` too (~14, non-exhaustive listed in RESEARCH.md):**
`agent_auth.rs:261,349,365,369`; `agent_controller.rs:822,830`; `run_controller.rs:1852,2286`;
`assistant_controller.rs:932,939`; `thread_controller.rs:1181,1749,1756,2299`;
`schedule_controller.rs:760,767`; `examples/http_service_host.rs:62`;
`examples/platform_api_client.rs:144` (examples must compile: `cargo build --examples --features
web-server,dev-ui`).

**Example test helper (`run_controller.rs:1851-1856`, current):**
```rust
fn tester_principal() -> Extension<Principal> {
    Extension(Principal {
        id: "tester".to_string(),
        role: paladin_core::platform::container::user::UserRole::Admin,
    })
}
```
Needs `tenant_id: TenantId::new("tester-tenant").unwrap()` added, plus a **second** principal in a
different tenant for the D-13 cross-tenant 404 matrix.

**`Principal::read_scope()` (new method beside `authorize_invoke`/`require_admin`):**
```rust
impl Principal {
    pub fn read_scope(&self) -> RunReadScope {
        RunReadScope::for_principal(self.role, &self.tenant_id)
    }
}
```

**Bearer tenant on `AgentAuthConfig`:** planner's discretion on field placement; required when
`bearer_token.enabled` is true, validated at config `validate()` time.

---

### `crates/paladin-web/src/run_controller.rs` (controller, request-response)

**Analog:** same file — current `get_run` 404 pattern (lines 813-817), router assembly (`run_openapi_router`, lines 1114-1129)

**404-for-hidden-resource pattern to copy verbatim:**
```rust
// Source: run_controller.rs:813-817 (get_run, current code)
let run = repository
    .get(&id)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;
```

**New `load_visible_run` helper (the ONE shared authorization function, D-12/D-13):**
```rust
async fn load_visible_run(
    repository: &Arc<dyn RunRepositoryPort>,
    principal: &Principal,
    run_id: &RunId,
) -> Result<Run, ApiError> {
    let run = repository.get(run_id).await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;
    if !principal.read_scope().permits(&run) {
        return Err(ApiError::not_found(format!("unknown run '{run_id}'")));
    }
    Ok(run)
}
```
Same 404 literal, same `ApiError::not_found` constructor as the miss-path — a scoped miss must be
byte-identical to a genuine miss (no `403`, no timing/shape difference — D-12).

**Four handlers to route through `load_visible_run`** (confirmed exhaustively against
`run_openapi_router`, lines 1114-1129):
- `get_run` (currently `run_controller.rs:813-817`)
- `stream_run` (currently NO visibility check at all before SSE upgrade, `run_controller.rs:1073-1104`)
- `cancel_run` (currently `submission.cancel(&id, Some((principal.id.clone(), principal.role)))` with
  no prior visibility check, `run_controller.rs:938-962`; visibility check goes BEFORE the existing
  `allowed_roles` check inside `RunSubmissionService`)
- `list_webhook_deliveries` (currently `deliveries.list_for_run(&id, ...)` with no check,
  `run_controller.rs:988-1017`; `list_for_run` itself stays untouched — `load_visible_run` gates from outside)

`submit_run` and `list_runs` are NOT gated by `load_visible_run` — `submit_run` creates a new run
(nothing to check yet); `list_runs` uses `RunQuery.scope` at the SQL layer instead (see storage
adapter patterns below).

**`RunResponse.submitted_by` addition (mirrors the `cost` field precedent from 39-06):**
```rust
#[serde(default)]
pub submitted_by: Option<RunAttributionDto>, // { tenant_id, api_key_id } — never role, never key value
```
Regeneration step: `UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline`
(or `make openapi`) — do NOT touch `openapi_golden_v0_9.rs` (`/v1/runs*` postdates the frozen v0.9.0
baseline entirely; see Pitfall 6 in RESEARCH.md).

---

### `crates/paladin-ports/src/input/run_submission_port.rs` (route/port, request-response)

**Analog:** same file, current `(String, UserRole)` tuple usage in `SubmitRun.requested_by`,
`ForkRun.requested_by`, `RunSubmissionPort::cancel(.., requested_by)`

Replace the tuple with `Option<PrincipalRef>` where `PrincipalRef { api_key_id: String, tenant_id:
TenantId, role: UserRole }` lives in `paladin-core`'s new `principal.rs`; `From<&paladin_web::
Principal>` lives in `paladin-web`. `None` keeps the existing "internal caller, skip role check"
semantics.

---

### `crates/paladin-ports/src/output/run_repository_port.rs` (route/port, CRUD)

**Analog:** same file, existing `RunQuery` struct fields

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunQuery {
    pub thread_id: Option<ThreadId>,
    pub assistant_id: Option<String>,
    pub status: Option<RunStatus>,
    pub limit: u32,
    pub cursor: Option<RunCursor>,
    pub scope: RunReadScope,   // NEW; Default = All (internal callers unchanged, D-12)
}
```
Verified safe against internal callers: `submission.rs:412-420`'s `fork` lookup uses `RunQuery {
thread_id: Some(...), limit: 1, ..Default::default() }` and must keep seeing every run on the thread
regardless of tenant — confirmed by Pitfall 8 in RESEARCH.md.

---

### `crates/paladin-ports/src/output/paladin_executor_port.rs` (route/port, request-response)

**Analog:** `crates/paladin-ports/src/output/paladin_port.rs`'s `execute_scoped` default-method
precedent, and the X-10.4 defaulted-trait-method precedent at `run_repository_port.rs:338-368`
(`insert_with_latest`, needed no allowlist entry).

**Addition — additive, defaulted, non-breaking for every existing implementor:**
```rust
async fn execute_scoped(
    &self,
    paladin: &Paladin,
    input: &str,
    scope: &RunScope,
) -> Result<PaladinResult, PaladinError> {
    let _ = scope; // default ignores scope — behavior-identical for any implementor that doesn't override
    self.execute(paladin, input).await
}
```
`PaladinExecutionService`'s impl overrides it to call its own inherent `execute_scoped`. This is the
architectural gap RESEARCH.md's Pitfall 1 flags explicitly — required for D-16's agent-execute
attribution; do not skip it as "just a controller change."

---

### `crates/paladin-storage/migrations/{sqlite,postgres}/008_add_run_attribution_columns.sql` (migration, batch)

**Analog:** `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` (unguarded, versioned
`ALTER TABLE ADD COLUMN` inside `sqlx::migrate!` — the CORRECT precedent) — NOT
`crates/paladin-storage/src/sqlite_user_repository.rs:87-91` (that one runs on every construction
outside `sqlx::migrate!` and needs duplicate-column error swallowing; do not copy that guard here).

**Header style analog:** `crates/paladin-storage/migrations/sqlite/002_create_runs_table.sql` and
`postgres/007_create_treasury_ledger_table.sql` (`-- Migration:`, `-- Purpose:`, `-- Version:`, `-- Date:`).

**Concrete SQLite file:**
```sql
-- Migration: Add Run Attribution Columns
-- Purpose: Persist the submitting principal's tenant/API-key on each run (TENANT-01, TENANT-02)
-- Version: 008
-- Date: <plan date>
--
-- NULL means "no principal recorded" (pre-v0.11 rows, schedule-fired runs, same-process
-- embedders, tests) -- never the ledger's "unattributed" string sentinel (that sentinel is a
-- treasury_ledger-row concept, LedgerScope::UNATTRIBUTED, not a runs-row value).

ALTER TABLE runs ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE runs ADD COLUMN api_key_id TEXT NULL;

-- Serves the scoped keyset list: WHERE tenant_id = ? combined with the existing
-- (submitted_at DESC, run_id DESC) ordering.
CREATE INDEX IF NOT EXISTS idx_runs_tenant_submitted
ON runs(tenant_id, submitted_at DESC, run_id DESC);
```
Postgres `008` is textually identical (no dialect difference for this DDL shape).
No `IF NOT EXISTS`/guard needed on the `ALTER TABLE` lines themselves — `sqlx::migrate!` tracks
applied versions and runs this file exactly once per database.

---

### `crates/paladin-storage/src/run/sqlite.rs` (service/adapter, CRUD)

**Analog:** same file, current `list()` `QueryBuilder` chain (lines 479-500)

```rust
// Source: sqlite.rs:479-500 (list, current code)
let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(LIST_SELECT_PREFIX);
if let Some(thread_id) = &query.thread_id {
    builder.push(" AND thread_id = ");
    builder.push_bind(thread_id.as_str().to_string());
}
// ... assistant_id, status identical shape ...
if let Some(cursor) = &query.cursor {
    builder.push(" AND (submitted_at < "); /* ... */
}
builder.push(" ORDER BY submitted_at DESC, run_id DESC LIMIT ");
builder.push_bind(fetch_limit);
```
**Add the scope filter as one more `if let` in this exact chain, grouped with the other equality
filters, BEFORE the cursor clause:**
```rust
if let RunReadScope::Tenant(tenant) = &query.scope {
    builder.push(" AND tenant_id = ");
    builder.push_bind(tenant.as_str().to_string());
}
```
**Anti-pattern — do not post-filter in Rust after `list()` returns.** Breaks keyset pagination
correctness (`limit`/`next_cursor` can under-fill or desync). The filter MUST live inside this same
`QueryBuilder` chain (Pitfall 2 in RESEARCH.md).

**Other changes in this file:**
- `INSERT_RUN`/`INSERT_RUN_WITH_LATEST` (lines 44-48, 81-86): add `tenant_id, api_key_id` columns
- `SELECT_RUN_BY_ID`/`SELECT_ACTIVE_RUN_FOR_THREAD`/`LIST_SELECT_PREFIX` (lines 50-70): same
- `row_to_run` (lines 221-314): read as `Option<String>`, construct `Option<RunAttribution>`
- `insert` (lines 319-378): bind `run.submitted_by.as_ref().map(|a| a.tenant_id.as_str())` etc.

---

### `crates/paladin-storage/src/run/postgres.rs` (service/adapter, CRUD)

**Analog:** `crates/paladin-storage/src/run/sqlite.rs` — mirrors 1:1 (`LIST_SELECT_PREFIX`/`list()` at
postgres.rs:76-79, 455-467 confirmed identical `QueryBuilder` shape). Apply the same changes.

---

### `crates/paladin-storage/src/run/in_memory.rs` (service/adapter, CRUD)

**Analog:** same file, existing `thread_id`/`assistant_id`/`status` `.filter()` chain (lines 151-165)

```rust
// add alongside existing filters (lines 155-162):
.filter(|r| query.scope.permits(r))
```

---

### `crates/paladin-storage/src/run/contract_tests.rs` (test, CRUD)

**Analog:** same file, existing `list_filters_by_thread_assistant_and_status` (referenced from
`in_memory.rs:523-526`)

Extend with: (1) a scoped-list case (tenant A cannot see tenant B's runs via `list()`), (2) an
attribution round-trip case (insert with `submitted_by`, read back, assert equal). Run against all
three adapters per the existing house pattern (this file already parameterizes over adapters).

---

### `src/config/agents.rs` (config, transform)

**Analog:** `src/config/run_store.rs` lines 103-126 (`validate()` fail-closed precedent)

```rust
// Source: run_store.rs:103-126 (current code)
pub fn validate(&self) -> Result<(), String> {
    ...
    Err(format!("run store postgres backend names env var '{url_env}', which is not set"))
}
```

**Current `ApiKeyConfig` (agents.rs:80-88, before):**
```rust
pub struct ApiKeyConfig {
    pub key: String,
    pub name: String,
    pub role: UserRole,
}
```
Add required `tenant: String` (parsed into `TenantId` at validation). `BearerTokenAuthConfig`
(agents.rs:91-96) gains `tenant: Option<String>`, required when `enabled: true`.

**New `AuthConfig::validate()` (none exists today — confirmed by reading the whole file,
agents.rs:102-127)** — rejects duplicate `name`, duplicate `key`, malformed `tenant`. Exact error
voice supplied by CONTEXT.md `<specifics>`:
```
http.auth.api_keys[<name>]: 'tenant' is required — every API key must map to a tenant (Phase 40, TENANT-01)
```

**Existing tests to extend as the sibling pattern:** `src/config/agents.rs:246-339` (6 tests
including `auth_config_parses_api_keys_with_roles`, `auth_config_rejects_unknown_role` — the
tenant-required test is new, matching `auth_config_rejects_unknown_role`'s shape).

---

### `src/bin/paladin-server.rs` (config/bootstrap, request-response)

**Analog:** same file, current `build_auth_config` `api_keys` map construction (lines 366-378)

```rust
// Source: paladin-server.rs:366-378 (current code)
let api_keys: HashMap<String, Principal> = cfg
    .api_keys
    .iter()
    .map(|k| (k.key.clone(), Principal { id: k.name.clone(), role: k.role }))
    .collect();
```
Becomes `Principal { id: k.name.clone(), role: k.role, tenant_id: <parsed from k.tenant> }`. Bearer
branch (lines 384-389) needs `AgentAuthConfig` to carry the configured bearer tenant so
`authenticate()`'s bearer branch can read it.

**Existing tests to extend:** `paladin-server.rs:695-744`
(`build_auth_config_warns_when_in_process_token_store_is_wired`,
`build_auth_config_fails_closed_when_enabled_with_no_credentials`).

---

### `src/application/services/run/submission.rs` (service, request-response)

**Analog:** same file, current tuple-based `requested_by` in `authorize_invocation`, `submit`, `fork`

`authorize_invocation` reads `.role` from `Option<PrincipalRef>` instead of the tuple.
`submit`/`fork` call `run.with_submitted_by(requested_by.as_ref().map(|p| p.attribution()))` (or
equivalent) to populate the new `Run` field. The `fork` lookup at `submission.rs:412-420` (`RunQuery
{ thread_id: Some(...), limit: 1, ..Default::default() }`) is unaffected because `RunReadScope`
defaults to `All`.

---

### `src/application/services/run/worker.rs` (service, event-driven)

**Analog:** same file, current hard-coded `SettlementContext` site (lines 965-973)

```rust
// Source: worker.rs:965-973 (current code, hard-coded)
if let Some(ledger) = &self.treasury_ledger {
    engine = engine.with_treasury_ledger(
        Arc::clone(ledger),
        SettlementContext {
            scope: LedgerScope::unattributed(),   // <-- replace
            run_id: run.run_id.clone(),
            attempt,
        },
    );
}
```
Replace with `scope: LedgerScope::from_attribution(run.submitted_by.as_ref())`. Also set
`RunScope.ledger_scope` from the run row for `run_agent`'s path.

---

### `src/application/services/paladin/paladin_execution_service.rs` (service, event-driven)

**Analog:** same file, current hard-coded settle sites

```rust
// Source: paladin_execution_service.rs:393-406 (current code)
async fn settle_agent_loop_call(
    ledger: &Arc<dyn TreasuryLedgerPort>,
    run_id: &RunId,
    ordinal: u64,
    model: &str,
    cost: &Cost,
) {
    let key = SettlementKey::new(run_id.clone(), ordinal, 1);
    let request = SettleRequest::unreserved(
        LedgerScope::unattributed(),   // <-- replace
        key,
        cost.clone(),
        BTreeMap::from([(model.to_string(), cost.nanos())]),
    );
    ...
}
```
Thread a `scope: &LedgerScope` (or `Option<&LedgerScope>`) parameter through
`settle_agent_loop_call`, `settle_model_call` (lines 991-996), and both call sites
(`execute_scoped`'s internal path, `execute_stream_inner`'s spawned-task path at lines 3574-3578),
reading it from `RunScope.ledger_scope`, falling back to `LedgerScope::unattributed()` when `None`.

---

### `crates/paladin-web/src/agent_controller.rs` (controller, request-response)

**Analog:** same file, current `entry.executor.execute(paladin, input)` call

`execute_agent`/`execute_agent_stream` build `RunScope::default().with_ledger_scope(<derived from
principal>)` and call `execute_scoped` (the new `PaladinExecutorPort` method) instead of `execute` —
this is the plumbing that makes D-16's "agent-execute spend is attributed too" actually reachable
(see Pitfall 1 above).

## Shared Patterns

### 404-for-hidden-resource (identity of miss vs. hidden)
**Source:** `crates/paladin-web/src/run_controller.rs:813-817`, `817`
**Apply to:** `load_visible_run` and every one of its four call sites (`get_run`, `stream_run`,
`cancel_run`, `list_webhook_deliveries`). Never emit `403` for a visibility failure — always the
exact same `404 "unknown run '{id}'"` literal as a genuinely missing row.

### Dynamic SQL filter composition via `QueryBuilder`
**Source:** `crates/paladin-storage/src/run/sqlite.rs:479-500` (mirrored in `postgres.rs`)
**Apply to:** the `scope` filter in all three `RunRepositoryPort::list` adapters. Must sit inside the
query builder chain before `ORDER BY ... LIMIT`, never as a post-fetch Rust-side filter.

### Fail-closed config validation, named-entry error message
**Source:** `src/config/run_store.rs:103-126`; error voice supplied verbatim in CONTEXT.md
**Apply to:** `AuthConfig::validate()` (new), `ApiKeyConfig.tenant` requirement,
`BearerTokenAuthConfig.tenant` requirement.

### Additive `#[non_exhaustive]` struct evolution + `#[serde(default)]`
**Source:** `crates/paladin-core/src/platform/container/run.rs` (`Run`'s existing builder pattern);
`crates/paladin-core/src/platform/container/waypoint.rs:1805-1811` (X-04 no-bump precedent)
**Apply to:** `Run.submitted_by`, `RunScope.ledger_scope`, `RunResponse.submitted_by`. Never bump
`RUN_SCHEMA_VERSION` for these additions.

### Identifier newtype validation
**Source:** `crates/paladin-core/src/platform/container/waypoint.rs:44-77` (`ThreadId`)
**Apply to:** `TenantId::new`. Three-variant error enum (`Empty`/`TooLong`/`ContainsWhitespace`),
bound is planner's discretion.

### Defaulted trait-method addition (non-breaking port evolution)
**Source:** `crates/paladin-ports/src/output/run_repository_port.rs:338-368` (`insert_with_latest`,
X-10.4 precedent); `crates/paladin-ports/src/output/paladin_port.rs` (`execute_scoped` precedent)
**Apply to:** `PaladinExecutorPort::execute_scoped` — verify via the Pitfall 5 `cargo semver-checks`
diagnostic (temporarily disable the crate-wide `allow`, run check-release, observe, revert) whether a
new `.cargo/semver-checks-allowlist.toml` entry is actually needed; all three touched crates
(`paladin-web`, `paladin-ports`, `paladin-core`) already crate-wide `allow`
`constructible_struct_adds_field`.

### Versioned, unguarded `ALTER TABLE ADD COLUMN` migration
**Source:** `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql:8`
**Apply to:** `008_add_run_attribution_columns.sql` (both sqlite and postgres). Do NOT copy the
duplicate-column-swallowing guard from `crates/paladin-storage/src/sqlite_user_repository.rs:87-91`
— that pattern exists only because that file's `ALTER TABLE` runs outside `sqlx::migrate!`'s
once-per-database tracking.

### Additive DTO field with openapi drift-guard regeneration
**Source:** `RunResponse.cost` (39-06 precedent); regeneration via
`crates/paladin-web/src/openapi.rs::openapi_matches_committed_baseline`
**Apply to:** `RunResponse.submitted_by`. Do NOT touch `openapi_golden_v0_9.rs` — `/v1/runs*` is
outside that file's frozen `V0_9_PATHS` scope entirely (Pitfall 6).

## No Analog Found

None. Every file in scope has a confirmed, cited, in-tree analog — this phase is a pure extension of
already-shipped patterns from Phases 27, 38, and 39.

## Metadata

**Analog search scope:** No new search was performed; this PATTERNS.md reorganizes RESEARCH.md's
already-exhaustive, line-cited codebase analysis (every excerpt traces to a specific file:line read
during the research session) into a per-target-file, planner-facing structure.
**Files scanned:** 0 additional (research already read every relevant file in full or via targeted
ranges — see RESEARCH.md "Sources" section for the complete list of ~30 files).
**Pattern extraction date:** 2026-09-28
