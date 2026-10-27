---
phase: 47-v0-11-0-crate-release
plan: 06
subsystem: release
tags: [release, ci-evidence, sign-off, redate, hard-stop, d-00a, d-00j, d-00i, d-13]

requires:
  - phase: 47
    provides: "47-05: SHA-A0 d4e66b47 as PR #59's head, the retitled PR and the first resume file"
provides:
  - "The per-job record of every run on SHA-A0 and the required-context tally from the live ruleset (44 contexts), with the audit's CI pointers"
  - "The recorded D-00j halt on the red required context `pre-commit run --all-files` and the maintainer's `Option 1` decision; SHA-A1 pushed over it"
  - "The maintainer's tick (SHA-A2 6ef9207a), the redate decision `accept-X redate 2026-10-26`, the restamp of 12 changelogs (SHA-A3 02f6ee2e) and re-seal row 7 re-read"
  - "The maintainer's re-confirmation commit SHA-A4 72e13f95 verified read-only as the final head, with the orchestrator's ruling recorded verbatim"
  - ".continue-here.md: the second resume file; resume condition = every run and required context on SHA-A4 concluded"
affects: [47-07, 47-08, 47-09]

tech-stack:
  added: []
  patterns:
    - "D-00j: a red required context is recorded and halted on; the way forward is a recorded maintainer decision, never a self-authorised fix or a re-run"
    - "D-13: the maintainer's commit is the last commit on the release branch; every later record stays in the working tree for the close branch"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-06-SUMMARY.md
  modified:
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md
    - .planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md
    - .planning/phases/47-v0-11-0-crate-release/.continue-here.md
    - .planning/STATE.md
    - .planning/ROADMAP.md

key-decisions:
  - "Option 1 for the `pre-commit` red on SHA-A0: push the held commits as SHA-A1; the incidental `end-of-file-fixer` fix rides the push; the SHA-A0 rows keep `failure`"
  - "accept-X: the maintainer's uppercase `[X]` tick is accepted as the tick; ticked boxes are counted case-insensitively"
  - "redate 2026-10-26: the twelve changelog headings move from 2026-10-25 to 2026-10-26 (D-00n); the pre-build `DATE=2026-10-25` stays recorded append-only"
  - "The re-confirmation commit SHA-A4 is accepted as the maintainer's re-confirmation on the orchestrator's ruling although it normalised the marker instead of adding the prescribed text line"

requirements-completed: []

duration: "not separable: 2026-10-25T21:10Z to 2026-10-26T13:45Z elapsed, dominated by maintainer waits and CI"
completed: 2026-10-26
status: complete
---

# Phase 47 Plan 06: PR CI evidence, the maintainer's sign-off and the hard stop on the final head Summary

**The v0.11.0 sign-off box is ticked on `origin` by the maintainer on the redated head SHA-A4 `72e13f9583ffe2b89580049045ac094ed3d46dbf`, which is PR #59's head and the last commit on the branch; CI on that head is in flight (nine runs, one concluded) and the phase is stopped on it. SHIP-07 is not closed here: it closes in plan 47-09 after the live registry re-read.**

## Result

