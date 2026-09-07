---
phase: 26-agent-runtime-enhancements
plan: 07
subsystem: database
tags: [garrison, sqlx, sqlite, migrations, non-exhaustive, semver, rust]

requires:
  - phase: 26-01
    provides: "The ExecutionMiddleware seam and prior plans' X-10 register precedent (26-03, 26-05)"
provides:
  - "GarrisonEntry.is_summary: bool (#[serde(default)]) plus GarrisonEntry::summary(content), under the full X-10 non-exhaustive treatment"
  - "crates/paladin-memory/src/migrations.rs -- the crate's one shared, compile-time-embedded sqlx migrator (static MIGRATOR, run_migrations), replacing SqliteGarrison's prior CWD-relative runtime Migrator::new path"
  - "crates/paladin-memory/migrations/002_add_garrison_is_summary.sql -- additive ALTER TABLE, 001 untouched"
  - "Root migrations/ mirror and its two Dockerfile COPY lines removed; docs/src/deployment/docker.md updated to match"
  - "MIGRATION.md §9.2 GarrisonEntry row resolved Y, §9.4 entry for 002, allowlist entry, and crates/paladin-core/Cargo.toml struct_marked_non_exhaustive suppression"
affects: [26-09, 26-15]

tech-stack:
  added: []
  patterns:
    - "X-10.3 option (a) for a Default-less struct (GarrisonEntry): #[non_exhaustive] + constructor is the only viable treatment, matching the LlmRequest (26-03) and StopReason (26-05) precedent"
    - "One shared static sqlx::migrate::Migrator per crate (crates/paladin-memory/src/migrations.rs), called by every adapter in that crate that needs schema, following the SqliteWaypointStore precedent (crates/paladin-storage/src/waypoint/sqlite.rs)"

key-files:
  created:
    - crates/paladin-memory/src/migrations.rs
    - crates/paladin-memory/migrations/002_add_garrison_is_summary.sql
  modified:
    - crates/paladin-core/src/platform/container/garrison.rs
    - crates/paladin-core/Cargo.toml
    - crates/paladin-memory/src/lib.rs
    - crates/paladin-memory/src/garrison/sqlite_garrison.rs
    - crates/paladin-memory/src/garrison/in_memory_garrison.rs
    - .cargo/semver-checks-allowlist.toml
    - MIGRATION.md
    - Dockerfile
    - docs/src/deployment/docker.md
  deleted:
    - migrations/001_create_garrison_tables.sql

key-decisions:
  - "Checkpoint (Task 1) auto-selected delete-mirror-now per AUTO_CFG/CONTEXT.md D-17 -- see Checkpoint resolutions below"
  - "sqlx::migrate!(\"./migrations\") used instead of the plan's literal sqlx::migrate!(\"migrations\") -- sqlx-macros-core 0.8.6 rejects a single-path-segment literal at compile time ('paths relative to the current file's directory are not currently supported'), a footgun-avoidance check unrelated to actual resolution (both forms resolve identically relative to CARGO_MANIFEST_DIR, confirmed by reading sqlx-macros-core's resolve_path source). The SqliteWaypointStore precedent's two-segment literal (\"migrations/sqlite\") never hit this guard, so the plan's literal single-segment form was untested against this sqlx version."
  - "existing_v0_9_database_migrates_forward builds its v0.9 fixture via sqlx::raw_sql(001's text).execute(&pool) rather than a scoped sqlx::migrate::Migrator::new(tmp_dir) -- the latter would have left a second Migrator::new call site in sqlite_garrison.rs, failing this plan's own acceptance criterion that greps that exact file for zero Migrator::new occurrences"

patterns-established:
  - "A source-level guard test (migrator_is_declared_once_in_the_crate) that assembles its search needle from concatenated string parts at runtime, so the guard's own source text is never counted as a second match of the pattern it is checking for"

requirements-completed: [RT-03]

