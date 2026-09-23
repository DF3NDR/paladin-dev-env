---
phase: 28-observability-tooling
plan: 09
subsystem: observability
tags: [opentelemetry, otlp, tracing, reqwest, span-exporter, cargo-features]

# Dependency graph
requires:
  - phase: 28-06
    provides: "src/infrastructure/telemetry/{mod.rs,log_sink.rs}: LogTraceSink, build_run_sink (the single per-run sink composition point this plan extends), and the twelve-variant TraceEvent/TraceRecord envelope from 28-01/28-02/28-03"
provides:
  - "OtelTraceSink (src/infrastructure/telemetry/otel_sink.rs, otel-gated): turns the TraceRecord stream alone into a span-per-attempt tree -- one run root span per thread_id, one child span per (node_id, attempt), retries as siblings never nested"
  - "The otel Cargo feature and its three optional dependencies (opentelemetry, opentelemetry_sdk, opentelemetry-otlp), absent from default and full, X-11.4"
  - "build_run_sink's fan-out generalized from two sinks to N (log, bus, otel), still unwrapping to a single sink or None when fewer than two are configured"
  - "tests/integration/otel_transport_test.rs: a hermetic axum stub proving the exporter's real OTLP/HTTP wire format, headers, service.name, and no-redirect client"
  - "A dedicated otel-feature CI job in .github/workflows/feature-flags.yml running both the span-tree shape tests and the transport test under --no-default-features --features otel"
affects: ["28-10 (graph overlay, builds on the same build_run_sink composition point)", "28-11/28-12 (telemetry/mod.rs and root Cargo.toml, this plan's own files -- coordinate on merge)"]

# Tech tracking
tech-stack:
  added: ["opentelemetry 0.32.0", "opentelemetry_sdk 0.32.1", "opentelemetry-otlp 0.32.0", "opentelemetry-proto 0.32.0 (dev-only)", "prost 0.14 (dev-only)"]
  patterns:
    - "A synthetic, flagged-partial span (paladin.trace.partial = true) opened on demand when a record arrives for a span this sink never saw start -- degrades the trace rather than losing the node, and is itself later closed normally if the corresponding Finished event does eventually arrive."
    - "Reusing an EXISTING cross-major-version reqwest alias (reqwest_mcp, already present for the Arsenal MCP client) for a second optional feature's HTTP client, rather than declaring a second alias to the same major version -- one compiled reqwest 0.13.x unit serves both call sites."
    - "SimpleSpanProcessor (synchronous, per-span export) chosen over BatchSpanProcessor specifically because opentelemetry_sdk 0.32's BatchSpanProcessor's own docs state async HTTP clients (reqwest-client) are NOT supported by its default background-thread design -- SimpleSpanProcessor's docs, by contrast, explicitly support an async client PROVIDED it is driven from a Tokio runtime thread, which every below-engine call site here already is."

key-files:
  created:
    - src/infrastructure/telemetry/otel_sink.rs
    - tests/integration/otel_transport_test.rs
    - .planning/phases/28-observability-tooling/deferred-items.md
  modified:
    - src/infrastructure/telemetry/mod.rs
    - Cargo.toml
    - Cargo.lock
    - tests/integration/mod.rs
    - .github/workflows/feature-flags.yml

