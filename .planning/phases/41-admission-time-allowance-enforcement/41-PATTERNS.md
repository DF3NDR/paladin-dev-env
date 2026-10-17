# Phase 41: Admission-Time Allowance Enforcement - Pattern Map

**Mapped:** 2026-10-02
**Files analyzed:** 36 new/modified (grouped where one analog covers several)
**Analogs found:** 34 / 36 (2 partial; see "No Analog Found")

Line numbers were read this session and may drift (RESEARCH validity note: Phases 42-44 touch the same files).
Where RESEARCH.md "C-numbered corrections" change how a CONTEXT decision is implemented, they are cited as `C#` and they win over CONTEXT wording.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `src/config/treasurer.rs` (add `AllowanceConfig`, entry, webhook, period/amount parsers, `resolve`, `validate_against`) | config | transform | same file (`TreasurerConfig`, `parse_price_nanos_per_million`) | exact |
| `src/application/services/treasurer/{mod,policy,window,tests}.rs` (`Treasurer`) | service | request-response (read + claim) | `RunSubmissionService` SSRF slot + `ledger.reserve` flow in `sqlite.rs` | role-match |
| `crates/paladin-core/src/platform/container/allowance.rs` (`AllowanceRefusal`, `AllowanceWarning`, `Admission`) | model (value objects) | transform | `treasury_ledger.rs` (`LedgerScope`, `ReserveRequest`, `format_cost`) | exact |
| `crates/paladin-core/.../treasury_ledger.rs` (+`BalanceQuery`) | model | CRUD | `SpendQuery` / `ReserveRequest` in same file | exact |
| `crates/paladin-core/.../trace.rs` (+`TraceEvent::AllowanceWarning`) | model | event-driven | `TraceEvent::MiddlewareEvent` (trace.rs 349-354, test 467-493) | exact |
| `crates/paladin-core/.../herald.rs` (+`allowance_warning_display`) | utility | transform | `ExecutionMetadata::cost_display` | exact |
| `crates/paladin-core/.../run_schedule.rs` (+`created_by`) | model | CRUD | `Run.submitted_by` / `with_submitted_by` (run.rs 417, 462) | exact |
| `crates/paladin-core/.../run.rs` (`RunEventKind::AllowanceWarning`, `#[non_exhaustive]`) | model | event-driven | `StopReason` non_exhaustive precedent (C3) | role-match |
| `crates/paladin-ports/src/output/treasury_ledger_port.rs` (+`balance`) | port | request-response | same trait (`spend`, `store_now`) | exact |
| `crates/paladin-ports/src/output/treasury_notice_port.rs` (new) | port | CRUD (idempotent claim) | `TreasuryLedgerPort::settle` / `SettleOutcome` | exact |
| `crates/paladin-ports/src/input/allowance_admission_port.rs` (new) | port | request-response | `RunSubmissionPort` (input port consumed by web) | role-match |
| `crates/paladin-ports/src/input/run_submission_port.rs` (`SubmitRun.attributed_to`, `AllowanceExhausted`) | port | request-response | same file (Phase 40 `requested_by` addition) | exact |
| `crates/paladin-ports/src/input/schedule_admin_port.rs` (`CreateRunSchedule.created_by`) | port | CRUD | same file | exact |
| `crates/paladin-storage/src/treasury/{in_memory,sqlite,postgres}.rs` (`balance`, notices) | adapter | CRUD | `sqlite.rs` `reserve` (probe+SUM) and `spend` (QueryBuilder) | exact |
| `crates/paladin-storage/src/treasury/contract_tests.rs` (+ notice clauses) | test | CRUD | `spend_window_is_half_open` (contract_tests.rs 1184) | exact |
| `crates/paladin-storage/migrations/{sqlite,postgres}/010_add_run_schedule_created_by.sql` | migration | CRUD | `008_add_run_attribution_columns.sql` | exact |
| `crates/paladin-storage/migrations/{sqlite,postgres}/011_create_treasury_notices.sql` | migration | CRUD | `007_create_treasury_ledger_table.sql` | exact |
| `crates/paladin-storage/src/run_schedule/*` + contract tests (`created_by` round-trip) | adapter | CRUD | Phase 40 run attribution columns in run adapters | role-match |
| `crates/paladin-storage/src/webhook/*` (`event_from_str` for new kind) | adapter | CRUD | `crates/paladin-storage/src/webhook/sqlite.rs` 64-82 | exact |
| `src/application/services/run/submission.rs` (`with_treasurer`, admit in `submit`/`fork`) | service | request-response | same file (`with_ssrf_guard`, SSRF check lines 289-298) | exact |
| `src/application/services/run/worker.rs` (readback + emit on `Queued` arm) | service | event-driven | existing `run_once` emitter setup (C6) | role-match |
| `src/application/services/run/schedule/service.rs` (fire site attribution, `SkipReason::AllowanceExhausted`) | service | batch | same file, lines 341-348 | exact |
| `src/application/services/run/webhook/{mod,service}.rs` (operator payload + secret branch) | service | event-driven | `WebhookPayload` + `WebhookDeliveryService::process` | exact |
| `src/infrastructure/telemetry/herald_sink.rs` (stateful fold) | adapter | event-driven | same file (`on_event`, 66-84) | role-match |
| `crates/paladin-herald/src/{markdown,json,table}_herald.rs` | adapter | transform | `cost_display()` call at markdown_herald.rs 422 | exact |
| `src/infrastructure/web/run_api_wiring.rs` (`build_treasurer`, `RunApiHandles.treasurer`) | config/wiring | request-response | `build_treasury_ledger` / `build_run_api` | exact |
| `src/bin/paladin-server.rs` (attach to `AgentApiState`) | wiring | request-response | `build_auth_config` voice | role-match |
| `crates/paladin-web/src/error.rs` (`with_retry_after`, helper) | utility | request-response | same file (`with_details`, `too_many_requests`) | exact |
| `crates/paladin-web/src/run_controller.rs` (`map_submission_error` arm) | controller | request-response | same function, lines 688-731 | exact |
| `crates/paladin-web/src/agent_controller.rs` (gate execute, stream, **jobs**) | controller | request-response | same file lines 301-321, 614-633, 678-703 | exact |
| `crates/paladin-web/src/schedule_controller.rs` (stamp `created_by`) | controller | CRUD | same file `create_schedule` 366-391 | exact |
| `crates/paladin-web/tests/openapi_golden_v0_9.rs` (429 exception) | test | transform | existing strip functions (C9) | exact |
| `config.example.yml`, docs, `MIGRATION.md`, `CHANGELOG.md`, ADR-0056, `.cargo/semver-checks-allowlist.toml` | docs/config | n/a | Phase 39/40 equivalents | role-match |
| `src/application/services/run/http_surface_tests.rs` (e2e 429) | test | request-response | `tenant_scoped_run_read_tracer` | exact |
| `crates/paladin-web/src/agent_auth.rs` (export open-access constants, optional) | utility | transform | `Principal::ledger_scope` (88-90) | role-match |

