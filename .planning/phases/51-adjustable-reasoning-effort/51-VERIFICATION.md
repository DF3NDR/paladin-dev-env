---
phase: 51-adjustable-reasoning-effort
verified: 2026-10-22T20:30:00Z
status: gaps_found
score: 5/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps:
  - truth: "SC5 / REASON-05: requested and effective effort are emitted on the Phase 28 trace stream"
    status: partial
    reason: "The graph-engine path reads paladin.reasoning_* from the attempt's run_metadata and fills NodeFinished. The single-agent run path (src/application/services/run/worker.rs, the /v1/agents path) hard-codes both fields to None on the success arm even though the PaladinResult in scope carries run_metadata with the two keys. Every single-agent run therefore traces no effort, and a substitution is invisible in its trace."
    artifacts:
      - path: "src/application/services/run/worker.rs"
        issue: "lines 1726-1727: reasoning_requested: None, reasoning_effective: None on NodeOutcomeKind::Succeeded"
    missing:
      - "Read REASONING_REQUESTED_METADATA_KEY / REASONING_EFFECTIVE_METADATA_KEY from result.run_metadata in the Ok(result) arm"
      - "A worker test mirroring node_finished_carries_the_attempts_own_reasoning_metadata"
  - truth: "CONTEXT D-09 / plan 51-07, 51-08: the build-time and host-load checks give the same answer as the send-time admission, with precedence request -> agent -> provider"
    status: partial
    reason: "Three pre-judging call sites do not reproduce the adapters' reasoning::in_force. (1) src/infrastructure/web/agent_host.rs:200-204 resolves the fallback as block.fallback.or(def.reasoning_fallback), the inverse of send time (agent value first, provider default second): with block nearest_lower and agent refuse the load check passes and every call then refuses; with block refuse and agent nearest_lower the load check refuses an agent that would have substituted. Re-verified by reading both sites. (2) PaladinBuilder::build (paladin_builder.rs:1363) and FallbackLlmAdapter::preflight_reasoning (fallback.rs:716) read only the agent/request fallback, never llm.<provider>.reasoning_fallback, so a configured provider fallback is inert for those two gates and the error text tells the operator to set something already set. (3) FallbackLlmAdapter::model_profile (fallback.rs:933-941) uses filter_map, so a hop reporting no profile is dropped from the chain intersection at build time but refuses at pre-flight. Every divergence fails loud before HTTP (no silent no-op), but the stated invariant that build-time and send-time answers cannot drift is false."
    artifacts:
      - path: "src/infrastructure/web/agent_host.rs"
        issue: "check_block_reasoning fallback precedence inverted relative to reasoning::in_force; its struct doc says the block's fallback wins over the agent's"
      - path: "src/application/services/paladin/paladin_builder.rs"
        issue: "build-time effort check ignores the provider-level reasoning_fallback"
      - path: "crates/paladin-llm/src/fallback.rs"
        issue: "preflight_reasoning ignores hop/provider reasoning_fallback; model_profile skips profile-less hops"
    missing:
      - "agent_host.rs: def.reasoning_fallback.or(block.fallback).unwrap_or_default(), fix the BlockReasoning.fallback doc, add tests for block nearest_lower + agent refuse (expects refusal) and the reverse (expects pass)"
      - "Expose provider reasoning defaults to the pre-judging gates (or document the limitation and correct the refusal text)"
      - "FallbackLlmAdapter::model_profile (or an effort-specific accessor) must not silently drop a hop that reports no profile when an explicit effort is being judged"
behavior_unverified_items: []
human_verification:
  - test: "CI coverage job: cargo llvm-cov --fail-under-lines 82 on the merged branch"
    expected: "Workspace line coverage at or above the 82 percent floor (ADR-0006) with the phase's 16.6k added Rust lines included"
    why_human: "cargo-llvm-cov is absent from the devcontainer; the floor is enforced only by the CI coverage job, so it cannot be confirmed here"
  - test: "Decide whether the Anthropic Budget path shipping as operator-override-only (no seeded budget span on any model) satisfies REASON-06"
    expected: "Owner accepts that Budget(n) on Anthropic refuses on every seeded model until llm.anthropic.models.<id>.effort.budget declares a span (the validation logic, lower bound 1024 and below max_tokens, is implemented and tested against an override), or asks for a seeded span for the claude-opus-4-6 / claude-sonnet-4-6 pair where 1024 was accepted and 1023 rejected"
    why_human: "A policy call about ADR-0061's seed rule against REASON-06's wording; the code is consistent with the seed rule and with D-00h"
  - test: "Decide whether Gemini 2.5 thinkingBudget ranges (REASON-06 'Gemini 2.5 thinkingBudget ranges, re-verified live') need an override-documented waiver"
    expected: "Owner accepts that no Gemini 2.5 id was probed (retired for new users, 404; excluded in the candidate set) so those ids stay Unmeasured and refuse until overridden"
    why_human: "The requirement text names a live re-verification that could not be performed against retired models"
