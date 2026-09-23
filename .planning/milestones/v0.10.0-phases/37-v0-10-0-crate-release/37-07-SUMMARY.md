---
phase: 37-v0-10-0-crate-release
plan: 07
subsystem: infra
tags: [release, ci-evidence, coverage, codeql, required-checks, ship-05]

# Dependency graph
requires:
  - phase: 37-v0-10-0-crate-release
    plan: 06
    provides: "Release PR #55 opened, pushed to origin, CI green, then merged by the maintainer
      before the §11 tick (a recorded process-order deviation); the four maintainer
      statements (merge report, CodeQL advisory decision, D-17 bootstrap, §11 pre-tag
      sign-off) recorded verbatim with provenance in 37-CI-EVIDENCE.md."
provides:
  - "Every workflow run on PR head SHA 1bb9406343b7fb965e0724ac7a689d45e4e5f61c recorded in the
    CI-run table: ci.yml (push + pull_request, both success, job tallies with skipped-by-design
    jobs named), codeql.yml (advisory workflow-run, both success), pre-commit, feature-flags.yml,
    and the first docs.yml run on this branch -- all success."
  - "Required-context tally computed from the live main ruleset: 44 required names, 87 required
    check-run entries, 85 pass / 2 skipping (End-to-End Tests, named skipped not passed) / 0
    failures -- 44/44 satisfied."
  - "The separate red CodeQL results check (distinct from the green codeql.yml workflow runs)
    independently re-verified and recorded as advisory: 10 rust/cleartext-logging alerts
    cross-referenced to #31 #32 #33 #38 #40 #43 #44 #45 #46 #47, all created 2026-08-27, already
    open on main, GitHub's own large-PR attribution note quoted, maintainer's 'proceed with 1'
    decision cited from the existing 37-06 provenance entry. Not re-triaged as false positives."
  - "SC2 recorded from CI alone (D-00e): both Coverage job instances on the PR head SHA conclude
    success; printed figures recovered verbatim -- pull_request run Lines: 111771/123587 = 90.44%,
    Functions: 11785/14096 = 83.61%; push run Lines: 111774/123587 = 90.44%, Functions:
    11786/14096 = 83.61%. Configured floor 82 read from scripts/coverage.sh, matching ADR-0006."
affects: [37-08, 37-09, 37-10, 37-11]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/phases/37-v0-10-0-crate-release/37-07-SUMMARY.md
  modified:
    - .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md

key-decisions:
  - "Task 1's precondition query (gh run list --branch feature/phase-33) was substituted with
    SHA-scoped queries (gh api .../actions/runs?head_sha=..., .../commits/<sha>/check-runs,
    .../rules/branches/main, gh pr checks 55 --required) because the maintainer's merge deleted
    the remote feature/phase-33 branch before this plan ran. Disclosed in the evidence file's own
    'Method substitution' paragraph, not silently swapped."
  - "Task 3 (push the appended evidence to feature/phase-33) was not run. The PR is merged, the
    remote branch is deleted, and D-10 routes all further commits to chore/37-close -- pushing
    would resurrect a stale branch and start a pointless CI cycle. Both Tasks 1 and 2's commits
    stay local, riding chore/37-close per plan 37-09, D-10, exactly as this dispatch's
    <reality_delta> instructed."
  - "D-15's single-rerun exception was not invoked -- every run and every required check on the
    PR head SHA was already green; there was nothing to classify as a flake and nothing to
    re-run."
  - "The red CodeQL results check was recorded, not treated as a D-14 stop: it is not a required
    context (confirmed against the live 44-name ruleset) and the maintainer's advisory-only
    disposition is already on record from plan 37-06's continuation. This plan added only the
    independently re-verified alert-number cross-reference that entry did not itemize."

patterns-established: []

requirements-completed: []
# SHIP-05 stays Pending: this plan records pre-merge CI evidence only. It does not publish, tag,
# or touch REQUIREMENTS.md, per repo rule 6.

# Metrics
duration: ~25min
completed: 2026-09-18
status: complete
---

# Phase 37 Plan 07: Pre-Merge CI Evidence (Every Run, Required-Context Tally, Coverage) Summary

**Recorded every workflow run on PR #55's head SHA (`1bb94063`) with the required-context tally
computed from the live ruleset (44/44 satisfied), the red CodeQL results check filed separately
as an independently re-verified advisory finding, and the CI `Coverage` job's own `success`
conclusion plus its verbatim `Lines: .../... = 90.44%` figure as the sole evidence for SC2 --
Task 3's push was not run because the maintainer had already merged the PR and deleted the
remote branch before this dispatch began.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-18T21:25:00Z (approx)
- **Completed:** 2026-09-18T21:49:39Z
- **Tasks:** 2/2 executed (Task 1, Task 2); Task 3 not applicable by deviation, recorded not run
- **Files modified:** 1 (`37-CI-EVIDENCE.md`), plus this SUMMARY and STATE/ROADMAP metadata in
  the final commit

