---
phase: 48-base-entity-rename
verified: 2026-10-14T00:45:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
human_verification:

  - test: "Read the CI result for HEAD f092886 (or any commit at or after 2ad9fa6): Docker Integration Tests, Coverage, pre-commit"
    expected: "Docker Integration Tests green (the 2ad9fa6 guard fix lets tests/entity_rename_guard.rs run in a build context with no docs/); Coverage green with --fail-under-lines 82"
    why_human: "The CI run for f092886 was still queued when verified. The only CI evidence is run 38487208284 on dcbcb63: Coverage success, Docker Integration Tests failure (the guard regression that 2ad9fa6 fixes), ci.yml overall cancelled. The fix was reproduced by plan 48-09 and by the positive control scanner_accepts_a_build_context_without_docs (which passes here), but CI has not yet confirmed it."

  - test: "Run `make security` (cargo-audit and cargo-deny) in an environment that has them, and read the Security Audit and License and Dependency Policy CI job logs"
    expected: "Pass, or a failure that is already present on the pre-phase tree"
    why_human: "cargo-audit and cargo-deny are not installed here. Both CI jobs are red on dcbcb63. 48-09-SUMMARY says they were red before any Phase 48 code, and Cargo.lock is byte-identical to the phase base, which I confirmed. Which advisory or licence rule fires is unknown. ROADMAP SC5 does not name make security, but the project's security.instructions.md requires it before a phase is sealed."

  - test: "Measure the workspace line-coverage percentage (cargo llvm-cov --fail-under-lines 82) or read it from the CI Coverage log"
    expected: ">= 82 %"
    why_human: "No cargo-llvm-cov, no Docker daemon and low disk here. The only evidence is the CI Coverage job conclusion on dcbcb63 (success), not a number."
---

# Phase 48: Base Entity Rename Verification Report

