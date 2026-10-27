---
phase: 42-mid-run-halt-sse-terminal-status
plan: 08
subsystem: web-agent-routes
tags: [allowance, derived-budget, token-budget, agent-routes, allowance-halted, model-unpriced, openapi-golden, option-b]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: ADR-0057 group (e) and the option-b gate decision (plan 42-01); HaltReason::wire_json (42-02); the derived budget, Admission::derived_budget, admit_for_model, AdmissionError::ModelUnpriced, Treasurer::with_pricing and the Treasurer-aware TokenBudget (42-07)
  - phase: 41-admission-time-allowance-enforcement
    provides: the admit_principal gate on the three agent routes, the 429 allowance_exhausted refusal
provides:
  - The one TokenBudget installed on every per-agent PaladinExecutionService (config-defined and runtime-provisioned), carrying the operator's agent_runtime.token_budget
  - build_run_api prices the Treasurer from treasurer.pricing so the agent routes derive budgets
  - stop_reason label allowance_halted and ExecuteResponse.halt_reason (HaltReason::wire_json), also on a job result and on the done data of a buffered execute/stream
  - admit_principal admits with the registered agent's model; ApiError::model_unpriced and 422 model_unpriced on execute, execute/stream and jobs
  - The derived budget on the RunScope of execute, execute/stream (both branches) and jobs
  - Option-b: a true streamed execute/stream done carries an informational halt_reason when the final chunk's total tokens strictly exceed the derived figure
  - The Phase 42 sanctioned exception in the frozen v0.9 golden gate (strip_known_v0_11_halt_reason) and the regenerated crates/paladin-web/openapi.json
  - MIGRATION 9.2 and 9.6 entries, configuration.md and platform-api.md sections, CHANGELOG entries
