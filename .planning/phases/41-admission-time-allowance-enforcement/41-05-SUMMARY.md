---
phase: 41-admission-time-allowance-enforcement
plan: 05
subsystem: treasurer
tags: [allowance, schedules, attribution, migration, postgres, sqlite, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: RunSubmissionService admit/confirm/abandon lifecycle, RunSubmissionError::AllowanceExhausted, the recorded design checkpoint (option-b, item 7)
  - phase: 41-admission-time-allowance-enforcement
    plan: 04
    provides: the shared admit_and_persist helper that schedule-fired submissions reuse
provides:
  - RunSchedule.created_by and with_created_by (serde-defaulted, skipped when None)
  - migration 010 on SQLite and PostgreSQL (run_schedules.tenant_id and api_key_id, PostgreSQL CHECK run_schedules_created_by_all_or_none)
  - created_by round-tripped by the in-memory, SQLite and PostgreSQL schedule adapters, never changed by update
  - CreateRunSchedule.created_by stamped by POST /v1/schedules from the authenticated principal (not echoed in ScheduleResponse)
  - SubmitRun.attributed_to (identity for principal-less submissions, never a role)
  - SkipReason::AllowanceExhausted: an exhausted creator's tick fires nothing and increments skipped_ticks
affects: [41-06, 41-07, 41-08, 41-09]

tech-stack:
  added: []
  patterns:
    - "Effective attribution is computed once in submit: requested_by.attribution() wins, attributed_to applies only when there is no principal; authorize_invocation and ensure_thread_visible keep reading requested_by alone"
    - "A schedule-fired run reuses the ordinary admit_and_persist lifecycle; the fire site only chooses the attribution and maps the refusal to a counted skip"
    - "Handler rustdoc is the OpenAPI description, so the frozen-baseline create_schedule handler keeps its doc comment unchanged and documents the stamping in // comments"

key-files:
  created:
    - crates/paladin-storage/migrations/sqlite/010_add_run_schedule_created_by.sql
    - crates/paladin-storage/migrations/postgres/010_add_run_schedule_created_by.sql
  modified:
    - crates/paladin-core/src/platform/container/run_schedule.rs
    - crates/paladin-storage/src/run_schedule/in_memory.rs
    - crates/paladin-storage/src/run_schedule/sqlite.rs
    - crates/paladin-storage/src/run_schedule/postgres.rs
    - crates/paladin-storage/src/run_schedule/contract_tests.rs
    - crates/paladin-ports/src/input/schedule_admin_port.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-web/src/schedule_controller.rs
    - crates/paladin-web/src/run_controller.rs
    - src/application/services/run/schedule/admin.rs
    - src/application/services/run/schedule/service.rs
    - src/application/services/run/schedule/tests.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/http_surface_tests.rs
    - src/application/services/assistant/tests.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - .planning/WINDOWS.md
    - docs/src/api-reference/platform-api.md
    - CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Schedule-fired runs keep skipping authorize_invocation (written decision, D-08/C13): attributed_to is identity only, so an Admin-created schedule on an assistant whose allowed_roles excludes Admin still fires"
  - "ScheduleResponse does not expose created_by (Open Question 7): GET /v1/schedules is not tenant-scoped"
  - "A pre-Phase-41 schedule (NULL created_by) fires unattributed and ungated; recorded as open WINDOWS.md row 63 with the closing condition 're-create through POST /v1/schedules'"
  - "A removed creator key is gated by the tenant allowance only and stays attributed by its persisted names (C12)"

patterns-established:
  - "Both-or-neither attribution columns: PostgreSQL CHECK plus read-side Serialization rejection on every adapter"

requirements-completed: [ALLOW-02]

duration: ~75min
completed: 2026-10-03
status: complete
---

# Phase 41 Plan 05: Schedule creator attribution and tick-time allowance admission Summary

**A schedule now remembers who created it (migration 010 on both backends), every fired run is attributed to and admitted against that creator, and a tick whose creator's tenant or API-key allowance is exhausted is a counted skip that writes nothing -- while the role check at fire time is provably unchanged.**

## Performance

- **Duration:** ~75 min (warm workspace; one disk-space recovery, see deviations)
- **Completed:** 2026-10-03
- **Tasks:** 3 (all `type="auto"`, Tasks 1 and 2 `tdd="true"`)
- **Files modified:** 22 (2 new migrations)

## Accomplishments

- **Persistence (Task 1).** `RunSchedule` gained `created_by: Option<RunAttribution>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`) and `with_created_by`. Migration `010` adds nullable `tenant_id` and `api_key_id` to `run_schedules` on both backends; PostgreSQL adds the CHECK `run_schedules_created_by_all_or_none`, and the SQLite header documents why there is no `009`. The SQLite and PostgreSQL adapters write both columns in `INSERT_SCHEDULE`, read them in `SELECT_BY_ID`, `LIST_PREFIX` and `DUE_QUERY`, map an invalid tenant or a half-attributed row to `Serialization`, and never touch the columns in `update`, `CLAIM_TICK` or `increment_skipped`. The in-memory adapter preserves the field through `apply_update`.
- **Stamping (Task 2).** `CreateRunSchedule.created_by` is set by `create_schedule` from `PrincipalRef::from(&principal).attribution()` (tenant id and key name, never the role) and applied by the admin service. `ScheduleResponse` does not carry it.
- **Attribution and admission (Task 2).** `SubmitRun.attributed_to` is used only when `requested_by` is `None`; `submit` computes the effective attribution once and uses it for `Run.submitted_by` and as the admission subject. The fire site passes `attributed_to: schedule.created_by.clone()` with `requested_by: None`, and an `AllowanceExhausted` refusal becomes `Skipped { reason: SkipReason::AllowanceExhausted }` with `increment_skipped` called exactly like the `ThreadBusy` arm. The warn line prints only the refusal's figures-only `Display`.
- **Registers and docs (Task 3).** MIGRATION.md 9.2 (extended `SubmitRun` row plus new `CreateRunSchedule` Y, `RunSchedule` N and `SkipReason` N rows), 9.4 `010` bullet, 9.6 schedule extension; two new semver allowlist entries; WINDOWS.md row 63 (open); platform-api.md "Schedules" gains creator attribution, the allowance skip, the legacy-NULL gap and the role-check statement; CHANGELOG entries; the public-API baseline refreshed (4158 items).

