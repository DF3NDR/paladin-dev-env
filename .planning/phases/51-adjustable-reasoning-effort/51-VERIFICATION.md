---
phase: 51-adjustable-reasoning-effort
verified: 2026-10-23T05:00:00Z
status: human_needed
score: 7/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 5/7
  gaps_closed:
    - "G-1 / SC5 / REASON-05: single-agent run_agent NodeFinished now carries reasoning_requested / reasoning_effective (plan 51-15)"
    - "G-2 / D-09: host-load fallback precedence (51-16); LlmPort::check_reasoning on all nine adapters plus Pricing/Cadence decorators (51-17, 51-18); chain per-hop check_reasoning, delegating pre-flight and fail-closed chain effort profile (51-19); conformance and host grid proof, MIGRATION/CHANGELOG/ADR/guide register (51-20)"
  gaps_remaining: []
  regressions: []
gaps: []
behavior_unverified_items: []
human_verification:
  - test: "CI coverage job: cargo llvm-cov --fail-under-lines 82 on the merged branch"
    expected: "Workspace line coverage at or above the 82 percent floor (ADR-0006) with the phase's added Rust lines (now about 3.3k more from plans 51-15 to 51-20) included"
    why_human: "cargo-llvm-cov is absent from the devcontainer and plan 51-20's summary states coverage stays the CI coverage job's call; nothing in the gap-closure summaries resolved it"
  - test: "Decide whether the Anthropic Budget path shipping as operator-override-only (no seeded budget span on any model) satisfies REASON-06"
    expected: "Owner accepts that Budget(n) on Anthropic refuses on every seeded model until llm.anthropic.models.<id>.effort.budget declares a span (validation logic, lower bound 1024 and below max_tokens, is implemented and tested against an override), or asks for a seeded span for the claude-opus-4-6 / claude-sonnet-4-6 pair where 1024 was accepted and 1023 rejected"
    why_human: "A policy call about ADR-0061's seed rule against REASON-06's wording. Unchanged by the gap-closure plans (none touched seeds)"
  - test: "Decide whether Gemini 2.5 thinkingBudget ranges (REASON-06 'Gemini 2.5 thinkingBudget ranges, re-verified live') need an override-documented waiver"
    expected: "Owner accepts that no Gemini 2.5 id was probed (retired for new users, 404; excluded in the candidate set) so those ids stay Unmeasured and refuse until overridden"
    why_human: "The requirement text names a live re-verification that could not be performed against retired models. Unchanged by the gap-closure plans"
---

# Phase 51: Adjustable Reasoning Effort Verification Report

**Phase Goal:** One provider-neutral, per-agent (and per-request) reasoning-effort setting that every adapter translates exactly to its vendor's native form, or refuses loudly before the HTTP call; the central hazard being the silent no-op (Kimi ignores top-level `reasoning_effort`).
**Verified:** 2026-10-23
**Status:** human_needed (all code and doc truths verified; three owner or CI items remain, none a code gap)
**Re-verification:** Yes, after gap closure (plans 51-15 to 51-20, commits `bdfa704a..337a91f9`)