affects: [42-09, 42-10, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "One wire builder: ExecuteResponse.halt_reason, the buffered stream done, the job result and the true-stream crossing report all come from HaltReason::wire_json, never a second object"
    - "A golden-gate exception applied at document load (one function over a whole OpenAPI document) with an unstripped loader used only by the test that proves the exception's scope"
    - "Attributed scope helper plus per-route budget attach: the three handlers each add with_derived_token_budget from the one admission"

key-files:
  created: []
  modified:
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/infrastructure/web/run_api_wiring.rs
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/src/error.rs
    - crates/paladin-web/tests/openapi_golden_v0_9.rs
    - crates/paladin-web/openapi.json
    - docs/src/getting-started/configuration.md
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Option-b supersedes any plan wording that says a true stream's done stays byte-identical in every case: a true stream's done is byte-identical only when the final chunk's total tokens do not strictly exceed the derived figure (or no usage was reported); when they do, it additionally carries an informational halt_reason built by HaltReason::wire_json. It reports a crossing, never a halt the server performed"
  - "The crossing test is strict greater-than on total tokens against DerivedTokenBudget.max_tokens, the same strict test TokenBudget applies, so a count exactly at the figure is not a crossing"
  - "A final chunk with no reported usage is never a crossing (D-17: nothing is fabricated)"
  - "The agent's model for admission is entry.paladin.node.model, the registered agent's, never anything from the request"
  - "FacadeProvisioner carries the operator token budget through a new with_token_budget builder that from_settings sets, so paladin-server's runtime provisioning installs the same figure as the boot registry"
  - "The v0.9 golden exception is one function applied to a whole document at load time (generated_spec and load_baseline), with unstripped loaders used only by the test that proves it removes the two additions and nothing else"
  - "The ExecuteResponse struct doc comment is left untouched; the new notes are a plain comment because utoipa copies the doc comment into the schema description and the golden gate compares it"

patterns-established:
  - "attributed_scope(principal, admission) as the shared start of every agent route's RunScope; each route attaches the derived budget itself"

requirements-completed: [ALLOW-05, ALLOW-03]

coverage:
  - id: D1
    description: "build_agent_with_llm and build_agent install the one TokenBudget with the operator's figure on the shared per-agent service; with the default config and no derived figure the run is not cut, a derived figure ends it with AllowanceHalted, and an enabled operator figure below the usage ends it with TokenBudget (D-11, G12)"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "src/infrastructure/web/agent_host.rs#tests::built_agent_without_any_budget_runs_as_before, #built_agent_cuts_a_derived_budget, #built_agent_honours_the_operator_budget (two of the three were red when the installation line was removed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "build_run_api builds the Treasurer with the price table: a priced model derives a budget through admit_for_model, an unpriced model under a ceiling is refused ModelUnpriced, and an invalid price row aborts boot naming its config path (D-09)"
    requirement: "ALLOW-05"
    verification:
      - kind: integration
        ref: "src/infrastructure/web/run_api_wiring.rs#tests::build_run_api_prices_the_treasurer, #build_run_api_rejects_an_invalid_price_row"
        status: pass
    human_judgment: false
  - id: D3
    description: "stop_reason_label maps AllowanceHalted to allowance_halted; ExecuteResponse.halt_reason equals HaltReason::AllowanceExhausted(..).wire_json() only for that stop reason and the key is absent from the serialized body for every other stop reason"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::stop_reason_labels_are_stable, #stop_reason_label_names_the_allowance_halt, #execute_response_carries_halt_reason_only_on_allowance_halted, #execute_answers_allowance_halted_with_the_halt_reason"
        status: pass
    human_judgment: false
  - id: D4
    description: "Edge (ALLOW-05, boundary): an unpriced model under a ceiling is refused 422 model_unpriced on execute, execute/stream and jobs before the agent runs, naming the model, with no Retry-After header, no key or tenant in the body and no job issued"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::unpriced_model_is_refused_422_on_execute_stream_and_jobs; cargo test -p paladin-web --doc model_unpriced"
        status: pass
      - kind: e2e
        ref: "src/infrastructure/web/agent_host.rs#tests::agent_execute_halts_on_the_derived_budget (leg 4, real router and real Treasurer)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Edge (ALLOW-05, boundary, precision): a derived budget of zero tokens is refused 429 allowance_exhausted over the real router with the binding ceiling's figures, and a principal with no ceiling is unaffected on a priced and on an unpriced model"
    requirement: "ALLOW-05"
    verification:
      - kind: e2e
        ref: "src/infrastructure/web/agent_host.rs#tests::agent_execute_halts_on_the_derived_budget (legs 3 and 5)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The derived budget rides the RunScope on execute, the true stream, the buffered fallback and jobs, and a call with no derived budget leaves the scope's field unset"
    requirement: "ALLOW-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::every_agent_route_carries_the_derived_budget_on_its_run_scope, #no_derived_budget_leaves_the_run_scope_budget_unset"
        status: pass
    human_judgment: false
  - id: D7
    description: "End to end over the real agent router with a real build_agent_with_llm agent, a scripted model and a priced Treasurer: execute answers 200 with allowance_halted, a halt_reason with reason allowance_exhausted, the partial output and the truncation notice; a smaller operator budget answers token_budget with no halt_reason; jobs records the same halt on its result; no key value reaches any body"
    requirement: "ALLOW-05"
    verification:
      - kind: e2e
        ref: "src/infrastructure/web/agent_host.rs#tests::agent_execute_halts_on_the_derived_budget (cargo test -p paladin-ai --all-features --lib agent_execute_halts_on_the_derived_budget: 1 passed)"
        status: pass
    human_judgment: false
  - id: D8
    description: "D-12 stream clause, buffered: the buffered fallback of execute/stream carries stop_reason allowance_halted and the same halt_reason in its done data, and a non-halt fallback done keeps its exact shape with no halt_reason key; the job result records the halt"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::execute_stream_buffered_fallback_done_carries_the_halt_reason, #jobs_record_allowance_halted_on_the_job_result"
        status: pass
    human_judgment: false
  - id: D9
    description: "D-12 stream clause, true stream (option-b): the done is byte-identical to a stream with no budget when the final chunk's total tokens are under or exactly at the derived figure or no usage is reported, and carries the informational halt_reason when they strictly exceed it; the strict comparison was shown red by a mutation to greater-or-equal"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::execute_stream_true_stream_done_is_byte_identical_without_a_crossing, #execute_stream_true_stream_done_carries_halt_reason_when_usage_crossed"
        status: pass
    human_judgment: false
  - id: D10
    description: "Edge (ALLOW-05, concurrency): two simultaneous calls by one principal each derive from the same pre-admission balance, so the accepted admission race of D-01 applies to the agent routes exactly as to 42-07's derivation; not reproducible deterministically in a test"
    requirement: "ALLOW-05"
    verification:
      - kind: backstop
        ref: ".planning/decisions/0057-mid-run-halt-contract.md D-01 (accepted race; its WINDOWS.md row is owned by plan 42-12)"
        status: pass
    human_judgment: false
  - id: D11
    description: "The frozen v0.9 golden gate passes with exactly one new documented exception that removes only the 422 entries on the three operations and halt_reason from ExecuteResponse; the committed crates/paladin-web/openapi.json matches the generated document"
    requirement: "ALLOW-05"
    verification:
      - kind: integration
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs#phase_42_exception_is_narrowly_scoped (cargo test -p paladin-web --test openapi_golden_v0_9: 9 passed); crates/paladin-web/src/openapi.rs#openapi_matches_committed_baseline"
        status: pass
    human_judgment: false
  - id: D12
    description: "The HTTP agent cutoff, its two additive wire changes and the operator-budget behaviour change are registered and documented: MIGRATION 9.2 and 9.6, configuration.md, platform-api.md, CHANGELOG; the public-API baseline is unchanged"
    requirement: "ALLOW-05"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface reports the surface unchanged (4209 items)"
        status: pass
    human_judgment: false

duration: ~45 min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 08: The HTTP agent cutoff, allowance_halted and model_unpriced Summary

**Every per-agent service now carries the one `TokenBudget`, the three HTTP agent routes admit with the agent's model and hand the derived budget to the run, a crossed budget answers `stop_reason: "allowance_halted"` with a `halt_reason` object (on execute, jobs and a buffered stream's `done`, plus an informational crossing report on a true stream under option-b), unpriced models are refused `422 model_unpriced`, and the frozen v0.9 golden gate passes with one documented exception.**

