---
phase: 28-observability-tooling
plan: 04
subsystem: observability
tags: [trace, sqlx, sqlite, postgres, retention, storage-adapter]

# Dependency graph
requires:
  - phase: 28-01
    provides: "The authoritative twelve-variant TraceEvent/TraceRecord envelope in paladin-core (TRACE_SCHEMA_VERSION = \"1\"), re-exported from paladin-ports::output::trace_sink_port"
provides:
  - "RunTracePort (append/read/prune_thread) and RunTraceError (Backend, Serialization, UnsupportedSchemaVersion) in paladin-ports"
  - "InMemoryRunTraceStore, SqliteRunTraceStore, PostgresRunTraceStore in paladin-storage, sharing one contract_tests suite (run_all)"
  - "Migration 006_create_run_traces_table.sql on both SQLite and Postgres backends: append-only run_traces table, PRIMARY KEY (thread_id, seq)"
  - "paladin_storage::run_trace::retention::prune -- a small sibling to waypoint::retention::prune, not a generalisation of it"
  - "WaypointRetentionService::with_run_trace_port -- additive optional second port, one config, one routine, two ports"
affects: ["28-10 (execution overlay reads fired/evaluated edges from run_traces)", "28-11 (SSE replay path reads run_traces through RunTracePort)", "28-17 (MIGRATION.md reconciliation across the whole phase)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Envelope-as-blob-plus-summary-columns: run_traces.record stores the FULL serialized TraceRecord JSON (thread_id/run_id/seq/at/event all flattened), while thread_id/seq/run_id/superstep/at/schema_version are broken out as their own columns purely to serve read()'s (thread_id, seq > ?) range scan and prune_thread()'s (thread_id, superstep < ?) deletion without parsing record for either -- mirrors Waypoint's payload+status column split"
    - "Derived superstep bucketing: only 5 of TraceEvent's 12 variants carry an explicit superstep field; run_trace::superstep_of() maps RunFinished to its own total_supersteps and every other variant (RunStarted, NodeProgress, EdgeEvaluated, ParleyRaised, FallbackHop, MiddlewareEvent) to 0, so prune_thread(thread, N>0) always removes them alongside genuinely stale rows"
    - "Backend-specific contract-clause extension via a second trait: RawSchemaVersionWriter (contract_tests.rs) lets one clause (unsupported_schema_version_is_typed) bypass append()'s always-current schema stamping without adding a raw-write method to RunTracePort itself"

key-files:
  created:
    - crates/paladin-ports/src/output/run_trace_port.rs
    - crates/paladin-storage/src/run_trace/mod.rs
    - crates/paladin-storage/src/run_trace/in_memory.rs
    - crates/paladin-storage/src/run_trace/sqlite.rs
    - crates/paladin-storage/src/run_trace/postgres.rs
    - crates/paladin-storage/src/run_trace/contract_tests.rs
    - crates/paladin-storage/src/run_trace/retention.rs
    - crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql
    - crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql
  modified:
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-storage/src/lib.rs
    - src/application/services/waypoint_retention.rs

key-decisions:
  - "Task 1 checkpoint:decision -- option-a auto-selected by the orchestrator under --auto (2026-09-08): proceed with D-17 verbatim (migration 006, both backends, run_traces (thread_id, seq, run_id NULL, superstep, at, schema_version, record, PRIMARY KEY (thread_id, seq)), append-only)."
  - "record stores the FULL serialized TraceRecord (not just the TraceEvent payload) so read() deserializes a row directly with zero reconstruction from columns; thread_id/seq/run_id/superstep/at/schema_version are query-serving duplicates, not the source of truth."
  - "superstep_of() derives a per-row superstep bucket for pruning: 5 of 12 TraceEvent variants carry one directly, RunFinished uses its own total_supersteps, everything else is stamped 0 (oldest bucket, never protected past the run's own progress) -- documented in run_trace/mod.rs since TraceRecord itself has no envelope-level superstep field."
  - "unsupported_schema_version_is_typed needed a way to write a row with an unsupported schema_version, which append() can never do (it always stamps the current TRACE_SCHEMA_VERSION). Added a second, test-only trait (RawSchemaVersionWriter) in contract_tests.rs rather than adding a raw-write escape hatch to RunTracePort itself -- keeps the public port's three-method surface exactly as D-17 specifies."
  - "WaypointRetentionService::prune()'s return type stays Result<PruneReport, WaypointError>, UNCHANGED, even though the plan's prose says to 'extend PruneReport additively with the trace-row count.' PruneReport is defined in crates/paladin-storage/src/waypoint/retention.rs, which is NOT in this plan's files_modified list, and paladin-ai's own [lib] (name = \"paladin\") is semver-checked -- extending PruneReport's shape (or changing prune()'s return type to a wrapper) would be a real breaking change requiring a Cargo.toml suppression comment + MIGRATION.md §9.2 entry + .cargo/semver-checks-allowlist.toml entry, all three together per this project's established precedent, and MIGRATION.md is explicitly plan 28-17's to own per this plan's own project_execution_rules. Resolved by exposing the trace-row count through a new, purely additive WaypointRetentionService::last_run_traces_removed() getter (backed by an AtomicU64, since prune() takes &self) instead -- zero semver break, zero bookkeeping-file edits needed."
  - "The run_traces superstep boundary handed to a thread's prune_thread call is derived from the MINIMUM superstep among that thread's SURVIVING (post-Waypoint-prune) history, computed via a per-thread self.port.history() lookup before the (synchronous) before_superstep_for closure runs -- paladin_storage::run_trace::retention::prune's closure signature is impl Fn(&ThreadId) -> u64, not async/fallible, so boundaries are precomputed into a HashMap first."

patterns-established:
  - "New RunTracePort-consuming code should route through TraceRecord (never bare TraceEvent) exactly like the existing TraceSink/TraceEmitter contract, and any future backend joins the same contract_tests::run_all suite"

requirements-completed: [OBS-02]

coverage:
  - id: D1
    description: "RunTracePort (append/read/prune_thread), Send+Sync, #[non_exhaustive] RunTraceError with structured Backend/Serialization/UnsupportedSchemaVersion variants"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/run_trace_port.rs#trait_is_object_safe"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/run_trace_port.rs#mock_store_implements_trait"
        status: pass
    human_judgment: false
  - id: D2
    description: "One shared contract suite (append_then_read_round_trips, read_paginates_by_after_seq, read_of_unknown_thread_is_empty_not_error, append_is_idempotent_on_same_seq, records_are_scoped_by_thread, prune_thread_removes_only_older_supersteps, unsupported_schema_version_is_typed) runs identically against InMemoryRunTraceStore and SqliteRunTraceStore; PostgresRunTraceStore self-skips locally with a visible SKIP: line"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib run_trace (21 passed, 0 failed)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite,postgres --lib run_trace (31 passed incl. 9 Postgres self-skips printing SKIP:, 0 failed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Migration 006_create_run_traces_table.sql on both backends: append-only run_traces table, PRIMARY KEY (thread_id, seq), idx_run_traces_thread_seq and idx_run_traces_thread_superstep indexes"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql (PRIMARY KEY (thread_id, seq) present)"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql (TIMESTAMPTZ, JSONB, same PRIMARY KEY)"
        status: pass
    human_judgment: false
  - id: D4
    description: "WaypointRetentionService prunes run_traces with the same age/count bounds it applies to Waypoints when a RunTracePort is wired, and behaves exactly as before when it is not -- a failing trace prune never aborts Waypoint pruning"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/application/services/waypoint_retention.rs#prune_without_run_trace_port_is_unchanged"
        status: pass
      - kind: unit
        ref: "src/application/services/waypoint_retention.rs#prune_run_traces_uses_the_waypoint_bounds"
        status: pass
      - kind: unit
        ref: "src/application/services/waypoint_retention.rs#run_trace_prune_error_does_not_abort_waypoint_pruning"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::waypoint_retention (7 passed, 0 failed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "No SQL injection surface: every statement uses bound parameters in named const SQL blocks; run_trace/sqlite.rs contains zero format! calls"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "grep -c 'format!' crates/paladin-storage/src/run_trace/sqlite.rs == 0"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage/src/run_trace/sqlite.rs#thread_id_with_sql_metacharacter_round_trips_as_data"
        status: pass
    human_judgment: false
  - id: D6
    description: "Workspace stays green: cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings, cargo check --workspace --all-targets --all-features, the fault-injection integration suite, all pass with this plan's changes"
    verification:
      - kind: unit
        ref: "cargo fmt --all --check (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo clippy --workspace --all-targets -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo check --workspace --all-targets --all-features (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test --test waypoint_retention_fault_injection (3 passed, 0 failed)"
        status: pass
    human_judgment: false

# Metrics
duration: 34min
completed: 2026-09-08
status: complete
---

# Phase 28 Plan 04: Run Trace Persistence Summary

**`RunTracePort` (append/read/prune_thread) with in-memory, SQLite and Postgres adapters over migration `006`, one shared contract suite, and an additive `WaypointRetentionService::with_run_trace_port` join that prunes `run_traces` under the existing Waypoint retention bounds.**

## Performance

- **Duration:** ~34 min
- **Started:** 2026-09-08T22:46:00Z (base commit for this wave)
- **Completed:** 2026-09-08T23:19:29Z
- **Tasks:** 3 (1 checkpoint:decision auto-resolved, 1 tracer, 1 auto/tdd)
- **Files modified:** 12 (9 created)

## Checkpoint Status

Task 1 `checkpoint:decision` ("Confirm the one-way `run_traces` schema before the migration is written") — **option-a auto-selected by the orchestrator under `--auto`** (2026-09-08): proceed with D-17 verbatim (migration `006` on both backends, `run_traces (thread_id, seq, run_id NULL, superstep, at, schema_version, record, PRIMARY KEY (thread_id, seq))`, append-only). No stop, no `## CHECKPOINT REACHED` — execution proceeded directly to Task 2.

## Accomplishments

- New `crates/paladin-ports/src/output/run_trace_port.rs`: `RunTracePort` (`append`/`read`/`prune_thread`), `#[non_exhaustive]` `RunTraceError` (`Backend`, `Serialization`, `UnsupportedSchemaVersion`), module docs mirroring `waypoint_port.rs`'s "missing is empty, not an error" and "`ThreadId` is not an authorization boundary" framing, plus a new statement that trace persistence is best-effort while the Waypoint stays the durability truth.
- New `crates/paladin-storage/src/run_trace/{mod,in_memory,sqlite,postgres,contract_tests,retention}.rs`: `InMemoryRunTraceStore`, `SqliteRunTraceStore` (Tier 1), `PostgresRunTraceStore` (Tier 2, Docker-gated self-skip), all three sharing `contract_tests::run_all` plus a backend-specific `unsupported_schema_version_is_typed` call (needs the test-only `RawSchemaVersionWriter` trait). `run_trace::superstep_of()` derives each row's pruning bucket from whichever `TraceEvent` variants carry a `superstep`.
- New migrations `006_create_run_traces_table.sql` on SQLite and Postgres: identical logical schema, `TEXT`/`TIMESTAMPTZ` and `TEXT`/`JSONB` type mapping split, same `PRIMARY KEY (thread_id, seq)` and two index names.
- `src/application/services/waypoint_retention.rs`: `WaypointRetentionService` gains an optional `run_trace_port: Option<Arc<dyn RunTracePort>>` field, a `with_run_trace_port` builder, and a `last_run_traces_removed()` getter. `prune()` applies the Waypoint bounds first (unchanged), then -- when a `RunTracePort` is wired -- prunes `run_traces` per thread using that thread's surviving-Waypoints' minimum superstep as the boundary, logging (never propagating) any trace-prune failure.

## Task Commits

1. **Task 1: checkpoint:decision (auto-resolved, no commit — resolution recorded in Task 2's commit message)**
2. **Task 2: `RunTracePort`, in-memory and SQLite adapters, migration 006, contract suite** - `19860f67` (feat)
3. **Task 3: Postgres adapter (Tier 2) and the retention join** - `bb801e8a` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `crates/paladin-ports/src/output/run_trace_port.rs` - New: `RunTracePort`, `RunTraceError`
- `crates/paladin-ports/src/output/mod.rs` - `pub mod run_trace_port;`
- `crates/paladin-storage/src/run_trace/mod.rs` - New: module wiring, `superstep_of()`
- `crates/paladin-storage/src/run_trace/in_memory.rs` - New: `InMemoryRunTraceStore`
- `crates/paladin-storage/src/run_trace/sqlite.rs` - New: `SqliteRunTraceStore`
- `crates/paladin-storage/src/run_trace/postgres.rs` - New: `PostgresRunTraceStore`
- `crates/paladin-storage/src/run_trace/contract_tests.rs` - New: shared contract suite, `RawSchemaVersionWriter`
- `crates/paladin-storage/src/run_trace/retention.rs` - New: `prune()` sibling
- `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql` - New: `run_traces` table (SQLite)
- `crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql` - New: `run_traces` table (Postgres)
- `crates/paladin-storage/src/lib.rs` - `pub mod run_trace;`
- `src/application/services/waypoint_retention.rs` - `with_run_trace_port`, `last_run_traces_removed()`, trace-prune join in `prune()`

## Decisions Made

See `key-decisions` in frontmatter. The most consequential: keeping `WaypointRetentionService::prune()`'s return type as `Result<PruneReport, WaypointError>` unchanged rather than following the plan text's "extend `PruneReport` additively" instruction literally, because `PruneReport` is defined in a file this plan is not scoped to touch and this crate's own public API is semver-checked (see Deviations below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 4-adjacent, resolved autonomously under `--auto`] `PruneReport` cannot be extended within this plan's `files_modified` scope**
- **Found during:** Task 3 (wiring `WaypointRetentionService`)
- **Issue:** The plan's action text says "Extend `PruneReport` additively with the trace-row count," but `PruneReport` is defined in `crates/paladin-storage/src/waypoint/retention.rs`, which is **not** in this plan's `files_modified` list. Changing `WaypointRetentionService::prune()`'s return type to a new wrapper struct is a public-API-breaking change to the `paladin` library crate (`paladin-ai`'s `[lib] name = "paladin"`, semver-checked in CI), which this project's own established precedent requires closing out with a `Cargo.toml` suppression comment + a `MIGRATION.md` §9.2 row + a `.cargo/semver-checks-allowlist.toml` entry, all in the same commit — and `project_execution_rules` explicitly states "MIGRATION.md is plan 28-17's — do not edit it."
- **Fix:** Kept `prune()`'s signature and return type byte-identical (`Result<PruneReport, WaypointError>`). Added a purely additive `WaypointRetentionService::last_run_traces_removed() -> u64` getter (backed by an internal `AtomicU64`, since `prune(&self)` takes a shared reference) that reports the most recent trace-row removal count separately. This satisfies the underlying requirement (the trace-removed count is observable right after a `prune()` call) with zero semver impact and zero out-of-scope file edits.
- **Files modified:** `src/application/services/waypoint_retention.rs` (already in `files_modified`)
- **Verification:** `cargo test -p paladin-ai --lib services::waypoint_retention` (7 passed), including the three new D-17 behavior tests (`prune_without_run_trace_port_is_unchanged`, `prune_run_traces_uses_the_waypoint_bounds`, `run_trace_prune_error_does_not_abort_waypoint_pruning`).
- **Committed in:** `bb801e8a` (Task 3 commit)

**2. [Rule 1 - Bug, caught by clippy] `cloned_ref_to_slice_refs` lint on three test-only single-element `&[x.clone()]` slices**
- **Found during:** Task 2/3 workspace clippy pass
- **Issue:** `port.append(&[record.clone()])` (and two backend-specific analogues) triggered `clippy::cloned_ref_to_slice_refs` (implied by `-D warnings`) — an unnecessary clone to build a one-element slice from a reference.
- **Fix:** Replaced with `std::slice::from_ref(&record)` where `record` is still used afterward, or a plain move (`&[record]`) where it is not.
- **Files modified:** `crates/paladin-storage/src/run_trace/contract_tests.rs`, `crates/paladin-storage/src/run_trace/sqlite.rs`, `crates/paladin-storage/src/run_trace/postgres.rs`
- **Verification:** `cargo clippy --all-targets --all-features -p paladin-ports -p paladin-storage -p paladin-ai -- -D warnings` exits 0.
- **Committed in:** `19860f67` and `bb801e8a` (the fixes landed in the same commits as the code they corrected, before either was ever committed uncorrected)

**3. [Rule 3 - Blocking, atomic-commit ordering] `run_trace/mod.rs` temporarily trimmed for Task 2's commit**
- **Found during:** staging Task 2's commit
- **Issue:** The plan lists `crates/paladin-storage/src/run_trace/mod.rs` under both Task 2's and Task 3's `files_modified`. Writing the file once with all four submodule declarations (`in_memory`, `contract_tests`, `sqlite`, `postgres`, `retention`) would have made Task 2's commit reference `postgres.rs`/`retention.rs`, files that don't exist until Task 3 -- breaking `cargo check` at the Task 2 commit.
- **Fix:** Committed Task 2 with `mod.rs` declaring only `in_memory`, `contract_tests` and (feature-gated) `sqlite`; re-added the `postgres`/`retention` declarations in a second edit staged as part of Task 3's commit.
- **Files modified:** `crates/paladin-storage/src/run_trace/mod.rs`
- **Verification:** `cargo check -p paladin-storage --features sqlite --lib` passes at the Task 2 commit; the full workspace check passes at the Task 3 commit.
- **Committed in:** `19860f67` (Task 2, trimmed version) and `bb801e8a` (Task 3, full version)

---

**Total deviations:** 3 auto-fixed (1 architectural-adjacent scope resolution, 1 lint bug, 1 blocking atomic-commit ordering fix). None expand behavior beyond what OBS-02 and this plan's own acceptance criteria require.
**Impact on plan:** The `PruneReport` deviation is the only one with externally-visible shape: `WaypointRetentionService::prune()`'s return type is unchanged from before this plan, and the new trace-removed count is reachable via `last_run_traces_removed()` instead of via the report itself. No other plan or in-tree caller was affected (there are no other callers of `WaypointRetentionService::prune()` in this tree today).

## Issues Encountered

None beyond the deviations documented above.

## User Setup Required

None - no external service configuration required. The Postgres Tier 2 suite requires `docker compose -f docker/docker-compose.test.yml up -d postgres-test` to run for real in CI's `postgres-integration` job; it self-skips locally with a visible `SKIP:` line (Docker unavailable in this devcontainer, per 24-CONTEXT D-28) and was never marked as passed locally.

## Next Phase Readiness

- `RunTracePort` + its three adapters give 28-11 (SSE replay) and 28-10 (execution overlay) a durable, paginated, schema-versioned, thread-scoped store to read from once a persisting `TraceSink` is wired -- that sink itself is a later plan's job; this plan only builds the storage half.
- `WaypointRetentionService::with_run_trace_port` is additive and opt-in: any code constructing the service today via `new()` alone is completely unaffected.
- `PruneReport`'s own shape is untouched; a future plan (likely 28-17, which owns `MIGRATION.md`) that wants the trace-removed count folded directly into `PruneReport` will need the full breaking-change bookkeeping (Cargo.toml suppression + MIGRATION.md §9.2 row + semver-checks-allowlist entry) this plan deliberately avoided.
- No blockers for 28-05 through 28-17.

## Self-Check: PASSED

- FOUND: `crates/paladin-ports/src/output/run_trace_port.rs`
- FOUND: `crates/paladin-storage/src/run_trace/mod.rs`
- FOUND: `crates/paladin-storage/src/run_trace/postgres.rs`
- FOUND: `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql`
- FOUND: `crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql`
- FOUND: `.planning/phases/28-observability-tooling/28-04-SUMMARY.md`
- FOUND commit: `19860f67`
- FOUND commit: `bb801e8a`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-08*
