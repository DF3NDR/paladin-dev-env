---
phase: 48-base-entity-rename
plan: 03
subsystem: core-base
tags: [rename, serde-read-alias, entity-version, public-api-baseline, changelog, d-05]

requires:
  - phase: 48-01
    provides: "pre-rename persisted-JSON fixtures and the entity_serde_compat.rs harness"
  - phase: 48-02
    provides: "ADR-0060 (the licence for the Phase 48 breaks, and the retained-items disposition)"
provides:
  - "The base version family renamed to EntityVersion* in base::service::entity_version_service"
  - "Serde read-aliases node_id / node_data on EntityVersion, proven against the captured v0.10 record"
  - "The written reason the five unused base items are kept, in the base module rustdoc"
  - "The regenerated public-API baseline (the 14 ContentItemService lines) and CHANGELOG entries"
affects: [48-04, 48-05, 48-07, 48-08, 48-09]

tech-stack:
  added: []
  patterns:
    - "Serde read-alias on a renamed persisted field: v0.10 key loads, new writes emit only the new key, both spellings rejected as duplicate field"
    - "Compat test compares re-serialization with the fixture after rename_key, never by editing the fixture"

key-files:
  created:
    - crates/paladin-core/src/base/service/entity_version_service.rs
  modified:
    - crates/paladin-core/src/base/service/mod.rs
    - crates/paladin-core/src/base/service/field_version_service.rs
    - crates/paladin-core/src/base/service/collection_versioning_service.rs
    - crates/paladin-core/src/base/mod.rs
    - crates/paladin-core/tests/entity_serde_compat.rs
    - src/core/platform/manager/content_service.rs
    - tests/functional/content_lifecycle_test.rs
    - tests/functional/content_fetching_pipeline_test.rs
    - .project/current-exports.txt
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
  removed:
    - crates/paladin-core/src/base/service/node_version_service.rs (git mv to entity_version_service.rs)

key-decisions:
  - "Aliases sit on the two EntityVersion fields only (the module's one serializable struct); no serde(default) anywhere, so a record with neither spelling still fails"
  - "The leading block comment became a //! module doc stating the version-flag contract (D-04) and the read-alias note (D-11, ADR-0060)"
  - "The retained-items rustdoc names the wrapper only as 'the base entity wrapper'; 48-04 links Entity<T> once it exists"
  - "The regenerated baseline keeps its new generated-at header line, so the diff is 14 content lines plus that one header line"

patterns-established:
  - "rename_key(value, pointer, from, to) test helper for proving a read-alias against a fixture"

requirements-completed: [ENTITY-03, ENTITY-04]

coverage:
  - id: D1
    description: "Version family renamed to EntityVersion* in entity_version_service with VersioningError::EntityNotFound, entity_id / entity_data / max_versions_per_entity; VersionHistory, VersionComparison, VersioningConfig, ChangeType, HashAlgorithm, VersioningError keep their names"
    requirement: "ENTITY-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib base::service (25 passed, the carried-over tests with identifier renames only)"
        status: pass
      - kind: other
        ref: "git grep -nE 'NodeVersion|node_version_service|NodeNotFound|max_versions_per_node' -- '*.rs' prints nothing"
        status: pass
    human_judgment: false
  - id: D2
    description: "The captured v0.10 version.json loads into EntityVersion<ContentData> through read-aliases and re-serializes to the fixture with only node_id / node_data renamed; both spellings of a key are rejected; new writes emit only the new keys"
    requirement: "ENTITY-03"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai-core --test entity_serde_compat -- pre_rename_version_json_still_loads (5 passed)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib entity_version_rejects_both_key_spellings"
        status: pass
    human_judgment: false
  - id: D3
    description: "EntityVersionService behaves as before: version numbering and history order unchanged through the root crate's content lifecycle and fetching pipeline"
    requirement: "ENTITY-04"
    verification:
      - kind: integration
        ref: "cargo test -p paladin-ai --test functional (10 passed, includes content_lifecycle and content_fetching_pipeline)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The five retained base items are untouched: the two retained services changed only their module path; base/entity, base/component and base/service declare the same modules"
    requirement: "ENTITY-04"
    verification:
      - kind: other
        ref: "git diff -U0 over field_version_service.rs and collection_versioning_service.rs: every changed line contains 'version_service::' (0 lines without)"
        status: pass
    human_judgment: false
  - id: D5
    description: "base/mod.rs carries a # Retained pending review section naming the five items, the content / general data orchestration system and the review todo"
    requirement: "ENTITY-04"
    verification:
      - kind: other
        ref: "acceptance greps from 48-03-PLAN.md Task 2 (heading count 1, all six names, three allow(missing_docs) kept); make doc-check"
        status: pass
    human_judgment: true
  - id: D6
    description: "Public-API baseline regenerated (14 ContentItemService / ContentItemServiceError lines) and root + paladin-core CHANGELOG [Unreleased] Changed entries added"
    requirement: "ENTITY-03"
    verification:
      - kind: other
        ref: "PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface exits 0; make check-api-examples; ./scripts/check-changelogs.sh"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 03: EntityVersion rename with v0.10 read-aliases Summary

