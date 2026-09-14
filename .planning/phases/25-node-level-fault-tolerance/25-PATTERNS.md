# Phase 25: Node-Level Fault Tolerance - Pattern Map

**Mapped:** 2026-09-05
**Files analyzed:** 24 (new) + ~14 (modified)
**Analogs found:** 24 / 24

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/.../transience.rs` (new) | model (value type) | transform | `crates/paladin-core/.../directive.rs` (plain serde value enum) | role-match |
| `crates/paladin-core/.../node_error.rs` (new) | model (value type) | transform | `crates/paladin-core/.../waypoint.rs` `ParleyRequest`/`NodeExecutionRecord` (serde value, no live error objects) | exact |
| `crates/paladin-core/.../aegis.rs` (new) | model (policy value type) | CRUD (attached, read at validation/execution) | `crates/paladin-core/.../battalion/mod.rs` legacy `RetryPolicy`/`ErrorStrategy` (sibling policy enums) | role-match |
| `crates/paladin-core/.../node_cache.rs` (new, `CachedDelta`) | model (persisted value) | file-I/O (cache read/write) | `crates/paladin-core/.../waypoint.rs` `Waypoint` (schema_version + persisted struct) | exact |
| `crates/paladin-core/.../paladin_error.rs` (extend) | model / error taxonomy | transform | itself; `crates/paladin-ports/src/output/llm_port.rs` `LlmError` (sibling error enum with predicates) | exact |
| `crates/paladin-core/.../battalion/mod.rs` (extend `BattalionError`) | model / error taxonomy | transform | `paladin_error.rs` (`#[non_exhaustive]` + `_` arm pattern) | exact |
| `crates/paladin-core/.../waypoint.rs` (extend) | model | file-I/O (persisted) | itself — additive `#[serde(default)]` field precedent already in file (`attempt` field, `AwaitingInput.parleys`) | exact |
| `crates/paladin-core/.../battlefield.rs` (extend `FieldSpec.cache`) | model / schema | CRUD | itself — `FieldSpec { name, dispatch, default, required }` | exact |
| `crates/paladin-core/.../execution_result.rs` (extend `PaladinResult.served_by`) | model | request-response | itself — existing `#[serde(default, skip_serializing_if)]` fields (`plan`, `handoff_history`) | exact |
| `crates/paladin-ports/src/output/llm_port.rs` (extend `LlmError`) | port (trait + error enum) | request-response | itself | exact |
| `crates/paladin-ports/src/output/paladin_port.rs` (extend, `execute_observed`) | port (trait, defaulted method) | streaming | itself — `execute`/`execute_stream` shape | exact |
| `crates/paladin-ports/src/output/node_cache_port.rs` (new) | port | file-I/O (CRUD-ish get/put/invalidate) | `crates/paladin-ports/src/output/waypoint_port.rs` (persistence port shape) | role-match |
| `crates/paladin-ports/src/output/trace_sink_port.rs` (extend `TraceEvent`) | port (event enum) | event-driven | itself | exact |
| `crates/paladin-llm/src/{openai,deepseek,anthropic,kimi,qwen,grok,gemini,ollama,openai_compatible}/adapter.rs` (extend non-2xx arm) | service adapter | request-response | `openai/adapter.rs:374-396` non-2xx match (the reference shape) | exact |
| `crates/paladin-llm/src/http_status.rs` (new, `map_http_status` helper — name at discretion) | utility | transform | `crates/paladin-llm/src/redaction.rs` (crate-level shared helper extracted from one adapter, applied everywhere) | exact |
| `crates/paladin-llm/src/fallback.rs` (new, `FallbackLlmAdapter`) | service adapter (composing port) | streaming + request-response | `crates/paladin-llm/src/mock.rs` `MockLlmAdapter` (LlmPort impl composing/faking behavior) | role-match |
| `crates/paladin-battalion/src/aegis_retry.rs` or `engine/retry.rs` (new) | service (execution logic) | event-driven (per-attempt loop) | `crates/paladin-battalion/src/edge_evaluator.rs` (registry + trait-object dispatch) for `RetryPredicate`; `engine/superstep.rs:1515-1595` dispatch closure for loop placement | exact |
| `crates/paladin-battalion/src/error_handler.rs` (new, `ErrorHandler` trait + registry) | service (registry) | event-driven | `crates/paladin-battalion/src/edge_evaluator.rs` (`EdgeConditionEvaluator` + `EdgeEvaluatorRegistry`) | exact |
| `crates/paladin-battalion/src/engine/graph.rs` (extend `WarGraph` sidecar + `validate`) | model / registry (sidecar builder) | CRUD | itself — `mark_dynamic_target`/`add_worker_template` sidecar-set pattern (lines 448-568) | exact |
| `crates/paladin-battalion/src/engine/mod.rs` (extend `WarEngine` builders, `EngineError`) | service (builder + orchestrator) | request-response | itself — `with_edge_evaluator`/`with_trace_sink`/`with_shutdown_grace` (lines 971-1052) | exact |
| `crates/paladin-battalion/src/engine/superstep.rs` (extend dispatch closure) | service (per-node execution) | event-driven | itself — the spawned closure at lines 1515-1599 | exact |
| `crates/paladin-battalion/src/engine/node.rs` (extend `NodeContext`, rename `NodeError`→`StateNodeError`) | model | transform | itself | exact |
| `crates/paladin-battalion/src/engine/hooks.rs` (doc-only touch) | service (interceptor contract) | event-driven | itself — existing rustdoc at 208-212 | exact |
| `crates/paladin-storage/src/node_cache/{in_memory,redis,contract_tests}.rs` (new) | service adapter (3-backend) | file-I/O (cache CRUD) | `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres,contract_tests}.rs` + `crates/paladin-storage/src/redis.rs` `ConnectionManager` pattern | exact |
| `src/config/node_cache.rs` (new) | config | CRUD (load) | `src/config/waypoint_store.rs` / `src/config/waypoint_retention.rs` | exact |
| `src/config/engine.rs` (extend, land `run_timeout_secs` semantics) | config | CRUD (load) | itself | exact |
| `src/infrastructure/resilience/circuit_breaker.rs` (no code change, boundary doc) | middleware | request-response | itself | exact |
| `tests/helpers/mock_paladin_port.rs` (extend `FaultyPaladinPort`) | test double | event-driven | itself — existing `fail_until_attempt`/`fail_paladin` builder | exact |
| `tests/integration/e2e_muster_defer_order_test.rs` (replace seam) | test | event-driven | itself — fenced "PHASE 25 SEAM" block (350-388) | exact |
| `tests/integration/e2e_crash_resume_test.rs` (template for new E2E) | test | event-driven | itself | exact |
| `docs/src/user-guides/fault-tolerance.md` (new) | docs | — | `docs/src/user-guides/parley-and-chronicle.md` | exact |

