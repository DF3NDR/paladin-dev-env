---
phase: 30-token-economy-vocabulary-commissary-anchoring
plan: 01
subsystem: docs
tags: [adr, mdbook, ubiquitous-language, commissary, token-economy]

# Dependency graph
requires: []
provides:
  - "ADR-0049: Commissary design, rename rationale, and the nine-name rejected list"
  - "The plain-vs-Medieval vocabulary rule stated in PROJECT.md and domain-model.md"
  - "Commissary present in all three ubiquitous-language lists (PROJECT.md, domain-model.md, copilot-instructions.md)"
  - "docs/src/architecture/commissary.md, linked from the architecture nav, mdbook build green"
affects: [30-02, 30-03, 33-commissary-in-tree-adoption]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Plain-vs-Medieval vocabulary split: units/measures and technical port traits keep plain industry names; domain roles/places/events get Medieval-Military names (ADR-0049)"
    - "ADR provenance citing an abandoned branch by full git show path, never renumbered into the in-tree series"

key-files:
  created:
    - .planning/decisions/0049-commissary-design-and-rename.md
    - docs/src/architecture/commissary.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - docs/src/architecture/domain-model.md
    - .github/copilot-instructions.md
    - docs/src/SUMMARY.md

key-decisions:
  - "ADR-0049 reconstructs the Commissary design and rename rationale from the abandoned branch's ADR-0010 (cited by full git show origin/feature/quartermaster-prompt-budgeting path) and port commits 348f5910/35fd8390, never copying either wholesale."
  - "All nine historically-rejected candidate names (Quartermaster, Convoy, apportion, Provisioner, ProvisioningPlan, Muster/muster, provision(), ContextRation, Allocation) are recorded in ADR-0049 Considered Options, each with its own reason and origin (abandoned-branch ADR-0010 Vocabulary section vs. the port commits' 0/0-verified naming list)."
  - "The reserved spend-governance officer name (Treasurer) is named in PROJECT.md's Ubiquitous language bullet as reserved prose only — it does not join the enumerated term list and gets no table row in domain-model.md or copilot-instructions.md, per VOCAB-01/empty."
  - "The Commissary mdBook page's two usage sketches are fenced rust,ignore rather than live doctests, because the shapes they mirror (MockCounter, capabilities_with_window) are test-only helpers in commissary.rs; inventing a public substitute was out of scope."
  - "Ran `mdbook-mermaid install docs/` locally (the same step docs.yml's CI job runs) to generate the gitignored mermaid.min.js/mermaid-init.js assets `mdbook build docs/` requires — a local-environment setup step, not a code or doc change, and nothing generated was committed."

patterns-established:
  - "ADR heading skeleton and numbering procedure (PROMOTION.md Part A) applied for the first ADR in the Phase 30 series: Status/Context/Decision/Considered Options/Code Locations/Code Conformance/Downstream Consumers, with Considered Options and Code Locations as bulleted lists for adr-parser.cjs."

requirements-completed: [VOCAB-01, VOCAB-02, VOCAB-03]

coverage:
  - id: D1
    description: "ADR-0049 records the Commissary design, the Quartermaster->Commissary rename rationale, and the nine-name rejected list, with provenance to the abandoned branch and the port commits."
    requirement: "VOCAB-02"
    verification:
      - kind: other
        ref: "test -f .planning/decisions/0049-commissary-design-and-rename.md && grep '^## ' matches the seven required headings in order"
        status: pass
      - kind: other
        ref: "grep of all nine rejected candidate names, each on a bulleted line"
        status: pass
    human_judgment: false
  - id: D2
    description: "The plain-vs-Medieval vocabulary rule is stated in PROJECT.md's Ubiquitous language bullet and domain-model.md's Naming Convention section, and Commissary appears exactly once in each of the three vocabulary lists with no spelling variants."
    requirement: "VOCAB-01"
    verification:
      - kind: other
        ref: "grep -c '**Commissary**' on domain-model.md and copilot-instructions.md; grep -c 'Commissary' vs grep -ci 'commissary' equality on all three list files"
        status: pass
    human_judgment: false
  - id: D3
    description: "docs/src/architecture/commissary.md exists, is linked once from the architecture nav, contains a mermaid flow diagram and two rust,ignore sketches mirrored from real tests, and names zero invented API or types."
    requirement: "VOCAB-03"
    verification:
      - kind: other
        ref: "the two anti-invention gates (pub fn / pub struct|enum cross-checks against commissary.rs) — 0 failures"
        status: pass
      - kind: other
        ref: "mdbook build docs/ exits 0, mdbook_linkcheck reports 'No broken links found'"
        status: pass
    human_judgment: false

