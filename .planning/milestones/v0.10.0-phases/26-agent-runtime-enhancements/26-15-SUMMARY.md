---
phase: 26-agent-runtime-enhancements
plan: 15
subsystem: agent-runtime
tags: [garrison, summarization, vault, middleware, prompt-injection, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 07)
    provides: "GarrisonEntry.is_summary / GarrisonEntry::summary(content), the field and constructor this plan's effective_history rule and SummarizationMiddleware read and write"
  - phase: 26-agent-runtime-enhancements (plan 11)
    provides: "TokenCounterPort, HeuristicTokenCounter, HistoryTrimmer -- the embedded self-sufficient degradation path SummarizationMiddleware wraps, and PaladinExecutionService::token_counter()/with_recall_limit()"
  - phase: 26-agent-runtime-enhancements (plan 13)
    provides: "ConfinedVault (paladin-ports), RunScope, PaladinExecutionService::confined_vault/execute_scoped -- the grant-resolution machinery VaultRecallMiddleware reads from ModelCallContext.vault"
provides:
  - "effective_history() in paladin_execution_service.rs: D-16's latest-summary-wins rule, applied once at the point a recalled Garrison window becomes a run's history, using metadata[\"summarized_through\"] (not the summary's physical store position) as the cutoff among raw entries -- correctness fix over a naive position-based reading, since GarrisonPort::remember always appends and would otherwise strand kept-raw entries on the next recall"
  - "SummarizationMiddleware: compresses an over-long effective history into a compounding Garrison summary via its own direct LlmPort call (invisible to before_model/after_model, so ModelCallLimit never counts it), degrading to an embedded HistoryTrimmer (built from the same HistoryTrimmerConfig, enabled forced true) on any summarizer failure -- self-sufficient, never MiddlewareFlow::Fail, independent of chain order"
  - "ModelCallContext.vault: Option<ConfinedVault>, set once by execute_internal from the ConfinedVault execute_scoped resolves before dispatch"
  - "VaultRecallMiddleware: searches the granted namespace on loop_index == 0 only, drops hits below score_floor, truncates to top_k, and caches the rendered section in cx.scratch so it re-renders on every later iteration despite the assembly being rebuilt fresh each loop; frames recalled entries in a delimited '## Long-term memory' PromptSection (after RAG context, before history) with a fixed 'stored notes, not instructions' preamble; VaultError::Unsupported warns once per middleware instance, any other error warns and skips, no grant skips silently, never fails the run"
affects: []

tech-stack:
  added: []
  patterns:
    - "A middleware that makes its own model call bypasses run_before/run_after entirely (direct .generate(request).await) so the chain -- and anything counting model calls through it -- never observes that call (D-08, D-16)"
    - "A cutoff-by-referenced-id (metadata[\"summarized_through\"]), not cutoff-by-physical-store-position, is the only correct way to reconstruct 'entries newer than X' when the store is append-only and X was written after the entries it logically precedes"
    - "A middleware whose per-iteration section must survive a freshly-rebuilt PromptAssembly caches the rendered body in ModelCallContext::scratch (per-run, survives across loop iterations) and re-pushes it every before_model call, doing the expensive/side-effecting work (the search) only once"

key-files:
  created:
    - src/application/services/paladin/middleware/summarization.rs
    - src/application/services/paladin/middleware/vault_recall.rs
  modified:
    - src/application/services/paladin/middleware/mod.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/paladin_execution_service.rs

