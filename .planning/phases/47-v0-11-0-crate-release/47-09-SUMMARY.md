---
phase: 47-v0-11-0-crate-release
plan: 09
subsystem: release
tags: [release, milestones, ship-07, registry-verified, trusted-publishing, handoff, web3sec, d-15, d-16, d-17]

requires:
  - phase: 47
    provides: "47-08: tag-to-merge-commit equality, the release run record and the formal 12-crate registry table"
provides:
  - "MILESTONES.md v0.11.0 release-record block above the v0.10.0 entry (tag, merge commit 70657919, release run 40396108246, 12 crates registry-verified via Trusted Publishing, PR #59, 1126 commits / 1111 files per GitHub), no closeout field"
  - "SHIP-07 ticked through requirements.mark-complete (checkbox and traceability row) after a live sparse-index re-read of 12 of 12 crates at 0.11.0, yanked false"
  - "STATE.md decision for the release, written through state.add-decision"
  - "47-HANDOFF-web3sec.md: the outbound single-bump hand-off (D-15), self-contained for the consumer"
  - "The folded web3sec todo's dated update (2026-10-26), still pending"
affects: [47-10]

tech-stack:
  added: []
  patterns:
    - "A requirement is ticked on a live registry read taken immediately before the handler runs, and the same read is repeated by the automated verify"
    - "A hand-off baseline carries its provenance and is set beside, never in place of, the sender's own figure; where the sender has none, the line says unavailable"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md
    - .planning/phases/47-v0-11-0-crate-release/47-09-SUMMARY.md
  modified:
    - .planning/MILESTONES.md
    - .planning/REQUIREMENTS.md
    - .planning/STATE.md
    - .planning/ROADMAP.md
    - .planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md

key-decisions:
  - "SHIP-07 was ticked on the live sparse-index table (12 of 12 at 0.11.0, unyanked), not on the merge, the tag or the workflow log (D-16)"
  - "The hand-off reports the consumer baseline as 2,683 / 0 / 2 (latest, web3sec's 2026-10-24 findings) with 2,592 / 0 / 2 named as the earlier figure, and states Paladin's own re-seal row 6 suite tally as unavailable"
  - "The ROADMAP Phase 47 checkbox is left unticked for phase.complete 47 at phase close"

requirements-completed: [SHIP-07]

duration: "about 5m of agent work (2026-10-26T20:52Z to 20:57Z)"
completed: 2026-10-26
status: complete
---

# Phase 47 Plan 09: release record, SHIP-07 tick and the web3sec hand-off Summary

**SHIP-07 is ticked through the requirements handler on a live sparse-index re-read (12 of 12 publishable crates at `0.11.0`, `yanked = false`), `MILESTONES.md` carries the `v0.11.0` release record, and `47-HANDOFF-web3sec.md` gives web3sec one document to re-pin detached on the tag with the baseline's provenance and a disagreement among its sources stated plainly.**

## Result