coverage:
  - id: D1
    description: "GarrisonEntry gains #[serde(default)] pub is_summary: bool, is marked #[non_exhaustive], gains GarrisonEntry::summary(content); the three existing constructors set is_summary: false"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/garrison.rs#tests::existing_constructors_default_is_summary_to_false, ::summary_constructor_sets_role_and_flag"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc (GarrisonEntry::summary doc test)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A JSON document written before this change (no is_summary key) deserializes with is_summary == false; an entry with is_summary: true round-trips through serde"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/garrison.rs#tests::is_summary_defaults_on_deserialize, ::garrison_entry_round_trips_with_the_new_field"
        status: pass
    human_judgment: false
  - id: D3
    description: "MIGRATION.md §9.2 GarrisonEntry row resolved Y with the allowlist entry and crates/paladin-core/Cargo.toml struct_marked_non_exhaustive suppression, in the same commit as the field"
    requirement: "RT-03"
    verification:
      - kind: other
        ref: "grep 'GarrisonEntry' MIGRATION.md | grep -c TBD (0); grep -B4 'pub struct GarrisonEntry' garrison.rs | grep non_exhaustive; .cargo/semver-checks-allowlist.toml paladin-ai-core/struct_marked_non_exhaustive entry"
        status: pass
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 -- 'no semver update required' (193 checks, 193 pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "paladin-memory runs exactly one compile-time-embedded sqlx migrator (crates/paladin-memory/src/migrations.rs) over exactly one migrations/ directory; SqliteGarrison's runtime Migrator::new(\"./migrations\") path is gone"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/migrations.rs#tests::migrator_is_declared_once_in_the_crate"
        status: pass
      - kind: other
        ref: "grep -rc 'sqlx::migrate!' crates/paladin-memory/src --include=*.rs (sum = 1); grep -v '^\\s*//' sqlite_garrison.rs | grep -c 'Migrator::new' (0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "002_add_garrison_is_summary.sql runs ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0 beside an untouched 001; a v0.9 database (001 only, with a pre-existing row) migrates forward with the row intact and is_summary == false; SqliteGarrison constructs idempotently twice against the same file"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/garrison/sqlite_garrison.rs#tests::fresh_sqlite_garrison_has_the_is_summary_column, ::existing_v0_9_database_migrates_forward, ::sqlite_garrison_constructs_twice_idempotently"
        status: pass
      - kind: other
        ref: "git diff HEAD~1 -- crates/paladin-memory/migrations/001_create_garrison_tables.sql | wc -l (0, i.e. 001 byte-untouched)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The SQLite adapter's INSERT and SELECT (remember/recall_recent/search) carry is_summary via bound parameters (never format!-built SQL); the in-memory adapter passes the field through unchanged; both round-trip is_summary"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/garrison/sqlite_garrison.rs#tests::is_summary_round_trips_through_sqlite; crates/paladin-memory/src/garrison/in_memory_garrison.rs#tests::is_summary_round_trips_through_in_memory"
        status: pass
      - kind: other
        ref: "grep -v '^\\s*//' sqlite_garrison.rs | grep -c 'format!(\"SELECT\\|format!(\"INSERT' (0)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The root migrations/ mirror and its two Dockerfile COPY lines are removed; docs/src/deployment/docker.md's matching lines are updated; MIGRATION.md §9.4 records the 002 migration"
    requirement: "RT-03"
    verification:
      - kind: other
        ref: "ls migrations/001_create_garrison_tables.sql (gone); grep -c 'COPY migrations' Dockerfile (0); grep -c 'COPY migrations ./migrations' docs/src/deployment/docker.md (0); grep -c '002_add_garrison_is_summary.sql' MIGRATION.md (2)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Workspace-wide gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --check, cargo clippy -- -D warnings, cargo doc --workspace --no-deps (no new warnings from changed files)"
    requirement: "RT-03"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0); cargo doc --workspace --no-deps (exit 0, no garrison/migrations.rs warnings)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib (513 passed); cargo test -p paladin-ai-core --doc (82 passed); cargo test -p paladin-memory --features sqlite --lib (93 passed); cargo test -p paladin-memory --all-features --lib (118 passed)"
        status: pass
    human_judgment: false

duration: 55min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 07: Garrison is_summary + Embedded Migrator Summary

