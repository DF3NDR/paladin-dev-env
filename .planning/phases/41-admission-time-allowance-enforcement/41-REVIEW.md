---
phase: 41-admission-time-allowance-enforcement
reviewed: 2026-10-04T00:00:00Z
depth: standard
files_reviewed: 79
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/allowance.rs
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/run_schedule.rs
  - crates/paladin-core/src/platform/container/run_scope.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-core/src/platform/container/treasury_ledger.rs
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-herald/src/table_herald.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/input/allowance_admission_port.rs
  - crates/paladin-ports/src/input/mod.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/input/schedule_admin_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/treasury_ledger_port.rs
  - crates/paladin-ports/src/output/treasury_notice_port.rs
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/migrations/postgres/010_add_run_schedule_created_by.sql
  - crates/paladin-storage/migrations/postgres/011_create_treasury_notices.sql
  - crates/paladin-storage/migrations/postgres/012_add_treasury_ledger_tenant_index.sql
  - crates/paladin-storage/migrations/sqlite/010_add_run_schedule_created_by.sql
  - crates/paladin-storage/migrations/sqlite/011_create_treasury_notices.sql
  - crates/paladin-storage/migrations/sqlite/012_add_treasury_ledger_tenant_index.sql
  - crates/paladin-storage/src/run_schedule/contract_tests.rs
  - crates/paladin-storage/src/run_schedule/in_memory.rs
  - crates/paladin-storage/src/run_schedule/postgres.rs
  - crates/paladin-storage/src/run_schedule/sqlite.rs
  - crates/paladin-storage/src/treasury/contract_tests.rs
  - crates/paladin-storage/src/treasury/in_memory.rs
  - crates/paladin-storage/src/treasury/notice_contract_tests.rs
  - crates/paladin-storage/src/treasury/postgres.rs
  - crates/paladin-storage/src/treasury/sqlite.rs
  - crates/paladin-storage/src/webhook/contract_tests.rs
  - crates/paladin-storage/src/webhook/in_memory.rs
  - crates/paladin-storage/src/webhook/postgres.rs
  - crates/paladin-storage/src/webhook/sqlite.rs
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_auth.rs
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/error.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/schedule_controller.rs
  - crates/paladin-web/src/thread_controller.rs
  - crates/paladin-web/tests/openapi_golden_v0_9.rs
  - docs/src/api-reference/platform-api.md
  - docs/src/deployment-topologies/http-service-host.md
  - docs/src/getting-started/configuration.md
  - src/application/services/assistant/tests.rs
  - src/application/services/mod.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/events.rs
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/schedule/admin.rs
  - src/application/services/run/schedule/service.rs
  - src/application/services/run/schedule/tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/webhook/mod.rs
  - src/application/services/run/webhook/service.rs
  - src/application/services/run/webhook/tests.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/application/services/treasurer/mod.rs
  - src/application/services/treasurer/policy.rs
  - src/application/services/treasurer/tests.rs
  - src/application/services/treasurer/window.rs
  - src/bin/paladin-server.rs
  - src/config/mod.rs
  - src/config/treasurer.rs
  - src/core/platform/mod.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/web/run_api_wiring.rs
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 41: Code Review Report

**Reviewed:** 2026-10-04
**Depth:** standard
**Files Reviewed:** 79
**Status:** issues_found

## Summary

Phase 41 adds the Treasurer admission slice (`allowance.rs`, `treasurer/`), a store-enforced
once-per-window notice table, schedule-creator attribution, the `429 allowance_exhausted`
contract, the operator `allowance_warning` webhook, and the trace/herald legs. The diff was
reviewed against `git diff 8b0184d^..HEAD` with the credential-handling rules in
`.github/instructions/security.instructions.md` applied.

