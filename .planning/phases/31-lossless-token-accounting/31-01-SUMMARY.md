---
phase: 31-lossless-token-accounting
plan: 01
subsystem: core-domain
tags: [rust, serde, token-usage, saturating-arithmetic, tdd]

# Dependency graph
requires:
  - phase: 30-token-economy-vocabulary-commissary-anchoring
    provides: locked vocabulary (Commissary/Treasurer) and ADR-0051's clean-break sanction for Phases 31-33
provides:
  - "TokenUsage with cache_read_tokens/cache_write_tokens/reasoning_tokens (Option<u32>, #[serde(default)]) after total_tokens"
  - "TokenUsage::with_cache_read/with_cache_write/with_reasoning doc-tested builders"
  - "impl Add/AddAssign/Sum for TokenUsage with saturating arithmetic and the D-06 Option-merge rule"
  - "Every in-tree full TokenUsage struct literal (~99 sites, ~43 files) migrated to TokenUsage::new(..) — workspace compiles clean on the six-field shape"
affects: [31-02, 31-03, 31-04, 31-05, 31-06, 31-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Saturating accumulation with an Option-merge private helper (None+None=None, None+Some(x)=Some(x), Some(a)+Some(b)=Some(a.saturating_add(b)))"
    - "Provider usage structs keep a now-unread total field under #[allow(dead_code)] with a rationale comment, rather than deleting it, so the deserializer still matches the full wire shape"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/token_usage.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - crates/paladin-llm/src/deepseek/adapter.rs
    - crates/paladin-llm/src/gemini/adapter.rs
    - crates/paladin-llm/src/compat/engine.rs
    - crates/paladin-llm/src/compat/types.rs
    - crates/paladin-llm/src/mock.rs
    - (36 further files across crates/, src/, tests/, examples/, benches/ — full list in Task Commits below)

key-decisions:
  - "Fixed the one in-crate TokenUsage literal in herald.rs's own test during Task 1 (Rule 3 auto-fix) because paladin-core is a single compilation unit — Task 1's own verify command (`cargo test -p paladin-ai-core --lib token_usage`) cannot pass while any literal in the same crate fails to compile, even though herald.rs's broader migration is nominally Task 2's file"
  - "Provider usage structs' now-unread total fields (OpenAIUsage.total_tokens, GeminiUsageMetadata.total_token_count, CompatUsage.total_tokens) are kept with #[allow(dead_code)] rather than deleted, preserving the full deserialization wire shape for future debugging"
  - "compat/engine.rs's redundant total_tokens local (previously .unwrap_or(prompt+completion)) was removed rather than kept dead, since TokenUsage::new now always recomputes the identical sum"
  - "mock.rs's with_token_usage(prompt, completion, total) keeps its three-parameter signature for source compatibility but the total is renamed _total_tokens and ignored, since TokenUsage::new enforces the D-02 invariant"

patterns-established:
  - "Constructible struct + Default derive gets new fields as Option<T> behind #[serde(default)], never #[non_exhaustive] (X-10.3 option (b), matches PaladinResult/Settings precedent)"
  - "Struct-literal-to-constructor migration is compiler-driven: run cargo check --workspace --all-targets --all-features repeatedly and fix exactly what it reports, never guess at additional sites"

requirements-completed: [ACCT-01]

coverage:
  - id: D1
    description: "TokenUsage carries three new Option<u32> sub-counts (cache_read_tokens, cache_write_tokens, reasoning_tokens) with #[serde(default)], documented inclusive-total contract, and doc-tested with_* builders"
    requirement: "ACCT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::builders_set_optionals_and_leave_prompt_completion_total_untouched"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc token_usage (3 builder doctests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "TokenUsage arithmetic (Add/AddAssign/Sum) saturates at u32::MAX, never panics, recomputes total_tokens as prompt+completion after every add, and follows the None/Some option-merge rule"
    requirement: "ACCT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::add_saturates_at_u32_max_without_panicking"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::default_is_the_additive_identity"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::option_merge_some_plus_some_saturating_adds"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::sum_over_iterator_equals_sequential_add"
        status: pass
    human_judgment: false
  - id: D3
    description: "Legacy three-key JSON deserialises with all three optionals None; a six-key document round-trips byte-identically with the optionals always emitted (as null when None)"
    requirement: "ACCT-01"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::legacy_json_without_optionals_deserialises_to_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::six_key_json_round_trips_and_serialises_all_six_keys"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/token_usage.rs#tests::none_optionals_serialise_as_null_not_omitted"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every in-tree full TokenUsage struct literal migrated to TokenUsage::new(..); the whole workspace compiles, tests, formats and lints clean on the six-field shape with no numeric fixture value changed"
    requirement: "ACCT-01"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md)"
        status: pass
      - kind: other
        ref: "cargo fmt --all -- --check (exit 0)"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
    human_judgment: false

duration: ~48min
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 01: TokenUsage Sub-Counts and Saturating Arithmetic Summary

**`TokenUsage` gains cache-read/cache-write/reasoning `Option<u32>` sub-counts and saturating `Add`/`AddAssign`/`Sum`, with every in-tree struct literal (~99 sites across ~43 files) migrated to `TokenUsage::new(..)` in the same commit sequence so the workspace stays green.**

## Performance

- **Duration:** ~48 min
- **Completed:** 2026-09-15
- **Tasks:** 2 (plus 1 pre-resolved checkpoint:decision)
- **Files modified:** 45 (2 in Task 1, 43 in Task 2)

## Accomplishments

- `TokenUsage` (`crates/paladin-core/src/platform/container/token_usage.rs`) carries three new
  `Option<u32>` fields — `cache_read_tokens`, `cache_write_tokens`, `reasoning_tokens` — each
  `#[serde(default)]`, positioned after `total_tokens` per D-01, with rustdoc stating the
  inclusive-total contract and both inequalities (D-02).
- Three doc-tested chainable builders (`with_cache_read`, `with_cache_write`, `with_reasoning`)
  are the only way to set the optionals; `new`/`from_total` leave them `None` (D-03, D-04).
- Saturating `impl Add`, `impl AddAssign`, `impl std::iter::Sum` with the D-06 Option-merge rule
  (`None+None=None`, `None+Some(x)=Some(x)`, `Some(a)+Some(b)=Some(a.saturating_add(b))`);
  `total_tokens` is recomputed as `prompt_tokens.saturating_add(completion_tokens)` after every
  add so the invariant survives summation. `new` itself now uses `saturating_add` too.
- 14 new unit tests plus 3 new doctests cover the saturating boundary, additive identity, all
  three Option-merge cases, legacy-JSON-without-optionals deserialization, and the six-key
  round-trip (JSON always emits the three optional keys as `null` when `None`, no
  `skip_serializing_if`, per D-21's downstream dependency).
- Every in-tree full `TokenUsage` struct literal — compiler-driven discovery via
  `cargo check --workspace --all-targets --all-features`, ~99 literal sites across ~43 files
  spanning `crates/`, `src/`, `tests/`, `examples/`, `benches/`, plus 6 doc-comment examples in
  `herald.rs` and `llm_port.rs` that only surface under `cargo test --doc` — migrated to
  `TokenUsage::new(prompt, completion)`. No numeric fixture value changed. `VisionTokenUsage`
  literals (a different type, D-20) and `vision_port.rs` are untouched.
- The whole workspace compiles (`cargo check --workspace --all-targets --all-features`), tests
  green (`cargo test --workspace --all-features`), formats clean (`cargo fmt --all -- --check`),
  and lints clean (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).

## Task Commits

Each task was committed atomically, following the plan's TDD requirement for the tracer task:

1. **Task 1 (RED): add failing tests for TokenUsage cache/reasoning sub-counts** - `18d69e5c` (test)
2. **Task 1 (GREEN): TokenUsage gains cache/reasoning sub-counts and saturating arithmetic** - `524d59c7` (feat) — includes the one blocking in-crate `herald.rs` test-literal fix (Rule 3)
3. **Task 2: migrate every in-tree TokenUsage struct literal to new()** - `338f44d4` (refactor)

_Note: the checkpoint:decision task (confirm the one-way wire and public-API shape) was
pre-resolved by the orchestrator as `proceed-as-decided` under auto-mode per the executor's
`<checkpoint_resolution>` instructions — no separate commit, recorded under Decisions Made below._

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the
final metadata commit after merge)

## Files Created/Modified

**Task 1:**
- `crates/paladin-core/src/platform/container/token_usage.rs` — three new `Option<u32>` fields, three builders, saturating `Add`/`AddAssign`/`Sum`, expanded rustdoc, 14 new tests
- `crates/paladin-core/src/platform/container/herald.rs` — one blocking in-crate literal fixed early (Rule 3); later received the remaining Task 2 doc-example migrations in the second commit

**Task 2 (43 files, ~99 literal sites; see `git show 338f44d4 --stat` for the exhaustive list):**
- `crates/paladin-ports/src/output/llm_port.rs`, `crates/paladin-llm/src/{openai,anthropic,deepseek,gemini}/adapter.rs`, `crates/paladin-llm/src/{compat/engine.rs,compat/types.rs,mock.rs,llm_analysis_service.rs}`, `crates/paladin-llm/src/{openai,anthropic}/vision.rs` (TokenUsage sites only — `VisionTokenUsage` untouched), `crates/paladin-llm/benches/llm_serialization_benchmarks.rs`
- `crates/paladin-battalion/src/grove_service.rs`, `crates/paladin-content/src/services/content_llm_analysis_service.rs`, `crates/paladin-memory/src/services/memory_extraction_service.rs`, `crates/paladin-eval/src/scripted_llm.rs`
- `crates/paladin-herald/src/{json_herald,markdown_herald,table_herald}.rs`
- `src/application/services/paladin/{planning_service,prompt_generation_service,temperature_service}.rs`
- `examples/herald_{custom_formatter,json_output,markdown_output,streaming}.rs`
- 15 files under `tests/` (integration, functional, unit, helpers)
- `.planning/phases/31-lossless-token-accounting/deferred-items.md` (new — logs the pre-existing `cli_isolation` / `--all-features` conflict)

## Decisions Made

- **Checkpoint pre-resolved as `proceed-as-decided`** (⚡ auto-selected first option per the
  orchestrator's `<checkpoint_resolution>` instruction): land the Phase 31 shape exactly as
  `31-CONTEXT.md` locks it — `TokenUsage` gains three `Option<u32>` sub-counts with
  `total_tokens` including them (D-01, D-02); the carrier field is named `usage` (D-07, to be
  implemented in a later plan); `StreamingResponse` gains `usage` and `#[non_exhaustive]`
  (D-13, later plan); the HTTP edge carries `TokenUsageResponse` (D-24, later plan). This plan
  (31-01) implements only the `TokenUsage` type itself and the workspace-wide literal migration.
- Fixed the one in-crate `TokenUsage` literal in `herald.rs`'s own test during Task 1 (Rule 3
  auto-fix: blocking issue) because `paladin-core` is a single compilation unit and Task 1's own
  verify command cannot pass while any literal in the same crate fails to compile.
- Kept provider usage structs' now-unread total fields (`OpenAIUsage.total_tokens`,
  `GeminiUsageMetadata.total_token_count`, `CompatUsage.total_tokens`) with `#[allow(dead_code)]`
  and a rationale comment, rather than deleting them, so the deserializer still matches the full
  provider wire shape for future debugging.
