---
phase: 43-rate-pacing
plan: 06
subsystem: llm-pacing
tags: [cadence, rate-limit, 429, fallback-chain, pace-budget, backoff, jitter, model-fallback-middleware, agent-runtime]

requires:
  - phase: 43-rate-pacing
    provides: CadenceWiring, CadenceLlmAdapter and with_cadence (43-01), typed retry_after minimum and max_wait cap (43-02), waiter spread and InMemoryCadence capacity (43-05), first-429 surfacing in every adapter (43-04)
provides:
  - FallbackLlmAdapter::with_cadence, which wraps every hop in the Cadence decorator and paces a 429 on the same hop before the chain hops (D-03)
  - ModelFallbackMiddleware::paced
  - AgentRuntimeDeps.cadence (in-process by default) and a paced build_chain
  - the ROADMAP success-criterion-2 tests (explicit delay honoured as a minimum, back-off with jitter, pace-budget boundary)
  - the surface registered: MIGRATION 9.1/9.2/9.5, CHANGELOG, API baseline
affects: [43-07, 43-08, 43-09, 43-13]

tech-stack:
  added: []
  patterns:
    - "Pace first, hop last: only a 429 on a chain with pacing attached re-enters the same hop, bounded by a per-hop per-call budget; every other error keeps the hop rule"
    - "A gate that cannot be paced against (unreadable, clear, or beyond max_wait) is not paced: the 429 reaches the hop rule, so the chain can never spin"
    - "Test-only thread-local seam pins the waiter spread so a paused-clock test can assert exact send times"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/src/cadence.rs
    - src/application/services/paladin/middleware/resilience.rs
    - src/config/agent_runtime.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The pace budget is measured in wall-clock time from the hop's first 429 of the call (elapsed + next gate <= budget paces; equal still paces), not as a sum of gates, so provider latency counts against it"
  - "A 429 is paced only when the hop's gate reads a non-zero wait no longer than max_wait; otherwise it falls to the hop rule (deviation from 'a port error counts as a zero wait', see below)"
  - "with_cadence on a chain that already has pacing is a no-op, since wrapping the hops twice would gate them twice"
  - "Config-built chains are paced by default: AgentRuntimeDeps::default builds an in-process wiring from CadenceConfig::default() without calling crate::infrastructure; None opts out"

patterns-established:
  - "Pacing state lives behind the shared CadencePort; the chain holds no cross-call state of its own"

requirements-completed: [PACE-02]

duration: about 1 h 40 min
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 06: Fallback chains pace first and hop last Summary

**A 429 on any fallback hop now waits out that provider's gate and retries the same provider until the pace budget is spent, and only then hops; every hop is wrapped in the Cadence decorator, and chains built from configuration are paced by default.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** 72465fd0, 7479a29e
- **Files:** 7 modified, none created

## Accomplishments