## Pattern Assignments

### `crates/paladin-core/.../transience.rs`, `node_error.rs`, `aegis.rs`, `node_cache.rs`

**Analog:** `crates/paladin-core/src/platform/container/waypoint.rs` (persisted value type family)

**Core pattern** (waypoint.rs:501-551, `NodeOutcomeKind` / `NodeExecutionRecord`):
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeOutcomeKind {
    Succeeded,
    Failed,
    Skipped { reason: String },
    Ended,
    Parleyed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    pub node_id: NodeId,
    pub paladin_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub token_count: u64,
    pub outcome: NodeOutcomeKind,
    pub attempt: u32,
}
```
Copy this shape for `NodeError`/`NodeErrorSource`/`AttemptRecord`/`CachedDelta`: plain
`Clone + PartialEq + Serialize + Deserialize` structs/enums, doc comments citing the FR that
motivates each field, no live error objects — only summaries built by `From` conversions
(D-07).

**Additive-field precedent** (waypoint.rs:562-567, `WaypointStatus::Failed`):
```rust
Failed {
    error: String,
    failed_node: NodeId,
},
```
D-08 adds `#[serde(default)] node_error: Option<NodeError>` beside these two, exactly the
same additive-field-on-an-existing-variant technique used for `attempt` on
`NodeExecutionRecord` (already present at line 550, "Populated meaningfully once per-node
retry lands (Doc 04); `1` until then" — literally waiting for this phase).

---

### `crates/paladin-battalion/src/error_handler.rs` (new `ErrorHandler` trait + registry), and `RetryPredicate` registration

**Analog:** `crates/paladin-battalion/src/edge_evaluator.rs` (full file — registry + fail-closed pattern, CF-01)

**Imports** (edge_evaluator.rs:21-27):
```rust
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::battlefield::Battlefield;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use thiserror::Error;
```

**Trait pattern** (edge_evaluator.rs:62-78):
```rust
#[async_trait]
pub trait EdgeConditionEvaluator: Send + Sync {
    async fn evaluate(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError>;
}
```
Copy verbatim for `ErrorHandler::handle(&self, err: &NodeError, state: &Battlefield) ->
Result<Directive, NodeError>` (D-13) and for `RetryPredicate` if it needs to be a trait
object rather than a plain enum.

**Registry pattern** (edge_evaluator.rs:112-141):
```rust
#[derive(Default, Clone)]
pub struct EdgeEvaluatorRegistry {
    inner: HashMap<String, Arc<dyn EdgeConditionEvaluator>>,
}

impl EdgeEvaluatorRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, name: impl Into<String>, evaluator: Arc<dyn EdgeConditionEvaluator>) {
        self.inner.insert(name.into(), evaluator);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn EdgeConditionEvaluator>> {
        self.inner.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }
}
```
`Clone` is load-bearing — a child `Battalion` node inherits the parent's registries wholesale
(D-13 cites this exact requirement), and forwarding into a `tokio::spawn`'d task needs an
owned copy. Bundle all three registries (`edge_evaluators`, `retry_predicates`,
`error_handlers`) into one `EngineRegistries` struct per D-13 rather than growing
`WarGraph::validate`'s parameter list a third time.

**Fail-closed error shape** (edge_evaluator.rs:80-92):
```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EdgeEvaluatorError {
    #[error("evaluator '{evaluator}' failed: {reason}")]
    Evaluation { evaluator: String, reason: String },
}
```

---

### `crates/paladin-battalion/src/engine/graph.rs` — `WarGraph::set_aegis` / `with_default_aegis`

**Analog:** itself, `mark_dynamic_target` / `add_worker_template` (graph.rs:448-568)

**Sidecar storage field** (graph.rs:461-468):
```rust
dynamic_targets: HashSet<NodeId>,
worker_templates: HashSet<NodeId>,
```
Add `aegis: HashMap<NodeId, Aegis>` and `default_aegis: Option<Aegis>` beside these,
initialized empty in `WarGraph::new` (graph.rs:477-489).

**Chainable builder + validation-listing-offenders pattern** (graph.rs:544-547, `mark_dynamic_target`):
```rust
pub fn mark_dynamic_target(&mut self, id: NodeId) -> &mut Self {
    self.dynamic_targets.insert(id);
    self
}
```
Copy this `&mut Self` chainable shape for `set_aegis(node_id, Aegis)`. The "list every
offender before any node executes" fail-closed validation contract (this file's own module
doc, lines 440-447: "that every declared node is in the eligible set... reachable... or
marked `mark_dynamic_target`") is the template for rejecting `set_aegis` on an undeclared
node, and for D-12's node-kind matrix (`retry`/`cache` rejected on `Battalion`, any `Aegis`
rejected on `Gate`) — same "typed validation error naming the reason" contract this file
already uses for its own worklist checks.

