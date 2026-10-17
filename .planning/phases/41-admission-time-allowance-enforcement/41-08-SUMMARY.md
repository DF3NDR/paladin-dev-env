---
phase: 41-admission-time-allowance-enforcement
plan: 08
subsystem: treasurer
tags: [allowance, operator-webhook, webhook-delivery, ssrf, hmac, run-event-kind, semver, rust]

requires:
  - phase: 41-admission-time-allowance-enforcement
    plan: 01
    provides: the recorded design checkpoint (option-b, item 6 C3 Option A, D-17 payload amended to twelve keys) and the AllowanceAdmissionPort confirm / abandon contract
  - phase: 41-admission-time-allowance-enforcement
    plan: 03
    provides: AllowanceWebhookConfig { url, secret } with redacting Debug, APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET, build_run_api's Treasurer construction
  - phase: 41-admission-time-allowance-enforcement
    plan: 06
    provides: AllowanceNotice (tenant_id, api_key_id, run_id, recorded_at), Treasurer::with_notices, the once-per-window notice store and the no-op confirm this plan fills
  - phase: 41-admission-time-allowance-enforcement
    plan: 07
    provides: TraceEvent::AllowanceWarning, the herald allowance line, RunWorkerPool::with_treasury_notices and the single notice store handle build_run_api shares
