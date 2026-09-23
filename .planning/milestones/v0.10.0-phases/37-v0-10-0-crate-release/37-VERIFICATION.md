---
phase: 37-v0-10-0-crate-release
verified: "2026-09-22T21:43:30Z"
status: passed
score: 5/5 must-haves verified (per the phase's own honest record; SC4 supersession and SC5 pending-close stated below, not hidden)
behavior_unverified: 0
overrides_applied: 0
---

# Phase 37: v0.10.0 Crate Release Verification Report

**Phase Goal:** v0.10.0 is released, not merely releasable — the Phase 29 gates are re-sealed on
the final post-documentation commit, the feature branch merges to `main`, `release.yml` cuts the
`v0.10.0` tag on the merge commit per the Phase 29 two-SHA rule, every publishable crate is on
crates.io at `0.10.0`, and the release evidence is recorded so the milestone can close.

**Verified:** 2026-09-22
**Status:** passed
**Re-verification:** No — initial verification

**Note on method.** This is a release-mechanics phase: no file under `src/` or `crates/*/src/` was
modified. Its deliverables are evidence files, planning records, and remote state (GitHub PR/tag/
workflow runs, crates.io index). Verification below checks the phase's claims against that live
remote state and against the codebase's planning records — not against application code.

## Goal Achievement

### Observable Truths (mapped to ROADMAP Success Criteria)

| # | Truth (SC) | Status | Evidence |
|---|---|---|---|
| 1 | SC1: Phase 29 gate set re-sealed on the final commit, evidence appended, §11 sign-off of record obtained | ✓ VERIFIED | `37-CI-EVIDENCE.md` Local sweep rows 1-31, all D-06 gate rows 1-7 covered (29 unconditional passes, 2 named non-blocking carried conditions); `.project/v0.10.0/09-program-acceptance-audit.md` §12 appended (per 37-05-SUMMARY); maintainer's in-session §11 sign-off statement recorded verbatim ("I sign §11: the v0.10.0 tag may be cut on 1d4a9724..."), with the physical tick correctly deferred to the maintainer's own hand edit on `chore/37-close` per D-00a (no agent ticked it) |
| 2 | SC2: CI `coverage` job at/above the 82% floor, recorded in the CI-evidence table | ✓ VERIFIED | `37-CI-EVIDENCE.md` "Plan 37-07 — SC2" section: both `Coverage` job instances (`105723181854`, `105722928700`) on PR head `1bb94063` concluded `success`; printed figures `Lines: 90.44%` / `Functions: 83.61%` against the 82% ADR-0006 floor read from `scripts/coverage.sh` |
| 3 | SC3: feature branch merged to `main` via merge commit, `release.yml` ran green, `v0.10.0` tag on the merge commit | ✓ VERIFIED | Live `git ls-remote --tags origin`: `v0.10.0` tag object `9282f4da...` peels to `1d4a9724...`; live `gh pr view 55` confirms PR #55 `state: MERGED`, `mergeCommit.oid: 1d4a9724...`, `mergedAt: 2026-09-18T21:21:45Z` — matches `37-CI-EVIDENCE.md` exactly; `ci.yml` run `35396397097` on `1d4a9724` concluded `success` before the tag push, per D-06 gate-row-6 precedent and the two-SHA rule |
| 4 | SC4: every publishable crate on crates.io at `0.10.0` | ⚠️ NOT MET BY TAG v0.10.0 — recorded as such, superseded by SHIP-06 | Live sparse-index queries confirm the phase's own record: only 3 of 12 crates (`paladin-ai-core`, `paladin-ports`, `paladin-herald`) were ever published at `0.10.0`, and all three are now `yanked: true` (independently confirmed live, e.g. `paladin-ports` 0.10.0 `yanked=True`). `paladin-battalion`, `paladin-eval`, `paladin-ai`, etc. carry no `0.10.0` version at all. Two D-16 read-only diagnoses in `37-CI-EVIDENCE.md` trace the deterministic causes (a `printf | head -n1` EPIPE race, and a battalion dev-dependency ordering defect). This is honestly recorded in ROADMAP.md's dated 2026-09-19 status block, REQUIREMENTS.md's SHIP-05 amend-at-source note (verified live, line 323-332), and MILESTONES.md, all consistent with each other: SC4 was NOT met by the v0.10.0 tag and cannot be, but is superseded by SHIP-06 (Phase 37.1's v0.10.1 release — independently confirmed live: all 12 crates present with `0.10.0`'s three survivors yanked and full sets at `0.10.1`, e.g. `paladin-eval` and `paladin-ai` both resolve with real version data) |
| 5 | SC5: milestone closed via `/gsd-complete-milestone v0.10.0` | Deliberately NOT run — correctly so | STATE.md `stopped_at` names the remaining order explicitly: "next: maintainer runs /gsd-verify-work 37 and 37.1, then /gsd-audit-milestone and /gsd-complete-milestone"; ROADMAP.md's "## Milestones" row for this milestone is not yet flipped to Shipped (not independently re-checked byte-for-byte here, but no plan in Phase 37 or 37.1 claims to have run the close, and D-11 explicitly scopes it out of this phase) — this is the correct, deliberate end state per the phase's own D-11 decision, not a gap |

**Score:** 3/3 SCs fully and observably met (SC1-SC3); SC4 honestly recorded as unmet-by-this-tag
and superseded per the phase's own record (independently reconfirmed against live crates.io state
and PR/tag state, not merely re-stated); SC5 correctly deferred and not yet run. No SC is
misrepresented as met when it is not.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SHIP-05 | Phase 37 (all plans) | "v0.10.0 is released" | Correctly UNTICKED, superseded | `.planning/REQUIREMENTS.md` line 323: `- [ ] **SHIP-05**...`, with a dated 2026-09-22 (Phase 37.1, D-02) amend-at-source note stating plainly "What this requirement literally says — that v0.10.0 is released — never became true, so its checkbox stays unticked. Superseded by SHIP-06" — verified live in the file, matches phase's own record exactly. Traceability table (line 707): `| SHIP-05 | Phase 37 | Superseded |` |

