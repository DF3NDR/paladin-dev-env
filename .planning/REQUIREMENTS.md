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
  control run, one Waypoint per superstep) (ENG-FR-12; overview §6 E2E-1)

- [x] **ENG-05**: Three `WaypointPort` backends — InMemory, SQLite (with migrations), Postgres
  (new `postgres` feature) — all pass one shared contract test suite; a
  `WaypointRetentionConfig` cleanup routine never deletes a thread's latest Waypoint or any
  `AwaitingInput` Waypoint (ENG-FR-15…18)

- [ ] **ENG-06**: Legacy string-based execution is bridged, not broken:
  `from_formation`/`from_phalanx`/`from_campaign` constructors reproduce today's data flow with
  golden output-equivalence tests, and the legacy execution services keep byte-identical public
  behavior (sole sanctioned exception: BUG-01, owned by CF-01) (ENG-FR-19, ENG-FR-20; X-03)

- [ ] **ENG-07**: The engine ships the seams later epics consume — a non-interfering `TraceSink`
  hook (bounded channel, drop-oldest, counted drops), an ordered `NodeInterceptor` chain, and a
  `CancellationToken` that finishes the in-flight superstep and persists a `Halted` Waypoint
  (ENG-FR-21…23; PRD 01 §8)

- [x] **ENG-08**: Program scaffolding mandated for the first epic: `MIGRATION.md` exists at the
  repository root with the §9 skeleton and pre-populated M-B-01…03 / §9.2 register entries, and
  CI gains the `cargo semver-checks` job (vs the published v0.9.0 crates, per-item allowlist
  only) and the MSRV job (Rust 1.85 toolchain, full workspace, `--all-features`), both running
  on every PR (overview §9, X-10.5, X-11.1)

### Control Flow (Doc 02, epic `CF`)

- [ ] **CF-01**: BUG-01 is fixed fail-closed and test-first: custom edge conditions resolve
  through a registered-`EdgeConditionEvaluator` mechanism on both `CampaignExecutionService` and
  the WarEngine; validation fails with `BattalionError::InvalidGraph` naming every unregistered
  `Custom(name)` **before any node executes**; the warn-and-return-true branch is removed with no
  restoring configuration; runtime evaluator errors fail the run rather than defaulting a branch
  (CF-FR-01…04; overview §7, M-B-01)

- [ ] **CF-02**: Nodes steer routing by returning a `Directive` — `NextStep::{Edges, Goto, End,
  Muster, Parley}` with Goto target validation, documented-and-tested End-over-Goto precedence,
  and a configurable `DirectiveParser` for Paladin nodes (`PlainOutput` backward-compatible
  default; `StructuredDirective` JSON envelope with `on_parse_error` modes) (CF-FR-05…08)

- [ ] **CF-03**: Muster dynamic fan-out works as map-reduce: a Directive spawns runtime-N worker
  tasks in one superstep with payload isolation, deterministic `task_key`-ordered aggregation
  (repeat-tested), duplicate-key rejection, `max_muster_tasks` limit, and mid-muster resume that
  re-runs only unfinished tasks (CF-FR-09…13)

- [ ] **CF-04**: Battalions nest — `NodeSpec::Battalion` embeds a child WarGraph with `StateMap`
  input/output mapping and private child fields, namespaced checkpoint inheritance with
  resume-mid-child, recursive-embedding rejection at validation, and legacy patterns embeddable
  (Formation-inside-Campaign integration test) (CF-FR-14…17)

- [ ] **CF-05**: LLM-evaluated routing is available and off by default: an `LlmDecision` edge
  evaluator (choice matching, `on_ambiguous` modes, application-layer registration) and
  `Commander` `StrategySelection::Semantic` that falls back to Heuristic on any LLM error with
  the fallback recorded — existing Commander tests pass unmodified (CF-FR-18, CF-FR-19)

### Pause/Resume, History, Shutdown (Doc 03, epic `HITL`)

- [ ] **HITL-01**: A workflow can pause indefinitely without holding compute: nodes (and a
  first-class `Gate` node with Battlefield templating) raise `ParleyRequest`s; suspension merges
  peer deltas, persists an `AwaitingInput` Waypoint carrying **all** of the superstep's parleys,
  releases every resource, and survives full process termination (resumable from a different
  process sharing the backend — integration-tested); partially-answered suspension is queryable
  (HITL-FR-01…03)

