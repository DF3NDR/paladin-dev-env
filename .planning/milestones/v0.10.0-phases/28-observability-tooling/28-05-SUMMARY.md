---
phase: 28-observability-tooling
plan: 05
subsystem: testing
tags: [paladin-eval, scenario-format, serde, schemars, scripted-llm, eval-harness]

# Dependency graph
requires:
  - phase: 28-01
    provides: "The authoritative TraceRecord/TraceEvent envelope in paladin-core (later eval plans capture it through a TraceSink); the LlmPort contract in paladin-ports that ScenarioLlm implements"
provides:
  - "New composition-tier crate crates/paladin-eval (publish = true, workspace version) depending downward on paladin-core, paladin-ports, paladin-battalion, paladin-llm (mock feature only) and paladin-storage (sqlite) — never on the facade paladin-ai (D-27)"
  - "The D-28 scenario file format: Scenario, ScenarioTarget { graph_doc | registered }, StoreKind (default in_memory), LlmScript, ScriptEntry { text | tool_call | error }, LlmErrorKind, MatchRule, Case, ParleyScript, Times, Assertion (the PRD 07 OBS-FR-12 names), LiveOptions, RunStatusValue, ScenarioError, EVAL_SCHEMA_VERSION = \"1\""
  - "ScenarioLlm — an LlmPort implementation with per-node routing (ScenarioLlm::for_node), sequence-per-call scripts, prompt-substring match rules checked before the sequence, error entries that surface an LlmError kind, and CapturedRequest recording for failure rendering (D-30)"
  - "docs/schemas/eval-scenario.schema.json — schemars-derived golden JSON Schema, blessed with UPDATE_EVAL_SCHEMA=1 (tests/schema_golden.rs), mirroring the wargraph-doc schema idiom"
affects: ["28-08 (assertion library consumes Assertion and the captured records)", "28-12 (runner globs .eval.yaml files, adds libtest-mimic and the CLI)", "28-16 (E2E-1/2/3 dogfood scenarios use registered targets)", "28-17 (ADR-0048, crate-list registration, MIGRATION §9.3)"]

# Tech tracking
tech-stack:
  added: ["paladin-eval (new workspace crate)", "serde_yaml 0.9 and schemars 1.2 as paladin-eval dependencies"]
  patterns:
    - "Scenario files are a closed, structured format: every field deserializes to typed data (no shell command, no executed path, no dynamically loaded code); custom(fn) assertions exist only in the Rust API and are never deserialized (T-28-05-01)"
    - "Golden schema idiom reused: schema_for!(Scenario) compared byte-for-byte to docs/schemas/eval-scenario.schema.json, regenerated only under UPDATE_EVAL_SCHEMA=1"
    - "Scripted LLM ownership stays in the harness: ScenarioLlm wraps its own script state instead of extending paladin-llm's MockLlmAdapter/MockScriptEntry, which stay byte-identical (D-30)"

key-files:
  created:
    - crates/paladin-eval/Cargo.toml
    - crates/paladin-eval/README.md
    - crates/paladin-eval/src/lib.rs
    - crates/paladin-eval/src/scenario.rs
    - crates/paladin-eval/src/scripted_llm.rs
    - crates/paladin-eval/tests/schema_golden.rs
    - docs/schemas/eval-scenario.schema.json
  modified:
    - Cargo.lock

key-decisions:
  - "Task 1 checkpoint:decision — option-a auto-selected by the orchestrator under --auto (2026-09-08): paladin-eval ships as a PUBLISHED crate (publish = true, versioned with the workspace) with the D-27 depends-downward shape; the ADR recording the composition-crate classification is 0048 (RESEARCH.md corrected CONTEXT.md's 0047; PROMOTION.md's next free number), written by plan 28-17."
  - "paladin-llm is depended on with default-features = false and only the mock feature, so the harness pulls no provider adapter or HTTP client into a downstream team's dev-dependency graph."
  - "libtest-mimic and glob are NOT added by this plan (plan 28-12 owns the runner and its dependencies); the 28-05 PLAN artifact row expecting the manifest to contain libtest-mimic is satisfied once 28-12 lands."
  - "ScenarioLlm records every request it receives (CapturedRequest) so plan 28-08's failure rendering can show the prompt that produced a divergent answer without reaching into engine internals."

patterns-established:
  - "New scenario-format fields are additive under schema_version \"1\"; an unknown schema_version is a typed ScenarioError naming the found version (X-04), never a silent misparse"

requirements-completed: [OBS-04]

