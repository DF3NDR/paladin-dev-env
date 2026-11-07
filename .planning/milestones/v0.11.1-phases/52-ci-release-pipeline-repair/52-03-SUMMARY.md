---
phase: 52-ci-release-pipeline-repair
plan: 03
subsystem: infra
tags: [codeql, code-scanning, governance, dismissal-register, issue-41, release-evidence]

requires:
  - phase: 52-01
    provides: "make check-gates and the tests/scripts harness conventions the register guard runs under"
provides:
  - "CODEQL-DISMISSALS.md: 31 new inspected [[dismissal]] rows (37 in total), review_date 2027-06-30 for every Phase 52 row, D-14 header sentence"
  - "52-CARRYOVER-EVIDENCE.md: issue #41 facts, the posted evidence comment, the maintainer's alert list verbatim, the 57-alert triage table, 37 platform dismissal commands"
  - "deferred-items.md: three out-of-scope findings (news-fetcher key in URL, three ungoverned advisories, two stale register line anchors) for plan 52-08's handover"
  - "Issue #41 closed (state_reason completed) with the fact-checked evidence comment present"
affects: [52-08]

tech-stack:
  added: []
  patterns:
    - "Register rows are written against the source line opened on the current tree, one hand-written justification per row, with a cross-reference (not a copied paragraph) where several sinks share one flow"
    - "A finding the plan does not define (advisory-type alerts in a CodeQL alert list) is recorded as out of scope with its governing control named, never forced into a register it does not belong to"

key-files:
  created:
    - .planning/phases/52-ci-release-pipeline-repair/52-CARRYOVER-EVIDENCE.md
    - .planning/phases/52-ci-release-pipeline-repair/deferred-items.md
  modified:
    - CODEQL-DISMISSALS.md

key-decisions:
  - "The 20 OSV-Scanner advisory alerts are out of the CodeQL register's scope: 17 are governed by SECURITY-EXCEPTIONS.md rows (by id or alias), 3 (alerts 25, 24, 23) are ungoverned and stay open for the maintainer"
  - "No alert was fixed: none of the 31 newly triaged CodeQL alerts is a genuine leak that a few-line, signature-neutral change would remove"
  - "New rows are merged in ascending alert_number order between the existing blocks (not appended after alert 49), because 25 of the 31 new numbers are below 49 and the plan's own verify asserts the whole list is sorted"
  - "The six earlier rows are untouched even where their line anchors are stale (rows 40 and 49), per D-14; the staleness is logged in deferred-items.md"

patterns-established:
  - "The platform dismissal commands cover the six earlier rows too, because their alerts are still open in the maintainer's list"

requirements-completed: [CIREL-06, CIREL-07]

coverage:
  - id: D1
    description: "Every CodeQL rule alert in the maintainer's list has exactly one disposition; every registered alert has a full eleven-field row; the guard and its harness are green"
    requirement: "CIREL-06"
    verification:
      - kind: integration
        ref: "make check-codeql-dismissals (37 rows) and ./tests/scripts/check-codeql-dismissals_test.sh (11 assertions)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Each new row's justification matches what the source line does (judgment: opened on the current tree and recorded per row)"
    requirement: "CIREL-06"
    verification: []
    human_judgment: true
    rationale: "The plan marks the transparency prohibition as judgment; a reviewer should spot-check a few rows against their source lines"
  - id: D3
    description: "Issue #41 closed with the fact-checked evidence comment"
    requirement: "CIREL-07"
    verification:
      - kind: manual
        ref: "public API re-read 2026-10-28T23:21:02Z: state closed, state_reason completed, comments 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "The platform side of the register (the code-scanning dismissals) matches the register"
    requirement: "CIREL-06"
    verification: []
    human_judgment: true
    rationale: "The sandbox token cannot write code-scanning alerts; the 37 PATCH commands are handed to the maintainer as H-04"

duration: about 51min wall-clock (includes the maintainer checkpoint)
completed: 2026-10-28
status: complete
---

# Phase 52 Plan 03: CodeQL triage and issue #41 close Summary

