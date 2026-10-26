---
phase: 50-provider-parity-config-driven-model-swap
plan: 08
subsystem: api
tags: [rust, paladin-llm, config, non-exhaustive, model-registry, adr-0061, parity, debug-redaction]

requires:
  - phase: 50
    provides: "plan 50-03: ModelOverride, ModelRegistry::with_overrides, sampling::check_max_tokens, CompatEngine::with_model_registry; plan 50-06: dated seeded per-model tables and new default models; plan 50-07: effective_capabilities and Commissary::for_model reading the per-model window"
provides:
  - "max_tokens, max_retries, context_window and model_overrides on all nine adapter configs (OpenAIConfig without a model field; AnthropicConfig keeps its required u32 max_tokens), each read by its adapter, with #[must_use] with_max_tokens / with_max_retries / with_context_window / with_model_overrides builders"
  - "all nine configs #[non_exhaustive] (owner decision option-a); every new() / from_env() / from_parts() signature unchanged"
  - "manual Debug that prints the API key as <redacted> on the eight key-carrying configs (OllamaConfig has no key)"
  - "registry::is_reasoning_model, registry::REASONING_DEFAULT_TIMEOUT_SECONDS (300), registry::default_timeout_seconds, registry::DEFAULT_MAX_RETRIES, ModelRegistry::with_context_window (all crate-private)"
  - "an adapter-wide context_window that beats seeded windows, with a per-model override beating both"
affects: [50-09, 50-10, 50-14, 50-16, 51]

tech-stack:
  added: []
  patterns:
    - "One macro (registry::config_knob_builders!) generates the four with_* builders and their rustdoc for all nine configs, so the knob docs cannot drift between types"
    - "Crate-level #[cfg(test)] test_support (mock server, capturing mock, send_with_max_tokens, attempt counter) shared by every adapter's wire tests and not gated on any adapter feature"
    - "Adapter-wide window folded into the registry (ModelRegistry::with_context_window), so it reaches model_profile, effective_capabilities and Commissary::for_model with no branch in the readers"

key-files:
  created:
    - crates/paladin-llm/src/test_support.rs
  modified:
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/mod.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/config/bridge.rs
    - crates/paladin-llm/src/redaction.rs
    - crates/paladin-llm/src/lib.rs
    - examples/vision_analysis.rs
    - examples/vision_battalion.rs
    - src/infrastructure/cadence.rs
    - tests/integration/provider_switching_test.rs
    - tests/unit/llm/anthropic_adapter_test.rs
    - tests/unit/llm/deepseek_adapter_test.rs
    - CHANGELOG.md

key-decisions:
  - "option-a (owner, Task 1): all nine configs #[non_exhaustive], every existing constructor signature unchanged, four #[must_use] with_* builders"
  - "Harmonising the drifted new(..) signatures is a separate, later registered break, recorded in .planning/todos/pending/2026-10-16-harmonise-llm-config-new-signatures.md (owner, Task 1); no signature was touched here"
  - "Anthropic and Gemini keep their long-standing retry counter (max_retries is the total attempt budget there); the compat presets, OpenAI and DeepSeek retry max_retries times after the first attempt; the rustdoc says so and the unification is left for later"
  - "A configured max_tokens is held to the same check_max_tokens limit as a request value on all nine adapters (refused, never clamped)"

patterns-established:
  - "A knob is added only with a behaviour test that fails when its read is removed (PARITY-03 prohibition); verified by mutating five reads and watching six tests fail"

requirements-completed: []

