---
phase: 28-observability-tooling
plan: 12
subsystem: testing
tags: [paladin-eval, libtest-mimic, scenario-runner, cli, live-mode, obs-fr-13]

# Dependency graph
requires:
  - phase: 28-05
    provides: "The .eval.yaml scenario file format (Scenario/ScenarioTarget/StoreKind/Case/LlmScript) and ScenarioLlm, the scripted LlmPort a scenario substitutes for a real provider"
  - phase: 28-08
    provides: "The assertion library: AssertionContext (built from exactly the trace record stream, final Battlefield, and RunOutcome), AssertionOutcome/AssertionFailure::render_failure, and pub fn evaluate over the closed Assertion enum"
  - phase: 28-06
    provides: "WarEngine::with_trace_sink / with_bound_trace and the per-run TraceDispatcher this plan's CapturingSink attaches to"
provides:
  - "crates/paladin-eval/src/runner.rs: ScenarioRunner (register_graph, register_registries, run_case, trials), ScriptedPorts/GraphConstructor/RegistriesFactory for registered targets, ScenarioPaladinPort (routes each Paladin node's own PaladinData.name to its substituted ScenarioLlm or, in live mode, a single real provider), CapturingSink, the interrupt_after_superstep control-run-then-seed crash/resume driver, and the eval_scenarios! macro expanding to a harness=false fn main()"
  - "tests/evals.rs: the [[test]] name = \"evals\" harness = false target -- 0 trials today since evals/ doesn't exist until plan 28-16"
  - "src/application/cli/commands/eval.rs: paladin-cli eval run with --repeat (normalized-fingerprint divergence detection across repeats, never raw TraceRecord which carries wall-clock/UUID noise), --bless (writes/refreshes final_state_snapshot files), --registries, and --live (gated by the runner's own three-way check_live_mode)"
  - "LiveMode (D-35): check_live_mode/LiveModeError/PALADIN_EVAL_LIVE_ENV, is_content_assertion, Verdict::Skipped for content assertions skipped under live mode without the scenario's own opt-in; a paladin-eval `live` feature gates real paladin-llm provider construction"
affects: ["28-16 (E2E-1/2/3 dogfood scenarios populate evals/ and register graph constructors + registries via ScenarioRunner::register_graph/register_registries)", "28-17 (ADR-0048 update noting the cli-feature exception to D-27's dev-dependency-only shape; MIGRATION.md/crate-list registration)"]

# Tech tracking
tech-stack:
  added: ["libtest-mimic 0.8.2 (paladin-eval library dependency, not dev-only)", "glob 0.3.4 (paladin-eval library dependency; also a root-facade optional dependency behind cli)", "paladin-eval as an optional root-facade dependency behind the cli feature (D-33's one named exception to D-27)"]
  patterns:
    - "One ScenarioRunner drives two front doors: eval_scenarios!'s libtest-mimic harness and paladin-cli eval run both call ScenarioRunner::run_case"
    - "Simulated crash-and-resume reuses tests/integration/e2e_crash_resume_test.rs's own technique verbatim (a throwaway control run seeds a fresh store with its early Waypoints) rather than a CancellationToken race against an async trace sink -- deterministic, no timing dependency"
    - "--repeat divergence detection normalizes each TraceRecord to a fingerprint excluding wall-clock (at, duration_ms) and per-run-random (waypoint_id, parley_id, thread_id) fields before comparing repeat runs, so a fully deterministic scripted scenario never false-positives on ordinary UUID/timestamp jitter"
    - "LiveMode's three-way gate (--live flag AND PALADIN_EVAL_LIVE env AND a configured provider credential) is enforced INSIDE ScenarioRunner::run_case itself, not only by the CLI caller -- a future caller (e.g. a test harness) gets the same refusal for free"

key-files:
  created:
    - crates/paladin-eval/src/runner.rs
    - tests/evals.rs
    - src/application/cli/commands/eval.rs
    - tests/cli/eval_run_test.rs
  modified:
    - crates/paladin-eval/src/lib.rs
    - crates/paladin-eval/Cargo.toml
    - Cargo.toml
    - Cargo.lock
    - src/application/cli/commands/mod.rs
    - src/bin/paladin-cli.rs
    - tests/cli/mod.rs

