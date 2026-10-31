---
phase: 53-tenant-isolation-closure
plan: 07
subsystem: tenant-isolation
tags: [adr, docs, windows-ledger, review-dispositions, credential-review, tenant-isolation]

requires:
  - phase: 53-tenant-isolation-closure
    provides: "plans 53-01..53-06: permits_thread, redact_store_error_for_scope, load_visible_thread, visible_thread_page, get_job_scoped, boot guards"
provides:
  - "ADR-0063 (Accepted 2026-10-31) amending ADR-0054: thread, job and unreadable-row rules and the accepted residuals"
  - "PROMOTION.md index row 0063 and numbering line advanced to 0064; dated Amended-by note in ADR-0054"
  - "four user-doc pages describing the shipped behavior (tenant-visible threads, short list pages, scoped job polls, reserved literals)"
  - "WINDOWS rows 58 and 63 fixed on re-run evidence"
  - "53-TENANT-EVIDENCE.md: closing-test re-run, six Phase 40 dispositions for Phase 57, manual credential-handling review"
affects: [53-08, 57]

tech-stack:
  added: []
  patterns:
    - "Closing a ledger row only after its closing tests are re-run on the head and their result lines recorded"
    - "Manual credential review as a per-message table (location, four checks) with limits stated rather than a blanket pass"

key-files:
  created:
    - .planning/decisions/0063-thread-and-job-tenant-visibility.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/decisions/0054-tenant-scoped-run-reads.md
    - docs/src/user-guides/parley-and-chronicle.md
    - docs/src/api-reference/platform-api.md
    - docs/src/deployment-topologies/http-service-host.md
    - docs/src/getting-started/configuration.md
    - .planning/WINDOWS.md
    - .planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md

key-decisions:
  - "ADR-0063 is short and amending (PD-07-2): six numbered rules, residuals and evidence; it does not restate ADR-0054"
  - "WINDOWS rows 58 and 63 are closed in this phase (PD-07-1); row 63 closes on the boot warning, with no backfill claimed"
  - "Phase 40 review files are not edited (PD-07-3); dispositions live in the evidence file"

patterns-established:
  - "ADR amendment: dated Amended-by paragraph under the amended ADR's Date line, nothing else changed"

requirements-completed: [TENANT-03, TENANT-04, TENANT-05, TENANT-06, TENANT-07, TENANT-08, TENANT-09, TENANT-10]