Both prior gaps are closed in code and proven by tests that this verifier ran. The five previously passing truths were re-checked (existence, wiring, and a scoped test run) and none regressed. What remains is the unchanged human list: the CI coverage floor and the two REASON-06 owner decisions.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1 / REASON-01: `#[non_exhaustive]` `ReasoningEffort` settable per request, per agent (`PaladinData`, `PaladinBuilder::reasoning_effort`, YAML `AgentDefinition`), per provider; resolved request, agent, provider; unset byte-identical | VERIFIED | Regression check: `crates/paladin-core/src/platform/container/reasoning.rs`, nine golden bodies under `crates/paladin-llm/tests/golden/reasoning_unset/` all present. `reasoning::in_force` (`crates/paladin-llm/src/reasoning.rs:355`) resolves `request.or(default)` field by field. The agent-host precedence defect that previously qualified this truth is now fixed (see Truth 7). `cargo test -p paladin-llm --lib --all-features -- reasoning effort kimi::` 384 passed. |
| 2 | SC2 / REASON-02: compile-enforced `ReasoningWire`, model-aware `map_reasoning`, exact map or `UnsupportedParameter` (Permanent) before HTTP; nearest-level opt-in | VERIFIED | Regression: `admit` is now `judge` plus the substitution record (`reasoning.rs:391,423`), so the refusal text is produced in one place; `anthropic/adapter.rs:544-566` shows `in_force` then `admit` then `map_reasoning` before the body is built; compat engine `engine.rs:860-884` same. Covered by the 384-test run above. |
| 3 | SC3 / REASON-03: no preset emits a field its vendor ignores; Kimi never emits top-level `reasoning_effort`; per-adapter wire-body cases; live proof | VERIFIED | Unchanged by the gap closure; the Kimi preset and conformance tests are in the 384-test run (pass). Probe and smoke evidence files (`51-EFFORT-PROBE.md`, `51-EFFORT-SMOKE.md`) still present. Warning W-3 (Kimi smoke run 2 non-monotonic) stands. |
| 4 | SC4 / REASON-04 + REASON-06: per-model levels and budget bounds from Phase 50's registry with overrides; each fallback hop resolves against its own model; `Budget(n)` validated against vendor bounds and `max_tokens` | VERIFIED (warnings W-1, W-2) | `judge` holds a budget below the effective `max_tokens` (`reasoning.rs:466-476`). The chain now judges each hop with its own `check_reasoning` using that hop's step model and provider defaults (`fallback.rs:951-985`). `fallback::` 65 passed, including `a_step_chain_resolves_each_hop_against_its_own_step_model`, `a_hops_own_provider_fallback_is_honoured_by_the_pre_flight`, `a_hops_provider_default_effort_is_pre_flighted_even_when_the_request_sets_none`, `a_hop_with_no_profile_at_all_refuses_an_explicit_effort`. |
| 5 | SC5 / REASON-05: requested and effective effort in `LlmResponse.metadata` and on the trace stream; every adapter fills `reasoning_tokens` | VERIFIED (G-1 closed) | `src/application/services/run/worker.rs:1737-1744`: the `Ok(result)` arm now reads `result.run_metadata.get(REASONING_REQUESTED_METADATA_KEY).cloned()` and `...EFFECTIVE...`; the `Err` arm keeps both `None` (`:1810-1811`). The data path is real: `PaladinExecutionService::merge_run_metadata` keeps every `paladin.*` key except served_by (`paladin_execution_service.rs:1286-1311`), pinned by `run_metadata_keeps_the_latest_calls_reasoning_keys`. Ran `cargo test --offline --lib agent_run_node_finished` (4 passed): `agent_run_node_finished_carries_the_calls_own_reasoning_metadata`, `..._reads_only_the_two_reasoning_keys`, `..._never_reads_the_agents_configured_effort`, `failed_agent_run_node_finished_carries_no_reasoning`. The engine path (`AttemptReasoning::from_run_metadata`, `superstep.rs:1137`) is unchanged. |
| 6 | SC6 / REASON-07: ADR amending ADR-0004; mdBook page with mapping table; MIGRATION 9.2 and CHANGELOG; api-surface baseline; 82 percent floor | VERIFIED (coverage floor deferred to CI, human item 1) | ADR-0062, the 580-plus-line guide, MIGRATION 9.2 and DR-21, root and crate CHANGELOGs present. Gap-closure additions verified: MIGRATION.md:356 registers `LlmPort::check_reasoning` as an `N` row and names the decorator-forwarding rule; ADR-0062 carries dated "Revised 2026-10-23" notes (lines 57, 105, 137); the guide's "Fallback chains" and "Build-time checks" sections (`reasoning-effort.md:217,234,254`) describe the per-hop, provider-default-aware gates; `CHANGELOG.md:12`, `crates/paladin-ports/CHANGELOG.md:13`, `crates/paladin-llm/CHANGELOG.md:12` carry the entries; `tests/consumer_surface_muster.rs:380-381` pins the method. The facade baseline `.project/current-exports.txt` lists no `LlmPort` methods, so no baseline change was needed (51-20 reports `make api-surface` unchanged at 4323 items). |
| 7 | CONTEXT D-08/D-09 and plans 51-07/51-08: build-time, host-load and chain pre-flight checks give the same answer as send-time admission, with precedence request, agent, provider | VERIFIED (G-2 closed) | See the G-2 breakdown below. |

**Score:** 7/7 truths verified; 0 present-but-behavior-unverified.

### Gap G-1 re-verification (SC5 / REASON-05)

Closed. Level 1 exists, level 2 substantive (copies the two adapter-stamped strings verbatim, never parses and never reads `PaladinData`), level 3 wired (`merge_run_metadata` to `PaladinResult.run_metadata` to the `NodeFinished` literal), level 4 data flows (the merge test and the 4 worker tests above use a stamping `PaladinPort`, and the negative test proves the agent's configured effort is not substituted for the call's own).

