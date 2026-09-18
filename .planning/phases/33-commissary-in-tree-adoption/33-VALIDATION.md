---
phase: 33
slug: commissary-in-tree-adoption
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-16
validated: 2026-09-17
validated_head: 7a17eada
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from `33-RESEARCH.md` § Validation Architecture (2026-09-16). Task IDs in the
> per-task map are filled in once `33-NN-PLAN.md` files exist; validate-phase promoted
> `status` to `validated` on 2026-09-17 (audit trail at the end of this file).

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
| T1 (edge) | 33-01 | 1 | COMM-01 | T-33-03 | no HTTP/TLS stack enters `paladin-memory` via the new edge (D-01) | build + dependency-graph check | `cargo build -p paladin-memory` ×3 feature shapes && `cargo tree -p paladin-memory -e normal -i reqwest` finds nothing | ✅ | ✅ green — `cargo tree -p paladin-memory -e normal -i reqwest` → "did not match any packages" (reqwest absent from the graph) |
| T3 (tracer) | 33-01 | 1 | COMM-01 | T-33-01, T-33-02 | `ShedItem.label` is the memory UUID, never memory content (D-09); budget `usize → u32` via `try_from`, never clamped (D-05) | unit (end-to-end, distinct non-round numbers) | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 6 passed) && `cargo check --workspace --all-features --all-targets` | ✅ | ✅ green — 23 passed (≥ 6) |
| T1 (crate renderer) | 33-02 | 2 | COMM-02 | T-33-06, T-33-07 | shed record present ⇔ budget exceeded; absent ⇔ everything fits; marker built in exactly one place (D-15) | unit | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 11 passed) | ✅ | ✅ green — 23 passed (≥ 11) |
| T2 (facade renderer) | 33-02 | 2 | COMM-02 | T-33-05 | both renderers emit the byte-identical marker; the `info!` line carries counts only, no content (D-15, D-16) | unit (one per renderer) + scoped source grep | `cargo test -p paladin-ai --lib rag` (≥ 2 passed) && the `awk`-scoped negative grep over the RAG-success log statement | ✅ | ✅ green — 7 passed (≥ 2); scoped grep over the `RAG retrieval succeeded` statement → 0 body/content matches; `rag_omission_marker` present, no `omitted to fit` literal |
| T1 (property) | 33-03 | 3 | COMM-01 | — | retained ≤ budget; no shed memory outscores a retained one; retained ∪ shed = input by id (D-17) | property (`proptest!`) | `cargo test -p paladin-memory --lib ration_respects_budget_and_rank_order` (exactly 1 passed) | ✅ | ✅ green — exactly 1 passed |
| T2 (edges) | 33-03 | 3 | COMM-01 | T-33-02, T-33-09 | oversized-single retained truncated (D-10a); budget overflow errors, never clamps (D-05); equal scores keep insertion order (D-08) | unit (named edge tests) | `cargo test -p paladin-memory --lib rag_retrieval_service` (≥ 16 passed) | ✅ | ✅ green — 23 passed (≥ 16) |
| T1 (integration) | 33-04 | 3 | COMM-03 | T-33-01 | real RAG path over `InMemorySanctum` exercises `Commissary::dispense`; ungated, service-free | integration | `cargo test --test rag_commissary` (≥ 3 passed, NO `--features` flag) | ✅ `tests/integration/rag_commissary_test.rs` | ✅ green — 3 passed, no `--features` flag |
| T2 (exit grep + docs) | 33-04 | 3 | COMM-03 | T-33-10 | no silent token-based truncation remains | grep (script-based) + docs build | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` → empty; `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` → empty; `mdbook build docs` | N/A | ✅ green — both greps empty; `mdbook build docs` exit 0, "No broken links found" |
| T2 (register) | 33-05 | 4 | COMM-04 | T-33-12 | `MIGRATION.md` no-TBD; §9.2 row-level set-equal with the allowlist; no entry for a lint that did not fire | automated | `grep -c TBD MIGRATION.md` → 0 && `make check-migration-allowlist` && `make check-gates` | ✅ | ✅ green — `TBD` count 0; `make check-gates` exit 0 (allowlist set-equal, 15 pairs) |
| T3 (changelog + baseline) | 33-05 | 4 | COMM-04 | T-33-13, T-33-14 | the behavioural change is announced; the API baseline matches the tree | automated | `make api-surface` && the `[0.10.0]`-scoped CHANGELOG greps (RAG note, `TokenUsage`, `Commissary`) | ✅ | ✅ green — `make api-surface` "unchanged" (3959 items); 22 `[0.10.0]`-scoped RAG/`TokenUsage`/`Commissary` hits |
| T1 (gate sweep) | 33-06 | 5 | COMM-04 | T-33-16 | frozen v0.9 compat tests pass; semver / MSRV / publish dry-run / API surface green; PRIM-04 regression green | integration + automated (CLI tools) | `cargo test --features web-server --test v0_9_config_boot` && `cargo test -p paladin-web --test openapi_golden_v0_9` && the D-24 command list && `cargo test -p paladin-ai --lib limit_resolution` / `... kept_set_equivalence_snapshot_pre_resolver` (non-zero passed counts) | ✅ | ✅ green — `v0_9_config_boot` 9 passed; `openapi_golden_v0_9` 7 passed; `limit_resolution` 3 passed; `kept_set_equivalence_snapshot_pre_resolver` 1 passed; semver / MSRV / publish-dry-run evidence recorded in `33-CI-EVIDENCE.md` on `69500c9b` (see audit note on SHA freshness) |
| T1 (coverage row) | 33-06 | 5 | COMM-04 | T-33-16 | 82 % workspace line-coverage floor | automated | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` (needs Redis/MinIO; CI `coverage` job is the evidence when Docker is absent) | N/A locally | ✅ green (lower bound) — `cargo llvm-cov --workspace --features llm-all --fail-under-lines 82 -- --test-threads=1` on `7a17eada` → 90.51 % lines, exit 0, 0 failed; dropping `integration-tests` only removes tests, so CI's figure is bounded below by this. Exact CI command still needs Docker; CI `coverage` job confirms once pushed |
| T2 (evidence) | 33-06 | 5 | COMM-04 | T-33-15, T-33-17 | §11 is append-only, carries one unticked human-only box and zero ticked boxes | automated | `grep -q '^## 11\. Re-seal after Phases 30-33'` && zero `- [x]` lines within §11 && `Re-sealed on` present in the Phase 29 pointer | ✅ | ✅ green — §11 header present; 0 `- [x]` and 1 `- [ ]` inside §11; `Re-sealed on` present in `29-ACCEPTANCE-AUDIT.md` |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/paladin-memory/Cargo.toml` — `proptest = "1.4"` under `[dev-dependencies]` (D-17); no property-test infrastructure exists in this crate yet
- [x] `tests/integration/rag_commissary_test.rs` — new ungated integration test (D-18), registered in `tests/integration/mod.rs` beside `in_memory_sanctum_tests`
- [x] Migration of the five pre-existing tests in `crates/paladin-memory/src/services/rag_retrieval_service.rs`'s `#[cfg(test)]` module to the new result-struct signature, in the same commit the signature changes (RESEARCH Pitfall 3)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maintainer sign-off boxes in `.project/v0.10.0/09-program-acceptance-audit.md` (the seven Phase 29 boxes plus the new §11 "the `0.10.0` tag may be cut" box) | COMM-04 | Phase 29 D-17: judgment-tier sign-off is closed by a human, never by an agent | Maintainer reviews §11's per-gate evidence and ticks the boxes at UAT / `/gsd-verify-work` |
| Post-merge CI `publish-dry-run` job (runs only on `main` pushes, `ci.yml:1974`) | COMM-04 | The job cannot run on the feature branch; the local `make publish-dry-run` is the pre-merge evidence (Phase 29 D-21 two-SHA rule) | After the `main` merge, record the run id and conclusion in `33-CI-EVIDENCE.md` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-09-17 by `/gsd-validate-phase 33` on `7a17eada` — 13/13 rows automated and green; 0 gaps; no tests generated (none missing)

