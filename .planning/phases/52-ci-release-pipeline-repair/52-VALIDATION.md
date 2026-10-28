---
phase: 52
slug: ci-release-pipeline-repair
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-28
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
| 52-01-01 | 01 | 1 | CIREL-01 | T-52-01, T-52-02 | Zero checks or no check count never passes; package list only from `publish-crates.sh` | shell harness + real tool | `./tests/scripts/check-semver_test.sh && SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-eval` | ❌ W0 (harness created red-first in this task) | ⬜ pending |
| 52-01-02 | 01 | 1 | CIREL-03 | T-52-05 | Allowlist gate runs from the required cargo-deny job | static (YAML) + gate | `./scripts/check-migration-allowlist.sh && ./tests/scripts/check-migration-allowlist_test.sh && ./scripts/check-workflow-triggers.sh` | ✅ | ⬜ pending |
| 52-02-01 | 02 | 1 | CIREL-05 | T-52-06, T-52-07 | Body never crosses argv; >131,072-byte body sent byte-identical | shell harness | `./tests/scripts/create-or-reuse-release_test.sh` | ✅ (Case 10 added red-first) | ⬜ pending |
| 52-02-02 | 02 | 1 | CIREL-05 | T-52-08, T-52-09, T-52-10 | Character-counted line-boundary fit; sizes logged, body text never logged | shell harness | `./tests/scripts/release-body-fit_test.sh` | ❌ W0 (created red-first in this task) | ⬜ pending |
| 52-02-03 | 02 | 1 | CIREL-05 | T-52-07, T-52-08 | Over-limit body fitted before create; invalid limit fails before any API call | shell harness | `./tests/scripts/create-or-reuse-release_test.sh && make test-shell-guards` | ✅ | ⬜ pending |
| 52-03-01 | 03 | 1 | CIREL-06, CIREL-07 | T-52-12, T-52-14 | Facts read from the public API; no token-shaped string recorded | static (evidence) | `grep` checks in the task's verify (sections, URLs, negative token grep) | ❌ W0 (evidence file created) | ⬜ pending |
| 52-03-02 | 03 | 1 | CIREL-06, CIREL-07 | T-52-15 | Maintainer-only acts handed over, not attempted twice | checkpoint (human-action) | — (reply recorded verbatim) | n/a | ⬜ pending |
| 52-03-03 | 03 | 1 | CIREL-06 | T-52-11, T-52-13 | Every alert dispositioned; register guard green; codeql.yml unchanged | gate + harness | `make check-codeql-dismissals && ./tests/scripts/check-codeql-dismissals_test.sh` | ✅ | ⬜ pending |
| 52-04-01 | 04 | 2 | CIREL-05 | T-52-16, T-52-17 | Composed body fitted under the limit; artifact tail byte-identical; two-run identity | shell harness | `./tests/scripts/finalize-release-body_test.sh` | ✅ (Cases 26-27 added red-first) | ⬜ pending |
| 52-04-02 | 04 | 2 | CIREL-05 | T-52-16, T-52-18 | Under-limit identity; named failure before `gh release edit` | shell harness | `./tests/scripts/finalize-release-body_test.sh && make test-shell-guards` | ✅ | ⬜ pending |
| 52-05-01 | 05 | 2 | CIREL-01, CIREL-02 | T-52-19, T-52-22 | No crate-wide allow; twelve non-zero check counts with no failing lint | static + real tool | `! grep -q 'cargo-semver-checks\.lints' Cargo.toml crates/*/Cargo.toml && SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` | ✅ | ⬜ pending |
| 52-05-02 | 05 | 2 | CIREL-02, CIREL-03 | T-52-20, T-52-21 | Gate reads §11.2 only; `## 11.3` terminator present; §9/§10 byte-frozen; no placeholder token | shell harness + gate | `./tests/scripts/check-migration-allowlist_test.sh && ./scripts/check-migration-allowlist.sh` | ✅ (cases 12a-12f added red-first) | ⬜ pending |
| 52-06-01 | 06 | 2 | CIREL-04 | T-52-23, T-52-24, T-52-25 | Label used only in a job-level `if:`; trigger register and ruleset record agree with ci.yml | static (YAML/JSON) + gate | `./scripts/check-workflow-triggers.sh && ./tests/scripts/check-workflow-triggers_test.sh && ./scripts/check-workflow-suppressions.sh` | ✅ | ⬜ pending |
| 52-06-02 | 06 | 2 | CIREL-04 | T-52-25, T-52-27 | Docs state the 46-context set and the skipped-job semantics | static (docs grep) | `grep` checks in the task's verify | ✅ | ⬜ pending |
| 52-06-03 | 06 | 2 | CIREL-09, CIREL-04 | T-52-27 | Publish-order reading, label act and size limit documented; release.yml unchanged | static (docs grep) | `grep` checks in the task's verify; `git diff --quiet HEAD -- .github/workflows/release.yml` | ✅ | ⬜ pending |
| 52-07-01 | 07 | 3 | CIREL-01 | T-52-29 | Real public-API break goes red on the broken crate only; tree restored | real tool + evidence | `grep` checks on `52-SEMVER-RED-EVIDENCE.md` and `git diff --quiet -- crates/` | ❌ W0 (evidence file created) | ⬜ pending |
| 52-07-02 | 07 | 3 | CIREL-01 | T-52-30 | No push before the maintainer's answer | checkpoint (decision) | — | n/a | ⬜ pending |
| 52-07-03 | 07 | 3 | CIREL-01 | T-52-28, T-52-31 | Scratch break never reachable from the phase branch; PR closed, branch deleted or H-03 | CI observation + git | `grep` checks on the evidence; `git log` subject grep; `git show-ref` absent | ✅ | ⬜ pending |
| 52-08-01 | 08 | 4 | CIREL-01, CIREL-04, CIREL-06, CIREL-07 | T-52-32, T-52-34 | Every gated act is an explicit item; no coverage figure claimed without a run | aggregate gates + static | `make check-gates && make test-shell-guards` plus the D-00a gate run | ❌ W0 (handover file created) | ⬜ pending |
| 52-08-02 | 08 | 4 | CIREL-04, CIREL-06, CIREL-07 | T-52-33 | Executor performs no maintainer act | checkpoint (human-action) | — (reply recorded verbatim) | n/a | ⬜ pending |
| 52-08-03 | 08 | 4 | CIREL-04 | T-52-32, T-52-35 | Eight items resolved with evidence; committed ruleset matches the H-01 choice | static + gate | `./scripts/check-workflow-triggers.sh && make check-gates && make test-shell-guards` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/scripts/check-semver_test.sh` — created red-first in 52-01 Task 1 (CIREL-01 tripwire)
- [ ] `tests/scripts/release-body-fit_test.sh` — created red-first in 52-02 Task 2 (CIREL-05 helper)
- [ ] New cases in existing harnesses, each written red-first: create-or-reuse-release cases 10-14 (52-02), finalize-release-body cases 26-29 (52-04), check-migration-allowlist cases 12a-12f (52-05)
- [ ] Evidence files: `52-CI-EVIDENCE.md` (52-05, 52-08), `52-CARRYOVER-EVIDENCE.md` (52-03), `52-SEMVER-RED-EVIDENCE.md` (52-07), `52-HANDOVER.md` (52-08)
- Framework install: none (the harness pattern and `cargo-semver-checks` 0.50.0 already exist)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CodeQL open-alert list | CIREL-06 | Sandbox gets HTTP 403 on code-scanning | Maintainer runs the command in `52-CARRYOVER-EVIDENCE.md` (52-03 Task 2) |
| Issue #41 comment and close | CIREL-07 | Sandbox `GH_TOKEN` invalid for writes | Executor tries first; fallback commands at 52-03 Task 2 / H-05 |
| Red `semver` job on a real PR | CIREL-01 | Push and PR creation are maintainer-authorised | 52-07 Task 2 decision; evidence captured in Task 3 or H-03 |
| Ruleset `20868126` PUT and read-back | CIREL-04 (D-08) | Live branch-protection change is maintainer-gated | H-01 in `52-HANDOVER.md` |
| `Publish Dry Run` and label-gated `End-to-End Tests` on a live PR | CIREL-04 | Needs a real PR and the maintainer's `release` label | H-02 in `52-HANDOVER.md` |
| CodeQL platform dismissals | CIREL-06 | Code-scanning write needs maintainer credentials | H-04 PATCH commands plus reconciliation |
| Local `make coverage` reproduction | folded todo | No `cargo-llvm-cov` or Docker daemon in the sandbox | H-06; todo stays pending (resolves_phase 56) |
| actionlint on edited workflows | D-00d | actionlint cannot execute in the sandbox | CI `Workflow Lint` job (required) on the pushed branch (H-08) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 52s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
