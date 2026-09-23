---
phase: 27
slug: platform-api
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 0
asvs_level: 1
created: 2026-09-08
last_audit: 2026-09-08
---

# Phase 27 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

**Verdict: THREAT-SECURE.** All 107 threats closed. The one `high` threat that blocked the
previous audit (T-27-22-02) was remediated in commit `4b6592de` and re-verified here; no open
threat remains at or above the `high` blocking threshold.

Register origin: `register_authored_at_plan_time: true` — 25 of the 26 plans in this phase
carried a parseable `<threat_model>` block (27-26 is a test-only gap-closure plan). The
auditor therefore verified mitigations rather than building a retroactive register.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HTTP client → `paladin-web` | Untrusted JSON request bodies (`assistant_id`, `thread_id`, arbitrary `input`) | Caller-controlled strings, webhook URLs and secrets |
| `paladin-web` → facade ports | Deserialized-but-unvalidated core values cross the crate boundary | Core domain values |
| Facade → run store / run queue | Caller-influenced values become persisted rows and queue messages | Rows, queue payloads |
| Facade → SQL backends (SQLite / Postgres) | Caller strings become bound parameters | Thread ids, assistant ids, JSON input, webhook URLs |
| Concurrent workers / replicas → one `runs` table | Two processes may race on the same row or thread | Status transitions, lease claims |
| Worker processes ↔ Redis | Queue messages and lease tokens cross a network boundary | Run pointers, lease tokens |
| Connection strings / Redis URL → error messages | Backend errors may embed a password | Credentials |
| Paladin → caller-chosen webhook receiver | **Outbound HTTP to an attacker-influenceable URL carrying a credential-shaped header** | Signed payload, `X-Paladin-Signature` |
| Schedule definitions → run submission | Admin-authored cron specs and webhook URLs become outbound requests | Webhook URLs, secrets |
| Assistant definition documents → engine | A document names Rust behaviour to execute | `WarGraphDoc` node kinds |
| CI job → external registries / LLM endpoints | `npm ci` / `pip install` and generated-SDK smoke traffic | Package integrity, API keys |

---

## Threat Register

107 threats across plans 27-01..27-25. Verified by `gsd-security-auditor` at ASVS L1 depth;
the two orchestrator-flagged items (the `signature.rs` fallback and dual-site SSRF placement)
received L2-depth data-flow tracing. T-27-22-02 was re-verified at L2 depth in the 2026-09-08
re-audit after commit `4b6592de` — see *Closed Threat — T-27-22-02* below.

