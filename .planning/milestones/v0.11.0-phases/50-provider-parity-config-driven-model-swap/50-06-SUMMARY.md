---
phase: 50-provider-parity-config-driven-model-swap
plan: 06
subsystem: api
tags: [rust, paladin-llm, model-registry, seeded-tables, adr-0061, parity, defaults]

requires:
  - phase: 50
    provides: "plan 50-03: SeededProfile, ModelRegistry, ProfileOrigin; plan 50-05: model_profile on the four bespoke adapters; plan 50-17: 50-SEED-PROBE.md, the operator's dated live transcript (2026-10-15)"
provides:
  - "crates/paladin-llm/src/registry/seeded.rs: dated per-vendor tables ANTHROPIC, OPENAI, DEEPSEEK, KIMI, QWEN, GROK, GEMINI, OLLAMA, every value traced to 50-SEED-PROBE.md and stamped verified_on 2026-10-15"
  - "every adapter except OpenAI-compatible resolves its models through its own table (ModelRegistry::new(<provider>, seeded::<VENDOR>, fallback))"
  - "pub const ANTHROPIC_DEFAULT_MODEL (claude-haiku-5-5) and pub const DEEPSEEK_DEFAULT_MODEL (deepseek-flash); both from_env defaults use them"
  - "default-model constants moved to the operator-identified current model for the six probed vendors that have one (kimi-k2.6, qwen3.8-flash, grok-4.3, gemini-3.5-flash-lite as well)"
  - "get_capabilities().max_context_tokens read from the default model's seeded window through registry::seeded_window (DeepSeek's 64,000 and Kimi's 131,072 replaced)"
  - "Anthropic get_available_models and validate_model, and DeepSeek get_available_models, served from the tables; no claude-3 or claude-2 list remains in non-test code"
  - "MIGRATION.md 9.1 row M-B-14 and root CHANGELOG Added and Changed entries"
affects: [50-07, 50-08, 50-09, 50-10, 50-13, 50-15, 50-16]

tech-stack:
  added: []
  patterns:
    - "One inline cfg(feature)-gated module per vendor table in seeded.rs, re-exported as seeded::<VENDOR>, so a single-feature build carries no unused table"
    - "An adapter's declared window is read from its default model's seeded entry (seeded_window), so the capability declaration and the dated table cannot drift"
    - "A seeded entry replaces the adapter-wide fallback for its model (it is built from ModelProfile::new()), so a model is listed only when it was measured end to end"

key-files:
  created:
    - crates/paladin-llm/src/registry/seeded.rs
  modified:
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - tests/provider_swap_matrix.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Seed only models the probe measured end to end: a seeded entry replaces the fallback, so an unmeasured model would turn every unmeasured field into a refusal. Not seeded: the other 13 Anthropic ids, deepseek-v4-pro, deepseek-chat and deepseek-coder (undocumented aliases), kimi-k2.7-code and kimi-k2.7-code-highspeed, qwen-turbo (429 on every probe), qwen3-max (404)"
  - "Kimi temperature, top_p and both penalties are NotSent on kimi-k2.6 and kimi-k3 (every probed value rejected). The vendor's 'only 0.6 is allowed' is stated for the thinking-disabled probes and an earlier measurement named 1, so no value is pinned"
  - "Grok temperature is Range(0.0, 1.99) although the 2.00 probe was UNREACHABLE: the bound comes from the vendor's own observed 'must be less than 2' rejection text (judgment call, see Deviations)"
  - "OpenAI temperature is Range(0.0, 1.0): 1.01 was also accepted, so the upper bound is not located; the highest accepted probed endpoint is used"
  - "top_p accepted at 0.5 is seeded Range(0.0, 1.0) and penalties accepted at 0.5 Range(-2.0, 2.0), both marked contract-derived (A3 applied to top_p as well)"
  - "OpenAI, Anthropic and Gemini penalties are NotSent regardless of the probe because their request types carry no penalty field; admitting a value the wire cannot send would drop it silently"
  - "reasoning is true only on reasoning-text evidence or a listing flag (OpenAI's reasoning_tokens usage key is present on every model, so gpt-5.4-nano and gpt-4o-mini are false)"
  - "validate_model and get_available_models for Anthropic and DeepSeek serve the table, so a current model that was not probed reports false from validate_model yet still runs on the fallback profile"

