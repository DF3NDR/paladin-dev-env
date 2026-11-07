# Phase 57: Register Hygiene Close-Out - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-04
**Phase:** 57-register-hygiene-close-out
**Areas discussed:** WINDOWS rows 85-87 + re-homing, Review-warning disposition home, Phase 40 judgment-tier read, zip exceptions: lift or re-date, Evidence file shape, REG-01 frontmatter form, Scope gate, Todo cross-reference

---

## Scope gate (asked with the area selection)

| Option | Description | Selected |
|--------|-------------|----------|
| Defer both | Keep Phase 57 register-only; record the check-gates key-shape scan and the Phase 54 advisory follow-ups as deferred ideas | ✓ |
| Fold the key-shape scan only | Add the tracked-tree key-shape scan to check-gates here | |
| Fold both | Also carry the four Phase 54 advisory fixes | |

**User's choice:** Defer both.

## Todo cross-reference

| Option | Description | Selected |
|--------|-------------|----------|
| Fold none | Seven keyword-matched todos stay pending, listed as reviewed-not-folded | ✓ |
| Let me pick | Name the ones to fold | |

**User's choice:** Fold none.

---

## WINDOWS rows 85-87 + re-homing

**Q1 — What happens to rows 85-87 in Phase 57?**

| Option | Description | Selected |
|--------|-------------|----------|
| Fix all three here | Small code plan: skip-and-log UnknownSchemaVersion, newest-run port method, earliest-attributed ownership; TDD, additive; rows close as fixed | ✓ |
| Re-home to a new 57.1 phase | Decimal phase for the fixes; rows waived in 57 with 57.1 as owner | |
| Waive to the next milestone | Ship v0.11.1 with the defects documented | |
| Fix 85 and 86, waive 87 | Mechanical fixes only; ownership semantics documented and waived | |

**Q2 — Which ownership rule does the row 87 fix implement?**

| Option | Description | Selected |
|--------|-------------|----------|
| Earliest attributed run owns | First run carrying a tenant owns; Admin forks and unattributed runs never move it; ADR-0063 and parley-and-chronicle.md amended | ✓ |
| Keep latest-run, exempt Admin and unattributed | Latest run owns except Admin/None keep the previous owner | |
| Document only, no code | Keep current behaviour, record in ADR-0063, waive row 87 | |

**Q3 — How are rows 64, 65, 66, 77 and 79 dispositioned with only waive/fixed available?**

| Option | Description | Selected |
|--------|-------------|----------|
| Waive with a structured reason | `Re-homed: owner=, revisit=, basis=` shape in the waive reason; no tool change | ✓ |
| Waive rows 64-66 and 77; re-open 79's probe | Re-run the 50.1 seed probe on an operator machine at UAT | |
| Extend the tool with a `rehome` status | New gsd-tools status and command | |

**Q4 — What evidence closes rows 83 and 84?**

| Option | Description | Selected |
|--------|-------------|----------|
| Cite the CI run id | Read the Workflow Lint result on the PR #61 merge commit via the GitHub API; record the run id | ✓ |
| Run actionlint locally too | Also install/run actionlint in the sandbox | |
| Waive with the CI job as owner | No evidence lookup | |

**User's choices:** Fix all three here; Earliest attributed run owns; Waive with a structured reason; Cite the CI run id.
**Notes:** User moved to the next area after four questions.

---

## Review-warning disposition home

**Q1 — Where does the per-warning disposition live?**

| Option | Description | Selected |
|--------|-------------|----------|
| One 57 register + archive pointer | `57-REVIEW-DISPOSITIONS.md` plus an appended `## Dispositions (Phase 57)` section per archived REVIEW.md | ✓ |
| In-place beside each warning | Edit every WR-nn heading in 14 archived files | |
| Register only, archive untouched | No archive edits | |

**Q2 — Vocabulary and bar for closed?**

| Option | Description | Selected |
|--------|-------------|----------|
| closed / waived / open, evidence required | closed needs commit/test/requirement pointer; waived needs reason + owner; open not allowed at seal | ✓ |
| fixed / accepted / deferred, free text | Looser, evidence optional | |
| Reuse the WINDOWS statuses | Every unfixed warning becomes a WINDOWS row | |

**Q3 — How far does the sweep reach?**

| Option | Description | Selected |
|--------|-------------|----------|
| Warnings + the two named info items | 47 warnings plus Phase 40 IN-01 and IN-04 | ✓ |
| Warnings + every Phase 40 info item | Phase 40 fully, other info items untouched | |
| Every warning and every info item | 121 rows | |

**Q4 — A warning found still live in the tree?**

| Option | Description | Selected |
|--------|-------------|----------|
| Mint a WINDOWS row, then waive or fix by severity | Correctness/security fixed here; others waived with the structured reason | ✓ |
| Waive all as carried to v0.12, no new rows | No tracking beyond the register | |
| Mint rows, fix nothing extra | All new rows waived with an owner | |

**User's choices:** One 57 register + archive pointer; closed / waived / open with evidence; warnings plus Phase 40 IN-01/IN-04; mint a WINDOWS row then fix or waive by severity.

---

## Phase 40 judgment-tier read

**Q1 — Who performs the bounded human read and how is it staged?**

| Option | Description | Selected |
|--------|-------------|----------|
| Claude prepares a worksheet, you confirm at UAT | `57-JUDGMENT-READ.md`, 11 test-backed rows pre-filled, 9 code-reading rows re-read with file:line evidence; owner confirms/rejects per row | ✓ |
| You read all 20 unaided | Owner records verdicts from the 40-VERIFICATION table and the Phase 40 diff | |
| Human read of the 9 code-reading rows only | Test-backed rows closed on tests without human eyes | |

