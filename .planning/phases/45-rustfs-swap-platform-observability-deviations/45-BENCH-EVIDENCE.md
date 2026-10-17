# 45 OBS-05: bench_superstep_cost sink-variant overhead re-measure (D-18)

## Purpose

PRD 07 acceptance criterion 6: the measured cost of default-on tracing must be at most 3 %
for BOTH `log_sink` (a single `LogTraceSink`) and `composite` (`LogTraceSink` plus a second
sink through a `CompositeSink`) against `none` (no `TraceSink` attached), with the
`paladin::trace` log target **enabled**.

Baseline (Phase 28, `.planning/milestones/v0.10.0-phases/28-observability-tooling/28-BENCH-EVIDENCE.md`,
D-37, accepted for v0.10.0): `log_sink` +22.18 % and `composite` +18.46 %. Those figures were
measured with NO logger installed, so the `log!` macro discarded the output but the record was
still serialised (the "target disabled but still serialising" condition, RESEARCH Pitfall 15).
Phase 45 re-measures on the SAME fixture (`build_width_graph(8)`), the SAME command and the SAME
three variants, adds a discarding `Info`-level logger so the enabled rows are the
operator-visible default, and records a `_target_off` row per variant for information.

The protocol was written in plan 45-03. The measurement was taken by the maintainer on
2026-09-30 (plan 45-07, Task 1, a human checkpoint: a release `cargo bench` does not fit the
authoring sandbox) and recorded here by plan 45-07, Task 2. Every figure below is computed from
the pasted raw criterion output.

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

## Session procedure and how the session actually ran

Protocol: one machine, one session; record the machine block, then for each of A, B, C run
`git checkout <sha>` and the command above, and paste the raw output.

What actually happened (recorded as a deviation from the single-session instruction, stated
plainly because the evidence depends on it):

- **Same machine, same toolchain, same fixture and command, two container sessions.** All
  three points ran inside the project's VS Code dev container on the maintainer's machine.
  Points A and B ran in one container session (hostname `4dc47c9ad9c2`). The dev container was
  then rebuilt (Phase 45's own compose change) and the authoritative point C ran in the rebuilt
  container (hostname `3037c6223ca7`) on the same host.
- **Point C reused fresh bench artifacts.** Its `cargo bench` reported `Finished ... in 0.60s`
  with no `Compiling` lines, i.e. cargo's fingerprint check found the bench artifacts already
  fresh for the point C tree, so the binary that ran is point C's code. The maintainer verified
  the checked-out commit with `git rev-parse HEAD` immediately after the run:
  `68182d7b1c0a76f6aac450c53a9fe2bc1a8a929a`. This agent could not re-verify the binary; the
  raw point C output simply contains no compile step.
- **Shared, busy machine, exactly as Phase 28 recorded.** VS Code and rust-analyzer were
  running alongside the benchmarks. rust-analyzer held the cargo build-directory lock at the
  start of point B and was waited out.
- **Compile times** (from the raw output): point A compiled fresh in 5m 32s; point B recompiled
  `paladin-ports` ... `paladin-ai` in 2m 28s; point C needed no compile.
- **An earlier point C run (`run 1`) is kept as a second sample.** It is the same tree, same
  command, 0.55s finish with fresh artifacts. It is not the authoritative point C (the
  maintainer designated the later run, whose HEAD was verified right after it) but it is
  reported below rather than dropped, because it differs from the authoritative run in the
  ratio even though its sink timings agree (see "Noise and the second point C sample").
- **`change:` lines in the raw output are criterion's own comparison against whatever run it
  had stored in `target/criterion`** (for B that is A's run; for C it is an earlier stored
  run). They are reproduced verbatim as part of the raw output and are NOT used anywhere below.

## Machine and toolchain

Captured by the maintainer before the runs (`lscpu | grep 'Model name'`, `nproc`, `free -h`,
`uname -srm`, `rustc -V`, `cargo -V`, `date -u`). This is the SAME machine Phase 28's
`28-BENCH-EVIDENCE.md` recorded (Intel Xeon E3-1505M v5, 8 logical cores, same kernel, same
rustc and cargo). Free memory is 8.4 GiB here against 1.2 GiB at Phase 28.

```
Model name:                              Intel(R) Xeon(R) CPU E3-1505M v5 @ 2.80GHz
8
               total        used        free      shared  buff/cache   available
Mem:            62Gi        17Gi       8.4Gi       2.4Gi        40Gi        45Gi
Swap:          2.0Gi          0B       2.0Gi
Linux 6.8.0-138-generic x86_64
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
Wed Sep 30 16:41:02 UTC 2026
```

## Raw criterion output

Verbatim from the maintainer's files (kept next to this file as `45-07-bench-A.txt`,
`45-07-bench-B.txt`, `45-07-bench-C.txt` and `45-07-bench-C-run1.txt`). Each triple is
criterion's own `[lower-bound point-estimate upper-bound]` for the mean at its default 95 %
confidence level; the middle value is what the tables use.

