# Phase 34 Documentation Currency Audit — The Single Canonical Inventory (D-01)

**The single D-01 record.** One artifact, seven sections in the order below, appended to by
plans 34-02 through 34-09 as each compiles its assigned section or sweeps its assigned files.
No other file in this phase carries a currency verdict, a rustdoc finding row, an examples row,
or a Phase 35/36 work-list item — `34-EVIDENCE.md` holds only the verbatim command captures this
file's rows cite by evidence anchor (D-02), and `deferred-items.md` holds only the non-doc,
non-example findings this file's §7 points into (D-19).

**Which plan appends which section:**
- §1 Shipped-surface checklist — plan 34-02
- §2 mdBook verdict table (rows) — plans 34-03, 34-04, 34-05
- §3 Rustdoc findings table (rows) — plans 34-06, 34-07
- §4 Examples table (rows) — plan 34-08
- §5, §6, §7 (work lists, deferred routing) — plan 34-09

## Method (read before adding or trusting a row)

A verdict is settled by **content, never by file existence or modification time** (D-00b,
Phase 16 `16-DOCS-01-VERDICTS.md` Method, restated here verbatim in spirit). "File exists" and
"file was touched recently" prove nothing about whether a page still describes the shipped tree.
A row's Verdict is `current` only if every applicable signal class was actually run against that
page and found to match; `stale` or `missing` only after the same run found a mismatch, and only
when the row's Findings cell names the phase and shipped item the page fails to describe. A
**settled verdict with an empty findings cell is invalid** — `34-check.sh` assertion (c) makes
this mechanical. A seeded row's Verdict is `pending` and its Findings cell reads exactly the
fixed placeholder text — `pending`, an em dash, then the six words "not", "yet", "swept",
"(no", "signal", "class run)" in that order — this placeholder is the
`16-DOCS-01-VERDICTS.md` concurrency rule made mechanical: an unswept page must never be
indistinguishable from a checked one. **Deviation note (Rule 1, plan 34-05):** this Method
prose previously quoted the placeholder contiguously, which collided with plan 34-05's own
Task 2 `<verify>` block (a literal single-quoted `grep -q` over the six-word phrase above,
followed by `&& exit 1`, intended to catch a still-unswept §2 row, not this explanatory
sentence) — reworded here, with zero change to the placeholder's actual seed-time text or to
any row, the same class of literal-string-collision fix every prior plan in this phase applied
to `MB-nn` ID cross-references.

**Evidence — nine signal classes** (D-07): the Phase 16 eight (version strings, dependency pins,
crate names, module/source paths, `make` targets, workflow/job names, error types, feature
flags), each with its own producing command, run per page by `34-signals.sh <path>`; plus a
ninth, the shipped-surface checklist hit (D-08), which degrades to an explicit `SKIPPED` line
until plan 34-02 writes `34-shipped-tokens.txt`. Every Findings cell below names the command
actually run, never a copy of this list.

**ID scheme (D-03):** every work-list item carries a stable ID — `MB-nn` (mdBook → Phase 35),
`RD-nn` (rustdoc → Phase 36), `EX-nn` (examples → Phase 36) — numbered in table order and never
renumbered, because Phases 35 and 36 close items by ID (their SUMMARY/VERIFICATION cite the row's
own ID against the commit that closed it). The first mdBook row worked in §2 below fixes the
numbering origin.

**Sizing rubric (D-04, verbatim):**
- **S** — a one-line or one-link fix on an existing page, a single rustdoc link repair, an
  example that needs one rename.
- **M** — a section rewrite or a new section on an existing page, a rustdoc block on an
  undocumented item family, an example needing a signature-level update.
- **L** — a new page, a new example, or a rewrite of more than half of an existing page or
  example.

**Recorded choice — spec-less probe fallback skipped this run:** this phase has no `SPEC.md`, and
no `CURR-*` requirement IDs existed at plan time (plan 34-01 mints them). The phase's `must_haves`
truths were therefore derived directly from the five ROADMAP success criteria and the CONTEXT.md
decisions, not from probe-derived predicates — the usual spec-less-probe fallback path was not
exercised because there was no spec to probe against in the first place.

**Recorded choice — D-22 reversibility, no blocking human checkpoint inserted:** CONTEXT.md rates
D-22 (read-only enforcement) one-way, and every plan in this phase carries that rating on the
tasks that implement it, but no plan inserts a blocking human checkpoint for it. The run is
unattended, and D-22's enforcement is itself a mechanical recorded `git diff` that every task's
`<verify>` re-runs before any commit — a human gate that cannot itself answer "did this diff touch
a file outside `.planning/`" any more definitively than the diff already does would stall the
audit without adding protection the diff does not already give.

## Measurement Header

**HEAD SHA measured:** `ee1fb160f8e743e638b32beb6c4e32be4ede9325`
**Date:** 2026-09-17
**Branch:** `feature/phase-33`

**D-23 invariance argument:** every Phase 34 commit touches only `.planning/` (SC5, proven per
commit by the git diff in `34-check.sh` assertion (d) — see the deviation note there for why the
base reference is the Phase 34 start SHA above, not `main`). Because the source tree every row in
this file measures is therefore identical at every Phase 34 commit, a later plan's
`git rev-parse HEAD` differing from the SHA above does not invalidate any row measured at this
SHA. If the branch moves for any other reason (a rebase, a maintainer commit on the same branch),
Phases 35/36 re-run the D-12/D-16 commands and diff against the rows recorded here rather than
trusting the counts unchanged.

**Toolchain versions (verbatim), each annotated with the pin it is compared against:**

```
$ cargo --version
cargo 1.97.1 (c980f4866 2026-06-30)
$ rustc --version
rustc 1.97.1 (8bab26f4f 2026-07-14)
```
Compared against: `rust-toolchain.toml` `[toolchain] channel = "1.97.1"` — **matches exactly**.

```
$ mdbook --version
mdbook v0.4.40
$ mdbook-linkcheck --version
mdbook-linkcheck 0.7.7
$ mdbook-mermaid --version
mdbook-mermaid 0.13.0
```
Compared against: `.github/workflows/docs.yml` — `cargo install mdbook --version 0.4.40 --locked`
(line 46), `cargo install mdbook-mermaid --version 0.13.0 --locked` (line 50),
`cargo install mdbook-linkcheck --version 0.7.7 --locked` (line 54) — **all three match exactly**.

**Toolchain-drift question (D-12) — closed, not open:** `rust-toolchain.toml`'s own header states
it "overrides whatever toolchain a workflow action installed," and the `lint` job's
`dtolnay/rust-toolchain@stable` step is exactly such an override target — both this devcontainer
and CI's lint job run `cargo`/`rustc` 1.97.1. Recorded here as the RESEARCH.md finding (Finding
F-12), not left open.

**Ubiquitous-language list identification (D-10):** the "three ubiquitous-language lists" Phase 35
SC4 names are taken to be the naming table in `.github/copilot-instructions.md`, the term table in
`.planning/PROJECT.md`, and `docs/src/architecture/domain-model.md` — confirmed by direct content
match (each contains a `Commissary` row/entry) per 34-RESEARCH.md Assumption A1.

## §1 Shipped-surface checklist

Compiled by plan 34-02, in D-08 precedence order: (1)
`git diff v0.9.0..HEAD -- .project/current-exports.txt` — cross-check only, never read
prose-style (see the note after the tables below); (2) `CHANGELOG.md [0.10.0]`; (3)
`MIGRATION.md` §9.1-§9.8; (4) `.planning/REQUIREMENTS.md`'s v0.10.0 capability list. In practice
every row's Source below is CHANGELOG or MIGRATION, confirmed by the exports-diff cross-check note
after the tables — matching the rule the Method statement above already states. Every §2/§3/§4
verdict cites into this checklist by copying a row, per this phase's own `success_criteria`.

#### Phase 22 — Battlefield State & Superstep Engine

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-01 | `WarEngine` executes cyclic graphs in supersteps, self-loops included | type | CHANGELOG [0.10.0] Behavioral changes | ENG-02 | WarEngine |
| SS-02 | `Waypoint` — a full `Battlefield` snapshot persisted automatically after every superstep | type | MIGRATION §9.2 | ENG-03 | Waypoint |
| SS-03 | `WaypointPort` — three backends (InMemory, SQLite with migrations, Postgres) | type | REQUIREMENTS ENG-05 | ENG-05 | WaypointPort |
| SS-04 | `waypoints` table — new per-backend persistence table for checkpoint snapshots | migration | MIGRATION §9.4 | ENG-03 | waypoints |
| SS-05 | `EngineConfig` (`max_supersteps`, `max_node_visits`, `run_timeout_secs`, `waypoint_durability`, `max_muster_tasks`) | config key | MIGRATION §9.5 | ENG-02 | EngineConfig |
| SS-06 | `APP_ENGINE_MAX_SUPERSTEPS` environment override (default 50) | env var | MIGRATION §9.5 | ENG-02 | APP_ENGINE_MAX_SUPERSTEPS |
| SS-07 | `WaypointRetentionService`/`WaypointRetentionConfig` — public application-layer pruning service | type | CHANGELOG [0.10.0] Added | ENG-05 | WaypointRetentionService |

#### Phase 22.1 — Engine readiness defect and MSRV follow-up (INSERTED)

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-08 | Workspace MSRV floor raised from 1.85 to 1.88 (X-11.2 stop-and-flag resolution, measured against `rmcp`/`process-wrap`/`time` chain) | dependency | CHANGELOG [0.10.0] Changed | — | rust-version = "1.88" |
| SS-09 | `[workspace] resolver = "3"` — recurrence guard against a future silent re-resolution above the MSRV floor | dependency | MIGRATION §9.3 | — | resolver = "3" |
| SS-10 | Graph fingerprint `v1:` → `v2:` bump — closes a delimiter-collision hash weakness (22-REVIEW CR-01) | migration | MIGRATION §9.4 | — | GraphFingerprint |

#### Phase 23 — Control Flow — Dynamic Routing, Fan-Out & Subgraphs

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-11 | `EdgeCondition::Custom` fails closed instead of always-routing when unregistered (M-B-01, BUG-01 fix) | behavioral change | CHANGELOG [0.10.0] Behavioral changes | CF-01 | EdgeCondition::Custom |
| SS-12 | Node-driven `Directive` routing (`NextStep::{Edges, Goto, Muster, End, Parley}`) | type | CHANGELOG [0.10.0] Added | CF-02 | Directive |
| SS-13 | Muster dynamic worker fan-out (`NextStep::Muster`, `WarGraph::add_worker_template`) | type | CHANGELOG [0.10.0] Added | CF-03 | Muster |
| SS-14 | `NodeSpec::Battalion` — nested subgraph composition via a declared `StateMap` | type | CHANGELOG [0.10.0] Added | CF-04 | NodeSpec::Battalion |
| SS-15 | `LlmDecisionEvaluator` + Commander `StrategySelection::Semantic`, both off by default | type | CHANGELOG [0.10.0] Added | CF-05 | LlmDecisionEvaluator |
| SS-16 | `APP_ENGINE_MAX_MUSTER_TASKS` environment override (default 100) | env var | MIGRATION §9.5 | CF-03 | APP_ENGINE_MAX_MUSTER_TASKS |

#### Phase 24 — Pause/Resume, History & Graceful Shutdown

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-17 | `NodeSpec::Gate` — first-class approval-gate node rendering prompt/payload from the Battlefield | type | CHANGELOG [0.10.0] Added | HITL-01 | NodeSpec::Gate |
| SS-18 | `WarEngine::resume_with(graph, thread, responses)` — typed, total-validation resume | type | CHANGELOG [0.10.0] Added | HITL-02 | resume_with |
| SS-19 | `ChronicleService` (`history`/`inspect`/`latest_on_branch`) + `WarEngine::replay`/`fork` | type | CHANGELOG [0.10.0] Added | HITL-03 | ChronicleService |
| SS-20 | Graceful shutdown on SIGTERM/SIGINT via `ShutdownCoordinator` (M-B-02) | behavioral change | CHANGELOG [0.10.0] Behavioral changes | HITL-04 | ShutdownCoordinator |
| SS-21 | `APP_ENGINE_SHUTDOWN_GRACE_SECS` environment override (default 30) | env var | MIGRATION §9.1 | HITL-04 | APP_ENGINE_SHUTDOWN_GRACE_SECS |
| SS-22 | `APP_ENGINE_GRACEFUL_SHUTDOWN` environment override (default true) | env var | MIGRATION §9.1 | HITL-04 | APP_ENGINE_GRACEFUL_SHUTDOWN |
| SS-23 | `GET /v1/threads/{id}/state` route | route | MIGRATION §9.6 | HITL-05 | GET /v1/threads/{id}/state |
| SS-24 | `POST /v1/threads/{id}/resume` route (202, background continuation) | route | MIGRATION §9.6 | HITL-05 | POST /v1/threads/{id}/resume |
| SS-25 | `GET /v1/threads/{id}/history` route (paginated) | route | MIGRATION §9.6 | HITL-05 | GET /v1/threads/{id}/history |
| SS-26 | Graph fingerprint `v3:` → `v4:` bump — `Gate` node routing properties now hashed | migration | CHANGELOG [0.10.0] Changed | HITL-01 | GRAPH_FINGERPRINT_VERSION |

#### Phase 25 — Node-Level Fault Tolerance

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-27 | `Transience { Transient, Permanent, Unknown }` typed error taxonomy | type | CHANGELOG [0.10.0] Added | FT-01 | Transience |
| SS-28 | `Aegis { retry, timeout, on_error, cache }` — per-node policy sidecar | type | CHANGELOG [0.10.0] Added | FT-02 | Aegis |
| SS-29 | `TimeoutPolicy { run_timeout, idle_timeout }` + `HeartbeatHandle` progress channel | type | CHANGELOG [0.10.0] Added | FT-03 | TimeoutPolicy |
| SS-30 | `ErrorHandlerSpec::{Route, Absorb, Custom}` typed compensation handlers | type | CHANGELOG [0.10.0] Added | FT-04 | ErrorHandlerSpec |
| SS-31 | `FallbackLlmAdapter` — ordered `LlmPort` chain with Transient/Unknown-only hops | type | CHANGELOG [0.10.0] Added | FT-05 | FallbackLlmAdapter |
| SS-32 | `CachePolicy { ttl, key }` node-result caching via the new `NodeCachePort` | type | CHANGELOG [0.10.0] Added | FT-06 | CachePolicy |
| SS-33 | `redis-cache` Cargo feature on `paladin-storage` (never in a default set) | feature flag | MIGRATION §9.3 | FT-06 | redis-cache |
| SS-34 | `APP_NODE_CACHE_ENABLED` environment override (default false) | env var | MIGRATION §9.5 | FT-06 | APP_NODE_CACHE_ENABLED |

#### Phase 26 — Agent Runtime Enhancements

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-35 | `ExecutionMiddleware` trait (`before_model`/`after_model`/`around_tool`, onion-ordered) | type | CHANGELOG [0.10.0] Changed | RT-01 | ExecutionMiddleware |
| SS-36 | `AgentRuntimeConfig` — one grouped config carrying twelve built-in middleware sub-structs | config key | MIGRATION §9.5 | RT-02 | AgentRuntimeConfig |
| SS-37 | `TokenCounterPort` — synchronous, infallible token-counting contract | type | MIGRATION §9.2 | RT-03 | TokenCounterPort |
| SS-38 | `HistoryTrimmer` + `SummarizationMiddleware` context-window management | type | CHANGELOG [0.10.0] Added | RT-03 | HistoryTrimmer |
| SS-39 | `VaultPort` (put/get/delete/list/search) + structural `ConfinedVault` namespacing | type | CHANGELOG [0.10.0] Added | RT-04 | VaultPort |
| SS-40 | `StructuredExecutorPort` / `execute_structured<T>` schema-validated output | type | CHANGELOG [0.10.0] Added | RT-05 | StructuredExecutorPort |
| SS-41 | `reasoning_agent(llm, arsenal, opts)` one-line preset | type | CHANGELOG [0.10.0] Added | RT-07 | reasoning_agent |
| SS-42 | `tool_error_mode = FailRun` opt-in + redact-then-bound sanitization of fed-back tool text (M-B-03) | behavioral change | CHANGELOG [0.10.0] Behavioral changes | RT-07 | tool_error_mode |
| SS-43 | `schemars = "1.2"` — new direct facade dependency for schema derivation | dependency | MIGRATION §9.3 | RT-05 | schemars |

#### Phase 27 — Platform API

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-44 | `POST /v1/runs` route (202, decoupled submission) | route | MIGRATION §9.6 | PLAT-01 | POST /v1/runs |
| SS-45 | `GET /v1/runs/{run_id}/stream` route (SSE, seven frozen wire events) | route | MIGRATION §9.6 | PLAT-03 | GET /v1/runs/{run_id}/stream |
| SS-46 | `POST /v1/runs/{run_id}/cancel` route | route | MIGRATION §9.6 | PLAT-02 | POST /v1/runs/{run_id}/cancel |
| SS-47 | `/v1/assistants*` — seven routes, append-only immutable versions | route | MIGRATION §9.6 | PLAT-04 | /v1/assistants |
| SS-48 | `/v1/schedules*` — five routes, cron-driven recurring run submission | route | MIGRATION §9.6 | PLAT-05 | /v1/schedules |
| SS-49 | `webhook_deliveries` table + `X-Paladin-Signature` HMAC delivery | type | MIGRATION §9.4 | PLAT-05 | webhook_deliveries |
| SS-50 | `APP_WEBHOOKS_ALLOW_PRIVATE` — the only SSRF-guard override | env var | MIGRATION §9.5 | PLAT-05 | APP_WEBHOOKS_ALLOW_PRIVATE |
| SS-51 | `RunQueuePort` (InMemory + Redis) durable worker-pool dispatch | type | REQUIREMENTS PLAT-02 | PLAT-02 | RunQueuePort |
| SS-52 | `APP_RUN_STORE_BACKEND` (disabled, sqlite or postgres) | env var | MIGRATION §9.5 | PLAT-01 | APP_RUN_STORE_BACKEND |

#### Phase 28 — Observability & Tooling

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-53 | `TraceRecord` envelope — twelve `TraceEvent` variants, moved to `paladin-core` | type | MIGRATION §9.2 | OBS-01 | TraceRecord |
| SS-54 | `TraceConfig` (`log_sink`/`persist`/`state_values`/`otel`) nested under `Settings.trace` | config key | MIGRATION §9.5 | OBS-02 | TraceConfig |
| SS-55 | `PALADIN_TRACE_OTEL_ENABLED` — deliberately `PALADIN_*`, not `APP_*` | env var | MIGRATION §9.5 | OBS-02 | PALADIN_TRACE_OTEL_ENABLED |
| SS-56 | `otel` Cargo feature (`opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp`, HTTP/protobuf only) | feature flag | MIGRATION §9.3 | OBS-02 | otel |
| SS-57 | `GET /v1/dev-ui/threads/{id}` — non-API, admin-gated, `dev-ui`-feature-gated route | route | MIGRATION §9.6 | OBS-03 | /v1/dev-ui/threads |
| SS-58 | `paladin-eval` crate + `eval_scenarios!` custom test-harness macro | type | MIGRATION §9.3 | OBS-04 | paladin-eval |
| SS-59 | `PALADIN_EVAL_LIVE` — one of three simultaneous live-mode gates | env var | MIGRATION §9.5 | OBS-04 | PALADIN_EVAL_LIVE |
| SS-60 | `paladin-cli eval run <glob>` subcommand | CLI subcommand | MIGRATION §9.3 | OBS-04 | eval run |
| SS-61 | Tracing overhead exceeds the ≤3% bar — accepted, documented deviation for v0.10.0 | behavioral change | CHANGELOG [0.10.0] Known limitations | OBS-02 | tracing overhead |
| SS-62 | `run_traces` table — append-only persisted trace history | migration | MIGRATION §9.4 | OBS-02 | run_traces |

#### Phase 29 — Program Gates & Release

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-63 | `cargo doc --workspace --no-deps` zero-`warning:` bar ratified as the CI documentation gate | behavioral change | REQUIREMENTS SHIP-01 | SHIP-01 | cargo doc --workspace --no-deps |
| SS-64 | `openapi_golden_v0_9.rs` — program-wide byte-identity proof for every pre-existing `/v1` path | type | MIGRATION §9.6 | SHIP-02 | openapi_golden_v0_9 |
| SS-65 | 82% workspace line coverage floor (`cargo llvm-cov ... --fail-under-lines 82`) | behavioral change | MIGRATION §9.6 | SHIP-04 | fail-under-lines 82 |
| SS-66 | `openapi-generator-cli:v7.25.0` — pinned image generating the Python/TypeScript SDK clients | dependency | MIGRATION §9.6 | SHIP-04 | openapi-generator-cli |

#### Phase 30 — Token-Economy Vocabulary & Commissary Anchoring

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-67 | Units-plain/roles-medieval vocabulary rule (ADR-0049's governing rule) | vocabulary term | REQUIREMENTS VOCAB-01 | VOCAB-01 | Medieval Military |
| SS-68 | `Commissary` anchored (ADR-0049 + `docs/src/architecture/commissary.md`) | vocabulary term | REQUIREMENTS VOCAB-02 | VOCAB-02 | Commissary |
| SS-69 | `Treasurer` reserved as the output-side, cross-run spend-governance officer name (ADR-0050) | vocabulary term | REQUIREMENTS VOCAB-04 | VOCAB-04 | Treasurer |
| SS-70 | Four `max_tokens` meanings disambiguation table | vocabulary term | REQUIREMENTS VOCAB-05 | VOCAB-05 | max_tokens |
| SS-71 | `Quartermaster` purge — zero in-tree references remain | vocabulary term | REQUIREMENTS VOCAB-06 | VOCAB-06 | Quartermaster |
| SS-72 | Token-economy clean-break versioning decision (ADR-0051) | vocabulary term | REQUIREMENTS VOCAB-07 | VOCAB-07 | ADR-0051 |

#### Phase 31 — Lossless Token Accounting

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-73 | `TokenUsage` gains `cache_read_tokens`/`cache_write_tokens`/`reasoning_tokens` | type | CHANGELOG [0.10.0] Changed | ACCT-01 | TokenUsage |
| SS-74 | `TokenUsage::from_total` deleted outright, no `#[deprecated]` replacement | type | CHANGELOG [0.10.0] Changed | ACCT-01 | TokenUsage::from_total |
| SS-75 | `PaladinResult.usage: TokenUsage` replaces the bare `token_count: u32` field | type | MIGRATION §9.2 | ACCT-02 | PaladinResult |
| SS-76 | `StreamingResponse.usage` / `ChunkMetadata.usage` additive fields | type | MIGRATION §9.2 | ACCT-03 | StreamingResponse |
| SS-77 | `ExecuteResponse.usage: TokenUsageResponse` — HTTP-surface DTO replacing `token_count: u32` | type | MIGRATION §9.2 | ACCT-02 | TokenUsageResponse |
| SS-78 | Anthropic `prompt_tokens` now includes cache-read/cache-write tokens (billed-figure fix) | behavioral change | CHANGELOG [0.10.0] Fixed | ACCT-03 | prompt_tokens |
| SS-79 | Battalion per-Paladin token split zero-fill bug fixed (`per_paladin_tokens`) | behavioral change | CHANGELOG [0.10.0] Fixed | ACCT-02 | per_paladin_tokens |

#### Phase 32 — Unified Token Primitives

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-80 | `TokenCounterPort::is_exact(&self) -> bool` defaulted method | type | CHANGELOG [0.10.0] Added | PRIM-01 | is_exact |
| SS-81 | `Commissary::new`/`from_port` drop the caller-supplied `is_exact_counter` argument | type | CHANGELOG [0.10.0] Changed | PRIM-02 | Commissary::new |
| SS-82 | `paladin_llm::window::resolve_context_window` — one shared precedence resolver | type | CHANGELOG [0.10.0] Changed | PRIM-04 | resolve_context_window |
| SS-83 | `WindowSource`/`WindowFallbackPolicy`/`ResolvedWindow` re-exported from the `paladin` facade | type | CHANGELOG [0.10.0] Added | PRIM-04 | WindowSource |
| SS-84 | Legacy `garrison::TokenCounter` trait removed outright, no `#[deprecated]` shim | type | CHANGELOG [0.10.0] Removed | PRIM-03 | TokenCounter |
| SS-85 | Legacy `TokenCounterFactory` struct removed outright | type | CHANGELOG [0.10.0] Removed | PRIM-03 | TokenCounterFactory |

#### Phase 33 — Commissary In-Tree Adoption

| SS ID | Shipped item | Kind | Source | Req ID | Grep token |
|---|---|---|---|---|---|
| SS-86 | `RagRetrievalService::retrieve_context` returns the new `RagRetrievalResult` | type | CHANGELOG [0.10.0] Changed | COMM-01 | RagRetrievalResult |
| SS-87 | `RagRetrievalResult.shed: Vec<ShedItem>` — truncation/shed record surfaced to the caller | type | CHANGELOG [0.10.0] Changed | COMM-02 | ShedItem |
| SS-88 | `RagRetrievalError` typed enum (Sanctum/Commissary/budget-conversion failures) | type | MIGRATION §9.2 | COMM-01 | RagRetrievalError |
| SS-89 | `retrieve_context_with_timeout` free function returns `RagRetrievalResult` | type | MIGRATION §9.2 | COMM-01 | retrieve_context_with_timeout |
| SS-90 | `paladin-memory` gains an unconditional production dependency on `paladin-llm` | dependency | CHANGELOG [0.10.0] Added | COMM-01 | paladin-llm |
| SS-91 | `RagRetrievalService::with_token_counter(Arc<dyn TokenCounterPort>)` builder | type | CHANGELOG [0.10.0] Changed | COMM-04 | with_token_counter |

### D-10 ubiquitous-language list confirmation

The three ubiquitous-language lists Phase 35 SC4 names are confirmed by direct, line-anchored
content match:

1. **`.github/copilot-instructions.md`** — the naming table. `grep -n 'Commissary'
   .github/copilot-instructions.md` finds line 36: `| **Commissary** | Input-side, per-call
   window-rationing officer | \`crates/paladin-llm/src/services/commissary.rs\` |`.
2. **`.planning/PROJECT.md`** — the term list. `grep -n 'Commissary' .planning/PROJECT.md` finds
   line 1324, the Ubiquitous-language bullet's enumerated term list: "...Herald, Armory, Sanctum,
   Sentinel, Quest, Commissary) are mandatory in code, docs and comments" — this prose bullet is
   the row PROJECT.md carries in place of a literal markdown table.
3. **`docs/src/architecture/domain-model.md`** — line 30: `| **Commissary** | Input-side,
   per-call window-rationing officer | \`Commissary\` · \`crates/paladin-llm/src/services/
   commissary.rs\` |`.

**A fourth, partial list exists.** `docs/src/introduction.md` lines 78-91 carry a twelve-term
"Medieval Military naming convention" table (`Paladin`, `Battalion`, `Formation`, `Phalanx`,
`Campaign`, `Chain of Command`, `Maneuver`, `Garrison`, `Arsenal`, `Armament`, `Citadel`,
`Herald`) — found via `grep -rlni 'medieval military' docs/src/`. It does **not** carry
`Commissary`, `Sanctum`, `Sentinel`, `Quest`, `Conclave`, `Council`, `Grove` or `Commander` — a
genuine fourth list distinct from the three D-10 names. Its currency (a stale/incomplete-list
candidate) is a §2 mdBook-sweep question for whichever plan sweeps `introduction.md`, not judged
here — this task's own scope is compiling the checklist, not settling verdicts.

### Exports-diff cross-check (D-08 precedence source 1)

`git diff v0.9.0..HEAD -- .project/current-exports.txt` adds exactly **4,376** lines
(`grep '^+' <diff> | grep -vc '^+++'`), matching CONTEXT.md's own recorded figure exactly.
`.project/current-exports.txt`'s own header states it is generated by `cargo-public-api` and
tracks **only the `paladin` facade crate's exports** (`pub paladin::...` — `grep -c '^pub
paladin::' .project/current-exports.txt` is 1095 of 7924 total lines, the remainder `pub use`
re-export lines and blank/doc lines under the same facade root). This scope is why a genuine
public type shipped in `paladin-ports`/`paladin-llm`/`paladin-core` but not re-exported through
the facade prelude can show zero diff hits below without being missing from the tree.

Twenty-two identifier tokens sampled (exceeds the required fifteen):

| Token | Diff hits | Disposition |
|---|---|---|
| `WarEngine` | 26 | confirmed present |
| `PaladinError` | 171 | confirmed present |
| `ExecutionMiddleware` | 64 | confirmed present |
| `GarrisonEntry` | 6 | confirmed present |
| `PaladinResult` | 6 | confirmed present |
| `StructuredExecutorPort` | 7 | confirmed present |
| `TokenCounterPort` | 7 | confirmed present |
| `StopReason` | 5 | confirmed present |
| `TokenUsage` | 3 | confirmed present |
| `Commissary` | 3 | confirmed present |
| `ShedItem` | 3 | confirmed present |
| `VaultPort` | 2 | confirmed present |
| `RagRetrievalResult` | 2 | confirmed present |
| `resolve_context_window` | 1 | confirmed present |
| `WindowSource` | 1 | confirmed present |
| `BattalionError` | 1 | confirmed present (new variant only; the type itself is directly `pub paladin::`-rooted) |
| `Aegis` | 0 | not found — defined/used entirely within `paladin-core`/`paladin-battalion`, never re-exported at the tracked `paladin::` root (facade-scope gap, see header note above, not a missing shipped item) |
| `FallbackLlmAdapter` | 0 | not found — same facade-scope gap (`paladin_llm::fallback` is not re-exported at the tracked root) |
| `StreamingResponse` | 0 | not found — same facade-scope gap (`paladin-ports` type) |
| `ChunkMetadata` | 0 | not found — same facade-scope gap (`paladin-ports` type) |
| `LlmError` | 0 | not found for the phase's own new variants — the enum's re-export path (`pub use paladin::prelude::LlmError`) is unchanged since v0.9.0; a diff over one `pub use` line cannot show an added enum variant |
| `NodeError` | 0 | not found — the structured `paladin_core::...::node_error::NodeError` this phase adds is not re-exported at the `paladin::` root either |

Sixteen of twenty-two tokens confirmed present in the diff; the six not-found tokens are all the
same documented facade-scope limitation of this evidence source (§1 preface above), never a
D-00g tree/document conflict — no CHANGELOG/MIGRATION item sampled here disagreed with the tree.

## §2 mdBook verdict table

Scope: every `.md` under `docs/src/` — 93 files, live-counted via `find docs/src -name '*.md'`,
including `appendix/` and any file not linked from `SUMMARY.md` (D-05). 92 rows below are seeded
`pending` (unswept — plans 34-03/34-04/34-05 sweep them); one row
(`docs/src/appendix/doc-coverage-report.md`) is fully worked here to prove the row schema
end-to-end (D-06/D-07), per this task's method self-test.

### Build baseline (D-11) — measured by plan 34-03, Task 1

**Run window:** 2026-09-17T04:02:42Z – 2026-09-17T04:02:57Z (mdBook build + linkcheck); the full
sequence including `check-doc-examples.sh`/`check-doc-config.sh` completed by 2026-09-17T04:03:05Z.
Raw capture: `34-evidence/34-03-mdbook-build.txt`.

**Toolchain versions (verbatim), each annotated against its `docs.yml` pin (repeats the
Measurement Header block above for this subsection's self-containment):**

```
$ mdbook --version
mdbook v0.4.40
$ mdbook-linkcheck --version
mdbook-linkcheck 0.7.7
$ mdbook-mermaid --version
mdbook-mermaid 0.13.0
```
Compared against `docs.yml:46,50,54` (`mdbook --version 0.4.40`, `mdbook-mermaid --version
0.13.0`, `mdbook-linkcheck --version 0.7.7`) — **all three match exactly**, no drift.

**`mdbook-mermaid install docs/` mutation check (D-22, T-34-01):** `mdbook-mermaid install docs/`
was run first, then `git status --porcelain -- docs` was run immediately after. **It printed
nothing — the mermaid install did NOT mutate `docs/mermaid.min.js` / `docs/mermaid-init.js` this
run.** No `git checkout -- docs/` restoration was needed; there is no drift to route to Phase 35.

**`mdbook build docs/` (with the linkcheck backend active, `warning-policy = "error"` per
`docs/book.toml`):** exit code **0**, wall time **3s**. The html backend and the linkcheck backend
both ran. Linkcheck scanned **1006 links (0 incomplete)**; the verbatim summary line is:

```
[2026-09-17T04:02:57Z INFO  mdbook_linkcheck] No broken links found
```

515 lines in the capture are `WARN linkcheck::validation … because fragment resolution isn't
implemented` — a documented `mdbook-linkcheck` limitation (it does not resolve `{{#include}}`d
anchor fragments across pages), not link failures; `warning-policy = "error"` governs true
linkcheck failures (broken links), and the build still exited 0. One further benign line appears:
`Warning: The mdbook-mermaid preprocessor was built against version 0.4.36 of mdbook, but we're
being called from version 0.4.40` — a pre-existing preprocessor/mdbook version-skew notice, not a
build failure (mdBook and mdbook-mermaid are independently pinned in `docs.yml` and both pins were
confirmed above). No red step this run.

**`bash scripts/check-doc-examples.sh`:** exit code **0**. Layer 1 (`cargo check` on
`crates/doc-examples`) — "All included examples compile." Layer 1b (README quick-example mirror) —
"README Quick Example is in sync." Layer 2 (inline fenced-block scan across `docs/src`) — **0
checked, 616 skipped, 0 failed** (every inline ```rust block found is either illustrative,
`{{#include}}`-backed, or otherwise marked as non-standalone; none failed validation).

**`bash scripts/check-doc-config.sh`:** exit code **0**. **154 YAML blocks checked, 0 failed** —
every fenced ` ```yaml ` block under `docs/src/**/*.md` parses as valid YAML.

No red step in the whole `docs.yml` sequence this run; nothing routes to a Phase 35 `MB-nn` or a
Phase 36 `EX-nn` from the build baseline itself.

### Orphan check (D-05)

`docs/src/SUMMARY.md`'s markdown link targets were extracted and compared against every `.md` file
on disk under `docs/src/`.

- On-disk page count: `find docs/src -name '*.md' | sort | wc -l` → **93**
- Nav-reachable count (pages linked from `SUMMARY.md`, plus `SUMMARY.md` itself, which is not an
  orphan by definition): **93**
- `comm -23 <(find docs/src -name '*.md' | sort) <(nav-reachable set)` → **(empty)**

**The orphan set is empty.** Every one of the 93 on-disk pages is reachable from `SUMMARY.md`'s
nav. This confirms 34-RESEARCH.md's "zero orphans" claim by direct measurement rather than
carrying it forward unproven (D-00b) — the command above is the proof, not the prior claim.

### Vocabulary sweep (D-10, D-00f)

`grep -rniE '\bQuartermaster\b' docs/src` — **1 hit**:

```
docs/src/architecture/commissary.md:7:Quartermaster→Commissary rename rationale, and the rejected-name list are in ADR-0049
```

This is the RESEARCH.md P-07 finding: a sentence pointing at the ADR-0049 rename rationale — a
legitimate historical pointer, not a stray leftover — but it is a literal match for the D-10 grep,
which CONTEXT.md and ROADMAP require to be empty. Recorded as its own row below, classified
*stale content*, citing Phase 30 — `Quartermaster` purge, zero in-tree references (VOCAB-06,
SS-71). Whether an ADR-pointer sentence is an intentional exception to the must-be-empty rule is
**Phase 35's call, not this audit's** (D-00c: recorded, never fixed) — the page is not edited here.

| MB ID | Location | Classification | Note | Size |
|---|---|---|---|---|
| MB-02 | `docs/src/architecture/commissary.md:7` | stale content | Literal `Quartermaster` match inside a sentence pointing at the ADR-0049 rename rationale (historical pointer, not a stray leftover); D-10's grep requires the corpus to be empty regardless of intent — Phase 35 decides whether this sentence is an intentional exception | S |

**Phase 31 D-29 `token_count` hit-list re-check** — for each of the 10 pages, a grep for the bare
token (`token_count`) and the split type (`TokenUsage`) was run and compared:

| Page | `token_count` hits | `TokenUsage` hits | Disposition |
|---|---|---|---|
| `docs/src/getting-started/quickstart.md` | none | line 118 (`usage` \| `TokenUsage` \| prompt/completion split) | clean |
| `docs/src/operations/observability.md` | none | lines 38, 43 (`NodeFinished`/`RunFinished` `usage: TokenUsage`) | clean |
| `docs/src/user-guides/battalion-patterns.md` | none | lines 313-314 (`usage: TokenUsage`, `per_paladin_tokens: HashMap<String, TokenUsage>`) | clean |
| `docs/src/user-guides/output-formatting.md` | none | line 170 (full `TokenUsage` split) | clean |
| `docs/src/user-guides/agent-orchestrator-bridge.md` | none | line 111 (`usage` — the full `TokenUsage` split) | clean |
| `docs/src/appendix/conclave-pattern.md` | none | line 726 (full `TokenUsage`) | clean |
| `docs/src/architecture/domain-model.md` | **line 102: `pub token_count: usize,`** | line 35 (prose, vocabulary-rule context only, not adjacent to the struct) | **offending — see the row minted below** |
| `docs/src/user-guides/herald-output.md` | none | line 55 (full `TokenUsage` split, six-key object) | clean |
| `docs/src/user-guides/memory-management.md` | 18 hits, all `GarrisonEntry.token_count: Option<u32>` usage (fields, SQL column, builder calls) | none | clean — see disposition note below |
| `docs/src/user-guides/paladin-agents.md` | none | line 161 (`usage` \| `TokenUsage` \| prompt/completion split) | clean |

**Disposition for `memory-management.md`:** its `token_count` occurrences all describe
`GarrisonEntry.token_count`, a Garrison memory-entry field distinct from the `PaladinResult` /
`StreamingResponse` / `TokenUsageResponse` carriers Phase 31 (ACCT-01…05) changed — the page's own
line 158 comment states this explicitly ("Token counting is a separate concern from
`GarrisonConfig`"). The page's type (`Option<u32>`, line 46) matches the live
`crates/paladin-core/src/platform/container/garrison.rs` field exactly (verified below). This page
was never an ACCT-01…05 target and is correctly clean, not a false negative.

**Disposition for `domain-model.md`, the offending row above:** the page's `GarrisonEntry` snippet (lines 96-104)
shows `token_count: usize` with no `id`, `timestamp` or `is_summary` field, and names the role
field's type `MessageRole`. The live struct at
`crates/paladin-core/src/platform/container/garrison.rs:57-75` is:

```rust
pub struct GarrisonEntry {
    pub id: Uuid,
    pub role: ConversationRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
    pub token_count: Option<u32>,
    pub is_summary: bool,
}
```

`token_count` is `Option<u32>` in the tree, not `usize`; the role field's live type is
`ConversationRole`, not `MessageRole`; and the struct is missing `id`, `timestamp` and — the
Phase 26 (RT-03/RT-FR-12, D-17) addition — `is_summary`, which the live rustdoc's own
"X-10 release note (v0.10.0, RT-03/RT-FR-12, D-17)" comment documents as load-bearing for the
effective-history definition the `HistoryTrimmer`/`SummarizationMiddleware` (SS-38) both read.

| MB ID | Location | Classification | Note | Size |
|---|---|---|---|---|
| MB-03 | `docs/src/architecture/domain-model.md:96-104` | stale content | `GarrisonEntry` snippet is stale on four counts: `token_count: usize` should be `Option<u32>`; `role: MessageRole` should be `ConversationRole`; missing `id: Uuid` and `timestamp: DateTime<Utc>` fields; missing the Phase 26 `is_summary: bool` field (`is_summary` is the effective-history marker `HistoryTrimmer`/`SummarizationMiddleware` depend on) | Phase 26 — `GarrisonEntry.is_summary` (RT-03, SS-38) | M |

This row is cross-referenced, not duplicated, when `domain-model.md`'s own §2 verdict is
settled below (Task 2) — per this task's action text, "do not mint a second row for the same
line" (the pattern already established for `commissary.md`'s vocabulary-hit row above).

### Object-store currency sweep (CONTEXT.md Folded Todos — MinIO slice)

`grep -rniE 'minio|dl\.min\.io|quay\.io' docs/src examples` — **247 hits** across 27 files (full
verbatim hit list: `34-evidence/34-03-mdbook-build.txt`, "object-store currency sweep" section).
Every hit was checked against the pin quick task 260913-15w introduced (Docker Hub community
images retired, 404 since 2026-09-12; `dl.min.io` returns 410; current form is a `quay.io` image
pin plus the `mc` client from an archived GitHub release asset):

- `grep -rniE 'dl\.min\.io' docs/src examples` → **(none)** — no page or example names the retired
  download host.
- `grep -rniE '(^|[^./])minio/minio' docs/src examples | grep -v 'quay.io/minio/minio'` →
  **(none)** — no page or example names the bare Docker Hub community image form.
- Every actual container-image reference already uses the pinned form,
  `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` — **9 occurrences across 6
  files** (`docs/src/contributing/testing-guide.md` ×2, `docs/src/contributing/branching-model.md`
  ×1 in prose, `docs/src/deployment/cicd.md` ×1, `docs/src/appendix/integration-tests.md` ×1,
  `docs/src/deployment/docker.md` ×1, `docs/src/appendix/minio-file-repository-setup.md` ×4) — all
  byte-identical to the pin.

**Verdict: zero MB-nn items from this slice.** No page or example under `docs/src` or `examples`
names a retired Docker Hub image or a `dl.min.io` URL; every image-pin occurrence already carries
the current `quay.io` form. The remaining 238 hits are the word "MinIO" naming the service, the
`s3-storage` feature flag, config keys (`minio_endpoint` etc.) and environment variables
(`APP_MINIO_*`, `MINIO_*`) — none of these name a retired image or URL, so none qualify for an
`MB-nn` under this slice's own scope. The RustFS evaluation itself stays out of scope, deferred to
plan 34-09's deferred register (`todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`
already tracks it as a pending todo, unchanged by this phase).

### SC5 read-only proof (Task 1 close)

`git status --porcelain -- . ':!.planning'` → **(empty)**. No file outside `.planning/` was
created, modified or deleted by this task, including `docs/mermaid.min.js` / `docs/mermaid-init.js`
(confirmed separately above).

| # | Page | Verdict | Findings (signal class → cmd → result) | Cites (Phase N — item (REQ)) | MB ID(s) | Size |
|---|------|---------|------------------------------------------|-------------------------------|----------|------|
| 1 | docs/src/SUMMARY.md | current | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → none (nav file carries no version pins to go stale); class 3 (crate names): `grep -noE 'paladin-[a-z-]+'` → `paladin-agents`, `paladin-configuration` (both are page-title fragments, not crate references) — no false crate claim; class 9 (shipped-surface, `34-signals.sh`): 3 hits (`fault-tolerance.md`, `commissary.md`, `cli-muster.md` link targets) — checked, matches; orphan check above (D-05) is this row's direct proof: `comm -23` of the 93 on-disk pages against SUMMARY.md's own link targets is empty, so every page this file's nav claims to reach does exist, and (per the same check run in reverse) every page on disk is reached from it — the two facts SUMMARY.md's entire content asserts | — | — | — |
| 2 | docs/src/api-reference/crate-map.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 12 "current published workspace, v0.5.0", lines 184/192/206/221-226 pin every consumer profile to `"0.5.0"` — live `Cargo.toml` `[workspace.package] version = "0.10.0"` (checked: `grep -n '^version = ' Cargo.toml`) — **mismatch**; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 crates listed (`paladin-ai-core/-ports/-battalion/-llm/-memory/-storage/-content/-notifications/-web`), line 17 says "nine member crates" — live `ls crates/` → 11 library crates + `doc-examples` (`paladin-eval`, `paladin-herald` both absent from this page) — **mismatch**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (page names no Phase 22-33 SS-nn token at all); direct check (mandated, D-03 crate-graph edge): the mermaid graph (lines 44-83) has no `mem --> llm` edge — live `paladin-memory/Cargo.toml` carries an unconditional `paladin-llm` dependency (Phase 33, COMM-01) — **mismatch, edge missing** | Phase 33 — `paladin-memory` gains an unconditional dependency on `paladin-llm` (COMM-01, SS-90) | MB-14 | L |
| 3 | docs/src/api-reference/feature-flags.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 120 `paladin-ai = "0.8"`, plus ~20 more `"0.5"` pins throughout the usage-example section, line 419 Dockerfile `FROM rust:1.75` — live workspace version `0.10.0`, MSRV `1.88` (`rust-toolchain.toml`) — **mismatch on both axes**; class 8 (feature flags): `grep -noE '"[a-z-]+"'` cross-checked against `grep -n '^\[features\]' -A60 Cargo.toml` → page correctly documents `llm-kimi`/`llm-qwen`/`llm-grok`/`llm-ollama`/`llm-gemini`/`llm-openai-compatible` (Phase 17) but is missing `otel` (Phase 28, OBS-02), `dev-ui` (Phase 28, OBS-03), `redis-cache` (Phase 25, FT-06) and `storage-postgres` (Phase 22, ENG-05) entirely from every flag table — **4 shipped Phase 22-33 flags undocumented**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits | Phase 25 — `redis-cache` Cargo feature on `paladin-storage` (FT-06, SS-33) | MB-15 | L |
| 4 | docs/src/api-reference/migration-guide.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 3 "up to the current v0.5.0 release", line 512 Timeline table marks `0.1.0` as "**Current**" — self-contradicted by the page's own "Upgrading to v0.10.0" section immediately below (lines 5-50), which is otherwise accurate; class 4 (module/source paths): the v0.10.0 bridging section (lines 16-50) correctly names `PaladinResult.usage`, `NodeExecutionRecord.usage`, `paladin_llm::window::resolve_context_window` — no path errors found; D-09 row-for-row check: the page does not duplicate MIGRATION.md §9.1/§9.8 content — it deliberately points to `upgrading.md`/`MIGRATION.md` instead (line 7-14) — checked, zero disagreements found because there is no duplicated content to disagree (the pointer targets are correct and current); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `TokenUsage`, `RagRetrievalResult`, `resolve_context_window`, `WindowSource`, `TokenCounterPort` (the v0.10.0 bridging section, lines 16-50) — checked, matches the live tree | Phase 29 — release versioning is v0.10.0-scoped (SHIP-01); the page's own opening line has not been updated to match its own v0.10.0 section | MB-16 | S |
| 5 | docs/src/api-reference/platform-api.md | current | class 4 (module/source paths): page names `paladin-web` (routes/DTOs), `paladin-ports`, `paladin-core`, `src/application/services/run/*` — all confirmed present (`test -d`); class 6 (workflow/job names): none on this page — routes and DTOs only, no CI job reference to go stale; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `POST /v1/runs`, `POST /v1/runs/{run_id}/cancel`, `GET /v1/runs/{run_id}/stream`, `GET /v1/threads/{id}/state`, `POST /v1/threads/{id}/resume`, `GET /v1/threads/{id}/history`, `webhook_deliveries`, `APP_WEBHOOKS_ALLOW_PRIVATE`, `RunQueuePort` — all SS-44…SS-52 items named correctly; direct route check: `grep -rn '"/v1/runs\|"/v1/threads\|"/v1/assistants\|"/v1/schedules' crates/paladin-web/src` reproduces every route path the page documents (`run_controller.rs`, `thread_controller.rs`, `assistant_controller.rs`, `schedule_controller.rs` test URIs) — checked, matches; page carries its own `**Since:** v0.10.0 (PRD 06, Phase 27)` marker | — | — | — |
| 6 | docs/src/api-reference/stable-api.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 3 "Version: 0.5.0", line 898-900 "Last Updated: 2026-04-16", "Paladin Version: 0.5.0" — live workspace version `0.10.0` — **mismatch**; class 4 (module/source paths): every fully-qualified path in the catalog (e.g. `paladin::core::platform::container::paladin::Paladin`, `paladin::application::services::paladin::PaladinBuilder`) uses the pre-hexagonal-reorg `src/core/`/`src/application/` layout; live paths root at `paladin_core::platform::container::...` (`paladin-ai-core` package) and `paladin_ports::output::...` — checked against `crates/paladin-core/src/platform/container/paladin.rs` and `crates/paladin-ports/src/output/paladin_port.rs` — **mismatch, every catalog path is stale**; line 920-931's "Public crates" list has 10 entries, missing `paladin-eval` (Phase 28) and `paladin-herald`; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (no Phase 22-33 item named anywhere on the page) | Phase 28 — `paladin-eval` crate + `eval_scenarios!` harness (OBS-04, SS-58) | MB-17 | L |
| 7 | docs/src/api-reference/upgrading.md | current | class 1 (version strings): none — page correctly avoids pinning a crate version, deferring to `MIGRATION.md`; D-09 row-for-row check against `MIGRATION.md` §9.1 (4 entries: M-B-01…M-B-04) and §9.8 (7 checklist steps): every entry present on this page, condensed to one line each, with no contradiction — checked line-by-line, `sed -n '/^## 9\.1/,/^## 9\.2/p'` and `/^## 9\.8/,$p' MIGRATION.md` against the page's own table (lines 19-24) and checklist (lines 26-63): **0 disagreements, 0 omissions** on both sides (4/4 and 7/7 entry coverage); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `Waypoint`, `WarEngine`, `TokenUsage`, `RagRetrievalResult`, `resolve_context_window`, `WindowSource`, `Commissary::new` — all correct; class 4 (module/source paths): none — page correctly defers all code detail to `MIGRATION.md` rather than duplicating file paths | — | — | — |
| 8 | docs/src/api-reference/wargraph-doc-schema.md | current | class 4 (module/source paths): `paladin_battalion::engine::graph_doc` confirmed at `crates/paladin-battalion/src/engine/graph_doc.rs`; class 7 (error types): `grep -noE '[A-Z][A-Za-z]*Error(::[A-Za-z]+)?'` → `CompileError`, `CompileError::UnsupportedNodeKind`, `CompileError::NestingTooDeep`, `CompileError::UnknownSchemaVersion`, `CompileError::UnregisteredEdgeEvaluator` etc. — all confirmed live: `grep -n 'pub enum CompileError' -A5 crates/paladin-battalion/src/engine/graph_doc.rs` and `grep -n 'WARGRAPH_DOC_SCHEMA_VERSION' crates/paladin-battalion/src/engine/graph_doc.rs` reproduce the constant (`"1"`) and every named variant exactly; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 direct SS-nn hits (this page predates no §1 row by name, but is itself the Phase 27 WarGraphDoc schema the SS-47 `/v1/assistants` route family persists) — checked, matches; page carries its own `**Since:** v0.10.0 (Doc 06, plan 27-05)` marker | — | — | — |
| 9 | docs/src/appendix/battalion-benchmarks.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` -> line 11 "Rust Version: 1.85+ (2024 edition)" -- live `rust-toolchain.toml`/`Cargo.toml` `rust-version = "1.88"` (Phase 22.1, SS-08) -- **mismatch**; class 5 (make targets): none, page uses raw `cargo bench --bench battalion_benchmarks` directly; direct check (D-04 producing command): `cargo bench --bench battalion_benchmarks` target confirmed live at `crates/paladin-battalion/benches/battalion_benchmarks.rs` (`ls crates/paladin-battalion/benches/`) -- checked, matches; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (this is a pre-milestone, Milestone-1-era Epic-24 benchmark snapshot dated 2026-01-25, no Phase 22-33 item named); the page's own closing status line ("Epic 24 Update") and its explicit dated header confirm it is a historical performance snapshot, not a claim about current MSRV, so only the MSRV line is a genuine content error | Phase 22.1 -- workspace MSRV floor raised 1.85 -> 1.88 (SS-08) | MB-37 | S |
| 10 | docs/src/appendix/battalion-patterns-guide.md | **stale** | class 4 (module/source paths), direct check (mandated, D-04 producing command): all four code samples (lines 38-39, 129-130, 227-228, 349-350) open with `use paladin::battalion::*;` -- reproduced with a throwaway `examples/_scratch.rs` + `cargo check --example _scratch --features cli`: `error[E0432]: unresolved import \`paladin::battalion\`... could not find \`battalion\` in \`paladin\`` -- **every code sample on the page is broken as written**; the facade's actual re-export surface for this content is `paladin::prelude::*` (confirmed compiling) plus item-level imports from `paladin::core::platform::container::battalion::*` (`src/core/platform/mod.rs`), never a bare `paladin::battalion` module; class 1 (version strings): none; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (Formation/Phalanx/Campaign/ChainOfCommand patterns predate Phase 22-33) | Phase 30 -- vocabulary/facade precision is a program-wide concern (VOCAB-01); no Phase 22-33 REQ-ID applies, the broken import predates this milestone (D-00g) | MB-38 | M |
| 11 | docs/src/appendix/battalion-vision-support.md | current | class 4 (module/source paths): all four `use paladin::...` imports (lines 271-274) reproduced against a throwaway `cargo check --example` -- all four compile (only unused-import warnings), confirmed via `src/application/services/battalion/mod.rs:14` (`pub use paladin_battalion::formation_service;`) and `src/core/platform/mod.rs`'s battalion re-export block; direct check (mandated, D-04 producing command): `Paladin.vision_enabled` field, `PaladinBuilder::enable_vision()`, `PaladinExecutionService::execute_with_vision()` all confirmed live (`grep -rn 'fn execute_with_vision\|fn enable_vision' src/`) with matching arity/behavior to the page's description; class 1 (version strings): none; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (vision predates Phase 22-33) | — | — | — |
| 12 | docs/src/appendix/branch-protection.md | current | class 4/direct check (mandated, D-04 producing command): the page's 44-context required-status-check list (lines 84-92) reproduced exactly, in the same order, via `python3 -c "import json; ..."` reading `.github/rulesets/protect-main-branch.json`'s `required_status_checks` array -- **44/44 exact match**; `required_approving_review_count: 0` confirmed on both `protect-main-branch.json` and `protect-release-branches.json`; `bypass_actors` confirmed empty on both branch rulesets and `[{actor_id: 5, actor_type: RepositoryRole, bypass_mode: always}]` on `protect-release-tags.json` -- all three claims match exactly; class 6 (workflow names): `verify-tag-source` job and `git merge-base --is-ancestor` command reproduced verbatim at `.github/workflows/release.yml:29,76`; `make release` branch/up-to-date checks reproduced at `Makefile:581-601`; class 1 (version strings): `make release VERSION=0.8.0`/`VERSION=0.4.1` are illustrative example invocations, not factual claims about current state; ADR-0043 and ADR-0044 both confirmed present on disk | — | — | — |
| 13 | docs/src/appendix/build-baselines.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` -> line 17 `rustc 1.95.0`, line 31 `1.95.0` -- live `rustc --version` this run is `1.97.1` (Measurement Header) -- **mismatch** (this is a Milestone 7 Epic 2, 2026-05-27-dated build-time benchmark, so the toolchain drift is expected for a snapshot, but the page's "M7 Current (10-crate)" framing (line 27) is misleading against today's tree); class 3 (crate names): `grep -noE 'paladin-[a-z-]+'` -> 6 named in the M5/M7 comparison table (`paladin-core`,`paladin-ports`,`paladin-llm`,`paladin-memory`,`paladin-battalion`,`paladin-storage`,`paladin-notifications`,`paladin-content`,`paladin-web`, i.e. 9 distinct + facade) -- live `ls crates/` -> 11 library crates + `doc-examples` (missing `paladin-eval`, `paladin-herald`, both added after this 2026-05-27 snapshot) -- **mismatch, the "10-crate" framing is stale**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (predates Phase 22) | Phase 28 -- `paladin-eval` crate added (OBS-04, SS-58); the `paladin-herald` facade-cleanup crate predates Phase 22-33 (D-00g) | MB-39 | S |
| 14 | docs/src/appendix/cli-configuration.md | **stale** | class 5 (make targets)/direct check (mandated, D-04 producing command): the Scheduler troubleshooting section ("Jobs scheduled but never run") tells the reader to "Ensure scheduler port is wired in application (no TODO at line 297)" -- `grep -rn 'TODO.*scheduler\|scheduler.*TODO' src/ crates/` returns **zero hits**; the scheduler is fully wired into the platform API's `/v1/schedules*` route family (Phase 27, PLAT-05, SS-48) -- the page's own troubleshooting caveat is now stale, describing a gap that closed in Phase 27; class 1 (version strings): none; class 7 (error types): `GarrisonConfigError`/`ArsenalConfigError`/`SchedulerError`/`LlmError` all confirmed live via `grep -rn` in `src/application/cli/config/`; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits | Phase 27 -- `/v1/schedules*` cron-driven recurring run submission wired (PLAT-05, SS-48) | MB-40 | S |
| 15 | docs/src/appendix/cli-council.md | **stale** | class 4/direct check (mandated, D-04 producing command, the CLI cluster's live-surface confirm-or-contradict): the page's "Command Syntax" block (lines 65-108) documents a positional `<QUESTION>` argument plus `-n/--num-agents`, `-m/--mode` (parallel/sequential/debate), `-r/--roles`, `-o/--output`, `-f/--format`, `--synthesize`/`--no-synthesize`, `--provider`, `--max-tokens`, `--timeout` -- the live `Commands::Council` variant (`src/bin/paladin-cli.rs:112-134`) has **no positional argument at all** and exposes only `--topic` (no short), `--participants` (default 3, no short), `--roles`, `--max-rounds` (default 5), `--save`, `--model`, `--temperature` -- **every flag on the page except `--roles`/`--model`/`--temperature`'s long names is fabricated** (`--mode`, `--synthesize`, `--provider`, `--max-tokens`, `--timeout`, `-f/--format` do not exist on any `Council` field; `--save` is renamed `-o/--output` on the page); the `cli` feature-flag requirement for the `paladin` binary is never mentioned; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone CLI surface); the divergence is item-level per D-03, tracked for Phase 35 CLI-family reconciliation | MB-41 | L |
| 16 | docs/src/appendix/cli-muster.md | **stale** | class 4/direct check (mandated, D-04 producing command): the page's "Command Syntax" block (lines 60-99) documents a positional `<DESCRIPTION>` argument plus `-p/--pattern`, `-o/--output`, `-f/--format`, `-y/--yes`, `--provider`, `--model`, `--temperature`, `--validate`/`--no-validate`, `--interactive` -- the live `Commands::Muster` variant (`src/bin/paladin-cli.rs:76-95`) has **no positional argument** and exposes `--task` (no short), `-o/--output` (short confirmed real), `--execute`, `--provider` (no short), `--model` (no short), `--no-review` -- fabricated flags: `--pattern`, `-f/--format`, `-y/--yes`, `--temperature`, `--validate`, `--interactive`; the `cli` feature-flag requirement is never mentioned; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone CLI surface); item-level per D-03 | MB-42 | L |
| 17 | docs/src/appendix/cli-onboarding.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): `src/application/cli/commands/onboarding.rs` confirmed present (`test -f`), `run_onboarding()` at line 644; direct check (mandated, D-04 producing command): the "Environment Variables" section (lines 307-317) claims `PALADIN_ENV_FILE=./config/.env paladin onboarding` and `PALADIN_SKIP_VALIDATION=1 paladin onboarding` -- `grep -rn 'PALADIN_ENV_FILE\|PALADIN_SKIP_VALIDATION' src/` returns **zero hits**; `run_onboarding()` takes no arguments and reads no such env vars -- both variables are fabricated; the top-level invocation `paladin onboarding` (no flags) is itself correct and matches the live zero-arg `Commands::Onboarding` variant; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies; item-level per D-03 | MB-43 | M |
| 18 | docs/src/appendix/cli-setup-check.md | **stale** | class 4/direct check (mandated, D-04 producing command): the "Command Options" block (lines 27-32) documents `-v/--verbose`, `-q/--quiet`, `--json` -- the live `Commands::SetupCheck { verbose: bool }` variant (`src/bin/paladin-cli.rs:61-65`) exposes only `#[arg(long)] verbose` -- **no short `-v`, no `--quiet`/`-q`, no `--json` exist**; class 5 (make targets): `cargo build --release --bin paladin-cli` (lines 366, 366) reproduced against `Cargo.toml:469-472` `[[bin]] name = "paladin-cli" required-features = ["cli"]` -- the command omits `--features cli` and would fail to build as written (the `cli` feature is not in the workspace `default` set, `Cargo.toml:492`); class 1 (version strings): example output "Paladin CLI: v0.1.0"/"Rust Toolchain: 1.75.0" is illustrative sample terminal output, not a factual claim; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies; item-level per D-03 | MB-44 | S |
| 19 | docs/src/appendix/cli-testing.md | **stale** | class 4 (module/source paths): all four cited test files (`tests/cli/environment_tests.rs`, `tests/integration/cli_real_services_test.rs`, `tests/integration/cli_real_providers_test.rs`, `tests/integration/llm_live_api_tests.rs`) confirmed present via `test -f`; class 8 (feature flags): `live-api-tests = []` confirmed live at `Cargo.toml:498`; direct check (mandated, D-04 producing command), the "Test Counts" table (lines 168-174): Tier 1 `45` matches `grep -c '#\[test\]\|#\[tokio::test\]' tests/cli/environment_tests.rs` exactly; Tier 2 `6` matches `cli_real_services_test.rs` exactly; Tier 3 `5` matches `cli_real_providers_test.rs` exactly; **Tier 4 claimed `12` ("4 per provider x 3 providers") but `grep -c` on `llm_live_api_tests.rs` returns `13`** -- one test over the claimed count, a genuine off-by-one; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies | MB-45 | S |
| 20 | docs/src/appendix/cli-usage.md | **stale** | class 5 (make targets)/direct check (mandated, D-04 producing command): the "Installation" section (`cargo build --release --bin paladin-cli`, lines 63-70, repeated in the Troubleshooting section) never mentions `--features cli`, though `Cargo.toml:469-472` marks the `paladin-cli` binary `required-features = ["cli"]` and `cli` is absent from the workspace `default` feature set (`Cargo.toml:492`) -- the quickstart will not build as written, the exact gap the plan's action text names; the inline "Commands Reference" sections for `paladin council`/`paladin muster` (correctly using `--topic`/`--task`, unlike the dedicated `cli-council.md`/`cli-muster.md` pages) invent short flags not on the live struct (`-p/--participants`, `-m/--model`, `-t/--temperature` for council; `-t/--task`, `-p/--provider`, `-m/--model` for muster all lack a `short` attribute on the live `clap::Args`) -- confirmed against `src/bin/paladin-cli.rs`'s field-level `#[arg(...)]` attributes; class 1 (version strings): "Paladin CLI: v0.1.0" (line 191) is illustrative sample output; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies | MB-46 | M |
| 21 | docs/src/appendix/conclave-pattern.md | current | class 4/direct check (mandated, D-04 producing command): `use paladin::prelude::*;`/`use paladin::battalion::conclave::*;` (lines 81-82) both confirmed compiling (facade-root style, unlike the sibling battalion-patterns-guide.md's broken import); `ConclaveExecutionService::new(paladin_port)` (single-arg) confirmed exactly at `crates/paladin-battalion/src/conclave_execution_service.rs:54`; `ConclaveResult` fields (`expert_outputs`, `aggregated_output`, `execution_time_ms`, `expert_execution_times`, `retry_counts`, `status`) all confirmed exactly at `crates/paladin-core/src/platform/container/battalion/conclave.rs:384-402`, matching every `result.*` accessor used on the page; `paladin battalion run -c conclave.yaml -t conclave -o result.json` (line 516) confirmed against `BattalionRunArgs`'s `-c/--config`, `-t/--type`, `-o/--output` and the live `conclave` entry in `BattalionYamlConfig`; class 1 (version strings): none; class 9 (shipped-surface, plus prior Vocabulary sweep line 452): `TokenUsage` (line 726) confirmed the full struct, checked, matches | — | — | — |
| 22 | docs/src/appendix/contributing-legacy.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` -> line 25 "Install Rust 1.70+" -- live `rust-toolchain.toml`/`Cargo.toml` `rust-version = "1.88"` -- **mismatch**; class 4 (module/source paths), direct check: the "Project Structure" block (lines 41-50) shows only `src/{core,application,infrastructure}` -- omits the `crates/` Cargo workspace entirely (11 library crates + `doc-examples`, `ls crates/`), a pre-workspace-decomposition (pre-Milestone-7) description of the tree; the `git clone https://github.com/your-org/paladin.git` placeholder (line 32) does not match the real repository (`https://github.com/DF3NDR/paladin-dev-env`, confirmed via `Cargo.toml:64` `repository =`); nav title already reads "Contributing (Legacy)" (`SUMMARY.md:121`) but, unlike `design-and-architecture.md`, the page's own body never self-declares superseded/archived status pointing a reader to `contributing/development-setup.md` (settled `current` in 34-04); class 9 (shipped-surface): 0 hits | Phase 22.1 -- MSRV floor raised 1.85->1.88 (SS-08); no Phase 22-33 REQ-ID for the workspace-decomposition drift (D-00g, predates this milestone) | MB-47 | M |
| 23 | docs/src/appendix/council.md | **stale** | class 4 (module/source paths): `use paladin::core::platform::container::battalion::council::{...}`/`use paladin::application::services::battalion::council_service::CouncilExecutionService;` (lines 82-83) both confirmed to still compile as backward-compatible facade re-exports (`src/core/platform/mod.rs`, `src/application/services/battalion/mod.rs`); direct check (mandated, D-04 producing command), the API shape itself: page's `CouncilExecutionService::new(Arc::new(paladin_port), Some(Arc::new(garrison_port)))` (2 args, line 116-119) vs live `CouncilExecutionService::new(paladin_port, garrison_port, registry)` (**3 args**, `crates/paladin-battalion/src/council_service.rs:82-86`) -- **missing the required `registry: Arc<dyn PaladinRegistry>` parameter**; page's `result.conversation_history`/`result.final_output` (lines 124-125) vs live `CouncilResult { transcript, conclusion, rounds_completed, termination_reason }` (`council_service.rs:25-37`) -- **neither field named on the page exists on the live struct**; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone API drift, D-00g) | MB-48 | L |
| 24 | docs/src/appendix/design-and-architecture.md | current | class 1 (version strings): none applicable -- the page opens with an explicit self-declared disposition block ("Archived -- historical document... superseded and is retained only as a historical record; it is not maintained"), pointing readers to `architecture/overview.md` and `sentinel.md`, per ADR-0047 (`.planning/decisions/0047-architecture-appendix-disposition.md`, confirmed present via `ls`); class 6 (workflow/job names): none on this page; direct check (mandated, D-04 producing command): nav title already reads "Design and Architecture (Archived)" (`grep -n design-and-architecture docs/src/SUMMARY.md` -> `SUMMARY.md:112`) -- the page's own claimed status matches its actual content exactly, it makes no claim to be current or authoritative architecture, so it cannot mismatch the tree; class 9 (shipped-surface): 0 hits (deliberately, per its own archival framing) | — | — | — |
| 25 | docs/src/appendix/doc-coverage-report.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → none; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 hits (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage` — an incomplete, 2026-05-28-era list missing `paladin-eval`/`paladin-herald`/the `paladin-ai` facade); direct measurement: `cargo doc --workspace --no-deps 2>&1 \| tee 34-evidence/34-01-cargo-doc-default.txt` → **73** `warning:` lines (34-EVIDENCE.md #5), directly contradicting this page's line 18 ("Current result: docs build succeeds with no warnings") | Phase 29 — cargo doc zero-`warning:` bar ratified (ADR-0033, D-00a); Phase 33 — 73-warning baseline carried (`33-CI-EVIDENCE.md` row 26) (CURR-02) | MB-01 | M |
| 26 | docs/src/appendix/flow-dsl-guide.md | current | class 4/direct check (mandated, D-04 producing command): all four `use paladin::...` imports (lines 78, 94, 104, 364) reproduced against a throwaway `cargo check --example` -- all compile (unused-import warnings only), confirmed against `src/core/platform/mod.rs`'s `container::battalion::maneuver`/`::parser` re-export block and `src/application/services/battalion/mod.rs`; `paladin maneuver visualize --config <file> --format <ascii\|mermaid> --output <file>` and `paladin maneuver validate --config <file> --verbose` (throughout) confirmed exactly against the live `ManeuverCommands::{Visualize,Validate}` variants and their `-c/--config`,`-f/--format`,`-o/--output` args (`src/application/cli/commands/maneuver.rs:28-58`) -- the page correctly documents only `visualize`/`validate` and never claims a third `execute` subcommand (even though the top-level CLI's own `--help` doc-comment in `src/bin/paladin-cli.rs:53` misleadingly says "visualize, validate, execute" -- that stale doc-comment is a code-side defect, not a docs/src page, out of this audit's file scope); class 1 (version strings): none; class 9 (shipped-surface): 0 hits | — | — | — |
| 27 | docs/src/appendix/grove.md | current | class 4/direct check (mandated, D-04 producing command): `use paladin::application::services::battalion::grove_service::GroveExecutionService;` and `use paladin::core::platform::container::battalion::grove::{...}` (lines 97-100) both confirmed compiling via a throwaway `cargo check --example` (unused-import warnings only), matching `src/application/services/battalion/mod.rs` and `src/core/platform/mod.rs`'s re-export blocks; `GroveExecutionService::execute()` confirmed live at `crates/paladin-battalion/src/grove_service.rs:185` (`pub async fn execute(&self, grove: &Grove, task: &str) -> Result<GroveResult, BattalionError>`), matching the page's own call sites; class 1 (version strings): none; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (Grove predates Phase 22-33) | — | — | — |
| 28 | docs/src/appendix/integration-tests.md | **stale** | class 4 (module/source paths), direct check (mandated, D-04 producing command): `ls tests/integration/*.rs \| wc -l` -> **60** top-level integration test files live; the page's "Main test files" table (lines 20-56) lists only **30**; `comm`-style diff against the page's own file-name mentions surfaces **26 live files entirely absent from the inventory**, including `e2e_approval_gate_test.rs`, `e2e_crash_resume_test.rs`, `e2e_muster_defer_order_test.rs`, `e2e_platform_api_test.rs` (Phase 29's own three named E2E acceptance scenarios plus the Phase 27 platform-API E2E test), `rag_commissary_test.rs` (Phase 33's F6 exit-criteria test), `vault_confinement_test.rs` (Phase 26), `otel_transport_test.rs` (Phase 28), `v0_9_config_boot_test.rs` (Phase 29 D-07 backward-compat test), `war_engine_tracer_test.rs`/`waypoint_retention_fault_injection_test.rs` (Phase 22/25), `structured_engine_node_test.rs` (Phase 26), `golden_bridge_equivalence_test.rs`, `middleware_under_engine_test.rs`, `multi_parley_suspension_test.rs`, `parley_resume_stress_test.rs` (Phase 24), `provider_switching_test.rs`, `ollama_docker_test.rs`, `reasoning_agent_test.rs`, `subgraph_formation_in_campaign_test.rs`, `commander_error_paths_test.rs`, `aegis_retry_stress_test.rs`, `arsenal_bridge_regression_test.rs`, `battalion_chain_of_command_herald_test.rs`, `battalion_herald_end_to_end_test.rs`, `orchestrator_workflow_lifecycle_test.rs`; the "Battalion sub-module" table (7 files) and the CI service-provisioning section (`.github/workflows/integration-tests.yml`, the MinIO `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` pin) are both confirmed exactly correct (cites the already-current pin from 34-03's object-store sweep rather than re-deriving it); class 6 (workflow/job names): `.github/workflows/integration-tests.yml` confirmed present; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> hits on `Muster`, `NodeSpec::Battalion`, `VaultPort`, `RagRetrievalResult` (none of which appear in the inventory table since the files naming them are entirely missing) | Phase 22 through Phase 33 -- essentially every phase's own integration-test additions are missing from the inventory (representative REQ-IDs: PLAT-01 SS-44, COMM-01 SS-86, RT-04 SS-39, OBS-02 SS-54) | MB-49 | L |
| 29 | docs/src/appendix/minio-file-repository-setup.md | **stale** | class 2 (dependency pins)/object-store currency (cites 34-03's sweep rather than re-deriving, per CONTEXT.md Folded Todos): all 4 `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` image-pin occurrences (lines 52, 501, 864, 868) already match the current pin recorded in 34-03's object-store sweep (34-EVIDENCE.md row 35) -- clean, not re-derived; class 4/direct check (mandated, D-04 producing command): `use paladin::paladin_ports::output::file_storage_port::{FileStoragePort, UploadOptions};` (lines 121, 570) and `use paladin::paladin_ports::output::file_storage_port::*;` (lines 261, 634) reproduced against a throwaway `cargo check --example --features cli,s3-storage`: `error[E0433]: cannot find \`paladin_ports\` in \`paladin\`` -- **broken**; the live crate is a top-level sibling crate, `paladin_ports::output::file_storage_port::...` (no `paladin::` prefix), never nested under the facade; `use paladin::infrastructure::adapters::file_storage::minio::MinioAdapter;` (lines 120, 569) DOES compile (`src/infrastructure/adapters/file_storage/mod.rs` re-exports `paladin_storage::minio`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits | Phase 22 through Phase 33 -- no single REQ-ID applies; the broken `paladin::paladin_ports::` import shape recurs on 3 other pages this plan settles (redis-queue-adapter-setup.md, provider-expansion.md, sanctum-migration.md, port-trait-template.md), tracked item-level per D-03 | MB-50 | M |
| 30 | docs/src/appendix/performance-baseline.md | current | class 1 (version strings): `rustc 1.97.1 (8bab26f4f 2026-07-14)` / `cargo 1.97.1 (c980f4866 2026-06-30)` (lines 33-34, its most recent 2026-08-05 run header) match this run's own Measurement Header toolchain block exactly; class 4/direct check (mandated, D-04 producing command): all 3 benchmark ids the 2026-08-05 run scopes (`battalion/chain_of_command_2_levels_3_subordinates`, `_2_levels_5_subordinates`, `_wide_10_subordinates`) confirmed present verbatim at `crates/paladin-battalion/benches/battalion_benchmarks.rs:176,196,216`; the page is a meticulous, self-documenting, append-only measurement log across three dated runs (2026-05-27, 2026-08-02, 2026-08-05) that explicitly states its own scope-limiting caveats ("not merged", "no cross-run delta is computed") rather than overclaiming currency; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (benchmark ids predate Phase 22-33) | — | — | — |
| 31 | docs/src/appendix/port-trait-template.md | **stale** | class 4/direct check (mandated, D-04 producing command): this is a generic rustdoc-structure template (`PortTrait`, `port_name`, `AdapterName1` placeholders) whose own illustrative `## Examples` blocks (lines 48, 60) use `use paladin::paladin_ports::output::port_name::PortTrait;` -- the same broken `paladin::paladin_ports::` double-nesting confirmed non-compiling on 3 other pages this plan settles (row 29's evidence); reproduced against the same throwaway `cargo check --example`: `error[E0433]: cannot find \`paladin_ports\` in \`paladin\`` -- a template that instructs every future port's rustdoc author to copy a broken import pattern; class 1 (version strings): none; class 9 (shipped-surface): 0 hits (this is a template, not a description of a specific shipped item) | Phase 22 through Phase 33 -- no REQ-ID applies (a pre-milestone documentation-template defect, D-00g); shares rows 29's/32's/33's broken-import root cause (their own MB ID(s) cells above) | MB-51 | S |
| 32 | docs/src/appendix/provider-expansion.md | **stale** | class 1 (version strings): line 559 "**Last Updated:** January 2026", line 560 "**Version:** 0.1.0" -- live workspace version `0.10.0` (`Cargo.toml`) -- **mismatch**; the top "Provider Comparison" table (lines 36-43) shows only 3 providers (OpenAI/DeepSeek/Anthropic columns) though the page's own later "Streamed usage" table (lines 115-134) correctly discusses all 9 shipped providers including Grok/Kimi/Qwen/Ollama/Gemini (Phase 17 PROV-01..04) -- the page is a patchwork of an original 3-provider-era table left unrefreshed alongside newer sections; class 4/direct check (mandated, D-04 producing command): `use paladin::infrastructure::adapters::llm::openai_adapter::OpenAILlmAdapter;` (line 201) and `use paladin::infrastructure::adapters::llm::{deepseek_adapter::...}`/`{anthropic_adapter::...}` (lines 214, 234) reproduced against a throwaway `cargo check --example`: `error[E0432]: unresolved import \`paladin::infrastructure::adapters::llm::openai_adapter\`` -- **broken**; `src/infrastructure/adapters/llm/mod.rs`'s own doc comment states "All LLM provider adapter implementations have been relocated to the \`paladin-llm\` workspace crate... Only the configuration bridge remains here" (`pub mod config_bridge;` is the module's only declaration) -- no backward-compat shim was kept for this path, unlike file_storage/sanctum; `use paladin::paladin_ports::output::llm_port::LlmPort;` (line 367) shares the same broken double-nesting as row 29/31; class 9 (shipped-surface): 0 hits | Phase 17 -- Kimi/Qwen/Grok/Ollama/Gemini/generic-OpenAI-compatible adapters shipped (PROV-01..04, predates Phase 22 but is the concrete contradiction) | MB-52 | L |
| 33 | docs/src/appendix/redis-queue-adapter-setup.md | **stale** | class 4/direct check (mandated, D-04 producing command): `use paladin::infrastructure::adapters::queue::redis::RedisQueueAdapter;` (line 100) DOES compile (`src/infrastructure/adapters/queue/mod.rs` re-exports `pub use paladin_storage::redis;` behind the `redis-queue` feature); `use paladin::paladin_ports::output::queue_port::QueuePort;` (line 101) does NOT -- reproduced against a throwaway `cargo check --example --features cli,redis-queue`: `error[E0433]: cannot find \`paladin_ports\` in \`paladin\`` -- **broken**, the same double-nesting defect as rows 29/31/32; `use paladin::core::base::entity::message::MessagePriority;`/`use paladin::core::platform::manager::queue_service::QueueError;` (lines 133, 242) both confirmed compiling; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 25 -- `redis-cache` Cargo feature on `paladin-storage` shipped in an adjacent context (FT-06, SS-33); no REQ-ID directly names the queue adapter's import-path drift | MB-53 | M |
| 34 | docs/src/appendix/release-automation.md | **stale** | class 6 (workflow/job names), direct check (mandated, D-04 producing command): all nine `name:` display strings quoted ("Verify Tag From Main", "Test Suite", "Create Release", "Build and Push Docker Images", "Build Binaries", "Pre-Publish Consistency Gate", "Generate SBOM", "Finalize Release Body", "Publish to crates.io") reproduced exactly via `awk` over `.github/workflows/release.yml`'s job/`name:` pairs -- **9/9 match**; class 5 (make targets): `make publish-dry-run` confirmed at `Makefile:567`, `cargo install --locked cargo-release` confirmed as the recommended install path; but the "Known operational caveats" bullet (lines 149-153) states "`publish-crates` depends only on `test` and `create-release`" -- live `publish-crates:` job (`release.yml:602-609`) declares `needs: [test, create-release, check-release-consistency]` -- **the claim omits the third dependency, `check-release-consistency` (the Pre-Publish Consistency Gate)**; the operator-guide's 8-step flow (lines 106-133), the `RELEASE_ALLOW_ANY_BRANCH` bypass description, and the dry-run (`gh workflow run release.yml -f dry_run=true`, `make publish-dry-run`) sections are all confirmed accurate; class 9 (shipped-surface): 0 hits | Phase 29 -- release-gate composition (SHIP-02, `check-release-consistency` job); the release automation itself predates Phase 22-33 (D-00g) | MB-54 | S |
| 35 | docs/src/appendix/release-checklist.md | current | class 3 (crate names)/direct check (mandated, D-04 producing command): the "twelve publishable crates" claim and canonical publish order (`paladin-ai-core`, `paladin-ports`, `paladin-herald`, the 7-crate leaf tier, `paladin-eval`, `paladin-ai`) reproduced against live `ls crates/` (11 library crates + facade = 12) and each crate's `Cargo.toml` `name`/`publish` field -- `paladin-doc-examples` confirmed `publish = false` ("skipped automatically", matches); class 6 (workflow/job names): `publish-crates` job and the `crates-io` GitHub Environment confirmed live at `release.yml:602,606`; class 5 (make targets): `make publish-dry-run` confirmed at `Makefile:567`; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (checklist process, not a named shipped item) | — | — | — |
| 36 | docs/src/appendix/release-recovery.md | current | class 7 (error types)/direct check (mandated, D-04 producing command): every gate-failure code the page documents (`MISMATCH`, `ZERO_PACKAGES`, `MISSING_TAG`, `CHANGELOG_MISMATCH`, `CI_MISMATCH`, `CI_LOOKUP_FAILED`, `MISSING_SHA`, the four combined-failure codes) confirmed present verbatim in `scripts/check-release-consistency.sh` via `grep -oE`; class 4 (module/source paths): `scripts/check-release-consistency.sh` confirmed present; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (release-recovery process, not a named Phase 22-33 item) | — | — | — |
| 37 | docs/src/appendix/sanctum-benchmarks.md | **stale** | class 3 (crate names)/direct check (mandated, D-04 producing command): line 17 states "**Qdrant Adapter** (future): < 500ms search latency" and the page's own "## Qdrant Adapter (Future Benchmarks)" section (lines 212-219) opens "When the Qdrant adapter is implemented, additional benchmarks will measure..." -- but `crates/paladin-memory/src/sanctum/qdrant_adapter.rs` confirmed present and the `qdrant` Cargo feature is shipped (PROJECT.md: "Epic 11 summary's 'Qdrant DEFERRED' record is stale — Qdrant shipped") -- **Qdrant has been implemented for the entire milestone**, this framing is stale; `cargo bench --bench sanctum_benchmarks` confirmed against `crates/paladin-memory/benches/sanctum_benchmarks.rs` (`ls crates/paladin-memory/benches/`) -- checked, matches; the page's own footer "**Last Updated**: TBD" and "**Benchmark Version**: Initial implementation" confirm it was never filled in with real numbers; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Milestone 2-3 -- Qdrant Sanctum adapter shipped ("Milestone 2-3 as-shipped ledger"; no Phase 22-33 REQ-ID, predates this milestone per D-00g) | MB-55 | M |
| 38 | docs/src/appendix/sanctum-deployment.md | current | class 4/direct check (mandated, D-04 producing command): `use paladin::infrastructure::adapters::sanctum::InMemorySanctum;` (line 48) confirmed compiling (`src/infrastructure/adapters/sanctum/mod.rs` re-exports `pub use paladin_memory::sanctum::InMemorySanctum;`); `InMemorySanctum::new()` (line 52) confirmed matching the live zero-arg constructor; class 2 (dependency pins): `qdrant/qdrant:v1.7.4` image tags (lines 91, 145, 251) are illustrative deployment-example tags with no authoritative current-version source in the tree to contradict them against (the project's own `docker/docker-compose.yml:180` pins `qdrant/qdrant:latest`, not a specific version) -- following the 34-04 `docker.md`/`kubernetes.md` precedent, an illustrative example tag is not itself a factual currency claim; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` -> 0 hits (Sanctum/Qdrant predates Phase 22-33) | — | — | — |
| 39 | docs/src/appendix/sanctum-migration.md | **stale** | class 4/direct check (mandated, D-04 producing command): `use paladin::infrastructure::adapters::sanctum::QdrantSanctumAdapter;` (lines 169, 240, 385) confirmed compiling; `use paladin::paladin_ports::output::sanctum_port::{SanctumPort, SanctumQuery};` (line 241) and `use paladin::paladin_ports::output::{SanctumPort, EmbeddingPort};` (line 386) reproduced against a throwaway `cargo check --example`: `error[E0433]: cannot find \`paladin_ports\` in \`paladin\`` -- **broken**, the same double-nesting defect as rows 29/31/32/33; class 2 (dependency pins): `qdrant/qdrant:v1.7.4` image tags (lines 119, 343, 368) are illustrative example tags per row 38's precedent, not independently flagged; class 9 (shipped-surface): 0 hits | Phase 22 through Phase 33 -- no REQ-ID applies; shares rows 29's/31's/32's/33's broken `paladin::paladin_ports::` import root cause (their own MB ID(s) cells above) | MB-56 | M |
| 40 | docs/src/appendix/security-scanning.md | **stale** | class 6 (workflow/job names): `security-audit`, `cargo-deny`, `osv-scanner` job names (Tooling Overview table, lines 10-15) confirmed live in `ci.yml`'s job enumeration (34-EVIDENCE.md row 62); class 2 (dependency pins)/direct check (mandated, D-04 producing command): the page's "Snyk Evaluation & Decision" section (lines 95-122) states "**Decision: Deferred**" with a comparison table rating Snyk "Partial"/"Limited on free tier" coverage -- but `.github/instructions/security.instructions.md` (dated 2026-08-18) records the actual, measured decision: Snyk was **evaluated and removed**, with Snyk Code (SAST) returning **0 findings** on a 4-class planted-vulnerability probe and Snyk Open Source (SCA) returning **0 supported target files** for the Cargo workspace (`SNYK-CLI-0008`) -- "Deferred"/"Partial" directly contradicts the measured "removed, zero Rust coverage" verdict; the page **entirely omits** the Rust-SAST/CodeQL question -- CodeQL was evaluated, provably analyses 100% of 385 first-party `.rs` files, and was itself disqualified as a required-check-grade Rust SAST (retained advisory-only, per `.github/workflows/codeql.yml` and PROJECT.md's v0.9.0 summary) -- no mention of this on a page whose own scope ("Milestone 10 — CI Hardening... Epic 2") predates the SAST evaluation (v0.9.0, Phases 18-21); "Current tracked exceptions" (lines 81-86) lists only **2** advisories (`RUSTSEC-2023-0071`, `RUSTSEC-2025-0111`) -- live `.cargo/audit.toml`'s `ignore = [...]` array holds **5** (`grep -c` confirms `RUSTSEC-2026-0187`/`-0194`/`-0195` are also present and missing from the page); class 9 (shipped-surface): 0 hits | v0.9.0 (Phases 18-21) -- the Rust-SAST evaluation and CodeQL advisory-only disposition (SAST-01..04); the Snyk removal decision (2026-08-18, `.github/instructions/security.instructions.md`) | MB-57 | L |
| 41 | docs/src/appendix/sentinel.md | **stale** | class 4/direct check (mandated, D-04 producing command): `use paladin::infrastructure::adapters::llm::OpenAiAdapter;` (line 188) and `use paladin::infrastructure::adapters::llm::AnthropicAdapter;` (line 216) reproduced against a throwaway `cargo check --example`: `error[E0432]: no \`OpenAiAdapter\` in \`infrastructure::adapters::llm\`` / `no \`AnthropicAdapter\` in \`infrastructure::adapters::llm\`` -- **both broken**, the same relocated-with-no-shim defect confirmed on provider-expansion.md (row 32, `src/infrastructure/adapters/llm/mod.rs` only exposes `config_bridge`); the live re-export is `paladin::OpenAIAdapter`/`paladin::AnthropicAdapter` at the facade root (`src/lib.rs:181,187`) or `paladin_llm::openai::OpenAIAdapter` directly; `use paladin::core::platform::container::vision::{VisionContent, ImageDetail};` (line 119) and `use paladin::application::services::paladin::paladin_builder::PaladinBuilder;` (line 253) both confirmed compiling; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 17 through Phase 26 -- no single Phase 22-33 REQ-ID applies (vision predates the milestone, D-00g); shares row 32's relocated-adapter-no-shim root cause (its own MB ID cell above) | MB-58 | M |
| 42 | docs/src/appendix/user-rest-api.md | **stale** | class 4/direct check (mandated, D-04 producing command): every CLI example on the page (`./paladin user register/login/get/update/list/activate/verify`) invokes a `paladin user ...` subcommand -- a full read of `src/bin/paladin-cli.rs`'s `Commands` enum (12 variants: Agent, Battalion, Arsenal, Maneuver, Onboarding, SetupCheck, Features, Muster, Eval, Graph, Run, Council) confirms **no `User` variant exists** -- the entire CLI surface this page documents does not exist in the shipped binary; the page is itself malformed as a docs/src page (opens as a raw markdown code block, ends mid-Rust-source with an unterminated string literal and no closing structure) -- a leaked implementation-planning artifact rather than maintained documentation; class 1 (version strings): none; class 9 (shipped-surface): 0 hits | Phase 22 through Phase 33 -- no REQ-ID applies (pre-milestone artifact, D-00g); PROJECT.md's WEB-01/WEB-02 confirm the auth/user-store mechanism remains genuine forward work, contradicting this page's own "production-ready" claim | MB-59 | L |
| 43 | docs/src/appendix/user-system.md | **stale** | class 3 (crate names)/direct check (mandated, D-04 producing command): the page's own "CLI Module Implementation" section (lines 37-49) claims "Re-enabled CLI module: Successfully integrated CLI with the main library" and lists `register`/`login`/`get`/`update`/`list`/`activate`/`deactivate`/`verify` as shipped CLI commands -- the same full read of `src/bin/paladin-cli.rs`'s `Commands` enum used for row 42 confirms **no `User` CLI subcommand exists in the live binary** -- this "Completion Summary" page's central claim is false against the shipped tree, though the underlying domain/service/repository layers (`crates/paladin-storage/src/sqlite_user_repository.rs`, `crates/paladin-core/src/platform/manager/user_service.rs`) do exist (`grep -rln`); class 2 (dependency pins): `clap = { version = "4.5.40", features = ["derive"] }` (line 39) matches live `Cargo.toml:188` (`clap = { version = "4.5.40", features = ["derive", "cargo", "env"], ... }`) exactly -- this claim alone is current; class 9 (shipped-surface): 0 hits | Phase 22 through Phase 33 -- no REQ-ID applies (pre-milestone artifact, D-00g); shares row 42's false-CLI-surface finding (its own MB ID cell above) | MB-60 | L |
| 44 | docs/src/architecture/commissary.md | current | class 4 (module/source paths): `crates/paladin-llm/src/services/commissary.rs` confirmed present, and the two cited line ranges (`644-698`, `911-929`) checked against the live file's own test helpers; class 7 (error types): all 5 `CommissaryError` variants named on the page (`UndeclaredContextWindow`, `ReservationExceedsWindow`, `FixedMaterialExceedsAllowance`, `ContextOverflow`, `InvalidConfig`) — `grep -n '^    [A-Z][A-Za-z]* {$\|^    [A-Z][A-Za-z]*(' crates/paladin-llm/src/services/commissary.rs` reproduces exactly these 5, no more, no fewer; direct check: `Commissary::new(provider, capabilities, counter, config)` signature (page lines 75-80) matches `sed -n '344,349p' crates/paladin-llm/src/services/commissary.rs` exactly (4 args, no `is_exact_counter`, per Phase 32 PRIM-02); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `Commissary`, `RagRetrievalResult`, `RagRetrievalError`, `with_token_counter`, `resolve_context_window` (via cross-reference) — all correct. **One pre-existing finding, not duplicated here:** line 7's literal `Quartermaster` match is recorded as its own row above (§2 Vocabulary sweep subsection, "stale content", historical ADR-pointer) — this page's own currency verdict is otherwise `current`; the page is judged `current` overall because that one line is a deliberate historical citation, not a content error, per D-00c (Phase 35's call) | — | — | — |
| 45 | docs/src/architecture/crate-map.md | **stale** | class 1 (version strings): line 9 "paladin-ai (root umbrella, v0.5.0)" — live `0.10.0` — **mismatch**; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 crates named, line 3 says "nine workspace crates" — live `ls crates/` → 11 library crates (`paladin-eval`, `paladin-herald` both absent) — **mismatch**; class 4 (module/source paths): `paladin-llm` feature table (lines 152-162) lists only `openai`/`anthropic`/`deepseek`/`mock`/`openai-embeddings`/`vision` — live `crates/paladin-llm/Cargo.toml` `[features]` also has `kimi`/`qwen`/`grok`/`ollama`/`openai-compatible`/`gemini` (Phase 17) — **6 features undocumented**; direct check (mandated, D-03 crate-graph edge): the mermaid dependency graph (lines 23-62) has no `mem --> llm` edge, though the prose immediately below (lines 174-176) correctly states the Phase 33 COMM-01 dependency — the diagram and the prose disagree with each other, and the diagram is what a reader actually parses; class 9: `grep -nFf 34-shipped-tokens.txt` → 0 hits | Phase 33 — `paladin-memory` gains an unconditional dependency on `paladin-llm` (COMM-01, SS-90) | MB-13 | L |
| 46 | docs/src/architecture/design-patterns.md | **stale** | class 1 (version strings): none on this page; class 4 (module/source paths): pattern 5's `PaladinExecutionService::new` code sample (lines 137-144) shows `new(llm, circuit_breaker, garrison, herald: Option<Arc<dyn Herald>>)` — live signature at `src/application/services/paladin/paladin_execution_service.rs:361-365` is `new(llm_port, circuit_breaker, garrison: Option<Arc<dyn GarrisonPort>>, arsenal: Option<Arc<dyn ArsenalPort>>)` — the fourth parameter is `arsenal`, not `herald` — **mismatch**, confirmed by direct read of the live constructor and its own doctest (which the page's pattern 1/2 code otherwise closely tracks); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this page's patterns predate Phase 22-33 and none of its code samples were touched by that work, so this is a pre-existing, not newly introduced, gap per D-00g) | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; no Phase 22-33 REQ-ID applies — the constructor's `arsenal` parameter predates this milestone) | MB-12 | S |
| 47 | docs/src/architecture/domain-model.md | **stale** | class 3 (crate names): the D-10 ubiquitous-language table (lines 12-32) confirmed current — `Commissary` present at line 30, matching the D-10 confirmation already recorded in §1; class 4 (module/source paths): "Core Domain Entities" (lines 63-159) documents `Paladin`/`Garrison`/`Arsenal`/`Citadel`/`Herald` from `crates/paladin-core/src/platform/container/`, but omits `Battlefield`, `Waypoint`, `Aegis` and `TraceRecord` — all four live in the exact same directory (`crates/paladin-core/src/platform/container/{battlefield,waypoint,aegis,trace}.rs`, confirmed via `grep -rln 'pub struct WarEngine\|pub struct Battlefield\|pub struct Waypoint\b\|pub struct TraceRecord\|pub struct Aegis\b' crates/`) — this page's own stated scope, "describes all domain entities in `paladin-ai-core`", is violated by the omission; the `GarrisonEntry` snippet (lines 96-104) is separately recorded as stale above (§2 Vocabulary sweep / Phase 31 D-29 subsection — cross-referenced, not duplicated); class 9: `grep -nFf 34-shipped-tokens.txt` → hits on `Commissary` (line 30) only, none of `WarEngine`/`Waypoint`/`Aegis`/`TraceRecord` | Phase 22 — `Waypoint`, a full `Battlefield` snapshot persisted automatically after every superstep (ENG-03, SS-02) | MB-11 | L |
| 48 | docs/src/architecture/hexagonal-design.md | **stale** | class 1 (version strings): none on this page; class 4 (module/source paths): the `LlmPort` trait code sample (lines 32-47) shows `async fn generate(&self, messages: &[Message], config: &LlmConfig) -> Result<LlmResponse, LlmError>` — live `crates/paladin-ports/src/output/llm_port.rs:1287-1304` is `async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>`, a single builder parameter, not `(messages, config)` — **mismatch**, confirmed by direct read of the trait definition and its own doctest at the same location; `GarrisonPort`/`SanctumPort`/`ArsenalPort`/`FileStoragePort` samples (lines 52-93) not independently re-verified this pass (out of scope for the confirmed defect above); class 9: `grep -nFf 34-shipped-tokens.txt` → 0 hits (pre-v0.10.0 API, D-00g) | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; the `LlmRequest` builder predates this milestone, no Phase 22-33 REQ-ID applies) | MB-10 | M |
| 49 | docs/src/architecture/overview.md | **stale** | class 3 (crate names): line 3 "nine focused crates", table lines 15-24 lists 9 infra/core crates + root — live `ls crates/` → 11 library crates + `doc-examples` + root facade; `paladin-eval` (Phase 28) and `paladin-herald` both absent from the table — **mismatch, first finding, ID in this row's MB ID(s) cell**; class 4 (module/source paths): the page never names `crates/paladin-eval` or `crates/paladin-herald` (`test -d` confirms both exist); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits — the page, as the primary architecture entry point, never names `WarEngine`, `Battlefield`, `Waypoint`, `Aegis`, `VaultPort`, `Commissary`, `TraceRecord`, or any Platform API route despite each shipping in Phases 22-28/30/33 — checked, confirmed absent by the same grep that finds them on `commissary.md`/`platform-api.md`; direct check: "Technology Stack" table (line 240) correctly states "MSRV 1.88" (matches `rust-toolchain.toml`), so the page was partially touched post-Phase-22.1 but never for the architectural additions — **mismatch, second finding, ID in this row's MB ID(s) cell** | Phase 28 — `paladin-eval` crate (OBS-04, SS-58); Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) | MB-08, MB-09 | M, L |
| 50 | docs/src/contributing/architecture-decisions.md | **stale** | class 1 (version strings): line 636 "0.1.0" (illustrative semver-bump example, not a fact claim); class 4 (module/source paths): `Port Architecture`/`LLM Adapter Development`/`Garrison Adapter Development`/`Arsenal Adapter Development`/`Citadel Adapter Development` sections all describe real hexagonal-architecture concepts accurately; direct check: the page's own title (`# Adapter Development Guide`, line 1) and its TOC (Overview, Port Architecture, LLM/Garrison/Arsenal/Citadel Adapter Development, Testing Adapters, Publishing Adapters) are entirely about writing adapters for existing ports — `grep -ic 'ADR\|architectural decision' architecture-decisions.md` → **0** — the page never once discusses an architecture decision or ADR, despite occupying the `docs/src/SUMMARY.md:81` nav slot titled "Architecture Decisions" — **mismatch, ID in this row's MB ID(s) cell**; this repository now has real ADRs (ADR-0033, ADR-0041, ADR-0042, ADR-0048…ADR-0051 named elsewhere in this corpus) with no page indexing or explaining any of them; the page's actual subject (adapter development) substantially overlaps `contributing-providers.md`'s LLM-specific scope (row 52); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits | pre-existing content/nav-title mismatch (D-00g: shipped tree — here, the nav's own title — outranks the document; no Phase 22-33 REQ-ID applies, this predates the milestone) | MB-35 | L |
| 51 | docs/src/contributing/branching-model.md | current | class 1 (version strings): none; class 5 (make targets): `make release VERSION=x.y.z` — `grep -n '^release:' Makefile:581` confirmed present, matching the described lockstep-bump/changelog/tag/push behavior; class 6 (workflow/job names): the trigger-policy register table (one row per `.github/workflows/*.yml` file) cross-checked against `ls .github/workflows/` — all 7 files present (`ci.yml`, `feature-flags.yml`, `pre-commit.yml`, `docs.yml`, `release.yml`, `benchmarks.yml`, `codeql.yml`) and `scripts/check-workflow-triggers.sh` confirmed present (`test -f`); direct check: `codeql.yml`'s row correctly cites "Evaluated and disqualified as a required-check-grade Rust SAST at CodeQL 2.26.3 / rust-queries 0.1.40 (2026-08-25)... retained deliberately advisory" — matches `.github/instructions/security.instructions.md`'s "Known gap: no Rust SAST" section verbatim; this page's object-store reference (34-03's sweep: 1 hit, "in prose") was already checked clean in 34-03's Task 1 — cited here, not re-derived; `ci.yml`'s `push: branches: ['**']` (line 13-14) confirmed matching the page's claimed match-all trigger; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this page is entirely about git workflow, not a Phase 22-33 capability) | — | — | — |
| 52 | docs/src/contributing/contributing-providers.md | current | class 1 (version strings): line 318 "Streaming Usage Terminal-Chunk Contract (v0.10.0, ACCT-03)" — this is a correct, current-milestone-aware citation, not a stale pin — matches Phase 31 SS-73's `TokenUsage` addition and the live workspace version; class 4 (module/source paths): `crates/paladin-llm/src/myprovider/mod.rs` (illustrative), `crates/paladin-llm/src/lib.rs`, `crates/paladin-llm/src/provider_factory.rs`, `crates/paladin-llm/src/conformance.rs` (`streaming_usage_equals_non_streaming_usage`), `crates/paladin-llm/src/openai_compatible/adapter.rs` all confirmed present; class 8 (feature flags): `llm-myprovider = ["paladin-llm/myprovider"]` pattern matches the live `[features]` convention (`llm-kimi`/`llm-qwen`/etc. all follow this exact `llm-<name> = ["paladin-llm/<name>"]` shape in `Cargo.toml`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 direct hits (no §1 SS-nn token matched, but the ACCT-03 citation is independently correct per the class-1 check above) | — | — | — |
| 53 | docs/src/contributing/development-setup.md | current | class 1 (version strings): lines 679, 802, 883, 886-887, 891 are illustrative semver-bump examples (`v0.4.0-rc.1`, `0.1.0`→`0.2.0`→`1.0.0`) inside a "how semver bumps work" walkthrough, not factual current-version claims; class 5 (make targets): `make dev`, `make hooks`, `make security`, `make sbom`, `make release`, `make publish-dry-run`, `make deny`, `make audit`, `make test-integration-docker` — all 9 distinct targets confirmed present via `grep -n '^<target>:' Makefile`; class 6 (workflow/job names): `gh workflow run release.yml -f tag=v0.4.0-rc.1 -f dry_run=true` cross-checked against `release.yml`'s own `workflow_dispatch.inputs` block (lines 7-17: `tag` (required string), `dry_run` (optional boolean, default false)) — **matches exactly**, both input names and types; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (contributor-workflow content, not a Phase 22-33 capability) | — | — | — |
| 54 | docs/src/contributing/testing-guide.md | **stale** | class 1 (version strings): none; class 5 (make targets): `make test-all`, `make test-integration-docker`, `make services-up`, `make coverage`, `make coverage-html` all confirmed present in `Makefile`; **the Code Coverage section subsection below records the full 3-way comparison this plan requires** — see "Coverage command comparison (Folded Todos)" immediately after this table; class 6 (workflow/job names): the "CI Integration → GitHub Actions Workflow" section (lines 628-686) shows a complete `name: Tests` / `.github/workflows/test.yml` sample — `test -f .github/workflows/test.yml` → **does not exist**, and `ls .github/workflows/` confirms no such file ever existed alongside the 7 real files — **mismatch, second finding, ID in this row's MB ID(s) cell** — the same class of fabrication `cicd.md`'s own two "Corrected 2026-08-24" callouts already fixed elsewhere on `docker-publish.yml`/`security.yml`, but never applied here; the fabricated sample also uses the deprecated `actions-rs/toolchain@v1` action (real `ci.yml` uses `dtolnay/rust-toolchain@stable`) and shows no `--fail-under-lines` enforcement at all; this page's object-store reference (34-03's sweep: 2 hits, both the pinned `quay.io/minio/minio:RELEASE.2025-09-07...` image) was already checked clean in 34-03's Task 1 — cited here, not re-derived; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 1 hit (`fail-under-lines 82`, line 448) — present but, per the coverage-comparison subsection below, the surrounding command is itself one flag short of the real invocation | Phase 29 — 82% workspace line coverage floor, `cargo llvm-cov --fail-under-lines` (SHIP-04, SS-65); the fabricated CI Integration section is pre-existing drift (D-00g), no REQ-ID applies | MB-36 | L |
| 55 | docs/src/deployment-topologies/battalion-orchestration.md | current | class 1 (version strings): none; class 4 (module/source paths): `crates/doc-examples/src/orchestration.rs:phalanx` `{{#include}}` anchor mechanically verified by the build-baseline doc-examples gate (34-03); the pattern comparison table (Formation/Phalanx/Campaign/Chain of Command/Commander mapped to their `*ExecutionService`/`CommanderBuilder` types) matches `orchestration.md`'s own (this same plan, row 87) accurate service-type list; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this topology page predates Phase 22-33, pattern selection is pre-milestone content) | — | — | — |
| 56 | docs/src/deployment-topologies/embedded-library.md | current | class 1 (version strings): none; class 4 (module/source paths): `crates/doc-examples/src/readme.rs:quickstart` and `crates/doc-examples/src/deployment_topologies.rs:embedded_registry` `{{#include}}` anchors mechanically verified by the build-baseline doc-examples gate; the page's own claim ("compiled examples... guaranteed to match the current API") is backed by that same gate; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (pre-milestone `PaladinBuilder` composition-root content) | — | — | — |
| 57 | docs/src/deployment-topologies/http-service-host.md | current | class 1 (version strings): none; class 4 (module/source paths): every route in the endpoint table (`POST /agents/{id}/execute`, `POST /agents/{id}/execute/stream`, `POST /agents/{id}/jobs`, `GET /agents/{id}/jobs/{job_id}`, `GET /agents`, `GET /agents/{id}`, `POST /agents`, `DELETE /agents/{id}`, `GET /health`, `GET /ready`, `GET /openapi.json`, `GET /docs`) cross-checked against `crates/paladin-web/src/agent_controller.rs`'s own module-doc route table (lines 8-16) — **matches exactly**, byte-for-byte route paths; direct check: `crates/doc-examples/src/http_service_host.rs:http_host` `{{#include}}` anchor mechanically verified; `k8s/server/` confirmed present (`ls k8s/server/`); ADR-0041's shared-`AuthPort` deferral correctly described (single-replica-scoped in-process token store); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 1 hit (`PaladinResult`, line 55) — checked, matches | — | — | — |
| 58 | docs/src/deployment-topologies/overview.md | current | class 1 (version strings): none; class 4 (module/source paths): the five-topology comparison table's "Key crates / features" column (`paladin-ai`, `paladin-battalion`, `paladin-server`/`web-server`, `paladin-storage`/`redis-queue`) all confirmed live; direct check: the "Capability note — Garrison and Arsenal" callout (HTTP host carries neither) matches `http-service-host.md`'s own equivalent callout (row 57) exactly, no contradiction between the two pages; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (topology-selection content, orthogonal to the Phase 22-33 capability axis) | — | — | — |
| 59 | docs/src/deployment-topologies/queue-worker.md | current | class 1 (version strings): "Platform API (v0.10, ...)" (line 53) — correctly current, matching the live workspace version; class 4 (module/source paths): `crates/doc-examples/src/queue_worker.rs:queue` `{{#include}}` anchor mechanically verified; direct check: `RunWorkerPool`, `DbCancellationProbe`/`CancellationProbe`, `LocalRunTokens` all confirmed present at `src/application/services/run/mod.rs:8-44` and `cancel.rs`; `k8s/server/worker-deployment.yaml` confirmed present (`test -f`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `POST /runs` (`POST /v1/runs` family, SS-44), `RunQueuePort` (SS-51), `APP_RUN_STORE_BACKEND` (SS-52) — checked, matches; the `APP_RUN_QUEUE_BACKEND`/`APP_RUN_WORKER_CONCURRENCY` env vars named (lines 59-60) are not literal §1 SS-nn tokens but are directly confirmed by the live `RunWorkerPool`/config wiring above | — | — | — |
| 60 | docs/src/deployment-topologies/sidecar.md | current | class 1 (version strings): none; class 4 (module/source paths): `crates/doc-examples/src/sidecar.rs:sidecar_client` `{{#include}}` anchor mechanically verified, uses the live `reqwest` API per the page's own claim; direct check: the "Paladin ships no IPC / gRPC / RPC / sidecar transport" disclaimer confirmed accurate — no `RemoteAgentPort` or equivalent trait exists anywhere in `crates/paladin-ports/src/` (`grep -rn 'RemoteAgentPort' crates/paladin-ports/src/` → 0 hits); server-side route reference (`POST /v1/agents/{id}/execute`) matches `http-service-host.md`'s (row 57) confirmed-live route; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits | — | — | — |
| 61 | docs/src/deployment/cicd.md | **stale** | class 1 (version strings): none; class 6 (workflow/job names): the "Workflow Structure" file listing (lines 30-40: `benchmarks.yml`, `ci.yml`, `docs.yml`, `feature-flags.yml`, `pre-commit.yml`, `release.yml`, `dependabot.yml`) is missing `codeql.yml` — `ls .github/workflows/` → 7 files including `codeql.yml` (added 2026-08-25) — **mismatch, first finding**; the "CI Pipeline → ci.yml" YAML sample (lines 46-142) shows only 3 jobs (`lint`, `test`, `coverage`) — live `ci.yml` has **~26 jobs** (`grep -n '^  [a-z][a-z_-]*:$' ci.yml`: `lint`, `actionlint`, `security-audit`, `cargo-deny`, `osv-scanner`, `api-surface`, `msrv`, `semver`, `test`, `examples`, `crate-isolation`, `integration-tests`, `docker-integration`, `ollama-integration`, `postgres-integration`, `redis-cache-integration`, `redis-queue`, `sdk-clients`, `coverage`, `cli-tests`, `bench-check`, `docker`, `kubernetes-smoke`, `benchmark-regression-signal`, `publish-dry-run`, plus two unnamed matrix jobs) — the shown `coverage` job's body also omits `--fail-under-lines` entirely — **mismatch, second finding, ID in this row's MB ID(s) cell**; the "Release Pipeline → release.yml" sample (lines 177-274) shows a `build-release`/`create-release` job pair — live `release.yml` has **no `build-release` job** (real jobs: `verify-tag-source`, `test`, `create-release`, `build-docker`, `build-binaries`, `check-release-consistency`, `sbom`, `finalize-release-body`, `publish-crates`), and the sample never mentions `verify-tag-source` (the D-21 two-SHA-rule tag-source guard) at all — **mismatch, third finding**; the "Security Scanning" section (lines 346-378) IS already correctly corrected ("Corrected 2026-08-24... No such workflow exists... Do not reintroduce a Snyk step") and its 3 real job names (`security-audit` at `ci.yml:83`, `cargo-deny` at `ci.yml:103`) both confirmed exact; `osv-scanner` cited at `ci.yml:155` vs live `ci.yml:164` (9-line drift, minor); the "Docker Build Pipeline" section is also already correctly corrected (matches `release.yml:21` `REGISTRY: ghcr.io` exactly); class 5 (make targets): `make audit`/`make deny`/`make security`/`make sbom` (lines 369-372) all confirmed present; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits; this page's object-store reference (34-03's sweep: 1 hit, the pinned MinIO image in the integration-tests job sample) was already checked clean in 34-03's Task 1 — cited here, not re-derived | Phase 18 — `codeql.yml` Rust SAST, evaluated and retained advisory-only (2026-08-25, pre-milestone but post-dates the page's own last correction); Phase 29 — `cargo llvm-cov --fail-under-lines` CI gate (SHIP-04, SS-65), absent from the shown sample; no §1 row exists for `release.yml`'s job structure, cited against the live workflow directly | MB-31 | L |
| 62 | docs/src/deployment/docker.md | current | class 1 (version strings): illustrative `v0.8.0` image tags appear 6 times throughout the "Tagging Strategy"/example sections (lines 91, 105, 500, 534, 536, 551, 649) — these are example command output, never a "the current version is X" factual claim, so they do not cross the D-06 stale bar the way `getting-started/installation.md`'s explicit "current published workspace, v0.5.0" claims did (34-03); class 5 (make targets): `make docker-build-server` confirmed present (`Makefile:449-450`); class 6 (workflow/job names): the "Automated Multi-Arch Builds" section already carries its own correction ("There is no `.github/workflows/docker-publish.yml`... the real pipeline is split across `ci.yml`'s `docker` job (builds, does not push) and `release.yml`'s `build-docker` job") — direct check against live `ci.yml`'s `docker` job (lines 1604-1650) confirms `push: false`, `platforms: linux/amd64,linux/arm64` exactly matching the page's claim; this page's object-store references (34-03's sweep: 1 hit, the pinned MinIO image) were already checked clean in 34-03's Task 1 — cited here, not re-derived; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 2 hits (`max_tokens`, config.yml samples lines 207, 769) — checked, matches | — | — | — |
| 63 | docs/src/deployment/kubernetes.md | current | class 1 (version strings): illustrative `v0.8.0` image tags in a "production shape" sample and a Helm `values.yaml` sample (lines 206, 518) — example content, not a factual current-version claim; class 4 (module/source paths): the "Scope note" (lines 33-42) enumerates the real `k8s/` manifests — `ls k8s/` confirms `namespace.yaml`, `deployment.yaml`, `service.yaml`, `configmap.yaml`, `secret.yaml.example`, `redis.yaml`, `minio.yaml`, plus `k8s/server/` — **matches exactly**, all 8 present; `k8s/deployment.yaml:72`'s `image: paladin:test` confirmed matching the page's own claim; `k8s/server/worker-deployment.yaml` confirmed present; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 9 hits — `APP_ENGINE_SHUTDOWN_GRACE_SECS`/`APP_ENGINE_GRACEFUL_SHUTDOWN` (SS-21/SS-22), `APP_RUN_STORE_BACKEND` (SS-52), `ShutdownCoordinator` (SS-20) — all correct and cross-checked against `src/config/engine.rs` per the page's own citation | — | — | — |
| 64 | docs/src/deployment/production.md | current | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-llm/src/config/llm.rs:9-22` (`LlmProviderConfig`) directly read and confirmed byte-exact — all 6 named fields (`api_key`, `base_url`, `default_model`, `default_temperature`, `timeout_seconds`, `max_retries`) present, no extra "caching knobs" field exists (matching the page's own "no caching knobs" disclaimer); class 5 (make targets): `make audit`/`make deny`/`make security`/`make sbom` all confirmed present; direct check: `ShutdownCoordinator`/`EngineConfig`/`APP_ENGINE_SHUTDOWN_GRACE_SECS`/`APP_ENGINE_GRACEFUL_SHUTDOWN` graceful-shutdown section (lines 312-388) cross-checked against `kubernetes.md`'s (row 63) equivalent, byte-identical description — no contradiction between the two pages; class 6 (workflow/job names): `dependabot.yml` confirmed present (`test -f .github/dependabot.yml`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 8 hits — `ShutdownCoordinator`, `APP_ENGINE_SHUTDOWN_GRACE_SECS`, `APP_ENGINE_GRACEFUL_SHUTDOWN`, `PaladinResult`, `max_tokens` — all correct | — | — | — |
| 65 | docs/src/getting-started/configuration.md | current | class 8 (feature flags): all nine LLM providers documented with base URLs and env vars (`openai`/`anthropic`/`deepseek`/`kimi`/`qwen`/`grok`/`ollama`/`gemini`/`openai-compatible`), matching the live `Cargo.toml` `llm-*` flag set exactly; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `reasoning_agent`, `max_tokens` (the four-meanings table, matching Phase 30 VOCAB-05/SS-70 exactly), `Commissary` (Phase 33 RAG cap note, lines 283-284, 416) — all correct and current; class 4 (module/source paths): `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`, `src/application/services/paladin/middleware/limits.rs` (line 417-418) all confirmed present (`test -f`) | — | — | — |
| 66 | docs/src/getting-started/installation.md | **stale** | class 1 (version strings): line 12 "Rust \| 1.85.0 \| Latest stable (1.95+)", line 17 "Why Rust >= 1.85?", line 19 "should print >= 1.85.0" — live `rust-toolchain.toml`/`Cargo.toml` `rust-version = "1.88"` (Phase 22.1, X-11.2) — **mismatch**; lines 48-78 pin every crate to `"0.5.0"` — live `0.10.0` — **mismatch**; class 8 (feature flags): the "Feature Flag Profiles" table (lines 84-90) lists 5 flags (`llm-openai`, `redis-queue`, `s3-storage`, `openai-embeddings`, `qdrant`) — live `Cargo.toml` `[features]` has 20+ flags including `llm-kimi`/`llm-qwen`/`llm-grok`/`llm-ollama`/`llm-gemini`/`llm-openai-compatible`/`vision`/`content-processing`/`web-server`/`notifications`/`storage-postgres`/`redis-cache`/`otel`/`dev-ui` — **15+ shipped flags undocumented**; class 3: `paladin-ai-core` crate name itself is correctly spelled (matches `crates/paladin-core/Cargo.toml` `name = "paladin-ai-core"`); class 9: `grep -nFf 34-shipped-tokens.txt` → 1 hit (the stale `"llm-openai"` feature string, line 57) | Phase 22.1 — Workspace MSRV floor raised from 1.85 to 1.88 (X-11.2, SS-08) | MB-06 | L |
| 67 | docs/src/getting-started/quickstart.md | **stale** | class 1 (version strings): lines 24-26 pin `paladin-ai`/`paladin-ports`/`paladin-llm` to `"0.7.0"` — live workspace version `0.10.0` (`Cargo.toml` `[workspace.package] version`) — **mismatch**; direct check: the code sample's `PaladinExecutionService::new(llm_port, circuit_breaker, None, None)` call (line 61) matches the live 4-arg constructor exactly (`src/application/services/paladin/paladin_execution_service.rs:361-365`) — **current**; `result.usage.total_tokens` (line 67) matches the live `TokenUsage.total_tokens: u32` public field (`crates/paladin-core/src/platform/container/token_usage.rs:28`) — **current**; the "Understanding the Output" table (lines 114-120) correctly names `usage: TokenUsage` (Phase 31, ACCT-01/SS-73); class 4 (module/source paths): `src/main.rs` reference (lines 32, 35) is a generic illustrative path, not a repo-relative claim — not applicable; class 9: `grep -nFf 34-shipped-tokens.txt` → hits on the stale version pin plus the correct `TokenUsage` row | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` (`Cargo.toml [workspace.package]`); no matching §1 row exists for a bare version-pin fact, so this citation is the workspace-wide version bump itself, not a single SS-nn item | MB-07 | S |
| 68 | docs/src/introduction.md | **stale** | class 1 (version strings): none on this page; class 3 (crate names): none relevant to the term table; the "Medieval Military Theme" table (lines 78-91) carries 12 terms — `grep -rlni 'medieval military' docs/src/` (34-02's D-10 sweep) already found this a genuine fourth, partial vocabulary list, missing `Commissary`, `Sanctum`, `Sentinel`, `Quest`, `Conclave`, `Council`, `Grove` and `Commander` against the three confirmed-current lists (`.github/copilot-instructions.md`, `PROJECT.md`, `domain-model.md`) — **mismatch, first finding, ID in this row's MB ID(s) cell**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits — the page's five nav-index sections (User Guides, Architecture, Deployment, Operations, Contributing) never link `control-flow.md`, `fault-tolerance.md`, `parley-and-chronicle.md`, `agent-runtime.md`, `platform-api.md`, `observability.md`, `eval-harness.md`, `commissary.md` or any `deployment-topologies/` page — checked, confirmed absent from every "## " section's link list — **mismatch, second finding, ID in this row's MB ID(s) cell**; the Architecture-Layers three-layer description (lines 93-101) remains accurate and unchanged | Phase 30 — `Commissary` anchored (VOCAB-02, SS-68); Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) | MB-04, MB-05 | M, L |
| 69 | docs/src/operations/logging.md | current | class 1 (version strings): line 27 `log = "0.4.21"` — confirmed matching `Cargo.toml`'s real pin (`grep -n '^log = ' Cargo.toml` → `log = "0.4.21"`, at line 29; the page's own "Cargo.toml:14" citation has drifted from the live line 29 — a cosmetic line-number-only drift, the version string itself is correct); class 4 (module/source paths): the page's own "Scope note (2026-08-24, D-09/D-12 currency sweep)" already corrects the entire framing from a fictional `tracing`-ecosystem description to the real `log`+`env_logger`+`LogOrchestrator` facade — `src/application/services/log_orchestrator/` confirmed present (`find`), `LogDestination` enum confirmed present (`grep -rln 'LogDestination' src/`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 2 hits (`max_tokens`, `PaladinResult`, lines 131, 154) — checked, matches | — | — | — |
| 70 | docs/src/operations/monitoring.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-web/src/health.rs`'s `GET /health`/`GET /ready` handlers confirmed matching the page's "Corrected 2026-08-24" overview note exactly; direct check (Phase 28 tracing-surface cross-check, per this task's action text): the page's blanket premise — "`prometheus` and `opentelemetry` are not dependencies anywhere in the workspace" — was TRUE when written (2026-08-24) but is now **false for `opentelemetry`**: `grep -c '^opentelemetry' Cargo.toml` → **5** (`opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` etc., added by Phase 28's `otel` feature, SS-56) — `prometheus` remains correctly absent (`grep -c '^prometheus' Cargo.toml` → 0), so the practical "no `/metrics` scrape route" conclusion still holds, but the stated premise is now wrong — **mismatch, first finding**; the "Distributed Tracing → Jaeger Integration" section (lines 384-410) shows fabricated code using `opentelemetry_jaeger`/`tracing_opentelemetry` — neither crate is a dependency (`grep -n 'opentelemetry_jaeger\|tracing-opentelemetry' Cargo.toml crates/*/Cargo.toml` → 0 hits) — and the section never mentions the REAL, shipped Phase 28 tracing export mechanism (`OtelTraceSink`, the `otel` Cargo feature, OTLP/HTTP export, fully documented on `operations/observability.md`, row 71) — the page's own top-level "not implemented" disclaimer, written before Phase 28 shipped, is now stale on this specific axis even though it remains correct for Prometheus/Grafana/Alertmanager — **mismatch, second finding, ID in this row's MB ID(s) cell**; the "Health Checks" section's own "Corrected 2026-08-24" callout is still accurate (verified against `crates/paladin-web/src/health.rs`); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 1 hit (`PaladinResult`, line 98) — no `otel`/`TraceRecord`/`TraceConfig` token anywhere on the page, confirming the Distributed Tracing section never names the real Phase 28 mechanism it should point readers to | Phase 28 — `OtelTraceSink`/`otel` Cargo feature, real OTLP/HTTP trace export (OBS-02, SS-56) | MB-32 | M |
| 71 | docs/src/operations/observability.md | current | class 1 (version strings): none, page carries its own `**Since:** v0.10.0 (Phase 28, PRD 07)` marker instead; class 4 (module/source paths): `crates/paladin-core/src/platform/container/trace.rs` (`TraceEvent`) confirmed present, `src/infrastructure/telemetry/mod.rs::build_run_sink` confirmed present; direct check: the "twelve variants" table cross-checked variant-by-variant against the live `#[non_exhaustive] pub enum TraceEvent` (`grep -c '^\s*[A-Z][A-Za-z]* {$' trace.rs` limited to the enum body) — `RunStarted`, `SuperstepStarted`, `NodeStarted`, `NodeProgress`, `NodeFinished`, `EdgeEvaluated`, `DeltaMerged`, `WaypointSaved`, `ParleyRaised`, `RunFinished`, `FallbackHop`, `MiddlewareEvent` — **exactly 12, matches**; Phase 28/29 D-16 tracing-overhead cross-check (per this task's action text): the page's "Known limitations" section states the measured overhead figures (`log_sink` +22.18%, `composite` +18.46%) verbatim from `28-BENCH-EVIDENCE.md`, correctly names the ≤3% bar as FAILED (not softened), and correctly records the v0.10.0 accepted-deviation disposition citing `.project/v0.10.0/09-program-acceptance-audit.md` — **matches the Phase 28/29 record exactly, no discrepancy**; `otel`/`opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` deps confirmed present (`grep -c '^opentelemetry' Cargo.toml` → 5); WINDOWS.md id 33 correctly cited (waived, not open) for the reduced parley/done SSE payload; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `TraceRecord`, `TraceConfig`, `PALADIN_TRACE_OTEL_ENABLED`, `otel`, `run_traces` (SS-53…SS-56, SS-62) — all correct | — | — | — |
| 72 | docs/src/operations/performance-tuning.md | **stale** | class 1 (version strings): line 111-112 `--save-baseline v0.8.0`/`--baseline v0.8.0` are criterion baseline-label examples, not factual version claims; class 4 (module/source paths): the "Benchmark Results" section's own "Dated, unverified against the current tree (corrected 2026-08-24)" callout states "`benches/` today contains only `config_benchmarks.rs`... no Garrison, Battalion, or Herald benchmark file exists" — `ls benches/` → **`BENCHMARK_FIXES.md`, `config_benchmarks.rs`, `engine_benchmarks.rs`** — a second benchmark file, `engine_benchmarks.rs` (380 lines, superstep-engine benchmarks), was added 2026-09-02 (`git log --diff-filter=A -- benches/engine_benchmarks.rs`), after the page's own 2026-08-24 correction — the correction's file-count claim is now itself one file stale — **mismatch, ID in this row's MB ID(s) cell**; `benches/BENCHMARK_FIXES.md` confirmed present, matching the rest of the correction's claim about the four never-fixed drafted files; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits | Phase 22 — the superstep engine gains its own benchmark file (`benches/engine_benchmarks.rs`, ENG-02); no specific SS-nn row names the benchmark file itself, cited against the live `benches/` directory listing directly | MB-33 | S |
| 73 | docs/src/operations/troubleshooting.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-web/src/health.rs` confirmed present, matching the page's "Corrected 2026-08-24" callout; direct check (the same Phase 28 dependency-claim cross-check as row 70): the page repeats monitoring.md's stale premise verbatim — "`prometheus` and `opentelemetry` are not dependencies anywhere in the workspace" — `opentelemetry` IS now a dependency (`grep -c '^opentelemetry' Cargo.toml` → 5, Phase 28's `otel` feature); the practical conclusion (no `/metrics` scrape route, port `8081` fabricated, real ports `8080`/`9090` per `Dockerfile:68`) remains correct, only the dependency-list premise is stale — **mismatch, ID in this row's MB ID(s) cell**; the "Enable Debug Logging" section's "no `logging:` YAML key anywhere in `Settings`" claim re-verified: `grep -n logging src/config/settings.rs` → 0 hits, matches; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits | Phase 28 — `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` become real, optional workspace dependencies behind the `otel` feature (OBS-02, SS-56) | MB-34 | S |
| 74 | docs/src/user-guides/agent-orchestrator-bridge.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 14 "Every example targets the current **v0.5.0** workspace" — live `Cargo.toml [workspace.package] version = "0.10.0"` (`grep -n '^version = ' Cargo.toml`) — **mismatch**; class 4 (module/source paths): `crates/paladin-ports/src/output/orchestrator_port.rs`, `paladin_executor_port.rs`, `battalion_port.rs`, `src/application/services/orchestration/` all confirmed present (`test -f`/`test -d`); class 9 (shipped-surface, `34-signals.sh`): 4 hits — `PaladinResult`/`TokenUsage` at lines 87, 102, 103, 111 — checked, matches; the page's 7 `{{#include}}` blocks are mechanically verified by the build-baseline doc-examples gate (34-03 Task 1), so the substantive Rust content is current — only the version banner is stale | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` (`Cargo.toml [workspace.package]`); no matching §1 row exists for a bare version-pin fact, same citation basis as `getting-started/quickstart.md`'s §2 row (row 67 above) | MB-18 | S |
| 75 | docs/src/user-guides/agent-runtime.md | current | class 1 (version strings): none on this page; class 4 (module/source paths): `src/config/agent_runtime.rs`, `paladin_core::platform::container::aegis::RetryPolicy` confirmed present; direct check: `superstep.rs`'s dispatch comment naming `PaladinPort::execute_observed` confirmed present (exact cited line number `946` has drifted with unrelated later commits to ~1054-1137, a cosmetic line-drift only — the underlying claim, that a `PaladinExecutionService` carrying middleware applies its chain automatically as a node with no engine-side change, is unchanged and confirmed by direct read); WR-02 cross-check: `26-REVIEW.md` confirms `execute_json_schema`/`execute_structured_call` bypass the `ExecutionMiddleware` chain exactly as the page states; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `ExecutionMiddleware`, `AgentRuntimeConfig`, `TokenCounterPort`, `HistoryTrimmer`, `VaultPort`, `StructuredExecutorPort`, `reasoning_agent`, `tool_error_mode`, `schemars` (SS-35…SS-43) — all correct and current | — | — | — |
| 76 | docs/src/user-guides/arsenal-tools.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-ports/src/output/arsenal_port.rs`, `crates/paladin-core/src/platform/container/arsenal/handoff_tool.rs` confirmed present; direct check (live struct read, `crates/paladin-core/src/platform/container/arsenal/core.rs:79-93`): live `ArmamentResult` has 5 fields (`call_id: Uuid`, `success`, `output`, `error`, `execution_time_ms`, not `#[non_exhaustive]`) — the page's "Custom Armaments" struct literal (`Ok(ArmamentResult { success: true, output: Some(...), error: None })`) omits `call_id` and `execution_time_ms` and would not compile — **mismatch, first finding, ID in this row's MB ID(s) cell**; same section's `call.args["expression"]` names a field that does not exist on the live `ArmamentCall` struct (`crates/paladin-core/src/platform/container/arsenal/core.rs:36-44`), whose real field is `arguments: HashMap<String, Value>` — **mismatch, second finding**; "Handoff Tool" section's `.with_specialist(Arc::new(...))` chainable-per-call method does not exist on `PaladinBuilder` — the live method is `with_handoffs(mut self, specialists: Vec<Arc<Paladin>>)`, taking the whole list at once (`src/application/services/paladin/paladin_builder.rs:859`, introduced 2026-05-30, pre-v0.10.0) — **mismatch, third finding**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 1 hit (`PaladinResult.handoff_history`, line 281) — checked, matches | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; `with_handoffs`/`ArmamentResult`'s 5-field shape both predate this milestone, no Phase 22-33 REQ-ID applies) | MB-19 | M |
| 77 | docs/src/user-guides/battalion-patterns.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 51 `paladin-ai = { version = "0.5.0", ... }` — live `0.10.0` — **mismatch, first finding**; class 4 (module/source paths): direct read of `crates/paladin-battalion/src/commander.rs` — live `Commander::new` takes 5 positional args (`strategy, paladins, config, aggregator, paladin_port`, lines 280-285), NOT the page's `Commander::new(paladin_port, paladin_registry)` (lines 265, no such 2-arg form exists, and `paladin_registry` is never a `Commander::new` parameter anywhere in the type); live `Commander::execute` takes only `(&self, input: &str)` (line 471), NOT the page's `commander.execute(paladins, "...", BattalionStrategy::Auto, config)` 4-arg call (lines 268-270, 273-275) — **mismatch, second finding, ID in this row's MB ID(s) cell**; the accurate current API is the `CommanderBuilder` pattern `orchestration.md` (row 87) uses; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `PaladinResult`/`TokenUsage` (lines 312-315) — checked, matches | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact; the Commander constructor/execute mismatch is pre-v0.10.0 drift (D-00g), no REQ-ID applies | MB-20 | M |
| 78 | docs/src/user-guides/content-processing.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 9 "Every code example targets the current **v0.5.0** workspace", line 162 "v0.5.0. To keep this guide honest" — live `0.10.0` — **mismatch**; class 8 (feature flags): `web-scraping`/`rss` declared-but-no-adapter and `content_filtering_service` disabled-module claims re-checked live — `crates/paladin-content/Cargo.toml:19-20` (`web-scraping = ["dep:scraper"]`, `rss = ["dep:rss"]`) and `crates/paladin-content/src/services/mod.rs:6,9` (`content_filtering_service` still commented out) both confirmed accurate, unchanged; class 4 (module/source paths): all 6 `{{#include}}` targets (`pdf`, `http`, `news`, `aggregate`, `summarize`, `llm_bridge`, `delivery` — 6 blocks) mechanically verified by the build-baseline doc-examples gate; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this page predates Phase 22-33's scope entirely — `paladin-content` was not touched by this milestone) | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact | MB-21 | S |
| 79 | docs/src/user-guides/control-flow.md | **stale** | class 1 (version strings): none on this page; class 4 (module/source paths): live `NextStep` enum read at `crates/paladin-core/src/platform/container/directive.rs:40-88` — `Directive`/`NextStep::{Edges,Goto,Muster,End,Parley}` variant order and fields match the page's code sample (lines 40-51) exactly; **but** the page's own prose (lines 72-74) describes `NextStep::Parley` as "declared for Doc 03's suspension mechanism but not implemented this phase: a node returning it fails the run with `EngineError::ParleyNotSupported`... Phase 24 (HITL-01) lands the real behavior" — the live enum's own rustdoc at that exact variant (`directive.rs:71-88`) documents Parley as fully implemented (HITL-01, D-02, D-03: "Pause the run awaiting external input"), and `EngineError::ParleyNotSupported`'s own doc comment (`crates/paladin-battalion/src/engine/mod.rs:662-668`) states verbatim "**Superseded (Phase 24, HITL-01):** ... no longer reachable from any production code path in this engine" — **mismatch, first finding, ID in this row's MB ID(s) cell**: Phase 24 shipped over a year before this audit and this page (this milestone's own Phase 23 deliverable) was never updated to drop the "not implemented this phase" framing, even though `parley-and-chronicle.md` (row 91, this same milestone) correctly documents the real Parley/Gate mechanism; `EngineLimits::max_muster_tasks (default 100)` is named but its `APP_ENGINE_MAX_MUSTER_TASKS` env var (SS-16) is not — **mismatch, second finding**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `Directive`, `Muster`, `NodeSpec::Battalion`, `LlmDecisionEvaluator`, `EdgeCondition::Custom` (SS-11…SS-15) — checked, matches; the "Migrating from v0.9: M-B-01" section correctly cites `MIGRATION.md` §9.1 | Phase 24 — `NodeSpec::Gate` first-class approval-gate node + `WarEngine::resume_with` (HITL-01/HITL-02, SS-17/SS-18); Phase 23 — `APP_ENGINE_MAX_MUSTER_TASKS` environment override (CF-03, SS-16) | MB-22 | M |
| 80 | docs/src/user-guides/eval-harness.md | current | class 1 (version strings): none, page carries its own `**Since:** v0.10.0 (Phase 28, PRD 07)` marker instead; class 4 (module/source paths): `docs/schemas/eval-scenario.schema.json` confirmed present (`test -f`), `crates/paladin-eval/tests/schema_golden.rs` confirms the `UPDATE_EVAL_SCHEMA=1` bless command; direct check: the "twelve assertions" table (lines 68-81) cross-checked variant-by-variant against the live `Assertion` enum (`crates/paladin-eval/src/scenario.rs:596-658`) — all 12 serde-representable variants (`FinalStateFieldEquals` … `FinalStateSnapshot`) present with matching field shapes and wire names, `custom(fn)` correctly noted as Rust-API-only (no serde repr on the enum); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `paladin-eval`, `PALADIN_EVAL_LIVE`, `eval run` (SS-58…SS-60) — checked, matches | — | — | — |
| 81 | docs/src/user-guides/fault-tolerance.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): every `{{#include ../../../crates/doc-examples/src/fault_tolerance.rs:...}}` block (9 anchors: `attach`, `transience`, `retry`, `custom_predicate`, `timeout`, `heartbeat`, `handlers`, `compensation`, `custom_handler`, `fallback`, `cache` — 11 blocks) mechanically verified by the build-baseline doc-examples gate; direct check: line 172 states "`on_error` and `cache`... are hashed (fingerprint version `v5`)" — live `GRAPH_FINGERPRINT_VERSION` is `"v6"` (`crates/paladin-core/src/platform/container/waypoint.rs:389`), bumped by Phase 26 D-29's `;output_schemas:` section (`git log -S GRAPH_FINGERPRINT_VERSION`, commit `d17a505f`) — the page's own v5 claim was correct at Phase 25 but was never updated when Phase 26 bumped it again — **mismatch, ID in this row's MB ID(s) cell**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `Transience`, `Aegis`, `TimeoutPolicy`, `ErrorHandlerSpec`, `FallbackLlmAdapter`, `CachePolicy`, `redis-cache`, `APP_NODE_CACHE_ENABLED` (SS-27…SS-34) — checked, matches; R-23-01 accepted-risk citation and M-B-04 `MIGRATION.md` §9.1 cross-reference both confirmed accurate | Phase 26 — `output_schema` on `NodeSpec::Paladin` bumps `GRAPH_FINGERPRINT_VERSION` to `v6` (D-29, RT-05, SS-40) | MB-23 | S |
| 82 | docs/src/user-guides/garrison-memory.md | current | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-ports/src/output/garrison_port.rs` (`GarrisonPort`, `LongTermGarrisonPort` traits, line 656) and `crates/paladin-memory/src/garrison/` confirmed present; direct check against Phase 26's `GarrisonEntry.is_summary` field (the exact defect `architecture/domain-model.md`'s §2 row recorded above) — this page's own "Summaries (`is_summary`)" section (lines 266-278) correctly documents the field, the compounding-summary behavior and the `HistoryTrimmer`/`SummarizationMiddleware` interaction; `GarrisonEntry::new(role, content)` (line 53) and `GarrisonConfig::new(max_entries, max_tokens)` (line 161) both match the live constructors exactly (`crates/paladin-core/src/platform/container/garrison.rs:98,292`) — **Phase 32 deleted-type check (explicit): the page names neither the deleted `garrison::TokenCounter` trait nor the deleted `TokenCounterFactory` struct anywhere** — `grep -n 'TokenCounter\b|TokenCounterFactory|garrison::TokenCounter' garrison-memory.md` returns zero hits, so PRIM-03's Phase 32 deletion has nothing to contradict on this page; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `max_tokens` (lines 163, 170, 247, 285) — checked, matches the VOCAB-05 four-meanings disambiguation, this page's usage is the `GarrisonConfig` token-budget meaning | — | — | — |
| 83 | docs/src/user-guides/graph-visualization.md | current | class 1 (version strings): none, page carries its own `**Since:** v0.10.0 (Phase 28, PRD 07)` marker instead; class 4 (module/source paths): `crates/paladin-web/src/dev_ui_controller.rs` (`mermaid_url` field, `InspectorView`) confirmed present, `crates/paladin-battalion/tests/golden/export/` confirmed present; direct check against WINDOWS.md rows 33/34 (both `waived`, not `open`) — the page's "Overlay source: Waypoints vs. persisted trace" section correctly describes the exact Waypoints-derived-edges-only limitation those waived rows document, so its silence on the underlying implementation detail is consistent with the accepted disposition, not a gap; `dev-ui` feature-flag gating and admin-auth requirement (lines 101-105) match `crates/paladin-web/Cargo.toml`'s feature declaration; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `run export`, Waypoint/`run_traces` cross-references (`/v1/dev-ui/threads`, `otel`) — checked, matches | — | — | — |
| 84 | docs/src/user-guides/herald-output.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): `crates/paladin-core/src/platform/container/herald.rs` confirmed present, `crates/paladin-herald/src/{json_herald,markdown_herald,table_herald}.rs` confirmed present; direct check: the page's "Herald Trait" section (lines 100-114) shows only **3** methods (`format_paladin_result`, `format_battalion_result`, `format_stream_chunk`) — the live `Herald` trait (`crates/paladin-core/src/platform/container/herald.rs:49-153`) has **7**: the same 3 plus `finalize_stream(&self, metadata: &ExecutionMetadata)`, `format_error(&self, error: &PaladinError) -> String`, `name(&self) -> &str`, `mime_type(&self) -> &str` — all four pre-existing (`git log -S 'fn finalize_stream'` → commit `b83325b7`, 2026-05-13, pre-v0.10.0) — **mismatch, ID in this row's MB ID(s) cell**; `output-formatting.md` (row 88, this same phase) documents the correct 7-method trait and even carries a self-correcting annotation naming the discrepancy directly ("has seven methods, not three"); the "Custom Herald Implementation" `CsvHerald` sample also only implements the 3 stale methods, so it would not satisfy the live trait; `JsonHerald::new()`/`with_config()`, `MarkdownHerald::new()`/`with_config()`, `TableHerald::new(config)` constructors and the six-key `TokenUsage` JSON sample (lines 55-58, matching SS-73) all confirmed correct; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `PaladinResult`, `TokenUsage`, `BattalionResult` — checked, matches | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; the 7-method `Herald` trait predates this milestone, no Phase 22-33 REQ-ID applies) | MB-24 | S |
| 85 | docs/src/user-guides/maneuver-flow-dsl.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 55 `paladin-battalion = { version = "0.8.0", ...}`, line 1154 "**Version**: 0.8.0" — live `0.10.0` — **mismatch**; class 4 (module/source paths): `src/application/cli/commands/maneuver.rs` confirmed present, live `ManeuverCommands` enum (`Visualize(ManeuverVisualizeArgs)`, `Validate(ManeuverValidateArgs)`) matches the page's documented `paladin maneuver visualize`/`paladin maneuver validate` subcommands exactly, including the `--format ascii`/`--format mermaid`/`-o` flags; `use crate::core::platform::container::battalion::maneuver::Maneuver` (the pre-hexagonal-reorg `src/core/` path, per `src/application/cli/commands/battalion.rs:776`) confirmed present — this is the facade binary crate's own internal path, unaffected by the `crates/` extraction; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (Maneuver predates Phase 22-33 entirely and was not touched by this milestone) | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact | MB-25 | S |
| 86 | docs/src/user-guides/memory-management.md | current | class 1 (version strings): none; class 4 (module/source paths): every constructor call cross-checked against live signatures — `GarrisonConfig::new(max_entries, max_tokens)` (no `with_max_entries()`/`with_max_tokens()` builder chain), `SqliteGarrison::connect(path, config, paladin_id)` (not `.new()` + `.with_config()`), `EvictionStrategy::{FIFO, ImportanceBased, SlidingWindow}` (not `EvictionPolicy`, no `Lru`/`Custom(..)`) — all confirmed exactly matching `crates/paladin-core/src/platform/container/garrison.rs` and `crates/paladin-memory/src/garrison/sqlite_garrison.rs:73`; this page already carries its own correction annotations for each of these (e.g. "there is no default-then-with_max_* builder chain") — the page has clearly been through a prior accuracy pass; the Phase 31 D-29 `token_count` disposition (lines 159-160, 240-253 area) was independently re-verified in 34-03's Task 1 vocabulary sweep as clean (a distinct, unmigrated `GarrisonEntry.token_count` field, not an ACCT-01…05 carrier); **Phase 32 deleted-type check (explicit): the page names neither the deleted `garrison::TokenCounter` trait nor the deleted `TokenCounterFactory` struct** — its only `TokenCounter*` reference is line 159's `TokenCounterPort` (the live, current port) plus the live `TiktokenCounter`/`HeuristicTokenCounter` implementors (lines 159-160), never the legacy trait PRIM-03 removed; the "RAG (Retrieval-Augmented Generation)" section (lines 611-650) is explicitly illustrative DIY code (`RAGPaladin`, a user-composed wrapper over `LongTermGarrisonPort::semantic_search`), not a claim about the shipped `RagRetrievalService`/`Sanctum` API, so it does not misdescribe Phase 33's surface; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `TokenCounterPort`, `max_tokens` (lines 159-160, 763-768, 1048-1059) — checked, matches | — | — | — |
| 87 | docs/src/user-guides/orchestration.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 14 "Every code example targets the current **v0.8.0** workspace" — live `0.10.0` — **mismatch**; class 4 (module/source paths): `crates/paladin-battalion/src/{formation_service,phalanx_service,campaign_service,chain_of_command_service,commander}.rs` all confirmed present; direct check: `CommanderBuilder::new(paladin_port).strategy(...).paladins(...).build()?` (lines 161-166, 198-206) matches the live `CommanderBuilder` builder pattern exactly (`crates/paladin-battalion/src/commander.rs:1508-1530`) — this is the ACCURATE Commander API `battalion-patterns.md`'s §2 row above (row 77) got wrong; `BattalionResult` field list (lines 301-304: `final_output`, `paladin_results: Vec<PaladinResult>`, `status`, `strategy_used`, `total_tokens: u64`, `per_paladin_times`, `per_paladin_tokens`) confirmed accurate; 6 of 6 `{{#include}}` blocks (`formation`, `phalanx`, `campaign`, `chain_of_command`, `commander`, `scheduling`, `events` — 7 blocks) mechanically verified by the build-baseline doc-examples gate; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `PaladinResult`, `TokenUsage`, `per_paladin_tokens` (lines 209-210, 302-304) — checked, matches | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact | MB-26 | S |
| 88 | docs/src/user-guides/output-formatting.md | current | class 1 (version strings): none; class 4 (module/source paths): direct check of the "Herald Architecture" section (lines 33-70) against the live 7-method `Herald` trait (`crates/paladin-core/src/platform/container/herald.rs:49-153`) — the page's own text states "has seven methods, not three" and its code sample lists all 7 (`format_paladin_result`, `format_battalion_result`, `format_stream_chunk`, `finalize_stream`, `format_error`, `name`, `mime_type`) exactly matching the live trait signature-for-signature — this is the ACCURATE trait listing `herald-output.md`'s §2 row above (row 84) is missing 4/7 methods on; `OutputFormat::{Text, Json, Structured}` (lines 44-48) confirmed matching `crates/paladin-core/src/platform/container/paladin_config.rs:12-19` exactly; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 11 hits, all `PaladinResult`/`HeraldError` in `format_paladin_result` signatures across the page's many formatter examples — checked, matches | — | — | — |
| 89 | docs/src/user-guides/paladin-agents.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → line 36 `paladin-ai = { version = "0.5.0", ...}` — live `0.10.0` — **mismatch, first finding**; class 4 (module/source paths): direct check — the "Memory — Garrison" section's `InMemoryGarrison::new()` (line 273, zero args) does not match the live constructor `pub fn new(config: GarrisonConfig) -> Self` (`crates/paladin-memory/src/garrison/in_memory_garrison.rs:79`, one required arg) — **mismatch, second finding, ID in this row's MB ID(s) cell**; the "Agent Handoffs" section's `.with_specialist(Arc::new(...))` chainable-per-call form (lines 254-255) does not exist — live is `with_handoffs(mut self, specialists: Vec<Arc<Paladin>>)` (`src/application/services/paladin/paladin_builder.rs:859`, pre-v0.10.0) — the same defect `arsenal-tools.md`'s §2 row above (row 76) carries — **mismatch, third finding**; `PaladinError` variant list (lines 364-372: `ConfigurationError`/`ExecutionError`/`LlmError`/`Timeout`/`StopWordDetected`) confirmed exactly matching the live enum (`crates/paladin-core/src/platform/container/paladin_error.rs:29-49`); `JsonHerald` import path `paladin::infrastructure::adapters::herald::JsonHerald` (line 317) confirmed still valid — `src/infrastructure/adapters/herald/mod.rs:16` re-exports `paladin_herald::{JsonHerald, MarkdownHerald}`, so the facade path is a live re-export, not stale; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `PaladinResult`, `TokenUsage` (lines 161, 213, 260, 300) — checked, matches | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`; the `InMemoryGarrison::new()`/`with_specialist` mismatches are pre-v0.10.0 drift (D-00g), no REQ-ID applies | MB-27 | M |
| 90 | docs/src/user-guides/paladin-configuration.md | current | class 1 (version strings): none; class 4 (module/source paths): every line-number citation the page makes independently re-verified byte-exact against the live tree — `paladin_builder.rs:686` (`with_arsenal_registry`), `execution_result.rs:38` (`StopReason::MaxLoops` variant), `paladin_builder.rs:1267` (`pub async fn build`), `paladin_builder.rs:1116` (private `fn validate`), `src/config/agents.rs:209` (`AgentDefinition` struct start) — all 5 line citations confirmed exact via direct `sed -n`/`grep -n` reads; direct check: "there is no `paladin:` top-level section in `config.yml`... the real top-level type is `Settings`, with no `paladin` field" and "`Settings.agents: Vec<AgentDefinition>`... no `retry_attempts`" both confirmed against `src/config/settings.rs:52-54` and `src/config/agents.rs`; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this page predates Phase 22-33's capability set — pure `PaladinBuilder` fluent-API tuning, untouched by this milestone) | — | — | — |
| 91 | docs/src/user-guides/parley-and-chronicle.md | current | class 1 (version strings): none; class 4 (module/source paths): `ThreadId::child_on_branch` confirmed present at `crates/paladin-core/src/platform/container/waypoint.rs:225` and used at `crates/paladin-battalion/src/engine/superstep.rs:1213`; `APP_WAYPOINT_STORE_BACKEND` confirmed present at `crates/paladin-web/src/thread_controller.rs:183,187`; direct check: commit `00b1e552` ("fix(24): CR-01 gate POST /v1/threads/{id}/resume behind require_admin") confirmed in `git log`, matching the page's own "CR-01, commit `00b1e552`" citation (lines 315-316) exactly; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 31 hits — `WarEngine`, `Waypoint`, `GET /v1/threads/{id}/state`, `POST /v1/threads/{id}/resume`, `GET /v1/threads/{id}/history`, `ChronicleService`, `ShutdownCoordinator`, `APP_ENGINE_SHUTDOWN_GRACE_SECS` — all SS-17…SS-26 items named and correctly described; page correctly cross-references `platform-api.md` for the full run lifecycle rather than duplicating it | — | — | — |
| 92 | docs/src/user-guides/sanctum-vector-memory.md | **stale** | class 1 (version strings): none; class 3 (crate names): `grep -noE 'paladin-[a-z-]+'` → `paladin_memory`/`paladin_ports`/`paladin_core` all confirmed present; class 4 (module/source paths): line 49 names `RAGRetrievalService` (all-caps RAG) — the live struct is `RagRetrievalService` (`crates/paladin-memory/src/services/rag_retrieval_service.rs:179`, camelCase Rag, confirmed the only such struct in the file) — **mismatch, naming, first finding**; the entire "RAG — Retrieval-Augmented Generation" section (lines 215-249) describes RAG as automatic `PaladinBuilder::with_sanctum()`/`with_embedding_port()` wiring with `config.yml`'s `rag: { top_k, min_score, inject_into_prompt }` block, and never mentions `RagRetrievalResult`, `ShedItem`, `RagRetrievalError`, `retrieve_context_with_timeout`, or `with_token_counter` — Phase 33's entire shipped RAG surface (SS-86…SS-91) is absent; direct check: `PaladinExecutionService::format_retrieved_context(&self, results: &RagRetrievalResult)` (`src/application/services/paladin/paladin_execution_service.rs:1979`) appends a `rag_omission_marker` when memories are shed for budget reasons (line 1995) — this truncation-marker behavior, the exact "RAG truncation marker" the plan's action text names, is completely undocumented on this page — **mismatch, missing Phase 33 surface, second finding, ID in this row's MB ID(s) cell**; `with_sanctum`/`with_embedding_port` themselves still exist on `PaladinBuilder` (`src/application/services/paladin/paladin_builder.rs:751,783`), so the page's basic attachment pattern is not wrong, only silent about everything Phase 33 added on top of it; **Phase 32 deleted-type check (explicit): the page names neither the deleted `garrison::TokenCounter` trait nor the deleted `TokenCounterFactory` struct** — `grep -n 'TokenCounter\b|TokenCounterFactory|garrison::TokenCounter' sanctum-vector-memory.md` returns zero hits, this page's staleness is entirely a Phase 33 RAG-surface omission, not a Phase 32 token-counter issue; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (page names no `RagRetrievalResult`/`ShedItem`/`RagRetrievalError`/`resolve_context_window` token at all) | Phase 33 — `RagRetrievalService::retrieve_context` returns `RagRetrievalResult` (COMM-01, SS-86); `RagRetrievalResult.shed: Vec<ShedItem>` truncation record (COMM-02, SS-87); `RagRetrievalError` typed enum (COMM-01, SS-88); `RagRetrievalService::with_token_counter` builder (COMM-04, SS-91) | MB-28 | L |
| 93 | docs/src/user-guides/tool-integration.md | **stale** | class 1 (version strings): none; class 4 (module/source paths): the page's `ArmamentCall`/`ArmamentResult` struct-literal samples (lines 457, 565, 638, 658) all correctly use the live field names (`call.tool_name`, `call.arguments`, and the complete 5-field `ArmamentResult { call_id, success, output, error, execution_time_ms }`) — notably MORE accurate than `arsenal-tools.md`'s equivalent samples (row 76 above); direct check: the "Reachability note" (lines 33-40) states "a Paladin's own reasoning loop never triggers an Armament on its own today" and that Arsenal invocation requires "a consumer-supplied `LlmPort` implementation that parses tool calls itself" — this is now incomplete: `agent-runtime.md`'s (row 75, this same phase) "Tool-Call Protocol" section documents `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware`, an opt-in, shipped, prompt-level mechanism that DOES make the Paladin's own reasoning loop trigger an Armament (installed by the `reasoning_agent` preset) without a consumer needing to write a custom `LlmPort` — this page's note only speaks to the still-true narrower fact (no adapter populates `LlmResponse.function_call`) but omits the broader capability Phase 26 shipped on top of it — **mismatch, ID in this row's MB ID(s) cell**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 1 hit (`reasoning_agent`, line 956) — checked, but the page's own reachability framing above contradicts what that cross-reference implies | Phase 26 — `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` prompt-level tool-call protocol via the `reasoning_agent` preset (RT-07, `tool_error_mode`, SS-41/SS-42) | MB-29 | S |
| 94 | docs/src/user-guides/the-superstep-engine.md *(proposed — does not exist on disk)* | **missing** | Decided by content, not by the class-9 grep alone (D-06): see the "Superstep-engine dedicated-page decision" subsection immediately below the §2 table for the full per-token evidence. Summary — `WarEngine`/`Battlefield`/`Waypoint`/`Vanguard`/`max_supersteps` appear on 6 `user-guides/` pages (`control-flow.md` 15 `superstep` hits, `fault-tolerance.md` 8, `parley-and-chronicle.md`/`graph-visualization.md`/`eval-harness.md` 4 each, `agent-runtime.md` 1) but every hit is a passing mention inside a page about something built ON the engine, never a page describing the engine's own supersteps/vanguard/fingerprinting/checkpointing mechanics as its primary subject; `control-flow.md`'s own line 29-30 states verbatim "This page assumes that much and no more; the full engine guide is future documentation (see the `Deferred` note in `23-CONTEXT.md`)" — a live, in-tree admission that the dedicated page was deferred and never written; `23-CONTEXT.md` line 529-531 confirms: "No mdBook page for the WarEngine exists (`docs/src/SUMMARY.md` has no engine/Battlefield/Waypoint entry) despite X-08 — a Phase 22 residual... the full engine page belongs to a docs pass or SHIP-01 (Phase 29)" — Phase 29's docs plan (29-06) never picked it up either (`29-06-PLAN.md`'s declared scope, per `34-CONTEXT.md`'s own canonical-refs note, explicitly left the default-feature doc-warning set out of scope and named no engine-page deliverable) | Phase 22 — `WarEngine` executes cyclic graphs in supersteps, self-loops included (ENG-02, SS-01); `Battlefield` typed state and superstep merge (ENG-01); `Waypoint` full-snapshot checkpointing (ENG-03, SS-02); `WaypointPort` three backends (ENG-05, SS-03); `EngineConfig`/`max_supersteps`/`max_node_visits` (ENG-02, SS-05); `APP_ENGINE_MAX_SUPERSTEPS` (ENG-02, SS-06) | MB-30 | L |

### Superstep-engine dedicated-page decision (D-06, plan 34-04 Task 1)

**Decision: `missing`.** No page in `docs/src/SUMMARY.md`, and no section on any existing page, is
dedicated to the Phase 22 superstep engine (`WarEngine`, `Battlefield`, `Waypoint`, `Vanguard`,
`max_supersteps`). Row 94 above records it; this subsection is the evidence that decided it, per
the token-by-token method the plan's action text requires — decided by content, never by the
class-9 grep summary alone.

**Per-token grep results, `docs/src`-wide:**

| Grep token | `grep -rc` total | Pages hit | Every hit inspected — a page primarily about the engine itself? |
|---|---|---|---|
| `WarEngine` | 19 hits across `user-guides/` | `control-flow.md` (2), `parley-and-chronicle.md` (9), `agent-runtime.md` (6), `fault-tolerance.md` (1), `eval-harness.md` (1) | No — every occurrence names the engine as the substrate a *different* capability (routing, pause/resume, middleware, fault-tolerance, eval scenarios) is built over, never the engine's own supersteps/vanguard/fingerprint mechanics |
| `Battlefield` | 23 hits across `user-guides/` | `control-flow.md` (7), `parley-and-chronicle.md` (6), `eval-harness.md` (5), `fault-tolerance.md` (4), `agent-runtime.md` (1) | No — same pattern: `Battlefield` is referenced as the state each guide's own subject reads/writes, never described as a type in its own right (schema, dispatch rules, merge semantics) |
| `Waypoint` (bare) | 45 hits across `user-guides/` | `agent-runtime.md` (3), `control-flow.md` (4), `fault-tolerance.md` (10), `graph-visualization.md` (5), `parley-and-chronicle.md` (23) | No — `parley-and-chronicle.md` documents Chronicle's *read* interface over Waypoint history in the most depth of any page, but even there Waypoint is the substrate for pause/resume/replay/fork, not the engine's own checkpointing algorithm (superstep merge, vanguard computation, fingerprint hashing) |
| `superstep` (case-insensitive) | 36 hits total | `control-flow.md` (15), `fault-tolerance.md` (8), `parley-and-chronicle.md` (4), `graph-visualization.md` (4), `eval-harness.md` (4), `agent-runtime.md` (1) | No — highest-density page is `control-flow.md`, and its own line 29-30 explicitly disclaims deeper engine coverage (quoted below) |
| `max_supersteps` | 2 hits total, one per page | `control-flow.md` line 29 (the same disclaiming sentence), `fault-tolerance.md` line 399 (naming it only as "the run-level bound," in a "Limitations You Should Know About" caveat about a *different*, unrelated accepted risk, R-23-01) | No — named twice, both times as a bound that exists, never explained (default value, `EngineError::RecursionLimitExceeded`, how it interacts with `max_node_visits`) |
| `Vanguard` | 1 hit | `fault-tolerance.md` line 228, inside the `ErrorHandlerSpec::Route` table row ("places `to` in the next Vanguard **instead of** the failed node's static successors") | No — the one occurrence uses Vanguard as an already-understood term inside an unrelated compensation-routing table cell, never defines or explains the concept itself; the term otherwise never appears in any user-guide page |

**The confirming in-tree admission.** `control-flow.md` line 29-30, quoted verbatim: *"every run is
bounded by `EngineLimits` (`max_supersteps`, `max_node_visits`). This page assumes that much and no
more; the full engine guide is future documentation (see the `Deferred` note in `23-CONTEXT.md`)."*
`23-CONTEXT.md` lines 529-531, quoted verbatim: *"No mdBook page for the WarEngine exists
(`docs/src/SUMMARY.md` has no engine/Battlefield/Waypoint entry) despite X-08 — a Phase 22
residual. D-28 adds only a short preamble on the control-flow page; the full engine page belongs to
a docs pass or SHIP-01 (Phase 29)."* Phase 29's own docs plan (`29-06-PLAN.md`) never picked this up
— `34-CONTEXT.md`'s own canonical-refs note states 29-06 "explicitly left the default-feature
`cargo doc` warning set out of scope," and no engine-page deliverable is named anywhere in that
plan's scope. The deferral chain (Phase 22 → Phase 23 D-28 preamble-only → "Phase 29" pointer →
Phase 29 scope excludes it) terminates unresolved at this audit.

**Proposed nav position.** In `docs/src/SUMMARY.md`, insert a new entry immediately before line 25
(`- [Control Flow: Dynamic Routing & Subgraphs](user-guides/control-flow.md)`), i.e. directly
after `- [Maneuver Flow DSL](user-guides/maneuver-flow-dsl.md)` (line 24) and before Control Flow —
matching `control-flow.md`'s own forward-reference and the fact that Parley/Aegis/Agent-Runtime all
build on engine concepts this page would establish first. Proposed title: **"The WarEngine:
Battlefield State & Superstep Execution"**, proposed path `user-guides/the-superstep-engine.md`
(placeholder — Phase 35 may choose a different filename so long as the nav position and content
scope match).

**Scope the missing page would need to cover**, drawn from the §1 Phase 22/22.1 rows this row's
Cites cell names: `WarGraph`/`Battlefield`/superstep merge semantics (ENG-01, SS-01), `Waypoint`
full-snapshot checkpointing and the `(ThreadId, WaypointId)` addressing scheme (ENG-03, SS-02),
the three `WaypointPort` backends (ENG-05, SS-03), `EngineConfig`/`EngineLimits`
(`max_supersteps`, `max_node_visits`, `run_timeout_secs`, `waypoint_durability`,
`max_muster_tasks`) and their `APP_ENGINE_*` env overrides (ENG-02, SS-05/SS-06),
`WaypointRetentionService` (ENG-05, SS-07), and the graph-fingerprint versioning scheme
(`GRAPH_FINGERPRINT_VERSION`, currently `v6` per row 81's finding above) that every dependent page
(`control-flow.md`, `fault-tolerance.md`, `parley-and-chronicle.md`) currently references but never
explains from first principles.

### Coverage command comparison (CONTEXT.md Folded Todos — the documentation slice)

Per the plan's action text, this compares `contributing/testing-guide.md`'s stated coverage
command and figure against the `Makefile` coverage target's actual body and the `ci.yml`
`coverage` job's real invocation, all three side by side. The end-to-end walk on a Docker machine
stays out of scope (no Docker in this devcontainer) and is recorded as a deferred-register pointer
below, not an `MB-nn`, per the plan's explicit instruction.

| Source | Stated/actual command | Feature list | Fail-under-lines value |
|---|---|---|---|
| `testing-guide.md` (lines 446-449, the page's own "full underlying invocation") | `cargo llvm-cov --workspace --features integration-tests --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` | `integration-tests` **only** | `82` (matches ADR-0006) |
| `Makefile` `coverage` target (`Makefile:307-313`) | `@# Delegates to scripts/coverage.sh — shared with CI's coverage job so the feature list cannot drift.` (no cargo command inlined in the Makefile itself) | delegated | delegated |
| `scripts/coverage.sh:100-101` (what both `make coverage` and `ci.yml`'s `coverage` job actually execute — `ci.yml`'s own comment at that job confirms: "Delegates to scripts/coverage.sh — shared with `make coverage`") | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1` | `integration-tests,llm-all` | `$FLOOR` defaults to `82` (`FLOOR="${COVERAGE_FLOOR:-82}"`, matches ADR-0006) |

**The disagreement, precisely.** All three sources agree the floor is `82`, matching ADR-0006 and
this page's own "Coverage Requirements" section (line 68) exactly — **the floor itself is not
stale**. The disagreement is the **feature list**: `testing-guide.md`'s shown command measures
`--features integration-tests` alone; the real, shared `scripts/coverage.sh` (added 2026-08-19,
commit `6aaf0743`, "fix(ci): measure all nine adapters in coverage") measures
`--features integration-tests,llm-all`. `scripts/coverage.sh`'s own comment states why this
matters, not as a cosmetic detail: the default feature set (`llm-openai`, `llm-anthropic`,
`llm-deepseek`) is only 3 of the 9 shipped LLM adapters — the six Phase 17 adapters (kimi, qwen,
grok, ollama, gemini, openai-compatible) sit behind non-default flags, so a measurement missing
`llm-all` "contributed ZERO lines to the measured figure" for those adapters, and "the gate passed
at 84.32% over 49209 lines while ignoring 5117 lines of shipped adapter code... With llm-all:
85.01% over 54326 lines, all nine adapters counted." `testing-guide.md`'s own promise — "every
command here is the same command CI runs, not an approximation of it" (line 401) — is what this
finding falsifies: the shown command is a strictly smaller measurement than what CI (via
`scripts/coverage.sh`) actually runs, and the page never mentions `scripts/coverage.sh` at all as
the real single source of truth. Recorded as the second half of `testing-guide.md`'s own §2 row above (row 54).

**Docker-machine walk — explicitly NOT closed by this phase.** The end-to-end reproduction of
`make services-up` → `make coverage` on a real Docker-capable machine remains the maintainer's own
item; this devcontainer has no Docker, so this audit cannot run it. Per the plan's explicit
instruction, this remainder is routed to `deferred-items.md` as a pointer for plan 34-09's
deferred-register assembly, **not** absorbed into `testing-guide.md`'s row above or invented as a separate `MB-nn`. This
row's verdict covers the commands and figures only, as compared mechanically above.

### D-09 subsection: `upgrading.md` / `migration-guide.md` vs `MIGRATION.md` §9.1 and §9.8

Per D-09, `api-reference/upgrading.md` and `api-reference/migration-guide.md` are checked
row-for-row against `MIGRATION.md` §9.1 (behavioral changes) and §9.8 (operator checklist).
Entry counts, measured live: `grep -c '^| M-B-' MIGRATION.md` → **4** §9.1 rows (M-B-01…M-B-04);
§9.8's checklist is a **7**-item ordered list (measured by reading the section, since it is
prose-numbered, not table-rowed: `sed -n '/^## 9\.8/,$p' MIGRATION.md | grep -cE '^[0-9]+\.'`
confirms 7).

**`upgrading.md` — §9.1 (4/4 entries), one line each:**

| §9.1 ID | `upgrading.md` disposition |
|---|---|
| M-B-01 (`EdgeCondition::Custom` fail-closed) | **carried** — page's table row 1 condenses the same change and required action correctly |
| M-B-02 (graceful shutdown grace window) | **carried** — page's table row 2 correctly states the default 30s grace and the `terminationGracePeriodSeconds >= 60` required action |
| M-B-03 (`tool_error_mode` no-op + redaction) | **carried** — page's table row 3 correctly states no behavioral change to the default and the sanitization detail |
| M-B-04 (automatic per-superstep Waypoint checkpointing) | **carried** — page's table row 4 correctly states the Waypoint/Battlefield snapshot behavior and that legacy Formation/Phalanx/Campaign/Commander paths are unaffected |

**`upgrading.md` — §9.8 (7/7 steps), one line each:**

| §9.8 step | `upgrading.md` disposition |
|---|---|
| 1. Back up state | **carried** — page step 1 names the same four state stores (waypoint store, run store, Garrison SQLite, Citadel files) |
| 2. Apply migrations via binary start, not a separate command | **carried** — page step 2 states the same `sqlx::migrate!`-at-construction mechanism and the Postgres note |
| 3. Update config — nothing required | **carried** — page step 3 cites the same `v0_9_config_boot` integration test |
| 4. Raise `terminationGracePeriodSeconds` to >= 60 | **carried** — page step 4 names the same three shipped manifests |
| 5. Register a custom evaluator per `EdgeCondition::Custom` name | **carried** — page step 5 names the same two registration APIs (`CampaignExecutionService::with_evaluator`, `WarEngine::with_edge_evaluator`) |
| 6. Deploy | **carried** — page step 6 is a verbatim paraphrase |
| 7. Verify (`setup-check`, `maneuver validate`, `eval run`, `graph export`) | **carried** — page step 7 names the same four verification commands and correctly states `GraphCommands` exposes only `export`, no runtime-probe subcommand |

**Result: 4/4 and 7/7 — zero disagreements, zero omissions.** No `MB-nn` is minted from this
comparison; `upgrading.md`'s own §2 row above is settled `current`.

**`migration-guide.md` — §9.1/§9.8 (0/4, 0/7 duplicated):** the page does not reproduce either
table — it deliberately points to `upgrading.md` and the root `MIGRATION.md` instead (lines 7-14:
"This historical guide stops at v0.5.0. The v0.10.0 upgrade record lives on the Upgrading page and
in the root `MIGRATION.md` file..."). Because there is no duplicated §9.1/§9.8 content on this
page, there is nothing to disagree with those sections — the row-for-row check therefore finds
**zero disagreements by design**, not by omission. The one currency defect this plan does record
for `migration-guide.md` (its own §2 row above, ID in that row's MB ID(s) cell) is a *different* issue: the page's opening
line ("...up to the current **v0.5.0** release") and its Timeline table ("0.1.0 | **Current**")
both contradict the fact that the page's own next section documents the v0.10.0 upgrade — a
self-consistency defect, not a §9.1/§9.8 disagreement.

### mdBook partition closure (D-06, plan 34-05 Task 2)

Every one of the ninety-three `docs/src/*.md` pages now carries a settled, command-backed
verdict, plus the one `missing`-verdict row (94) for the never-written superstep-engine page —
the ninety-four-row §2 table is closed. Counted directly from the table above, not recalled or
estimated (D-00b):

```
$ python3 -c "
import re
with open('.planning/phases/34-documentation-currency-audit/34-AUDIT.md') as f:
    lines = f.readlines()
current = stale = missing = 0
for line in lines:
    if not re.match(r'^\| \d+ \| docs/src/', line):
        continue
    v = line.split('|')[3].strip().replace('*', '').strip()
    if v == 'current': current += 1
    elif v == 'stale': stale += 1
    elif v == 'missing': missing += 1
print(current, stale, missing, current + stale + missing)
"
38 55 1 94
```

**Verdict distribution:** `current` **38**, `stale` **55**, `missing` **1** — total **94** rows,
matching `find docs/src -name '*.md' | wc -l` (**93**) plus the one `missing`-verdict page that
does not exist on disk.

**`MB-nn` total:** `grep -oE 'MB-[0-9]+' 34-AUDIT.md | sort -u | wc -l` → **60** — a
contiguous, unbroken numbering run from the first ID this file ever minted through the last
one this plan mints below, no gaps, no duplicates (`grep -oE 'MB-[0-9]+' 34-AUDIT.md | sort |
uniq -d` is empty — see 34-check.sh assertion (b) above). Per-plan contribution, by count only
(never by re-quoting a specific sibling row's own ID token, per this section's own D-03
discipline): plan 34-01 minted the first ID (1); plan 34-03 minted the next 16; plan 34-04
minted the next 19 (including the row-94 missing-page ID, cited only in that row's own MB
ID(s) cell above); plan 34-05 (this plan) minted the final 24 across its two tasks (12 per
task) — 1 + 16 + 19 + 24 = 60.

**Measured HEAD SHA:** the partition was measured against the Phase 34 start SHA recorded in
this file's Measurement Header, `ee1fb160f8e743e638b32beb6c4e32be4ede9325` (D-23). This plan's
own commits landed at further SHAs (this task's own HEAD at close: `be5d1a72c077873708b4fbafdfec385cd6e330e7`)
— per the D-23 invariance argument, every Phase 34 commit touches only `.planning/` (proven per
commit by `34-check.sh` assertion (d2) against the fixed start SHA above), so the source tree
every row in this file measures is identical at the Phase 34 start SHA, at this plan's own HEAD,
and at every SHA in between. A later plan's `git rev-parse HEAD` differing from either SHA above
does not invalidate any row recorded here.

## §3 Rustdoc findings table

Rows land in plans 34-06 (default-feature `cargo doc --workspace --no-deps` enumeration against
the `ci.yml:63` bar) and 34-07 (the per-crate `-D warnings --all-features` sweep, D-14). One row
is fully worked here — the known-answer `HeuristicTokenCounter` case CONTEXT.md names as a method
self-test — to prove the P-01 grep-recovery method plans 34-06/34-07 depend on before either
enumerates a single additional row.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-01 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps` (D-12/D-14) | paladin-memory | crates/paladin-memory/src/token_counter/mod.rs:3 | unresolved link (broken_intra_doc_links) | `error: unresolved link to \`HeuristicTokenCounter\`` | grep recovery (no `-->` span — the link lives in a `//!` module-level doc comment, per Pitfall P-01; recovered via `grep -n "HeuristicTokenCounter" crates/paladin-memory/src/token_counter/mod.rs` → `3://! [\`HeuristicTokenCounter\`] is the phase-wide default...`, matching `.planning/WINDOWS.md` row 37 exactly) | 34-EVIDENCE.md #6, #7 | S |

This same warning also appears in the default-feature `cargo doc --workspace --no-deps` run
(34-evidence/34-01-cargo-doc-default.txt line 9, "warning:" not "error:" — the severity differs by
run, the location and message do not); plan 34-06 enumerates it there under its own row without
re-deriving the location, citing back to the row worked above — see the matching row in the
default-feature enumeration's own table below (last row, same file:line).

### Default-feature enumeration (plan 34-06, D-13)

**The bar, quoted byte-identical from `.github/workflows/ci.yml:62-63`** (the `lint` job's "Check
documentation" step) and ratified by ADR-0033:

```
cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt
```

`.planning/decisions/0033-cargo-doc-warning-bar.md` ("ADR-0033: One `cargo doc` bar") ratifies this
exact command as the project's single zero-warning bar (Decision (i)) and records the measured
residue as debt with a named owner (Decision (ii): Phase 16/DOCS-03 at the time, now inherited by
this ground-truth phase's own downstream, Phase 36) — this plan does not reopen or re-litigate the
bar, only re-measures against it.

**This run's measurement (plan 34-06, re-run live, not copied from RESEARCH.md or 34-01's own
capture per this plan's operating rule):**
- HEAD measured: `d81de538d5697c215eb5cad346077f75fddabe1b` (this plan's own HEAD at capture time;
  the Phase 34 start SHA `ee1fb160f8e743e638b32beb6c4e32be4ede9325` recorded in the Measurement
  Header above remains the D-23 invariance reference — every intervening commit touches only
  `.planning/`, so the source tree the two SHAs measure is identical).
- `cargo --version` / `rustc --version`: `1.97.1`, matching `rust-toolchain.toml` exactly (Measurement
  Header above; re-confirmed at this plan's own precondition check before any measurement was taken).
- Full CI expression: `cargo doc` exit `0`; the trailing `! grep -q "warning:" ...` negation exits
  `1` (i.e. the composite `&&` expression is non-zero — the gate is **RED**, warnings are present).
  Wall time: 6s (warm `target/`; `Documenting` lines for already-built crates were skipped by
  cargo's own incremental cache, confirmed by diffing this run's raw capture against 34-01's own
  independent capture of the same command — the two differ only in `Documenting` line presence/order,
  never in warning content, location, or count).
- Raw capture: `34-evidence/34-06-cargo-doc-default.txt` (578 lines, verbatim `tee` output).

**Count reconciliation:** `grep -c '^warning:' 34-evidence/34-06-cargo-doc-default.txt` → **73**
total `warning:`-prefixed lines. Of those, **8** are per-crate summary lines (`` `<crate>` (lib
doc) generated N warnings ``, D-13's own instruction to skip these as totals, not findings) —
`paladin-ai` 5, `paladin-web` 3, `paladin-battalion` 36, `paladin-storage` 1, `paladin-llm` 4,
`paladin-ports` 1, `paladin-ai-core` 14, `paladin-memory` 1 (5+3+36+1+4+1+14+1 = 65, confirmed
against the 8 summary lines' own stated counts). The remaining **65** are content diagnostics, and
**65** rows are enumerated below — `bash 34-rustdoc-rows.sh 34-evidence/34-06-cargo-doc-default.txt
default` prints `RECONCILED: 65 content diagnostics == 65 rows emitted` on its stderr and exits 0.

**Drift against the two prior HEAD SHA counts D-13 names:** this run's **73** total `warning:` line
count is unchanged from **73 at Phase 33 close** (33-05-SUMMARY.md carried commit, STATE.md's
Phase 33 close note) and up **1** from **72 at Phase 29** close. The count has not moved since
Phase 33; this phase does not attribute the earlier 72→73 movement to any specific commit (D-13
only asks that drift be visible, not diagnosed), and this run's own re-measurement confirms 73 is
still current rather than stale.

**Method — `34-rustdoc-rows.sh` (RESEARCH.md Pattern 3, Pitfall P-01):** cargo documents crates
concurrently; a crate's diagnostics are flushed as one contiguous block but crates finishing near
the same moment have their `generated N warnings` summary lines batched together at the end of the
shared stream segment (verified live: `paladin-ai`'s 5 and `paladin-web`'s 3 diagnostics are
interleaved as one 8-diagnostic run before their two summaries; `paladin-battalion`'s 36,
`paladin-storage`'s 1 and `paladin-llm`'s 4 likewise share one 41-diagnostic run). The script
attributes each diagnostic to a crate by consuming the pending queue front-to-back against each
summary's own count, in the order the summaries appear — never by stream position alone. For a
block carrying rustdoc's own `-->` span, File:line is read directly (Location source: `rustdoc
span`). For the 36 of 65 blocks carrying no `-->` at all (P-01's majority case — here 34 of 36
`unresolved link` diagnostics plus both `unclosed HTML tag` diagnostics), the script recovers
File:line by `grep -rnF` for the exact source-line snippet rustdoc quotes under its own `= note:
the link appears in this line:` note (or, for `unclosed HTML tag`, the bracket-identifier form),
restricted to doc-comment lines (`///`/`//!`) inside the *attributed* crate's own `src/` tree —
never the whole workspace — which is why every one of the 36 recovered rows below resolves to
**exactly one** match with no ambiguity (confirmed live: two separate `TraceRecord` warnings
correctly resolve to two different lines, `trace.rs:6` and `trace.rs:17`, rather than collapsing to
one — the snippet, not the bare identifier alone, is what disambiguates them; two `unclosed HTML
tag` warnings with no snippet note at all, `<status>`/`<body>`, independently resolve to
`crates/paladin-llm/src/http_status.rs:6-7`, matching the crate the summary-line chunking already
assigned them to).

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-02 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai | src/application/services/paladin/paladin_execution_service.rs:1014 | private intra-doc link | `warning: public documentation for `execute_scoped` links to private item `Self::execute_bounded`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:1 | S |
| RD-03 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai | src/application/services/parley/adapter.rs:28 | private intra-doc link | `warning: public documentation for `adapter` links to private item `shadow_validate`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:10 | S |
| RD-04 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai | src/application/services/run/worker.rs:641 | private intra-doc link | `warning: public documentation for `with_event_bus` links to private item `Self::record_engine_failure`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:18 | S |
| RD-05 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai | src/config/agent_runtime.rs:1174 | private intra-doc link | `warning: public documentation for `resolve_chain` links to private item `KNOWN_PROVIDER_NAMES`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:26 | S |
| RD-06 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai | src/presets/mod.rs:55 | private intra-doc link | `warning: public documentation for `ReasoningAgentOptions` links to private item `DEFAULT_SYSTEM_PROMPT`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:34 | S |
| RD-07 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-web | crates/paladin-web/src/thread_controller.rs:483 | private intra-doc link | `warning: public documentation for `limit` links to private item `MAX_HISTORY_LIMIT`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:42 | S |
| RD-08 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-web | crates/paladin-web/src/thread_controller.rs:686 | private intra-doc link | `warning: public documentation for `resume_thread` links to private item `map_parley_error`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:51 | S |
| RD-09 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-web | crates/paladin-web/src/thread_controller.rs:757 | private intra-doc link | `warning: public documentation for `get_thread_history` links to private item `MAX_HISTORY_LIMIT`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:59 | S |
| RD-10 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/commander.rs:35 | private intra-doc link | `warning: public documentation for `StrategySelection` links to private item `Commander::analyze_and_select`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:69 | S |
| RD-11 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/commander.rs:49 | private intra-doc link | `warning: public documentation for `Heuristic` links to private item `Commander::analyze_and_select`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:78 | S |
| RD-12 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/edge_evaluator.rs:3 | unresolved link | `warning: unresolved link to `EdgeCondition`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/edge_evaluator.rs:3`) | 34-evidence/34-06-cargo-doc-default.txt:86 | S |
| RD-13 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:3 | unresolved link | `warning: unresolved link to `WarGraph`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:3`) | 34-evidence/34-06-cargo-doc-default.txt:96 | S |
| RD-14 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:4 | unresolved link | `warning: unresolved link to `StateNode`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:4`) | 34-evidence/34-06-cargo-doc-default.txt:105 | S |
| RD-15 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:5 | unresolved link | `warning: unresolved link to `Battlefield`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:5`) | 34-evidence/34-06-cargo-doc-default.txt:114 | S |
| RD-16 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:6 | unresolved link | `warning: unresolved link to `Waypoint`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:6`) | 34-evidence/34-06-cargo-doc-default.txt:123 | S |
| RD-17 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:6 | unresolved link | `warning: unresolved link to `WaypointPort`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:6`) | 34-evidence/34-06-cargo-doc-default.txt:132 | S |
| RD-18 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:10 | unresolved link | `warning: unresolved link to `WarEngine::start`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:10`) | 34-evidence/34-06-cargo-doc-default.txt:141 | S |
| RD-19 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:20 | unresolved link | `warning: unresolved link to `bridges`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:20`) | 34-evidence/34-06-cargo-doc-default.txt:149 | S |
| RD-20 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:25 | unresolved link | `warning: unresolved link to `graph`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:25`) | 34-evidence/34-06-cargo-doc-default.txt:158 | S |
| RD-21 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:27 | unresolved link | `warning: unresolved link to `directive_parser`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:27`) | 34-evidence/34-06-cargo-doc-default.txt:167 | S |
| RD-22 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:31 | unresolved link | `warning: unresolved link to `input_mapping`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:31`) | 34-evidence/34-06-cargo-doc-default.txt:176 | S |
| RD-23 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:33 | unresolved link | `warning: unresolved link to `node`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:33`) | 34-evidence/34-06-cargo-doc-default.txt:185 | S |
| RD-24 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:34 | unresolved link | `warning: unresolved link to `dispatch_registry`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:34`) | 34-evidence/34-06-cargo-doc-default.txt:194 | S |
| RD-25 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:36 | unresolved link | `warning: unresolved link to `hooks`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:36`) | 34-evidence/34-06-cargo-doc-default.txt:203 | S |
| RD-26 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/cache_key.rs:23 | unresolved link | `warning: unresolved link to `graph_prefix`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/cache_key.rs:23`) | 34-evidence/34-06-cargo-doc-default.txt:212 | S |
| RD-27 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/cache_key.rs:23 | unresolved link | `warning: unresolved link to `node_prefix`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/cache_key.rs:23`) | 34-evidence/34-06-cargo-doc-default.txt:221 | S |
| RD-28 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/directive_parser.rs:47 | private intra-doc link | `warning: public documentation for `directive_parser` links to private item `crate::engine::graph::validate_parley_value_for_kind`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:230 | S |
| RD-29 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:772 | private intra-doc link | `warning: public documentation for `validate` links to private item `WarGraph::validate_schedulable`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:238 | S |
| RD-30 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:1375 | private intra-doc link | `warning: public documentation for `validate_node_cache_backend` links to private item `WarGraph::validate_aegis_undeclared_nodes`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:246 | S |
| RD-31 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2258 | private intra-doc link | `warning: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:254 | S |
| RD-32 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2287 | private intra-doc link | `warning: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:262 | S |
| RD-33 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2303 | private intra-doc link | `warning: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:270 | S |
| RD-34 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2337 | private intra-doc link | `warning: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:278 | S |
| RD-35 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2358 | private intra-doc link | `warning: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:286 | S |
| RD-36 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:744 | private intra-doc link | `warning: public documentation for `ResponseShapeInvalid` links to private item `graph::validate_parley_value_for_kind`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:294 | S |
| RD-37 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:1423 | unresolved link | `warning: unresolved link to `Waypoint`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:302 | S |
| RD-38 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:2686 | private intra-doc link | `warning: public documentation for `replay` links to private item `superstep::run_with_namespace`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:310 | S |
| RD-39 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/llm_decision.rs:40 | unresolved link | `warning: unresolved link to `llm_error_class`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_decision.rs:40`) | 34-evidence/34-06-cargo-doc-default.txt:318 | S |
| RD-40 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:1 | unresolved link | `warning: unresolved link to `PaladinError::LlmFailure`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:1`) | 34-evidence/34-06-cargo-doc-default.txt:327 | S |
| RD-41 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:8 | unresolved link | `warning: unresolved link to `PaladinError::LlmFailure`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:8`) | 34-evidence/34-06-cargo-doc-default.txt:335 | S |
| RD-42 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:38 | unresolved link | `warning: unresolved link to `PaladinError::is_retryable`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:38`) | 34-evidence/34-06-cargo-doc-default.txt:343 | S |
| RD-43 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/input_mapping.rs:30 | redundant explicit link | `warning: redundant explicit link target` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:351 | S |
| RD-44 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/input_mapping.rs:40 | redundant explicit link | `warning: redundant explicit link target` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:368 | S |
| RD-45 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:1085 | redundant explicit link | `warning: redundant explicit link target` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:384 | S |
| RD-46 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-storage | crates/paladin-storage/src/waypoint/contract_tests.rs:673 | private intra-doc link | `warning: public documentation for `muster_progress_round_trips` links to private item `muster_progress_fixture`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:400 | S |
| RD-47 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-llm | crates/paladin-llm/src/redaction.rs:164 | private intra-doc link | `warning: public documentation for `redact_secret_patterns` links to private item `JWT_MIN_SEGMENT_LEN`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:410 | S |
| RD-48 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-llm | crates/paladin-llm/src/services/commissary.rs:89 | private intra-doc link | `warning: public documentation for `pessimistic_tokens_per_1000_bytes` links to private item `PESSIMISTIC_TOKENS_PER_1000_BYTES`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:419 | S |
| RD-49 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-llm | crates/paladin-llm/src/http_status.rs:6 | unclosed HTML tag | `warning: unclosed HTML tag `status`` | grep recovery (`grep -rnF "<status>" crates/paladin-llm/src`, resolved to `crates/paladin-llm/src/http_status.rs:6`) | 34-evidence/34-06-cargo-doc-default.txt:427 | S |
| RD-50 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-llm | crates/paladin-llm/src/http_status.rs:7 | unclosed HTML tag | `warning: unclosed HTML tag `body`` | grep recovery (`grep -rnF "<body>" crates/paladin-llm/src`, resolved to `crates/paladin-llm/src/http_status.rs:7`) | 34-evidence/34-06-cargo-doc-default.txt:431 | S |
| RD-51 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ports | crates/paladin-ports/src/output/structured_executor_port.rs:158 | private intra-doc link | `warning: public documentation for `run_structured` links to private item `repair_prompt`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:436 | S |
| RD-52 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/directive.rs:3 | unresolved link | `warning: unresolved link to `StateNode::run`` | rustdoc span | 34-evidence/34-06-cargo-doc-default.txt:445 | S |
| RD-53 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/structured.rs:13 | unresolved link | `warning: unresolved link to `extract_json`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/structured.rs:13`) | 34-evidence/34-06-cargo-doc-default.txt:453 | S |
| RD-54 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:3 | unresolved link | `warning: unresolved link to `TraceEvent`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:3`) | 34-evidence/34-06-cargo-doc-default.txt:462 | S |
| RD-55 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:6 | unresolved link | `warning: unresolved link to `TraceRecord`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:6`) | 34-evidence/34-06-cargo-doc-default.txt:471 | S |
| RD-56 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:17 | unresolved link | `warning: unresolved link to `TraceRecord`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:17`) | 34-evidence/34-06-cargo-doc-default.txt:480 | S |
| RD-57 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:21 | unresolved link | `warning: unresolved link to `TraceEvent::DeltaMerged`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:21`) | 34-evidence/34-06-cargo-doc-default.txt:489 | S |
| RD-58 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:22 | unresolved link | `warning: unresolved link to `FieldChange`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:22`) | 34-evidence/34-06-cargo-doc-default.txt:497 | S |
| RD-59 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:25 | unresolved link | `warning: unresolved link to `FieldChange::value`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:25`) | 34-evidence/34-06-cargo-doc-default.txt:506 | S |
| RD-60 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:36 | unresolved link | `warning: unresolved link to `TraceEvent::NodeProgress`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:36`) | 34-evidence/34-06-cargo-doc-default.txt:514 | S |
| RD-61 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:37 | unresolved link | `warning: unresolved link to `TraceEvent::ParleyRaised`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:37`) | 34-evidence/34-06-cargo-doc-default.txt:522 | S |
| RD-62 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:44 | unresolved link | `warning: unresolved link to `TraceEvent::NodeProgress`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:44`) | 34-evidence/34-06-cargo-doc-default.txt:530 | S |
| RD-63 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:45 | unresolved link | `warning: unresolved link to `TraceEvent::ParleyRaised`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:45`) | 34-evidence/34-06-cargo-doc-default.txt:538 | S |
| RD-64 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/webhook.rs:19 | unresolved link | `warning: unresolved link to `WebhookDelivery`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/webhook.rs:19`) | 34-evidence/34-06-cargo-doc-default.txt:546 | S |
| RD-65 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-ai-core | crates/paladin-core/src/platform/container/webhook.rs:20 | unresolved link | `warning: unresolved link to `WEBHOOK_DELIVERY_SCHEMA_VERSION`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/webhook.rs:20`) | 34-evidence/34-06-cargo-doc-default.txt:555 | S |
| RD-66 | `cargo doc --workspace --no-deps` (D-12/D-13) | paladin-memory | crates/paladin-memory/src/token_counter/mod.rs:3 | unresolved link | `warning: unresolved link to `HeuristicTokenCounter`` | grep recovery — not re-derived here; matches the known-answer row worked at the top of this section exactly (same file:line, same `HeuristicTokenCounter` `//!`-comment link this run's own warning stream also reports) | 34-evidence/34-06-cargo-doc-default.txt:566 | S |

**Kind distribution (65 rows):** 24 private intra-doc link, 36 unresolved link, 3 redundant
explicit link, 2 unclosed HTML tag, 0 missing docs, 0 other — every row's Kind cell holds one of
the six D-13 kinds, and none fell through to `other` (every diagnostic in this capture matched a
named class from its own first line).

**WINDOWS.md cross-check (D-00d — rows read, never edited):** row 36 (`cargo doc --workspace
--no-deps emits 16 pre-existing warnings ... see deferred-items.md Plan 31-05 entry`, phase 31,
`open`) is a workspace-wide summary observation, not a single findable diagnostic — its subject (the
default-feature warning set existing at all) is the entire enumeration above, not one row; it
remains accurate in substance (warnings still exist) though its own count (16) is stale against
this run's 65 content diagnostics / 73 total lines, a drift this row's own text does not claim to
track. Row 37 (the `HeuristicTokenCounter` broken link, phase 32, `open`) appears explicitly in the
enumeration below — the table's own last row, matching the WINDOWS.md row's `file`/`line` cells
(`crates/paladin-memory/src/token_counter/mod.rs`, `3`) exactly. Both rows remain `open` through
this phase — neither is edited, moved, or waived here (D-00c/D-00d); Phase 36 is their named closer
per this section's own header note.

### Workspace all-features run (plan 34-06, D-12/D-14) — a floor, not an enumeration

**Command, quoted verbatim (D-12):**

```
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

**This is what the `--workspace` invocation of the all-features bar surfaces — a partial view, not
the enumeration.** No `RD-nn` row is minted from this run. Its findings are a subset of the
per-crate `-D warnings --all-features` sweep plan 34-07 performs crate-by-crate; minting rows here
would double-count them against that enumeration. The `RD` row count in this file is unchanged from
the end of the previous subsection (66 rows total, the tracer plus the full default-feature
enumeration) by this subsection's own edit.

**Measured result (re-run live, this plan's own HEAD `d81de538d5697c215eb5cad346077f75fddabe1b`):**
exit `101`; wall time 34s; raw capture
`34-evidence/34-06-cargo-doc-allfeatures-workspace.txt` (164 lines, post-hook trailing-whitespace normalization).

**Crates that reported errors before the abort, and how many errors each reported** (`grep -c
'^error:' ` on the capture returns 21 total `error:`-prefixed lines; 4 are `error: could not
document \`<crate>\`` boundary lines, leaving **17** content errors, attributed per-error by its own
`-->` path where present and — for the 4 carrying none — by the same crate-scoped
quoted-snippet grep recovery `34-rustdoc-rows.sh` uses, confirmed against the live tree):

| Crate | Content errors | Evidence |
|-------|-----------------|----------|
| `paladin-memory` | 1 | `unresolved link to \`HeuristicTokenCounter\`` — the same known-answer case worked at the top of this section and re-confirmed in the default-feature enumeration above |
| `paladin-web` | 8 | 3 location-less (`RunInspectorPort` → `crates/paladin-web/src/dev_ui_controller.rs:3`; `dev_ui_inspector_page` → `:20`; `InspectorView::supersteps` → `:28`, all confirmed by snippet grep restricted to `crates/paladin-web/src`) + 5 `-->`-bearing (`dev_ui_controller.rs:69`, `:131`, `thread_controller.rs:483`, `:686`, `:757`) |
| `paladin-storage` | 1 | `-->` `crates/paladin-storage/src/waypoint/contract_tests.rs:673` |
| `paladin-ai` (facade) | 7 | all `-->`-bearing, all under `src/` (`cli/commands/eval.rs:281`, `application/services/paladin/paladin_execution_service.rs:1014`, `application/services/parley/adapter.rs:28`, `application/services/run/worker.rs:641`, `config/agent_runtime.rs:1174`, `infrastructure/telemetry/otel_sink.rs:42`, `presets/mod.rs:55`) |

`1 + 8 + 1 + 7 = 17`, reconciling exactly against the capture's own content-error count.

**The concurrency-driven abort behaviour, stated in plain terms:** cargo documents independent
crates concurrently under `--workspace`. When one job's diagnostics trip `-D warnings`, cargo stops
*scheduling new* documentation jobs but lets every job already in flight finish and report its own
errors before the whole invocation exits `101`. The stream itself is **not** reliably ordered by
crate — `error: could not document \`paladin-memory\`` prints as the *first* abort boundary in this
capture even though the bulk of the errors preceding it (`RunInspectorPort`, `dev_ui_inspector_page`,
`InspectorView::supersteps`, plus all five `-->`-bearing `paladin-web` errors) belong to
`paladin-web`, not `paladin-memory` — confirmed only by re-deriving each error's true crate from its
own `-->` path or grep-recovered location, never by its position in the stream relative to the
nearest `could not document` line. **Which crates and how many errors this run shows therefore
depends on scheduling, and is a floor, not a total** — a different run, or the same command at a
different HEAD, can surface an entirely different subset of failing crates before the same abort
point.

**This explicitly corrects D-14's own prose.** `34-CONTEXT.md` D-14 states the run "aborts at the
first failing crate in build order (`paladin-ai-core`, 14 unresolved links at Phase 32-05)" — a
single-crate framing. RESEARCH.md's Pitfall P-02 already measured this wrong once (3 crates —
`paladin-ports`, `paladin-ai-core`, `paladin-storage`, 16 errors — at its own HEAD `c36b7729`); this
plan's own independent re-run finds a **third**, entirely different 4-crate set (`paladin-memory`,
`paladin-web`, `paladin-storage`, `paladin-ai`, 17 errors) that does not even include
`paladin-ai-core` at all. Three independent measurements, three different abort sets — the pattern
itself (concurrency-dependent, non-deterministic scheduling) is now confirmed twice over, not just
once. The per-crate sweep (Pattern 2 — plan 34-07) is never optional; it is the only accurate
enumeration, and this subsection's own finding reinforces rather than merely repeats that
conclusion.

**Drift against the Phase 32 close figure:** `32-05-SUMMARY.md` recorded "the build fails at the
very first crate in build order, `paladin-ai-core`, with 14 unresolved-intra-doc-link errors" —
a figure scoped to one crate from a run that happened to abort there first, **never claimed or
presented as a workspace-wide total**. This run does not reach `paladin-ai-core` at all (a
different 4-crate subset aborted first this time), so this command cannot confirm or refute whether
`paladin-ai-core`'s own count is still 14 — only the per-crate sweep (`RUSTDOCFLAGS="-D warnings"
cargo doc -p paladin-ai-core --all-features --no-deps`, plan 34-07) can settle that directly. No
figure this run did not itself produce is carried forward as current.

### Per-crate `-D warnings --all-features` sweep (plan 34-07, D-12/D-14) — the true floor

**Crate list, derived from the tree, not copied from RESEARCH.md or CONTEXT.md:** `ls crates/`
lists twelve directories; `paladin-doc-examples` (`crates/doc-examples`, `publish = false`) is
excluded — D-14 scopes this sweep to "the eleven library crates plus the facade", and
`doc-examples` is a compile-verified example holder (§4's own subject, plan 34-08), not a library
crate the `-D warnings --all-features` bar applies to. The remaining eleven directories'
`[package] name` fields plus the root `Cargo.toml`'s facade package name give the twelve
invocation targets, matching RESEARCH.md Pattern 2's list exactly: `paladin-battalion`,
`paladin-content`, `paladin-ai-core` (directory `crates/paladin-core`), `paladin-eval`,
`paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`,
`paladin-storage`, `paladin-web`, facade `paladin-ai`.

**Method extension (Rule 3 — blocking-issue auto-fix, this task's own precondition check):** the
task's precondition asked for a dry run against 34-06's own default-feature capture before
trusting the sweep; that dry run (re-running the parser against 34-06's own committed
capture, unmodified) reproduced the identical, already-committed 65-row output exactly — but the
*equivalent* dry run against a fresh single-crate capture (`paladin-memory`, the known-answer
case) failed with `FATAL: 1 diagnostic block(s) never attributed to a crate` — `34-rustdoc-rows.sh`
(written by plan 34-06) attributes every diagnostic block to a crate by consuming a per-crate
"`<crate>` (lib doc) generated N warnings/errors" summary line, a mechanism the `--workspace`
capture always emits but a single-crate `cargo doc -p <crate> --all-features --no-deps` invocation
never does: under `-D warnings` the job aborts on its first content diagnostic before any summary
line would print at all, and a clean (0-error) crate's own closing line ("Generated
.../index.html") does not match the summary pattern either — confirmed live against both a red
capture (`paladin-memory`, 1 error) and a green one (`paladin-herald`, 0 errors) before any of the
remaining ten-crate sweep ran. Per the precondition's own instruction, the parser was extended
rather than the rows hand-transcribed: `34-rustdoc-rows.sh` now accepts an optional third
argument, `<crate-override>`, which bypasses the summary-line attribution pass entirely and
assigns every parsed block directly to the named crate — correct by construction, since a `-p
<crate>` invocation can only ever emit diagnostics for that one crate. The existing default-feature
(`--workspace`) code path plan 34-06 exercised is unchanged by this edit — re-running the parser
against `34-evidence/34-06-cargo-doc-default.txt` without the new argument reproduces the identical
65-row, byte-identical output already pasted into the subsection above (confirmed by diff before
this subsection's own rows were added). Two further corrections landed in the same edit: the Run
cell for a crate-override run now quotes the actual `-p <crate>` invocation (D-12/D-14) instead of
the workspace command's D-12/D-13 text, and the evidence-anchor path now recovers the full
`34-evidence/...` suffix from the capture's own absolute path rather than assuming every capture
sits one level below `34-evidence/` directly — this task's captures sit a level deeper, at
`34-evidence/34-07-percrate/<crate>.txt`, which the old basename-only anchor would have silently
mis-pointed.

**Known-answer verification (CONTEXT.md's own method self-test, checked before trusting any other
row below):** the `paladin-memory` sweep's one row resolves to
`crates/paladin-memory/src/token_counter/mod.rs:3` — the same file:line the tracer row worked at
the top of this section and the default-feature enumeration's own last row both already carry.
Verified before the remaining ten crates were swept, per the plan's own instruction to stop and
fix rather than continue past a mismatch.

**Twelve invocations, run live at HEAD `f53daa8a845c34433fcd30ff0867348105b8330c`** (warm
`target/`, well under RESEARCH.md's cold-cache ~4m47s combined estimate):

| Crate | Invocation | Exit | Content errors | Wall time | Capture |
|-------|-----------|------|-----------------|-----------|---------|
| `paladin-battalion` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` | 101 | 36 | 4s | 34-evidence/34-07-percrate/paladin-battalion.txt |
| `paladin-content` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-content --all-features --no-deps` | 0 | 0 | 4s | 34-evidence/34-07-percrate/paladin-content.txt |
| `paladin-ai-core` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` | 101 | 14 | 7s | 34-evidence/34-07-percrate/paladin-ai-core.txt |
| `paladin-eval` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-eval --all-features --no-deps` | 0 | 0 | 5s | 34-evidence/34-07-percrate/paladin-eval.txt |
| `paladin-herald` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-herald --all-features --no-deps` | 0 | 0 | 4s | 34-evidence/34-07-percrate/paladin-herald.txt |
| `paladin-llm` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` | 101 | 9 | 4s | 34-evidence/34-07-percrate/paladin-llm.txt |
| `paladin-memory` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps` | 101 | 1 | 4s | 34-evidence/34-07-percrate/paladin-memory.txt |
| `paladin-notifications` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-notifications --all-features --no-deps` | 0 | 0 | 4s | 34-evidence/34-07-percrate/paladin-notifications.txt |
| `paladin-ports` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ports --all-features --no-deps` | 101 | 1 | 4s | 34-evidence/34-07-percrate/paladin-ports.txt |
| `paladin-storage` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-storage --all-features --no-deps` | 101 | 1 | 6s | 34-evidence/34-07-percrate/paladin-storage.txt |
| `paladin-web` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` | 101 | 8 | 4s | 34-evidence/34-07-percrate/paladin-web.txt |
| `paladin-ai` | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` | 101 | 7 | 18s | 34-evidence/34-07-percrate/paladin-ai.txt |
| **Total** | — | — | **77** | **68s** | — |

**8 of 12 crates are RED** (`paladin-battalion`, `paladin-ai-core`, `paladin-llm`, `paladin-memory`, `paladin-ports`, `paladin-storage`, `paladin-web`, `paladin-ai`); the other 4
(`paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-notifications`) are green with zero
content errors and zero rows. This run's totals match RESEARCH.md Pattern 2's table exactly,
crate-for-crate and error-for-error (77 total, the same 8 red crates), at a HEAD several commits
past RESEARCH.md's own measurement SHA `c36b7729` — the D-22 invariance argument holds: every
intervening Phase 34 commit touches only `.planning/`, so the source tree the two SHAs measure is
identical.

**This is the true floor D-14 asks for — 77, not 14 and not 17.** The figure the
roadmap (`ROADMAP.md`'s "Phase 36" goal statement and its Source line), this phase's own
`34-CONTEXT.md` D-14 prose, and `STATE.md`'s Phase 32 close note all still carry ("14 unresolved
intra-doc links [in `paladin-ai-core`]") is `paladin-ai-core`'s own count alone — a figure Phase
32-05's own summary never claimed as a workspace total, and one plan 34-06's workspace-run
subsection above already flagged as undersized against its own 17-error partial view. Against the
77-error true floor, the carried "14" figure would have undersized Phase 36's `RD-nn`
work list by 63 items (a roughly 5.5x undercount, 77 / 14 ≈ 5.5); even this plan's own
predecessor subsection's 17-error four-crate workspace-run floor would have undersized the list by
60 items. The per-crate sweep is not a refinement of either number — it is the only way to reach
the true one at all, since eight of the twelve red crates never all appear together in any single
`--workspace` invocation's partial output before that invocation's own concurrency-driven abort
(Pitfall P-02, corrected again by this task's own measurement: this run's per-crate sweep found
red crates — `paladin-battalion`, `paladin-llm` — that neither RESEARCH.md's 3-crate workspace
abort nor plan 34-06's own 4-crate workspace abort ever reached).

**HEAD SHA and D-23 note:** measured at `f53daa8a845c34433fcd30ff0867348105b8330c`, past the Phase
34 start SHA `ee1fb160f8e743e638b32beb6c4e32be4ede9325` recorded in the Measurement Header above;
every intervening commit touches only `.planning/` (this phase's own SC5 constraint), so per
D-23's own invariance argument the source tree these 77 errors were measured against
is identical to the tree at the phase's start SHA.

#### `paladin-battalion` — 36 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` — exit `101`, 4s, capture `34-evidence/34-07-percrate/paladin-battalion.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-67 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/commander.rs:35 | private intra-doc link | `error: public documentation for `StrategySelection` links to private item `Commander::analyze_and_select`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:2 | S |
| RD-68 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/commander.rs:49 | private intra-doc link | `error: public documentation for `Heuristic` links to private item `Commander::analyze_and_select`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:12 | S |
| RD-69 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/edge_evaluator.rs:3 | unresolved link | `error: unresolved link to `EdgeCondition`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/edge_evaluator.rs:3`) | 34-evidence/34-07-percrate/paladin-battalion.txt:20 | S |
| RD-70 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:3 | unresolved link | `error: unresolved link to `WarGraph`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:3`) | 34-evidence/34-07-percrate/paladin-battalion.txt:31 | S |
| RD-71 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:4 | unresolved link | `error: unresolved link to `StateNode`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:4`) | 34-evidence/34-07-percrate/paladin-battalion.txt:40 | S |
| RD-72 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:5 | unresolved link | `error: unresolved link to `Battlefield`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:5`) | 34-evidence/34-07-percrate/paladin-battalion.txt:49 | S |
| RD-73 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:6 | unresolved link | `error: unresolved link to `Waypoint`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:6`) | 34-evidence/34-07-percrate/paladin-battalion.txt:58 | S |
| RD-74 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:6 | unresolved link | `error: unresolved link to `WaypointPort`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:6`) | 34-evidence/34-07-percrate/paladin-battalion.txt:67 | S |
| RD-75 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:10 | unresolved link | `error: unresolved link to `WarEngine::start`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:10`) | 34-evidence/34-07-percrate/paladin-battalion.txt:76 | S |
| RD-76 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:20 | unresolved link | `error: unresolved link to `bridges`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:20`) | 34-evidence/34-07-percrate/paladin-battalion.txt:84 | S |
| RD-77 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:25 | unresolved link | `error: unresolved link to `graph`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:25`) | 34-evidence/34-07-percrate/paladin-battalion.txt:93 | S |
| RD-78 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:27 | unresolved link | `error: unresolved link to `directive_parser`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:27`) | 34-evidence/34-07-percrate/paladin-battalion.txt:102 | S |
| RD-79 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:31 | unresolved link | `error: unresolved link to `input_mapping`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:31`) | 34-evidence/34-07-percrate/paladin-battalion.txt:111 | S |
| RD-80 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:33 | unresolved link | `error: unresolved link to `node`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:33`) | 34-evidence/34-07-percrate/paladin-battalion.txt:120 | S |
| RD-81 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:34 | unresolved link | `error: unresolved link to `dispatch_registry`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:34`) | 34-evidence/34-07-percrate/paladin-battalion.txt:129 | S |
| RD-82 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:36 | unresolved link | `error: unresolved link to `hooks`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/mod.rs:36`) | 34-evidence/34-07-percrate/paladin-battalion.txt:138 | S |
| RD-83 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/cache_key.rs:23 | unresolved link | `error: unresolved link to `graph_prefix`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/cache_key.rs:23`) | 34-evidence/34-07-percrate/paladin-battalion.txt:147 | S |
| RD-84 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/cache_key.rs:23 | unresolved link | `error: unresolved link to `node_prefix`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/engine/cache_key.rs:23`) | 34-evidence/34-07-percrate/paladin-battalion.txt:156 | S |
| RD-85 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/directive_parser.rs:47 | private intra-doc link | `error: public documentation for `directive_parser` links to private item `crate::engine::graph::validate_parley_value_for_kind`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:165 | S |
| RD-86 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:772 | private intra-doc link | `error: public documentation for `validate` links to private item `WarGraph::validate_schedulable`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:173 | S |
| RD-87 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:1375 | private intra-doc link | `error: public documentation for `validate_node_cache_backend` links to private item `WarGraph::validate_aegis_undeclared_nodes`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:181 | S |
| RD-88 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2258 | private intra-doc link | `error: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:189 | S |
| RD-89 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2287 | private intra-doc link | `error: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:197 | S |
| RD-90 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2303 | private intra-doc link | `error: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:205 | S |
| RD-91 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2337 | private intra-doc link | `error: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:213 | S |
| RD-92 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/graph.rs:2358 | private intra-doc link | `error: public documentation for `fingerprint` links to private item `push_field`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:221 | S |
| RD-93 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:744 | private intra-doc link | `error: public documentation for `ResponseShapeInvalid` links to private item `graph::validate_parley_value_for_kind`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:229 | S |
| RD-94 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:1423 | unresolved link | `error: unresolved link to `Waypoint`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:237 | S |
| RD-95 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:2686 | private intra-doc link | `error: public documentation for `replay` links to private item `superstep::run_with_namespace`` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:245 | S |
| RD-96 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/llm_decision.rs:40 | unresolved link | `error: unresolved link to `llm_error_class`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_decision.rs:40`) | 34-evidence/34-07-percrate/paladin-battalion.txt:253 | S |
| RD-97 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:1 | unresolved link | `error: unresolved link to `PaladinError::LlmFailure`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:1`) | 34-evidence/34-07-percrate/paladin-battalion.txt:262 | S |
| RD-98 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:8 | unresolved link | `error: unresolved link to `PaladinError::LlmFailure`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:8`) | 34-evidence/34-07-percrate/paladin-battalion.txt:270 | S |
| RD-99 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/llm_failure.rs:38 | unresolved link | `error: unresolved link to `PaladinError::is_retryable`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-battalion/src`, resolved to `crates/paladin-battalion/src/llm_failure.rs:38`) | 34-evidence/34-07-percrate/paladin-battalion.txt:278 | S |
| RD-100 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/input_mapping.rs:30 | redundant explicit link | `error: redundant explicit link target` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:286 | S |
| RD-101 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/input_mapping.rs:40 | redundant explicit link | `error: redundant explicit link target` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:304 | S |
| RD-102 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-battalion --all-features --no-deps` (D-12/D-14) | paladin-battalion | crates/paladin-battalion/src/engine/mod.rs:1085 | redundant explicit link | `error: redundant explicit link target` | rustdoc span | 34-evidence/34-07-percrate/paladin-battalion.txt:320 | S |

Row count check: 36 rows above == 36 content errors recorded in the summary table for `paladin-battalion`.

#### `paladin-content` — 0 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-content --all-features --no-deps` — exit `0`, 4s, capture `34-evidence/34-07-percrate/paladin-content.txt`.

No content errors — 0 rows minted (green crate).

#### `paladin-ai-core` — 14 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` — exit `101`, 7s, capture `34-evidence/34-07-percrate/paladin-ai-core.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-103 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/directive.rs:3 | unresolved link | `error: unresolved link to `StateNode::run`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai-core.txt:2 | S |
| RD-104 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/structured.rs:13 | unresolved link | `error: unresolved link to `extract_json`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/structured.rs:13`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:11 | S |
| RD-105 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:3 | unresolved link | `error: unresolved link to `TraceEvent`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:3`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:20 | S |
| RD-106 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:6 | unresolved link | `error: unresolved link to `TraceRecord`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:6`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:29 | S |
| RD-107 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:17 | unresolved link | `error: unresolved link to `TraceRecord`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:17`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:38 | S |
| RD-108 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:21 | unresolved link | `error: unresolved link to `TraceEvent::DeltaMerged`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:21`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:47 | S |
| RD-109 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:22 | unresolved link | `error: unresolved link to `FieldChange`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:22`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:55 | S |
| RD-110 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:25 | unresolved link | `error: unresolved link to `FieldChange::value`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:25`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:64 | S |
| RD-111 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:36 | unresolved link | `error: unresolved link to `TraceEvent::NodeProgress`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:36`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:72 | S |
| RD-112 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:37 | unresolved link | `error: unresolved link to `TraceEvent::ParleyRaised`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:37`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:80 | S |
| RD-113 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:44 | unresolved link | `error: unresolved link to `TraceEvent::NodeProgress`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:44`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:88 | S |
| RD-114 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/trace.rs:45 | unresolved link | `error: unresolved link to `TraceEvent::ParleyRaised`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/trace.rs:45`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:96 | S |
| RD-115 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/webhook.rs:19 | unresolved link | `error: unresolved link to `WebhookDelivery`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/webhook.rs:19`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:104 | S |
| RD-116 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core --all-features --no-deps` (D-12/D-14) | paladin-ai-core | crates/paladin-core/src/platform/container/webhook.rs:20 | unresolved link | `error: unresolved link to `WEBHOOK_DELIVERY_SCHEMA_VERSION`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-core/src`, resolved to `crates/paladin-core/src/platform/container/webhook.rs:20`) | 34-evidence/34-07-percrate/paladin-ai-core.txt:113 | S |

Row count check: 14 rows above == 14 content errors recorded in the summary table for `paladin-ai-core`.

#### `paladin-eval` — 0 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-eval --all-features --no-deps` — exit `0`, 5s, capture `34-evidence/34-07-percrate/paladin-eval.txt`.

No content errors — 0 rows minted (green crate).

#### `paladin-herald` — 0 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-herald --all-features --no-deps` — exit `0`, 4s, capture `34-evidence/34-07-percrate/paladin-herald.txt`.

No content errors — 0 rows minted (green crate).

#### `paladin-llm` — 9 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` — exit `101`, 4s, capture `34-evidence/34-07-percrate/paladin-llm.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-117 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/redaction.rs:164 | private intra-doc link | `error: public documentation for `redact_secret_patterns` links to private item `JWT_MIN_SEGMENT_LEN`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:2 | S |
| RD-118 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/services/commissary.rs:89 | private intra-doc link | `error: public documentation for `pessimistic_tokens_per_1000_bytes` links to private item `PESSIMISTIC_TOKENS_PER_1000_BYTES`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:12 | S |
| RD-119 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/compat/engine.rs:114 | private intra-doc link | `error: public documentation for `temperature` links to private item `CompatEngine::build_request`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:20 | S |
| RD-120 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/compat/engine.rs:201 | private intra-doc link | `error: public documentation for `redirect_policy` links to private item `CompatEngine::map_error`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:28 | S |
| RD-121 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/compat/engine.rs:1041 | private intra-doc link | `error: public documentation for `available_models` links to private item `classify_fetch_failure`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:36 | S |
| RD-122 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/gemini/adapter.rs:28 | private intra-doc link | `error: public documentation for `adapter` links to private item `GeminiResponse`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:44 | S |
| RD-123 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/gemini/adapter.rs:59 | private intra-doc link | `error: public documentation for `adapter` links to private item `GeminiAdapter::map_error`` | rustdoc span | 34-evidence/34-07-percrate/paladin-llm.txt:52 | S |
| RD-124 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/http_status.rs:6 | unclosed HTML tag | `error: unclosed HTML tag `status`` | grep recovery (`grep -rnF "<status>" crates/paladin-llm/src`, resolved to `crates/paladin-llm/src/http_status.rs:6`) | 34-evidence/34-07-percrate/paladin-llm.txt:60 | S |
| RD-125 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-llm --all-features --no-deps` (D-12/D-14) | paladin-llm | crates/paladin-llm/src/http_status.rs:7 | unclosed HTML tag | `error: unclosed HTML tag `body`` | grep recovery (`grep -rnF "<body>" crates/paladin-llm/src`, resolved to `crates/paladin-llm/src/http_status.rs:7`) | 34-evidence/34-07-percrate/paladin-llm.txt:65 | S |

Row count check: 9 rows above == 9 content errors recorded in the summary table for `paladin-llm`.

#### `paladin-memory` — 1 content error

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps` — exit `101`, 4s, capture `34-evidence/34-07-percrate/paladin-memory.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-126 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --all-features --no-deps` (D-12/D-14) | paladin-memory | crates/paladin-memory/src/token_counter/mod.rs:3 | unresolved link | `error: unresolved link to `HeuristicTokenCounter`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-memory/src`, resolved to `crates/paladin-memory/src/token_counter/mod.rs:3`) | 34-evidence/34-07-percrate/paladin-memory.txt:2 | S |

Row count check: 1 rows above == 1 content errors recorded in the summary table for `paladin-memory`.

#### `paladin-notifications` — 0 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-notifications --all-features --no-deps` — exit `0`, 4s, capture `34-evidence/34-07-percrate/paladin-notifications.txt`.

No content errors — 0 rows minted (green crate).

#### `paladin-ports` — 1 content error

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ports --all-features --no-deps` — exit `101`, 4s, capture `34-evidence/34-07-percrate/paladin-ports.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-127 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ports --all-features --no-deps` (D-12/D-14) | paladin-ports | crates/paladin-ports/src/output/structured_executor_port.rs:158 | private intra-doc link | `error: public documentation for `run_structured` links to private item `repair_prompt`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ports.txt:2 | S |

Row count check: 1 rows above == 1 content errors recorded in the summary table for `paladin-ports`.

#### `paladin-storage` — 1 content error

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-storage --all-features --no-deps` — exit `101`, 6s, capture `34-evidence/34-07-percrate/paladin-storage.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-128 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-storage --all-features --no-deps` (D-12/D-14) | paladin-storage | crates/paladin-storage/src/waypoint/contract_tests.rs:673 | private intra-doc link | `error: public documentation for `muster_progress_round_trips` links to private item `muster_progress_fixture`` | rustdoc span | 34-evidence/34-07-percrate/paladin-storage.txt:2 | S |

Row count check: 1 rows above == 1 content errors recorded in the summary table for `paladin-storage`.

#### `paladin-web` — 8 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` — exit `101`, 4s, capture `34-evidence/34-07-percrate/paladin-web.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-129 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/dev_ui_controller.rs:3 | unresolved link | `error: unresolved link to `RunInspectorPort`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-web/src`, resolved to `crates/paladin-web/src/dev_ui_controller.rs:3`) | 34-evidence/34-07-percrate/paladin-web.txt:2 | S |
| RD-130 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/dev_ui_controller.rs:20 | unresolved link | `error: unresolved link to `dev_ui_inspector_page`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-web/src`, resolved to `crates/paladin-web/src/dev_ui_controller.rs:20`) | 34-evidence/34-07-percrate/paladin-web.txt:13 | S |
| RD-131 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/dev_ui_controller.rs:28 | unresolved link | `error: unresolved link to `InspectorView::supersteps`` | grep recovery (snippet quoted under "the link appears in this line:" — `grep -rnF "<snippet>" crates/paladin-web/src`, resolved to `crates/paladin-web/src/dev_ui_controller.rs:28`) | 34-evidence/34-07-percrate/paladin-web.txt:22 | S |
| RD-132 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/dev_ui_controller.rs:69 | private intra-doc link | `error: public documentation for `DevUiState` links to private item `NOT_WIRED_MESSAGE`` | rustdoc span | 34-evidence/34-07-percrate/paladin-web.txt:30 | S |
| RD-133 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/dev_ui_controller.rs:131 | private intra-doc link | `error: public documentation for `dev_ui_inspector_page` links to private item `escape_for_script`` | rustdoc span | 34-evidence/34-07-percrate/paladin-web.txt:40 | S |
| RD-134 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/thread_controller.rs:483 | private intra-doc link | `error: public documentation for `limit` links to private item `MAX_HISTORY_LIMIT`` | rustdoc span | 34-evidence/34-07-percrate/paladin-web.txt:48 | S |
| RD-135 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/thread_controller.rs:686 | private intra-doc link | `error: public documentation for `resume_thread` links to private item `map_parley_error`` | rustdoc span | 34-evidence/34-07-percrate/paladin-web.txt:56 | S |
| RD-136 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-web --all-features --no-deps` (D-12/D-14) | paladin-web | crates/paladin-web/src/thread_controller.rs:757 | private intra-doc link | `error: public documentation for `get_thread_history` links to private item `MAX_HISTORY_LIMIT`` | rustdoc span | 34-evidence/34-07-percrate/paladin-web.txt:64 | S |

Row count check: 8 rows above == 8 content errors recorded in the summary table for `paladin-web`.

#### `paladin-ai` — 7 content errors

`RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` — exit `101`, 18s, capture `34-evidence/34-07-percrate/paladin-ai.txt`.

| RD ID | Run | Crate | File:line | Kind | Message (verbatim first line) | Location source | Evidence anchor | Size |
|-------|-----|-------|-----------|------|-------------------------------|------------------|------------------|------|
| RD-137 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/application/cli/commands/eval.rs:281 | private intra-doc link | `error: public documentation for `first_divergence` links to private item `stabilized_fingerprints`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:2 | S |
| RD-138 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/application/services/paladin/paladin_execution_service.rs:1014 | private intra-doc link | `error: public documentation for `execute_scoped` links to private item `Self::execute_bounded`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:12 | S |
| RD-139 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/application/services/parley/adapter.rs:28 | private intra-doc link | `error: public documentation for `adapter` links to private item `shadow_validate`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:20 | S |
| RD-140 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/application/services/run/worker.rs:641 | private intra-doc link | `error: public documentation for `with_event_bus` links to private item `Self::record_engine_failure`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:28 | S |
| RD-141 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/config/agent_runtime.rs:1174 | private intra-doc link | `error: public documentation for `resolve_chain` links to private item `KNOWN_PROVIDER_NAMES`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:36 | S |
| RD-142 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/infrastructure/telemetry/otel_sink.rs:42 | private intra-doc link | `error: public documentation for `otel_sink` links to private item `build_reqwest_client`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:44 | S |
| RD-143 | `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai --all-features --no-deps` (D-12/D-14) | paladin-ai | src/presets/mod.rs:55 | private intra-doc link | `error: public documentation for `ReasoningAgentOptions` links to private item `DEFAULT_SYSTEM_PROMPT`` | rustdoc span | 34-evidence/34-07-percrate/paladin-ai.txt:52 | S |

Row count check: 7 rows above == 7 content errors recorded in the summary table for `paladin-ai`.

### Doctest baseline (plan 34-07, D-15)

**Command, quoted verbatim (D-15):**

```
cargo test --workspace --doc
```

Run under the default feature set only — RESEARCH.md Pitfall P-08 already found that widening the
feature set for this specific invocation trips a pre-existing, unrelated `cli_isolation` test
conflict (`deferred-items.md`, Phase 31/32) that has nothing to do with doctests; that wider
invocation is not run here, per D-15's own default-features instruction.

**Why this baseline exists at all:** the coverage gate and the `--tests` test selector both skip
doctests entirely, so a red doctest can sit unnoticed behind two green gates simultaneously, and
Phase 36 SC3 requires every doctest green — this command is the only place in the project's own
gate set that would ever catch one.

**Result (this run, HEAD `f53daa8a845c34433fcd30ff0867348105b8330c`):** exit `0`; **462 passed, 0
failed, 210 ignored** (summed across all 13 per-crate `test result:` lines in the capture); wall
time 32s. Raw capture: `34-evidence/34-07-doctests.txt`. This matches RESEARCH.md's own
independent measurement exactly (462 passed / 0 failed / 210 ignored).

**The run is fully green — no `RD-nn` row is minted for this subsection.** Per D-15's own
instruction, any failing doctest would be its own row (crate, item path, failure's first line);
none exists to record.

### Entry-point `# Examples`-heading gate (plan 34-07, D-15, D-00e)

**Commands, run in both modes, teed to `34-evidence/34-07-public-api-examples.txt`:**

```
bash scripts/check-public-api-examples.sh          # gate mode (default)
bash scripts/check-public-api-examples.sh --list   # report mode
```

**Gate mode:** exit `1`. **Report mode's own derivation: 101 entry points — 82 OK, 19 MISSING, 0
SINGULAR** (`TOTAL: 101 entry points -- 82 OK, 19 MISSING, 0 SINGULAR`, the script's own closing
line). The 19 MISSING violations, exactly as gate mode emitted them:

| Kind | Item | File:line | Violation |
|------|------|-----------|-----------|
| Port | `RunTracePort` | crates/paladin-ports/src/output/run_trace_port.rs:130 | MISSING |
| Port | `NodeCachePort` | crates/paladin-ports/src/output/node_cache_port.rs:135 | MISSING |
| Port | `RunRepositoryPort` | crates/paladin-ports/src/output/run_repository_port.rs:145 | MISSING |
| Port | `AssistantRepositoryPort` | crates/paladin-ports/src/output/assistant_repository_port.rs:119 | MISSING |
| Port | `RunQueuePort` | crates/paladin-ports/src/output/run_queue_port.rs:115 | MISSING |
| Port | `WebhookDeliveryRepositoryPort` | crates/paladin-ports/src/output/webhook_delivery_port.rs:78 | MISSING |
| Port | `RunScheduleRepositoryPort` | crates/paladin-ports/src/output/run_schedule_repository_port.rs:84 | MISSING |
| Port | `WaypointPort` | crates/paladin-ports/src/output/waypoint_port.rs:167 | MISSING |
| Port | `StructuredExecutorPort` | crates/paladin-ports/src/output/structured_executor_port.rs:82 | MISSING |
| Port | `AssistantAdminPort` | crates/paladin-ports/src/input/assistant_admin_port.rs:124 | MISSING |
| Port | `RunSubmissionPort` | crates/paladin-ports/src/input/run_submission_port.rs:180 | MISSING |
| Port | `ScheduleAdminPort` | crates/paladin-ports/src/input/schedule_admin_port.rs:90 | MISSING |
| Service | `ScheduleService` | src/application/services/run/schedule/service.rs:143 | MISSING |
| Service | `WebhookDeliveryService` | src/application/services/run/webhook/service.rs:103 | MISSING |
| Service | `RunInspectorService` | src/application/services/run/inspector.rs:75 | MISSING |
| Service | `RunEventStreamService` | src/application/services/run/events.rs:764 | MISSING |
| Service | `RunSubmissionService` | src/application/services/run/submission.rs:105 | MISSING |
| Service | `AssistantService` | src/application/services/assistant/service.rs:29 | MISSING |
| Service | `WaypointRetentionService` | src/application/services/waypoint_retention.rs:73 | MISSING |

12 `Port` + 7 `Service` = 19, matching the script's own closing-line count exactly.

**Drift against the frozen Phase 16 enumeration:** `16-DOCS-03-ENTRY-POINTS.md` recorded **76**
entry points (11 Builders + 35 `*Port` traits + 30 `*Service` structs) at Phase 16 close, with
"carry an example block: 76 of 76 (100%)" as its own closing verdict. This run's live
re-derivation finds **101** — a **+25-item drift** (101 − 76 = 25, ≈33% growth) as Phases 22-33
added `pub *Builder`/`*Port`/`*Service` items faster than either the frozen file or this script's
own gate were re-run against them.

**No CI job and no make target runs this script — proven, not assumed:**

```
$ grep -rn check-public-api-examples .github/workflows/*.yml Makefile
(no output; grep's own exit code 1 — zero matches)
```

**Disposition, routed explicitly (D-00c, D-00e):** the script's own scope drift (76→101) and its
19 current MISSING violations are a finding about the D-05/D-06 rule's own apparatus — not about
any single `docs/src` page, any single rustdoc diagnostic, or any single example program — so the
phase's `MB-nn`/`RD-nn`/`EX-nn` ID taxonomy has no slot for it (RESEARCH.md's Open Question 1
reaches the same conclusion and recommends this exact routing). Per D-00e ("the audit reports on
that set only; it does not extend the rule"), none of the 19 MISSING items is fixed here, and the
frozen 76-item entry-point set is not widened to 101 by this audit — doing either would silently
re-litigate a rule scope this phase has no mandate to change. **Plan 34-09 files this finding in
`deferred-items.md`** with the numbers recorded here (101 derived, 76 frozen, +25 drift, 19
MISSING, 0 SINGULAR, no CI/make wiring confirmed empty); no `RD-nn`/`EX-nn`/`MB-nn` row is minted
for any of the 19 individual violations or for the drift itself.

### §3 close — counted totals (plan 34-07, D-01, D-23)

Every figure below is counted from the rows and captures above, not recalled from RESEARCH.md or
CONTEXT.md:

- **Default-feature `cargo doc --workspace --no-deps` content-warning total** (plan 34-06): **65**
  content diagnostics, spanning the default-feature enumeration's own row range, plus the tracer
  row worked separately at the top of this section — 66 rows total from that measurement.
- **Per-crate `-D warnings --all-features` all-features total** (this plan, Task 1): **77** content
  errors across 8 of 12 red crates, spanning this task's own twelve per-crate subheadings' row
  range immediately above.
- **Doctest result** (this plan, Task 2): **462 passed, 0 failed, 210 ignored** — green, 0 rows.
- **Entry-point gate result** (this plan, Task 2): **101** derived entry points, **19 MISSING**,
  **0 SINGULAR**, gate exit `1` — findings routed to the deferred register (D-00e), 0 rows.
- **Total row count in §3: 143** rustdoc rows, spanning the tracer row through this task's own
  last per-crate row with no gap and no duplicate — every ID unique per `34-check.sh` assertion
  (b), every File:line cell ending in a real `.rs:<line>` per the task's own verify.
- **HEAD SHA:** `f53daa8a845c34433fcd30ff0867348105b8330c`, past the Phase 34 start SHA
  `ee1fb160f8e743e638b32beb6c4e32be4ede9325` recorded in the Measurement Header; every intervening
  commit touches only `.planning/` (D-22/D-23 invariance — the source tree these totals were
  measured against is unchanged from the phase's start SHA).

ROADMAP Success Criterion 2 is satisfiable in full from this section: both D-12 bars are quoted
verbatim above, every warning and every error is enumerated with crate, file and line, and the
all-features floor Phase 36 sizes against (77, not 14) is this phase's own measurement rather than
a figure carried from a phase that never re-ran the per-crate sweep.

## §4 Examples table

One row is fully worked here — `examples/README.md` audited as a page in its own right (D-17) —
the second CONTEXT.md-named method self-test. Plan 34-08 fills every remaining row: the four
`ci.yml:548-558` build invocations, the `doc-examples` gate, and `live_vendor_smoke`.

**Live surface re-count (D-16), never trusted from a comment:** `find examples -name '*.rs' |
wc -l` → **48**; `ls crates/doc-examples/src/*.rs` excluding `lib.rs` → **11**
(`agent_runtime.rs`, `bridge.rs`, `content.rs`, `deployment_topologies.rs`,
`fault_tolerance.rs`, `http_service_host.rs`, `orchestration.rs`, `queue_worker.rs`, `readme.rs`,
`sidecar.rs`, `support.rs`); `ls crates/paladin-llm/examples/*.rs` → **1**
(`live_vendor_smoke.rs`). Total live program/module count: **60**.

**Stale CI comment (routed, not an `EX-nn` finding, D-19):** `.github/workflows/ci.yml`'s own
comment at line 538 reads "`examples/` holds 47 .rs files. Exactly 4 are declared `[[example]]`
targets" — the live count above is **48**, one more than the comment states. A CI workflow
comment is neither documentation nor an example (D-19), so this mismatch is not minted as an
`EX-nn` row; it is routed to `deferred-items.md` under a new `## Plan 34-08, Task 1` heading, and
`34-AUDIT.md` §7 (assembled by plan 34-09) will point to it from there.

**The four `ci.yml:548-558` invocations, quoted byte-identical** (`.github/workflows/ci.yml`
lines 548, 551, 554, 557 — the step names are at 547, 550, 553, 556):

```
cargo build --examples --offline
cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline
cargo build --example document_processing --features "content-processing" --offline
cargo build --example http_service_host --features "web-server" --offline
```

**This run's measurement, live, re-run rather than copied from RESEARCH.md or any prior phase**
(HEAD `e0c12333` at capture time — see the plan-34-08 §4 close subsection below for the exact SHA
and the D-23 invariance argument): all four invocations exited **0** (green), each in ~1s
(warm `target/`), teed verbatim to
`34-evidence/34-08-examples-builds.txt`. Two extra targets D-16 also requires are captured in the
same file: `bash scripts/check-doc-examples.sh` (Layer 1 `cargo check --manifest-path
crates/doc-examples/Cargo.toml` — compiles all eleven `doc-examples` modules as one crate;
Layer 1b the README quick-example mirror check; Layer 2 the inline fenced-block scan) — exit
**0**, "All included examples compile.", "README Quick Example is in sync.", "Results: 0
checked, 616 skipped, 0 failed" (every inline block in `docs/src` is either `{{#include}}`-backed,
per plan 34-07's own note that a green doctest baseline already proves `{{#include}}` content
compiles, or explicitly `,ignore`-tagged, so Layer 2 has nothing left to syntax-check directly) —
and `cargo build -p paladin-llm --example live_vendor_smoke --features
"kimi,qwen,grok,gemini"` (`crates/paladin-llm/Cargo.toml`'s `required-features` names all four
vendor flags at once) — exit **0**, built only, **never run** (T-34-02: it reaches a live vendor
and needs a credential; no `PALADIN_*`/vendor API-key environment variable was read or exported
by any command this task ran).

**`[[example]]` declaration cross-check, both directions (root `Cargo.toml` lines 444-462 plus
`crates/paladin-llm/Cargo.toml` lines 60-62):** five `[[example]]` targets are declared workspace-wide
(`vision_analysis`, `vision_battalion`, `document_processing`, `http_service_host` in the root
manifest; `live_vendor_smoke` in `paladin-llm`'s own manifest) — every one of the five has a
matching file on disk (zero declared-with-no-file). In the other direction, the remaining 43 of
48 `examples/*.rs` files carry no `[[example]]` entry at all — this is **not** itself a finding:
cargo auto-discovers any `examples/*.rs` file with no `required-features` as a build target
without a manifest entry, and an explicit `[[example]]` block exists in this workspace only to
attach `required-features` (confirmed: none of the 43 undeclared files needs a non-default
feature, and all 43 build clean under the bare bulk selector, Invocation 1). Zero
file-with-no-declaration findings that indicate an actual gap.

**Build invocation coverage, resolved per program:** the four declared `required-features`
targets (`vision_analysis`, `vision_battalion` → Invocation 2; `document_processing` →
Invocation 3; `http_service_host` → Invocation 4) are skipped by the bulk selector and covered
only by their named invocation; the remaining 44 `examples/*.rs` files (48 − 4) are covered by
Invocation 1 alone. Every row below states its covering invocation in the Build invocation cell;
none is uncovered by every invocation (the finding D-16 anticipates — "a program whose only
coverage is the bulk selector while its `required-features` are unmet is not built at all" —
does not occur in this tree: the manifest's four `required-features` declarations exactly track
the four feature-gated files).

### D-17(a) — obsolete-API sweep (plan 34-08, Task 2)

**Target-set derivation, from §1's removed/renamed rows** (D-17's own named minimum plus every
other §1 removal/rename this checklist carries): `TokenUsage::from_total` (SS-74, deleted
outright), the bare `token_count: u32` field `PaladinResult.usage: TokenUsage` replaced (SS-75),
`Quartermaster` (SS-71, purged — zero in-tree references), the legacy `garrison::TokenCounter`
trait (SS-84, removed outright), `TokenCounterFactory` (SS-85, removed outright), the facade's
local `LimitSource` enum (deleted in Phase 32, `32-04-SUMMARY.md` — not itself an SS row but
named explicitly by D-17), and the pre-Phase-33 `retrieve_context` rendering pattern MIGRATION.md
calls out by name: `memory.content`/`.entry.memory.content` read directly off a RAG-retrieved
result instead of the new `RagRetrievalResult.memories[].body` (SS-86/SS-87; MIGRATION.md's own
words: "a caller that rendered raw `memory.content` before must switch to
`RagRetainedMemory::body`").

**Sweep command, run once across every file this section covers:**
```
grep -rn "TokenUsage::from_total\|\btoken_count\b\|Quartermaster\|\bTokenCounter\b\|TokenCounterFactory\|LimitSource\|memory\.content\|\.entry\.memory\.content" examples/ crates/doc-examples/src/ crates/paladin-llm/examples/
```

**Result: zero true hits, seven coincidental grep matches, each individually resolved:**
- `TokenUsage::from_total`, `Quartermaster`, `TokenCounterFactory`, `LimitSource` — **0 hits**
  each, confirmed absent.
- `token_count` — 15 hits, all in `herald_streaming.rs`/`herald_custom_formatter.rs`, every one
  a call to `StreamChunk::token_count(mut self, token_count: u32) -> Self`
  (`crates/paladin-core/src/platform/container/herald.rs:328`) — a currently-shipped builder
  method on `StreamChunk`, unrelated to the removed `PaladinResult` bare field. Not obsolete.
- `token_count` (as a local variable) — `commander_with_metadata_export.rs:48-61` builds
  `usage: paladin_ports::output::llm_port::TokenUsage::new(token_count, 0)` —
  `TokenUsage::new(prompt_tokens: u32, completion_tokens: u32)` at
  `crates/paladin-core/src/platform/container/token_usage.rs:66` is the exact current two-arg
  constructor (the other four fields default `None`/derived). This is in fact a correct,
  current illustration of the post-Phase-31 `PaladinResult.usage: TokenUsage` shape. Not obsolete.
- `TokenCounter` — 0 hits as a bare identifier (`TokenCounterPort` does not match the
  word-boundary pattern above); confirmed absent.
- `memory.content` / `.entry.memory.content` — 6 hits, all in `paladin_with_sanctum.rs`,
  `sanctum_basic_inmemory.rs` and `examples/README.md`'s own Sanctum code snippet (line 578) —
  every one calls `SanctumPort::search` directly (`sanctum.search(query).await?`), never
  `RagRetrievalService::retrieve_context`. `SanctumSearchResult.entry.memory.content` is a
  distinct, untouched-by-Phase-33 field path — the MIGRATION.md warning is about RAG retrieval
  output specifically, not raw Sanctum search results. Not obsolete.

**Conclusion, stated once and cited by every row below rather than repeated per row:** every one
of the 60 programs and modules this plan swept builds clean, and the target-set sweep found no
genuine obsolete-API reference anywhere under `examples/`, `crates/doc-examples/src/` or
`crates/paladin-llm/examples/` — Rust's own compiler is a second, independent proof for the
"deleted outright, no shim" removals (SS-71/74/84/85, `LimitSource`): a program calling a symbol
that no longer exists cannot compile, and all 60 do.

Currency verdict and Claimed-capability cells below are settled per-row through D-17 check (b) —
capability mapping — with two `stale` findings below (the `examples/http_service_host.rs` row and
its `crates/doc-examples` sibling row, both carrying the same server-parity divergence), detailed
in their own rows. Size is `n/a` for every `current` row (no fix needed) and `M` for the two
`stale` rows.

| EX ID | Program / module | Build invocation | Build status | Currency verdict | Obsolete-API hits | Claimed capability → tree check | Evidence anchor | Size |
|-------|-------------------|-------------------|---------------|-------------------|--------------------|-----------------------------------|------------------|------|
| EX-01 | examples/README.md | n/a — documentation page, not a compiled program | n/a | stale | none (not an API-obsolescence finding) | Line 24 states "Rust 1.70 or later" as the minimum Rust version; `Cargo.toml` `[workspace.package] rust-version = "1.88"` (line 18) is the measured, live MSRV floor — the two disagree | 34-EVIDENCE.md #8 | S |
| EX-02 | examples/agent_handoffs.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Intelligent task delegation to specialist agents. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-03 | examples/arsenal_stdio_tools.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): MCP STDIO tool servers. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-04 | examples/arsenal_streamable_http_tools.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): MCP Streamable-HTTP tool servers (authenticated remote transport). Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-05 | examples/autonomous_full_config.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): All autonomous features working together. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-06 | examples/autonomous_planning.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Autonomous planning with MaxLoops::Auto. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-07 | examples/autonomous_prompt_generation.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Automatic system prompt generation. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-08 | examples/basic_paladin.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Creating and executing a simple Paladin agent. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-09 | examples/battalion_checkpoint_recovery.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Battalion state management. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-10 | examples/campaign_workflow.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Graph-based agent orchestration. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-11 | examples/chain_of_command_delegation.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Hierarchical agent delegation. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-12 | examples/citadel_autosave.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Automatic state persistence. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-13 | examples/citadel_restore.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): State restoration after failure. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-14 | examples/commander_auto.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Automatic strategy selection. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-15 | examples/commander_basic.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Basic Commander usage. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-16 | examples/commander_council.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Commander orchestrating Council discussions across strategies (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-17 | examples/commander_full_config.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Complete Commander configuration. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-18 | examples/commander_grove.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Commander orchestrating Grove routing across all three strategies (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-19 | examples/commander_with_metadata_export.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Battalion execution metadata export. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-20 | examples/conclave_expert_panel.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: The Conclave Mixture-of-Agents pattern (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-21 | examples/council_discussion.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: The Council pattern, multiple expert Paladins in discussion (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-22 | examples/document_processing.rs | `cargo build --example document_processing --features "content-processing" --offline (ci.yml:554)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: PDF text extraction and intelligent document chunking (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 3) | n/a |
| EX-23 | examples/dynamic_temperature.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Dynamic temperature adjustment by task type. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-24 | examples/formation_sequential.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Sequential multi-agent execution. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-25 | examples/garrison_in_memory.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): In-memory conversation history. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-26 | examples/garrison_persistent.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): SQLite-backed persistent memory. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-27 | examples/garrison_semantic_search.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Vector embeddings and semantic search. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-28 | examples/grove_routing.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: The Grove pattern for routing tasks to specialized agent trees (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-29 | examples/herald_custom_formatter.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Custom Herald implementation. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-30 | examples/herald_json_output.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Structured JSON output. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-31 | examples/herald_markdown_output.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Markdown formatting. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-32 | examples/herald_streaming.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Real-time streaming output. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-33 | examples/http_service_host.rs | `cargo build --example http_service_host --features "web-server" --offline (ci.yml:557)` | green (exit 0) | stale | none — builds clean (no removed/renamed API named); this is a scope-claim divergence, not an obsolete-API hit | Own doc comment (examples/http_service_host.rs:6) claims the example "assembles the app exactly as the `paladin-server` binary does (agent router under `/v1`...)". `src/bin/paladin-server.rs:230-233` shows the real binary now merges THREE routers — `agent_router` **+** `thread_router` (Phase 24 HITL, SS-23..SS-25) **+** `run_router` (Phase 27 Platform API, SS-44..SS-52) — while this example mounts only `agent_router` + `docs_router`. "Exactly" no longer holds: what the example demonstrates (the agent API) is itself correct and builds clean, but the server-parity claim is stale by two routers' worth of shipped surface since Phases 24 and 27 | 34-evidence/34-08-examples-builds.txt (Invocation 4) | M |
| EX-34 | examples/llm_provider_selection.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Using different LLM providers. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-35 | examples/maneuver_basic.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Flow DSL orchestration basics. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-36 | examples/maneuver_dynamic_flow.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Dynamic workflow generation. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-37 | examples/maneuver_nested_flow.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Complex nested Flow DSL patterns. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-38 | examples/muster_baseline.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Performance-baseline measurement harness (recorded harness behind docs/src/appendix/performance-baseline.md). Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-39 | examples/paladin_with_config.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Advanced Paladin configuration. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-40 | examples/paladin_with_rag.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: RAG configuration and conceptual workflow — self-labeled "a conceptual demonstration" (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-41 | examples/paladin_with_sanctum.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Integrating Sanctum with Paladin agents. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-42 | examples/phalanx_parallel.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Concurrent multi-agent execution. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-43 | examples/sanctum_adapter_migration.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Migrating memories between adapters. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-44 | examples/sanctum_basic_inmemory.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Basic Sanctum usage with InMemory adapter. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-45 | examples/sanctum_configuration.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Sanctum configuration patterns. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-46 | examples/sanctum_qdrant_production.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (examples/README.md "Demonstrates:"): Production-ready Qdrant adapter with real embeddings. Confirmed current — the program builds clean (no removed/renamed API named, per the sweep above) and its primary imports resolve against the shipped tree at this HEAD | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-47 | examples/vision_analysis.rs | `cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline (ci.yml:551)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Single-image analysis via the Sentinel Vision System (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 2) | n/a |
| EX-48 | examples/vision_battalion.rs | `cargo build --example vision_analysis --example vision_battalion --features "vision,llm-openai" --offline (ci.yml:551)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Multi-agent vision processing via Battalion orchestration (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 2) | n/a |
| EX-49 | examples/war_engine_memory_baseline.rs | `cargo build --examples --offline (ci.yml:548, bulk selector)` | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: War Engine memory-per-superstep baseline harness, ENG-NFR-02 (own doc comment — not in examples/README.md at all, see the README gallery-gap finding below). Confirmed current — builds clean, primary imports resolve against the shipped tree; not cross-checked against examples/README.md because it is absent from that page entirely (see the README gallery-gap finding below) | 34-evidence/34-08-examples-builds.txt (Invocation 1) | n/a |
| EX-50 | crates/doc-examples/src/agent_runtime.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/user-guides/agent-runtime.md's reasoning_agent example. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-51 | crates/doc-examples/src/bridge.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/user-guides/agent-orchestrator-bridge.md's five agent-trigger/orchestration-invoke/policy/recipe anchors. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-52 | crates/doc-examples/src/content.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/user-guides/content-processing.md's seven pipeline anchors (pdf/http/news/aggregate/summarize/llm_bridge/delivery). Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-53 | crates/doc-examples/src/deployment_topologies.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/deployment-topologies/embedded-library.md's embedded_registry anchor. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-54 | crates/doc-examples/src/fault_tolerance.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/user-guides/fault-tolerance.md's Aegis/retry/timeout/cache/compensation anchors. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-55 | crates/doc-examples/src/http_service_host.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | stale | none — builds clean (no removed/renamed API named); this is a scope-claim divergence, not an obsolete-API hit | Own doc comment (crates/doc-examples/src/http_service_host.rs:6, `{{#include}}`-d verbatim into docs/src/deployment-topologies/http-service-host.md) claims "the same router the `paladin-server` binary uses" — identical stale-claim finding as examples/http_service_host.rs (this row's sibling): `paladin-server.rs` now merges `agent_router` + `thread_router` + `run_router`, this module only `agent_router`. The §2 sweep of the including page (34-AUDIT.md §2 row 57) checked the page's own documented `/agents/*` route table against `agent_controller.rs` and found it accurate — that check did not cover this broader "same router" claim, which is a distinct, narrower divergence this §4 sweep catches | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | M |
| EX-56 | crates/doc-examples/src/orchestration.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/user-guides/orchestration.md's formation/phalanx/campaign/chain_of_command/commander/scheduling/events anchors, and docs/src/deployment-topologies/battalion-orchestration.md's phalanx anchor. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-57 | crates/doc-examples/src/queue_worker.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/deployment-topologies/queue-worker.md's queue anchor (RedisQueueAdapter). Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-58 | crates/doc-examples/src/readme.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/deployment-topologies/embedded-library.md's quickstart anchor and README.md's own Quick Example (Layer 1b sync-checked). Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-59 | crates/doc-examples/src/sidecar.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Backs docs/src/deployment-topologies/sidecar.md's sidecar_client anchor. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-60 | crates/doc-examples/src/support.rs | `bash scripts/check-doc-examples.sh` (Layer 1: `cargo check --manifest-path crates/doc-examples/Cargo.toml`) | green (exit 0) | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed: Shared mock adapters/constructors for the other ten doc-examples modules — not itself {{#include}}-targeted by any docs/src page. Confirmed current — `scripts/check-doc-examples.sh` Layer 1 compiles this module as part of the `paladin-doc-examples` crate (exit 0), which is the project's own stated guarantee that an `{{#include}}`-d anchor cannot drift from the shipped API | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) | n/a |
| EX-61 | crates/paladin-llm/examples/live_vendor_smoke.rs | `cargo build -p paladin-llm --example live_vendor_smoke --features "kimi,qwen,grok,gemini"` (D-16; built only, never run — reaches a live vendor and needs a credential) | green (exit 0), built not run | current | none — confirmed by the global obsolete-API sweep above (build green; target-set tokens grepped, zero true hits) | Claimed (own doc comment): proves against REAL vendor endpoints that base_url/get_available_models()/generate() work for Kimi/Qwen/Grok/Gemini (Phase 17 UAT test 4). Confirmed current — builds clean under all four required-features; not run (T-34-02), so live-vendor behavior itself is unverified by this audit, only compilation against the shipped adapter API | 34-evidence/34-08-examples-builds.txt (Extra target 2) | n/a |

### D-17(c) — capability gap list (Phase 22-33 shipped items no example demonstrates)

Walked every §1 row carrying a requirement ID (91 total; the three rows with Req ID `—` —
SS-08/SS-09/SS-10, MSRV/resolver/fingerprint-version housekeeping, not requirement-attributed
capabilities — are out of scope for this walk by construction) against a fixed-string grep of
its own §1 Grep-token column across `examples/` and `crates/doc-examples/src/` (the same tree
the D-17(a) sweep above covers). Excluded from the candidate set below, with reason: removed
items are absences by design, not undemonstrated capabilities (`SS-74` `TokenUsage::from_total`,
`SS-84` `TokenCounter`, `SS-85` `TokenCounterFactory`, `SS-71` `Quartermaster`); pure CI/tooling
gates and release artifacts are not example-representable (`SS-63` `cargo doc` bar, `SS-64`
`openapi_golden_v0_9`, `SS-65` coverage floor, `SS-66` `openapi-generator-cli`); governance/
vocabulary records are not code capabilities (`SS-67` Medieval Military rule, `SS-69` `Treasurer`
— reserved, not yet implemented — `SS-72` `ADR-0051`); `SS-61` records an accepted deviation,
not a capability; `SS-90` states a `Cargo.toml` dependency fact, not a capability; `SS-73`,
`SS-75`, `SS-76`, `SS-79`, `SS-70` already have nonzero hits (demonstrated, not gapped — see the
§4 table rows citing them). Every remaining requirement-attributed row with **zero** grep hits
becomes its own row below — **59** of them, confirming RESEARCH.md's own prediction that this
list "is expected to be long."

| EX ID | Capability | Req ID | Phase | Grep evidence (0 hits) | Size |
|-------|-----------|--------|-------|-------------------------|------|
| EX-62 | Injecting a custom WaypointPort backend (InMemory/SQLite/Postgres) for checkpoint snapshots (SS-03) | ENG-05 | 22 | `grep -rlF 'WaypointPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-63 | Reading/inspecting the waypoints persistence table directly (SS-04) | ENG-03 | 22 | `grep -rlF 'waypoints' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-64 | Configuring WarEngine via EngineConfig (max_supersteps, max_node_visits, run_timeout_secs, waypoint_durability, max_muster_tasks) (SS-05) | ENG-02 | 22 | `grep -rlF 'EngineConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-65 | Overriding the superstep cap via APP_ENGINE_MAX_SUPERSTEPS (SS-06) | ENG-02 | 22 | `grep -rlF 'APP_ENGINE_MAX_SUPERSTEPS' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-66 | Pruning old Waypoints via WaypointRetentionService/WaypointRetentionConfig (SS-07) | ENG-05 | 22 | `grep -rlF 'WaypointRetentionService' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-67 | EdgeCondition::Custom's fail-closed behavior when unregistered (SS-11) | CF-01 | 23 | `grep -rlF 'EdgeCondition::Custom' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-68 | Nested subgraph composition via NodeSpec::Battalion (SS-14) | CF-04 | 23 | `grep -rlF 'NodeSpec::Battalion' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-69 | LLM-driven dynamic routing via LlmDecisionEvaluator / Commander StrategySelection::Semantic (SS-15) | CF-05 | 23 | `grep -rlF 'LlmDecisionEvaluator' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-70 | Overriding the Muster fan-out cap via APP_ENGINE_MAX_MUSTER_TASKS (SS-16) | CF-03 | 23 | `grep -rlF 'APP_ENGINE_MAX_MUSTER_TASKS' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-71 | First-class approval-gate nodes via NodeSpec::Gate (SS-17) | HITL-01 | 24 | `grep -rlF 'NodeSpec::Gate' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-72 | Typed, total-validation resume via WarEngine::resume_with(graph, thread, responses) (SS-18) | HITL-02 | 24 | `grep -rlF 'resume_with' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-73 | History/replay/fork via ChronicleService + WarEngine::replay/fork (SS-19) | HITL-03 | 24 | `grep -rlF 'ChronicleService' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-74 | Graceful shutdown on SIGTERM/SIGINT via ShutdownCoordinator (SS-20) | HITL-04 | 24 | `grep -rlF 'ShutdownCoordinator' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-75 | Configuring shutdown grace period via APP_ENGINE_SHUTDOWN_GRACE_SECS (SS-21) | HITL-04 | 24 | `grep -rlF 'APP_ENGINE_SHUTDOWN_GRACE_SECS' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-76 | Toggling graceful shutdown via APP_ENGINE_GRACEFUL_SHUTDOWN (SS-22) | HITL-04 | 24 | `grep -rlF 'APP_ENGINE_GRACEFUL_SHUTDOWN' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-77 | Reading paused-thread state via GET /v1/threads/{id}/state (SS-23) | HITL-05 | 24 | `grep -rlF 'GET /v1/threads/{id}/state' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-78 | Resuming a paused thread via POST /v1/threads/{id}/resume (SS-24) | HITL-05 | 24 | `grep -rlF 'POST /v1/threads/{id}/resume' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-79 | Paginated history retrieval via GET /v1/threads/{id}/history (SS-25) | HITL-05 | 24 | `grep -rlF 'GET /v1/threads/{id}/history' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-80 | The v3→v4 graph-fingerprint bump for Gate node routing properties (SS-26) | HITL-01 | 24 | `grep -rlF 'GRAPH_FINGERPRINT_VERSION' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-81 | Enabling node-result caching via the redis-cache Cargo feature (SS-33) | FT-06 | 25 | `grep -rlF 'redis-cache' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-82 | Toggling node-result caching via APP_NODE_CACHE_ENABLED (SS-34) | FT-06 | 25 | `grep -rlF 'APP_NODE_CACHE_ENABLED' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-83 | Writing a custom ExecutionMiddleware (before_model/after_model/around_tool) (SS-35) | RT-01 | 26 | `grep -rlF 'ExecutionMiddleware' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-84 | Configuring the twelve built-in middleware sub-structs via AgentRuntimeConfig (SS-36) | RT-02 | 26 | `grep -rlF 'AgentRuntimeConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-85 | Implementing a custom TokenCounterPort (SS-37) | RT-03 | 26 | `grep -rlF 'TokenCounterPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-86 | Context-window management via HistoryTrimmer + SummarizationMiddleware (SS-38) | RT-03 | 26 | `grep -rlF 'HistoryTrimmer' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-87 | Structural memory namespacing via VaultPort / ConfinedVault (SS-39) | RT-04 | 26 | `grep -rlF 'VaultPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-88 | Schema-validated structured output via StructuredExecutorPort / execute_structured<T> (SS-40) | RT-05 | 26 | `grep -rlF 'StructuredExecutorPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-89 | Opting into tool_error_mode = FailRun with redact-then-bound tool-text sanitization (SS-42) | RT-07 | 26 | `grep -rlF 'tool_error_mode' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-90 | Deriving a JSON schema for structured output via schemars (SS-43) | RT-05 | 26 | `grep -rlF 'schemars' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-91 | Decoupled run submission via POST /v1/runs (SS-44) | PLAT-01 | 27 | `grep -rlF 'POST /v1/runs' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-92 | SSE run streaming via GET /v1/runs/{run_id}/stream (seven frozen wire events) (SS-45) | PLAT-03 | 27 | `grep -rlF 'GET /v1/runs/{run_id}/stream' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-93 | Cancelling an in-flight run via POST /v1/runs/{run_id}/cancel (SS-46) | PLAT-02 | 27 | `grep -rlF 'POST /v1/runs/{run_id}/cancel' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-94 | Managing immutable assistant versions via /v1/assistants* (SS-47) | PLAT-04 | 27 | `grep -rlF '/v1/assistants' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-95 | Cron-driven recurring run submission via /v1/schedules* (SS-48) | PLAT-05 | 27 | `grep -rlF '/v1/schedules' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-96 | Webhook delivery with X-Paladin-Signature HMAC verification (SS-49) | PLAT-05 | 27 | `grep -rlF 'webhook_deliveries' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-97 | The SSRF-guard override APP_WEBHOOKS_ALLOW_PRIVATE (SS-50) | PLAT-05 | 27 | `grep -rlF 'APP_WEBHOOKS_ALLOW_PRIVATE' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-98 | Durable worker-pool dispatch via RunQueuePort (InMemory + Redis) (SS-51) | PLAT-02 | 27 | `grep -rlF 'RunQueuePort' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-99 | Selecting a durable run-store backend via APP_RUN_STORE_BACKEND (SS-52) | PLAT-01 | 27 | `grep -rlF 'APP_RUN_STORE_BACKEND' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-100 | Consuming the TraceRecord envelope / twelve TraceEvent variants (SS-53) | OBS-01 | 28 | `grep -rlF 'TraceRecord' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-101 | Configuring tracing via TraceConfig (log_sink/persist/state_values/otel) (SS-54) | OBS-02 | 28 | `grep -rlF 'TraceConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-102 | Enabling OTel export via PALADIN_TRACE_OTEL_ENABLED (SS-55) | OBS-02 | 28 | `grep -rlF 'PALADIN_TRACE_OTEL_ENABLED' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-103 | Wiring the otel Cargo feature (opentelemetry/opentelemetry_sdk/opentelemetry-otlp) (SS-56) | OBS-02 | 28 | `grep -rlF 'otel' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-104 | The admin-gated, dev-ui-feature-gated GET /v1/dev-ui/threads/{id} route (SS-57) | OBS-03 | 28 | `grep -rlF '/v1/dev-ui/threads' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-105 | Writing eval scenarios via the paladin-eval crate + eval_scenarios! macro (SS-58) | OBS-04 | 28 | `grep -rlF 'paladin-eval' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-106 | Enabling live-mode eval runs via PALADIN_EVAL_LIVE (SS-59) | OBS-04 | 28 | `grep -rlF 'PALADIN_EVAL_LIVE' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-107 | Running eval scenarios via paladin-cli eval run <glob> (SS-60) | OBS-04 | 28 | `grep -rlF 'eval run' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-108 | Querying the append-only run_traces persisted-trace-history table (SS-62) | OBS-02 | 28 | `grep -rlF 'run_traces' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-109 | Constructing/using Commissary directly (input-side, per-call window-rationing) (SS-68) | VOCAB-02 | 30 | `grep -rlF 'Commissary' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-110 | The HTTP-surface TokenUsageResponse DTO on ExecuteResponse.usage (SS-77) | ACCT-02 | 31 | `grep -rlF 'TokenUsageResponse' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-111 | Anthropic's fixed prompt_tokens figure now including cache-read/cache-write tokens (SS-78) | ACCT-03 | 31 | `grep -rlF 'prompt_tokens' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-112 | TokenCounterPort::is_exact(&self) -> bool's defaulted behavior (SS-80) | PRIM-01 | 32 | `grep -rlF 'is_exact' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-113 | Commissary::new/from_port's is_exact_counter-argument removal (exactness read live from is_exact) (SS-81) | PRIM-02 | 32 | `grep -rlF 'Commissary::new' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-114 | The shared precedence resolver paladin_llm::window::resolve_context_window (SS-82) | PRIM-04 | 32 | `grep -rlF 'resolve_context_window' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-115 | WindowSource/WindowFallbackPolicy/ResolvedWindow re-exported from the paladin facade (SS-83) | PRIM-04 | 32 | `grep -rlF 'WindowSource' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-116 | Reading RagRetrievalService::retrieve_context's new RagRetrievalResult return type (SS-86) | COMM-01 | 33 | `grep -rlF 'RagRetrievalResult' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-117 | Reading the truncation/shed record via RagRetrievalResult.shed: Vec<ShedItem> (SS-87) | COMM-02 | 33 | `grep -rlF 'ShedItem' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-118 | Handling the typed RagRetrievalError enum (Sanctum/Commissary/budget-conversion failures) (SS-88) | COMM-01 | 33 | `grep -rlF 'RagRetrievalError' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-119 | The free function retrieve_context_with_timeout returning RagRetrievalResult (SS-89) | COMM-01 | 33 | `grep -rlF 'retrieve_context_with_timeout' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |
| EX-120 | Injecting an exact token counter via RagRetrievalService::with_token_counter (SS-91) | COMM-04 | 33 | `grep -rlF 'with_token_counter' examples/ crates/doc-examples/src/` → (no output, 0 hits) | L |

### `examples/README.md` audit (D-17, a page in its own right)

`34-signals.sh examples/README.md` was not run — that script is scoped to `docs/src/*.md` pages
(§2's mdBook sweep); `examples/README.md` lives outside `docs/src` and is audited here under D-17's
own instruction instead, using the same signal-class discipline (content match, never file
existence/mtime).

**MSRV claim (already recorded in this section's first worked row, cited not re-minted, per this**
**task's own instruction):** line 24 states "Rust 1.70 or later" against the measured
`rust-version = "1.88"` — see the `examples/README.md` row at the top of the main table above;
not repeated here as a second row for the same finding.

**Listed-versus-on-disk cross-check, both directions:**
- *On disk, not listed:* `grep -oE '^### \[[a-zA-Z0-9_]+\.rs\]' examples/README.md` extracts 37
  `.rs`-file section headers. Diffing the full 48-file `find examples -name '*.rs'` list against
  those 37 names (`comm -23`) finds **11 programs with zero mentions anywhere in the file**
  (`grep -c <name> examples/README.md` is 0 for every one, confirmed individually, not just
  absent from a section header): `commander_council.rs`, `commander_grove.rs`,
  `conclave_expert_panel.rs`, `council_discussion.rs`, `document_processing.rs`,
  `grove_routing.rs`, `http_service_host.rs`, `paladin_with_rag.rs`, `vision_analysis.rs`,
  `vision_battalion.rs`, `war_engine_memory_baseline.rs`. Five of the eleven are the four
  feature-gated `[[example]]` targets plus `paladin_with_rag.rs` — not a coincidence of newness
  alone (`war_engine_memory_baseline.rs`, `council_discussion.rs`, `conclave_expert_panel.rs`,
  `commander_council.rs`, `commander_grove.rs`, `grove_routing.rs` carry no feature gate at all
  and are still absent). Every §4 row for these 11 files above notes it is "not cross-checked
  against examples/README.md because it is absent from that page entirely."
- *Listed, not on disk:* all 37 listed `.rs` names have a matching file — **zero** dangling
  listings.

**Finding — 11 on-disk programs entirely absent from the gallery (D-17), a new row below.**
Currency verdict: **stale**. The README's own Table of Contents has no section for Vision,
Document Processing, HTTP Service Host, RAG, or the Council/Grove/Conclave Commander strategies —
a reader browsing the gallery cannot discover these 11 programs exist. Size: **L** (eleven new
subsections, more than a single-section `M` addition, per D-04's "new page... or a rewrite of
more than half" — eleven of forty-eight programs is not "more than half," but eleven full new
subsections each carrying a Demonstrates line, a run command, and key concepts is qualitatively
more than one `M` section rewrite; sized `L` on that basis). Evidence anchor: 34-EVIDENCE.md #159.

**Finding — the "Code snippet" blocks show a `PaladinResult` shape that no longer exists (D-17),**
**a second new row below.** Currency verdict: **stale**. Three illustrative snippets under
`## Basic Paladin Examples` (line ~71) and `## Advanced Examples` → "Logging and Observability"
(lines 1328-1329) read `response.content`, `response.token_usage.total_tokens`, and
`response.execution_time` — `PaladinResult`'s real fields, confirmed live at
`crates/paladin-core/src/platform/container/execution_result.rs:50-65`, are `output`,
`usage: TokenUsage` and `execution_time_ms: u64`; there is no `content` field, no `token_usage`
field (the field is `usage`), and no bare `execution_time` field (it is
`execution_time_ms`, in milliseconds). The real `basic_paladin.rs` build row above already
uses the correct current shape (`result.output`, `result.usage.total_tokens`,
`result.execution_time_ms`) — the README's own simplified "Code snippet" for the same program
has drifted from the file it is meant to preview. Obsolete-API hits: `response.content` /
`response.token_usage` / `response.execution_time` at README.md lines 71, 1328, 1329 — three
field names that never existed on this shape in the current tree (a renamed-field case, not
merely a different field name choice: `output`/`usage`/`execution_time_ms` are the only fields).
Size: **S** (three snippet lines to correct). Evidence anchor: 34-EVIDENCE.md #160.

| EX ID | Program / module | Build invocation | Build status | Currency verdict | Obsolete-API hits | Claimed capability → tree check | Evidence anchor | Size |
|-------|-------------------|-------------------|---------------|-------------------|--------------------|-----------------------------------|------------------|------|
| EX-121 | examples/README.md | n/a — documentation page, not a compiled program | n/a | stale | none (not an API-obsolescence finding) | Table of Contents / section structure claims to gallery every example; 11 of 48 on-disk programs (`commander_council.rs`, `commander_grove.rs`, `conclave_expert_panel.rs`, `council_discussion.rs`, `document_processing.rs`, `grove_routing.rs`, `http_service_host.rs`, `paladin_with_rag.rs`, `vision_analysis.rs`, `vision_battalion.rs`, `war_engine_memory_baseline.rs`) have zero mentions anywhere in the file | 34-EVIDENCE.md #159 | L |
| EX-122 | examples/README.md | n/a — documentation page, not a compiled program | n/a | stale | `response.content` (line 71), `response.token_usage.total_tokens` (line 1328), `response.execution_time` (line 1329) — none of these three field names exist on `PaladinResult` (real fields: `output`, `usage: TokenUsage`, `execution_time_ms`) | The "Code snippet" for `basic_paladin.rs` (and the "Logging and Observability" advanced snippet) claim a `PaladinResult` shape the real `basic_paladin.rs` file does not use — the real file correctly reads `result.output`/`result.usage.total_tokens`/`result.execution_time_ms` | 34-EVIDENCE.md #160 | S |

### D-18 — `doc-examples` module → page include map

Derived mechanically from `grep -roE '\{\{#include \.\./\.\./\.\./crates/doc-examples/src/[a-z_]+\.rs:[a-z_]+\}\}' docs/src/ -r`
(never from a module's own doc comment claim) — one row per module, every including page and
anchor name listed:

| Module (`EX-nn`) | Anchors | Including page(s) |
|---|---|---|
| `agent_runtime.rs` | `reasoning_agent` | `docs/src/user-guides/agent-runtime.md` |
| `bridge.rs` | `agent_triggers`, `orchestration_invokes`, `bridge_policy`, `recipe_news`, `recipe_schedule`, `recipe_trigger` | `docs/src/user-guides/agent-orchestrator-bridge.md` |
| `content.rs` | `pdf`, `http`, `news`, `aggregate`, `summarize`, `llm_bridge`, `delivery` | `docs/src/user-guides/content-processing.md` |
| `deployment_topologies.rs` | `embedded_registry` | `docs/src/deployment-topologies/embedded-library.md` |
| `fault_tolerance.rs` | `attach`, `cache`, `compensation`, `custom_handler`, `custom_predicate`, `fallback`, `handlers`, `heartbeat`, `retry`, `timeout`, `transience` | `docs/src/user-guides/fault-tolerance.md` |
| `http_service_host.rs` | `http_host` | `docs/src/deployment-topologies/http-service-host.md` |
| `orchestration.rs` | `formation`, `phalanx`, `campaign`, `chain_of_command`, `commander`, `scheduling`, `events` | `docs/src/user-guides/orchestration.md` (all seven anchors) **and** `docs/src/deployment-topologies/battalion-orchestration.md` (`phalanx` only — two pages share one module) |
| `queue_worker.rs` | `queue` | `docs/src/deployment-topologies/queue-worker.md` |
| `readme.rs` | `quickstart` | `docs/src/deployment-topologies/embedded-library.md` (mdBook) **and** the root `README.md` (Layer 1b sync check, not an mdBook include) |
| `sidecar.rs` | `sidecar_client` | `docs/src/deployment-topologies/sidecar.md` |
| `support.rs` | none | **none — no page includes it (D-18 finding).** It is a shared mock-adapter/constructor module the other ten modules import as an ordinary Rust dependency (confirmed: `grep -rl 'mod support\|support::' crates/doc-examples/src/*.rs` hits the other ten files), not an `{{#include}}` target itself. Recorded as required rather than omitted — a future page correction to any of the other ten modules may need to touch this shared file, and this row is how Phase 35/36 would know to check it. |

This map lets a Phase 35 page edit find which module(s) it must re-verify (e.g. touching
`orchestration.md` means re-checking `orchestration.rs`'s five page-specific anchors, but not
`phalanx`, which is shared with a second page) and lets Phase 36 find which page(s) a module fix
re-renders (e.g. a `fault_tolerance.rs` signature fix re-renders eleven anchors on one page).

### §4 close — counted totals (plan 34-08)

Every figure below is counted from the rows and captures on disk at this plan's own HEAD, not
recalled from RESEARCH.md, CONTEXT.md or an earlier phase's total.

- **Program/module count:** 60 live items swept (48 `examples/*.rs` + 11 `crates/doc-examples/src/*.rs`
  excl. `lib.rs` + 1 `crates/paladin-llm/examples/live_vendor_smoke.rs`), each carrying exactly one
  row in the main build table above.
- **Build-status distribution:** 60/60 green (100%) across the six captures in
  `34-evidence/34-08-examples-builds.txt` — zero red, zero uncovered-by-any-invocation.
- **Currency-verdict distribution (main 9-column table — the one worked row plan 34-01 wrote, the**
  **60 build rows this plan's Task 1 added, and the two README-page rows this section's own audit**
  **added, 63 rows total):** 58 `current`, 5 `stale` (the pre-existing MSRV row; the
  `examples/http_service_host.rs` row and its `crates/doc-examples` sibling row, both for the same
  server-parity claim; the 11-missing-programs gallery-gap row; the stale-`PaladinResult`-fields
  code-snippet row). Zero `missing` (no example references a page that does not exist — not an
  applicable category for this table).
- **Gap-list length:** 59 undemonstrated Phase 22-33 capabilities (the separate gap-list table
  above), spanning eight of the twelve phases this audit's §1 checklist covers (22, 23, 24, 25, 26,
  27, 28, 30, 31, 32, 33 all contribute at least one row; Phase 29's four SS rows are all excluded
  as CI/release tooling, not example-representable capabilities — the only phase with zero
  gap-list rows).
- **Total `EX-nn` count this plan minted:** 121 new rows (the 60 build rows, the 59 gap-list rows,
  and the 2 README-page rows) plus the one row plan 34-01 already worked — **122** total `EX-nn`
  IDs in `34-AUDIT.md` at this plan's close.
- **HEAD SHA this plan measured:** `8678b5cec926f56753698236be4a2921008bd172` (this plan's own
  Task 1 commit). The Phase 34 start SHA `ee1fb160f8e743e638b32beb6c4e32be4ede9325` recorded in
  the Measurement Header remains the D-23 invariance reference — every intervening commit touches
  only `.planning/` (confirmed again by this plan's own `git status --porcelain -- . ':!.planning'`
  and `git status --porcelain -- examples crates Cargo.toml` checks before every commit), so the
  source tree both SHAs measure is identical.

ROADMAP Success Criterion 3 is satisfiable from this section in full: every program and module has
a build status under the exact feature sets CI splits on, a three-check currency verdict, and the
capabilities Phases 22-33 shipped with no demonstrating example are enumerated by name rather than
left for Phase 36 to rediscover.

## §5 Phase 35 work list

Sufficient for `/gsd-plan-phase 35` to plan every mdBook (`MB-nn`) finding in §2 by ID. Phase 35 may re-batch these rows into its own plans/waves, but must close every ID listed below — none may be dropped, merged away, or silently absorbed into a different ID's fix. Measured HEAD SHA: `ee1fb160f8e743e638b32beb6c4e32be4ede9325` (D-23). If the branch moves before Phase 35 plans, re-run the D-12/D-16 commands this audit ran (34-EVIDENCE.md) and diff against the rows below rather than trusting the counts unchanged.

Ordered per D-21: MB-30 (the missing superstep-engine page, the one `L` item every other row in this list depends on being written first, since it is the page `control-flow.md` (MB-22) already points forward to) leads; the remainder follow in `docs/src/SUMMARY.md` nav order, each MB ID keeping the size, location and citation the originating §2 row already carries.

| Order | ID | Classification | Size | Location | Cites (Phase N — item (REQ)) | Blocks | Evidence anchor |
|---|---|---|---|---|---|---|---|
| 1 | MB-30 | missing page | L | docs/src/user-guides/the-superstep-engine.md *(proposed — does not exist on disk)* | Phase 22 — `WarEngine` executes cyclic graphs in supersteps, self-loops included (ENG-02, SS-01) | MB-22 | 34-AUDIT.md §2 row 94 |
| 2 | MB-04 | stale content | M | docs/src/introduction.md | Phase 30 — `Commissary` anchored (VOCAB-02, SS-68) |  | 34-AUDIT.md §2 row 68 |
| 3 | MB-05 | stale content | L | docs/src/introduction.md | Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) |  | 34-AUDIT.md §2 row 68 |
| 4 | MB-06 | stale content | L | docs/src/getting-started/installation.md | Phase 22.1 — Workspace MSRV floor raised from 1.85 to 1.88 (X-11.2, SS-08) |  | 34-AUDIT.md §2 row 66 |
| 5 | MB-07 | stale content | S | docs/src/getting-started/quickstart.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` (`Cargo.toml [workspace.package]`) |  | 34-AUDIT.md §2 row 67 |
| 6 | MB-27 | stale content | M | docs/src/user-guides/paladin-agents.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` |  | 34-AUDIT.md §2 row 89 |
| 7 | MB-20 | stale content | M | docs/src/user-guides/battalion-patterns.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact |  | 34-AUDIT.md §2 row 77 |
| 8 | MB-26 | stale content | S | docs/src/user-guides/orchestration.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact |  | 34-AUDIT.md §2 row 87 |
| 9 | MB-21 | stale content | S | docs/src/user-guides/content-processing.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact |  | 34-AUDIT.md §2 row 78 |
| 10 | MB-18 | stale content | S | docs/src/user-guides/agent-orchestrator-bridge.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` (`Cargo.toml [workspace.package]`) |  | 34-AUDIT.md §2 row 74 |
| 11 | MB-19 | stale content | M | docs/src/user-guides/arsenal-tools.md | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document |  | 34-AUDIT.md §2 row 76 |
| 12 | MB-28 | stale content | L | docs/src/user-guides/sanctum-vector-memory.md | Phase 33 — `RagRetrievalService::retrieve_context` returns `RagRetrievalResult` (COMM-01, SS-86) |  | 34-AUDIT.md §2 row 92 |
| 13 | MB-24 | stale content | S | docs/src/user-guides/herald-output.md | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document |  | 34-AUDIT.md §2 row 84 |
| 14 | MB-25 | stale content | S | docs/src/user-guides/maneuver-flow-dsl.md | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0`, no matching §1 row for a bare version-pin fact |  | 34-AUDIT.md §2 row 85 |
| 15 | MB-22 | stale content | M | docs/src/user-guides/control-flow.md | Phase 24 — `NodeSpec::Gate` first-class approval-gate node + `WarEngine::resume_with` (HITL-01/HITL-02, SS-17/SS-18) |  | 34-AUDIT.md §2 row 79 |
| 16 | MB-23 | stale content | S | docs/src/user-guides/fault-tolerance.md | Phase 26 — `output_schema` on `NodeSpec::Paladin` bumps `GRAPH_FINGERPRINT_VERSION` to `v6` (D-29, RT-05, SS-40) |  | 34-AUDIT.md §2 row 81 |
| 17 | MB-29 | stale content | S | docs/src/user-guides/tool-integration.md | Phase 26 — `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` prompt-level tool-call protocol via the `reasoning_agent` preset (RT-07, `tool_error_mode`, SS-41/SS-42) |  | 34-AUDIT.md §2 row 93 |
| 18 | MB-08 | stale content | M | docs/src/architecture/overview.md | Phase 28 — `paladin-eval` crate (OBS-04, SS-58) |  | 34-AUDIT.md §2 row 49 |
| 19 | MB-09 | stale content | L | docs/src/architecture/overview.md | Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) |  | 34-AUDIT.md §2 row 49 |
| 20 | MB-10 | stale content | M | docs/src/architecture/hexagonal-design.md | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document |  | 34-AUDIT.md §2 row 48 |
| 21 | MB-03 | stale content | M | docs/src/architecture/domain-model.md:96-104 | Phase 26 — `GarrisonEntry.is_summary` effective-history marker (RT-03, SS-38) |  | 34-AUDIT.md §2 Phase 31 D-29 token_count subsection |
| 22 | MB-11 | stale content | L | docs/src/architecture/domain-model.md | Phase 22 — `Waypoint`, a full `Battlefield` snapshot persisted automatically after every superstep (ENG-03, SS-02) |  | 34-AUDIT.md §2 row 47 |
| 23 | MB-02 | stale content | S | docs/src/architecture/commissary.md:7 | Phase 30 — `Quartermaster` purged, zero in-tree references required (VOCAB-06, SS-71) |  | 34-AUDIT.md §2 Vocabulary sweep subsection |
| 24 | MB-12 | stale content | S | docs/src/architecture/design-patterns.md | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document |  | 34-AUDIT.md §2 row 46 |
| 25 | MB-13 | stale content | L | docs/src/architecture/crate-map.md | Phase 33 — `paladin-memory` gains an unconditional dependency on `paladin-llm` (COMM-01, SS-90) |  | 34-AUDIT.md §2 row 45 |
| 26 | MB-31 | stale content | L | docs/src/deployment/cicd.md | Phase 18 — `codeql.yml` Rust SAST, evaluated and retained advisory-only (2026-08-25, pre-milestone but post-dates the page's own last correction) |  | 34-AUDIT.md §2 row 61 |
| 27 | MB-32 | stale content | M | docs/src/operations/monitoring.md | Phase 28 — `OtelTraceSink`/`otel` Cargo feature, real OTLP/HTTP trace export (OBS-02, SS-56) |  | 34-AUDIT.md §2 row 70 |
| 28 | MB-33 | stale content | S | docs/src/operations/performance-tuning.md | Phase 22 — the superstep engine gains its own benchmark file (`benches/engine_benchmarks.rs`, ENG-02) |  | 34-AUDIT.md §2 row 72 |
| 29 | MB-34 | stale content | S | docs/src/operations/troubleshooting.md | Phase 28 — `opentelemetry`/`opentelemetry_sdk`/`opentelemetry-otlp` become real, optional workspace dependencies behind the `otel` feature (OBS-02, SS-56) |  | 34-AUDIT.md §2 row 73 |
| 30 | MB-14 | stale content | L | docs/src/api-reference/crate-map.md | Phase 33 — `paladin-memory` gains an unconditional dependency on `paladin-llm` (COMM-01, SS-90) |  | 34-AUDIT.md §2 row 2 |
| 31 | MB-15 | stale content | L | docs/src/api-reference/feature-flags.md | Phase 25 — `redis-cache` Cargo feature on `paladin-storage` (FT-06, SS-33) |  | 34-AUDIT.md §2 row 3 |
| 32 | MB-16 | stale content | S | docs/src/api-reference/migration-guide.md | Phase 29 — release versioning is v0.10.0-scoped (SHIP-01) |  | 34-AUDIT.md §2 row 4 |
| 33 | MB-17 | stale content | L | docs/src/api-reference/stable-api.md | Phase 28 — `paladin-eval` crate + `eval_scenarios!` harness (OBS-04, SS-58) |  | 34-AUDIT.md §2 row 6 |
| 34 | MB-36 | stale content | L | docs/src/contributing/testing-guide.md | Phase 29 — 82% workspace line coverage floor, `cargo llvm-cov --fail-under-lines` (SHIP-04, SS-65) |  | 34-AUDIT.md §2 row 54 |
| 35 | MB-35 | stale content | L | docs/src/contributing/architecture-decisions.md | pre-existing content/nav-title mismatch (D-00g: shipped tree — here, the nav's own title — outranks the document |  | 34-AUDIT.md §2 row 50 |
| 36 | MB-48 | stale content | L | docs/src/appendix/council.md | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone API drift, D-00g) |  | 34-AUDIT.md §2 row 23 |
| 37 | MB-58 | stale content | M | docs/src/appendix/sentinel.md | Phase 17 through Phase 26 -- no single Phase 22-33 REQ-ID applies (vision predates the milestone, D-00g) |  | 34-AUDIT.md §2 row 41 |
| 38 | MB-38 | stale content | M | docs/src/appendix/battalion-patterns-guide.md | Phase 30 -- vocabulary/facade precision is a program-wide concern (VOCAB-01) |  | 34-AUDIT.md §2 row 10 |
| 39 | MB-46 | stale content | M | docs/src/appendix/cli-usage.md | Phase 26 -- no Phase 22-33 REQ-ID applies |  | 34-AUDIT.md §2 row 20 |
| 40 | MB-60 | stale content | L | docs/src/appendix/user-system.md | Phase 22 through Phase 33 -- no REQ-ID applies (pre-milestone artifact, D-00g) |  | 34-AUDIT.md §2 row 43 |
| 41 | MB-59 | stale content | L | docs/src/appendix/user-rest-api.md | Phase 22 through Phase 33 -- no REQ-ID applies (pre-milestone artifact, D-00g) |  | 34-AUDIT.md §2 row 42 |
| 42 | MB-52 | stale content | L | docs/src/appendix/provider-expansion.md | Phase 17 -- Kimi/Qwen/Grok/Ollama/Gemini/generic-OpenAI-compatible adapters shipped (PROV-01..04, predates Phase 22 but is the concrete contradiction) |  | 34-AUDIT.md §2 row 32 |
| 43 | MB-49 | stale content | L | docs/src/appendix/integration-tests.md | Phase 22 through Phase 33 -- essentially every phase's own integration-test additions are missing from the inventory (representative REQ-IDs: PLAT-01 SS-44, COMM-01 SS-86, RT-04 SS-39, OBS-02 SS-54) |  | 34-AUDIT.md §2 row 28 |
| 44 | MB-57 | stale content | L | docs/src/appendix/security-scanning.md | v0.9.0 (Phases 18-21) -- the Rust-SAST evaluation and CodeQL advisory-only disposition (SAST-01..04) |  | 34-AUDIT.md §2 row 40 |
| 45 | MB-39 | stale content | S | docs/src/appendix/build-baselines.md | Phase 28 -- `paladin-eval` crate added (OBS-04, SS-58) |  | 34-AUDIT.md §2 row 13 |
| 46 | MB-37 | stale content | S | docs/src/appendix/battalion-benchmarks.md | Phase 22.1 -- workspace MSRV floor raised 1.85 -> 1.88 (SS-08) |  | 34-AUDIT.md §2 row 9 |
| 47 | MB-55 | stale content | M | docs/src/appendix/sanctum-benchmarks.md | Milestone 2-3 -- Qdrant Sanctum adapter shipped ("Milestone 2-3 as-shipped ledger" |  | 34-AUDIT.md §2 row 37 |
| 48 | MB-56 | stale content | M | docs/src/appendix/sanctum-migration.md | Phase 22 through Phase 33 -- no REQ-ID applies |  | 34-AUDIT.md §2 row 39 |
| 49 | MB-54 | stale content | S | docs/src/appendix/release-automation.md | Phase 29 -- release-gate composition (SHIP-02, `check-release-consistency` job) |  | 34-AUDIT.md §2 row 34 |
| 50 | MB-01 | stale content | M | docs/src/appendix/doc-coverage-report.md | Phase 29 — cargo doc zero-`warning:` bar ratified (ADR-0033, D-00a) |  | 34-AUDIT.md §2 row 25 |
| 51 | MB-51 | stale content | S | docs/src/appendix/port-trait-template.md | Phase 22 through Phase 33 -- no REQ-ID applies (a pre-milestone documentation-template defect, D-00g) |  | 34-AUDIT.md §2 row 31 |
| 52 | MB-50 | stale content | M | docs/src/appendix/minio-file-repository-setup.md | Phase 22 through Phase 33 -- no single REQ-ID applies |  | 34-AUDIT.md §2 row 29 |
| 53 | MB-53 | stale content | M | docs/src/appendix/redis-queue-adapter-setup.md | Phase 25 -- `redis-cache` Cargo feature on `paladin-storage` shipped in an adjacent context (FT-06, SS-33) |  | 34-AUDIT.md §2 row 33 |
| 54 | MB-40 | stale content | S | docs/src/appendix/cli-configuration.md | Phase 27 -- `/v1/schedules*` cron-driven recurring run submission wired (PLAT-05, SS-48) |  | 34-AUDIT.md §2 row 14 |
| 55 | MB-41 | stale content | L | docs/src/appendix/cli-council.md | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone CLI surface) |  | 34-AUDIT.md §2 row 15 |
| 56 | MB-42 | stale content | L | docs/src/appendix/cli-muster.md | Phase 26 -- no Phase 22-33 REQ-ID applies (pre-milestone CLI surface) |  | 34-AUDIT.md §2 row 16 |
| 57 | MB-43 | stale content | M | docs/src/appendix/cli-onboarding.md | Phase 26 -- no Phase 22-33 REQ-ID applies |  | 34-AUDIT.md §2 row 17 |
| 58 | MB-44 | stale content | S | docs/src/appendix/cli-setup-check.md | Phase 26 -- no Phase 22-33 REQ-ID applies |  | 34-AUDIT.md §2 row 18 |
| 59 | MB-45 | stale content | S | docs/src/appendix/cli-testing.md | Phase 26 -- no Phase 22-33 REQ-ID applies |  | 34-AUDIT.md §2 row 19 |
| 60 | MB-47 | stale content | M | docs/src/appendix/contributing-legacy.md | Phase 22.1 -- MSRV floor raised 1.85->1.88 (SS-08) |  | 34-AUDIT.md §2 row 22 |

**Count:** 60 `MB-nn` rows — 1 missing page, 59 stale content. Sizes: 20 S, 19 M, 21 L.

## §6 Phase 36 work list

Sufficient for `/gsd-plan-phase 36` to plan every rustdoc finding (`RD-nn`, §3) and every example finding (`EX-nn`, §4) by ID. Phase 36 may re-batch these rows into its own plans/waves, but must close every ID listed below. Closing an `RD-nn` lead row's underlying source line also closes every ID in its `Blocks` cell (same file:line, found by a different measurement run — D-12/D-13 default-feature vs D-12/D-14 per-crate all-features — never two separate defects). Measured HEAD SHA: `ee1fb160f8e743e638b32beb6c4e32be4ede9325` (D-23). If the branch moves before Phase 36 plans, re-run the D-12/D-16 commands this audit ran (34-EVIDENCE.md) and diff against the rows below rather than trusting the counts unchanged.

Ordered per D-21: rustdoc rows (`RD-nn`) first, grouped by crate in the order `paladin-ai, paladin-web, paladin-battalion, paladin-storage, paladin-llm, paladin-ports, paladin-ai-core, paladin-memory` (row 112's own summary-line order); within a crate, a lead row with a non-empty `Blocks` cell precedes every follower it names, then the remainder in file:line order. Example rows (`EX-nn`) follow: the 5 stale rows from the build/currency sweep (§4's Program/module table) in their existing ID order, then the 59 gap-list rows (§4's Capability table) in their existing ID order, which is already ascending-phase (Phase 22 → Phase 33).

### RD-nn (rustdoc findings — the full enumeration below is Phase 36's closure surface for WINDOWS.md row 36's workspace-wide observation; RD-01/RD-66/RD-126 specifically close row 37's named `HeuristicTokenCounter` link)

| Order | ID | Classification | Size | Location | Cites (Phase N — item (REQ)) | Blocks | Evidence anchor |
|---|---|---|---|---|---|---|---|
| 1 | RD-137 | rustdoc warning or broken intra-doc link | S | src/application/cli/commands/eval.rs:281 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-ai.txt:2 |
| 2 | RD-02 | rustdoc warning or broken intra-doc link | S | src/application/services/paladin/paladin_execution_service.rs:1014 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-138 | 34-evidence/34-06-cargo-doc-default.txt:1 |
| 3 | RD-138 | rustdoc warning or broken intra-doc link | S | src/application/services/paladin/paladin_execution_service.rs:1014 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-02 |  | 34-evidence/34-07-percrate/paladin-ai.txt:12 |
| 4 | RD-03 | rustdoc warning or broken intra-doc link | S | src/application/services/parley/adapter.rs:28 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-139 | 34-evidence/34-06-cargo-doc-default.txt:10 |
| 5 | RD-139 | rustdoc warning or broken intra-doc link | S | src/application/services/parley/adapter.rs:28 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-03 |  | 34-evidence/34-07-percrate/paladin-ai.txt:20 |
| 6 | RD-04 | rustdoc warning or broken intra-doc link | S | src/application/services/run/worker.rs:641 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-140 | 34-evidence/34-06-cargo-doc-default.txt:18 |
| 7 | RD-140 | rustdoc warning or broken intra-doc link | S | src/application/services/run/worker.rs:641 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-04 |  | 34-evidence/34-07-percrate/paladin-ai.txt:28 |
| 8 | RD-05 | rustdoc warning or broken intra-doc link | S | src/config/agent_runtime.rs:1174 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-141 | 34-evidence/34-06-cargo-doc-default.txt:26 |
| 9 | RD-141 | rustdoc warning or broken intra-doc link | S | src/config/agent_runtime.rs:1174 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-05 |  | 34-evidence/34-07-percrate/paladin-ai.txt:36 |
| 10 | RD-142 | rustdoc warning or broken intra-doc link | S | src/infrastructure/telemetry/otel_sink.rs:42 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-ai.txt:44 |
| 11 | RD-06 | rustdoc warning or broken intra-doc link | S | src/presets/mod.rs:55 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-143 | 34-evidence/34-06-cargo-doc-default.txt:34 |
| 12 | RD-143 | rustdoc warning or broken intra-doc link | S | src/presets/mod.rs:55 | paladin-ai — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-06 |  | 34-evidence/34-07-percrate/paladin-ai.txt:52 |
| 13 | RD-133 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/dev_ui_controller.rs:131 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-web.txt:40 |
| 14 | RD-130 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/dev_ui_controller.rs:20 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-web.txt:13 |
| 15 | RD-131 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/dev_ui_controller.rs:28 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-web.txt:22 |
| 16 | RD-129 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/dev_ui_controller.rs:3 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-web.txt:2 |
| 17 | RD-132 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/dev_ui_controller.rs:69 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-web.txt:30 |
| 18 | RD-07 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:483 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-134 | 34-evidence/34-06-cargo-doc-default.txt:42 |
| 19 | RD-134 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:483 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-07 |  | 34-evidence/34-07-percrate/paladin-web.txt:48 |
| 20 | RD-08 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:686 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-135 | 34-evidence/34-06-cargo-doc-default.txt:51 |
| 21 | RD-135 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:686 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-08 |  | 34-evidence/34-07-percrate/paladin-web.txt:56 |
| 22 | RD-09 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:757 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-136 | 34-evidence/34-06-cargo-doc-default.txt:59 |
| 23 | RD-136 | rustdoc warning or broken intra-doc link | S | crates/paladin-web/src/thread_controller.rs:757 | paladin-web — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-09 |  | 34-evidence/34-07-percrate/paladin-web.txt:64 |
| 24 | RD-10 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/commander.rs:35 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-67 | 34-evidence/34-06-cargo-doc-default.txt:69 |
| 25 | RD-67 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/commander.rs:35 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-10 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:2 |
| 26 | RD-11 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/commander.rs:49 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-68 | 34-evidence/34-06-cargo-doc-default.txt:78 |
| 27 | RD-68 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/commander.rs:49 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-11 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:12 |
| 28 | RD-12 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/edge_evaluator.rs:3 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-69 | 34-evidence/34-06-cargo-doc-default.txt:86 |
| 29 | RD-69 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/edge_evaluator.rs:3 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-12 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:20 |
| 30 | RD-26 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/cache_key.rs:23 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-27, RD-83, RD-84 | 34-evidence/34-06-cargo-doc-default.txt:212 |
| 31 | RD-27 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/cache_key.rs:23 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-26 |  | 34-evidence/34-06-cargo-doc-default.txt:221 |
| 32 | RD-83 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/cache_key.rs:23 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-26 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:147 |
| 33 | RD-84 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/cache_key.rs:23 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-26 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:156 |
| 34 | RD-28 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/directive_parser.rs:47 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-85 | 34-evidence/34-06-cargo-doc-default.txt:230 |
| 35 | RD-85 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/directive_parser.rs:47 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-28 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:165 |
| 36 | RD-30 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:1375 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-87 | 34-evidence/34-06-cargo-doc-default.txt:246 |
| 37 | RD-87 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:1375 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-30 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:181 |
| 38 | RD-31 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2258 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-88 | 34-evidence/34-06-cargo-doc-default.txt:254 |
| 39 | RD-88 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2258 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-31 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:189 |
| 40 | RD-32 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2287 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-89 | 34-evidence/34-06-cargo-doc-default.txt:262 |
| 41 | RD-89 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2287 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-32 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:197 |
| 42 | RD-33 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2303 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-90 | 34-evidence/34-06-cargo-doc-default.txt:270 |
| 43 | RD-90 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2303 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-33 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:205 |
| 44 | RD-34 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2337 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-91 | 34-evidence/34-06-cargo-doc-default.txt:278 |
| 45 | RD-91 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2337 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-34 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:213 |
| 46 | RD-35 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2358 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-92 | 34-evidence/34-06-cargo-doc-default.txt:286 |
| 47 | RD-92 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:2358 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-35 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:221 |
| 48 | RD-29 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:772 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-86 | 34-evidence/34-06-cargo-doc-default.txt:238 |
| 49 | RD-86 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/graph.rs:772 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-29 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:173 |
| 50 | RD-43 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/input_mapping.rs:30 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-100 | 34-evidence/34-06-cargo-doc-default.txt:351 |
| 51 | RD-100 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/input_mapping.rs:30 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-43 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:286 |
| 52 | RD-44 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/input_mapping.rs:40 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-101 | 34-evidence/34-06-cargo-doc-default.txt:368 |
| 53 | RD-101 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/input_mapping.rs:40 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-44 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:304 |
| 54 | RD-18 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:10 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-75 | 34-evidence/34-06-cargo-doc-default.txt:141 |
| 55 | RD-75 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:10 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-18 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:76 |
| 56 | RD-45 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:1085 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-102 | 34-evidence/34-06-cargo-doc-default.txt:384 |
| 57 | RD-102 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:1085 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-45 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:320 |
| 58 | RD-37 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:1423 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-94 | 34-evidence/34-06-cargo-doc-default.txt:302 |
| 59 | RD-94 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:1423 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-37 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:237 |
| 60 | RD-19 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:20 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-76 | 34-evidence/34-06-cargo-doc-default.txt:149 |
| 61 | RD-76 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:20 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-19 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:84 |
| 62 | RD-20 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:25 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-77 | 34-evidence/34-06-cargo-doc-default.txt:158 |
| 63 | RD-77 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:25 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-20 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:93 |
| 64 | RD-38 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:2686 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-95 | 34-evidence/34-06-cargo-doc-default.txt:310 |
| 65 | RD-95 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:2686 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-38 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:245 |
| 66 | RD-21 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:27 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-78 | 34-evidence/34-06-cargo-doc-default.txt:167 |
| 67 | RD-78 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:27 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-21 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:102 |
| 68 | RD-13 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:3 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-70 | 34-evidence/34-06-cargo-doc-default.txt:96 |
| 69 | RD-70 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:3 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-13 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:31 |
| 70 | RD-22 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:31 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-79 | 34-evidence/34-06-cargo-doc-default.txt:176 |
| 71 | RD-79 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:31 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-22 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:111 |
| 72 | RD-23 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:33 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-80 | 34-evidence/34-06-cargo-doc-default.txt:185 |
| 73 | RD-80 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:33 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-23 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:120 |
| 74 | RD-24 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:34 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-81 | 34-evidence/34-06-cargo-doc-default.txt:194 |
| 75 | RD-81 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:34 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-24 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:129 |
| 76 | RD-25 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:36 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-82 | 34-evidence/34-06-cargo-doc-default.txt:203 |
| 77 | RD-82 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:36 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-25 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:138 |
| 78 | RD-14 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:4 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-71 | 34-evidence/34-06-cargo-doc-default.txt:105 |
| 79 | RD-71 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:4 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-14 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:40 |
| 80 | RD-15 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:5 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-72 | 34-evidence/34-06-cargo-doc-default.txt:114 |
| 81 | RD-72 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:5 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-15 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:49 |
| 82 | RD-16 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:6 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-17, RD-73, RD-74 | 34-evidence/34-06-cargo-doc-default.txt:123 |
| 83 | RD-17 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:6 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-16 |  | 34-evidence/34-06-cargo-doc-default.txt:132 |
| 84 | RD-73 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:6 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-16 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:58 |
| 85 | RD-74 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:6 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-16 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:67 |
| 86 | RD-36 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:744 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-93 | 34-evidence/34-06-cargo-doc-default.txt:294 |
| 87 | RD-93 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/engine/mod.rs:744 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-36 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:229 |
| 88 | RD-39 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_decision.rs:40 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-96 | 34-evidence/34-06-cargo-doc-default.txt:318 |
| 89 | RD-96 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_decision.rs:40 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-39 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:253 |
| 90 | RD-40 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:1 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-97 | 34-evidence/34-06-cargo-doc-default.txt:327 |
| 91 | RD-97 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:1 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-40 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:262 |
| 92 | RD-42 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:38 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-99 | 34-evidence/34-06-cargo-doc-default.txt:343 |
| 93 | RD-99 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:38 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-42 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:278 |
| 94 | RD-41 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:8 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-98 | 34-evidence/34-06-cargo-doc-default.txt:335 |
| 95 | RD-98 | rustdoc warning or broken intra-doc link | S | crates/paladin-battalion/src/llm_failure.rs:8 | paladin-battalion — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-41 |  | 34-evidence/34-07-percrate/paladin-battalion.txt:270 |
| 96 | RD-46 | rustdoc warning or broken intra-doc link | S | crates/paladin-storage/src/waypoint/contract_tests.rs:673 | paladin-storage — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-128 | 34-evidence/34-06-cargo-doc-default.txt:400 |
| 97 | RD-128 | rustdoc warning or broken intra-doc link | S | crates/paladin-storage/src/waypoint/contract_tests.rs:673 | paladin-storage — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-46 |  | 34-evidence/34-07-percrate/paladin-storage.txt:2 |
| 98 | RD-121 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/compat/engine.rs:1041 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-llm.txt:36 |
| 99 | RD-119 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/compat/engine.rs:114 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-llm.txt:20 |
| 100 | RD-120 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/compat/engine.rs:201 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-llm.txt:28 |
| 101 | RD-122 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/gemini/adapter.rs:28 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-llm.txt:44 |
| 102 | RD-123 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/gemini/adapter.rs:59 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) |  | 34-evidence/34-07-percrate/paladin-llm.txt:52 |
| 103 | RD-49 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/http_status.rs:6 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-124 | 34-evidence/34-06-cargo-doc-default.txt:427 |
| 104 | RD-124 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/http_status.rs:6 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-49 |  | 34-evidence/34-07-percrate/paladin-llm.txt:60 |
| 105 | RD-50 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/http_status.rs:7 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-125 | 34-evidence/34-06-cargo-doc-default.txt:431 |
| 106 | RD-125 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/http_status.rs:7 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-50 |  | 34-evidence/34-07-percrate/paladin-llm.txt:65 |
| 107 | RD-47 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/redaction.rs:164 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-117 | 34-evidence/34-06-cargo-doc-default.txt:410 |
| 108 | RD-117 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/redaction.rs:164 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-47 |  | 34-evidence/34-07-percrate/paladin-llm.txt:2 |
| 109 | RD-48 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/services/commissary.rs:89 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-118 | 34-evidence/34-06-cargo-doc-default.txt:419 |
| 110 | RD-118 | rustdoc warning or broken intra-doc link | S | crates/paladin-llm/src/services/commissary.rs:89 | paladin-llm — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-48 |  | 34-evidence/34-07-percrate/paladin-llm.txt:12 |
| 111 | RD-51 | rustdoc warning or broken intra-doc link | S | crates/paladin-ports/src/output/structured_executor_port.rs:158 | paladin-ports — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-127 | 34-evidence/34-06-cargo-doc-default.txt:436 |
| 112 | RD-127 | rustdoc warning or broken intra-doc link | S | crates/paladin-ports/src/output/structured_executor_port.rs:158 | paladin-ports — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-51 |  | 34-evidence/34-07-percrate/paladin-ports.txt:2 |
| 113 | RD-52 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/directive.rs:3 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-103 | 34-evidence/34-06-cargo-doc-default.txt:445 |
| 114 | RD-103 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/directive.rs:3 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-52 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:2 |
| 115 | RD-53 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/structured.rs:13 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-104 | 34-evidence/34-06-cargo-doc-default.txt:453 |
| 116 | RD-104 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/structured.rs:13 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-53 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:11 |
| 117 | RD-56 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:17 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-107 | 34-evidence/34-06-cargo-doc-default.txt:480 |
| 118 | RD-107 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:17 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-56 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:38 |
| 119 | RD-57 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:21 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-108 | 34-evidence/34-06-cargo-doc-default.txt:489 |
| 120 | RD-108 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:21 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-57 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:47 |
| 121 | RD-58 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:22 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-109 | 34-evidence/34-06-cargo-doc-default.txt:497 |
| 122 | RD-109 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:22 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-58 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:55 |
| 123 | RD-59 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:25 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-110 | 34-evidence/34-06-cargo-doc-default.txt:506 |
| 124 | RD-110 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:25 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-59 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:64 |
| 125 | RD-54 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:3 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-105 | 34-evidence/34-06-cargo-doc-default.txt:462 |
| 126 | RD-105 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:3 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-54 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:20 |
| 127 | RD-60 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:36 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-111 | 34-evidence/34-06-cargo-doc-default.txt:514 |
| 128 | RD-111 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:36 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-60 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:72 |
| 129 | RD-61 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:37 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-112 | 34-evidence/34-06-cargo-doc-default.txt:522 |
| 130 | RD-112 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:37 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-61 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:80 |
| 131 | RD-62 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:44 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-113 | 34-evidence/34-06-cargo-doc-default.txt:530 |
| 132 | RD-113 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:44 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-62 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:88 |
| 133 | RD-63 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:45 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-114 | 34-evidence/34-06-cargo-doc-default.txt:538 |
| 134 | RD-114 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:45 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-63 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:96 |
| 135 | RD-55 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:6 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-106 | 34-evidence/34-06-cargo-doc-default.txt:471 |
| 136 | RD-106 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/trace.rs:6 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-55 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:29 |
| 137 | RD-64 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/webhook.rs:19 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-115 | 34-evidence/34-06-cargo-doc-default.txt:546 |
| 138 | RD-115 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/webhook.rs:19 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-64 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:104 |
| 139 | RD-65 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/webhook.rs:20 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a) | RD-116 | 34-evidence/34-06-cargo-doc-default.txt:555 |
| 140 | RD-116 | rustdoc warning or broken intra-doc link | S | crates/paladin-core/src/platform/container/webhook.rs:20 | paladin-ai-core — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); same source line as RD-65 |  | 34-evidence/34-07-percrate/paladin-ai-core.txt:113 |
| 141 | RD-01 | rustdoc warning or broken intra-doc link | S | crates/paladin-memory/src/token_counter/mod.rs:3 | paladin-memory — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); closes WINDOWS.md row 37 (`HeuristicTokenCounter`) | RD-66, RD-126 | 34-EVIDENCE.md #6, #7 |
| 142 | RD-66 | rustdoc warning or broken intra-doc link | S | crates/paladin-memory/src/token_counter/mod.rs:3 | paladin-memory — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); closes WINDOWS.md row 37 (`HeuristicTokenCounter`); same source line as RD-01 |  | 34-evidence/34-06-cargo-doc-default.txt:566 |
| 143 | RD-126 | rustdoc warning or broken intra-doc link | S | crates/paladin-memory/src/token_counter/mod.rs:3 | paladin-memory — cargo doc zero-`warning:` bar (Phase 29 — ADR-0033, D-00a); closes WINDOWS.md row 37 (`HeuristicTokenCounter`); same source line as RD-01 |  | 34-evidence/34-07-percrate/paladin-memory.txt:2 |

**Count:** 143 `RD-nn` rows in 75 location groups (63 groups have a follower closed by the same fix). All sized S per D-04 (a rustdoc link repair is a one-line fix).

### EX-nn (examples — currency and gap findings)

| Order | ID | Classification | Size | Location | Cites (Phase N — item (REQ)) | Blocks | Evidence anchor |
|---|---|---|---|---|---|---|---|
| 144 | EX-01 | non-compiling or obsolete example | S | examples/README.md | Phase 22.1 — workspace MSRV floor raised 1.85 → 1.88 (SS-08) |  | 34-EVIDENCE.md #8 |
| 145 | EX-33 | non-compiling or obsolete example | M | examples/http_service_host.rs | Phase 24 — `thread_router` mounted (HITL, SS-23…SS-25); Phase 27 — `run_router` mounted (Platform API, SS-44…SS-52) |  | 34-evidence/34-08-examples-builds.txt (Invocation 4) |
| 146 | EX-55 | non-compiling or obsolete example | M | crates/doc-examples/src/http_service_host.rs | Phase 24 — `thread_router` mounted (HITL, SS-23…SS-25); Phase 27 — `run_router` mounted (Platform API, SS-44…SS-52) — sibling of EX-33 |  | 34-evidence/34-08-examples-builds.txt (Extra target 1: scripts/check-doc-examples.sh, Layer 1) |
| 147 | EX-121 | non-compiling or obsolete example | L | examples/README.md | Cross-phase — examples/README.md gallery completeness; the 11 undocumented programs span Phases 22-33 (see the EX-62…EX-120 gap list for the underlying capabilities) |  | 34-EVIDENCE.md #159 |
| 148 | EX-122 | non-compiling or obsolete example | S | examples/README.md | Pre-milestone — `PaladinResult` field naming (`output`/`usage`/`execution_time_ms`); the README's own code snippet was never updated to match, not a Phase 22-33 regression |  | 34-EVIDENCE.md #160 |
| 149 | EX-62 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 22 — Injecting a custom WaypointPort backend (InMemory/SQLite/Postgres) for checkpoint snapshots (SS-03) (ENG-05) |  | `grep -rlF 'WaypointPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 150 | EX-63 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 22 — Reading/inspecting the waypoints persistence table directly (SS-04) (ENG-03) |  | `grep -rlF 'waypoints' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 151 | EX-64 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 22 — Configuring WarEngine via EngineConfig (max_supersteps, max_node_visits, run_timeout_secs, waypoint_durability, max_muster_tasks) (SS-05) (ENG-02) |  | `grep -rlF 'EngineConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 152 | EX-65 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 22 — Overriding the superstep cap via APP_ENGINE_MAX_SUPERSTEPS (SS-06) (ENG-02) |  | `grep -rlF 'APP_ENGINE_MAX_SUPERSTEPS' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 153 | EX-66 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 22 — Pruning old Waypoints via WaypointRetentionService/WaypointRetentionConfig (SS-07) (ENG-05) |  | `grep -rlF 'WaypointRetentionService' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 154 | EX-67 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 23 — EdgeCondition::Custom's fail-closed behavior when unregistered (SS-11) (CF-01) |  | `grep -rlF 'EdgeCondition::Custom' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 155 | EX-68 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 23 — Nested subgraph composition via NodeSpec::Battalion (SS-14) (CF-04) |  | `grep -rlF 'NodeSpec::Battalion' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 156 | EX-69 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 23 — LLM-driven dynamic routing via LlmDecisionEvaluator / Commander StrategySelection::Semantic (SS-15) (CF-05) |  | `grep -rlF 'LlmDecisionEvaluator' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 157 | EX-70 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 23 — Overriding the Muster fan-out cap via APP_ENGINE_MAX_MUSTER_TASKS (SS-16) (CF-03) |  | `grep -rlF 'APP_ENGINE_MAX_MUSTER_TASKS' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 158 | EX-71 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — First-class approval-gate nodes via NodeSpec::Gate (SS-17) (HITL-01) |  | `grep -rlF 'NodeSpec::Gate' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 159 | EX-72 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Typed, total-validation resume via WarEngine::resume_with(graph, thread, responses) (SS-18) (HITL-02) |  | `grep -rlF 'resume_with' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 160 | EX-73 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — History/replay/fork via ChronicleService + WarEngine::replay/fork (SS-19) (HITL-03) |  | `grep -rlF 'ChronicleService' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 161 | EX-74 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Graceful shutdown on SIGTERM/SIGINT via ShutdownCoordinator (SS-20) (HITL-04) |  | `grep -rlF 'ShutdownCoordinator' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 162 | EX-75 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Configuring shutdown grace period via APP_ENGINE_SHUTDOWN_GRACE_SECS (SS-21) (HITL-04) |  | `grep -rlF 'APP_ENGINE_SHUTDOWN_GRACE_SECS' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 163 | EX-76 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Toggling graceful shutdown via APP_ENGINE_GRACEFUL_SHUTDOWN (SS-22) (HITL-04) |  | `grep -rlF 'APP_ENGINE_GRACEFUL_SHUTDOWN' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 164 | EX-77 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Reading paused-thread state via GET /v1/threads/{id}/state (SS-23) (HITL-05) |  | `grep -rlF 'GET /v1/threads/{id}/state' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 165 | EX-78 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Resuming a paused thread via POST /v1/threads/{id}/resume (SS-24) (HITL-05) |  | `grep -rlF 'POST /v1/threads/{id}/resume' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 166 | EX-79 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — Paginated history retrieval via GET /v1/threads/{id}/history (SS-25) (HITL-05) |  | `grep -rlF 'GET /v1/threads/{id}/history' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 167 | EX-80 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 24 — The v3→v4 graph-fingerprint bump for Gate node routing properties (SS-26) (HITL-01) |  | `grep -rlF 'GRAPH_FINGERPRINT_VERSION' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 168 | EX-81 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 25 — Enabling node-result caching via the redis-cache Cargo feature (SS-33) (FT-06) |  | `grep -rlF 'redis-cache' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 169 | EX-82 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 25 — Toggling node-result caching via APP_NODE_CACHE_ENABLED (SS-34) (FT-06) |  | `grep -rlF 'APP_NODE_CACHE_ENABLED' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 170 | EX-83 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Writing a custom ExecutionMiddleware (before_model/after_model/around_tool) (SS-35) (RT-01) |  | `grep -rlF 'ExecutionMiddleware' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 171 | EX-84 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Configuring the twelve built-in middleware sub-structs via AgentRuntimeConfig (SS-36) (RT-02) |  | `grep -rlF 'AgentRuntimeConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 172 | EX-85 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Implementing a custom TokenCounterPort (SS-37) (RT-03) |  | `grep -rlF 'TokenCounterPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 173 | EX-86 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Context-window management via HistoryTrimmer + SummarizationMiddleware (SS-38) (RT-03) |  | `grep -rlF 'HistoryTrimmer' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 174 | EX-87 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Structural memory namespacing via VaultPort / ConfinedVault (SS-39) (RT-04) |  | `grep -rlF 'VaultPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 175 | EX-88 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Schema-validated structured output via StructuredExecutorPort / execute_structured<T> (SS-40) (RT-05) |  | `grep -rlF 'StructuredExecutorPort' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 176 | EX-89 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Opting into tool_error_mode = FailRun with redact-then-bound tool-text sanitization (SS-42) (RT-07) |  | `grep -rlF 'tool_error_mode' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 177 | EX-90 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 26 — Deriving a JSON schema for structured output via schemars (SS-43) (RT-05) |  | `grep -rlF 'schemars' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 178 | EX-91 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Decoupled run submission via POST /v1/runs (SS-44) (PLAT-01) |  | `grep -rlF 'POST /v1/runs' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 179 | EX-92 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — SSE run streaming via GET /v1/runs/{run_id}/stream (seven frozen wire events) (SS-45) (PLAT-03) |  | `grep -rlF 'GET /v1/runs/{run_id}/stream' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 180 | EX-93 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Cancelling an in-flight run via POST /v1/runs/{run_id}/cancel (SS-46) (PLAT-02) |  | `grep -rlF 'POST /v1/runs/{run_id}/cancel' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 181 | EX-94 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Managing immutable assistant versions via /v1/assistants* (SS-47) (PLAT-04) |  | `grep -rlF '/v1/assistants' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 182 | EX-95 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Cron-driven recurring run submission via /v1/schedules* (SS-48) (PLAT-05) |  | `grep -rlF '/v1/schedules' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 183 | EX-96 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Webhook delivery with X-Paladin-Signature HMAC verification (SS-49) (PLAT-05) |  | `grep -rlF 'webhook_deliveries' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 184 | EX-97 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — The SSRF-guard override APP_WEBHOOKS_ALLOW_PRIVATE (SS-50) (PLAT-05) |  | `grep -rlF 'APP_WEBHOOKS_ALLOW_PRIVATE' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 185 | EX-98 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Durable worker-pool dispatch via RunQueuePort (InMemory + Redis) (SS-51) (PLAT-02) |  | `grep -rlF 'RunQueuePort' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 186 | EX-99 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 27 — Selecting a durable run-store backend via APP_RUN_STORE_BACKEND (SS-52) (PLAT-01) |  | `grep -rlF 'APP_RUN_STORE_BACKEND' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 187 | EX-100 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Consuming the TraceRecord envelope / twelve TraceEvent variants (SS-53) (OBS-01) |  | `grep -rlF 'TraceRecord' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 188 | EX-101 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Configuring tracing via TraceConfig (log_sink/persist/state_values/otel) (SS-54) (OBS-02) |  | `grep -rlF 'TraceConfig' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 189 | EX-102 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Enabling OTel export via PALADIN_TRACE_OTEL_ENABLED (SS-55) (OBS-02) |  | `grep -rlF 'PALADIN_TRACE_OTEL_ENABLED' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 190 | EX-103 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Wiring the otel Cargo feature (opentelemetry/opentelemetry_sdk/opentelemetry-otlp) (SS-56) (OBS-02) |  | `grep -rlF 'otel' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 191 | EX-104 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — The admin-gated, dev-ui-feature-gated GET /v1/dev-ui/threads/{id} route (SS-57) (OBS-03) |  | `grep -rlF '/v1/dev-ui/threads' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 192 | EX-105 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Writing eval scenarios via the paladin-eval crate + eval_scenarios! macro (SS-58) (OBS-04) |  | `grep -rlF 'paladin-eval' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 193 | EX-106 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Enabling live-mode eval runs via PALADIN_EVAL_LIVE (SS-59) (OBS-04) |  | `grep -rlF 'PALADIN_EVAL_LIVE' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 194 | EX-107 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Running eval scenarios via paladin-cli eval run <glob> (SS-60) (OBS-04) |  | `grep -rlF 'eval run' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 195 | EX-108 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 28 — Querying the append-only run_traces persisted-trace-history table (SS-62) (OBS-02) |  | `grep -rlF 'run_traces' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 196 | EX-109 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 30 — Constructing/using Commissary directly (input-side, per-call window-rationing) (SS-68) (VOCAB-02) |  | `grep -rlF 'Commissary' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 197 | EX-110 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 31 — The HTTP-surface TokenUsageResponse DTO on ExecuteResponse.usage (SS-77) (ACCT-02) |  | `grep -rlF 'TokenUsageResponse' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 198 | EX-111 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 31 — Anthropic's fixed prompt_tokens figure now including cache-read/cache-write tokens (SS-78) (ACCT-03) |  | `grep -rlF 'prompt_tokens' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 199 | EX-112 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 32 — TokenCounterPort::is_exact(&self) -> bool's defaulted behavior (SS-80) (PRIM-01) |  | `grep -rlF 'is_exact' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 200 | EX-113 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 32 — Commissary::new/from_port's is_exact_counter-argument removal (exactness read live from is_exact) (SS-81) (PRIM-02) |  | `grep -rlF 'Commissary::new' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 201 | EX-114 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 32 — The shared precedence resolver paladin_llm::window::resolve_context_window (SS-82) (PRIM-04) |  | `grep -rlF 'resolve_context_window' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 202 | EX-115 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 32 — WindowSource/WindowFallbackPolicy/ResolvedWindow re-exported from the paladin facade (SS-83) (PRIM-04) |  | `grep -rlF 'WindowSource' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 203 | EX-116 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 33 — Reading RagRetrievalService::retrieve_context's new RagRetrievalResult return type (SS-86) (COMM-01) |  | `grep -rlF 'RagRetrievalResult' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 204 | EX-117 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 33 — Reading the truncation/shed record via RagRetrievalResult.shed: Vec<ShedItem> (SS-87) (COMM-02) |  | `grep -rlF 'ShedItem' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 205 | EX-118 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 33 — Handling the typed RagRetrievalError enum (Sanctum/Commissary/budget-conversion failures) (SS-88) (COMM-01) |  | `grep -rlF 'RagRetrievalError' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 206 | EX-119 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 33 — The free function retrieve_context_with_timeout returning RagRetrievalResult (SS-89) (COMM-01) |  | `grep -rlF 'retrieve_context_with_timeout' examples/ crates/doc-examples/src/` → (no output, 0 hits) |
| 207 | EX-120 | non-compiling or obsolete example | L | examples/ (gap — no program demonstrates this capability) | Phase 33 — Injecting an exact token counter via RagRetrievalService::with_token_counter (SS-91) (COMM-04) |  | `grep -rlF 'with_token_counter' examples/ crates/doc-examples/src/` → (no output, 0 hits) |

**Count:** 64 `EX-nn` rows — 5 currency/obsolescence findings on existing programs (2 S, 2 M, 1 L), plus 59 gap-list rows for a Phase 22-33 capability no example demonstrates (all sized L per D-17(c)).

### EX-nn confirmed current (not work items, D-19 / phase_specific_rules point 5 — listed here only for ID-completeness/reconciliation, excluded from the Order sequence and the work-item counts above)

Every program in `examples/` and `crates/doc-examples/src/` that this audit's build/currency sweep confirmed `current` (builds green, no removed/renamed API named, capability claim matches the tree) carries its own `EX-nn` ID in §4's Program/module table, per D-17's per-program numbering — but a `current` verdict is not a finding (mirrors §2's own rule that a `current` mdBook page mints no `MB-nn` at all). These IDs require no Phase 36 action; they are listed below, once each, so the reconciliation check below can account for every ID §4 names without misrepresenting a passing check as an open work item.

| ID | Program / module |
|---|---|
| EX-02 | examples/agent_handoffs.rs |
| EX-03 | examples/arsenal_stdio_tools.rs |
| EX-04 | examples/arsenal_streamable_http_tools.rs |
| EX-05 | examples/autonomous_full_config.rs |
| EX-06 | examples/autonomous_planning.rs |
| EX-07 | examples/autonomous_prompt_generation.rs |
| EX-08 | examples/basic_paladin.rs |
| EX-09 | examples/battalion_checkpoint_recovery.rs |
| EX-10 | examples/campaign_workflow.rs |
| EX-11 | examples/chain_of_command_delegation.rs |
| EX-12 | examples/citadel_autosave.rs |
| EX-13 | examples/citadel_restore.rs |
| EX-14 | examples/commander_auto.rs |
| EX-15 | examples/commander_basic.rs |
| EX-16 | examples/commander_council.rs |
| EX-17 | examples/commander_full_config.rs |
| EX-18 | examples/commander_grove.rs |
| EX-19 | examples/commander_with_metadata_export.rs |
| EX-20 | examples/conclave_expert_panel.rs |
| EX-21 | examples/council_discussion.rs |
| EX-22 | examples/document_processing.rs |
| EX-23 | examples/dynamic_temperature.rs |
| EX-24 | examples/formation_sequential.rs |
| EX-25 | examples/garrison_in_memory.rs |
| EX-26 | examples/garrison_persistent.rs |
| EX-27 | examples/garrison_semantic_search.rs |
| EX-28 | examples/grove_routing.rs |
| EX-29 | examples/herald_custom_formatter.rs |
| EX-30 | examples/herald_json_output.rs |
| EX-31 | examples/herald_markdown_output.rs |
| EX-32 | examples/herald_streaming.rs |
| EX-34 | examples/llm_provider_selection.rs |
| EX-35 | examples/maneuver_basic.rs |
| EX-36 | examples/maneuver_dynamic_flow.rs |
| EX-37 | examples/maneuver_nested_flow.rs |
| EX-38 | examples/muster_baseline.rs |
| EX-39 | examples/paladin_with_config.rs |
| EX-40 | examples/paladin_with_rag.rs |
| EX-41 | examples/paladin_with_sanctum.rs |
| EX-42 | examples/phalanx_parallel.rs |
| EX-43 | examples/sanctum_adapter_migration.rs |
| EX-44 | examples/sanctum_basic_inmemory.rs |
| EX-45 | examples/sanctum_configuration.rs |
| EX-46 | examples/sanctum_qdrant_production.rs |
| EX-47 | examples/vision_analysis.rs |
| EX-48 | examples/vision_battalion.rs |
| EX-49 | examples/war_engine_memory_baseline.rs |
| EX-50 | crates/doc-examples/src/agent_runtime.rs |
| EX-51 | crates/doc-examples/src/bridge.rs |
| EX-52 | crates/doc-examples/src/content.rs |
| EX-53 | crates/doc-examples/src/deployment_topologies.rs |
| EX-54 | crates/doc-examples/src/fault_tolerance.rs |
| EX-56 | crates/doc-examples/src/orchestration.rs |
| EX-57 | crates/doc-examples/src/queue_worker.rs |
| EX-58 | crates/doc-examples/src/readme.rs |
| EX-59 | crates/doc-examples/src/sidecar.rs |
| EX-60 | crates/doc-examples/src/support.rs |
| EX-61 | crates/paladin-llm/examples/live_vendor_smoke.rs |

**Count:** 58 `EX-nn` IDs confirmed `current`, 0 Phase 36 action required.

## Reconciliation (plan 34-09, Task 1)

Both directions checked mechanically, not asserted. Every command below was run against this file
after §5/§6 were written; results are verbatim.

**Forward direction — every ID minted in §2/§3/§4 is routed:**

```
$ sed -n '/^## §2/,/^## §3/p' 34-AUDIT.md | grep -oE 'MB-[0-9]+' | sort -u | wc -l
60
$ sed -n '/^## §5/,/^## §6/p' 34-AUDIT.md | grep -oE 'MB-[0-9]+' | sort -u | wc -l
60
$ diff <(sed -n '/^## §2/,/^## §3/p' 34-AUDIT.md | grep -oE 'MB-[0-9]+' | sort -u) \
       <(sed -n '/^## §5/,/^## §6/p' 34-AUDIT.md | grep -oE 'MB-[0-9]+' | sort -u)
(empty — the two 60-ID sets are byte-identical)

$ sed -n '/^## §3/,/^## §4/p' 34-AUDIT.md | grep -oE 'RD-[0-9]+' | sort -u | wc -l
143
$ diff <(sed -n '/^## §3/,/^## §4/p' 34-AUDIT.md | grep -oE 'RD-[0-9]+' | sort -u) \
       <(sed -n '/^## §6/,/^## §7/p' 34-AUDIT.md | grep -oE 'RD-[0-9]+' | sort -u)
(empty — the two 143-ID sets are byte-identical)

$ sed -n '/^## §4/,/^## §5/p' 34-AUDIT.md | grep -oE 'EX-[0-9]+' | sort -u | wc -l
122
$ diff <(sed -n '/^## §4/,/^## §5/p' 34-AUDIT.md | grep -oE 'EX-[0-9]+' | sort -u) \
       <(sed -n '/^## §6/,/^## §7/p' 34-AUDIT.md | grep -oE 'EX-[0-9]+' | sort -u)
(empty — the two 122-ID sets are byte-identical; 64 as Order-numbered work-item rows, 58 as
"confirmed current" ID-completeness rows, per the classification split immediately above)
```

**Reverse direction — every ID appearing in a work list resolves to an originating row:** by
construction, every row in §5 and §6 (and the confirmed-current list) was generated directly from
its §2/§3/§4 originating row (Location, Size and Cites copied verbatim, never re-derived) — the
forward-direction set-equality proof above is symmetric proof of the reverse, since an empty `diff`
between two ID sets means neither set contains an element absent from the other.

**No `MB-nn` in §6, no `RD-nn`/`EX-nn` in §5:**

```
$ awk '/^## §5/,/^## §6/' 34-AUDIT.md | grep -oE '(RD|EX)-[0-9]+' | sort -u | wc -l
0
$ awk '/^## §6/,/^## §7/' 34-AUDIT.md | grep -oE 'MB-[0-9]+' | sort -u | wc -l
0
```

**Counts per list and per classification:**

| List | Classification | Count |
|---|---|---|
| §5 (Phase 35) | missing page | 1 |
| §5 (Phase 35) | stale content | 59 |
| §5 total | | **60** |
| §6 (Phase 36) | rustdoc warning or broken intra-doc link | 143 |
| §6 (Phase 36) | non-compiling or obsolete example | 64 |
| §6 total (work items) | | **207** |
| §6 (Phase 36) | confirmed current (not a work item) | 58 |
| §6 total (all EX/RD IDs accounted for) | | **265** |

**Sizes:** §5 — 1 L (the missing-page row leading the list), plus the remaining 59 mixed S/M/L (kept identical to their §2
originating row, never re-derived, per this task's action text). §6 RD-nn — 143 S (D-04: a
rustdoc link repair is always a one-line fix). §6 EX-nn work items — 59 L (the gap list, D-17(c))
plus 2 S / 2 M / 1 L from the 5 currency findings on existing programs.

**SC5 read-only proof (Task 1 close):**

```
$ git status --porcelain -- . ':!.planning'
(empty)
```

No file outside `.planning/` was created, modified or deleted by this task.

## §7 Deferred routing

Empty. Assembled by plan 34-09 from `deferred-items.md` — pointers only, per D-19 ("nothing is
absorbed into 35 or 36 by convenience").

---

*Phase: 34-documentation-currency-audit*
*Plan 34-01 wrote the header, all seven section headings, the 92 seeded §2 rows, and the one
worked row in each of §2/§3/§4. Every later plan in this phase appends rows; none renumbers or
removes a row already present here (D-03).*
