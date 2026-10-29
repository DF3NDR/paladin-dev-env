---
phase: 52-ci-release-pipeline-repair
plan: 07
subsystem: infra
tags: [ci, semver, cargo-semver-checks, red-proof, release-gates]

requires:
  - phase: 52-01
    provides: "scripts/check-semver.sh, Semver Checks (vs v0.11.0) job, Semver check counts annotation"
  - phase: 52-05
    provides: "52-CI-EVIDENCE.md D-02 twelve-package green table; crate-wide semver allow lines removed"
  - phase: 52-06
    provides: "pre-merge gate path (push runs ci.yml on every branch)"
provides:
  - "52-SEMVER-RED-EVIDENCE.md: local proven-red record (function_missing, 222 pass / 1 fail), green baseline (local and CI), CI half with the scratch run URLs, close-out and H-03"
  - "scratch/52-semver-red-proof pushed to origin (one commit, never an ancestor of the phase branch); its ci.yml push run is in flight"
affects: [52-08]

tech-stack:
  added: []
  patterns:
    - "Red proof by one scripted visibility edit outside the repository, guarded by DIRTY-BASELINE, an EXIT trap and MUTATION-NOT-APPLIED; verdict by counts and presence only"

key-files:
  created:
    - .planning/phases/52-ci-release-pipeline-repair/52-SEMVER-RED-EVIDENCE.md
  modified: []

key-decisions:
  - "PD-07-1 executed: the break is the leaf-crate paladin_storage::cron::cron_field_count (pub to pub(crate)); detected on the first attempt, so the paladin-ports fallback was not needed"
  - "PD-07-2 executed: maintainer answered Push-ok; gh pr create was refused (GraphQL 403) and no REST workaround was attempted because Push-ok covered one gh pr create attempt plus the compare URL"
  - "The remote scratch branch was deliberately NOT deleted at close-out: its push-event semver run is the only CI red evidence and had not concluded; deletion is H-03 step 3"

requirements-completed: [CIREL-01]

coverage:
  - id: D1
    description: "Local red proof: one public-API break makes ./scripts/check-semver.sh go red on paladin-storage alone (function_missing on cron_field_count, 223 checks: 222 pass, 1 fail), dependents paladin-eval and paladin-ai stay green, tree restored"
    requirement: "CIREL-01"
    verification:
      - kind: other
        ref: "red_proof_semver.sh (kept outside the repo, verbatim in the evidence file): RED-PROVEN, six proof conditions met; git diff --quiet -- crates/ exit 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Green baseline on a real CI run: Semver Checks (vs v0.11.0) job on the phase branch, success, twelve non-zero check lines"
    requirement: "CIREL-01"
    verification:
      - kind: other
        ref: "https://github.com/DF3NDR/paladin-dev-env/actions/runs/40807444391/job/123230155353 (head 0c2b4a2bd, conclusion success, 12 lines of 223 checks read from the public annotations endpoint)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Red Semver Checks (vs v0.11.0) job URL and failing check line from the scratch branch run"
    requirement: "CIREL-01"
    verification: []
    human_judgment: true
    rationale: "Job https://github.com/DF3NDR/paladin-dev-env/actions/runs/40808979632/job/123234542712 was still in_progress when the bounded poll ended; capture is handover H-03 step 1 (public API, no token)"
  - id: D4
    description: "Draft PR for the scratch branch, PR closed unmerged, remote scratch branch deleted"
    requirement: "CIREL-01"
    verification: []
    human_judgment: true
    rationale: "gh pr create refused in this sandbox (GraphQL 403); the maintainer opens it from the compare URL if a pull_request-event run is wanted; remote delete follows the red capture (H-03 steps 2-3)"

duration: about 70min across two executor sessions (includes about 10min of bounded CI polling)
completed: 2026-10-29
status: complete
---

# Phase 52 Plan 07: Semver gate proven-red evidence Summary

**One leaf-crate visibility edit (`paladin_storage::cron::cron_field_count`, `pub` to `pub(crate)`) turns the re-armed semver gate red on `paladin-storage` alone locally (`223 checks: 222 pass, 1 fail`, `function_missing`); the scratch branch is pushed and its CI semver run is in flight, with the red job capture and the remote branch delete handed over as H-03.**