**The base version family is now `EntityVersion*` in `entity_version_service`, a v0.10 version record still loads through serde read-aliases, and the base module rustdoc states once why the five unused base items stay.**

## Performance

- **Duration:** about 25 min
- **Completed:** 2026-10-13
- **Tasks:** 2 of 2
- **Files modified:** 12 (one git move)

## Accomplishments

- `node_version_service.rs` moved with `git mv` (R052) to `entity_version_service.rs`. `NodeVersion`, `NodeVersionMetadata`, `NodeVersionRepository`, `NodeVersionService` and `InMemoryNodeVersionRepository` became their `EntityVersion*` forms; `VersioningError::NodeNotFound` became `EntityNotFound` (message `Entity not found: {0}`); the disabled message reads `Versioning disabled for entity: {0}`.
- Fields renamed: `entity_id` / `entity_data` on `EntityVersion`, `entity_id` on `VersionHistory` and `VersionComparison`, `max_versions_per_entity` on `VersioningConfig`. `VersionHistory`, `VersionComparison`, `VersioningConfig`, `ChangeType`, `HashAlgorithm` and `VersioningError` keep their names.
- RED then GREEN, observed: after the rename alone, `pre_rename_version_json_still_loads` failed with ``missing field `entity_id` `` and the new `entity_version_rejects_both_key_spellings` failed because the doubled record was accepted; adding `#[serde(alias = "node_id")]` and `#[serde(alias = "node_data")]` turned both green.
- The consumer (`ContentItemService`) and the two functional tests follow the new names; the retained `FieldVersionService` and `CollectionVersionService` changed only their `use` line and doc-example path.
- `base/mod.rs` gained a `//!` header with a `# Retained pending review` section (D-05).
- `.project/current-exports.txt` regenerated with the CI-pinned `cargo-public-api` 0.52.0 and the `nightly-2026-09-20` toolchain; root and `paladin-core` CHANGELOGs record the rename.

## Task Commits

1. **Task 1: Rename the version family to EntityVersion*, with read-aliases proven against the captured v0.10 record** - `56213e4` (refactor)
2. **Task 2: Retained-items reason, API baseline refresh and CHANGELOG** - `109144d` (docs)

## Files Created/Modified

- `crates/paladin-core/src/base/service/entity_version_service.rs` - the renamed module, `//!` module doc, aliases, `entity_version_rejects_both_key_spellings`
- `crates/paladin-core/src/base/service/{mod,field_version_service,collection_versioning_service}.rs` - module declaration and import / doc-example path only
- `crates/paladin-core/tests/entity_serde_compat.rs` - `EntityVersion` import, `rename_key` helper, version compat test compares against the renamed fixture value
- `src/core/platform/manager/content_service.rs`, `tests/functional/content_{lifecycle,fetching_pipeline}_test.rs` - imports, type names, `entity_id` / `entity_data` field uses
- `crates/paladin-core/src/base/mod.rs` - module rustdoc
- `.project/current-exports.txt`, `CHANGELOG.md`, `crates/paladin-core/CHANGELOG.md`

