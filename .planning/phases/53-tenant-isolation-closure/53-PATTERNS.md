# Phase 53: Tenant Isolation Closure - Pattern Map

**Mapped:** 2026-10-31
**Files analyzed:** 13 modified code files, 5 doc/evidence files (no new source files)
**Analogs found:** 13 / 13 (all in-repo; most are same-file analogs)

Line numbers below were read this session from the tree; re-check before quoting.

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/paladin-core/src/platform/container/principal.rs` | model (value rule + consts) | transform | `RunReadScope::permits` (same file, ~l.216-236); `TenantId::OPEN_ACCESS` (l.67) | exact |
| `src/application/services/run/submission.rs` (`ensure_thread_visible`) | service | request-response | itself (l.269-296) | exact |
| `crates/paladin-web/src/run_controller.rs` (shared mapper, `load_visible_run`) | controller | request-response | `load_visible_run` l.892 + `internal_repo_error` l.688 | exact |
| `crates/paladin-web/src/thread_controller.rs` (`load_visible_thread`, list walk, matrix tests) | controller | request-response + paging | `load_visible_run`; `published_run_id_operations` (run_controller.rs l.3167); matrix test l.3215 | role-match |
| `crates/paladin-web/src/job_store.rs` | store | CRUD (in-memory) | itself (`Inner`, `create`, `get`) | exact |
| `crates/paladin-web/src/agent_controller.rs` (`enqueue_job`, `get_job`) | controller | request-response | `enqueue_job` l.835 (already takes `Extension<Principal>`) | exact |
| `src/application/services/run/worker.rs` (`has_engine_factory`) | service | inspector | builder `with_engine_factory` | role-match |
| `src/infrastructure/web/run_api_wiring.rs` (assertion, schedule scan) | config / wiring | boot-time | `build_run_api_with_cadence` pool build l.850-890 | exact |
| `src/config/agents.rs` (`AuthConfig::validate`) | config | validation | existing `TenantId::new` checks l.193-235 | exact |
| `crates/paladin-web/src/thread_controller.rs` tests | test | request-response | run matrix test in `run_controller.rs` | exact |
| `.planning/decisions/0063-*.md`, `PROMOTION.md`, `MIGRATION.md` §11.1/11.6, CHANGELOGs, `.project/current-exports.txt`, `53-VERSION-DECISION.md`, `53-TENANT-EVIDENCE.md`, WINDOWS rows 58/63 | docs | n/a | existing ADR 0054 + PROMOTION line | role-match |

## Pattern Assignments

### `principal.rs` -- `RunReadScope::permits_thread` + reserved consts (model, transform)

**Analog:** same file, `RunReadScope` (l.200-236).

```rust
impl RunReadScope {
    pub fn for_principal(role: UserRole, tenant_id: &TenantId) -> Self {
        match role {
            UserRole::Admin => Self::All,
            UserRole::User => Self::Tenant(tenant_id.clone()),
        }
    }
    pub fn permits(&self, run: &Run) -> bool {
        match self {
            Self::All => true,
            Self::Tenant(t) => run.submitted_by.as_ref().is_some_and(|a| &a.tenant_id == t),
        }
    }
}
```
Add `permits_thread(&self, latest: Option<&Run>) -> bool` as an additive inherent method: `(All,_)=>true`, `(Tenant,None)=>false`, `(Tenant,Some(r))=>self.permits(r)`. Needs a doctest (public API). Place the consts beside `TenantId::OPEN_ACCESS` (l.67, `pub const OPEN_ACCESS: &'static str = "open-access"`); build `RESERVED_PRINCIPAL_LITERALS` from `TenantId::OPEN_ACCESS` and `LedgerScope::UNATTRIBUTED` so literals have one source. Second const for key-name-only `"anonymous"`. Unit tests live in `#[cfg(test)] mod tests` (l.238+, helper `tenant(s)`).

### `submission.rs::ensure_thread_visible` (service)

