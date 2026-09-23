# Phase 26: Agent Runtime Enhancements - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-06
**Phase:** 26-agent-runtime-enhancements
**Mode:** `--auto` — every question below was auto-resolved on the recommended option without
`AskUserQuestion`; the "User's choice" rows record the auto-selection, not a human answer.
**Areas discussed:** Middleware home & prompt buffer (RT-01); StopReason X-10 & limit semantics
(RT-02); Retry/fallback delegation & config (RT-02); Context-window management & GarrisonEntry
(RT-03); Vault placement, confinement & host grant (RT-04); Structured output port shape &
LlmRequest X-10 (RT-05); Provider conformance protocol (RT-06); Presets, M-B-03 inversion &
tool-loop enablement (RT-07); Program bookkeeping

---

## Middleware home & prompt buffer (RT-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Facade `middleware/` module | Trait + built-ins beside `PaladinExecutionService`, mirroring `NodeInterceptor`-beside-engine | ✓ |
| `paladin-ports` | Trait as an extension port so third-party crates can implement it without the facade | |
| Structured `PromptAssembly` | Middleware mutate structured parts; rendered after `before_model`; byte-identical when untouched | ✓ |
| Flat prompt `String` | Middleware edit the rendered string | |
| Per-run scratch on the context | Stateless `Arc<dyn>` middleware; isolation by construction | ✓ |
| `MiddlewareFactory` per run | Fresh instances per run | |
| Post-retry `after_model`, `around_tool` over Arsenal + handoff, `before_model`-only on streams | Hooks placed against the existing retry/breaker and stream path | ✓ |
| Per-attempt hooks + stream-end `after_model` | Hooks inside the buffered retry and after stream assembly | |
| Automatic engine bridging via `PaladinPort` | Documented two-layer table + one engine integration test | ✓ |
| Engine-level middleware registry | A second chain owned by `WarEngine` | |

**User's choice:** `[auto]` recommended defaults (D-01…D-06).
**Notes:** The tree's prompt is a flat string, not a message list — PRD 05's "messages" wording is
flagged ⚠ in D-02. `[auto] Middleware home — Q: "Home crate for ExecutionMiddleware?" → Selected:
"facade middleware module" (recommended default)`. `[auto] Prompt buffer — Q: "What is the mutable
prompt buffer?" → Selected: "structured PromptAssembly" (recommended default)`. `[auto] Isolation —
Q: "Per-run state isolation?" → Selected: "per-run scratch" (recommended default)`. `[auto] Hook
placement — Q: "Where do hooks fire vs retry/breaker/handoff/streaming?" → Selected: "post-retry /
Arsenal+handoff / before_model-only on streams" (recommended default)`. `[auto] Engine bridging —
Q: "How does the chain reach engine nodes?" → Selected: "automatic via PaladinPort" (recommended
default)`.

---

## StopReason X-10 & limit semantics (RT-02)

| Option | Description | Selected |
|--------|-------------|----------|
| `#[non_exhaustive]` + `Y` row | The D-04 house pattern; `_` arms at three first-party mappers | ✓ |
| Deliberate-breaking exception | X-10.2's named `StopReason` exception, no attribute | |
| `is_successful()` true for `CallLimit`/`TokenBudget` | PRD's "true-with-warning"; `is_limit()` true | ✓ |
| `is_successful()` false like `MaxLoops` | Treat limits as non-success | |
| One `AgentRuntimeConfig` with sub-structs | `src/config/agent_runtime.rs`, all off by default, scalar env overrides | ✓ |
| One config file per middleware | Twelve X-09 structs under `src/config/` | |

**User's choice:** `[auto]` recommended defaults (D-07, D-08, D-09, D-10).
**Notes:** `[auto] StopReason — Q: "X-10.2 treatment?" → Selected: "#[non_exhaustive]" (recommended
default)`. `[auto] Limits — Q: "is_successful() for the new variants?" → Selected: "true"
(recommended default)`. `[auto] Config — Q: "X-09 layout?" → Selected: "one grouped
AgentRuntimeConfig" (recommended default)`. Guardrail `Finish` uses `StopReason::Completed`; a
`Guardrail` variant is deferred.

---

## Retry/fallback delegation & config (RT-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Port-shaping middleware | `before_model` sets `llm_override` / `retry_policy`; the single model-call site honors them; `RetryPredicate::admits` shared with the engine | ✓ |
| `around_model` hook | A fourth hook wrapping the call; cannot express fallback | |
| Provider names via `LlmProviderFactory` | `ModelFallbackConfig.providers` resolved at `build_chain`; closes Phase 25's deferred idea | ✓ |
| Code-only fallback chain | No config surface | |

**User's choice:** `[auto]` recommended defaults (D-11, D-12).
**Notes:** `[auto] Delegation — Q: "How do retry/fallback middleware avoid duplicating FT-02/FT-05
logic?" → Selected: "port-shaping" (recommended default)`. `[auto] Fallback config — Q: "Config
shape?" → Selected: "provider names via factory" (recommended default)`. Without middleware the
existing `max_loops.min(10)` / `100ms × 2^n` loop is byte-identical (X-03).

