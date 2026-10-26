---
phase: 47-v0-11-0-crate-release
reviewed: 2026-10-26T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - CHANGELOG.md
  - Cargo.toml
  - crates/doc-examples/Cargo.toml
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-content/CHANGELOG.md
  - crates/paladin-content/Cargo.toml
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/Cargo.toml
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-eval/Cargo.toml
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-herald/Cargo.toml
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-memory/CHANGELOG.md
  - crates/paladin-memory/Cargo.toml
  - crates/paladin-notifications/CHANGELOG.md
  - crates/paladin-notifications/Cargo.toml
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/openapi.json
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 47: Code Review Report

**Reviewed:** 2026-10-26
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

Reviewed `git diff 2e857c6ce7e0d9f37ff5cc74def293d2d0587756` for each listed file against the working tree on `chore/47-close`. No publish-breaking defects were found. Verified:

- Every one of the 13 manifests carries `version = "0.11.0"`, and `Cargo.lock` agrees: all 13 workspace packages are at 0.11.0 and tokio is at 1.53.3.
- Every intra-workspace dependency version string in root `Cargo.toml` (workspace table, the `paladin-llm` mock dev-dep, both `paladin-eval` edges) and in each crate manifest and `crates/doc-examples/Cargo.toml` reads 0.11.0.
- The one exact pin, `paladin-ports` -> `paladin-ai-core` `=0.11.0`, matches.
- `fixtures/codeql-probe/Cargo.toml` is at 0.0.0 and not a workspace member. The remaining `0.10.1` strings in manifests are comments recording the historical cargo-semver-checks baseline, which is correct.
- `openapi.json` differs by exactly one line, `info.version`, now 0.11.0, and still parses.
- All 12 crate changelogs plus the root have a well-formed `## [0.11.0] - 2026-10-26` heading, and no `[Unreleased]` heading remains in any changelog.
- The root `[0.11.0]` section body is 108,951 bytes, under the 110,000-byte limit.
- Commit `700a92d86`, which the root changelog cites as the uncondensed source, exists and is an ancestor of both `v0.11.0` and `HEAD`.
- The `paladin-content`, `paladin-memory` and `paladin-notifications` source diffs since `v0.10.1` are test-only, apart from effects inherited from `paladin-core`.

## Warnings

### WR-01: "No functional change" in paladin-memory and paladin-content changelogs understates inherited persisted-format change

**File:** `crates/paladin-memory/CHANGELOG.md:8-10` (same text in `crates/paladin-content/CHANGELOG.md:8-10`)
**Issue:** Both crates are stamped `[0.11.0]` with "No functional change; version bumped in lockstep with the workspace." The crates' own `src` changes since `v0.10.1` are test-only, but the `paladin-core` Phase 48 rename changes what these crates read and write. `FileCitadel` (`paladin-memory`) now writes `paladin.data` instead of `paladin.node` in Citadel snapshots. The root `[0.11.0]` entry (`CHANGELOG.md:176-179`) states this change is one-way and that a v0.10 binary cannot read what v0.11 wrote. `paladin-memory` also gains a new test, `pre_rename_citadel_snapshot_loads_through_file_citadel`, that pins this behaviour. `paladin-content`'s `ContentItem` is likewise stored under `entity` rather than `node`. A consumer who depends only on `paladin-memory` and reads only its changelog is told nothing changed, while a one-way on-disk format change applies to the snapshots `FileCitadel` writes. The root condensation note also says per-change detail lives in "each crate's CHANGELOG `[0.11.0]` section", which is not true for these two crates.
**Fix:** Replace the lockstep line with a pointer, for example:
```markdown
## [0.11.0] - 2026-10-26

No source change in this crate beyond tests. `FileCitadel` snapshots are now written with the
`paladin.data` key (previously `paladin.node`) as a consequence of the `paladin-ai-core` base
Entity rename (ADR-0060); v0.11 reads both spellings, but a v0.10 binary cannot read what v0.11
wrote. See the root `CHANGELOG.md` and `MIGRATION.md` 10.1 M-B-12 and 10.4.
```
Apply equivalent wording to `paladin-content` for `ContentItem` and `PromptItem`. Do this in the next patch release's changelog, since `v0.11.0` is already tagged.

### WR-02: Dangling `[Unreleased]` link reference, and a stale link-reference block, remain at the end of the root changelog

**File:** `CHANGELOG.md:3396-3398`
**Issue:** The release deleted the `## [Unreleased]` heading in the root changelog. The trailing link-reference definition `[Unreleased]: https://github.com/jamatulli/paladin/compare/v0.2.0...HEAD` remains and now refers to nothing. Its compare base is `v0.2.0`, so it would advertise every release since 0.2.0 as unreleased if it were ever used. The block defines only `0.2.0` and `0.1.0`. Headings from `[0.3.0]` through `[0.11.0]`, including the new `## [0.11.0]`, have no link definitions, so those headings render as plain text rather than compare links. The stale reference predates this phase, but the phase's "delete the `[Unreleased]` anchor" step left it as the only `Unreleased` token in any changelog.
**Fix:** Either delete the `[Unreleased]:` line, or regenerate the block so it is current, for example `[Unreleased]: .../compare/v0.11.0...HEAD` plus `[0.11.0]: .../compare/v0.10.1...v0.11.0` and the intermediate versions. If the repo is no longer `jamatulli/paladin`, correct the host path too.

## Info

### IN-01: Root `[0.11.0]` body leaves about 1 KB of headroom under the release-body limit

**File:** `CHANGELOG.md:8-658`
**Issue:** The section is 108,951 bytes against a 110,000-byte limit, leaving 1,049 bytes (under 1%). Any post-release correction to this section, or a different extraction boundary (heading and trailing blank lines included or excluded), could push the next release over. This is a robustness note only, since the release has shipped.
**Fix:** For the next release, keep the root entry short by design, with detail in the per-crate changelogs, rather than condensing after the fact. Add a CI byte-count check on the extracted section.

### IN-02: Phase ordering inside the condensed `Added` subsection is not monotonic

**File:** `CHANGELOG.md:14-139`
**Issue:** The `Added` phase groups run 51, 50.1, 50, 49, 43, 42, 41, 38, 39, 40, 45, 46. Phases 38, 39 and 40 appear out of numeric order, and 38 sits after 41-43. The `Changed` subsection (lines 144-219) uses a different order again (51, 50.1, 50, 48, 43, 42, 41, 39, 38, 40, 45, 44). This is cosmetic, but it makes a specific phase's entries harder to find.
**Fix:** Sort phase groups newest-first, or ascending, consistently across subsections.

---

_Reviewed: 2026-10-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
