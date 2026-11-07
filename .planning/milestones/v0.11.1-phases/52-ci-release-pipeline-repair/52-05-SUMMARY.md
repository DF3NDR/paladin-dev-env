---
phase: 52-ci-release-pipeline-repair
plan: 05
subsystem: infra
tags: [ci, cargo-semver-checks, migration-register, shell-guards, release-gates]

requires:
  - phase: 52-01
    provides: "scripts/check-semver.sh with --package and SEMVER_PRUNE_LOCAL_CACHE=1; the allowlist gate running from the required License & Dependency Policy job"
provides:
  - "No crate-wide cargo-semver-checks allow remains in any manifest (28 lines, six tables, measured redundant at 0.11.0)"
  - "52-CI-EVIDENCE.md: twelve per-package check lines with no allow in force"
  - "MIGRATION.md part 11 (v0.11.0 to v0.11.1) with an empty X-10 register; parts 10 and 9 frozen byte-for-byte"
  - "check-migration-allowlist.sh reading only the §11.2 window (terminator: the §11.3 heading); allowlist emptied"
affects: [52-06, 52-07, 53, 55, ruleset-contexts]

tech-stack:
  added: []
  patterns:
    - "Register window with a mandatory terminator heading, pinned by a harness case that omits it"
    - "Harness proves it wrote nothing with a BEFORE/AFTER snapshot (git status plus checksums), not a clean-tree assertion"

key-files:
  created:
    - .planning/phases/52-ci-release-pipeline-repair/52-CI-EVIDENCE.md
  modified:
    - Cargo.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-battalion/Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-web/Cargo.toml
    - MIGRATION.md
    - scripts/check-migration-allowlist.sh
    - tests/scripts/check-migration-allowlist_test.sh
    - .cargo/semver-checks-allowlist.toml
    - Makefile

key-decisions:
  - "PD-05-1: removed all six crate-wide tables (28 lines), not only the two CIREL-02 names; the measurement is the deliverable"
  - "PD-05-2: window, script, harness and allowlist header moved in one commit so set-equality holds on the real tree"
  - "PD-05-3: harness closing check is a BEFORE/AFTER snapshot, with checksums added to the git status line so an in-place rewrite of an already-modified file is still caught"
  - "PD-05-4: no ADR; .planning/decisions/PROMOTION.md untouched"

patterns-established:
  - "Measure-then-remove: a suppression is deleted only after the tool is run with it absent and every package shows a non-zero, all-pass check count"

requirements-completed: [CIREL-01, CIREL-02, CIREL-03]

coverage:
  - id: D1
    description: "All six crate-wide cargo-semver-checks lints tables (28 allow lines) removed; every manifest still parses"
    requirement: "CIREL-02"
    verification:
      - kind: other
        ref: "grep for cargo-semver-checks.lints over Cargo.toml and crates/*/Cargo.toml (0 hits); cargo metadata --no-deps; python3 tomllib load of each manifest"
        status: pass
    human_judgment: false
  - id: D2
    description: "All twelve packages evaluate lints and pass at the 0.11.0 baseline at --release-type patch with no allow in force"
    requirement: "CIREL-01"
    verification:
      - kind: integration
        ref: "SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package <pkg> x12; 52-CI-EVIDENCE.md (223 checks: 223 pass, 31 skip each)"
        status: pass
    human_judgment: false
  - id: D3
    description: "MIGRATION.md opens part 11 with an empty §11.2 register; §10 and §9 are byte-identical to the previous commit"
    requirement: "CIREL-03"
    verification:
      - kind: other
        ref: "python3 byte-identity assertion from '# 10. Upgrading from v0.10.1 to v0.11.0' to EOF against git show HEAD~1:MIGRATION.md; grep -c TBD MIGRATION.md = 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Gate script reads only §11.2 up to §11.3; harness cases 1-10, 10a/10b, 12a-12f, real-tree zero-pair case and BEFORE/AFTER snapshot"
    requirement: "CIREL-03"
    verification:
      - kind: unit
        ref: "tests/scripts/check-migration-allowlist_test.sh (20 assertions); make test-shell-guards (all harnesses green)"
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-10-28
status: complete
---

# Phase 52 Plan 05: Remove crate-wide semver allows and open the §11.2 register Summary

