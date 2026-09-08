---
phase: 27-platform-api
reviewed: 2026-09-08T00:00:00Z
depth: standard
files_reviewed: 117
files_reviewed_list:
  - .github/workflows/ci.yml
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/graph_doc.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/tests/graph_doc_round_trip.rs
  - crates/paladin-core/src/platform/container/assistant.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/run_schedule.rs
  - crates/paladin-core/src/platform/container/webhook.rs
  - crates/paladin-ports/src/input/assistant_admin_port.rs
  - crates/paladin-ports/src/input/mod.rs
  - crates/paladin-ports/src/input/parley_port.rs
  - crates/paladin-ports/src/input/run_event_stream_port.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/input/schedule_admin_port.rs
  - crates/paladin-ports/src/output/assistant_repository_port.rs
  - crates/paladin-ports/src/output/cancellation_probe.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/run_queue_port.rs
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-ports/src/output/run_schedule_repository_port.rs
  - crates/paladin-ports/src/output/webhook_delivery_port.rs
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-storage/migrations/postgres/002_create_runs_table.sql
  - crates/paladin-storage/migrations/postgres/003_create_assistants_tables.sql
  - crates/paladin-storage/migrations/postgres/004_create_run_schedules_table.sql
  - crates/paladin-storage/migrations/postgres/005_create_webhook_deliveries_table.sql
  - crates/paladin-storage/migrations/sqlite/002_create_runs_table.sql
  - crates/paladin-storage/migrations/sqlite/003_create_assistants_tables.sql
  - crates/paladin-storage/migrations/sqlite/004_create_run_schedules_table.sql
  - crates/paladin-storage/migrations/sqlite/005_create_webhook_deliveries_table.sql
  - crates/paladin-storage/src/assistant/contract_tests.rs
  - crates/paladin-storage/src/assistant/in_memory.rs
  - crates/paladin-storage/src/assistant/mod.rs
  - crates/paladin-storage/src/assistant/postgres.rs
  - crates/paladin-storage/src/assistant/sqlite.rs
  - crates/paladin-storage/src/cron.rs
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/run/contract_tests.rs
  - crates/paladin-storage/src/run/in_memory.rs
  - crates/paladin-storage/src/run/mod.rs
  - crates/paladin-storage/src/run/postgres.rs
  - crates/paladin-storage/src/run/sqlite.rs
  - crates/paladin-storage/src/run_queue/contract_tests.rs
  - crates/paladin-storage/src/run_queue/in_memory.rs
  - crates/paladin-storage/src/run_queue/mod.rs
  - crates/paladin-storage/src/run_queue/redis.rs
  - crates/paladin-storage/src/run_schedule/contract_tests.rs
  - crates/paladin-storage/src/run_schedule/in_memory.rs
  - crates/paladin-storage/src/run_schedule/mod.rs
  - crates/paladin-storage/src/run_schedule/postgres.rs
  - crates/paladin-storage/src/run_schedule/sqlite.rs
  - crates/paladin-storage/src/scheduler.rs
  - crates/paladin-storage/src/webhook/contract_tests.rs
  - crates/paladin-storage/src/webhook/in_memory.rs
  - crates/paladin-storage/src/webhook/mod.rs
  - crates/paladin-storage/src/webhook/postgres.rs
  - crates/paladin-storage/src/webhook/sqlite.rs
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/src/assistant_controller.rs
  - crates/paladin-web/src/lib.rs
  - crates/paladin-web/src/openapi.rs
  - crates/paladin-web/src/pagination.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/schedule_controller.rs
  - crates/paladin-web/src/thread_controller.rs
  - k8s/server/worker-deployment.yaml
  - scripts/sdk-smoke/run.sh
  - scripts/sdk-smoke/smoke-config.yml
  - scripts/sdk-smoke/smoke.py
  - scripts/sdk-smoke/smoke.ts
  - src/application/services/assistant/doc_registry.rs
  - src/application/services/assistant/mod.rs
  - src/application/services/assistant/resolver.rs
  - src/application/services/assistant/service.rs
  - src/application/services/assistant/tests.rs
  - src/application/services/assistant/validator.rs
  - src/application/services/mod.rs
  - src/application/services/parley/adapter.rs
  - src/application/services/parley/registry.rs
  - src/application/services/run/cancel.rs
  - src/application/services/run/cancel_tests.rs
  - src/application/services/run/events.rs
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/mod.rs
  - src/application/services/run/resolver.rs
  - src/application/services/run/schedule/admin.rs
  - src/application/services/run/schedule/mod.rs
  - src/application/services/run/schedule/service.rs
  - src/application/services/run/schedule/tests.rs
  - src/application/services/run/stream_tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/tracer_e2e.rs
  - src/application/services/run/webhook/client.rs
  - src/application/services/run/webhook/mod.rs
  - src/application/services/run/webhook/service.rs
  - src/application/services/run/webhook/signature.rs
  - src/application/services/run/webhook/ssrf.rs
  - src/application/services/run/webhook/tests.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/bin/paladin-server.rs
  - src/config/assistants.rs
  - src/config/mod.rs
  - src/config/run_queue.rs
  - src/config/run_store.rs
  - src/config/run_stream.rs
  - src/config/run_worker.rs
  - src/config/schedules.rs
  - src/config/webhooks.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/infrastructure/web/mod.rs
  - src/infrastructure/web/run_api_wiring.rs
  - tests/integration/e2e_platform_api_test.rs
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
status: issues_found
---

