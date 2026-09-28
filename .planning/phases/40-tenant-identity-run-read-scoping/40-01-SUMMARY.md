---
phase: 40-tenant-identity-run-read-scoping
plan: 01
subsystem: auth
tags: [tenant-scoping, principal, sqlite, run-submission, axum, semver-register]

# Dependency graph
requires:
  - phase: 27-platform-api-run-submission
    provides: RunSubmissionPort, RunApiState, run_router, SqliteRunRepository
  - phase: 24-thread-agent-auth
    provides: AgentAuthConfig, Principal, require_authentication middleware
provides:
  - "TenantId/PrincipalRef/RunAttribution/RunReadScope (paladin-core, the one shared D-12 read rule)"
  - "Run.submitted_by + Run::with_submitted_by (additive, RUN_SCHEMA_VERSION stays v1)"
  - "SubmitRun/ForkRun/RunSubmissionPort::cancel retyped to Option<PrincipalRef>"
  - "runs.tenant_id / runs.api_key_id columns + idx_runs_tenant_submitted (SQLite; Postgres owned by 40-02)"
  - "Principal.tenant_id, Principal::new, Principal::read_scope, AgentAuthConfig.bearer_tenant"
  - "load_visible_run -- the single-run half of the D-12 read gate on GET /v1/runs/{id}"
  - "ApiKeyConfig.tenant / BearerTokenAuthConfig.tenant required at boot, fail-closed"
  - "MIGRATION.md 9.2/9.4/9.5 rows + mirrored semver allowlist entries for every break above"
