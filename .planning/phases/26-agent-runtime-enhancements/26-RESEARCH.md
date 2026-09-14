# Phase 26: Agent Runtime Enhancements - Research

**Researched:** 2026-09-06
**Domain:** Rust hexagonal-architecture agent runtime — middleware pipelines, context-window
management, confined cross-session memory, structured LLM output, multi-provider conformance
testing
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

PRD 05 is the FR-level source of truth and already locks the trait sketches (§2.1, §2.3, §2.4),
the FR semantics (RT-FR-01…24), the acceptance criteria (§3), the TDD ordering (§4) and the
out-of-scope list (§5). The decisions below settle only what PRD 05 left open, or what the shipped
Phase 22–25 tree makes concrete — including four places where the tree contradicts a PRD premise
(D-02, D-29, D-31, D-32), which are flagged with ⚠ for the developer to overturn at plan review if
wanted. Do not re-litigate anything PRD 05, PRD 01–04, overview §3 (X-01…X-11) or the Phase
22/22.1/23/24/25 CONTEXT decisions state.

#### Middleware chain: home, contexts & hook placement (RT-01; RT-FR-01…03)

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

#### Built-in limits & guardrails (RT-02; RT-FR-04…07)

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

#### Retry & fallback middleware (RT-02; RT-FR-09)

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

#### Context-window management (RT-03; RT-FR-08, 10…12)

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

#### Vault: types, adapters, confinement & the host grant (RT-04; RT-FR-13…16)

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

#### Structured output (RT-05; RT-FR-17…19)

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

#### Provider conformance close-out (RT-06; RT-FR-20…22)

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

#### Presets & tool-error ergonomics (RT-07; RT-FR-23, 24)

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

#### Program bookkeeping, tests & docs

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

### Deferred Ideas (OUT OF SCOPE)

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

**Phase boundary (out of this phase, from CONTEXT.md `<domain>`):** LLM-native tool calling on
`LlmRequest` and the shipped adapters (ADR-0042 stays deferred with its trigger unchanged); automatic
Vault write/extraction policies (FUT-06); per-token cost accounting (FUT-08); provider rate-limit
pacing (FUT-09); UI for memory management (PRD 05 §5); rebuilding any v0.8.0 adapter
(REQUIREMENTS.md out-of-scope table); the authoritative `TraceEvent` enum and any new trace consumer
(OBS-01/02, Phase 28); background runs, assistants and the per-user platform surface that would
derive a `RunScope` from an HTTP request (PLAT-*, Phase 27); `MIGRATION.md` §9.8 finalisation and the
v0.9-config boot test (SHIP-01/02, Phase 29). Any other behavioral change discovered
mid-implementation is an X-03 stop-and-flag event, not a judgment call.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|--------------------|
| RT-01 | Ordered `ExecutionMiddleware` chain (before/after model, around tool) with onion ordering, short-circuit semantics, per-run state isolation, and engine-node parity | Architecture Patterns Pattern 1 (onion-ordered middleware); Standard Stack confirms no new dependency needed (bespoke trait, `tower` explicitly not evaluated per locked D-01); Validation Architecture maps the onion-ordering and isolation-stress tests to concrete `cargo test` commands |
| RT-02 | Built-in `ModelCallLimit`/`TokenBudget` (new `StopReason` variants), `ToolCallLimit`, `Guardrail`, and retry/fallback middleware delegating to FT-05 | Don't Hand-Roll (reuse FT-05 backoff/predicate, do not duplicate); Common Pitfall 5 + Code Example 2 (Guardrail regex size-bound construction); Security Domain (ReDoS mitigation via `regex`'s linear-time guarantee) |
| RT-03 | `TokenCounterPort`, `HistoryTrimmer`, compounding `SummarizationMiddleware`, Garrison `is_summary` migration | Standard Stack (`tiktoken-rs` already resolved, no version change); Common Pitfall 3 (`get_bpe_from_model` deprecation, informational only); Common Pitfall 4 + Code Example 3 (shared embedded `sqlx` migrator across two stores) |
| RT-04 | `VaultPort` (InMemory/SQLite/semantic) with confinement, `vault_get`/`vault_put` Armaments, `NodeContext::vault()` | Architecture Pattern 3 (confinement-by-decorator, segment-wise prefix matching); Common Pitfall 4 (shared migrator for Vault + Garrison tables); Security Domain (namespace traversal threat pattern); Open Question 2 (Vault-at-rest encryption is out of scope for this phase) |
| RT-05 | `execute_structured<T>` via `StructuredExecutorPort`, schemars-generated schemas, native `response_format`, bounded repair loop | Standard Stack + Package Legitimacy Audit (`schemars` 1.2.1 already resolved via `rmcp`); Common Pitfalls 1 & 2 + Code Example 1 (version-pinning and the `Schema`→`Value` conversion API that changed in schemars 1.0) |
| RT-06 | Shared conformance suite across the shipped OpenAI-compatible/Gemini/Ollama paths, FT-01-correct transience mapping | Environment Availability (`OLLAMA_TEST_URL` fallback already established); Validation Architecture Req→Test map row for RT-06's macro-generated per-adapter suite |
| RT-07 | `reasoning_agent(llm, tools, opts)` one-liner preset, tool failures fed back to the model by default | Don't Hand-Roll is not directly applicable (no new library); Validation Architecture Req→Test map (doc-test command); User Constraints D-33/D-35/D-36 carry the full behavioral spec verbatim |
</phase_requirements>

## Summary

Phase 26 is unusual among the phases in this milestone: `26-CONTEXT.md` (from `/gsd-discuss-phase`)
already carries 41 numbered decisions (D-01…D-41) that pin exact types, file paths, line numbers in
the existing tree, and test shapes for every one of RT-01…RT-07. It is not a set of open questions
for a planner to resolve — it is close to a plan already. This RESEARCH.md therefore does two
things: (1) reproduces that context verbatim as `## User Constraints` so the planner does not have
to re-derive it, and (2) adds the external verification the context did not need to do itself —
confirming the exact crate versions already resolved in `Cargo.lock`, the current API shape of the
two crates newly promoted from transitive to direct dependencies (`schemars`, `tiktoken-rs`), the
`regex` crate's ReDoS-safety claim, and `sqlx::migrate!`'s compile-time embedding semantics — plus a
handful of pitfalls the context's line-number citations do not surface (a second `schemars` major
version already in the lockfile; `schemars` 1.x's `Schema` wraps a `serde_json::Value` and needs an
explicit `.to_value()` call, not a struct literal; `tiktoken-rs`'s `get_bpe_from_model`, used by the
file this phase touches, is upstream-deprecated).