## Pattern Assignments

### `src/config/treasurer.rs` -> `AllowanceConfig` (config, transform)

**Analog:** same file.

**Grammar parser to factor, not fork** (lines 53-104). It already scales by 1e9, so `"25.00"` yields `25_000_000_000`. Rename or wrap it (`parse_decimal_nanos`) and keep `PriceParseError`. It accepts `"0"`, so the allowance `validate()` must reject zero explicitly.
```rust
fn parse_price_nanos_per_million(raw: &str) -> Result<i64, PriceParseError> {
    let mut parts = raw.splitn(2, '.');
    ...
    let scaled_int = int_value.checked_mul(1_000_000_000).ok_or(PriceParseError::Overflow)?;
    scaled_int.checked_add(frac_value).ok_or(PriceParseError::Overflow)
}
```

**Strict-struct pattern** (lines 114-130): `#[serde(deny_unknown_fields)]` on the entry and webhook structs (Pitfall 17). Optional fields use `#[serde(default)] Option<String>`.

**Resolve + validate pair** (lines 194-252): build once, validate is the discarded result, and every error names the full path.
```rust
pub fn validate(&self) -> Result<(), String> { self.price_table().map(|_| ()) }
// error voice:
"treasurer.pricing.{model}.{axis} must be a non-negative decimal string ... (got {raw:?})"
```
Mirror this as `treasurer.allowance.tenants.{id}.amount`. Add `resolve(&self, currency) -> Result<AllowancePolicy, String>`, and `validate() = resolve().map(|_| ())`.