provides:
  - RunEventKind::AllowanceWarning (#[non_exhaustive]) and RunEventKind::as_str, now the single to-string source for storage, both controllers and the delivery service
  - AllowanceWarningPayload (twelve keys, from_notice) beside WebhookPayload
  - WebhookDeliveryService::with_operator_notice_secret and the operator signing branch before any run lookup
  - OperatorNoticeTarget, OPERATOR_NOTICE_THREAD_ID and Treasurer::with_operator_webhook; confirm enqueues one allowance_warning delivery per won notice
  - the boot-time SSRF check of treasurer.allowance.webhook.url in build_run_api and the operator secret wired onto the delivery service
  - the integrated warn-path proof allowance_warn_path_tracer and the production-builder proof build_run_api_wires_the_allowance_warn_path
  - MIGRATION 9.2 / 9.6 entries (with the mixed-version rollout rule), allowlist entry, docs, CHANGELOG, refreshed public-API baseline
affects: [41-09]

tech-stack:
  added: []
  patterns:
    - "An operator-level notice reuses the durable webhook_deliveries queue through an event discriminator plus a correlation run id no run owns, so no schema change and no second HTTP client is needed"
    - "A signing secret that is not a run's own is held on the delivery service and applied by an event branch placed before the run lookup, never written to a row"
    - "RunEventKind::as_str is the one to-string source; adapters keep only the inverse parser"

key-files:
  created:
    - .planning/phases/41-admission-time-allowance-enforcement/41-08-SUMMARY.md
  modified:
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-storage/src/webhook/sqlite.rs
    - crates/paladin-storage/src/webhook/postgres.rs
    - crates/paladin-storage/src/webhook/in_memory.rs
    - crates/paladin-storage/src/webhook/contract_tests.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/schedule_controller.rs
    - src/application/services/run/webhook/mod.rs
    - src/application/services/run/webhook/service.rs
    - src/application/services/run/webhook/tests.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/infrastructure/web/run_api_wiring.rs
    - src/application/services/run/http_surface_tests.rs
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - docs/src/api-reference/platform-api.md
    - docs/src/deployment-topologies/http-service-host.md
    - docs/src/getting-started/configuration.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "Checkpoint option-b implemented everywhere: the operator payload carries tenant_id and api_key_id (name only), twelve keys, all always present (null where not applicable)"
  - "C3 Option A: the delivery row names a fresh RunId::new_v7() no run owns and the fixed thread treasurer-notices; the real admitting run id travels only in the payload"
  - "Operator signing branch sits after the send-time SSRF check and before runs.get, so an operator delivery never loops in Retrying and never consults a caller's secret"
  - "The boot SSRF check runs right after the disabled-store and waypoint checks, before any repository is built or task spawned, and only when a Treasurer will be built (an allowance entry exists)"
  - "The storage contract clause round-trips the row through enqueue and get with a far-future next_attempt_at and never calls claim_due, so it cannot be swept into another clause's batch on a shared PostgreSQL database"

patterns-established:
  - "Mutation evidence for wiring: dropping the operator secret from the delivery service or returning early from confirm fails the integrated tests"

requirements-completed: [ALLOW-04]

duration: ~2h
completed: 2026-10-04
status: complete
---

# Phase 41 Plan 08: Operator allowance webhook and the end-to-end warn path Summary

**Every warn-threshold notice an admission wins is now delivered once to the operator's URL as a signed `allowance_warning` webhook through the existing durable, SSRF-guarded, no-redirect delivery queue (HMAC over the exact stored bytes with an operator secret held only on the delivery service), never listed under the admitting run, and `allowance_warn_path_tracer` proves one admission at 80% yields exactly one notice row, one signed delivery, one trace event before `RunStarted` and one herald line, with a second admission adding nothing.**

## Performance

- **Duration:** ~2 h (warm workspace; PostgreSQL leg run for real)
- **Completed:** 2026-10-04
- **Tasks:** 3 (all `tdd="true"`)
- **Files:** 1 created (this summary), 21 modified

## Accomplishments

- **Event kind and plumbing (Task 1).** `RunEventKind::AllowanceWarning` plus `#[non_exhaustive]` and `as_str` (doc example, unit test asserting it equals the serde string for all six variants). The five hand-written to-string matches (`event_to_str` in both storage adapters, `event_kind_label` in both controllers, `event_wire_name` in the delivery service) now delegate to `as_str`; both `event_from_str` functions accept `allowance_warning`; both controllers' `parse_event_kind` keep rejecting it, proven by `caller_cannot_subscribe_a_run_webhook_to_allowance_warning` and the schedule twin.
- **Payload.** `AllowanceWarningPayload` (twelve keys, always present, `null` where not applicable) with `from_notice` (`timestamp` = the notice's store instant), a rustdoc example, `allowance_warning_payload_serializes_with_exactly_the_documented_keys` (sorted-key equality, both scopes), a run-id/`null` test and `allowance_warning_payload_has_no_secret_or_input`.
- **Delivery service.** `with_operator_notice_secret`; in `process`, after the send-time SSRF check and before the run lookup, an `AllowanceWarning` delivery takes the operator secret (or the empty key) and skips `runs.get` entirely. Run deliveries are byte-for-byte unchanged. Tests: `operator_delivery_is_signed_with_the_operator_secret_without_a_run_lookup` (a `NoLookupRunRepository` double counts and fails `get`), `operator_delivery_without_a_secret_is_signed_with_the_empty_key`, `run_delivery_signing_is_unchanged` (an operator secret configured, the run's own secret still signs a run delivery).
- **Storage contract clause.** `operator_allowance_delivery_round_trips_and_is_not_listed_for_other_runs`, wired on the in-memory, SQLite and PostgreSQL adapters (PostgreSQL ran for real).
- **Enqueue on confirm (Task 2).** `OPERATOR_NOTICE_THREAD_ID`, `OperatorNoticeTarget` (manual `Debug` printing the URL only, no secret field), `Treasurer::with_operator_webhook`; `confirm` serializes `AllowanceWarningPayload` once and enqueues one row per won notice with a correlation `RunId` no run owns. A serialization, thread-id or enqueue failure is `log::error!`ed (scope kind, tenant id, error) and swallowed; no target means no enqueue; `abandon` never enqueues. Six treasurer tests.
- **Wiring.** `build_run_api` SSRF-checks `treasurer.allowance.webhook.url` at boot (fail closed, message names `treasurer.allowance.webhook.url` and `webhooks.allow_private`, never the secret), attaches the `OperatorNoticeTarget` over the same `webhook_repository` the run webhooks use, and chains `.with_operator_notice_secret(..)` onto the one `WebhookDeliveryService`. Tests: private target rejected without `allow_private`, accepted with it, the metadata address rejected even with it, no webhook builds no target, and `build_run_api_wires_the_allowance_warn_path` (a key at exactly 80%, `POST /v1/runs` -> 202, exactly one notice row, the spawned drain loop delivers exactly one POST whose HMAC verifies with `op-secret`).
- **Integrated proof (Task 3).** `allowance_warn_path_tracer` over ONE on-disk SQLite file: config-built `Treasurer` + `with_notices` + `with_operator_webhook`, the real `run_router`, a `WebhookDeliveryService` and a `RunWorkerPool` on the `with_engine_factory` path with `with_treasury_notices`, a persisting trace store and a recording herald double. Asserts in order: 202; exactly one notice row (api_key, window, `0.8000 USD` of `1.0000 USD`, `warn_at` 80, the admitted run id); an empty `GET /v1/runs/{id}/webhook-deliveries`; exactly one signed POST (`X-Paladin-Event: allowance_warning`, HMAC over the captured bytes, the twelve keys, `run_id` the admitted run, no key value or secret); one worker dispatch yields exactly one `allowance_warning` trace record with `seq` below `RunStarted` and exactly one herald `allowance:` line, run completed; a second POST in the window adds no notice, no delivery (receiver count stays 1), no trace record and no herald line.
- **Registers and docs (D-20).** MIGRATION.md 9.2 rows (`RunEventKind` `Y` with its allowlist entry, `WebhookDeliveryService` `N`) and a 9.6 operator-notice entry carrying the rollout rule; the platform-api.md "Operator allowance notices" subsection (payload, headers, signature, semantics, the agent-route caveat); the http-service-host.md "Treasurer allowances" rollout section; configuration.md's "delivery lands later" wording replaced with the live behaviour; the CHANGELOG paragraph; the refreshed `.project/current-exports.txt` (4161 to 4187 items, every added line is this plan's symbol; `make api-surface` exits 0).

## Task Commits

1. **Task 1: operator event kind, payload, signing branch** - `1994953` (feat)
2. **Task 2: enqueue on confirm, boot SSRF check, secret wiring, production proof** - `4f82e3d` (feat)
3. **Task 3: end-to-end warn-path tracer, registers, docs, baseline** - `bccba43` (feat)

## TDD / red evidence

- Honest note: Tasks 1 and 2 wrote implementation and tests together in one pass each; there was no separate red-first commit. Compensating mutation evidence, each reverted afterwards (the file restored from a copy and `git diff` empty):
  - `build_run_api_wires_the_allowance_warn_path`: replacing the operator secret passed to the delivery service with `None` failed the test at `signed with the operator secret`.
  - `allowance_warn_path_tracer`: an early `return` in `Treasurer::confirm` failed it at the operator-delivery assertion (`left: 0, right: 1`).

## Verification

- `cargo test -p paladin-ai-core --lib container::run` (45) and `--doc run` (9) pass; `cargo test -p paladin-storage --features sqlite --lib webhook::` 23 passed.
- **PostgreSQL ran for real** on the throwaway cluster (port 5433): `cargo test -p paladin-storage --features sqlite,postgres --lib webhook::postgres -- --test-threads=1` **11 passed, 0 SKIP**, including the new clause. See deviation 3 for the shared-database caveat.
- `cargo test -p paladin-web --lib` 278 passed; `cargo test -p paladin-ai --lib application::services::run::webhook` 32 passed; `--lib application::services::treasurer` 43 passed; `--doc treasurer` 11 and `--doc AllowanceWarningPayload` 1 passed; `--features web-server --lib infrastructure::web::run_api_wiring` 25 passed; `allowance_warn_path_tracer` and `allowance_admission_tracer` pass.
- `cargo test --workspace` (default features): **6344 passed, 0 failed** (6317 after 41-07).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `./scripts/check-migration-allowlist.sh` exit 0; no `TBD` in MIGRATION.md; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exits 0 after `make api-surface-update`; **`make security` exit 0** (advisories, bans, licenses, sources ok).
- Acceptance greps: `grep -c 'RunEventKind::Cancelled => "cancelled"'` is 0 in all five files; the pre-`#[cfg(test)]` part of `run_controller.rs` has 0 `"allowance_warning"`; `grep -rn "reqwest::Client::\|ClientBuilder::new" src/application/services/treasurer/` prints 0; `OperatorNoticeTarget` is in `.project/current-exports.txt`.
- Manual credential-handling review (security.instructions.md webhook checklist): the URL passes `SsrfGuard` at boot and again in `WebhookDeliveryService::process` (the existing send-time check, untouched); delivery uses only the existing `build_webhook_client` (`Policy::none()`) -- no new client anywhere in the treasurer module; the HMAC is over `delivery.payload`, the exact bytes stored at enqueue and sent verbatim; the operator secret lives only in `WebhookDeliveryService.operator_notice_secret` (no `Debug` impl on the service, never on a row, never in `OperatorNoticeTarget`, `AllowanceWebhookConfig`'s redaction from 41-03 unchanged); the boot error never echoes the secret (asserted); log lines carry scope kind, tenant id and the adapter error only; the payload carries the API key NAME (never a value) and `api_key_id` is the one key the secret-or-input key-name test explicitly exempts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking/design] Storage clause avoids `claim_due`**
- **Found during:** Task 1 PostgreSQL run
- **Issue:** the plan's `claim_due`-free clause name left room for a final `claim_due` assertion; on the shared PostgreSQL database a due operator row would be swept into another clause's `claim_due` batch (the existing claim clauses already assume a clean table).
- **Fix:** the clause round-trips through `enqueue`, `get` and `list_for_run` only, with a far-future `next_attempt_at`, so it never perturbs or is perturbed by the claim clauses. The shared decoder `get` uses is the one `claim_due` uses, so the unknown-event risk is still covered.
- **Commit:** `1994953`

**2. [Rule 2 - Missing critical] Boot SSRF check placed before anything spawns**
- **Found during:** Task 2
- **Issue:** checking inside the Treasurer branch (after the worker pool is spawned) would leave spawned tasks behind on a fail-closed boot error.
- **Fix:** the check sits right after the disabled-store and waypoint-store guards, before any repository is built, and only when an allowance entry exists (a configured webhook with no entry builds nothing and is not checked). Added `build_run_api_rejects_the_metadata_address_even_with_allow_private`.
- **Commit:** `4f82e3d`

### Environment notes (not code deviations)

**3. PostgreSQL shared-database caveat.** The existing webhook `claim_due_*` clauses assume an empty `webhook_deliveries` table and fail when run in parallel or against rows left by earlier runs (they failed identically before my clause's rows existed, once enough stale due rows had accumulated). I deleted all rows from `webhook_deliveries` on the throwaway `paladin_run_test` database and ran the PostgreSQL module with `--test-threads=1` (the way CI runs the other PostgreSQL modules); 11 passed. Pre-existing, not changed here. The throwaway cluster's table has the new clause's far-future row afterwards, which is harmless to the claim clauses.

**4. Delivery poll interval.** `WebhooksConfig` has no poll-interval key (the service's 5 s default applies), so the wiring test waits up to 10 s for the drain loop rather than shortening an interval.

**5. Commit trailers.** The three commits carry `Co-Authored-By: Claude Fable 5.1` plus the session line, as the dispatch note instructs.

**Total deviations:** 1 blocking/design, 1 missing-critical, 3 notes; no scope change.

## Authentication Gates

None.

## Known Stubs

None. `Treasurer::confirm` is no longer a no-op.

## Threat Flags

None beyond the plan's register. T-41-38 (SSRF) is mitigated at boot (`build_run_api_rejects_a_private_operator_webhook_without_allow_private`, the metadata-address test) and at send time by the unchanged service check; DNS rebinding stays the documented, accepted limitation of `ssrf.rs`. T-41-39 (secret disclosure) by holding the secret only on the service and the no-secret payload tests. T-41-40 by `operator_delivery_is_signed_with_the_operator_secret_without_a_run_lookup`. T-41-41 by the correlation run id, `operator_delivery_is_not_listed_for_the_admitting_run`, the storage clause, the empty listing in the tracer and the caller parsers' rejection. T-41-42 by the grep (no second client). T-41-43 by the rollout rule in MIGRATION.md 9.6 and http-service-host.md.

## Notes for later plans

- `requirements-completed: [ALLOW-04]`. All three legs of ALLOW-04 (trace event, herald line, operator webhook) now exist and are proven together; I marked it complete in REQUIREMENTS.md. 41-09 also lists ALLOW-04 in its frontmatter, so its closeout should not need to re-mark it (the command is idempotent either way). ALLOW-01 stays for 41-09.
- 41-09 owns the `src/core/platform/mod.rs` re-export and the consolidated allowance register row; this plan added only the `RunEventKind` and `WebhookDeliveryService` rows.
- The PostgreSQL webhook clauses need `--test-threads=1` and a clean `webhook_deliveries` table (see deviation 3).

## Self-Check: PASSED

Commits `1994953`, `4f82e3d` and `bccba43` are present in `git log`; `OperatorNoticeTarget`, `with_operator_webhook`, `with_operator_notice_secret`, `allowance_warn_path_tracer` and `build_run_api_wires_the_allowance_warn_path` exist in the tree; the `.cargo/semver-checks-allowlist.toml` entry and the MIGRATION.md rows are present and the set-equality script exits 0.
