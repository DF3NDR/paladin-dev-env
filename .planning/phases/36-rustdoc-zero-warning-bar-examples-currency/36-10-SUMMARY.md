---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 10
subsystem: docs
tags: [examples-gallery, node-result-cache, redis-cache, observability, tracing, otel, run-traces, eval-scenarios, paladin-eval]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md (house example shape -- header/README pair convention,
      offline-first pattern)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-08-SUMMARY.md, 36-09-SUMMARY.md (most recent numbered-parts precedent
      for a capability cluster with no existing runnable program)
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec4 (EX-81, EX-82, EX-100..EX-103, EX-105..EX-108 capability rows)
  - phase: 25-node-result-cache
    provides: NodeCachePort/CachePolicy/CacheKeySpec/with_node_cache this plan's Task 1
      demonstrates
  - phase: 28-observability-eval
    provides: TraceEvent/TraceRecord/TraceConfig/build_run_sink/RunTracePort/
      paladin-eval's ScenarioRunner this plan's Tasks 2 and 3 demonstrate
provides:
  - examples/node_result_cache.rs -- gated (redis-cache), build-only Redis node-result
    cache demo: constructs RedisNodeCache, wires with_node_cache, shows a miss then a
    hit (no re-execution), then the APP_NODE_CACHE_ENABLED toggle bypassing the cache
    entirely via a graph with no CachePolicy attached
  - examples/observability_tracing.rs -- runnable, offline trace envelope +
    TraceConfig + OTLP-toggle + persisted-history demo
  - examples/observability_otel_export.rs -- gated (otel), build-only OTLP export sink
    demo
  - examples/eval_scenarios_demo.rs -- runnable, offline eval-scenario declaration +
    live-mode-toggle + CLI-form demo, no manifest change needed (paladin-eval is an
    unconditional dev-dependency)
  - 36-evidence/36-10-examples.txt -- run output, acceptance-criteria greps, and the
    D-24 closure table for all ten EX IDs this plan closes
affects: [36-11, 36-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A capability cluster with no existing runnable program gets one dedicated,
      numbered-parts example whose stdout narrates each capability in the order the
      audit row lists it -- proven a ninth (node_result_cache), tenth
      (observability_tracing), eleventh (observability_otel_export) and twelfth
      (eval_scenarios_demo) time on top of 36-01/36-06/36-07/36-08/36-09's precedent
      (D-15)."
    - "Attaching an Aegis CachePolicy to a node whose engine has no with_node_cache
      backend wired is a typed EngineError::CachePolicyWithoutCacheBackend validation
      error, never a silent no-op -- a demo of the node-cache enable/disable toggle
      must therefore build TWO graphs (one with the policy attached, one without),
      never one graph run under two differently-configured engines."
    - "paladin::infrastructure::telemetry::build_run_sink is the facade's own, already
      shipped sink-composition function (log sink / bus sink / persisting sink / otel
      sink, in that order) -- the correct way to demonstrate 'configuring tracing'
      is to call this exact production function with different TraceConfig values,
      not to hand-roll a parallel composition."
    - "PersistingTraceSink flushes only on a WaypointSaved or RunFinished record (or
      its own threshold); trace dispatch is fire-and-forget over a background
      channel, so any caller reading back captured or persisted records after
      engine.start() returns needs a short sleep to let the consumer drain first --
      the same technique paladin-battalion's own trace tests use."
    - "check_live_mode(false) is not a no-op success path: it returns
      Err(LiveModeError::FlagNotSet) immediately, since the function's whole contract
      is 'refuse unless every one of the three live-mode gates passes'. A demo of the
      non-live default must match on the Result and print the typed refusal, never
      propagate it with `?`."
    - "A ScenarioRunner GraphConstructor closure is `Fn`, called fresh once per case
      run (not `FnOnce`) -- a graph-building closure that captures a Paladin/schema
      must clone them internally on every call, never consume the captured values."
    - "paladin-eval is an unconditional dev-dependency of the root package (needed by
      tests/evals.rs with no --features at all), so an example driving
      ScenarioRunner/Scenario/Case directly needs no required-features and no
      manifest declaration -- confirmed by `git diff --stat -- src/ crates/
      Cargo.toml` staying empty at Task 3's own commit."