**Config-file-only collections + one scalar env override** (lines 272-279). Add `APP_TREASURER_ALLOWANCE_WARN_AT` and, per C7, `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`:
```rust
impl EnvOverridable for TreasurerConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_TREASURER_CURRENCY") { self.currency = v; }
    }
}
```

**Struct-level default** (lines 159-179): `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] #[serde(default)]` plus a manual `Default`. Add `pub allowance: AllowanceConfig` (default inert). `TreasurerConfig` derives `Debug` and `Serialize`, so the webhook secret needs a manual redacting `Debug` (copy the `WebhookSpec` pattern from `webhook.rs`) and `#[serde(skip_serializing)]` (Pitfall 18).

**Test pattern:** `#[serial]` env tests as in `env_override_currency`; the `Wrapper` + `config::Config::builder` YAML load helper at ~lines 298+.

---

### `src/application/services/treasurer/` -> `Treasurer` (service, request-response)

**Analog:** `RunSubmissionService` for module shape and builder; `sqlite.rs` `reserve` for the probe-then-SUM semantics.

**Port-first imports** (the `submission.rs` 10-29 style: `paladin_core` + `paladin_ports` only, never storage):
```rust
use std::sync::Arc;
use async_trait::async_trait;
use paladin_core::platform::container::principal::RunAttribution;
use paladin_ports::output::treasury_ledger_port::{TreasuryLedgerError, TreasuryLedgerPort};
```

**Admission loop and window math:** copy RESEARCH.md Code Examples 1 and 2 verbatim. The `window_for` function is pure `div_euclid` arithmetic on `store_now()`. Use short-circuit refusal and collect crossings only on the admitted path. Warn math is `i128::from(balance) * 100 >= i128::from(ceiling) * i128::from(warn_at)`.

**Take `&RunAttribution`, not `PrincipalRef`** (RESEARCH Anti-Patterns, D-09, C13). `PrincipalRef::attribution()` is at `principal.rs` 190.

**Claim-before-insert / `confirm` / `abandon`** (RESEARCH Pattern 3). `Treasurer` implements `AllowanceAdmissionPort` (C1: `paladin-web` cannot name the facade type).

**Fail-closed mapping** (D-10): any `TreasuryLedgerError` on an allowanced principal becomes `AdmissionError::Backend` -> `RunSubmissionError::Backend { message }`. Use the existing `map_queue_error` shape (`submission.rs` 82-86).

**Test pattern:** scripted `FakeLedger` with settable `now: Mutex<DateTime<Utc>>` (RESEARCH "How the in-memory path simulates `store_now()`"). There is no injectable clock on `InMemoryTreasuryLedger`.

---

### `crates/paladin-core/.../allowance.rs` (model, transform)

**Analog:** `crates/paladin-core/src/platform/container/treasury_ledger.rs`.

Follow `LedgerScope` (`from_attribution` at line 113) and the `ReserveRequest` window-bounds shape. Derive `Debug, Clone, PartialEq, Serialize, Deserialize`. Add `Display` rendering the D-13 sentence and `evaluated_at: DateTime<Utc>` (C8). Use `format_cost` for display only.

**`BalanceQuery`** (C10, in `treasury_ledger.rs` beside `SpendQuery`):
```rust
BalanceQuery { tenant_id, api_key_id: Option<String>, currency: CurrencyCode,
               since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>> }  // lifetime = both None
```

---

### `TraceEvent::AllowanceWarning` (model, event-driven)

**Analog:** `trace.rs` `MiddlewareEvent` (lines 347-354).
```rust
    /// The facade's `ExecutionMiddleware` chain ... took an action ...
    MiddlewareEvent { name: String, action: MiddlewareAction },
}
```
The enum is `#[non_exhaustive]` and uses `#[serde(default)]` on new fields. **Two in-file edits are mandatory** (Pitfall 15): the exhaustive no-wildcard match at trace.rs ~480-493 (add `TraceEvent::AllowanceWarning { .. } => "allowance_warning"`), and `assert_eq!(events.len(), 12)` at line 472 (-> 13, add a sample to the vec). Every other workspace match already has a wildcard.

---

### `herald.rs` `allowance_warning_display` + three heralds (utility, transform)

**Analog:** `ExecutionMetadata::cost_display()`, consumed at `crates/paladin-herald/src/markdown_herald.rs` 422-424:
```rust
if let Some(display) = metadata.cost_display() {
    output.push_str(&self.format_field("Cost", &display));
}
```
Add one `allowance_warning_display() -> Option<String>` beside `cost_display` and call it from all three renderers (markdown, json, table) the same way. Target line: `⚠ allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)`.

