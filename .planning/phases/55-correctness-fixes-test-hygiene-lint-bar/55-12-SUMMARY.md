---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 12
subsystem: release-registers
tags: [changelog, migration, base-layer-review, fixes-01, fixes-02, fixes-03, fixes-06]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "55-01 storage refusal, 55-02 model_unpriced halt, 55-05 versioning fences, 55-06 ingest_content refusal and planner output, 55-09 boot warning"
provides:
  - "crates/paladin-storage/CHANGELOG.md: its first ## [Unreleased] anchor, with the ContentListRepository refusal entry the release tool stamps as [0.11.1]"
  - "MIGRATION.md section 11.1 rows M-B-30..M-B-35, a section 11.6 paragraph on the model_unpriced halt, a section 11.8 step 6 for the paladin::pricing boot warning"
  - "[Unreleased] entries in the root, paladin-core, paladin-content and paladin-web changelogs"
  - "55-BASE-LAYER-REVIEW.md: keep / fence decision with file:line and test evidence for eight items"
  - "the 2026-10-13 base-layer todo closed in .planning/todos/completed/ with a Resolution section"
affects: [55-13, phase 58 release finalize]

tech-stack:
  added: []
  patterns:
    - "A new [Unreleased] anchor is proved on a scratch copy with finalize-crate-changelogs.sh before it is relied on at release"
    - "Review evidence cells name file:line plus an existing test, each confirmed by a fresh read and a fresh caller grep at the head"

key-files:
  created:
    - .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-BASE-LAYER-REVIEW.md
  modified:
    - crates/paladin-storage/CHANGELOG.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-content/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - MIGRATION.md
  moved:
    - ".planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md -> .planning/todos/completed/ (Resolution section appended)"

key-decisions:
  - "PD-12-1 followed: ids M-B-30..M-B-35 in the planned order; M-B-29 was the last section 11.1 id, so no fallback was needed"
  - "The review's verdict is keep everything, fence five fabricating items, remove / rename / finish nothing; options A, B and C are next-minor candidates"

requirements-completed: [FIXES-01, FIXES-02, FIXES-03, FIXES-06]

duration: ~30 min
completed: 2026-11-01
---

# Phase 55 Plan 12: Behaviour records and the base-layer review Summary

**Every behaviour change of Phase 55 is now recorded for the v0.11.1 release (a new `paladin-storage` `[Unreleased]` anchor, four more changelogs, `MIGRATION.md` rows `M-B-30` to `M-B-35`, a section 11.6 paragraph and a section 11.8 operator step), and `55-BASE-LAYER-REVIEW.md` records a keep / fence decision with evidence for each content / general-data item and closes the 2026-10-13 todo.**

## Plan-start SHA

`d6affb869eed35b601894f34c138a22c7d04e017` (`d6affb869`)

## MIGRATION ids used

`M-B-29` was the last section 11.1 row, so the planned ids were used as written:

| Id | Change | Requirement |
|----|--------|-------------|
| `M-B-30` | `ContentListRepository` on SQLite and MySQL refuses instead of panicking | FIXES-01 |
| `M-B-31` | unpriced model at dispatch halts (`model_unpriced`) instead of failing | FIXES-03 |
| `M-B-32` | `paladin::pricing` boot warning per unpriced reachable model | FIXES-06 |
| `M-B-33` | `FileContentFetcher::ingest_content` refuses instead of `Ok(())` | FIXES-02 |
| `M-B-34` | `FieldVersionService` history refusal and the `Uuid::nil()` collection sentinel | FIXES-02 |
| `M-B-35` | planner builds subtasks with an empty expected output | FIXES-02 |

Section 11.6 gained the `model_unpriced` paragraph (run status, SSE `done`, webhook `halted`; OpenAPI changes only the `RunResponse.halt_reason` description; the admission `422` is unchanged). Section 11.8 gained operator step 6. Section 11.2 is still header-only.

## Scratch finalize run (release-path proof)

In a scratch directory under the session scratchpad, `crates/paladin-storage/CHANGELOG.md` was copied to `s/`, and a metadata JSON named that copy's `Cargo.toml` with `publish: null`:

```
./scripts/finalize-crate-changelogs.sh --version 0.11.1 --date 2026-11-01 --metadata-json <scratch>/meta.json
✅ OK: 1 publishable package(s) checked for version '0.11.1' (date 2026-11-01):
  - paladin-storage: finalized
```

Stamped lines in the scratch copy (the real file was not touched by this run):

```
## [Unreleased]

## [0.11.1] - 2026-11-01

### Changed

- Every `ContentListRepository` method on `SqliteStore` (feature `sqlite`) and `MySqlContentRepository` (feature `mysql` ...

## [0.11.0] - 2026-10-26
```

## Base-layer review verdict line

