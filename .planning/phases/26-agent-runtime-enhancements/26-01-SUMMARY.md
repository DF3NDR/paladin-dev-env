---
phase: 26-agent-runtime-enhancements
plan: 01
subsystem: agent-runtime
tags: [middleware, execution-hooks, paladin-execution-service, war-engine, rust]

requires: []
provides:
  - "ExecutionMiddleware trait, MiddlewareFlow/ToolFlow enums, and FinalResult in its final shape"
  - "PromptAssembly/PromptSection/SectionPlacement structured prompt in place of the flat prompt String"
  - "ModelCallContext/ToolCallContext/LlmResponseView per-run and per-call value types, including the llm_override/retry_policy fields plan 26-10 reads"
  - "The onion-ordering chain driver (run_before/run_after/run_around_tool) with pinned Finish/Fail short-circuit semantics"
  - "PaladinExecutionService::with_middleware/with_middleware_chain wired at all four reasoning-loop hook sites"
  - "The two-layer NodeInterceptor/ExecutionMiddleware documentation contract and a proven engine-node parity path"
affects: [26-02, 26-03, 26-05, 26-08, 26-10, 26-15, 26-19, 26-21]

tech-stack:
  added: []
  patterns:
    - "Hook chain beside the loop it wraps (mirrors paladin-battalion::engine::hooks::NodeInterceptor)"
    - "Empty-chain golden equivalence test as the regression baseline for a new hook seam"
    - "Stateless Arc<dyn Trait> middleware with all per-run state on a context value type (no factory trait)"
    - "AnyMap-shaped typed state bag keyed by (middleware name, TypeId)"

key-files:
  created:
    - src/application/services/paladin/middleware/mod.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/middleware/chain.rs
    - tests/integration/middleware_under_engine_test.rs
  modified:
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/paladin/mod.rs
    - src/prelude.rs
    - crates/paladin-llm/src/mock.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - tests/integration/mod.rs

key-decisions:
  - "ModelCallContext borrows &'p Paladin (not Arc<Paladin>) for the lifetime of execute_internal, matching the PRD's 'read-only &Paladin handle' literally"
  - "ToolCallContext.scratch is a cloned snapshot of the run's scratch at dispatch time, not a live reference, to avoid threading a second lifetime through the tool-call path"
  - "MiddlewareFlow::Finish from before_model builds a synthetic LlmResponseView (from FinalResult.output) so after_model still runs over the reached prefix with zero LlmPort calls"
  - "MockLlmAdapter (crates/paladin-llm/src/mock.rs) gained request recording (requests()/last_request()/last_prompt()) and MockScriptEntry::ToolCall scripting -- needed by the golden equivalence and tool/handoff tests, and reusable by every later Phase 26 plan"
  - "The plan's stated `cargo test --test integration middleware_under_engine` verify command does not resolve in this workspace (no `[[test]] name = \"integration\"` target exists); corrected to `cargo test --test lib middleware_under_engine`, matching the auto-discovered `tests/lib.rs` binary every other `tests/integration/*_test.rs` file already compiles into"

patterns-established:
  - "Structured PromptAssembly renders byte-identically to the old flat-string builder with zero sections; new middleware insert PromptSection at a documented SectionPlacement, never by string concatenation into the prompt"
  - "around_tool wraps both the Arsenal branch and the handoff branch identically via one ArmamentCall/ToolCallContext shape, with Deny/Rewrite handled once per branch"

requirements-completed: [RT-01]