coverage:
  - id: D1
    description: "crates/paladin-eval exists as a published composition crate whose default dependency graph reaches the five leaf crates and never the facade; the facade's own default build does not include it"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "cargo tree -e features -p paladin-ai | grep -c paladin-eval  (prints 0)"
        status: pass
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0 on merged base ba7862d9)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A .eval.yaml scenario with schema_version, target (graph_doc | registered), optional store, llm script and cases deserializes with serde; the JSON form is equal; an unknown schema_version is a typed error"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::minimal_scenario_parses"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::json_and_yaml_agree"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::registered_target_parses"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::unknown_schema_version_is_typed"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::unknown_assertion_kind_is_an_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::script_entry_spellings_round_trip"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scenario.rs#scenario::tests::times_variants_round_trip"
        status: pass
    human_judgment: false
  - id: D3
    description: "ScenarioLlm implements LlmPort with per-node routing, one script entry per call, match rules before the sequence, error entries surfacing an LlmError kind, tool_call entries carrying a function call, and captured requests"
    requirement: "OBS-04"
    verification:
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::sequence_is_consumed_one_per_call"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::match_rule_wins_over_sequence"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::per_node_script_overrides_global"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::error_entry_returns_the_llm_error_kind"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::tool_call_entry_carries_a_function_call"
        status: pass
      - kind: unit
        ref: "crates/paladin-eval/src/scripted_llm.rs#scripted_llm::tests::requests_are_captured_for_failure_rendering"
        status: pass
    human_judgment: false
  - id: D4
    description: "The scenario JSON Schema is schemars-derived and golden-checked against docs/schemas/eval-scenario.schema.json, blessed with UPDATE_EVAL_SCHEMA=1"
    requirement: "OBS-04"
    verification:
      - kind: integration
        ref: "crates/paladin-eval/tests/schema_golden.rs#schema_matches_golden"
        status: pass
    human_judgment: false
  - id: D5
    description: "crates/paladin-llm/src/mock.rs is byte-identical after this plan (MockLlmAdapter/MockScriptEntry untouched, D-30)"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "git diff --quiet dfc64a70..ba7862d9 -- crates/paladin-llm/src/mock.rs  (exit 0)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Crate rustdoc builds clean and every public item is documented (#![warn(missing_docs)]); doc tests pass"
    requirement: "OBS-04"
    verification:
      - kind: other
        ref: "cargo doc -p paladin-eval --no-deps (0 warnings); cargo test -p paladin-eval --doc (4 passed)"
        status: pass
    human_judgment: false

# Metrics
duration: 28min executor + orchestrator close-out
completed: 2026-09-09
status: complete
---

# Phase 28: Observability & Tooling — Plan 05 Summary

**The `paladin-eval` crate exists with its two foundations — the D-28 scenario file format (golden-checked JSON Schema) and `ScenarioLlm`, the scripted `LlmPort` that makes a run deterministic without a provider — so plans 28-08/28-12/28-16 can build assertions, the runner and the E2E dogfood on it.**

## Performance

- **Duration:** ~28 min of executor work (dispatch 2026-09-08T23:22Z → last commit 23:50Z), then an orchestrator close-out after a session crash
- **Started:** 2026-09-08T23:22:01Z
- **Completed:** 2026-09-09T00:25:07Z
- **Tasks:** 3/3 (Task 1 checkpoint auto-resolved; Tasks 2 and 3 implemented and committed)
- **Files modified:** 8 (7 created, `Cargo.lock` updated)

## Checkpoint Status

Task 1 `checkpoint:decision` (gate `blocking`, not `blocking-human`) — **option-a auto-selected by the orchestrator under `--auto` (2026-09-08)**: proceed as decided, `publish = true`, D-27 depends-downward dependency shape. Recorded in `crates/paladin-eval/Cargo.toml`'s manifest comment.

## Accomplishments

- `crates/paladin-eval` joins the workspace as a composition-tier tool crate: `publish = true`, workspace version, depends on `paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm` (`default-features = false`, `mock` only) and `paladin-storage` (`sqlite`); never on `paladin-ai`, and `paladin-ai`'s default `cargo tree` contains no `paladin-eval` edge (X-11.4).
- The scenario file format (`scenario.rs`, 1092 lines incl. tests): `Scenario` with `schema_version`, `target` (`graph_doc` path or `registered` constructor name), `store` (defaults to `in_memory`), global and per-node `llm` scripts (`text` / `tool_call` / `error` entries plus `match` prompt-substring rules), `cases` with `input`, `interrupt_after_superstep`, `parley_responses` and the full PRD 07 OBS-FR-12 `Assertion` list, `live.allow_content_assertions`; YAML and JSON forms deserialize to equal values; an unknown `schema_version` is a typed `ScenarioError`.
- `ScenarioLlm` (`scripted_llm.rs`, 518 lines incl. tests) implements `LlmPort`: `for_node` routing, sequence consumed one entry per call, match rules checked first, `error` entries returning the named `LlmError` kind, `tool_call` entries carrying a function call, and `CapturedRequest` recording via `requests()`.
- `docs/schemas/eval-scenario.schema.json` (697 lines) is derived with `schemars` and golden-tested (`tests/schema_golden.rs`), blessed with `UPDATE_EVAL_SCHEMA=1` exactly like `UPDATE_WARGRAPH_SCHEMA=1`.

