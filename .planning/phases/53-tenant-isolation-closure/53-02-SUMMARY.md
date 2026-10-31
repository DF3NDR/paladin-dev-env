---
phase: 53-tenant-isolation-closure
plan: 02
subsystem: tenant-isolation
tags: [rust, tenant-isolation, redaction, run-routes, axum, tdd, prove-then-lock]

requires:
  - phase: 40-tenant-scoped-runs
    provides: RunReadScope, load_visible_run, Principal::read_scope
  - phase: 53-tenant-isolation-closure
    provides: "plan 53-01 RunReadScope::permits_thread (unused here; same scope type)"
provides:
  - "redact_store_error_for_scope (pub(crate)): the one role-aware hidden-equals-missing redaction rule"
  - "run_repo_error_for_scope (pub(crate)): the run-repository classifier (Serialization only is hideable)"
  - "load_visible_run routed through the rule: an unreadable run row is the missing-run 404 for a non-admin, the fixed 500 for an Admin"
  - "MockRepository Backend failure seam (failing_reads_with_backend)"
  - "MIGRATION.md 11.1 row M-B-23 and 11.6 paragraph; paladin-web CHANGELOG [Unreleased] Security entry"
affects: [53-03, 53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "One pub(crate) redaction helper plus a per-store classifier that decides hideable; the route builds its own missing-object 404 in a closure so hidden and missing share one constructor"
    - "Hidden-equals-missing proven by comparing (status, sorted headers, body bytes) of the failing router against an empty healthy router for the same id, never a hand-written expected string"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "Only RunRepositoryError::Serialization is hideable; UnknownSchemaVersion and every other variant stay the fixed 500 for every role (D-09, PD-02-2)"
  - "internal_repo_error is unchanged and still serves the non-scoped call sites (list, submission, stream subscription)"
  - "Tenant-scoped detail logging: both branches log::error! the route context and the repository error; the hidden branch adds the words 'hidden as not found' so an operator sees the row the tenant cannot (T-53-11)"

patterns-established:
  - "Watched-red by local mutation: replace the hide arm with 'if false && ..', run the new tests, restore from a backup copy"

requirements-completed: []

coverage:
  - id: D1
    description: "redact_store_error_for_scope decision table: only (Tenant, hideable) yields the hidden 404; every other cell is the fixed 500 and no detail reaches the body"
    requirement: "TENANT-06"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib run_controller::tests::redact_store_error_for_scope_hides_only_hideable_errors_for_a_tenant"
        status: pass
    human_judgment: false
  - id: D2
    description: "A corrupt run row on GET /v1/runs/{id} is byte-identical to the missing-run 404 for a non-admin and the fixed 'run store error' 500 for an Admin (tracer)"
    requirement: "TENANT-06"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-web --lib run_controller::tests::corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal"
        status: pass
    human_judgment: false
  - id: D3
    description: "All four /v1/runs/{run_id}* operations (read from the router's OpenAPI document) hide an unreadable row from a non-admin byte-identically, redact it for an Admin, leak no repository text, and never reach the cancel port"
    requirement: "TENANT-06"
    verification:
      - kind: integration
        ref: "run_controller::tests::run_controller_auth::serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin"
        status: pass
    human_judgment: false
  - id: D4
    description: "GET /v1/runs keeps the fixed 500 for every role; a Backend error stays the fixed 500 for a tenant (no existence oracle)"
    requirement: "TENANT-06"
    verification:
      - kind: integration
        ref: "run_controller::tests::run_controller_auth::run_list_serialization_stays_a_fixed_500_for_every_role"
        status: pass
      - kind: integration
        ref: "run_controller::tests::run_controller_auth::run_backend_error_stays_a_fixed_500_for_a_tenant"
        status: pass
    human_judgment: false
  - id: D5
    description: "An unattributed valid run is the missing-run 404 for a non-admin and 200 for an Admin (ADR-0054 rule 3)"
    requirement: "TENANT-06"
    verification:
      - kind: integration
        ref: "run_controller::tests::run_controller_auth::unattributed_valid_run_is_hidden_from_tenant_and_visible_to_admin"
        status: pass
    human_judgment: false
  - id: D6
    description: "MIGRATION.md 11.1 and 11.6 record the behavioral change; 11.2 stays empty; 9 and 10 untouched"
    requirement: "TENANT-06"
    verification:
      - kind: other
        ref: "make check-migration-allowlist"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 02: Role-aware redaction of unreadable run rows Summary

**One `pub(crate)` rule, `redact_store_error_for_scope`, makes an unreadable run row the byte-identical missing-run `404` for a non-admin and the fixed `500 "run store error"` for an Admin on all four `/v1/runs/{run_id}*` routes, with the detail only in the server log.**

## Performance

- **Duration:** about 25 min wall clock
- **Started:** 2026-10-31T02:12Z
- **Completed:** 2026-10-31T02:35Z
- **Tasks:** 2 (Task 1 tracer, Task 2 auto)
- **Files modified:** 3
- **Plan start SHA:** `91df4d19d` (the 53-01 docs commit; code base for this plan)

## Accomplishments

- `redact_store_error_for_scope(scope, context, detail, hideable, fixed_message, hidden)`: when `hideable` and the scope is `Tenant`, it logs at `error` and returns `hidden()`; every other case logs at `error` and returns `ApiError::internal(fixed_message)`. Rustdoc names D-09, ADR-0054 rule 5 and ADR-0063.
- `run_repo_error_for_scope(scope, context, err, hidden)`: `hideable` is `matches!(err, RunRepositoryError::Serialization { .. })`, message `RUN_STORE_ERROR_MESSAGE` (`"run store error"`, unchanged). Rustdoc names the SQLite read-time half-attribution rejection and the `UnknownSchemaVersion` residual (PD-02-2).
- `load_visible_run` computes `principal.read_scope()` once, maps the `get` error through `run_repo_error_for_scope` with the same `ApiError::not_found(format!("unknown run '{run_id}'"))` constructor the missing and hidden branches use, and calls `scope.permits(&run)`. The existing Admin-principal test `get_run_repository_error_is_a_generic_500_without_backend_text` passes unchanged.
- Route matrix over the four operations read from the router's own OpenAPI document (asserted to be exactly four), for the owner and foreign keys: failing-router response equals empty-router response for the same id in status, every header and body bytes; Admin gets `500` with message `run store error`; no body contains `db.internal` or `attribution is incomplete`; the recording cancel port is never called.
- `MockRepository` gained `failing_reads_with_backend` (a `Backend` error on `get`) so the "non-Serialization stays 500" truth is tested.
- `MIGRATION.md` 11.1 table row `M-B-23` and 11.6 bold-lead paragraph; `paladin-web` `CHANGELOG.md` `## [Unreleased]` with `### Security`; module docs "Single run" paragraph gained one sentence.

## Task Commits

1. **Task 1 (tracer): redaction rule wired through `load_visible_run` to `GET /v1/runs/{run_id}`** - `30b532813` (feat)
2. **Task 2: route matrix, list/Backend/unattributed tests, MIGRATION 11.1/11.6, CHANGELOG** - `839a8119a` (test)

Tracer feedback gate (auto mode active): the tracer's `<verify>` (tracer test, unit table, whole `run_controller` module, 58 passed) was green after its commit, before Task 2 began.

## Prove-then-lock

- Tracer was RED before the wiring: with the helpers added but `load_visible_run` unchanged, `corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal` failed (a tenant got `500`, not `404`). The unit table first failed to compile (`cannot find function redact_store_error_for_scope`).
- With the hide arm mutated locally (`if false && hideable && ..`, uncommitted, restored from a backup copy): `corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal`, `serialization_on_every_run_id_route_is_hidden_for_tenant_and_redacted_for_admin` and the unit table failed; 59 of 62 module tests passed.
- **Passed on arrival** (behavior already in place before this plan, locked here): `run_list_serialization_stays_a_fixed_500_for_every_role`, `run_backend_error_stays_a_fixed_500_for_a_tenant`, `unattributed_valid_run_is_hidden_from_tenant_and_visible_to_admin`.

## Verification

- `cargo test -p paladin-web`: lib 301 passed (62 in the `run_controller` module, up from 56 on the base), `auth_rbac` 5, `openapi_golden_v0_9` 9, doctests 2; all passed.
- `cargo fmt --check`, `cargo clippy -p paladin-web --all-targets -- -D warnings`, `make check-migration-allowlist`, `make check-changelogs`: exit 0.
- `make api-surface`: "API surface unchanged" (4323 items). No `pub` signature changed, no `#[deprecated]`, no 11.2 row (CONTEXT D-02).
- `git diff 91df4d19d -- MIGRATION.md` hunks start at lines 5-16 and 35-37 only (11.1 and 11.6); sections 9 and 10 are untouched and the 11.2 table has the header and separator but no data row.
- Manual credential-handling check (security.instructions.md): the new `log::error!` lines format only the route context string and the repository error; no API key, config type or request body is formatted.

## Decisions Made

See `key-decisions` above; all follow the plan's PD-02-1, PD-02-2 and PD-02-4.

## Deviations from Plan

None - plan executed exactly as written.

Notes that are not deviations:
- The tracer tests use a small borrowing helper `send_as_key` and shared `tenant_auth_config` / `capture` helpers in the outer test module, because the matrix module's `send_as` is private to `run_controller_auth`. The Task 2 matrix tests reuse the matrix module's own `send_as`.
- `requirements-completed` is empty on purpose. TENANT-06's text covers both the run and the thread halves; the thread half lands in plan 53-03, so TENANT-06 stays Pending in REQUIREMENTS.md until then (same handling 53-01 used for TENANT-03).

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or schema change. T-53-07, T-53-08, T-53-09 and T-53-11 are mitigated and tested; T-53-10 (`UnknownSchemaVersion` row after a downgrade answers a tenant `500`) is accepted and recorded in the `redact`/`run_repo_error_for_scope` rustdoc for ADR-0063.

## Next Phase Readiness

Plan 53-03 can call `redact_store_error_for_scope` (and mirror `run_repo_error_for_scope` for waypoint errors) from `thread_controller.rs`; both are `pub(crate)`. The module docs' closing paragraph about thread routes was left untouched for 53-03 to rewrite.

## Self-Check: PASSED

- FOUND: crates/paladin-web/src/run_controller.rs (`pub(crate) fn redact_store_error_for_scope`, `pub(crate) fn run_repo_error_for_scope`, `run_repo_error_for_scope(` in `load_visible_run`)
- FOUND: MIGRATION.md (`M-B-23` in 11.1 and 11.6)
- FOUND: crates/paladin-web/CHANGELOG.md (`## [Unreleased]` above `## [0.11.0]`)
- FOUND: commits `30b532813` and `839a8119a`
