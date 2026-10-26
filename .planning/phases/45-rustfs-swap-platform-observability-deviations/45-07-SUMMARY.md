---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 07
subsystem: observability
tags: [criterion, bench, trace-sink, obs-05, windows-ledger, d-19]

requires:
  - phase: 45-rustfs-swap-platform-observability-deviations
    provides: "45-03 measurement points A, B, C and the 45-BENCH-EVIDENCE.md protocol"
provides:
  - "45-BENCH-EVIDENCE.md filled: machine block, raw criterion output at A, B, C (plus a corroborating second C run), two results tables, delta commentary, Verdict: AMEND"
  - "WINDOWS.md row 61 amending row 35, left OPEN for the maintainer's UAT acceptance"
  - "observability.md Known limitations, CHANGELOG [Unreleased] and PROJECT.md carry the +19.36 %/+16.19 % figure"
affects: [phase-45-uat, gsd-ship]

tech-stack:
  added: []
  patterns:
    - "Raw maintainer measurement files committed beside the evidence file they support"

key-files:
  created:
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-07-machine-block.txt
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-07-bench-A.txt
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-07-bench-B.txt
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-07-bench-C.txt
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-07-bench-C-run1.txt
  modified:
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-BENCH-EVIDENCE.md
    - docs/src/operations/observability.md
    - CHANGELOG.md
    - .planning/PROJECT.md
    - .planning/WINDOWS.md

key-decisions:
  - "Verdict: AMEND. Point C, target enabled: log_sink +19.36 %, composite +16.19 %, both far above 3.00 %"
  - "The ledger row stays OPEN: acceptance is the maintainer's at Phase 45 UAT (D-19); row 35 stays waived and untouched"
  - "The authoritative point C is the later, HEAD-verified run; the earlier run is reported, not dropped"

requirements-completed: [OBS-05]

duration: n/a (human measurement plus recording)
completed: 2026-09-30
status: complete
---

# Phase 45 Plan 07: OBS-05 tracing-overhead re-measure Summary

**The D-18 three-point re-measure is on the record and the OBS-05 bar is not met: at point C with the `paladin::trace` target enabled, `log_sink` costs +19.36 % and `composite` +16.19 % over `none`, so D-19's amend branch applies and the new figure awaits the maintainer's acceptance at UAT.**

## Verdict

`Verdict: AMEND` (in `45-BENCH-EVIDENCE.md`). Decided solely by point C's two target-enabled overheads against 3.00 %.

## Figures (point estimates, microseconds; overhead = variant / none - 1)

Target enabled (the gate rows):

| Point | none | log_sink | composite | log_sink overhead | composite overhead |
|-------|------|----------|-----------|-------------------|--------------------|
| A | 112.19 | 131.85 | 129.60 | +17.52 % | +15.52 % |
| B | 115.16 | 135.96 | 135.05 | +18.06 % | +17.27 % |
| C (authoritative) | 120.00 | 143.23 | 139.43 | +19.36 % | +16.19 % |

Target disabled (`_target_off`, for information):

| Point | none | log_sink | composite | log_sink overhead | composite overhead |
|-------|------|----------|-----------|-------------------|--------------------|
| A | 109.30 | 133.19 | 129.99 | +21.86 % | +18.93 % |
| B | 111.88 | 133.51 | 132.88 | +19.33 % | +18.77 % |
| C | 113.31 | 131.71 | 134.35 | +16.24 % | +18.57 % |

Phase 28 baseline (history): +22.18 % / +18.46 % against 110.18 us.

## What the numbers show

