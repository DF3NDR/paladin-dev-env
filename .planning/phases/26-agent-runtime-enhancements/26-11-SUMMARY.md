---
phase: 26-agent-runtime-enhancements
plan: 11
subsystem: agent-runtime
tags: [token-counter, history-trimmer, context-window, middleware, tiktoken, rust]

requires:
  - phase: 26-01
    provides: "ExecutionMiddleware trait, MiddlewareFlow, ModelCallContext/PromptAssembly/PromptSection, and the onion-ordering chain driver this plan's HistoryTrimmer implements against"
  - phase: 26-02
    provides: "AgentRuntimeConfig and the HistoryTrimmerConfig sub-struct (enabled, reserve_for_response, default_context_tokens, model_context_limits, recall_limit) this plan's HistoryTrimmer consumes unmodified"
  - phase: 26-05
    provides: "The StopReason/ExecutionMiddleware built-in pattern (config sub-struct, enabled-gate-first before_model) HistoryTrimmer mirrors"
  - phase: 26-07
    provides: "GarrisonEntry.is_summary and GarrisonEntry::summary -- the field this plan's trimmer deliberately gives no special treatment"
  - phase: 26-10
    provides: "The ModelCallContext.llm_override/retry_policy port-shaping pattern and its documented assembly-order placement (resilience LAST), which is why HistoryTrimmer cannot see a per-run override and instead holds the service's own LlmPort"
provides:
  - "TokenCounterPort in paladin-ports: a synchronous, infallible fn count(&self, text: &str, model: &str) -> u32 / fn name(&self) -> &str trait -- the one counting path every budget feature in this phase uses"
  - "HeuristicTokenCounter in paladin-memory::token_counter -- chars/4 rounded up, ungated, the phase-wide default"
  - "impl TokenCounterPort for TiktokenCounter (paladin-memory::garrison, under the existing content-processing feature) -- delegates to the encoding already loaded at construction, so count() never re-resolves model and never errors for an unknown model string"
  - "HistoryTrimmer (ExecutionMiddleware, KeepSystemAndRecent, D-15) with D-14's three-step context-limit resolution logged at debug"
  - "PaladinExecutionService::with_token_counter/token_counter() (defaults to HeuristicTokenCounter) and with_recall_limit() (overrides the hard-coded recall_recent(20) only when explicitly set)"
affects: [26-15, 26-20]

tech-stack:
  added: []
  patterns:
    - "A budget-shaped ExecutionMiddleware holds its own Arc<dyn LlmPort> (the service's own, not a per-run override) when it needs provider capabilities the middleware chain itself cannot yet see, because port-shaping middleware (resilience) runs LATER in the documented assembly order"
    - "A local, minimal LlmPort test fixture (CapabilityOnlyLlmPort) implementing every method but only exercising get_capabilities(), used instead of MockLlmAdapter because the shared mock hardcodes max_context_tokens: Some(4096) with no builder override"

key-files:
  created:
    - crates/paladin-ports/src/output/token_counter_port.rs
    - crates/paladin-memory/src/token_counter/mod.rs
    - crates/paladin-memory/src/token_counter/heuristic.rs
    - src/application/services/paladin/middleware/history.rs
  modified:
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-memory/src/garrison/token_counter.rs
    - crates/paladin-memory/src/lib.rs
    - crates/paladin-memory/src/prelude.rs
    - src/application/services/paladin/middleware/mod.rs
    - src/application/services/paladin/paladin_execution_service.rs

key-decisions:
  - "HistoryTrimmer::new takes three constructor arguments (HistoryTrimmerConfig, Arc<dyn TokenCounterPort>, Arc<dyn LlmPort>), one more than the plan's literal two-arg text -- see Deviations"
  - "TiktokenCounter's impl TokenCounterPort::count ignores the model parameter and delegates to the encoding this instance already loaded (fallibly) at TiktokenCounter::new -- an unrecognised model string passed at count-time is never re-resolved, which is what makes the port method infallible without changing TiktokenCounter::new's existing fallible constructor"
  - "History admission stops at the FIRST entry (walking newest-to-oldest) that does not fit the remaining budget, rather than skipping it and trying older/smaller entries -- matches the plan's 'admitted newest-first while sum <= limit' phrasing as a contiguous prefix, not a best-fit packing"
  - "The 'fixed parts alone exceed the limit' degrade-to-empty-history trigger is implemented as fixed_tokens > (limit - reserve_for_response), i.e. the budget already accounts for the reserve -- the plan's CONTEXT.md prose says 'exceed the limit' but the general admission formula explicitly includes reserve_for_response in the same inequality, so treating the reserve as already spoken for is the internally consistent reading"
  - "recall_limit is a plain Option<u32> field on PaladinExecutionService, set independently via with_recall_limit() rather than auto-detected from the installed middleware chain -- ExecutionMiddleware has no Any/downcast capability (adding one would be a breaking change to a closed trait, D-01), so 'only when the trimmer is installed' is enforced by convention (a caller installing HistoryTrimmer also calls with_recall_limit with the same config value), to be wired together in plan 26-20's build_chain"

