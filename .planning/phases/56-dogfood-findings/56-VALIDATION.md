---
phase: 56
slug: dogfood-findings
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-02
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `56-RESEARCH.md`
> § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (libtest) for the config round-trip. The bash harness (`assert_fire`/`assert_silent`, `tests/scripts/*_test.sh`) covers the findings-log checker. |
| **Config file** | Root `Cargo.toml`: `tests/*.rs` is auto-discovered. `Makefile` `test-shell-guards` globs `tests/scripts/*_test.sh`. |
| **Quick run command** | `./tests/scripts/check-findings-log_test.sh` and `cargo test --test config_example_roundtrip` |
| **Full suite command** | `cargo test` + `make test-shell-guards` + `make check-gates` + `make clean-code` |
| **Estimated runtime** | Quick: ~5 s for the shell test, plus a ~60 s incremental build for the round-trip test. Full: several minutes. |

**Disk constraint:** about 4.2 GB is free. Clear `target/` incrementals before full builds, and build any published-crate scratch walk with a `CARGO_TARGET_DIR` that is deleted afterwards (see `56-RESEARCH.md`).

---

## Sampling Rate

- **After every task commit:** run the quick command for the files touched, plus `shellcheck --severity=warning` on any new `*.sh`.
- **After every plan wave:** run `cargo test`, `make test-shell-guards`, `make check-gates` and `make clean-code`.
- **Before `/gsd-verify-work`:** the full suite is green, `make check-findings-log` exits 0, and `make security` and `make api-surface` pass.
- **Max feedback latency:** 120 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 56-checker | 56-01 (T1-T2) | 1 | DOGF-01 | T-56-01..05 | Anchored regexes; no shell interpolation of log content; never vacuous (`ZERO_ENTRIES`, `UNCLOSED_FENCE`, `UNREADABLE`) | shell fixture | `./tests/scripts/check-findings-log_test.sh` | ❌ W0 | ⬜ pending |
| 56-roundtrip | 56-02 (T1-T2) | 1 | DOGF-03 | T-56-06..08 | Failure messages carry key paths, never values; `Settings` is never `{:?}`-printed; fixture creds are fake | integration | `cargo test -p paladin-ai --test config_example_roundtrip` | ❌ W0 | ⬜ pending |
| 56-drift-guard | 56-02 (T3) | 1 | DOGF-03 | T-56-09 | Only `steps: []` may be added to the example (L5) | integration | `cargo test -p paladin-ai --test config_example_roundtrip` | ❌ W0 | ⬜ pending |
| 56-runbooks | 56-03 (T1-T3) | 1 | DOGF-02, DOGF-04, DOGF-05 | T-56-11..13 | `read -rs` key entry; redaction before paste; W1 empty until relayed | runbook self-check | snippet on synthetic `lcov.info` prints `Lines: 165/200 = 82.50%  (truncated: 82)` | ❌ W0 | ⬜ pending |
| 56-df-proofs | 56-04 (T1-T2) | 2 | DOGF-01 | T-56-15 | Re-runs the owners' regression tests (FIRST-01 refusal, TENANT scope) | existing tests + gates | commands in `56-RESEARCH.md` Answer 7, each grepped as `<name> ... ok` | ✅ | ⬜ pending |
| 56-dispositions | 56-04 (T2) | 2 | DOGF-01 | T-56-16 | Append-only (0 deleted log lines) | script on real log | `./scripts/check-findings-log.sh` names none of DF-01…DF-04 | ❌ W0 | ⬜ pending |
| 56-leads | 56-05 (T1-T2) | 3 | DOGF-01, DOGF-06 | T-56-18..21 | Fake key only; evidence shows `<fake-key>`; L3/L6 logged `open → promote` for Stage C | process probes + checker | `## Lead L3/L4/L6` in `56-DOGFOOD-EVIDENCE.md` + `./scripts/check-findings-log.sh` | ❌ | ⬜ pending |
| 56-walk-keyless | 56-06 (T1-T2) | 4 | DOGF-04 | T-56-22..25 | Registry source only (cargo metadata source line); no key used | manual evidence + checker | `test -f .planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md` + `./scripts/check-findings-log.sh` | ❌ W0 | ⬜ pending |
| 56-docs-fixes | 56-07 (T1-T3) | 5 | DOGF-04, DOGF-01 | T-56-26..28 | `--locked` install line; frozen records untouched | resolution proof + gates | `make check-version-strings` + `cargo metadata` in scratch | ❌ | ⬜ pending |
| 56-walk-keyed | 56-08 (T1-T3) | 6 (B) | DOGF-04 | T-56-29..31 | Key only in the maintainer's shell; scratch `.env` deleted; secret scan before commit | manual + scan | runbook checkpoint; 9 K-rows in `## Keyed half (maintainer)` | ❌ | ⬜ pending |
| 56-coverage-walk | 56-09 (T1-T3) | 7 (B) | DOGF-05 | T-56-32..34 | Same-commit comparison; `int()` truncation | manual (Docker) + gh | `test -f .planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md` (pass) or `## Update` in the pending todo (fail) | ❌ | ⬜ pending |
| 56-web3sec | 56-10 (T1-T3) | 8 (B) | DOGF-02, DOGF-06 | T-56-35..37 | No simulated return; maintainer-only decision | manual + checker | `./scripts/check-findings-log.sh` + grep for owner/date on re-home | ❌ | ⬜ pending |
| 56-fixes | Stage C (`/gsd-plan-phase 56 --gaps`) | C | DOGF-06 | per finding | per finding (e.g. redacting `Debug` for keys, lead L3) | per-fix regression test | each fix's own test + `make api-surface` + `make check-semver` | ❌ Stage C | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*. Plan ids filled in by the planner (2026-11-02). Stage C plans are written later from the log (D-06, D-07).

---

## Wave 0 Requirements

- [ ] `scripts/check-findings-log.sh` + `Makefile` target `check-findings-log` (with a `## ` help comment) — DOGF-01
- [ ] `tests/scripts/check-findings-log_test.sh` (executable), with these fixture cases: open, missing,
  unrecognised, bare `in v0.11.1`, `out` without reason, `duplicate of` with a missing target,
  duplicate id, non-increasing id, fenced template ignored, last-bullet-wins, zero entries,
  unreadable, determinism, no-mutation. It must NOT assert that the real log passes; it asserts
  only that the real log parses (≥ 4 entries). — DOGF-01
- [ ] `tests/config_example_roundtrip.rs` + `tests/fixtures/config/config.example.full.yml` + a provenance paragraph in `tests/fixtures/config/README.md` — DOGF-03
- [ ] Optional `[dev-dependencies] serde_yaml = "0.9"` (already in `Cargo.lock`) — DOGF-03
- [ ] Stage B runbooks: the keyed walk; the coverage walk with the LF/LH summation snippet and the port-mismatch check; the web3sec triage — DOGF-02/04/05

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `onboarding` / `setup-check` / Quickstart against a real OpenAI key | DOGF-04 | Needs a real provider key, which never enters the repo | Runbook in the walk evidence file; paste redacted output; log stumbles as DF-nn |
| Local `make services-up` → `make coverage` matches the CI figure to the whole percent | DOGF-05 | Needs a Docker daemon (none in the agent sandbox) | Coverage runbook; record the local figure, CI run id and commit; move the todo to completed |
| web3sec §6 return triage, or a re-home decision | DOGF-02 | External consumer's return; the maintainer's decision | `checkpoint:decision`: triage one DF per divergence, or re-home with owner and date |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
