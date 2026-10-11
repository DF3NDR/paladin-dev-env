# Phase 44 deferred items

Out-of-scope discoveries logged during execution. Not fixed by the plan that found them.

## 44-01

- `cargo test --workspace --no-fail-fast` reports 2 failures in the root `paladin-ai` lib, both
  deterministic on re-run:
  - `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set`
  - `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default`

  Both panic at `src/infrastructure/web/run_api_wiring.rs:2369` with "the run never reached a
  terminal status" (a 10 s poll on an agent-kind run driven by a `FailingExecutor` through the Run
  API worker and `WarEngine`). That path touches neither Campaign, `aegis_attempt` nor
  `BattalionConfig`, so it is not caused by 44-01's changes as far as could be reasoned from the
  code. **Confirmed pre-existing by the orchestrator (2026-10-09):** with 44-01's five source files
  restored to the pre-phase baseline `061ee3dc` (and `aegis_attempt.rs` removed), the same two tests
  fail identically (`cargo test -p paladin-ai --lib --all-features run_api_wiring::tests::build_run_api_persists`,
  0 passed / 2 failed at `run_api_wiring.rs:2369`). Not a Phase 44 regression; owner of the Run API
  worker / `WarEngine` wiring to triage separately.

## Code review dispositions (44-REVIEW.md, 44-REVIEW-FIX.md), 2026-10-11

- **WR-02** Phalanx `with_max_concurrency(0)` hang: **fixed** (`229a28a5` RED, `cf0be3db` GREEN).
- **WR-01** cancelled Phalanx starting queued Paladins: **fixed** (`229a28a5` RED, `cf0be3db` GREEN).
- **WR-N1** (re-review) panic above `Semaphore::MAX_PERMITS`: **fixed** (RED then `dc55f18e`).
- **WR-03** removed v0.10 config keys silently ignored: **queued**, decision pending (operator leans to
  rejecting with a named error) -- `.planning/todos/pending/2026-10-11-reject-removed-v0-10-battalion-config-keys.md`.
- **WR-04, WR-05, IN-01..IN-05, IN-N1, IN-N2:** **queued** --
  `.planning/todos/pending/2026-10-11-phase-44-review-follow-ups.md`. Both todos carry
  `recheck_by: before the Phase 47 v0.11.0 publish`.
