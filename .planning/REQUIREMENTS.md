# Requirements: Paladin — Milestone v0.10.0 "Durable Agent Execution Runtime"

**Defined:** 2026-09-01
**Core Value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.

**Source of truth:** the approved design corpus in `.project/v0.10.0/` (program overview `00`,
epic PRDs `01`-`07`, traceability matrix `08`). Each requirement below is a capability cluster
over named PRD functional requirements (FR ranges cited per item); **the PRDs remain the FR-level
behavior source of truth** — a phase executing a cluster implements every FR in its cited range,
plus the cross-cutting rules X-01…X-11 from `00-program-overview.md` §3, which apply to every
requirement without being restated per item. Every epic's per-item versioning gate (X-10/X-11:
register touched public types in `MIGRATION.md` §9.2, keep `cargo semver-checks` and the MSRV job
green, record new deps/migrations/config in §9.3-9.5) is part of each requirement's definition of
done.

**Scope-time conflict record:** PRD 05 §1/§2.5's premise that provider coverage is
"OpenAI/Anthropic/DeepSeek only" is stale — the OpenAI-compatible generic adapter, Gemini adapter
and Ollama path shipped in v0.8.0 (PROV-01…04). Under this project's precedence order (shipped
tree outranks PRD), RT-06 is scoped as conformance verification and gap-closure, not greenfield.

**Extension record (2026-09-14):** Phases 30-33 "Token Economy" were added to this milestone
before the `0.10.0` tag was cut, sourced from `.project/Milestone_13-Token-Economy/` (overview +
Epics 1-4; handoff corpus authored from the downstream Web3 Security Paladin repo's token-economy
systems analysis, findings F1-F8 / decisions D-1…D-9). Four new prefixes: `VOCAB-*`, `ACCT-*`,
`PRIM-*`, `COMM-*`. **Scope-time conflicts, resolved:** (a) those PRDs target `v0.11.0`; the
operator's instruction is that this work ships in **v0.10.0** (possible because the bump landed
untagged, Phase 29 D-18/D-21), so COMM-04 re-seals the Phase 29 release gates; (b) the PRDs'
clean-break policy (overview §5.1) **supersedes X-03** ("removals are not allowed before v0.11.0")
for Phases 31-33 only — VOCAB-07 records that supersession as an ADR; (c) Epic 2's "keep a
`token_count` deprecation shim" goals bullet is overridden by its own R3 (no shim, ACCT-02), and
Epic 4 R3 is folded into PRIM-04 rather than duplicated. **The Treasurer (Milestone 14,
`.project/Milestone_14-Treasurer/`) is reserved by VOCAB-04 and not roadmapped** — FUT-08 and
FUT-09 remain v2, now with a named owner milestone.

## v1 Requirements

### Battlefield State & Superstep Engine (Doc 01, epic `ENG`)

- [x] **ENG-01**: A developer can declare a `BattlefieldSchema` and nodes exchange typed
  `StateDelta`s instead of bare strings — per-field dispatch rules (`LastWrite`, `Append`,
  `MergeObject`, `Sum`, `Custom`), typed accessors, schema enforcement (unknown-field and
  missing-required hard errors), and structured `BattlefieldError` variants, in `paladin-core`
  with no new core dependencies (PRD 01 §3.1-3.2; ENG-FR-07…10)

- [x] **ENG-02**: The `WarEngine` executes cyclic graphs (self-loops included) in supersteps with
  bounded iteration (`max_supersteps`, `max_node_visits`, typed limit errors), deterministic
  frontier and merge order (byte-identical Battlefields over ≥20 randomized-scheduling
  iterations), same-superstep snapshot isolation, and precise join/defer semantics that never
  deadlock on a not-firing branch (ENG-FR-01…06)

- [x] **ENG-03**: Exactly one Waypoint is persisted automatically after every superstep, addressed
  by `(thread_id, waypoint_id)` with parent lineage and a stable graph fingerprint; write failure
  fails the run under the default `Strict` durability (documented `BestEffort` downgrade
  available) (ENG-FR-11, ENG-FR-13, ENG-FR-14; §3.3-3.4)

- [x] **ENG-04**: `resume(graph, thread)` restores Battlefield, Vanguard and per-node visit counts
  from the latest Waypoint and continues with zero re-execution of completed nodes — proven by
  program scenario E2E-1 (crash after superstep 3, fresh engine, final state equals uninterrupted
  control run, one Waypoint per superstep) (ENG-FR-12; overview §6 E2E-1) *(BUG-04, found 2026-09-03 by code reading: resume rebuilt the Frontier from scratch, so a pre-crash fired edge into a not-yet-ready join was lost; re-opened for Phase 22.1, fixed by persisting the frontier on the Waypoint — ENG-FR-12a.)*

- [x] **ENG-05**: Three `WaypointPort` backends — InMemory, SQLite (with migrations), Postgres
  (new `postgres` feature) — all pass one shared contract test suite; a
  `WaypointRetentionConfig` cleanup routine never deletes a thread's latest Waypoint or any
  `AwaitingInput` Waypoint (ENG-FR-15…18)

- [x] **ENG-06**: Legacy string-based execution is bridged, not broken:
  `from_formation`/`from_phalanx`/`from_campaign` constructors reproduce today's data flow with
  golden output-equivalence tests, and the legacy execution services keep byte-identical public
  behavior (sole sanctioned exception: BUG-01, owned by CF-01) (ENG-FR-19, ENG-FR-20; X-03)

- [x] **ENG-07**: The engine ships the seams later epics consume — a non-interfering `TraceSink`
  hook (bounded channel, drop-oldest, counted drops), an ordered `NodeInterceptor` chain, and a
  `CancellationToken` that finishes the in-flight superstep and persists a `Halted` Waypoint
  (ENG-FR-21…23; PRD 01 §8)

