---
phase: 50-provider-parity-config-driven-model-swap
plan: 01
subsystem: testing
tags: [rust, paladin-llm, provider-swap, mockito, tdd, user-turn, openai, parity]

requires:
  - phase: 49
    provides: "the [[test]] required-features registration shape and the red-evidence file shape this plan copies"
provides:
  - "crate-private paladin_llm turn::user_turn_text: the one user-turn text rule all nine providers call"
  - "the OpenAI empty-user-message defect (handoff B-4, PARITY-08) fixed, with an end-to-end and an adapter-level test that were red first"
  - "tests/provider_swap_matrix.rs: 49 tests over nine providers built from an LlmProviderConfig block (30 green, 19 ignored with the fixing plan named)"
  - "50-MATRIX-RED-EVIDENCE.md: the pre-fix and --ignored run record, case by case"
affects: [50-03, 50-05, 50-10, 50-12, 50-13, 50-14]

tech-stack:
  added: []
  patterns:
    - "Red-first matrix: every case the tree fails is #[ignore = \"red until plan 50-NN: ...\"] naming the fixing plan; the fixing plan un-ignores it without weakening the assertion"
    - "One shared helper for a per-adapter convention that had silently split (user turn text), called from every request builder"
    - "Config-only provider switch in tests: matrix_block + port_from_config over the existing From<&LlmProviderConfig> bridges"

key-files:
  created:
    - tests/provider_swap_matrix.rs
    - crates/paladin-llm/src/turn.rs
    - .planning/phases/50-provider-parity-config-driven-model-swap/50-MATRIX-RED-EVIDENCE.md
  modified:
    - Cargo.toml
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs

key-decisions:
  - "The matrix asserts the query is contained in the last user turn, not equal to it: the execution service folds the system prompt and the query into the one string it passes as UserPrompt::query"
  - "Cases found green on the current tree (Gemini same-agent, Anthropic empty-stop) are left un-ignored although the plan's map assigned them to 50-05 and 50-13"
  - "PARITY-01 is not marked complete: its text requires the matrix to pass for all nine providers, and 19 cases are red by design until 50-03/05/12/13/14"

patterns-established:
  - "Mock vendor per test (own mockito server, records path and JSON body), so cases share no state"
  - "Refusal oracle: a build refusal passes only if it names the provider (matrix name or the port's own name), the model and the parameter"

requirements-completed: [PARITY-08]

coverage:
  - id: D1
    description: "OpenAI sends the query on the execution-service path (PaladinBuilder -> PaladinExecutionService -> mock wire), red before the fix"
    requirement: "PARITY-08"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all openai_prompt_reaches_the_wire"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib openai_user_turn_carries_query_on_the_execution_service_path"
        status: pass
    human_judgment: false
  - id: D2
    description: "All nine providers build the user turn through crate::turn::user_turn_text (query, one blank line, context; text unchanged)"
    requirement: "PARITY-08"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib turn:: user_turn_appends_context"
        status: pass
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all context_reaches_the_wire"
        status: pass
    human_judgment: false
  - id: D3
    description: "Nine-provider provider-swap matrix with a recorded red run; 19 red cases ignored with their fixing plan"
    requirement: "PARITY-01"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all (30 passed, 19 ignored)"
        status: pass
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all -- --ignored (19 failed, as recorded)"
        status: pass
    human_judgment: false

duration: 32min
completed: 2026-10-15
status: complete
---

# Phase 50 Plan 01: Provider-Swap Tracer and Matrix Summary

**OpenAI user-turn defect (B-4) fixed through one shared `user_turn_text` helper that all nine providers now call, plus a 49-test, config-switched provider-swap matrix with a recorded red run and every red case ignored under the plan that fixes it.**

## Performance

- **Duration:** about 32 min
- **Started:** 2026-10-15T14:12:00Z (approximate)
- **Completed:** 2026-10-15T14:44:00Z
- **Tasks:** 3 (one tracer, one TDD, one auto)
- **Files modified:** 10 (3 created)

## Accomplishments

- The tracer runs end to end: an `llm.openai`-shaped block, the adapter built from it, `PaladinBuilder`, `PaladinExecutionService::execute` and a mock wire. It was red on the unfixed tree (the user message went out as `""`) and is green after the fix.
- `crates/paladin-llm/src/turn.rs` holds `pub(crate) fn user_turn_text`. A part is present when non-empty after trimming; present parts are joined by one blank line; each part's own text is sent unchanged (multibyte tested). OpenAI, Anthropic, DeepSeek, Gemini and `CompatEngine` (Kimi, Qwen, Grok, Ollama, OpenAI-compatible) all call it.
- `tests/provider_swap_matrix.rs` (registered with `required-features = ["llm-all"]`) covers all nine providers: prompt, model and context on the wire; the same agent equivalent-or-refused-by-name; Kimi fixed temperature (explicit and auto); empty stop completion typed; the fallback hop's model.
- `50-MATRIX-RED-EVIDENCE.md` records the pre-fix OpenAI output and the full `--ignored` run, with a table naming the fixing plan per red case.

