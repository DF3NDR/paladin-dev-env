---
phase: 31-lossless-token-accounting
fixed_at: 2026-09-15T09:02:41Z
review_path: .planning/phases/31-lossless-token-accounting/31-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 31: Code Review Fix Report

**Fixed at:** 2026-09-15T09:02:41Z
**Source review:** .planning/phases/31-lossless-token-accounting/31-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 2 (critical + warning; IN-01 is Info-level and out of scope for this fix pass)
- Fixed: 2
- Skipped: 0

## Fixed Issues

### CR-01: Streamed token usage never reaches the `/v1/agents/{id}/execute/stream` SSE client

**Files modified:** `crates/paladin-web/src/agent_controller.rs`
**Commit:** d214a4c1
**Applied fix:** `chunk_to_event`'s terminal branch now reads `chunk.metadata.usage`, maps it
through the existing `TokenUsageResponse::from` (D-24's six-key DTO shape), and includes it as a
`usage` key on the `"done"` SSE event's JSON payload — `null` when the final chunk's metadata
carries no usage, never a fabricated value (D-17). Added two handler-level tests
(`stream_done_event_carries_usage_when_final_chunk_reports_it`,
`stream_done_event_usage_is_null_when_final_chunk_has_none`) that exercise the real streaming
path through a mock `StreamingExecutorPort` whose final chunk carries `ChunkMetadata`, reading
the actual SSE response body rather than asserting on the handler's internal types. Verified:
`cargo test -p paladin-web` (226 lib tests + 5 auth tests + 7 openapi-golden tests, all passing,
including the golden-diff baseline which needed no regeneration since the SSE endpoint's OpenAPI
schema was already an untyped `text/event-stream`), `cargo fmt --all -- --check`, and
`cargo clippy -p paladin-web --all-targets -- -D warnings` all clean.

### WR-01: Doc comment misattachment leaves `annotate_with_usage` undocumented and `map_usage` documented with stale, self-contradicting content

**Files modified:** `crates/paladin-llm/src/deepseek/adapter.rs`
**Commit:** 7cd9bc73
**Applied fix:** Reordered the two functions so each keeps its own correctly-attached doc
comment: `map_usage`'s doc (D-20 cache/reasoning mapping) now sits directly above `map_usage`,
and `annotate_with_usage`'s doc (the `EmptyCompletion`-annotation purpose) sits directly above
`annotate_with_usage`, with no blank line breaking either attachment. Removed the now-false
"the reasoning/content token split stays unobservable" claim from `annotate_with_usage`'s doc
(previously inherited by the misattached block) since `map_usage` now maps
`completion_tokens_details.reasoning_tokens` into `TokenUsage::reasoning_tokens`, replacing it
with a cross-reference to `map_usage`. Verified: `cargo doc -p paladin-llm --no-deps` produces
the same 4 pre-existing warnings (all in unrelated files: `redaction.rs`, `commissary.rs`, and
two unrelated unclosed-HTML-tag warnings) with no new warnings from this file;
`cargo test -p paladin-llm --lib --features deepseek deepseek::adapter::tests` (49 tests,
including `map_usage_leaves_optionals_none_when_the_payload_omits_them` and
`map_usage_maps_cache_hit_and_reasoning_when_the_payload_carries_them`) all passing;
`cargo fmt --all -- --check` and `cargo clippy -p paladin-llm --lib --features deepseek -- -D
warnings` both clean. This is a documentation-only change with no behavioral difference, so no
new test was required by the project's TDD rule beyond confirming the existing `map_usage`
coverage still passes.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-09-15T09:02:41Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