- [x] **ENG-08**: Program scaffolding mandated for the first epic: `MIGRATION.md` exists at the
  repository root with the §9 skeleton and pre-populated M-B-01…03 / §9.2 register entries, and
  CI gains the `cargo semver-checks` job (vs the published v0.9.0 crates, per-item allowlist
  only) and the MSRV job (Rust 1.85 toolchain, full workspace, `--all-features`), both running
  on every PR (overview §9, X-10.5, X-11.1)

### Control Flow (Doc 02, epic `CF`)

- [x] **CF-01**: BUG-01 is fixed fail-closed and test-first: custom edge conditions resolve
  through a registered-`EdgeConditionEvaluator` mechanism on both `CampaignExecutionService` and
  the WarEngine; validation fails with `BattalionError::InvalidGraph` naming every unregistered
  `Custom(name)` **before any node executes**; the warn-and-return-true branch is removed with no
  restoring configuration; runtime evaluator errors fail the run rather than defaulting a branch
  (CF-FR-01…04; overview §7, M-B-01)

- [x] **CF-02**: Nodes steer routing by returning a `Directive` — `NextStep::{Edges, Goto, End,
  Muster, Parley}` with Goto target validation, documented-and-tested End-over-Goto precedence,
  and a configurable `DirectiveParser` for Paladin nodes (`PlainOutput` backward-compatible
  default; `StructuredDirective` JSON envelope with `on_parse_error` modes) (CF-FR-05…08)

- [x] **CF-03**: Muster dynamic fan-out works as map-reduce: a Directive spawns runtime-N worker
  tasks in one superstep with payload isolation, deterministic `task_key`-ordered aggregation
  (repeat-tested), duplicate-key rejection, `max_muster_tasks` limit, and mid-muster resume that
  re-runs only unfinished tasks (CF-FR-09…13)

- [x] **CF-04**: Battalions nest — `NodeSpec::Battalion` embeds a child WarGraph with `StateMap`
  input/output mapping and private child fields, namespaced checkpoint inheritance with
  resume-mid-child, recursive-embedding rejection at validation, and legacy patterns embeddable
  (Formation-inside-Campaign integration test) (CF-FR-14…17)

- [x] **CF-05**: LLM-evaluated routing is available and off by default: an `LlmDecision` edge
  evaluator (choice matching, `on_ambiguous` modes, application-layer registration) and
  `Commander` `StrategySelection::Semantic` that falls back to Heuristic on any LLM error with
  the fallback recorded — existing Commander tests pass unmodified (CF-FR-18, CF-FR-19)

### Pause/Resume, History, Shutdown (Doc 03, epic `HITL`)

- [x] **HITL-01**: A workflow can pause indefinitely without holding compute: nodes (and a
  first-class `Gate` node with Battlefield templating) raise `ParleyRequest`s; suspension merges
  peer deltas, persists an `AwaitingInput` Waypoint carrying **all** of the superstep's parleys,
  releases every resource, and survives full process termination (resumable from a different
  process sharing the backend — integration-tested); partially-answered suspension is queryable
  (HITL-FR-01…03)

- [x] **HITL-02**: `resume_with(graph, thread, responses)` validates responses per kind
  (Approval/Choice/FreeText/StateEdit) with typed errors that leave the thread suspended, honors
  `expires_at` with an `on_expire` policy, and delivers values to the paused node's continuation
  — proven by program scenario E2E-2 (approval gate, both branches, across process
  drop/recreate) (HITL-FR-04…06; overview §6 E2E-2)

- [x] **HITL-03**: The Chronicle is inspectable and forkable: history/inspect over `WaypointPort`,
  `replay` and `fork`-with-edit create new chains with `fork_of` lineage while the original
  chain stays byte-identical (immutability hard invariant), branch-aware latest resolution, and
  defined subgraph-fork semantics (HITL-FR-07…12)

- [x] **HITL-04**: Graceful shutdown loses no work: cancellation finishes the in-flight superstep
  within `shutdown_grace` (default 30 s; over-grace nodes recorded `Skipped` and re-listed in
  the vanguard), `resume` continues a `Halted` thread, and the facade wires SIGTERM/SIGINT to
  all in-flight runs with `k8s/` manifests and docs updated and a documented disable switch —
  registered as `MIGRATION.md` M-B-02 (HITL-FR-13…15)

- [x] **HITL-05**: Engine-backed threads are reachable over HTTP: `GET /threads/{id}/state`,
  `POST /threads/{id}/resume` (409/400/404 semantics), `GET /threads/{id}/history` (paginated),
  following existing utoipa + error-envelope conventions with `openapi.json` regenerated
  (HITL-FR-16)

### Node-Level Fault Tolerance (Doc 04, epic `FT`)

- [x] **FT-01**: Errors carry machine-usable transience: `transience()` on `PaladinError` and
  `LlmError` with per-variant table-driven tests, provider adapters gaining status-carrying
  variants (no string parsing), a structured `NodeError` carried through engine execution, and
  `BattalionError::Node(NodeError)` — all three touched pre-existing public enums handled per
  X-10 (`#[non_exhaustive]` or justified deliberate-breaking) and registered in `MIGRATION.md`
  §9.2 (FT-FR-01, FT-FR-02, FT-FR-02a)

- [x] **FT-02**: Per-node Aegis retry works and is provable: exact backoff sequence under paused
  clock (jitter bounds asserted), transience-predicate gating (Permanent → 1 attempt),
  attempt isolation (failed-attempt deltas discarded, `AttemptRecord` history kept), per-task
  retries inside a Muster, and retries-within-a-superstep Waypoint semantics
  (FT-FR-03…07; PRD 04 §2.1)