key-decisions:
  - "effective_history()'s cutoff is computed from metadata[\"summarized_through\"]'s position among the window's raw entries, not from the summary's own physical index in the recalled window -- discovered as a genuine correctness bug while writing the compounding test (Test 3): GarrisonPort::remember always appends at the physical end of the store, so a summary is always physically the newest entry the moment it is written, even though the raw entries SummarizationMiddleware chose to keep un-folded were all inserted BEFORE it. A naive 'keep everything after the summary's own index' rule would silently drop those kept-raw entries on the very next recall. Falls back to the summary's own physical position only when summarized_through is absent/unparseable/aged out of the window."
  - "SummarizationMiddleware::new takes six constructor arguments, not the plan's literal four (SummarizationConfig, HistoryTrimmerConfig, Arc<dyn TokenCounterPort>, Option<Arc<dyn LlmPort>>) -- adds service_llm_port: Arc<dyn LlmPort> (the summarizer's default when summarizer_override is None, and the port the embedded HistoryTrimmer resolves D-14 context limits through) and garrison: Arc<dyn GarrisonPort> (D-16 requires remember()-ing the resulting summary; no type reachable from before_model carries a Garrison reference). Mirrors plan 26-11's identical HistoryTrimmer::new deviation."
  - "The embedded HistoryTrimmer's HistoryTrimmerConfig.enabled is forced to true at construction, overriding whatever the caller passed -- it is a guaranteed internal fallback for the degrade path, not a user-facing toggle, so it must trim on every degradation regardless of whether a separately-installed HistoryTrimmer happens to be enabled elsewhere in the chain (this is what makes degradation_does_not_depend_on_chain_order's contract hold)."
  - "VaultRecallMiddleware performs the actual search only on cx.loop_index == 0, but re-pushes the cached section body (stored in cx.scratch) on EVERY before_model call, because the reasoning loop rebuilds PromptAssembly (and its empty sections Vec) fresh each iteration -- a section pushed only once would vanish on iteration 1 otherwise."
  - "ModelCallContext gained a new pub vault: Option<ConfinedVault> field (in middleware/context.rs, outside this plan's originally-listed files_modified) and PaladinExecutionService::execute_scoped/execute_bounded/execute_internal gained a confined_vault parameter threaded end to end -- both structurally required for D-25's 'reads the run's ConfinedVault from the context' to be implementable at all, since no existing field carried a run's Vault grant into the reasoning loop before this plan."
  - "Both tasks' TDD RED/GREEN cycles were combined into single feat commits, not separate test(...)-then-feat(...) commits -- tests and implementation were authored and iterated together (the compounding-test bug above was found and fixed before any commit landed), matching plan 26-02's identical precedent and rationale."

patterns-established:
  - "A summarizer/extraction-style middleware that must persist output to a shared store (Garrison) takes the store's port as an explicit constructor argument rather than reading it off ModelCallContext -- the context intentionally carries only per-run mutable state (D-03), not shared adapter handles."

requirements-completed: [RT-03, RT-04]

