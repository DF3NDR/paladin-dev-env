---
phase: 46-docs-currency-hygiene
verified: 2026-10-24T17:00:00Z
status: passed
score: 4/4 roadmap success criteria verified (plus plan-level truths spot-checked)
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
---

# Phase 46: Docs Currency & Hygiene Verification Report

**Phase Goal:** The v0.10.0 audit's docs debt is closed, v0.11.0's own surface is documented, and the milestone's validation/tech-debt bookkeeping is current.
**Verified:** 2026-10-24
**Status:** passed
**Re-verification:** No (initial verification)

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | v0.10.0 audit docs fixes closed: "twelve publishable crates" in the three pages; `paladin-eval` Trusted Publishing and Credential History rows; CHANGELOG `[0.10.0]` heading date corrected | VERIFIED | `twelve publishable crates` present at `development-setup.md:652`, `release-automation.md:77,321`, `release-recovery.md:40`. `release-automation.md:318` has the `paladin-eval` Trusted Publishing row (`on or before 2026-09-21`, `linked (proven by publish)`); the paragraph at :332 says it is proven by the 0.10.1 publish (run 35659477719), not operator-reported; Credential History row at :345 cites that run. Historical rc.1/rc.2 rows keep "eleven crates at the time" parentheticals (:341-342). Root and all 11 crate CHANGELOGs read `[0.10.0] - 2026-09-18` and `[0.10.1] - 2026-09-21`, matching `git tag` dates for v0.10.0 / v0.10.1. Commit `f7dae267` cited in the doc exists and equals the `v0.10.1` commit. |
| 2 | Treasurer mdBook page, config reference for pricing/allowance/warn threshold, and v0.10 to v0.11 MIGRATION.md guide published and reachable from book navigation | VERIFIED | `docs/src/SUMMARY.md` lists `architecture/treasurer.md` (l.43), `getting-started/configuration.md` (l.9), `api-reference/upgrading.md` (l.75), `api-reference/migration-guide.md` (l.76). `treasurer.md` is a substantive 193-line page (concept, model table, flow diagram, pricing, Cadence, usage sketch). `configuration.md` has `## Treasurer` with Key reference, Pricing, Allowances, `#### Warn threshold`, Cadence subsections; documented behaviour (deny_unknown_fields, 60 s to 31,622,400 s period bounds, `warn_at` `0..=100`, 9223372036.854775807 cap) matches `src/config/treasurer.rs`. Root `MIGRATION.md` is two-part: section 10 "Upgrading from v0.10.1 to v0.11.0" (10.1-10.8, operator and library-consumer checklists, 0 TBD/FIXME/XXX) above the kept section 9. The book's Upgrading page leads with v0.10.1 to v0.11.0 and Migration Guide page points to section 10 via a GitHub anchor (`#10-upgrading-from-v0101-to-v0110`, matches heading). ADR targets cited by the Treasurer page (0050/0052/0053/0056/0057/0058) exist. |
| 3 | Nyquist validation for Phases 22, 24, 29, 30, 34, 36, 36.1 out of `draft`; Phase 28 `nyquist_compliant` or gaps recorded | VERIFIED | Under `.planning/milestones/v0.10.0-phases/`, all of 22, 24, 29, 30, 34, 36, 36.1 have `status: validated`, `nyquist_compliant: true`, `wave_0_complete: true`, `validated: 2026-10-24` with a `reconciled:` note and an Approval line; no `⬜`/`❌` task-row statuses remain (only legend text). Phase 28 is `status: validated`, `nyquist_compliant: true`, promoted 2026-10-24 by plan 46-03 with UAT 2/2 and the overhead deviation recorded against WINDOWS rows 35/61. Spot-check of a cited test target: `tests/integration/e2e_approval_gate_test.rs` exists (Phase 24 row 24-05-01). These are reconcile-only (D-14/D-16; no new tests, decided policy), with re-runs on HEAD recorded for Phases 24 and 34. |
| 4 | Each of the three v2 debt lines fixed or carries a written waiver | VERIFIED | `.planning/WINDOWS.md` row 80 (oversized files: waived with measured figures; `wc -l` confirms superstep.rs 16,018, engine/mod.rs 14,324, paladin_execution_service.rs 10,858), row 81 (clone/lock: waived with written reason and re-open trigger), row 82 (dependency-allowlist drift: fixed, ADR-0015 amended and pinned by `scripts/check-dependency-allowlist.sh`). STATE.md "Tech debt" lines point at rows 80/81. Gate wired into `Makefile` `check-gates` (l.252) and `.github/workflows/ci.yml` (l.167). |