---

# Phase 51: Adjustable Reasoning Effort Verification Report

**Phase Goal:** One provider-neutral, per-agent (and per-request) reasoning-effort setting that every adapter translates exactly to its vendor's native form, or refuses loudly before the HTTP call; the central hazard being the silent no-op (Kimi ignores top-level `reasoning_effort`).
**Verified:** 2026-10-22
**Status:** gaps_found
**Re-verification:** No, initial verification

The phase delivers its core: the type, the single admission rule, nine adapters that map or refuse before HTTP, probe-derived seed data, the Kimi regression, the ADR, the page, and the register. Two verified defects remain in the observability and build-time-check layers, both previously identified by the untracked `51-REVIEW.md` and independently re-verified here against the code.

## Goal Achievement

### Observable Truths

Roadmap success criteria are the contract; PLAN and CONTEXT truths that add detail are merged in.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1 / REASON-01: `#[non_exhaustive]` `ReasoningEffort` (nine values) settable per request, per agent (`PaladinData`, `PaladinBuilder::reasoning_effort`, YAML `AgentDefinition`), per provider (`llm.<provider>.reasoning_effort`), resolved request, agent, provider on every `LlmRequest` the execution service builds; unset is byte-identical | VERIFIED | Enum in `crates/paladin-core/src/platform/container/reasoning.rs` (non_exhaustive, nine variants, explicit `xhigh` serde rename). `LlmRequest.reasoning` + `with_reasoning` (`llm_port.rs:1206,1370`). `PaladinData.reasoning_effort` (`paladin.rs:216`), `AgentDefinition.reasoning_effort` (`agents.rs:367`), builder setter in `.project/current-exports.txt:2731`. Three construction sites wire the agent value (`paladin_execution_service.rs:3150,3364,4004`). Adapters apply `reasoning::in_force` (request value first, provider default second). Nine raw-byte goldens under `tests/golden/reasoning_unset/` and `unset_reasoning_leaves_the_wire_body_unchanged` conformance case; `cargo test -p paladin-llm --lib --all-features -- kimi:: conformance fallback::` 360 passed. |
| 2 | SC2 / REASON-02: compile-enforced `ReasoningWire` on `CompatRequestParameters`, generic compat defaults to `Unsupported`, private model-aware `map_reasoning` in Anthropic/OpenAI/DeepSeek/Gemini, exact map or `UnsupportedParameter` (Permanent) before HTTP; coerced, gated-off, Off-on-always-thinking, Budget-where-only-levels all refuse; nearest-level is opt-in | VERIFIED | `CompatRequestParameters.reasoning: ReasoningWire` with no `Default`; `all(reasoning)` takes the wire as a required argument (`engine.rs:204`). `map_reasoning` exists in all five code paths (compat `engine.rs:550`, anthropic `:1234`, deepseek `:347`, gemini `:1483`, openai `:380`) and each calls `reasoning::admit` before the request body is built (`anthropic/adapter.rs:551`, etc.). `LlmError::UnsupportedParameter` at `llm_port.rs:609`, `Transience::Permanent` at `:1049`, label rows in `llm_failure.rs:118` and `llm_decision.rs:312`. `EffortSupport::resolve` (`model_profile.rs:832`) refuses `coerced` levels whatever the fallback, substitutes only level to level along the caller's direction. `OpenAiCompatibleConfig` / `OllamaConfig` default `reasoning_wire: Unsupported`. 247 `reasoning` and 158 `effort` lib tests pass. |
| 3 | SC3 / REASON-03: no preset emits a field its vendor ignores; Kimi never emits top-level `reasoning_effort`; per-adapter wire-body cases; live smoke proves invalid effort rejected and higher level reports more reasoning tokens | VERIFIED | Kimi declares `ReasoningWire::ThinkingObject` (`kimi/adapter.rs:355`); tests `kimi_preset_never_emits_top_level_reasoning_effort` (`:2280`) and `..._enable_thinking_or_a_budget_beside_thinking` pass. The probe (`51-EFFORT-PROBE.md`, 718 rows, 62 models, 7 of 9 vendors) shows Kimi top-level `bogus-effort-51` drew 200 and `thinking_object` drew 400 on both Kimi models, and the verdict table marks each shipped form `parsed: yes` and `effect: yes`. Grok `top_level_effort` is confirmed 5 of 5 (D-04 satisfied; not shipped on desk research). Ollama and OpenAI-compatible were not probed (no endpoint) and ship `Unsupported`, as D-02 allows. Adapter-path smoke `51-EFFORT-PROBE`-independent: `claude-opus-4-8` 0 to 35 and `deepseek-v4-pro` 70 to 77 reasoning tokens low to max. See warning W-3 for the Kimi smoke pair. |
| 4 | SC4 / REASON-04 + REASON-06: levels and budget bounds are per model from Phase 50's dated registry with config overrides; each fallback hop resolves against its own model; `Budget(n)` validated against vendor bounds and `max_tokens`; `EmptyCompletion` stays the backstop | VERIFIED (with warnings W-1, W-2) | `ModelProfile.effort: EffortSupport` + `effort_verified_on`; seeded rows carry a `// Source for the effort data` comment and date `2026-10-22` for every probed model; unprobed ids are `Unmeasured` and refuse naming `llm.<provider>.models.<id>.effort`. `reasoning::admit` holds a budget below the effective `max_tokens` (`reasoning.rs:410`). `BudgetBounds` inclusive span (`model_profile.rs:547`). Qwen seeded `1..=32768`. `FallbackLlmAdapter::preflight_reasoning` resolves every hop against `FallbackStep.model` before the first call (`fallback.rs:716`, `reasoning_preflight_tests`). The build-time check consistency is carried separately as gap G-2. |
| 5 | SC5 / REASON-05: requested and effective effort in `LlmResponse.metadata` beside `SERVED_BY_METADATA_KEY` and on the trace stream; every adapter fills `reasoning_tokens`; reasoning text typed on normal and streaming paths | FAILED (partial) | Metadata keys `paladin.reasoning_requested` / `paladin.reasoning_effective` stamped by every adapter and the terminal stream chunk; smoke rows show both values on all 8 live calls. `TraceEvent::NodeFinished` carries both fields with `skip_serializing_if` (`trace.rs:274-291`), filled on the engine path. **But** the single-agent run path emits `None` for both (`worker.rs:1726-1727`) although the `PaladinResult` carries the keys in `run_metadata`. Reasoning text surfaces were delivered by Phase 50; this phase added no second surface. See G-1. |
| 6 | SC6 / REASON-07: ADR amending ADR-0004; mdBook "Reasoning effort" page with the per-provider/per-model mapping table; MIGRATION 9.2 and CHANGELOG carry the additive surface; api-surface baseline refreshed; 82 percent floor and doc tests met | VERIFIED (coverage floor deferred to CI, see human item 1) | `.planning/decisions/0062-reasoning-effort-fail-loud-policy.md` (Accepted, "Amends ADR-0004"); ADR-0004 carries `**Amended by ADR-0062**`; PROMOTION.md advances to 0063. `docs/src/user-guides/reasoning-effort.md` (580 lines, in `SUMMARY.md:32`) with a mapping table covering every seeded model, guarded by the drift test `registry::reasoning_page_tests::the_reasoning_effort_page_agrees_with_the_adapters` (passes). MIGRATION 9.2 rows, 9.5 keys and DR-20..DR-26 present; CHANGELOG entries in root and three crates. `.project/current-exports.txt` contains the new `PaladinBuilder::reasoning_effort` and `AgentDefinition::reasoning_effort` entries. |
| 7 | CONTEXT D-08/D-09 and plans 51-07/51-08: build-time and host-load checks agree with send-time admission (same precedence) | FAILED (partial) | See G-2. Fail-loud in every divergent case, so no silent no-op, but the drift the code comments promise cannot happen does happen. |

