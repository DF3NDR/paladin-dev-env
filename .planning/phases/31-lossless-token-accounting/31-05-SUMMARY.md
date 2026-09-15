---
phase: 31-lossless-token-accounting
plan: 05
subsystem: observability
tags: [rust, serde, token-usage, herald, cli, mdbook, tdd]

# Dependency graph
requires:
  - phase: 31-lossless-token-accounting
    provides: "Full TokenUsage carrier chain (PaladinResult.usage, NodeExecutionRecord.usage, TraceEvent::NodeFinished/RunFinished.usage) from plan 31-02, and the streaming usage parity from plan 31-03/31-04"
provides:
  - "JsonHerald's stable six-key `usage` object (prompt/completion/total always, cache-read/cache-write/reasoning `null` when unreported) for a PaladinResult and for ExecutionMetadata, replacing the bare token_count/total_tokens keys"
  - "MarkdownHerald's 'Token Usage' block (Prompt/Completion/Total always, Cache read/Cache write/Reasoning only when Some) and its per-Paladin usage table for a BattalionResult"
  - "A shared format_token_usage_summary() CLI helper rendering the split, used by output.rs's human text and by agent.rs/battalion.rs's Formation/Phalanx/Conclave per-Paladin lines"
  - "output.rs's JSON formatters emitting the full usage object instead of a bare token_count scalar"
  - "Nine of the eleven grep-hit mdBook pages updated to the usage carrier; the paladin_port.rs rustdoc field list corrected"
