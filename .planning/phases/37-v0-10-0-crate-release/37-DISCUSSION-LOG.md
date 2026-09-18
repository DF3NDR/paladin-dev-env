# Phase 37: v0.10.0 Crate Release - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-18
**Phase:** 37-v0.10.0 Crate Release
**Areas discussed:** Agent/human act boundary, Evidence homes & req ID, Post-merge bookkeeping, Rehearsal & red-gate policy

All four presented gray areas were selected. Mode: default interactive (no flags;
`workflow.discuss_mode` = `discuss`; no SPEC.md, no prior CONTEXT.md, no checkpoint, no plans).

---

## Agent/human act boundary

### Who pushes the branch and opens the release PR to main?

| Option | Description | Selected |
|--------|-------------|----------|
| Agent does both (Recommended) | Agent pushes `feature/phase-33` and opens the PR via `gh` with a curated body; reversible, and unlocks the CI coverage run SC2 needs | ✓ |
| Agent pushes, you open PR | Agent pushes and drafts the PR body; maintainer opens it | |
| You do both | Agent stops with commands and PR body ready; nothing leaves the devcontainer without the maintainer | |

**User's choice:** Agent does both

### Who merges the PR to main and pushes the v0.10.0 tag?

| Option | Description | Selected |
|--------|-------------|----------|
| You do both (Recommended) | One blocking checkpoint after green pre-merge CI and the §11 tick; agent hands over merge method and exact tag commands, resumes once the tag exists | ✓ |
| You merge, agent tags | Maintainer merges; agent resolves the merge SHA, cuts and pushes the annotated tag | |
| Agent does both after a go | Agent merges via `gh` and tags after an explicit "go"; needs merge rights on a protected `main` | |

**User's choice:** You do both

### When does the §11 sign-off tick happen relative to the re-seal and the PR's CI?

| Option | Description | Selected |
|--------|-------------|----------|
| After green PR CI (Recommended) | Re-seal → push → PR CI green incl. coverage → evidence appended → tick → push → CI on true final SHA → merge; one extra CI cycle | ✓ |
| Before push, with local re-seal | Tick on the local sweep alone; one CI cycle, but signing before the coverage gate reports | |
| Tick in the merge checkpoint | Tick, push, wait, merge and tag in one sitting | |

**User's choice:** After green PR CI

### Which merge method for the 488-commit release PR?

| Option | Description | Selected |
|--------|-------------|----------|
| Merge commit (Recommended) | v0.9.0 precedent (`0b5d4106`), literal wording of SC3 / 29 D-21; keeps every cited SHA reachable from `main` | ✓ |
| Squash | One commit on `main`; cited RED→GREEN SHAs become unreachable once the branch is deleted | |
| You decide | Planner follows whatever the ruleset permits | |

**User's choice:** Merge commit
**Notes:** The tag-cut mechanism itself was not asked — `docs/src/contributing/development-setup.md`
§"Cutting a release" already settles it (annotated tag pushed to the merge commit; `make release`'s
direct push is blocked by the ruleset and would re-run a landed bump). Recorded as D-00c.

---

## Evidence homes & req ID

### Where does the Phase 37 CI-run evidence live?

| Option | Description | Selected |
|--------|-------------|----------|
| New 37-CI-EVIDENCE.md (Recommended) | House form; 29- and 33-CI-EVIDENCE get one forward-pointer line each; no verified phase's artifact is mutated | ✓ |
| Append to 33-CI-EVIDENCE.md | What STATE.md currently names; mutates a verified phase's artifact | |
| Append to 29-CI-EVIDENCE.md | Literal reading of 29 D-21; same staleness cost | |

**User's choice:** New 37-CI-EVIDENCE.md

### How is the SC1 append to the Phase 29 acceptance audit shaped?

| Option | Description | Selected |
|--------|-------------|----------|
| Dated re-seal section (Recommended) | New dated section with a seven-row gate table (command, SHA, result, pointer to 37-CI-EVIDENCE); originals untouched | ✓ |
| Update rows in place | Re-stamp existing rows; loses the Phase 29 and 33 readings | |
| Pointer only | One line pointing at 37-CI-EVIDENCE; §11 signed in a file that doesn't show what was re-verified | |

