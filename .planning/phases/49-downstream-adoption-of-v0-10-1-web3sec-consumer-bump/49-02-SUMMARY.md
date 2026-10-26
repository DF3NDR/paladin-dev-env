---
phase: 49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump
plan: 02
subsystem: docs
tags: [migration, changelog, consumer-upgrade, token-usage, api-stability]

requires:
  - phase: 49
    provides: "plan 49-01's tests/consumer_surface_muster.rs and make check-consumer-surface, which the note points at"
provides:
  - "MIGRATION.md subsection '### Library consumers: v0.10.1 → v0.11.0' at the end of section 9.8 (ADOPT-05)"
  - "The D-2 'no total-only TokenUsage constructor' pitfall with the lossless recommendation"
  - "Break table DR-1..DR-11, each row pointing at its existing section 9.2 / 9.1 entry"
  - "CHANGELOG [Unreleased] Added bullet announcing the muster and the note"
affects: [49-03, 49-04, 49-05, 49-06]

tech-stack:
  added: []
  patterns:
    - "Consumer-shaped index over existing register rows: a ### subsection after the last numbered section, rows labelled DR-n so check-migration-allowlist.sh never reads them"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "Placed the note as a ### subsection after section 9.8 step 7 rather than a new ## 9.9, keeping the 9.1-9.8 numbering and the last-heading invariant."
  - "Rows start with '| DR-n ' so the allowlist guard (backtick-led rows between ## 9.2 and ## 9.3 only) cannot see them."
  - "Followed section 9.2 wording over the research drift register wherever they differed (see Deviations, A2-49-02)."

patterns-established:
  - "Docs-only adoption note: append/insert only, zero deleted lines against the phase base, no register row, no allowlist entry, no deprecation"

requirements-completed: [ADOPT-05]

coverage:
  - id: D1
    description: "MIGRATION.md ends with the library-consumer subsection: scope, D-2 pitfall, DR-1..DR-11 in order, insulation advice, muster pointer"
    requirement: "ADOPT-05"
    verification:
      - kind: other
        ref: "row-label extraction prints DR-1,DR-2,...,DR-11; 'no total-only' present; grep -F for from_total, AddAssign, .sum::<TokenUsage>(), Entity<T>, RateLimitExceeded, rate_limited, LlmFailure, with_aegis, PaladinResult::new and tests/consumer_surface_muster.rs all hit"
        status: pass
    human_judgment: false
  - id: D2
    description: "No library API, register row or allowlist entry changed; MIGRATION.md and CHANGELOG.md are append/insert only"
    requirement: "ADOPT-05"
    verification:
      - kind: other
        ref: "git diff --numstat 91be2280 -- MIGRATION.md CHANGELOG.md deleted-lines column 0 and 0; section 9.2..9.3 span diff empty; allowlist git diff --exit-code 0; git diff --name-only 91be2280 -- crates src .cargo Cargo.lock empty"
        status: pass
    human_judgment: false
  - id: D3
    description: "CHANGELOG [Unreleased] Added opens with the Phase 49 bullet; gates and repository-scan guards stay green"
    requirement: "ADOPT-05"
    verification:
      - kind: integration
        ref: "make check-gates exit 0; cargo fmt --all -- --check exit 0; cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard: 8 + 3 + 3 passed"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-14
status: complete
---

# Phase 49 Plan 02: Library-consumer upgrade note Summary

**An appended `### Library consumers: v0.10.1 → v0.11.0` subsection in MIGRATION.md section 9.8 carrying the "no total-only `TokenUsage` constructor" pitfall and an eleven-row break table (DR-1..DR-11), plus a CHANGELOG `[Unreleased]` bullet announcing the muster and the note.**

## Performance

- **Duration:** about 25 minutes
- **Tasks:** 2 of 2
- **Files modified:** 2 (`MIGRATION.md`, `CHANGELOG.md`), none under `crates/` or `src/`

## Accomplishments

- MIGRATION.md gains 48 inserted lines after §9.8 step 7: a scope paragraph, the D-2 pitfall (both defects stated: invariant violation, and contributing 0 under `Add` / `AddAssign` / `Sum`), the lossless recommendation, the DR-1..DR-11 table, the insulation paragraph and the muster pointer.
- Every replacement name was confirmed against its section 9.2 row before writing; `PaladinResult::new`, `impl Default for PaladinResult`, and the saturating `Add` / `AddAssign` / `Sum` were also confirmed present in the `v0.10.1` tag.
- CHANGELOG.md gains 9 inserted lines: one bullet at the head of `[Unreleased]` / `### Added`; `[Unreleased]` still has its seven `### ` headings.

