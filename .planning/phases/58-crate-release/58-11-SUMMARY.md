---
phase: 58-crate-release
plan: 11
subsystem: release-signoff
tags: [release, tag-day, codeql, dismissals, signoff, ship-08, d-04, d-00a]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-10 (CI evidence on SHA-A0, milestone audit passed, maintainer decisions redate / dismiss-now / validate-first)
provides:
  - "twelve changelog headings dated to the tag day 2026-11-06 (D-16/D-00n), re-seal row 7 re-read green"
  - "eight governed CODEQL-DISMISSALS.md entries for the F-58-08 alerts 93-100 (register 37 to 45)"
  - "SHA-A1 e56d4d4ae10b5b0793d020b4c362306f20dd8003: evidence pushed before the sign-off"
  - "SHA-A2 dffa996fa84c7b35c69f95987231426296f864b2: the maintainer's tick, verified read-only on origin"
  - "resume file for the CI-on-SHA-A2 stop"
affects: [58-12, 58-13, 58-14]

tech-stack:
  added: []
  patterns:
    - "Evidence first, push as SHA-A1, then the maintainer's signature as the very next commit; the agent verifies parent, file list, changed token and PR head read-only"
    - "Platform writes (CodeQL dismissals, the sign-off tick) belong to the maintainer; the agent registers the governed record and hands over exact commands"

key-files:
  created:
    - .planning/phases/58-crate-release/58-11-SUMMARY.md
  modified:
    - CHANGELOG.md
    - "crates/*/CHANGELOG.md (eleven publishable crates)"
    - CODEQL-DISMISSALS.md
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/phases/58-crate-release/58-ACCEPTANCE-AUDIT.md
    - .planning/phases/58-crate-release/.continue-here.md

key-decisions:
  - "Tag day is 2026-11-06: one docs(release) commit changed only the twelve heading lines (1f6e639f2)"
  - "The eight F-58-08 alerts are registered as false positives with owner, review date 2027-06-30, scope and compensating controls; the GitHub dismissal is the maintainer's write and is taken on their reply"
  - "The tick was verified against origin, not against the maintainer's word: single parent SHA-A1, one file, one token, PR head equal to SHA-A2"

requirements-completed: []

duration: "Task 1 about 2026-11-06T13:30Z to 14:07Z, checkpoint pause to 15:13Z, Task 3 about 15:13Z to 15:20Z"
completed: 2026-11-06
---

# Phase 58 Plan 11: Tag Day, CodeQL Register, Evidence Push and the Maintainer's Tick Summary

**The twelve changelog sections are dated to the tag day, the eight F-58-08 CodeQL alerts are in the governed register, the evidence was pushed as SHA-A1, and the maintainer's tick (SHA-A2 `dffa996fa`) is verified on origin as a one-token change on top of SHA-A1 and the PR 64 head.**

## Task results

| Task | Name | Commit | Result |
|------|------|--------|--------|
| 1a | Redate the twelve `## [0.11.1]` headings to 2026-11-06 (D-16/D-00n) | `1f6e639f2` | 12 insertions / 12 deletions, heading lines only; re-seal row 7 re-read green (`make check-gates`, `make check-changelogs`, `make check-release-consistency RELEASE_TAG=v0.11.1`, 12 of 12 heading greps, release-body extraction 16,075 characters) |
| 1b | Register the eight F-58-08 alerts (93-100) | `045e1aef0` | `CODEQL-DISMISSALS.md` 37 to 45 entries; `make check-codeql-dismissals` exit 0 |
| 1c | Evidence before the sign-off, pushed plain as SHA-A1 | `e56d4d4ae` | `58-CI-EVIDENCE.md` 58-11 section, `58-ACCEPTANCE-AUDIT.md` row 7 re-read; remote tip and PR head equalled SHA-A1; `main` unmoved |
| 2 | The maintainer's tick (human action, `blocking-human`) | `dffa996fa` (Am0rfu5, `docs(58): sign off the v0.11.1 tag`) | reply `ticked` at 2026-11-06T15:13Z |
| 3 | Verify the tick read-only, record SHA-A2, rewrite the resume file | `75f36c72d` | all checks below pass; local branch fast-forwarded to SHA-A2 |

## Tag-day handling (D-16 / D-00n)

The maintainer's `redate:2026-11-06` (plan 58-10) was carried out exactly: one `docs(release)` commit changed line 10 of `CHANGELOG.md` and the eleven `crates/*/CHANGELOG.md` headings from `2026-11-05` to `2026-11-06`, and nothing else. `- STAMP:` in Provenance reads `2026-11-06` with a note. Row 7 of the acceptance audit was re-read on that head.

## CodeQL register (F-58-08)

Alerts 93-100 are all `rust/cleartext-logging`: `src/main.rs:59` (`info!("{}", settings_summary(&config))`, counts only) and seven sites in `src/infrastructure/web/run_api_wiring.rs` (a fixed warning sentence naming a config key and env var, its test assertions, and a `panic!` over a loop variable that is `None` or empty). None logs a credential. Each got a `[[dismissal]]` entry (owner `DF3NDR`, `false positive`, review date 2027-06-30 with the Phase 52 batch, compensating controls DF-07, the T-57-19 test, the `settings_summary` sentinel tests, advisory-only status and the manual credential-handling review).

