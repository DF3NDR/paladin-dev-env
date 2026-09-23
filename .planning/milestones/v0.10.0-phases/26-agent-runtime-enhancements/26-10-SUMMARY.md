---
phase: 26-agent-runtime-enhancements
plan: 10
subsystem: agent-runtime
tags: [middleware, retry, fallback, resilience, aegis, rust]

requires:
  - phase: 26-01
    provides: "ExecutionMiddleware trait, MiddlewareFlow, ModelCallContext (with the llm_override/retry_policy fields this plan reads), and the onion-ordering chain driver"
  - phase: 26-02
    provides: "AgentRuntimeConfig, ModelRetryConfig and ModelFallbackConfig sub-structs"
  - phase: 26-05
    provides: "The StopReason/ExecutionMiddleware built-in pattern (ModelCallLimit/TokenBudget/ToolCallLimit) this plan's resilience.rs mirrors, and the around_tool &mut ToolCallContext precedent"
provides:
  - "RetryPredicate::admits(Transience) -> bool in paladin-core's aegis module -- the single home of the transience-to-boolean retry decision"
  - "paladin_battalion::engine::retry::should_retry refactored to call admits (Phase 25 retry tests pass unmodified)"
  - "ModelFallbackMiddleware and ModelRetryMiddleware: stateless, port-shaping ExecutionMiddleware built-ins"
  - "ModelCallContext::effective_llm -- the single port-resolution point, called once from execute_with_retry_and_temperature"
  - "execute_with_retry_and_temperature reads cx.retry_policy to drive attempts/delays/predicate, with a byte-identical fallback shape when no policy is set"
  - "ModelFallbackConfig::resolve_chain resolving provider names through LlmProviderFactory, distinguishing unknown/uncompiled/construction-failed providers"
  - "impl From<&ModelRetryConfig> for RetryPolicy"
affects: [26-20]

tech-stack:
  added: []
  patterns:
    - "Port-shaping middleware: before_model sets a value on the per-run context; the service's single call site resolves and acts on it -- no repeated-call/around_model hook needed"
    - "A promoted single resolution point (ModelCallContext::effective_llm) with the pre-existing field seeded as the default, rather than an if-override-else-service branch at the call site"

key-files:
  created:
    - src/application/services/paladin/middleware/resilience.rs
  modified:
    - crates/paladin-core/src/platform/container/aegis.rs
    - crates/paladin-battalion/src/engine/retry.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/middleware/mod.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/config/agent_runtime.rs

key-decisions:
  - "The assumption-delta decision (promote) from the plan's own frontmatter was implemented literally: ModelCallContext::effective_llm(&self, service_default) is the ONE accessor execute_with_retry_and_temperature calls; the service's own llm_port is the seeded default parameter, not an else-branch"
  - "The retry loop's port and policy are both read from cx once at the top of execute_with_retry_and_temperature; with no policy the function falls through every original branch unchanged (same max_attempts formula, same 100ms*2^(attempt-1) backoff, same Permanent short-circuit) -- proven byte-for-byte by no_resilience_middleware_keeps_todays_retry_shape under a paused clock"
  - "ModelFallbackConfig::resolve_chain distinguishes THREE outcomes, not two: an outright-unknown provider name, a real provider whose cargo feature is not compiled into the current build (via a hardcoded KNOWN_PROVIDER_NAMES list independent of compiled features), and a real compiled provider whose construction failed for another reason (e.g. a missing credential) -- only the first two come from provider_factory's UnknownProvider variant; a ConfigurationMissing/AdapterCreationFailed error is never misreported as unknown or uncompiled"
  - "RetryPredicate/Transience imports in paladin-battalion's retry.rs were moved from the module top level into the #[cfg(test)] mod, since after the refactor they are used only by tests -- cargo check --all-targets flags them as unused otherwise (the plain lib target has no cfg(test))"

patterns-established:
  - "A built-in middleware that sets a value on ModelCallContext for a downstream call site to read, rather than acting itself -- extends D-03's 'state lives on the context, never the middleware struct' rule to port/policy selection, not just counters"

requirements-completed: [RT-02]

