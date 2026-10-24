---
phase: 40-tenant-identity-run-read-scoping
fixed_at: 2026-09-29T00:00:00Z
review_path: /home/user/paladin-dev-env/.planning/phases/40-tenant-identity-run-read-scoping/40-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 40: Code Review Fix Report

**Fixed at:** 2026-09-29
**Source review:** /home/user/paladin-dev-env/.planning/phases/40-tenant-identity-run-read-scoping/40-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (WR-01..WR-04; no Critical findings; IN-* out of scope)
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: `POST /v1/runs {thread_id}` and `fork` let a user-role principal run against, and mutate, another tenant's thread

**Files modified:** `src/application/services/run/submission.rs`, `src/application/services/run/http_surface_tests.rs`
**Commit:** 6d7efb16
**Status:** fixed: requires human verification (authorization logic)
**Applied fix:** Added a private `RunSubmissionService::ensure_thread_visible` guard. When `requested_by` is a non-Admin principal, it looks up the thread's latest run with the unrestricted `All` scope and, if `RunReadScope::for_principal(..).permits(&latest)` is false, returns `RunSubmissionError::UnknownThread` (the value a fork of a run-less thread already yields). It runs in `submit` (only when the caller supplies a `thread_id`, before any insert/enqueue) and first in `fork` (before the busy-thread and waypoint checks, so neither can confirm another tenant's thread exists). `None` principals (internal callers), Admins, and threads with no runs are untouched. Contract-style tests added for submit, fork, and the owner/Admin/internal/fresh-thread controls.

Notes for the reviewer:
- The existing `fork_run_completes_from_waypoint` tracer forked an unattributed thread as a tenant `User`. Under the stated rule (a tenant scope never permits an unattributed run, D-10/D-11) that is now refused, so the tracer forks as an Admin principal of the same tenant; the D-08 attribution assertion is unchanged.
- Residual, inherent limitation: `submit` accepts a caller-chosen id for a thread with no runs, so a refused hidden thread (404) is still distinguishable from an unused id (202). Closing that needs tenant-namespaced threads, which is the deferred thread-tenancy work (40-CONTEXT D-14). Documented in the helper's rustdoc.

### WR-02: Tenant isolation for `cancel` is enforced only in the HTTP controller

**Files modified:** `src/application/services/run/submission.rs`
**Commit:** fa054746
**Status:** fixed: requires human verification (authorization logic)
**Applied fix:** `RunSubmissionService::cancel` now applies `RunReadScope::for_principal(role, &tenant).permits(&run)` right after loading the run (when `requested_by` is `Some`) and returns `NotFound { run_id }` -- identical to a missing run -- before role authorization and before the durable cancel flag is written. Tests: cross-tenant cancel is `NotFound` and `cancel_requested` stays false; the owning tenant and an Admin can still cancel. The HTTP `load_visible_run` pre-check remains as the fast path.

### WR-03: A corrupt or half-attributed row turns into a 500 that leaks internals

**Files modified:** `crates/paladin-web/src/run_controller.rs`, `crates/paladin-web/CHANGELOG.md` (handler half, commit 05612748, shared with WR-04); `crates/paladin-storage/migrations/postgres/009_add_run_attribution_check.sql`, `crates/paladin-storage/src/run/postgres.rs`, `crates/paladin-storage/CHANGELOG.md`, `MIGRATION.md` (schema half)
**Commits:** 05612748 (handler half), 9afaef2b (Postgres CHECK)
**Applied fix:**
- Handler half: `load_visible_run` and `list_runs` now log the repository error server-side and return a fixed `500` body (`"run store error"`) -- see WR-04.
- Schema half: the review said to put the CHECK in migration 008. 008 is already committed, pushed to `origin/claude/laughing-dirac-e0h2ax`, and documented in `MIGRATION.md` §9.4 with "byte-untouched" wording for earlier migrations; sqlx records a checksum per applied version. It was therefore added as a NEW file, `postgres/009_add_run_attribution_check.sql`: `CHECK ((tenant_id IS NULL) = (api_key_id IS NULL))` named `runs_attribution_all_or_none`. SQLite has no counterpart (`ALTER TABLE ADD COLUMN` cannot add a cross-column CHECK) and relies on the read-time guard; documented in the migration header, the storage CHANGELOG and `MIGRATION.md`. A Postgres test (`migration_009_rejects_a_half_attributed_row`) was watched failing first, then passing, against a live local Postgres 16.
- Not changed (per the review's Fix text): a corrupt row for another tenant still yields a generic `500` rather than the uniform `404`; the CHECK prevents such rows from being written on Postgres, and SQLite relies on the read-time guard. Raising this to "treat corrupt rows as 404" was judged a behavior change beyond the review's guidance.

### WR-04: The new `Serialization` messages surface raw backend text to API clients

**Files modified:** `crates/paladin-web/src/run_controller.rs`, `crates/paladin-web/CHANGELOG.md`
**Commit:** 05612748
**Applied fix:** Added `internal_repo_error(context, err)`, which logs with `log::error!` and returns `ApiError::internal("run store error")`. Used for every repository/backend error in the file: `load_visible_run`, `list_runs`, webhook-delivery listing, the `RunSubmissionError::Backend`/catch-all arms of `map_submission_error` (shared with `thread_controller`), and the `RunStreamError::Backend`/catch-all arms of `stream_run`. Tests added for `get_run`, `list_runs` and `map_submission_error` asserting the body carries no backend text. The workspace has no `tracing` dependency, so the existing `log` facade (already a `paladin-web` dependency) is used.

## Skipped Issues

None -- all in-scope findings were fixed.

## Verification

- `cargo fmt --check`: pass.
- `cargo clippy -p paladin-ai -p paladin-web -p paladin-storage --all-features --all-targets -- -D warnings`: pass.
- `cargo test -p paladin-ai --lib`: 1029 passed, 0 failed.
- `cargo test -p paladin-web`: 260 + 5 + 7 passed, 0 failed.
- `cargo test -p paladin-storage --all-features` (with a live Postgres 16 on :5433): 548 passed, 0 failed, 9 ignored.
- `make api-surface`: exits non-zero, but the reported diff lists only unrelated items (e.g. `InProcessArsenal::invoke`/`list_armaments` signatures); none of the added symbols (`ensure_thread_visible`, `internal_repo_error`, `RUN_STORE_ERROR_MESSAGE`) appear -- all are private. No public-surface change was made by these fixes, so no baseline refresh was done.

---

_Fixed: 2026-09-29_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
