---
phase: 34-documentation-currency-audit
plan: 03
subsystem: docs
tags: [documentation-audit, mdbook, crate-map, vocabulary, msrv, token-economy]

# Dependency graph
requires:
  - phase: 34-02
    provides: the D-08 shipped-surface checklist (91 SS-nn rows) and 34-shipped-tokens.txt, closing 34-signals.sh's class 9 SKIPPED path
provides:
  - 34-AUDIT.md §2 build-baseline subsection (mdBook build, linkcheck, doc-examples/doc-config gates all measured green)
  - 34-AUDIT.md §2 orphan/vocabulary/object-store subsections (D-05, D-10, Folded Todos MinIO slice)
  - 34-AUDIT.md §2 — 18 settled root/getting-started/architecture/api-reference page verdicts, 14 new MB-nn rows (MB-04..MB-17)
  - 34-AUDIT.md D-09 subsection — upgrading.md vs MIGRATION.md §9.1/§9.8 row-for-row (4/4, 7/7, zero disagreements)
  - 34-evidence/34-03-mdbook-build.txt — raw build/linkcheck/gate captures
affects: [34-04-mdbook-verdicts, 34-05-mdbook-verdicts, 34-09-work-list-assembly]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Live-code cross-check as the primary D-07 evidence source for architecture/API-reference pages (struct field reads, trait signature reads, constructor arity reads) — heavier than a grep signal class but the only way to catch a page whose code sample silently drifted from the live type"

key-files:
  created:
    - .planning/phases/34-documentation-currency-audit/34-evidence/34-03-mdbook-build.txt
  modified:
    - .planning/phases/34-documentation-currency-audit/34-AUDIT.md
    - .planning/phases/34-documentation-currency-audit/34-EVIDENCE.md

key-decisions:
  - "Two architecture pages (crate-map.md, overview.md) and both crate-map.md siblings judged stale primarily on a measured crate-graph gap: paladin-eval and paladin-herald absent from every crate table/diagram, and the Phase 33 D-03 paladin-memory->paladin-llm edge missing from both crate-map.md mermaid diagrams despite being correctly described in architecture/crate-map.md's own prose"
  - "Pure version-pin staleness (quickstart.md, installation.md, stable-api.md, feature-flags.md, migration-guide.md) cited against the general workspace-version/MSRV facts (Cargo.toml, rust-toolchain.toml) rather than forcing an artificial match to an unrelated §1 SS-nn row, since no §1 row records a bare version bump"
  - "Two pre-v0.10.0 code-sample mismatches (design-patterns.md's PaladinExecutionService::new 4th-arg, hexagonal-design.md's LlmPort::generate signature) recorded as stale under D-00g (shipped tree outranks document) with no Phase 22-33 REQ-ID, since both defects predate this milestone"
  - "migration-guide.md's D-09 row-for-row check finds zero disagreements by design, not by omission — the page deliberately points to upgrading.md/MIGRATION.md rather than duplicating §9.1/§9.8 content; its one recorded defect (MB-16) is a self-contradictory framing line ('current v0.5.0' immediately above its own v0.10.0 section), not a content disagreement"

patterns-established:
  - "D-09 comparison method: read MIGRATION.md §9.1/§9.8 in full, build the entry-count baseline via grep/sed, then check each entry as carried/contradicted/omitted against the target page — recorded as its own AUDIT.md subsection, not folded into a single §2 row's findings cell"

requirements-completed: []