key-decisions:
  - "SimpleSpanProcessor, not BatchSpanProcessor: opentelemetry_sdk 0.32.1's BatchSpanProcessor's own module docs state async HTTP clients (reqwest-client, the one D-12 mandates) are unsupported by its default background-OS-thread design -- only reqwest-blocking-client or grpc-tonic (with its own tokio runtime) are. SimpleSpanProcessor's docs instead say an async client works PROVIDED `on_end`/`Span::end_with_timestamp` is called from a thread where that client can run -- true here, since `OtelTraceSink::on_event` always runs inside the dispatcher's own tokio task. Empirically verified both ways (28-09): the exact SimpleSpanProcessor + async-reqwest combination deadlocks under a #[tokio::test]'s DEFAULT current_thread runtime (futures_executor::block_on hijacks the only worker thread, no thread left to drive the socket's I/O readiness) but works correctly under #[tokio::test(flavor = \"multi_thread\")] and #[tokio::main] (production's own default, unlike #[tokio::test]). Documented prominently in otel_sink.rs's own module docs so a future editor doesn't 'simplify' back to the default flavor and reintroduce the hang."
  - "The OTLP HTTP client reuses `reqwest_mcp` (the existing 0.13.x alias already required unconditionally for the Arsenal MCP client) instead of declaring a second reqwest-0.13 alias. Discovered empirically: `opentelemetry-otlp`'s `reqwest-client` feature needs `opentelemetry-http`'s own `reqwest` feature, which pins reqwest 0.13.1 -- a DIFFERENT major version from this crate's own default `reqwest` line (0.12.4). Since `reqwest_mcp` already resolves to that same 0.13.x line (rmcp's own dependency, unconditional), reusing its type for `.with_http_client()` compiles against the identical crate `opentelemetry-http`'s blanket `impl HttpClient for reqwest::Client` targets, adding zero new reqwest major versions to the graph."
  - "One HTTP POST per span, not batched: SimpleSpanProcessor exports on every `Span::end_with_timestamp` call individually. A run with N ended spans therefore makes N separate OTLP export requests. Not optimized in this plan -- BatchSpanProcessor's own docs rule it out for this exporter's client choice (see above), and the plan's own acceptance criteria don't require batching. Left as a known characteristic, not a defect; a future plan wanting batched export would need `opentelemetry_sdk`'s `experimental_trace_batch_span_processor_with_async_runtime` feature and a real `tokio::spawn`-driven processor instead of the default background-OS-thread `BatchSpanProcessor`."
  - "build_run_sink generalized from a 2-sink match to an N-sink Vec (log, bus, otel): 0 sinks -> None, exactly 1 -> that sink unwrapped (no CompositeSink overhead), 2+ -> CompositeSink of all configured sinks in that order. Preserves the exact behavior of the two existing 28-06 tests (log-only, bus-only, both) while adding the otel slot without a third bespoke match arm."
  - "A failed OtelTraceSink::new (e.g. a malformed operator-supplied endpoint the exporter itself rejects at build time) is diagnostics-only inside build_run_sink: logged at error level, then treated as 'OTLP export not attached this run' -- the run itself is never failed, matching every other TraceSink's own contract and T-28-09-05's DoS mitigation reasoning."
  - "opentelemetry_sdk's `testing` feature (needed for InMemorySpanExporter, D-13a) and opentelemetry-proto/prost (needed to decode captured protobuf bodies, D-13b) are all dev-dependencies, not gated behind the `otel` feature. This means `cargo tree -e features -p paladin-ai` (the literal orchestrator check, which includes dev-edges by default) shows opentelemetry* even on a build with no --features otel passed -- but `cargo tree -e no-dev,features -p paladin-ai` (the actual PRODUCTION-build view X-11.4 cares about) shows zero. Both were run and recorded below; this is the plan's own explicitly-instructed dev-only widening (\"Add opentelemetry_sdk with its testing feature to [dev-dependencies] so the in-memory exporter is available to tests without entering the production feature set\"), not a gap."

patterns-established:
  - "A TraceSink implementation that must synthesize a partial parent context on demand (D-12's 'never lose the node' contract) locks its own span map, checks-then-creates-and-reinserts under the SAME lock discipline every other lifecycle method uses -- future sinks with an open/close span or session concept should follow this exact get-or-synthesize shape rather than inventing a new one."

requirements-completed: [OBS-02]