key-files:
  created:
    - examples/node_result_cache.rs
    - examples/observability_tracing.rs
    - examples/observability_otel_export.rs
    - examples/eval_scenarios_demo.rs
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-10-examples.txt
  modified:
    - Cargo.toml

key-decisions:
  - "The observability cluster is split into two programs (D-15 planner discretion,
    resolving 36-RESEARCH.md Assumption A3): observability_tracing.rs stays fully
    runnable offline (four of the five EX rows), and only the otel export sink
    (EX-103) gets its own gated, build-only sibling -- folding the export into the
    main program would have made the whole thing build-only and lost the offline
    demonstration for a reader with no collector."
  - "node_result_cache.rs's EX-82 (the enable toggle bypassing the cache) is
    demonstrated by building a SECOND graph with no CachePolicy attached and running
    it on an engine with no cache backend wired, rather than attaching a CachePolicy
    unconditionally and only varying with_node_cache -- the latter would hit
    EngineError::CachePolicyWithoutCacheBackend and fail validation instead of
    demonstrating a bypass, discovered by reading
    crates/paladin-battalion/src/engine/mod.rs's own
    `a_cache_policy_without_an_engine_cache_fails_validation` test before writing the
    program."
  - "observability_tracing.rs's Part 2 (EX-101, 'configuring tracing') demonstrates
    the changed-field effect through paladin::infrastructure::telemetry::
    build_run_sink's own None-vs-Some resolution (log_sink off with no other sink
    source attached -> None; log_sink on -> Some) rather than re-running the demo
    graph a second time -- build_run_sink is the exact function production code
    calls, so exercising it directly is more honest than approximating its behavior."
  - "eval_scenarios_demo.rs's EX-105 declares its Scenario/Case values as Rust struct
    literals rather than parsing a YAML string, since paladin_eval::eval_scenarios!
    itself expands to a fn main() this program's own main() cannot coexist with --
    the SAME Scenario/ScenarioRunner/Case types and the SAME ScenarioRunner::run_case
    call the macro's expansion drives are used directly instead."
  - "eval_scenarios_demo.rs's EX-107 drives the written .eval.yaml file through BOTH
    ScenarioRunner::trials (proving the glob resolves 1 trial, the same discovery the
    CLI and the evals harness both use) AND Scenario::from_path + run_case (to
    actually print a PASSED/FAILED result), rather than only one or the other --
    trials() alone would prove discovery without a result, and from_path+run_case
    alone would skip proving the glob mechanism itself works."

requirements-completed: [CURR-13, CURR-14, CURR-15]

