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
renumbered, because Phases 35 and 36 close items by ID (their SUMMARY/VERIFICATION cite
"MB-07 closed by commit X"). The first mdBook row worked in §2 below fixes the numbering origin.

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

| # | Page | Verdict | Findings (signal class → cmd → result) | Cites (Phase N — item (REQ)) | MB ID(s) | Size |
|---|------|---------|------------------------------------------|-------------------------------|----------|------|
| 1 | docs/src/SUMMARY.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 2 | docs/src/api-reference/crate-map.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 3 | docs/src/api-reference/feature-flags.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 4 | docs/src/api-reference/migration-guide.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 5 | docs/src/api-reference/platform-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 6 | docs/src/api-reference/stable-api.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 7 | docs/src/api-reference/upgrading.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 8 | docs/src/api-reference/wargraph-doc-schema.md | pending | pending — not yet swept (no signal class run) | — | — | — |
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
| 44 | docs/src/architecture/commissary.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 45 | docs/src/architecture/crate-map.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 46 | docs/src/architecture/design-patterns.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 47 | docs/src/architecture/domain-model.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 48 | docs/src/architecture/hexagonal-design.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 49 | docs/src/architecture/overview.md | pending | pending — not yet swept (no signal class run) | — | — | — |
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
| 65 | docs/src/getting-started/configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 66 | docs/src/getting-started/installation.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 67 | docs/src/getting-started/quickstart.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 68 | docs/src/introduction.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 69 | docs/src/operations/logging.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 70 | docs/src/operations/monitoring.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 71 | docs/src/operations/observability.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 72 | docs/src/operations/performance-tuning.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 73 | docs/src/operations/troubleshooting.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 74 | docs/src/user-guides/agent-orchestrator-bridge.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 75 | docs/src/user-guides/agent-runtime.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 76 | docs/src/user-guides/arsenal-tools.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 77 | docs/src/user-guides/battalion-patterns.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 78 | docs/src/user-guides/content-processing.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 79 | docs/src/user-guides/control-flow.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 80 | docs/src/user-guides/eval-harness.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 81 | docs/src/user-guides/fault-tolerance.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 82 | docs/src/user-guides/garrison-memory.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 83 | docs/src/user-guides/graph-visualization.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 84 | docs/src/user-guides/herald-output.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 85 | docs/src/user-guides/maneuver-flow-dsl.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 86 | docs/src/user-guides/memory-management.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 87 | docs/src/user-guides/orchestration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 88 | docs/src/user-guides/output-formatting.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 89 | docs/src/user-guides/paladin-agents.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 90 | docs/src/user-guides/paladin-configuration.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 91 | docs/src/user-guides/parley-and-chronicle.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 92 | docs/src/user-guides/sanctum-vector-memory.md | pending | pending — not yet swept (no signal class run) | — | — | — |
| 93 | docs/src/user-guides/tool-integration.md | pending | pending — not yet swept (no signal class run) | — | — | — |

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