## Performance

- **Duration:** about 45 minutes
- **Completed:** 2026-10-06
- **Tasks:** 3
- **Files modified:** 11 (none created)

## Accomplishments

- Installed the cutoff everywhere an agent is built. `build_agent_with_llm` and `build_agent` take the operator's `agent_runtime.token_budget` and install `TokenBudget::new(..)` on the shared service before it is split into the buffered and streaming handles; `build_agent_registry_with_ledger` and the runtime `FacadeProvisioner` (through the new `with_token_budget`, set by `from_settings`) pass it. The operator figure now caps HTTP-served agents when enabled, as the docs always said; a deployment with it disabled and no allowance is unchanged.
- Priced the boot Treasurer: `build_run_api` builds it `with_pricing` from `treasurer.pricing`, so the agent state's admission port derives budgets.
- Wired the routes. `admit_principal` calls `admit_for_model` with the registered agent's model, maps `ModelUnpriced` to the new `ApiError::model_unpriced` (`422`, `details.model`, no `Retry-After`) and returns the `Admission`; `execute`, `execute/stream` (true stream and buffered fallback) and `jobs` attach `with_derived_token_budget` to their `RunScope`. The `422` is declared on the three `utoipa::path` blocks.
- Reported the halt on the wire through the one builder: `stop_reason_label` gained `allowance_halted`, `ExecuteResponse.halt_reason` is `HaltReason::AllowanceExhausted(..).wire_json()` for that stop reason only, and the same field carries D-12's stream clause on a buffered stream (its `done` is the serialized `ExecuteResponse`) and on a job result.
- Implemented the option-b true-stream leg: `chunk_to_event` takes the derived budget and, when the final chunk's reported `total_tokens` strictly exceeds `max_tokens`, adds the informational `halt_reason` to `{ "done": true, "usage" }`. No-crossing streams are byte-identical.
- Kept the v0.9 golden gate honest: `strip_known_v0_11_halt_reason` removes exactly the three `422` entries and `ExecuteResponse.halt_reason`, `phase_42_exception_is_narrowly_scoped` fails if it widens, and `crates/paladin-web/openapi.json` was regenerated with `make openapi`.
- Registered and documented the surface (MIGRATION 9.2 and 9.6, configuration.md, platform-api.md, CHANGELOG).

## Task Commits

1. **Task 1: the one `TokenBudget` on every per-agent service, and a priced Treasurer at boot** -- `92537068` (feat)
2. **Task 2: routes admit with the model, carry the derived budget, answer allowance_halted, refuse unpriced models; golden exception; end-to-end proof** -- `c9b72bfe` (feat)
3. **Task 3: MIGRATION 9.2 and 9.6, configuration.md, platform-api.md, CHANGELOG** -- `fab22701` (docs)

Plan metadata (SUMMARY, STATE, ROADMAP) is committed after this file; its hash is given in the orchestrator report.

## Decisions Made

