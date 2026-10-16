---
phase: 50-provider-parity-config-driven-model-swap
verified: 2026-10-16T23:00:00Z
status: human_needed
score: 6/6 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps: []
deferred: []
human_verification:
  - test: "CI `coverage` job (cargo llvm-cov --fail-under-lines, 82 % workspace floor, ADR-0006)"
    expected: "Job green on the Phase 50 head"
    why_human: "cargo-llvm-cov is not installed in this environment; no percentage can be measured here"
  - test: "CI `test-integration-docker` job (Docker-bound integration suites)"
    expected: "All Docker-backed suites green"
    why_human: "Docker is absent here; those suites cannot run"
  - test: "mdBook build of docs/src (Switching Providers page renders, links resolve)"
    expected: "`mdbook build` exits 0; the page is reachable from the SUMMARY nav and its table renders"
    why_human: "mdbook is not installed; only grep-level link checks and the Rust registry drift test were run"
  - test: "Operator credential-gated smoke run: cargo run -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible"
    expected: "Every configured provider PASS; unconfigured ones print SKIPPED; non-zero exit if nothing configured"
    why_human: "Needs real vendor credentials. The harness and its --probe/--dry-run modes exist; the 2026-10-15 probe run is on record in 50-SEED-PROBE.md, but the plain smoke mode against the finished tree has no run on record"
  - test: "Owner decision on open ledger rows WINDOWS 74, 75, 76"
    expected: "74: vendor total_tokens vs prompt+completion; 75: OpenAI max_tokens vs max_completion_tokens for gpt-5.4-nano; 76: Kimi adapter-wide fallback profile vs the 2026-10-15 probe"
    why_human: "Accepted, ledgered deviations; each needs an owner call that code cannot make"
---

# Phase 50: Provider Parity & Configuration-Driven Model Swap Verification Report

**Phase Goal:** Any Paladin agent can be moved between the nine LLM providers by changing configuration only: equivalent behaviour, or an early build failure naming provider, model and parameter. Library-level APIs matter as much as server config.
**Verified:** 2026-10-16
**Status:** human_needed (no gaps; five CI/operator/owner items that cannot be run or decided here)
**Re-verification:** No, initial verification (no prior VERIFICATION.md)

## Goal Achievement

Verified against the code on HEAD `7dae1443`. SUMMARY claims were treated as leads, not evidence. I ran the tests named below myself.

