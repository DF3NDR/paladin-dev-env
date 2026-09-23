---
status: complete
phase: 27-platform-api
source: [27-01-SUMMARY.md, 27-02-SUMMARY.md, 27-03-SUMMARY.md, 27-04-SUMMARY.md, 27-05-SUMMARY.md, 27-06-SUMMARY.md, 27-07-SUMMARY.md, 27-08-SUMMARY.md, 27-09-SUMMARY.md, 27-10-SUMMARY.md, 27-11-SUMMARY.md, 27-12-SUMMARY.md, 27-13-SUMMARY.md, 27-14-SUMMARY.md, 27-15-SUMMARY.md, 27-16-SUMMARY.md, 27-17-SUMMARY.md, 27-18-SUMMARY.md, 27-19-SUMMARY.md, 27-20-SUMMARY.md, 27-21-SUMMARY.md, 27-22-SUMMARY.md, 27-23-SUMMARY.md, 27-24-SUMMARY.md, 27-25-SUMMARY.md, 27-26-SUMMARY.md]
started: 2026-09-08T18:35:35Z
updated: 2026-09-08T19:10:16Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: |
  Kill any running paladin-server. Clear ephemeral state (temp SQLite DBs, caches, lock files).
  Boot the server from scratch on the all-InMemory/SQLite profile. Migrations
  003_create_assistants_tables, 004_create_run_schedules_table and
  005_create_webhook_deliveries_table apply without error, the server starts clean, and a
  primary query (GET /v1/assistants, or submit a run then poll status) returns live data.
result: pass

### 2. Redis run-queue suite proves the live server in CI (27-03 D5)
expected: |
  The redis-queue CI job runs the run_queue::redis suite against a real Redis server and does
  NOT take the self-skip path: the log shows "All run_queue::redis tests exercised the live
  server", a declared-vs-selected count that matches, and 0 failed. A local run in this
  devcontainer prints SKIP: lines and is NOT evidence (D-51).
result: pass

### 3. sdk-clients CI job generates both clients and smokes them live (27-18 D2)
expected: |
  The sdk-clients CI job generates a Python and a TypeScript client from
  crates/paladin-web/openapi.json with the pinned openapi-generator-cli, both trees are
  non-empty (>=20 files), paladin-server boots on the InMemory/SQLite profile, and each client
  completes list assistants -> submit run -> poll status. This cannot run locally (no Java/Docker).
result: pass

### 4. Redis live-server Tier-2 clauses proven only by CI (27-19 D4)
expected: |
  fifo, lease-expiry, extend, ack, nack, expired/unknown token, depth, namespacing and
  concurrency are all proven green by the redis-queue CI job — not by a local run, which
  self-skips. Confirm the CI job actually fired and passed at a SHA containing this work.
result: pass

### 5. Postgres run repository round-trips timestamps exactly (27-20 D2)
expected: |
  In the postgres-integration CI job, insert-then-get through PostgresRunRepository round-trips
  submitted_at / started_at / finished_at under exact assert_eq!, with fixtures stamped at
  storage resolution by contract_timestamp. The three previously-failing contract_tests.rs
  clauses pass. Local runs self-skip and are not evidence.
result: pass

### 6. Postgres timestamps truncate rather than round (27-20 D3)
expected: |
  postgres_run_timestamps_round_trip_at_microsecond_precision passes in the postgres-integration
  CI job: a sub-microsecond submitted_at/started_at round-trips equal to storage_timestamp(original)
  and NOT equal to the original value — proving truncation toward zero, not rounding.
result: pass

### 7. Both SDK smokes demand terminal status exactly 'completed' (27-21 D3)
expected: |
  smoke.py and smoke.ts each route the terminal-status decision through an extracted,
  self-testable function that accepts only 'completed'. Any other terminal status, or an
  unreached deadline, fails loudly printing the observed status and the run's own error text.
  smoke.ts's real-client field/method names are proven only by the live sdk-clients CI job.
result: pass

### 8. Human approval of the CI evidence record (27-25 D8)
expected: |
  27-CI-EVIDENCE.md's Tier-2 table was filled in only AFTER a live CI run, from quoted per-job
  log lines, with the required-proof column written before the run so the bar could not be
  lowered. A human reviewed the quoted evidence and explicitly approved it — not auto-passed.
result: pass

### 9. Full Tier-2 CI sweep green at the phase SHA (27-26 D4)
expected: |
  The redis-queue, coverage and integration-tests CI jobs are all green at a SHA containing the
  complete phase-27 work, including the 17 run_queue::redis tests (with the fixed run_all clause).
  Docker is unavailable locally, so every local run of these took the SKIP path and is not evidence.
result: pass

### 10. [27-01 D1] RunStatus::try_transition implements D-02's exact edge table, exhaustively tested over the full 7x7 status cross-product plus self-transitions and terminal-absorption, with IllegalTransition{from,to} on every illegal pair
expected: RunStatus::try_transition implements D-02's exact edge table, exhaustively tested over the full 7x7 status cross-product plus self-transitions and terminal-absorption, with IllegalTransition{from,to} on every illegal pair
result: pass
source: automated
coverage_id: 27-01/D1

### 11. [27-01 D2] POST /v1/runs accepts an empty/omitted input and returns 202; a run submitted over HTTP is durably recorded, enqueued, dequeued by a worker, driven through the real WarEngine, and reaches Completed with GET /v1/runs/{run_id} reporting each status
expected: POST /v1/runs accepts an empty/omitted input and returns 202; a run submitted over HTTP is durably recorded, enqueued, dequeued by a worker, driven through the real WarEngine, and reaches Completed with GET /v1/runs/{run_id} reporting each status
result: pass
source: automated
coverage_id: 27-01/D2

### 12. [27-01 D3] GET /v1/runs/{unknown} returns 404 not_found; POST /v1/runs on an unwired RunApiState returns 501 not_implemented naming the config key; a run whose graph fails ends Failed with the engine's error recorded, not a panic
expected: GET /v1/runs/{unknown} returns 404 not_found; POST /v1/runs on an unwired RunApiState returns 501 not_implemented naming the config key; a run whose graph fails ends Failed with the engine's error recorded, not a panic
result: pass
source: automated
coverage_id: 27-01/D3

### 13. [27-01 D4] POST /runs performs exactly one repository insert and one queue enqueue and names no engine type; paladin-web's run_controller.rs names no WarEngine/WarGraph/RunQueuePort and paladin-web carries no default-build edge to paladin-battalion
expected: POST /runs performs exactly one repository insert and one queue enqueue and names no engine type; paladin-web's run_controller.rs names no WarEngine/WarGraph/RunQueuePort and paladin-web carries no default-build edge to paladin-battalion
result: pass
source: automated
coverage_id: 27-01/D4

### 14. [27-01 D5] InMemoryRunQueue's lease semantics are real, not stubbed: a dequeued message stays hidden for the lease duration, an expired lease becomes visible again, and ack/nack/extend_lease/depth all behave correctly -- so 27-03's contract suite runs against this adapter unchanged
expected: InMemoryRunQueue's lease semantics are real, not stubbed: a dequeued message stays hidden for the lease duration, an expired lease becomes visible again, and ack/nack/extend_lease/depth all behave correctly -- so 27-03's contract suite runs against this adapter unchanged
result: pass
source: automated
coverage_id: 27-01/D5

### 15. [27-01 D6] InMemoryRunRepository routes every status write through RunStatus::try_transition and rejects insert with ThreadBusy under its write lock whenever the target thread already has an active run (T-27-01 / D-17-D-18 in-memory twin)
expected: InMemoryRunRepository routes every status write through RunStatus::try_transition and rejects insert with ThreadBusy under its write lock whenever the target thread already has an active run (T-27-01 / D-17-D-18 in-memory twin)
result: pass
source: automated
coverage_id: 27-01/D6

### 16. [27-02 D1] runs SQL table exists on both backends with the idx_runs_thread_active partial unique index (the D-17 409 ThreadBusy invariant, busy set queued|running|awaiting_input per D-18) and idx_runs_submitted for list ordering; no status-history table (D-05)
expected: runs SQL table exists on both backends with the idx_runs_thread_active partial unique index (the D-17 409 ThreadBusy invariant, busy set queued|running|awaiting_input per D-18) and idx_runs_submitted for list ordering; no status-history table (D-05)
result: pass
source: automated
coverage_id: 27-02/D1