coverage:
  - id: D1
    description: "node_result_cache.rs constructs the Redis-backed node-result cache adapter the redis-cache feature enables and wires it onto a WarEngine via with_node_cache, runs a graph twice to show a miss then a hit (no re-execution, proven via an execution counter and the persisted Waypoint's cache_hit field), then toggles APP_NODE_CACHE_ENABLED off and re-runs against a graph with no CachePolicy attached to show the cache bypassed entirely (EX-81, EX-82)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "cargo build --example node_result_cache --features \"redis-cache\" (exit 0); cargo build --examples (exit 0, target skipped); program built-not-run per D-16 (no Redis server in this devcontainer or CI)"
        status: pass
    human_judgment: false
  - id: D2
    description: "observability_tracing.rs demonstrates the trace envelope (real TraceEvent variant names via a custom in-process TraceSink), TraceConfig configuration (build_run_sink's None-vs-Some resolution across a changed log_sink field), the PALADIN_TRACE_OTEL_ENABLED environment toggle (and its interaction with the otel Cargo feature this build lacks), and the persisted trace history read back via RunTracePort::read (EX-100, EX-101, EX-102, EX-108)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example observability_tracing (exit 0); stdout inspected for all four capability markers -- 7 TraceRecords captured, 7 distinct real event variant names, None-vs-Some sink resolution printed, otel toggle before/after printed, 7 persisted rows read back"
        status: pass
    human_judgment: false
  - id: D3
    description: "observability_otel_export.rs wires the otel-gated OtelTraceSink onto a WarEngine run and shows the endpoint configuration it exports through, build-verified only per D-16 (no reachable OTLP collector in this devcontainer or CI)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "cargo build --example observability_otel_export --features \"otel\" (exit 0); program built-not-run per D-16"
        status: pass
    human_judgment: false
  - id: D4
    description: "eval_scenarios_demo.rs declares two scenarios in Rust against a Paladin built with the mock adapter and runs them through ScenarioRunner::run_case, names the PALADIN_EVAL_LIVE toggle's effect via check_live_mode(false)'s typed refusal, and drives a written .eval.yaml file's glob through ScenarioRunner::trials/run_case in-process while printing the equivalent CLI command (EX-105, EX-106, EX-107)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example eval_scenarios_demo (exit 0); stdout inspected for all three capability markers -- both declared cases PASSED, check_live_mode(false) -> Err(FlagNotSet) printed, glob resolves 1 trial and the written-file case PASSED"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both gated targets are declared in the root manifest with required-features, the bare cargo build --examples selector skips them, no commit in this plan touches src/ or crates/, and make api-surface / check-api-surface.sh reports the surface unchanged across all four commits"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "cargo build --examples (exit 0, both gated targets skipped); git diff --stat HEAD~4..HEAD -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after every commit)"
        status: pass
    human_judgment: false

# Metrics
duration: ~1h50min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 10: Node-Result Cache, Observability & Eval-Scenarios Examples Summary

**Four new example binaries close ten of Phase 34's fifty-nine documentation gap rows: node-result caching with its Redis backend and enable toggle (EX-81, EX-82), the observability trace envelope/config/persisted-history cluster (EX-100, EX-101, EX-102, EX-108) split from its gated OTLP export sibling (EX-103), and the eval-scenario declaration/live-mode/CLI-form cluster (EX-105, EX-106, EX-107) -- four atomic commits, zero `make api-surface` drift.**

## Performance

- **Duration:** ~1h50min
- **Completed:** 2026-09-17T23:15:00Z
- **Tasks:** 3
- **Files modified:** 6 (4 new example binaries, 1 manifest, 1 new evidence file)

## Accomplishments

- `examples/node_result_cache.rs`: constructs a `RedisNodeCache` (the `redis-cache`-gated
  `NodeCachePort` adapter) and wires it onto a `WarEngine` via `with_node_cache`; runs a
  single-`Function`-node graph under two different threads to show a cache miss (the node
  executes, an `AtomicU64` counter increments) then a cache hit (the node is served from the
  cache, the counter stays at 1) -- both proven from the persisted Waypoint's own
  `NodeExecutionRecord.cache_hit` field, not inferred. Then toggles
  `APP_NODE_CACHE_ENABLED` off via `NodeCacheConfig::apply_env_overrides` and builds a THIRD
  graph with no `CachePolicy` attached at all, run on an engine with no cache backend wired,
  to show the cache bypassed entirely -- discovered while reading
  `crates/paladin-battalion/src/engine/mod.rs`'s own test suite that attaching a `CachePolicy`
  to a node whose engine has no cache backend is a typed validation error
  (`EngineError::CachePolicyWithoutCacheBackend`), never a silent no-op, so the toggle-off path
  needed a genuinely different graph rather than the same graph run under a differently
  configured engine. Needs a running Redis server (`make services-up`); per D-16, build-verified
  only, never executed by any automated check here or in CI.