**Score:** 5/7 truths verified (SC5 and the D-09 consistency truth are partial failures); 0 present-but-behavior-unverified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/reasoning.rs` | `ReasoningEffort`, `ReasoningFallback`, metadata keys | VERIFIED | 370 lines, doc tests, non_exhaustive; re-exported through `paladin-ports` and the facade |
| `crates/paladin-llm/src/reasoning.rs` | `ReasoningWire`, `admit`, `in_force`, `mask_effort`, `SubstitutionLog` | VERIFIED | 890 lines; used by all nine adapters |
| `crates/paladin-ports/src/output/model_profile.rs` | `EffortSupport`, `EffortLevels`, `BudgetBounds`, `resolve`, `intersect` | VERIFIED | One admission rule, AND-intersection with budget overlap |
| `crates/paladin-llm/src/registry/seeded.rs` | Effort rows only where measured, each dated | VERIFIED | Source comments cite the probe sections; round-trip test `every_seeded_effort_level_round_trips_to_its_wire_body` |
| `crates/paladin-llm/tests/golden/reasoning_unset/*.json` | Nine golden unset bodies | VERIFIED | All nine present |
| `51-EFFORT-PROBE.md` | Scrubbed live transcript with provenance and verdict table | VERIFIED | Header, provenance, owner decision, verdict table read; scrub check described |
| `51-EFFORT-SMOKE.md` | Adapter-path smoke on the three web3sec models | VERIFIED | Read in full; all 8 requests answered |
| `.planning/decisions/0062-*.md` | ADR | VERIFIED | Accepted, dated, owner checkpoints recorded |
| `docs/src/user-guides/reasoning-effort.md` | Page with mapping table | VERIFIED | Drift-tested against the adapters |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `PaladinData.reasoning_effort` | `LlmRequest.reasoning` | `with_reasoning` at three sites in `paladin_execution_service.rs` | WIRED | Plus `reasoning_wiring_tests` |
| `LlmRequest.reasoning` + `config.reasoning_effort` | wire body | `reasoning::in_force` then `admit` then `map_reasoning` | WIRED | All five adapter code paths |
| Adapter response | `NodeFinished` (engine) | `AttemptReasoning::from_run_metadata` in `superstep.rs` | WIRED | |
| Adapter response | `NodeFinished` (single-agent worker) | `worker.rs:1726` | NOT_WIRED | Hard-coded `None` (G-1) |
| Provider-block fallback | host load check | `agent_host.rs:200` | PARTIAL | Inverted precedence (G-2) |
| Agent effort | fallback chain | `preflight_reasoning` per hop | WIRED | Provider fallback not consulted (G-2) |
| Agent effort fingerprint | node cache key | `cache_key.rs:263-283` | WIRED | Provider default intentionally excluded (W-4) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `TraceEvent::NodeFinished.reasoning_*` (engine) | `run_metadata` keys | adapter-stamped response metadata | Yes | FLOWING |
| `TraceEvent::NodeFinished.reasoning_*` (worker) | literal `None` | none | No | HOLLOW (G-1) |
| Seeded effort rows | `EffortSupport` | dated probe file | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Reasoning admission, wire mapping, seeds, drift page | `cargo test --offline -p paladin-llm --lib --all-features reasoning` | 247 passed, 0 failed | PASS |
| Conformance, Kimi preset, fallback chain | `cargo test --offline -p paladin-llm --lib --all-features -- kimi:: conformance fallback::` | 360 passed, 0 failed | PASS |
| Effort-named tests | `cargo test --offline -p paladin-llm --lib --all-features -- effort` | 158 passed, 0 failed | PASS |

The orchestrator's full-workspace evidence (9,269 lib/bin tests, 711 doc tests, clean-code, check-gates, api-surface, check-api-examples, check-migration-allowlist, check-consumer-surface, security all exit 0) was accepted as given and not re-run.

### Probe Execution

Step 7c: SKIPPED. No `probe-*.sh` is declared by any plan; the live probe is the Rust harness `live_vendor_smoke --effort`, whose recorded output is the evidence file above and which this verifier did not re-run (it spends the owner's vendor keys).

### Requirements Coverage

All seven IDs appear in plan frontmatter (`requirements:` across plans 01-14) and in REQUIREMENTS.md; none is orphaned.

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| REASON-01 | 51-01, 03, 06, 07, 08, 09 | Enum, three placement levels, precedence, unset byte-identical | SATISFIED | Truth 1 (fallback precedence defect carried under G-2) |
| REASON-02 | 51-01, 03, 04, 05, 06, 11, 12, 13 | Wire declaration, map or refuse, opt-in nearest level | SATISFIED | Truth 2 |
| REASON-03 | 51-02, 03, 04, 05, 10, 11, 12, 14 | No silent no-op, wire-body cases, live proof | SATISFIED | Truth 3 |
| REASON-04 | 51-01, 02, 03, 04, 06, 07, 08, 10, 11, 12 | Per-model levels, per-hop resolution | SATISFIED (build-time consistency gap G-2) | Truth 4 |
| REASON-05 | 51-03, 05, 07, 09, 12 | Requested/effective on metadata and trace; reasoning_tokens | BLOCKED (partial) | Truth 5, G-1 |
| REASON-06 | 51-01, 03, 05, 11, 12, 13, 14 | Budget validation; effort/budget/timeout guidance | SATISFIED with caveats | Truth 4; W-1, W-2; guidance in the page and live smoke run 2 |
| REASON-07 | 51-04, 07, 08, 09, 13, 14 | ADR, page, MIGRATION, CHANGELOG, baseline, coverage | SATISFIED pending CI coverage | Truth 6 |

REQUIREMENTS.md still shows REASON-01..07 as `[ ]` / Pending and ROADMAP.md shows Phase 51 unchecked; these are tracking updates for the orchestrator after the gaps close, not code gaps.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/application/services/run/worker.rs` | 1726-1727 | Hard-coded `reasoning_requested: None` on a path that has the data | BLOCKER | G-1 |
| `src/infrastructure/web/agent_host.rs` | 200-204 | Precedence inverted against `reasoning::in_force` | BLOCKER | G-2 |
| `crates/paladin-llm/src/fallback.rs` | 933-941 | `filter_map` drops profile-less hops in the chain intersection | WARNING | G-2 |
| `crates/paladin-llm/src/provider_factory.rs` | 1935 | Unused import `ReasoningFallback` in the default-feature test build | INFO | Recorded in `deferred-items.md`; clippy and `make clean-code` run all-features |
| `crates/paladin-battalion/src/engine/cache_key.rs` | 274-283 | `unwrap_or_default()` on a serde string for an effort | INFO | Cannot fail for current shapes; `Display` is infallible |

No `TBD`, `FIXME` or `XXX` marker was added by the phase (diff `b6ccecc5..HEAD` scanned). No `unwrap`/`expect`/`panic!` was found in the library code reviewed.

### Warnings (not blocking, but the owner should know)

- **W-1 Anthropic Budget is operator-override-only.** No seeded Anthropic row carries a budget span: `claude-opus-4-6` and `claude-sonnet-4-6` accepted `budget_tokens` 1024 and rejected 1023, but the vendor documents no fixed maximum, so by the seed rule no span is written and a budget refuses. The validation (lower bound, below `max_tokens`) is implemented and tested against an override (`anthropic/adapter.rs:4160`). Human item 2.
- **W-2 Gemini 2.5 `thinkingBudget` ranges were not re-verified live.** REASON-06 names them; the 2.5 ids are retired for new users and were never probed, so they stay `Unmeasured`. Gemini ships no native Budget. Human item 3.
- **W-3 Kimi adapter-path smoke run 2 is not monotonic** (`low` 47 vs `max` 26 reasoning tokens; run 1 was 29 vs 76). The evidence file says so plainly; the effect claim for Kimi rests on the raw probe (`low` 40, `high` 48, `max` 116).
- **W-4 A provider-level default effort is not in the node-cache key**, so changing `llm.<provider>.reasoning_effort` serves stale cached answers for agents that set none; the page tells operators to rotate `key_prefix`. A documented design choice, but easy to miss.
- **W-5 The 82 percent coverage floor** cannot be checked locally (human item 1).

### Human Verification Required

1. **CI coverage job** - confirm `cargo llvm-cov --fail-under-lines 82` is green on the merged branch. Why human: the tool is not installed locally.
2. **Anthropic Budget policy** - accept override-only, or request a seeded span for the 4.6 pair. Why human: seed-rule versus requirement-wording judgement.
3. **Gemini 2.5 waiver** - accept that those ids cannot be live re-verified and remain Unmeasured. Why human: requirement text names a check that could not be done.

### Gaps Summary

Two verified gaps, one root theme: the three "pre-judging" and observability sites were written separately from the adapters' `in_force` precedence and from the engine's trace path.

- **G-1 (REASON-05):** single-agent runs emit `NodeFinished` with no effort. A two-line change plus a test.
- **G-2 (D-09/D-08, REASON-01 precedence):** the host load check inverts fallback precedence, and the builder and chain pre-flight cannot see a provider-level fallback or a profile-less hop. Every divergence refuses loudly before HTTP, so the phase's central hazard (the silent no-op) is closed, but the build-time answer can differ from the send-time answer.

Neither gap involves a vendor wire form, a seed value or a credential, and none is deferrable: Phases 46 and 47 are docs/release phases that do not cover this work. Recommend `/gsd-plan-phase 51 --gaps`.

---

_Verified: 2026-10-22_
_Verifier: Claude (gsd-verifier)_