key-decisions:
  - "interrupt_after_superstep reuses the e2e_crash_resume_test.rs control-run-then-seed technique, not a live CancellationToken halt -- fully deterministic, no async-trace-sink-delivery race. Documented, deliberate limitation: a scenario's SHARED llm.global sequence is consumed in call order across every node, so the resumed run's fresh ScenarioLlm cursor cannot fast-forward past calls the already-completed (seeded) nodes made pre-crash; author interrupt-testing scenarios with PER-NODE llm.per_node sequences for deterministic replay (each node's own cursor is independent of which other nodes already ran)."
  - "libtest-mimic and glob are LIBRARY (not dev-only) dependencies of paladin-eval: eval_scenarios! expands to a fn main() calling a function compiled INTO paladin-eval, so the caller (tests/evals.rs, a root-facade dev-dependency) never needs either crate itself. This keeps the root Cargo.toml's own additions to exactly the [[test]] stanza and the unconditional paladin-eval dev-dependency."
  - "paladin-eval joins the facade's optional [dependencies] behind the cli feature (D-33) -- a deliberate, narrow exception to D-27/28-05's 'dev-dependency only' dependency shape, documented explicitly in paladin-eval's own lib.rs doc comment so it reads as an intentional exception, not a violation. The default facade build (no --features cli) still has zero edges to paladin-eval (cargo tree -e normal,build -p paladin-ai | grep -c paladin-eval prints 0)."
  - "Live-mode provider construction is gated behind a NEW paladin-eval `live` feature (forwarding to paladin-llm/openai,anthropic,deepseek, mirroring the root facade's own llm-* naming) -- a build without it refuses with LiveModeError::FeatureNotCompiled rather than silently proceeding once the gate otherwise passes. None of Task 3's five required behavior tests need this feature; they exercise check_live_mode/is_content_assertion's pure logic, per the plan's own scoping."
  - "ScenarioPaladinPort routes by the Paladin's own PaladinData.name (the only identity PaladinPort::execute receives), mapped to a ScenarioLlm::for_node clone tagged with the GRAPH node's id (the identity a scenario's llm.per_node map is keyed by) -- the two identities are NOT interchangeable in general (linear.json's own fixture authors them differently: node id \"start\" vs paladin name \"Start\"), confirmed by the crate's own graph_doc test."

patterns-established:
  - "A CaseReport::Errored(String) folds every resolution/compile/engine-level failure into one uniform, never-panicking outcome distinct from an assertion failure -- CaseOutcome::Ran{..}/Errored(_) is the shape both the harness and the CLI report against."

requirements-completed: [OBS-04]