# Phase 27: Code Review Report

**Reviewed:** 2026-09-08T00:00:00Z
**Depth:** standard
**Files Reviewed:** 117
**Status:** issues_found

## Summary

Phase 27 (Platform API) is a large, well-engineered slice: the SSRF guard
(`ssrf.rs`) is table-tested and correctly classifies loopback/link-local/
RFC1918/unique-local/unspecified/metadata addresses, handles decimal-IPv4 and
IPv4-mapped-IPv6 obfuscation, and is applied both at write time and send
time exactly as documented. HMAC signing (`signature.rs`) signs the exact
byte buffer stored on the delivery row, never a re-serialization. Every SQL
adapter reviewed (`run/sqlite.rs`, `run/postgres.rs`, `webhook/sqlite.rs`,
`webhook/postgres.rs`, `run_schedule/sqlite.rs`) uses bound parameters or
`QueryBuilder::push_bind` exclusively — no string-built SQL was found. The
compare-and-set `update_status`/`claim_tick`/`claim_due` primitives are
correctly ordered (state-machine legality checked before the CAS predicate;
`rows_affected() == 1` is the sole arbiter of a race winner) and are
exercised by real concurrent-connection tests, not just sequential ones.
Redaction is consistently applied redact-then-truncate
(`webhook/service.rs::bounded_error`), and secrets never reach `Debug`
output (`WebhookSpec`, `RedisRunQueueConfig`) or a `RunStore`/`RunQueue`
config's serialized form (only the referring env var name is stored, never
the connection string).

Against that generally strong baseline, this review found one resource-
exhaustion-shaped gap in the webhook delivery HTTP client (an attacker-
influenced response body is read unboundedly before it is ever truncated),
plus four correctness/robustness gaps: a webhook delivery is signed with a
silently-empty key (and still sent, consuming a retry attempt) when the
run-repository lookup that supplies the real signing secret fails; the
legacy `Agent`-kind run-execution path bypasses both the SSE event bus and
the webhook-delivery hook entirely, so a caller-configured webhook on an
`Agent`-kind assistant is never fired; `GET /runs`/`GET /runs/{id}`/`GET
/runs/{id}/webhook-deliveries` have no per-caller/tenant scoping, so any
authenticated principal (any role) can enumerate every run and webhook
target in the deployment; and a defensive gap in `LeaseHeartbeat` around a
theoretical zero-duration lease.

## Critical Issues

### CR-01: Webhook delivery HTTP client reads an unbounded, attacker-influenced response body before truncation

**File:** `src/application/services/run/webhook/service.rs:223-226`
**Issue:** On any non-2xx response, the delivery service does:
```rust
let body = response.text().await.unwrap_or_default();
let error = Some(bounded_error(&format!("http {status}: {body}")));
```
`bounded_error` truncates to 256 characters, but only *after* `response.text()`
has already buffered the **entire** response body into memory. The
`reqwest::Client` built in `client.rs` (`build_webhook_client`) sets no
response-size cap (`Content-Length` check, streaming byte limit, etc.).

The webhook target URL is supplied by any authenticated, non-admin
principal via `POST /runs` (`webhook.url`, only constrained by the SSRF
guard's *address-class* checks — a public-facing attacker-controlled host
passes it trivially). A malicious or compromised webhook receiver can
answer every delivery attempt with a `500` (or any non-2xx status) and an
arbitrarily large body (e.g. several GB); a `5xx` response is retried up to
`max_attempts` (default 5) with the D-43 backoff schedule, and the caller
can trigger unlimited additional deliveries simply by submitting more runs
against the same assistant/webhook. Each attempt fully buffers the
attacker-controlled body into the `paladin-server` process's memory before
the 256-byte truncation ever applies — a straightforward, repeatable
memory-exhaustion vector reachable by any authenticated `User`-role
principal, not just an admin.

