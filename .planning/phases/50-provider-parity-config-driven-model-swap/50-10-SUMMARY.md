---
phase: 50-provider-parity-config-driven-model-swap
plan: 10
subsystem: api
tags: [rust, paladin-llm, agent-host, facade, create-from-config, semver-register, migration, parity]

requires:
  - phase: 50
    provides: "plan 50-09: LlmProviderFactory::create_from_config / is_configured, the D-13 layering, LlmProviderConfig knobs; plan 50-08: the nine #[non_exhaustive] *Config types (option-a); plan 50-06: dated default models"
provides:
  - "agent_host::build_agent takes the operator's Option<&LlmConfig> and builds every config-defined and runtime-provisioned agent through create_from_config(cfg, provider, Some(&def.model)); no composition root calls the env-only create(name) for an agent any more"
  - "D-02: a block's default_temperature is passed to PaladinBuilder::temperature as an explicit value when the agent sets none; a refusal that names temperature is suffixed with the YAML key that set it (llm.<provider>.default_temperature or agents[<id>].temperature)"
  - "validate_config decides availability with LlmProviderFactory::is_configured over the settings' llm section; an unconfigured registered provider fails as HostBuildError::Provider naming agent, provider, model and the variable"
  - "FacadeProvisioner::with_llm_config (from_settings sets it); paladin_port_from_settings_with_cadence builds the run engine through create_from_config"
  - "LlmConfig::validate no longer requires the default provider's block to exist (env-only default provider is valid)"
  - "facade re-exports KimiAdapter/KimiConfig, QwenAdapter/QwenConfig, GrokAdapter/GrokConfig, GeminiAdapter/GeminiConfig, OllamaAdapter/OllamaConfig, OpenAiCompatibleAdapter/OpenAiCompatibleConfig behind their llm-* features, plus LlmConfig, LlmProviderConfig, ModelOverride unconditionally; ADR-0046 default features unchanged"
  - "provider_swap_matrix port_from_config builds every provider through create_from_config from one LlmConfig (40 passed, 9 ignored for later plans)"
  - "config.example.yml documents max_tokens, context_window and a models: override, uses the adapters' current default models, drops default_temperature where the default model does not admit it"
  - "D-27 register: paladin-llm [package.metadata.cargo-semver-checks.lints] (struct_marked_non_exhaustive), ten MIGRATION 9.2 rows, ten allowlist entries, MIGRATION 9.1 M-B-15 and a 9.5 block"
affects: [50-11, 50-14, 50-15, 50-16, 51]

tech-stack:
  added: []
  patterns:
    - "One construction path: every composition root resolves provider, model and block through create_from_config; an absent llm section is an empty LlmConfig, so env-only deployments resolve as create(name) did"
    - "The block's default_temperature and the agent's own temperature reach the builder the same way (explicit), and name_temperature_key appends the YAML key to a builder refusal so the operator knows which line to change"
    - "Register rows carry measured numbers only: both D-27 runs were saved, the lints table holds exactly the one lint that fired, and a clean re-run closes it"

key-files:
  created: []
  modified:
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/lib.rs
    - config.example.yml
    - tests/provider_swap_matrix.rs
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-llm/src/config/llm.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - .project/current-exports.txt
    - CHANGELOG.md

key-decisions:
  - "HostBuildError is not changed: an unconfigured registered provider is reported as HostBuildError::Provider (the factory's own ConfigurationMissing text names provider, model and variable); UnknownProvider is kept for a name this build does not register. The model cannot be added to UnknownProvider without a new field on a public enum"
  - "The run engine passes no model to create_from_config: None resolves the block's default_model (else the adapter default), identical to passing it, and a blank block default_model would be a refused malformed id if passed explicitly"
  - "LlmConfig::validate accepts a recognised default provider with no block (50-09's open item): the block is optional, is_configured and create_from_config report a credential missing from both places"
  - "Only struct_marked_non_exhaustive fired in the D-27 runs, so it is the only lint registered; constructible_struct_adds_field did not fire because a non_exhaustive struct is no longer constructible"
  - "The Kimi block's max_tokens, context_window and models examples are comments, so the shipped file behaves exactly as its defaults"

