---
phase: 26-agent-runtime-enhancements
plan: 20
subsystem: agent-runtime
tags: [presets, middleware-chain, tool-loop, build-chain, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 02)
    provides: "AgentRuntimeConfig and its twelve sub-structs (ModelCallLimitConfig, TokenBudgetConfig, ToolCallLimitConfig, GuardrailConfig, HistoryTrimmerConfig, SummarizationConfig, VaultRecallConfig, ModelRetryConfig, ModelFallbackConfig, ToolErrorConfig, StructuredOutputConfig, VaultToolsConfig), attached to Settings, inert by default -- the config surface build_chain assembles against"
  - phase: 26-agent-runtime-enhancements (plans 03, 11, 15)
    provides: "ModelCallLimit/TokenBudget/ToolCallLimit, Guardrail, HistoryTrimmer/SummarizationMiddleware, VaultRecallMiddleware -- the built-in ExecutionMiddleware implementations build_chain constructs"
  - phase: 26-agent-runtime-enhancements (plan 10)
    provides: "ModelFallbackConfig::resolve_chain, ModelRetryMiddleware/ModelFallbackMiddleware -- the resilience section build_chain assembles last"
  - phase: 26-agent-runtime-enhancements (plan 16)
    provides: "InProcessArsenal, CompositeArsenalPort -- the executable-arsenal types a caller composes reasoning_agent's second argument from"
  - phase: 26-agent-runtime-enhancements (plan 19)
    provides: "ToolCallProtocolMiddleware, FinishOnPlainAnswerMiddleware, with_tool_error_config, PaladinError::ArmamentFailed -- the tool-loop middleware set and error policy reasoning_agent installs directly"
  - phase: 26-agent-runtime-enhancements (plan 17)
    provides: "StructuredExecutorExt::execute_structured -- the machinery ReasoningAgent::run_structured delegates to"
provides:
  - "AgentRuntimeConfig::build_chain(&self, deps: &AgentRuntimeDeps) -> Result<Vec<Arc<dyn ExecutionMiddleware>>, AgentRuntimeConfigError> in src/config/agent_runtime.rs: assembles every ENABLED section in the documented fixed order (limits -> guardrail -> trimmer/summarizer -> recall -> resilience), skipping disabled sections without validating them, and collecting every construction problem (invalid guardrail pattern, unresolved fallback providers, a section enabled without its AgentRuntimeDeps dependency) into one AgentRuntimeConfigError::BuildFailed"
  - "AgentRuntimeDeps: the dependency bag build_chain needs (LlmProviderFactory, a token counter defaulting to HeuristicTokenCounter, and optional llm_port/garrison/vault/arsenal), defaulting to fully inert"
  - "paladin::presets::reasoning_agent(llm: Arc<dyn LlmPort>, arsenal: Arc<dyn ArsenalPort>, opts: ReasoningAgentOptions) -> Result<ReasoningAgent, PaladinError> -- the one-liner returning a runnable ReasoningAgent { paladin, service } exposing run()/run_structured<T>()"
  - "ReasoningAgentOptions (Default + builder): system_prompt (documented tool-use default), model, max_loops: 5, max_tool_calls: 20, garrison, tool_errors: FeedToModel, circuit_breaker: CircuitBreaker::new(3, 2, 30s)"
  - "CircuitBreaker::failure_threshold()/success_threshold()/timeout() getters (crates/../circuit_breaker.rs), added so a caller (and this plan's own test) can pin the figures a breaker was actually built with"
  - "crates/doc-examples/src/agent_runtime.rs: a <=15-line ANCHOR region showing reasoning_agent end to end, compiled by cargo check -p paladin-doc-examples and textually aligned with the rustdoc doc test on reasoning_agent itself"
  - "tests/integration/reasoning_agent_test.rs: the seven end-to-end tool-loop behaviors (tool call + answer, empty arsenal, tool-call budget, fed-back tool failure, structured delegation, garrison read/write, vault tools absent)"
affects: [26-21]

tech-stack:
  added: []
  patterns:
    - "A facade helper (build_chain) that assembles a Vec<Arc<dyn ExecutionMiddleware>> from a config struct plus a small dependency bag, collecting every construction problem into one typed error rather than stopping at the first"
    - "A preset function (reasoning_agent) composing a Paladin value type built by direct Node::new construction (not the async PaladinBuilder::build, since the preset's own signature is deliberately synchronous) with a PaladinExecutionService assembled via its existing with_* builder chain"
    - "Installation-order-as-correctness: two after_model middleware (FinishOnPlainAnswerMiddleware, ToolCallProtocolMiddleware) must be installed in the OPPOSITE order from their conceptual reading, because run_after visits the reached prefix last-to-first (the onion shape) -- the LAST-installed middleware's after_model runs FIRST"

