---
phase: 55
slug: correctness-fixes-test-hygiene-lint-bar
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-01
---

# Phase 55 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `55-RESEARCH.md` "Validation Architecture". The per-task map below is
> keyed by requirement; the planner's PLAN.md `<verify>` blocks fill in the task and plan ids, and
> `/gsd-validate-phase` seals it.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust libtest (`cargo test`), `#[tokio::test]`, `serial_test` 3.2.0; bash regression harnesses in `tests/scripts/*_test.sh` (auto-discovered by `make test-shell-guards`); `cargo doc` under the two ADR-0033 bars; `cargo clippy` per feature |
| **Config file** | none: the workspace `Cargo.toml`, per-crate features (`web-server` for the run API tests), the `Makefile` targets |
| **Quick run command** | `cargo test -p paladin-ai --features web-server --lib --offline <filter>` (facade rebuild about 1 min warm, tests under 1 s each); `./tests/scripts/check-no-stub-panics_test.sh`; `cargo test -p paladin-ai-core --offline allowance::` |
| **Full suite command** | `cargo test --workspace --no-fail-fast`, then `make test-shell-guards`, `make check-gates`, `make clean-code` (includes `make doc-check`: default bar about 4 min cold, all-features about 2 min, then `cargo test --workspace --doc`), `make api-surface`, `make security`, `make check-migration-allowlist`, `make check-version-strings`, `make check-doc-config`, `./scripts/check-semver.sh` |
| **Estimated runtime** | about 1-2 minutes for a quick command on a warm cache; about 15 minutes for the workspace test run; 10-15 minutes for the twelve-package semver run |

---

## Sampling Rate

