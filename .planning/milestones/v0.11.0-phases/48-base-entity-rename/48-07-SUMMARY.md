---
phase: 48-base-entity-rename
plan: 07
subsystem: api-register
tags: [semver, migration-register, allowlist, changelog, d-14, d-27, adr-0060]

requires:
  - phase: 48-03
    provides: "EntityVersion family rename and its field/variant renames"
  - phase: 48-04
    provides: "Entity<T>, base::entity::node module removal, citadel placeholder removal, PaladinState.paladin retype"
  - phase: 48-05
    provides: "node -> data and node -> entity field renames with serde read-aliases"
provides:
  - "Phase 48 MIGRATION.md 9.2 rows (6 Y, 3 register-only N), measured from the D-27 diagnostic on the final tree"
  - "Set-equal allowlist block 'Phase 48 (Base Entity Rename, ENTITY-01..05)' with 12 entries"
  - "Two new crate-wide allow lines in paladin-ai-core (module_missing, trait_missing)"
  - "MIGRATION.md 9.1 M-B-12, the 9.4 Phase 48 bullet and the 9.7 sentence"
  - "Root and paladin-core CHANGELOG Removed / Changed / Breaking entries"
affects: [48-08, 48-09]

tech-stack:
  added: []
  patterns:
    - "Register lands once, from a measured diagnostic: allow lines, 9.2 rows and allowlist entries in one commit, set-equal"

key-files:
  created: []
  modified:
    - crates/paladin-core/Cargo.toml
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md

key-decisions:
  - "Only the lints the diagnostic fired were registered; the pre-D-05/D-10 forecast in 48-RESEARCH.md was not used as evidence"
  - "ContentItem and PromptItem each got two entries: struct_pub_field_missing (the removed node field) and constructible_struct_adds_field (the new entity field), both measured"
  - "The ContentItemService signatures are a register-only N row because the paladin-ai diagnostic fired nothing for them"
  - "No row, entry or changelog removal for the five retained base items (D-05); 9.7 still opens with Empty and nothing is deprecated (D-02)"

patterns-established:
  - "Manifest allow-line prose names removed items only in words, so the 48-08 guard (which scans .toml prose) stays green"

requirements-completed: [ENTITY-01, ENTITY-02, ENTITY-03, ENTITY-05]

duration: about 30 min
completed: 2026-10-13
status: complete
---

# Phase 48 Plan 07: Register the Phase 48 breaks Summary

**Every Phase 48 public break is registered once from a measured cargo-semver-checks diagnostic (6 `Y` rows, 12 set-equal allowlist entries, 2 new allow lines, 3 register-only rows), and the operator record (M-B-12, 9.4, 9.7) and CHANGELOG entries say what changed on disk and in the API, in what order to upgrade, and how to roll back.**

## Accomplishments

- Installed `cargo-semver-checks` 0.50.0 (`--locked`, the CI pin) into `~/.cargo/bin`; ran the D-27 diagnostic on the final tree for both crates against the published 0.10.1 (`--release-type minor`, `--default-features`), each crate's crate-wide allow lines removed and the manifest restored byte-for-byte.
- Wrote the Phase 48 9.2 rows, the allowlist block and the two new allow lines in one commit; `make check-migration-allowlist` exits 0 (row-level set equality).
- Wrote 9.1 `M-B-12`, the 9.4 bullet (readers before writers, back up, restore-only downgrade, the `Reasoning` status case), the 9.7 sentence, and the CHANGELOG blocks.

## Measured diagnostic (D-27, cargo-semver-checks 0.50.0 vs the published 0.10.1)

