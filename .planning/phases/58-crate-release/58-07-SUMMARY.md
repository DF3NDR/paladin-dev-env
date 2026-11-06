---
phase: 58-crate-release
plan: 07
subsystem: release-bump
tags: [release, version-bump, changelog, migration, ship-08, ship-09, openapi]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-06 register lock and D-11 semver; plans 58-15 and 58-16 credential-class fixes (M-B-48..M-B-52) and the SHA-P2 pre-bump head
provides:
  - "0.11.1 in 13 manifests (root plus 12 members) and in the 13 paladin-* Cargo.lock entries; paladin-ports keeps the exact =0.11.1 pin"
  - "the 52 gate-listed strings at 0.11.1 and crates/paladin-web/openapi.json info.version at 0.11.1 (one-line diff)"
  - "one dated ## [0.11.1] - 2026-11-05 section in each of the 12 changelogs, under an empty [Unreleased]"
  - "MIGRATION 11.8 ### Library consumers: v0.11.0 -> v0.11.1 in the maintainer's pointer-list wording, naming SHA-S2 and 2026-11-05"
  - "docs/src/api-reference/upgrading.md Upgrading from v0.11.0 to v0.11.1 and migration-guide.md naming v0.11.1 as current"
  - "SHA-C1, SHA-C2, SHA-C3 and STAMP in the evidence Provenance; nothing pushed"
affects: [58-08, 58-09, 58-10, 58-11, 58-12, 58-14]

tech-stack:
  added: []
  patterns:
    - "Gate output as the edit list: line-addressed substitution of exactly the lines check-version-strings.sh reports, never a blanket replace"
    - "Three consecutive bump commits (chore, chore, docs) then one .planning/-only evidence commit"

key-files:
  created:
    - .planning/phases/58-crate-release/58-07-SUMMARY.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/*/Cargo.toml (11 members and crates/doc-examples)
    - crates/paladin-web/openapi.json
    - README.md
    - docs/src/ (9 files carrying a copyable version pin)
    - k8s/ (3 files)
    - CHANGELOG.md and crates/*/CHANGELOG.md (12 files)
    - MIGRATION.md
    - docs/src/api-reference/upgrading.md
    - docs/src/api-reference/migration-guide.md
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md

key-decisions:
  - "Pre-bump CI was read on SHA-P2 (not the plan's SHA-P, whose ci.yml and codeql.yml runs were cancelled); every run and job was success or skipped, so the bump started"
  - "The consumer statement carries the one exception M-B-50 and M-B-51 create to 'no serialized schema changed' (nine config types stop writing credential fields) and adds M-B-48 to M-B-52 to the maintainer's nine ids, as the 58-05 reconciliation note and the 58-15/58-16 hand-offs require"
  - "M-B-27 and M-B-36 are embedder-adjacent rows left off the pointer list and surfaced for the maintainer at the 58-08 re-read, rather than added by the executor"

requirements-completed: []

duration: "about 15 minutes (2026-11-05, 20:08Z to 20:21Z on the sandbox clock)"
completed: 2026-11-05
---

# Phase 58 Plan 07: The Three Bump Commits Summary

**Workspace lockstep bump to 0.11.1 held by the gate (52 listed strings, one-line `openapi.json` diff), twelve changelogs stamped 2026-11-05, and the SHIP-09 library-consumer statement in the maintainer's pointer-list wording naming `SHA-S2`, all as three consecutive commits; nothing pushed.**

## Commits

| # | Commit | Message | Label |
|---|--------|---------|-------|
| 1 | `96eb49d385770a90271372a82376a9016ac496dc` | chore(release): bump the workspace to 0.11.1 with every gated version string | `SHA-C1` |
| 2 | `d88c519abb556ef32ad6def7533a4851754597f0` | chore(release): stamp the 0.11.1 changelog sections | `SHA-C2` |
| 3 | `ac9411ea0039eea6a4f61b3328977228cd766253` | docs(release): state the v0.11.0 -> v0.11.1 library-consumer upgrade (SHIP-09) | `SHA-C3` |
| 4 | `8508b42f9` | docs(58-07): record the bump set (`.planning/`-only evidence commit) | none |