- **Option-b is the recorded gate outcome, and it supersedes the "true stream stays byte-identical in all cases" reading.** The plan text offers both options and, for option-a, says a true stream's `done` stays `{ "done": true, "usage" }` byte for byte even when its usage crossed the figure. The operator selected option-b at the plan 42-01 design gate (`.planning/phases/42-mid-run-halt-sse-terminal-status/42-01-SUMMARY.md`, `## Checkpoint decision`; ADR-0057 group (e)), so that wording is superseded: the true stream is byte-identical only when it did not cross; when it did, `done` additionally carries the informational `halt_reason`. There is still no behavioural halt on that path: the call has already finished and the server never cuts a true stream.
- **Scope of D-12's stream clause: both stream kinds.** The buffered fallback is proven by `crates/paladin-web/src/agent_controller.rs#tests::execute_stream_buffered_fallback_done_carries_the_halt_reason`; the true stream by `#execute_stream_true_stream_done_carries_halt_reason_when_usage_crossed` (crossing) and `#execute_stream_true_stream_done_is_byte_identical_without_a_crossing` (under the figure, exactly at it, and no reported usage). The G2 row in WINDOWS.md (plan 42-12) still records that a single streamed call is not cut mid-flight.
- **Strict comparison, no fabrication.** The crossing test is `total_tokens > max_tokens`, the same strict test `TokenBudget` applies; a count exactly at the figure continues. A final chunk with no usage is not a crossing.
- **The golden exception lives at document level.** One function, `strip_known_v0_11_halt_reason`, runs inside `generated_spec()` and `load_baseline()` on the whole OpenAPI document; unstripped loaders exist only for the test that proves its scope. The plan's literal "called beside the existing strippers" reads equally well as this single call point.
- **`ExecuteResponse` documentation stays as it was.** A first draft extended the struct's rustdoc, and the golden gate immediately reported the schema `description` difference (utoipa copies the doc comment). The additions are a plain comment above the derive; only the new field carries rustdoc, and it is stripped from both documents by the exception.

## Deviations from Plan

None of Rules 1 to 4 required a change of design; the points below are departures from the letter of the dispatch or the plan text, or small corrections found while executing.

