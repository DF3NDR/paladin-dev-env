---
phase: 50-provider-parity-config-driven-model-swap
plan: 17
subsystem: api
tags: [paladin-llm, live-probe, seed-transcript, adr-0061, owner-reading-r-2, parity]

requires:
  - phase: 50
    provides: "plan 50-04: the live_vendor_smoke --probe harness; plan 50-02: ADR-0061 and its Owner readings table with R-2 written pending"
provides:
  - "50-SEED-PROBE.md: the dated, scrubbed, operator-run transcript (2026-10-15 22:23:15Z, commit c33022b9) that plan 50-06 seeds the registry from and plan 50-13 reads usage-cap evidence from"
  - "A header naming 7 providers PROBED (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Gemini) and 2 SKIPPED (Ollama, OpenAI-compatible) with per-provider models-listed and candidates-probed counts"
  - "Owner reading R-2 recorded for all five compat presets as accept unverified (2026-10-16), in the transcript header and in ADR-0061's Owner readings row R-2"
  - "Operator-supplied evidence carried verbatim: the per-vendor default model (D-08 step 4d), docs-sourced context windows and maximum outputs with URL and access date, stop-sequence maxima, and the Moonshot temperature finding"
affects: [50-06, 50-13, 50-16]

tech-stack:
  added: []
  patterns:
    - "Curated evidence file = Claude-written provenance header + operator transcript verbatim; the raw file is deleted in the same commit so one file is the sole source"

key-files:
  created:
    - .planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md
  modified:
    - .planning/decisions/0061-per-model-parameter-support-policy.md

key-decisions:
  - "R-2 is accept unverified for kimi, qwen, grok, ollama and openai-compatible: no quota-exhaustion body was captured for any compat preset, so plan 50-13 encodes no usage-cap error_override for them and each is listed as unverified (PARITY-05)"
  - "Ollama and OpenAI-compatible were deliberately skipped by the operator (no local daemon, no endpoint configured); the run counts as 7 of 9 probed"
  - "The curated header cites the transcript's own commit line (c33022b9) and states that the first, rate-limited Gemini run survives only in git history"

patterns-established:
  - "Scrub check by re-implementing redact_secret_patterns semantics over the file (bearer, sk-, AKIA, key=, token= markers and JWT triples) plus extra key shapes and the literal values of environment credentials; the curated header avoids writing those marker strings so the file stays clean under the same check"

requirements-completed: [PARITY-04, PARITY-05]

duration: 25min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 17: Operator live seed probe and owner reading R-2 Summary

**Operator-run live probe of seven vendors committed as the sole, dated, scrubbed seed source (50-SEED-PROBE.md), with owner reading R-2 recorded as accept unverified for all five compat presets in the transcript and in ADR-0061.**

## Performance