patterns-established:
  - "TokenCounterPort: a synchronous, infallible port for CPU-bound, purely-local computation, deliberately NOT async_trait/Result-shaped, mirroring GarrisonPort's small-port style rather than the general-purpose async ports elsewhere in paladin-ports"

requirements-completed: [RT-03]

coverage:
  - id: D1
    description: "TokenCounterPort exists in paladin-ports as a synchronous, infallible trait (no async fn, no Result<...>) with a passing doctest"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/token_counter_port.rs (doctest, line 47)"
        status: pass
    human_judgment: false
  - id: D2
    description: "HeuristicTokenCounter counts Unicode scalar values (chars), not bytes, rounds up, is infallible for any model string, and is deterministic across repetitions and fresh instances"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/token_counter/heuristic.rs#tests::heuristic_counts_chars_not_bytes, tests::heuristic_rounds_up, tests::heuristic_is_infallible_for_any_model_string, tests::heuristic_is_deterministic, tests::name_identifies_the_counter"
        status: pass
    human_judgment: false
  - id: D3
    description: "TiktokenCounter implements TokenCounterPort under the content-processing feature, its exact BPE count differs from the heuristic, and an unrecognised model string at count-time never errors or panics"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/garrison/token_counter.rs#tests::tiktoken_counter_implements_the_port, tests::tiktoken_counter_falls_back_inside_the_adapter_for_an_unknown_model, tests::tiktoken_name_identifies_the_counter"
        status: pass
    human_judgment: false
  - id: D4
    description: "The legacy TokenCounter trait, TokenCounterFactory, and the pre-existing rag_retrieval_service.rs inline /4 heuristic are provably untouched; all 12 pre-existing tests in token_counter.rs pass unmodified"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "git diff f6aff45d HEAD -- crates/paladin-memory/src/garrison/token_counter.rs (0 removed trait/get_bpe_from_model lines); git diff --name-only f6aff45d HEAD -- crates/paladin-memory/src/services/rag_retrieval_service.rs (0 lines changed); cargo test -p paladin-memory --all-features --lib token_counter (21/21 pass, including the 12 pre-existing tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "HistoryTrimmer's limit resolution follows the documented three-step order (model_context_limits -> provider capabilities -> default_context_tokens), with each step's precedence provable"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/history.rs#tests::limit_resolution_prefers_the_config_table, tests::limit_resolution_falls_back_to_provider_capabilities, tests::limit_resolution_falls_back_to_the_default"
        status: pass
    human_judgment: false
  - id: D6
    description: "History is admitted newest-first within budget, an entry is kept whole or dropped whole (never truncated), raising reserve_for_response drops exactly one more entry, fixed parts are always kept, an oversized-fixed-parts case degrades to an empty history without failing the run, the kept set is stable across 20 repetitions and a fresh counter instance, and a is_summary entry gets no special treatment"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/history.rs#tests::history_is_admitted_newest_first_until_the_budget, tests::an_entry_is_kept_whole_or_dropped_whole, tests::reserve_for_response_is_subtracted, tests::fixed_parts_are_always_kept, tests::oversized_fixed_parts_yield_an_empty_history_and_the_run_proceeds, tests::trimming_is_stable_across_repetitions_and_instances, tests::a_summary_entry_is_an_ordinary_entry_to_the_trimmer, tests::disabled_trimmer_changes_nothing, tests::history_trimmer_never_fails_the_run, tests::pushed_sections_count_toward_the_fixed_parts"
        status: pass
    human_judgment: false
  - id: D7
    description: "PaladinExecutionService gains with_token_counter/token_counter() defaulting to HeuristicTokenCounter, and with_recall_limit() which replaces the hard-coded recall_recent(20) only when explicitly set -- with no call, behavior is byte-identical to today"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#token_counter_and_recall_limit_tests::recall_limit_applies_only_when_the_trimmer_is_installed, token_counter_and_recall_limit_tests::token_counter_defaults_to_heuristic_and_with_token_counter_overrides_it"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 11: Context-Window Token Counting and History Trimming Summary

