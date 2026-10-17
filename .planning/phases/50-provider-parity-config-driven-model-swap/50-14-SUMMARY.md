---
phase: 50-provider-parity-config-driven-model-swap
plan: 14
subsystem: api
tags: [rust, paladin-llm, fallback, cadence, model-fallback, agent-runtime, semver-register, migration, parity]

requires:
  - phase: 50
    provides: "plan 50-12/50-13: LlmError::Refusal (Permanent) and the shared response guard on every adapter; plan 50-09/50-10: LlmProviderFactory::create_from_config and the facade/register pattern; plan 50-11: the extend-not-duplicate register rule; plan 50-02: the chain's model_profile intersection"
provides:
  - "FallbackStep { port, model } (FallbackStep::new) and FallbackLlmAdapter::from_steps: each hop is sent its own model on generate and generate_stream; FallbackLlmAdapter::new is unchanged"
  - "The hop's model feeds pace_hop and the inner CadenceLlmAdapter, so a 429 gates (hop provider, hop model) only"
  - "SERVED_MODEL_METADATA_KEY = paladin.served_model beside paladin.served_by on every served response, new and from_steps chains alike"
  - "FallbackLlmAdapter::hop_on_refusal(bool), default false: a Refusal hops only when opted in; every other Permanent error still short-circuits"
  - "FallbackChainError::EmptyStepModel { index }; chain model_profile asks each step about its own model"
  - "agent_runtime.model_fallback.steps: [{provider, model}] (FallbackStepConfig), AgentRuntimeDeps.llm_config, ModelFallbackConfig::resolve_steps through create_from_config, ModelFallbackMiddleware::from_steps / paced_steps, validation (providers and steps exclusive, names both keys)"
  - "Facade re-exports FallbackLlmAdapter, FallbackStep, FallbackChainError, SERVED_BY_METADATA_KEY, SERVED_MODEL_METADATA_KEY"
  - "provider_swap_matrix: fallback_hop_gets_its_own_model on from_steps, un-ignored; new config-path case; 50 passed, 0 ignored"
  - "Root-crate and paladin-llm register from a measured D-27 run: MIGRATION 9.2 rows, set-equal allowlist entries, 9.5 steps bullet, CHANGELOG Added and Breaking Changes"
affects: [50-15, 50-16]

tech-stack:
  added: []
  patterns:
    - "Per-hop request rewrite: the chain clones the caller's request per hop and sets model from a parallel Vec<Option<String>>, so a new chain (all None) is byte-identical to before"
    - "A public enum with one-name variants (UnresolvedProvider) carries step index and model inside the name rather than gaining fields"
    - "Register extend-not-duplicate: a second field on an already-registered struct edits the existing row and entry (AgentRuntimeDeps), counted before and after"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/fallback.rs
    - src/config/agent_runtime.rs
    - src/application/services/paladin/middleware/resilience.rs
    - src/lib.rs
    - tests/provider_swap_matrix.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - .project/current-exports.txt
    - CHANGELOG.md

key-decisions:
  - "A stream carries no response metadata, so paladin.served_by and paladin.served_model are recorded on buffered responses only (as served_by always was); a streamed chain still rewrites the model per hop"
  - "hop_on_refusal has no configuration switch: it is a code-level opt-in on the adapter (D-10); the config path always stops on a refusal"
  - "build_chain rejects providers and steps together as a problem as well as validate does, because build_chain can run without validate and silently preferring one spelling would hide the other"
  - "Mutual exclusion and empty-step checks apply only while model_fallback.enabled is true, matching every other section (a disabled section is inert)"
  - "paladin-llm gains one crate-wide allow line (enum_discriminants_undefined_non_unit_variant), added because the measurement fired it, not because it was forecast"

patterns-established:
  - "Config steps resolve through the same LlmProviderFactory::create_from_config path agents use, so a step is built from the operator's llm.<provider> block"

requirements-completed: [PARITY-06, PARITY-01]

