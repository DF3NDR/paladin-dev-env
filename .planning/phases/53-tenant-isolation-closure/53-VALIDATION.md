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
| 53-xx-xx | — | — | TENANT-03 | — | foreign tenant 404 == missing on every `/v1/threads/{id}*` op; list omits hidden and zero-run threads | unit/HTTP | `cargo test -p paladin-web --lib thread_controller` | ❌ W0 | ⬜ pending |
| 53-xx-xx | — | — | TENANT-03 | — | `RunReadScope` thread-owner rule table | unit | `cargo test -p paladin-ai-core --lib principal` | ❌ W0 | ⬜ pending |
| 53-xx-xx | — | — | TENANT-04 | — | HTTP per route: foreign 404; owner/Admin accepted; fresh id accepted (service) | integration | `cargo test --lib --features web-server http_surface_tests` | ❌ W0 (HTTP-level) | ⬜ pending |
| 53-xx-xx | — | — | TENANT-05 | — | service cancel foreign → `NotFound`, flag false | unit | `cargo test --lib --features web-server cancel_of_another` | ✅ (evidence only) | ⬜ pending |
| 53-xx-xx | — | — | TENANT-06 | — | `Serialization` → 404 non-admin / fixed 500 Admin; no body carries injected text; unattributed run 404/200 | unit/HTTP | `cargo test -p paladin-web --lib run_controller` | ⚠ partial (Admin only) | ⬜ pending |
| 53-xx-xx | — | — | TENANT-07 | — | job matrix: owner 200, peer key 200, foreign 404, other agent id 404, Admin 200, bodies equal unknown-job; evicted 404 | unit | `cargo test -p paladin-web --lib job_store` / `agent_controller` | ❌ W0 | ⬜ pending |
| 53-xx-xx | — | — | TENANT-08 | — | refusal when Treasurer configured without engine factory; shipped wiring boots | unit/integration | `cargo test --lib --features web-server run_api_wiring` | ❌ W0 | ⬜ pending |
| 53-xx-xx | — | — | TENANT-09 | — | each reserved literal in each position rejected; case variants accepted; message names entry not key | unit | `cargo test --lib --features web-server config::agents` | ❌ W0 rows | ⬜ pending |
| 53-xx-xx | — | — | TENANT-10 | — | 0/1/many unattributed → 0/1/1 `warn!` with exact count + ascending ids; half-attributed → one `error!` + boot ok; no writes | unit | `cargo test --lib --features web-server run_api_wiring` | ❌ W0 | ⬜ pending |
| 53-xx-xx | — | — | SC5 | — | semver green all 12 packages, §11.2 empty, allowlist zero | CI/gate | `make check-migration-allowlist`; CI `Semver Checks (vs v0.11.0)` | CI only | ⬜ pending |

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
| Semver job line for all twelve packages on the phase PR head | SC5 | Produced by CI `Semver Checks (vs v0.11.0)`; local reproduction needs crates.io + a full build | Open the PR's `semver` job log; copy each package's `N checks: X pass, 0 fail` line into `53-VERSION-DECISION.md` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
