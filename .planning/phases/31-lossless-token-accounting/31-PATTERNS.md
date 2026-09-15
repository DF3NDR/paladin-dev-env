# Phase 31: Lossless Token Accounting - Pattern Map

**Mapped:** 2026-09-14
**Files analyzed:** ~35 modified files (no wholly new files — this phase is a carrier-shape
migration; every "analog" is either the file's own current shape or the closest in-tree
precedent for the KIND of change)
**Analogs found:** 35 / 35 (every touched file has at least a same-file precedent; several also
have a cross-file precedent for the specific mechanic)

## File Classification

| File | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|-----------------|----------------|
| `crates/paladin-core/src/platform/container/token_usage.rs` | model (value type) | transform | own current shape (below) | exact — self |
| `crates/paladin-core/src/platform/container/execution_result.rs` (`PaladinResult`) | model | CRUD-ish (constructed/read) | `served_by` additive-field precedent (same file) | exact — self, documented precedent |
| `crates/paladin-core/src/platform/container/battalion/mod.rs` (`BattalionResult.per_paladin_tokens`) | model | transform (aggregation) | own current shape; no signature change | exact — self |
| `crates/paladin-core/src/platform/container/waypoint.rs` (`NodeExecutionRecord`) | model (persisted) | event-driven/CRUD (persisted record) | `attempts: Vec<AttemptRecord>` / `cache_hit: bool` additive-`#[serde(default)]` precedent (same file, ~D-16) | exact — same-file precedent |
| `crates/paladin-core/src/platform/container/trace.rs` (`TraceEvent::NodeFinished`/`RunFinished`) | model (event/DTO) | event-driven | own current variant shape; `#[non_exhaustive]` wildcard-arm convention (module test ~383-435) | exact — self |
| `crates/paladin-core/src/platform/container/herald.rs` (`ExecutionMetadata.token_usage`) | model | transform | unchanged — already full `TokenUsage`; no analog needed | N/A (no change) |
| `crates/paladin-battalion/src/formation_service.rs`, `phalanx_service.rs` | service (orchestration) | CRUD/aggregation | own current `from_total(..)` call sites | exact — self |
| `crates/paladin-battalion/src/engine/superstep.rs` | service (dispatch/engine) | event-driven (tuple threading) | own current `(paladin_id, token_count: u64, ..)` tuple sites | exact — self |
| `crates/paladin-battalion/src/engine/hooks.rs` (`TraceDispatcher`) | service (sync counters) | event-driven | own current `AtomicU64 token_total` accumulator | exact — self |
| `crates/paladin-battalion/src/engine/mod.rs` (5× `RunFinished` sites) | service | event-driven | own current construction sites | exact — self |
| `crates/paladin-battalion/src/engine/test_support.rs` (`RecordingPaladinPort`) | test helper | request-response (test double) | own `set_output_with_tokens` | exact — self |
| `crates/paladin-ports/src/output/llm_port.rs` (`StreamingResponse`) | model/port DTO | streaming | `LlmRequest` `#[non_exhaustive]` + builder migration precedent (MIGRATION.md §9.2 row) | strong cross-file precedent |
| `crates/paladin-ports/src/output/paladin_port.rs` (`ChunkMetadata`) | model/port DTO | streaming | same `LlmRequest` precedent (no `Default` today → option (a)) | strong cross-file precedent |
| `crates/paladin-ports/src/input/run_inspector_port.rs` (`CompletedRow`) | model/port DTO | request-response | `ResumeAccepted`/new-in-0.10 "N/A" precedent (MIGRATION.md §9.2) | strong cross-file precedent |
| `crates/paladin-llm/src/compat/engine.rs`, `compat/types.rs` | adapter (streaming parser) | streaming | own current `generate_stream` loop (no usage frame today) | exact — self |
| `crates/paladin-llm/src/openai/adapter.rs`, `deepseek/adapter.rs`, `anthropic/adapter.rs`, `gemini/adapter.rs` | adapter | streaming + request-response | own current `*Usage` structs + `generate`/`generate_stream` | exact — self, per-provider |
| `crates/paladin-llm/src/mock.rs` | adapter (test double) | streaming | own current `generate_stream` (terminal chunk, no usage attached today) | exact — self |
| `crates/paladin-llm/src/fallback.rs` | adapter (pass-through) | streaming | own current pass-through (no change needed) | N/A |
| `crates/paladin-llm/src/conformance.rs` (`llm_conformance_suite!`) | test harness | streaming/request-response | existing 8 cases + `ConformanceFixture` trait | exact — self, extend |
| `crates/paladin-herald/src/json_herald.rs`, `markdown_herald.rs`, `table_herald.rs` | presentation (herald) | transform | own current `token_count`/`total_tokens` emission | exact — self |
| `src/application/services/paladin/paladin_execution_service.rs` | service | event-driven (reasoning loop) + streaming | own current accumulator + 6 `PaladinResult` sites | exact — self |
| `crates/paladin-eval/src/assertion.rs` | service (eval) | request-response | own current `total_tokens_max` read of `RunFinished` | exact — self |
| `crates/paladin-web/src/agent_controller.rs` (`ExecuteResponse`) | controller (HTTP DTO) | request-response | `ResumeAcceptedResponse` new-`utoipa::ToSchema` DTO precedent | strong cross-file precedent |
| `MIGRATION.md` §9.2 | config/docs | N/A | `PaladinResult`/`LlmRequest`/`ResumeAccepted` row templates (already in file) | exact — in-file template |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/token_usage.rs`

**Analog:** its own current shape (full file, 76 lines, read this session).

**Current struct + constructors** (lines 12-40):
```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self { prompt_tokens, completion_tokens, total_tokens: prompt_tokens + completion_tokens }
    }

    pub fn from_total(total_tokens: u32) -> Self {   // DELETE per D-05
        Self { prompt_tokens: 0, completion_tokens: 0, total_tokens }
    }
}
```

**Pattern to copy — additive optional fields + saturating arithmetic (from RESEARCH.md's D-06
Code Example, no prior art in-repo to copy verbatim, this is the canonical shape to write):**
```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub cache_read_tokens: Option<u32>,
    #[serde(default)]
    pub cache_write_tokens: Option<u32>,
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
}

