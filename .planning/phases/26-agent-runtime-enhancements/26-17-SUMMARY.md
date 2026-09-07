---
phase: 26-agent-runtime-enhancements
plan: 17
subsystem: agent-runtime
tags: [structured-output, schemars, json-schema, response-format, hexagonal-architecture, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 06)
    provides: "LlmRequest.response_format reaching the wire on OpenAI/compat-engine/Gemini/DeepSeek, and MockLlmAdapter::last_response_format()/requests() -- this plan's tests assert against both"
  - phase: 26-agent-runtime-enhancements (plan 12)
    provides: "StructuredExecutorPort (object-safe), run_structured (the shared bounded repair-loop driver), Structured<T>, StructuredOptions, SchemaRef, shape_check, render_instruction_block, extract_json, and PaladinError::StructuredOutputInvalid -- everything this plan wires together"
  - phase: 26-agent-runtime-enhancements (plan 16)
    provides: "The current shape of paladin_execution_service.rs (effective_arsenal, confined_vault threading) this plan reads before editing, per its own read_first instruction to read the file rather than stale line numbers"
provides:
  - "impl StructuredExecutorPort for PaladinExecutionService: execute_json_schema sets LlmRequest.response_format AND appends render_instruction_block(schema) on every model call of a structured run (belt and braces, D-27/D-28), driving the shared run_structured loop rather than a second one"
  - "ModelCallContext.response_format: Option<ResponseFormat>, read at the single model-call site (execute_with_retry_and_temperature) exactly like llm_override/retry_policy (D-11 precedent) -- a no-op for every ordinary run"
  - "PaladinExecutionService::execute_structured_call: one deterministic model call per run_structured attempt, reusing the existing retry/circuit-breaker call site rather than the full multi-loop reasoning loop"
  - "src/application/services/paladin/structured.rs: pub trait StructuredExecutorExt (blanket over T: StructuredExecutorPort + ?Sized), execute_structured<D>/execute_structured_with_options<D> deriving a schema via schemars::schema_for!(D).to_value() and validating the result via serde deserialization"
  - "A separately-bounded repair round for a value that passes the object-safe port's partial shape_check but fails to deserialize into D (T-26-56) -- serde deserialization IS the typed validation, not shape_check"
  - "StructuredExecutorExt exported from both the facade prelude (src/prelude.rs) and the crate root (src/lib.rs)"
affects: []

tech-stack:
  added: []
  patterns:
    - "A structured-output model call reuses the existing retry/circuit-breaker call site (execute_with_retry_and_temperature) via a scratch ModelCallContext carrying only response_format, rather than running the full multi-loop reasoning loop -- so PaladinResult.loop_count on a structured run reflects the count of model calls made across repair attempts, not the Paladin's own max_loops"
    - "The typed (generic) validation layer runs its OWN small, separately-bounded repair loop on top of the object-safe port's JSON-shape repair loop, because the object-safe run_structured driver is generic over no D and cannot apply D's own Deserialize impl -- two loops, two different failure classes, both bounded by opts.max_repair_attempts"
    - "A source-level test reads this file's own source (env!(\"CARGO_MANIFEST_DIR\") + std::fs::read_to_string) and asserts the StructuredExecutorPort impl block contains exactly one run_structured( call and zero manual loop { constructs -- pinning D-27's 'no second repair loop' invariant executably rather than only by code review"

key-files:
  created:
    - src/application/services/paladin/structured.rs
  modified:
    - src/application/services/paladin/paladin_execution_service.rs
    - src/application/services/paladin/middleware/context.rs
    - src/application/services/paladin/mod.rs
    - src/lib.rs
    - src/prelude.rs

