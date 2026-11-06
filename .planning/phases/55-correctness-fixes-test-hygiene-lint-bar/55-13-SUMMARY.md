---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 13
subsystem: phase-evidence
tags: [evidence, semver, windows-ledger, credential-review, carry-forward, fixes-01, fixes-02, fixes-03, fixes-05, fixes-06, tests-01, tests-02, tests-03, tests-04, tests-05, tests-06, cirel-08]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01 .. 55-12: the guard, the halt flip, the flake fix, the lint matrix, the base-layer fences, the FIXES-04 re-home, the rustdoc link fixes, the hermetic trace tests and boot warning, the release records"
provides:
  - "55-CORRECTNESS-EVIDENCE.md: one dated page with prove-then-lock rows for FIXES-01..06, TESTS-01..06, CIREL-08 and ten scout facts; the guard red run; the hermetic and flake proofs; the lint and rustdoc bars; FIXES-05 verification; the twelve-line semver record; the phase-head gate sweep; the manual credential-handling review; the carry-forward; known residuals"
  - "WINDOWS row 73 fixed and row 67 waived on re-run evidence (open 15 -> 13, waived 39 -> 40, fixed 33 -> 34, total 87)"
  - "one STATE.md blocker line pointing Phases 57 and 58 at the carry-forward"
affects: [phase 57 register sweep, phase 58 pre-tag audit, next minor]

tech-stack:
  added: []
  patterns:
    - "Every WINDOWS closure is preceded by a re-run of its closing test on the head, and the ledger is round-tripped through the tool's own parseLedger / renderLedger after a hand-set reason cell"
    - "CI-only gates (coverage floor, actionlint) are labelled CI-only and never recorded as passing"

key-files:
  created:
    - .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CORRECTNESS-EVIDENCE.md
  modified:
    - .planning/WINDOWS.md
    - .planning/STATE.md

key-decisions:
  - "PD-13-1 followed: rows 73 and 67 are dispositioned here; row 56 is not touched and its measurement (0 warnings under both bars for paladin-ai-core) is handed to Phase 57"
  - "PD-13-2 did not fire: make api-surface reported API surface unchanged, so no snapshot regeneration and no CHANGELOG line were needed"
  - "FIXES-04 is carried as re-homed, not complete: it is excluded from requirements.mark-complete"

requirements-completed: [FIXES-01, FIXES-02, FIXES-03, FIXES-05, FIXES-06, TESTS-01, TESTS-02, TESTS-03, TESTS-04, TESTS-05, TESTS-06, CIREL-08]

duration: ~45 min
completed: 2026-11-01
---

# Phase 55 Plan 13: Phase evidence, WINDOWS dispositions and carry-forward Summary

**Phase 55 closes on one measured evidence page: every requirement and scout fact has a before, a head re-run and a disposition; the twelve-line semver gate reads `223 checks: 223 pass, 31 skip` with `0 failure(s)` and nothing registered; the eleven local D-00a gates are green on the head; WINDOWS rows 73 (fixed) and 67 (waived) are closed on re-run evidence; Phases 57 and 58 inherit a measured carry-forward.**

## Plan-start SHA

`8202da48ccd718f91bf17f5a3bf67b092bb34162` (the 55-12 state-update commit). The phase-start SHA used for the "unchanged since the phase began" checks is `7f2eabe93cb32bccf3da1eb3121172a382193525` (the 55-01 plan start).

## Performance

- **Tasks:** 3 of 3 (Task 1 is the tracer; its `<verify>` passed end to end and the run was in auto mode, so execution expanded: Tracer verified end-to-end, expanding)
- **Commits:** `00426600b` (Task 1), `d8b2fdc52` (Task 2), `e620bd404` (Task 3)
- **Files:** 1 created (the evidence file), 2 modified (`.planning/WINDOWS.md`, `.planning/STATE.md`); no source file touched
- **Gate head:** the gates ran with the tree at `d8b2fdc529abb30b2c978fe7e16d35fe5c05d917`; `git diff --stat 8202da48c d8b2fdc52 -- . ':!.planning'` is empty, so the code under test is `8202da48c` exactly

