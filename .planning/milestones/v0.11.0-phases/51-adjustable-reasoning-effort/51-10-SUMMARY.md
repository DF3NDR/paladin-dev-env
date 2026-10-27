---
phase: 51-adjustable-reasoning-effort
plan: 10
subsystem: llm-live-evidence
tags: [reasoning-effort, effort-probe, live-vendor-smoke, evidence, scrub-check]
status: complete

requires:
  - phase: 51-02
    provides: "live_vendor_smoke --effort (raw-wire effort probe) and the 62-id candidate table"
provides:
  - "51-EFFORT-PROBE.md: the dated (2026-10-22, commit 32ce9dd0), scrubbed, verbatim effort-probe transcript over the confirmed 62-id set, with a curated header and a mechanically computed verdict table; the only source plans 51-11 and 51-12 may seed effort data from"
  - "51-EFFORT-PROBE-CANDIDATES.md: a `## Final candidate set` section recording the owner's answers before any live call"
affects: [51-11, 51-12, 51-14]

tech-stack:
  added: []
  patterns:
    - "scratchpad-only raw capture per vendor run, assembled into one untracked raw file, scanned, then folded verbatim under a curated header; raw deleted in the same commit"
    - "verdict tables generated from the transcript rows by a scratch script, never typed by hand"

key-files:
  created:
    - .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md
  modified:
    - .planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE-CANDIDATES.md

key-decisions:
  - "No seed value is decided in the evidence file; every verdict is a rule applied to quoted rows (parsed = invalid value drew 400/422, effect = highest accepted level reports more reasoning tokens than the lowest, confirmed = both, D-04)."
  - "Kimi was run as the kimi-k3 slice first, then the Kimi vendor run probed kimi-k2.6 only, so kimi-k3 was not paid for twice."
  - "The one lost row (qwen3.7-max enable_thinking off, transport failure) was re-probed once, alone, following the 50.1 precedent; both runs are in the transcript."

requirements-completed: []
requirements-progress: [REASON-03, REASON-04]

duration: about 1h 15m (including a cold example build of about 8 minutes)
completed: 2026-10-22
---

# Phase 51 Plan 10: Live effort probe Summary

**`live_vendor_smoke --effort` ran live over all 62 confirmed candidate ids of seven vendors (718 probe rows, 2026-10-22, commit `32ce9dd0`); the scrubbed transcript and a mechanically computed verdict table are committed as `51-EFFORT-PROBE.md`, with `kimi-k3` probed first through both wire forms.**

REASON-03 and REASON-04 are advanced, not marked complete: the evidence now exists, but the seeded effort data and the adapter wire positions are plans 51-11 and 51-12. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Checkpoint decision

Task 1 (`checkpoint:decision`, gate blocking) was resolved by the owner before dispatch, in two messages on 2026-10-22. Task 1 built nothing and has no commit of its own. The answers, verbatim, with the date, and recorded (in `51-EFFORT-PROBE-CANDIDATES.md` under `## Final candidate set`, commit `32ce9dd0`) before any live call was made:

