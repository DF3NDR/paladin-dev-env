---
phase: 29
slug: program-gates-release
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-09
validated: 2026-10-24
reconciled: "2026-10-24 by Phase 46 plan 46-03, reconcile-only (D-14/D-16): map rebuilt from the archived PLAN/SUMMARY/VERIFICATION; no test written"
---

# Phase 29 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

Reconciled 2026-10-24 by Phase 46 plan 46-03 on HEAD `f365604e` (branch `claude/laughing-dirac-e0h2ax`), reconcile-only:
the per-task map below was rebuilt mechanically from the 20 task elements of `29-01`..`29-09-PLAN.md`, the
nine SUMMARYs and `29-VERIFICATION.md` (status `passed`, 4/4 roadmap criteria, 18/18 plan-level must-haves) and
`29-UAT.md` (3/3 passed). Commands that run offline in the sandbox were re-run on HEAD in this session; commands
that need a network, `mdbook` or a registry baseline are marked "CI evidence" and cite the record that holds the
run. No test was written and no implementation file was touched (D-14). Phase 29 is a gates-and-release phase,
so most tasks map to `make` targets, scripts and `grep` checks rather than unit tests.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust 1.97.1 pinned; MSRV 1.88), `tokio::test` for async; `libtest-mimic` harness for `tests/evals.rs`; offline gate scripts under `scripts/` and `make check-gates` |
| **Config file** | Cargo.toml `[[test]]` registry (root) + `crates/*/Cargo.toml`; `.cargo/semver-checks-allowlist.toml`; `.github/workflows/ci.yml` (`semver`, `e2e-platform-api` jobs) -- no Wave 0 install needed |
| **Quick run command** | `cargo test --offline --features web-server --test v0_9_config_boot` / `cargo test --offline -p paladin-web --test openapi_golden_v0_9` (per-target) |
| **Gate command** | `make check-gates` (eight offline release-gate guards on HEAD; includes `check-migration-allowlist`, `check-changelogs`, `check-publish-order`) |
| **Full suite command** | `cargo test --workspace` (+ `cargo test --test evals`) |
| **Estimated runtime** | per-target commands under 30 seconds warm; `make check-gates` under 1 second |

---

## Sampling Rate

- **After every task commit:** Run the per-target command for the touched target (see Test Infrastructure)
- **After every plan wave:** Run `cargo test --workspace` and `make check-gates`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