impl TokenUsage {
    fn merge_optional(a: Option<u32>, b: Option<u32>) -> Option<u32> {
        match (a, b) {
            (None, None) => None,
            (None, Some(x)) | (Some(x), None) => Some(x),
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
        }
    }
    pub fn with_cache_read(mut self, n: u32) -> Self { self.cache_read_tokens = Some(n); self }
    pub fn with_cache_write(mut self, n: u32) -> Self { self.cache_write_tokens = Some(n); self }
    pub fn with_reasoning(mut self, n: u32) -> Self { self.reasoning_tokens = Some(n); self }
}

impl std::ops::Add for TokenUsage {
    type Output = TokenUsage;
    fn add(self, rhs: Self) -> Self::Output {
        let prompt_tokens = self.prompt_tokens.saturating_add(rhs.prompt_tokens);
        let completion_tokens = self.completion_tokens.saturating_add(rhs.completion_tokens);
        TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens), // D-02 invariant
            cache_read_tokens: Self::merge_optional(self.cache_read_tokens, rhs.cache_read_tokens),
            cache_write_tokens: Self::merge_optional(self.cache_write_tokens, rhs.cache_write_tokens),
            reasoning_tokens: Self::merge_optional(self.reasoning_tokens, rhs.reasoning_tokens),
        }
    }
}
impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, rhs: Self) { *self = self.clone() + rhs; }
}
impl std::iter::Sum for TokenUsage {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self { iter.fold(TokenUsage::default(), |a, x| a + x) }
}
```

**Test pattern to extend** (existing 4 tests, lines 42-75): add
`legacy_json_without_optionals_round_trips` and `new_json_with_optionals_round_trips`, mirroring
`from_total_leaves_prompt_and_completion_at_zero`'s literal-assertion style. Delete
`from_total_leaves_prompt_and_completion_at_zero` with the method (D-05).

---

### `crates/paladin-core/src/platform/container/execution_result.rs` (`PaladinResult`)

**Analog:** the file's own `served_by` row and its own already-migrated tests — this is the
**exact template** for a constructible-struct additive/replacing field change on this same type.

**Constructible-struct rationale already on file** (lines 36-40, keep verbatim reasoning style):
```rust
/// The struct is deliberately **constructible** and **not** `#[non_exhaustive]`
/// (Doc 04 D-26, X-10.3 option (b)): functional-update syntax
/// (`..Default::default()`) is disallowed cross-crate on a `#[non_exhaustive]`
/// struct, so marking it would break every downstream construction site...
```

**Field replacement** (`token_count: u32` at line ~53 → `usage: TokenUsage`):
```rust
pub struct PaladinResult {
    pub output: String,
    pub usage: TokenUsage,          // was: pub token_count: u32,
    pub execution_time_ms: u64,
    pub loop_count: u32,
    // ...
}
```

**Constructor pattern to copy** (`new`, lines ~213-230): keep the same positional-arg shape, just
retype the second parameter:
```rust
pub fn new(
    output: String,
    usage: TokenUsage,             // was: token_count: u32,
    execution_time_ms: u64,
    loop_count: u32,
    stop_reason: StopReason,
) -> Self {
    Self { output, usage, execution_time_ms, loop_count, stop_reason, plan: None, handoff_history: Vec::new(), served_by: None }
}
```

**Legacy-JSON test pattern to rewrite** (copy this exact shape, lines ~253-280, for the new
zero-usage-on-legacy-row contract per D-25):
```rust
#[test]
fn served_by_is_absent_from_legacy_json() {
    let result = PaladinResult { output: "answer".to_string(), token_count: 3, /* ... */ };
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("served_by"), "{json}");
    assert_eq!(json, r#"{"output":"answer","token_count":3,...}"#);
}