**A synchronous, infallible `TokenCounterPort` (heuristic default + tiktoken exact adapter) and a `HistoryTrimmer` middleware that resolves a model's context limit through a documented three-step order and trims Garrison history newest-first without ever splitting an entry or failing the run.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-09-07T07:23:28Z (first RED commit)
- **Completed:** 2026-09-07T07:52:20Z
- **Tasks:** 2
- **Files modified:** 10 (4 created, 6 modified)

## Accomplishments

- `TokenCounterPort` trait in `paladin-ports`: synchronous (`fn count`, `fn name`), infallible, with a passing doctest, and the single counting path both `HeuristicTokenCounter` and `TiktokenCounter` implement.
- `HeuristicTokenCounter` in `paladin-memory::token_counter`, ungated: `text.chars().count().div_ceil(4)`, counting Unicode scalar values (not bytes), deterministic, infallible for any model string.
- `impl TokenCounterPort for TiktokenCounter` under the existing `content-processing` feature, delegating to the already-loaded BPE encoding so `count()` never re-resolves `model` and never errors for an unrecognised model at count-time.
- `HistoryTrimmer` (`ExecutionMiddleware`): resolves the context-token limit via `model_context_limits[model] -> llm_port.get_capabilities().max_context_tokens -> default_context_tokens`, logging the resolved value and its source at debug; trims history newest-first (`KeepSystemAndRecent`), keeping an entry whole or dropping it whole, degrading to an empty history with a warning (never a failure) when the fixed prompt parts alone exceed the budget, and giving `is_summary` entries no special treatment.
- `PaladinExecutionService::with_token_counter`/`token_counter()` (defaults to `HeuristicTokenCounter`) and `with_recall_limit()` (overrides the hard-coded `recall_recent(20)` only when set).

## Task Commits

Each task followed RED-then-GREEN, per `tdd="true"`:

1. **Task 1: TokenCounterPort, the heuristic adapter, and TiktokenCounter's implementation**
   - `69a56dd6` test(26-11): add failing tests for TokenCounterPort adapters (RED)
   - `fd7854db` feat(26-11): implement TokenCounterPort for HeuristicTokenCounter and TiktokenCounter (GREEN)
2. **Task 2: Context-limit resolution and the HistoryTrimmer contract**
   - `d3dd1ebd` test(26-11): add failing tests for HistoryTrimmer's resolution and trimming contract (RED)
   - `0f86d5b5` test(26-11): add failing tests for the service's token_counter and recall_limit wiring (RED)
   - `e20f63e8` feat(26-11): implement HistoryTrimmer and wire token_counter/recall_limit into the service (GREEN)

**Plan metadata:** this commit (SUMMARY.md)

_Note: Task 2 split its RED phase across two commits (the middleware's own tests, then the service's wiring tests) before one unified GREEN commit implemented both together — the service wiring depends on the same `HistoryTrimmerConfig`/`TokenCounterPort` types the middleware's RED commit had already introduced, so a single GREEN commit for both kept the two halves of the feature consistent._

## Files Created/Modified

- `crates/paladin-ports/src/output/token_counter_port.rs` - `TokenCounterPort` trait (new)
- `crates/paladin-ports/src/output/mod.rs` - registers the `token_counter_port` module
- `crates/paladin-memory/src/token_counter/mod.rs` - `token_counter` module declaration (new)
- `crates/paladin-memory/src/token_counter/heuristic.rs` - `HeuristicTokenCounter` (new)
- `crates/paladin-memory/src/garrison/token_counter.rs` - adds `impl TokenCounterPort for TiktokenCounter` and its tests, leaves the legacy `TokenCounter` trait/`TokenCounterFactory`/`get_bpe_from_model` call sites untouched
- `crates/paladin-memory/src/lib.rs` - registers `token_counter` module, documents it in the crate-level feature table
- `crates/paladin-memory/src/prelude.rs` - re-exports `HeuristicTokenCounter`
- `src/application/services/paladin/middleware/history.rs` - `HistoryTrimmer` (new)
- `src/application/services/paladin/middleware/mod.rs` - registers and re-exports `HistoryTrimmer`
- `src/application/services/paladin/paladin_execution_service.rs` - adds `token_counter`/`recall_limit` fields, `with_token_counter`/`token_counter()`/`with_recall_limit()`, and switches `recall_recent(20)` to `recall_recent(self.recall_limit.unwrap_or(20))`

## Decisions Made