| Crate | Checks | Lints fired across the crate | On Phase 48 items | Restore proof |
|---|---|---|---|---|
| `paladin-ai-core` | 196 | 11 | 6 lint ids, listed below | `cmp` of the byte copy against the restored manifest: `RESTORED_OK`, `git status` clean |
| `paladin-ai` | 196 | 1 (`constructible_struct_adds_field`, on earlier phases' fields only) | 0 | `cmp`: `RESTORED_OK`, `git status` clean |

Phase 44 recorded 9 lints for `paladin-ai-core`; the two new ones are `module_missing` and `trait_missing`, as forecast. Lints fired on Phase 48 items (the rest belong to earlier phases and were not registered again):

- `struct_missing`, `module_missing`: `Node` struct and `base::entity::node` module (row `Node`, 2 lints)
- `enum_missing` (`VersioningError`, `ChangeType`, `HashAlgorithm`), `module_missing` (`node_version_service`), `struct_missing` (7 structs), `trait_missing` (`NodeVersionRepository`): row `NodeVersionService`, 4 lints
- `struct_missing` on the citadel `PaladinData`; `enum_missing` on the citadel `PaladinStatus`
- `struct_pub_field_missing` on `ContentItem.node` and `PromptItem.node`, and `constructible_struct_adds_field` on `ContentItem.entity` and `PromptItem.entity`
- Nothing fired for the citadel `Paladin` alias, `PaladinState.paladin`, the five retained base items or `ContentItemService`.

## CI-equivalent loop (baseline 0.9.0, allow lines in place)

`cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0`, every package in ci.yml's `PACKAGES` array:

| Package | Exit |
|---|---|
| paladin-ai | 0 |
| paladin-ai-core | 0 |
| paladin-ports | 0 |
| paladin-battalion | 0 |
| paladin-herald | 0 |
| paladin-llm | 0 |
| paladin-memory | 0 |
| paladin-storage | 0 |
| paladin-notifications | 0 |
| paladin-content | 0 |
| paladin-web | 0 (see Deviation 1) |

A2-48-07: against the 0.9.0 baseline the tool runs 0 checks for every package (0.9.0 to 0.10.1 is treated as a breaking-permitted bump), so this loop passes without examining the allow lines. The 0.10.1 diagnostic above is what measures the lints. No lint was seen only against 0.9.0.

## Task Commits

1. **Task 1: measured register (rows, allowlist, allow lines)** - `7fea514` (docs)
2. **Task 2: operator and changelog record** - `2795c91` (docs)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md and ROADMAP.md (docs: complete plan)

## Register contents

- `Y` rows (`paladin-ai-core`): `Node`, `NodeVersionService`, `PaladinData`, `PaladinStatus`, `ContentItem`, `PromptItem`; 12 allowlist entries, requirement ids `ENTITY-01, ENTITY-02` (wrapper), `ENTITY-03` (version family, citadel types), `ENTITY-01` (content wrapper fields).
- Register-only `N` rows: citadel `Paladin` alias, `PaladinState`, and `paladin-ai | ContentItemService` (14 lines of `.project/current-exports.txt` moved; the diagnostic fired nothing for it).
- New allow lines in `crates/paladin-core/Cargo.toml`: `module_missing`, `trait_missing`. Root `Cargo.toml` unchanged. `Cargo.lock` untouched (`git diff --exit-code HEAD~2 -- Cargo.lock` exits 0).
- Verification: `make check-gates`, `make check-migration-allowlist`, `./scripts/check-changelogs.sh` and `cargo fmt --check` exit 0; `git grep '#[deprecated' -- '*.rs'` finds 0; 9.8 is byte-identical to HEAD; 9.7 still opens with `Empty`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] paladin-web 0.9.0 baseline build could not download Swagger UI**
- **Found during:** Task 1, CI-equivalent loop
- **Issue:** the baseline crate (0.9.0) lacks the `vendored` feature, so its `utoipa-swagger-ui` build script downloads a zip from GitHub through `curl`; the sandbox proxy returned a non-zip (`InvalidArchive("Could not find EOCD")`), exit 101. The current tree vendors the bundle, so only the baseline build was affected.
- **Fix:** re-ran that one package with `SWAGGER_UI_DOWNLOAD_URL=file://<vendored v5.17.14.zip from the cargo registry>` (the same upstream version); exit 0. No file in the repo changed.
- **Files modified:** none
- **Commit:** n/a

**2. [Rule 3 - Blocking] Disk pressure during the loop**
- **Found during:** Task 1
- **Issue:** free space fell to 2.7 GB mid-loop.
- **Fix:** deleted `target/semver-checks/local-*` caches of packages that had already passed (inside `target/`, regenerated on demand); free space recovered to 6.6 GB.

**3. Measured finding that differs from the plan's forecast (not a deviation in outcome)**
- `ContentItem` and `PromptItem` fired `constructible_struct_adds_field` (for the new `entity` field) in addition to `struct_pub_field_missing` (for the removed `node` field); both are registered, each already covered by an existing crate-wide allow line.

None of the above changed the committed files beyond the plan's file list.

## Authentication Gates

None. The `cargo install` printed a warning that `chacha20 v0.10.1` in the tool's own lockfile is yanked; the install proceeded under `--locked` as specified.

## Known Stubs

None.

## Threat Flags

None. Manifests, registers and documentation only; no network endpoint, credential path or trust-boundary change (manual credential-handling review, D-00a: nothing to review).

## Self-Check: PASSED

- Files found: `crates/paladin-core/Cargo.toml`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `CHANGELOG.md`, `crates/paladin-core/CHANGELOG.md`
- Commits found: `7fea514`, `2795c91`
