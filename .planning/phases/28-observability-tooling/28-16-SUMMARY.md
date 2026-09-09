---
phase: 28-observability-tooling
plan: 16
subsystem: testing
tags: [paladin-eval, e2e-dogfood, scenario-format, libtest-mimic, cli, determinism]

# Dependency graph
requires:
  - phase: 28-12
    provides: "ScenarioRunner (register_graph, register_registries, run_case, trials), ScriptedPorts/GraphConstructor/RegistriesFactory, ScenarioPaladinPort, the eval_scenarios! macro, the empty tests/evals.rs [[test]] target, and paladin-cli eval run --repeat/--bless/--registries"
  - phase: 28-08
    provides: "The twelve OBS-FR-12 assertion evaluators (node_executed, route_taken, run_status, supersteps_max, final_state_snapshot, ...) over AssertionContext"
  - phase: 28-05
    provides: "The .eval.yaml scenario file format (Scenario/ScenarioTarget/StoreKind/Case/LlmScript/Assertion) and ScenarioLlm"
provides:
  - "tests/helpers/e2e_fixtures.rs: build_crash_resume_graph, build_approval_gate_graph, build_muster_defer_order_graph[_with_a_template_per_task] -- the three E2E graph builders moved verbatim out of the integration tests, shared by both the integration tests and the eval harness"
  - "evals/e2e-1-crash-resume.eval.yaml, evals/e2e-2-approval-gate.eval.yaml, evals/e2e-3-map-reduce-fault-tolerance.eval.yaml -- the program's own three acceptance scenarios (E2E-1/2/3) expressed in the harness format, green under a default cargo test and 20-of-20 deterministic under --repeat"
  - "evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json -- the blessed final-state snapshot"
  - "paladin_eval::eval_scenarios!'s second, registration-closure macro arm plus run_eval_harness_main_with -- the mechanism a harness=false [[test]] binary needs to populate a ScenarioRunner with registered targets before globbing"
  - "src/application/cli/commands/eval.rs's own e2e_scenario_runner() -- the facade's eval registry (anticipated but left unfilled by 28-12), so paladin-cli eval run resolves the same registered targets tests/evals.rs does"
  - "Three real runner/CLI bugs found and fixed by this dogfood exercise (see key-decisions): a legacy LlmError string-erasure that broke scripted-retry transience classification, a stale-first-RunFinished-record bug in run_status/total_tokens_max/supersteps_max for any multi-call (parley-answering) case, and two --repeat false-divergence sources (a volatile WaypointSaved fingerprint, and strict positional comparison of genuinely-concurrent muster-fanout trace records)"