- The enablement guard (A to B) and the buffer reuse plus `CompositeSink` move (B to C) are **not separable from noise** at this sample: the confidence intervals overlap at every row, and the `none` baseline itself drifts 112 to 120 us across the points. This is weaker than the plan's expected "real but small" framing, and the evidence file says so rather than claiming a per-fix benefit.
- With serialisation fully skipped (the `_target_off` rows at B and C) the sink path still costs about +16 % to +19 %. The remaining per-record cost is therefore not serialisation. The bench does not isolate what it is; the evidence names the candidates (bounded dispatcher queue, consumer task hand-off, the log macro's own dispatch) as unmeasured. No I/O-bound bench was added (D-19).
- Against Phase 28 the enabled point estimates are 2.8 and 2.3 percentage points lower, but the absolute timings are not lower, so no improvement is claimed.

## Noise caveat the maintainer should weigh at UAT

The earlier point C run (same tree, kept as `45-07-bench-C-run1.txt`) gave +6.98 % / +7.29 % enabled, because its `none` read 128.97 us against 120.00 us while its sink timings (137.97 / 138.37 us) agree with the authoritative run within their intervals. The ratio against `none` is therefore noisy on this shared machine (the `none` interval is about 7 to 9 percent of its own value wide). Both samples exceed 3 %, so the verdict does not depend on the choice, but the accepted-pending figure is a point estimate from one run, not a tight measurement. The hand-over note described the two runs as agreeing within noise; that holds for the sink timings but not for the overhead ratio, and the evidence file records the difference.

## Task Commits

| Task | Name | Commit |
|------|------|--------|
| prep | Pin the point C SHA in the evidence protocol | `181f8815` |
| 1 | Maintainer runs the three-point D-18 measurement (human checkpoint) | no commit; the maintainer's five raw files were committed with Task 2 |
| 2 | Record the outcome under D-19 | `f9c2c379` |

## Ledger

- **Appended row: `WINDOWS.md` row 61**, phase 45, kind `deviation`, file `45-BENCH-EVIDENCE.md`, status **OPEN**. Its description records the amended closing condition: accepted-pending at +19.36 %/+16.19 % after the D-17 fixes, 2026-09-30.
- Row 35 is still `waived` and unedited. Ledger counts now: open 3, waived 36, fixed 22, total 61.
- Open rows block `/gsd-ship`, by design.

## UAT follow-up (required)

The maintainer accepts or rejects the +19.36 %/+16.19 % figure at Phase 45 UAT. On acceptance run `node .claude/gsd-core/bin/gsd-tools.cjs windows waive 61 "<acceptance text>"`. On rejection the row stays open and the I/O-bound re-scope or further optimisation becomes a follow-up decision. An agent must not waive row 61.

## Deviations from Plan

### Process deviations

**1. [Measurement] Two container sessions instead of one**
- **Found during:** Task 1 (as reported by the maintainer)
- **Issue:** The plan requires one session. Points A and B ran in one dev-container session (hostname `4dc47c9ad9c2`); the container was then rebuilt (Phase 45's own compose change) and the authoritative point C ran in the rebuilt container (hostname `3037c6223ca7`). Same host, same toolchain, same fixture and command.
- **Point C freshness:** its `cargo bench` finished in 0.60 s with no compile step, reported by the maintainer as cargo's fingerprint check finding C's artifacts fresh, with `git rev-parse HEAD` confirming `68182d7b1c0a76f6aac450c53a9fe2bc1a8a929a` right after the run. This executor could not independently verify the binary.
- **Disposition:** recorded in the evidence file's session section; the earlier run 1 corroborates the sink timings.
- **Commit:** `f9c2c379`

**2. [Measurement] Shared, busy machine**
VS Code and rust-analyzer ran alongside (as in Phase 28); rust-analyzer held the cargo lock at the start of point B and was waited out. Free memory was 8.4 GiB against Phase 28's 1.2 GiB.

### Auto-fixed issues

None. The plan's task text was followed as written; observability.md's stale Phase 28 evidence path was corrected to the `milestones/v0.10.0-phases/` location as the plan directs.

## Decisions Made

- Replaced the +22 %/+18 % figure on the current-state lines (PROJECT.md Context and Key Decisions) and in observability.md's bullet; annotated, not rewrote, the dated Phase 28 close note and the Validated line. The released `[0.10.0]` CHANGELOG section is untouched (verified additions-only diff).

## Verification

- No `PENDING` marker remains in `45-BENCH-EVIDENCE.md`; exactly one `Verdict: AMEND` line; 126 lines naming `engine/bench_superstep_cost_sinks_` IDs.
- `make check-doc-config`: 150 YAML blocks, 0 failed. `windows status` shows row 61 open.
- Every figure in the tables was computed by script from the pasted raw files, then cross-checked against the hand-over point estimates.

## Known Stubs

None.

## Threat Flags

None. No new network endpoints, auth paths or trust-boundary schema. T-45-29 (raw output, machine block and SHAs pasted verbatim) and T-45-30 (ledger changed only through `windows append`, row left open) are both mitigated.

## Issues Encountered

None beyond the noise caveat above.

## Self-Check

## Self-Check: PASSED

All five raw files and `45-BENCH-EVIDENCE.md` exist; commits `181f8815` and `f9c2c379` are present in the log.
