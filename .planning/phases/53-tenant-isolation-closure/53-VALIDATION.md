---
phase: 53
slug: tenant-isolation-closure
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-31
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
| **Quick run command** | `cargo test -p paladin-web --lib thread_controller` (34 tests pass offline on the phase base) |
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
| 53-03-01, 53-03-02, 53-03-03 | 53-03 | 2 | TENANT-03 | T-53-12, T-53-13, T-53-14 | foreign tenant 404 == missing on every `/v1/threads/{id}*` op (matrix from `versioned_thread_parts(..).1`) | unit/HTTP | `cargo test -p paladin-web --lib thread_controller` | ❌ W0 | ⬜ pending |
| 53-04-01, 53-04-02 | 53-04 | 3 | TENANT-03 | T-53-20, T-53-21 | list omits hidden and zero-run threads; bounded walk; cursor never repeats | unit/HTTP | `cargo test -p paladin-web --lib thread_controller::tests::list_threads_` | ❌ W0 | ⬜ pending |
| 53-01-01 | 53-01 | 1 | TENANT-03 | T-53-01 | `RunReadScope` thread-owner rule table | unit | `cargo test -p paladin-ai-core --lib principal` | ❌ W0 | ⬜ pending |
| 53-01-01, 53-01-02 | 53-01 | 1 | TENANT-04 | T-53-01, T-53-02 | HTTP per route: foreign 404; owner/Admin accepted; fresh id accepted (service) | integration | `cargo test --lib --features web-server http_surface_tests` | ❌ W0 (HTTP-level) | ⬜ pending |
| 53-01-02 | 53-01 | 1 | TENANT-05 | T-53-03 | service cancel foreign → `NotFound`, flag false | unit | `cargo test --lib --features web-server cancel_of_another` | ✅ (evidence only) | ⬜ pending |
| 53-02-01, 53-02-02, 53-03-03 | 53-02, 53-03 | 1, 2 | TENANT-06 | T-53-07, T-53-08, T-53-16 | `Serialization` → 404 non-admin / fixed 500 Admin; no body carries injected text; unattributed run 404/200 | unit/HTTP | `cargo test -p paladin-web --lib run_controller` | ⚠ partial (Admin only) | ⬜ pending |
| 53-05-01, 53-05-02 | 53-05 | 4 | TENANT-07 | T-53-25, T-53-26 | job matrix: owner 200, peer key 200, foreign 404, other agent id 404, Admin 200, bodies equal unknown-job; evicted 404 | unit | `cargo test -p paladin-web --lib job_store` / `agent_controller` | ❌ W0 | ⬜ pending |
| 53-06-01 | 53-06 | 5 | TENANT-08 | T-53-30, T-53-31 | refusal when Treasurer configured without engine factory; shipped wiring boots | unit/integration | `cargo test --lib --features web-server run_api_wiring` | ❌ W0 | ⬜ pending |
| 53-06-03 | 53-06 | 5 | TENANT-09 | T-53-33, T-53-34 | each reserved literal in each position rejected; case variants accepted; message names entry not key | unit | `cargo test --lib --features web-server config::agents` | ❌ W0 rows | ⬜ pending |
| 53-06-02 | 53-06 | 5 | TENANT-10 | T-53-34, T-53-35 | 0/1/many unattributed → 0/1/1 `warn!` with exact count + ascending ids; half-attributed → one `error!` + boot ok; no writes | unit | `cargo test --lib --features web-server run_api_wiring` | ❌ W0 | ⬜ pending |
| 53-08-01, 53-08-03 | 53-08 | 7 | SC5 | T-53-40, T-53-41 | semver green all 12 packages, §11.2 empty, allowlist zero | CI/gate | `make check-migration-allowlist`; CI `Semver Checks (vs v0.11.0)` | CI only | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/paladin-web/src/thread_controller.rs` tests — doubles: a run repository honoring `thread_id` with attributed runs and an injectable `Serialization`; a waypoint store whose `list_threads` honors `limit`/`before`
- [ ] `crates/paladin-web/src/thread_controller.rs` tests — `published_thread_id_operations` + route-matrix test, list/walk tests
- [ ] `src/application/services/run/http_surface_tests.rs` — HTTP-level foreign submit/fork tests
- [ ] `src/infrastructure/web/run_api_wiring.rs` tests — capturing logger (or `ScanOutcome` split) for the schedule scan; Treasurer-wiring refusal
- [ ] Re-seed existing thread tests that assert pre-gate behavior (research Pitfalls 3-4)
- [ ] Environment: disk freed 2026-10-31 (`target/debug` removed, 21 GB available) so root-crate `--features web-server` tests run locally again

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Semver job line for all twelve packages (fallback only) | SC5 | 53-08 Task 1 reproduces the CI job locally with `./scripts/check-semver.sh`; the 53-08 Task 2 checkpoint fires only if that run is UNAVAILABLE (crates.io or build) | Open the PR's `Semver Checks (vs v0.11.0)` job log or its `::notice` annotation; copy each package's `semver: <pkg>: N checks: ...` line and the verdict into `53-VERSION-DECISION.md` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
