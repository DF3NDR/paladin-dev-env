---
phase: 43-rate-pacing
verified: 2026-10-09T12:00:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
human_verification: []
warnings:
  - "OpenAI header rows in 43-PROVIDER-HEADER-EVIDENCE.md rest on operator attestation (2026-10-09), not a fetched quote; OpenAI doc hosts are still unreachable from the sandbox (HTTP 000 re-confirmed). Anthropic rows were independently re-confirmed by this verifier."
  - "PACE-04 stampede lock is a tested library capability (WarEngine::with_cadence + put_fenced); no shipped composition root attaches it because the server attaches no node cache today. Documented in ADR-0058 and the WarEngine::with_cadence rustdoc."
  - "Documented non-blocking limitations (ADR-0058 A1-A3): during a Redis outage a worker cannot see other workers' 429s, and a pre-outage fleet gate is invisible to the in-process fallback; hand-built FallbackLlmAdapter chains that never call with_cadence and AgentRuntimeDeps (private default in-process wiring unless the caller passes the shared one) are not fleet-paced."
  - ".planning/PROJECT.md Key Decisions has no ADR-0058 row (flagged by 43-13; housekeeping for the phase-transition step)."
---

# Phase 43: Rate Pacing Verification Report

**Phase Goal:** Outbound LLM calls back off on provider 429s instead of thrashing, in-process and across the whole worker fleet, without ever leaving a run unpaced.
**Verified:** 2026-10-09
**Status:** passed
**Re-verification:** No, initial verification
**Repo state:** branch `claude/laughing-dirac-e0h2ax`, HEAD `b5982ef5`, working tree clean at start. Only this file was written.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `LlmError::RateLimitExceeded` carries a retry delay parsed from `Retry-After` (delta-seconds or HTTP-date) and provider rate-limit headers, names verified against official OpenAI and Anthropic docs | VERIFIED | `llm_port.rs:357` variant is `{ retry_after: Option<Duration>, hints: Option<Box<RateLimitHints>> }`, `#[non_exhaustive]`, with `retry_after()` / `rate_limit_hints()` accessors that look through `AllProvidersFailed`. `rate_limit_headers.rs` parses delta-seconds, decimals, all three HTTP-date forms (`httpdate`), `retry-after-ms`, the six OpenAI `x-ratelimit-*` names with Go-style durations, and the twelve `anthropic-ratelimit-*` names with RFC 3339 resets; bounded, panic-free (`try_from_secs_f64`, saturating u128, 128-byte cap, 24 h clamp). Every adapter snapshots headers before the body is consumed: OpenAI `adapter.rs:68`, Anthropic `:77`, DeepSeek `:54`, Gemini `:110`, and `CompatEngine` `:57` (which Qwen, Grok, Kimi, openai-compatible delegate to). `cargo test -p paladin-llm --lib`: 237 passed. Anthropic names: independently re-fetched from `platform.claude.com/docs/en/api/rate-limits` this session; all twelve plus `retry-after` present. OpenAI names: operator-attested (see Warnings). |
| 2 | A mocked 429 against any `FallbackLlmAdapter` hop proves back-off with jitter (retry delay a minimum) rather than immediate-retry thrash | VERIFIED | `CadenceLlmAdapter` (`cadence.rs`) gates before delegating, records the 429 with the provider delay as the minimum, never retries itself. `CadencePolicy::delay_for` (`cadence_port.rs:333`): an explicit delay is returned exactly (clamped to 24 h ceiling), otherwise `max(base, floor(min(max, base*2^(streak-1)) * u))`. Waiter spread is add-only (`waiter_spread`). `FallbackLlmAdapter::with_cadence` (`fallback.rs:264`) wraps every hop and `pace_hop` retries the same hop within `fallback_pace_budget` before hopping. Tests in the 237-test llm run include `paced_chain` cases (pace at 0/30/60 s boundary, 5xx still hops at once, streaming 429 paces, unreadable gate never hot-loops). Production chains: `ModelFallbackMiddleware::paced` (`resilience.rs:148`) and `AgentRuntimeDeps.cadence`. |
| 3 | With Redis configured, a 429 on one worker measurably slows every worker sharing that provider/model's state, through an atomic Lua script using the Redis server clock | VERIFIED | `redis.rs:155-275`: `CADENCE_RECORD_429_LUA`, `CADENCE_GATE_LUA`, `CADENCE_RECORD_SUCCESS_LUA` each call `redis.call('TIME')`; state is relative-wait from the server clock; the in-flight rule keeps a stale 429 from escalating the streak. Test `scripts_read_the_server_clock_and_format_integers` ran. Live run against `redis-server` on port 6380 (shut down afterwards): `cargo test -p paladin-ai --features redis-cadence --lib cadence` 21 passed including `cadence_fleet_429_on_one_worker_slows_the_other` (two `RedisCadence` workers, A's 1 s 429 holds B's same-model call for at least 900 ms, a different model is not delayed) and `build_cadence_redis_backend_shares_state_between_two_workers`; `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- cadence node_cache`: 128 passed, no `SKIP:` line. |
| 4 | A distributed stampede lock (set-if-absent with expiry, fencing token, delete-only-if-owner) stops concurrent workers issuing the same cached request twice | VERIFIED (library capability) | `CADENCE_TRY_LOCK_LUA` (EXISTS check, `INCR` fence counter, `SET ... PX` in one script) and `CADENCE_UNLOCK_LUA` (`GET == token` then `DEL`). Engine integration in `superstep.rs:287` (`try_lock`), `:225` (`unlock` on every exit), `:466` (`put_fenced`). Live tests passed: `redis_cadence_passes_the_lock_contract`, `a_lock_expires_in_redis_and_the_stale_holder_cannot_release_the_new_one`, `redis_put_fenced_keeps_the_highest_token_under_concurrent_writers`, `redis_put_fenced_ignores_a_lower_distributed_token`. Engine: `cargo test -p paladin-battalion --lib stampede_lock_tests` 8 passed, including `concurrent_identical_cache_misses_execute_the_node_once`, `failed_holder_hands_the_lock_over_within_one_poll`, `unlock_runs_on_every_exit`. The lock fails open (`ResilientCadence`) so it can never fail a run. Warning: not attached by any shipped composition root (server has no node cache). |
| 5 | When Redis is unavailable, pacing degrades to conservative per-process pacing with a trace warning; a run is never left unpaced, covered by a test | VERIFIED | `ResilientCadence` (`resilient.rs`): first primary error latches, the same call is served by `InMemoryCadence::with_multiplier(2.0)`, one `warn` per outage under `paladin::cadence` plus one recovery `info`, probe at most once per 5 s, max rule on recovery. `build_cadence` builds Redis without connecting, so boot succeeds with Redis down. Tests: `cadence_with_redis_down_still_paces` (dead backend, call 2 held at least 2x base, outcome unchanged), `unreachable_redis_degrades_within_the_timeout_budget`, `one_warning_per_outage_and_one_recovery_line`, `concurrent_failures_in_one_outage_warn_once`, plus both contract suites run degraded. "Trace warning" is a `log::warn!`, not a `TraceEvent` variant: recorded as a deliberate resolution (ADR-0058, Open Question 5). |

