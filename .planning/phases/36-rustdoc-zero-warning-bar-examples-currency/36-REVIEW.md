---
phase: 36-rustdoc-zero-warning-bar-examples-currency
reviewed: 2026-09-18T00:00:00Z
depth: standard
files_reviewed: 47
files_reviewed_list:
  - .github/workflows/ci.yml
  - crates/doc-examples/src/http_service_host.rs
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/edge_evaluator.rs
  - crates/paladin-battalion/src/engine/cache_key.rs
  - crates/paladin-battalion/src/engine/directive_parser.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/input_mapping.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-core/src/platform/container/directive.rs
  - crates/paladin-core/src/platform/container/structured.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-core/src/platform/container/webhook.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/http_status.rs
  - crates/paladin-llm/src/redaction.rs
  - crates/paladin-llm/src/services/commissary.rs
  - crates/paladin-memory/src/token_counter/mod.rs
  - crates/paladin-ports/src/output/structured_executor_port.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-web/src/dev_ui_controller.rs
  - crates/paladin-web/src/thread_controller.rs
  - examples/README.md
  - examples/agent_runtime_middleware.rs
  - examples/control_flow_dynamic_routing.rs
  - examples/eval_scenarios_demo.rs
  - examples/graceful_shutdown.rs
  - examples/http_service_host.rs
  - examples/human_in_the_loop_gate.rs
  - examples/node_result_cache.rs
  - examples/observability_otel_export.rs
  - examples/observability_tracing.rs
  - examples/platform_api_client.rs
  - examples/sanctum_rag_retrieval.rs
  - examples/structured_output_schema.rs
  - examples/token_economy_commissary.rs
  - examples/war_engine_configuration.rs
  - examples/webhook_receiver.rs
  - scripts/check-all-examples.sh
  - src/application/cli/commands/eval.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/parley/adapter.rs
  - src/application/services/run/worker.rs
  - src/config/agent_runtime.rs
  - src/infrastructure/telemetry/otel_sink.rs
  - src/presets/mod.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 36: Code Review Report

**Reviewed:** 2026-09-18
**Depth:** standard
**Files Reviewed:** 47
**Status:** issues_found

## Summary

This phase is scoped as documentation-only: intra-doc link repairs across 31 library-crate
and `src/` files, 14 brand-new `examples/*.rs` programs, one modified pre-existing example
(`examples/http_service_host.rs`), and the CI/script/README gate wiring that keeps the
"Example Muster" split and the README's table of contents matching the 62 files on disk.

I verified every one of the 31 library/`src/` files' diffs against `8fab0788..HEAD` line by
line. Thirty of the thirty-one changed **only** doc-comment lines (`//!`/`///`), consistent
with the phase's D-05/D-28 no-behaviour-change mandate. One file did not:
`crates/doc-examples/src/http_service_host.rs` gained real, non-comment code (new imports,
two new router states, two new `.merge()` calls, two new `println!` HTTP round-trips) — see
CR-01. This is a scope violation of the phase's own stated boundary, not a `#[cfg(test)]`-only
change, so it is flagged as a blocker per the review brief's explicit instruction, with the
caveat that the change itself looks correct and mirrors the legitimate EX-33 router-parity
fix already reviewed and accepted in the real `examples/http_service_host.rs`.

The 14 new example programs are well-constructed: every unused `PaladinPort` stub correctly
uses `unreachable!()` (never on a reachable path), no example calls `.unwrap()`/`.expect()`/
`panic!()` on a reachable path, every fallible call uses `?`, every program terminates on its
own (no example blocks on a real external signal or an unbounded read), and the
security-sensitive ones (`webhook_receiver.rs`) verify HMAC signatures in constant time via
`hmac::Mac::verify_slice`, generate their shared secret from a CSPRNG, and never log/print a
secret. I spot-checked every "exotic" symbol these programs call (`ArmamentFailed`,
`RagRetrievalService`, `ShedItem`, `SsrfGuard`, `Commissary`, `WaypointRetentionService`,
`RunApiState`/`ThreadApiState`/`run_router`/`thread_router`, etc.) against the real crate
source; all resolve to real, currently-shipped items, so the examples are not exercising a
hallucinated or stale API surface.

