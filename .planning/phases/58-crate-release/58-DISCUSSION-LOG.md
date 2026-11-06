# Phase 58: Crate Release - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-05
**Phase:** 58-crate-release
**Areas discussed:** Todo cross-reference, Branch and PR shape, Pre-tag audit dispositions,
Consumer statement and web3sec, Bump commit set and changelogs

---

## Todo cross-reference

Seven pending todos keyword-matched the phase. Multi-select.

| Option | Description | Selected |
|--------|-------------|----------|
| None as work; audit acknowledges all seven | No todo becomes a plan; the audit lists all seven as re-homed with owner and date | ✓ |
| Fold (6) Cadence muster pin | Pin web3sec's three Cadence import paths in the consumer muster before the tag | ✓ |
| Fold (1) gpt-4 literal sweep | Replace the deprecated model literal across docs/examples before the tag | ✓ |
| Fold (5) devcontainer cargo-llvm-cov | Install cargo-llvm-cov 0.8.7 in the devcontainer image | ✓ |

**User's choice:** all four options.
**Notes:** Read as: fold (6), (1) and (5) as build-wave work; the audit acknowledges the other
four (Responses API, setup-check output, treasurer price digits, DeepSeek reasoning hint) with
their existing owners and dates. The reading was stated back and not corrected.

---

## Branch and PR shape

### How does the release reach main?

| Option | Description | Selected |
|--------|-------------|----------|
| One PR: branch is the release PR | Bump set lands last on the branch; one merge commit is the tag target (Phase 47 precedent) | ✓ |
| Two PRs: merge phases first, then a bump PR | Operator Guide steps 1-7 off main; two CI cycles, main briefly at 0.11.0 with 0.11.1 code | |
| Two PRs, but the audit sits between them | Audit runs on merged main before the bump PR; slowest | |

**User's choice:** One PR.

### Freeze main until the release PR merges?

| Option | Description | Selected |
|--------|-------------|----------|
| Freeze main; branch merges main in if it moves | Merge commit never rebase; re-seal restarts; maintainer re-confirms | ✓ |
| No freeze; rebase the release PR on whatever main holds | Allows docs/chore merges meanwhile | |

**User's choice:** Freeze main.

### When is the release label applied and where is the sign-off box?

| Option | Description | Selected |
|--------|-------------|----------|
| Label once, after the bump push; box in 58-ACCEPTANCE-AUDIT.md | 63-minute rebuild on the final head; eight-row table and tick box in the 47 shape | ✓ |
| Label early, before the bump | Faster first E2E signal, second full re-run after the bump | |
| Skip the label for a patch | No End-to-End Tests before the merge | |

**User's choice:** Label once after the bump push.

### The red Generated SDK Clients smoke job?

| Option | Description | Selected |
|--------|-------------|----------|
| Diagnose first; fix only if CI-side and small, else record and ask | Read the log before the bump; src-touching fixes go to the maintainer | ✓ |
| Fix it whatever it takes before the bump | Phase 58 owns it to green including code | |
| Record only; maintainer decides before the merge | No diagnosis beyond required-context status | |

**User's choice:** Diagnose first.

**Continue check:** Next area.

---

## Pre-tag audit dispositions

### What rule lets the audit read passed?

| Option | Description | Selected |
|--------|-------------|----------|
| Every carried item gets one of three verdicts, recorded in one register | 58-AUDIT-DISPOSITIONS.md; tech_debt may cite only its rows | ✓ |
| Fix everything fixable; only owner-level items may be re-homed | Widest release PR, smallest tech_debt list | |
| Re-home everything to v0.12.0 Phase 1 with one shared date | No fixes; fastest path to the tag | |

**User's choice:** One register, three verdicts.

### Which of 57-REVIEW CR-01, WR-01, WR-02 land in v0.11.1?

| Option | Description | Selected |
|--------|-------------|----------|
| WR-02 only (script); CR-01 and WR-01 re-homed to v0.12.0 | Gate tooling fix keeps the no-src rule | ✓ |
| All three, as the first commits before the bump | Three more crate diffs and MIGRATION entries | |
| None; all three re-homed | Checker keeps its known weakness through this audit | |

**User's choice:** WR-02 only.

### The tracked sk-proj key literal?

| Option | Description | Selected |
|--------|-------------|----------|
| Scrub the literal in the release PR; rotation and history purge are the maintainer's | Docs-only commit, scanner clean; maintainer checkpoints recorded | ✓ |
| Leave the file; record as a maintainer item only | Literal ships in the source archive | |
| Delete the whole file | Loses the note's other content from the tree | |

**User's choice:** Scrub in the release PR.

### Owner, date and home for re-homed items?

| Option | Description | Selected |
|--------|-------------|----------|
| One pending todo per item, owner named, date = v0.12.0 first planning day | Register cites the todo path; WINDOWS stays at 0 | ✓ |
| WINDOWS.md rows with status waived and a reason | One register for everything; moves open_count unless closed in the same commit | |
| Only the audit register; no todo or WINDOWS entry | Lightest; nothing reminds the next milestone | |

**User's choice:** One pending todo per item.

**Continue check:** Next area.

---

## Consumer statement and web3sec

### Shape of the SHIP-09 no-adaptation statement?