# Metrics
duration: 25min
completed: 2026-09-14
status: complete
---

# Phase 30 Plan 01: Vocabulary Rule & Commissary Anchoring Summary

**ADR-0049 records the `Commissary` design and its nine-name rejected-name history; the
plain-vs-Medieval vocabulary rule and `Commissary` now agree across all three ubiquitous-language
lists; a new mdBook page gives the shipped service a documentation home reachable from the
architecture nav, with `mdbook build docs/` green.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-09-14T18:22:00Z (approx.)
- **Completed:** 2026-09-14T18:46:57Z
- **Tasks:** 2
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- ADR-0049 (`.planning/decisions/0049-commissary-design-and-rename.md`) records the `Commissary`
  design (`verify_fits` guard + `dispense` allocator, fail-loud/never-silent), the
  Quartermaster→Commissary rename rationale (citing port commits `348f5910`/`35fd8390`), and all
  nine historically-rejected candidate names, reconstructed from the abandoned branch's own
  ADR-0010 (cited by its full `git show origin/feature/quartermaster-prompt-budgeting` path,
  never copied wholesale or renumbered into the in-tree series).
- `PROMOTION.md` indexes ADR-0049 and now reads `**Next free ADR number: 0050**`, with a dated
  note recording the advance.
- `.planning/PROJECT.md` gains a Key Decisions row linking ADR-0049 and states the plain-vs-Medieval
  vocabulary rule in its Ubiquitous language bullet, adding `Commissary` to the enumerated term
  list while naming `Treasurer` as reserved prose only.
- `docs/src/architecture/domain-model.md` gains the same vocabulary-rule prose plus a `Commissary`
  table row; `.github/copilot-instructions.md` gains a matching `Commissary` row in its own
  Naming Convention table. All three lists agree and none introduces a bolded `Treasurer` row.
- `docs/src/architecture/commissary.md` (new) documents the concept, the full public model
  (`Consignment`, `ConsignmentItem`, `DispensedItem`, `ShedItem`, `Stockpile`, `CommissaryPlan`,
  `CommissaryError`), a mermaid flow diagram, two `rust,ignore` usage sketches mirrored verbatim
  from real `commissary.rs` unit tests, and an honesty-about-exactness note — linked once from
  `docs/src/SUMMARY.md`'s Architecture nav.
- `mdbook build docs/` exits 0 with `mdbook_linkcheck` reporting "No broken links found", after
  both task commits.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer): Commissary end-to-end — ADR-0049, the vocabulary rule, and all three lists** - `3efcf2ff` (docs)
2. **Task 2: The Commissary mdBook page, reachable from the architecture nav** - `df3fbc66` (docs)

## Files Created/Modified
- `.planning/decisions/0049-commissary-design-and-rename.md` - New ADR: Commissary design, rename rationale, nine rejected names
- `.planning/decisions/PROMOTION.md` - Index row for ADR-0049; `Next free ADR number` advanced to 0050; dated note
- `.planning/PROJECT.md` - New Key Decisions row; Ubiquitous language bullet states the vocabulary rule and adds Commissary
- `docs/src/architecture/domain-model.md` - Commissary table row + plain-vs-Medieval vocabulary-rule prose
- `.github/copilot-instructions.md` - Commissary row in the Naming Convention table
- `docs/src/architecture/commissary.md` - New mdBook page: concept, model table, mermaid diagram, two usage sketches, honesty note, see-also
- `docs/src/SUMMARY.md` - One new nav line under `# Architecture`, `- [Commissary](architecture/commissary.md)`

## Decisions Made
- ADR-0049's rejected-name list carries all nine names from both the abandoned branch's own
  ADR-0010 "Vocabulary" section (`Muster`/`muster`, `provision()`, `ContextRation`, `Allocation`)
  and the port commits' 0/0-verified naming list (`Quartermaster`, `Convoy`, `apportion`,
  `Provisioner`, `ProvisioningPlan`), each attributed to its origin.
- The reserved `Treasurer` term is named in `PROJECT.md`'s Ubiquitous language bullet as reserved
  prose only, per the plan's explicit instruction and VOCAB-01/empty — it does not appear as a
  table row in `domain-model.md` or `copilot-instructions.md` (verified: neither file contains a
  bolded `Treasurer` table-row first-cell).
- The Commissary mdBook page's two code sketches are fenced `rust,ignore`, not live doctests,
  because they mirror test-only helpers (`MockCounter`, `capabilities_with_window`) that have no
  public equivalent — inventing one was explicitly out of scope for this phase.