### Point A (bench harness only) - 54400e2d2ed9789c81665f52f1ebf381a860dbe8

```
    Finished `bench` profile [optimized] target(s) in 5m 32s
     Running benches/engine_benchmarks.rs (target/release/deps/engine_benchmarks-dcbe029d8138705c)
Gnuplot not found, using plotters backend
Benchmarking engine/bench_superstep_cost_sinks_none
Benchmarking engine/bench_superstep_cost_sinks_none: Warming up for 1.0000s
Benchmarking engine/bench_superstep_cost_sinks_none: Collecting 100 samples in estimated 3.5627 s (30k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none: Analyzing
engine/bench_superstep_cost_sinks_none
                        time:   [109.93 µs 112.19 µs 114.88 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Collecting 100 samples in estimated 3.4228 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Analyzing
engine/bench_superstep_cost_sinks_log_sink
                        time:   [129.91 µs 131.85 µs 134.08 µs]
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite
Benchmarking engine/bench_superstep_cost_sinks_composite: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite: Collecting 100 samples in estimated 3.5354 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite: Analyzing
engine/bench_superstep_cost_sinks_composite
                        time:   [128.34 µs 129.60 µs 131.00 µs]
Found 8 outliers among 100 measurements (8.00%)
  1 (1.00%) low mild
  4 (4.00%) high mild
  3 (3.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_none_target_off
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Warming upfor 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Collecting100 samples in estimated 3.4366 s (30k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Analyzing
engine/bench_superstep_cost_sinks_none_target_off
                        time:   [107.05 µs 109.30 µs 111.60 µs]
Found 13 outliers among 100 measurements (13.00%)
  5 (5.00%) low mild
  3 (3.00%) high mild
  5 (5.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Collecting 100 samples in estimated 3.4012 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Analyzing
engine/bench_superstep_cost_sinks_log_sink_target_off
                        time:   [131.54 µs 133.19 µs 134.88 µs]
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) low mild
  3 (3.00%) high mild
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite_target_off
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Collecting 100 samples in estimated 3.6642 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Analyzing
engine/bench_superstep_cost_sinks_composite_target_off
                        time:   [127.77 µs 129.99 µs 132.48 µs]
Found 9 outliers among 100 measurements (9.00%)
  3 (3.00%) low mild
  5 (5.00%) high mild
  1 (1.00%) high severe
```

### Point B (+ enablement guard) - 1f95afeffae13c92db840578a0120d0aef641e54