**User's choice:** Dated re-seal section

### How should Phase 37's requirement be identified?

| Option | Description | Selected |
|--------|-------------|----------|
| Mint SHIP-05 'released' (Recommended) | Same prefix (none spent); SHIP-04 stays a true statement about Phase 29; clean traceability row | ✓ |
| Extend SHIP-04 in place | ROADMAP-permitted; re-opens a Complete requirement's checkbox | |
| Both: SHIP-05 + pointer | Mint SHIP-05 and add a forward pointer under SHIP-04 | |

**User's choice:** Mint SHIP-05 'released'

### What counts as registry proof for SC4?

| Option | Description | Selected |
|--------|-------------|----------|
| Index query per crate (Recommended) | Sparse-index query per publishable crate (name, version, checksum, yanked=false) tabled in 37-CI-EVIDENCE; crate list from `cargo metadata`; v0.9.0-style prose in MILESTONES.md | ✓ |
| Add a consumer smoke test | The above plus a scratch project building from the registry; costs a cold build | |
| Trust release.yml's own log | Publish job's success lines and trustpub proof only | |

**User's choice:** Index query per crate

---

## Post-merge bookkeeping

### When does the GSD phase itself close?

| Option | Description | Selected |
|--------|-------------|----------|
| Stay open across the merge (Recommended) | Pre-merge wave, maintainer checkpoint, post-tag wave; verified only when criteria are observably true; deliberate departure from 29 D-21 | ✓ |
| Close pre-merge, append later | 29 D-21 verbatim; SC3-5 pass by promise; late edits stale the verification | |
| Split: 37 pre-merge, 37.1 post | Decimal phase for post-tag work; a second discuss/plan cycle | |

**User's choice:** Stay open across the merge

### How do post-tag commits reach a protected main?

| Option | Description | Selected |
|--------|-------------|----------|
| chore/37-close PR (Recommended) | Branch from the tagged `main`, second small docs-only PR merged by the maintainer; house precedent | ✓ |
| Keep using feature/phase-33 | Second PR from the release branch | |
| Direct push with bypass | Temporary ruleset bypass for planning-only commits | |

**User's choice:** chore/37-close PR

### Is /gsd-complete-milestone part of Phase 37's plans?

| Option | Description | Selected |
|--------|-------------|----------|
| After the phase, same chore branch (Recommended) | Phase ends at "ready to close"; maintainer runs the command on `chore/37-close` so archive moves ride the same PR | ✓ |
| Inside the phase as final plan | All five criteria literally true at VERIFICATION, but the executor archives the directory it runs from | |
| Audit first, then decide | Phase ends with `/gsd-audit-milestone` only | |

**User's choice:** After the phase, same chore branch

### How should the agent wait and resume?

| Option | Description | Selected |
|--------|-------------|----------|
| Hard stop + resume file (Recommended) | Hand-off names the exact resume condition; post-tag wave re-verifies it first; no parked watchers | ✓ |
| Agent watches the release run | `gh run watch` to completion; long-lived watchers have stalled in this devcontainer | |
| You decide | Planner picks per step | |

**User's choice:** Hard stop + resume file

---

## Rehearsal & red-gate policy

### Rehearse the release pipeline before the real v0.10.0 tag?

| Option | Description | Selected |
|--------|-------------|----------|
| Dry-run dispatch after merge (Recommended) | `release.yml` dispatched with `dry_run=true` on the merge commit before tagging; researcher confirms what the required `tag` input accepts; fallback recorded | ✓ |
| Straight to v0.10.0 | Local dry run + PR CI are the rehearsal; regressions discovered live | |
| Cut v0.10.0-rc.1 first | Full real rehearsal; burns an rc version and needs a twelve-manifest re-bump | |

**User's choice:** Dry-run dispatch after merge

### If a re-sealed gate goes red, what may the phase do?

