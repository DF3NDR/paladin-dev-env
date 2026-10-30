---
phase: 51-adjustable-reasoning-effort
plan: 04
subsystem: llm-adapters
tags: [reasoning-effort, compat-engine, kimi, qwen, grok, ollama, openai-compatible, wire-declaration, semver-register]
status: complete

requires:
  - phase: 51-01
    provides: "ReasoningEffort, EffortSupport::resolve, LlmRequest.reasoning, LlmError::UnsupportedParameter, nine unset-effort golden bodies"
  - phase: 51-03
    provides: "paladin_llm::reasoning (ReasoningWire, admit, mask_effort, WireCaps, SubstitutionLog), RequestNotes reasoning fields, ModelOverride.effort"
provides:
  - "CompatRequestParameters.reasoning: ReasoningWire (compile-enforced, no Default) and all(reasoning)"
  - "Compat engine: exact per-wire mapping (top_level_effort, thinking_object, enable_thinking), wire-masked model_profile, refusal before HTTP via reasoning::admit"
  - "Operator-declared reasoning_wire + with_reasoning_wire on OllamaConfig and OpenAiCompatibleConfig"
  - "MIGRATION 9.2 Y row and allowlist entry for CompatRequestParameters"
affects: [51-06, 51-11, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "one declared wire per preset, stated in every CompatRequestParameters construction site"
    - "engine folds the wire into the reported profile with mask_effort(support, wire_caps(wire))"
    - "map_reasoning fails closed: a (wire, effort) pair without a mapping returns UnsupportedParameter and sends nothing"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/compat/mod.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/ollama/adapter.rs
    - crates/paladin-llm/src/openai_compatible/adapter.rs
    - crates/paladin-llm/src/registry/mod.rs
    - crates/paladin-llm/src/config/bridge.rs
    - crates/paladin-llm/src/test_support.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml

key-decisions:
  - "Starting wires (pre-probe, A1-51-04): Kimi ThinkingObject (handoff-measured), Qwen EnableThinking (documented), Grok Unsupported (field unconfirmed), Ollama and OpenAI-compatible Unsupported until the operator declares one."
  - "Wire capabilities: TopLevelEffort and ThinkingObject carry levels/off/auto and no budget; EnableThinking carries off/auto/budget and no levels; everything else is WireCaps::NONE."
  - "A refused effort is classified Misconfiguration in classify_fetch_failure (explicit arm above the wildcard)."

patterns-established:
  - "Later presets and plan 51-11 restate a preset's wire by editing its request_parameters literal; the kimi_pre_probe_ tests are the ones to rewrite."

requirements-completed: []
requirements-progress: [REASON-02, REASON-03, REASON-04, REASON-07]

duration: about 2h (including a warm worktree build and the tracer checkpoint)
completed: 2026-10-21
---

# Phase 51 Plan 04: Compat engine reasoning wires Summary

**Every Kimi, Qwen, Grok, Ollama and OpenAI-compatible preset now states a non-bool `ReasoningWire` at compile time; the shared engine sends an effort in exactly that field or refuses it with `UnsupportedParameter` before any HTTP call, and self-hosted endpoints take an operator-declared wire.**

REASON-02, -03, -04 and -07 advance for these five presets but are not marked complete: the configuration surface, builder, docs and probe-confirmed positions land in plans 51-06 to 51-14, and no model is seeded with effort data until 51-10 to 51-12. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: every preset states a reasoning wire at compile time; a Kimi effort reaches the wire as `thinking.effort` or is refused before HTTP | `31b553a0` |
| 2 | The other three wires mapped exactly (top-level effort, `enable_thinking` with a bounded budget, `Unsupported`) on both paths | `075f227c` |
| 3 | Operator-declared wires for Ollama and OpenAI-compatible (D-10), CHANGELOG entries | `1cc5513e` |

The Task 1 tracer was verified by the user at the interactive checkpoint ("approved") before Tasks 2 and 3 began.

## What was built

- **`CompatRequestParameters`** gains `pub reasoning: ReasoningWire`; `all()` is `pub const fn all(reasoning: ReasoningWire)` (doc test). Still no `Default`; no `..CompatRequestParameters` spread anywhere. `compat` re-exports `ReasoningWire`.
- **`CompatRequest`** gains four skip-if-`None` fields: `reasoning_effort`, `thinking` (`CompatThinking { type, effort? }`), `enable_thinking`, `thinking_budget`. Added last, so an unset request body is byte-identical (all nine goldens pass).
- **Engine**: private `wire_caps(wire)`; `CompatEngine::model_profile` masks `effort` with it; `build_request` runs `reasoning::admit` (after the max-tokens check, never through `sampling::apply` or `OmissionLog`) and a private `map_reasoning`:
  - `TopLevelEffort`: level -> `reasoning_effort: "L"`, `Off` -> `"none"`, `Auto` -> nothing (still stamped).
  - `ThinkingObject`: level -> `thinking: {type: "enabled", effort: "L"}`, `Off` -> `{type: "disabled"}`, `Auto` -> `{type: "enabled"}`.
  - `EnableThinking`: `Off` -> `enable_thinking: false`, `Auto` -> `true`, `Budget(n)` -> `true` + `thinking_budget: n`.
  - `Unsupported` and any unmapped pair: refused, nothing sent.
  - `RequestNotes.reasoning_requested` / `reasoning_effective` are stamped, so buffered responses and the stream's terminal chunk carry both metadata keys.
- **`classify_fetch_failure`** has an explicit `UnsupportedParameter` arm (`Misconfiguration`) above the wildcard.
- **Presets**: Kimi `ThinkingObject`, Qwen `EnableThinking`, Grok `Unsupported`, Ollama and OpenAI-compatible `CompatRequestParameters::all(config.reasoning_wire)` (default `Unsupported`). Each position carries a rustdoc/comment citing its source.
- **D-10**: `OllamaConfig.reasoning_wire` and `OpenAiCompatibleConfig.reasoning_wire` (default `Unsupported`, set in every constructor, `from_env` path and the two `config/bridge.rs` literals), `with_reasoning_wire` (doc tests), shown in each `Debug`. No network call at construction. `KimiConfig`, `QwenConfig`, `GrokConfig` have no such knob.
- **Registry**: MIGRATION 9.2 `Y` row `paladin-llm | CompatRequestParameters` and one allowlist entry (set-equal, `make check-migration-allowlist` exits 0).
- **CHANGELOGs**: root and `paladin-llm` carry an `### Added` entry (compat mapping, `with_reasoning_wire`) and a `### Changed` BREAKING entry with the one-line migration.

## TDD record

- **Task 1:** the mapping and preset positions were written before the Kimi tests (order inversion, noted honestly); the tests were then written and pass. Five Kimi tests are named `kimi_pre_probe_*` and carry `// Pre-probe position (A1-51-04): superseded by plan 51-11.`: wire position, `High` mapping with no top-level `reasoning_effort`, `Off`/`Auto`, the `kimi-k2.6` refusal naming `llm.kimi.models.kimi-k2.6.effort` with zero HTTP hits, and the masked profile.
- **Task 2:** the `reasoning_wires` engine module (15 tests), a Qwen module (2) and a Grok test cover every `<behavior>` case: per-wire mapping, bounded budgets (the bound comes from the profile, not a constant), all nine values refused on `Unsupported` with zero hits, the cross-wire field-absence test, the streaming path (same fields, only the terminal chunk stamped, refusal before HTTP) and the `classify_fetch_failure` arm. The first run of the two budget tests failed for a useful reason: the synthetic request carries a 1000-token cap, so a 2048-token budget is (correctly) refused by the budget-below-`max_tokens` rule. The tests now use 512 and a separate test pins the refusal.
- **Task 3:** default/builder/`Debug` tests for both configs, an Ollama `ThinkingObject` request, and the three OpenAI-compatible cases (declared wire plus override sends `reasoning_effort: "high"`; default wire refuses naming `llm.openai-compatible.reasoning_wire`; wire set without an override refuses naming `llm.openai-compatible.models.m.effort`).

## Verification

- `cargo test --offline -p paladin-llm --all-features --lib`: 1343 passed. `--doc`: 34 passed. The nine `unset_reasoning_leaves_the_wire_body_unchanged` goldens pass.
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `cargo test --offline --workspace --all-features --no-run` compiles.
- `make check-migration-allowlist`, `make check-gates`, `make check-api-examples` exit 0. `make api-surface` exits 0 with "API surface unchanged": the baseline tracks the `paladin` facade, which does not re-export `paladin_llm::compat` or `paladin_llm::reasoning`, so `.project/current-exports.txt` needed no change (same finding as plan 51-03).
- **cargo-semver-checks: not run** (`cargo semver-checks --version` fails; the tool is absent in this container). No `Cargo.toml` lint table was touched, so there was nothing to restore. The allowlist entry's lint is `register-only (unmeasured: cargo-semver-checks absent on 2026-10-21; measured in plan 51-14)`; the lints the tool will report for the type are not yet known.
- Not run: the full `make clean-code` chain and `make security` (no dependency added); `cargo-llvm-cov` is absent locally.
- Manual credential-handling review (security.instructions.md): `reasoning_wire` is a closed enum, not a credential; `OpenAiCompatibleConfig`'s hand-written `Debug` still prints the key as `<redacted>` and now also prints `reasoning_wire`; `OllamaConfig` has no key. Every compat client keeps `redirect::Policy::none()` (untouched). The refusal text names provider, model, effort and the override key only, never a prompt, key or response body.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality / prohibition] MIGRATION row and allowlist entry registered in the Task 1 commit, not Task 3**
- **Found during:** Task 1 commit.
- **Issue:** The plan's prohibition (REASON-07) says `CompatRequestParameters` must not change without its MIGRATION 9.2 row and allowlist entry in the same commit, but the plan scheduled the registration in Task 3, two commits later.
- **Fix:** Added the `Y` row and the allowlist entry in `31b553a0`, with the field. Task 3 only re-confirmed them (`make check-migration-allowlist`). Accepted by the user at the tracer checkpoint.
- **Commit:** `31b553a0`

**2. [Rule 3 - Blocking] Test helpers added to `crates/paladin-llm/src/test_support.rs`**
- `plain_request` and `send_capturing` (test-only) were added so the five presets' tests share one capture helper; the file is outside `files_modified`.
- **Commit:** `31b553a0`

### Notes against the plan's wording

- **Qwen budget example:** the plan's behavior used `Budget(2048)`; the synthetic request carries a 1000-token output cap, so the adapter tests use `Budget(512)`, and a separate engine test pins that a budget not below `max_tokens` is refused before HTTP.
- **Grok refusal wording:** the plan says the reason should say the field is "not verified for `grok`". `admit`'s shared `WireUnsupported` text says "not verified for this provider in this release"; the error's `provider` field carries `grok`, and the test asserts both. `reasoning.rs` was not changed.
- **`.project/current-exports.txt`** is unchanged (api-surface unchanged, see Verification).
- **Commit trailer:** the system attribution reminder (`Claude Sonnet 5.5`) was used; commits used `--no-verify` as the wave allows.

## Authentication Gates

None.

## Known Stubs

None. The deliberate limitations: no model carries seeded effort data until plans 51-10 to 51-12, so only a `ModelOverride.effort` unlocks an explicit effort for now; Kimi, Qwen and Grok wire positions are pre-probe and plan 51-11 restates them (A1-51-04). Plan 51-06 wires the `llm.<provider>.reasoning_wire` YAML key; the two `config/bridge.rs` literals set `Unsupported` until then.

## Threat Flags

None. No new endpoint, auth path or file access. T-51-13 (a preset sending a field its vendor ignores) is mitigated by the compile-enforced declaration, exact mapping and the cross-field absence test; T-51-14 by `ReasoningWire` being a closed enum; T-51-15 by the profile-supplied budget bounds and the budget-below-`max_tokens` rule; T-51-16 is accepted (redirects remain disabled).

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present; `git status` clean before this file.
- Commits `31b553a0`, `075f227c`, `1cc5513e` present in `git log` on `worktree-agent-a4c89143f9a16e2a3`.
