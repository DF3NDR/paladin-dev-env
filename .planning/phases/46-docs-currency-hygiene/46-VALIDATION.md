---
phase: 46
slug: docs-currency-hygiene
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-24
---

# Phase 46 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (libtest; `serde_json` fixtures), bash harnesses `tests/scripts/*_test.sh` (run by `make test-shell-guards`), repo guards as `cargo test --test <guard>`, offline script gates run by `make`, `gsd-tools windows status` for the ledger |
| **Config file** | workspace `Cargo.toml` (`[[test]] consumer_surface_muster`, default features carry `llm-anthropic`, `llm-deepseek`); `Makefile`; none dedicated for docs |
| **Quick run command** | `make check-migration-allowlist check-doc-config check-changelogs check-gates && make test-shell-guards` plus the task's own `<automated>` command |
| **Full suite command** | `make clean-code && make api-surface && make check-gates && make check-doc-config && make check-doc-examples && make check-api-examples && make check-consumer-surface && cargo test --offline --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard && cargo test --offline -p paladin-ai-core --lib battalion && cargo test --offline -p paladin-battalion --lib commander`; `make security` where the advisory DB is reachable |
| **Estimated runtime** | seconds for the script gates and greps; ~1-3 min warm for a single-crate `cargo test`; several minutes for `make clean-code` |

---

## Sampling Rate