## Task Commits

1. **Task 1: Confirm the one-way publishing posture** — no commit (checkpoint auto-resolved, see above)
2. **Task 2 (tracer): Crate skeleton and one scenario file parsed end to end** — `f00c156c` (feat)
3. **Task 3: `ScenarioLlm` and the golden JSON Schema** — `74b086ed` (feat)

**Merge into `feature/phase-26`:** `ba7862d9` (chore: merge executor worktree)
**Plan metadata + ADR-number correction:** this SUMMARY's commit (docs)

## Files Created/Modified

- `crates/paladin-eval/Cargo.toml` — manifest: `publish = true`, D-27 dependency shape, crate metadata
- `crates/paladin-eval/README.md` — crate overview and hexagonal position
- `crates/paladin-eval/src/lib.rs` — crate docs (safety argument T-28-05-01), module wiring, re-exports
- `crates/paladin-eval/src/scenario.rs` — the scenario file format types, `EVAL_SCHEMA_VERSION`, `ScenarioError`, `Scenario::from_path`
- `crates/paladin-eval/src/scripted_llm.rs` — `ScenarioLlm`, `ScenarioLlmError`, `CapturedRequest`
- `crates/paladin-eval/tests/schema_golden.rs` — schemars golden test with the `UPDATE_EVAL_SCHEMA=1` bless path
- `docs/schemas/eval-scenario.schema.json` — the committed golden schema
- `Cargo.lock` — new crate and its `serde_yaml`/`schemars` entries

## Decisions Made

See `key-decisions` in the frontmatter. Most consequential: `paladin-llm` is consumed with `default-features = false, features = ["mock"]` so a downstream dev-dependency on `paladin-eval` pulls no provider adapter; and the runner's dependencies (`libtest-mimic`, `glob`) are deliberately deferred to plan 28-12.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Correctness] ADR number in crate docs corrected 0047 → 0048**
- **Found during:** orchestrator close-out
- **Issue:** `src/lib.rs` and `README.md` cited `.planning/decisions/0047-paladin-eval-composition-crate.md` (28-CONTEXT.md's number); 28-RESEARCH.md established the next free ADR is **0048** and plan 28-17 asserts `0047-…` does not exist.
- **Fix:** both references now name `0048-paladin-eval-composition-crate.md` / ADR-0048.
- **Files modified:** `crates/paladin-eval/src/lib.rs`, `crates/paladin-eval/README.md`
- **Verification:** `grep -rn 0047 crates/paladin-eval` is empty; `cargo doc -p paladin-eval --no-deps` clean.
- **Committed in:** this SUMMARY's commit.

**2. [Process] Orchestrator close-out after a session crash**
- **Found during:** wave 2 execution — the Claude Code session died after the executor's Task 3 commit (`74b086ed`, 23:50Z) and before its verification pass and SUMMARY.
- **Issue:** the worktree was clean with both implementation commits present but no SUMMARY.md; the executor agent was no longer addressable.
- **Fix:** the orchestrator merged the clean worktree through the manifest path (`worktree.cleanup-wave` → `ba7862d9`), re-ran the plan's full verification on the merged checkout (all green, recorded in `coverage`), and authored this SUMMARY — the workflow's `close out manually` recovery option.
- **Verification:** see the Self-Check below.

---

**Total deviations:** 2 (1 correctness auto-fix, 1 process recovery). **Impact on plan:** none on scope; the crate is exactly what D-27/D-28/D-30 specify.

## Issues Encountered

The session crash above. No implementation problems were left open: the two landed commits compile and pass under `cargo check --workspace --all-targets --all-features`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- 28-08 can implement the assertion evaluators against `Assertion` and the captured trace records.
- 28-12 adds `libtest-mimic`/`glob`, the `evals` harness target and `paladin-cli eval run`; the 28-05 PLAN artifact row `Cargo.toml contains libtest-mimic` is satisfied there.
- 28-17 writes ADR-0048 (the number the crate docs now cite) and registers the twelfth crate in the release/semver/api-surface lists.

## Self-Check: PASSED

- `cargo fmt --all --check` — exit 0
- `cargo check --workspace --all-targets --all-features` — exit 0
- `cargo clippy --all-targets --all-features -p paladin-eval -- -D warnings` — exit 0
- `cargo doc -p paladin-eval --no-deps` — exit 0, no warnings
- `cargo test -p paladin-eval` — 13 unit + 1 golden + 4 doc tests passed, 0 failed
- `cargo tree -e features -p paladin-ai | grep -c paladin-eval` — `0`
- `git diff --quiet dfc64a70..ba7862d9 -- crates/paladin-llm/src/mock.rs` — exit 0 (byte-identical)

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