patterns-established:
  - "A refusal test over the shipped example config: replace every key with a literal and build an agent on each block's default model under the block's own default_temperature, so the example cannot refuse itself (mutation-checked)"

requirements-completed: [PARITY-03, PARITY-01]

coverage:
  - id: D1
    description: "agent_host and facade_provisioner build agents through create_from_config with the agent's provider, model and block: host and key from the block, model from the agent, no env var needed; a runtime AgentSpec builds on the default provider with the caller's model"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "agent_host::config_driven_tests::an_agent_is_built_from_its_providers_block_and_its_own_model; facade_provisioner::tests::a_runtime_agent_builds_on_the_default_providers_block_with_the_callers_model; the_run_engine_is_built_from_the_default_providers_yaml_block (mockito sees the block's key and the agent's model)"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-02: a block default_temperature is an explicit value; a refusal names provider, model, temperature and the YAML key (llm.<provider>.default_temperature or agents[<id>].temperature); the agent's own value beats the block's; silence expresses nothing; no key in any error"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "agent_host::config_driven_tests::a_blocks_default_temperature_is_explicit_and_a_refusal_names_its_key; an_agents_own_temperature_refusal_names_the_agent_key; the_agents_temperature_beats_the_blocks_and_silence_expresses_nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "validate_config accepts a YAML-only provider and rejects an unconfigured one naming agent, provider, model and variable; LlmConfig::validate accepts an env-only default provider"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "agent_host::config_driven_tests::validate_config_accepts_a_provider_configured_only_in_yaml; validate_config_rejects_an_unconfigured_provider_naming_agent_provider_and_model; llm_config_validate_accepts_a_default_provider_with_no_block; paladin-llm test_llm_config_validate_default_provider_block_is_optional"
        status: pass
    human_judgment: false
  - id: D4
    description: "Pitfall 13 and the PARITY-03 prohibition: the agent's timeout_seconds never becomes the provider HTTP timeout; a runtime caller supplies a model and nothing else (tripwire destructure of AgentSpec); a malformed runtime model id is refused naming provider and model"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "agent_host::config_driven_tests::the_agent_timeout_does_not_become_the_http_client_timeout (1 s agent timeout, 1.3 s vendor, 30 s block timeout, completes); facade_provisioner::tests::a_runtime_spec_carries_no_provider_construction_input; a_malformed_runtime_model_is_refused_naming_provider_and_model"
        status: pass
    human_judgment: false
  - id: D5
    description: "the facade exports all nine adapter/config pairs behind their llm-* features plus LlmConfig, LlmProviderConfig and ModelOverride; default features unchanged; each feature resolves alone"
    requirement: "PARITY-03"
    verification:
      - kind: integration
        ref: "cargo check --no-default-features --features llm-kimi (and llm-qwen, llm-grok, llm-gemini, llm-ollama, llm-openai-compatible, each alone); make check-consumer-surface; make api-surface (baseline refreshed to 4278 items); grep '^default = [\"llm-openai\", \"llm-anthropic\", \"llm-deepseek\"]' Cargo.toml prints 1"
        status: pass
    human_judgment: false
  - id: D6
    description: "PARITY-01 through config: the matrix builds every provider with create_from_config from one LlmConfig and every non-ignored case is green"
    requirement: "PARITY-01"
    verification:
      - kind: integration
        ref: "cargo test --test provider_swap_matrix --features llm-all (40 passed, 9 ignored for later plans)"
        status: pass
    human_judgment: false
  - id: D7
    description: "config.example.yml loads, documents the new keys, uses current default models, and does not refuse itself (Pitfall 7)"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "agent_host::config_driven_tests::the_shipped_example_config_builds_every_agent_it_describes (default features and llm-all; mutation-checked by re-adding default_temperature: 0.7 to the Anthropic block); paladin-llm example_config_llm_section_loads_unchanged"
        status: pass
    human_judgment: false
  - id: D8
    description: "D-00b register for paladin-llm: measured D-27 run, lints table, ten 9.2 rows, ten allowlist entries, set-equal; the diagnostic re-run with the table in place exits 0"
    requirement: "PARITY-03"
    verification:
      - kind: integration
        ref: "cargo semver-checks check-release --package paladin-llm --baseline-version 0.10.1 --release-type minor (--default-features, and --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible): before 196 checks / 1 lint fired; after 195 checks, 195 pass, exit 0 on both; make check-migration-allowlist exits 0"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 10: Composition roots on create_from_config, nine-adapter facade, measured semver register Summary

