---
phase: 47-v0-11-0-crate-release
plan: 04
subsystem: release
tags: [release, re-seal, acceptance-audit, sign-off, d-00j, ci-attribution]

requires:
  - phase: 47
    provides: "47-03 build wave (SHA-H c9456846; 0.11.0 bump, stamp, OpenAPI baseline) and the 47-01 pre-build decision (B1=a B2=a R7=a EMPTY=a SPACE=a DATE=2026-10-25)"
provides:
  - "47-ACCEPTANCE-AUDIT.md: the v0.11.0 re-seal table (eight rows, six-column form) measured on SHA-R d82d67ca, closing paragraphs, a 21-line Findings list and the UNTICKED sign-off box"
  - "47-CI-EVIDENCE.md Local sweep rows 49-68, the D-12 CI-evidence-only entries, and Findings entries for the two D-00j halts and the two maintainer decisions"
affects: [47-05, 47-06, 47-07, 47-08, 47-09]

tech-stack:
  added: []
  patterns:
    - "D-00j: a red row halts the plan with nothing fixed; a maintainer-gated decision attributes the leg to its CI twin, never to a local pass"
    - "evidence is append-only: red rows stay as written and a later row records the decision"

key-files:
  created:
    - .planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md
    - .planning/phases/47-v0-11-0-crate-release/47-04-SUMMARY.md
  modified:
    - .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md

key-decisions:
  - "Row 4 (maintainer, 2026-10-25T19:45:29Z, verbatim `1`): the paladin-web semver-checks line is CI-attributed to `Semver Checks (vs v0.9.0)`; no local accommodation authorised"
  - "Row 6 (maintainer, 2026-10-25 between 20:00Z and 20:04Z, option 1 gated on CI, fallback option 4): the `make test` leg is CI-attributed to `Unit Tests (stable)`; gate met on the job conclusion (run 40228639490 job 121275766339 success)"
  - "The sign-off box stays unticked; no agent ticks it (D-00a)"

requirements-completed: []

duration: "this continuation about 45m (20:21Z to about 21:05Z); earlier segments are timestamped in their commits"
completed: 2026-10-25
status: complete
---

# Phase 47 Plan 04: v0.11.0 re-seal and sign-off section Summary

**The v0.11.0 re-seal section exists on SHA-R `d82d67ca` with eight measured rows, two of which are honestly CI-attributed (row 4 `paladin-web`, row 5 MSRV) and one of which is mixed (row 6: its `make test` leg is RED locally and CI-attributed); `make security` and the shell guards are green, and the sign-off box waits unticked for the maintainer.**

## Result

- `SHA-R`: `d82d67cac926412a21cccd72278fe8fe9df672c0`. Code head `SHA-H`: `c945684647c4f9c78fdde2fbe2fc8fc2e8806106`. `git diff --name-only SHA-H SHA-R -- . ':!.planning' ':!.project'` is empty.
- Row results at `SHA-R`:

| # | Gate | Result |
|---|------|--------|
| 1 | MIGRATION no-TBD + allowlist set-equality | PASS (`COUNT=0`; 56 pairs both sides) |
| 2 | `v0_9_config_boot` | PASS (9 passed) |
| 3 | `openapi_golden_v0_9` | PASS (9 passed; baseline lib test 1 passed) |
| 4 | `cargo semver-checks` vs 0.9.0, 11 crates | 10 of 11 PASS locally (known no-op, D-09); `paladin-web` CI-attributed (maintainer decision), not a local pass |
| 5 | MSRV 1.88 | not run locally, CI-attributed (`MSRV (Rust 1.88)`) |
| 6 | `make publish-dry-run` | mixed: `clean-code`, doc tests (681 passed, 0 failed, 210 ignored), `make audit` (9 allowed warnings), `make build-release` and `cargo publish --workspace --dry-run` pass locally; the `make test` leg is RED locally (WINDOWS row 73) and CI-attributed to `Unit Tests (stable)` (run 40228639490 job 121275766339 success). The chain as one command was never green locally |
| 7 | `make check-gates` + R7 greps + B-1 size | PASS (nine guards OK; release body 108926 bytes) |
| 8 | `make check-publish-order` | PASS (12 crates, full OK line recorded) |