`STAMP` is `2026-11-05`. The three bump commits are consecutive and are the last code commits on the branch; the re-seal (plan 58-08) starts on `SHA-C3`.

## What was done

**Task 1 (tracer), commit 1.**
- Pre-bump CI on `SHA-P2` read per run and per job: `pre-commit` 42085634093, `feature-flags.yml` 42085634381 (21 jobs), `codeql.yml` 42085634162 and `ci.yml` 42085634184 (40 jobs: 38 success, 2 skipped) all success; the SDK-smoke job `127450272758`, Example Muster `127455524399` and Code Quality `127450272761` (shell guards) read success. The F-58-03 flake rule was not needed.
- `cargo release version 0.11.1 --execute --no-confirm --workspace` moved all 13 packages, `doc-examples` included (no hand edit); `paladin-ports` reads `=0.11.1`; `Cargo.lock` moved 13 entries and nothing else.
- Gate finding count: **52**, equal to RESEARCH (**42** class-1 Cargo pins, **1** class-2 `Current version` line, **8** class-3 `k8s/` labels and image tags, **1** class-4 `openapi.json`). The 51 non-`openapi.json` lines each held one `0.11.0` token and were edited line-addressed.
- `make openapi` diff, the one line: `-    "version": "0.11.0"` / `+    "version": "0.11.1"` (hunk `@@ -14 +14 @@`).
- Proved: gate `OK: 128 file(s) scanned ... '0.11.1'`; `make check-gates` exit 0; `openapi_matches_committed_baseline` ok; scoped zero check empty; `scripts/check-semver.sh`, `Makefile`, `ci.yml`, the ruleset, `.devcontainer` and `.project/current-exports.txt` unchanged (`SEMVER_BASELINE_VERSION` and the `Semver Checks (vs v0.11.0)` job name still read `0.11.0`). Pre-commit hooks ran and passed. The tracer's mechanical verify passed before expansion.

**Task 2, commit 2.** `make finalize-crate-changelogs VERSION=0.11.1`: `OK: 12 publishable package(s) checked for version '0.11.1' (date 2026-11-05)`. Each file has one `## [0.11.1] - 2026-11-05` under an empty `[Unreleased]`; `[0.11.0]` and older text byte-identical. `make check-changelogs` and `make check-release-consistency RELEASE_TAG=v0.11.1` OK. Root section: **16,065 characters (16,075 bytes)**, under 125,000, not condensed. The commit touches only the 12 changelogs.

**Task 3, commit 3 and the evidence commit.** The consumer paragraph as committed:

> A library consumer moves from `0.11.0` to `0.11.1` without changing source: no public Rust signature or type changed, and no serialized schema changed except that the credential fields of nine config types are no longer written when those types are serialised (`M-B-50`, `M-B-51`; loading is unchanged); section 11.2 has no row and `.cargo/semver-checks-allowlist.toml` has no entry; `scripts/check-semver.sh` on `0143b50d5` (`0143b50d596eee90a3b185c2e9ce94ad0292a2a8`, 2026-11-05) reads `0 failure(s)` for all twelve packages at `--release-type patch`, 223 checks each. Run-time behaviour changes that a library consumer can observe are in section 11.1: M-B-30, M-B-33, M-B-34, M-B-35, M-B-40, M-B-41, M-B-45, M-B-46, M-B-47, and the credential-class changes M-B-48 to M-B-52 (`M-B-50` supersedes the sentence in `M-B-36` that `Serialize` output is unchanged, and `M-B-52` changes the `Debug` text of two config types); `get_job` is unrouted but not deprecated (section 11.7).