```
   Compiling paladin-ports v0.10.1 (/workspace/crates/paladin-ports)
   Compiling paladin-llm v0.10.1 (/workspace/crates/paladin-llm)
   Compiling paladin-battalion v0.10.1 (/workspace/crates/paladin-battalion)
   Compiling paladin-storage v0.10.1 (/workspace/crates/paladin-storage)
   Compiling paladin-web v0.10.1 (/workspace/crates/paladin-web)
   Compiling paladin-memory v0.10.1 (/workspace/crates/paladin-memory)
   Compiling paladin-eval v0.10.1 (/workspace/crates/paladin-eval)
   Compiling paladin-ai v0.10.1 (/workspace)
    Finished `bench` profile [optimized] target(s) in 2m 28s
     Running benches/engine_benchmarks.rs (target/release/deps/engine_benchmarks-dcbe029d8138705c)
Gnuplot not found, using plotters backend
Benchmarking engine/bench_superstep_cost_sinks_none
Benchmarking engine/bench_superstep_cost_sinks_none: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none: Collecting 100 samples in estimated 3.2650 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none: Analyzing
engine/bench_superstep_cost_sinks_none
                        time:   [111.94 µs 115.16 µs 119.42 µs]
                        change: [+0.3894% +3.7474% +7.4699%] (p = 0.04 < 0.05)
                        Change within noise threshold.
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Collecting 100 samples in estimated 3.5298 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Analyzing
engine/bench_superstep_cost_sinks_log_sink
                        time:   [132.83 µs 135.96 µs 140.00 µs]
                        change: [+0.9739% +5.2970% +11.639%] (p = 0.04 < 0.05)
                        Change within noise threshold.
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) high mild
  4 (4.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite
Benchmarking engine/bench_superstep_cost_sinks_composite: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite: Collecting 100 samples in estimated 3.6379 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite: Analyzing
engine/bench_superstep_cost_sinks_composite
                        time:   [132.68 µs 135.05 µs 137.93 µs]
                        change: [+0.9575% +4.4573% +7.6790%] (p = 0.01 < 0.05)
                        Change within noise threshold.
Found 4 outliers among 100 measurements (4.00%)
  3 (3.00%) high mild
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_none_target_off
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Collecting 100 samples in estimated 3.5279 s (30k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Analyzing
engine/bench_superstep_cost_sinks_none_target_off
                        time:   [110.56 µs 111.88 µs 113.28 µs]
                        change: [-8.3097% -3.1537% +1.4058%] (p = 0.25 > 0.05)
                        No change in performance detected.
Found 3 outliers among 100 measurements (3.00%)
  3 (3.00%) high mild

Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Collecting 100 samples in estimated 3.3784 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Analyzing
engine/bench_superstep_cost_sinks_log_sink_target_off
                        time:   [129.95 µs 133.51 µs 138.10 µs]
                        change: [-2.5572% +1.6115% +7.4187%] (p = 0.66 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite_target_off
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Collecting 100 samples in estimated 3.4379 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Analyzing
engine/bench_superstep_cost_sinks_composite_target_off
                        time:   [131.11 µs 132.88 µs 135.12 µs]
                        change: [+0.9377% +4.2589% +8.0002%] (p = 0.02 < 0.05)
                        Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
  6 (6.00%) high severe
```

### Point C (+ buffer reuse and CompositeSink move) - 68182d7b1c0a76f6aac450c53a9fe2bc1a8a929a

This is the authoritative point C run (HEAD verified with `git rev-parse HEAD` immediately
after it).

