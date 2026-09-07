# Agent Runtime: Middleware, Context Management, Vault Memory, Structured Output and the Reasoning Agent

An ordered `ExecutionMiddleware` chain wrapping `PaladinExecutionService`'s reasoning loop, the
built-in policies that ride it (call/token/tool budgets, guardrails, retry/fallback,
summarization, Vault recall), cross-session Vault memory, typed structured output, and a
`reasoning_agent` preset that assembles all of it into a runnable agent — built as an opt-in layer
over the execution service the [Paladin Agents](paladin-agents.md) guide introduces.

> Every Rust sample on this page is compiled code pulled from the `paladin-doc-examples` crate via
> mdBook `{{#include}}`, so a sample cannot drift from the landed API.

---

## Table of Contents

1. [The Two-Layer Contract: `NodeInterceptor` vs `ExecutionMiddleware`](#the-two-layer-contract-nodeinterceptor-vs-executionmiddleware)
2. [Installing a Chain](#installing-a-chain)
3. [Built-in Middleware](#built-in-middleware)
4. [Vault: Cross-Session Memory](#vault-cross-session-memory)
5. [Structured Output](#structured-output)
6. [Native `response_format` by Provider](#native-response_format-by-provider)
7. [The Tool-Call Protocol](#the-tool-call-protocol)
8. [The `reasoning_agent` Preset](#the-reasoning_agent-preset)
9. [The Ollama Recipe](#the-ollama-recipe)
10. [Security Notes](#security-notes)

---

## The Two-Layer Contract: `NodeInterceptor` vs `ExecutionMiddleware`

Paladin has two, deliberately independent, hook layers. Confusing them is the most common mistake
when adding cross-cutting behavior to a Paladin — use this table before reaching for either:

| | `NodeInterceptor` | `ExecutionMiddleware` |
|---|---|---|
| **Scope** | The whole node, as the `WarEngine` sees it | Inside one Paladin's own reasoning loop |
| **Frequency** | Once per Aegis attempt | Once per model call (`before_model`/`after_model`), once per tool/handoff dispatch (`around_tool`) |
| **Owning crate** | `paladin-battalion` (`engine::hooks`) | the facade, beside `PaladinExecutionService` (`paladin::application::services::paladin::middleware`) |
| **What it can do** | `Skip`/`Fail` a node before it runs; observe or mutate its resulting delta | Rewrite the prompt assembly, cap loops/tokens/tool calls, screen prompts/responses, override the model port or retry policy, recall Vault memory, finish the run early |
| **Registered on** | the `WarEngine` (`with_interceptor`) | the `PaladinExecutionService` (`with_middleware`) |

The two never share a registry and neither wraps the other's decision as if it were the node's own
fault: the interceptor decides whether the node runs at all; the middleware chain runs *inside* the
node's own execution, once the interceptor has already let it proceed. When a `WarEngine`
dispatches a `NodeSpec::Paladin` node it always calls `PaladinPort::execute_observed`
(`superstep.rs:946`) — a `PaladinExecutionService` carrying middleware applies its chain
automatically as a node, with **no engine-side change and no second registry**. See
`paladin-battalion`'s `NodeInterceptor` and `paladin-ports`'s `PaladinPort` rustdoc for the same
table restated on the two traits themselves.

## Installing a Chain

Middleware is stateless and `Arc`-shared; per-run mutable state (counters, scratch data) lives on
the `ModelCallContext`/`ToolCallContext` the chain receives, never on the middleware value itself
— so one `Arc<dyn ExecutionMiddleware>` is safe to reuse across concurrent runs.

```rust,ignore
service
    .with_middleware(Arc::new(ModelCallLimit::new(10)))
    .with_middleware(Arc::new(TokenBudget::new(50_000)));

// Or replace the whole chain at once:
service.with_middleware_chain(vec![limit, guardrail, recall]);
```

Every built-in is **inert by default** — a service with no middleware installed, or an
`AgentRuntimeConfig` with every section `enabled: false` (the default), behaves byte-identically to
v0.9: the rendered prompt bytes, the port call count and the `PaladinResult` are unchanged. A single
grouped `AgentRuntimeConfig::build_chain(&self, deps)` assembles every enabled section in a fixed
order (limits → guardrail → trimmer/summarizer → recall → protocol → resilience) from one
`config.yml` block, so an operator never has to hand-assemble the chain in code.

## Built-in Middleware

| Built-in | Effect |
|---|---|
| `ModelCallLimit` | Finishes with `StopReason::CallLimit` once the reasoning loop's model-call count (post-retry) reaches the configured max — the accumulated output is kept, with a truncation notice appended |
| `TokenBudget` | Finishes with `StopReason::TokenBudget` once cumulative `total_tokens` crosses the budget; the response that crossed it is kept (at most one response of overshoot) |
| `ToolCallLimit` | Denies a tool or handoff call past a global or per-tool cap through `ToolFlow::Deny` — **never fails the run** |
| `Guardrail` | Screens the rendered prompt (`before_model`) and/or the model's response (`after_model`) against `Regex`/`Predicate` rules, each with `Fail` / `Redact(replacement)` / `Finish(message)` |
| `ModelRetryMiddleware` | Sets a per-run `RetryPolicy` (Phase 25's `paladin_core::platform::container::aegis::RetryPolicy`) that the model-call site honors, without duplicating the backoff math |
| `ModelFallbackMiddleware` | Wraps an ordered `Vec<Arc<dyn LlmPort>>` in one `FallbackLlmAdapter` and installs it as the call's port override |
| `HistoryTrimmer` | Stable, never-splits-a-message trimming: an entry is kept whole or dropped whole, newest-first, until the resolved context-token budget is respected |
| `SummarizationMiddleware` | Compounds a running summary into Garrison (`GarrisonEntry::summary`, `is_summary: true`) once history exceeds a token/message threshold; degrades to `HistoryTrimmer` on summarizer failure — never fails the run |
| `VaultRecallMiddleware` | Best-effort: injects the run's top-K Vault search results into the prompt assembly on loop 1, framed as stored notes rather than instructions |
| `ToolCallProtocolMiddleware` / `FinishOnPlainAnswerMiddleware` | The prompt-level tool-call protocol (below) |

`StopReason` is `#[non_exhaustive]`; `CallLimit` and `TokenBudget` both report
`is_successful() == true` (the run ended with the model's last answer intact) and
`is_limit() == true`. `Guardrail` failures surface as the typed, structured
`PaladinError::GuardrailTripped { rule, target }`.

Every built-in's configuration lives under one `AgentRuntimeConfig` (`src/config/agent_runtime.rs`)
— every section `enabled: false` by default, every scalar field overridable through
`APP_AGENT_RUNTIME_<SECTION>_<FIELD>`. A v0.9 `config.yml` with no `agent_runtime:` section boots
identically to before.

## Vault: Cross-Session Memory

Paladin ships three distinct "memory" concepts, and it is easy to reach for the wrong one:

| | Vault | Garrison | Waypoint |
|---|---|---|---|
| **Scope** | cross-thread namespaced key/value | one conversation's transcript | one run's durable engine state |
| **Lifetime** | until explicitly deleted | the conversation | the retention policy |
| **Addressed by** | `Namespace` + key | a `GarrisonPort` instance | `(ThreadId, WaypointId)` |
| **Who writes** | the host, or the agent through a confined tool | the execution service | the `WarEngine` |
| **Typical content** | durable facts about a user or a domain | conversation turns | Battlefield snapshots and Frontier state |

Reach for the Vault when a fact needs to survive past the end of the conversation or run that
produced it and be readable from a different thread later (the user's preferred language, set once
and read by every future conversation). Reach for the Garrison for the transcript of the current
conversation. Reach for a Waypoint only if you are the engine itself checkpointing execution state.

**Adapters:** `InMemoryVault` (always available), `SqliteVault` (feature `sqlite`), `SemanticVault`
(composes an existing `SanctumPort` + `EmbeddingPort` — ungated, since it holds only trait objects).

**Confinement is structural, not a convention.** The host grants a namespace subtree through a
`RunScope` (`WarEngine::with_vault(vault, base)` for every engine node, or
`PaladinExecutionService::execute_scoped` for a direct call); the `vault_get`/`vault_put` Armaments
take an **absolute** namespace argument, and `ConfinedVault` rejects any call whose namespace does
not have the grant as a prefix with `VaultError::NamespaceDenied` — **before** the inner store is
ever touched. A run with no grant gets no Vault tools listed at all, never a fallback to the root
namespace. `Namespace::is_prefix_of` compares path segments, not raw strings, so a namespace
`user/alice2` is never treated as a descendant of `user/alice`.

Vault values are JSON, size-bounded (`max_value_bytes`, default 64 KiB) and returned to the model
as **data recalled from storage, not instructions** — the same framing this page's
[Security Notes](#security-notes) apply to every other model-facing string on this page.

## Structured Output

`execute_structured<T: DeserializeOwned + JsonSchema>` runs through a new `StructuredExecutorPort`
(deliberately **not** `PaladinPort` — a structured run is a distinct execution shape, not an
`execute` variant). The schema comes from `schemars::schema_for!` at the application layer; the
value comes from `serde_json::from_value` — serde's own deserialization **is** the typed
validation. `Structured<T> { value, raw: PaladinResult }` preserves the underlying `PaladinResult`
alongside the typed value.

A bounded repair loop drives every structured run: attempt 1 appends a documented instruction block
to the input (and, where the provider supports it, also sets a native `response_format` —
belt-and-braces, so correctness never depends on the native mode); a parse or shape failure
re-prompts, up to `max_repair_attempts` (default 1), with the parse error and the offending output;
exhaustion raises the structured, typed `PaladinError::StructuredOutputInvalid { attempts,
last_error, raw_output }`, preserving the raw text for inspection rather than discarding it.

On a `WarEngine`, a `NodeSpec::Paladin` with `output_schema` set writes the **parsed JSON value** to
its `output_field` through the same driver; a node declaring `output_schema` on an engine with no
`with_structured_executor` (or naming an unregistered `Registered` schema) is a typed
`EngineError` at graph validation, before any node runs.

**The `ExecutionMiddleware` chain does not run on this path** (WR-02, `26-REVIEW.md`):
`execute_structured`/`execute_json_schema` dispatch directly, bypassing `run_before`/`run_after`/
`run_around_tool` entirely, so `Guardrail`, `VaultRecallMiddleware`, `ToolCallLimit`,
`TokenBudget`/`ModelCallLimit`, and any custom middleware installed on the same
`PaladinExecutionService` are silently inert for a structured-output call — this is intentional
(the bounded repair loop is not the multi-loop reasoning loop `before_model`/`after_model` model),
but it means a Guardrail rule or a token budget installed for `execute()` gives no protection on
`execute_structured()`/`execute_json_schema()`.

## Native `response_format` by Provider

| Provider | Native mode | Wire shape |
|---|---|---|
| OpenAI | yes | `response_format: { type: "json_object" }` or `{ type: "json_schema", json_schema: {...} }` in the chat-completions body |
| OpenAI-compatible engine (Kimi / Qwen / Grok / Ollama / generic) | yes | same chat-completions `response_format` field |
| DeepSeek | yes (JSON-object only) | `response_format: { type: "json_object" }` |
| Gemini | yes | `generationConfig.responseMimeType: "application/json"` plus `responseSchema` for the schema case |
| Anthropic | **none** | ignored — relies entirely on the prompt-level instruction block described above |

An absent `response_format` leaves every request body byte-identical to before this phase; no
`ProviderCapabilities` field was added for it (a structured-output capability flag is a deferred
idea, tracked separately from tool-calling capability).

## The Tool-Call Protocol

No shipped `LlmPort` adapter ever populates `LlmResponse.function_call` (ADR-0042's deferred,
wire-level tool calling — unchanged by this phase). `ToolCallProtocolMiddleware` makes the
reasoning loop's tool branch reachable for a shipped provider anyway, at the **prompt level**:

- `before_model` renders the arsenal's `list_armaments()` (name, description, parameter schema)
  plus the call-format instructions into a `## Tools` prompt section.
- `after_model`, when the response carries no real `function_call`, runs the same JSON extraction
  the structured-output machinery uses over the response text. If it yields the documented
  envelope `{"tool": "<name>", "arguments": {...}}` naming a tool the arsenal actually has,
  `after_model` synthesizes a `function_call` so the reasoning loop's existing tool-dispatch branch
  fires unchanged. An unknown tool name in the envelope is never synthesized into a call.
- `FinishOnPlainAnswerMiddleware` finishes the run with `StopReason::Completed` as soon as a
  response carries no tool call — without it, the loop runs to `max_loops` and returns
  `StopReason::MaxLoops` even after answering, which is the wrong default for a tool-using agent.

Both middlewares are **opt-in** (installed by the `reasoning_agent` preset, not by default), and
this is a prompt-level protocol, not a wire-level one: no `LlmRequest.tools` field exists, no
adapter changed, and every adapter's tool-calling capability flag stays exactly as it was.
ADR-0042's native, wire-level tool calling remains deferred with its trigger unchanged.

## The `reasoning_agent` Preset

`paladin::presets::reasoning_agent(llm, arsenal, opts)` assembles an `LlmPort`, an executable
`Arc<dyn ArsenalPort>` and a `ReasoningAgentOptions` into a runnable `ReasoningAgent` — a thin
wrapper exposing `run(&self, input) -> Result<PaladinResult, PaladinError>` (and
`run_structured::<T>`). The `arsenal` argument must be **executable**, not just a list of tool
definitions — build one with `InProcessArsenal` (an in-process closure-backed `ArsenalPort`), an
MCP-backed `ArsenalExecutionService`, or a `CompositeArsenalPort` combining several.

```rust,ignore
{{#include ../../../crates/doc-examples/src/agent_runtime.rs:reasoning_agent}}
```

By default, a failed tool call is fed back into the model's context as sanitized text and the loop
continues (`tool_error_mode: FeedToModel`) — this is v0.9's existing behavior, named rather than
changed; `FailRun` is the new opt-in that raises a structured `PaladinError::ArmamentFailed`
instead. Every reason fed back to the model is redacted (bearer tokens, provider API-key shapes,
`key=`/`token=` query values, JWT-shaped triples) **before** it is bounded to an excerpt length —
redact, then bound, never the other way around.

## The Ollama Recipe

Ollama is verified through the same OpenAI-compatible path every other compat-engine provider
uses, not as a separate adapter — see the [Configuration guide](../getting-started/configuration.md)
for the `config.yml` block, `OLLAMA_BASE_URL`, and running the env-probed integration suite:

```bash
cargo test --test ollama_docker --features integration-tests,llm-ollama
```

## Security Notes

- **Recalled Vault content and fed-back tool errors are data, not instructions.** Both are wrapped
  in a delimited section that states this explicitly; treat anything an agent `vault_put`s the same
  way you would treat any other provider-influenced or user-influenced text — never a place to
  carry a credential, and never templated into a prompt as if it were trusted.
- **Redact, then bound, everywhere a model-facing or error-facing string is built.** The order is
  load-bearing: bounding first can slice a secret across the truncation boundary and leak the
  surviving tail.
- **`Guardrail` regex patterns compile once, at construction, under an explicit size bound** — the
  `regex` crate is linear-time, so there is no ReDoS surface, and an oversized or invalid pattern is
  a typed construction error rather than a runtime surprise.
- **No secret-shaped field exists in `AgentRuntimeConfig`.** `ModelFallbackConfig` names providers
  by string; credentials still come from the existing provider-factory env/config path, never from
  this struct.
- **A hanging `ExecutionMiddleware` hook is bounded only by the node's Aegis `run_timeout` or the
  service's per-run timeout** — there is no per-hook timeout yet. This is a stated, accepted
  limitation (alongside R-23-01, the hanging `EdgeConditionEvaluator`), not a gap this page hides.
</content>