**Phase Goal:** The word "node" means exactly one thing in the Paladin workspace, a vertex in a WarGraph/Campaign graph, because `paladin_core::base::entity::node::Node<T>` is renamed `Entity<T>` with payload field `node` renamed `data`, and the unused remainder of the base layer is dispositioned.
**Verified:** 2026-10-14
**Status:** human_needed (all five success criteria hold in the code; three environment/CI items could not be confirmed here)
**Re-verification:** No, initial verification
**Verified against:** HEAD f092886, using the corrected 2026-10-14 ROADMAP wording for criteria 1, 4 and 5.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `base::entity::Entity<T>` exists with payload `data`. Paladin, Council, Grove, User, Log, ContentItem and PromptItem are built on it. `#[serde(alias = "node")]` plus fixture tests prove pre-rename JSON still loads. | VERIFIED | `crates/paladin-core/src/base/entity/wrapper.rs` declares `pub struct Entity<T>` with `#[serde(alias = "node")] pub data: T`. It is re-exported by `base/entity/mod.rs` (`pub use wrapper::Entity`). Aliases: `paladin.rs:229`, `council.rs:268`, `grove.rs:291`, `user.rs:165`, `log.rs:397`. `ContentItem` and `PromptItem` hold `entity: Entity<..>` with `#[serde(alias = "node")]` (D-08). The five pre-rename fixtures exist in `crates/paladin-core/tests/fixtures/pre_rename/`. They were added in 28c9bc5 and not touched since. They carry `"node"` keys and none carries `"data"`. I ran `cargo test -p paladin-ai-core --test entity_serde_compat`: 11/11 pass. That includes paladin, citadel, content, prompt and version loads, both-keys rejected, no payload key rejected, and new writes never emit the old keys. I ran `pre_rename_citadel_snapshot_loads_through_file_citadel` in paladin-memory: pass, through the real FileCitadel. |
| 2 | The old name's fate is decided and recorded (default path): `Node<T>` and `base::entity::node` removed outright; ADR-0060 records the Phase-48-only X-03 supersession; MIGRATION §9.7 still empty. | VERIFIED | There is no `node.rs` in `base/entity/`. My grep for `Node::new`, `Node<`, `entity::node` and `node_version_service` over `.rs`, `.md`, `.toml`, `.svg` and `.json` found no live hit. The only hits are the allowlist and MIGRATION register text that must name the old items, plus the historic `.project/.../facade-audit.md`, which is a dot-directory and in the guard's declared history set. `.planning/decisions/0060-base-entity-rename.md` has the seven PROMOTION headings, Status Accepted, and subsection (a) states the Phase-48-only supersession and the no-API-shim rule with the serde carve-out. `PROMOTION.md` has the 0060 row and next free 0061. MIGRATION §9.7 still says "Empty" and names Phase 48 only as a no-deprecation note. D-02 is honoured. |
| 3 | Exactly one `NodeError` (structured). `legacy_removal_guard` green. Exactly one `Paladin` alias. `NodeVersion*` family renamed `EntityVersion*`. | VERIFIED | `pub struct NodeError` appears only in `platform/container/node_error.rs:152`. `pub type Paladin =` appears only in `paladin.rs:229`. `citadel.rs` no longer declares `PaladinData`, `PaladinStatus` or the alias. `PaladinState.paladin: Paladin` (the real one), and the citadel tests import from `super::paladin` (D-10). `entity_version_service.rs` declares `EntityVersion`, `EntityVersionMetadata`, `EntityVersionRepository`, `EntityVersionService` and `InMemoryEntityVersionRepository`. `EntityVersion` carries `alias = "node_id"` and `alias = "node_data"` (D-09). I ran `cargo test --test legacy_removal_guard`: 3/3 pass. The `entity_version_service` and `base::entity` lib tests: 16/16 pass. |
| 4 | Each of the five dormant base items is removed or kept with a written reason. `Entity<T>::version`'s contract is stated in rustdoc, including that only `ContentItem` honours it. | VERIFIED | All five are retained (D-05, as the owner decided). `base/mod.rs` has a `# Retained pending review` section naming the content / general data orchestration system and the todo path. That todo file exists. ADR-0060 (g) records the disposition. `wrapper.rs` has a `# Revisioning contract` section. Its claim that `create_version` returns `VersioningDisabled` when the flag is false matches the code (`entity_version_service.rs:184`). It says only `ContentItem` is revisioned (via `ContentItemService`) and that platform-wide revisioning is not promised. The `Entity` doctests pass (2/2). |
| 5 | The graph vocabulary is untouched. An ADR, the naming tables and the mdBook pages state the node/Entity rule. A guard test pins the freeze and the absence of old paths. Every rename has a §9.2 row and a set-equal allowlist entry. The API baseline is refreshed with a CHANGELOG entry. `cargo doc`, `make check-api-examples`, `make clean-code` and the 82 % floor stay green. | VERIFIED, with the environment gaps listed under Human Verification | **Untouched graph:** `git diff 25dc267..HEAD` shows `docs/schemas/wargraph-doc.schema.json`, `src/config/node_cache.rs`, `node_error.rs` and `Cargo.lock` unchanged. The engine and battalion edits (including `graph.rs` and `graph_doc.rs`, which the SUMMARY's "ten frozen sources" list leaves out) are only entity-sense lines: `paladin.node.x` becomes `paladin.data.x`, `Node::new` becomes `Entity::new`, and a doc comment changes. No graph identifier is removed or renamed. **Rule recorded:** ADR-0060 (i), the `domain-model.md` vocabulary paragraph, the `CLAUDE.md` crate-table row and Working-agreements line, and the `Entity` row of the `.github/copilot-instructions.md` naming table. `design-patterns.md`, `crate-map.md`, `overview.md`, `stable-api.md` and `LayerArchitecture.svg` all mention Entity. **Guard:** `tests/entity_rename_guard.rs` is std only. Its needles are assembled at run time. It has five rules, including a graph freeze over 12 files, and positive controls. I ran it: 5/5 pass. **Register:** `make check-migration-allowlist` reports the allowlist set-equal to §9.2. §9.2 has Phase 48 rows for `Node`, the `NodeVersionService` family, citadel `PaladinData`, `PaladinStatus` and `Paladin`, `PaladinState`, `ContentItem`, `PromptItem` and `ContentItemService`. `crates/paladin-core/Cargo.toml` gains the measured allow lines. **Baseline and changelogs:** `.project/current-exports.txt` moved 30 lines and has no old names. CHANGELOG.md and crates/paladin-core/CHANGELOG.md both gained entries. **Gates I re-ran:** `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo check --workspace --all-targets` clean; `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-ai-core` clean; `make check-api-examples` passes (105/105). **Not confirmed here:** the 82 % coverage figure and `make security` (see Human Verification). |

**Score:** 5/5 truths verified. No behavior-dependent truth is left without a test. The serde alias, duplicate-key and missing-key behaviours and the `VersioningDisabled` contract each have a passing named test.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/base/entity/wrapper.rs` | `Entity<T>`, field `data`, alias, rustdoc contract | VERIFIED | Substantive, with unit tests, and used by 7 container types |
| `crates/paladin-core/src/base/service/entity_version_service.rs` | EntityVersion family | VERIFIED | Wired into `ContentItemService`; the baseline shows the new paths |
| `crates/paladin-core/tests/entity_serde_compat.rs` and `tests/fixtures/pre_rename/*.json` | Compatibility harness | VERIFIED | 11 tests pass; fixtures are unmodified since capture |
| `tests/entity_rename_guard.rs` | Rename and freeze guard | VERIFIED | 5/5 pass, auto-discovered by root `tests/` |
| `.planning/decisions/0060-base-entity-rename.md` | The ADR | VERIFIED | Code Conformance: conforms |
| `crates/paladin-core/src/base/mod.rs` | Retained-items reason | VERIFIED | Names the system and the todo; the todo exists |
| `MIGRATION.md` §9.1 M-B-12, §9.2, §9.4, §9.7 and `.cargo/semver-checks-allowlist.toml` | Register | VERIFIED | Set-equal check passes |
| `CLAUDE.md`, `.github/copilot-instructions.md`, mdBook pages | Vocabulary rule | VERIFIED | Present |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `Entity` wrapper | seven domain types | WIRED | Aliases and fields confirmed in code |
| `PaladinState.paladin` | real `Paladin` | WIRED | Pre-rename snapshot loads through `FileCitadel` |
| `ContentItemService` | `EntityVersionRepository` | WIRED | `.project/current-exports.txt` shows the signatures |
| MIGRATION §9.2 rows | allowlist entries | WIRED | `make check-migration-allowlist` is set-equal |
| Graph definitions | guard rule 5 | WIRED | The guard passes on the real tree |

### Data-Flow Trace (Level 4)

Not applicable: the phase produces no UI or rendering artifact. Persisted-JSON flow was traced through the fixture and `FileCitadel` tests (load, then save, then equal).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Rename guard and its positive controls | `cargo test --test entity_rename_guard` | 5 passed | PASS |
| Phase 44 guard still green | `cargo test --test legacy_removal_guard` | 3 passed | PASS |
| Pre-rename JSON compatibility | `cargo test -p paladin-ai-core --test entity_serde_compat` | 11 passed | PASS |
| Citadel snapshot through the real adapter | `cargo test -p paladin-memory pre_rename` | 1 passed | PASS |
| Version service and wrapper tests | `cargo test -p paladin-ai-core --lib -- entity_version_service base::entity` | 16 passed | PASS |
| Entity doctests | `cargo test -p paladin-ai-core --doc -- Entity` | 2 passed | PASS |
| Register is set-equal | `make check-migration-allowlist` | set-equal | PASS |
| Lint, format, check, docs, examples gate | clippy `-D warnings` over all targets, `cargo fmt --check`, `cargo check --all-targets`, `cargo doc -D warnings`, `make check-api-examples` | all clean | PASS |

I did not re-run the full workspace test suite. I took the orchestrator's figure (4,706 passed, 2 sandbox-baseline failures) and the SUMMARY's 6,907 as reported. The 2 baseline failures are documented in 48-RESEARCH.md Pitfall 10 as failing on the unmodified tree.

### Probe Execution

No phase-declared probes. `make check-migration-allowlist` is the nearest runnable gate and it passes.

### Requirements Coverage

Every ID in the PLAN frontmatter appears in REQUIREMENTS.md, and no Phase 48 ID is orphaned. The plan frontmatter (48-01..48-09) claims ENTITY-01..05. REQUIREMENTS.md lists ENTITY-01..05 as Phase 48, Complete.

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| ENTITY-01 | `Entity<T>` with `data`, seven types built on it, serde alias, fixture proof | SATISFIED | Truth 1 |
| ENTITY-02 | Old name's fate decided and recorded (clean break and ADR) | SATISFIED | Truth 2 |
| ENTITY-03 | One `NodeError`, one `Paladin` alias, `EntityVersion*` | SATISFIED | Truth 3 |
| ENTITY-04 | Five dormant items removed or kept with a reason; `version` contract in rustdoc | SATISFIED (retained pending review, per D-05) | Truth 4 |
| ENTITY-05 | Graph freeze, vocabulary ADR and docs, guard, register, baseline, gates | SATISFIED; gates not fully confirmable here | Truth 5 |

### Owner Decisions (48-CONTEXT.md) Honoured

- D-01 and D-02 (name `Entity`, clean break): confirmed.
- D-03 (read-alias, no version bump): confirmed.
- D-04 (rustdoc contract only): confirmed.
- D-05 (keep all five): confirmed. Nothing removed, and the guard does not ban their paths.
- D-07 (graph pins): confirmed.
- D-08 (`entity` field): confirmed.
- D-09 (`EntityVersion*`): confirmed.
- D-10 (citadel unification): confirmed.
- D-12 (one ADR): confirmed.
- D-13 (private `wrapper.rs`): confirmed.
- D-14 (register once): the register is in 7fea514. Per the SUMMARY, the D-27 diagnostic measured 11 lints in the crate, 6 lint ids on Phase 48 items. I did not re-run the semver diagnostic.
- D-00d (zero dependency change): `Cargo.lock` is unchanged.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `base/service/field_version_service.rs` (`node_id` fields, `get_node_field_versions`) and `base/entity/field.rs` (`Field.nid`) | Entity-sense "node" identifiers remain | Info | ADR-0060 (g) records these as a bounded exception inside the retained items. D-05 says the five files change only as the sweep forces. The guard does not ban them. The review todo lists them. This is the one place the literal goal sentence ("exactly one thing") is not fully true in identifiers. It is an owner-sanctioned, written, deferred residual, not a gap. |
| `tests/entity_rename_guard.rs` rule 5 | Schema requirement is skipped when the tree has no `docs/` | Info | A deliberate fix for the Docker build context. The byte-identical freeze is separately proved by `git diff`, and I confirmed the schema is unchanged. |
| debt markers (TBD, FIXME, XXX) | None found in files this phase changed | None | |

### Human Verification Required

1. **CI result for the commits after dcbcb63.** Test: read the CI run for f092886. Expected: Docker Integration Tests and Coverage green. Why human: it was queued at verification time. Docker Integration Tests failed on dcbcb63, which is the guard regression that 2ad9fa6 fixes.
2. **`make security`.** Test: run it somewhere with cargo-audit and cargo-deny and read the Security Audit and License and Dependency Policy job logs. Expected: pass, or a failure that predates Phase 48. Why human: the tools are absent here. Both CI jobs are red, but per the SUMMARY they were already red on Phase 44 and earlier commits. `Cargo.lock` is byte-identical to the phase base, so Phase 48 added no dependency change.
3. **Coverage percentage.** Test: read it from the CI Coverage log. Expected: at least 82 %. Why human: I could not measure it here. The only evidence is the Coverage job conclusion on dcbcb63 (success).

Other items outside the phase's scope that the SUMMARY records: the `pre-commit` workflow is red (also on dcbcb63) and its cause is unread. Both are reported honestly in 48-09-SUMMARY.

### Gaps Summary

There are no code gaps. All five success criteria are met by what is in the tree, and I re-ran the guards, the compatibility tests, the register gate, clippy, fmt, doc and the examples gate myself. The status is `human_needed`, not `passed`, only because three acceptance items that ROADMAP SC5 and the project security rules name could not be confirmed in this sandbox: the CI outcome after the guard fix, `make security`, and the numeric coverage floor. 48-09 recorded these as gaps and did not claim them, and that is accurate.

---

_Verified: 2026-10-14_
_Verifier: Claude (gsd-verifier)_

## Human verification closure (2026-10-14)

All three `human_verification` items were resolved by `/gsd-verify-work 48` on 2026-10-14 (see
`48-UAT.md`, 3/3 pass). In short: the Docker Integration Tests and Coverage jobs are green on
c73476eb, 440896a6 and 2388fc13 (all after the 2ad9fa6 guard fix) and the pre-commit workflow is
green on every commit since 0a20e7ba; `make security` exits 0 locally on HEAD e3075023 and the
Security Audit and License & Dependency Policy jobs are green on 2388fc13 (run 38585690891); the
Coverage job passes `--fail-under-lines 82` on the same run, so workspace line coverage is at or
above 82 % (the exact figure is in that run's "Coverage summary" step; the artifact host is not
reachable from the sandbox). Frontmatter `status` moved `human_needed` → `passed`.