## Task Commits

1. **Task 1: Library-consumer note in MIGRATION.md section 9.8** - `d80ab609` (docs)
2. **Task 2: CHANGELOG [Unreleased] bullet and docs-plan gates** - `5b5014ab` (docs)

**Plan metadata:** recorded in the final `docs(49-02): complete library-consumer upgrade note plan` commit.

## Gate Results (exit codes)

Diff base for every assertion: `91be228043c67e1843ce793d11f9169652b2d99d` (also checked against the plan's grep-resolved base `e5d830d6`; identical results).

| Gate | Result |
|------|--------|
| Row labels in file order | `DR-1,DR-2,DR-3,DR-4,DR-5,DR-6,DR-7,DR-8,DR-9,DR-10,DR-11` |
| Deleted lines vs base, MIGRATION.md / CHANGELOG.md | 0 / 0 |
| `## 9.2` .. `## 9.3` span diff vs base | empty (byte-identical) |
| `git diff --exit-code BASE -- .cargo/semver-checks-allowlist.toml` | exit 0 |
| `git diff --name-only BASE -- crates src .cargo Cargo.lock` | empty |
| Last `## ` heading of MIGRATION.md | `## 9.8 Upgrade checklist`; `## 9.9` count 0 |
| Section 9.8 steps 1-7 still present | 7 |
| `[Unreleased]` `### ` heading count | 7 |
| `cargo fmt --all -- --check` | exit 0 |
| `make check-gates` (after each task) | exit 0 |
| `cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard` (after each task) | exit 0 (8 + 3 + 3 passed) |
| Vocabulary check on both new texts (fixture term, officer word, to-do/placeholder markers) | 0 matches |

Every cargo invocation ran with `CARGO_INCREMENTAL=0`; free disk stayed at about 7.1 GB; no ENOSPC.

## Decisions Made

- The subsection is a `###` after step 7 (not a new `## 9.9`), as the plan and research recommend.
- DR-6's replacement says to name the field (`cost: None`) and route mock responses through one helper, since `LlmResponse` has no `Default` and no constructor.

## Deviations from Plan

### Wording taken from section 9.2 over the research drift register (A2-49-02)

1. **DR-2 register anchor** gains `9.1 M-B-08`. The research register cites only the `NodeError` and `BattalionResult` rows; the section 9.2 `NodeError` / `BattalionResult` rows cite M-B-08 for the node-error summary change, and the plan's own M-B list omitted it. Added because it is the register's own anchor.
2. **DR-9 replacement** is worded from the section 9.2 `BattalionError` row: `BattalionError::Node` with a `Timeout` source for Formation and Campaign, or inside `AggregationError` for Phalanx (research said only "`BattalionError::Node` with a `Timeout` source"). Anchor `9.1 M-B-06` taken from that row.
3. **DR-4** cites `9.1 M-B-12` as the `ContentItem` row does; the research register gave no M-B id for it.
4. **DR-11** cites "the `StopReason` rows" (plural): `CallLimit` / `TokenBudget` and `AllowanceHalted(..)` are registered in separate section 9.2 rows (Phase 25-era row and Phase 42 row). The note does not name the framework officer that the `AllowanceHalted` row mentions, per the vocabulary guardrail.
5. **DR-8** names `Aegis { timeout, retry, on_error, .. }` as the `with_aegis` argument shape, from the section 9.2 `BattalionConfig` row.

### Auto-fixed Issues

None. Plan executed as written; no Rule 1-4 deviations.

## Authentication Gates

None.

## Known Stubs

None. Documentation only; no placeholder or open marker was written (the file's scope note forbids them).

## Threat Flags

None. No network endpoint, auth path, file access or schema surface introduced. T-49-07 (register/allowlist pairing) mitigated by the byte-identical section 9.2 span, the unchanged allowlist and a green `make check-gates`. T-49-08 (replacement names) mitigated by confirming each name against its section 9.2 row; plan 49-06 re-checks against the consumer inventory.

## Issues Encountered

- The plan's `BASE` grep resolves to `e5d830d6`; the orchestrator supplied `91be2280` as the phase base. Both were checked and agree.

## Next Phase Readiness

- Plan 49-03 (red proof) is unaffected by this docs-only change. Plan 49-04 records the phase-wide "no library API changed" evidence and `make api-surface`; plan 49-06 adds `DR-12`+ rows only if the consumer inventory surfaces an unlisted break (flagged assumption A1-49-02).

## Self-Check: PASSED

- FOUND: MIGRATION.md subsection heading (1 match), CHANGELOG.md Phase 49 bullet
- FOUND commits: d80ab609, 5b5014ab
