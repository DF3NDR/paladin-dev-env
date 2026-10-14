---
phase: 49
slug: downstream-adoption-of-v0-10-1-web3sec-consumer-bump
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-14
---

# Phase 49 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (libtest) for Rust; shell-script gates (`scripts/check-*.sh`) |
| **Config file** | `Cargo.toml` (`[[test]]` tables); `Makefile`; `.github/workflows/ci.yml` |
| **Quick run command** | `cargo test --offline --test consumer_surface_muster` |
| **Full suite command** | `cargo test --workspace` plus `make clean-code`, `make check-gates`, `make api-surface` (coverage via CI `coverage` job) |
| **Estimated runtime** | ~45 seconds (quick, warm); gates minutes |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --offline --test consumer_surface_muster` (or `make check-gates` for doc-only tasks)
- **After every plan wave:** Run the three guard tests, `cargo clippy --test consumer_surface_muster -- -D warnings`, `cargo fmt --all -- --check`, `make check-gates`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| {N}-01-01 | 01 | 1 | REQ-{XX} | T-{N}-01 / — | {expected secure behavior or "N/A"} | unit | `{command}` | ✅ / ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `{tests/test_file.py}` — stubs for REQ-{XX}
- [ ] `{tests/conftest.py}` — shared fixtures
- [ ] `{framework install}` — if no framework detected

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| {behavior} | REQ-{XX} | {reason} | {steps} |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < {N}s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