coverage:
  - id: D1
    description: "D-15 each hop is sent its own model on generate and generate_stream; new chains keep sending the request's model"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "fallback::tests::{from_steps_sends_each_hop_its_own_model, from_steps_sends_each_hop_its_own_model_on_the_stream, a_new_chain_keeps_sending_every_hop_the_requests_model, same_port_different_models_are_two_hops}"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-00h Cadence is keyed on each step's model: a 429 on hop 1 gates (hop-1 provider, hop-1 model) only, and pace_hop reads that hop's own gate"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "fallback::tests::{cadence_is_keyed_on_each_steps_model, a_429_is_paced_on_the_hops_own_model}"
        status: pass
    human_judgment: false
  - id: D3
    description: "PARITY-06 served_by and served_model name the hop that succeeded, in declared order, not first or last configured"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "fallback::tests::{steps_are_tried_in_order_and_the_server_is_recorded, the_served_model_key_is_namespaced}"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-10 a Refusal short-circuits by default; hop_on_refusal(true) hops on a Refusal only, other Permanent errors still stop, exhaustion lists every refusal, stream path covered"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "fallback::tests::{refusal_stops_by_default, hop_on_refusal_tries_the_next_step, hop_on_refusal_does_not_hop_past_other_permanent_errors, hop_on_refusal_exhaustion_lists_every_refusal, hop_on_refusal_applies_to_the_stream}"
        status: pass
    human_judgment: false
  - id: D5
    description: "Edge empty: from_steps(vec![]) is EmptyChain, a step model empty after trimming is EmptyStepModel { index }; chain model_profile uses each step's own model"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "fallback::tests::{from_steps_rejects_an_empty_chain_and_an_empty_model, from_steps_model_profile_uses_each_steps_own_model, a_step_debug_names_the_provider_and_the_model}"
        status: pass
    human_judgment: false
  - id: D6
    description: "Config path (OQ7): steps resolve through create_from_config against AgentRuntimeDeps.llm_config and build a middleware whose hop 2 is sent its own model; providers-only configs build as before"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "config::agent_runtime::tests::{steps_deserialize_from_yaml_and_providers_only_configs_are_unchanged, resolve_steps_builds_each_step_with_its_own_model_from_the_llm_blocks, resolve_steps_collects_every_failure_naming_provider_index_and_model, resolve_steps_on_a_disabled_section_builds_nothing, build_chain_builds_a_steps_chain_paced_and_unpaced, build_chain_reports_an_unresolvable_step_and_a_providers_steps_clash, default_deps_carry_no_llm_config}; resilience::tests::{fallback_steps_middleware_sends_each_hop_its_own_model, fallback_steps_construction_validates_up_front}"
        status: pass
      - kind: integration
        ref: "tests/provider_swap_matrix.rs config_fallback_steps_send_each_hop_its_own_model (config -> build_chain -> middleware before_model -> mock vendor wire body)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Validation: providers and steps together name both keys; enabled with neither names both; an empty step provider or model names its index; a disabled section is not inspected; config_carries_no_secret_shaped_field covers steps"
    requirement: "PARITY-06"
    verification:
      - kind: unit
        ref: "config::agent_runtime::tests::{providers_and_steps_together_are_a_validation_error_naming_both_keys, enabled_with_neither_providers_nor_steps_names_both_keys, a_step_with_an_empty_provider_or_model_names_its_index, a_disabled_section_is_not_inspected, config_carries_no_secret_shaped_field}"
        status: pass
    human_judgment: false
  - id: D8
    description: "Matrix: fallback_hop_gets_its_own_model uses from_steps, is un-ignored and green; no case in provider_swap_matrix remains ignored"
    requirement: "PARITY-06"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all: 50 passed, 0 failed, 0 ignored; cargo test -q --test provider_swap_matrix --features llm-all -- --ignored --list lists no test"
        status: pass
    human_judgment: false
  - id: D9
    description: "D-00b register from a measured D-27 run on the final tree: rows, set-equal entries, the paladin-llm lint line; clean re-run with the tables in place exits 0 on both crates"
    requirement: "PARITY-01"
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ai / paladin-llm --baseline-version 0.10.1 --release-type minor --default-features: with allows disabled 196 checks, 1 and 2 lints fired; with tables in place 195 and 194 checks, exit 0; make check-migration-allowlist, make api-surface, make check-consumer-surface, make check-gates, make check-api-examples exit 0"
        status: pass
    human_judgment: false

duration: ~1h30m
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 14: Per-step models in fallback chains Summary

**A fallback chain now sends each hop its own model (library `FallbackLlmAdapter::from_steps` and configuration `agent_runtime.model_fallback.steps` alike), paces each hop on that model, records the served provider and model, and stops on a refusal unless the operator opted in; the provider-swap matrix runs 50 passed with nothing ignored.**