**All 28 crate-wide cargo-semver-checks allows are gone and measured redundant (twelve packages, 223 checks each, all pass at 0.11.0, `--release-type patch`), and the allowlist gate now reads only an empty MIGRATION.md §11.2 against an empty allowlist, with the frozen §10 and §9 records untouched.**

## Performance

- **Duration:** about 14 min of wall clock (23:40 to 23:54 UTC)
- **Tasks:** 2 of 2 (Task 1 was the tracer)
- **Files:** 1 created, 11 modified

## Accomplishments

- CIREL-02 / D-02: the `[package.metadata.cargo-semver-checks.lints]` table and the justifying comment block above it were deleted from `Cargo.toml` (1 allow), `paladin-core` (11), `paladin-battalion` (2), `paladin-llm` (4), `paladin-ports` (6) and `paladin-web` (4): 28 lines, 327 deleted lines in all, no added line. `git diff -U0` showed only comments, the six headers, six separating blank lines and the 28 allow lines removed.
- CIREL-01 / D-02: `52-CI-EVIDENCE.md` records the exact command, `cargo-semver-checks 0.50.0`, `rustc 1.97.1`, the HEAD SHA and one check line per package. Every one of the twelve printed `223 checks: 223 pass, 31 skip`. Lints that fired: none.
- CIREL-03 / D-03: `MIGRATION.md` gained `# 11. Upgrading from v0.11.0 to v0.11.1` above the frozen part, headings `## 11.1` to `## 11.8` with the exact §10.x heading texts, an empty §11.2 table (the §10.2 header and separator rows, no data row), and a scope note saying three parts, newest first. No placeholder token appears.
- `scripts/check-migration-allowlist.sh` reads `## 11.2 ` up to `## 11.3 `; every echo and `::error::` string names §11.2; the register-side comment explains that §10.2 and §9.2 are frozen records of breaks 0.11.0 already contains and that the `## 11.3 ` terminator must exist. `.cargo/semver-checks-allowlist.toml` holds zero entries and a header starting `EMPTY AS OF PHASE 52`. `make check-migration-allowlist` help text names §11.2.

## Task Commits

1. **Task 1: Measure, then remove all six crate-wide allow tables (tracer)** - `2d3cf119a` (chore)
2. **Task 2: Open MIGRATION.md section 11 and retarget gate, harness and allowlist** - `e9aaa5965` (feat)

Metadata commit: see git log (docs(52-05)).

## Tracer evidence (Task 1)

