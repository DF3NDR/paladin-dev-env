---
phase: 47-v0-11-0-crate-release
plan: 02
subsystem: release
tags: [release, changelog, cargo-deny, rustsec, tokio, release-body]

requires:
  - phase: 47
    provides: "47-01 pre-build decision (B1=a B2=a R7=a EMPTY=a SPACE=a DATE=2026-10-25) and the reproduced B-1 and B-2 blockers"
provides:
  - "B-2 cleared: Cargo.lock tokio 1.52.3 -> 1.53.3, cargo deny check exit 0, cargo audit exit 0"
  - "B-1 cleared: root CHANGELOG.md [Unreleased] body 156528 -> 108926 bytes (limit 110000), full text one git show away"
  - "D-08 authorisation entry in 47-CI-EVIDENCE.md (reply verbatim, one effect line per item)"
  - "B-1 condensation ledger (13 chunk rows) and the maintainer-read request"
affects: [47-03, 47-04, 47-05, 47-06]

tech-stack:
  added: []
  patterns:
    - "chunked condensation with a per-chunk mechanical re-diff (ids, numeric figures, code spans) against a recorded pre-condensation SHA"
    - "remedy commits precede the version bump so the bump commit set stays pure (D-05, D-08)"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-02-SUMMARY.md
  modified:
    - Cargo.lock
    - CHANGELOG.md
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md

key-decisions:
  - "Applied B2=a and B1=a exactly as recorded in 47-01-SUMMARY.md; no other remedy, no script or workflow edited"
  - "Tightened Added as well as Changed in the Task 3 commit because the body was 116,534 bytes after Changed alone (plan Task 3 step 1 allows this); the six preserved subsections were never touched"

requirements-completed: []

duration: ~2h 20m
completed: 2026-10-25
status: complete
---

# Phase 47 Plan 02: Clear the two pre-build blockers Summary

**tokio bumped to 1.53.3 in the lockfile only (RUSTSEC-2026-0386 gone, `cargo deny check` green) and the root `[Unreleased]` condensed from 156,528 to 108,926 bytes in two ledgered commits, with every break, id and the pre-0.10.1 history untouched.**

## Pre-build decision applied

Read from `47-01-SUMMARY.md` `## Pre-build decision` (reply received 2026-10-25T18:19:21Z, "All (a) choices", `DATE=Today`): `B1=a B2=a R7=a EMPTY=a SPACE=a DATE=2026-10-25`. No hold, no `B1=c`, no `B2=c`, so the plan ran. The reply is quoted verbatim, with one effect line per item, in a dated `Maintainer's pre-build decision (47-01)` entry in `47-CI-EVIDENCE.md` (the D-08 authorisation). R7, EMPTY, SPACE and DATE are consumed by plans 47-03 to 47-07, not here.

## Commits (all on `claude/laughing-dirac-e0h2ax`, nothing pushed)

| # | SHA | Subject |
|---|-----|---------|
| 1 | `0e1f7958976b84ac92cac7cd0a965b44c1359463` | `fix(deps): bump tokio to 1.53.3 for RUSTSEC-2026-0386 (lockfile only)` (B-2; `Cargo.lock` only) |
| 2 | `700a92d86f3d6b2e24bbd2c163daec7c307297e0` | `docs(47-02): record the B-2 remedy` (decision entry, Local sweep rows 19-23) |
| 3 | `adf15d799bd835ea3a0a8792eae8e43e3b343d39` | `docs(changelog): condense the root [Unreleased] Added subsection ... (B-1, 1 of 2)` |
| 4 | `06f3ed47` | `docs(47-02): record the B-1 Added condensation ledger` (provenance lines, A rows) |
| 5 | `677b6ea3655eb11229ed44b19e334b9f12e1ec08` | `docs(changelog): condense the root [Unreleased] Changed subsection ... (B-1, 2 of 2)` |
| 6 | `ea9d3810` | `docs(47-02): record the B-1 remedy` (C rows, measurements, maintainer-read request) |

Provenance lines in the evidence: `- SHA-PRE-B1: 700a92d86f3d6b2e24bbd2c163daec7c307297e0` and `- B1-MODE: a`.

## B-2: lockfile package changes

`cargo update -p tokio --precise 1.53.3`; `git diff --name-only` was exactly `Cargo.lock`.

- **Package version change: `tokio` 1.52.3 -> 1.53.3** (checksum `8fc7f01b...` -> `ce3335fa...`). No package added or removed.
- Seven Windows-only dependency edges were re-pointed by the resolver to `windows-sys` entries already in the lockfile: `errno` 0.3.14, `is-terminal` 0.4.17, `quinn-udp` 0.5.16, `rustix` 1.1.4, `rustls-platform-verifier` 0.7.0 and `tempfile` 3.27.0 went from `windows-sys 0.61.2` to `0.52.0`; `nu-ansi-term` 0.50.3 went from `0.61.2` to `0.59.0`. None affects a Linux build; recorded in Local sweep row 19.
- Gates on the remedy tree: `cargo deny check` exit 0 (`advisories ok, bans ok, licenses ok, sources ok`), `cargo audit` exit 0 (9 allowed warnings, down from 10; `RUSTSEC-2026-0386` gone), `cargo metadata --locked --offline` exit 0, `make check-gates` exit 0.