**Fix:** Bound the read before it starts, e.g. via a streamed read with an
early-abort cap, or reject/truncate based on `Content-Length` before
calling `.text()`:
```rust
const MAX_ERROR_BODY_BYTES: usize = 64 * 1024;

let body = {
    use bytes::Buf;
    let mut buf = Vec::with_capacity(1024);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap_or_default();
        if buf.len() + chunk.len() > MAX_ERROR_BODY_BYTES {
            buf.extend_from_slice(&chunk[..MAX_ERROR_BODY_BYTES - buf.len()]);
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    String::from_utf8_lossy(&buf).into_owned()
};
```
(or the simpler `reqwest::Client::builder().https_only(..)`-style
equivalent of capping via `Response::content_length()` combined with a
`take(N)` on the byte stream). The exact mechanism matters less than
ensuring the full body is never resident in memory before `bounded_error`'s
truncation runs.

## Warnings

### WR-01: A webhook is signed with a silently-empty key (and still sent) when the run lookup fails

**File:** `src/application/services/run/webhook/service.rs:181-195`
**Issue:**
```rust
let signing_key = match self.runs.get(&delivery.run_id).await {
    Ok(Some(run)) => run.webhook.as_ref().and_then(|webhook| webhook.secret.clone()).unwrap_or_default(),
    Ok(None) => String::new(),
    Err(error) => {
        log::warn!("webhook delivery service: failed to load run {} for delivery {delivery_id}: {error}", delivery.run_id);
        String::new()
    }
};
```
When `self.runs.get(...)` fails with a **transient backend error** (a DB
hiccup unrelated to the webhook target), the code does not retry or skip —
it silently falls back to signing with an empty key and proceeds to `POST`
the delivery to the real target URL anyway (`client.post(&delivery.url)...`
a few lines below). This burns one of the delivery's limited
`max_attempts` retries on a failure that has nothing to do with the
target, and delivers a payload whose `X-Paladin-Signature` a correctly-
implemented receiver will reject as invalid — the caller's receiver-side
verification silently fails for a reason invisible to them, and the
delivery may exhaust its retry budget and dead-letter even though the
target was perfectly reachable and correct the whole time.

**Fix:** On `Err` from the run lookup, treat it the same way a transient
send failure is treated (i.e., schedule a retry without sending and
without incrementing `attempt`), rather than sending a delivery you know
carries a wrong signature:
```rust
Err(error) => {
    log::warn!("...");
    self.finish(&delivery_id, WebhookAttemptResult {
        outcome: WebhookAttemptOutcome::Retrying {
            next_attempt_at: (self.options.now)() + chrono::Duration::seconds(5),
        },
        response_status: None,
        error: Some(bounded_error(&format!("failed to load signing secret: {error}"))),
    }).await;
    return;
}
```

### WR-02: `Agent`-kind runs never emit SSE events or webhook deliveries

**File:** `src/application/services/run/worker.rs:940-986` (`run_agent`), contrast with `run_once` lines 727-928
**Issue:** `RunWorkerPool::run_once` dispatches a `Runnable::Agent` straight
to `run_agent` (line 686) *before* any of the D-24 event-bus
`bind`/`publish`/`unbind` calls or the D-40 `webhook_delivery_for_outcome`
+ `deliveries.enqueue(...)` logic that every `Runnable::Workflow` dispatch
goes through after `map_outcome`. `run_agent`'s own success/failure paths
(lines 958-985) call `update_status`/`record_outcome`/`queue.ack` directly
and return, never touching `self.event_bus` or `self.webhook_deliveries`.