- `FallbackLlmAdapter::with_cadence` wraps every hop with the Cadence decorator (each hop is gated under its own provider name, never under `"fallback"`) and keeps the shared port, the pace budget and `max_wait` in a private `FallbackPacing`.
- The per-hop retry loop in both `generate` and `generate_stream`: on a `RateLimitExceeded` it reads the hop's gate and, while `elapsed since the hop's first 429 + gate <= budget`, records the attempt, logs a `paladin::cadence` debug line (provider, model, durations) and re-enters the same hop. `TraceEvent::FallbackHop` fires only at a real hop. `AllProvidersFailed.attempts` lists every attempt, paced ones included, in order.
- `ModelFallbackMiddleware::paced`, `AgentRuntimeDeps.cadence` (in-process by default, `None` opts out), and a private `fallback_middleware` helper that `build_chain` calls.
- Module docs of `fallback.rs` rewritten: the one 429 exception to the hop rule with the operator rationale, the composition `Pricing(Fallback(Cadence(hop1), Cadence(hop2)))`, and "no cross-call state of its own".

## Success criterion 2, as executable tests

All in `crates/paladin-llm/src/fallback.rs`, on a paused clock over `InMemoryCadence`:

- `mocked_429_on_a_fallback_hop_backs_off_and_stays_on_the_provider`: `[rate_limited(Some(7s)), Text]` is served by the primary, primary called twice, backup never, gap at least 7 s.
- `delayless_429s_on_a_hop_back_off_with_jitter`: with jitter pinned to 0.999 and no spread, gaps are 500 ms, 999 ms, 1998 ms; with real jitter and spread, 20 repetitions stay within `[base, ceiling + spread]` and the third gaps are not all identical.
- `pace_budget_boundary_paces_at_the_budget_and_hops_beyond_it`: 30 s delays, 60 s budget: primary sends at exactly 0, 30 and 60 s, then the backup serves, one `FallbackHop`.
- Also: `a_5xx_still_hops_immediately_with_cadence_attached`, `streaming_429_on_a_hop_paces_before_hopping`, `all_providers_failed_lists_paced_attempts`, `chain_without_cadence_still_hops_on_a_429`, `with_cadence_wraps_every_hop_under_its_own_provider_name`, plus two guards added by this plan: `a_broken_gate_hops_instead_of_retrying_hot` and `a_gate_beyond_max_wait_hops_instead_of_spinning`.
- Middleware and configuration: `paced_middleware_serves_from_the_primary_after_its_gate`, `paced_rejects_an_empty_chain_like_new`, `fallback_middleware_installs_the_paced_chain_when_cadence_is_present`, `fallback_middleware_without_cadence_hops_on_a_429`, `default_deps_carry_an_in_process_cadence`, `build_chain_with_model_fallback_builds_paced_and_unpaced`.

## Red / green record

- **Task 1:** tests written first; `cargo test -p paladin-llm --lib fallback` failed to compile with a single error (`no method named with_cadence`), then went green: 26 passed before, 36 after (10 more). Mutation check: changing the budget comparison from `>` to `>=` failed `pace_budget_boundary_paces_at_the_budget_and_hops_beyond_it` and `all_providers_failed_lists_paced_attempts`; reverted.
- **Task 2:** mutation check: making `fallback_middleware` ignore the wiring failed `fallback_middleware_installs_the_paced_chain_when_cadence_is_present`; reverted. Strict red-first was not shown separately for Task 2 (tests and implementation were written together).

## Task Commits

1. **Task 1: fallback chains pace a 429 on the same hop before hopping** - `72465fd0`
2. **Task 2: pace middleware- and config-built fallback chains by default, register the surface** - `7479a29e`

## Verification

- `cargo test -p paladin-llm --lib fallback`: 36 passed (stable over repeated runs); `cargo test -p paladin-llm --all-features`: 637 passed; `--doc fallback`: 2 passed; `cargo check -p paladin-llm --no-default-features` clean.
- `cargo test -p paladin-ai --lib -- --skip build_run_api_persists`: 1272 passed; `config::agent_runtime` 25 passed over 8 consecutive runs; `--doc ModelFallbackMiddleware`: 2 passed; `cargo build --example agent_runtime_middleware` exits 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm -p paladin-ai --no-deps --all-features` clean.
- `./scripts/check-migration-allowlist.sh` exits 0 (all three new 9.2 rows are `N`); `make check-changelogs` passes.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` failed while stale with exactly two changed items (`ModelFallbackMiddleware::paced`, `AgentRuntimeDeps::cadence`), then after `make api-surface-update` exits 0 ("API surface unchanged"); `.project/current-exports.txt` contains `paced`.
- Prohibition PACE-02 (no provider switch on a 429 before the budget is spent) is proven by the three criterion tests above: the primary is retried at 0/30/60 s and the backup is untouched until the budget would be exceeded.
- Manual credential-handling review: the new debug line names provider, model and durations only; no key, URL, header value or request body is interpolated.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A gate that is unreadable, clear or beyond `max_wait` is not paced (plan said a port error counts as a zero wait)**
- **Found during:** Task 1
- **Issue:** treating a port error as a zero wait would retry the same hop immediately until the wall-clock budget ran out, which is the immediate-retry thrash this phase removes. Separately, when a gate exceeds `max_wait` the decorator refuses the call without sleeping, so a budget above `max_wait` would make the chain spin without time passing.
- **Fix:** `pace_hop` only paces when the gate reads a non-zero wait of at most `max_wait` (stored in the private `FallbackPacing` beside port and budget); otherwise the 429 reaches the hop rule. Both cases have tests.
- **Files modified:** `crates/paladin-llm/src/fallback.rs`
- **Commit:** 72465fd0

**2. [Rule 3 - Blocking] Test-only seam in `cadence.rs` to pin the waiter spread**
- **Found during:** Task 1
- **Issue:** 43-05's waiter spread adds `u * min(wait / 10, 1 s)` to every sleep, so the plan's exact sends at 0, 30 and 60 s (the boundary where elapsed plus gate equals the budget) cannot be asserted with a random spread.
- **Fix:** `#[cfg(test)]` thread-local `pin_waiter_spread` (guard restores the random draw) and a `spread_fraction()` helper; production behaviour is unchanged (`rand::random`). `cadence.rs` was not in the plan's `files_modified`.
- **Files modified:** `crates/paladin-llm/src/cadence.rs`
- **Commit:** 72465fd0