key-files:
  created:
    - src/presets/mod.rs
    - crates/doc-examples/src/agent_runtime.rs
    - tests/integration/reasoning_agent_test.rs
  modified:
    - src/config/agent_runtime.rs
    - src/lib.rs
    - src/infrastructure/resilience/circuit_breaker.rs
    - crates/doc-examples/src/lib.rs
    - tests/integration/mod.rs

key-decisions:
  - "reasoning_agent is a synchronous fn, not async fn -- the plan's own doc example chains `reasoning_agent(...)?.run(...).await?` with no `.await` between the call and `?`, which only type-checks if reasoning_agent itself is not async. This rules out composing it from `PaladinBuilder::build()` (which is async, to support its own optional auto-prompt/auto-temperature/handoff-registration async branches this preset never uses); the Paladin is instead constructed directly via `Node::new(PaladinData { .. }, None)`, mirroring the exact fixture pattern already used by this phase's own middleware unit tests (tool_protocol.rs's make_paladin, mod.rs's make_paladin)."
  - "Middleware installation order is FinishOnPlainAnswerMiddleware then ToolCallProtocolMiddleware -- the REVERSE of the reading order 26-19-SUMMARY.md's own 'Next Phase Readiness' note suggested ('ordering them [ToolCallProtocolMiddleware, ..., FinishOnPlainAnswerMiddleware]'). chain.rs's own onion-ordering tests (onion_ordering_full_pass_runs_after_in_reverse) prove `run_after` visits the reached prefix LAST-TO-FIRST: with chain=[A,B], after_model runs B then A. For ToolCallProtocolMiddleware's after_model (which SYNTHESIZES function_call from a tool-call envelope) to run before FinishOnPlainAnswerMiddleware's after_model (which READS function_call to decide whether to finish), Protocol must be installed AFTER Finish in the Vec, not before. Verified empirically: reasoning_agent_runs_a_tool_and_answers (loop_count==2, StopReason::Completed) only passes with this order; the naive reading-order installation would finish the run after one loop, before the tool call is ever recognized. Documented at the exact call site in reasoning_agent's own rustdoc so a future edit does not silently reverse it back."
  - "AgentRuntimeDeps gains two fields beyond the plan's literal 4-dependency list (LlmProviderFactory, a token counter, an optional vault, an optional arsenal): `llm_port: Option<Arc<dyn LlmPort>>` (HistoryTrimmer::new and SummarizationMiddleware::new both require the service's own LlmPort, D-14/D-16) and `garrison: Option<Arc<dyn GarrisonPort>>` (SummarizationMiddleware::new requires a GarrisonPort to remember() the resulting summary, D-16) -- mirroring the identical, already-precedented deviations plans 26-11 and 26-15 recorded for those same two constructors' own literal-text gaps."
  - "build_chain does NOT construct ToolCallProtocolMiddleware/FinishOnPlainAnswerMiddleware at its documented 'protocol' position: that pair has no AgentRuntimeConfig sub-struct (D-36 says so explicitly -- 'no config sub-struct of its own'), so there is no `enabled` flag build_chain could gate on. Installing it there unconditionally whenever `deps.arsenal` is Some would also silently change every existing caller's loop behavior (FinishOnPlainAnswerMiddleware turns v0.9's always-run-to-max_loops default into a stop-on-plain-answer default). The tool-loop middleware set stays dependency-injected by a preset (reasoning_agent) that already holds the arsenal in hand, per D-35's own text -- build_chain's rustdoc records this explicitly at the 'protocol' position, and its own exact-order test (`build_chain_uses_the_documented_fixed_order`) checks only the six positions this plan's config sub-structs actually cover (limits x3, guardrail, trimmer/summarizer, recall, resilience x2)."
  - "AgentRuntimeConfigError gains a second variant, BuildFailed(Vec<String>), beside the pre-existing UnresolvedProviders (plan 26-10) and is marked #[non_exhaustive]. This required fixing two pre-existing `let AgentRuntimeConfigError::UnresolvedProviders(problems) = &err;` irrefutable-pattern statements (plan 26-10's own tests) that only compiled while the enum had exactly one variant -- converted to `let ... else { panic!(...) }`, same assertions, same behavior."
  - "vault_recall's AgentRuntimeDeps.vault is NOT consumed to construct VaultRecallMiddleware (whose `new` takes only a VaultRecallConfig -- the middleware reads a run's actual grant from ModelCallContext::vault at runtime, installed at the SERVICE level via PaladinExecutionService::with_vault). Its presence in AgentRuntimeDeps is instead a build_chain-time acknowledgement that the vault is actually wired: enabling vault_recall while deps.vault is None is a typed BuildFailed error rather than installing a middleware that can only ever be a permanent no-op for every run on that service (T-26-64)."
  - "Test discovery: this workspace's tests/ integration binary target is named `lib`, not `integration` (confirmed by `cargo test --test integration ...` failing with 'no test target named integration' and listing `lib` among the available targets) -- the plan's own <verify> commands literally say `--test integration`; this plan's actual verification commands use `--test lib`, per the project's own house rules (\"every file under tests/integration/ is a module of the single tests/lib.rs binary\")."
  - "Six of this plan's own <behavior> tests (Tests 1, 3-8) live in tests/integration/reasoning_agent_test.rs, not as unit tests inside src/presets/mod.rs, because the plan's own verify commands run them via `cargo test --test integration <name>` (a binary-scoped lookup that cannot find a unit test in the lib crate). Only Test 2 (defaults_match_the_documented_options) stays a unit test beside reasoning_agent, since it asserts ReasoningAgentOptions' PRIVATE fields, which only a same-module test can reach."
  - "CircuitBreaker gained three new public getters (failure_threshold, success_threshold, timeout) -- a minimal, additive, non-breaking change required because Test 2 (defaults_match_the_documented_options) must assert the README's 3/2/30s figures against the ACTUAL Arc<CircuitBreaker> ReasoningAgentOptions::default() builds, and CircuitBreaker previously exposed no way to read back its own construction arguments."

