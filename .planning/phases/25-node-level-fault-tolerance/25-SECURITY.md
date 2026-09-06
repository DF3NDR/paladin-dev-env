---
phase: 25
slug: node-level-fault-tolerance
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-09-06
---

# Phase 25 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 25 (Node-Level Fault Tolerance: Aegis retry, timeouts, error handlers, provider
fallback, node cache) shipped 14 plans, every one of which carried a `<threat_model>` block at
plan time. This file consolidates those 14 registers, records the verification evidence for each
mitigation, and logs the risks the phase accepted by decision. Ten of the fourteen SUMMARY files
carry a `## Threat Flags` section; all ten read "None" beyond the plan register. The other four
(25-01, 25-02, 25-03, 25-07) recorded no new threat surface either, and their mitigations are
pinned by the tests named below.

**Verification depth.** ASVS level 1 with `register_authored_at_plan_time: true`. The
preliminary grep-depth (L1) classification closed every entry, so per the secure-phase
short-circuit rule the deeper auditor pass was not spawned. Evidence below is a file and line, a
test name, or a document line that pins the mitigation; every test named in the register was
re-run live in this session (see "Verification Notes").

**Post-execution hardening folded in.** `/gsd-code-review 25 --fix` (25-REVIEW-FIX.md) landed
five commits after the plans closed that strengthen entries in this register rather than add new
ones: `21e9c989` (CR-01, Anthropic usage-cap hint read from the redacted body), `2ea6328b`
(CR-02, the three pre-Phase-17 adapters now refuse redirects), `1e50ca3f` (WR-01, DeepSeek's
private redaction copy replaced by the shared module), `724db484` (WR-02, Permanent LLM failures
no longer retried by `PaladinExecutionService`) and `6b566916` (Anthropic deserialization-failure
excerpt now redacted before bounding). They are cited where relevant in the Evidence column.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| provider HTTP response body → `LlmError` / `ProviderError.message` | Remote, attacker-influenceable bytes become an error field that is logged, persisted and (via `error_field`) written into Battlefield state | Untrusted text; must be redacted before it is bounded, and never read for classification |
| adapter API key → error excerpt | The credential is present in the process while the excerpt is built | Secret; must never survive into any excerpt, log or `Debug` rendering |
| `LlmError` (port layer) → `PaladinError::LlmFailure` (core taxonomy) | Provider-influenced status, provider name and message cross into the error the engine and circuit breaker branch on | Typed fields only decide transience |
| converted error → circuit breaker accounting | The breaker's open/close decision reads `is_retryable()` on the converted value | Behaviour must be unchanged from the legacy variant |
| node implementation → `StateNodeError` → `NodeError` → persisted Waypoint | Author- and provider-influenced text becomes durable state readable by any process with store access | Redacted, bounded summary; raw-content warning (M-B-04) applies |
| `NodeError` → `error_field` → Battlefield state | A structured failure summary is written into author-visible state another node or an LLM call may read | Structured `NodeError` only; documented raw-content warning |
| run `CancellationToken` → retry backoff sleep | A shutdown signal must interrupt a sleeping attempt within the grace window | Abort recorded `Skipped { reason: "shutdown" }`, re-listed for exactly-once resume |
| graph author → `WarGraph::set_aegis` / `Custom(name)` / `ErrorHandlerSpec` | Author-supplied policy values and registry names reach scheduling, timing and control flow | Validated fail-closed before any node executes |
| registered handler → engine control flow / visit accounting | An author-supplied handler decides where the run goes next; a compensation path could consume scheduling budget | Bounded by `max_node_visits`; no second counter |
| stored `GraphFingerprint` → `resume` | A persisted hash decides whether a Waypoint may continue against the in-process graph | `on_error` and `cache` now enter the fingerprint (`v5`) |
| node or provider → progress signal → idle timer | A progress report suppresses a safety timer | `run_timeout` is a hard wall-clock cap progress cannot extend |
| timed-out or failed attempt → Battlefield | A cancelled or failed attempt's partial work must not survive | Discarded structurally; only the succeeding attempt's `Directive` leaves the closure |
| concurrent Muster tasks → shared engine state | Independent retrying futures run in parallel over one superstep snapshot and one aggregation | Per-task attempt state inside each spawned future; no shared counters |
| provider chain → caller / provider stream → consumer | A response may come from a provider the caller did not name; a partial stream must not be completed by a different model | Hop only before the first chunk; every hop traced and named on `served_by` |
| engine → `NodeCachePort` → Redis; `NodeCacheKey` → shared keyspace | A `StateDelta` is written to and read back from an external store addressed by a composed key | Fingerprint-scoped, length-prefixed key; best-effort backend; namespaced by `key_prefix` |
| operator config → `redis_password` | A credential enters process configuration | Never `Debug`-printed; rendered as `[REDACTED]` |
| published crate API → downstream consumers | The migration register and semver allowlist tell a consumer which breaks were intentional | Set-equality checked in CI; non-exhaustive enums fail to compile, never silently change |
| dependency graph → the build | New advisories and license changes enter through dependencies | `make security` must pass; no new `audit.toml` suppression |
| documentation → operator and author behaviour | The guide is what an operator configures the cache and timeouts from | Limitations stated explicitly, including what the timeouts do not bound |
| evidence record → phase seal | A tier reported green that never ran is a false assurance | Docker-gated tiers recorded CI-only, never green locally |