- **Runs on SHA-A0 recorded per job.** Nine runs, read once; the `ci.yml` jobs individually; the 44 required contexts of ruleset `Protect main branch` (id 20868126) matched against the check runs. 42 `success`, one `skipped` by design (`End-to-End Tests`), one red: the required context `pre-commit run --all-files`. The `Check publish order` step of `License & Dependency Policy` is recorded by name. CI pointers were added to the audit's Evidence column. Commit `c80394fa`.
- **D-00j halt and recovery.** `end-of-file-fixer` rewrote `47-CI-EVIDENCE.md` to remove one trailing blank line (cause diagnosed; no file outside `.planning/` implicated). Nothing was fixed or re-run. The maintainer answered `Option 1` (2026-10-25T23:27:07Z); the four held local commits were pushed as SHA-A1 `b2feb0a1`; `pre-commit` concluded `success` on both SHA-A1 runs (40244991097, 40244995803).
- **The tick.** The maintainer's commit `6ef9207a` (SHA-A2) ticked the box with an uppercase `[X]`. Verified read-only on `origin`: author and committer the maintainer, one file, one line, remote tip = PR head = local HEAD, `main` unmoved. Accepted (`accept-X`).
- **The redate.** On `redate 2026-10-26` the twelve changelog headings were restamped (`c64dc37d`, `1 1` per file, twelve paths only) and re-seal row 7 was re-read on the redated tree (`02f6ee2e`: nine `check-gates` guards, `check-changelogs`, `check-release-consistency.sh --tag v0.11.0`, the R7 greps, release body 108926 bytes). The result was pushed as SHA-A3 `02f6ee2e`; all nine SHA-A3 runs concluded `success` (re-read in this continuation, `ci.yml` jobs individually; `Coverage` `success`).
- **The re-confirmation (this continuation).** SHA-A4 `72e13f95` (maintainer, 2026-10-26T13:37:14Z, parent SHA-A3) changes exactly one line of `47-ACCEPTANCE-AUDIT.md`: the sign-off box, `[X]` to `[x]`. Ticked count 1 at SHA-A3 and 1 at SHA-A4; lowercase count 0 then 1. Remote tip = PR head = SHA-A4; `main` = `df24689c3d1fd52b0e730105c15b22c2c4ad1404`; no `v0.11.0*` tag on origin. Accepted on the orchestrator's ruling (recorded verbatim in `47-CI-EVIDENCE.md` Findings, "2026-10-26 — Plan 47-06: the maintainer's re-confirmation commit SHA-A4 (orchestrator ruling)").
- **Runs on SHA-A4.** Nine, read once at 2026-10-26T13:42:54Z: `ci.yml` 40332190995 (push) and 40332201657 (pull_request), `codeql.yml` 40332190540 and 40332201694, `feature-flags.yml` 40332190592 and 40332201653, `Docs` 40332201764, `pre-commit` 40332190830 (completed, `success` at the read) and 40332201801. None else had concluded.

## Task and plan commits

| Commit | Author | What |
|---|---|---|
| `c80394fa` | agent | `docs(47-06)`: record the PR run on SHA-A0 and point the re-seal rows at their CI twins (Task 1) |
| `b2feb0a1` | agent | `docs(47-06)`: record the maintainer's decision to push SHA-A1 over the `pre-commit` red (SHA-A1) |
| `6ef9207a` | **maintainer** | `docs(47)`: sign off the v0.11.0 tag (SHA-A2; the box tick, uppercase `[X]`) |
| `c42e7100` | agent | `docs(47-06)`: verify the tick (SHA-A2), record `accept-X` and the redate decision (Task 3) |
| `c64dc37d` | agent | `chore(release)`: restamp 0.11.0 changelogs to 2026-10-26 (redate) |
| `02f6ee2e` | agent | `docs(47-06)`: re-read re-seal row 7 on the redated head (SHA-A3) |
| `72e13f95` | **maintainer** | `docs(47)`: re-confirm the v0.11.0 sign-off on the redated head (SHA-A4; the final head) |

This continuation made no commit. The SHA-A3 push record, the SHA-A4 record, the resume file, this SUMMARY, `STATE.md` and `ROADMAP.md` are uncommitted in the working tree so the maintainer's `72e13f95` stays the last commit on the branch; plan 47-08 commits them on `chore/47-close`.

## Deviations from Plan

**1. [Plan sequence] The tick preceded the redate.** The maintainer ticked SHA-A2 on 2026-10-26T02:18Z, after the 2026-10-25 UTC stamp date; the tag cannot be cut on a date earlier than the day CI finishes, so the stamp date moved. D-13's order (date correct, then tick) could not be restored by this plan; the orchestrator prescribed the maintainer's re-confirmation on the redated head to restore it. Recorded in the evidence Findings (`accept-X redate 2026-10-26`).

**2. [Plan verify] Uppercase marker accepted.** The plan's Task 3 verify greps `^- \[x\]`; the maintainer's tick was `[X]`. Accepted by the maintainer's `accept-X`; counts are case-insensitive for SHA-A2. Superseded at SHA-A4, where the marker is the plan's own lowercase `[x]` and the plan's literal verify would match.