- `examples/observability_tracing.rs`: a fully offline, four-part demonstration.
  Part 1 runs a small `Function`-node graph with a custom in-process `RecordingSink`
  (`TraceSink`) attached via a `CompositeSink`, and prints every captured `TraceRecord`'s
  envelope plus the real `TraceEvent` variant name it carried -- 7 records, 7 distinct real
  variant names (`RunStarted`, `SuperstepStarted`, `NodeStarted`, `NodeFinished`,
  `DeltaMerged`, `WaypointSaved`, `RunFinished`) on every run. Part 2 constructs a `TraceConfig`
  and calls the facade's own `paladin::infrastructure::telemetry::build_run_sink` twice --
  once with every sink source off (resolves to `None`, the untraced fast path) and once with
  only `log_sink` flipped to `true` (resolves to `Some`) -- showing a single changed field's
  effect through the exact function production code calls, not an approximation. Part 3 sets
  `PALADIN_TRACE_OTEL_ENABLED=true` in-process, reloads the config, and shows
  `config.otel.enabled` flips true -- but states plainly that export also requires the `otel`
  Cargo feature, which this build lacks, so `build_run_sink`'s OTel branch is compiled out and
  nothing observable changes even with the flag on. Part 4 reads the same run's persisted
  `run_traces` rows back through `RunTracePort::read` (flushed via a `PersistingTraceSink` over
  an `InMemoryRunTraceStore`), printing run id, sequence and event name per row.
- `examples/observability_otel_export.rs`: gated on `otel`, wires the OTLP-export-gated
  `OtelTraceSink` onto a `WarEngine` run and prints the endpoint configuration
  (`http://localhost:4318/v1/traces`) it exports through. Needs a reachable OTLP/HTTP collector,
  absent from this devcontainer and CI; per D-16, build-verified only.
