---
phase: 34-documentation-currency-audit
plan: 09
subsystem: docs
tags: [documentation-audit, work-list-assembly, reconciliation, read-only-proof, mdbook, rustdoc, examples]

# Dependency graph
requires:
  - phase: 34-documentation-currency-audit (plans 34-01 through 34-08)
    provides: "34-AUDIT.md §1-§4 fully swept — 60 MB-nn mdBook findings, 143 RD-nn rustdoc findings, 122 EX-nn example rows (63 build/currency + 59 gap-list)"
provides:
  - "34-AUDIT.md §5: the Phase 35 work list — 60 MB-nn rows, ordered per D-21 (missing superstep-engine page MB-30 leading, then docs/src/SUMMARY.md nav order)"
  - "34-AUDIT.md §6: the Phase 36 work list — 143 RD-nn rows grouped by crate with same-source-line rows cross-referenced via Blocks (75 location groups), 64 EX-nn work items (5 currency findings + 59 gap-list rows), plus a separate 58-row 'confirmed current' list for ID-completeness"
  - "34-AUDIT.md §7 and a closed deferred-items.md (5 entries across 4 contributing plans: Docker-coverage-walk remainder, public-API example-heading gate drift, stale CI comment, PROJECT.md crate-examples contradiction, object-store/RustFS evaluation remainder)"
  - "34-AUDIT.md close-out subsection: counted totals for every table, the D-23 re-run rule, and the D-00d WINDOWS.md-untouched statement backed by a diff"
  - "Both-directions ID reconciliation proven by diff (not asserted) in 34-AUDIT.md and 34-EVIDENCE.md rows 172-194"
  - "SC5 proven over the whole phase range against the pinned Phase 34 start SHA (ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD, both against '. :!.planning' and against WINDOWS.md), both empty and recorded verbatim"
affects: [35-mdbook-documentation-currency-repair, 36-rustdoc-examples-currency-repair]

# Tech tracking
tech-stack:
  added: []
  patterns: ["work-list assembly by mechanical transcription (Location/Size/Cites copied verbatim from the originating §2/§3/§4 row, never re-derived)", "same-file:line grouping across independent measurement runs as the objective, non-invented basis for D-21's 'shared fix pattern' Blocks grouping"]

key-files:
  created: []
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md
    - .planning/phases/34-documentation-currency-audit/deferred-items.md
    - .planning/phases/34-documentation-currency-audit/34-check.sh

key-decisions:
  - "MB-30 (the missing superstep-engine page) leads §5 as the one blocking L item, with control-flow.md's own MB-22 named in its Blocks cell — justified by control-flow.md's own in-tree text pointing forward to the not-yet-written page, not an invented dependency"
  - "RD-nn Blocks grouping is derived mechanically from identical File:line across the two independent measurement runs (default-feature workspace vs per-crate all-features), never from subjective judgment of 'similar' warnings — 75 location groups, 63 singletons, 12 multi-row groups"
  - "The 58 EX-nn rows the build/currency sweep confirmed 'current' are excluded from the Order-numbered work-item sequence (mirroring §2's own rule that a current mdBook page mints no MB-nn) but are still listed, once, in a clearly separate 'confirmed current' subsection so every EX-nn ID minted in §4 is still mechanically routed"
  - "Rule 1 fix to 34-check.sh assertion (b): the original whole-file duplicate-ID scan is structurally incompatible with §5/§6 legitimately referencing every ID a second time (D-03's own citability guarantee requires that second occurrence) — scoped the uniqueness check to §1-§4, the ID-minting sections, so a genuine duplicate mint still fails identically"
  - "Two new deferred-items.md entries added under 'Plan 34-09, Task 2': the PROJECT.md 'no crate ships its own examples/' claim (contradicted by crates/paladin-llm/examples/) and the object-store (MinIO->RustFS) evaluation itself, both planning-corpus/infrastructure findings with no MB/RD/EX slot per D-19"

requirements-completed: [CURR-04, CURR-05]

coverage:
  - id: D1
    description: "Phase 35 work list (§5): every MB-nn from §2 routed exactly once, ordered per D-21, with Location/Size/Cites copied verbatim from the originating row"
    requirement: "CURR-04"
    verification:
      - kind: other
        ref: "bash 34-check.sh --final assertion (g); 34-AUDIT.md Reconciliation subsection diff (60/60 exact set equality)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Phase 36 work list (§6): every RD-nn from §3 and every EX-nn from §4 routed exactly once (work item or confirmed-current), ordered per D-21, blocking relationships structurally valid"
    requirement: "CURR-04"
    verification:
      - kind: other
        ref: "bash 34-check.sh --final assertion (g); 34-AUDIT.md Reconciliation subsection diff (143/143 RD, 122/122 EX exact set equality); Blocks-precedes-target check (0 violations across 267 rows)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Deferred register and §7 closed: every non-documentation, non-example finding routed with an out-of-scope rationale, pointed to from §7"
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "grep -c '^## Plan 34-0' deferred-items.md == 4; 34-AUDIT.md §7 five pointer lines"
        status: pass
    human_judgment: false
  - id: D4
    description: "Success Criterion 5 proven mechanically over the whole phase range: no file outside .planning/ touched, WINDOWS.md untouched"
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- . ':!.planning' (empty); git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- .planning/WINDOWS.md (empty); git status --porcelain -- . ':!.planning' (empty)"
        status: pass
    human_judgment: false