1. **[Rule 1 - Bug, found by the gate] Schema description drift.** Extending the `ExecuteResponse` struct doc comment changed the published `description` of a frozen v0.9 schema, which `ref_closure_schemas_match_the_frozen_baseline` caught. Fixed by moving the notes to a plain comment (Decisions above). Commit `c9b72bfe`.
2. **Commit trailer model name.** The dispatch notes name `Claude Fable 5.1`; the session's attribution reminder specifies `Claude Sonnet 5.5`, which is also the actual model. All three task commits carry `Co-Authored-By: Claude Sonnet 5.5` and the session line, matching the 42-01 and 42-07 precedent. The orchestrator can re-trailer before push if it wants the dispatch wording.
3. **Invalid-price boot test names the existing error.** `build_run_api` builds the run engine's LLM port, which checks the same price table first, so an invalid row is reported by that earlier existing error (`failed to build the run engine's LLM port: ... treasurer.pricing.gpt-4.prompt`), not by the Treasurer's own `invalid treasurer configuration` mapping. The new mapping stays as the defensive path for any ordering change; `src/infrastructure/web/run_api_wiring.rs#tests::build_run_api_rejects_an_invalid_price_row` asserts the boot error naming the offending path.
4. **Public-API baseline not changed.** The plan lists `.project/current-exports.txt` and says to refresh it. The baseline lists the `paladin` facade's default surface and does not include the `web-server` module where `FacadeProvisioner::with_token_budget` lives (the facade items in `crates/paladin-web` are likewise outside it), so `make api-surface-update` changed only the generated-at timestamp line; that was reverted as drift noise and `make api-surface` reports the surface unchanged.
5. **`FacadeProvisioner::with_token_budget`.** A small public builder added so the runtime provisioner can carry the operator figure (the plan said it "passes the same settings value"); `from_settings` sets it, and `src/bin/paladin-server.rs` constructs the provisioner through `from_settings`. Registered in the CHANGELOG `### Changed` entry (the crate-private `build_agent*` signatures need no register row).
6. **TDD commit shape.** Tests and implementation are committed together per task (the plan's per-task granularity). Red-first was shown by mutation for the two installation tests (installation line removed: the derived and operator tests went red, the no-budget test stayed green) and for the true-stream boundary (greater-or-equal turned the byte-identical test red); the e2e test was written against the finished wiring.
7. **Helper instead of three inline scope blocks.** A shared `attributed_scope` builds the ledger scope and warnings; each of the three handlers attaches `with_derived_token_budget` itself, so the plan's literal "at least three call sites" holds (the count is 3).

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-42-28 (an unpriced model run for a ceilinged principal) is mitigated by the `422` before the agent runs on all three routes, with an end-to-end proof. T-42-31 (disclosure through `halt_reason` and the `422`): both are built from the binding ceiling's own figures and the model name; the new tests assert the raw `422` and halt bodies contain no key id, key value or tenant, and no new log line was added, so the manual credential-handling review found nothing to fix. T-42-30 (the operator budget newly effective) is accepted and documented in configuration.md and the CHANGELOG `### Changed` entry.

## Handoff notes for plan 42-09

- `build_agent_with_llm` now takes `TokenBudgetConfig`; the worker's shared engine service is separate and still has no `TokenBudget` (42-09 installs it in Treasurer-only mode with the operator figure forced off, per ADR-0057 D-11).
- `Treasurer` built in `build_run_api` already carries the price table, so the worker's re-derivation at dispatch (`admit_for_model` or `derive_budget`) can use it directly.
- `RunSubmissionError::ModelUnpriced` is not mapped yet; `AdmissionError::ModelUnpriced` is mapped to `ApiError::model_unpriced` only on the three agent routes.

## Gates and tests run

- `cargo test -p paladin-ai --all-features --lib infrastructure::web::agent_host`: 18 passed, including `built_agent_without_any_budget_runs_as_before`, `built_agent_cuts_a_derived_budget`, `built_agent_honours_the_operator_budget`, `build_agent_with_llm_settles_priced_calls_when_a_ledger_is_installed` and `agent_execute_halts_on_the_derived_budget` (the last also run alone: 1 passed).
- `cargo test -p paladin-ai --all-features --lib run_api_wiring`: the new `build_run_api_prices_the_treasurer` and `build_run_api_rejects_an_invalid_price_row` pass; every other test in the module passes except the two known sandbox-only cases (`build_run_api_persists_no_run_traces_by_default`, `build_run_api_persists_run_traces_when_trace_persist_is_set`), which need outbound network, are already in `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md`, and did not pass here for that reason alone.
- `cargo test -p paladin-ai --all-features --lib infrastructure::web::facade_provisioner`: 6 passed.
- `cargo test -p paladin-web --lib`: 292 passed (including the 11 new `crates/paladin-web/src/agent_controller.rs` tests listed in the coverage block and `openapi_matches_committed_baseline`); `cargo test -p paladin-web --doc`: 2 passed, including the `model_unpriced` doctest.
- `cargo test -p paladin-web --test openapi_golden_v0_9`: 9 passed, including the new `phase_42_exception_is_narrowly_scoped`.
- The orchestrator gate: `cargo build --workspace --all-features` built; `cargo test --workspace --lib --bins` with the keep-going flag passed every crate except the two known cases above in `src/infrastructure/web/run_api_wiring.rs` (1270 passed in the `paladin-ai` lib, 2 not passing for the network reason); `application::services::run::cancel_tests::local_cancel_signals_token` passed in the full suite.
- `cargo check --workspace --all-targets --all-features` clean (integration tests under `tests/` and `crates/*/tests` compile); `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean.
- `./scripts/check-migration-allowlist.sh` exit 0 (set-equal, no allowlist change; the `ExecuteResponse` row was extended and no duplicate entry added); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports the surface unchanged (4209 items).
- Documentation greps: `tightest` appears in `docs/src/getting-started/configuration.md` (3 lines); `allowance_halted`, `halt_reason` and `model_unpriced` appear in `docs/src/api-reference/platform-api.md`; section 9.6 of `MIGRATION.md` names `model_unpriced` and `allowance_halted`.
- Mutation checks: removing the `TokenBudget` installation line turned `built_agent_cuts_a_derived_budget` and `built_agent_honours_the_operator_budget` red; changing the strict comparison to greater-or-equal turned `execute_stream_true_stream_done_is_byte_identical_without_a_crossing` red. Both were restored.
- Not run: the Redis integration tests (no Redis here); the PostgreSQL contract legs (PostgreSQL is not running, so they self-skip and are not claimed); `make security` (no dependency was added or changed in this plan); `cargo-semver-checks` (not available locally; the one `Y` row extended is already covered by the existing allowlist entry for `paladin-web | ExecuteResponse`).

## Self-Check: PASSED

- FOUND: commits `92537068`, `c9b72bfe` and `fab22701` in `git log`
- FOUND: `src/infrastructure/web/agent_host.rs` contains `TokenBudget::new(` and `agent_runtime.token_budget`; `src/infrastructure/web/run_api_wiring.rs` contains `with_pricing(`
- FOUND: `crates/paladin-web/src/agent_controller.rs` contains `"allowance_halted"`, `admit_for_model(` and three `with_derived_token_budget(` call sites; `crates/paladin-web/src/error.rs` contains `pub fn model_unpriced`; `crates/paladin-web/tests/openapi_golden_v0_9.rs` contains `fn strip_known_v0_11_halt_reason` and a `Phase 42` module-doc paragraph