- [x] **FT-03**: Per-attempt timeouts distinguish stalled from slow: wall-clock `run_timeout` and
  progress-aware `idle_timeout` (stream chunks, trace events, `ctx.heartbeat()`), nested with
  engine/Battalion bounds so the tightest fires and the error names which (FT-FR-08…10)

- [x] **FT-04**: Typed error handlers enable compensation: `Route` (structured NodeError into a
  declared state field, recovery node into the vanguard, run not failed), `Absorb` (fallback
  delta, continue), registered `Custom` handlers (fail-closed on unregistered names, may Parley),
  no-handler exhaustion failing with a structured — never stringified — error, and handler loops
  bounded by `max_node_visits`; program scenario E2E-3 passes together with CF-03
  (FT-FR-11…15; overview §6 E2E-3)

- [x] **FT-05**: `FallbackLlmAdapter` fails over across a provider chain on Transient/Unknown
  errors only (Permanent short-circuits), propagates mid-stream errors without silent provider
  switch, returns `LlmError::AllProvidersFailed` with per-hop attempts, and records the serving
  provider on `PaladinResult` under X-10.3 (`Default` preserved, `#[non_exhaustive]`, §9.2
  register entry) (FT-FR-16, FT-FR-17)

- [x] **FT-06**: Expensive deterministic nodes can cache: `CachePolicy` keyed by default on node
  id + resolved input + Paladin config fingerprint (prompt/model change invalidates naturally),
  `NodeCachePort` with InMemory and Redis adapters sharing a contract suite, hits merging the
  stored delta with `cache_hit: true` and no execution, failures never cached, Append-dispatch
  replay hazard documented with a schema-level `cache: Deny` marker (FT-FR-18…20)

### Agent Runtime Enhancements (Doc 05, epic `RT`)

- [x] **RT-01**: `PaladinExecutionService` gains an ordered `ExecutionMiddleware` chain
  (before/after model, around tool) with onion ordering and short-circuit semantics
  (order-asserted by test), per-run state isolation under concurrency, and the same chain
  applying when a Paladin runs as an engine node — with the NodeInterceptor-vs-middleware
  two-layer distinction documented (RT-FR-01…03)

- [x] **RT-02**: Built-in middleware ships, each config-structured per X-09: `ModelCallLimit` and
  `TokenBudget` finishing with new `StopReason::CallLimit`/`TokenBudget` variants (X-10 decision
  applied to both in the same change and registered in §9.2), `ToolCallLimit` denying without
  failing the run, `Guardrail` prompt/response screens with Fail/Redact/Finish actions, and
  retry/fallback middleware that delegates to the FT-05 implementations without duplicating
  logic (RT-FR-04…07, RT-FR-09)

- [x] **RT-03**: Long conversations fit the context window: `TokenCounterPort` (heuristic default,
  provider adapters where possible; no inline heuristics), a stable never-splits-a-message
  `HistoryTrimmer`, and compounding `SummarizationMiddleware` persisting summaries to Garrison
  flagged `is_summary: true` — the Garrison entry field `#[serde(default)]`, the SQLite column
  additive-migrated and §9.2/§9.4-registered — degrading to trimming on summarizer failure,
  never failing the run (RT-FR-08, RT-FR-10…12)

- [x] **RT-04**: Agents get confined cross-session memory: `VaultPort` (put/get/delete/list/
  search) with InMemory, SQLite and semantic (Sanctum/Qdrant-composed, `qdrant` feature)
  adapters under a shared contract suite; `vault_get`/`vault_put` Armaments confined to a
  host-granted namespace subtree (`NamespaceDenied` on traversal, attack-tested);
  `NodeContext::vault()`; opt-in `VaultRecallMiddleware` injecting top-k results — and the
  Garrison/Waypoint/Vault three-way distinction documented (RT-FR-13…16)

- [x] **RT-05**: Structured output is first-class: `execute_structured<T>` on a new
  `StructuredExecutorPort` (not `PaladinPort`), schemars-generated schema in the application
  layer (MSRV-verified per X-11), native provider JSON modes via an additive `response_format`
  request field (X-10.3 handled and registered), a bounded repair loop with typed
  `StructuredOutputInvalid` exhaustion preserving raw output, and engine nodes with
  `output_schema` writing parsed JSON to their `output_field` — reused by `StructuredDirective`
  (RT-FR-17…19)

- [x] **RT-06**: Provider conformance close-out (verify-then-fix, not greenfield — see scope-time
  conflict record): the shipped v0.8.0 OpenAI-compatible, Gemini and Ollama paths are measured
  against PRD 05's bar — shared conformance suite across adapters, FT-01 transience-correct
  429/5xx mapping, mock-server streaming coverage, documented Ollama recipe with an env-gated
  integration test — and only measured gaps are closed (RT-FR-20…22)

- [x] **RT-07**: A tool-loop agent is a one-liner: `reasoning_agent(llm, tools, opts)` preset with
  a ≤15-line doc-tested example, and tool failures fed back into the model context by default
  (sanitized; `tool_error_mode: FeedToModel | FailRun` with per-tool override) — the chosen
  default and rationale recorded as `MIGRATION.md` M-B-03 (RT-FR-23, RT-FR-24)

### Platform API (Doc 06, epic `PLAT`)

- [x] **PLAT-01**: Run submission is decoupled from execution: `POST /runs` returns 202 within
  250 ms p99 (enqueue only), a `RunRepositoryPort` (SQLite + Postgres, contract suite) persists
  every status transition, and the status machine is monotonic with typed illegal-transition
  errors (PLAT-FR-01; PRD 06 §2.1)

- [x] **PLAT-02**: A worker pool executes runs durably: `RunQueuePort` (InMemory + Redis
  adapters, shared contract suite including lease-expiry redelivery), lease heartbeats at
  ≤ lease/3, at-least-once redelivery that **resumes** the thread rather than restarting
  (kill-mid-run test), cross-instance cancellation via a persisted flag + `CancellationProbe`
  observed at superstep boundaries, and the one-active-run-per-thread `409 ThreadBusy` invariant
  holding under 10 concurrent submits (PLAT-FR-02…05; PRD 06 §2.2)