**Every agent the server builds (from YAML, at runtime, and the run engine) now resolves its provider, model and `llm.<provider>` block through `create_from_config`, a block's `default_temperature` is an explicit value whose refusal names the YAML key, the facade exports all nine adapter/config pairs, and the ten `paladin-llm` config breaks are registered from a measured cargo-semver-checks run.**

## Performance

- **Duration:** about 45 min (start time not captured; estimated from the 50-09 docs commit and the first 50-10 commit)
- **Completed:** 2026-10-16
- **Tasks:** 3 of 3 (five commits: Task 1 has a RED and a GREEN commit, plus a doc-example follow-up)
- **Files:** 11 modified, 0 created

## Accomplishments

- **Task 1 (composition roots).** `build_agent` takes the settings' `Option<&LlmConfig>` and calls `create_from_config(cfg, &provider, Some(&def.model))` (an absent section is an empty `LlmConfig`, so env-only deployments resolve as `create(name)` did). When the agent sets no `temperature` and the resolved block sets `default_temperature`, `PaladinBuilder::temperature` receives it; a builder refusal that names `temperature` gains a suffix ``(set by `llm.<provider>.default_temperature`)`` or ``(set by `agents[<id>].temperature`)``. `validate_config` pass 2 uses `LlmProviderFactory::is_configured`. `FacadeProvisioner` gains `with_llm_config` (set by `from_settings`) and `paladin_port_from_settings_with_cadence` builds the run engine through `create_from_config`. `def.timeout_seconds` is never read by the build path (comment at the call site, proven by a behavioural test).
- **Task 2 (facade, matrix, docs).** Six `#[cfg(feature = "llm-*")] pub use` pairs and three unconditional `pub use` lines in `src/lib.rs`. The matrix's `port_from_config` is one `LlmConfig` plus `create_from_config`, everything imported from the facade. `config.example.yml`: precedence and `${VAR}` paragraph, current default models (OpenAI `gpt-4o-mini` since the adapter has no default constant, Anthropic `claude-haiku-5-5`, DeepSeek `deepseek-flash`, Kimi `kimi-k2.6`, Qwen `qwen3.8-flash`, Grok `grok-4.3`, Gemini `gemini-3.5-flash-lite`), the Anthropic `default_temperature` removed (the model takes only 1.0), commented `max_tokens`/`context_window`/`models:` in the Kimi block, example agents moved to `gpt-4o-mini`. MIGRATION.md 9.1 M-B-15 and a 9.5 block; `make api-surface-update` (4275 -> 4278 items: `LlmConfig`, `LlmProviderConfig`, `ModelOverride`); CHANGELOG `### Added` and `### Changed` entries.
- **Task 3 (register).** D-27 measured twice on this tree (details below), `crates/paladin-llm/Cargo.toml` gains the lints table with the one lint that fired, MIGRATION.md 9.2 gains ten `paladin-llm` rows (nine configs and `LlmProviderConfig`), the allowlist gains ten entries under the Phase 50 header, and the re-run with the table in place exits 0.

## D-27 diagnostic (measured, saved)

Tool: `cargo-semver-checks 0.50.0` (already installed, nothing installed). Baseline: published `paladin-llm` 0.10.1, `--release-type minor`. `paladin-llm` had no lints table, so nothing was disabled and nothing needed restoring.