### 17. [27-02 D2] One shared RunRepositoryPort contract suite (14 pub async fn clauses) covering round-trip, CAS transitions (D-04) including self-transition and terminal-absorption, thread-busy across all three active statuses then success after terminal (D-17/D-18), list pagination/filters, cancellation idempotence, attempt/resume bookkeeping, outcome recording, schema-version guard (X-04), and the ten-concurrent-inserts stress test (D-52)
expected: One shared RunRepositoryPort contract suite (14 pub async fn clauses) covering round-trip, CAS transitions (D-04) including self-transition and terminal-absorption, thread-busy across all three active statuses then success after terminal (D-17/D-18), list pagination/filters, cancellation idempotence, attempt/resume bookkeeping, outcome recording, schema-version guard (X-04), and the ten-concurrent-inserts stress test (D-52)
result: pass
source: automated
coverage_id: 27-02/D2

### 18. [27-02 D3] InMemoryRunRepository, SqliteRunRepository and PostgresRunRepository all pass the identical contract suite unchanged -- InMemory 21 tests (14 contract + 7 pre-existing), SQLite 15 tests, Postgres 14 tests (self-skipping locally, never recorded as passed)
expected: InMemoryRunRepository, SqliteRunRepository and PostgresRunRepository all pass the identical contract suite unchanged -- InMemory 21 tests (14 contract + 7 pre-existing), SQLite 15 tests, Postgres 14 tests (self-skipping locally, never recorded as passed)
result: pass
source: automated
coverage_id: 27-02/D3

### 19. [27-02 D4] The 409 ThreadBusy invariant is a database property: ten concurrent insert calls for one thread against SqliteRunRepository over a real on-disk WAL file (not sqlite::memory:) yield exactly one Ok and nine ThreadBusy, proving the partial unique index -- not an in-process lock -- enforces the invariant
expected: The 409 ThreadBusy invariant is a database property: ten concurrent insert calls for one thread against SqliteRunRepository over a real on-disk WAL file (not sqlite::memory:) yield exactly one Ok and nine ThreadBusy, proving the partial unique index -- not an in-process lock -- enforces the invariant
result: pass
source: automated
coverage_id: 27-02/D4

