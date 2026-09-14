---
phase: 27-platform-api
reviewed: 2026-09-08T18:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - .github/workflows/ci.yml
  - .gitignore
  - crates/paladin-storage/src/run/contract_tests.rs
  - crates/paladin-storage/src/run/mod.rs
  - crates/paladin-storage/src/run/postgres.rs
  - crates/paladin-storage/src/run_queue/contract_tests.rs
  - crates/paladin-storage/src/run_queue/in_memory.rs
  - crates/paladin-storage/src/run_queue/redis.rs
  - crates/paladin-web/src/run_controller.rs
  - docs/src/api-reference/platform-api.md
  - scripts/extract-public-api.sh
  - scripts/normalize-api-bounds.py
  - scripts/sdk-smoke/lib-boot.sh
  - scripts/sdk-smoke/mock-llm.py
  - scripts/sdk-smoke/package.json
  - scripts/sdk-smoke/run.sh
  - scripts/sdk-smoke/smoke-config.yml
  - scripts/sdk-smoke/smoke-http.sh
  - scripts/sdk-smoke/smoke.py
  - scripts/sdk-smoke/smoke.ts
  - src/application/services/run/webhook/client.rs
  - src/application/services/run/webhook/mod.rs
  - src/application/services/run/webhook/service.rs
  - src/application/services/run/webhook/tests.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
findings:
  critical: 0
  warning: 1
  info: 1
  total: 2
status: issues_found
---

# Phase 27: Code Review Report

**Reviewed:** 2026-09-08T18:00:00Z
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Summary