coverage:
  - id: D1
    description: "ScenarioRunner drives a real WarEngine end to end for both a graph_doc and a registered target, substituting a ScenarioLlm per Paladin node (graph_doc, automatic via the compiled WarGraph's own node table) or per ScriptedPorts::for_node call (registered, for Function nodes), capturing every trace record through WarEngine::with_trace_sink and evaluating assertions against exactly that stream, the final Battlefield, and the RunOutcome"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::runner_executes_a_graph_doc_case"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::runner_executes_a_registered_target"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::runner_reports_a_failing_assertion_with_its_rendering"
        status: pass
    human_judgment: false
  - id: D2
    description: "interrupt_after_superstep drops the engine after the named superstep, seeds a fresh store, and resumes -- the resumed run's final Battlefield equals an uninterrupted control run's"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::interrupt_and_resume_reproduces_the_control_run"
        status: pass
    human_judgment: false
  - id: D3
    description: "eval_scenarios! expands to a harness=false fn main() discovering one Trial per (file, case) pair, named <file-stem>::<case>, and a libtest-mimic name filter selects a single case; cargo test --test evals builds and runs green (0 trials today, evals/ lands in 28-16)"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::evals_target_discovers_one_trial_per_case"
        status: pass
      - kind: other
        ref: "cargo test --test evals (test result: ok. 0 passed; 0 failed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "paladin-cli eval run reports one line per case plus a summary count, exits non-zero on failure, --repeat N reports a pass rate and exits non-zero on ANY divergence across repeats (naming the diverging seq range via a wall-clock/UUID-excluding record fingerprint), --bless (re)writes and round-trips a final_state_snapshot file, and an unresolvable registered target produces a clear error naming the target and the (empty) registered set"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "tests/cli/eval_run_test.rs#eval_run_reports_per_case_results"
        status: pass
      - kind: integration
        ref: "tests/cli/eval_run_test.rs#eval_run_exits_nonzero_on_failure"
        status: pass
      - kind: integration
        ref: "tests/cli/eval_run_test.rs#repeat_twenty_is_twenty_of_twenty"
        status: pass
      - kind: unit
        ref: "tests/cli/eval_run_test.rs#repeat_divergence_exits_nonzero_and_names_the_seq_range"
        status: pass
      - kind: integration
        ref: "tests/cli/eval_run_test.rs#bless_writes_a_snapshot_beside_the_scenario"
        status: pass
      - kind: integration
        ref: "tests/cli/eval_run_test.rs#unresolvable_registered_target_is_a_clear_error"
        status: pass
    human_judgment: false
  - id: D5
    description: "Live mode requires --live AND PALADIN_EVAL_LIVE AND a configured provider credential (ALL three) or refuses with a typed LiveModeError naming which is missing, never falling back to scripted mocks; content-bearing assertions are skipped (Verdict::Skipped, never counted as passed) in live mode unless the scenario's own live.allow_content_assertions opts in; structural assertions always run; a plain (non-live) run never enters the gate at all"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::live_mode_requires_flag_and_env_and_keys"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::content_assertions_are_skipped_in_live_mode_without_opt_in"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::content_assertions_run_in_live_mode_with_opt_in"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::structural_assertions_always_run_in_live_mode"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/runner.rs#runner::tests::default_test_run_never_enters_live_mode"
        status: pass
    human_judgment: false
  - id: D6
    description: "cargo fmt clean; cargo clippy clean (paladin-eval default/all-features, and workspace with --features cli); cargo doc -p paladin-eval --no-deps zero warnings; the facade's default build gains no edge to paladin-eval (cargo tree -e normal,build -p paladin-ai | grep -c paladin-eval prints 0); cargo deny check: advisories/bans/licenses/sources all ok"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo check --workspace --all-targets --all-features (exit 0, 4m20s cold); cargo clippy --all-targets --all-features -p paladin-eval -p paladin-ai -- -D warnings (0 warnings); cargo doc -p paladin-eval --no-deps (0 warnings); cargo tree -e normal,build -p paladin-ai | grep -c paladin-eval (0); cargo deny check (advisories ok, bans ok, licenses ok, sources ok)"
        status: pass
    human_judgment: false

# Metrics
duration: ~85min (extensive upfront API-surface research across paladin-battalion/paladin-core/paladin-ports/paladin-llm preceded the first commit; commit-to-commit span alone was ~17min for the 3 task commits)
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 12: One `ScenarioRunner`, Two Front Doors -- `libtest-mimic` Harness and `paladin-cli eval run`, Gated Live Mode Summary

**`ScenarioRunner` drives a real `WarEngine` from a `.eval.yaml` scenario (graph_doc or a code-registered target), captured through the SAME engine trace-sink API 28-06 built, evaluated against 28-08's assertion library; `eval_scenarios!` turns any glob into one `libtest-mimic` `Trial` per case, `paladin-cli eval run` adds `--repeat`'s normalized-fingerprint divergence detection and `--bless`'s snapshot regeneration on top of the identical runner, and live mode is refused three ways at once unless a scenario deliberately opts in.**

## Performance

- **Duration:** ~85 min total session (see `duration` above for the breakdown); the three task commits themselves span 03:40:04Z → 03:57:13Z (~17 min)
- **Started:** 2026-09-09 (worktree base `deda7875`)
- **Completed:** 2026-09-09T04:04:12Z (final verification pass)
- **Tasks:** 3/3
- **Files modified:** 11 (4 created, 7 modified)

## Accomplishments