coverage:
  - id: D1
    description: "ExecutionMiddleware trait, MiddlewareFlow/ToolFlow, FinalResult, and the four per-run/per-call context types exist in their final shape and are re-exported from paladin::prelude"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/mod.rs#tests, src/application/services/paladin/middleware/chain.rs#tests"
        status: pass
    human_judgment: false
  - id: D2
    description: "An empty middleware chain reproduces today's rendered prompt bytes, LlmPort call count, and PaladinResult exactly"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#middleware_wiring_tests::empty_chain_renders_byte_identical_prompt"
        status: pass
    human_judgment: false
  - id: D3
    description: "before_model/after_model fire once per reasoning-loop iteration, onion-ordered, with Finish short-circuit and Fail propagation pinned"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#middleware_wiring_tests::recording_middleware_observes_before_model_then_after_model_per_iteration, src/application/services/paladin/middleware/chain.rs#tests::onion_ordering_finish_from_second_middleware, src/application/services/paladin/middleware/chain.rs#tests::onion_ordering_full_pass_runs_after_in_reverse, src/application/services/paladin/middleware/chain.rs#tests::fail_from_before_model_propagates_unchanged_and_is_not_retried"
        status: pass
    human_judgment: false
  - id: D4
    description: "around_tool wraps both the Arsenal and handoff dispatch branches, with Deny/Rewrite/Allow semantics pinned"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#middleware_wiring_tests::around_tool_fires_for_both_arsenal_and_handoff_dispatch, ::tool_flow_deny_injects_the_reason_where_a_tool_error_is_injected_today, ::tool_flow_rewrite_replaces_the_call_before_dispatch"
        status: pass
    human_judgment: false
  - id: D5
    description: "Ten concurrent runs through one PaladinExecutionService instance sharing one middleware Arc keep independent per-run context state; sequential runs do not leak scratch; typed state is keyed by middleware name + TypeId"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/mod.rs#tests::concurrent_runs_keep_independent_context_state, ::sequential_runs_do_not_leak_scratch, ::typed_state_is_keyed_by_middleware_name_and_type"
        status: pass
    human_judgment: false
  - id: D6
    description: "The streaming path (execute_stream) runs before_model only; a Paladin node under a WarEngine applies its service's middleware chain unchanged, nested inside the NodeInterceptor layer, with no engine-side middleware registry"
    requirement: "RT-01"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#middleware_wiring_tests::streaming_path_runs_before_model_only"
        status: pass
      - kind: integration
        ref: "tests/integration/middleware_under_engine_test.rs::paladin_node_under_engine_runs_the_service_middleware_chain, ::node_interceptor_and_execution_middleware_are_independent_layers, ::engine_needs_no_middleware_registry"
        status: pass
    human_judgment: false

duration: ~2h
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 01: ExecutionMiddleware Seam Summary

**Ordered `ExecutionMiddleware` chain (`before_model`/`after_model`/`around_tool`) wired into `PaladinExecutionService`'s reasoning loop via a structured `PromptAssembly`, proven byte-identical to today's behavior on an empty chain and dispatched correctly under a `WarEngine`.**

## Performance

- **Duration:** ~2h
- **Tasks:** 3 (tracer + 2 auto)
- **Files modified:** 11 (4 created, 7 modified)

## Accomplishments

- The `ExecutionMiddleware` trait, `MiddlewareFlow`/`ToolFlow` enums, `FinalResult`, and the `ModelCallContext`/`ToolCallContext`/`LlmResponseView`/`PromptAssembly`/`PromptSection` value types now exist in their final shape (`src/application/services/paladin/middleware/{mod,context}.rs`), re-exported from `paladin::prelude`.
- `PaladinExecutionService`'s flat per-iteration prompt `String` is now a `PromptAssembly` built from the same inputs and rendered through `PromptAssembly::render()` — byte-identical output when no middleware is attached, proven by a golden test against a real `paladin_llm::mock::MockLlmAdapter`.
- `before_model` fires once per loop iteration (after the assembly is built, before the model call); `after_model` fires on the final per-iteration response (after the buffered retry and circuit breaker); `around_tool` wraps both the Arsenal branch and the handoff branch with `Allow`/`Deny`/`Rewrite` semantics.
- The onion-ordering chain driver (`middleware/chain.rs`) is exercised by ordering, short-circuit, per-run-isolation, and typed-state tests, including a `#[tokio::test(flavor = "multi_thread")]` stress test asserting exact counts across ten concurrent runs sharing one `PaladinExecutionService` and one middleware `Arc`.
- A one-node `WarGraph` dispatched through a `WarEngine` applies the same service's middleware chain unchanged (no engine change), with the interceptor/middleware nesting order pinned by an integration test; both `NodeInterceptor` and `ExecutionMiddleware`'s rustdoc now carry the two-layer distinction table.

## Task Commits

