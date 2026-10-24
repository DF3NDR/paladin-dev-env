---
phase: 40-tenant-identity-run-read-scoping
reviewed: 2026-09-29T12:40:00Z
depth: standard
files_reviewed: 35
files_reviewed_list:
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/principal.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/run_scope.rs
  - crates/paladin-core/src/platform/container/treasury_ledger.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/output/paladin_executor_port.rs
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-ports/src/output/streaming_executor_port.rs
  - crates/paladin-storage/src/run/contract_tests.rs
  - crates/paladin-storage/src/run/in_memory.rs
  - crates/paladin-storage/src/run/postgres.rs
  - crates/paladin-storage/src/run/sqlite.rs
  - crates/paladin-web/src/agent_auth.rs
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/run_controller.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/worker.rs
  - src/bin/paladin-server.rs
  - src/config/agents.rs
  - src/core/platform/mod.rs
  - src/infrastructure/web/agent_host.rs
  - crates/paladin-storage/migrations/postgres/008_add_run_attribution_columns.sql
  - crates/paladin-storage/migrations/sqlite/008_add_run_attribution_columns.sql
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/openapi.json
  - docs/src/api-reference/platform-api.md
  - docs/src/deployment-topologies/http-service-host.md
findings:
  critical: 0
  warning: 4
  info: 5
  total: 9
status: issues_found
---

# Phase 40: Code Review Report

**Depth:** standard. **Status:** issues_found. **Diff base:** `629ef660..HEAD`.

## Summary

The core read-scoping design holds up under adversarial tracing.

- **Handlers:** every `/v1/runs/{run_id}*` handler (`get_run`, `cancel_run`, `list_webhook_deliveries`, `stream_run`) calls `load_visible_run` before touching anything else. Hidden and missing runs return an identical `ApiError::not_found` with the same message.
- **List scope:** the SQL scope predicate in `sqlite.rs` and `postgres.rs` is a bound `AND tenant_id = ?` placed before the cursor clause and `ORDER BY ... LIMIT`. It is not a post-filter, so the keyset walk stays correct. The in-memory adapter filters before sorting and paging.
- **NULL rows:** `NULL tenant_id` is never matched by a `Tenant` scope in either SQL or `permits`.
- **Half-attributed rows:** they fail closed on read in both adapters.
- **Secrets:** no secret reaches logs, errors or Debug output in the changed code. `validate` names entries by name or index and never prints a key value, and the `TenantIdError` messages do not echo the offending value.
- **Panics:** no `unwrap`/`expect`/`panic` was found in non-test library code.

No BLOCKER was found. The most important residual problem is a cross-tenant mutation path through a `thread_id` that neither the scope work nor the documented thread-route deferral covers (WR-01).

## Warnings

### WR-01: `POST /v1/runs {thread_id}` and `fork` let a user-role principal run against, and mutate, another tenant's thread

**File:** `src/application/services/run/submission.rs:262-285` (submit) and `:437-466` (fork). Supporting code in `src/application/services/run/worker.rs:279-300` (`WorkerDispatch::decide`) and `crates/paladin-web/src/run_controller.rs:744-770`.
**Issue:**
- `submit` accepts a caller-supplied `thread_id` with no tenant check. The only guard is the "thread busy" check.
- The worker then sees a latest Waypoint on that thread and dispatches `Resume`/`ResumeWith`. A tenant-B `User` can therefore advance and write Waypoints onto tenant A's thread.
- The new run is attributed to B, so B can read it back via `GET /runs/{id}`.
- `fork` has the same shape. It reads A's Waypoint and copies A's assistant reference through `RunQuery::default()`, which is `scope: All`.
- The 40-CONTEXT deferral (D-14) covers only the `/threads/{id}/...` routes. `POST /v1/runs` is a run route, and the "threads carry no tenant" rationale does not apply: each thread's runs already carry `submitted_by`.
- This is a cross-tenant mutation (`POST /v1/runs` and `POST /threads/{id}/fork`), which PLAT-07's intent is to prevent.

**Fix:** in `RunSubmissionService::submit` and `fork`, when `request.thread_id` is `Some` and `requested_by` is `Some`, look up the thread's latest run with `scope: All`. If it exists and `!RunReadScope::for_principal(role, &tenant).permits(&latest)`, return `RunSubmissionError::NotFound`/`UnknownThread`, the same value as for a nonexistent thread so existence is not leaked. A thread with no runs stays open. Add a contract-style test for both operations. If this is intentionally left to the deferred thread-tenancy work, record `POST /runs` with `thread_id` explicitly in the WINDOWS.md deferred row.

### WR-02: Tenant isolation for `cancel` is enforced only in the HTTP controller; the port carries the tenant but the service ignores it

