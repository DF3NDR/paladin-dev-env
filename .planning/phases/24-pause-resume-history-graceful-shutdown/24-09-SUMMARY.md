---
phase: 24-pause-resume-history-graceful-shutdown
plan: 09
subsystem: infra
tags: [rust, tokio, graceful-shutdown, kubernetes, axum, migration-guide]

# Dependency graph
requires:
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "ShutdownCoordinator/RunGuard/ShutdownOutcome (paladin-battalion::engine::shutdown), WarEngine::with_shutdown_grace, and EngineConfig.shutdown_grace_secs/graceful_shutdown from plan 24-08"
provides:
  - "src/bin/paladin-server.rs constructs one EngineConfig + ShutdownCoordinator per process; shutdown_signal cancels it and waits <= shutdown_grace (skipped when graceful_shutdown=false) before axum::serve(...).with_graceful_shutdown completes"
  - "ServiceRunner (src/config/setup/service_runner.rs) gains its own ShutdownCoordinator + shutdown_coordinator() accessor; wait_for_shutdown applies the identical cancel-then-drain pattern before the runner exits"
  - "drain_on_shutdown(coordinator, grace, graceful) split out of both shutdown_signal and wait_for_shutdown so the drain behaviour is unit-tested against a simulated trigger, never a real OS signal"
  - "resume_continues_a_halted_thread_after_process_shutdown -- the explicit HITL-FR-14 assertion at the process-wiring level (a run Halted via a coordinator-cancelled token, then resumed by a fresh WarEngine, completes)"
  - "k8s/server/deployment.yaml and k8s/deployment.yaml declare terminationGracePeriodSeconds: 60 (2x the 30s default shutdown_grace) with an inline derivation comment"
  - "k8s/README.md, docs/src/deployment/kubernetes.md and docs/src/deployment/production.md document the 2x rule, both env vars, and the Skipped/re-listed-for-resume operator-observable outcome; production.md's stale 30s figure and pre-ShutdownCoordinator code sample are reconciled with the shipped implementation"
  - "MIGRATION.md 9.1's M-B-02 row and worked example move from future-tense placeholder to a concrete before/after; 9.5 lists shutdown_grace_secs/graceful_shutdown with their fingerprint-exclusion note; 9.8's termination-grace bullet names 60 and the 2x derivation"
