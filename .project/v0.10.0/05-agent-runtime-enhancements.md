# PRD 05 — Agent Runtime Enhancements: Middleware, Context Management, Vault Memory, Structured Output, Providers, Prebuilt Agents (Epic `RT`)

**Depends on:** Mostly standalone. §2.1 uses the `NodeInterceptor` seam (ENG-FR-22) when engine-hosted, but the middleware chain also wraps the existing `PaladinExecutionService` independently of the engine, so this epic can start immediately.
**Primary crates:** facade `src/application/services/paladin/`, `paladin-core`, `paladin-ports`, `paladin-llm`, `paladin-memory`.

---

## 1. Problem Statement

Individual agent execution in Paladin is a fixed pipeline: compose prompt → LLM call → tool loop → result. There is no extension chain around the model call, so cross-cutting behaviors — call/cost limits, conversation summarization, guardrails, automatic fallback, planning aids — must be forked into the service. Long-running conversations have no token-budget management (Garrison stores history; nothing trims it). There is no cross-session memory an agent can read/write ("this user prefers metric units") — Garrison is per-conversation and Sanctum is a raw vector port. Agent output is an untyped `String`, so callers who need structured data must parse by hand. Provider coverage is OpenAI/Anthropic/DeepSeek only.

## 2. Functional Requirements

### 2.1 Execution middleware chain

- **RT-FR-01 (Chain).** `PaladinExecutionService` gains an ordered `Vec<Arc<dyn ExecutionMiddleware>>`:

  ```rust
  #[async_trait]
  pub trait ExecutionMiddleware: Send + Sync {
      /// Before each LLM call in the reasoning loop. May mutate the pending prompt/messages,
      /// short-circuit with a final result, or abort with an error.
      async fn before_model(&self, cx: &mut ModelCallContext) -> Result<MiddlewareFlow, PaladinError>;
      /// After each LLM response, before tool handling.
      async fn after_model(&self, cx: &mut ModelCallContext, resp: &mut LlmResponseView) -> Result<MiddlewareFlow, PaladinError>;
      /// Around each tool invocation.
      async fn around_tool(&self, cx: &ToolCallContext) -> Result<ToolFlow, PaladinError>;
      fn name(&self) -> &str;
  }
  pub enum MiddlewareFlow { Continue, Finish(FinalResult), Fail(PaladinError) }
  pub enum ToolFlow { Allow, Deny { reason: String }, Rewrite(ArmamentCall) }
  ```

  `ModelCallContext` exposes: loop index, cumulative token count, mutable message/prompt buffer, per-run KV scratch (`HashMap<String, Value>`), the Paladin config (read-only). Ordering: `before_model` runs first-to-last, `after_model` last-to-first (onion). All hooks have default no-op implementations.
