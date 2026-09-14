# Phase 26: Agent Runtime Enhancements - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected on recommended defaults; audit trail in 26-DISCUSSION-LOG.md)

<domain>
## Phase Boundary

Phase 26 delivers epic `RT` (PRD 05) on top of the Phase 22/22.1/23/24/25 tree, and nothing from
later epics beyond the seams PRD 05 itself declares:

1. **Execution middleware chain (RT-01).** `PaladinExecutionService` gains an ordered
   `Vec<Arc<dyn ExecutionMiddleware>>` with `before_model` / `after_model` / `around_tool` hooks,
   onion ordering (`before_model` first-to-last, `after_model` last-to-first), short-circuit
   semantics (`Finish` from #2 skips #3's `before` and still runs #1's `after`), per-run state
   isolation proven by a multi-thread test, and the same chain applying when a Paladin runs as a
   `WarEngine` node — with the `NodeInterceptor`-vs-middleware two-layer distinction documented
   (RT-FR-01…03).
2. **Built-in middleware (RT-02).** `ModelCallLimit` and `TokenBudget` finishing with new
   `StopReason::CallLimit` / `StopReason::TokenBudget` variants (X-10 decision applied to both
   in one change, §9.2-registered), `ToolCallLimit` denying without failing the run, `Guardrail`
   prompt/response screens with `Fail` / `Redact` / `Finish`, and retry/fallback middleware that
   delegate to Phase 25's `RetryPolicy` and `FallbackLlmAdapter` without duplicating logic — every
   built-in config-structured per X-09 (RT-FR-04…07, RT-FR-09).
3. **Context-window management (RT-03).** A `TokenCounterPort` (heuristic default, tiktoken where
   available — never an inline heuristic in budget code), a stable never-splits-a-message
   `HistoryTrimmer`, and a compounding `SummarizationMiddleware` persisting summaries to Garrison
   flagged `is_summary: true` (the entry field `#[serde(default)]`, the SQLite column additively
   migrated, §9.2/§9.4-registered), degrading to trimming on summarizer failure and never failing
   the run (RT-FR-08, RT-FR-10…12).
4. **Vault cross-session memory (RT-04).** `VaultPort` (put/get/delete/list/search) with InMemory,
   SQLite and semantic adapters under one contract suite; `vault_get` / `vault_put` Armaments
   confined to a host-granted namespace subtree (`VaultError::NamespaceDenied` on traversal,
   attack-tested); `NodeContext::vault()`; opt-in `VaultRecallMiddleware`; the
   Garrison/Waypoint/Vault three-way distinction documented with a table (RT-FR-13…16).
5. **Structured output (RT-05).** `execute_structured<T>` reachable through a new
   `StructuredExecutorPort` (not `PaladinPort`), schemars-generated schemas in the application
   layer (MSRV-verified per X-11), native provider JSON modes through an additive
   `LlmRequest.response_format` (X-10.3 handled and registered), a bounded repair loop with a typed
   `PaladinError::StructuredOutputInvalid` preserving raw output, and engine Paladin nodes with an
   `output_schema` writing parsed JSON to `output_field` — with `StructuredDirective` reusing the
   same extraction machinery (RT-FR-17…19).
6. **Provider conformance close-out (RT-06).** Verify-then-fix, not greenfield: the shipped
   v0.8.0 OpenAI-compatible, Gemini and Ollama paths are measured against a shared conformance
   suite (success, streaming, FT-01-correct 429/5xx/408 transience by value, credential redaction),
   the documented Ollama recipe points at the existing env-probed integration suite, and only
   measured gaps are closed (RT-FR-20…22).
7. **Prebuilt agent & tool ergonomics (RT-07).** `reasoning_agent(llm, tools, opts)` as a ≤15-line
   doc-tested one-liner, and tool failures fed back into the model context by default (sanitized;
   `tool_error_mode: FeedToModel | FailRun` with per-tool override) with the chosen default and
   rationale recorded as `MIGRATION.md` M-B-03 (RT-FR-23, RT-FR-24).

**Out of this phase:** LLM-native tool calling on `LlmRequest` and the shipped adapters
(ADR-0042 stays deferred with its trigger unchanged — this phase adds a prompt-level protocol, not
a wire-level one); automatic Vault write/extraction policies (FUT-06); per-token cost accounting
(FUT-08); provider rate-limit pacing (FUT-09); UI for memory management (PRD 05 §5);
rebuilding any v0.8.0 adapter (REQUIREMENTS.md out-of-scope table); the authoritative
`TraceEvent` enum and any new trace consumer (OBS-01/02, Phase 28); background runs, assistants
and the per-user platform surface that would derive a `RunScope` from an HTTP request (PLAT-*,
Phase 27); `MIGRATION.md` §9.8 finalisation and the v0.9-config boot test (SHIP-01/02, Phase 29).
Any other behavioral change discovered mid-implementation is an X-03 stop-and-flag event, not a
judgment call.

</domain>

<decisions>
## Implementation Decisions

PRD 05 is the FR-level source of truth and already locks the trait sketches (§2.1, §2.3, §2.4),
the FR semantics (RT-FR-01…24), the acceptance criteria (§3), the TDD ordering (§4) and the
out-of-scope list (§5). The decisions below settle only what PRD 05 left open, or what the shipped
Phase 22–25 tree makes concrete — including four places where the tree contradicts a PRD premise
(D-02, D-29, D-31, D-32), which are flagged with ⚠ for the developer to overturn at plan review if
wanted. Do not re-litigate anything PRD 05, PRD 01–04, overview §3 (X-01…X-11) or the Phase
22/22.1/23/24/25 CONTEXT decisions state.

### Middleware chain: home, contexts & hook placement (RT-01; RT-FR-01…03)

- **D-01: The trait and every built-in live in the facade, beside the service they hook.**
  `src/application/services/paladin/middleware/{mod,chain,context,limits,guardrail,history,
  summarization,vault_recall,resilience,tool_protocol}.rs` (file split at Claude's discretion),
  re-exported from `paladin::prelude`. `ExecutionMiddleware`, `MiddlewareFlow`, `ToolFlow`,
  `ModelCallContext`, `ToolCallContext`, `LlmResponseView` and `FinalResult` are all new types; no
  X-10 row. Rationale: the house pattern puts a hook beside the thing it wraps
  (`NodeInterceptor` in `paladin-battalion::engine::hooks`, `EdgeConditionEvaluator` in
  `paladin-battalion`), and every consumer of the chain already depends on the facade because
  `PaladinExecutionService` lives there. Rejected: `paladin-ports` — it would let a third-party
  crate implement middleware without the facade, but no such consumer exists and the contexts
  expose the facade's own prompt-assembly types. — **Reversibility:** costly — a public trait
  moved between published crates later touches facade re-exports and every downstream `use`.
- **D-02: The prompt buffer is a structured assembly.** The "mutable message/prompt buffer" is a structured `PromptAssembly`, rendered after
  `before_model`. Today `execute_internal` builds one flat `String` per loop iteration
  (`paladin_execution_service.rs:1112-1153`: system prompt, `## Relevant Context from Memory`,
  `Previous conversation:` with the newest 10 of `recall_recent(20)`, `User: {input}`,
  `Previous output: {accumulated}`) and sends it as `PromptType::User(UserPrompt { query })`;
  there is no message list. So the context carries `PromptAssembly { system, retrieved_context:
  Option<String>, history: Vec<GarrisonEntry>, input, accumulated_output, sections:
  Vec<PromptSection> }` (field set at Claude's discretion) and the service renders it to the flat
  string **after** the `before_model` chain runs, through the same code path as today. ⚠ **PRD 05's
  "messages" wording assumed a message list**; the tree has a string prompt, and turning it into a
  provider message list is ADR-0042 / X-03 territory this phase does not enter. Locked invariant:
  with an empty chain the rendered prompt bytes, the port call count and the `PaladinResult` are
  identical to today's — proven by a golden equivalence test on `MockLlmAdapter` (the
  `engine::hooks` empty-chain equivalence precedent). `HistoryTrimmer`, `SummarizationMiddleware`,
  `VaultRecallMiddleware`, `ToolCallProtocol` and `Guardrail(Prompt)` all mutate the assembly, never
  the rendered string.
- **D-03: Middleware are stateless; run state lives on the context.** Middleware are stateless `Arc<dyn>` values; all per-run mutable state lives on the
  context. `ModelCallContext` carries `run_id`, `loop_index`, `cumulative_tokens`, the assembly,
  the read-only `&Paladin`, `scratch: HashMap<String, serde_json::Value>` (PRD) and a typed
  per-run bag (`state::<T>()` / `state_mut::<T>()` keyed by middleware name + `TypeId`, at
  discretion). No `MiddlewareFactory`. `ModelCallLimit`'s counter, `TokenBudget`'s total and
  `ToolCallLimit`'s per-tool counts are per-run scratch by construction, so RT-FR-02's
  multi-thread test (two concurrent runs through one service instance, exact independent counts,
  `flavor = "multi_thread"`, timeout guard — the `listener.rs` X-05 house pattern) passes without
  a factory. Rejected: a factory trait — it doubles every built-in's surface for no isolation
  gain.
- **D-04: Where the hooks fire relative to the existing machinery.** `before_model` runs once per
  loop iteration after the assembly is built and before `execute_with_retry_and_temperature`
  (`:862`); `after_model` runs on the response that call returns — i.e. **after** the service's own
  buffered retry and the `CircuitBreaker`, so a middleware sees one final response per iteration,
  never per attempt. `around_tool` wraps **both** the Arsenal branch (`:922-968`) and the handoff
  branch (`:891-921`) — a handoff is a tool call the model made — with `ToolCallContext { call:
  ArmamentCall, kind: Armament | Handoff, loop_index, .. }`; `ToolFlow::Deny { reason }` injects
  `reason` into the accumulated output exactly where a tool error is injected today, and
  `Rewrite(call)` replaces the call before dispatch. Layer-1 planning / prompt-generation calls,
  the summarizer's own model call and a handoff *specialist's* run are outside this run's chain
  (the specialist runs as its own run through `PaladinExecutorPort`, with its own scratch —
  documented). The **streaming path** (`execute_stream_inner`, `:2045`, one model call, no tool
  loop) runs `before_model` only; `after_model`/`around_tool` are not invoked there and
  `execute_stream`'s rustdoc says so — response screens on streams are a deferred idea.
- **D-05: Engine bridging is automatic and documented, not a second registry.** A `WarEngine`
  dispatches every Paladin node through `PaladinPort::execute_observed` (`superstep.rs:946`), so a
  `PaladinExecutionService` carrying middleware applies it unchanged as a node; no engine change is
  needed for RT-FR-03. The two layers are documented on both traits and in the guide with a table
  (`NodeInterceptor` = around the whole node, once per attempt, inside the Aegis; `ExecutionMiddleware`
  = inside the reasoning loop, per model call / per tool call), and one integration test runs a
  Paladin node under an engine with a recording middleware and asserts the hook sequence. The
  Phase 23 deferred idea "`NodeInterceptor` visibility of `NextStep`" stays deferred.
- **D-06: Chain attachment.** `PaladinExecutionService::with_middleware(Arc<dyn ExecutionMiddleware>)`
  (chainable, appends) and `with_middleware_chain(Vec<..>)` (replaces), mirroring `with_herald` /
  `with_rag_retrieval` (`:238-403`); `new()` keeps its signature (X-03). `MiddlewareFlow::Finish`
  carries `FinalResult { output, stop_reason }`; `Fail(PaladinError)` fails the run with the
  middleware's error unchanged; a `Fail` from a `before_model` hook is not retried. A panic inside
  a hook is not caught (the house rule: no `catch_unwind` in library code).

### Built-in limits & guardrails (RT-02; RT-FR-04…07)

- **D-07: `StopReason` becomes `#[non_exhaustive]`.** `StopReason` becomes `#[non_exhaustive]` — the X-10.2 exception is *not* taken.
  `paladin_core::platform::container::execution_result::StopReason` (`execution_result.rs:102`,
  four variants, not `#[non_exhaustive]`, matched exhaustively only by first-party display
  mappers: `src/application/cli/formatters/output.rs:281-292, 496-498` and
  `crates/paladin-web/src/agent_controller.rs:151-154`) gains `CallLimit` and `TokenBudget` in
  **one** change, is marked `#[non_exhaustive]`, every in-tree match gains a `_` arm, the doc
  comment lists the variants added in this release, and the §9.2 row is resolved `Y`
  (`enum_marked_non_exhaustive`) with the `.cargo/semver-checks-allowlist.toml` entry and the
  `crates/paladin-core/Cargo.toml` lint suppression in the same commit — exactly the D-04 treatment
  of `PaladinError` / `LlmError` / `BattalionError`. Overview §3 X-10.2 names `StopReason` as the
  *example* of a type where exhaustive matching might be legitimate; nothing in the tree or the
  examples relies on it, and one mechanism across four enums beats a bespoke exception. — **Reversibility:**
  costly — the attribute is a one-release decision; the `_` arms stay.
- **D-08: Limit semantics.** `ModelCallLimit` counts the reasoning loop's model calls (one per
  `before_model`, post-retry — the service's buffered retries and the summarizer's calls are not
  counted, documented); on `calls >= max` it returns `Finish` with the accumulated output plus an
  appended truncation notice and `StopReason::CallLimit`. `TokenBudget` accumulates
  `response.usage.total_tokens` in `after_model` (the existing `total_tokens` running sum) and,
  once the budget is crossed, finishes with the response that crossed it kept, the notice appended
  and `StopReason::TokenBudget` — the overshoot is at most one response and is documented.
  `is_successful()` returns **`true`** for both new variants (PRD RT-FR-04's "true-with-warning";
  the run ended with the model's last answer intact) and `is_limit()` returns `true`;
  `MaxLoops` / `Timeout` keep today's answers (X-03) and the asymmetry is stated on the enum.
  `ToolCallLimit { max_calls, per_tool: HashMap<String, u32> }` denies with a fixed model-facing
  message ("tool budget exhausted for `<tool>`") through `ToolFlow::Deny` — never a run failure —
  and applies to handoff calls too (D-04).
- **D-09: `Guardrail` rules.** `GuardrailRule { name, target: Prompt | Response | Both, matcher:
  Regex(String) | Predicate(Arc<dyn Fn(&str) -> bool + Send + Sync>), on_match: Fail |
  Redact(String) | Finish(String) }`; regexes compile at construction with a typed error and a
  documented pattern-size bound (the `regex` crate is linear-time, so there is no ReDoS surface —
  stated in the security section); `Predicate` is code-only, `Regex` rules are config-supplied
  (D-10). `Prompt` screens run in `before_model` over the rendered-prompt *parts* (assembly text
  sections, so a redaction lands in the right section), `Response` screens in `after_model` over
  `response.content`. `Fail` → new structured `PaladinError::GuardrailTripped { rule, target }`
  (X-06; `PaladinError` is already `#[non_exhaustive]` — the §9.2 row's Change cell is extended,
  no new row); `Redact(r)` replaces every match with `r`; `Finish(msg)` finishes with `msg` as
  the output and `StopReason::Completed` (a `StopReason::Guardrail` variant is a deferred idea —
  free under `#[non_exhaustive]`, but not an RT FR). Default rule set empty (PRD).
- **D-10: One grouped `AgentRuntimeConfig` under X-09.** One `AgentRuntimeConfig` in `src/config/agent_runtime.rs` carries every built-in's
  X-09 struct as a sub-struct, all off by default. `AgentRuntimeConfig { model_call_limit:
  ModelCallLimitConfig, token_budget: TokenBudgetConfig, tool_call_limit: ToolCallLimitConfig,
  guardrail: GuardrailConfig, history_trimmer: HistoryTrimmerConfig, summarization:
  SummarizationConfig, vault_recall: VaultRecallConfig, model_retry: ModelRetryConfig,
  model_fallback: ModelFallbackConfig, tool_errors: ToolErrorConfig, structured:
  StructuredOutputConfig, vault_tools: VaultToolsConfig }` — each with `enabled: false` (or a
  today's-behavior default), `Default`, `validate()` and `EnvOverridable` for its *scalar* fields
  under `APP_AGENT_RUNTIME_<SECTION>_<FIELD>` (regex rules, per-tool maps and model tables are
  config-file only), mirroring `NodeCacheConfig` (`src/config/node_cache.rs`) including the
  hand-written `Debug` convention if any field is secret-shaped (none is expected). Each built-in's
  constructor takes its sub-struct; a facade helper `AgentRuntimeConfig::build_chain(&self, deps)
  -> Result<Vec<Arc<dyn ExecutionMiddleware>>, ..>` assembles enabled sections in a documented
  fixed order (limits → guardrail → trimmer/summarizer → recall → protocol → resilience). Whether
  `paladin-server`'s agent provisioning calls `build_chain` is Claude's discretion; either way a
  v0.9 `config.yml` (no `agent_runtime:` section) boots identically, asserted by a
  `default_agent_runtime_config_is_inert` test. `PaladinConfig` (`paladin_config.rs:44`, pub
  fields, `Default`, builder) is **not** touched. — **Reversibility:** costly — env names and
  defaults are operator-facing once documented in §9.5.

### Retry & fallback middleware (RT-02; RT-FR-09)

- **D-11: Retry and fallback are port-shaping middleware.** Retry and fallback are port-shaping middleware; the service's single model-call site
  honors what they set. `ModelCallContext` gains `llm_override: Option<Arc<dyn LlmPort>>` and
  `retry_policy: Option<RetryPolicy>`. `ModelFallbackMiddleware::new(chain: Vec<Arc<dyn LlmPort>>)`
  builds one `FallbackLlmAdapter` (`crates/paladin-llm/src/fallback.rs:152`, D-24) at construction
  and sets `llm_override` in `before_model`; `ModelRetryMiddleware::new(policy: RetryPolicy)`
  (`paladin_core::platform::container::aegis::RetryPolicy`, `aegis.rs:98`) sets `retry_policy`.
  `execute_with_retry_and_temperature` (`:1643`) reads them: the port is the override or the
  service's own; attempts, delays and the predicate come from the policy when present via
  `paladin_battalion::engine::retry::backoff_delay` (`retry.rs:52`) and a new pure
  `RetryPredicate::admits(Transience) -> bool` in core that `retry::should_retry` (`retry.rs:105`)
  is refactored to call — so the backoff math and the predicate have exactly one home. With no
  retry middleware the loop keeps today's shape byte-for-byte (`max_attempts = max_loops.min(10)`,
  `100ms × 2^n`, `Permanent` short-circuit; X-03) — the two shapes are pinned by tests. Rejected:
  an `around_model` hook — it cannot express fallback (a different port, not a repeated call) and
  would change the PRD's three-hook trait. `served_by` keeps working through the fallback adapter's
  metadata stamp (D-26). — **Reversibility:** reversible — both fields are additive on a new type.
- **D-12: `ModelFallbackConfig` names providers; the factory resolves them.** `ModelFallbackConfig
  { enabled, providers: Vec<String> }` resolves each name through
  `paladin_llm::provider_factory::LlmProviderFactory::create` (`provider_factory.rs:356`) at
  `build_chain` time (typed error listing every unknown or uncompiled provider), closing Phase 25's
  "config-driven fallback chain" deferred idea inside RT-02's own config requirement; credentials
  keep coming from the factory's existing env/config path, never from this struct. Code
  composition (`ModelFallbackMiddleware::new(chain)`) remains the primary API.
  `ModelRetryConfig` mirrors `RetryPolicy`'s fields (`max_attempts`, `initial_interval_ms`,
  `backoff_factor`, `max_interval_ms`, `jitter`, `retry_on: transient_only |
  transient_and_unknown`) with `RetryPolicy::default()` values.

### Context-window management (RT-03; RT-FR-08, 10…12)

- **D-13: `TokenCounterPort` is sync and heuristic by default.** `TokenCounterPort` is synchronous and infallible; the default is heuristic; tiktoken
  rides the existing feature. `paladin_ports::output::token_counter_port::TokenCounterPort:
  Send + Sync { fn count(&self, text: &str, model: &str) -> u32; fn name(&self) -> &str }` (PRD
  sketch; unknown models fall back inside the adapter, never an error). `HeuristicTokenCounter`
  (`chars / 4`, rounded up, rustdoc says "approximate, ±30 %") lives in
  `crates/paladin-memory/src/token_counter/` beside the existing `garrison::TokenCounter` /
  `TiktokenCounter` (`token_counter.rs:12-131`), which gains `impl TokenCounterPort` under the
  existing `content-processing` feature (`tiktoken-rs` is already the crate's optional dependency —
  no new package). `PaladinExecutionService::with_token_counter(Arc<dyn TokenCounterPort>)`
  defaults to the heuristic. Provider count endpoints (Anthropic, Gemini) are a deferred idea. The
  pre-existing inline `content.len() / 4` in `rag_retrieval_service.rs:196` is left as-is (X-03)
  and listed under Deferred Ideas — RT-FR-10's "never inline heuristics" governs the budget
  features this phase adds. The legacy `garrison::TokenCounter` trait is untouched.
- **D-14: Context-limit resolution order.** `HistoryTrimmerConfig { reserve_for_response: 1024,
  default_context_tokens: 8192, model_context_limits: HashMap<String, u32>, recall_limit: 20 }`;
  the limit for a model resolves `model_context_limits[model]` → the service port's
  `get_capabilities().max_context_tokens` (`llm_port.rs:960`, already populated per adapter) →
  `default_context_tokens`; the resolved value and its source are logged at debug. `recall_limit`
  replaces the hard-coded `recall_recent(20)` only when the trimmer is installed.
- **D-15: Trimming contract.** `KeepSystemAndRecent`: system prompt, retrieved context, the current
  input and the accumulated output are always kept; history entries are admitted newest-first
  while `counted(fixed parts) + Σ counted(kept) + reserve_for_response ≤ limit`; an entry is kept
  whole or dropped whole (never split); if the fixed parts alone exceed the limit the history is
  empty and the run proceeds (logged, never failed). Stability test: identical inputs → identical
  kept set across 20 runs and across a fresh counter instance. Summary entries (D-16) are ordinary
  history entries to the trimmer.
- **D-16: Summaries compound without a Garrison delete API — latest-summary-wins.** `GarrisonPort`
  has no delete-by-id (`garrison_port.rs:380-491`: `remember`, `recall_recent`, `search`,
  `forget_all`, `stats`), so old summaries stay in the store. Rule, applied by the assembly whenever
  history is loaded: find the newest entry with `is_summary == true` in the recalled window; the
  effective history is that entry plus every raw entry newer than it. `SummarizationMiddleware`
  fires in `before_model` when the effective history exceeds `threshold_tokens` (via the port) or
  `threshold_messages` (default 30, the PRD acceptance figure); it summarizes [latest summary +
  the oldest raw entries beyond `keep_recent` (default 10)] through its own `Arc<dyn LlmPort>` and
  `summarizer_model` (defaults to the service port and the Paladin's model), `remember`s the result
  as `GarrisonEntry::summary(content)` (`ConversationRole::System`, `is_summary: true`,
  `metadata["summarized_through"] = <newest summarized entry id>`) and rewrites the assembly's
  history in place. Compound test: 30 messages → one summary; 20 more → the second summary is built
  from [summary #1 + raw tail], never from the 30 originals. Summarizer failure (`LlmError` of any
  transience) → `log::warn!`, `scratch["summarization.degraded"] = true`, and the middleware
  applies its own embedded `HistoryTrimmer` (constructed from the same config) — degradation never
  depends on chain order and never fails the run; a `TraceEvent` for it is Phase 28's.
- **D-17: `GarrisonEntry.is_summary` and embedded Garrison migrations.** `GarrisonEntry.is_summary` under X-10.3 option (a); Garrison migrations become
  embedded. `GarrisonEntry` (`garrison.rs:41-54`, six pub fields, constructors `new` /
  `with_metadata` / `with_token_count`, two in-tree struct literals — both in
  `crates/paladin-memory/benches/garrison_benchmarks.rs`) gains `#[serde(default)] pub is_summary:
  bool`, is marked `#[non_exhaustive]`, and gains `GarrisonEntry::summary(content)`; the §9.2 row is
  resolved `Y` (`struct_marked_non_exhaustive`) with allowlist entry + `crates/paladin-core/Cargo.toml`
  suppression in the same commit, and the bench literals move to constructors. The SQLite adapter
  today runs `sqlx::migrate::Migrator::new("./migrations")` at **runtime, relative to the process
  CWD** (`sqlite_garrison.rs:108`), which is why a byte-identical copy of
  `001_create_garrison_tables.sql` exists at the repo root and is `COPY`ed by `Dockerfile:28,57`.
  This phase switches the adapter to `sqlx::migrate!("migrations")` embedded at compile time — the
  `SqliteWaypointStore` precedent (`crates/paladin-storage/src/waypoint/sqlite.rs:95`) — with
  `002_add_garrison_is_summary.sql` (`ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT
  NULL DEFAULT 0`) beside the untouched `001` (same version number and checksum, so an existing
  v0.9 database migrates forward cleanly). The root `migrations/` copy and the two `Dockerfile`
  lines (plus `docs/src/deployment/docker.md:138,152`) are removed in the same change — keeping a
  mirror is Claude's discretion if the planner prefers a one-release overlap, but the embedded
  migrator must be the only reader. `INSERT`/`SELECT` (`:290-301`, `:315`) carry the column; the
  in-memory adapter passes the field through; both round-trip `is_summary` in tests. §9.4 records
  the migration (auto at construction, no down migration, no growth). — **Reversibility:** one-way
  once v0.10.0 ships (a persisted column and a public field).

### Vault: types, adapters, confinement & the host grant (RT-04; RT-FR-13…16)

- **D-18: Value types in core, port in ports, adapters in `paladin-memory`.**
  `paladin_core::platform::container::vault::{Namespace, VaultRecord, ScoredVaultRecord, Page,
  VaultError}` (ADR-0016: core owns port value types; no new core dependency, ADR-0015) and
  `paladin_ports::output::vault_port::VaultPort` per PRD §2.3. Adapters go to
  `crates/paladin-memory/src/vault/{mod,in_memory,sqlite,semantic,contract_tests}.rs`: memory is
  this crate's domain, its `sqlite` feature and `sqlx` dependency already exist and are enabled by
  the facade (`Cargo.toml:87`), and the Sanctum adapters it would compose live there too
  (`sanctum/{in_memory_adapter,qdrant_adapter}.rs`). Rejected: `paladin-storage` — Phase 25 D-27
  went there for the `redis` dependency, which the Vault does not need. `InMemoryVault` ungated;
  `SqliteVault` behind the existing `sqlite` feature; **no new cargo feature**. — **Reversibility:**
  costly — adapters in a published crate.
- **D-19: `Namespace` invariants and the three-way table.** `Namespace(Vec<String>)`: 1–16 segments,
  each 1–64 chars, no `/`, not `.` or `..`, no control characters; `Namespace::new(segments)` and
  `parse("user/alice/prefs")` validate, `Display` joins with `/`, `is_prefix_of(&other)` is the
  confinement primitive. `VaultRecord { namespace, key, value: serde_json::Value, created_at,
  updated_at }` (PRD's four fields plus the namespace, harmless); keys are 1–256 chars; values are
  bounded (`max_value_bytes`, default 64 KiB) with a typed `VaultError::ValueTooLarge`. `VaultError`
  is `thiserror`, `#[non_exhaustive]`, structured (X-06): `NamespaceDenied { requested, granted }`,
  `InvalidNamespace { reason }`, `InvalidKey { reason }`, `ValueTooLarge { bytes, max }`,
  `Unsupported { operation }`, `Storage { message }`, `Serialization { message }`. The Vault /
  Garrison / Waypoint table (scope, lifetime, addressed by, who writes, typical content) is rustdoc
  on `VaultPort` and a section of the guide.
- **D-20: Confinement is a decorator with absolute addressing.** `ConfinedVault { inner: Arc<dyn
  VaultPort>, granted: Namespace }` implements `VaultPort` and rejects every call whose namespace
  does not have `granted` as a prefix with `VaultError::NamespaceDenied` — before touching
  `inner`. The `vault_get` / `vault_put` Armaments take an **absolute** `namespace: [string]`
  argument (plus `key`, and `value` for put) so the PRD's attack test reads literally: a grant of
  `["user","alice"]`, a tool call for `["user","bob"]` → denied, the denial fed back to the model
  as a tool error (D-31), the store untouched (call-count mock). Rejected: namespaces relative to
  the grant — harmless, but the PRD's test would be unexpressible and an agent could never inspect
  its own grant.
- **D-21: The host grant travels in a `RunScope`.** The host grant travels in a `RunScope`, through one new defaulted `PaladinPort`
  method. `RunScope { vault_namespace: Option<Namespace> }` — `#[non_exhaustive]`, `Default`,
  in core beside the vault types, so Phase 27 can add `user_id` / `run_id` without a break.
  Inherent `PaladinExecutionService::execute_scoped(&self, paladin, input, heartbeat, &RunScope)`;
  `execute` / `execute_observed` are `execute_scoped` with `RunScope::default()`.
  `PaladinPort::execute_scoped(&self, paladin, input, heartbeat: &HeartbeatHandle, scope:
  &RunScope)` is added as a **defaulted** method whose body is `self.execute_observed(..)` — a
  correct default (it claims no scoped capabilities), the D-19 pattern; the engine always calls it.
  `PaladinExecutionService::with_vault(vault: Arc<dyn VaultPort>, default_namespace:
  Option<Namespace>)` installs the store; a run's grant is `scope.vault_namespace` else the
  service default else **no grant** — with no grant the vault tools are not listed for that run and
  any call is denied. `WarEngine::with_vault(vault, base: Namespace)` grants `base` to every node
  of every run on that engine (cross-thread memory is the point; a per-thread sub-namespace is the
  host's choice via `base`); `NodeContext` gains `vault: Option<ConfinedVault>` with a `vault()`
  accessor (a `PartialEq` handle the D-18 way — compares by granted namespace), and a Paladin node
  receives the same grant through `execute_scoped`. `PaladinPort`'s §9.2 row gains a second
  default-method line (`N`). — **Reversibility:** costly — a published port method and a
  `RunScope` shape Phase 27 builds on.
- **D-22: Built-in Armaments ride an in-process arsenal and a composite.** New public
  `InProcessArsenal` (`src/application/services/arsenal/in_process_arsenal.rs`, name at
  discretion): `ArsenalPort` over registered `Armament` definitions + async closures — the piece
  the tree lacks (every shipped `ArsenalPort` routes to an MCP client:
  `arsenal_execution_service.rs:184-230`) and the piece RT-07's doc test needs. `VaultTools::new(
  confined) -> InProcessArsenal` builds `vault_get` (`{ namespace, key }` → the record's value or
  a documented "not found" result) and `vault_put` (`{ namespace, key, value }` → ok) with JSON
  Schemas in `Armament.parameters`. `CompositeArsenalPort::new(Vec<Arc<dyn ArsenalPort>>)` unions
  `list_armaments` (first registration of a name wins, duplicates logged) and routes `invoke` /
  `validate_call` by name. `PaladinExecutionService::enable_vault_tools()` (or
  `VaultToolsConfig.enabled`) wraps the configured arsenal (or none) in a composite with the run's
  `VaultTools` — opt-in, per PRD. HTTP-served agents have no Arsenal (ADR-0039), so vault tools are
  unreachable over `/v1/agents/*` by construction — stated in the guide, not worked around.
- **D-23: SQLite Vault schema and one migrator per crate.** `crates/paladin-memory/migrations/
  003_create_vault_tables.sql`: `vault_records (ns TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT
  NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY (ns, key))` plus an index
  on `(ns, key)`; `ns` is the `/`-joined path (safe because segments forbid `/`); `list(ns,
  prefix, page)` returns records **in exactly `ns`** (not descendants) whose key starts with
  `prefix`, ordered by key, `Page { limit ≤ 1000 (default 50), after: Option<String> }` (the last
  returned key — opaque to callers). Both `SqliteGarrison` and `SqliteVault` run the same embedded
  paladin-memory migrator (D-17), so pointing both at one file is safe and pointing them at
  separate files creates a few empty tables in each — documented, harmless (`IF NOT EXISTS`).
  `SqliteVault::new(path)` constructs twice idempotently (the Waypoint precedent test).
- **D-24: The semantic adapter composes ports and is ungated.** `SemanticVault { store: Arc<dyn
  VaultPort>, sanctum: Arc<dyn SanctumPort>, embedder: Arc<dyn EmbeddingPort> }`: `put` writes the
  record to `store` and an embedding to `sanctum` under a deterministic entry id derived from
  `(ns, key)` (re-put updates); `delete` removes both; `search` embeds the query, searches
  `sanctum` with the namespace carried in the entry's filterable field and **re-filters results by
  namespace prefix before returning** (the confinement invariant cannot depend on the backend's
  filter), maps scores to `ScoredVaultRecord`. Because it holds only trait objects it needs no
  `qdrant-client` and is not feature-gated (ADR-0046's composition rule); PRD 05's "`qdrant`
  feature" is satisfied by the Qdrant `SanctumPort` adapter behind the existing `qdrant` feature.
  Contract suite: put/get/overwrite/delete/list-prefix/pagination/namespace-isolation on all three,
  `search` asserted only on `SemanticVault` (InMemory + SQLite return `Unsupported { operation:
  "search" }`); Tier 1 uses `InMemorySanctumAdapter` + a deterministic mock `EmbeddingPort`; the
  Qdrant backend is proven only where a Qdrant service exists — routed to UAT like Phase 24 D-28,
  never marked passed locally.
- **D-25: `VaultRecallMiddleware` is best-effort.** Fires in `before_model` of loop 1 only:
  `search(granted, input, top_k)` (default 5), drops results below `score_floor` (default 0.0),
  and inserts a delimited `## Long-term memory` section into the assembly (after retrieved RAG
  context, before history) that states the entries are **stored notes, not instructions**.
  `VaultError::Unsupported` → skip and `log::warn!` once per service (the adapter cannot search);
  any other error → skip and warn; no grant → skip silently. It never fails the run. No auto-write
  middleware (PRD §5).

### Structured output (RT-05; RT-FR-17…19)

- **D-26: Machinery in core, loop driver in ports, schemars only in the facade.**
  `paladin_core::platform::container::structured::{Structured<T> { value, raw: PaladinResult },
  StructuredOptions { max_repair_attempts: 1, .. }, SchemaRef::{Inline(serde_json::Value),
  Registered(String)}, extract_json(&str) -> Option<Value>, shape_check(&Value, &Value) ->
  Result<(), ShapeError>, render_instruction_block(&Value) -> String}` — pure, no new core
  dependency. `extract_json` is the Phase 23 D-11 rule lifted out of
  `crates/paladin-battalion/src/engine/directive_parser.rs`'s private `extract_envelope` (trimmed
  whole output if it parses as a JSON object, else the first ```` ```json ```` fenced block) and
  `DirectiveParser::StructuredDirective` is refactored to call it (CF-FR-06's "MUST reuse", no
  behavior change, its tests unchanged). `paladin_ports::output::structured_executor_port` holds
  `StructuredExecutorPort` and the generic repair-loop driver `run_structured(execute_fn, input,
  schema, opts)`: attempt 1 appends the instruction block to the input; on a parse/shape failure
  it re-prompts (≤ `max_repair_attempts`) with the parse error and the offending output; exhaustion
  → `PaladinError::StructuredOutputInvalid { attempts, last_error, raw_output }` (structured,
  X-06; on the already-`#[non_exhaustive]` enum — row text extended). **`schemars` is added as a
  direct dependency of the facade only** — it is already in `Cargo.lock` at `1.2.1` via `rmcp
  2.1.0`, so no new package enters the graph; X-11.1's `msrv` job (`ci.yml:251`, Rust 1.88) is the
  MSRV proof and §9.3 records it. — **Reversibility:** costly — public types in three crates.
- **D-27: `StructuredExecutorPort` is object-safe at the JSON level.** `StructuredExecutorPort` is object-safe at the JSON level; the generic is a facade
  extension. `#[async_trait] trait StructuredExecutorPort: Send + Sync { async fn
  execute_json_schema(&self, paladin, input, schema: &serde_json::Value, opts:
  &StructuredOptions) -> Result<Structured<serde_json::Value>, PaladinError>; async fn
  execute_json_schema_observed(.., heartbeat) }` (the observed variant defaulted to the plain one,
  D-19 pattern). `paladin::application::services::paladin::structured::StructuredExecutorExt`
  (blanket over `T: StructuredExecutorPort + ?Sized`) provides `execute_structured<T:
  DeserializeOwned + JsonSchema>(&self, paladin, input) -> Result<Structured<T>, PaladinError>`
  (schema via `schemars::schema_for!`, value via `serde_json::from_value` — serde deserialization
  **is** the typed validation). `PaladinExecutionService` implements the port natively: it sets
  `LlmRequest.response_format` (D-28) for every model call of the structured run **and** appends
  the instruction block (belt and braces — correctness never depends on the native mode), then
  runs the shared driver. `PaladinPort` gains nothing for structured output (PRD: "NOT on
  `PaladinPort`"). Tests: derive-based happy path, repair success on attempt 2 (scripted mock),
  typed exhaustion with `raw_output` preserved, default-on-a-plain-port equivalence.
- **D-28: `LlmRequest.response_format` under X-10.3 option (a), with a builder.** `LlmRequest`
  (`llm_port.rs:626-641`) is a pre-existing constructible struct with six pub fields, no `Default`
  and no constructor, built by full struct literal in **37 files** (`crates/paladin-llm` 12,
  `paladin-battalion` 3, `paladin-ports` 2, the root `tests/` 20). It gains `pub response_format:
  Option<ResponseFormat>` with `#[serde(default)]`, is marked `#[non_exhaustive]`, and gains
  `LlmRequest::new(model: impl Into<String>, prompt: PromptItem)` (fresh `id`, no attachments,
  `stream: false`, empty metadata, `response_format: None`) plus chainable `with_attachments`,
  `with_stream`, `with_metadata`, `with_response_format`, all doc-tested. ⚠ **Option (b) is
  strictly worse here, unlike Phase 25 D-26**: `LlmRequest` has no `Default`, so there is no
  functional-update site to preserve — every downstream literal breaks under (a) and (b) alike,
  and only (a) makes the *next* field free. Every in-tree literal migrates to the constructor in
  the same commit; §9.2 row `Y` (`struct_marked_non_exhaustive`), allowlist entry and
  `crates/paladin-ports/Cargo.toml` suppression together. `ResponseFormat` is `#[non_exhaustive]`:
  `JsonObject`, `JsonSchema { name: String, schema: serde_json::Value, strict: bool }`. Native
  wiring this phase: the OpenAI adapter and the compat engine's `build_request` (one change covers
  Kimi/Qwen/Grok/Ollama/OpenAI-compatible — `response_format` in the chat-completions body),
  Gemini (`generationConfig.responseMimeType: "application/json"` + `responseSchema`), DeepSeek
  (`response_format: { type: json_object }`); Anthropic ignores the field (no native mode —
  prompt-only, documented in the guide's per-provider table); each wired path gets one mockito
  test asserting the wire field, and the mock records it. **No `ProviderCapabilities` field is
  added** (another X-10.3 event for no FR) — a structured-output capability flag is a deferred
  idea. — **Reversibility:** one-way once v0.10.0 ships (a public struct attribute and constructor
  contract).
- **D-29: Engine integration follows the FT-06 fail-closed wiring.** `NodeSpec::Paladin` (new in
  0.10, `graph.rs:41-58`) gains `output_schema: Option<SchemaRef>` (constructor-preserved via
  `NodeSpec::paladin(..)` + `with_output_schema`, deliberate-zero note). `WarEngine::
  with_structured_executor(Arc<dyn StructuredExecutorPort>)` and `with_output_schema(name,
  Arc<dyn StructuredSchema>)` (registry, CF-01 pattern; `TypedSchema::<T>::new()` implements it by
  `serde_json::from_value::<T>` so a registered schema gets full typed validation) sit beside
  `with_node_cache`; a node with `output_schema` on an engine with no structured executor, or a
  `Registered` name that is not registered, is a typed `EngineError` at validation before any node
  runs. When set, the node executes through `execute_json_schema_observed`, the **parsed JSON
  value** is written to `output_field` (the schema's declared field type must be `Json`/compatible
  — validated), and `directive_parser` must be `PlainOutput` (`OutputSchemaWithStructuredDirective`
  typed validation error — combining the two is a deferred idea). Exhaustion becomes a
  `NodeError` with `source: Paladin { kind: "StructuredOutputInvalid", .. }` and transience
  **`Unknown`** (the repair loop was the retry; a `TransientAndUnknown` Aegis may still retry the
  node — documented). `output_schema` (canonical JSON of an inline schema, or the registered name)
  enters `WarGraph::fingerprint()` sorted and length-prefixed (D-11 discipline; `v5` → `v6`, golden
  re-pinned, one difference test). ⚠ **PRD 05's "default-implemented in terms of `execute` on the
  execution service" is honored by the driver, not by a `PaladinPort` default** — a defaulted
  `PaladinPort` method was rejected because X-10.4's own text names `StructuredExecutorPort` as the
  reason the program does not extend `PaladinPort` for this. — **Reversibility:** one-way after
  v0.10.0 (fingerprints are stored on Waypoints).
- **D-30: Untyped validation is a documented shape check, not a JSON Schema engine.** For
  `SchemaRef::Inline` and the JSON-level port, `shape_check` enforces `type`, `required`,
  `properties` (recursively), `additionalProperties: false`, `enum`, `items` and `anyOf`-with-null
  nullability, and nothing else — stated in rustdoc. A full validator (`jsonschema`) would be a new
  heavyweight dependency in the default set (X-11.4) for no acceptance criterion; it is a deferred
  idea behind a future feature.

### Provider conformance close-out (RT-06; RT-FR-20…22)

- **D-31: One shared conformance suite, instantiated per adapter, run as measurement first.**
  `crates/paladin-llm/src/conformance.rs` (`#[cfg(test)]`): a `ConformanceFixture` trait
  (`adapter(base_url)`, `success_body()`, `stream_body()`, `error_body(status)`, `wire:
  OpenAiChat | Gemini`) and an `llm_conformance_suite!(Fixture)` macro producing the fixed case
  list — generate success + usage extraction, streaming assembly in wire order with a terminal
  stop, streaming error before/after the first chunk, `401`/`404`/`400`/`402` dedicated mappings,
  `408`/`429`/`5xx` → `Transience::Transient` **by value** and other `4xx` → `Permanent`
  (`LlmError::transience()`, never string parsing — Phase 25 D-03's `map_http_status`), credential
  never present in any rendered error (redact-then-bound), redirects not followed with a
  credential header (security.instructions.md). The suite is instantiated for the three v0.8.0
  paths (`openai_compatible`, `gemini`, `ollama`) — and, at Claude's discretion, for the six others
  where the fixture is free (the compat engine makes Kimi/Qwen/Grok one fixture each). ⚠ **PRD
  05 §2.5's premise that these adapters are new is stale** (REQUIREMENTS.md scope-time conflict
  record); the plan's first task *measures* — runs the suite, records a per-adapter case table
  (pass / gap) in the plan SUMMARY and `08-traceability-matrix.md` G-22 — and only then closes
  gaps. Expected gaps from the scout: no explicit `429`/`5xx`-by-value case on the Ollama path
  (`ollama/adapter.rs` has one mockito test and no `429` literal), no mid-stream-error case on
  `openai_compatible` (one stream test at `:955`), Gemini's `5xx` transience asserted only via
  `map_error` units (`gemini/adapter.rs:1624-1636`). Nothing is rebuilt.
- **D-32: The Ollama recipe documents the suite that already exists.** `tests/integration/
  ollama_docker_test.rs` is `required-features`-gated (`integration-tests`, `llm-ollama`), probes
  `OLLAMA_TEST_URL` and skips with a printed reason, and runs in CI's `ollama-integration` job
  (`ci.yml:748`) — that *is* the "ignored-by-default integration test gated on an env var" in
  effect. ⚠ **No second Ollama test file is written.** The recipe lands in the guide (D-38):
  `ollama serve`, `ollama pull <model>`, the `config.yml` block already at
  `docs/src/getting-started/configuration.md:84-91`, `OLLAMA_BASE_URL`, a `reasoning_agent` snippet
  against Ollama, and the exact `cargo test --test ollama_docker --features integration-tests,
  llm-ollama` line; if measurement (D-31) finds an Ollama gap, it is fixed in the adapter and
  covered by the shared suite, not by a new integration file.

### Presets & tool-error ergonomics (RT-07; RT-FR-23, 24)

- **D-33: The default tool-error mode is `FeedToModel`.** M-B-03's premise is inverted by the tree — the default is `FeedToModel` and that is no
  behavioral change. Today's loop already injects a failed Armament call back into the model's
  context and continues (`paladin_execution_service.rs:955-967`: `"🔧 Tool Execution: {name}\nResult:
  FAILED\nError: {e}"` appended to the accumulated output; a handoff failure at `:912-920` does the
  same). ⚠ **PRD RT-FR-24 and `MIGRATION.md` M-B-03 (`:17`, `:117`) assume v0.9 aborted the run on
  a tool failure; it did not.** Decision: `tool_error_mode` defaults to `FeedToModel` — identical
  to v0.9 — and `FailRun` (`PaladinError::ArmamentFailed { tool, source }`-style structured error,
  X-06) is the **new** opt-in; §9.1 M-B-03 is rewritten to say exactly this ("no behavioral change:
  the v0.9 loop already fed tool failures back; v0.10 names the policy, adds `FailRun`, and
  sanitizes the text the model sees"), with the sanitization named as the only observable
  difference and a before/after example of the fed-back text. Rationale recorded there per the
  PRD's own "the choice and its rationale are documented, not which side is chosen".
- **D-34: Service-level `ToolErrorConfig`, one formatter, shared redaction.** `tool_error_mode` is service-level config; the fed-back text is one formatter
  method; sanitization reuses the redaction module. `ToolErrorConfig { mode: FeedToModel |
  FailRun, per_tool: HashMap<String, ToolErrorMode> }` under `AgentRuntimeConfig` (D-10), set via
  `PaladinExecutionService::with_tool_error_config` — `PaladinConfig` (pre-existing, pub fields) is
  not touched. `ToolResultFormatter` (`src/infrastructure/adapters/arsenal/tool_result_formatter.rs:66`)
  gains `format_error(&ArmamentCall, &str) -> String` used by both the Arsenal and handoff arms,
  keeping today's `🔧 Tool Execution … FAILED` shape and appending the PRD sentence ("You may retry
  with corrected arguments or proceed without it."). The reason passes through a new key-less
  `paladin_llm::redaction::redact_secret_patterns(&str)` (the pattern half of
  `redact_credentials`, `redaction.rs:107`, factored out — documented regex set: bearer tokens,
  `sk-`/`sk-ant-`/`AKIA`-style keys, `key=`/`token=` query values, JWT-shaped triples) **before**
  `bounded_excerpt` (`redaction.rs:50`) — redact-then-bound, the security.instructions.md rule.
- **D-35: `reasoning_agent` takes an executable arsenal and returns a runnable pair.**
  `paladin::presets::reasoning_agent(llm: Arc<dyn LlmPort>, arsenal: Arc<dyn ArsenalPort>, opts:
  ReasoningAgentOptions) -> Result<ReasoningAgent, PaladinError>` where `ReasoningAgent { paladin:
  Paladin, service: PaladinExecutionService }` exposes `run(&self, input) ->
  Result<PaladinResult, PaladinError>` and `run_structured<T>(..)` (D-27), and `Deref`s nothing.
  `ReasoningAgentOptions` (`Default`, builder): `system_prompt` (default a documented tool-use
  prompt), `model`, `max_loops` (default 5), `max_tool_calls` (default 20), `garrison:
  Option<Arc<dyn GarrisonPort>>`, `tool_errors: ToolErrorConfig` (default `FeedToModel`),
  `circuit_breaker: Option<Arc<CircuitBreaker>>` (default `CircuitBreaker::new(3, 2, 30 s)`, the
  README figure). ⚠ **Deviates from PRD RT-FR-23's `tools: Vec<Armament>`**: an `Armament` is a
  definition (`arsenal/core.rs:17`) and cannot execute, so a preset taking definitions could never
  run a tool; callers build an `InProcessArsenal` (D-22), an MCP-backed `ArsenalExecutionService`,
  or a `CompositeArsenalPort`. The ≤15-line example (`MockLlmAdapter::with_responses([a tool-call
  envelope, a final answer])` + `InProcessArsenal` with one closure → `run` → assert) lives in
  `crates/doc-examples/src/agent_runtime.rs` under an `// ANCHOR:` region included by the guide
  (the `fault_tolerance.rs` pattern — compile-verified by `cargo check -p paladin-doc-examples` in
  CI, PRD §3.7) **and** as a rustdoc example on `reasoning_agent` itself. — **Reversibility:**
  costly — a public preset signature.
- **D-36: A prompt-level tool-call protocol enables the loop.** The tool loop is made real for shipped providers by a prompt-level protocol, not a wire
  change. No shipped adapter ever populates `LlmResponse.function_call` (ADR-0042; `mock.rs`
  included), and nothing in the tree renders a tool catalogue into the prompt (the builder only
  auto-registers the handoff tool, `paladin_builder.rs:1365-1378`), so today the reasoning loop's
  tool branch is reachable only through a consumer-supplied `LlmPort`. `ToolCallProtocolMiddleware`
  (built-in, opt-in; the preset installs it): `before_model` renders the arsenal's
  `list_armaments()` (name, description, parameter schema) plus a documented call format into a
  `## Tools` assembly section; `after_model`, when `function_call` is `None`, runs `extract_json`
  (D-26) over `response.content` and, if it yields the documented envelope `{"tool": "<name>",
  "arguments": {..}}`, synthesizes `response.function_call` so the existing branch (`:889`) fires
  unchanged. `FinishOnPlainAnswerMiddleware` (built-in, opt-in; the preset installs it):
  `after_model` returns `Finish(content, StopReason::Completed)` when the response carries no tool
  call — today's loop always runs to `max_loops` and returns `StopReason::MaxLoops` (`:987-1022`),
  which is the wrong default for a tool-loop agent but stays untouched for existing users (X-03).
  ⚠ **This is the phase's one scope interpretation**: RT-FR-23's "tool loop enabled" is read as
  "a shipped provider can call a tool", which the tree cannot do otherwise; ADR-0042 is untouched
  (no `LlmRequest.tools`, no adapter change, the mock's capability flags stay `false`, the
  correspondence test in `crates/paladin-llm/src/lib.rs` still holds), and native tool calling
  remains that ADR's deferred capability with its trigger unchanged. The developer may overturn this
  at plan review, in which case the preset ships with the loop inert for shipped providers and the
  doc test drives it through `MockLlmAdapter` only — which D-35's example already does.

### Program bookkeeping, tests & docs

- **D-37: `MIGRATION.md` and the semver gate.** §9.2: resolve the three RT-owned `TBD` rows
  (`StopReason` D-07, `LlmRequest` D-28, `GarrisonEntry` D-17 — all `Y`, each with its allowlist
  entry and per-crate `[package.metadata.cargo-semver-checks.lints]` suppression in the same
  commit; the `Crate` cell uses the published package name, `paladin-ai-core`, per the Plan 25-14
  correction); extend the `PaladinError` row's Change cell (`StructuredOutputInvalid`,
  `GuardrailTripped`, the `FailRun` variant) and the `PaladinPort` row (`execute_scoped`, `N`);
  list the new traits `ExecutionMiddleware`, `TokenCounterPort`, `VaultPort`,
  `StructuredExecutorPort` for completeness (`N/A`, new); one deliberate-zero note (the Phase
  23/24/25 form) for every new-in-0.10 type touched — `NodeSpec::Paladin.output_schema`,
  `NodeContext.vault`, `EngineError` variants, `WarGraph` builders + fingerprint `v6`,
  `DirectiveParser` internals, `MockLlmAdapter`'s recorded `response_format`. §9.3: `schemars`
  as a facade dependency (no new lockfile package; MSRV 1.88 proven by the `msrv` job), no new
  cargo feature (`tiktoken` counter under `content-processing`, `SqliteVault` under `sqlite`,
  `SemanticVault` ungated). §9.4: `002_add_garrison_is_summary.sql` and `003_create_vault_tables.sql`
  (auto at construction, no down migration, embedded migrator replacing the runtime path, root
  `migrations/` copy removed). §9.5: `AgentRuntimeConfig` and every `APP_AGENT_RUNTIME_*` variable,
  all inert by default. §9.1: M-B-03 per D-33. Gates green on the phase's final commit:
  `cargo semver-checks` (vs 0.9.0), `msrv` (1.88), `make security`, `cargo clippy -- -D warnings`,
  coverage ≥ 82 % (ADR-0006), **and `scripts/check-api-surface.sh .project/current-exports.txt`
  with the export file regenerated** — the Phase 25 carried concern that left the `api-surface` job
  red for two phases (STATE.md, 2026-09-06). `CHANGELOG.md` `[Unreleased]`;
  `08-traceability-matrix.md` G-13 (RT-FR-09), G-16…G-19, G-21, G-22 gain test anchors.
- **D-38: Docs (X-08).** One new mdBook page `docs/src/user-guides/agent-runtime.md` ("Agent
  Runtime: Middleware, Context Management, Vault Memory, Structured Output and the Reasoning
  Agent") registered after the fault-tolerance page in `docs/src/SUMMARY.md:25`, in the
  `parley-and-chronicle.md` / `fault-tolerance.md` shape with `{{#include}}` anchors from
  `crates/doc-examples/src/agent_runtime.rs`: the two-layer table (D-05), the Vault/Garrison/
  Waypoint table (D-19), the per-provider native-JSON-mode table (D-28), the Ollama recipe (D-32),
  the `reasoning_agent` example (D-35) and the tool-call protocol (D-36). `docs/src/user-guides/
  tool-integration.md` gains a pointer to the protocol and to `InProcessArsenal`;
  `garrison-memory.md` a paragraph on summaries; rustdoc + doc tests on every new public item;
  `cargo doc` with no new broken intra-doc links.
- **D-39: Test tiers and the X-05 obligations.** Everything RT adds is Tier 1 (`MockLlmAdapter`,
  `InMemoryVault`, `InMemoryGarrison`, `SqliteVault`/`SqliteGarrison` on temp files,
  `InProcessArsenal`, mockito) except the Qdrant `SemanticVault` backend (UAT, D-24) and the Ollama
  suite (CI `ollama-integration`, D-32). X-05 stress tests with exact counts and a timeout guard
  (`listener.rs` pattern): concurrent runs through one service with counting middleware (RT-FR-02),
  concurrent confined `vault_put`s from N runs under distinct grants asserting zero cross-namespace
  records, and concurrent structured runs sharing one schema registry. The onion-ordering test
  (three recording middleware, `Finish` from #2) and the 30-message compound-summarization test are
  PRD §3 items 1 and 3 verbatim.
- **D-40: Plan shape follows PRD 05 §4.** Suggested waves: (1) trait, chain, contexts,
  `PromptAssembly`, empty-chain equivalence, ordering, isolation stress; (2) `StopReason`, limits,
  guardrail, `AgentRuntimeConfig`; (3) retry/fallback port-shaping, `RetryPredicate::admits`
  refactor, `FinishOnPlainAnswer`, `ToolCallProtocol`; (4) `TokenCounterPort` + adapters, trimmer,
  `GarrisonEntry.is_summary` + embedded migrator + `002`, summarization; (5) Vault core types,
  port, three adapters, contract suite, `003`; (6) `ConfinedVault`, `InProcessArsenal`,
  `CompositeArsenalPort`, vault tools, `RunScope` / `execute_scoped`, `WarEngine::with_vault`,
  `NodeContext::vault()`, recall middleware, attack + stress tests; (7) structured core/ports
  machinery, `extract_json` lift, `LlmRequest` builder + 37-file migration, `response_format` in
  four adapter paths, `StructuredExecutorPort` impl, engine `output_schema` + registry +
  fingerprint `v6`; (8) conformance suite, measurement table, gap closure, Ollama recipe; (9)
  `reasoning_agent`, tool-error config + sanitization, M-B-03, doc-examples anchors; (10) guide,
  MIGRATION sweep, traceability, gate evidence including `check-api-surface.sh`. (1) precedes
  everything; (2)/(3) follow (1); (4), (5), (7), (8) are mutually independent after (1) — (7)'s
  `LlmRequest` migration is standalone and may go first; (6) needs (5); (9) needs (3), (6), (7).
- **D-41: Security posture for the planner's threat model.** Namespace traversal is closed by
  construction (`ConfinedVault` prefix check + `Namespace` segment validation; attack tests for
  sibling, parent, `..`, empty and over-long segments); recalled Vault content and fed-back tool
  errors are model-controlled text inserted into the prompt — both sections are framed as data,
  not instructions, and the guide inherits M-B-04's raw-content warning for anything an agent
  `vault_put`s; every tool-error string and every provider excerpt is redacted before it is bounded
  (D-34, D-03 of Phase 25); `Guardrail` patterns compile under the linear-time `regex` crate with a
  size bound; `AgentRuntimeConfig` carries no secret and `ModelFallbackConfig` names providers
  only — credentials stay on the factory's env path; `response_format` schemas are caller-authored
  and never logged with request bodies; `LlmRequest::new` and the mock's recorder never `Debug`-
  print credentials (none flow through them); Vault values are size-bounded and JSON-only (no
  deserialization into executable types). R-23-01 (hanging `EdgeConditionEvaluator`) stays
  accepted and is re-listed, and a hanging **middleware** is bounded only by the node's Aegis
  `run_timeout`/the service's per-run timeout — stated, not solved.

### Claude's Discretion

- Exact field sets of `ModelCallContext` / `ToolCallContext` / `LlmResponseView` / `FinalResult` /
  `PromptAssembly` / `PromptSection`; the typed scratch API; the middleware module file split; the
  chain's fixed `build_chain` order beyond the documented outline.
- Names: `ConfinedVault`, `InProcessArsenal`, `CompositeArsenalPort`, `VaultTools`,
  `ToolCallProtocolMiddleware`, `FinishOnPlainAnswerMiddleware`, `StructuredExecutorExt`,
  `TypedSchema`, `StructuredSchema`, `RunScope`, `redact_secret_patterns`; `EngineError` variant
  names for D-29; `VaultError` variant names beyond `NamespaceDenied` / `Unsupported`.
- Whether `paladin-server` calls `AgentRuntimeConfig::build_chain` for HTTP agents (D-10), and
  whether the conformance suite is instantiated for all nine adapters or only the three v0.8.0
  paths (D-31).
- Whether the root `migrations/` copy is deleted outright or mirrored for one release (D-17 —
  the embedded migrator is locked either way); the `Page` cursor encoding (D-23); the Sanctum
  filter field the semantic adapter uses to carry the namespace (D-24); the deterministic entry-id
  derivation.
- The instruction-block wording and the repair-prompt wording (D-26); the tool-catalogue and
  envelope wording (D-36); the truncation-notice text (D-08); the summarizer prompt (D-16).
- Whether `BATTLEFIELD_SCHEMA_VERSION` bumps for `NodeContext.vault` (it is not persisted — expected
  no) and how `NodeContext: PartialEq` treats the vault handle.
- Plan count and wave assignment within D-40; which plan owns the 37-file `LlmRequest` migration
  (D-28) and the `_` arms for `StopReason` (D-07).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)
- `.project/v0.10.0/05-agent-runtime-enhancements.md` — **The FR-level source of truth for this
  phase.** §2.1 middleware trait sketch and built-ins (RT-FR-01…09), §2.2 context management
  (RT-FR-10…12), §2.3 `VaultPort` sketch, adapters, agent access and recall (RT-FR-13…16), §2.4
  structured output (RT-FR-17…19), §2.5 provider breadth (RT-FR-20…22 — read with the scope-time
  conflict record below), §2.6 presets and tool-error feedback (RT-FR-23, 24), §3 acceptance
  criteria 1–9, §4 TDD ordering, §5 out of scope. Every plan task traces to an FR here.
- `.project/v0.10.0/00-program-overview.md` — §3 X-01…X-11 (X-03 backward compatibility and the
  stop-and-flag rule; X-05 stress pattern; X-06 structured errors; X-07 features; X-09 config —
  D-10; X-10.2/X-10.3/X-10.4 register rules — D-07, D-17, D-21, D-28; X-11 — the MSRV is **1.88**
  since Phase 22.1, superseding §3's "1.85"), §4 ubiquitous language (Vault), §9.1 M-B-03 — D-33,
  §9.2 the three RT-owned rows, §9.3–9.5 — D-37.
- `.project/v0.10.0/04-fault-tolerance.md` — FT-FR-01 (transience by value — the bar RT-06
  verifies), FT-FR-03…05 (`RetryPolicy` — D-11 delegates to it), FT-FR-16/17 (`FallbackLlmAdapter`,
  `served_by` — D-11/D-12).
- `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` — CF-FR-06 (`StructuredDirective`
  MUST reuse the structured machinery — D-26), CF-FR-01/02 (the fail-closed registry pattern —
  D-29).
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — ENG-FR-14 (fingerprint
  contents — D-29's `v6`), ENG-FR-22 (`NodeInterceptor` — the other layer in D-05).
- `.project/v0.10.0/08-traceability-matrix.md` — G-13, G-16, G-17, G-18, G-19, G-21, G-22 rows
  (test anchors owed by D-37).
- `.planning/REQUIREMENTS.md` — RT-01…RT-07 capability clusters with FR ranges; the **scope-time
  conflict record** (PRD 05 §1/§2.5 stale — RT-06 is verify-then-fix); the X-10/X-11 versioning
  gate as part of every requirement's definition of done; FUT-06/08/09 and the out-of-scope table.
- `.planning/ROADMAP.md` — Phase 26 goal, dependency (22; parallelizable with 24/25), the five
  success criteria; Phase 27 consumes `RunScope` (D-21) and Phase 28 the trace seams.

### Program deliverable this phase appends to
- `MIGRATION.md` — §9.1 M-B-03 row (line 17) and its `TBD` bullet (line 117) — D-33; §9.2 rows at
  lines 130 (`StopReason`), 134 (`LlmRequest`), 135 (`GarrisonEntry`) — "TBD — owner RT-0x, Phase
  26"; the `PaladinError` and `PaladinPort` rows to extend; the Phase 23/24/25 deliberate-zero
  notes (form D-37 follows); §9.3 (line 153), §9.4 (163), §9.5 (174).
- `.cargo/semver-checks-allowlist.toml` — entry schema and the set-equality rule with §9.2's `Y`
  rows (D-07, D-17, D-28).
- `.project/current-exports.txt` + `scripts/check-api-surface.sh` — the `api-surface` CI job
  (`.github/workflows/ci.yml:193-216`) this phase must leave green (D-37).

### Prior-phase decisions that constrain this phase
- `.planning/phases/25-node-level-fault-tolerance/25-CONTEXT.md` — D-03 (`map_http_status`, the
  nine-adapter mapper RT-06 re-verifies — D-31), D-04 (`StopReason` left to this phase — D-07),
  D-05 (transience table — D-29's `Unknown`), D-19 (defaulted `PaladinPort` methods — D-21, D-27),
  D-24/D-25 (`FallbackLlmAdapter` as a plain port, first-chunk rule — D-11), D-26 (X-10.3 option-b
  reasoning D-28 distinguishes itself from), D-29 (config-struct shape — D-10), D-34 (redact-then-
  bound — D-34/D-41), and its Deferred Ideas ("config-driven fallback chain" — D-12 closes it;
  "retry/fallback as `ExecutionMiddleware`" — D-11).
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` — D-11 (the
  envelope and extraction rule D-26 lifts), D-18 (fingerprint discipline — D-29), D-26 (code-
  configured, off by default — D-10), and its Deferred Ideas ("native provider JSON mode for
  `StructuredDirective`" — D-26/D-28; "`NodeInterceptor` visibility of `NextStep`" — stays
  deferred, D-05).
- `.planning/phases/24-pause-resume-history-graceful-shutdown/24-CONTEXT.md` — D-26 (X-09
  `WaypointStoreConfig` shape), D-28 (UAT routing for Docker-only tiers — D-24, D-39), D-29
  (deliberate-zero note form).
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-CONTEXT.md` — D-06…D-11
  (MSRV 1.88 — D-26), D-15…D-19 (fingerprint encoding — D-29).
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — D-09/D-10 (contract-
  suite style, Tier 2 provable only in CI — D-24), D-12 (snapshot isolation).
- `.planning/phases/22-battlefield-state-superstep-engine/22-09-SUMMARY.md` — the empty-chain
  equivalence test D-02 copies for the middleware chain.
- `.planning/STATE.md` — the Phase 25 close-out carried concern: add `scripts/check-api-surface.sh`
  to the gate list from Phase 26 on (D-37).

### Standing decisions and governance
- `.planning/decisions/0042-llm-native-tool-calling-deferred.md` (ADR-0042) — native tool calling
  stays deferred with its trigger; the mock stays unchanged; D-36 adds a prompt-level protocol
  only and must not touch `LlmRequest`'s tool surface or any adapter's capability flags.
- `.planning/decisions/0039-http-topology-no-garrison-no-arsenal.md` (ADR-0039) — HTTP-served
  agents have no Arsenal, so vault tools are unreachable over HTTP by construction (D-22).
- `.planning/decisions/0046-facade-llm-feature-flag-wiring.md` (ADR-0046) — composition types
  (`FallbackLlmAdapter`, `SemanticVault`) are ungated; provider adapters stay behind their flags
  (D-12, D-24, D-28).
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — the vault and
  structured core modules add no core dependency; `schemars` lives in the facade only (D-26).
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns `Namespace`,
  `VaultRecord`, `VaultError`, `RunScope`, `Structured<T>`, `SchemaRef`; ports re-export.
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — 82 % workspace floor.
- `.github/instructions/security.instructions.md` — redaction before truncation for every provider
  body and every tool-error string that reaches a model, an error or a log (D-34, D-41); no
  credential in any config `Debug` (D-10).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/application/services/paladin/paladin_execution_service.rs` — the service (struct `:109`,
  `new` `:176`, chainable builders `:218-403` — D-06 adds beside them); `execute` `:474`,
  `execute_observed` `:527`, `execute_bounded` `:547`, `execute_internal` `:731`; the reasoning
  loop `:840-1022` (assembly `:853-859`, model call `:862-871`, heartbeat `:873`, handoff branch
  `:891-921`, Arsenal branch `:922-968` with the tool-error arm `:955-967` — D-04, D-33; stop
  words `:978`; `MaxLoops` return `:987-1022` — D-36); `build_prompt_with_custom_system`
  `:1112-1153` (the flat prompt D-02 replaces with an assembly rendered by the same code);
  `execute_with_retry_and_temperature` `:1643` (`max_attempts = max_loops.min(10)`, `100ms × 2^n`,
  `Permanent` short-circuit — D-11's single model-call site); `handle_tool_call` `:1925-1962`;
  `PaladinExecutorPort` impl `:1971`; `execute_stream_inner` `:2045` (one call, no loop — D-04).
- `crates/paladin-core/src/platform/container/execution_result.rs` — `StopReason` `:102-111`,
  `is_successful` `:118`, `is_limit` `:123` (D-07/D-08); `PaladinResult` with `served_by`.
- `crates/paladin-ports/src/output/llm_port.rs` — `LlmRequest` `:626-641` (D-28), `LlmResponse`
  `:705` (`function_call` `:730` — D-36 synthesizes it), `FunctionCall` `:743`, `FinishReason`
  `:795`, `ProviderCapabilities` `:942` (`max_context_tokens` `:960` — D-14; **not** extended —
  D-28), `LlmError` `:300` (`#[non_exhaustive]`, `transience()` — D-31).
- `crates/paladin-core/src/platform/container/garrison.rs` — `GarrisonEntry` `:41-54`,
  constructors `:72-108`, `ConversationRole` `:24` (D-16, D-17);
  `crates/paladin-ports/src/output/garrison_port.rs` — `GarrisonPort` `:380-491` (no delete-by-id
  — D-16).
- `crates/paladin-memory/src/garrison/sqlite_garrison.rs` — runtime migrator `:108` (D-17),
  `INSERT` `:290-301`, `SELECT` `:315`; `crates/paladin-memory/migrations/001_create_garrison_tables.sql`
  (byte-identical root copy at `migrations/`, `Dockerfile:28,57`,
  `docs/src/deployment/docker.md:138,152`); `crates/paladin-memory/src/garrison/token_counter.rs`
  — `TokenCounter` `:12`, `TiktokenCounter` `:28-131`, `TokenCounterFactory` `:136` (D-13);
  `crates/paladin-memory/Cargo.toml:18-22` features `sqlite` / `qdrant` / `content-processing`;
  `crates/paladin-memory/src/sanctum/{in_memory_adapter,qdrant_adapter}.rs`,
  `crates/paladin-ports/src/output/{sanctum_port.rs:585,embedding_port.rs:371}` (D-24).
- `crates/paladin-memory/src/services/rag_retrieval_service.rs:187-196` — the pre-existing inline
  `/4` heuristic D-13 leaves alone.
- `crates/paladin-battalion/src/engine/hooks.rs` — `InterceptDecision` `:175`, `NodeInterceptor`
  `:232` and its "Aegis wraps OUTSIDE" rustdoc `:208-230` (the two-layer table's other half, D-05).
- `crates/paladin-battalion/src/engine/node.rs` — `NodeContext` `:51-100` + accessors `:102-135`
  (D-21 adds `vault`); `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Paladin`
  `:41-58` (D-29 adds `output_schema`), `validate`, `fingerprint()` (`v5` → `v6`);
  `crates/paladin-battalion/src/engine/superstep.rs` — `execute_vanguard_node` `:892`, the Paladin
  arm `:907-960` (`execute_observed` `:946` → `execute_scoped`, D-21; `directive_parser.parse`
  `:956` — the structured branch D-29 adds beside it); `crates/paladin-battalion/src/engine/mod.rs`
  — `WarEngine` builders (`with_node_cache` is the D-29 wiring precedent).
- `crates/paladin-battalion/src/engine/directive_parser.rs` — `DirectiveParser` `:65`,
  `OnParseError` `:85`, private `extract_envelope` + `plain_output_directive` (D-26 lifts the
  extraction rule to core).
- `crates/paladin-battalion/src/engine/retry.rs` — `backoff_delay` `:52`, `wait_backoff` `:78`,
  `should_retry` `:105` (D-11 refactors the predicate into core);
  `crates/paladin-core/src/platform/container/aegis.rs` — `RetryPolicy` `:98-126`,
  `RetryPredicate` `:149` (`#[non_exhaustive]` — gains `admits`).
- `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter` `:115`, `new` `:152`,
  `with_trace_sink` `:168`, `SERVED_BY_METADATA_KEY` (D-11); `crates/paladin-llm/src/http_status.rs`
  — `map_http_status` (D-31's bar); `crates/paladin-llm/src/compat/engine.rs` — `CompatEngine`,
  `CompatRequestParameters` `:106-134`, `build_request`, `map_error` `:604`, shared tests
  `:1123-1508` (one `response_format` change covers five presets — D-28);
  `crates/paladin-llm/src/{openai_compatible,gemini,ollama}/adapter.rs` — the measured test
  inventories D-31 starts from (`openai_compatible` `:911-1267`, `gemini` `:1251-2494`, `ollama`
  one mockito test); `crates/paladin-llm/src/provider_factory.rs` — `LlmProviderFactory::create`
  `:356`, `provider_names` `:312` (D-12); `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter`
  `:79` (`with_responses` `:92`, `with_error` `:106`, `with_error_then_response` `:156`,
  `with_stream_items` `:182`, `call_count` `:196`) — unchanged except recording `response_format`;
  `crates/paladin-llm/src/lib.rs` — `test_capabilities_tool_calling_matches_request_surface`
  (must still pass, D-36); `crates/paladin-llm/src/redaction.rs` — `redact_credentials` `:107`,
  `bounded_excerpt` `:50`, `diagnostic_excerpt` `:124` (D-34).
- `src/application/services/arsenal/arsenal_execution_service.rs` — `McpToolInvoker` seam `:27`,
  `ArsenalExecutionService` `:60`, `invoke` `:189` (routes by `clients_by_tool` — why D-22 adds an
  in-process arsenal); `src/application/services/arsenal/arsenal_registry_service.rs`;
  `src/infrastructure/adapters/arsenal/tool_result_formatter.rs` — `ToolResultFormatter` `:66`,
  `format_result` `:157` (D-34 adds `format_error`);
  `crates/paladin-core/src/platform/container/arsenal/core.rs` — `Armament` `:17`, `ArmamentCall`
  `:36`, `ArmamentResult`; `crates/paladin-ports/src/output/arsenal_port.rs` — `ArsenalPort`
  `:470` (`list_armaments` `:488`, `invoke` `:522`, `validate_call` `:551`), `ArsenalRegistry` `:788`.
- `crates/paladin-ports/src/output/paladin_port.rs` — `PaladinPort` `:630` with the Phase 25
  defaulted `execute_observed` (D-21 adds `execute_scoped` the same way);
  `paladin_executor_port.rs:60`, `streaming_executor_port.rs:66`.
- `src/config/node_cache.rs` (the X-09 template incl. hand-written `Debug`), `src/config/engine.rs`,
  `src/config/env_utils.rs:33` (`EnvOverridable`), `src/config/mod.rs` (registration + re-exports)
  — D-10.
- `src/application/services/paladin/paladin_builder.rs` — `PaladinBuilder` `:77` (`new` `:122`,
  `system_prompt` `:165`, `max_loops` `:285`, `with_garrison` `:658`, `with_arsenal_registry`
  `:686`, `build` `:1267` → `Paladin`; handoff-tool auto-registration `:1365-1378`) — D-35's preset
  composes it; `crates/paladin-core/src/platform/container/paladin.rs` — `MaxLoops` `:42`,
  `PaladinData` `:142`; `crates/paladin-core/src/platform/container/paladin_config.rs` —
  `PaladinConfig` `:44` (untouched — D-10, D-34); `paladin_error.rs` — `PaladinError`
  (`#[non_exhaustive]` since Phase 25 — D-09, D-26, D-33).
- `crates/doc-examples/src/fault_tolerance.rs` + `crates/doc-examples/Cargo.toml` — the `// ANCHOR:`
  + `cargo check` pattern D-35/D-38 follow; `docs/src/SUMMARY.md:25` — the slot the new page
  follows; `docs/src/getting-started/configuration.md:84-91` — the Ollama config block D-32 links.
- `tests/integration/ollama_docker_test.rs` (D-32), `.github/workflows/ci.yml` — `api-surface`
  `:193`, `msrv` `:251`, `semver` `:303`, `ollama-integration` `:748`, `coverage` `:979`.
- `tests/helpers/{mock_llm_adapter,mock_arsenal_adapter,mock_paladin_port}.rs`,
  `tests/unit/paladin_execution_service_test.rs`, `src/application/services/orchestration/listener.rs`
  (the X-05 house pattern).

### Established Patterns
- Hooks live beside what they wrap; an empty chain is proven equivalent to no chain — D-01, D-02.
- Defaulted `PaladinPort` methods whose default is *correct* (claims no capability) — D-21, D-27.
- X-10: `#[non_exhaustive]` + `_` arms / constructor migration + `Y` row + allowlist entry +
  per-crate lint suppression in one commit; deliberate-zero notes for new-in-0.10 types — D-07,
  D-17, D-28, D-37.
- Fail-closed engine wiring resolved at validation, listing offenders before any node runs
  (`with_node_cache`, CF-01 registries) — D-29.
- Fingerprint discipline: hash what changes state or routing, exclude tuning, bump the tag, re-pin
  the golden — D-29.
- Config structs standalone under `src/config/` (`Default` + `validate()` + `EnvOverridable`,
  `APP_*`), inert by default; per-node/per-run policy in code — D-10.
- Persistence adapters behind existing cargo features, InMemory ungated, one contract suite,
  Docker-only tiers routed to UAT — D-18, D-23, D-24, D-39.
- Embedded `sqlx::migrate!` migrations, idempotent construction, no down migration — D-17, D-23.
- Redact-then-bound for every remote or model-facing string that enters an error, a log or a prompt
  — D-34, D-41.
- Typed `thiserror` errors, `#[non_exhaustive]`, no new stringly variants (X-06) — D-09, D-19, D-26.
- Ubiquitous language: Vault, Armament, Arsenal, Garrison, Waypoint, Aegis, WarEngine, Directive —
  in code, docs and comments.

### Integration Points
- `src/application/services/paladin/` — new `middleware/`, `structured.rs`, `run_scope` plumbing
  in `paladin_execution_service.rs` (assembly, hooks, port-shaping call site, `execute_scoped`,
  `with_middleware` / `with_token_counter` / `with_vault` / `enable_vault_tools` /
  `with_tool_error_config`, `StructuredExecutorPort` impl); new `src/presets/` (or
  `application/presets`) for `reasoning_agent`; `src/application/services/arsenal/` — new
  `in_process_arsenal.rs`, `composite_arsenal.rs`, `vault_tools.rs`; `src/lib.rs` prelude exports.
- `src/config/agent_runtime.rs` (+ `mod.rs` registration and re-exports).
- `crates/paladin-core/src/platform/container/` — new `vault.rs`, `structured.rs`, `run_scope.rs`;
  `execution_result.rs` (`StopReason`), `garrison.rs` (`is_summary`, `summary()`,
  `#[non_exhaustive]`), `aegis.rs` (`RetryPredicate::admits`), `paladin_error.rs` (variants),
  `lib.rs` / prelude.
- `crates/paladin-ports/src/output/` — new `vault_port.rs`, `token_counter_port.rs`,
  `structured_executor_port.rs`; `llm_port.rs` (`LlmRequest` builder + `response_format`,
  `ResponseFormat`); `paladin_port.rs` (`execute_scoped` default); `mod.rs`.
- `crates/paladin-memory/src/` — new `vault/{mod,in_memory,sqlite,semantic,contract_tests}.rs`,
  `token_counter/` (heuristic + `TiktokenCounter` impl); `garrison/sqlite_garrison.rs` (embedded
  migrator, column); `garrison/in_memory_garrison.rs`; `migrations/002_*.sql`, `003_*.sql`;
  `lib.rs` / prelude.
- `crates/paladin-llm/src/` — `openai/adapter.rs`, `compat/engine.rs`, `gemini/adapter.rs`,
  `deepseek/adapter.rs` (`response_format`), `mock.rs` (records it), new `conformance.rs` +
  per-adapter instantiations, `redaction.rs` (`redact_secret_patterns`), `lib.rs`.
- `crates/paladin-battalion/src/engine/` — `graph.rs` (`output_schema`, validation matrix,
  fingerprint `v6`), `superstep.rs` (`execute_scoped`, structured branch), `node.rs`
  (`NodeContext.vault`), `mod.rs` (`with_vault`, `with_structured_executor`, `with_output_schema`,
  `EngineError` variants), `directive_parser.rs` (calls `extract_json`), `retry.rs`
  (`should_retry` via `admits`), `test_support.rs`.
- Root: `migrations/` (removed), `Dockerfile:28,57`, `Cargo.toml` (`schemars` dependency; no new
  feature), `.cargo/semver-checks-allowlist.toml`, per-crate lint metadata, `MIGRATION.md`,
  `CHANGELOG.md`, `.project/current-exports.txt`, `.project/v0.10.0/08-traceability-matrix.md`.
- Docs: `docs/src/user-guides/agent-runtime.md` (new), `docs/src/SUMMARY.md`,
  `docs/src/user-guides/{tool-integration,garrison-memory}.md`, `docs/src/deployment/docker.md`,
  `crates/doc-examples/src/agent_runtime.rs` (+ `lib.rs` registration).
- Tests: `tests/integration/` — middleware-under-engine, vault confinement + stress, structured
  engine node, `reasoning_agent` end-to-end on the mock; `tests/unit/` — service tests; per-crate
  units and contract suites.
- **Constraints confirmed in tree:** the prompt is a flat string, not a message list (D-02);
  tool failures are already fed back to the model (D-33); no shipped adapter emits
  `function_call` and no tool catalogue reaches the prompt (D-36); the Garrison SQLite adapter
  reads migrations from the process CWD and the root copy is byte-identical (D-17); `LlmRequest`
  has no `Default` and 37 literal-construction files (D-28); `StopReason` is matched exhaustively
  only by three first-party display mappers (D-07); `GarrisonPort` has no delete-by-id (D-16);
  `schemars 1.2.1` is already resolved via `rmcp 2.1.0` (D-26); `ProviderCapabilities.
  max_context_tokens` already exists (D-14); every shipped `ArsenalPort` routes to MCP (D-22);
  the loop always runs to `max_loops` (D-36); `tests/integration/ollama_docker_test.rs` already
  probes `OLLAMA_TEST_URL` (D-32).

</code_context>

<specifics>
## Specific Ideas

- **Onion-ordering test (PRD §3.1):** three recording middleware A, B, C; B's `before_model`
  returns `Finish` on call 1 → observed sequence `A.before, B.before, A.after` (C's `before` and
  `after` never run, B's `after` never runs); result `stop_reason` from `FinalResult`; zero port
  calls.
- **Isolation test (RT-FR-02):** one service, `ModelCallLimit { max_calls: 3 }`, ten concurrent
  runs on a scripted mock → every run makes exactly 3 model calls, `StopReason::CallLimit` on all
  ten, total port calls 30, under a 10 s guard on `flavor = "multi_thread"`.
- **Limits by count (PRD §3.2):** `TokenBudget { max_tokens: 250 }` with a mock reporting 100
  tokens per call → 3 calls (the third crosses), `token_count == 300`, `StopReason::TokenBudget`,
  `is_successful()`; `ToolCallLimit { max_calls: 2 }` with a scripted tool-calling mock → the third
  tool call is denied, the model sees "tool budget exhausted", the run completes.
- **Summarization (PRD §3.3):** 30 messages, `threshold_messages: 30`, `keep_recent: 10` → one
  summary entry (`is_summary`, role `System`, `summarized_through` set) and the next prompt renders
  summary + 10 raw; add 20 more → second summary built from summary #1 + the 10 oldest raw; a
  summarizer mock returning `ProviderError { status: 503 }` → `scratch["summarization.degraded"]`,
  the trimmed history fits the limit, the run completes.
- **Vault attack test (PRD §3.4):** grant `["user","alice"]`; scripted tool call `vault_put
  {"namespace": ["user","bob"], "key": "x", "value": 1}` → `NamespaceDenied`, the model sees the
  denial as a tool error, the store's call count is 0; sibling grants `["user","alice"]` /
  `["user","alice2"]` do not prefix-match each other; `["user","alice",".."]` fails
  `Namespace::new`.
- **Structured (PRD §3.5):** `#[derive(Deserialize, JsonSchema)] struct Weather { city: String,
  temp_c: f32 }`; mock responses `["{\"city\": \"Oslo\"}", "{\"city\":\"Oslo\",\"temp_c\":4.5}"]`
  → attempt 2 succeeds, `raw.loop_count` reflects both calls; with `max_repair_attempts: 1` and
  two bad responses → `StructuredOutputInvalid { attempts: 2, raw_output: "<second>" }`; an engine
  node with `output_schema: Inline(schema_for!(Weather))` writes `{"city":..,"temp_c":..}` (a JSON
  object, not a string) to `output_field`.
- **`reasoning_agent` doc example (≤ 15 lines):** `MockLlmAdapter::with_responses([r#"```json
  {"tool":"add","arguments":{"a":2,"b":2}}```"#, "The answer is 4"])`, `InProcessArsenal::new()
  .with_tool(add_armament, |args| async { .. })`, `reasoning_agent(llm, Arc::new(arsenal),
  Default::default())?.run("What is 2+2?").await?` → output contains "4", `loop_count == 2`,
  `StopReason::Completed`.
- **Conformance measurement table** (D-31): rows = nine adapters, columns = the suite's cases,
  cells = pass / gap-closed-in-plan-NN / n.a.; recorded in the plan SUMMARY and G-22.
- **`LlmRequest` builder shape:** `LlmRequest::new("gpt-4", prompt).with_stream(true)
  .with_response_format(ResponseFormat::JsonObject)`; the doc test on `new` shows every field's
  default.
- Ubiquitous language holds: Vault, Armament, Arsenal, Garrison, Waypoint, Aegis, WarEngine,
  Directive, Muster, Parley.

</specifics>

<deferred>
## Deferred Ideas

- **`after_model` and `Guardrail` response screens on the streaming path** (D-04) — needs a
  buffered or chunk-wise screen; the stream path runs `before_model` only this phase.
- **LLM-native tool calling** (`LlmRequest.tools`, adapter emission of `function_call`) — ADR-0042's
  deferred capability with its own trigger; D-36's prompt-level protocol is not it.
- **`StopReason::Guardrail` variant** (D-09) — free under `#[non_exhaustive]`, not an RT FR.
- **`ProviderCapabilities` structured-output flag** (D-28) — an X-10.3 event for no FR; the
  per-provider table in the guide covers it.
- **Full JSON Schema validation** (`jsonschema` crate behind a feature) for untyped `SchemaRef::
  Inline` schemas (D-30) — the shape check is documented as partial.
- **Combining `output_schema` with `StructuredDirective`** (a schema-typed envelope `delta`) — D-29
  makes them mutually exclusive.
- **Provider token-count endpoints** (Anthropic, Gemini) as `TokenCounterPort` adapters (D-13).
- **Routing `RagRetrievalService`'s inline `/4` heuristic through `TokenCounterPort`** (D-13) — a
  pre-existing site left under X-03.
- **A `GarrisonPort` delete-by-id / compaction method** so old summaries can be removed (D-16) —
  latest-summary-wins makes it unnecessary for correctness.
- **`NodeInterceptor` visibility of `NextStep`** — Phase 23 deferred idea, still deferred (D-05).
- **A `TraceEvent` for summarization degradation, middleware `Finish`/`Fail`, vault recall and
  structured repair attempts** — OBS-01/02 (Phase 28).
- **Deriving `RunScope` (user id, run id, vault namespace) from an HTTP run request** — PLAT-*
  (Phase 27) builds on D-21.
- **Automatic Vault write/extraction policies** — FUT-06 (PRD 05 §5); **per-token cost accounting**
  — FUT-08; **rate-limit pacing** — FUT-09.
- **A hanging middleware hook** is bounded only by the surrounding timeouts (D-41) — a per-hook
  timeout is a later decision.
- **Registering `TypedSchema<T>` by name from config** — schemas are code-registered this phase
  (the CF-05 / Aegis "policy is code" rule).
- **22-REVIEW.md WR-01/WR-02, 22-deferred-items.md item 1 (`qdrant` `--all-features` rustdoc
  break), 24-REVIEW.md's advisory warnings, 25-REVIEW.md follow-ups** — unchanged, not this
  phase's.

### Reviewed Todos (not folded)
- None matched this phase (`todo.match-phase 26` returned no matches; the single pending todo,
  "Verify local make coverage reproduces CI's 82.39% figure", is a local-tooling check owned by
  the maintainer).

</deferred>

---

*Phase: 26-agent-runtime-enhancements*
*Context gathered: 2026-09-06*