**Primary recommendation:** Plan directly from `26-CONTEXT.md`'s D-01…D-41 and D-40's ten-wave
outline; use this document's Standard Stack, Common Pitfalls and Code Examples sections to fill the
external-verification gaps the context intentionally left to research (exact crate versions, API
shapes, and the two or three failure modes that only show up when actually wiring `schemars` 1.x and
a second embedded `sqlx` migrator into an existing crate).

## Architectural Responsibility Map

Paladin is a Rust hexagonal-architecture workspace, not a multi-tier web app, so tiers are mapped to
the project's own layers (core → ports → application/facade → infrastructure adapters; see
`CLAUDE.md` "Workspace layout" and the "Dependencies flow inward only" rule) rather than
browser/SSR/API/CDN.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Execution middleware chain (RT-01) | Application/Facade (`paladin-ai` root crate, beside `PaladinExecutionService`) | — | D-01: the hook lives beside the thing it wraps, mirroring `NodeInterceptor` in `paladin-battalion`; no consumer outside the facade needs the trait, so it does not belong in `paladin-ports` |
| Built-in middleware / limits & guardrail (RT-02) | Application/Facade | Core (`paladin-core`, `StopReason`/`PaladinError` enum growth) | Middleware logic is facade-owned; the new `StopReason`/`PaladinError` variants it emits are core value types |
| Context-window management (RT-03) | Ports (`paladin-ports::output::token_counter_port`) | Infrastructure (`paladin-memory` — heuristic + tiktoken adapters) | `TokenCounterPort` is a stable abstraction other crates depend on; concrete counting logic is an adapter concern |
| Vault cross-session memory (RT-04) | Core (`paladin-core` — `Namespace`, `VaultRecord`, `VaultError`) + Ports (`VaultPort`) | Infrastructure (`paladin-memory` — InMemory/SQLite/semantic adapters) | ADR-0016: core owns port value types; adapters are infrastructure by definition |
| Structured output (RT-05) | Core (pure `extract_json`/`shape_check` machinery) + Ports (`StructuredExecutorPort`) | Application/Facade (`schemars` glue, service impl) | Parsing/validation logic has no I/O and belongs in core per ADR-0015 (no new core dependency); `schemars` itself is facade-only |
| Provider conformance close-out (RT-06) | Infrastructure (`paladin-llm` adapters) | — | Verification of adapters already shipped in that crate; no new port surface |
| Presets & tool-error ergonomics (RT-07) | Application/Facade (`presets::reasoning_agent`) | — | A convenience composition over existing facade services; not a new abstraction layer |

## Package Legitimacy Audit

Neither `schemars` nor `tiktoken-rs` is a *new* addition to the dependency graph — both are already
present in `Cargo.lock` as transitive dependencies (`schemars 1.2.1` via `rmcp 2.1.0`; `tiktoken-rs
0.6.0` already a direct optional dependency of `paladin-content`/`paladin-memory` behind the
`content-processing`/`tiktoken` features). This phase promotes `schemars` from transitive to a
**direct** facade dependency (D-26) and reuses the existing `tiktoken-rs` dependency as-is (D-13).
Verified via `gsd-tools query package-legitimacy check --ecosystem crates`:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|--------------|---------|-------------|
| `schemars` | crates.io | published 2019-08-08 (~7 yrs) | ~12.3M/wk | github.com/GREsau/schemars | OK | Approved — already resolved at `1.2.1` in `Cargo.lock` via `rmcp`; add as a direct facade dependency pinned to that same major.minor |
| `tiktoken-rs` | crates.io | published 2023-02-02 (~3.5 yrs) | ~509K/wk | github.com/zurawiki/tiktoken-rs | OK | Approved — already a direct optional dependency at `0.6.0`; no version change needed for RT-03 |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

