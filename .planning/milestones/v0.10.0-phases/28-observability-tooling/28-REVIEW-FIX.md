---
phase: 28-observability-tooling
fixed_at: 2026-09-09T12:55:22Z
review_path: .planning/phases/28-observability-tooling/28-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 4
skipped: 1
status: partial
---

# Phase 28: Code Review Fix Report

**Fixed at:** 2026-09-09T12:55:22Z
**Source review:** .planning/phases/28-observability-tooling/28-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (Critical + Warning, per `fix_scope: critical_warning`): 5 (CR-01, WR-01, WR-02, WR-03, WR-04)
- Fixed: 4
- Skipped: 1 (WR-04)

IN-01 (Info) was out of scope for this run (`fix_scope: critical_warning`) and was not attempted.

## Fixed Issues

### CR-01: `TraceEvent::RunStarted.run_id` is always `None` in production, breaking OTel run-span correlation

**Files modified:** `crates/paladin-battalion/src/engine/hooks.rs`, `crates/paladin-battalion/src/engine/mod.rs`, `src/infrastructure/telemetry/otel_sink.rs`
**Commit:** `6c0c8b69`
**Applied fix:** Added `TraceDispatcher::run_id(&self) -> Option<&RunId>`, mirroring the existing `thread_id()` accessor. Replaced the five hardcoded `run_id: None,` literals in `WarEngine::start`/`resume_with_options`/`fork` with `run_id: trace.run_id().cloned(),`, so `RunStarted` now carries the dispatcher's own bound `RunId` (the same value the production worker already passes into `TraceDispatcher::with_capacity`). As defense-in-depth, `OtelTraceSink::on_event`'s `RunStarted` arm now falls back to the envelope's own `record.run_id` when the event-level field is `None`, so a future producer repeating the same mistake doesn't silently reintroduce the gap.
**Test added:** `crates/paladin-battalion/src/engine/mod.rs::engine::tests::run_started_carries_the_bound_run_id` -- binds a real `RunId` via `with_bound_trace`, runs `start()`, and asserts the emitted `RunStarted` record's `run_id` equals the bound id (would have failed pre-fix, since the site hardcoded `None`). `src/infrastructure/telemetry/otel_sink.rs::tests::run_started_falls_back_to_envelope_run_id_when_event_field_is_none` -- proves the OTel sink's fallback path independently.
**Verification:** `cargo test -p paladin-battalion --lib engine::` (541 passed), `cargo test --lib --features otel infrastructure::telemetry::otel_sink` (7 passed), `cargo clippy -p paladin-battalion --all-targets -- -D warnings` and `cargo clippy --features otel --all-targets -- -D warnings` (clean).

### WR-01: `MiddlewareEvent{action: Retry|Fallback}` fired on every model call, not only when a retry/fallback actually happened

**Files modified:** `src/application/services/paladin/middleware/resilience.rs`, `src/application/services/paladin/middleware/chain.rs`, `src/application/services/paladin/paladin_execution_service.rs`
**Commit:** `4b3a223a`
**Applied fix:** Removed the unconditional `cx.middleware_action_hint = Some(MiddlewareAction::Retry|Fallback)` from `ModelRetryMiddleware`/`ModelFallbackMiddleware::before_model` (both now only install a capability, matching what their doc comments already claimed). `MiddlewareAction::Retry` is now emitted directly from `PaladinExecutionService::execute_with_retry_and_temperature`'s own retry arm -- the exact moment a retry is about to happen -- gated on `retry_policy.is_some()` so it is never misattributed when no `ModelRetryMiddleware` is installed. `MiddlewareAction::Fallback` was dropped entirely in favor of `FallbackLlmAdapter`'s own `TraceEvent::FallbackHop`, which already fires only on a real hop. Updated `chain.rs`'s module doc to describe the new split (Retry/Fallback no longer go through the generic `emit_pending_hint` mechanism; that mechanism remains available for future middleware).
**Tests added:** In `resilience.rs`'s test module: `retry_middleware_emits_no_event_when_first_attempt_succeeds`, `retry_middleware_emits_exactly_one_event_per_actual_retry` (asserts exactly 3 Retry events for 3 real retries, not 4 `before_model` calls), `fallback_middleware_emits_no_event_when_primary_succeeds`, `fallback_middleware_emits_no_middleware_action_event_on_a_real_hop` (asserts `FallbackHop` still fires but `MiddlewareAction::Fallback` never does). All four would have failed pre-fix.
**Verification:** `cargo test --lib application::services::paladin::middleware` (90 passed), `cargo test --lib application::services::paladin::paladin_execution_service` (55 passed), `cargo clippy --workspace --all-targets --features cli,dev-ui,otel -- -D warnings` (clean).

