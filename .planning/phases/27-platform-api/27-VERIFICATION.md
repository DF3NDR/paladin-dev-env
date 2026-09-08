---
phase: 27-platform-api
verified: 2026-09-08T12:10:00Z
status: gaps_found
score: 12/17 must-have truth clusters verified (5 confirmed gaps; see below)
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "PLAT-FR-03 redelivery: 'A message dequeued with lease L is invisible to every other dequeue until L elapses, then becomes visible again with the same run_id and an attempt one higher — on InMemoryRunQueue and RedisRunQueue alike' (27-03 must_haves, D-06)."
    status: failed
    reason: "Live CI run 34222317640, job 'Redis Run Queue Contract Suite' (log 102048101987): 13 passed, 3 FAILED — redis_run_queue_full_contract_suite_via_run_all, redis_run_queue_lease_expiry_redelivers_with_attempt_incremented, redis_run_queue_nack_requeues_after_delay_with_attempt_incremented. All three fail the SAME assertion shape: `contract_tests.rs:105` expects `leased.queued.attempt == 1` on the very FIRST dequeue after enqueue but observes `2`. Root cause confirmed by reading the adapters: `RUN_QUEUE_CLAIM_LUA` (crates/paladin-storage/src/run_queue/redis.rs:111, `decoded.attempt = decoded.attempt + 1`) increments attempt UNCONDITIONALLY on every claim, including the very first claim of a freshly-enqueued message — whereas `InMemoryRunQueue::dequeue` (crates/paladin-storage/src/run_queue/in_memory.rs:138-164) does NOT increment attempt on a fresh claim; it only increments in `reclaim_expired_leases` (:75, on lease-expiry redelivery) and `nack` (:195, on explicit nack). The nack Lua (`redis.rs:208`, also unconditional `decoded.attempt = decoded.attempt + 1`) double-counts for the same reason on the nack path (contract_tests.rs:220 expects attempt==1 on first dequeue before the nack under test)."
    artifacts:
      - path: "crates/paladin-storage/src/run_queue/redis.rs"
        issue: "RUN_QUEUE_CLAIM_LUA (~line 111) increments `attempt` on every claim; must only increment when reclaiming a message whose lease already expired (redelivery), not on the first-ever claim of a message that was never leased before."
    missing:
      - "Encode the enqueue-time attempt (or a 'never claimed' marker) in the ZSET member so RUN_QUEUE_CLAIM_LUA can distinguish a first claim (no increment) from a lease-expiry reclaim (increment), matching InMemoryRunQueue's semantics exactly."
      - "Re-run the Redis contract suite (`RUN_QUEUE_REDIS_TEST_URL` set) and confirm all tests in crates/paladin-storage/src/run_queue/contract_tests.rs pass, including the two assertions at :105 and :220."
    impact_note: "This same root cause also fails the `coverage` CI job (log job-102048101815.log): with a live Redis service (`redis=localhost:6380`), `cargo llvm-cov --workspace --features integration-tests,llm-all ...` hits the identical three assertions in paladin-storage (279 passed; 3 failed) and the whole coverage run aborts with exit 101 BEFORE any line-coverage percentage is computed — so the 82% floor is currently unproven on CI (not failed on the number, blocked before measurement). This is not a second, independent gap; it resolves automatically once the Redis attempt-counter fix above lands. The only coverage figure of record remains 27-18's local Tier-1-scoped 89.84%."
  - truth: "27-02 must_haves: 'Every RunStatus transition through SqliteRunRepository and PostgresRunRepository is a single compare-and-set UPDATE...' — proven by the shared contract suite round-tripping every persisted field, including timestamps, without loss (D-01 X-04, D-03)."
    status: failed
    reason: "Live CI run 34222317640, job 'Postgres Storage Contract Suites' (log 102048101953): 83 passed, 3 FAILED, all in `run::postgres::tests`: insert_then_get_round_trips_every_field (contract_tests.rs:88), update_status_queued_to_running_sets_started_at_then_stale_cas_fails (contract_tests.rs:126), update_status_running_to_completed_sets_finished_at_then_terminal_is_absorbing (contract_tests.rs:165). Each fails a `DateTime<Utc>` equality assertion where the left (originally-constructed Rust value, nanosecond precision, e.g. `...533307246Z`) does not equal the right (value read back from Postgres, e.g. `...533307Z`) — Postgres TIMESTAMPTZ stores microsecond precision and silently truncates the sub-microsecond digits on write. `run/postgres.rs` (~line 332, `INSERT_RUN`) binds the nanosecond-precision `chrono::DateTime<Utc>` values unchanged; nothing truncates to µs before persisting or before comparing on read-back. assistant::postgres, run_schedule::postgres, webhook::postgres and waypoint::postgres suites in the SAME job all pass — this is confined to the `run` repository's timestamp fields (submitted_at/started_at/finished_at)."
    artifacts:
      - path: "crates/paladin-storage/src/run/postgres.rs"
        issue: "Persists nanosecond-precision DateTime<Utc> values into a TIMESTAMPTZ column (microsecond precision) with no truncation, so a naive round-trip equality check fails."
      - path: "crates/paladin-storage/src/run/contract_tests.rs"
        issue: "Lines 88, 126, 165 assert exact DateTime<Utc> equality between the value passed in and the value read back, which cannot hold across a microsecond-precision store."
    missing:
      - "Truncate timestamps to microsecond precision before insert/compare in the Postgres adapter (or in the shared contract suite's helper), consistent with Postgres's native TIMESTAMPTZ resolution — the same fix class this codebase already documents for the analogous waypoint suite (which passes, so a working precedent likely exists to copy)."
      - "Re-run the Postgres run-repository contract suite (STORAGE_POSTGRES_TEST_URL set) and confirm 0 failures."
  - truth: "27-18 must_haves: 'A sdk-clients CI job generates a Python and a TypeScript client from openapi.json ... boots paladin-server ... and smoke-tests list assistants → submit run → poll status from each client; it runs on every PR' (D-49, PLAT-FR-17)."
    status: failed
    reason: "Live CI run 34222317640, job 'Generated SDK Clients smoke' (log 102048101784) exited 1. Two independent causes: (1) The TypeScript smoke never runs: `npm ci` (scripts/sdk-smoke/run.sh:93) requires an existing package-lock.json, but scripts/sdk-smoke/ ships no lockfile (`npm error code EUSAGE ... npm ci can only install with an existing package-lock.json`). (2) The Python smoke reports 'all checks passed' despite the submitted run reaching status 'failed', not 'completed': scripts/sdk-smoke/smoke-config.yml configures the resident smoke agent with a REAL `openai` provider (`llm_type: openai`, `llm_url: https://api.openai.com/v1`) and an empty API key, so the LLM call fails with 'Authentication failed: Invalid API key for provider openai' — the run legitimately fails end to end. scripts/sdk-smoke/smoke.py's `poll_until_terminal` treats ANY status in TERMINAL_STATUSES = {completed, failed, halted, cancelled} as success and never asserts `status == 'completed'`, so the script prints 'SDK smoke (Python): all checks passed.' on a run that did not actually work — this also means the 27-18 prohibition ('MUST NOT report green when ... the smoke server was unreachable') is satisfied only by accident (the job still failed overall because of the TypeScript half); the Python half's own pass/fail logic does not actually prove a working client round trip."
    artifacts:
      - path: "scripts/sdk-smoke/package.json"
        issue: "No committed package-lock.json alongside it; run.sh's `npm ci` step cannot succeed without one."
      - path: "scripts/sdk-smoke/smoke-config.yml"
        issue: "Wires the smoke agent to the real OpenAI API (llm_type: openai, llm_url: https://api.openai.com/v1) with no valid key, instead of a mock/test provider — the submitted run cannot reach Completed."
      - path: "scripts/sdk-smoke/smoke.py"
        issue: "poll_until_terminal() accepts any of {completed, failed, halted, cancelled} as success; it should require status == 'completed' (or explicitly fail loudly on failed/halted/cancelled) to actually prove the generated client can drive a run to success."
    missing:
      - "Commit a package-lock.json for scripts/sdk-smoke/ (or change run.sh to `npm install` if a floating lockfile is intentional)."
      - "Point smoke-config.yml's smoke agent at a mock/test LLM double (matching how Tier-1 Rust tests use MockLlmAdapter/mockito) so the submitted run actually completes."
      - "Tighten smoke.py (and smoke.ts, for parity) to assert the terminal status is specifically 'completed', failing loudly otherwise."
  - truth: "27-18 must_haves: '.project/current-exports.txt is regenerated with ./scripts/extract-public-api.sh so the api-surface CI job passes on the phase's purely additive public-API growth' (X-10.1 inventory)."
    status: failed
    reason: "Live CI run 34222317640, job 'API Surface Tracking' (log 102048101721) FAILED. `scripts/check-api-surface.sh .project/current-exports.txt` diffs the committed baseline against a fresh cargo-public-api v0.52.0 extraction on CI's runner and finds a real difference — but the diff (visible after the `+++ /tmp/current-api.txt` marker) is confined to auto-trait `impl` bound ORDERING for `RunWorkerPool<W>` (`Sync + core::marker::Send` locally vs `Send + core::marker::Sync` on CI, repeated across Freeze/Send/Sync/Unpin/UnsafeUnpin), not a real public-API addition or removal. This is a toolchain-dependent nondeterminism in how rustc/cargo-public-api orders synthesized auto-trait bounds between the devcontainer's local Rust toolchain and CI's pinned toolchain — the committed baseline was generated locally and does not match what CI's pinned toolchain produces."
    artifacts:
      - path: ".project/current-exports.txt"
        issue: "Generated with a local toolchain whose cargo-public-api output orders auto-trait bounds differently from CI's pinned toolchain for RunWorkerPool<W>'s Freeze/Send/Sync/Unpin/UnsafeUnpin impls."
    missing:
      - "Regenerate .project/current-exports.txt using CI's exact toolchain (or run the extraction inside CI and commit the artifact it produces), then re-verify ./scripts/check-api-surface.sh reports 'API surface unchanged' both locally and on CI."
      - "If this class of drift is expected to recur across toolchain versions, consider normalizing/sorting auto-trait bound lists in scripts/extract-public-api.sh so the baseline is toolchain-order-independent."
  - truth: "27-18's own Task-3 checklist: '`test` — green, includes `e2e_platform_api`' (the phase's flagship PRD 06 acceptance-1 integration test)."
    status: failed
    reason: "Confirmed by direct inspection, not by a CI run: no CI job enables the `web-server` feature together with the `tests/integration/e2e_platform_api_test.rs` binary. Cargo.toml's `[[test]] name = \"e2e_platform_api\"` carries `required-features = [\"web-server\"]`; the `integration-tests` feature is `[]` (empty) so it does not imply `web-server`. `.github/workflows/ci.yml`'s `test` job runs `cargo test --workspace --lib --bins` only (no `--tests`, so no integration-test binaries run at all); its `integration` job runs `cargo test --workspace --features integration-tests --verbose -- --test-threads=1`, which does not enable `web-server` either. `.github/workflows/feature-flags.yml`'s `web-server` matrix entry runs `cargo test --workspace --lib --features web-server` — `--lib` only, so it never reaches the `tests/integration/` binaries. No job anywhere greps/invokes `e2e_platform_api` or `--test e2e_platform_api`. The test itself is real and passes locally (`cargo test --features web-server --test e2e_platform_api` → `test result: ok. 1 passed` in ~12s, re-confirmed during this verification), so the underlying functionality is correct — this is a CI-wiring gap, not a functional defect."
    artifacts:
      - path: ".github/workflows/ci.yml"
        issue: "Neither the `test` job nor the `integration` job runs the `e2e_platform_api` integration test binary (it needs `--features web-server --test e2e_platform_api` or equivalent, which no job supplies)."
    missing:
      - "Add a step (or extend an existing job) that runs `cargo test --features web-server --test e2e_platform_api` on every PR/push, so PRD 06 acceptance criterion 1 is continuously proven in CI rather than only locally."
