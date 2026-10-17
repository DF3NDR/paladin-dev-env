---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 03
subsystem: observability
tags: [trace-sink, log-sink, composite-sink, criterion, bench, obs-05]

requires:
  - phase: 28-observability
    provides: LogTraceSink, CompositeSink, per-run TraceDispatcher and the bench_superstep_cost_sink_variants fixture (D-16, D-37)
provides:
  - "Three separately committed measurement points (A bench harness, B enablement guard, C buffer reuse) so 45-07 can re-measure before and after each fix on one machine"
  - "LogTraceSink skips JSON serialisation and message formatting when the paladin::trace target is disabled"
  - "write_trace_line serialises into a per-thread reused Vec<u8> released above 64 KiB (no per-record String)"
  - "CompositeSink moves the record into its last child (one deep clone fewer per event)"
  - "TraceDispatcher::emit rustdoc records why no enablement guard belongs in the dispatcher"
  - "45-BENCH-EVIDENCE.md protocol skeleton with results marked PENDING (45-07)"
affects: [45-07]

tech-stack:
  added: []
  patterns:
    - "Enablement guard via log::log_enabled!(target: ..., Info) reading the operator's real filter, kept outside the serialising function so its error path stays testable"
    - "Bounded thread-local scratch buffer with try_borrow_mut and a fresh-buffer fallback (no panic path)"

key-files:
  created:
    - .planning/phases/45-rustfs-swap-platform-observability-deviations/45-BENCH-EVIDENCE.md
  modified:
    - benches/engine_benchmarks.rs
    - src/infrastructure/telemetry/log_sink.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-battalion/src/engine/hooks.rs

key-decisions:
  - "LogTraceSink stays a Copy unit struct with a thread-local buffer (not a Mutex<Vec<u8>> field), so make api-surface reports the surface unchanged (4044 items)"
  - "The enablement guard lives in write_trace_line_if_enabled, not inside write_trace_line, so log_sink_never_returns_err_and_logs_diagnostic keeps exercising the error path"
  - "TraceDispatcher::emit gets NO guard: it cannot know which sinks need a record and skipping would bypass the seq stamp and tallies replay depends on (D-00f); its thread_id/run_id clones are forced by TraceRecord's owned fields (D-17), so the change there is rustdoc only"
  - "The bench keeps the three Phase 28 IDs as the target-ENABLED rows and adds _target_off rows; fixture, command and BatchSize unchanged"

patterns-established:
  - "Measurement points as commits: bench harness first, each fix in its own commit, SHAs recorded in the evidence file"

requirements-completed: [OBS-05]

duration: 40min
completed: 2026-09-29
status: complete
---

# Phase 45 Plan 03: Trace-sink overhead fixes and bench harness Summary

**`LogTraceSink` now skips serialisation entirely for a filtered `paladin::trace` target and otherwise serialises into a reused, bounded per-thread buffer; `CompositeSink` moves the record into its last child; and the sink-variant bench measures both the enabled and disabled target on the unchanged Phase 28 fixture, as three separately committed measurement points.**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-29
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 5 modified, 1 created

## Measurement points

| Point | SHA | Subject |
|-------|-----|---------|
| A | `54400e2d2ed9789c81665f52f1ebf381a860dbe8` | `test(45-03): install a discarding logger and add target-off rows to the sink-variant bench` |
| B | `1f95afeffae13c92db840578a0120d0aef641e54` | `perf(45-03): skip LogTraceSink serialisation when the paladin::trace target is disabled` |
| C | `68182d7b1c0a76f6aac450c53a9fe2bc1a8a929a` | `perf(45-03): reuse a per-thread trace buffer and move the record into the last CompositeSink child` |

No overhead figure is claimed here: the release `cargo bench` measurement is plan 45-07's human checkpoint (this sandbox has ~4 GB free and no dedicated benchmarking machine). `45-BENCH-EVIDENCE.md` carries the exact command, session procedure and D-19 rules, with every results section `PENDING (45-07)`.

## Accomplishments