---

### `crates/paladin-battalion/src/engine/superstep.rs` — Aegis retry loop around the dispatch closure

**Analog:** itself, lines 1500-1599 (the exact closure D-14 says the retry loop must wrap)

**Core dispatch shape** (superstep.rs:1515-1599, abbreviated):
```rust
handles.push(IndexedHandle {
    index: dispatch_index,
    handle: tokio::spawn(async move {
        node_trace.emit(TraceEvent::NodeStarted { thread_id: ctx.thread_id.clone(), superstep: ctx.superstep, node_id: nid.clone() });
        let started_at = Utc::now();

        // before-interceptor chain, short-circuiting on non-Proceed
        let mut decision = InterceptDecision::Proceed;
        for interceptor in &node_interceptors {
            decision = interceptor.before(&ctx, &snap).await;
            if !matches!(decision, InterceptDecision::Proceed) { break; }
        }

        let (paladin_id, token_count, outcome) = match decision {
            InterceptDecision::Skip(reason) => (None, 0u64, NodeRunOutcome::Skipped(reason)),
            InterceptDecision::Fail(err) => (None, 0u64, NodeRunOutcome::Failed(NodeFailure::Node(err))),
            InterceptDecision::Proceed => match sem.acquire_owned().await {
                Ok(_permit) => {
                    let (paladin_id, token_count, result) =
                        execute_vanguard_node(dispatch, &snap, &ctx, &port).await;
                    match result {
                        Ok(mut directive) => {
                            for interceptor in &node_interceptors {
                                interceptor.after(&ctx, &mut directive.delta).await;
                            }
                            (paladin_id, token_count, NodeRunOutcome::Succeeded(directive))
                        }
                        Err(e) => (paladin_id, token_count, NodeRunOutcome::Failed(e)),
                    }
                }
                Err(_) => (None, 0u64, NodeRunOutcome::Failed(/* typed internal error, never panic */)),
            },
        };
        let duration_ms = (Utc::now() - started_at).num_milliseconds().max(0) as u64;
        node_trace.emit(TraceEvent::NodeFinished { .. });
        (nid, started_at, duration_ms, paladin_id, token_count, outcome)
    }),
});
```
D-14's instruction — "the retry loop lives around the `tokio::spawn` body ... so interceptors
run once per attempt" — means: wrap this *entire* body (from `NodeStarted` emit through
`NodeFinished` emit) in a `for attempt in 1..=max_attempts` loop with a `RetryPredicate` check
and a `select!`-against-`CancellationToken` backoff sleep (D-15) between iterations, never
inside it. Note the existing library-code discipline at lines 1567-1582: no `.expect()`/panic
on an unreachable arm, report through the existing typed-error plumbing instead — follow this
for any new "should never happen" arm the retry loop introduces.

