---
phase: 40-tenant-identity-run-read-scoping
plan: 02
subsystem: auth
tags: [tenant-scoping, run-repository, sqlite, postgres, in-memory, keyset-pagination, contract-tests, axum]

# Dependency graph
requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: "40-01: TenantId, RunAttribution, RunReadScope, Run.submitted_by, migration 008 (both dialects), SQLite attribution columns, Principal::read_scope, load_visible_run"
  - phase: 27-platform-api-run-submission
    provides: RunRepositoryPort, RunQuery keyset list, SqliteRunRepository, PostgresRunRepository, InMemoryRunRepository, run_router
provides:
  - "RunQuery.scope: RunReadScope (Default = All) -- the list half of the shared D-12 read rule"
  - "Tenant predicate INSIDE each adapter's own list query (SQLite/Postgres WHERE tenant_id = <bound>; in-memory RunReadScope::permits before sort/page)"
  - "GET /v1/runs scoped from Principal::read_scope() -- user key sees its tenant, Admin sees all, ?tenant_id changes nothing"
  - "Six shared contract clauses (attribution round trip, null attribution, immutability under every mutating method, scoped list, empty scoped page, scoped keyset pagination) wired into all three adapters"
  - "PostgreSQL attribution parity: tenant_id/api_key_id on every INSERT/SELECT constant, row_to_run three-way mapping, private list_query builder"
  - "tenant_scoped_run_list_e2e -- the list half of PLAT-07 through the real run_router over on-disk SQLite"