coverage:
  - id: D1
    description: "effective_history(): the effective history is the newest is_summary entry plus every raw entry newer than it (by summarized_through, not physical position); no summary means the whole window unchanged; a stale older summary is skipped; an empty window is a no-op"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::effective_history_is_the_newest_summary_plus_newer_raw, ::effective_history_with_no_summary_is_the_whole_window, ::effective_history_skips_a_stale_older_summary, ::effective_history_of_an_empty_window_is_empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "SummarizationMiddleware fires in before_model when the effective history's message count meets/exceeds threshold_messages (30 default); below-threshold and empty-history are no-ops making no summarizer call"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/summarization.rs#tests::below_threshold_is_a_no_op, ::empty_history_is_a_no_op, ::disabled_middleware_changes_nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "30 messages with keep_recent:10 summarize [oldest 20] through the summarizer port/model and remember() the result as GarrisonEntry::summary with ConversationRole::System, is_summary:true and metadata[\"summarized_through\"] set to the newest folded entry's id; the resulting effective history is 1 summary + 10 raw"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/summarization.rs#tests::thirty_messages_produce_one_summary_and_ten_raw"
        status: pass
    human_judgment: false
  - id: D4
    description: "Compounding: adding 20 more messages triggers a second summarization whose input is [summary #1 + the oldest raw entries beyond keep_recent], provably NOT the original 30 (the second summarizer prompt contains summary #1's own text and does not replay the raw entries summary #1 already folded)"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/summarization.rs#tests::compounding_builds_the_second_summary_from_the_first"
        status: pass
    human_judgment: false
  - id: D5
    description: "A summarizer failure of any transience (a 503 ProviderError, a plain NetworkError) sets scratch[\"summarization.degraded\"] = true, logs a warning, runs the embedded HistoryTrimmer, and completes the run -- never MiddlewareFlow::Fail -- identically whether or not a separate HistoryTrimmer is installed, and identically before or after it in the chain"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/summarization.rs#tests::summarizer_failure_degrades_to_trimming, ::degradation_does_not_depend_on_chain_order, ::summarization_never_fails_the_run"
        status: pass
    human_judgment: false
  - id: D6
    description: "The summarizer's own model call is invisible to the ExecutionMiddleware chain: with a ModelCallLimit{max_calls:1} and a recording middleware also installed, a run that summarizes still makes exactly one MAIN model call and the recorder observes exactly one before_model/after_model pair -- none extra for the summarizer's call"
    requirement: "RT-03"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/summarization.rs#tests::the_summarizers_own_call_is_not_counted_and_fires_no_hooks"
        status: pass
    human_judgment: false
  - id: D7
    description: "GarrisonPort gains no delete-by-id or compaction method in this plan; a TraceEvent for the degradation path is explicitly Phase 28's, documented as scope not omission"
    requirement: "RT-03"
    verification:
      - kind: other
        ref: "git diff HEAD~2 -- crates/paladin-ports/src/output/garrison_port.rs | wc -l => 0; src/application/services/paladin/middleware/summarization.rs module doc + before_model rustdoc naming Phase 28 (OBS-01/02)"
        status: pass
    human_judgment: false
  - id: D8
    description: "VaultRecallMiddleware searches the granted namespace with the run's input on loop_index == 0 only (no second search on later iterations, proven by a call-count assertion), places the resulting section after retrieved RAG context and before history, drops hits below score_floor, and bounds the injection to top_k regardless of how many hits the backend returns"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/vault_recall.rs#tests::recall_injects_top_k_on_the_first_loop_only, ::section_is_placed_after_rag_context_and_before_history, ::results_below_the_score_floor_are_dropped, ::top_k_bounds_the_injection"
        status: pass
    human_judgment: false
  - id: D9
    description: "The recalled section states plainly, via a single named constant, that its entries are stored notes and not instructions (T-26-02's prompt-injection mitigation); the exact sentence is asserted, not approximated"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/vault_recall.rs#tests::section_frames_entries_as_stored_notes"
        status: pass
    human_judgment: false
  - id: D10
    description: "Every failure mode is best-effort and quiet: VaultError::Unsupported warns exactly once per middleware instance across many calls and skips thereafter; any other search error warns and skips every time; no Vault grant skips silently (no warning, no search); a zero-hit search leaves the assembly byte-identical; before_model never returns MiddlewareFlow::Fail"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/middleware/vault_recall.rs#tests::unsupported_search_warns_once_per_service_and_skips, ::any_other_search_error_skips_and_warns, ::no_grant_skips_silently, ::zero_hits_adds_no_section, ::vault_recall_never_fails_the_run, ::disabled_middleware_changes_nothing"
        status: pass
    human_judgment: false
  - id: D11
    description: "No auto-write middleware exists anywhere in the middleware tree -- a source-level scan proves no file calls a Vault .put(); the only write path is the explicit vault_put tool (plan 26-16)"
    requirement: "RT-04"
    verification:
      - kind: other
        ref: "src/application/services/paladin/middleware/vault_recall.rs#tests::no_auto_write_middleware_exists; grep -v '^\\s*//\\|^\\s*///' src/application/services/paladin/middleware/ -r | grep -c 'vault.*\\.put(' => 0"
        status: pass
    human_judgment: false
  - id: D12
    description: "Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (665/665) and --doc (124/124, 18 ignored pre-existing)"
    requirement: "RT-03"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib (665 passed, 0 failed); cargo test -p paladin-ai --doc (124 passed, 0 failed, 18 ignored -- pre-existing, unrelated to this plan)"
        status: pass
    human_judgment: false