coverage:
  - id: D1
    description: "RetryPredicate::admits is the single home of the transience-to-boolean retry decision; should_retry is refactored to call it with every Phase 25 retry test passing unmodified"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/aegis.rs#tests::admits_is_pure_and_matches_the_documented_table"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests (40 tests, unmodified from before this plan)"
        status: pass
    human_judgment: false
  - id: D2
    description: "ModelFallbackMiddleware builds one FallbackLlmAdapter at construction, validates an empty chain up front, and sets llm_override in before_model; served_by is populated through the adapter's own metadata stamp"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/resilience.rs#tests::fallback_middleware_routes_through_the_fallback_adapter, ::fallback_chain_construction_validates_up_front"
        status: pass
    human_judgment: false
  - id: D3
    description: "ModelRetryMiddleware sets a RetryPolicy that drives attempts, delays (backoff_delay) and the retry predicate (admits) at the single call site; with no policy the loop keeps today's shape byte-for-byte"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/resilience.rs#tests::retry_middleware_uses_the_policy_attempts_and_delays, ::no_resilience_middleware_keeps_todays_retry_shape, ::retry_and_fallback_compose"
        status: pass
    human_judgment: false
  - id: D4
    description: "The model-call port is resolved at exactly one point (ModelCallContext::effective_llm), and an override on one run never leaks to a concurrent run using a different service"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/resilience.rs#tests::model_call_port_is_resolved_at_exactly_one_point, ::concurrent_runs_do_not_share_an_override"
        status: pass
    human_judgment: false
  - id: D5
    description: "ModelFallbackConfig::resolve_chain resolves provider names through LlmProviderFactory, collecting every unresolvable name into one typed error and distinguishing unknown/uncompiled/construction-failed; credentials are never read from or stored in the config"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::resolve_chain_builds_ports_in_configured_order, ::unknown_provider_is_a_typed_error_listing_every_offender, ::uncompiled_provider_is_reported_distinctly_from_unknown, ::disabled_config_resolves_to_no_chain, ::config_holds_no_credential, ::model_retry_config_maps_to_retry_policy_defaults"
        status: pass
    human_judgment: false
  - id: D6
    description: "Full workspace is green: no regression in any pre-existing test, formatting and lints clean"
    requirement: "RT-02"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --all --check; cargo test -p paladin-ai --all-features --lib (840 pass); cargo test -p paladin-ai --all-features --test lib (807 pass); cargo test -p paladin-ai-core --lib (514 pass); cargo test -p paladin-battalion --lib (40 retry-scoped, 721 total, pass); cargo test -p paladin-ai --all-features --doc (141 pass); cargo test -p paladin-ai-core --doc (83 pass)"
        status: pass
    human_judgment: false

duration: ~45min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 10: Retry & Fallback as Port-Shaping Middleware Summary