affects: [24-10, 24-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "wait_for_termination_signal() / drain_on_shutdown(coordinator, grace, graceful) split: the untestable OS-signal wait is a thin wrapper around a fully unit-testable drain step, applied identically in both paladin-server.rs and service_runner.rs"
    - "RED via a no-op stub, not a compile error: for TDD tasks where the test names an existing public type (ShutdownCoordinator, already landed in 24-08) rather than a not-yet-existing one, RED is produced by writing the real function signature with a no-op body first, running the tests to observe genuine assertion failures, then filling in the real body for GREEN"

key-files:
  created: []
  modified:
    - src/bin/paladin-server.rs
    - src/config/setup/service_runner.rs
    - k8s/server/deployment.yaml
    - k8s/deployment.yaml
    - k8s/README.md
    - docs/src/deployment/kubernetes.md
    - docs/src/deployment/production.md
    - MIGRATION.md

key-decisions:
  - "Test 6 (resume-after-Halted at the process-wiring level) triggers Halted via the pre-existing top-of-loop boundary check (cancel the token BEFORE calling start(), so superstep 1 Halts with the entry vanguard unexecuted) rather than the mid-superstep grace race from plan 24-08 -- avoids reimplementing a self-cancelling node test double that plan 24-08's test_support module keeps pub(crate) (unreachable from src/bin/paladin-server.rs), while still proving the exact coordinator-cancel -> Halted -> resume -> Completed chain HITL-FR-14 requires."
  - "EngineConfig is constructed fresh (Default + apply_env_overrides + validate) inside both run() and wait_for_shutdown/run_services, never added to Settings -- continuing the X-09/Phase 22-23 precedent D-20 states explicitly, confirmed by a zero-diff check against src/config/settings.rs after both wiring tasks."
  - "ServiceRunner::wait_for_shutdown falls back to EngineConfig::default() (rather than propagating an error) when apply_env_overrides produces an invalid config, since shutdown itself must not fail to run; paladin-server.rs's run(), by contrast, fails closed at startup (before serving any traffic) since an invalid config there is a legitimate refuse-to-start condition."

patterns-established: []

requirements-completed: [HITL-04]

coverage:
  - id: D1
    description: "Both paladin-server.rs's shutdown_signal and ServiceRunner::wait_for_shutdown cancel a ShutdownCoordinator and wait <= shutdown_grace for in-flight runs (skipped when graceful_shutdown=false)"
    requirement: "HITL-04"
    verification:
      - kind: unit
        ref: "src/bin/paladin-server.rs#shutdown_signal_cancels_the_coordinator"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#process_waits_up_to_grace_for_in_flight_runs"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#process_stops_waiting_at_the_grace_deadline"
        status: pass
      - kind: unit
        ref: "src/bin/paladin-server.rs#graceful_shutdown_disabled_skips_the_wait"
        status: pass
      - kind: unit
        ref: "src/config/setup/service_runner.rs#service_runner_wait_for_shutdown_cancels_the_coordinator"
        status: pass
    human_judgment: false
  - id: D2
    description: "resume continues a Halted thread, asserted explicitly at the process-wiring level (HITL-FR-14)"
    requirement: "HITL-04"
    verification:
      - kind: unit
        ref: "src/bin/paladin-server.rs#resume_continues_a_halted_thread_after_process_shutdown"
        status: pass
    human_judgment: false
  - id: D3
    description: "k8s/server/deployment.yaml and k8s/deployment.yaml declare terminationGracePeriodSeconds: 60; k8s/README.md, docs/src/deployment/kubernetes.md and docs/src/deployment/production.md teach the 2x rule and both env vars"
    requirement: "HITL-04"
    verification:
      - kind: other
        ref: "python3 -c \"import yaml; [yaml.safe_load(open(f)) for f in ['k8s/server/deployment.yaml','k8s/deployment.yaml']]\" (parses, both contain terminationGracePeriodSeconds: 60)"
        status: pass
      - kind: other
        ref: "mdbook build docs (after mdbook-mermaid install) -- 0 broken links"
        status: pass
    human_judgment: false
  - id: D4
    description: "MIGRATION.md M-B-02 worked example, section 9.5 EngineConfig fields, and section 9.8's termination-grace bullet are concrete; sections 9.2/9.6 untouched"
    requirement: "HITL-04"
    verification:
      - kind: other
        ref: "grep -c APP_ENGINE_SHUTDOWN_GRACE_SECS MIGRATION.md (5 matches) && grep -c terminationGracePeriodSeconds MIGRATION.md (5 matches)"
        status: pass
      - kind: other
        ref: "git diff -U0 MIGRATION.md hunks confined to lines ~16, ~70-116, ~172-175, ~196 -- no touch to the 9.2 register (~74-94) or 9.6 (~139)"
        status: pass
    human_judgment: false

duration: ~30min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 09: Graceful Shutdown -- Process Wiring & Operator Surface Summary

**SIGTERM/SIGINT now cancel a shared `ShutdownCoordinator` from both `paladin-server`'s `shutdown_signal` and `ServiceRunner::wait_for_shutdown`, draining in-flight engine runs within a configurable grace (or skipping the wait via `APP_ENGINE_GRACEFUL_SHUTDOWN=false`), and every operator-facing surface -- both k8s manifests, three deployment docs, and `MIGRATION.md`'s M-B-02 entry -- now teaches the same 2x-grace termination rule.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 (Task 1 as two RED/GREEN pairs -- one per entry point; Tasks 2-3 as direct doc/manifest edits per their own acceptance criteria, no test cycle)
- **Files modified:** 8

## Accomplishments

- `src/bin/paladin-server.rs`'s `run()` now constructs one `EngineConfig` (env-overridden, validated -- refuses to start on an invalid override) and one `ShutdownCoordinator` per process, both threaded into a refactored `shutdown_signal(coordinator, grace, graceful)`. The OS-signal wait (`wait_for_termination_signal`) and the actual cancel-and-drain step (`drain_on_shutdown`) are separate functions specifically so the latter is unit-tested against a simulated trigger rather than requiring a real SIGTERM in CI.
- `src/config/setup/service_runner.rs`'s `ServiceRunner` gains its own `ShutdownCoordinator` field plus a `shutdown_coordinator()` accessor (so a future in-process engine-running component can register with the SAME instance), and `wait_for_shutdown` applies the identical `wait_for_termination_signal` / `drain_on_shutdown` split -- falling back to `EngineConfig::default()` rather than failing shutdown itself if `apply_env_overrides` ever produces an invalid config at that late stage.
- Six named tests (HITL-04, D-22) all green: `shutdown_signal_cancels_the_coordinator`, `process_waits_up_to_grace_for_in_flight_runs`, `process_stops_waiting_at_the_grace_deadline`, `graceful_shutdown_disabled_skips_the_wait`, `service_runner_wait_for_shutdown_cancels_the_coordinator`, and `resume_continues_a_halted_thread_after_process_shutdown` -- the last one built entirely against public `WarEngine`/`WarGraph`/`StateNode` API (mirroring `src/config/engine.rs`'s own test doubles) since `paladin-battalion`'s `test_support` module is `pub(crate)` and unreachable from `src/`.
- `k8s/server/deployment.yaml` and `k8s/deployment.yaml` both declare `terminationGracePeriodSeconds: 60` with an inline comment stating the 2x-the-30s-default derivation; `k8s/README.md`, `docs/src/deployment/kubernetes.md` and `docs/src/deployment/production.md` each gained a section teaching the 2x rule, both env vars (`APP_ENGINE_SHUTDOWN_GRACE_SECS`, `APP_ENGINE_GRACEFUL_SHUTDOWN`), and what an operator observes on SIGTERM (a run finishes in-grace, or is `Skipped`/re-listed for exactly-once resume). `production.md`'s stale `terminationGracePeriodSeconds: 30` figure and its pre-`ShutdownCoordinator` illustrative `shutdown_signal` code sample were both rewritten to match the shipped implementation.
- `MIGRATION.md`'s M-B-02 row (9.1) moved from future tense ("A documented disable switch will be provided") to shipped present tense naming `APP_ENGINE_GRACEFUL_SHUTDOWN` and the 60s manifests; the "TBD — owner HITL-04, Phase 24" worked-example placeholder was replaced with a concrete before/after YAML pair, an env-var table, and the Skipped/re-listed operator-outcome note. Section 9.5's `EngineConfig` bullet list gained `shutdown_grace_secs`/`graceful_shutdown` (including the fingerprint-exclusion rationale). Section 9.8's "termination grace lands with HITL-04" bullet now names `60` and its 2x derivation. Sections 9.2 and 9.6 (plan 24-12 scope) were left untouched, confirmed by inspecting `git diff -U0`'s hunk boundaries.

## Task Commits

1. **Task 1: Wire the coordinator into both process entry points** -- two RED/GREEN pairs (one per entry point, since the two files' test suites and stub states are independent):
   - `68e13d68` -- `test(24-09): reproduce ShutdownCoordinator process wiring on a no-op drain (red)` -- `paladin-server.rs`: 6 tests added against a no-op `drain_on_shutdown` stub; 4 fail as expected (1 upper-bound-only assertion is vacuously satisfied by the no-op, noted in the commit message).
   - `ca81f6e0` -- `feat(24-09): wire ShutdownCoordinator into paladin-server::shutdown_signal (HITL-04, D-22)` -- real `drain_on_shutdown` body; all 7 tests in the file (6 new + 1 pre-existing pair) green.
   - `66b7a265` -- `test(24-09): reproduce ServiceRunner ShutdownCoordinator wiring on a no-op drain (red)` -- `service_runner.rs`: `shutdown_coordinator` field + accessor added, `wait_for_shutdown` split, 1 test added against a no-op stub; fails as expected.
   - `861a1e8c` -- `feat(24-09): wire ShutdownCoordinator into ServiceRunner::wait_for_shutdown (HITL-04, D-22)` -- real `drain_on_shutdown` body; all 10 tests in the module green.
2. **Task 2: Operator surface -- k8s manifests and deployment docs**:
   - `2d5b3ffe` -- `docs(24-09): operator surface for graceful shutdown -- k8s manifests + deployment docs (HITL-04, D-23)`.
3. **Task 3: MIGRATION.md M-B-02 worked example, section 9.5 and section 9.8**:
   - `a1fa207c` -- `docs(24-09): fill M-B-02 worked example, EngineConfig fields, and 9.8 termination-grace value (HITL-04, D-23)`.

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/bin/paladin-server.rs` -- `EngineConfig`/`ShutdownCoordinator` construction in `run()`; `wait_for_termination_signal`/`drain_on_shutdown`/`shutdown_signal(coordinator, grace, graceful)`; 6 new tests.
- `src/config/setup/service_runner.rs` -- `ServiceRunner.shutdown_coordinator` field + `shutdown_coordinator()` accessor; `wait_for_termination_signal`/`drain_on_shutdown`; `wait_for_shutdown` now constructs `EngineConfig` and delegates; 1 new test.
- `k8s/server/deployment.yaml`, `k8s/deployment.yaml` -- `terminationGracePeriodSeconds: 60` + derivation comment.
- `k8s/README.md` -- new "Graceful Shutdown" section.
- `docs/src/deployment/kubernetes.md` -- new "Graceful Shutdown" section (+ TOC entry); illustrative Deployment example gains `terminationGracePeriodSeconds: 60`.
- `docs/src/deployment/production.md` -- "Graceful Shutdown" section rewritten to match the shipped `ShutdownCoordinator` pattern; termination-grace YAML updated to `60`.
- `MIGRATION.md` -- 9.1 M-B-02 row + worked example, 9.5 `EngineConfig` bullets, 9.8 termination-grace bullet.

## Decisions Made

- Test 6 (`resume_continues_a_halted_thread_after_process_shutdown`) triggers `Halted` via the pre-existing top-of-loop boundary check (cancelling the token before `start()`, so superstep 1 Halts with the entry vanguard unexecuted) rather than the mid-superstep grace-race path plan 24-08 exercises -- this avoids needing a self-cancelling `StateNode` test double, since plan 24-08's `SlowFunctionNode::cancelling` lives in `paladin-battalion`'s `pub(crate)` `test_support` module and is unreachable from `src/bin/paladin-server.rs`. The chain proven (coordinator cancel -> `Halted` -> fresh-engine `resume` -> `Completed`) is exactly what HITL-FR-14 requires at this layer; the engine-internal mid-superstep mechanics stay pinned by plan 24-08's own tests.
- `EngineConfig` is constructed fresh via `Default` + `apply_env_overrides` + `validate` inside both `run()` and `wait_for_shutdown`/`run_services`, never added as a field on `Settings` -- continuing the X-09/Phase 22-23 precedent D-20 states explicitly. Verified with a zero-diff check: `git diff <task-1-range> -- src/config/settings.rs` is empty.
- `ServiceRunner::wait_for_shutdown` falls back to `EngineConfig::default()` (logging a warning) rather than propagating an error when `apply_env_overrides` yields an invalid config, since shutdown itself must not fail to run. `paladin-server.rs`'s `run()`, by contrast, fails closed at startup — before serving any traffic — since an invalid config there is a legitimate refuse-to-start condition, matching the existing `build_auth_config` fail-closed precedent in the same file.

## Deviations from Plan

None — plan executed exactly as written. Task 1's TDD RED state used a no-op stub (rather than a compile error) because the test names `ShutdownCoordinator`, a type that already exists (landed in plan 24-08) — the tests compile immediately but fail on real assertions against the stub, which is a genuine RED state, just not a compile-error one. This is noted here for transparency, not tracked as a Rule 1-4 deviation since it does not change any delivered behavior or its test coverage.

## Issues Encountered

None. `mdbook build docs` initially failed on a missing generated `docs/mermaid.min.js` (a `.gitignore`d asset regenerated by `mdbook-mermaid install` at build time, unrelated to this plan's edits) — running `mdbook-mermaid install docs` first resolved it, and the subsequent build reported zero broken links.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- HITL-04 is now fully wired end-to-end: engine mechanics (24-08) + process wiring and operator surface (this plan). Both process entry points cancel the same `ShutdownCoordinator` instance and wait `<= shutdown_grace` (skipped when `graceful_shutdown = false`) before completing shutdown; `resume` on a `Halted` thread is proven at the process-wiring level, not only inside the engine.
- Plan 24-10 (parallel wave) can register its background resume continuation with `shutdown_coordinator()` (paladin-server.rs's local variable is not yet exposed past `run()` — plan 24-10's own facade wiring is expected to construct or receive the coordinator per its own plan text, "coordinator is constructed here and handed forward").
- Plan 24-12 owns `MIGRATION.md` sections 9.2 and 9.6, deliberately untouched here.
- No blockers.

## Self-Check: PASSED

All 8 files verified present on disk; all 6 commit hashes (`68e13d68`, `ca81f6e0`, `66b7a265`, `861a1e8c`, `2d5b3ffe`, `a1fa207c`) verified present in `git log --oneline --all`.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