The core admission logic is sound. Integer-only comparison is used throughout, with no `f64` and
no overflow: `crosses_warn_threshold` widens to `i128`. The `>=` ceiling rule, whole-second
store-clock windows, and fail-closed behaviour all hold. Binding is used for every SQL value, so
there is no injection surface in `balance` or the notice statements. The operator webhook secret
is redacted in `Debug`, skipped by `Serialize`, and never written to a delivery row. The operator
notice cannot be subscribed to by callers (`parse_event_kind` still answers 400). The webhook
client remains no-redirect with the SSRF guard applied at send time. I traced SQLite's textual
timestamp comparison in `balance` (`'+'` < `'.'` ordering of sqlx's RFC 3339 encoding) and it is
correct at sub-second window edges. I found no unwrap, expect or panic in non-test code added by
this phase, and no secret or key value in any log, error, payload or `Debug` output.

No blockers were found. The warnings concern the durability of the once-per-window notice (the
claim is recorded before it is delivered, and delivery is best-effort), boot-time coupling to DNS,
a silent env-override parse failure, and a clock mix in the delivery queue.

## Warnings

### WR-01: Once-per-window notice can be permanently lost between claim and delivery (cancellation or enqueue failure)

**File:** `src/application/services/treasurer/mod.rs:185-230, 397-407` and `src/application/services/run/submission.rs:362-385`

**Issue:** `admit` writes the durable once-per-window notice row, and that row is the dedupe
key. The only delivery step for the operator webhook is `confirm`, which makes one best-effort
`enqueue` (failures are logged and swallowed). Two ways the notice can be lost with no recovery:

1. **Enqueue failure.** If `target.deliveries.enqueue(delivery)` fails (a transient DB error), or
   `serde_json::to_string` / `ThreadId::new` fails, the notice row stays recorded. Every later
   admission in the window gets `AlreadyRecorded` and emits nothing, so the operator never gets
   the webhook for that window and nothing retries it. The durable queue gives retries only
   after a row exists, and this is the step that creates the row.
2. **Future cancellation.** `admit_and_persist` awaits `admit`, then `persist_and_enqueue`, then
   `confirm`/`abandon`. If the request future is dropped at one of those await points (an axum
   handler is dropped when the client disconnects), neither `confirm` nor `abandon` runs. The
   notice is recorded but never delivered and never given back, so the window's webhook (and, for
   a run, nothing else rereads it besides the worker trace leg) is silently consumed.

The module docs accept "a crash can lose a window's notice", but the enqueue-failure and
client-disconnect cases need no crash and are far more likely than one.

**Fix:** Make the claim recoverable. For case 1, on enqueue failure in `confirm`, give the notice
back so the next admission re-wins it:

```rust
// in Treasurer::confirm, after a failed enqueue_operator_delivery for `notice`
if let Some(notices) = &self.notices {
    if let Err(e) = notices.discard(std::slice::from_ref(&notice.notice_id)).await {
        log::error!("operator notice give-back failed: {e}");
    }
}
```

(Have `enqueue_operator_delivery` return `bool`/`Result`.) For case 2, run the
persist-then-confirm/abandon tail in a detached task, or hold a drop guard that spawns `abandon`,
so the lifecycle completes even when the handler future is cancelled.

### WR-02: Server boot depends on DNS availability for an optional notification leg

**File:** `src/infrastructure/web/run_api_wiring.rs:586-599`