**Platform state is not verified.** The code-scanning endpoints return 403 from this sandbox, so the dismissal of 93-100 on GitHub cannot be read. The maintainer's single reply `ticked` is taken to cover both checkpoint steps (dismiss, tick); this is the maintainer's word, not an agent read. It should be reconciled with the register's manual command.

## The tick: maintainer's reply and the read-only verification (Task 3)

Reply, verbatim, 2026-11-06T15:13Z (UTC):

```
ticked
```

| Check | Result |
|-------|--------|
| `SHA-A2` (`origin/claude/laughing-dirac-e0h2ax`) | `dffa996fa84c7b35c69f95987231426296f864b2` |
| parents | exactly one, `e56d4d4ae10b5b0793d020b4c362306f20dd8003` (SHA-A1) |
| `git diff --name-only SHA-A1 SHA-A2` | only `58-ACCEPTANCE-AUDIT.md` |
| diff size and token | 1 insertion, 1 deletion (line 43); word-diff shows only `[ ]` to `[x]` |
| PR 64 | open, base `main`, head `dffa996fa` (equals SHA-A2) |
| `main` (D-02) | `8b23cb4c5`, unmoved; no `v0.11.1*` tag on origin |

Nothing was pushed, merged, tagged, labelled or dispatched by this plan after SHA-A1.

## Observations

- **24 pre-existing `Cargo.lock:1` alerts.** The maintainer's alert listing showed 24 older open alerts at `Cargo.lock:1` (alert numbers 2 through 92), not among the eight new alerts on this PR. Recorded in `58-CI-EVIDENCE.md` so the count is on file; not dispositioned and not acted on (outside F-58-08).
- **Leftover wording on the ticked line.** Line 43 of `58-ACCEPTANCE-AUDIT.md` still contains "Unticked, as designed" after the box, because the maintainer changed the one character the hand-off prescribed. Left as written.

## Deviations from Plan

### Auto-fixed Issues

None.

### Recorded deviations

**1. [Precondition reading] PR head at the start of Task 1 was `d17716713`, not `SHA-A0`.** Three planning-only `docs(58-09)` commits had moved the head; `git diff --name-only 801dd237e d17716713 -- . ':!.planning'` printed nothing. The orchestrator pre-approved reading the precondition that way. Recorded in `58-CI-EVIDENCE.md`.

**2. [Verifier quirk] The plan's Task 3 automated check cannot pass as written.** It counts changed lines with `grep -c '^[-+][^-+]'` and expects 2, but the removed box line begins `-- [ ]`, so the pattern matches 0. Every other clause of that check passes (single parent, file list, `[x]` line, remote tip equals PR head). The substance was verified directly (stat 1/1, word-diff `[ ]` to `[x]`).

**3. [Orchestrator instruction over plan text] The records and this SUMMARY are committed locally.** The plan said to leave them uncommitted for `chore/58-close` (plan 58-13). The continuation instructions from the orchestrator said to commit them (`75f36c72d`, plus the metadata commit that carries this file). They sit after SHA-A2, are not pushed, and the tick remains the last commit that reached origin. The orchestrator decides when the post-tick docs commits go out; pushing them would start new CI on a new head, so plan 58-12 should read the runs on SHA-A2 first.

**4. [Local HEAD differs from SHA-A2] HEAD is no longer SHA-A2** because of deviation 3; the plan's `test "$(git rev-parse HEAD)" = "$A2"` clause was true at the moment of the fast-forward and false after the records commit.

## Auth gates

Task 2 was a `checkpoint:human-action` (`blocking-human`): CodeQL dismissals and the sign-off tick are the maintainer's writes (D-00a, D-00b). Documented as normal flow.

## Known Stubs

None.

## Threat Flags

None. No file added a network endpoint, auth path or trust-boundary schema change.

## Threat model dispositions

| Threat | Outcome |
|--------|---------|
| T-58-110 (spoofed or misplaced tick) | mitigated: parent, file list, single token and PR head asserted; the agent never edited the box |
| T-58-111 (agent commit after the tick) | mitigated on origin: nothing pushed after SHA-A2; local records commits are unpushed |
| T-58-112 (changelog dates disagree with tag date) | mitigated: redate commit before the tick, row 7 re-read |

## Next

The phase is stopped (D-00i). Resume condition: every run on SHA-A2 has `status: completed`; next action plan 58-12 Task 1. The maintainer alone merges PR 64, pushes the `v0.11.1` tag and applies the `release` label.

## Self-Check: PASSED

- Commits `1f6e639f2`, `045e1aef0`, `e56d4d4ae`, `dffa996fa`, `75f36c72d` exist in `git log`.
- `58-CI-EVIDENCE.md` and `.continue-here.md` (phase directory) and `CODEQL-DISMISSALS.md` (repo root) exist; `- SHA-A1:` and `- SHA-A2:` are recorded.
