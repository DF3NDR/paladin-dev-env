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
| 48-01-01 | 01 | 1 | ENTITY-01 | T-48-01, T-48-02 | fixtures are v0.10 bytes, no secrets | integration (tracer) | `cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel` | ❌ W0 (created by this task) | ⬜ pending |
| 48-01-02 | 01 | 1 | ENTITY-01 | T-48-02 | lossless round trip on the pre-rename tree | integration | `cargo test -p paladin-ai-core --test entity_serde_compat` | ❌ W0 (created by this task) | ⬜ pending |
| 48-02-01 | 02 | 1 | ENTITY-02, ENTITY-05 | T-48-04, T-48-05 | licence precedes every removal | doc check | `test -f .planning/decisions/0060-base-entity-rename.md && grep -q 'X-03 is superseded for Phase 48 only' .planning/decisions/0060-base-entity-rename.md` | n/a | ⬜ pending |
| 48-02-02 | 02 | 1 | ENTITY-02 | T-48-04 | — | doc check + decision gate | `node .claude/gsd-core/bin/gsd-tools.cjs check decision-coverage-plan .planning/phases/48-base-entity-rename .planning/phases/48-base-entity-rename/48-CONTEXT.md` | n/a | ⬜ pending |
| 48-03-01 | 03 | 2 | ENTITY-04 | — | — | compile + unit + script | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --lib && make check-api-examples` | ✅ | ⬜ pending |
| 48-03-02 | 03 | 2 | ENTITY-03 | T-48-06, T-48-07 | both key spellings rejected; graph `node_id` untouched | unit + functional + script | `cargo test -p paladin-ai-core --lib entity_version_service && cargo test -p paladin-ai --test functional content_ && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ (renamed) | ⬜ pending |
| 48-04-01 | 04 | 3 | ENTITY-01, ENTITY-03 | T-48-08, T-48-09, T-48-10 | wire unchanged by the type rename | compile + integration + lint | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core && cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings` | ✅ | ⬜ pending |
| 48-04-02 | 04 | 3 | ENTITY-04 | — | — | doctest + rustdoc | `cargo test -p paladin-ai-core --doc entity && make doc-check` | ✅ | ⬜ pending |
| 48-05-01 | 05 | 4 | ENTITY-01, ENTITY-05 | T-48-12, T-48-13 | graph-sense lines byte-identical | compile + integration | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-battalion --lib && cargo test -p paladin-eval --lib` | ✅ | ⬜ pending |
| 48-05-02 | 05 | 4 | ENTITY-01 | T-48-11, T-48-12 | both keys / neither key rejected | integration + unit | `cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-ai-core --lib test_entity_serializes_fields_in_declaration_order` | ✅ | ⬜ pending |
| 48-06-01 | 06 | 4 | ENTITY-05 | T-48-16 | — | docs | `make check-doc-examples` plus the banned-string grep in the task verify | ✅ | ⬜ pending |
| 48-06-02 | 06 | 4 | ENTITY-05 | T-48-15 | instruction files changed minimally | docs | `make check-doc-examples` plus the naming-row grep in the task verify | ✅ | ⬜ pending |
| 48-07-01 | 07 | 5 | ENTITY-05 | T-48-18, T-48-SC | lints measured, not guessed | script | `make check-migration-allowlist` (plus the CI-equivalent semver loop recorded in the SUMMARY) | ✅ | ⬜ pending |
| 48-07-02 | 07 | 5 | ENTITY-02 | T-48-17 | downgrade procedure documented | script | `./scripts/check-changelogs.sh && make check-migration-allowlist` | ✅ | ⬜ pending |
| 48-08-01 | 08 | 5 | ENTITY-02, ENTITY-03, ENTITY-05 | T-48-20, T-48-21 | the guard can fail (positive controls) | guard | `cargo test --test entity_rename_guard scanner_` | ❌ W0 (created by this task) | ⬜ pending |
| 48-08-02 | 08 | 5 | ENTITY-03, ENTITY-05 | T-48-19 | graph declarations frozen | guard + git | `cargo test --test entity_rename_guard && cargo test --test legacy_removal_guard` plus the byte-identical `git diff --exit-code` in the task verify | ✅ | ⬜ pending |
| 48-09-01 | 09 | 6 | ENTITY-05 | T-48-22, T-48-24, T-48-SC | full gate, credential review | full suite | `cargo fmt --all --check && make lint && make doc-check && make check-api-examples && make check-gates && cargo test --test entity_rename_guard && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 48-09-02 | 09 | 6 | ENTITY-01..05 | T-48-23 | requirement IDs unchanged | doc check | the ADR `conforms` / ENTITY-count check in the task verify | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky — rows are refined to concrete task IDs once PLAN.md files exist.*

---

## Wave 0 Requirements

- [ ] `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt}.json` — captured from the **unmodified** tree before the rename lands (plan 48-01 Task 1, wave 1; every rename plan depends on it)
- [ ] `crates/paladin-core/tests/entity_serde_compat.rs` — serde compatibility harness (ENTITY-01): created in its pre-rename form by plan 48-01 Task 2, finalized with the renamed-key round trip and the alias edge cases by plan 48-05
- [ ] `tests/entity_rename_guard.rs` — repository-scan guard in the `legacy_removal_guard.rs` shape (ENTITY-02/03/05); plan 48-08, wave 5, because it cannot pass before the rename and the docs
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