---

### `herald_sink.rs` (adapter, event-driven)

**Analog:** same file, `on_event` (lines 66-84):
```rust
async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
    let Some(metadata) = ExecutionMetadata::from_run_finished(&record, self.model_used.as_str())
    else { return Ok(()); };
    match self.herald.finalize_stream(&metadata) { ... }
}
```
The sink is currently stateless and acts only on `RunFinished`. Add `Mutex<Vec<AllowanceWarning>>` interior state, record on `AllowanceWarning`, and at `RunFinished` insert `metadata.metadata["treasurer.allowance_warning"]` before `finalize_stream` (RESEARCH Pattern 7). Tests use the existing `RecordingHerald` (lines 127-165).

---

### `TreasuryLedgerPort::balance` on three adapters (adapter, CRUD)

**Analog:** `crates/paladin-storage/src/treasury/sqlite.rs`.

**Probe then SUM** (lines 251-282). The foreign-currency probe runs first with the same predicates, a hit becomes `CurrencyMismatch`, and then the SUM runs:
```rust
let foreign: Option<String> = sqlx::query_scalar(FOREIGN_CURRENCY_QUERY)... .fetch_optional(&mut *tx)
if let Some(found) = foreign { ... return Err(TreasuryLedgerError::CurrencyMismatch { expected, found: found_currency }); }
let balance: i64 = sqlx::query_scalar(BALANCE_QUERY)... .fetch_one(&mut *tx)
```
Constants at lines 66-80. For `balance` the key and window predicates are optional, so use the `spend` shape (lines 522-540): a static `&'static str` prefix plus `QueryBuilder` pushes, never `? IS NULL OR ...`.
```rust
let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(SPEND_SELECT_PREFIX);
if let Some(since) = query.since {
    builder.push(" AND attributed_at >= ");
    builder.push_bind(crate::run::storage_timestamp(since));
}
```
Run probe and SUM in one read transaction. No `BEGIN IMMEDIATE` is needed, because it is a read. `wrap_error` (line 202) and `redact_database_url_password` handle error redaction. The Postgres twin mirrors line for line (Pitfall 19: Postgres is outside the local coverage gate, so keep it thin).

**Contract clauses:** copy the `spend_window_is_half_open` structure (contract_tests.rs 1184+: `contract_scope("...")` unique tenant, 20 ms sleep-bracketed `store_now()` samples). Each clause is a plain `pub async fn clause(port: &dyn TreasuryLedgerPort)` wired from `in_memory.rs`, `sqlite.rs` and `postgres.rs` `#[tokio::test]`s (`sqlite.rs` ~821-841 shows wiring). The first red test is `tenant_balance_equals_sum_of_key_balances`.

---

### `TreasuryNoticePort` + notices adapters (port/adapter, idempotent claim)

**Analog:** the settle idempotency path (`sqlite.rs` 54-60 `SETTLE_INSERT`, 494-516):
```rust
const SETTLE_INSERT: &str = "INSERT INTO treasury_ledger (...) VALUES (...) \
    ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING";
let result = sqlx::query(SETTLE_INSERT)...;
Ok(if result.rows_affected() == 0 { SettleOutcome::AlreadySettled } else { ... })
```
Map `rows_affected() == 0` to `NoticeOutcome::AlreadyRecorded`. The `ON CONFLICT` column list must textually match the unique index (the `settle_arbiter_predicate_matches_the_migration` test at sqlite.rs ~900 is the template for a `NOTICE_INSERT` text test). **C5 override of D-16:** `api_key_id TEXT NOT NULL` with `''` for tenant scope, since NULL in a UNIQUE key never conflicts. Implement both ports on the same adapter struct so one adapter yields two `Arc`s.

---

### Migrations `010` and `011` (migration, CRUD)

**`010` analog:** `migrations/sqlite/008_add_run_attribution_columns.sql`. Copy the header style (Migration / Purpose / Version / Date) and the two plain `ALTER TABLE ... ADD COLUMN ... NULL` lines:
```sql
ALTER TABLE runs ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE runs ADD COLUMN api_key_id TEXT NULL;
```
Target `run_schedules`. Postgres may add an all-or-none CHECK as `009` did (`postgres/009_add_run_attribution_check.sql`). C2: `009` exists on Postgres only, so SQLite goes `008 -> 010`. Document the gap in the `010` header. Fallback is a no-op SQLite `009`, proven by the first SQLite adapter test (A1).