#[test]
fn legacy_json_deserialises_with_served_by_none() {
    let legacy = r#"{"output":"answer","token_count":3,...}"#;
    let result: PaladinResult = serde_json::from_str(legacy).unwrap();
    assert_eq!(result.output, "answer");
    assert!(result.served_by.is_none());
}
```
D-25's version of the second test should assert `result.usage == TokenUsage::default()` when
deserializing a `{"token_count": N, ...}` legacy blob (the key is simply dropped/ignored by serde
since there's no `deny_unknown_fields`).

---

### `crates/paladin-core/src/platform/container/waypoint.rs` (`NodeExecutionRecord`)

**Analog:** the file's own `attempts: Vec<AttemptRecord>` / `cache_hit: bool` additive-field
precedent, immediately above/around the struct (verified, lines ~562-592).

**Additive-field doc pattern to copy** (this is the exact rustdoc style for a
`#[serde(default)]` field with a "why this doesn't break old rows" justification):
```rust
/// Every FAILED attempt of this node this superstep, ordered by attempt
/// number ascending (D-16, FT-FR-03). ... Additive, `#[serde(default)]`,
/// following `visit_counts`/`frontier`/`fork_of`'s precedent, so a
/// `Waypoint` written before Phase 25 deserialises with an empty
/// history and `BATTLEFIELD_SCHEMA_VERSION` is unchanged.
#[serde(default)]
pub attempts: Vec<AttemptRecord>,
```

**Field to replace**: `pub token_count: u64,` → `pub usage: TokenUsage,` (this is a **replacement**,
not additive like `attempts`/`cache_hit` — no `#[serde(default)]` needed on the new field itself
since it's mandatory going forward, but the D-25 zero-on-legacy-row contract still applies because
old JSON blobs lack `usage` entirely: use `#[serde(default)]` on `usage` too so a pre-Phase-31 blob
deserializes with `usage: TokenUsage::default()` rather than failing to parse).

**No SQL migration needed** — verified precedent: `attempts`/`cache_hit`/`fork_of` all landed with
no `ALTER TABLE`, confirming `NodeExecutionRecord` is stored as an opaque JSON blob column, not
typed SQL columns (RESEARCH.md Assumption A4 — verify at the start of Wave 2 per its own
recommendation: `grep -n "token_count" crates/paladin-storage/src -r`).

---

### `crates/paladin-core/src/platform/container/trace.rs` (`NodeFinished`/`RunFinished`)

**Analog:** the enum's own current variant shapes (verified, lines ~219-238 `NodeFinished`,
~282-297 `RunFinished`) plus the module's `#[non_exhaustive]` wildcard-arm test convention.

