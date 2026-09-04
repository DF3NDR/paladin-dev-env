---
phase: 22-battlefield-state-superstep-engine
plan: 10
subsystem: infra
tags: [rust, criterion, benchmark, sqlite, memory, performance, nfr]

# Dependency graph
requires:
  - phase: 22-06
    provides: "SqliteWaypointStore, PostgresWaypointStore and InMemoryWaypointStore, all passing the identical shared WaypointPort contract suite"
  - phase: 22-08
    provides: "NodeSpec::Paladin execution, complete WarEngine::resume, and the public WarEngine::start/resume API surface this plan's harnesses drive"
provides:
  - "benches/engine_benchmarks.rs: a criterion bench measuring SqliteWaypointStore::save latency at three Battlefield payload sizes (1 KiB, 512 KiB, just under 1 MiB) against the ENG-NFR-01 target"
  - "benches/engine_benchmarks.rs: a second criterion group measuring WarEngine::start per-superstep wall-clock cost at two Vanguard widths (1, 8) against InMemoryWaypointStore, isolating engine overhead from persistence cost"
  - "examples/war_engine_memory_baseline.rs: a recorded RSS-delta harness measuring resident-memory growth per superstep for a fixed, sized-payload graph, plus a measured Battlefield-clone-per-superstep count that fails loudly on regression (ENG-NFR-02)"
  - "MIGRATION.md section 9.4 memory/storage growth pointer to this plan's measured figures"