- [x] **PLAT-03**: Parley and streaming integrate with runs: `AwaitingInput` releases the worker,
  `POST /threads/{id}/resume` validates then re-enqueues under the same `run_id` (attempt++),
  and `GET /runs/{id}/stream` bridges live TraceSink events to SSE with a documented
  polling-backed degraded mode (terminal events always eventually delivered) and 15 s heartbeats
  (PLAT-FR-06, PLAT-FR-07)

- [x] **PLAT-04**: Assistants are named, versioned configurations: append-only immutable versions
  (no PUT, ever), `latest` frozen at submit time for each run, full publish-time validation with
  machine-readable violations, creator/timestamp/note audit trail, the code-registered registry
  exposed read-only as synthetic entries, and `WarGraphDoc` with documented JSON Schema,
  registry-resolving `compile()`, and restart-stable fingerprint round-trip (PLAT-FR-08…12;
  PRD 06 §2.3)

- [x] **PLAT-05**: Schedules and webhooks are API-managed: cron (5-field + optional seconds, UTC,
  thread strategies, skip/catch-up policies) surviving restart without duplicate or missed-then-
  double firing; webhook delivery on terminal + `AwaitingInput` events with HMAC
  `X-Paladin-Signature`, 5-attempt bounded retry on 5xx/timeout only, persisted queryable
  delivery attempts, async off the completion path; and an SSRF guard rejecting non-http(s),
  loopback, link-local, private and metadata targets at write and send time unless explicitly
  allowlisted (PLAT-FR-13…15; PRD 06 §2.4)

- [x] **PLAT-06**: The new API surface is production-shaped: existing auth + rate limiting on
  every new endpoint, admin/writer scopes on mutating routes, pagination everywhere (limit
  ≤ 100, opaque cursor), `openapi.json` regenerated and diff-reviewed, and a CI job generating
  Python + TypeScript clients from the spec and smoke-testing them against a test server
  (PLAT-FR-16, PLAT-FR-17)

### Observability & Tooling (Doc 07, epic `OBS`)

- [x] **OBS-01**: Every run has a machine-consumable account: the authoritative serde `TraceEvent`
  enum with per-run monotonic `seq` (causal-order guarantee, gapless-or-counted-drops test),
  bounded-payload field changes (never full state values by default), and a `TraceSinkPort`
  whose slow/panicking implementations cannot stall or fail a run (bounded channel, drop-oldest,
  counted; catch_unwind; never awaited), with `CompositeSink` fan-out (OBS-FR-01…03)

- [x] **OBS-02**: Traces reach real consumers: a default-on structured-log sink; an `otel`-gated
  OpenTelemetry exporter with span-per-attempt trees verified against a collector stub; the SSE
  bridge for `GET /runs/{id}/stream` as a TraceSink adapter (one pathway, two consumers); and
  opt-in `run_traces` persistence upgrading post-hoc stream replay to full fidelity, sharing
  ENG-05 retention (OBS-FR-04…07)

- [x] **OBS-03**: Graphs and runs are visualizable: golden-tested `WarGraphDoc → Mermaid/DOT`
  exporters, an execution-overlay export annotating outcomes/visit counts/fired edges/durations,
  `paladin-cli graph export` and `run export` commands, and a minimal auth-gated `dev-ui`
  inspector page from which a human can answer "which branch fired and why did node X run 3
  times" on the fixture run (OBS-FR-08…10)

- [x] **OBS-04**: Agent behavior is regression-testable: the new `paladin-eval` crate with a
  scenario file format (scripted mock LLM behavior), an assertion library over the trace record

  + final Battlefield, a `cargo test`-integrable runner macro and `paladin-cli eval run` with
  `--repeat`/`--bless`, a gated live-model mode, and the three program E2E fixtures dogfooded as
  eval scenarios (OBS-FR-11…15)

### Program Gates & Release (overview §5/§9, doc 08, epic `SHIP`)

- [x] **SHIP-01**: `MIGRATION.md` is complete: every §9 section filled with no "TBD" — M-B-01…03
  resolved with chosen defaults and worked examples, the §9.2 register matching the
  `cargo semver-checks` allowlist exactly, §9.3 toolchain/deps, §9.4 schema migrations with the
  Citadel-files-unchanged statement, §9.5 config/env with the disabled-by-default claim, §9.6
  HTTP surface, §9.7 deprecations, §9.8 operator checklist — linked from the README and the
  mdBook "Upgrading" page (overview §9; DoD 4)

- [x] **SHIP-02**: Backward compatibility is proven, not asserted: an integration test boots
  v0.10 with a v0.9 sample config and asserts legacy behavior (all new subsystems disabled by
  default), and a golden diff of `openapi.json` restricted to pre-existing paths is empty
  (overview §9.5, §9.6)

- [x] **SHIP-03**: The program acceptance audit passes: E2E-1/2/3 green as integration tests in
  `tests/`, the doc-08 verification protocol run (every FR has a passing test, no orphan
  behavior, ubiquitous-language names conform), and BUG-01's old warn-and-default-true path
  grep-absent with the fix's failing-then-passing test order visible in history (overview §5-§6;
  doc 08)

- [x] **SHIP-04**: v0.10.0 is releasable: all workspace crates at `0.10.0` with changelogs
  updated, `cargo publish --dry-run` green for every publishable crate in dependency order,
  mdBook + rustdoc updated with no new broken intra-doc links, and the semver and MSRV CI jobs
  green on the release commit (overview §5 DoD 1, 3, 6, 7; X-08)

