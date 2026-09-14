---
phase: 27
slug: platform-api
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-08
validated: 2026-09-08
---

# Phase 27 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from `27-RESEARCH.md` § Validation Architecture. Task IDs, commands and statuses
> below were reconciled against the implemented tree by `/gsd-validate-phase` on 2026-09-08 —
> see § Validation Audit for what the seeded draft got wrong.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (with `#[tokio::test]` for async), `cargo llvm-cov` for coverage |
| **Config file** | none dedicated — `scripts/coverage.sh` is the single source of truth for the coverage invocation, shared by `make coverage` and CI's `coverage` job |
| **Quick run command** | `cargo test -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-storage -p paladin-web` (the core crate's *package* name is `paladin-ai-core`; `-p paladin-core` exits 101) |
| **Full suite command** | `cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82 -- --test-threads=1` (CI's invocation; `scripts/coverage.sh` wraps the same command but **hard-fails locally** when Redis/MinIO are unreachable, so it is a CI/UAT command, not a devcontainer one) |
| **Estimated runtime** | ~180 seconds (Tier 1, no Docker services) |

### Package-scoping rules that decide whether a filter selects anything

The workspace root is itself a package (`paladin-ai`), so a bare `cargo test <filter>` selects
**only root-package tests** — it silently selects zero in every other crate. Three of the four
zero-selecting commands in the seeded draft failed exactly this way. When recording a command:

- Facade tests (`src/application/services/**`) → `cargo test --features web-server <filter>`
- Crate tests → `cargo test -p <crate> <filter>`, plus `--features sqlite` for `paladin-storage`
  adapter suites (the SQLite adapter modules are feature-gated and select zero without it)
- Always confirm the filter selects ≥ 1 test with `-- --list` before recording it

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate touched by the task>` (Tier 1 only — no Docker required)
- **After every plan wave:** Run `cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82 -- --test-threads=1` (not `scripts/coverage.sh`, which hard-fails without Redis/MinIO)
- **Before `/gsd-verify-work`:** Full CI green — `coverage`, `msrv` (**1.88**), `semver`, `postgres-integration`, and the new Redis-queue job
- **Max feedback latency:** 180 seconds

**Tier rule (D-51).** Docker is unavailable in this devcontainer. A locally unreachable Redis or
Postgres makes the suite fall back through the InMemory/SQLite Tier-1 paths — that is **not** a
failure and must never be recorded as one. Tier-2 evidence comes from CI/UAT only.

**Two verification traps that apply to every row below.** Doc tests do not count toward
`cargo llvm-cov` and are skipped by `--tests`, so the 82% floor cannot lean on them; and a test
filter that selects zero tests still exits `0`, so a green command whose filter matched nothing is
not evidence. Every automated command below has been shown to select at least one test, with the
observed count recorded in the Selected column.

---

## Per-Task Verification Map

Every command below was executed on 2026-09-08 against the implemented tree. **Selected** is the
observed passing-test count for that exact filter — the anti-false-green receipt.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Selected | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|----------|-------------|--------|
| 27-01-T2 | 27-01 | 1 | PLAT-01 | — | Monotonic status machine; no edge leaves a terminal | unit | `cargo test -p paladin-ai-core --lib platform::container::run::tests` | 20 | ✅ `crates/paladin-core/src/platform/container/run.rs` | ✅ green |
| 27-02-T2 | 27-02 | 2 | PLAT-01 | T-27-02-01 | CAS transition rejects illegal `from` state | integration | `cargo test -p paladin-storage --features sqlite --lib run::sqlite` | 19 | ✅ `crates/paladin-storage/src/run/{contract_tests,sqlite}.rs` | ✅ green |
| 27-03-T1 | 27-03 | 2 | PLAT-02 | — | Lease-expiry redelivery to a second worker | integration | `cargo test -p paladin-storage --lib run_queue::` | 12 | ✅ `crates/paladin-storage/src/run_queue/contract_tests.rs` | ✅ green |
| 27-04-T2 | 27-04 | 2 | PLAT-02 | — | Exactly one completion after worker death; no node re-executes past the interrupted superstep | `#[tokio::test(flavor = "multi_thread")]` | `cargo test --features web-server worker_pool_lease_expiry_exactly_once` | 1 | ✅ `src/application/services/run/worker_tests.rs` | ✅ green |
| 27-15-T1 | 27-15 | 7 | PLAT-02 | T-27-02-02 | 10 concurrent submits to one thread → exactly 1 accepted, 9 × `409 ThreadBusy` | `#[tokio::test(flavor = "multi_thread")]` | `cargo test --features web-server ten_concurrent_submits_one_accepted` | 1 | ✅ `src/application/services/run/http_surface_tests.rs` | ✅ green |
| 27-07-T2 | 27-07 | 3 | PLAT-02 | — | Cross-instance cancel observed at a superstep boundary → `Halted` waypoint, `Cancelled` run | integration | `cargo test --features web-server cross_instance_cancel_probe` | 1 | ✅ `src/application/services/run/cancel_tests.rs` | ✅ green |
| 27-04-T2 | 27-04 | 2 | PLAT-03 | — | `AwaitingInput` acks; queue depth returns to 0 | integration | `cargo test --features web-server awaiting_input_acks_queue` | 1 | ✅ `src/application/services/run/worker_tests.rs` | ✅ green |
| 27-08-T1 | 27-08 | 3 | PLAT-03 | — | Resume re-enqueues the same `run_id`, `attempt++` | integration | `cargo test --features web-server resume_reenqueues_same_run_id` | 1 | ✅ `src/application/services/parley/adapter.rs` | ✅ green |
| 27-10-T2 | 27-10 | 4 | PLAT-03 | — | SSE framing, 15 s heartbeat, `501` when unwired, `404` on unknown run | controller (`tower::oneshot`) | `cargo test -p paladin-web run_stream` | 4 | ✅ `crates/paladin-web/src/run_controller.rs` | ✅ green |
| 27-10-T1 | 27-10 | 4 | PLAT-03 | — | Live bus path **and** degraded polling path; `done` always delivered; state-delta leaks field names only | integration | `cargo test --features web-server run::stream_tests` | 5 | ✅ `src/application/services/run/stream_tests.rs` | ✅ green |
| 27-12-T1 | 27-12 | 5 | PLAT-04 | — | Version immutability: no `update_version` on the admin port | unit | `cargo test --features web-server assistant_version_immutable` | 1 | ✅ `src/application/services/assistant/tests.rs` | ✅ green |
| 27-12-T2 | 27-12 | 5 | PLAT-04 | — | No `PUT`/`PATCH` route is registered for a version (the routing half of immutability) | controller | `cargo test -p paladin-web no_put_or_patch` | 1 | ✅ `crates/paladin-web/src/assistant_controller.rs` | ✅ green |
| 27-09-T1 | 27-09 | 4 | PLAT-04 | — | `latest` frozen at submit under a concurrent publish | `#[tokio::test]` (contract suite) | `cargo test -p paladin-storage --features sqlite assistant_version_freeze_at_submit` | 2 | ✅ `crates/paladin-storage/src/run/contract_tests.rs` | ✅ green |
| 27-05-T2 | 27-05 | 2 | PLAT-04 | — | `WarGraphDoc` → compile → fingerprint stable across a two-process boundary | integration | `cargo test -p paladin-battalion wargraph_doc_fingerprint_two_process` | 1 | ✅ `crates/paladin-battalion/tests/graph_doc_round_trip.rs` | ✅ green |
| 27-05-T2 | 27-05 | 2 | PLAT-04 | — | Unsupported node kind → typed `CompileError`, never a silent drop (D-33 scope correction) | unit | `cargo test -p paladin-battalion wargraph_doc_unsupported_node_kind` | 2 | ✅ `crates/paladin-battalion/tests/graph_doc_round_trip.rs` | ✅ green |
| 27-11-T2 | 27-11 | 5 | PLAT-05 | — | Scheduler restarted across a tick boundary → exactly once per `on_missed` policy | `#[tokio::test(flavor = "multi_thread")]`, `tokio::time::pause` | `cargo test --features web-server schedule_restart_exactly_once` | 3 | ✅ `src/application/services/run/schedule/tests.rs` | ✅ green |
| 27-13-T1 | 27-13 | 6 | PLAT-05 | T-27-02 | HMAC signature verifies against the raw captured body (signed once, sent verbatim) | unit + `mockito` | `cargo test --features web-server webhook_signature` | 6 | ✅ `src/application/services/run/webhook/` | ✅ green |
| 27-13-T1/T2 | 27-13 | 6 | PLAT-05 | T-27-03 | SSRF table (non-http(s)/loopback/link-local/private/ULA/metadata) enforced at **write** time (submit + schedule create) **and** at **send** time (dead-letters) | unit + integration | `cargo test --features web-server ssrf` | 6 | ✅ `src/application/services/run/webhook/ssrf.rs`, `submission.rs`, `schedule/tests.rs` | ✅ green |
| 27-13-T2 | 27-13 | 6 | PLAT-05 | T-27-03 | Submit-time rejection touches nothing (no run persisted) | integration | `cargo test --features web-server submit_with_a_loopback_webhook_url_is_rejected` | 1 | ✅ `src/application/services/run/submission.rs` | ✅ green |
| 27-13-T1 | 27-13 | 6 | PLAT-05 | T-27-03 | Webhook client follows no redirects (D-42) | unit | `cargo test --features web-server webhook_client_no_redirects` | 1 | ✅ `src/application/services/run/webhook/client.rs` | ✅ green |
| 27-13-T2 | 27-13 | 6 | PLAT-05 | — | Retry: 5 attempts 1 s…60 s on 5xx/timeout only; 4xx dead-letters | integration, paused clock | `cargo test --features web-server webhook_retry_schedule` | 1 | ✅ `src/application/services/run/webhook/tests.rs` | ✅ green |
| 27-22-T1 | 27-22 | 1 (gap) | PLAT-05 | CR-01 | Webhook response body is capped at 64 KiB **before** truncation/redaction (bounded memory) | unit | `cargo test --features web-server bounded_body_` | 3 | ✅ `src/application/services/run/webhook/client.rs` | ✅ green |
| 27-15-T1 | 27-15 | 7 | PLAT-06 | T-27-04 | Auth + rate limiting + scope enforcement on every new endpoint | controller | `cargo test -p paladin-web run_controller_auth` | 5 | ✅ `crates/paladin-web/src/run_controller.rs` | ✅ green |
| 27-15-T1 | 27-15 | 7 | PLAT-06 | — | Pagination: `limit ≤ 100`, opaque base64url cursor, cursor errors never echo raw input | controller | `cargo test -p paladin-web pagination` | 10 | ✅ `crates/paladin-web/src/pagination.rs` | ✅ green |
| 27-18-T2 | 27-18 | 9 | PLAT-06 | — | `openapi.json` matches the committed baseline | unit | `cargo test -p paladin-web --lib openapi` | 12 | ✅ `crates/paladin-web/openapi.json` | ✅ green |
| 27-21-T1..T3 | 27-18, 27-21 | 9 / 1 (gap) | PLAT-06 | — | Generated Python + TS clients compile and reach terminal status `completed` against a hermetic mock-LLM server | CI job | `sdk-clients` job (`ci.yml:1077`) — **Tier 2, CI-only** (no Java/Docker locally) | CI | ✅ `.github/workflows/ci.yml`, `scripts/sdk-smoke/` | ✅ green (CI job 102125436564) |
| 27-24-T1/T2 | 27-24 | 2 (gap) | PLAT-06 | — | Public API surface unchanged, independent of toolchain auto-trait bound ordering | CI job + script | `./scripts/check-api-surface.sh .project/current-exports.txt` (`api-surface` job, `ci.yml:193`) | 1 | ✅ `scripts/normalize-api-bounds.py` | ✅ green (CI job 102125436884) |
| 27-19-T2, 27-26-T1 | 27-19, 27-26 | 1 (gap) | PLAT-02 | — | Redis claim-marker distinguishes a first claim from a lease-expiry reclaim; `run_all` provisions a fresh queue per clause | unit guard (Tier 1) + `redis-queue` job (Tier 2) | `cargo test -p paladin-storage --features redis-queue --lib claim_marker` | 3 | ✅ `crates/paladin-storage/src/run_queue/redis.rs` | ✅ green (Tier 2: CI job 102125436850, 19 passed) |
| 27-20-T1/T2 | 27-20 | 1 (gap) | PLAT-01 | — | Timestamps round-trip at microsecond resolution; sub-microsecond digits truncate toward zero | integration | `cargo test -p paladin-storage --features sqlite --lib run::` | 46 | ✅ `crates/paladin-storage/src/run/` | ✅ green (Tier 2: CI job 102125436566, 87 passed) |
| 27-18-T1, 27-24-T3 | 27-18, 27-24 | 9 / 2 (gap) | PLAT-01…06 | — | Full lifecycle (PRD 06 acceptance 1): assistant → run → SSE → `AwaitingInput` → webhook → resume → complete → history/fork | E2E integration | `cargo test --features web-server --test e2e_platform_api` | 1 | ✅ `tests/e2e_platform_api.rs` | ✅ green (CI job 102125436985) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/paladin-core/src/platform/container/run.rs` (package `paladin-ai-core`) + its `#[cfg(test)]` status-machine tests — the phase's literal first test (PRD 06 §5 item 1) → 20 tests green
- [x] `crates/paladin-storage/src/run/contract_tests.rs` — shared contract suite, mirroring `waypoint/contract_tests.rs`; 19 `pub async fn` clauses fanned out across InMemory/SQLite/Postgres
- [x] `crates/paladin-storage/src/run_queue/{in_memory,redis}.rs` + its contract suite (D-08 greenfield) → 12 Tier-1 clauses green; Redis Tier-2 proven on CI
- [x] Facade-crate test modules for `RunWorkerPool` / `ScheduleService` / `WebhookDeliveryService` → `worker_tests.rs`, `cancel_tests.rs`, `stream_tests.rs`, `http_surface_tests.rs`, `schedule/tests.rs`, `webhook/tests.rs`
- [x] `crates/paladin-battalion/tests/fixtures/graph_docs/` — example corpus (`approval_gate.json`, `nested_workflow.json`, `two_paladins_custom_edge.json`) plus the `schemars`-derived golden schema test (D-34 revised)
- [x] New CI jobs: `redis-queue` (`ci.yml:1003`), `sdk-clients` (`ci.yml:1077`), `e2e-platform-api` (`ci.yml:1187`), `api-surface` (`ci.yml:193`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `POST /runs` p99 ≤ 250 ms under nominal load | PLAT-01 | A wall-clock latency assertion is flaky in CI. The *architectural* claim — no engine work on the request path — is what gets asserted automatically (the handler performs one insert and one enqueue and touches no engine type); the timing figure is confirmed once by hand under UAT. | Run the test server with the SQLite+InMemory profile, submit 200 runs, record the p99 of the enqueue-only path. |
| Redis queue contract suite + worker-death redelivery on Redis | PLAT-02 | Docker is unavailable in the devcontainer (D-51). | CI's `redis-queue` job, or a UAT run with `make services-up`. Confirmed: CI job 102125436850, 19 passed, log asserts *"All run_queue::redis tests exercised the live server"* (the self-skip path was not taken). |
| Postgres run/schedule repository contract suites | PLAT-01, PLAT-05 | Same — Docker unavailable locally. | CI's `postgres-integration` job. Confirmed: CI job 102125436566, 87 passed. |
| Generated SDK client smoke (Python + TypeScript) | PLAT-06 | Requires Java + Docker for `openapi-generator-cli`; unavailable in the devcontainer. | CI's `sdk-clients` job. Confirmed: CI job 102125436564, both clients reach terminal status `completed`. |

### Accepted limitations (not validation gaps)

Carried from `27-VERIFICATION.md` — deliberate, documented scope decisions, each tracked in
`WINDOWS.md`, not missing coverage:

- **WR-02** — `Agent`-kind runs emit no SSE events or webhook deliveries (WINDOWS.md row 31).
  Pinned by a deliberate test: `agent_kind_run_with_a_webhook_enqueues_no_delivery`.
- **WR-03** — the run read model (`GET /runs`, `/runs/{id}`, `/runs/{id}/webhook-deliveries`) has
  no per-caller/tenant scoping; v0.10 assumes a single-tenant, mutually-trusted-principal
  deployment (WINDOWS.md row 32).
- **DNS rebinding** — the SSRF guard does not pin the resolved address between check and connect.
  Documented in `webhook/ssrf.rs`'s module docs and `security.instructions.md`; resolve-then-connect
  pinning is out of scope for this phase.

---

## Validation Audit 2026-09-08

| Metric | Count |
|--------|-------|
| Rows audited | 23 (seeded) → 30 (reconciled) |
| Gaps found | 7 |
| Resolved | 7 |
| Escalated | 0 |
| Tests generated | 0 — all behaviors were already covered |

**What the seeded draft got wrong.** Every requirement in this phase already had real, passing
tests. All seven gaps were defects in this document's own recorded commands, not in coverage:

*Zero-selecting filters (green, but selected nothing — the false-green trap this file warns about):*

| Requirement | Seeded command | Selected | Corrected command | Selected |
|---|---|---|---|---|
| PLAT-01 status machine | `cargo test -p paladin-ai-core run_status_transition` | **0** | `… --lib platform::container::run::tests` | 20 |
| PLAT-01 CAS | `… --features sqlite run_repository_contract` | **0** | `… --features sqlite --lib run::sqlite` | 19 |
| PLAT-02 queue | `cargo test -p paladin-storage run_queue_contract` | **0** | `… --lib run_queue::` | 12 |
| PLAT-04 freeze at submit | `cargo test assistant_version_freeze_at_submit` | **0** | `-p paladin-storage --features sqlite …` | 2 |

The first three named functions/modules that **do not exist anywhere in the tree** — the draft
invented plausible test names before the tests were written and they were never reconciled. The
fourth existed but was unreachable from the root package without `-p` and `--features sqlite`.

*Under-selecting filters (green, but covering only part of the row's own stated claim):*

| Requirement | Claim | Seeded command missed | Fix |
|---|---|---|---|
| PLAT-03 SSE | "live + degraded, 15 s heartbeat, `done`/`error` always delivered" | Selected 1 of 9; both degraded-path tests and the heartbeat-interval test were outside the filter | Split into two rows: `-p paladin-web run_stream` (4) + `--features web-server run::stream_tests` (5) |
| PLAT-04 immutability | "no `update_version`, **no `PUT` route registered**" | The routing half (`no_put_or_patch_route_exists`, in `paladin-web`) was never selected | Split into two rows: service-level (1) + controller-level (1) |
| PLAT-05 SSRF | "rejected at write **and** send" | `webhook_ssrf_guard` selects only the guard's own unit tests — neither call-site test | Broadened to `ssrf` (6), plus a dedicated submit-time row (1) |

**Method.** Each command was executed against the implemented tree; `-- --list` confirmed the
selected set before the pass/fail count was recorded. Counts appear in the Selected column so a
future regression from 20 → 0 is visible rather than silent.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 180s
- [x] Every automated command demonstrably selects ≥ 1 test (a zero-match filter exits 0) — observed counts recorded per row
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-09-08 by `/gsd-validate-phase` — 30 rows, 0 escalations, 0 tests generated (coverage was already complete; the audit corrected 7 defective commands).