## Accomplishments

- **Task 1** recorded all 9 workflow runs on PR head `1bb9406343b7fb965e0724ac7a689d45e4e5f61c`
  (`ci.yml` push + pull_request, `codeql.yml` push + pull_request, `pre-commit` push +
  pull_request, `feature-flags.yml` push + pull_request, and the first `docs.yml` run on this
  branch), every one `success`, with `ci.yml`'s job tallies naming its skipped-by-design jobs
  (`Publish Dry Run` — tag-gated, `End-to-End Tests` — conditional, `Benchmark Regression Signal
  (Non-Blocking)` on the push run only) rather than leaving them unexplained. Filled the two
  `pending` conclusion cells plan 37-06 left in the CI-run table with the real `success`
  conclusions — the one sanctioned exception to this file's append-only rule.
- Computed the required-context tally from the **live** ruleset rather than asserting it: `gh api
  repos/DF3NDR/paladin-dev-env/rules/branches/main` returned 44 unique required context names;
  `gh pr checks 55 --required` returned 87 required check-run entries (44 names × 2 trigger
  events, `Docs` only firing on `pull_request`) — 85 `pass`, 2 `skipping` (`End-to-End Tests`,
  named skipped rather than counted as passed), 0 failures. **44/44 required contexts
  satisfied.**
- Independently re-verified the red CodeQL results check (check-run `105726197798`, distinct
  from the two green `codeql.yml` workflow runs): fetched its 10 annotations and cross-referenced
  them against the open `rust/cleartext-logging` alerts on `main`, confirming the exact alert
  numbers `#31, #32, #33, #38, #40, #43, #44, #45, #46, #47`, all `created_at: 2026-08-27`,
  already open before PR #55 existed. Confirmed `CodeQL` is absent from the 44 required names.
  Cited (not re-quoted at length) the maintainer's existing "proceed with 1" advisory decision
  from plan 37-06's provenance entry. Stated plainly that the ten alerts were not re-triaged as
  false positives.
- **Task 2** recorded the CI `Coverage` job as the sole evidence for SC2: both job instances on
  the PR head SHA (`105723181854` pull_request, `105722928700` push) conclude `success`. Fetched
  each job's raw log via a bearer-token `curl` (token obtained through `gh auth token`, never
  printed, echoed, or written to any file) and recorded the "Coverage summary" step's output
  verbatim: pull_request run `Lines: 111771/123587 = 90.44%`, `Functions: 11785/14096 = 83.61%`;
  push run `Lines: 111774/123587 = 90.44%`, `Functions: 11786/14096 = 83.61%`. Stated the verdict
  rule before quoting the number (job's own `success` conclusion is the verdict; the figure is
  corroboration only). Read the configured floor directly from `scripts/coverage.sh`
  (`FLOOR="${COVERAGE_FLOOR:-82}"`, no override set in `ci.yml`) rather than assuming `82`, and
  recorded the exact `cargo llvm-cov ... --fail-under-lines "$FLOOR"` invocation. Named Docker's
  absence from this devcontainer as the structural reason the gate is CI-attributed (D-00e).
- Disclosed the method substitution required by the branch's deletion: Task 1's precondition text
  names a branch-scoped `gh run list --branch feature/phase-33` query, but the remote branch no
  longer exists (deleted by GitHub after the maintainer's merge, per plan 37-06's own record).
  Substituted SHA-scoped `gh api` queries throughout and wrote the substitution into the evidence
  file itself, under a dedicated "Method substitution" paragraph, rather than silently swapping
  methods.
- **Did not run Task 3.** Per this dispatch's `<reality_delta>`, the PR is merged, the remote
  `feature/phase-33` branch is deleted, and D-10 already routes every post-merge/post-tick commit
  to `chore/37-close`. Pushing the appended evidence now would resurrect a stale, deleted branch
  and trigger a pointless CI cycle against a commit the maintainer has already superseded by
  merging. No `git push` of any kind was made.

## Task Commits

Each task was committed atomically, plain git, no `--no-verify`:

1. **Task 1: Record every workflow run on the PR head SHA, and classify any red result under
   D-14/D-15** - `623fc92e` (docs)
2. **Task 2: Record the CI `coverage` job's conclusion and printed figure — the sole evidence for
   SC2** - `98ae409a` (docs)
3. **Task 3: Push the appended evidence to `feature/phase-33`** - **NOT RUN, not applicable by
   deviation** (see "Deviations from Plan" below). No commit exists for Task 3, and none is
   expected — Tasks 1 and 2's commits stay local, riding `chore/37-close` per plan 37-09 (D-10).

**Plan metadata:** recorded in the final metadata commit following this SUMMARY.

