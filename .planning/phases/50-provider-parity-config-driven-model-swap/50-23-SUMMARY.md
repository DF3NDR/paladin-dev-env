---
phase: 50-provider-parity-config-driven-model-swap
plan: 23
subsystem: llm
tags: [rust, paladin-llm, gemini, openai, deepseek, anthropic, token-usage, conformance, gap-closure, g-50-5a, adr-0061, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-21 usage_reconcile (USAGE_UNRECONCILED_METADATA_KEY, reconcile, UsageLog) and its compat wiring; 50-18 terminal-chunk metadata stamp; 50-15 shared conformance suite"
provides:
  - "Gemini: GeminiUsageMetadata reads toolUsePromptTokenCount (Option, serde default) and folds it into prompt_tokens (saturating); totalTokenCount is Option<u32> and read; map_gemini_usage returns (TokenUsage, Option<u32>); buffered parse and the Ok streamed finish-reason chunk record findings through a per-instance UsageLog"
  - "OpenAI and DeepSeek: total_tokens is Option<u32> with a serde default and read; map_usage returns the vendor total; UsageLog on the buffered response and the Ok [DONE] terminal chunk"
  - "Anthropic: UsageLog with vendor_total None on both paths (containment findings only)"
  - "Anthropic vision: ClaudeVisionUsage gains cache_read/cache_creation_input_tokens (Option, serde default); map_vision_usage folds them into prompt_tokens on both vision surfaces"
  - "conformance: usage_fields_are_mapped asserts reasoning <= completion, cache <= prompt and no paladin.usage_unreconciled; ConformanceFixture::disagreeing_usage_body (default via wire_defaults::disagreeing_usage_body, DISAGREEMENT = 1000); case vendor_total_disagreement_is_recorded on all nine adapters; CASE_COUNT 19"
  - "Tests: gemini_tool_use_prompt_tokens_reach_prompt_tokens (tracer), gemini_absent_total_and_tool_use_map_as_before_with_no_finding, gemini_stream_terminal_usage_is_folded_and_reconciled, anthropic_vision_usage_counts_cache_tokens, openai/deepseek/anthropic usage_reconciliation modules (12 tests)"
affects: [50-24, 50-27, 50-31]

tech-stack:
  added: []
  patterns:
    - "Every bespoke adapter holds a UsageLog beside its OmissionLog and records on the buffered response and the Ok terminal chunk only"
    - "A held vendor total beside the held usage, taken at the terminal chunk, mirroring compat's held_vendor_total"
    - "A conformance default derived from the fixture's own body (disagreeing_usage_body) so no fixture override is needed"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/anthropic/vision.rs
    - crates/paladin-llm/src/conformance.rs
    - crates/paladin-llm/src/usage_reconcile.rs

key-decisions:
  - "D-02 kept (owner): no vendor total is ever copied; total_tokens stays prompt + completion on all nine adapters"
  - "The usage log keys warnings on the request model on every adapter, matching OmissionLog and 50-21's compat wiring (Anthropic's stream keeps a separate usage_model because its model variable is replaced by the response model)"
  - "Gemini, DeepSeek generate merge the request notes into the response metadata (extend) rather than replace it, so a finding recorded during parsing survives"
  - "Documentation of the new reconciliation (switching-providers, CHANGELOGs, MIGRATION) is left to plan 50-24, which the plan assigns it to"

patterns-established:
  - "All nine adapters answer vendor_total_disagreement_is_recorded; a new adapter inherits it through ConformanceFixture's default"

requirements-completed: [PARITY-05, PARITY-07]

duration: 37min
completed: 2026-10-18
---

# Phase 50 Plan 23: Gemini tool-use fold, vendor totals reconciled on the bespoke adapters, Anthropic vision cache counts (G-50-5a) Summary

**Gemini now counts its tool-use prompt tokens in `prompt_tokens` (debug repro 100/20/300/10 maps to prompt 400, completion 30, total 430, agreeing with `totalTokenCount`). OpenAI, DeepSeek and Gemini check their own vendor total on both paths and record a disagreement under `paladin.usage_unreconciled`; Anthropic runs the containment checks; the Anthropic vision path counts cache tokens. One conformance case proves the disagreement is surfaced on all nine adapters. D-02 kept.**

## Performance

- **Duration:** about 37 min
- **Started:** 2026-10-18T11:26:23Z
- **Completed:** 2026-10-18T12:02:59Z
- **Tasks:** 2 of 2 (Task 1 was the tracer. Its gate passed in autonomous mode, re-run after the commit: "Tracer verified end-to-end - expanding")
- **Files modified:** 7 (0 created)

## Accomplishments

- **Gemini (Task 1, tracer):** `toolUsePromptTokenCount` is deserialised and folded into `prompt_tokens`, so the `map_gemini_usage` rustdoc's "agrees with `totalTokenCount = prompt + candidates + toolUsePrompt + thoughts`" claim is now true (the rustdoc says so). `totalTokenCount` is `Option<u32>`, read, and never copied (the dead-code allow is gone). `parse_response` records any finding. The stream path holds the finish-reason frame's total and reconciles it in a new `reconcile_terminal_usage` pass, which runs after `classify_stream_items`, so an `Err` terminal carries no metadata. The path is latent today because `GeminiRequest` sends no tools.
- **OpenAI and DeepSeek (Task 2):** `total_tokens` is `Option<u32>` with `#[serde(default)]`, so a usage object without it now parses (A2-50-23). The total is reconciled on the buffered response and on the `[DONE]` terminal chunk next to the 50-18 metadata stamp. DeepSeek's `annotate_with_usage` still compiles and renders an absent total as `total_tokens=not reported`. Plan 50-27 removes that function.
- **Anthropic:** records with `vendor_total = None` on both paths. A `thinking_tokens` count above `output_tokens` gives `reasoning:reasoning=25,completion=10`, and a normal response gives no key.
- **Anthropic vision:** `ClaudeVisionUsage` gains both cache fields. `map_vision_usage` folds them as `map_claude_usage` does and sets each sub-count only when reported (D-03). Both vision surfaces (`LlmResponse.usage` and `VisionTokenUsage`) use it, so they report the same billed input figure.
- **Conformance:**
  - `usage_fields_are_mapped` now also asserts containment and the absence of the key. All nine fixtures already satisfied the new assertions, so there was nothing to relax or fix.
  - The Gemini wire-default usage body carries 50 tool-use tokens: prompt 150, total 190.
  - The new `vendor_total_disagreement_is_recorded` case runs through `disagreeing_usage_body()`, whose default raises `usage.total_tokens` or `usageMetadata.totalTokenCount` by 1000 and is `None` for Anthropic.
  - `CASE_COUNT` is 19. The macro rustdoc's stale `17` / "eight behaviour cases" now reads `19` / "nine".

## Task Commits

1. **Task 1 (tracer): Gemini tool-use fold, Gemini total reconciled, containment assertions** - `73e30731` (fix)
2. **Task 2: OpenAI/DeepSeek/Anthropic reconciliation, vision cache counts, disagreement case** - `a395496f` (feat)

Red first:
- Task 1: the tracer and the Gemini stream test failed before the fold (prompt 100, total 130). The absent-total test passed as written, because it pins unchanged behaviour.
- Task 2: `vendor_total_disagreement_is_recorded` failed on OpenAI and DeepSeek and passed on the seven others (compat from 50-21, Gemini from Task 1, Anthropic's `None` arm). The DeepSeek struct-shape tests and the vision test failed to compile before the implementation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `usage_reconcile.rs` dead-code `cfg_attr` extended (file not in the plan's list)**
- **Found during:** Task 1, then Task 2
- **Issue:** The module allowed dead code unless a compat feature was on. Its comment said the bespoke adapters would join in 50-23. Without the extension, the list stays stale and a bespoke-only build is not checked for real dead code.
- **Fix:** Task 1 added `gemini`. Task 2 added `openai`, `deepseek` and `anthropic`, and rewrote the comment.
- **Commits:** `73e30731`, `a395496f`

**2. [Rule 1 - Bug] Buffered metadata merged, not replaced, on Gemini and DeepSeek**
- **Found during:** Task 1 (Gemini), Task 2 (DeepSeek)
- **Issue:** Both adapters build the response inside the retry closure and then overwrote `response.metadata` with the request notes. That would drop a finding recorded during parsing.
- **Fix:** `response.metadata.extend(notes.into_metadata())`. The keys cannot collide. OpenAI and Anthropic record after the notes are set, so they needed no change.
- **Commits:** `73e30731`, `a395496f`

Smaller calls, all within scope:
- `parse_sse_lines` (Gemini) takes a `&mut Option<u32>` out-parameter for the held total instead of a log, so warning and recording stay on the `Ok` terminal only.
- Anthropic's stream keys the log on a separate `usage_model` (the request model), because its `model` variable is overwritten with the response model at `message_start`.
- Extra tests beyond the plan's named ones: the absent-total and stream tests for Gemini, and a `usage_reconciliation` test module each for OpenAI (4 tests), DeepSeek (5, one of which covers the absent-total annotation) and Anthropic (3).

## Verification

- Tracer gate: `cargo test -p paladin-llm --features gemini --lib gemini_tool_use_prompt_tokens_reach_prompt_tokens` gave 1 passed. `--all-features --lib gemini::` gave 122 passed. `--all-features --lib conformance_suite` gave 162 passed, with `usage_fields_are_mapped` 9/9. All of these were re-run after the Task 1 commit.
- Task 2 verify: `--all-features --lib` gave `openai::` 93, `deepseek::` 93, `anthropic::` 114 and `conformance` 182, all passed, 0 failed.
- `cargo test -p paladin-llm --all-features --lib vendor_total_disagreement_is_recorded`: **10 passed, not 9.** The filter also matches 50-21's adapter-level `compat::engine::tests::compat_vendor_total_disagreement_is_recorded`. The conformance case itself (`conformance_suite::vendor_total_disagreement_is_recorded`) runs 9 and passes 9.
- `cargo test -p paladin-llm --all-features` gave 1178 passed, 0 failed. `cargo test -p paladin-llm` (default features) gave 447 passed. The crate sets `doctest = false`, so there are no paladin-llm doc tests to run.
- `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo clippy -p paladin-llm -- -D warnings` lib builds are clean under default, no-default, all-features, and openai-, deepseek-, anthropic-, gemini- and kimi-only.
- `make clean-code`: exit 0. `make check-gates`: exit 0. `make api-surface`: "API surface unchanged" (4298 items), so no `make api-surface-update` or CHANGELOG entry was needed. Every new item is crate-private or `#[cfg(test)]`.
- **`make security`: exit 2, not caused by this plan.** `cargo-audit` fails on RUSTSEC-2026-0367/0368/0369 in `zip 3.0.0` (via `utoipa-swagger-ui` / `paladin-web`), and `make deny`, run separately, fails `advisories` on the same three (bans, licenses and sources ok). This is already an open owner decision in `50-VALIDATION.md`. This plan changed no `Cargo.toml` or `Cargo.lock`.
- Not run:
  - The credential-gated live smoke, which needs vendor keys. Plan 50-22 owns the live usage probe, and its operator run is in progress elsewhere; no 50-22 file was touched.
  - Docker-bound integration tests.
- Pre-existing failures: single-feature `--all-targets` test builds fail clippy (`cadence::PinnedSpread`, `fallback::ProfiledStubPort`, the `provider_factory` `chat` variable). These are already in `deferred-items.md` and are in files this plan did not touch.
- Environment note: midway through the gates the shared disk filled (ENOSPC on tool output). Deleting `target/debug/incremental` freed the space, and every gate above was run after that.
- **Diff check for `unwrap(` / `expect(` / `panic!(`:** every added occurrence is inside a `#[cfg(test)]` module or in `conformance.rs`, which is compiled only under `cfg(test)`. None were added to library code.
- **Manual credential-handling review (T-50-23-05):** the new `warn!` lines come only from `UsageLog::record` (50-21) and interpolate the provider name, the request model id and the finding (fixed check names plus `u32` counts). The metadata values are the same finding strings. DeepSeek's annotation adds only the literal `not reported`. No API key, URL, header, response body or content reaches any of them. No HTTP client, redirect policy or redaction path was touched.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or trust-boundary schema. Against the plan's register:
- T-50-23-01: the tool-use count is folded, covered by the tracer and the Gemini conformance body (prompt 150).
- T-50-23-02: `UsageLog` is wired on OpenAI, DeepSeek and Gemini on both paths, with containment checks on Anthropic, and the conformance case runs on all nine.
- T-50-23-04: every fold saturates, and absent totals are `None`.
- T-50-23-05: values carry numbers, provider and model only, as reviewed above.

## Flagged assumptions

- A1-50-23 held: the tool-use fold follows Gemini's documented arithmetic and is fixture-tested only. No tools are sent today.
- A2-50-23: `total_tokens` / `totalTokenCount` are now optional on OpenAI, DeepSeek and Gemini, so a usage object without them parses. Plan 50-24's MIGRATION 9.1 row records this.
- A3-50-23 held: Anthropic's `thinking_tokens` mapping is unchanged, and `reasoning <= completion` is now its runtime guard.

## Handoff to plan 50-24

Four forward references now describe the pre-50-23 state, and plan 50-24 rewrites the usage-mapping documentation anyway:
- `CHANGELOG.md:22` and `:1298` ("bespoke adapters follow in plan 50-23")
- `crates/paladin-llm/CHANGELOG.md:17`
- `docs/src/user-guides/switching-providers.md:295` ("plan 50-23 extends the check to them")

The new behaviour also belongs in the Responses usage mapping: the Gemini tool-use fold, the Anthropic vision cache fold, reconciliation on the four bespoke adapters, and the optional totals.

## Self-Check: PASSED

- Every modified file listed above exists.
- Commits `73e30731` and `a395496f` exist on `claude/laughing-dirac-e0h2ax`.
- Acceptance greps:
  - `toolUsePromptTokenCount` in `gemini/adapter.rs` = 6 (needs at least 2).
  - `usage_log` count is 5 in openai, 5 in deepseek, 5 in anthropic and 7 in gemini (needs at least 2 each).
  - `CASE_COUNT, 19` in `conformance.rs` = 1.
  - `cargo test -p paladin-llm --all-features --lib` exits 0.