coverage:
  - id: D1
    description: "The mdBook build, linkcheck, doc-examples and doc-config gates are all measured live (not assumed) and recorded in 34-AUDIT.md's build-baseline subsection with the verbatim linkcheck summary line and gate results"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "test -s .planning/phases/34-documentation-currency-audit/34-evidence/34-03-mdbook-build.txt && grep -qi linkcheck 34-evidence/34-03-mdbook-build.txt"
        status: pass
    human_judgment: false
  - id: D2
    description: "The corpus-wide vocabulary sweep (Quartermaster, Phase 31 D-29 token_count hit list) is recorded with real hit counts and dispositions, including the commissary.md:7 ADR-pointer hit"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep -c 'docs/src/architecture/commissary.md:7' .planning/phases/34-documentation-currency-audit/34-AUDIT.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "All 18 root/getting-started/architecture/api-reference pages carry settled, command-backed verdicts (12 stale, 6 current), each findings cell naming at least 3 signal classes"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "bash .planning/phases/34-documentation-currency-audit/34-check.sh --seed (all 5 seed-mode assertions PASS)"
        status: pass
    human_judgment: false
  - id: D4
    description: "upgrading.md and migration-guide.md checked row-for-row against MIGRATION.md §9.1 (4 entries) and §9.8 (7 steps); upgrading.md carries all 11 entries with zero disagreements; migration-guide.md's deliberate-pointer design produces zero disagreements by construction"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "D-09 subsection in 34-AUDIT.md, entry-by-entry table for both pages"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both crate-map.md pages and architecture/overview.md checked against ls crates/ and the Phase 33 paladin-memory->paladin-llm edge; the edge is confirmed missing from both mermaid diagrams and recorded as MB-13/MB-14"
    requirement: "CURR-01"
    verification:
      - kind: other
        ref: "grep 'Phase 33 — \\`paladin-memory\\`' 34-AUDIT.md (present in both crate-map.md rows)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Phase remains read-only outside .planning/ across both task commits (SC5/D-22/CURR-05); mermaid install did not mutate docs/; 34-check.sh --seed stays green"
    requirement: "CURR-05"
    verification:
      - kind: other
        ref: "git status --porcelain -- . ':!.planning' (empty); git status --porcelain -- docs (empty)"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-09-17
status: complete
---

# Phase 34 Plan 03: mdBook Build Baseline and First 18 Page Verdicts Summary

**Measured the mdBook build/linkcheck/gate baseline green, confirmed zero mdBook orphans and zero retired-image references, and settled all 18 root/getting-started/architecture/api-reference page verdicts — 12 stale (14 new MB-04..MB-17 rows, including two crate-map.md pages missing the Phase 33 paladin-memory→paladin-llm edge and a stale-crate-count omitting paladin-eval/paladin-herald) and 6 current (upgrading.md proven 4/4 + 7/7 against MIGRATION.md §9.1/§9.8 with zero disagreements).**

## Performance

- **Duration:** 18 min
- **Started:** 2026-09-17T04:02:39Z
- **Completed:** 2026-09-17T04:19:49Z
- **Tasks:** 2 completed
- **Files modified:** 3 (2 modified, 1 created)

