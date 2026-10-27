# Phase 47: v0.11.0 Crate Release - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-24
**Phase:** 47-v0.11.0 Crate Release
**Areas discussed:** Branch & PR shape, Bump mechanics & gate scope, Sign-off home & re-seal table, Post-tag wave & hand-offs

---

## Todo cross-reference

| Option | Description | Selected |
|--------|-------------|----------|
| web3sec single bump | Score 0.6; recheck_by "Phase 47 close"; fold as an input, post-tag wave gives it a dated update and the consumer hand-off | ✓ |
| Content/data orchestration review | Score 0.6; keyword match only; core-layer review | |
| OpenAI Responses API (pro tier) | Score 0.6; new adapter wire, own phase | |
| Verify local make coverage | Score 0.4; re-homed to maintainer by Phase 36.1 D-23 | |

**User's choice:** web3sec single bump folded; the other three reviewed, not folded.

---

## Branch & PR shape

| Option | Description | Selected |
|--------|-------------|----------|
| One PR: bump on #59, retitle | Bump lands as last commits on the existing branch; #59 retitled as the release PR; one merge commit is the tag target (v0.10.0 PR #55 precedent) | ✓ |
| Two PRs: merge #59, then release/v0.11.0 | Merge #59 at 0.10.1, then a tiny bump PR off main (Operator Guide pattern); main briefly at the wrong version, second sign-off cycle | |
| Two PRs: bump branch off #59 head | Stacked release branch; two merges; complicates two-SHA evidence | |

**User's choice:** One PR: bump on #59, retitle.

| Option | Description | Selected |
|--------|-------------|----------|
| main frozen; #59 is the only merge | origin/main stays at df24689c; if main moves, re-seal from the top and maintainer re-confirms | ✓ |
| Allow unrelated docs merges | Merge main back in (merge commit) before the re-seal; costs a re-seal each time | |

**User's choice:** main frozen.

| Option | Description | Selected |
|--------|-------------|----------|
| Agent edits #59 via gh api | REST only (GraphQL blocked); curated body; fallback hand command to maintainer | ✓ |
| Maintainer retitles by hand | Agent drafts into 47-CI-EVIDENCE.md | |

**User's choice:** Agent edits #59 via gh api.

| Option | Description | Selected |
|--------|-------------|----------|
| Keep claude/laughing-dirac-e0h2ax | Renaming closes #59 and loses its thread; STATE citations stay true; release/* ruleset not engaged | ✓ |
| Recreate as release/v0.11.0 | Conventional name; loses PR thread; one-push ruleset applies | |

**User's choice:** Keep the branch name.
**Notes:** Four questions asked; user chose "Next area" without further questions.

---

## Bump mechanics & gate scope

| Option | Description | Selected |
|--------|-------------|----------|
| cargo release version | Install cargo-release; `cargo release version 0.11.0 --execute --no-confirm --workspace`; grep-zero for 0.10.1; hand-touch doc-examples if skipped | ✓ |
| By hand, 13 manifests | sed/edit each pin; v0.10.1 showed pins are easy to miss | |

**User's choice:** cargo release version.

| Option | Description | Selected |
|--------|-------------|----------|
| finalize-crate-changelogs VERSION=0.11.0 | Tool stamps all twelve, dated to the tag date; hand-curate root only if unreadable | ✓ |
| Stamp root by hand, crates by tool | Root curated manually with clean-break callouts up top | |

**User's choice:** finalize-crate-changelogs.

| Option | Description | Selected |
|--------|-------------|----------|
| make openapi, version-only diff expected | Any other diff is a finding and a stop | ✓ |
| Regen and accept whatever changes | Commit the full diff as mechanical | |

**User's choice:** make openapi, version-only diff expected.

| Option | Description | Selected |
|--------|-------------|----------|
| Defer; record as finding | No ci.yml edit on release day; recommend re-pin in next milestone's first phase | ✓ |
| Re-pin to 0.10.1 in the build wave | Truthful gate now, but widens the release PR into CI/allowlist work | |
| Re-pin to 0.11.0 post-tag | On chore/47-close after 0.11.0 is published | |

**User's choice:** Defer; record as finding.
**Notes:** User chose "Next area" rather than the offered follow-ups (build-wave fix boundary, api-surface/muster handling, MIGRATION §10 re-read trigger, heading date source); those are recorded under Claude's Discretion / D-08 with stated defaults.

---

## Sign-off home & re-seal table