---

### `crates/paladin-battalion/src/engine/mod.rs` — `WarEngine::with_retry_predicate` / `with_error_handler` / `with_node_cache`

**Analog:** itself, `with_edge_evaluator` / `with_trace_sink` / `with_shutdown_grace` (mod.rs:963-1052)

**Builder pattern** (mod.rs:971-978):
```rust
pub fn with_edge_evaluator(
    mut self,
    name: impl Into<String>,
    evaluator: Arc<dyn EdgeConditionEvaluator>,
) -> Self {
    self.edge_evaluators.register(name, evaluator);
    self
}
```
Copy this exact `mut self -> Self` consuming-builder shape for the three new methods. Note
the rustdoc convention each builder follows: cite the FR/decision id, state default behavior
when never called, and (for `with_shutdown_grace`) include a doc-tested `# Examples` block
(mod.rs:1019-1048) — follow this for `with_node_cache` since it is a new public API surface
needing a doc test (X-08).

---

### LLM provider non-2xx mapping (`D-03`, `FT-FR-02a`)

**Analog:** `crates/paladin-llm/src/openai/adapter.rs:374-396` (the reference shape named directly in CONTEXT.md)

**Current per-status match to replace with `map_http_status(...)`:**
```rust
if !status.is_success() {
    return match status.as_u16() {
        401 => Err(LlmError::AuthenticationError("Invalid OpenAI API key".to_string())),
        429 => Err(LlmError::RateLimitExceeded),
        400 => {
            if response_text.contains("maximum context length") {
                Err(LlmError::TokenLimitExceeded)
            } else {
                Err(LlmError::InvalidPrompt(response_text))
            }
        }
        500..=599 => Err(LlmError::ProcessingError(format!("OpenAI server error: {}", response_text))),
        _ => Err(LlmError::ProcessingError(format!("HTTP {}: {}", status, response_text))),
    };
}
```
D-03: keep the dedicated 401/429/400 arms byte-identical; replace only the `500..=599` and
final `_` catch-all arms with a call to the new shared `map_http_status(provider, status,
redacted_excerpt) -> LlmError` helper that returns `LlmError::ProviderError { provider,
status, message }`. Apply the same replacement to all nine adapters' equivalent match blocks
(deepseek/adapter.rs:501-520 is the other named reference).

