---
phase: 22-battlefield-state-superstep-engine
plan: 03
subsystem: infra
tags: [rust, hexagonal-architecture, waypoint, contract-testing, tdd, sqlite, postgres]

# Dependency graph
requires:
  - phase: 22-01
    provides: "Waypoint/ThreadId/WaypointId/GraphFingerprint types, tracer-level WaypointPort trait, and InMemoryWaypointStore adapter"
provides:
  - "Fully documented WaypointPort contract (save upsert semantics, latest/get not-found, history/list_threads ordering+pagination, delete_thread count) with rustdoc stating ThreadId is not an authorization boundary"
  - "ThreadId validation with typed errors (Empty/TooLong/ContainsWhitespace) and an explicitly documented byte-length unit"
  - "Waypoint::new_root/new_child lineage constructors and WaypointId::generate()"
  - "Shared, named-per-clause WaypointPort contract suite (paladin_storage::waypoint::contract_tests, D-09) — 13 pub async fn contract functions plus run_all — green against InMemoryWaypointStore and callable unchanged by SQLite/Postgres backends in Plan 22-06"
  - "Corrected InMemoryWaypointStore semantics: save() upserts by waypoint_id; history()/latest() compute ordering explicitly from (created_at, superstep) rather than storage position"
affects: [22-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Generic async contract-test functions taking &dyn Port, invoked from per-backend #[tokio::test]s (D-09) for clearer failure diagnostics than a declarative macro"
    - "Explicit sort/max by (created_at, superstep) for ordering contracts, never inferred from in-memory storage/insertion position"
    - "Port rustdoc explicitly disclaiming an authorization boundary at the identifier level, so a later HTTP-exposure epic cannot inherit the gap silently"

key-files:
  created:
    - crates/paladin-storage/src/waypoint/contract_tests.rs
  modified:
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-ports/src/output/waypoint_port.rs
    - crates/paladin-storage/src/waypoint/in_memory.rs
    - crates/paladin-storage/src/waypoint/mod.rs

key-decisions:
  - "WaypointPort::save documented and implemented as an upsert on re-save of an existing waypoint_id (replace the row in place), not a rejection — chosen so callers can safely retry a save after a transient failure without first checking whether it partially landed; every backend must match this"
  - "InMemoryWaypointStore's history()/latest() ordering is computed by explicit sort/max over (created_at desc, superstep desc), not inferred from Vec insertion or upsert position — required once save() upserts in place, since insertion order no longer tracks recency"
  - "WaypointId::generate() added as a documented alias of the existing WaypointId::new() (kept, not removed) rather than a rename, to avoid touching the tracer plan's existing call sites"
  - "Waypoint::new_root/new_child take individual fields (not a builder) with #[allow(clippy::too_many_arguments)], matching the plan's 'parent Waypoint or its id' latitude by taking the parent Waypoint directly for new_child so thread_id is copied automatically"

requirements-completed: [ENG-03, ENG-05]

coverage:
  - id: D1
    description: "WaypointPort contract fully documented (Purpose/Hexagonal Architecture Context/Thread Safety/Error Handling structure) with per-method not-found/ordering/upsert contracts, and module rustdoc stating the Citadel-separation rationale and the ThreadId-is-not-an-authorization-boundary statement"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/waypoint_port.rs#tests (trait_is_object_safe, mock_store_implements_trait, waypoint_summary_round_trips_through_serde_json, thread_summary_round_trips_through_serde_json)"
        status: pass
      - kind: unit
        ref: "cargo test --doc -p paladin-ai-core -p paladin-ports"
        status: pass
    human_judgment: false
  - id: D2
    description: "ThreadId validates non-empty, whitespace-free (space/tab/newline/NBSP), and at-most-256-bytes with a stated unit (UTF-8 bytes, not Unicode scalar count), proven at a multi-byte boundary"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#tests (thread_id_rejects_empty, thread_id_rejects_whitespace, thread_id_rejects_tab, thread_id_rejects_newline, thread_id_rejects_non_breaking_space, thread_id_accepts_exactly_max_len, thread_id_multibyte_string_measured_in_bytes_at_boundary)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Waypoint::new_root/new_child construct lineage correctly (root has no parent; child's parent_waypoint_id equals the parent's id; WaypointId::generate() never collides and sorts in creation order)"
    requirement: "ENG-03"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#tests (new_root_has_no_parent, new_child_points_at_parent_and_inherits_thread, waypoint_id_generate_never_collides_and_sorts_in_creation_order)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Shared generic WaypointPort contract suite (13 named pub async fn plus run_all) green against InMemoryWaypointStore, covering save/latest/get not-found semantics, history newest-first ordering with same-created_at superstep tiebreak, limit/before pagination (including limit zero and no-overlap-no-gap), list_threads empty-then-populated ordering, delete_thread count and unknown-thread zero, upsert-on-resave, and parent-lineage round trip"
    requirement: "ENG-05"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/in_memory.rs#tests (14 tests: one #[tokio::test] per contract_tests:: function plus run_all_contract_functions_smoke_aggregate)"
        status: pass
    human_judgment: false
  - id: D5
    description: "InMemoryWaypointStore corrected to match the documented contract: save() upserts in place on an existing waypoint_id instead of appending a duplicate; history()/latest() ordering is computed explicitly rather than relying on insertion position"
    requirement: "ENG-05"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/in_memory.rs#tests::resave_existing_waypoint_id_upserts, ::history_same_created_at_tiebreaks_by_descending_superstep_stably"
        status: pass
    human_judgment: false

# Metrics
duration: ~35min
completed: 2026-09-01
status: complete
---

# Phase 22 Plan 03: WaypointPort Contract & Shared Backend Test Suite Summary

**Fully specified `WaypointPort` (upsert `save`, `Ok(None)`/`Ok(0)` not-found semantics, documented ordering and pagination, explicit non-authorization statement) plus a 13-function generic contract suite (`paladin_storage::waypoint::contract_tests`) green against `InMemoryWaypointStore` and ready for the SQLite/Postgres backends in Plan 22-06 to run unchanged.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2 completed
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments

- `WaypointPort`'s six methods each now carry a rustdoc block stating their not-found, ordering, or upsert contract; the module doc adds a "Thread Safety" section and an explicit statement that `ThreadId` is not an authorization boundary, satisfying the T-22-07 threat disposition.
- `ThreadId::new` rejects empty, whitespace-bearing (space/tab/newline/NBSP), and over-length input with three distinct typed errors; the length unit (UTF-8 bytes) is stated explicitly in rustdoc and proven at a multi-byte boundary (128×`é` = 256 bytes accepted, 129×`é` = 258 bytes rejected with the byte count in the error).
- `Waypoint::new_root`/`new_child` and `WaypointId::generate()` added so lineage cannot be constructed incorrectly by hand; `WaypointSummary`/`ThreadSummary` now derive `Serialize`/`Deserialize` with round-trip tests.
- `paladin_storage::waypoint::contract_tests` (D-09): 13 named `pub async fn` contract functions plus a `run_all` aggregator and a `sample_waypoint`/`sample_waypoint_at` fixture builder — every function takes `&dyn WaypointPort` so SQLite and Postgres backends (Plan 22-06) can call them unchanged.
- Found and fixed two real contract violations in the tracer-level `InMemoryWaypointStore` while writing the suite (see Deviations): `save` was append-only instead of upsert, and `history`/`latest` inferred ordering from storage position instead of the documented `(created_at, superstep)` sort.

## Task Commits

Each task was committed atomically:

1. **Task 1: Complete the WaypointPort contract — ordering, pagination, not-found and ThreadId validation** - `cb1fa044` (feat)
2. **Task 2: Shared generic contract suite, green against InMemoryWaypointStore** - `a7e1e0f7` (feat)

**Plan metadata:** committed alongside this SUMMARY (docs: complete plan).

## Files Created/Modified

- `crates/paladin-storage/src/waypoint/contract_tests.rs` - New: 13 named contract functions, `run_all`, `sample_waypoint`/`sample_waypoint_at` fixtures
- `crates/paladin-core/src/platform/container/waypoint.rs` - `ThreadId` doc/tests for the byte-length unit and whitespace classes; `WaypointId::generate()`; `Waypoint::new_root`/`new_child`
- `crates/paladin-ports/src/output/waypoint_port.rs` - Full per-method rustdoc contracts; module-level authorization statement; `WaypointSummary`/`ThreadSummary` now `Serialize`/`Deserialize`
- `crates/paladin-storage/src/waypoint/in_memory.rs` - `save` upsert semantics; `history`/`latest` explicit ordering; tests rewired to call `contract_tests::` functions
- `crates/paladin-storage/src/waypoint/mod.rs` - Registered `pub mod contract_tests`

## Decisions Made

- `save`'s re-save-of-existing-`waypoint_id` behavior is documented and implemented as an **upsert** (replace in place), not a rejection, so backends can be retried safely after a transient failure. All three backends (`InMemoryWaypointStore` now; `SqliteWaypointStore`/`PostgresWaypointStore` in Plan 22-06) must match this.
- `WaypointId::generate()` was added as an alias of the existing `WaypointId::new()` rather than a rename, so the tracer plan's (22-01) existing call sites are untouched.
- `InMemoryWaypointStore`'s `history`/`latest` now compute ordering by explicit `sort`/`max_by` over `(created_at, superstep)` rather than relying on `Vec` insertion or in-place-upsert position — a hard requirement once `save` upserts, since insertion order stops tracking recency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `InMemoryWaypointStore::save` was append-only, not upsert**
- **Found during:** Task 2, while writing `resave_existing_waypoint_id_upserts`
- **Issue:** The tracer-level `save` always pushed a new entry onto the thread's `Vec<Waypoint>`, so re-saving an existing `waypoint_id` created a duplicate row rather than replacing it — contradicting the upsert contract Task 1 documents on `WaypointPort::save`.
- **Fix:** `save` now searches the thread's entries for a matching `waypoint_id` and replaces it in place; only a genuinely new `waypoint_id` is appended.
- **Files modified:** `crates/paladin-storage/src/waypoint/in_memory.rs`
- **Verification:** `resave_existing_waypoint_id_upserts` contract test passes; `history` after a re-save reports exactly one entry, not two.
- **Committed in:** `a7e1e0f7` (Task 2 commit)

**2. [Rule 1 - Bug] `history`/`latest` inferred ordering from storage position instead of the documented sort**
- **Found during:** Task 2, while writing `history_same_created_at_tiebreaks_by_descending_superstep_stably`
- **Issue:** `history` reversed the `Vec`'s insertion order to present "newest-first," and `latest` returned `.last()` — both silently correct only when saves happen in strictly increasing chronological order with no upserts. Once `save` upserts in place (fix #1), and for the explicit same-`created_at` tiebreak test, insertion position no longer determines the documented ordering.
- **Fix:** `history` now explicitly sorts by `(created_at desc, superstep desc)` before applying `before`/`limit`; `latest` now takes the `max_by` element under the same comparator.
- **Files modified:** `crates/paladin-storage/src/waypoint/in_memory.rs`
- **Verification:** `history_same_created_at_tiebreaks_by_descending_superstep_stably` and `history_limit_and_before_paginate_with_no_overlap_or_gap` contract tests pass, including two calls returning byte-identical order.
- **Committed in:** `a7e1e0f7` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - Bug, both found while writing the contract suite the plan asked for, both fixed in the store rather than by weakening the assertion, per the plan's own instruction).
**Impact on plan:** Both fixes are exactly the "fix any contract violation in the store" work the plan's Task 2 `<action>` anticipated ("expect the tracer-level store to be missing the ordering tiebreak, the before cursor and the limit of zero case"). No scope creep — no new files, no new dependencies, no public API changes beyond what Task 1/2 already specified.

## Issues Encountered

None beyond the two deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `paladin_storage::waypoint::contract_tests::run_all` and its 13 named functions are ready for Plan 22-06's `SqliteWaypointStore`/`PostgresWaypointStore` to call unchanged, per ENG-FR-17's "identical suite across backends" requirement.
- The `WaypointPort` rustdoc's non-authorization statement is in place for the later HTTP-exposure epic to read before adding network access — it is documented but no authorization layer exists yet, by design (out of this phase's scope).
- No blockers. The tracer's `war_engine_tracer` integration test suite remains green after both the `save`/`history`/`latest` corrections and the `paladin-core`/`paladin-ports` doc/type additions.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-01*

## Self-Check: PASSED

- FOUND: crates/paladin-storage/src/waypoint/contract_tests.rs
- FOUND: crates/paladin-core/src/platform/container/waypoint.rs
- FOUND: crates/paladin-ports/src/output/waypoint_port.rs
- FOUND: crates/paladin-storage/src/waypoint/in_memory.rs
- FOUND: crates/paladin-storage/src/waypoint/mod.rs
- FOUND: cb1fa044 (git log --oneline)
- FOUND: a7e1e0f7 (git log --oneline)