---

## Validation Audit 2026-09-17

| Metric | Count |
|--------|-------|
| Rows audited | 13 |
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

All 13 per-task rows were re-run on HEAD `7a17eada` (not read from the SUMMARYs) and are green.
No `gsd-nyquist-auditor` spawn was needed: every requirement (COMM-01..04) already has an
automated command that selects a non-zero test count or a deterministic grep/tool verdict.

**Coverage row, reclassified from CI-attributed to locally bounded.** `33-CI-EVIDENCE.md` row 32
records the 82 % floor as "not measurable locally" because the exact CI command needs Docker-backed
Redis/MinIO. This audit measured the strict lower bound instead — the same `cargo llvm-cov`
invocation minus the `integration-tests` feature, which only removes tests — and got **90.51 %
lines (exit 0 under `--fail-under-lines 82`, 0 failed tests)**. The CI `coverage` job remains the
evidence for the exact command; `feature/phase-33` has not been pushed yet (`gh run list` → none),
so that run id is still to be recorded in `33-CI-EVIDENCE.md` per the Manual-Only table.

**SHA-freshness note (not a Nyquist gap, flagged for the re-seal owner).** §11 of the corpus
acceptance audit and `33-CI-EVIDENCE.md` record the D-24 sweep on `69500c9b`. Since then the
code-review fixes WR-01/WR-02/WR-03/IN-01 changed `crates/paladin-memory/src/prelude.rs` and
`.../rag_retrieval_service.rs` (+127/−6, incl. the new `RagRetrievalError::DuplicateMemoryId`
variant). Every gate this audit could re-run on `7a17eada` is green (compat tests, PRIM-04
regression, `make api-surface` unchanged, `make check-gates`, docs build, coverage lower bound);
the `cargo semver-checks`, MSRV `cargo check --locked`, and `make publish-dry-run` rows were not
re-run here and still cite `69500c9b`. Plan 33-06's own rule ("any gate whose recorded SHA differs
from the phase's final commit is re-run") applies before the `0.10.0` tag is cut.