`[VERIFIED: crates.io registry + Cargo.lock]` — both packages exist, are not deprecated, carry no
postinstall/build-script network calls flagged by the check, and are already load-bearing in this
exact workspace.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|---------------|
| `schemars` | `1.2.1` (pin `"1.2"` in the facade's `Cargo.toml`) | Derives JSON Schema from Rust types (`#[derive(JsonSchema)]`, `schema_for!`) for `execute_structured<T>` (RT-05) | Already resolved in `Cargo.lock` via `rmcp 2.1.0` at exactly this version — adding it as a direct dependency at the same minor version adds zero new packages to the graph `[VERIFIED: Cargo.lock]` |
| `tiktoken-rs` | `0.6.0` (existing) | `TiktokenCounter`'s BPE-based token counting for the optional precise `TokenCounterPort` adapter (RT-03) | Already a direct optional dependency of `paladin-memory` behind `content-processing`; no version bump required `[VERIFIED: crates/paladin-memory/Cargo.toml:38]` |
| `regex` | `1.12.3` (existing) | `GuardrailRule::Regex` prompt/response screens (RT-02) | Already a workspace dependency; its Pike-VM/finite-automata design guarantees O(m·n) worst-case matching with no backtracking — the crate is explicitly designed to run untrusted patterns and untrusted haystacks without ReDoS risk `[VERIFIED: docs.rs/regex, github.com/rust-lang/regex]` |
| `sqlx` | `0.8.6` (existing, `sqlite` feature) | Embedded compile-time migrations for the Garrison `is_summary` column and the new Vault tables (RT-03, RT-04) | Already the workspace's persistence layer; `sqlx::migrate!` is the same macro `SqliteWaypointStore` already uses (the precedent D-17/D-23 cite) `[VERIFIED: Cargo.lock; docs.rs/sqlx/latest/sqlx/macro.migrate.html]` |
| `thiserror` | existing | Structured, `#[non_exhaustive]` error enums for `VaultError`, `PaladinError::StructuredOutputInvalid`/`GuardrailTripped` | Already the project's house error pattern (`CLAUDE.md`, `rust.instructions.md`) |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | existing | `Structured<T>`'s untyped JSON value, `VaultRecord.value`, `LlmRequest.response_format` payloads | Already ubiquitous in the workspace |
| `async-trait` | existing | `#[async_trait]` on `VaultPort`, `StructuredExecutorPort`, `ExecutionMiddleware` if any hook is async | Matches every other port trait in `paladin-ports` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Bespoke `ExecutionMiddleware` trait (facade-owned) | `tower::Service`/`tower::Layer` | `tower` is designed around request/response transforms, not a three-hook (before/after/around) chain with typed short-circuit results and per-run scratch state; the PRD's trait sketch is bespoke and D-01 already rejected moving it to `paladin-ports`, so introducing `tower` would add an unrelated dependency for a shape mismatch — not evaluated further per the locked decision |
| `regex` for Guardrail patterns | `fancy-regex` | `fancy-regex` adds backtracking for look-around/backreferences at the cost of the ReDoS-safety guarantee D-09 explicitly relies on ("the `regex` crate is linear-time, so there is no ReDoS surface") — do not substitute |
| Partial `shape_check` (D-30) | `jsonschema` crate (full Draft 2020-12 validator) | Full validation is heavier (new dependency, broader default feature surface, X-11.4 concern) for no RT-FR that requires it; deferred by design, not oversight |
| Hand-rolled token estimator | `tiktoken-rs` | BPE tokenization is provider-specific and easy to get subtly wrong (wrong vocab, wrong special-token handling); `tiktoken-rs` is already vetted and in the tree — do not reimplement |

**Installation (Cargo.toml changes, no new lockfile packages):**
```toml
# Facade Cargo.toml — new direct dependency, same version already in Cargo.lock via rmcp
[dependencies]
schemars = "1.2"
```
No other `Cargo.toml` changes are required for RT-03/RT-04 — `tiktoken-rs`, `regex`, `sqlx` are
already present at the versions above.

**Version verification performed this session:**
```bash
grep -A2 '^name = "schemars"' Cargo.lock     # -> 0.9.0 (via serde_with) AND 1.2.1 (via rmcp)
grep -A2 '^name = "tiktoken-rs"' Cargo.lock  # -> 0.6.0
grep -A2 '^name = "regex"' Cargo.lock        # -> 1.12.3
grep -A2 '^name = "sqlx"' Cargo.lock         # -> 0.8.6
```
`[VERIFIED: Cargo.lock, read directly this session]`

## Architecture Patterns

### System Architecture Diagram

```
                         PaladinExecutionService::execute_scoped(paladin, input, heartbeat, scope)
                                              │
                                              ▼
                          ┌───────────────────────────────────────┐
                          │  before_model chain (first → last)     │  ← D-01, D-04
                          │  limits/guardrail/trimmer/recall/       │
                          │  protocol/resilience (fixed order,      │
                          │  D-10 build_chain)                      │
                          └───────────────────┬───────────────────┘
                                              │ Continue | Finish | Fail
                                              ▼
                       PromptAssembly rendered → flat prompt string (D-02)
                                              │
                                              ▼
                          execute_with_retry_and_temperature(..)      ← D-11 reads
                          (llm_override / retry_policy from context)     ModelCallContext
                                              │
                                              ▼
                       LlmPort::generate(LlmRequest{ response_format }) ← D-28
                                              │
                                              ▼
                          ┌───────────────────────────────────────┐
                          │  after_model chain (last → first)      │  ← onion close, D-01
                          └───────────────────┬───────────────────┘
                                              │
                              ┌───────────────┴────────────────┐
                              ▼                                 ▼
                    function_call present?                function_call absent?
                              │                                 │
                     ┌────────┴─────────┐              ToolCallProtocol.after_model
                     ▼                  ▼              (extract_json envelope,       ← D-36
              Armament branch    Handoff branch          synthesize function_call)
              around_tool wraps  around_tool wraps
              (Deny/Rewrite/     (Deny/Rewrite/
               Continue)          Continue)                     │
                     │                  │                        ▼
                     └────────┬─────────┘              FinishOnPlainAnswer
                              ▼                          (Finish + Completed)  ← D-36
                    accumulated_output, loop again
                    until StopReason::{Completed,MaxLoops,
                    CallLimit,TokenBudget,Timeout}

  Cross-cutting (not in the per-call path):
   VaultRecallMiddleware (loop-1 before_model only) → search(granted_ns, input) → "## Long-term
   memory" assembly section (D-25)
   SummarizationMiddleware (before_model, threshold-triggered) → compresses history, remember()s a
   GarrisonEntry{is_summary:true} (D-16)
   StructuredExecutorPort::execute_json_schema → same service, response_format set + instruction
   block appended, bounded repair loop on parse/shape failure (D-26/D-27)
```

### Recommended Project Structure

```
src/application/services/paladin/
├── paladin_execution_service.rs   # existing — gains with_middleware/with_token_counter/
│                                  # with_vault/enable_vault_tools/with_tool_error_config,
│                                  # execute_scoped, StructuredExecutorPort impl (D-06, D-21, D-27)
├── middleware/
│   ├── mod.rs                     # ExecutionMiddleware, MiddlewareFlow, ToolFlow, contexts (D-01)
│   ├── chain.rs                   # onion-order driver
│   ├── context.rs                 # ModelCallContext, ToolCallContext, scratch bag (D-03)
│   ├── limits.rs                  # ModelCallLimit, TokenBudget, ToolCallLimit (D-08)
│   ├── guardrail.rs                # GuardrailRule, Guardrail (D-09)
│   ├── history.rs                  # HistoryTrimmer (D-15)
│   ├── summarization.rs            # SummarizationMiddleware (D-16)
│   ├── vault_recall.rs             # VaultRecallMiddleware (D-25)
│   ├── resilience.rs               # ModelRetryMiddleware, ModelFallbackMiddleware (D-11)
│   └── tool_protocol.rs            # ToolCallProtocolMiddleware, FinishOnPlainAnswerMiddleware (D-36)
├── structured.rs                   # StructuredExecutorExt blanket impl (D-27)
└── run_scope wiring inline in paladin_execution_service.rs (D-21)

crates/paladin-core/src/platform/container/
├── vault.rs                        # Namespace, VaultRecord, ScoredVaultRecord, Page, VaultError (D-18)
├── structured.rs                   # Structured<T>, StructuredOptions, SchemaRef, extract_json,
│                                    # shape_check, render_instruction_block (D-26)
└── run_scope.rs                    # RunScope (D-21)

crates/paladin-ports/src/output/
├── vault_port.rs                   # VaultPort
├── token_counter_port.rs           # TokenCounterPort (D-13)
└── structured_executor_port.rs     # StructuredExecutorPort, run_structured driver (D-27)

crates/paladin-memory/src/
├── vault/{mod,in_memory,sqlite,semantic,contract_tests}.rs   # (D-18, D-23, D-24)
├── token_counter/                                             # HeuristicTokenCounter (D-13)
└── migrations/{002_add_garrison_is_summary,003_create_vault_tables}.sql  # (D-17, D-23)

crates/paladin-llm/src/
├── conformance.rs                  # ConformanceFixture, llm_conformance_suite! macro (D-31)
└── redaction.rs                    # + redact_secret_patterns (D-34)

src/presets/ (or application/presets)
└── reasoning_agent                 # (D-35)
```

### Pattern 1: Onion-ordered middleware with typed short-circuit

**What:** An ordered `Vec<Arc<dyn ExecutionMiddleware>>` runs `before_model` first-to-last and
`after_model` last-to-first (classic onion/decorator composition), where any hook may return
`Continue`, `Finish(FinalResult)`, or `Fail(PaladinError)`. A `Finish` from hook *i* skips every
`before_model` after *i* but still runs every `after_model` for hooks *before* i whose `before_model`
already ran (D-01, D-06).

**When to use:** Any cross-cutting concern that must (a) see and potentially rewrite the
prompt/response, (b) short-circuit the loop without failing the run, and (c) compose predictably
with other such concerns (this is exactly RT-02's built-ins).

**Example (shape, not literal source — no upstream library implements this exact trait):**
```rust
#[async_trait]
pub trait ExecutionMiddleware: Send + Sync {
    async fn before_model(&self, ctx: &mut ModelCallContext) -> MiddlewareFlow { MiddlewareFlow::Continue }
    async fn after_model(&self, ctx: &mut ModelCallContext, resp: &LlmResponseView) -> MiddlewareFlow { MiddlewareFlow::Continue }
    async fn around_tool(&self, ctx: &ToolCallContext) -> ToolFlow { ToolFlow::Continue }
}

pub enum MiddlewareFlow { Continue, Finish(FinalResult), Fail(PaladinError) }
```
Golden equivalence rule (D-02): with an empty `Vec`, the rendered prompt bytes, port call count and
`PaladinResult` must be byte-identical to today's behavior — provable the same way
`22-09-SUMMARY.md`'s empty-chain equivalence test proved it for `NodeInterceptor`.

### Pattern 2: Bounded typed repair loop for structured output

**What:** Attempt 1 appends a schema-derived instruction block to the input; on parse or shape
failure, re-prompt (≤ `max_repair_attempts`, default 1) with the parse error and the offending raw
output; on exhaustion, return a typed error that preserves the raw output rather than discarding it
(D-26, D-27).

**When to use:** Any `execute_structured<T>` call; this is the generic driver, parameterized over an
`execute_fn` closure so it works identically whether the schema is derived (`schemars::schema_for!`)
or registered by name at the engine layer (`TypedSchema<T>`).

**Example:**
```rust
// schemars 1.x: schema_for! returns schemars::Schema, which wraps a serde_json::Value.
// Convert explicitly — there is no implicit Deref to Value in 1.x (this differs from 0.8,
// where Schema was a struct with named fields). Source: docs.rs/schemars/latest/schemars/struct.Schema.html
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Weather { city: String, temp_c: f32 }

let schema: serde_json::Value = schemars::schema_for!(Weather).to_value(); // .into() also works
                                                                            // via `impl From<Schema> for Value`
```
`[VERIFIED: docs.rs/schemars/latest/schemars/struct.Schema.html — Schema::to_value/as_value,
From<Schema> for Value, TryFrom<Value> for Schema]`

### Pattern 3: Confinement by decorator, not by trusting the caller

**What:** `ConfinedVault { inner: Arc<dyn VaultPort>, granted: Namespace }` wraps any `VaultPort` and
rejects (with a typed error, before touching `inner`) every call whose target namespace does not have
`granted` as a **segment-wise** prefix (`Namespace::is_prefix_of`, not raw string `starts_with`) —
D-19/D-20. Segment-wise prefix matching is the load-bearing detail: a naive string-prefix check would
let `["user","alice2"]` pass a grant of `["user","alice"]` because `"user/alice2".starts_with("user/alice")`
is true even though the segments are siblings, not an ancestor/descendant pair. This is the same class
of bug as path-traversal-via-substring in filesystem APIs; the fix is identical — compare the
tokenized path components, not the joined string.

**When to use:** Any host-granted resource confinement (this phase's Vault; potentially reusable for
future per-tenant scoping).

### Anti-Patterns to Avoid

- **String-prefix namespace matching:** `namespace_str.starts_with(granted_str)` instead of segment
  comparison — passes the sibling-namespace attack the PRD's own test targets. Always compare
  `Vec<String>` segments.
- **Regex without a size bound for user/config-supplied patterns:** `Regex::new(pattern)` alone
  relies on the crate's large default program-size ceiling; for a *documented* bound (D-09), use
  `RegexBuilder::new(pattern).size_limit(N).build()` and surface the resulting error as a typed
  construction failure. `[VERIFIED: docs.rs/regex/latest/regex/struct.RegexBuilder.html]`
- **Treating recalled Vault content or fed-back tool errors as instructions:** both are
  model-controllable text; render them in a clearly delimited, framed section ("stored notes, not
  instructions") rather than inline in the system prompt, per D-25/D-41.
- **Redacting after truncation instead of before:** truncating a string that contains a credential
  can slice the secret across the truncation boundary and leak the tail — `security.instructions.md`'s
  house rule, reinforced by D-34/D-41 for every new tool-error and Guardrail-adjacent string this
  phase adds.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|--------------|-----|
| JSON Schema generation from a Rust type | A custom schema walker/reflection layer | `schemars::JsonSchema` derive + `schema_for!` | Already resolved in the lockfile at `1.2.1`; handles nested types, enums, `Option`, collections, and `#[serde]` renames correctly — a hand-rolled version would need to re-derive all of that |
| Full JSON Schema validation | A from-scratch validator supporting the whole spec | The documented partial `shape_check` (D-30) for now; `jsonschema` crate later if a future phase needs full spec coverage | D-30 already scoped this down deliberately — building more than the documented subset this phase is scope creep, not thoroughness |
| BPE token counting | A hand-written tokenizer approximating GPT vocabularies | `tiktoken-rs` (already a dependency) | Tokenizer vocabularies and merge rules are provider-specific and change; a hand-rolled approximation drifts silently |
| ReDoS-safe pattern matching | A custom regex engine or backtracking-detection heuristic | The `regex` crate as-is (already linear-time by construction) | Reinventing a linear-time regex engine is a multi-year undertaking; the crate already guarantees the property the PRD needs |
| Exponential-backoff retry math and transience predicates | A second copy of backoff/predicate logic inside the new retry middleware | `paladin_battalion::engine::retry::backoff_delay` and the refactored `RetryPredicate::admits` (FT-05/D-11) | RT-FR-09 explicitly requires delegating to the FT-05 implementation "without duplicating logic" — this is a locked decision, not just good practice |
| SQL schema migration tracking | A hand-rolled "has this migration run" table/flag | `sqlx::migrate!` (compile-time embedded, already the `SqliteWaypointStore` precedent) | `sqlx` already tracks applied migrations by version + checksum in `_sqlx_migrations`; duplicating that bookkeeping is pure risk for no benefit |

**Key insight:** every "don't hand-roll" item in this phase already has an in-tree precedent or an
already-resolved dependency — the risk this phase carries is not "no library exists," it is
"forgetting that one already does and re-deriving it slightly differently" (e.g., a second backoff
formula, a second regex-safety story, a second migration-tracking table).

## Common Pitfalls

### Pitfall 1: A second, older `schemars` major version already sits in the lockfile

**What goes wrong:** `Cargo.lock` currently resolves **two** `schemars` versions: `0.9.0` (pulled in
by `serde_with`, unrelated to this phase) and `1.2.1` (pulled in by `rmcp 2.1.0`, which D-26 already
identified). Adding `schemars` as a direct facade dependency without pinning the version precisely
risks Cargo resolving a *third*, newer major version if one has shipped since this research was
performed, bloating the dependency graph and potentially producing a `Schema` type that is not
identical to the one `rmcp`'s tool-schema code already uses.

**Why it happens:** Cargo resolves each dependency edge independently unless versions are
compatible; `schemars = "*"` or an unpinned range in a new `Cargo.toml` line does not automatically
reuse an existing resolution.

**How to avoid:** Pin the facade's new dependency line to `schemars = "1.2"` (matching the minor
version already resolved via `rmcp`), then run `cargo tree -i schemars` after adding it and confirm
exactly two entries remain (`0.9.0` and `1.2.1`) — not three.

**Warning signs:** `cargo build` succeeds but `cargo tree -i schemars` shows three distinct versions;
unexpectedly large diff in `Cargo.lock` touching unrelated crates when only a `schemars` line was
added.

### Pitfall 2: `schemars` 1.x's `Schema` is not a struct with named fields — it wraps a `Value`

**What goes wrong:** Code written against the muscle memory of `schemars` 0.8 (`RootSchema`, a
struct with a `schema: SchemaObject` field, in the `schemars::schema` module) will not compile
against 1.x. `schema_for!` now returns `schemars::Schema`, a newtype-ish wrapper around
`serde_json::Value` (must be `Value::Object` or `Value::Bool`), and `schemars::schema` no longer
exists.

**Why it happens:** `schemars` 1.0 was a deliberate breaking rewrite; the crate already in
`Cargo.lock` (`1.2.1`) is post-rewrite, but this phase's design language (`SchemaRef::Inline(serde_json::Value)`
in D-26) already anticipated the `Value`-wrapping shape correctly — the risk is only in
implementation, not in the decisions already made.

**How to avoid:** Use `Schema::to_value(self) -> Value` (consumes) or `Schema::as_value(&self) ->
&Value` (borrows), or the `From<Schema> for Value` impl, to get the `serde_json::Value` the rest of
this phase's types expect. `[VERIFIED: docs.rs/schemars/latest/schemars/struct.Schema.html]`

**Warning signs:** A compile error naming `schemars::schema::RootSchema` or `SchemaObject` not found;
attempting `schema_for!(T).schema` (0.8 field access) on a 1.x `Schema`.

### Pitfall 3: `TiktokenCounter` already uses an upstream-deprecated function

**What goes wrong:** `crates/paladin-memory/src/garrison/token_counter.rs:9,74` imports and calls
`tiktoken_rs::get_bpe_from_model`, which the `tiktoken-rs` crate's current docs mark deprecated in
favor of `bpe_for_model`. This is not a functional break (the deprecated function still compiles and
works) but D-13 has this file gaining `impl TokenCounterPort` in this phase, so a
contributor touching the file may notice the deprecation warning.

**Why it happens:** `tiktoken-rs` renamed the function after `paladin-memory`'s dependency was
pinned; `Cargo.lock`'s `0.6.0` already contains both names.

**How to avoid:** This is **not** an RT-FR and not a locked decision — do not silently rename it as
part of this phase's diff (X-03 discipline: unplanned behavioral/API changes are a stop-and-flag
event, not a judgment call). If the deprecation warning is noisy enough to fail `cargo clippy -- -D
warnings`, flag it to the developer as a one-line opportunistic fix rather than folding it into an
RT-03 commit silently.