duration: ~75min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 9: Work-List Assembly and Phase Close-Out Summary

**Partitioned the 325-finding inventory into a 60-item Phase 35 mdBook work list and a 207-item Phase 36 rustdoc/examples work list, both mechanically reconciled against their §2/§3/§4 origins, then closed the phase's deferred register and proved the read-only guarantee over the whole commit range.**

## Performance

- **Duration:** ~75 min
- **Tasks:** 2
- **Files modified:** 4 (`34-AUDIT.md`, `34-EVIDENCE.md`, `deferred-items.md`, `34-check.sh`)

## Accomplishments

- Parsed all 60 `MB-nn`, 143 `RD-nn` and 122 `EX-nn` rows out of §2/§3/§4 (verified against the phase's own high-water marks) and assembled §5 (Phase 35, 60 rows) and §6 (Phase 36, 207 Order-numbered work-item rows + 58 confirmed-current rows), copying Location/Size/Cites verbatim from each originating row rather than re-deriving them.
- Ordered both lists per D-21: §5 leads with `MB-30` (the missing superstep-engine page `control-flow.md` already points forward to), then follows `docs/src/SUMMARY.md` nav order; §6 groups `RD-nn` by crate (the same crate order the default-feature `cargo doc` run's own summary lines use) with same-file:line rows across the two independent measurement runs cross-referenced via `Blocks` (75 location groups, 12 with a follower), then the `EX-nn` currency findings and gap-list rows.
- Proved both-directions reconciliation by diff, not assertion: every ID minted in §2/§3/§4 appears in exactly one of §5/§6, and every ID in §5/§6 resolves to an originating row (three empty `diff`s, 60/143/122 exact set equality).
- Closed `deferred-items.md` (added a `Plan 34-09, Task 2` heading with the PROJECT.md crate-examples contradiction and the object-store/RustFS evaluation remainder) and `34-AUDIT.md` §7 (five pointer lines, one per deferred entry, no evidence duplicated).
- Wrote the phase close-out subsection: counted totals for every table, the D-23 re-run rule, and the D-00d WINDOWS.md-untouched statement backed by a recorded diff.
- Proved Success Criterion 5 over the whole phase range: both `git diff --stat` checks (against `. ':!.planning'` and against `.planning/WINDOWS.md`) empty from the pinned Phase 34 start SHA through this plan's own final commit; `bash 34-check.sh --final` all 8 assertions pass.

## Task Commits

1. **Task 1: Assemble the Phase 35 and Phase 36 work lists, ordered and sized** - `847a0a2a` (docs)
2. **Task 2: Deferred register, audit close-out and the phase-range read-only proof** - `f2b46ce8` (docs)

_No plan-metadata commit yet — STATE.md/ROADMAP.md/REQUIREMENTS.md updates and this SUMMARY.md are committed next, per the executor's final_commit step._

## Files Created/Modified

- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` - Wrote §5 (Phase 35 work list), §6 (Phase 36 work list, including the confirmed-current EX-nn subsection and the Reconciliation subsection), §7 (deferred-routing pointers), and the phase close-out subsection.
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` - Appended `Plan 34-09, Task 1` (rows 172-184: extraction/parsing method, nav/crate-order derivation, Task 1 `<verify>` run, bidirectional reconciliation, the `34-check.sh` assertion-(b) deviation) and `Plan 34-09, Task 2` (rows 185-194: deferred-register additions, §7 write, counted totals, both phase-range diffs, the full `34-check.sh --final` verbatim output).
- `.planning/phases/34-documentation-currency-audit/deferred-items.md` - Added a `Plan 34-09, Task 2` heading with two entries (PROJECT.md crate-examples contradiction; object-store/RustFS evaluation remainder).
- `.planning/phases/34-documentation-currency-audit/34-check.sh` - Fixed assertion (b)'s duplicate-ID scan to be scoped to §1-§4 (see Deviations).

## Decisions Made

- **MB-30 leads §5.** The missing superstep-engine page is the phase's one clear "blocking `L` item" per D-21 — `control-flow.md`'s own in-tree text ("the full engine guide is future documentation") is direct evidence the page it names should exist before that page's own `MB-22` fix references it, so `MB-22` is named in `MB-30`'s `Blocks` cell rather than left empty or filled with an invented dependency.
- **RD-nn grouping is mechanical, not judgment-based.** Rather than deciding by inspection which warnings "share a fix pattern," every `RD-nn` row was grouped by exact `File:line` identity across the two independent measurement runs (default-feature workspace, per-crate all-features) — the same source line found twice by two different `cargo doc` invocations is definitionally the same fix, never two separate defects. This produced 75 location groups (63 with no follower, 12 with 1-3 followers) with zero manual curation.
- **58 "confirmed current" EX-nn rows are not work items.** §4's build/currency sweep (plan 34-08) minted an `EX-nn` ID for every program regardless of verdict, unlike §2's mdBook table (which mints `MB-nn` only for `stale`/`missing` rows). To keep §6 an honest work list — only genuine findings, each classifiable under one of the four ROADMAP classifications — the 58 rows confirmed `current` are listed in their own clearly-labeled subsection (mirroring §2's own "a `current` page mints no ID" convention) rather than being force-fit into the four classifications or silently dropped from ID-completeness routing.
- **34-check.sh assertion (b) fix.** See Deviations below.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `34-check.sh` assertion (b) incompatible with §5/§6 existing**
- **Found during:** Task 1, first `bash 34-check.sh --seed` re-run after §5/§6 were written.
- **Issue:** Assertion (b) scanned the *whole file* for any `MB-`/`RD-`/`EX-` token appearing more than once, which was a correct duplicate-mint detector through plan 34-08 (when §5/§6/§7 were still `Empty.` stubs and every ID appeared exactly once). Once §5/§6 are populated — which is the explicit deliverable of this plan, and a hard requirement of D-03's "Phases 35/36 close items by ID" citability guarantee and this plan's own Task 1 `<verify>` line — every one of the 265 `MB`/`RD`/`EX` IDs is *designed* to appear a second time, in its work-list row. Run unmodified, assertion (b) reported all 265 IDs as "duplicates," a false positive on every ID rather than catching a genuine double-mint bug.
- **Fix:** Scoped assertion (b)'s scan to `awk '/^## §5/{exit} {print}' "$AUDIT"` — everything before the first `## §5` heading, i.e. §1-§4, the sections where an ID is actually minted. A genuine duplicate mint (the same ID accidentally assigned to two different findings within §2/§3/§4) still fails this narrower check exactly as before; a legitimate §5/§6/§7 cross-reference no longer does.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-check.sh`
- **Verification:** `bash 34-check.sh --seed` and `bash 34-check.sh --final` both pass all assertions after the fix (34-EVIDENCE.md rows 181-183, 194).
- **Committed in:** `847a0a2a` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug)
**Impact on plan:** The fix corrects a check-script bug that was itself an artifact of the phase's own design maturing (stubs → populated work lists); it does not weaken any genuine invariant — a real duplicate ID mint would still be caught. No scope creep.

## Issues Encountered

**Table-cell parsing across §2/§3/§4's dense pipe-table rows.** Several table rows in §2 (Findings/Cites cells) and none in §3/§4 (which split cleanly by `' | '`, verified 9-column and 6-column consistency before trusting the split) contain raw, unescaped shell-pipe characters (`grep ... | wc -l`) inside cell text, which makes a naive whole-row `split('|')` ambiguous. Resolved by parsing §2's main table from the fixed, unambiguous columns at each end of the row (leading `# | Page |` and trailing `Cites | MB ID(s) | Size |`) rather than a uniform column split, and by writing a small Python extraction script (not committed — scratch-only, per the read-only constraint) that was verified against every one of the 60 `MB-nn` IDs, both directly-numbered rows and the two multi-ID rows (`MB-04`/`MB-05` on `introduction.md`, `MB-08`/`MB-09` on `overview.md`) before being trusted for the full transcription.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 35 (mdBook documentation repair) has a complete, ordered, sized 60-item work list at `34-AUDIT.md` §5, headed with the D-23 re-run rule and the measured HEAD SHA.
- Phase 36 (rustdoc/examples repair) has a complete, ordered, sized 207-item work list at `34-AUDIT.md` §6 (plus the 58-row confirmed-current reference list), headed the same way; closing the 75 `RD-nn` location groups by ID also closes their listed `Blocks` followers, and closes `.planning/WINDOWS.md` rows 36 and 37.
- `deferred-items.md` carries 5 entries (4 contributing plans) for maintainer attention outside this milestone's `/gsd-plan-phase 35`/`36` scope: the Docker-machine coverage-reproduction walk, the public-API `# Examples`-heading gate's scope drift, `ci.yml:538`'s stale example-file-count comment, PROJECT.md's crate-examples contradiction, and the MinIO→RustFS evaluation itself.
- Phase 34's own Success Criteria 1-5 are all satisfied and mechanically proven (34-check.sh --final, the phase-range read-only diffs); the orchestrator's own phase verification is the next step, not further execution here.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- FOUND: `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md`
- FOUND: `.planning/phases/34-documentation-currency-audit/deferred-items.md`
- FOUND: `.planning/phases/34-documentation-currency-audit/34-check.sh`
- FOUND: `.planning/phases/34-documentation-currency-audit/34-09-SUMMARY.md`
- FOUND commit `847a0a2a` (Task 1)
- FOUND commit `f2b46ce8` (Task 2)
