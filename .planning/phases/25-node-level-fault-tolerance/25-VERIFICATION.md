---
phase: 25-node-level-fault-tolerance
verified: 2026-09-06T03:38:04Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 25: Node-Level Fault Tolerance Verification Report

**Phase Goal:** Individual nodes retry with provable backoff, distinguish stalled from slow work via
nested timeouts, compensate typed errors instead of failing the run, fail over across LLM providers,
and cache node results — all governed by an Aegis policy bundle, without changing persisted
Waypoint/Battlefield shapes.

**Verified:** 2026-09-06T03:38:04Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (FT-01) | `transience()` on `PaladinError`/`LlmError` is table-driven per variant; provider adapters carry status-carrying error variants with no string parsing; `BattalionError::Node(NodeError)` carries a structured `NodeError`; every touched enum is `#[non_exhaustive]` and registered in `MIGRATION.md` §9.2 | ✓ VERIFIED | `paladin_error.rs:141` (`transience()`), `llm_port.rs:519` (`transience()`), `LlmError::ProviderError{provider,status,message}` (`llm_port.rs:482`), `BattalionError::Node(NodeError)` (`battalion/mod.rs:839`); all three enums `#[non_exhaustive]` (`paladin_error.rs:28`, `llm_port.rs:300`, `battalion/mod.rs:750`); `MIGRATION.md` §9.2 rows 131-137 resolved `Y`/`N`; `.cargo/semver-checks-allowlist.toml` has exactly 5 `[[entry]]` blocks / 3 `enum_marked_non_exhaustive` lints, set-equal with the 5 `Y` rows (both counted directly). Ran `cargo test -p paladin-ai-core --lib -- transience_table` → pass. |
| 2 (FT-02) | Per-node Aegis retry follows exact backoff under a paused clock with asserted jitter bounds, gates on transience (Permanent → 1 attempt), discards failed-attempt deltas while keeping `AttemptRecord` history, retries per-task inside a Muster | ✓ VERIFIED | `crates/paladin-battalion/src/engine/retry.rs` (`backoff_delay`/`wait_backoff`/`should_retry`); ran `backoff_sequence_is_exact_with_jitter_off`, `permanent_error_under_transient_only_takes_one_attempt`, `backoff_wait_returns_early_when_the_run_is_cancelled` → all pass. Per-task Muster retry proven by `one_worker_recovers_by_real_per_task_retry` (`cargo test --test e2e_muster_defer_order` → 37/37 pass, replacing the Phase 23 scripted seam) and `muster_with_per_task_retry_under_concurrency_has_exact_counts`/`concurrent_tasks_do_not_share_retry_state` (`cargo test --test aegis_retry_stress` → 37/37 pass). Kill-during-backoff resume-from-attempt-1 proven by `resuming_after_a_kill_during_backoff_restarts_at_attempt_one` (same run). |
| 3 (FT-03) | A wall-clock `run_timeout` and progress-aware `idle_timeout` are distinguished and nested with engine/Battalion bounds so the tightest fires and the error names which | ✓ VERIFIED | `crates/paladin-core/src/platform/container/heartbeat.rs` (`HeartbeatHandle`), `PaladinPort::execute_observed` default method (`llm_port.rs`/`paladin_port.rs`), `TimeoutKind::{Run,Idle,EngineRun}` (`node_error.rs`), `EngineError::RunTimeoutExceeded`. Ran `a_timed_out_attempts_partial_work_is_discarded`, `a_slow_but_progressing_node_fails_on_run_timeout_not_idle`, `a_port_that_stalls_300ms_fails_with_timeout_idle`, `the_tightest_bound_fires` (all pass) plus `an_engine_run_timeout_tighter_than_both_bounds_names_enginerun`/`a_node_run_timeout_tighter_than_the_engine_budget_names_run` from `aegis_retry_stress` (pass) — the nested-bound naming rule is exercised over a real `SqliteWaypointStore`, not just unit doubles. |
| 4 (FT-04) | Program scenario E2E-3 passes together with CF-03: `Route`/`Absorb`/registered-`Custom` compensates a transiently-failing Muster worker without failing the run; unregistered `Custom` fails closed; handler loops bounded by `max_node_visits` | ✓ VERIFIED | `crates/paladin-battalion/src/engine/superstep.rs::dispatch_error_handler` (Route/Absorb/Custom, `superstep.rs:679`); `WarGraph::validate_aegis_handler_wiring` fail-closed clauses (`UnregisteredRetryPredicate`/`UnregisteredErrorHandler`, `graph.rs`). Ran `cargo test --test e2e_compensation_chain` → 5/5 pass, including `compensation_chain_routes_a_permanent_failure_to_a_recovery_node`, `compensation_loop_terminates_at_the_visit_limit` (max_node_visits bound), and `on_payment_failure_parley_a_human` (handler→Parley composition across a simulated process drop/resume). |
| 5 (FT-05, FT-06) | `FallbackLlmAdapter` fails over on Transient/Unknown only (short-circuits Permanent) without silently switching providers mid-stream; a `CachePolicy`-keyed node hits its `NodeCachePort` cache with `cache_hit: true` and no re-execution; failures are never cached | ✓ VERIFIED | `crates/paladin-llm/src/fallback.rs` (`FallbackLlmAdapter`, `AllProvidersFailed`, first-chunk streaming peek at `generate_stream`); `PaladinResult.served_by` (`execution_result.rs:94`, MIGRATION §9.2 row 136, `constructible_struct_adds_field` allowlist entry). `crates/paladin-ports/src/output/node_cache_port.rs` (`NodeCachePort`), `WarEngine::with_node_cache` (`engine/mod.rs:1514`), `engine::cache_key` (D-28 composition). Ran `a_hit_merges_the_stored_delta_with_no_execution`, `a_cache_hit_consumes_no_retry_budget`, `a_hit_emits_node_started_and_finished_with_cache_hit_true`, `cache_hit_defaults_to_false_on_every_record_and_event`, and the 8 `engine::cache_key::tests::*` (key composition/stability/collision-avoidance) → all pass. |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/transience.rs` | `Transience` value type, not `#[non_exhaustive]` | ✓ VERIFIED | Present, `Copy+Eq+Hash+Serialize+Deserialize`, exhaustive by design (D-01) |
| `crates/paladin-core/src/platform/container/node_error.rs` | `NodeError`/`NodeErrorSource`/`TimeoutKind`/`AttemptRecord` | ✓ VERIFIED | Present, serde value family, redact-before-bound documented and tested |
| `crates/paladin-core/src/platform/container/aegis.rs` | `Aegis`, `RetryPolicy`, `RetryPredicate`, `TimeoutPolicy`, `ErrorHandlerSpec`, `CachePolicy`, `CacheKeySpec` | ✓ VERIFIED | All seven types present with `Aegis::validate()` |
| `crates/paladin-battalion/src/engine/retry.rs` | Backoff math + attempt loop | ✓ VERIFIED | `backoff_delay`/`wait_backoff`/`should_retry`; 12 tests incl. 7 paused-clock, all pass |
| `crates/paladin-battalion/src/retry_predicate.rs`, `error_handler.rs`, `engine/registries.rs` | Fail-closed registries (FT-02/FT-04) | ✓ VERIFIED | Mirrors `edge_evaluator.rs` pattern; `EngineRegistries` bundle wired into `WarGraph::validate`/`WarEngine` |
| `crates/paladin-core/src/platform/container/heartbeat.rs` | `HeartbeatHandle` progress channel | ✓ VERIFIED | `watch`-based, `NodeContext.attempt`/`heartbeat()`, `PaladinPort::execute_observed` default method |
| `crates/paladin-llm/src/fallback.rs` | `FallbackLlmAdapter` | ✓ VERIFIED | Present, ungated (ADR-0046), ~30 unit tests incl. streaming first-chunk rule |
| `crates/paladin-core/src/platform/container/node_cache.rs`, `crates/paladin-ports/src/output/node_cache_port.rs`, `crates/paladin-storage/src/node_cache/{in_memory,redis,contract_tests}.rs` | `CachedDelta`, `NodeCachePort`, InMemory + Redis adapters | ✓ VERIFIED | Present; shared 9-case contract suite; `redis-cache` feature ungated from default; Redis Tier-2 self-skips locally (no Docker), CI-only evidence (documented, not a gap per orchestrator note) |
| `crates/paladin-battalion/src/engine/cache_key.rs` | D-28 key composition | ✓ VERIFIED | `H(graph_fingerprint, node_id, input_component, paladin_config_fingerprint)`; 8 tests pass |
| `docs/src/user-guides/fault-tolerance.md`, `crates/doc-examples/src/fault_tolerance.rs` | Fault-tolerance user guide | ✓ VERIFIED | Present, registered in `docs/src/SUMMARY.md:25`; doc-examples crate builds and its lib test passes |
| `MIGRATION.md` §9.1-§9.7 | Register close-out | ✓ VERIFIED | 4 `Y` + 1 `N` §9.2 rows present with justifications; §9.1/§9.3/§9.4/§9.5/§9.7 notes present; deliberate-zero note covers new-in-0.10 types |
| `.cargo/semver-checks-allowlist.toml` | Set-equal with §9.2 `Y` rows | ✓ VERIFIED | 5 `[[entry]]` blocks, 3 `enum_marked_non_exhaustive` + 1 `constructible_struct_adds_field` (+1 pre-existing `paladin-web` entry), matching the 5 `Y` rows exactly |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `WarGraph::aegis_for(node_id)` | `superstep.rs` dispatch closure | Retry loop wraps trace→interceptor-before→execute→interceptor-after→trace per attempt | ✓ WIRED | `interceptors_run_once_per_attempt_not_once_per_node` and `interceptor_fail_decision_is_not_retried` pass — proves the Aegis wraps OUTSIDE the interceptor chain (D-14) |
| `LlmError::transience()` | `PaladinError::LlmFailure` | `paladin-battalion::llm_failure::to_paladin_error` (single conversion helper, 8 erasure sites migrated) | ✓ WIRED | `crates/paladin-battalion/src/llm_failure.rs`; 25-06 migrated all four `paladin_execution_service.rs` sites + `temperature_service.rs`; circuit-breaker retryability guard test passes |
| `Aegis.on_error` | `dispatch_error_handler` | `NodeError` exhaustion → `Route`/`Absorb`/`Custom` dispatch → `Directive` | ✓ WIRED | `e2e_compensation_chain` 5/5 pass; `Route` target auto-eligible via the reachability worklist (22-15 insertion point), proven by `route_writes_the_structured_error_and_places_the_target` |
| `FallbackLlmAdapter` | `PaladinResult.served_by` | `LlmResponse.metadata["paladin.served_by"]` → `PaladinExecutionService::execute_internal` copy | ✓ WIRED | `served_by_is_absent_from_legacy_json` (backward-compat) + fallback-chain tests in `fallback.rs` |
| `CachePolicy` | `NodeCachePort::get/put` | `engine::cache_key` composed key, lookup before attempt 1, put only on successful `Edges`-routed delta | ✓ WIRED | `a_hit_merges_the_stored_delta_with_no_execution`, `a_cache_hit_consumes_no_retry_budget` pass; `CachePolicyWithoutCacheBackend`/`CachePolicyOnDeniedField` fail-closed validation present in `graph.rs` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Backoff sequence exact under paused clock | `cargo test -p paladin-battalion --lib -- backoff_sequence_is_exact_with_jitter_off` | 1 passed | ✓ PASS |
| Permanent error takes exactly 1 attempt | `cargo test -p paladin-battalion --lib -- permanent_error_under_transient_only_takes_one_attempt` | 1 passed | ✓ PASS |
| Backoff aborts immediately on cancellation | `cargo test -p paladin-battalion --lib -- backoff_wait_returns_early_when_the_run_is_cancelled` | 1 passed | ✓ PASS |
| Timed-out attempt discards partial work; tightest bound fires | `cargo test -p paladin-battalion --lib -- a_timed_out_attempts_partial_work_is_discarded a_slow_but_progressing_node_fails_on_run_timeout_not_idle a_port_that_stalls_300ms_fails_with_timeout_idle the_tightest_bound_fires` | 4 passed | ✓ PASS |
| E2E-3 seam replaced by real per-task retry | `cargo test --test e2e_muster_defer_order` | 37 passed (incl. `one_worker_recovers_by_real_per_task_retry`) | ✓ PASS |
| Compensation chain + loop bound + handler-Parley | `cargo test --test e2e_compensation_chain` | 5 passed | ✓ PASS |
| Multi-thread retry stress, kill-during-backoff, nested timeout naming | `cargo test --test aegis_retry_stress` | 37 passed | ✓ PASS |
| Cache hit path (no execution, cache_hit flag, no retry-budget consumption) | `cargo test -p paladin-battalion --lib -- cache_hit lookup_before_attempt merges_the_stored_delta` | 4 passed | ✓ PASS |
| Cache key composition (stability, collision-avoidance, prompt-change invalidation) | `cargo test -p paladin-battalion --lib -- cache_key::` | 8 passed | ✓ PASS |
| `transience()` table-driven classification | `cargo test -p paladin-ai-core --lib -- transience_table` | 1 passed | ✓ PASS |
| Doc-examples crate (fault-tolerance guide anchors compile) | `cargo test -p paladin-doc-examples --lib` / `--doc` | 1 passed / 0 (no doctests, anchors are `ANCHOR` regions) | ✓ PASS |
| Workspace lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 | ✓ PASS |