coverage:
  - id: D1
    description: "the compat engine sends a configured default max_tokens when a request sets none, a request value wins, none configured sends no key, and the default is refused above the profiled maximum output before any HTTP call"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "kimi_/qwen_/grok_/ollama_/openai_compatible_max_tokens_default_reaches_the_wire; compat_max_tokens_default_is_checked_against_the_profile (kimi); openai_/anthropic_/deepseek_/gemini_ max_tokens tests"
        status: pass
    human_judgment: false
  - id: D2
    description: "max_retries is read by every adapter (the five presets, OpenAI, Anthropic, DeepSeek, Gemini), and a 429 stays a single attempt"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "<provider>_max_retries_is_read_by_the_engine / _by_the_adapter; compat_engine_honours_configured_max_retries; anthropic_429_is_one_attempt_whatever_the_retry_budget"
        status: pass
    human_judgment: false
  - id: D3
    description: "an adapter-wide context_window beats a seeded window, a per-model override beats both, a window below a seeded maximum output fails construction naming provider, model and both numbers, and an invalid override fails construction naming provider and model"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "context_window_override_beats_a_seeded_window (kimi) and ..._on_openai/_anthropic/_deepseek/_gemini; kimi_model_override_wins_and_invalid_override_fails_construction; registry::knob_tests::with_context_window_*; qwen_context_window_below_a_seeded_max_output_fails_construction; for_model_reads_the_adapter_wide_window_knob_over_a_seeded_window (commissary)"
        status: pass
    human_judgment: false
  - id: D4
    description: "the direct constructors default a seeded reasoning model to a 300 s timeout, any other model keeps its compiled default, and an explicit value always wins"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "<config>_defaults_a_reasoning_model_to_300s for kimi, qwen, grok, ollama, openai_compatible, anthropic, deepseek, gemini; registry::knob_tests::is_reasoning_model_prefers_the_override"
        status: pass
    human_judgment: false
  - id: D5
    description: "no key reaches Debug output: the eight key-carrying configs print <redacted>, an empty key prints empty, every other field is still printed (D-00a, T-50-08-01)"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "<config>_debug_redacts_the_api_key (eight types); redaction::redacted_key_tests"
        status: pass
    human_judgment: false
  - id: D6
    description: "the tree builds and tests pass with the new shape: every struct literal of the nine types in the workspace is migrated, the consumer muster and the swap matrix are unaffected"
    requirement: "PARITY-03"
    verification:
      - kind: integration
        ref: "cargo build --workspace --all-targets --all-features; cargo test --workspace --no-run --features llm-all; make check-consumer-surface; cargo test --test provider_swap_matrix --features llm-all (40 passed, 9 ignored for later plans); cadence_tracer_paces_a_real_openai_429_end_to_end; cargo test --test unit llm:: (37 passed); the atomic-commit coverage awk prints nothing"
        status: pass
    human_judgment: false

duration: ~65min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 08: Configuration knobs on every adapter config Summary

**Owner decision at the Task 1 checkpoint (2026-10-16), recorded verbatim: `option-a` -- "option-a but we also want to record fixing the signature drift as a todo."** All nine `*Config` types are `#[non_exhaustive]`, every `new()` / `from_env()` / `from_parts()` signature is unchanged, and `with_max_tokens`, `with_max_retries`, `with_context_window` and `with_model_overrides` are `#[must_use]` builders with rustdoc. The todo for harmonising the drifted `new(..)` signatures is already recorded at `.planning/todos/pending/2026-10-16-harmonise-llm-config-new-signatures.md` (commit `6b7ca9b7`); this plan did not touch any signature.

**Every adapter config now carries `max_tokens`, `max_retries`, `context_window` and `model_overrides`, each read by its adapter: an adapter-wide window beats seeded windows (a per-model override beats both), a seeded reasoning model defaults to a 300 s timeout, and API keys never reach `Debug`.**

## Performance

- **Duration:** about 65 min for the continuation (start time not captured; estimated from the todo commit at 11:00 UTC and the last build)
- **Completed:** 2026-10-16
- **Tasks:** 3 of 3 (Task 1 was the decision checkpoint; Tasks 2 and 3 produced three commits)
- **Files:** 22 modified, 1 created

## Accomplishments