- **After every task commit:** the task's own `<automated>` verify (every task has one; no task relies on a later task's check), plus `cargo fmt --check` and `cargo clippy --offline -p <crate> --all-targets -- -D warnings` when a `.rs` file changed, and `make lint-shell` when a script changed
- **After every plan wave:** the quick run command; after waves 2-3 (code) also `make clean-code`, `make api-surface`, `make check-consumer-surface`
- **Before `/gsd-verify-work`:** the full suite command green (plan 46-13 `<verification>`), then CI `coverage` (82 %, ADR-0006), `semver`, `docs` (mdBook + linkcheck) and `consumer-surface` green
- **Max feedback latency:** ~3 min for a single-crate test; seconds for docs and ledger tasks

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 46-01-01 | 01 | 1 | CURR-22 | T-46-01, T-46-02 | paladin-eval row's evidence is its publish; history amended, not rewritten | grep gate | Task 1 verify (greps + `make check-doc-config`) | ✅ | ⬜ pending |
| 46-01-02 | 01 | 1 | CURR-22 | — | N/A | grep gate | Task 2 verify (greps) | ✅ | ⬜ pending |
| 46-01-03 | 01 | 1 | CURR-22 | — | N/A | script | heading-date loop over 12 changelogs vs `git log -1 --format=%cs v0.10.x` + `make check-changelogs` | ✅ | ⬜ pending |
| 46-02-01 | 02 | 1 | CURR-24 | T-46-03, T-46-04 | flag set only by evidence; no test/impl file written | grep (frontmatter, row count) | Task 1 verify (status, `reconciled:`, rows ≥ task count, no placeholder rows) | ✅ | ⬜ pending |
| 46-02-02 | 02 | 1 | CURR-24 | T-46-03, T-46-04 | as above | grep | Task 2 verify | ✅ | ⬜ pending |
| 46-03-01 | 03 | 1 | CURR-24 | T-46-03 | promotion cites UAT 2/2 and WINDOWS 35/61 | grep | Task 1 verify (`nyquist_compliant: true`, no unchecked box) | ✅ | ⬜ pending |
| 46-03-02 | 03 | 1 | CURR-24 | T-46-03, T-46-04 | as 46-02 | grep | Task 2 verify | ✅ | ⬜ pending |
| 46-03-03 | 03 | 1 | CURR-24 | T-46-03, T-46-04 | as 46-02 | grep | Task 3 verify (+ no `draft` among 28/29/30) | ✅ | ⬜ pending |
| 46-04-01 | 04 | 1 | CURR-24 | T-46-03, T-46-04 | as 46-02 | grep | Task 1 verify | ✅ | ⬜ pending |
| 46-04-02 | 04 | 1 | CURR-24 | T-46-03, T-46-04 | as 46-02 | grep | Task 2 verify | ✅ | ⬜ pending |
| 46-04-03 | 04 | 1 | CURR-24 | T-46-03, T-46-04 | as 46-02 | grep | Task 3 verify (+ no `draft` among 34/36/36.1) | ✅ | ⬜ pending |
| 46-05-01 | 05 | 1 | CURR-25 | T-46-05, T-46-06, T-46-08 | a surplus / renamed / build / target dependency fails the gate in make and CI | shell harness + gates | `bash tests/scripts/check-dependency-allowlist_test.sh && make check-gates && make test-shell-guards && make lint-shell` | ❌ W0 (harness written red first in the task) | ⬜ pending |
| 46-05-02 | 05 | 1 | CURR-25 | T-46-07 | ledger written only through the tool, sequentially | tool + grep | `gsd-tools windows status` assertion (3 Phase 46 rows: 2 waived, 1 fixed) + STATE/ROADMAP greps | ✅ | ⬜ pending |
| 46-06-01 | 06 | 1 | CURR-23 | T-46-09 | N/A | compile-only test | `make check-consumer-surface` (red proof recorded) | ✅ | ⬜ pending |
| 46-06-02 | 06 | 1 | CURR-23 | T-46-09 | N/A | compile-only test + clippy | `make check-consumer-surface && cargo clippy --offline --test consumer_surface_muster -- -D warnings` | ✅ | ⬜ pending |
| 46-07-01 | 07 | 1 | CURR-23 | T-46-11, T-46-13 | no DNS-rebinding claim; no admin bypass | doc gates + guards | Task 1 verify (names, nav, link targets, ADR index rows, `make check-doc-config check-doc-examples`, 3 guards) | ✅ | ⬜ pending |
| 46-07-02 | 07 | 1 | CURR-23 | T-46-10, T-46-12 | webhook secret env-only; no secret in YAML | doc gates + key-coverage script + example-config tests | Task 2 verify (key coverage vs `src/config/treasurer.rs`, `cargo test --offline -p paladin-ai --lib example_config`) | ✅ | ⬜ pending |
| 46-08-01 | 08 | 2 | CURR-23 | T-46-14, T-46-15, T-46-16 | removed keys refused naming key only; unknown keys load | unit (tdd) | `cargo test --offline -p paladin-ai-core --lib battalion && cargo test --offline --test legacy_removal_guard && make api-surface` | ❌ W0 (tests written red first) | ⬜ pending |
| 46-08-02 | 08 | 2 | CURR-23 | T-46-14 | nested refusal; CLI YAML key still loads | unit + CLI unit | `cargo test --offline -p paladin-ai --features cli --lib cli_conclave_yaml_timeout_seconds_still_loads` + core battalion tests | ❌ W0 (tests written red first) | ⬜ pending |
| 46-09-01 | 09 | 3 | CURR-23 | T-46-17 | an ignored Maneuver policy is warned once | unit (tdd) | `cargo test --offline -p paladin-battalion --lib maneuver_ignored_aegis_policies` | ❌ W0 (test written red first) | ⬜ pending |
| 46-09-02 | 09 | 3 | CURR-23 | T-46-18 | no semver allow line changed | gates | `make check-migration-allowlist && make doc-check && cargo test --offline --test legacy_removal_guard` | ✅ | ⬜ pending |
| 46-10-01 | 10 | 4 | CURR-23 | T-46-19, T-46-20 | a 10.2 register row is read by the semver gate | shell harness + gate | `bash tests/scripts/check-migration-allowlist_test.sh && make check-migration-allowlist && make test-shell-guards` | ❌ W0 (new harness cases) | ⬜ pending |
| 46-10-02 | 10 | 4 | CURR-23 | T-46-21 | move is verbatim | diff vs `v0.10.1` tag + gate | part-9 `diff` against `git show v0.10.1:MIGRATION.md` + `make check-migration-allowlist` | ✅ | ⬜ pending |
| 46-11-01 | 11 | 5 | CURR-23 | T-46-20 | N/A | grep + diff + gate | Task 1 verify | ✅ | ⬜ pending |
| 46-11-02 | 11 | 5 | CURR-23 | T-46-22, T-46-23 | both timeout outcomes stated | grep + gate + guards | Task 2 verify | ✅ | ⬜ pending |
| 46-11-03 | 11 | 5 | CURR-23 | T-46-20 | N/A | grep + gates + guards | Task 3 verify | ✅ | ⬜ pending |
| 46-12-01 | 12 | 5 | CURR-23 | T-46-24 | history untouched | count + diff | Task 1 verify (≥ 30 section-10 citations; older sections byte-identical) | ✅ | ⬜ pending |
| 46-12-02 | 12 | 5 | CURR-23 | T-46-25 | no allowlist value changed | diff filter + gates | Task 2 verify | ✅ | ⬜ pending |
| 46-12-03 | 12 | 5 | CURR-23 | T-46-24 | comment-only `.rs` changes | diff filter + build + guards | Task 3 verify (`cargo check --workspace --all-targets --all-features`, `make doc-check`, muster) | ✅ | ⬜ pending |
| 46-13-01 | 13 | 6 | CURR-23 | T-46-26 | page indexes, does not duplicate | grep + doc gate + guards | Task 1 verify | ✅ | ⬜ pending |
| 46-13-02 | 13 | 6 | CURR-23 | T-46-27 | vendor fact dated and attributed | grep + doc gate + guards | Task 2 verify | ✅ | ⬜ pending |
| 46-13-03 | 13 | 6 | CURR-24 | T-46-28 | 05-21 item stays open | grep | Task 3 verify (+ eight VALIDATION files not `draft`) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/check-dependency-allowlist.sh` + `tests/scripts/check-dependency-allowlist_test.sh` -- CURR-25 (plan 46-05 Task 1, harness red first)
- [ ] `crates/paladin-core/src/platform/container/battalion/legacy_keys.rs` tests + the inverted `battalion_config_v0_10_document_is_refused_naming_each_removed_key` -- D-23 (plan 46-08, red first)
- [ ] `cli_conclave_yaml_timeout_seconds_still_loads` in `src/application/cli/config/battalion_config.rs` -- D-23 CLI constraint (plan 46-08 Task 2)
- [ ] `maneuver_ignored_aegis_policies_names_only_the_fields_maneuver_drops` in `commander.rs` -- D-24 (plan 46-09, red first)
- [ ] the four `## 10.2` cases in `tests/scripts/check-migration-allowlist_test.sh` -- F1 (plan 46-10 Task 1)
- Framework install: none -- every new check is a file inside existing infrastructure; no package is installed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Rendered mdBook navigation and `mdbook-linkcheck` (Treasurer page, Upgrading page, SUMMARY entry) | CURR-23 | `mdbook` / `mdbook-linkcheck` are not installed in the sandbox; relative targets are checked with `test -f` in plan 46-07 / 46-13 verifies | CI `docs` workflow green on the phase branch |
| GitHub anchors for `MIGRATION.md` section 10 (`#101-...` .. `#108-...`) | CURR-23 | web links are not link-checked (`follow-web-links = false`); slugs follow the rule the existing `#92-...` links use (RESEARCH A1) | open `MIGRATION.md` on `main` after merge and follow one link from `upgrading.md` and one from `CHANGELOG.md` |
| 82 % workspace line coverage after D-23 / D-24 | CURR-23 | no `cargo-llvm-cov` in the sandbox | CI `coverage` job green |
| web3sec's re-pin on `v0.11.0` and its return report | CURR-23 (D-25) | belongs to the consumer after Phase 47 | tracked in `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (32 tasks, each with its own command)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (five in-phase test files, each written red first)
- [x] No watch-mode flags
- [x] Feedback latency < 180 s for per-task checks
- [ ] `nyquist_compliant: true` set in frontmatter (set by `/gsd-validate-phase 46` after execution)

**Approval:** pending