**GarrisonEntry gains a serde-defaulted `is_summary` flag and `summary()` constructor under the full X-10 non-exhaustive treatment; `paladin-memory` moved off a process-CWD-relative runtime SQL migrator onto one shared compile-time-embedded `sqlx::migrate!` static, with the root `migrations/` mirror and its Dockerfile `COPY` lines deleted.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-09-07T00:00:00Z (approx.)
- **Completed:** 2026-09-07T00:55:00Z (approx.)
- **Tasks:** 2 (Task 1 was a checkpoint, auto-resolved without stopping)
- **Files modified:** 11 (2 created, 8 modified, 1 deleted)

## Checkpoint resolutions

**Task 1 (checkpoint:decision, gate="blocking"):** Auto-selected **delete-mirror-now** — "Proceed as D-17 locks it, deleting the root mirror in this change." Rationale from the plan's option text: one reader of migrations immediately (the class of bug where the container's copy and the crate's copy drift cannot exist); the `Dockerfile` gets smaller; the `SqliteWaypointStore` precedent already proves the embedded pattern in this workspace; plan 26-09's `SqliteVault` inherits one unambiguous numbering sequence. Implemented in Task 3: root `migrations/001_create_garrison_tables.sql` deleted, `Dockerfile:28`/`:57` `COPY` lines deleted, `docs/src/deployment/docker.md`'s matching lines updated.

## Accomplishments
- `GarrisonEntry` (`crates/paladin-core/src/platform/container/garrison.rs`) gained `#[serde(default)] pub is_summary: bool`, `#[non_exhaustive]`, and `GarrisonEntry::summary(content)` — the three existing constructors (`new`, `with_metadata`, `with_token_count`) updated to set `is_summary: false`
- `crates/paladin-memory/src/migrations.rs` (new): the crate's one shared, compile-time-embedded `sqlx::migrate::Migrator` static and `run_migrations` wrapper, gated on the `sqlite` feature, with a source-level guard proving it is the crate's only such invocation
- `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` (new): additive `ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0`; `001` byte-untouched
- `SqliteGarrison::initialize()` now calls the shared embedded migrator instead of the prior `Migrator::new("./migrations")` runtime path; `INSERT`/`SELECT` in `remember`/`recall_recent`/`search` carry `is_summary` via bound parameters
- Root `migrations/` mirror and the two `Dockerfile` `COPY` lines removed; `docs/src/deployment/docker.md` updated to state migrations are embedded at compile time
- `MIGRATION.md` §9.2 (`GarrisonEntry` row resolved `Y`) and §9.4 (the `002` migration entry) updated; `.cargo/semver-checks-allowlist.toml` and `crates/paladin-core/Cargo.toml`'s lint-suppression table gained the matching entries, all in the same commits as the code changes they govern

## Task Commits

Each task was committed atomically, following RED/GREEN for both `tdd="true"` tasks:

1. **Task 1 (checkpoint:decision)** — auto-resolved, no commit (decision only; see Checkpoint resolutions)
2. **Task 2: GarrisonEntry.is_summary under the X-10 treatment**
   - `6f0de05e` (test) — failing tests for `existing_constructors_default_is_summary_to_false`, `summary_constructor_sets_role_and_flag`, `is_summary_defaults_on_deserialize`, `garrison_entry_round_trips_with_the_new_field`
   - `db61280e` (feat) — the field, `#[non_exhaustive]`, `summary()`, the four constructors, `Cargo.toml` suppression, allowlist entry, `MIGRATION.md` §9.2 row
3. **Task 3: One embedded migrator, the 002 column migration, and the root mirror removal**
   - `6a25459d` (test) — failing tests for `fresh_sqlite_garrison_has_the_is_summary_column`, `sqlite_garrison_constructs_twice_idempotently`, `is_summary_round_trips_through_sqlite`; `migrations.rs` scaffold; `is_summary_round_trips_through_in_memory` (already green, pure pass-through)
   - `4f08e88c` (feat) — the embedded migrator wiring, `002_add_garrison_is_summary.sql`, `INSERT`/`SELECT` column plumbing, root mirror + Dockerfile + docs removal, `MIGRATION.md` §9.4

**Plan metadata:** this commit (docs: complete plan) — see final commit below.