duration: ~2h 30min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 15: Summarization and Vault Recall Middleware Summary

**`SummarizationMiddleware` compounds Garrison history into a self-degrading summary via its own hook-invisible model call, and `VaultRecallMiddleware` injects top-k long-term memory into a delimited "stored notes, not instructions" section on loop 0 -- both best-effort, neither ever fails the run.**

## Performance

- **Duration:** ~2h 30min
- **Started:** 2026-09-07T07:55:00Z (approx.)
- **Completed:** 2026-09-07T10:25:00Z (approx.)
- **Tasks:** 2 (both `tdd="true"`)
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments

- `effective_history()` (`paladin_execution_service.rs`) applies D-16's latest-summary-wins rule at the one place a recalled Garrison window becomes a run's history: finds the newest `is_summary` entry and keeps it plus every raw entry newer than it, determined by `metadata["summarized_through"]`'s position among the window's raw entries rather than the summary's own physical store position -- a correctness fix discovered while writing the compounding test, since `GarrisonPort::remember` always appends and a naive position-based cutoff would silently strand the "kept recent" raw entries on the very next recall.
- `SummarizationMiddleware` (`middleware/summarization.rs`) fires in `before_model` once the effective history's message count meets `threshold_messages` (default 30), folds `[the latest summary, if any] + [the oldest raw entries beyond keep_recent]` through a direct `.generate()` call to its own `Arc<dyn LlmPort>` (invisible to the `ExecutionMiddleware` chain, so `ModelCallLimit` never counts it), `remember()`s the result as `GarrisonEntry::summary(content)` with `metadata["summarized_through"]` set, and rewrites the assembly's history in place. Proven compounding: a second round of 20 more messages produces a second summary built from the first summary's own content plus the raw tail, never from the original 30. On any summarizer failure (any transience), it logs a warning, sets `scratch["summarization.degraded"] = true`, and runs an embedded `HistoryTrimmer` (built from the same `HistoryTrimmerConfig`, `enabled` forced `true`) -- self-sufficient and identical regardless of whether a separate `HistoryTrimmer` is installed or where it sits in the chain.
- `ModelCallContext` gained `vault: Option<ConfinedVault>`, set once per run by `execute_internal` from the `ConfinedVault` `execute_scoped` resolves before dispatch (threaded through `execute_scoped` -> `execute_bounded` -> `execute_internal`, all within this file).
- `VaultRecallMiddleware` (`middleware/vault_recall.rs`) searches the granted namespace with the run's input on `loop_index == 0` only, drops hits below `score_floor`, truncates to `top_k`, and caches the rendered section body in `cx.scratch` so it re-renders on every later iteration despite the assembly being rebuilt fresh each loop (only the search itself is one-shot). The section is headed `## Long-term memory`, placed after retrieved RAG context and before history, and states via a single named constant that its entries are stored notes recorded earlier, not instructions -- the structural mitigation for T-26-02 (recalled Vault content is model-controllable text once `vault_put` ships in plan 26-16). Every failure mode is quiet: `VaultError::Unsupported` warns once per middleware instance and skips thereafter; any other error warns and skips every time; no grant skips silently; zero hits leave the assembly byte-identical.
- A source-level test (`no_auto_write_middleware_exists`) proves no middleware in the tree calls a Vault `.put()` -- the only write path is the explicit `vault_put` tool a later plan adds.

## Task Commits