---

## Context-window management & GarrisonEntry (RT-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Sync infallible `TokenCounterPort`; heuristic in `paladin-memory`; tiktoken under `content-processing` | No new dependency, no network | ✓ |
| Async port with provider count endpoints | Anthropic/Gemini endpoints as adapters now | |
| Layered limit resolution | config table → `ProviderCapabilities.max_context_tokens` → default 8192 | ✓ |
| Config-only limit | Operator must set every model | |
| Latest-summary-wins | Summaries as `System` entries with `is_summary`; embedded trimmer for degradation | ✓ |
| Add delete-by-id to `GarrisonPort` | Remove superseded summaries from the store | |
| `#[non_exhaustive]` + `summary()`; embedded `sqlx::migrate!`; root copy removed | X-10.3 option (a); `002_add_garrison_is_summary.sql` | ✓ |
| Deliberate-breaking field; keep the runtime `./migrations` path | X-10.3 option (b) | |

**User's choice:** `[auto]` recommended defaults (D-13…D-17).
**Notes:** `[auto] TokenCounterPort — Q: "Shape and adapters?" → Selected: "sync port + existing
tiktoken" (recommended default)`. `[auto] Limits — Q: "max_context_tokens resolution?" →
Selected: "layered resolution" (recommended default)`. `[auto] Summaries — Q: "Compounding without a
delete API?" → Selected: "latest-summary-wins" (recommended default)`. `[auto] GarrisonEntry —
Q: "X-10.3 option and migration path?" → Selected: "non_exhaustive + embedded migrator"
(recommended default)`. The runtime `./migrations` path (`sqlite_garrison.rs:108`) and the
byte-identical root copy copied by the `Dockerfile` were discovered during scouting.

---

## Vault placement, confinement & host grant (RT-04)

| Option | Description | Selected |
|--------|-------------|----------|
| `paladin-memory` | Memory domain; `sqlite` feature and Sanctum adapters already there | ✓ |
| `paladin-storage` | The Phase 25 node-cache precedent (chosen there for `redis`) | |
| Absolute namespaces + `ConfinedVault` prefix check | The PRD attack test reads literally; `NamespaceDenied` | ✓ |
| Namespaces relative to the grant | Cannot express the PRD test; agent cannot see its grant | |
| `RunScope` + defaulted `PaladinPort::execute_scoped` | Host grant per run; `WarEngine::with_vault(vault, base)`; `NodeContext::vault()` | ✓ |
| Service-level grant only | One service per user | |
| `InProcessArsenal` + `CompositeArsenalPort` | In-process built-in tools; `enable_vault_tools()` | ✓ |
| `McpToolInvoker` shim | Fake an MCP client for built-ins | |
| Skip + warn once on `Unsupported` search | Recall is best-effort | ✓ |
| Fall back to `list()` | Inject keyed records when search is unsupported | |

**User's choice:** `[auto]` recommended defaults (D-18…D-25).
**Notes:** `[auto] Crate — Q: "Adapter crate?" → Selected: "paladin-memory" (recommended default)`.
`[auto] Confinement — Q: "Namespace addressing?" → Selected: "absolute + prefix check"
(recommended default)`. `[auto] Grant — Q: "How does the host inject the grant?" → Selected:
"RunScope + defaulted port method" (recommended default)`. `[auto] Armaments — Q: "How are
vault_get/vault_put registered?" → Selected: "in-process arsenal + composite" (recommended
default)`. `[auto] Recall — Q: "Unsupported search?" → Selected: "skip + warn" (recommended
default)`. `SemanticVault` composes ports and is ungated (ADR-0046).

---

## Structured output port shape & LlmRequest X-10 (RT-05)

| Option | Description | Selected |
|--------|-------------|----------|
| Object-safe JSON-level port + facade generic extension | `schemars` only in the facade; machinery in core; loop driver in ports; `extract_json` shared with `StructuredDirective` | ✓ |
| Generic-only trait | Not object-safe; `schemars` in ports | |
| `LlmRequest` option (a): `#[non_exhaustive]` + `new()` builder | 37 in-tree files migrated; next field free | ✓ |
| `LlmRequest` option (b): deliberate-breaking field | Breaks the same literals with no builder gain | |
| Native modes on OpenAI, compat engine, Gemini, DeepSeek | Anthropic prompt-only; no `ProviderCapabilities` field | ✓ |
| OpenAI + compat engine only | Narrower first cut | |
| `WarEngine::with_structured_executor` + schema registry | FT-06 fail-closed wiring; fingerprint `v6`; exclusive with `StructuredDirective` | ✓ |
| Defaulted `PaladinPort::execute_structured_json` | Zero-config engine use; contradicts X-10.4's own text | |
| In-house shape check | `type`/`required`/`properties`/`enum`/`items`/nullability; full validator deferred | ✓ |
| `jsonschema` crate | Full validation; heavyweight default-set dependency | |