### Gap G-2 re-verification (D-09)

Closed on all three original sub-defects plus the structural fix that prevents recurrence.

| Original defect | Fix verified | Evidence |
|-----------------|--------------|----------|
| (1) `agent_host.rs` inverted fallback precedence | `check_block_reasoning` now computes `def.reasoning_fallback.or(block.fallback)` (`agent_host.rs:208-209`); struct doc and fn doc rewritten to "agent's fallback first" (`:183-185`, `:253`). | Ran the 48 agent-host tests (binary built from HEAD source; `--features web-server`): `block_nearest_lower_agent_refuse_is_refused`, `block_refuse_agent_nearest_lower_passes`, `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, `the_agents_fallback_applies_to_a_block_default_that_sets_none`, `the_host_fallback_hint_appears_only_when_refuse_is_in_force` all pass. These are the two cases the prior report required. |
| (2) Builder and chain pre-flight ignored provider-level `reasoning_fallback` | New provided method `LlmPort::check_reasoning(&self, model, effort, fallback)` (`llm_port.rs:2587`) with a default that judges the profile alone (and refuses an explicit effort with no profile). `PaladinBuilder::build` calls `self.llm_port.check_reasoning(...)` (`paladin_builder.rs:1380`); `FallbackLlmAdapter::preflight_reasoning` calls `self.check_reasoning(...)` (`fallback.rs:720`). | Overrides present on all nine adapters (anthropic, openai, gemini, deepseek natively via `reasoning::check`; kimi, grok, qwen, ollama, openai_compatible via `CompatEngine::check_reasoning`, `engine.rs:776`) and on `PricingLlmAdapter` (`pricing.rs:356`), `CadenceLlmAdapter` (`cadence.rs:469`) and `FallbackLlmAdapter` (`fallback.rs:951`). `reasoning::check` (`reasoning.rs:489`) runs the same `in_force` and the same `judge` that `admit` runs. Send-time sites (`anthropic/adapter.rs:544`, `compat/engine.rs:860`, and openai, deepseek, gemini) use `in_force` with the same provider defaults and the same masked profile. Ran `cargo test -p paladin-llm --lib --all-features -- check_reasoning` (24 passed): `check_reasoning_agrees_with_send_time_admission` on all nine fixtures (72 effort-by-fallback-by-model rows each, equal verdict and equal error text), per-adapter `..._consults_the_provider_defaults`, Pricing/Cadence forwarding. Ran builder tests (56 passed) including `the_builder_verdict_equals_check_reasoning_for_every_combination`, `a_provider_level_fallback_lets_an_agent_effort_substitute_at_build`. |
| (3) `FallbackLlmAdapter::model_profile` dropped profile-less hops from the effort intersection | The chain's `model_profile` sets `effort = Unmeasured` and clears `effort_verified_on` whenever any hop reports no profile (`fallback.rs:925-935`); more importantly the effort authority is now the AND of each hop's own `check_reasoning`, not the intersected profile. | `a_chain_profile_is_effort_unmeasured_when_any_hop_reports_none`, `a_chain_with_a_profileless_hop_refuses_an_explicit_effort_at_build`, `a_hop_with_no_profile_at_all_refuses_an_explicit_effort`, `a_chain_check_reasoning_is_the_and_of_every_hop_check` pass. |
| Recurrence guard | Host grid test: 54 combinations of block effort x block fallback x agent effort x agent fallback, host-and-builder verdict equals `adapter.check_reasoning(...)` of the adapter the same block creates, asserting both verdicts occur and no secret in any error. Pricing decorator survival test. | `the_host_and_builder_verdicts_equal_the_adapters_check_for_every_combination` and `the_host_verdict_survives_the_pricing_decorator` pass. |

Note (INFO, not a gap): `check_block_reasoning` in the host still re-implements the profile and fallback rule rather than calling `llm.check_reasoning`, and the "set a reasoning_fallback" hint text exists in three copies. Both are deliberate (51-20), are pinned by the grid and by `the_host_fallback_hint_appears_only_when_refuse_is_in_force`, and the builder immediately re-judges through `check_reasoning`, so a drift would surface as a failing grid row.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/reasoning.rs` | `ReasoningEffort`, `ReasoningFallback`, metadata keys | VERIFIED | Present, unchanged |
| `crates/paladin-llm/src/reasoning.rs` | `in_force`, `admit`, new `judge`, new `check` | VERIFIED | Gap closure added `judge` and `check`; `admit` = `judge` + record |
| `crates/paladin-ports/src/output/llm_port.rs` | `LlmPort::check_reasoning` provided method | VERIFIED | Line 2587, doc with decorator-forwarding rule, doc test, 2 unit tests pass |
| `crates/paladin-ports/src/output/model_profile.rs` | `EffortSupport` and resolve | VERIFIED | Unchanged |
| `src/application/services/run/worker.rs` | `NodeFinished` effort fields from `run_metadata` | VERIFIED | Lines 1737-1744 |
| `src/application/services/run/worker_tests.rs` | Four agent-run trace tests | VERIFIED | All 4 run and pass |
| `src/infrastructure/web/agent_host.rs` | Corrected precedence plus grid | VERIFIED | Line 208, tests pass |
| `crates/paladin-llm/src/fallback.rs` | Per-hop `check_reasoning`, delegating pre-flight, fail-closed profile | VERIFIED | Lines 720, 925-985 |
| `crates/paladin-llm/src/conformance.rs` | Nine-adapter parity case (25-case suite) | VERIFIED | Line 2134; invoked by all nine fixtures |
| `51-EFFORT-PROBE.md`, `51-EFFORT-SMOKE.md`, ADR-0062, guide page, goldens | Evidence and docs | VERIFIED | Present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `PaladinData.reasoning_effort` | `LlmRequest.reasoning` | `with_reasoning` at three sites | WIRED | Unchanged |
| `LlmRequest.reasoning` + config default | wire body | `in_force` then `admit` then `map_reasoning` | WIRED | |
| Adapter response | `NodeFinished` (engine) | `AttemptReasoning::from_run_metadata` | WIRED | |
| Adapter response | `NodeFinished` (single-agent worker) | `run_metadata` to `worker.rs:1737` | WIRED | Was NOT_WIRED (G-1) |
| Provider-block fallback | host load check | `agent_host.rs:208` | WIRED | Was PARTIAL (G-2) |
| Agent effort and fallback | builder gate | `llm_port.check_reasoning` (`paladin_builder.rs:1380`) | WIRED | Provider defaults now consulted |
| Request | fallback chain pre-flight | `preflight_reasoning` to per-hop `check_reasoning` (`fallback.rs:743,796`) | WIRED | Called from `generate` and `generate_stream` |
| `PricingLlmAdapter`, `CadenceLlmAdapter` | inner port | delegating `check_reasoning` | WIRED | Tests pass |
| Agent effort fingerprint | node cache key | `cache_key.rs` | WIRED | Provider default intentionally excluded (W-4) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `NodeFinished.reasoning_*` (engine) | `run_metadata` keys | adapter-stamped response metadata | Yes | FLOWING |
| `NodeFinished.reasoning_*` (worker) | `result.run_metadata` | `merge_run_metadata` of each call's `paladin.*` keys | Yes (4 tests) | FLOWING (was HOLLOW) |
| Chain effort verdict | per-hop `check_reasoning` | each hop's profile and defaults | Yes | FLOWING |
| Seeded effort rows | `EffortSupport` | dated probe file | Yes | FLOWING |

