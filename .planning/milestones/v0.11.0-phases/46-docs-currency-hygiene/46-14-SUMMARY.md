---
phase: 46-docs-currency-hygiene
plan: 14
subsystem: docs-code-comments
tags: [rustdoc, consumer-surface-muster, switching-providers, curr-23, d-02, d-00f, f4, citations]

requires:
  - phase: 46-10
    provides: two-part MIGRATION.md (section 10 for v0.11.0 above the kept v0.10 record in section 9)
  - phase: 46-11
    provides: frozen section 10 headings 10.1 .. 10.8
  - phase: 46-12
    provides: the CHANGELOG and TOML half of the same recipe, and the loose citation shapes the recipe misses
  - phase: 46-06
    provides: the muster pins whose comments are retargeted here
provides:
  - "The consumer-surface muster's red-muster rule names the MIGRATION.md section 10.2 row and the section 10.8 library-consumer table (D-00f)"
  - "The two D-11 exhaustive-match pin comments name the section 10.2 row"
  - "The three seeded-registry small-output-cap comments cite MIGRATION section 10.8"
  - "switching-providers.md cites section 10.8 for DR-19 and DR-18"
  - "conclave_execution_service.rs cites 10.1 rows for the Phase 44 retries"
  - "One classification table (26 citation lines) for the .rs and book-page half of F4"
affects: [46-13]

tech-stack:
  added: []
  patterns:
    - "Citation retarget by line-addressed perl substitution, checked by a comment-only diff filter against the plan base"

key-files:
  created: []
  modified:
    - tests/consumer_surface_muster.rs
    - docs/src/user-guides/switching-providers.md
    - crates/paladin-llm/src/registry/seeded.rs
    - crates/paladin-battalion/src/conclave_execution_service.rs

key-decisions:
  - "The three #[non_exhaustive] blocks (PaladinError, LlmError, BattalionError) and the three PaladinError variant comments stay at 9.2: the marking rows and the Guardrail / StructuredOutput / ArmamentFailed extensions are in the v0.10.1 register, so Task 2 made no edit and no commit"
  - "The two forward-rule pin comments in the muster (FinishReason, ArsenalError) retarget to 10.2: a new break goes to the v0.11 register"

requirements-completed: []

duration: 15 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 14: Retarget `.rs` comment and book-page citations to section 10 Summary

Four comment lines in the consumer-surface muster, one battalion test comment, three seeded-registry comments and two `switching-providers.md` lines now point at `MIGRATION.md` section 10 where the cited row moved; every v0.10 citation stays as written and only comment and prose lines changed.

## Accomplishments

