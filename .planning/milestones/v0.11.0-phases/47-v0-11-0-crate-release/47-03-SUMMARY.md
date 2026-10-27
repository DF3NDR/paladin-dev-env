---
phase: 47-v0-11-0-crate-release
plan: 03
subsystem: release
tags: [release, version-bump, cargo-release, changelog, openapi, lockstep]

requires:
  - phase: 47
    provides: "47-01 pre-build decision (R7=a EMPTY=a SPACE=a DATE=2026-10-25) and 47-02 remedies (SHA-PRE-B1, condensed root body 108,926 bytes)"
provides:
  - "Lockstep 0.11.0 in 13 manifests and 13 Cargo.lock workspace entries, proven structurally (SHA 1a77f7bd)"
  - "Twelve changelogs stamped ## [0.11.0] - 2026-10-25, R7=a and EMPTY=a applied (SHA 31082973)"
  - "OpenAPI baseline regenerated, version line only (SHA-H c9456846)"
  - "Release body 108,926 bytes and payload body 108,824 characters, rehearsed offline on the stamped section"
affects: [47-04, 47-05, 47-06, 47-08]

tech-stack:
  added: []
  patterns:
    - "tool-generated bump proven by cargo metadata structure (jq) rather than by reading text"
    - "three isolated chore(release) commits, each path set proven with git diff --name-only"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-03-SUMMARY.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/doc-examples/Cargo.toml
    - crates/paladin-battalion/Cargo.toml
    - crates/paladin-content/Cargo.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-eval/Cargo.toml
    - crates/paladin-herald/Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-memory/Cargo.toml
    - crates/paladin-notifications/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-storage/Cargo.toml
    - crates/paladin-web/Cargo.toml
    - CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-content/CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-eval/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-memory/CHANGELOG.md
    - crates/paladin-notifications/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - crates/paladin-web/openapi.json
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md

key-decisions:
  - "Applied R7=a, EMPTY=a, SPACE=a and DATE=2026-10-25 exactly as recorded in 47-01-SUMMARY.md; no script, workflow or non-bump file edited"
  - "Evidence rows are committed after each code commit so every chore(release) commit's path set stays pure"

requirements-completed: []

duration: ~50m
completed: 2026-10-25
status: complete
---

# Phase 47 Plan 03: Build wave (bump, stamp, OpenAPI) Summary

**Workspace bumped to 0.11.0 by `cargo release version` (no hand edit), twelve changelogs stamped 2026-10-25 with the `[Unreleased]` anchor removed, and the OpenAPI baseline regenerated with only its `version` line changed; the release body fits at 108,926 bytes.**

## Pre-build decision applied

Read from `47-01-SUMMARY.md` `## Pre-build decision`: `R7=a EMPTY=a SPACE=a DATE=2026-10-25` (B1 and B2 were consumed by 47-02). SHA-PRE-BUMP was read from `git rev-parse HEAD` at the start: `680cf9c294dc22de02c1faae19afc45188470a35` (the 47-02 completion commit).

## Commits (all on `claude/laughing-dirac-e0h2ax`, nothing pushed, tagged or published)

| # | SHA | Subject | Path set (`git diff --name-only`) |
|---|-----|---------|------------------------------------|
| 1 | `1a77f7bd39d8c6d3f0cbc3f25ff900ec03e15efb` | `chore(release): bump workspace to 0.11.0` | 13 manifests plus `Cargo.lock` (14) |
| 2 | `dacdbfc0` | `docs(47-03): record the bump proofs` | evidence file |
| 3 | `3108297339ccf7e74e75f85ed5f64400aadd710b` | `chore(release): stamp 0.11.0 changelogs (2026-10-25)` | 12 changelogs |
| 4 | `fe5d0736` | `docs(47-03): record the stamp and the release-body rehearsal` | evidence file |
| 5 | `c945684647c4f9c78fdde2fbe2fc8fc2e8806106` | `chore(release): regenerate OpenAPI baseline for 0.11.0` | `crates/paladin-web/openapi.json` |
| 6 | `97a1c3a9` | `docs(47-03): record the OpenAPI regeneration and SHA-H` | evidence file |