**Current `NodeFinished` shape** (to replace `token_count: u64` with `usage: TokenUsage`):
```rust
NodeFinished {
    superstep: u64,
    node_id: NodeId,
    attempt: u32,
    outcome: NodeOutcomeKind,
    duration_ms: u64,
    token_count: u64,     // -> usage: TokenUsage
    cache_hit: bool,
},
```

**Current `RunFinished` shape** (to replace `total_tokens: u64` with `usage: TokenUsage`):
```rust
RunFinished {
    status: RunFinishStatus,
    total_supersteps: u64,
    total_tokens: u64,    // -> usage: TokenUsage
    duration_ms: u64,
    trace_dropped_total: u64,
},
```

**Enforcement pattern already in the codebase (must be preserved):** every `match` over
`TraceEvent` in the workspace carries a wildcard arm (enforced by the module's own test at
`trace.rs:~437`) — the field-rename inside a variant is a compile-visible break only at the sites
that destructure `token_count`/`total_tokens` (listed in RESEARCH.md's canonical_refs: `hooks.rs`,
`assertion.rs`, `inspector.rs`, `events.rs`, `engine/mod.rs` tests, `overlay.rs`).

---

### `crates/paladin-battalion/src/formation_service.rs` / `phalanx_service.rs`

**Analog:** own current `from_total(..)` call sites (~231, ~279).

**Current (buggy — zeroes the split) pattern:**
```rust
per_paladin_tokens.insert(name.clone(), TokenUsage::from_total(result.token_count as u64_or_u32));
total_tokens += result.token_count as u64;
```

**Target pattern (D-10):**
```rust
per_paladin_tokens.insert(name.clone(), result.usage.clone());
total_tokens += u64::from(result.usage.total_tokens);
```

---

### `crates/paladin-battalion/src/engine/superstep.rs` (dispatch closure tuple)

**Analog:** own current structured-executor arm (verified, lines 1094-1099).

**Current shape:**
```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:1094-1099
Ok(structured) => {
    let token_count = u64::from(structured.raw.token_count);
    let mut delta = StateDelta::new();
    delta.set_raw(output_field, structured.value);
    (paladin_id, token_count, Ok(delta.into()))
}
```

**Target shape (mechanical rename, D-11):**
```rust
Ok(structured) => {
    let usage = structured.raw.usage.clone();
    let mut delta = StateDelta::new();
    delta.set_raw(output_field, structured.value);
    (paladin_id, usage, Ok(delta.into()))
}
```
Same treatment applies to the plain-`PaladinPort` arm (~1140) and every `NodeExecutionRecord{..}`/
`NodeFinished{..}` literal downstream (~2991, ~3085, ~3186, ~3257, ~3277). A cache hit uses
`TokenUsage::default()` where it previously used `0`.

---

### `crates/paladin-battalion/src/engine/hooks.rs` (`TraceDispatcher`)