## Performance

- **Duration:** about 1h30m (about 25 min of it two cargo-semver-checks measurements)
- **Tasks:** 3 of 3; Tasks 1 and 2 as a RED commit then a GREEN commit, Task 3 as one register commit
- **Files:** 11 modified, none created (plus this SUMMARY)

## Accomplishments

- **Task 1 (library).** `FallbackStep { port, model }` (`Debug` prints provider name and model, no secret reachable), `FallbackLlmAdapter::from_steps`, `hop_on_refusal`, `SERVED_MODEL_METADATA_KEY` and `FallbackChainError::EmptyStepModel { index }`. The adapter keeps a parallel `Vec<Option<String>>` of step models (all `None` for `new`). Both hop loops build the hop's request once through `request_for_hop`, send it, pass its model to `pace_hop`, and stamp `paladin.served_by` and `paladin.served_model`. `after_failure` lets a `Refusal` through only when the flag is on. `model_profile` asks each step about its own model. The module rustdoc gained "One model per hop" and "A refusal stops the chain by default" sections; `from_steps` and `FallbackStep` carry `# Examples`.
- **Task 2 (configuration and middleware).** `FallbackStepConfig`, `ModelFallbackConfig.steps`, `AgentRuntimeDeps.llm_config`, `ModelFallbackConfig::resolve_steps` (each step through `create_from_config`; unknown, not-compiled and construction failures collected into the existing `UnresolvedProviders` shape with the step index and model in the name), `ModelFallbackMiddleware::from_steps` and `paced_steps`, and `build_chain` using `steps` when set. Facade re-exports added to `src/lib.rs`. The matrix case now builds on `from_steps` and is un-ignored; a second case drives config -> `build_chain` -> the middleware's `before_model` -> two mock vendors and asserts the second vendor's wire body carries the step's model and the response names `kimi` and that model.
- **Task 3 (register).** One measured D-27 run, then the rows, entries, manifest line, 9.5 bullet, CHANGELOG entries and refreshed API baseline in one commit.

## Task Commits

1. **Task 1 RED** - `d57a54b2` (test; API scaffolded without behaviour, 11 of the new tests failed)
2. **Task 1 GREEN** - `5b0c05f9` (feat; per-hop rewrite, served model, refusal opt-in)
3. **Task 2 RED** - `f85685df` (test; config types, middleware constructors and facade re-exports scaffolded, 4 unit tests and the config-path matrix case failed)
4. **Task 2 GREEN** - `6b41767e` (feat; `resolve_steps` and `build_chain` wired to `steps`)
5. **Task 3** - `9e7d2297` (feat; exactly `.cargo/semver-checks-allowlist.toml`, `.project/current-exports.txt`, `CHANGELOG.md`, `Cargo.toml`, `MIGRATION.md`, `crates/paladin-llm/Cargo.toml`)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## D-27 register (Task 3)

Tool: `cargo-semver-checks 0.50.0` (already installed, nothing installed). Baseline: published 0.10.1, `--release-type minor`, `--default-features`. For each crate the manifest was byte-copied into a `mktemp -d` directory, its crate-wide `[package.metadata.cargo-semver-checks.lints]` allow lines removed, the tool run, then the manifest restored; `cmp` proved the restore for both (`root cmp ok`, `llm cmp ok`). Raw outputs are in the session scratchpad (`d27-root.txt`, `d27-llm.txt`, `d27c-root.txt`, `d27c-llm.txt`); `target/semver-checks` was deleted afterwards.