### Behavioral Spot-Checks

Run with `CARGO_BUILD_JOBS=4`, one scoped command at a time; the orchestrator's workspace evidence on HEAD 1740041f was accepted as given and not re-run.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Single-agent trace carries effort | `cargo test --offline --lib agent_run_node_finished` | 4 passed | PASS |
| Host precedence, block x agent grid, Pricing survival | agent-host tests (`paladin-5c048b30aa0afdb9 agent_host`, web-server build from HEAD source) | 48 passed, 0 failed | PASS |
| Builder verdict equals adapter check | `paladin-b3e859b0259c2495 paladin_builder` | 56 passed | PASS |
| Nine-adapter parity, defaults, decorators | `cargo test --offline -p paladin-llm --lib --all-features -- check_reasoning` | 24 passed | PASS |
| Chain pre-flight and profile | `cargo test --offline -p paladin-llm --lib --all-features -- fallback::` | 65 passed | PASS |
| Port default | `cargo test --offline -p paladin-ports --lib -- check_reasoning` | 2 passed | PASS |
| Regression of reasoning, effort, Kimi, seeds, drift page | `cargo test --offline -p paladin-llm --lib --all-features -- reasoning effort kimi::` | 384 passed | PASS |

### Probe Execution

Step 7c: SKIPPED. No `probe-*.sh` is declared by any plan; the live probe is the Rust harness whose recorded output is the evidence file, not re-run (spends vendor keys).