**Warning signs:** `cargo clippy -- -D warnings` failing on `token_counter.rs` with a
`deprecated` lint after this phase's changes land nearby.

### Pitfall 4: Two SQLite adapters (`SqliteGarrison`, `SqliteVault`) sharing one embedded migrator

**What goes wrong:** D-17 switches `SqliteGarrison`'s migrator from a runtime path
(`sqlx::migrate::Migrator::new("./migrations")`, relative to process CWD) to
`sqlx::migrate!("migrations")` (embedded at compile time, relative to `crates/paladin-memory`'s
`Cargo.toml`). D-23 then adds `SqliteVault` using "the same embedded paladin-memory migrator." If
implemented as two *separate* `sqlx::migrate!("migrations")` call sites (one in
`sqlite_garrison.rs`, one in the new `vault/sqlite.rs`), both macro invocations embed the **same**
directory's `.sql` files at compile time (numbered `001`, `002`, `003` together) — that is correct
and is what D-23 means by "run the same embedded migrator." The pitfall is only if a future edit adds
a *second, separate* `migrations/` directory (e.g., a Vault-only one) — that would silently split
migration numbering across two independently-tracked sets, which `sqlx`'s single
`_sqlx_migrations` table (keyed by version + checksum, not by which call site ran it) cannot
reconcile if the two directories ever reuse a version number.