### Observable Truths (ROADMAP success criteria are the contract)

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | Provider-swap matrix: same agent, nine providers, config-only switch, wire body and outcome asserted; covers manual and auto temperature on a (1,1) provider, user prompt on the wire, per-step fallback model, empty `stop` completion; was red before the fixes | VERIFIED | `cargo test --test provider_swap_matrix --features llm-all`: 50 passed, 0 failed, 0 ignored. The only `#[ignore]` text left in `tests/provider_swap_matrix.rs` is a doc comment. All nine providers have prompt/context/model/empty-stop/same-agent rows. Kimi has `kimi_fixed_temperature_explicit` (refuses 1.0, 0.5 and 1.0+1ulp, naming provider, model and temperature) and `kimi_fixed_temperature_auto` (builds, executes, no `temperature` key in any recorded body, `paladin.params_omitted` lists it). `fallback_hop_gets_its_own_model` and `config_fallback_steps_send_each_hop_its_own_model` pass. Ports are built through the facade's `LlmConfig` plus `create_from_config`. Red-before-fix is recorded in `50-MATRIX-RED-EVIDENCE.md`: 19 red cases, plus the pre-fix OpenAI body `{"content":"","role":"user"}`. |
| 2 | OpenAI sends `query` (plus `context` when present) on the execution-service path with a regression test red on the old tree; parameter support is per provider and per model (`Range`/`Fixed`/`NotSent`) under one ADR amending ADR-0004; Kimi swap with default `auto_temperature` builds, explicit out-of-range refused, nothing substituted undiscoverably | VERIFIED | `crates/paladin-llm/src/turn.rs::user_turn_text` is called by all five request builders (openai:665, anthropic:407, deepseek:646, gemini:524, compat/engine:675). The old `context.unwrap_or_default()` build of the user turn is gone. `openai_user_turn_carries_query_on_the_execution_service_path` exists, and its pre-fix failure is recorded. `crates/paladin-ports/src/output/model_profile.rs` (807 lines) holds `ModelProfile` (fail-closed `new()`), `ParamSupport::{Range,Fixed,NotSent}`, `StopSupport` and `SamplingParam`. `LlmPort::model_profile` defaults to `None`. `PaladinBuilder` (`paladin_builder.rs:1185-1245`) consults the profile first. It refuses an explicit temperature or stop set by name (provider, model, parameter) and omits an auto value. The root-crate builder tests passed (`not_sent_temperature_refuses_naming_the_parameter`, `fixed_profile_*`, `auto_temperature_outside_profile_is_omitted_not_refused`, `stop_words_the_profile_does_not_take_are_refused_by_name`). ADR-0061 is Accepted, states "Amends ADR-0004", and ADR-0004 carries the back-reference. No clamping anywhere (`sampling.rs` admits or omits). |
| 3 | `llm.<provider>` blocks honoured through a public library API routed through `config/bridge.rs`; per-agent provider+model resolve the same way in `agent_host`; existing YAML loads unchanged; 300 s reasoning default; facade exports all nine adapter/config pairs behind ADR-0046 features; `api-surface` refreshed plus CHANGELOG | VERIFIED | `LlmProviderFactory::create_from_config` and `is_configured` are public (`provider_factory.rs:494,555`). Every provider row's `construct_from_config` calls a `config::bridge::*_from_layers` function (9 of 9). `agent_host.rs:284,423` and `facade_provisioner.rs:390` call `create_from_config`; there is no `.create(&provider)` call left. `LlmProviderConfig` has `base_url`, `timeout_seconds`, `max_retries`, `max_tokens`, `context_window` and `models`. Tests `existing_yaml_loads_unchanged` and `example_config_llm_section_loads_unchanged` pass (`config::layers::` and the 261-test targeted run are green). The 300 s default is in `layers.rs` with a test. `src/lib.rs:28-60` exports all nine pairs behind `llm-*` features. `Cargo.toml:543` default is still `openai, anthropic, deepseek`. `LlmConfig`/`LlmProviderConfig` are exported. CHANGELOG has PARITY-03 entries (lines 599-636). `.project/current-exports.txt` contains the new items. All nine configs are `#[non_exhaustive]` and share the `config_knob_builders!` builders (owner option-a honoured). |
| 4 | Dated per-model registry with config overrides; Commissary and `resolve_context_window` read it; kimi-k3 rationed at 1,048,576; stale Anthropic/DeepSeek defaults and model lists replaced | VERIFIED | `registry/seeded.rs` has 14 model ids, all dated 2026-10-15. A script confirmed every id and date occurs in `50-SEED-PROBE.md`. `kimi-k3` is seeded at 1,048,576. Precedence is override > `context_window` knob > seed > visible fallback (ADR-0061 (f), 54 `registry::` tests green). `window::effective_capabilities` is read by `HistoryTrimmer` (production, `history.rs:103`) and by `Commissary::for_model`. `Commissary::for_model` rations kimi-k3 at the real window (test at `commissary.rs:1388`, green). Anthropic's `validate_model`/`get_available_models` come from the seeded table. `ANTHROPIC_DEFAULT_MODEL` is `claude-haiku-5-5`, DeepSeek's is `deepseek-flash`. The only remaining `claude-3-5-sonnet-20241022` strings are test fixtures and an `unmeasured_and_retired_ids_are_not_seeded` negative test. |
| 5 | Every adapter maps empty output (any stop reason), typed `Refusal` (Permanent), usage cap, cache read/write, reasoning tokens and text, stop sequences; fallback chains send each hop its own model, key Cadence on it, record the served model | VERIFIED (see notes) | `LlmError::Refusal` exists, with a `Permanent` arm at `llm_port.rs:694`. The single `response_guards::classify_completion` (refusal, then content-filter/function-call exemption, then `EmptyCompletion` on any other finish reason) is called by openai, deepseek, gemini and the compat engine (buffered and streamed via `StreamCompletion`). Anthropic has its own refusal-before-empty path. Stop sequences reach the wire: `stop: applied.stop` (openai, deepseek, compat), `stop_sequences` (anthropic), `stopSequences` (gemini). Reasoning text is typed on `LlmResponse.reasoning` and `StreamingResponse.reasoning` for the compat presets, DeepSeek and Anthropic thinking blocks, which matches D-11's "every adapter that receives it". Cache write and reasoning tokens are mapped in all five code paths. Fallback: `FallbackStep`, `from_steps`, `request_for_hop` rewrites the model per hop. `with_cadence` wraps each hop in Cadence, which keys on `request.model`. `paladin.served_model` is stamped (buffered and streamed). `hop_on_refusal` is opt-in. `agent_runtime.model_fallback.steps` is wired. Owner R-1 is honoured: `PaladinResult.run_metadata` (`execution_result.rs:148`), `merge_run_metadata` in the execution service, `StreamingResponse.metadata` and `ChunkMetadata.run_metadata`. Owner R-2 is honoured: no usage-cap `error_override` was encoded for any compat preset, and all five are recorded as unverified in ADR-0061 and on the page. |
| 6 | All nine adapters in the conformance suite (capability truthfulness, `response_format` wire shape, empty completion, refusal, usage fields, parameter-drop policy) and the credential-gated live smoke; mdBook "Switching providers" page with the per-provider/per-model table; MIGRATION/CHANGELOG carry every break; gates stay green; no `unwrap`/`expect`/`panic!` added to library code | VERIFIED locally; CI items below | `llm_conformance_suite!` is instantiated in all nine adapter modules. `cargo test -p paladin-llm --all-features --lib conformance`: 164 passed, 0 failed, including `capability_declarations_match_the_wire`, `response_format_reaches_the_wire`, `refusal_is_typed_and_permanent`, `usage_fields_are_mapped` and `unsupported_parameters_are_omitted_and_recorded` on every adapter. `examples/live_vendor_smoke.rs` is table-driven over nine providers with a credential gate, `--probe`, `--dry-run`, and a refusal to exit 0 when nothing was tested. `docs/src/user-guides/switching-providers.md` (361 lines) is in `docs/src/SUMMARY.md:31`. It has the per-provider/per-model table and a response-mapping table, and `switching_providers_page_lists_every_seeded_model` is green. CHANGELOG has Phase 50 entries (30 mentions) and MIGRATION.md has Phase 50 rows (32 mentions). `make check-migration-allowlist` and `make check-gates` exit 0 on this run. A scripted scan of added non-test lines across the phase diff found no `unwrap`/`expect`/`panic!`; the only hits were in `crates/paladin-llm/src/test_support.rs`, which is `#[cfg(test)]`-gated. No `TBD`/`FIXME`/`XXX` was added. The 82 % coverage floor, Docker suites and mdBook build are human/CI items. |