- [ ] **HITL-02**: `resume_with(graph, thread, responses)` validates responses per kind
  (Approval/Choice/FreeText/StateEdit) with typed errors that leave the thread suspended, honors
  `expires_at` with an `on_expire` policy, and delivers values to the paused node's continuation
  — proven by program scenario E2E-2 (approval gate, both branches, across process
  drop/recreate) (HITL-FR-04…06; overview §6 E2E-2)

- [ ] **HITL-03**: The Chronicle is inspectable and forkable: history/inspect over `WaypointPort`,
  `replay` and `fork`-with-edit create new chains with `fork_of` lineage while the original
  chain stays byte-identical (immutability hard invariant), branch-aware latest resolution, and
  defined subgraph-fork semantics (HITL-FR-07…12)

- [ ] **HITL-04**: Graceful shutdown loses no work: cancellation finishes the in-flight superstep
  within `shutdown_grace` (default 30 s; over-grace nodes recorded `Skipped` and re-listed in
  the vanguard), `resume` continues a `Halted` thread, and the facade wires SIGTERM/SIGINT to
  all in-flight runs with `k8s/` manifests and docs updated and a documented disable switch —
  registered as `MIGRATION.md` M-B-02 (HITL-FR-13…15)

- [ ] **HITL-05**: Engine-backed threads are reachable over HTTP: `GET /threads/{id}/state`,
  `POST /threads/{id}/resume` (409/400/404 semantics), `GET /threads/{id}/history` (paginated),
  following existing utoipa + error-envelope conventions with `openapi.json` regenerated
  (HITL-FR-16)

### Node-Level Fault Tolerance (Doc 04, epic `FT`)

- [ ] **FT-01**: Errors carry machine-usable transience: `transience()` on `PaladinError` and
  `LlmError` with per-variant table-driven tests, provider adapters gaining status-carrying
  variants (no string parsing), a structured `NodeError` carried through engine execution, and
  `BattalionError::Node(NodeError)` — all three touched pre-existing public enums handled per
  X-10 (`#[non_exhaustive]` or justified deliberate-breaking) and registered in `MIGRATION.md`
  §9.2 (FT-FR-01, FT-FR-02, FT-FR-02a)

- [ ] **FT-02**: Per-node Aegis retry works and is provable: exact backoff sequence under paused
  clock (jitter bounds asserted), transience-predicate gating (Permanent → 1 attempt),
  attempt isolation (failed-attempt deltas discarded, `AttemptRecord` history kept), per-task
  retries inside a Muster, and retries-within-a-superstep Waypoint semantics
  (FT-FR-03…07; PRD 04 §2.1)

- [ ] **FT-03**: Per-attempt timeouts distinguish stalled from slow: wall-clock `run_timeout` and
  progress-aware `idle_timeout` (stream chunks, trace events, `ctx.heartbeat()`), nested with
  engine/Battalion bounds so the tightest fires and the error names which (FT-FR-08…10)

- [ ] **FT-04**: Typed error handlers enable compensation: `Route` (structured NodeError into a
  declared state field, recovery node into the vanguard, run not failed), `Absorb` (fallback
  delta, continue), registered `Custom` handlers (fail-closed on unregistered names, may Parley),
  no-handler exhaustion failing with a structured — never stringified — error, and handler loops
  bounded by `max_node_visits`; program scenario E2E-3 passes together with CF-03
  (FT-FR-11…15; overview §6 E2E-3)

- [ ] **FT-05**: `FallbackLlmAdapter` fails over across a provider chain on Transient/Unknown
  errors only (Permanent short-circuits), propagates mid-stream errors without silent provider
  switch, returns `LlmError::AllProvidersFailed` with per-hop attempts, and records the serving
  provider on `PaladinResult` under X-10.3 (`Default` preserved, `#[non_exhaustive]`, §9.2
  register entry) (FT-FR-16, FT-FR-17)