**Analog:** itself, l.269-296. Keep the early returns (no principal -> Ok, `All` -> Ok) and the `RunQuery{thread_id, limit:1, ..Default::default()}` lookup; replace the `permits` arm with `scope.permits_thread(Some(&latest))`. **Keep `None => Ok(())`** (thread birth) - do not copy the read gate's no-run rule here (research Pitfall 2).
```rust
match page.items.into_iter().next() {
    Some(latest) if !scope.permits(&latest) => Err(RunSubmissionError::UnknownThread { thread_id: thread_id.clone() }),
    _ => Ok(()),
}
```

### `run_controller.rs` -- shared role-aware mapper (controller)

**Analog:** l.681-691 and l.892-911.
```rust
const RUN_STORE_ERROR_MESSAGE: &str = "run store error";
fn internal_repo_error(context: &str, err: impl std::fmt::Display) -> ApiError {
    log::error!("run API {context} failed: {err}");
    ApiError::internal(RUN_STORE_ERROR_MESSAGE)
}
async fn load_visible_run(repository: &Arc<dyn RunRepositoryPort>, principal: &Principal, run_id: &RunId) -> Result<Run, ApiError> {
    let run = repository.get(run_id).await
        .map_err(|e| internal_repo_error("run lookup", e))?
        .ok_or_else(|| ApiError::not_found(format!("unknown run '{run_id}'")))?;
    if !principal.read_scope().permits(&run) {
        return Err(ApiError::not_found(format!("unknown run '{run_id}'")));
    }
    Ok(run)
}
```
Change: add a `pub(crate)` mapper (`repo_error_for_scope(scope, context, err, hidden)`): `Serialization` + `Tenant` -> `log::error!` then `hidden()`; everything else -> `internal_repo_error`. `load_visible_run` uses it for the `get` error. Also route `/stream`, `/webhook-deliveries`, `/cancel` loads through the same helper (they enter via `load_visible_run`). Module docs' "not tenant-scoped" paragraph at the end is stale - rewrite.

### `thread_controller.rs` -- `load_visible_thread` (controller)

**Analog:** `load_visible_run` above; current un-gated handler shape (`get_thread`, l.942-975):
```rust
let waypoints = state.waypoints.as_ref().ok_or_else(|| ApiError::not_implemented(WAYPOINT_BACKEND_HINT))?;
let thread = parse_thread_id(&id)?;
let wp = waypoints.latest(&thread).await
    .map_err(|e| ApiError::internal(e.to_string()))?          // leaks backend text - replace
    .ok_or_else(|| ApiError::not_found(format!("unknown thread '{id}'")))?;
```
Replace with `load_visible_thread(&state, &principal, &thread)` after `parse_thread_id` and the 501 check: latest waypoint (missing -> 404) -> `All` returns early (no run lookup) -> `state.runs == None` -> 404 -> `runs.list(RunQuery{thread_id:Some(..), limit:1, ..Default::default()})` -> `scope.permits_thread(items.first())` else the identical `unknown thread '{id}'` 404.
- Handlers currently ignoring the principal use `Extension(_principal)` (get_thread l.945) - rename and use it.
- `resume_thread` (l.704) and `delete_thread` (l.1072): `delete_thread` calls `require_admin(&principal)?` FIRST (l.1078); move the gate before it (D-06).
- Seven `ApiError::internal(e.to_string())` sites (lines ~620, 667, 805, 890, 956, 1088, 1101) -> thread-side fixed-message helper, detail to `log::error!` (research Pitfall 7).
- `get_thread_history` (l.~805) currently returns `200 []` for an unknown thread; gate makes it 404 - add `404` to its utoipa responses.
- List walk: keep the `encode_cursor`/`decode_cursor::<DateTime<Utc>>` already in `list_threads`; Admin keeps the single-fetch path; add the examination cap (research Pattern 3).

### Thread route matrix tests (test)