| Run | Command flags | Checks | Lints fired | Items flagged |
|---|---|---|---|---|
| 1 | `--default-features` | 196 (195 pass, 1 fail, 58 skip) | 1: `struct_marked_non_exhaustive` | `OpenAIConfig`, `LlmProviderConfig` |
| 2 | `--features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible` | 196 (195 pass, 1 fail, 58 skip) | 1: `struct_marked_non_exhaustive` | all ten: the nine `*Config` types and `LlmProviderConfig` |
| re-run, table in place | both of the above | 195 (195 pass, 0 fail, 59 skip) | 0 | exit 0 on both |

- Phase 50 items versus other (A1-50-10): all ten flagged items are Phase 50 items. **Nothing pre-existing fired** against 0.10.1 for `paladin-llm`, so nothing is reported to the owner as unregistered.
- `constructible_struct_adds_field` and `struct_pub_field_missing` (which the 50-09 note anticipated) did **not** fire: a `#[non_exhaustive]` struct is no longer constructible outside the crate, so the only lint the tool models is `struct_marked_non_exhaustive`. Only that line is in the table; no other lint was registered or guessed.
- Raw outputs were saved in the session scratchpad (`d27-default.txt`, `d27-features.txt`); `target/semver-checks` was deleted afterwards.

## Task Commits