- **Tracer, Task 1 (commit `cdafa960`):** the muster header reads "its MIGRATION.md section 10.2 row and the / section 10.8 library-consumer table"; the `finish_reason_is_exhaustive` and `arsenal_error_is_exhaustive` comments read "a registered break with a MIGRATION.md section 10.2 row". `switching-providers.md` cites `section 10.8 DR-19` (line 137) and `10.8 DR-18` (line 439, a shape the plan's recipe does not print, see Deviations). The three `seeded.rs` comments (lines 635, 1299, 1392) read `MIGRATION section 10.8`. The tracer verify was re-run on the committed tree (auto-mode gate) and passed before expansion: "Tracer verified end-to-end -- expanding".
- **Task 2 (no commit):** every candidate in `battalion/mod.rs`, `paladin_error.rs`, `run.rs`, `waypoint.rs` and `llm_port.rs` classifies keep (rows below), so no edit and no commit was made, as the plan's fallback prescribes. "No retarget in Task 2."
- **Task 3 (commit `db2d598f`):** `conclave_execution_service.rs:745` reads "recorded as MIGRATION.md 10.1 rows" (the retries are M-B-09 and M-B-10, Phase 44 rows now in 10.1). The other five files in the task classify keep. The recipe re-run for this partition prints 16 lines, all keep-classified (listed below).

## Verification records

- Comment-only filter against the plan base (parent of `cdafa960`) across all thirteen `.rs` files: empty. `fault-tolerance.md` and `control-flow.md` do not differ from the plan base.
- Task 3 verify, all exit 0: `cargo check --offline --workspace --all-targets --all-features`, `make doc-check`, `make check-consumer-surface`, `make check-doc-config` (161 YAML blocks, 0 failed), `cargo test --offline --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard` (8 + 3 + 3 passed), `make api-surface` ("API surface unchanged", 4323 items), `cargo fmt --check`.
- `make security` not run: pre-existing `cargo-deny` advisory RUSTSEC-2026-0386 (tokio), unrelated; no dependency changed. mdbook is not installed, so no mdbook leg was run locally; the only book page edited is `switching-providers.md` (two prose citations, no fenced block touched; `make check-doc-config` covers its YAML blocks).
- Re-run of the recipe plus a loose sweep (`MIGRATION(.md)? +9.x`, line-end `MIGRATION.md` followed by a wrapped number, bare `9.x` near the words migration, row, register, section) over `.rs`, `.md`, `.yml`, `.sh`: no live v0.11 citation under 9.x remains outside the keep rows. Out-of-partition hits seen and left alone: `.github/workflows/ci.yml:279` (`§9.3`, v0.10 toolchain section, ci.yml is outside the recipe), `scripts/check-migration-allowlist.sh` and `tests/scripts/check-migration-allowlist_test.sh` (name both registers, owned by 46-10), `tests/integration/commander_integration_tests.rs` (unrelated test numbering).

## Classification table

Built once in Task 1 against `HEAD` `7107fc96` (plans 46-08 and 46-09 had already edited `battalion/mod.rs`, so lines are re-located; `battalion/mod.rs` moved from 782 to 800). "Source" is `recipe` when the plan's candidate recipe prints the line and `loose` when only the looser sweep does. v0.10 test: the cited row or `crate | type` key is in `git show v0.10.1:MIGRATION.md`. Totals: 26 lines (25 recipe, 1 loose); 10 retarget (9 + 1), 16 keep. A further 5 lines name a row id but no section number (last block below) and need no edit.

| File:line (HEAD) | Source | Citation | Decision | New target | Row / reason |
|---|---|---|---|---|---|
| `tests/consumer_surface_muster.rs:23` | recipe | `section 9.2` | retarget | `section 10.2` | red-muster rule; a forward rule naming where a new break is registered (D-00f) |
| `tests/consumer_surface_muster.rs:24` | recipe | `section 9.8` | retarget | `section 10.8` | the library-consumer table lives in 10.8 (46-10) |
| `tests/consumer_surface_muster.rs:580` | recipe | `section 9.2` | retarget | `section 10.2` | D-11 pin comment (`FinishReason`), forward rule |
| `tests/consumer_surface_muster.rs:592` | recipe | `section 9.2` | retarget | `section 10.2` | D-11 pin comment (`ArsenalError`), forward rule |
| `crates/paladin-llm/src/registry/seeded.rs:635` | recipe | `section 9.8` | retarget | `section 10.8` | Phase 50 empty-completion / small-output-cap text, now in 10.8 |
| `crates/paladin-llm/src/registry/seeded.rs:1299` | recipe | `section 9.8` | retarget | `section 10.8` | same |
| `crates/paladin-llm/src/registry/seeded.rs:1392` | recipe | `section 9.8` | retarget | `section 10.8` | same |
| `docs/src/user-guides/switching-providers.md:137` | recipe | `section 9.8` | retarget | `section 10.8` | DR-19 (Phase 50.1), a DR row, v0.11 |
| `docs/src/user-guides/switching-providers.md:439` | loose | `9.8` | retarget | `10.8` | DR-18 (`SpendScope`), a DR row, v0.11 |
| `crates/paladin-battalion/src/conclave_execution_service.rs:745` | recipe | `9.1 rows` | retarget | `10.1 rows` | M-B-09 / M-B-10, Phase 44 rows |
| `crates/paladin-core/src/platform/container/battalion/mod.rs:800` | recipe | `§9.2` | keep | - | `BattalionError` `#[non_exhaustive]` marking row is in the v0.10.1 register |
| `crates/paladin-core/src/platform/container/paladin_error.rs:24` | recipe | `§9.2` | keep | - | `PaladinError` marking row is in the v0.10.1 register |
| `crates/paladin-core/src/platform/container/paladin_error.rs:91` | recipe | `§9.2` | keep | - | `GuardrailTripped` (Phase 26, D-09): in the v0.10.1 `PaladinError` row |
| `crates/paladin-core/src/platform/container/paladin_error.rs:118` | recipe | `§9.2` | keep | - | `StructuredOutputInvalid` (Phase 26, D-26): in the v0.10.1 `PaladinError` row |
| `crates/paladin-core/src/platform/container/paladin_error.rs:145` | recipe | `§9.2` | keep | - | `ArmamentFailed` (Phase 26, D-33/D-34): in the v0.10.1 `PaladinError` row |
| `crates/paladin-ports/src/output/llm_port.rs:378` | recipe | `§9.2` | keep | - | `LlmError` marking row is in the v0.10.1 register |
| `crates/paladin-core/src/platform/container/run.rs:594` | recipe | `§9.6` | keep | - | names Phase 27 and "new-in-0.10" |
| `crates/paladin-core/src/platform/container/waypoint.rs:325` | recipe | `§9.4` | keep | - | names Plan 22-04 |
| `crates/paladin-battalion/src/engine/graph.rs:59` | recipe | `§9.2` | keep | - | "New in 0.10", type absent at `v0.9.0` (D-29/D-37) |
| `src/config/trace.rs:26` | recipe | `§9.5` | keep | - | `TraceConfig` D-36, Phase 28, in the v0.10.1 9.5 |
| `src/config/node_cache.rs:13` | recipe | `§9.5` | keep | - | names Phase 23 D-26 and Phase 25 D-29, in the v0.10.1 9.5 |
| `src/config/agent_runtime.rs:12` | recipe | `§9.5` | keep | - | X-09 "v0.9 config boots v0.10", in the v0.10.1 9.5 |
| `src/config/agent_runtime.rs:1786` | recipe | `§9.5` | keep | - | same obligation, Phase 26 test |
| `src/application/services/waypoint_retention.rs:107` | recipe | `§9.2` | keep | - | names plan 28-17 |
| `docs/src/user-guides/fault-tolerance.md:415` | recipe | `§9.1` | keep | - | M-B-04, a v0.10 row; read, not edited |
| `docs/src/user-guides/control-flow.md:249` | recipe | `§9.1` | keep | - | M-B-01, a v0.10 row; read, not edited |

Lines that name a row id but no section number (found by the loose sweep, correct as written, no edit): `battalion/mod.rs:63` (`M-B-06`), `battalion/legacy_keys.rs:7` (`M-B-06`), `src/config/engine.rs:89` (`M-B-02`, a v0.10 row), `llm_port.rs:377` and `paladin_error.rs:23` (the wrapped line before the `§9.2` line already listed).

Keep rows left in the recipe re-run (16): `trace.rs:26`, `node_cache.rs:13`, `agent_runtime.rs:12`, `agent_runtime.rs:1786`, `waypoint_retention.rs:107`, `llm_port.rs:378`, `paladin_error.rs:24, 91, 118, 145`, `battalion/mod.rs:800`, `waypoint.rs:325`, `run.rs:594`, `graph.rs:59`, `fault-tolerance.md:415`, `control-flow.md:249`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] The recipe misses `switching-providers.md:439` (`MIGRATION.md` 9.8 DR-18)**
- **Found during:** Task 1 (loose sweep, as the 46-12 findings advised).
- **Issue:** the line has a backtick between the file name and the number, so the recipe pattern `MIGRATION(\.md)? 9\.[0-9]` does not print it. DR-18 is a v0.11 row, so the citation was stale the moment 46-10 moved the DR table.
- **Fix:** classified it in the table (Source = loose) and retargeted it to `10.8 DR-18` with the DR-19 line. The plan's `files_modified` already lists the page.
- **Files modified:** `docs/src/user-guides/switching-providers.md`
- **Commit:** `cdafa960`

