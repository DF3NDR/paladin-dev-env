---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 07
subsystem: config
tags: [config, serde, engine, muster, waypoint-durability, x-09]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Plan 23-05's EngineLimits.max_muster_tasks field and the MusterTaskLimitExceeded typed error"
provides:
  - "EngineConfig at src/config/engine.rs: Default, validate(), EnvOverridable, mirroring CitadelConfig/WaypointRetentionConfig"
  - "APP_ENGINE_* env override surface for all five engine tunables, proven end-to-end into a running WarEngine's muster limit"
  - "impl From<EngineConfig> for EngineLimits conversion, legal under the orphan rule with zero paladin-battalion edits"
  - "MIGRATION.md 9.5 EngineConfig entry updated from 'not yet in the tree' to landed"
affects: [23-12, ship-gate-x-09-audit]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config struct mirrors house shape (Default + validate() -> Result<(), String> + EnvOverridable), matching CitadelConfig/WaypointRetentionConfig"
    - "Private #[serde(with = ...)] shim module for a foreign enum field that doesn't derive Serialize/Deserialize, avoiding a cross-crate edit"

key-files:
  created: [src/config/engine.rs]
  modified: [src/config/mod.rs, MIGRATION.md]

key-decisions:
  - "waypoint_durability keeps its real WaypointDurability type (not widened to String) via a private waypoint_durability_serde module implementing #[serde(with = ...)], because WaypointDurability derives neither Serialize nor Deserialize in paladin-battalion and this plan may not edit that crate"
  - "APP_ENGINE_WAYPOINT_DURABILITY parsing normalizes case and strips underscores before matching (to_ascii_lowercase().replace('_', \"\")), so 'Strict', 'STRICT', 'best_effort', 'BestEffort' and 'BEST_EFFORT' all resolve to the same two variants; an unparseable value leaves the field at its prior value, matching read_env's own silent-parse-failure contract"

patterns-established:
  - "Cross-crate config struct wrapping a foreign non-serde enum: keep the real type on the field, add a private serde shim module in the same file rather than widening to String or editing the foreign crate"

requirements-completed: [CF-03]

coverage:
  - id: D1
    description: "EngineConfig exists at src/config/engine.rs with Default, validate(), EnvOverridable, mirroring CitadelConfig/WaypointRetentionConfig field-for-field, carrying all five engine tunables (four documented plus max_muster_tasks) with their APP_ENGINE_* env overrides"
    requirement: "CF-03"
    verification:
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::default_engine_config_matches_todays_engine_defaults"
        status: pass
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::validate_rejects_zero_limits"
        status: pass
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::env_overrides_apply_for_every_field"
        status: pass
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::waypoint_durability_parses_both_variants_case_insensitively"
        status: pass
    human_judgment: false
  - id: D2
    description: "APP_ENGINE_MAX_MUSTER_TASKS reaches a running engine's effective limit end-to-end: env override -> validate -> conversion into EngineLimits -> a muster exceeding it fails with the typed MusterTaskLimitExceeded error naming the configured limit"
    requirement: "CF-03"
    verification:
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::app_engine_max_muster_tasks_reaches_a_running_engines_limit"
        status: pass
    human_judgment: false
  - id: D3
    description: "MIGRATION.md 9.5 no longer describes EngineConfig as planned-but-absent; records it as landed at its real path with all five fields, and points the identical-boot claim at the named passing test"
    requirement: "CF-03"
    verification:
      - kind: other
        ref: "grep -c 'not yet in the tree' MIGRATION.md == 0; grep -c 'APP_ENGINE_MAX_MUSTER_TASKS' MIGRATION.md >= 1"
        status: pass
    human_judgment: false

duration: ~68min
completed: 2026-09-04
status: complete
---

# Phase 23 Plan 07: EngineConfig Landing Summary

