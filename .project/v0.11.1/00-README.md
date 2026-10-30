# v0.11.1 — Dogfood Cleanup Milestone: working notes

**Status:** pre-planning notes only. Nothing in this directory is a GSD planning artifact; `.planning/` is
untouched. When v0.11.1 is roadmapped, these notes are the input to `/gsd-new-milestone`, not a substitute
for it.
**Baseline:** v0.11.0 (tag `v0.11.0` → merge commit `70657919`, 2026-10-26; all 12 publishable crates on
crates.io at `0.11.0`). Milestone audit `.planning/v0.11.0-MILESTONE-AUDIT.md` (2026-10-27) reads
`tech_debt`: 63/66 requirements satisfied, 3 superseded, 0 unsatisfied; 15/15 phases verified.
**Snapshot taken:** 2026-10-27, on `main` at `ebf04f1` (one docs commit after the v0.11.0 tag; no `src/`
or `crates/*/src/` change has landed since the tag).

## What v0.11.1 is

A **cleanup and dogfood milestone**: bug fixes, gaps and usability issues that v0.11.0 left behind or that
surface while the maintainer and the web3sec consumer run v0.11.0 for real. Primarily bug fixes; feature
changes only where a bug fix needs one.

## What v0.11.1 is not

- Not a place to finish partially-implemented features that exist only as stubs (content ingestion,
  trigger JSONPath/cooldown, Vault policies, the `paladin user` CLI surface, …). Those stay on the v2 /
  FUT register unless dogfooding shows a stub *panics or misleads* a real user — then the fix is to make it
  refuse cleanly or remove it, not to finish it.
- Not a performance milestone (the OBS-05 tracing-overhead bar is accepted-as-is, row 61 waived).
- Not a design-revisit of accepted Treasurer limits (ADR-0057: check-only boundary, streamed-call overshoot,
  agent-loop no-Waypoint). Those are documented contracts; v0.11.1 only fixes where the *documentation* of
  them is stale or where the behavior contradicts the documentation.

## Files in this directory

| File | Purpose |
|---|---|
| `00-README.md` | This file: scope statement, baseline snapshot, how to use the directory |
| `01-carried-gaps-inventory.md` | Every open item carried out of v0.11.0, with its source of record, a severity read, and a triage recommendation (**in** / **decide** / **out**) |
| `02-dogfood-findings-log.md` | The running log for issues found while dogfooding v0.11.0, with the entry format; pre-seeded with the first-run defects found by reading the tree |
| `03-suggestions-and-scoping.md` | Suggested milestone shape: themes, ordering, exit criteria, explicit exclusions, and what to decide before roadmapping |

## Sources of record (read these, do not duplicate them)

- `.planning/v0.11.0-MILESTONE-AUDIT.md` — the `tech_debt` frontmatter is the authoritative per-phase list.
- `.planning/WINDOWS.md` — 13 open rows: 56, 58, 62–69, 73, 77, 79.
- `.planning/todos/pending/` — 4 pending todos.
- `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md` §"Findings carried forward (D-00d)".
- `.planning/phases/40-…/40-REVIEW.md` WR-01..04, IN-04 — the one review whose warnings were never
  dispositioned.
- `.planning/phases/{42,50,51}-…/deferred-items.md` — small per-phase registers with open rows.
- `CHANGELOG.md` `[0.11.0]` → *Known limitations* — the user-facing statement of what is accepted.
- `SECURITY-EXCEPTIONS.md` — the three `zip 3.0.0` exceptions (review 2026-12-31).
- GitHub: one open issue (`DF3NDR/paladin-dev-env#41`), zero open PRs as of the snapshot.

## Inputs still to arrive

- The web3sec consumer's return against the `v0.11.0` tag (`47-HANDOFF-web3sec.md` §6 return contract:
  commit SHA, D-2 choice, gate totals, divergences). Its "divergences" list is expected to be the single
  richest source of v0.11.1 bug reports.
- The maintainer's own dogfood findings (log them in `02-dogfood-findings-log.md`).
