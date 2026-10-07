---
phase: 42-mid-run-halt-sse-terminal-status
plan: 09
subsystem: run-worker-agent-kind
tags: [allowance, derived-budget, token-budget, worker, agent-kind, halted, model-unpriced, shared-service, treasurer-only]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: ADR-0057 group (e) and the option-b gate decision (42-01); HaltReason::wire_json, record-before-status, enqueue_webhook_delivery(.., halt_reason) and TraceEvent::RunFinished.halt_reason (42-02, 42-03, 42-05); Treasurer::derive_budget, admit_for_model, AdmissionError::ModelUnpriced, RunScope::with_derived_token_budget, StopReason::AllowanceHalted and the Treasurer-aware TokenBudget (42-07); the priced boot Treasurer and ApiError::model_unpriced (42-08)
provides:
  - shared_engine_execution_port in src/infrastructure/web/facade_provisioner.rs, the TokenBudget installed on the one shared run-engine service in Treasurer-only mode (operator figure forced off), so an engine node is never capped
  - run_agent in src/application/services/run/worker.rs re-derives the budget at dispatch, carries it on the RunScope and records an AllowanceHalted stop as Halted with the allowance_exhausted halt_reason and the partial output
  - The three no-call outcomes at dispatch (exhausted or zero figure, unreadable ledger, model that lost its price row) decided before the LLM is reached
  - RunSubmissionError::ModelUnpriced and POST /v1/runs admitting agent-kind assistants through admit_for_model with the resolved assistant's model, answered 422 model_unpriced
  - The agent-kind halt proven end to end over the real router (agent_kind_run_halts_on_the_derived_budget)
  - WINDOWS.md row 64 (D-08, agent-kind runs have no checkpoint), MIGRATION 9.2 and 9.6, platform-api.md, configuration.md, CHANGELOG, regenerated openapi.json