- [ ] **FT-06**: Expensive deterministic nodes can cache: `CachePolicy` keyed by default on node
  id + resolved input + Paladin config fingerprint (prompt/model change invalidates naturally),
  `NodeCachePort` with InMemory and Redis adapters sharing a contract suite, hits merging the
  stored delta with `cache_hit: true` and no execution, failures never cached, Append-dispatch
  replay hazard documented with a schema-level `cache: Deny` marker (FT-FR-18…20)

### Agent Runtime Enhancements (Doc 05, epic `RT`)

- [ ] **RT-01**: `PaladinExecutionService` gains an ordered `ExecutionMiddleware` chain
  (before/after model, around tool) with onion ordering and short-circuit semantics
  (order-asserted by test), per-run state isolation under concurrency, and the same chain
  applying when a Paladin runs as an engine node — with the NodeInterceptor-vs-middleware
  two-layer distinction documented (RT-FR-01…03)

- [ ] **RT-02**: Built-in middleware ships, each config-structured per X-09: `ModelCallLimit` and
  `TokenBudget` finishing with new `StopReason::CallLimit`/`TokenBudget` variants (X-10 decision
  applied to both in the same change and registered in §9.2), `ToolCallLimit` denying without
  failing the run, `Guardrail` prompt/response screens with Fail/Redact/Finish actions, and
  retry/fallback middleware that delegates to the FT-05 implementations without duplicating
  logic (RT-FR-04…07, RT-FR-09)

- [ ] **RT-03**: Long conversations fit the context window: `TokenCounterPort` (heuristic default,
  provider adapters where possible; no inline heuristics), a stable never-splits-a-message
  `HistoryTrimmer`, and compounding `SummarizationMiddleware` persisting summaries to Garrison
  flagged `is_summary: true` — the Garrison entry field `#[serde(default)]`, the SQLite column
  additive-migrated and §9.2/§9.4-registered — degrading to trimming on summarizer failure,
  never failing the run (RT-FR-08, RT-FR-10…12)

- [ ] **RT-04**: Agents get confined cross-session memory: `VaultPort` (put/get/delete/list/
  search) with InMemory, SQLite and semantic (Sanctum/Qdrant-composed, `qdrant` feature)
  adapters under a shared contract suite; `vault_get`/`vault_put` Armaments confined to a
  host-granted namespace subtree (`NamespaceDenied` on traversal, attack-tested);
  `NodeContext::vault()`; opt-in `VaultRecallMiddleware` injecting top-k results — and the
  Garrison/Waypoint/Vault three-way distinction documented (RT-FR-13…16)

- [ ] **RT-05**: Structured output is first-class: `execute_structured<T>` on a new
  `StructuredExecutorPort` (not `PaladinPort`), schemars-generated schema in the application
  layer (MSRV-verified per X-11), native provider JSON modes via an additive `response_format`
  request field (X-10.3 handled and registered), a bounded repair loop with typed
  `StructuredOutputInvalid` exhaustion preserving raw output, and engine nodes with
  `output_schema` writing parsed JSON to their `output_field` — reused by `StructuredDirective`
  (RT-FR-17…19)

- [ ] **RT-06**: Provider conformance close-out (verify-then-fix, not greenfield — see scope-time
  conflict record): the shipped v0.8.0 OpenAI-compatible, Gemini and Ollama paths are measured
  against PRD 05's bar — shared conformance suite across adapters, FT-01 transience-correct
  429/5xx mapping, mock-server streaming coverage, documented Ollama recipe with an env-gated
  integration test — and only measured gaps are closed (RT-FR-20…22)

- [ ] **RT-07**: A tool-loop agent is a one-liner: `reasoning_agent(llm, tools, opts)` preset with
  a ≤15-line doc-tested example, and tool failures fed back into the model context by default
  (sanitized; `tool_error_mode: FeedToModel | FailRun` with per-tool override) — the chosen
  default and rationale recorded as `MIGRATION.md` M-B-03 (RT-FR-23, RT-FR-24)

### Platform API (Doc 06, epic `PLAT`)

- [ ] **PLAT-01**: Run submission is decoupled from execution: `POST /runs` returns 202 within
  250 ms p99 (enqueue only), a `RunRepositoryPort` (SQLite + Postgres, contract suite) persists
  every status transition, and the status machine is monotonic with typed illegal-transition
  errors (PLAT-FR-01; PRD 06 §2.1)

