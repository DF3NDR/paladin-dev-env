---
status: complete
phase: 37-v0-10-0-crate-release
source: 37-01-SUMMARY.md, 37-02-SUMMARY.md, 37-03-SUMMARY.md, 37-04-SUMMARY.md, 37-05-SUMMARY.md, 37-06-SUMMARY.md, 37-07-SUMMARY.md, 37-08-SUMMARY.md
started: 2026-09-22T12:35:02Z
updated: 2026-09-22T21:38:22Z
---

## Current Test

[testing complete]

<!-- Extraction notes: summaries 37-01..37-07 have no coverage: block (legacy prose extraction);
     37-08 has an empty coverage: [] block (classifier: coverage mode, 0 entries, nothing auto-passed).
     Plans 37-09, 37-10, 37-11 were never executed (superseded by Phase 37.1) and have no SUMMARY.
     No cold-start smoke test: no summary touches a server/db/migration/docker path (docs-only phase).
     verify:pre api-coverage gate: block=false (COVERAGE.md declares no external API integration). -->

## Tests

### 1. [Plan 37-01 / SC1] 37-CI-EVIDENCE.md is the end-to-end release-evidence tracer
expected: .planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md exists with its five required sections (Provenance, Local sweep, CI-run table, Findings carried forward (D-00d), Summary and what remains); MIGRATION.md contains zero "TBD"; the D-17 paladin-eval checkpoint reply is recorded verbatim as the deferred branch, later superseded (not edited) by the maintainer's bootstrap record showing paladin-eval 0.0.1 live on crates.io; the five carried documentation-currency findings are recorded, not fixed.
result: pass

### 2. [Plan 37-02 / SC1] Local sweep rows 1-5 and 7 re-sealed green on the final pre-merge tree
expected: 37-CI-EVIDENCE.md's Local sweep records, on the final pre-merge tree (head 522ab1d4): MIGRATION.md TBD count 0 with its section 9.2 register matching the semver-checks allowlist row-for-row, v0_9_config_boot passing, the OpenAPI golden diff clean, cargo semver-checks 11/11 (rows 17-27 plus tally row 28, paladin-eval excluded with its no-0.9.0-baseline reason, the DNS-outage interruption and single authorized re-run recorded in-row rather than smoothed), the MSRV 1.88 floor (row 16, 0 warnings), and a complete CHANGELOG.md [0.10.0]; no source, manifest or lockfile was touched.
result: pass

### 3. [Plan 37-03 / SC1] Gate row 6 (make publish-dry-run), adjacent checks, and the Local sweep close
expected: make publish-dry-run exit 0 (12/12 dry-run-abort lines in dependency order, paladin-doc-examples absent from the Uploading list, all 40 test result lines ok, cargo audit with no new advisory); make security and make api-surface both exit 0 with .project/current-exports.txt unmodified; the Local sweep closes with a 31-row verdict tally (29 unconditional passes, 2 carried conditions, 0 rows claiming the CI-attributed coverage gate) and a dated addendum naming the 82% ADR-0006 floor as CI-attributed because Docker is absent locally.
result: pass

### 4. [Plan 37-04] SHIP-05 is minted and the evidence pointers are corrected, additions-only
expected: REQUIREMENTS.md has a SHIP-05 definition row directly after SHIP-04 (unchecked, "v0.10.0 is released" per D-07) plus a SHIP-05 | Phase 37 traceability row, with SHIP-04 byte-identical; ROADMAP Phase 37 reads "**Requirements**: SHIP-05"; STATE.md's Project Reference evidence pointer names 37-CI-EVIDENCE.md's CI-run table; 29-CI-EVIDENCE.md and 33-CI-EVIDENCE.md each end with one dated forward-pointer line naming 37-CI-EVIDENCE.md, with their own rows unchanged.
result: pass

### 5. [Plan 37-05 / SC1] Corpus acceptance audit section 12 carries the seven-row re-seal table
expected: .project/v0.10.0/09-program-acceptance-audit.md has an appended "## 12. Re-seal for v0.10.0 release (Phase 37, SHIP-05)" section before the italic footer: head-SHA paragraph naming 522ab1d4 plus the four later doc-only commit SHAs, a seven-row D-06 gate table each pointing at a 37-CI-EVIDENCE.md Local sweep row, rows 4 and 7 carrying the semver-checks interruption and the two zero-valued documentation readings as findings not failures, a Findings list, a new footer line, and no new sign-off box (the section 11 tag-cut box unchanged); 29-ACCEPTANCE-AUDIT.md gained one dated "Re-sealed on 522ab1d4..., 2026-09-18" paragraph after its 2026-09-16 paragraph.
result: pass

### 6. [Plan 37-06 / SC3] Release PR #55 opened from feature/phase-33 and merged as a genuine merge commit
expected: PR #55 (feature/phase-33 -> main) is MERGED at 2026-09-18T21:21:45Z via two-parent merge commit 1d4a9724 whose tree is byte-identical to PR head 1bb94063; the D-12 pause hand-off pair exists and .continue-here.md carries a dated SUPERSEDED-IN-PART note; 37-CI-EVIDENCE.md's "Findings carried forward (D-00d)" holds the four maintainer statements verbatim with provenance (merge report, CodeQL advisory-only decision, D-17 bootstrap, in-session pre-tag section 11 sign-off), every orchestrator classification labelled as the orchestrator's own reading; the Provenance block's merge-commit token is filled with 1d4a9724.
result: pass

