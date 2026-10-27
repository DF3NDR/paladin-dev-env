---
phase: 50-provider-parity-config-driven-model-swap
plan: 03
subsystem: api
tags: [rust, paladin-llm, paladin-builder, sampling-filter, model-registry, compat-engine, kimi, adr-0061, parity]

requires:
  - phase: 50
    provides: "plan 50-01: the nine-provider swap matrix with its ignored red rows; plan 50-02: ADR-0061, ModelProfile/ParamSupport/StopSupport/SamplingParam and LlmPort::model_profile (defaulted None, forwarded by the decorators)"
provides:
  - "paladin_llm::sampling: apply (the one wire-time filter), Applied, OmissionLog, apply_response_format, FormatLog, check_max_tokens, and the public keys paladin.params_omitted and paladin.response_format_adjusted"
  - "paladin_llm::registry: ModelOverride (public, deny_unknown_fields, validate, apply_to), ModelRegistry (override over seed over adapter-wide fallback, exact match), SeededProfile, ProfileOrigin, fallback_from_compat, and the public key paladin.unprofiled_model"
  - "CompatEngine::with_model_registry and CompatEngine::model_profile; stop sequences on the compat wire; model_profile on Kimi, Qwen, Grok, Ollama and OpenAI-compatible"
  - "PaladinBuilder profile-first policy: explicit temperature/stop refused by name (provider, model, parameter, reason); auto temperature warned and omitted at the wire, never refused"
  - "seven provider-swap matrix rows un-ignored and green (Kimi explicit and auto, five same-agent rows)"
affects: [50-05, 50-06, 50-07, 50-08, 50-09, 50-14, 50-15, 50-16, 50-18, 51]

tech-stack:
  added: []
  patterns:
    - "Refuse at build, filter at the wire: one admission rule (ParamSupport::admits) serves PaladinBuilder and sampling::apply, so a value is never judged twice and never swapped"
    - "Once-per-instance warn logs keyed by (model, parameter) or (model, adjustment), with the metadata key stamped on every buffered response"
    - "Adapter-wide fallback profile derived from the preset's own CompatCapabilities and CompatRequestParameters (fallback_from_compat), with seeded per-model entries to follow in 50-06"
    - "Private registry/logs on CompatEngine rather than new fields on the public CompatEngineConfig"

key-files:
  created:
    - crates/paladin-llm/src/sampling.rs
    - crates/paladin-llm/src/registry/mod.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/Cargo.toml
    - Cargo.lock
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/compat/mod.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - src/application/services/paladin/paladin_builder.rs
    - tests/provider_swap_matrix.rs
    - CHANGELOG.md

key-decisions:
  - "ModelOverride is tested through the `config` crate (the loader application settings go through), not serde_yaml: serde_yaml 0.9 reads an externally tagged enum from a YAML tag (`!fixed 0.95`), so the plan's `{fixed: 0.95}` form only works through the production loader"
  - "fallback_from_compat gives a preset that declares no temperature_range ADR-0004's default (0.0, 1.0), so the profile and the builder's range check agree for such a port"
  - "Kimi's request_parameters temperature and top_p flip from false to true: the preset's request path now carries them when the profile admits them, and its Fixed(1.0) / Fixed(0.95) profile is what restricts them"
  - "An effective max_tokens above a profile's max_output_tokens is refused by name (InvalidPrompt) before any request, never clamped; the engine checks the value after its own request_parameters.max_tokens gate"
  - "No registry attached means no unprofiled stamp: an engine built outside a preset derives its profile from its own config with stop NotSent"

patterns-established:
  - "A shared #[cfg(test)] compat::test_support (mock_completion, server) so each preset's wire tests capture the real outgoing JSON body"
  - "Metadata assertions added to existing tests that pinned a silent omission (Kimi temperature and top_p, engine tests 1, 4 and 6)"

requirements-completed: [PARITY-02, PARITY-01, PARITY-04, PARITY-05]