Full-suite evidence (`cargo test --workspace`, `cargo llvm-cov`, `cargo semver-checks`, MSRV, `make security`) was not re-run in this verification per the orchestrator's constraint (disk-constrained sandbox; already recorded in `25-14-SUMMARY.md`'s Gate Evidence table: 4413 passed/0 failed, semver clean 11/11, MSRV 1.88 clean, `make security` clean, 89.34% coverage vs 82% floor). The targeted re-runs above independently confirm the specific behaviors the roadmap's five success criteria assert, rather than trusting the SUMMARY narrative alone.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| FT-01 | 25-01, 25-02, 25-05, 25-06, 25-07, 25-14 | Error taxonomy, transience(), structured NodeError, X-10 register | ✓ SATISFIED | Code + tests above; REQUIREMENTS.md checkbox already `[x]` |
| FT-02 | 25-01, 25-03, 25-07, 25-12, 25-14 | Aegis retry, backoff, predicate gating, per-task Muster retry | ✓ SATISFIED | Code + tests above |
| FT-03 | 25-09, 25-12, 25-14 | Nested run/idle timeouts, heartbeat, RunTimeoutExceeded | ✓ SATISFIED | Code + tests above |
| FT-04 | 25-03, 25-10, 25-11, 25-12, 25-14 | Route/Absorb/Custom handlers, E2E-3, max_node_visits | ✓ SATISFIED | Code + tests above |
| FT-05 | 25-08, 25-14 | FallbackLlmAdapter, served_by | ✓ SATISFIED | Code + tests above |
| FT-06 | 25-04, 25-13, 25-14 | NodeCachePort, InMemory/Redis, engine cache integration | ✓ SATISFIED | Code + tests above; Redis live-server tier is CI-only (Docker absent locally), a pre-existing, documented tiering convention (Phase 22 D-10), not a gap |