**Analog:** `run_controller.rs` l.3167-3290 (`published_run_id_operations`, `send_as`, `every_run_id_route_hides_foreign_runs_behind_the_missing_run_404`). Copy verbatim, swapping `/v1/runs/{run_id}` for `/v1/threads/{id}` and `versioned_run_parts` for `versioned_thread_parts(..).1`:
```rust
fn published_thread_id_operations(api: &utoipa::openapi::OpenApi) -> Vec<(String, String)> { /* same 8-method table */ }
let mut api_keys = HashMap::new();
api_keys.insert("owner-key".into(), Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()));
api_keys.insert("foreign-key".into(), Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()));
api_keys.insert("admin-key".into(), Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()));
let auth = crate::agent_auth::AgentAuthConfig { enabled: true, api_keys, token_verifier: None, bearer_tenant: None };
```
Table assertion: `assert_eq!(published, tabled)` so an ungated new route fails. Existing thread test doubles (`MockRunRepository.list` ignores query; `MockWaypointStore.list_threads` returns `vec![]`) are inadequate - new doubles must honor `thread_id`, `limit`/`before`, and inject `Serialization` (copy the `fail_reads_with` seam from run_controller's `MockRepository`). Use non-substring ids (`acme-thread-1` vs `no-such-9f3a`); compare status, code, headers and body. Re-seed `post_resume_with_non_admin_role_is_403`, `get_thread_history_paginates_with_limit_and_cursor`, `delete_thread_non_admin_is_403`.

### `job_store.rs` (store, in-memory CRUD)

**Analog:** itself (l.62-125). Extend `struct Inner { jobs, order }` with `owners: HashMap<String,(String, TenantId)>`; evict in the same `while inner.order.len() > self.capacity` loop as `jobs.remove`. Keep poison handling `unwrap_or_else(|e| e.into_inner())`. Add `create_owned(&self, agent_id:&str, principal:&Principal) -> String` (reuse `create` body) and `get_visible` (research code example: `All` -> record; `Tenant(t)` -> owner exactly `(agent_id, t)`; legacy `create()` jobs are All-only). Leave `JobRecord`, `create`, `get` unchanged.

### `agent_controller.rs` (controller)

**Analog:** `enqueue_job` l.835-853 already has `Extension(principal): Extension<Principal>`; change `state.jobs.create()` (l.853) to `create_owned(&id, &principal)`. `get_job` (l.906) today:
```rust
pub async fn get_job(State(state): State<AgentApiState>, Path((_id, job_id)): Path<(String, String)>) -> Result<(StatusCode, JsonValue), ApiError> {
    match state.jobs.get(&job_id) { Some(record) => Ok((StatusCode::OK, ok_body(&record))), None => Err(ApiError::not_found(format!("unknown job '{job_id}'"))) }
}
```
WARNING (research Pitfall 1, Open Question 1): adding `Extension<Principal>` to `pub fn get_job` is a semver-major `function_parameter_count_changed`. Planner must resolve first: recommended additive `get_job_scoped` handler (new `#[utoipa::path]`, routed in `agent_openapi_router`) + `#[deprecated]` on `get_job`; else record a break and go to `v0.12.0`. Existing `poll_job`/`get_job_unknown_returns_404` tests call `get_job` directly and must move.

### `worker.rs` + `run_api_wiring.rs` (wiring, boot-time)

**Analog:** `run_api_wiring.rs` l.850-890: `.with_engine_factory(engine_factory)` in the builder chain, `if let Some(treasurer) = &treasurer { pool = pool.with_treasurer(Arc::clone(treasurer)); }`, `let pool = Arc::new(pool);` then `Arc::clone(&pool).spawn(...)`. Insert `assert_treasurer_enforced(treasurer.is_some(), pool.has_engine_factory())?;` between `with_treasurer` block and `Arc::new(pool)`/`spawn`. Errors in this function are `String` (`.map_err(|e| format!("invalid treasurer configuration: {e}"))?`, l.815). Add `RunWorkerPool::has_engine_factory(&self) -> bool` (and optionally `has_treasurer`) following the existing inherent builder style; needs `make api-surface-update` + root CHANGELOG entry. Tests: pure function table, refusal using a real pool built without `with_engine_factory`, and reuse `build_run_api_wires_a_treasurer_only_when_allowances_are_configured` (l.1496) for the shipped-wiring pass.

Schedule scan: `schedule_repository: Arc<dyn RunScheduleRepositoryPort>` is built at l.313 / l.348 and imported at l.53. Add `scan_unattributed_schedules(&dyn RunScheduleRepositoryPort) -> ScanOutcome` + `report_scan` (one `log::warn!` with count, ids ascending, "re-create via POST /v1/schedules"; page error -> one `log::error!` with cursor position, fixed text, never `{err}`). Only `list(100, cursor)` calls. Capture logs using the existing `CapturingLogger` pattern (`src/bin/paladin-server.rs` tests, `crates/paladin-storage/src/cadence/test_logger.rs`), match by unique marker not global count.

### `src/config/agents.rs::AuthConfig::validate` (config, validation)

**Analog:** l.193-235 (existing message shape, name never the key secret):
```rust
TenantId::new(entry.name.as_str()).map_err(|e| format!("http.auth.api_keys[#{index}]: 'name' is not a valid API key id: {e}"))?;
let name = entry.name.as_str();
...
TenantId::new(entry.tenant.as_str()).map_err(|e| format!("http.auth.api_keys[{name}]: 'tenant' is not a valid tenant id: {e}"))?;
...
Some(tenant) => { TenantId::new(tenant).map_err(|e| format!("http.auth.bearer_token: 'tenant' is not a valid tenant id: {e}"))?; }
```
Insert reserved-literal checks immediately after each `TenantId::new` check (name -> `RESERVED_PRINCIPAL_LITERALS` + `"anonymous"`; tenant and `bearer_token.tenant` -> `RESERVED_PRINCIPAL_LITERALS`), exact case-sensitive, message `http.auth.api_keys[{name}]: 'tenant' is the reserved literal '{x}' ...`. Table tests next to `validate_ignores_the_bearer_tenant_when_bearer_is_disabled` (l.767); that test must stay green (bearer check stays inside the enabled branch). Add a root-crate test asserting `paladin_web::agent_auth::OPEN_ACCESS_PRINCIPAL_ID == "open-access"` to prevent drift.

## Shared Patterns

### Hidden equals missing
**Source:** `run_controller.rs` `load_visible_run` + matrix test body-swap comparison. **Apply to:** `load_visible_thread`, `get_job_scoped`, thread matrix. Same status, code, headers, body after swapping ids; never `403` for existence.

### Server-derived scope only
**Source:** `Principal::read_scope()` / `RunReadScope::for_principal`. **Apply to:** every new gate. Take `RunReadScope`; never read tenant from header/query/body (AC-N3).

### Redacted repository errors
**Source:** `internal_repo_error` (log detail, return constant). **Apply to:** all thread-controller error sites and the schedule scan. New constant for thread store (research Open Question 4).

### Public-surface bookkeeping
`make api-surface-update`, per-crate CHANGELOG under an unreleased heading (never inside byte-frozen `[0.11.0]`), `MIGRATION.md` §11.1/§11.6 behavioral entries (not §11.2), allowlist stays zero entries, ADR 0063 + PROMOTION advance to 0064.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| Thread list bounded-walk (`list_threads`) | handler | No existing per-item-authorized paging loop; use CONTEXT D-04 + research Pattern 3 (cap 500, batch 100). |
| `53-VERSION-DECISION.md` | evidence | Needs the CI `Semver Checks (vs v0.11.0)` line from the PR head; cannot be produced locally (disk full). Needs a checkpoint. |

## Metadata

**Analog search scope:** `crates/paladin-core`, `crates/paladin-web`, `src/application/services/run`, `src/infrastructure/web`, `src/config`
**Files scanned:** ~9 targeted reads/greps
**Pattern extraction date:** 2026-10-31
**Caveats from research to carry into plans:** `Run` orders by `submitted_at` (not `created_at`); root-crate tests can't compile locally while the disk is full; the 53-RESEARCH.md Validation Architecture section (lines 496-588) was not read by this mapper.