**Why it happens:** `sqlx::migrate!` embeds whatever directory path is passed to it, resolved at
compile time relative to `CARGO_MANIFEST_DIR`; it has no awareness of "the" migrations directory for
a crate beyond that argument.

**How to avoid:** Keep exactly one `migrations/` directory in `paladin-memory` (`001`, `002`, `003`
side by side, as D-17/D-23 already specify) and point every `sqlx::migrate!("migrations")` call site
in that crate at that same literal string. Do not create a second directory for the Vault tables.

**Warning signs:** `sqlx::migrate!` compile errors about duplicate version numbers; a fresh SQLite
file ending up with only some of the expected tables after construction.
`[CITED: docs.rs/sqlx/latest/sqlx/macro.migrate.html — "the directory must be relative to the
project root (the directory containing Cargo.toml)"; "doesn't require the .sql files to be present
at runtime"]`

### Pitfall 5: Guardrail regex construction without an explicit size bound

**What goes wrong:** D-09 requires "a documented pattern-size bound" for `GuardrailRule::Regex`, on
the stated grounds that the crate's linear-time guarantee means there is no ReDoS surface. That
guarantee is about *matching* time, not about *compiling* an arbitrarily large or deeply-nested
pattern, which the crate does bound internally but at a generous default (compilation fails past an
internal size ceiling rather than hanging, but the ceiling is large enough that a config-supplied
pattern could still produce a very large compiled program before hitting it).