- **After every task commit:** Run the narrow command from the map for that requirement, plus `cargo fmt --check` and `cargo clippy -p <touched crate> -- -D warnings`; `shellcheck` on any script touched
- **After every plan wave:** Run `make test-shell-guards`, `make check-gates`, the touched crates' tests, and both rustdoc bars after any doc-comment edit
- **Before `/gsd-verify-work`:** Full suite must be green: the D-00a set including `make clean-code`, `make api-surface`, `make security`, `./scripts/check-semver.sh` (twelve lines recorded) and the manual credential review of every new log line and error message
- **Max feedback latency:** 120 seconds for the quick commands

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| `55-01-01`, `55-01-02` | 55-01 | 1 | FIXES-01 | T-55-02, T-55-04 | The guard fails on a planted `todo!()` in production code and passes a stub inside `#[cfg(test)]`, a `///` example, a string literal and an out-of-line `#[cfg(test)] mod` file; the real tree reads 26 lines before the fixes and 0 after; it prints only `file:line` | shell harness | `./tests/scripts/check-no-stub-panics_test.sh`; `./scripts/check-no-stub-panics.sh` | ❌ W0 (new) | ⬜ pending |
| `55-01-02` | 55-01 | 1 | FIXES-01 | T-55-01, T-55-03 | Every `ContentListRepository` method on both backends returns `Err(RepositoryError::QueryError(..))` naming the method and backend; nothing panics | unit, feature-gated | `cargo test -p paladin-storage --features sqlite --offline content_list`; `cargo test -p paladin-storage --features mysql --offline content_list` | ❌ W0 (new tests) | ⬜ pending |
| `55-01-03` | 55-01 | 1 | FIXES-01 | — | The guard is wired into `make check-gates` and CI | grep | `make -n check-gates \| grep -q check-no-stub-panics.sh`; `grep -n 'check-no-stub-panics.sh' .github/workflows/ci.yml` | n/a | ⬜ pending |
| `55-05-01`, `55-05-02`, `55-06-01`, `55-06-02` | 55-05, 55-06 | 1 | FIXES-02 | T-55-13, T-55-15, T-55-16 | Versioning fences return existing `VersioningError` variants or the documented nil-UUID sentinel; `ingest_content` refuses (or logs once, per the review file); no fabricated `expected_output` sentence appears in a produced plan | unit | `cargo test -p paladin-ai-core --offline base::`; `cargo test -p paladin-content --offline file_content_fetcher`; `cargo test -p paladin-ai --features web-server --lib --offline planning_service` | edit existing + new asserts | ⬜ pending |
| `55-05-03` | 55-05 | 1 | FIXES-02 | T-55-14 | The vocabulary freeze is unaffected by the base-layer rustdoc and fence edits | integration | `cargo test -p paladin-ai --test entity_rename_guard --offline` | ✅ | ⬜ pending |
| `55-02-01` | 55-02 | 1 | FIXES-03 | T-55-06 | `DispatchBudget::Unpriced` on an agent-kind run records `halted` with `halt_reason.reason == "model_unpriced"` on the row, the SSE terminal event and the webhook payload; no LLM call is made | unit | `cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced` (renamed to the halt wording) | edit existing (`worker_tests.rs`) | ⬜ pending |
| `55-02-01` | 55-02 | 1 | FIXES-03 | T-55-05 | `HaltReason::ModelUnpriced { model }` serde round-trips as `{"reason":"model_unpriced","model":"…"}`, `as_str` is `model_unpriced`, the herald line names the model and never a tenant or key | unit + doctest | `cargo test -p paladin-ai-core --offline allowance::`; `cargo test -p paladin-ai-core --offline --doc` | edit existing | ⬜ pending |
| `55-02-02` | 55-02 | 1 | FIXES-03 | T-55-07 | The committed OpenAPI document matches the router after the halt-reason description change | golden | `cargo test -p paladin-web --lib --offline openapi_matches_committed_baseline` (`make openapi` to regenerate) | ✅ | ⬜ pending |
| `55-07-01`, `55-07-02` | 55-07 | 1 | FIXES-04 | T-55-17, T-55-18 | FIXES-04 is re-homed with a dated reason: the deferred entry and the traceability row exist; `RunStatusValue` rustdoc and `eval-harness.md` carry the note | grep | `grep -n "FIXES-04" .planning/REQUIREMENTS.md`; `grep -n -i "cancelled" crates/paladin-eval/src/scenario.rs docs/src/user-guides/eval-harness.md` | n/a | ⬜ pending |
| `55-04-03` | 55-04 | 1 | FIXES-05 | T-55-12 | The `claude-opus-4-8` seed row's `reasoning` flag matches the two probe records under the field's documented contract (stays `false`; comment corrected) and is pinned | unit | `cargo test -p paladin-llm --no-default-features --features anthropic --offline anthropic_opus_4_8_seed_matches_the_transcript` (or the planner's test name) | edit existing | ⬜ pending |
| `55-09-02` | 55-09 | 2 | FIXES-06 | T-55-21, T-55-22, T-55-24 | The pure unpriced-reachable-models function: no Treasurer → empty; Treasurer but no ceiling → empty; priced → empty; unpriced → one entry per distinct model, sorted; the rendered line names the model and the uncovered classes, never a key | unit | `cargo test -p paladin-ai --features web-server --lib --offline unpriced_reachable` | ❌ W0 (new) | ⬜ pending |
| `55-09-01` | 55-09 | 2 | TESTS-01 | T-55-20, T-55-23 | Both `build_run_api_persists_*` tests pass with the network off and no `OPENAI_API_KEY`; the scripted port answers in-process | unit | `env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` | edit existing | ⬜ pending |
| `55-03-01` | 55-03 | 1 | TESTS-02 | T-55-09 | `local_cancel_signals_token` waits on the registration inspector, never a fixed sleep, and passes 20 of 20 runs under the full facade-binary load | unit, loop | the test run 20 times inside `cargo test -p paladin-ai --features web-server --lib --offline` (about 43 s per full-binary run) | edit existing | ⬜ pending |
| `55-04-01`, `55-04-02` | 55-04 | 1 | TESTS-03 | T-55-11 | `cargo clippy --tests` is warning-free for every single feature of `paladin-llm`, `mock` alone included | lint | the per-feature loop `cargo clippy -p paladin-llm --tests --no-default-features --features <f> -- -D warnings` over every feature in `crates/paladin-llm/Cargo.toml` (about 35 s each warm) | n/a | ⬜ pending |
| `55-04-01`, `55-04-02` | 55-04 | 1 | TESTS-04 | T-55-11 | The four Phase 51 lint sites are clean under default features and under both doc bars | lint + doc | the clippy loop above plus `cargo doc -p paladin-battalion -p paladin-llm --no-deps` with zero `warning:` lines | n/a | ⬜ pending |
| `55-04-03` | 55-04 | 1 | TESTS-05 | — | `SELF_HOSTED_PROVIDERS` is the only definition of the pair; `layers.rs` imports it; both call sites see the same slice | unit | `cargo test -p paladin-llm --offline layers` plus the new same-slice test; `grep -rn 'REASONING_WIRE_PROVIDERS' crates/paladin-llm/src` returns nothing | ❌ W0 (new test) | ⬜ pending |
| `55-03-02` | 55-03 | 1 | TESTS-06 | T-55-10 | The four commander tests still pass and each points at `tests/integration/commander_error_paths_test.rs`; no `TODO: Enhance MockPaladinPort` remains | unit + grep | `cargo test -p paladin-battalion --lib --offline test_error_handling test_partial_failure_handling`; `grep -n 'Enhance MockPaladinPort' crates/paladin-battalion/src/commander.rs` returns nothing | edit existing | ⬜ pending |
| `55-08-01`, `55-08-02`, `55-10-01`, `55-10-02`, `55-11-01`, `55-11-02` | 55-08, 55-10, 55-11 | 1, 2, 3 | CIREL-08 | T-55-19, T-55-25 | Zero diagnostics under both bars with all eight allows removed; no `#[allow(rustdoc::` at crate or item level remains outside the ports `warn` line | doc build + grep | `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`; `grep -rn 'allow(rustdoc' crates/*/src src --include=*.rs` returns nothing | n/a | ⬜ pending |
| `55-11-03` | 55-11 | 3 | CIREL-08 | T-55-26, T-55-27 | `docs/src/appendix/doc-coverage-report.md` is the dated measurement record (banner, bar commands, results at a named SHA, the removed-allow table) | grep | `grep -n 'Regenerated' docs/src/appendix/doc-coverage-report.md`; `make check-doc-config` | n/a | ⬜ pending |
| `55-13-03` | 55-13 | 4 | all | T-55-32 | Phase gate: every public change reads additive at `--release-type patch` against `0.11.0` for all twelve packages | semver | `./scripts/check-semver.sh` (expected `223 checks: 223 pass, 31 skip` per package, `0 failure(s)`), recorded in `55-CORRECTNESS-EVIDENCE.md` | n/a | ⬜ pending |
| `55-12-01`, `55-12-02` | 55-12 | 3 | FIXES-01, FIXES-02, FIXES-03, FIXES-06 | T-55-28 | Every behaviour change has a `MIGRATION.md` §11.1 row (`M-B-30`..`M-B-35`) and an `[Unreleased]` changelog entry; `paladin-storage` gains its anchor and the release tool stamps it; frozen `[0.11.0]` sections and §9/§10 are untouched; §11.2 stays header-only | grep + gate | `make check-changelogs`; `make check-migration-allowlist`; `grep -c '^\| M-B-3[0-5] \|' MIGRATION.md` | n/a | ⬜ pending |
| `55-12-03` | 55-12 | 3 | FIXES-02 | T-55-29 | `55-BASE-LAYER-REVIEW.md` has eight item rows with intent, state, decision and test evidence; the 2026-10-13 todo is in `completed/` with a `## Resolution` | grep | `grep -c '^\| ' .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-BASE-LAYER-REVIEW.md`; `test -f .planning/todos/completed/2026-10-13-review-content-data-orchestration-base-items.md` | ❌ W0 (new) | ⬜ pending |
| `55-13-01`, `55-13-02` | 55-13 | 4 | all | T-55-30, T-55-31, T-55-33 | `55-CORRECTNESS-EVIDENCE.md` holds a re-run head row per requirement; WINDOWS row 73 `fixed` and row 67 `waived` with test names; ledger counters consistent | grep + ledger | `grep -cE '^\| (FIXES\|TESTS\|CIREL)-' .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CORRECTNESS-EVIDENCE.md`; `node .claude/gsd-core/bin/gsd-tools.cjs windows status` | ❌ W0 (new) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] (55-01-01) `scripts/check-no-stub-panics.sh` and `tests/scripts/check-no-stub-panics_test.sh` with fixtures built under `mktemp -d` and a test-only root override variable (the `VERSION_STRINGS_ROOT` pattern); the red run against the real tree (26 lines, 3 files) captured before the stub fixes
- [ ] (55-01-02) Feature-gated table tests in `crates/paladin-storage/src/sqlite_content_repository.rs` and `mysql_content_repository.rs` (watched failing: today each method panics)
- [ ] (55-09-02) `unpriced_reachable_models` plus its renderer and table tests in `src/infrastructure/web/run_api_wiring.rs`
- [ ] (55-09-01) A scripted-port double and the `pub(crate)` seam for the two hermetic tests
- [ ] (55-04-03) A same-slice test for `SELF_HOSTED_PROVIDERS`
- [ ] (55-12-01, 55-12-02) `## [Unreleased]` anchors in the crate changelogs whose behaviour changes: only `paladin-storage` lacks one today (`paladin-ai-core`, `paladin-content`, `paladin-web` and the root already have it; `paladin-llm` owes none, since FIXES-05 changes no behaviour)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| No new log line or error message prints a key value, tenant id or backend text outside `log::error!` | FIXES-03, FIXES-06, FIXES-01 | The credential-handling review in `.github/instructions/security.instructions.md` is a read, not a lint | Read every added `log::warn!`, `Err(..)` message and herald line in the diff; confirm model name / method name / backend only |
| `55-BASE-LAYER-REVIEW.md` records a keep / fence / remove decision with evidence for each of the seven FIXES-02 items | FIXES-02 | A review record is judged by reading it | Open the file; one row per item with design intent, current state, decision, evidence pointer |
| WINDOWS rows 67 and 73 carry dispositions that cite the warning test and the two hermetic test names | FIXES-06, TESTS-01 | Register edit | `grep -n '^| 67 \|^| 73 ' .planning/WINDOWS.md` and read the status and reason columns |
| The 82 % workspace coverage floor and `actionlint` | all | `cargo-llvm-cov` and `actionlint` are not installed here | CI `Coverage` and `Workflow Lint` jobs on the pushed branch are the proof of record; record both as unrun locally |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
