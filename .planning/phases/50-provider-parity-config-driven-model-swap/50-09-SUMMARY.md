---
phase: 50-provider-parity-config-driven-model-swap
plan: 09
subsystem: api
tags: [rust, paladin-llm, config, layering, provider-factory, placeholders, redaction, adr-0061, parity]

requires:
  - phase: 50
    provides: "plan 50-08: the nine #[non_exhaustive] *Config types with max_tokens/max_retries/context_window/model_overrides and with_* builders, registry::is_reasoning_model, REASONING_DEFAULT_TIMEOUT_SECONDS, DEFAULT_MAX_RETRIES, RedactedKey, crate-level test_support; plans 50-06/07: seeded tables and default-model constants, effective_capabilities"
provides:
  - "LlmProviderConfig.{max_tokens, context_window, models} (all #[serde(default)], skipped when empty), #[non_exhaustive] with new(api_key) and with_* builders including with_models, a manual Debug printing the key as <redacted>, and #[serde(default)] on api_key"
  - "crates/paladin-llm/src/config/layers.rs: resolve_secret_placeholder / SecretSource, ProviderEnv, CompiledDefaults, resolve_layers (D-13: agent > block > legacy env > compiled, 300 s reasoning default honouring the block's models overrides), resolve_layers_lenient, validate_model_id, subject"
  - "config/bridge.rs: <provider>_from_layers for all nine adapters; the From/TryFrom conversions re-expressed over the same layering with no environment; stale bridge defaults replaced by the adapters' own"
  - "LlmProviderFactory::create_from_config(&LlmConfig, provider, model) and LlmProviderFactory::is_configured(Option<&LlmConfig>, provider); ProviderRegistration.construct_from_config; create(name) goes through the same layering with no block"
  - "pub(crate) create_from_config_with_env / is_configured_with_env taking an injected environment closure (no std::env access in any test)"
affects: [50-10, 50-14, 50-15, 50-16, 51]

tech-stack:
  added: []
  patterns:
    - "One resolve() core returns (Layered, problems): the strict entry reports the first problem as Err, the lenient entry (used by the infallible From impls) turns every problem into an empty field, so there is one layering and no second code path to drift"
    - "The environment is a closure argument everywhere; the layering is a pure function of its inputs and an environment snapshot, so concurrent callers share no state and tests never write the process environment"
    - "An environment variable is read only for a field the block leaves unset, so a variable the block overrides can never matter (even a malformed one)"
    - "Error strings carry a provider 'p' (model 'm') prefix built by one helper that escapes and bounds the model id, and name variables only, never values"

key-files:
  created:
    - crates/paladin-llm/src/config/layers.rs
  modified:
    - crates/paladin-llm/src/config/llm.rs
    - crates/paladin-llm/src/config/bridge.rs
    - crates/paladin-llm/src/config/mod.rs
    - crates/paladin-llm/src/provider_factory.rs
    - crates/paladin-llm/src/redaction.rs
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - src/infrastructure/adapters/llm/config_bridge.rs
    - tests/provider_swap_matrix.rs
    - CHANGELOG.md

key-decisions:
  - "An unset ${NAME} placeholder is an error even when the provider's own key variable is set: the operator named a specific credential, and substituting a different one would be a silent override of the more specific statement. is_configured agrees (false), so it reports what create_from_config would do"
  - "Blank block strings (base_url, default_model) make no statement and fall through to the next layer; a blank per-agent model is a refused malformed id, not an absent one"
  - "The two operator-written model statements (the per-agent model and the block's default_model) are shape-checked; a model arriving from a legacy env var keeps its pre-layering behaviour of reaching the adapter unchecked, so create(name) does not start refusing a previously accepted value"
  - "The 300 s reasoning default is decided by the same registry::is_reasoning_model as the direct constructors, but fed the block's models overrides, so an override-only reasoning: true model gets 300 s here (the direct constructors cannot see it)"
  - "The From conversions stay infallible by running the lenient layering with an empty environment: an unresolved key or base URL becomes empty and the adapter's own construction check refuses it; a ${NAME} key is never carried through as the credential"
  - "api_key stays serialisable: no production path serialises Settings or LlmConfig outward (evidence below)"