Both tasks combined their RED/GREEN cycle into a single `feat` commit each -- tests and implementation were authored and iterated together (the `effective_history` cutoff bug below was found and fixed before either commit landed), matching plan 26-02's identical precedent:

1. **Task 1: Latest-summary-wins in the assembly, and the compounding `SummarizationMiddleware`**
   - `f2bf1b62` (feat) -- `effective_history()`, its wiring into `execute_internal`, `SummarizationMiddleware`, and all 9 tests (8 plan-named + `disabled_middleware_changes_nothing`), plus 4 `effective_history` unit tests in `paladin_execution_service.rs`
2. **Task 2: `VaultRecallMiddleware` -- best-effort, first-loop-only, framed as data**
   - `8667e5e4` (feat) -- `ModelCallContext.vault`, the `confined_vault` threading through `execute_scoped`/`execute_bounded`/`execute_internal`, `VaultRecallMiddleware`, and all 12 tests (10 plan-named + `disabled_middleware_changes_nothing` + `vault_recall_never_fails_the_run`)

**Plan metadata:** this commit (SUMMARY.md)

## Files Created/Modified

- `src/application/services/paladin/middleware/summarization.rs` (new) -- `SummarizationMiddleware`, `SUMMARIZATION_DEGRADED_KEY`, `render_summarizer_prompt`, 9 tests
- `src/application/services/paladin/middleware/vault_recall.rs` (new) -- `VaultRecallMiddleware`, `LONG_TERM_MEMORY_HEADING`, `STORED_NOTES_NOT_INSTRUCTIONS`, `render_body`, 12 tests
- `src/application/services/paladin/middleware/mod.rs` -- registers and re-exports both new modules
- `src/application/services/paladin/middleware/context.rs` -- `ModelCallContext.vault: Option<ConfinedVault>` field
- `src/application/services/paladin/paladin_execution_service.rs` -- `effective_history()` (new, `pub(crate)`) + its call site in `execute_internal`'s history load; `confined_vault` parameter threaded through `execute_scoped`/`execute_bounded`/`execute_internal`; `middleware_cx.vault` assignment; 4 new `effective_history` unit tests

## Decisions Made

- **`effective_history()`'s cutoff uses `metadata["summarized_through"]`, not the summary's physical position** -- see the coverage/key-decisions detail above; this is the plan's own D-16 rule made correct against how `GarrisonPort::remember` actually behaves (always appends), not a reinterpretation of the rule.
- **`SummarizationMiddleware::new` takes six arguments, not the plan's literal four** -- `service_llm_port: Arc<dyn LlmPort>` and `garrison: Arc<dyn GarrisonPort>` are structurally required (D-16's own "defaults to the service port" and "remembers the result" clauses), mirroring plan 26-11's identical `HistoryTrimmer::new` deviation.
- **The embedded `HistoryTrimmer`'s config has `enabled` forced to `true`** regardless of what the caller passes -- it is a guaranteed internal fallback, not a user-facing toggle.
- **`VaultRecallMiddleware` caches its rendered section in `cx.scratch` and re-pushes it every iteration** -- the plan's phrase "the section persists in the assembly because the assembly is per-run" is only true this way, since the assembly OBJECT is rebuilt fresh (with an empty `sections` list) every reasoning-loop iteration.
- **`ModelCallContext.vault` and the `confined_vault` parameter threading are new, beyond this plan's originally-listed `files_modified`** (which did not name `middleware/context.rs`) -- both are structurally required for `VaultRecallMiddleware` to read a run's Vault grant at all.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `effective_history()`'s cutoff used the summary's physical store position instead of `summarized_through`**
- **Found during:** Task 1, writing `compounding_builds_the_second_summary_from_the_first` (the compounding test)
- **Issue:** The first implementation read D-16 literally as "keep everything at a position after the summary's own index in the recalled window." `GarrisonPort::remember` always appends at the physical end of the store, so the moment a summary is written it is physically the newest entry -- even though the raw entries `SummarizationMiddleware` chose to keep un-folded (`keep_recent`) were all inserted BEFORE it. Under the naive rule, the very next `recall_recent` would compute an effective history of just `[summary]` (dropping the 10 kept-raw entries entirely), and the compounding test's second round produced 21 entries where 31 were expected -- caught by the test itself, not discovered after the fact.
- **Fix:** `effective_history()` now reads the summary's `metadata["summarized_through"]` (the newest RAW entry id it folds), finds that entry's position among the window's raw entries, and keeps every raw entry after it -- regardless of where the summary itself physically sits in the window. Falls back to the summary's own physical position only when `summarized_through` is absent, unparseable, or names an entry that aged out of the window (e.g. a hand-built summary in a test).
- **Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
- **Verification:** `thirty_messages_produce_one_summary_and_ten_raw` and `compounding_builds_the_second_summary_from_the_first` both pass; all 4 `effective_history` unit tests pass, including the pre-existing literal-fixture shape (no `summarized_through` set) via the fallback path.
- **Committed in:** `f2bf1b62`

