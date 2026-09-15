---
phase: 31-lossless-token-accounting
reviewed: 2026-09-15T00:00:00Z
depth: standard
files_reviewed: 65
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - crates/paladin-battalion/src/engine/export/overlay.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/formation_service.rs
  - crates/paladin-battalion/src/phalanx_service.rs
  - crates/paladin-core/Cargo.toml
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/token_usage.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-eval/src/scripted_llm.rs
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/mock.rs
  - crates/paladin-llm/src/ollama/adapter.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/input/run_inspector_port.rs
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/dev_ui_controller.rs
  - crates/paladin-web/src/lib.rs
  - crates/paladin-web/tests/openapi_golden_v0_9.rs
  - docs/src/api-reference/migration-guide.md
  - docs/src/api-reference/upgrading.md
  - docs/src/appendix/conclave-pattern.md
  - docs/src/appendix/provider-expansion.md
  - docs/src/contributing/contributing-providers.md
  - docs/src/getting-started/quickstart.md
  - docs/src/operations/observability.md
  - docs/src/user-guides/agent-orchestrator-bridge.md
  - docs/src/user-guides/battalion-patterns.md
  - docs/src/user-guides/herald-output.md
  - docs/src/user-guides/orchestration.md
  - docs/src/user-guides/output-formatting.md
  - docs/src/user-guides/paladin-agents.md
  - src/application/cli/commands/agent.rs
  - src/application/cli/commands/battalion.rs
  - src/application/cli/formatters/mod.rs
  - src/application/cli/formatters/output.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/events.rs
  - src/application/services/run/inspector.rs
  - tests/functional/content_llm_analysis_pipeline_test.rs
  - tests/helpers/mock_llm_adapter.rs
  - tests/integration/herald_integration_test.rs
findings:
  critical: 1
  warning: 1
  info: 1
  total: 3
status: issues_found
---

# Phase 31: Code Review Report

**Reviewed:** 2026-09-15
**Depth:** standard
**Files Reviewed:** 65 (diffed against `418e2bc6dbe5beaa62320d2d8b57f3ae366ad7ab`)
**Status:** issues_found

## Summary

This phase carries the provider's `TokenUsage` prompt/completion split (plus cache/reasoning
sub-counts) losslessly from every LLM adapter through `PaladinResult`, `BattalionResult`,
`NodeExecutionRecord`, `TraceEvent`, the SSE run-stream, the heralds, the CLI, and the HTTP edge.
The implementation is unusually disciplined: the `TokenUsage` saturating `Add`/`AddAssign`/`Sum`
arithmetic and `Option`-merge rule are correct and well-tested; the D-14 hold-and-emit
terminal-chunk contract is applied consistently and correctly across `CompatEngine`, `openai`,
`deepseek`, `anthropic` (event-accumulation variant) and `gemini` (cumulative-frame variant); the
D-02 `total_tokens == prompt_tokens + completion_tokens` invariant holds at every mapping site,
including Anthropic's cache-inclusive `prompt_tokens` correction and Gemini's
`candidates + thoughts` completion; `TokenUsage::from_total` is gone with zero remaining callers
anywhere in the tree; the `Mutex<TokenUsage>` accumulator in `engine/hooks.rs` correctly recovers
from poisoning via `PoisonError::into_inner` rather than panicking; and the OpenAPI golden-diff
exception in `openapi_golden_v0_9.rs` is narrowly scoped and self-verifying
(`execute_response_exception_is_narrowly_scoped`).

Two real defects were found despite this. First, and most significant: the phase's own D-18
decision ("The SSE run-stream and paladin-web streaming responses forward it") is only half
honored — the `/v1/agents/{id}/execute/stream` SSE endpoint in `paladin-web` was never touched by
this phase and its terminal `"done"` event still discards `PaladinStreamChunk.metadata` (and
therefore `usage`) entirely, so a real streaming HTTP client of the single-agent endpoint never
receives the token usage the rest of the pipeline now computes and carries losslessly all the way
to that chunk. Second, a doc-comment insertion bug in `deepseek/adapter.rs` misattaches an entire
pre-existing rustdoc block (including a claim that is now factually false) from
`annotate_with_usage` onto the newly-inserted `map_usage`, leaving `annotate_with_usage`
undocumented and `map_usage` documented with contradictory, unrelated content.