> **Verdict: every item is kept. Five items that invented a value or reported a success they did not earn are fenced (a typed refusal or a documented sentinel); nothing is removed, renamed or finished in this milestone. Finishing or retiring the system is a next-minor decision (see "Next-minor candidates").**

The review has eight item rows. It states the finding that no design record exists beyond the todo's own account, quotes the 2026-10-13 owner position verbatim, explains why nothing is removed (every item is `pub` in a published crate, so removal is a section 11.2 break), and lists options A, B and C as next-minor candidates with their triggers. A fresh caller grep at the head confirmed there is no consumer of the five base items outside `crates/paladin-core/src/base/` (only `tests/entity_rename_guard.rs` prose and fixture strings), none of `ContentListRepository` outside the two adapters and the trait, and none of `ContentIngestionPort::ingest_content`.

## Task Commits

1. **Task 1 (tracer): storage anchor, M-B-30, scratch finalize proof** - `f67c29236` (docs)
2. **Task 2: M-B-31..M-B-35, section 11.6 paragraph, section 11.8 step, four changelogs** - `b51e9ca2e` (docs)
3. **Task 3: review file and the closed todo** - `e7281b212` (docs)

The tracer gate ran in autonomous mode: Task 1's `<verify>` was re-run end to end after its commit and passed (`TRACER_OK`) before Task 2.

## Verification

- `make check-changelogs`: 11 publishable crates, all have a changelog (exit 0). `make check-migration-allowlist`: allowlist set-equal to the (empty) section 11.2 register (exit 0), re-run after Tasks 1 and 2.
- `grep -c TBD MIGRATION.md` prints `0`; the string appears nowhere I added.
- Frozen sections: for the root, `paladin-core`, `paladin-content`, `paladin-web` and `paladin-storage` changelogs, `diff` of everything from `## [0.11.0]` down against the plan-start SHA is empty; `MIGRATION.md` from `# 10.` down is identical. `git diff d6affb869 -- MIGRATION.md CHANGELOG.md crates/*/CHANGELOG.md` has no removed line (additions only). No `#[deprecated]` is mentioned, and section 11.2 has no data row.
- `M-B-30`..`M-B-35` each appear once, in order after `M-B-29`; `M-B-31` appears in section 11.6 and `M-B-32` in section 11.8.
- Review: 10 table-style lines (header, separator, eight rows), the `no design record exists beyond the todo` phrase and `## Next-minor candidates` are present; every test named in the evidence column was found with `grep -rln "fn <name>"`. The todo is in `completed/`, absent from `pending/`, has `## Resolution`, and keeps `resolves_phase: 55`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The review table's separator row did not match the plan's `grep -c '^| '` check**
- **Found during:** Task 3 verify
- **Issue:** a Markdown separator written `|------|...` does not start with `| `, so the verify's count was 9, not the required 10.
- **Fix:** the separator is written `| ---- | ---- | ---- | ---- | ---- |`, which renders identically.
- **Commit:** `e7281b212`

**2. [Rule 3 - Blocking] The verify's lowercase phrase differed from my first wording**
- **Found during:** Task 3 verify
- **Issue:** the no-design-record statement began with a capital letter, so `grep -q 'no design record exists beyond the todo'` did not match.
- **Fix:** reworded to `Finding: no design record exists beyond the todo's own account.`
- **Commit:** `e7281b212`

### Plan text notes (no behaviour change)

- The plan names "section 12.2 rows" for a future retirement. This repository's `MIGRATION.md` has no section 12; the review file says "deliberate-breaking rows in the section 11.2-style X-10 register of the releasing version" instead.
- The plan lists `paladin-ai-core` for the core changelog entries; the file is `crates/paladin-core/CHANGELOG.md` (published name `paladin-ai-core`), and that is the file edited.
- Commit trailers use the attribution the harness supplied for this session (Claude Sonnet 5.5), which differs from the trailer the orchestrator's note named; the session URL line is identical to earlier commits.

**Total deviations:** 2 auto-fixed (both verify-matching text), 3 notes.

## Authentication Gates

None.

## Known Stubs

None. This plan edits registers and a decision record only; no code, so nothing was appended to `.planning/WINDOWS.md`.

## Threat Flags

None. T-55-28 (an unrecorded change or an edited frozen section) is mitigated by the one-row-per-change registers and the additions-only diff check above. T-55-29 (review decisions the code does not support) is mitigated by evidence cells that each cite a `file:line` read at head `b51e9ca2e` and an existing test name.

## Self-Check: PASSED

- FOUND: `55-BASE-LAYER-REVIEW.md`, `.planning/todos/completed/2026-10-13-review-content-data-orchestration-base-items.md`; MISSING (intentionally moved): the `pending/` copy.
- FOUND: `## [Unreleased]` in `crates/paladin-storage/CHANGELOG.md`; rows `M-B-30` to `M-B-35` in `MIGRATION.md`.
- FOUND commits: `f67c29236`, `b51e9ca2e`, `e7281b212`.