1. **Task 1: End-to-end "one recording middleware observes a whole run"** — `0e522216` (test: MockLlmAdapter extension), `49673d3d` (feat: wiring the chain into the reasoning loop)
2. **Task 2: Pin onion ordering, short-circuit semantics and per-run isolation under concurrency** — ordering/short-circuit tests landed with the chain driver in `0e522216`; isolation/typed-state tests in `1d0c6579`
3. **Task 3: Engine-node parity and the two-layer NodeInterceptor/ExecutionMiddleware contract** — `a7ee9dc3`

_Note: the RED/GREEN split for Task 1 is approximate rather than literal — `0e522216` lands the middleware trait, contexts and chain driver (new code with no prior behavior, plus the MockLlmAdapter test-infra extension the wiring tests need to compile), and `49673d3d` wires those types into the reasoning loop's four call sites. The chain driver's own ordering/short-circuit tests (`chain.rs`) were written and passed alongside the driver in the same commit as genuinely new logic; the service-level `middleware_wiring_tests` module (8 behavior tests: empty-chain equivalence, before/after alternation, around_tool on both branches, Deny/Rewrite, assembly mutation, streaming coverage, context isolation) was written against the already-compiling-but-unwired service and initially failed on everything except the golden equivalence test (which is a regression baseline, not new behavior) before the wiring commit made all 8 pass — this is the closest practical RED/GREEN split for a change of this structural scope, documented here rather than fabricating a literal per-test RED commit._

## Files Created/Modified

- `src/application/services/paladin/middleware/mod.rs` — `ExecutionMiddleware` trait (default no-op bodies, no default for `name()`), `MiddlewareFlow`, `ToolFlow`, `FinalResult`, module re-exports, isolation/typed-state tests
- `src/application/services/paladin/middleware/context.rs` — `PromptAssembly`, `PromptSection`, `SectionPlacement`, `ModelCallContext<'p>` (scratch, typed state bag, `llm_override`, `retry_policy`), `ToolCallContext`, `ToolCallKind`, `LlmResponseView`
- `src/application/services/paladin/middleware/chain.rs` — `run_before`/`run_after`/`run_around_tool`, `BeforeOutcome`, ordering/short-circuit tests
- `src/application/services/paladin/paladin_execution_service.rs` — `middleware` field, `with_middleware`/`with_middleware_chain` builders, the four hook call sites, `execute_stream_inner`'s before-only wiring, removal of the now-dead `build_prompt_with_custom_system`, `middleware_wiring_tests` module
- `src/application/services/paladin/mod.rs` — `pub mod middleware;`
- `src/prelude.rs` — re-exports of the new middleware public surface
- `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter` request recording (`requests`/`last_request`/`last_prompt`) and `MockScriptEntry`/`with_script` for scripting `FunctionCall` responses
- `crates/paladin-battalion/src/engine/hooks.rs` — `NodeInterceptor` rustdoc gains the two-layer table's third row
- `crates/paladin-ports/src/output/paladin_port.rs` — `PaladinPort` rustdoc states the middleware-applies-automatically contract
- `tests/integration/middleware_under_engine_test.rs` — the engine-node parity integration test (new file)
- `tests/integration/mod.rs` — registers the new test module

## Decisions Made