**Score:** 5/5 roadmap truths verified; 0 present-but-behavior-unverified. Behavior-dependent truths (cancellation, unlock-on-every-exit, latch and recovery, fence ordering) each have a passing named test.

### Required Artifacts

`gsd-tools query verify.artifacts` on all 13 plans: every declared artifact exists and passes (13/13 plans `all_passed: true`). Substance checked by reading: `rate_limit_headers.rs` (1160 lines), `cadence.rs` (1158), `cadence_port.rs` (680), `in_memory.rs` (875), `redis.rs` (1650), `resilient.rs` (1281), `src/infrastructure/cadence.rs` (785), `contract_tests.rs` (865). No stubs.

### Key Link Verification

`verify.key-links` returned `verified: false` for 8 plans only because the plan patterns are double-escaped (`Invalid regex pattern: compose_llm\\(`); that is a tooling artifact, so the links were checked by hand:

| From | To | Via | Status |
|------|----|-----|--------|
| `agent_host.rs:216`, `facade_provisioner.rs:354` | `infrastructure/cadence.rs::compose_llm` | `with_pricing(with_cadence(llm))` | WIRED |
| openai/anthropic/deepseek/gemini adapters, `compat/engine.rs` | `rate_limit_headers::hints_from_headers` | headers snapshotted before `.text()` | WIRED |
| `CadenceLlmAdapter` | `CadencePort::record_rate_limited` | `err.retry_after()` and `rate_limit_hints()` as the minimum (`cadence.rs:277,292`) | WIRED |
| `fallback.rs:264`, `resilience.rs:148` | `cadence::with_cadence` | every hop wrapped under its own provider name | WIRED |
| `paladin-server.rs:122-237` | `build_agent_registry_with_cadence`, `FacadeProvisioner::with_cadence`, `build_run_api_with_cadence` | one shared `CadenceWiring` clone | WIRED |
| `superstep.rs:287/225/466` | `CadencePort::try_lock/unlock`, `NodeCachePort::put_fenced` | cache-miss window | WIRED |
| `build_cadence` (redis backend) | `ResilientCadence(RedisCadence, InMemoryCadence x multiplier)` | `src/infrastructure/cadence.rs:111` | WIRED |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| `RateLimitExceeded.retry_after` | header delay | real response headers via `hints_from_headers`, asserted in adapter tests (mockito) | yes | FLOWING |
| `CadenceLlmAdapter` gate | `GateReading` | shared `CadencePort` (in-memory table or Redis `TIME`-based script) | yes | FLOWING |
| Fleet gate | `nb`, `streak` hash | Lua on the live server, exercised by two-worker test | yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Header parsing, decorator, fallback pacing, conformance | `cargo test -p paladin-llm --lib` | 237 passed | PASS |
| In-memory, resilient, contract suites | `cargo test -p paladin-storage --lib cadence` | 66 passed | PASS |
| Port policy math | `cargo test -p paladin-ports --lib cadence` | 10 passed | PASS |
| Engine stampede lock | `cargo test -p paladin-battalion --lib stampede_lock_tests` | 8 passed | PASS |
| Live Redis storage suites | `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- cadence node_cache` with `CADENCE_REDIS_TEST_URL` and `NODE_CACHE_REDIS_TEST_URL` set | 128 passed, no SKIP | PASS |
| Root composition and fleet | `cargo test -p paladin-ai --features redis-cadence --lib cadence` (live Redis) | 21 passed | PASS |
| Provider doc cross-check (Anthropic) | `curl` of `platform.claude.com/docs/en/api/rate-limits` | all twelve `anthropic-ratelimit-*` names present | PASS |