**Why it happens:** `Regex::new(pattern)` uses the crate's built-in default `size_limit`; a
"documented" bound per D-09 means the built-in still-generous default should not be silently relied
upon as "the" documented number.

**How to avoid:** Construct `Guardrail` regexes via `RegexBuilder::new(pattern).size_limit(N).build()`
with an explicit, smaller `N` chosen and written down in `GuardrailConfig`'s rustdoc, surfacing a
`Regex(String)` size-limit failure through the same "regexes compile at construction with a typed
error" path D-09 already specifies. `[VERIFIED: docs.rs/regex/latest/regex/struct.RegexBuilder.html
— size_limit affects buildability, not runtime performance; dfa_size_limit is the separate,
per-thread runtime cache bound]`

**Warning signs:** A `GuardrailConfig` with no documented pattern-size number in its rustdoc; relying
on "the regex crate is safe" as the entire security argument without an explicit builder call.

## Code Examples

### `schemars` 1.x derive + conversion to the `serde_json::Value` this phase's ports expect
```rust
// Source: docs.rs/schemars (JsonSchema derive macro, Schema struct) — verified this session
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
struct Weather {
    city: String,
    temp_c: f32,
}

fn weather_schema() -> serde_json::Value {
    schemars::schema_for!(Weather).to_value() // consumes; .as_value() borrows instead
}
```

### `RegexBuilder` with an explicit, documented size bound (D-09)
```rust
// Source: docs.rs/regex/latest/regex/struct.RegexBuilder.html — verified this session
use regex::RegexBuilder;

const GUARDRAIL_PATTERN_SIZE_LIMIT_BYTES: usize = 1 << 16; // documented in GuardrailConfig rustdoc

fn compile_guardrail_pattern(pattern: &str) -> Result<regex::Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .size_limit(GUARDRAIL_PATTERN_SIZE_LIMIT_BYTES)
        .build()
}
```

### Embedded `sqlx` migrations shared by two stores in one crate (D-17, D-23)
```rust
// Source: docs.rs/sqlx/latest/sqlx/macro.migrate.html — verified this session
// Both SqliteGarrison and SqliteVault call this same macro invocation shape; the directory
// path is resolved once at compile time relative to crates/paladin-memory/Cargo.toml.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("migrations");

pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|-------------------|---------------|--------|
| `schemars` 0.8 struct-based `RootSchema`/`SchemaObject` | `schemars` 1.x `Schema` wrapping `serde_json::Value` | `schemars` 1.0 release | Any code copied from pre-1.0 examples (blog posts, older Stack Overflow answers) will not compile against the `1.2.1` already in this lockfile — always check against the 1.x docs, not memory of 0.8 |
| `tiktoken_rs::get_bpe_from_model` | `tiktoken_rs::bpe_for_model` | upstream `tiktoken-rs` deprecation (exact version not pinned by this research; confirmed present in the version already vendored, `0.6.0`) | Cosmetic only for this phase (existing call site keeps working) — noted so it is not silently perpetuated if the file is touched anyway |
| Runtime `sqlx::migrate::Migrator::new(path)` relative to process CWD | Compile-time `sqlx::migrate!("migrations")` embedded in the binary | This phase, D-17 (following the pre-existing `SqliteWaypointStore` precedent) | Removes the fragile root-level `migrations/` mirror copy and the `Dockerfile` `COPY` steps that existed only to keep the runtime path working in containers |

**Deprecated/outdated:**
- `tiktoken_rs::get_bpe_from_model` — superseded by `bpe_for_model`; not this phase's concern to fix,
  flagged only as a nearby-file observation.
- Runtime-relative Garrison migrations (`sqlite_garrison.rs:108`) — superseded in this phase by the
  embedded macro per D-17; the root `migrations/` copy and its two `Dockerfile` `COPY` lines are
  removed in the same change (or mirrored for one release, at Claude's discretion per D-17).

## Assumptions Log

`26-CONTEXT.md` is itself the product of a prior discuss-phase session and its claims are already
treated as locked decisions rather than research findings — this log covers only claims this
research session added that were not independently re-verified against an authoritative source.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|-----------------|
| A1 | The exact upstream version at which `tiktoken_rs::get_bpe_from_model` became deprecated was not pinned — only that the function is deprecated in the docs surfaced for the crate's latest release | Common Pitfalls, Pitfall 3 | Low — this is an FYI, not an action item; no plan task depends on it |
| A2 | `regex::RegexBuilder`'s default `size_limit` is "generous" without citing the exact byte figure | Common Pitfalls, Pitfall 5 | Low — the recommendation (set an explicit, documented bound) is correct regardless of the exact default value; if precision is needed, `docs.rs/regex/latest/regex/struct.RegexBuilder.html#method.size_limit` states the current default at time of implementation |

**If this table is empty:** N/A — see above; both entries are low-risk documentation gaps, not
decisions that need to flip.

## Open Questions

1. **Does `paladin-memory`'s existing `migrations/` directory sit at the crate root, and will both
   `SqliteGarrison` and the new `SqliteVault` resolve `sqlx::migrate!("migrations")` to the identical
   embedded `Migrator` without a second directory being created by mistake?**
   - What we know: D-17/D-23 both state the intent (one shared embedded migrator); the precedent
     (`SqliteWaypointStore`) already does this successfully in the same crate family.
   - What's unclear: the exact call-site wiring (one `static MIGRATOR` reused by both stores, vs. two
     independent `sqlx::migrate!("migrations")` invocations that happen to embed the same directory)
     is Claude's discretion per the context's own "Claude's Discretion" list.
   - Recommendation: prefer one shared `static MIGRATOR` constant in a common module over two
     `sqlx::migrate!` call sites, purely to make the "one embedded migrator" invariant impossible to
     accidentally violate later (see Pitfall 4).