---

## Threat Register

Status legend: **closed** = mitigation located in the implementation (Evidence column), or
accepted risk recorded in the Accepted Risks Log below. Paths are relative to `crates/` unless
they start with `src/`, `tests/` or `docs/`.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status | Evidence |
|-----------|----------|-----------|----------|-------------|------------|--------|----------|
| T-25-01 | Information Disclosure | `NodeError` / `NodeErrorSource.message` | high | mitigate | Provider-sourced excerpts are redacted **before** they are bounded; the ordering is documented on the type and no engine code interpolates an API key into a `NodeError` | closed | `paladin-core/src/platform/container/node_error.rs:16-28,82-83` (D-34 contract); negative grep `api_key` over `paladin-battalion/src/engine/` = 0 hits; upstream `paladin-llm/src/http_status.rs:87-92` |
| T-25-02 | Denial of Service | retry backoff sleep vs shutdown grace | high | mitigate | Every backoff wait is `tokio::select!`ed against the run's `CancellationToken`; a node in backoff at SIGTERM aborts at once and is recorded `Skipped { reason: "shutdown" }` | closed | `paladin-battalion/src/engine/retry.rs:81-83`; test `backoff_wait_returns_early_when_the_run_is_cancelled` (`retry.rs:336`); `superstep.rs:2583-2626`; stress pair in `tests/integration/aegis_retry_stress_test.rs` (abort < 1 s virtual against a 60 s backoff) |
| T-25-03 | Denial of Service | `RetryPolicy { max_attempts: 0 }` | medium | mitigate | Zero attempts is a typed validation error before execution, never "unlimited" | closed | test `max_attempts_zero_is_a_typed_validation_error` (`paladin-core/.../aegis.rs:275`; `paladin-battalion/src/engine/retry.rs`) |
| T-25-04 | Tampering | failed-attempt delta leaking into the Battlefield | high | mitigate | Each attempt clones the same immutable pre-superstep snapshot; only the succeeding attempt's `Directive` leaves the closure | closed | test `failed_attempt_delta_never_reaches_the_battlefield` (`paladin-battalion/src/engine/mod.rs`) |
| T-25-05 | Elevation of Privilege | retrying an interceptor policy decision | medium | mitigate | The retry loop wraps the interceptor chain; `InterceptDecision::Skip`/`Fail` are honoured outside the loop and never retried | closed | `paladin-battalion/src/engine/superstep.rs:2283-2286`; test `interceptor_fail_decision_is_not_retried` (`engine/mod.rs`) |
| T-25-06 | Information Disclosure | `ProviderError.message` / `LlmFailure.message` | high | mitigate | The message is populated only by the shared `map_http_status`, which redacts before bounding; classification never reads it | closed | `paladin-llm/src/http_status.rs:87-92` (single population point); `paladin-ports/src/output/llm_port.rs:519` reads typed fields only |
| T-25-07 | Tampering | transience classification | high | mitigate | `transience()` reads typed fields and variant identity only; no substring inspection on either path | closed | `paladin-ports/src/output/llm_port.rs:507-560` (typed `match`, `ProviderError { status, .. }` arm by value); negative grep `contains(`/`split(`/`parse::<u16>` over `paladin_error.rs` and `paladin-battalion/src/llm_failure.rs` = 0 |
| T-25-08 | Spoofing | downstream consumers matching on the enums | medium | accept | `#[non_exhaustive]` is a deliberate break registered in MIGRATION §9.2 with an allowlist entry; consumers get a compile error, not a silent behaviour change | closed (accepted) | `paladin_error.rs:28`, `llm_port.rs:300`, `paladin-battalion/src/engine/mod.rs:229`; `.cargo/semver-checks-allowlist.toml`; AR-25-01 |
| T-25-09 | Denial of Service | `circuit_breaker.rs` behaviour drift | high | mitigate | `is_retryable()` answers `true` for `LlmFailure` exactly as the legacy arm did; the breaker source is untouched | closed | test `legacy_retryability_predicates_are_unchanged` (`paladin_error.rs:354`); `src/infrastructure/resilience/circuit_breaker.rs` last commit `8cbc11fb` (2026-05-30, pre-phase) |
| T-25-10 | Elevation of Privilege | `RetryPredicate::Custom` / `ErrorHandlerSpec::Custom` resolution | high | mitigate | Unregistered names fail graph validation, listing every offender; a name never resolves to a default and there is no post-validation registration path | closed | `EngineError::UnregisteredRetryPredicate { names }` / `UnregisteredErrorHandler` (`engine/mod.rs:742,756`); tests `unregistered_custom_retry_predicate_fails_validation`, `unregistered_custom_error_handler_fails_validation`, `every_unregistered_custom_name_is_listed_sorted_and_deduped` (`graph.rs`) |
| T-25-11 | Tampering | `resume` against a redeployed graph | high | mitigate | `on_error` and `cache` enter the fingerprint under the bumped `v5` tag, so changed failure routing or cache policy cannot silently resume against an old Waypoint | closed | `paladin-battalion/src/engine/graph.rs:2051,2216`; test `fingerprint_golden_hex_v5` (`graph.rs:2898`) |
| T-25-12 | Denial of Service | `max_attempts: 0` / zero-duration timeouts | medium | mitigate | Typed validation errors before execution; `Duration::ZERO` is never an immediate-kill loop | closed | `graph.rs:1186-1195`; tests `timeout_policy_with_zero_duration_fails_validation` (`graph.rs:5605`), `max_attempts_zero_is_a_typed_validation_error` |
| T-25-13 | Denial of Service | Aegis on a node kind whose attempt unit is durable child state | medium | mitigate | `retry`/`cache` on a Battalion node and any Aegis on a Gate node are rejected at validation | closed | `graph.rs:974,981,989`; `EngineError::AegisUnsupportedForNodeKind` (`engine/mod.rs:786`); tests at `graph.rs:5483,5508,5570` |
| T-25-14 | Information Disclosure | validation error messages listing offenders | low | accept | Offender lists carry author-supplied node ids and policy names only, the same class `EngineError` already exposes | closed (accepted) | `graph.rs:1251`; AR-25-02 |
| T-25-15 | Tampering | cross-graph or cross-node cache-key collision | high | mitigate | The key includes the graph fingerprint by construction; the Redis adapter additionally namespaces by `key_prefix` | closed | `paladin-battalion/src/engine/cache_key.rs:4-5`; tests `keys_differing_only_by_graph_prefix_do_not_collide` (`paladin-storage/src/node_cache/{contract_tests,in_memory}.rs`), `redis_keys_are_namespaced_by_the_configured_prefix` (`node_cache/redis.rs`, CI-only tier) |
| T-25-16 | Denial of Service | an unavailable Redis failing runs | high | mitigate | A `get` failure is a miss; a `put` failure is logged and never fails the run | closed | `paladin-battalion/src/engine/superstep.rs:223,264` |
| T-25-17 | Denial of Service | `invalidate(prefix)` blocking the Redis server | medium | mitigate | Cursor-based `SCAN MATCH`, never a full-keyspace or whole-database command | closed | `paladin-storage/src/node_cache/redis.rs:14-15,235-252` (`scan_match`); grep `KEYS`/`FLUSH` in `redis.rs` = 0 |
| T-25-18 | Information Disclosure | `redis_password` in logs or `Debug` output | high | mitigate | Both config types implement `Debug` by hand and render the password as `[REDACTED]`; no log statement interpolates it | closed | `paladin-storage/src/node_cache/redis.rs:88-99` (fix `462a1442`); `src/config/node_cache.rs:96-105`; tests `debug_rendering_never_prints_the_password`, `src/config/node_cache.rs:278` |
| T-25-19 | Information Disclosure | cached `StateDelta` contents | medium | accept | A cached delta is exactly the state the node already writes to the Battlefield and Waypoints; it inherits M-B-04's raw-content warning and cache code adds no provider body or credential | closed (accepted) | `docs/src/user-guides/fault-tolerance.md:377-392`; AR-25-03 |
| T-25-20 | Information Disclosure | credential leaking through a truncated error excerpt | high | mitigate | `map_http_status` redacts through `redaction.rs` **before** bounding, in one place for all adapters; the wrong-order control is computed and asserted not to match | closed | `paladin-llm/src/http_status.rs:13-18,41-44,87-92`; test `excerpt_is_redacted_before_it_is_bounded` (`http_status.rs:199`); adjacent Anthropic paths closed post-review by `21e9c989` and `6b566916` (`anthropic/adapter.rs:354,366`) |
| T-25-21 | Information Disclosure | provider body embedded verbatim in a persisted error | medium | mitigate | Every excerpt is bounded after redaction, on character boundaries | closed | `paladin-llm/src/redaction.rs:50-55` (`chars()`-based); test `multibyte_body_is_bounded_on_char_boundaries` (`http_status.rs:233`) |
| T-25-22 | Tampering | provider shaping its error text to influence retry | high | mitigate | Transience is read from the typed `status`, never the message; the last status-in-text paths were removed | closed | `llm_port.rs:519` `ProviderError { status, .. }` arm; crate-wide negative grep recorded in `25-05-SUMMARY.md` Threat Flags |
| T-25-23 | Spoofing | one adapter drifting from the shared mapping | medium | mitigate | Exactly one mapping helper exists and every adapter calls it | closed | `grep -rn "fn map_http_status" paladin-llm/` = 1 definition; call sites in 9 adapters + `compat/engine.rs` |
| T-25-24 | Tampering | provider influencing transience through message text (conversion helper) | high | mitigate | `to_paladin_error` reads `transience()`, `status` and `provider` from typed fields only | closed | `paladin-battalion/src/llm_failure.rs` (`typed_origin`); negative grep = 0 (`25-06-SUMMARY.md:122`) |
| T-25-25 | Denial of Service | circuit breaker behaviour drift after the migration | high | mitigate | `is_retryable()` unchanged for `LlmFailure`; `circuit_breaker.rs` not edited | closed | test `legacy_retryability_predicates_are_unchanged`; `git log -- src/infrastructure/resilience/circuit_breaker.rs` shows no Phase 25 commit |
| T-25-26 | Information Disclosure | provider message carried into the core error | medium | mitigate | The message is the source `LlmError`'s own rendering, already redacted and bounded upstream; no new unredacted path | closed | `llm_failure.rs` (`message: err.to_string()`); `http_status.rs:87-92` |
| T-25-27 | Repudiation | log-line drift at the migrated sites | low | mitigate | Rendered text is byte-identical to the legacy erasure | closed | test `rendered_error_text_at_every_migrated_site_is_unchanged` (`src/application/services/paladin/paladin_execution_service.rs:2708`) |
| T-25-28 | Information Disclosure | `NodeError` persisted on a Failed Waypoint | high | mitigate | Every message reaching a `NodeError` was redacted and bounded upstream; payloads inherit the raw-content warning | closed | `node_error.rs:82-83`; `http_status.rs:87-92`; `fault-tolerance.md:392` |
| T-25-29 | Tampering | one Muster task's retry state leaking into a sibling | high | mitigate | Attempt counters and `AttemptRecord`s live inside each task's own spawned future with no shared captured state | closed | test `each_muster_task_has_its_own_attempt_counter` (`engine/mod.rs`); X-05 stress with exact per-task counts (`aegis_retry_stress_test.rs`) |
| T-25-30 | Tampering | a retry becoming a durable checkpoint boundary | medium | mitigate | No Waypoint is written between attempts; muster-progress Waypoints record only completed tasks | closed | tests `no_waypoint_is_written_between_attempts`, `muster_progress_waypoints_record_only_completed_tasks` (`engine/mod.rs`) |
| T-25-31 | Denial of Service | a Parley consuming a retry budget or being re-raised | medium | mitigate | `NextStep::Parley` leaves the retry loop as a success and is never classified by the predicate | closed | test `a_parley_directive_is_never_retried` (`engine/mod.rs`) |
| T-25-32 | Repudiation | an older reader failing on the new `Failed` payload | medium | mitigate | The field is additive and `#[serde(default)]`, with no reshape | closed | `paladin-core/src/platform/container/battalion/mod.rs:581-582` (`node_errors: Vec<NodeError>`); absent-key contract case on all three backends (25-07 SUMMARY) |
| T-25-33 | Tampering | silent provider switch mid-response | high | mitigate | Fall-through happens only before the first chunk; after any `Ok` chunk the error propagates and no hop occurs | closed | test `streaming_error_after_the_first_chunk_propagates_with_the_prefix` (`paladin-llm/src/fallback.rs`) asserting next-provider call count 0 |
| T-25-34 | Repudiation | the caller not knowing which provider answered | medium | mitigate | Every hop emits `TraceEvent::FallbackHop` and a `tracing::warn!`; the serving provider is stamped on `served_by` | closed | `fallback.rs:183`; `paladin-core/src/platform/container/execution_result.rs:94` |
| T-25-35 | Denial of Service | a permanent failure burning every provider in the chain | high | mitigate | `Permanent` short-circuits after one call | closed | test `permanent_error_short_circuits_after_one_call` (`fallback.rs`) |
| T-25-36 | Tampering | shared hop state across concurrent calls | high | mitigate | No mutable cross-call state; every call starts at element 0 | closed | tests `every_call_starts_at_the_first_provider`, `concurrent_calls_share_no_hop_state` (multi-thread, timeout-guarded; `fallback.rs`) |
| T-25-37 | Information Disclosure | provider error summaries in `AllProvidersFailed.attempts` | medium | mitigate | Each summary is the source `LlmError`'s own rendering, already redacted and bounded | closed | `fallback.rs`; `http_status.rs:87-92` |
| T-25-38 | Spoofing | a caller relying on `served_by` for provenance | low | accept | `served_by` is an observability field stamped in-process, not an attestation; its rustdoc says so and no security decision reads it | closed (accepted) | `execution_result.rs:94`; AR-25-04 |
| T-25-39 | Denial of Service | a node beating forever to evade its idle timeout | high | mitigate | `run_timeout` is a hard wall-clock cap progress cannot extend; the engine-level `run_timeout` bounds the whole run | closed | test `a_slow_but_progressing_node_fails_on_run_timeout_not_idle` (`superstep.rs`); `Timeout(EngineRun)` never retried (`aegis_retry_stress_test.rs`) |
| T-25-40 | Denial of Service | a stalled stream holding a superstep open indefinitely | high | mitigate | `idle_timeout` fires on absence of progress; a port that never beats degrades to the wall clock, never to no bound | closed | test `a_port_that_stalls_300ms_fails_with_timeout_idle` (`superstep.rs`); `fault-tolerance.md:377` limitation 1 |
| T-25-41 | Tampering | a timed-out attempt's partial delta reaching the Battlefield | high | mitigate | Cancellation discards the attempt exactly as any other failure does | closed | test `a_timed_out_attempts_partial_work_is_discarded` (`superstep.rs`) |
| T-25-42 | Tampering | misreading which bound fired | medium | mitigate | The fired bound is a typed `TimeoutKind` on the `NodeError`, never inferred from a message | closed | `node_error.rs:51`; test `the_fired_bound_is_read_from_the_typed_kind` (`superstep.rs`) |
| T-25-43 | Elevation of Privilege | a required method appearing on a published trait | medium | mitigate | `execute_observed` is defaulted, delegating to `execute` | closed | `paladin-ports/src/output/paladin_port.rs:747-753`; test `execute_observed_defaults_to_execute` (`paladin_port.rs:895`); MIGRATION §9.2 `N` row |
| T-25-44 | Denial of Service | a hanging `EdgeConditionEvaluator` | medium | accept | **R-23-01 stays accepted, not closed.** Per-attempt timeouts wrap node execution, not edge evaluation | closed (accepted) | `fault-tolerance.md:396` limitation 4; `23-SECURITY.md:123`; AR-25-05 |
| T-25-45 | Information Disclosure | `error_field` carrying provider-influenced content into state | medium | mitigate | The written value is the structured `NodeError`, redacted and bounded upstream; `error_field` is author-visible state under the raw-content warning | closed | test `route_writes_the_structured_error_and_places_the_target` (`superstep.rs`); `fault-tolerance.md:392` |
| T-25-46 | Tampering | `Route` writing into a field whose dispatch corrupts the value | medium | mitigate | `error_field` must not use `Sum` dispatch; rejected at validation with an explanatory message | closed | test `route_error_field_must_not_use_sum_dispatch` (`graph.rs`) |
| T-25-47 | Denial of Service | an unbounded compensation cycle | high | mitigate | Handler-routed visits count against `max_node_visits` through the existing accounting, no second counter, no exemption | closed | test `a_compensation_cycle_terminates_with_the_visit_limit` (`superstep.rs`, timeout-guarded) |
| T-25-48 | Elevation of Privilege | a handler running before the retry policy is honoured | medium | mitigate | A handler is entered only on exhaustion or non-retryability | closed | tests `a_handler_does_not_run_while_retries_remain`, `a_non_retryable_error_reaches_the_handler_immediately` (`superstep.rs`) |
| T-25-49 | Tampering | a `Route` target that is not a real node, or is a worker template | medium | mitigate | Both are typed validation errors before any node runs; the template message names the alternative | closed | `EngineError::RouteTargetUnknown` / `RouteTargetIsWorkerTemplate` (`engine/mod.rs:1054,1069`); `graph.rs:1232-1233` |
| T-25-50 | Tampering | a handler observing partially merged state | medium | mitigate | The handler receives the same immutable pre-superstep snapshot the node's attempts read | closed | `superstep::dispatch_error_handler` (25-10 SUMMARY:21); test `a_custom_handler_receives_the_structured_error_and_the_battlefield` (`superstep.rs`) |
| T-25-51 | Tampering | a mustered task escaping its aggregation | high | mitigate | `Route` on a worker template is a validation error; a `Custom` handler on a template must return a delta-only `Directive` or the run fails with a typed error naming node and task key | closed | `EngineError::HandlerNotAllowedOnWorkerTemplate` (`engine/mod.rs:905`), `MusterHandlerMustBeDeltaOnly` (`mod.rs:1154-1159`); `superstep.rs:666`; four delta-only tests (25-11 SUMMARY Threat Flags) |
| T-25-52 | Denial of Service | an unbounded compensation cycle (end to end) | high | mitigate | `max_node_visits` bounds handler-routed visits; proven end to end under a timeout guard | closed | test `compensation_loop_terminates_at_the_visit_limit` (`tests/integration/e2e_compensation_chain_test.rs`) |
| T-25-53 | Information Disclosure | a handler-authored `ParleyRequest` persisted verbatim | medium | mitigate | The request is author-supplied content persisted exactly as a node-raised parley already is; the E2E handler places only the redacted `NodeErrorSource` display string in the payload; guide warning covers it | closed | 25-11 SUMMARY Threat Flags; `fault-tolerance.md:392`; Phase 24 T-24-02 lineage |
| T-25-54 | Tampering | a second, non-durable suspension path | high | mitigate | The handler's `Parley` reuses the existing HITL-01 suspension path | closed | exactly one production `WaypointStatus::AwaitingInput {` construction in the engine (`superstep.rs:3307`) |
| T-25-55 | Elevation of Privilege | a handler consuming retry budget to extend a node's lifetime | low | mitigate | A `Parley` is not an attempt failure and consumes no retry budget | closed | test `a_handler_raised_parley_does_not_consume_the_retry_budget` (`superstep.rs`) |
| T-25-56 | Tampering | cross-task retry-state leakage under concurrency | high | mitigate | Multi-thread stress asserts exact per-task counts with no tolerance | closed | `tests/integration/aegis_retry_stress_test.rs` (24 tasks × 3 plans; 16-task two-failing case; `flavor = "multi_thread"`) |
| T-25-57 | Denial of Service | shutdown grace consumed by a sleeping backoff | high | mitigate | Kill-during-backoff aborts well inside the remaining backoff; the node is recorded `Skipped` and re-listed | closed | `aegis_retry_stress_test.rs` kill-during-backoff pair under `start_paused`; `retry.rs:81-83` |
| T-25-58 | Tampering | a resume continuing a partially attempted node | high | mitigate | No Waypoint is written between attempts, so a resume restarts at attempt 1 | closed | test `resuming_after_a_kill_during_backoff_restarts_at_attempt_one` (`aegis_retry_stress_test.rs`) |
| T-25-59 | Repudiation | a green E2E-3 that proves nothing | high | mitigate | The scripted stand-in is deleted; the default predicate is used unwidened; exactly 7 port calls are asserted | closed | grep `PHASE 25 SEAM` over `tests/` = 0; test `one_worker_recovers_by_real_per_task_retry` (`tests/integration/e2e_muster_defer_order_test.rs:518`) |
| T-25-60 | Denial of Service | a stress or timing test hanging CI | medium | mitigate | Every concurrent test carries a timeout guard; timing tests use a paused clock | closed | 7 `timeout(` guards in `aegis_retry_stress_test.rs`; `start_paused` cases |
| T-25-61 | Tampering | cross-graph or stale-configuration cache hit | high | mitigate | The key includes the graph fingerprint and the Paladin config fingerprint, so a graph or prompt change invalidates by construction | closed | tests `key_includes_the_graph_fingerprint` (`cache_key.rs`), `changing_the_graph_re_executes`, `changing_the_system_prompt_re_executes` (`superstep.rs`) |
| T-25-62 | Tampering | key collision from unescaped concatenation | high | mitigate | Same canonical length-prefixed encoding `WarGraph::fingerprint()` adopted at its `v1` → `v2` bump | closed | `cache_key.rs:4-5,26,46` |
| T-25-63 | Denial of Service | a cache backend failure failing runs | high | mitigate | `get` failure is a miss; `put` failure is logged only | closed | tests `a_get_failure_is_a_miss_not_an_error`, `a_put_failure_leaves_the_run_completed` (`superstep.rs`); `superstep.rs:223,264` |
| T-25-64 | Tampering | caching a failure or a partial result | high | mitigate | `put` happens only after a successful attempt; a failed attempt issues zero `put` calls | closed | test `a_failed_attempt_is_never_stored` (`superstep.rs`) |
| T-25-65 | Repudiation | a silently uncached node the author asked to cache | medium | mitigate | A `CachePolicy` with no configured backend is a typed validation error, not a no-op | closed | `WarGraph::validate_node_cache_backend` (`graph.rs:1109`) |
| T-25-66 | Information Disclosure | cached delta contents (plan 25-13 restatement of T-25-19) | medium | accept | Same accepted posture as T-25-19 | closed (accepted) | AR-25-03 |
| T-25-67 | Tampering | `Append`-dispatch replay duplication on a fork | medium | mitigate | `CacheMarker::Deny` lets an author exclude an `Append` field, validated fail-closed; the fork replay hazard is documented | closed | `graph.rs:1013-1043`; `fault-tolerance.md:377` limitation 2 |
| T-25-68 | Repudiation | an unregistered breaking change shipping silently | high | mitigate | `cargo semver-checks` against 0.9.0 and the allowlist/§9.2 set-equality confirmed in both directions | closed | `25-14-SUMMARY.md:45,66-70,103` (semver 11/11 exit 0; set-equality PASS after the `paladin-ai-core` crate-name fix that had made the pre-fix run FAIL) |
| T-25-69 | Tampering | a vulnerable or newly advised dependency | high | mitigate | `make security` passes; no new `.cargo/audit.toml` suppression | closed | `25-14-SUMMARY.md:154` (`cargo audit` + `cargo deny` exit 0; `audit.toml` diff = 0 lines) |
| T-25-70 | Information Disclosure | a credential reaching a log, an error or a cached payload | high | mitigate | Manual credential-handling review performed and recorded on every path the phase added; one finding fixed in-plan, four adjacent findings fixed post-review | closed | `25-14-SUMMARY.md:168-182`; `462a1442`; `25-REVIEW-FIX.md` commits `21e9c989`, `2ea6328b`, `1e50ca3f`, `6b566916`; `anthropic/adapter.rs:163` `Policy::none()` |
| T-25-71 | Denial of Service | a hanging `EdgeConditionEvaluator` (close-out restatement) | medium | accept | **R-23-01 stays accepted.** Re-listed in the guide, MIGRATION §9.1 and the 25-14 SUMMARY rather than claimed closed | closed (accepted) | `25-14-SUMMARY.md:178`; `fault-tolerance.md:396`; AR-25-05 |
| T-25-72 | Repudiation | reporting a tier as green that never ran | high | mitigate | Redis and Postgres contract tiers recorded as CI-only evidence naming the Docker-gated jobs; never reported green locally | closed | `25-14-SUMMARY.md:92,164`; UAT evidence: CI run `34051074633` green on the phase head (STATE.md:8) |
| T-25-73 | Repudiation | a traceability anchor pointing at a non-existent test | medium | mitigate | Every anchor verified against the named file before it was written | closed | `25-14-SUMMARY.md:81,128` (86/86 anchors resolve) |
| T-25-SC | Tampering | package-manager installs (plans 25-01 … 25-14) | low | accept | No new third-party package entered the build; see Accepted Risks Log | closed (accepted) | RESEARCH.md Package Legitimacy Audit (zero proposed); `Cargo.lock` gained only a direct `blake3` edge (25-13); AR-25-06 |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Totals:** 74 register entries (73 numbered plus the consolidated supply-chain row) — 39 high,
30 medium, 5 low. 66 mitigated and verified, 8 accepted. **threats_open: 0.**

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-25-01 | T-25-08 | `LlmError`, `PaladinError` and `EngineError` are now `#[non_exhaustive]`. A downstream crate that matched exhaustively gains a compile error rather than a silent behaviour change, which is the safe failure direction. Registered as deliberate-breaking under X-10.6 in MIGRATION §9.2 with a `.cargo/semver-checks-allowlist.toml` entry; CI checks the two sets for equality in both directions. | Plan 25-02 (D-01 / X-10.6) | 2026-09-06 |
| AR-25-02 | T-25-14 | Validation errors that list offenders (`UnregisteredRetryPredicate { names }` and siblings) name graph-author-supplied node ids and policy names only — the same class of author-visible content `EngineError`'s existing CF-01 messages already carry. No provider content or credential reaches these messages. | Plan 25-03 (D-13) | 2026-09-06 |
| AR-25-03 | T-25-19, T-25-66 | A cached `StateDelta` is exactly the state the node already writes to the Battlefield and persists in Waypoints; caching adds no new content class. It inherits M-B-04's raw-content warning for author-visible state, restated as limitation 3 of the fault-tolerance guide. Cache code adds no provider response body or credential to the delta. | Plans 25-04 and 25-13 (D-34) | 2026-09-06 |
| AR-25-04 | T-25-38 | `PaladinResult.served_by` is an observability field stamped in-process by the fallback adapter, not an authenticated attestation of provenance. Its rustdoc says so, and no security decision in the tree reads it. | Plan 25-08 (D-26) | 2026-09-06 |
| AR-25-05 | T-25-44, T-25-71 | **R-23-01 remains accepted.** A hostile or hanging `EdgeConditionEvaluator` is author-supplied in-process code with the same trust as a `StateNode`. Phase 25's per-attempt `run_timeout`/`idle_timeout` and the run-level `EngineLimits.run_timeout` wrap node execution, not edge evaluation; `EngineLimits::max_supersteps` stays the run-level bound. Re-listed here, in the guide (limitation 4), in MIGRATION §9.1 and in 25-14's SUMMARY so the new timeout machinery does not read as coverage it does not provide. | Plans 25-09 and 25-14 (D-34); original acceptance in 23-SECURITY.md R-23-01 | 2026-09-06 |
| AR-25-06 | T-25-SC | Supply chain. No new `[[package]]` entered `Cargo.lock` in any of the 14 plans; RESEARCH.md's Package Legitimacy Audit recorded zero proposed packages and `rand 0.8` / `redis 0.32.2` were already declared. Changes that touched the dependency graph without adding a package: the `redis-cache` feature reuses the existing optional `redis` dependency with the `safe_iterators` feature enabled (25-04); `blake3 1.8.2` became a direct edge of `paladin-battalion` (25-13, already in the graph); three `[[test]]` entries were registered in the root `Cargo.toml`. `make security` (cargo-audit + cargo-deny) passed on the phase head with `.cargo/audit.toml` unchanged (25-14). | Plans 25-01 … 25-14 | 2026-09-06 |