affects: [40-04, 40-05, 40-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Read scope is a query-time predicate, never a Rust post-filter: RunQuery.scope is pushed into the same QueryBuilder chain as the other filters, ahead of the cursor clause and ORDER BY ... LIMIT, so limit/next_cursor stay a correct keyset walk"
    - "Pure query-assembly function (postgres::list_query) returning a QueryBuilder so the SQL text and placeholder positions are unit-testable without a database"
    - "Contract clauses that must share one Postgres database scope themselves with uuid-suffixed tenant/thread/assistant ids"
    - "Handler test double records every RunQuery it receives so tests assert what the handler asked for, not just what came back"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/run_repository_port.rs
    - crates/paladin-storage/src/run/contract_tests.rs
    - crates/paladin-storage/src/run/sqlite.rs
    - crates/paladin-storage/src/run/in_memory.rs
    - crates/paladin-storage/src/run/postgres.rs
    - crates/paladin-web/src/run_controller.rs
    - src/application/services/run/http_surface_tests.rs

key-decisions:
  - "RunQuery.scope defaults to RunReadScope::All so every internal caller (fork's latest-run lookup, run inspector, schedule service -- all use ..Default::default()) stays unscoped; list_runs is the only caller that narrows it (D-12 / Pitfall 8)"
  - "The Postgres RED commit intentionally does not compile under --features postgres (list_query did not exist yet); the feature is non-default so the workspace build is unaffected, and the GREEN commit follows immediately"
  - "40-01's sqlite-local insert_then_get_round_trips_attribution_on_sqlite removed as byte-equivalent to the new shared clause; the other sqlite-local attribution tests (migration shape, SQL NULL, half-attributed row, insert_with_latest) kept"
  - "postgres.rs's stale 'no extra #[tokio::test]s beyond exactly one' comment rewritten to describe what CI actually asserts (passed >= declared, no SKIP lines) and to name the two new postgres-local clauses"

patterns-established:
  - "Pattern: scope predicate ordering is asserted by a DB-free SQL-text test (list_query_applies_the_tenant_scope_before_order_by) and by a live keyset walk (list_scoped_pagination_has_no_gap_or_overlap) -- the first pins the mechanism, the second pins the observable guarantee"

requirements-completed: [TENANT-02, PLAT-07]

coverage:
  - id: D1
    description: "GET /v1/runs returns only the calling principal's tenant's runs (across two API keys of the same tenant), every run for an Admin, an exact `{\"items\":[],\"next_cursor\":null}` for a tenant with no runs, an empty page for another tenant's thread_id, and a gap-free ?limit=1 keyset walk, with ?tenant_id ignored -- through the real run_router over on-disk SQLite"
    requirement: "PLAT-07"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#tenant_scoped_run_list_e2e"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#list_runs_passes_the_callers_read_scope"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#list_runs_ignores_a_tenant_query_parameter"
        status: pass
    human_judgment: false
  - id: D2
    description: "The tenant filter is a predicate inside each adapter's own query (SQLite and Postgres: WHERE tenant_id = <bound> before ORDER BY submitted_at DESC, run_id DESC LIMIT; in-memory: RunReadScope::permits before sort/page) so a scoped keyset walk at limit 2 over interleaved tenants yields full non-final pages, no overlap, no gap"
    requirement: "PLAT-07"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#list_scoped_pagination_has_no_gap_or_overlap"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/in_memory.rs#list_scoped_pagination_has_no_gap_or_overlap"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/postgres.rs#list_query_applies_the_tenant_scope_before_order_by"
        status: pass
      - kind: integration
        ref: "crates/paladin-storage/src/run/postgres.rs#list_scoped_pagination_has_no_gap_or_overlap (CI postgres-integration job; self-skipped locally, no Docker)"
        status: unknown
    human_judgment: false
  - id: D3
    description: "Scoped list semantics on every adapter: Tenant(A) sees A's runs from two keys and never B's or an unattributed run; All sees everything; Tenant(A) + B's thread_id is an empty page (D-14); a tenant with no runs gets items == [] and next_cursor == None"
    requirement: "PLAT-07"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#list_scoped_to_tenant_returns_only_that_tenants_runs"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#list_scoped_to_a_tenant_with_no_runs_is_an_empty_page"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/in_memory.rs#list_scoped_to_tenant_returns_only_that_tenants_runs"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/in_memory.rs#list_scoped_to_a_tenant_with_no_runs_is_an_empty_page"
        status: pass
    human_judgment: false
  - id: D4
    description: "TENANT-02 attribution guarantees on every adapter: submitted_by round-trips exactly; an unattributed run reads back None and is never listed under a Tenant scope; update_status, bump_attempt, request_cancel, record_resume, clear_pending_responses and record_outcome leave submitted_by byte-for-byte unchanged"
    requirement: "TENANT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#insert_then_get_round_trips_attribution"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#unattributed_run_round_trips_null_attribution"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#attribution_survives_every_status_and_attempt_update"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/in_memory.rs#attribution_survives_every_status_and_attempt_update"
        status: pass
    human_judgment: false
  - id: D5
    description: "PostgreSQL adapter writes tenant_id/api_key_id in the same single INSERT as the rest of the row on both insert paths (INSERT_RUN $19,$20; INSERT_RUN_WITH_LATEST $18,$19 with the assistant predicate at $20), reads them on every SELECT, and maps both-present/both-NULL/half-set exactly as SQLite does"
    requirement: "TENANT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/postgres.rs#insert_constants_carry_the_attribution_columns"
        status: pass
      - kind: integration
        ref: "crates/paladin-storage/src/run/postgres.rs#insert_with_latest_persists_attribution, unattributed_run_is_stored_as_sql_null and the six contract clauses (CI postgres-integration job; self-skipped locally, no Docker)"
        status: unknown
    human_judgment: false
  - id: D6
    description: "A forked run is attributed to the forking principal, and the fork's own latest-run lookup is unscoped (finds the unattributed original)"
    requirement: "TENANT-02"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#fork_run_completes_from_waypoint"
        status: pass
    human_judgment: false

# Metrics
duration: ~1h
completed: 2026-09-28
status: complete
---

# Phase 40 Plan 02: Tenant-Scoped Run List Summary

**`GET /v1/runs` now returns only the caller's tenant's runs (every run for an Admin), with the tenant predicate living inside each adapter's own keyset query rather than a Rust post-filter, and the in-memory and PostgreSQL adapters brought to attribution parity with SQLite under one shared six-clause contract suite.**

## Performance

- **Duration:** ~1h
- **Completed:** 2026-09-28
- **Tasks:** 2 (both `tdd="true"`, each with a RED and a GREEN commit)
- **Files modified:** 7

## Accomplishments

- `RunQuery.scope: RunReadScope` (Default = `All`) on the run repository port, with module-level and `list` rustdoc stating the D-12 rule: a predicate inside the adapter's query, never a post-filter, so `limit`/`next_cursor` stay a correct keyset walk
- `SqliteRunRepository::list` and the new pure `postgres::list_query` push ` AND tenant_id = ` + `push_bind` for `Tenant(t)` after the `status` filter and before the cursor clause and `ORDER BY submitted_at DESC, run_id DESC LIMIT`; `InMemoryRunRepository::list` applies `query.scope.permits(run)` before sort and page
- `list_runs` derives `scope: principal.read_scope()` from the authenticated principal only; rustdoc and `#[utoipa::path]` left byte-unchanged (`openapi_matches_committed_baseline` still passes, 40-05 regenerates)
- Six shared contract clauses in `contract_tests.rs` (plus `sample_attributed_run`), each uuid-scoped, wired one-`#[tokio::test]`-per-clause into SQLite, in-memory and (Docker-gated) PostgreSQL test modules
- PostgreSQL: attribution columns on `INSERT_RUN`, `INSERT_RUN_WITH_LATEST`, `SELECT_RUN_BY_ID`, `SELECT_ACTIVE_RUN_FOR_THREAD`, `LIST_SELECT_PREFIX`; `row_to_run` three-way mapping identical to SQLite; two DB-free tests pin the SQL text and the scope predicate's position/bound form; two postgres-local live clauses mirror SQLite's
- `tenant_scoped_run_list_e2e`: real `run_router` over on-disk SQLite, five keys (two acme, one globex, one Admin, one empty tenant), asserting per-key listings, the exact empty body, D-14 cross-tenant `thread_id`, `?tenant_id` spoofing, and a `?limit=1` cursor walk; `fork_run_completes_from_waypoint` now proves D-08 fork attribution

## Task Commits

1. **Task 1 (RED)** — `4be609b7` test(40-02): add failing tenant-scoped run list tests and RunQuery.scope
2. **Task 1 (GREEN)** — `85fb7cab` feat(40-02): scope GET /v1/runs to the caller's tenant on SQLite and in-memory
3. **Task 2 (RED)** — `2ae68e53` test(40-02): add failing PostgreSQL attribution and scoped-list tests
4. **Task 2 (GREEN)** — `18c784ec` feat(40-02): bring the PostgreSQL run adapter to attribution parity and scope its list

## Files Created/Modified

- `crates/paladin-ports/src/output/run_repository_port.rs` — `RunQuery.scope`, `RunReadScope` import, D-12 module section and `list` rustdoc
- `crates/paladin-storage/src/run/contract_tests.rs` — `sample_attributed_run`, uuid helpers, six new `pub async fn` clauses
- `crates/paladin-storage/src/run/sqlite.rs` — scope `match` in `list`; six clause wirings; removed the byte-equivalent 40-01 local round-trip test
- `crates/paladin-storage/src/run/in_memory.rs` — `.filter(|r| query.scope.permits(r))`; six clause wirings
- `crates/paladin-storage/src/run/postgres.rs` — five SQL constants, `row_to_run`, both insert paths' binds, `list_query`, two DB-free tests, six clause wirings, two postgres-local tests, corrected module comment
- `crates/paladin-web/src/run_controller.rs` — `list_runs` scope; `MockRepository.recorded_queries`/`last_list_query`; two new tests
- `src/application/services/run/http_surface_tests.rs` — `tenant_scoped_run_list_e2e`; fork attribution assertion

## Decisions Made

- `RunQuery.scope` defaults to `All`; only `list_runs` narrows it. The two facade callers (`inspector.rs`, `submission.rs` fork lookup) already used `..Default::default()` and needed no edit (D-12 / Pitfall 8).
- Removed only the sqlite-local `insert_then_get_round_trips_attribution_on_sqlite` (byte-equivalent to the shared clause); kept `migration_008_...`, `sql_constants_name_the_attribution_columns`, `insert_with_latest_persists_attribution`, `unattributed_run_is_stored_as_sql_null`, `half_attributed_row_is_a_serialization_error`.
- Rewrote `postgres.rs`'s trailing "exactly one extra test" comment: CI's actual gate (`.github/workflows/ci.yml` postgres-integration) asserts passed >= declared `#[tokio::test]` count and fails on any `SKIP:` line — the two new postgres-local tokio tests satisfy it because they run for real in CI.

## Deviations from Plan

None — plan executed as written. No files owned by 40-03 or 40-04 were touched.

## Issues Encountered

- **PostgreSQL live clauses were compiled but NOT executed here.** There is no Docker daemon on this machine, so `store_or_skip` printed `SKIP:` and every live-DB test in `run::postgres` returned early (reported `ok` by the harness, 29 passed). Only `insert_constants_carry_the_attribution_columns` and `list_query_applies_the_tenant_scope_before_order_by` genuinely exercised code. The eight live clauses (six shared + `insert_with_latest_persists_attribution` + `unattributed_run_is_stored_as_sql_null`) will run in CI's `postgres-integration` job with `STORAGE_POSTGRES_TEST_URL` set; their `coverage` status above is recorded as `unknown`, not `pass`.
- Disk: 13 GB free at start, 9.1 GB at finish (the `--features postgres` test/clippy builds added ~3 GB to `target/`). No `cargo clean`; each full clippy ran exactly once per task.
- The Task 2 RED commit (`2ae68e53`) does not compile under `--features postgres` (missing `list_query`); the feature is non-default, so the default workspace build at that commit is unaffected, and `18c784ec` immediately restores it.

## Known Stubs

None — every code path is wired to real data; no placeholders, skipped tests or TODOs were introduced.

## Threat Flags

None beyond the plan's own `<threat_model>` (T-40-08..T-40-11, all mitigated as specified: scope from principal only; predicate inside the keyset query; `push_bind` only; no `UPDATE` touches the attribution columns).

## User Setup Required

None.

## Next Phase Readiness

- 40-04 (ledger scope) and 40-05 (`RunResponse.submitted_by`, `load_visible_run` on stream/cancel/webhook routes, openapi regeneration) have everything they need; `list_runs`'s rustdoc/utoipa attribute is untouched for 40-05 to rewrite once.
- 40-06 should register `RunQuery.scope` (an additive field on a `Default`-deriving struct with public fields — a struct-literal break for any downstream that built `RunQuery { .. }` without `..Default::default()`) in MIGRATION.md 9.2 and the semver allowlist alongside the other Phase 40 rows, and refresh the API-surface baseline.
- `make api-surface` will report drift until 40-06 refreshes the baseline (expected, 39-08 precedent).

---
*Phase: 40-tenant-identity-run-read-scoping*
*Completed: 2026-09-28*

## Self-Check: PASSED

All seven modified files exist on disk; all four task commit hashes (`4be609b7`, `85fb7cab`, `2ae68e53`, `18c784ec`) are present in `git log --oneline --all`.