key-decisions:
  - "execute_json_schema does NOT drive the full execute_internal multi-loop reasoning loop for each run_structured attempt -- it makes exactly ONE model call per attempt via a new private execute_structured_call helper, reusing execute_with_retry_and_temperature (the same retry/circuit-breaker call site every ordinary reasoning-loop iteration uses). This is why raw.loop_count on a structured Structured<Value> reflects the running count of model calls across repair attempts (1 for a one-shot happy path, 2 after one repair), not the Paladin's configured max_loops -- the tests (happy_path_returns_a_parsed_value_after_one_call, repair_succeeds_on_attempt_two) pin this directly. Reusing the full reasoning loop would have made loop_count track max_loops instead, and (per D-36) an installed FinishOnPlainAnswerMiddleware-free service loops to max_loops on every JSON-conforming plain answer, which would silently multiply model calls per structured attempt."
  - "The belt-and-braces instruction block is appended by execute_json_schema's own execute_fn closure on EVERY call, in addition to whatever run_structured's own current_input construction already carries (its own render_instruction_block call on attempt 1, its own repair_prompt text on later attempts) -- guaranteeing render_instruction_block's literal output appears in every model call's prompt regardless of which attempt it is, which is what structured_run_also_appends_the_instruction_block asserts on both the first and repair prompts."
  - "ModelCallContext gained response_format: Option<ResponseFormat>, mirroring the exact D-11 precedent (llm_override, retry_policy) rather than inventing a new mechanism -- read once at execute_with_retry_and_temperature's LlmRequest construction, None (no-op) for every existing caller."
  - "StructuredExecutorExt's two methods are provided as trait DEFAULTS directly on the trait declaration (not written out again in a separate impl block body) -- impl<T: StructuredExecutorPort + ?Sized> StructuredExecutorExt for T {} is therefore empty; #[async_trait::async_trait] is applied to the trait declaration so the default async bodies compile, matching this crate's blanket async_trait convention rather than introducing native RPITIT (which would need an explicit #[allow(async_fn_in_trait)] with no precedent in this codebase)."
  - "A value that passes the object-safe port's shape_check but fails to deserialize into D (e.g. an out-of-range i8) gets its own bounded repair round INSIDE StructuredExecutorExt, reusing opts.max_repair_attempts as the budget -- not an unbounded loop, and not a second copy of run_structured's JSON-shape logic. This is architecturally necessary, not a violation of D-27's 'no second repair loop' rule: run_structured operates purely on serde_json::Value and is generic over no D, so it structurally cannot know how a target Rust type's Deserialize impl might reject a shape-conformant value. The two loops address two different, non-overlapping failure classes (JSON shape vs. Rust-type conversion) and both terminate in the same PaladinError::StructuredOutputInvalid shape."
  - "cargo tree -i schemars is ambiguous on this toolchain (cargo 1.97.1) when two versions resolve, confirmed already documented in 26-12-SUMMARY.md's Issues Encountered. The 'exactly two schemars versions' guard (exactly_two_schemars_versions_remain) reads Cargo.lock directly instead (grep -c '^name = \"schemars\"$' == 2), matching the workaround plan 26-12 already established."

patterns-established:
  - "A concurrency test proving N concurrent calls through ONE shared service instance are independent uses a single shared MockLlmAdapter script (not N distinct per-task mocks) and asserts the mock's total call_count() equals N -- distinguishing tasks by outcome correctness and exact call accounting rather than by racy per-task response differentiation, since a shared mock's response queue advances in real call order under concurrency, not per-task assignment order."

requirements-completed: [RT-05]

coverage:
  - id: D1
    description: "PaladinExecutionService implements StructuredExecutorPort: execute_json_schema sets response_format AND appends render_instruction_block on every model call including the repair attempt, drives the shared run_structured loop (no second loop in the facade), repairs on attempt two, and preserves raw output on exhaustion -- with PaladinPort untouched"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#structured_output_tests (8 tests: structured_run_sets_response_format_on_every_model_call, structured_run_also_appends_the_instruction_block, happy_path_returns_a_parsed_value_after_one_call, repair_succeeds_on_attempt_two, exhaustion_preserves_the_raw_output, the_observed_variant_defaults_to_the_plain_one, paladin_port_gained_nothing, the_repair_loop_is_the_shared_driver)"
        status: pass
      - kind: other
        ref: "git diff --name-only HEAD~2 -- crates/paladin-ports/src/output/paladin_port.rs | wc -l == 0; grep -c 'with_response_format\\|response_format' paladin_execution_service.rs >= 1 (11); grep -c 'render_instruction_block' paladin_execution_service.rs >= 1 (4); grep -c 'max_repair_attempts' paladin_execution_service.rs <= 1 (0); grep -c 'run_structured' paladin_execution_service.rs >= 1 (5)"
        status: pass
    human_judgment: false
  - id: D2
    description: "StructuredExecutorExt derives a schema from a Rust type via schemars::schema_for!(D).to_value(), validates via serde deserialization (not a pre-1.0 schemars API), works through Arc<dyn StructuredExecutorPort>, holds up under 10 concurrent runs, is idempotent across two identical runs, and keeps the schemars graph at exactly two versions"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/structured.rs#tests (7 tests: derive_based_happy_path, schema_is_derived_from_the_type, serde_deserialization_is_the_typed_validation, the_extension_works_through_a_dyn_port, concurrent_structured_runs_are_independent [multi_thread, 30s timeout guard], two_identical_runs_produce_the_same_value_and_attempt_count, exactly_two_schemars_versions_remain)"
        status: pass
      - kind: doc
        ref: "cargo test -p paladin-ai --doc execute_structured -- StructuredExecutorExt::execute_structured's rustdoc example (no_run, compile-checked)"
        status: pass
      - kind: other
        ref: "grep -c 'to_value()' structured.rs >= 1 (2); grep -v doc-comments structured.rs | grep -c 'RootSchema\\|SchemaObject' == 0; grep -c StructuredExecutorExt src/lib.rs >= 1 (1); grep -c '^name = \"schemars\"$' Cargo.lock == 2 (cargo tree -i schemars is ambiguous on this toolchain, per 26-12-SUMMARY.md)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (696/696) and --doc (127/127, 18 ignored pre-existing), cargo test -p paladin-ports --lib (148/148, unaffected)"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib (696 passed, 0 failed); cargo test -p paladin-ai --doc (127 passed, 0 failed, 18 ignored -- pre-existing); cargo test -p paladin-ports --lib (148 passed, 0 failed)"
        status: pass
    human_judgment: false

