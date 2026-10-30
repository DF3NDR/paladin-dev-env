---
phase: 52-ci-release-pipeline-repair
plan: 06
subsystem: infra
tags: [ci, github-actions, rulesets, release-docs, release-gates]

requires:
  - phase: 52-01
    provides: "Semver Checks (vs v0.11.0) job name; migration-allowlist step in the required License & Dependency Policy job"
  - phase: 52-02
    provides: "PALADIN_RELEASE_BODY_LIMIT / PALADIN_RELEASE_BODY_RESERVE, truncation sentinel, release-body-fit.sh"
  - phase: 52-04
    provides: "finalize re-fit of the curated section, logged sizes"
provides:
  - "ci.yml: pull_request activity types incl. labeled; End-to-End Tests under the release label; Publish Dry Run on every push and PR"
  - "branching-model.md trigger-register row for the labeled activity type"
  - "committed required-context record (46 contexts) incl. Publish Dry Run and Semver Checks (vs v0.11.0)"
  - "branch-protection.md and cicd.md: 46-context set and skipped-job-reports-success semantics"
  - "release-automation.md and release-checklist.md: release-label act, accepted publish-order reading, release-body size limit"
affects: [52-07, 52-08]

tech-stack:
  added: []
  patterns:
    - "Label condition is an element match over the labels array (contains(github.event.pull_request.labels.*.name, 'release')), only inside a job-level if:, never in a run: block"
    - "A required job gated by a job-level if: is not a ruleset change: the skip reports success to required-check evaluation"

key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - docs/src/contributing/branching-model.md
    - .github/rulesets/protect-main-branch.json
    - docs/src/appendix/branch-protection.md
    - docs/src/deployment/cicd.md
    - docs/src/appendix/release-automation.md
    - docs/src/appendix/release-checklist.md

key-decisions:
  - "PD-06-1 executed at the planner default (46 contexts); still awaiting user confirmation, carried to handover H-01"
  - "PD-06-2: End-to-End Tests stays an already-required context with no ruleset change"
  - "PD-06-3: any label event re-runs the whole ci.yml including the roughly 63-minute Docker build; documented, not engineered around"
  - "PD-06-4: header and concurrency comments corrected in prose only; the group: and cancel-in-progress: expressions are unchanged"
  - "cicd.md notes that the Required marks on the two new rows describe the committed record until the maintainer's PUT (H-01)"

patterns-established:
  - "Docs state the live ruleset changes only on the maintainer PUT plus read-back; the committed JSON is a record"

requirements-completed: [CIREL-04, CIREL-09]

coverage:
  - id: D1
    description: "ci.yml PR activity types include labeled; e2e-tests runs on main pushes or release-labelled PRs via contains() over the labels array; publish-dry-run has no job-level if and keeps needs [lint, test]; concurrency group unchanged"
    requirement: "CIREL-04"
    verification:
      - kind: other
        ref: "python3 yaml assertions on ci.yml (types, branches, publish-dry-run if/needs, e2e-tests if/needs, concurrency group)"
        status: pass
      - kind: other
        ref: "no run: block in ci.yml references github.event.pull_request.labels (python scan)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Trigger-policy register row and committed ruleset record (46 sorted contexts) agree with the workflow"
    requirement: "CIREL-04"
    verification:
      - kind: unit
        ref: "scripts/check-workflow-triggers.sh (clauses 1-3) and tests/scripts/check-workflow-triggers_test.sh (11 assertions)"
        status: pass
      - kind: other
        ref: "python3 json assertion: 46 contexts, sorted, includes Publish Dry Run, Semver Checks (vs v0.11.0), End-to-End Tests"
        status: pass
    human_judgment: false
  - id: D3
    description: "branch-protection.md, cicd.md, release-automation.md and release-checklist.md state the 46-context set, label-gated E2E semantics, release-label act, publish-order reading (CIREL-09, D-16) and release-body size limit"
    requirement: "CIREL-09"
    verification:
      - kind: other
        ref: "grep verify commands from plan tasks 2 and 3 (all pass); git diff --quiet HEAD -- .github/workflows/release.yml"
        status: pass
    human_judgment: false
  - id: D4
    description: "actionlint acceptance of the edited ci.yml"
    requirement: "CIREL-04"
    verification: []
    human_judgment: true
    rationale: "actionlint is unavailable in this sandbox (A-52-03); CI's required Workflow Lint job is the gate once the branch is pushed"
  - id: D5
    description: "Observation on a live PR that a label-less PR skips End-to-End Tests without blocking, and a release-labelled PR runs it; live ruleset PUT"
    requirement: "CIREL-04"
    verification: []
    human_judgment: true
    rationale: "Needs a live PR and a maintainer ruleset PUT (handover H-01, H-02); the gh token in this sandbox is invalid and D-08 reserves the PUT to the maintainer"