- **RT-FR-02 (Determinism & isolation).** Middleware state is per-run (constructed via a factory or interior per-run scratch); two concurrent runs MUST NOT share mutable middleware state (multi-thread test).
- **RT-FR-03 (Engine bridging).** When a Paladin runs as an engine node, the same chain applies (the engine's `NodeInterceptor` (ENG-FR-22) remains a separate, node-granularity mechanism; document the two layers: NodeInterceptor = around the whole node, ExecutionMiddleware = inside the reasoning loop).

Built-in middleware (each individually testable, each with a config struct per X-09):

- **RT-FR-04 (ModelCallLimit).** Max LLM calls per run (default off). Exceeding → `Finish` with a truncation notice appended and `StopReason::MaxLoops`-analogous new `StopReason::CallLimit` (add the variant; `is_successful()` = true-with-warning semantics, documented). **X-10:** `StopReason` is a pre-existing public enum that callers plausibly match exhaustively. Decide explicitly between `#[non_exhaustive]` (preferred) and a deliberate-breaking register entry with justification; apply the same decision to `TokenBudget` (RT-FR-06) in the same change, and record it in `MIGRATION.md` §9.2.
- **RT-FR-05 (ToolCallLimit).** Max tool invocations per run and optional per-tool caps; breach → `Deny` with a message the model sees ("tool budget exhausted"), not a run failure.
- **RT-FR-06 (TokenBudget).** Max cumulative tokens per run; breach → `Finish` with `StopReason::TokenBudget` (new variant).
- **RT-FR-07 (Guardrail).** Regex/predicate screens on outbound prompts and inbound responses with `on_match: Fail | Redact(replacement) | Finish(message)`. Ships with an empty default rule set; rules are config-supplied.
- **RT-FR-08 (Summarization).** See RT-FR-10..12 (context management) — implemented as middleware.
- **RT-FR-09 (ModelRetry/Fallback as middleware).** Thin adapters over PRD-04's `FallbackLlmAdapter` and retry, so users who don't adopt the engine still get them per-Paladin. No duplicate logic: these MUST delegate to the PRD-04 implementations.

### 2.2 Context-window management

- **RT-FR-10 (Token counting port).** `output::token_counter_port::TokenCounterPort { count(&str, model: &str) -> u32 }` with a heuristic default adapter (chars/4, clearly documented as approximate) and provider-specific adapters where SDKs allow. All budget features consume this port — never inline heuristics.
- **RT-FR-11 (Trimming).** `HistoryTrimmer` middleware: given `max_context_tokens` for the model (config table per model, overridable), trims Garrison-derived history to fit using strategy `KeepSystemAndRecent { reserve_for_response: u32 }` — system prompt always kept, then most-recent-first until budget. Trimming MUST be stable (same inputs → same kept set) and MUST never split a message.
- **RT-FR-12 (Summarization).** `SummarizationMiddleware`: when history exceeds a threshold (token count or message count), replace the oldest N messages with a single summary message generated via a configured (cheap) model through `LlmPort`, and persist the summary to Garrison flagged `is_summary: true` (Garrison entry metadata gains this flag) so re-summarization compounds instead of re-reading raw history. **X-10/§9.4:** the Garrison entry is a pre-existing persisted public type — the new field MUST be `#[serde(default)]` (existing entries deserialize as `false`), the SQLite adapter needs an additive migration (`ALTER TABLE … ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0`) registered in `MIGRATION.md` §9.4, and the struct change is registered in §9.2. Summarization failure degrades to trimming (RT-FR-11), never fails the run; degradation is logged + traced.

### 2.3 Vault — cross-session long-term memory

- **RT-FR-13 (VaultPort).** New port `output::vault_port::VaultPort`:

  ```rust
  pub struct Namespace(pub Vec<String>);     // e.g. ["user", "alice", "prefs"]; each segment non-empty, ≤64 chars, no '/'
  #[async_trait]
  pub trait VaultPort: Send + Sync {
      async fn put(&self, ns: &Namespace, key: &str, value: serde_json::Value) -> Result<(), VaultError>;
      async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError>;
      async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError>;
      async fn list(&self, ns: &Namespace, prefix: Option<&str>, page: Page) -> Result<Vec<VaultRecord>, VaultError>;
      /// Optional semantic search; adapters without embedding support return Unsupported.
      async fn search(&self, ns: &Namespace, query: &str, limit: u32) -> Result<Vec<ScoredVaultRecord>, VaultError>;
  }
  ```

  `VaultRecord { key, value, created_at, updated_at }`. Vault is distinct from Garrison (per-conversation transcript) and from Waypoints (run state); rustdoc MUST state the three-way distinction with a table.
- **RT-FR-14 (Adapters).** InMemory (tests), SQLite (kv table, ns as joined path column, prefix queries indexed), and a semantic adapter composing SQLite storage + the existing Sanctum/Qdrant embedding path for `search` (feature `qdrant`). Shared contract suite across adapters (search asserted only where supported).
- **RT-FR-15 (Agent access).** The Vault is reachable from execution: (a) `NodeContext::vault()` in engine nodes; (b) two built-in Armaments (Arsenal tools) `vault_get` / `vault_put`, registered opt-in, whose namespace is CONFINED to a per-run base namespace injected by the host (an agent can never address outside its granted subtree — traversal attempts are `VaultError::NamespaceDenied`, tested); (c) `ExecutionMiddleware` access via context for automatic memory patterns.
- **RT-FR-16 (Recall middleware, opt-in).** `VaultRecallMiddleware`: before the first model call of a run, `search` the granted namespace with the user input and inject top-k results into the prompt under a clearly delimited "Long-term memory" section (k, score floor configurable). No auto-write middleware ships in this epic (explicit `vault_put` tool only) — automatic memory extraction is out of scope.

### 2.4 Structured output

- **RT-FR-17 (Schema-constrained execution).** New method (default-implemented in terms of `execute`) on the execution service, NOT on `PaladinPort` (avoid breaking implementors; expose via a new small trait `StructuredExecutorPort`):

  ```rust
  async fn execute_structured<T: DeserializeOwned + JsonSchema>(
      &self, paladin: &Paladin, input: &str,
  ) -> Result<Structured<T>, PaladinError>;
  // Structured<T> { value: T, raw: PaladinResult }
  ```

  Mechanism: generate the JSON Schema (via the `schemars` crate — new core-adjacent dependency; place schema generation in the application layer, not `paladin-core`, to keep core lean), append a documented schema-conformance instruction block to the prompt, then parse. Where a provider supports native constrained/JSON output modes, the adapter uses them (`LlmPort` gains an optional `response_format: Option<ResponseFormat>` on its request struct with a default that all existing adapters ignore harmlessly — additive per X-03). **X-10.3:** the request struct is pre-existing and public; adding the field requires it to be `#[non_exhaustive]` with a guaranteed `Default`/builder path (doc-tested), and a §9.2 register entry. **X-11:** `schemars` is a new dependency — verify it builds at MSRV 1.85 before adopting; if its current major does not, pin the newest major that does and note it in §9.3.
- **RT-FR-18 (Repair loop).** On parse failure, up to `max_repair_attempts` (default 1) re-prompts with the parse error and the offending output. Exhaustion → `PaladinError::StructuredOutputInvalid { attempts, last_error, raw_output }` (typed; raw preserved for diagnosis).
- **RT-FR-19 (Engine integration).** `NodeSpec::Paladin` gains optional `output_schema: Option<SchemaRef>`; when set, the node executes structured, and the parsed JSON value (not the raw string) is written to `output_field` — making typed agent-to-agent data flow first-class. `StructuredDirective` parsing (CF-FR-06) MUST reuse this machinery.

### 2.5 Provider breadth

- **RT-FR-20 (OpenAI-compatible generic adapter).** `OpenAiCompatibleAdapter { base_url, api_key, model }` in `paladin-llm` implementing `LlmPort` incl. streaming, usable for any endpoint speaking the de-facto chat-completions wire format (self-hosted inference servers, various hosted providers). Configurable extra headers. This single adapter is the highest-leverage breadth item; test against a local mock HTTP server (mockito, already a dev-dep) covering success, streaming, 429/5xx mapping (per FT-FR-01 classification).
- **RT-FR-21 (Google Gemini adapter).** Native adapter (feature `llm-gemini`): generate + streaming + error mapping + token usage extraction. Same conformance suite as existing adapters (extract the existing adapter tests into a shared conformance macro if not already shared).
- **RT-FR-22 (Local/Ollama).** Ollama's OpenAI-compatible endpoint MUST be verified working through RT-FR-20 (documented recipe + an ignored-by-default integration test gated on an env var), not a separate adapter.

### 2.6 Prebuilt agent constructors & tool ergonomics

- **RT-FR-23 (Tool-loop agent one-liner).** `PaladinBuilder` (or a new `presets` module) gains `reasoning_agent(llm, tools: Vec<Armament>, opts)` producing a fully wired Paladin + execution service with: tool loop enabled, sensible loop/stop defaults, `ToolErrorFeedback` (RT-FR-24) on, structured-output opt-in. One doc-tested example ≤ 15 lines from zero to executed result with a mock.
- **RT-FR-24 (Tool error feedback).** When an Armament invocation fails, the failure MUST be formatted and returned INTO the model context as a tool result ("tool X failed: <sanitized reason>. You may retry with corrected arguments or proceed without it.") rather than failing the run — configurable `tool_error_mode: FeedToModel (default) | FailRun`, with a per-tool override. This default is a user-visible behavioral change for existing tool-loop users and is pre-registered as `MIGRATION.md` M-B-03; if the implementer judges the default too surprising for v0.9 users, flip it to `FailRun` and record the decision there — the requirement is that the choice and its rationale are documented, not which side is chosen. Sanitization strips secrets-looking substrings (documented regex set) before feeding errors to the model.

## 3. Acceptance Criteria

1. Onion-ordering test: three recording middlewares assert exact before/after call order and short-circuit behavior (`Finish` from #2 skips #3's before, still runs #1's after).
2. Limits: call/tool/token budget breaches produce the specified StopReasons/denials without run failure; exact counts via mock.
3. Summarization: 30-message history compresses; compound re-summarization verified; degradation-to-trim on summarizer failure verified; trimming stability test.
4. Vault: contract suite on all adapters; namespace confinement attack test (tool tries `["user","bob"]` from a grant of `["user","alice"]` → denied); recall middleware injects top-k.
5. Structured output: derive-based happy path, repair loop success on attempt 2, typed exhaustion error with raw preserved; engine node writes parsed JSON to state.
6. Provider adapters pass the shared conformance suite; OpenAI-compatible adapter verified against mock server incl. streaming + transience mapping.
7. `reasoning_agent` doc test compiles in CI (doc-examples crate).
8. Coverage per X-02; middleware isolation multi-thread test (RT-FR-02).
9. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 4. Test Plan (TDD ordering)

1. Middleware trait + chain ordering units (no LLM).
2. Each built-in middleware unit-tested with MockLlmAdapter.
3. TokenCounter + trimmer + summarizer.
4. VaultPort contract suite → adapters → confinement → recall middleware.
5. Structured output units (schema gen, parse, repair, error shape) → engine integration.
6. Provider conformance suites.
7. Presets doc tests + tool-error feedback tests.

## 5. Out of Scope

Automatic memory extraction/writing policies (explicit tool only); UI for memory management; embedding-model management beyond existing Sanctum; per-token cost accounting in currency (token counts only).