This review covers the files changed by phase 27's gap-closure plans 27-19..27-26 (`git diff
75b82bd8..HEAD`), the second review pass over this phase. The first pass (same path, still visible
in git history) raised four findings — CR-01 (unbounded webhook response body), WR-01 (signing-key
load failure sent the webhook anyway with a fallback empty key), WR-02 (Agent-kind runs silently
skip the webhook/event-bus hooks), WR-03 (unscoped reads on the three run read routes), WR-04 (a
zero-duration lease heartbeat spins). All four were addressed by this gap-closure work; each is
verified below rather than re-raised.

**CR-01 — verified fixed.** `webhook/client.rs`'s `read_bounded_body` caps the response body during
the read itself (never buffers past `MAX_ERROR_BODY_BYTES`, truncates the final chunk rather than
overshooting), and `webhook/service.rs`'s non-2xx branch now calls it instead of `response.text()`.
Covered by `bounded_body_stops_at_the_cap`, `bounded_body_at_exactly_the_cap_is_not_truncated`, and
`bounded_body_returns_a_small_body_whole` in `client.rs`.

**WR-01 — verified fixed, for the `Err` branch only (see WR-27-01 below for the adjacent gap this
fix left behind).** `webhook/service.rs::process` now reschedules (`Retrying`, backoff-delayed)
rather than sending with a fallback empty signing key when `self.runs.get(&delivery.run_id)`
returns `Err`. Covered by
`webhook_signing_key_load_failure_reschedules_without_sending` in `tests.rs`, which asserts the
mock target records zero hits.

**WR-02 — verified addressed by documentation and a pinning test, not by closing the gap (as
intended by the gap-closure plan).** The `Runnable::Agent` path still never binds the D-24 event
bus or enqueues a webhook delivery; this is now explicitly documented on `RunWorkerPool`'s
`event_bus`/`webhook_deliveries` fields and on `run_agent` itself, and pinned by
`agent_kind_run_with_a_webhook_enqueues_no_delivery` in `worker_tests.rs` plus a corresponding
"Known limitations" section in `docs/src/api-reference/platform-api.md`. Consistent with
`.planning/WINDOWS.md` ledger tracking.

**WR-03 — verified addressed by documentation only (as intended).** `run_controller.rs` gained a
"Read scope (WR-03)" module-doc section and `platform-api.md` gained a matching "Authentication and
scopes" paragraph; no code change. This matches `.planning/WINDOWS.md` row 32, which records the
same deviation as accepted for v0.10 (single-tenant/mutually-trusted-principal model) with an
explicit closing condition. No new code-level scoping was expected here, and none was needed.

**WR-04 — verified fixed.** `LeaseHeartbeat::spawn` now returns a handle with no background task
(`handle: None`) when `lease.is_zero()`, instead of the prior `interval = lease` (zero) which drove
`tokio::time::sleep(Duration::ZERO)` into a CPU-bound spin. `Drop` was updated to tolerate the
`Option`. Covered by `lease_heartbeat_with_a_zero_lease_never_extends`, which asserts zero
`extend_lease` calls over 200ms of real time.

Beyond re-verifying the four carried-over findings, this pass also confirmed a real, independently
fixed defect in `run_queue/redis.rs` (the claim script previously incremented `attempt` on every
claim, including the first — now gated behind a `_claimed` marker matching
`InMemoryRunQueue::dequeue`'s never-increment-on-first-claim semantics) and a CI-suite-isolation
defect in `run_queue/contract_tests.rs::run_all` (a shared queue instance leaked leases forward
between clauses; now takes a `fresh_queue` factory). Both are adequately tested and out of the
scope of new findings below.

One new issue was found: a signing-key handling inconsistency left behind by the WR-01 fix (see
below). It applies the same "sign-and-send-with-a-fallback-empty-key" pattern WR-01 just removed
from the `Err` arm, to the sibling `Ok(None)` arm, which the fix did not touch.

## Warnings

### WR-27-01: `Ok(None)` signing-key branch still sends with a fallback empty key, unlike the just-fixed `Err` branch

**File:** `src/application/services/run/webhook/service.rs:188-194`
**Issue:** `WebhookDeliveryService::process` reads the run that supplies this delivery's HMAC
signing secret via `self.runs.get(&delivery.run_id)`. The gap-closure fix for WR-01
(`27-REVIEW.md`) correctly changed the `Err(error)` arm to reschedule the delivery (`Retrying`,
backoff-delayed) rather than sign-and-send with a fallback empty key — but the adjacent `Ok(None)`
arm (the run row does not exist) was left unchanged:

```rust
let signing_key = match self.runs.get(&delivery.run_id).await {
    Ok(Some(run)) => run
        .webhook
        .as_ref()
        .and_then(|webhook| webhook.secret.clone())
        .unwrap_or_default(),
    Ok(None) => String::new(),          // <-- still falls through to sign-and-send
    Err(error) => {
        // WR-01 fix: reschedule instead, see above
        ...
        return;
    }
};
```

Both branches represent the exact same underlying hazard WR-01's own doc comment names
("signing with a fallback empty key and sending anyway would hand a receiver a payload it must
reject as mis-signed, while still burning one of the delivery's budgeted five attempts") — the fix
was applied to one branch and not its sibling. There is no code-level guarantee preventing
`Ok(None)`: `RunRepositoryPort` has no delete method today (so this is not reachable through any
currently-wired API), but there is also no foreign-key constraint between `webhook_deliveries.run_id`
and `runs.run_id` in either migration
(`crates/paladin-storage/migrations/{postgres,sqlite}/005_create_webhook_deliveries_table.sql`), so
an orphaned delivery row (e.g. from a future run-deletion feature, a cross-backend inconsistency, or
a data-repair script) silently ships a payload signed with an empty key rather than being rescheduled
or dead-lettered with a diagnosable `last_error`. This path also carries no `log::warn!` at all
(the `Err` arm does), so even the operational visibility WR-01 added for the sibling case is absent
here.

Untested: no test in `tests.rs` or `worker_tests.rs` exercises `Ok(None)`, unlike
`webhook_signing_key_load_failure_reschedules_without_sending`, which pins the `Err` arm.

**Fix:** Apply the same reschedule to `Ok(None)` that WR-01 applied to `Err`, sharing the retry/
dead-letter decision so the two failure shapes are handled identically:

```rust
let signing_key = match self.runs.get(&delivery.run_id).await {
    Ok(Some(run)) => run
        .webhook
        .as_ref()
        .and_then(|webhook| webhook.secret.clone())
        .unwrap_or_default(),
    Ok(None) => {
        log::warn!(
            "webhook delivery service: run {} not found for delivery {delivery_id}; \
             rescheduling rather than sending with a fallback empty key",
            delivery.run_id
        );
        let delay = chrono::Duration::from_std(backoff_for(new_attempt))
            .unwrap_or_else(|_| chrono::Duration::zero());
        self.finish(
            &delivery_id,
            WebhookAttemptResult {
                outcome: WebhookAttemptOutcome::Retrying {
                    next_attempt_at: (self.options.now)() + delay,
                },
                response_status: None,
                error: Some(bounded_error(&format!(
                    "run {} not found while loading signing key",
                    delivery.run_id
                ))),
            },
        )
        .await;
        return;
    }
    Err(error) => { /* unchanged */ }
};
```

Add a test mirroring `webhook_signing_key_load_failure_reschedules_without_sending`, using a
`RunRepositoryPort` double whose `get` returns `Ok(None)` instead of `Err`, asserting the same
zero-hits/`Retrying`/`next_attempt_at > now` shape.

## Info

### IN-27-01: Stale `eslint-disable-next-line` comment on an ES `import`, not a `require()`

**File:** `scripts/sdk-smoke/smoke.ts:30-31`
**Issue:** The import statement is preceded by `// eslint-disable-next-line @typescript-eslint/no-var-requires`, but the following line is an ES module `import`, not a `require()` call — `no-var-requires` only fires on `require(...)`. The directive is a no-op (harmless, since ESLint does not appear to run in this smoke-only package) but is misleading to a future reader and would itself trigger an "unused eslint-disable directive" lint if ESLint were ever wired up for this file.

```ts
// eslint-disable-next-line @typescript-eslint/no-var-requires
import { AssistantsApi, Configuration, RunsApi } from "paladin-sdk";
```

**Fix:** Remove the stale directive:

```ts
import { AssistantsApi, Configuration, RunsApi } from "paladin-sdk";
```

---

_Reviewed: 2026-09-08T18:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
