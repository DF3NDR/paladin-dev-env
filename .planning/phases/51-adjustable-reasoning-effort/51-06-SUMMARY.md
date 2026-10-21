---
phase: 51-adjustable-reasoning-effort
plan: 06
subsystem: llm-adapters
tags: [reasoning-effort, provider-default, config, yaml, reasoning-wire, config-knob-builders]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort/ReasoningFallback, LlmRequest.reasoning, nine unset-effort golden bodies"
  - phase: 51-03
    provides: "paladin_llm::reasoning (ReasoningWire, admit, WireCaps, SubstitutionLog), DeepSeek and OpenAI mappings"
  - phase: 51-04
    provides: "compat engine wire mapping, OllamaConfig/OpenAiCompatibleConfig.reasoning_wire"
  - phase: 51-05
    provides: "Anthropic and Gemini mappings"
provides:
  - "reasoning_effort / reasoning_fallback fields and with_* builders on all nine *Config types (one macro arm)"
  - "llm.<provider>.reasoning_effort / reasoning_fallback / reasoning_wire on LlmProviderConfig, layered and bridged"
  - "reasoning::in_force: request-over-provider-default resolution per request; admit names the config key on a refused default"
  - "CompatEngine::with_default_reasoning, called by the five presets"
  - "D-10: self-hosted reasoning_wire from YAML; named vendors refuse it"