## Accomplishments

- **Evidence file.** `55-CORRECTNESS-EVIDENCE.md` has eleven sections: Prove-then-lock (23 rows: 13 requirements and 10 scout facts), Guard red run, Hermetic and flake proofs, Lint and rustdoc bars, FIXES-05 verification, Semver, Phase-head gates, Manual credential-handling review, WINDOWS rows 73 and 67, Carry-forward, Known residuals. "Before" cells are copied from the twelve plan SUMMARYs; every "Head" cell was re-run in this session.
- **Guard (FIXES-01).** 55-01's real-tree red run (26 lines in 3 files) quoted verbatim; head run `OK: scanned 623 file(s) (19 test-only), 0 finding(s)`.
- **Hermetic and flake proofs (TESTS-01, TESTS-02).** With every proxy variable and `OPENAI_API_KEY` removed by `env -u`: `2 passed; 0 failed ... finished in 1.40s` (55-09's before: 2 failed in 20.74 s). 55-03's 20-of-20 loop carried. The unskipped facade lib binary: `1490 passed; 0 failed`, with `local_cancel_signals_token` and both trace tests passing.
- **Lint and rustdoc bars (TESTS-03, TESTS-04, CIREL-08).** The 13 `paladin-llm` clippy combinations under `-D warnings`: 13 exits of 0, 13 counts of 0 (55-04's before: 1, 5, 7, 4, 6, 8, 5, 5, 5, 7, 10, 5, 5). Default rustdoc bar `0` `warning:` lines; all-features `-D warnings` bar exit 0; zero rustdoc suppressions left; the five-crate before and after table (474 / 455 diagnostics to 0).
- **FIXES-05.** The three source lines re-read from `50.1-SEED-PROBE.md`, `51-EFFORT-PROBE.md` and `model_profile.rs`; the file ends with `FIXES-05 verified against both probe records, flag not wrong`.
- **Semver (D-13).** `./scripts/check-semver.sh` on the head, exit 0: twelve `semver: <pkg>: 223 checks: 223 pass, 31 skip` lines and `12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`. `.cargo/semver-checks-allowlist.toml` unchanged with zero entries; `MIGRATION.md` section 11.2 byte-identical to the phase start (header-only). `make api-surface`: `API surface unchanged` (4325 items); `grep -c HaltReason .project/current-exports.txt` prints `0` because `HaltReason` is in `paladin-ai-core`, which the facade snapshot does not list.
- **Phase-head gates (D-00a).** `cargo test --workspace --no-fail-fast` 62 result lines, 7803 passed, 0 failed, 229 ignored; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `make clean-code`; `make check-gates`; `make test-shell-guards` (18 harnesses, 650 assertions); `make security` (audit and deny); `make check-migration-allowlist`; `make check-version-strings`; `make check-doc-config` (161 blocks, 0 failed): all exit 0. The 82 % coverage floor and `actionlint` are recorded as CI-only (the `Coverage` and `Workflow Lint` jobs; neither tool is installed here).
- **WINDOWS.** Row 73: closing no-network run re-run (2 passed), `windows fixed 73`, reason cell set. Row 67: `unpriced_reachable_models_table` re-run (1 passed), `windows waive 67 "<reason>"` under B4. Counters: open 15 to 13, waived 39 to 40, fixed 33 to 34, total 87 unchanged; `git diff` touches the frontmatter and rows 67 and 73 only (table and JSON entry); the file loaded with `parseLedger` and re-rendered with `renderLedger` is byte-identical.
- **Carry-forward.** Phase 57: row 56 can close (`cargo doc -p paladin-ai-core --no-deps` prints no `warning:`, both bars green); rows 83 and 84 await CI `Workflow Lint`; rows 85-87 deferred to a `56.x` phase. Phase 58: six crate changelogs still lack an `[Unreleased]` / `[0.11.1]` anchor (battalion, eval, herald, llm, notifications, ports). Next minor: FIXES-04, the base-layer finish or retire options, the graph-node pricing check, widening `ModelProfile.reasoning`. STATE.md carries one blocker line pointing at the section.

## Task Commits

1. **Task 1 (tracer): evidence file, prove-then-lock, guard, hermetic and flake proofs, lint and rustdoc bars, FIXES-05** - `00426600b` (docs)
2. **Task 2: WINDOWS rows 73 fixed and 67 waived, carry-forward, STATE blocker** - `d8b2fdc52` (docs)
3. **Task 3: semver record, phase-head gates, credential review, known residuals** - `e620bd404` (docs)

## WINDOWS counters

| Counter | Plan start (`8202da48c`) | After |
|---------|--------------------------|-------|
| `open_count` | 15 | 13 |
| `waived_count` | 39 | 40 |
| `fixed_count` | 33 | 34 |
| `total_count` | 87 | 87 |

## Decisions Made

- Rows 73 and 67 are dispositioned in this plan because all their closing evidence is produced in this phase (PD-13-1); row 56 is left for Phase 57 with its measurement recorded.
- FIXES-04 is deliberately left out of `requirements-completed`: it is re-homed to the next minor (55-07), so it must not be ticked off.
- The herald-line observation (below) is recorded as a known residual rather than fixed, because this plan is evidence-only and a code change would have invalidated the phase-head measurements.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The state tool corrupted an unrelated STATE.md line**
- **Found during:** Task 2, step 5 (`state add-blocker`)
- **Issue:** after the call, an existing line ending `` `Policy::none()` `` read `` `Policy::()` `` (the word `none` was dropped); the diff showed it next to the intended blocker line.
- **Fix:** restored the text with a single-line `sed` before committing; `git diff` for STATE.md now shows only the `last_updated` stamp and the new blocker line. Later state commands in this plan's close-out are checked the same way.
- **Files modified:** `.planning/STATE.md`
- **Commit:** `d8b2fdc52`

### Plan-mandated manual edit

**2. Row 73's reason cell set by hand**
- `windows fixed <id>` takes no reason argument, but the plan requires the reason cell to cite the two test names and the evidence file. After `windows fixed 73` the reason was set in both places the ledger stores it (the table row and the JSON entry), as rows 58, 62 and 63 were closed before. The result round-trips byte-identically through the tool's `parseLedger` and `renderLedger`, and `windows status` reports consistent counts. Row 67 was waived entirely through `windows waive`.

### Notes (no outcome impact)

- **Commit trailers.** Commits carry the attribution lines the harness supplied for this session (a different model name from the one the orchestrator's note named, the same session URL line), as plan 55-12 also recorded. No model identifier appears in any code, comment, doc or planning artifact.
- **Semver run.** `./scripts/check-semver.sh` ran with `SEMVER_PRUNE_LOCAL_CACHE=1` to protect the 9 GB disk; no `target/` file was deleted by hand, `CARGO_INCREMENTAL=0` was set throughout, and free space never fell below 5.2 GB.
- **PD-13-2** did not fire: no API-surface drift, so no `make api-surface-update` and no CHANGELOG line.
- **Observation, not a defect fixed here.** The `HaltReason::ModelUnpriced` herald line interpolates the model name without `escape_debug`, unlike the boot warning. In the ordinary path the name is an operator-priced model, and the line goes to Herald output (a JSON string value), not a log call. Recorded under `## Known residuals` in the evidence file.

## Known Stubs

None. This plan wrote an evidence file and two ledger rows and added no code; no stub, skipped test or unrun `<verify>` arose that is not already recorded as CI-only (the coverage floor and `actionlint`, WINDOWS rows 83 and 84 for the latter). No new `.planning/WINDOWS.md` row was appended.

## Threat Flags

None. No endpoint, auth path, file-access pattern or schema change was introduced; the manual credential-handling review of the phase's new messages found no key, tenant or backend text in any of them.

## Self-Check: PASSED

- FOUND: `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CORRECTNESS-EVIDENCE.md` (11 sections), `.planning/WINDOWS.md`, `.planning/STATE.md`.
- FOUND: commits `00426600b`, `d8b2fdc52`, `e620bd404` in `git log`.
- All three tasks' `<verify>` commands were run and passed after their commits; the evidence file contains no key-shaped string and no `snyk` mention.