| Threat ID | Plan | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|------|----------|-----------|----------|-------------|------------|--------|
| T-27-01 | 27-01 | Tampering | `RunStatus::try_transition` / `update_status` | high | mitigate | Status changes are only reachable through `try_transition`; the InMemory adapter routes every write through it so an out-of-band status write cannot compile. Exhaustive cross-pr... | closed |
| T-27-02 | 27-01 | Information disclosure | `Run.input`, `WebhookSpec.secret`, error bodies | high | mitigate | `WebhookSpec` gets a manual `Debug` that redacts `secret`; no handler interpolates `input` into an error message; errors render only through `ApiError`, whose `details` slot car... | closed |
| T-27-03 | 27-01 | Elevation of privilege | `POST /v1/runs` | high | mitigate | `RunApiState` carries `AgentAuthConfig` and the router is mounted behind the same auth middleware as `/v1/agents/*`; scope enforcement itself is completed in plan 27-15, and unt... | closed |
| T-27-04 | 27-01 | Denial of service | `POST /v1/runs` handler | medium | mitigate | The handler performs one insert and one enqueue and touches no engine type, so a submit cannot consume worker time; asserted by the source-level grep in Task 3's acceptance crit... | closed |
| T-27-05 | 27-01 | Repudiation | run identity | low | accept | `RunId` is a UUIDv7; per-run actor attribution is a Phase 28 observability concern (OBS-01) and no PLAT FR requires it here. | closed |
| T-27-02-01 | 27-02 | Tampering | `runs.status` under concurrent writers | high | mitigate | Every transition is a single CAS `UPDATE … WHERE status = ?from` (D-04); `rows_affected() == 0` → `IllegalTransition`; contract clause + multi-thread stress test. | closed |
| T-27-02-02 | 27-02 | Tampering / DoS | one-active-run-per-thread invariant | high | mitigate | Partial unique index `idx_runs_thread_active` enforced by the database (D-17); `is_unique_violation()` mapped to `ThreadBusy`; ten-concurrent-inserts test on an on-disk SQLite f... | closed |
| T-27-02-03 | 27-02 | Tampering | SQL construction from caller strings | high | mitigate | Only parameterised `sqlx::query`/`query_as`; acceptance grep forbids `format!("SELECT…` style SQL. | closed |
| T-27-02-04 | 27-02 | Information disclosure | connection-string password in `Backend` errors | medium | mitigate | `waypoint::redact::redact_database_url_password` reused on every wrapped error (primary trust boundary, so mitigated at L1). | closed |
| T-27-02-05 | 27-02 | Information disclosure | `webhook.secret` persisted in the `runs` row | medium | accept | The secret must be available at send time (27-13) and the row is only readable through the authenticated API, which projects `WebhookSpec` with the secret redacted (27-01's manu... | closed |
| T-27-03-01 | 27-03 | Tampering | claim-and-lease across replicas | high | mitigate | Single atomic Lua `EVAL` for claim, extend, ack and nack using server `TIME`; contract clause `concurrent_workers_each_message_exactly_once` run with two adapter instances. | closed |
| T-27-03-02 | 27-03 | Spoofing | lease tokens | medium | mitigate | Tokens are UUID v7 strings generated per claim and validated against `{prefix}:leases`/`lease_expiry` inside the script; unknown/expired tokens are typed errors that touch nothi... | closed |
| T-27-03-03 | 27-03 | Information disclosure | Redis URL password in `Backend` errors / `Debug` | medium | mitigate | `RedisRunQueueConfig` custom `Debug` redacts; every error message passes through the redaction helper before construction. | closed |
| T-27-03-04 | 27-03 | Denial of service | unbounded queue depth | low | accept | Depth is bounded by the number of rows the `runs` table admits (one active run per thread, D-17) and by API auth + rate limiting (27-15); no separate cap is introduced this phase. | closed |
| T-27-03-05 | 27-03 | Repudiation | Tier-2 suite reported green without running | medium | mitigate | CI `SKIP:` detector + declared-vs-selected test count on the `redis-queue` job (the Snyk lesson in `security.instructions.md`). | closed |
| T-27-04-01 | 27-04 | Tampering | queue message contents | high | mitigate | The worker never trusts the message payload: it re-reads the `Run` through `RunRepositoryPort` and branches on the persisted status; a stale/terminal message is ACKed and dropped. | closed |
| T-27-04-02 | 27-04 | Elevation of privilege | duplicate execution via redelivery | high | mitigate | Dispatch resumes from the latest Waypoint (D-09); `worker_pool_lease_expiry_exactly_once` asserts every node ran exactly once. | closed |
| T-27-04-03 | 27-04 | Denial of service | lost work on shutdown or crash | medium | mitigate | Drain via `ShutdownCoordinator` + NACK(0) on shutdown-halt (D-13); lease expiry redelivers on crash — primary boundary, mitigated. | closed |
| T-27-04-04 | 27-04 | Information disclosure | `EngineError` text stored in `Run.error` | low | accept | Engine errors name node ids and limits, not credentials (Phase 25's redaction of provider bodies happens below the engine, `paladin-llm/src/redaction.rs`); the string is bounded... | closed |
| T-27-05-01 | 27-05 | Tampering | `WarGraphDoc` → `WarGraph` | high | mitigate | `deny_unknown_fields`, typed `CompileError` for every unresolved name or structural fault, `validate()` invoked inside `compile()` — nothing partially compiled is ever returned. | closed |
| T-27-05-02 | 27-05 | Elevation of privilege | naming arbitrary Rust behaviour from a document | high | mitigate | No `function` node kind exists; only names present in `EngineRegistries` resolve, and only kinds `paladin`/`gate`/`workflow` compile (D-33 correction). | closed |
| T-27-05-03 | 27-05 | Denial of service | pathological documents (deep nesting, huge limits) | medium | mitigate | `LimitsDoc` maps onto `EngineLimits` whose `validate` rejects `max_supersteps == 0`; nested `workflow` depth is bounded by a `MAX_NESTING_DEPTH` (8) check in `compile()` returni... | closed |
| T-27-05-04 | 27-05 | Information disclosure | `system_prompt` in stored documents | low | accept | Documents are admin-authored and read only through authenticated routes (27-12); prompts are configuration, not credentials, and no credential field exists in the document. | closed |
| T-27-06-01 | 27-06 | Information disclosure | Postgres/Redis URLs in config/`Debug`/logs | medium | mitigate | Backends carry `url_env` (the variable NAME) only; `validate()` checks presence without storing the value — the `waypoint_store.rs` pattern (primary boundary). | closed |
| T-27-06-02 | 27-06 | Elevation of privilege | `allow_private` flipped on by accident | medium | mitigate | Defaults `false`; the env var is parsed as a strict boolean; rustdoc names the consequence. | closed |
| T-27-06-03 | 27-06 | Denial of service | pathological worker/lease/timeout values | low | mitigate | `validate()` rejects zero concurrency, sub-4 s leases, zero intervals and zero attempts/timeouts. | closed |
| T-27-07-01 | 27-07 | Denial of service | probe hammering the database on fast graphs | medium | mitigate | `DbCancellationProbe` caches per thread for `min_probe_interval` (D-15); counting-mock test. | closed |
| T-27-07-02 | 27-07 | Tampering | lost cancel between flag write and local signal | medium | mitigate | Flag first, signal second (D-16); the flag alone is sufficient because the probe reads it. | closed |
| T-27-07-03 | 27-07 | Denial of service | probe failure failing a run | high | mitigate | Signature is infallible; adapter returns `false` on error with a `warn` log; unit test. | closed |
| T-27-07-04 | 27-07 | Elevation of privilege | cancelling another caller's run | medium | transfer | Authorisation belongs to the HTTP layer (27-15, D-46 invocation tier over the same auth middleware as `/v1/agents/*`); the service is process-internal. | closed |
| T-27-08-01 | 27-08 | Tampering | unvalidated responses reaching the engine | high | mitigate | Phase 24's `shadow_validate` runs synchronously before `record_resume`; the four 400 variants are unchanged. | closed |
| T-27-08-02 | 27-08 | Tampering | resume racing a cancel / double resume | medium | mitigate | `record_resume` is a CAS on `status = 'awaiting_input'`; the loser gets `ThreadNotAwaitingInput` (409). | closed |
| T-27-08-03 | 27-08 | Denial of service | resume creating a second active run on the thread | medium | mitigate | Resume is an `UPDATE` of the existing run — no insert, so the busy index is never a factor (D-19). | closed |
| T-27-08-04 | 27-08 | Information disclosure | `run_id` in the response | low | accept | Run ids are UUID v7 handles to authenticated resources; disclosure to the resuming principal is the feature. | closed |
| T-27-09-01 | 27-09 | Tampering | version mutation | high | mitigate | No update method on the port, no UPDATE statement on `assistant_versions` in any adapter (grep gate), PK `(assistant_id, version)`. | closed |
| T-27-09-02 | 27-09 | Tampering | torn `latest` at submit | high | mitigate | Single-statement `INSERT … SELECT latest` on SQL backends; `assistant_version_freeze_at_submit` concurrency clause. | closed |
| T-27-09-03 | 27-09 | Repudiation | who published what | medium | mitigate | `created_by`/`created_at`/`note` mandatory columns; `list_versions` is the changelog (PLAT-FR-10). | closed |
| T-27-09-04 | 27-09 | Information disclosure | definition bodies (prompts) | low | accept | Admin-authored configuration read only through authenticated routes; no credential field exists in either kind's body (27-12 prohibition). | closed |
| T-27-10-01 | 27-10 | Information disclosure | `state_delta` payloads | high | mitigate | Field names, superstep and byte counts only — no values, no vault-confined data, no `input`; `state_delta_carries_field_names_only` test; prohibition P1. | closed |
| T-27-10-02 | 27-10 | Denial of service | slow consumer back-pressure on the engine | high | mitigate | `tokio::sync::broadcast` with capacity 64, drop-oldest with a `dropped` count; `on_event` never awaits a receiver. | closed |
| T-27-10-03 | 27-10 | Denial of service | unbounded degraded polling | medium | mitigate | Poll at `poll_interval` (default 1 s), terminate on terminal status, one stream per request under the existing rate limiter (primary boundary). | closed |
| T-27-10-04 | 27-10 | Spoofing | streaming another principal's run | medium | transfer | Same auth middleware as `/v1/agents/*`; per-run ownership scoping is a deferred idea (D-46, §9.6). | closed |
| T-27-11-01 | 27-11 | Tampering / DoS | duplicate firing across replicas or restarts | high | mitigate | Conditional-update tick claim (D-37) + persisted `next_tick`; `two_services_one_tick_exactly_one_fire` and `schedule_restart_exactly_once`. | closed |
| T-27-11-02 | 27-11 | Denial of service | pathological cron (`* * * * * *` every second) | medium | mitigate | Parsing is bounded by `croner`; the busy-thread invariant and `FixedThread` skip counting bound concurrent runs; a per-schedule minimum interval is not an FR — documented as a... | closed |
| T-27-11-03 | 27-11 | Tampering | unbounded catch-up after downtime | medium | mitigate | `OnMissed::Skip` default; `RunOnce` fires at most one run per missed window (PRD policy). | closed |
| T-27-11-04 | 27-11 | Information disclosure | schedule `input`/`webhook.secret` rows | low | accept | Same posture as `runs` rows (T-27-02-05): admin-authored, authenticated reads, DTO redacts secrets. | closed |
| T-27-12-01 | 27-12 | Tampering | invalid or malicious definitions persisted | high | mitigate | Compile-is-validation before any repository write; `{}` → 400 + nothing persisted (test). | closed |
| T-27-12-02 | 27-12 | Elevation of privilege | non-admin publishing assistants | high | mitigate | `require_admin` on create/version/delete (D-46); 403 test. | closed |
| T-27-12-03 | 27-12 | Elevation of privilege | invoking an assistant outside its `allowed_roles` | medium | mitigate | `RunSubmissionService` enforces `allowed_roles` from the resolved definition → `Forbidden` (403 in 27-15). | closed |
| T-27-12-04 | 27-12 | Information disclosure | credentials smuggled into definition bodies | medium | mitigate | `credential_field_forbidden` violation on `api_key`/`token`/`secret`/`authorization` keys (prohibition P1). | closed |
| T-27-12-05 | 27-12 | Tampering | mutating code-registered entries | medium | mitigate | 409 `code_registered_immutable`; `AgentRegistry` untouched (X-03). | closed |
| T-27-13-01 | 27-13 | Spoofing / Elevation | SSRF via webhook URL (write and send, incl. redirects) | critical | mitigate | `SsrfGuard` table at write time and on resolved addresses at send time; `Policy::none()` client; metadata IP always rejected even with `allow_private`; DNS rebinding documented ... | closed |
| T-27-13-02 | 27-13 | Tampering | signature mismatch via re-serialisation | high | mitigate | Payload stored as the exact string; signed and sent from that buffer; receiver-side verification test. | closed |
| T-27-13-03 | 27-13 | Information disclosure | secret / API key / input in payloads or rows | high | mitigate | No `secret` field on `WebhookDelivery`; payload struct has a fixed key set; `webhook_payload_has_no_secret_or_input`; `last_error` redacted before truncation. | closed |
| T-27-13-04 | 27-13 | Information disclosure | credential header forwarded on redirect | high | mitigate | Redirects disabled; `webhook_client_no_redirects` test; 3xx dead-letters. | closed |
| T-27-13-05 | 27-13 | Denial of service | retry storms / slow receivers on the run path | medium | mitigate | Bounded 5 attempts with capped backoff; drain loop off the completion path; per-request timeout; batch-limited `claim_due` (primary boundary). | closed |
| T-27-13-06 | 27-13 | Repudiation | which attempts happened | low | mitigate | Every attempt persisted with status/response/error and queryable. | closed |
| T-27-14-01 | 27-14 | Elevation of privilege | non-admin creating schedules | high | mitigate | `require_admin` on create/patch/delete (D-46); 403 test. | closed |
| T-27-14-02 | 27-14 | Spoofing / Elevation | SSRF through a schedule webhook | critical | mitigate | Write-time `SsrfGuard::check_url` in `create`/`patch`; send-time guard in 27-13's service. | closed |
| T-27-14-03 | 27-14 | Information disclosure | webhook secret echoed in responses | medium | mitigate | `ScheduleResponse` redacts the secret; test asserts `"***"`. | closed |
| T-27-14-04 | 27-14 | Denial of service | absurd cron frequency | low | accept | See T-27-11-02; operator concern documented in 27-16. | closed |
| T-27-15-01 | 27-15 | Elevation of privilege | mutating routes without proper scope | high | mitigate | `require_admin` on assistant/schedule/thread-delete; `authorize_invoke`/`allowed_roles` on submit/cancel/fork; `run_controller_auth` tests. | closed |
| T-27-15-02 | 27-15 | Denial of service | unbounded pagination / cursor abuse | high | mitigate | `resolve_limit` 1..=100 typed 400; `decode_cursor` typed 400 with no scan; keyset queries hit indexes. | closed |
| T-27-15-03 | 27-15 | Denial of service | request floods | medium | mitigate | Same tower-governor layer as `/v1/agents/*`; 429 proven on the new router (primary boundary). | closed |
| T-27-15-04 | 27-15 | Information disclosure | cursor internals / secrets in responses | medium | mitigate | Generic "invalid cursor" message; `webhook.secret` redacted in every DTO. | closed |
| T-27-15-05 | 27-15 | Spoofing | cross-principal run/thread access | medium | accept | Per-resource ownership scoping is an explicit deferred idea (D-46, §9.6); all routes still require authentication. | closed |
| T-27-16-01 | 27-16 | Information disclosure (mis-documentation) | SSRF / DNS-rebinding coverage claims | medium | mitigate | The page names resolve-then-connect pinning as NOT implemented (D-42); grep gate on "rebinding". | closed |
| T-27-16-02 | 27-16 | Spoofing (deployment) | multi-replica API with in-process token store | medium | mitigate | Topology page states ADR-0041's single-replica scope; grep gate. | closed |
| T-27-16-03 | 27-16 | Information disclosure | secrets in example manifests | low | mitigate | Manifest references `paladin-secrets` names only; no literal credentials (grep for `sk-`/`password:` is 0). | closed |
| T-27-17-01 | 27-17 | Elevation of privilege | new routes reachable without auth | high | mitigate | `run_router` merged before `with_http_layers` and carries the same `require_authentication` layer; binary test asserts 501-not-404 and auth wiring. | closed |
| T-27-17-02 | 27-17 | Information disclosure | connection strings / API keys in logs | high | mitigate | Only `url_env` names are logged; provider keys stay inside `Settings`-built adapters (existing redaction rules); grep gate for `NoRegisteredGraphsPaladinPort` misuse only. | closed |
| T-27-17-03 | 27-17 | Denial of service | services left running after shutdown | medium | mitigate | All tasks registered with the coordinator; drained on SIGTERM within grace. | closed |
| T-27-17-04 | 27-17 | Tampering | silent fallback on missing feature | medium | mitigate | Fail-closed startup errors naming the feature (precedent). | closed |
| T-27-18-01 | 27-18 | Tampering | unpinned generator image | medium | mitigate | Pinned `v7.x` tag recorded in the job; documented npm fallback also pinned. | closed |
| T-27-18-02 | 27-18 | Repudiation | vacuous green SDK job | medium | mitigate | Non-empty tree guards, compile steps, real 202 + terminal status assertions (prohibition P1). | closed |
| T-27-18-03 | 27-18 | Information disclosure | API key in CI logs | low | mitigate | A throwaway static key from `config.test.yml` only; never a real provider key (`LLM_API_KEY=test-key`, the `e2e-tests` job precedent). | closed |
| T-27-18-04 | 27-18 | Tampering | supply-chain on `pip install`/`npm ci` of generated code | high | mitigate | Installs only the generated tree plus `typescript`; no third-party runtime packages beyond the generator's declared deps; `cargo-deny`/`audit` unaffected (no Rust deps added). | closed |
| T-27-19-01 | 27-19 | Tampering | `RUN_QUEUE_CLAIM_LUA` attempt accounting | high | mitigate | The increment is gated on a marker the script itself sets; the shared contract suite asserts 1 → 2 → 3 across first claim, reclaim and second reclaim, on both backends, unmo... | closed |
| T-27-19-02 | 27-19 | Tampering | marker key colliding with a real `QueuedRun` field | medium | mitigate | The marker is an underscore-prefixed key that matches no `QueuedRun` field; `claim_marker_is_ignored_by_queued_run_deserialization` proves a member carrying it still deserialize... | closed |
| T-27-19-03 | 27-19 | Denial of service | a member stuck invisible after a script edit | medium | mitigate | The claim script's `ZREM`/`ZADD` re-score path is untouched; only the payload's attempt/marker handling changes, so visibility scheduling is unaffected. | closed |
| T-27-19-04 | 27-19 | Repudiation | retry history mis-stated to an operator | medium | mitigate | `attempt` now counts deliveries identically on both backends, so a run's recorded attempt is backend-independent. | closed |
| T-27-20-01 | 27-20 | Tampering | silent, driver-defined timestamp mutation on write | medium | mitigate | `storage_timestamp` normalises before the bind, so the stored value is decided by this codebase, not by whether the driver or server rounds; a dedicated test asserts truncation ... | closed |
| T-27-20-02 | 27-20 | Repudiation | a run's recorded start/finish time disagreeing with what the caller submitted | medium | mitigate | The contract suite asserts exact equality for all three fields on every backend; fixtures are stamped at the persisted resolution rather than the assertion being loosened. | closed |
| T-27-20-03 | 27-20 | Information disclosure | pagination cursor overlap/gap under equal timestamps | low | mitigate | The `(submitted_at DESC, run_id DESC)` keyset and the existing tie-exercising pagination clause remain unchanged and are re-run on both backends. | closed |
| T-27-20-04 | 27-20 | Denial of service | a weakened assertion masking a broken adapter | medium | accept | Accepted only in the sense that no automated gate can prove an assertion was not weakened; the plan's prohibition plus the explicit `assert_eq!` grep criterion is the control. | closed |
| T-27-21-01 | 27-21 | Spoofing / Information disclosure | outbound LLM call from CI with a caller-visible config | high | mitigate | The provider base URL is a loopback stub started by the job itself; no request leaves the runner and no real credential is needed or accepted. | closed |
| T-27-21-SC | 27-21 | Tampering | npm/pip installs in the job | high | mitigate | A committed `package-lock.json` makes `npm ci` an integrity-pinned install (an improvement on the current unpinned state); no new package is introduced — `typescript` and `@ty... | closed |
| T-27-21-02 | 27-21 | Repudiation | a green job that proved nothing | high | mitigate | Success requires the exact `completed` status; the failure path prints the status and the run error; a self-test proves the decision function itself. | closed |
| T-27-21-03 | 27-21 | Information disclosure | a real credential leaking through the committed config | medium | mitigate | The config's key is a fixed placeholder and the endpoint is loopback, so a copied config cannot carry or exercise a real secret. | closed |
| T-27-21-04 | 27-21 | Denial of service | stub or server left running after a failure | low | mitigate | Both processes are started and stopped by one library with an `EXIT` trap in every caller. | closed |
| T-27-22-01 | 27-22 | Denial of service | unbounded response-body read on the failure path (CR-01) | critical | mitigate | `read_bounded_body` caps the read at `MAX_ERROR_BODY_BYTES` as it streams; `Content-Length` is not trusted; a mockito body an order of magnitude over the cap proves the stop. | closed |
| T-27-22-02 | 27-22 | Tampering / Repudiation | delivery sent with a signature from an unloadable key (WR-01 / WR-27-01) | high | mitigate | **Both** non-sending arms now reschedule before any request: the `Err` arm (WR-01, `c2fe4afd`) and the `Ok(None)` arm (WR-27-01, `4b6592de`) each `log::warn!` and finish `Retrying`. Receiver mocks with `expect(0)` prove no send on either path. | closed |
| T-27-22-03 | 27-22 | Information disclosure | a credential inside a receiver's error page persisted in `last_error` | high | mitigate | Redact-then-truncate order preserved: the bounded body still passes through `redact_secret_patterns` before `bounded_excerpt`. | closed |
| T-27-22-04 | 27-22 | Information disclosure | credential header forwarded to a redirect target | high | mitigate | Unchanged and re-asserted: the client's no-redirect policy and its existing test stay in place; 3xx dead-letters. | closed |
| T-27-22-05 | 27-22 | Denial of service | retry budget consumed by a rescheduled attempt | medium | accept | `record_attempt` always increments; suppressing it needs a new port method across three adapters, out of scope for gap closure and recorded as a deliberate non-goal in the code.... | closed |
| T-27-23-01 | 27-23 | Denial of service | `LeaseHeartbeat::spawn` with a non-positive lease (WR-04) | medium | mitigate | A zero lease starts no task and logs the misuse; an exact-zero call-count test pins it. | closed |
| T-27-23-02 | 27-23 | Information disclosure | unscoped run/webhook-delivery reads (WR-03) | high | accept | Accepted for v0.10 as a single-tenant/trusted-principal deployment model, now stated explicitly in the controller docs, in the published reference, and as ledger row 32 with a n... | closed |
| T-27-23-03 | 27-23 | Repudiation | webhook silence on `Agent`-kind runs (WR-02) | medium | accept | Accepted and documented in two module doc blocks, in the published reference, and as ledger row 31; a test pins the current behaviour so the deferral cannot rot into an untracke... | closed |
| T-27-23-04 | 27-23 | Tampering | documentation drifting from behaviour | low | mitigate | Each documented limitation has a named test or a grep-checked doc assertion in this plan's acceptance criteria. | closed |
| T-27-24-01 | 27-24 | Tampering | a baseline regeneration absorbing a real API change | high | mitigate | The regeneration is gated on an empty diff between the normalised previous baseline and the new one, plus an equal `pub ` item count; a non-empty diff halts the task instead of ... | closed |
| T-27-24-02 | 27-24 | Tampering | an over-broad normaliser rewriting real signatures | medium | mitigate | The marker set is closed and explicit; the self-test asserts a plain API line passes through byte-identically. | closed |
| T-27-24-03 | 27-24 | Repudiation | a green CI job that selected zero tests | high | mitigate | The run step asserts a passing test-result line with a non-zero count, the workspace's documented trap. | closed |
| T-27-24-04 | 27-24 | Denial of service | added CI wall-clock time | low | accept | One extra cached job running a ~12s test; accepted against continuously proving the phase's flagship acceptance criterion. | closed |
| T-27-25-01 | 27-25 | Repudiation | evidence attributed to the wrong SHA or an older run | high | mitigate | The record carries the run id and the SHA, and the checkpoint requires the SHA to contain all six gap-closure commits. | closed |
| T-27-25-02 | 27-25 | Repudiation | a self-skipping suite recorded as a pass | high | mitigate | Each Tier-2 row names the log string that distinguishes a live run from a skip; a conclusion alone does not satisfy those rows. | closed |
| T-27-25-03 | 27-25 | Tampering | the bar being lowered after seeing results | medium | mitigate | The thresholds and log strings are written into the evidence file before the run, with empty result columns. | closed |
| T-27-25-04 | 27-25 | Information disclosure | CI logs quoted into the record carrying secrets | medium | mitigate | Only the named result lines and counts are quoted, never environment dumps or raw request/response bodies. | closed |

*Status: closed · open — all 107 rows are `closed` as of the 2026-09-08 re-audit*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (`high`) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Closed Threat — T-27-22-02 (was BLOCKING)

**Category:** Tampering / Repudiation · **Severity:** high · **Disposition:** mitigate ·
**Plan:** 27-22 · **Threshold:** at `block_on: high` · **Status: CLOSED 2026-09-08**

**What was open at the previous audit.** The WR-01 fix (`c2fe4afd`) rescheduled only the `Err`
arm of the signing-key lookup. The sibling `Ok(None)` arm — the run row supplying the
delivery's own signing secret is absent — still fell through to sign-and-send with an
empty-string key, shipping a payload a receiver must reject as mis-signed while charging one
of the delivery's five budgeted attempts, with no warning logged.

**Remediation verified.** Commit `4b6592de` (*"fix(27): reschedule webhook delivery when run
row is missing (WR-27-01)"*) mirrors the `Err` arm exactly:

```rust
// src/application/services/run/webhook/service.rs:188-221
Ok(None) => {
    log::warn!("webhook delivery service: run {} not found for delivery {delivery_id}; \
                rescheduling rather than sending with a fallback empty key", delivery.run_id);
    let delay = chrono::Duration::from_std(backoff_for(new_attempt))
        .unwrap_or_else(|_| chrono::Duration::zero());
    self.finish(&delivery_id, WebhookAttemptResult {
        outcome: WebhookAttemptOutcome::Retrying {
            next_attempt_at: (self.options.now)() + delay,
        },
        response_status: None,
        error: Some(bounded_error(&format!(
            "run {} not found while loading signing key", delivery.run_id))),
    }).await;
    return;   // <- returns before any request is issued
}
```

The `return` precedes `sign_webhook_body` (`service.rs:240-243`) and the send, so no signed
request can leave the process on this path.

**Evidence — the exact gaps the previous audit named, each now filled:**

| Previous finding | Resolution |
|---|---|
| `Ok(None)` falls through to sign-and-send | Arm returns after `finish(..., Retrying)`; the sign call is unreachable from it |
| No `log::warn!` on the arm | `log::warn!` naming the run id, the delivery id, and the reason |
| No diagnosable `last_error` | `bounded_error("run {id} not found while loading signing key")` persisted — redact-then-truncate order preserved (T-27-22-03 unaffected) |
| No test exercises the `Ok(None)` arm | `webhook_signing_key_missing_run_reschedules_without_sending` (`tests.rs:707`) drives a `MissingRunRepository` against a `mockito` target asserted with `expect(0)`, then asserts `Retrying`, `next_attempt_at > now`, `last_error.is_some()`, `last_response_status.is_none()` |

**Test run (this audit, 2026-09-08):**

```
cargo test --lib webhook_signing_key
test ...::webhook_signing_key_load_failure_reschedules_without_sending ... ok
test ...::webhook_signing_key_missing_run_reschedules_without_sending ... ok
test result: ok. 2 passed; 0 failed
```

Two tests selected, not zero — the workspace's documented 0-selecting-filter trap
(`27-24-01` / T-27-24-03) does not apply to this result.

**Related item, now unreachable:** `signature.rs:33` still returns a zero-key digest
(`hex_encode(&[0u8; 32])`) if HMAC key construction fails. Its own rustdoc records that
`Hmac<Sha256>` accepts a key of any length (RFC 2104), so the branch is unreachable in
practice; with both non-sending arms rescheduling, no caller can now reach the sign path
carrying an unusable key. Retained as a panic-free degradation, not a finding.

**Scope check.** Four other commits landed after the previous audit — `b5ee33d4` (removes a
stale `eslint-disable` directive in `scripts/sdk-smoke/smoke.ts`) and three `.planning/`
documentation commits (`80341635`, `5dd49a59`, `d4e4a2ca`). None touches security-relevant
Rust, so the remaining 106 closures carry forward unchanged.

---

## Accepted Risks Log

17 threats carry an `accept` or `transfer` disposition. All are below the `high` blocking
threshold except T-27-23-02, which was accepted at plan time with a named closing condition
and is traceable in the codebase and docs.

| Risk ID | Threat Ref | Severity | Rationale | Accepted By | Date |
|---------|------------|----------|-----------|-------------|------|
| AR-27-01 | T-27-23-02 | high | Unscoped run / webhook-delivery reads (WR-03). Single-tenant / trusted-principal posture for this milestone. Documented in `run_controller.rs` module docs (Read scope section), `docs/src/api-reference/platform-api.md:400-406`, and `.planning/WINDOWS.md` ledger row 32 with a named closing condition. | Phase 27 plan 27-23 | 2026-09-08 |
| AR-27-02 | T-27-02-05 | medium | `webhook.secret` persisted in the `runs` row — the secret must be available at send time (27-13); the row is readable only through the authenticated API, which projects `WebhookSpec` with `secret` redacted by a manual `Debug` (`run.rs:314-325`). No key store exists in this milestone (ADR-0041 scope). | Phase 27 plan 27-02 | 2026-09-08 |
| AR-27-03 | T-27-07-04 | medium | *(transfer)* Cancelling another caller's run — the cancel route sits behind the same `require_authentication` layer (`run_controller.rs:1008`, test `:1969`); per-principal ownership is the same single-tenant posture as AR-27-01. | Phase 27 plan 27-07 | 2026-09-08 |
| AR-27-04 | T-27-10-04 | medium | *(transfer)* Streaming another principal's run — same `require_authentication` layer covers `GET /runs/{id}/stream`; same posture as AR-27-01. | Phase 27 plan 27-10 | 2026-09-08 |
| AR-27-05 | T-27-15-05 | medium | Cross-principal run / thread access — same single-tenant posture, documented `docs/src/api-reference/platform-api.md:400`. | Phase 27 plan 27-15 | 2026-09-08 |
| AR-27-06 | T-27-20-04 | medium | A weakened assertion masking a broken adapter — mitigated in practice by exact `assert_eq!` on timestamps in the shared contract suite (`contract_tests.rs:108,146,185`). | Phase 27 plan 27-20 | 2026-09-08 |
| AR-27-07 | T-27-22-05 | medium | Retry budget consumed by a rescheduled attempt — suppressing the increment needs a new `WebhookDeliveryRepositoryPort` method threaded across three adapters; recorded as a deliberate non-goal at `service.rs:214-219`. | Phase 27 plan 27-22 | 2026-09-08 |
| AR-27-08 | T-27-23-03 | medium | Webhook silence on `Agent`-kind runs (WR-02) — documented in `worker_tests.rs:865-939`, `platform-api.md:406`, and `.planning/WINDOWS.md` row 31. | Phase 27 plan 27-23 | 2026-09-08 |
| AR-27-09 | T-27-05 | low | Run identity / per-run actor attribution — `RunId` is a UUIDv7 (`run.rs:75`); attribution is a Phase 28 observability concern (OBS-01). | Phase 27 plan 27-01 | 2026-09-08 |
| AR-27-10 | T-27-03-04 | low | Unbounded queue depth — bounded by the one-active-run-per-thread invariant (D-17) and by API auth + rate limiting (27-15). | Phase 27 plan 27-03 | 2026-09-08 |
| AR-27-11 | T-27-04-04 | low | `EngineError` text stored in `Run.error` — engine error text scoping documented at plan time. | Phase 27 plan 27-04 | 2026-09-08 |
| AR-27-12 | T-27-05-04 | low | `system_prompt` in stored documents — admin-authored content. | Phase 27 plan 27-05 | 2026-09-08 |
| AR-27-13 | T-27-08-04 | low | `run_id` in the response — UUIDv7 handle, non-guessable and non-sensitive. | Phase 27 plan 27-08 | 2026-09-08 |
| AR-27-14 | T-27-09-04 | low | Definition bodies (prompts) — admin-authored content. | Phase 27 plan 27-09 | 2026-09-08 |
| AR-27-15 | T-27-11-04 | low | Schedule `input` / `webhook.secret` rows — same posture as AR-27-02. | Phase 27 plan 27-11 | 2026-09-08 |
| AR-27-16 | T-27-14-04 | low | Absurd cron frequency — bounded by the busy-thread interplay (cross-ref T-27-11-02). | Phase 27 plan 27-14 | 2026-09-08 |
| AR-27-17 | T-27-24-04 | low | Added CI wall-clock time (~12s) — documented tradeoff. | Phase 27 plan 27-24 | 2026-09-08 |

---

## Verified Mitigation Highlights

The three `critical` threats and the credential-handling surface named in
`.github/instructions/security.instructions.md` were verified at L2 depth:

| Threat | Evidence |
|---|---|
| T-27-13-01 — SSRF via webhook URL | `SsrfGuard::check_url` at **write time** (`submission.rs:203` `submit`, `:328` `fork`) **and send time** (`webhook/service.rs:172` `process`); no-redirect client (`client.rs:86`); `169.254.169.254` always rejected regardless of `allow_private` (`ssrf.rs:203-205`) |
| T-27-14-02 — SSRF through a schedule webhook | Write-time `check_url` in `schedule/admin.rs:135` (create) and `:255` (patch); ticks re-enter `submission.rs:203` and are delivered by the same send-time-guarded service |
| T-27-22-01 — unbounded response-body read | `read_bounded_body` caps during streaming via a `chunk()` loop and never trusts `Content-Length` (`client.rs:47-70`); `MAX_ERROR_BODY_BYTES = 64 * 1024`; tests `bounded_body_stops_at_the_cap`, `..._at_exactly_the_cap_is_not_truncated` |
| T-27-13-04 / T-27-22-04 — credential header on redirect | `reqwest::redirect::Policy::none()` (`client.rs:86`); `webhook_client_no_redirects` proves the redirect target mock receives 0 hits |
| T-27-22-03 — credential in a receiver's error page | `bounded_error()` calls `redact_secret_patterns` **then** `bounded_excerpt` (`service.rs:389-391`) — redact-before-truncate order preserved |
| T-27-13-02 — signature over re-serialised bytes | `WebhookDelivery.payload: String` stored verbatim (`webhook.rs:189`) and signed from that exact buffer (`service.rs:240-243`) |
| T-27-02-03 — SQL injection | Zero hits for `format!("SELECT` / `INSERT` / `UPDATE` across `crates` and `src`; parameterized `sqlx::query` throughout |
| T-27-02-04 / T-27-03-03 — credentials in errors | `redact_database_url_password` at 27 call sites; `redact_connection_url` in `redis.rs:299,310,384` |

**Known limitation carried forward (documented, not a finding):** DNS rebinding — neither the
write-time nor the send-time SSRF check pins the resolved address between check and connect.
This is recorded in `ssrf.rs`'s own module docs and in `security.instructions.md`.

---

## Threat Flags from Summaries

None. No `## Threat Flags` section appears in any of the 26 `27-*-SUMMARY.md` files — no
executor flagged new attack surface through the designated channel this phase. Spot-checks of
router composition (`run_openapi_router`, `thread_controller::fork_thread`) found the
`require_authentication` convention applied consistently.

---

## Security Audit Trail

## Security Audit 2026-09-08 (re-audit after WR-27-01)

| Metric | Count |
|--------|-------|
| Threats found | 107 |
| Closed | 107 |
| Open | 0 |
| Open at or above `high` | 0 |

Verified T-27-22-02's remediation directly (code path, covering test, green run) rather than
re-deriving it. `register_authored_at_plan_time: true` and `asvs_level: 1`, so with
`threats_open: 0` the workflow's L1 short-circuit applies and no deeper re-verification of the
106 already-closed threats was required.

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-08 | 107 | 106 | 1 | gsd-security-auditor (ASVS L1, block_on high) |
| 2026-09-08 | 107 | 107 | 0 | /gsd-secure-phase re-audit (ASVS L1, block_on high) |

Severity breakdown: 3 critical (3 closed) · 41 high (41 closed) · 47 medium (47 closed) · 16 low (16 closed).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed — T-27-22-02 closed by `4b6592de`, re-verified 2026-09-08
- [x] `status: verified` set in frontmatter

**Approval:** granted — no blocking threats remain.