Test type legend: **integ** = `tests/` binary; **golden** = frozen-baseline comparison; **gate** = offline file/tool/grep check; **semver** = `cargo-semver-checks`; **CI** = evidence lives in a CI run record.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 29-01-01 | 01 | 1 | SHIP-02 | T-29-01-01 / T-29-01-03 | A v0.9 config boots v0.10 with every new subsystem off; both frozen fixtures keep their v0.9.0 blob SHAs (provenance checkable, not asserted) | integ (tracer) | `cargo test --features web-server --test v0_9_config_boot && test "$(git hash-object tests/fixtures/config/v0.9.0-config.test.yml)" = e63d9f93582e0e06f2a1b40530fac94e846ffe32` | ✅ | ✅ green (re-run on HEAD: 9 passed; blob SHA `e63d9f93` matches) |
| 29-01-02 | 01 | 1 | SHIP-02 | T-29-01-01 / T-29-01-02 | Every `/v1/runs`, `/v1/threads`, `/v1/assistants`, `/v1/schedules` family answers `501` from a v0.9 config; `apply_env_overrides_is_a_no_op_with_no_app_vars` clears all 26 `APP_*` vars under `#[serial]` | integ | `cargo test --features web-server --test v0_9_config_boot` | ✅ | ✅ green (9 passed on HEAD) |
| 29-02-01 | 02 | 1 | SHIP-02 | T-29-02-01 / T-29-02-04 | The OpenAPI baseline is the v0.9.0 blob (`f9d22f27`, not HEAD-sourced); `v0_9_path_restriction_is_non_empty` rules out a vacuous pass | integ (tracer, golden) | `cargo test -p paladin-web --test openapi_golden_v0_9 && test "$(git hash-object crates/paladin-web/tests/fixtures/openapi-v0.9.0.json)" = f9d22f27f57f8da0957d8bc28c662a546ee6b0a6` | ✅ | ✅ green (re-run on HEAD: blob SHA `f9d22f27` matches; suite 9 passed, see 29-02-02) |
| 29-02-02 | 02 | 1 | SHIP-02 | T-29-02-02 / T-29-02-03 / T-29-02-04 | Transitive `$ref` closure and `securitySchemes` compared in full (a field change under an unchanged path key, or an auth-scheme rename, fails) | integ (golden) | `cargo test -p paladin-web --test openapi_golden_v0_9` | ✅ | ✅ green (9 passed on HEAD; the file held 6 at 2026-09-10, later phases added three exception-scoping tests: `phase_42_exception_is_narrowly_scoped`, `allowance_429_exception_is_narrowly_scoped`, `execute_response_exception_is_narrowly_scoped`) |
| 29-03-01 | 03 | 1 | SHIP-01, SHIP-04 | T-29-03-01 / T-29-03-03 | MIGRATION.md §9.2 register and `.cargo/semver-checks-allowlist.toml` compared as a deduplicated `crate | type` pair set in both directions, no wildcards | gate | `make check-migration-allowlist` (plan's inline awk pipeline, now `scripts/check-migration-allowlist.sh`) | ✅ | ⚠ superseded: the inline awk moved into `scripts/check-migration-allowlist.sh` + Makefile target `check-migration-allowlist` (PRIM-05, a later phase) and is a member of `make check-gates`. Equivalent re-run on HEAD: exit 0, "Allowlist is set-equal to the MIGRATION.md §9.2 deliberate-breaking register (crate|type pairs)" |
| 29-03-02 | 03 | 1 | SHIP-01, SHIP-04 | T-29-03-02 | `make publish-dry-run` is one command whose exit status is the verdict: no `|| true`, no `--allow-dirty`, no stale `docs/RELEASE_CHECKLIST.md` pointer | gate | `awk '/^publish-dry-run:/,/^$/' Makefile` greps; `make -n publish-dry-run`; `grep -q paladin-eval docs/src/appendix/release-checklist.md`; `mdbook build docs/` | ✅ | ✅ green (Makefile and checklist legs re-run on HEAD: 0 `|| true`, 0 stale pointer, `make -n` ok, 3 `paladin-eval` hits, 0 `eleven-crate`) ⚠ `mdbook build` leg is CI evidence (`mdbook` is not installed in the sandbox; `docs.yml` "Build MDBook") |
| 29-04-01 | 04 | 2 | SHIP-03 | T-29-04-03 / T-29-04-04 | Audit document has one `Verdict:` and one `Findings:` heading per protocol section; evidence cites commands, counts, SHAs and `file#test_name` only | gate | `grep -c '^## '` / `grep -c '^Verdict:'` / `grep -c 'Findings:'` on `.project/v0.10.0/09-program-acceptance-audit.md`; `grep -q 09-program-acceptance-audit.md 29-ACCEPTANCE-AUDIT.md` | ✅ | ⚠ superseded: the literal `-eq 10` counts no longer hold because later phases appended §11-§14 (Phase 33 re-seal, Phase 37, Phase 37.1 x2); §1-§10 are intact and in order. Equivalent re-run on HEAD: 0 `Verdict: pending`, 11 `Verdict:` lines. The plan's `-ge 150` per-FR row threshold was already corrected to 138 at execution (29-04-SUMMARY deviation) |
| 29-04-02 | 04 | 2 | SHIP-03 | T-29-04-01 / T-29-04-02 | E2E-1/2/3 green as integration tests; BUG-01's old path is grep-absent and its four regression tests exist; no `.rs` file changed by the audit | integ + gate | `cargo test --test evals && cargo test --test e2e_crash_resume --test e2e_approval_gate --test e2e_muster_defer_order && ! grep -rn 'defaulting to true' crates/ src/` | ✅ | ✅ green (re-run on HEAD: evals 4 passed; e2e_crash_resume 32, e2e_approval_gate 35, e2e_muster_defer_order 37 passed; BUG-01 grep 0 hits; `unregistered_custom_condition_is_rejected_before_any_paladin_executes` 1 passed) |
| 29-04-03 | 04 | 2 | SHIP-03 | T-29-04-01 / T-29-04-02 | Ubiquitous-language table covers all twelve terms with the rename-deferred findings recorded; SHIP-02 proofs cited by target name | gate | `for t in Battlefield Dispatch Superstep Waypoint Thread Directive Muster Parley Vanguard Chronicle Aegis Vault; do grep -q "$t" $A; done; grep -q 'rename deferred' $A; grep -q v0_9_config_boot $A; grep -q openapi_golden_v0_9 $A` | ✅ | ✅ green (re-run on HEAD: all twelve terms present; "rename deferred" x2; `v0_9_config_boot` x16, `openapi_golden_v0_9` x14) -- the plan's commit-relative `git diff --name-only HEAD~1` leg is not re-runnable on HEAD and was green at 29-04 commit time |
| 29-05-01 | 05 | 2 | SHIP-01, SHIP-02 | T-29-05-01 / T-29-05-02 / T-29-05-03 | MIGRATION.md has zero `TBD` and no invented `paladin-cli health` / `graph validate`; §9.5/§9.6 cite the two SHIP-02 proofs by target name; §9.8 is a numbered checklist | gate | `test "$(grep -c TBD MIGRATION.md)" -eq 0; grep -q v0_9_config_boot MIGRATION.md; grep -q openapi_golden_v0_9 MIGRATION.md; ...; test "$(awk '/^## 9\.8 /,0' MIGRATION.md | grep -cE '^[0-9]+\.')" -ge 7` | ✅ | ✅ green (re-run on HEAD: `TBD` 0; `v0_9_config_boot` x3, `openapi_golden_v0_9` x5; `paladin-cli health` / `graph validate` 0; "Still owed" 0; four `## 9.5`..`## 9.8` headings; §9.8 numbered items 8 >= 7) |
| 29-05-02 | 05 | 2 | SHIP-01, SHIP-02 | T-29-05-04 / T-29-05-05 | `semver` job fails on any `TBD` left in MIGRATION.md; `e2e-platform-api` runs the boot test and fails if it selects fewer than the full 9 tests | gate (CI wiring) | `python3 -c "import yaml; ..."` step-count assertions on `.github/workflows/ci.yml`; `grep -c v0_9_config_boot .github/workflows/ci.yml` | ✅ | ⚠ superseded: the literal step counts (`semver` 6, `e2e-platform-api` 7) and `baseline-version 0.9.0` x5 drifted after later CI edits (`semver` job now 7 steps, `e2e-platform-api` 7, `baseline-version 0.9.0` x3 -- one loop at ci.yml:393); the plan's own SUMMARY already recorded the `semver` count as a stale literal. Equivalent re-run on HEAD: `v0_9_config_boot` referenced 6 times, the `TBD` gate step at ci.yml:497, and the "fewer than 9 tests" guard at ci.yml:1484-1496 all present. CI run evidence: CI evidence (`29-CI-EVIDENCE.md`) |
| 29-06-01 | 06 | 3 | SHIP-01 | T-29-06-01 / T-29-06-02 / T-29-06-04 | Upgrading page is hand-written (no `{{#include}}`), links MIGRATION.md by repository URL, names M-B-01..04 and no invented subcommand | gate | `test -f docs/src/api-reference/upgrading.md`; M-B-01..04 greps; `grep -c '{{#include'` = 0; SUMMARY ordering; `mdbook build docs/` | ✅ | ✅ green (grep legs re-run on HEAD: M-B-01 x2, M-B-02 x2, M-B-03 x1, M-B-04 x1, 0 `{{#include`, `Upgrading` entry at `docs/src/SUMMARY.md:74`, migration-guide points at CHANGELOG.md) ⚠ `mdbook build` leg is CI evidence (`mdbook` not installed in the sandbox) |
| 29-06-02 | 06 | 3 | SHIP-01 | T-29-06-03 | Overview §4 errata note is bounded: twelve term rows, twelve term names unchanged, `README.md` untouched | gate | `grep -q RunOutcome::Halted .project/v0.10.0/00-program-overview.md`; `awk '/^## 4\./,/^## 5\./' ... | grep -cE '^\| \*\*'` = 12; twelve `**Term**` greps | ✅ | ✅ green (re-run on HEAD: `RunOutcome::Halted` x1, `WaypointStatus::Halted` x1, `directive.rs` x1, §4 term rows 12) |
| 29-07-01 | 07 | 3 | SHIP-03 | T-29-07-01 / T-29-07-04 | Audit §6 reconciles §9.2 against the v0.9.0-to-HEAD export diff and per-crate semver-checks in both directions | gate + semver | `awk '/^## 6/,/^## 7/' $A` greps (`current-exports.txt`, `required method`, no `Verdict: pending`); `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` | ✅ | ✅ green (audit §6 legs re-run on HEAD: `current-exports.txt` x7, `required method` x2, `ADR-0048` x2, 0 pending) ⚠ semver-checks leg is CI evidence: the local re-run on HEAD built the current crate but its baseline `cargo doc` of 0.9.0 failed in the sandbox (no registry network), so it is cited to `29-CI-EVIDENCE.md` row 10 and the `semver` CI job instead |
| 29-07-02 | 07 | 3 | SHIP-03 | T-29-07-02 / T-29-07-05 | Seven judgment-tier boxes authored unticked (agents never tick them); the accepted overhead deviation recorded in the audit and the published observability page | gate | `grep -q 22.18 $A`; `grep -q 22.18 docs/src/operations/observability.md`; `awk '/^## Maintainer sign-off/,/^## 10/' $A | grep -c '^- \[ \]'` = 7; `mdbook build docs/` | ✅ | ✅ green (re-run on HEAD: `22.18` in audit x3 and `observability.md` x1; sign-off section still carries 7 unticked boxes, see Manual-Only; the file-wide `- [ ]` count is now 8 because Phase 33 §11 added a `v0.10.0` tag box) ⚠ `mdbook build` leg is CI evidence |
| 29-08-01 | 08 | 4 | SHIP-03 | T-29-08-01 / T-29-08-05 | Every waived `.planning/WINDOWS.md` row carries a non-empty cited reason; no row deleted | gate | `node .claude/gsd-core/bin/gsd-tools.cjs windows status`; awk row count `-ge 34`; `grep -c v0.9.0-MILESTONE-AUDIT .planning/WINDOWS.md` `-ge 12`; empty-reason scan | ✅ | ✅ green (re-run on HEAD: 79 rows >= 34, `v0.9.0-MILESTONE-AUDIT` x24 >= 12, 0 waived rows with an empty reason) |
| 29-08-02 | 08 | 4 | SHIP-03 | T-29-08-02 / T-29-08-03 / T-29-08-04 | Docker-gated rows 22, 26, 27, 28 closed `fixed` against real CI runs; overhead deviation filed as row 35; counts reconcile | gate | `windows status`; rows 22/26/27/28 `fixed`; row count `-eq 35`; frontmatter `open+waived+fixed == total` | ✅ | ⚠ superseded: the ledger is cross-phase and has grown (79 rows, `open_count` 13 belong to later phases), so the literal `-eq 35` / `open_count` 0 no longer apply. Equivalent re-run on HEAD: rows 22, 26, 27, 28 `fixed`; row 35 `waived` with the 22.18% / 18.46% text; `open + waived + fixed == total` (13 + 37 + 29 = 79) |
| 29-09-01 | 09 | 5 | SHIP-04 | T-29-09-01 / T-29-09-06 | Every manifest bumped by `cargo-release` only; semver baseline pins untouched; `openapi.json` `info.version` regenerated | gate | `grep -q 'version = "0.10.0"' Cargo.toml` and the eleven `crates/*/Cargo.toml` greps; `grep -c 'baseline-version 0.9.0' .github/workflows/ci.yml` `-eq 5` | ✅ | ⚠ superseded: the v0.10.0 pin was replaced by the v0.10.1 patch release (Phase 37.1; Phase 46 plan 46-01 dated the headings) -- all 13 manifests now read `0.10.1`; `baseline-version 0.9.0` is x3 on HEAD. Equivalent re-run on HEAD: `./scripts/check-release-consistency.sh --tag v0.10.1` exit 0 ("12 publishable package(s) checked, all match tag version '0.10.1'") |
| 29-09-02 | 09 | 5 | SHIP-04 | T-29-09-04 | Every publishable changelog carries a dated `## [0.10.0]` section and `## [Unreleased]`; root notes name M-B-01..04 and the accepted deviation | gate | `for f in CHANGELOG.md crates/*/CHANGELOG.md; do grep -q '^## \[0.10.0\]' $f; done`; root `[0.10.0]` section greps; `./scripts/check-release-consistency.sh --tag v0.10.0` | ✅ | ✅ green (re-run on HEAD: `## [0.10.0]` present in all 12 changelogs; root section is 507 lines with `behavioral change` x1, M-B-01..04 x9, `22.18` present; `make check-changelogs` inside `make check-gates` exit 0). The final `--tag v0.10.0` leg is superseded by `--tag v0.10.1` (see 29-09-01), exit 0 |
| 29-09-03 | 09 | 5 | SHIP-04 | T-29-09-02 / T-29-09-03 / T-29-09-05 | Twelve crates dry-run packaged in dependency order; unrunnable gates recorded as owed-to-CI, never as passing; no tag cut from the branch | gate + CI | `cargo publish --workspace --dry-run`; `./scripts/check-release-consistency.sh`; `mdbook build docs/`; `test -s 29-CI-EVIDENCE.md`; `git tag -l v0.10.0` | ✅ | ✅ green (CI evidence: `29-CI-EVIDENCE.md` rows 14-15 record 12/12 crates verified in order; the real release run `35659477719` published all 12 crates at `0.10.1`, per MILESTONES.md. Re-run on HEAD offline: `make check-publish-order` (member of `make check-gates`) exit 0 on the 12-crate CRATES order; `29-CI-EVIDENCE.md` present and non-empty; tags `v0.10.0` and `v0.10.1` exist, cut on `main` merge commits) ⚠ `cargo publish --dry-run` and `mdbook build` need a network / `mdbook` and are not re-run here |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky / caveat / superseded*

All 20 task elements are `tracer` (3) or `auto` (17); Phase 29 has no `checkpoint:*` task.

**Requirement roll-up**

| Requirement | Tasks | Automated & green | Notes |
|---|---|---|---|
| SHIP-01 | 29-03-01..02, 29-05-01..02, 29-06-01..02 | 6/6 (2 superseded with a green equivalent) | `make check-migration-allowlist` and the `TBD` count re-run on HEAD |
| SHIP-02 | 29-01-01..02, 29-02-01..02, 29-05-01..02 | 6/6 (1 superseded with a green equivalent) | both backward-compat proofs re-run on HEAD: 9 + 9 passed |
| SHIP-03 | 29-04-01..03, 29-07-01..02, 29-08-01..02 | 7/7 (2 superseded with a green equivalent) | E2E-1/2/3 and `evals` re-run on HEAD; semver-checks leg is CI evidence |
| SHIP-04 | 29-03-01..02, 29-09-01..03 | 5/5 (1 superseded with a green equivalent) | `check-release-consistency.sh --tag v0.10.1` and `make check-gates` exit 0; dry-run publish is CI evidence |

**Superseded rows (5 of 20, each with a re-run equivalent):** 29-03-01 (inline awk moved to `scripts/check-migration-allowlist.sh`), 29-04-01 (audit grew to §14), 29-05-02 (CI step-count literals drifted), 29-08-02 (ledger grew to 79 rows), 29-09-01 (`0.10.0` pin replaced by `0.10.1`).

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. Every test file the map names was authored inside its own plan (the two tracer tasks 29-01-01 and 29-02-01 each began with the slice that proved the target) and exists on HEAD:

- [x] `tests/integration/v0_9_config_boot_test.rs` + `tests/fixtures/config/v0.9.0-config.{test,example}.yml` + `README.md` -- SHIP-02 config proof (29-01)
- [x] `crates/paladin-web/tests/openapi_golden_v0_9.rs` + `crates/paladin-web/tests/fixtures/openapi-v0.9.0.json` + `README.md` -- SHIP-02 OpenAPI proof (29-02)
- [x] `.project/v0.10.0/09-program-acceptance-audit.md` + `29-ACCEPTANCE-AUDIT.md` -- SHIP-03 audit corpus (29-04, 29-07)
- [x] `docs/src/api-reference/upgrading.md` -- SHIP-01 Upgrading page (29-06)
- [x] `29-CI-EVIDENCE.md` -- SHIP-04 evidence record (29-09)

---

## Manual-Only Verifications

All three items are the `human_verification` block of `29-VERIFICATION.md`; each is closed by `29-UAT.md` (3/3 passed, 2026-09-10).

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Seven maintainer sign-off checkboxes in `.project/v0.10.0/09-program-acceptance-audit.md` (six judgment-tier safety/privacy prohibitions plus the M-B-04 provenance countersignature) | SHIP-03 | D-17: an agent's verdict on a safety/privacy claim is non-authoritative by design; a human countersigns. **Closed:** `29-UAT.md` test 1 = pass. | Review each item's cited evidence. Note: the seven boxes themselves are still `- [ ]` on disk although the pass is recorded in `29-UAT.md`; this reconciliation does not tick them (STATE.md tracks that bookkeeping drift separately). |
| Semver and MSRV CI jobs green on the real release commit | SHIP-04 | SHIP-04 requires a real CI run, which a local devcontainer sweep cannot produce. **Closed:** `29-UAT.md` test 2 = pass; the `v0.10.0` / `v0.10.1` release runs followed (run `35659477719`, all 12 crates on crates.io at `0.10.1`). | Push, read back the `ci`, `docs` and `feature-flags` runs. |
| The Phase 28 tracing-overhead deviation (+22.18% / +18.46% against the <=3% PRD 07 bar) stays an acceptable waiver for v0.10.0 | SHIP-03 | A substantive quality-bar deviation, not a mechanical check. **Closed:** `29-UAT.md` test 3 = pass; recorded at `.planning/WINDOWS.md` rows 35 and 61 and the `CHANGELOG.md` `[0.10.0]` Known limitations (cited here, not re-worded). | Reaffirm or reopen; reaffirmed. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (20 of 20; no `checkpoint:*` task)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none were missing)
- [x] No watch-mode flags
- [x] Feedback latency < 30s for the per-target quick commands
- [x] `nyquist_compliant: true` set in frontmatter -- every requirement id (SHIP-01..04) has at least one green automated row, every row is green or superseded with a re-run equivalent, and the three manual-only rows cite the `29-UAT.md` pass