**User's choice:** `[auto]` recommended defaults (D-26…D-30).
**Notes:** `[auto] Port shape — Q: "StructuredExecutorPort shape?" → Selected: "object-safe +
extension" (recommended default)`. `[auto] LlmRequest — Q: "X-10.3 option?" → Selected: "(a)
non_exhaustive + builder" (recommended default)`. `[auto] Native modes — Q: "Which adapters?" →
Selected: "four paths" (recommended default)`. `[auto] Engine — Q: "output_schema wiring?" →
Selected: "with_structured_executor" (recommended default)`. `[auto] Validation — Q: "Untyped
validation?" → Selected: "shape check" (recommended default)`. `schemars 1.2.1` is already in the
lockfile via `rmcp`.

---

## Provider conformance protocol (RT-06)

| Option | Description | Selected |
|--------|-------------|----------|
| Shared `#[cfg(test)]` suite + macro, measure-then-fix | Per-adapter case table recorded before any fix | ✓ |
| Per-adapter ad hoc gap tests | No shared case list | |
| Reuse `ollama_docker_test.rs` + mdBook recipe | The env-probed suite already exists; no second file | ✓ |
| New `#[ignore]` env-gated test | A duplicate Ollama suite | |

**User's choice:** `[auto]` recommended defaults (D-31, D-32).
**Notes:** `[auto] Suite — Q: "Suite shape?" → Selected: "shared macro suite" (recommended
default)`. `[auto] Ollama — Q: "Recipe and env-gated test?" → Selected: "reuse + document"
(recommended default)`. PRD 05 §2.5's greenfield premise is stale (REQUIREMENTS.md scope-time
conflict record) — flagged ⚠ in D-31.

---

## Presets, M-B-03 inversion & tool-loop enablement (RT-07)

| Option | Description | Selected |
|--------|-------------|----------|
| Default `FeedToModel` | The tree's existing behavior; `FailRun` is the new opt-in; M-B-03 rewritten | ✓ |
| Default `FailRun` | Would be a real v0.9→v0.10 behavioral change | |
| Service-level `ToolErrorConfig` | `PaladinConfig` untouched | ✓ |
| `PaladinConfig` field | An X-10.3 event on a pre-existing pub struct | |
| Reuse `paladin_llm::redaction` | New key-less `redact_secret_patterns` | ✓ |
| New regex set in the facade | Duplicate patterns | |
| `reasoning_agent(llm, Arc<dyn ArsenalPort>, opts)` | Executable arsenal; `ReasoningAgent { paladin, service }` | ✓ |
| `tools: Vec<Armament>` (PRD literal) | Definitions cannot execute | |
| `ToolCallProtocol` + `FinishOnPlainAnswer` middleware | Prompt-level protocol; mock and ADR-0042 untouched | ✓ |
| Inert loop + scripted mock only | Preset cannot call a tool through a shipped provider | |

**User's choice:** `[auto]` recommended defaults (D-33…D-36).
**Notes:** `[auto] M-B-03 — Q: "Default tool_error_mode?" → Selected: "FeedToModel" (recommended
default)`. `[auto] Placement — Q: "Where does tool_error_mode live?" → Selected: "service-level
config" (recommended default)`. `[auto] Sanitization — Q: "Regex set?" → Selected: "reuse
redaction.rs" (recommended default)`. `[auto] Preset — Q: "reasoning_agent signature?" → Selected:
"Arc<dyn ArsenalPort>" (recommended default)`. `[auto] Tool loop — Q: "Enablement for shipped
providers?" → Selected: "prompt-level protocol middleware" (recommended default)`. Two ⚠ flags for
plan review: the PRD/M-B-03 premise inversion (D-33) and the scope interpretation (D-36).

---

## Program bookkeeping

| Option | Description | Selected |
|--------|-------------|----------|
| Full gate list incl. `check-api-surface.sh` | Phase 25 carried concern honored | ✓ |
| Prior gate list only | Leaves the `api-surface` job unguarded | |

**User's choice:** `[auto]` recommended default (D-37…D-41).
**Notes:** `[auto] Gates — Q: "Gate list?" → Selected: "full gate list incl. api-surface"
(recommended default)`.

## Claude's Discretion

See CONTEXT.md "Claude's Discretion": context/assembly field sets, type names, whether
`paladin-server` calls `build_chain`, conformance-suite breadth, the root `migrations/` copy's
disposition, cursor encoding, the Sanctum namespace filter field, prompt/notice wordings,
`BATTLEFIELD_SCHEMA_VERSION`, plan/wave decomposition.

## Deferred Ideas

See CONTEXT.md "Deferred Ideas": streaming response screens; LLM-native tool calling (ADR-0042);
`StopReason::Guardrail`; a `ProviderCapabilities` structured-output flag; full JSON Schema
validation; `output_schema` + `StructuredDirective` combination; provider token-count endpoints;
the RAG `/4` heuristic; a Garrison delete-by-id; `NodeInterceptor` visibility of `NextStep`; trace
events for the new seams (Phase 28); `RunScope` from HTTP (Phase 27); FUT-06/08/09; per-hook
timeouts; config-registered schemas.
