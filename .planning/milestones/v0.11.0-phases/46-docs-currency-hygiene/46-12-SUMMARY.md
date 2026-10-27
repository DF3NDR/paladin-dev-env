---
phase: 46-docs-currency-hygiene
plan: 12
subsystem: docs-release-notes
tags: [changelog, semver-allowlist, cargo-toml, curr-23, d-02, f4, citations]

requires:
  - phase: 46-10
    provides: two-part MIGRATION.md (section 10 for v0.11.0 above the kept v0.10 record in section 9) and the widened register window
  - phase: 46-11
    provides: frozen section 10 headings and anchors (10.1 .. 10.8)
provides:
  - "Root CHANGELOG [Unreleased] (the v0.11.0 release note) cites MIGRATION.md section 10 for every v0.11 row"
  - "The battalion, core, llm, ports, web and herald [Unreleased] sections cite section 10"
  - "Allowlist header naming section 9.2 and section 10.2 as the register the gate mirrors; Phase 38-51 manifest and allowlist comment blocks cite section 10.2"
  - "One classification table (188 citation lines) for the CHANGELOG and TOML half of F4"
affects: [46-13, 46-14]

tech-stack:
  added: []
  patterns:
    - "Citation retarget by line-range-limited perl substitution, checked by word-diff (only the number tokens changed) and by a base-vs-working-tree history diff"

key-files:
  created: []
  modified:
    - CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - .cargo/semver-checks-allowlist.toml
    - Cargo.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-battalion/Cargo.toml

key-decisions:
  - "Every 9.x citation in any [Unreleased] section names a v0.11 row (M-B-05 onward, DR rows, Phase 38-51 register rows), so none is a keep row; the keep rows are all history or Phase 22-33 manifest blocks"
  - "The crate-changelog rows the recipe cannot match (the form MIGRATION.md 9.1 M-B-NN, with a backtick or line break between the file name and the number) are classified and retargeted too, because the plan's acceptance criterion is about every 9.x citation, not only the recipe's"
  - "Five allowlist lines name both registers (9.2 and 10.2) instead of swapping, because the gate reads both"

patterns-established:
  - "A classification table built once (against the plan base) and applied, never re-derived"

requirements-completed: [CURR-23]

duration: 9 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 12: Retarget CHANGELOG and TOML citations to section 10 Summary

Every live `CHANGELOG.md` `[Unreleased]` citation and every Phase 38-51 TOML comment that pointed at a v0.11 row under its old 9.x number now points at section 10 of `MIGRATION.md`; history sections and all non-comment TOML lines are unchanged.

## Accomplishments

- **Tracer (commit `dcaf7556`):** the root `[Unreleased]` section, 98 citations on 76 lines retargeted (`§9.2` to `§10.2`, `section 9.8` to `section 10.8`, `MIGRATION.md 9.1 M-B-NN` to `10.1`, "the Phase 44 9.2 rows" to "10.2 rows", and so on). The section now carries 49 matches of the plan's `§10.x` / `section 10.x` / `#10N-` pattern (acceptance floor 30), plus the bare `10.x` forms. From `## [0.10.1]` down the file is byte-identical to the plan base. The tracer verify was re-run on the committed tree (auto-mode gate) and passed before expansion started: "Tracer verified end-to-end -- expanding".
- **Task 2 (commit `03250719`):** the `[Unreleased]` sections of the battalion (4 lines), core (9), llm (4), ports (9) and web (3) changelogs, plus herald (1, see Deviations). The five listed crates carry 20 section-10 citations (floor 15). Every heading from the first released version down is byte-identical to the plan base in all six files; `make check-changelogs` exits 0.
- **Task 3 (commit `0bcbd33d`):** comment lines only. Allowlist header names "§9.2 and §10.2" and says the gate reads both (the schema line and the `D-18` register-only comment follow); the Phase 44, 48 and 50 block comments and the `LlmError` row note say "section 10.2". `Cargo.toml` semver blocks for Phase 38-51 (root PACE-02, PARITY-06, REASON-01; core LEGACY-01..04, ENTITY-01..05, both REASON blocks; ports PRICE-03, PACE-01, the Phase 50 `LlmError` note; llm PARITY-03, PARITY-06, SURFACE-02, REASON-02 and the type-table sentence; battalion LEGACY-01) cite section 10.2. Phase 22-33 blocks and the root `§9.3` notes keep 9.x.
- Plan-level checks, all on the committed tree: `make clean-code` exit 0, `make api-surface` exit 0 ("API surface unchanged", 4323 items), `make check-gates` exit 0, `make check-migration-allowlist` exit 0, `cargo metadata --offline --no-deps` parses, `cargo test --offline --test legacy_removal_guard` 3 passed. mdbook is not installed and no mdbook page was touched.