## Task Commits

1. **Task 1: persist the schedule creator on both backends (migration 010)** - `e469820` (feat)
2. **Task 2: stamp the schedule creator and admit fired runs against it** - `fe144a8` (feat)
3. **Task 3: register schedule attribution in MIGRATION, allowlist, WINDOWS, docs and API baseline** - `8954c18` (docs)

## TDD / red evidence

- **Task 1:** honest note: the core field and adapter changes landed before the contract clauses were run, so the clauses passed on the first run rather than failing first. They are not vacuous: `half_attributed_schedule_row_is_rejected_on_read` asserts a real `Serialization` error from a raw `UPDATE` on SQLite and the named CHECK violation on PostgreSQL, and `migration_010_adds_nullable_creator_columns` reads `PRAGMA table_info`.
- **Task 2:** the fire-site mutation `attributed_to: None` made three of the four allowance tick tests fail (`tick_fires_attributed_to_the_creator`, `tick_for_an_exhausted_creator_is_skipped_and_counted`, `removed_creator_key_is_gated_by_its_tenant_allowance_only`), then was restored. `submission.rs` and `schedule_controller.rs` tests were written after the implementation (the same note as Task 1 applies).

## Verification

- **PostgreSQL ran for real.** The throwaway cluster at `/var/tmp/pg41` (port 5433) was down after the container restart; I brought it up with the `pg_ctl` command recorded in 41-02-SUMMARY. `cargo test -p paladin-storage --features postgres --lib run_schedule::postgres` ran **14 passed, 0 SKIP**, including `half_attributed_schedule_row_is_rejected_on_read` against the real CHECK. The cluster is still running.
- `cargo test -p paladin-storage --features sqlite --lib run_schedule::` 29 passed (in-memory 13, SQLite 16), including the five named clauses. `sqlx::migrate!` accepted the SQLite 008 -> 010 gap, so no `009_no_op_alignment.sql` was needed (RESEARCH A1 resolved: the gap is fine).
- `cargo test -p paladin-ai-core --lib run_schedule` 12 passed; `cargo test -p paladin-ai --lib application::services::run` 190 passed (also with `--features web-server`); `application::services::assistant` 28 passed; `cargo test -p paladin-web --lib` 272 passed; `openapi_golden_v0_9` 8 passed (unchanged: the handler rustdoc was not touched); both ports doctests passed.
- `cargo test --workspace` (default features): 6229 passed, 0 failed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` clean for the four touched crates.
- `./scripts/check-migration-allowlist.sh` exit 0; no `TBD` in MIGRATION.md; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exits 0 after `make api-surface-update`, whose only diff was this plan's two `SkipReason::AllowanceExhausted` lines (plus the header date and count).
- Acceptance greps: `ScheduleResponse` contains 0 `created_by`; `authorize_invocation` contains 0 `attributed_to`; `grep -c "tenant_id, api_key_id"` is 6 in each of `sqlite.rs` and `postgres.rs`.
- Not run: `make security` (no dependency changed in this plan).
- Manual credential-handling review: no API key value is read, logged or persisted (only the key's name); the skip warn line carries the refusal's figures-only `Display`; the creator is not exposed over HTTP (`schedule_response_does_not_expose_the_creator`); no HTTP client or redirect behaviour changed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Disk exhaustion during the all-features workspace test**
- **Found during:** final verification (`cargo test --workspace --all-features`)
- **Issue:** the linker died with "No space left on device".
- **Fix:** `rm -rf target/debug/incremental` (as instructed) and reran the workspace tests with `CARGO_INCREMENTAL=0` on the default feature set (6229 passed). The all-features build was covered by the earlier `cargo check` and `cargo clippy --workspace --all-targets --all-features`.
- **Commit:** none (no source change)

**2. [Rule 2 - Missing critical] Per-crate changelog entry for the migration**
- `crates/paladin-storage/CHANGELOG.md` has recorded every storage migration since `008`; added the `010` bullet there (not in the plan's file list). Committed in `8954c18`.

**3. [Note - mechanical edit] `SubmitRun` literals migrated by script**
- All 22 in-tree literals (two of them rustdoc examples) plus the fire site were updated with `attributed_to: None`; the first scripted pass mangled the two doc examples and two test helper functions, which I repaired by hand before any commit (the compiler and the doctests prove the result).

**Total deviations:** 1 blocking (disk), 1 additive changelog, 1 note; no scope change.

## Authentication Gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-41-23 (tick as an allowance bypass) is mitigated by the fire site's `attributed_to` plus `tick_for_an_exhausted_creator_is_skipped_and_counted` and the mutation evidence above. T-41-24 (silently granting or dropping a role) by `authorize_invocation` never reading `attributed_to` (awk check 0) and `attributed_to_never_triggers_the_role_check`. T-41-25 (half-attributed rows) by the PostgreSQL CHECK, the read-side rejection on both SQL adapters, and the contract and adapter tests. T-41-26 (creator disclosure) by `ScheduleResponse` omitting `created_by` and `schedule_response_does_not_expose_the_creator`. T-41-27 (legacy NULL-creator schedules ungated) is accepted and tracked as open WINDOWS.md row 63.

## Notes for later plans

- WINDOWS.md `open_count` is now 4 (row 63 is this plan's, left `open` on purpose: the plan's closing condition is an operator re-creating the schedules).
- `.planning` and the PostgreSQL cluster at `/var/tmp/pg41` (port 5433) are left as found; the cluster is running for 41-06 (migrations `011`/`012` follow `010`).
- Commit trailers use the `Claude Fable 5.1` line from this dispatch, as 41-03 and 41-04 did.

## Self-Check: PASSED

Commits `e469820`, `fe144a8` and `8954c18` are present in `git log`; both `010_add_run_schedule_created_by.sql` files exist; every file in the key-files list exists and was modified.
