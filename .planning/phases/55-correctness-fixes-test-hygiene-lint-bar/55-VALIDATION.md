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
| TBD | TBD | TBD | FIXES-01 | T-55-xx | The guard fails on a planted `todo!()` in production code and passes a stub inside `#[cfg(test)]`, a `///` example, a string literal and an out-of-line `#[cfg(test)] mod` file; the real tree reads 26 lines before the fixes and 0 after; it prints only `file:line` | shell harness | `./tests/scripts/check-no-stub-panics_test.sh`; `./scripts/check-no-stub-panics.sh` | ❌ W0 (new) | ⬜ pending |
| TBD | TBD | TBD | FIXES-01 | T-55-xx | Every `ContentListRepository` method on both backends returns `Err(RepositoryError::QueryError(..))` naming the method and backend; nothing panics | unit, feature-gated | `cargo test -p paladin-storage --features sqlite --offline content_list`; `cargo test -p paladin-storage --features mysql --offline content_list` | ❌ W0 (new tests) | ⬜ pending |
| TBD | TBD | TBD | FIXES-01 | — | The guard is wired into `make check-gates` and CI | grep | `make -n check-gates \| grep -q check-no-stub-panics.sh`; `grep -n 'check-no-stub-panics.sh' .github/workflows/ci.yml` | n/a | ⬜ pending |
| TBD | TBD | TBD | FIXES-02 | T-55-xx | Versioning fences return existing `VersioningError` variants or the documented nil-UUID sentinel; `ingest_content` refuses (or logs once, per the review file); no fabricated `expected_output` sentence appears in a produced plan | unit | `cargo test -p paladin-ai-core --offline base::`; `cargo test -p paladin-content --offline file_content_fetcher`; `cargo test -p paladin-ai --features web-server --lib --offline planning_service` | edit existing + new asserts | ⬜ pending |
| TBD | TBD | TBD | FIXES-02 | — | The vocabulary freeze is unaffected by the base-layer rustdoc and fence edits | integration | `cargo test -p paladin-ai --test entity_rename_guard --offline` | ✅ | ⬜ pending |
| TBD | TBD | TBD | FIXES-03 | T-55-xx | `DispatchBudget::Unpriced` on an agent-kind run records `halted` with `halt_reason.reason == "model_unpriced"` on the row, the SSE terminal event and the webhook payload; no LLM call is made | unit | `cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced` (renamed to the halt wording) | edit existing (`worker_tests.rs`) | ⬜ pending |
| TBD | TBD | TBD | FIXES-03 | T-55-xx | `HaltReason::ModelUnpriced { model }` serde round-trips as `{"reason":"model_unpriced","model":"…"}`, `as_str` is `model_unpriced`, the herald line names the model and never a tenant or key | unit + doctest | `cargo test -p paladin-ai-core --offline allowance::`; `cargo test -p paladin-ai-core --offline --doc` | edit existing | ⬜ pending |
| TBD | TBD | TBD | FIXES-03 | — | The committed OpenAPI document matches the router after the halt-reason description change | golden | `cargo test -p paladin-web --lib --offline openapi_matches_committed_baseline` (`make openapi` to regenerate) | ✅ | ⬜ pending |
| TBD | TBD | TBD | FIXES-04 | — | FIXES-04 is re-homed with a dated reason: the deferred entry and the traceability row exist; `RunStatusValue` rustdoc and `eval-harness.md` carry the note | grep | `grep -n "FIXES-04" .planning/REQUIREMENTS.md`; `grep -n -i "cancelled" crates/paladin-eval/src/scenario.rs docs/src/user-guides/eval-harness.md` | n/a | ⬜ pending |
| TBD | TBD | TBD | FIXES-05 | — | The `claude-opus-4-8` seed row's `reasoning` flag matches the two probe records under the field's documented contract (stays `false`; comment corrected) and is pinned | unit | `cargo test -p paladin-llm --no-default-features --features anthropic --offline anthropic_opus_4_8_seed_matches_the_transcript` (or the planner's test name) | edit existing | ⬜ pending |
| TBD | TBD | TBD | FIXES-06 | T-55-xx | The pure unpriced-reachable-models function: no Treasurer → empty; Treasurer but no ceiling → empty; priced → empty; unpriced → one entry per distinct model, sorted; the rendered line names the model and the uncovered classes, never a key | unit | `cargo test -p paladin-ai --features web-server --lib --offline unpriced_reachable` | ❌ W0 (new) | ⬜ pending |
| TBD | TBD | TBD | TESTS-01 | T-55-xx | Both `build_run_api_persists_*` tests pass with the network off and no `OPENAI_API_KEY`; the scripted port answers in-process | unit | `env -u HTTPS_PROXY -u https_proxy -u HTTP_PROXY -u http_proxy cargo test -p paladin-ai --features web-server --lib --offline build_run_api_persists` | edit existing | ⬜ pending |
| TBD | TBD | TBD | TESTS-02 | — | `local_cancel_signals_token` waits on the registration inspector, never a fixed sleep, and passes 20 of 20 runs under the full facade-binary load | unit, loop | the test run 20 times inside `cargo test -p paladin-ai --features web-server --lib --offline` (about 43 s per full-binary run) | edit existing | ⬜ pending |
| TBD | TBD | TBD | TESTS-03 | — | `cargo clippy --tests` is warning-free for every single feature of `paladin-llm`, `mock` alone included | lint | the per-feature loop `cargo clippy -p paladin-llm --tests --no-default-features --features <f> -- -D warnings` over every feature in `crates/paladin-llm/Cargo.toml` (about 35 s each warm) | n/a | ⬜ pending |
| TBD | TBD | TBD | TESTS-04 | — | The four Phase 51 lint sites are clean under default features and under both doc bars | lint + doc | the clippy loop above plus `cargo doc -p paladin-battalion -p paladin-llm --no-deps` with zero `warning:` lines | n/a | ⬜ pending |
| TBD | TBD | TBD | TESTS-05 | — | `SELF_HOSTED_PROVIDERS` is the only definition of the pair; `layers.rs` imports it; both call sites see the same slice | unit | `cargo test -p paladin-llm --offline layers` plus the new same-slice test; `grep -rn 'REASONING_WIRE_PROVIDERS' crates/paladin-llm/src` returns nothing | ❌ W0 (new test) | ⬜ pending |
| TBD | TBD | TBD | TESTS-06 | — | The four commander tests still pass and each points at `tests/integration/commander_error_paths_test.rs`; no `TODO: Enhance MockPaladinPort` remains | unit + grep | `cargo test -p paladin-battalion --lib --offline test_error_handling test_partial_failure_handling`; `grep -n 'Enhance MockPaladinPort' crates/paladin-battalion/src/commander.rs` returns nothing | edit existing | ⬜ pending |
| TBD | TBD | TBD | CIREL-08 | — | Zero diagnostics under both bars with all eight allows removed; no `#[allow(rustdoc::` at crate or item level remains outside the ports `warn` line | doc build + grep | `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`; `grep -rn 'allow(rustdoc' crates/*/src src --include=*.rs` returns nothing | n/a | ⬜ pending |
| TBD | TBD | TBD | CIREL-08 | — | `docs/src/appendix/doc-coverage-report.md` is the dated measurement record (banner, bar commands, results at a named SHA, the removed-allow table) | grep | `grep -n 'Regenerated' docs/src/appendix/doc-coverage-report.md`; `make check-doc-config` | n/a | ⬜ pending |
| TBD | TBD | TBD | all | — | Phase gate: every public change reads additive at `--release-type patch` against `0.11.0` for all twelve packages | semver | `./scripts/check-semver.sh` (expected `223 checks: 223 pass, 31 skip` per package, `0 failure(s)`), recorded in `55-CORRECTNESS-EVIDENCE.md` | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/check-no-stub-panics.sh` and `tests/scripts/check-no-stub-panics_test.sh` with fixtures built under `mktemp -d` and a test-only root override variable (the `VERSION_STRINGS_ROOT` pattern); the red run against the real tree (26 lines, 3 files) captured before the stub fixes
- [ ] Feature-gated table tests in `crates/paladin-storage/src/sqlite_content_repository.rs` and `mysql_content_repository.rs` (watched failing: today each method panics)
- [ ] `unpriced_reachable_models` plus its renderer and table tests in `src/infrastructure/web/run_api_wiring.rs`
- [ ] A scripted-port double and the `pub(crate)` seam for the two hermetic tests
- [ ] A same-slice test for `SELF_HOSTED_PROVIDERS`
- [ ] `## [Unreleased]` anchors in the crate changelogs whose behaviour changes (`paladin-storage`, `paladin-ai-core`, `paladin-web`, the facade; `paladin-llm` only if a behaviour changes)

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
