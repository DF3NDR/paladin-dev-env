---
phase: 52
slug: ci-release-pipeline-repair
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-28
validated: 2026-10-30
---

# Phase 52 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Bash regression harnesses (`tests/scripts/*_test.sh`: stubbed `gh`/`cargo` via `GH_BIN` / `SEMVER_CARGO_BIN`, `mktemp -d` scratch, `FAILED`/`ASSERTIONS` accumulation, BEFORE/AFTER real-tree mutation watch) plus the offline gate scripts; `cargo semver-checks` 0.50.0 for the semver proof; CI (`Workflow Lint`, `Build MDBook`) for what the sandbox cannot run |
| **Config file** | none — `Makefile` targets `test-shell-guards` (glob discovery) and `check-gates` |
| **Quick run command** | `./tests/scripts/<touched-script>_test.sh && shellcheck --severity=warning <touched files>` |
| **Full suite command** | `make test-shell-guards && make check-gates` |
| **Estimated runtime** | ~52 seconds for the full suite; `./scripts/check-semver.sh` (all twelve crates, real tool) 5-20 minutes, run only in plans 52-01 (one crate), 52-05 (twelve) and 52-07 (three) |

---

## Sampling Rate

- **After every task commit:** Run the touched script's own `_test.sh`, `shellcheck --severity=warning` on touched scripts, and `make check-gates`
- **After every plan wave:** Run `make test-shell-guards && make check-gates`; after any wave that changed `ci.yml` or a `Cargo.toml`, also `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh`
- **Before `/gsd-verify-work`:** Full suite must be green, plus plan 52-08's D-00a gate run and a green CI run on the phase branch (H-08)
- **Max feedback latency:** 52 seconds (harness suite); the real-tool semver loop is a per-wave check, not a per-commit one

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 52-01-01 | 01 | 1 | CIREL-01 | T-52-01, T-52-02 | Zero checks or no check count never passes; package list only from `publish-crates.sh` | shell harness + real tool | `./tests/scripts/check-semver_test.sh && SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval` | ✅ (was W0: harness created red-first in this task) | ✅ green |
| 52-01-02 | 01 | 1 | CIREL-03 | T-52-05 | Allowlist gate runs from the required cargo-deny job | static (YAML) + gate | `./scripts/check-migration-allowlist.sh && ./tests/scripts/check-migration-allowlist_test.sh && ./scripts/check-workflow-triggers.sh` | ✅ | ✅ green |
| 52-02-01 | 02 | 1 | CIREL-05 | T-52-06, T-52-07 | Body never crosses argv; >131,072-byte body sent byte-identical | shell harness | `./tests/scripts/create-or-reuse-release_test.sh` | ✅ (Case 10 added red-first) | ✅ green |
| 52-02-02 | 02 | 1 | CIREL-05 | T-52-08, T-52-09, T-52-10 | Character-counted line-boundary fit; sizes logged, body text never logged | shell harness | `./tests/scripts/release-body-fit_test.sh` | ✅ (was W0: created red-first in this task) | ✅ green |
| 52-02-03 | 02 | 1 | CIREL-05 | T-52-07, T-52-08 | Over-limit body fitted before create; invalid limit fails before any API call | shell harness | `./tests/scripts/create-or-reuse-release_test.sh && make test-shell-guards` | ✅ | ✅ green |
| 52-03-01 | 03 | 1 | CIREL-06, CIREL-07 | T-52-12, T-52-14 | Facts read from the public API; no token-shaped string recorded | static (evidence) | `grep` checks in the task's verify (sections, URLs, negative token grep) | ✅ (was W0: evidence file created) | ✅ green |
| 52-03-02 | 03 | 1 | CIREL-06, CIREL-07 | T-52-15 | Maintainer-only acts handed over, not attempted twice | checkpoint (human-action) | — (reply recorded verbatim) | n/a | ✅ green (checkpoint recorded) |
| 52-03-03 | 03 | 1 | CIREL-06 | T-52-11, T-52-13 | Every alert dispositioned; register guard green; codeql.yml unchanged | gate + harness | `make check-codeql-dismissals && ./tests/scripts/check-codeql-dismissals_test.sh` | ✅ | ✅ green |
| 52-04-01 | 04 | 2 | CIREL-05 | T-52-16, T-52-17 | Composed body fitted under the limit; artifact tail byte-identical; two-run identity | shell harness | `./tests/scripts/finalize-release-body_test.sh` | ✅ (Cases 26-27 added red-first) | ✅ green |
| 52-04-02 | 04 | 2 | CIREL-05 | T-52-16, T-52-18 | Under-limit identity; named failure before `gh release edit` | shell harness | `./tests/scripts/finalize-release-body_test.sh && make test-shell-guards` | ✅ | ✅ green |
| 52-05-01 | 05 | 2 | CIREL-01, CIREL-02 | T-52-19, T-52-22 | No crate-wide allow; twelve non-zero check counts with no failing lint | static + real tool | `! grep -q 'cargo-semver-checks\.lints' Cargo.toml crates/*/Cargo.toml && SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` | ✅ | ✅ green |
| 52-05-02 | 05 | 2 | CIREL-02, CIREL-03 | T-52-20, T-52-21 | Gate reads §11.2 only; `## 11.3` terminator present; §9/§10 byte-frozen; no placeholder token | shell harness + gate | `./tests/scripts/check-migration-allowlist_test.sh && ./scripts/check-migration-allowlist.sh` | ✅ (cases 12a-12f added red-first) | ✅ green |
| 52-06-01 | 06 | 2 | CIREL-04 | T-52-23, T-52-24, T-52-25 | Label used only in a job-level `if:`; trigger register and ruleset record agree with ci.yml | static (YAML/JSON) + gate | `./scripts/check-workflow-triggers.sh && ./tests/scripts/check-workflow-triggers_test.sh && ./scripts/check-workflow-suppressions.sh` | ✅ | ✅ green |
| 52-06-02 | 06 | 2 | CIREL-04 | T-52-25, T-52-27 | Docs state the 46-context set and the skipped-job semantics | static (docs grep) | `grep` checks in the task's verify | ✅ | ✅ green |
| 52-06-03 | 06 | 2 | CIREL-09, CIREL-04 | T-52-27 | Publish-order reading, label act and size limit documented; release.yml unchanged | static (docs grep) | `grep` checks in the task's verify; `git diff --quiet HEAD -- .github/workflows/release.yml` | ✅ | ✅ green |
| 52-07-01 | 07 | 3 | CIREL-01 | T-52-29 | Real public-API break goes red on the broken crate only; tree restored | real tool + evidence | `grep` checks on `52-SEMVER-RED-EVIDENCE.md` and `git diff --quiet -- crates/` | ✅ (was W0: evidence file created) | ✅ green |
| 52-07-02 | 07 | 3 | CIREL-01 | T-52-30 | No push before the maintainer's answer | checkpoint (decision) | — | n/a | ✅ green (checkpoint recorded) |
| 52-07-03 | 07 | 3 | CIREL-01 | T-52-28, T-52-31 | Scratch break never reachable from the phase branch; PR closed, branch deleted or H-03 | CI observation + git | `grep` checks on the evidence; `git log` subject grep; `git show-ref` absent | ✅ | ✅ green |
| 52-08-01 | 08 | 4 | CIREL-01, CIREL-04, CIREL-06, CIREL-07 | T-52-32, T-52-34 | Every gated act is an explicit item; no coverage figure claimed without a run | aggregate gates + static | `make check-gates && make test-shell-guards` plus the D-00a gate run | ✅ (was W0: handover file created) | ✅ green |
| 52-08-02 | 08 | 4 | CIREL-04, CIREL-06, CIREL-07 | T-52-33 | Executor performs no maintainer act | checkpoint (human-action) | — (reply recorded verbatim) | n/a | ✅ green (checkpoint recorded) |
| 52-08-03 | 08 | 4 | CIREL-04 | T-52-32, T-52-35 | Eight items resolved with evidence; committed ruleset matches the H-01 choice | static + gate | `./scripts/check-workflow-triggers.sh && make check-gates && make test-shell-guards` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/scripts/check-semver_test.sh` — created red-first in 52-01 Task 1 (CIREL-01 tripwire)
- [x] `tests/scripts/release-body-fit_test.sh` — created red-first in 52-02 Task 2 (CIREL-05 helper)
- [x] New cases in existing harnesses, each written red-first: create-or-reuse-release cases 10-14 (52-02), finalize-release-body cases 26-29 (52-04), check-migration-allowlist cases 12a-12f (52-05)
- [x] Evidence files: `52-CI-EVIDENCE.md` (52-05, 52-08), `52-CARRYOVER-EVIDENCE.md` (52-03), `52-SEMVER-RED-EVIDENCE.md` (52-07), `52-HANDOVER.md` (52-08)
- Framework install: none (the harness pattern and `cargo-semver-checks` 0.50.0 already exist)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CodeQL open-alert list | CIREL-06 | Sandbox gets HTTP 403 on code-scanning | Maintainer runs the command in `52-CARRYOVER-EVIDENCE.md` (52-03 Task 2) Done: maintainer list recorded verbatim in `52-CARRYOVER-EVIDENCE.md` (57 alerts: 37 CodeQL + 20 OSV). |
| Issue #41 comment and close | CIREL-07 | Sandbox `GH_TOKEN` invalid for writes | Executor tries first; fallback commands at 52-03 Task 2 / H-05 Done: public API re-read, closed `completed` 2026-10-28T23:09:45Z, 1 evidence comment. |
| Red `semver` job on a real PR | CIREL-01 | Push and PR creation are maintainer-authorised | 52-07 Task 2 decision; evidence captured in Task 3 or H-03 Done: job 123234542712 on `scratch/52-semver-red-proof` concluded `failure` (`paladin-storage: lint failure (exit 100)`); remote branch deleted (H-03). |
| Ruleset `20868126` PUT and read-back | CIREL-04 (D-08) | Live branch-protection change is maintainer-gated | H-01 in `52-HANDOVER.md` Done by the maintainer (H-01); live read-back equals the committed 46-context record. |
| `Publish Dry Run` and label-gated `End-to-End Tests` on a live PR | CIREL-04 | Needs a real PR and the maintainer's `release` label | H-02 in `52-HANDOVER.md` Done on PR #61: dry-run success unlabelled; e2e skipped unlabelled, success labelled (H-02). |
| CodeQL platform dismissals | CIREL-06 | Code-scanning write needs maintainer credentials | H-04 PATCH commands plus reconciliation Done per the maintainer's pasted `state=dismissed` read-back (H-04); still a `/gsd-verify-work` UAT item because the sandbox cannot re-read it (HTTP 403). |
| Local `make coverage` reproduction | folded todo | No `cargo-llvm-cov` or Docker daemon in the sandbox | H-06; todo stays pending (resolves_phase 56) Deferred (H-06) to a Docker-capable machine; `resolves_phase: 56`. |
| actionlint on edited workflows | D-00d | actionlint cannot execute in the sandbox | CI `Workflow Lint` job (required) on the pushed branch (H-08) Done: `Workflow Lint` success on head `716923cac` (runs 40952344198 / 40952352062). |
| Live GitHub release create / finalize with a long changelog (D-12) | CIREL-05 | No live release call in this phase; the guard is exercised only by stubbed `gh` harnesses | First exercised by the `v0.11.1` tag (Phase 58); watch for `paladin:release-truncated` and the size log in the Finalize Release Body job. Open until then. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the three checkpoint tasks 52-03-02, 52-07-02, 52-08-02 are human-action by design; their outcomes are recorded verbatim in the evidence files)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 52s (measured: `make test-shell-guards` 33 s, `make check-gates` 0.6 s)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-30 (`/gsd-validate-phase 52`)

---

## Validation Audit 2026-10-30

Retroactive Nyquist audit after phase execution (`/gsd-validate-phase 52`, State A). Every
automated command in the Per-Task Verification Map was re-run on the tree at `7ad707c8b`;
SUMMARY claims were not used as evidence.

| Metric | Count |
|--------|-------|
| Requirements in scope | 8 (CIREL-01..07, CIREL-09) |
| Tasks in map | 21 (18 automated, 3 human-action checkpoints) |
| Gaps found | 0 |
| Resolved | 0 (nothing to resolve) |
| Escalated | 0 |
| Manual-only behaviors | 9 (7 done with recorded read-back, 1 deferred to Phase 56, 1 open until the `v0.11.1` tag) |

### Re-run evidence

| Check | Result |
|-------|--------|
| `make test-shell-guards` | rc 0, 33 s |
| `make check-gates` | rc 0, 0.6 s (9 gates) |
| `check-semver_test.sh` / `release-body-fit_test.sh` / `create-or-reuse-release_test.sh` / `finalize-release-body_test.sh` / `check-migration-allowlist_test.sh` / `check-codeql-dismissals_test.sh` / `check-workflow-triggers_test.sh` | 70 / 85 / 49 / 129 / 20 / 11 / 11 assertions, all pass |
| `shellcheck --severity=warning` on the five touched scripts and their harnesses | clean |
| `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval` | `223 checks: 223 pass, 31 skip`, `0 failure(s)`, rc 0 |
| Crate-wide `cargo-semver-checks.lints` tables | 0 in `Cargo.toml` and `crates/*/Cargo.toml`; allowlist has 0 `[[entry]]` |
| `ci.yml` structure (PR types incl. `labeled`, semver job name and script call, e2e label `if:`, dry-run without `if:`, allowlist step in `cargo-deny`, no `labels` in any `run:`) | all assertions pass |
| `.github/rulesets/protect-main-branch.json` | 46 sorted contexts incl. `Publish Dry Run`, `Semver Checks (vs v0.11.0)`, `End-to-End Tests` |
| `release.yml` and `codeql.yml` vs phase base `3ece15f4c` | unchanged |
| Plan verify blocks 52-03 T1/T3, 52-06 T1-T3, 52-07 T1/T3, 52-08 T1/T3 run verbatim | all pass |
| Evidence files token-shaped grep | 0 hits in all four |
| Scratch break containment | `6f3d8d78` not an ancestor of HEAD; no `deliberate semver break` subject; local and remote scratch refs absent |

No test files were generated: every requirement already had a green automated test and no
behavior was found covered only by a claim.