**Redact-before-bound helper pattern to model the new helper on** (`crates/paladin-llm/src/redaction.rs:1-51`):
```rust
//! **Ordering is load-bearing: redact, then bound.** Bounding a response body
//! before redaction can slice a secret in half at the truncation boundary and
//! leak the surviving prefix.
pub const RESPONSE_EXCERPT_CHAR_BUDGET: usize = 512;
pub fn bounded_excerpt(body: &str, budget: usize) -> String { /* char-count-safe truncation */ }
```
`map_http_status` MUST call `redact_credentials` before `bounded_excerpt`, exactly per this
module's own load-bearing ordering rule (security.instructions.md, D-34) — never the reverse.

---

### `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter`

**Analog:** `crates/paladin-llm/src/mock.rs` `MockLlmAdapter` (a `LlmPort` impl that composes/simulates behavior rather than calling a real provider — closest existing shape to a chain-composing adapter) and `crates/paladin-ports/src/output/llm_port.rs` for the trait it implements (`generate` 1073, `generate_stream` 1173, `get_provider_name` 1291, `get_capabilities` 1363).

Implement `LlmPort` for `FallbackLlmAdapter { chain: Vec<Arc<dyn LlmPort>> }` the same way
`MockLlmAdapter` implements every trait method by delegating/simulating rather than making a
real HTTP call — `FallbackLlmAdapter` delegates to `chain[i]` in order instead. `get_provider_name`
returns `"fallback"`; `get_capabilities`/`validate_model`/`get_available_models` delegate to
the first element that answers (D-24).

---

### `crates/paladin-storage/src/node_cache/{in_memory,redis,contract_tests}.rs`

**Analog:** `crates/paladin-storage/src/redis.rs` (`ConnectionManager` pattern, lines 1-90) + the sibling `waypoint/{in_memory,sqlite,postgres,contract_tests}.rs` three-backend + one-contract-suite shape (structure only; not re-read in full since D-27 says to mirror it verbatim).

**Config-struct pattern to mirror for `NodeCacheConfig`-adjacent Redis settings** (redis.rs:20-45):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisQueueConfig {
    pub redis_host: String,
    pub redis_port: u16,
    pub redis_password: Option<String>,
    pub redis_db: u8,
    pub connection_timeout: u64,
    pub key_prefix: String,
    pub max_retries: u32,
}

impl Default for RedisQueueConfig {
    fn default() -> Self {
        Self { redis_host: "localhost".to_string(), redis_port: 6379, redis_password: None,
               redis_db: 0, connection_timeout: 30, key_prefix: "paladin:queue".to_string(),
               max_retries: 3 }
    }
}
```
**Key-namespacing helper pattern** (redis.rs:47-81, `queue_key`/`priority_queue_key`/...):
```rust
fn queue_key(config: &RedisQueueConfig, queue_name: &str) -> String {
    format!("{}:queue:{}", config.key_prefix, queue_name)
}
```
`RedisNodeCache` should have an analogous `cache_key(prefix, hash) -> String` free function,
and import `redis::{AsyncCommands, Client, aio::ConnectionManager}` exactly as this file does
(line 3) — no new Redis client type.

---

### `src/config/node_cache.rs`

**Analog:** `src/config/waypoint_store.rs` / `src/config/waypoint_retention.rs` (X-09 template, not re-read here — CONTEXT.md/D-29 names them directly as the mirror target: `Default` + `validate()` + `EnvOverridable`, `APP_*` prefix, `enabled: false` by default).

---

### `tests/helpers/mock_paladin_port.rs` — `fail_paladin_until_attempt`

**Analog:** itself — the existing `fail_until_attempt` global counter / `fail_paladin(name)` chainable builder (D-31: "additive, chainable per-Paladin counter... its global `fail_until_attempt` semantics stay exactly as the recorded STATE.md decision describes"). Add the new method beside the existing ones without altering their behavior; return `PaladinError::LlmFailure { status: Some(503), .. }` (the new D-02 structured variant, Transient-classified) rather than any string-based error.

---

## Shared Patterns

### Fail-closed registry + validation (CF-01)
**Source:** `crates/paladin-battalion/src/edge_evaluator.rs` (whole file)
**Apply to:** `EngineRegistries` (retry predicates, error handlers), `WarGraph::validate`'s
`set_aegis`/node-kind checks, `NodeCachePort` presence check (D-29). Every unregistered name
or invalid attachment must be collected and reported together, before any node executes —
never a first-offender-only error.

### Per-node sidecar annotation on `WarGraph`
**Source:** `crates/paladin-battalion/src/engine/graph.rs:448-568`
**Apply to:** `WarGraph::set_aegis`/`with_default_aegis` (D-10), any other new per-node marker
this phase needs. Pattern: a `HashMap<NodeId, T>` or `HashSet<NodeId>` field beside
`defer_flags`/`dynamic_targets`/`worker_templates`, a chainable `&mut Self` setter, a `bool`/
`Option` getter, validated at `WarGraph::validate` time.

### Additive `#[serde(default)]` persisted fields, no schema bump
**Source:** `crates/paladin-core/src/platform/container/waypoint.rs` (`NodeExecutionRecord.attempt`,
`WaypointStatus::AwaitingInput`)
**Apply to:** `NodeExecutionRecord.attempts`/`cache_hit`, `WaypointStatus::Failed.node_error`,
`FieldSpec.cache` (D-08, D-16, D-29). Never reshape an existing field; always `Option`/`bool`
with a sensible default that reproduces pre-phase behavior when absent.