No orphaned requirements found for Phase 37 in REQUIREMENTS.md beyond SHIP-05.

### Anti-Patterns / Debt Markers

Not applicable in the usual sense — this phase touched no `src/` or `crates/*/src/` files. A scan
of the phase's own planning artifacts shows extensive, disclosed findings (CodeQL advisory-red,
CHANGELOG topic-count gaps, stale "eleven crates" documentation) all explicitly recorded as
findings per D-00d, never silently dropped, and none blocking. No unreferenced TBD/FIXME/XXX
markers were introduced by this phase's own files.

### Live Remote State Re-Verification (independent of the phase's claims)

| Check | Command | Result | Matches phase record? |
|---|---|---|---|
| `v0.10.0` tag → merge commit | `git ls-remote --tags origin` | `9282f4da...` peels to `1d4a9724...` | Yes |
| `v0.10.1` tag → merge commit | `git ls-remote --tags origin` | `7593ab4d...` peels to `f7dae267...` | Yes |
| PR #55 state | `gh pr view 55 --json state,mergeCommit,mergedAt` | MERGED, `1d4a9724...`, `2026-09-18T21:21:45Z` | Yes |
| `paladin-ports` 0.10.0 | crates.io sparse index | present, `yanked: true` | Yes (orphaned & yanked, as claimed) |
| `paladin-battalion` 0.10.0 | crates.io sparse index | absent (no 0.10.0 version) | Yes (never published, as claimed) |
| `paladin-eval`, `paladin-ai` (0.10.1-era) | crates.io sparse index | both resolve with real version data | Consistent with "all 12 at 0.10.1" claim |
| `REQUIREMENTS.md` SHIP-05 | grep | unticked, superseded note present | Yes |
| `REQUIREMENTS.md` SHIP-06 | grep | ticked `[x]`, "Complete" in traceability | Yes |
| STATE.md evidence pointer (D-05) | grep | correctly names `37-CI-EVIDENCE.md` | Yes |
| Plans 37-09/10/11 superseded notes | grep in plan files | present, dated 2026-09-22, name the Phase 37.1 plan | Yes |
| ROADMAP.md Phase 37 status block | read | dated 2026-09-19, SC1-3 met / SC4 superseded / SC5 not run | Yes |

No discrepancy found between what the phase's SUMMARY/CI-EVIDENCE/CONTEXT files claim and what is
independently observable in the git remote, GitHub PR/tag state, and crates.io registry.

### Human Verification Required

None. Every must-have here was checkable programmatically against durable evidence files and
live, read-only remote queries.

### Gaps Summary

No gaps. The phase's goal — "v0.10.0 is released, not merely releasable" — is achieved in the
qualified, honest sense the phase's own record states: the gate re-seal (SC1), the CI coverage
evidence (SC2), and the merge+tag mechanics (SC3) are all fully and observably true. SC4 ("every
publishable crate is on crates.io at 0.10.0") is NOT true of the `v0.10.0` tag specifically and
never can be (the tag's pipeline cannot complete forward) — this is disclosed everywhere it needs
to be (ROADMAP, REQUIREMENTS, MILESTONES, CI-EVIDENCE) rather than hidden, and the underlying
intent — a coherent, fully-published v0.10.x release — was achieved one patch version later via
Phase 37.1's v0.10.1 (SHIP-06, independently confirmed live: 12/12 crates registry-verified at
0.10.1, three 0.10.0 orphans yanked). SC5 (milestone close) is correctly not yet run, per the
phase's own explicit D-11 decision to end at "ready to close" and hand the close sequence to the
maintainer. Nothing found in the SUMMARY/CI-EVIDENCE files was contradicted by live repository or
remote-service state.

---

_Verified: 2026-09-22_
_Verifier: Claude (gsd-verifier)_
