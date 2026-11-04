# Phase 57 Review Dispositions (REG-05)

**Date:** 2026-11-04
**Gate:** `./scripts/check-review-dispositions.sh` (derives every expected id from the archived `NN-REVIEW.md` headings, never from this file)

## Scope

- Source: `.planning/milestones/v0.11.0-phases/*/NN-REVIEW.md` (13 files; Phases 39 and 49 have no review).
- Every `### WR-<n>:` and `### CR-<n>:` heading (47 warnings + 2 criticals: PD-08-1, `38-REVIEW` CR-01 and `45-REVIEW` CR-01), plus Phase 40's `IN-01` and `IN-04` (D-12): 51 findings in total.
- Phase 50: 0 warnings, 0 criticals (`50-REVIEW.md` frontmatter `warning: 0`); it has no rows, and carries the appended pointer section only.
- Phase 42: the archived file is review iteration 2 and holds `WR-5` and `WR-6` (iteration 1's WR-1..4 were fixed before it was written, see `42-REVIEW-FIX.md`).
- Other info items are out of scope (D-12).

## Vocabulary (D-11)

- `closed`: fixed and still fixed; Evidence names a commit, test, requirement or REVIEW-FIX entry that was checked in this phase.
- `waived`: accepted by the owner; Evidence is the reason and the Owner column is filled.
- `open`: still live in the tree. Evidence reads `LIVE <file:line> — <correctness|security|other>: <one line>` (PD-08-2). The 57-10 triage decides fix or waive (D-14); 57-11 runs the checker strict, where an `open` row is a failure.

## Evidence rule (D-13)

The archive is read-only. Every `closed` pointer was checked in this phase: a commit that exists (`git cat-file`, or `gh api repos/DF3NDR/paladin-dev-env/commits/<sha>` for commits older than the shallow clone), a test re-run on the head, a requirement with evidence. No closure is asserted from memory.

## Dispositions

| Phase | Id | Title | Disposition | Evidence | Owner (if waived) |
|-------|----|-------|-------------|----------|-------------------|
| 40 | WR-01 | POST /v1/runs {thread_id} and fork run against another tenant's thread | closed | TENANT-04; submit_on_another_tenants_thread_is_rejected_as_unknown_thread and fork_on_another_tenants_thread_is_rejected_as_unknown_thread (re-run 2026-11-04: ok); 40-REVIEW-FIX 6d7efb16; 53 locks 2b16763d0, 66002e74b | |
| 40 | WR-02 | Tenant isolation for cancel enforced only in the HTTP controller | closed | TENANT-05; cancel_of_another_tenants_run_is_not_found_and_writes_nothing (re-run 2026-11-04: ok); 40-REVIEW-FIX fa054746 | |
| 40 | WR-03 | A corrupt or half-attributed row is a 500 that breaks hidden equals missing | closed | TENANT-06; corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal (re-run 2026-11-04: ok); 40-REVIEW-FIX 05612748, 9afaef2b; migration crates/paladin-storage/migrations/postgres/009_add_run_attribution_check.sql; 53 commits 30b532813, 839a8119a | |
| 40 | WR-04 | Serialization messages surface raw backend text to API clients | closed | TENANT-06; get_run_repository_error_is_a_generic_500_without_backend_text (re-run 2026-11-04: ok); 40-REVIEW-FIX 05612748 | |
| 40 | IN-01 | Reserved literals accepted as real tenant and key names | closed | TENANT-09; validate_rejects_reserved_literals_in_every_position (re-run 2026-11-04: ok); commit 4de0bcbc2 | |
| 40 | IN-04 | GET /agents/{id}/jobs/{job_id} ignores the principal and tenant | closed | TENANT-07; job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass (re-run 2026-11-04: ok); commits cc0077527, 010ee3158 | |

**Totals:** closed: 6, waived: 0, open: 0

## Live in tree

None yet.