The executor-inserted tracer feedback gate was not stopped on (unattended run, per the run's checkpoint policy); its evidence is recorded here. The plan's own D-02 stop rule (a decision checkpoint if any lint fires) did not trigger: none fired.

- Command per package: `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package <pkg>`, which runs `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch`.
- Result for all twelve (`paladin-ai-core`, `paladin-ports`, `paladin-herald`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage`, `paladin-eval`, `paladin-ai`): `223 checks: 223 pass, 31 skip`, exit 0, `0 failure(s)`.
- Tracer `<verify>` (the plan's Task 1 automated command) passed: no `cargo-semver-checks.lints` string in any manifest, `cargo metadata --no-deps` parses, twelve measurement rows, no failing lint, `make check-gates` exit 0.

### Disk readings (`df -h /`, free space)

| Point | Free |
|---|---|
| Before the first package | 575 MB |
| After `paladin-ai-core`, `paladin-ports`, `paladin-herald`, `paladin-battalion`, `paladin-llm`, `paladin-memory` (each) | 575 MB |
| After the first `paladin-web` attempt (failed, see Deviations) | 575 MB (the failed build's `local-*` dir was pruned) |
| After removing the stale 0.9.0 baseline dir | 1.2 GB |
| After the retried `paladin-web` and each of `paladin-notifications`, `paladin-content`, `paladin-storage`, `paladin-eval`, `paladin-ai` | 1.2 GB |
| End of plan | 1.2 GB |

Free space never fell under 575 MB at a package boundary. I measured per package rather than in one all-packages call: the script's per-package loop body, command and parsing are identical in both modes, and the per-package form let disk be read after every package.

## Red-first harness run (Task 2)

With the harness retargeted and `scripts/check-migration-allowlist.sh` and `MIGRATION.md` still on the old `(9|10)` window, the harness exited 1 with `11/20 assertion(s) failed` (cases 1, 5, 6, 10, 10a, 10b, 12a, 12b, 12e, the real-tree shape case 12f, and the real-tree zero-pair case 11). After the window change, the §11 part and the emptied allowlist it exits 0 with `20 assertion(s) passed`. Case 12d passed on both runs (the old window also read §10.2); it pins the missing-terminator behaviour only once the window is §11.2, and it also asserts the §10.2 pair is named in the output.

## Verification (plan level)

- `./tests/scripts/check-migration-allowlist_test.sh`: 20 assertions passed. `./scripts/check-migration-allowlist.sh` on the real tree: exit 0, both pair lists empty, crate-wide-allow mirror line green.
- The plan's full automated verify for Task 2 (including the byte-identity assertion from `# 10.` to EOF and the empty `## 11.2` to `## 11.3` window) passed.
- `make check-gates` exit 0; `make test-shell-guards` exit 0 (all harnesses); `shellcheck --severity=warning` clean on the script and its harness.
- `grep -c '^## 11\.[1-8] '` is 8; `grep -c '(9|10)'` in the script is 0; `.planning/decisions/PROMOTION.md` unchanged.

## Decisions Made

Followed PD-05-1 to PD-05-4 as planned (see `key-decisions`). One refinement within PD-05-3: the BEFORE/AFTER snapshot also records a `cksum` of the two register files next to the `git status --porcelain` line. A porcelain line alone reads ` M` before and after if the harness rewrote an already-modified file, so the checksum is what makes the check meaningful while the register is mid-edit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Out-of-disk failure of the first `paladin-web` measurement**
- **Found during:** Task 1, measurement step
- **Issue:** the rustdoc build of the `paladin-web` 0.11.0 baseline (large dependency graph) exhausted the 575 MB of free space: `No space left on device`, a linker bus error in `rust-embed-impl`, then a failed `rmeta` write. `scripts/check-semver.sh` reported `paladin-web: lint failure (exit 101)`. No lint fired; no check line was produced.
- **Fix:** made the one attempt the run's rules allow: removed the stale, unused 0.9.0 baseline cache dir `target/semver-checks/registry-paladin_web-0_9_0-*` (about 571 MB). Free space rose to 1.2 GB. Retried only `paladin-web`, which then printed `223 checks: 223 pass, 31 skip` with no `No space left` text in its log. The remaining five packages ran on the same footing.
- **Files modified:** none tracked (build cache under `target/`)
- **Commit:** n/a (recorded in `52-CI-EVIDENCE.md`)

The script's failure label `lint failure (exit 101)` is misleading for a build error; not changed here (out of scope), noted for whoever next touches `check-semver.sh`.

---

**Total deviations:** 1 (Rule 3). **Impact:** none on the plan's result.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or trust-boundary file access. T-52-19 (28 allows removed and measured; the crate-wide-allow mirror fails any future table without an entry), T-52-20 (the `## 11.3 ` terminator pinned by cases 12d/12f, frozen registers by 12a/12b, out-of-window rows by 12c), T-52-21 (byte-identity of §10/§9, placeholder-token count 0) and T-52-22 (`SEMVER_PRUNE_LOCAL_CACHE=1`, disk read after every package) are mitigated as planned.

## Next Phase Readiness

- Any public-API break in Phases 53 to 57 must add a `Y` row under `## 11.2` and an allowlist entry in the same commit, or the required `License & Dependency Policy` job fails.
- The semver job now silences nothing crate-wide; a break in 53 or 55 fails it red at patch level, which plan 52-07 proves on a scratch branch.
- `actionlint` and the pushed-branch CI run remain the confirmation for the workflow edits from 52-01; this plan edited no workflow file.

## Self-Check: PASSED

Files verified present: `52-CI-EVIDENCE.md`, `MIGRATION.md`, `scripts/check-migration-allowlist.sh`, `tests/scripts/check-migration-allowlist_test.sh`, `.cargo/semver-checks-allowlist.toml`, `Makefile`, and the six manifests. Commits `2d3cf119a` and `e9aaa5965` verified in git log.