### 20. [27-02 D5] SQL construction is 100% parameterized: every fixed-shape query is a plain &'static str constant, list()'s dynamic filters use sqlx::QueryBuilder::push_bind, and no format!(\"SELECT|UPDATE|INSERT|DELETE ...\") call exists in either adapter
expected: SQL construction is 100% parameterized: every fixed-shape query is a plain &'static str constant, list()'s dynamic filters use sqlx::QueryBuilder::push_bind, and no format!(\"SELECT|UPDATE|INSERT|DELETE ...\") call exists in either adapter
result: pass
source: automated
coverage_id: 27-02/D5

### 21. [27-02 D6] Connection-string passwords are redacted from every Backend error on both adapters, reusing waypoint::redact::redact_database_url_password rather than a second helper (T-27-02-04 mitigation)
expected: Connection-string passwords are redacted from every Backend error on both adapters, reusing waypoint::redact::redact_database_url_password rather than a second helper (T-27-02-04 mitigation)
result: pass
source: automated
coverage_id: 27-02/D6

### 22. [27-03 D1] A message dequeued with lease L is invisible to every other dequeue until L elapses, then becomes visible again with the same run_id and attempt one higher -- on both InMemoryRunQueue and RedisRunQueue
expected: A message dequeued with lease L is invisible to every other dequeue until L elapses, then becomes visible again with the same run_id and attempt one higher -- on both InMemoryRunQueue and RedisRunQueue
result: pass
source: automated
coverage_id: 27-03/D1

### 23. [27-03 D2] extend_lease pushes expiry out by exactly the requested duration from the call time; ack removes the message permanently; nack(delay) re-queues it visible only after delay, with attempt incremented; extend_lease/ack/nack on an expired or unknown token return QueueError::LeaseExpired/UnknownLease and never touch another message
expected: extend_lease pushes expiry out by exactly the requested duration from the call time; ack removes the message permanently; nack(delay) re-queues it visible only after delay, with attempt incremented; extend_lease/ack/nack on an expired or unknown token return QueueError::LeaseExpired/UnknownLease and never touch another message
result: pass
source: automated
coverage_id: 27-03/D2

### 24. [27-03 D3] depth() counts ready plus leased messages and returns to 0 after every message is acked
expected: depth() counts ready plus leased messages and returns to 0 after every message is acked
result: pass
source: automated
coverage_id: 27-03/D3

### 25. [27-03 D4] The Redis adapter's claim-and-expire is a single atomic EVAL over a sorted set scored by visibility time; redis::Script handles NOSCRIPT reload transparently
expected: The Redis adapter's claim-and-expire is a single atomic EVAL over a sorted set scored by visibility time; redis::Script handles NOSCRIPT reload transparently
result: pass
source: automated
coverage_id: 27-03/D4

### 26. [27-04 D1] A worker extends its lease every lease/4 while a run executes and stops the moment the run returns (D-10)
expected: A worker extends its lease every lease/4 while a run executes and stops the moment the run returns (D-10)
result: pass
source: automated
coverage_id: 27-04/D1

### 27. [27-04 D2] A run redelivered after its worker died is RESUMED from the thread's latest Waypoint (never restarted): a second worker completes it and no node executes beyond the interrupted superstep (PLAT-FR-03, D-09)
expected: A run redelivered after its worker died is RESUMED from the thread's latest Waypoint (never restarted): a second worker completes it and no node executes beyond the interrupted superstep (PLAT-FR-03, D-09)
result: pass
source: automated
coverage_id: 27-04/D2

### 28. [27-04 D3] The worker has one entry point that branches on the thread's latest Waypoint: absent -> start; present with pending responses -> resume_with; present otherwise -> resume (D-09)
expected: The worker has one entry point that branches on the thread's latest Waypoint: absent -> start; present with pending responses -> resume_with; present otherwise -> resume (D-09)
result: pass
source: automated
coverage_id: 27-04/D3

### 29. [27-04 D4] RunOutcome::AwaitingInput releases the worker by ACKing the queue message and recording AwaitingInput; queue depth returns to 0 while the run sits suspended (D-22)
expected: RunOutcome::AwaitingInput releases the worker by ACKing the queue message and recording AwaitingInput; queue depth returns to 0 while the run sits suspended (D-22)
result: pass
source: automated
coverage_id: 27-04/D4

### 30. [27-04 D5] Redelivery and resume share one attempt counter on the run row (D-23)
expected: Redelivery and resume share one attempt counter on the run row (D-23)
result: pass
source: automated
coverage_id: 27-04/D5

### 31. [27-04 D6] On shutdown a worker stops dequeuing, lets its in-flight run reach a superstep boundary, leaves the run Running with its message NACKed for immediate redelivery, and exits -- draining, not dropping (D-13)
expected: On shutdown a worker stops dequeuing, lets its in-flight run reach a superstep boundary, leaves the run Running with its message NACKed for immediate redelivery, and exits -- draining, not dropping (D-13)
result: pass
source: automated
coverage_id: 27-04/D6

### 32. [27-05 D1] WarGraphDoc + sub-documents (NodeDoc/NodeKindDoc, PaladinNodeDoc, GateNodeDoc, WorkflowNodeDoc, EdgeDoc/EdgeConditionDoc, AegisDoc family, SchemaDoc/FieldDoc, LimitsDoc) with schemars(JsonSchema) + serde(deny_unknown_fields) derives
expected: WarGraphDoc + sub-documents (NodeDoc/NodeKindDoc, PaladinNodeDoc, GateNodeDoc, WorkflowNodeDoc, EdgeDoc/EdgeConditionDoc, AegisDoc family, SchemaDoc/FieldDoc, LimitsDoc) with schemars(JsonSchema) + serde(deny_unknown_fields) derives
result: pass
source: automated
coverage_id: 27-05/D1

### 33. [27-05 D2] WarGraphDoc::compile(&EngineRegistries) resolves every named reference (edge evaluator, retry predicate, error handler, output schema) and delegates structural checks to WarGraph::validate — compile is validation
expected: WarGraphDoc::compile(&EngineRegistries) resolves every named reference (edge evaluator, retry predicate, error handler, output schema) and delegates structural checks to WarGraph::validate — compile is validation
result: pass
source: automated
coverage_id: 27-05/D2

### 34. [27-05 D3] Every unresolved name and structural fault is a typed, single-offender CompileError variant naming node/edge/name — never a silent drop
expected: Every unresolved name and structural fault is a typed, single-offender CompileError variant naming node/edge/name — never a silent drop
result: pass
source: automated
coverage_id: 27-05/D3

### 35. [27-05 D4] Node kinds are exactly paladin/gate/workflow; any other kind (e.g. function) fails compile with a typed CompileError::UnsupportedNodeKind naming the rejected string, documented as a v0.10 limitation
expected: Node kinds are exactly paladin/gate/workflow; any other kind (e.g. function) fails compile with a typed CompileError::UnsupportedNodeKind naming the rejected string, documented as a v0.10 limitation
result: pass
source: automated
coverage_id: 27-05/D4

### 36. [27-05 D5] Nested workflow node kind compiles recursively to NodeSpec::Battalion, bounded by MAX_NESTING_DEPTH=8
expected: Nested workflow node kind compiles recursively to NodeSpec::Battalion, bounded by MAX_NESTING_DEPTH=8
result: pass
source: automated
coverage_id: 27-05/D5

### 37. [27-05 D6] GraphFingerprint round-trips byte-identically across a real OS process boundary (D-35)
expected: GraphFingerprint round-trips byte-identically across a real OS process boundary (D-35)
result: pass
source: automated
coverage_id: 27-05/D6

### 38. [27-05 D7] schemars-derived JSON Schema is golden-guarded byte-for-byte against docs/schemas/wargraph-doc.schema.json, with UPDATE_WARGRAPH_SCHEMA=1 as the bless command
expected: schemars-derived JSON Schema is golden-guarded byte-for-byte against docs/schemas/wargraph-doc.schema.json, with UPDATE_WARGRAPH_SCHEMA=1 as the bless command
result: pass
source: automated
coverage_id: 27-05/D7

### 39. [27-05 D8] Every fixture under tests/fixtures/graph_docs/ deserialises, compiles, re-serialises to the same JSON Value, and compiles again to the same fingerprint
expected: Every fixture under tests/fixtures/graph_docs/ deserialises, compiles, re-serialises to the same JSON Value, and compiles again to the same fingerprint
result: pass
source: automated
coverage_id: 27-05/D8

### 40. [27-05 D9] mdBook page documents the document format, the v0.10 node-kind boundary, registry-name resolution, schema_version, and the fingerprint-stability guarantee; linked from docs/src/SUMMARY.md; mdbook build succeeds
expected: mdBook page documents the document format, the v0.10 node-kind boundary, registry-name resolution, schema_version, and the fingerprint-stability guarantee; linked from docs/src/SUMMARY.md; mdbook build succeeds
result: pass
source: automated
coverage_id: 27-05/D9

### 41. [27-06 D1] RunStoreConfig/RunStoreBackend default Disabled; validate() rejects empty sqlite path and unresolvable postgres url_env; env overrides via APP_RUN_STORE_*
expected: RunStoreConfig/RunStoreBackend default Disabled; validate() rejects empty sqlite path and unresolvable postgres url_env; env overrides via APP_RUN_STORE_*
result: pass
source: automated
coverage_id: 27-06/D1

### 42. [27-06 D2] RunQueueConfig/RunQueueBackend default InMemory; Redis variant with url_env + key_prefix (default paladin:run_queue); validate() rejects unresolvable url_env
expected: RunQueueConfig/RunQueueBackend default InMemory; Redis variant with url_env + key_prefix (default paladin:run_queue); validate() rejects unresolvable url_env
result: pass
source: automated
coverage_id: 27-06/D2

### 43. [27-06 D3] RunWorkerConfig{concurrency:4, lease_seconds:60, min_probe_interval_ms:1000}, no heartbeat field, validate() rejects zero concurrency / sub-4s lease / zero probe interval
expected: RunWorkerConfig{concurrency:4, lease_seconds:60, min_probe_interval_ms:1000}, no heartbeat field, validate() rejects zero concurrency / sub-4s lease / zero probe interval
result: pass
source: automated
coverage_id: 27-06/D3

### 44. [27-06 D4] RunStreamConfig{poll_interval_ms:1000} for the D-26 degraded SSE polling path; validate() rejects 0
expected: RunStreamConfig{poll_interval_ms:1000} for the D-26 degraded SSE polling path; validate() rejects 0
result: pass
source: automated
coverage_id: 27-06/D4

### 45. [27-06 D5] AssistantsConfig{expose_code_registry:true} (D-32, the deliberate ON default)
expected: AssistantsConfig{expose_code_registry:true} (D-32, the deliberate ON default)
result: pass
source: automated
coverage_id: 27-06/D5

### 46. [27-06 D6] SchedulesConfig{enabled:false, tick_interval_ms:1000}; validate() rejects zero tick interval
expected: SchedulesConfig{enabled:false, tick_interval_ms:1000}; validate() rejects zero tick interval
result: pass
source: automated
coverage_id: 27-06/D6

### 47. [27-06 D7] WebhooksConfig{allow_private:false, max_attempts:5, timeout_secs:10}; rustdoc names the SSRF guard's rejected target classes, write/send-time application, no-redirects policy, and the DNS-rebinding limitation
expected: WebhooksConfig{allow_private:false, max_attempts:5, timeout_secs:10}; rustdoc names the SSRF guard's rejected target classes, write/send-time application, no-redirects policy, and the DNS-rebinding limitation
result: pass
source: automated
coverage_id: 27-06/D7

### 48. [27-07 D1] WarEngine::with_cancellation_probe is an added builder method; the engine calls probe.is_cancelled(&thread) once per superstep boundary beside -- not instead of -- the existing CancellationToken check, and a true answer produces the same Halted Waypoint path
expected: WarEngine::with_cancellation_probe is an added builder method; the engine calls probe.is_cancelled(&thread) once per superstep boundary beside -- not instead of -- the existing CancellationToken check, and a true answer produces the same Halted Waypoint path
result: pass
source: automated
coverage_id: 27-07/D1

### 49. [27-07 D2] CancellationProbe::is_cancelled is infallible by signature; the DB-backed adapter (DbCancellationProbe) logs and returns false on any repository error, so a probe failure can never fail a run
expected: CancellationProbe::is_cancelled is infallible by signature; the DB-backed adapter (DbCancellationProbe) logs and returns false on any repository error, so a probe failure can never fail a run
result: pass
source: automated
coverage_id: 27-07/D2

### 50. [27-07 D3] The facade adapter debounces: two boundary checks inside min_probe_interval cause one repository read
expected: The facade adapter debounces: two boundary checks inside min_probe_interval cause one repository read
result: pass
source: automated
coverage_id: 27-07/D3

### 51. [27-07 D4] RunSubmissionPort::cancel(run_id) persists cancel_requested FIRST, then best-effort cancels the in-process token if the run is local; it is idempotent on a non-terminal run and returns AlreadyTerminal on a terminal one
expected: RunSubmissionPort::cancel(run_id) persists cancel_requested FIRST, then best-effort cancels the in-process token if the run is local; it is idempotent on a non-terminal run and returns AlreadyTerminal on a terminal one
result: pass
source: automated
coverage_id: 27-07/D4

### 52. [27-07 D5] A run executing on instance A is cancelled by a flag written through instance B's repository handle; A halts at the next superstep boundary with a Halted Waypoint and the run is recorded Cancelled
expected: A run executing on instance A is cancelled by a flag written through instance B's repository handle; A halts at the next superstep boundary with a Halted Waypoint and the run is recorded Cancelled
result: pass
source: automated
coverage_id: 27-07/D5

### 53. [27-08 D1] POST /threads/{id}/resume keeps Phase 24's published 202/{thread_id, state_url} contract, 404/409/400/501 status table verbatim, with only the ParleyPort mechanism changed from spawn to enqueue (D-20)
expected: POST /threads/{id}/resume keeps Phase 24's published 202/{thread_id, state_url} contract, 404/409/400/501 status table verbatim, with only the ParleyPort mechanism changed from spawn to enqueue (D-20)
result: pass
source: automated
coverage_id: 27-08/D1

### 54. [27-08 D2] A complete parley submission against a thread with an active run row is validated synchronously then re-enqueued under the SAME run_id with attempt incremented, instead of spawned in-process; the queue message carries only the pointer, responses ride on the run row
expected: A complete parley submission against a thread with an active run row is validated synchronously then re-enqueued under the SAME run_id with attempt incremented, instead of spawned in-process; the queue message carries only the pointer, responses ride on the run row
result: pass
source: automated
coverage_id: 27-08/D2

### 55. [27-08 D3] A thread with no run row falls back to Phase 24's in-process spawn unchanged, run_id: None (X-03); record_resume's IllegalTransition maps to ThreadNotAwaitingInput
expected: A thread with no run row falls back to Phase 24's in-process spawn unchanged, run_id: None (X-03); record_resume's IllegalTransition maps to ThreadNotAwaitingInput
result: pass
source: automated
coverage_id: 27-08/D3

### 56. [27-08 D4] ResumeAccepted and ResumeAcceptedResponse gain run_id, are #[non_exhaustive] with construction paths preserved, and are registered in MIGRATION.md §9.2/§9.6
expected: ResumeAccepted and ResumeAcceptedResponse gain run_id, are #[non_exhaustive] with construction paths preserved, and are registered in MIGRATION.md §9.2/§9.6
result: pass
source: automated
coverage_id: 27-08/D4

### 57. [27-09 D1] AssistantDefinition is a tagged envelope over opaque JSON in paladin-core (kind: Agent|Workflow, body: serde_json::Value) -- grep-verified zero mentions of paladin-battalion/paladin_battalion anywhere in assistant.rs, including comments
expected: AssistantDefinition is a tagged envelope over opaque JSON in paladin-core (kind: Agent|Workflow, body: serde_json::Value) -- grep-verified zero mentions of paladin-battalion/paladin_battalion anywhere in assistant.rs, including comments
result: pass
source: automated
coverage_id: 27-09/D1

### 58. [27-09 D2] AssistantRepositoryPort has exactly create/append_version/get/get_version/list/list_versions/soft_delete -- no update method exists anywhere in the file (grep-verified), immutability enforced by the PRIMARY KEY (assistant_id, version) on assistant_versions, not by handler discipline
expected: AssistantRepositoryPort has exactly create/append_version/get/get_version/list/list_versions/soft_delete -- no update method exists anywhere in the file (grep-verified), immutability enforced by the PRIMARY KEY (assistant_id, version) on assistant_versions, not by handler discipline
result: pass
source: automated
coverage_id: 27-09/D2

### 59. [27-09 D3] InMemoryAssistantRepository, SqliteAssistantRepository and PostgresAssistantRepository all pass the identical 10-clause contract suite, including the concurrent_append_admits_exactly_one_per_version stress test (ten concurrent append_version calls -> versions 2..=11, no gaps, no duplicates)
expected: InMemoryAssistantRepository, SqliteAssistantRepository and PostgresAssistantRepository all pass the identical 10-clause contract suite, including the concurrent_append_admits_exactly_one_per_version stress test (ten concurrent append_version calls -> versions 2..=11, no gaps, no duplicates)
result: pass
source: automated
coverage_id: 27-09/D3

### 60. [27-09 D4] RunRepositoryPort::insert_with_latest resolves and freezes assistant_version from assistants.latest inside ONE insert statement on both SQL backends; a concurrent version publish is observed strictly before or strictly after, proven by assistant_version_freeze_at_submit (20 alternating append_version/insert_with_latest calls, no run resolves version 0 or a version above the final latest)
expected: RunRepositoryPort::insert_with_latest resolves and freezes assistant_version from assistants.latest inside ONE insert statement on both SQL backends; a concurrent version publish is observed strictly before or strictly after, proven by assistant_version_freeze_at_submit (20 alternating append_version/insert_with_latest calls, no run resolves version 0 or a version above the final latest)
result: pass
source: automated
coverage_id: 27-09/D4

### 61. [27-09 D5] Every version records created_at/created_by/note; a soft-deleted assistant's versions stay readable by get_version so historical runs remain reconstructable (PLAT-FR-10)
expected: Every version records created_at/created_by/note; a soft-deleted assistant's versions stay readable by get_version so historical runs remain reconstructable (PLAT-FR-10)
result: pass
source: automated
coverage_id: 27-09/D5

### 62. [27-10 D1] GET /v1/runs/{run_id}/stream is a text/event-stream response whose event: lines are exactly the seven frozen wire names, each documented with its payload schema in the OpenAPI operation
expected: GET /v1/runs/{run_id}/stream is a text/event-stream response whose event: lines are exactly the seven frozen wire names, each documented with its payload schema in the OpenAPI operation
result: pass
source: automated
coverage_id: 27-10/D1

### 63. [27-10 D2] superstep/node_started/node_finished/state_delta bridge live from a TraceSink adapter feeding the per-run bus; parley/done/error are published by the worker from RunOutcome; unmapped TraceEvent variants are dropped
expected: superstep/node_started/node_finished/state_delta bridge live from a TraceSink adapter feeding the per-run bus; parley/done/error are published by the worker from RunOutcome; unmapped TraceEvent variants are dropped
result: pass
source: automated
coverage_id: 27-10/D2

### 64. [27-10 D3] The bus never blocks the engine: a full per-run channel drops the oldest events and counts them, exposed as dropped on the next event
expected: The bus never blocks the engine: a full per-run channel drops the oldest events and counts them, exposed as dropped on the next event
result: pass
source: automated
coverage_id: 27-10/D3

### 65. [27-10 D4] If the run executes elsewhere or is already terminal, the handler synthesizes events by polling WaypointPort::latest + RunRepositoryPort::get and always terminates with done/error, every event carrying mode: degraded
expected: If the run executes elsewhere or is already terminal, the handler synthesizes events by polling WaypointPort::latest + RunRepositoryPort::get and always terminates with done/error, every event carrying mode: degraded
result: pass
source: automated
coverage_id: 27-10/D4

### 66. [27-10 D5] Heartbeat comment lines are emitted every 15s via Sse::keep_alive(KeepAlive::new().interval(15s)) on both paths
expected: Heartbeat comment lines are emitted every 15s via Sse::keep_alive(KeepAlive::new().interval(15s)) on both paths
result: pass
source: automated
coverage_id: 27-10/D5

### 67. [27-10 D6] state_delta payloads never carry field values, vault-confined values or full Battlefield state -- only changed field names, superstep and byte-size counts; the run's input is never echoed
expected: state_delta payloads never carry field values, vault-confined values or full Battlefield state -- only changed field names, superstep and byte-size counts; the run's input is never echoed
result: pass
source: automated
coverage_id: 27-10/D6

### 68. [27-10 D7] paladin-web consumes RunEventStreamPort (a Stream of core RunStreamEvent) and never names the bus, the engine or TraceEvent
expected: paladin-web consumes RunEventStreamPort (a Stream of core RunStreamEvent) and never names the bus, the engine or TraceEvent
result: pass
source: automated
coverage_id: 27-10/D7

### 69. [27-11 D1] RunSchedule/RunScheduleId/ThreadStrategy/OnMissed core types exist with the documented serde shapes: ThreadStrategy::NewThreadPerTick is the default and serializes as the bare string \"new_thread_per_tick\"; FixedThread(ThreadId) serializes as {\"fixed_thread\":\"<id>\"}; OnMissed::Skip is the default
expected: RunSchedule/RunScheduleId/ThreadStrategy/OnMissed core types exist with the documented serde shapes: ThreadStrategy::NewThreadPerTick is the default and serializes as the bare string \"new_thread_per_tick\"; FixedThread(ThreadId) serializes as {\"fixed_thread\":\"<id>\"}; OnMissed::Skip is the default
result: pass
source: automated
coverage_id: 27-11/D1

### 70. [27-11 D2] cron::parse_run_cron accepts both 5-field and 6-field cron forms via croner's with_seconds_optional(), computing identical next-occurrence instants for both; IANA timezones parse via chrono-tz; wrong field count and unknown timezone produce typed CronParseError variants; scheduler.rs's existing six-field TokioCronSchedulerAdapter validation is byte-behaviourally unchanged, now delegating only its field-COUNTING primitive
expected: cron::parse_run_cron accepts both 5-field and 6-field cron forms via croner's with_seconds_optional(), computing identical next-occurrence instants for both; IANA timezones parse via chrono-tz; wrong field count and unknown timezone produce typed CronParseError variants; scheduler.rs's existing six-field TokioCronSchedulerAdapter validation is byte-behaviourally unchanged, now delegating only its field-COUNTING primitive
result: pass
source: automated
coverage_id: 27-11/D2

### 71. [27-11 D3] InMemoryRunScheduleRepository, SqliteRunScheduleRepository and PostgresRunScheduleRepository all pass the identical 10-clause contract suite, including claim_tick_race_admits_exactly_one (8 concurrent claim_tick calls against one schedule -> exactly one true)
expected: InMemoryRunScheduleRepository, SqliteRunScheduleRepository and PostgresRunScheduleRepository all pass the identical 10-clause contract suite, including claim_tick_race_admits_exactly_one (8 concurrent claim_tick calls against one schedule -> exactly one true)
result: pass
source: automated
coverage_id: 27-11/D3

### 72. [27-11 D4] ScheduleService::tick_once claims a tick (RunScheduleRepositoryPort::claim_tick) BEFORE ever calling RunSubmissionPort::submit; a restarted service instance over the same repository fires a schedule exactly once per due tick (schedule_restart_exactly_once); OnMissed::Skip recomputes next_tick from now without submitting after a >2x-tick-interval-late discovery, OnMissed::RunOnce submits exactly once then recomputes
expected: ScheduleService::tick_once claims a tick (RunScheduleRepositoryPort::claim_tick) BEFORE ever calling RunSubmissionPort::submit; a restarted service instance over the same repository fires a schedule exactly once per due tick (schedule_restart_exactly_once); OnMissed::Skip recomputes next_tick from now without submitting after a >2x-tick-interval-late discovery, OnMissed::RunOnce submits exactly once then recomputes
result: pass
source: automated
coverage_id: 27-11/D4

### 73. [27-11 D5] Two ScheduleService instances over one InMemoryRunScheduleRepository racing tick_once concurrently against 50 simultaneously-due schedules submit exactly 50 runs total, never 100; a FixedThread schedule landing on a busy thread increments skipped_ticks and submits nothing
expected: Two ScheduleService instances over one InMemoryRunScheduleRepository racing tick_once concurrently against 50 simultaneously-due schedules submit exactly 50 runs total, never 100; a FixedThread schedule landing on a busy thread increments skipped_ticks and submits nothing
result: pass
source: automated
coverage_id: 27-11/D5

### 74. [27-12 D1] AssistantValidator makes compile the validation gate: Agent bodies validate structurally into a real Paladin, Workflow bodies validate via WarGraphDoc::compile against EngineRegistries with every CompileError variant mapped to a distinct ValidationViolation code, and a credential_field_forbidden scan rejects api_key/token/secret/authorization at any depth
expected: AssistantValidator makes compile the validation gate: Agent bodies validate structurally into a real Paladin, Workflow bodies validate via WarGraphDoc::compile against EngineRegistries with every CompileError variant mapped to a distinct ValidationViolation code, and a credential_field_forbidden scan rejects api_key/token/secret/authorization at any depth
result: pass
source: automated
coverage_id: 27-12/D1

### 75. [27-12 D2] No update method exists anywhere on AssistantAdminPort or AssistantRepositoryPort; a stored version is immutable -- publishing v2 never rewrites v1, proven behaviorally
expected: No update method exists anywhere on AssistantAdminPort or AssistantRepositoryPort; a stored version is immutable -- publishing v2 never rewrites v1, proven behaviorally
result: pass
source: automated
coverage_id: 27-12/D2

### 76. [27-12 D3] POST /runs without version resolves a stored assistant's latest through insert_with_latest (D-30); publishing a new version after a run was submitted never changes the already-persisted run's frozen version
expected: POST /runs without version resolves a stored assistant's latest through insert_with_latest (D-30); publishing a new version after a run was submitted never changes the already-persisted run's frozen version
result: pass
source: automated
coverage_id: 27-12/D3

### 77. [27-12 D4] The seven /v1/assistants* routes exist on the shared RunApiState, merged into openapi.json; no PUT/PATCH route registered; mutating routes require admin and reject a code-registered id with 409 before the admin port is ever called
expected: The seven /v1/assistants* routes exist on the shared RunApiState, merged into openapi.json; no PUT/PATCH route registered; mutating routes require admin and reject a code-registered id with 409 before the admin port is ever called
result: pass
source: automated
coverage_id: 27-12/D4

### 78. [27-12 D5] POST /assistants with an empty definition body {} returns 400 with a non-empty machine-readable violation list in details, and nothing is persisted (a follow-up GET returns 404)
expected: POST /assistants with an empty definition body {} returns 400 with a non-empty machine-readable violation list in details, and nothing is persisted (a follow-up GET returns 404)
result: pass
source: automated
coverage_id: 27-12/D5

### 79. [27-13 D1] Delivery is a persisted queue (webhook_deliveries with next_attempt_at) drained by WebhookDeliveryService, never a spawned task; claim_due is a per-row conditional UPDATE proven under an 8-task race to admit each due row exactly once on all three adapters
expected: Delivery is a persisted queue (webhook_deliveries with next_attempt_at) drained by WebhookDeliveryService, never a spawned task; claim_due is a per-row conditional UPDATE proven under an 8-task race to admit each due row exactly once on all three adapters
result: pass
source: automated
coverage_id: 27-13/D1

### 80. [27-13 D2] X-Paladin-Signature: sha256=<hex> is an HMAC-SHA256 over the exact byte buffer signed once and sent verbatim (never re-serialized); a mockito receiver recomputing the HMAC over the RAW captured body matches the header exactly
expected: X-Paladin-Signature: sha256=<hex> is an HMAC-SHA256 over the exact byte buffer signed once and sent verbatim (never re-serialized); a mockito receiver recomputing the HMAC over the RAW captured body matches the header exactly
result: pass
source: automated
coverage_id: 27-13/D2

### 81. [27-13 D3] The SSRF guard is a standalone table-tested function applied at write time AND send time: non-http(s), loopback, link-local, RFC1918, unique-local, unspecified and the cloud metadata address (169.254.169.254, ALWAYS rejected regardless of allow_private) are all rejected; the webhook client follows no redirects
expected: The SSRF guard is a standalone table-tested function applied at write time AND send time: non-http(s), loopback, link-local, RFC1918, unique-local, unspecified and the cloud metadata address (169.254.169.254, ALWAYS rejected regardless of allow_private) are all rejected; the webhook client follows no redirects
result: pass
source: automated
coverage_id: 27-13/D3

### 82. [27-13 D4] 4xx (and 3xx, since redirects are not followed) dead-letters immediately; 5xx/timeout/connect-error retries up to 5 attempts with 1s..60s exponential backoff; 2xx delivers; the clock is injectable
expected: 4xx (and 3xx, since redirects are not followed) dead-letters immediately; 5xx/timeout/connect-error retries up to 5 attempts with 1s..60s exponential backoff; 2xx delivers; the clock is injectable
result: pass
source: automated
coverage_id: 27-13/D4

### 83. [27-13 D5] Delivery outcome never affects run status; every attempt is persisted and queryable (PLAT-FR-14) -- a webhook-delivery-repository error is logged and never changes the run's own status
expected: Delivery outcome never affects run status; every attempt is persisted and queryable (PLAT-FR-14) -- a webhook-delivery-repository error is logged and never changes the run's own status
result: pass
source: automated
coverage_id: 27-13/D5

### 84. [27-13 D6] Webhook payloads and persisted delivery rows contain only { run_id, thread_id, assistant, status, event, timestamp, attempt, parleys? } -- no signing value, API key, run input or Battlefield state ever appears
expected: Webhook payloads and persisted delivery rows contain only { run_id, thread_id, assistant, status, event, timestamp, attempt, parleys? } -- no signing value, API key, run input or Battlefield state ever appears
result: pass
source: automated
coverage_id: 27-13/D6

### 85. [27-14 D1] ScheduleAdminPort::create validates the cron (5/6 field), the IANA timezone, the assistant reference (when a resolver is wired), a FixedThread's ThreadId, and the webhook URL (write-time SSRF guard) BEFORE ever persisting -- each failure reports a distinct /path violation and nothing is written on any failure path
expected: ScheduleAdminPort::create validates the cron (5/6 field), the IANA timezone, the assistant reference (when a resolver is wired), a FixedThread's ThreadId, and the webhook URL (write-time SSRF guard) BEFORE ever persisting -- each failure reports a distinct /path violation and nothing is written on any failure path
result: pass
source: automated
coverage_id: 27-14/D1

### 86. [27-14 D2] ScheduleAdminPort::patch recomputes next_tick when cron or timezone changes, leaves next_tick in place for an enabled-only change, re-runs the SSRF guard on a webhook change, and delete-then-get returns None
expected: ScheduleAdminPort::patch recomputes next_tick when cron or timezone changes, leaves next_tick in place for an enabled-only change, re-runs the SSRF guard on a webhook change, and delete-then-get returns None
result: pass
source: automated
coverage_id: 27-14/D2

### 87. [27-14 D3] SsrfGuard::check_url rejects non-http(s) schemes and every literal-IP host classifying as loopback, link-local (incl. the 169.254.169.254 metadata address), RFC1918, unique-local or unspecified, plus the well-known name 'localhost'; allow_private overrides every rejection
expected: SsrfGuard::check_url rejects non-http(s) schemes and every literal-IP host classifying as loopback, link-local (incl. the 169.254.169.254 metadata address), RFC1918, unique-local or unspecified, plus the well-known name 'localhost'; allow_private overrides every rejection
result: pass
source: automated
coverage_id: 27-14/D3

### 88. [27-14 D4] The five /v1/schedules* routes exist on the shared RunApiState, merged into openapi.json with PATCH registered; create/patch/delete require_admin (403 for a non-admin principal), reads need authentication only, and every route answers 501 naming schedules.enabled when unwired
expected: The five /v1/schedules* routes exist on the shared RunApiState, merged into openapi.json with PATCH registered; create/patch/delete require_admin (403 for a non-admin principal), reads need authentication only, and every route answers 501 naming schedules.enabled when unwired
result: pass
source: automated
coverage_id: 27-14/D4

### 89. [27-14 D5] ScheduleResponse never echoes a raw webhook secret -- it renders \"***\" when a secret is set on the stored schedule and null otherwise, even though the secret is accepted on write
expected: ScheduleResponse never echoes a raw webhook secret -- it renders \"***\" when a secret is set on the stored schedule and null otherwise, even though the secret is accepted on write
result: pass
source: automated
coverage_id: 27-14/D5

### 90. [27-15 D1] pagination.rs's resolve_limit/encode_cursor/decode_cursor is the one shared implementation every list handler (runs, threads, assistants, schedules) calls; limit=0/101 -> 400, limit=100 succeeds, malformed cursor -> 400 invalid_cursor never 500
expected: pagination.rs's resolve_limit/encode_cursor/decode_cursor is the one shared implementation every list handler (runs, threads, assistants, schedules) calls; limit=0/101 -> 400, limit=100 succeeds, malformed cursor -> 400 invalid_cursor never 500
result: pass
source: automated
coverage_id: 27-15/D1

### 91. [27-15 D2] GET /v1/runs is filterable/paginated with an empty-set 200 shape; POST /v1/runs/{id}/cancel is idempotent 202 with D-46 authorization and 409 on a terminal run; GET /v1/runs/{id}/webhook-deliveries is paginated and never leaks a secret
expected: GET /v1/runs is filterable/paginated with an empty-set 200 shape; POST /v1/runs/{id}/cancel is idempotent 202 with D-46 authorization and 409 on a terminal run; GET /v1/runs/{id}/webhook-deliveries is paginated and never leaks a secret
result: pass
source: automated
coverage_id: 27-15/D2

### 92. [27-15 D3] D-46 two-tier scopes hold at the router level: submit/cancel are invocation-shaped (Forbidden -> 403), the admin-gated assistant route merged onto the SAME run_router is still 403 for a non-admin, an unauthenticated request is 401, and the pre-existing rate limiter answers 429 on /v1/runs for the first time
expected: D-46 two-tier scopes hold at the router level: submit/cancel are invocation-shaped (Forbidden -> 403), the admin-gated assistant route merged onto the SAME run_router is still 403 for a non-admin, an unauthenticated request is 401, and the pre-existing rate limiter answers 429 on /v1/runs for the first time
result: pass
source: automated
coverage_id: 27-15/D3

### 93. [27-15 D4] GET /threads, GET /threads/{id}, POST /threads/{id}/fork and DELETE /threads/{id} exist on ThreadApiState (now #[non_exhaustive] with runs/run_submission), covering the 501/200/400/403/404/409 table
expected: GET /threads, GET /threads/{id}, POST /threads/{id}/fork and DELETE /threads/{id} exist on ThreadApiState (now #[non_exhaustive] with runs/run_submission), covering the 501/200/400/403/404/409 table
result: pass
source: automated
coverage_id: 27-15/D4

### 94. [27-15 D5] WorkerDispatch::Fork drives WarEngine::fork exactly once when a run's fork_from Waypoint has not yet been produced, and falls through to normal resume once it has; a run forked from its second superstep's Waypoint completes end to end and its history records fork_of == Some(wp2)
expected: WorkerDispatch::Fork drives WarEngine::fork exactly once when a run's fork_from Waypoint has not yet been produced, and falls through to normal resume once it has; a run forked from its second superstep's Waypoint completes end to end and its history records fork_of == Some(wp2)
result: pass
source: automated
coverage_id: 27-15/D5

### 95. [27-15 D6] Ten concurrent POST /v1/runs for one thread through the real run_router over SqliteRunRepository produce exactly one 202 and nine 409 thread_busy (PRD acceptance 3, D-52), proven under real concurrent writers, not a single-threaded check-then-insert
expected: Ten concurrent POST /v1/runs for one thread through the real run_router over SqliteRunRepository produce exactly one 202 and nine 409 thread_busy (PRD acceptance 3, D-52), proven under real concurrent writers, not a single-threaded check-then-insert
result: pass
source: automated
coverage_id: 27-15/D6

### 96. [27-15 D7] schedule/admin.rs's independent, 27-14-era SsrfGuard duplicate is deleted and both write-time (schedule create/patch) and send-time (webhook client) checks route through the ONE shared webhook::SsrfGuard, with no behavior change to either call site
expected: schedule/admin.rs's independent, 27-14-era SsrfGuard duplicate is deleted and both write-time (schedule create/patch) and send-time (webhook client) checks route through the ONE shared webhook::SsrfGuard, with no behavior change to either call site
result: pass
source: automated
coverage_id: 27-15/D7

### 97. [27-16 D1] docs/src/api-reference/platform-api.md documents the run status machine (mermaid, exact D-02 edges), every endpoint with its auth tier and pagination, the seven SSE wire events and the degraded mode's stated limitation (no ordering guarantee, may coalesce supersteps), assistants (append-only, no PUT, synthetic code entries), schedules (5/6-field cron, UTC/IANA, strategies, on_missed, skipped_ticks), and webhooks (payload, signature verification recipe, retry table, SSRF rejection list, allow_private, DNS-rebinding limitation)
expected: docs/src/api-reference/platform-api.md documents the run status machine (mermaid, exact D-02 edges), every endpoint with its auth tier and pagination, the seven SSE wire events and the degraded mode's stated limitation (no ordering guarantee, may coalesce supersteps), assistants (append-only, no PUT, synthetic code entries), schedules (5/6-field cron, UTC/IANA, strategies, on_missed, skipped_ticks), and webhooks (payload, signature verification recipe, retry table, SSRF rejection list, allow_private, DNS-rebinding limitation)
result: pass
source: automated
coverage_id: 27-16/D1

### 98. [27-16 D2] The queue/worker topology page and the Kubernetes docs carry a worker-replica example manifest reading the APP_RUN_* env vars, and neither implies the in-process auth token store is multi-replica safe
expected: The queue/worker topology page and the Kubernetes docs carry a worker-replica example manifest reading the APP_RUN_* env vars, and neither implies the in-process auth token store is multi-replica safe
result: pass
source: automated
coverage_id: 27-16/D2

### 99. [27-16 D3] mdbook build succeeds under warning-policy = \"error\" — every link on the new/edited pages resolves
expected: mdbook build succeeds under warning-policy = \"error\" — every link on the new/edited pages resolves
result: pass
source: automated
coverage_id: 27-16/D3

### 100. [27-17 D1] build_run_api turns the seven config structs into a fully wired run API when run_store is enabled, and an unwired, no-task-spawning RunApiState when it is Disabled (the default) — every new route then answers 501 not_implemented
expected: build_run_api turns the seven config structs into a fully wired run API when run_store is enabled, and an unwired, no-task-spawning RunApiState when it is Disabled (the default) — every new route then answers 501 not_implemented
result: pass
source: automated
coverage_id: 27-17/D1

### 101. [27-17 D2] A configured postgres run store or redis queue on a binary built without storage-postgres/redis-queue is a startup error naming the missing cargo feature — never a silent fallback
expected: A configured postgres run store or redis queue on a binary built without storage-postgres/redis-queue is a startup error naming the missing cargo feature — never a silent fallback
result: pass
source: automated
coverage_id: 27-17/D2

### 102. [27-17 D3] run_store enabled with no waypoint store wired is a startup error naming waypoint_store.backend, never a silent InMemory/degraded fallback
expected: run_store enabled with no waypoint store wired is a startup error naming waypoint_store.backend, never a silent InMemory/degraded fallback
result: pass
source: automated
coverage_id: 27-17/D3

### 103. [27-17 D4] Code-registered agents are exposed to the run pipeline through CodeAgentResolver so POST /runs for a code-registered agent id resolves to Runnable::Agent, executed via the worker's wired PaladinPort — AgentRegistry itself is never mutated
expected: Code-registered agents are exposed to the run pipeline through CodeAgentResolver so POST /runs for a code-registered agent id resolves to Runnable::Agent, executed via the worker's wired PaladinPort — AgentRegistry itself is never mutated
result: pass
source: automated
coverage_id: 27-17/D4

### 104. [27-17 D5] paladin-server.rs's run() merges run_router alongside agent_router/thread_router before with_http_layers, threads handles.parley_extras into the SAME ParleyPortAdapter the thread surface builds (PLAT-FR-06), and threads run_repository/thread_run_submission onto ThreadApiState
expected: paladin-server.rs's run() merges run_router alongside agent_router/thread_router before with_http_layers, threads handles.parley_extras into the SAME ParleyPortAdapter the thread surface builds (PLAT-FR-06), and threads run_repository/thread_run_submission onto ThreadApiState
result: pass
source: automated
coverage_id: 27-17/D5

### 105. [27-17 D6] MIGRATION.md §9.5 lists every new config struct, key, env var and default; config.example.yml documents all seven subsystems as env-var-only
expected: MIGRATION.md §9.5 lists every new config struct, key, env var and default; config.example.yml documents all seven subsystems as env-var-only
result: pass
source: automated
coverage_id: 27-17/D6

### 106. [27-18 D1] PRD 06 acceptance 1 passes as one integration test: publish a Workflow assistant with a Gate, submit a run, observe SSE superstep-then-parley, the run suspends AwaitingInput, a mockito webhook receives the awaiting_input payload (parleys non-empty, attempt==1, X-Paladin-Signature verified via sign_webhook_body), resume via POST /threads/{id}/resume, the run completes (attempt==2 webhook, both deliveries recorded delivered), GET /threads/{id}/history shows >=3 waypoints with one AwaitingInput, and POST /threads/{id}/fork from the terminal waypoint reaches a terminal status — all Docker-free
expected: PRD 06 acceptance 1 passes as one integration test: publish a Workflow assistant with a Gate, submit a run, observe SSE superstep-then-parley, the run suspends AwaitingInput, a mockito webhook receives the awaiting_input payload (parleys non-empty, attempt==1, X-Paladin-Signature verified via sign_webhook_body), resume via POST /threads/{id}/resume, the run completes (attempt==2 webhook, both deliveries recorded delivered), GET /threads/{id}/history shows >=3 waypoints with one AwaitingInput, and POST /threads/{id}/fork from the terminal waypoint reaches a terminal status — all Docker-free
result: pass
source: automated
coverage_id: 27-18/D1

### 107. [27-18 D3] openapi.json is diff-reviewed as a whole for the phase: every new path is additive and no pre-existing path or schema changed except the registered ResumeAcceptedResponse.run_id
expected: openapi.json is diff-reviewed as a whole for the phase: every new path is additive and no pre-existing path or schema changed except the registered ResumeAcceptedResponse.run_id
result: pass
source: automated
coverage_id: 27-18/D3

### 108. [27-18 D4] MIGRATION.md §9.6 lists every new endpoint with its auth tier, and the resume-response field
expected: MIGRATION.md §9.6 lists every new endpoint with its auth tier, and the resume-response field
result: pass
source: automated
coverage_id: 27-18/D4

### 109. [27-18 D5] Workspace line coverage stays at or above the 82% floor under CI's exact invocation
expected: Workspace line coverage stays at or above the 82% floor under CI's exact invocation
result: pass
source: automated
coverage_id: 27-18/D5

### 110. [27-18 D6] .project/current-exports.txt is regenerated so the api-surface CI job passes on the phase's purely additive public-API growth
expected: .project/current-exports.txt is regenerated so the api-surface CI job passes on the phase's purely additive public-API growth
result: pass
source: automated
coverage_id: 27-18/D6

### 111. [27-19 D1] A message's first-ever dequeue from RedisRunQueue reports attempt == 1, matching InMemoryRunQueue, because the claim script only increments on a marker-carrying (previously-claimed) member
expected: A message's first-ever dequeue from RedisRunQueue reports attempt == 1, matching InMemoryRunQueue, because the claim script only increments on a marker-carrying (previously-claimed) member
result: pass
source: automated
coverage_id: 27-19/D1

### 112. [27-19 D2] A lease-expiry redelivery reports attempt == 2 and a nack requeue reports attempt == 2 on next dequeue, unchanged shared contract suite, both backends
expected: A lease-expiry redelivery reports attempt == 2 and a nack requeue reports attempt == 2 on next dequeue, unchanged shared contract suite, both backends
result: pass
source: automated
coverage_id: 27-19/D2

### 113. [27-19 D3] The claim marker is invisible to QueuedRun deserialization and its shape is pinned in the claim and nack scripts
expected: The claim marker is invisible to QueuedRun deserialization and its shape is pinned in the claim and nack scripts
result: pass
source: automated
coverage_id: 27-19/D3

### 114. [27-20 D1] storage_timestamp truncates toward zero at microsecond resolution (999ns past a boundary lands on the boundary, never the next one) and is the identity for values already at that resolution
expected: storage_timestamp truncates toward zero at microsecond resolution (999ns past a boundary lands on the boundary, never the next one) and is the identity for values already at that resolution
result: pass
source: automated
coverage_id: 27-20/D1

### 115. [27-20 D4] PLAT-01 adjacency/ordering probes: the keyset pagination clause with two fixtures sharing a submitted_at still separates them with no overlap/gap, tiebreak on descending run_id, unchanged by the precision normalization
expected: PLAT-01 adjacency/ordering probes: the keyset pagination clause with two fixtures sharing a submitted_at still separates them with no overlap/gap, tiebreak on descending run_id, unchanged by the precision normalization
result: pass
source: automated
coverage_id: 27-20/D4

### 116. [27-21 D1] A run submitted against the smoke boot reaches `completed` end to end, locally, with no network egress and no credential — the hermetic loopback LLM stub (mock-llm.py) plus the shared boot (lib-boot.sh) plus the generator-free curl round trip (smoke-http.sh).
expected: A run submitted against the smoke boot reaches `completed` end to end, locally, with no network egress and no credential — the hermetic loopback LLM stub (mock-llm.py) plus the shared boot (lib-boot.sh) plus the generator-free curl round trip (smoke-http.sh).
result: pass
source: automated
coverage_id: 27-21/D1

### 117. [27-21 D2] A committed package-lock.json makes `npm ci` succeed in scripts/sdk-smoke/, so the TypeScript half of the sdk-clients CI job actually runs; the generated client installs separately via --no-save, never as an unresolvable file: dependency in the lockfile.
expected: A committed package-lock.json makes `npm ci` succeed in scripts/sdk-smoke/, so the TypeScript half of the sdk-clients CI job actually runs; the generated client installs separately via --no-save, never as an unresolvable file: dependency in the lockfile.
result: pass
source: automated
coverage_id: 27-21/D2

### 118. [27-22 D1] A webhook receiver's response body is bounded as it is read (CR-01): at most MAX_ERROR_BODY_BYTES ever enter memory for one attempt, proven against a body an order of magnitude over a small test cap, a body smaller than the cap, and a body exactly at the cap
expected: A webhook receiver's response body is bounded as it is read (CR-01): at most MAX_ERROR_BODY_BYTES ever enter memory for one attempt, proven against a body an order of magnitude over a small test cap, a body smaller than the cap, and a body exactly at the cap
result: pass
source: automated
coverage_id: 27-22/D1

### 119. [27-22 D2] The bounded body still goes through redact-then-truncate before being persisted as last_error, preserving the existing D-43 ordering
expected: The bounded body still goes through redact-then-truncate before being persisted as last_error, preserving the existing D-43 ordering
result: pass
source: automated
coverage_id: 27-22/D2

### 120. [27-22 D3] A signing-key load failure (WR-01) reschedules the delivery instead of sending a mis-signed payload — proven by a receiver mock recording zero hits
expected: A signing-key load failure (WR-01) reschedules the delivery instead of sending a mis-signed payload — proven by a receiver mock recording zero hits
result: pass
source: automated
coverage_id: 27-22/D3

### 121. [27-22 D4] The full pre-existing webhook suite (retry schedule, receiver-side signature verification, payload contents, send-time SSRF, spawn idle poll) is unaffected by both changes
expected: The full pre-existing webhook suite (retry schedule, receiver-side signature verification, payload contents, send-time SSRF, spawn idle poll) is unaffected by both changes
result: pass
source: automated
coverage_id: 27-22/D4

### 122. [27-23 D1] LeaseHeartbeat::spawn starts no background task for a non-positive lease and logs a warn naming the misuse (WR-04)
expected: LeaseHeartbeat::spawn starts no background task for a non-positive lease and logs a warn naming the misuse (WR-04)
result: pass
source: automated
coverage_id: 27-23/D1

### 123. [27-23 D2] A positive-lease heartbeat still extends at lease/4 exactly as before (D-10 regression guard)
expected: A positive-lease heartbeat still extends at lease/4 exactly as before (D-10 regression guard)
result: pass
source: automated
coverage_id: 27-23/D2

### 124. [27-23 D3] An Agent-kind run carrying a webhook spec completes normally and enqueues zero deliveries, proving the carve-out (WR-02)
expected: An Agent-kind run carrying a webhook spec completes normally and enqueues zero deliveries, proving the carve-out (WR-02)
result: pass
source: automated
coverage_id: 27-23/D3

### 125. [27-23 D4] The Agent-kind carve-out is stated in worker.rs's event_bus/webhook_deliveries field docs, run_agent's own doc, and webhook/mod.rs's WebhookPayload docs
expected: The Agent-kind carve-out is stated in worker.rs's event_bus/webhook_deliveries field docs, run_agent's own doc, and webhook/mod.rs's WebhookPayload docs
result: pass
source: automated
coverage_id: 27-23/D4

### 126. [27-23 D5] docs/src/api-reference/platform-api.md publishes the Agent-kind webhook/stream carve-out under a new Known limitations subsection
expected: docs/src/api-reference/platform-api.md publishes the Agent-kind webhook/stream carve-out under a new Known limitations subsection
result: pass
source: automated
coverage_id: 27-23/D5

### 127. [27-23 D6] run_controller.rs's module docs state the unscoped read model (every run in the deployment, UUIDv7 enumeration) in a new Read scope section
expected: run_controller.rs's module docs state the unscoped read model (every run in the deployment, UUIDv7 enumeration) in a new Read scope section
result: pass
source: automated
coverage_id: 27-23/D6

### 128. [27-23 D7] docs/src/api-reference/platform-api.md's Authentication and scopes section states the unscoped read model concretely, replacing the general deferral sentence
expected: docs/src/api-reference/platform-api.md's Authentication and scopes section states the unscoped read model concretely, replacing the general deferral sentence
result: pass
source: automated
coverage_id: 27-23/D7

### 129. [27-23 D8] run_controller.rs's own unit suite is undisturbed by the docs-only change
expected: run_controller.rs's own unit suite is undisturbed by the docs-only change
result: pass
source: automated
coverage_id: 27-23/D8

### 130. [27-23 D9] Ledger rows 31 (WR-02) and 32 (WR-03) opened with named closing conditions, counters bumped consistently
expected: Ledger rows 31 (WR-02) and 32 (WR-03) opened with named closing conditions, counters bumped consistently
result: pass
source: automated
coverage_id: 27-23/D9

### 131. [27-24 D1] scripts/normalize-api-bounds.py canonicalises adjacent auto-trait marker bound ordering, with a --self-test mode covering convergence, sorted-run emission, byte-identical passthrough of ordinary lines, and a real RunWorkerPool<W> line from both observed toolchain orderings
expected: scripts/normalize-api-bounds.py canonicalises adjacent auto-trait marker bound ordering, with a --self-test mode covering convergence, sorted-run emission, byte-identical passthrough of ordinary lines, and a real RunWorkerPool<W> line from both observed toolchain orderings
result: pass
source: automated
coverage_id: 27-24/D1

### 132. [27-24 D2] .project/current-exports.txt regenerated through the canonicalising extraction; proven to contain no real public-API change via normalised-old-vs-new empty diff and equal pub-item counts (3763 = 3763)
expected: .project/current-exports.txt regenerated through the canonicalising extraction; proven to contain no real public-API change via normalised-old-vs-new empty diff and equal pub-item counts (3763 = 3763)
result: pass
source: automated
coverage_id: 27-24/D2

### 133. [27-24 D3] e2e-platform-api CI job runs cargo test --features web-server --test e2e_platform_api on every push/PR, with no needs: edge, and fails if the run selects zero tests
expected: e2e-platform-api CI job runs cargo test --features web-server --test e2e_platform_api on every push/PR, with no needs: edge, and fails if the run selects zero tests
result: pass
source: automated
coverage_id: 27-24/D3

### 134. [27-25 D1] Redis Run Queue Contract Suite (live server) proven green on live CI with the declared-vs-passed guard intact (17 declared, 19 passed) and the live-server log line present
expected: Redis Run Queue Contract Suite (live server) proven green on live CI with the declared-vs-passed guard intact (17 declared, 19 passed) and the live-server log line present
result: pass
source: automated
coverage_id: 27-25/D1

### 135. [27-25 D2] Postgres Storage Contract Suites (live server) proven green, including the new postgres_run_timestamps_round_trip_at_microsecond_precision clause and the three previously-failing run::postgres clauses
expected: Postgres Storage Contract Suites (live server) proven green, including the new postgres_run_timestamps_round_trip_at_microsecond_precision clause and the three previously-failing run::postgres clauses
result: pass
source: automated
coverage_id: 27-25/D2

### 136. [27-25 D3] Coverage job completes with no exit 101 and reports 89.98% workspace line coverage, above the 82% ADR-0006 floor
expected: Coverage job completes with no exit 101 and reports 89.98% workspace line coverage, above the 82% ADR-0006 floor
result: pass
source: automated
coverage_id: 27-25/D3

### 137. [27-25 D4] Generated SDK Clients (Python + TypeScript) smoke green, both clients generate >=20 files, submit a run, and reach terminal status exactly 'completed'
expected: Generated SDK Clients (Python + TypeScript) smoke green, both clients generate >=20 files, submit a run, and reach terminal status exactly 'completed'
result: pass
source: automated
coverage_id: 27-25/D4

### 138. [27-25 D5] API Surface Tracking green with 'API surface unchanged' on CI's own pinned toolchain
expected: API Surface Tracking green with 'API surface unchanged' on CI's own pinned toolchain
result: pass
source: automated
coverage_id: 27-25/D5

### 139. [27-25 D6] e2e-platform-api job (new, added by 27-24) runs for the first time in CI and passes with a non-zero test count
expected: e2e-platform-api job (new, added by 27-24) runs for the first time in CI and passes with a non-zero test count
result: pass
source: automated
coverage_id: 27-25/D6

### 140. [27-25 D7] Regression guards (Unit Tests, MSRV 1.88, Semver Checks vs v0.9.0) and the Integration Tests job all green at the evidence SHA
expected: Regression guards (Unit Tests, MSRV 1.88, Semver Checks vs v0.9.0) and the Integration Tests job all green at the evidence SHA
result: pass
source: automated
coverage_id: 27-25/D7

### 141. [27-26 D1] run_all provisions a fresh, empty queue for every one of the eight contract clauses (fifo, lease-expiry, extend-lease, ack, nack, expired-token, unknown-token, depth), eliminating the suite-isolation defect that leaked leases from earlier clauses into ack_removes_message_permanently's depth() assertion
expected: run_all provisions a fresh, empty queue for every one of the eight contract clauses (fifo, lease-expiry, extend-lease, ack, nack, expired-token, unknown-token, depth), eliminating the suite-isolation defect that leaked leases from earlier clauses into ack_removes_message_permanently's depth() assertion
result: pass
source: automated
coverage_id: 27-26/D1

### 142. [27-26 D2] Running run_all twice against the same factory passes both times (idempotency: no state carried between separate run_all invocations, not just between clauses within one invocation)
expected: Running run_all twice against the same factory passes both times (idempotency: no state carried between separate run_all invocations, not just between clauses within one invocation)
result: pass
source: automated
coverage_id: 27-26/D2

### 143. [27-26 D3] The Redis backend also exercises the fixed run_all, via a factory that builds a fresh RedisRunQueue with its own randomized key_prefix per clause, without adding or removing any #[tokio::test] in redis.rs and without changing the SKIP path/message
expected: The Redis backend also exercises the fixed run_all, via a factory that builds a fresh RedisRunQueue with its own randomized key_prefix per clause, without adding or removing any #[tokio::test] in redis.rs and without changing the SKIP path/message
result: pass
source: automated
coverage_id: 27-26/D3

## Summary

total: 143
passed: 143
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