**Q2 — Repeat the credential-handling review, and over what range?**

| Option | Description | Selected |
|--------|-------------|----------|
| Repeat over Phase 40's own diff range | Checklist re-run over the Phase 40 commits only, file:line evidence | ✓ |
| Repeat over the current tenant/auth surface | agent_auth.rs, run_controller.rs, submission.rs, principal.rs as they stand | |
| Accept the 40-06 record, no re-run | Cite the 40-06 SUMMARY | |

**Q3 — Where do the outcomes land?**

| Option | Description | Selected |
|--------|-------------|----------|
| Worksheet in 57 + gap-row updates in the archived VALIDATION files | 40-VALIDATION gap row amended; 42-VALIDATION gets an accepted-as-is row; nothing else changes | ✓ |
| Amend 40-VERIFICATION.md and 42-VERIFICATION.md in place | Flip verdicts, change `human_needed` to `passed` | |
| Worksheet only, archive untouched | | |

**Q4 — If a verdict is rejected at UAT?**

| Option | Description | Selected |
|--------|-------------|----------|
| Mint a WINDOWS row; fix or waive by severity | Credential/tenant-isolation violations fixed here; others waived | ✓ |
| Fix in this phase regardless of severity | No waiver path | |
| Record the rejection, defer every fix | Row minted and waived to v0.12 | |

**User's choices:** Claude prepares the worksheet, owner confirms at UAT; repeat over Phase 40's own diff range; worksheet plus VALIDATION gap-row updates; mint a WINDOWS row and fix or waive by severity.

---

## zip exceptions: lift or re-date

Context surfaced during discussion: `utoipa-swagger-ui` 10.0.2 depends on `zip ^9.0` (build) and `utoipa ^6.0.0` (crates.io, checked 2026-11-04); the tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2.

**Q1 — Lift or re-date?**

| Option | Description | Selected |
|--------|-------------|----------|
| Re-date, record the check, route the lift to v0.12 | Rows kept with scope and compensating control; fields rewritten to name the now-available fix and its migration cost | ✓ |
| Lift now: migrate utoipa 5 -> 6 in this phase | Bump utoipa, utoipa-axum, utoipa-swagger-ui; regenerate openapi.json; delete rows | |
| Try the lift behind a time box, fall back to re-date | One attempt plan | |

**Q2 — New review date and revisit condition?**

| Option | Description | Selected |
|--------|-------------|----------|
| Date = v0.12 roadmap open; condition = utoipa 6 migration lands | Condition plus fallback date no later than 2027-03-31; row text records the met upstream condition | ✓ |
| Keep 2026-12-31, change the condition only | Falls due in Phase 58's window | |
| Roll forward six months, keep the condition | Register would state a satisfied condition as trigger | |

**Q3 — Where is the check recorded?**

| Option | Description | Selected |
|--------|-------------|----------|
| Row text + a dated history note in SECURITY-EXCEPTIONS.md | why_not_fixable cites the crates.io listing; history paragraph gets a dated entry | ✓ |
| Row text + a 57 evidence file | Raw crates.io output and make security run in a 57 file | |
| Row text only | | |

**User's choices:** Re-date and route the lift to v0.12; review date at the next milestone open with a 2027-03-31 fallback and a utoipa-6-migration condition; row text plus a dated history note.

---

## Evidence file shape

| Option | Description | Selected |
|--------|-------------|----------|
| One 57-REGISTER-EVIDENCE.md plus the two registers | Evidence file + 57-REVIEW-DISPOSITIONS.md + 57-JUDGMENT-READ.md | ✓ |
| Everything in one 57-REGISTER-EVIDENCE.md | | |
| Per-register evidence files | Six files | |

**User's choice:** One evidence file plus the two registers.

## REG-01 frontmatter form

| Option | Description | Selected |
|--------|-------------|----------|
| Add to the plan that finished each | STORE-02 on 45-01-SUMMARY; REASON ids on the last SUMMARY that progressed each per 51-VERIFICATION; requirements-progress untouched | ✓ |
| Add all five to the final SUMMARY of each phase | 45-07 and 51-13 | |
| Teach the audit to read requirements-progress | gsd-tools change | |

**User's choice:** Add to the plan that finished each.

---

## Closing check

| Option | Description | Selected |
|--------|-------------|----------|
| I'm ready for context | Write 57-CONTEXT.md | ✓ |
| Explore more gray areas | Phase 52-56 REVIEW warnings, rows 85-87 UAT shape, audit G1/G2 | |

## Claude's Discretion

- Plan ordering and count; whether rows 85-87 are one plan or two.
- Exact `windows waive` reason text per row within the structured shape.
- Layout of the three Phase 57 files.
- How the Workflow Lint run id is retrieved and the fallback if the API refuses.
- Which `run_repo_error_for_scope` arm handles `UnknownSchemaVersion` (row 85).
- UAT shape for rows 85-87 beyond the regression tests.
- Archiving the three debug sessions to `.planning/debug/resolved/` with `status: resolved`.

## Deferred Ideas

- Tracked-tree API-key-shape scan in `check-gates` (Phase 56 close).
- Phase 54 advisory review follow-ups (WR-01, WR-04, WR-05, IN-02).
- utoipa 5 -> 6 migration lifting RUSTSEC-2026-0367/0368/0369 (next feature milestone).
- A first-class `rehomed` WINDOWS status in gsd-tools.
- Phase 52-56 REVIEW warnings and audit integration warnings G1/G2 (Phase 58 pre-tag audit).
- Seven reviewed-not-folded pending todos (listed in CONTEXT.md).
