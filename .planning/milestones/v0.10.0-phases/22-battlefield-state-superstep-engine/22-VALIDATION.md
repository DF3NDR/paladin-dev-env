---
phase: 22
slug: battlefield-state-superstep-engine
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-01
---

# Phase 22 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in), `#[tokio::test]` / `#[tokio::test(flavor = "multi_thread")]`, `criterion` 0.5 for benches |
| **Config file** | none dedicated — workspace `Cargo.toml` + per-crate `[dev-dependencies]` |
| **Quick run command** | `cargo test -p paladin-core --lib` / `cargo test -p paladin-battalion --lib` / `cargo test -p paladin-storage --lib` (Tier 1, no services; pick the crate the task touched) |
| **Full suite command** | `make test-all` (unit + integration); `make test-integration-docker` for Tier 2 Postgres contract tests |
| **Estimated runtime** | ~60–180 seconds (Tier 1 per-crate); minutes for full workspace + Docker tier |

---

## Sampling Rate

- **After every task commit:** Run the touched crate's quick command (`cargo test -p <crate> --lib`), plus `cargo fmt --check` and `cargo clippy -- -D warnings`
- **After every plan wave:** Run `cargo test` (full workspace) + `cargo test --doc`
- **Before `/gsd-verify-work`:** Full suite green — `make test-all`, plus `make test-integration-docker` for the Postgres contract suite, plus the new `semver` and `msrv` CI jobs green
- **Max feedback latency:** ~180 seconds

---

## Per-Task Verification Map

*(Seeded from research; task IDs to be finalized by the planner.)*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | ENG-01 | — | N/A | unit | `cargo test -p paladin-core battlefield::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-02 | — | N/A | unit + multi_thread | `cargo test -p paladin-battalion engine::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-03 | — | N/A | unit + contract | `cargo test -p paladin-storage waypoint_contract::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-04 | — | N/A | integration | `cargo test --test e2e_crash_resume` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-05 | — | N/A | contract (Tier 1 InMemory/SQLite; Tier 2 Postgres) | `cargo test -p paladin-storage waypoint_contract::` / `make test-integration-docker` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-06 | — | N/A | integration (golden) | `cargo test --test golden_bridge_equivalence` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-07 | — | N/A | unit | `cargo test -p paladin-battalion engine::hooks::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | ENG-08 | — | N/A | CI | `cargo semver-checks check-release --baseline-version 0.9.0`; MSRV job in CI | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/paladin-core/src/platform/container/battlefield.rs` + `#[cfg(test)] mod tests` — covers ENG-01
- [ ] `crates/paladin-core/src/platform/container/waypoint.rs` + `#[cfg(test)] mod tests` — covers ENG-03 type shapes
- [ ] `crates/paladin-ports/src/output/waypoint_port.rs` — covers ENG-03 port contract
- [ ] `crates/paladin-storage/src/waypoint/contract_tests.rs` (shared generic test fns, D-09) — covers ENG-05
- [ ] `crates/paladin-battalion/src/engine/mod.rs` + `#[cfg(test)] mod tests` — covers ENG-02
- [ ] `tests/e2e_crash_resume.rs` (or under `tests/integration/`) — covers ENG-04 / E2E-1
- [ ] `tests/golden_bridge_equivalence.rs` — covers ENG-06
- [ ] `crates/paladin-storage/migrations/00X_create_waypoints_table.sql` — schema for ENG-05 SQLite
- [ ] `.github/workflows/ci.yml` `semver` and `msrv` jobs — covers ENG-08
- [ ] Framework install: none — `cargo test` / `criterion` already fully set up; only new test *files*, not new test *tooling*, are needed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| MIGRATION.md §9 skeleton completeness (pre-populated M-B-01…03 and §9.2 rows present, TBD only in later-epic sections) | ENG-08 | Document-structure review, not executable | Open `MIGRATION.md`; check §9.1–§9.8 headings exist; check M-B-01…03 rows and every §9.2 register row from overview §9.2 are present |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