### Redact-before-bound for every provider-sourced error string
**Source:** `crates/paladin-llm/src/redaction.rs`
**Apply to:** `map_http_status`'s excerpt (D-03), any `NodeError`/`NodeErrorSource::Llm`
message built from a provider response (D-34). Ordering is load-bearing and MUST NOT be
reversed.

### `Arc<dyn Trait>` + async_trait for every new engine-visible port/registry entry
**Source:** `crates/paladin-battalion/src/edge_evaluator.rs`, `crates/paladin-ports/src/output/paladin_port.rs`
**Apply to:** `ErrorHandler`, `RetryPredicate` (if trait-object shaped), `NodeCachePort`.

### `#[non_exhaustive]` + `_` arm + register row + allowlist entry, same commit
**Source:** existing `EdgeEvaluatorError`, `NodeOutcomeKind`, `WaypointStatus`, `TraceEvent`
(all already `#[non_exhaustive]` in this tree)
**Apply to:** `PaladinError`, `LlmError`, `BattalionError` (D-04) — every in-tree exhaustive
match on these three gains a `_` arm in the same commit that adds the attribute.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| Pausable backoff test (`tokio::time::pause`/`start_paused` usage in the new retry-loop test module) | test | timing/event-driven | RESEARCH.md confirms zero existing `tokio::time::pause` usage anywhere in `paladin-battalion` src or tests — this is a genuinely new test pattern in this codebase; follow the standard `tokio::test(start_paused = true)` idiom from `tokio`'s own docs, not an in-repo precedent. Budget extra iteration here per RESEARCH.md's explicit flag. |
| `HeartbeatHandle` (`Arc<tokio::sync::watch::Sender<Instant>>` or similar) | utility/model | event-driven | No existing struct in `engine/` holds a `tokio::sync::watch` handle; nearest analog (`CancellationToken` field on `WarEngine`) is a different primitive shape. Build per RESEARCH.md's recommendation (watch-channel over hand-rolled Mutex+Notify). |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`,
`crates/paladin-ports/src/output/`, `crates/paladin-llm/src/{openai,deepseek,redaction,mock}*`,
`crates/paladin-battalion/src/{edge_evaluator.rs,engine/{graph,mod,superstep,node,hooks}.rs}`,
`crates/paladin-storage/src/{redis.rs,waypoint/}`, `src/config/`,
`src/infrastructure/resilience/circuit_breaker.rs`, `tests/helpers/`, `tests/integration/`,
`docs/src/user-guides/`.
**Files scanned:** 10 read directly (edge_evaluator.rs, graph.rs, superstep.rs, mod.rs,
redaction.rs, openai/adapter.rs, redis.rs, waypoint.rs) plus CONTEXT.md/RESEARCH.md file:line
anchors trusted for files not independently re-read (waypoint_store.rs, mock.rs, llm_port.rs,
node.rs, hooks.rs, paladin_port.rs — all cited with exact line numbers in the upstream docs and
consistent with the patterns confirmed in files that were read).
**Pattern extraction date:** 2026-09-05
