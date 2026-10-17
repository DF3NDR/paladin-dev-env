# Architecture Research: v0.11.0 "Treasurer Spend Governance"

**Domain:** Rust hexagonal multi-agent orchestration framework (Paladin) — output-side spend
governance added to an existing durable-run platform.
**Researched:** 2026-09-24
**Confidence:** HIGH (every claim below is grounded in a direct `Read`/`Grep` of the shipped
tree, not the PRD's aspirational text — file paths and line-anchored quotes are given throughout)

## Standard Architecture (as it exists today, before this milestone)

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│  paladin-web (Axum)               crates/paladin-web/src/                │
│   POST /runs · GET /runs* · agent_auth.rs (Principal{id,role}, NO tenant)│
└───────────────────────────────────┬──────────────────────────────────────┘
                                    │  Extension<Principal>
┌───────────────────────────────────▼──────────────────────────────────────┐
│  facade application services      src/application/services/run/          │
│                                                                           │
│  RunSubmissionService::submit  ──▶ resolve ─▶ insert ─▶ enqueue          │
│    (admission — no engine touched, no Treasurer today)                  │
│                                                                           │
│  RunWorkerPool::run_once  ──▶ per-run WarEngine + TraceDispatcher        │
│    (Runnable::Workflow → superstep engine; Runnable::Agent → legacy      │
│     run_agent, bypasses SSE bus / webhook hooks entirely)                │
└───────────────────────────────────┬──────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        │                           │                           │
┌───────▼────────┐        ┌─────────▼─────────┐        ┌────────▼────────┐
│ paladin-battalion│        │ src/config/        │        │ paladin-storage │
│  WarEngine/WarGraph│      │ agent_runtime.rs   │        │ RunRepositoryPort│
│  Aegis retry/cache│       │  AgentRuntimeConfig│        │ RunQueuePort     │
│  NodeError (Aegis) │      │  ::build_chain →   │        │ WaypointPort     │
│  (per-node, engine-│      │  TokenBudget mw    │        │ (in-mem/sqlite/  │
│   level, unrelated │      │  (NO PRODUCTION    │        │  postgres)       │
│   to battalion::   │      │  CALL SITE TODAY —  │        │                 │
│   NodeError, D-06) │      │  tests/doctests     │        └─────────────────┘
└─────────────────┘         │  only, see below)   │
                             └──────────┬──────────┘
                                        │
                             ┌──────────▼──────────┐
                             │ PaladinExecutionService│  ← the agent reasoning
                             │  (facade, per-loop     │    loop the TokenBudget
                             │  middleware onion)     │    middleware attaches to
                             └──────────┬──────────┘
                                        │
                             ┌──────────▼──────────┐
                             │ paladin-llm             │
                             │  Commissary (input-side,│ ← untouched by Treasurer
                             │   per-call ration)      │   (two-officer model)
                             │  FallbackLlmAdapter      │
                             │  (Transient/Unknown hop)│
                             │  http_status::map_http_ │
                             │   status → LlmError::   │
                             │   RateLimitExceeded (429)│
                             └─────────────────────────┘
```

### Component Responsibilities (existing, load-bearing for this milestone)

| Component | Responsibility | File | Note |
|-----------|----------------|------|------|
| `RunSubmissionService::submit` | Admission: resolve → insert → enqueue, zero engine imports | `src/application/services/run/submission.rs` | The 250ms p99 architectural claim rests on this file staying engine-free — a Treasurer admission check must not violate that |
| `AgentRuntimeConfig::build_chain` | Assembles the fixed-order middleware chain (`limits → guardrail → trimmer/summarizer → recall → resilience`) | `src/config/agent_runtime.rs:327` | **Has zero production call sites** — grep of `src/**/*.rs` finds `.build_chain(` only inside `agent_runtime.rs`'s own doctests/tests (lines 323, 1779, 1804, 1858, 1906, 1932, 1956, 1988). No `worker.rs`, `run_api_wiring.rs`, or preset wires it into the run path today. |
| `TokenBudget` middleware | Caps accumulated `total_tokens` for one `PaladinExecutionService` run via `after_model`, finishes with `StopReason::TokenBudget` | `src/application/services/paladin/middleware/limits.rs:134-174` | Per-run counter lives on `ModelCallContext`, never on the middleware struct (D-03) — the SAME `Arc<dyn ExecutionMiddleware>` instance safely backs many concurrent runs |
| `ExecutionMetadata.cost_estimate` | `Option<f64>`, dead field | `crates/paladin-core/src/platform/container/herald.rs:505` | Already wired through `json_herald.rs:212` and `markdown_herald.rs:422` — both read it and render `None` today. No producer anywhere in the tree. |
| `PaladinError::LlmError(String)` | Legacy stringly-typed erasure, retained-but-unconstructed | `crates/paladin-core/src/platform/container/paladin_error.rs:29,191,256` | Phase 25 (25-06) already migrated every first-party production erasure site to `PaladinError::LlmFailure{transience,status,provider,message}`; the variant survives only in exhaustive `match` arms (`is_retryable`, `transience`, `Display`) and two test/doc sites (`conclave_execution_service.rs:347,736,739`, `llm_failure.rs:196,251`, `paladin_port.rs` doc comments) |
| Legacy `battalion::RetryPolicy`/`ErrorStrategy`/`NodeError` | Pre-Aegis, pre-v0.10 Formation/Phalanx/Campaign-only config+result types | `crates/paladin-core/src/platform/container/battalion/mod.rs:189,240,520` | **Distinct type from the Aegis `NodeError`** in `crates/paladin-core/src/platform/container/node_error.rs:152` (superstep-engine, structured, `D-07`) — the module doc at `battalion/mod.rs:496-518` explicitly warns "not the same type... never aliased here" |
| `RunStatus::Halted` | Already-modeled "gracefully halted, terminal" status | `crates/paladin-core/src/platform/container/run.rs:126-127` | Exists today for cancellation/shutdown drain — the Treasurer's mid-run halt is a **new caller of an existing status**, not a new status |
| `TraceDispatcher` / per-run trace sink assembly | Built once per run inside `RunWorkerPool::run_once`, before the engine exists, handed to engine + fallback adapter + middleware chain via one `Arc<dyn TraceEmitter>` | `src/application/services/run/worker.rs:847-880` | The single load-bearing "one counter, not two racing dispatchers" seam (D-03) — any Treasurer trace event must go through this same handle |
| `Principal` | `{id: String, role: UserRole}` — **no tenant field** | `crates/paladin-web/src/agent_auth.rs:36-42` | Confirmed by direct read: `grep tenant crates/paladin-web/src/**/*.rs` returns nothing relevant. Per-tenant allowances have **no existing identity carrier**. |
| `list_runs` (`GET /runs`) | Extracts `Extension(_principal)` **and discards it** (underscore-prefixed, unused) | `crates/paladin-web/src/run_controller.rs:747-787` | This is the exact, already-flagged site for the "GET /runs* per-caller scoping" deviation (WINDOWS.md row 32) |
| `Runnable::Agent` dispatch | `RunWorkerPool::run_once` branches to `run_agent`, which never binds the D-24 event bus and never reaches the webhook-delivery enqueue block | `src/application/services/run/worker.rs:488-518,806-816` | Exact wiring gap for the "SSE/webhook emission for legacy Runnable::Agent runs" deviation (WINDOWS.md row 31) |

## Recommended Integration Points for Treasurer Features

### 1. New crate/layer placement

| New component | Layer | Crate | Rationale |
|---|---|---|---|
| `TreasuryLedgerPort` (trait) | Port (output) | `paladin-ports/src/output/treasury_ledger_port.rs` | Mirrors the existing `RunRepositoryPort` pattern exactly — same crate, same directory, same `Send + Sync` async-trait shape (see §3) |
| `TreasurerPort` / allowance-authorization port, if the Treasurer's admission check needs to cross the facade→port boundary (it likely does not — see §4) | Port (input or output, TBD at plan time) | `paladin-ports` | Only needed if `paladin-web` or another leaf crate must call the Treasurer without depending on the facade; `RunSubmissionService` itself is already facade-layer, so a plain facade-internal collaborator may suffice — **flag this as a build-order-1 design decision**, not pre-decided by this research |
| `Treasurer` service (pricing math, allowance check, ledger writes) | Application service (facade) | `src/application/services/treasurer/` (new directory, sibling of `src/application/services/run/`) | Matches every other cross-cutting facade service (`RunSubmissionService`, `RunWorkerPool`) — composes ports, is not itself a port |
| Pricing table types (`PriceTable`, `ModelPrice`) | Config | `src/config/treasurer.rs` (new, sibling of `src/config/agent_runtime.rs`) | Operator-configured, mirrors `AgentRuntimeConfig`'s `Default` + `validate()` + `EnvOverridable` shape — "nothing bundled" (PRD R1) means this ships with an empty default table, not seeded prices |
| `InMemoryTreasuryLedger` / `SqliteTreasuryLedger` / `PostgresTreasuryLedger` | Adapter | `paladin-storage/src/treasury/{in_memory,sqlite,postgres}.rs` | Exact sibling of `paladin-storage/src/run/{in_memory,sqlite,postgres}.rs` |
| Migrations | Adapter data | `crates/paladin-storage/migrations/{postgres,sqlite}/007_create_treasury_ledger_table.sql` (and an `008_...` for an `allowance_windows`/rolling-period table if the allowance state itself is persisted, not derived per-query) | **Not** the stale root `migrations/*.sql` the codebase-map docs describe — the real, shipped location is `crates/paladin-storage/migrations/{postgres,sqlite}/NNN_*.sql`, currently at `006_create_run_traces_table.sql` (confirmed by direct `find`) |
| Rate-pacer (in-process token-bucket / back-off state) | Application service or LLM-adjacent adapter | `crates/paladin-llm/src/services/pacer.rs` (sibling of `crates/paladin-llm/src/services/commissary.rs`) OR a new `paladin-llm/src/pacing/` module | Belongs beside `Commissary` structurally (both are per-call LLM-port-adjacent concerns) but is conceptually **output/Treasurer-owned**, not input/Commissary-owned — keep the two-officer boundary by not touching `commissary.rs` itself (ADR-0049/0050 guardrail) |
| Redis-shared pacing + stampede lock | Adapter | `paladin-storage/src/pacing/redis.rs`, gated behind the existing `redis-queue`/`redis` Cargo feature (`crates/paladin-storage/Cargo.toml:29,62`) | Reuses the same optional `redis` dependency already wired for `RunQueuePort`'s Lua lease claims — no new external dependency |

### 2. Where the Treasurer hooks admission (blocking a draw)

`RunSubmissionService::submit` (`src/application/services/run/submission.rs:240-311`) is the ONE
place a run is admitted today. Its documented invariant (module doc, lines 1-9) is **zero engine
imports, one resolve/insert/enqueue** — that invariant must survive. The Treasurer's admission
check is a new step **before** `self.repository.insert(...)`, structurally identical to the
existing write-time SSRF guard (`self.ssrf_guard.check_url(...)`, lines 244-250) and the
`authorize_invocation` role check (line 262): a fast, synchronous-shaped async call that returns
early with a new `RunSubmissionError` variant on refusal, touching nothing durable.

```
submit():
  1. SSRF guard (existing)
  2. resolve assistant (existing)
  3. authorize_invocation (existing, role-based)
  4. NEW: treasurer.authorize_draw(tenant/api-key, estimated cost) -> refuse before insert
  5. insert (existing)
  6. enqueue (existing)
```

This mirrors the PRD's own language ("refused at admission") and keeps `RunSubmissionService`
free of the engine dependency it currently guards. A new `RunSubmissionError::AllowanceExhausted`
(or similar) variant follows the exact pattern `RunSubmissionError::ThreadBusy`/`WebhookRejected`
already establish.

### 3. Where the Treasurer hooks per-draw checks (mid-run, installing `TokenBudget`)

The PRD is explicit: "The Treasurer installs the per-run `TokenBudget` rather than replacing it"
(PRD §1). Today, `AgentRuntimeConfig::build_chain` — the ONLY code that constructs a `TokenBudget`
middleware — has **no production caller** (confirmed above). This means:

- The Treasurer's per-draw hook is most naturally the **first production caller of
  `build_chain`**, or of a Treasurer-owned equivalent that constructs a `TokenBudgetConfig` whose
  `max_tokens` is derived from the tenant/key's remaining allowance (converted token-equivalent or
  tracked directly in currency, per whichever build decision R1/R3 resolve to) and passes it
  through the SAME assembly order (`limits` first, per the documented fixed order in
  `agent_runtime.rs:49-58`).
- This wiring must happen where `PaladinExecutionService` is actually constructed for a run. That
  site is **not** `RunWorkerPool` today — a grep of `src/application/services/run/**/*.rs` for
  `PaladinExecutionService::new`/`with_middleware` finds only `tracer_e2e.rs` (a test). The
  production `Runnable::Agent` path (`run_agent`) and the `NodeSpec::Paladin` dispatch inside
  `WarEngine` (via `crates/paladin-battalion/src/engine/*.rs`, which has zero references to
  `PaladinExecutionService`/`AgentRuntimeConfig`/`middleware`) currently execute Paladin nodes
  through the lower-level `PaladinPort` directly, **bypassing the middleware onion entirely**.
- **Architectural implication, stated plainly:** wiring a per-run, allowance-derived `TokenBudget`
  into the actual `WarGraph`/`WarEngine` execution path (where the overwhelming majority of runs
  execute today, per v0.10.0's own framing as "a durable agent execution runtime") requires either
  (a) threading `AgentRuntimeConfig`/`build_chain` output into the engine's `NodeSpec::Paladin`
  dispatch for the first time, or (b) the Treasurer enforcing the budget at a DIFFERENT layer than
  `TokenBudget` for engine-driven runs — e.g. reading the same `cumulative_tokens`/`TokenUsage`
  the engine already aggregates per `NodeExecutionRecord`/`RunFinished` trace event, and raising a
  typed halt from there instead. This is a genuine open design question this research surfaces,
  not one the codebase already answers — the PRD's "installs the per-run TokenBudget" language
  matches the `PaladinExecutionService` agent-loop shape cleanly, but the shipped platform's
  primary run path is the superstep engine, where `TokenBudget` has never been wired.
- Recommendation for planning: scope Milestone 14's first phase to explicitly resolve this seam
  (does the Treasurer wire through `build_chain` for `Runnable::Agent` runs only, mirroring the
  PRD's literal text, and reserve engine-level enforcement for a documented gap/follow-up? Or does
  it need a new engine-level hook analogous to `EngineRegistries`/Aegis retry predicates?). Either
  answer is buildable; neither is free, and the PRD does not resolve it.

### 4. Mid-run halt → typed error / run status / Waypoint resume

No new `RunStatus` variant is needed. `RunStatus::Halted` already exists
(`crates/paladin-core/src/platform/container/run.rs:126-127`, doc: "Gracefully halted (cancellation
or shutdown drain). Terminal.") — a Treasurer-triggered halt is a **third caller** of the same
terminal status, alongside cancellation and graceful shutdown. Because checkpointing is automatic
per-superstep (Waypoint, v0.10.0's keystone), a halt raised between supersteps needs no special
resume handling beyond what cancellation already exercises — `fork`/resume already works from any
persisted Waypoint via `RunSubmissionService::fork` (`submission.rs:367-471`).

The typed-error side needs a new variant analogous to `StopReason::TokenBudget`
(`crates/paladin-ports/src/output/paladin_port.rs:141-161`) for the agent-loop path — e.g.
`StopReason::AllowanceExhausted` — and, for the engine path, a new `NodeErrorSource`/`NodeError`
(Aegis, `node_error.rs`) or a run-level halt reason surfaced through `RunOutcomeRecord`
(`RunRepositoryPort::record_outcome`, referenced at `27-02-PLAN.md:96`). `StopReason::TokenBudget`
is `is_successful() == true` (budget stopping a run is not a failure) — the Treasurer's own
stop reason should follow the same convention: an allowance exhaustion is a policy stop, not an
execution failure, so it should NOT be classified as an Aegis-retryable transient error.

**Known open bug this milestone must not regress:** the SSE `done` event currently reports
`Cancelled` (not `Halted`) for a caller-cancelled run (D-14, PROJECT.md line 437) — a pre-existing
defect in whatever maps `RunStatus` to the SSE terminal payload. A Treasurer-triggered halt uses
the SAME `RunStatus::Halted` path, so fixing D-14 and wiring the Treasurer halt are the same
code region and should be sequenced together, not independently.

### 5. Tenant/API-key identity flow (paladin-web auth → runs)

**This is the most significant open gap Treasurer plans must resolve.** The existing `Principal`
type (`crates/paladin-web/src/agent_auth.rs:36-42`) carries only `{id: String, role: UserRole}`.
There is no `tenant_id` field anywhere in the auth model, and `grep -rn tenant` across
`crates/paladin-web/src/**/*.rs` returns no relevant hits. `list_runs` already extracts
`Extension(_principal)` and discards it (line 749) — the deviation item this milestone must close
regardless of Treasurer scope.

Two design paths, both buildable on the existing shape:
- **Minimal (per-API-key only):** use `Principal.id` (the API-key name, already unique per
  `AgentAuthConfig::api_keys: HashMap<String, Principal>`) as the allowance key directly — no
  schema change to `Principal` needed. This satisfies R3's "per-API-key" half immediately.
- **Full (per-tenant too):** `Principal` needs a new `tenant_id: Option<String>` (or similar) field,
  sourced from the API-key config map's value type or from `AuthPort::verify_token`'s claims (the
  opaque-bearer-token path). This is a **breaking change to a public struct** (`Principal` has no
  `#[non_exhaustive]` marker observed) — must be scoped, semver-checked, and given a MIGRATION.md
  row like every other v0.11.0 clean-break item.
- `RunSubmissionService::submit`/`cancel`/`fork` already carry `requested_by: Option<(String,
  UserRole)>` end-to-end from the web layer through to the authorization check
  (`authorize_invocation`, `submission.rs:221-235`) — this is the existing, precedented channel a
  Treasurer draw-check would extend (add a third element, or a new field, carrying tenant/key
  identity through to the admission check in §2).

### 6. Ledger schema & migrations location

Confirmed by direct filesystem inspection — **not** the stale `migrations/*.sql` path the
`.planning/codebase/STRUCTURE.md` map (dated 2026-07-30) still describes:

```
crates/paladin-storage/migrations/
├── postgres/
│   ├── 001_create_waypoints_table.sql
│   ├── 002_create_runs_table.sql
│   ├── 003_create_assistants_tables.sql
│   ├── 004_create_run_schedules_table.sql
│   ├── 005_create_webhook_deliveries_table.sql
│   └── 006_create_run_traces_table.sql        ← current tip
└── sqlite/
    └── (mirrored 001-006)
```

A `TreasuryLedgerPort` adapter set follows this exact convention: `007_create_treasury_ledger_
table.sql` (append-only spend rows: run_id, tenant/key, model, token breakdown, currency cost,
recorded_at) in both `postgres/` and `sqlite/`, mirroring the twin-directory pattern every prior
port (`run`, `run_queue`, `waypoint`, `run_schedules`, `webhook_deliveries`, `run_traces`) already
uses. If allowance state itself is persisted (rolling-window remaining balance, not recomputed
per-query from the ledger), it needs a second migration (`008_create_allowance_windows_table.sql`)
— a build-order-1 decision (derive-on-read vs. maintain-a-running-balance) that materially changes
the schema shape and should be settled before either migration is written.

### 7. Rate pacing relative to `FallbackLlmAdapter` and Aegis retry

`LlmError::RateLimitExceeded` is the canonical 429 signal, produced uniformly by
`crates/paladin-llm/src/http_status.rs:98` (`map_http_status`, shared across all nine provider
adapters per Phase 25) and classified `Transience::Transient` in
`crates/paladin-ports/src/output/llm_port.rs:509`. Two existing consumers already react to it:

- **Aegis per-node retry** (`crates/paladin-core/src/platform/container/aegis.rs`, engine-level,
  `TransientOnly` predicate) — retries a `RateLimitExceeded` node failure with backoff+jitter,
  per-node, inside the superstep engine.
- **`FallbackLlmAdapter`** (`crates/paladin-llm/src/fallback.rs`) — hops to the next provider in a
  configured chain on a Transient/Unknown failure, first-chunk streaming rule, `served_by` on the
  result.

Both are **reactive** (they respond to a 429 that already happened). Rate **pacing** (FUT-09) is
**proactive** — it should sit BEFORE the call is made, one layer below the middleware chain and
above (or beside) the raw LLM adapter call, so that:

```
Treasurer pacer (proactive, in-process token-bucket + Redis-shared cross-worker)
        ↓ (paces the call before it's dispatched)
Commissary::verify_fits / dispense (pre-flight, input-side, UNTOUCHED — two-officer boundary)
        ↓
LLM adapter HTTP call
        ↓ (on 429)
Aegis retry (engine) / ModelRetryMiddleware+ModelFallbackMiddleware (agent-loop, per D-10's fixed
  order "...resilience" last) / FallbackLlmAdapter provider hop
```

Concretely: the pacer is a new, thin wrapper/decorator around whichever `LlmPort` a run resolves
(same shape as `FallbackLlmAdapter` itself — an `LlmPort`-implementing adapter that wraps another
`Arc<dyn LlmPort>`), inserted OUTSIDE `FallbackLlmAdapter` (paces every hop, not just the first
provider) but the pacer itself should never retry — it only delays/gates the call, leaving retry
and fallback exactly where they are today. This keeps the change additive: no existing adapter,
middleware, or Aegis code needs modification, only a new decorator composed at the same site
`FallbackLlmAdapter` is composed today (wherever a run's `Arc<dyn LlmPort>` is assembled — currently
inside `LlmProviderFactory`/preset construction, not yet located precisely in this research pass;
flag for the discuss-phase step).

The stampede lock (PRD's "distributed cache-stampede lock") is a Redis primitive parallel to
`RunQueuePort`'s existing Lua-scripted lease claims (`paladin-storage/src/run_queue/redis.rs`) —
same optional `redis` dependency, same feature gate, no new external service.

### 8. Blast radius of the legacy removals (measured call-site counts)

| Symbol to remove | Grep count (occurrences / files) | What's actually there |
|---|---|---|
| `battalion::RetryPolicy` (legacy) | 5 files import/use it directly: `battalion/mod.rs` (definition), `aegis.rs` (unrelated, its OWN distinct `RetryPolicy` at `aegis.rs:99` — do not conflate), `paladin-battalion/src/retry.rs` (`calculate_retry_delay`, the whole file's reason to exist), `formation_service.rs`, `commander.rs` | `RetryPolicy` total occurrences across the tree: 179 across 16 files — but the bulk (`engine/retry.rs`, `engine/graph.rs`, `engine/mod.rs`, `engine/superstep.rs`, `engine/graph_doc.rs`, `middleware/resilience.rs`, `middleware/context.rs`, `node_error.rs`, `aegis.rs`) are the **Aegis** `RetryPolicy` (`aegis.rs:99`), a **different type in a different module** that is explicitly NOT in scope for removal. The legacy-only subset is materially smaller — concentrated in `paladin-battalion/src/retry.rs`, `formation_service.rs`, `commander.rs`, plus doctests/examples (`commander_full_config.rs`) |
| `battalion::ErrorStrategy` (legacy) | `ErrorStrategy` total: 113 occurrences across 16 files — but `crates/paladin-battalion/src/maneuver/mod.rs:18` defines its **own, separate** `ErrorStrategy` enum for the Maneuver DSL, unrelated to `battalion::ErrorStrategy`. The legacy-Battalion-config variant is used in `commander.rs` (25), `phalanx_service.rs` (6), `formation_service.rs` (13), `battalion/mod.rs` itself (24, mostly doc examples), plus `examples/*.rs` (formation_sequential, phalanx_parallel, commander_*). **The Maneuver DSL's `ErrorStrategy` in `maneuver/mod.rs`/`maneuver/service.rs` must be left untouched** — confirming its own identity is essential before any removal grep/sed runs |
| `battalion::NodeError` (legacy, `battalion/mod.rs:520`) | Used on `BattalionResult.node_errors: Vec<NodeError>` (line 582) | Small, contained blast radius — the type's own doc comment (lines 496-518) already pins the exact boundary against the Aegis `NodeError`, so this removal is a scoped, single-struct change plus its one field on `BattalionResult` |
| Legacy Formation/Phalanx/Campaign timeout handling | `formation_service.rs:163,178,180` (`Duration::from_secs(formation.config.timeout_seconds)`, `BattalionError::Timeout`) is the clearest concrete site; Phalanx/Campaign likely mirror this pattern (not individually re-verified in this pass — flag for a code-audit subagent at plan time) | Distinct from the v0.10.0 Aegis `TimeoutPolicy`/`TimeoutKind` (`node_error.rs:36-66`, `EngineLimits.run_timeout`), which is NOT in scope |
| `PaladinError::LlmError(String)` | **Zero first-party production constructions** (Phase 25-06 already migrated every real site to `PaladinError::LlmFailure`). Surviving references: 3 exhaustive-match arms in `paladin_error.rs` (`is_retryable`, `transience`, `Display`, ~lines 191/256/54), one exhaustive-match arm + 2 doc-comment mentions in `llm_failure.rs`, one match arm in `conclave_execution_service.rs:347` plus 2 test literals at 736/739, doc-comment mentions in `paladin_port.rs`, `temperature_service.rs`, `paladin_execution_service.rs` | **Smallest of the four removals** — this is a genuinely mechanical deletion: drop the variant, drop its match arms (compiler will name every site), drop the now-dead doc comments. The 2026-09-14 decision record already anticipated this exact follow-up ("a future plan may deprecate it under X-10 without touching any call site," `25-06-SUMMARY.md`) |

Net assessment: the removal work is **real but bounded** — roughly a dozen first-party call sites
across 3-4 crates for the two `battalion::` types, a handful of `formation_service.rs`/
`phalanx_service.rs`/`campaign_service.rs` timeout sites, and a near-mechanical deletion for
`LlmError(String)`. The dominant risk is **false-positive greps** (Aegis's own `RetryPolicy`, the
Maneuver DSL's own `ErrorStrategy`, Aegis's own structured `NodeError`) rather than removal volume
— any plan/execution step must disambiguate by full path (`battalion::RetryPolicy` vs
`aegis::RetryPolicy`; `battalion::ErrorStrategy` vs `maneuver::ErrorStrategy`; `battalion::NodeError`
vs `node_error::NodeError`) before running a removal grep, or it will delete or flag the wrong type.

## Suggested Build Order

Numbered by dependency, not necessarily by phase — a phase can bundle adjacent, non-conflicting
items.

1. **Resolve the two open design seams first (no code yet):**
   - Where does the Treasurer's per-draw check actually attach given `build_chain` has no
     production caller today and the engine path bypasses the middleware onion entirely (§3)?
   - Derive-on-read vs. maintain-a-running-balance for allowance state (shapes the ledger schema,
     §6)?
   These two decisions gate every subsequent item's design; get them recorded (ADR or D-nn) before
   planning phases.

2. **Pricing + `cost_estimate` producer (R1/R2).** Self-contained: a new `src/config/treasurer.rs`
   pricing table, a pure function `TokenUsage × PriceTable → f64`, and one write site into
   `ExecutionMetadata.cost_estimate` (builder already has `.cost_estimate(f64)` at
   `herald.rs:592`). No port changes, no admission-path changes, low risk, immediately visible in
   heralds/CLI (which already read the field). **Do this first** — it has zero dependency on the
   allowance/ledger work and de-risks the "populate a long-dead field" claim early.

3. **`TreasuryLedgerPort` + three adapters + migrations (R5).** Depends on step 1's schema
   decision. Build exactly like `RunRepositoryPort`: trait in `paladin-ports`, in-memory first
   (fast tests), then SQLite/Postgres with the `crates/paladin-storage/src/run/contract_tests.rs`
   pattern reused for a `treasury/contract_tests.rs`. Depends on step 2's cost-per-token math to
   have something meaningful to record.

4. **Tenant/API-key identity flow (part of R3).** Decide minimal-vs-full (§5) and, if full, land
   the `Principal`/auth-config breaking change with its own MIGRATION.md row and semver-allowlist
   entry — independent of the ledger, but the allowance-authorization step (5) needs it done first.

5. **Treasurer service: admission-time allowance check (R3, first half).** Wires into
   `RunSubmissionService::submit` per §2, using the ledger (step 3) and identity (step 4). This is
   the "refuses a draw at admission" acceptance criterion and can ship independently of the mid-run
   halt.

6. **Treasurer service: per-draw / mid-run halt (R3, second half).** Depends on step 1's resolved
   attachment point. Requires the new `StopReason`/halt-reason variant and reuses the existing
   `RunStatus::Halted` (§4) — bundle the SSE `done`→`Cancelled`-vs-`Halted` fix (D-16, row 35's
   sibling) into this same phase since it's the same code region.

7. **Rate pacing (R4/FUT-09).** Independent of steps 2-6 — can run in parallel once step 1 (or a
   scoped subset of it) is settled. New `LlmPort`-wrapping decorator (§7) plus a Redis-shared
   pacer/stampede-lock adapter behind the existing `redis-queue` feature gate.

8. **Legacy removals (X-03 supersession).** Independent of the Treasurer feature work itself —
   can be sequenced anywhere, but doing it AFTER steps 2-7 avoids rebasing removal diffs against
   concurrent Treasurer-touched files in `formation_service.rs`/`battalion/mod.rs` neighbors.
   Requires the disambiguation discipline in §8 before any removal grep/sed. Needs its own
   MIGRATION.md §9.2 rows and `cargo semver-checks` allowlist entries per the milestone's stated
   X-03 supersession requirement.

9. **Platform deviations (row 31, row 32, D-14) + tracing overhead (D-16, row 35).** Independently
   schedulable bug-fix-shaped work; row 32 (`GET /runs*` scoping) is now a trivial change given the
   identity work in step 4 already threads a real `Principal`/tenant through — sequence it right
   after step 4 to avoid re-deriving the same plumbing twice.

10. **RustFS (FUT-10) and docs/hygiene closing phases.** Fully independent of the Treasurer
    feature surface; schedule last, alongside the closing crates.io v0.11.0 release phase.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Conflating the two `RetryPolicy`/`ErrorStrategy`/`NodeError` type families

**What people do:** grep for `RetryPolicy` or `ErrorStrategy` and remove every hit.
**Why it's wrong:** `aegis::RetryPolicy` (v0.10.0, engine-level, actively used by the superstep
engine's per-node retry) and `maneuver::ErrorStrategy` (Flow DSL) share names with the legacy
`battalion::` types this milestone actually targets. Removing the wrong one breaks the superstep
engine or the Maneuver DSL, not the intended legacy surface.
**Do this instead:** always qualify by full path (`paladin_core::platform::container::battalion::
RetryPolicy` vs. `paladin_core::platform::container::aegis::RetryPolicy`) and cite the module
before touching a call site.

### Anti-Pattern 2: Assuming `build_chain`/`TokenBudget` is already wired into the run path

**What people do:** plan the Treasurer's per-draw enforcement as "extend the existing
`TokenBudget` wiring."
**Why it's wrong:** there is no existing wiring to extend — `build_chain` has zero production
callers, and the primary v0.10.0 run path (`WarEngine`/`NodeSpec::Paladin`) never touches
`PaladinExecutionService`'s middleware chain at all.
**Do this instead:** treat "wire `AgentRuntimeConfig::build_chain` (or an equivalent) into a real
run path for the first time" as first-class, estimated work inside this milestone, not a given.

### Anti-Pattern 3: Adding a `tenant_id` field silently, without a MIGRATION.md row

**What people do:** bolt a `tenant_id: String` onto `Principal` or `SubmitRun` and move on.
**Why it's wrong:** `Principal` is a public type without an observed `#[non_exhaustive]` marker;
widening it is a breaking change under this project's own semver discipline (X-10), and the
milestone's own scope text explicitly calls for a recorded X-03 supersession plus MIGRATION.md
rows for its breaking changes.
**Do this instead:** scope the `Principal`/auth-config change explicitly, run
`cargo semver-checks`, and add the MIGRATION.md §9.2 row alongside the legacy removals' rows.

## Integration Points Summary

| Boundary | Communication | File(s) | Notes |
|---|---|---|---|
| `paladin-web` auth → `RunSubmissionService::submit` | `requested_by: Option<(String, UserRole)>` parameter, already threaded end-to-end | `crates/paladin-web/src/agent_auth.rs`, `src/application/services/run/submission.rs:221-235` | Extend, don't replace, this existing channel for tenant/key identity |
| `RunSubmissionService` ↔ Treasurer | New in-process call before `repository.insert` | `submission.rs:240-311` | Same shape as the existing SSRF guard call |
| Treasurer ↔ `TreasuryLedgerPort` | New port, `Arc<dyn TreasuryLedgerPort>` injected, `Send + Sync` async trait mirroring `RunRepositoryPort` | `paladin-ports/src/output/treasury_ledger_port.rs` (new) | See §1, §6 |
| Treasurer ↔ engine/agent-loop (per-draw halt) | **Undecided** — either `AgentRuntimeConfig::build_chain` (agent-loop only) or a new engine-level hook (`WarEngine`/Aegis-adjacent) | TBD at plan time | The single largest open architectural question this research surfaces (§3) |
| Pacer ↔ `LlmPort` | New decorator adapter, composed alongside `FallbackLlmAdapter` | `crates/paladin-llm/src/services/pacer.rs` (new) or `paladin-llm/src/pacing/` | Wraps, never replaces, existing adapters (§7) |
| SSE `done` event ↔ `RunStatus` | Existing mapping site (location not re-derived in this pass — the D-14 bug is already filed against it) | Wherever `RunEventBusSink`/SSE terminal-event construction reads `RunStatus` | Fix alongside the Treasurer halt (§4) since both consume `RunStatus::Halted` |

## Sources

- Direct `Read`/`Grep` of the shipped tree at the 2026-09-24 HEAD (post-v0.10.1), specifically:
  `src/config/agent_runtime.rs`, `src/application/services/paladin/middleware/limits.rs`,
  `src/application/services/run/{submission,worker,resolver}.rs`,
  `crates/paladin-web/src/{agent_auth,run_controller}.rs`,
  `crates/paladin-core/src/platform/container/{herald,run,battalion/mod,node_error,paladin_error}.rs`,
  `crates/paladin-ports/src/output/{paladin_port,llm_port,run_repository_port}.rs`,
  `crates/paladin-battalion/src/{retry,llm_failure,engine/*}.rs`,
  `crates/paladin-llm/src/{http_status,services/commissary}.rs`, and the real
  `crates/paladin-storage/migrations/{postgres,sqlite}/` directory listing.
- `.planning/PROJECT.md` (Current Milestone section, v0.10.0 closing narrative, X-03 precedent from
  Phases 30-33).
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` (requirements R1-R6,
  scope boundary in §4).
- `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md` (dated 2026-07-30 —
  **stale on crate count and migrations path**; this research corrects the migrations location
  claim against the live filesystem).

---
*Architecture research for: v0.11.0 Treasurer Spend Governance*
*Researched: 2026-09-24*