**3. [Rule 1 - Bug] Pre-existing env-test race: `build_chain_uses_the_documented_fixed_order` was not `#[serial]`**
- **Found during:** Task 2 (the new serial test failed intermittently with "OPENAI_API_KEY environment variable not set")
- **Issue:** the plan calls it an existing serial env test, but it sets and removes `OPENAI_API_KEY` without `#[serial]`, racing the other env tests in the module.
- **Fix:** added `#[serial]` with a comment. Stable over 8 consecutive runs afterwards.
- **Files modified:** `src/config/agent_runtime.rs`
- **Commit:** 7479a29e

**4. [Plan acceptance wording] `FallbackLlmAdapter::with_cadence` second call is a no-op**
- Not in the plan: a second call on an already-paced chain would wrap hops twice and double-gate them, so it returns the chain unchanged (documented on the method).

**Total deviations:** 4 (2 Rule 1, 1 Rule 3, 1 addition). **Impact:** none on the plan's behaviour truths; two extra tests.

## Intentional scope notes

- No server composition root builds a fallback chain today (`agent_runtime.model_fallback` is consumed only by `AgentRuntimeConfig::build_chain`, a library API), as the plan recorded; the server's own ports stay covered by 43-01/43-09's `compose_llm`.
- A chain a library user builds with `FallbackLlmAdapter::new` and never calls `with_cadence` on keeps hopping immediately on a 429 by design.
- `treasurer.cadence.fallback_pace_budget_secs` is now consumed (it was reserved in 43-01); `lock_ttl_secs` remains reserved for the cache-stampede plans. The environment-override keys for these arrive in 43-09.
- The paladin-llm crate CHANGELOG entry for this plan is left to 43-13, which owns the crate CHANGELOG entries.

## Issues Encountered

- Free disk stayed near 7 GB; no ENOSPC. The workspace test run was done per crate (`paladin-llm` all-features, `paladin-ai` lib) rather than as one `cargo test --workspace`, with `--skip build_run_api_persists` as instructed.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. T-43-20 (a chain pinned by endless 429s): the cumulative per-hop budget plus the boundary test; T-43-21 (invisible paced retries): attempts listed in `AllProvidersFailed.attempts` and a debug line under `paladin::cadence`; T-43-22 (stacked retries): the decorator still never retries, only the chain re-enters a hop, and only for a 429 with pacing attached.

## Next Phase Readiness

43-07 and 43-08 add the shared Redis backend and the resilient wrapper behind the same `CadencePort`; the chain reads gates through the port, so it needs no change. 43-09 threads one shared wiring through the server and can pass it as `AgentRuntimeDeps.cadence`.

## Self-Check: PASSED

Modified files exist and commits 72465fd0 and 7479a29e are present in `git log` on `claude/laughing-dirac-e0h2ax`.