patterns-established:
  - "Fallback-path adapter tests use an unseeded id (UNSEEDED_MODEL in the Kimi tests) so they keep describing the adapter-wide fallback now that the default model has its own entry"

requirements-completed: [PARITY-04]

coverage:
  - id: D1
    description: "seeded.rs holds eight dated static tables, every model id and every verified_on date occurs in 50-SEED-PROBE.md, and the tables are proven unique, dated, non-zero and window-consistent"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "seeded_tables_are_dated_unique_and_consistent; every_entry_carries_the_probe_date; unmeasured_and_retired_ids_are_not_seeded; seeded_lookup_is_exact_match_only; acceptance greps: pub(crate) static = 8, ids and dates all found in 50-SEED-PROBE.md"
        status: pass
    human_judgment: false
  - id: D2
    description: "each probed vendor's seeded entries match the transcript rows they cite"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "openai_gpt_5_4_nano_seed_matches_the_transcript; openai_gpt_4o_mini_seed_matches_the_transcript; anthropic_haiku_5_5_seed_matches_the_transcript; deepseek_flash_seed_matches_the_transcript; kimi_k3_seed_matches_the_transcript; kimi_k2_6_seed_matches_the_transcript; qwen_3_8_flash_seed_matches_the_transcript; qwen_plus_seed_matches_the_transcript; grok_seeds_match_the_transcript; gemini_seeds_match_the_transcript"
        status: pass
    human_judgment: false
  - id: D3
    description: "every adapter except OpenAI-compatible resolves through its own table, and each probed vendor's default model resolves Seeded with the transcript date"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "anthropic_default_model_is_seeded; deepseek_default_model_is_seeded; gemini_default_model_is_seeded; kimi_default_model_is_seeded; qwen_default_model_is_seeded; grok_default_model_is_seeded; openai_default_model_is_seeded (gpt-5.4-nano, no adapter default exists); ollama_models_run_on_the_adapter_wide_fallback_because_the_probe_skipped_it; grep -c 'seeded::' is 2, 7, 5, 4, 3, 3, 3, 1 across the eight adapters"
        status: pass
    human_judgment: false
  - id: D4
    description: "SC-4 stale defaults: the Anthropic claude-3 and claude-2 lists, the claude-3-5-sonnet-20241022 default and doc example, DeepSeek's deepseek-chat default and 64,000 window and Kimi's 131,072 window are gone from non-test code; the two new constants exist"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "anthropic_available_models_are_the_seeded_table_ids_in_order; anthropic_validate_model_accepts_a_seeded_id_and_rejects_a_claude_3_id; deepseek_available_models_come_from_the_seeded_table_and_include_the_default; *_capabilities_report_the_default_models_seeded_window; acceptance awk greps for claude-3 (0) and 64000/64_000 (0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "the dated registry does not regress the rest of the workspace and the nine-provider matrix"
    requirement: "PARITY-04"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-llm --all-features --lib (783 passed); cargo test --test provider_swap_matrix --features llm-all (40 passed, 9 ignored for later plans); cargo test --workspace --lib --bins --no-fail-fast (14 binaries, 4824 passed, 2 failed: the two ledgered run_api_wiring failures, WINDOWS.md row 73)"
        status: pass
    human_judgment: false

duration: ~1h30m
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 06: Seed the dated registry and replace the stale defaults Summary

**Eight dated per-vendor tables seeded only from the operator's 2026-10-15 live probe now back every adapter's model profile, and the six probed vendors that have a default model move to the operator-identified current one, with Anthropic's claude-3 list and DeepSeek's 64,000 window gone.**

## Performance

- **Duration:** about 1h30m (the start time was not captured; first task commit `2118ccd7`, last task commit `579d9772`)
- **Completed:** 2026-10-16
- **Tasks:** 3 of 3
- **Files modified:** 13 (1 created)

## Accomplishments