| Crate | Allows disabled | Checks | Lints fired | On this plan's items |
|-------|-----------------|--------|-------------|----------------------|
| `paladin-ai` | `constructible_struct_adds_field` | 196: 195 pass, 1 fail | 1 (`constructible_struct_adds_field`) | `ModelFallbackConfig.steps`, `AgentRuntimeDeps.llm_config` (the rest of the lint's list is earlier-phase items: `WebhookPayload.halt_reason`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `Settings.treasurer`, `AgentRuntimeDeps.cadence`) |
| `paladin-llm` | `struct_marked_non_exhaustive` | 196: 194 pass, 2 fail | 2 | `enum_discriminants_undefined_non_unit_variant` on `FallbackChainError` (new here); `struct_marked_non_exhaustive` is the ten Phase 50 configs from plan 50-10, already registered |

Clean re-run with the tables in place: `paladin-ai` 195 checks, 195 pass, exit 0 ("no semver update required"); `paladin-llm` 194 checks, 194 pass, exit 0. `FallbackStep`, `from_steps`, `hop_on_refusal`, `SERVED_MODEL_METADATA_KEY`, `FallbackStepConfig`, `from_steps` and `paced_steps` on the middleware, `resolve_steps` and the facade re-exports fired no lint.

Register edits: new 9.2 rows `paladin-ai | ModelFallbackConfig` (`Y`) and `paladin-llm | FallbackChainError` (`Y`), register-only `N` rows `paladin-llm | FallbackLlmAdapter` (covers `FallbackStep`, `from_steps`, `hop_on_refusal`, the new constant and the facade re-exports) and `paladin-ai | ModelFallbackMiddleware`; the existing `paladin-ai | AgentRuntimeDeps` row and entry extended (`PACE-02, PARITY-06`). Allowlist entries with `migration_row = "paladin-ai | AgentRuntimeDeps"`: **1 before, 1 after** (extended, not duplicated); entries total 70 before, 72 after (the two new ones). `make check-migration-allowlist` exits 0.

## Verification

- `cargo fmt --check`: clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean. `make clean-code` not run end to end (shellcheck absent, as in the baseline).
- `cargo test -p paladin-llm --all-features --lib fallback::`: 46 passed (16 new). `cargo test -p paladin-llm --all-features --doc FallbackStep`: 1 passed. `cargo test --doc ModelFallbackMiddleware`: 3 passed.
- `cargo test --lib agent_runtime`: 36 passed (11 new). `cargo test --lib resilience`: 19 passed (2 new).
- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, **5020 passed, 2 failed** (baseline 4991 passed, 2 failed; the +29 are this plan's new tests). The two failures are exactly the ledgered `run_api_wiring` tests (WINDOWS.md row 73).
- `cargo test --test provider_swap_matrix --features llm-all`: **50 passed, 0 failed, 0 ignored** (was 48 passed, 1 ignored; the fallback case is un-ignored and one config-path case is new). `-- --ignored --list` lists no test.
- `make check-migration-allowlist`, `make api-surface` (after `make api-surface-update`: 4298 items, +20), `make check-consumer-surface`, `make check-gates`, `make check-api-examples` (105 entry points), `scripts/check-changelogs.sh`: exit 0.
- Acceptance greps: `pub struct FallbackStep` 1, `pub fn from_steps` 1, `pub const SERVED_MODEL_METADATA_KEY: &str = "paladin.served_model"` 1, `pub fn hop_on_refusal` 1 in `fallback.rs`; `pub steps: Vec<FallbackStepConfig>` 1 and `pub llm_config: Option<LlmConfig>` 1 in `agent_runtime.rs`; `pub fn from_steps` 1 in `resilience.rs`; 9.2 `ModelFallbackConfig` count 2.
- **WINDOWS row 71 (ignored matrix cases by plan):** after this plan no matrix case is ignored by design. Row 71 still lists the "50-14: 1" case; the ledger was not edited, so plan 50-16's closeout check reconciles it.
- **Manual credential-handling review (D-00a):** `FallbackStepConfig` and `ModelFallbackConfig` hold names only and derive `Debug` plainly; `config_carries_no_secret_shaped_field` now renders a config with steps and finds no `key`, `secret`, `token` or `password`. Keys reach a step only through `create_from_config` from the `llm:` block or its environment variable. The test that fails a step resolves a block whose key is an unset `${VAR}` and asserts the literal block key never appears in the error. `FallbackStep`'s `Debug` prints provider name and model only. No HTTP client, redirect policy or log line was added or changed.
- **Threat model:** T-50-14-01 mitigated (refusal stops by default; both directions tested on `generate` and `generate_stream`). T-50-14-02 mitigated (names-only config, extended secret-shape test). T-50-14-03 mitigated (`cadence_is_keyed_on_each_steps_model` and `a_429_is_paced_on_the_hops_own_model`). T-50-14-04 mitigated (per-hop rewrite on both paths, unit tests, matrix row and the config-path wire assertion).

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-14 held (`paladin.served_model` is the model the hop was sent; the vendor-echoed `LlmResponse.model` is untouched). **A2-50-14 did not hold:** the enum is `#[non_exhaustive]`, but its only variant was a unit variant, so adding the struct variant `EmptyStepModel { index }` fires `enum_discriminants_undefined_non_unit_variant` (see Deviations). A3-50-14 held (`constructible_struct_adds_field` on both root types, measured and registered).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `FallbackChainError::EmptyStepModel` is not purely additive; one crate-wide allow line added to `crates/paladin-llm/Cargo.toml`**
- **Found during:** Task 3 (the measured `paladin-llm` re-run). The plan forecast "all additive" and listed only the root `Cargo.toml` among files to modify.
- **Issue:** the enum's only variant was `EmptyChain` (a unit variant); adding a struct variant removes the enum's defined discriminants, which cargo-semver-checks reports as `enum_discriminants_undefined_non_unit_variant` (a numeric cast `FallbackChainError::EmptyChain as isize` stops compiling). Left unregistered, CI's `semver` job would fail.
- **Fix:** registered it on the measured evidence: one `paladin-llm | FallbackChainError` row (`Y`), one allowlist entry, one `= "allow"` line plus header paragraph in `crates/paladin-llm/Cargo.toml`, a CHANGELOG Breaking Changes bullet. The variant stays as the plan's truths specify (`EmptyStepModel { index }`); a unit variant would have avoided the lint but could not name the step.
- **Files modified:** `crates/paladin-llm/Cargo.toml`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`
- **Commit:** `9e7d2297`

**2. [Rule 2 - Missing critical functionality] `build_chain` also rejects `providers` and `steps` set together**
- **Found during:** Task 2. The plan puts the exclusion in `validate`; `build_chain` can run without `validate` and would otherwise silently prefer `steps`, hiding the operator's `providers` list.
- **Fix:** a `BuildFailed` problem naming both keys, tested in `build_chain_reports_an_unresolvable_step_and_a_providers_steps_clash`.
- **Commit:** `6b41767e`

### Scope and process notes

- **Streams carry no response metadata.** The plan says to record `paladin.served_model` on streams "on the terminal chunk path the existing served-by mechanism uses"; no such mechanism exists (`StreamingResponse` has no metadata map and `served_by` was never stamped on streams). Both keys are recorded on buffered responses only, as `served_by` always was; the stream path does rewrite the model per hop and pace on it, and is tested. Plan 50-18 option-b (`StreamingResponse.metadata`) is the place to extend this if the owner wants it. Stated in the module rustdoc.
- **`UnresolvedProvider` carries the step in its name.** The enum is public and its variants have one `name` field, so a step is named `provider (step N, model M)` instead of adding fields (a break).
- **No `hop_on_refusal` configuration key**; it is code-level only (D-10).
- **`paladin-llm` has doctests running** (the plan noted `doctest = false`, but `cargo test -p paladin-llm --doc FallbackStep` ran and passed 1 doctest).
- **TDD gate:** Task 1 and Task 2 each have a `test(...)` commit before the `feat(...)` commit. Each RED commit carries compile-clean scaffolds so the new tests fail on behaviour, not on a missing symbol; the pure validation tests in RED already passed against the scaffolds.
- No crate CHANGELOG (`crates/paladin-llm/CHANGELOG.md`) entry was written: plan 50-16 owns the Phase 50 crate changelog entries.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. The new surface (config steps selecting credentialed adapters) is the `operator agent_runtime config -> fallback chain` boundary already in the threat model.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-15's conformance work can rely on `Refusal` being `Permanent` in a chain by default and on `hop_on_refusal` being explicit.
- Plan 50-16's docs page should describe `agent_runtime.model_fallback.steps` (MIGRATION 9.5 has the text), reconcile WINDOWS row 71 (no ignored matrix case remains), add the crate CHANGELOG entries for `FallbackStep`, `from_steps`, `hop_on_refusal` and the `FallbackChainError` variant, and mention that `paladin.served_model` is recorded on buffered responses only.
- Open owner question carried from 50-12: the `total_tokens` decision (WINDOWS row 74). Not reopened here.

## Self-Check: PASSED

- Files: every entry in `key-files.modified` exists and is committed; this SUMMARY present.
- Commits `d57a54b2`, `5b0c05f9`, `f85685df`, `6b41767e`, `9e7d2297` present in `git log`; no file deletions in any of them.
