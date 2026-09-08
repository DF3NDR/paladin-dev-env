---
phase: 27
slug: platform-api
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-08
---

# Phase 27 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from `27-RESEARCH.md` § Validation Architecture. Task IDs are filled by
> `/gsd-validate-phase` once PLAN.md files exist.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (with `#[tokio::test]` for async), `cargo llvm-cov` for coverage |
| **Config file** | none dedicated — `scripts/coverage.sh` is the single source of truth for the coverage invocation, shared by `make coverage` and CI's `coverage` job |
| **Quick run command** | `cargo test -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-storage -p paladin-web` (the core crate's *package* name is `paladin-ai-core`; `-p paladin-core` exits 101) |
| **Full suite command** | `cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82 -- --test-threads=1` (CI's invocation; `scripts/coverage.sh` wraps the same command but **hard-fails locally** when Redis/MinIO are unreachable, so it is a CI/UAT command, not a devcontainer one) |
| **Estimated runtime** | ~180 seconds (Tier 1, no Docker services) |

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
not evidence. Every automated command below must be shown to select at least one test.

---

## Per-Task Verification Map

Task IDs are assigned when plans are written; rows are keyed by requirement until then.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 1 | PLAT-01 | — | Monotonic status machine; no edge leaves a terminal | unit | `cargo test -p paladin-ai-core run_status_transition` | ❌ W0 | ⬜ pending |
| TBD | TBD | 1 | PLAT-01 | — | CAS transition rejects illegal `from` state | integration | `cargo test -p paladin-storage --features sqlite run_repository_contract` | ❌ W0 | ⬜ pending |
| TBD | TBD | 1 | PLAT-02 | — | Lease-expiry redelivery to a second worker | integration | `cargo test -p paladin-storage run_queue_contract` | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | PLAT-02 | — | Exactly one completion after worker death; no node re-executes past the interrupted superstep | `#[tokio::test(flavor = "multi_thread")]` | `cargo test worker_pool_lease_expiry_exactly_once` | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | PLAT-02 | T-27-01 | 10 concurrent submits to one thread → exactly 1 accepted, 9 × `409 ThreadBusy` | `#[tokio::test(flavor = "multi_thread")]` | `cargo test ten_concurrent_submits_one_accepted` | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | PLAT-02 | — | Cross-instance cancel observed at a superstep boundary → `Halted` waypoint, `Cancelled` run | integration | `cargo test cross_instance_cancel_probe` | ❌ W0 | ⬜ pending |
| TBD | TBD | 3 | PLAT-03 | — | `AwaitingInput` acks; queue depth returns to 0 | integration | `cargo test awaiting_input_acks_queue` | ❌ W0 | ⬜ pending |
| TBD | TBD | 3 | PLAT-03 | — | Resume re-enqueues the same `run_id`, `attempt++` | integration | `cargo test resume_reenqueues_same_run_id` | ❌ W0 | ⬜ pending |
| TBD | TBD | 3 | PLAT-03 | — | SSE live + degraded mode; 15 s heartbeat; `done`/`error` always delivered | controller (`tower::oneshot`) | `cargo test -p paladin-web run_stream_sse` | ❌ W0 | ⬜ pending |
| TBD | TBD | 4 | PLAT-04 | — | Version immutability: no `update_version`, no `PUT` route registered | unit | `cargo test assistant_version_immutable` | ❌ W0 | ⬜ pending |
| TBD | TBD | 4 | PLAT-04 | — | `latest` frozen at submit under a concurrent publish | `#[tokio::test(flavor = "multi_thread")]` | `cargo test assistant_version_freeze_at_submit` | ❌ W0 | ⬜ pending |
| TBD | TBD | 4 | PLAT-04 | — | `WarGraphDoc` → compile → fingerprint stable across a two-process boundary | integration | `cargo test -p paladin-battalion wargraph_doc_fingerprint_two_process` | ❌ W0 | ⬜ pending |
| TBD | TBD | 4 | PLAT-04 | — | Unsupported node kind → typed `CompileError`, never a silent drop (D-33 scope correction) | unit | `cargo test -p paladin-battalion wargraph_doc_unsupported_node_kind` | ❌ W0 | ⬜ pending |
| TBD | TBD | 5 | PLAT-05 | — | Scheduler restarted across a tick boundary → exactly once per `on_missed` policy | `#[tokio::test(flavor = "multi_thread")]`, `tokio::time::pause` | `cargo test schedule_restart_exactly_once` | ❌ W0 | ⬜ pending |
| TBD | TBD | 5 | PLAT-05 | T-27-02 | HMAC signature verifies against the raw captured body | unit + `mockito` | `cargo test webhook_signature` | ❌ W0 | ⬜ pending |
| TBD | TBD | 5 | PLAT-05 | T-27-03 | SSRF table: non-http(s)/loopback/link-local/private/metadata rejected at write **and** send | unit | `cargo test webhook_ssrf_guard` | ❌ W0 | ⬜ pending |
| TBD | TBD | 5 | PLAT-05 | T-27-03 | Webhook client follows no redirects (D-42) | unit | `cargo test webhook_client_no_redirects` | ❌ W0 | ⬜ pending |
| TBD | TBD | 5 | PLAT-05 | — | Retry: 5 attempts 1 s…60 s on 5xx/timeout only; 4xx dead-letters | integration, paused clock | `cargo test webhook_retry_schedule` | ❌ W0 | ⬜ pending |
| TBD | TBD | 6 | PLAT-06 | T-27-04 | Auth + rate limiting + scope enforcement on every new endpoint | controller | `cargo test -p paladin-web run_controller_auth` | ❌ W0 | ⬜ pending |
| TBD | TBD | 6 | PLAT-06 | — | Pagination: `limit ≤ 100`, opaque cursor, `next_cursor` on every list route | controller | `cargo test -p paladin-web pagination` | ❌ W0 | ⬜ pending |
| TBD | TBD | 6 | PLAT-06 | — | `openapi.json` matches the committed baseline | unit | `cargo test -p paladin-web --lib openapi` | ✅ existing | ⬜ pending |
| TBD | TBD | 6 | PLAT-06 | — | Generated Python + TS clients compile and smoke-pass | CI job | new `sdk-clients` workflow job | ❌ W0 | ⬜ pending |
| TBD | TBD | 7 | PLAT-01…06 | — | Full lifecycle (PRD 06 acceptance 1): assistant → run → SSE → `AwaitingInput` → webhook → resume → complete → history/fork | E2E integration | `cargo test --test e2e_platform_api` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/paladin-core/src/platform/container/run.rs` (package `paladin-ai-core`) + its `#[cfg(test)]` status-machine tests — the phase's literal first test (PRD 06 §5 item 1)
- [ ] `crates/paladin-storage/src/run/contract_tests.rs` — new shared contract suite, mirroring `waypoint/contract_tests.rs`
- [ ] `crates/paladin-storage/src/run_queue/{in_memory,redis}.rs` + its contract suite — **no existing Lua/`EVAL` fixture to extend** (D-08 is greenfield)
- [ ] A facade-crate test module for `RunWorkerPool` / `ScheduleService` / `WebhookDeliveryService` stress tests, following the X-05 exact-count + timeout-guard house pattern in `src/application/services/orchestration/listener.rs`
- [ ] `crates/paladin-battalion/tests/fixtures/graph_docs/` — the example corpus, plus the `schemars`-derived golden schema test (D-34 revised; `schemars 1.2` is already a direct dependency, `Cargo.toml:143`)
- [ ] New CI jobs: a Redis-queue integration job, and the `sdk-clients` generation/smoke job

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `POST /runs` p99 ≤ 250 ms under nominal load | PLAT-01 | A wall-clock latency assertion is flaky in CI. The *architectural* claim — no engine work on the request path — is what gets asserted automatically (the handler performs one insert and one enqueue and touches no engine type); the timing figure is confirmed once by hand under UAT. | Run the test server with the SQLite+InMemory profile, submit 200 runs, record the p99 of the enqueue-only path. |
| Redis queue contract suite + worker-death redelivery on Redis | PLAT-02 | Docker is unavailable in the devcontainer (D-51). | CI's Redis job, or a UAT run with `make services-up`. |
| Postgres run/schedule repository contract suites | PLAT-01, PLAT-05 | Same — Docker unavailable locally. | CI's `postgres-integration` job. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] Every automated command demonstrably selects ≥ 1 test (a zero-match filter exits 0)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