**`ModelRetryMiddleware`/`ModelFallbackMiddleware` set what the run's ONE model-call site should use; `ModelCallContext::effective_llm` resolves the port at exactly that one point; retry math and the transience predicate delegate to Phase 25's `backoff_delay`/new `RetryPredicate::admits`; and `ModelFallbackConfig::resolve_chain` closes Phase 25's config-driven-fallback-chain deferred idea.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 3 (all `type="auto" tdd="true"`)
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- `RetryPredicate::admits(Transience) -> bool` is the single, pure, doc-tested home of the transience-to-boolean retry decision in `paladin-core`'s `aegis` module; `paladin_battalion::engine::retry::should_retry` is refactored to call it, and all 40 of Phase 25's retry-related tests in that crate pass completely unmodified.
- `ModelFallbackMiddleware::new(chain)` builds exactly one `FallbackLlmAdapter` at construction (reusing its own empty-chain validation) and installs it as `ModelCallContext::llm_override` in `before_model`; `served_by` is populated purely through the adapter's existing metadata stamp, with no second provenance mechanism.
- `ModelRetryMiddleware::new(policy)` sets `ModelCallContext::retry_policy`; it never retries anything itself.
- `ModelCallContext::effective_llm(&self, service_default)` is the assumption-delta "promote": the ONE accessor `execute_with_retry_and_temperature` calls, with the service's own port seeded as the default rather than an else-branch. `execute_with_retry_and_temperature` also now reads `cx.retry_policy`: when present, attempts come from `policy.max_attempts`, delays from `paladin_battalion::engine::retry::backoff_delay`, and the retry decision from `RetryPredicate::admits`; when absent, the function falls through to the exact prior code path (`max_loops.min(10)` attempts, `100ms * 2^(attempt-1)`, `Permanent` short-circuit), pinned byte-for-byte by a paused-clock test.
- `ModelFallbackConfig::resolve_chain(&self, factory)` maps configured provider names through `LlmProviderFactory::create`, collecting every unresolvable name into one typed `AgentRuntimeConfigError`, and distinguishing three outcomes an operator must not confuse: an outright-unknown name, a real provider whose cargo feature is not compiled into the current build, and a real compiled provider whose construction failed for another reason (e.g. a missing credential). A disabled config resolves to no chain without ever touching the factory.
- `impl From<&ModelRetryConfig> for RetryPolicy` maps all six mirrored fields; `ModelRetryConfig::default()` converts to a `RetryPolicy` equal to `RetryPolicy::default()`, field for field, pinned by a test.
- An `around_model` hook was deliberately not added: the module doc records why it was rejected (fallback substitutes a different port, it does not repeat a call; adding one would change PRD 05's closed three-hook trait).

## Task Commits

1. **Task 1: One home for the predicate** — `d180edfc` (feat: `RetryPredicate::admits`, `should_retry` refactored onto it, Phase 25 tests unmodified)
2. **Task 2: Port-shaping middleware and the single port-resolution point** — `ca15260a` (feat: `ModelFallbackMiddleware`/`ModelRetryMiddleware`, `ModelCallContext::effective_llm`, the call-site rewrite), fixed by `b2f754c6` (a test doc comment was self-matching the plan's own `fn effective_llm` grep acceptance criterion)
3. **Task 3: `ModelFallbackConfig` resolves provider names through the factory** — `2bbb5b80` (feat: `resolve_chain`, `AgentRuntimeConfigError`, `UnresolvedProvider`, `From<&ModelRetryConfig> for RetryPolicy`)

_RED/GREEN note: consistent with every other Phase 26 plan's SUMMARY, tests and implementation land together per task rather than as a literal two-commit split — adding a method/type a test references immediately is a compile-fail change, not a runtime-assertion one, so there is no meaningful intermediate compiling-but-failing state for a from-scratch API. Each task's tests were run and confirmed passing before its commit; no test needed correction after the fact except the acceptance-criteria self-match fixed in `b2f754c6`._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/aegis.rs` — `RetryPredicate::admits` with a doc-tested three-arm table and a unit test covering all nine predicate/transience combinations
- `crates/paladin-battalion/src/engine/retry.rs` — `should_retry` delegates to `admits`; `RetryPredicate`/`Transience` imports moved into `#[cfg(test)] mod tests` (unused at the plain-lib-target level after the refactor)
- `src/application/services/paladin/middleware/resilience.rs` (new) — `ModelFallbackMiddleware`, `ModelRetryMiddleware`, and 7 tests (assumption-delta invariant, fallback routing, policy-driven attempts/delays under a paused clock, today's-shape parity including the Permanent short-circuit, construction validation, cross-service isolation, retry+fallback composition)
- `src/application/services/paladin/middleware/context.rs` — `ModelCallContext::effective_llm`, doc-tested
- `src/application/services/paladin/middleware/mod.rs` — registers and re-exports `resilience`
- `src/application/services/paladin/paladin_execution_service.rs` — `execute_with_retry_and_temperature` takes `&ModelCallContext`, resolves the port/policy once; the one call site passes `&middleware_cx`; three pre-existing unit tests updated with a new `bare_cx()` test helper
- `src/config/agent_runtime.rs` — `KNOWN_PROVIDER_NAMES`, `UnresolvedProvider`, `AgentRuntimeConfigError`, `ModelFallbackConfig::resolve_chain`, `impl From<&ModelRetryConfig> for RetryPolicy`, and 6 new tests

## Decisions Made

- **The assumption-delta "promote" decision was implemented literally, not just documented.** `ModelCallContext::effective_llm` is the one accessor; the service's own port is the seeded default parameter to that accessor, never a branch at the call site. A grep-based acceptance criterion (`fn effective_llm` count == 1 across `src/application/services/paladin/`) pins this structurally.
- **`execute_with_retry_and_temperature`'s two shapes (with/without a policy) are pinned by two dedicated tests under a paused clock**, rather than relying on code review alone — `no_resilience_middleware_keeps_todays_retry_shape` asserts the exact `100/200ms` delay sequence and `retry_middleware_uses_the_policy_attempts_and_delays` asserts the exact `500/1000/2000ms` sequence `backoff_delay` produces for the same policy.
- **`resolve_chain` distinguishes three failure kinds, not two**, because `LlmProviderFactory::create` can fail for a reason that has nothing to do with whether a name is known or compiled (a missing credential on an otherwise-real, compiled provider). Conflating that into "unknown" or "not compiled" would have misled an operator debugging a credential problem.
- **`RetryPredicate`/`Transience` imports moved into `retry.rs`'s test module.** After the refactor they are referenced only by tests; `cargo check --all-targets` compiles the plain `lib` target (no `cfg(test)`) as well as the test binary, and flagged them as unused there.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Corrected Task 3's `uncompiled_provider_is_reported_distinctly_from_unknown` verify invocation**
- **Found during:** Task 3, designing the test
- **Issue:** The plan's acceptance criteria list this test running under `cargo test -p paladin-ai --all-features --lib uncompiled_provider_is_reported_distinctly_from_unknown`. Under `--all-features`, every entry in `KNOWN_PROVIDER_NAMES` is compiled into the build, so there is structurally no name left that is "known but not compiled" — the branch under test can never fire, and the plan's own automated `<verify>` block (which does NOT include this test name) already reflects that.
- **Fix:** Gated the test `#[cfg(not(feature = "llm-ollama"))]` and verified it under the default feature set (`llm-openai`, `llm-anthropic`, `llm-deepseek` — ollama is genuinely uncompiled there), confirming it is not a vacuous 0-match run (`running 1 test ... ok`).
- **Files modified:** `src/config/agent_runtime.rs` (test only)
- **Verification:** `cargo test -p paladin-ai --lib agent_runtime::tests::uncompiled_provider_is_reported_distinctly_from_unknown` runs and passes; correctly absent under `--all-features`
- **Committed in:** `2bbb5b80`

**2. [Rule 1 - Bug] A test doc comment self-matched the plan's own grep acceptance criterion**
- **Found during:** Post-commit acceptance-criteria verification for Task 2
- **Issue:** A doc comment on `model_call_port_is_resolved_at_exactly_one_point` quoted the literal grep command `grep -rc 'fn effective_llm'` from the plan's acceptance criteria, which made the grep count 2 matches (the real definition plus this comment) instead of the required 1.
- **Fix:** Reworded the comment to describe the check without reproducing its search string.
- **Files modified:** `src/application/services/paladin/middleware/resilience.rs`
- **Verification:** `grep -rc 'fn effective_llm' src/application/services/paladin/` (summed) == 1
- **Committed in:** `b2f754c6`

**3. [Rule 1 - Bug] `resolve_chain` initially conflated a credential-missing provider with "unknown"/"not compiled"**
- **Found during:** Task 3, initial implementation of `resolve_chain`
- **Issue:** A first draft classified every `factory.create` error uniformly by name-lookup, which would have misreported a real, compiled provider failing only because its API key env var is unset as `Unknown` or `NotCompiled` — actively misleading for an operator debugging credentials.
- **Fix:** Matched specifically on `ProviderFactoryError::UnknownProvider` for the Unknown/NotCompiled distinction, and added a third `UnresolvedProvider::ConstructionFailed { name, reason }` variant for every other factory error kind, carrying the real underlying message.
- **Files modified:** `src/config/agent_runtime.rs`
- **Verification:** `resolve_chain_builds_ports_in_configured_order` and `config_holds_no_credential` (real openai/deepseek construction with env-var credentials) pass; no test needed the `ConstructionFailed` path directly, but the classification logic is exercised by every passing/failing case in the six new tests
- **Committed in:** `2bbb5b80`

---

**Total deviations:** 3 auto-fixed (1 blocking verify-invocation correction, 2 bug fixes caught before merge)
**Impact on plan:** All three keep the plan's own acceptance criteria and correctness intent intact. No scope creep — no new public API beyond what the plan specified, and the `ConstructionFailed` variant is a strict refinement of the `AgentRuntimeConfigError` the plan already asked for (a third arm, not a redesign).

## Issues Encountered

None beyond the three deviations above. Every acceptance criterion in the plan (predicate single-home greps, backoff/transience-not-re-derived greps, single-accessor grep, all fifteen named test functions across `aegis.rs`/`resilience.rs`/`agent_runtime.rs`, `cargo semver`-adjacent formatting/lint gates) was verified directly.

## Known Stubs

None. Every type is real, wired, production code: `ModelFallbackMiddleware`/`ModelRetryMiddleware` are consumed by `execute_with_retry_and_temperature` today, not deferred to a future plan, and `resolve_chain` is a complete, tested implementation (its only forward reference is that `AgentRuntimeConfig::build_chain`, landing in plan 26-20, will be its caller — stated in its own rustdoc, not a stub).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `ModelFallbackMiddleware`, `ModelRetryMiddleware` and `ModelFallbackConfig::resolve_chain` are ready for plan 26-20's `AgentRuntimeConfig::build_chain` to assemble in the documented `limits -> guardrail -> trimmer/summarizer -> recall -> protocol -> resilience` order — no further construction-signature changes expected.
- `ModelCallContext::effective_llm` and `execute_with_retry_and_temperature`'s dual-shape retry loop are locked in their final public/internal shape; no later Phase 26 plan is expected to touch this seam again.
- No blockers for wave 6+ plans.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created/modified files verified present on disk; all four commits (`d180edfc`, `ca15260a`, `2bbb5b80`, `b2f754c6`) verified present in `git log`. Full workspace `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --all --check` all pass clean.