- **`ModelCallContext<'p>` borrows the Paladin by reference**, not by `Arc`, matching the plan's literal "a read-only `&Paladin` handle" wording and avoiding an unnecessary clone/Arc-wrap per run.
- **`ToolCallContext.scratch` is a cloned snapshot**, not a live reference into the run's scratch — simpler lifetime story, and `around_tool` implementations only need read access per the plan.
- **A `Finish` from `before_model` still runs `after_model`** over the reached prefix, via a synthetic `LlmResponseView` built from the `FinalResult`'s output — matches the plan's Test 1 assertion (`A.before, B.before, A.after`, zero `LlmPort` calls) exactly.
- **`MockLlmAdapter` was extended rather than building a parallel test double** — `requests()`/`last_prompt()` and `MockScriptEntry::ToolCall` are additive, non-breaking, and now available to every subsequent Phase 26 plan's tests.
- **Verify-command correction**: the plan's Task 3 `<verify>` names `cargo test --test integration middleware_under_engine`; this workspace has no `[[test]] name = "integration"` Cargo target (confirmed via `cargo test --test integration -- --list`, which lists the actual target names, including `lib` — the auto-discovered binary from `tests/lib.rs` that every other `tests/integration/*_test.rs` file already compiles into). Used `cargo test --test lib middleware_under_engine` instead; all 3 tests pass.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Corrected the Task 3 verify command's test binary name**
- **Found during:** Task 3 verification
- **Issue:** The plan's `<verify>` command (`cargo test --test integration middleware_under_engine`) references a Cargo test target that does not exist in this workspace's `Cargo.toml`.
- **Fix:** Ran `cargo test --test integration -- --list` to enumerate real targets, confirmed `tests/integration/*_test.rs` files are compiled into the auto-discovered `lib` binary (from `tests/lib.rs`), and used `cargo test --test lib middleware_under_engine` instead.
- **Files modified:** none (verification-only)
- **Verification:** `cargo test --test lib middleware_under_engine` selects and passes 3 tests
- **Committed in:** `a7ee9dc3` (documented in the commit message)

**2. [Rule 1 - Bug] Preserved exact malformed-JSON error surfacing when refactoring `handle_tool_call`**
- **Found during:** Task 1, wiring `around_tool` around the Arsenal branch
- **Issue:** Moving argument parsing out of `handle_tool_call` (so `around_tool` can inspect a concrete `ArmamentCall` before dispatch) risked silently swallowing a malformed-JSON `ArsenalError::InvalidArguments` that the original code surfaced as a tool-error message before ever calling the Arsenal.
- **Fix:** Added `parse_armament_call` as a dedicated fallible step run BEFORE `around_tool`/dispatch, reproducing the original error variant, message and control flow (no Arsenal call on parse failure) exactly; `function_call_to_armament_call` (used only on the handoff path, which already silently defaults on parse failure via `execute_handoff`'s own `unwrap_or_default()`) is a separate, infallible helper.
- **Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
- **Verification:** all pre-existing `paladin_execution_service` tests continue to pass unchanged (102/102); `tool_flow_deny_injects_the_reason_where_a_tool_error_is_injected_today` and `tool_flow_rewrite_replaces_the_call_before_dispatch` cover the new call sites
- **Committed in:** `49673d3d`

---

**Total deviations:** 2 auto-fixed (1 blocking test-target correction, 1 bug-prevention during refactor)
**Impact on plan:** Both were necessary to make the plan's own verification runnable and to avoid a byte-for-byte behavior regression on the empty-chain path. No scope creep.

## Issues Encountered

None beyond the two deviations above — every acceptance criterion in the plan (RED/GREEN empty-chain equivalence, onion ordering, `Finish`/`Fail` semantics, per-run isolation, engine-node parity, two-layer documentation) was met on the first or second implementation pass, verified against `cargo check --workspace --all-targets --all-features`, `cargo test -p paladin-ai --lib` (569 passed), `cargo test --test lib` (706 passed), `cargo test -p paladin-ai --doc` (118 passed, including 3 new middleware doc tests), `cargo doc --workspace --no-deps` (no new broken links), `cargo fmt --all --check`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings` (all clean).

## Known Stubs

None. Every new type and call site is real, wired, production code (this plan is the phase's tracer, not a prototype).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The `ExecutionMiddleware`, `MiddlewareFlow`, `ToolFlow`, `ModelCallContext`, `ToolCallContext`, `LlmResponseView`, `PromptAssembly`, `PromptSection` types are locked in their final public shape; plans 26-05 through 26-19's eleven built-in middleware can be written against them without further seam changes.
- `ModelCallContext` already carries `llm_override: Option<Arc<dyn LlmPort>>` and `retry_policy: Option<RetryPolicy>` for plan 26-10's port-shaping middleware to consume at the single model-call site.
- `PromptAssembly`'s `SectionPlacement::AfterRetrievedContext` is ready for plan 26-15's `VaultRecallMiddleware` without a struct reshape.
- No blockers for wave 2 plans.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created files verified present on disk; all five commits (`0e522216`, `49673d3d`, `1d0c6579`, `a7ee9dc3`, `30ccff90`) verified present in `git log`.
