---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 08
subsystem: docs
tags: [examples-gallery, agent-runtime, execution-middleware, structured-output, schemars, rag-retrieval, sanctum, commissary]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md (house example shape -- header/README pair convention, mock-adapter/no-external-service offline-first pattern)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-06-SUMMARY.md (worktree-serialization scheduling precedent for build-heavy plans, not a code dependency)
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec4 (EX-83..EX-90, EX-116..EX-120 capability rows)
  - phase: 33-token-economy-rag-rationing
    provides: RagRetrievalService/RagRetrievalResult/ShedItem/RagRetrievalError/retrieve_context_with_timeout (D-12) and with_token_counter (D-07) this plan's Task 3 demonstrates
provides:
  - examples/agent_runtime_middleware.rs -- a custom ExecutionMiddleware's before_model/
    after_model/around_tool hooks, AgentRuntimeConfig::build_chain resolving built-in
    middleware from configuration, a custom TokenCounterPort vs. the defaulted heuristic,
    HistoryTrimmer + SummarizationMiddleware reducing a long history, ConfinedVault
    namespacing two agents' memories with a denied cross-namespace read, and the
    fail-run tool error mode producing a structured PaladinError::ArmamentFailed
  - examples/structured_output_schema.rs -- a JSON Schema derived from a Rust type via
    schemars::schema_for!, a conforming structured-execution response returning a typed
    value, and a schema-violating response rejected via the typed
    PaladinError::StructuredOutputInvalid
  - examples/sanctum_rag_retrieval.rs -- a runnable sibling to the conceptual
    paladin_with_rag.rs: RagRetrievalResult/ShedItem read back from a real rationed
    retrieval, a typed RagRetrievalError::BudgetTooLarge, the timeout-bounded
    retrieve_context_with_timeout free function, and an exact TokenCounterPort injected
    via with_token_counter
  - 36-evidence/36-08-examples.txt -- run output, acceptance-criteria greps, the D-02
    post-audit drift table for the five RAG capability tokens, and the D-24 closure
    table for all thirteen EX IDs this plan closes