- Sections 9 and 10 byte-identical to the parent; no `later v0.11.1` sentence remains (preamble, section 11 intro, 11.2 to 11.5, 11.7 and 11.8 rewritten to the released form).
- `upgrading.md` gained `## Upgrading from v0.11.0 to v0.11.1` (pointers to 11.8, the consumer subsection and 11.1; no rows copied) and its intro now counts three upgrades; `migration-guide.md:4` names v0.11.1.
- `make check-migration-allowlist`, `make check-gates`, `make check-doc-config` green.

## Marker substitutions (traceable)

| Plan text | Used | Value |
|-----------|------|-------|
| `SHA-P` | `SHA-P2` | `042df99b2c48bc249c206fdba6372e1e67b34e7d` |
| `SHA-S` / `### D-11 semver on SHA-S` | `SHA-S2` / `### D-11 semver on SHA-S2` | `0143b50d596eee90a3b185c2e9ce94ad0292a2a8`, 2026-11-05, 12 packages, 223 checks each, 0 failures |

The plan's Task 3 `verify` greps `- SHA-S:`; the equivalent was run on `0143b50d5` (the `SHA-S2` prefix) and printed OK. Its `git log -4 ... sed -n '2p;3p;4p'` index assumes the evidence commit is on top, so after the evidence commit the three release subjects sit at lines 2 to 4; they were checked before it as lines 1 to 3.

## Deviations from Plan

**1. [Marker substitution, orchestrator instruction] `SHA-P` and `SHA-S` read as `SHA-P2` and `SHA-S2`.** Recorded in Provenance and above. No other deviation from the plan's actions.

**2. [Reconciliation, plan-mandated at 58-05] The pointer-list wording was completed, not copied.** The "no serialized schema changed" clause carries the M-B-50/M-B-51 exception, and the id list gains M-B-48 to M-B-52. The maintainer's nine ids are unchanged. This is the correction the 58-05 note and the 58-15/58-16 hand-offs ask for, not a narrowing or broadening of the maintainer's scope sentence.

No Rule 1 to 4 fixes were needed and there was no authentication gate.

## Items for the maintainer or the 58-08 re-read

- **Embedder-adjacent rows not on the pointer list:** `M-B-27` (embedders that build their own `RunWorkerPool` wiring refuse to boot without the engine factory) and `M-B-36` (code that parses a secret out of a `Debug` string). Neither was on the maintainer's recorded list; whether a library consumer should be pointed at them is the maintainer's call.
- **Stale "open" comments, outside this plan's files:** the header comments of `scripts/check-migration-allowlist.sh` (line 71) and `tests/scripts/check-migration-allowlist_test.sh` (line 22) still describe section 11.2 as the open register. Behaviour is unaffected.
- Plan 58-08 replaces the consumer paragraph's `SHA-S2` read with its `SHA-R` read (MIGRATION section 11.8 evidence line), per the plan 58-08 text.

## Known Stubs

None. No stub, TODO or placeholder was added.

## Threat Flags

None. The plan adds no network endpoint, auth path, file access pattern or trust-boundary schema change; T-58-70 to T-58-74 are mitigated by the line-addressed edit, the unchanged-baseline and byte-identity checks and the one-line `openapi.json` diff.

## Self-Check: PASSED

- Commits `96eb49d38`, `d88c519ab`, `ac9411ea0` and `8508b42f9` resolve in `git log`; the three bump commits are consecutive and carry the plan's exact subjects (`chore(release)`, `chore(release)`, `docs(release)`).
- `58-CI-EVIDENCE.md` carries `- SHA-C1:`, `- SHA-C2:`, `- SHA-C3:` and `- STAMP:` lines; `git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax` still equals `df11b61c4` (nothing pushed).
- `make check-gates`, `make check-changelogs`, `make check-release-consistency RELEASE_TAG=v0.11.1` and `make check-migration-allowlist` read OK on the bump head.
