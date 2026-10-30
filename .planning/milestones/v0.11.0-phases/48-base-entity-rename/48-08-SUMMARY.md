---
phase: 48-base-entity-rename
plan: 08
subsystem: repository-guard
tags: [guard, repository-scan, positive-controls, graph-freeze, d-07, adr-0060]

requires:
  - phase: 48-04
    provides: "Entity<T> in base/entity/wrapper.rs, single Paladin alias, citadel placeholders removed"
  - phase: 48-05
    provides: "Entity.data and ContentItem/PromptItem.entity with read-aliases; the 18 graph-sense .node lines byte-identical"
  - phase: 48-06
    provides: "documentation agrees with the new names"
  - phase: 48-07
    provides: "register and manifest allow-line prose that names removed items only in words"
provides:
  - "tests/entity_rename_guard.rs: five-rule repository scan in the tests/legacy_removal_guard.rs shape, std only"
  - "Tests scanner_reports_planted_entity_rename_violations, scanner_reports_missing_scoped_targets, scanner_accepts_a_clean_tree, entity_surfaces_are_renamed_in_the_tree"
  - "Byte-identical freeze check of the 11 frozen graph files and the schema against the phase base"
affects: [48-09]

tech-stack:
  added: []
  patterns:
    - "Scoped-rule table (GRAPH_DECLARATIONS, SINGLE_DECLARATIONS) with a clean-content generator reused by every positive control"
    - "Expected-report arithmetic for removal controls: one report per rule the file is a target of, plus one per rule-4 declaration homed there"

key-files:
  created:
    - tests/entity_rename_guard.rs
  modified: []

key-decisions:
  - "Skip set, HISTORY_FILES and SCANNED_EXTENSIONS copied verbatim from the Phase 44 guard; nothing widened, no look-alike allowance added"
  - "Rule 5 pins the node_cache config definition sites (src/config/node_cache.rs, src/config/mod.rs), not a YAML section, because config.example.yml has none (A2-48-08)"
  - "The real-tree run found zero hits, so no source file was touched"

patterns-established:
  - "A scoped target with no production lines is reported as proves-nothing, closing the emptied-file hole the Phase 44 shape leaves open"

requirements-completed: [ENTITY-05, ENTITY-02, ENTITY-03]

duration: about 25 min
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 08: Entity rename guard Summary

**`tests/entity_rename_guard.rs` makes the Phase 48 rename and the graph freeze mechanical: five rules over the repository (old paths, old identifiers, entity-sense fields in the wrapper/container/version files, exact single declarations, graph definitions still in their files), three positive controls that prove each rule can fail, and a real-tree test that is green with nothing added to the skip set.**

## Accomplishments

- Copied the legacy guard's skeleton (walker, `occurs` boundary matcher, `production_lines`, `scan_scoped`, `ScratchTree`) and replaced only the rule tables, the target lists, `GUARD_FILE` and the scratch prefix (`paladin_entity_rename_guard_`). Every forbidden string is assembled with `concat()` (13 sites); the file holds none whole.
- Rules implemented as specified: (1) old wrapper module path, old version-module path and their slash forms; (2) the old wrapper name before `<`, `::new`, `::<` with a non-identifier char before it, the old version-family prefix with no boundary, the old per-vertex version cap; (3) `.node` / `node:` in `wrapper.rs` and the eight container files, and the old not-found variant plus `node_id:`, `node_data:`, `.node_id`, `.node_data` in `entity_version_service.rs`; (4) exact single declarations of `pub type Paladin =`, `pub struct PaladinData`, `pub enum PaladinStatus`, `pub struct NodeError`, `pub struct Entity<`; (5) 12 graph files (11 Rust/TOML sources plus `wargraph-doc.schema.json` by explicit path) still holding their declarations.
- RED first: against an empty `scan` stub the planted and missing-target controls failed (the clean-tree control passed vacuously, as expected); then the scanner went in and all three went green.
- Retained base items are not banned: the clean-tree control contains `base::entity::field`, `base::entity::collection`, `base::component::element`, both retained services, `get_node_field_versions`, `Field.nid` and `FieldVersion.node_id`, and passes.

## Task Commits

1. **Task 1: scanner and positive controls** - `5882953` (test)
2. **Task 2: real-tree test and freeze proof** - `2bbd14f` (test)

## Verification

