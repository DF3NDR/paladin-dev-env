---
phase: 48
slug: base-entity-rename
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-13
---

# Phase 48 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `48-RESEARCH.md` § Validation Architecture; the per-task map is
> filled in as PLAN.md files land and refined by `/gsd-validate-phase 48`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest via `cargo test` (no nextest); doctests via `cargo test --doc`; shell guards in `scripts/` |
| **Config file** | none per-crate; root `Cargo.toml` auto-discovers `tests/*.rs`; `crates/paladin-core/tests/` is auto-discovered once created |
| **Quick run command** | `cargo test -p paladin-ai-core && cargo test -p paladin-ai --test legacy_removal_guard --test entity_rename_guard` |
| **Full suite command** | `cargo check --workspace --all-targets --all-features && cargo test --workspace && cargo test --workspace --doc` |
| **Estimated runtime** | ~20 s warm (quick) / several minutes (full; 6,869 tests on the rehearsal tree, 2 known sandbox-baseline failures in `run_api_wiring`) |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core` plus both guard tests
- **After every plan wave:** Run the full suite command plus `make clean-code`, `make check-gates`, `make api-surface`
- **Before `/gsd-verify-work`:** Full suite must be green (modulo the two baseline sandbox failures), semver diagnostic run, `make coverage` ≥ 82 %
- **Max feedback latency:** ~60 seconds (quick command)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 48-XX-XX | — | — | ENTITY-01 | — | N/A | compile + unit | `cargo check --workspace --all-targets --all-features` | ✅ | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-01 | — | N/A | integration | `cargo test -p paladin-ai-core --test entity_serde_compat` | ❌ W0 | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-02 | — | N/A | guard | `cargo test --test entity_rename_guard` | ❌ W0 | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-03 | — | N/A | guard | `cargo test --test legacy_removal_guard --test entity_rename_guard` | ✅ / ❌ W0 | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-03 | — | N/A | unit + functional | `cargo test -p paladin-ai-core entity_version_service` | ✅ (renamed) | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-04 | — | N/A | guard + doctest | `cargo test -p paladin-ai-core --doc entity` | ❌ W0 | ⬜ pending |
| 48-XX-XX | — | — | ENTITY-05 | — | N/A | guard + git + script | `make check-migration-allowlist && make api-surface && make doc-check && make check-api-examples` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky — rows are refined to concrete task IDs once PLAN.md files exist.*

---

## Wave 0 Requirements

- [ ] `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt}.json` — captured from the **unmodified** tree before the rename lands
- [ ] `crates/paladin-core/tests/entity_serde_compat.rs` — serde alias compatibility test (ENTITY-01)
- [ ] `tests/entity_rename_guard.rs` — repository-scan guard in the `legacy_removal_guard.rs` shape (ENTITY-02/03/05); lands last because it cannot pass before the rename
- [ ] Framework install: none — existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ADR-0060 exists and `MIGRATION.md` §9.7 still opens with "Empty" | ENTITY-02 | `.planning/` is in the guard's skip set | `ls .planning/decisions/0060-*.md`; `sed -n '/^## 9.7/,/^## 9.8/p' MIGRATION.md` |
| 82 % workspace coverage floor | ENTITY-05 | needs Docker services (`make services-up && make coverage`) | run in CI `coverage` job or locally with services up |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