## Accomplishments
- Ran the exact `docs.yml` sequence (`mdbook-mermaid install docs/` → `mdbook build docs/` with the linkcheck backend → `check-doc-examples.sh` → `check-doc-config.sh`), all green: mermaid install did not mutate `docs/`, linkcheck found 1006 links / 0 broken, doc-examples Layer 1/1b/2 all passed (0 checked / 616 skipped / 0 failed for the inline scan), doc-config checked 154 YAML blocks with 0 failures
- Confirmed zero mdBook orphans by direct measurement (93 on-disk pages, 93 nav-reachable) and zero retired MinIO Docker Hub/`dl.min.io` references across 247 object-store grep hits (every image pin already `quay.io/minio/minio:RELEASE.2025-09-07...`)
- Recorded the Phase 31 D-29 `token_count`/`TokenUsage` re-check across 10 pages: 8 clean, `memory-management.md` clean by content (a distinct, unmigrated `GarrisonEntry.token_count` field), `domain-model.md` offending (MB-03: `GarrisonEntry` snippet stale on 4 counts vs the live 7-field struct, missing the Phase 26 `is_summary` field)
- Settled 18 page verdicts: **current** — `docs/src/SUMMARY.md`, `getting-started/configuration.md`, `architecture/commissary.md`, `api-reference/platform-api.md`, `api-reference/upgrading.md`, `api-reference/wargraph-doc-schema.md`; **stale** — `introduction.md` (MB-04/MB-05: vocab table gap + missing links to every Phase 22-33 guide), `getting-started/installation.md` (MB-06: MSRV 1.85 vs live 1.88, version pins, 15+ undocumented feature flags), `getting-started/quickstart.md` (MB-07: version pin only, code samples confirmed current), `architecture/overview.md` (MB-08/MB-09: crate-count/table gap + zero Phase 22-33 concepts named), `architecture/hexagonal-design.md` (MB-10: `LlmPort::generate` signature stale), `architecture/domain-model.md` (MB-11: missing `Battlefield`/`Waypoint`/`Aegis`/`TraceRecord` core entities), `architecture/design-patterns.md` (MB-12: `PaladinExecutionService::new` 4th-arg wrong), `architecture/crate-map.md` (MB-13) and `api-reference/crate-map.md` (MB-14) (both: crate count, missing crates, missing `mem --> llm` edge), `api-reference/feature-flags.md` (MB-15: version pins + missing `otel`/`dev-ui`/`redis-cache`/`storage-postgres`), `api-reference/migration-guide.md` (MB-16: self-contradictory framing line), `api-reference/stable-api.md` (MB-17: whole page pinned to v0.5.0 with obsolete module paths)
- Ran the D-09 row-for-row check: `upgrading.md` carries all 4 §9.1 behavioral-change entries and all 7 §9.8 checklist steps with zero disagreements/omissions against `MIGRATION.md`; `migration-guide.md` deliberately points to `upgrading.md`/`MIGRATION.md` instead of duplicating, so the comparison itself finds zero disagreements by design
- Cross-checked both `crate-map.md` pages and `architecture/overview.md` against `ls crates/` (11 library crates + `doc-examples`, confirming `paladin-eval` and `paladin-herald` both exist and are undocumented) and against the Phase 33 D-03 `paladin-memory` → `paladin-llm` edge (present in prose on `architecture/crate-map.md`, absent from both pages' mermaid diagrams)
- Appended 22 numbered evidence rows (26-38 for Task 1, 39-47 for Task 2) to `34-EVIDENCE.md`, teed the full build/gate captures to `34-evidence/34-03-mdbook-build.txt`

## Task Commits

1. **Task 1: mdBook build baseline, link check, config/doc-example gates and the corpus-wide vocabulary sweep** — `a35e309a` (docs)
2. **Task 2: Settle verdicts for the 18 root, getting-started, architecture and api-reference pages** — `48931bc2` (docs)

_Both commits touch only `.planning/`; no TDD cycle applies to this documentation-only plan._

## Files Created/Modified
- `.planning/phases/34-documentation-currency-audit/34-AUDIT.md` — §2 build-baseline/orphan/vocabulary/object-store subsections, 18 settled page verdicts, D-09 subsection (modified)
- `.planning/phases/34-documentation-currency-audit/34-EVIDENCE.md` — evidence rows 26-47 appended (modified)
- `.planning/phases/34-documentation-currency-audit/34-evidence/34-03-mdbook-build.txt` — verbatim mdbook build/linkcheck/gate captures (created)

## Decisions Made
- Crate-graph staleness (missing `paladin-eval`/`paladin-herald`, missing the Phase 33 `mem --> llm` edge) was treated as the headline finding for both `crate-map.md` pages and `architecture/overview.md` rather than three independent minor findings, since all three trace to the same root cause: no page's crate inventory has been updated since before Phase 28 (`paladin-eval`) or Phase 33 (the new edge).
- Pure version-pin staleness (no §1 SS-nn row exists for "the crate is now published at 0.10.0") was cited against the live `Cargo.toml`/`rust-toolchain.toml` facts directly rather than forcing an artificial SS-nn match, following the spirit of D-06/D-07 rather than the letter of a citation format built for phase-scoped capability additions.
- `design-patterns.md`'s and `hexagonal-design.md`'s code-sample mismatches (both pre-date v0.10.0) were recorded under D-00g rather than left unrecorded, since D-06 measures content against the live tree regardless of which milestone introduced the drift.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reworded 8 prose mentions of newly-minted MB IDs that collided with `34-check.sh`'s duplicate-ID assertion**
- **Found during:** Task 1 and Task 2, first `34-check.sh --seed` re-runs after minting MB-02/MB-03 (Task 1) and MB-04/MB-05/MB-07/MB-08/MB-09/MB-16 (Task 2)
- **Issue:** `34-check.sh` assertion (b) (`grep -oE '(MB|RD|EX)-[0-9]+' | sort | uniq -d` must be empty) flagged each of these IDs as "duplicate" because the row's own findings-cell prose repeated the literal ID string in addition to the row's MB ID(s) cell — the same false-positive class 34-01 and 34-02 already hit and fixed the same way.
- **Fix:** Reworded each prose mention to describe the finding positionally ("first finding, ID in this row's MB ID(s) cell", "the row minted below") instead of repeating the literal ID string, and reworded the Method statement's own illustrative "MB-07 closed by commit X" example to avoid a literal collision with a real, later-minted MB-07.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `bash 34-check.sh --seed` assertion (b) passes; `grep -oE 'MB-[0-9]+' 34-AUDIT.md | sort | uniq -d` prints nothing.
- **Committed in:** `a35e309a` (Task 1), `48931bc2` (Task 2)

**2. [Rule 1 - Bug] Added a third explicit signal-class citation to 9 rows that initially named only 2**
- **Found during:** Task 2, self-checking the acceptance criterion "every settled row's findings cell names at least three signal classes"
- **Issue:** Several rows (SUMMARY.md, introduction.md, quickstart.md, overview.md, hexagonal-design.md, design-patterns.md, migration-guide.md, platform-api.md, upgrading.md) initially named only 2 formally-tagged `class N` checks, relying on an un-tagged "direct check" phrase for a third piece of evidence that doesn't match the acceptance criterion's literal wording.
- **Fix:** Added an explicit third `class N` citation to each (typically `class 1` "none" where no version string exists, or converting an existing "direct check" phrase into a formally-tagged class), without changing any row's verdict or MB ID.
- **Files modified:** `.planning/phases/34-documentation-currency-audit/34-AUDIT.md`
- **Verification:** `grep -F "| $p " 34-AUDIT.md | grep -oE 'class [0-9]' | wc -l` returns ≥3 for all 18 pages.
- **Committed in:** `48931bc2` (Task 2)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs in the plan's own literal ID-uniqueness gate and this task's own acceptance-criteria wording, discovered while proving the task's own `<verify>` block)
**Impact on plan:** Both fixes were required to make the plan's own tracer verification pass as specified; neither touches anything outside `.planning/`, and neither changes any verdict, MB ID, or size already assigned.

## Issues Encountered
- The D-29 `token_count` hit list (compiled by Phase 31) includes two pages (`domain-model.md`, `memory-management.md`) whose `token_count` references turned out to be about `GarrisonEntry.token_count` — a field Phase 31's `TokenUsage` carrier work never touched — rather than the `PaladinResult`/`StreamingResponse` bare counts Phase 31 replaced. `memory-management.md`'s references are current (type matches the live `Option<u32>` field); `domain-model.md`'s are stale for an unrelated reason (the whole `GarrisonEntry` snippet predates the Phase 26 `is_summary` addition). Both are recorded with their real disposition rather than assumed offending by list membership alone.
- `architecture/crate-map.md`'s own prose (lines 174-176) correctly describes the Phase 33 `paladin-memory` → `paladin-llm` dependency, while the mermaid diagram three sections above it does not show the edge — the page contradicts itself, not just the tree. Recorded as a diagram defect (a reader parses the diagram, not the prose) rather than judging the page current because *some* of its content is right.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- `34-AUDIT.md` §2 now carries 19 settled rows (1 worked by 34-01 + 18 by this plan) out of 93; 75 remain `pending` for plans 34-04 and 34-05.
- MB numbering is at MB-17; plans 34-04/34-05 continue from MB-18 without renumbering any row above.
- `34-check.sh --seed` remains green; `--final` mode's three additional assertions are still not exercised (expected — the §2 sweep is not complete, `examples/*.rs` mapping is plan 34-08's scope, and work-list assembly is plan 34-09's scope).
- No blockers. The crate-graph staleness (missing `paladin-eval`/`paladin-herald`, missing edges) recorded here across three pages is a strong signal that Phase 35's crate-map fix should be planned as one coordinated edit across `architecture/crate-map.md`, `api-reference/crate-map.md` and `architecture/overview.md` rather than three independent page fixes, since all three need the same corrected crate inventory and dependency graph.

---
*Phase: 34-documentation-currency-audit*
*Completed: 2026-09-17*
</content>
