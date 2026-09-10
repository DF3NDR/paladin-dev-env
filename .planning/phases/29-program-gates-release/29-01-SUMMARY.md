---
phase: 29-program-gates-release
plan: 01
subsystem: testing
tags: [rust, axum, config, backward-compat, ship-02, integration-test]

# Dependency graph
requires:
  - phase: 27-platform-api
    provides: every platform config struct defaults off, every new v0.10 route answers 501 not 404 when unwired
  - phase: 28-observability-tooling
    provides: trace/web_server Settings fields (the two v0.10-era Settings additions this fixture must NOT mention)
provides:
  - Frozen, provenance-tracked v0.9.0 config fixtures (loadable + documentation-only) committed under tests/fixtures/config/
  - New root integration test target v0_9_config_boot (required-features = web-server), 9 passing tests
  - Machine-checkable proof that a v0.9 operator's config file boots v0.10 HEAD with every new subsystem inert and the exact v0.9 route set live
affects: [29-05 (cites v0_9_config_boot by name in MIGRATION.md §9.5/CI step), 29-04 (SHIP-03 acceptance audit FR-to-test anchors)]

# Tech tracking
tech-stack:
  added: []
  patterns: [frozen-fixture-with-provenance-readme, config-resolution-plus-behavioral-boot-proof, scoped-env-save-clear-restore-under-serial]

key-files:
  created:
    - tests/fixtures/config/v0.9.0-config.test.yml
    - tests/fixtures/config/v0.9.0-config.example.yml
    - tests/fixtures/config/README.md
    - tests/integration/v0_9_config_boot_test.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Followed RESEARCH.md Open Question 1 resolution: froze BOTH v0.9.0 config files. v0.9.0-config.test.yml (loads cleanly, is what the test actually calls Settings::load_from_file against) and v0.9.0-config.example.yml (documentation evidence only -- does not parse at either tag, pre-existing LlmProviderConfig::api_key/ollama defect, not a Phase 29 regression)."
  - "AssistantsConfig::default().expose_code_registry is true, not false -- corrected in the test and its comments against the plan's Task 2 <behavior> text, which had it backwards. Verified directly against src/config/assistants.rs's own Default impl and doc comment ('the one subsystem that starts ON')."
  - "Composed the app router directly in the test (agent_router + thread_router over an unwired ThreadApiState::new().with_auth(...) + run_router + docs_router) rather than reusing src/bin/paladin-server.rs's #[cfg(test)]-only build_thread_state helper, which is private to that binary crate and unreachable from an external tests/integration/ target."

patterns-established:
  - "Config-resolution + behavioral two-level boot proof: assert every v0.10 config surface (both Settings fields AND the nine off-Settings platform structs) resolves to Default, then separately assert the composed real routers answer the right status codes -- reusable for any future backward-compat boot claim."

requirements-completed: [SHIP-02]

coverage:
  - id: D1
    description: "Frozen v0.9.0 config fixtures with checkable provenance (git hash-object matches the named blob SHAs) committed under tests/fixtures/config/"
    requirement: SHIP-02
    verification:
      - kind: other
        ref: "git hash-object tests/fixtures/config/v0.9.0-config.test.yml == e63d9f93582e0e06f2a1b40530fac94e846ffe32; same for v0.9.0-config.example.yml == fecb9bd94278612b7246cc5ff03839bf80cdd249"
        status: pass
    human_judgment: false
  - id: D2
    description: "v0_9_config_boot integration test target: a v0.9-shaped config loads at v0.10 HEAD, every v0.10 config surface (Settings fields and the nine off-Settings platform structs) resolves to its own inert Default, apply_env_overrides() is a no-op with no APP_* vars set, and the composed real routers answer the v0.9 route set's 200s while every new-in-v0.10 route family answers 501 (never 404, except the genuinely-unregistered dev-ui path)"
    requirement: SHIP-02
    verification:
      - kind: integration
        ref: "tests/integration/v0_9_config_boot_test.rs — cargo test --features web-server --test v0_9_config_boot"
        status: pass
    human_judgment: false

# Metrics
duration: ~40min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 01: v0.9 Config Boot Proof Summary

**New `v0_9_config_boot` integration test target proves a frozen, provenance-tracked v0.9.0 config file boots v0.10 HEAD with every new subsystem inert and the exact v0.9 route set live, closing SHIP-02's config half.**

## Performance

- **Duration:** ~40 min
- **Tasks:** 2
- **Files modified:** 5 (2 new fixture YAML files, 1 new README, 1 new test file, 1 Cargo.toml edit)

## Accomplishments

