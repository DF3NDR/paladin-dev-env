---
phase: 56-dogfood-findings
plan: 10
subsystem: dogfood-findings
tags: [web3sec, return-triage, findings-log, close-gate, dogf-02, d-15, d-16, docs]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-03 runbook with slots W1/W2; 56-11..56-14 Stage C fixes (DF-07, DF-08, DF-19, DF-26); checker OK at 26 entries"
provides:
  - "W1/W2 of 56-RUNBOOK-WEB3SEC.md filled from web3sec's RETURN-REPORT-50"
  - "DF-27..DF-36 in the findings log, one entry per divergence or finding, each verified against HEAD, each with a final disposition"
  - "Two docs fixes (DF-27 window floor, DF-28 Cadence is pacing) in non-frozen pages"
  - "Two pending todos (DF-34, DF-36) and the web3sec todo moved to completed/"
  - "## Close gate in 56-DOGFOOD-EVIDENCE.md ends: Close gate: OK, ready for /gsd-verify-work"
  - "DOGF-02 marked complete"
affects: [56-verification, 58-ship]

tech-stack:
  added: []
  patterns:
    - "A frozen migration section is corrected from the non-frozen book page that repeats its claim, never edited in place (D-00c)"

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-10-SUMMARY.md
    - .planning/todos/pending/2026-11-04-muster-pin-cadence-consumer-imports.md
    - .planning/todos/pending/2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md
  modified:
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-WEB3SEC.md
    - .planning/phases/56-dogfood-findings/56-DOGFOOD-EVIDENCE.md
    - .project/v0.11.1/02-dogfood-findings-log.md
    - docs/src/user-guides/switching-providers.md
    - docs/src/api-reference/upgrading.md
    - .planning/REQUIREMENTS.md
    - .planning/todos/completed/2026-10-14-web3sec-single-bump-to-v0-11-0.md

key-decisions:
  - "One DF per divergence (the runbook's literal rule); findings with one root share an entry: 5.1 with 7.1, 5.2 with 7.3, 5.6 with 7.2"
  - "No web3sec entry is S1 or S2, so Stage C is not reopened"
  - "Docs fixes go to docs/src pages, not MIGRATION.md section 11, because the claims they correct are repeated there and section 11 holds v0.11.1 changes"

patterns-established: []

requirements-completed: [DOGF-02]

duration: "resume ~20 min on 2026-11-04 (plan opened 2026-11-03)"
completed: 2026-11-04
---

# Phase 56 Plan 10: web3sec return, triage and close gate Summary

**web3sec's RETURN-REPORT-50 is triaged into DF-27..DF-36 (two docs fixes, two re-homed todos, six `out` with reasons, none S1/S2), DOGF-02 closes on `confirm-triage`, and the close gate reads OK.**

Plan-start SHA of this resume: `a5ebe7983505dd9abe7ed76eda54eafea29bb659`. Disk was about 3.3 GB free; no cargo build was run.