affects: [36-11]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A capability cluster with no existing runnable program gets one dedicated,
      numbered-parts example whose stdout narrates each capability in the order the
      audit row lists it -- proven a fourth time (agent-runtime middleware),
      fifth time (structured output) and sixth time (RAG retrieval) on top of
      36-01/36-06/36-07's precedent (D-15)."
    - "InMemorySanctum's storage is a HashMap, so its iteration order (and therefore
      which of several score-tied search results comes first) is randomized per
      process, not per program run's inputs -- a RAG example that seeds several
      memories at identical similarity needs a deterministic, content-derived
      embedding (not one constant vector) to get a reproducible ranking across
      repeated runs."
    - "PaladinError::ArmamentFailed's own doc comment promises a redacted `reason`
      under ToolErrorMode::FailRun, matching ToolResultFormatter::format_error's
      sanitization under FeedToModel -- but the FailRun call sites in
      paladin_execution_service.rs never route through that sanitizer. Running the
      fail-run demo with a credential-shaped tool error and reading the printed
      `reason` field verbatim is what surfaced this; recorded as a discovered
      library defect (WINDOWS.md #38), not fixed here (D-18 docs-only boundary)."

key-files:
  created:
    - examples/agent_runtime_middleware.rs
    - examples/structured_output_schema.rs
    - examples/sanctum_rag_retrieval.rs
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-08-examples.txt
  modified: []

key-decisions:
  - "Split the agent-runtime cluster's six EX rows and the structured-output cluster's
    two EX rows into two separate programs (36-06/36-07/36-01 precedent for D-15's
    per-planner discretion) so structured output reads as its own self-contained
    story rather than a seventh part bolted onto the middleware program."
  - "The RAG cluster's five EX rows became a sibling program
    (sanctum_rag_retrieval.rs) rather than a rewrite of the existing
    paladin_with_rag.rs conceptual walkthrough, per D-18 and the plan's own stated
    D-15 discretion -- paladin_with_rag.rs stays untouched, verified by
    `git status --porcelain` on every commit."
  - "HistoryTrimmer's model-context-limit resolution tries the config table BEFORE
    the provider's own declared capabilities (MockLlmAdapter reports 4096 tokens) --
    the standalone HistoryTrimmer demo therefore names the Paladin's model
    explicitly and overrides `model_context_limits` for that exact name, rather than
    relying on `default_context_tokens` alone, which the provider capability would
    otherwise shadow."
  - "sanctum_rag_retrieval.rs's embedder derives each memory's vector from a
    fixed-key content hash (`DefaultHasher::new()`, not `HashMap`'s randomized
    `RandomState`) rather than returning one constant vector for every text --
    discovered necessary after two runs of the program printed different retained
    memories, traced to `InMemorySanctum`'s `HashMap`-backed storage iteration order
    being randomized per process when every search result ties at the same score."
  - "Recorded, not fixed: PaladinError::ArmamentFailed's FailRun arms in
    paladin_execution_service.rs build `reason` from raw `e.to_string()`, bypassing
    ToolResultFormatter's redact-then-bound sanitizer their own doc comment promises.
    Out of scope for this docs-only phase (D-18: no src/ or crates/ changes) --
    recorded in this SUMMARY, the evidence file, and WINDOWS.md entry #38."

requirements-completed: [CURR-13, CURR-14, CURR-15]

coverage:
  - id: D1
    description: "agent_runtime_middleware.rs demonstrates a custom ExecutionMiddleware's three hooks firing in order, AgentRuntimeConfig::build_chain resolving built-in middleware from configuration, a custom TokenCounterPort contrasted with the defaulted heuristic, HistoryTrimmer + SummarizationMiddleware reducing a long history, ConfinedVault namespacing with a denied cross-namespace read, and the fail-run tool error mode's structured PaladinError::ArmamentFailed (EX-83, EX-84, EX-85, EX-86, EX-87, EX-89)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example agent_runtime_middleware (exit 0); stdout inspected for all six capability markers"
        status: pass
    human_judgment: false
  - id: D2
    description: "structured_output_schema.rs demonstrates a JSON Schema derived from a Rust type via schemars::schema_for! and a typed value returned by the structured-execution path, plus a schema-violating response rejected via PaladinError::StructuredOutputInvalid rather than silently accepted (EX-88, EX-90)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example structured_output_schema (exit 0); stdout inspected for the derived schema, the typed value's fields, and the typed rejection"
        status: pass
    human_judgment: false
  - id: D3
    description: "sanctum_rag_retrieval.rs demonstrates a retrieval result's typed context, its ShedItem shed records, a typed RagRetrievalError from a deliberately failing retrieval, the timeout-bounded retrieve_context_with_timeout free function, and an exact TokenCounterPort injected via with_token_counter, as a runnable sibling that leaves the existing paladin_with_rag.rs untouched (EX-116, EX-117, EX-118, EX-119, EX-120)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example sanctum_rag_retrieval (exit 0); stdout inspected for all five capability markers; git status --porcelain examples/paladin_with_rag.rs empty"
        status: pass
    human_judgment: false
  - id: D4
    description: "All three programs are default-feature targets covered by the bulk cargo build --examples selector, no commit touches src/ or crates/, and make api-surface / check-api-surface.sh reports the surface unchanged across all three commits"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "cargo build --examples (exit 0, all three binaries present); git diff --stat HEAD~3..HEAD -- src/ crates/ (empty); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after each commit)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The post-audit drift in the five RAG capability-token greps (RagRetrievalResult, ShedItem, RagRetrievalError, retrieve_context_with_timeout, with_token_counter) is re-checked against examples/ and crates/doc-examples/src/ and recorded verbatim rather than skipped (D-02)"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "36-evidence/36-08-examples.txt Task 3 drift table: 3 of 5 tokens confirmed stale against crates/doc-examples/src/sanctum_vector_memory.rs (Phase 35), examples/ gallery itself still 0 hits pre-commit"
        status: pass
    human_judgment: false

# Metrics
duration: ~2h10min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 08: Agent-Runtime, Structured-Output & RAG-Retrieval Examples Summary

**Three new offline example binaries close thirteen of Phase 34's fifty-nine documentation gap rows: the agent-runtime middleware cluster (EX-83 through EX-87, EX-89), structured output (EX-88, EX-90), and RAG retrieval (EX-116 through EX-120) -- and surface a genuine, previously undetected redaction defect in the fail-run tool error path.**

## Performance

- **Duration:** ~2h10min
- **Completed:** 2026-09-17T22:05:24Z
- **Tasks:** 3
- **Files modified:** 4 (3 new example binaries, 1 new evidence file)

## Accomplishments

- `examples/agent_runtime_middleware.rs`: a custom `ExecutionMiddleware` whose
  `before_model`/`after_model`/`around_tool` hooks print in firing order against a
  real `PaladinExecutionService::execute` run; `AgentRuntimeConfig::build_chain`
  resolving `model_call_limit`/`token_budget`/`history_trimmer` from configuration
  alone and printing the resolved fields; a custom `TokenCounterPort` contrasted
  with the defaulted heuristic counter's `is_exact` answer; `SummarizationMiddleware`
  folding an 80-entry history into 1 summary + 10 kept-recent entries, and a
  standalone `HistoryTrimmer` demo trimming an 80-entry history to 15 under a tight,
  explicitly-named model-context-limit override; two `ConfinedVault` handles over one
  `InMemoryVault` store namespacing two agents apart, with the attempted
  cross-namespace read printed as a denied `VaultError::NamespaceDenied`; and the
  `ToolErrorMode::FailRun` policy failing a run with a structured
  `PaladinError::ArmamentFailed`, whose printed `reason` field is shown verbatim
  unredacted -- see the discovered-defect note below.
- `examples/structured_output_schema.rs`: derives a JSON Schema from a
  `WeatherReport` type via `schemars::schema_for!` and prints it; executes
  `StructuredExecutorExt::execute_structured::<WeatherReport>` against a conforming
  mock response and prints the typed value's own fields (not a raw string); then
  drives an `i8`-bounded `NarrowByte` type past its schema-permitted-but-serde-invalid
  range and prints the resulting typed `PaladinError::StructuredOutputInvalid`
  (attempts, last_error, raw_output) rather than accepting it silently.
- `examples/sanctum_rag_retrieval.rs`: a runnable sibling to the existing
  `paladin_with_rag.rs` conceptual walkthrough (left untouched, per D-18). Seeds five
  memories into a real `InMemorySanctum` through `RagRetrievalService::retrieve_context`,
  printing the typed `RagRetrievalResult` and its rendered `format_for_prompt` output;
  forces truncation with a tight budget and prints each `ShedItem`'s label/priority/
  original_bytes; triggers a typed `RagRetrievalError::BudgetTooLarge` from an
  impossible (`usize::MAX`) budget; calls the timeout-bounded
  `retrieve_context_with_timeout` free function; and injects an exact
  `TokenCounterPort` via `with_token_counter`, printing its `exact_tally: true` answer
  beside the heuristic default's `false`. Re-ran the five RAG capability-token greps
  against `examples/` and `crates/doc-examples/src/` before writing the program (D-02)
  and recorded the resulting drift verbatim in the evidence file, rather than skipping
  any row.
- All three binaries are default-feature targets (no `required-features` manifest
  entry needed), picked up by the bulk `cargo build --examples` selector, run to
  exit 0 with all three provider-key environment variables unset, and none of the
  three commits touches `src/` or `crates/` (`./scripts/check-api-surface.sh` reports
  the surface unchanged, 3959 items, checked after every commit).

## Task Commits

Each task was committed atomically:

1. **Task 1: examples/agent_runtime_middleware.rs (EX-83, EX-84, EX-85, EX-86, EX-87, EX-89)** - `4f88bf66` (docs)
2. **Task 2: examples/structured_output_schema.rs (EX-88, EX-90)** - `0d4f043b` (docs)
3. **Task 3: examples/sanctum_rag_retrieval.rs (EX-116, EX-117, EX-118, EX-119, EX-120)** - `b8a78324` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified

- `examples/agent_runtime_middleware.rs` - custom middleware hooks, AgentRuntimeConfig::build_chain, custom token counter, history trimming/summarization, confined vault namespacing, fail-run tool error mode
- `examples/structured_output_schema.rs` - schemars-derived JSON Schema, accepted and rejected structured-execution paths
- `examples/sanctum_rag_retrieval.rs` - real RagRetrievalService retrieval, shed records, typed error, timeout-bounded free function, exact counter injection
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-08-examples.txt` - run output, acceptance-criteria grep results, D-02 drift table, D-24 closure table

## Closure Table (D-24)

| ID | capability | program | commit |
|---|---|---|---|
| EX-83 | Custom ExecutionMiddleware hooks | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-84 | AgentRuntimeConfig-resolved built-in middleware | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-85 | Custom TokenCounterPort injection | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-86 | Context-window management (HistoryTrimmer + SummarizationMiddleware) | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-87 | ConfinedVault structural memory namespacing | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-89 | Fail-run tool error mode | examples/agent_runtime_middleware.rs | 4f88bf66 |
| EX-88 | Schema-validated structured output (accept + reject) | examples/structured_output_schema.rs | 0d4f043b |
| EX-90 | JSON Schema derivation via schemars | examples/structured_output_schema.rs | 0d4f043b |
| EX-116 | RagRetrievalResult typed retrieval | examples/sanctum_rag_retrieval.rs | b8a78324 |
| EX-117 | ShedItem shed records | examples/sanctum_rag_retrieval.rs | b8a78324 |
| EX-118 | Typed RagRetrievalError | examples/sanctum_rag_retrieval.rs | b8a78324 |
| EX-119 | retrieve_context_with_timeout free function | examples/sanctum_rag_retrieval.rs | b8a78324 |
| EX-120 | Exact TokenCounterPort injection (with_token_counter) | examples/sanctum_rag_retrieval.rs | b8a78324 |

## Post-Audit Drift (D-02)

Re-ran the five RAG capability-token greps from the Phase 34 audit against BOTH
`examples/` and `crates/doc-examples/src/` before writing Task 3, per the plan's own
instruction. Three of the five tokens (`RagRetrievalResult`, `ShedItem`,
`retrieve_context_with_timeout`) already had non-zero hits in
`crates/doc-examples/src/sanctum_vector_memory.rs`, added by Phase 35 after the
Phase 34 audit SHA -- the audit's recorded zero-hit figure for those three was
already stale before this plan started. The gap this plan's own Task 3 closes
(`examples/` gallery, not `crates/doc-examples/`) was unaffected: `examples/` itself
showed 0 hits for all five tokens right up to this plan's own commit. Full table with
file/hit counts is in `36-evidence/36-08-examples.txt`'s Task 3 section.

## Decisions Made

- Split the agent-runtime cluster (six EX rows) and structured-output cluster (two EX
  rows) into two separate programs, and kept the RAG cluster (five EX rows) as a
  sibling to the existing `paladin_with_rag.rs` rather than a rewrite -- all three
  choices exercise this phase's own D-15 per-planner discretion, following the
  36-01/36-06/36-07 precedent.
- `HistoryTrimmer`'s standalone demo names the Paladin's model explicitly
  (`"demo-model"`) and overrides `model_context_limits` for that exact name, because
  the resolver tries the config table before the provider's own declared
  capabilities and `MockLlmAdapter::get_capabilities()` reports a 4096-token window
  that would otherwise shadow a smaller `default_context_tokens` and produce a
  no-op trim -- discovered by actually running the program before committing.
- `sanctum_rag_retrieval.rs`'s embedder derives each memory's vector from a
  fixed-key content hash rather than returning one constant vector, after two runs
  of the program printed a *different* retained memory each time. Traced to
  `InMemorySanctum`'s `HashMap`-backed storage, whose iteration order is randomized
  per process (not per input) when every search result ties at the same similarity
  score -- confirmed by reading `InMemorySanctum::cosine_similarity` and by
  re-running the program three times after the fix with byte-identical output each
  time (only the freshly-generated memory UUIDs differ, as expected).
- Chose *not* to force the exact-counter contrast (Part 5) toward a numerically
  different retained/shed count than the heuristic run: after computing a
  principled midpoint budget from both counters' totals, the two still coincided at
  the same boundary for this prose. Rather than hand-tuning further to manufacture a
  difference, the printed narrative states plainly that `exact_tally` is the
  property `with_token_counter` reliably changes, and the retained/shed counts may
  or may not differ depending on where the ration boundary falls -- honest over
  convenient.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected a no-op HistoryTrimmer demo caused by provider capability precedence**
- **Found during:** Task 1 (agent_runtime_middleware.rs, Part 4's standalone-trimmer demo)
- **Issue:** First draft left `model_context_limits` empty and relied on
  `default_context_tokens: 256` alone; `HistoryTrimmer::resolve_limit`'s three-step
  precedence (config table, then the LLM port's own `get_capabilities()`, then the
  configured default) picked up `MockLlmAdapter::get_capabilities()`'s reported
  4096-token window before ever reaching the 256-token default, so the demo's
  80-entry, ~2600-token history fit comfortably and nothing was trimmed (80 entries
  -> 80 entries, contradicting the printed claim).
- **Fix:** Named the demo Paladin's model explicitly (`"demo-model"`) and added a
  `model_context_limits` entry for that exact name, which the resolver consults
  before the provider capability.
- **Files modified:** examples/agent_runtime_middleware.rs
- **Verification:** Re-ran the example; trimming now reduces 80 entries to 15,
  confirmed stable across a second run.
- **Committed in:** 4f88bf66 (Task 1 commit)

**2. [Rule 2 - Missing Critical] Made sanctum_rag_retrieval.rs's ranking deterministic across runs**
- **Found during:** Task 3 (sanctum_rag_retrieval.rs, Parts 1/2's retrieval demo)
- **Issue:** First draft's `FixedEmbedder` returned one constant vector for every
  text, so all five seeded memories tied at similarity 1.0. Running the program
  twice printed a *different* retained memory each time -- traced to
  `InMemorySanctum`'s `HashMap`-backed storage, whose iteration order is randomized
  per process (Rust's default `HashMap` uses a per-process random seed), so tied
  search results surface in a different order on every invocation.
- **Fix:** Changed `FixedEmbedder` to derive each memory's vector from a fixed-key
  content hash (`std::collections::hash_map::DefaultHasher::new()`, whose key is
  fixed, unlike `HashMap`'s own `RandomState`) instead of one constant vector, so
  distinct texts get distinct (non-tied) similarity scores and the ranking is
  reproducible.
- **Files modified:** examples/sanctum_rag_retrieval.rs
- **Verification:** Re-ran the example three times; retained/shed byte lengths,
  priorities and every typed value were byte-identical across all three runs (only
  the freshly-generated memory UUIDs differed, as expected -- identity, not content).
- **Committed in:** b8a78324 (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing-critical -- both discovered by
actually running each program before committing, not assumed from reading the
source).
**Impact on plan:** Both fixes were necessary for the examples to demonstrate what
their own printed claims say; no scope change beyond making the demos correct.

## Discovered Library Defect (recorded, not fixed -- D-18 boundary)

While building Task 1's fail-run tool error mode demo (EX-89), reading
`PaladinError::ArmamentFailed`'s own doc comment (`crates/paladin-core/src/platform/container/paladin_error.rs`)
revealed it promises `reason` is "a redacted, human-readable summary ... never raw
provider or tool output" -- the same sanitized text `ToolResultFormatter::format_error`
produces under `ToolErrorMode::FeedToModel` (proven by that mode's own
`a_secret_in_a_tool_error_never_reaches_the_model` test). Reading the actual
`ToolErrorMode::FailRun` call sites in `src/application/services/paladin/paladin_execution_service.rs`
(the Armament arm around line 1648, the handoff arm around line 1769) showed both
build `reason: e.to_string()` directly, never routing through
`ToolResultFormatter::sanitize_tool_text` -- the module's one sanitization point
(`redact_secret_patterns` then `bounded_excerpt`). Running the example's fail-run demo
with a fake credential embedded in the failing tool's error text (`Bearer
sk-live-demo0123456789`) confirmed this empirically: the printed `reason` field shows
the fake credential verbatim, unredacted.

This is a genuine, previously undetected contradiction between a documented API
invariant and its shipped implementation -- a real security-relevant defect (T-26-03's
redact-then-bound invariant, violated for exactly one of its two call paths). Per this
plan's own D-18 boundary ("no library source under src/ or crates/ changes" -- a
docs-only phase), it is **not fixed here**. Recorded in:
- This SUMMARY (above)
- `36-evidence/36-08-examples.txt` (Task 1 section)
- `.planning/WINDOWS.md` entry #38 (kind: deviation, phase 36, file
  `src/application/services/paladin/paladin_execution_service.rs:1648`)

## Issues Encountered

None beyond the two auto-fixed issues and the discovered library defect documented
above.

## User Setup Required

None - no external service configuration required. All three examples are fully
offline.

## Next Phase Readiness

- Thirteen more of the fifty-nine Phase 34 audit gap rows are closed (EX-83 through
  EX-90 except EX-88's own row order, plus EX-116 through EX-120). Combined with
  plans 36-01, 36-06 and 36-07, twenty-nine of fifty-nine rows are now closed.
- Plan 36-11, which owns `examples/README.md`, still needs to add a section for each
  of these three new programs -- no README edit was made here per this plan's own
  scope note (README-writing is plan 36-11's job).
- WINDOWS.md entry #38 (the FailRun redaction defect) is a genuine, unresolved
  library defect that a future phase or a dedicated fix plan should address --
  flagged here for visibility, not silently absorbed into "documentation currency."
- No blockers for subsequent Phase 36 plans.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: examples/agent_runtime_middleware.rs
- FOUND: examples/structured_output_schema.rs
- FOUND: examples/sanctum_rag_retrieval.rs
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-08-examples.txt
- FOUND commit: 4f88bf66
- FOUND commit: 0d4f043b
- FOUND commit: b8a78324