- The Key Decisions table in `PROJECT.md` has a pre-existing gap (ADR-0040 through ADR-0048 have
  no corresponding table rows, though earlier ADRs do) — out of scope for this task (Rule
  2/3 does not apply to pre-existing, unrelated gaps); the new ADR-0049 row was appended after the
  table's actual last row (ADR-0039) rather than attempting to backfill the gap.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Generated the mermaid preprocessor's gitignored asset files before running `mdbook build docs/`**
- **Found during:** Task 1's own `<verify>` step (the plan's required `mdbook build docs/` command)
- **Issue:** `mdbook build docs/` failed with "Unable to copy /workspace/.../docs/mermaid.min.js" — `docs/book.toml`'s `additional-js = ["mermaid.min.js", "mermaid-init.js"]` names two files that are gitignored (`.gitignore:21-22`) and only exist after running `mdbook-mermaid install docs/`, the same step `.github/workflows/docs.yml`'s `Inject mermaid assets` step runs in CI.
- **Fix:** Ran `mdbook-mermaid install docs/` locally, matching the documented CI step exactly. This is a local build-environment setup step, not a code or documentation change — the two generated files are gitignored and were not staged or committed.
- **Files modified:** None (generated files are gitignored; no repo file was changed by this fix).
- **Verification:** `mdbook build docs/` subsequently exits 0 for both Task 1 and Task 2, with `mdbook_linkcheck` reporting "No broken links found".
- **Committed in:** N/A (no file changes to commit — the fix is a local tool-asset generation step, reproducible from `docs.yml`).

**2. [Rule 1 - Bug] Fixed a self-inflicted `(ADR-0049)` grep-count collision in the PROJECT.md prose edit**
- **Found during:** Task 1's own acceptance-criteria verification loop (`grep -c '(ADR-0049)' .planning/PROJECT.md` must return `1`)
- **Issue:** The first draft of the Ubiquitous language bullet referenced "The governing rule (ADR-0049):", which duplicated the literal `(ADR-0049)` string already used by the new Key Decisions row, making the acceptance-criteria grep return `2` instead of `1`.
- **Fix:** Reworded the bullet to "The governing rule, per ADR-0049:" — same citation, no parenthesized `(ADR-0049)` string.
- **Files modified:** `.planning/PROJECT.md`
- **Verification:** `grep -c '(ADR-0049)' .planning/PROJECT.md` returns `1`, confirmed before committing.
- **Committed in:** `3efcf2ff` (part of Task 1's commit — caught and fixed before the commit was made)

---

**Total deviations:** 2 auto-fixed (1 blocking environment-setup fix, 1 self-caught bug in a first-draft edit)
**Impact on plan:** Neither deviation touched scope, added files, or changed any planned artifact's shape. No scope creep.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ADR-0049 is live and cited; the vocabulary rule and `Commissary` agree across all three
  ubiquitous-language lists; ready for plan 30-02 (Treasurer reservation ADR-0050 and the
  versioning-supersession ADR-0051) and plan 30-03 (the `max_tokens` table, `cost_estimate`
  rustdoc, and the Quartermaster purge).
- No blockers. `mdbook build docs/` is green; both commits touch no `.rs`, `Cargo.toml`,
  `Cargo.lock` or `MIGRATION.md` path, and `.planning/phases/` history is untouched by this
  plan's own commits.

## Self-Check: PASSED

Files verified to exist on disk:
- FOUND: `.planning/decisions/0049-commissary-design-and-rename.md`
- FOUND: `docs/src/architecture/commissary.md`
- FOUND (modified): `.planning/decisions/PROMOTION.md`
- FOUND (modified): `.planning/PROJECT.md`
- FOUND (modified): `docs/src/architecture/domain-model.md`
- FOUND (modified): `.github/copilot-instructions.md`
- FOUND (modified): `docs/src/SUMMARY.md`

Commits verified in `git log --oneline`:
- FOUND: `3efcf2ff` — `docs(30-01): anchor Commissary with ADR-0049 and the vocabulary rule`
- FOUND: `df3fbc66` — `docs(30-01): add Commissary mdBook page and architecture nav entry`

Verification commands re-run at Self-Check time:
- `mdbook build docs/` — exit 0, "No broken links found"
- Anti-invention gates (Task 2) — 0 failures across both `pub fn` and `pub struct|enum` checks
- `git diff --name-only f22661e3..HEAD -- '*.rs' Cargo.toml Cargo.lock MIGRATION.md` — empty
- `git diff --name-only f22661e3..HEAD -- .planning/phases/` — empty

---
*Phase: 30-token-economy-vocabulary-commissary-anchoring*
*Completed: 2026-09-14*