- `HistoryTrimmer::new` takes a third constructor argument, `Arc<dyn LlmPort>`, beyond the plan's literal two-argument text (`HistoryTrimmerConfig`, `Arc<dyn TokenCounterPort>`). D-14 requires resolving "the service port's `get_capabilities()`", but neither `ModelCallContext` nor `PromptAssembly` (both outside this plan's `files_modified` list) carries a reference to the service's own `LlmPort`, and `ModelCallContext::llm_override` is a per-run value set by resilience middleware LATER in the documented assembly order (plan 26-10), so it is never visible to `HistoryTrimmer`'s own `before_model` in the same iteration. The added parameter is the service's own configured default port, read once per `before_model` call — never a per-run override.
- `TiktokenCounter`'s `count()` ignores the `model` parameter and delegates to the encoding already loaded (fallibly) at `TiktokenCounter::new` — this is what makes the port method infallible without touching the existing fallible `get_bpe_from_model` call site the RESEARCH.md pitfall flags as a stop-and-flag item, not an action item.
- History admission stops at the first entry (walking newest-to-oldest) that doesn't fit the remaining budget, rather than skipping it to try an older, possibly-smaller entry. This reads "admitted newest-first while `sum <= limit`" as describing a contiguous kept prefix, matching Test 5's "the three newest are kept" wording.
- The oversized-fixed-parts trigger is `fixed_tokens > (limit - reserve_for_response)` rather than `fixed_tokens > limit`. CONTEXT.md's prose says "if the fixed parts alone exceed the limit", but the general admission inequality (`counted(fixed) + Σ counted(kept) + reserve_for_response <= limit`) already folds the reserve in — treating the reserve as already spoken for is the internally consistent reading, and the RED test (`oversized_fixed_parts_yield_an_empty_history_and_the_run_proceeds`) uses fixed parts that clearly exceed the raw limit too, so the test is correct under either reading.
- `recall_limit` is a plain `Option<u32>` field, set independently via `with_recall_limit()`, rather than the service auto-detecting "a `HistoryTrimmer` is installed" by inspecting the `Vec<Arc<dyn ExecutionMiddleware>>` chain. `ExecutionMiddleware` has no `Any`/downcast capability, and adding one would be a breaking change to a trait explicitly documented as closed (D-01, PRD 05's three-hook surface). The two settings (installing the trimmer, setting `recall_limit`) are meant to be wired together from the same `HistoryTrimmerConfig` value in plan 26-20's `AgentRuntimeConfig::build_chain`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] `HistoryTrimmer::new` gained a third constructor parameter**
- **Found during:** Task 2 (HistoryTrimmer implementation)
- **Issue:** The plan's literal text specifies `HistoryTrimmer::new(HistoryTrimmerConfig, Arc<dyn TokenCounterPort>)`, but D-14's resolution order requires consulting "the service port's `get_capabilities()`" — no type reachable from `before_model` (within this plan's `files_modified` list) carries a reference to an `LlmPort`.
- **Fix:** Added a third parameter, `llm_port: Arc<dyn LlmPort>`, representing the service's own configured port at construction time.
- **Files modified:** `src/application/services/paladin/middleware/history.rs`
- **Verification:** `limit_resolution_falls_back_to_provider_capabilities` and `limit_resolution_falls_back_to_the_default` exercise both branches; `cargo test -p paladin-ai --lib middleware::history::` (13/13 pass).
- **Committed in:** `e20f63e8` (Task 2 GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical functionality)
**Impact on plan:** Necessary for the plan's own stated D-14 resolution contract to be implementable at all; no scope creep beyond adding the one parameter the contract requires.

## Issues Encountered

- `MockLlmAdapter` (in `paladin-llm`, outside this plan's `files_modified` list) hardcodes `get_capabilities().max_context_tokens: Some(4096)` with no builder to override it, so it could not exercise D-14's provider-capabilities and default-fallback branches. Resolved by writing a minimal `CapabilityOnlyLlmPort` test fixture local to `history.rs`'s own test module, implementing the full `LlmPort` trait but exercising only `get_capabilities()` — no change to `paladin-llm` was needed or made.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `TokenCounterPort` and both adapters are available for plan 26-15's `SummarizationMiddleware`, which the CONTEXT.md decisions (D-16) describe as consuming the same counting path.
- `HistoryTrimmer` and the service's `token_counter()`/`with_recall_limit()` are ready for plan 26-20's `AgentRuntimeConfig::build_chain` to wire together from one `HistoryTrimmerConfig` value (installing the trimmer AND setting the matching `recall_limit` in the same place).
- No blockers. The `recall_limit`-only-when-installed contract currently relies on caller discipline (both settings driven from the same config value) rather than structural enforcement — flagged above as a decision, not a defect, since enforcing it structurally would require a breaking change to the closed `ExecutionMiddleware` trait.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

- All 5 created/referenced files verified present on disk.
- All 5 commit hashes verified present in `git log --oneline --all`.
