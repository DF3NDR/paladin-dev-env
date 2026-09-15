---
phase: 31-lossless-token-accounting
verified: 2026-09-15T09:30:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 31: Lossless Token Accounting Verification Report

**Phase Goal:** The provider's prompt/completion split survives from the LLM port to `RunFinished`
and a herald — no carrier above the port collapses `TokenUsage` to a bare total any more,
`TokenUsage` gains optional cache and reasoning fields, and streamed runs report the same usage as
non-streamed ones — so that everything cost-shaped (currency pricing, the Treasurer) becomes
buildable on the shipped shape. This is the keystone phase: Phase 32 depends on it and Milestone 14
will.

**Verified:** 2026-09-15
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria, mapped 1:1 to ACCT-01..05)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `TokenUsage` carries `cache_read_tokens`/`cache_write_tokens`/`reasoning_tokens` as `#[serde(default)] Option<u32>`, documents the inclusive-total contract and both inequalities, legacy JSON deserialises to `None`, a six-key document round-trips (ACCT-01) | VERIFIED | `crates/paladin-core/src/platform/container/token_usage.rs:1-330` — fields, rustdoc inequalities, builders, saturating `Add`/`AddAssign`/`Sum`, `legacy_json_without_optionals_deserialises_to_none`, `six_key_json_round_trips_and_serialises_all_six_keys`, `none_optionals_serialise_as_null_not_omitted`. `cargo test -p paladin-ai-core --lib token_usage`: 21/21 passed (run live) |
| 2 | `PaladinResult`, `BattalionResult.per_paladin_tokens`, `NodeExecutionRecord`, `TraceEvent::NodeFinished`/`RunFinished` carry a full `TokenUsage`; `from_total` gone; round-trip proves a non-zero split reaches `RunFinished` intact; real per-Paladin splits (ACCT-02) | VERIFIED | `execution_result.rs:61` `pub usage: TokenUsage`; `waypoint.rs:578` same; `trace.rs:237,292` `NodeFinished`/`RunFinished` `usage: TokenUsage`; `hooks.rs` `Mutex<TokenUsage>` accumulator with `PoisonError::into_inner` recovery, `total_usage()`; `engine/mod.rs` 5× `usage: trace.total_usage()`; `formation_service.rs:229-230` / `phalanx_service.rs:278-279` insert `result.usage.clone()` into `per_paladin_tokens` and add `total_tokens`. `grep -rn 'from_total'` across crates/src/tests/examples/benches returns zero matches |
| 3 | Every LLM adapter's streaming path is audited: streamed usage equals non-streamed usage per adapter, or the exception is documented (ACCT-03) | VERIFIED | `StreamingResponse.usage: Option<TokenUsage>` + `#[non_exhaustive]` + `delta`/`terminal`/`with_usage` constructors (`llm_port.rs:1052-1126`); `ChunkMetadata.usage` + `#[non_exhaustive]` (`paladin_port.rs:419-433`); `compat/engine.rs` hold-and-emit + `stream_options.include_usage`; Anthropic `cache_read_input_tokens`/`cache_creation_input_tokens`/`thinking_tokens` mapping with the billed cache-inclusive `prompt_tokens` correction, accumulated to `message_stop`; Gemini `cached_content_token_count`/`thoughts_token_count`, last-frame terminal usage, `Default` derive intact; shared conformance case `streaming_usage_equals_non_streaming_usage`, `CASE_COUNT == 9` (confirmed via `grep`); `cargo test -p paladin-llm --lib conformance`: 19/19 passed (run live, includes the parity case) |
| 4 | The breakdown is observable end-to-end in at least one herald in both JSON and Markdown output (ACCT-04) | VERIFIED | `json_herald.rs` emits `"usage": result.usage` and `"usage": metadata.token_usage`, zero `skip_serializing_if` occurrences, six-key stability tests; `markdown_herald.rs` renders a "Token Usage" block (Prompt/Completion/Total always, Cache read/Cache write/Reasoning only when `Some`) plus a "Per-Paladin Token Usage" table; CLI (`output.rs`) prints total + split + optional cache/reasoning lines |
| 5 | Every touched public type has a `MIGRATION.md` §9.2 row and a matching `cargo semver-checks` allowlist row (row-level gate green); `CHANGELOG.md` `[0.10.0]` records the carrier change; `make clean-code` and the 82% coverage floor are green (ACCT-05) | VERIFIED | `MIGRATION.md` §9.2 rows for `TokenUsage`, extended `PaladinResult`, `StreamingResponse`, `ChunkMetadata`, `ExecuteResponse`, plus N/A completeness rows for `NodeExecutionRecord`/`TraceEvent`/`CompletedRow`; `.cargo/semver-checks-allowlist.toml` carries matching `migration_row` entries for all four `Y`-marked rows; `CHANGELOG.md` `[0.10.0]` `### Changed`/`### Fixed` bullets name `PaladinResult`, `StreamingResponse`, `ExecuteResponse`, the battalion split correction and the Anthropic billed-input correction; per task context, `cargo clippy -p paladin-llm -p paladin-web -p paladin-ai-core -p paladin-battalion -p paladin-herald --all-targets -- -D warnings` ran clean live; `make test`/clippy/fmt reported green on current HEAD per verification context |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/token_usage.rs` | Three new `Option<u32>` fields, builders, `Add`/`AddAssign`/`Sum`, tests | VERIFIED | Confirmed present, doc-tested, unit-tested (21/21 live) |
| `crates/paladin-core/src/platform/container/execution_result.rs` | `PaladinResult.usage: TokenUsage` | VERIFIED | `pub usage: TokenUsage` at line 61 |
| `crates/paladin-core/src/platform/container/waypoint.rs` | `NodeExecutionRecord.usage: TokenUsage` with `#[serde(default)]` | VERIFIED | line 578 |
| `crates/paladin-core/src/platform/container/trace.rs` | `NodeFinished`/`RunFinished` carry `usage: TokenUsage` | VERIFIED | lines 237, 292 |
| `crates/paladin-battalion/src/engine/hooks.rs` | `TraceDispatcher` `TokenUsage` accumulator + `total_usage()` | VERIFIED | `Mutex<TokenUsage>`, poison recovery, accessor present |
| `crates/paladin-ports/src/output/llm_port.rs` | `StreamingResponse.usage`, `#[non_exhaustive]`, constructors | VERIFIED | Present with doc-tested constructors |
| `crates/paladin-ports/src/output/paladin_port.rs` | `ChunkMetadata.usage`, `#[non_exhaustive]` | VERIFIED | Present |
| `crates/paladin-llm/src/anthropic/adapter.rs` | Cache/reasoning fields, cache-inclusive `prompt_tokens` correction | VERIFIED | `cache_read_input_tokens`/`cache_creation_input_tokens`/`thinking_tokens`, correction formula present |
| `crates/paladin-llm/src/gemini/adapter.rs` | `cached_content_token_count`/`thoughts_token_count`, `Default` intact | VERIFIED | Present |
| `crates/paladin-llm/src/conformance.rs` | Shared parity case, `CASE_COUNT == 9` | VERIFIED | `streaming_usage_equals_non_streaming_usage`, `CASE_COUNT, 9` pin present |
| `crates/paladin-herald/src/json_herald.rs` | Full `usage` object, stable 6-key set | VERIFIED | `"usage": result.usage`, no `skip_serializing_if` |
| `crates/paladin-herald/src/markdown_herald.rs` | "Token Usage" block, per-Paladin table | VERIFIED | Present, tested |
| `crates/paladin-web/src/agent_controller.rs` | `TokenUsageResponse`, `ExecuteResponse.usage`, SSE `done` event usage (CR-01 fix) | VERIFIED | DTO + `From<TokenUsage>` present; `chunk_to_event` reads `chunk.metadata.usage` and includes it on the `"done"` event (post-review fix, commit d214a4c1); tests pass live |
| `crates/paladin-ports/src/input/run_inspector_port.rs` | `CompletedRow.usage: Option<TokenUsage>` | VERIFIED | line 89 |
| `crates/paladin-web/openapi.json` | Regenerated baseline with `TokenUsageResponse` schema | VERIFIED | `usage` property + `TokenUsageResponse` component present |
| `MIGRATION.md` §9.2 | Rows for every touched public type | VERIFIED | All 8 rows present (5 `Y`-marked + 3 N/A completeness) |
| `.cargo/semver-checks-allowlist.toml` | Row-matched `[[entry]]` blocks | VERIFIED | `migration_row` entries match all `Y`-marked §9.2 rows |
| `CHANGELOG.md` | `[0.10.0]` Changed/Fixed bullets | VERIFIED | Present, names carrier change and both corrections |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `TokenUsage` builders | every in-tree struct literal | compiler-driven migration | WIRED | `from_total` fully removed; zero remaining literal sites reported by grep |
| `formation_service.rs`/`phalanx_service.rs` | `PaladinResult.usage` | `result.usage.clone()` into `per_paladin_tokens` | WIRED | Confirmed by grep + passing tests |
| `TraceDispatcher::emit`'s `NodeFinished` arm | `RunFinished.usage` | `TokenUsage::AddAssign`, 5 construction sites in `engine/mod.rs` | WIRED | `grep -c 'usage: trace.total_usage()'` == 5 |
| execution service reasoning loop | `middleware_cx.cumulative_tokens` | `usage += response.usage.clone()` then `.total_tokens` | WIRED | Confirmed at `paladin_execution_service.rs:1507-1508` |
| `CompatEngine::generate_stream` | terminal `StreamingResponse` | hold-and-emit (finish_reason + usage on `[DONE]`) | WIRED | `stream_options`/`include_usage` request field + terminal-chunk construction confirmed; mockito tests pass |
| `PaladinExecutionService` streaming consumer | `ChunkMetadata.usage` on `is_final` chunk | direct read of terminal `StreamingResponse.usage` | WIRED | Confirmed by execution-service tests (streamed-equals-non-streamed parity) |
| `chunk_to_event` (SSE `/execute/stream`) | `ChunkMetadata.usage` | `.and_then(|m| m.usage.clone()).map(TokenUsageResponse::from)` on the `"done"` event | WIRED (post-review fix) | `agent_controller.rs:492-513`; `stream_done_event_carries_usage_when_final_chunk_reports_it` / `..._is_null_when_final_chunk_has_none` pass live |
| `ExecuteResponse::from(PaladinResult)` | `TokenUsageResponse` | `TokenUsageResponse::from(result.usage)` | WIRED | Confirmed, tests pass |
| SSE run events (`events.rs`) | `TraceEvent` serialization | direct serde of `NodeFinished`/`RunFinished` | WIRED | `node_finished_payload_carries_six_key_usage_object` / `run_finished_payload_carries_six_key_usage_object` pass live |
| allowlist `[[entry]]` rows | `MIGRATION.md` §9.2 rows | row-level `migration_row` string match | WIRED | Verified by grep cross-reference; verification context confirms local awk set-equality script passes 13=13 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `TokenUsage` arithmetic/serde unit+doctests | `cargo test -p paladin-ai-core --lib token_usage` | 21/21 passed | PASS |
| LLM conformance suite incl. streaming-parity case | `cargo test -p paladin-llm --lib conformance` | 19/19 passed | PASS |
| Agent controller incl. CR-01 SSE usage fix | `cargo test -p paladin-web --lib agent_controller` | 40/40 passed | PASS |
| Run inspector cache-hit/executed usage rows | `cargo test -p paladin-ai --lib run::inspector` | 12/12 passed | PASS |
| SSE trace-event payload usage assertions | `cargo test -p paladin-ai --lib run::events` | 12/12 passed | PASS |
| Workspace lint on touched crates | `cargo clippy -p paladin-llm -p paladin-web -p paladin-ai-core -p paladin-battalion -p paladin-herald --all-targets -- -D warnings` | clean, 0 warnings | PASS |
| mdBook build | `mdbook build docs/` | exits 0, "No broken links found" | PASS |
| Docs-currency gate (no stray `token_count`/wrong field names) | grep gates from plan 31-05 task 3 | zero matches | PASS |