patterns-established:
  - "A redirect test must give the target mock both GET and POST at expect(0) and use a 307: reqwest turns a followed 302 POST into a GET, so a POST-only target mock passes even when redirects are followed"

requirements-completed: [PARITY-03, PARITY-04]

coverage:
  - id: D1
    description: "PROV-03: every existing YAML file loads unchanged; the new block keys are optional and skipped when empty; an unknown key under a model is rejected naming it; a block with no api_key deserializes and validate accepts it"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "llm::tests::existing_yaml_loads_unchanged; example_config_llm_section_loads_unchanged (config.example.yml, including its keyless ollama block); a_models_entry_deserializes_into_one_override; an_unknown_key_under_a_model_is_rejected_naming_it; a_block_with_no_api_key_deserializes_with_an_empty_key; validate_accepts_an_empty_key_filled_from_env; empty_new_fields_are_not_serialized"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-13 precedence per field with an injected environment: block beats env for every field, env fills gaps, compiled default last, per-agent model beats the block's default_model and an equal one yields an identical config, the order is fixed whichever layer sets which field"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "layers::tests::block_beats_env_for_every_field; env_fills_what_the_block_leaves_unset; compiled_defaults_apply_when_nothing_else_does; per_agent_model_beats_block_default_model; a_per_agent_model_equal_to_the_block_default_yields_an_identical_config; the_precedence_order_does_not_depend_on_which_layers_set_which_fields; a_block_value_equal_to_its_env_var_yields_the_same_config_as_either_alone; bridge tests block_beats_env_for_every_field (anthropic, openai, kimi); env_never_overrides_the_block_end_to_end (a mock vendor sees the block's key, not the env's)"
        status: pass
    human_judgment: false
  - id: D3
    description: "the reasoning-model timeout default is 300 s only when neither block nor env sets one, any other model keeps the adapter's compiled default, an override-only reasoning model counts"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "layers::tests::reasoning_model_defaults_to_300s; the_reasoning_check_sees_the_resolved_model; bridge kimi::reasoning_model_defaults_to_300s_through_the_bridge (seeded default, unseen model, override-only reasoning true, override reasoning false beating the seed, explicit block and env timeouts)"
        status: pass
    human_judgment: false
  - id: D4
    description: "placeholders: only an exact ${NAME} resolves; an unset variable is a construction error naming provider, model and variable and is never sent; abc${X} and $X are literal; an empty key falls back to the provider variable; no error contains a key value"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "layers::tests::only_an_exact_placeholder_is_a_placeholder; an_unset_or_blank_placeholder_variable_is_missing_not_literal; an_unset_placeholder_variable_is_an_error_naming_provider_and_variable; no_error_ever_contains_a_key_value; bridge kimi::a_placeholder_key_resolves_from_its_variable, an_unset_placeholder_variable_names_provider_model_and_variable, literal_lookalikes_are_used_as_written, the_infallible_from_never_carries_a_placeholder_as_the_key; factory a_placeholder_key_is_resolved_and_never_sent_literally (mock vendor sees Bearer <resolved value>)"
        status: pass
    human_judgment: false
  - id: D5
    description: "create_from_config builds every compiled provider from its block (request reaches the mocked endpoint with the block's model), honours models overrides and the adapter-wide window, and names provider and model on every error including unknown provider and malformed model id"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "provider_factory::tests::from_config::create_from_config_builds_each_provider_from_its_block; create_from_config_honours_model_overrides; a_bespoke_adapter_honours_model_overrides_too; an_invalid_override_fails_construction_naming_provider_and_model; create_from_config_names_provider_and_model_on_error (loops every compiled provider, seven bad model shapes and a 257-character id, asserts the key is absent from every message); a_malformed_model_is_refused_before_any_adapter_is_built; a_per_agent_model_equal_to_the_block_default_builds_an_identical_adapter"
        status: pass
    human_judgment: false
  - id: D6
    description: "a factory-built adapter refuses redirects on every compiled provider; create_from_config is safe under 16-way concurrency; spellings of openai-compatible resolve to one block"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "factory_built_adapter_refuses_redirects (307 from the vendor, GET and POST both expect(0) on the target; verified sensitive by mutating Kimi to Policy::limited(5)); concurrent_create_from_config_builds_independent_adapters (16 spawn_blocking tasks); provider_name_spellings_resolve_alike"
        status: pass
    human_judgment: false
  - id: D7
    description: "is_configured: a YAML-only provider is available, env-only likewise, keyless Ollama always, an unset placeholder is not configured; create(name) goes through the layering with no block and still names the provider and variable"
    requirement: "PARITY-03"
    verification:
      - kind: unit
        ref: "is_configured_accepts_yaml_only_and_env_only_providers; is_configured_agrees_with_create_from_config; create_uses_the_layering_without_a_block; create_keeps_naming_openai_api_key_for_an_openai_with_no_credential; the 14 pre-existing provider_factory tests"
        status: pass
    human_judgment: false
  - id: D8
    description: "the stale bridge defaults are the adapters' own and the rest of the workspace is not regressed"
    requirement: "PARITY-04"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-llm --all-features --lib (954 passed); cargo test --workspace --lib --bins --no-fail-fast (4924 passed, 2 failed: the two ledgered run_api_wiring baseline failures); cargo test --test provider_swap_matrix --features llm-all (40 passed, 9 ignored for later plans); cargo test --test unit llm:: (37 passed); cargo test --lib config_bridge --features llm-all (6 passed); make check-consumer-surface, make check-api-examples, make check-migration-allowlist, make api-surface (unchanged), scripts/check-changelogs.sh"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 09: Config-driven provider construction Summary