- **Duration:** about 25 min for this continuation (the operator's live run happened before it)
- **Completed:** 2026-10-16
- **Tasks:** 2 of 2 (Task 1 by the operator, Task 2 by the executor)
- **Files modified:** 2 (one created, one edited; the raw transcript was removed)

## Accomplishments

- Verified the operator's Task 1 hand-back: the raw transcript exists with 9 provider sections, an `## Operator-supplied evidence` section and a `## Owner reading R-2` section holding exactly one line per compat preset. The plan's Task 1 automated verify exits 0.
- Validated the transcript: 7 of 9 providers probed; Ollama (`OLLAMA_BASE_URL` not set) and OpenAI-compatible (`OPENAI_COMPATIBLE_API_KEY` not set) skipped, both named by the operator as deliberate. Counts per provider (models listed / candidates probed): OpenAI 135/2, Anthropic 14/9, DeepSeek 2/3, Kimi 4/4, Qwen 99/4, Grok 14/3, Gemini 63/3.
- Scrub-checked the whole file with `redact_secret_patterns` semantics (a Python port run in the scratchpad, comparing against the Rust patterns by hand) plus the Google and xAI key shapes, authorization-style header lines and the literal values of every credential variable in this environment. Nothing matched.
- Wrote `50-SEED-PROBE.md` (provenance header, PROBED and SKIPPED table, the five-line `### Usage-cap evidence (owner reading R-2)` list, sole-seed-source note, then the transcript byte-for-byte, confirmed by a diff against the raw file at HEAD).
- Replaced R-2's `pending` answer in ADR-0061's Owner readings table with the five per-vendor answers and the date 2026-10-16, keeping the `reversible` rating and the `plan 50-17 Task 1` reference.

## Task Commits

1. **Task 1: Operator runs the probe and answers R-2** - completed by the operator, no executor commit. Operator commits: `c33022b9` (raw transcript and R-2 lines) and `2e941ef8` (re-run with Gemini billing upgraded, all three Gemini models measured).
2. **Task 2: Validate, scrub check, record R-2, commit as 50-SEED-PROBE.md** - `7702a80b` (docs)

**Plan metadata:** committed separately with this SUMMARY.

## Files Created/Modified

- `.planning/phases/50-provider-parity-config-driven-model-swap/50-SEED-PROBE.md` - the curated evidence file (created; supersedes the raw file, which `git` records as a rename in `7702a80b`)
- `.planning/decisions/0061-per-model-parameter-support-policy.md` - Owner readings row R-2 answered

## Decisions Made

- R-2 answers are copied from the operator's own `## Owner reading R-2` lines; every one is `accept unverified`, so no `re-probe` stop applied and no override is to be encoded by plan 50-13. The date in the header lines is the resume message's date (2026-10-16), as the plan prescribes.
- The header repeats no value that is not in the transcript (PARITY-04). Counts and the Anthropic and DeepSeek remarks are derived from the transcript's own tables and the operator's evidence section.
- Marker words such as the bearer, `sk-`, `AKIA`, `key=` and `token=` strings were kept out of the header prose after a first draft tripped the scrub port on its own description of the check.

## Deviations from Plan

None - plan executed as written, apart from two cosmetic rewordings of the header's scrub-check sentence (described above) made before the commit. No measured value was edited.

## Issues Encountered

- Several platform credential variables exist in this environment (GitHub, Google Cloud, AWS, session tokens), contrary to the plan's expectation of none. None of the nine vendor variables is set, and no literal value of any of them appears in the transcript. Values were compared in-process and never printed.
- The operator's transcript records `commit: c33022b9`, the commit that first carried the raw file, not the harness commit. The header reports it as the transcript states and does not reinterpret it.

## Information Plans 50-06 and 50-13 Should Carry Forward (all from the transcript)

- Operator-chosen defaults (section 4d): openai `gpt-5.4-nano`, anthropic `claude-haiku-5-5`, deepseek `deepseek-flash`, kimi `kimi-k2.6`, qwen `qwen3.8-flash`, grok `grok-4.3`, gemini `gemini-3.5-flash-lite`.
- Do not seed: the eight legacy Claude ids (404), `deepseek-chat` and `deepseek-coder` (undocumented aliases), `qwen3-max` (404 on the International endpoint), `qwen-turbo` (rate-limited on every probe, nothing measured), and the two `kimi-k2.7-code` models (every probe rejected, not general-purpose).
- Kimi's measured temperature rule (only 0.6 accepted on `kimi-k2.6` and `kimi-k3`) supersedes the handoff's 2026-10-07 note.

## Known Stubs

None. No code was written; the evidence file contains no placeholder text.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced; only documentation files changed.

## Manual credential-handling review (D-00a, security.instructions.md)

The committed transcript carries no key, token, authorization header or key-bearing URL. Credential variables appear by name only (the transcript header lists them as names, never values) and every credential header is named without its value. Request bodies in the transcript passed through `diagnostic_excerpt` in the harness; the vendor error bodies quoted by the operator carry error text and vendor request ids only. No key-shaped string, no JWT-shaped triple and no env-var literal matched. This review is manual; no static taint analysis gates this file, per the project's stated Rust SAST gap.

## Next Phase Readiness

- Plan 50-06 is unblocked: it may seed the dated registry from `50-SEED-PROBE.md` and nothing else.
- Plan 50-13 must encode no usage-cap `error_override` for kimi, qwen, grok, ollama or openai-compatible, and must list each as unverified in its SUMMARY. Plan 50-16's Switching providers page must do the same.
- ADR-0061 R-1 stays `pending` until plan 50-18.

## Self-Check: PASSED

- `50-SEED-PROBE.md` exists; the raw file is gone.
- Commit `7702a80b` exists (Task 2); the operator commits `c33022b9` and `2e941ef8` exist.
- The Task 2 automated verify exits 0; the curated file has 16 `## ` headings; ADR-0061's R-2 row no longer reads pending.