affects: [22-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Criterion iter_batched setup/routine split used to keep database construction, migration, and per-iteration Waypoint payload construction entirely outside the timed region -- the timed routine is the save() call alone"
    - "Battlefield-clone counting done by recording the raw pointer address of the &Battlefield each node execution observes (state as *const Battlefield as usize), the same technique crates/paladin-battalion/src/engine/test_support.rs's CountingFunctionNode and its battlefield_cloned_once_per_superstep_arc_ptr_eq test already use internally -- this harness re-proves the identical property from outside the crate through the public WarEngine API, rather than instrumenting Battlefield's Clone impl itself (which would add overhead to every consumer's hot path)"
    - "A fixed CHAIN_WIDTH-wide, CHAIN_LAYERS-deep multi-chain WarGraph (each chain's field touched by exactly one writer per superstep) used as the harness's fixed workload, avoiding any DispatchConflict by construction while still exercising concurrent same-superstep node execution"

key-files:
  created:
    - benches/engine_benchmarks.rs
    - examples/war_engine_memory_baseline.rs
  modified:
    - Cargo.toml
    - MIGRATION.md

key-decisions:
  - "The measured SQLite Waypoint save p50 (73.09 ms for the just-under-1-MiB case) is recorded honestly as a large overshoot of the ENG-NFR-01 10ms target, not tuned away -- the plan's own instruction is to record an overshoot as information the phase needs to surface. The finding and a plausible cause (see Deviations) are documented rather than hidden."
  - "The two tasks share one criterion bench binary (benches/engine_benchmarks.rs) per the plan's single-artifact spec; for atomic per-task commits, Task 1's commit contains only the waypoint_save group and Task 2's commit adds the superstep_cost group -- verified byte-identical bench_waypoint_save content across both commits so the measured p50s below are valid against the final, fully-committed file."
  - "Battlefield clone counting is measured via pointer-address observation from a public StateNode implementation rather than by modifying Battlefield's Clone impl or by grep-counting .clone() call sites in the source -- satisfies the plan's 'measured, not inspected from source' requirement without adding instrumentation overhead to the production hot path."
  - "The memory harness's bound check (max_distinct_per_superstep <= 1 + CHAIN_WIDTH) is the plan's literal ceiling; the actually measured value (1, well under the ceiling of 5) is the number that matters and is what a future per-node-clone regression would move."

requirements-completed: [ENG-02, ENG-05]

coverage:
  - id: D1
    description: "ENG-NFR-01: SqliteWaypointStore::save latency measured at three Battlefield payload sizes (1 KiB, 512 KiB, just under 1 MiB) via a criterion benchmark with setup outside the timed region, and the just-under-1-MiB p50 recorded against the under-10ms target"
    requirement: "ENG-05"
    verification:
      - kind: other
        ref: "cargo bench --bench engine_benchmarks -- --test (smoke mode, exit 0); cargo bench --bench engine_benchmarks (real run); p50 extracted via jq from target/criterion/engine_waypoint_save_sqlite_just_under_1mib/new/sample.json"
        status: pass
    human_judgment: false
  - id: D2
    description: "ENG-NFR-02: engine memory measured, not guessed -- a recorded RSS-delta harness reports bytes-per-superstep for a fixed, sized graph, and a pointer-identity clone counter proves exactly one Battlefield clone per superstep (well under the 1+concurrent-views ceiling), failing loudly on regression"
    requirement: "ENG-02"
    verification:
      - kind: other
        ref: "cargo run --release --example war_engine_memory_baseline (exit 0); prints battlefield_clone_bound_check=PASS (observed 1 <= ceiling 5)"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-02
status: complete
---

# Phase 22 Plan 10: Engine NFR Benchmarks Summary

**A criterion benchmark and a recorded RSS harness measure both ENG-NFR claims instead of asserting them: SQLite Waypoint-save p50 comes in at 73.09 ms (7.3x over the stated 10ms target), while the engine's Battlefield clone count is proven to be exactly one per superstep via a pointer-identity check, well inside its bound.**

## Performance

- **Duration:** ~50 min (dominated by cold release/bench compiles: two full `--release` builds and one `--all-features` clippy pre-warm)
- **Tasks:** 2 completed
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments

- `benches/engine_benchmarks.rs` measures `SqliteWaypointStore::save` at three Battlefield payload sizes, with database construction, migration, and per-iteration Waypoint construction entirely outside the timed region (criterion's `iter_batched` setup facility)
- The same bench file adds a `superstep_cost` group measuring `WarEngine::start` wall-clock cost at Vanguard widths 1 and 8 against `InMemoryWaypointStore`, isolating engine overhead from persistence cost
- `examples/war_engine_memory_baseline.rs` measures real process RSS growth across 20 supersteps of a 4-chain fixed graph carrying a 512 KiB Battlefield payload, and counts Battlefield clones per superstep via the same pointer-identity technique the engine crate's own internal test already uses, proving the Arc-shared-snapshot design holds from outside the crate
- MIGRATION.md section 9.4 now points at this plan's measured figures alongside the existing storage-growth guidance

## Task Commits

Each task was committed atomically:

1. **Task 1: ENG-NFR-01 — criterion benchmark for Waypoint save overhead on SQLite** - `ca4c4448` (test)
2. **Task 2: ENG-NFR-02 — measured engine memory per superstep** - `4b3110e2` (test)

_Note: both commits touch `benches/engine_benchmarks.rs`; Task 1's commit contains only the `waypoint_save` criterion group (matching the plan's Task 1 file scope), Task 2's commit adds the `superstep_cost` group. The `bench_waypoint_save` function body is byte-identical across both commits (verified), so the measured figures below are valid against the fully-committed file at `4b3110e2`._

## Measured Figures (commit `4b3110e2`)

All figures measured on this execution host inside its container/worktree sandbox; not a portable performance claim (see Deviations for a discussion of the SQLite figure's likely environment sensitivity).

### ENG-NFR-01 — SQLite Waypoint save latency (p50, from `sample.json`, n=100 per size)

| Payload size | p50 | p95 | p99 | Clears 10ms target? |
|---|---|---|---|---|
| 1 KiB | 40.32 ms (40,319,305 ns) | 66.72 ms | 71.45 ms | **No** |
| 512 KiB | 75.79 ms (75,793,920 ns) | 126.00 ms | 141.05 ms | **No** |
| Just under 1 MiB | **73.09 ms (73,090,420 ns)** | 114.90 ms | 156.65 ms | **No — 7.3x over target** |

Derived via the project's established method (`.planning/.../03-04-SUMMARY.md`'s jq filter over criterion's `SavedSample{iters,times}` on-disk schema, nearest-rank `round((n-1)*0.50)`, no interpolation) against `target/criterion/engine_waypoint_save_sqlite_{1kib,512kib,just_under_1mib}/new/sample.json`.

**This does not clear ENG-NFR-01's stated target, and is recorded as a measured finding rather than tuned to pass.** Even the smallest (1 KiB) payload's p50 is 40.3 ms — 4x the target — indicating the overhead is dominated by a fixed per-write cost, not by payload size scaling (512 KiB and just-under-1-MiB land within ~3ms p50 of each other, both well above the 1 KiB case). The most likely cause, not independently verified in this plan, is `SqliteWaypointStore`'s connection setup: `SqliteConnectOptions::from_str` plus `.create_if_missing(true)` sets no explicit `journal_mode`/`synchronous` pragma, so each `INSERT ... ON CONFLICT DO UPDATE` may be paying a full `fsync` under this sandbox's storage backend (commonly slow on containerized/overlay filesystems). A future plan investigating this should measure with `PRAGMA journal_mode=WAL` / `synchronous=NORMAL` explicitly set and re-run this same bench before concluding whether the 10ms target is achievable in principle or needs revision.

### Engine per-superstep wall-clock cost (p50, from `sample.json`, n=100 per width)

| Vanguard width | p50 |
|---|---|
| 1 node | 24.35 µs (24,352.21 ns) |
| 8 nodes | 52.71 µs (52,707.29 ns) |

Purely in-process engine overhead (`InMemoryWaypointStore`, no disk I/O) — three orders of magnitude below the SQLite save cost above, confirming the persistence path (not the superstep loop itself) is where ENG-NFR-01's overshoot lives.

### ENG-NFR-02 — engine memory per superstep

```
rss_before_kb=3972
rss_after_kb=15976
superstep_count=20
chain_width=4
payload_bytes=524288
elapsed_ms=14
rss_delta_kb=12004
bytes_per_superstep=614604
node_execution_observations=80
battlefield_addresses_observed_max_per_superstep=1
battlefield_clone_bound_check=PASS (observed 1 <= ceiling 5)
```

- **~614.6 KB resident-memory growth per superstep** for a 512 KiB payload Battlefield — consistent with one Battlefield clone per superstep plus bookkeeping overhead, not per-node cloning (which would scale with `chain_width=4`).
- **Exactly one distinct Battlefield snapshot address observed per superstep** (`battlefield_addresses_observed_max_per_superstep=1`), across 80 total node executions (4 nodes x 20 supersteps) — proving the Arc-shared read snapshot design holds: every node executing within a superstep reads the SAME snapshot, not its own clone. Comfortably inside the bound of `1 + CHAIN_WIDTH (4) = 5`.
- This clears ENG-NFR-02 cleanly: the engine's own memory design (Arc-shared snapshot, cloned once per superstep) is confirmed from outside the crate, not just by the crate's own internal test.

## Files Created/Modified

- `benches/engine_benchmarks.rs` - criterion benchmarks for `SqliteWaypointStore::save` (3 payload sizes) and `WarEngine::start` per-superstep cost (2 Vanguard widths)
- `examples/war_engine_memory_baseline.rs` - recorded RSS-delta + Battlefield-clone-count harness for ENG-NFR-02
- `Cargo.toml` - registers the `engine_benchmarks` `[[bench]]` target (`harness = false`)
- `MIGRATION.md` - section 9.4 gains a memory/storage growth pointer to this plan's measured figures

## Decisions Made

- Recorded the SQLite save p50 overshoot honestly (73.09 ms vs a 10ms target) rather than tuning batch size, sample count, or SQLite pragmas to produce a passing number — the plan explicitly asks for the measured figure and the observation when a target isn't cleared, and this codebase's own `security.instructions.md`/`STATE.md` culture (see e.g. the CodeQL and Snyk evaluations) consistently prefers a measured negative result stated plainly over a result that reads as assurance it doesn't have.
- Measured the Battlefield-clone count via pointer-identity observation from a `StateNode` implementation (matching the engine crate's own internal `CountingFunctionNode` technique) rather than adding a counting instrumentation layer to `Battlefield`'s `Clone` impl itself, which would add overhead to every consumer's hot path just to serve one benchmark's measurement.
- Split the single-file, two-task bench artifact into two commits along task boundaries (Task 1: `waypoint_save` group only; Task 2: adds `superstep_cost`), verifying the shared function's content stayed byte-identical across the split so the measured numbers remain valid against the final committed state.

## Deviations from Plan

**None — plan executed exactly as written; the SQLite p50 overshoot is a measured finding the plan explicitly asked to surface honestly, not a deviation from the plan's own instructions.**

### Auto-fixed Issues

None.

## Issues Encountered

- The first `git commit` attempt for Task 1 hit the repo's `--all-features` clippy pre-commit hook cold (no prior build of that feature combination existed), which exceeded a 2-minute foreground timeout and was killed mid-hook. Pre-commit's own patch-based stash-and-restore mechanism (a plain diff file under `~/.cache/pre-commit/`, not `git stash`) had stashed the in-progress `MIGRATION.md` edit and not yet restored it when the process was killed. Recovered by inspecting the still-present patch file, confirming it held exactly the intended edit, and re-applying it with `git apply` before retrying the commit (this time after pre-warming the `--all-features` clippy build in the background so the hook itself completed quickly on retry). No `git stash` command was ever used; no cross-worktree stash contamination risk was present.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both ENG-NFR-01 and ENG-NFR-02 are now measured with a recorded baseline and the exact commit (`4b3110e2`) a future regression check can diff against.
- **Open item for a future plan (not this plan's scope):** ENG-NFR-01's SQLite save p50 does not clear its 10ms target on this host. Before concluding the target is unreachable or needs revision, a follow-up should re-measure with explicit `journal_mode=WAL`/`synchronous=NORMAL` pragmas on `SqliteWaypointStore`'s connection and on non-containerized storage, since the measured 40ms floor even at 1 KiB payload strongly suggests a fixed per-fsync cost rather than a data-volume-scaling cost.
- Plan 22-11 (bridging legacy Formation/Phalanx/Campaign into `WarGraph`, ENG-06) depends on 22-08/22-09/22-10; this plan's artifacts (benches, example) do not block it — no shared file conflicts with 22-11's `<files_modified>` list.

## Self-Check: PASSED

- FOUND: `/workspace/.claude/worktrees/agent-abfd80919a97b0cbf/benches/engine_benchmarks.rs`
- FOUND: `/workspace/.claude/worktrees/agent-abfd80919a97b0cbf/examples/war_engine_memory_baseline.rs`
- FOUND commit `ca4c4448` in `git log --oneline`
- FOUND commit `4b3110e2` in `git log --oneline`
- MIGRATION.md section 9.4 contains the added "Memory and storage growth are measured, not asserted" bullet
- Cargo.toml contains the `[[bench]] name = "engine_benchmarks"` block

---
*Phase: 22-battlefield-state-superstep-engine*
*Completed: 2026-09-02*