affects: [31-06, 31-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TableHerald left untouched by explicit plan discretion: its tokens column and its (execution_time_ms, usage.total_tokens) battalion name-matching key already read the correct carrier from plan 31-02 -- widening the key to the full struct would change which rows collide (RESEARCH.md Pitfall 2)"
    - "A derived bare total kept beside a full usage object (JSON finalize_stream, and BattalionResult.total_tokens) is always paired with a test asserting the two cannot drift (D-08)"
    - "One shared format_token_usage_summary() helper in src/application/cli/formatters/output.rs, reused by five separate CLI render sites, instead of five ad hoc string templates"

key-files:
  created: []
  modified:
    - crates/paladin-herald/src/json_herald.rs
    - crates/paladin-herald/src/markdown_herald.rs
    - src/application/cli/formatters/output.rs
    - src/application/cli/formatters/mod.rs
    - src/application/cli/commands/agent.rs
    - src/application/cli/commands/battalion.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - tests/integration/herald_integration_test.rs
    - docs/src/getting-started/quickstart.md
    - docs/src/operations/observability.md
    - docs/src/user-guides/output-formatting.md
    - docs/src/user-guides/agent-orchestrator-bridge.md
    - docs/src/user-guides/battalion-patterns.md
    - docs/src/user-guides/herald-output.md
    - docs/src/user-guides/orchestration.md
    - docs/src/user-guides/paladin-agents.md
    - docs/src/appendix/conclave-pattern.md

key-decisions:
  - "TableHerald and its battalion name-matching pool key stay untouched (plan discretion): the tokens column already reads usage.total_tokens and widening the key would change row-matching semantics"
  - "The Markdown per-Paladin usage table renders an empty cell (not an omitted row) for an unreported optional, since a table's column set is fixed -- omission is per-cell rather than per-row there, vs. per-row omission in the single-result Token Usage block"
  - "The CLI's format_token_usage_summary() intentionally omits the standalone word 'tokens' redundancy where a caller's own label already implies it (e.g. 'Tokens Used: 1801 tokens (...)' reads acceptably; exact wording was left to the executor per the plan's own 'Exact text is yours')"
  - "A derived bare total is kept beside the JSON usage object in finalize_stream (permitted by D-08) with an equality assertion so the two representations cannot silently diverge"

patterns-established:
  - "Presentation-layer usage rendering (JSON keys, Markdown blocks/tables, CLI text) always derives from the six-field TokenUsage struct directly -- no presentation surface reconstructs or approximates a split from a bare total"

requirements-completed: [ACCT-04]

coverage:
  - id: D1
    description: "JsonHerald emits the full six-key usage object (with explicit nulls for unreported optionals) for a PaladinResult and for ExecutionMetadata; per_paladin_tokens entries carry real non-zero splits; the object round-trips into an equal TokenUsage"
    requirement: "ACCT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-herald/src/json_herald.rs#tests::test_usage_object_key_set_is_stable_with_nulls_when_optionals_unreported"
        status: pass
      - kind: unit
        ref: "crates/paladin-herald/src/json_herald.rs#tests::test_per_paladin_tokens_carry_real_non_zero_splits"
        status: pass
      - kind: unit
        ref: "crates/paladin-herald/src/json_herald.rs#tests::test_usage_object_deserializes_back_into_equal_token_usage"
        status: pass
    human_judgment: false
  - id: D2
    description: "MarkdownHerald renders a 'Token Usage' block with Prompt/Completion/Total always and Cache read/Cache write/Reasoning only when Some, plus a per-Paladin usage table under a BattalionResult's total-tokens summary"
    requirement: "ACCT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-herald/src/markdown_herald.rs#tests::test_token_usage_block_omits_unreported_optionals"
        status: pass
      - kind: unit
        ref: "crates/paladin-herald/src/markdown_herald.rs#tests::test_token_usage_block_renders_optionals_when_reported"
        status: pass
      - kind: unit
        ref: "crates/paladin-herald/src/markdown_herald.rs#tests::test_per_paladin_usage_table_renders_real_non_zero_splits"
        status: pass
    human_judgment: false
  - id: D3
    description: "The CLI prints a total with its prompt/completion split (cache/reasoning appended only when reported) in human-readable output, and its JSON mode emits the usage object rather than a bare number"
    requirement: "ACCT-04"
    verification:
      - kind: unit
        ref: "src/application/cli/formatters/output.rs#tests::test_format_paladin_result_human_output_shows_split"
        status: pass
      - kind: unit
        ref: "src/application/cli/formatters/output.rs#tests::test_format_paladin_result_json_emits_usage_object_not_scalar"
        status: pass
      - kind: unit
        ref: "src/application/cli/formatters/output.rs#tests::test_format_battalion_result_json_emits_usage_object_per_paladin"
        status: pass
    human_judgment: false
  - id: D4
    description: "None of the eleven grep-hit documentation pages shows a stale bare Paladin/battalion result token count or names a token-carrier field that does not exist on the post-phase result types; doctests, the mdBook build and cargo doc all run clean modulo pre-existing out-of-scope warnings"
    requirement: "ACCT-04"
    verification:
      - kind: other
        ref: "grep -rn 'token_count' docs/src --include='*.md' | grep -v memory-management.md | grep -v domain-model.md | grep -v api-reference/ (0 matches)"
        status: pass
      - kind: other
        ref: "grep -n 'token_usage: TokenUsage' docs/src/user-guides/battalion-patterns.md (0 matches)"
        status: pass
      - kind: other
        ref: "cargo test --workspace --doc --all-features (exit 0); mdbook build docs/ (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~49min
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 05: Herald and CLI Token-Split Presentation Summary

**JsonHerald's stable six-key usage object, MarkdownHerald's Token Usage block and per-Paladin table, and a shared CLI helper make the prompt/completion/cache/reasoning breakdown observable end to end, with nine of eleven grep-hit mdBook pages brought current.**

## Performance

- **Duration:** ~49 min
- **Tasks:** 3 (one `tracer` task with TDD, one `auto` task with TDD, one `auto` task)
- **Files modified:** 18 across 4 commits

## Accomplishments

- **Task 1 (tracer, TDD) — JsonHerald's usage object.** `paladin_result_to_json` replaces the bare `token_count` key with a `usage` key carrying the full `TokenUsage` object; `finalize_stream` emits the full `ExecutionMetadata.token_usage` object under `usage`, keeping the pre-existing derived `total_tokens` beside it (D-08) with a test proving the two can never drift. New tests prove the object always carries exactly six keys with the three optionals `null` when unreported, that `per_paladin_tokens` entries carry real non-zero prompt/completion splits, and that the emitted object round-trips into an equal `TokenUsage`. Verified end-to-end (tracer feedback gate) before expanding to Task 2.
- **Task 2 (auto, TDD) — MarkdownHerald, TableHerald and the CLI.** `MarkdownHerald::format_paladin_result` and `finalize_stream` replace the single "Token Count" field with a "Token Usage" block: Prompt/Completion/Total always render, Cache read/Cache write/Reasoning render only when `Some` (an unreported figure is omitted, never a placeholder). `format_battalion_result` gains a per-Paladin usage table (name, prompt, completion, total, cache read, cache write, reasoning) under the existing total-tokens summary, with an empty cell (not an omitted row) for an unreported optional. `TableHerald` is deliberately untouched — its tokens column and its `(execution_time_ms, usage.total_tokens)` battalion name-matching key already read the right carrier from plan 31-02, and widening the key would change which rows collide (RESEARCH.md Pitfall 2). A new `format_token_usage_summary()` helper in `output.rs` renders `"N tokens (prompt P, completion C[, cache read/write/reasoning])"`, reused by `output.rs`'s human-readable paladin/battalion text and by `agent.rs`'s and `battalion.rs`'s Formation/Phalanx/Conclave per-Paladin loop lines. `output.rs`'s `format_paladin_result_json`/`format_battalion_result_json` emit the full `usage` object instead of a bare `token_count` scalar.
- **Task 3 (auto) — documentation currency.** Corrected the stale `PaladinPort::execute` rustdoc field list (`token_count` → `usage: TokenUsage`) and updated nine of the eleven grep-hit mdBook pages (`memory-management.md` and `domain-model.md` are Garrison-entry `token_count` references this phase does not touch, confirmed unchanged) so no page shows a bare result token count or a non-existent token-carrier field. `battalion-patterns.md`'s `BattalionResult` field-list sentence was rewritten against the real struct (`final_output`, `per_paladin_tokens: HashMap<String, TokenUsage>`, `total_tokens: u64` — no `execution_time_ms` or `token_usage` field exists on that type). `herald-output.md` and `output-formatting.md`'s rendered JSON examples now show the full six-key object with explicit nulls. Logged 16 pre-existing `cargo doc` warnings (private intra-doc links, unclosed HTML tags, none in files this plan touches) to `deferred-items.md` and the `WINDOWS.md` ledger rather than silently treating the plan's "zero warnings" criterion as met.
- **Post-Task-3 regression fix.** `cargo test --workspace --all-features` surfaced two failures in `tests/integration/herald_integration_test.rs` (not in this plan's declared `files_modified`) asserting the retired `token_count`/`Token Count` field/text — a direct, in-scope consequence of Tasks 1-2's own carrier changes (Rule 1). Fixed both assertions to the new `usage` object / "Token Usage" block; confirmed passing along with the rest of the workspace (`cargo test --workspace --all-features` clean except the pre-existing, already-documented `test_cli_feature_is_not_default` conflict from plan 31-01's `deferred-items.md`).
- Full verification: `cargo test -p paladin-herald` (both default and `table` feature) green; `cargo test -p paladin-ai --lib --features cli` green (210 tests); `cargo test --workspace --doc --all-features` green; `mdbook build docs/` exits 0; `cargo doc --workspace --no-deps --all-features` exits 0 (16 pre-existing, out-of-scope warnings, documented); `cargo test --workspace --all-features --no-fail-fast` green except the one pre-existing `cli_isolation` failure; `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `make clean-code` exits 0.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer, TDD): JsonHerald's stable six-key usage object** - `cd3c7a6c` (feat)
2. **Task 2 (auto, TDD): MarkdownHerald, TableHerald and CLI render the split** - `421c34c3` (feat)
3. **Task 3 (auto): mdBook and rustdoc currency** - `d8a763e4` (docs)
4. **Post-Task-3 Rule 1 fix: herald integration test assertions** - `0fc736a5` (fix)

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the final metadata commit after merge)

## Files Created/Modified

- `crates/paladin-herald/src/json_herald.rs` — `usage` object replaces `token_count`/derived-only `total_tokens`; six new tests
- `crates/paladin-herald/src/markdown_herald.rs` — "Token Usage" block, per-Paladin usage table; four new tests
- `src/application/cli/formatters/output.rs` — `format_token_usage_summary()` helper; JSON formatters emit `usage`; new test module
- `src/application/cli/formatters/mod.rs` — re-exports `format_token_usage_summary`
- `src/application/cli/commands/agent.rs`, `src/application/cli/commands/battalion.rs` — human-readable per-Paladin lines use the shared helper
- `crates/paladin-ports/src/output/paladin_port.rs` — `PaladinPort::execute`'s doc comment field list corrected
- `tests/integration/herald_integration_test.rs` — assertions updated to the `usage` object / "Token Usage" block (Rule 1 fix)
- Nine mdBook pages under `docs/src/` — see frontmatter `key-files.modified` for the full list
- `.planning/phases/31-lossless-token-accounting/deferred-items.md` — logged the pre-existing `cargo doc` warnings

## Decisions Made

- TableHerald and its battalion name-matching pool key stay untouched (plan discretion) — already correct from plan 31-02.
- Markdown's per-Paladin table renders an empty cell (not an omitted row) for an unreported optional, since a table's column set is fixed — contrasted with the single-result block's per-row omission.
- `format_token_usage_summary()` intentionally does not fight every caller's own label wording for redundancy; the plan explicitly left exact text to the executor.
- Kept the derived bare `total_tokens` beside the JSON `usage` object in `finalize_stream` (D-08 permits coexistence), with a same-commit test asserting the two agree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] Switched from the shared `/workspace/target` build cache to this worktree's local `target/`**
- **Found during:** Task 2 verification
- **Issue:** `cargo test -p paladin-ai --lib --features cli` failed to compile against a cached `paladin-ports` rlib whose `CompletedRow` struct had already been reshaped by the concurrently-running sibling worktree (plan 31-06, which owns `crates/paladin-ports/src/input/run_inspector_port.rs`). The shared `CARGO_TARGET_DIR=/workspace/target` was serving a build artifact from the sibling's in-progress edit to my own compile of `src/application/services/run/inspector.rs` (untouched by this plan), producing `E0560`/`E0609` "no field `token_count`" errors against source that, read directly, still had the field.
- **Fix:** Verified the hypothesis by rebuilding the identical command with a local (default) `CARGO_TARGET_DIR` — the build succeeded cleanly. Used this worktree's local `target/` for all subsequent cargo invocations in this plan rather than the shared cache.
- **Files modified:** None (build configuration only, no source changed).
- **Verification:** `cargo check -p paladin-ai --lib --features cli` and `cargo test -p paladin-ai --lib --features cli cli` both pass cleanly with the local target dir.
- **Committed in:** N/A (build-environment workaround, not a commit).

**2. [Rule 1 - Bug] Fixed two herald integration test assertions broken by this plan's own carrier changes**
- **Found during:** Post-Task-3 full-workspace verification (`cargo test --workspace --all-features`)
- **Issue:** `tests/integration/herald_integration_test.rs::test_paladin_with_json_herald` and `::test_paladin_with_markdown_herald` asserted the retired `token_count` JSON key and `"Token Count"` Markdown text, both replaced by Tasks 1-2's `usage` object and "Token Usage" block. Not in this plan's declared `files_modified`, but a direct regression from this plan's own changes.
- **Fix:** Updated both assertions to check for the `usage` key (asserting it is a JSON object, not a scalar) and the `"Token Usage"` block text respectively.
- **Files modified:** `tests/integration/herald_integration_test.rs`
- **Verification:** `cargo test -p paladin-ai --test lib --all-features integration::herald_integration_test` (8/8 pass); full `cargo test --workspace --all-features --no-fail-fast` re-run clean except the pre-existing `cli_isolation` failure.
- **Committed in:** `0fc736a5`

---

**Total deviations:** 2 (1 Rule 3 — build-environment workaround, no source change; 1 Rule 1 — self-caught regression in an undeclared file directly caused by this plan's own carrier changes, fixed and verified).
**Impact on plan:** Both were necessary to reach a genuinely green workspace. No scope creep — the Rule 1 fix touched only the two broken assertions, and the Rule 3 workaround changed no source at all.

## Issues Encountered

**Pre-existing, unrelated test/feature-flag conflict (not caused by this plan), already logged by plan 31-01:**
`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `cargo test --workspace --all-features` because the test asserts the `cli` feature is NOT active, while `--all-features` necessarily activates it. Confirmed still pre-existing and unrelated (this plan touches no file the test exercises); left unfixed as out of scope, consistent with plan 31-01's and 31-02's prior treatment.

**Pre-existing, out-of-scope `cargo doc` warnings:** 16 warnings across `paladin-llm` and `paladin-ai` (private intra-doc links, unclosed HTML tags) in files this plan does not touch and did not modify. Logged to `deferred-items.md` and the `WINDOWS.md` ledger (`unmet-truth`, phase 31) rather than silently treating the plan's "zero warnings" verification criterion as satisfied.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

ACCT-04 is satisfied: the prompt/completion/cache/reasoning breakdown is observable end to end in JSON (stable six-key object with explicit nulls), Markdown (omission of unreported figures, per-Paladin table), and the CLI (split rendering in both human and JSON modes). Plan 31-06 (the HTTP edge — `paladin-web` DTOs, `openapi.json`, `run_inspector_port.rs`, `dev_ui_controller.rs`) runs independently and was not touched here; its own `CompletedRow` reshape was the source of the transient build-cache collision documented above, not a functional dependency on this plan's work. Plan 31-07's MIGRATION.md/docs work (the `v0.11.0` version bump and `api-reference/upgrading.md`/`migration-guide.md` rows) is unblocked; nothing in this plan wrote `v0.11.0` anywhere. No blockers.

## Self-Check: PASSED

- FOUND: `crates/paladin-herald/src/json_herald.rs`
- FOUND: `crates/paladin-herald/src/markdown_herald.rs`
- FOUND: `src/application/cli/formatters/output.rs`
- FOUND: `src/application/cli/commands/agent.rs`
- FOUND: `src/application/cli/commands/battalion.rs`
- FOUND: `crates/paladin-ports/src/output/paladin_port.rs`
- FOUND: `tests/integration/herald_integration_test.rs`
- FOUND: `docs/src/user-guides/battalion-patterns.md`
- FOUND commit `cd3c7a6c` (feat: Task 1 JsonHerald usage object)
- FOUND commit `421c34c3` (feat: Task 2 Markdown/CLI split rendering)
- FOUND commit `d8a763e4` (docs: Task 3 mdBook/rustdoc currency)
- FOUND commit `0fc736a5` (fix: herald integration test assertions)

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
