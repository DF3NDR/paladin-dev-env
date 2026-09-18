---
phase: 31-lossless-token-accounting
plan: 06
subsystem: api
tags: [rust, http, openapi, sse, serde, token-usage, utoipa, tdd]

# Dependency graph
requires:
  - phase: 31-lossless-token-accounting
    provides: "Full TokenUsage carrier chain (PaladinResult.usage / NodeExecutionRecord.usage / TraceEvent::NodeFinished.usage / TraceEvent::RunFinished.usage) from plan 31-02"
provides:
  - "TokenUsageResponse (paladin-web DTO, six fields, utoipa::ToSchema, From<TokenUsage>) and ExecuteResponse.usage replacing the bare token_count on the published HTTP contract (D-24)"
  - "CompletedRow.usage: Option<TokenUsage> (core type) on the run-inspector port, replacing the bare token_count, following the same cache-hit None rule as duration_ms"
  - "map_trace_event's node_finished and run_finished SSE wire payloads now include a six-key usage object, proven by tests over the actual serialized JSON"
  - "Regenerated, diff-clean crates/paladin-web/openapi.json carrying the new response schema"
  - "A narrowly-scoped, documented Phase-31 exception in the SHIP-02 golden-diff gate (openapi_golden_v0_9.rs) for exactly the ExecuteResponse token_count -> usage divergence"
affects: [31-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Web-layer DTO with its own utoipa::ToSchema + From<CoreType> conversion at the HTTP boundary only -- paladin-core and paladin-ports never gain a utoipa/paladin-web dependency (hexagonal rule enforced by grep acceptance criteria, not just convention)"
    - "A sanctioned, narrowly-scoped exception to a golden-diff regression gate is itself proven narrow by a dedicated test (execute_response_exception_is_narrowly_scoped), mirroring the pre-existing info.version normalisation pattern rather than loosening the gate wholesale"

key-files:
  created: []
  modified:
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/openapi.json
    - crates/paladin-web/src/lib.rs
    - crates/paladin-ports/src/input/run_inspector_port.rs
    - src/application/services/run/inspector.rs
    - crates/paladin-web/src/dev_ui_controller.rs
    - src/application/services/run/events.rs
    - crates/paladin-web/tests/openapi_golden_v0_9.rs

key-decisions:
  - "TokenUsageResponse lives in agent_controller.rs beside ExecuteResponse (not a separate dto module) -- only one controller (agent_controller) constructs it this plan, so the plan's own 'shared module if more than one controller imports it' threshold was not crossed"
  - "map_trace_event's node_finished/run_finished payloads required an actual code change to carry usage -- the plan's own read_first assumption ('SSE events serialize TraceEvent directly') did not hold for this hand-built JSON payload; the TDD test written first proved the gap (RED) before the two-line fix (GREEN)"
  - "The SHIP-02 golden-diff gate (openapi_golden_v0_9.rs, Phase 29) failed as a direct, foreseeable consequence of this plan's own approved D-24/ADR-0051 break. Fixed via one narrowly-scoped, fully-documented exception (strip_known_v0_10_execute_response_divergence) that strips exactly the ExecuteResponse token_count/usage/TokenUsageResponse divergence before comparison -- mirroring the pre-existing info.version normalisation precedent -- rather than loosening or disabling the gate. A dedicated test proves every other ExecuteResponse field (and the six pre-existing v0.9 paths) still fails the gate on any other change."
  - "record_with_cost (inspector.rs test helper) now delegates to a new record_with_usage helper taking a full TokenUsage, so the cache-hit vs. executed test could exercise a genuine non-zero prompt+completion split (7, 3) instead of the pre-existing token_count-only shape (x, 0)"

patterns-established:
  - "A CompletedRow-shaped view type embeds the core TokenUsage directly rather than its own bare-count DTO, since paladin-ports already depends on paladin-core and the type crosses no framework boundary -- only the HTTP/OpenAPI-facing ExecuteResponse needed a dedicated utoipa DTO"

requirements-completed: [ACCT-02, ACCT-05]

coverage:
  - id: D1
    description: "TokenUsageResponse (six fields, utoipa::ToSchema, From<TokenUsage>) added in paladin-web; ExecuteResponse.usage replaces the bare token_count field, converted through the one From<PaladinResult> conversion point; paladin-core gains no utoipa dependency"
    requirement: "ACCT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::token_usage_response_from_token_usage_maps_all_six_fields_unchanged"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#tests::execute_response_serializes_usage_object_with_six_keys"
        status: pass
      - kind: other
        ref: "grep -c 'utoipa' crates/paladin-core/Cargo.toml (0 matches)"
        status: pass
    human_judgment: false
  - id: D2
    description: "crates/paladin-web/openapi.json is regenerated via make openapi in the same commit as the DTO change; the committed-baseline test passes and git diff --exit-code is clean"
    requirement: "ACCT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/openapi.rs#tests::openapi_matches_committed_baseline"
        status: pass
      - kind: other
        ref: "git diff --exit-code crates/paladin-web/openapi.json (after make openapi, exit 0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "CompletedRow.usage: Option<TokenUsage> replaces the bare token_count on the run-inspector port (core type, no paladin-web dependency introduced into paladin-ports); None on a cache-hit, Some with a real prompt+completion split on an executed attempt"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/inspector.rs#tests::completed_row_partial_values_are_representable"
        status: pass
      - kind: other
        ref: "grep -c 'paladin-web' crates/paladin-ports/Cargo.toml (0 matches)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The dev-UI InspectorView page embeds the full six-key usage object for an executed row and null for a cache-served one, in the actual rendered page JSON"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#tests::dev_ui_page_embeds_cache_hit_row_with_no_duration_or_tokens"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/dev_ui_controller.rs#tests::dev_ui_page_embeds_executed_row_with_full_usage_object"
        status: pass
    human_judgment: false
  - id: D5
    description: "SSE node_finished and run_finished wire payloads (map_trace_event) carry a usage object with all six TokenUsage keys, proven against the actual serialized JSON"
    requirement: "ACCT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::node_finished_payload_carries_six_key_usage_object"
        status: pass
      - kind: unit
        ref: "src/application/services/run/events.rs#tests::run_finished_payload_carries_six_key_usage_object"
        status: pass
    human_judgment: false
  - id: D6
    description: "Full workspace compiles, tests, formats and lints clean on the new DTO/port/payload shapes (cargo check/test/fmt/clippy --workspace --all-targets --all-features, plus make clean-code); the pre-existing SHIP-02 golden-diff gate stays green via one narrowly-scoped, proven-narrow exception"
    requirement: "ACCT-05"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md by plan 31-01)"
        status: pass
      - kind: other
        ref: "cargo fmt --all -- --check (exit 0)"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: other
        ref: "make clean-code (fmt + clippy + shellcheck + check, exit 0)"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/tests/openapi_golden_v0_9.rs (all 7 tests pass, including new execute_response_exception_is_narrowly_scoped)"
        status: pass
    human_judgment: false