coverage:
  - id: D1
    description: "ADR-0063 exists, Accepted and dated, with six numbered rules, residuals and evidence; ADR-0054 carries the Amended-by note; PROMOTION indexes 0063 and reads Next free ADR number: 0064"
    requirement: "TENANT-03"
    verification:
      - kind: other
        ref: "Task 1 automated verify (file, heading, submitted_at, PROMOTION line and row, Amended-by grep, one 0063 file); git diff of ADR-0054 shows 2 added lines and 0 removed"
        status: pass
    human_judgment: true
  - id: D2
    description: "User docs state the shipped behavior; the interim-posture note and the 'not yet tenant-scoped' paragraph are gone"
    requirement: "TENANT-07"
    verification:
      - kind: other
        ref: "Task 2 automated verify (negative grep for 'Interim authorization posture', greps for ADR-0063, next_cursor, reserved, enqueu); git diff --stat touches exactly the four listed docs"
        status: pass
    human_judgment: true
  - id: D3
    description: "WINDOWS rows 58 and 63 fixed with evidence pointers; open_count 15 -> 13, fixed_count 30 -> 32, total unchanged; no other row changed"
    requirement: "TENANT-10"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib thread_controller::tests::every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404 (1 passed); thread_controller::tests::list_threads_ (11 passed); cargo test --lib --features web-server schedule_scan_reports_every_unattributed_schedule_ascending_in_one_line (1 passed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Six Phase 40 findings (WR-01..WR-04, IN-01, IN-04) each have one disposition with a commit or test; credential review lists every new message with file:line and the four checks"
    requirement: "TENANT-06"
    verification:
      - kind: other
        ref: "Task 3 automated verify (rows fixed, matrix test named in row 58, both evidence headings, no Snyk mention)"
        status: pass
    human_judgment: true
---

# Phase 53 Plan 07: ADR-0063, docs, ledgers and the credential review Summary

**ADR-0063 records the phase's rules (latest-run thread owner, one `load_visible_thread` gate, bounded 500-thread list walk, `(agent id, tenant)` job rule, role-aware unreadable-row mapping) and amends ADR-0054; the four user docs now describe what shipped; WINDOWS rows 58 and 63 are fixed on re-run evidence; the Phase 40 findings have dispositions and the manual credential review is on the record.**

## Performance

- **Duration:** about 30 min wall clock (no start stamp was recorded; estimate)
- **Completed:** 2026-10-31
- **Tasks:** 3 (Task 1 tracer, Tasks 2 and 3 auto)
- **Files:** 1 created, 8 modified (all docs or planning files; no Rust source touched, no gate script run beyond the three closing tests)
- **Plan start SHA:** `908649c02` (the 53-06 docs commit)

## Accomplishments

- **ADR-0063** (`.planning/decisions/0063-thread-and-job-tenant-visibility.md`): six numbered rules naming `permits_thread`, `load_visible_thread`, `get_job_scoped` and `redact_store_error_for_scope`, the 500-thread cap in batches of 100, and the measured reason `get_job` carries no `#[deprecated]` (the patch gate's `function_marked_deprecated`). Consequences list the accepted residuals; Evidence names the tests from 53-01..53-06. ADR-0054 gained one dated `**Amended by ADR-0063**` paragraph (2 added lines, 0 removed). `PROMOTION.md` gained the 0063 row, the line `**Next free ADR number: 0064**` and a dated note; `ls .planning/decisions/0063-*.md` was empty before the write.
- **Docs:** `parley-and-chronicle.md` replaced the interim-posture blockquote with "Who can see a thread" and fixed the secret-template note that said any authenticated caller reads the state payload; `platform-api.md` `## Threads` gained the rule paragraph, the short-page `next_cursor` rule and the history `404`; `http-service-host.md` job-poll row and one thread sentence; `configuration.md` names `open-access`, `unattributed` and `anonymous` as reserved.
- **Ledgers:** the three closing tests were re-run on the head and recorded, then `windows fixed 58` and `windows fixed 63` were run and the reason cells carry the evidence pointers.
- **Evidence file:** `## Closing-test re-run (plan 53-07, T-53-37)`, `## Review dispositions` (six rows) and `## Manual credential-handling review` (twelve message rows, four checks each, CodeQL described as advisory-only, no Snyk mention).

## WINDOWS counters

| Counter | Before | After |
|---------|--------|-------|
| `open_count` | 15 | 13 |
| `waived_count` | 39 | 39 |
| `fixed_count` | 30 | 32 |
| `total_count` | 84 | 84 |

Only rows 58 and 63 changed (compared row by row against a pre-edit copy). The embedded JSON block in `WINDOWS.md` was kept in step with the table (13 open, 39 waived, 32 fixed, 84 entries).

## Task Commits

1. **Task 1 (tracer): ADR-0063, ADR-0054 note, PROMOTION advance** - `e1e888d3b` (docs)
2. **Task 2: user docs** - `f4af0329a` (docs)
3. **Task 3: WINDOWS rows, dispositions, credential review** - `4ec9e7b26` (docs)

Tracer feedback gate: the tracer is a documentation slice with no runnable end-to-end path; its `<verify>` (file, headings, PROMOTION line and row, Amended-by note, single 0063 file) was re-run after the commit and passed before Task 2 began.

## Verification

- Task 1, 2 and 3 `<automated>` verify commands all printed their OK marker (run as written).
- `53-TENANT-EVIDENCE.md` now has `## Prove-then-lock (D-00d)`, `## Semver probe`, `## Residuals`, `## Review dispositions` and `## Manual credential-handling review`; `grep -ci snyk` is 0.
- `mdbook` is not installed in this environment, so the docs were not built; the one new internal link (`[Threads](#threads)`) targets an existing `## Threads` heading, and no external URL was added.
- Not run: `make check-gates`, `cargo fmt`, `cargo clippy` (no Rust or gate-covered file changed; the offline gates cover changelogs, migration allowlist and similar, none of which this plan touched).

## Decisions Made

See `key-decisions`. All three were pre-made in the plan (PD-07-1 to PD-07-3) and followed.

## Deviations from Plan

**1. [Rule 1 - Stale doc] Extra rewrite in `platform-api.md`**
- **Found during:** Task 2
- **Issue:** a second paragraph in `platform-api.md` (the run-scope section) still said `/v1/threads/*` reads were "not yet tenant-scoped" and "deployment-wide for any authenticated principal", and the Gate-secret blockquote said the state payload goes to "any authenticated caller". The plan's acceptance criterion forbids any edited page from saying that.
- **Fix:** rewrote both statements to the shipped rule. Same four files, no extra file touched.
- **Commit:** `f4af0329a`

**2. [Rule 1 - Evidence accuracy] Phase 40 commit hashes cannot be resolved locally**
- **Found during:** Task 3
- **Issue:** `6d7efb16`, `fa054746`, `05612748` and `9afaef2b` (cited in `40-REVIEW-FIX.md`) are not in this clone's object database (`git cat-file` fails for all four); every Phase 53 hash in the evidence resolves.
- **Fix:** the dispositions quote the Phase 40 hashes as cited, say so in one sentence, and rest each closure on a Phase 53 commit and a named test that can be resolved here.

No other deviations. Rows 58 and 63 closed in this phase as PD-07-1 directs.

## Findings worth knowing (not blocking)

- **Credential review limit (stated in the evidence file, note 1).** The log lines that interpolate a store error (`run_controller.rs:724/727`, and the list-walk line at `thread_controller.rs:810`) write the error's `Display` text to the server log. For a `Serialization` error that is a serde decode message, which can quote a short fragment of the offending stored value. This phase adds no row text of its own and nothing reaches a response body (tests assert it), and the pre-existing `internal_repo_error` logs the same way, but a fragment of a stored row can reach the operator's log. The evidence file asks Phase 57 to decide whether decode-error text needs the `paladin-llm` redaction treatment. I did not change code for this (out of this plan's scope and files).
- **Row 63 closes on the warning, not on a backfill.** Schedules created before Phase 41 still fire unattributed until an operator re-creates them; the ledger reason and the evidence file say so.

## Known Stubs

None.

## Threat Flags

None. This plan changes documentation and planning files only. T-53-37 (rows closed without proof), T-53-38 (CodeQL read as coverage) and T-53-39 (docs promising the wrong isolation) are mitigated as planned: closing tests re-run before `windows fixed`; the evidence states CodeQL is advisory-only and has no Snyk mention; the stale posture note and the "not yet tenant-scoped" paragraph are removed.

## Next Phase Readiness

Plan 53-08 owns the binding semver run and `53-VERSION-DECISION.md`; this plan did not write that file. The ADR, docs and ledgers agree with the tree, so the version decision can be taken against them.

## Self-Check: PASSED

- FOUND: .planning/decisions/0063-thread-and-job-tenant-visibility.md
- FOUND: `**Next free ADR number: 0064**` in .planning/decisions/PROMOTION.md
- FOUND: `Amended by ADR-0063` in .planning/decisions/0054-tenant-scoped-run-reads.md
- FOUND: `## Review dispositions` and `## Manual credential-handling review` in 53-TENANT-EVIDENCE.md
- FOUND: WINDOWS rows 58 and 63 `fixed`; counters 13 / 39 / 32 / 84
- FOUND: commits `e1e888d3b`, `f4af0329a`, `4ec9e7b26`