`.github/workflows/ci.yml`'s "Example Muster" job, `scripts/check-all-examples.sh` and
`examples/README.md`'s table of contents are internally consistent with each other and with
the repository: `find examples -name '*.rs' | wc -l` returns exactly 62, `Cargo.toml`
declares exactly 8 `[[example]]` `required-features` targets, and both CI and the local
script split those 8 into the same 7 feature-scoped invocations in the same order. Every one
of the 62 files has a matching `### [name.rs](name.rs)` heading in the README.

A few WARNING-level robustness/timing observations and two INFO-level documentation-accuracy
items round out the findings below. Two items the review brief flagged as already-recorded
(not to be re-derived) are present exactly as described and are referenced, not re-filed: the
`ToolErrorMode::FailRun` redaction bypass surfaced by `agent_runtime_middleware.rs` Part 6
(WINDOWS.md #38), and the 309 rustdoc diagnostics silenced by 8 pre-existing crate-level
`#![allow(rustdoc::…)]` attributes (recorded in `deferred-items.md`).

## Critical Issues

### CR-01: `crates/doc-examples/src/http_service_host.rs` contains a real code change, not just doc-comment repairs — violates this phase's D-05/D-28 no-behaviour-change scope

**File:** `crates/doc-examples/src/http_service_host.rs:1-80` (diff vs. `8fab0788`)
**Issue:** `crates/doc-examples` is a first-class workspace member (`Cargo.toml`'s
`members = [".", "crates/*"]`), so it falls inside the "library crates" bucket this phase's
own D-05/D-28 restricts to doc-comment-only edits. The diff against `8fab0788` shows more
than a doc fix: two new imports (`RunApiState`, `ThreadApiState`, `run_router`,
`thread_router`), two new local bindings (`thread_state`, `run_state`), and the `app`
assembly changed from a single `agent_router(state)` to
`agent_router(state).merge(thread_router(thread_state)).merge(run_router(run_state))`. This
is executable code that changes what `serve_agents()` actually builds and serves — not a
comment.

The change is very likely *correct* — it mirrors the legitimate, already-reviewed EX-33
router-parity fix applied to the real, hand-driven `examples/http_service_host.rs` in the same
commit range (same merge order, same "both routes 501 until a store is wired" framing) — but
it was made in a crate this phase's own contract said would carry doc-comment-only changes.
Because a docs-only phase's verification story (the whole reason CR/WR severities exist here)
assumes no behavior moved, an unflagged, unreviewed-as-behavior code change slipping through
under a "doc fix" label is exactly the kind of drift the phase's own scope boundary exists to
catch — regardless of whether this particular instance turns out to be benign.

**Fix:** Either (a) re-classify this file's change explicitly as an in-scope behavior fix in
the phase's own closure notes (SUMMARY.md/deferred-items.md), with a one-line justification
("kept the mdBook-included example in sync with the real EX-33 parity fix"), so a future
audit does not read this as an undocumented scope breach, or (b) if the doc-examples crate is
truly meant to stay in the doc-comment-only bucket, revert the router-merge change here and
track the currency gap between this file and `examples/http_service_host.rs` as a follow-up
item instead.

## Warnings

### WR-01: `agent_runtime_middleware.rs` and `examples/README.md`'s Advanced Examples section print/demonstrate a credential-shaped literal alongside the known unredacted fail-run path

**File:** `examples/agent_runtime_middleware.rs:140-149, 471-491`
**Issue:** `FailingArsenal::invoke` returns
`ArsenalError::TransportError("upstream gateway rejected the request: Authorization: Bearer sk-live-demo0123456789")`,
and the `main()` fail-run arm prints that `reason` verbatim via `println!("   reason = {reason}")`
to demonstrate the already-recorded `ToolErrorMode::FailRun` redaction bypass (WINDOWS.md
#38). The fake key is clearly a demo string (`sk-live-demo…`), not a real credential, and the
whole point of Part 6 is to prove the bypass exists — that intent is fine and is not being
re-litigated here. The residual concern is narrower: this is the only place in the new
example set that deliberately prints something formatted like a live secret to stdout, and a
reader skimming example output (rather than the surrounding prose) could copy the pattern
into a *real* fail-run scenario assuming the printed `reason` is already safe to log, since
every other example in this phase is careful to say "never printed/logged" about anything
credential-shaped.
**Fix:** Consider prefixing the printed `reason` line with an explicit one-line warning (e.g.
"NOTE: this reason is intentionally NOT redacted — see the defect note below") immediately
above the value, rather than only after it, so a reader who does not finish reading the NOTE
block still sees the caveat before the raw string.

### WR-02: Fixed-delay synchronization in `graceful_shutdown.rs` and `observability_tracing.rs` is a source of CI flakiness, not determinism

**File:** `examples/graceful_shutdown.rs:168-174` (`drain_run`'s `sleep(30ms)` before
`cancel_and_wait`); `examples/observability_tracing.rs:206-212` (`run_demo_graph`'s
`sleep(50ms)` to let the trace consumer drain)
**Issue:** Both programs use a fixed `tokio::time::sleep` to paper over a real race: in
`drain_run`, the "fast" node (10ms/5ms hold) must have already completed and the "slow" node
must still be in flight when `coordinator.cancel_and_wait` fires 30ms after `start()` is
spawned; in `run_demo_graph`, the background trace-sink consumer must have drained its queue
within 50ms before `recording.records()` is read. Both are documented as "deterministic
enough for a demo" and mirror an existing test-suite pattern, but a fixed wall-clock delay is
inherently a race under CPU contention (a loaded CI runner, a `--release` vs. debug build
timing difference, or scheduler jitter can widen or shrink the actual margin). Since these
are examples (not gated CI tests) the blast radius is limited to a confusing but non-blocking
local run, not a red CI job — hence WARNING rather than CRITICAL.
**Fix:** Where practical, replace the fixed sleep with a polling loop bounded by a generous
timeout (e.g. poll `coordinator.in_flight()` until it stabilizes, or poll
`recording.records().len()` until it stops growing for two consecutive checks), matching the
robustness bar the rest of this phase's examples hold themselves to elsewhere (e.g. the
`--nocapture`/SKIP-detection discipline `ci.yml`'s own live-server jobs use).

### WR-03: `examples/README.md`'s pre-existing "Advanced Examples" boilerplate still references a non-existent API shape and placeholder community links, left untouched by this "examples currency" phase

**File:** `examples/README.md:1810-2071` (Error Handling Patterns / Logging and Observability
/ Building a Custom Example / Questions? sections)
**Issue:** This phase's own stated mandate is examples currency, and it did touch two fields
in this section (`response.content` → `response.output`, `response.token_usage.total_tokens`
→ `response.usage.total_tokens`, `response.execution_time` → `response.execution_time_ms`).
But the surrounding snippets in the same sections were left otherwise unverified against the
real API: `PaladinBuilder::max_retries`/`retry_delay`, `OpenAiAdapter::new().api_key(&api_key)`
(builder-style chained `.api_key()` rather than the real adapter's constructor shape), and the
"Questions?" section's `https://github.com/your-org/paladin/issues` and
`https://discord.gg/paladin (if available)` are unresolved placeholders. None of this is new
in this phase, but a phase whose explicit charter is "examples currency" leaving known-stale,
unverified illustrative snippets sitting directly below freshly-corrected field names in the
same file is a missed-scope item worth flagging rather than silently carrying forward.
**Fix:** Either mark these snippets explicitly as "illustrative, not compiled/verified" (a
disclaimer this file does not currently carry anywhere), or fold them into the same currency
sweep this phase already applied to the two field-name fixes immediately above them.

## Info

### IN-01: `crates/doc-examples/src/http_service_host.rs`'s change duplicates EX-33 parity logic instead of sharing it with `examples/http_service_host.rs`

**File:** `crates/doc-examples/src/http_service_host.rs:59-74` vs.
`examples/http_service_host.rs:60-78`
**Issue:** Both files now independently assemble
`agent_router(...).merge(thread_router(...)).merge(run_router(...))` with near-identical
prose explaining the 501-until-wired behavior. This is expected for `crates/doc-examples`
(it exists specifically to be a self-contained `{{#include}}`-able snippet, so duplication is
the accepted cost of that pattern), but it does mean the two copies can now drift
independently — see CR-01 for why this pairing exists at all in a nominally doc-only phase.
**Fix:** No action required beyond what CR-01 already recommends; noting the duplication here
so a future reviewer diffing the two files understands why they look alike without assuming
one was copy-pasted from the other without review.

### IN-02: `examples/eval_scenarios_demo.rs` writes a scenario file to a `tempfile::tempdir()` that is silently dropped (and its directory removed) at end of scope

**File:** `examples/eval_scenarios_demo.rs:226-274`
**Issue:** `temp_dir` (a `tempfile::TempDir`) is created, a scenario file is written under it,
and its path is printed as "the equivalent CLI form" a user could copy-paste and run
themselves (`cargo run --bin paladin --features cli -- eval run "<glob_pattern>"`). Because
`temp_dir` goes out of scope (and its directory is deleted) when `main()` returns, that
printed command line is no longer runnable by the time the program has finished printing
"eval_scenarios_demo complete" and control returns to the shell. This is a minor UX
inconsistency rather than a bug — the in-process demonstration (`runner.trials`/`run_case`)
that follows the printed command is what the example is actually testing, and the printed
command is illustrative, not intended to be copy-pasted after the fact — but the prose ("The
equivalent CLI form:") reads as if it invites exactly that.
**Fix:** Add a short parenthetical after the printed command noting the temp directory is
removed when the program exits (e.g. "(the temp file above is removed when this program
exits; point the glob at a scenario file of your own to actually run this command)").

---

_Reviewed: 2026-09-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

## Orchestrator triage (2026-09-18, before any fix pass)

- **CR-01 — not a defect; disposition `no-fix`.** The `crates/doc-examples/src/http_service_host.rs`
  router-merge change is `EX-55`, an audit work row (`34-AUDIT.md` §6, "sibling of EX-33") that
  CONTEXT D-19 explicitly assigned to this phase and plan 36-09 Task 1 closed alongside EX-33 in
  commit `3363d08d`; its closure is recorded in `36-09-SUMMARY.md` and the phase closure map in
  `36-EVIDENCE.md`. `crates/doc-examples` is `publish = false`, is excluded from the api-surface
  baseline (Phase 35 D-26), and is the compile-verified snippet source for the mdBook — it is an
  example, not a library crate, so D-05/D-28 ("no library behaviour change, no visibility widening")
  are not engaged. The reviewer's fix option (a) is therefore already satisfied by the plan and
  summary record; option (b) (revert) would reopen EX-55. **The fixer must not revert this change.**
- **IN-01** follows from CR-01 and takes the same disposition (the mdBook-included module and the
  hand-driven example are deliberately separate compile targets, Phase 35 D-11).
- **WR-01, WR-02, WR-03, IN-02** stand as written and are in scope for a fix pass: all four are
  confined to files this phase created or owns (`examples/*.rs`, `examples/README.md`).
- **WR-01 context:** the credential-shaped literal is the deliberate, fake demonstration of the
  `ToolErrorMode::FailRun` redaction bypass recorded as `WINDOWS.md` #38; the fix is a printed
  caveat before the demonstration, not removal of the demonstration.