duration: ~1h10m
completed: 2026-09-15
status: complete
---

# Phase 31 Plan 06: HTTP Edge Carries the Full TokenUsage Split (ExecuteResponse, CompletedRow, SSE) Summary

**`ExecuteResponse.usage: TokenUsageResponse` replaces the bare `token_count` on the published HTTP contract, `CompletedRow.usage: Option<TokenUsage>` follows the same shape on the run-inspector port, SSE `node_finished`/`run_finished` payloads now carry the same six-key object, and `openapi.json` is regenerated diff-clean — with a narrowly-scoped, test-proven exception keeping the pre-existing SHIP-02 golden-diff gate green.**

## Performance

- **Duration:** ~1h10m
- **Tasks:** 2 (one `tracer` task with TDD, one `auto` task with TDD)
- **Files modified:** 8 across 2 commits

## Accomplishments

- **Task 1 (tracer, TDD) — the HTTP edge.** `TokenUsageResponse` (six public fields mirroring `TokenUsage`, `Serialize`/`Deserialize`/`utoipa::ToSchema`, `From<TokenUsage>`) added in `paladin-web/agent_controller.rs`. `ExecuteResponse.usage` replaces the bare `token_count: u32` field; its `From<PaladinResult>` impl converts `result.usage` through `TokenUsageResponse::from` at the one conversion point. `paladin-core` gains no `utoipa` dependency (verified by grep, not just convention). `crates/paladin-web/openapi.json` regenerated via `make openapi` in the same commit; the committed-baseline drift-guard test passes with no further edits needed. New tests written first (RED, confirmed failing to compile before the struct existed): the `From` conversion field-for-field including all three optional sub-counts, and a serialization test asserting the six-key usage object with `null` for unreported optionals against the real JSON.
- **Tracer feedback gate:** re-ran Task 1's full `<verify>` end-to-end (unit tests + `make openapi` + baseline test + `git diff --exit-code`) before starting Task 2 — all green, confirmed the proven slice before expanding.
- **Task 2 (auto, TDD) — inspector, dev-UI, SSE.** `CompletedRow.usage: Option<TokenUsage>` (the core type, not a web DTO) replaces the bare `token_count: Option<u64>` in `paladin-ports/run_inspector_port.rs` — `paladin-ports` gains no `paladin-web` dependency. `inspector.rs` maps the record's usage under the same cache-hit `None` rule `duration_ms` already used; the existing partial-values test was extended to exercise a genuine non-zero prompt+completion split (`(7, 3)`) via a new `record_with_usage` test helper, rather than only ever asserting a single total. `dev_ui_controller.rs`'s fixtures/tests followed the field rename — the rendered page's embedded `InspectorView` JSON carries the full six-key object for an executed row and `null` for a cache-served one. `events.rs`'s `map_trace_event` required an actual two-field code change (not just a read-through, as the plan's own `read_first` assumption suggested) to carry `usage` on the hand-built `node_finished`/`run_finished` wire payloads — the TDD test written first proved this gap (RED: the payload had no `usage` key) before the fix (GREEN).
- **Deviation, fixed inline (Rule 3):** the pre-existing SHIP-02 golden-diff gate (`openapi_golden_v0_9.rs`, Phase 29) failed as a direct, foreseeable consequence of Task 1's own approved, one-way-door-confirmed `ExecuteResponse` break (D-24/ADR-0051). Added one narrowly-scoped, fully-documented exception (`strip_known_v0_10_execute_response_divergence`) that strips exactly the `token_count`/`usage`/`TokenUsageResponse` divergence from both documents' `ExecuteResponse` schema before the ref-closure comparison — mirroring the pre-existing `info.version` normalisation pattern rather than loosening the gate wholesale. A new dedicated test (`execute_response_exception_is_narrowly_scoped`) proves the exception strips exactly those keys and nothing else, and that every other `ExecuteResponse` field survives.
- Full workspace: `cargo check/test/fmt/clippy --workspace --all-targets --all-features` and `make clean-code` all green, except the pre-existing, unrelated `cli_isolation`/`--all-features` conflict plan 31-01 already logged in `deferred-items.md`.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer, TDD): TokenUsageResponse DTO on ExecuteResponse, OpenAPI baseline regenerated** - `3646cbb3` (feat)
2. **Task 2 (auto, TDD): CompletedRow.usage, SSE usage payloads, dev-UI row follow the split** - `c11c5664` (feat) — includes the Rule 3 SHIP-02 gate fix