- `crates/paladin-eval/src/runner.rs` (Task 1, ~1050 lines before Task 3's additions, ~1370 after): `ScenarioRunner` (`register_graph`, `register_registries`, `run_case`, `trials`), `ScriptedPorts`/`GraphConstructor`/`RegistriesFactory` for `registered` targets, `ScenarioPaladinPort` (routes each Paladin node's own `PaladinData.name` to its substituted `ScenarioLlm`), `CapturingSink` attached via `WarEngine::with_trace_sink`, the `interrupt_after_superstep` control-run-then-seed crash/resume driver (reusing `tests/integration/e2e_crash_resume_test.rs`'s own documented technique), and the `eval_scenarios!` macro expanding to a `harness = false` `fn main()`.
- `tests/evals.rs` + root `Cargo.toml`'s new `[[test]] name = "evals" harness = false` target -- the first `harness = false` `[[test]]` in this workspace (28-RESEARCH.md's own finding), building and running green with 0 trials today (`evals/` lands in plan 28-16).
- `src/application/cli/commands/eval.rs` (Task 2, ~420 lines after Task 3): `paladin-cli eval run <glob> [--repeat N] [--bless] [--live] [--registries <name>]` driving the SAME `ScenarioRunner` the harness uses. `--repeat N` compares a normalized per-record fingerprint (excluding `at`/`duration_ms`/`waypoint_id`/`parley_id`/`thread_id` -- fields that legitimately differ run-to-run even under a fully deterministic scripted LLM) across all N repeats, exiting non-zero and naming the diverging `seq` range on ANY divergence, never merely a failure count.
- LiveMode (Task 3, D-35): `check_live_mode` enforces `--live` AND `PALADIN_EVAL_LIVE` AND a configured provider credential (ADR-0012), ALL three, called by `ScenarioRunner::run_case` itself (not only the CLI); `is_content_assertion` classifies the four content-bearing kinds; `Verdict::Skipped` reports them skipped (never folded into the pass count) unless a scenario opts in via `live.allow_content_assertions`; structural assertions always run. A new `paladin-eval` `live` feature gates real `paladin-llm` provider construction, refusing cleanly with `LiveModeError::FeatureNotCompiled` when not compiled in.
- `tests/cli/eval_run_test.rs`: 6 tests covering per-case reporting, non-zero exit on failure, `--repeat 20` (20/20), injected-divergence detection (`first_divergence` exposed `pub` for exactly this), the `--bless` write/round-trip/delete-fails cycle, and an unresolvable `registered` target's clear error.

## Task Commits

1. **Task 1 (tracer): `ScenarioRunner` and the `evals` custom test harness target** -- `d91e878f` (feat)
2. **Task 2: `paladin-cli eval run` with `--repeat`, `--bless`, `--registries`** -- `124a7d71` (feat)
3. **Task 3: Gated live-model mode** -- `da6eea51` (feat)

**Tracer feedback gate:** Task 1's own `<verify>` (`cargo test -p paladin-eval --lib runner` -> 5 passed; `cargo test --test evals` -> ok) was re-run after the commit and passed before Task 2 began.

## Files Created/Modified

- `crates/paladin-eval/src/runner.rs` -- `ScenarioRunner`, `eval_scenarios!`, `ScenarioPaladinPort`, `CapturingSink`, the interrupt/resume driver, `LiveMode` (Task 3)
- `crates/paladin-eval/src/lib.rs` -- `pub mod runner;` + re-exports; amended the D-27 doc comment to name the new `cli`-feature exception (D-33)
- `crates/paladin-eval/Cargo.toml` -- `libtest-mimic = "0.8.2"`, `glob = "0.3.4"` (library deps), `tokio` promoted out of `[dev-dependencies]`, the new `live` feature
- `tests/evals.rs` -- the `evals` harness target's `fn main()` entry point
- `src/application/cli/commands/eval.rs` -- `run_eval`/`run_eval_report`, `EvalCommands`/`EvalRunArgs`, `first_divergence`/`record_fingerprint`
- `src/application/cli/commands/mod.rs` -- `pub mod eval;`
- `src/bin/paladin-cli.rs` -- the `Eval { action: EvalCommands }` subcommand group
- `tests/cli/eval_run_test.rs` -- 6 CLI-level tests
- `tests/cli/mod.rs` -- `mod eval_run_test;`
- Root `Cargo.toml` -- the `evals` `[[test]]` stanza; `paladin-eval` as both an unconditional `[dev-dependencies]` entry (the harness) and an optional `cli`-feature `[dependencies]` entry (the CLI); `glob` as an optional `cli`-feature dependency
- `Cargo.lock` -- resolved entries for the two new library dependencies

