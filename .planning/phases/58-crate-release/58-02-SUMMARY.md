---
phase: 58-crate-release
plan: 02
subsystem: release-gate-tooling
tags: [shell-guard, review-dispositions, tdd, key-scrub, ship-10]

requires:
  - phase: 58-crate-release
    provides: plan 58-01 CI-health baseline
provides:
  - check-review-dispositions.sh that rejects near-miss tokens as closed-row evidence (57-REVIEW WR-02 fixed)
  - nine committed near-miss and guard fixtures that run in CI through make test-shell-guards
  - LIVE_API_TESTS_FIX.md with its key-shaped values redacted, proven absent by counts only
affects: [58-05, 58-14]

tech-stack:
  added: []
  patterns:
    - "Near-miss fixtures are committed failing first (RED), then the checker is tightened (GREEN)"
    - "A key scrub is proven by counts and shell variables, never by echoing the literal"

key-files:
  created:
    - tests/scripts/fixtures/check-review-dispositions/ (9 new fixture directories)
  modified:
    - scripts/check-review-dispositions.sh
    - tests/scripts/check-review-dispositions_test.sh
    - tests/scripts/fixtures/check-review-dispositions/ (one-line backtick edit in 15 existing registers)
    - .project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md

key-decisions:
  - "No register amendment: the tightened checker reads 0 failures on the real archive, so 57-REVIEW-DISPOSITIONS.md is unchanged"
  - "The key literal remains in git history since 928c6d5f9; rotation and any history purge are the maintainer's (D-08), handed to plan 58-05"

patterns-established:
  - "Backticked test names and single-slash backticked paths are the accepted pointer forms for test and path evidence"

requirements-completed: []
requirements-progress: "SHIP-10 gate tooling is in place; SHIP-10 itself is earned by plan 58-14 after the tag"

duration: not measured (continuation agent)
completed: 2026-11-05
status: complete

coverage:
  - id: D1
    description: "A closed row whose Evidence holds only a near-miss token fails CLOSED_NO_POINTER"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "RED after cb2021567: 9/34 assertion(s) failed; GREEN after 5d143727b: harness 34 assertion(s) passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "The tightened checker still reads 0 failures on the real archive and the shell-guard suite passes"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "real archive: checked 51 finding(s) across 13 review file(s); 0 failure(s); make test-shell-guards: 85 assertion(s) passed, exit 0; shellcheck clean"
        status: pass
    human_judgment: false
  - id: D3
    description: "The key-shaped values at LIVE_API_TESTS_FIX.md lines 49 and 53 are absent from the HEAD tree"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "d36268239: project-key-shape hits 0; 56-08 hits under .project 0; removed tokens of length >=5 present in HEAD tree 0 of 3"
        status: pass
    human_judgment: false
---

# Phase 58 Plan 02: Review-disposition evidence gate and key scrub Summary

**The disposition checker now demands real evidence for a closed review row (near-miss tokens such as a date, `ADR-0063` or `SHA-256` no longer count), and the key-shaped values at `LIVE_API_TESTS_FIX.md:49` and `:53` are redacted, proven absent by counts only.**

## Performance

- **Tasks:** 2 of 2 (Task 1 as RED then GREEN, Task 2 docs-only)
- **Completed:** 2026-11-05
- **Files:** one checker, one harness, nine new fixture directories, fifteen one-line fixture-register edits, one docs file

## Accomplishments

- 57-REVIEW WR-02 fixed as gate tooling: the commit, requirement, path and test-name pointer heuristics were tightened so loose tokens no longer satisfy a `closed` row.
- Near-miss evidence fixtures run in CI via `make test-shell-guards`, plus a guard fixture that keeps every genuine pointer form passing.
- The tracked tree no longer carries the key-shaped value; the proof never printed it.

## Task Commits

1. **Task 1 RED: near-miss evidence fixtures that must fire CLOSED_NO_POINTER** - `cb2021567` (`test(58-02)`)
2. **Task 1 GREEN: require real evidence for a closed review disposition** - `5d143727b` (`fix(58-02)`)
3. **Task 2: redact the key-shaped values at LIVE_API_TESTS_FIX.md lines 49 and 53** - `d36268239` (`docs(58-02)`)

## Recorded results

- **RED** (after `cb2021567`, before the checker change): `9/34 assertion(s) failed` - the 8 `closed_near_miss_*` assertions (`expected exit 1 ... (got 0)`) plus `closed_real_pointers`, which exited 1 where 0 was expected (the row `` `scripts/sdk-smoke` `` has one slash and no extension; the old checker rejected it, the new backtick rule accepts it, so the guard fixture also tests the widening).
- **GREEN:** harness `34 assertion(s) passed`; `make test-shell-guards` `85 assertion(s) passed`, exit 0; `shellcheck scripts/check-review-dispositions.sh` clean.
- **Real archive:** `checked 51 finding(s) across 13 review file(s); 0 failure(s)`. No register amendment; `57-REVIEW-DISPOSITIONS.md` is unchanged.

## Scrub proof (counts and file:line only)

Target: `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md:49` and `:53`. Line 49 now reads `OPENAI_API_KEY=sk-proj-REDACTED REDACTED` (the space example is kept); line 53 reads `OPENAI_API_KEY=sk-proj-REDACTED`. `git diff --stat HEAD~1`: one file, 2 insertions, 2 deletions. The commit message carries no key-shaped token.

| Proof | Count |
|-------|-------|
| (a) `sk-proj-` followed by 16+ key characters, tracked tree at HEAD | 0 lines |
| (b) `sk-` followed by 12+ key characters, not containing `REDACTED`, under `.project/` | 0 |
| (c) removed tokens of length 5+ (3 tokens, read from `HEAD~1` into a subshell) found in the HEAD tree | 0 of 3 |

Context only (never the matched text): tree-wide `sk-` + 12 or more key characters, lines not containing `REDACTED`, by top-level directory at HEAD: `.claude` 6, `.planning` 221, `MIGRATION.md` 1, `crates` 49, `docs` 1, `examples` 3, `scripts` 3, `src` 45, `tests` 6, `.project` 0. These are the test fakes, documentation placeholders and planning-text patterns that predate the phase; the count is the `sk-` shape alone, a superset of the 56-08 rule, and is not a finding.

## Deviations from Plan

**1. [Rule 3 - Blocking] Bare test name backticked in fixtures.** The bare name `first_warning_stays_fixed_test` in the `ok` fixture and 14 copies would fail under the tightened test-name rule, which breaks the existing `ok`/`scoped` passing fixtures. The one-line backtick edit was made in the RED commit `cb2021567`. Files: 15 fixture registers.

**2. [Observation] RED showed 9 failures, not the 8 the plan expected.** The ninth is the guard fixture `closed_real_pointers`, explained above: it also tests the widening of the backtick path rule. No action needed.

Task 2 executed exactly as written.

## Hand-offs

- The key literal remains in git history since `928c6d5f9` (T-58-24, transferred). Whether the key was ever live, its rotation and any history purge are the maintainer's call (D-08, D-00b); to be put to the maintainer in plan 58-05.
- SHIP-10 is not ticked here; plan 58-14 earns it after the tag.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- `cb2021567`, `5d143727b`, `d36268239` present in `git log`.
- `scripts/check-review-dispositions.sh` and the edited `LIVE_API_TESTS_FIX.md` exist.