- Froze `v0.9.0-config.test.yml` (loadable, byte-identical to blob `e63d9f93582e0e06f2a1b40530fac94e846ffe32`) and `v0.9.0-config.example.yml` (documentation-only, byte-identical to blob `fecb9bd94278612b7246cc5ff03839bf80cdd249`) directly from `git show v0.9.0:...` output — never copied from the working tree — with a `README.md` recording the tag commit and both blob SHAs for independent re-verification.
- Wrote `tests/integration/v0_9_config_boot_test.rs`, registered as `[[test]] name = "v0_9_config_boot"` with `required-features = ["web-server"]`, containing 9 tests across both D-07 levels:
  - **Config resolution:** the fixture loads with no edits; `Settings::agent_runtime`/`trace`/`web_server` all resolve to `Default`; all nine platform config structs (`EngineConfig`, `WaypointStoreConfig`, `RunStoreConfig`, `RunQueueConfig`, `RunWorkerConfig`, `RunStreamConfig`, `AssistantsConfig`, `SchedulesConfig`, `WebhooksConfig`) report their off/inert state and validate `Ok`; `EngineConfig::default().graceful_shutdown` is `true` (M-B-02's deliberate exception); `apply_env_overrides()` is a no-op on all nine structs with all 26 `APP_*` variables cleared under `#[serial]`.
  - **Behavioral:** the composed real routers (`agent_router` + `thread_router` + `run_router` + `docs_router`, mirroring `src/bin/paladin-server.rs`'s own composition) answer `200` for the six v0.9 `/v1/agents…` paths plus `/health`/`/ready`/`/openapi.json`, and `501` (never `404`) for `/v1/runs`, `/v1/threads/*/history`, `/v1/assistants`, `/v1/schedules`; the `dev-ui` path is the one genuine `404` because that router is never merged at all.
- Verified end to end: `cargo test --features web-server --test v0_9_config_boot` → `test result: ok. 9 passed`; `cargo fmt --all --check` clean; `cargo clippy --all-targets --features web-server -- -D warnings` clean; `cargo check --workspace --all-targets --features web-server` clean; `git diff --stat v0.9.0..HEAD -- tests/fixtures/config/` shows only additions; `cargo test --workspace --lib --bins --no-run` confirms the new target is NOT selected by CI's `test` job scope (feature-gated, as designed by D-09's replan).

## Task Commits

1. **Task 1: One v0.9 config file, end to end (tracer)** — `fe5b55be` (feat) — froze both fixtures, wrote the provenance README, wrote the 2-test tracer slice, registered the `[[test]]` block. Verified: `cargo test --features web-server --test v0_9_config_boot` → `2 passed` before proceeding.
2. **Task 2: Expand the proof (tdd="true")** — `892d063b` (test) — added the remaining 7 tests. No RED-then-GREEN split: see "TDD Gate Compliance" below.

**Plan metadata:** committed as part of this SUMMARY (see final commit).

## Files Created/Modified

- `tests/fixtures/config/v0.9.0-config.test.yml` — frozen, loadable v0.9-shaped config; the file the boot test actually loads
- `tests/fixtures/config/v0.9.0-config.example.yml` — frozen, verbatim v0.9.0 sample config; documentation evidence only, does not parse (pre-existing defect, out of scope)
- `tests/fixtures/config/README.md` — provenance record for both fixtures (tag, commit, blob SHAs, producing commands)
- `tests/integration/v0_9_config_boot_test.rs` — the 9-test SHIP-02 boot proof
- `Cargo.toml` — new `[[test]]` block registering `v0_9_config_boot`

## Decisions Made

- **RESEARCH.md Open Question 1 resolved as planned (D-06 deviation):** both v0.9.0 config files are frozen; the loadable one (`config.test.yml`) is what the test calls `Settings::load_from_file` against, the non-parsing one (`config.example.yml`) is kept as documentation evidence only, with the pre-existing `ollama.api_key` defect stated plainly in the README rather than silently worked around.
- **Corrected the plan's Task 2 `<behavior>` text against the actual source:** `AssistantsConfig::default().expose_code_registry` is `true` (deliberately, per the struct's own doc comment: "the one subsystem in this phase that starts ON"), not `false` as the plan draft's prose claimed. The test asserts the true value with an explanatory comment rather than propagating the plan's incorrect characterization into a test that would otherwise fail. This does not affect the "inert" framing the plan cares about: the flag exposes a read-only view of agents already resident in the pre-existing registry, wiring no new backend and spawning no new task.
- **Thread-state composition built directly in the test** rather than reusing `paladin-server.rs`'s `#[cfg(test)]`-only `build_thread_state` helper, which is private to that binary crate and not reachable from an external `tests/integration/` target. With `WaypointStoreConfig::default()` (backend `Disabled`), `paladin-server.rs`'s own `thread_state_from_store` reduces to exactly `ThreadApiState::new().with_auth(auth)` — reproduced directly, not duplicating any decision logic.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan text, not in code] `AssistantsConfig`'s default flag value corrected**
- **Found during:** Task 2, writing `platform_config_defaults_are_inert`
- **Issue:** The plan's Task 2 `<behavior>` text states "AssistantsConfig's code-registry exposure is false" as one of the nine structs' "off state" assertions. Direct read of `src/config/assistants.rs::Default for AssistantsConfig` shows `expose_code_registry: true`, with its own doc comment explicitly calling this out as "the one subsystem in this phase that starts ON".
- **Fix:** Asserted the correct value (`true`) with an explanatory comment citing the source doc comment, rather than writing an assertion that would fail against real behavior.
- **Files modified:** `tests/integration/v0_9_config_boot_test.rs`
- **Verification:** `cargo test --features web-server --test v0_9_config_boot` passes 9/9.
- **Committed in:** `892d063b` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 plan-text correction, category: bug in the plan's characterization of already-shipped behavior)
**Impact on plan:** No scope creep; the fix keeps the test asserting ground truth rather than a plan draft's incorrect claim about existing, unmodified production code (X-03 forbids changing that code to match the plan's claim instead).