## Decisions Made

See `key-decisions` in the frontmatter. Most consequential: the crash/resume driver deliberately reuses the E2E integration test's own deterministic control-run-then-seed technique rather than a `CancellationToken`-based live halt (which would race an async trace sink's delivery against the engine's own superstep-boundary check) -- documented as a real, bounded limitation (global vs per-node LLM scripts) rather than a silently-accepted gap.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `tokio` promoted from `[dev-dependencies]` to `[dependencies]` in `crates/paladin-eval/Cargo.toml`**
- **Found during:** Task 1, first `cargo check -p paladin-eval --all-targets`
- **Issue:** `ScenarioRunner`'s own library code (`tokio::time::sleep` to let the async `TraceDispatcher` consumer drain, `tokio::sync::mpsc` for `execute_stream`, `tokio::runtime::Builder` for `trials()`'s per-trial fresh runtime) is compiled unconditionally, not only under `#[cfg(test)]` -- `tokio` as a dev-only dependency does not link into the library target at all.
- **Fix:** Moved `tokio = { workspace = true }` into `[dependencies]`.
- **Files modified:** `crates/paladin-eval/Cargo.toml`
- **Verification:** `cargo check -p paladin-eval --all-targets` clean afterward.
- **Committed in:** `d91e878f` (Task 1 commit)

**2. [Rule 1 - Bug] `clippy::result_large_err` on `RunnerError`**
- **Found during:** Task 1, `cargo clippy -p paladin-eval --all-targets --all-features -- -D warnings`
- **Issue:** `RunnerError::Compile`'s `CompileError` source made the whole enum's happy-path `Result` size very large (144+ bytes), flagged on four separate `Result<_, RunnerError>`-returning functions.
- **Fix:** Boxed the `CompileError` field (`source: Box<paladin_battalion::engine::graph_doc::CompileError>`).
- **Files modified:** `crates/paladin-eval/src/runner.rs`
- **Verification:** `cargo clippy -p paladin-eval --all-targets --all-features -- -D warnings` clean afterward.
- **Committed in:** `d91e878f` (Task 1 commit)

**3. [Rule 1 - Bug] Two broken `rustdoc::broken_intra_doc_links`/`private_intra_doc_links` warnings**
- **Found during:** Task 1 and Task 3, `cargo doc -p paladin-eval --no-deps`
- **Issue:** `[`eval_scenarios!`]` (a `#[macro_export]` macro, hoisted to the crate root, not the `runner` module) needed a `crate::` prefix; a `LiveModeError` doc comment linked to the private `resolve_live_provider` function, which `rustdoc` flags in public-facing docs.
- **Fix:** `[`crate::eval_scenarios!`]`; reworded the `LiveModeError` doc to name `resolve_live_provider` in backticks (prose, not a broken intra-doc link) with an explicit "private" note.
- **Files modified:** `crates/paladin-eval/src/runner.rs`
- **Verification:** `cargo doc -p paladin-eval --no-deps` -- 0 warnings both times.
- **Committed in:** `d91e878f` (Task 1), `da6eea51` (Task 3)

**4. [Process] Test-input fixture mismatches surfaced by real engine runs, fixed before commit**
- **Found during:** Task 1, `cargo test -p paladin-eval --lib runner` (2 of 5 tests failing on first run)
- **Issue:** `runner_executes_a_registered_target`'s test scenario reused `minimal_scenario`'s default `{"topic": "widgets"}` input against a graph whose own schema declares only `out`; `interrupt_and_resume_reproduces_the_control_run`'s 2-node chain graph declared its `topic` field with no default while its own scenario case supplied no input at all.
- **Fix:** Cleared the mismatched input for the first; added a `topic` field default to the second's schema.
- **Files modified:** `crates/paladin-eval/src/runner.rs` (test module only)
- **Verification:** `cargo test -p paladin-eval --lib runner` -- 5/5 passed afterward.
- **Committed in:** `d91e878f` (Task 1 commit)

---

**Total deviations:** 4 (1 Rule 3 blocking dependency fix, 1 Rule 1 clippy lint fix, 1 Rule 1 doc-link fix applied twice across two tasks, 1 process fixture-correction pass before the first commit). **Impact on plan:** None on scope -- all four are mechanical corrections required for the plan's own stated deliverables (a compiling, clippy-clean, doc-clean, green-tested crate) to hold; no scope creep.