`SHA-PRE-BUMP` = `680cf9c294dc22de02c1faae19afc45188470a35`. **`SHA-H` = `c945684647c4f9c78fdde2fbe2fc8fc2e8806106`** (the OpenAPI commit, the candidate final code head); both are recorded as `- SHA-...:` lines in the evidence Provenance. `git diff --name-only SHA-PRE-BUMP SHA-H -- . ':!.planning'` is exactly 27 paths: the 13 manifests, `Cargo.lock`, the 12 changelogs, `openapi.json` (set-equal, asserted).

## Task 1: bump (tracer)

- Dry run of `cargo release version 0.11.0 --workspace` read first: upgrades for all 13 packages, no tag, publish or push step. Then `--execute --no-confirm --workspace` (cargo-release 1.1.7).
- **Hand-touched `doc-examples` lines: none.** The tool rewrote the manifest version and all eight `paladin-*` pins (the plan text says nine; the file carries eight, recorded as a finding).
- Structural proof: unique workspace version `["0.11.0"]`; all 45 versioned path-dependency requirements name `0.11.0`; the three `req "*"` path dev-dependencies are the Phase 37.1 publish-order design (`paladin-battalion -> paladin-llm`, `paladin-battalion -> paladin-storage`, `paladin-llm -> paladin-storage`); comment-filtered grep for `0.10.1` empty (14 comment-only history lines left untouched); `cargo metadata --locked --offline` exit 0; lockfile old-version entries are third-party only (`chacha20poly1305`, `core-foundation`, `rand`, `rand_core`); `crates/paladin-ports/Cargo.toml` line 16 reads `=0.11.0`; derived publishable list 12 and unchanged; `make check-gates` exit 0.
- Tracer gate (autonomous run): the automated verify exited 0, so expansion continued without a checkpoint.

## Task 2: stamp

- `scripts/finalize-crate-changelogs.sh --version 0.11.0 --date 2026-10-25` finalized all 12 changelogs.
- **Crates handled under EMPTY=a:** `paladin-content`, `paladin-memory`, `paladin-notifications` (zero non-blank lines under `[0.11.0]`); each now carries "No functional change; version bumped in lockstep with the workspace."
- R7=a: every changelog `[Unreleased]` count 0, `[0.11.0] - 2026-10-25` count 1, first release heading is the new one; highest pre-image line touched is 8 in every file, well above each `## [0.10.1]` heading.
- `make check-gates`, `make check-changelogs` and `check-release-consistency.sh --tag v0.11.0` exit 0; clause 3 (CI conclusion) is recorded as not checked locally.
- **Extracted release body: 108,926 bytes** (limit 110,000). **`_cor_build_payload` payload `.body` length: 108,824 characters** (cap 125,000), exit 0, offline.

## Task 3: OpenAPI

- Capacity: `df -h /` 2.5G free and `target` 20G before; `cargo clean` removed 19.8GiB (22G free after); `make openapi` took 1m2.7s with no out-of-space abort. **No capacity event.**
- `git diff --numstat` on `openapi.json` is `1 1`; the diff is the single `"version": "0.10.1"` to `"0.11.0"` line (D-07: no route or schema moved).
- `openapi_matches_committed_baseline` passes against the regenerated file (run with `UPDATE_OPENAPI` unset).

## Deviations from Plan

None - plan executed exactly as written. Minor notes recorded in the evidence: the plan's "nine" `doc-examples` pins is eight in the file (no effect); the evidence rows are committed after each code commit rather than inside it, as the plan's step order prescribes.

## Auth gates

None.

## Known Stubs

None. This plan edits manifests, a lockfile, changelogs and a generated JSON baseline.

## Threat Flags

None. No new endpoint, auth path or schema surface; T-47-09 to T-47-12 mitigated by the structural proofs, the body and payload rehearsal, the one-line OpenAPI assertion and the asserted 27-path set.

## Self-Check: PASSED

- Commits `1a77f7bd`, `dacdbfc0`, `31082973`, `fe5d0736`, `c9456846`, `97a1c3a9` exist: FOUND.
- `47-CI-EVIDENCE.md` carries `- SHA-PRE-BUMP:` and `- SHA-H:` and Local sweep rows 24-46: FOUND.
- Task 1, Task 2 and Task 3 automated verifies each exited 0. Nothing pushed, tagged or published.