**Score:** 6/6 truths verified; 0 present-but-behavior-unverified. The behavioural invariants (refusal, empty-on-any-stop, per-hop model, omission recording) are each exercised by a passing named test, not inferred from symbol presence.

### Required Artifacts

| Artifact | Status | Details |
|---|---|---|
| `crates/paladin-llm/src/turn.rs` | VERIFIED | Substantive, wired in 5 builders |
| `crates/paladin-ports/src/output/model_profile.rs` | VERIFIED | Types plus a documented edge table |
| `crates/paladin-llm/src/sampling.rs` | VERIFIED | Shared filter, `apply_response_format`, `check_max_tokens` |
| `crates/paladin-llm/src/registry/{mod,seeded}.rs` | VERIFIED | Dated, probe-traceable |
| `crates/paladin-llm/src/response_guards.rs` | VERIFIED | One rule, 4 adapters plus Anthropic's own ordering |
| `crates/paladin-llm/src/window.rs` | VERIFIED | `effective_capabilities` wired to `HistoryTrimmer` and `Commissary::for_model` |
| `crates/paladin-llm/src/config/{llm,layers,bridge}.rs`, `provider_factory.rs` | VERIFIED | Wired from `agent_host` and `facade_provisioner` |
| `tests/provider_swap_matrix.rs` | VERIFIED | 50 green, 0 ignored |
| `crates/paladin-llm/examples/live_vendor_smoke.rs` | VERIFIED | Nine providers, credential gated |
| `docs/src/user-guides/switching-providers.md` | VERIFIED | In nav; drift-tested |
| `.planning/decisions/0061-per-model-parameter-support-policy.md` | VERIFIED | Accepted; amends ADR-0004; R-1 and R-2 recorded as answered |