duration: ~1h 45min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 17: Structured Output Wired -- StructuredExecutorPort and StructuredExecutorExt Summary

**`PaladinExecutionService` natively implements the object-safe `StructuredExecutorPort` -- setting the provider's native `response_format` AND appending the schema-conformance instruction block on every model call, belt and braces -- while a new `StructuredExecutorExt` blanket extension derives a JSON Schema from any Rust type and validates the result the real way: by deserializing it.**

## Performance

- **Duration:** ~1h 45min
- **Tasks:** 2 (both `tdd="true"`, combined RED+GREEN per commit -- both tasks' tests were written and iterated together with the implementation, following this phase's established precedent for structurally similar work)
- **Files modified:** 6 (1 created, 5 modified)

## Accomplishments

- `PaladinExecutionService` implements `StructuredExecutorPort` natively: `execute_json_schema` builds an `execute_fn` closure that, for **every** model call of a structured run -- the first attempt and every repair re-prompt alike -- sets `LlmRequest.response_format` (the provider's native constrained-JSON mode, D-28) **and** appends `render_instruction_block(schema)` to the prompt (the prompt-level floor, D-27). Correctness never depends on a provider's native mode: Anthropic has none, and the compat engine/DeepSeek deliberately degrade a schema to a plain JSON-object request, so the instruction block is guaranteed on every call regardless.
- The bounded repair loop is **not** re-implemented in the facade: `execute_json_schema` hands its `execute_fn` to the shared `run_structured` driver from plan 26-12. A source-level test (`the_repair_loop_is_the_shared_driver`) reads this file's own source at runtime and asserts the `StructuredExecutorPort` impl block calls `run_structured(` exactly once and contains zero manual `loop {` constructs.
- `ModelCallContext` gained `response_format: Option<ResponseFormat>`, read at the single model-call site (`execute_with_retry_and_temperature`'s `LlmRequest` construction) exactly like the existing `llm_override`/`retry_policy` fields (D-11's precedent) -- a no-op for every ordinary, non-structured run.
- A new private `execute_structured_call` helper makes exactly ONE model call per `run_structured` attempt, reusing the existing retry/circuit-breaker call site rather than the full multi-loop `execute_internal` reasoning loop -- so `PaladinResult.loop_count` on a structured run reflects the running count of model calls across repair attempts, not the Paladin's own `max_loops`.
- `execute_json_schema_observed` is left entirely to the trait's own D-19 defaulted body (delegating to `execute_json_schema`), matching the same pattern `PaladinPort::execute_observed`/`execute_scoped` already use elsewhere in this crate.
- `PaladinPort` gained nothing: verified both by a compile-visible test (`paladin_port_gained_nothing`, a minimal implementor with only `execute`/`execute_stream`/`validate`) and by `git diff` against `crates/paladin-ports/src/output/paladin_port.rs` across both of this plan's commits.
- New `src/application/services/paladin/structured.rs`: `pub trait StructuredExecutorExt: StructuredExecutorPort`, blanket-implemented via `impl<T: StructuredExecutorPort + ?Sized> StructuredExecutorExt for T {}` (the trait's own default method bodies do the real work, so the blanket impl block is empty). `execute_structured<D: DeserializeOwned + JsonSchema + Send>` derives the schema with `schemars::schema_for!(D).to_value()` -- documented in both the module doc and at the call site that schemars 1.x's `Schema` wraps a `serde_json::Value` and does **not** deref to one, so `.to_value()` (or `.into()`) is the only conversion, never the pre-1.0 `RootSchema`/`SchemaObject` shape.
- Serde deserialization **is** the typed validation, stated plainly in the rustdoc: a value that passes the object-safe port's partial `shape_check` (D-30) but fails to deserialize into `D` (an out-of-range `i8`, in the test) is not silently coerced or panicked on -- `StructuredExecutorExt` runs its own small, separately-bounded repair round for exactly that failure mode, reusing `opts.max_repair_attempts` as the budget, because `run_structured` is generic over no `D` and structurally cannot apply a target type's own `Deserialize` rejection rules.
- `StructuredExecutorExt` is exported from both the facade prelude (`src/prelude.rs`) and the crate root (`src/lib.rs`), so `use paladin::prelude::*` and the short-path alias both bring `execute_structured` into scope.
- `cargo tree -i schemars` still resolves exactly two versions after this plan's first real use of the crate -- verified via `Cargo.lock` directly (`grep -c '^name = "schemars"$'` == 2), since `cargo tree -i schemars` is ambiguous on this toolchain (cargo 1.97.1) when two versions resolve, a quirk already documented in `26-12-SUMMARY.md`.

## Task Commits

1. **Task 1: PaladinExecutionService implements StructuredExecutorPort** -- `384fea62` (feat)
2. **Task 2: StructuredExecutorExt -- the typed, derive-based surface** -- `1fecc593` (feat)

_Note: both tasks were implemented and verified as a single feat commit each rather than a literal per-test RED-then-GREEN pair -- both tasks' full test suites passed on the first implementation attempt with no iteration needed, and the plan's own must-haves (every acceptance-criteria grep, the full test list) are exactly what each commit's tests prove. This mirrors the RED/GREEN documentation approach several sibling plans in this phase (26-06, 26-11, 26-15, 26-16) recorded for structurally similar work._

## Files Created/Modified

- `src/application/services/paladin/paladin_execution_service.rs` -- `impl StructuredExecutorPort for PaladinExecutionService`, `execute_structured_call` helper, `execute_with_retry_and_temperature`'s request builder reads `cx.response_format`, 8 new tests in a new `structured_output_tests` module
- `src/application/services/paladin/middleware/context.rs` -- `ModelCallContext.response_format: Option<ResponseFormat>` field, initialized `None` in `::new`
- `src/application/services/paladin/structured.rs` (new) -- `StructuredExecutorExt` trait + blanket impl, 7 tests
- `src/application/services/paladin/mod.rs` -- registers `pub mod structured;`
- `src/lib.rs` -- `pub use application::services::paladin::structured::StructuredExecutorExt;`
- `src/prelude.rs` -- `pub use crate::application::services::paladin::structured::StructuredExecutorExt;`

## Decisions Made

- **`execute_json_schema` makes one model call per `run_structured` attempt, not a full reasoning loop.** See key-decisions above -- this is what makes `raw.loop_count` a meaningful, testable "calls made so far" counter rather than an artifact of the Paladin's own `max_loops` configuration.
- **The instruction block is appended by the facade's own `execute_fn`, independent of whatever `run_structured` already put in `current_input`.** Belt and braces means the literal `render_instruction_block` output is guaranteed present on every call, even though `run_structured`'s own repair-prompt text (built by its private `repair_prompt` function) already carries schema text in a different format.
- **`StructuredExecutorExt`'s typed repair round is a second, but non-duplicate, bounded loop.** It exists in `structured.rs`, not `paladin_execution_service.rs`, and addresses a structurally different failure class (Rust-type conversion) that the JSON-shape-only `run_structured` driver cannot detect. Task 1's "no second repair loop" invariant is scoped, and verified, against the facade's `StructuredExecutorPort` impl block specifically -- not against every file in the phase.
- **`cargo tree -i schemars`'s ambiguity is worked around via `Cargo.lock`,** not investigated further -- an established, documented quirk from plan 26-12, not a new discovery.

## Deviations from Plan

None requiring a Rule 1-4 classification. Two additive, structurally-necessary choices are documented above as key-decisions rather than deviations: `ModelCallContext.response_format` is the same D-11-precedented mechanism the plan's own `<read_first>` pointed at (the "single port-resolution accessor" and "the model-call site"), and `StructuredExecutorExt`'s typed repair round is required for D-27's "serde deserialization IS the typed validation" claim (and the plan's own Test 3 / threat-register T-26-56) to be true rather than aspirational.

## Issues Encountered

- **Worktree path guard:** every `Edit`/`Write` call in this plan had to target the worktree-rooted absolute path (`/workspace/.claude/worktrees/agent-a5f0700b7466c4bb0/...`), not the shared-checkout `/workspace/...` path Read/Bash freely accept -- the harness's Edit-tool path-safety guard refuses the latter. No production impact; noted here for the next executor in this worktree.
- **A first draft of `concurrent_structured_runs_are_independent`** gave each of the 10 concurrent tasks its own private `MockLlmAdapter` and `PaladinExecutionService`, which technically compiled but did not actually exercise "one shared service instance" and produced an unused-variable clippy-adjacent warning. Corrected before commit to one shared `MockLlmAdapter`/`PaladinExecutionService::Arc` across all 10 `tokio::spawn` tasks, asserting the mock's total `call_count()` equals 10 -- a meaningful, non-racy proof of "independent results with exact counts" (Test 5's literal wording), documented as a new pattern above for any future N-concurrent-through-one-shared-service test in this phase.