| Option | Description | Selected |
|--------|-------------|----------|
| New 47-ACCEPTANCE-AUDIT.md in the phase dir | Own v0.11.0 document beside 47-CI-EVIDENCE.md; the v0.10.0 corpus audit and 29 pointer untouched | ✓ |
| Append §14 to the v0.10.0 corpus audit | Exact 37/37.1 precedent; a v0.10.0 document carrying v0.11.0 sign-off | |
| New .project/v0.11.0/ acceptance audit | Mirror the corpus layout under .project/ | |

**User's choice:** New 47-ACCEPTANCE-AUDIT.md.

| Option | Description | Selected |
|--------|-------------|----------|
| Keep all eight as-is | 37.1 rows 1-8 re-read at 0.11.0; row 1 reads 9.2 and 10.2; row 4 stays at 0.9.0 as the known no-op | ✓ |
| + api-surface baseline | Row 9 | |
| + consumer-surface muster | Row 10 | |
| + coverage ≥ 82 % from CI | Row 11 | |

**User's choice:** Keep all eight as-is (multi-select; no extra rows chosen).

| Option | Description | Selected |
|--------|-------------|----------|
| CI-evidence only, not signed rows | api-surface, muster and coverage recorded as CI-run-table entries | ✓ |
| Add them after all | Rows 9-11 join the signed table | |

**User's choice:** CI-evidence only (confirmation of the previous answer).

| Option | Description | Selected |
|--------|-------------|----------|
| 37.1 D-10 order, tick commit via PR push | Maintainer ticks the file and the tick commit is pushed (or agent commits with reply quoted); CI on the true final SHA; agent verifies tick on origin before the hand-off | ✓ |
| Tick in-session, agent commits | Maintainer answers the checkpoint; agent commits the tick | |

**User's choice:** 37.1 D-10 order.

| Option | Description | Selected |
|--------|-------------|----------|
| Record plainly, as 37.1 did | Each row names its CI job or says it has none; no ci.yml change | ✓ |
| Wire the missing two into ci.yml first | Add check-migration-allowlist and a PR-event publish-dry-run | |

**User's choice:** Record plainly.
**Notes:** User chose "Next area" after the confirmation round.

---

## Post-tag wave & hand-offs

| Option | Description | Selected |
|--------|-------------|----------|
| 47-HANDOFF-web3sec.md + todo update | Outbound hand-off mirroring the inbound ones; todo gets a dated update and stays pending | ✓ |
| Todo update only | No separate file; MIGRATION §10 is the spec | |
| Close the todo at tag time | Paladin's side done at the tag | |

**User's choice:** 47-HANDOFF-web3sec.md + todo update.

| Option | Description | Selected |
|--------|-------------|----------|
| After registry verification, on chore/47-close | SHIP-07 ticked only once the sparse index shows twelve crates at 0.11.0 yanked=false, via gsd-tools | ✓ |
| Tick on the release PR before merge | Ticks a claim not yet true at merge time | |

**User's choice:** After registry verification.

| Option | Description | Selected |
|--------|-------------|----------|
| Post-tag evidence + records; stop at 'ready to close' | Registry table, CI runs, MILESTONES entry, ticks, hand-off, findings; /gsd-complete-milestone is the maintainer's | ✓ |
| Also run /gsd-complete-milestone in the last plan | Phase archives itself (rejected in 37) | |

**User's choice:** Stop at 'ready to close'.

| Option | Description | Selected |
|--------|-------------|----------|
| 37 D-16 unchanged: read-only diagnosis, maintainer decides | Runbook §1-§2, hard stop; option A the recorded default | ✓ |
| Pre-authorise complete-forward re-dispatch | Agent may re-dispatch once on a transient registry error | |

**User's choice:** 37 D-16 unchanged.
**Notes:** User chose "I'm ready for context" at the Done prompt.

---

## Claude's Discretion

- Bump commit granularity (default three commits), stamp date handling if the tag slips, PR title/body wording, re-seal command order and capture, hand-off checkpoint texts, a short preamble in 47-ACCEPTANCE-AUDIT.md, plan count and wave layout, the v0.11.0 GitHub Release object (no manual edit planned), branch deletion after merge.

## Deferred Ideas

- Re-pin CI's cargo-semver-checks baseline to 0.11.0 and add paladin-eval (next milestone's first phase).
- Wire check-migration-allowlist into ci.yml; run Publish Dry Run on pull requests.
- release.yml dry-runnable against a bare SHA (carried from 37/37.1).
- Consumer smoke test as a standing post-release check (carried).
- Path-only workspace dev-dependencies as a rule plus a lint (carried from 37.1).
- Carried from 36.1/37/37.1: `cargo test --workspace --all-features` in CI; removing the eight crate-level rustdoc allows; regenerating doc-coverage-report.md.