deferred:
  - truth: "GET /assistants merges synthetic code-registry entries across every paginated page, not just the first."
    addressed_in: "Documented as an accepted, in-scope-for-later limitation (WINDOWS.md id 29, phase 27, status open — not phase-27-blocking per the plan's own documented decision in crates/paladin-web/src/assistant_controller.rs:396)."
    evidence: "WINDOWS.md row 29: 'GET /assistants merges synthetic code-registry entries only on the first page (cursor=None); a heterogeneous keyset merge ... is out of scope for 27-12 (documented in-code).' — a pre-existing, explicitly documented scope window, not a fresh verification finding."
  - truth: "Generated TypeScript client field/method names (Configuration/AssistantsApi/RunsApi) match smoke.ts's hand-written stub."
    addressed_in: "WINDOWS.md id 30, phase 27, status open — explicitly deferred to CI's real openapi-generator-cli run (which this verification's CI evidence shows currently fails for unrelated reasons — see the sdk-clients gap above)."
    evidence: "WINDOWS.md row 30: 'TypeScript generated-client field/method names ... could not be verified against the real openapi-generator-cli output locally ... CI's own sdk-clients job is the first real proof.'"
human_verification:
  - test: "Re-run `.github/workflows/ci.yml` on `feature/phase-26` at the commit that closes the five gaps above, and confirm the `coverage`, `msrv`, `semver` and `test` jobs are green (still pending/in-progress as of this verification, run 34222317640, pushed 2026-09-08T11:44:56Z). `pre-commit` (workflow run 34222317676) already PASSED and needs no re-check; `feature-flags` (34222317534) and `codeql` (34222317535, advisory-only) were still in progress."
    expected: "coverage completes and reports >= 82% once the Redis attempt-counter fix lands (it currently aborts with exit 101 before computing a percentage — see the Redis gap's impact_note; 27-18's local Tier-1-scoped figure remains 89.84%); msrv passes at 1.88; semver-checks passes (no undeclared breaking change); `test` job green."
    why_human: "These jobs need live CI infrastructure (this devcontainer has no Docker) and were still queued/in-progress or blocked by the still-open Redis gap when this verification ran."
  - test: "After closing the Redis run-queue and Postgres run-repository gaps, confirm the `redis-queue`, `postgres-integration` and `coverage` CI jobs are fully green (not just improved) — re-check the exact test counts against the module's own declared test count per D-51's self-skip guard, and confirm `coverage` now completes and reports a percentage >= 82%."
    expected: "redis-queue: log contains 'All run_queue::redis tests exercised the live server', 0 failed. postgres-integration: declared-vs-selected counts equal across waypoint/run/assistant/run_schedule/webhook, 0 failed. coverage: completes (no exit 101), percentage >= 82%."
    why_human: "Requires a live CI run against real Redis/Postgres services this devcontainer cannot provide."