- `compat/engine.rs`'s `total_tokens` local (previously `.unwrap_or(prompt+completion)`) was
  removed outright rather than marked dead, since `TokenUsage::new` now always recomputes the
  identical sum — keeping a dead local would have needed its own suppression for no benefit.
- `mock.rs`'s `with_token_usage(prompt, completion, total)` keeps its three-parameter signature
  for source compatibility with existing call sites, but the third parameter is renamed
  `_total_tokens` and ignored, since `TokenUsage::new` now enforces the D-02 invariant
  unconditionally.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Fixed herald.rs's in-crate TokenUsage test literal during Task 1**
- **Found during:** Task 1
- **Issue:** Task 1's own verify command (`cargo test -p paladin-ai-core --lib token_usage`)
  requires the whole `paladin-ai-core` crate to compile, but `herald.rs`'s
  `test_finalize_stream` test still had a full three-field `TokenUsage { .. }` literal, which no
  longer compiles once the struct gains three additional fields.
- **Fix:** Migrated that one literal to `TokenUsage::new(300, 200)` (its total 500 already equalled
  the sum, so this preserves the exact test value).
- **Files modified:** `crates/paladin-core/src/platform/container/herald.rs`
- **Verification:** `cargo test -p paladin-ai-core --lib token_usage` — 22 passed, 0 failed
- **Committed in:** `524d59c7` (part of the Task 1 GREEN commit)