**Orphaned requirements:** none — `.planning/REQUIREMENTS.md`'s "Node-Level Fault Tolerance (Doc 04, epic FT)" section lists exactly FT-01..FT-06, and all six are claimed across the 14 plans' `requirements:` frontmatter (cross-checked against every `25-*-PLAN.md`).

**Documentation-tracking gap (non-blocking):** `.planning/REQUIREMENTS.md`'s checkboxes and status table (lines ~369-374) show FT-01 as `[x]`/Complete but FT-02 through FT-06 still as `[ ]`/Pending, even though the phase fully implements and tests all six (verified above). Git history shows only one commit (`8f27d683`, from plan 25-02) ever touched `REQUIREMENTS.md`'s checkboxes; no later plan (25-03 through 25-14) flipped FT-02..FT-06. This is a bookkeeping omission, not a functional gap — the underlying capability is verified against the codebase independently of this file. Recommended fix: check FT-02..FT-06 and update the status table to `Complete` before archiving the phase.

### Anti-Patterns Found

None. Scanned every plan-01-through-14 created/heavily-modified core file for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder` — zero hits. No stub `return null`/`Ok(())`/empty-vec patterns found in the reviewed fault-tolerance modules; every "Known Stubs" section across the 14 SUMMARYs self-reports "None," and spot-checked code matches (e.g., `PaladinPort::execute_observed`'s default body is a real, correct delegation to `execute`, not a placeholder).

### Human Verification Required

None. This phase is pure backend engine work (no UI, no ambiguous UX judgment calls). The one `human_judgment: true` item recorded in `25-04-SUMMARY.md` (Redis node-cache Tier-2 live-server contract suite) is an environment-availability constraint explicitly called out by the orchestrator as CI-only evidence, not a gap — Docker/Redis are absent from this devcontainer, the tests self-skip by design (a pre-existing Phase 22 tiering convention), and the `redis-cache-integration` CI job is the intended place this evidence is produced.

### Gaps Summary

No gaps block the phase goal. All five ROADMAP success criteria are independently verified against the codebase (not just SUMMARY narrative) via direct code inspection plus targeted, passing test runs covering the exact behaviors each criterion asserts: exact backoff sequences under a paused clock, transience-gated retry, nested run/idle timeout naming, the real (non-scripted) E2E-3 per-task retry and compensation-chain/loop-bound scenarios, provider fallback with the first-chunk streaming rule, and cache-hit/no-execution/no-cached-failures behavior. `MIGRATION.md` §9.2 and `.cargo/semver-checks-allowlist.toml` are registered and set-equal (verified by direct count, not by trusting the SUMMARY's claimed count). The only non-blocking finding is the REQUIREMENTS.md checkbox lag noted above.

---

*Verified: 2026-09-06T03:38:04Z*
*Verifier: Claude (gsd-verifier)*
