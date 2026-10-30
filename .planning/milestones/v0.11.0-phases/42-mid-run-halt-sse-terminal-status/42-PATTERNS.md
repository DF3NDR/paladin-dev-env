# Phase 42: Mid-Run Halt & SSE Terminal Status - Pattern Map

**Mapped:** 2026-10-06
**Files analyzed:** 27 new/modified (grouped by area)
**Analogs found:** 26 / 27 (one has no analog: vocabulary guard test)

Line numbers were read this session and are anchors, not contracts; re-check if Phase 43 lands first.
Resolutions G1-G17 from 42-RESEARCH.md are assumed adopted; this file only says WHAT TO COPY.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/paladin-ports/src/output/spend_guard.rs` (new) | port | request-response | `crates/paladin-ports/src/output/cancellation_probe.rs` | exact (placement), differs: fallible decision |
| `crates/paladin-core/src/platform/container/allowance.rs` (+`HaltReason`, `Eq` on `AllowanceRefusal`) | model | transform | same file, `AllowanceRefusal` (L113-128) | exact |
| `crates/paladin-core/src/platform/container/trace.rs` (`Cancelled`, `RunFinished.halt_reason`) | model | event-driven | same file, `RunFinishStatus` (L159-170) and the `cost` field on `RunFinished` | exact |
| `crates/paladin-core/src/platform/container/execution_result.rs` (`StopReason::AllowanceHalted`) | model | transform | same file, `StopReason::TokenBudget` (L154-174) | exact |
| `crates/paladin-core/src/platform/container/run.rs` (`Run.halt_reason`, `RunEventKind::AllowanceHalted`) | model | CRUD | same file; `final_waypoint_id` / `AllowanceWarning` event kind | exact |
| `crates/paladin-core/src/platform/container/run_scope.rs` (+budget carrier) | model | request-response | same file, `allowance_warnings` + `with_allowance_warnings` (L100, L193) | exact |
| `crates/paladin-battalion/src/engine/mod.rs` (`RunOutcome::Halted{cause}`, `with_spend_guard`, `run_finish_status`, `spend_guard_tests`) | service | event-driven | same file: `with_cancellation_probe` (L1843), `cancellation_probe_tests` (L6598) | exact |
| `crates/paladin-battalion/src/engine/superstep.rs` (guard at top-of-loop, `ChildEngineResources`) | service | event-driven | same file L2217-2244 | exact |
| `crates/paladin-storage/migrations/{sqlite,postgres}/013_add_run_halt_reason.sql` | migration | CRUD | `012_add_treasury_ledger_tenant_index.sql`, `011_create_treasury_notices.sql` | role-match |
| `crates/paladin-storage/src/run/{in_memory,sqlite,postgres}.rs` | repository | CRUD | same files: `final_waypoint_id` handling (sqlite L48-87, L284-335, L396, L479-483) | exact |
| `crates/paladin-storage/src/run/contract_tests.rs` (halt_reason clauses) | test | CRUD | same file, `insert_then_get_round_trips_every_field` (L87) | exact |
| `crates/paladin-storage/src/treasury/*` + `notice_contract_tests.rs` (`notice_kind`) | repository | CRUD | `011_create_treasury_notices.sql` + existing `NOTICE_INSERT` | exact |
| `src/application/services/treasurer/{mod,evaluate,guard,derive}.rs` | service | request-response | `treasurer/mod.rs` `admit` (L279-366), `policy.rs` | exact (factor, do not fork) |
| `src/application/services/run/worker.rs` (`map_outcome`, attach guard, `run_agent`, G14 order) | service | event-driven | same file L458-515, L1062-1090, L1236-1282 | exact |
| `src/application/services/run/cancel.rs` (`PerRunCancelProbe`, G1) | adapter | request-response | `DbCancellationProbe` in same file | exact |
| `src/application/services/run/events.rs` (`map_trace_event`, `terminal_payload`, `replay_stream`) | service | streaming | same file L206-231, L453-484, L627-720 | exact |
| `src/application/services/run/webhook/{mod,service}.rs` (+`halt_reason` key, `AllowanceHalted` event) | service | event-driven | `service.rs` L240 operator branch; `allowance_warning_payload_*` tests | exact |
| `src/infrastructure/telemetry/herald_sink.rs` + `crates/paladin-herald/src/{markdown,json,table}_herald.rs` | adapter | event-driven | same file's `AllowanceWarning` fold (L19-20, L50, L88) | exact |
| `src/application/services/paladin/middleware/limits.rs` (`TokenBudget` Treasurer-aware) | middleware | request-response | same file L146-171 | exact |
| `src/application/services/paladin/paladin_execution_service.rs` (scope -> `ModelCallContext`) | service | request-response | same file `execute_scoped`, `settle_agent_loop_call` | exact |
| `src/infrastructure/web/{run_api_wiring,agent_host,facade_provisioner}.rs` | config/wiring | request-response | `run_api_wiring.rs` L745-843 | exact |
| `crates/paladin-web/src/{run_controller,agent_controller,error}.rs` | controller | request-response | `agent_controller.rs` `stop_reason_label` (L216); `error.rs` 429 builder (L180-195) | exact |
| `crates/paladin-web/tests/openapi_golden_v0_9.rs` | test | transform | same file L306-326 (Phase 31/39/41 exception blocks) | exact |
| `src/application/services/run/http_surface_tests.rs` | test | request-response | existing end-to-end tests in same file | exact |
| `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `WINDOWS.md`, `CHANGELOG.md`, docs | config/docs | n/a | rows from Phases 39/41 (MIGRATION.md 207-210 `cost` field precedent) | exact |
| `.planning/decisions/0057-*.md`, `PROMOTION.md` | docs | n/a | `0056-allowance-admission-model.md` | exact |
| `tests/treasurer_vocabulary_guard.rs` (new) | test | file-I/O | none | no analog |

## Pattern Assignments

### `crates/paladin-ports/src/output/spend_guard.rs` (port, request-response)

**Analog:** `crates/paladin-ports/src/output/cancellation_probe.rs`

Copy the module-doc structure (why a port, policy-in-adapter/mechanism-in-engine, Thread Safety), the imports and the `#[async_trait]` trait shape plus a `Never*` default impl. Deviation: return a decision, not a `bool` (D-03).

**Imports + trait shape** (L36-65):
```rust
use async_trait::async_trait;
use paladin_core::platform::container::waypoint::ThreadId;

#[async_trait]
pub trait CancellationProbe: Send + Sync {
    async fn is_cancelled(&self, thread: &ThreadId) -> bool;
}
```
New shape (from RESEARCH Pattern 1): `async fn check(&self, thread: &ThreadId) -> SpendDecision;` with `#[non_exhaustive] enum SpendDecision { Continue, Halt(HaltReason) }`. Ship a doctest (public API needs doc tests) and register the module in the ports `output/mod.rs` the way `cancellation_probe` is registered.

---

### `crates/paladin-core/.../allowance.rs` (`HaltReason`)

**Analog:** `AllowanceRefusal`, L100-128 (derive list, field docs, `retry_after_secs`, doctest style).
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]   // add Eq (G17)
pub struct AllowanceRefusal { scope_kind, limit_kind, balance: Cost, ceiling: Cost,
    window: Option<(DateTime<Utc>, DateTime<Utc>)>, evaluated_at: DateTime<Utc> }
```
`HaltReason`: `#[serde(tag = "reason", rename_all = "snake_case")] #[non_exhaustive]`; variants `AllowanceExhausted(figures)` | `LedgerUnavailable`. Discriminator is `reason`, NOT `kind` (G6: `kind` is the limit kind). One wire builder shared by web, events, webhook. The 429 body at `crates/paladin-web/src/error.rs:180-195` is the field-set to stay byte-compatible with:
```rust
.with_details(json!({
    "scope": refusal.scope_kind.as_str(),
    "kind": refusal.limit_kind.as_str(),
    "balance": format_cost(&refusal.balance),
    "ceiling": format_cost(&refusal.ceiling),
    "window_start": window_start, "window_end": window_end }))
```

---

### `trace.rs` (`RunFinishStatus` + `RunFinished.halt_reason`)

**Analog:** same file L159-170 and the `cost` additive field on `RunFinished` (MIGRATION.md rows ~208-210).
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFinishStatus { Completed, Failed, Halted, AwaitingInput }
```
Add `Cancelled` and `#[non_exhaustive]`; keep it a unit enum (G4). Reason goes on the event: `#[serde(default, skip_serializing_if = "Option::is_none")] halt_reason: Option<HaltReason>`. `TraceRecord` has hand-written Serialize/Deserialize (commit `bc9cdf0`); add the round-trip to its `run_trace` contract tests. Known exhaustive-match sites: `events.rs:208-211`, `herald.rs:757`, `paladin-eval/src/assertion.rs:423-430`.

---

### `execution_result.rs` (`StopReason`)

**Analog:** same file L154-174 (`TokenBudget` variant doc style; enum already `#[non_exhaustive]`, derives `Eq`).
Add `AllowanceHalted(HaltReason-or-AllowanceRefusal)`; update `is_limit()` true / `is_successful()` false (see impls just below L176). Extend `stop_reason_label` (`agent_controller.rs:216-226`, has a `_ => "unknown"` arm) and `stop_reason_labels_are_stable` (L2715-2727).

---

### `run_scope.rs` (budget carrier)

**Analog:** `allowance_warnings` field (L100) and `with_allowance_warnings` builder (L193); test `run_scope_allowance_warnings_default_empty_omitted_and_round_trip` (L216). Struct is `#[non_exhaustive]` with `Default`; new field is `#[serde(default, skip_serializing_if = ...)]` and set only via a `with_*` builder with a doctest.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (boundary check)

**Analog:** same file L2217-2244 (existing top-of-loop check).
```rust
let token_cancelled = cancellation.as_ref().is_some_and(CancellationToken::is_cancelled);
let probe_cancelled = match probe { Some(p) => p.is_cancelled(&thread).await, None => false };
if token_cancelled || probe_cancelled {
    let waypoint = build_waypoint(&thread, parent_waypoint_id, superstep_number, graph,
        &battlefield, vanguard.clone(), Vec::new(), WaypointStatus::Halted, visit_counts,
        frontier.snapshot(graph), None, checkpoint_ns.clone(), fork_of);
    persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
    return Ok(RunOutcome::Halted { waypoint: waypoint.waypoint_id });
}
```
Extend into a `cause` (priority: probe cancel > token > guard) per RESEARCH Pattern 2; guard consulted only when attached. Also: second `Halted` constructor near L3826; nested-child `Ok(RunOutcome::Halted { .. })` arm near L1511-1523; `ChildEngineResources` must carry the guard (G11: memoise first Halt in `Arc<OnceLock>`). Other `RunOutcome::Halted` sites needing `{ waypoint, cause }`/`..`: `engine/mod.rs` L372 and tests L6445/6648/6810, `paladin-eval/src/runner.rs:935`, `examples/graceful_shutdown.rs:236,294`, `worker.rs` tests.

### `engine/mod.rs`

**Analogs:** `RunOutcome` (L305-340), `run_finish_status` (L368-375), field + builder `cancellation_probe` (L1495, L1843), four `trace.emit(TraceEvent::RunFinished{..})` sites.
```rust
Ok(RunOutcome::Halted { .. }) => RunFinishStatus::Halted,   // becomes (status, halt_reason) from cause
```
Add `with_spend_guard(Arc<dyn SpendGuard>)` mirroring L1843.

**Test analog** `cancellation_probe_tests` (L6598-6670): `CountingProbe` (AtomicUsize, halt-from-call-N), `NeverCancellingProbe`, `four_node_chain_graph()`, `RecordingWaypointStore`, `ascending_history`, `tokio::time::timeout(5s)`. Copy for `spend_guard_tests`: guard Halt at boundary 2 yields Halted Waypoint with `vanguard == [ids[1]]` and cause Spend; never-halting guard changes nothing; no guard = no check. Do not assert absolute superstep numbers across halt/fork (G13).

---

### Migration `013_add_run_halt_reason.sql` (both backends)

**Analog:** `012_*.sql` header style (Migration/Purpose/Version/Date comment block, idempotent), `011_create_treasury_notices.sql` for the notices table.
Contents: `ALTER TABLE runs ADD COLUMN halt_reason TEXT NULL`; `ALTER TABLE treasury_notices ADD COLUMN notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning','halt'))`; drop/recreate `idx_treasury_notices_once` (current key: `scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos`) appending `notice_kind`. Update adapters' `NOTICE_INSERT ... ON CONFLICT (...)` arbiter list and the `notice_arbiter_matches_the_migration` tests; `notices_for_run` filters `notice_kind = 'warning'`. Check the existing `007`-`012` files in the postgres dir for dialect differences before writing.

### `crates/paladin-storage/src/run/{sqlite,postgres,in_memory}.rs`

**Analog:** how `final_waypoint_id` flows: SQL constants (sqlite L48-87, four constants; postgres `:56-92`), row mapper (`sqlite.rs:284-335`, `postgres.rs:250-320`), insert bind (L396, L758), `record_outcome`:
```rust
"UPDATE runs SET error = ?, output = ?, final_waypoint_id = ? WHERE run_id = ?"
```
Add `halt_reason` (serialised JSON text, `None` on legacy rows) to every SELECT/INSERT and to `record_outcome`/`RunOutcomeRecord` (in `paladin-ports` `run_repository_port.rs`). `record_outcome` has no status guard, so the worker may call it BEFORE `update_status` (G14).

### `crates/paladin-storage/src/run/contract_tests.rs`

**Analog:** `insert_then_get_round_trips_every_field` (L87) and `update_status_*` (L134+): free `pub async fn clause(port: &dyn RunRepositoryPort)`, run by all three adapters. Add halt_reason round-trip, legacy `None`, and record-before-status clauses; register in each adapter's test harness.

---

### `src/application/services/treasurer/` (shared evaluation, guard impl, derivation)

**Analog:** `treasurer/mod.rs` `admit` (L279-366): `policy.ceilings_for(subject)` -> empty returns `Admission::none()` (no ledger read) -> single `store_now()` truncated to whole seconds -> per ceiling `window_for(evaluated_at, period)` -> `BalanceQuery{..}` -> `ledger.balance(query)` -> `balance.nanos() >= ceiling.ceiling_nanos` is refusal -> `crosses_warn_threshold(..)` collects `Crossing`s claimed after all admit.
```rust
let query = BalanceQuery { tenant_id: ceiling.tenant_id.clone(), api_key_id: ceiling.api_key_id.clone(),
    currency: self.policy.currency().clone(), since: window.map(|(s, _)| s), until: window.map(|(_, e)| e) };
```
Factor this loop into `evaluate(subject)`; `admit`, guard `check`, and `derive_budget` all call it (D-17 one function per rule). Error mapping: backend failures use the `backend(..)` helper (L291) at admission; in the guard they become `SpendDecision::Halt(HaltReason::LedgerUnavailable)` plus `log::error!` (no key values). Logging style: `log::warn!("allowance refused: scope={} limit={} ...", ..)` using `format_cost` once. Notice claiming + operator webhook: reuse `with_notices` / `with_operator_webhook` code paths already in this file (`Crossing` at L286-291).
Derivation arithmetic: use `PriceRow` accessors (`cost.rs:321-342`) and `cost_of_call` defaults (`cost.rs:375-403`); `i128`; max over five axes (G16).

---

### `src/application/services/run/worker.rs`

**`map_outcome`** (L458-515) currently takes `(outcome, cancel_requested, shutting_down)` and for `Halted` checks cancel_requested -> `Cancelled`, else shutting_down -> `LeaveRunningAndRequeue`, else `Halted`; every arm builds `OutcomeAction::Transition { to, outcome: RunOutcomeRecord { error, output, final_waypoint_id } }`. Extend `RunOutcomeRecord` with `halt_reason` and map cause: `CancelRequested` -> Cancelled; `Spend/LedgerUnavailable` -> Halted + reason (error stays `None`); `Token` -> keep existing cancel/shutdown/halted re-query (G1a). Test analogs: `map_outcome_halted_with_cancel_requested_transitions_to_cancelled` (L1942), `..._while_shutting_down_leaves_running_and_requeues` (L1960), `..._otherwise_transitions_to_halted` (L1968): add one row per new cause.

**Attach guard** (L1062-1090), copy the shape:
```rust
let mut engine = factory(child_token);
if let Some(probe) = &self.cancellation_probe { engine = engine.with_cancellation_probe(Arc::clone(probe)); }
if let Some(ledger) = &self.treasury_ledger {
    engine = engine.with_treasury_ledger(Arc::clone(ledger),
        SettlementContext { scope: LedgerScope::from_attribution(run.submitted_by.as_ref()),
                            run_id: run.run_id.clone(), attempt });
}
```
Add `if let (Some(t), Some(attr)) = (&self.treasurer, &run.submitted_by) { engine = engine.with_spend_guard(..) }` (None attribution = no guard). Pool builder: add `with_treasurer` beside `with_treasury_ledger` (L840) and `with_treasury_notices`.

**Transition write order** (L1258-1278): today `update_status` -> `record_outcome` -> `ack` -> `enqueue_webhook_delivery`. For halting outcomes swap to `record_outcome` first (G14). Delete/replace the D-14 comment block (L1239-1256) that documents the "halted for cancelled" simplification.

**`run_agent`** (L1349+; status writes at ~L1468-1488): map `StopReason::AllowanceHalted` -> `RunStatus::Halted` + `halt_reason`; else Completed as today. `enqueue_webhook_delivery` (L1649) gains the `halt_reason` key and the `allowance_halted` operator enqueue.

### `src/application/services/run/cancel.rs`

**Analog:** `DbCancellationProbe` (same file, debounced probe, swallow-errors-to-false). `PerRunCancelProbe { db, local, shutdown }` answers `db.is_cancelled(t) || (local.is_cancelled() && !shutdown.is_cancelled())` (G1b). Built in worker L1062-1075 where `child_token` exists.

### `src/application/services/run/events.rs`

**`map_trace_event`** (L206-231): currently
```rust
RunFinishStatus::Completed => (Done, "completed"),
RunFinishStatus::Halted => (Done, "halted"),
RunFinishStatus::AwaitingInput => (Done, "awaiting_input"),
RunFinishStatus::Failed => (Error, "failed"),
```
payload `json!({"status", "waypoint_id": Null, "usage", "trace_seq"})`. Add `Cancelled => (Done, "cancelled")` and, when the event carries `halt_reason`, a `"halt_reason"` key built by the shared `HaltReason` wire builder. **`terminal_payload`** (L453-484): the `Halted` arm becomes `json!({"status":"halted","waypoint_id":run.final_waypoint_id, "halt_reason": run.halt_reason})`, same builder (byte-identical, D-16). **`replay_stream`** (L627-720): on a mapped `RunFinished` fetch the row and emit `terminal_payload(&run)` (G10). Drain suppression (D-15, G1c): `RunEventBusSink` drops `RunFinished{Halted}` with no reason while the shutdown token is cancelled. Enumeration test analog: `map_trace_event_covers_exactly_seven_of_twelve` (L922) and the `RunFinished must always map` test (L1102); add the new status rows.

### `webhook/service.rs`, `webhook/mod.rs`

**Analog:** operator branch at `service.rs:240`:
```rust
let signing_key = if delivery.event == RunEventKind::AllowanceWarning {
    self.operator_notice_secret.clone().unwrap_or_default()
} else { match self.runs.get(&delivery.run_id).await { .. } }
```
Change to `matches!(delivery.event, AllowanceWarning | AllowanceHalted)`; adapters' event-string parse (`webhook/sqlite.rs:62-74`, `webhook/postgres.rs:61`) gets the new arm. Payload: reuse the twelve-key `AllowanceWarningPayload` with `event` set (no thirteenth key); add contract clause beside `webhook contract_tests.rs:300-340`; key-set test pattern: `allowance_warning_payload_*` in `webhook/mod.rs`. Caller `Halted` payload gets optional `halt_reason`, signing unchanged (sign the exact stored bytes; see security.instructions.md).

### `herald_sink.rs` and heralds

**Analog:** `HeraldTraceSink` folds `AllowanceWarning` events into metadata at `RunFinished` (L19-20, L50, L69, L88; tests L227-284 incl. `herald_sink_without_warnings_renders_as_before`). Fold `RunFinished.halt_reason` into `ExecutionMetadata` (see `ExecutionMetadata::with_allowance_warnings`) and render one line in markdown/json/table heralds. Note `herald.rs:757` compares `*status == RunFinishStatus::Failed`; keep working.

---

### `limits.rs` (`TokenBudget`)

**Analog:** same file L146-171:
```rust
if !self.config.enabled { return Ok(MiddlewareFlow::Continue); }
if cx.cumulative_tokens > self.config.max_tokens {
    resp.content.push_str(TOKEN_BUDGET_NOTICE);
    return Ok(MiddlewareFlow::Finish(FinalResult::new(resp.content.clone(), StopReason::TokenBudget)));
}
```
Extend: effective = min(operator if enabled, derived figure from the call context); record which won; Treasurer win returns `StopReason::AllowanceHalted(..)`. Per-run figure is read from the `ModelCallContext` (fed by `RunScope`), never stored on the struct (limits.rs D-03). Shared engine service gets Treasurer-only mode (G12: operator `enabled=false`). Test shape: existing `TokenBudget` unit tests in same file.

### `run_api_wiring.rs` (wiring)

**Analog:** L745-843. Current order: `RunWorkerPool::new(..).with_*` chain (L754-771), `with_treasury_ledger` (L790-792), `build_treasury_notices` + `with_treasury_notices` (L799-802), `Arc::new(pool)` + `spawn` (L804-806), then the Treasurer (L808+). Reorder so notices and the Treasurer are built before the pool, then `.with_treasurer(Arc<Treasurer>)` (G3); coerce to `Arc<dyn AllowanceAdmissionPort>` for the submission service and agent state.

### `crates/paladin-web` controllers

`error.rs:180-195` 429 builder (reuse for the unpriced-model refusal variant, D-10). `run_controller.rs` `RunResponse` (L397-455) gains `halt_reason: Option<..>` and `final_waypoint_id: Option<String>` (G7); `utoipa::ToSchema` derive like neighbours. `agent_controller.rs` `execute` responds `stop_reason: "allowance_halted"` plus `halt_reason`; `admit_principal` is the existing admission call site to extend with the model (G8). `jobs` handler gated per Discretion.

### `openapi_golden_v0_9.rs`

**Analog:** `strip_known_v0_10_execute_response_divergence` (L306-326) strips `token_count`, `usage`, `cost`; add `halt_reason` plus a "Phase 42 exception" doc block in the Phase 31/39/41 style; run `make openapi` to regenerate `crates/paladin-web/openapi.json` (G9).

### Registers/docs

Copy row format from the existing Phase 39/41 rows in `MIGRATION.md` §9.2/§9.4/§9.6, `.cargo/semver-checks-allowlist.toml` (`requirement_id = "ALLOW-03"` / `"PLAT-09"`), `WINDOWS.md` (row 33 untouched; new rows for D-08, D-01 race, G2 streamed-call, G15 unpriced engine nodes). ADR-0057: copy structure of `.planning/decisions/0056-allowance-admission-model.md`; add a dated note to 0056; advance `PROMOTION.md` to 0058 in the same commit.

---

## Shared Patterns

### Policy in the facade, mechanism in the engine
**Source:** `cancellation_probe.rs` + `cancel.rs` `DbCancellationProbe`. **Apply to:** `spend_guard.rs`, `treasurer/guard.rs`, engine. `paladin-battalion` consults a port only; `None` attached = no check.

### Fail closed where a ceiling applies, inert where none does
**Source:** `treasurer/mod.rs:286-290` (`ceilings.is_empty()` -> `Admission::none()`, no ledger read). **Apply to:** guard `check`, `derive_budget`. Ledger error with a ceiling -> `LedgerUnavailable`; no ceiling -> `Continue`.

### Additive serde fields on non-exhaustive types
**Source:** `RunScope` (`#[serde(default)]`, `with_*` builder), `cost` on `RunFinished`. **Apply to:** `Run.halt_reason`, `RunFinished.halt_reason`, `RunScope` carrier, `RunResponse`.

### One wire builder per type, money formatted once
**Source:** `error.rs:184-191` (`format_cost` once at the edge). **Apply to:** `HaltReason` JSON used by 429-compatible details, `GET /runs`, SSE `done`, webhook key, agent response.

### Store-enforced idempotency for notices
**Source:** `011_create_treasury_notices.sql` unique index + `INSERT ... ON CONFLICT DO NOTHING`. **Apply to:** halt notice (`notice_kind='halt'`), once per window per ceiling.

### Credential hygiene
**Source:** `.github/instructions/security.instructions.md`. No API key in logs, trace, webhook payload, herald line; webhook SSRF + no-redirect + sign-stored-bytes unchanged.

### Test layout
Unit tests in `#[cfg(test)] mod` beside code; shared adapter contract clauses in `crates/paladin-storage/src/run/contract_tests.rs`; root integration tests under `tests/` (auto-discovered); TDD red test first.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `tests/treasurer_vocabulary_guard.rs` | test | file-I/O | No existing source-tree-scanning guard test found for a reserved word; plan from RESEARCH Assumption A1 (`std::fs` recursive walk, path allowlist, no new crates). Worth a quick `Grep` for existing grep-style guard tests in `tests/` before writing. |

## Metadata

**Analog search scope:** `crates/paladin-{ports,core,battalion,storage,web,herald}/src`, `src/application/services/{run,treasurer,paladin}`, `src/infrastructure/{web,telemetry}`, storage migrations
**Files read (targeted ranges):** ~14; remaining line anchors taken from 42-RESEARCH.md (read in full) and not re-verified: `paladin_execution_service.rs` execute path, `facade_provisioner.rs`, `agent_host.rs`, `herald.rs:757`, postgres adapters.
**Pattern extraction date:** 2026-10-06