**3. [Orchestrator-prescribed edit] The re-confirmation commit normalised the marker instead of adding the text line.** The prescribed line `Sign-off re-confirmed by the maintainer on the redated head ...` is not present at SHA-A4 (`grep -c` prints 0). Accepted by the orchestrator (authority: the maintainer's `reconfirmed 72e13f95…` reply, 2026-10-26T13:40Z). This is a deviation from the orchestrator-prescribed edit, not from the plan. The maintainer retains a blocking decision at plan 47-07 Task 2 before any merge or tag.

**4. [Plan literal] Task 3's `HEAD == SHA-A2` and "branch tip still equals SHA-A2" assertions cannot hold.** The branch advanced past the tick by design (the redate and the re-confirmation). The relevant final-head facts were verified instead (remote tip = PR head = SHA-A4; parent chain; one-line diff). In this continuation local `HEAD` was also found at SHA-A3 `02f6ee2e`, not at SHA-A4 as the orchestrator had expected; the orchestrator's fast-forward had not happened. No ref was changed here (the instructions forbid it); a fast-forward is possible and is noted in the resume file.

**5. [Records left uncommitted by design] After the tick nothing may be committed to the release branch.** Several records written after SHA-A2 were committed because the redate required docs commits before the re-confirmation (`c42e7100`, `c64dc37d`, `02f6ee2e`); everything written after the SHA-A3 push is uncommitted, as listed above. The SHA-A1 push record, which the previous continuation had left uncommitted, was committed in `c42e7100`.

**6. [Coverage figure unreadable] D-00e / A-7.** The `Coverage` job concluded `success` on every run read (SHA-A0 and SHA-A3), but its printed `Lines:` figure sits in a job log or artifact on a proxy-blocked host and is not readable from this sandbox. It is recorded as unreadable here, requested from the maintainer, and no expected number was recorded. Still outstanding with the B-1 condensation read (47-02 Task 3).

## Authentication gates

None. The `pre-commit` red, the tick and the re-confirmation were maintainer decisions and acts, handled as checkpoints.

## Known Stubs

None. This plan wrote no code and no placeholder text; the only non-evidence change outside the audit was the twelve-line changelog restamp.

## Threat Flags

None. No new endpoint, credential path or trust-boundary surface. This plan never ticked, edited or re-committed the sign-off box (T-47-23), recorded the figure as unreadable instead of expected (T-47-25), verified the tick from the remote commit and not the working tree (T-47-26), and re-ran nothing (T-47-27).

## Hard stop (D-00i)

**Resume condition:** every workflow run on SHA-A4 `72e13f9583ffe2b89580049045ac094ed3d46dbf` has `status: completed` and every required context of the `Protect main branch` ruleset has a concluded check run on it, read once. **Next action:** plan 47-07 Task 1, reading its checks on SHA-A4 wherever its text says `SHA-A2` (the plan's "tick commit, the true final commit" role now belongs to SHA-A4). Note for the plan 47-07 executor: its automated verify reads `- SHA-A2:` from the resume file and compares it with the remote tip; the file's `- SHA-A2:` is the superseded first tick `6ef9207a`, so that check must be run against `- SHA-A4:` and the substitution recorded. Nothing is left running. No push or commit may be made to the branch while the SHA-A4 runs are in flight.

## Self-Check

**PASSED.**

- Files found: `.continue-here.md`, `47-06-SUMMARY.md`, `47-CI-EVIDENCE.md` (carries `- SHA-A4: 72e13f9583ffe2b89580049045ac094ed3d46dbf`, as does the resume file).
- Commits found in the repository: `c80394fa`, `b2feb0a1`, `6ef9207a`, `c42e7100`, `c64dc37d`, `02f6ee2e`, `72e13f95`.
- Remote tip and PR #59 `head.sha` read `72e13f95` at 2026-10-26T13:42Z; this continuation made no commit, push, tag, merge, dispatch or ref change.
