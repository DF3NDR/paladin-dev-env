---
phase: 33
slug: commissary-in-tree-adoption
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-16
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from `33-RESEARCH.md` § Validation Architecture (2026-09-16). Task IDs in the
> per-task map are filled in once `33-NN-PLAN.md` files exist; validate-phase promotes
> `status` to `validated`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in), `#[tokio::test]` for async paths, `proptest 1.4` (`proptest!` macro) for the COMM-01 property test |
| **Config file** | none — feature gates in `Cargo.toml` and `tests/integration/mod.rs` `#[cfg(feature = "...")]` attributes serve this role |
| **Quick run command** | `cargo test -p paladin-memory --lib` |
| **Full suite command** | `cargo test --features integration-tests,llm-all --workspace -- --test-threads=1` |
| **Estimated runtime** | quick ~30 s warm; full suite ~5–8 min warm (workspace); pre-commit hook adds ~70–140 s of workspace clippy per commit |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p paladin-memory --lib`
- **After every plan wave:** Run `cargo test --workspace` (add `--all-features` once the `paladin-memory` → `paladin-llm` dependency edge lands, to catch the `crate-isolation`-equivalent break locally before CI does)
- **Before `/gsd-verify-work`:** Full suite must be green, plus the full COMM-04 gate list (CONTEXT.md D-24) on the phase's final commit with evidence recorded per D-25
- **Max feedback latency:** 60 seconds (quick command)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 1 | COMM-01 | — | `ShedItem.label` is the memory UUID, never memory content (D-09) | property (`proptest!`) | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ W0 | ⬜ pending |
| TBD | TBD | 1 | COMM-01 | — | budget `usize → u32` via `try_from`, never clamped (D-05) | unit (example-based, distinct non-round numbers) | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ W0 | ⬜ pending |
| TBD | TBD | 1 | COMM-02 | — | shed record present ⇔ budget exceeded; absent ⇔ everything fits | unit | `cargo test -p paladin-memory --lib rag_retrieval_service::tests` | ❌ W0 | ⬜ pending |
| TBD | TBD | 1 | COMM-02 | — | marker in both renderers; `info!` line carries counts only, no content (D-16) | unit (one per renderer) | `cargo test -p paladin-memory --lib` and `cargo test -p paladin-ai paladin_execution_service` | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | COMM-03 | — | real RAG path over `InMemorySanctum` exercises `Commissary::dispense`; ungated | integration | `cargo test --features integration-tests --test integration_tests rag_commissary_test` (confirm the aggregator binary name via `ls tests/*.rs`) | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | COMM-03 | — | no silent token-based truncation remains | grep (script-based) | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` → empty; `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` → empty | N/A | ⬜ pending |
| TBD | TBD | 3 | COMM-04 | — | `MIGRATION.md` no-TBD; §9.2 row-level set-equal with the allowlist | automated | `grep -c TBD MIGRATION.md` → 0 && `bash scripts/check-migration-allowlist.sh` | ✅ | ⬜ pending |
| TBD | TBD | 3 | COMM-04 | — | frozen v0.9 compat tests pass | integration | `cargo test --features web-server --test v0_9_config_boot` && `cargo test -p paladin-web --test openapi_golden_v0_9` | ✅ | ⬜ pending |
| TBD | TBD | 3 | COMM-04 | — | semver / MSRV / publish dry-run / API surface green | automated (CLI tools) | per CONTEXT.md D-24 command list (`cargo semver-checks … --release-type minor`, `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked`, `make publish-dry-run`, `make api-surface`) | ✅ | ⬜ pending |
| TBD | TBD | 3 | COMM-04 | — | 82 % workspace line-coverage floor | automated | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` (needs Redis/MinIO; CI `coverage` job is the evidence when Docker is absent) | N/A locally | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/paladin-memory/Cargo.toml` — `proptest = "1.4"` under `[dev-dependencies]` (D-17); no property-test infrastructure exists in this crate yet
- [ ] `tests/integration/rag_commissary_test.rs` — new ungated integration test (D-18), registered in `tests/integration/mod.rs` beside `in_memory_sanctum_tests`
- [ ] Migration of the five pre-existing tests in `crates/paladin-memory/src/services/rag_retrieval_service.rs`'s `#[cfg(test)]` module to the new result-struct signature, in the same commit the signature changes (RESEARCH Pitfall 3)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maintainer sign-off boxes in `.project/v0.10.0/09-program-acceptance-audit.md` (the seven Phase 29 boxes plus the new §11 "the `0.10.0` tag may be cut" box) | COMM-04 | Phase 29 D-17: judgment-tier sign-off is closed by a human, never by an agent | Maintainer reviews §11's per-gate evidence and ticks the boxes at UAT / `/gsd-verify-work` |
| Post-merge CI `publish-dry-run` job (runs only on `main` pushes, `ci.yml:1974`) | COMM-04 | The job cannot run on the feature branch; the local `make publish-dry-run` is the pre-merge evidence (Phase 29 D-21 two-SHA rule) | After the `main` merge, record the run id and conclusion in `33-CI-EVIDENCE.md` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