- `registry/seeded.rs`: one `cfg(feature)`-gated module per vendor, each exposing `seeded::<VENDOR>`. 15 models are listed: Anthropic 1, OpenAI 2, DeepSeek 1, Kimi 2, Qwen 2, Grok 3, Gemini 3, Ollama 0 (the probe skipped it). Every entry carries `verified_on: "2026-10-15"`, and a value taken from the operator's vendor-doc evidence carries its URL and access date in a comment.
- Adapters: `ModelRegistry::new(.., seeded::<VENDOR>, ..)` in seven adapters plus the empty Ollama table; OpenAI-compatible keeps `&[]` by design.
- Defaults, old to new: Anthropic `claude-3-5-sonnet-20241022` to `claude-haiku-5-5`, DeepSeek `deepseek-chat` to `deepseek-flash`, Kimi `kimi-k3` to `kimi-k2.6`, Qwen `qwen-plus` to `qwen3.8-flash`, Grok `grok-4.6` to `grok-4.3`, Gemini `gemini-3.6-flash` to `gemini-3.5-flash-lite`. Each `*_FALLBACK_MODELS` list now leads with its default; the previous default stays in the list and in the table.
- Windows now read from the default model's entry: Anthropic 200,000 to 1,000,000; DeepSeek 64,000 to 1,048,576; Kimi 131,072 to 262,144; Qwen 131,072 to 1,000,000; Grok 131,072 to 1,000,000; Gemini unchanged at 1,048,576.
- `AnthropicAdapter::validate_model` and `get_available_models` serve the Anthropic table; `DeepSeekAdapter::get_available_models` serves `deepseek-flash`.
- `ANTHROPIC_DEFAULT_MODEL` and `DEEPSEEK_DEFAULT_MODEL` are new public constants used by the two `from_env` defaults.
- MIGRATION.md row M-B-14, root CHANGELOG Added and Changed bullets.

## Task Commits