## Decisions Made

- Aliases only on `EntityVersion`'s two fields; no `serde(default)`, so a record with neither spelling still fails, and both spellings fail with `duplicate field` (T-48-07).
- The graph-sense `node_id` was not touched: Task 1's commit contains only the eight files in the plan's set (checked against the plan's file-set pattern; 0 outside).
- No platform-wide revisioning was added (D-04): `create_version` still returns `VersioningDisabled` when the wrapper's `version` flag is false, and the module doc says only callers routed through the service honour it.

## Deviations from Plan

### Auto-fixed Issues

None. Two notes on literal wording:

**1. Baseline diff counts 15/15, not 14/14, by the plan's grep.**
- **Found during:** Task 2
- **Issue:** The acceptance greps `grep -c '^-[^-]'` / `'^+[^+]'` over `git diff -U0 .project/current-exports.txt` also match the file's first line, `# Public API Surface - Generated <timestamp>`, which `make api-surface-update` always rewrites. The diff is exactly 14 removed and 14 added content lines, every one a `ContentItemService` / `ContentItemServiceError` line, plus that one header line (`grep -c entity_version_service` prints 14).
- **Resolution:** Kept the truthful regenerated header rather than restoring the old timestamp. `make api-surface` exits 0 on the committed file. The research-measured 14-line change is exactly what landed.

**2. `cargo install cargo-public-api` was needed.** The tool was absent. Installed `cargo-public-api` 0.52.0 with `--locked` as the plan specifies (T-48-SC); it went to `~/.cargo/bin`, and `Cargo.lock` and every manifest are untouched (`git diff --exit-code HEAD~2 -- Cargo.lock` exits 0).

**Total deviations:** 0 behavioural; 1 literal-count note.

## Issues Encountered

- The two sandbox failures named in 48-RESEARCH.md Pitfall 10 still fail identically (`run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set`). `cargo test --workspace --lib --bins --no-fail-fast` shows 1348 passed and exactly those 2 failed in the root crate; every other binary passes. Not mine to fix.
- CI's `semver` job is expected red from commit `56213e4` until 48-07 registers the breaks (D-14). Recorded here, not suppressed.

## Verification

- `cargo test -p paladin-ai-core --lib base::service`: 25 passed; `--test entity_serde_compat`: 5 passed; `cargo test -p paladin-ai-core`: 709 lib, 5 integration, doctests all pass
- `cargo test -p paladin-ai --test functional`: 10 passed; doctests on `EntityVersionService` and `ContentItemService` pass
- `cargo clippy -p paladin-ai-core -p paladin-ai --all-targets --all-features -- -D warnings`: clean; `cargo fmt --check`: clean; `cargo check --workspace --all-targets --all-features`: clean
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`, `make doc-check`, `make check-api-examples`, `./scripts/check-changelogs.sh`: all exit 0
- `cargo test --test legacy_removal_guard`: 3 passed; `Cargo.lock` unchanged (D-00d)
- Manual credential-handling review (D-00a): the changed files hold no API key, response body, log statement or HTTP client

## Known Stubs

None.

## Threat Flags

None. T-48-07 mitigated (`entity_version_rejects_both_key_spellings`, no custom deserializer); T-48-08 mitigated (rename confined to the plan's file set); T-48-09 mitigated (retained services changed only on `version_service::` lines); T-48-SC mitigated (pinned `--locked` install, lockfile untouched).

## Next Phase Readiness

48-04 can rename the wrapper (`Node<T>` to `Entity<T>`): `entity_version_service.rs` still imports `crate::base::entity::node::Node` and builds `Node { .. node: .. }` literals in `restore_version` and its tests, which 48-04 and 48-05 will update. The compat harness is green and must stay green; 48-07 owns registering the semver breaks and the MIGRATION rows.

## Self-Check: PASSED

- FOUND: crates/paladin-core/src/base/service/entity_version_service.rs; node_version_service.rs absent
- FOUND commits: 56213e4, 109144d