## Performance

- **Duration:** about 70 min across two sessions, including about 10 min of bounded CI polling
- **Completed:** 2026-10-29
- **Tasks:** 3 of 3 (Task 1 was the tracer; Task 2 was the decision checkpoint)
- **Files:** 1 created (the evidence file), 0 modified

## Checkpoint reply

Task 2 (`checkpoint:decision`, `gate="blocking"`). Maintainer reply, verbatim, received 2026-10-29 UTC via the orchestrator's chat:

> `Push-ok`

Recorded at 2026-10-29T02:46Z (start of the continuation session). It selected option 1, `push-ok`: the executor creates the scratch commit, pushes both branches, tries `gh pr create --draft`, and prints the compare URL if that is refused.

## Accomplishments

- **Task 1, local red proof (commit `0c2b4a2bd`, previous session).** `./scripts/check-semver.sh --package paladin-storage` with the break exited 1 (the tool inside exited 100): `223 checks: 222 pass, 1 fail, 0 warn, 31 skip`, `function_missing` on `paladin_storage::cron::cron_field_count`. `paladin-eval` and `paladin-ai` exited 0 with `223 checks: 223 pass, 31 skip`. The script's `RED-PROVEN` verdict covered all six proof conditions by counts and presence, the tree was restored and `git diff --quiet -- crates/` held. The primary break was detected, so the `paladin-ports` fallback was not used. No `paladin-herald` and no non-default-feature item was used.
- **Task 3, push and CI half (commit `161c11a29`).**
  - From the clean phase HEAD `0c2b4a2bd`, `git switch -c scratch/52-semver-red-proof`, the same single `sed` (grep count 1), one commit `6f3d8d7845780d40e6f49b05127b06a60fecce96` with subject `chore(scratch): deliberate semver break for the Phase 52 red proof -- DO NOT MERGE` (one file, 1 insertion, 1 deletion).
  - `git push origin scratch/52-semver-red-proof` exit 0 (2026-10-29T02:53:08Z); `git switch -` exit 0; `git push origin claude/laughing-dirac-e0h2ax` exit 0 (`Everything up-to-date`, `0c2b4a2bd` was already on origin).
  - `gh pr create --draft --base main --head scratch/52-semver-red-proof --title "DO NOT MERGE: Phase 52 semver red proof" --body "Throwaway branch for D-04; closed unmerged."` was **refused, exit 1**: `HTTP 403: GitHub GraphQL is not available from Claude Code sessions; use the REST API (gh api repos/{owner}/{repo}/...) ...` (full text in the evidence file). No REST workaround was attempted: `Push-ok` covered one `gh pr create` attempt plus the compare URL. Compare URL for the maintainer:
    `https://github.com/DF3NDR/paladin-dev-env/compare/main...scratch/52-semver-red-proof?expand=1`
  - **Green CI baseline captured.** Phase-branch `ci.yml` run <https://github.com/DF3NDR/paladin-dev-env/actions/runs/40807444391>, job <https://github.com/DF3NDR/paladin-dev-env/actions/runs/40807444391/job/123230155353> (`Semver Checks (vs v0.11.0)`, head `0c2b4a2bd`), conclusion `success`. The public annotations endpoint was readable without a token (A-52-17 holds): twelve lines, every one `223 checks: 223 pass, 31 skip`, matching the local table.
  - **Red CI job: pending.** Scratch run <https://github.com/DF3NDR/paladin-dev-env/actions/runs/40808979632> (`push` event, head `6f3d8d784`), job <https://github.com/DF3NDR/paladin-dev-env/actions/runs/40808979632/job/123234542712>. Eight polls 70 s apart (02:53:52Z to 03:02:17Z) plus final reads at 03:03:38Z and 03:04:29Z all showed `in_progress` (step 6, the gate itself, running; the comparable green job took 9 min 30 s). The bounded window ended, so no conclusion is claimed.
  - **Containment.** `git merge-base --is-ancestor 6f3d8d784 HEAD` exits 1; `git log --format=%s HEAD | grep -cF 'deliberate semver break'` prints 0; `git diff --quiet -- crates/` exits 0; no `crates/` change between `0c2b4a2bd~1` and HEAD; `red_proof_semver.sh` is not tracked.
  - **Close-out.** Local `git branch -D scratch/52-semver-red-proof` done (was `6f3d8d784`; `refs/heads/scratch/52-semver-red-proof` absent). The remote branch was left in place on purpose (see H-03).