*Accepted risks do not resurface in future audit runs.*

---

## Verification Notes

- **Method.** Grep-depth (L1) verification per the secure-phase short-circuit rule: ASVS level 1,
  register authored at plan time, zero open entries after preliminary classification. The
  `gsd-security-auditor` subagent was therefore not spawned. Each row's Evidence column names the
  file and line, test, or document line that pins the mitigation. Negative greps recorded in the
  register (`api_key` in the engine, `KEYS`/`FLUSH` in the Redis adapter, substring inspection on
  the transience paths, `PHASE 25 SEAM` in `tests/`) were re-run in this session and returned 0.
- **Targeted test re-run.** Every security-pinning test named in the register was re-run live on
  2026-09-06 on a warm build cache; every invocation was green. Counts per invocation:
  `paladin-ai-core` 2 passed; `paladin-ports` 1 passed; `paladin-llm --all-features` 6 passed
  (redact-then-bound control, char-boundary bound, four fallback-chain tests); `paladin-storage
  --features redis-cache` 4 passed (key non-collision, namespace test, `SCAN` pattern literal,
  password `Debug` redaction); `paladin-battalion` 31 passed (retry cancellation and zero-attempt
  validation, attempt isolation, interceptor non-retry, Muster attempt isolation, no inter-attempt
  Waypoint, Parley non-retry, all four timeout tests, `Sum`-dispatch rejection, compensation
  bound, handler ordering, retry-budget isolation, five cache tests, six unregistered-`Custom`
  validation tests, zero-duration rejection, `v5` fingerprint pin); root `paladin-ai` lib 6 passed
  (byte-identical rendering at every migrated site, `NodeCacheConfig` redaction and validation);
  integration binaries `aegis_retry_stress` 37 passed, `e2e_compensation_chain` 5 passed,
  `e2e_muster_defer_order::one_worker_recovers_by_real_per_task_retry` 1 passed. **Total 93 passed,
  0 failed.** The Redis live-server case `redis_keys_are_namespaced_by_the_configured_prefix`
  self-skipped locally (no Docker daemon); its real evidence is the green `redis-cache-integration`
  job on CI run `34051074633`, consistent with T-25-72.
