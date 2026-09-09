---
phase: 28-observability-tooling
verified: 2026-09-09T13:27:03Z
status: human_needed
score: 4/4 roadmap truths verified (all 17 plans' must_have truths independently spot-checked against code and tests; 6 judgment-tier prohibitions confirmed by code inspection, non-authoritative)
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Adjudicate PRD 07 acceptance criterion 6 (≤3% superstep-overhead-with-tracing bar, D-37) against the measured result in 28-BENCH-EVIDENCE.md: log_sink +22.18%, composite +18.46% vs the untraced baseline."
    expected: "A maintainer decision: (a) accept the deviation for v0.10.0 and record it as a closed WINDOWS.md/ADR item, (b) re-scope the acceptance bar to an I/O-bound (LLM-call) superstep rather than the synthetic all-Function-node microbenchmark, or (c) require follow-up optimization of `TraceDispatcher::emit`/`LogTraceSink` serialization before shipping. The measurement and its honest FAIL verdict are correctly recorded (28-BENCH-EVIDENCE.md, 28-06-SUMMARY.md, docs/src/operations/observability.md's Known Limitations section) — what's missing is the close-out adjudication D-37 itself calls for (\"This FAIL is flagged... for the phase close-out to adjudicate\") and a WINDOWS.md entry (only #33/#34 exist for this phase; no entry logs the bench-overhead FAIL)."
    why_human: "This is an explicit, PRD-named numeric acceptance bar that the implementation itself measured and failed by a wide margin (6-7x over budget). CONTEXT D-37 deliberately made this non-CI-gating (\"the record is the gate\") and directed the FAIL to be adjudicated at close-out — that adjudication has not happened. A verifier cannot make this policy call; a maintainer must decide whether v0.10.0 ships with this overhead, the bar is re-scoped, or the hot path is optimized first."
  - test: "Confirm the six judgment-tier safety/privacy prohibitions across the phase's must_haves (28-01 state-values-never-default, 28-05 scenario-file-not-an-execution-vector, 28-06 observability-never-load-bearing, 28-09 OTel-header-redaction-and-no-redirect, 28-12 live-mode-never-in-default-CI, 28-15 dev-ui-auth-gate-and-no-values-shown) against the codebase."
    expected: "Each prohibition holds. This verifier's own code inspection (recorded below) found supporting evidence for all six: redact-before-truncate ordering (`superstep.rs:3570-3574`), no `serde` derive on `CustomAssertion` (`assertion.rs:250-255`), `BlockingTraceSink`/`PanickingTraceSink`/the 500ms-sink timing test (`hooks.rs`), the OTel client's `Policy::none()` plus a passing `otlp_client_does_not_follow_redirects` test and `OtelConfig`'s manual redacting `Debug` impl, the three-way `--live` AND `PALADIN_EVAL_LIVE` AND provider-key gate with a passing test, and `dev_ui_unauthenticated_request_is_rejected` plus the `field_changes: Vec<FieldName>`-only (never values) `InspectorView` shape. This assessment is a non-authoritative LLM-judge verdict, per the judgment-tier prohibition policy — a human sign-off is the authoritative closure."
    why_human: "All six prohibitions are `verification: judgment` in the PLAN frontmatter, not `verification: test`-tier with an automated enforcement gate. Per the escalation-gate policy for judgment-tier prohibitions, this verifier's code-level confirmation is recorded but is explicitly non-authoritative; formal closure is a human sign-off, not a silent pass."
gaps: []
deferred: []
---

# Phase 28: Observability & Tooling Verification Report

**Phase Goal:** Every run emits a machine-consumable trace that reaches real consumers, graphs and runs are visualizable, and agent behavior is regression-testable.
**Verified:** 2026-09-09T13:27:03Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The authoritative `TraceEvent` enum carries a per-run monotonic `seq` with a gapless-or-counted-drops guarantee, and a `TraceSinkPort` whose slow/panicking implementations cannot stall or fail a run fans out via `CompositeSink` (OBS-01) | ✓ VERIFIED | `crates/paladin-core/src/platform/container/trace.rs` (12-variant `TraceEvent`, `#[non_exhaustive]`); `TraceDispatcher::emit` stamps `seq`/`at`/`thread_id` at enqueue (`hooks.rs:330`); `sixteen_concurrent_runs_keep_per_run_seq_gapless` and `trace_seq_is_gapless_over_twenty_supersteps` both re-run and PASS on HEAD; `catch_unwind` + `sink_panics` counter (`hooks.rs:239-241`); `PanickingTraceSink`/`BlockingTraceSink` test doubles; `CompositeSink` in `trace_sink_port.rs` forwards to every child under its own panic guard. |
| 2 | Traces reach a default-on structured-log sink, an `otel`-gated OpenTelemetry exporter with span-per-attempt trees verified against a collector stub, and the SSE bridge for `GET /runs/{id}/stream` as a TraceSink adapter (OBS-02) | ✓ VERIFIED | `src/infrastructure/telemetry/log_sink.rs` (default-on via `trace.log_sink`); `otel_sink.rs` span-tree model behind the `otel` feature (absent from `default`/`full`); `branch_retry_muster_fixture_tree_shape`, `attempt_span_carries_every_attribute`, and the axum-stub transport tests `otlp_export_reaches_the_stub`/`otlp_client_does_not_follow_redirects` all re-run and PASS; `map_trace_event` is total over 7 wire names (`events.rs`), `RunEventBusSink` is a `TraceSink`, one producer only (D-14 collapse) — 28-11 wired `RunStreamMode::Replay` + `trace_seq`. |
| 3 | Golden-tested `WarGraphDoc → Mermaid/DOT` exporters and an execution-overlay export let a human answer "which branch fired and why did node X run 3 times" via `paladin-cli graph export`/`run export` and a minimal auth-gated `dev-ui` inspector page (OBS-03) | ✓ VERIFIED | `crates/paladin-battalion/src/engine/export/{shape,mermaid,dot,overlay}.rs`; `export_golden.rs`'s 4 tests (`golden_exports`, `overlay_goldens`, `doc_and_graph_shapes_agree`, `muster_renders_worker_template_and_deferred_node`) re-run and PASS; CLI `graph export`/`run export` snapshot tests (13 total) re-run and PASS; `dev-ui` route re-run — 17/17 `paladin-web --features dev-ui dev_ui` tests PASS including `dev_ui_page_embeds_the_fired_branch_and_the_three_visit_summary` (the literal acceptance question) and `dev_ui_unauthenticated_request_is_rejected`. |
| 4 | The new `paladin-eval` crate runs scripted mock-LLM scenario files through a `cargo test`-integrable runner macro and `paladin-cli eval run --repeat`/`--bless`, with the three program E2E fixtures dogfooded as eval scenarios (OBS-04) | ✓ VERIFIED | `crates/paladin-eval` (composition crate, `paladin-ai` dependency absent even as dev-dep — confirmed in `Cargo.toml`); `eval_scenarios!` macro (`tests/evals.rs`, `harness = false`) — `cargo test --test evals` 4/4 PASS; 12/12 assertion kinds with `insta`-frozen failure rendering re-run and PASS; CLI `eval run` 6/6 tests PASS including `repeat_twenty_is_twenty_of_twenty` and `repeat_divergence_exits_nonzero_and_names_the_seq_range`; all three E2E fixtures (`evals/e2e-{1,2,3}-*.eval.yaml`) dogfooded, `--repeat 20` reported 20/20 for every case per `28-16-SUMMARY.md`'s recorded CLI runs. |

**Score:** 4/4 roadmap truths verified.

### Plan-Level Must-Haves (17 plans, automated + manual spot-check)

`gsd-tools query verify.artifacts`/`verify.key-links` ran against all 17 `*-PLAN.md` files:

- **Artifacts:** 51/51 pass across all plans (exists, substantive, contains-pattern) — no MISSING or STUB artifacts found.
- **Key links:** 30/32 auto-verified; 2 auto-tool false negatives on regex matching (`crates/paladin-battalion/src/engine/hooks.rs → trace.rs` and `worker.rs → engine/mod.rs`) — both manually confirmed WIRED by direct code inspection (`TraceRecord {` literal present at `hooks.rs:330`; `engine.trace_emitter()` called at `worker.rs:883`, threaded through `with_run_trace_scope`/`current_trace_emitter()` ambient task-local so every below-engine producer — `FallbackLlmAdapter`, the middleware chain, `PaladinExecutionService` — draws from the same per-run counter). **32/32 effectively WIRED.**

### Behavioral Re-Verification (this session, not carried from SUMMARYs)

The following tests were independently re-run on HEAD `ff78a6b5` by this verifier (not merely cited from SUMMARY.md claims):

| Test | Result |
|------|--------|
| `paladin-battalion::engine::hooks::tests::sixteen_concurrent_runs_keep_per_run_seq_gapless` | ok |
| `paladin-battalion::engine::hooks::tests::trace_seq_is_gapless_over_twenty_supersteps` | ok |
| `paladin-battalion::engine::tests::run_started_carries_the_bound_run_id` (CR-01 fix regression test) | ok |
| `otel_sink::tests::run_started_falls_back_to_envelope_run_id_when_event_field_is_none` (CR-01 defense-in-depth) | ok |
| `cargo test -p paladin-web --features dev-ui dev_ui` | 17/17 ok |
| `cargo test --lib --features otel infrastructure::telemetry` | 19/19 ok |
| `cargo test --test lib --features otel otel_transport` (axum OTLP stub) | 2/2 ok |
| `cargo test -p paladin-battalion --lib --test export_golden` | 4/4 ok |
| `cargo test --test cli --features cli graph_export` / `run_export` | 6/6, 7/7 ok |
| `cargo test --test cli --features cli eval_run` | 6/6 ok |
| `cargo test -p paladin-eval --test assertion_snapshots` | 12/12 ok |
| `cargo test -p paladin-storage --lib run_trace` (in-memory) | 11/11 ok |
| `cargo test -p paladin-storage --lib --features sqlite run_trace::sqlite` | 10/10 ok |
| `cargo test --lib config::trace` | 7/7 ok |
| `cargo tree -p paladin-battalion --no-default-features -e normal \| grep -c paladin-llm` | 0 (ADR-0031 held) |

All consistent with the orchestrator-supplied evidence (`make test` 3592/0, `postgres-integration` CI job 97/97 including all 10 `run_trace::postgres::*` tests).

### Code Review Findings — Verified Fixed

`28-REVIEW.md` found 1 critical (CR-01) + 4 warnings (WR-01..04) + 1 info (IN-01). `28-REVIEW-FIX.md` fixed CR-01, WR-01, WR-02, WR-03 (WR-04 explicitly skipped with documented rationale; IN-01 out of scope). All 4 fix commits (`6c0c8b69`, `4b3a223a`, `8325accc`, `7f0dcb92`) confirmed as ancestors of HEAD `ff78a6b5` via `git merge-base --is-ancestor`. CR-01's regression tests re-run and pass (above). WR-02's regression test (`dev_ui_page_escapes_the_mermaid_url_payload`) is in the 17/17 dev-ui pass. WR-04 (test-fixture `#[path]` reach-across into the shipped CLI binary) remains an open architectural note, correctly left unfixed as low-severity per the review's own "no urgent action required" language — not a blocker.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| OBS-01 | 28-01, 28-02, 28-03 | Trace event model, `seq`, `TraceSinkPort`, `CompositeSink` | ✓ SATISFIED | See Truth #1 above |
| OBS-02 | 28-02, 28-04, 28-06, 28-09, 28-11 | Log/OTel/SSE consumers, `run_traces` persistence | ✓ SATISFIED | See Truth #2 above |
| OBS-03 | 28-07, 28-10, 28-13, 28-14, 28-15 | Mermaid/DOT export, overlay, CLI, dev-ui | ✓ SATISFIED | See Truth #3 above |
| OBS-04 | 28-05, 28-08, 28-12, 28-16 | `paladin-eval` crate, assertions, runner, dogfood | ✓ SATISFIED | See Truth #4 above |

No orphaned requirements — REQUIREMENTS.md lines 259-282 map exactly to OBS-01..04, all four claimed by at least one of the 17 plans' frontmatter. (REQUIREMENTS.md's own checkboxes for OBS-02/03/04 are still `[ ]` pending — that's an artifact of this being the first verification pass, not a discrepancy; updating them is downstream of this report, not this report's job.)

