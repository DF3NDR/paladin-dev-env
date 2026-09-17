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
literal `pending — not yet swept (no signal class run)` — this placeholder is the
`16-DOCS-01-VERDICTS.md` concurrency rule made mechanical: an unswept page must never be
indistinguishable from a checked one.

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
| 9 | docs/src/appendix/battalion-benchmarks.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 10 | docs/src/appendix/battalion-patterns-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 11 | docs/src/appendix/battalion-vision-support.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 12 | docs/src/appendix/branch-protection.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 13 | docs/src/appendix/build-baselines.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 14 | docs/src/appendix/cli-configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 15 | docs/src/appendix/cli-council.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 16 | docs/src/appendix/cli-muster.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 17 | docs/src/appendix/cli-onboarding.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 18 | docs/src/appendix/cli-setup-check.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 19 | docs/src/appendix/cli-testing.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 20 | docs/src/appendix/cli-usage.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 21 | docs/src/appendix/conclave-pattern.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 22 | docs/src/appendix/contributing-legacy.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 23 | docs/src/appendix/council.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 24 | docs/src/appendix/design-and-architecture.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 25 | docs/src/appendix/doc-coverage-report.md | **stale** | class 1 (version strings): `grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+'` → none; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 hits (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`, `paladin-notifications`, `paladin-content`, `paladin-storage` — an incomplete, 2026-05-28-era list missing `paladin-eval`/`paladin-herald`/the `paladin-ai` facade); direct measurement: `cargo doc --workspace --no-deps 2>&1 \| tee 34-evidence/34-01-cargo-doc-default.txt` → **73** `warning:` lines (34-EVIDENCE.md #5), directly contradicting this page's line 18 ("Current result: docs build succeeds with no warnings") | Phase 29 — cargo doc zero-`warning:` bar ratified (ADR-0033, D-00a); Phase 33 — 73-warning baseline carried (`33-CI-EVIDENCE.md` row 26) (CURR-02) | MB-01 | M |
| 26 | docs/src/appendix/flow-dsl-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 27 | docs/src/appendix/grove.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 28 | docs/src/appendix/integration-tests.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 29 | docs/src/appendix/minio-file-repository-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 30 | docs/src/appendix/performance-baseline.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 31 | docs/src/appendix/port-trait-template.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 32 | docs/src/appendix/provider-expansion.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 33 | docs/src/appendix/redis-queue-adapter-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 34 | docs/src/appendix/release-automation.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 35 | docs/src/appendix/release-checklist.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 36 | docs/src/appendix/release-recovery.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 37 | docs/src/appendix/sanctum-benchmarks.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 38 | docs/src/appendix/sanctum-deployment.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 39 | docs/src/appendix/sanctum-migration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 40 | docs/src/appendix/security-scanning.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 41 | docs/src/appendix/sentinel.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 42 | docs/src/appendix/user-rest-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 43 | docs/src/appendix/user-system.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 44 | docs/src/architecture/commissary.md | current | class 4 (module/source paths): `crates/paladin-llm/src/services/commissary.rs` confirmed present, and the two cited line ranges (`644-698`, `911-929`) checked against the live file's own test helpers; class 7 (error types): all 5 `CommissaryError` variants named on the page (`UndeclaredContextWindow`, `ReservationExceedsWindow`, `FixedMaterialExceedsAllowance`, `ContextOverflow`, `InvalidConfig`) — `grep -n '^    [A-Z][A-Za-z]* {$\|^    [A-Z][A-Za-z]*(' crates/paladin-llm/src/services/commissary.rs` reproduces exactly these 5, no more, no fewer; direct check: `Commissary::new(provider, capabilities, counter, config)` signature (page lines 75-80) matches `sed -n '344,349p' crates/paladin-llm/src/services/commissary.rs` exactly (4 args, no `is_exact_counter`, per Phase 32 PRIM-02); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `Commissary`, `RagRetrievalResult`, `RagRetrievalError`, `with_token_counter`, `resolve_context_window` (via cross-reference) — all correct. **One pre-existing finding, not duplicated here:** line 7's literal `Quartermaster` match is recorded as its own row above (§2 Vocabulary sweep subsection, "stale content", historical ADR-pointer) — this page's own currency verdict is otherwise `current`; the page is judged `current` overall because that one line is a deliberate historical citation, not a content error, per D-00c (Phase 35's call) | — | — | — |
| 45 | docs/src/architecture/crate-map.md | **stale** | class 1 (version strings): line 9 "paladin-ai (root umbrella, v0.5.0)" — live `0.10.0` — **mismatch**; class 3 (crate names): `grep -noE 'paladin-[a-z-]+' \| sort -u` → 9 crates named, line 3 says "nine workspace crates" — live `ls crates/` → 11 library crates (`paladin-eval`, `paladin-herald` both absent) — **mismatch**; class 4 (module/source paths): `paladin-llm` feature table (lines 152-162) lists only `openai`/`anthropic`/`deepseek`/`mock`/`openai-embeddings`/`vision` — live `crates/paladin-llm/Cargo.toml` `[features]` also has `kimi`/`qwen`/`grok`/`ollama`/`openai-compatible`/`gemini` (Phase 17) — **6 features undocumented**; direct check (mandated, D-03 crate-graph edge): the mermaid dependency graph (lines 23-62) has no `mem --> llm` edge, though the prose immediately below (lines 174-176) correctly states the Phase 33 COMM-01 dependency — the diagram and the prose disagree with each other, and the diagram is what a reader actually parses; class 9: `grep -nFf 34-shipped-tokens.txt` → 0 hits | Phase 33 — `paladin-memory` gains an unconditional dependency on `paladin-llm` (COMM-01, SS-90) | MB-13 | L |
| 46 | docs/src/architecture/design-patterns.md | **stale** | class 1 (version strings): none on this page; class 4 (module/source paths): pattern 5's `PaladinExecutionService::new` code sample (lines 137-144) shows `new(llm, circuit_breaker, garrison, herald: Option<Arc<dyn Herald>>)` — live signature at `src/application/services/paladin/paladin_execution_service.rs:361-365` is `new(llm_port, circuit_breaker, garrison: Option<Arc<dyn GarrisonPort>>, arsenal: Option<Arc<dyn ArsenalPort>>)` — the fourth parameter is `arsenal`, not `herald` — **mismatch**, confirmed by direct read of the live constructor and its own doctest (which the page's pattern 1/2 code otherwise closely tracks); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits (this page's patterns predate Phase 22-33 and none of its code samples were touched by that work, so this is a pre-existing, not newly introduced, gap per D-00g) | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; no Phase 22-33 REQ-ID applies — the constructor's `arsenal` parameter predates this milestone) | MB-12 | S |
| 47 | docs/src/architecture/domain-model.md | **stale** | class 3 (crate names): the D-10 ubiquitous-language table (lines 12-32) confirmed current — `Commissary` present at line 30, matching the D-10 confirmation already recorded in §1; class 4 (module/source paths): "Core Domain Entities" (lines 63-159) documents `Paladin`/`Garrison`/`Arsenal`/`Citadel`/`Herald` from `crates/paladin-core/src/platform/container/`, but omits `Battlefield`, `Waypoint`, `Aegis` and `TraceRecord` — all four live in the exact same directory (`crates/paladin-core/src/platform/container/{battlefield,waypoint,aegis,trace}.rs`, confirmed via `grep -rln 'pub struct WarEngine\|pub struct Battlefield\|pub struct Waypoint\b\|pub struct TraceRecord\|pub struct Aegis\b' crates/`) — this page's own stated scope, "describes all domain entities in `paladin-ai-core`", is violated by the omission; the `GarrisonEntry` snippet (lines 96-104) is separately recorded as stale above (§2 Vocabulary sweep / Phase 31 D-29 subsection — cross-referenced, not duplicated); class 9: `grep -nFf 34-shipped-tokens.txt` → hits on `Commissary` (line 30) only, none of `WarEngine`/`Waypoint`/`Aegis`/`TraceRecord` | Phase 22 — `Waypoint`, a full `Battlefield` snapshot persisted automatically after every superstep (ENG-03, SS-02) | MB-11 | L |
| 48 | docs/src/architecture/hexagonal-design.md | **stale** | class 1 (version strings): none on this page; class 4 (module/source paths): the `LlmPort` trait code sample (lines 32-47) shows `async fn generate(&self, messages: &[Message], config: &LlmConfig) -> Result<LlmResponse, LlmError>` — live `crates/paladin-ports/src/output/llm_port.rs:1287-1304` is `async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>`, a single builder parameter, not `(messages, config)` — **mismatch**, confirmed by direct read of the trait definition and its own doctest at the same location; `GarrisonPort`/`SanctumPort`/`ArsenalPort`/`FileStoragePort` samples (lines 52-93) not independently re-verified this pass (out of scope for the confirmed defect above); class 9: `grep -nFf 34-shipped-tokens.txt` → 0 hits (pre-v0.10.0 API, D-00g) | pre-v0.10.0 API drift (D-00g: shipped tree outranks any document; the `LlmRequest` builder predates this milestone, no Phase 22-33 REQ-ID applies) | MB-10 | M |
| 49 | docs/src/architecture/overview.md | **stale** | class 3 (crate names): line 3 "nine focused crates", table lines 15-24 lists 9 infra/core crates + root — live `ls crates/` → 11 library crates + `doc-examples` + root facade; `paladin-eval` (Phase 28) and `paladin-herald` both absent from the table — **mismatch, first finding, ID in this row's MB ID(s) cell**; class 4 (module/source paths): the page never names `crates/paladin-eval` or `crates/paladin-herald` (`test -d` confirms both exist); class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits — the page, as the primary architecture entry point, never names `WarEngine`, `Battlefield`, `Waypoint`, `Aegis`, `VaultPort`, `Commissary`, `TraceRecord`, or any Platform API route despite each shipping in Phases 22-28/30/33 — checked, confirmed absent by the same grep that finds them on `commissary.md`/`platform-api.md`; direct check: "Technology Stack" table (line 240) correctly states "MSRV 1.88" (matches `rust-toolchain.toml`), so the page was partially touched post-Phase-22.1 but never for the architectural additions — **mismatch, second finding, ID in this row's MB ID(s) cell** | Phase 28 — `paladin-eval` crate (OBS-04, SS-58); Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) | MB-08, MB-09 | M, L |
| 50 | docs/src/contributing/architecture-decisions.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 51 | docs/src/contributing/branching-model.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 52 | docs/src/contributing/contributing-providers.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 53 | docs/src/contributing/development-setup.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 54 | docs/src/contributing/testing-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 55 | docs/src/deployment-topologies/battalion-orchestration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 56 | docs/src/deployment-topologies/embedded-library.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 57 | docs/src/deployment-topologies/http-service-host.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 58 | docs/src/deployment-topologies/overview.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 59 | docs/src/deployment-topologies/queue-worker.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 60 | docs/src/deployment-topologies/sidecar.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 61 | docs/src/deployment/cicd.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 62 | docs/src/deployment/docker.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 63 | docs/src/deployment/kubernetes.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 64 | docs/src/deployment/production.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 65 | docs/src/getting-started/configuration.md | current | class 8 (feature flags): all nine LLM providers documented with base URLs and env vars (`openai`/`anthropic`/`deepseek`/`kimi`/`qwen`/`grok`/`ollama`/`gemini`/`openai-compatible`), matching the live `Cargo.toml` `llm-*` flag set exactly; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → hits on `reasoning_agent`, `max_tokens` (the four-meanings table, matching Phase 30 VOCAB-05/SS-70 exactly), `Commissary` (Phase 33 RAG cap note, lines 283-284, 416) — all correct and current; class 4 (module/source paths): `crates/paladin-llm/src/openai/adapter.rs`, `crates/paladin-llm/src/anthropic/adapter.rs`, `src/application/services/paladin/middleware/limits.rs` (line 417-418) all confirmed present (`test -f`) | — | — | — |
| 66 | docs/src/getting-started/installation.md | **stale** | class 1 (version strings): line 12 "Rust \| 1.85.0 \| Latest stable (1.95+)", line 17 "Why Rust >= 1.85?", line 19 "should print >= 1.85.0" — live `rust-toolchain.toml`/`Cargo.toml` `rust-version = "1.88"` (Phase 22.1, X-11.2) — **mismatch**; lines 48-78 pin every crate to `"0.5.0"` — live `0.10.0` — **mismatch**; class 8 (feature flags): the "Feature Flag Profiles" table (lines 84-90) lists 5 flags (`llm-openai`, `redis-queue`, `s3-storage`, `openai-embeddings`, `qdrant`) — live `Cargo.toml` `[features]` has 20+ flags including `llm-kimi`/`llm-qwen`/`llm-grok`/`llm-ollama`/`llm-gemini`/`llm-openai-compatible`/`vision`/`content-processing`/`web-server`/`notifications`/`storage-postgres`/`redis-cache`/`otel`/`dev-ui` — **15+ shipped flags undocumented**; class 3: `paladin-ai-core` crate name itself is correctly spelled (matches `crates/paladin-core/Cargo.toml` `name = "paladin-ai-core"`); class 9: `grep -nFf 34-shipped-tokens.txt` → 1 hit (the stale `"llm-openai"` feature string, line 57) | Phase 22.1 — Workspace MSRV floor raised from 1.85 to 1.88 (X-11.2, SS-08) | MB-06 | L |
| 67 | docs/src/getting-started/quickstart.md | **stale** | class 1 (version strings): lines 24-26 pin `paladin-ai`/`paladin-ports`/`paladin-llm` to `"0.7.0"` — live workspace version `0.10.0` (`Cargo.toml` `[workspace.package] version`) — **mismatch**; direct check: the code sample's `PaladinExecutionService::new(llm_port, circuit_breaker, None, None)` call (line 61) matches the live 4-arg constructor exactly (`src/application/services/paladin/paladin_execution_service.rs:361-365`) — **current**; `result.usage.total_tokens` (line 67) matches the live `TokenUsage.total_tokens: u32` public field (`crates/paladin-core/src/platform/container/token_usage.rs:28`) — **current**; the "Understanding the Output" table (lines 114-120) correctly names `usage: TokenUsage` (Phase 31, ACCT-01/SS-73); class 4 (module/source paths): `src/main.rs` reference (lines 32, 35) is a generic illustrative path, not a repo-relative claim — not applicable; class 9: `grep -nFf 34-shipped-tokens.txt` → hits on the stale version pin plus the correct `TokenUsage` row | Phase 22-33 (v0.10.0 milestone) — workspace crate version `0.10.0` (`Cargo.toml [workspace.package]`); no matching §1 row exists for a bare version-pin fact, so this citation is the workspace-wide version bump itself, not a single SS-nn item | MB-07 | S |
| 68 | docs/src/introduction.md | **stale** | class 1 (version strings): none on this page; class 3 (crate names): none relevant to the term table; the "Medieval Military Theme" table (lines 78-91) carries 12 terms — `grep -rlni 'medieval military' docs/src/` (34-02's D-10 sweep) already found this a genuine fourth, partial vocabulary list, missing `Commissary`, `Sanctum`, `Sentinel`, `Quest`, `Conclave`, `Council`, `Grove` and `Commander` against the three confirmed-current lists (`.github/copilot-instructions.md`, `PROJECT.md`, `domain-model.md`) — **mismatch, first finding, ID in this row's MB ID(s) cell**; class 9 (shipped-surface): `grep -nFf 34-shipped-tokens.txt` → 0 hits — the page's five nav-index sections (User Guides, Architecture, Deployment, Operations, Contributing) never link `control-flow.md`, `fault-tolerance.md`, `parley-and-chronicle.md`, `agent-runtime.md`, `platform-api.md`, `observability.md`, `eval-harness.md`, `commissary.md` or any `deployment-topologies/` page — checked, confirmed absent from every "## " section's link list — **mismatch, second finding, ID in this row's MB ID(s) cell**; the Architecture-Layers three-layer description (lines 93-101) remains accurate and unchanged | Phase 30 — `Commissary` anchored (VOCAB-02, SS-68); Phase 22 — `WarEngine` executes cyclic graphs in supersteps (ENG-02, SS-01) | MB-04, MB-05 | M, L |
| 69 | docs/src/operations/logging.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 70 | docs/src/operations/monitoring.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 71 | docs/src/operations/observability.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 72 | docs/src/operations/performance-tuning.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 73 | docs/src/operations/troubleshooting.md | pending | pending — not yet swept (no signal class run) | — | — | — |
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
re-deriving the location, citing back to the row worked above.

## §4 Examples table

Rows land in plan 34-08 (the four `ci.yml:548-558` build invocations, the `doc-examples` gate, and
`live_vendor_smoke`). One row is fully worked here — `examples/README.md` audited as a page in its
own right (D-17) — the second CONTEXT.md-named method self-test.

| EX ID | Program / module | Build invocation | Build status | Currency verdict | Obsolete-API hits | Claimed capability → tree check | Evidence anchor | Size |
|-------|-------------------|-------------------|---------------|-------------------|--------------------|-----------------------------------|------------------|------|
| EX-01 | examples/README.md | n/a — documentation page, not a compiled program | n/a | stale | none (not an API-obsolescence finding) | Line 24 states "Rust 1.70 or later" as the minimum Rust version; `Cargo.toml` `[workspace.package] rust-version = "1.88"` (line 18) is the measured, live MSRV floor — the two disagree | 34-EVIDENCE.md #8 | S |

## §5 Phase 35 work list

Empty. Assembled by plan 34-09 from every `MB-nn` row in §2 once plans 34-03/34-04/34-05 have
swept all 93 pages, ordered per D-21 (blocking `L` items first, then by page order).

## §6 Phase 36 work list

Empty. Assembled by plan 34-09 from every `RD-nn` row in §3 and every `EX-nn` row in §4 once
plans 34-06/34-07/34-08 have completed their sweeps, ordered per D-21.

## §7 Deferred routing

Empty. Assembled by plan 34-09 from `deferred-items.md` — pointers only, per D-19 ("nothing is
absorbed into 35 or 36 by convenience").

---

*Phase: 34-documentation-currency-audit*
*Plan 34-01 wrote the header, all seven section headings, the 92 seeded §2 rows, and the one
worked row in each of §2/§3/§4. Every later plan in this phase appends rows; none renumbers or
removes a row already present here (D-03).*