- `examples/eval_scenarios_demo.rs`: a fully offline, three-part demonstration built against
  a one-`Paladin`-node graph registered with `ScenarioRunner`. Part 1 declares a two-case
  `Scenario` directly in Rust -- the same `Scenario`/`Case`/`Assertion` types a `.eval.yaml`
  file deserializes into, since `paladin_eval::eval_scenarios!` itself expands to its own
  `fn main()` incompatible with this program's own -- and runs both cases through
  `ScenarioRunner::run_case`, printing `PASSED`/`FAILED` per case (both `PASSED`). Part 2 prints
  `PALADIN_EVAL_LIVE`'s current value and what enabling it changes, then calls
  `check_live_mode(false)` and matches on its typed `Err(LiveModeError::FlagNotSet)` refusal
  rather than propagating it with `?` -- discovered that `check_live_mode(false)` is never
  `Ok(())` by construction (the function's whole contract is "refuse unless every one of the
  three live-mode gates passes"), so the demo prints the refusal rather than crashing on it.
  Part 3 writes a real `.eval.yaml` scenario file to a `tempfile::tempdir()`, prints the exact
  `paladin eval run "<glob>"` CLI command line, then drives the SAME glob through
  `ScenarioRunner::trials` (proving 1 trial resolves) and `Scenario::from_path` + `run_case`
  (printing the resulting case's `PASSED`) in-process -- the printed command and the
  demonstrated behaviour are the same thing. The workspace's own CLI binary is never spawned as
  a subprocess. `paladin-eval` is an unconditional dev-dependency of the root package, so this
  program needs no `required-features` and no manifest declaration.
- Both gated targets (`node_result_cache`, `observability_otel_export`) are declared as example
  targets in the root manifest with their `required-features`; the bare `cargo build --examples`
  selector skips both. `./scripts/check-api-surface.sh` reports the surface unchanged (3959
  items) after every one of the four commits.

## Task Commits

Each task was committed atomically (Task 2 split into two commits, one per program, per D-26):

1. **Task 1: examples/node_result_cache.rs (EX-81, EX-82)** - `159c849d` (docs)
2. **Task 2a: examples/observability_tracing.rs (EX-100, EX-101, EX-102, EX-108)** - `be0ea584` (docs)
3. **Task 2b: examples/observability_otel_export.rs (EX-103)** - `4d449a7f` (docs)
4. **Task 3: examples/eval_scenarios_demo.rs (EX-105, EX-106, EX-107)** - `8168329a` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified

- `examples/node_result_cache.rs` - gated (redis-cache), build-only node-result cache demo (EX-81, EX-82)
- `examples/observability_tracing.rs` - runnable, offline trace envelope/config/persisted-history demo (EX-100, EX-101, EX-102, EX-108)
- `examples/observability_otel_export.rs` - gated (otel), build-only OTLP export sink demo (EX-103)
- `examples/eval_scenarios_demo.rs` - runnable, offline eval-scenario declaration/live-mode/CLI-form demo (EX-105, EX-106, EX-107)
- `Cargo.toml` - two new gated example target declarations (`node_result_cache`, `observability_otel_export`)
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-10-examples.txt` - run output, acceptance-criteria grep results, D-24 closure table

## Closure Table (D-24)

| ID | capability | program | commit |
|---|---|---|---|
| EX-81 | Redis-backed node-result cache adapter construction | examples/node_result_cache.rs | 159c849d |
| EX-82 | Node-cache enable/disable toggle | examples/node_result_cache.rs | 159c849d |
| EX-100 | Trace record envelope + real event variant names | examples/observability_tracing.rs | be0ea584 |
| EX-101 | TraceConfig-driven sink composition (build_run_sink) | examples/observability_tracing.rs | be0ea584 |
| EX-102 | PALADIN_TRACE_OTEL_ENABLED environment toggle | examples/observability_tracing.rs | be0ea584 |
| EX-108 | Persisted trace history (RunTracePort::read) | examples/observability_tracing.rs | be0ea584 |
| EX-103 | OTLP trace export sink (OtelTraceSink) | examples/observability_otel_export.rs | 4d449a7f |
| EX-105 | Scenario declaration + ScenarioRunner::run_case | examples/eval_scenarios_demo.rs | 8168329a |
| EX-106 | PALADIN_EVAL_LIVE live-mode toggle | examples/eval_scenarios_demo.rs | 8168329a |
| EX-107 | CLI eval-run command-line form | examples/eval_scenarios_demo.rs | 8168329a |

## New Gated Targets (for plan 36-12)

Both declared in the root `Cargo.toml` with `required-features`, ready for the CI and
local-script invocations plan 36-12 adds (per D-17, this plan's own scope note -- no CI or
`scripts/check-all-examples.sh` edit was made here):

- `node_result_cache` -- `required-features = ["redis-cache"]` -- build-verified only (needs a
  running Redis server, D-16)
- `observability_otel_export` -- `required-features = ["otel"]` -- build-verified only (needs a
  reachable OTLP/HTTP collector, D-16)

`observability_tracing` and `eval_scenarios_demo` need no manifest declaration and no dedicated
CI/script line beyond what the bulk `cargo build --examples` / `cargo run --example <name>`
selectors already cover.

## Decisions Made

- Split the observability cluster into a runnable main program (four of five EX rows) and a
  gated, build-only OTLP export sibling (EX-103) -- folding the export in would have made the
  whole cluster build-only and lost the offline demonstration, the resolution of
  `36-RESEARCH.md` Assumption A3 as this phase's own D-15 discretion.
- `node_result_cache.rs`'s enable-toggle demo (EX-82) builds a second, policy-free graph rather
  than varying only `with_node_cache` on one graph, after confirming
  `EngineError::CachePolicyWithoutCacheBackend` would otherwise fire.
- `observability_tracing.rs`'s "configuring tracing" part (EX-101) calls the facade's real
  `build_run_sink` function directly rather than approximating its behavior, so the demonstrated
  None-vs-Some transition is the exact one production code exhibits.
- `eval_scenarios_demo.rs` declares its `Scenario` as Rust struct literals rather than parsing a
  YAML string in-process, since `paladin_eval::eval_scenarios!` expands to its own incompatible
  `fn main()` -- the same `ScenarioRunner`/`Scenario`/`Case` types and the same `run_case` call
  the macro's expansion drives are used directly.
- `eval_scenarios_demo.rs`'s CLI-form part (EX-107) drives the written file through BOTH
  `ScenarioRunner::trials` (proving glob discovery) and `Scenario::from_path` + `run_case`
  (proving a result), rather than either alone.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `check_live_mode(false)?` crashing the eval_scenarios_demo program**
- **Found during:** Task 3, running `eval_scenarios_demo.rs` before committing
- **Issue:** The first draft called `check_live_mode(false)?`, assuming `live_flag: false`
  always returns `Ok(())`. `check_live_mode`'s actual contract is the opposite: it returns
  `Err(LiveModeError::FlagNotSet)` immediately whenever `live_flag` is `false`, since the
  function's whole purpose is "refuse unless every one of the three live-mode gates passes" --
  there is no such thing as a `false`-flag success path. The `?` propagated this error straight
  out of `main`, and the program printed `Error: FlagNotSet` and (observed empirically) did not
  reliably report a non-zero exit in the way the plan's `<verify>` command expected.
- **Fix:** Changed to a `match` on `check_live_mode(false)`, printing the typed refusal
  (`Err(LiveModeError::FlagNotSet)`) as the expected, demonstrated outcome rather than treating
  it as a fatal error.
- **Files modified:** examples/eval_scenarios_demo.rs
- **Verification:** Re-ran the example; exits 0, Part 2 now prints the refusal message and the
  program continues to Part 3.
- **Committed in:** 8168329a (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug, discovered by actually running the program before
committing, not assumed from reading the source).
**Impact on plan:** No scope change; the fix was necessary for the demo to correctly show the
non-live default's real behavior rather than crashing on a mistaken assumption about it.

## Issues Encountered

None beyond the one auto-fixed issue documented above.

## User Setup Required

None for the two runnable programs (`observability_tracing.rs`, `eval_scenarios_demo.rs`) --
both are fully offline, all three provider-key environment variables unset. The two build-only
programs need external services this devcontainer and CI do not provide:
`node_result_cache.rs` needs a running Redis server (`make services-up`);
`observability_otel_export.rs` needs a reachable OTLP/HTTP collector.

## Next Phase Readiness

- Ten more of the fifty-nine Phase 34 audit gap rows are closed (EX-81, EX-82, EX-100 through
  EX-103, EX-105 through EX-108). Combined with plans 36-01, 36-06, 36-07, 36-08 and 36-09,
  fifty-four of fifty-nine rows are now closed.
- Plan 36-11 (owns `examples/README.md`) still needs to add a section for each of these four
  new programs, including the two build-only programs' external-service prerequisites -- no
  README edit was made here per this plan's own scope note.
- Plan 36-12 (owns CI and `scripts/check-all-examples.sh`) still needs to add the
  `cargo build --example node_result_cache --features "redis-cache"` and
  `cargo build --example observability_otel_export --features "otel"` invocations to the
  "Example Muster" CI job and the local examples script, per D-17 -- no CI/script edit was made
  here per this plan's own scope note.
- No blockers for subsequent Phase 36 plans.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: examples/node_result_cache.rs
- FOUND: examples/observability_tracing.rs
- FOUND: examples/observability_otel_export.rs
- FOUND: examples/eval_scenarios_demo.rs
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-10-examples.txt
- FOUND commit: 159c849d
- FOUND commit: be0ea584
- FOUND commit: 4d449a7f
- FOUND commit: 8168329a