```
    Finished `bench` profile [optimized] target(s) in 0.60s
     Running benches/engine_benchmarks.rs (target/release/deps/engine_benchmarks-dcbe029d8138705c)
Gnuplot not found, using plotters backend
Benchmarking engine/bench_superstep_cost_sinks_none
Benchmarking engine/bench_superstep_cost_sinks_none: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none: Collecting 100 samples in estimated 3.1200 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none: Analyzing
engine/bench_superstep_cost_sinks_none
                        time:   [115.92 µs 120.00 µs 124.77 µs]
                        change: [-7.4553% -3.5835% +0.4507%] (p = 0.08 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  5 (5.00%) high mild

Benchmarking engine/bench_superstep_cost_sinks_log_sink
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Collecting 100 samples in estimated 3.4115 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Analyzing
engine/bench_superstep_cost_sinks_log_sink
                        time:   [137.71 µs 143.23 µs 150.77 µs]
                        change: [-5.9689% -2.9993% +0.5529%] (p = 0.07 > 0.05)
                        No change in performance detected.
Found 6 outliers among 100 measurements (6.00%)
  4 (4.00%) high mild
  2 (2.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite
Benchmarking engine/bench_superstep_cost_sinks_composite: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite: Collecting 100 samples in estimated 3.4356 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite: Analyzing
engine/bench_superstep_cost_sinks_composite
                        time:   [136.05 µs 139.43 µs 142.87 µs]
                        change: [-5.9620% -3.4758% -0.9484%] (p = 0.01 < 0.05)
                        Change within noise threshold.
Found 8 outliers among 100 measurements (8.00%)
  5 (5.00%) high mild
  3 (3.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_none_target_off
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Collecting 100 samples in estimated 3.4240 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Analyzing
engine/bench_superstep_cost_sinks_none_target_off
                        time:   [110.49 µs 113.31 µs 116.25 µs]
                        change: [-7.9772% -2.1588% +2.7789%] (p = 0.51 > 0.05)
                        No change in performance detected.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Collecting 100 samples in estimated 3.4224 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Analyzing
engine/bench_superstep_cost_sinks_log_sink_target_off
                        time:   [130.04 µs 131.71 µs 133.57 µs]
                        change: [-12.898% -8.9059% -5.3310%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 9 outliers among 100 measurements (9.00%)
  1 (1.00%) low mild
  5 (5.00%) high mild
  3 (3.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite_target_off
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Collecting 100 samples in estimated 3.4667 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Analyzing
engine/bench_superstep_cost_sinks_composite_target_off
                        time:   [132.31 µs 134.35 µs 136.70 µs]
                        change: [-3.2491% -0.5789% +2.3305%] (p = 0.69 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe
```

### Point C, earlier run 1 (corroborating second sample, same tree)

```
    Finished `bench` profile [optimized] target(s) in 0.55s
     Running benches/engine_benchmarks.rs (target/release/deps/engine_benchmarks-dcbe029d8138705c)
Gnuplot not found, using plotters backend
Benchmarking engine/bench_superstep_cost_sinks_none
Benchmarking engine/bench_superstep_cost_sinks_none: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none: Collecting 100 samples in estimated 3.1706 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none: Analyzing
engine/bench_superstep_cost_sinks_none
                        time:   [123.67 µs 128.97 µs 135.20 µs]
                        change: [+1.4488% +5.9348% +10.835%] (p = 0.01 < 0.05)
                        Performance has regressed.
Found 5 outliers among 100 measurements (5.00%)
  3 (3.00%) high mild
  2 (2.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Collecting 100 samples in estimated 3.6871 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink: Analyzing
engine/bench_superstep_cost_sinks_log_sink
                        time:   [135.12 µs 137.97 µs 141.45 µs]
                        change: [-5.1012% +0.7068% +5.3921%] (p = 0.83 > 0.05)
                        No change in performance detected.
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high mild

Benchmarking engine/bench_superstep_cost_sinks_composite
Benchmarking engine/bench_superstep_cost_sinks_composite: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite: Collecting 100 samples in estimated 3.3262 s (20k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite: Analyzing
engine/bench_superstep_cost_sinks_composite
                        time:   [136.47 µs 138.37 µs 140.64 µs]
                        change: [-1.6255% +1.3223% +4.2804%] (p = 0.38 > 0.05)
                        No change in performance detected.
Found 6 outliers among 100 measurements (6.00%)
  4 (4.00%) high mild
  2 (2.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_none_target_off
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Collecting 100 samples in estimated 3.1094 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_none_target_off: Analyzing
engine/bench_superstep_cost_sinks_none_target_off
                        time:   [112.09 µs 114.12 µs 116.26 µs]
                        change: [+3.1157% +7.3273% +13.174%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 4 outliers among 100 measurements (4.00%)
  3 (3.00%) high mild
  1 (1.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Collecting 100 samples in estimated 3.5134 s (25k iterations)
Benchmarking engine/bench_superstep_cost_sinks_log_sink_target_off: Analyzing
engine/bench_superstep_cost_sinks_log_sink_target_off
                        time:   [136.91 µs 139.91 µs 143.73 µs]
                        change: [-0.4212% +6.8897% +13.586%] (p = 0.04 < 0.05)
                        Change within noise threshold.
Found 12 outliers among 100 measurements (12.00%)
  2 (2.00%) high mild
  10 (10.00%) high severe

Benchmarking engine/bench_superstep_cost_sinks_composite_target_off
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Warming up for 1.0000 s
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Collecting 100 samples in estimated 3.1048 s (20k iterations)
Benchmarking engine/bench_superstep_cost_sinks_composite_target_off: Analyzing
engine/bench_superstep_cost_sinks_composite_target_off
                        time:   [135.57 µs 138.37 µs 141.73 µs]
                        change: [-2.1069% +1.2697% +4.6875%] (p = 0.48 > 0.05)
                        No change in performance detected.
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe
```