## Critical Issues

### CR-01: Streamed token usage never reaches the `/v1/agents/{id}/execute/stream` SSE client

**File:** `crates/paladin-web/src/agent_controller.rs:489-493`

**Issue:** D-18 (`31-CONTEXT.md`) explicitly requires: *"The execution service's streaming surface
carries the usage to its consumer: `ChunkMetadata`... populated only on the `is_final` chunk from
the provider's terminal chunk... **The SSE run-stream and paladin-web streaming responses forward
it.**"* The "SSE run-stream" half of that sentence is satisfied (`src/application/services/run/events.rs`
now serializes `usage` on `node_finished`/`run_finished` payloads). The "paladin-web streaming
responses" half is not: `chunk_to_event`, the function that renders a `PaladinStreamChunk` into an
SSE `Event` for the `/v1/agents/{id}/execute/stream` endpoint, was not touched by this phase at
all (confirmed via `git log -- crates/paladin-web/src/agent_controller.rs` against the phase's
commit range — only `31-06`/`31-02` touched this file, and neither touched this function). Its
terminal branch is:

```rust
Ok(chunk) if chunk.is_final => Event::default()
    .event("done")
    .data(json!({ "done": true }).to_string()),
```

`chunk.metadata` (which, per `PaladinExecutionService`'s streaming consumer at
`src/application/services/paladin/paladin_execution_service.rs:~3231-3260`, now correctly carries
`Some(ChunkMetadata::new().with_usage(usage))` on exactly the final chunk) is read nowhere in this
function and is dropped on the floor. Every other consumer of the same data — the non-streaming
`ExecuteResponse.usage` (D-24), the run-inspector's `CompletedRow.usage`, the dev-UI page, both
heralds, the CLI — was updated in this phase; this one SSE surface was missed. A real client
polling `/v1/agents/{id}/execute/stream` (as opposed to the test-only in-process consumers this
phase added tests for) has no way to learn how many tokens a streamed call actually used, which
silently defeats the phase's own stated goal ("lossless token accounting... out to... web
streaming responses") for this specific, real, externally-facing endpoint.

**Fix:**
```rust
fn chunk_to_event(item: Result<PaladinStreamChunk, PaladinError>) -> Event {
    match item {
        Ok(chunk) if chunk.is_final => {
            let usage = chunk
                .metadata
                .as_ref()
                .and_then(|m| m.usage.as_ref())
                .map(TokenUsageResponse::from_ref); // or serialize TokenUsage directly
            Event::default()
                .event("done")
                .data(json!({ "done": true, "usage": usage }).to_string())
        }
        Ok(chunk) => Event::default()
            .event("chunk")
            .data(json!({ "text": chunk.text }).to_string()),
        Err(error) => Event::default().event("error").data(
            ApiError::bad_gateway(error.to_string())
                .to_body()
                .to_string(),
        ),
    }
}
```
Add a test asserting the `"done"` SSE event's JSON payload carries a `usage` object (mirroring
`dev_ui_page_embeds_executed_row_with_full_usage_object` and
`execute_response_serializes_usage_object_with_six_keys` in the same file), and update
`docs/src/user-guides/agent-orchestrator-bridge.md`'s SSE section if it documents the `"done"`
event shape.

## Warnings

### WR-01: Doc comment misattachment leaves `annotate_with_usage` undocumented and `map_usage` documented with stale, self-contradicting content

**File:** `crates/paladin-llm/src/deepseek/adapter.rs:289-330`