**Approval:** reconciled 2026-10-24 (Phase 46 plan 46-03, D-14) -- 20 task rows, 0 gaps, 3 manual-only sign-offs closed

---

## Validation Audit 2026-10-24

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

- **Method:** reconcile-only (D-14). Map rebuilt from the archived plans and evidence; `/gsd-validate-phase` Step 4 (gap menu) and Step 5 (`gsd-nyquist-auditor`) skipped, so no test was written.
- **Re-run on HEAD 2026-10-24 (all exit 0):** `cargo test --offline --features web-server --test v0_9_config_boot` (9 passed); `cargo test --offline -p paladin-web --test openapi_golden_v0_9` (9 passed); `cargo test --offline --test evals` (4 passed); `cargo test --offline --test e2e_crash_resume` (32), `--test e2e_approval_gate` (35), `--test e2e_muster_defer_order` (37); `cargo test --offline -p paladin-battalion --lib unregistered_custom_condition_is_rejected_before_any_paladin_executes` (1 passed); `make check-gates`; `make check-migration-allowlist`; `./scripts/check-release-consistency.sh --tag v0.10.1`; the `grep` legs named in the map.
- **CI evidence (not re-run):** `mdbook build docs/` (`mdbook` not installed in the sandbox); `cargo publish --workspace --dry-run` (needs the registry); `cargo semver-checks check-release ... --baseline-version 0.9.0` (attempted; the baseline `cargo doc` of 0.9.0 failed offline). Cited to `29-CI-EVIDENCE.md` rows 10, 14, 15 and the `semver` / `docs` CI jobs.

## Gaps

None.