affects: [42-10, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "Re-derive at dispatch, never persist: the budget is read from the ledger when the worker starts the call, so an allowance spent while the run was queued is honoured"
    - "Treasurer-only mode on a shared service: the same TokenBudget middleware with the operator switch forced off acts only when a scope carries a derived figure"
    - "One halt writer (persist_agent_halt) shared by the post-call halt and the no-call halts: outcome with reason first, then status flip, then ack, then the halted webhook"
    - "A test-side shared_agent_port helper that uses the production constructor when web-server is on and a local adapter over the same middleware otherwise, so the worker is proven on both builds"

key-files:
  created: []
  modified:
    - src/infrastructure/web/facade_provisioner.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/run/submission.rs
    - src/application/services/run/schedule/service.rs
    - src/application/services/run/schedule/tests.rs
    - src/application/services/run/http_surface_tests.rs
    - src/infrastructure/web/run_api_wiring.rs
    - crates/paladin-ports/src/input/run_submission_port.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json
    - .planning/WINDOWS.md
    - docs/src/api-reference/platform-api.md
    - docs/src/getting-started/configuration.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "The budget is derived in run_agent after the allowance warnings and before RunStarted, so the three no-call outcomes emit RunStarted then one terminal RunFinished and never a NodeStarted/NodeFinished pair for a node that did not run"
  - "A refusal at dispatch halts with the binding ceiling's real figures; a backend error or any unrecognised AdmissionError halts ledger_unavailable (fail closed, the enum is non_exhaustive); ModelUnpriced records Failed naming the model (a deployment incoherence, not quota exhaustion)"
  - "The halt is recorded outcome first (reason, partial output, error null, no Waypoint), then Running -> Halted, then ack, then the halted webhook, the same order run_once uses for an engine halt (G14), so no reader sees halted without its reason"
  - "admit_and_persist takes the model from the resolved Runnable::Agent, never from the request; workflow assistants and fork pass None and keep today's admit (an agent-kind thread has no Waypoint to fork from)"
  - "A schedule-fired agent-kind run refused as ModelUnpriced is a SubmissionError skip that leaves skipped_ticks unchanged, exactly as the schedule's other non-allowance submission failures, never SkipReason::AllowanceExhausted"
  - "crates/paladin-web/src/thread_controller.rs needs no change: it maps submission errors through the same map_submission_error as crates/paladin-web/src/run_controller.rs, and fork never produces ModelUnpriced"
  - "ALLOW-03's continue-from-checkpoint is met on the engine path and not applicable by construction on the agent loop (D-08); recorded as WINDOWS.md row 64 in both representations"

patterns-established:
  - "A fixture that submits an agent-kind assistant under a ceiling must price its model: the new admission refuses an unpriced one 422"

requirements-completed: [ALLOW-03, ALLOW-05]

coverage:
  - id: D1
    description: "The shared run-engine service installs the TokenBudget in Treasurer-only mode: with the operator budget enabled at one token, a call whose scope carries no derived figure runs to max_loops with no cut and no notice, while the same installed middleware cuts a scope carrying a derived figure"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/facade_provisioner.rs#tests::shared_service_never_caps_an_engine_node (red when the enabled: false override was removed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A worker-dispatched agent-kind run whose re-derived budget is crossed is recorded Halted with the allowance_exhausted halt_reason, error null, the partial output and truncation notice kept and no checkpoint; the single terminal done and the halted webhook both carry the reason"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_budget::agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason (red when the derived budget was left off the RunScope)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Edge (ALLOW-03, boundary): an allowance spent while the run was queued, and a derived figure of zero tokens, record Halted with the binding ceiling's real balance and ceiling and never call the LLM (call count 0)"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_budget::agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm"
        status: pass
    human_judgment: false
  - id: D4
    description: "Edge (ALLOW-03, safety): an unreadable ledger at dispatch records Halted with ledger_unavailable and no LLM call, on the row, the terminal done and the halted webhook"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_budget::agent_kind_run_with_an_unreadable_ledger_at_dispatch_halts_ledger_unavailable_without_calling_the_llm"
        status: pass
    human_judgment: false
  - id: D5
    description: "Edge (ALLOW-05, boundary): a model that lost its price row between admission and dispatch records Failed with an error naming the model and treasurer.pricing, one terminal error event, a failed webhook, and no LLM call"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_budget::agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm"
        status: pass
    human_judgment: false
  - id: D6
    description: "Edge (precision): a run with no ceiling, and a run whose row records no submitter, complete as before with every loop run, zero allowance ledger reads, and no halt_reason key on the done or the webhook"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_budget::agent_kind_run_without_a_ceiling_completes_as_before"
        status: pass
    human_judgment: false
  - id: D7
    description: "POST /v1/runs admits an agent-kind assistant with the resolved assistant's model: an unpriced model is refused ModelUnpriced and a zero derived budget AllowanceExhausted, both before any run row or queue entry; workflow assistants keep the model-less admit"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/submission.rs#agent_model_admission_tests::submit_agent_kind_with_an_unpriced_model_is_refused_before_any_row, #submit_agent_kind_with_a_zero_derived_budget_is_refused_allowance_exhausted, #submit_workflow_assistant_keeps_admit_and_agent_kind_admits_with_its_model (all three red when the model was not passed)"
        status: pass
    human_judgment: false
  - id: D8
    description: "ModelUnpriced maps to 422 model_unpriced naming the model with no Retry-After; a schedule-fired run refused as unpriced is a SubmissionError skip, never an allowance skip, and the tick stays claimed"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#tests::map_submission_error_maps_model_unpriced_to_422; src/application/services/run/schedule/tests.rs#tick_with_an_unpriced_agent_model_is_a_submission_error_not_an_allowance_skip"
        status: pass
    human_judgment: false
  - id: D9
    description: "End to end over the real run_router, an on-disk SQLite run store and ledger, a real PaladinExecutionService behind the Treasurer-only wrapper and a config-priced Treasurer: an unpriced agent is refused 422 with no row, a priced agent is accepted, run_once cuts it after the crossing response, and GET /v1/runs/{id} answers halted, error null, halt_reason allowance_exhausted (api_key, lifetime, ceiling 0.0015 USD), final_waypoint_id null, the partial output on the stored row, and no key value in any body"
    requirement: "ALLOW-03"
    verification:
      - kind: e2e
        ref: "src/application/services/run/http_surface_tests.rs#agent_kind_run_halts_on_the_derived_budget (1 passed with and without the web-server feature; red when admission stopped passing the model)"
        status: pass
    human_judgment: false
  - id: D10
    description: "Edge (ALLOW-05, concurrency): two simultaneous submissions by one principal each derive from the same pre-admission balance, so the accepted admission race of D-01 applies to agent-kind runs as to the agent routes; not reproducible deterministically in a test"
    requirement: "ALLOW-05"
    verification:
      - kind: other
        ref: ".planning/decisions/0057-mid-run-halt-contract.md D-01 (accepted race; its WINDOWS.md row is owned by plan 42-12)"
        status: pass
    human_judgment: false
  - id: D11
    description: "D-08 is recorded, not silent: one WINDOWS.md row in both the markdown table and the trailing JSON block with the frontmatter counters bumped, plus the documented resume-by-resubmission behaviour"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: ".planning/WINDOWS.md row 64 (phase 42, kind deviation, status open; total_count 64, open_count 5)"
        status: pass
    human_judgment: false
  - id: D12
    description: "The change is registered and documented: MIGRATION 9.2 row and 9.6 paragraph, platform-api.md, configuration.md, CHANGELOG, regenerated openapi.json with the 422 on POST /runs; the frozen v0.9 golden gate needs no new exception and the public-API baseline is unchanged"
    requirement: "ALLOW-05"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0; cargo test -p paladin-web --test openapi_golden_v0_9 (9 passed); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface reports the surface unchanged (4209 items)"
        status: pass
    human_judgment: false

duration: ~66 min
completed: 2026-10-07
status: complete
---

# Phase 42 Plan 09: Agent-kind runs stop on the allowance Summary

**A worker-dispatched agent-kind run now re-derives its budget from the ledger at dispatch, runs under it through the shared service's Treasurer-only `TokenBudget`, and is recorded `Halted` with the `allowance_exhausted` reason and its partial output when it crosses; `POST /v1/runs` admits agent-kind assistants with their model (422 `model_unpriced`, 429 on a zero budget) before any row exists, and engine nodes are never capped.**

## Performance

- **Duration:** about 66 minutes
- **Completed:** 2026-10-07
- **Tasks:** 2
- **Files modified:** 16 (none created)

## Accomplishments

- **Treasurer-only mode on the shared service (G12).** `shared_engine_execution_port` in `src/infrastructure/web/facade_provisioner.rs` wraps the one shared service and installs `TokenBudget` with `enabled: false` (the operator's other fields carried), so the middleware acts only on a scope carrying a derived figure. `paladin_port_from_settings_with_ledger` uses it, and `EngineExecutionPort` is now `pub(crate)`. The rustdoc names ADR-0052's rejected hazard.
- **Re-derive at dispatch (G8, A8).** `run_agent` in `src/application/services/run/worker.rs` asks `Treasurer::derive_budget` after the allowance warnings and before `RunStarted`, only when the pool has a Treasurer and the row records a submitter. `Ok(Some(budget))` rides the `RunScope`; `Ok(None)` runs unbounded.
- **Three no-call outcomes.** A refusal halts with the binding ceiling's real figures; a backend error or any unrecognised error halts `ledger_unavailable` (logged at `error` with the run id only); `ModelUnpriced` records `Failed` with `model {model} has no treasurer.pricing row`. None calls the LLM.
- **One halt writer.** `persist_agent_halt` writes the outcome (reason, partial output or none, `error: null`, no Waypoint), flips `Running -> Halted`, acks, and enqueues the `halted` webhook with the reason. The post-call halt and the no-call halts share it, and each emits exactly one `RunFinished { Halted, halt_reason }` through the run's own dispatcher, so the live and replayed SSE `done` and the webhook carry the reason.
- **Front door.** `admit_and_persist` in `src/application/services/run/submission.rs` takes the resolved agent's model and calls `admit_for_model`; `RunSubmissionError::ModelUnpriced` is mapped to `422 model_unpriced` in `crates/paladin-web/src/run_controller.rs`, documented on the `POST /runs` `utoipa::path`, and handled in the schedule tick as a submission error.
- **End to end.** `agent_kind_run_halts_on_the_derived_budget` drives the real router, SQLite store, ledger, config-priced Treasurer and a real service over a scripted model, including the 422 leg.
- **Recorded and registered.** WINDOWS.md row 64 (D-08), MIGRATION 9.2 and 9.6, `docs/src/api-reference/platform-api.md`, `docs/src/getting-started/configuration.md`, CHANGELOG, regenerated `crates/paladin-web/openapi.json`.

## Task Commits

1. **Task 1: Treasurer-only shared service and the worker's agent-kind halt path** -- `1d99a40f` (feat)
2. **Task 2: POST /runs admits agent-kind assistants with their model, 422 mapping, end-to-end proof, D-08 window, registers** -- `401a3df0` (feat)

Plan metadata (SUMMARY, STATE, ROADMAP) is committed after this file; its hash is given in the orchestrator report.

## Decisions Made

- **The derivation point.** Deriving after the warnings and before `RunStarted` lets the three no-call outcomes emit `RunStarted` plus one terminal `RunFinished` and no `NodeStarted`/`NodeFinished` pair for a node that never ran, while the warnings still take the run's lowest `seq` values (D-18).
- **Failure classes at dispatch.** A refusal is a halt with real figures; a ledger fault (and any future unrecognised `AdmissionError`) is a halt `ledger_unavailable`, fail closed; an unpriced model is a `Failed` run with a typed error, because it is a deployment incoherence rather than quota exhaustion (D-10).
- **Order of writes.** Outcome with reason before the status flip, then ack, then webhook, matching `run_once`'s halting transition (G14).
- **Model source.** Admission uses the resolved assistant's model only. Fork passes no model (an agent-kind thread has no Waypoint to fork from) and workflow assistants keep `admit`.
- **Schedule tick.** An explicit `ModelUnpriced` arm logs the schedule id and model and returns `SkipReason::SubmissionError`, leaving `skipped_ticks` unchanged like the other non-allowance submission errors.
- **D-08.** Agent-kind runs write no Waypoint; resume is a fresh `POST /v1/runs`. Recorded as a known window rather than left silent.

## Deviations from Plan

None of Rules 1 to 4 required a change of design. The points below are departures from the letter of the plan or corrections found while executing.

1. **[Rule 1 - Bug, found by the workspace test run] Existing fixtures that submit an agent-kind assistant under a ceiling.** The new admission correctly refused the default `gpt-4` agent as unpriced in four existing tests (`src/application/services/run/schedule/tests.rs` `allowance::tick_fires_attributed_to_the_creator` and `allowance::removed_creator_key_is_gated_by_its_tenant_allowance_only`, `src/infrastructure/web/run_api_wiring.rs` `build_run_api_wires_the_allowance_warn_path` and `wired_treasurer_records_a_warn_notice_for_the_admitted_run`). Fixed by giving the fixtures a `gpt-4` price row (the schedule `Harness` and the `allowance_settings` helper). This is the intended D-10 behaviour surfacing in fixtures, not a product bug. Commit `401a3df0`.
2. **Worker tests run on default features.** The plan wraps the scripted service with `shared_engine_execution_port`, which lives in a `web-server`-gated module, but the plan's own verify commands run `cargo test -p paladin-ai --lib ...` without features. `shared_agent_port` in `src/application/services/run/worker_tests.rs` calls the production constructor when `web-server` is on and a local adapter installing the same middleware otherwise, and `src/application/services/run/http_surface_tests.rs` reuses it, so every worker and end-to-end test runs on both builds. Commits `1d99a40f`, `401a3df0`.
3. **`crates/paladin-web/src/thread_controller.rs` is unchanged.** The plan lists it conditionally ("only if it maps the variant distinctly"); it uses the shared `map_submission_error`, and fork cannot produce `ModelUnpriced`.
4. **Public-API baseline unchanged.** The plan lists `.project/current-exports.txt`; `RunSubmissionError` lives in a crate the `paladin` facade baseline does not enumerate and the new constructor is `pub(crate)`, so `make api-surface` reports the surface unchanged (4209 items) and no refresh was committed, as in 42-08.
5. **WINDOWS.md written through the tool.** `gsd-tools windows append` writes the table row, the JSON object and the counters together, producing exactly the shape commit 8954c18 used (row id 64, `open_count` 5, `total_count` 64). The description drops the plan's hyphen in "continue-from-checkpoint" to read `ALLOW-03 continue-from-checkpoint`.
6. **Commit trailer model name.** The dispatch notes name `Claude Fable 5.1`; the session's attribution reminder specifies `Claude Sonnet 5.5`, which matches the 42-01, 42-07 and 42-08 precedent. Both commits carry `Co-Authored-By: Claude Sonnet 5.5` and the session line; the orchestrator can re-trailer before push if it wants the dispatch wording.
7. **TDD commit shape.** Tests and implementation are committed together per task. Red-first was shown by mutation: removing the `enabled: false` override turned `shared_service_never_caps_an_engine_node` red, leaving the derived budget off the `RunScope` turned `agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason` red (both in one run), and making `submit` stop passing the model turned the two `submit_agent_kind_*` tests and the end-to-end test red. All were restored.
8. **Explicit schedule arm.** The existing catch-all arm already produced `SkipReason::SubmissionError`; the explicit `ModelUnpriced` arm adds a log line that names the schedule and model, with a test pinning the outcome.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-42-33 (agent-kind spend past the allowance via the worker) is mitigated by the dispatch re-derivation, the no-call halts and the end-to-end proof. T-42-34 (engine nodes capped by a shared budget) is mitigated by the Treasurer-only wrapper and `shared_service_never_caps_an_engine_node`. T-42-35 (unpriced agent model at `POST /runs`) is mitigated by the `422` before any row, with a test that no run row or queue entry exists. T-42-37 (disclosure): the `422` body carries the model name only and the end-to-end test asserts it contains neither the key value nor the tenant; the new dispatch log lines name the run id and the ledger error text only, and the manual credential-handling review found no key value in any new log or body. T-42-36 (no checkpoint) is accepted and recorded in `.planning/WINDOWS.md`.

## Gates and tests run

- `cargo test -p paladin-ai --lib application::services::run::worker`: 67 passed, including `agent_budget::agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason`, `agent_budget::agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm`, `agent_budget::agent_kind_run_with_an_unreadable_ledger_at_dispatch_halts_ledger_unavailable_without_calling_the_llm`, `agent_budget::agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm`, `agent_budget::agent_kind_run_without_a_ceiling_completes_as_before` and the existing `agent_kind_run_with_a_webhook_enqueues_a_delivery`.
- `cargo test -p paladin-ai --all-features --lib -- application::services::run::worker infrastructure::web::facade_provisioner engine_spend_halt_tracer`: 75 passed, including `infrastructure::web::facade_provisioner::tests::shared_service_never_caps_an_engine_node` and the worker tests over the production `shared_engine_execution_port`.
- `cargo test -p paladin-ai --lib application::services::run::submission`: 35 passed, including the three `agent_model_admission_tests` cases; `cargo test -p paladin-ai --lib agent_kind_run_halts_on_the_derived_budget -- --nocapture`: 1 passed (also run with `--all-features`).
- `cargo test -p paladin-ai --all-features --lib -- run_api_wiring schedule::tests`: every case passed except the two known sandbox-only cases (`build_run_api_persists_no_run_traces_by_default`, `build_run_api_persists_run_traces_when_trace_persist_is_set`), which need outbound network, are already in `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md`, and did not pass here for that reason alone.
- The orchestrator gate: `cargo build --workspace --all-features` built (exit 0); `cargo test --workspace --lib --bins --no-fail-fast` passed every crate except the two known cases above (1281 passed in the `paladin-ai` lib, 2 not passing for the network reason); `application::services::run::cancel_tests::local_cancel_signals_token` passed in the full suite.
- `cargo test -p paladin-web --lib --test openapi_golden_v0_9`: 293 and 9 passed; `POST /runs` is not in the frozen v0.9 document, so no new golden exception was needed. `crates/paladin-web/openapi.json` was regenerated with `make openapi`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo clippy -p paladin-ai --all-targets -- -D warnings` clean (default features, covering the local adapter branch); `cargo fmt --check` clean. `cargo check --workspace --all-targets --all-features` is covered by the all-targets clippy run (integration tests under `tests/` and `crates/*/tests` compile).
- `./scripts/check-migration-allowlist.sh` exit 0 (the new `RunSubmissionError` row is `N`, so no allowlist entry is added); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports the surface unchanged (4209 items).
- Not run: the Redis integration tests (no Redis here); the PostgreSQL contract legs (PostgreSQL is not running, so they self-skip and are not claimed); `make security` (no dependency was added or changed in this plan); `cargo-semver-checks` (not available locally; the added variant is under an existing `#[non_exhaustive]` enum).

## Self-Check: PASSED

- FOUND: commits `1d99a40f` and `401a3df0` in `git log`
- FOUND: `src/infrastructure/web/facade_provisioner.rs` contains `pub(crate) fn shared_engine_execution_port` and `enabled: false`
- FOUND: `src/application/services/run/worker.rs` contains `derive_budget(`, `with_derived_token_budget(` and `StopReason::AllowanceHalted`
- FOUND: `src/application/services/run/submission.rs` contains `admit_for_model(`; `crates/paladin-ports/src/input/run_submission_port.rs` contains `ModelUnpriced`
- FOUND: `crates/paladin-web/src/run_controller.rs` maps `RunSubmissionError::ModelUnpriced` before the catch-all arm of `map_submission_error`
- FOUND: `src/application/services/run/http_surface_tests.rs` contains `async fn agent_kind_run_halts_on_the_derived_budget`; `src/application/services/run/worker_tests.rs` contains `agent_kind_run_crossing_its_derived_budget_is_halted_with_the_reason`
- FOUND: `.planning/WINDOWS.md` row 64 (phase 42, description containing `D-08`) in the table and as the last object of the JSON block, with `total_count` 64 and `open_count` 5