### Pre-fix failing assertions (acceptance criterion)

- End to end (`openai_prompt_reaches_the_wire`): `openai: the query must reach the wire inside the last user message; body was {"max_tokens":4096,"messages":[{"content":"","role":"user"}],"model":"matrix-openai-model",...}` (content empty).
- Adapter level (`openai_user_turn_carries_query_on_the_execution_service_path`): `assertion left == right failed  left: ""  right: "What is 2+2?"`.
- The four sibling `*_user_turn_appends_context` tests failed 4 of 4 before Task 2.

### Matrix outcome

| | Count | Detail |
|---|---|---|
| Green, un-ignored | 30 | prompt, model, context on the wire for all nine; Gemini (b); Anthropic (e); `matrix_covers_every_provider` |
| Ignored, red | 19 | 50-03: 7, 50-05: 3, 50-12: 7, 50-13: 1, 50-14: 1 |

Final test names (macro-generated, `<provider>_<case>`): `kimi_fixed_temperature_explicit`, `kimi_fixed_temperature_auto`, `fallback_hop_gets_its_own_model`, `<provider>_prompt_reaches_the_wire`, `<provider>_model_reaches_the_wire`, `<provider>_context_reaches_the_wire`, `<provider>_same_agent_is_equivalent_or_refused_by_name`, `<provider>_empty_stop_completion_is_typed` (`openai_compatible` for the hyphenated provider).

## Task Commits

1. **Task 1: Tracer, OpenAI from a config block through to the wire, red then fixed** - `e4399ab0` (fix)
2. **Task 2: Other four request builders on the shared helper** - `0206d031` (fix; tests written and run red first)
3. **Task 3: Nine-provider matrix, red cases ignored, red evidence** - `27109e42` (test)

**Plan metadata:** recorded in the final `docs(50-01)` commit.

## Files Created/Modified

- `crates/paladin-llm/src/turn.rs` - the shared user-turn text helper and its 9 unit tests
- `crates/paladin-llm/src/lib.rs` - `mod turn;` behind the nine-provider `cfg(any(...))` gate
- `crates/paladin-llm/src/openai/adapter.rs` - `PromptType::User` arm calls the helper; `user_turn_wiring` adapter test
- `crates/paladin-llm/src/{anthropic,deepseek,gemini}/adapter.rs`, `compat/engine.rs` - same one-line switch; one `*_user_turn_appends_context` test each
- `tests/provider_swap_matrix.rs` - the matrix (`MockVendor`, `matrix_block`, `port_from_config`, `matrix_case!`)
- `Cargo.toml` - `[[test]] provider_swap_matrix`, `required-features = ["llm-all"]`
- `.planning/phases/50-.../50-MATRIX-RED-EVIDENCE.md` - the red-before-fix record

## Decisions Made

- The wire assertion for the execution-service path is `contains(query)`, not equality (see Deviation 1).
- Green-on-arrival cases stay un-ignored, per the plan's own rule ("a case found green is not ignored"): `gemini_same_agent_is_equivalent_or_refused_by_name` (Gemini already sends `generationConfig.temperature` and `stopSequences`) and `anthropic_empty_stop_completion_is_typed` (the Anthropic adapter already raises a permanent `Empty completion`). 50-05 and 50-13 therefore have one fewer case to un-ignore than their map suggested.
- PARITY-01 stays open in REQUIREMENTS.md; only PARITY-08 is marked complete.
- `port_from_config` uses the existing `From<&LlmProviderConfig>` bridges for all nine providers, as flagged assumption A1 expected. `OpenAiCompatibleConfig` is a `TryFrom` and compiled fine under `llm-all`; the matrix block names `base_url` and `default_model`, so no fallback to `*Config::new` was needed. Ollama ignores the block's `api_key` by design.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Tracer assertion in the plan could not hold on the execution-service path**
- **Found during:** Task 1 (after the OpenAI fix, the test stayed red)
- **Issue:** The plan asserts the last user message `content` equals `MATRIX_QUERY`. `PaladinExecutionService` builds one prompt string (system prompt, `User: <input>`, plus `Previous output:` on later loops) and passes that as `UserPrompt::query`. The wire content is therefore never the bare query. With the default loop count a second request also went out carrying `Previous output: Paris`.
- **Fix:** The matrix agent sets `.max_loops(1)` (one request per run, as `build_agent_with_llm` does when a definition sets it), and the assertion is `content.contains(MATRIX_QUERY)`, byte for byte. The adapter-level test keeps the exact-equality assertion the plan specified. The corrected assertion was re-run red against the unfixed adapter (content `""`), so it still proves the defect.
- **Files modified:** `tests/provider_swap_matrix.rs`
- **Committed in:** `e4399ab0`