**`011` analog:** `007_create_treasury_ledger_table.sql`. Copy the long header-comment discipline (explain every unique index and why the `ON CONFLICT` target must match it), `CREATE TABLE IF NOT EXISTS`, `CHECK (... IN (...))` enums, and `CREATE UNIQUE INDEX IF NOT EXISTS`. The full DDL is RESEARCH Code Example 4. It adds `window_end` and `warn_at` columns, which are a checkpoint item. Migrations are append-only (sqlx checksum).

---

### `src/application/services/run/submission.rs` (service, request-response)

**Analog:** same file.

**Builder** (lines 200-206), mirrored by `with_treasurer`:
```rust
pub fn with_ssrf_guard(mut self, ssrf_guard: SsrfGuard) -> Self {
    self.ssrf_guard = ssrf_guard;
    self
}
```
Add a field `treasurer: Option<Arc<dyn AllowanceAdmissionPort>>` initialised `None` in `new` (lines 181-188).

**Slot in `submit`** (lines 288-371). Order: SSRF (292-298) -> `resolve` (300-303) -> `authorize_invocation` (310) -> `ensure_thread_visible` (322-325) -> `RunId::new_v7()` (329) -> **admit here, after the id exists and before `insert_with_latest`/`insert` (344-356)** -> `enqueue` (357-365) -> `confirm`; on insert/enqueue error, `abandon`. Effective attribution is `requested_by.as_ref().map(|p| p.attribution())` else `request.attributed_to` (C13). Schedule-fired runs keep skipping `authorize_invocation` (written decision required by D-08).

**Error mapping** follows `map_repository_error` / `map_queue_error` (lines 68-86): local `fn map_admission_error(AdmissionError) -> RunSubmissionError`. Do not write `From` between foreign types (orphan-rule comment, lines 63-67).

**Test pattern:** `submit_with_a_loopback_webhook_url_is_rejected_and_touches_nothing` (~line 617). Seed with `ledger.settle(.. LedgerScope::new(tenant, key) ..)`, submit with `requested_by: Some(PrincipalRef::new(key, tenant, UserRole::User))`, then assert `AllowanceExhausted`, `queue.depth()==0`, and `repository.list(..)` empty.

**Struct-literal fallout (C13):** adding `attributed_to` to `SubmitRun` breaks ~6 `SubmitRun {` literals, including the doctest at submission.rs 135-143 (`requested_by: None,`) and `schedule/service.rs` 341-348.

---

### `src/application/services/run/schedule/service.rs` (service, batch)

**Analog:** fire site, lines 341-356.
```rust
let request = SubmitRun { assistant_id: ..., webhook: schedule.webhook.clone(), requested_by: None };
match self.submission.submit(request).await {
    Ok(accepted) => ScheduleTickOutcome::Fired { .. },
    Err(RunSubmissionError::ThreadBusy { .. }) => { ... }
```
Set `attributed_to: schedule.created_by.clone()` and keep `requested_by: None`. Add an `AllowanceExhausted` arm with `SkipReason::AllowanceExhausted` + `increment_skipped` (RESEARCH Open Q6).

---

### `webhook/{mod,service}.rs` operator notice (service, event-driven)

**Analog:** `WebhookPayload` (`webhook/mod.rs` 53-73). Use a new `AllowanceWarningPayload` beside it with the same derive set, the key set pinned as a prohibition boundary, and a `serializes_with_exactly_the_documented_keys` test modelled on `webhook_payload_tests` (mod.rs ~82+). Serialize once, store verbatim, sign and send from that buffer (D-41).

**Process branch:** `WebhookDeliveryService::process` (service.rs) loads the signing secret from `runs.get(delivery.run_id)`. Add `with_operator_notice_secret(Option<String>)` and branch on `event == RunEventKind::AllowanceWarning` before the run lookup (C3). Reuse `sign_webhook_body`, `SsrfGuard` and `build_webhook_client` unchanged. Row construction: `WebhookDelivery::new(delivery_id, correlation_run_id, ThreadId("treasurer-notices"), RunEventKind::AllowanceWarning, url, payload_json, now)`. Storage `event_from_str` (`webhook/sqlite.rs` 64-82) and the Postgres twin must learn the new string (Pitfall 12). The caller-facing `parse_event_kind` already rejects unknown strings.