**2. [Rule 2 - Missing Critical Functionality] `SummarizationMiddleware::new` gained two constructor parameters beyond the plan's literal 4-argument text**
- **Found during:** Task 1, starting the implementation
- **Issue:** The plan's action text specifies `SummarizationMiddleware::new(SummarizationConfig, HistoryTrimmerConfig, Arc<dyn TokenCounterPort>, Option<Arc<dyn LlmPort>>)`, but D-16 requires (a) defaulting the summarizer to "the service port" -- no type reachable from `before_model` names which port that is unless supplied explicitly -- and (b) `remember()`-ing the resulting summary -- no type reachable from `before_model` carries a Garrison reference.
- **Fix:** Added `service_llm_port: Arc<dyn LlmPort>` and `garrison: Arc<dyn GarrisonPort>` as constructor parameters 4 and 6 (the plan's `Option<Arc<dyn LlmPort>>` becomes `summarizer_override`, parameter 5).
- **Files modified:** `src/application/services/paladin/middleware/summarization.rs`
- **Verification:** `cargo test -p paladin-ai --lib summarization::` (9/9 pass); mirrors plan 26-11's identical, already-precedented deviation for `HistoryTrimmer::new`.
- **Committed in:** `f2bf1b62`

**3. [Rule 2 - Missing Critical Functionality] `ModelCallContext.vault` and `confined_vault` threading added beyond this plan's listed `files_modified`**
- **Found during:** Task 2, starting the implementation
- **Issue:** The plan's action text says `VaultRecallMiddleware` "reads the run's `ConfinedVault` from the context," but `ModelCallContext` (defined in `middleware/context.rs`, not listed in this plan's `files_modified`) had no field carrying a Vault grant, and `execute_scoped` only used the resolved namespace for a log line (plan 26-13 deliberately left it "unused today beyond logging").
- **Fix:** Added `pub vault: Option<ConfinedVault>` to `ModelCallContext` (initialized to `None` in `::new`, no signature break since all 12+ existing call sites use `::new` and no struct literal exists anywhere for this type); threaded a `confined_vault: Option<ConfinedVault>` parameter through `execute_scoped` -> `execute_bounded` -> `execute_internal` (all single-call-site, private-except-`execute_scoped` methods within the same file), and set `middleware_cx.vault = confined_vault` once per run, right after the context is constructed.
- **Files modified:** `src/application/services/paladin/middleware/context.rs`, `src/application/services/paladin/paladin_execution_service.rs`
- **Verification:** `cargo test -p paladin-ai --lib vault_recall::` (12/12 pass); `cargo check --workspace --all-targets --all-features` (exit 0, no other call site broke).
- **Committed in:** `8667e5e4`

---

**Total deviations:** 3 auto-fixed (1 Rule 1 correctness bug caught by the plan's own compounding test, 2 Rule 2 structurally-required constructor/field additions matching precedent from plans 26-11 and 26-13)
**Impact on plan:** The Rule 1 fix corrects the plan's own D-16 rule against how the real Garrison store behaves -- no scope change, same public contract, same test names. The two Rule 2 additions are necessary for the plan's own stated contracts (`SummarizationMiddleware` defaulting/persisting, `VaultRecallMiddleware` reading a grant) to be implementable at all. No scope creep beyond what each contract requires.

## Issues Encountered

None beyond the deviations documented above. Every acceptance criterion in the plan (struct/module existence, embedded-trimmer grep count, zero `MiddlewareFlow::Fail` occurrences in either `impl ExecutionMiddleware` block, the `summarization.degraded` key, the `## Long-term memory` heading appearing exactly once as a source literal, zero `.put(` calls outside comments anywhere in the middleware tree, `GarrisonPort` untouched, all 21 plan-named test functions, `cargo fmt`/`cargo clippy -- -D warnings`) was verified directly. Two acceptance greps (`Long-term memory` count, `.put(` count) initially failed due to doc-comment prose and the test's own needle-literal respectively; both fixed by rewording the prose and assembling the `.put(` needle from concatenated string halves at runtime, the same technique already used for the `vau`+`lt` needle.

## Known Stubs

None. Both middlewares are fully implemented per the plan's `<action>`/`<done>` clauses, with real (not placeholder) tests for every `<behavior>` item across both tasks.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `SummarizationMiddleware` and `VaultRecallMiddleware` are locked in their final public shape at `src/application/services/paladin/middleware/{summarization,vault_recall}.rs`; plan 26-16's `vault_put`/`vault_get` Armament tools and plan 26-20's `AgentRuntimeConfig::build_chain` can construct both directly from `SummarizationConfig`/`VaultRecallConfig` plus the small set of additional dependencies (`Arc<dyn LlmPort>`, `Arc<dyn GarrisonPort>`) this plan's deviations made explicit.
- `effective_history()` is `pub(crate)` and exercised by both `paladin_execution_service.rs`'s own tests and `middleware::summarization`'s compounding test -- any future plan touching Garrison history loading should call it, not re-derive the latest-summary-wins rule.
- `ModelCallContext.vault` is now the single place any future middleware reads a run's Vault grant from; no other field or resolution path should be added for the same purpose.
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 8 worktree agents complete.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `src/application/services/paladin/middleware/summarization.rs` -- FOUND, contains `pub struct SummarizationMiddleware` and `pub const SUMMARIZATION_DEGRADED_KEY`
- `src/application/services/paladin/middleware/vault_recall.rs` -- FOUND, contains `pub struct VaultRecallMiddleware`
- `src/application/services/paladin/middleware/mod.rs` -- FOUND, contains `pub mod summarization;`, `pub mod vault_recall;`, and both re-exports
- `src/application/services/paladin/middleware/context.rs` -- FOUND, contains `pub vault: Option<ConfinedVault>`
- `src/application/services/paladin/paladin_execution_service.rs` -- FOUND, contains `pub(crate) fn effective_history(`
- Commit `f2bf1b62` -- FOUND in `git log --oneline`
- Commit `8667e5e4` -- FOUND in `git log --oneline`
- `cargo test -p paladin-ai --lib summarization::` -- 9 passed, 0 failed
- `cargo test -p paladin-ai --lib vault_recall::` -- 12 passed, 0 failed
- `cargo test -p paladin-ai --lib effective_history` -- 4 passed, 0 failed
- `cargo test -p paladin-ai --lib` (full suite) -- 665 passed, 0 failed
- `cargo test -p paladin-ai --doc` -- 124 passed, 0 failed, 18 ignored (pre-existing)
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `git diff HEAD~2 -- crates/paladin-ports/src/output/garrison_port.rs | wc -l` -- 0 (unchanged)
