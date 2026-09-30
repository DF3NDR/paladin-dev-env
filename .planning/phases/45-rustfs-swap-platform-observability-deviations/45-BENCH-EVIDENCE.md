# 45 OBS-05: bench_superstep_cost sink-variant overhead re-measure (D-18)

## Purpose

PRD 07 acceptance criterion 6: the measured cost of default-on tracing must be at most 3 %
for BOTH `log_sink` (a single `LogTraceSink`) and `composite` (`LogTraceSink` plus a second
sink through a `CompositeSink`) against `none` (no `TraceSink` attached), with the
`paladin::trace` log target **enabled**.

Baseline (Phase 28, `28-BENCH-EVIDENCE.md`, D-37, accepted for v0.10.0): `log_sink` +22.18 %
and `composite` +18.46 %. Those figures were measured with NO logger installed, so the
`log!` macro discarded the output but the record was still serialised (the "target disabled
but still serialising" condition, RESEARCH Pitfall 15). Phase 45 re-measures on the SAME
fixture (`build_width_graph(8)`), the SAME command and the SAME three variants, adds a
discarding `Info`-level logger so the enabled rows are the operator-visible default, and
records a `_target_off` row per variant for information.

This file is a protocol skeleton written in plan 45-03. The measurement itself is plan 45-07's
human checkpoint: a release `cargo bench` does not fit the authoring sandbox. Every results
section below is `PENDING (45-07)` until then.

## Command

```bash
cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3
```

Identical to Phase 28. The filter selects six benchmark IDs (the width rows are named
`engine/superstep_cost_width_*` and are not selected):

| Row | Target | Meaning |
|-----|--------|---------|
| `engine/bench_superstep_cost_sinks_none` | enabled | baseline, no sink |
| `engine/bench_superstep_cost_sinks_log_sink` | enabled | the gate row (Phase 28 ID, unchanged) |
| `engine/bench_superstep_cost_sinks_composite` | enabled | the gate row (Phase 28 ID, unchanged) |
| `engine/bench_superstep_cost_sinks_none_target_off` | disabled | for information |
| `engine/bench_superstep_cost_sinks_log_sink_target_off` | disabled | for information |
| `engine/bench_superstep_cost_sinks_composite_target_off` | disabled | for information |

## Measurement points

Three commits, each a complete, buildable tree, so the effect of each fix is isolated:

| Point | Commit subject | What it adds | SHA |
|-------|----------------|--------------|-----|
| A | `test(45-03): install a discarding logger and add target-off rows to the sink-variant bench` | Bench harness only: `DiscardLogger`, `set_trace_target`, the `_target_off` rows. `LogTraceSink` is unchanged, so the `_target_off` rows still pay serialisation (Phase 28's condition) | 54400e2d2ed9789c81665f52f1ebf381a860dbe8 |
| B | `perf(45-03): skip LogTraceSink serialisation when the paladin::trace target is disabled` | A + the enablement guard: a filtered target costs neither serialisation nor formatting | 1f95afeffae13c92db840578a0120d0aef641e54 |
| C | `perf(45-03): reuse a per-thread trace buffer and move the record into the last CompositeSink child` | B + thread-local `Vec<u8>` buffer reuse (`serde_json::to_writer`) and the `CompositeSink` last-child move | 68182d7b1c0a76f6aac450c53a9fe2bc1a8a929a |

Resolve a SHA with `git log --format=%H -1 --grep='<subject>'`.

## Session procedure

Everything below runs on ONE machine in ONE session; do not mix numbers from different
sessions or machines.

1. Record the machine block (commands below) before the first bench run.
2. For each point in order A, B, C:
   1. `git checkout <sha>`
   2. Run the command above and capture the full stdout (`... | tee target/45-bench-<point>.log`).
   3. Paste the raw criterion output into the matching subsection below.
3. Return to the working branch (`git checkout claude/laughing-dirac-e0h2ax`).

Machine block commands: `lscpu`, `nproc`, `free -h`, `uname -srm`, `rustc -V`, `cargo -V`,
`date -u`.

## Machine and toolchain

PENDING (45-07)

## Raw criterion output

### Point A (bench harness only)

PENDING (45-07)

### Point B (+ enablement guard)

PENDING (45-07)

### Point C (+ buffer reuse and CompositeSink move)

PENDING (45-07)

## Results table

Overhead is `(variant_mean - none_mean) / none_mean * 100`, using criterion's middle value
(point estimate), against the `none` row of the same condition and the same point.

Target ENABLED (the gate rows):

| Point | none (us) | log_sink (us) | composite (us) | log_sink overhead | composite overhead |
|-------|-----------|---------------|----------------|-------------------|--------------------|
| A | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |
| B | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |
| C | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |

Target DISABLED (`_target_off`, for information):

| Point | none (us) | log_sink (us) | composite (us) | log_sink overhead | composite overhead |
|-------|-----------|---------------|----------------|-------------------|--------------------|
| A | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |
| B | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |
| C | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) | PENDING (45-07) |

## Verdict

PENDING (45-07)

## D-19 recording rules

- **Meet:** both enabled-row overheads at point C are at most 3 %. `WINDOWS.md` row 35 is
  closed as fixed with this file named as the evidence; the "Known limitations" bullet in
  `docs/src/operations/observability.md` and the `[0.10.0]`-era CHANGELOG limitation get an
  `[Unreleased]` "Fixed" entry citing the new figures.
- **Miss:** either enabled-row overhead at point C exceeds 3 %. Row 35 is amended, not
  silently re-waived: its closing condition is rewritten to the new measured figure
  ("accepted at +N %/+M % after the D-17 fixes, `45-BENCH-EVIDENCE.md`"), the same figure
  replaces +22 %/+18 % in `observability.md`, `PROJECT.md`'s known-deviation lines and the
  CHANGELOG, and the maintainer's acceptance is requested at this phase's UAT (the Phase 28
  D-37 precedent).
- Either way, no I/O-bound bench variant is added in this phase.

## Prediction (not a result)

RESEARCH Pitfall 14's arithmetic: 3 % of the ~110 us baseline is ~3.3 us, spread over ~22
trace records per run (`SuperstepStarted`, 8 x `NodeStarted`, 8 x `NodeFinished`,
`DeltaMerged`, `RunStarted`, `RunFinished`), i.e. roughly 150 ns per record for dispatch,
clones, a `Mutex`, a doorbell `try_send`, an `async_trait` box, `catch_unwind` and (with the
target enabled) JSON serialisation of a ~250-byte record. That budget is below a typical
`serde_json` cost for such a record, so the amend branch is the likely outcome for the
enabled rows, and the guard's benefit should show mainly in the `_target_off` rows. This is
a planner assumption to be checked, not a measured figure.