---

### `crates/paladin-web/src/error.rs` + controllers (utility/controller, request-response)

**Analog:** `error.rs` lines 53-70.
```rust
pub struct ApiError { status: StatusCode, code: &'static str, message: String, details: Option<Value> }
pub fn with_details(mut self, details: Value) -> Self { self.details = Some(details); self }
```
Add a private `retry_after: Option<u64>` + `with_retry_after`, and insert `RETRY_AFTER` in `IntoResponse` (RESEARCH Code Example 5; A5: confirm `HeaderValue::from(u64)` in the first test). One shared helper `allowance_exhausted(&AllowanceRefusal) -> ApiError`, with code `"allowance_exhausted"` (never `too_many_requests`, line 107-109). `Retry-After = ceil(window_end - evaluated_at).max(1)`, omitted for lifetime (C8). The `ApiError` struct literal sits in `to_body`/`IntoResponse` further down (read lines 130-242 before editing).

**`run_controller.rs` `map_submission_error`** (lines 688-731) ends with `other => internal_repo_error("run submission", other)`. Add an explicit arm before it (Pitfall 7):
```rust
RunSubmissionError::AllowanceExhausted(refusal) => allowance_exhausted(&refusal),
```
`thread_controller::fork_thread` reuses this function, so test `/threads/{id}/fork` too.

**`agent_controller.rs`:** three handlers share the shape at lines 318-321 (`execute_agent`), 614-633 (stream), 699-703 (`enqueue_job`, **the fourth path, C4**):
```rust
let scope = RunScope::default().with_ledger_scope(principal.ledger_scope());
... .execute_scoped(entry.paladin.as_ref(), &request.input, &scope);
```
Call `state.treasurer` admit (+confirm) just before this line in all three, converting a refusal through the shared helper. For `enqueue_job` gate synchronously before the spawn. `AgentApiState.treasurer: Option<Arc<dyn AllowanceAdmissionPort>>` (C1). Map `Principal` to attribution via `PrincipalRef::from(&principal).attribution()` (`agent_auth.rs` 88-98). Test with the counting `MockExecutor`-style doubles at ~line 1005+.

**`schedule_controller.rs` `create_schedule`** (lines 366-391): add `created_by: Some(PrincipalRef::from(&principal).attribution())` to the `CreateRunSchedule { ... }` literal at 381-391 (`principal` is already extracted at line 368). Do not expose `created_by` in `ScheduleResponse` (Open Q7).

**OpenAPI (C9):** add `429` to the three agent `utoipa::path` annotations and a third sanctioned strip in `openapi_golden_v0_9.rs` (copy `strip_known_v0_10_execute_response_divergence`), then regenerate with `make openapi`.

---

### `src/infrastructure/web/run_api_wiring.rs` (wiring)

**Analog:** `build_treasury_ledger` / `build_run_api` (lines ~476-500 early-return on `Disabled`, C14). Build the `Treasurer` inside `build_run_api` from `settings.get_treasurer_config()`, `&configs.run_store` and the passed `AgentAuthConfig`. Run the D-11 disabled-store check before the early return. Check the webhook URL with `SsrfGuard` at wiring time, failing closed with the key path (`treasurer.allowance.webhook.url`; document `webhooks.allow_private`, Pitfall 16). Return it as `RunApiHandles.treasurer`, with no `build_run_api` signature change. `paladin-server.rs` ~200-235 attaches it to `AgentApiState`; follow the `build_auth_config` fail-closed message voice.

---

### `crates/paladin-storage/src/run_schedule/*` (adapter, CRUD)

**Analog:** the Phase 40 run attribution round trip (`Run.submitted_by`, `run.rs` 417; `with_submitted_by` 462), and `row_to_run` rejecting half-attributed rows on read. Replicate for `created_by`: both columns NULL maps to `None`, one set is a `Serialization` error. Add contract clauses: `created_by_round_trips`, `null_created_by_reads_back_none`, `half_attributed_row_is_rejected_on_read`, `update_never_changes_created_by`.

---

## Shared Patterns

