---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 05
subsystem: paladin-core
tags: [base-layer, versioning, fences, rustdoc, fixes-02, adr-0060]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01..55-04 landed; no code dependency"
provides:
  - "FieldVersionService::get_field_version_history (and the in-memory repository behind it) returns Err(VersioningError::EntityNotFound(field_id)) for a field with no stored version, instead of a history with a random node_id"
  - "purge_old_field_versions on an unversioned field still returns Ok(0)"
  - "CollectionVersionService's count-change detection emits ItemAdded / ItemRemoved carrying Uuid::nil(), a documented and deterministic 'item not tracked' sentinel, instead of a random id"
  - "rustdoc on Field, Field::new, Item, CollectionType, the element module, base/mod.rs and both versioning services that says the item is unfinished scaffolding, retained pending the ADR-0060 (g) review, and names 55-BASE-LAYER-REVIEW.md"
affects: [55-12, 55-13]

tech-stack:
  added: []
  patterns:
    - "A fence refuses or reports a documented sentinel; it never presents a fabricated value as data"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/base/service/field_version_service.rs
    - crates/paladin-core/src/base/service/collection_versioning_service.rs
    - crates/paladin-core/src/base/entity/field.rs
    - crates/paladin-core/src/base/entity/collection.rs
    - crates/paladin-core/src/base/component/element.rs
    - crates/paladin-core/src/base/mod.rs

key-decisions:
  - "The history refusal lives in the repository, the one place that builds node_id; purge maps EntityNotFound to Ok(0) (PD-05-1)"
  - "The collection fence is the nil sentinel, not a method refusal; the detector stays count-only (PD-05-2)"
  - "No VersioningError variant, no pub item removed, renamed or re-signatured; make api-surface reports the surface unchanged"

patterns-established:
  - "Prove-then-lock: grep the fabricated-value sites, watch the new test fail, then fix"

requirements-completed: [FIXES-02]

coverage:
  - id: D1
    description: "A field with no stored versions is EntityNotFound for its history (service and repository) and Ok(0) for a purge"
    requirement: "FIXES-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service::field_version_service#field_version_history_of_an_unversioned_field_is_entity_not_found"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service::field_version_service#purging_an_unversioned_field_purges_nothing"
        status: pass
    human_judgment: false
  - id: D2
    description: "Collection count changes carry the nil sentinel, comparisons are idempotent, equal / same-length-different / empty item lists report exactly the documented changes"
    requirement: "FIXES-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service::collection_versioning_service#collection_count_changes_carry_the_nil_sentinel"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service::collection_versioning_service#comparing_the_same_collection_versions_twice_is_identical"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service::collection_versioning_service#equal_collections_report_no_change"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every retained base-layer item says in its rustdoc it is unfinished and names the recorded review; the vocabulary freeze, rustdoc and doctests are intact"
    requirement: "FIXES-02"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai --test entity_rename_guard --offline"
        status: pass
      - kind: other
        ref: "cargo doc -p paladin-ai-core --no-deps --offline (0 warning lines); cargo test -p paladin-ai-core --doc"
        status: pass
    human_judgment: true
    rationale: "Whether the rustdoc wording is accurate and sufficient for a reader is a judgment no test asserts; the greps only prove the review file is named"

duration: ~55 min
completed: 2026-11-01
---

# Phase 55 Plan 05: base-layer versioning fences and rustdoc Summary

**`FieldVersionHistory` refuses an unversioned field with `EntityNotFound` instead of inventing a `node_id`, the collection change detector reports a documented `Uuid::nil()` sentinel instead of a random item id, and every retained `paladin-core` base-layer item now says in rustdoc that it is unfinished and names `55-BASE-LAYER-REVIEW.md`.**

## Plan-start SHA

`0e27b1889e4d19a3992bb329332b08aa4ba817e0`

## Performance

- **Tasks:** 3 of 3 (Task 1 is the tracer; its gate ran in auto mode and passed)
- **Files modified:** 6 (0 created)
- **Commits:** `4eb02f0b9` (Task 1), `6e459a5bb` (Task 2), `36d658b73` (Task 3)

## Prove-then-lock

`grep -n 'Uuid::new_v4'` over the two service files at the plan-start SHA, split into fabricated-value sites and legitimate fresh-id sites:

| File | Line | Site | Class |
|------|------|------|-------|
| `field_version_service.rs` | 487 | `.unwrap_or_else(Uuid::new_v4)` building `FieldVersionHistory.node_id` | fabricated value (fenced, Task 1) |
| `collection_versioning_service.rs` | 415, 417 | `ItemAdded(Uuid::new_v4())`, `ItemRemoved(Uuid::new_v4())` in `detect_collection_changes` | fabricated value (fenced, Task 2) |
| `field_version_service.rs` | 196 | `version_id` of a new version | legitimate fresh id |
| `collection_versioning_service.rs` | 216 | `version_id` of a new version | legitimate fresh id |
| `collection_versioning_service.rs` | 632-1082 | test fixtures and unknown-collection probes | legitimate (tests) |

## Watched failing

- `field_version_history_of_an_unversioned_field_is_entity_not_found` failed before the change (`expected a refusal, got a history for 27e742b7-...`); `purging_an_unversioned_field_purges_nothing` passed before and after (it pins behaviour the fence must keep).
- `collection_count_changes_carry_the_nil_sentinel` failed before the change (`left: [ItemAdded(78ae3f47-...)]`, `right: [ItemAdded(0000...0000)]`); `comparing_the_same_collection_versions_twice_is_identical` failed before the change (two different random ids from two calls); `equal_collections_report_no_change` passed before and after (it pins the unchanged branch logic).
- After each fix the module tests are green: 4 of 4 in `field_version_service`, 19 of 19 in `collection_versioning_service`, 57 of 57 under `base::`.

