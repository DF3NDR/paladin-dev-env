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
| T1 (edge) | 33-01 | 1 | COMM-01 | T-33-03 | no HTTP/TLS stack enters `paladin-memory` via the new edge (D-01) | build + dependency-graph check | `cargo build -p paladin-memory` ×3 feature shapes && `cargo tree -p paladin-memory -e normal -i reqwest` finds nothing | ✅ | ⬜ pending |
| T3 (tracer) | 33-01 | 1 | COMM-01 | T-33-01, T-33-02 | `ShedItem.label` is the memory UUID, never memory content (D-09); budget `usize → u32` via `try_from`, never clamped (D-05) | unit (end-to-end, distinct non-round numbers) | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 6 passed) && `cargo check --workspace --all-features --all-targets` | ❌ W0 → created by this task | ⬜ pending |
| T1 (crate renderer) | 33-02 | 2 | COMM-02 | T-33-06, T-33-07 | shed record present ⇔ budget exceeded; absent ⇔ everything fits; marker built in exactly one place (D-15) | unit | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 11 passed) | ❌ W0 → created by this task | ⬜ pending |
| T2 (facade renderer) | 33-02 | 2 | COMM-02 | T-33-05 | both renderers emit the byte-identical marker; the `info!` line carries counts only, no content (D-15, D-16) | unit (one per renderer) + scoped source grep | `cargo test -p paladin-ai --lib rag` (≥ 2 passed) && the `awk`-scoped negative grep over the RAG-success log statement | ❌ W0 → created by this task | ⬜ pending |
| T1 (property) | 33-03 | 3 | COMM-01 | — | retained ≤ budget; no shed memory outscores a retained one; retained ∪ shed = input by id (D-17) | property (`proptest!`) | `cargo test -p paladin-memory --lib ration_respects_budget_and_rank_order` (exactly 1 passed) | ❌ W0 → needs the `proptest` dev-dep from 33-01 T1 | ⬜ pending |
| T2 (edges) | 33-03 | 3 | COMM-01 | T-33-02, T-33-09 | oversized-single retained truncated (D-10a); budget overflow errors, never clamps (D-05); equal scores keep insertion order (D-08) | unit (named edge tests) | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 16 passed) | ❌ W0 → created by this task | ⬜ pending |
| T1 (integration) | 33-04 | 3 | COMM-03 | T-33-01 | real RAG path over `InMemorySanctum` exercises `Commissary::dispense`; ungated, service-free | integration | `cargo test --test rag_commissary` (≥ 3 passed, NO `--features` flag) | ❌ W0 → new file `tests/integration/rag_commissary_test.rs` | ⬜ pending |
| T2 (exit grep + docs) | 33-04 | 3 | COMM-03 | T-33-10 | no silent token-based truncation remains | grep (script-based) + docs build | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` → empty; `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` → empty; `mdbook build docs` | N/A | ⬜ pending |
| T2 (register) | 33-05 | 4 | COMM-04 | T-33-12 | `MIGRATION.md` no-TBD; §9.2 row-level set-equal with the allowlist; no entry for a lint that did not fire | automated | `grep -c TBD MIGRATION.md` → 0 && `make check-migration-allowlist` && `make check-gates` | ✅ | ⬜ pending |
| T3 (changelog + baseline) | 33-05 | 4 | COMM-04 | T-33-13, T-33-14 | the behavioural change is announced; the API baseline matches the tree | automated | `make api-surface` && the `[0.10.0]`-scoped CHANGELOG greps (RAG note, `TokenUsage`, `Commissary`) | ✅ | ⬜ pending |
| T1 (gate sweep) | 33-06 | 5 | COMM-04 | T-33-16 | frozen v0.9 compat tests pass; semver / MSRV / publish dry-run / API surface green; PRIM-04 regression green | integration + automated (CLI tools) | `cargo test --features web-server --test v0_9_config_boot` && `cargo test -p paladin-web --test openapi_golden_v0_9` && the D-24 command list && `cargo test -p paladin-ai --lib limit_resolution` / `... kept_set_equivalence_snapshot_pre_resolver` (non-zero passed counts) | ✅ | ⬜ pending |
| T1 (coverage row) | 33-06 | 5 | COMM-04 | T-33-16 | 82 % workspace line-coverage floor | automated | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` (needs Redis/MinIO; CI `coverage` job is the evidence when Docker is absent) | N/A locally | ⬜ pending |
| T2 (evidence) | 33-06 | 5 | COMM-04 | T-33-15, T-33-17 | §11 is append-only, carries one unticked human-only box and zero ticked boxes | automated | `grep -q '^## 11\. Re-seal after Phases 30-33'` && zero `- [x]` lines within §11 && `Re-sealed on` present in the Phase 29 pointer | ✅ | ⬜ pending |

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