- [ ] **PLAT-02**: A worker pool executes runs durably: `RunQueuePort` (InMemory + Redis
  adapters, shared contract suite including lease-expiry redelivery), lease heartbeats at
  ≤ lease/3, at-least-once redelivery that **resumes** the thread rather than restarting
  (kill-mid-run test), cross-instance cancellation via a persisted flag + `CancellationProbe`
  observed at superstep boundaries, and the one-active-run-per-thread `409 ThreadBusy` invariant
  holding under 10 concurrent submits (PLAT-FR-02…05; PRD 06 §2.2)

- [ ] **PLAT-03**: Parley and streaming integrate with runs: `AwaitingInput` releases the worker,
  `POST /threads/{id}/resume` validates then re-enqueues under the same `run_id` (attempt++),
  and `GET /runs/{id}/stream` bridges live TraceSink events to SSE with a documented
  polling-backed degraded mode (terminal events always eventually delivered) and 15 s heartbeats
  (PLAT-FR-06, PLAT-FR-07)

- [ ] **PLAT-04**: Assistants are named, versioned configurations: append-only immutable versions
  (no PUT, ever), `latest` frozen at submit time for each run, full publish-time validation with
  machine-readable violations, creator/timestamp/note audit trail, the code-registered registry
  exposed read-only as synthetic entries, and `WarGraphDoc` with documented JSON Schema,
  registry-resolving `compile()`, and restart-stable fingerprint round-trip (PLAT-FR-08…12;
  PRD 06 §2.3)

- [ ] **PLAT-05**: Schedules and webhooks are API-managed: cron (5-field + optional seconds, UTC,
  thread strategies, skip/catch-up policies) surviving restart without duplicate or missed-then-
  double firing; webhook delivery on terminal + `AwaitingInput` events with HMAC
  `X-Paladin-Signature`, 5-attempt bounded retry on 5xx/timeout only, persisted queryable
  delivery attempts, async off the completion path; and an SSRF guard rejecting non-http(s),
  loopback, link-local, private and metadata targets at write and send time unless explicitly
  allowlisted (PLAT-FR-13…15; PRD 06 §2.4)

- [ ] **PLAT-06**: The new API surface is production-shaped: existing auth + rate limiting on
  every new endpoint, admin/writer scopes on mutating routes, pagination everywhere (limit
  ≤ 100, opaque cursor), `openapi.json` regenerated and diff-reviewed, and a CI job generating
  Python + TypeScript clients from the spec and smoke-testing them against a test server
  (PLAT-FR-16, PLAT-FR-17)

### Observability & Tooling (Doc 07, epic `OBS`)

- [ ] **OBS-01**: Every run has a machine-consumable account: the authoritative serde `TraceEvent`
  enum with per-run monotonic `seq` (causal-order guarantee, gapless-or-counted-drops test),
  bounded-payload field changes (never full state values by default), and a `TraceSinkPort`
  whose slow/panicking implementations cannot stall or fail a run (bounded channel, drop-oldest,
  counted; catch_unwind; never awaited), with `CompositeSink` fan-out (OBS-FR-01…03)

- [ ] **OBS-02**: Traces reach real consumers: a default-on structured-log sink; an `otel`-gated
  OpenTelemetry exporter with span-per-attempt trees verified against a collector stub; the SSE
  bridge for `GET /runs/{id}/stream` as a TraceSink adapter (one pathway, two consumers); and
  opt-in `run_traces` persistence upgrading post-hoc stream replay to full fidelity, sharing
  ENG-05 retention (OBS-FR-04…07)

- [ ] **OBS-03**: Graphs and runs are visualizable: golden-tested `WarGraphDoc → Mermaid/DOT`
  exporters, an execution-overlay export annotating outcomes/visit counts/fired edges/durations,
  `paladin-cli graph export` and `run export` commands, and a minimal auth-gated `dev-ui`
  inspector page from which a human can answer "which branch fired and why did node X run 3
  times" on the fixture run (OBS-FR-08…10)