### Key Link Verification

| From | To | Status |
|---|---|---|
| `PaladinBuilder::build` | `LlmPort::model_profile` | WIRED (refusal tests green) |
| Adapter request builders | `sampling::apply` and `user_turn_text` | WIRED |
| `agent_host` / `facade_provisioner` | `LlmProviderFactory::create_from_config` | WIRED |
| `create_from_config` | `bridge::*_from_layers` (9 of 9) | WIRED |
| `FallbackLlmAdapter` hop | Cadence (per-hop wrap) keyed on `request.model` | WIRED |
| `PaladinExecutionService` | `PaladinResult.run_metadata` (R-1 option-b) | WIRED |
| `HistoryTrimmer` | `effective_capabilities` | WIRED |
| Switching Providers page | registry (drift test) | WIRED |

### Behavioral Spot-Checks (run by me)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Nine-provider matrix | `cargo test --test provider_swap_matrix --features llm-all` | 50 passed, 0 ignored | PASS |
| Conformance on nine adapters | `cargo test -p paladin-llm --all-features --lib conformance` | 164 passed | PASS |
| Registry, sampling, fallback, guards, window, layers, bridge, factory, commissary, page drift | `cargo test -p paladin-llm --all-features --lib -- <filters>` | 261 passed | PASS |
| Builder refusal, run metadata, agent_host config-driven, agent_runtime | `cargo test -p paladin-ai --lib -- paladin_builder:: run_metadata config_driven_tests agent_runtime::` | 83 passed | PASS |
| Migration allowlist and publish-order gates | `make check-migration-allowlist`, `make check-gates` | exit 0 | PASS |

I did not re-run the full workspace gate. The orchestrator's HEAD run (5069 passed, 2 failed, both the ledgered row-73 `run_api_wiring` pair) and plan 50-16's full gate (7840 passed, same 2 failed) are taken as recorded.

### Requirements Coverage

All eight PARITY IDs appear in plan frontmatter, and all eight are marked `[x]` and mapped "Phase 50 | Complete" in REQUIREMENTS.md. There are no orphans.

| Requirement | Plans | Status | Evidence |
|---|---|---|---|
| PARITY-01 | 01, 03, 05, 10, 12, 13, 14, 16 | SATISFIED | Truth 1 |
| PARITY-02 | 02, 03, 05, 16, 18 | SATISFIED | Truth 2, ADR-0061 |
| PARITY-03 | 08, 09, 10, 16 | SATISFIED | Truth 3 |
| PARITY-04 | 03, 04, 06, 07, 09, 16, 17, 18 | SATISFIED | Truth 4 |
| PARITY-05 | 03, 04, 05, 11, 12, 13, 16, 17 | SATISFIED | Truth 5 (usage-cap clause: R-2 owner-accepted unverified) |
| PARITY-06 | 14, 16, 18 | SATISFIED | Truth 5 fallback |
| PARITY-07 | 04, 15, 16 | SATISFIED (locally) | Truth 6 |
| PARITY-08 | 01, 16 | SATISFIED | Truth 2 |