### Code Review Findings — Verified Fixed (not just claimed)

The phase's own `31-REVIEW.md` found one CRITICAL (CR-01) and one WARNING (WR-01) defect. Both were
fixed in commits `d214a4c1` and `7cd9bc73`, which post-date the plan SUMMARYs. Independently
re-verified against the current tree rather than trusting `31-REVIEW-FIX.md`'s claims:

- **CR-01** (the `/v1/agents/{id}/execute/stream` SSE `"done"` event dropped `ChunkMetadata.usage`):
  confirmed fixed — `chunk_to_event` in `crates/paladin-web/src/agent_controller.rs:492-513` now
  reads `chunk.metadata.as_ref().and_then(|m| m.usage.clone()).map(TokenUsageResponse::from)` and
  includes it on the `"done"` event payload; both new regression tests
  (`stream_done_event_carries_usage_when_final_chunk_reports_it`,
  `stream_done_event_usage_is_null_when_final_chunk_has_none`) pass live.
- **WR-01** (misattached doc comment on DeepSeek's `map_usage`/`annotate_with_usage`): confirmed
  fixed — the two functions now each carry their own correctly-attached, non-contradictory doc
  comment in `crates/paladin-llm/src/deepseek/adapter.rs:288-332`.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| ACCT-01 | 31-01 | `TokenUsage` gains optionals, documented contract, legacy/round-trip JSON | SATISFIED | See Truth #1 |
| ACCT-02 | 31-02, 31-06 | Full `TokenUsage` carrier chain, real per-Paladin splits, no deprecated accessor | SATISFIED | See Truth #2, HTTP-edge artifacts |
| ACCT-03 | 31-03, 31-04 | Streaming usage parity across every adapter or documented exception | SATISFIED | See Truth #3 |
| ACCT-04 | 31-05 | Breakdown observable in JSON and Markdown herald | SATISFIED | See Truth #4 |
| ACCT-05 | 31-07 | `MIGRATION.md` + allowlist + `CHANGELOG.md` + gate green | SATISFIED | See Truth #5 |

No orphaned requirements found — REQUIREMENTS.md maps exactly ACCT-01..05 to Phase 31, and every one
appears in a plan's `requirements:` frontmatter.

### Anti-Patterns Found

None found in the reviewed carrier/herald/adapter/web files that would block the goal. The known,
tracked, non-blocking items (documented in `deferred-items.md`, consistent with verification
context) are:
- `cli_isolation` test fails only under `--all-features` (pre-existing, unrelated to this phase,
  confirmed by git blame in `deferred-items.md`).
- 77 pre-existing `cargo doc` warnings, none originating from files this phase touched.

No debt markers (`TBD`/`FIXME`/`XXX`) found in the files this phase's plans modified.

### Human Verification Required

None. All five ROADMAP success criteria resolve to observable, test-backed truths; no
visual/real-time/external-service behavior remains unverified for this phase's scope.

### Gaps Summary

No gaps. All five must-haves (ACCT-01 through ACCT-05) verified against the codebase — not just
plan SUMMARYs — with live test runs, grep-confirmed artifact/wiring evidence, and independent
re-verification of the two post-summary code-review fixes (CR-01, WR-01), both of which are
confirmed landed and tested in the current tree.

---

_Verified: 2026-09-15_
_Verifier: Claude (gsd-verifier)_