| Option | Description | Selected |
|--------|-------------|----------|
| A 'Library consumers: v0.11.0 → v0.11.1' subsection under 11.8, citing the release-head semver run | Mirrors 10.8; one evidence-bearing paragraph | ✓ |
| One sentence in the section 11 intro only | Shortest; 11.8 story stays implied | |
| Both: intro sentence plus the 11.8 subsection | Two places to keep in step | |

**User's choice:** 11.8 subsection.

### If check-semver reads red on the release head?

| Option | Description | Selected |
|--------|-------------|----------|
| Hard stop; reshape to additive if possible, else maintainer decides patch-with-register or v0.12.0 | 47 D-00j stop, 53 PD-05-1 reshape first | ✓ |
| Register the break and ship 0.11.1 regardless | Contradicts SHIP-09's quietly-breaking clause | |
| Any red line means v0.12.0, no discussion | A reshapeable item would force a minor | |

**User's choice:** Hard stop, reshape, else maintainer decides.

### Does v0.11.1 get a web3sec hand-off?

| Option | Description | Selected |
|--------|-------------|----------|
| Short hand-off, no return contract | Tag, SHA, run id, registry table, 11.8 and operator pointers, muster pin | ✓ |
| Full hand-off with a return contract, as in Phase 47 | Return fields and a pending todo | |
| No hand-off | Registry and MIGRATION are enough | |

**User's choice:** Short hand-off.

### A consumer smoke on the published 0.11.1?

| Option | Description | Selected |
|--------|-------------|----------|
| Keyless scratch-crate build on 0.11.1, recorded as evidence | cargo add paladin-ai@0.11.1 outside the workspace; failures recorded, not fixed | ✓ |
| Registry table only | As Phases 37 and 47 | |
| Full first-run walk again, keyed half by the maintainer | One more maintainer checkpoint | |

**User's choice:** Keyless scratch-crate build.

**Continue check:** Next area.

---

## Bump commit set and changelogs

### How is the bump set committed?

| Option | Description | Selected |
|--------|-------------|----------|
| Three commits: manifests + strings + openapi; changelog stamps; MIGRATION subsection | Version gate green on commit 1; a defect in one is isolated | ✓ |
| One consolidated chore(release): version 0.11.1 commit | Simplest history, hardest to bisect | |
| Four commits: manifests; strings; stamps; MIGRATION | First commit alone reads red on the version gate | |

**User's choice:** Three commits.

### What goes under the three anchor-less crate changelogs' 0.11.1 heading?

| Option | Description | Selected |
|--------|-------------|----------|
| Entries derived from git log on each crate path since v0.11.0; 'no functional change' only if that log is empty | Sanity-check the other eight the same way | ✓ |
| Add bare [Unreleased] anchors; stamp them empty | Risks repeating 47 WR-01 | |
| Copy the matching lines from the root CHANGELOG [Unreleased] | Consistent by construction, workspace phrasing | |

**User's choice:** Log-derived entries.

### Stamp date and what if the tag slips?

| Option | Description | Selected |
|--------|-------------|----------|
| Date the bump is pushed; redated in a docs-only commit before the tick if the tag slips | The Phase 47 default | ✓ |
| Ask the maintainer for the intended tag date before stamping | One more wait before the bump | |
| Stamp with the tag date only after the merge, on chore/58-close | Tagged tree lacks the [0.11.1] heading the pipeline requires | |

**User's choice:** Push-day stamp, redated if needed.

### Where does the non-bump work sit relative to the bump?

| Option | Description | Selected |
|--------|-------------|----------|
| All before the bump, in a build wave; the re-seal starts only after the last of them | Bump set is the last three commits; fixes stay inside the allowed window | ✓ |
| Bump first, then the fixes, then re-seal | Complicates the release diff and the grep-zero check | |
| Fixes ride a separate PR merged to main first | Breaks the main freeze | |

**User's choice:** All before the bump.

**Continue check:** Finish this area.

---

## Wrap-up

| Option | Description | Selected |
|--------|-------------|----------|
| I'm ready for context | Write CONTEXT.md now | ✓ |
| Explore more gray areas | Post-tag wave, release-run-red stance, body size guard | |

**User's choice:** Ready for context.

## Claude's Discretion

Order of operations inside bump commit 1; root CHANGELOG `[Unreleased]` curation; PR title and
body wording; re-seal command order and output capture; hand-off checkpoint texts;
`58-ACCEPTANCE-AUDIT.md` preamble; plan count and wave layout; the GitHub Release object;
`chore/58-close` contents; the devcontainer install shape; the `gpt-4` replacement literal.

## Deferred Ideas

57-REVIEW CR-01 and WR-01 (v0.12.0, maintainer-owned); the News API key in the URL and the
Treasurer webhook secret placeholder (re-homed with owner and date); FIXES-04 `Cancelled`
(next minor); OSV alerts 23-25 (maintainer); a history purge of the `sk-proj-…` literal
(maintainer); dry-runnable `release.yml` before a tag; a keyed standing post-release smoke; the
`get_job` deprecation attribute (next minor); Codecov token and the v0.10.0 sign-off boxes
(maintainer-only). Reviewed, not folded: the OpenAI Responses API, setup-check output order,
treasurer price digits, DeepSeek reasoning hint todos.