- Twelve-crate order emitted by `cargo publish --workspace --dry-run`, `Packaging` pass: paladin-ai-core, paladin-ports, paladin-battalion, paladin-llm, paladin-content, paladin-storage, paladin-eval, paladin-herald, paladin-memory, paladin-notifications, paladin-web, paladin-ai. Dry-run `Uploading` order: paladin-ai-core, paladin-ports, paladin-battalion, paladin-herald, paladin-llm, paladin-notifications, paladin-storage, paladin-web, paladin-content, paladin-eval, paladin-memory, paladin-ai. Both are cargo-chosen topological orders and differ from the `CRATES` array in `scripts/publish-crates.sh`, whose order is the one row 8 proves.
- Task 3: `make security` exit 0 (9 allowed audit warnings, `advisories ok, bans ok, licenses ok, sources ok`; B-2 cleared); `make test-shell-guards` exit 0 (14 harnesses, 347 assertions). The three D-12 CI-evidence-only entries (`API Surface Tracking`, `Consumer-Surface Muster`, `Coverage`) are recorded for plan 47-06. Coverage has no local figure and none is expected.
- The unticked sign-off box is at line 62 of `47-ACCEPTANCE-AUDIT.md` (`- [ ] **The `v0.11.0` tag may be cut**`). Plan 47-06 names its edit by that line. No ticked marker exists in the file.
- Capacity events: none. Two `cargo clean` runs under SPACE=a, both pre-emptive headroom judgements, not out-of-space aborts: one before the first row-6 run (`Removed 25948 files, 10.4GiB total`), one before the direct `make build-release` leg (`Removed 24983 files, 9.9GiB total`). No leg was re-run.

## Commits

| Task | Commit | Content |
|------|--------|---------|
| 1 (tracer) | `c42410e8` | Mint `47-ACCEPTANCE-AUDIT.md` at SHA-R; rows 1, 7, 8 |
| 2 (partial) | `c3aa399f` | Rows 2-3 PASS; red row 4 (D-00j halt) |
| 2 (partial) | `d900b21b` | Row 4 maintainer decision, row 5 CI-attributed, red row 6 (D-00j halt) |
| 2 (complete) | `72af77e2` | Row 6 decision, CI attribution and the four legs run directly (Local sweep rows 61-65) |
| 3 | `342aeac2` | House sweep, `make security`, shell guards, D-12 entries, closing paragraphs, Findings, unticked box |

## Deviations from Plan

### D-00j halts (plan behaviour, not auto-fixes)

**1. [Row 4 red] `paladin-web` semver-checks cannot build its 0.9.0 baseline in this sandbox**
- **Found during:** Task 2, row 4.
- **Issue:** the baseline's `utoipa-swagger-ui 9.0.2` build script downloads from `github.com`; the sandbox egress proxy answers with a deterministic HTTP 403 policy denial. Exit 101, no semver verdict.
- **Handling:** plan halted, nothing fixed or re-run. Maintainer decision `1`: attribute to the CI twin `Semver Checks (vs v0.9.0)`, no local accommodation. Recorded in Local sweep rows 57-58 and two Findings entries.
- **Commits:** `c3aa399f`, `d900b21b`.

**2. [Row 6 red] `make test` fails two `run_api_wiring` tests in `paladin-ai --lib`**
- **Found during:** Task 2, row 6 (`make publish-dry-run`).
- **Issue:** `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` panic `the run never reached a terminal status` (1457 passed, 2 failed). Same two tests are WINDOWS row 73 (open since 2026-10-15; reproduced at the pre-Phase-50 commit `28b6a69b`, sandbox-specific). Cause undetermined.
- **Handling:** plan halted, nothing fixed or re-run. Maintainer decision, verbatim: "agree to your recommendation to option one, gated on the CI result, falling back to 4 if that result is red." Gate met on the job conclusion (`Unit Tests (stable)` success on run 40228639490). The per-test log line was not readable (run `in_progress`; blob log host denied by the proxy), so the CI twin is cited on the job and step conclusions only.
- **Commits:** `d900b21b`, `72af77e2`.

### Auto-fixed Issues

None. No file outside `.planning/` was changed; no gate was fixed or retried.

### Plan-text drift

- Task 2's commit message in the plan (`docs(47-04): record re-seal rows 2-6`) was split across three commits because of the two halts.
- The plan's row-6 definition is a single chain; it ran as the chain's first leg (red) plus four legs run directly after the maintainer's decision, in the Makefile's order, after a second pre-emptive `cargo clean`. The audit states this and does not present row 6 as a local pass.

## Auth gates

None.

## Known Stubs

None. No source file was created or modified.

## Threat Flags

None. Only `.planning/` documents changed.

## Open items carried forward

- Plan 47-06 must fill the final CI twins (rows 2, 3, 4 `paladin-web`, 5, 6 test leg, 7, 8 and the three D-12 entries including the `Coverage` figure) from the PR run on the pushed head and re-read the per-test log for the two `run_api_wiring` tests once run 40228639490 (or its successor) is complete.
- The sign-off box is the maintainer's to tick; SHIP-07 is not marked complete here (the requirements table row closes in plan 47-09 after the tag).
- WINDOWS row 73 stays open; this plan did not edit `.planning/WINDOWS.md`.

## Self-Check

## Self-Check: PASSED

All three phase files exist; commits `c42410e8`, `c3aa399f`, `d900b21b`, `72af77e2` and `342aeac2` are in `git log`; the sign-off box is unticked; `git diff --name-only d82d67ca HEAD -- . ':!.planning' ':!.project'` is empty.