## Verification records

- Value-line diff against the plan base for the allowlist and the six `Cargo.toml` files (`grep -E '^[-+][[:space:]]*[^-+#[:space:]]'`) is empty: no `migration_row`, `crate`, `lint`, `requirement_id`, `justification` or manifest value changed.
- `git log -F --grep='(46-12):' --name-only --format=` lists no `.rs` file.
- Word-diff of the root CHANGELOG change shows only the number tokens differ (`9.x` to `10.x`, `§9.x` to `§10.x`); no other word moved.
- Re-run of the candidate recipe filtered to this partition prints 55 lines, all keep-classified: the CHANGELOG history lines, the 17 `Cargo.toml` Phase 22-33 / `§9.3` keep lines, allowlist lines 24 and 26 (the Phase 22 "inherited from the program overview" sentence), and the five allowlist lines that deliberately name both registers (now 3, 8, 9, 10 and 32 after the header grew one line).
- The loose sweep `(^|[^0-9.])9\.[1-8]([^0-9]|$)` over every `[Unreleased]` section now returns 0 lines in all seven files.

## Classification table

Built once against the plan base (`e3828419`, parent of the first `(46-12):` commit) and applied by Tasks 1 to 3. "Source" is `recipe` when the plan's candidate recipe prints the line and `loose` when only the looser sweep does (see Deviations). Line numbers are base line numbers. A CHANGELOG row is `keep` when it sits at or below the first released heading; every other row cites a v0.11 row (M-B-05 onward, DR rows, Phase 38-51 register rows) and is `retarget`. Totals: 188 lines (144 recipe, 44 loose); 132 retarget (89 + 43), 5 name-both-registers, 51 keep (50 + 1).