**2. [Rule 3 - Blocking] Matrix cases needed to reach the plan's 40-test floor**
- **Found during:** Task 3
- **Issue:** The plan's cases (a, b, e for nine providers plus Kimi and fallback) give 30 tests, but its acceptance criterion asks for at least 40.
- **Fix:** Added two all-nine cases that are green and meaningful: `*_model_reaches_the_wire` (the agent's model on the wire; Gemini carries it in the path) and `*_context_reaches_the_wire` (the shared PARITY-08 rule across all nine providers via a direct `generate`), plus `matrix_covers_every_provider`. Total 49.
- **Files modified:** `tests/provider_swap_matrix.rs`
- **Committed in:** `27109e42`

**3. [Process] Tracer feedback gate satisfied by re-running the automated verify**
- **Found during:** after the Task 1 commit
- **Issue:** The executor protocol says an interactive run (auto mode off) stops at a `checkpoint:human-verify` after the tracer. This plan was spawned as a sequential executor with the instruction to run to completion, and the tracer's `<verify>` is fully automated.
- **Action:** Re-ran the tracer's `<verify>` end to end (both tests green) and logged "Tracer verified end-to-end, expanding" before Task 2, which is the autonomous-mode form of the gate. No human checkpoint was returned.

**Total deviations:** 3 (1 bug in the plan's assertion, 1 blocking acceptance-count gap, 1 process note). **Impact on plan:** none on scope; the defect is fixed and proven red-then-green.

## Issues Encountered

- **Disk exhaustion.** A `cargo test --workspace` run exhausted the sandbox quota (`No space left on device` while linking `e2e_muster_defer_order` and the `paladin-battalion`/`paladin` lib tests). Recovered by deleting build leftovers (`*.tmp*` in `target/debug/deps`, `*-working` incremental sessions) and then the regenerable `target/debug/incremental` cache (about 4.3 GB). No `cargo clean`, no extra target directory. The full workspace test could not be completed; targeted verifies stood in and are recorded in `.planning/WINDOWS.md` as an `unrun-verify` entry. The change is confined to `paladin-llm` request builders plus a new test target; a grep found no other test in the repo with a body matcher or a `context: Some(..)` user prompt that the change could affect.
- **Kimi explicit temperature has two causes.** `kimi_fixed_temperature_explicit` is red both because the compat engine omits `temperature` for a declared `(1, 1)` provider and because the refusal of other values does not name the model. Both are 50-03's to settle; the assertion stands as the plan wrote it.

## Verification

- `cargo test --test provider_swap_matrix --features llm-all`: 30 passed, 19 ignored
- `cargo test --test provider_swap_matrix --features llm-all -- --ignored`: 19 failed (recorded)
- `cargo test -p paladin-llm --all-features --lib`: 651 passed
- `cargo fmt --check`, `cargo clippy -p paladin-llm --all-features --all-targets -- -D warnings`, `cargo clippy --all-targets --features llm-all -- -D warnings`: clean
- `make check-consumer-surface`: 1 passed; `make api-surface`: unchanged (4274 items)
- Manual credential-handling review: the only key is the literal `matrix-test-key` sent to a local mock; no env var read, nothing logged, no adapter client or redirect policy touched; `turn` is crate-private.

## Known Stubs

None. The 19 ignored cases are deliberate, not stubs, and are tracked in `.planning/WINDOWS.md` (entry 71) so `/gsd-ship` stays blocked until the fixing plans un-ignore them.

## Threat Flags

None. No new network endpoint, auth path or schema surface: the helper only decides the text of a field adapters already sent (T-50-01-01 mitigated by byte-for-byte text and the multibyte tests; T-50-01-03 mitigated by the per-case plan names and the evidence file).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Wave 2 plans can un-ignore their cases by name: 50-03 (7 cases: the five compat `same_agent`, `kimi_fixed_temperature_explicit`, `kimi_fixed_temperature_auto`), 50-05 (3: openai, anthropic, deepseek `same_agent`), 50-12 (7: `empty_stop_completion_is_typed` for openai, deepseek, kimi, qwen, grok, ollama, openai_compatible), 50-13 (gemini `empty_stop_completion_is_typed`), 50-14 (`fallback_hop_gets_its_own_model`). 50-10 replaces the body of `port_from_config` only.
- Concern: `cargo test --workspace` needs more headroom than this sandbox has; later plans should keep using targeted runs and `CARGO_INCREMENTAL=0`.

---
*Phase: 50-provider-parity-config-driven-model-swap*
*Completed: 2026-10-15*

## Self-Check: PASSED

All created files exist (`tests/provider_swap_matrix.rs`, `crates/paladin-llm/src/turn.rs`, `50-MATRIX-RED-EVIDENCE.md`, this summary) and commits `e4399ab0`, `0206d031` and `27109e42` are present in `git log`.