## Files Created/Modified
- `crates/paladin-core/src/platform/container/garrison.rs` — `is_summary` field, `#[non_exhaustive]`, `summary()`, updated constructors, X-10 rustdoc
- `crates/paladin-core/Cargo.toml` — `struct_marked_non_exhaustive = "allow"` suppression
- `crates/paladin-memory/src/migrations.rs` (new) — shared `MIGRATOR` static + `run_migrations`
- `crates/paladin-memory/src/lib.rs` — registers the `migrations` module behind `sqlite`
- `crates/paladin-memory/src/garrison/sqlite_garrison.rs` — embedded-migrator wiring; `is_summary` in INSERT/SELECT; 4 new tests
- `crates/paladin-memory/src/garrison/in_memory_garrison.rs` — 1 new round-trip test (no production change needed)
- `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` (new)
- `.cargo/semver-checks-allowlist.toml` — `paladin-ai-core` / `GarrisonEntry` entry
- `MIGRATION.md` — §9.2 `GarrisonEntry` row resolved `Y`; §9.4 `002` entry
- `Dockerfile` — two `COPY migrations` lines removed
- `docs/src/deployment/docker.md` — matching lines removed, prose updated
- `migrations/001_create_garrison_tables.sql` — **deleted** (root mirror)

## Decisions Made
- Checkpoint auto-resolved to `delete-mirror-now` (see Checkpoint resolutions above)
- `sqlx::migrate!("./migrations")` used instead of the plan's literal `"migrations"` (see Deviations)
- `existing_v0_9_database_migrates_forward`'s v0.9 fixture built via `sqlx::raw_sql` instead of a scoped `Migrator::new` (see Deviations)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `sqlx::migrate!("migrations")` fails to compile; used `"./migrations"` instead**
- **Found during:** Task 3 (writing `crates/paladin-memory/src/migrations.rs`)
- **Issue:** The plan's action text specifies `sqlx::migrate!("migrations")` (a single path segment). `sqlx-macros-core` 0.8.6's `resolve_path` rejects any relative literal whose `Path::parent()` is empty — i.e. any single-segment literal — with `"paths relative to the current file's directory are not currently supported"`. This is a compile-time footgun guard, not an actual difference in resolution: reading `resolve_path`'s source confirms both `"migrations"` and `"./migrations"` resolve identically, relative to `CARGO_MANIFEST_DIR`, once past the guard. The `SqliteWaypointStore` precedent this plan cites uses a two-segment literal (`"migrations/sqlite"`), which never triggers this guard — so the plan's exact literal was untested against this crate's single-level `migrations/` layout.
- **Fix:** Used `sqlx::migrate!("./migrations")`, which resolves to the same `crates/paladin-memory/migrations/` directory and compiles.
- **Files modified:** `crates/paladin-memory/src/migrations.rs`
- **Verification:** `cargo check --workspace --all-targets --all-features` passes; `migrator_is_declared_once_in_the_crate` confirms exactly one invocation.
- **Committed in:** `6a25459d` / `4f08e88c`

**2. [Rule 1 - Bug] `existing_v0_9_database_migrates_forward`'s v0.9 fixture built via `sqlx::raw_sql`, not `Migrator::new`**
- **Found during:** Task 3, writing the forward-migration test
- **Issue:** The straightforward way to build a "v0.9-shaped" database for the test is a scoped `sqlx::migrate::Migrator::new(tmp_dir_with_only_001)`. But this plan's own acceptance criteria (and Task 3's `<done>` clause) require `grep -v '^\s*//' crates/paladin-memory/src/garrison/sqlite_garrison.rs | grep -c 'Migrator::new'` to be `0` — a second `Migrator::new` call site in that same file, even in test code proving the *opposite* of the runtime bug this plan fixes, would fail that literal check and (more importantly) reads as ambiguous evidence that the runtime path might still be reachable.
- **Fix:** The test instead executes `001`'s raw SQL text directly via `sqlx::raw_sql(...).execute(&pool)` (no `_sqlx_migrations` bookkeeping created), then calls `SqliteGarrison::connect` — the crate's real embedded migrator sees no recorded migrations, re-applies `001` (idempotent by design: `CREATE TABLE/INDEX/TRIGGER IF NOT EXISTS`, `INSERT OR IGNORE`) and then applies `002`. This is arguably a *stronger* test of the embedded migrator's tolerance for a schema that predates any `_sqlx_migrations` bookkeeping at all, not just a schema that recorded `001` under a different migrator instance.
- **Files modified:** `crates/paladin-memory/src/garrison/sqlite_garrison.rs`
- **Verification:** `existing_v0_9_database_migrates_forward` passes; `grep -v '^\s*//' sqlite_garrison.rs | grep -c 'Migrator::new'` returns `0`.
- **Committed in:** `4f08e88c`