### 7. [Plan 37-07 / SC2] Pre-merge CI evidence on PR head 1bb94063, including the Coverage job figure
expected: 37-CI-EVIDENCE.md's CI-run table records all 9 workflow runs on PR head 1bb94063 (ci.yml, codeql.yml, pre-commit, feature-flags.yml push+pull_request, docs.yml) each success, skipped-by-design jobs named; the required-context tally reads 44/44 from the live ruleset (87 required check-run entries, 85 pass, 2 skipping, 0 failures); the red CodeQL results check is explained as 10 pre-existing rust/cleartext-logging alerts (#31-#47, created 2026-08-27) on a non-required context, not re-triaged; the Coverage job (both instances success) is recorded as the sole SC2 evidence with Lines 90.44% against the 82% floor read from scripts/coverage.sh; the deleted-branch method substitution (SHA-scoped gh api instead of --branch) is disclosed in the file; no push was made.
result: pass

### 8. [Plan 37-08 / SC3] Section 11 sign-off of record, and tag v0.10.0 sits on the main merge commit
expected: The maintainer's in-session statement "I sign section 11: the v0.10.0 tag may be cut on 1d4a9724, with the carried findings as recorded" is recorded verbatim in 37-CI-EVIDENCE.md as the pre-tag sign-off of record, with no agent editing the corpus audit file; annotated tag v0.10.0 (object 9282f4da, tagger Am0rfu5, subject "v0.10.0 Durable Agent Execution Runtime") peels to merge commit 1d4a9724 on origin, and ci.yml run 35396397097 on that commit concluded success (2026-09-18T23:05:47Z) before the tag push; the D-17 paladin-eval pre-tag gate re-read HTTP 200 / 0.0.1 / not yanked.
result: pass

### 9. [Plan 37-08 / D-16] Both release.yml failure diagnoses are recorded read-only, ending at the hard stop
expected: 37-CI-EVIDENCE.md (and 37-08-SUMMARY.md) record release.yml run 35404826303 with diagnosis #1 (Create Release failed on "printf: write error: Broken pipe", traced to the printf | head -n1 EPIPE race under pipefail, worsened by the 46,274-byte v0.10.0 response; GitHub Release v0.10.0 created with 0 assets as a side effect) and diagnosis #2 (publish-crates failed at position 4 of 12, paladin-battalion, "failed to select a version for the requirement paladin-llm = ^0.10.0", traced to the CRATES ordering defect where battalion's versioned dev-deps paladin-llm and paladin-storage publish at positions 5 and 10; exactly 3 of 12 crates on the registry: paladin-ai-core, paladin-ports, paladin-herald); the status reads HARD STOP (D-16), SC3 met, SC4 not met by this tag; the maintainer's recovery reply "A" is recorded verbatim; no agent re-ran, dispatched, published or yanked anything.
result: pass

### 10. [Phase close / SC4] Phase 37's record reads SC4 as not met by v0.10.0 and superseded by SHIP-06
expected: ROADMAP Phase 37 carries the dated 2026-09-19 status block (SC1-SC3 met; SC4 not met by tag v0.10.0, superseded by SHIP-06 via Phase 37.1's v0.10.1; SC5 not run) with plans 37-09, 37-10, 37-11 marked [~] superseded, each plan file opening with a dated note naming the Phase 37.1 plan that did the equivalent work; REQUIREMENTS.md's SHIP-05 keeps its original text, is never ticked, and carries the dated amend-at-source note reading superseded; 37-08-SUMMARY.md's Next Phase Readiness ends at the D-16 hard stop and defers everything after the "A" decision to Phase 37.1.
result: pass

### 11. [Phase close / SC4] MILESTONES.md carries the v0.10.0 release record alongside v0.9.0
expected: .planning/MILESTONES.md has a "v0.10.0 Durable Agent Execution Runtime" entry above the v0.9.0 entry stating: tag v0.10.0 on 1d4a9724 with only 3 of 12 crates published (ai-core, ports, herald), the two traced defects, v0.10.1 (tag on f7dae267) as the release with every publishable crate registry-verified at 0.10.1, the three orphaned 0.10.0 versions yanked by the maintainer with the "not defective — orphaned partial publish" disposition, the v0.10.0 GitHub Release kept, bannered and flipped to pre-release, and the requirement status (SHIP-06 satisfied, SHIP-05 superseded); the header reads "Released: 2026-09-21; milestone close pending".
result: pass

### 12. [Phase close / SC5] The milestone close is queued, not run, and the hand-off names it
expected: The ROADMAP "## Milestones" row for Durable Agent Execution Runtime still reads In progress (not Shipped), no milestones/v0.10.0-ROADMAP.md archive exists yet, and Phase 38 has not been started; STATE.md's stopped_at names the remaining order explicitly: /gsd-verify-work 37 and 37.1, then /gsd-audit-milestone and /gsd-complete-milestone v0.10.0. (SC5 is satisfied only by that later close; this test confirms nothing in Phase 37 or 37.1 pre-empted it.)
result: pass

## Summary

total: 12
passed: 12
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