## B-1: root `[Unreleased]` condensation

| Quantity | Before (`SHA-PRE-B1`) | After |
|---|---|---|
| root `[Unreleased]` body | 156,528 bytes | **108,926 bytes** (limit 110,000; 1,074 bytes of margin) |
| `### Added` | 84,557 bytes, 57 top-level bullets | 46,648 bytes |
| `### Changed` | 36,407 bytes, 44 top-level bullets | 26,374 bytes |
| six preserved subsections | 35,445 bytes | byte-identical (`cmp`, all six) |
| lines at or below `## [0.10.1]` | -- | identical; highest pre-image line touched is 1418 (heading at 1852) |
| `M-B-`/`DR-`/`ADR-` ids | 43 distinct | 43 distinct, none missing |
| `###` headings in the body | 8 | 8, none missing |

- **Ledger chunk counts:** `Added` 9 chunks (`A1`-`A9`, 57 source bullets); `Changed` 4 chunks (`C1`-`C4`, 44 source bullets). Each chunk was re-diffed (ids of its source lines, numeric-figure subset, code-span subset) before the next was written; the Task 3 tightening pass re-ran the re-diff over every chunk and the ledger's `Re-diff` column is the result of that final run.
- All four `BREAKING` items in `Changed`, the one-way persisted-JSON warning (back up, readers before writers) and every `Migration:` line are kept. The Phase 43 pacing known limitations (six items, only in the `Added` text) are kept in full.
- `make check-changelogs` exit 0 and `make check-gates` exit 0 after each commit.

## Maintainer read requested (B1=a)

`- B-1 maintainer read: requested` is in the ledger section of `47-CI-EVIDENCE.md`. The figure and code-span checks are a mechanical proxy that catches a new number or identifier, not a changed meaning. Please read the condensed root `[Unreleased]` against the ledger (and, where in doubt, `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md`) and confirm it adds no claim and loses no break, at the latest at the 47-06 sign-off. This did not pause the plan.

## Deviations from Plan

### Auto-fixed Issues

None of Rules 1-3 beyond the following plan-sanctioned or minor points.

**1. [Plan-sanctioned tightening] `Added` rewritten again in the Task 3 commit**
- **Found during:** Task 3
- **Issue:** after `Changed` was condensed at the first-pass density the body measured 116,534 bytes, over 110,000.
- **Fix:** shortened condensed bullets in `A2`-`A8` and `C1`-`C3` (name lists, per-model enumerations, public-surface lists), per Task 3 step 1; re-ran the re-diff over both subsections. The commit body says `Added` was tightened too.
- **Files modified:** `CHANGELOG.md`
- **Commit:** `677b6ea3`

**2. [Target not met] the "about 60,000 bytes" aim for `Added` plus `Changed`**
- `Added` plus `Changed` ended at 73,022 bytes (60 percent of the original 120,964). The hard limit (body at most 110,000) holds; going lower would have dropped per-capability detail. Margin is 1,074 bytes under the limit, and the real caps (125,000 characters, 131,072-byte argv) are above it.

**3. [Order] the decision entry was written after the lockfile edit but before any commit**
- The plan's step 1 puts the entry first. The lockfile was edited in the working tree first (nothing committed); the entry and the remedy were committed as the plan's step 4 orders (remedy commit, then evidence commit). No change reached history before the authorisation entry.

## Auth gates

None.

## Known Stubs

None. This plan edits a lockfile, a changelog and an evidence record; no code path or UI.

## Threat Flags

None. No new network endpoint, auth path or schema change; the only dependency change is the existing `tokio` pin moving to a patched release (T-47-07, T-47-SC mitigated: `--precise` on one package, no package added, `cargo deny` bans and sources green).

## Self-Check: PASSED

- `Cargo.lock` contains `name = "tokio"` followed by `version = "1.53.3"`: FOUND.
- `CHANGELOG.md` root `[Unreleased]` body 108,926 bytes: FOUND.
- `47-CI-EVIDENCE.md` carries `Maintainer's pre-build decision (47-01)`, `- SHA-PRE-B1:`, `- B1-MODE: a`, the A1-A9 and C1-C4 ledger rows and `- B-1 maintainer read: requested`: FOUND.
- Commits `0e1f7958`, `700a92d8`, `adf15d79`, `06f3ed47`, `677b6ea3`, `ea9d3810` exist: FOUND.
- Task 1, Task 2 and Task 3 automated verifies exited 0. Nothing pushed (`HEAD` is not an ancestor of `origin/claude/laughing-dirac-e0h2ax`).