patterns-established:
  - "A preset's Options struct keeps its fields private with a Default + fluent with_*/plain-name builder methods (matching PaladinBuilder's own convention), rather than public fields -- so a unit test asserting documented defaults lives beside the type, and external callers only ever go through the builder."

requirements-completed: [RT-07, RT-02]

coverage:
  - id: D1
    description: "AgentRuntimeConfig::build_chain assembles every enabled section (limits, guardrail, trimmer/summarizer, recall, resilience) in the documented fixed order, skipping disabled sections without constructing or validating them, and yields an empty chain on a fully-disabled config"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::build_chain_on_a_default_config_returns_an_empty_chain, tests::build_chain_assembles_only_enabled_sections, tests::build_chain_uses_the_documented_fixed_order, tests::a_disabled_section_is_never_constructed"
        status: pass
    human_judgment: false
  - id: D2
    description: "build_chain collects every configuration problem (an invalid guardrail regex AND unresolved fallback providers) into one AgentRuntimeConfigError::BuildFailed rather than stopping at the first, and a section enabled without the AgentRuntimeDeps dependency it needs (vault_recall without a vault, summarization without a garrison) is a typed error naming the missing dependency, never a silent skip"
    requirement: "RT-02"
    verification:
      - kind: unit
        ref: "src/config/agent_runtime.rs#tests::build_chain_reports_every_configuration_failure_at_once, tests::build_chain_requires_the_dependency_a_section_needs"
        status: pass
    human_judgment: false
  - id: D3
    description: "reasoning_agent(llm, arsenal, opts) returns a runnable ReasoningAgent whose run() completes a scripted tool-call-then-answer sequence with loop_count == 2 and StopReason::Completed, taking an executable Arc<dyn ArsenalPort> rather than PRD RT-FR-23's Vec<Armament> (an Armament cannot execute)"
    requirement: "RT-07"
    verification:
      - kind: integration
        ref: "tests/integration/reasoning_agent_test.rs#reasoning_agent_runs_a_tool_and_answers"
        status: pass
      - kind: other
        ref: "grep -c 'Arc<dyn ArsenalPort>' src/presets/mod.rs >= 1; grep -v '^\\s*//\\|^\\s*///' src/presets/mod.rs | grep -c 'Vec<Armament>' == 0; grep -c 'RT-FR-23\\|cannot execute' src/presets/mod.rs >= 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "ReasoningAgentOptions::default() matches every documented figure: max_loops 5, max_tool_calls 20, tool_errors FeedToModel, a non-empty system prompt, and a circuit breaker with the README's 3/2/30s figures (pinned via new CircuitBreaker getters)"
    requirement: "RT-07"
    verification:
      - kind: unit
        ref: "src/presets/mod.rs#tests::defaults_match_the_documented_options"
        status: pass
    human_judgment: false
  - id: D5
    description: "An arsenal with no registered tools still runs the preset (no ## Tools section, one loop, StopReason::Completed); a tool-call budget of 1 denies a second attempted call and the run still completes; a failing tool closure's error is fed back by default and the run completes; run_structured delegates to StructuredExecutorExt; a supplied garrison is written to and read from; the preset never lists vault_get/vault_put"
    requirement: "RT-07"
    verification:
      - kind: integration
        ref: "tests/integration/reasoning_agent_test.rs#an_empty_arsenal_still_runs, max_tool_calls_is_enforced, tool_failure_is_fed_back_by_default, run_structured_delegates_to_the_extension, a_garrison_is_used_when_supplied, the_preset_does_not_enable_vault_tools"
        status: pass
    human_judgment: false
  - id: D6
    description: "The reasoning_agent example is anchored in crates/doc-examples/src/agent_runtime.rs at <=15 lines (14 measured), compiled by cargo check -p paladin-doc-examples, and the SAME example is also a rustdoc doc test on reasoning_agent itself that actually executes under cargo test -p paladin-ai --doc"
    requirement: "RT-07"
    verification:
      - kind: other
        ref: "cargo check -p paladin-doc-examples (exit 0); sed -n '/ANCHOR: reasoning_agent/,/ANCHOR_END: reasoning_agent/p' crates/doc-examples/src/agent_runtime.rs | grep -vc '^\\s*$|ANCHOR' == 14"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --doc reasoning_agent (2 passed: line 227, line 269)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo check -p paladin-doc-examples, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --all-features --lib (941/941), cargo test -p paladin-ai --doc (131/131, 18 ignored pre-existing), cargo test --test lib reasoning_agent (7/7)"
    requirement: "RT-02"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo check -p paladin-doc-examples (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --all-features --lib (941 passed, 0 failed, 6 ignored); cargo test -p paladin-ai --doc (131 passed, 0 failed, 18 ignored -- pre-existing)"
        status: pass
      - kind: integration
        ref: "cargo test --test lib reasoning_agent (7 passed, 0 failed)"
        status: pass
    human_judgment: false

duration: ~3h
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 20: AgentRuntimeConfig::build_chain and the reasoning_agent Preset Summary

**`AgentRuntimeConfig::build_chain` assembles every enabled built-in middleware into one ordered chain with a collected typed error, and `paladin::presets::reasoning_agent(llm, arsenal, opts)` is the one-liner that turns a mock model and one in-process tool closure into an executed, tool-using `PaladinResult` -- both proven end to end, not just unit-tested in isolation.**

## Performance

- **Duration:** ~3h (dominated by full-workspace `--all-features` compiles: three `cargo check`, one `cargo clippy`, and several `cargo test` invocations against a cold `target/`)
- **Tasks:** 3 (2 `tdd="true"`, 1 `type="auto"`)
- **Files modified:** 8 (3 created, 5 modified)

## Accomplishments

- `AgentRuntimeConfig::build_chain(&self, deps: &AgentRuntimeDeps) -> Result<Vec<Arc<dyn ExecutionMiddleware>>, AgentRuntimeConfigError>` (`src/config/agent_runtime.rs`) assembles every ENABLED section -- `ModelCallLimit`, `TokenBudget`, `ToolCallLimit` (limits); `Guardrail`; `SummarizationMiddleware` (or, absent that, a standalone `HistoryTrimmer`) for the trimmer/summarizer slot; `VaultRecallMiddleware` (recall); `ModelRetryMiddleware` then `ModelFallbackMiddleware` (resilience) -- in that documented fixed order, never validating or constructing a disabled section. Every construction problem across the whole call -- an invalid guardrail regex, unresolved fallback provider names (reusing `ModelFallbackConfig::resolve_chain`, never re-resolving), or a section enabled without the `AgentRuntimeDeps` dependency it needs -- is collected into ONE `AgentRuntimeConfigError::BuildFailed(Vec<String>)`, never the first-problem-wins shape.
- `AgentRuntimeDeps` is the small dependency bag `build_chain` needs: an `LlmProviderFactory`, a token counter defaulting to `HeuristicTokenCounter`, and optional `llm_port`/`garrison`/`vault`/`arsenal` -- `AgentRuntimeDeps::default()` plus `AgentRuntimeConfig::default()` builds an empty chain, the assembly-point half of the inertness guarantee plan 26-02's `default_agent_runtime_config_is_inert` already proved at the config point.
- `paladin::presets::reasoning_agent(llm: Arc<dyn LlmPort>, arsenal: Arc<dyn ArsenalPort>, opts: ReasoningAgentOptions) -> Result<ReasoningAgent, PaladinError>` (`src/presets/mod.rs`, new top-level `pub mod presets;`) composes a directly-constructed `Paladin` (`Node::new`, not the async `PaladinBuilder::build`) with a `PaladinExecutionService` carrying `FinishOnPlainAnswerMiddleware`, `ToolCallProtocolMiddleware` over `arsenal`, a `ToolCallLimit` from `opts.max_tool_calls`, and `opts.tool_errors` via `with_tool_error_config`. `ReasoningAgent { paladin, service }` exposes `run()` and a `run_structured<T>()` that thinly delegates to `StructuredExecutorExt::execute_structured`. The signature deliberately deviates from PRD RT-FR-23's `tools: Vec<Armament>` (an `Armament` is a definition, not something that can execute) -- documented prominently in the function's own rustdoc.
- `ReasoningAgentOptions` (`Default` + fluent builder, private fields) carries the documented defaults: a tool-use `system_prompt`, `max_loops: 5`, `max_tool_calls: 20`, no `garrison`, `tool_errors: FeedToModel`, and `circuit_breaker: Some(CircuitBreaker::new(3, 2, 30s))` -- the README figures, now readable back off a real `CircuitBreaker` via three new getters (`failure_threshold`/`success_threshold`/`timeout`) this plan added.
- `crates/doc-examples/src/agent_runtime.rs` anchors a 14-line `reasoning_agent` example (compiled by `cargo check -p paladin-doc-examples`) textually aligned with the SAME example committed as a rustdoc doc test on `reasoning_agent` itself (the copy `cargo test -p paladin-ai --doc` actually runs) -- both script a tool-call envelope then a plain answer and assert `loop_count == 2`, `StopReason::Completed`, and the output containing `4`.
- `tests/integration/reasoning_agent_test.rs` proves the preset end to end for every `<behavior>` item this plan lists: the tool loop, an empty arsenal, the tool-call budget, a fed-back tool failure, structured-output delegation, a garrison being read from and written to, and the absence of vault tools -- registered in `tests/integration/mod.rs` and run via `cargo test --test lib` (this workspace's `tests/` binary target is named `lib`, not `integration`, contrary to the plan's own literal `<verify>` text).

## Task Commits

1. **Task 1: `AgentRuntimeConfig::build_chain`** -- `ab804126` (feat)
2. **Task 2: `reasoning_agent` -- the runnable pair** -- `88d3a0a1` (feat)
3. **Task 3: The <=15-line doc-tested example** -- `a5ae685e` (docs)

**Plan metadata:** this commit (SUMMARY.md)

Both Task 1 and Task 2 combine their RED/GREEN cycle into a single `feat` commit -- the types under test (`build_chain`, `AgentRuntimeDeps`, `reasoning_agent`, `ReasoningAgent`, `ReasoningAgentOptions`) did not exist anywhere in the tree before each commit, so a genuinely-compiling RED state was impossible; the same precedent is already recorded in `26-02-SUMMARY.md`, `26-15-SUMMARY.md`, `26-16-SUMMARY.md` and `26-19-SUMMARY.md` for this phase. Every named test was written and run to green before its commit landed.

## Files Created/Modified

- `src/config/agent_runtime.rs` -- `AgentRuntimeDeps`, `AgentRuntimeConfig::build_chain`, `AgentRuntimeConfigError::BuildFailed` (new variant, enum now `#[non_exhaustive]`), 6 new tests; fixed two pre-existing irrefutable-`let` statements the new variant broke
- `src/presets/mod.rs` (new) -- `ReasoningAgent`, `ReasoningAgentOptions`, `reasoning_agent`, 1 unit test
- `src/lib.rs` -- `pub mod presets;`
- `src/infrastructure/resilience/circuit_breaker.rs` -- `failure_threshold()`, `success_threshold()`, `timeout()` getters
- `crates/doc-examples/src/agent_runtime.rs` (new) -- the anchored `reasoning_agent` example
- `crates/doc-examples/src/lib.rs` -- registers `agent_runtime`
- `tests/integration/reasoning_agent_test.rs` (new) -- 7 end-to-end tests
- `tests/integration/mod.rs` -- registers `reasoning_agent_test`

## Decisions Made

See `key-decisions` in the frontmatter for full reasoning. In short: `reasoning_agent` is synchronous (the plan's own doc example requires it, ruling out `PaladinBuilder::build`); `FinishOnPlainAnswerMiddleware` must be installed BEFORE `ToolCallProtocolMiddleware` (the reverse of a plausible reading, required by `run_after`'s onion-shaped reverse traversal -- verified empirically, not just reasoned); `AgentRuntimeDeps` gained `llm_port`/`garrison` fields beyond the plan's literal 4-dependency list (structurally required, precedented in plans 26-11/26-15); `build_chain` does not construct the tool-call protocol pair (no config sub-struct exists for it, per D-36); `AgentRuntimeConfigError` gained a second variant requiring two pre-existing test fixes; six of this plan's own behavior tests live in the integration test file rather than as unit tests, matching the plan's own binary-scoped verify commands; `CircuitBreaker` gained three getters to make its own defaults independently verifiable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `clippy::default_constructed_unit_structs` on `LlmProviderFactory::default()`**
- **Found during:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` after Task 1's implementation
- **Issue:** `LlmProviderFactory` is a unit struct; `clippy::default_constructed_unit_structs` (implied by `-D warnings`) rejects `LlmProviderFactory::default()` in favor of the bare literal.
- **Fix:** Changed `AgentRuntimeDeps::default()` to construct `LlmProviderFactory` directly (`llm_provider_factory: LlmProviderFactory`).
- **Files modified:** `src/config/agent_runtime.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0
- **Committed in:** `ab804126` (Task 1 commit)

**2. [Rule 3 - Blocking] Two pre-existing irrefutable-`let` patterns broke when `AgentRuntimeConfigError` gained a second variant**
- **Found during:** Task 1, first compile after adding `AgentRuntimeConfigError::BuildFailed`
- **Issue:** Plan 26-10's own tests (`unknown_provider_is_a_typed_error_listing_every_offender`, `uncompiled_provider_is_reported_distinctly_from_unknown`) used `let AgentRuntimeConfigError::UnresolvedProviders(problems) = &err;` -- an irrefutable pattern that only compiles when the enum has exactly one variant. Adding `BuildFailed` made the pattern refutable (E0005).
- **Fix:** Converted both to `let AgentRuntimeConfigError::UnresolvedProviders(problems) = &err else { panic!(...) };` -- same assertions, same runtime behavior, now exhaustive.
- **Files modified:** `src/config/agent_runtime.rs`
- **Verification:** `cargo test -p paladin-ai --all-features --lib config::agent_runtime::tests` (20/20 pass, including both fixed tests)
- **Committed in:** `ab804126` (Task 1 commit)

**3. [Rule 3 - Blocking] `cargo fmt` drift caught by the pre-commit hook, not by manual review**
- **Found during:** First `git commit` attempt for Task 1
- **Issue:** Several multi-line expressions in `build_chain` and its tests did not match `rustfmt`'s canonical wrapping.
- **Fix:** Ran `cargo fmt --all` before re-committing.
- **Files modified:** `src/config/agent_runtime.rs`, `tests/integration/reasoning_agent_test.rs` (the latter had no semantic change, formatting only)
- **Verification:** `cargo fmt --all --check` exits 0; the subsequent commit's pre-commit hook passed `cargo fmt (check)`
- **Committed in:** `ab804126`

---

**Total deviations:** 3 auto-fixed (1 clippy-driven bug fix, 1 blocking compile fix required by this plan's own new error variant, 1 blocking formatting fix). None touched planned behavior; no scope creep.

## Issues Encountered

- **The plan's own `<verify>` commands name a `--test integration` binary that does not exist.** `cargo test --test integration reasoning_agent` fails with "no test target named `integration`"; the actual target is `lib` (confirmed by the error's own "available test targets" listing and by this project's house rules, which state the same thing). All verification in this SUMMARY uses `cargo test --test lib` instead -- the tests themselves, their names, and their content are exactly what the plan specifies.
- **The acceptance criterion `grep -c 'LlmProviderFactory::create' src/config/agent_runtime.rs is at most 1` does not hold** -- it reads `3` on this tree, from three PRE-EXISTING rustdoc comments in `ModelFallbackConfig`'s own documentation (plan 26-10, unmodified by this plan; confirmed via `git show HEAD~1... | grep`). The actual code calls `factory.create(name)` (a different literal) exactly once, inside `resolve_chain`, and `build_chain` reuses `resolve_chain` rather than calling the factory a second time -- the real invariant this criterion exists to protect ("provider resolution is not duplicated") holds; the grep's literal string just also matches three doc-comment mentions that predate this plan. Not fixed here (editing plan 26-10's rustdoc prose is out of this plan's scope per the deviation rules' scope boundary), and flagged here for visibility.

## Known Stubs

None. `build_chain`, `reasoning_agent`, `ReasoningAgent`, `ReasoningAgentOptions` and the anchored doc example are all fully implemented per the plan's `<action>`/`<done>` clauses, with real (not placeholder) tests for every `<behavior>` item across all three tasks.

## Threat Flags

None beyond what the plan's own `<threat_model>` already covers (T-26-63, T-26-64, T-26-22, T-26-65) -- no new network endpoint, auth path, file-access pattern, or schema change at a trust boundary was introduced. `build_chain`'s typed-error-over-silent-skip behavior (T-26-64) and `reasoning_agent`'s vault-tools absence (T-26-63) are both directly asserted by named tests listed in `coverage` above.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `AgentRuntimeConfig::build_chain` and `paladin::presets::reasoning_agent` are both locked in their final public shape. Plan 26-21 (the user guide) can `{{#include}}` `crates/doc-examples/src/agent_runtime.rs`'s `reasoning_agent` ANCHOR region directly, and can cite `build_chain`'s documented order and `AgentRuntimeDeps`'s shape without re-deriving either.
- Whether `paladin-server`'s agent provisioning calls `build_chain` remains open, per D-10's own "Claude's discretion" framing -- this plan did not wire it into `src/infrastructure/web/agent_host.rs` or `facade_provisioner.rs`, since no plan task named that integration point and doing so unprompted would be scope creep beyond this plan's own `files_modified` list.
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 13 worktree agents complete.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `src/config/agent_runtime.rs` -- FOUND, contains `pub fn build_chain(` and `pub struct AgentRuntimeDeps`
- `src/presets/mod.rs` -- FOUND, contains `pub fn reasoning_agent(`, `pub struct ReasoningAgent`, `pub struct ReasoningAgentOptions`
- `src/lib.rs` -- FOUND, contains `pub mod presets;`
- `crates/doc-examples/src/agent_runtime.rs` -- FOUND, contains `// ANCHOR: reasoning_agent` / `// ANCHOR_END: reasoning_agent`
- `crates/doc-examples/src/lib.rs` -- FOUND, registers `pub mod agent_runtime;`
- `tests/integration/reasoning_agent_test.rs` -- FOUND, registered in `tests/integration/mod.rs`
- `src/infrastructure/resilience/circuit_breaker.rs` -- FOUND, contains `pub fn failure_threshold(`
- Commit `ab804126` -- FOUND in `git log --oneline`
- Commit `88d3a0a1` -- FOUND in `git log --oneline`
- Commit `a5ae685e` -- FOUND in `git log --oneline`
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo check -p paladin-doc-examples` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo test -p paladin-ai --all-features --lib` -- 941 passed, 0 failed, 6 ignored (pre-existing)
- `cargo test -p paladin-ai --doc` -- 131 passed, 0 failed, 18 ignored (pre-existing)
- `cargo test --test lib reasoning_agent` -- 7 passed, 0 failed
- `cargo test -p paladin-ai --doc reasoning_agent` -- 2 passed, 0 failed
- `cargo test -p paladin-ai --all-features --lib config::agent_runtime::tests` -- 20 passed, 0 failed
- `cargo test -p paladin-ai --all-features --lib presets::` -- 1 passed, 0 failed
- `git diff --diff-filter=D --name-only HEAD~3 HEAD` -- empty (no unexpected deletions across all three task commits)