- Option selected: **run-here** (owner's words: "run here."). This is also the go-ahead to spend.
- Struck / added ids: **none / none**. The owner confirmed the drafted candidate set (owner's words: "that should be the ones that have been previously probed ... the models that are the currently active latest models available"); the orchestrator had verified that set is exactly the seeded registry's model ids. Every id in the candidate table was probed, the planner's additions included.
- D-02 treatment of an unprobed seeded `reasoning: false` entry: **unmeasured** (owner's words: "D02 unmeasured"), i.e. `EffortSupport::Unmeasured`, not `NoReasoning`.

The same three answers, dated, are in the `## Owner decision` section of `51-EFFORT-PROBE.md`.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Owner checkpoint (candidate set, D-02 treatment, go-ahead) | none (resolved by the owner; recorded in `32ce9dd0` and `91ad525f`) |
| 2 | Run the probe: `kimi-k3` slice first, then every confirmed id one vendor at a time | `32ce9dd0` (the `## Final candidate set` section, committed before the first call); the raw transcript is deliberately untracked |
| 3 | Header, verdict table, scrub check, commit `51-EFFORT-PROBE.md`, delete the raw file | `91ad525f` |

## What was done

- **Sequencing.** Task 2 first appended the owner's decision and the final candidate set (62 ids, delta 0) to the candidates file and committed it, then built the example (debug build, about 8 minutes cold) and ran `--effort --models kimi=kimi-k3` alone into the scratchpad. That slice had both forms' `low`, `max` and invalid rows with numbers and scanned clean, so the remaining vendors ran one at a time in vendor order (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini). Each run's stdout went to the scratchpad and was scanned, then the runs were concatenated (one blank line between) into the untracked raw file.
- **Result.** 9 runs, 7 providers PROBED, Ollama and OpenAI-compatible SKIPPED (no daemon and no endpoint configured, as in 50.1). 718 probe rows: 491 with a 2xx, 226 with 400 or 422, 1 error row. Every run exited 0 with empty stderr. No authentication failure, outage or rate limit occurred.
- **Retry.** `qwen3.7-max` row 2 (`enable_thinking` off) was lost to a client-side transport failure (`error sending request`); the candidate was re-probed once, alone (8 more calls, run 7), which measured the row cleanly. Both runs are in the transcript and the header says which row run 7 supplies.
- **Header and verdict table.** `51-EFFORT-PROBE.md` has `## Provenance`, `## Owner decision`, `## Providers PROBED and SKIPPED` (with `### Candidate set`, `### web3sec models`, `### Kimi wire form`), `## Verdict table`, `## Vendor-documented bounds and coercions (docs, not observed)`, then the transcript verbatim (byte-identical to the raw file, checked with `cmp` before the raw file was deleted). The verdict tables, the Anthropic paired-versus-alone table and the per-model off / auto / budget / sampling / Gemini-spelling rows were generated by a scratch script from the transcript rows.
- **Docs section.** Fetched on 2026-10-22 with the vendors' own pages, each entry with URL, access date and the quoted sentence: Qwen `thinking_budget` (console range `1~32768`; the transcript's own rejection body for `qwen3.7-max` names `262144`, and the two are explicitly not reconciled), Anthropic `budget_tokens` (minimum 1,024, below `max_tokens`; deprecated on 4.6, rejected on 4.7 and later), DeepSeek's silent effort mapping (minimal to low, medium and xhigh to high, ultra to max), and xAI's `xhigh` treated as `high` on `grok-4.5` (the page names neither `grok-4.3` nor `grok-4.20-reasoning`). No Gemini 2.5 id is in the set, so no Gemini budget bound was fetched.

## Findings the seed plans will read (numbers from the verdict table; no seed value decided here)

- **Kimi wire form (research Pitfall 2).** Through the top-level `reasoning_effort` form the invalid value `bogus-effort-51` drew 200 on both `kimi-k3` and `kimi-k2.6` (field not parsed, silent); through `thinking.effort` it drew 400 on both. On `kimi-k3`, `low`/`max` reasoning tokens were 47/86 (top-level) and 40/116 (`thinking`); on `kimi-k2.6`, 128/327 and 141/141. The `thinking` form accepted `low`, `high`, `max` and rejected `minimal`, `medium`, `xhigh`. Per the D-04 rule as stated in the file, `thinking_object` is confirmed on Kimi and `top_level_effort` is not.
- **web3sec's three models.** All probed end to end with no lost row: `claude-opus-4-8` (17 rows; reports 0 reasoning tokens on every `output_config` row and 41 on the paired `max` row; `thinking.type` off and adaptive both 200 at 0 tokens), `deepseek-v4-pro` (12 rows; levels minimal to max all accepted; `auto` reports 110), `kimi-k3` (18 rows).
- **Vendor-form verdicts.** OpenAI `top_level_effort`, Anthropic `output_config`, DeepSeek `top_level_effort`, Grok `top_level_effort`, Gemini `thinking_level` and Kimi `thinking_object` read parsed and with effect (confirmed). Qwen `detector` and `thinking_budget` read partial (9 of 13 and 12 of 13 invalid values rejected; the older `qwen-*` alias ids accepted the bogus top-level field silently). `grok-4.20-reasoning` rejected every level and the sampling row with 400.

## Deviations from Plan

### Auto-fixed Issues

None needing a rule.

### Notes against the plan's wording

- **Kimi run probed `kimi-k2.6` only.** The plan's combined Kimi command lists `kimi-k2.6,kimi-k3`; since the `kimi-k3` slice had just run, the Kimi vendor run used `kimi=kimi-k2.6` and the slice stands as `kimi-k3`'s measurement (the plan: "the `kimi-k3` slice goes in the Kimi section"). This saves 18 calls and is stated in the header. Every one of the 62 ids was probed exactly once, except `qwen3.7-max`.
- **8 extra calls.** The `qwen3.7-max` retry makes 718 rows against the 710 planned. It follows the 50.1 retry precedent, on a confirmed id, for one row lost to a transport failure; the header discloses it.
- **Per-vendor stdout went to the scratchpad first.** The plan appends each vendor's output directly to the raw file in the phase directory; I wrote each run to the scratchpad, scanned it, and concatenated into the raw file afterward, so a hit would never sit in the repository tree. The assembled raw file was never staged and is gone.
- **Run numbering vs. time.** Runs are numbered in transcript order (the `kimi-k3` slice is run 4 there but ran first); the header states the chronological order.
- **Binary run directly.** After the first `cargo build`, the runs used the built example binary directly (same example, same flags as `cargo run`).
- **Final candidate set committed before the calls**, rather than only at the end, so the owner's answers exist in git before any spend (the plan's "before any live call is made").
- **gitleaks is not installed in this container**, so only the scratch scanner and the plan's own negative key-shape grep were run; the orchestrator's post-wave hook chain applies `.gitleaks.toml`.

## Manual credential-handling review (security.instructions.md)

- The harness prints numbers, statuses and `diagnostic_excerpt`-bounded, redacted excerpts of non-2xx bodies only; response content and request headers are never printed (plan 51-02's tests pin this). No key value was printed, echoed or logged by this plan: variable presence was checked with the `${VAR:+set}` form only.
- Scrub method, by concept: a scratch scanner (scratchpad, not committed) covering the shapes `redact_secret_patterns` recognises (authorization-scheme token marker in both capitalisations, vendor-prefixed secret-key marker, AWS access-key-id marker, key and token query-value markers, three-segment base64url triples), plus Google, xAI, GitHub and Slack key shapes, long mixed-case tokens, authorization-style header lines carrying a value, and the literal value of every credential variable set in the environment and of every file under the operator's `~/.config/paladin/` (27 values, compared programmatically, never printed). It was shown to fire on a planted sample first. Result: `clean` on each of the nine raw runs, on the assembled raw file, and on the final `51-EFFORT-PROBE.md`. The plan's negative key-shape grep also found nothing.
- The committed files carry no key, token, authorization header value or key-bearing URL. The documentation quotes were read for key-shaped sample strings (the Qwen page contains one in its sample code) and that text was not quoted.
- No static taint analysis gates this; the above is the manual review.

## Known Stubs

None.

## Threat Flags

None. No endpoint, file access or schema change was added; the only new files are planning evidence. T-51-31 (key in transcript) is mitigated by the scanner and the grep; T-51-32 (unobserved fact) by generated tables and the separate docs heading with URLs and dates; T-51-33 (runaway spend) by the checkpoint, the 2,048-token cap, the 120 s timeout, per-vendor runs and `--models`-only ids.

## Self-Check: PASSED

- Files present: `.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md`, `.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE-CANDIDATES.md` (with `## Final candidate set`), this SUMMARY; `51-EFFORT-PROBE.raw.md` is absent.
- Commits `32ce9dd0` and `91ad525f` present on `worktree-agent-a0acb35b8bdf52939`.
- Task 2 and Task 3 automated verify commands exited 0 (Task 2's before the raw file was deleted).