### Requirements Coverage

All seven IDs appear in plan frontmatter (plans 01-14 as before; gap plans: 51-15 REASON-05, 51-16 to 51-18 REASON-01, 51-19 REASON-04 and REASON-01, 51-20 REASON-01, REASON-04, REASON-07) and in REQUIREMENTS.md. None orphaned.

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| REASON-01 | 01, 03, 06-09, 16, 17, 18, 19, 20 | SATISFIED | Truths 1 and 7; precedence now identical at host, builder, chain and send time |
| REASON-02 | 01, 03-06, 11-13 | SATISFIED | Truth 2 |
| REASON-03 | 02-05, 10-12, 14 | SATISFIED | Truth 3 |
| REASON-04 | 01-04, 06-08, 10-12, 19, 20 | SATISFIED | Truths 4 and 7; per-hop verdicts |
| REASON-05 | 03, 05, 07, 09, 12, 15 | SATISFIED | Truth 5; both trace paths now fill the fields |
| REASON-06 | 01, 03, 05, 11-14 | SATISFIED with caveats | Truth 4; W-1, W-2 owner decisions |
| REASON-07 | 04, 07-09, 13, 14, 20 | SATISFIED pending CI coverage | Truth 6 |

REQUIREMENTS.md still shows REASON-01..07 as `[ ]` / Pending and ROADMAP.md shows Phase 51 unchecked: tracking updates for the orchestrator after the human items are settled, not code gaps.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none new) | | Diff `bdfa704a~1..HEAD` scanned for `TBD`, `FIXME`, `XXX`: none added. `unwrap`/`expect`/`panic!` additions are in test modules only; the production `check_reasoning`, `check`, `judge`, host and worker hunks read clean | | |
| `crates/paladin-llm/src/provider_factory.rs` | 1935 | Unused import in the default-feature test build | INFO | Recorded in `deferred-items.md`; all-features clippy unaffected |
| `crates/paladin-battalion/src/engine/cache_key.rs` | 274-283 | `unwrap_or_default()` on a serde string (REVIEW IN-02) | INFO | Cannot fail for current shapes; recorded as deferred with a revisit point |

### Warnings (not blocking)

- **W-1 Anthropic Budget is operator-override-only.** Unchanged. Human item 2.
- **W-2 Gemini 2.5 `thinkingBudget` ranges not re-verified live.** Unchanged. Human item 3.
- **W-3 Kimi adapter-path smoke run 2 is not monotonic** (`low` 47 vs `max` 26 reasoning tokens); the Kimi effect claim rests on the raw probe.
- **W-4 A provider-level default effort is not in the node-cache key** (REVIEW WR-04). Now explicitly recorded in `deferred-items.md` with reason and revisit point (decide with `NODE_CACHE_KEY_VERSION`); the guide tells operators to rotate `key_prefix`. A documented design choice, not part of either gap and not a must-have truth.
- **W-5 82 percent coverage floor** cannot be checked locally (human item 1).
- **W-6 `make security`** was noted open in plan 51-14 (three RustSec advisories in transitive TLS-root crates, outside this phase's changes); the orchestrator reported the full chain green on HEAD, accepted as given.

### Human Verification Required

1. **CI coverage job** - confirm `cargo llvm-cov --fail-under-lines 82` is green on the merged branch. Why human: the tool is not installed locally.
2. **Anthropic Budget policy** - accept override-only, or request a seeded span for the 4.6 pair. Why human: seed-rule versus requirement-wording judgement.
3. **Gemini 2.5 waiver** - accept that those ids cannot be live re-verified and remain Unmeasured. Why human: requirement text names a check that could not be done.

### Gaps Summary

None. G-1 and G-2 are closed with executed evidence, and the invariant the second gap broke ("a build-time answer cannot differ from a send-time answer") is now enforced by a shared `reasoning::check`/`judge` path, a nine-adapter conformance case and a 54-combination host grid rather than by comments. Every divergence the prior report listed also failed loud before HTTP, so the silent no-op hazard was never open; it is now consistent as well. Remaining items are owner or CI decisions only.

---

_Verified: 2026-10-23_
_Verifier: Claude (gsd-verifier)_