---

# Phase 27: Platform API Verification Report

**Phase Goal:** Runs execute durably in the background on a worker pool, integrate with Parley pauses and live streaming, and are managed through versioned assistants, cron schedules and webhooks — all reachable over a production-shaped HTTP API.
**Verified:** 2026-09-08T12:10:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | `POST /runs` → 202 (enqueue only), `RunRepositoryPort` persists every status transition, monotonic status machine with typed illegal-transition errors (PLAT-01) | ⚠️ PARTIAL — code path VERIFIED, Postgres backend FAILED | `try_transition` + CAS `UPDATE` verified in code (run.rs:207, run/sqlite.rs:405); SQLite/InMemory contract suites pass locally. **Live CI (postgres-integration job, run 34222317640): 3/86 `run::postgres` tests FAIL** on exact-timestamp round-trip (see gaps). |
| 2 | Worker pool executes runs via `RunQueuePort` (InMemory/Redis) with lease heartbeats, at-least-once redelivery that resumes (not restarts), cross-instance cancellation at superstep boundaries, `409 ThreadBusy` under 10 concurrent submits (PLAT-02) | ⚠️ PARTIAL — InMemory/logic VERIFIED, Redis backend FAILED | Heartbeat `lease/4` (worker.rs:130), resume-not-restart dispatch (worker.rs), `CancellationProbe` wired into `WarEngine`/superstep (mod.rs:1615, superstep.rs:2032), partial unique index `idx_runs_thread_active` present and `ten_concurrent_submits_one_accepted` test present. **Live CI (redis-queue job, run 34222317640): 3/16 Redis contract tests FAIL** — attempt counter off-by-one on first dequeue (see gaps). |
| 3 | `AwaitingInput` releases the worker, `POST /threads/{id}/resume` re-enqueues under the same `run_id`, `GET /runs/{id}/stream` bridges live TraceSink events to SSE with documented degraded mode and 15s heartbeats (PLAT-03) | ✓ VERIFIED | `record_resume` + `enqueue` in parley/adapter.rs; `ResumeAcceptedResponse.run_id` present in thread_controller.rs; SSE seven wire event names + `KeepAlive::new().interval` in run_controller.rs/events.rs; **`tests/integration/e2e_platform_api_test.rs` (full lifecycle incl. AwaitingInput → webhook → resume → Completed → history → fork) re-run during this verification: `cargo test --features web-server --test e2e_platform_api` → `test result: ok. 1 passed` in ~12s.** Note: this test is not wired into any CI job (see gap 5) — locally proven, not continuously proven. |
| 4 | Assistants are append-only immutable versions (no PUT, ever), `latest` frozen at submit time, `WarGraphDoc` compiles through registry-resolving `compile()` with restart-stable fingerprint round-trip (PLAT-04) | ✓ VERIFIED | No PUT/PATCH route exists (`no_put_or_patch_route_exists` test, assistant_controller.rs:1097); `assistant_versions` PK `(assistant_id, version)`, no update method (assistant_repository_port.rs); `insert_with_latest` freezes latest transactionally; `WarGraphDoc::compile` + `CompileError::UnsupportedNodeKind` (graph_doc.rs); two-process fingerprint test present (`wargraph_doc_fingerprint_two_process`); **Postgres `assistant::postgres` suite: 10/10 pass in live CI.** |
| 5 | Cron schedules survive restart without duplicate/missed-then-double firing; HMAC-signed webhook delivery with bounded retry and SSRF guard; every new endpoint carries auth/rate limiting/scopes/pagination; `openapi.json` regenerated; Python/TS clients generated and smoke-tested in CI (PLAT-05, PLAT-06) | ⚠️ PARTIAL — schedules/webhooks/pagination/auth VERIFIED, SDK-client CI gate FAILED | `claim_tick` conditional UPDATE (run_schedule contract_tests.rs); `RUN_QUEUE_CLAIM_LUA`-style Lua for schedules N/A (uses SQL claim); HMAC `sign_webhook_body` (signature.rs) verified via mockito in e2e test; SSRF guard rejects metadata/private ranges (ssrf.rs); no-redirect webhook client (client.rs); `resolve_limit`/`decode_cursor` 400s verified (pagination.rs); **`run_schedule::postgres` (10/10) and `webhook::postgres` (9/9) pass in live CI.** **`sdk-clients` CI job FAILED** (live run 34222317640) and **`api-surface` CI job FAILED** (same run) — see gaps. |