### Token-Economy Vocabulary & Commissary Anchoring (`.project/Milestone_13-Token-Economy/Epic_1`, epic `VOCAB`)

Docs-only, non-breaking. Source PRD: `prd-vocabulary-and-docs-foundation.md` (D-1, D-2, D-3,
D-8, D-9; F4 partial, F5 docs). Locked by the overview §0: `Commissary` is kept and not renamed;
`TokenBudget`, `TokenCounterPort`, `TokenUsage`, `max_tokens`, `token_budget.*` are not renamed;
`Quartermaster` stays retired; `Paymaster` is rejected; `Treasurer` is the reserved term.

- [x] **VOCAB-01**: The vocabulary rule is written into `PROJECT.md` and
  `docs/src/architecture/domain-model.md` — units/measures (`TokenUsage`, `max_tokens`,
  `max_context_tokens`) and technical ports (`TokenCounterPort`, `LlmPort`, `EmbeddingPort`) keep
  plain names; domain roles, places and events get Medieval-Military names — and `Commissary` is
  in both the ubiquitous-language list and the domain-model table as the input-side, per-call
  window-rationing officer (PRD R1, R2; D-1, D-2)

- [x] **VOCAB-02**: A numbered ADR in `.planning/decisions/` records the `Commissary` design
  (`verify_fits` guard + `dispense` allocator, fail-loud / never-silent), the
  Quartermaster→Commissary rename rationale and the explicit rejected-name list, reconstructed
  from `origin/feature/quartermaster-prompt-budgeting:.planning/decisions/0010-prompt-context-budgeting.md`
  and the port commit history (PRD R3; D-2)

- [x] **VOCAB-03**: An mdBook page for `Commissary` under `docs/src/` (concept, the
  `Consignment`/`Stockpile`/`ShedItem` model, a usage sketch) is linked from the architecture nav
  in `docs/src/SUMMARY.md` with the link-check green (PRD R4; D-2)

- [x] **VOCAB-04**: A one-page `Treasurer` reservation ADR: reserved (0/0 in-tree by grep), will
  own cross-run / per-tenant / per-API-key allowances, per-model currency pricing, `cost_estimate`
  production and rate pacing, installs a per-run `TokenBudget` rather than replacing it, is built
  in Milestone 14 (`.project/Milestone_14-Treasurer/`), and is a framework-only word that must
  never appear as an audit-target or fixture domain term downstream (PRD R5; D-3)

- [x] **VOCAB-05**: `docs/src/getting-started/configuration.md` carries one table naming the four
  `max_tokens` meanings (Garrison store cap, RAG injection cap, per-request completion cap,
  run-level `token_budget` cap) and states any future Treasurer cap uses a distinct `allowance`
  key; the rustdoc on `ExecutionMetadata.cost_estimate` (`paladin-core` `herald.rs`) says
  "reserved for the Treasurer (Milestone 14 / FUT-08); no in-tree producer yet" and the field is
  not removed (PRD R6, R7; D-8, D-9; F5, F7)

- [x] **VOCAB-06**: `grep -rniE '\bQuartermaster\b' crates src` returns nothing — the
  `src/lib.rs` provenance comment is reworded without the retired term — and the
  `SirQuartermaster` example in `.project/project-management/paladin-project-plan-final.md` is
  annotated as historical; `.planning/` phase history is untouched (PRD R8)

- [x] **VOCAB-07**: A token-economy versioning ADR records that Phases 31-33 land as clean breaks
  inside the untagged v0.10.0, superseding X-03 for those phases on the operator's 2026-09-14
  decision (single coordinated downstream consumer, pre-1.0), with every break still registered
  in `MIGRATION.md` §9.2 and the semver-checks allowlist as documentation for the downstream
  refactor rather than a shim; the supersession is recorded in `PROJECT.md` Key Decisions
  (roadmap-time addition; overview §5.1; Roadmap Extension Protocol item 4)

### Lossless Token Accounting (`.project/Milestone_13-Token-Economy/Epic_2`, epic `ACCT`)

Keystone; **breaking** (clean break, no shims). Source PRD: `prd-lossless-token-accounting.md`
(D-4; F1, F8). Verified anchors: `TokenUsage` at
`crates/paladin-core/src/platform/container/token_usage.rs`; `PaladinResult.token_count: u32` at
`crates/paladin-core/src/platform/container/execution_result.rs`; `TokenUsage::from_total` on the
battalion path at `crates/paladin-battalion/src/formation_service.rs` and `phalanx_service.rs`.

- [x] **ACCT-01**: `TokenUsage` (single definition) gains `cache_read_tokens`,
  `cache_write_tokens` and `reasoning_tokens` as `#[serde(default)]` optionals; the rustdoc
  states whether `total_tokens` includes them; legacy JSON without the fields deserializes via
  defaults and new JSON round-trips (PRD R2)

- [x] **ACCT-02**: `PaladinResult`, `BattalionResult.per_paladin_tokens`, the Waypoint
  `NodeExecutionRecord`, `TraceEvent::NodeFinished` and `RunFinished` carry a full `TokenUsage`
  rather than a bare count; `TokenUsage::from_total` is removed from the battalion aggregation
  path; a round-trip test proves a usage with non-zero prompt AND completion (plus cache/reasoning)
  reaches `RunFinished` intact and a battalion test proves `per_paladin_tokens` preserves the
  split; no `#[deprecated]` bare-count accessor is added for downstream compatibility (PRD R1, R3)

- [x] **ACCT-03**: Every LLM adapter's `execute_stream` path is audited; a per-adapter test
  asserts accumulated streaming `TokenUsage` equals the non-streaming path, or the inability is
  documented as an explicit exception in the adapter rustdoc and the mdBook provider page (PRD
  R4; F8)

- [x] **ACCT-04**: The prompt/completion/cache/reasoning breakdown is observable in at least one
  herald in both JSON and Markdown output (PRD R5)