- **Five compat presets (Task 2).** `KimiConfig`, `QwenConfig`, `GrokConfig`, `OllamaConfig`, `OpenAiCompatibleConfig` gain the four fields. `CompatEngineConfig.max_retries` comes from the config; the engine gains a private `default_max_tokens` (`with_default_max_tokens`, no field on the public `CompatEngineConfig`) that stands in for an unset request value and goes through `sampling::check_max_tokens` exactly as a request value does. `context_window` replaces `get_capabilities().max_context_tokens` and the fallback profile's window and is folded into the registry; `model_overrides` attach through `with_overrides`.
- **Registry (Task 2).** `ModelRegistry::with_context_window(Option<u32>)` overrides the window of the fallback and of every seeded entry at resolve time and validates the merged result per model (zero, or below a seeded maximum output, is an error naming provider, model and both numbers; never clamps). `is_reasoning_model(provider, model, overrides)` (override, else seed, else false, exact match), `REASONING_DEFAULT_TIMEOUT_SECONDS`, `default_timeout_seconds` and `DEFAULT_MAX_RETRIES` are crate-private.
- **Four bespoke adapters (Task 3).** OpenAI (`new` / `from_env` set `max_tokens: Some(4096)`; `None` sends no key), Anthropic (required `u32` `max_tokens` kept, `new(api_key, base_url, model, max_tokens)` unchanged, the literal `3` in `execute_with_retry` replaced), DeepSeek and Gemini (default output budget, retries, window, overrides). Gemini's `maxOutputTokens` carries the default.
- **D-00a.** Manual `Debug` on eight configs via a shared `RedactedKey`; Ollama has no key.
- **Migration.** The six root-crate struct-literal sites (two examples, `src/infrastructure/cadence.rs`, three test files, seven literals) became `new(..)` plus field assignment, keeping every value; the nine `Self { .. }` conversions in `config/bridge.rs` gained the new fields; ten in-crate OpenAI test literals gained the new fields.
- **Tests:** 77 new tests (registry 8, compat engine 1, five presets ~35, four bespoke 36 incl. `anthropic_new_keeps_its_four_argument_signature_and_defaults`, commissary 1, redaction 1).

## Task Commits

1. **Task 1: public shape of the nine configs** - decision only, no commit (owner: `option-a`)
2. **Task 2 RED: failing tests for the five presets** - `bfbc93ff` (test)
3. **Task 2 GREEN: the five presets read their knobs** - `6d842be5` (feat)
4. **Task 3: every adapter config carries the PARITY-03 knobs** - `70d6cb48` (feat; fields, readers, every literal migration and the CHANGELOG in one commit per A4-50-08)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md and ROADMAP.md.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 869 passed. `cargo test --workspace --lib --bins --no-fail-fast`: every binary green except the two ledgered baseline failures (`build_run_api_persists_no_run_traces_by_default`, `..._persists_run_traces_when_trace_persist_is_set`, WINDOWS.md row 73); root crate 1360 passed, 2 failed, `paladin-llm` 875 passed. No new failure.
- `cargo build --workspace --all-targets --all-features` and `cargo test --workspace --no-run --features llm-all` exit 0 after Task 2 and after Task 3; single-feature `cargo test -p paladin-llm --no-default-features --features <each adapter> --lib --no-run` compile for all nine adapters.
- `cargo fmt --check` clean; `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean; `cargo clippy -p paladin-llm --lib -- -D warnings` clean for default, `--no-default-features` and each of kimi, anthropic, gemini, deepseek, ollama, openai-compatible alone. `--tests` under a single feature still reports only the pre-existing `PinnedSpread` / `pin_waiter_spread` dead-code errors in `cadence.rs` (baseline).
- `make check-consumer-surface`, `make check-api-examples`, `make check-migration-allowlist`, `make api-surface` (unchanged, 4275 items), `scripts/check-changelogs.sh`: pass. `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored (later plans). `cadence_tracer_paces_a_real_openai_429_end_to_end`: pass. `cargo test --test unit llm::`: 37 passed.
- Atomic-commit coverage check (A4-50-08) prints nothing; `grep -rnE '\b(<nine>Config) \{' --include=*.rs . | grep -v target` lists no hit outside `crates/paladin-llm`. `grep -n 'max_retries: 3'` over the five preset adapters lists nothing (the defaults come from `DEFAULT_MAX_RETRIES`).
- **RED evidence.** Task 2: 37 new tests failing at `bfbc93ff` against the inert fields. Task 3 (single commit, so no committed RED): tests were run after mutating five reads (OpenAI and Gemini `max_tokens` default, Anthropic and DeepSeek `max_retries`, Gemini `with_context_window`); six knob tests failed, then the mutations were reverted.
- **Manual credential-handling review (D-00a).** The diff touches no HTTP client, header or redirect policy (`Policy::none()` untouched in all nine adapters). The only change near credentials is `RedactedKey` and the manual `Debug` impls; each has a test asserting the literal key is absent and `<redacted>` present. Error text built from overrides names only provider, model and numbers.
- **Threat model.** T-50-08-01 mitigated (eight redacting `Debug` impls plus tests); -02 and -05 mitigated (`ModelOverride::validate` and `with_context_window` refuse at construction, naming provider and model; no clamping); -04 mitigated (a failing-when-removed test per knob); -03 accepted as planned.

## Decisions Made