- `cargo test --test entity_rename_guard`: 4 passed, 0 failed (three `scanner_*` controls plus `entity_surfaces_are_renamed_in_the_tree`).
- `cargo test --test legacy_removal_guard`: 3 passed; `git log --oneline -- tests/legacy_removal_guard.rs | grep -c '(48-'` prints 0 (untouched). The legacy guard also scans the new file and finds nothing.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- Task 1 acceptance greps: no forbidden string whole in the guard; `concat()` 13; the three constants present; `"tests/entity_rename_guard.rs"` once; schema and `node_cache.rs` named; no `unwrap`/`expect` before the first `#[test]`; only `std::` imports; `tests/legacy_removal_guard.rs` and `Cargo.toml` unchanged.
- Skip-set check: the `skip_directory`, `skip_file` and `HISTORY_FILES` blocks are identical between Task 1's commit and the working tree at Task 2.
- Graph freeze, byte for byte: `BASE=f0535858594006707df3827ef83719465a00d00e` (`docs(48-02): record ADR-0060 ...`, the first commit whose subject contains `(48-0`; 48-02 landed before 48-01 in history). `git diff --exit-code "$BASE~1" -- docs/schemas/wargraph-doc.schema.json src/config/node_cache.rs src/config/mod.rs .../trace.rs .../waypoint.rs .../node_error.rs .../node_cache_port.rs .../engine/node.rs .../engine/hooks.rs .../node_cache/in_memory.rs .../node_cache/redis.rs` exits 0. `$BASE` is non-empty, so the check is not vacuous.

## Deviations from Plan

### Auto-fixed Issues

None. The real-tree scan reported no hit, so no scanned file was edited and nothing was added to the skip set or to the allowed look-alikes (prohibition upheld).

### Plan-text refinements (judgment calls, not rule deviations)

**1. Removal reports are exact per file, not always exactly one**
- **Plan said:** removing each rule-3 target, rule-5 target or rule-4 declaration file yields "exactly one report per removal".
- **What is true:** `paladin.rs` is both a rule-3 target and the home of three rule-4 declarations (removal reports 1 + 3), and `wrapper.rs` is a rule-3 target and the home of `Entity<` (1 + 2). A zero count is a violation by the same plan (Edge ENTITY-05/empty), so reporting only one would hide a real failure.
- **Resolution:** `scanner_reports_missing_scoped_targets` asserts the computed count (`expected_reports_for_removal`: one per rule the file is a target of, plus one per rule-4 declaration homed there), and that every report names the removed file. Every other target yields exactly one.
- **Commit:** 5882953

**2. Emptied rule-3 targets are reported**
- **Reason:** the plan's Edge ENTITY-05/empty says an "absent or emptied" target must fail rather than pass vacuously, but the Phase 44 `scan_scoped` shape reports only unreadable files; an emptied container file would have passed.
- **Resolution:** `scan_scoped` also reports a target with no production lines (`... has no production lines, so the rule proves nothing`), and the missing-target control empties each rule-3 target and requires that report.
- **Commit:** 5882953

## Flagged assumptions

- A1-48-08 (ENTITY-03 edge, unclassified): kept as an explicit assumption. Rule 4's exact counts (both zero and two fail, and a wrong file fails), rule 2's family-prefix ban, rule 3's version-module needles and the untouched legacy guard cover it; no ordering, encoding or concurrency class applies to a declaration count.
- A2-48-08: `config.example.yml` has no `node_cache` section, so rule 5 pins the config definition sites. If 48-09's closeout finds a YAML section added by another phase, rule 5 gains it.
- A3-48-08: entity-sense residuals inside the retained items are by owner decision (ADR-0060 (g)); no rule targets them and the clean-tree control includes them.
- Edge ENTITY-02/encoding is verified by the controls (backstop): matching is exact, case-sensitive substring with identifier boundaries `is_alphanumeric() || '_'`; the clean-tree look-alikes (`StateNode::new`, `NodeId::new`, `node_id`, `NodeCacheKey`, `NodeError`) never match while each old name always does.

## Authentication Gates

None.

## Known Stubs

None. Test code only.

## Threat Flags

None. Test code that only reads the repository; no network endpoint, credential path or trust-boundary change (manual credential-handling review, D-00a: nothing to review). T-48-21 mitigated (skip set diffed against Task 1, no widening), T-48-22 mitigated (per-test trees from pid, atomic counter and clock, removed on drop; the real-tree scan only reads), T-48-23 mitigated (`DirEntry::file_type` does not follow symlinks; dot-directories and `target` skipped).

## Self-Check: PASSED

- File found: `tests/entity_rename_guard.rs`
- Commits found: `5882953`, `2bbd14f`