**Score:** 4/4 truths verified (0 behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/src/architecture/treasurer.md` | Treasurer page | VERIFIED | Substantive, in nav, links resolve |
| `docs/src/getting-started/configuration.md` `## Treasurer` | Config reference | VERIFIED | Matches `src/config/treasurer.rs`; `make check-doc-config`: 161 YAML blocks, 0 failed |
| `MIGRATION.md` section 10 | v0.10.1 to v0.11.0 guide | VERIFIED | 10.1-10.8, frozen note, no open markers |
| `scripts/check-dependency-allowlist.sh` + harness | Drift guard | VERIFIED | Script exit 0 (core 14, ports 12); harness 15 assertions pass |
| `crates/paladin-core/.../battalion/legacy_keys.rs` (D-23) | Removed-key refusal | VERIFIED | `cargo test -p paladin-ai-core --lib legacy_keys`: 15 passed |
| `crates/paladin-battalion/src/commander.rs` Maneuver warn (D-24) | Ignored-policy warning | VERIFIED | `maneuver_ignored_aegis_policies_names_only_the_fields_maneuver_drops` passes |
| `tests/consumer_surface_muster.rs` | Four new pins | VERIFIED | `cargo test --test consumer_surface_muster`: 1 passed |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Allowlist gate | `./scripts/check-dependency-allowlist.sh` | exit 0 | PASS |
| Gate harness | `bash tests/scripts/check-dependency-allowlist_test.sh` | 15 passed | PASS |
| All offline gates (incl. widened migration-allowlist, publish-order 12 crates) | `make check-gates` | exit 0 | PASS |
| Shell guard harnesses | `make test-shell-guards` | 54 assertions passed, exit 0 | PASS |
| Doc config snippets | `make check-doc-config` | 161 checked, 0 failed | PASS |
| Changelog coverage | `make check-changelogs` | all present | PASS |
| Removed-key refusal | `cargo test -p paladin-ai-core --lib legacy_keys` | 15 passed | PASS |
| Maneuver warn helper | `cargo test -p paladin-battalion --lib -- maneuver_ignored` | passed | PASS |
| Consumer-surface muster | `cargo test --test consumer_surface_muster` | passed | PASS |
| Formatting | `cargo fmt --check` | clean | PASS |

Full workspace `cargo test` not run (disk budget; known sandbox-only `build_run_api_persists_*` failures are out of scope).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| CURR-22 | 46-01 | v0.10.0 audit docs fixes | SATISFIED | SC1 evidence above |
| CURR-23 | 46-06, 07, 08, 09, 10, 11, 12, 13, 14 | Treasurer page, config reference, v0.10 to v0.11 MIGRATION guide | SATISFIED | SC2 evidence above; REQUIREMENTS.md `[x]`, traceability "Complete" |
| CURR-24 | 46-02, 03, 04, 13 | Nyquist validation reconciled; Phase 28 compliant | SATISFIED | SC3 evidence above |
| CURR-25 | 46-05 | Three v2 debt lines fixed or waived | SATISFIED | SC4 evidence above |

All four phase requirement IDs appear in plan frontmatter; no orphaned Phase 46 requirement exists in REQUIREMENTS.md.

### Anti-Patterns Found

None blocking. No `TBD`/`FIXME`/`XXX` markers were added by the phase outside descriptive mentions of the gate in validation rows (not open markers). The three docs files and `MIGRATION.md` have 0 such markers.

### Informational notes (not gaps)

- `mdbook` is not installed, so a rendered book build was not run. Navigation was verified through `docs/src/SUMMARY.md` and file existence; relative links in the three new or rewritten pages resolve.
- The root `MIGRATION.md` is reached from the book through GitHub URLs (the same pattern the v0.10.0 guide used), not as a book page.
- `make security` fails on pre-existing RUSTSEC-2026-0386 (tokio), recorded in `deferred-items.md`; unrelated to this phase.
- The ROADMAP Phase 46 header checkbox (`- [ ] **Phase 46`) is still unticked, though all 14 plan checkboxes are `[x]`. This is orchestrator-owned phase-close bookkeeping.
- Nyquist reconciliation for 22/24/29/30/34/36/36.1 is reconcile-only by design (D-14/D-16): test maps were rebuilt from archived artifacts, with selective re-runs on HEAD, not a full re-execution.

### Human Verification Required

None.

### Gaps Summary

No gaps. All four ROADMAP success criteria are met in the codebase, and the four requirement IDs (CURR-22..25) are accounted for.

---

_Verified: 2026-10-24_
_Verifier: Claude (gsd-verifier)_