## Issues Encountered

- **Acceptance-criteria grep discrepancy (documented, not fixed):** Task 1's acceptance criteria state `grep -c 'harness = false' Cargo.toml` should be "at least 7 (six pre-existing bench targets plus the new test target)". Measured: the SIX pre-existing `harness = false` hits 28-RESEARCH.md found are spread across FIVE separate manifests (`crates/paladin-memory/Cargo.toml` x2, `crates/paladin-battalion/Cargo.toml` x1, `crates/paladin-llm/Cargo.toml` x1, root `Cargo.toml` x2 -- `config_benchmarks`/`engine_benchmarks`), not all six in the root `Cargo.toml` the literal grep command scopes to. A literal `grep -c 'harness = false' Cargo.toml` (root only, no `-r`) can therefore only ever reach `2 (pre-existing) + 1 (new) = 3`, never 7, regardless of how this plan is implemented -- confirmed via `grep -rn 'harness = false' --include=Cargo.toml .`, which sums to exactly 6 pre-existing + this plan's 1 new = 7 across the WHOLE workspace. This reads as a pre-existing inconsistency in the plan's acceptance-criteria wording (the underlying research finding is accurate; the literal single-file grep command scoping is not), not something this plan's own implementation can resolve by adding more `Cargo.toml` content. The other acceptance-criteria grep in the same line (`grep -c 'name = "evals"' Cargo.toml` is 1) is satisfied exactly as written.
- **`grep -c 'pub async fn run_eval' src/application/cli/commands/eval.rs` reads 2, not the acceptance criterion's stated 1:** both `run_eval` (the thin CLI wrapper) and `run_eval_report` (the testable core `run_eval` delegates to, per this executor's own summary-creation convention of splitting side-effecting CLI commands from a directly-testable core) start with the literal substring `pub async fn run_eval`. The underlying requirement -- `pub async fn run_eval(glob, repeat, bless, live, registries) -> Result<(), CliError>` exists with exactly this signature -- is satisfied; the grep's un-anchored pattern also matches the intentionally-added helper.

## User Setup Required

None -- no external service configuration required. (Live mode itself requires a user to set `PALADIN_EVAL_LIVE` and a provider credential before ever using `--live`, but that is the FEATURE's own documented gate, not setup this plan's completion depends on.)

## Next Phase Readiness

- 28-16 can populate `evals/` with the E2E-1/2/3 dogfood scenarios and register their graph constructors via `ScenarioRunner::register_graph`/`register_registries` -- `eval_scenarios!("evals/**/*.eval.yaml")` will pick them up with no `tests/evals.rs` change needed.
- 28-17 should update `.planning/decisions/0048-paladin-eval-composition-crate.md` to record the `cli`-feature exception to D-27's "dev-dependency only" shape (already documented inline in `crates/paladin-eval/src/lib.rs`'s own doc comment) and register the crate's new `live` feature in whatever crate-feature inventory that plan maintains.
- No blockers for the rest of Phase 28.

## Self-Check: PASSED

- FOUND: `crates/paladin-eval/src/runner.rs`
- FOUND: `tests/evals.rs`
- FOUND: `src/application/cli/commands/eval.rs`
- FOUND: `tests/cli/eval_run_test.rs`
- FOUND commit: `d91e878f`
- FOUND commit: `124a7d71`
- FOUND commit: `da6eea51`
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0 (4m20s, cold)
- `cargo clippy --all-targets --all-features -p paladin-eval -p paladin-ai -- -D warnings` -- exit 0, 0 warnings
- `cargo doc -p paladin-eval --no-deps` -- exit 0, 0 warnings
- `cargo test -p paladin-eval` -- 56 lib + 12 snapshot + 1 golden + 4 doc tests passed, 0 failed
- `cargo test --test evals` -- test result: ok. 0 passed (evals/ doesn't exist yet, expected)
- `cargo test -p paladin-ai --features cli --test cli` -- 117 passed, 0 failed
- `cargo tree -e normal,build -p paladin-ai | grep -c paladin-eval` -- `0`
- `cargo deny check` -- advisories ok, bans ok, licenses ok, sources ok

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
