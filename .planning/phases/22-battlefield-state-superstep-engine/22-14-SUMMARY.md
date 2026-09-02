---
phase: 22-battlefield-state-superstep-engine
plan: 14
subsystem: database
tags: [rust, waypoint-retention, prune, fault-injection, sqlite, hexagonal-architecture]

# Dependency graph
requires:
  - phase: 22-battlefield-state-superstep-engine
    provides: "WaypointPort::delete_waypoint and WaypointPort::prune_thread (Plan 22-13) -- the keep-set primitive this plan's rewrite depends on"
provides:
  - "crates/paladin-storage/src/waypoint/retention.rs rewritten: prune takes the protected set as a borrowed-fn argument and performs every deletion through WaypointPort::prune_thread; the delete-then-resave sequence is fully removed"
  - "src/application/services/waypoint_retention.rs (new): the single application-layer definition of 'protected' (latest + AwaitingInput) and WaypointRetentionService driving the routine with it"
  - "tests/integration/waypoint_retention_fault_injection_test.rs: fault-injection sweep, real-engine resume-after-interrupted-prune, and SQLite cancellation acceptance test"
  - "ENG-FR-18 extended with the monotone/idempotent/crash-safe invariant as requirement text"
affects: ["Phase 24 (Pause/Resume, History & Graceful Shutdown) -- extends protected_waypoints with the two named seams (unresolved-Parley reference, active fork lineage) when Parley pauses and Chronicle forking land"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Policy-as-argument: the storage-layer prune routine is handed the protected set via a borrowed `&dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>` rather than deriving it, keeping the adapter free of any notion of what 'protected' means (X-01)"
    - "Recording port double (in-crate test-only, not shipped) to assert on the exact keep-set and call count/shape a routine hands to a port, rather than inferring behavior from after-state alone"
    - "Fault-injection decorator kept local to the acceptance test file (not shipped adapter code) -- delegates every WaypointPort method except the one under test, and deliberately omits overriding a provided trait method so the default composition is what's exercised under fault"

key-files:
  created:
    - src/application/services/waypoint_retention.rs
    - tests/integration/waypoint_retention_fault_injection_test.rs
  modified:
    - crates/paladin-storage/src/waypoint/retention.rs
    - crates/paladin-storage/src/waypoint/mod.rs
    - src/application/services/mod.rs
    - src/config/waypoint_retention.rs
    - Cargo.toml
    - .project/v0.10.0/01-battlefield-state-and-execution-engine.md
    - MIGRATION.md

key-decisions:
  - "protected is passed as `&dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>` (a borrowed function) rather than a new trait -- the plan left the choice open ('a borrowed function or a small trait object, whichever reads better') and a function reads more directly at both the storage-layer test call sites and the one real call site in WaypointRetentionService"
  - "WaypointRetentionService::prune short-circuits to an empty PruneReport without touching the port when config.enabled is false -- not explicitly required by the plan's task text, but consistent with WaypointRetentionConfig's own documented disabled-by-default contract (X-09) and the existing both-bounds-none no-op behavior; a service that ignored its own config's `enabled` flag would be a Rule 2 gap"
  - "Task 3's MIGRATION.md fix (§9.5 WaypointRetentionConfig entry) was not in the plan's `files_modified` list but was explicitly required by Task 3's action text ('repoint them at the application-layer service') -- the entry described the removed own-invariant-enforcement mechanism and the pre-22-13 absence of a per-waypoint delete primitive, both now false"
  - "Coverage measurement (Task 3) was attempted but not completed: scripts/coverage.sh hard-fails without reachable Redis/MinIO, and no Docker daemon exists in this sandbox -- the same limitation already recorded for the Postgres Tier 2 suite (gap G-22-1). Not reclassified as passing; recorded as not measured"

requirements-completed: [ENG-05]

coverage:
  - id: D1
    description: "prune (paladin-storage) rewritten to take the protected set as an argument and perform every deletion through WaypointPort::prune_thread; the whole-thread delete-then-resave sequence is fully removed"
    requirement: "ENG-05"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/waypoint/retention.rs#tests (11 tests: 8 adapted prior behavioral guarantees + 3 new keep-set/call-shape assertions via a recording port double)"
        status: pass
    human_judgment: false
  - id: D2
    description: "src/application/services/waypoint_retention.rs holds the single definition of protected (latest + AwaitingInput) with the two future seams (unresolved-Parley reference, active fork lineage) named and owned by Phase 24; WaypointRetentionService drives the routine with configured bounds and returns the routine's report unchanged"
    requirement: "ENG-05"
    verification:
      - kind: unit
        ref: "src/application/services/waypoint_retention.rs#tests (4 tests: protected-set fixture over both/neither/each-alone, both-latest-and-awaiting single case, bounds pass-through, disabled no-op)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Fault-injection acceptance test proves the gap's core claim: an injected backend failure at every delete position leaves protected Waypoints loadable and superset-of-keep-set behind, a real engine resumes an interrupted-prune thread to the control run's final Battlefield, re-running converges, and an aborted prune against the transactional SQLite backend leaves the keep-set intact at every sampled abort delay"
    requirement: "ENG-05"
    verification:
      - kind: integration
        ref: "tests/integration/waypoint_retention_fault_injection_test.rs (3 tests: part_a_fault_injection_sweep_leaves_protected_waypoints_and_converges, part_b_resume_after_interrupted_prune_matches_control_run, part_c_aborted_prune_against_sqlite_leaves_keep_set_intact)"
        status: pass
    human_judgment: false
  - id: D4
    description: "ENG-FR-18 requirement text extended with the monotone/idempotent/crash-safe invariant, not left only in rustdoc"
    requirement: "ENG-05"
    verification:
      - kind: other
        ref: "grep -c monotone .project/v0.10.0/01-battlefield-state-and-execution-engine.md -> 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full workspace gate green: cargo test --workspace, cargo fmt --check, cargo clippy --workspace --all-targets -D warnings, cargo build --workspace --all-features, make security; pre-release compatibility classification confirmed from the repository"
    verification:
      - kind: other
        ref: "cargo test --workspace; cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo build --workspace --all-features; make security"
        status: pass
    human_judgment: false
  - id: D6
    description: "Workspace coverage measurement against the 82% ADR-0006 floor"
    verification: []
    human_judgment: true
    rationale: "scripts/coverage.sh hard-fails without reachable Redis/MinIO (make services-up) and no Docker daemon is available in this sandbox -- the same limitation already recorded for the Postgres Tier 2 contract suite (gap G-22-1). Not measured this plan; a human (or CI, which does have the services) should confirm the workspace figure has not fallen below floor before treating D6 as proven."

# Metrics
duration: ~20min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 14: Rewrite retention on the keep-set primitive Summary

**Retention's prune routine now takes its protected set as an argument instead of deriving it, deletes exclusively through `WaypointPort::prune_thread`, and an executable fault-injection/cancellation test proves an interrupted prune never costs a thread its latest or `AwaitingInput` checkpoint.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-09-02T18:27:00Z (session start)
- **Completed:** 2026-09-02T18:47:30Z
- **Tasks:** 3
- **Files modified:** 8 (6 planned + 1 test-registration line in Cargo.toml + 1 required doc fix, MIGRATION.md)

## Accomplishments
- `crates/paladin-storage/src/waypoint/retention.rs::prune` rewritten around two changes: it now takes `protected: &dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>` instead of hard-coding the latest/`AwaitingInput` exclusions itself, and every deletion goes through exactly one `WaypointPort::prune_thread` call per thread with something to remove -- the enumeration-then-whole-thread-delete-then-resave sequence is gone entirely, along with the module doc's now-false "no per-waypoint primitive existed" rationale.
- All 8 prior behavioral tests (both-bounds-none no-op, single-waypoint-thread untouched, count bound keeps newest N, age bound protects the newest, age bound removes old non-latest, `AwaitingInput` survives both bounds, idempotent second run, per-thread report counts) adapted to the new signature and still pass unchanged in substance, plus 3 new tests using an in-module `RecordingStore` test double: the keep-set handed to the port always contains the latest and `AwaitingInput` ids, exactly one `prune_thread` call happens per thread with something to remove (and none for an untouched thread), and `delete_thread` is never called during a prune.
- `src/application/services/waypoint_retention.rs` (new) holds the one definition of "protected" in the system -- a thread's latest Waypoint plus every `AwaitingInput` Waypoint -- and `WaypointRetentionService`, which owns an `Arc<dyn WaypointPort>` and `WaypointRetentionConfig` and drives the routine with that definition, passing the configured bounds through unchanged and returning the routine's report. The two not-yet-existing protected classes (a Waypoint referenced by an unresolved Parley; a Waypoint pinned by an active fork lineage) are named in rustdoc as seams owned by Phase 24 (Pause/Resume, History & Graceful Shutdown), not fabricated as stub fields.
- `tests/integration/waypoint_retention_fault_injection_test.rs` (new, registered as its own `[[test]]` target in `Cargo.toml`) proves the gap's actual claim in three parts:
  - **Part A** sweeps an injected `delete_waypoint` failure across every position the fixture can produce (against the port's *provided* `prune_thread` default, since the decorator deliberately does not override it): the latest and `AwaitingInput` Waypoints survive byte-identical at every position, the remainder is always a superset of a control run's keep-set, a recovery run (fault disabled) converges to exactly the keep-set, and a third run removes nothing further.
  - **Part B** runs a 6-node Function-only chain to completion on a real `WarEngine`, interrupts a prune against its backend, and proves a freshly constructed engine resumes the thread to the same final `Battlefield` as an uninterrupted control run -- tying the fix back to the program's core crash-resume invariant.
  - **Part C** spawns and aborts a real `prune_thread` call against a transactional `SqliteWaypointStore` at a swept range of short delays (0-25ms), asserting every keep-set id still loads regardless of whether the abort landed before or after the transaction committed, then runs to completion and confirms exact convergence.
- ENG-FR-18 in `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` extended with the monotone/idempotent/crash-safe invariant as requirement text, and `MIGRATION.md`'s §9.5 entry repointed at `WaypointRetentionService` as the entry point (it previously described the now-removed mechanism).
- Full workspace gate green: `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace --all-features`, `make security`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite prune on the keep-set primitive and move the protected-set definition up a layer** - `3696844` (feat)
2. **Task 2: Fault-injection and cancellation acceptance test** - `d064b64` (test)
3. **Task 3: Workspace green and the pre-release classification recorded** - `f1cf782` (docs)

_No RED/GREEN/REFACTOR split despite `tdd="true"` on Tasks 1-2: each task's failing-first behavior was the existing test suite adapted to the new signature (Task 1) and the new acceptance test itself (Task 2), verified failing against the not-yet-rewritten/not-yet-fixed code before the implementation landed, then committed together with the implementation in the same commit -- matching the pattern established in 22-13's SUMMARY for this same module family._

## Files Created/Modified
- `crates/paladin-storage/src/waypoint/retention.rs` - `prune` rewritten to take the protected set as an argument and delete exclusively through `WaypointPort::prune_thread`; module doc rewritten; 8 adapted tests + 3 new tests via a `RecordingStore` double.
- `crates/paladin-storage/src/waypoint/mod.rs` - no functional change; `retention` module doc comment left as-is (already accurate).
- `src/application/services/waypoint_retention.rs` (new) - `protected_waypoints` (the one definition, with the two future seams named) and `WaypointRetentionService`.
- `src/application/services/mod.rs` - declares `pub mod waypoint_retention;`.
- `src/config/waypoint_retention.rs` - doc comment repointed at `WaypointRetentionService` as the entry point.
- `Cargo.toml` - registered `waypoint_retention_fault_injection` as a standalone `[[test]]` target, following the `e2e_crash_resume` pattern.
- `tests/integration/waypoint_retention_fault_injection_test.rs` (new) - the three-part fault-injection and cancellation acceptance test.
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` - ENG-FR-18 extended with the monotone/idempotent/crash-safe invariant.
- `MIGRATION.md` - §9.5 `WaypointRetentionConfig` entry repointed at the service and the actual keep-set mechanism.

## Decisions Made
- `protected` is a borrowed function (`&dyn Fn(&ThreadId, &[WaypointSummary]) -> HashSet<WaypointId>`), not a new trait -- reads directly at every call site and avoids an extra trait definition for what is, in every current use, a pure computation over already-fetched history.
- `WaypointRetentionService::prune` treats `config.enabled == false` as a no-op that never touches the port, mirroring the existing both-bounds-`None` no-op and `WaypointRetentionConfig`'s own disabled-by-default doc contract (X-09) -- not explicitly spelled out in the plan's task text, applied as a Rule 2 (missing critical functionality) judgment call rather than left for a future plan to discover as a gap.
- `MIGRATION.md`'s stale §9.5 description of the pre-22-13/pre-22-14 mechanism was fixed even though `MIGRATION.md` was not in the plan's `files_modified` list, because Task 3's action text explicitly required searching shipped documentation for exactly this kind of stale claim.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `WaypointRetentionService::prune` respects `config.enabled`**
- **Found during:** Task 1 (application-layer service)
- **Issue:** The plan's task text for the service didn't explicitly mention checking `enabled`; a service that ignored it would silently prune even when an operator has retention turned off, contradicting `WaypointRetentionConfig`'s own documented disabled-by-default contract.
- **Fix:** `prune()` returns an empty `PruneReport` without touching the port when `!config.enabled`.
- **Files modified:** `src/application/services/waypoint_retention.rs`
- **Verification:** `service_disabled_is_a_no_op_and_does_not_touch_the_port` test.
- **Committed in:** `3696844` (Task 1 commit)

**2. [Rule 2 - Missing Critical] Repointed `MIGRATION.md`'s stale mechanism description**
- **Found during:** Task 3 (workspace green / documentation search)
- **Issue:** `MIGRATION.md` §9.5 described the routine as enforcing the latest/`AwaitingInput` exclusions "as invariants of the routine itself" and as composing only `history`/`get`/`delete_thread`/`save` with no per-waypoint delete primitive -- both now false after Plans 22-13/22-14.
- **Fix:** Repointed the entry at `WaypointRetentionService` as the entry point and described the actual keep-set-via-`prune_thread` mechanism.
- **Files modified:** `MIGRATION.md`
- **Verification:** Manual re-read against the current `retention.rs`/`waypoint_retention.rs` implementation; grep confirmed no other shipped doc references the old mechanism.
- **Committed in:** `f1cf782` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 2 - missing critical functionality)
**Impact on plan:** Both are narrow correctness/documentation-accuracy fixes directly implied by the plan's own task text and the existing config's documented contract. No scope creep.

## Issues Encountered
- Coverage measurement (Task 3's "confirm workspace coverage has not fallen below the project floor") could not be completed: `scripts/coverage.sh` hard-fails when Redis/MinIO are unreachable, and no Docker daemon is available in this sandbox to run `make services-up`. This is the same limitation already documented for the Postgres Tier 2 contract suite (gap G-22-1) and is not new to this plan. Recorded as `human_judgment: true` (D6) rather than silently asserting the floor holds.

## User Setup Required
None - no external service configuration required. (Coverage measurement requires Redis/MinIO reachable via `make services-up`, which CI provides; verify there if a coverage figure is needed before shipping.)

## Next Phase Readiness
- Gap G-22-2 (blocker) is now fully closed: the storage-adapter half (Plan 22-13's `prune_thread` primitive) and the retention-routine/acceptance-test half (this plan) are both landed and proven.
- `WaypointRetentionService` is the stable entry point any future scheduler wiring (Doc 06/Platform API, Phase 27) should call rather than reaching into `paladin_storage::waypoint::retention::prune` directly.
- Phase 24 (Pause/Resume, History & Graceful Shutdown) has a named, rustdoc'd extension point (`protected_waypoints`) for the two protected classes it introduces -- no rediscovery needed.
- No blockers for subsequent phase-22 work.

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
