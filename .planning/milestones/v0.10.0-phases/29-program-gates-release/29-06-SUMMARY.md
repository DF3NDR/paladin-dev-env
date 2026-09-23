---
phase: 29-program-gates-release
plan: 06
subsystem: docs
tags: [mdbook, migration-guide, documentation, ubiquitous-language]

# Dependency graph
requires:
  - phase: 29-program-gates-release
    provides: "MIGRATION.md fully closed (29-05) — §9.1 M-B-01..04 rows and the §9.8 7-step checklist this plan mirrors verbatim into the new Upgrading page"
provides:
  - "docs/src/api-reference/upgrading.md — the mdBook Upgrading page SHIP-01 requires, registered in SUMMARY.md above the Migration Guide entry"
  - "A v0.10.0 pointer section on the historical migration-guide.md, plus the errata note closing the last item of D-25's bounded doc sweep"
affects: [29-08/29-09 (release evidence and the final SHIP-01 verdict cite this plan's closed doc sweep)]

# Tech tracking
tech-stack:
  added: []
  patterns: [hand-written-page-not-include, repository-url-link-not-relative]

key-files:
  created:
    - docs/src/api-reference/upgrading.md
  modified:
    - docs/src/SUMMARY.md
    - docs/src/api-reference/migration-guide.md
    - .project/v0.10.0/00-program-overview.md

key-decisions:
  - "check-doc-examples.sh was not run: it invokes `cargo check` on the doc-examples crate, and this worktree's repo_operating_rules explicitly forbid starting any cargo build here (parallel-wave isolation). This plan touches no {{#include}} anchors and no fenced Rust code block, so the script's outcome could not be affected by this plan's changes; check-doc-config.sh (154 YAML blocks, 0 failed) and `mdbook build docs/` (0 broken links) were run instead, matching the plan's own repo_operating_rules override of the generic <verification> block."
  - "The overview §4 errata was added as a footnote paragraph immediately below the term table rather than editing the Directive row's cell text, keeping the row-cell diff at 0 deletions and the table's twelve rows byte-for-byte otherwise unchanged, satisfying both the plan's own line-budget acceptance criteria and the T-29-06-03 tampering mitigation."

requirements-completed: [SHIP-01]

coverage:
  - id: D1
    description: "docs/src/api-reference/upgrading.md exists, hand-written (no {{#include}}), >=40 lines, carries orientation, the M-B-01..04 table, the mirrored §9.8 checklist, and a MIGRATION.md repository-URL link"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "wc -l upgrading.md = 70; grep for M-B-01..04 all found; grep -c '{{#include' = 0; grep -c 'paladin-cli health'/'graph validate' = 0; terminationGracePeriodSeconds count = 3; setup-check count = 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Upgrading entry is registered in docs/src/SUMMARY.md directly above the Migration Guide entry, with a diff of exactly one added line"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "grep -n -A1 'api-reference/upgrading.md' SUMMARY.md | grep -c 'api-reference/migration-guide.md' = 1; git diff --numstat HEAD~1 -- docs/src/SUMMARY.md = '1 0'"
        status: pass
    human_judgment: false
  - id: D3
    description: "migration-guide.md gains a v0.10.0 pointer section before its existing table of contents, linking upgrading.md and MIGRATION.md by repository URL, noting 0.6-0.9 changes live in CHANGELOG.md, with every historical dated section untouched"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "grep -c 'v0.10.0' = 4; grep -c 'CHANGELOG.md' = 2; grep -c 'Migrating to v0.5.0' = 2 (heading + TOC entry, unchanged)"
        status: pass
    human_judgment: false
  - id: D4
    description: "mdbook build docs/ exits 0 with mdbook-linkcheck at warning-policy=\"error\" finding no broken links, after mdbook-mermaid install docs/ left no new tracked/untracked git entries"
    requirement: SHIP-01
    verification:
      - kind: integration
        ref: "mdbook-mermaid install docs/ then mdbook build docs/ -> 'No broken links found', exit 0; git status --short --ignored confirms mermaid*.js are gitignored (!!), not untracked"
        status: pass
    human_judgment: false
  - id: D5
    description: ".project/v0.10.0/00-program-overview.md §4 gains a one-line errata citing RunOutcome::Halted, WaypointStatus::Halted and directive.rs, confined to at most 4 added / 1 deleted diff lines, with all twelve term rows and names unchanged"
    requirement: SHIP-01
    verification:
      - kind: other
        ref: "grep -c RunOutcome::Halted / WaypointStatus::Halted / directive.rs all >=1; term-row count = 12; all twelve term names present; git diff HEAD~1 add=3 (incl. +++ header), del=1 (incl. --- header, 0 real deletions); README.md and every *.rs file absent from the commit's changed-file list"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 06: mdBook Upgrading page and program-overview errata Summary

**A hand-written `docs/src/api-reference/upgrading.md` page (orientation, the four M-B behavioral changes, the mirrored §9.8 checklist, a repository-URL link to `MIGRATION.md`) is registered above the Migration Guide in `SUMMARY.md`; the historical guide now points forward to it; and the program overview's `NextStep`/`Halt` errata — the last item D-25's bounded doc sweep owed — is recorded as a footnote, not a table rewrite.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments

- Wrote `docs/src/api-reference/upgrading.md`: two orientation paragraphs, a `## Behavioral changes` table condensing MIGRATION.md §9.1's M-B-01…04 to one line each, a `## Upgrade checklist` section mirroring §9.8's 7 ordered steps verbatim (including the real `terminationGracePeriodSeconds: 60` value and the real `paladin-cli setup-check`/`maneuver validate`/`eval run`/`graph export` subcommands — no invented `paladin-cli health` or `graph validate` command), and a closing `## Full migration record` section linking `MIGRATION.md` by its `github.com/DF3NDR/paladin-dev-env/blob/main/...` repository URL rather than an `{{#include}}` (which would fail the linkchecker on `MIGRATION.md`'s relative `crates/…`/`k8s/…` links).
- Inserted `- [Upgrading](api-reference/upgrading.md)` into `docs/src/SUMMARY.md` on the line directly above the existing Migration Guide entry — a one-line diff, nothing else in the file touched.
- Added an `## Upgrading to v0.10.0 (from v0.9.x)` section to `docs/src/api-reference/migration-guide.md`, immediately after its orientation paragraph and before the existing table of contents (which also gained a matching new TOC entry), pointing at the new Upgrading page and `MIGRATION.md`, and noting the 0.6–0.9 changes live in `CHANGELOG.md`. Every existing dated `## Migrating to vX.Y.Z` section is untouched.
- Verified `mdbook-mermaid install docs/` then `mdbook build docs/` exits 0 with `mdbook-linkcheck` reporting "No broken links found" (`warning-policy = "error"`), and confirmed the generated `mermaid*.js` assets are gitignored, not left untracked.
- Added a one-line errata footnote to `.project/v0.10.0/00-program-overview.md` §4, below the twelve-row term table, citing the real `NextStep` enum (`Edges`, `Goto`, `Muster`, `End`, `Parley` — no `Halt`) and the real run-level `RunOutcome::Halted`/`WaypointStatus::Halted` outcome, closing the reconciliation `23-DISCUSSION-LOG.md` line 133 parked for "Phase 29 doc sweep". §4's table rows and all twelve term names are byte-for-byte unchanged.
- Re-verified (without changing) that `README.md`'s MSRV badge (line 8) and prerequisites line (line 101) and `MIGRATION.md` §9.3 all already state 1.88 — recorded here per D-25, no edit made or needed.

## Task Commits

1. **Task 1: Write the Upgrading page, register it in SUMMARY, and point the historical guide at it** — `42edd8d3` (docs)
2. **Task 2: Land the overview §4 errata note and record the MSRV agreement** — `ada9a6f4` (docs)

**Plan metadata:** committed alongside this SUMMARY (see final commit).

## Files Created/Modified

- `docs/src/api-reference/upgrading.md` — new, 70 lines: orientation, M-B-01…04 table, 7-step upgrade checklist, `MIGRATION.md` repository-URL link.
- `docs/src/SUMMARY.md` — one new line, `- [Upgrading](api-reference/upgrading.md)`, directly above the Migration Guide entry.
- `docs/src/api-reference/migration-guide.md` — new `## Upgrading to v0.10.0 (from v0.9.x)` section plus its matching table-of-contents entry; historical content untouched.
- `.project/v0.10.0/00-program-overview.md` — one new errata paragraph below §4's term table; table rows and term names unchanged.

## Decisions Made

- **`check-doc-examples.sh` was not run.** It runs `cargo check --manifest-path crates/doc-examples/Cargo.toml`, and this worktree's repo_operating_rules explicitly prohibit starting any `cargo` build here (parallel-wave isolation with sibling worktree agents). This plan introduces no `{{#include}}` anchor and no fenced Rust code block, so the script's result is unaffected by this plan's changes. Ran the two checks the repo_operating_rules do authorize instead: `check-doc-config.sh` (154 fenced YAML blocks checked, 0 failed) and `mdbook build docs/` (0 broken links). This is a plan-instruction override, not a skipped verification: the operating rules for this specific worktree take precedence over the plan's generic `<verification>` list.
- **The §4 errata is a footnote below the table, not an edit to the Directive row's cell.** Editing the cell text in place would touch a table row the plan's own acceptance criteria and 29-04's ubiquitous-language audit both check byte-for-byte; a footnote paragraph after the table states the same verified fact while leaving every row (including Directive's) completely unchanged, which is also the more conservative reading of "MUST NOT restructure or reclassify any ubiquitous-language term" (T-29-06-03).

## Deviations from Plan

None — plan executed as written, with the one instruction-precedence note above (repo_operating_rules override of the generic `<verification>` block for `check-doc-examples.sh`).

## Issues Encountered

None.

## Known Stubs

None. Every table entry, checklist step, and command named on the new Upgrading page is copied from MIGRATION.md §9.1/§9.8 as plan 29-05 actually wrote it (verified by reading that file directly, not this plan's description of it).

## Threat Flags

None. This plan is documentation-only — no new network endpoint, auth path, file-access pattern, or schema change at a trust boundary. Both STRIDE items this plan's own `<threat_model>` names are mitigated: T-29-06-01 (docs build DoS via a bad relative link) by the hand-written page plus a green `mdbook build docs/`; T-29-06-02 (checklist mirror drift) by reading §9.8 as written and asserting the concrete grace-period value and a real subcommand appear on the page; T-29-06-03 (errata growing into a restructuring) by the footnote-not-cell-edit approach and the measured 3-line-added/0-real-deletion diff; T-29-06-04 (credential-shaped example) — no example on the page carries any key value, only env var names and file paths.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- SHIP-01's "linked from the README and the mdBook Upgrading page" requirement is now fully satisfied: the README link was already present (verified, unchanged) and the Upgrading page now exists, is registered, and builds clean.
- D-25's bounded doc sweep is now complete: the Upgrading page, the migration-guide pointer, the MSRV re-verification, and the `NextStep`/`Halt` errata are all recorded.
- No blockers for 29-07/29-08/29-09.

## Self-Check: PASSED

- `docs/src/api-reference/upgrading.md` — FOUND, 70 lines
- `docs/src/SUMMARY.md` — FOUND, modified, diff +1/-0
- `docs/src/api-reference/migration-guide.md` — FOUND, modified
- `.project/v0.10.0/00-program-overview.md` — FOUND, modified
- Commit `42edd8d3` (Task 1) — FOUND in `git log --oneline`
- Commit `ada9a6f4` (Task 2) — FOUND in `git log --oneline`
- `mdbook build docs/` — exit 0, "No broken links found" — CONFIRMED
- `git diff --name-only HEAD~1 | grep -c 'README.md'` → `0` — CONFIRMED
- `git diff --name-only HEAD~1 | grep -c '\.rs$'` → `0` — CONFIRMED
- Twelve §4 term rows present and unrenamed — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