## Task Commits

1. **Task 1 (tracer): local red proof** - `0c2b4a2bd` (docs)
2. **Task 2: maintainer decision** - no commit (reply recorded above)
3. **Task 3: CI half, refused PR, close-out, H-03** - `161c11a29` (docs)
4. **Scratch commit, on `scratch/52-semver-red-proof` only, never merged** - `6f3d8d7845780d40e6f49b05127b06a60fecce96` (chore(scratch))

Metadata commit: see git log (docs(52-07)).

## Handover item H-03 (open): capture the red semver job, then delete the scratch branch

Full text with commands is in `52-SEMVER-RED-EVIDENCE.md` under `## Close-out`. In brief, for 52-08's `52-HANDOVER.md`:

1. Read the red job (no token needed) and record its conclusion, URL and the `paladin-storage` check line under `## Experiment 2 -- CI`; set the evidence frontmatter `status:` to `ci-proven`:
   `curl -fsS "https://api.github.com/repos/DF3NDR/paladin-dev-env/actions/jobs/123234542712"` and `.../check-runs/123234542712/annotations`. Expected: `completed`, `failure`, a `paladin-storage` line with ` 1 fail`. If it reads `success`, the gate is NOT proven red in CI and that is a Phase 52 blocker.
2. Optional: open the draft PR from the compare URL above if a `pull_request`-event run is wanted too (it costs another full pipeline); `gh pr close <n> --delete-branch` afterwards.
3. After step 1 is recorded: `git push origin --delete scratch/52-semver-red-proof` (the local branch is already deleted).

Phase 58's pre-tag audit must see H-03 closed.

## Deviations from Plan

### Auto-fixed Issues

None.

### Recorded departures and observations

1. **Remote scratch branch not deleted at close-out.** Plan Task 3 step 5 deletes it under `push-ok`, but the plan's own step 3 requires the red job to be observed first and that run had not concluded; deleting the branch would have cancelled the only red evidence. This follows the run instructions ("do NOT delete the remote scratch branch before its CI run has been observed red") and is H-03 step 3.
2. **Draft PR not opened.** `gh pr create` refused (GraphQL not available from this session); the plan anticipates this and the push-event run is its named fallback.
3. **Observation, not a defect.** At execution time `scheduler.rs:90` in `paladin-storage` also calls `cron_field_count` (the planning-time grep missed it). It is in the same crate, so `pub(crate)` keeps it compiling; recorded in Experiment 1.
4. **Observation, concurrency.** `ci.yml` uses `cancel-in-progress` per branch. The final push of this plan's metadata commit to the phase branch cancels the still-running `ci.yml` run for `0c2b4a2bd` (its semver job had already concluded `success` and was captured). It does not affect the scratch branch's separate group.

## Authentication gates

None at the git layer: `git push` worked with the sandbox credentials. `gh` GraphQL (`gh pr create`) is refused in this environment; handled as the plan's fallback, not an auth failure.

## Known Stubs

None. This plan created one evidence document and no source.

## Threat Flags

None. The only new surface is the throwaway scratch branch on origin, which is the plan's own threat T-52-28; mitigations held (ancestry check exits 1, subject grep empty, local branch deleted, remote deletion carried as H-03).

## Self-Check: PASSED

Files found: `52-SEMVER-RED-EVIDENCE.md`, `52-07-SUMMARY.md`, and the mutation script in the scratchpad (outside the repo). Commits found: `0c2b4a2bd`, `161c11a29`, and the scratch commit `6f3d8d784` (reachable via the remote-tracking ref, remote ref confirmed by `git ls-remote`). Task 1 and Task 3 automated verify commands both passed. The red-job conclusion is explicitly NOT claimed (H-03).