### Anti-Patterns Found

Scanned all 101 files touched across the 17 plans for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`:

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `MIGRATION.md` | 8, 318, 630 | `TBD` | ℹ️ Info | Each explicitly references `SHIP-01`/`SHIP-02`, Phase 29 — satisfies the debt-marker gate's formal-follow-up-reference exception. Not a blocker. |
| `config.test.yml` | 63 | `PLACEHOLDER_KEY_FOR_TESTS` | ℹ️ Info | Pre-existing test fixture value, unrelated to this phase's own code. |
| `paladin_execution_service.rs` | 5843 | `CREDENTIAL_PLACEHOLDER` (comment referencing an existing symbol) | ℹ️ Info | Not a debt marker — a code comment naming a real constant. |

No unresolved debt markers. No blocker anti-patterns found.

### Known Limitations — Disclosed, Cross-Checked Against WINDOWS.md

| Item | WINDOWS.md entry | Assessment |
|---|---|---|
| `trace.heartbeat_interval_secs` parsed but not wired to the engine's rate limiter (hardcoded `DEFAULT_HEARTBEAT_INTERVAL = 5s`, coincidentally equal to the config default) | Documented in-code (`superstep.rs:399-403,2072-2073`); not a WINDOWS.md row | WARNING, non-blocking. Config field exists and validates; only a non-default operator-configured value would silently not apply. Does not affect the roadmap truth as stated (rate-limiting exists and works at the default). |
| `NodeProgress` events below the engine use a placeholder `NodeId` (WR-03) | Documented via code comment + REVIEW-FIX doc-only fix | WARNING, non-blocking. No current consumer reads `NodeProgress.node_id` (OTel no-ops it, SSE drops it) — latent, not user-visible today. |
| `run export` from Waypoints-only source with no resolvable graph document derives no edges (visits only) | WINDOWS.md #34 | WARNING, non-blocking. Narrow edge case (no `--graph`, no run-linked assistant version); the overlay's other three sourcing paths (explicit `--graph`, run-linked doc, trace-sourced) all produce edges. |
| SSE `parley`/`error` payload content reduced vs. pre-collapse (no `prompt`/`choices`/`expires_at`, no `cancelled`-vs-`halted`) | WINDOWS.md #33 | WARNING, non-blocking. Field names preserved (published contract intact); full detail still reachable via `GET /threads/{id}/state`, documented in the observability page. |
| **Superstep-overhead-with-tracing bench: +22.18% (log_sink) / +18.46% (composite) vs. the PRD's ≤3% acceptance bar (D-37, acceptance criterion 6)** | **No WINDOWS.md entry** | **Routed to human verification above — genuinely unadjudicated.** The measurement itself is honest, real, and correctly recorded (not a gap in the *measuring*), but the FAIL verdict against a PRD-named numeric bar has not been closed by a maintainer decision, unlike #33/#34 which both have closed, cross-referenced WINDOWS.md rows. |

### Human Verification Required

See the `human_verification` block in this file's frontmatter for the two items (bench-overhead adjudication; judgment-tier prohibition sign-off) in full detail.

## Gaps Summary

No must-have truth, artifact, or key link failed. All four roadmap success criteria are independently verified against the codebase and re-run tests, not merely SUMMARY.md claims. The one confirmed pre-verification BLOCKER (CR-01, `RunStarted.run_id` always `None`, breaking OTel↔Platform-API run correlation) was found by code review and is now fixed, tested, and confirmed present on HEAD by this verifier.

The phase does not fail any of the four ROADMAP success criteria. It is held at `human_needed` rather than `passed` because one PRD-named, previously-flagged acceptance criterion (≤3% tracing overhead) genuinely failed as measured and that failure has not yet received the close-out adjudication the phase's own CONTEXT explicitly calls for, and because six safety/privacy prohibitions are judgment-tier (not automatically enforced) and therefore require a human sign-off per the escalation-gate policy rather than a verifier-issued pass.

---

_Verified: 2026-09-09T13:27:03Z_
_Verifier: Claude (gsd-verifier)_