### One attribution -> scope mapping
**Source:** `crates/paladin-core/src/platform/container/treasury_ledger.rs` 113-118 and `crates/paladin-web/src/agent_auth.rs` 88-98.
**Apply to:** `Treasurer`, agent handlers, schedule stamping. Never write a second mapping; the `balance` scope is `(attribution.tenant_id, Some(attribution.api_key_id))` for key ceilings and `(tenant_id, None)` for tenant ceilings.

### Store-enforced idempotency
**Source:** `sqlite.rs` 54-60 and 494-516, migration `007` header. **Apply to:** `treasury_notices` claim on all three adapters.

### Fail-closed config error voice
**Source:** `src/config/treasurer.rs` 254-268. **Apply to:** `AllowanceConfig::resolve`, `validate_against`, `build_treasurer`. Always name the full path and the offending raw value.

### No secrets or key values anywhere
**Source:** `.github/instructions/security.instructions.md`; `WebhookSpec` redacting `Debug`; `webhook.rs` prohibition P1. **Apply to:** refusal body (only figures, D-13), trace event, herald line, webhook payload, `warn` log (scope kind + tenant id + figures). Add `format!("{config:?}")` and `serde_json::to_string(&config)` no-secret tests.

### Additive-on-non-exhaustive plus register bookkeeping
**Source:** `RunSubmissionError`, `TraceEvent`, `RunEventKind` (after making it non_exhaustive); `MIGRATION.md` §9.2/9.4/9.5/9.6; `.cargo/semver-checks-allowlist.toml`. **Apply to:** every public surface change (`balance` Y + allowlist `requirement_id = "ALLOW-01"` per D-04/C11; `SubmitRun`/`CreateRunSchedule` field adds `Y` with `constructible_struct_adds_field`, C13). Then `make api-surface-update` and a CHANGELOG entry.

### Store clock only
**Apply to:** window math, `Retry-After`, notice `window_start`. Never `Utc::now()` in window or boundary logic (`submission.rs` 362 uses `Utc::now()` for `enqueued_at` only; do not copy it for windows).

### Never `f64`/`unwrap`/`expect` in library code
**Apply to:** all new library code. `format_cost` is display-only. Use `checked_*` and `i128` for threshold math.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/application/services/treasurer/window.rs` (`window_for`) | utility | transform | No epoch-aligned window code exists; use RESEARCH Code Example 1 (pure arithmetic) |
| `allowance_admission_port.rs` claim/confirm/abandon lifecycle | port | request-response | No input port with a two-phase confirm/abandon exists; shape after `RunSubmissionPort` and follow RESEARCH Pattern 3 |

Partial analogs: the worker-side readback (`worker.rs` `run_once` `Queued` arm, RESEARCH Pattern 4) has no existing "emit before start" precedent, so read lines ~1019-1070 first. The `period` parser (RESEARCH Code Example 6) is new but follows the checked-arithmetic style of `parse_price_nanos_per_million`.

## Planner Decisions Needed Before One-Way Migrations (from RESEARCH)

C1 (new `AllowanceAdmissionPort`), C3 (operator webhook row/secret disposition), C5 (`api_key_id NOT NULL ''`), C6 (worker-side trace emission), C11 (§9.2 `Y` vs `N/A` for `balance`), plus adding `window_end` and `warn_at` to `011`. C12 (D-08 vs D-11 on a removed creator key) and C4 (gate `jobs`) also change implementation scope.

## Metadata

**Analog search scope:** `src/config`, `src/application/services/run`, `src/infrastructure/{web,telemetry}`, `crates/paladin-{core,ports,storage,web,herald}`, `crates/paladin-storage/migrations`.
**Files read this session:** CONTEXT.md, RESEARCH.md (full), `submission.rs` (1-400), `treasurer.rs` (1-300), plus targeted excerpts from `sqlite.rs`, `error.rs`, `run_controller.rs`, `trace.rs`, `herald_sink.rs`, `webhook/mod.rs`, `schedule/service.rs`, `agent_controller.rs`, `agent_auth.rs`, `principal.rs`, migrations `007`/`008`/`009`.
**Not read (planner should read before editing):** `worker.rs` `run_once`, `webhook/service.rs` `process`, `run_api_wiring.rs`, `paladin-server.rs`, `error.rs` 130-242, run_schedule adapters, `markdown/json/table` herald renderers beyond one call site.
**Pattern extraction date:** 2026-10-02