coverage:
  - id: D1
    description: "OtelTraceSink builds a correct span-per-attempt tree from the record stream alone: one run root span per thread_id (RunStarted/RunFinished), one child span per (node_id, attempt) with retries as siblings not nested, attributes (node_id/superstep/attempt/outcome/tokens/cache_hit/muster_task_key) on every attempt span, EdgeEvaluated/DeltaMerged/MiddlewareEvent as run-span events, ParleyRaised/FallbackHop as attempt-span events when a node_id is known"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/otel_sink.rs#run_start_and_finish_produce_one_root_span"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/otel_sink.rs#retried_node_produces_sibling_attempt_spans"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/otel_sink.rs#attempt_span_carries_every_attribute"
        status: pass
      - kind: unit
        ref: "src/infrastructure/telemetry/otel_sink.rs#branch_retry_muster_fixture_tree_shape"
        status: pass
    human_judgment: false
  - id: D2
    description: "A record that would need a span this sink never saw opened (a dropped NodeStarted or RunStarted) synthesizes a span flagged paladin.trace.partial = true rather than being discarded, and that synthesized span is the SAME one a later Finished event closes (not a second, disconnected span)"
    requirement: "OBS-02"
    verification:
      - kind: unit
        ref: "src/infrastructure/telemetry/otel_sink.rs#orphan_record_opens_a_partial_span"
        status: pass
    human_judgment: false
  - id: D3
    description: "The otel Cargo feature is absent from default and full's member list; a default build gains zero opentelemetry* dependency in its PRODUCTION graph (cargo tree -e no-dev); both `cargo build` (default) and `cargo build --features otel` succeed; the reqwest-blocking-client default of opentelemetry-otlp is turned off in favor of the async reqwest-client"
    requirement: "OBS-02"
    verification:
      - kind: other
        ref: "cargo build -p paladin-ai (exit 0); cargo build -p paladin-ai --features otel (exit 0); cargo tree -e no-dev,features -p paladin-ai | grep -c opentelemetry == 0; cargo tree -e no-dev,features -p paladin-ai --features otel | grep -c opentelemetry == 66; grep -c 'reqwest-blocking-client' Cargo.toml == 0; grep -c '^otel = ' Cargo.toml == 1 and full's member list omits it"
        status: pass
    human_judgment: false
  - id: D4
    description: "The exporter's HTTP client never follows a redirect (Policy::none()), so a 3xx from the configured collector can never carry OtelConfig.headers' credential-shaped values to a different host -- proven against a REAL second stub receiving zero requests, not merely asserted"
    requirement: "OBS-02"
    verification:
      - kind: integration
        ref: "tests/integration/otel_transport_test.rs#otlp_client_does_not_follow_redirects"
        status: pass
    human_judgment: false
  - id: D5
    description: "A real fixture run's export reaches a real axum stub over the wire with Content-Type: application/x-protobuf, every header configured on OtelConfig, and a body that decodes to a resource whose service.name is the configured value -- the transport layer, not just the in-process span tree"
    requirement: "OBS-02"
    verification:
      - kind: integration
        ref: "tests/integration/otel_transport_test.rs#otlp_export_reaches_the_stub"
        status: pass
    human_judgment: false
  - id: D6
    description: "The CI feature-flags matrix gains a leg that builds --no-default-features --features otel AND actually runs both the span-tree shape tests and the transport integration test in that same leg (not merely a build-only matrix entry, since the shared 14-leg matrix's Test step is `cargo test --workspace --lib`, which never executes the tests/lib.rs integration binary the transport test compiles through)"
    requirement: "OBS-02"
    verification:
      - kind: other
        ref: ".github/workflows/feature-flags.yml otel-feature job (Check/Build/span-tree-tests/transport-test steps); locally reproduced: cargo check --no-default-features --features otel (exit 0); cargo build --workspace --no-default-features --features otel (exit 0); cargo test --workspace --no-default-features --features otel --lib infrastructure::telemetry::otel_sink (6 passed); cargo test --workspace --no-default-features --features otel --test lib -- otel_transport_test (2 passed)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Whole-workspace fmt/clippy/build/deny/audit stay green, both with and without the otel feature, and no OTel dependency introduces a new advisory"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo check --workspace --all-targets --all-features (exit 0); cargo check -p paladin-ai --all-targets (default, exit 0); cargo clippy -p paladin-ai --all-targets --features otel -- -D warnings (exit 0); cargo clippy -p paladin-ai --all-targets -- -D warnings (exit 0); cargo clippy --workspace --all-targets --features otel -- -D warnings (exit 0); cargo test --workspace --lib (default features, 13 lib targets, all `ok`, 0 failed); cargo deny check licenses bans advisories (advisories ok, bans ok, licenses ok -- pre-existing chacha20/spin `yanked` warnings unrelated to this plan's dependencies); cargo audit (exit 0, 10 pre-existing allowed warnings, none opentelemetry-related)"
        status: pass
    human_judgment: false

# Metrics
duration: 56min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 09: OTLP Trace Export — Span-Per-Attempt Tree Over the Real Wire Summary

**An `otel`-gated `OtelTraceSink` turning the trace record stream into a span-per-attempt tree (retries as siblings, orphan events synthesizing flagged-partial spans rather than being lost) exported over real OTLP/HTTP with a no-redirect client, verified in two layers — in-process tree shape against `InMemorySpanExporter`, and real transport against a hermetic axum stub proving headers, content-type, and `service.name` actually cross the wire.**

## Performance

- **Duration:** ~56 min (session start ~02:19 UTC to final commit 02:54 UTC, plus verification/summary work)
- **Started:** 2026-09-09T02:19:00Z (approx., worktree branch check)
- **Completed:** 2026-09-09T02:54:05Z (last task commit)
- **Tasks:** 2 (1 tracer, 1 auto/tdd) — executed as 3 commits
- **Files modified:** 8 (3 created)

## Accomplishments

- `src/infrastructure/telemetry/otel_sink.rs`: `OtelTraceSink` implementing `TraceSink` — one `run` root span per `thread_id` (`RunStarted`/`RunFinished`), one child span per `(node_id, attempt)` (`NodeStarted`/`NodeFinished`) as SIBLINGS on retry, `EdgeEvaluated`/`DeltaMerged`/`MiddlewareEvent` as run-span events, `ParleyRaised`/`FallbackHop` as attempt-span events when a `node_id` is known. A record needing a span this sink never opened synthesizes one flagged `paladin.trace.partial = true` and keeps it open for a later matching Finished event to close normally.
- Root `Cargo.toml`: `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` (all `optional = true`, pinned, `default-features = false`) behind a new `otel` feature — absent from `default` and `full`'s explicit member list (X-11.4), mirroring the `storage-postgres`/`redis-cache` precedent's own comment style.
- `build_run_sink` (28-06's composition root) generalized from a 2-sink match to an N-sink `Vec` fan-out: `config.otel.enabled` (behind `#[cfg(feature = "otel")]`) adds `OtelTraceSink` to the same composite the log/bus sinks already join; a construction failure is diagnostics-only (logged, OTLP export skipped for that run, the run itself never fails).
- `tests/integration/otel_transport_test.rs` (otel-gated): a real axum stub on an ephemeral `127.0.0.1` port captures the exporter's actual OTLP/HTTP POST — `Content-Type: application/x-protobuf`, every configured header, a decoded `service.name` — and a second stub proves the no-redirect client leaves a `302`'s target with zero requests.
- `.github/workflows/feature-flags.yml`: a new dedicated `otel-feature` job (not a 14-leg matrix entry, since the shared matrix's `Test` step is `--lib`-only and never runs the transport integration test) building `--no-default-features --features otel` and running both test layers in the same leg; `feature-matrix-summary` now requires it.
- `.planning/phases/28-observability-tooling/deferred-items.md`: logged a pre-existing, unrelated gap (`paladin_builder.rs`'s own test module references `paladin_llm::deepseek`/`anthropic` with no feature gate, surfacing only under `-p paladin-ai --no-default-features` in isolation) discovered while choosing the CI job's exact command — routed around (`--workspace` instead of `-p paladin-ai`, matching the existing matrix legs' own convention), not fixed, per the executor's scope-boundary rule.

## Task Commits

1. **Task 1 (tracer): otel-gated OTLP span exporter with span-per-attempt tree** — `4afecd0a` (feat)
2. **Task 2 (auto/tdd), part 1: hermetic OTLP transport integration test** — `2f95a471` (test)
3. **Task 2, part 2: otel leg in the feature-flags CI matrix** — `d82751ee` (chore)

**Plan metadata:** (this commit)

## Files Created/Modified

- `src/infrastructure/telemetry/otel_sink.rs` - New: `OtelTraceSink`, `OtelSinkError`, the span-per-attempt model, six behavior tests
- `src/infrastructure/telemetry/mod.rs` - `build_run_sink` generalized to N sinks; wires in `OtelTraceSink` behind `#[cfg(feature = "otel")]` + `config.otel.enabled`
- `Cargo.toml` - `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` optional deps + `otel` feature; `opentelemetry_sdk`'s `testing` feature, `opentelemetry-proto`, `prost` as dev-dependencies
- `Cargo.lock` - Resolved additions (opentelemetry family, opentelemetry-proto, prost; no new reqwest major version)
- `tests/integration/otel_transport_test.rs` - New: `otlp_export_reaches_the_stub`, `otlp_client_does_not_follow_redirects`
- `tests/integration/mod.rs` - `#[cfg(feature = "otel")] pub mod otel_transport_test;`
- `.github/workflows/feature-flags.yml` - New `otel-feature` job; `feature-matrix-summary` needs/condition updated
- `.planning/phases/28-observability-tooling/deferred-items.md` - New: logged out-of-scope pre-existing gap

## Decisions Made

See `key-decisions` in frontmatter. The two most consequential: (1) `SimpleSpanProcessor`, not `BatchSpanProcessor` — `opentelemetry_sdk` 0.32's own docs rule out `BatchSpanProcessor`'s default background-thread design for an async `reqwest` client, and the synchronous alternative is safe here because every call site already runs inside a Tokio task, empirically confirmed both ways (deadlocks under `#[tokio::test]`'s default `current_thread` flavor, works under `multi_thread`/`#[tokio::main]`); (2) the OTLP HTTP client reuses the EXISTING `reqwest_mcp` 0.13.x alias (discovered empirically that `opentelemetry-otlp`'s async client needs a different reqwest major version than this crate's default 0.12 line) rather than declaring a second alias — zero new reqwest major versions enter the graph.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `InMemorySpanExporter` lives at `opentelemetry_sdk::trace::InMemorySpanExporter`, not `opentelemetry_sdk::testing::trace::InMemorySpanExporter` as the plan's `<interfaces>` section stated**
- **Found during:** Task 1, while locating the real API before writing tests
- **Issue:** The plan's interfaces note (sourced from 28-RESEARCH.md) named a module path that doesn't exist at `opentelemetry_sdk` 0.32.1 — the real re-export is `pub use in_memory_exporter::{InMemorySpanExporter, InMemorySpanExporterBuilder};` directly under `opentelemetry_sdk::trace`, gated `#[cfg(any(feature = "testing", test))]`.
- **Fix:** Used the real path (`opentelemetry_sdk::trace::InMemorySpanExporter`) throughout `otel_sink.rs`'s test module.
- **Files modified:** `src/infrastructure/telemetry/otel_sink.rs` (test module only)
- **Verification:** `cargo test -p paladin-ai --features otel --lib infrastructure::telemetry::otel_sink` (6 passed)
- **Committed in:** `4afecd0a`

**2. [Rule 3 - Blocking] `opentelemetry-otlp`'s `reqwest-client` feature resolves to `reqwest` 0.13.x, not this crate's own default `reqwest` 0.12.4 line — the plan's `<interfaces>` section named the Cargo feature but not this version mismatch**
- **Found during:** Task 1, first compile attempt against a scratch probe crate (deliberately isolated from the real workspace to de-risk the unfamiliar API surface before touching the real tree) — `reqwest::Client` (0.12) does not implement `opentelemetry_http::HttpClient`; only `reqwest`'s OWN 0.13.1-pinned dependency does.
- **Issue:** Passing a 0.12 client to `.with_http_client()` fails to compile (`E0277`). This workspace already has an existing precedent for exactly this situation: `reqwest_mcp = { package = "reqwest", version = "0.13", default-features = false }`, added for `rmcp`'s own 0.13 requirement.
- **Fix:** Built the OTLP exporter's client as `reqwest_mcp::Client` instead of `reqwest::Client`, reusing the already-unconditional 0.13.x line rather than declaring a second alias.
- **Files modified:** `src/infrastructure/telemetry/otel_sink.rs`, `Cargo.toml` (comment documenting the reuse, no new dependency line for reqwest itself)
- **Verification:** `cargo build -p paladin-ai --features otel` (exit 0); `cargo tree -e no-dev,features -p paladin-ai --features otel | grep -c 'reqwest v0.1[23]'` shows exactly the two pre-existing major versions, no third
- **Committed in:** `4afecd0a`

**3. [Rule 1 - Bug] `Drop for OtelTraceSink` calling `self.provider.shutdown()` reset `InMemorySpanExporter`'s recorded spans before tests could read them**
- **Found during:** Task 1, first real test run — all five non-trivial behavior tests failed with `0` exported spans despite spans being ended correctly.
- **Issue:** `InMemorySpanExporterBuilder::new()`'s default `reset_on_shutdown: true` means `SdkTracerProvider::shutdown()` (called both explicitly by this sink's own `Drop` impl AND implicitly by `opentelemetry_sdk`'s own `TracerProviderInner::Drop` when the last `Arc` reference is released) clears the exporter's buffer. Every test called `drop(sink)` before reading `exporter.get_finished_spans()`, wiping the very spans under test.
- **Fix:** `SimpleSpanProcessor` exports synchronously on every `Span::end_with_timestamp` call (confirmed by reading `opentelemetry_sdk`'s own source) — no flush/shutdown is needed before reading. Removed all `drop(sink)` calls preceding an assertion; tests now read the exporter directly after the `on_event` calls that end the spans under test.
- **Files modified:** `src/infrastructure/telemetry/otel_sink.rs` (test module only)
- **Verification:** All 6 tests pass; re-verified the orphan-span test specifically distinguishes "still open, not yet exported" from "exported" by sending `RunFinished` in a second phase and re-reading the exporter
- **Committed in:** `4afecd0a`

**4. [Rule 3 - Blocking] Three `clippy::collapsible_if` errors under `-D warnings` in the mutex-lock-then-check pattern**
- **Found during:** Task 1, `cargo clippy -p paladin-ai --all-targets --features otel -- -D warnings`
- **Issue:** `if let Ok(x) = self.mutex.lock() { if let Some(y) = x.get(...) { ... } }` — clippy 1.97 (this toolchain) flags nested `if let` as collapsible via the stabilized `let ... && let ...` chain syntax.
- **Fix:** Collapsed all three sites (`parent_context_for_run`, `add_run_event`, `add_node_event`) into `if let Ok(x) = ... && let Some(y) = ... { }`.
- **Files modified:** `src/infrastructure/telemetry/otel_sink.rs`
- **Verification:** `cargo clippy -p paladin-ai --all-targets --features otel -- -D warnings` (exit 0)
- **Committed in:** `4afecd0a`

**5. [Rule 1 - Bug] `SimpleSpanProcessor` exports one HTTP request PER SPAN, not batched — the transport test's initial fixture (root-only, no node) undercounted the real request count once a node was added**
- **Found during:** Task 2, first transport test run against the real axum stub — `otlp_export_reaches_the_stub` failed asserting exactly 1 captured request when the fixture (deliberately made more realistic mid-task by adding a `NodeStarted`/`NodeFinished` pair) produced 2 spans and therefore 2 separate POSTs.
- **Issue:** Not a bug in `OtelTraceSink` itself — a wrong assumption in the test's own assertion about how many spans the fixture would end, given `SimpleSpanProcessor`'s synchronous, unbatched export contract.
- **Fix:** Updated the test to assert 2 requests (one per span) and to check every captured request's headers/content-type/service.name, not just the first.
- **Files modified:** `tests/integration/otel_transport_test.rs`
- **Verification:** `otlp_export_reaches_the_stub` passes; documented the one-POST-per-span characteristic in `key-decisions` above
- **Committed in:** `2f95a471`

**6. [Documented scope boundary, not a rule-driven fix] `-p paladin-ai --no-default-features` (in isolation, without `--workspace`) fails to compile its OWN `--lib` test target for a reason unrelated to this plan**
- **Found during:** Task 2, choosing the exact CI command for the new `otel-feature` job.
- **Reasoning:** `paladin_builder.rs`'s own `#[cfg(test)]` module references `paladin_llm::deepseek::{DeepSeekConfig, DeepSeekAdapter}` (and the same gap recurs for `anthropic` in three other test files) with no feature gate. This only surfaces when `paladin-ai`'s `--lib` target is compiled in ISOLATION under `--no-default-features` (`-p paladin-ai`); `cargo test --workspace --lib --no-default-features` (matching the existing 14-leg matrix's own `Test` step) works fine, because feature unification across the whole workspace build keeps `deepseek`/`anthropic` enabled via some other member's edge. Confirmed this is pre-existing and unrelated to `otel` by reproducing it with NO otel feature at all.
- **Action:** The new CI job uses `--workspace` (not `-p paladin-ai`), matching the convention every existing matrix leg already uses — a routing decision, not a fix to the underlying gap. Logged in `.planning/phases/28-observability-tooling/deferred-items.md` for a future plan that touches those test files.
- **Files NOT modified:** `src/application/services/paladin/paladin_builder.rs`, `tests/unit/llm/{anthropic,deepseek}_adapter_test.rs`, `tests/integration/provider_switching_test.rs`

---

**Total deviations:** 5 auto-fixed (2 Rule 3 - blocking API-mismatch discoveries, 1 Rule 1 - bug in test setup causing false failures, 1 Rule 3 - blocking clippy lint, 1 Rule 1 - bug in a test's own assumption), 1 documented scope-boundary routing decision (pre-existing, unrelated gap logged and routed around, not fixed).
**Impact on plan:** All five auto-fixes were necessary to make this plan's own must-have truths and acceptance criteria hold; none expanded scope beyond the plan's own stated deliverables. The scope-boundary item is explicitly deferred per the executor's own rules.

## Issues Encountered

- **`SimpleSpanProcessor` + async `reqwest` + `#[tokio::test]`'s default `current_thread` flavor deadlocks the whole test process.** Discovered via a scratch probe crate BEFORE touching the real workspace (a `timeout 20`-wrapped isolated run confirmed the hang; the `multi_thread` variant of the identical scenario passed in 0.21s). This is `opentelemetry_sdk`'s own documented caveat ("ensure this processor is only used from a thread where \[async HTTP clients\] can run") made concrete: `futures_executor::block_on` (what `SimpleSpanProcessor::on_end` uses internally) hijacks the calling OS thread's own polling loop, and a `current_thread` Tokio runtime has no OTHER thread left to drive the socket's I/O readiness notification, so the export future never completes. Resolved by using `#[tokio::test(flavor = "multi_thread")]` throughout this plan's own tests and documenting the constraint prominently in `otel_sink.rs`'s module docs; production is unaffected since `#[tokio::main]` (this crate's own binaries) defaults to `multi_thread`, unlike `#[tokio::test]`.
- **`opentelemetry-otlp`'s `reqwest-client` feature pulls a different reqwest major version than this crate's default line** — see Deviation #2 above; resolved by reusing the existing `reqwest_mcp` alias.
- **`InMemorySpanExporter`'s default `reset_on_shutdown: true` silently clears recorded spans on `Drop`** — see Deviation #3 above; resolved by relying on `SimpleSpanProcessor`'s synchronous export contract instead of an explicit flush/shutdown-then-read pattern in tests.

## User Setup Required

None - no external service configuration required. Operators who want OTLP export must compile with `--features otel` and set `trace.otel.enabled: true` plus `trace.otel.endpoint`/`headers`/`service_name` in their own config — both were already validated by `TraceConfig::validate_typed` in 28-02 (this plan doesn't add new config surface, only wires the existing `OtelConfig` into a real exporter).

## Next Phase Readiness

- `build_run_sink`'s N-sink `Vec` fan-out is the same composition point 28-10 (graph overlay) will extend — no further restructuring needed for a fourth sink to join.
- The `otel` feature's dependency shape (three optional deps, one umbrella feature, absent from `default`/`full`) is now the concrete precedent for any future optional observability backend this workspace adds.
- `.planning/phases/28-observability-tooling/deferred-items.md` carries one unrelated, pre-existing test-gating gap (`paladin_builder.rs`'s test module) for a future plan to pick up — not blocking for 28-10/28-11/28-12.
- No blockers for the rest of Phase 28. Plans 28-11 (`telemetry/mod.rs`) and 28-12 (root `Cargo.toml`) can now build on this plan's own edits to those exact two files once merged.

## Self-Check: PASSED

- FOUND: `src/infrastructure/telemetry/otel_sink.rs`
- FOUND: `tests/integration/otel_transport_test.rs`
- FOUND: `.planning/phases/28-observability-tooling/deferred-items.md`
- FOUND commit: `4afecd0a`
- FOUND commit: `2f95a471`
- FOUND commit: `d82751ee`

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