**EngineConfig at `src/config/engine.rs` brings all five `WarEngine` tunables (including CF-FR-13's `max_muster_tasks`) into the house config shape, with `APP_ENGINE_MAX_MUSTER_TASKS` proven end-to-end into a running engine's muster-limit error.**

## Performance

- **Duration:** ~68 min
- **Started:** 2026-09-03T23:00:31Z (base commit)
- **Completed:** 2026-09-04T00:08:19Z
- **Tasks:** 2
- **Files modified:** 3 (`src/config/engine.rs` created; `src/config/mod.rs`, `MIGRATION.md` modified)

## Accomplishments

- `EngineConfig` struct at `src/config/engine.rs`, mirroring `CitadelConfig`/`WaypointRetentionConfig`'s shape field-for-field: manual `Default`, `validate() -> Result<(), String>`, `EnvOverridable`.
- Carries the four fields `MIGRATION.md` §9.5 already documented (`max_supersteps: 50`, `max_node_visits: 25`, `run_timeout_secs: None`, `waypoint_durability: Strict`) plus the new `max_muster_tasks: 100` (D-16, CF-FR-13), each with its `APP_ENGINE_*` environment override.
- `impl From<EngineConfig> for EngineLimits` — legal under the orphan rule since `EngineConfig` is local to this crate — with zero edits to `crates/paladin-battalion/` (`git diff HEAD -- crates/paladin-battalion/` is empty).
- End-to-end tracer test (`app_engine_max_muster_tasks_reaches_a_running_engines_limit`): sets `APP_ENGINE_MAX_MUSTER_TASKS`, builds `EngineConfig`, applies overrides, validates, converts into `EngineLimits`, runs a real `WarEngine` over a `WarGraph` whose planner musters more tasks than the configured limit, and asserts the typed `EngineError::MusterTaskLimitExceeded` error names the configured limit.
- `default_engine_config_matches_todays_engine_defaults` mechanically proves `EngineConfig::default()` converts to exactly `EngineLimits::default()` and `WaypointDurability::Strict` — the checkable form of §9.5's "a v0.9 configuration boots identically" claim.
- `MIGRATION.md` §9.5's `EngineConfig` bullet rewritten from "not yet in the tree" to landed, with the real path, all five fields/env-vars, the `EngineLimits` conversion, and the closing identical-boot claim now pointing at the named test.

## Task Commits

Each task was committed atomically:

1. **Task 1: EngineConfig from environment to a running engine's effective limits** - `8fd4fc2a` (feat)
2. **Task 2: Close MIGRATION.md §9.5's EngineConfig claim and record CF-05's no-config decision** - `7f43f496` (docs)

**Plan metadata:** _pending — this SUMMARY's own commit_

_Note: no TDD-gated tasks in this plan; Task 1 is `type="tracer"` with tests written and run as part of the single commit, matching the plan's `<verify>` requirement, not a separate RED/GREEN commit pair._

## Files Created/Modified

- `src/config/engine.rs` - `EngineConfig` struct, `validate()`, `EnvOverridable` impl, `From<EngineConfig> for EngineLimits`, the private `waypoint_durability_serde` shim, and the full test module (7 unit tests + 3 doc tests).
- `src/config/mod.rs` - registers `pub mod engine;` and re-exports `EngineConfig`, in the same position/style as the other config sub-modules.
- `MIGRATION.md` - §9.5's `EngineConfig` bullet rewritten as landed; closing identical-boot paragraph updated to cite the named test. No other section touched.

## Decisions Made

- **`waypoint_durability` keeps its real `WaypointDurability` type, via a private serde shim, not a `String` field.** `WaypointDurability` (`paladin-battalion::engine`) derives neither `Serialize` nor `Deserialize`, which would otherwise break `EngineConfig`'s own struct-level `#[derive(Serialize, Deserialize)]`. Widening the field to `String` would lose type safety at every call site and diverge from the house shape's typed-field convention; editing `paladin-battalion` to add the derive is out of this plan's `files_modified` scope (`src/config/engine.rs`, `src/config/mod.rs`, `MIGRATION.md` only) and the acceptance criteria explicitly assert `git diff HEAD -- crates/paladin-battalion/` stays empty. The private `waypoint_durability_serde` module (`#[serde(with = ...)]`) resolves this within the file, rendering `"strict"` / `"best_effort"`.
- **Env-var parsing for `waypoint_durability` normalizes case and underscores before matching** (`to_ascii_lowercase().replace('_', "")`), so any of `Strict`, `STRICT`, `strict`, `BestEffort`, `best_effort`, `BEST_EFFORT` resolves correctly, satisfying the plan's `waypoint_durability_parses_both_variants_case_insensitively` behavior requirement without over-fitting to one casing convention.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Manual-Default-impl comment contained the literal substring the acceptance criteria's negative grep checks for**
- **Found during:** Task 1, self-verification pass after first `clippy` run
- **Issue:** The doc comment above `impl Default for EngineConfig` originally read `// A manual impl (not #[derive(Default)]), ...` — this literal text matched the acceptance criteria's own `grep -c '#\[derive(.*Default' src/config/engine.rs` check (intended to verify no `#[derive(Default)]` attribute exists), producing a false-positive count of 1 instead of the required 0.
- **Fix:** Reworded the comment to describe the same fact ("a manual Default impl, no derive macro") without the literal `#[derive(` + `Default` substring on one line.
- **Files modified:** `src/config/engine.rs`
- **Verification:** `grep -c '#\[derive(.*Default' src/config/engine.rs` now returns `0`; `grep -c 'impl Default for EngineConfig' src/config/engine.rs` still returns `1`.
- **Committed in:** `8fd4fc2a` (Task 1 commit — caught before the commit was made, not a follow-up fix)

**2. [Rule 1 - Bug] clippy `field_reassign_with_default` in `validate_rejects_zero_limits`**
- **Found during:** Task 1, first `cargo clippy --workspace --all-targets --all-features -- -D warnings` run
- **Issue:** The test built `EngineConfig::default()` then reassigned a single field (`config.max_supersteps = 0;`, etc.) four times — clippy's `field_reassign_with_default` lint (deny-by-default under `-D warnings`) flags this pattern because it silently drops any future default-value change to the untouched fields.
- **Fix:** Rewrote each case using struct-update syntax (`EngineConfig { max_supersteps: 0, ..EngineConfig::default() }`), matching the lint's own suggested fix.
- **Files modified:** `src/config/engine.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0; `cargo test -p paladin-ai --lib config::engine` still reports 7 passed, 0 failed.
- **Committed in:** `8fd4fc2a` (Task 1 commit — caught before the commit was made)

---

**Total deviations:** 2 auto-fixed (1 blocking/self-check text collision, 1 blocking/clippy lint), both resolved before their task's commit landed.
**Impact on plan:** Neither changed the plan's design or scope — both are wording/style fixes internal to the test module. No scope creep.

## Issues Encountered

None beyond the two auto-fixed deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `EngineConfig` is available at `paladin::config::engine::EngineConfig` (re-exported as `paladin::config::EngineConfig`) for any later plan that wires it into `Settings`/`Settings::get_engine_config()`-style plumbing — this plan supplies the configuration surface only, not a call site that constructs a `WarEngine` from it in production code (out of scope per the plan's action step 4: "Add nothing to `EngineLimits` in this plan").
- `MIGRATION.md` §9.5's `EngineConfig` claim is closed; the section's remaining TBD (the v0.9-sample-config integration test) stays owned by SHIP-02, Phase 29, untouched by this plan.
- `git diff HEAD -- crates/paladin-battalion/` is empty — Plan 23-06 (Waypoints, running concurrently in a sibling worktree) and this plan touch disjoint files as designed.

## Self-Check: PASSED

- `src/config/engine.rs` — FOUND
- `src/config/mod.rs` — FOUND
- `MIGRATION.md` — FOUND
- Commit `8fd4fc2a` (Task 1) — FOUND in `git log --oneline --all`
- Commit `7f43f496` (Task 2) — FOUND in `git log --oneline --all`
- `cargo test -p paladin-ai --lib config::engine` — 7 passed, 0 failed
- `cargo test -p paladin-ai --doc config::engine` — 3 passed, 0 failed
- `cargo test --workspace --lib --bins` — 523+2+430+431+96+1+43+110+76+0+105+103+117 = all passed, 0 failed across every workspace crate
- `cargo fmt --check` — clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean
- `git diff HEAD -- crates/paladin-battalion/` — empty

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-04*