**Issue:** When `treasurer.allowance` has entries and a `webhook` is configured,
`SsrfGuard::check_url(&webhook.url)` runs at boot and propagates its error with `?`. For a
hostname, `check_url` resolves it, and a host that does not resolve is `SsrfRejection::HostUnresolved`
("did not resolve to any address"). A transient DNS failure at startup (a cold cluster resolver, a
restart storm, the operator target's DNS not yet reachable) therefore stops the whole Platform API
from booting, taking enforcement and every run route with it, because of an optional operator
notification target. Static rejections (scheme, literal private/metadata IP) are correct to fail
closed on at boot. Resolution failures are not a config error, and the send-time guard in
`WebhookDeliveryService::process` already re-checks.

**Fix:** At boot, fail only on static rejections, and downgrade an unresolved host to a warning:

```rust
match SsrfGuard::new(configs.webhooks.allow_private).check_url(&webhook.url).await {
    Ok(()) => {}
    Err(SsrfRejection::HostUnresolved { .. }) => {
        log::warn!("treasurer.allowance.webhook.url does not resolve yet; deliveries retry at send time");
    }
    Err(rejection) => return Err(format!("treasurer.allowance.webhook.url rejected by the SSRF guard: {rejection} -- ...").into()),
}
```

(Exact variant name per `SsrfRejection`; if the guard cannot distinguish, expose a syntax-only
check for boot.)

### WR-03: `APP_TREASURER_ALLOWANCE_WARN_AT` silently ignored when it does not parse as `u8`

**File:** `src/config/treasurer.rs:638-640` (via `src/config/env_utils.rs:52-54`)

**Issue:** `read_env::<u8>` returns `None` on any parse failure, so `APP_TREASURER_ALLOWANCE_WARN_AT=300`,
`=-1`, `=80%`, or `=eighty` is dropped without a trace and the file value (or the default 80) is
used. The docs (`configuration.md` Warn threshold) state a value above 100 "is rejected, never
clamped"; that holds for `101..=255` but not for the common typo/overflow cases, which silently
change when operators are warned. Only `101..=255` reaches `warn_at_error`.

**Fix:** Parse the variable explicitly and surface the failure through `validate()`, e.g. read the
raw `String`, and store a `warn_at_env_error: Option<String>` (or return `Result` from the override)
that `AllowanceConfig::resolve` reports as `treasurer.allowance.warn_at (from APP_TREASURER_ALLOWANCE_WARN_AT) must be an integer percent between 0 and 100`.

### WR-04: Operator delivery is scheduled with the store clock but claimed with the process clock

**File:** `src/application/services/treasurer/mod.rs:216-224`

**Issue:** `WebhookDelivery::new(.., notice.recorded_at)` sets `next_attempt_at` to the ledger
store instant. `WebhookDeliveryService::run_once` claims with `claim_due(now)` where `now` comes
from the process clock (`options.now`). The run-webhook producer (`worker.rs` `webhook_delivery_for_outcome`)
passes the process `now`. With a PostgreSQL host whose clock runs ahead of the app host, every
operator notice is delayed by the skew; the rest of this phase works to avoid mixing the two
clocks. (Window arithmetic correctly uses only the store clock; this is a queue-scheduling use.)

**Fix:** Use the process clock for queue scheduling: pass `Utc::now()` (or the delivery service's
injected `now`) as `next_attempt_at`, keep `recorded_at` for the payload `timestamp` only.

### WR-05: Operator webhook configured without a secret is signed with the empty key and gives no boot signal

**File:** `src/application/services/run/webhook/service.rs:240-246` and `src/infrastructure/web/run_api_wiring.rs:800-812`

**Issue:** With `treasurer.allowance.webhook.url` set and no secret (including
`APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET=""`, which yields `Some("")`), the delivery is still sent
with an `X-Paladin-Signature: sha256=<hex>` header computed over the empty key. Any third party can
compute that value, so the header reads as authentication while providing none. The docs mention
it (platform-api.md "Without a secret the body is signed with the empty key"), but nothing in
`build_run_api` warns the operator at boot, and the payload carries tenant ids and key names.
(This mirrors run webhooks, where the caller chooses the secret; here the operator is the sole
configurer and can be required to.)

**Fix:** Treat an empty secret as absent (`filter(|s| !s.is_empty())` in the env override and in
`with_operator_notice_secret`), and log a boot-time warning (or require the secret) when an
operator webhook is configured without one.

## Info

### IN-01: Notice identity omits the window period, so changing `period` can suppress one window's notice

**File:** `crates/paladin-storage/migrations/sqlite/011_create_treasury_notices.sql` (unique index `idx_treasury_notices_once`), `crates/paladin-core/src/platform/container/allowance.rs:330-340`

**Issue:** The key is `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)`.
Two different periods can share a `window_start` (a `24h` and a `1h` window both begin at 00:00
UTC). If an operator changes `period` without changing `amount`, the first new-period window that
starts at the same instant dedupes against the old row and is not notified. `window_end` is stored
but not part of the identity.

**Fix:** Either add `window_end` to the unique index and the `ON CONFLICT` column list (keeping the
textual-match tests), or document that a period change at an aligned boundary can skip one notice.

### IN-02: One `warn`-level log line per refused request

**File:** `src/application/services/treasurer/mod.rs:352-360`

**Issue:** Every refused admission logs at `warn`. A client that keeps retrying while exhausted (the
`Retry-After` is honoured only by well-behaved clients) can flood logs. The line carries figures and
the tenant id (log-safe per D-00g), not a credential.

**Fix:** Log at `info`/`debug`, or log once per (scope, window) using the same dedupe the notice uses.

### IN-03: `treasury_notices` and operator delivery rows have no retention

**File:** `crates/paladin-storage/migrations/{sqlite,postgres}/011_create_treasury_notices.sql`

**Issue:** The table is append-only apart from `discard`, and a `1m` window (the allowed minimum)
under sustained spend at or above `warn_at` adds up to 1,440 rows per day per ceiling. No prune hook
is wired, unlike waypoints and `run_traces`.

**Fix:** Add a retention pass keyed on `recorded_at` (the notice is only needed while its window is
current, plus any worker first-dispatch read), or record the gap in `WINDOWS.md`.

### IN-04: OpenAPI documents the fail-closed `500` on two of the five gated operations

**File:** `crates/paladin-web/src/agent_controller.rs:348-361, 651-665, 754-760`, `crates/paladin-web/openapi.json`

**Issue:** `POST /v1/runs` and `POST /v1/threads/{id}/fork` document `500 (allowance check failed)`,
but `execute`, `execute/stream` and `jobs` do not, although `admit_principal` returns the same `500`
on a ledger failure and the MIGRATION entry says "answers `500` ... on every gated route". The omission
is probably forced by the v0.9 golden gate (which strips only `429`), but the contract is then
inconsistent.

**Fix:** Either extend the golden exception to a `500` entry on those three operations (narrowly
scoped and tested like the `429` one), or state in `platform-api.md` that the agent routes' `500` is
undocumented in the frozen spec.

### IN-05: Stale variant counts in trace documentation

**File:** `crates/paladin-core/src/platform/container/trace.rs:17`, `crates/paladin-storage/src/run_trace/mod.rs:36`, `crates/paladin-ports/src/output/trace_sink_port.rs:510`

**Issue:** `TraceEvent` now has thirteen variants (the header at `trace.rs:4` and the construct test
were updated), but these three still say "twelve" (`superstep_of`'s doc even lists the variants
without `AllowanceWarning`, which now falls into the wildcard `0` bucket).

**Fix:** Update the counts and add `AllowanceWarning` to the `superstep_of` "stamped 0" list.

### IN-06: Notice store opened as a second full ledger instance/pool per backend

**File:** `src/infrastructure/web/run_api_wiring.rs:449-466` (`build_treasury_notices`), `:753`

**Issue:** `build_treasury_notices` constructs a second `SqliteTreasuryLedger`/`PostgresTreasuryLedger`
(its own pool; Postgres `max_connections(5)`) over the same database that `build_treasury_ledger`
already opened, because the ledger is only held as `Arc<dyn TreasuryLedgerPort>`. This doubles pool
connections and migration runs for no behavioural gain, and on `sqlite::memory:` yields two separate
databases.

**Fix:** Build one concrete store and coerce it to both trait objects (or add a combined
`TreasuryStore: TreasuryLedgerPort + TreasuryNoticePort` supertrait return) so the ledger and notice
store share one pool.

---

_Reviewed: 2026-10-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