coverage:
  - id: D1
    description: "sampling::apply sends a set parameter only when the profile admits it and names every omission in fixed order; an unset value is never omitted, defaulted or named; stop sequences keep order and duplicates"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib sampling:: (21 tests: apply_*, omission_log_*)"
        status: pass
    human_judgment: false
  - id: D2
    description: "every omission warns once per adapter instance per (model, parameter) and stamps paladin.params_omitted on buffered responses; absent when nothing was omitted"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "omission_log_warns_once_per_model_and_param; compat_omission_is_recorded_in_metadata; compat_nothing_omitted_leaves_the_metadata_key_absent"
        status: pass
    human_judgment: false
  - id: D3
    description: "ModelRegistry resolves override over seed over adapter-wide fallback by exact model id; an unknown model (including the empty string) runs on the fallback, visibly, and is never refused; ModelOverride rejects unknown keys, u32 overflow and invalid ranges"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --all-features --lib registry:: (17 tests: resolve_*, override_wins_field_by_field, model_override_*); compat_unprofiled_model_is_recorded"
        status: pass
    human_judgment: false
  - id: D4
    description: "response_format follows the profile (schema downgraded to object and recorded, object omitted when unsupported) and max_tokens above the profiled maximum is refused by name before anything is sent"
    requirement: "PARITY-05"
    verification:
      - kind: unit
        ref: "apply_response_format_*; check_max_tokens_*; compat_response_format_follows_the_profile; compat_max_tokens_above_the_profile_is_refused"
        status: pass
    human_judgment: false
  - id: D5
    description: "the five compat presets declare an adapter-wide profile (Kimi Fixed 1.0 and 0.95, Grok stop NotSent) and put stop sequences on the wire in the order declared"
    requirement: "PARITY-01"
    verification:
      - kind: unit
        ref: "kimi_fixed_values_follow_the_profile; grok_stop_is_omitted_and_recorded; compat_stop_reaches_the_wire_in_order; qwen_/ollama_/openai_compatible_stop_reaches_the_wire_in_order"
        status: pass
    human_judgment: false
  - id: D6
    description: "PaladinBuilder asks the profile first: explicit temperature or stop words it does not admit fail build() naming provider, model and parameter; an inadmissible auto temperature builds with a warning and is omitted at the wire"
    requirement: "PARITY-02"
    verification:
      - kind: unit
        ref: "cargo test --lib paladin_builder (42 tests incl. auto_temperature_outside_profile_is_omitted_not_refused, fixed_profile_accepts_exactly_its_value_and_refuses_others_by_name)"
        status: pass
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all (37 passed, 12 ignored for plans 50-05, 50-12, 50-13, 50-14)"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-10-15
status: complete
---

# Phase 50 Plan 03: Per-model parameter policy on the compat path and in PaladinBuilder Summary

**ADR-0061 made real on five providers: one wire-time filter and one per-model registry core in paladin-llm, Kimi/Qwen/Grok/Ollama/OpenAI-compatible each declaring an honest profile with stop sequences on the wire, and PaladinBuilder refusing explicit values by name while omitting auto ones visibly, so a Kimi swap with the default `auto_temperature` now builds.**

## Performance

- **Duration:** about 45 min (start time not captured at launch; estimated from the first build and the commit timestamps)
- **Completed:** 2026-10-15T17:47:11Z (last task commit)
- **Tasks:** 3 of 3
- **Files modified:** 16 (2 created)

## Accomplishments

- `sampling::apply` is the single place that decides what reaches a wire: a set parameter is sent only when `ParamSupport::admits` / `StopSupport::admits` accept it, every other set value is omitted and named, nothing is clamped, defaulted or swapped. The omission is recorded twice: one `warn!` per instance per (model, parameter) and `paladin.params_omitted` on every buffered response from a request that omitted something.
- `ModelRegistry` resolves a model by exact id to an override laid field by field over a seeded entry, else the seeded entry, else the adapter-wide fallback. An unprofiled model logs once and stamps `paladin.unprofiled_model`; it is never refused. `ModelOverride` rejects unknown keys, `u32` overflow (naming the field), zero or inverted limits, non-finite bounds and a zero stop maximum, and the merged result is re-validated.
- The compat engine now filters through the request model's profile, sends `stop` in declared order, records a schema-to-object downgrade as `paladin.response_format_adjusted`, and refuses an effective `max_tokens` above the profile's maximum output before any HTTP call.
- Kimi now sends an explicit temperature `1.0` and `top_p` `0.95` (it dropped both unconditionally); any other value is omitted and recorded, never swapped. Grok declares stop `NotSent`; the other four declare `Supported { max: None }`.
- `PaladinBuilder` consults `model_profile` first. `.temperature(0.5)` on a `Fixed(1.0)` model fails `build()` with `model 'kimi-k3' on provider 'kimi' does not accept parameter 'temperature' = 0.5: it takes only 1`; the next representable `f32` above `1.0` is refused too. ADR-0004 A2 is superseded: an auto temperature the profile does not admit warns and is left for the wire filter to omit.
- Seven matrix rows tagged `red until plan 50-03` are un-ignored with their assertions untouched and pass (`kimi_fixed_temperature_explicit`, `kimi_fixed_temperature_auto`, and `same_agent_is_equivalent_or_refused_by_name` for kimi, qwen, grok, ollama, openai-compatible).