**Thirty-one inspected, source-specific rows (alerts 30 through 81) now sit in `CODEQL-DISMISSALS.md` beside the six earlier ones, all dated 2027-06-30 for one sweep, and issue #41 is closed with an evidence comment whose every fact was read from the public GitHub API.**

## Performance

- **Duration:** about 51 min wall-clock, including the wait for the maintainer's checkpoint reply
- **Completed:** 2026-10-28
- **Tasks:** 3 of 3 (Task 1 tracer, Task 2 checkpoint, Task 3 triage)
- **Files:** 2 created, 1 modified (plus this summary and the planning-state files)

## Accomplishments

- CIREL-06 / D-13: the maintainer's alert list is recorded verbatim, and every one of its 37 CodeQL rule alerts has a disposition. Six were already registered; 31 were opened on the current tree and registered (false positive 19, used in tests 11, won't fix 1). No alert was fixed.
- CIREL-06 / D-14: every added row carries `review_date = "2027-06-30"` (31 occurrences), the register header says so in one sentence, `Declared dismissals: 37` and `## The 37 live dismissals` agree with the 37 rows, and the six earlier rows are byte-identical (`git diff e41cd87e4 -- CODEQL-DISMISSALS.md` removes exactly two lines: the old count line and the old heading).
- CIREL-06 posture: `.github/workflows/codeql.yml` is unchanged; the snyk count in the register is 0 before and after; nothing added describes CodeQL as a required or merge-gating check.
- CIREL-07 / D-15: the evidence comment was posted by the sandbox over REST (https://github.com/DF3NDR/paladin-dev-env/issues/41#issuecomment-6482461863), the maintainer closed the issue, and the public re-read confirms `state` `closed`, `state_reason` `completed`, `closed_at` 2026-10-28T23:09:45Z, one comment. No handover item H-05 is needed.
- 31 `PATCH /code-scanning/alerts/<n>` commands for the new rows, plus 6 for the earlier rows whose alerts are still open on the platform, and the reconciliation command, are in the evidence file for plan 52-08 to hand over as H-04.

## Task Commits

1. **Task 1 (tracer): public facts, evidence comment, alert-pull attempt** - `2acba84d1` (docs)
2. **Task 2 (checkpoint:human-action): maintainer supplies the alert list and closes #41** - no commit; the reply is recorded below and in the evidence file
3. **Task 3: triage into CODEQL-DISMISSALS.md** - `697afb221` (docs)

Metadata commit: see git log (docs(52-03)).

## Tracer evidence (Task 1)

- **Public reads (HTTP 200, unauthenticated):** the v0.11.0 Release (`published_at` 2026-10-26T20:29:52Z, ten assets), run 40396108246 on merge commit `70657919beadbd341a72be2d0107080017a803f0` (12 jobs, all `success`), and issue #41 (open, 0 comments at the time). Values and read times are in the evidence file's `## Issue #41` table.
- **Comment write:** `gh issue comment` was refused (`HTTP 403: GitHub GraphQL is not available from Claude Code sessions; use the REST API`), a transport refusal rather than an authorization verdict. The REST equivalent (`gh api --method POST .../issues/41/comments`) succeeded: comment id 6482461863, created 2026-10-28T22:36:39Z. The platform appended a trailer line to the posted comment (`_Generated by [Claude Code](https://claude.ai/code)_`); it is not part of the saved comment text.
- **Close:** the REST close was denied by the Claude Code permission system (External System Writes) before it ran, was not retried, and no workaround was attempted. The maintainer closed the issue instead.
- **Alert read:** code-scanning alert reads returned HTTP 403 `Resource not accessible by integration` on both refs, authenticated and unauthenticated, so the alert list was a maintainer hand-off.
- **Tracer gate:** the tracer's verify (the evidence file's greps, including the negative token-shaped-string grep) was re-run in Task 3 and passes; the interactive checkpoint that followed was the plan's own Task 2.

## Checkpoint reply