affects: ["28-17 (MIGRATION.md, ADR updates, crate/feature inventory: paladin-eval's eval_scenarios! macro gained a second arm; the CLI's own e2e_fixtures module and e2e_scenario_runner() are new, cli-feature-gated surface)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One source of truth for a graph builder shared across THREE compilation units (tests/integration/*_test.rs, tests/evals.rs, and the cli-feature-gated src/application/cli/commands/eval.rs) via #[path] inclusion of the same tests/helpers/e2e_fixtures.rs file from all three -- no crate-boundary change, no duplicated logic; paladin_battalion/paladin_core/paladin_ports/async-trait are all unconditional (never feature-gated) facade dependencies, so the shared file compiles identically wherever it's included"
    - "eval_scenarios!(pattern, |runner: &mut ScenarioRunner| { ... }) -- a second macro arm mirroring the existing single-arg form, letting a harness=false [[test]] binary register graph constructors/registries factories before the pattern is globbed, without touching the existing single-arg callers"
    - "A --repeat divergence detector must be order-insensitive for genuinely concurrent trace records (same-superstep muster fan-out): stabilized_fingerprints sorts each maximal contiguous run of same-superstep NodeStarted/NodeFinished fingerprints before comparison, while every other record (including cross-superstep order) still compares strictly -- content differences still diverge, only harmless interleaving does not"

key-files:
  created:
    - tests/helpers/e2e_fixtures.rs
    - evals/e2e-1-crash-resume.eval.yaml
    - evals/e2e-2-approval-gate.eval.yaml
    - evals/e2e-3-map-reduce-fault-tolerance.eval.yaml
    - evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json
  modified:
    - tests/helpers/mod.rs
    - tests/integration/e2e_crash_resume_test.rs
    - tests/integration/e2e_approval_gate_test.rs
    - tests/integration/e2e_muster_defer_order_test.rs
    - tests/evals.rs
    - crates/paladin-eval/src/runner.rs
    - crates/paladin-eval/src/assertion.rs
    - src/application/cli/commands/eval.rs

key-decisions:
  - "Plan-file correction: the plan's files_modified/interfaces named tests/integration/e2e_compensation_chain_test.rs as E2E-3's source file. That file's actual content (verified by reading it) is a DIFFERENT, unrelated PRD-04 scenario (a compensation-chain/ask-a-human-on-payment-failure fixture, book/cancel/payment nodes) -- nothing about a planner mustering five workers. The real E2E-3 muster/defer/order + recovering-worker fixture lives in tests/integration/e2e_muster_defer_order_test.rs, used instead. e2e_compensation_chain_test.rs was left completely untouched (out of this plan's actual scope)."
  - "E2E-3's eval scenario registers the per-template (w1..w5) recovering-worker graph builder, not the single shared-\"worker\"-node-id builder the plan's <behavior> text describes. A single shared worker template dispatched for five concurrent muster tasks cannot deterministically script ONE specific task's failure through either FaultyPaladinPort (keyed by Paladin name) or ScenarioLlm (keyed by node id) -- all five tasks would share one script sequence consumed in real, OS-scheduling-dependent call order. The per-template design (already how the integration test's OWN recovering-worker case is built, Phase 25/FT-FR-06/D-31) gives w3 an addressable identity. Asserted as five separate exact per-node node_executed counts (w1=1, w2=1, w3=3, w4=1, w5=1, summing to seven dispatches) instead of a single node_executed{node:\"worker\",times:{min:7}} assertion against a node id this graph shape doesn't have."
  - "Fixed ScenarioPaladinPort::execute (crates/paladin-eval/src/runner.rs, outside this plan's stated files_modified but required for Task 2's own acceptance criteria) to route every LlmError through paladin_battalion::llm_failure::to_paladin_error instead of the legacy PaladinError::LlmError(source.to_string()) string erasure. The legacy erasure classifies EVERY scripted error as Transience::Unknown, which the DEFAULT TransientOnly retry predicate never retries -- so no eval scenario could ever exercise a real Aegis retry recovery before this fix, silently defeating exactly what E2E-3 needs to dogfood. Discovered because E2E-3's w3 never got retried until this fix landed."
  - "Fixed AssertionContext::run_finished() (crates/paladin-eval/src/assertion.rs) to find the LAST TraceRecord::RunFinished, not the first (find_map -> rev().find_map()). Any case with case.parley_responses drives the engine through MORE than one top-level call (start, then one-or-more resume_with), and each call emits its own RunFinished record; the first is necessarily AwaitingInput (that's why the run paused), and only the last reflects the true terminal outcome. Discovered because E2E-2's approve/deny cases both completed correctly (verified via a throwaway debug trace dump) yet every run_status/route_taken-adjacent assertion read the stale first record."
  - "Fixed record_fingerprint's WaypointSaved arm and rewrote first_divergence/added stabilized_fingerprints (src/application/cli/commands/eval.rs) to fix two real --repeat false-divergence sources, both discovered running the plan's own required 20-repeat check: (a) WaypointSaved's fingerprint embedded the FULL pre-rendered status string for an AwaitingInput waypoint, including the random parley_id and wall-clock created_at the function's own doc comment already promises to exclude -- normalized to just the leading status-variant tag; (b) the engine dispatches every muster task as an independent, unlinked async task in the SAME superstep (CF-03), so which sibling's NodeStarted/NodeFinished record lands first in the captured trace is real OS-scheduling nondeterminism, not a behavioural difference (proven correct by the engine's own worker_deltas_merge_in_task_key_order_not_completion_order test) -- stabilized_fingerprints sorts each maximal contiguous same-superstep NodeStarted/NodeFinished run by its own fingerprint text before comparison (still detects a genuine content divergence, e.g. \"b\" vs \"c\"; only harmless interleaving of otherwise-identical work stops being reported). Verified the existing tests/cli/eval_run_test.rs::repeat_divergence_exits_nonzero_and_names_the_seq_range test (injected divergence) still passes unchanged."
  - "The approval-gate graph_doc twin (Claude's-discretion item in 28-CONTEXT.md: whether E2E-2 also carries a WarGraphDoc-format twin as a doc-expressible format demonstration) was skipped -- E2E-2's `registered` scenario already dogfoods the harness's shared-Rust-builder path; the twin would demonstrate the graph_doc/schemars path, a real but separate concern 28-05/28-12 already covered with their own graph_doc-target tests, and building/registering a second document for a discretionary demonstration was not worth the added surface given this plan's own scope was already extended by three genuine bug fixes."

patterns-established:
  - "e2e_fixtures_are_the_only_definition: grep -c 'pub fn' tests/helpers/e2e_fixtures.rs plus a `grep -c e2e_fixtures` check in each integration test file are how a future audit confirms no graph-building code re-duplicated back into the integration tests."

requirements-completed: [OBS-04]

coverage:
  - id: D1
    description: "The three E2E graph builders are extracted verbatim into tests/helpers/e2e_fixtures.rs and used by both the integration tests (unchanged assertions, same pass counts) and the eval harness's registered constructors"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "cargo test --test e2e_crash_resume (32/32 passed, same as before extraction)"
        status: pass
      - kind: integration
        ref: "cargo test --test e2e_approval_gate (35/35 passed, same as before extraction)"
        status: pass
      - kind: integration
        ref: "cargo test --test e2e_muster_defer_order (37/37 passed, same as before extraction)"
        status: pass
    human_judgment: false
  - id: D2
    description: "E2E-1 (crash-resume) exists as evals/e2e-1-crash-resume.eval.yaml with sqlite_temp + interrupt_after_superstep: 3, exact post-resume node-execution counts, supersteps_max, and a final_state_snapshot matching the blessed control-run snapshot"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "cargo test --test evals e2e-1-crash-resume::crash_after_superstep_3"
        status: pass
      - kind: other
        ref: "paladin-cli eval run 'evals/*.eval.yaml' --repeat 20 -- crash_after_superstep_3: 20/20 passed, no divergence"
        status: pass
    human_judgment: false
  - id: D3
    description: "E2E-2 (approval gate) exists as evals/e2e-2-approval-gate.eval.yaml with approve/deny cases via parley_responses and route_taken assertions on both branches"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "cargo test --test evals e2e-2-approval-gate::approve, e2e-2-approval-gate::deny"
        status: pass
      - kind: other
        ref: "paladin-cli eval run 'evals/*.eval.yaml' --repeat 20 -- approve/deny: 20/20 passed each, no divergence"
        status: pass
    human_judgment: false
  - id: D4
    description: "E2E-3 (map-reduce fault-tolerance) exists as evals/e2e-3-map-reduce-fault-tolerance.eval.yaml scripting the recovering worker's two transient errors then success, asserting exact per-worker execution counts (summing to the plan's seven-execution bar), the aggregator running exactly once, and the aggregated field holding all 5 results in task_key order"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "cargo test --test evals e2e-3-map-reduce-fault-tolerance::recovering_worker"
        status: pass
      - kind: other
        ref: "paladin-cli eval run 'evals/*.eval.yaml' --repeat 20 -- recovering_worker: 20/20 passed, no divergence"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both the eval harness and the three integration tests run in a default cargo test --workspace with no feature flag or environment variable, and cargo test --test evals <filter> selects a single scenario's cases"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "cargo test --test evals (4/4 passed); cargo test --test evals e2e-2 (2/2 passed, 2 filtered out)"
        status: pass
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~145min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 16: E2E-1/2/3 Program Acceptance Scenarios, Dogfooded And Three Real Bugs Fixed Along The Way Summary

**The program's own three acceptance scenarios (E2E-1 crash-resume, E2E-2 approval gate, E2E-3 map-reduce fault-tolerance) now run as `.eval.yaml` scenarios sharing their graph builders verbatim with the integration tests, and dogfooding them surfaced and fixed three real bugs in the eval runner/CLI: a retry-defeating error-classification erasure, a stale-`RunFinished`-record read for any parley-answering case, and two sources of false `--repeat` divergence.**

## Performance

- **Duration:** ~145 min
- **Started:** 2026-09-09T04:56:00Z (worktree base `971c9b4f`)
- **Completed:** 2026-09-09T05:20:15Z (excludes SUMMARY authoring)
- **Tasks:** 2/2
- **Files modified:** 13 (5 created, 8 modified)

## Accomplishments

- `tests/helpers/e2e_fixtures.rs`: `build_crash_resume_graph`, `build_approval_gate_graph`, `build_muster_defer_order_graph`/`_with_a_template_per_task`, plus the shared `make_paladin`/`field` helpers -- moved verbatim (a move, not a redesign) out of the three E2E integration test files. All three integration test binaries pass with the SAME test counts as before the extraction (32/35/37).
- Three `.eval.yaml` scenarios at the repository root under `evals/`, one blessed snapshot (`evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json`), registered via `ScenarioRunner::register_graph` from BOTH `tests/evals.rs` (the `libtest-mimic` harness) and `src/application/cli/commands/eval.rs`'s new `e2e_scenario_runner()` (the CLI) -- one definition, three consumers (the two integration-test callers plus the two eval front doors), reached via `#[path]` inclusion rather than a duplicated copy.
- `paladin_eval::eval_scenarios!` gained a second macro arm (`eval_scenarios!(pattern, |runner: &mut ScenarioRunner| { ... })`) and `run_eval_harness_main_with`, because the existing single-arg form always built an EMPTY `ScenarioRunner` -- there was no way for a `harness = false` `[[test]]` binary to register a `registered`-target constructor before this plan.
- **Three real bugs found and fixed, each discovered by actually running the dogfood scenarios, not by inspection:**
  1. `ScenarioPaladinPort::execute` erased every scripted `LlmError` into the legacy `PaladinError::LlmError(String)` string variant (`Transience::Unknown`), so a scripted `error: transient` entry could never be retried under the DEFAULT `TransientOnly` predicate. Fixed by routing through `paladin_battalion::llm_failure::to_paladin_error`. Without this, E2E-3's whole premise (a scripted transient failure recovering under a real Aegis retry) was unreachable.
  2. `AssertionContext::run_finished()` used `.find_map()` (first match) instead of the last `TraceRecord::RunFinished`. Any case answering a raised Parley makes MORE than one top-level engine call (`start`, then `resume_with`), and each call emits its own `RunFinished`; the first necessarily reads `AwaitingInput` (that's why the run paused). E2E-2's `approve`/`deny` cases were genuinely completing correctly (confirmed via a throwaway debug trace dump before reverting it) while every `run_status`-dependent assertion read the stale pre-resume record.
  3. `record_fingerprint`'s `WaypointSaved` arm embedded the FULL pre-rendered status string (including the random `parley_id`/wall-clock `created_at` for an `AwaitingInput` waypoint) -- exactly the class of field the function's own doc comment promises to exclude. Separately, `first_divergence` compared genuinely-concurrent muster-fan-out `NodeStarted`/`NodeFinished` records in strict positional order, when the engine dispatches every mustered task as an independent, unlinked async task in the SAME superstep (CF-03) -- real OS-scheduling nondeterminism in trace-record ORDER, not a behavioural difference (the engine's own `worker_deltas_merge_in_task_key_order_not_completion_order` test already proves the resulting STATE always merges deterministically). Both were real sources of false `--repeat` divergence on fully deterministic scripted scenarios; fixed via a normalized status-tag fingerprint and `stabilized_fingerprints`'s same-superstep sort-before-compare. The existing `repeat_divergence_exits_nonzero_and_names_the_seq_range` test (a genuinely injected divergence) still passes unchanged, proving the fix didn't just widen the tolerance to hide real problems.
- `paladin-cli eval run "evals/*.eval.yaml" --repeat 20` reports `4/4 cases passed` with every case `20/20 passed` and NO divergence line, exit 0 -- the program's own acceptance fixtures are proven deterministic under scripted mocks.

## Task Commits

1. **Task 1 (tracer): Extract the three graph builders and register them once, with the integration tests still green** -- `e2329287` (feat)
2. **Task 2: The three scenario files, green and twenty-of-twenty** -- `99d95ab9` (feat)

**Tracer feedback gate:** Task 1's own `<verify>` (the three E2E integration tests, adjusted to the real target names and the real E2E-3 source file per the plan-file correction below) was re-run immediately after the Task 1 commit and passed before Task 2 began.

## Files Created/Modified

- `tests/helpers/e2e_fixtures.rs` -- the three shared E2E graph builders and helpers (new)
- `tests/helpers/mod.rs` -- `pub mod e2e_fixtures;`
- `tests/integration/e2e_crash_resume_test.rs`, `e2e_approval_gate_test.rs`, `e2e_muster_defer_order_test.rs` -- call the extracted builders; inline graph-building code and now-unused imports removed
- `evals/e2e-1-crash-resume.eval.yaml`, `evals/e2e-2-approval-gate.eval.yaml`, `evals/e2e-3-map-reduce-fault-tolerance.eval.yaml` -- the three dogfood scenarios (new)
- `evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json` -- the blessed final-state snapshot (new)
- `tests/evals.rs` -- registers the three graph constructors + the (empty, documented) `"e2e"` registries factory via the macro's new second arm
- `crates/paladin-eval/src/runner.rs` -- `run_eval_harness_main_with` + the `eval_scenarios!` second arm; the `ScenarioPaladinPort::execute` transience-classification fix
- `crates/paladin-eval/src/assertion.rs` -- `run_finished()`'s last-not-first `RunFinished` fix
- `src/application/cli/commands/eval.rs` -- `e2e_fixtures` module (`#[path]`-included) and `e2e_scenario_runner()`; `record_fingerprint`'s `WaypointSaved` normalization; `stabilized_fingerprints` + rewritten `first_divergence`

## Decisions Made

See `key-decisions` in the frontmatter. Most consequential: this plan discovered its own dogfooding target (the runner/CLI built in plan 28-12) had never actually been exercised against a scripted-retry scenario or a multi-call parley-answering scenario or a genuinely-concurrent muster scenario before now -- all three of those exact shapes are what E2E-1/2/3 require, and all three surfaced a real, previously-latent bug on first contact. This is exactly the value the plan's own objective names ("if the harness cannot express the program's own three acceptance scenarios, it cannot express a user's -- this is the proof").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan-file correction: E2E-3's real source file is `e2e_muster_defer_order_test.rs`, not `e2e_compensation_chain_test.rs`**
- **Found during:** Task 1, `<read_first>` — reading `tests/integration/e2e_compensation_chain_test.rs` in full per the plan's own instruction.
- **Issue:** The plan's frontmatter `files_modified` and `<interfaces>` section name `tests/integration/e2e_compensation_chain_test.rs` as the source of E2E-3's "planner musters five workers" fixture. That file's actual content (verified by reading it in full) is PRD 04's unrelated compensation-chain/ask-a-human-on-payment-failure scenario (`book`/`cancel`/`payment` nodes, D-21/D-23) -- no planner, no muster, no aggregator anywhere in it. The real muster/defer/order + recovering-worker fixture the plan describes is `tests/integration/e2e_muster_defer_order_test.rs`, confirmed by `grep -rl "muster\|aggregat\|worker" tests/integration/*.rs`.
- **Fix:** Used `tests/integration/e2e_muster_defer_order_test.rs` as E2E-3's source throughout; left `e2e_compensation_chain_test.rs` completely untouched (out of this plan's actual scope, a different PRD-04 acceptance bar with its own already-passing tests).
- **Files modified:** `tests/helpers/e2e_fixtures.rs`, `tests/integration/e2e_muster_defer_order_test.rs`, `tests/evals.rs`, `src/application/cli/commands/eval.rs` -- all reference the correct file/builders.
- **Verification:** `cargo test --test e2e_muster_defer_order` -- 37/37 passed, unchanged assertions.
- **Committed in:** `e2329287` (Task 1), `99d95ab9` (Task 2)

**2. [Rule 1 - Bug] `ScenarioPaladinPort::execute`'s legacy `PaladinError::LlmError(String)` erasure defeated scripted-error retry classification**
- **Found during:** Task 2, first `cargo test --test evals` run of `evals/e2e-3-map-reduce-fault-tolerance.eval.yaml` (w3 was not being retried).
- **Issue:** Every scripted `LlmError` was converted via `.map_err(|source| PaladinError::LlmError(source.to_string()))?` -- the exact legacy string erasure `paladin_battalion::llm_failure`'s own module doc says it replaced everywhere else in the codebase. `PaladinError::LlmError(_)`'s `transience()` is `Transience::Unknown` unconditionally, which the DEFAULT `TransientOnly` retry predicate never retries -- so no scripted `error:` entry could ever be retried by any scenario, regardless of `LlmErrorKind`.
- **Fix:** Route through `paladin_battalion::llm_failure::to_paladin_error(&source)` instead, preserving the real `LlmError::transience()` classification (`LlmErrorKind::Transient` maps to `LlmError::NetworkError`, `Transience::Transient`).
- **Files modified:** `crates/paladin-eval/src/runner.rs`
- **Verification:** `cargo test -p paladin-eval` (56 lib + 12 snapshot + 1 golden + 4 doc tests, all pass); E2E-3's scenario then shows w3 retried exactly twice and succeeding on attempt 3.
- **Committed in:** `99d95ab9` (Task 2)

**3. [Rule 1 - Bug] `AssertionContext::run_finished()` read the first `RunFinished` record, not the last**
- **Found during:** Task 2, `evals/e2e-2-approval-gate.eval.yaml`'s both cases failing `run_status: completed` with `observed: AwaitingInput` despite the run genuinely completing (confirmed by a throwaway `eprintln!` debug trace dump of `drive_parleys`'s own returned `RunOutcome`, reverted before commit).
- **Issue:** `.records.iter().find_map(...)` returns the FIRST `TraceEvent::RunFinished` record. Any case with `parley_responses` drives the engine through more than one top-level call (`start`, then `resume_with`), and EACH call emits its own `RunFinished` -- the first is necessarily `AwaitingInput` (that's why the run paused for a response). `run_status`/`total_tokens_max`/`supersteps_max` all delegate to this one method, so all three were reading a stale pre-resume snapshot for any parley-answering case.
- **Fix:** `.records.iter().rev().find_map(...)` -- the LAST `RunFinished` record.
- **Files modified:** `crates/paladin-eval/src/assertion.rs`
- **Verification:** `cargo test -p paladin-eval` still 56/12/1/4 all pass (no regression to the existing `run_status_fails_when_run_never_finished` single-call test); `evals/e2e-2-approval-gate.eval.yaml`'s both cases pass.
- **Committed in:** `99d95ab9` (Task 2)

**4. [Rule 1 - Bug] `--repeat`'s divergence detector reported two classes of false positive on fully deterministic scenarios**
- **Found during:** Task 2, running the plan's own required `paladin-cli eval run "evals/*.eval.yaml" --repeat 20` acceptance check.
- **Issue:** (a) `record_fingerprint`'s `WaypointSaved` arm used the raw pre-rendered `status` string directly, which for an `AwaitingInput` waypoint embeds the full `ParleyRequest` list including the random `parley_id` and wall-clock `created_at` -- exactly the class of field the function's own doc comment says it excludes. (b) `first_divergence` compared fingerprints in strict positional order, but the engine dispatches every mustered task's execution as an independent, unlinked async task within the SAME superstep (CF-03, same-superstep fan-out) -- which sibling's `NodeStarted`/`NodeFinished` record lands first in the captured trace is real OS-scheduling nondeterminism across separate process runs, not a behavioural difference (the engine's own `engine::superstep::tests::worker_deltas_merge_in_task_key_order_not_completion_order` unit test already proves the resulting state always merges deterministically by `task_key`, regardless of completion order).
- **Fix:** (a) Normalized `WaypointSaved`'s fingerprint to just the leading status-variant tag (the text before the first `{`, trimmed) -- drops the volatile struct body, keeps the deterministic status kind. (b) Added `stabilized_fingerprints`, which sorts each maximal contiguous run of same-superstep `NodeStarted`/`NodeFinished` fingerprints by their own text before comparison (still detects a genuine content difference at a different position; only harmless interleaving of otherwise-identical concurrent work stops being reported), and rewrote `first_divergence` to consume `(seq, fingerprint)` pairs so seq reporting stays meaningful after reordering.
- **Files modified:** `src/application/cli/commands/eval.rs`
- **Verification:** `paladin-cli eval run "evals/*.eval.yaml" --repeat 20` -- `4/4 cases passed`, every case `20/20 passed`, no divergence line, exit 0. The existing `tests/cli/eval_run_test.rs::repeat_divergence_exits_nonzero_and_names_the_seq_range` test (a genuinely injected, non-concurrent divergence) still passes unchanged -- confirming the fix narrows false positives without hiding real ones.
- **Committed in:** `99d95ab9` (Task 2)

**5. [Rule 2 - Missing critical functionality] `eval_scenarios!`'s single-arg form could never populate a `ScenarioRunner` with `registered` targets**
- **Found during:** Task 2, before writing `tests/evals.rs`'s own registration -- `run_eval_harness_main` (what the existing macro arm expands to) hard-codes `ScenarioRunner::new()` with nothing registered, so `tests/evals.rs` had no way to make a `registered` target resolve at all.
- **Issue:** Plan 28-12's own SUMMARY explicitly anticipated this landing here ("28-16 can populate `evals/` ... `eval_scenarios!("evals/**/*.eval.yaml")` will pick them up with no `tests/evals.rs` change needed" was optimistic about the file glob, but the REGISTRATION mechanism itself did not yet exist).
- **Fix:** Added a second macro arm, `eval_scenarios!($pattern, $configure)`, expanding to a new `run_eval_harness_main_with(pattern, configure)` that builds a `ScenarioRunner`, calls `configure` on it, then globs -- the existing single-arg form is completely unchanged and still used by any future scenario suite with no `registered` targets.
- **Files modified:** `crates/paladin-eval/src/runner.rs`, `tests/evals.rs`
- **Verification:** `cargo test --test evals` (4/4 passed, all `registered` targets resolve); `cargo doc -p paladin-eval --no-deps` (0 warnings, both macro-arm doc examples build).
- **Committed in:** `99d95ab9` (Task 2)

**6. [Rule 2 - Missing critical functionality] `paladin-cli eval run` had no way to resolve `registered` targets at all**
- **Found during:** Task 2, before running the plan's own required `--repeat 20` CLI acceptance check -- `run_eval_report`'s own code comment already documented the gap ("the facade's own eval registry ... lands in plan 28-16 ... a `registered` target is therefore always unresolved today").
- **Issue:** `run_eval_report` built a bare `ScenarioRunner::new()`, so the CLI could only ever run `graph_doc` targets; the plan's own acceptance criterion requires the SAME `registered` targets `tests/evals.rs` runs to also pass through `paladin-cli eval run --repeat 20`.
- **Fix:** Added a `#[path]`-included `e2e_fixtures` module (pointing at the same `tests/helpers/e2e_fixtures.rs` `tests/evals.rs` uses) and `e2e_scenario_runner()` to `src/application/cli/commands/eval.rs`, gated by the `cli` feature at the module-declaration level exactly like the rest of this file; `run_eval_report` now calls `e2e_scenario_runner()` instead of `ScenarioRunner::new()`.
- **Files modified:** `src/application/cli/commands/eval.rs`
- **Verification:** `cargo check --features cli --bin paladin-cli` (exit 0); `cargo test --features cli --test cli` (117/117 passed, including the pre-existing `unresolvable_registered_target_is_a_clear_error` test against an UNRELATED name, still correctly erroring); `paladin-cli eval run "evals/*.eval.yaml" --repeat 20` (4/4 cases, 20/20 each).
- **Committed in:** `99d95ab9` (Task 2)

---

**Total deviations:** 6 (1 Rule 3 plan-file correction, 3 Rule 1 bug fixes, 2 Rule 2 missing-critical-functionality additions). **Impact on plan:** All six were required for this plan's own stated acceptance criteria (E2E-1/2/3 green under both `cargo test --test evals` and `paladin-cli eval run --repeat 20`) to be achievable at all -- none is scope creep beyond what "dogfood the harness on the program's own acceptance bar" already demanded. The three bug fixes are exactly the kind of finding the plan's own objective predicts ("if the harness cannot express the program's own three acceptance scenarios, it cannot express a user's -- this is the proof").

## Issues Encountered

None beyond the deviations above, all resolved. Two throwaway `eprintln!` debug blocks were added and removed during investigation (in `drive_parleys` and `run_repeat_sweep`) -- confirmed absent via `grep -rn "DEBUG_FINGERPRINTS\|eprintln!(\"DEBUG"` before the Task 2 commit.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- 28-17 should note in `MIGRATION.md`/the crate-feature inventory: `paladin_eval::eval_scenarios!` gained a second, optional-argument macro arm (additive, source-compatible with every existing single-arg caller); `src/application/cli/commands/eval.rs` gained a new `cli`-feature-gated `e2e_fixtures` module and `e2e_scenario_runner()` function (private, not part of any public API surface, X-10).
- The three program acceptance scenarios (E2E-1/2/3) are now proven expressible in, and deterministic under, the `paladin-eval` harness -- OBS-FR-15's dogfood requirement and PRD 07 acceptance criterion 5's final clause both hold.
- No blockers for the rest of Phase 28.

## Self-Check: PASSED

- FOUND: `tests/helpers/e2e_fixtures.rs`
- FOUND: `evals/e2e-1-crash-resume.eval.yaml`
- FOUND: `evals/e2e-2-approval-gate.eval.yaml`
- FOUND: `evals/e2e-3-map-reduce-fault-tolerance.eval.yaml`
- FOUND: `evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json`
- FOUND commit: `e2329287`
- FOUND commit: `99d95ab9`
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo doc -p paladin-eval --no-deps` -- exit 0, 0 warnings
- `cargo test -p paladin-eval` -- 56 lib + 12 snapshot + 1 golden + 4 doc tests passed, 0 failed
- `cargo test --test e2e_crash_resume` -- 32/32 passed (same count as before extraction)
- `cargo test --test e2e_approval_gate` -- 35/35 passed (same count as before extraction)
- `cargo test --test e2e_muster_defer_order` -- 37/37 passed (same count as before extraction)
- `cargo test --test evals` -- 4/4 passed
- `cargo test --test evals e2e-2` -- 2/2 passed, 2 filtered out (name filter works)
- `cargo test --features cli --test cli` -- 117/117 passed
- `paladin-cli eval run "evals/*.eval.yaml" --repeat 20` -- 4/4 cases passed, 20/20 each, no divergence, exit 0

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