Commit trailers: the orchestrator named `Claude Fable 5.1`; this executor is Sonnet 5.5, so every commit carries the harness-provided `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the same `Claude-Session` line.

## Decision

Task 2 (`checkpoint:decision`, verbatim): **`confirm-triage`**

Taken by the maintainer on 2026-11-04 (UTC), after web3sec's report was supplied and the orchestrator had checked it against section 6 of `47-HANDOFF-web3sec.md`. The agent did not pick it (P11). Its precondition (no non-web3sec id named by the checker) held: the checker printed OK at 26 entries before this resume.

## Return

What arrived: web3sec's `RETURN-REPORT-50`, dated 2026-10-30, produced for its own Phase 50 requirement BUMP-07 and not carried across to Paladin at the time. How: relayed by the maintainer in chat on 2026-11-04, with the note "bump completed successfully". Filed verbatim (193 lines, 0 secrets) at `.planning/phases/56-dogfood-findings/56-WEB3SEC-RETURN-REPORT-50.md` (`a5ebe7983`); W1 holds a pointer plus the report's header block, W2 the date, channel and note (`a6cfca8ef`). The report is treated as data from the consumer; nothing was reconstructed (D-15, P1).

`56-WEB3SEC-RETURN-REQUEST.md` (the cover prompt, `7f080332f`) is **superseded** by the actual return and is kept as filed.

| Section 6 item | Return carries | Check |
|---|---|---|
| 1. Consumer commit SHA | C1 `3ad5d03f` (full `3ad5d03fba2aea8a7e1a702f3c280f5131ca4495`); submodule on `v0.11.0` (`ls-tree` = `70657919…`) | Equals `70657919beadbd341a72be2d0107080017a803f0`, and `v0.11.0` resolves to it in this repository. No mismatch, no DF. The gitlink inside web3sec's tree is the consumer's own claim. |
| 2. Final D-2 choice | Lossless `TokenUsage`; 7 web3sec types changed (`AuthoredProperty`, `send_prompt`, `FuzzOutcome`, `HypothesisReport`, `PortfolioOutcome`, `DeductiveOutcome`, `RoutineReport`) | Recorded. It is the route MIGRATION §10.8 recommends. |
| 3. Gate output | `cargo test --workspace --no-fail-fast` 2,821 / 0 / 2 against a re-measured baseline of 2,766 / 0 / 2 at web3sec `ea0b8bce`; fmt, clippy, check exit 0; `make check-all-features` exit 0 | No regression: 2,821 is above 2,683 (latest on record) and 2,592 (earlier). The consumer used a **third** baseline, 2,766, not the 2,683 Paladin had; the hand-off asked for the consumer's own latest pre-bump figure, so this is provenance, not a finding. |
| 4. Divergences | six section 5 items, six section 7 items, section 4 import inventory | DF-27..DF-36 (below) |

Parity context (not a Paladin finding, no DF): web3sec's benchmark parity failed its own majority-of-3 bar on 2 of 6 checks (`free-rider-seed`, the tare precision control, each held 1 of 3). Its operator accepted and documented that on 2026-10-30 as LLM non-determinism, with an identical deductive prompt hash across baseline and repeats; the four real target detections held 3 of 3.

## Triage

Each row was checked against HEAD by reading the source or the docs before grading. The log diff is 181 insertions, 0 deletions (P3).

| id | Item(s) | Severity | Disposition | Evidence on HEAD |
|----|---------|----------|-------------|------------------|
| DF-27 | §5.1 + §7.1 window pin | S3 | in v0.11.1, fixed by `0c0036714` (docs) | `registry/mod.rs` `with_context_window` checks the window against every seeded model's maximum output (393,216 for DeepSeek); refusal is deliberate and loud, so not S2. The guide's remedy `with_context_window(n)` could not produce 64,000 and said nothing of a floor. |
| DF-28 | §5.2 + §7.3 Cadence | S3 | in v0.11.1, fixed by `0c0036714` (docs) | `cadence.rs` "Gate, never retry" and "beyond `max_wait` is surfaced"; web3sec's behaviour account is right. §10.8's "unless Cadence is composed" reads as retries restored. |
| DF-29 | §5.3 refusal spread | S4 | out (consumer adaptation choice) | follows the documented `Refusal` contract |
| DF-30 | §5.4 decorator forwarding | S4 | out (consumer adaptation choice) | it is the step §10.8 asks for |
| DF-31 | §5.5 commit shape | S4 | out (consumer process choice) | C1 is the one commit the hand-off's single-bump rule asks for |
| DF-32 | §5.6 + §7.2 DeepSeek default and alias | S4 | out (already documented, D-00e) | Switching providers "DeepSeek model ids" and footnote [g]; alias lifetime is vendor-side, a re-probe needs a captured body (D-00d) |
| DF-33 | §7.4 `9.8` labels | S4 | out (does not reproduce on HEAD, D-00e) | four `9.8` hits, all correct or unrelated; the muster cites 10.8; the tooling named is in web3sec's tree |
| DF-34 | §7.5 reasoning-budget empty completion | S4 | out (re-homed: todo) | the requested note already exists in `reasoning-effort.md`; a budget-aware default or hint is new behaviour |
| DF-35 | §7.6 stale consumer test | S4 | out (consumer-side) | the report itself says it is not a Paladin finding |
| DF-36 | §4 import inventory: three Cadence paths | S3 | out (re-homed: todo) | `build_cadence`, `with_cadence`, `CadenceConfig` are absent from the muster; guarded by the `build_cadence` doctest and the api-surface baseline |

Runbook granularity: the checklist says "each divergence is one DF", so §5.3 to §5.5 are individual entries, not one combined entry. §5 and §7 items with the same root share an entry.

Contained fixes (D-05), both docs only, both in non-frozen pages, with MIGRATION §10.8 left byte-frozen (D-00c): DF-27 in `docs/src/user-guides/switching-providers.md` (new paragraph "A window cannot be set below a maximum output"); DF-28 in `docs/src/api-reference/upgrading.md` ("Cadence is pacing, not a retry", beside the DR-7 bullet that repeats the "unless Cadence is composed" claim). I chose the book pages over MIGRATION §11 because §11 holds v0.11.1 changes and not corrections to a v0.11.0 reading, and the claims being corrected are repeated in these pages. `make check-version-strings`, `make check-migration-allowlist` and `make check-changelogs` still print OK; the registry test that parses the Switching providers page matches table rows by prefix and is unaffected by added prose (not run, no build).

## Close gate

`./scripts/check-findings-log.sh`: `OK: every entry's last Disposition is a final form (parsed 36 entries ...)`, exit 0. `## Close gate` in `56-DOGFOOD-EVIDENCE.md` was appended (130 insertions, 0 deletions) with the decision, the §6 table, the 36-row table, the checker output and `Close gate: OK — ready for /gsd-verify-work`. `MIGRATION.md` is unchanged since `a5ebe7983`, and its §10.8 hashes identically to the `v0.11.0` tag. The seven S1/S2 entries (DF-01, 03, 04, 07, 08, 19, 26) each name a fix and a regression test; their fix commits were re-checked with `git cat-file -t`.