**Issue:** The pre-existing rustdoc block above `annotate_with_usage` (lines 289-312) ends with:

```rust
/// **Known, deliberate limitation.** The reasoning/content token SPLIT stays
/// unobservable: [`DeepSeekUsage`] does not deserialize
/// `completion_tokens_details.reasoning_tokens`. Recording `prompt_tokens`
/// here is a strictly smaller, separately-scoped change — splitting
/// reasoning from content is a distinct upstream change left for its own
/// task, not silently implied as solved by this one.
```

The new `map_usage` function's doc comment (lines 313-317) was inserted directly after this, with
no blank line separating the two blocks:

```rust
/// task, not silently implied as solved by this one.
/// Map a wire-reported [`DeepSeekUsage`] into [`TokenUsage`], applying the
/// D-20 cache/reasoning sub-count builders only when the payload actually
/// carried the figure (D-03: an absent figure is `None`, never a fabricated
/// `Some(0)`). Shared by both the non-streaming usage-construction site and
/// the streaming terminal-chunk usage, so the two paths cannot drift.
fn map_usage(usage: DeepSeekUsage) -> TokenUsage {
```

Rust attaches a doc comment to the item immediately following it with no blank line, so the ENTIRE
combined nine-paragraph block (describing `annotate_with_usage`'s `EmptyCompletion`-annotation
purpose) is now the doc comment for `map_usage`, and `annotate_with_usage` (line 332) has no doc
comment at all. Beyond the misattachment, the inherited claim is now actively false where it
lands: "the reasoning/content token SPLIT stays unobservable" is attached to the very function
(`map_usage`) that this same diff introduces specifically to map
`completion_tokens_details.reasoning_tokens` into `TokenUsage::reasoning_tokens` (D-20) — i.e. the
doc comment on `map_usage` now contradicts `map_usage`'s own body three lines below it. This is
private (`fn`, not `pub fn`) so it does not fail `cargo doc`'s public-API lint, but it is
genuinely misleading to a future reader and violates the project's "clear and concise comments"
convention (`rust.instructions.md`).

**Fix:** Insert a blank line between the two doc blocks so each attaches to its own function, and
remove/update the now-false "reasoning/content token SPLIT stays unobservable" sentence in
`annotate_with_usage`'s restored doc comment (or note explicitly that this limitation applied only
until `map_usage`/D-20 landed):
```rust
/// task, not silently implied as solved by this one.

/// Map a wire-reported [`DeepSeekUsage`] into [`TokenUsage`], applying the
/// D-20 cache/reasoning sub-count builders only when the payload actually
/// carried the figure (D-03: an absent figure is `None`, never a fabricated
/// `Some(0)`). Shared by both the non-streaming usage-construction site and
/// the streaming terminal-chunk usage, so the two paths cannot drift.
fn map_usage(usage: DeepSeekUsage) -> TokenUsage {
```

## Info

### IN-01: `ExecutionOverlay`'s exported `Visit.tokens` collapses the full split back to a bare total

**File:** `crates/paladin-battalion/src/engine/export/overlay.rs:154, ~220`

**Issue:** `Visit.tokens: u64` is populated as `u64::from(record.usage.total_tokens)` /
`u64::from(usage.total_tokens)` at both sites that build a `Visit` from a `NodeExecutionRecord`/
`TraceEvent::NodeFinished`. This is consistent with the D-08 bare-total-coexistence rule (a bare
total may sit beside a full carrier elsewhere) and is not a violation of any locked decision —
`overlay.rs` is not named in `31-CONTEXT.md`'s carrier list — so this is not a defect. It is worth
flagging for awareness: any future Milestone 14 (Treasurer) consumer that wants a per-visit
cache/reasoning split from an exported overlay will not find it here and will need to go back to
`NodeExecutionRecord.usage` directly.

**Fix:** No action required for this phase; note for Milestone 14 planning if a per-visit split is
ever needed in exported overlay data.

---

_Reviewed: 2026-09-15_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
