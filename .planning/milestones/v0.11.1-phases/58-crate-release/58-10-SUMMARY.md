---
phase: 58-crate-release
plan: 10
subsystem: release-evidence-and-audit
tags: [release, ci-evidence, milestone-audit, nyquist, codeql, ship-10, tag-day]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-09 (SHA-A0 801dd237e, release PR 64, labelled ci.yml run 42131869587)
provides:
  - "per-run, per-job CI record for SHA-A0 with the 46-of-46 required-context tally, Coverage 92.86 %, the PR-event semver counts, E2E, Publish Dry Run, SDK smoke and MSRV reads"
  - "CI-twin pointers (job URLs) in 58-ACCEPTANCE-AUDIT.md; box still unticked"
  - ".planning/v0.11.1-MILESTONE-AUDIT.md, status passed (first computed tech_debt), scope Phases 52-57"
  - "the maintainer's tag-day, CodeQL and audit decisions recorded verbatim for plan 58-11"
affects: [58-11, 58-12, 58-13, 58-14]

tech-stack:
  added: []
  patterns:
    - "Read CI once per run and per job through REST (manual paging), tally required contexts by name against the live ruleset, never re-run"
    - "Compute the milestone-audit status from the workflow's own rules, write the honest first result, and re-read after the cause is resolved"

key-files:
  created:
    - .planning/v0.11.1-MILESTONE-AUDIT.md
    - .planning/phases/58-crate-release/58-10-SUMMARY.md
  modified:
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/phases/58-crate-release/58-ACCEPTANCE-AUDIT.md
    - .planning/phases/54-first-run-usability/54-VALIDATION.md
    - .planning/phases/56-dogfood-findings/56-VALIDATION.md

key-decisions:
  - "Tag day is 2026-11-06 (redate from STAMP 2026-11-05): plan 58-11 redates the twelve changelog headings in one docs(release) commit and re-reads re-seal row 7"
  - "CODEQL=dismiss-now: the maintainer dismisses the eight F-58-08 alerts on GitHub; plan 58-11 adds eight governed CODEQL-DISMISSALS.md rows with make check-codeql-dismissals green"
  - "AUDIT=validate-first: Phases 54 and 56 validated by a planning-only nyquist audit (47fd13d53) before the audit was re-read; the milestone audit now reads passed with tech_debt: []"
  - "Milestone audit integration check ran inline (no agent-spawn tool in this runtime), labelled inline in the report"

requirements-completed: []

duration: "about one hour of executor clock across three segments (00:59Z to about 01:55Z on 2026-11-06), plus the checkpoint pause"
completed: 2026-11-06
---

# Phase 58 Plan 10: CI Evidence on SHA-A0, Pre-Tag Milestone Audit, Tag-Day Decision Summary

**Every run and job on `SHA-A0` is recorded (46 of 46 required contexts `success`, `Coverage` 92.86 % against the 82 % floor, semver 12 packages x 223 checks with 0 failures), the pre-tag milestone audit over Phases 52-57 read `tech_debt` first and reads `passed` after the maintainer's `AUDIT=validate-first` choice validated Phases 54 and 56, and the maintainer named 2026-11-06 as the tag day.**

## Task results

| Task | Name | Commit | Result |
|------|------|--------|--------|
| 1 (tracer) | Every run on SHA-A0 per job, required-context tally, Coverage and semver reads, CI twins | `5f1d91a57` | verify passed; no red anywhere, nothing re-run |
| 2 | Pre-tag milestone audit over Phases 52-57 | `402617c84` (status `tech_debt`), re-read in `c89b6fbca` (status `passed`) | full Task 2 verify passes in the re-read |
| (orchestrator) | Validate Phases 54 and 56 (planning-only nyquist audit) | `47fd13d53` | both `status: validated`, `nyquist_compliant: true`; no code or test file changed |
| 3 | Sign-off packet and tag-day decision | `c89b6fbca` (decision recorded) | decision recorded verbatim; resolved to 58-11 |

## The tracer: CI on SHA-A0 (Task 1)

- Labelled `ci.yml` run `42131869587` (event `pull_request`, created after the `release` label): `success`, 39 jobs all `success`, `run_attempt` 1. The other two `ci.yml` runs and the `feature-flags.yml` and `pre-commit` runs on `SHA-A0` are also `success`; the only non-success jobs are four `skipped` ones, none a required context's deciding run.
- Required contexts (live ruleset `rules/branches/main`): **46 of 46 `success`**, tallied by name in `58-CI-EVIDENCE.md`.
- `Coverage`: **92.86 %** line coverage against the 82 % floor (ADR-0006).
- PR-event `Semver Checks (vs v0.11.0)`: 12 packages, `223 checks` each, `semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`. The twelve CI counts equal the twelve local counts; the zero-check tripwire did not fire.
- `End-to-End Tests` `127617989806` `success` (not `skipped`; 12 tests passed), `Publish Dry Run` `127613034679` `success`, SDK smoke `127610217747` `success`, `MSRV (Rust 1.88)` `127610217740` `success`, `pre-commit run --all-files` `success`.
- D-00j did not trip: the F-58-03 flake rule and the infrastructure re-run were not needed.
- The acceptance audit's rows carry their CI twins' job URLs (row 5 MSRV now reads its CI result); the box is still unticked.