### Probe Execution

Step 7c: SKIPPED. No probe scripts are declared in any PLAN or SUMMARY, and no `scripts/*/tests/probe-*.sh` is tied to this phase.

### Requirements Coverage

Plan frontmatter requirement IDs: 01 [PACE-02]; 02 [PACE-01, PACE-02]; 03 [PACE-01, PACE-02]; 04 [PACE-01, PACE-02]; 05 [PACE-02]; 06 [PACE-02]; 07 [PACE-03]; 08 [PACE-05]; 09 [PACE-02, PACE-03, PACE-05]; 10 [PACE-04]; 11 [PACE-04]; 12 [PACE-01]; 13 [PACE-01..05]. All five IDs are claimed; no orphaned Phase 43 requirement in REQUIREMENTS.md.

| Requirement | Source plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| PACE-01 | 02, 03, 04, 12, 13 | 429 carries retry delay from `Retry-After` and provider headers, names verified | SATISFIED | Truth 1; OpenAI half by operator attestation |
| PACE-02 | 01-06, 09, 13 | `LlmPort` pacing decorator, back-off with jitter, wraps every fallback hop | SATISFIED | Truth 2 |
| PACE-03 | 07, 09, 13 | Redis-shared pacing via atomic Lua on server clock | SATISFIED | Truth 3 |
| PACE-04 | 10, 11, 13 | Stampede lock: set-if-absent+expiry, fencing token, delete-only-if-owner | SATISFIED | Truth 4 |
| PACE-05 | 08, 09, 13 | Degrade to conservative per-process pacing with warning, never unpaced, tested | SATISFIED | Truth 5 |

REQUIREMENTS.md already shows all five `[x]` and "Complete" in the traceability table; the code backs that up.

### Anti-Patterns Found

Debt-marker scan over the added lines of the phase diff (`bc57061b..HEAD`, `*.rs/*.toml/*.sh/*.yml`): no `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, `todo!` or `unimplemented!`. Production sections of the new modules (before their test modules) contain no `unwrap()`, `expect()` or `panic!`. Security review items spot-checked: no Debug/log path renders the Redis URL (hand-written redacted `Debug`, `url_env` stores only the variable name), no new HTTP client or redirect policy, header parser stores no raw value. None found.

### Human Verification Required

None open. The one human-dependent fact (OpenAI header names and the `insufficient_quota` code) was already confirmed by the operator on 2026-10-09 and is recorded as such in the evidence file; it is disclosed under Warnings, not left as a pending item.

### Gaps Summary

No gaps. All five roadmap success criteria are backed by code that exists, is wired into the server and agent composition roots, and is covered by passing named tests, including live-Redis runs this session.

Non-blocking items (see frontmatter `warnings`):

1. OpenAI header evidence is attestation, not a quote. If a future operator can reach `platform.openai.com`, a verbatim quote would harden it.
2. The stampede lock is wired as a library builder only; adding it to the server depends on the server first attaching a node cache (outside this phase's scope per CONTEXT).
3. Known design limits: outage blind spot (A1-A3), hand-built fallback chains that skip `with_cadence`, and `AgentRuntimeDeps` defaulting to a private in-process wiring.
4. Environment-skipped items, all recorded by 43-13: `cargo-llvm-cov` (82 % floor, gated in CI), `actionlint`, and the two pre-existing `build_run_api_persists_*` failures at the pre-phase commit.
5. `.planning/phases/43-rate-pacing/43-13-SUMMARY.md` flags a Phase 42 item, `RunOutcome::Halted.cause`, which fires `enum_struct_variant_field_added` against 0.10.1; it is outside Phase 43.
6. `.planning/PROJECT.md` lacks an ADR-0058 Key Decisions row (PROMOTION.md has it).

---

_Verified: 2026-10-09_
_Verifier: Claude (gsd-verifier)_