**Plan metadata:** this SUMMARY.md commit (docs, worktree mode — orchestrator handles the final metadata commit after merge)

## Files Created/Modified

**Task 1 (2 files):**
- `crates/paladin-web/src/agent_controller.rs` — `TokenUsageResponse` DTO, `ExecuteResponse.usage`, updated `From<PaladinResult>`, new tests
- `crates/paladin-web/openapi.json` — regenerated baseline carrying the new response schema

**Task 2 (6 files):**
- `crates/paladin-ports/src/input/run_inspector_port.rs` — `CompletedRow.usage: Option<TokenUsage>`
- `src/application/services/run/inspector.rs` — usage mapping under the cache-hit rule, `record_with_usage` test helper, updated partial-values test
- `crates/paladin-web/src/dev_ui_controller.rs` — fixture/test updates for the field rename, new full-usage-object test
- `src/application/services/run/events.rs` — `map_trace_event`'s `NodeFinished`/`RunFinished` arms now include `usage`; two new payload tests
- `crates/paladin-web/src/lib.rs` — re-export `TokenUsageResponse` alongside `ExecuteResponse`
- `crates/paladin-web/tests/openapi_golden_v0_9.rs` — narrowly-scoped Phase 31 exception + its own proof test

## Decisions Made

- `TokenUsageResponse` placed beside `ExecuteResponse` in `agent_controller.rs` rather than a shared `dto` module — only one controller constructs it this plan, so the plan's own "shared module if more than one controller imports it" threshold was not crossed.
- `map_trace_event`'s SSE payloads needed a real code change to carry `usage` — confirmed by reading the code (it hand-builds a restricted JSON object per wire event, not a direct `TraceEvent` serialization as the plan's `read_first` note assumed) and proven by a failing test before the two-field fix.
- The SHIP-02 golden-diff gate's failure was treated as a Rule 3 blocking-issue auto-fix (a direct, foreseeable consequence of Task 1's own approved, checkpoint-confirmed break), not a Rule 4 architectural question — the underlying decision to break `ExecuteResponse`'s wire shape was already made and confirmed at plan 31-01's consolidated checkpoint; what remained was the mechanical, narrowly-scoped update to the existing gate, proven not to weaken it for anything else.
- `record_with_cost` (inspector.rs test helper) now delegates to a new `record_with_usage` helper taking a full `TokenUsage`, letting the cache-hit/executed test assert a genuine non-zero prompt+completion split instead of only a total.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] SSE map_trace_event required a code change to carry `usage` (plan's own read_first assumption did not hold)**
- **Found during:** Task 2
- **Issue:** The plan's `read_first` note said to "confirm ... SSE run events serialize TraceEvent directly ... with no further code change." Reading `events.rs` showed `map_trace_event` hand-builds a restricted JSON payload per wire event (7 named kinds out of `TraceEvent`'s 12 variants) — it does NOT serialize `TraceEvent` directly, and neither `NodeFinished` nor `RunFinished`'s hand-built payload included `usage`.
- **Fix:** Added `usage` to the destructured pattern and JSON payload for both `TraceEvent::NodeFinished` and `TraceEvent::RunFinished` arms of `map_trace_event`. The degraded-mode `terminal_payload` function (a separate, `Run`-sourced synthesis path with no `TraceEvent` and no usage field available) was deliberately left untouched — out of scope, no data source exists there.
- **Files modified:** `src/application/services/run/events.rs`
- **Verification:** Two new tests (`node_finished_payload_carries_six_key_usage_object`, `run_finished_payload_carries_six_key_usage_object`) assert the six-key object against the real serialized JSON; `cargo test -p paladin-ai --lib run::events` passes
- **Committed in:** `c11c5664` (Task 2 commit)