**2. [Rule 3 - Blocking issue] Fixed dead-code warnings introduced by discarding provider-reported totals**
- **Found during:** Task 2
- **Issue:** Migrating `TokenUsage { prompt_tokens, completion_tokens, total_tokens: <provider's
  own total> }` sites to `TokenUsage::new(prompt, completion)` made four provider-side total
  fields/locals genuinely unread (`OpenAIUsage.total_tokens`, `GeminiUsageMetadata
  .total_token_count`, `CompatUsage.total_tokens`, `compat/engine.rs`'s local `total_tokens`,
  and `mock.rs`'s `with_token_usage` `total_tokens` parameter), which `cargo clippy -- -D
  warnings` would fail on.
- **Fix:** Added `#[allow(dead_code)]` with a rationale comment to the three struct fields
  (preserves the deserializer's full wire-shape match), removed the now-fully-redundant local in
  `compat/engine.rs`, and renamed the unused mock parameter to `_total_tokens` (kept for source
  compatibility with existing call sites).
- **Files modified:** `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/gemini/adapter.rs`, `crates/paladin-llm/src/compat/types.rs`, `crates/paladin-llm/src/compat/engine.rs`, `crates/paladin-llm/src/mock.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0
- **Committed in:** `338f44d4` (part of the Task 2 commit)

**3. [Rule 3 - Blocking issue] Fixed 6 doc-comment literal sites that `cargo check` does not compile**
- **Found during:** Task 2 (surfaced only when running the full test suite, since `cargo check`
  skips doctests)
- **Issue:** `herald.rs` (4 sites) and `llm_port.rs` (2 sites) had `TokenUsage { .. }` literals
  inside `///`/`//!` doc-comment code examples. These compile only under `cargo test --doc`, so
  `cargo check --workspace --all-targets --all-features` — Task 2's primary discovery mechanism
  — did not surface them; they were caught by the subsequent full `cargo test --workspace
  --all-features` run.
- **Fix:** Migrated all 6 to `TokenUsage::new(prompt, completion)`; all totals already equalled
  the sum.
- **Files modified:** `crates/paladin-core/src/platform/container/herald.rs`, `crates/paladin-ports/src/output/llm_port.rs`
- **Verification:** `cargo test --workspace --all-features` — the 4 previously-failing
  `paladin-ai-core` doctests and 2 previously-failing `paladin-ports` doctests now pass
- **Committed in:** `338f44d4` (part of the Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 3 — blocking compilation/lint issues directly caused
by this plan's own field addition, discovered progressively as each verification layer ran).
**Impact on plan:** All three were necessary to keep the workspace compiling, testing, and
linting clean on the new six-field shape, as the plan's own acceptance criteria require. No scope
creep — no carrier types (`PaladinResult`, `NodeExecutionRecord`, `TraceEvent`, `StreamingResponse`,
HTTP DTOs) were touched; those remain scoped to plans 31-02 through 31-06.

## Issues Encountered

**Pre-existing, unrelated test/feature-flag conflict (not an issue with this plan's changes):**
`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `cargo test
--workspace --all-features` because the test asserts the `cli` feature is NOT active, while
`--all-features` (the plan's own verify command) necessarily activates it. Confirmed pre-existing
via `git diff --stat <base> -- tests/cli_isolation_test.rs` (empty — file untouched by this plan)
and by running `cargo test --test cli_isolation` without `--all-features` (passes: 9/9). Logged
to `.planning/phases/31-lossless-token-accounting/deferred-items.md` per the scope-boundary rule;
left unfixed as out of scope.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`TokenUsage`'s six-field shape, saturating arithmetic, and doc-tested builders are the load-bearing
type every later plan in this phase depends on (D-06: "every accumulator in the tree ... uses this
one implementation"). Plan 31-02 can now proceed to delete `TokenUsage::from_total` and fix the
Formation/Phalanx zeroing bug, since `from_total` was deliberately left in place by this plan and
every other in-tree caller of the old three-field literal shape has already been migrated to
`TokenUsage::new`. No blockers.

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