## Files Created/Modified

- `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` - Task 1 filled the two `pending`
  conclusion cells the prior plan left (sanctioned exception) and appended a dated "Plan 37-07 —
  every workflow run on the PR head SHA, and the required-context tally" section (9-row CI-run
  table, the non-required Docker Build/Kubernetes Smoke Test pair, the 44-name required-context
  tally, and the independently re-verified CodeQL results-check record). Task 2 appended a
  further dated "Plan 37-07 — SC2: the CI `Coverage` job's conclusion and printed figure" section
  (job conclusions table, verbatim printed figures for both runs, the tool invocation and
  configured floor, and the CI-attribution paragraph). 104 insertions / 2 deletions in Task 1's
  commit (the 2 deletions are exactly the two sanctioned `pending` placeholder lines); 73
  insertions / 0 deletions in Task 2's commit.
- `.planning/phases/37-v0-10-0-crate-release/37-07-SUMMARY.md` - this file (new).

## Decisions Made

See `key-decisions` in frontmatter. In prose: the branch-scoped precondition query in Task 1's
own text could not run as written because the maintainer's merge (recorded in plan 37-06) deleted
the remote `feature/phase-33` branch before this dispatch began; SHA-scoped `gh api` queries
against the same PR head SHA were substituted and the substitution was written into the evidence
file itself. Task 3 was recognized as not-applicable given the same fact pattern — the dispatch's
own `<reality_delta>` said so explicitly, and this plan complied rather than second-guessing it:
pushing now would resurrect a branch GitHub already deleted and start a CI cycle nobody will read,
in service of an evidence-append the maintainer's merge has already made moot for the branch it
targeted. D-15's rerun exception was never triggered because nothing on the SHA was red among the
required checks; the one red item (the CodeQL results check) is not a required context and
already carries the maintainer's own advisory disposition from the prior plan, so it was recorded,
not stopped on.

## Deviations from Plan

### Not an auto-fixed Rule 1-3 item — recorded per the dispatch's own explicit instruction, not discovered mid-execution