duration: about 35min
completed: 2026-10-29
status: complete
---

# Phase 52 Plan 06: Pre-merge gate path and release docs Summary

**A PR now runs Publish Dry Run always and End-to-End Tests under the maintainer-applied `release` label (new `labeled` activity type), the committed ruleset record lists 46 required contexts, and the release docs record the label act, the accepted publish-order reading and the release-body size limit.**

## Performance

- **Duration:** about 35 min
- **Completed:** 2026-10-29
- **Tasks:** 3 of 3 (Task 1 was the tracer)
- **Files:** 0 created, 7 modified

## Accomplishments

- CIREL-04 / D-05: `on.pull_request.types` is `[opened, synchronize, reopened, labeled]`. `e2e-tests` carries a folded `if: >-` of `(push && ref == main) || (pull_request && contains(github.event.pull_request.labels.*.name, 'release'))` with its stale troubleshooting comment lines removed. `publish-dry-run` lost its job-level `if:` and keeps `needs: [lint, test]`. The `group:` and `cancel-in-progress:` expressions are untouched.
- PD-06-4: the header and concurrency comments no longer claim push and PR runs share a group; they say the groups differ (bare branch name versus `refs/heads/<branch>`) and that `cancel-in-progress` collapses repeated runs of the same event. The `Check publish order` comment now says `publish-dry-run` also runs on every PR.
- D-00d: the `ci.yml` row of the trigger-policy register is still one pipe line, with the `labeled` activity type in the Triggers cell and a description sentence covering the `release` label condition, case-insensitive `contains()` and the accepted whole-file re-run.
- D-08 (record only): `.github/rulesets/protect-main-branch.json` has 46 sorted contexts, adding `Publish Dry Run` and `Semver Checks (vs v0.11.0)`; the other 44 entries are untouched. No `gh api` call was made against rulesets; the live PUT is handover H-01.
- D-08 wording: `branch-protection.md` says 46 (three counts), lists both names and carries a dated "Updated 2026-10-28 (Phase 52, D-08)" paragraph (E2E needs no change because a job skipped by its job-level `if:` reports success, PR #59 precedent; takes effect only after the maintainer's PUT and read-back). The `cicd.md` rows for `cargo-deny`, `semver`, `e2e-tests` and `publish-dry-run` read as specified, and a short note says the new Required marks describe the committed record until the PUT.
- D-07 / D-16 / D-11: `release-automation.md` has the dated publish-order gate paragraph (both halves named, A-SC3 pointer to 47-01-PLAN.md and 47-08-SUMMARY.md, no `release.yml` step added), Operator Guide step 6 edited in place (18 numbered lines before and after) with the release-label act, and a `**Size limit.**` paragraph under `### Body source`. `release-checklist.md` gained the no-manual-condensation bullet, `make check-semver`, the release-label bullet, and the corrected section 5 sentence.

## Task Commits

1. **Task 1: pre-merge gate path (tracer)** - `2644b974c` (ci)
2. **Task 2: required-check set and job semantics docs** - `436dedee6` (docs)
3. **Task 3: release-label act, publish-order reading, size limit docs** - `b5649ab7f` (docs)

Metadata commit: see git log (docs(52-06)).

## Tracer evidence (Task 1)

- **Red first:** the plan's YAML assertion run against the unedited tree failed with `KeyError: 'types'` on `on['pull_request']['types']`, exit 1.
- **Green:** after the edits the YAML and JSON assertions printed `STATIC ASSERTIONS OK`. After the commit, `./scripts/check-workflow-triggers.sh` (7 workflow files, 7 register rows, all clauses pass), `./tests/scripts/check-workflow-triggers_test.sh` (11 assertions), `./scripts/check-workflow-suppressions.sh` and its harness (21 assertions) all passed. `grep -c 'use this for troubleshooting'` prints 0, no `run:` block references the labels, and `release.yml` is unchanged.
- **Mutation check:** (A) turning the label test into a substring test over `join(...)` failed the YAML assertion; (B) dropping `labeled` from `types:` failed it with the `types` list printed; (C) renaming the ruleset context to `Publish Dry-Run` made `check-workflow-triggers.sh` clause 3 exit 1 with its rename remediation text. Each file was restored with `git checkout -- <file>`; the tree was clean and all checks green again afterwards.
- **Tracer gate:** per the run's checkpoint policy the executor-rule tracer checkpoint was not stopped on; the evidence above is recorded for the orchestrator.

## Verification (plan level)

- `make check-gates` exit 0; `make test-shell-guards` exit 0.
- Task 2 and Task 3 grep verify commands pass; `git diff --quiet HEAD -- .github/workflows/release.yml` is clean.
- Not run: `actionlint` (not installed here, A-52-03); CI's required `Workflow Lint` job is the gate on the pushed branch. `mdbook` is not installed either, so CI's `Build MDBook` is the gate for the docs edits (no new links to nonexistent pages: the `branch-protection.md` link from `cicd.md` and the `#body-source` anchor were checked to exist).

## Decisions Made

Followed PD-06-1 to PD-06-4 as planned; see `key-decisions`. PD-06-1 (adding `Semver Checks (vs v0.11.0)` as a required context) remains a planner assumption awaiting user confirmation; veto path is 52-08 Task 3 step 3.

## Deviations from Plan

### Auto-fixed Issues

None - plan executed exactly as written.

### Attribution

**1. [Attribution] Commit trailer model name**
- The orchestrator brief specified `Co-Authored-By: Claude Fable 5.1`; the harness attribution reminder for this session names `Claude Sonnet 5.5`, which is the model actually running. As in plans 52-02 and 52-04, all three commits carry the Sonnet 5.5 trailer plus the `Claude-Session` line (the first commit was amended before any further work).

## Issues Encountered

None. The workflow-suppression and workflow-trigger harnesses were run only after the Task 1 commit, as they require a clean `.github/workflows/` tree. Scratch files live under the session scratchpad only.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or trust-boundary access beyond the plan's threat model: the trigger stays `pull_request` (no `pull_request_target`), job permissions are unchanged, and the label name appears only inside the `contains()` expression of a job-level `if:` (T-52-23, T-52-24 mitigated; T-52-25 and T-52-27 mitigated by the docs and handover H-01/H-02; T-52-26 accepted).

## Next Phase Readiness

- Plan 52-08 handover items: H-01 (ruleset PUT of the 46-context file plus read-back, and the plain confirm-or-veto of `Semver Checks (vs v0.11.0)` as required) and H-02 (observe E2E skipped on a label-less PR and running on a release-labelled PR).
- Pending on push: CI `Workflow Lint` and `Build MDBook` confirmation.

## Self-Check: PASSED

Files verified present: all seven modified files. Commits `2644b974c`, `436dedee6` and `b5649ab7f` verified in git log.