**2. [Rule 3 - Blocking issue] Pre-existing SHIP-02 golden-diff gate failed as a direct consequence of this plan's own approved break**
- **Found during:** Task 2's `cargo test -p paladin-web` verify step
- **Issue:** `crates/paladin-web/tests/openapi_golden_v0_9.rs` (Phase 29, SHIP-02) asserts the six pre-existing v0.9 `/v1/agents/*` paths' transitive schema closure is byte-identical to the frozen `v0.9.0` baseline. Task 1's approved `ExecuteResponse.token_count -> usage` break (D-24, ADR-0051, one-way door confirmed at plan 31-01's checkpoint) is exactly the kind of change this gate exists to catch, and the frozen baseline file itself is immutable by the file's own documented design (no regeneration escape hatch).
- **Fix:** Added `strip_known_v0_10_execute_response_divergence`, a narrowly-scoped normalisation applied to both closures before comparison — removes exactly `ExecuteResponse.token_count`, `ExecuteResponse.usage`, and the new `TokenUsageResponse` schema, leaving every other field and every other schema subject to the gate's full power. Documented in the file's own module docs alongside the pre-existing `info.version` exception. Added `execute_response_exception_is_narrowly_scoped`, a dedicated test proving the exception strips exactly those keys (and that `output`/`execution_time_ms`/`loop_count`/`stop_reason` all survive).
- **Files modified:** `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- **Verification:** `cargo test -p paladin-web --test openapi_golden_v0_9` — all 7 tests pass (5 pre-existing + 1 new proof test); `cargo test -p paladin-web` green overall
- **Committed in:** `c11c5664` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking issues directly caused by either this plan's own field renames or its approved, checkpoint-confirmed breaking change).
**Impact on plan:** Both were necessary to keep `cargo test -p paladin-web` and the workspace-wide verification green, exactly as the plan's own acceptance criteria and `<verification>` section require. No scope creep: the SHIP-02 gate's general power (catching any OTHER unintentional break) is unchanged and proven so by a dedicated test; no schema outside the one named exception was touched.

## Issues Encountered

**Pre-existing, unrelated test/feature-flag conflict (not an issue with this plan's changes), already logged by plan 31-01:**
`tests/cli_isolation_test.rs::test_cli_feature_is_not_default` fails under `cargo test --workspace --all-features` because the test asserts the `cli` feature is NOT active, while `--all-features` (this plan's own verify command) necessarily activates it. Confirmed pre-existing and already documented in `.planning/phases/31-lossless-token-accounting/deferred-items.md`; left unfixed as out of scope for this plan too.

## Known Stubs

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

The HTTP edge, the run-inspector port, the dev-UI page and the SSE trace stream all now carry the full `TokenUsage` split rather than a bare count — no carrier collapses the split above the core chain plan 31-02 built. `crates/paladin-web/openapi.json` is regenerated and diff-clean, ready for plan 31-07's MIGRATION.md §9.2/§9.6 rows (`paladin-web | ExecuteResponse` gets a semver-checks allowlist entry; `paladin-ports | CompletedRow` gets an N/A completeness row, no allowlist entry, per 31-CONTEXT.md's correction of the original citation) and CHANGELOG.md entry — this plan did not touch either file, by design (31-07's explicit job). No blockers.

## Self-Check: PASSED

- FOUND: `crates/paladin-web/src/agent_controller.rs`
- FOUND: `crates/paladin-web/openapi.json`
- FOUND: `crates/paladin-ports/src/input/run_inspector_port.rs`
- FOUND: `src/application/services/run/inspector.rs`
- FOUND: `crates/paladin-web/src/dev_ui_controller.rs`
- FOUND: `src/application/services/run/events.rs`
- FOUND: `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- FOUND commit `3646cbb3` (feat: Task 1 TokenUsageResponse DTO)
- FOUND commit `c11c5664` (feat: Task 2 CompletedRow/SSE/dev-UI + SHIP-02 exception)

---
*Phase: 31-lossless-token-accounting*
*Completed: 2026-09-15*