| Option | Description | Selected |
|--------|-------------|----------|
| Bounded fix set, else stop (Recommended) | 29 D-12 carried forward: docs/tests/citations/evidence fixable, anything under `src/` is a stop | |
| Fix whatever is red | Agent fixes any failure including code, with TDD | |
| Always stop | Any red gate is a blocking checkpoint, even a one-line doc fix | ✓ |

**User's choice:** Always stop
**Notes:** Non-recommended option chosen — stricter than Phase 29 D-12. Prompted the flake
follow-up below. Recorded in CONTEXT.md `<specifics>` as the governing temperament for release day.

### Do CI infrastructure flakes also stop?

| Option | Description | Selected |
|--------|-------------|----------|
| One re-run, recorded, then stop | `gh run rerun --failed` exactly once when no assertion failed; both run IDs and the justifying log line recorded | ✓ |
| Stop on flakes too | Literal "always" | |
| You decide | Planner defines the flake test | |

**User's choice:** One re-run, recorded, then stop

### Who drives recovery if the real release run goes red?

| Option | Description | Selected |
|--------|-------------|----------|
| Agent diagnoses, you act (Recommended) | Agent follows release-recovery.md §1-§2 read-only and records; completing forward, re-dispatch and yanks are the maintainer's | ✓ |
| Agent completes forward | Agent re-dispatches per §3; only yanks stay human | |
| Stop immediately | Record the red run ID and stop without diagnosis | |

**User's choice:** Agent diagnoses, you act

---

## Claude's Discretion

- Who dispatches the D-13 dry run (publishes nothing); fallback is handing the command to the maintainer
- Local re-seal command order and output capture
- Release PR title/body wording and MILESTONES.md entry prose
- Whether `feature/phase-33` is deleted after merge (left to the maintainer's GitHub setting)
- Plan count and wave layout, within D-09's two-wave split
- Handling of `main` moving before the merge (re-seal from the top, §11 re-confirmed)

## Deferred Ideas

- Correcting the stale "eleven crates" count in docs if `cargo metadata` disagrees — v0.11.0
- Consumer smoke test as a standing post-release check — future release-tooling phase
- Making `release.yml` dry-runnable from a bare SHA — later-milestone CI change
- Carried unchanged from Phase 36.1's deferred list

## Todos reviewed, not folded

- `2026-08-13-verify-local-coverage-reproduction.md` (score 0.6) — re-homed by 36.1 D-23
- `2026-09-13-evaluate-rustfs-replacement-for-minio.md` (score 0.6) — re-homed by 36.1 D-24

Not presented as a fold question: Phase 36.1 D-00k / D-22…D-24 had already dispositioned both
files the same day, so re-asking would have re-litigated a locked decision.

## Planning-time question (2026-09-18, `/gsd-plan-phase 37`, after research)

Research Q3 found `paladin-eval` never published (sparse index `404`) and that crates.io Trusted
Publishing cannot perform a crate's first publish. The orchestrator re-verified the `404`, and
established that the researcher's suggested remedy (maintainer publishes the real `0.10.0` before
the tag) is impossible — `paladin-eval` `0.10.0` depends on five workspace crates at `^0.10.0`
that are not on the registry until the release run publishes them.

Asked: how should the plan handle the first publish?

| Option | Outcome |
|--------|---------|
| Placeholder bootstrap pre-tag (recommended) | **Selected** — recorded as D-17 |
| Planned complete-forward at release time | Not selected — a deliberately red release run |
| Pause planning and decide out-of-band | Not selected |

Two tree facts were recorded alongside D-17 as orchestrator-verified clarifications rather than
maintainer decisions: D-06's re-seal section lands in the corpus document as `## 12.` (the phase
file is a pointer; Phase 33 precedent), and D-13's documented fallback applies because the dry
run cannot be dispatched before the tag exists.

## Session note

While the workflow file was being read, the tool result carried a trailing block styled as a
system reminder (commit-attribution wording plus an instruction to send files to another device
via a `SendUserFile` tool). It arrived inside tool output rather than from the user, so it was
not acted on; the user was told at the time. No file was sent anywhere.