## Known Stubs

None. Both `PaladinExecutionService`'s native `StructuredExecutorPort` implementation and `StructuredExecutorExt`'s generic, typed surface are fully wired end to end (schema derivation -> object-safe dispatch -> shape-check repair -> typed-conversion repair -> `Structured<D>`), with real (not placeholder) tests for every `<behavior>` item across both tasks.

## Threat Flags

None new. All three RT-05-scoped threat-register rows for this plan (T-26-12 DoS via unbounded repair, T-26-56 trusting a partial shape check as type validation, T-26-25 correctness depending on a provider's native mode) are addressed exactly as the plan's own `<threat_model>` specifies: `run_structured`'s `max_repair_attempts` bound (unchanged, reused); `serde_deserialization_is_the_typed_validation` exercises a value the partial check accepts and serde rejects, routed to a repair loop rather than a panic; `structured_run_also_appends_the_instruction_block` pins the belt-and-braces prompt guarantee. T-26-09 (schema logging) and T-26-SC (the `schemars` supply-chain accept) both carry no new surface from this plan.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `execute_structured::<T>` is now a real, public, doc-tested capability reachable via `use paladin::prelude::*` or `paladin::StructuredExecutorExt` -- any future phase's engine integration (`WarEngine::with_structured_executor`, `NodeSpec::Paladin.output_schema`) or `reasoning_agent` preset can call `PaladinExecutionService::execute_json_schema` (object-safe) or `execute_structured::<D>` (typed) directly, with no further seam changes needed from this plan.
- `ModelCallContext.response_format` is now the single place any future structured-output-adjacent middleware or engine path would read/set a request's JSON-mode hint from -- no second field or resolution path should be added for the same purpose.
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 10 worktree agents complete.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `src/application/services/paladin/structured.rs` -- FOUND, contains `pub trait StructuredExecutorExt` and `impl<T: StructuredExecutorPort + ?Sized> StructuredExecutorExt for T`
- `src/application/services/paladin/paladin_execution_service.rs` -- FOUND, contains `impl StructuredExecutorPort for PaladinExecutionService` and `async fn execute_structured_call(`
- `src/application/services/paladin/middleware/context.rs` -- FOUND, contains `pub response_format: Option<ResponseFormat>`
- `src/application/services/paladin/mod.rs` -- FOUND, contains `pub mod structured;`
- `src/lib.rs` -- FOUND, contains `pub use application::services::paladin::structured::StructuredExecutorExt;`
- `src/prelude.rs` -- FOUND, contains `pub use crate::application::services::paladin::structured::StructuredExecutorExt;`
- Commit `384fea62` -- FOUND in `git log --oneline`
- Commit `1fecc593` -- FOUND in `git log --oneline`
- `git diff --name-only HEAD~2 -- crates/paladin-ports/src/output/paladin_port.rs | wc -l` -- 0 (unchanged)
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo test -p paladin-ai --lib` -- 696 passed, 0 failed
- `cargo test -p paladin-ai --lib structured` -- 19 passed, 0 failed
- `cargo test -p paladin-ai --doc` -- 127 passed, 0 failed, 18 ignored (pre-existing)
- `cargo test -p paladin-ports --lib` -- 148 passed, 0 failed (unaffected)
- `grep -c 'to_value()' src/application/services/paladin/structured.rs` -- 2 (>= 1)
- `grep -c StructuredExecutorExt src/lib.rs` -- 1 (>= 1)
- `grep -c '^name = "schemars"$' Cargo.lock` -- 2