### Owner decisions, honoured in code

- 50-08 option-a (`#[non_exhaustive]` plus `with_*`): all nine configs carry `non_exhaustive` and the shared builder macro.
- R-1 option-b: `PaladinResult.run_metadata`, `ChunkMetadata.run_metadata` and the terminal-chunk `StreamingResponse.metadata` exist and are merged by the execution service. The HTTP `ExecuteResponse` is deliberately not extended, as ADR-0061 (h) states.
- R-2: no usage-cap override encoded; every compat vendor labelled "unverified -- owner accepted (2026-10-16)" on the page and in the ADR.

### Ledgered deviations, confirmed recorded (not counted as gaps)

- WINDOWS.md rows 73, 74, 75 and 76 all exist and are `open`/recorded with the matching text.
- The deferred todo `.planning/todos/pending/2026-10-16-harmonise-llm-config-new-signatures.md` exists.
- `deferred-items.md` records the single-feature test-build warnings. The workspace `--all-targets -D warnings` gate is stated clean.

### Anti-Patterns Found

None blocking. No debt markers were added. No library-code `unwrap`/`expect`/`panic!` was added.

| Observation | Severity | Note |
|---|---|---|
| The OpenAI adapter still sends `max_tokens`, which `gpt-5.4-nano` (the probed default model) rejects with HTTP 400 (WINDOWS 75) | Info/Warning | Accepted and documented on the page. On this wire the "equivalent behaviour or early failure" promise does not hold for that one model. The failure is a runtime 400, not a build-time refusal. Owner item. |
| Kimi adapter-wide fallback profile contradicts the 2026-10-15 probe, for unseeded Kimi ids only (WINDOWS 76) | Info | Accepted. The seeded kimi-k2.6 and kimi-k3 entries are correct. |
| `total_tokens = prompt + completion` kept (WINDOWS 74); plan 50-12's "vendor total, never recomputed" truth is not implemented | Info | The ROADMAP SCs do not require it. The plan-level truth is knowingly unimplemented. |
| OpenAI and Gemini reasoning text is "not mapped" | Info | Within D-11's scope ("every adapter that receives it"), and the page says so. Gemini would need `includeThoughts`, which is Phase 51 territory. |
| Ollama and OpenAI-compatible have no seeded models (not probed) | Info | They run on the adapter-wide fallback profile with the `paladin.unprofiled_model` marker (D-07). Documented as a stated limit. |
| `Commissary::for_model` has no production caller in the LLM call path (only `rag_retrieval_service` builds a `Commissary`, via `new`). The production reader of the per-model window is `HistoryTrimmer` | Info | Pre-existing: no LLM-path Commissary construction exists on the tree. The library API is wired and tested. |
| `.planning/ROADMAP.md` line 242 still shows Phase 50 as `- [ ]` while all 18 plan lines are `[x]` | Info | Bookkeeping for the orchestrator on phase completion. |

### Human Verification Required

See the frontmatter `human_verification` list: CI coverage floor, CI Docker integration job, mdBook build, operator live smoke on the final tree, and owner decisions on WINDOWS 74/75/76.

### Gaps Summary

No gaps. All six ROADMAP success criteria are backed by code I read and tests I ran, and every owner decision (option-a, R-1, R-2) matches the shipped code. Status is `human_needed` only because the coverage floor, the Docker suites, the mdBook render and a credentialed live smoke cannot be exercised in this environment. The three open owner items (rows 74-76) are accepted, ledgered deviations. Row 75 is the most user-visible: an OpenAI model that wants `max_completion_tokens` fails at runtime instead of at build.

---

_Verified: 2026-10-16_
_Verifier: Claude (gsd-verifier)_
