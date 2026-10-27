---
phase: 50-provider-parity-config-driven-model-swap
plan: 22
subsystem: llm
tags: [rust, paladin-llm, live-probe, token-usage, reasoning-tokens, grok, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-17 live_vendor_smoke --probe harness (run_probe_call, parse_success, diagnostic_excerpt bodies); 50-SEED-PROBE.md seeded ids and Grok listing prices; 50-20 provider specs"
provides:
  - "live_vendor_smoke --usage: one reasoning-eliciting call per candidate model over the raw vendor wire (1024-token cap under cap_field, no acceptance_extras, no sampling), numbers-only usage table and a fixed-grammar containment verdict per model; --usage --dry-run offline for all nine providers; exclusive with --probe"
  - "Example functions usage_row, containment_verdict (signed difference, i128), usage_cells, usage_candidates, render_usage_dry_run, usage_table_cells"
  - "Tests containment_verdicts_follow_the_arithmetic, usage_values_are_numbers_only and six more in the example's test module (33 example tests total)"
  - "50-USAGE-PROBE.md: operator-run 2026-10-18 transcript (7 of 9 measured), 16 containment verdict lines, Grok billed-cost cross-check consistent to the tick on grok-4.3/4.5/4.6"
affects: [50-24, 50-31]

tech-stack:
  added: []
  patterns:
    - "A live measurement records numbers only; verdicts come from fixed arithmetic and are copied, never edited"
    - "Operator-run evidence is committed raw, then folded verbatim into a dated, scrubbed record in one commit that deletes the raw file"

key-files:
  created:
    - .planning/phases/50-provider-parity-config-driven-model-swap/50-USAGE-PROBE.md
  modified:
    - crates/paladin-llm/examples/live_vendor_smoke.rs

key-decisions:
  - "Grok reasoning containment is live-measured 2026-10-18: reasoning beside completion on grok-4.3/4.5/4.6, and xAI bills reasoning at the completion rate (cost_in_usd_ticks matches to the tick, with listing prices read as ticks per token, 1 tick = 1e-10 USD)"
  - "Ollama and OpenAI-compatible were not configured by the operator; their lines read not measured (not configured)"

patterns-established:
  - "Usage-containment verdict grammar: reasoning inside completion | reasoning beside completion | indeterminate (no reasoning reported) | no vendor total | other (total - (prompt + completion) = <n>) | not measured (<reason>)"

requirements-completed: [PARITY-05, PARITY-07]

duration: "Task 1 2026-10-17 (separate session); Task 3 about 10 min"
completed: 2026-10-18
---

# Phase 50 Plan 22: Live usage-containment probe (G-50-5a) Summary

**`live_vendor_smoke --usage` records each vendor's usage arithmetic, numbers only. The operator ran it on 2026-10-18
against 7 of 9 providers. The results: Grok's reasoning sits beside `completion_tokens` (grok-4.3 212 + 1 + 457 = 670)
and is billed at the completion rate, matching `cost_in_usd_ticks` exactly. DeepSeek, Kimi and Qwen's
qwen3.8-flash report reasoning inside completion. Gemini's thoughts sit beside candidates. OpenAI, qwen-plus and
gemini-3.5-flash-lite reported no reasoning.**

## Performance

- **Duration:** Task 1 was executed and committed on 2026-10-17 (`8b113aea`, 19:09Z). Task 2 was the operator's run,
  committed as `b249d25e`. Task 3 took about 10 minutes (2026-10-18 13:56Z to about 14:06Z).
- **Tasks:** 3 of 3. Task 1 was the tracer; Task 2 was a `checkpoint:human-action`, resumed on the operator's signal
  "usage probe committed".
- **Files:** 1 created (`50-USAGE-PROBE.md`), 1 modified (`live_vendor_smoke.rs`). `50-USAGE-PROBE.raw.md` was deleted
  in Task 3, as the plan requires.

## Accomplishments

- **Task 1 (tracer, `8b113aea`):** added the `--usage` mode, all offline-tested.
  - `usage_row` reads integer fields only, for each wire: OpenAI chat (including Grok `cost_in_usd_ticks` and
    `num_sources_used`), Anthropic and Gemini.
  - `containment_verdict` emits the fixed grammar with a signed difference.
  - `--usage --dry-run` prints nine provider sections without credentials.
  - Calls go through the shared `run_probe_call`: 120 s timeout, redirects refused, error bodies through
    `diagnostic_excerpt`.
- **Task 2 (operator, `b249d25e`):** one call per seeded model. All 15 calls on the 7 configured providers returned
  `ACCEPTED (200)`. Ollama and OpenAI-compatible were `SKIPPED` (not configured).
- **Task 3 (`4d76c807`):** wrote `50-USAGE-PROBE.md`.
  - Provenance header, 16 `### Containment verdicts` lines copied from the transcript rows, Gemini/Anthropic/
    indeterminate reading notes, the Grok billed-cost cross-check, and the transcript embedded verbatim (diff-checked
    against the raw file before deletion).
  - The raw file is removed in the same commit.

### Verdict lines written (`50-USAGE-PROBE.md`, `### Containment verdicts`)

- openai gpt-5.4-nano: indeterminate (no reasoning reported) (2026-10-18)
- openai gpt-4o-mini: indeterminate (no reasoning reported) (2026-10-18)
- anthropic claude-haiku-5-5: no vendor total (2026-10-18)
- deepseek deepseek-flash: reasoning inside completion (2026-10-18)
- kimi kimi-k2.6: reasoning inside completion (2026-10-18)
- kimi kimi-k3: reasoning inside completion (2026-10-18)
- qwen qwen3.8-flash: reasoning inside completion (2026-10-18)
- qwen qwen-plus: indeterminate (no reasoning reported) (2026-10-18)
- grok grok-4.3: reasoning beside completion (2026-10-18)
- grok grok-4.5: reasoning beside completion (2026-10-18)
- grok grok-4.6: reasoning beside completion (2026-10-18)
- gemini gemini-3.5-flash-lite: indeterminate (no reasoning reported) (2026-10-18)
- gemini gemini-3.5-flash: reasoning beside completion (2026-10-18)
- gemini gemini-3.6-flash: reasoning beside completion (2026-10-18)
- ollama llama3: not measured (not configured) (2026-10-18)
- openai-compatible <OPENAI_COMPATIBLE_MODEL>: not measured (not configured) (2026-10-18)

### Grok billed-cost cross-check

Unit assumption: listing prices are read as ticks per token, with 1 tick = 1e-10 USD. Short-context prices apply, and
cached tokens are a subset of prompt.

Expected cost = (prompt − cached) × prompt price + cached × cached price + (completion + reasoning) × completion price.

| model | expected | reported | result |
|---|---|---|---|
| grok-4.3 | 84 × 12500 + 128 × 2000 + 458 × 25000 = 12 756 000 | 12 756 000 | consistent |
| grok-4.5 | 130 × 20000 + 384 × 3000 + 130 × 60000 = 11 552 000 | 11 552 000 | consistent |
| grok-4.6 | 144 × 20000 + 512 × 5000 + 274 × 60000 = 21 880 000 | 21 880 000 | consistent |

Without the reasoning tokens the expected figures would be 1 331 000, 3 812 000 and 5 500 000 ticks, which do not
match. Reasoning is billed at the completion rate.

## Task Commits

1. **Task 1 (tracer): `--usage` mode and its offline tests**: `8b113aea` (feat)
2. **Task 2 (checkpoint:human-action): operator usage probe transcript**: `b249d25e` (chore, by the operator)
3. **Task 3: validated, scrubbed `50-USAGE-PROBE.md`; raw file removed**: `4d76c807` (docs)

## Deviations from Plan

### Auto-fixed Issues

None in Task 3.

Smaller calls, all within scope:
- Task 1 computes the signed difference in `i128`, not the plan's `i64`. That is wider and still free of underflow,
  and the grammar is unchanged.
- Task 1's commit carries a `Claude Sonnet 5.5` co-author trailer from the session that executed it. Task 3's commit
  carries the trailer this session was given.
- The Ollama and OpenAI-compatible verdict lines name the dry run's default model placeholders (`llama3`,
  `<OPENAI_COMPATIBLE_MODEL>`), because the operator passed no `--models` for them.
