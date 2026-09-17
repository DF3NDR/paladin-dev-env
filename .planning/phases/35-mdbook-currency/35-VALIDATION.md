---
phase: 35
slug: mdbook-currency
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-17
---

# Phase 35 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdBook build + linkcheck backend, `scripts/check-doc-examples.sh` (Layer 1 `cargo check` on `crates/doc-examples`, Layer 2 inline-block scan), `scripts/check-doc-config.sh` (YAML fence parse) — no unit-test framework applies to prose pages |
| **Config file** | `docs/book.toml` (`[output.linkcheck] warning-policy = "error"`, `follow-web-links = false`) |
| **Quick run command** | `mdbook build docs/` |
| **Full suite command** | `mdbook-mermaid install docs/ && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh` (the exact `docs.yml` sequence) |
| **Estimated runtime** | ~4 seconds quick (measured 3.3 s); ~90 seconds full when `doc-examples` needs a warm `cargo check` |

---

## Sampling Rate

- **After every task commit:** Run `mdbook build docs/` (plus `./scripts/check-doc-examples.sh` after any `crates/doc-examples` edit)
- **After every plan wave:** Run `mdbook-mermaid install docs/ && mdbook build docs/ && ./scripts/check-doc-examples.sh && ./scripts/check-doc-config.sh`
- **Before `/gsd-verify-work`:** Full suite must be green, plus the CONTEXT.md D-21 exit greps and `make api-surface`, recorded verbatim in `35-EVIDENCE.md`
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 35-01-01 | 01 | 1 | CURR-{XX} | T-35-01 / — | {expected secure behavior or "N/A"} | build | `mdbook build docs/` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*(Seeded by plan-phase from `35-RESEARCH.md` §Validation Architecture; the planner fills one row per task and `/gsd-validate-phase 35` sets `nyquist_compliant`.)*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements: the phase's tests ARE the `docs.yml` gate sequence (already wired into CI as the required "Build MDBook" check), the `crates/doc-examples` compile gate, and the exit greps in CONTEXT.md D-21 — no new test file, fixture or framework is needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `CHANGELOG.md` `[0.10.0]` `### Documentation` subsection summarises the pages added and corrected | CURR-{XX} (ROADMAP SC5) | Content adequacy is a reading judgment; presence is automated | `grep -n -A 8 '^### Documentation' CHANGELOG.md` after the final plan, then read the bullets against the closure table |
| Each corrected page's prose describes the shipped surface accurately | CURR-{XX} (ROADMAP SC3) | Type/flag names are grep-checkable; prose meaning is not | Re-run `.planning/phases/34-documentation-currency-audit/34-signals.sh <page>` and read the row's §2 finding against the edited page |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