1. **Task 1 RED: failing tests for config-driven agent construction** - `ee636e85` (test; `build_agent` signature and `with_llm_config` scaffold compile, 11 tests fail)
2. **Task 1 GREEN: agents through create_from_config** - `011d0824` (feat)
3. **Task 2: facade exports, matrix, example config, MIGRATION 9.1/9.5, baseline, CHANGELOG** - `22ad0a80` (feat)
4. **Task 3: the paladin-llm register** - `565567ab` (docs; exactly `.cargo/semver-checks-allowlist.toml`, `MIGRATION.md`, `crates/paladin-llm/Cargo.toml`)
5. **Doc example for `FacadeProvisioner::with_llm_config`** - `27265380` (docs)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, **4938 passed, 2 failed** -- exactly the two ledgered `run_api_wiring` baseline failures (WINDOWS.md row 73); baseline before the plan was 4924 passed, so the delta is this plan's 14 new tests.
- `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored (later plans). `cargo test -p paladin-llm --all-features --lib`: 954 passed. `cargo test --test unit llm::`: 37 passed. `cargo test --test web_server_e2e --test paladin_server_smoke --features web-server`: pass.
- `cargo fmt --check` clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean. `make check-gates`, `make check-consumer-surface`, `make api-surface` (unchanged after the refresh), `make check-api-examples` (105 entry points), `make check-migration-allowlist`, `scripts/check-changelogs.sh`: pass. `make clean-code` was not run end to end (shellcheck absent); fmt and clippy were run directly.
- `cargo check --no-default-features --features <llm-kimi|llm-qwen|llm-grok|llm-gemini|llm-ollama|llm-openai-compatible>`: each compiles alone; default features unchanged.
- Acceptance greps: `create_from_config` 4 in `agent_host.rs`, 1 in `facade_provisioner.rs`; `.create(&provider)` 0 in both; `is_configured` 5 in `agent_host.rs`; `pub use paladin_llm::kimi::{KimiAdapter, KimiConfig};` 1, six `llm-*` cfg lines; `grep -c create_from_config tests/provider_swap_matrix.rs` at least 1; `default = [...]` line 1; 9.5 `Phase 50` at least 1 and 9.1 `Phase 50 plans 50-09/50-10` exactly 1; `paladin-llm` rows in 9.2: 15 (5 earlier plus 10 new); allowlist entries after the Phase 50 header: 10, all `PARITY-03`; `git show --name-only HEAD` for the Task 3 commit lists exactly the three files.
- **Mutation evidence.** Re-adding `default_temperature: 0.7` to the Anthropic block of `config.example.yml` made `the_shipped_example_config_builds_every_agent_it_describes` fail with the refusal text ("model 'claude-haiku-5-5' on provider 'anthropic' does not accept parameter 'temperature' = 0.7: it takes only 1 (set by `llm.anthropic.default_temperature`)"); reverted (restored from a copy; see Issues Encountered).
- **Manual credential-handling review (D-00a).** (1) No composition root logs or formats a key: `build_agent`, `validate_config` and `paladin_port_from_settings_with_cadence` pass blocks by reference to `create_from_config`, whose errors name provider, model and variable only (the 50-09 contract); tests assert the block key is absent from the temperature-refusal text, the unconfigured-provider text and the malformed-model text. (2) The new `llm_config` field on `FacadeProvisioner` holds `LlmProviderConfig` values whose manual `Debug` prints `<redacted>`; `FacadeProvisioner` itself has no `Debug`. (3) No HTTP client, header or redirect policy was touched: every adapter is still built by the factory, and 50-09's `factory_built_adapter_refuses_redirects` covers `Policy::none()`. (4) The configured `base_url` still decides where a credential goes; it is operator-only (`AgentSpec` carries none), enforced by the tripwire test. (5) The shipped example uses `${VAR}` placeholders only, no literal key.
- **Threat model.** T-50-10-01 mitigated (tripwire destructure of `AgentSpec`, `spec_to_definition` leaves `provider` `None`, the factory shape-checks the model, malformed-model test); -02 mitigated (key-absent assertions on every new error string); -03 mitigated (example `default_temperature` removed where not admitted, the example-build test over every block and agent, mutation-checked); -04 mitigated (both D-27 runs measured and saved, one lint registered); -SC not triggered (the tool was already installed at 0.50.0, no package added, `Cargo.lock` untouched).

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-10 held (nothing outside Phase 50 fired), A2-50-10 held (0.50.0 present, no install), A3-50-10 applied (runtime agents always use `llm.default_provider` with the caller's model; the factory's model-id shape check guards the string).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] `LlmConfig::validate` required the default provider's block (50-09 hand-off, in the prompt's prior_decisions)**
- **Found during:** Task 1 (a YAML-less default provider could not pass `validate`, though `create_from_config` builds it from the environment)
- **Fix:** a recognised default provider with no block is now valid; an unrecognised name is still rejected with the registered names. The pinned test `test_llm_config_validate_default_provider_must_be_configured` was flipped to `test_llm_config_validate_default_provider_block_is_optional` (it encoded the old behaviour).
- **Files modified:** `crates/paladin-llm/src/config/llm.rs` (not in the plan's file list)
- **Commit:** `011d0824`

**2. [Rule 3 - Blocking] `HostBuildError::UnknownProvider` cannot name the model without changing a public enum**
- **Issue:** the plan says to name the model in the `validate_config` error "extend the message, not the enum", but the variant has no model field and a format string can only use fields.
- **Fix:** a registered provider with no usable credential is reported as `HostBuildError::Provider { id, provider, source }` whose `source` is the factory's own `ConfigurationMissing` text (provider, model, variable); `UnknownProvider` is kept, unchanged, for a name this build does not register (its `available` list is now the registered providers that `is_configured` accepts). No enum change. The existing `validate_rejects_unknown_provider` test is unchanged and passes.
- **Files modified:** `src/infrastructure/web/agent_host.rs`
- **Commit:** `011d0824`

**3. [Rule 3 - Blocking] `build_agent_with_llm` became test-only**
- **Issue:** with the block temperature threaded through a new private `build_agent_with_llm_and_block_temperature`, the five-argument `build_agent_with_llm` had no production caller (a `dead_code` error under `-D warnings`), though 14 hermetic tests use it.
- **Fix:** `#[cfg(test)]` on it, with a doc note pointing production builds to `build_agent`. No test signature changed.
- **Commit:** `011d0824`

**4. [Plan wording not followed literally] The run engine passes no model**
- The plan says `create_from_config(cfg, &default_provider, block default model)`. The layering resolves `None` to exactly the block's `default_model` (else the adapter default), and passing a blank block `default_model` explicitly would be refused as a malformed per-agent id, so `None` is equivalent and safer. Commented at the call site.