## Results table

Overhead is `(variant / none - 1) * 100`, using criterion's middle value (point estimate),
against the `none` row of the same condition and the same point. Values are microseconds.

Target ENABLED (the gate rows):

| Point | none (us) | log_sink (us) | composite (us) | log_sink overhead | composite overhead |
|-------|-----------|---------------|----------------|-------------------|--------------------|
| A | 112.19 | 131.85 | 129.60 | +17.52 % | +15.52 % |
| B | 115.16 | 135.96 | 135.05 | +18.06 % | +17.27 % |
| C | 120.00 | 143.23 | 139.43 | +19.36 % | +16.19 % |

Target DISABLED (`_target_off`, for information; this reproduces Phase 28's condition at A):

| Point | none (us) | log_sink (us) | composite (us) | log_sink overhead | composite overhead |
|-------|-----------|---------------|----------------|-------------------|--------------------|
| A | 109.30 | 133.19 | 129.99 | +21.86 % | +18.93 % |
| B | 111.88 | 133.51 | 132.88 | +19.33 % | +18.77 % |
| C | 113.31 | 131.71 | 134.35 | +16.24 % | +18.57 % |

Point C, the earlier corroborating run (same tree; not the gate figure):

| Point | none (us) | log_sink (us) | composite (us) | log_sink overhead | composite overhead |
|-------|-----------|---------------|----------------|-------------------|--------------------|
| C run 1, target enabled | 128.97 | 137.97 | 138.37 | +6.98 % | +7.29 % |
| C run 1, target off | 114.12 | 139.91 | 138.37 | +22.60 % | +21.25 % |

Phase 28's own figures for reference (different session, same machine): `log_sink` +22.18 %,
`composite` +18.46 %, against a 110.18 us `none`.

## Delta commentary

- **A to B (the enablement guard).** With the target enabled the guard changes nothing, as
  expected: `log_sink` +17.52 % to +18.06 %, `composite` +15.52 % to +17.27 %. The guard's
  effect belongs in the `_target_off` rows, where B skips serialisation and formatting
  entirely. There, `log_sink_target_off` reads 133.19 us at A and 133.51 us at B, and
  `composite_target_off` 129.99 us and 132.88 us: no reduction in absolute time, and the
  confidence intervals overlap. The overhead column drops from +21.86 % to +19.33 % for
  `log_sink` only because `none_target_off` moved from 109.30 us to 111.88 us between the two
  runs. No guard benefit is measurable at this sample size.
- **B to C (buffer reuse and the `CompositeSink` move).** Enabled: `log_sink` +18.06 % to
  +19.36 %, `composite` +17.27 % to +16.19 %. The absolute sink times moved from 135.96 to
  143.23 us and from 135.05 to 139.43 us, but `none` also moved from 115.16 to 120.00 us, and
  the intervals overlap at every row. The buffer reuse and the last-child move are not
  distinguishable from noise in this measurement.
- **Against Phase 28.** The point C enabled overheads (+19.36 % / +16.19 %) are 2.82 and 2.27
  percentage points below Phase 28's (+22.18 % / +18.46 %), but the absolute timings are not
  lower: `log_sink` reads 143.23 us against Phase 28's 134.58 us (the intervals overlap) and
  `composite` 139.43 us against 130.52 us (they do not), while `none` reads 120.00 us against
  110.18 us. The session-to-session drift in `none` is as large as the difference in ratio, so
  this sample does not support a claim of improvement.
- **What the `_target_off` rows show.** With serialisation fully skipped (the guard, at B and
  C) the sink path still costs +19.33 % and +18.77 % at B and +16.24 % and +18.57 % at C over
  `none_target_off`. So the per-record cost that remains is not serialisation. This bench
  does not isolate what it is; the candidates are the parts that run whether or not the
  target is enabled (the bounded dispatcher queue, the consumer task hand-off, and the log
  macro's own dispatch), and apportioning the cost between them would need its own
  measurement. That is the honest finding OBS-05 asked for, not a fault of the guard.

### Noise and the second point C sample

The earlier point C run disagrees with the authoritative one in the ratio, not in the sink
timings: `log_sink` 137.97 us against 143.23 us and `composite` 138.37 us against 139.43 us
overlap within their intervals, but that run's `none` read 128.97 us against 120.00 us, which
turns the enabled overheads into +6.98 % and +7.29 %. The `none` confidence interval on this
shared machine is about 7 to 9 percent of its own value wide (for example [115.92, 124.77] us
at C), so the ratio against `none` is itself noisy. The gate figure is the authoritative run
(+19.36 % / +16.19 %), as designated; the second sample is reported so the spread is visible.
Both samples put both variants above 3.00 %, so the verdict does not depend on which is used.

## Verdict

Decided solely by point C's two target-enabled overheads against 3.00 %: `log_sink` +19.36 %
and `composite` +16.19 %. Both exceed the bar; the OBS-05 criterion is not met on this
measurement, and the measured figures become the new accepted-pending figure under D-19's
amend branch, to be put to the maintainer at Phase 45 UAT.

Verdict: AMEND

## D-19 recording rules

- **Meet:** both enabled-row overheads at point C are at most 3 %. `WINDOWS.md` row 35 is
  closed as fixed with this file named as the evidence; the "Known limitations" bullet in
  `docs/src/operations/observability.md` and the `[0.10.0]`-era CHANGELOG limitation get an
  `[Unreleased]` "Fixed" entry citing the new figures.
- **Miss (taken):** either enabled-row overhead at point C exceeds 3 %. Row 35 is amended, not
  silently re-waived: its closing condition is rewritten to the new measured figure
  ("accepted-pending at +N %/+M % after the D-17 fixes, `45-BENCH-EVIDENCE.md`"), the same
  figure replaces +22 %/+18 % in `observability.md`, `PROJECT.md`'s known-deviation lines and
  the CHANGELOG, and the maintainer's acceptance is requested at this phase's UAT (the Phase
  28 D-37 precedent). The appended ledger row stays open until then.
- Either way, no I/O-bound bench variant is added in this phase.

## Prediction (not a result)

RESEARCH Pitfall 14's arithmetic: 3 % of the ~110 us baseline is ~3.3 us, spread over ~22
trace records per run (`SuperstepStarted`, 8 x `NodeStarted`, 8 x `NodeFinished`,
`DeltaMerged`, `RunStarted`, `RunFinished`), i.e. roughly 150 ns per record for dispatch,
clones, a `Mutex`, a doorbell `try_send`, an `async_trait` box, `catch_unwind` and (with the
target enabled) JSON serialisation of a ~250-byte record. That budget is below a typical
`serde_json` cost for such a record, so the amend branch was the likely outcome for the
enabled rows, and the guard's benefit was expected mainly in the `_target_off` rows. The
measurement bears out the first half (amend) and does not bear out the second (no guard
benefit is visible in the `_target_off` rows).