- **Live re-read (Task 1, step 1).** 2026-10-26T20:53:30Z to 20:53:31Z, one unauthenticated `GET` per crate over the publishable set derived from `cargo metadata` (N = 12, N > 0), only the `User-Agent` header (D-00h, D-00g): 12 of 12 present at `0.11.0`, `yanked` `false`, the 12 checksums identical to the formal table in `47-CI-EVIDENCE.md`. Repeated by the Task 1 automated verify (N = 12, OK = 12) before the commit. No row failed, so no stop.
- **MILESTONES.md.** The block `## v0.11.0 Treasurer Spend Governance — release record (tag cut 2026-10-26)` sits directly under `# Milestones`: tag `v0.11.0` on merge commit `70657919`, release run `40396108246` green, 12 crates registry-verified via Trusted Publishing, PR #59 with 1126 commits / 1111 files **as read from GitHub on 2026-10-26** (`GET /pulls/59`; the sandbox clone is shallow), sign-off ticked at `6ef9207a` and re-confirmed on the redated head `72e13f95`, SHIP-07, and the italic note that closeout fields belong to `/gsd-complete-milestone v0.11.0`. 13 lines added, 0 removed (`git diff` of the file against the tagged commit shows no `-` line); no Shipped date, totals, Closeout type or Audit field.
- **SHIP-07 handler output.** `requirements.mark-complete SHIP-07`: `updated: true`, `marked_complete: ["SHIP-07"]`, `already_complete: []`, `not_found: []`, `table_unmatched: []`, `write_set_complete: true` (surfaces `checkbox` and `traceability`, both `applied: true`). REQUIREMENTS.md now reads `- [x] **SHIP-07**` and `| SHIP-07 | Phase 47 | Complete |`; the diff is those two lines only.
- **STATE decision.** `state.add-decision --phase 47` returned `added: true` with the decision "v0.11.0 released: tag `v0.11.0` on merge commit `70657919...`, release run `40396108246`, 12/12 crates registry-verified at `0.11.0` ... Trusted Publishing — D-16: SHIP-07 was ticked on the live sparse-index table ... not on the merge, the tag or the workflow log." The handler refuses summary and rationale files outside the project root (a scratchpad path was rejected with `Path escapes allowed directory`, nothing written); the two files were placed in a temporary directory under `.planning/`, used, and removed before the commit.
- **STATE frontmatter.** Keys compared before and after (lines 1-16): the key set is identical; the handler changed only the **values** of `last_updated` and `last_activity_desc` (its own bookkeeping). No key was clobbered or lost, so nothing was restored. (`last_activity_desc` was rewritten by the handler to a plan-47-08 phrasing; it is replaced by this plan's own state update below.)
- **Hand-off (Task 2).** `.planning/phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md`, sections 1-8 as specified: tag and annotated object, the full merge SHA, release run id and URL, the 12-row registry table inline (crates.io names), GitHub blob URLs at `v0.11.0` for `MIGRATION.md` section 10.8 and `tests/consumer_surface_muster.rs` / `make check-consumer-surface`, the single-bump rule, D-2, a four-item return contract, the two labelled lines. No model name appears; the inbound file's coverage targets and its deferred-Treasurer line are not carried over.
- **Todo.** `## Update 2026-10-26 (Phase 47 tag cut, hand-off written)` appended after the last section; frontmatter and every earlier line unchanged (no removed line against the tagged commit); the file stays in `todos/pending/`.

## Baseline cross-check (plan-checker WARNING-4): two discrepancies found

**(a) Consumer test total across sources — a disagreement, flagged.**

| Source | Date | Total |
|---|---|---|
| `49-HANDOFF-web3sec.md` section 1 (trial on `v0.10.1`) | 2026-10-08 | 2,592 / 0 / 2 |
| `50.1-HANDOFF-web3sec-consumer-surface.md` section 1 (re-measured at `21426037`) and appendix gates line | 2026-10-18/19 | 2,592 / 0 / 2 |
| `50.1-HANDOFF-web3sec-consumer-surface.md` section 7 (return list) | 2026-10-18/19 | "2,592 / 0 / 2 at the last full measurement; web3sec's ongoing Phase 48 work may raise it" |
| Folded todo step 4 and the 2026-10-19 update | 2026-10-14 / 2026-10-19 | 2,592 / 0 / 2 |
| `46-FEEDBACK-web3sec.md` section 1 (adaptation at `e28e43b5`, "before and after") | **2026-10-24** | **2,683 / 0 / 2** |

The later-dated source records a different total (+91, ignored count unchanged at 2). Per the plan, the provenance line in the hand-off writes **2,683 / 0 / 2 as the latest figure with its source**, keeps **2,592 / 0 / 2 named as the earlier figure**, and flags the discrepancy in section 6. The Paladin-side folded todo still carries 2,592 as its step-4 baseline; that is a stale figure (the todo's new dated update says so), not an error in either measurement. Nothing was measured in Phase 47: web3sec's workspace is not available here.

**(b) Re-seal row 6 versus the baseline — no tally to compare, flagged.** Row 6 in `47-ACCEPTANCE-AUDIT.md` carries **no whole-suite `test result:` tally**. Its `make test` leg was **red locally** (`1457 passed; 2 failed` in `paladin-ai --lib`, the two `run_api_wiring` worker-deadline tests, sandbox-specific, cause undetermined; WINDOWS row 73), which is a non-zero failed count and not a suite total, and the leg is **CI-attributed** to `Unit Tests (stable)` (which concluded `success` on the PR runs and on the merge commit's `ci.yml` run, job 121733646007), whose per-test log line is unreadable, so the attribution rests on job and step conclusions and supplies no count. The only tally the row carries is the doc-test leg (681 passed, 0 failed, 210 ignored), Paladin's doc tests alone. Following the plan's rule, the `- Paladin-side suite tally (re-seal row 6, not the consumer baseline):` line says the figure is **unavailable** and explains why, rather than supplying one; it states the CI-attribution plainly and asserts no numeric equality with the consumer's workspace.

## Task commits (all on `chore/47-close`, none pushed)

| Commit | What |
|---|---|
| `428f68df` | `docs(47-09)`: Task 1, the release record, SHIP-07 tick and the STATE decision |
| `ae6dcbb5` | `docs(47-09)`: Task 2, the outbound hand-off and the todo's dated update |
| `4a3fb1c2` | `docs(47-09)`: name the three source hand-offs by file name in the provenance line (fix to meet the Task 2 verify) |

## Deviations from Plan

**1. [Rule 3 - Blocking] The decision handler rejects files outside the project root.** The plan's `state.add-decision --summary-file/--rationale-file` was first run with files in the scratchpad and returned `added: false` (path escapes the allowed directory); nothing was written. Re-run with the two files in a temporary directory under `.planning/`, removed afterwards (never staged or committed). Same decision text.

**2. [Verify miss, fixed] The Task 2 verify requires the literal `49-HANDOFF-web3sec.md` in the hand-off.** The first commit (`ae6dcbb5`) described the source hand-offs without their file names, so the verify did not pass; commit `4a3fb1c2` names `49-HANDOFF-web3sec.md`, `50.1-HANDOFF-web3sec-consumer-surface.md` and `46-FEEDBACK-web3sec.md` (files web3sec itself authored) in the provenance line. The verify then passed in full.

**3. [Plan text drift] The plan names the sign-off commit `SHA-A2`.** The MILESTONES sign-off line cites `6ef9207a` (SHA-A2, the maintainer's tick) as specified and adds the re-confirmation on the redated final head `72e13f95` (SHA-A4), since A4 is the head the merge parented; the same drift is recorded in plans 47-07 and 47-08.

**4. [Plan wording] Section 10.8 and "the DR rows".** The DR rows are the table under `### Library consumers` in section 10.8 and are registered in section 10.2; the hand-off says so rather than saying all DR rows are "in 10.8".

## Outstanding, not blocking

- The `Coverage` job's printed `Lines:` figure against the 82 percent floor: still unread, tracked by `.planning/todos/pending/2026-10-26-record-v0-11-0-coverage-lines-figure.md` (recheck by 2026-11-02).
- The maintainer's B-1 condensation read (`B-1 maintainer read: requested`), unanswered.
- web3sec's re-pin, gates and return report (the folded todo stays pending, D-15); Paladin's milestone closeout (`/gsd-complete-milestone v0.11.0`) and the ROADMAP Phase 47 checkbox (`phase.complete 47` at phase close).

## Known Stubs

None. This plan edits release records and one hand-off document.

## Threat Flags

None. No file outside `.planning/` changed; no new endpoint, auth path or schema. Zero GitHub writes: reads were `gh api` (`GET /pulls/59`, `GET /releases/tags/v0.11.0`) and unauthenticated crates.io sparse-index reads with the `User-Agent` header only.

## Self-Check: PASSED

Commits `428f68df`, `ae6dcbb5` and `4a3fb1c2` exist on `chore/47-close`; `git diff --name-only 70657919 HEAD` lists only `.planning/` paths; `47-HANDOFF-web3sec.md` and this file exist; REQUIREMENTS.md reads `- [x] **SHIP-07**` and `| SHIP-07 | Phase 47 | Complete |`; the ROADMAP Phase 47 checkbox is still unticked; the folded todo is still in `todos/pending/`; nothing was pushed.