- **Point A (tracer):** `DiscardLogger` (formats `record.args()` into `std::io::sink()`), `static LOGGER` and `set_trace_target(enabled)` in `benches/engine_benchmarks.rs`. The three Phase 28 IDs are now the enabled rows; three `..._target_off` rows are new. `cargo test --bench engine_benchmarks -- bench_superstep_cost` runs all six once, each `Testing <id>` followed by `Success`. Tracer gate passed in auto mode ("Tracer verified end-to-end - expanding").
- **Point B:** `trace_target_enabled()` (`log::log_enabled!(target: "paladin::trace", Info)`) and `write_trace_line_if_enabled`; `on_event` returns `Ok(())` without touching `serde_json` when the target is filtered. TDD red was observed (the guard test failed against an unconditional-write stub) before the guard went in.
- **Point C:** const-initialised `thread_local! TRACE_BUF`, `TRACE_BUF_RETAIN_MAX = 64 * 1024`, private `write_into` (`serde_json::to_writer`, checked `str::from_utf8`, error lines name only the error), `try_borrow_mut` with a fresh-`Vec` fallback and release of capacity above the bound. `CompositeSink::on_event` uses `split_last()`: clones for every child but the last, moves into the last, each call still in its own `catch_unwind`, with a private `fold_outcome` helper holding the shared bookkeeping. TDD red was observed (tests failed to compile against the missing `TRACE_BUF_RETAIN_MAX`/`trace_buf_capacity`).
- **Dispatcher finding (D-17, no code change):** `TraceDispatcher::emit` performs no serialisation; its `thread_id`/`run_id` clones are forced by `TraceRecord`'s owned fields; the `usage` clone is a plain-integer copy; no guard is added because the bus, persisting, OTel and herald sinks all need every record and skipping would bypass the `seq` stamp and tallies that gapless `seq` and `RunStreamMode::Replay` depend on (D-00f). Recorded as a `# Serialisation cost (OBS-05, Phase 45)` rustdoc section; the `hooks.rs` diff is doc-only.

## Verification

- `cargo test -p paladin-ai --lib infrastructure::telemetry`: 20 passed (includes `log_sink_skips_serialisation_when_the_trace_target_is_disabled`, `on_event_writes_no_line_when_the_trace_target_is_disabled`, `log_sink_reuses_its_buffer_without_bleeding_between_records`, `log_sink_writes_an_oversize_record_then_a_small_record_exactly`, and the unchanged `log_sink_never_returns_err_and_logs_diagnostic`, `on_event_always_returns_ok`, `log_sink_writes_one_json_line_per_record`).
- `cargo test -p paladin-ports --lib output::trace_sink_port`: 11 passed (includes `composite_sink_hands_an_equal_record_to_its_last_child`; panic isolation and all-fail/one-success tests unchanged).
- D-00f gate: `cargo test -p paladin-battalion --lib engine::hooks` 19 passed; `application::services::run::stream_tests` and `::events` 23 passed, including `replay_and_live_produce_the_same_wire_sequence` and `terminal_run_with_rows_replays`.
- `cargo clippy -p paladin-ai -p paladin-ports -p paladin-battalion --all-targets -- -D warnings` clean; `cargo clippy --benches -- -D warnings` clean; `cargo fmt --check` clean.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (4044 items).
- Bench test mode re-run after point C: six `Success` lines.
- Not run here: full `cargo test`, `make clean-code`, `make security` (no dependency changed; cargo-audit/deny not exercised in this sandbox), and CI's `bench-check` release compile (CI-attributed).

## Deviations from Plan

None. The plan was executed as written. Two small implementation choices inside the plan's latitude: the `CompositeSink` bookkeeping was factored into a private `fold_outcome` helper (avoids duplicating the panic-logging match for the rest/last calls), and the redundant `is_empty()` early return was dropped because `split_last()` already returns `None` for an empty composite.

## Auth gates

None.

## Known Stubs

None.

## Threat Flags

None. T-45-12 (retention bound), T-45-13 (error lines carry no record content), T-45-14 (no dispatcher guard; replay suites green) and T-45-15 (`try_borrow_mut` fallback, per-child `catch_unwind`) are mitigated as planned.

## Follow-ups for other plans

- **45-07:** fill the `45-BENCH-EVIDENCE.md` results for points A/B/C on one machine in one session and apply D-19 (row 35 closure or amend). Planner expectation, not a result: the amend branch is likely for the enabled rows (RESEARCH Pitfall 14).

## Self-Check: PASSED

- FOUND: benches/engine_benchmarks.rs, src/infrastructure/telemetry/log_sink.rs, crates/paladin-ports/src/output/trace_sink_port.rs, crates/paladin-ports/CHANGELOG.md, crates/paladin-battalion/src/engine/hooks.rs, 45-BENCH-EVIDENCE.md
- FOUND commits: 54400e2d, 1f95afef, 68182d7b