**3. [Documentation only] The bench struct-literal migration was already done**
- **Found during:** Task 2, reading `crates/paladin-memory/benches/garrison_benchmarks.rs`
- **Issue:** The plan's `<read_first>`/`<action>` describe "the two in-tree `GarrisonEntry { .. }` struct literals" in this bench file, to be replaced with constructor calls. A direct read and grep of the current file found zero `GarrisonEntry { .. }` struct literals — `create_entry` already uses `GarrisonEntry::with_token_count(..)`. The acceptance criterion's literal grep command (`grep -rc 'GarrisonEntry {' crates/paladin-memory/benches/garrison_benchmarks.rs`) returns `1`, but that one match is the function signature `fn create_entry(index: usize) -> GarrisonEntry {` (the arrow-return-type followed by the function body's opening brace), not a struct literal — the same class of grep false-positive noted in plan 26-03's SUMMARY for `LlmRequest {`.
- **Fix:** No code change was needed or made in this file; `garrison_benchmarks.rs` already compiles under the new construction contract (verified by `cargo check -p paladin-memory --benches --all-features`).
- **Files modified:** none
- **Verification:** `cargo check -p paladin-memory --benches --all-features` exits 0.
- **Committed in:** n/a (no change required)

---

**Total deviations:** 3 (2 auto-fixed under Rules 1/3, 1 documentation-only clarification)
**Impact on plan:** All three are necessary corrections to plan assumptions that didn't hold against this sqlx version or this file's current state; no scope creep, no architectural change.

## Issues Encountered
None beyond the deviations above.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Plan 26-09's `SqliteVault` can call `crate::migrations::run_migrations` directly and add `003_create_vault_tables.sql` beside `002`, sharing the one embedded migrator and one `_sqlx_migrations` numbering sequence, per D-23.
- Plan 26-15's summarization middleware can call `GarrisonEntry::summary(content)` and set `metadata["summarized_through"]` itself, per D-16 — this plan deliberately left that key unset in the constructor.
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor — the orchestrator owns those writes after all wave 4 worktree agents complete.

## Self-Check: PASSED

Verified on disk / in git history:
- `crates/paladin-core/src/platform/container/garrison.rs` — FOUND, contains `pub is_summary: bool` and `pub fn summary(`
- `crates/paladin-memory/src/migrations.rs` — FOUND
- `crates/paladin-memory/migrations/002_add_garrison_is_summary.sql` — FOUND
- `migrations/001_create_garrison_tables.sql` (root mirror) — CONFIRMED ABSENT
- `.cargo/semver-checks-allowlist.toml` — FOUND, contains `paladin-ai-core` / `struct_marked_non_exhaustive` / `GarrisonEntry` entry
- Commit `6f0de05e` — FOUND in `git log --oneline`
- Commit `db61280e` — FOUND in `git log --oneline`
- Commit `6a25459d` — FOUND in `git log --oneline`
- Commit `4f08e88c` — FOUND in `git log --oneline`
- `cargo test -p paladin-ai-core --lib` — 513 passed, 0 failed
- `cargo test -p paladin-ai-core --doc` — 82 passed, 0 failed
- `cargo test -p paladin-memory --features sqlite --lib` — 93 passed, 0 failed
- `cargo test -p paladin-memory --all-features --lib` — 118 passed, 0 failed
- `cargo check --workspace --all-targets --all-features` — exit 0
- `cargo fmt --all --check` — exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — exit 0
- `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` — "no semver update required" (193 checks, 193 pass)
- `cargo doc --workspace --no-deps` — exit 0, no warnings attributable to changed files

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*
