---
phase: 50-provider-parity-config-driven-model-swap
plan: 21
subsystem: llm
tags: [rust, paladin-llm, grok, token-usage, treasurer, gap-closure, g-50-5a, adr-0061, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-12 compat usage mapping and the D-02 recompute; 50-18 terminal-chunk metadata stamp and the run_metadata merge rule; 50-20 (wave order)"
provides:
  - "crates/paladin-llm/src/usage_reconcile.rs: pub const USAGE_UNRECONCILED_METADATA_KEY = \"paladin.usage_unreconciled\"; pub(crate) fn reconcile (total, reasoning, cache findings in fixed order, saturating); pub(crate) struct UsageLog (warn once per instance per (model, check), target paladin::usage)"
  - "pub(crate) enum compat::engine::CompatUsageSemantics { ReasoningInsideCompletion (default), ReasoningBesideCompletion } and pub(crate) fn CompatEngine::with_usage_semantics; CompatEngineConfig unchanged"
  - "map_compat_usage folds reasoning per the preset's semantics and returns the vendor total; buffered generate and the Ok stream terminal chunk record any finding under paladin.usage_unreconciled"
  - "Grok declares ReasoningBesideCompletion (secondary-sourced, not live-measured; plans 50-22/50-24)"
  - "Tests grok_reasoning_tokens_reach_completion_and_the_price (tracer, 2_403_000 nanos through PricingLlmAdapter), grok_stream_terminal_usage_is_folded_and_reconciled, compat_vendor_total_disagreement_is_recorded (flipped from compat_usage_fields_are_copied_exactly), usage_log_warns_once_per_model_and_check, reconcile table tests; Grok conformance usage fixture xAI-shaped"
  - "Switching providers keys table and Responses section, both CHANGELOGs, MIGRATION 9.1 M-B-19, cost_of_call rustdoc on the T-38-05 clamp"
affects: [50-22, 50-23, 50-24, 50-31]

tech-stack:
  added: []
  patterns:
    - "Per-preset vendor semantics declared through a crate-private engine builder (with_usage_semantics), never branched on provider identity"
    - "A warn-once log behind an Arc so a 'static stream closure records into the same adapter instance's log"
    - "The vendor's own total as the check on a mapping: parsed, compared, surfaced under a paladin.* key, never copied over the D-02 total"

key-files:
  created:
    - crates/paladin-llm/src/usage_reconcile.rs
  modified:
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-core/src/platform/container/cost.rs
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "D-02 kept (owner, A1-50-21): no vendor_total_tokens field on TokenUsage; a disagreement is surfaced through the ADR-0061 channel (warn once plus paladin.usage_unreconciled), the only new public item being the key constant"
  - "Grok's reasoning-beside-completion reading ships on secondary sources (Vertex AI partner page, Glama doc), labelled not live-measured; a wrong declaration is self-revealing through the total finding"
  - "The usage log keys warnings on the request model, matching OmissionLog, on both paths"

patterns-established:
  - "usage_reconcile is the shared check plan 50-23 extends to the four bespoke adapters"

requirements-completed: [PARITY-05, PARITY-07]

duration: 29min
completed: 2026-10-18
---

# Phase 50 Plan 21: Grok reasoning tokens counted and priced; compat usage reconciled (G-50-5a) Summary

**A Grok reasoning call (prompt 663 / completion 50 / reasoning 124 / vendor total 837) now maps to completion 174 and total 837 and is priced at 2_403_000 nanos at $1/$10 per M, what xAI bills (it was 1_163_000). Every compat preset parses the vendor's own total on both paths and records any disagreement or broken containment under `paladin.usage_unreconciled` instead of discarding it, with D-02 kept.**

## Performance

- **Duration:** about 29 min
- **Started:** 2026-10-18T02:48:01Z
- **Completed:** 2026-10-18T03:17:37Z
- **Tasks:** 3 of 3 (Task 1 was the tracer. Its gate passed in autonomous mode: "Tracer verified end-to-end - expanding")
- **Files modified:** 10 (1 created)

## Accomplishments

- `usage_reconcile.rs` holds the module contract (inclusive `TokenUsage`, mappers fold, this module only checks, nothing overwrites `total_tokens` or attributes a residual), the key constant documented in the `PARAMS_OMITTED_METADATA_KEY` style with its value grammar, the pure `reconcile` and the warn-once `UsageLog`.
- `CompatUsageSemantics` declares each preset's reading of `completion_tokens`. Grok alone declares `ReasoningBesideCompletion` through `with_usage_semantics`. Kimi, Qwen, Ollama and OpenAI-compatible keep the default, pinned by the shared OpenAI-shaped conformance body, and Grok's override body (25 + 15 -> 40) pins the fold by behaviour.
- `CompatUsage.total_tokens` is now read (dead-code allow removed). `map_compat_usage` returns `(TokenUsage, Option<u32>)`. The buffered `generate` and the `Ok` stream terminal chunk insert the finding when there is one. Content chunks and `Err` terminals carry none.
- `compat_usage_fields_are_copied_exactly` (which pinned the discard of vendor 9999 against 1801) was flipped on purpose and renamed `compat_vendor_total_disagreement_is_recorded`: the total stays 1801 and the metadata reads `total:vendor=9999,mapped=1801`.
- Docs: the keys table names `paladin.usage_unreconciled` with the last-value merge rule, the values sentence includes token counts, the grok Responses cell states the fold and that it is not yet live-measured, and the vendor-total sentence is worded per preset family (compat reconciled, the four bespoke adapters in 50-23). Both CHANGELOGs gained Fixed and Added entries, MIGRATION 9.1 gained row M-B-19, and `cost_of_call`'s rustdoc explains why the T-38-05 clamp stays (rustdoc-only diff).

## Task Commits

1. **Task 1 (tracer): fold Grok reasoning, the key, `reconcile` (total), `UsageLog`, buffered reconciliation** - `4bbdf297` (fix)
2. **Task 2: reasoning and cache findings, stream terminal reconciliation, flipped compat test, warn-once test** - `bea18f39` (test)
3. **Task 3: docs, CHANGELOGs, MIGRATION M-B-19, `cost_of_call` rustdoc** - `85f261b8` (docs)

Red-first: the tracer failed before the fold (completion 50, total 713). In Task 2 five of the six new tests failed red (the reasoning and cache findings, the u32::MAX case, the warn-once-per-check count, and the Grok stream fold). The flipped `compat_vendor_total_disagreement_is_recorded` already passed when written, because Task 1 had wired the buffered recording.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Grok conformance usage override moved from Task 2 into Task 1**
- **Found during:** Task 1
- **Issue:** Declaring `ReasoningBesideCompletion` made the shared OpenAI-shaped body map to completion 55 for Grok, so `grok::...::conformance_suite::usage_fields_are_mapped` failed and the Task 1 commit would have left the lib suite red.
- **Fix:** Added the plan's xAI-shaped `GrokFixture::usage_body`/`expected_usage` override (prompt 100, completion 25, reasoning 15, cached 30, total 140 -> completion 40) in Task 1. Task 2's acceptance (`grep -c 'fn usage_body'` = 1) still holds.
- **Commit:** `4bbdf297`

**2. [Rule 3 - Blocking] `usage_reconcile`'s dead-code `cfg_attr` lists the five compat features, not sampling's nine**
- **Found during:** Task 1 (clippy)
- **Issue:** Copying sampling's nine-feature list would leave `reconcile`/`UsageLog` unused, and `-D warnings` red, in a default (`openai`) build, because only the compat engine consumes them until plan 50-23.
- **Fix:** The module's `cfg_attr` names kimi, qwen, grok, ollama and openai-compatible, with a comment that 50-23 brings in the bespoke adapters. `CompatUsageSemantics::ReasoningBesideCompletion` and `with_usage_semantics` carry `#[cfg_attr(not(feature = "grok"), allow(dead_code))]`, because a kimi-only lib build failed clippy without it.
- **Commit:** `4bbdf297`

**3. [Rule 1 - Bug/structure] `UsageLog::record` goes through `reconcile`**
- **Found during:** Task 1 (clippy: `reconcile` never used outside tests)
- **Fix:** `record` calls `reconcile` and keys each warning on the finding's check-name prefix, so the value grammar has a single source.
- **Commit:** `4bbdf297`

Smaller calls, all within scope: `map_compat_usage`'s rustdoc was rewritten in Task 1 rather than Task 2, since its signature changed there. Root CHANGELOG known limitation (4) ("a vendor's own total is not copied") was updated to describe the compat reconciliation. In Task 1 the stream path kept the inclusive reading explicitly until Task 2 switched it to the engine's semantics.

## Verification

- Tracer gate: `cargo test -p paladin-llm --features grok --lib grok_reasoning_tokens_reach_completion_and_the_price` passed (re-run after commit), and `cargo test -p paladin-llm --all-features --lib usage_reconcile` gave 7 passed.
- `cargo test -p paladin-llm --all-features --lib`: 1153 passed, 0 failed. `compat::` 78 passed. `conformance_suite` 162 passed. `conformance_suite::usage_fields_are_mapped` passed on all 9 fixtures. `switching_page_tests` 13 passed.
- `cargo test -p paladin-llm` (default features): 442 passed.
- `cargo test -p paladin-ai-core`: 713 + 141 (38 ignored) + 11 passed. `cargo test -p paladin-ai-core --doc cost_of_call`: 1 passed.
- `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
- `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo clippy -p paladin-llm -- -D warnings` lib builds are clean under default, all-features, and kimi-only, grok-only and openai-compatible-only.
- `make clean-code`: exit 0. `make check-gates`: exit 0. `make api-surface`: "API surface unchanged" (4298 items), so no `make api-surface-update` was needed. The new constant lives in `paladin-llm` and does not reach the facade baseline.
- **`make security`: exit 2, not caused by this plan.** It fails on RUSTSEC-2026-0367/0368/0369 in `zip 3.0.0`, a build-dependency of `utoipa-swagger-ui` via `paladin-web`. This failure was already recorded in `50-VALIDATION.md` as an open owner decision. This plan changed no `Cargo.toml` or `Cargo.lock`.
- Not run: the credential-gated live smoke (needs vendor keys; plan 50-22 owns the live usage probe), and Docker-bound integration tests. The single-feature `--all-targets --no-default-features` test-build warnings (`cadence::PinnedSpread`, the `provider_factory` `chat` variable, `fallback::ProfiledStubPort`) were already in `deferred-items.md` and are in files this plan did not touch.
- Diff check for `unwrap(` / `expect(` / `panic!(`: every added occurrence is inside a `#[cfg(test)]` module.
- **Manual credential-handling review:** the new `warn!` line interpolates only the provider name, the model id, and the rendered finding (check name and integers). The metadata value is built only from `format!` over `u32` counts and fixed check names. No API key, URL, header, response body or content reaches either. No HTTP client, redirect policy or redaction path was touched.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or trust-boundary schema. The plan's register is mitigated:
- T-50-21-01: the fold is in place, and the tracer asserts the exact vendor-equal cost.
- T-50-21-02: the vendor total is compared on both paths.
- T-50-21-03: every sum saturates, covered by `u32::MAX` table tests.
- T-50-21-04: each instance warns once per (model, check).
- T-50-21-05: the key and the warn line carry numbers only, as reviewed above.
- T-50-21-06: the Grok rustdoc, docs, CHANGELOG and MIGRATION say "secondary-sourced, not live-measured".

## Flagged assumptions

- A1-50-21 held: `TokenUsage` is unchanged and the only new public item is the key constant.
- A2-50-21 remains open by design: Grok's reasoning-beside-completion reading is secondary-sourced. Plan 50-22 measures it live and plan 50-24 applies the verdict. If Grok turns out to be inclusive, every reasoning call will carry `total:vendor=<p+c>,mapped=<p+c+r>`.
- A3-50-21: no metrics dependency was added. The warn line plus the metadata key cover UAT missing item 2.
- A4-50-21: `paladin.usage_unreconciled` is not a union key, so a run's `run_metadata` keeps the last call that recorded one (stated in the docs).
- A5-50-21: the T-38-05 clamp code is unchanged; only its rustdoc changed.

## Self-Check: PASSED

- `crates/paladin-llm/src/usage_reconcile.rs` exists, and every modified file listed above exists.
- Commits `4bbdf297`, `bea18f39` and `85f261b8` exist on `claude/laughing-dirac-e0h2ax`.
- Acceptance greps: the key constant line = 1; `with_usage_semantics(CompatUsageSemantics::ReasoningBesideCompletion)` in grok = 1; `allow(dead_code)` in `compat/types.rs` went from 4 to 3; `fn compat_usage_fields_are_copied_exactly` = 0; `fn compat_vendor_total_disagreement_is_recorded` = 1; `fn usage_body` in grok = 1; MIGRATION 9.1 `Phase 50 plan 50-21` = 1.