- [x] **ACCT-05**: Every touched public type has a `MIGRATION.md` §9.2 row and a matching
  `cargo semver-checks` allowlist row (Phase 29 D-04 row-level gate green); the `CHANGELOG.md`
  `[0.10.0]` section records the carrier change; `make clean-code` and the 82 % coverage floor
  are green (PRD R6; X-10)

### Unified Token Primitives (`.project/Milestone_13-Token-Economy/Epic_3`, epic `PRIM`)

**Breaking** (clean break, no shims). Source PRD: `prd-unify-token-primitives.md` (D-5, D-6; F2,
F3). Verified anchors: `TokenCounterPort` at
`crates/paladin-ports/src/output/token_counter_port.rs` (`count`, `name`; no `is_exact`);
`Commissary::new` takes `is_exact_counter: bool` at `crates/paladin-llm/src/services/commissary.rs`;
legacy `TokenCounter`/`TokenCounterFactory` re-exported from `paladin-memory` `garrison/mod.rs`,
`prelude.rs` and the facade `src/infrastructure/adapters/garrison/mod.rs`; `HistoryTrimmer::resolve_limit`
at `src/application/services/paladin/middleware/history.rs`.

- [x] **PRIM-01**: `TokenCounterPort` has `fn is_exact(&self) -> bool` defaulting to `false`; the
  tiktoken-backed counter returns `true`, the heuristic returns `false`, each proven by a test
  (PRD R1; D-5)

- [x] **PRIM-02**: `Commissary::new` drops the `is_exact_counter: bool` argument and reads
  exactness from the port — no forwarding constructor — and every in-tree call site compiles
  against the new signature (PRD R2; D-5)

- [x] **PRIM-03**: The legacy `garrison::TokenCounter` trait and `TokenCounterFactory` are
  removed with their three re-exports, every former in-tree caller consuming `TokenCounterPort`;
  if one internal caller genuinely cannot migrate it is `#[deprecated]` with the blocking reason
  recorded in the phase context and removal assigned to Phase 33 (PRD R3; D-5)

- [x] **PRIM-04**: A shared resolver in `paladin-llm` owns the precedence config table →
  provider capabilities → default with an explicit strict mode that errors rather than defaults
  when the window is unknown; both `HistoryTrimmer` and `Commissary` consume it; precedence tests
  cover all four outcomes, and equivalence snapshots prove `Commissary` resolves the same windows
  and `HistoryTrimmer` produces the same trims as before this phase (PRD R4, R5 + Epic 4 R3;
  D-6)

- [x] **PRIM-05**: The `Commissary::new` change and the legacy-counter removal each have a
  `MIGRATION.md` §9.2 row and a semver-checks allowlist row (row-level gate green); the
  `CHANGELOG.md` `[0.10.0]` section records them; `make clean-code` and the coverage floor are
  green (PRD R6; X-10)

### Commissary In-Tree Adoption (`.project/Milestone_13-Token-Economy/Epic_4`, epic `COMM`)

Non-breaking (behavioural change in RAG output, CHANGELOG-noted) plus the release re-seal.
Source PRD: `prd-commissary-in-tree-adoption.md` (D-7; F6, F4 completes). Verified anchor:
`RagRetrievalService::truncate_to_token_budget` at
`crates/paladin-memory/src/services/rag_retrieval_service.rs` (inline `len() / 4`, silent drop —
the Phase 26 D-13 deferral); `Commissary` has no in-tree caller outside `paladin-llm` and the
facade re-export.

- [x] **COMM-01**: RAG truncation goes through `Commissary::dispense` over a `Consignment` built
  from the retrieved memories with priority derived from relevance score and budget
  `rag.max_tokens`; a property test proves the retained total is ≤ the budget and the
  highest-scoring memories are retained (PRD R1; D-7)

- [x] **COMM-02**: The `ShedItem` list is surfaced through the RAG result path and a truncation
  marker is emitted when content was shed; tests assert both present when the budget is exceeded
  and both absent when everything fits (PRD R2; D-7)

- [x] **COMM-03**: An integration test exercises `Commissary::dispense` through the real RAG path
  (the F4 production-caller evidence), and no silent token-based truncation remains in-tree,
  grep-provable (PRD §5, §6; F4, F6)

- [x] **COMM-04**: The Phase 29 release gates are re-sealed on this phase's final commit —
  `MIGRATION.md` no-TBD with §9.2 matching the allowlist row-for-row, `v0_9_config_boot` and the
  OpenAPI golden diff passing, `cargo semver-checks` and MSRV green, `cargo publish --dry-run`
  green in dependency order — and the `CHANGELOG.md` `[0.10.0]` section carries the RAG
  truncation-marker note plus the Phase 31/32 API entries, with evidence appended to the Phase 29
  acceptance audit rather than a new audit (PRD R4; roadmap-time addition; SHIP-01…04
  re-verification)

### Documentation Currency Audit (Release Readiness — Phases 34-36, epic `CURR`)