See `key-decisions`. Flagged assumptions: A1 applied (OpenAI has no `model` field), A2 applied (the 300 s default is decided from the seeded entry on the direct path; an override-only `reasoning: true` needs `timeout_seconds` set explicitly, stated in each `timeout_seconds` rustdoc and the `with_model_overrides` doc), A3 and A4 applied (semver register left to 50-10; fields and literals in one commit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan conflict - Rule 1] Anthropic and Gemini retry convention preserved; the plan's "set 1, observe two attempts" does not hold**
- **Found during:** Task 3
- **Issue:** the plan expects an Anthropic test with `max_retries` 1 to observe two attempts on a network error. Anthropic's (and Gemini's) loop treats the number as the total attempt budget (`attempt >= max_retries`, so the old literal `3` meant three calls), and DeepSeek's comment records that Phase 41 deliberately did not unify the counters. The plan also says the default is "each loop's current hard-coded count", which only holds if the value is passed through unchanged.
- **Fix:** passed `config.max_retries` through unchanged (default 3 keeps exactly today's attempt counts everywhere), documented the two conventions on the `max_retries` fields and in the shared `with_max_retries` rustdoc, and wrote tests to the real behaviour (Anthropic and Gemini: budget 1 sends once, 2 sends twice; a 429 is one attempt whatever the budget). Unifying the conventions would change default latency and cost and is left for a later change.
- **Files modified:** `crates/paladin-llm/src/{anthropic,gemini}/adapter.rs`, `crates/paladin-llm/src/registry/mod.rs`
- **Commit:** `70d6cb48`

**2. [Rule 3 - Blocking] Mock-endpoint test helpers moved to a crate-level `test_support`**
- **Found during:** Task 3 verification (`cargo test --workspace --lib` failed to build `paladin-llm`)
- **Issue:** the first version put the shared helpers in `compat::test_support`, but `compat` is compiled only when a compat preset feature is on, so the OpenAI, Anthropic, DeepSeek and Gemini knob tests broke any build without a preset.
- **Fix:** new `crates/paladin-llm/src/test_support.rs` (`#[cfg(test)]`, not feature-gated); `compat::test_support` re-exports it so the existing preset tests are unchanged. `lib.rs` is not in the plan's file list.
- **Files modified:** `crates/paladin-llm/src/test_support.rs` (new), `crates/paladin-llm/src/lib.rs`, `crates/paladin-llm/src/compat/mod.rs`
- **Commit:** `70d6cb48`

**3. [Rule 2 - Missing critical] Shared helpers for the nine types instead of nine copies**
- **Issue:** the plan lists per-adapter edits only; nine hand-copied builder sets, redaction impls and timeout rules would drift.
- **Fix:** `registry::config_knob_builders!` (the four builders and their rustdoc), `redaction::RedactedKey`, `registry::default_timeout_seconds` and `registry::DEFAULT_MAX_RETRIES`. `redaction.rs` and the macro are not in the plan's file list.
- **Commits:** `bfbc93ff`, `6d842be5`, `70d6cb48`

**4. [Plan acceptance not satisfiable as written] `.project/current-exports.txt` is unchanged**
- **Found during:** Task 3
- **Issue:** `make api-surface` reports the surface unchanged (4275 items) and `grep -c model_overrides .project/current-exports.txt` prints 0, not at least 9. The baseline covers only the root crate, which re-exports just `OpenAIConfig`, `AnthropicConfig` and `DeepSeekConfig` by name (the same limitation plans 50-03, 50-06 and 50-07 recorded); the fields live in `paladin-llm`. `make api-surface-update` would write no change and was not run. The CHANGELOG entries the plan asks for are written. The `paladin-llm` surface is measured in plan 50-10 (A3-50-08).
- **Files modified:** none

**5. [TDD gate] Task 3 has no separate RED commit**
- The plan requires the fields, readers, every literal migration and the changelog in ONE commit (A4-50-08), so a committed failing-test state cannot compile. Task 2 has the RED (`bfbc93ff`) and GREEN (`6d842be5`) commits; its RED scaffold added inert fields and builders so the tree still compiled, and `is_reasoning_model`, a pure lookup, was implemented with that scaffold, so its test passes at RED. Task 3's RED was observed by mutation (see Verification).

**6. [Rule 1 - existing tests encoded the old default] Four default-timeout assertions updated**
- `kimi_config_defaults_base_url_and_model_when_only_key_is_set`, the Qwen and Grok equivalents and `gemini_config_from_parts_defaults_base_url_model_and_timeout_when_only_key_is_set` asserted 60 s for a default model that is a seeded reasoning model; they now assert 300 s (D-13), with a comment saying why.

### Notes, not deviations

- **Ollama.** `OllamaConfig` has no API key, so it keeps the derived `Debug` and has no `<config>_debug_redacts_the_api_key` test: eight manual `Debug` impls, not nine.
- **Anthropic 300 s.** No Anthropic entry is seeded as a reasoning model, so `anthropic_config_defaults_a_reasoning_model_to_300s` can only assert the 60 s side; the 300 s path is covered by DeepSeek, Gemini, Kimi, Qwen and Grok and by `is_reasoning_model_prefers_the_override`. `ANTHROPIC_TIMEOUT_SECONDS` precedence is code-level (`from_env` reads the variable first); it is not unit-tested because `env::set_var` is `unsafe` and the crate denies `unsafe_code`.
- **`LlmProviderConfig.max_retries`** is now honoured by the eight `config/bridge.rs` conversions that did not read it (it was inert for them); the CHANGELOG `### Changed` entry says so. The bridges' timeouts and defaults are otherwise unchanged; plan 50-09 replaces them with the layered path.
- **Grep acceptance counts.** `grep -c 'fn is_reasoning_model'` prints 2 and `grep -c 'fn with_context_window'` prints 7 because the planned test name `is_reasoning_model_prefers_the_override` and the macro, registry and test functions match the substring; each definition exists exactly once.
- **mdBook.** `docs/src/appendix/sentinel.md` still shows `OpenAIConfig { .. }` literals in non-compiled blocks; plan 50-16 rewrites them as the plan says.
- **Environment.** The sandbox filesystem filled during the first RED build; cleared by deleting `target/debug/incremental` and superseded duplicate artifacts under `target/debug/deps`. No tracked file was affected.

## Issues Encountered

None blocking.

## Observations for later plans (not fixed here)

- **OpenAI sends `max_tokens`, which `gpt-5.4-nano` rejects (it wants `max_completion_tokens`)** (50-06 finding, probe 11). Out of scope here: the profile has no field for the token-limit parameter name, and this plan's `max_tokens: Some(4096)` default keeps sending the old key. Anyone pinning `gpt-5.4-nano` through the OpenAI adapter still sees a 400.
- The retry counter conventions still differ (Anthropic and Gemini count total attempts; the compat presets, OpenAI and DeepSeek count retries after the first). Unifying them changes default latency and cost, so it wants its own change.
- `OpenAiCompatibleCapabilitiesConfig` (a public, `Deserialize`, constructible type) is not one of the nine and is unchanged.
- The `new(..)` signature drift is the owner's separate todo (`2026-10-16-harmonise-llm-config-new-signatures.md`); after it lands, `tests/consumer_surface_muster.rs` needs the new shapes.
- PARITY-03 is not marked complete: this plan delivers the direct-construction half (D-12); the `llm.<provider>` layered path, `create_from_config` and the YAML block that make the requirement whole land in plan 50-09.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or trust boundary was added; the credential surface only shrank (`Debug`).

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-09 can add `LlmProviderConfig.{max_tokens, context_window, models}` and layering on top of the nine option-a configs, calling `registry::is_reasoning_model` and the `with_*` builders; the bridge conversions currently set `max_tokens: None` (compat, DeepSeek, Gemini), `Some(4096)` (OpenAI), `4096` (Anthropic), `context_window: None` and empty overrides.
- Plan 50-10 measures the semver register for the nine types (`struct_marked_non_exhaustive`, `constructible_struct_adds_field`) and re-exports the six remaining configs from the facade.
- Plan 50-16 states the retry-counter conventions and the precedence (per-model override over adapter-wide `context_window` over seed over fallback) on the switching-providers page.
- Phase 51 can add `reasoning_effort` to the same types without another literal break.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/test_support.rs` (created), `registry/mod.rs`, the nine adapter files, `config/bridge.rs` all present; `#[non_exhaustive]` confirmed on all nine config types (`grep -B2 'pub struct <Config>'`).
- Commits: `bfbc93ff`, `6d842be5`, `70d6cb48` present in `git log`; `git diff --diff-filter=D --name-only HEAD~1 HEAD` empty (no deletions).