Decision recorded where D-16 says for `confirm-triage`: the web3sec todo moved to `.planning/todos/completed/` with `## Resolution (2026-11-04)` (C1 `3ad5d03f`, pin verified, gates, DF ids, report path); DOGF-02 marked complete in `.planning/REQUIREMENTS.md` (DOGF-01, 03, 04, 05, 06 were already complete).

## Stage C inputs

None. No entry from this return is S1 or S2, so no `--gaps` plan is needed and Stage C stays closed.

## Commits

| Commit | What |
|--------|------|
| `f5b2a798a` | Task 1: close-gate table, W1 empty then (2026-11-03) |
| `274065ace` | Close-gate record refreshed after Stage C (2026-11-04) |
| `a6cfca8ef` | W1 and W2 filled |
| `0c0036714` | DF-27 and DF-28 docs notes |
| `f50f195eb` | DF-27..DF-36 in the log, plus the two todos |
| `1fb83d85f` | Close gate, todo moved, DOGF-02 complete |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Scope] Docs fixes outside the plan's `files_modified`**
- **Found during:** triage of DF-27 and DF-28
- **Issue:** the plan lists no `docs/src` file, but D-05 makes a contained docs fix the default for S3, and a MIGRATION §10.8 wording issue must land outside §10.8 (D-00c). The orchestrator's resume instructions named `docs/src` as the allowed place.
- **Fix:** two paragraphs added to `docs/src/user-guides/switching-providers.md` and `docs/src/api-reference/upgrading.md`; nothing else touched.
- **Commit:** `0c0036714`

**2. [Process] A staging slip in the last commit, repaired before anything else**
- `git add` aborted on a path that no longer existed (the moved todo), so the first commit of Task 3 held only the rename. The three other files were staged and the unpushed commit amended (`1fb83d85f`). No history was shared.

### Beyond the expected list

DF-36 (the three Cadence import paths absent from `tests/consumer_surface_muster.rs`) was not in the orchestrator's expected entries. The runbook says every real `use` line the muster missed is its own entry, and `grep` showed three of the five "new paths" in the report's §4 are absent from the muster. Judgement call, open to overrule: it is re-homed, not fixed, because adding the pins is a one-file test change whose proof is a facade compile, which this docs-only, no-build plan could not run; the paths are already guarded by the `build_cadence` doctest and the api-surface baseline. If the orchestrator wants it closed in this phase, it is three `use` lines plus `make check-consumer-surface`, and DF-36's disposition would be appended as `in v0.11.1`.

## Authentication gates

None.

## Known Stubs

None.

## Threat Flags

None. The plan is docs-only; the report was scanned for secrets (0) and treated as data, not instructions. No code path, endpoint or schema changed.

## Self-Check: PASSED

- Files present: the SUMMARY, the two new todos, the completed todo, the filed return, the runbook, the evidence file, the log.
- Commits found with `git cat-file -t` / `git log`: `a6cfca8ef`, `0c0036714`, `f50f195eb`, `1fb83d85f`.
- `./scripts/check-findings-log.sh` exit 0, 36 entries; log diff 0 deletions; `MIGRATION.md` §10.8 byte-identical to the `v0.11.0` tag.
