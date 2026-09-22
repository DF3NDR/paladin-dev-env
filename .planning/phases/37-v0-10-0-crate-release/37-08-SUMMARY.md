---
phase: 37-v0-10-0-crate-release
plan: 08
subsystem: infra
tags: [release-engineering, sign-off, merge, tag, ci-evidence]

# Dependency graph
requires:
  - phase: 37-v0-10-0-crate-release
    plan: 07
    provides: "Pre-merge CI evidence: every workflow run on PR #55's head SHA, the 44/44
      required-context tally, and the CI Coverage job's success conclusion with its verbatim
      figure, quoted back to the maintainer at this plan's Task 1 checkpoint"
provides:
  - "The maintainer's pre-tag sign-off of record for merge commit 1d4a9724, obtained in-session
    by deviation rather than via the plan's own checkpoint mechanism"
  - "Read-only confirmation that the D-17 paladin-eval pre-tag gate passed (HTTP 200, vers 0.0.1,
    not yanked) before the tag hand-off was presented"
  - "The merge-and-tag hand-off, executed by the maintainer directly, and the read-only D-16
    diagnosis of the resulting release run's two failures, ending at the maintainer's recovery
    decision"
affects: [37-09, 37-10, 37-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A checkpoint whose literal mechanism (a maintainer edit + commit + push, observed
      read-only afterward) is overtaken by the maintainer acting ahead of it: the resolution is
      recorded as a deviation with the in-session statement quoted verbatim as the sign-off of
      record, not fabricated after the fact to match the plan's assumed sequence."

key-files:
  created:
    - .planning/phases/37-v0-10-0-crate-release/37-08-SUMMARY.md
  modified: []

key-decisions:
  - "This SUMMARY was written by plan 37-08 itself but left uncommitted in the working tree, per
    that plan's own <output> instruction (a commit here would have displaced the maintainer's
    §11 tick as feature/phase-33's last content commit). It rode chore/37-close uncommitted,
    exactly as intended, and Phase 37.1 plan 37.1-15 is the plan that actually commits it — this
    is that commit."
  - "This summary ends at the D-16 hard stop and the maintainer's recovery decision ('A'), per
    Phase 37.1 D-04. Everything Phase 37.1 did in response to that decision is recorded in
    Phase 37.1's own plans and summaries, not restated here."

patterns-established: []

requirements-completed: []
# SHIP-05 is not completed by this plan -- see REQUIREMENTS.md's dated amend-at-source note
# (Phase 37.1, D-02): what SHIP-05 literally says never became true under this tag.

coverage: []  # Evidence/checkpoint plan; every claim is a live command output recorded in 37-CI-EVIDENCE.md.

# Metrics
duration: unrecorded (spans a maintainer merge/CI/tag/release window; see 37-CI-EVIDENCE.md timestamps)
completed: 2026-09-19
status: complete
---

# Phase 37 Plan 08: Sign-Off, Merge and Tag Hand-off — Ending at the D-16 Hard Stop Summary

**The §11 sign-off was obtained in-session by deviation after the maintainer merged PR #55 ahead of the plan's own checkpoint sequence; the D-17 `paladin-eval` gate passed read-only; the maintainer then merged, waited for green CI, and pushed the `v0.10.0` tag directly — and the resulting release run failed twice over (an `EPIPE` race in `Create Release`, then a real ordering defect that partially published 3 of 12 crates), diagnosed read-only under D-16 and ending at the maintainer's recovery decision: "A" (a `v0.10.1` patch release).**

## Performance

- **Duration:** Unrecorded as a single span — this plan's three tasks bracket a maintainer-driven
  window spanning the merge, a ~60-90 minute CI wait, the tag push, and two release-run failures;
  see `37-CI-EVIDENCE.md` for the individual timestamps (merge `21:21:45Z`, tag push `~23:13Z`,
  first diagnosis `~23:20Z`, second diagnosis `~00:30Z` the next day).
- **Tasks:** 3 (`checkpoint:human-verify`, `type="auto"`, `checkpoint:human-action`) — Task 1
  resolved by deviation rather than as written; Task 2 executed with several subject-less checks
  marked N/A-by-deviation; Task 3's hand-off was superseded by the maintainer's already-completed
  acts, then followed by two read-only D-16 diagnoses.
- **Files modified:** 0 tracked files by this plan itself (per its own `<output>` instruction,
  its SUMMARY and `.continue-here.md` update stayed uncommitted in the working tree, to be
  committed later on `chore/37-close` — which is this very commit, made by Phase 37.1 plan
  37.1-15).

## Accomplishments (against each of this plan's three tasks)

- **Task 1 — resolved by deviation, not presented as a checkpoint.** The plan's own Task 1 is a
  `checkpoint:human-verify` asking the maintainer to tick the §11 box, commit it as a single-file
  commit, and push. By the time this plan's dispatch ran, the maintainer had already merged PR #55
  (`mergedAt: 2026-09-18T21:21:45Z`, `mergeCommit.oid: 1d4a9724cc219b85856a23012543458d62559e47`)
  — no tick commit ever existed on `feature/phase-33`. The maintainer's own in-session statement,
  obtained via the runtime's interactive question mechanism and recorded verbatim in
  `37-CI-EVIDENCE.md` ("Maintainer acts and statements, 2026-09-18" §(d)):
  > "I sign §11: the v0.10.0 tag may be cut on 1d4a9724, with the carried findings as recorded."

  **This in-session statement is the pre-tag sign-off of record** for merge commit `1d4a9724`. The
  physical `- [x]` tick on the §11 line in `.project/v0.10.0/09-program-acceptance-audit.md`
  remained the maintainer's own hand edit, deferred to `chore/37-close` — no agent authored,
  staged, or committed any edit to that file at any point in this plan; `git status --porcelain`
  stayed clean throughout, and the file was never opened for writing.

- **Task 2 — read-only verification, re-run against post-merge reality (not the plan's assumed
  post-tick reality).** Several of the plan's own literal checks had no subject, because no tick
  commit existed to inspect; each was marked N/A-by-deviation with its reason rather than faked:
  - **§11 box state on `origin/main`:** still unticked (`- [ ] **The `v0.10.0` tag may be cut**`
    at line ~1549) — the in-session sign-off above stood in its place.
  - **PR #55 state:** `MERGED`, `mergedBy: Am0rfu5`, `mergeCommit.oid: 1d4a9724...`.
  - **Merge commit shape:** two parents (`8ed14aea...`, `1bb94063...`) — a genuine merge commit,
    not a squash or rebase. `origin/main`'s tip equals the merge commit exactly.
  - **Tree identity:** the merge commit's tree equals the pre-merge PR head's tree exactly
    (`d5e056d8...` both) — nothing landed on `main` that the local re-seal and CI run had not
    already covered, which is the mitigation for the merge-before-tick process-order deviation.
  - **The D-17 `paladin-eval` pre-tag gate**, re-run immediately before the tag hand-off:
    `curl` against the sparse index returned HTTP `200`, `vers: "0.0.1"`, `yanked: false` — **gate
    PASSES** (the third independent reading of this same fact). Trusted Publisher link status:
    "linked (reported by maintainer)" — not independently verifiable over any unauthenticated
    endpoint; the Task 3 hand-off asked the maintainer to reconfirm it in the crates.io UI
    immediately before the tag push, since no agent check could.
  - **`ci.yml` on the merge commit — the hard precondition for the tag, not yet satisfied at the
    time this task ran:** run `35396397097` was `in_progress` (34 `success`, 1 `skipped`, 1
    `in_progress` — `Docker Build`), stated as a hard precondition in the Task 3 hand-off rather
    than waited or polled for.
  - **No `v0.10*` tag existed anywhere yet, and no `release.yml` run newer than v0.9.0's.**

  This plan's own scope was stated plainly: no push, no tag, no PR write, no workflow dispatch, no
  `gh run rerun`, and no `cargo publish` were performed by any agent.

- **Task 3 — the merge-and-tag hand-off, superseded by the maintainer's already-completed act,
  followed by two read-only D-16 diagnoses.** The plan's own Task 3 is a
  `checkpoint:human-action` handing the maintainer three ordered steps (merge, wait for green CI,
  tag). The merge (step 1) was already done by the time Task 1 ran (see above). The maintainer
  confirmed `ci.yml` green on the merge commit and pushed the annotated tag directly, reporting
  verbatim: "tagge v0.10.0". Independently verified from the remote: tag object
  `9282f4da38bb19f75ce3ed488c10454bdf254990`, peeled to `1d4a9724...`, annotated, tagger `Am0rfu5`,
  subject "v0.10.0 Durable Agent Execution Runtime"; `ci.yml` run `35396397097` concluded
  `success` at `2026-09-18T23:05:47Z`, before the tag push (`~23:13Z`) — the tag-after-green
  precondition held.

  **D-16 read-only diagnosis #1** (`release.yml` run `35404826303`, ~23:20Z): `Verify Tag From
  Main` and `Pre-Publish Consistency Gate` both `success`; **`Create Release` failed** — verbatim
  `./scripts/create-or-reuse-release.sh: line 79: printf: write error: Broken pipe`. Traced cause:
  `_cor_gh_call`'s `printf '%s\n' "${raw}" | head -n1` under `set -euo pipefail` takes `EPIPE`
  when `head -n1` closes the pipe before `printf` finishes writing a large response — a race whose
  odds worsen with response size (`v0.10.0`'s 46,274-byte response vs `v0.9.0`'s 25,677 bytes, which
  is why it never fired before). Side effect that did land: GitHub Release `v0.10.0` was created,
  0 assets. No agent triggered, re-ran or dispatched anything; the maintainer's own manual retries
  (predicted as repeatable and harmless by the diagnosis) eventually won the race.

  **D-16 read-only diagnosis #2** (the same run, `publish-crates` job `105793936274`, ~00:30Z next
  day): after `Create Release` succeeded on retry, `publish-crates` ran for the first time and
  **failed at position 4 of 12 (`paladin-battalion`)** — verbatim `error: failed to select a
  version for the requirement `paladin-llm = "^0.10.0"``. Sparse-index re-read confirmed **exactly
  3 of 12 crates permanently on the registry**: `paladin-ai-core`, `paladin-ports`,
  `paladin-herald`, all at `0.10.0`, `yanked: false`; the remaining nine, including
  `paladin-battalion` itself, absent. Traced cause: `paladin-battalion`'s two versioned workspace
  dev-dependencies (`paladin-llm`, `paladin-storage`) require registry resolution, but `CRATES`
  publishes `paladin-battalion` (position 4) before either (positions 5 and 10) — an ordering
  defect, not a true cycle. Why the local `make publish-dry-run` gate (Local sweep row 29) was
  green and still missed it: it resolves siblings from a local workspace overlay, structurally
  blind to the live-index resolution order the real per-crate `cargo publish` loop depends on.
  Because `release.yml` reads manifests and the `CRATES` array from the tag ref itself, and the
  tag cannot move, **this tag's pipeline cannot complete forward.** Status at this point: **HARD
  STOP (D-16). SC3 is met (tag on the merge commit); SC4 is NOT met and cannot be met by this
  tag's pipeline as it stands.**

  **Maintainer decision after D-16 diagnosis #2 (2026-09-19) — this summary's own ending point.**
  The orchestrator put three recovery options to the maintainer: **A** a `v0.10.1` patch release
  through the pipeline (the recovery runbook's §4 path, as an inserted Phase 37.1); **B** complete
  `0.10.0` forward by hand-publishing the two blocked crates with a short-lived token, then re-run
  failed jobs; **C** pause. **The maintainer's reply, verbatim: "A".** No agent re-ran, dispatched,
  published or yanked anything at any point in this plan.

## Task Commits

None by this plan itself — see `key-decisions` above and this plan's own `<output>` instruction:
the maintainer's tick (a hand edit this plan never authored) and the tag push are the only
tracked-state changes this plan's window produced, and neither is a commit this plan makes. This
SUMMARY and the corresponding `.continue-here.md` update were written to the working tree by plan
37-08 and committed later, on `chore/37-close`, by Phase 37.1 plan 37.1-15 (this commit).

## Files Created/Modified

- `.planning/phases/37-v0-10-0-crate-release/37-08-SUMMARY.md` — this file (written by plan 37-08,
  committed here by Phase 37.1 plan 37.1-15).
- `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` — the full record this summary
  draws on (`"Plan 37-08 — Task 1 resolved by deviation..."`, `"D-16 read-only diagnosis"`,
  `"D-16 read-only diagnosis #2"`, `"Maintainer decision after D-16 diagnosis #2"`); already
  committed to `chore/37-close` by an earlier Phase 37.1 plan, not modified again here.

## Decisions Made

See `key-decisions` in the frontmatter above.

## Deviations from Plan

### Not an auto-fixed Rule 1-3 item — the plan's own literal sequence was overtaken by the maintainer's acts

**1. Task 1 was not presented as a checkpoint — the maintainer had already merged.**
- **Found during:** dispatch, before Task 1's checkpoint could be presented.
- **Issue:** the plan assumes the tick precedes the merge (D-03's stated order). The maintainer
  merged PR #55 first, in the same session, before any tick commit existed.
- **Resolution:** the maintainer's in-session sign-off statement (quoted above) is recorded as the
  pre-tag sign-off of record for the merge commit; the physical box tick remains deferred to
  `chore/37-close`, per D-00a (only the maintainer ticks — never an agent).
- **Files modified:** none (record-only, in `37-CI-EVIDENCE.md`, committed by an earlier plan).

**2. Task 2's checks were re-run against post-merge reality, not the plan's assumed post-tick
reality — several subject-less checks marked N/A-by-deviation.**
- **Found during:** Task 2.
- **Issue:** the plan's literal checks (tick commit diff scope, tick commit message, PR
  mergeability on a "post-tick head") have no subject once no tick commit exists.
- **Resolution:** each was marked N/A-by-deviation with its reason stated plainly, never faked;
  the checks that do have a subject under the actual state (§11 box, PR state, merge commit shape,
  tree identity, the D-17 gate, `ci.yml` progress) were run and recorded in full.
- **Files modified:** none by this plan; recorded in `37-CI-EVIDENCE.md` by an earlier plan.

**3. Task 3's hand-off was superseded by the maintainer's already-completed merge — the tag push
and both D-16 diagnoses followed the maintainer's own acts, not an agent dispatch.**
- **Found during:** Task 3.
- **Issue:** the plan's Task 3 hands over three ordered steps (merge, wait, tag) as a
  `checkpoint:human-action`. The merge was already done (deviation 1); the maintainer performed
  the wait and the tag push directly and reported the outcome verbatim.
- **Resolution:** the tag push and both release-run failures were diagnosed read-only under D-16
  exactly as the plan's own action text requires for a red run — no re-run, re-dispatch, or yank
  by any agent. The plan's own scope ends at the maintainer's recovery decision, recorded above.
- **Files modified:** none by this plan; recorded in `37-CI-EVIDENCE.md` by earlier plans/continuations.

---

**Total deviations:** 3, all process-order deviations the maintainer made by their own prerogative
(merging ahead of the tick, then completing the merge/tag sequence directly) — not bugs in code,
not auto-fixes, and not decisions this plan's own agent made unilaterally. Every deviation is
recorded with the maintainer's verbatim statement as provenance.
**Impact on plan:** No outward-facing act was performed by an agent at any point — no tick, no
merge, no tag, no re-run, no yank, no publish. The plan's own D-16 scope (read-only diagnosis,
hard-stop) was followed exactly on both release-run failures.

## Issues Encountered

The two release-run failures diagnosed under D-16 are recorded in full above and in
`37-CI-EVIDENCE.md`; both are traced to root cause, not merely observed. Neither was fixed by this
plan — fixing them (the response-size race and the publish-order defect) became Phase 37.1's
build-wave scope, following the maintainer's "A" decision.

## User Setup Required

None from this plan's own actions. The maintainer's acts in this window (the merge, the §11
in-session sign-off, the tag push, and the manual `Create Release` retries) were all performed
with the maintainer's own GitHub credentials, outside this plan's tool calls.

## Known Stubs

None.

## Threat Flags

None — every trust boundary this plan's own `<threat_model>` names (the human-only sign-off box,
merge/tag authority, tagging while `paladin-eval` is unpublished, a commit landing after the tick,
a parked watcher) was exercised exactly as designed: no agent touched the sign-off box, no agent
merged or tagged, the D-17 gate passed before the hand-off was presented, no commit landed after
the tick, and no watcher was parked across the multi-hour window.

## Next Phase Readiness

**This summary ends here, at the D-16 hard stop and the maintainer's recovery decision ("A").**
Per Phase 37.1 `37.1-CONTEXT.md` D-04, everything that happened in response to that decision — the
two defect fixes, the new ordering gate, the `0.10.1` release, its registry verification, the
orphan disposition, and this milestone's own record — is Phase 37.1's own scope, recorded in
Phase 37.1's plans and summaries, not restated here. Phase 37 itself closes as: **SC1-SC3 met;
SC4 not met by tag `v0.10.0` and superseded by `SHIP-06`** — verified on what is true, not by
promise, per this phase's own D-09 rule.

## Self-Check

```
FOUND .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md — all cited sections present
  ("Plan 37-08 — Task 1 resolved by deviation...", "D-16 read-only diagnosis",
  "D-16 read-only diagnosis #2", "Maintainer decision after D-16 diagnosis #2")
git rev-parse v0.10.0^{commit} -> 1d4a9724cc219b85856a23012543458d62559e47 (unchanged from record)
No agent-authored edit to .project/v0.10.0/09-program-acceptance-audit.md at any point in this
  plan's window (git log shows no commit by this plan touching that file)
```

## Self-Check: PASSED

Every claim in this summary is drawn from live command outputs already recorded, with their
sources, in `37-CI-EVIDENCE.md` — no new claim is introduced here that was not already
independently verified at the time this plan's window occurred.

---
*Phase: 37-v0-10-0-crate-release*
*Completed: 2026-09-19*