2. **Vault-at-rest encryption is not mentioned anywhere in RT-04's FRs or in D-18…D-25.** SQLite-backed
   Vault values are stored as plaintext JSON (`value TEXT NOT NULL` per D-23's schema).
   - What we know: Milestone 12's non-goals explicitly exclude "encrypting config at rest," and the
     security posture note (D-41) covers redaction, ReDoS and namespace confinement but not
     at-rest encryption for Vault content.
   - What's unclear: whether Vault content is expected to ever hold sensitive data given it is
     explicitly "agent cross-session memory" that a Paladin can `vault_put` arbitrary values into.
   - Recommendation: treat this as consistent with the project's existing "secrets management is the
     operator's responsibility" stance (already applied to LLM API keys) rather than a gap unique to
     this phase — but the planner should not assume encryption-at-rest is silently in scope; it is not
     named by any RT-FR.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|--------------|-----------|---------|----------|
| Rust toolchain | Everything | ✓ | `rust-version = "1.88"` per root `Cargo.toml` (X-11, MSRV since Phase 22.1) | — |
| `sqlite` feature (bundled `sqlx-sqlite`) | `SqliteVault`, `SqliteGarrison` migration change | ✓ | `sqlx 0.8.6` | — |
| `qdrant` feature / Qdrant service | `SemanticVault`'s Sanctum backend proof | Not provable in this environment | — | Contract suite proven with `InMemorySanctumAdapter` + a deterministic mock `EmbeddingPort` (Tier 1); the Qdrant-backed path is routed to UAT per D-24/D-39, matching the existing Phase 24 D-28 precedent — never marked passed locally |
| Docker | Postgres/Redis-tiered contract suites (pre-existing pattern from Phase 22-25) | Not confirmed in this session | — | RT-01…RT-07 do not introduce a new Docker-only tier; all new Tier 2 exposure is limited to the pre-existing Qdrant/Postgres pattern, unaffected by this phase |
| `OLLAMA_TEST_URL` env var | RT-06's Ollama conformance measurement | Not set in this session | — | The existing `tests/integration/ollama_docker_test.rs` already skips gracefully with a printed reason when unset (D-32) — no new fallback needed |

**Missing dependencies with no fallback:** none identified — every external dependency this phase
touches already has an established fallback or UAT-routing precedent from prior phases.

**Missing dependencies with fallback:** Qdrant/`qdrant` feature (routed to UAT), `OLLAMA_TEST_URL`
(existing skip-with-reason pattern), Docker-only tiers (pre-existing pattern, not newly introduced).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in `#[test]`/`#[tokio::test]`), no separate config file |
| Config file | none — workspace-standard `#[cfg(test)] mod tests` + `tests/` integration dirs per crate |
| Quick run command | `cargo test -p paladin-battalion -p paladin-memory -p paladin-llm --lib` (or narrower, per touched crate) |
| Full suite command | `make test-all` (unit + integration); `make test-integration-docker` for Docker-gated tiers |

### Phase Requirements → Test Map