1. **Task 1: dated per-vendor tables from 50-SEED-PROBE.md** - `2118ccd7` (feat)
2. **Task 2: every adapter on its table, stale defaults and model lists replaced** - `52ac8f09` (feat)
3. **Task 3: M-B-14, CHANGELOG** - `579d9772` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 783 passed, including every adapter's `conformance_suite` and the new registry and per-adapter seeded tests.
- `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored (plans 50-12, 50-13, 50-14).
- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, 4824 passed, 2 failed. The two failures are exactly the known baseline pair (`build_run_api_persists_no_run_traces_by_default`, `..._persists_run_traces_when_trace_persist_is_set`); the orchestrator's baseline was 4806 passed, so the delta is this plan's new tests.
- `cargo fmt --check` clean. `cargo clippy -p paladin-llm --all-features --all-targets -- -D warnings` clean; `--lib` clean for `--no-default-features` and for each of `openai`, `anthropic`, `deepseek`, `gemini`, `kimi`, `qwen`, `grok`, `ollama`, `openai-compatible` alone. `make clean-code` passes fmt, clippy and check and stops at `lint-shell` because `shellcheck` is not installed (environment note, same as plan 50-05).
- `make api-surface`: unchanged (4274 items). `make check-gates`, `make check-consumer-surface`: pass.
- Acceptance greps: `pub(crate) static` count 8; every `verified_on` and every `model:` id in `seeded.rs` found in `50-SEED-PROBE.md`; `grep -c 'seeded::'` at least 1 in all eight adapter files; `pub const ANTHROPIC_DEFAULT_MODEL` and `pub const DEEPSEEK_DEFAULT_MODEL` once each; `claude-3` count 0 and `64000|64_000` count 0 before the first `#[cfg(test)]`; `sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md | grep -c 'Phase 50 plan 50-06'` prints 1.
- Manual credential-handling review (D-00a, T-50-06-01..03): the diff adds only model ids, dates, windows and sampling facts. No line touches a redirect policy, a credential header, an API key or a response body. Every window and bound traces to the transcript, so T-50-06-01 (a window seeded too large) is bounded by evidence, with one caveat recorded below (Qwen's window).

## Decisions Made

See `key-decisions` above. The ones worth a reviewer's attention are the Grok and OpenAI temperature bounds (two places where the transcript could not place the upper bound by measurement) and the Kimi `NotSent` entries.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Grok temperature seeded `Range(0.0, 1.99)`, not left unseeded, although the high-end probe was UNREACHABLE**
- **Found during:** Task 2 (the first Grok tests failed with temperature omitted)
- **Issue:** the plan's derivation rule says `Range` only when both probed endpoints were accepted. For all three Grok models `temperature=2.00` returned `UNREACHABLE: error sending request` (three times), while `0.00` and `0.50` were accepted and `2.01` was rejected with "Temperature must be less than 2". Following the rule literally gives `NotSent`, which would refuse every explicit Grok temperature at build and omit the framework's `0.7` on every call, for a parameter the probe showed Grok accepts.
- **Fix:** seeded `Range(0.0, 1.99)`, the largest value under the bound the vendor's own observed text states, the same half-open shape the Qwen probe measured at 1.99. The comment in `seeded.rs` says the bound is stated, not measured. This is a judgment call on a written must-have; if the owner prefers the literal rule, the one-line change is `NotSent` in three `grok_*` builders plus `grok_seeds_match_the_transcript`. Re-running the Grok probe (a transport blip) turns the stated bound into a measured one.
- **Files modified:** `crates/paladin-llm/src/registry/seeded.rs`, `crates/paladin-llm/src/registry/mod.rs`
- **Commit:** `52ac8f09`

**2. [Rule 2 - Missing critical] OpenAI penalties seeded `NotSent` although both models accepted 0.5 (flagged assumption A3-50-06 not applied to the wire-less adapters)**
- **Found during:** Task 1
- **Issue:** A3 says an accepted penalty is seeded `Range(-2.0, 2.0)`. The OpenAI, Anthropic and Gemini request types have no penalty field (plan 50-05), so a `Range` would let the filter admit a value the adapter never sends, replacing today's recorded omission with a silent drop, the failure ADR-0061 exists to prevent.
- **Fix:** `NotSent` for those three adapters' penalties, with the reason in the table comment. DeepSeek, Qwen (which carry penalties on the wire) are `Range(-2.0, 2.0)`. Widening the OpenAI wire is out of scope here.
- **Files modified:** `crates/paladin-llm/src/registry/seeded.rs`
- **Commit:** `2118ccd7`

**3. [Rule 1 - Bug] `provider_swap_matrix::kimi_fixed_temperature_explicit` encoded a superseded measurement**
- **Found during:** Task 2 verification
- **Issue:** the case asserted an explicit `1.0` builds and is sent for Kimi's default model. The 2026-10-15 probe rejected `1.00`, `0.50` and `1.01` on `kimi-k2.6` and `kimi-k3` ("only 0.6 is allowed"), and the operator's evidence says this supersedes the earlier note. The seeded entry is `NotSent`, so every explicit temperature is refused at build.
- **Fix:** the case now asserts all three explicit values are refused naming provider, model and `temperature`; its doc comment records why. `tests/provider_swap_matrix.rs` is not in this plan's `files_modified`; the change is confined to that one case.
- **Files modified:** `tests/provider_swap_matrix.rs`
- **Commit:** `52ac8f09`

**4. [Rule 1 - Bug] Kimi, Grok and DeepSeek tests that described the adapter-wide fallback on the default model**
- **Found during:** Task 2
- **Issue:** six Kimi and Grok tests (and one DeepSeek capability assertion and one Anthropic fallback-window assertion) read the default model's profile expecting the fallback values, which the default model's own entry now replaces.
- **Fix:** the Kimi fallback tests run on an unseeded id (`UNSEEDED_MODEL`), the Grok profile test expects the seeded `Range(0.0, 1.99)`, and the window assertions expect the seeded windows. New tests pin the seeded behaviour (for example `kimi_default_model_omits_every_sampling_parameter_the_probe_rejected`).
- **Files modified:** the Kimi, Grok, DeepSeek and Anthropic adapter test modules
- **Commit:** `52ac8f09`

**5. [Rule 1 - Bug] DeepSeek `get_available_models` and `validate_model` listed only the two aliases the probe found undocumented**
- **Found during:** Task 2
- **Issue:** with the default moved to `deepseek-flash`, the hard-coded list `[deepseek-chat, deepseek-coder]` would have made `validate_model(default)` false. The plan names only Anthropic's list.
- **Fix:** `get_available_models` serves the DeepSeek table (`deepseek-flash`); `validate_model` already reads it.
- **Files modified:** `crates/paladin-llm/src/deepseek/adapter.rs`
- **Commit:** `52ac8f09`

**6. [Plan step not applicable] `make api-surface-update` not run; `grep -c ANTHROPIC_DEFAULT_MODEL .project/current-exports.txt` cannot reach 1**
- `.project/current-exports.txt` tracks the root `paladin` crate. `paladin_llm::<adapter>::adapter::*_DEFAULT_MODEL` constants are not re-exported there (the sibling `KIMI_DEFAULT_MODEL` count in the file is also 0; plan 50-10 adds the facade re-exports). `make api-surface` reports the surface unchanged, so the baseline file is untouched and that one acceptance grep does not hold as written. `make check-consumer-surface` passes.

### Notes, not deviations

- `get_available_models` for Anthropic now returns one id and `validate_model` returns `false` for current Anthropic models that were not probed (for example `claude-sonnet-5-5`). Neither gates a request; such a model runs on the adapter-wide fallback profile, visibly unprofiled.
- OpenAI has no adapter-level default model (the model arrives per request), so there is no default constant to move and its 128,000 `max_context_tokens` is unchanged; `gpt-5.4-nano` (the operator's default) and `gpt-4o-mini` are seeded and tested.

## Issues Encountered

None blocking.

## Observations for later plans (not fixed here)

- **OpenAI adapter sends `max_tokens`, and `gpt-5.4-nano` rejects it** ("Unsupported parameter: 'max_tokens' ... Use 'max_completion_tokens' instead", probe 11). The profile has no field for the token-limit parameter name. Anyone pinning `gpt-5.4-nano` through the OpenAI adapter will see a 400 until the adapter sends `max_completion_tokens` for such models.
- **The adapter-wide Kimi fallback is still `Fixed(1.0)` temperature and `Fixed(0.95)` top_p** (and `temperature_range (1.0, 1.0)` in the declared capabilities). The seeded entries say `NotSent`, from probes that disabled thinking; the fallback's values come from the 2026-08-22 measurement. They apply only to unseeded Kimi models. Plan 50-15's capability-truthfulness case is the place to reconcile them.
- **The declared windows now follow the default model, and the fallback profile follows the declared window.** An unprofiled model on Anthropic (200,000 for some listed ids), Qwen or Grok therefore gets the default model's larger window as its fallback, so a prompt sized for the fallback can overflow at the vendor; the vendor's own overflow status still maps to `TokenLimitExceeded` (T-50-06-01). Qwen's 1,000,000 comes from the vendor's China/Beijing page only (the International pages state no limit).
- `QWEN_FALLBACK_MODELS` still lists `qwen3-max`, which answered 404 "Model not exist." on the International endpoint in the probe. It is a curated degrade-gracefully list, untouched here.
- Not probed, so their defaults and windows are unchanged and unverified: Ollama (default `llama3`, no table, no window) and OpenAI-compatible (no table). OpenAI's table covers only the two probed models.
- Penalty (`-2..=2`) and `top_p` (`0..=1`) ranges are contract-derived from a single `0.5` probe, as the table comments say; plan 50-16's docs page should label them that way.

## Known Stubs

None.

## Threat Flags

None. The change adds static data and reads it; no endpoint, auth path or trust boundary was added. T-50-06-01 (window too large) is mitigated by tracing every value to the transcript and by the table test that rejects zero windows and a maximum output above the window; T-50-06-02 (unsourced fact) by `verified_on` on every entry and the URL comments; T-50-06-03 (default no longer resolves) by moving each default to a model present in the live listing and recording it in M-B-14 with the pin-to-keep instruction.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-07 can point the window readers at `ModelProfile.context_window` for every seeded model; the adapters' declared windows already equal their default model's seeded window.
- Plans 50-08 and 50-09 layer config overrides on these entries; the bridge and `config.example.yml` defaults (still the old model names in the root crate) follow through the new constants.
- Plan 50-13 can encode its usage-cap `error_override`s knowing the five compat presets stay unverified (owner reading R-2).
- Plan 50-16's drift test `switching_providers_page_lists_every_seeded_model` can iterate `seeded::<VENDOR>`.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/registry/seeded.rs` found; all twelve modified files are in the three task commits (`git status` clean apart from planning files).
- Commits: `2118ccd7`, `52ac8f09`, `579d9772` present in `git log`.
