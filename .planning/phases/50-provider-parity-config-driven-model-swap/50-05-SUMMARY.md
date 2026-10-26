---
phase: 50-provider-parity-config-driven-model-swap
plan: 05
subsystem: api
tags: [rust, paladin-llm, openai, anthropic, deepseek, gemini, sampling-filter, model-registry, adr-0061, parity]

requires:
  - phase: 50
    provides: "plan 50-03: sampling::apply, apply_response_format, check_max_tokens, OmissionLog, FormatLog, ModelRegistry and the three metadata keys; plan 50-02: ModelProfile and LlmPort::model_profile; plan 50-01: the nine-provider swap matrix"
provides:
  - "OpenAI, DeepSeek, Anthropic and Gemini build their request (buffered and streamed) through sampling::apply with the profile their ModelRegistry resolves, and return Some(profile) from model_profile"
  - "sampling::RequestNotes: one crate-private carrier for the three response-metadata keys, now used by the compat engine and all four bespoke adapters"
  - "stop sequences on every wire: OpenAI stop (max 4), DeepSeek stop, Anthropic stop_sequences, Gemini generationConfig.stopSequences, in declared order with duplicates kept"
  - "OpenAI no longer fabricates temperature 0.7 or top_p 1.0; Anthropic's validate-then-discard replaced by a NotSent profile (refused at build by name)"
  - "provider_swap_matrix: same_agent_is_equivalent_or_refused_by_name green for all nine providers"
  - "MIGRATION.md 9.1 row M-B-13 for the whole per-model policy (plans 50-03 and 50-05) and a root CHANGELOG Changed entry"
affects: [50-06, 50-08, 50-09, 50-12, 50-13, 50-15, 50-16, 50-18]

tech-stack:
  added: []
  patterns:
    - "Per-adapter fallback_profile() and declared_capabilities() pair: the profile is derived from the same declaration get_capabilities() returns, so the two cannot drift"
    - "build_request returns (wire body, RequestNotes); the buffered path stamps notes.into_metadata() on the successful response after the retry loop, the streaming path discards the notes after the warn! has fired"
    - "A private model_registry field swapped in-module by tests (ModelRegistry::with_overrides) until the config knob arrives in plans 50-08 and 50-09"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/sampling.rs
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/compat/engine.rs
    - tests/provider_swap_matrix.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "RequestNotes moved out of compat/engine.rs into sampling.rs so the compat engine and the four bespoke adapters stamp paladin.params_omitted, paladin.response_format_adjusted and paladin.unprofiled_model through one type"
  - "OpenAI keeps max_tokens 4096 when unset as a compiled default, and the effective value (default included) goes through check_max_tokens"
  - "Anthropic's fallback profile is temperature and top_p NotSent (what the adapter has always sent), so an explicit temperature is refused at build; per-model seeds in 50-06 may re-open a range"
  - "Anthropic records a requested response_format as paladin.response_format_adjusted = omitted (profile json_object and json_schema false)"
  - "Gemini's anthropic-style early return for a system-role text prompt is folded into the normal path (messages stay empty, system field set) so the sampling decision is computed in one place"

patterns-established:
  - "Existing build_request unit tests keep their assertions and read the wire struct through .0 of the new tuple"
  - "Each adapter gets a parameter_policy test module that captures the real outgoing JSON body through mockito, on both the buffered and streaming paths"

requirements-completed: [PARITY-02, PARITY-05, PARITY-01]