The prefix also carries Phases 35-36 per ROADMAP ("assigned at planning under the Phase 34
prefix"); those phases mint their own `CURR-nn` numbers when planned.

- [x] **CURR-01**: Every `.md` under `docs/src/` carries a `current`/`stale`/`missing` verdict
  against the Phase 22-33 shipped surface, and every `stale`/`missing` verdict cites the phase and
  the shipped item (ROADMAP Phase 34 SC1; D-05, D-06, D-07, D-08, D-09, D-10)

- [x] **CURR-02**: Every `warning:` line from the default-feature `cargo doc` run and every error
  from the per-crate `-D warnings --all-features` sweep is enumerated with crate, file and line,
  with the `ci.yml` lint-job command quoted verbatim as the bar (ROADMAP Phase 34 SC2; D-00a, D-12,
  D-13, D-14, D-15)

- [x] **CURR-03**: Every program under `examples/`, every `crates/doc-examples` module and
  `crates/paladin-llm/examples/live_vendor_smoke.rs` carries a build status under the CI
  feature-set split plus a currency verdict (ROADMAP Phase 34 SC3; D-16, D-17, D-18)

- [x] **CURR-04**: The inventory is partitioned into sized Phase 35 (`MB-nn`) and Phase 36
  (`RD-nn`, `EX-nn`) work lists, and any finding that is neither documentation nor an example is
  routed to the deferred register (ROADMAP Phase 34 SC4; D-03, D-04, D-19, D-21)

- [x] **CURR-05**: The audit is read-only against the tree: the phase's commits touch only
  `.planning/` (ROADMAP Phase 34 SC5; D-00c, D-22, D-23)

- [x] **CURR-06**: Every item in the Phase 34 mdBook work list is closed by a page edit or a new
  page, and `docs/src/SUMMARY.md` links each new page from the nav position the audit assigned
  (ROADMAP Phase 35 SC1; D-00a, D-01, D-07)

- [x] **CURR-07**: `mdbook build docs/` with the `linkcheck` backend passes with zero broken
  links — the exact `docs.yml` command sequence, including `mdbook-mermaid install`
  (ROADMAP Phase 35 SC2; D-00e)

- [x] **CURR-08**: No touched page names a type, function, config key, route or CLI flag the
  v0.10.0 tree does not export; snippets meant to run are compile-verified in
  `crates/doc-examples`, and illustrative snippets are marked as such
  (ROADMAP Phase 35 SC3; D-11, D-12, D-13, D-14, D-20, D-22)

- [x] **CURR-09**: The book's vocabulary matches the three ubiquitous-language lists: no
  `Quartermaster`, and no bare token total where the prompt / completion split shipped in
  Phase 31 (ROADMAP Phase 35 SC4; D-00d, D-17, D-18, D-21)

- [x] **CURR-10**: `CHANGELOG.md` `[0.10.0]` carries a Documentation entry summarising the pages
  added and corrected (ROADMAP Phase 35 SC5; D-25)

- [x] **CURR-11**: `cargo doc --workspace --no-deps` emits zero `warning:` lines under the exact
  `ci.yml:63` lint-job command and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --all-features --no-deps` exits 0, with the per-crate `-D warnings --all-features` sweep green
  for all thirteen crates (ROADMAP Phase 36 SC1; D-00a, D-01, D-02, D-03, D-04, D-05, D-06, D-07,
  D-08, D-10)

- [x] **CURR-12**: Every one of the 143 `RD-nn` rows in `34-AUDIT.md` §6 is closed at its cited
  crate / file / line under lead-row discipline, and both rustdoc commands plus
  `cargo test --workspace --doc` are wired into `make doc-check`, `make clean-code` and the
  pre-push hook, with the all-features command added to the CI lint job and proven by a real CI
  run (ROADMAP Phase 36 SC2; D-00b, D-09, D-11, D-12, D-13, D-24, D-26, D-27)

- [x] **CURR-13**: `cargo build --examples` passes under each of the four feature-set invocations
  the CI "Example Muster" job splits on — including a dedicated invocation for every new
  `required-features` target — and `cargo test --workspace --doc` is green, run explicitly
  (ROADMAP Phase 36 SC3; D-00f, D-14, D-17, D-23)

- [x] **CURR-14**: Every one of the 64 `EX-nn` work rows in `34-AUDIT.md` §6 is closed — the five
  stale rows are corrected against the shipped API and each of the 59 undemonstrated Phase 22-33
  capabilities has a runnable `examples/*.rs` program with an `examples/README.md` section whose
  **Demonstrates:** line names the capability (ROADMAP Phase 36 SC4; D-00e, D-00g, D-00i, D-00j,
  D-15, D-16, D-18, D-19, D-20, D-21, D-22, D-25, D-29)

- [x] **CURR-15**: `make api-surface` reports no change across every commit in the phase — the
  rustdoc and examples work moves no public surface, and no private item is widened to `pub` and
  no rustdoc lint is suppressed to satisfy a link (ROADMAP Phase 36 SC5; D-00c, D-00d, D-00h,
  D-05, D-28)

## v2 Requirements

Deferred beyond this program (named out of scope by the corpus; tracked, not roadmapped):

### Platform & Tooling

- **FUT-01**: Hand-polished Python/TypeScript SDKs (generated-client CI gate is v1; PLAT §6)
- **FUT-02**: Full graphical IDE / live-editing studio (OBS §5; overview §8)
- **FUT-03**: Multi-region/HA storage replication (backend concern; overview §8)
- **FUT-04**: Billing / usage metering (overview §8)
- **FUT-05**: Multi-tenant orgs / RBAC beyond existing scopes (PLAT §6)

### Runtime

- **FUT-06**: Automatic memory extraction/writing policies for the Vault (explicit tool only in
  v1; RT §5)

- **FUT-07**: LLM-as-judge eval scoring (assertion `custom` leaves the door open; OBS §5)
- **FUT-08**: Per-token cost accounting in currency (token counts only in v1; RT §5)
- **FUT-09**: Provider-level rate-limit pacing and distributed cache-stampede locks (FT §5)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Porting the engine to other languages | Overview §8; generated clients (PLAT-06) are the multi-language surface |
| Removing any existing public API | X-03: deprecations allowed, removals are not (before v0.11.0) |
| Behavioral changes beyond BUG-01, M-B-02, M-B-03 | X-03 stop-and-flag rule; anything else discovered mid-implementation halts for a decision, it is not a judgment call |
| Changing legacy Battalion `ErrorStrategy` semantics | FT §5; untouched per X-03 |
| Notification-of-parley delivery mechanism | HITL §6; compose with existing `paladin-notifications` in application code — doc example only, no new port |
| Rebuilding the shipped OpenAI-compatible/Gemini/Ollama adapters | Shipped in v0.8.0; RT-06 verifies conformance instead (precedence: tree over PRD) |
| Browser-automation tests for the inspector page | OBS §5; DOM-level assertion via the existing HTTP harness only |
| Carried-in v0.9.0 debt items and Nyquist backfill for phases 05-21 | Tracked in PROJECT.md carried-in items; adopted by a phase only by explicit decision |

## Traceability

Which phases cover which requirements. Populated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENG-01 | Phase 22 | Complete |
| ENG-02 | Phase 22 | Complete |
| ENG-03 | Phase 22 | Complete |
| ENG-04 | Phase 22 | Complete |
| ENG-05 | Phase 22 | Complete |
| ENG-06 | Phase 22 | Complete |
| ENG-07 | Phase 22 | Complete |
| ENG-08 | Phase 22 | Complete |
| CF-01 | Phase 23 | Complete |
| CF-02 | Phase 23 | Complete |
| CF-03 | Phase 23 | Complete |
| CF-04 | Phase 23 | Complete |
| CF-05 | Phase 23 | Complete |
| HITL-01 | Phase 24 | Complete |
| HITL-02 | Phase 24 | Complete |
| HITL-03 | Phase 24 | Complete |
| HITL-04 | Phase 24 | Complete |
| HITL-05 | Phase 24 | Complete |
| FT-01 | Phase 25 | Complete |
| FT-02 | Phase 25 | Complete |
| FT-03 | Phase 25 | Complete |
| FT-04 | Phase 25 | Complete |
| FT-05 | Phase 25 | Complete |
| FT-06 | Phase 25 | Complete |
| RT-01 | Phase 26 | Complete |
| RT-02 | Phase 26 | Complete |
| RT-03 | Phase 26 | Complete |
| RT-04 | Phase 26 | Complete |
| RT-05 | Phase 26 | Complete |
| RT-06 | Phase 26 | Complete |
| RT-07 | Phase 26 | Complete |
| PLAT-01 | Phase 27 | Complete |
| PLAT-02 | Phase 27 | Complete |
| PLAT-03 | Phase 27 | Complete |
| PLAT-04 | Phase 27 | Complete |
| PLAT-05 | Phase 27 | Complete |
| PLAT-06 | Phase 27 | Complete |
| OBS-01 | Phase 28 | Complete |
| OBS-02 | Phase 28 | Complete |
| OBS-03 | Phase 28 | Complete |
| OBS-04 | Phase 28 | Complete |
| SHIP-01 | Phase 29 | Complete |
| SHIP-02 | Phase 29 | Complete |
| SHIP-03 | Phase 29 | Complete |
| SHIP-04 | Phase 29 | Complete |
| VOCAB-01 | Phase 30 | Complete |
| VOCAB-02 | Phase 30 | Complete |
| VOCAB-03 | Phase 30 | Complete |
| VOCAB-04 | Phase 30 | Complete |
| VOCAB-05 | Phase 30 | Complete |
| VOCAB-06 | Phase 30 | Complete |
| VOCAB-07 | Phase 30 | Complete |
| ACCT-01 | Phase 31 | Complete |
| ACCT-02 | Phase 31 | Complete |
| ACCT-03 | Phase 31 | Complete |
| ACCT-04 | Phase 31 | Complete |
| ACCT-05 | Phase 31 | Complete |
| PRIM-01 | Phase 32 | Complete |
| PRIM-02 | Phase 32 | Complete |
| PRIM-03 | Phase 32 | Complete |
| PRIM-04 | Phase 32 | Complete |
| PRIM-05 | Phase 32 | Complete |
| COMM-01 | Phase 33 | Complete |
| COMM-02 | Phase 33 | Complete |
| COMM-03 | Phase 33 | Complete |
| COMM-04 | Phase 33 | Complete |
| CURR-01 | Phase 34 | Complete |
| CURR-02 | Phase 34 | Complete |
| CURR-03 | Phase 34 | Complete |
| CURR-04 | Phase 34 | Complete |
| CURR-05 | Phase 34 | Complete |
| CURR-06 | Phase 35 | Complete |
| CURR-07 | Phase 35 | Complete |
| CURR-08 | Phase 35 | Complete |
| CURR-09 | Phase 35 | Complete |
| CURR-10 | Phase 35 | Complete |
| CURR-11 | Phase 36 | Complete |
| CURR-12 | Phase 36 | Complete |
| CURR-13 | Phase 36 | Complete |
| CURR-14 | Phase 36 | Complete |
| CURR-15 | Phase 36 | Complete |

**Coverage:**

- v1 requirements: 81 total (45 from the `.project/v0.10.0/` corpus, complete; 21 added
  2026-09-14 from `.project/Milestone_13-Token-Economy/`; 15 added 2026-09-17 for Phases 34-36)

- Mapped to phases: 81
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-01*
*Last updated: 2026-09-01 after initial definition from the `.project/v0.10.0/` design corpus*
*Extended: 2026-09-14 — VOCAB-01…07, ACCT-01…05, PRIM-01…05, COMM-01…04 added for Phases 30-33 from `.project/Milestone_13-Token-Economy/`; X-03 supersession for Phases 31-33 recorded above; FUT-08/FUT-09 now owned by the reserved Milestone 14; CURR-01…05 added 2026-09-17 for Phase 34 (Release Readiness); CURR-06…10 added 2026-09-17 for
Phase 35 (mdBook Currency); CURR-11…15 added 2026-09-17 for Phase 36 (Rustdoc Zero-Warning Bar &
Examples Currency), one per ROADMAP Phase 36 Success Criterion, minted under the Phase 34 `CURR-*`
prefix per 36-CONTEXT.md D-00h*