## Accomplishments

- **Field history fence.** `InMemoryFieldVersionRepository::get_field_version_history` returns `Err(VersioningError::EntityNotFound(field_id))` when a field has no stored version; a field with versions still gets its first version's `node_id`, and `test_field_version_history` now also asserts `history.node_id == field.nid`. `purge_old_field_versions` maps that refusal to `Ok(0)`, so the auto-purge path after `create_field_version` is untouched. The trait method, the service method and `FieldVersionService` carry `# Errors` and fence rustdoc.
- **Collection sentinel.** `detect_collection_changes` emits `ItemAdded(Uuid::nil())` / `ItemRemoved(Uuid::nil())`; branch logic is unchanged. The `// Simplified` and "In a real implementation" comments are replaced by one comment, and `ItemAdded`, `ItemRemoved`, `CollectionVersionComparison::detected_changes` and `CollectionVersionService` document the sentinel and that detection is count-only.
- **Rustdoc.** `field.rs`, `collection.rs` and `element.rs` lose their `/* */` headers for `///` / `//!` rustdoc; the stale `DataItem` / `data_points` comments about types that do not exist are gone. `base/mod.rs` now says the review is recorded in `55-BASE-LAYER-REVIEW.md`, every item is kept, the two versioning services are fenced, and removal or finishing is a next-minor decision.

## Per-item read for 55-12's review file

| Item | Design intent | Current state |
|------|---------------|---------------|
| `Field<T>` (`fid`, `nid`, `value`) | A single value held as a child of an `Entity` (username, email, ...); the Drupal-style field | Plain struct with no consumer outside `base` except `FieldVersionService`; `Field::new` assigns a `nid` that refers to no parent. Kept |
| `Item<T>` | One member of a collection, wrapping a payload | Plain wrapper; consumed only by `CollectionType` and the collection versioning service. Kept |
| `CollectionType<T>` | A generic, uuid-identified list of `Item`s (content lists, taxonomies) | Plain container with accessors; consumed only by `CollectionVersionService`. Kept |
| `component::element` | A (potential) shared reference / edge connecting entities (the `tag` example) | Comment-only module, no type. Nothing to fence. Kept |
| `FieldVersionService` + `InMemoryFieldVersionRepository` | Field-level revision history, per-field opt-in | Works against the in-memory repository only; fenced so an unversioned field is `EntityNotFound` rather than given an invented parent id. Keep and fence |
| `CollectionVersionService` + `InMemoryCollectionVersionRepository` | Collection-level revision history with change detection | Versioning works; change detection is count-only and now reports the nil sentinel. No per-item diff, no persistence. Keep and fence |

## Task Commits

1. **Task 1 (tracer): unversioned field history refusal** - `4eb02f0b9` (fix)
2. **Task 2: collection nil sentinel** - `6e459a5bb` (fix)
3. **Task 3: rustdoc on retained items** - `36d658b73` (docs)

## Verification

| Check | Result |
|-------|--------|
| `cargo test -p paladin-ai-core --offline --lib base::` | 57 passed |
| `cargo test -p paladin-ai-core --offline --doc` | 153 passed, 38 ignored (pre-existing) |
| `cargo test -p paladin-ai --test entity_rename_guard --offline` | 8 passed |
| `cargo doc -p paladin-ai-core --no-deps --offline` | 0 `warning` lines |
| `cargo fmt --check -p paladin-ai-core` | clean |
| `cargo clippy -p paladin-ai-core --tests --offline -- -D warnings` | exit 0 |
| `make api-surface` | `API surface unchanged` (4325 items) |
| `git diff 0e27b1889 -- entity_version_service.rs` | empty (no new `VersioningError` variant) |
| `git diff` removed lines naming a `pub` item under `base/` | none |
| `grep -c Simplified collection_versioning_service.rs` | 0 |
| `grep -n 'unwrap_or_else(Uuid::new_v4)' field_version_service.rs` | nothing |
| `grep -c '^/\*'` on field.rs, collection.rs, element.rs | 0 each |

## Deviations from Plan

None - plan executed exactly as written. Two small notes, neither a deviation:

- `CollectionChangeType` already derives `PartialEq`, so the idempotency test compares `detected_changes` directly rather than through `format!("{:?}", ..)`.
- The two new collection fixtures helpers (`named`, `two_versions`) live in the existing test module; every pre-existing test is unchanged (the diff of the test module adds lines only).

## Authentication Gates

None.

## Known Stubs

None added. The content / general-data base-layer items are unfinished by design and are recorded as such in `55-BASE-LAYER-REVIEW.md` (plan 55-12); this plan only fences them and documents it.

## Threat Flags

None. T-55-13 (fabricated ids presented as data) is mitigated by the typed refusal, the documented nil sentinel and the tests that pin both and the idempotency of comparisons. T-55-14 (rename or removal of a base-layer `pub` item) is mitigated by doc-only edits outside the two bodies, a green `entity_rename_guard`, an unchanged `make api-surface` and an empty removed-`pub`-line diff.

## Notes

- `FIXES-02` is shared with plan 55-06 (the content-processing half); `requirements.mark-complete` is gated on both plans having a SUMMARY.
- Disk: no space problems; `target/debug/incremental` was not deleted (`CARGO_INCREMENTAL=0` was used throughout).

## Self-Check: PASSED

- Files modified exist on disk: field_version_service.rs, collection_versioning_service.rs, field.rs, collection.rs, element.rs, mod.rs (all FOUND).
- Commits `4eb02f0b9`, `6e459a5bb`, `36d658b73` exist (`git log --oneline --grep='55-05'`).
- All three tasks' `<acceptance_criteria>` and the plan-level `<verification>` were re-run after the last commit and pass (table above).
