# 28-06 Task 3: `bench_superstep_cost` sink-variant overhead evidence

**Purpose:** PRD 07 acceptance 6's ≤3% overhead bar (D-37) — the measured cost of
default-on tracing (the facade's `LogTraceSink`, and a two-child `CompositeSink`
mirroring `build_run_sink`'s own log+bus case) against the engine's untraced path
(`none`, no `TraceSink` attached, D-10).

## Command

```bash
cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3
```

Bounded warm-up/measurement time per this plan's own execution rules (a full-default
criterion run on this shared machine would exceed a single bounded tool call). The three
variants live in `bench_superstep_cost_sink_variants` (`benches/engine_benchmarks.rs`),
reusing `build_width_graph(8)` (the existing "many nodes" fixture `bench_superstep_cost`
itself already measures, unchanged) — no new fixture builder.

## Machine and toolchain

- CPU: Intel(R) Xeon(R) CPU E3-1505M v5 @ 2.80GHz, 8 logical cores (`nproc`)
- Memory at run time: 62 GiB total, **1.2 GiB free** (`free -h`) — a shared, busy
  machine, not a dedicated benchmarking rig. Criterion's own outlier counts below
  (4–16% of samples flagged) are consistent with that.
- OS: Linux 6.8.0-138-generic (Ubuntu 22.04 base), x86_64
- Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`
- Build profile: `bench` (optimized), single run, 100 samples per variant
  (`Collecting 100 samples in estimated ~3.2-3.4s`)
- Date: 2026-09-09T01:45Z

## Raw criterion output (one run)

```
engine/bench_superstep_cost_sinks_none
                        time:   [108.51 µs 110.18 µs 111.91 µs]
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe

engine/bench_superstep_cost_sinks_log_sink
                        time:   [130.78 µs 134.58 µs 140.17 µs]
Found 16 outliers among 100 measurements (16.00%)
  5 (5.00%) low mild
  7 (7.00%) high mild
  4 (4.00%) high severe

engine/bench_superstep_cost_sinks_composite
                        time:   [128.63 µs 130.52 µs 132.63 µs]
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) low mild
  3 (3.00%) high mild
  1 (1.00%) high severe
```

Each triple is criterion's own `[lower-bound point-estimate upper-bound]` for the mean,
at its default 95% confidence level. The middle value (point estimate) is what the table
below uses.

## Results table

| Variant     | Mean (µs) | 95% CI (µs)      | Overhead vs `none` | Outliers |
|-------------|-----------|------------------|---------------------|----------|
| `none`      | 110.18    | [108.51, 111.91] | — (baseline)        | 4%       |
| `log_sink`  | 134.58    | [130.78, 140.17] | **+22.18%**          | 16%      |
| `composite` | 130.52    | [128.63, 132.63] | **+18.46%**          | 5%       |

Overhead computed as `(variant_mean - none_mean) / none_mean * 100`.

## Verdict against PRD 07 acceptance 6 (≤3%)

**FAIL.** Both sinked variants exceed the ≤3% bar by a wide margin: `log_sink` at
+22.18% and `composite` at +18.46% (measured against the `none`/untraced baseline on
this run). The criterion is not softened here per this plan's own instruction — the
number is recorded honestly, not adjusted to fit.

### Why this number is what it is (analysis, not an excuse)

- **Absolute magnitude first:** the untraced baseline itself is ~110 microseconds for
  an 8-node single-superstep run. The `TraceDispatcher`'s per-event cost (enqueue under
  a `Mutex`, `AtomicU64::fetch_add` for `seq`, a `try_send` doorbell wakeup) is on the
  order of hundreds of nanoseconds to low microseconds per call — small in absolute
  terms, but an 8-node superstep with the engine's own trace producers (`SuperstepStarted`,
  8×`NodeStarted`, 8×`NodeFinished`, `DeltaMerged`, `WaypointSaved`, plus any
  `EdgeEvaluated` records) emits on the order of 20+ records per superstep, and at a
  ~110µs baseline, even a modest per-record cost compounds into a double-digit percentage
  overhead. This is a genuinely different regime from a longer-running, I/O-bound
  superstep (e.g. a real Paladin node making an LLM call), where the SAME per-record
  dispatcher cost would be a much smaller fraction of the total.
- **`log_sink` costs more than `composite` in this run**, which looks
  counter-intuitive (`composite` fans out to TWO sinks, including the same
  `LogTraceSink`) but is within the noise band both variants report (`log_sink`'s own
  95% CI is [130.78, 140.17], `composite`'s is [128.63, 132.63] — they overlap). The
  16% outlier rate on `log_sink` (5 low-mild, 7 high-mild, 4 high-severe) is the
  strongest signal that this specific number is noisy on this shared, low-free-memory
  machine, not a stable measurement of `log_sink` genuinely costing more than
  `composite`.
- **Serialization is the likely dominant cost inside `LogTraceSink`**, not the
  dispatcher machinery: every record is `serde_json::to_string`'d before the
  `log::info!` call. This bench does not isolate that from the dispatcher's own
  enqueue/doorbell cost — a future investigation splitting "dispatcher overhead alone"
  from "LogTraceSink's own serialize+format cost" would clarify which half of the
  budget to optimize, if optimization is judged worthwhile.

### Recommendation for phase close-out

This FAIL is flagged in `28-06-SUMMARY.md` for the phase close-out to adjudicate, per
this task's own instruction. Two honest paths forward, neither taken here (out of this
plan's scope):
1. **Treat the ≤3% bar as measured against a realistic (I/O-bound) superstep, not this
   synthetic all-Function-node microbenchmark** — re-derive/re-scope the acceptance
   criterion against a fixture closer to a real Paladin-node workload, where the
   dispatcher's fixed per-record cost is a much smaller fraction of a call that also
   waits on network I/O.
2. **Optimize the hot path** — reduce `TraceDispatcher::emit`'s per-call cost (e.g. a
   lock-free queue instead of `Mutex<VecDeque>`) and/or `LogTraceSink`'s serialize cost,
   then re-measure.

Neither is attempted in this plan: the task's own scope is "measure and record," and the
record above is what a real, repeated run on this machine shows.