coverage:
  - id: D1
    description: "OpenAI, DeepSeek, Anthropic and Gemini send a sampling parameter only when the caller set it and the model's profile admits it; an unset value is absent from the body, an inadmissible one is omitted and recorded, nothing is clamped or substituted"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "openai_unset_sampling_values_are_absent; openai_out_of_range_temperature_is_omitted_and_recorded; deepseek_out_of_range_temperature_is_omitted_and_recorded; anthropic_temperature_is_omitted_and_recorded; gemini_out_of_range_temperature_is_omitted_and_recorded"
        status: pass
    human_judgment: false
  - id: D2
    description: "no adapter-local fabricated default remains: OpenAI's unwrap_or(0.7) and top_p Some(1.0) are gone, and max_tokens 4096 is the one compiled, documented default"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "openai_unset_sampling_values_are_absent (no temperature, top_p or stop key; max_tokens 4096); acceptance greps unwrap_or(0.7) = 0, top_p: Some(1.0) = 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "stop sequences reach all four wires in declared order with duplicates kept; OpenAI's list longer than 4 is omitted whole and recorded, never truncated; the streaming bodies follow the same rules"
    requirement: "PARITY-01"
    verification:
      - kind: unit
        ref: "openai_stop_sequences_keep_declared_order; openai_stop_sequences_past_the_maximum_are_omitted_not_truncated; openai_stream_follows_the_profile; deepseek_stop_sequences_keep_declared_order; deepseek_stream_stop_sequences_keep_declared_order; anthropic_stop_sequences_keep_declared_order; anthropic_stream_follows_the_profile; gemini_stop_sequences_keep_declared_order; gemini_stream_follows_the_profile"
        status: pass
    human_judgment: false
  - id: D4
    description: "response_format goes through apply_response_format and the effective max_tokens through check_max_tokens on every bespoke wire: OpenAI and Gemini send both formats unchanged, DeepSeek's schema-to-object downgrade and Anthropic's never-sent format are recorded, an over-limit max_tokens (OpenAI's 4096 default included) is refused by name before any request"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "openai_json_schema_still_reaches_the_wire_as_json_schema; deepseek_schema_request_is_recorded_as_downgraded; anthropic_response_format_is_recorded_as_omitted; gemini_json_schema_still_sets_mime_type_and_schema; openai_max_tokens_default_is_checked_against_the_profile; deepseek_/anthropic_/gemini_max_tokens_above_a_profiled_maximum_is_refused_by_name"
        status: pass
    human_judgment: false
  - id: D5
    description: "the same agent definition is equivalent or refused by name on all nine providers (Anthropic is refused at build naming provider, model and temperature; the other eight carry temperature 0.5 and [END, STOP] on the wire)"
    requirement: "PARITY-01"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all same_agent_is_equivalent_or_refused_by_name (9 passed, 0 ignored); full file 40 passed, 9 ignored for plans 50-12, 50-13, 50-14"
        status: pass
    human_judgment: false
  - id: D6
    description: "the credential path is untouched: redirect policy and key headers unchanged on all four adapters, conformance cases stay green"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib (746 passed, includes every conformance_suite case); diff review: no added or removed line touches redirect, Authorization, api_key, x-api-key, x-goog or .header( outside a test config literal"
        status: pass
    human_judgment: false

duration: ~20min
completed: 2026-10-15
status: complete
---

# Phase 50 Plan 05: The original adapters follow the per-model parameter policy Summary

**OpenAI, DeepSeek, Anthropic and Gemini now build every request through the shared ADR-0061 filter and report a model profile: OpenAI stops inventing temperature and top_p, Anthropic stops validating a temperature it then discards, stop sequences reach all four wires, and the nine-provider "same agent is equivalent or refused by name" row is green.**

## Performance

- **Duration:** about 20 min (start 2026-10-15T17:55:39Z, last task commit 18:14:09Z)
- **Completed:** 2026-10-15T18:14:09Z (last task commit)
- **Tasks:** 3 of 3
- **Files modified:** 10 (0 created)

## Accomplishments

- Each of the four adapters gained a private `ModelRegistry`, `OmissionLog` and `FormatLog`, a `fallback_profile()` derived from the same `declared_capabilities()` its `get_capabilities()` returns, and a `build_request` that returns the wire body plus `RequestNotes`. `generate` stamps the notes on the buffered response after the retry loop; `generate_stream` shares the same builder, so a streamed call follows the same filter and warns the same way.
- **OpenAI:** `temperature` and `top_p` are absent unless the caller set them and the profile admits them (the fabricated `0.7` and `1.0` are gone). `stop` is sent in order (`Supported { max: Some(4) }`; five sequences are omitted whole and recorded). `max_tokens` keeps the compiled 4096 default, and that effective value goes through `check_max_tokens`.
- **DeepSeek:** fallback sends all four numeric fields (`Range`) plus `stop`; a JSON-schema request is still sent as a JSON object and is now recorded as `paladin.response_format_adjusted = json_object`.
- **Anthropic:** the adapter-wide profile states `temperature` and `top_p` `NotSent`, so an agent's explicit temperature is refused at build by name and a direct call's value is omitted and recorded instead of dropped silently. `stop_sequences` reaches the Messages body in order. A requested `response_format` (never sent) is recorded as `omitted`.
- **Gemini:** temperature, `topP` and `stopSequences` go through the filter; penalties never appear; `responseMimeType` and `responseSchema` are unchanged for a profile that admits them.
- `same_agent_is_equivalent_or_refused_by_name` is un-ignored for openai, anthropic and deepseek with assertions untouched: 9 of 9 pass.
- MIGRATION.md 9.1 gains row M-B-13 for the whole policy (plans 50-03 and 50-05), folding in 50-03's list including the `openai-compatible` default `(0.0, 1.0)` temperature range, and the root CHANGELOG gains a Changed entry.

## Task Commits

1. **Task 1: OpenAI and DeepSeek on the shared filter** - `c1318830` (feat)
2. **Task 2: Anthropic and Gemini on the shared filter** - `2a2eea9e` (feat)
3. **Task 3: bespoke matrix rows green, M-B-13 and CHANGELOG** - `0faabc22` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md (below the task commits).

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 746 passed (up from 710 after 50-03), including every adapter's `conformance_suite`.
- `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored; the 9 are tagged for plans 50-12, 50-13 and 50-14, none for 50-05. `... same_agent_is_equivalent_or_refused_by_name`: 9 passed, 0 ignored.
- `cargo test --lib` (root crate): 1296 passed, 0 failed. `cargo test --test unit llm`: 55 passed. The two `run_api_wiring` failures (WINDOWS.md row 73) did not appear in this run.
- `cargo fmt --check` clean. `cargo clippy -p paladin-llm --all-features --lib --tests -- -D warnings` clean, and `--lib` clean for `--no-default-features` and for `--features openai | anthropic | deepseek | gemini | kimi` separately; `cargo clippy --test provider_swap_matrix --features llm-all -- -D warnings` clean.
- `make api-surface`: unchanged (4274 items). `make check-gates` and `make check-consumer-surface`: pass. `make clean-code` stops at `lint-shell` because `shellcheck` is not installed (environment note); fmt and clippy were run directly instead.
- Acceptance greps: `sampling::apply` x4 in each of openai, deepseek, anthropic, gemini (buffered and streaming, plus doc references); `apply_response_format` x2 and `check_max_tokens` x1 in openai and deepseek; `fn model_profile` x1 in all four; `unwrap_or(0.7)` and `top_p: Some(1.0)` both 0 in openai; zero `red until plan 50-05` left in the matrix.
- Manual credential-handling review (D-00a, T-50-05-03): no added or removed line touches a redirect policy, `Authorization`, `x-api-key`, `x-goog-api-key`, `.header(` or an API key outside a test config literal. Every new `warn!` (from `OmissionLog` and `FormatLog`) names provider, model, parameter and a number or a count, never a key, URL or body.

## Decisions Made

See `key-decisions` above. The two worth a future reader's attention: `RequestNotes` is now the one carrier for the three metadata keys (the compat engine's private copy is gone), and Anthropic's `NotSent` temperature is the conservative reading of RESEARCH A7 (flagged A1-50-05) that plan 50-06 may refine per model from the live probe.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RequestNotes` shared through `sampling.rs`; `registry` and `sampling` dead-code gates widened**
- **Found during:** Task 1
- **Issue:** the plan says the four adapters stamp the metadata keys "exactly as the compat engine does", but the engine's `RequestNotes` was private to `compat/engine.rs`, which is only compiled with a compat feature. Copying it four times would have been the duplication 50-03 warned against, and the `#![cfg_attr(not(any(<compat features>)), allow(dead_code))]` gates on `sampling.rs` and `registry/mod.rs` would have warned (and failed `-D warnings`) under `--features openai` and the other bespoke features.
- **Fix:** moved `RequestNotes` (with `pub(crate)` fields and `into_metadata`) into `sampling.rs`; the engine imports it and its tests import the key constants they use; the two `dead_code` gates now also name `openai`, `anthropic`, `deepseek` and `gemini`. `sampling.rs`, `registry/mod.rs` and `compat/engine.rs` are not in the plan's `files_modified`. No behavior change; the 50-03 engine tests pass unchanged.
- **Commit:** `c1318830`

**2. [Rule 1 - Bug] Anthropic's system-role text-prompt branch returned early with its own hard-coded `None` temperature**
- **Found during:** Task 2
- **Issue:** `build_request` had two places that built a `ClaudeRequest` with a hard-coded `temperature: None`; routing only the second through the filter would have left the first path outside the policy.
- **Fix:** the system-role arm now sets the `system` field and falls through (no message pushed), so one construction site applies the profile. The resulting body is identical (empty `messages`, `system` set); `anthropic_system_role_text_prompt_still_becomes_the_system_field` pins it.
- **Commit:** `2a2eea9e`

**3. [Plan step not needed] `make api-surface-update` was not run**
- `make api-surface` reports the surface unchanged and `.project/current-exports.txt` is untouched: the new adapter fields are private and `RequestNotes` is `pub(crate)`. The CHANGELOG entry records this.

### Notes, not deviations

- Existing `build_request` unit tests in deepseek, anthropic and gemini now read the wire struct through `.0` of the returned tuple; none of their assertions changed.
- The penalties note in M-B-13: a prompt built with `PromptParameters::default()` carries `0.0` frequency and presence penalties, which OpenAI, Anthropic and Gemini have never put on a wire, so their buffered responses now list `frequency_penalty,presence_penalty` in `paladin.params_omitted` (with one `warn!` per model). Agents run through `PaladinExecutionService` leave the penalties unset and record nothing. The same default `max_tokens` of 1000 that 50-03 flagged applies only where a prompt carries it; OpenAI via the service still sends the 4096 default.
- OpenAI reasoning models (flagged A2-50-05) still receive `max_tokens` and `stop`; with no OpenAI key on the operator machine they stay on the adapter-wide fallback, and a vendor 400 surfaces as today's typed error.

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. The only surface touched is request-body construction; T-50-05-01 (silent drop) is mitigated by `OmissionLog` plus `anthropic_temperature_is_omitted_and_recorded`, T-50-05-02 (silent truncation) by `openai_stop_sequences_past_the_maximum_are_omitted_not_truncated`, and T-50-05-03 (credential regression) by the diff review above and the unchanged conformance cases.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-06 can fill `ModelRegistry::new(<PROVIDER>, &[], fallback)` for all nine providers; the four adapters here already pass an empty seeded slice and resolve through `profile()`.
- Plans 50-08 and 50-09 feed `ModelRegistry::with_overrides` from configuration; the adapters' private `model_registry` field is where it lands, and OpenAI's `OPENAI_DEFAULT_MAX_TOKENS` becomes the `OpenAIConfig::max_tokens` knob.
- Plans 50-12 and 50-13 own the nine-provider empty-stop-completion rows still ignored in the matrix.

## Self-Check: PASSED

- Files: all ten modified files are in the three task commits (`git status` clean after Task 3); no file was created.
- Commits: `c1318830`, `2a2eea9e`, `0faabc22` present in `git log`.