**Total deviations:** 1 auto-fixed (Rule 2). **Impact:** none on scope; one more prose citation number in a file the plan already owns.

## Issues Encountered

- None. Task 2 produced no commit by design (all keep rows). `requirements.mark-complete` was not run: CURR-23 stays Pending until 46-13 finishes (the `requirements.ready-ids` gate decides).

## Known Stubs

None.

## Threat Flags

None. T-46-29 (wrong citation target, v0.10 citation rewritten) mitigated: one classification table drove every edit, the keep rows are listed above, and the recipe re-run prints only keeps. T-46-30 (production line changed) mitigated: the comment-only diff filter against the plan base is empty for all thirteen `.rs` files, and workspace `cargo check`, the legacy guard and `make api-surface` are unchanged.

## Next Phase Readiness

Ready for 46-13 (`upgrading.md` and `migration-guide.md`, CHANGELOG `### Documentation` entries, STATE / ROADMAP dated updates). After 46-13, CURR-23's citation work is complete in `MIGRATION.md`, the CHANGELOGs, the TOML comments, the `.rs` comments and the book pages.

## Self-Check: PASSED

- Commits found: `cdafa960`, `db2d598f`.
- Files exist: the four modified files; the table covers every recipe line of the partition plus the loose line.
- Acceptance criteria re-run on the committed tree: muster header names sections 10.2 and 10.8; `seeded.rs` carries three `MIGRATION section 10.8` comments; `conclave_execution_service.rs` cites "10.1 rows"; comment-only filter empty; `fault-tolerance.md` and `control-flow.md` unchanged.
