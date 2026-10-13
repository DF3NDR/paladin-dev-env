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

Regenerated 2026-10-13 for the nine plans written from the owner-updated `48-CONTEXT.md` (D-15). One
row per task; the command is the task's exact `<verify><automated>` (pipes escaped for the table).
**ENTITY-04's expectation is "retained pending review", not removal** (D-05): its rows check that the
five base items are still declared, changed only where the rename forces it, and documented with a
written reason -- never that they are gone.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 48-01-01 | 48-01 | 1 | ENTITY-01 | T-48-01, T-48-02 | fixtures hold only v0.10 keys and no secret; a v0.10 snapshot loads through FileCitadel and saves back identical | tracer (adapter integration) | `cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel` | ❌ W0 (created by this task) | ⬜ pending |
| 48-01-02 | 48-01 | 1 | ENTITY-01, ENTITY-03 | T-48-02 | five v0.10 shapes load and round-trip losslessly; no relaxed assertion | integration | `cargo test -p paladin-ai-core --test entity_serde_compat && cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings && cargo fmt --all --check` | ❌ W0 (created by this task) | ⬜ pending |
| 48-02-01 | 48-02 | 1 | ENTITY-02, ENTITY-04, ENTITY-05 | T-48-04, T-48-05, T-48-06 | licence scoped to Phase 48; read-aliases recorded as data compatibility, not an API shim; retained items recorded as retained pending review | doc check | `test -f .planning/decisions/0060-base-entity-rename.md && grep '^## ' .planning/decisions/0060-base-entity-rename.md \| tr -d '\r' \| paste -sd'\|' \| grep -qx '## Status\|## Context\|## Decision\|## Considered Options\|## Code Locations\|## Code Conformance\|## Downstream Consumers' && grep -q 'X-03 is superseded for Phase 48 only' .planning/decisions/0060-base-entity-rename.md && grep -q 'means a vertex in a WarGraph or Campaign graph' .planning/decisions/0060-base-entity-rename.md && grep -q 'retained pending review' .planning/decisions/0060-base-entity-rename.md` | — (doc) | ⬜ pending |
| 48-02-02 | 48-02 | 1 | ENTITY-02 | T-48-04 | ADR indexed; next free number advanced; review todo appended only | doc check | `` grep -q '^\| 0060 \| `base-entity-rename` \|' .planning/decisions/PROMOTION.md && grep -q 'Next free ADR number: 0061' .planning/decisions/PROMOTION.md && grep -q '0060-base-entity-rename.md' .planning/PROJECT.md && grep -q 'Phase 48 records its own supersession, ADR-0060' .planning/PROJECT.md && grep -q '^## Residual entity-sense identifiers (Phase 48 planning note)' .planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md `` | — (doc) | ⬜ pending |
| 48-03-01 | 48-03 | 2 | ENTITY-03, ENTITY-04 | T-48-07, T-48-08, T-48-09 | both key spellings rejected; no graph-sense node_id touched; retained services change only their module path | unit + integration + functional | `cargo test -p paladin-ai-core --lib base::service && cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-ai --test functional content_ && cargo clippy -p paladin-ai-core -p paladin-ai --all-targets --all-features -- -D warnings` | ✅ (renamed tests; compat from 48-01) | ⬜ pending |
| 48-03-02 | 48-03 | 2 | ENTITY-04, ENTITY-03 | T-48-SC | baseline moves by exactly the 14 measured lines; CI-pinned tool only; retained-items reason written | script + rustdoc | `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface && make doc-check && make check-api-examples && ./scripts/check-changelogs.sh` | ✅ | ⬜ pending |
| 48-04-01 | 48-04 | 3 | ENTITY-03 | T-48-10, T-48-11 | v0.10 snapshot and every v0.10 status load into the unified type; no fixture or assertion edited | integration | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-memory --lib citadel && cargo test -p paladin-ports citadel_port && cargo test -p paladin-ai --test citadel_integration` | ✅ (adds one compat case) | ⬜ pending |
| 48-04-02 | 48-04 | 3 | ENTITY-01, ENTITY-02 | T-48-12 | no graph-sense Node token rewritten; wire unchanged; no shim | compile + unit + guard | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core && cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel && cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings && cargo test --test legacy_removal_guard && make check-doc-examples` | ✅ | ⬜ pending |
| 48-04-03 | 48-04 | 3 | ENTITY-05, ENTITY-04 | T-48-12 | graph prose untouched; retained files change prose only | rustdoc + grep | `make doc-check && cargo test -p paladin-ai-core --doc && cargo fmt --all --check && ! git grep -nE '(Node pattern\|Node wrapper\|Node infrastructure\|Node entities\|Node structure\|type of Node\|Node fields\|Reconstruct Node\|Node properties\|Node integration\|Node Integration\|Create Node\|wrapped by Node\|built on Node\|base entity Node\|as a Node\|Node Type\|of Nodes)([^A-Za-z0-9_]\|$)' -- '*.rs'` | ✅ | ⬜ pending |
| 48-05-01 | 48-05 | 4 | ENTITY-01, ENTITY-05 | T-48-13, T-48-14 | graph-sense .node lines byte-identical; v0.10 JSON still loads | compile + integration | `cargo check --workspace --all-targets --all-features && cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel && cargo test -p paladin-ai-core --lib && cargo test -p paladin-battalion --lib && cargo test -p paladin-eval --lib` | ✅ | ⬜ pending |
| 48-05-02 | 48-05 | 4 | ENTITY-01, ENTITY-04 | T-48-13, T-48-15 | both keys and neither key rejected; writes never emit an old key; contracts documented | integration + unit + doctest | `cargo test -p paladin-ai-core --test entity_serde_compat && cargo test -p paladin-ai-core --lib test_entity_serializes_fields_in_declaration_order && cargo test -p paladin-ai-core --doc entity && cargo test -p paladin-memory --lib pre_rename_citadel_snapshot_loads_through_file_citadel && make doc-check` | ✅ (adds six cases) | ⬜ pending |
| 48-06-01 | 48-06 | 5 | ENTITY-05, ENTITY-04 | T-48-17 | docs name only real paths; retained items documented as retained pending review | doc build + grep | `make check-doc-examples && python3 -c "import xml.dom.minidom; xml.dom.minidom.parse('docs/src/assets/LayerArchitecture.svg')" && ! grep -rnE '(^\|[^A-Za-z0-9_:"])Node(<\|::new\(\|::<)\|base::entity::node\|NodeVersion\|entity/node\.rs\|base/node\.rs\|entity::collection::Collection\b' docs/src/architecture docs/src/api-reference/crate-map.md docs/src/api-reference/stable-api.md docs/src/appendix/user-rest-api.md` | ✅ | ⬜ pending |
| 48-06-02 | 48-06 | 5 | ENTITY-05 | T-48-16 | always-loaded instruction files change only the Entity rule lines | grep + doc build | `grep -q '\| \*\*Entity\*\* \|' .github/copilot-instructions.md && grep -q 'ADR-0060' CLAUDE.md && ! grep -nE '(^\|[^A-Za-z0-9_:"])Node(<\|::new\(\|::<)\|base::(entity::)?node\|NodeVersion' CLAUDE.md README.md crates/paladin-core/README.md .github/copilot-instructions.md && make check-doc-examples` | ✅ | ⬜ pending |
| 48-07-01 | 48-07 | 5 | ENTITY-02, ENTITY-05, ENTITY-01, ENTITY-03 | T-48-18, T-48-19, T-48-SC | lints measured, not guessed; register set-equal; CI semver loop green; no row for a retained item | script + semver | `make check-migration-allowlist && grep -q 'Phase 48 (Base Entity Rename, ENTITY-01..05)' .cargo/semver-checks-allowlist.toml && cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` | ✅ | ⬜ pending |
| 48-07-02 | 48-07 | 5 | ENTITY-02, ENTITY-05 | T-48-20 | 9.7 still empty; operator upgrade order and restore-only downgrade documented | script + grep | `./scripts/check-changelogs.sh && grep -q '^\| M-B-12 \|' MIGRATION.md && sed -n '/^## 9.7/,/^## 9.8/p' MIGRATION.md \| grep -v '^## 9.7' \| grep -m1 . \| grep -q '^Empty' && make check-migration-allowlist` | ✅ | ⬜ pending |
| 48-08-01 | 48-08 | 6 | ENTITY-05, ENTITY-02, ENTITY-03 | T-48-22, T-48-23 | each rule proven able to fail; retained items not banned; std only | guard (positive controls) | `cargo test --test entity_rename_guard scanner_ && cargo clippy --test entity_rename_guard -- -D warnings` | ❌ W0 (created by this task) | ⬜ pending |
| 48-08-02 | 48-08 | 6 | ENTITY-05, ENTITY-03 | T-48-21 | skip set not widened; graph files byte-identical to the phase base; legacy guard untouched | guard + git | `cargo test --test entity_rename_guard && cargo test --test legacy_removal_guard && BASE=$(git log --reverse --format=%H -F --grep='(48-0' \| head -1) && test -n "$BASE" && git diff --exit-code "$BASE~1" -- docs/schemas/wargraph-doc.schema.json src/config/node_cache.rs src/config/mod.rs crates/paladin-core/src/platform/container/trace.rs crates/paladin-core/src/platform/container/waypoint.rs crates/paladin-core/src/platform/container/node_error.rs crates/paladin-ports/src/output/node_cache_port.rs crates/paladin-battalion/src/engine/node.rs crates/paladin-battalion/src/engine/hooks.rs crates/paladin-storage/src/node_cache/in_memory.rs crates/paladin-storage/src/node_cache/redis.rs` | ✅ (from 48-08-01) | ⬜ pending |
| 48-09-01 | 48-09 | 7 | ENTITY-01..05 | T-48-24, T-48-25 | every gate run and recorded; coverage measured; credential review recorded | full gate | `cargo fmt --all --check && make lint && make doc-check && make check-api-examples && make check-gates && cargo test --test entity_rename_guard && cargo test --test legacy_removal_guard && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 48-09-02 | 48-09 | 7 | ENTITY-01..05 | T-48-26 | ADR conforms; wording corrected with IDs unchanged; ENTITY-04 reads retained pending review | doc check | `sed -n '/^## Code Conformance/,/^## Downstream Consumers/p' .planning/decisions/0060-base-entity-rename.md \| grep -q 'conforms' && ! sed -n '/^## Code Conformance/,/^## Downstream Consumers/p' .planning/decisions/0060-base-entity-rename.md \| grep -q 'must change' && grep -q 'current-exports.txt' .planning/REQUIREMENTS.md && test "$(grep -c '^- \[.\] \*\*ENTITY-0[1-5]\*\*' .planning/REQUIREMENTS.md)" = 5` | — (doc) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky. Every task carries an `<automated>` command; no
command uses a watch-mode flag; the four ❌ W0 rows create their own test in the same task (TDD or
tracer), so no `MISSING` placeholder exists. Waves: 1 = 48-01, 48-02; 2 = 48-03; 3 = 48-04;
4 = 48-05; 5 = 48-06, 48-07; 6 = 48-08; 7 = 48-09.*

---

## Wave 0 Requirements

- [ ] `crates/paladin-core/tests/fixtures/pre_rename/{paladin,citadel,content,prompt,version}.json` — captured from the **unmodified** tree by 48-01-01 before any rename commit (the citadel snapshot over the placeholder `PaladinData` v0.10 writes, so 48-04-01 can prove D-10; `version.json` for D-09)
- [ ] `pre_rename_citadel_snapshot_loads_through_file_citadel` in `crates/paladin-memory/src/citadel/file_citadel.rs` — the tracer, 48-01-01
- [ ] `crates/paladin-core/tests/entity_serde_compat.rs` — serde read-alias compatibility harness (ENTITY-01, D-03/D-09/D-10), 48-01-02; 5 cases, 6 after 48-04-01, 11 after 48-05-02
- [ ] `tests/entity_rename_guard.rs` — repository-scan guard in the `legacy_removal_guard.rs` shape (ENTITY-02/03/05, D-07 rules (a)-(d)), 48-08-01; lands last because it cannot pass before the rename and the docs
- [ ] Framework install: none. Tools only, both CI pins with legitimacy `OK` in 48-RESEARCH.md: `cargo-public-api` 0.52.0 (48-03-02) and `cargo-semver-checks` 0.50.0 (48-07-01). `cargo-audit` / `cargo-deny` are not installed by the phase (48-09-01 falls back to CI's security job).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ADR-0060 exists and `MIGRATION.md` §9.7 still opens with "Empty" | ENTITY-02 | `.planning/` is in the guard's skip set, so the guard cannot see the ADR (the greps in 48-02-01 and 48-07-02 automate the text checks) | `ls .planning/decisions/0060-*.md`; `sed -n '/^## 9.7/,/^## 9.8/p' MIGRATION.md` |
| 82 % workspace coverage floor | ENTITY-05 | needs Docker services (`make services-up && make coverage`) | run in CI `coverage` job or locally with services up (48-09-01 step 7) |
| Five base items retained pending review, not removed (D-05) | ENTITY-04 | wording judgment: the rustdoc, ADR and docs must say *retained pending review* and name the content / general data orchestration review | read `crates/paladin-core/src/base/mod.rs` `# Retained pending review`, ADR-0060 (g), `docs/src/api-reference/stable-api.md` rows; confirm `ls crates/paladin-core/src/base/entity/{field,collection}.rs crates/paladin-core/src/base/component/element.rs crates/paladin-core/src/base/service/{field_version_service,collection_versioning_service}.rs` lists all five |
| Guard matching semantics (backstop edge ENTITY-02/encoding) | ENTITY-02 | the verifier must confirm with evidence that matching is exact, case-sensitive and identifier-bounded | read `occurs` / `is_identifier` in `tests/entity_rename_guard.rs` and the clean-tree look-alikes in `scanner_accepts_a_clean_tree`; an unconfirmed backstop abstains to `human_needed` |
| Manual credential-handling review | all | security.instructions.md requires a human/agent review of credential-bearing code | 48-09-01 step 6 grep over the whole phase diff, outcome recorded in 48-09-SUMMARY.md |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