Recorded at 2026-10-28T23:16:37Z (the orchestrator saved the reply to its scratchpad at that time; the maintainer's close of #41 is dated 2026-10-28T23:09:45Z by the public API). Verbatim, as saved (tab separators were flattened to runs of spaces in transit):

````
`issue41: closed`


```
86      RUSTSEC-2026-0187               refs/heads/main Cargo.lock:1
85      RUSTSEC-2026-0369               refs/heads/main Cargo.lock:1
84      RUSTSEC-2026-0368               refs/heads/main Cargo.lock:1
83      RUSTSEC-2026-0367               refs/heads/main Cargo.lock:1
81      rust/cleartext-logging  high    refs/heads/main crates/paladin-llm/src/config/layers.rs:750
80      rust/cleartext-logging  high    refs/heads/main crates/paladin-llm/src/config/layers.rs:700
79      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/paladin_builder.rs:1558
78      rust/cleartext-logging  high    refs/heads/main examples/paladin_with_config.rs:196
77      rust/cleartext-logging  high    refs/heads/main examples/citadel_restore.rs:135
76      rust/cleartext-logging  high    refs/heads/main examples/citadel_autosave.rs:116
70      RUSTSEC-2024-0449               refs/heads/main Cargo.lock:1
69      rust/cleartext-logging  high    refs/heads/main src/config/treasurer.rs:1583
68      rust/cleartext-logging  high    refs/heads/main src/config/treasurer.rs:1567
67      rust/cleartext-logging  high    refs/heads/main src/config/treasurer.rs:1566
66      rust/cleartext-logging  high    refs/heads/main src/config/treasurer.rs:1556
65      rust/cleartext-logging  high    refs/heads/main src/config/treasurer.rs:1555
63      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/infrastructure/security/encryption.rs:340
62      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/infrastructure/security/encryption.rs:333
61      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/infrastructure/security/encryption.rs:281
60      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/infrastructure/security/encryption.rs:172
54      rust/cleartext-transmission     high    refs/heads/main crates/paladin-content/src/adapters/input/news_api_fetcher.rs:143
53      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/core/platform/manager/user_service.rs:724
52      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/core/platform/manager/user_service.rs:719
51      rust/cleartext-logging  high    refs/heads/main src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:202
50      rust/cleartext-logging  high    refs/heads/main src/infrastructure/adapters/scheduling/tokio_cron_adapter.rs:166
49      rust/cleartext-logging  high    refs/heads/main src/bin/paladin-server.rs:472
47      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/prompt_generation_service.rs:169
46      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/prompt_generation_service.rs:119
45      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/prompt_generation_service.rs:112
44      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/paladin_builder.rs:1591
43      rust/cleartext-logging  high    refs/heads/main src/application/services/paladin/paladin_builder.rs:1518
42      rust/cleartext-logging  high    refs/heads/main src/application/services/orchestration/mod.rs:807
41      rust/cleartext-logging  high    refs/heads/main src/application/services/orchestration/mod.rs:797
40      rust/cleartext-logging  high    refs/heads/main crates/paladin-memory/src/services/rag_retrieval_service.rs:360
39      rust/cleartext-logging  high    refs/heads/main crates/paladin-memory/src/citadel/file_citadel.rs:175
37      rust/cleartext-logging  high    refs/heads/main examples/citadel_restore.rs:142
36      rust/cleartext-logging  high    refs/heads/main examples/citadel_restore.rs:141
34      rust/cleartext-logging  high    refs/heads/main examples/citadel_restore.rs:76
33      rust/cleartext-logging  high    refs/heads/main examples/citadel_autosave.rs:130
31      rust/cleartext-logging  high    refs/heads/main examples/citadel_autosave.rs:81
30      rust/cleartext-logging  high    refs/heads/main examples/battalion_checkpoint_recovery.rs:87
28      rust/hard-coded-cryptographic-value     critical        refs/heads/main src/core/platform/manager/user_service.rs:1582
26      RUSTSEC-2026-0249               refs/heads/main Cargo.lock:1
25      RUSTSEC-2026-0221               refs/heads/main Cargo.lock:1
24      RUSTSEC-2026-0205               refs/heads/main Cargo.lock:1
23      GHSA-7gcf-g7xr-8hxj             refs/heads/main Cargo.lock:1
21      RUSTSEC-2026-0195               refs/heads/main Cargo.lock:1
19      RUSTSEC-2026-0194               refs/heads/main Cargo.lock:1
17      RUSTSEC-2026-0187               refs/heads/main Cargo.lock:1
14      RUSTSEC-2024-0436               refs/heads/main Cargo.lock:1
13      RUSTSEC-2025-0134               refs/heads/main Cargo.lock:1
11      RUSTSEC-2025-0119               refs/heads/main Cargo.lock:1
10      CVE-2025-62518          refs/heads/main Cargo.lock:1
9       RUSTSEC-2025-0057               refs/heads/main Cargo.lock:1
6       CVE-2023-49092          refs/heads/main Cargo.lock:1
4       RUSTSEC-2021-0141               refs/heads/main Cargo.lock:1
2       CVE-2025-62518          refs/heads/main Cargo.lock:1
```
````

**Count discrepancy:** the roadmap said 24 alerts; the relayed summary of this reply said 60; the saved paste holds **57** rows with 57 distinct alert numbers (37 CodeQL rule alerts and 20 advisory-type alerts). The saved paste is the authority and 57 is the number triaged. The roadmap's 24 is the set of 24 annotations on PR #59's head, and all 24 of those locations appear in the list.

## Verification (plan level)

- `make check-codeql-dismissals`: exit 0, "37 governed dismissal(s) checked; schema, drift, staleness, uniqueness and reachability clauses all pass".
- `./tests/scripts/check-codeql-dismissals_test.sh`: 11 assertions passed. Note: its closing "no mutation" assertion (`git status --porcelain -- CODEQL-DISMISSALS.md` empty) fails while the register has uncommitted edits, so it only goes green after the Task 3 commit. It was run red before the commit (expected) and green after.
- `make check-gates`: exit 0.
- The plan's python check: row count 37 equals `Declared dismissals: 37` equals the heading's 37; `alert_number` list strictly ascending with no duplicates.
- `git diff --quiet HEAD -- .github/workflows/codeql.yml`: clean. `grep -ci snyk CODEQL-DISMISSALS.md`: 0 before and after. `grep -c 'review_date = "2027-06-30"'`: 31 (rows added by this plan).
- No token-shaped string in the evidence file (the Task 1 negative grep re-run).
- Not run: no `cargo clippy`, `cargo test` or `make api-surface`, because no Rust source file was changed (no alert was fixed). No code-scanning write was possible or attempted.

## Decisions Made

- **Advisory-type alerts.** See Deviations, item 1.
- **Disposition of each registered row.** `false positive` for the Paladin-UUID and session-UUID logs, the builder over-approximation, the two zero-buffer-then-`OsRng` initialisers in production code, and the literal-`https://` news URL; `used in tests` for the test-module literals and assertion messages; `won't fix` for alert 78 only (the `println!` of `PaladinData::user_name` in an example is an accurate flow whose value is a hard-coded demo literal, so calling it a false positive would overstate).
- **Alert 54 is registered as a false positive for the alert as named only.** The row says in its own text that it does not certify the surrounding credential handling (see the deferred items).

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan gap - disposition for advisory-type alerts] 20 OSV-Scanner alerts recorded as out of the CodeQL register's scope**
- **Found during:** Task 3 (intake of the maintainer's list)
- **Issue:** the plan's dispositions are fixed or registered, but 20 of the 57 alerts are not CodeQL results: their "rule id" is a RUSTSEC / CVE / GHSA identifier at `Cargo.lock:1`, uploaded by the `osv-scanner` job in `.github/workflows/ci.yml` through `upload-sarif`. The register describes itself as the register for CodeQL alerts.
- **Decision:** no row and no platform dismissal for them; each is listed in the triage table as out of scope with its governing control named. 17 map to a `SECURITY-EXCEPTIONS.md` row (14 by advisory id, alerts 10, 2 and 6 by alias: CVE-2025-62518 is an alias of RUSTSEC-2025-0111, CVE-2023-49092 of RUSTSEC-2023-0071), so a second row would put two sources of truth on one risk. Three have no governing row: alerts 25 (`RUSTSEC-2026-0221`, `event-listener` 5.4.1) and 24 (`RUSTSEC-2026-0205`, `scc` 2.4.0) are informational `unsound` advisories on crates present in `Cargo.lock`, and alert 23 (`GHSA-7gcf-g7xr-8hxj`) could not be identified offline. Dismissing real ungoverned findings to finish a list would be the failure the register exists to prevent, so they stay open and go to the handover. Rationale and the maintainer option to dismiss the 17 governed ones are in the evidence file.
- **Files modified:** `52-CARRYOVER-EVIDENCE.md`, `deferred-items.md`
- **Commit:** `697afb221`

**2. [Rule 3 - Blocking: contradiction inside the plan] New rows are merged in ascending order, not appended after alert 49**
- **Found during:** Task 3
- **Issue:** the plan says to append new rows "in ascending order after alert 49", but 25 of the 31 new alert numbers (30 to 44) are below 49, and the plan's own verify asserts the entire `alert_number` list is sorted. Appending would fail that assertion.
- **Fix:** the new blocks are inserted at their numeric position between the existing blocks. The existing blocks are not edited, so the diff against `e41cd87e4` removes only the old count line and the old heading.
- **Commit:** `697afb221`

**3. [Rule 2 - Missing coverage of the plan's own intent] Platform commands also cover the six earlier rows**
- **Found during:** Task 3
- **Issue:** alerts 28, 40, 45, 46, 47 and 49 already had register rows but appear in the maintainer's open list, so their platform dismissal never happened; the plan's step 4 only asks for commands for alerts registered by this plan.
- **Fix:** a second command group for those six (reasons copied from their rows). Without it the register and the alert store would still disagree after the maintainer ran the first group.
- **Commit:** `697afb221`

**4. [Attribution] Commit trailer model name**
- The orchestrator brief named `Co-Authored-By: Claude Fable 5.1`; the harness attribution reminder names `Claude Sonnet 5.5`, which is the model running this executor. The Task 3 commit carries the Sonnet 5.5 trailer (amended once, immediately, before any further work). Task 1's commit `2acba84d1`, made by the earlier executor, carries the Fable 5.1 trailer and was not rewritten. Both carry the same `Claude-Session` line.

## Auth gates

- Task 1: `gh issue comment` (GraphQL) refused with HTTP 403; the REST route succeeded. The REST close was denied by the permission system. Documented as the normal flow of this plan's checkpoint, not as a failure. The token in `GH_TOKEN` is reported invalid by `gh auth status` yet the REST comment was accepted; this is recorded in the evidence file as observed.

## Notes for plan 52-08 (handover)

- **H-04:** the 37 PATCH commands and the reconciliation command are in `52-CARRYOVER-EVIDENCE.md` under `## Platform dismissal commands` (31 new, 6 earlier). The 17 SECURITY-EXCEPTIONS-governed advisory alerts are an optional, separate maintainer decision.
- **H-05 is not needed:** issue #41 is closed.
- **From `deferred-items.md`:** D-52-03-01 (the News API key sits in the request URL and `reqwest` 0.12.28's error Display writes the URL; the client follows redirects), D-52-03-02 (alerts 25, 24, 23 have no governing exception), D-52-03-03 (rows 40 and 49 carry stale line numbers; D-14 forbids editing them here).

## Issues Encountered

- The register harness's "no mutation" assertion cannot pass until the register is committed (see Verification). Not a defect; the harness is meant to run on a clean tree.
- `api.osv.dev` is not reachable from the sandbox (proxy refuses the tunnel), so advisory identification used the local RustSec database at `~/.cargo/advisory-db` (last commit 2026-10-27). Alert 23 is the only identifier it could not resolve.

## Known Stubs

None. (No source file with data wiring was created or changed.)

## Threat Flags

None. No new network endpoint, auth path or trust-boundary file access was introduced. T-52-11 to T-52-14 are mitigated as planned (inspected rows, equal counts, guard green, `codeql.yml` diff empty, token-shaped-string grep clean, evidence comment facts read from the public API); T-52-15 accepted as planned. The one security-relevant discovery (D-52-03-01) predates this plan and is deferred, not introduced.

## Self-Check: PASSED

Files verified present: `CODEQL-DISMISSALS.md`, `.planning/phases/52-ci-release-pipeline-repair/52-CARRYOVER-EVIDENCE.md`, `.planning/phases/52-ci-release-pipeline-repair/deferred-items.md`. Commits `2acba84d1` and `697afb221` verified in git log.