affects: [40-02, 40-03, 40-04, 40-05, 40-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Newtype identifier validated on construction AND on serde deserialize (TenantId mirrors ThreadId/ waypoint.rs)"
    - "One shared authorization rule (RunReadScope) applied by exactly two mechanisms: RunQuery.scope (list, 40-02) and load_visible_run (single-run, this plan)"
    - "Hidden-vs-missing existence oracle closed by returning the byte-identical ApiError::not_found on both paths"
    - "register-only semver-allowlist entries for a field/parameter type change the tool's lint catalog cannot model"

key-files:
  created:
    - crates/paladin-core/src/platform/container/principal.rs
    - crates/paladin-storage/migrations/sqlite/008_add_run_attribution_columns.sql
    - crates/paladin-storage/migrations/postgres/008_add_run_attribution_columns.sql
  modified:
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-storage/src/run/sqlite.rs
    - crates/paladin-web/src/agent_auth.rs
    - crates/paladin-web/src/run_controller.rs
    - src/application/services/run/submission.rs
    - src/config/agents.rs
    - src/bin/paladin-server.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml

key-decisions:
  - "Task 1 checkpoint: option-a (Principal marked #[non_exhaustive] plus Principal::new) -- auto-selected under --auto mode, gate=\"blocking\""
  - "Postgres adapter deliberately left untouched this plan -- it compiles unchanged by ignoring the new nullable columns; 40-02 owns its attribution wiring and contract-suite coverage"
  - "api_key_auth_with_tenant test helper (run_controller.rs) fixed to take an explicit principal_id parameter -- it previously hardcoded \"svc\" regardless of caller, which desynced from a test asserting api_key_id == \"svc-a\" (Rule 1 bug fix)"

patterns-established:
  - "Pattern: RunReadScope::for_principal(role, tenant) -> All | Tenant(t); permits() is the one predicate every read path calls"
  - "Pattern: PrincipalRef::attribution() drops the role before it ever reaches a persisted RunAttribution -- roles are config, not data (D-08)"

requirements-completed: [TENANT-01, TENANT-02, PLAT-07]

coverage:
  - id: D1
    description: "An API key's configured tenant travels config -> Principal.tenant_id -> PrincipalRef -> Run.submitted_by -> the 008 SQLite columns -> GET /v1/runs/{id}'s read gate (owner 200, Admin 200, other-tenant 404 identical to missing-run 404), through the real run_router over an on-disk SqliteRunRepository, with header/query/body tenant spoofing proven ineffective"
    requirement: "TENANT-01"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#tenant_scoped_run_read_tracer"
        status: pass
    human_judgment: false
  - id: D2
    description: "runs.tenant_id/runs.api_key_id are nullable TEXT, non-unique idx_runs_tenant_submitted exists, SqliteRunRepository writes both columns inside the same INSERT as the rest of the row, reads them back, stores SQL NULL for an unattributed run, and rejects a half-attributed row as RunRepositoryError::Serialization"
    requirement: "TENANT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#migration_008_adds_nullable_attribution_columns_and_the_scoped_index"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#insert_then_get_round_trips_attribution_on_sqlite"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#insert_with_latest_persists_attribution"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#unattributed_run_is_stored_as_sql_null"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#half_attributed_row_is_a_serialization_error"
        status: pass
    human_judgment: false
  - id: D3
    description: "A hidden (out-of-tenant) run and a genuinely missing run answer the byte-identical 404 -- no 403, no shape/timing difference"
    requirement: "PLAT-07"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#get_run_hides_a_foreign_tenant_run_behind_the_missing_run_404"
        status: pass
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#tenant_scoped_run_read_tracer"
        status: pass
    human_judgment: false
  - id: D4
    description: "paladin-server refuses to boot when an api_keys entry has an empty/invalid tenant, or bearer_token.enabled is true with no bearer_token.tenant -- error names the key's name, never its secret value"
    requirement: "TENANT-01"
    verification:
      - kind: unit
        ref: "src/bin/paladin-server.rs#build_auth_config_fails_closed_when_an_api_key_has_no_tenant"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#build_auth_config_requires_a_bearer_tenant_when_bearer_is_enabled"
        status: pass
    human_judgment: false
  - id: D5
    description: "MIGRATION.md 9.2 carries Y rows for every deliberate break this plan lands, mirrored by semver-checks-allowlist.toml entries; 9.4/9.5 record the migration and the new required config keys; check-migration-allowlist.sh confirms set-equality"
    requirement: "TENANT-01"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~2h (continuation session; original session interrupted by ENOSPC mid-Task-2)
completed: 2026-09-28
status: complete
---

# Phase 40 Plan 01: Tenant Identity Tracer Summary

**An API key's configured tenant now travels config -> `Principal.tenant_id` -> `PrincipalRef` -> `Run.submitted_by` -> SQLite's new `008` columns -> `GET /v1/runs/{id}`'s read gate, proven end-to-end by a real-router tracer test, with every landed break registered in MIGRATION.md and the semver allowlist.**

## Performance

- **Duration:** ~2h for this continuation session (a prior session was interrupted by disk exhaustion partway through Task 2; this session picked up from its documented handoff state, verified the untested edits compiled and passed, finished the remaining Task 2 scope, then completed Task 3)
- **Completed:** 2026-09-28
- **Tasks:** 3 (Task 1 checkpoint: auto-approved; Task 2: tracer; Task 3: register)
- **Files modified:** 27 (Task 2 commit) + 2 (Task 3 commit)

## Checkpoint decision

**Task 1 (`checkpoint:decision`, `gate="blocking"`):** option-a — "Approve as proposed, with Principal marked `#[non_exhaustive]` plus `Principal::new`" — was selected.

⚡ Auto-selected in --auto mode (gate="blocking")

This means: `Principal { pub id, pub role, pub tenant_id: TenantId }` is `#[non_exhaustive]` with `Principal::new(id, role, tenant_id)` as its one constructor (X-10.3 option (a), the `LlmRequest`/`GarrisonEntry` precedent — the next field this type gains stays non-breaking); `PrincipalRef { api_key_id, tenant_id, role }` replaces the `(String, UserRole)` tuple on `SubmitRun`/`ForkRun`/`RunSubmissionPort::cancel`; `008_add_run_attribution_columns.sql` adds nullable `tenant_id`/`api_key_id` TEXT columns plus the non-unique `idx_runs_tenant_submitted` index to both SQLite and Postgres dialects, with `RUN_SCHEMA_VERSION` staying `v1`.

## Accomplishments

- `TenantId`/`TenantIdError`/`PrincipalRef`/`RunAttribution`/`RunReadScope` — the one shared D-12 read-scope rule, in a new `paladin-core` module, 9 unit tests + 2 doctests
- `Run.submitted_by: Option<RunAttribution>` — additive, `#[serde(default, skip_serializing_if)]`, `RUN_SCHEMA_VERSION` unchanged (X-04)
- `SqliteRunRepository` writes/reads the new `runs.tenant_id`/`runs.api_key_id` columns in the same `INSERT` as the rest of the row, rejects half-attributed rows as `Serialization`; Postgres adapter deliberately untouched (40-02 owns it)
- `Principal.tenant_id` (required, server-derived, D-01/D-02), `Principal::read_scope()`, `AgentAuthConfig.bearer_tenant` (fail closed when a verified bearer token has no configured tenant, D-03)
- `run_controller::load_visible_run` — the single-run half of the shared read gate: a foreign tenant's run and a genuinely missing run answer the byte-identical `404`, never `403` (PLAT-07)
- `RunSubmissionService::submit`/`fork` stamp the caller's `RunAttribution` onto the run before insert; `authorize_invocation` now reads the role off `PrincipalRef`
- `ApiKeyConfig.tenant`/`BearerTokenAuthConfig.tenant` required at boot; `build_auth_config` fails closed naming the key's `name`, never its secret value
- `tenant_scoped_run_read_tracer` — the phase's end-to-end proof, driven through the real `run_router` over an on-disk `SqliteRunRepository`, spoofing tenant via header/query/body and proving none of them influence attribution
- MIGRATION.md 9.2 (8 rows), 9.4 (1 bullet), 9.5 (1 bullet) plus 8 mirrored `.cargo/semver-checks-allowlist.toml` entries; `./scripts/check-migration-allowlist.sh` exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Confirm the three one-way doors** — checkpoint, auto-approved, no commit (decision recorded above; code lands in Task 2's commit)
2. **Task 2: Tracer** — `79201d90` (feat)
3. **Task 3: Register the tracer's breaks** — `66416d01` (docs)

_Note: this was a continuation run — a prior session's uncommitted Task 2 edits (principal.rs, run.rs, run_submission_port.rs, migration 008, sqlite.rs, agent_auth.rs, run_controller.rs and the other controllers) were verified against the plan, completed (the tracer test, submission.rs's attribution stamping, schedule fallout, config/agents.rs, paladin-server.rs's build_auth_config and its new tests, the operator config files, and every example/test compile-fallout site), then committed as a single Task 2 commit rather than split further, since the prior session's edits and this session's completions form one coherent tracer._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/principal.rs` — new: `TenantId`, `TenantIdError`, `PrincipalRef`, `RunAttribution`, `RunReadScope`
- `crates/paladin-core/src/platform/container/mod.rs` — registers `pub mod principal;`
- `crates/paladin-core/src/platform/container/run.rs` — `Run.submitted_by`, `Run::with_submitted_by`, X-04 doc note
- `crates/paladin-ports/src/input/run_submission_port.rs` — `requested_by` retyped to `Option<PrincipalRef>` everywhere
- `crates/paladin-storage/migrations/{sqlite,postgres}/008_add_run_attribution_columns.sql` — new
- `crates/paladin-storage/src/run/sqlite.rs` — SQL constants, `row_to_run`, both insert paths, 6 new tests
- `crates/paladin-web/src/agent_auth.rs` — `Principal.tenant_id`, `Principal::new`, `read_scope()`, `AgentAuthConfig.bearer_tenant`, `From<&Principal> for PrincipalRef`
- `crates/paladin-web/src/run_controller.rs` — `load_visible_run`, `get_run`/`submit_run`/`cancel_run` wiring, new tests, fixed `api_key_auth_with_tenant` test helper
- `crates/paladin-web/src/{thread,agent,assistant,schedule}_controller.rs` — compile fallout (test literals, `fork_thread`, mock `cancel` signatures)
- `src/application/services/run/submission.rs` — `authorize_invocation`, `submit`/`fork` attribution stamping, `cancel` retype, 4 new tests
- `src/application/services/run/schedule/{service.rs,tests.rs}` — doc-example/mock signature updates
- `src/application/services/run/http_surface_tests.rs` — `tenant_scoped_run_read_tracer`
- `src/config/agents.rs` — `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, 2 new tests
- `src/bin/paladin-server.rs` — `build_auth_config` tenant parsing + fail-closed messages, 3 new tests
- `config.example.yml`, `k8s/server/configmap.yaml`, `scripts/sdk-smoke/smoke-config.yml` — `tenant:` lines
- `examples/http_service_host.rs`, `examples/platform_api_client.rs`, `tests/web_server_e2e.rs`, `tests/integration/e2e_platform_api_test.rs`, `tests/paladin_server_smoke.rs` — `Principal`/`AgentAuthConfig` literal fallout
- `MIGRATION.md` — 9.2 (8 rows), 9.4 (1 bullet), 9.5 (1 bullet)
- `.cargo/semver-checks-allowlist.toml` — 8 mirrored entries + register-only convention header

## Decisions Made

- Task 1 checkpoint: option-a, auto-selected (see "Checkpoint decision" above)
- Postgres adapter left untouched this plan — it compiles unchanged by ignoring the new nullable columns; 40-02 owns its attribution wiring and contract-suite coverage (per the plan's own explicit scope note)
- `api_key_auth_with_tenant`'s hardcoded `"svc"` principal id was a pre-existing test bug (desynced from `submit_run_forwards_the_callers_principal_ref`'s `api_key_id == "svc-a"` assertion) — fixed by adding an explicit `principal_id` parameter (Rule 1)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `api_key_auth_with_tenant` test helper always assigned the principal id `"svc"` regardless of caller**
- **Found during:** Task 2 verification (`cargo test -p paladin-web --lib`)
- **Issue:** `submit_run_forwards_the_callers_principal_ref` asserted `requested_by.api_key_id == "svc-a"`, but the shared test helper hardcoded `"svc"`, causing the new test to fail
- **Fix:** Added an explicit `principal_id: &str` parameter to `api_key_auth_with_tenant`; `api_key_auth` (its thin wrapper) passes `"svc"` to preserve every existing caller's behavior unchanged
- **Files modified:** `crates/paladin-web/src/run_controller.rs`
- **Verification:** `cargo test -p paladin-web --lib` — 243 passed, 0 failed
- **Committed in:** `79201d90` (Task 2 commit)

**2. [Rule 3 - Blocking] `Result::expect_err`/`unwrap_err` require `T: Debug`, which `AgentAuthConfig` does not derive**
- **Found during:** Task 2 verification (`cargo test -p paladin-ai --features web-server --bin paladin-server`)
- **Issue:** The two new fail-closed tests (`build_auth_config_fails_closed_when_an_api_key_has_no_tenant`, `build_auth_config_requires_a_bearer_tenant_when_bearer_is_enabled`) used `.expect_err(...)` on `Result<AgentAuthConfig, _>`, which does not compile because `AgentAuthConfig` has no `Debug` impl
- **Fix:** Replaced with an explicit `match` that panics on `Ok` and extracts the error string on `Err`, avoiding the `Debug` bound entirely
- **Files modified:** `src/bin/paladin-server.rs`
- **Verification:** `cargo test -p paladin-ai --features web-server --bin paladin-server build_auth_config` — 5 passed, 0 failed
- **Committed in:** `79201d90` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking compile issue)
**Impact on plan:** Both fixes were necessary to reach a green build; neither changed the plan's intended behavior or scope.

## Issues Encountered

- The prior session died from disk exhaustion (`ENOSPC`) mid-Task-2, after most of its edits were made but before it could run `cargo test`/`git commit`/write `SUMMARY.md`. Disk was freed before this session started (25 GB available, `target/` deleted). The first `cargo build` therefore recompiled the whole dependency graph from scratch (~5-6 min per full-workspace invocation); intermediate incremental builds were fast. No further disk issues occurred.
- Task 3's acceptance-criteria check `awk '/^## 9\.2 /.../ grep -cE ...' prints 8` actually prints **9**, not 8: a legitimate pre-existing MIGRATION.md row (`paladin-ports | RunSubmissionPort`, marked `N/A`, recording a *different*, Phase-27-landed change — the `cancel`/`fork` signature shipped in that phase) already matches the same crate/type regex. This is **not** a bug to fix — deleting that row would destroy real historical content — it is an off-by-one in the plan's own acceptance-criteria count, which did not anticipate a pre-existing row for the same type. Every substantive Task 3 check passes: `./scripts/check-migration-allowlist.sh` confirms full set-equality (both directions), the `TBD`-free grep passes, and `grep -c "register-only"` reports 5 (the header-comment mention, the three new register-only `lint` field values, and one `justification` field that also names "register-only" — all accounted for by direct inspection, not a bare count check the plan gates on).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- 40-02 (RunQuery.scope, list filter on all three adapters, Postgres attribution columns, shared contract clauses) has everything it needs: `RunReadScope`/`TenantId`/`RunAttribution` are defined and tested, the SQLite half of the `008` migration is proven, and the Postgres adapter is confirmed to compile unchanged in the interim.
- 40-03 (duplicate name/key rejection, `AuthConfig::validate()`), 40-04 (ledger scope source), 40-05 (`load_visible_run` on stream/cancel/webhook-deliveries, route matrix, `RunResponse.submitted_by`), and 40-06 (ADR-0054, CHANGELOG, WINDOWS.md, remaining register rows) are all unblocked — none of their prerequisites were deferred by this plan.
- `make api-surface` will report public-surface drift until 40-06 refreshes the baseline with a CHANGELOG entry (expected, per the plan's own `<verification>` note — the 39-08 precedent).
- No blockers identified.

---
*Phase: 40-tenant-identity-run-read-scoping*
*Completed: 2026-09-28*

## Self-Check: PASSED

All created files exist on disk (`principal.rs`, both `008` migrations, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, this SUMMARY.md); both commit hashes (`79201d90`, `66416d01`) are present in `git log --oneline --all`.