## Task Commits

1. **Task 1: shared sampling filter and per-model registry core** - `255026eb` (feat)
2. **Task 2: compat engine and five presets on the filter and registry, stop on the wire** - `03af66f9` (feat)
3. **Task 3: profile-first PaladinBuilder and the compat matrix rows green** - `87a48a42` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md and ROADMAP.md (below the task commits).

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 710 passed (21 `sampling::`, 17 `registry::`, the seven plan-named engine and preset tests).
- `cargo test --lib`: 1296 passed, 0 failed (includes 42 `paladin_builder` tests).
- `cargo test --test provider_swap_matrix --features llm-all`: 37 passed, 12 ignored; the 12 ignored are tagged for plans 50-05, 50-12, 50-13 and 50-14, none for 50-03.
- `cargo clippy -p paladin-llm --all-features --lib --tests -- -D warnings`, the same for `--lib` under default features, `--no-default-features` and `--features kimi`, and `cargo clippy --lib --tests -- -D warnings` on the root crate: clean. `cargo fmt --check`: clean.
- `make api-surface`: unchanged (4274 items; `.project/current-exports.txt` tracks only the root crate and no root public signature changed). `make check-consumer-surface`, `make check-gates` and `make check-api-examples`: pass.
- Manual credential-handling review (D-00a): no credential path touched; every `warn!` names provider, model, parameter and a number or a count (stop sequences are logged as a count, never their text); no key, URL or body is interpolated.
- Acceptance greps: every Task 1-3 grep from the plan holds (`sampling::apply` x3, `with_model_registry` x3, `fn model_profile` in all five presets, `Fixed(1.0)` x3, `Fixed(0.95)` x2, zero `red until plan 50-03` left, zero public fields added to `CompatEngineConfig`).

## For plan 50-05's MIGRATION.md 9.1 row (one row for the whole policy)