**`LlmProviderFactory::create_from_config` builds any of the nine adapters from its `llm.<provider>` block through one layering (per-agent model, then block, then the provider's legacy env vars, then the compiled default, per field), with a 300 s default for reasoning models, exact `${VAR}` key placeholders, errors that name provider and model and never a key, and a YAML-only provider reported available by `is_configured`.**

## Performance

- **Duration:** about 40 min (start time not captured; estimated from the STATE.md timestamp of plan 50-08 and the first commit)
- **Completed:** 2026-10-16
- **Tasks:** 3 of 3 (four commits: Task 1 has a RED and a GREEN commit)
- **Files:** 12 modified, 1 created

## Accomplishments

- **Provider block (Task 1).** `LlmProviderConfig` gains `max_tokens`, `context_window` and `models: BTreeMap<String, ModelOverride>`, is `#[non_exhaustive]` per the 50-08 option-a decision (`new(api_key)` plus `with_base_url`, `with_default_model`, `with_default_temperature`, `with_timeout_seconds`, `with_max_retries`, `with_max_tokens`, `with_context_window`, `with_models`), and prints its key as `<redacted>` from a manual `Debug`. `api_key` is `#[serde(default)]` and `LlmConfig::validate` no longer rejects an empty key (A1-50-09), so `config.example.yml`'s keyless `ollama` block now loads (the whole file used to fail `try_deserialize` on it). Literal sites in `config/llm.rs` tests, the root `config_bridge.rs` tests and the matrix's `matrix_block` migrated to the builders.
- **Layering (Task 2).** `config/layers.rs` is a pure function of its inputs: `resolve_secret_placeholder` (exact `${[A-Z0-9_]+}` only), `ProviderEnv` naming one provider's legacy variables (read from each adapter's `from_env`, none added), `CompiledDefaults`, `resolve_layers`, `validate_model_id` and a `subject()` helper that builds the `provider 'p' (model 'm')` prefix with the model escaped and bounded. `config/bridge.rs` has `<provider>_from_layers` for all nine; the `From`/`TryFrom` conversions run the same layering with no environment. Bridge defaults are now the adapters' own: Anthropic `claude-haiku-5-5` at 60 s (was the October-2024 Sonnet id at 300 s), DeepSeek `deepseek-flash` at 300 s as a reasoning model (was `deepseek-chat` at 60 s). The generic provider's capability variables are read through a new `OpenAiCompatibleCapabilitiesConfig::from_lookup`.
- **Factory (Task 3).** `ProviderRegistration.construct_from_config` replaces `construct`; each provider's row calls its `from_layers` then `*Adapter::new`, and an adapter refusal is prefixed with provider and model. `create_from_config` (public), `create_from_config_with_env` and `is_configured_with_env` (crate-private, injected env) added; `create(name)` is `construct_from_config(None, None, &process_env)`. CHANGELOG `### Added`, `### Changed` and `### Breaking Changes` entries.
- **Tests:** 9 new in `config/llm.rs`, 31 in `layers.rs`, 29 in `bridge.rs`, 17 in `provider_factory.rs` (86 new; `paladin-llm` all-features lib 954 passed).