### WR-02: `dev-ui` inspector page escaped the inspector JSON payload but not the sibling Mermaid-URL payload

**Files modified:** `crates/paladin-web/src/dev_ui_controller.rs`
**Commit:** `8325accc`
**Applied fix:** `mermaid_url_json` is now run through `escape_for_script`, exactly like `escaped_json` (the inspector payload) already was, per the REVIEW's own suggested patch.
**Test added:** `dev_ui_page_escapes_the_mermaid_url_payload` -- configures `mermaid_url` with an embedded `</script><!--pwned-->` breakout sequence and asserts the raw sequence never appears verbatim in the response body while the escaped forms (`<\/script>`, `<\!--pwned-->`) do. Mirrors the existing `dev_ui_page_escapes_the_embedded_payload` test for the inspector-data payload; would have failed pre-fix.
**Verification:** `cargo test -p paladin-web --features dev-ui dev_ui` (17 passed, including the new test), `cargo clippy -p paladin-web --all-targets --features dev-ui -- -D warnings` (clean).

### WR-03: `PaladinExecutionService`'s `NodeProgress` records use a single fixed placeholder `NodeId`

**Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
**Commit:** `7f0dcb92`
**Applied fix:** Documentation-only, per the REVIEW's own explicitly offered fallback ("If that plumbing genuinely isn't available at this layer yet, at minimum document the placeholder's false-precision risk more strongly"). Threading the real `NodeId` through requires either a breaking change to the public `PaladinPort` trait signature (every implementor across `paladin-ports`, `paladin-battalion`, and this crate) or a new cross-crate task-local mirroring `RUN_TRACE_EMITTER`, whose correctness would need separate verification against the engine's own per-node `tokio::spawn` concurrency (a plain task-local does not propagate across a `tokio::spawn` boundary the way it does across a same-task `.await`). Both are genuinely invasive, out of scope for a fix pass, and are called out as follow-up work rather than silently absorbed. `placeholder_node_id()`'s doc comment now spells out precisely why this is a misattribution risk (not merely "no node context") for the next engineer who considers wiring a `NodeProgress` consumer.
**Verification:** `cargo test --lib application::services::paladin::paladin_execution_service::tests::stream_and_tool_progress_are_emitted` (passed, unchanged behavior), `cargo clippy --features cli -- -D warnings` (clean). No behavioral change, so no new test was required (doc-only).

## Skipped Issues

### WR-04: `tests/helpers/e2e_fixtures.rs` is pulled into the shipped `src/` library/CLI surface via a cross-directory `#[path]`

**File:** `src/application/cli/commands/eval.rs:33-35`
**Reason:** The REVIEW itself states "No urgent action required given the explicit design rationale" and that the current `#[path]` inclusion is a deliberate, documented choice (X-10), not an oversight. Both suggested remediations are invasive relative to this fix pass's scope:
  (a) extracting the 534-line `tests/helpers/e2e_fixtures.rs` into a proper library module (`paladin_eval::fixtures` or a new crate) touches `tests/evals.rs`, all three `tests/integration/e2e_*_test.rs` files, and `eval.rs` itself, and risks introducing a new dependency edge that needs re-validation against ADR-0031's hexagonal rules;
  (b) adding a CI job that fails if `e2e_fixtures.rs` is edited without a matching `eval.rs` touch in the same commit means authoring and validating a new `.github/workflows/ci.yml` job, which cannot be locally verified end-to-end in this environment and risks destabilizing the existing CI pipeline (23 jobs) without a real CI run to confirm it behaves as intended.
  Per this task's own instructions ("If the fix is invasive ... document it as skipped with rationale instead"), no code change was made. No Dockerfile changes were made either, since `eval.rs`'s `#[path]` include was not touched.
**Original issue:** `eval.rs` compiles a `tests/`-directory file directly into the `cli`-feature-gated portion of the shipped library/CLI binary's dependency closure -- an architectural smell (test-fixture changes can silently break the `cli` feature build) rather than a functional bug.

---

_Fixed: 2026-09-09T12:55:22Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
