# Phase 42 deferred items

Out-of-scope discoveries logged during execution (not fixed by the plan that found them).

## Found during 42-02 (2026-10-06)

- `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and
  `build_run_api_persists_run_traces_when_trace_persist_is_set` (both need
  `cargo test -p paladin-ai --lib --features web-server`) fail in this sandbox with "the run never
  reached a terminal status" after the 10 s poll. They submit an agent-kind run whose model call
  goes out to a real provider with a hermetic fake key; the sandbox has no direct network. Confirmed
  independent of 42-02: the failure reproduces with `run_api_wiring.rs` reverted to its pre-plan
  content. Every other `run_api_wiring` test (24) passes. Not touched by this plan; CI (network
  available) is the authority.
- The doc comment at the top of `src/application/services/treasurer/tests.rs` (and ADR-0056's
  Downstream Consumers line, already carrying the 42-01 dated note) still says Phase 42 closes the
  over-admission race with a reservation at the superstep boundary; ADR-0057 supersedes that
  (check-only boundary, race accepted). Plan 42-12's closeout is the natural place to reword it.

## Found during 42-06 (2026-10-06)

- `paladin-eval`'s `RunStatusValue` (`crates/paladin-eval/src/scenario.rs`) mirrors
  `RunFinishStatus` field for field and has no `Cancelled` value, so an eval scenario cannot assert
  `run_status: cancelled` now that the engine reports a caller cancel as `Cancelled`
  (`run_status_matches` uses `matches!`, so a cancelled finish simply matches no expected value).
  Adding the value changes the generated scenario JSON schema and `paladin-eval`'s public surface,
  neither in 42-06's file scope; no shipped scenario cancels a run. A later plan or phase can add
  `RunStatusValue::Cancelled` with its schema snapshot.