**1. Task 3 (push the appended evidence to `feature/phase-33`) was not run — NOT-APPLICABLE-BY-DEVIATION.**
- **Found during:** Dispatch read, before any task began (this was pre-declared in the dispatch's
  `<reality_delta>`, not discovered by this plan's own execution).
- **What the plan says:** Task 3 instructs committing the evidence append and then, per the plan's
  `<output>` section, pushing both this plan's SUMMARY commit and Task 3's evidence commit in one
  `git push origin feature/phase-33` so the branch and its remote stay in sync before plan 37-08's
  checkpoint.
- **Why it does not apply:** By the time this plan ran, the maintainer had already merged PR #55
  (recorded in `37-06-SUMMARY.md` and in `37-CI-EVIDENCE.md`'s "Maintainer acts and statements,
  2026-09-18" entry) and GitHub had deleted the remote `feature/phase-33` branch as a
  consequence. D-10 (`37-CONTEXT.md`) already states that no further commits go onto
  `feature/phase-33` after the merge — everything post-merge routes through a `chore/37-close`
  branch cut from the tagged `main` instead. Pushing to a branch GitHub has deleted would either
  fail outright or resurrect a stale ref referencing a commit already superseded by the merge,
  and would trigger a CI cycle on a commit nobody will read as the PR head anymore.
- **What was done instead:** Tasks 1 and 2's evidence-append commits (`623fc92e`, `98ae409a`) and
  this SUMMARY's own commit stay local on the current `feature/phase-33` checkout. No push of any
  kind was made. Per this dispatch's own instruction, these commits ride `chore/37-close` when
  plan 37-09 cuts it (D-10) rather than the branch Task 3's text names.
- **Files modified:** none by Task 3 itself — the evidence content Task 3 would have pushed was
  already written and committed by Tasks 1 and 2.
- **Commits:** none for Task 3; see Tasks 1/2's commits above.

**2. Task 1's precondition query was run against SHA, not branch — a disclosed method substitution, not a silent swap.**
- **Found during:** Task 1, before the first `gh` query.
- **What happened:** the plan's precondition text and its `<verify>` block both invoke
  `gh run list --branch feature/phase-33 ...`. That branch no longer exists on the remote (see
  deviation 1 above), so the query would return nothing useful.
- **What was done instead:** substituted `gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=<sha>"`,
  `gh api repos/DF3NDR/paladin-dev-env/commits/<sha>/check-runs`,
  `gh api repos/DF3NDR/paladin-dev-env/rules/branches/main`, and `gh pr checks 55 --required` —
  all read-only GETs against the same PR head SHA the branch-scoped query would have targeted.
  The substitution is disclosed in `37-CI-EVIDENCE.md`'s own "Method substitution" paragraph, per
  this dispatch's explicit instruction, not silently applied.
- **Files modified:** `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`.
- **Commit:** `623fc92e`.

---

**Total deviations:** 2 recorded (1 task not run by explicit dispatch instruction, given a fact
pattern the dispatch itself supplied; 1 disclosed method substitution required by the same fact
pattern). Neither is a Rule 1-3 auto-fix of broken code — this plan touches no source file. Both
are pre-declared by the dispatch's own `<reality_delta>`, not discoveries this plan made and then
adjudicated on its own authority.
**Impact on plan:** No outward-facing act was performed or skipped by this plan's own choice — no
push, no tag, no PR write, no workflow dispatch, no gate re-run, no alert dismissal. The one task
not run (Task 3) was withheld under explicit dispatch instruction because running it would have
been actively wrong (resurrecting a deleted branch), not because this plan declined a task it was
supposed to do.

## Issues Encountered

None. No `pre-commit`/`cargo-clippy` contention was found running before either commit
(`pgrep -x pre-commit` / `pgrep -x cargo-clippy` both returned no match immediately before each
commit); both commits' pre-commit hooks (fmt, clippy, secret-detection, etc.) passed cleanly on
the first attempt. No `gh` query failed with an auth or permission error.

## User Setup Required

None - no external service configuration required by this plan. The bearer token used to fetch
the two `Coverage` job logs was obtained via `gh auth token` and used only in-process for a single
`curl` invocation each; it was never printed, echoed, logged, or written to any file in this
repository, and no credential-shaped text appears anywhere in this SUMMARY or in
`37-CI-EVIDENCE.md`.

## Next Phase Readiness

- All pre-merge CI evidence D-03 names as a precondition for the §11 tick is now on record:
  every workflow run on the PR head SHA, the 44/44 required-context tally, and SC2's `Coverage`
  job `success` conclusion with its verbatim figure. Plan 37-08 (per this plan's own `<output>`
  instruction) quotes the coverage figure and the required-context tally back to the maintainer.
- The PR is already merged (recorded in plan 37-06) and the physical §11 tick box in
  `.project/v0.10.0/09-program-acceptance-audit.md` remains unticked — still the maintainer's own
  hand edit, still to be made on `chore/37-close` per D-10/D-00a. No agent, including this plan,
  touched it.
- Tasks 1 and 2's two commits (`623fc92e`, `98ae409a`) plus this plan's own SUMMARY/metadata
  commit(s) all stay local on the current `feature/phase-33` checkout, unpushed, per this plan's
  explicit deviation record above. Plan 37-09 is responsible for landing this work via
  `chore/37-close` (D-10) — not by pushing to the deleted `feature/phase-33` remote.
  **Orchestrator note carried forward:** this plan's own `<output>` instruction to make "no
  progress-tracking commit... for wave 8 until `chore/37-close` exists" applies to the
  `/gsd-execute-phase` orchestrator dispatching plan 37-08 next, not to this plan itself.
- `ci.yml` on the merge commit (`1d4a9724cc219b85856a23012543458d62559e47`, run `35396397097`)
  was reported `in_progress` as of plan 37-06's own SUMMARY; this plan did not check it further
  (out of scope — post-merge evidence belongs to plan 37-09 per this dispatch's own instruction).
- `SHIP-05` remains `Pending` in `.planning/REQUIREMENTS.md`; not touched by this plan.

## Self-Check

**Files:**
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`
- FOUND: `.planning/phases/37-v0-10-0-crate-release/37-07-SUMMARY.md` (this file)

**Commits:**
- FOUND: `623fc92e` (Task 1)
- FOUND: `98ae409a` (Task 2)

**Diff integrity:** `git diff --numstat 623fc92e~1 623fc92e` reports `104 2` for
`37-CI-EVIDENCE.md` — the 2 deletions are exactly the two `pending` placeholder lines repo rule 4
explicitly authorizes editing. `git diff --numstat 98ae409a~1 98ae409a` reports `73 0` — additions
only. `git diff --diff-filter=D --name-only` on both commits is empty (no file deleted).

**External verification (read-only, re-run by this plan, not merely restated):** both `ci.yml`
runs (`35382874018` push, `35382953376` pull_request) `conclusion: success` at head `1bb94063`;
both `Coverage` job instances (`105722928700`, `105723181854`) `conclusion: success`; the
required-context ruleset query returns exactly 44 names; `gh pr checks 55 --required` returns 87
entries, 85 `pass` / 2 `skipping` / 0 other; check-run `105726197798` (`CodeQL`) `conclusion:
failure` with annotations matching alerts `#31 #32 #33 #38 #40 #43 #44 #45 #46 #47`, all
`created_at: 2026-08-27T12:52:26Z`.

## Self-Check: PASSED

---
*Phase: 37-v0-10-0-crate-release*
*Completed: 2026-09-18*