**Analog:** own current `AtomicU64 token_total` accumulator and its `emit()` accumulation logic
(full file read; ~84-95 field, ~298-308 `emit()`'s `NodeFinished` arm, ~371 accessor).

**Current pattern (synchronous-inside-`emit` guarantee to preserve):**
```rust
token_total: AtomicU64,
// inside emit()'s NodeFinished match arm:
self.token_total.fetch_add(token_count, Ordering::Relaxed);
// accessor:
pub fn token_total(&self) -> u64 { self.token_total.load(Ordering::Relaxed) }
```

**Target pattern (D-11 — `Mutex<TokenUsage>` variant, planner's discretion vs. five atomics):**
```rust
usage_total: std::sync::Mutex<TokenUsage>,
// inside emit()'s NodeFinished match arm:
*self.usage_total.lock().expect("usage_total mutex poisoned") += usage.clone();
// accessor:
pub fn total_usage(&self) -> TokenUsage {
    self.usage_total.lock().expect("usage_total mutex poisoned").clone()
}
```
Preserve the existing rustdoc guarantee ("`RunFinished` is exact the instant `emit` returns") —
this is the load-bearing invariant this pattern must not regress.

---

### `crates/paladin-ports/src/output/llm_port.rs` (`StreamingResponse`)

**Analog:** the `LlmRequest` `#[non_exhaustive]` + builder-migration row, already landed and
recorded in `MIGRATION.md` §9.2 (line 178) — this is the closest cross-file precedent for
"port-trait DTO gains a field, gets marked `#[non_exhaustive]`, gains constructors that replace
every in-tree literal in the same commit."

**MIGRATION.md §9.2 template row to copy verbatim in structure:**
```
| `paladin-ports` | `LlmRequest` (the type passed to `LlmPort::generate`) | new additive field
`response_format: Option<ResponseFormat>` with `#[serde(default)]`; struct marked
`#[non_exhaustive]`; new constructor `LlmRequest::new(model, prompt)` plus chainable
`with_attachments`/`with_stream`/`with_metadata`/`with_response_format`, all doc-tested |
`#[non_exhaustive]`, suppressed via `[package.metadata.cargo-semver-checks.lints]
struct_marked_non_exhaustive = "allow"` in `crates/paladin-ports/Cargo.toml`, mirrored in
`.cargo/semver-checks-allowlist.toml`; `LlmRequest::new` + `with_*` builders replace every in-tree
full struct literal in the same commit | Y ...
```

**Target `StreamingResponse` shape (D-13):**
```rust
#[non_exhaustive]
pub struct StreamingResponse {
    pub id: Uuid,
    pub delta: String,
    pub finish_reason: Option<FinishReason>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
}
impl StreamingResponse {
    pub fn delta(text: impl Into<String>) -> Self { /* .. */ }
    pub fn terminal(finish_reason: FinishReason) -> Self { /* .. */ }
    pub fn with_usage(mut self, usage: TokenUsage) -> Self { self.usage = Some(usage); self }
}
```
Suppression: mirror the existing `struct_marked_non_exhaustive = "allow"` already in
`crates/paladin-ports/Cargo.toml`; add allowlist entry `paladin-ports | StreamingResponse`.

---

### `crates/paladin-ports/src/output/paladin_port.rs` (`ChunkMetadata`)

**Analog:** same `LlmRequest` precedent as `StreamingResponse` — confirmed `ChunkMetadata` has
**no** `Default` derive today (RESEARCH.md verified this directly), so X-10.3 option (a)
(`#[non_exhaustive]`) applies identically, not option (b).

```rust
#[non_exhaustive]
pub struct ChunkMetadata {
    // existing fields incl. per-chunk `tokens: Option<u32>` hint (untouched)
    #[serde(default)]
    pub usage: Option<TokenUsage>,   // populated only on the is_final chunk
}
```

---

### `crates/paladin-ports/src/input/run_inspector_port.rs` (`CompletedRow`)

**Analog:** the `ResumeAccepted`/new-in-0.10 "N/A, no allowlist entry" convention (MIGRATION.md
§9.2 line 189) — `CompletedRow` is itself new-in-0.10 (Phase 28 OBS-03), so this row's
`token_count: Option<u64>` → `usage: Option<TokenUsageResponse-or-TokenUsage>` replacement gets
the same "listed for completeness — N/A, not in the 0.9.0 baseline" treatment, no allowlist entry.

**MIGRATION.md template to copy:**
```
| `paladin-ports` | `ParleyPort` (new in 0.10; listed for completeness) | ... | N/A — new trait
introduced in v0.10, not a pre-existing public API. | N/A (new type; X-10 governs only
pre-existing types) | ...
```

---

### `crates/paladin-llm/src/compat/engine.rs` + `compat/types.rs` (streaming terminal-chunk contract)

**Analog:** own current `generate_stream` loop (verified, lines 893-943) — the hold-and-emit
pattern is new logic layered onto the existing `[DONE]`-terminal-`Stop`-chunk behavior already
present.

**Current shape (emits `finish_reason` immediately, never captures usage):**
```rust
// Source: crates/paladin-llm/src/compat/engine.rs:893-943 (verified)
if let Some(choice) = response.choices.first() {
    if let Some(reason) = &choice.finish_reason {
        // finish_reason emitted on THIS frame today
    }
}
```

**Target hold-and-emit pattern (D-14, sketch from RESEARCH.md Code Examples):**
```rust
let mut held_finish_reason: Option<FinishReason> = None;
let mut held_usage: Option<TokenUsage> = None;
// per-line parse loop:
if json_str.trim() == "[DONE]" {
    items.push(Ok(StreamingResponse {
        id: Uuid::new_v4(),
        delta: String::new(),
        finish_reason: held_finish_reason.take().or(Some(FinishReason::Stop)),
        usage: held_usage.take(),
    }));
    continue;
}
match serde_json::from_str::<CompatStreamResponse>(json_str) {
    Ok(response) => {
        if let Some(usage) = response.usage { held_usage = Some(map_compat_usage(usage)); }
        if let Some(choice) = response.choices.first() {
            if let Some(reason) = &choice.finish_reason {
                held_finish_reason = Some(Self::map_finish_reason(Some(reason.clone())));
                continue; // do NOT emit finish_reason on this frame
            }
            items.push(Ok(StreamingResponse { /* delta chunk, no usage, no finish_reason */ }));
        }
    }
    Err(e) => { /* unchanged, already returns Result, no unwrap/expect */ }
}
```
Send `stream_options: {"include_usage": true}` on the request (`CompatRequest` gains
`stream_options: Option<CompatStreamOptions>`); `CompatUsage` gains optional
`prompt_tokens_details.cached_tokens` / `completion_tokens_details.reasoning_tokens`.

Same hold-and-emit pattern applies verbatim to `openai/adapter.rs`'s own `make_streaming_request`
and `deepseek/adapter.rs`'s `generate_stream` (`DeepSeekUsage` additionally gains
`prompt_cache_hit_tokens`).

---

### `crates/paladin-llm/src/anthropic/adapter.rs`

**Analog:** own current `ClaudeUsage`/`ClaudeStreamEvent` + the three captured fixture constants
already in the test module (verified: `THINKING_TEXT_OPUS_5_JSON` has `input_tokens: 85`,
`output_tokens: 3000`, `output_tokens_details.thinking_tokens: 2561`).

**Mapping to copy (D-20):**
```rust
// prompt_tokens correction — Anthropic's input_tokens EXCLUDES cached tokens:
prompt_tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens;
// cache_read_input_tokens  -> cache_read_tokens
// cache_creation_input_tokens -> cache_write_tokens
// output_tokens_details.thinking_tokens -> reasoning_tokens
```
**Test fixture to use as the D-20 mapping test's primary case:** `THINKING_TEXT_OPUS_5_JSON` →
expect `reasoning_tokens == Some(2561)`, `completion_tokens == 3000`. **Add a new fixture** with
non-zero `cache_read_input_tokens`/`cache_creation_input_tokens` — none of the three existing
fixtures exercise the cache-inclusive `prompt_tokens` correction (Pitfall 3/5 in RESEARCH.md).

Streaming: accumulate `message_start.message.usage` (input_tokens + cache fields) and
`message_delta.usage` (cumulative output_tokens + thinking_tokens), attach the final `TokenUsage`
on the `message_stop` terminal event — this IS the D-14 terminal chunk for this adapter (no
`[DONE]` sentinel exists here, unlike the OpenAI family).

---

### `crates/paladin-llm/src/gemini/adapter.rs`

**Analog:** own current `GeminiUsageMetadata` (verified `#[derive(Debug, Default, Deserialize)]`
— **keep the `Default` derive**, per Pitfall 4).

```rust
#[derive(Debug, Default, Deserialize)]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
    #[serde(default)]
    cached_content_token_count: Option<u32>,   // -> cache_read_tokens (prefer Option, not bare u32, per Pitfall 4/D-03)
    #[serde(default)]
    thoughts_token_count: Option<u32>,          // -> reasoning_tokens
}
```
Every SSE frame's `usageMetadata` is cumulative; take the **last** frame's value (the one
carrying `finishReason`) and attach it to that frame's `StreamingResponse` (this is the D-14
terminal chunk here). `completion_tokens = candidatesTokenCount + thoughtsTokenCount` (D-02).