## TDD Gate Compliance

Task 2 carries `tdd="true"`, but this plan's whole objective is to **prove — not build — already-shipped behavior** (Phase 27's D-50 defaults-off design, RESEARCH.md, 27-DISCUSSION-LOG.md line 216: "makes SHIP-02's boot test pass by construction"). Task 2's `<files>` names only the test file itself — no production source is in scope (X-03 forbids it) — so there was no "implementation" step to hold back behind a failing test. Writing the 7 additional test functions produced `test result: ok. 9 passed` on the first run; no assertion failed at any point.

Per the fail-fast rule ("if a test passes unexpectedly during RED phase, investigate — the feature may already exist"): the feature does already exist, deliberately, by Phase 27's own design decision, and Phase 29's stated job is to add the proof, not to change behavior. Contriving a RED state (e.g., temporarily asserting a wrong status code) would not have exercised any new code path — it would only have demonstrated that a deliberately-wrong assertion fails, which is not informative here. This is recorded as a deviation from the literal RED-then-GREEN sequence, not skipped silently:

- No `test(...)` → `feat(...)` commit pair exists for Task 2 (there is one `test(29-01)` commit).
- Both task commits (`fe5b55be` feat, `892d063b` test) are verified present in `git log`.
- `git diff --stat HEAD~1 -- 'src/**/*.rs' 'crates/**/*.rs'` is empty for the Task 2 commit (confirmed before committing), satisfying X-03 and the acceptance criterion's literal check regardless of the RED/GREEN question.

## Issues Encountered

None beyond the plan-text correction documented above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `v0_9_config_boot` is ready for plan 29-05 to cite by name when rewriting `MIGRATION.md` §9.5's placeholder sentence and adding the CI step to the existing `e2e-platform-api` job (D-09's replan; no new CI job created by this plan).
- Both frozen fixtures' bytes are independently re-verifiable via `git hash-object` with no network or tag access at test time, satisfying this plan's own success criteria.
- No blockers for 29-02 through 29-09; this plan touched no production code and no shared orchestrator artifact (STATE.md/ROADMAP.md/REQUIREMENTS.md checkboxes untouched, per worktree isolation instructions).

## Self-Check: PASSED

- `tests/fixtures/config/v0.9.0-config.test.yml` — FOUND
- `tests/fixtures/config/v0.9.0-config.example.yml` — FOUND
- `tests/fixtures/config/README.md` — FOUND
- `tests/integration/v0_9_config_boot_test.rs` — FOUND
- Commit `fe5b55be` — FOUND in `git log --oneline --all`
- Commit `892d063b` — FOUND in `git log --oneline --all`
- `git hash-object tests/fixtures/config/v0.9.0-config.test.yml` == `e63d9f93582e0e06f2a1b40530fac94e846ffe32` — MATCHED
- `git hash-object tests/fixtures/config/v0.9.0-config.example.yml` == `fecb9bd94278612b7246cc5ff03839bf80cdd249` — MATCHED
- `cargo test --features web-server --test v0_9_config_boot` → `test result: ok. 9 passed; 0 failed` — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