**PR-head note.** The PR head moved from `SHA-A0` to `d17716713` (three `docs(58-09)` commits pushed by the orchestrator at about 2026-11-06T00:57Z, after the labelled run was created). `git diff --name-only 801dd237e d17716713` lists five files, all under `.planning/`; code and manifests equal `SHA-A0`. Recorded, not hidden, in `58-CI-EVIDENCE.md`.

## The milestone audit: tech_debt, then passed (Task 2)

| | First computation (2026-11-06T01:08:54Z, head `5f1d91a57`) | Re-read (head `47fd13d53`) |
|---|---|---|
| status | `tech_debt` | **`passed`** |
| Nyquist (section 5.5) | 4 compliant / 0 partial / 2 not-validated (Phases 54, 56: `draft`, `nyquist_compliant: false`) | 6 compliant / 0 partial / 0 not-validated / 0 missing |
| `tech_debt:` | two items (54 and 56 NOT-VALIDATED) | `[]` |
| ROADMAP exit criterion 2 | not met | met |

- The first computation was the honest one under P-10-1: requirements 49/50 (FIXES-04 re-homed, row `R-13`), phases 6/6, no `unsatisfied` requirement (section 5e), but two NOT-VALIDATED Nyquist files are accumulated non-critical debt, so `passed` was not given.
- Resolution: at the Task 3 checkpoint the maintainer replied `AUDIT=validate-first`; the orchestrator's planning-only nyquist audit (`47fd13d53`) validated Phase 54 (14 rows: 12 covered, 2 manual-only) and Phase 56 (17 rows: 10 covered, 7 manual-only accepted, each with a recorded reason). No code or test file changed (D-00j: the re-seal head stays sealed; `git diff --name-only 801dd237e HEAD -- src crates Cargo.toml Cargo.lock` is empty).
- Re-read: all six `VALIDATION.md` files read `status: validated`, `nyquist_compliant: true`; every other read (requirements, phases, integration 6/6, flows 4/4) was re-checked against `HEAD` and unchanged. The Task 2 verify now passes in full, including the two clauses (`status: passed`, `tech_debt: []`) that failed by design the first time. `R-13` is the only register row cited and exists in the register.
- Exit-criteria commands, re-run on `47fd13d53`: `windows status` `open_count: 0` (waived 53, fixed 55, total 108); `check-review-dispositions.sh` `checked 51 finding(s) across 13 review file(s); 0 failure(s)`; `make check-findings-log` OK (36 entries); strict register verifier `checked 135 row(s) (R: 95, V: 40); fixed 52, waived 37, re-homed 46; 0 failure(s)`.
- Integration check: **inline** (this runtime cannot spawn `gsd-integration-checker`); labelled inline in the report.
- A caveat in `56-VALIDATION.md` and `56-VERIFICATION.md` (`make security` red on RUSTSEC-2026-0429) is stale for this head: Phase 57 closed it (`.cargo/audit.toml` row and the `SECURITY-EXCEPTIONS.md` entry; CI `Security Audit` and `License & Dependency Policy` are `success`, part of the 46 of 46).

## F-58-08 (CodeQL, recorded and carried)