- [ ] **OBS-04**: Agent behavior is regression-testable: the new `paladin-eval` crate with a
  scenario file format (scripted mock LLM behavior), an assertion library over the trace record

  + final Battlefield, a `cargo test`-integrable runner macro and `paladin-cli eval run` with
  `--repeat`/`--bless`, a gated live-model mode, and the three program E2E fixtures dogfooded as
  eval scenarios (OBS-FR-11…15)

### Program Gates & Release (overview §5/§9, doc 08, epic `SHIP`)

- [ ] **SHIP-01**: `MIGRATION.md` is complete: every §9 section filled with no "TBD" — M-B-01…03
  resolved with chosen defaults and worked examples, the §9.2 register matching the
  `cargo semver-checks` allowlist exactly, §9.3 toolchain/deps, §9.4 schema migrations with the
  Citadel-files-unchanged statement, §9.5 config/env with the disabled-by-default claim, §9.6
  HTTP surface, §9.7 deprecations, §9.8 operator checklist — linked from the README and the
  mdBook "Upgrading" page (overview §9; DoD 4)

- [ ] **SHIP-02**: Backward compatibility is proven, not asserted: an integration test boots
  v0.10 with a v0.9 sample config and asserts legacy behavior (all new subsystems disabled by
  default), and a golden diff of `openapi.json` restricted to pre-existing paths is empty
  (overview §9.5, §9.6)

- [ ] **SHIP-03**: The program acceptance audit passes: E2E-1/2/3 green as integration tests in
  `tests/`, the doc-08 verification protocol run (every FR has a passing test, no orphan
  behavior, ubiquitous-language names conform), and BUG-01's old warn-and-default-true path
  grep-absent with the fix's failing-then-passing test order visible in history (overview §5-§6;
  doc 08)

- [ ] **SHIP-04**: v0.10.0 is releasable: all workspace crates at `0.10.0` with changelogs
  updated, `cargo publish --dry-run` green for every publishable crate in dependency order,
  mdBook + rustdoc updated with no new broken intra-doc links, and the semver and MSRV CI jobs
  green on the release commit (overview §5 DoD 1, 3, 6, 7; X-08)

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
| ENG-06 | Phase 22 | Pending |
| ENG-07 | Phase 22 | Pending |
| ENG-08 | Phase 22 | Complete |
| CF-01 | Phase 23 | Pending |
| CF-02 | Phase 23 | Pending |
| CF-03 | Phase 23 | Pending |
| CF-04 | Phase 23 | Pending |
| CF-05 | Phase 23 | Pending |
| HITL-01 | Phase 24 | Pending |
| HITL-02 | Phase 24 | Pending |
| HITL-03 | Phase 24 | Pending |
| HITL-04 | Phase 24 | Pending |
| HITL-05 | Phase 24 | Pending |
| FT-01 | Phase 25 | Pending |
| FT-02 | Phase 25 | Pending |
| FT-03 | Phase 25 | Pending |
| FT-04 | Phase 25 | Pending |
| FT-05 | Phase 25 | Pending |
| FT-06 | Phase 25 | Pending |
| RT-01 | Phase 26 | Pending |
| RT-02 | Phase 26 | Pending |
| RT-03 | Phase 26 | Pending |
| RT-04 | Phase 26 | Pending |
| RT-05 | Phase 26 | Pending |
| RT-06 | Phase 26 | Pending |
| RT-07 | Phase 26 | Pending |
| PLAT-01 | Phase 27 | Pending |
| PLAT-02 | Phase 27 | Pending |
| PLAT-03 | Phase 27 | Pending |
| PLAT-04 | Phase 27 | Pending |
| PLAT-05 | Phase 27 | Pending |
| PLAT-06 | Phase 27 | Pending |
| OBS-01 | Phase 28 | Pending |
| OBS-02 | Phase 28 | Pending |
| OBS-03 | Phase 28 | Pending |
| OBS-04 | Phase 28 | Pending |
| SHIP-01 | Phase 29 | Pending |
| SHIP-02 | Phase 29 | Pending |
| SHIP-03 | Phase 29 | Pending |
| SHIP-04 | Phase 29 | Pending |

**Coverage:**

- v1 requirements: 45 total
- Mapped to phases: 45
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-01*
*Last updated: 2026-09-01 after initial definition from the `.project/v0.10.0/` design corpus*
