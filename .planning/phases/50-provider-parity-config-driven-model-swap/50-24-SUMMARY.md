---
phase: 50-provider-parity-config-driven-model-swap
plan: 24
subsystem: llm
tags: [rust, paladin-llm, token-usage, reasoning-tokens, live-probe, grok, gemini, docs, migration, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-22 50-USAGE-PROBE.md (operator run 2026-10-18, containment verdicts, Grok billed-cost cross-check); 50-21 CompatUsageSemantics and Grok's ReasoningBesideCompletion; 50-23 bespoke-adapter reconciliation, Gemini tool-use fold, Anthropic vision cache fold"
provides:
  - "Every provider's usage semantics decided by the fixed rules over 50-USAGE-PROBE.md: no flip anywhere (Grok stays ReasoningBesideCompletion, now labelled live-measured 2026-10-18 with the cost cross-check)"
  - "Evidence labels at every declaration/mapper: GrokAdapter::new, CompatUsageSemantics rustdoc, KimiAdapter::new, QwenAdapter::new, OpenAI and DeepSeek map_usage, Gemini map_gemini_usage"
  - "Measured-shape characterization tests: grok_measured_usage_prices_to_the_billed_cost (Paladin's price equals xAI's cost_in_usd_ticks), kimi_measured_usage_is_read_inside_completion, qwen_measured_usage_is_read_inside_completion, deepseek_measured_usage_is_read_inside_completion, openai_measured_usage_reconciles, gemini_measured_thoughts_are_beside_candidates"
  - "Switching providers ## Responses: the usage mapping, every fold and the vendor-total reconciliation for all nine, each reasoning cell labelled measured 2026-10-18 or not live-measured; suite size corrected 17 -> 19"
  - "MIGRATION.md 9.1 M-B-20 (plans 50-23 and 50-24); CHANGELOG.md and crates/paladin-llm/CHANGELOG.md Changed + Fixed entries; the four stale 50-23 forward references and the 50-22 'not yet live-measured' wording rewritten"
affects: [50-31]

tech-stack:
  added: []
  patterns:
    - "A semantics declaration carries its evidence label (measured-on-date with the source file, or not live-measured with why) next to the code that applies it"
    - "A live measurement that confirms existing behaviour is pinned as a characterization test built from the recorded numbers, mutation-checked against the opposite reading"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - docs/src/user-guides/switching-providers.md
    - CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "No provider's usage semantics flipped: Grok all-beside keeps ReasoningBesideCompletion; Kimi and DeepSeek all-inside keep the inclusive default; Qwen, OpenAI, Gemini, Anthropic, Ollama and OpenAI-compatible meet no flip condition"
  - "An indeterminate line (no reasoning reported) is not a containment reading, so inside + indeterminate (Qwen) or beside + indeterminate (Gemini) is not a mixed result and opens no WINDOWS row"
  - "Gemini's measured beside verdict (thoughts outside candidates) confirms the existing thoughts fold; only an inside verdict would have flipped it"

patterns-established:
  - "User docs label each provider's reasoning reading 'measured <date>' or 'not live-measured' in its existing table cell, without new columns"

requirements-completed: [PARITY-05, PARITY-07]

duration: 21min
completed: 2026-10-18
---

# Phase 50 Plan 24: Live usage verdicts applied, usage mapping documented for all nine providers (G-50-5a) Summary

**Every provider's reasoning reading now rests on the 2026-10-18 measurement or is labelled not live-measured. No
semantics flipped. Grok's reasoning-beside fold is now measured, and a new test shows the measured grok-4.3 call priced
by Paladin at exactly xAI's billed `cost_in_usd_ticks` (1_275_600 nanos = 12_756_000 ticks). The Switching providers
guide, both CHANGELOGs and MIGRATION 9.1 M-B-20 now describe the folds and the reconciliation for all nine providers.**

## Performance

- **Duration:** about 21 min
- **Started:** 2026-10-18T14:01:08Z
- **Completed:** 2026-10-18T14:22:08Z
- **Tasks:** 3 of 3. Task 1 was the tracer. Its gate passed in autonomous mode and was re-run after the commit:
  "Tracer verified end-to-end - expanding".
- **Files modified:** 11 (0 created)

## Accomplishments

- **Task 1 (tracer, `84828ef5`): branch 1 applied.** Every Grok line reads `reasoning beside completion`.
  - `ReasoningBesideCompletion` stays.
  - The `GrokAdapter::new` evidence comment now reads LIVE-MEASURED 2026-10-18 (`50-USAGE-PROBE.md`) and cites the
    billed-cost cross-check, consistent to the tick on all three models. This replaces the A2-50-21 secondary-sourced
    label. The `CompatUsageSemantics` rustdoc follows.
  - The 50-21 tracer test and the Grok conformance fixture are unchanged; only their comments were updated.
  - New test `grok_measured_usage_prices_to_the_billed_cost`. The measured grok-4.3 object (212 prompt / 128 cached
    / 1 completion / 457 reasoning / total 670) maps to completion 458 and total 670 with no finding. Priced at the
    `50-SEED-PROBE.md` listing prices ($1.25 / $0.20 / $2.50 per M), it costs 1_275_600 nanos, which is xAI's
    12_756_000 ticks.
  - Mutation check: with Grok switched to `ReasoningInsideCompletion` the test fails (`left: 1, right: 458`). The file
    was then restored.
- **Task 2 (`adcbfba4`):** applied the fixed rules to the other eight providers. No flip and no WINDOWS row (see the
  table). Evidence labels were added at each declaration or mapper, plus five measured-shape characterization tests.
  - Mutation check: Kimi switched to beside fails `kimi_measured_usage_is_read_inside_completion` (`left: 291, right:
    147`). The file was then restored.
- **Task 3 (`0567c875`):** docs, changelogs and migration.
  - `## Responses` names every fold: Grok reasoning, Gemini thoughts and tool-use prompt, and Anthropic cache read
    and write, including the vision path. It says the vendor total is compared on every adapter except Anthropic,
    and that findings go under `paladin.usage_unreconciled`.
  - Each of the nine reasoning cells carries a `measured 2026-10-18` or `not live-measured` label. Anthropic's cache
    cell notes the fold on both paths.
  - Root and crate CHANGELOG: `### Changed` (bespoke reconciliation, probe applied with no flip) and `### Fixed`
    (Gemini tool-use fold, Anthropic vision cache fold).
  - MIGRATION 9.1 M-B-20.
  - `make api-surface`: unchanged.

### Provider -> verdict lines -> decision (all nine)

| Provider | Verdict lines (`50-USAGE-PROBE.md`, 2026-10-18) | Rule applied | Decision | Label |
|---|---|---|---|---|
| grok | grok-4.3, grok-4.5, grok-4.6: reasoning beside completion | all beside | keep `ReasoningBesideCompletion` (Task 1, branch 1) | measured 2026-10-18; cost cross-check consistent |
| kimi | kimi-k2.6, kimi-k3: reasoning inside completion | all inside -> the default | keep the default `ReasoningInsideCompletion` (no chain) | measured 2026-10-18 |
| qwen | qwen3.8-flash: inside; qwen-plus: indeterminate | not all inside -> no change | keep the default | measured 2026-10-18 (qwen3.8-flash; qwen-plus reported none) |
| deepseek | deepseek-flash: inside | flip only if beside | no change (inclusive `map_usage`) | measured 2026-10-18 |
| openai | gpt-5.4-nano, gpt-4o-mini: indeterminate | flip only if beside | no change | not live-measured (0 reasoning reported) |
| gemini | gemini-3.5-flash, gemini-3.6-flash: beside; gemini-3.5-flash-lite: indeterminate | flip only if inside | no change: the thoughts fold already applies the beside reading | measured 2026-10-18 (flash-lite reported none) |
| anthropic | claude-haiku-5-5: no vendor total | A3-50-24 | no change (inclusive `thinking_tokens`, guarded by the 50-23 containment check) | not live-measured |
| ollama | llama3: not measured (not configured) | A2-50-24, never flipped | no change (engine default); adapter file not edited | not live-measured |
| openai-compatible | `<OPENAI_COMPATIBLE_MODEL>`: not measured (not configured) | A2-50-24, never flipped | no change (engine default); adapter file not edited | not live-measured |

- **WINDOWS rows appended:** none. No line reads `other`. No provider has both inside and beside lines. Ollama and
  OpenAI-compatible have no beside verdict.
- **Flipped tests:** none, because no semantics changed. The tests the plan names as "flipped" do not exist.
- **New characterization tests:** these pin one measured model per provider. They pass by construction, as the plan
  requires for unchanged providers (existing usage tests unedited).
  - `grok_measured_usage_prices_to_the_billed_cost`
  - `kimi_measured_usage_is_read_inside_completion`
  - `qwen_measured_usage_is_read_inside_completion`
  - `deepseek_measured_usage_is_read_inside_completion`
  - `openai_measured_usage_reconciles`
  - `gemini_measured_thoughts_are_beside_candidates`

## Task Commits

1. **Task 1 (tracer): Grok's usage semantics follow the recorded probe**: `84828ef5` (fix)
2. **Task 2: apply the live usage verdicts to every provider**: `adcbfba4` (fix)
3. **Task 3: document usage mapping and reconciliation for all nine providers**: `0567c875` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale evidence and suite-size text outside the plan's file list**
- **Found during:** Tasks 1 and 3
- **Issue:** Three pieces of text were stale:
  - The `CompatUsageSemantics::ReasoningBesideCompletion` rustdoc in `compat/engine.rs` still said "secondary-sourced".
  - `## Responses` said "17-case conformance suite"; `CASE_COUNT` has been 19 since 50-23.
  - MIGRATION M-B-19 and both CHANGELOG Grok entries said "plan 50-22 measures it".
- **Fix:** each now states the measurement or the correct count.
- **Commits:** `84828ef5` (engine rustdoc), `0567c875` (docs, CHANGELOGs, M-B-19)

Smaller calls, all within scope:
- **Task 2 code edits.** Task 2 is `tdd="true"`, but no rule called for a flip, so there was no red step. The plan's
  prescribed `fix(50-24): ...` commit message was kept. The commit holds the evidence comments and the
  characterization tests; production code is unchanged apart from comments.
- **Reading of "mixed".** An `indeterminate` line is treated as not a containment reading, so Qwen
  (inside + indeterminate) and Gemini (beside + indeterminate) are not "mixed", and neither opens a WINDOWS row. The
  outcome is "no change" either way. Only the ledger row depends on this reading.
- **M-B-20 table format.** The row uses the table's four cells (ID, Change, Who, Action). M-B-18 and M-B-19 inline
  "Who" and "Action" into the Change cell.
- **The four forward references from 50-23.** `CHANGELOG.md:22` and `:1298`, `crates/paladin-llm/CHANGELOG.md:17` and
  `switching-providers.md:295` were rewritten, as the 50-23 handoff asked.

## Verification

- Task 1 verify: `cargo test -p paladin-llm --features grok --lib grok::` gave 46 passed. The
  `grep -q '50-USAGE-PROBE' grok/adapter.rs` check succeeded. Both were re-run after the commit (the tracer gate).
- Task 2 verify:
  - `cargo test -p paladin-llm --all-features --lib`: 1184 passed, 0 failed (1178 before, plus 6 new).
  - `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
- Task 3 verify:
  - All three greps succeeded.
  - `sed -n '/^## 9.1/,/^## 9.2/p' MIGRATION.md | grep -c 'Phase 50 plans 50-23 and 50-24'` gave 1.
  - `cargo test -p paladin-llm --all-features --lib switching_page_tests`: 13 passed.
  - `make api-surface`: "API surface unchanged" (4298 items). No update or CHANGELOG entry was needed.
- Gates, run after the last commit:
  - `cargo fmt --check`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
  - `cargo test -p paladin-llm --all-features`: 1184 passed. `cargo test -p paladin-llm` (default features): 448
    passed. The crate sets `doctest = false`, so there are no doc tests to run.
  - `make clean-code`: exit 0. `make check-gates`: exit 0.
- **`make security`: exit 2, not caused by this plan.**
  - `cargo-audit` reports exactly 3 vulnerabilities, RUSTSEC-2026-0367/0368/0369 in `zip 3.0.0` (via
    `utoipa-swagger-ui`), plus 9 allowed warnings.
  - `make deny`, run separately: `advisories FAILED` on the same three IDs; bans, licenses and sources ok.
  - This is the open owner decision in `50-VALIDATION.md`. No `Cargo.toml` or `Cargo.lock` changed.
- **Diff check for `unwrap(` / `expect(` / `panic!(`:** every added occurrence is inside the six new
  `#[cfg(test)]` test functions. Non-test code changed only in comments and rustdoc.
- **Manual credential-handling review:** no credential path was touched. There is no HTTP client, redirect policy,
  header, log statement or redaction change. The test bodies use the fixed `"test-key"` mock credential, as the
  surrounding tests do. The docs and changelogs carry token counts, tick figures and dates only, which are billing
  metadata (T-50-24-03, accepted).
- Not run:
  - A live vendor call. No key exists here; the 50-22 operator transcript is the evidence.
  - Docker-bound integration tests.
  - The paladin-llm example tests, which this plan did not touch. They ran green under 50-22 Task 3: 33 passed.
- Environment: about 7 GB of disk was free after the gates (83% used), and there was no ENOSPC.

## Known Stubs

None.

## Threat Flags

None. Against the plan's register:
- T-50-24-01: fixed rules only. No flip was made, and no mixed result exists.
- T-50-24-02: every provider is labelled measured or not live-measured, in code and in the guide.
- T-50-24-03: accepted, billing metadata only.

## Flagged assumptions

- A1-50-24 held: semantics stay per provider. No provider's models disagreed, so no per-model row was needed.
- A2-50-24 held: Ollama and OpenAI-compatible were not configured. Their adapter files were not edited, and their
  lines are recorded above and labelled not live-measured in the guide.
- A3-50-24 held: Anthropic is `no vendor total`. The inclusive `thinking_tokens` reading stays, and the probe's 61
  thinking tokens within 64 output tokens agree with it.

## Self-Check: PASSED

- All 11 modified files exist.
- Commits `84828ef5`, `adcbfba4` and `0567c875` exist on `claude/laughing-dirac-e0h2ax`.
- `grep -c '50-USAGE-PROBE' crates/paladin-llm/src/grok/adapter.rs` is at least 1, and kimi/adapter.rs names
  `CompatUsageSemantics`.