Check run `CodeQL` (id `127553770360`, Code Scanning) on `SHA-A0` concluded `failure`: eight new high `rust/cleartext-logging` alerts at `src/infrastructure/web/run_api_wiring.rs` (lines 1377, 2934, 2937, 2941, 2943, 2944, 2945) and `src/main.rs:59`. Each was read line by line and is a false positive of the taint rule (the production line logs a fixed sentence; the other six are a unit test's assertion and `panic!` messages; `main.rs:59` logs `settings_summary`, whose only key-list value is a `.len()` count). Disposition `waived` in the audit's `dispositioned_findings:` with five compensating controls. The maintainer answered `CODEQL=dismiss-now`; the dismissal on GitHub is the maintainer's own write, and the eight governed rows plus `make check-codeql-dismissals` green are plan 58-11's. The agent dismissed nothing and changed no source line. Until dismissed the check run still reads `failure` on the PR; it is advisory-only and not one of the 46 required contexts.

## Maintainer decision (Task 3), verbatim

Reply on 2026-11-06 (UTC):

```
redate:2026-11-06 CODEQL=dismiss-now AUDIT=validate-first
```

| Answer | Value | Lands in |
|--------|-------|----------|
| tag day | `redate:2026-11-06` (STAMP was `2026-11-05`) | 58-11: one `docs(release)` commit redating the twelve `## [0.11.1] - 2026-11-05` headings to `2026-11-06`, then a re-read of re-seal row 7 on that head (D-00n) |
| `CODEQL` | `dismiss-now` | the maintainer dismisses the eight alerts on GitHub; 58-11 adds eight governed rows to `CODEQL-DISMISSALS.md`, `make check-codeql-dismissals` green |
| `AUDIT` | `validate-first` | done: `47fd13d53`, then the audit re-read in this plan |

Also recorded under `## Maintainer decisions (58-10)` in `58-CI-EVIDENCE.md`.

## Sign-off packet for plan 58-11's tick

What the maintainer receives at the tick, all read from the files:

- `SHA-A0` `801dd237e` (PR head `d17716713`, planning-only difference): labelled `ci.yml` run `42131869587` `success`; 46 of 46 required contexts `success`; `Coverage` 92.86 %; semver 12 x 223 checks, 0 failures; `End-to-End Tests`, `Publish Dry Run`, SDK smoke and MSRV `success`.
- The eight re-seal results on `SHA-R` / `SHA-R1` with their CI twins in `58-ACCEPTANCE-AUDIT.md` (box unticked).
- `.planning/v0.11.1-MILESTONE-AUDIT.md`: `status: passed`, requirements 49/50 (FIXES-04 re-homed, `R-13`), phases 6/6, `tech_debt: []`; register 135 rows, 0 failures.
- To come in 58-11 and not done here: the 2026-11-06 redate commit and the row 7 re-read; the eight `CODEQL-DISMISSALS.md` rows and the maintainer's GitHub dismissal of the eight alerts; the push of the evidence as `SHA-A1`; CI green on the tick commit before the merge and tag hand-off (D-04). After the tick the agent commits nothing on the release branch.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `gh api --paginate` does not work through the sandbox proxy**
- **Found during:** Task 1
- **Issue:** the plan's interfaces name `actions/runs/<id>/jobs --paginate` and `commits/<A0>/check-runs --paginate`; paginated calls break through the agent proxy.
- **Fix:** paged manually with `per_page=100&page=N` until a page came back empty (163 jobs, 167 check runs); results are identical to a paginated read.
- **Files modified:** none (read method only; noted in `58-CI-EVIDENCE.md`)

**2. [Rule 1 - Bug] `feature-flags.yml` job count corrected**
- **Found during:** Task 1
- **Issue:** the plan 58-07 read of `feature-flags.yml` on `SHA-P2` records 21 jobs; both runs on `SHA-A0` read 20 jobs, all success.
- **Fix:** the `SHA-A0` section records the count measured on `SHA-A0` (20: 20 success, per run); the older row (a different head) is left as the record of its own read.
- **Files modified:** `.planning/phases/58-crate-release/58-CI-EVIDENCE.md`

**3. [Rule 1 - Bug] Publish Dry Run runs on pull-request runs**
- **Found during:** Task 1
- **Issue:** the plan 58-08 note says `Publish Dry Run` runs only on `main` pushes; it ran and passed on the labelled pull-request run (`127613034679`) and on the two earlier `SHA-A0` runs.
- **Fix:** the evidence records the actual conclusions and marks the earlier note superseded.
- **Files modified:** `.planning/phases/58-crate-release/58-CI-EVIDENCE.md`

### Plan expectation not met first time, then met

- Task 2's expected `status: passed` did not hold on the first computation (Phases 54 and 56 NOT-VALIDATED). Per P-10-1 the honest `tech_debt` was written and committed (`402617c84`), the plan halted at the Task 3 checkpoint, and the status was changed only after the cause was resolved by the maintainer's decision, with the first computation preserved in the audit body and frontmatter.

### Authentication gates

None.

## Known Stubs

None. This plan created and edited release documents only; no source file changed.

## Threat Flags

None. No network endpoint, auth path, file-access pattern or schema changed.

## Issues Encountered

- The `47fd13d53` commit trailer names a different model identifier than this plan's other commits; it is the orchestrator's local commit, was left as made (not amended), and is not pushed.

## Self-Check: PASSED

- Found: `.planning/v0.11.1-MILESTONE-AUDIT.md`, `.planning/phases/58-crate-release/58-CI-EVIDENCE.md` (with `## Maintainer decisions (58-10)`), `.planning/phases/58-crate-release/58-ACCEPTANCE-AUDIT.md`.
- Found commits: `5f1d91a57`, `402617c84`, `47fd13d53`, `c89b6fbca`.
- Task 1 verify (evidence and acceptance audit clauses) passes; Task 2 verify passes in full (`status: passed`, `tech_debt: []`, `R-13` exists in the register, `open_count` 0).
- Nothing pushed; `git status` shows no uncommitted tracked changes before this file.