- **Noteworthy observations.**
  - T-25-20 and T-25-70 shipped stronger than planned. The mandated credential-handling review
    fixed `RedisNodeCacheConfig`'s derived `Debug` in-plan (`462a1442`), and the post-execution
    review-fix pass closed four adjacent findings outside the plan registers: two Anthropic paths
    that bounded a body before redacting it (`21e9c989`, `6b566916`), DeepSeek's private copy of
    the redaction helpers (`1e50ca3f`), and the three pre-Phase-17 adapters that followed
    redirects with a credential header attached (`2ea6328b`). The security instruction "HTTP
    clients sending a credential header do not follow redirects" is now met by every adapter.
  - Two pre-existing items surfaced by the review are recorded in `deferred-items.md` and are
    **not** Phase 25 threats: `RedisQueueConfig` (v0.8 queue config) still derives `Debug` and
    `Serialize` over a raw `redis_password` (not `Debug`-formatted anywhere in-tree today), and
    `RedisNodeCache::scan_pattern` interpolates the prefix into a `SCAN MATCH` glob without
    escaping (REVIEW-FIX IN-01, Info severity; unreachable with today's `NodeId`/`FieldName`
    grammar, since the prefix is fingerprint hex plus operator-configured `key_prefix`).
  - `cargo audit` reports two `unsound` informational advisories (`RUSTSEC-2026-0221`
    event-listener, `RUSTSEC-2026-0205` scc) and two yanked versions (`chacha20 0.10.0`,
    `spin 0.9.8`) as allowed warnings; neither tool fails under the configured policy and no
    suppression was added (T-25-69). `SECURITY-EXCEPTIONS.md` does not yet record them; 25-14
    left that decision to the orchestrator and it remains open as a housekeeping item, not a
    threat.
  - Known gap carried from `security.instructions.md`: there is still no merge-gating Rust SAST.
    The manual credential-handling review in 25-14 plus the review-fix pass are the primary
    control for T-25-01/06/18/20/28/70; CodeQL stays advisory-only.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-06 | 74 | 74 | 0 | /gsd-secure-phase 25 (Claude, L1 grep-depth + targeted test re-run) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-06