**Score:** 2/5 ROADMAP success criteria fully green (SC3, SC4); 3/5 have a confirmed, CI-evidenced backend/CI-gate gap (SC1 Postgres timestamps, SC2 Redis attempt counter, SC5 sdk-clients + api-surface). 5 gaps total (see YAML frontmatter), plus the e2e_platform_api CI-wiring gap folded into SC3's evidence.

### Required Artifacts (representative sample across all 18 plans)

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/paladin-core/src/platform/container/run.rs` | RunId/RunStatus/try_transition/IllegalTransition | ✓ VERIFIED | 884 lines; `try_transition` + exhaustive self-transition-rejection test present |
| `crates/paladin-ports/src/output/run_queue_port.rs` + `run_repository_port.rs` | RunQueuePort/RunRepositoryPort traits | ✓ VERIFIED | present, substantive |
| `crates/paladin-storage/src/run/{sqlite,postgres,contract_tests}.rs` | Run repository adapters + shared suite | ⚠️ HOLLOW (Postgres) | Files present, substantive, wired; **Postgres adapter fails 3 contract-suite assertions in live CI** (timestamp precision) |
| `crates/paladin-storage/src/run_queue/{redis,in_memory,contract_tests}.rs` | Run queue adapters + shared suite | ⚠️ HOLLOW (Redis) | Files present, substantive, wired; **Redis adapter fails 3 contract-suite assertions in live CI** (attempt off-by-one) |
| `src/application/services/run/worker.rs` | RunWorkerPool, LeaseHeartbeat, dispatch | ✓ VERIFIED | 1482 lines; `lease/4` heartbeat, resume-not-restart dispatch, ShutdownCoordinator registration all present |
| `crates/paladin-battalion/src/engine/graph_doc.rs` | WarGraphDoc + compile() | ✓ VERIFIED | 1625 lines; typed CompileError variants present |
| `crates/paladin-ports/src/output/cancellation_probe.rs` | CancellationProbe trait | ✓ VERIFIED | wired into WarEngine builder and superstep boundary check |
| `src/application/services/run/webhook/{ssrf,signature,client,service}.rs` | SSRF guard, HMAC sign, no-redirect client, drain service | ✓ VERIFIED | all present, substantive; live CI `webhook::postgres` suite green |
| `src/application/services/run/schedule/{service,admin}.rs` | ScheduleService claim-then-submit | ✓ VERIFIED | `claim_tick` present; live CI `run_schedule::postgres` suite green |
| `crates/paladin-web/src/{run,assistant,schedule}_controller.rs` + `pagination.rs` | HTTP surface, pagination clamps | ✓ VERIFIED | routes present; `resolve_limit`/`decode_cursor` 400-path tests present |
| `tests/integration/e2e_platform_api_test.rs` | PRD 06 acceptance-1 lifecycle | ⚠️ VERIFIED LOCALLY, NOT IN CI | passes locally (1/1); no CI job runs it (gap 5) |
| `scripts/sdk-smoke/{smoke.py,smoke.ts,package.json,smoke-config.yml}` | SDK generated-client smoke | ✗ FAILING IN CI | present, but functionally broken — see gaps (missing lockfile, wrong LLM provider, weak success assertion) |
| `.project/current-exports.txt` | Public API baseline | ✗ FAILING IN CI | present, locally matches HEAD, but diverges from CI's toolchain output (auto-trait bound ordering) |
| `k8s/server/worker-deployment.yaml`, `docs/src/api-reference/platform-api.md` | Ops docs | ✓ VERIFIED | present, substantive |

### Key Link Verification (sample)

| From | To | Via | Status |
|---|---|---|---|
| `run_controller.rs` | `run_submission_port.rs` | `RunApiState` holds `Option<Arc<dyn RunSubmissionPort>>` | ✓ WIRED |
| `worker.rs` | `run_queue_port.rs` | `dequeue(` / `ack` / `nack` | ✓ WIRED (logic correct; Redis adapter's attempt semantics are the bug, not the wiring) |
| `worker.rs` | `run_repository_port.rs` | `update_status(` | ✓ WIRED |
| `engine/mod.rs` | `cancellation_probe.rs` | `with_cancellation_probe` → `superstep::run` | ✓ WIRED |
| `parley/adapter.rs` | `run_queue_port.rs` | `enqueue(QueuedRun {..})` after `record_resume` | ✓ WIRED |
| `assistant/validator.rs` | `graph_doc.rs` | `WarGraphDoc` deserialize + `compile()` | ✓ WIRED |
| `run/worker.rs` | `webhook_delivery_port.rs` | enqueue on terminal/AwaitingInput outcome | ✓ WIRED |
| `.github/workflows/ci.yml` | `crates/paladin-web/openapi.json` | `openapi-generator-cli -i ...` | ⚠️ WIRED BUT FAILING — job exits 1 (see gaps) |
| `.github/workflows/ci.yml` (`test` job) | `tests/integration/e2e_platform_api_test.rs` | *(none)* | ✗ NOT WIRED — no job supplies `--features web-server --test e2e_platform_api` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| PRD 06 acceptance-1 full lifecycle | `cargo test --features web-server --test e2e_platform_api` (re-run during this verification) | `test result: ok. 1 passed` in 12.28s | ✓ PASS (locally; not CI-wired, see gap) |
| No PUT/PATCH on assistants | code inspection: `no_put_or_patch_route_exists` test | asserts 404/405 for both methods | ✓ PASS |
| Pagination clamps | code inspection: `resolve_limit_zero_is_400`, `resolve_limit_101_is_400`, `decode_cursor_bad_base64_is_invalid_cursor_400` | present, asserting `bad_request`/`invalid_cursor` | ✓ PASS |

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` convention; verification instead relied on the live CI job results supplied by the orchestrator (run 34222317640, SHA `1e81939a`) plus one locally re-run named test (`e2e_platform_api`).

### Requirements Coverage

| Requirement | Description (abridged) | Status | Evidence |
|---|---|---|---|
| PLAT-01 | Run submission decoupled, RunRepositoryPort, monotonic status machine | ⚠️ PARTIAL | Status machine + SQLite/InMemory verified; **Postgres adapter fails contract suite in live CI (timestamp precision)** |
| PLAT-02 | Durable worker pool, RunQueuePort (InMemory+Redis), lease heartbeats, redelivery, cross-instance cancel, 409 ThreadBusy | ⚠️ PARTIAL | Worker/cancellation/busy-index all verified; **Redis adapter fails contract suite in live CI (attempt off-by-one)** |
| PLAT-03 | Parley/streaming integration | ✓ SATISFIED | Verified in code + locally re-run e2e test (not CI-wired — see gap 5, folded under PLAT-06/X-02 below) |
| PLAT-04 | Versioned assistants, WarGraphDoc | ✓ SATISFIED | No-PUT enforced; compile-is-validation; Postgres assistant suite green in live CI |
| PLAT-05 | Schedules and webhooks | ✓ SATISFIED | claim_tick, SSRF, HMAC all verified in code; Postgres schedule/webhook suites green in live CI |
| PLAT-06 | Production-shaped HTTP surface: auth, pagination, openapi, generated-client CI gate | ✗ BLOCKED | Auth/pagination/openapi verified in code; **sdk-clients CI job FAILED and api-surface CI job FAILED** in live CI; e2e_platform_api not wired into any CI job |

No orphaned requirements found — PLAT-01…06 are the complete set declared across the 18 plans' `requirements:` frontmatter, matching REQUIREMENTS.md lines 216-255 verbatim.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `MIGRATION.md` | 271, 538 | `TBD` | ℹ️ Info, not a blocker | Both lines explicitly name the owning phase/requirement (`SHIP-02`/`SHIP-01`, Phase 29) per this phase's own D-53 design ("Every TBD below carries the requirement or phase that owns closing it") — this is the documented, intentional MIGRATION.md structure, not an unresolved debt marker left by this phase. |

No other TBD/FIXME/XXX, empty-implementation, or hardcoded-empty-data patterns found across the ~124 files this phase's 18 plans declared in `files_modified`.

### Gaps Summary

Five concrete, CI-evidenced gaps block a clean pass, all traced to precise root causes:

1. **Redis run-queue attempt counter is off by one** (PLAT-02) — `RUN_QUEUE_CLAIM_LUA` increments `attempt` on every claim, not just on lease-expiry redelivery, so the very first dequeue already reports `attempt=2` instead of `1`. 3 live-CI test failures in the `redis-queue` job, plus the SAME 3 failures abort the `coverage` job (exit 101, before any percentage is computed) — the 82% floor is currently unproven on CI, not because coverage is too low but because the run never finishes. One fix closes both.
2. **Postgres run-repository timestamp round-trip loses precision** (PLAT-01) — nanosecond-precision Rust `DateTime<Utc>` values are compared for exact equality against microsecond-precision Postgres `TIMESTAMPTZ` reads. 3 live-CI test failures.
3. **`sdk-clients` CI job fails** (PLAT-06) — TypeScript smoke never runs (`npm ci` needs a missing `package-lock.json`); Python smoke silently "passes" despite the submitted run reaching `failed` because the smoke config wires a real (unauthenticated) OpenAI provider instead of a mock, and the smoke script accepts any terminal status as success.
4. **`api-surface` CI job fails** (PLAT-06/X-10.1) — the committed `.project/current-exports.txt` was generated with a local toolchain that orders auto-trait bounds differently from CI's pinned toolchain; a real but toolchain-cosmetic diff, not a genuine API change.
5. **`e2e_platform_api` (PRD 06 acceptance-1) is never run by any CI job** (PLAT-06/X-02) — the test binary requires `--features web-server`, which no `test`/`integration`/`feature-flags` job supplies; the test itself passes locally.

All five are precisely scoped, single-file-class fixes (a Lua script, a timestamp truncation, two smoke-script files + one config file, a baseline regeneration, and one CI YAML addition) — none require architectural rework. Every other must-have across the 18 plans (status machine, worker pool mechanics, cancellation, assistants/WarGraphDoc, schedules, webhook signing/SSRF, pagination, auth, docs) was verified directly against the codebase and, where live CI evidence exists, is green (Postgres assistant/schedule/webhook suites, waypoint suite all pass in the same CI run that surfaced these five gaps).

---

_Verified: 2026-09-08T12:10:00Z_
_Verifier: Claude (gsd-verifier)_