## Task Commits

1. **Task 1 RED: failing tests for the block knobs, redacting Debug and empty-key validation** - `e9b51042` (test; scaffold fields compile, two tests fail)
2. **Task 1 GREEN: LlmProviderConfig redacts its key and may leave it to the environment** - `c18a83a8` (feat)
3. **Task 2: D-13 layering and per-provider from_layers** - `a868b517` (feat)
4. **Task 3: config-driven provider construction through the bridges** - `6b6c5f22` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 954 passed. `cargo test --workspace --lib --bins --no-fail-fast`: 14 binaries, 4924 passed, 2 failed -- exactly the two ledgered baseline failures (`build_run_api_persists_no_run_traces_by_default`, `..._persists_run_traces_when_trace_persist_is_set`, WINDOWS.md row 73); the orchestrator baseline was 4868 passed, so the delta is this plan's new default-feature tests.
- `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored (later plans). `cargo test --test unit llm::`: 37 passed. `cargo test --lib config_bridge --features llm-all`: 6 passed.
- `cargo fmt --check` clean. `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean; `cargo clippy -p paladin-llm --lib -- -D warnings` clean for `--no-default-features` and each of the nine adapter features alone (this caught three feature-gating gaps, fixed). `cargo test -p paladin-llm --no-default-features --features <openai-compatible|kimi|anthropic|openai> --lib` compile and the layering tests run; the only warnings are the pre-existing `PinnedSpread` / `pin_waiter_spread` / `ProfiledStubPort` dead-code ones.
- `make check-consumer-surface`, `make check-api-examples` (105 entry points), `make check-migration-allowlist`, `make api-surface` (unchanged, 4275 items), `scripts/check-changelogs.sh`: pass. `make clean-code` was not run end to end (shellcheck is not installed); its fmt and clippy parts were run directly.
- Acceptance greps: `fn resolve_secret_placeholder` 1, `_from_layers` in `bridge.rs` 52 (at least 9), `claude-3-5-sonnet-20241022` 0, `"deepseek-chat"` 0, `set_var` 0 in both `layers.rs` and `bridge.rs`, `pub fn create_from_config` 1, `pub fn is_configured` 1, `fn with_models` 1.
- **Mutation evidence (tests fail when the behaviour is removed).** Swapping block/env order for `base_url` and dropping the block's timeout made 10 layering and bridge tests fail; passing `None` instead of the block in `create_from_config_with_env` made 13 factory tests fail; changing Kimi's `Policy::none()` to `Policy::limited(5)` made `factory_built_adapter_refuses_redirects` fail (all reverted).
- **Manual credential-handling review (D-00a).** (1) Placeholders: only an exact `${[A-Z0-9_]+}` resolves; an unset or blank variable is `MissingEnv`, a construction error naming the variable, and the literal placeholder text is never used as a key on any path (the lenient `From` path yields an empty key which the adapter refuses). (2) `Debug`: `LlmProviderConfig`, `SecretSource` and `Layered` print `<redacted>`; each has a test asserting the literal is absent. (3) Errors: every message built here names provider, model and a variable, never a value; tests assert a block key, an env key and a numeric env var holding a key-shaped string are absent from every error, and a refused model id is echoed escaped and capped at 80 characters. (4) Redirects: no HTTP client, header or redirect policy was touched; `factory_built_adapter_refuses_redirects` proves `Policy::none()` holds on an adapter the factory built, for all compiled providers. (5) Outward serialisation: `Settings` derives `Serialize` and holds `llm: Option<LlmConfig>`, but `grep` for `serde_json::to_*`, `serde_yaml::to_*`, `toml::to_*`, `Json(`, `Config::try_from` and `to_string(&settings)` across `src`, `crates`, `tests`, `examples` and `benches` found no production path that serialises `Settings` or `LlmConfig` (only `serde_json::to_string` of other config structs inside tests), so `api_key` was left serialisable as the plan allows. (6) The configured `base_url` still decides where a credential goes; unchanged and covered by T-50-09-01 above and the module-level trust-boundary docs.
- **Threat model.** T-50-09-01 mitigated (redirect test on factory-built adapters, 307 plus both methods on the target); -02 mitigated (exact placeholders, unset variable is an error, tests); -03 mitigated (redacting `Debug`, names-only errors, tests); -04 mitigated (`validate_model_id` before construction, tested for `..`, `?`, `#`, `%`, leading `/`, whitespace, control characters, empty and 257 characters; Gemini's own `validate_model_identifier` is untouched); -05 mitigated (block beats env per field, tested for every field with an env var, plus the end-to-end mock-vendor check).

## Decisions Made

See `key-decisions`. Flagged assumptions: A1 applied (empty key allowed, missing-key failure moved to construction and `is_configured`; the pinned empty-key test flipped to `validate_accepts_an_empty_key_filled_from_env`), A2 applied (option-a shape on `LlmProviderConfig`), A3 applied (OpenAI's `model` only selects the profile for the timeout default and labels errors; its compiled timeout is already 300 s, so a reasoning model changes nothing there).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The planned redirect check does not detect a followed redirect**
- **Found during:** Task 3, mutation check of `factory_built_adapter_refuses_redirects`
- **Issue:** the plan says to reuse `conformance::redirect_is_not_followed_with_a_credential_header`. That case answers `302` and puts a `POST` mock at `expect(0)` on the target. reqwest turns a followed `302` `POST` into a `GET`, which that mock never matches, so the case passes even when redirects are followed: with Kimi's policy mutated to `Policy::limited(5)` the 302 version of my test still passed.
- **Fix:** the new test answers `307` (method and headers preserved) and gives the target both a `GET` and a `POST` mock at `expect(0)`; the same mutation now fails it. The conformance case itself is unchanged (out of scope, and plan 50-15 owns the suite).
- **Files modified:** `crates/paladin-llm/src/provider_factory.rs`
- **Commit:** `6b6c5f22`

**2. [Rule 3 - Blocking] Three files outside the plan's list were touched**
- **Issue:** `RedactedKey` (`redaction.rs`) was gated on the eight key-carrying features but `LlmProviderConfig` exists in every build; `REASONING_DEFAULT_TIMEOUT_SECONDS` (`registry/mod.rs`) was gated on eight features but `layers.rs` is always compiled; the OpenAI and Anthropic default `max_tokens` were private literals the layering needed, and the generic provider's capabilities had no injectable environment.
- **Fix:** removed the two feature gates; made `OPENAI_DEFAULT_MAX_TOKENS` `pub(crate)`; added `pub(crate) ANTHROPIC_DEFAULT_MAX_TOKENS` (used by `from_env` too); added `OpenAiCompatibleCapabilitiesConfig::from_lookup` (`from_env` now calls it). No public signature changed.
- **Files modified:** `crates/paladin-llm/src/redaction.rs`, `registry/mod.rs`, `openai/adapter.rs`, `anthropic/adapter.rs`, `openai_compatible/adapter.rs`
- **Commits:** `c18a83a8`, `a868b517`

**3. [Rule 1 - existing tests encoded the stale defaults] Two root-crate bridge assertions updated**
- `test_anthropic_config_from_provider_config_defaults` asserted the October-2024 Sonnet id and 300 s; `test_deepseek_config_from_provider_config_defaults` asserted `deepseek-chat` and 60 s. Both now assert the adapters' default-model constants (Anthropic 60 s; DeepSeek 300 s as a seeded reasoning model), with a comment saying why. The plan asks for exactly this replacement.
- **Files modified:** `src/infrastructure/adapters/llm/config_bridge.rs`
- **Commit:** `a868b517`

**4. [Plan acceptance not satisfiable as written] `.project/current-exports.txt` is unchanged**
- `make api-surface` reports the surface unchanged (4275 items) and `grep -c 'create_from_config' .project/current-exports.txt` prints 0, not at least 1. The baseline tracks only root-crate re-exports by item (it lists `pub use paladin::LlmProviderFactory` and `pub use paladin::config::LlmProviderConfig` but not their methods or fields), the same limitation plans 50-03, 50-06, 50-07 and 50-08 recorded. `make api-surface-update` was run and rewrote only the generation timestamp, which was reverted. The `paladin-llm` surface is measured in plan 50-10. The CHANGELOG entries the plan asks for are written.
- **Files modified:** none

**5. [TDD gate] Tasks 2 and 3 have no separate RED commit**
- Task 1 has `e9b51042` (RED: scaffold fields compile, two tests fail) before `c18a83a8` (GREEN). For Tasks 2 and 3 the new function signatures do not exist before the implementation, so a committed failing state cannot compile without contrived stubs; the layering was written with its tests and the behaviour was proven by the mutation checks listed under Verification.

**6. [Rule 3 - Blocking] Disk exhausted twice**
- The sandbox reached 0 bytes free during the workspace build (a linker failure, not a code failure). Cleared by deleting `target/debug/incremental` and `target/debug/examples` as instructed; no tracked file affected, and a final `cargo clippy --workspace --all-features --all-targets` and the full `--lib --bins` run were repeated afterwards. The `cargo test --workspace --no-run --features llm-all` that 50-08 ran was replaced by the all-targets clippy, which compiles every target without linking.

### Notes, not deviations

- **`From` default behaviour changed on purpose.** Callers of `AnthropicConfig::from(&block)` and `DeepSeekConfig::from(&block)` see the new default models and timeouts, and a `${NAME}` key now yields an empty key (refused at construction) instead of the placeholder text. All three are in the CHANGELOG `### Changed` entry.
- **`models:` keys keep their case** through the `config` crate loader (checked with a throwaway test, removed), so an override for a mixed-case model id matches.
- **`default_temperature` is still read by nothing.** The block field existed before and no adapter config has a temperature knob; unchanged.

## Issues Encountered

None blocking.

## Observations for later plans (not fixed here)

- **The conformance suite's redirect case is weak** (`conformance.rs::cases::redirect_is_not_followed_with_a_credential_header`): see Deviation 1. Plan 50-15 owns the suite; the fix is a `307` and a `GET` mock on the target.
- **`LlmConfig::validate` still requires a `default_provider`'s block to exist.** A deployment whose default provider is configured only through environment variables (no block) still fails validation with "config is not present", although `create_from_config` and `is_configured` now handle it. Plan 50-10 moves the composition roots onto this API and can relax or reword it.
- **A malformed legacy env value is reported only for fields the block leaves unset** (by design: the block wins), so a typo in an overridden variable stays silent.
- **OpenAI sends `max_tokens`, which `gpt-5.4-nano` rejects** (carried forward from 50-06 and 50-08; unchanged).
- Plan 50-10 registers `LlmProviderConfig`'s `#[non_exhaustive]` and the three new fields in the semver measurement, and moves `agent_host` and `facade_provisioner` onto `create_from_config`; plan 50-16 documents the precedence, the placeholder grammar and the 300 s rule on the switching-providers page.

## Known Stubs

None.

## Threat Flags

None. The new public surface (`create_from_config`, `is_configured`) takes a credential from configuration, but it adds no endpoint, auth path, file access or trust boundary beyond those the existing adapters already had; the credential surface shrank (redacting `Debug`, no literal placeholder is ever sent).

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-10 can call `LlmProviderFactory::create_from_config` and `is_configured` from `agent_host` and `facade_provisioner`, replace the matrix's `port_from_config` body with it, re-export the six remaining adapter configs from the facade, and measure the semver register.
- `LlmProviderConfig` and the nine adapter configs are all `#[non_exhaustive]` with builders, so Phase 51's `reasoning_effort` is free.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/config/layers.rs` (created), `config/bridge.rs`, `config/llm.rs`, `provider_factory.rs`, `CHANGELOG.md` all present.
- Commits: `e9b51042`, `c18a83a8`, `a868b517`, `6b6c5f22` present in `git log`; `git diff --diff-filter=D --name-only HEAD~1 HEAD` empty (no deletions).