Test shapes below are lifted directly from `26-CONTEXT.md`'s `<specifics>` block (PRD §3 acceptance
items), which the planner should treat as the canonical test list, not a suggestion:

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|---------------------|--------------|
| RT-01 | Onion ordering + short-circuit: 3 recording middleware, B's `before_model` returns `Finish` on call 1 → sequence `A.before, B.before, A.after` only | unit | `cargo test -p paladin-ai onion_ordering -- --nocapture` | ❌ Wave 0 (new middleware test module) |
| RT-01 | Per-run isolation under concurrency: 10 concurrent runs, `ModelCallLimit{max_calls:3}`, exact independent counts | stress (X-05 pattern) | `cargo test -p paladin-ai middleware_isolation_stress -- --test-threads=1` (`flavor = "multi_thread"`, timeout guard) | ❌ Wave 0 |
| RT-01 | Same chain applies to a Paladin run as an engine node | integration | `cargo test --test middleware_under_engine` | ❌ Wave 0 |
| RT-02 | `TokenBudget{max_tokens:250}` with 100-tok/call mock → 3 calls, `StopReason::TokenBudget`, `is_successful()==true` | unit | `cargo test -p paladin-ai token_budget_crossing` | ❌ Wave 0 |
| RT-02 | `ToolCallLimit{max_calls:2}` → 3rd tool call denied, model sees the budget message, run completes | unit | `cargo test -p paladin-ai tool_call_limit_denies` | ❌ Wave 0 |
| RT-03 | 30 messages, `threshold_messages:30`, `keep_recent:10` → one summary; +20 more → second summary from [summary#1 + 10 raw] | unit | `cargo test -p paladin-memory summarization_compounds` | ❌ Wave 0 |
| RT-03 | Summarizer failure (mock 503) → `scratch["summarization.degraded"]`, falls back to trimming, run completes | unit | `cargo test -p paladin-ai summarization_degrades_to_trimming` | ❌ Wave 0 |
| RT-04 | Vault attack test: grant `["user","alice"]`, tool call targets `["user","bob"]` → `NamespaceDenied`, store call count 0 | unit (attack-tested) | `cargo test -p paladin-memory vault_namespace_denied` | ❌ Wave 0 |
| RT-04 | Concurrent confined `vault_put`s from N runs under distinct grants → zero cross-namespace records | stress (X-05) | `cargo test -p paladin-memory vault_confinement_stress -- --test-threads=1` | ❌ Wave 0 |
| RT-05 | Derive-based happy path + repair-on-attempt-2 (scripted mock) + typed exhaustion preserving `raw_output` | unit | `cargo test -p paladin-ai structured_output_repair_loop` | ❌ Wave 0 |
| RT-05 | Engine node with `output_schema` writes parsed JSON (not a string) to `output_field` | integration | `cargo test --test structured_engine_node` | ❌ Wave 0 |
| RT-06 | Shared conformance suite instantiated per adapter — success/streaming/429/5xx/408 transience/redaction | unit (macro-generated per adapter) | `cargo test -p paladin-llm --lib conformance` | ❌ Wave 0 (new `conformance.rs`) |
| RT-07 | `reasoning_agent` ≤15-line doc example: tool-call envelope → tool result fed back → final answer | doc test | `cargo test -p paladin-doc-examples --doc` | ❌ Wave 0 (new `agent_runtime.rs` doc-examples module) |

### Sampling Rate
- **Per task commit:** `cargo test -p <touched-crate>` plus `cargo fmt --check` and `cargo clippy -- -D warnings` on touched files (per `CLAUDE.md`'s "before committing a parent task" rule).
- **Per wave merge:** `cargo test --workspace` (Tier 1 only is sufficient locally; Docker/Qdrant/Ollama tiers run in CI).
- **Phase gate:** `make security`, `cargo semver-checks` (vs 0.9.0, allowlist entries for `StopReason`/`LlmRequest`/`GarrisonEntry`), the `msrv` job at `1.88`, `cargo llvm-cov --fail-under-lines 82` (ADR-0006), and `scripts/check-api-surface.sh .project/current-exports.txt` with the export file regenerated (the Phase 25 carried concern D-37 names explicitly) — full suite green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `src/application/services/paladin/middleware/` module tree — no existing tests to extend; entirely new (RT-01, RT-02)
- [ ] `crates/paladin-memory/src/vault/contract_tests.rs` — new shared contract suite for InMemory/SQLite/Semantic Vault adapters (RT-04)
- [ ] `crates/paladin-llm/src/conformance.rs` — new `ConformanceFixture` trait + `llm_conformance_suite!` macro (RT-06)
- [ ] `crates/doc-examples/src/agent_runtime.rs` — new doc-example module with `// ANCHOR:` regions, registered in `crates/doc-examples/src/lib.rs` (RT-07)
- [ ] Framework install: none — `cargo test` and existing `mockito`/`tests/helpers/mock_*` infrastructure already cover every new test shape above

*(Not empty — ten new test surfaces are genuinely new per the table above; this matches D-39's
statement that "everything RT adds is Tier 1... except the Qdrant `SemanticVault` backend and the
Ollama suite.")*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|--------------------|
| V2 Authentication | No | This phase has no network-facing surface — `RunScope`/Vault confinement is in-process only; deriving a scope from an HTTP request is explicitly Phase 27 (PLAT-*), out of this phase's scope |
| V3 Session Management | No | Same reasoning as V2 |
| V4 Access Control | Yes | `ConfinedVault`'s segment-wise namespace-prefix check (D-19/D-20) is this phase's access-control primitive — analogous to path-traversal protection, implemented in-process rather than at a network boundary |
| V5 Input Validation | Yes | `Namespace::new`/`parse` (segment count/length/charset validation), `VaultRecord` key/value bounds (`ValueTooLarge`), the partial JSON `shape_check` (D-30) for untyped structured output, and `Guardrail`'s regex/predicate input screens |
| V6 Cryptography | Not addressed this phase | Vault values are stored as plaintext JSON in SQLite (see Open Questions #2) — no new cryptographic primitive is introduced or required by any RT-FR; do not silently add encryption-at-rest as unplanned scope (X-03) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-------------------------|
| Namespace/path traversal in Vault confinement (sibling-namespace bypass, `..`/empty/over-long segments) | Tampering, Elevation of Privilege | Segment-wise `Namespace::is_prefix_of` (never raw string `starts_with`); `Namespace::new`/`parse` reject `.`, `..`, `/`, control chars, and out-of-bounds segment counts/lengths — attack-tested per D-19/D-20/D-41 |
| ReDoS via a config-supplied `Guardrail` regex | Denial of Service | The `regex` crate's linear-time, no-backtracking engine (verified this session) plus an explicit `RegexBuilder::size_limit` bound (Pitfall 5) |
| Prompt injection via Vault-recalled content or fed-back tool-error text | Tampering (of model behavior) | Both are rendered in a delimited section explicitly framed as "stored notes, not instructions" (D-25, D-41) rather than inlined into the system prompt |
| Credential/secret leakage through a tool-error string or provider error surfaced to the model, a log, or an error type | Information Disclosure | Redact-then-bound pipeline: `redact_secret_patterns` (D-34, factored out of the existing `redact_credentials`) runs *before* `bounded_excerpt` on every new tool-error and provider-excerpt string this phase adds — truncating first can slice a secret across the boundary and leak the tail (`security.instructions.md`'s explicit house rule) |
| SQL injection via Vault's SQLite adapter | Tampering | `sqlx`'s compile-time-checked/parameterized query macros (the existing `SqliteGarrison`/`SqliteWaypointStore` pattern this phase's `SqliteVault` follows) — no string-concatenated SQL |
| A hanging middleware hook stalling a run indefinitely | Denial of Service | Explicitly **not solved** this phase (D-41): bounded only by the node's Aegis `run_timeout`/the service's per-run timeout; a per-hook timeout is named as a later decision, not a gap unique to this research |

## Sources

### Primary (HIGH confidence)
- `docs.rs/schemars/latest/schemars/struct.Schema.html` — `Schema::to_value`/`as_value`,
  `From<Schema> for Value`, `TryFrom<Value> for Schema` — fetched and read directly this session
- `docs.rs/regex/latest/regex/struct.RegexBuilder.html` — `size_limit` vs `dfa_size_limit` semantics
- `docs.rs/sqlx/latest/sqlx/macro.migrate.html` — compile-time embedding semantics, path resolution
  relative to `Cargo.toml`
- `Cargo.lock` (this repository, read directly) — exact resolved versions of `schemars` (`0.9.0` and
  `1.2.1`), `tiktoken-rs` (`0.6.0`), `regex` (`1.12.3`), `sqlx` (`0.8.6`)
- `crates/paladin-memory/src/garrison/token_counter.rs`, `crates/paladin-memory/Cargo.toml`,
  `crates/paladin-content/Cargo.toml` (this repository, read directly) — confirmed `tiktoken-rs`
  dependency shape and the `get_bpe_from_model` call site
- `.planning/phases/26-agent-runtime-enhancements/26-CONTEXT.md` — the phase's locked decisions
  (D-01…D-41), canonical references, and code-location citations; treated as the primary behavioral
  source of truth for this research, per the phase's own precedence rules

### Secondary (MEDIUM confidence)
- `github.com/rust-lang/regex` (README, fetched via search) — "guarantees linear time matching on
  all inputs," corroborating the docs.rs page
- `github.com/GREsau/schemars` CHANGELOG / migration guide summary (fetched via search) — 1.0
  breaking-change shape (`Schema` wraps `Value`, `RootSchema` removed)

### Tertiary (LOW confidence)
- The exact upstream release at which `tiktoken_rs::get_bpe_from_model` was marked deprecated (only
  confirmed present-tense in current docs, not version-pinned) — see Assumptions Log A1

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every version cited was read directly from this repository's `Cargo.lock`,
  not assumed from training data
- Architecture: HIGH — sourced from `26-CONTEXT.md`'s D-01…D-41, which already cite exact file paths
  and line numbers in the current tree
- Pitfalls: HIGH for `schemars`/`regex`/`sqlx` API shapes (docs.rs, read directly this session);
  MEDIUM for the exact `tiktoken-rs` deprecation version (see Assumptions Log)

**Research date:** 2026-09-06
**Valid until:** 30 days (stable, mature Rust crates; no fast-moving dependency in this phase's
critical path) — re-verify `schemars`/`regex`/`sqlx` versions in `Cargo.lock` if this research is
consumed significantly later than the plan it supports