The module docs for `webhook_deliveries` (worker.rs:431-440) and
`WebhookPayload`'s own module doc (`webhook/mod.rs:1-6`, PRD 06 PLAT-FR-14)
describe the delivery hook unconditionally ("`run_once` enqueues a
`Pending` `WebhookDelivery` on every terminal/suspension transition whose
run subscribes to that event") with no stated carve-out for the legacy
`Agent`-kind path. A caller who sets `webhook` on a `SubmitRun` request
against an `Agent`-kind (code-registered) assistant will never receive a
delivery for that run's completion or failure, and `GET
/runs/{run_id}/stream` for such a run only ever falls back to the degraded
polling path (never live) — the polling path does still work because
`run.status` transitions correctly, but the live-bus behavior documented
elsewhere in this phase silently does not apply here.

**Fix:** Either wire the same webhook-enqueue/event-bus calls into
`run_agent`'s two return paths, or explicitly document (in both
`worker.rs`'s module docs and the PLAT-FR-14 wire docs) that `Agent`-kind
runs are excluded from PLAT-FR-14 webhook delivery and from the D-24 live
SSE bus, so this is a documented limitation rather than a silent gap.

### WR-03: `GET /runs`, `GET /runs/{id}`, and `GET /runs/{id}/webhook-deliveries` have no per-caller scoping

**File:** `crates/paladin-web/src/run_controller.rs:722-762` (`list_runs`), `:669-687` (`get_run`), `:846-875` (`list_webhook_deliveries`)
**Issue:** All three read routes require only `require_authentication` —
any authenticated principal, of any `UserRole`, can list and read every run
in the entire deployment (`RunQuery` carries no caller identity, and
`RunRepositoryPort::list`/`get` apply no filter derived from the
requester). `RunResponse` includes the webhook target `url` (via
`RunWebhookDto::from`, secret redacted but URL not), `thread_id`,
`assistant_id`, and `error` text for runs the caller never submitted, and
`list_webhook_deliveries` similarly exposes another user's delivery
history (target URL, HTTP status codes, redacted-but-present error
diagnostics) for a `run_id` the caller need only guess or enumerate (run
ids are UUIDv7, time-ordered — sequential enumeration by a low-privilege
authenticated user is materially easier than for a random UUID). This is
not in the set of documented "deliberate tightenings" carved out for this
review (D-18/D-42/D-32); the module doc simply states these routes "need
authentication only" without discussing cross-tenant exposure.

**Fix:** If this is intentional for v0.10 (single-tenant/trusted-principal
deployment model), say so explicitly in the module docs and PRD 06 so a
future multi-tenant deployment doesn't inherit it silently. If it is not
intentional, scope `RunQuery`/`get`/`list_for_run` by the requesting
principal (or principal's org/tenant) the same way `submit`/`cancel`
already resolve `allowed_roles` per caller.

### WR-04: `LeaseHeartbeat::spawn` busy-loops if constructed with a zero-duration lease

**File:** `src/application/services/run/worker.rs:126-143`
**Issue:**
```rust
let interval = if lease.is_zero() { lease } else { lease / 4 };
let handle = tokio::spawn(async move {
    loop {
        tokio::time::sleep(interval).await;
        if queue.extend_lease(&token, lease).await.is_err() { break; }
    }
});
```
If `lease` is `Duration::ZERO`, `interval` is also `Duration::ZERO`, and
`tokio::time::sleep(Duration::ZERO)` resolves essentially immediately, so
this becomes a tight loop calling `extend_lease` as fast as the executor
can schedule it until the call errors. `RunWorkerConfig::validate()`
enforces `lease_seconds >= 4` for the one production call site
(`run_api_wiring.rs`), so this is not reachable through normal
configuration today, but `RunWorkerPool::new`/`LeaseHeartbeat::spawn` are
both public API with no assertion guarding this precondition, so any
future caller (or a misconfigured direct `RunWorkerPool::new` construction
bypassing the config layer) can trigger a CPU-spinning task.

**Fix:** Guard `LeaseHeartbeat::spawn` itself (not just the config layer)
against a non-positive lease, e.g. clamp `interval` to a minimum floor
(`lease.max(Duration::from_millis(1)) / 4`) or return early / log-and-skip
when `lease.is_zero()`.

## Info

### IN-01: `attempt` default-deserializes to `0` while `Run::new` always starts at `1`

**File:** `crates/paladin-core/src/platform/container/run.rs:386-387, 431`
**Issue:** `Run::new` sets `attempt: 1` for a freshly constructed run, but
the `#[serde(default)]` fallback for a `Run` payload missing the `attempt`
field is `u32::default() == 0` (proven by the module's own test,
`run_deserializes_with_only_identity_trio_and_required_fields_present`,
which asserts `run.attempt == 0`). In practice every persisted row always
carries `attempt` (the INSERT statements bind it explicitly), so this only
matters for a hand-crafted or very old payload missing the field — but the
inconsistency between the constructor's true "no attempts yet" value (1)
and the deserialization fallback (0) is worth a `default_run_attempt()`
helper (mirroring `default_run_schema_version()`) so the two paths agree,
rather than relying on the field always being present in practice.

### IN-02: `fork_edit_to_state_delta` silently drops a fork edit key that fails `FieldName::new`

**File:** `src/application/services/run/worker.rs:220-237`
**Issue:** A caller-supplied `fork.edit` object key that is empty (the only
way `FieldName::new` currently fails) is dropped from the merged
`StateDelta` with no error surfaced back to the caller — the fork still
proceeds, just without that field's edit applied. This is deliberate per
the function's own doc comment (`WarEngine::fork` would reject an unknown
field name at merge time via a typed error, so this silent-drop path is
scoped to only the pathological empty-string-key case), but it means a
client that accidentally sends `{"": "value"}` in a fork edit gets no
diagnostic anywhere — the fork "succeeds" with that one field silently
unapplied. Consider surfacing this as a validation warning in the fork
response, or rejecting the edit body outright with a `400` if any key is
empty, rather than accepting-and-dropping.

---

_Reviewed: 2026-09-08T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
