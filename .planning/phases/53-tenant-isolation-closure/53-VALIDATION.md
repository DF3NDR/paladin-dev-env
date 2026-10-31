---
phase: 53
slug: tenant-isolation-closure
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-31
validated: 2026-10-31
validated_at_head: 50f5ccfae
---

# Phase 53 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `53-RESEARCH.md` "Validation Architecture"; the per-task map is
> completed by the planner's PLAN.md `<verify>` blocks and sealed by `/gsd-validate-phase`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` / `#[tokio::test]`; axum `Router::oneshot` (tower `util`); `insta` available in root dev-deps |
| **Config file** | none — workspace `Cargo.toml`; coverage via `scripts/coverage.sh` |
| **Quick run command** | `cargo test -p paladin-web --lib thread_controller` (52 tests pass offline at phase head; 34 on the phase base) |
| **Full suite command** | `cargo test` then `make clean-code`, `make check-gates`, `make security`, `make api-surface`, `make check-migration-allowlist`; CI adds `Semver Checks (vs v0.11.0)` and `Coverage` |
| **Estimated runtime** | ~60 seconds quick (warm cache); ~15 minutes full workspace |

---

## Sampling Rate

- **After every task commit:** Run the quick command for the touched crate (`cargo test -p paladin-web --lib <module>` or `cargo test --lib --features web-server <module>`), plus `cargo fmt --check` and `cargo clippy -p <crate> -- -D warnings`
- **After every plan wave:** Run `cargo test` (workspace), `make clean-code`, `make check-gates`
- **Before `/gsd-verify-work`:** Full suite must be green; `make api-surface`, `make security`; CI semver line recorded in `53-VERSION-DECISION.md`; coverage ≥ 82 %
- **Max feedback latency:** 120 seconds for the quick command

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 53-03-01, 53-03-02, 53-03-03 | 53-03 | 2 | TENANT-03 | T-53-12, T-53-13, T-53-14 | foreign tenant 404 == missing on every `/v1/threads/{id}*` op (matrix from `versioned_thread_parts(..).1`) | unit/HTTP | `cargo test -p paladin-web --lib thread_controller` | ✅ `crates/paladin-web/src/thread_controller.rs` | ✅ green |
| 53-04-01, 53-04-02 | 53-04 | 3 | TENANT-03 | T-53-20, T-53-21 | list omits hidden and zero-run threads; bounded walk; cursor never repeats | unit/HTTP | `cargo test -p paladin-web --lib thread_controller::tests::list_threads_` | ✅ `crates/paladin-web/src/thread_controller.rs` | ✅ green |
| 53-01-01 | 53-01 | 1 | TENANT-03 | T-53-01 | `RunReadScope` thread-owner rule table | unit + doctest | `cargo test -p paladin-ai-core --lib principal` and `cargo test -p paladin-ai-core --doc permits_thread` | ✅ `crates/paladin-core/src/platform/container/principal.rs` | ✅ green |
| 53-01-01, 53-01-02 | 53-01 | 1 | TENANT-04 | T-53-01, T-53-02 | HTTP per route: foreign 404; owner/Admin accepted; fresh id accepted (service) | integration | `cargo test --lib --features web-server cross_tenant_` and `cargo test --lib --features web-server run::submission` | ✅ `src/application/services/run/http_surface_tests.rs`, `submission.rs` | ✅ green |
| 53-01-02 | 53-01 | 1 | TENANT-05 | T-53-03 | service cancel foreign → `NotFound`, flag false | unit + HTTP | `cargo test --lib --features web-server cancel_of_another` and `cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag` | ✅ `src/application/services/run/submission.rs`, `http_surface_tests.rs` | ✅ green |
| 53-02-01, 53-02-02, 53-03-03 | 53-02, 53-03 | 1, 2 | TENANT-06 | T-53-07, T-53-08, T-53-16 | `Serialization` → 404 non-admin / fixed 500 Admin; no body carries injected text; unattributed run 404/200 | unit/HTTP | `cargo test -p paladin-web --lib run_controller` and `thread_controller::tests::serialization_on_thread_routes_is_hidden_for_tenant_and_redacted_for_admin` | ✅ `crates/paladin-web/src/run_controller.rs`, `thread_controller.rs` | ✅ green |
| 53-05-01, 53-05-02 | 53-05 | 4 | TENANT-07 | T-53-25, T-53-26 | job matrix: owner 200, peer key 200, foreign 404, other agent id 404, Admin 200, bodies equal unknown-job; evicted 404 | unit | `cargo test -p paladin-web --lib job_store` / `agent_controller` | ✅ `crates/paladin-web/src/job_store.rs`, `agent_controller.rs` | ✅ green |
| 53-06-01 | 53-06 | 5 | TENANT-08 | T-53-30, T-53-31 | refusal when Treasurer configured without engine factory; shipped wiring boots | unit/integration | `cargo test --lib --features web-server -- treasurer_assertion pool_built_without_an_engine_factory shipped_wiring_with_a_treasurer build_run_api_wires_a_treasurer` | ✅ `src/infrastructure/web/run_api_wiring.rs` | ✅ green |
| 53-06-03 | 53-06 | 5 | TENANT-09 | T-53-33, T-53-34 | each reserved literal in each position rejected; case variants accepted; message names entry not key | unit | `cargo test --lib --features web-server config::agents` | ✅ `src/config/agents.rs` | ✅ green |
| 53-06-02 | 53-06 | 5 | TENANT-10 | T-53-34, T-53-35 | 0/1/many unattributed → 0/1/1 `warn!` with exact count + ascending ids; half-attributed → one `error!` + boot ok; no writes | unit/integration | `cargo test --lib --features web-server schedule_scan_` | ✅ `src/infrastructure/web/run_api_wiring.rs` | ✅ green |
| 53-07-01, 53-07-02, 53-07-03 | 53-07 | 6 | TENANT-03..10 (docs/ledger) | — | ADR-0063 indexed; user docs describe shipped behavior; WINDOWS rows 58/63 fixed; credential review recorded | gate (grep) | the three `<automated>` blocks in `53-07-PLAN.md` | ✅ ADR-0063, docs pages, `53-TENANT-EVIDENCE.md` | ✅ green |
| 53-08-01, 53-08-03 | 53-08 | 7 | SC5 | T-53-40, T-53-41 | semver green all 12 packages, §11.2 empty, allowlist zero | CI/gate | `make check-migration-allowlist`; `./scripts/check-semver.sh` (CI `Semver Checks (vs v0.11.0)`) | ✅ `53-VERSION-DECISION.md` | ✅ green (allowlist re-run; semver lines recorded on `4ef9ddd2a`, not re-run here) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/paladin-web/src/thread_controller.rs` tests — doubles: a run repository honoring `thread_id` with attributed runs and an injectable `Serialization`; a waypoint store whose `list_threads` honors `limit`/`before`
- [x] `crates/paladin-web/src/thread_controller.rs` tests — `published_thread_id_operations` + route-matrix test, list/walk tests
- [x] `src/application/services/run/http_surface_tests.rs` — HTTP-level foreign submit/fork tests
- [x] `src/infrastructure/web/run_api_wiring.rs` tests — capturing logger (or `ScanOutcome` split) for the schedule scan; Treasurer-wiring refusal (the `scan_report` formatter split was used)
- [x] Re-seed existing thread tests that assert pre-gate behavior (research Pitfalls 3-4)
- [x] Environment: disk freed 2026-10-31 (`target/debug` removed, 21 GB available) so root-crate `--features web-server` tests run locally again

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Semver job line for all twelve packages (fallback only) | SC5 | 53-08 Task 1 reproduces the CI job locally with `./scripts/check-semver.sh`; the 53-08 Task 2 checkpoint fires only if that run is UNAVAILABLE (crates.io or build). It did not fire: Task 1 recorded twelve lines | Open the PR's `Semver Checks (vs v0.11.0)` job log or its `::notice` annotation; copy each package's `semver: <pkg>: N checks: ...` line and the verdict into `53-VERSION-DECISION.md` |
| P5 / AC-N5: no configuration key or no-op factory lets a configured Treasurer boot without the engine-factory path | TENANT-08 (judgment-tier prohibition) | `has_engine_factory` reports only `Option::is_some`, so a no-op factory closure is invisible to any test. The SPEC assigns this to code review; the test-tier half of TENANT-08 (refusal without a factory) is automated above | Read `src/infrastructure/web/run_api_wiring.rs` around the pool construction and `paladin-server.rs` bind order; confirm no settings key disables `assert_treasurer_enforced` and the shipped path always passes a real factory. Tracked as test 1 in `53-UAT.md` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-31

---

## Validation Audit 2026-10-31

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

Audited at head `50f5ccfae`. All 52 tests named in the eight SUMMARY coverage blocks exist in the tree
and were run by this audit:

| Suite | Command | Result |
|-------|---------|--------|
| Core thread-owner rule | `cargo test -p paladin-ai-core --lib principal` | 10 passed |
| Core doctest | `cargo test -p paladin-ai-core --doc permits_thread` | 1 passed |
| Thread, run, job routes | `cargo test -p paladin-web --lib -- thread_controller run_controller agent_controller job_store` | 195 passed |
| OpenAPI baseline and golden | `cargo test -p paladin-web --lib openapi_matches_committed_baseline`; `--test openapi_golden_v0_9` | 1 + 9 passed; `openapi.json` unchanged |
| Submit/fork/cancel HTTP locks | `cargo test --lib --features web-server cross_tenant_` | 3 passed |
| Service submit/fork/cancel | `cargo test --lib --features web-server run::submission` | 35 passed |
| Reserved literals | `cargo test --lib --features web-server config::agents` | 29 passed |
| Treasurer assertion | `run_api_wiring` treasurer and engine-factory tests | 4 passed |
| Schedule scan | `cargo test --lib --features web-server schedule_scan_` | 6 passed |
| Register gate | `make check-migration-allowlist` | set-equal; 0 `[[entry]]` |
| 53-07 / 53-08 artifact gates | grep `<automated>` blocks from both plans | all pass |

The `run_api_wiring` module reports 37 passed and 2 failed. The two failures are
`build_run_api_persists_no_run_traces_by_default` and
`build_run_api_persists_run_traces_when_trace_persist_is_set` ("the run never reached a terminal
status"). Both were added in Phase 43 (`998ae11ee`), are not Phase 53 tests, and are tracked as
TESTS-01 for Phase 55. They are not counted as Phase 53 gaps.

Not re-run by this audit: `./scripts/check-semver.sh` (twelve-package build, disk-limited here) and
the 82 % coverage floor (`cargo-llvm-cov` not installed locally). Both remain CI evidence.