| File:line | Source | Citation | Decision | New target | Row / reason |
|---|---|---|---|---|---|
| `CHANGELOG.md:24` | loose | `9.2, 9.8` | retarget | `10.2, 10.8` | rows DR-21 |
| `CHANGELOG.md:38` | loose | `9.2, 9.5` | retarget | `10.2, 10.5` | Phase 51 rows |
| `CHANGELOG.md:39` | loose | `9.8` | retarget | `10.8` | rows DR-20/DR-26 |
| `CHANGELOG.md:167` | recipe | `9.8` | retarget | `10.8` | Phase 50.1 rows |
| `CHANGELOG.md:178` | recipe | `9.8` | retarget | `10.8` | rows DR-1/DR-2/DR-8/DR-9 |
| `CHANGELOG.md:192` | recipe | `9.8` | retarget | `10.8` | Phase 50.1 rows |
| `CHANGELOG.md:299` | recipe | `9.8` | retarget | `10.8` | Phase 49 rows |
| `CHANGELOG.md:301` | recipe | `§9.2` | retarget | `§10.2` | Phase 49 rows |
| `CHANGELOG.md:432` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:456` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:477` | recipe | `§9.2, §9.4` | retarget | `§10.2, §10.4` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:488` | recipe | `§9.6` | retarget | `§10.6` | Phase 41 rows |
| `CHANGELOG.md:502` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:519` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:536` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:553` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:560` | recipe | `§9.2` | retarget | `§10.2` | Phase 41 rows |
| `CHANGELOG.md:575` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | Phase 41 rows |
| `CHANGELOG.md:660` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:681` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:697` | recipe | `§9.2` | retarget | `§10.2` | Phase 38 rows |
| `CHANGELOG.md:716` | recipe | `§9.2` | retarget | `§10.2` | Phase 38 rows |
| `CHANGELOG.md:793` | recipe | `§9.2, §9.5, §9.8` | retarget | `§10.2, §10.5, §10.8` | Phase 40 rows |
| `CHANGELOG.md:923` | loose | `9.1` | retarget | `10.1` | rows M-B-16 |
| `CHANGELOG.md:944` | loose | `9.5` | retarget | `10.5` | Phase 50 rows |
| `CHANGELOG.md:1002` | loose | `9.2, 9.8` | retarget | `10.2, 10.8` | rows DR-21 |
| `CHANGELOG.md:1062` | loose | `9.2` | retarget | `10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1077` | loose | `9.2` | retarget | `10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1083` | loose | `9.2` | retarget | `10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1092` | loose | `9.2` | retarget | `10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1098` | loose | `9.8` | retarget | `10.8` | Phase 50.1 rows |
| `CHANGELOG.md:1106` | loose | `9.1` | retarget | `10.1` | rows M-B-20 |
| `CHANGELOG.md:1131` | loose | `9.1` | retarget | `10.1` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1132` | loose | `9.5` | retarget | `10.5` | rows M-B-15 |
| `CHANGELOG.md:1193` | loose | `9.1` | retarget | `10.1` | rows DR-0061/M-B-13 |
| `CHANGELOG.md:1211` | loose | `9.2` | retarget | `10.2` | Phase 48 rows |
| `CHANGELOG.md:1227` | loose | `9.1, 9.4` | retarget | `10.1, 10.4` | rows DR-0060/M-B-12 |
| `CHANGELOG.md:1239` | recipe | `9.2` | retarget | `10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1247` | recipe | `9.1` | retarget | `10.1` | Phase 43 rows |
| `CHANGELOG.md:1252` | recipe | `9.1` | retarget | `10.1` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1268` | recipe | `§9.6` | retarget | `§10.6` | Phase 42 rows |
| `CHANGELOG.md:1278` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | Phase 42 rows |
| `CHANGELOG.md:1285` | recipe | `§9.2` | retarget | `§10.2` | Phase 41 rows |
| `CHANGELOG.md:1296` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1299` | recipe | `§9.2` | retarget | `§10.2` | Phase 38 rows |
| `CHANGELOG.md:1315` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1326` | recipe | `§9.6` | retarget | `§10.6` | Phase 40 rows |
| `CHANGELOG.md:1359` | recipe | `9.1` | retarget | `10.1` | rows DR-0059/M-B-06..M-B-11 |
| `CHANGELOG.md:1360` | recipe | `9.2` | retarget | `10.2` | Phase 44 rows |
| `CHANGELOG.md:1369` | recipe | `9.1, 9.4, 9.2` | retarget | `10.1, 10.4, 10.2` | rows M-B-08 |
| `CHANGELOG.md:1377` | recipe | `9.1, 9.2` | retarget | `10.1, 10.2` | rows M-B-09/M-B-10 |
| `CHANGELOG.md:1385` | recipe | `9.1, 9.2` | retarget | `10.1, 10.2` | rows DR-0059/M-B-06/M-B-09/M-B-11 |
| `CHANGELOG.md:1387` | loose | `9.1` | retarget | `10.1` | rows M-B-16 |
| `CHANGELOG.md:1398` | loose | `9.1` | retarget | `10.1` | rows M-B-17 |
| `CHANGELOG.md:1424` | recipe | `9.1` | retarget | `10.1` | rows DR-0059/M-B-06..M-B-11 |
| `CHANGELOG.md:1425` | recipe | `9.2` | retarget | `10.2` | Phase 44 rows |
| `CHANGELOG.md:1449` | recipe | `9.1, 9.2` | retarget | `10.1, 10.2` | rows DR-0060/M-B-12 |
| `CHANGELOG.md:1497` | loose | `9.1` | retarget | `10.1` | rows M-B-21 |
| `CHANGELOG.md:1524` | loose | `9.1` | retarget | `10.1` | rows M-B-20 |
| `CHANGELOG.md:1535` | loose | `9.1` | retarget | `10.1` | rows M-B-19 |
| `CHANGELOG.md:1550` | loose | `9.1` | retarget | `10.1` | rows M-B-18 |
| `CHANGELOG.md:1684` | recipe | `9.2, 9.8` | retarget | `10.2, 10.8` | rows DR-19/DR-12 |
| `CHANGELOG.md:1690` | recipe | `§9.2` | retarget | `§10.2` | Phase 40 rows |
| `CHANGELOG.md:1693` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1699` | recipe | `§9.5, §9.8` | retarget | `§10.5, §10.8` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1703` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1706` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1711` | recipe | `§9.6` | retarget | `§10.6` | v0.11 rows (Phase 38-51) |
| `CHANGELOG.md:1717` | loose | `9.1, 9.2, 9.4` | retarget | `10.1, 10.2, 10.4` | rows M-B-12 |
| `CHANGELOG.md:1736` | loose | `9.2` | retarget | `10.2` | Phase 50 rows |
| `CHANGELOG.md:1737` | loose | `9.8` | retarget | `10.8` | rows DR-6 |
| `CHANGELOG.md:1742` | loose | `9.2` | retarget | `10.2` | Phase 50 rows |
| `CHANGELOG.md:1757` | loose | `9.2` | retarget | `10.2` | Phase 50 rows |
| `CHANGELOG.md:1771` | loose | `9.2` | retarget | `10.2` | Phase 50 rows |
| `CHANGELOG.md:1772` | loose | `9.8` | retarget | `10.8` | rows DR-17 |
| `CHANGELOG.md:1776` | loose | `9.1` | retarget | `10.1` | rows DR-0061/M-B-13/M-B-22 |
| `CHANGELOG.md:1884` | recipe | `§9.1, #91-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1886` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1888` | recipe | `§9.1` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1936` | recipe | `§9.3, #93-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1946` | recipe | `§9.1, #91-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1975` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:1992` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2007` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2023` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2145` | recipe | `§9.1, #91-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2157` | recipe | `§9.6, #96-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2219` | recipe | `§9.1, #91-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2224` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2251` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:2289` | recipe | `§9.2, #92-` | keep | - | history (below the first released heading) |
| `CHANGELOG.md:3694` | loose | `9.1, 9.7` | keep | - | history (below the first released heading) |
| `crates/paladin-battalion/CHANGELOG.md:46` | recipe | `§9.2` | retarget | `§10.2` | Phase 42 rows |
| `crates/paladin-battalion/CHANGELOG.md:103` | loose | `9.1` | retarget | `10.1` | Phase 44 rows |
| `crates/paladin-battalion/CHANGELOG.md:110` | loose | `9.1` | retarget | `10.1` | Phase 44 rows |
| `crates/paladin-battalion/CHANGELOG.md:115` | loose | `9.2` | retarget | `10.2` | rows DR-0059 |
| `crates/paladin-battalion/CHANGELOG.md:154` | recipe | `§9.2` | keep | - | history (below the first released heading) |
| `crates/paladin-core/CHANGELOG.md:51` | recipe | `§9.2` | retarget | `§10.2` | Phase 42 rows |
| `crates/paladin-core/CHANGELOG.md:61` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-core/CHANGELOG.md:68` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-core/CHANGELOG.md:126` | loose | `9.1, 9.4` | retarget | `10.1, 10.4` | rows M-B-12 |
| `crates/paladin-core/CHANGELOG.md:135` | loose | `9.1` | retarget | `10.1` | rows M-B-06 |
| `crates/paladin-core/CHANGELOG.md:136` | recipe | `9.2` | retarget | `10.2` | rows M-B-07 |
| `crates/paladin-core/CHANGELOG.md:141` | loose | `9.1, 9.4` | retarget | `10.1, 10.4` | rows M-B-08 |
| `crates/paladin-core/CHANGELOG.md:146` | loose | `9.2` | retarget | `10.2` | Phase 44 rows |
| `crates/paladin-core/CHANGELOG.md:154` | loose | `9.2` | retarget | `10.2` | rows DR-0060 |
| `crates/paladin-core/CHANGELOG.md:199` | recipe | `§9.2` | keep | - | history (below the first released heading) |
| `crates/paladin-llm/CHANGELOG.md:171` | loose | `9.1` | retarget | `10.1` | rows M-B-22 |
| `crates/paladin-llm/CHANGELOG.md:269` | loose | `9.1` | retarget | `10.1` | rows M-B-22 |
| `crates/paladin-llm/CHANGELOG.md:319` | recipe | `9.2, 9.8` | retarget | `10.2, 10.8` | rows DR-19 |
| `crates/paladin-llm/CHANGELOG.md:325` | recipe | `9.2` | retarget | `10.2` | Phase 50 rows |
| `crates/paladin-ports/CHANGELOG.md:91` | recipe | `9.2` | retarget | `10.2` | Phase 43 rows |
| `crates/paladin-ports/CHANGELOG.md:108` | recipe | `§9.2` | retarget | `§10.2` | Phase 42 rows |
| `crates/paladin-ports/CHANGELOG.md:114` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-ports/CHANGELOG.md:118` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-ports/CHANGELOG.md:129` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-ports/CHANGELOG.md:142` | recipe | `9.2` | retarget | `10.2` | Phase 43 rows |
| `crates/paladin-ports/CHANGELOG.md:151` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-ports/CHANGELOG.md:162` | recipe | `9.2` | retarget | `10.2` | Phase 50 rows |
| `crates/paladin-ports/CHANGELOG.md:171` | recipe | `9.2` | retarget | `10.2` | Phase 50 rows |
| `crates/paladin-ports/CHANGELOG.md:212` | recipe | `§9.2` | keep | - | history (below the first released heading) |
| `crates/paladin-web/CHANGELOG.md:19` | recipe | `§9.2, §9.6` | retarget | `§10.2, §10.6` | Phase 42 rows |
| `crates/paladin-web/CHANGELOG.md:34` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-web/CHANGELOG.md:50` | recipe | `§9.2` | retarget | `§10.2` | v0.11 rows (Phase 38-51) |
| `crates/paladin-web/CHANGELOG.md:84` | recipe | `§9.5` | keep | - | history (below the first released heading) |
| `crates/paladin-web/CHANGELOG.md:85` | recipe | `§9.6` | keep | - | history (below the first released heading) |
| `crates/paladin-web/CHANGELOG.md:93` | recipe | `§9.2` | keep | - | history (below the first released heading) |
| `crates/paladin-content/CHANGELOG.md:20` | recipe | `§9.3` | keep | - | history (below the first released heading) |
| `crates/paladin-herald/CHANGELOG.md:33` | loose | `9.1` | retarget | `10.1` | rows DR-0059/M-B-08 |
| `crates/paladin-herald/CHANGELOG.md:51` | recipe | `§9.3` | keep | - | history (below the first released heading) |
| `crates/paladin-memory/CHANGELOG.md:36` | recipe | `§9.4` | keep | - | history (below the first released heading) |
| `crates/paladin-notifications/CHANGELOG.md:20` | recipe | `§9.3` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:125` | recipe | `§9.4` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:126` | recipe | `§9.5` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:130` | recipe | `§9.5` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:134` | recipe | `§9.5` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:137` | recipe | `§9.5` | keep | - | history (below the first released heading) |
| `crates/paladin-storage/CHANGELOG.md:144` | recipe | `§9.4, §9.5` | keep | - | history (below the first released heading) |
| `.cargo/semver-checks-allowlist.toml:3` | recipe | `§9.2` | retarget (name both registers) | `§9.2 and §10.2` | gate reads both registers (header / schema) |
| `.cargo/semver-checks-allowlist.toml:7` | recipe | `§9.2` | retarget (name both registers) | `§9.2 and §10.2` | gate reads both registers (header / schema) |
| `.cargo/semver-checks-allowlist.toml:8` | recipe | `§9.2` | retarget (name both registers) | `§9.2 or §10.2` | gate reads both registers (header / schema) |
| `.cargo/semver-checks-allowlist.toml:9` | recipe | `§9.2` | retarget (name both registers) | `§9.2 or §10.2` | gate reads both registers (header / schema) |
| `.cargo/semver-checks-allowlist.toml:23` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `.cargo/semver-checks-allowlist.toml:25` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `.cargo/semver-checks-allowlist.toml:31` | recipe | `§9.2` | retarget (name both registers) | `§9.2 or §10.2` | gate reads both registers (header / schema) |
| `.cargo/semver-checks-allowlist.toml:202` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:311` | loose | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:330` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:335` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:443` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:449` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `.cargo/semver-checks-allowlist.toml:545` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `Cargo.toml:16` | recipe | `§9.3` | keep | - | Phase 22-33 / v0.10 row or section |
| `Cargo.toml:53` | recipe | `§9.3` | keep | - | Phase 22-33 / v0.10 row or section |
| `Cargo.toml:84` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `Cargo.toml:94` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `Cargo.toml:97` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `Cargo.toml:104` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `Cargo.toml:113` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-core/Cargo.toml:42` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-core/Cargo.toml:48` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-core/Cargo.toml:66` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-core/Cargo.toml:87` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-core/Cargo.toml:91` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-core/Cargo.toml:106` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-core/Cargo.toml:118` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-core/Cargo.toml:127` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-ports/Cargo.toml:29` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-ports/Cargo.toml:34` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-ports/Cargo.toml:38` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-ports/Cargo.toml:43` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-ports/Cargo.toml:46` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-ports/Cargo.toml:58` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-ports/Cargo.toml:64` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-ports/Cargo.toml:67` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-ports/Cargo.toml:75` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-ports/Cargo.toml:86` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |
| `crates/paladin-llm/Cargo.toml:90` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-llm/Cargo.toml:102` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-llm/Cargo.toml:106` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-llm/Cargo.toml:118` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-llm/Cargo.toml:137` | recipe | `§9.2` | retarget | `§10.2` | Phase 38-51 suppression |
| `crates/paladin-web/Cargo.toml:63` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-web/Cargo.toml:70` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-web/Cargo.toml:78` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-web/Cargo.toml:80` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-web/Cargo.toml:89` | recipe | `§9.2` | keep | - | Phase 22-33 / v0.10 row or section |
| `crates/paladin-battalion/Cargo.toml:103` | recipe | `9.2` | retarget | `10.2` | Phase 38-51 suppression |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] The candidate recipe misses 44 citation lines, 43 of them retarget rows**
- **Found during:** Task 1 (classification).
- **Issue:** the recipe pattern needs `MIGRATION.md` followed by a space and the number (or `§`/`section`), so `` `MIGRATION.md` 9.1 M-B-16 ``, a bare `9.2 and 9.8 DR-21`, `9.5).**` and number-on-the-next-line forms are not printed. The acceptance criterion (every 9.x citation left in `[Unreleased]` is a keep row) and the truth ("every live citation ... retargeted") cover them. 31 such lines are in the root `[Unreleased]`, 11 in the crate changelogs, 1 in the allowlist, 1 in history (keep).
- **Fix:** classified them in the same table (Source = loose) and retargeted them with the rest.
- **Commits:** `dcaf7556`, `03250719`, `0bcbd33d`.

**2. [Rule 2 - Consistency] `crates/paladin-herald/CHANGELOG.md` is edited although it is not in `files_modified`**
- **Found during:** Task 2.
- **Issue:** the plan says the other crate changelogs carry no `[Unreleased]` citation. The herald `[Unreleased]` section (lines 8 to 40) holds one: "Phase 44 (LEGACY-01; ADR-0059; see `MIGRATION.md` 9.1 M-B-08)". M-B-08 is a v0.11 row (v0.10.1 has M-B-01 to M-B-04 only), so the citation was stale the moment 46-10 moved the row. It is not a recipe line (backtick before the number), which is why the plan missed it. Plan 46-14 owns only non-CHANGELOG, non-TOML lines, so nobody else would have fixed it.
- **Fix:** retargeted to `10.1 M-B-08`. The Task 2 verify's closing "no other crate changelog differs" check was therefore run with herald in the allowed set; every other crate changelog (content, memory, notifications, storage) is unchanged, and herald's history is byte-identical.
- **Commit:** `03250719`.

**3. [Record] The Task 2 commit message miscounts the loose lines**
- The body says "19 recipe lines plus 20 more"; the true figures are 19 recipe lines and 11 loose lines (10 in the five listed changelogs plus the herald line), 30 lines in all. Commits cannot be amended in this run; the table above is authoritative.

**Total deviations:** 2 auto-fixed (Rule 2), 1 record correction. **Impact:** none on scope; every edit is a citation number in an `[Unreleased]` section or a comment line.

## Issues Encountered

- None blocking. The background run of `make clean-code` and `make api-surface` reported exit 1 only because its trailing `tail -3 a b` was a usage error; the two make logs both record exit 0.
- `make security` was not run (pre-existing `cargo-deny` advisory RUSTSEC-2026-0386 on tokio, unrelated to this plan; no dependency or `.rs` file changed).

## Known Stubs

None.

## Threat Flags

None. T-46-24 (wrong citation target or history rewritten) mitigated: one classification table drove every edit, and the history diff against the plan base is empty for the root and all six crate changelogs. T-46-25 (allowlist or manifest value edited) mitigated: the value-line diff is empty, `cargo metadata` parses, `make check-migration-allowlist` and `make check-gates` exit 0.

## Next Phase Readiness

Ready for 46-13 (the `upgrading.md` and `migration-guide.md` rewrites, CHANGELOG `### Documentation` entries, STATE / ROADMAP dated updates) and 46-14 (the `.rs` comment, muster-header and book-page half of the same recipe). 46-13 will add `### Documentation` entries to the root and crate `[Unreleased]` sections; those entries should cite section 10 from the start. CURR-23 stays Pending in `REQUIREMENTS.md` until 46-13 and 46-14 finish (the `requirements.ready-ids` gate decides).

## Self-Check: PASSED

- Commits found: `dcaf7556`, `03250719`, `0bcbd33d`.
- Files exist: the 13 listed files plus `crates/paladin-herald/CHANGELOG.md`.
- Acceptance criteria re-run on the committed tree: root and six crate changelog history byte-identical to the plan base; TOML value-line diff empty; no `.rs` path in this plan's commits; 188 table rows (every recipe line of the partition plus the loose lines); 49 root section-10 citations (floor 30) and 20 in the five listed crate changelogs (floor 15); allowlist header names "§9.2 and §10.2".
