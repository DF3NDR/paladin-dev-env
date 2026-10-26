---
phase: 47
slug: v0-11-0-crate-release
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-25
---

# Phase 47 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash guard scripts + `tests/scripts/*_test.sh` regression harnesses; `cargo test`; GitHub Actions as the authoritative gate |
| **Config file** | `Makefile` (`check-gates`, `test-shell-guards`), `.github/workflows/{ci,release}.yml` |
| **Quick run command** | `make check-gates && ./scripts/check-release-consistency.sh --tag v0.11.0` |
| **Full suite command** | `make test-shell-guards` plus the eight re-seal rows (see `47-RESEARCH.md` §Validation Architecture) |
| **Estimated runtime** | ~10 seconds (quick run); the heavy re-seal rows (2, 3, 5, 6) are wave-level and take minutes |

---

## Sampling Rate

- **After every task commit:** Run `make check-gates` plus the task-specific structural check
- **After every plan wave:** Run `make test-shell-guards && cargo deny check && cargo metadata --locked --offline`
- **Before `/gsd-verify-work`:** Full suite must be green — all eight re-seal rows green on the final head, CI green on the tick SHA and on the merge SHA, registry table 12/12 at `0.11.0` before any D-16 tick
- **Max feedback latency:** 30 seconds for per-task checks (heavy re-seal rows run per wave, not per task)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 47-01-01 | 01 | 1 | SHIP-07 | — | N/A | structural | `{populated from the PLAN.md <verify><automated> blocks by /gsd-validate-phase 47}` | ⬜ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Seeded as a draft by plan-phase; the map is populated from the phase PLAN.md tasks by `/gsd-validate-phase 47`.*

---

## Wave 0 Requirements

(from `47-RESEARCH.md` §Validation Architecture → Wave 0 Gaps)

- [ ] Body-size gate and offline payload rehearsal for the root `[0.11.0]` changelog section (research blocker B-1) — new check, no existing script
- [ ] Decide B-2 (tokio RUSTSEC-2026-0386, `cargo deny` red on the PR head) and the D-11 row-7 `[Unreleased]` wording; record both as findings before the build wave
- [ ] `cargo install --locked cargo-release --version 1.1.7` and a `df -h /` free-space threshold check
- [ ] Empty evidence skeletons: `47-CI-EVIDENCE.md` (three tables), `47-ACCEPTANCE-AUDIT.md` (8 rows + sign-off box)
- [ ] Space-recovery decision for re-seal rows 2/3/5/6 (research Pitfall 9: 2.5 GB free against a 20 GB `target/`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maintainer ticks the sign-off box in `47-ACCEPTANCE-AUDIT.md` | SHIP-07 | D-00a: only the maintainer ticks; the agent only verifies the tick is on `origin` at the PR head | `git fetch origin && git show origin/claude/laughing-dirac-e0h2ax:.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md \| grep -c '\[x\].*v0.11.0'` → `1` |
| Release PR #59 merged to `main` with a merge commit | SHIP-07 | D-00b: maintainer-only irreversible act | `git rev-list --parents -n 1 <merge-sha>` shows two parents |
| Annotated `v0.11.0` tag pushed on the merge commit | SHIP-07 | D-00b/D-00c: maintainer-only | `git cat-file -t v0.11.0` = `tag`; `git rev-parse 'v0.11.0^{commit}'` = merge SHA |
| MSRV 1.88 check (re-seal row 5) | SHIP-07 | The 1.88 toolchain is absent in the sandbox; the CI `MSRV` job is the twin | Read the CI `MSRV` job conclusion on the final head via `gh api` (REST) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