---

### `crates/paladin-llm/src/mock.rs`

**Analog:** own current `generate_stream` impls — confirmed neither attaches usage to the
terminal chunk today (verified this session).

Target: the scripted stream's terminal chunk gets `usage: Some(configured_token_usage)`
(`MockLlmAdapter::with_token_usage_struct` already configures the full struct for the
non-streaming path — reuse that same stored value on the streaming terminal chunk).

---

### `crates/paladin-llm/src/conformance.rs` (parity test)

**Analog:** the existing `llm_conformance_suite!` macro + `ConformanceFixture` trait, already
instantiated by `ollama`/`gemini`/`openai_compatible` (verified — only these three today).

**Pattern to copy:** add a new case `streaming_usage_equals_non_streaming_usage` inside the macro
body, opening `success_body()` through `generate()` and `stream_body()` through
`generate_stream()` against the same mockito server, asserting the terminal chunk's `usage`
equals `LlmResponse.usage` field-for-field. Bump `CASE_COUNT` from 8 to 9 (the assertion at
`~649`). Instantiate the suite for `openai`, `anthropic`, `deepseek`, `grok`, `kimi`, `qwen`, and
`mock` (currently not instantiated) — use a dedicated `#[tokio::test]` per-adapter where the wire
shape diverges too much for the shared macro (e.g., Anthropic's event-accumulation stream).

---

### `crates/paladin-herald/src/json_herald.rs` / `markdown_herald.rs` / `table_herald.rs`

**Analog:** own current exact output strings (verified this session).

**JSON — current** (`~118-125` `paladin_result_to_json`, `~195-205` `finalize_stream`):
```rust
"token_count": result.token_count,
// ...
"total_tokens": metadata.token_usage.total_tokens,
```
**Target (D-21):** emit `"usage": <full TokenUsage object>` in both places; JSON always emits the
three optional keys as `null` when `None` (no `skip_serializing_if` on the herald's own
serialization — confirm `TokenUsage`'s own serde derive doesn't skip them either).

**Markdown — current** (`~254-256`, `~330-334`):
```rust
format!("**Token Count**: {}", result.token_count)
format!("**Total Tokens**: {}", metadata.token_usage.total_tokens)
```
**Target (D-22):** a "Token Usage" block — `Prompt`/`Completion`/`Total` always; `Cache
read`/`Cache write`/`Reasoning` rows only when `Some`.

**TableHerald name-matching pitfall (must fix in the same change, per RESEARCH.md Pitfall 2):**
```rust
// current (~217-222): matches (execution_time_ms, token_count) as a compound key
let name_pool: Vec<(String, u64, u32)> = /* (name, time, tokens.total_tokens) */;
// target: match key becomes (execution_time_ms, usage.total_tokens) — same semantics, new field path
```

---

### `src/application/services/paladin/paladin_execution_service.rs`

**Analog:** own current loop accumulator + 6 `PaladinResult` construction sites (verified,
`~1257`, `~1501-1502`, `~1216`/`~1468`/`~1533`/`~1814`/`~1841`/`~3065`).

**Current pattern:**
```rust
let mut total_tokens: u32 = 0; // ~1257
// ...
total_tokens += response.usage.total_tokens; // ~1501
middleware_cx.cumulative_tokens = total_tokens;
```
**Target pattern (D-12, D-06's `AddAssign`):**
```rust
let mut usage = TokenUsage::default();
// ...
usage += response.usage.clone();
middleware_cx.cumulative_tokens = usage.total_tokens; // unchanged semantics — TokenBudget unaffected
```
Vision site (`~1216`) maps `VisionTokenUsage` explicitly: `TokenUsage::new(vt.prompt_tokens,
vt.completion_tokens)` (untouched shape, per D-20's "vision adapters untouched").

---

### `crates/paladin-web/src/agent_controller.rs` (`ExecuteResponse`)

**Analog:** the `ResumeAcceptedResponse` new-DTO-with-`utoipa::ToSchema` precedent (MIGRATION.md
§9.2 line 190) — same "shipped-in-0.9.0 type, additive/replacing field, gets its own §9.2 row and
allowlist entry" treatment as `PaladinResult`'s `served_by` row, but scoped to `paladin-web`.

**New DTO pattern (D-24):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenUsageResponse {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}
impl From<TokenUsage> for TokenUsageResponse { /* field-for-field */ }
```
`ExecuteResponse.token_count: u32` → `usage: TokenUsageResponse`. Confirmed `utoipa` is already a
`paladin-web` dependency (`ExecuteResponse`'s existing `#[derive(... utoipa::ToSchema)]`).

---

## Shared Patterns

### Additive `#[serde(default)]` persisted field
**Source:** `crates/paladin-core/src/platform/container/waypoint.rs` (`attempts`, `cache_hit`,
`fork_of`), `execution_result.rs` (`served_by`)
**Apply to:** every new `usage`/optional field on `TokenUsage`, `StreamingResponse`,
`ChunkMetadata`, `NodeExecutionRecord`
```rust
#[serde(default)]
pub some_new_field: Option<T>,
```

### X-10.3 constructible-vs-`#[non_exhaustive]` decision
**Source:** MIGRATION.md §9.2 rows for `PaladinResult` (constructible, has `Default`) vs.
`LlmRequest`/`GarrisonEntry`/`ResumeAccepted` (`#[non_exhaustive]`, no `Default`)
**Apply to:** `TokenUsage` (constructible, D-04), `PaladinResult` (constructible, D-09),
`StreamingResponse`/`ChunkMetadata` (`#[non_exhaustive]`, D-13/D-18) — the deciding factor is
purely "does the type derive `Default` today?"

### `TraceDispatcher` synchronous accumulation inside `emit`
**Source:** `crates/paladin-battalion/src/engine/hooks.rs` `token_total: AtomicU64` +
`emit()`'s `fetch_add`
**Apply to:** the new `TokenUsage`-typed accumulator (`Mutex<TokenUsage>` or five atomics) —
preserve the "exact the instant `emit` returns" guarantee stated in the file's own rustdoc.

### MIGRATION.md §9.2 row templates
**Source:** `MIGRATION.md` lines 178 (`LlmRequest`, `#[non_exhaustive]`), 180 (`PaladinResult`,
constructible), 189 (`ResumeAccepted`, new-in-0.10 N/A)
**Apply to:** every §9.2 row this phase adds — copy the exact five-column format
(`crate | Type | Change | Compatibility mechanics + suppression | requirement_id`), and reuse the
"N/A — new trait/type introduced in v0.10, not a pre-existing public API" wording verbatim for
`NodeExecutionRecord`, `TraceEvent`, `CompletedRow`.

### `#[non_exhaustive]` wildcard-arm enforcement
**Source:** `crates/paladin-core/src/platform/container/trace.rs` module test (~437) requiring
every `match` over `TraceEvent` to carry a wildcard arm
**Apply to:** every call site that destructures `NodeFinished`/`RunFinished` after the field
rename (`hooks.rs`, `assertion.rs`, `inspector.rs`, `events.rs`, `engine/mod.rs` tests,
`overlay.rs`) — the compiler will catch stale field names; no new enforcement code needed.

## No Analog Found

None — every file in scope has at least a same-file precedent (its own current shape being the
thing under change) plus, for the genuinely novel mechanics (saturating `Option`-merge arithmetic
on `TokenUsage`, the cross-provider terminal-chunk usage contract), a design pattern supplied
directly in RESEARCH.md's Code Examples section (no prior art exists in this repo for those two
specific mechanics, which is expected — they are the phase's own new contribution, not a copy of
an existing pattern).

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`,
`crates/paladin-battalion/src/{engine,formation_service.rs,phalanx_service.rs}`,
`crates/paladin-ports/src/{output,input}/`, `crates/paladin-llm/src/{compat,openai,anthropic,
gemini,deepseek,mock.rs,fallback.rs,conformance.rs}`, `crates/paladin-herald/src/`,
`src/application/services/paladin/`, `crates/paladin-web/src/`, `crates/paladin-eval/src/`,
`MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`
**Files scanned:** every file named in RESEARCH.md's canonical_refs/Sources sections (all read
directly by the researcher this session); this pass re-read the four highest-leverage originals
(`token_usage.rs` full file, `execution_result.rs` relevant sections, `waypoint.rs`/`trace.rs`
relevant sections, `MIGRATION.md` §9.2 rows) to pull exact, current excerpts for this document.
**Pattern extraction date:** 2026-09-14