- Explicit temperature or stop words a profile does not admit now fail the build, naming provider, model and parameter.
- `auto_temperature` is no longer refused on a narrow provider (it warns and is omitted at the wire).
- Kimi now sends an explicit `1.0` (and `0.95` `top_p`); the framework's `0.7` default and any other value are omitted and recorded.
- The compat presets now send stop sequences, in declared order, except Grok, which does not take them (a stop-word agent is refused at build, a direct call's `stop` is omitted and recorded).
- A JSON-schema request on a compat preset is still sent as `json_object` and is now recorded as `paladin.response_format_adjusted`.
- A `max_tokens` above a model's profiled maximum output is refused by name; nothing is clamped.
- An `openai-compatible` endpoint that declares no temperature range is now held to ADR-0004's default `(0.0, 1.0)` at the wire as well as at build: a direct call with a temperature above `1.0` is omitted and recorded where it used to be sent (set `OPENAI_COMPATIBLE_TEMPERATURE_MIN` / `_MAX` to declare the real range).
- Streamed calls warn only (no metadata channel) until owner reading R-1 is decided (A1-50-03).

## Decisions Made

See `key-decisions` above. The two worth a future reader's attention: the override shape is pinned through the real `config` loader (not `serde_yaml`), and a profile-less compat range is ADR-0004's `(0.0, 1.0)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `config` as a dev-dependency instead of `serde_yaml`**
- **Found during:** Task 1 (registry tests)
- **Issue:** the plan's `temperature: {fixed: 0.95}` is rejected by `serde_yaml` 0.9 (it expects an enum as a YAML tag, `!fixed 0.95`), which is not the loader application settings use. A `serde_yaml` test would have pinned a form no deployment writes.
- **Fix:** `crates/paladin-llm/Cargo.toml` gains `config = "0.15.11"` under `[dev-dependencies]` (same pin as the root crate, already in `Cargo.lock`, so no new package; `Cargo.lock` gains one dependency line). The override tests parse YAML through `config::Config::builder()`, the production path; the `deny_unknown_fields` and `u32`-overflow errors both name their field. `Cargo.toml` and `Cargo.lock` are not in the plan's `files_modified`.
- **Commit:** `255026eb`

**2. [Rule 1 - Bug] Kimi's `request_parameters` declared `temperature` and `top_p` not carried**
- **Found during:** Task 2
- **Issue:** with `temperature: false` and `top_p: false` the fallback derivation would mark both `NotSent` before Kimi's `Fixed` override, and the old comment ("the key is omitted entirely") contradicted the new behaviour, where an explicit `1.0` / `0.95` is sent.
- **Fix:** both flags are now `true`, the comment block is rewritten as history plus the ADR-0061 rationale, and the two existing Kimi tests that pinned silent omission (`generate_omits_temperature_...`, `generate_omits_top_p_...`) now also assert the `paladin.params_omitted` entry. Engine tests 1, 4 and 6 gained the same assertion through a `generate_and_capture_all` helper.
- **Commit:** `03af66f9`

**3. [Rule 3 - Blocking] Stale cached `paladin-ports` artifact and low disk**
- **Found during:** Task 3 (`cargo test --lib paladin_builder`)
- **Issue:** one cached `paladin-ports` build for the root test profile was marked Fresh but lacked `output::model_profile` (E0432 in `paladin-llm`). Disk also dropped to 1.6 GB free.
- **Fix:** `touch crates/paladin-ports/src/lib.rs` forced the rebuild (no content change); 2,853 older duplicate artifacts under `target/debug/deps` dated before 2026-10-15 (3.7 GB) were deleted per the environment note. No `cargo clean`.

**4. [Plan step not needed] `make api-surface-update` was not run**
- `make api-surface` reports the surface unchanged and `.project/current-exports.txt` is untouched, so there was nothing to refresh (the same outcome 50-02 recorded). The CHANGELOG `### Added` and `### Changed` bullets are written.

### Notes, not deviations

- `ModelRegistry::with_overrides` and `validate_profile` carry a targeted `#[allow(dead_code)]` (config overrides first arrive through the bridge in plans 50-08 and 50-09), and each new module has a feature-gated `dead_code` allow that applies only when none of the five compat features is enabled (plan 50-05 widens its consumers).
- The builder's "logs a warning" for an inadmissible auto temperature is not asserted by a log-capture test: `log` allows one process-wide logger and the crate's other tests already install one. The build-succeeds and value-kept behaviour is asserted; the `warn!` text is covered by review.
- The framework's default `PromptParameters.max_tokens` is `Some(1000)`. With `check_max_tokens` a seeded or overridden `max_output_tokens` below 1000 would refuse the fabricated default. No such value exists yet; plans 50-06 and 50-08 (which route a configured default through the same check) should keep every seeded `max_output_tokens` at or above it, or make the default explicit.

## Issues Encountered

None beyond the deviations above. The two `run_api_wiring` tests recorded in WINDOWS.md row 73 did not appear in this plan's `cargo test --lib` run (1296 passed).

## Known Stubs

None.

## Threat Flags

None. The only new trust surface is the `ModelOverride` input, which is the plan's own threat-model item T-50-03-01 (validated, `deny_unknown_fields`, merged result re-validated). T-50-03-02 (silent drop) is mitigated by the `debug!` -> `warn!` change plus the metadata key and tests; T-50-03-03 (log flooding) by the once-per-instance logs; T-50-03-04 (secrets in a warn) by the number-or-count-only text; T-50-03-05 (wrong limit refusing requests) by the by-name refusal text that points at a `models.<id>` override.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-05 can put OpenAI, Anthropic, DeepSeek and Gemini on `sampling::apply` / `apply_response_format` / `check_max_tokens` and their own `model_profile`, then write the MIGRATION.md 9.1 row from the list above.
- Plan 50-06 fills `ModelRegistry::new(<PROVIDER>, &[], fallback)`'s empty seeded slice (`SeededProfile` and `ProfileOrigin` already carry `verified_on`); plans 50-08 and 50-09 feed `ModelRegistry::with_overrides` from configuration.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/sampling.rs` and `crates/paladin-llm/src/registry/mod.rs` exist; all modified files are in the three task commits.
- Commits: `255026eb`, `03af66f9`, `87a48a42` present in `git log`.
