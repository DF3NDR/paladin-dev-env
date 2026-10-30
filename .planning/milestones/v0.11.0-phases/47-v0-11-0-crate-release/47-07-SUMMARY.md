---
phase: 47-v0-11-0-crate-release
plan: 07
subsystem: release
tags: [release, ci-evidence, one-way-door, merge-and-tag, hand-off, d-00b, d-00c, d-00i, d-13]

requires:
  - phase: 47
    provides: "47-06: the maintainer's sign-off on the redated final head SHA-A4 72e13f95 and the nine runs on it"
provides:
  - "The nine runs on SHA-A4 recorded per run and per job, with the 44 required contexts of ruleset 20868126 tallied (43 success, 1 skipped by design, 0 failure, 0 absent)"
  - "The pre-tag guard: twelve of twelve publishable crates ABSENT at 0.11.0 and no v0.11.0* tag on the remote at 2026-10-26T15:40Z"
  - "The maintainer's Task 2 decision `proceed` (2026-10-26T16:49Z) and Task 3 reply `tagged ...` (2026-10-26T20:32Z), both recorded verbatim"
  - ".continue-here.md: the third resume file, resume condition = tag on the remote and in main, and the tag-triggered release run concluded; the D-18 red-run rule"
affects: [47-08, 47-09, 47-10]

tech-stack:
  added: []
  patterns:
    - "D-00b / D-00k: the merge, the tag and every later release act are the maintainer's; the agent records and hands over"
    - "A pasted SHA is never trusted: every value is resolved from the tag itself (`git rev-parse 'v0.11.0^{commit}'`)"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-07-SUMMARY.md
  modified:
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md
    - .planning/phases/47-v0-11-0-crate-release/.continue-here.md

key-decisions:
  - "Plan 47-07's `SHA-A2` role is read as SHA-A4 72e13f95, the final head the merge carries as its PR-side parent"
  - "Task 2 = `proceed` (maintainer, 2026-10-26T16:49Z)"
  - "Task 3 = `tagged <merge sha> <tag object sha>` (maintainer, 2026-10-26T20:32Z); the first token is 41 characters and is not used"

requirements-completed: []

duration: "not separable: 2026-10-26T15:38Z to 20:32Z elapsed, dominated by maintainer decisions, the merge, and ci.yml on the merge commit"
completed: 2026-10-26
status: complete
---

# Phase 47 Plan 07: checks on the final head, the one-way door and the merge-and-tag hand-off Summary

**All nine runs on the final head SHA-A4 `72e13f9583ffe2b89580049045ac094ed3d46dbf` were green, the version was unclaimed, the maintainer answered `proceed` at the one-way door, merged PR #59 with a merge commit and pushed the annotated tag `v0.11.0` onto it; the plan ends at its two-part resume condition, both parts of which hold.**

## Result

- **Task 1 (tracer).** Checks recorded read-only on SHA-A4 at 2026-10-26T15:38Z to 15:42Z: nine runs, every one `completed`/`success` (two `ci.yml`, two `codeql.yml`, two `feature-flags.yml`, `Docs`, two `pre-commit`); `ci.yml` jobs 39 each, skipped by design only. Required contexts of ruleset 20868126 (44): 43 `success`, 1 `skipped` (`End-to-End Tests`), 0 failure, 0 absent. `Check publish order` `success` in both `ci.yml` runs. PR #59 `mergeable` true, `mergeable_state` `unstable` (the advisory, non-required `CodeQL` results check). `main` unmoved at `df24689c`. Twelve of twelve publishable crates `ABSENT` at `0.11.0`; no `v0.11.0*` tag on origin. Sections added to `47-CI-EVIDENCE.md`: "Plan 47-07 — every run on SHA-A4", "Pre-tag guard", one Findings entry.
- **Task 2 (one-way door, `checkpoint:decision`, gate blocking).** Presented; the maintainer's reply, received 2026-10-26T16:49Z: "Create a ToDo for the `Coverage Lines` or it will have to be picked up in the verification step. Proceed with the PR `#59`  merge." Decision = `proceed`. The Coverage request was met by creating `.planning/todos/pending/2026-10-26-record-v0-11-0-coverage-lines-figure.md`; the `Lines:` figure itself is still unread.
- **Task 3 (`checkpoint:human-action`, gate blocking-human, never auto-approved).** Presented at 2026-10-26T16:51Z. The maintainer merged PR #59 with a merge commit (2026-10-26T17:27:08Z), waited for `ci.yml` on the merge commit, and pushed the annotated tag at 2026-10-26T20:29:02Z. Reply, received 2026-10-26T20:32Z, verbatim: `tagged 770657919beadbd341a72be2d0107080017a803f0` then `77a37c1846e5ab6716a51b78fb39541bccd1a7ae` on the next line.
- **Resume condition (both parts hold).** (a) `refs/tags/v0.11.0` is on origin and its commit `70657919beadbd341a72be2d0107080017a803f0` is `origin/main`; (b) `release.yml` run 40396108246 concluded `completed`/`success` at 2026-10-26T20:42:21Z. Plan 47-08 re-read both.

## Commits

None by this plan: every record stayed in the working tree so the maintainer's `72e13f95` remained the release branch's last commit (D-13, D-02). Plan 47-08 committed them on `chore/47-close`.

## Deviations from Plan

**1. [Plan text drift] `SHA-A2` read as SHA-A4.** The plan calls the tick commit `SHA-A2` "the true final commit". The redate (SHA-A3) and the maintainer's re-confirmation (SHA-A4) came after the tick, so the final head, and the merge's PR-side parent, is SHA-A4 `72e13f95`. The automated verify ran against the `- SHA-A4:` line. Recorded once in `47-CI-EVIDENCE.md`.

**2. [Maintainer paste defect] The Task 3 reply's first token has 41 characters.** `770657919beadbd341a72be2d0107080017a803f0` carries a stray leading `7` before the real merge SHA `70657919beadbd341a72be2d0107080017a803f0`; it is not an object in the repository. The reply is recorded verbatim, the defect is named, and every SHA used is resolved from the tag itself (`git rev-parse 'v0.11.0^{commit}'`). The second token, the tag object `77a37c18...`, is exact. No action is needed: the tag was pushed correctly, only the paste is wrong.

## Known Stubs

None. This plan edits release records only.

## Self-Check: PASSED

Both resume-condition facts re-read on 2026-10-26T20:44Z by plan 47-08 (tag on origin, tagged commit equals `origin/main`, release run `success`); the reply is recorded in `.continue-here.md` under `## Task 3 reply (47-07)`.