affects: [51-07, 51-08, 51-11, 51-12, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "provider default resolved per request with reasoning::in_force, never at construction"
    - "a refused default ends with (set by `llm.<provider>.reasoning_effort`) via admit's source_key"
    - "reasoning keys are block-only in resolve: no environment variable, no compiled default"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/reasoning.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/config/llm.rs
    - crates/paladin-llm/src/config/layers.rs
    - crates/paladin-llm/src/config/bridge.rs
    - crates/paladin-llm/src/provider_factory.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - tests/consumer_surface_muster.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md

key-decisions:
  - "Precedence is resolved field by field per request: request.reasoning.or(config default) and request.reasoning_fallback.or(config fallback). The source key is attached only when the effort itself came from configuration."
  - "reasoning_wire is checked in resolve (strict path) against ollama / openai-compatible; the lenient From conversions ignore it on a named vendor, as they do every other problem."
  - "No per-model default (A1-51-06) and no node-cache key input for provider defaults (A2-51-06): both stay as flagged."

patterns-established:
  - "Later plans (51-07 builder pre-flight, 51-08 agent build-time check) read the provider default off the *Config / LlmProviderConfig fields."

requirements-completed: []
requirements-progress: [REASON-01, REASON-02, REASON-04]

duration: about 3h (cold worktree build, tracer checkpoint round trip)
completed: 2026-10-21
---

# Phase 51 Plan 06: Provider-level reasoning default Summary

**`llm.<provider>.reasoning_effort` / `reasoning_fallback` and `with_reasoning_effort` / `with_reasoning_fallback` on all nine configs set a provider default that every adapter applies per request below the request's own effort; self-hosted endpoints declare their wire with `llm.<provider>.reasoning_wire` and named vendors refuse it.**

REASON-01, -02 and -04 advance but are not marked complete: the PaladinBuilder and agent levels, the docs and the probe-seeded model data land in plans 51-07 to 51-14. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: `llm.deepseek.reasoning_effort: high` in YAML reaches DeepSeek's wire for a request that sets none, below a request that sets one | `97b7cc7b` |
| 2 | The other eight adapters apply the provider default, self-hosted `reasoning_wire` from YAML, named vendors refuse it, surface record | `5edf3c7f` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("approved") before Task 2 began; the orchestrator re-ran the evidence on `97b7cc7b` (2 factory tests, `registry::` 73, `config::` 115, goldens 9; clippy and fmt clean).

## What was built

- **Nine configs.** `pub reasoning_effort: Option<ReasoningEffort>` and `pub reasoning_fallback: Option<ReasoningFallback>` with rustdoc on `OpenAIConfig`, `AnthropicConfig`, `DeepSeekConfig`, `GeminiConfig`, `KimiConfig`, `QwenConfig`, `GrokConfig`, `OllamaConfig`, `OpenAiCompatibleConfig`. `None` in every constructor, `from_env`/`from_parts` and bridge literal (and the OpenAI test literals); shown in each hand-written `Debug` (Ollama derives `Debug`; it has no key). The two builders are generated once in `config_knob_builders!`'s `@common` arm.
- **`LlmProviderConfig`.** `reasoning_effort`, `reasoning_fallback`, `reasoning_wire: Option<ReasoningWire>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), `with_*` builders (doc test), `Debug`, `new`. `Layered` carries the three, filled from the block only; each bridge fn copies effort and fallback; the Ollama and OpenAI-compatible bridge fns copy the wire (default `Unsupported`).
- **Precedence.** `reasoning::in_force(provider, request_effort, request_fallback, default_effort, default_fallback)` returns the effort, fallback and an optional `llm.<provider>.reasoning_effort` source key. `admit` gained `source_key: Option<&str>`; a refusal then ends with "(set by `<key>`)". OpenAI, Anthropic, DeepSeek, Gemini and the compat engine call it per request against the request's own model. `CompatEngine::with_default_reasoning` is called by the five presets.
- **D-10.** `resolve` refuses `reasoning_wire` for any provider other than `ollama` / `openai-compatible`, with a message naming `llm.<provider>.reasoning_wire` and saying named vendors change their wire only through a Paladin release.
- **Surface record.** Muster fn `phase_51_provider_defaults` pins the builders on `DeepSeekConfig`, `AnthropicConfig` and `LlmProviderConfig` (including `with_reasoning_wire`) and the public fields. CHANGELOG entries in the root and `paladin-llm`: all nine adapters honour `LlmRequest.reasoning` exactly or refuse before HTTP, plus the new keys.

## TDD record

- **Task 1:** the implementation was written before the tests (order inversion), so there was no RED run. Sensitivity was checked by mutation: flipping `in_force` to `default.or(request)` made `deepseek_applies_the_provider_default_below_the_request` fail; reverted.
- **Task 2:** the D-10 resolve check already existed from Task 1's `layers.rs` work, so the named-vendor refusal tests and the other adapter default tests passed at once; they are regression guards, not RED tests. New tests: `provider_factory::tests::from_config::reasoning_defaults` (13 tests, through `create_from_config_with_env` to a mock vendor, covering request-wins, same-level, neither-set, and a refused default naming the key), two `config::layers` tests (block-only defaults; wire refused outside the self-hosted providers), muster pins.
- Kimi's case is named `kimi_pre_probe_applies_the_provider_default_as_thinking_effort` and carries the pre-probe comment for plan 51-11; Grok (declared `Unsupported`) asserts its default refuses before HTTP.

## Verification

- `cargo test --offline -p paladin-llm --all-features --lib`: 1408 passed (nine goldens included). `--doc`: 35 passed. Muster (`--features llm-anthropic,llm-deepseek`): passed.
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; also clean with `--no-default-features` and with only `ollama` / only `deepseek`. `cargo fmt --all` clean. `cargo test --offline --workspace --all-features --no-run` compiles.
- `make api-surface` exits 0 ("API surface unchanged", 4315 items); `make api-surface-update` changed only the generated-at timestamp, which I reverted, so `.project/current-exports.txt` is unchanged (the `paladin` facade does not re-export these items; same finding as plans 51-03 and 51-04). `make check-gates`, `make check-api-examples`, `make check-consumer-surface` exit 0.
- Not run: the full `make clean-code` chain and `make security` (no dependency added); `cargo-semver-checks` is absent in this container. No register row: every touched type is `#[non_exhaustive]` and the change is additive.
- Manual credential-handling review (security.instructions.md): every hand-written `Debug` keeps `RedactedKey` and the nine-type test asserts the key never appears; `LlmProviderConfig::api_key` is still `skip_serializing` and its `Debug` still redacts (tested); the new refusal text is built from provider, model, effort and config key names only; no client, header or redirect policy was touched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `with_default_reasoning` wired into the five presets in Task 1, not Task 2**
- **Found during:** Task 1 clippy. `CompatEngine::with_default_reasoning` was unused until the presets called it, which fails `-D warnings`.
- **Fix:** called it from the five presets' constructors in the Task 1 commit. Task 2 added the per-adapter tests. Accepted at the tracer checkpoint.
- **Commit:** `97b7cc7b`

**2. [Rule 3 - Blocking] `#[allow(clippy::too_many_arguments)]` on `admit`**
- **Found during:** Task 1 clippy. Adding `source_key` makes eight arguments. I kept the plan's signature and added a commented allow rather than bundling the inputs. Accepted at the checkpoint.
- **Commit:** `97b7cc7b`

**3. Order inversion in Task 1 (no RED run)** with the mutation check as sensitivity evidence, as above. Accepted at the checkpoint.

### Notes against the plan's wording

- **`.project/current-exports.txt` is unchanged** (see Verification); the plan listed it in `files_modified`.
- **`ReasoningWire` is not re-exported by the `paladin` facade.** The muster names it as `paladin_llm::reasoning::ReasoningWire`, so a consumer using `LlmProviderConfig::with_reasoning_wire` needs a direct `paladin-llm` dependency. Not changed here; flagged for plan 51-13/51-14.
- **Field types** in the struct declarations are the short `ReasoningEffort` / `ReasoningFallback` (imports added), so the plan's acceptance grep prints `10`.
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used; commits used `--no-verify` as the wave allows.

## Authentication Gates

None.

## Known Stubs

None. Deliberate limitations: A1-51-06 (no per-model default; docs in 51-13 say to set a provider default only where every served model supports it) and A2-51-06 (a provider default is not part of the node-cache key; a changed provider default needs a new cache namespace). Kimi, Qwen and Grok wire positions stay pre-probe until plan 51-11.

## Threat Flags

None. T-51-20 (named vendor wire overridden from config) is mitigated and tested; T-51-21 is accepted (the refusal names the key); T-51-22 (a `Debug` printing the key) is mitigated and tested.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present; `git status` clean before this file.
- Commits `97b7cc7b` and `5edf3c7f` present in `git log` on `worktree-agent-a9275bfd1f0124f1d`.