**File:** `src/application/services/run/submission.rs:333-367`; `crates/paladin-ports/src/input/run_submission_port.rs` (`cancel`'s `requested_by: Option<PrincipalRef>`).
**Issue:**
- `RunSubmissionPort::cancel` now takes a `PrincipalRef` including `tenant_id` and `role`.
- `RunSubmissionService::cancel` loads the run but only feeds `role` to `authorize_invocation`. It never checks `run.submitted_by` against the principal.
- Isolation therefore depends on every caller doing what `paladin-web`'s `cancel_run` does (`load_visible_run` first). Any other adapter or embedder that calls `cancel(&id, Some(principal))` can cancel another tenant's run while appearing to pass a fully-attributed principal.
- The port doc implies the tenant "travels together" for a reason, but nothing consumes it.

**Fix:** inside `cancel`, when `requested_by` is `Some(p)`, apply `RunReadScope::for_principal(p.role, &p.tenant_id).permits(&run)`. If it does not permit, return `NotFound { run_id }`, the same as a missing run. The HTTP pre-check stays as the fast path.

### WR-03: A corrupt or half-attributed row turns into a 500 that leaks internals and breaks the "hidden == missing" guarantee

**File:** `crates/paladin-storage/src/run/sqlite.rs:294-313` and `postgres.rs:274-295` (`row_to_run`); `crates/paladin-web/src/run_controller.rs:849-865` (`load_visible_run`) and `:978-991` (`list_runs`).
**Issue:**
- `row_to_run` returns `RunRepositoryError::Serialization` with the message "run row attribution is incomplete: ..." or "invalid tenant_id: ...".
- `load_visible_run` maps it to `ApiError::internal(e.to_string())`. `ApiError::internal` echoes the message to the client (`error.rs:135`).
- A GET for another tenant's corrupt run returns 500 instead of the uniform 404. That is an existence oracle for that row.
- A single corrupt row inside a tenant's scope makes the whole `list` page fail, so `GET /runs` errors for that tenant.
- Nothing in the schema prevents half-attributed rows. The migration only adds two independent nullable columns.

**Fix:**
- Add a Postgres `CHECK ((tenant_id IS NULL) = (api_key_id IS NULL))` in migration 008. SQLite cannot add a cross-column CHECK via `ALTER TABLE ADD COLUMN`, so document that it relies on the read-time guard there.
- In `load_visible_run` and `list_runs`, log the repository error server-side and return a fixed message such as `ApiError::internal("run store error")`.

### WR-04: The new `Serialization` messages surface raw backend text to API clients (same handler path as WR-03)

**File:** `crates/paladin-web/src/run_controller.rs:856` and `:990`.
**Issue:** `ApiError::internal(e.to_string())` is a pre-existing pattern. Phase 40 adds new, more revealing error texts to it (attribution and tenant validation messages). `RunRepositoryError::Backend` text can also carry connection detail.
**Fix:** use one shared `internal_repo_error(e)` helper that logs the detail and returns a generic body, and use it for every repository error in this file.

## Info

### IN-01: Reserved literals are accepted as real tenant and key names

**File:** `src/config/agents.rs:170-235` (`AuthConfig::validate`); `crates/paladin-core/src/platform/container/principal.rs`; `crates/paladin-web/src/agent_auth.rs:75`.
**Issue:**
- `tenant: "open-access"` or `name: "anonymous"` validates fine and collides with the open-access principal `(open-access, anonymous)` in ledger rows.
- A key with tenant and name both `"unattributed"` makes `LedgerScope::is_unattributed()` true for a real principal. That conflicts with the doc's "never conflate the two" rule.

**Fix:** reject the reserved values (`open-access`, `unattributed`, and `anonymous` as a key name) in `validate` with a named-entry message.

### IN-02: Unreachable defensive branches in `build_auth_config`

**File:** `src/bin/paladin-server.rs:381-400` and `:410-419`.
**Issue:**
- `cfg.validate()?` already guarantees every tenant is valid. The re-derived, differently phrased error messages are dead code (the `k.tenant.is_empty()` branch and the bearer `map_err`).
- The duplicated message text can drift from `validate`.

**Fix:** map the error with a single generic "validated config rejected" message, or return the `validate` error type directly. Alternatively, have `validate` return a parsed structure that `build_auth_config` consumes.

### IN-03: `run.rs` and `principal.rs` depend on each other

**File:** `crates/paladin-core/src/platform/container/principal.rs:26` (`use ...run::Run`) and `run.rs:46` (`use ...principal::RunAttribution`).
**Issue:** `RunReadScope::permits(&Run)` makes a two-module cycle inside `paladin-core`. It compiles, but it couples the identity value types to the run aggregate.
**Fix:** take `Option<&RunAttribution>` in `permits` (for example `permits_attribution`) and keep `Run` out of `principal.rs`.

### IN-04: `GET /agents/{id}/jobs/{job_id}` ignores the principal and tenant

**File:** `crates/paladin-web/src/agent_controller.rs:740-748`.
**Issue:**
- `get_job` ignores both `_id` and the principal. Any authenticated principal of any tenant, and any role, can read a job's result if it holds the job id.
- Job ids are UUIDv4, so this is not guessable.
- Jobs are now spend-attributed to a tenant while their output remains tenant-unscoped, which is inconsistent with the phase's tenant model. This predates the phase and is out of PLAT-07's stated scope.

**Fix:** record the owning `(tenant_id, principal id)` on the `JobRecord` and return 404 to non-owners, or add the gap to the deferred-ideas list.

### IN-05: Small doc, test and consistency points

**Files:** `crates/paladin-core/src/platform/container/principal.rs:120` and `crates/paladin-ports/src/output/run_repository_port.rs:62`.
**Issue:**
- `RunAttribution.api_key_id` is an unvalidated `String`, while `tenant_id` is validated on construction and on read. An empty `api_key_id` is stored and then satisfies the "both set" check.
- `RunQuery` gains a public field (breaking for struct literals). It is covered by the semver allowlist and `..Default::default()` is the documented pattern, so no action beyond confirming the allowlist entry names it.

**Fix:** validate `api_key_id` the same way as `TenantId` (or make it a newtype) in `RunAttribution::new`.

---

Positive checks, for the record:
- Postgres `INSERT_RUN`/`INSERT_RUN_WITH_LATEST` bind counts and ordering are consistent (`$19/$20` and `$18/$19/$20`).
- Migration 008 is numbered without collision.
- The in-memory, SQLite and Postgres list paths agree on scope semantics.
- The HTTP agent handlers build the `RunScope` from the `Principal` only, never from the body.
- `TenantId` validation rejects untrimmed, whitespace, non-ASCII and over-128-byte values.

_Reviewed: 2026-09-29_
_Reviewer: gsd-code-reviewer_
_Depth: standard_