- The header's scrub description avoids writing the literal secret-marker strings, so a later scan of the file does
  not flag the file's own prose.

## Verification

- Task 1 verify, re-run in this session:
  - `cargo test -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible`:
    33 passed, 0 failed, including `containment_verdicts_follow_the_arithmetic` and `usage_values_are_numbers_only`.
  - `--usage --dry-run` with every credential variable unset: exit 0, 9 `## ` sections.
- Task 2 verify: the raw transcript had 9 `## ` sections (it is now deleted per Task 3).
- Task 3 verify (the plan's automated command): exit 0.
  - `### Containment verdicts` is present, and the grok-4.3 line matches the grammar.
  - No `sk-…{20,}`, `AIza…{30,}` or `xai-…{20,}` string is in the file, and the raw file is gone.
  - Every seeded Grok, Kimi and Qwen model has exactly one verdict line (1 each).
- Scrub re-run (orchestrator asked):
  - The raw file and the final file were scanned for every `redact_secret_patterns` marker (`Bearer `/`bearer `,
    `sk-`, `AKIA`, `key=`, `token=`, JWT triples), plus `AIza` and `xai-`. No match in either.
  - None of the nine credential variables was set in this environment, so there was no literal value to check. No
    URL query string appears.
- `cargo fmt --check`: clean. `cargo clippy -p paladin-llm --examples --all-features -- -D warnings`: clean.
- **Diff check for `unwrap(` / `expect(` / `panic!(` (Task 1, non-test code):** none added. The one added `panic!(`
  (at "--usage --dry-run must parse") is inside the example's `#[cfg(test)]` module.
- **Manual credential-handling review:**
  - The `--usage` mode prints integers, the verdict, credential variable names, header names and base URLs only. No
    response content, key, header value or URL query reaches stdout.
  - Non-200 bodies go through `diagnostic_excerpt` (redact, then bound).
  - The client refuses redirects (shared `run_probe_call` / `build_probe_client`).
  - The committed transcript was checked for these properties, as above.
- Not run: the live probe itself in this environment (no vendor key). The operator's transcript is the measurement.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or schema. Against the plan's register:
- T-50-22-01: bodies are excerpted through redaction, and the committed file was scanned.
- T-50-22-02: verdicts were copied verbatim, unconfigured providers read `not measured`, and indeterminate rows stay
  indeterminate.
- T-50-22-03: one call per model with a 1024 cap and a 120 s timeout.
- T-50-22-04: redirects are refused.

## Flagged assumptions

- A1-50-22 held: every seeded reasoning model reported a reasoning count under a 1024 cap. gpt-5.4-nano reported 0,
  and qwen-plus and gemini-3.5-flash-lite reported none; all three are `indeterminate`, never guessed.
- A2-50-22 held: no extras and no sampling were sent, and all 15 bodies were accepted.
- A3-50-22 held: Anthropic is `no vendor total` (its thinking 61 is at most output 64, recorded only). Ollama and
  OpenAI-compatible were not configured.
- A4-50-22: token counts and `cost_in_usd_ticks` are committed as billing metadata.

## Handoff to plan 50-24

Applying 50-24's fixed rules to these lines:
- Grok: every line is beside, so `ReasoningBesideCompletion` stays, now labelled measured.
- Kimi and DeepSeek: every line is inside, so they keep the default.
- Qwen: one inside, one indeterminate, so no change.
- OpenAI: indeterminate, so no change.
- Gemini: the measured lines are beside. This is the reading `map_gemini_usage` already applies, and only an inside
  verdict flips Gemini.
- Anthropic: no vendor total, so no change.
- Ollama and OpenAI-compatible: not measured, never flipped (A2-50-24).

## Self-Check: PASSED

- `.planning/phases/50-provider-parity-config-driven-model-swap/50-USAGE-PROBE.md`: FOUND.
- `crates/paladin-llm/examples/live_vendor_smoke.rs`: FOUND.
- `.planning/phases/50-provider-parity-config-driven-model-swap/50-USAGE-PROBE.raw.md`: absent, as required.
- Commits `8b113aea`, `b249d25e` and `4d76c807`: FOUND on `claude/laughing-dirac-e0h2ax`.