**5. [Rule 1 - Bug] Example config would have refused itself, and two pinned assertions encoded the old example**
- The Anthropic block's `default_temperature: 0.7` (and the retired default models) would now refuse every agent on that provider; the example agents named `gpt-4`, which is unprofiled. Fixed as listed under Task 2. `example_config_llm_section_loads_unchanged` in `paladin-llm` pinned `gpt-4`, `claude-3-5-sonnet-20241022` and `kimi-k3`; it now pins the new example values and asserts the Anthropic block carries no `default_temperature`.
- **Files modified:** `config.example.yml`, `crates/paladin-llm/src/config/llm.rs`
- **Commit:** `22ad0a80`

**6. [Plan wording interpreted] Kimi block's new keys are commented**
- "document `max_tokens`, `context_window` and a commented `models:` example": all three are shown as comments so the shipped file behaves exactly as its defaults (the pinned test also keeps asserting `max_tokens` and `context_window` are `None` for every block).

**7. [Rule 2 - public API needs a doc test] `FacadeProvisioner::with_llm_config` example**
- Added a runnable doc example in a follow-up commit (`27265380`).

**8. [TDD gate] Tasks 2 and 3 have no RED commit**
- Task 1 has `ee636e85` (RED) before `011d0824` (GREEN). Task 2 is re-exports, a test-helper rewrite, config and documentation (verified by existing and new tests: the matrix, the example-build test with its mutation check, `cargo check` per feature); Task 3 is a measured register verified by the tool itself (fires before, exit 0 after).

**9. [Rule 3 - Blocking] Disk**
- Free space fell to 4.2 GB before the semver run; deleted `target/debug/incremental` (11 GB) and `target/doc`, both regenerable, then `target/semver-checks` after the run. No tracked file affected.

### Notes, not deviations

- **A config-defined agent's `provider` and `model` were already honoured by name;** what is new is that the block now supplies host, key, limits and overrides, and the agent's model reaches the adapter.
- **Behaviour change for existing deployments** (M-B-15, CHANGELOG `### Changed`): a populated `llm.<provider>` block now takes effect, and its `default_temperature` is judged against the model's profile at startup.

## Issues Encountered

- During the mutation check I ran `git checkout config.example.yml` to undo the mutation, which also discarded my uncommitted edits to that file. They were restored byte-for-byte from a copy taken just before the mutation and re-verified (`git diff --stat`, the example-build test, the matrix); nothing was lost.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or trust boundary was added; the credential surface did not grow (blocks are operator-only, `AgentSpec` still carries a model and nothing else).

## Observations for later plans (not fixed here)

- `default_provider_name` still falls back to the env-only `LlmProviderFactory::get_default_provider()` when `llm.default_provider` is unset, so a YAML-only deployment must name its default provider (the shipped example does). Resolving the default from the blocks would be a behaviour change beyond this plan.
- OpenAI `gpt-5.4-nano` still rejects `max_tokens` (carried from 50-06/50-08/50-09); the example uses `gpt-4o-mini` for the OpenAI block and agents for that reason.
- Plans 50-11, 50-14 and 50-18 change other published types and register their own rows and allowlist entries in their own commits; this register covers only the ten configs measured here.

## User Setup Required

None.

## Next Phase Readiness

- Phase 50 plans that follow can rely on `create_from_config` as the one construction path in the server, the facade exporting all nine adapters, and the `paladin-llm` lints table existing (a later `paladin-llm` break adds its lint line, row and entry together).
- `MIGRATION.md` 9.1 M-B-15 and the 9.5 block are the consumer-facing description of live `llm:` blocks; plan 50-16 links them from the switching-providers page.

## Self-Check: PASSED

- Files: `.planning/phases/50-provider-parity-config-driven-model-swap/50-10-SUMMARY.md` (this file), and the eleven modified files above are present in the tree.
- Commits: `ee636e85`, `011d0824`, `22ad0a80`, `565567ab`, `27265380` present in `git log`; no deletions in any of them.
