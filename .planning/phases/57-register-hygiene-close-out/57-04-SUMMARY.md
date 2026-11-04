---
phase: 57-register-hygiene-close-out
plan: 04
subsystem: docs
tags: [adr-0063, thread-ownership, tenant-isolation, migration, changelog, operator-notes]

requires:
  - phase: 57-register-hygiene-close-out
    provides: "owner_run_for_thread and the thread-owner wiring (57-02, 57-03), M-B-40 and the storage changelog entries (57-06)"
provides:
  - "ADR-0063 amended in place with the earliest-attributed-run owner rule, quoting every replaced sentence"
  - "user guide, platform API reference and HTTP service host page stating the shipped rule"
  - "MIGRATION 11.1 M-B-24/M-B-25, 11.6 and a new 11.8 operator step 12 for legacy mixed-tenant threads"
  - "root and paladin-web [Unreleased] changelog entries for the rule and the row 85 fix"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "ADR amendment: one dated bold paragraph under the decision, every replaced sentence quoted, decision text rewritten above it"

key-files:
  created: []
  modified:
    - .planning/decisions/0063-thread-and-job-tenant-visibility.md
    - docs/src/user-guides/parley-and-chronicle.md
    - docs/src/api-reference/platform-api.md
    - docs/src/deployment-topologies/http-service-host.md
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md

key-decisions:
  - "PD-04-1 followed: M-B-24 and M-B-25 are amended in place (the latest-run rule never shipped in a release), with no reversal row; the one visible upgrade effect is an 11.8 operator step"
  - "The root CHANGELOG [Unreleased] carried no thread-tenancy bullet to amend, so it gets one new Fixed bullet that states the rule and row 85 together"

patterns-established:
  - "A decision text is rewritten to the new rule while the amendment paragraph keeps the old wording quoted"

requirements-completed: []  # REG-02 is ticked by 57-11, not here

duration: ~35min
completed: 2026-11-04
status: complete
---

# Phase 57 Plan 04: Owner-rule contract, guides and upgrade notes Summary

**ADR-0063, three guide pages, MIGRATION 11.1/11.6/11.8 and both `[Unreleased]` changelogs now state that a thread belongs to the tenant of its earliest attributed run, served by `owner_run_for_thread`, with the replaced latest-run sentences quoted in a dated amendment.**

Plan-start SHA: `74c38f2f8d83e460984dfdfad633a6d6f32ea839`

## Accomplishments

- ADR-0063 Decision 1 is rewritten to the owner rule (earliest run carrying `submitted_by`, by `submitted_at` then `run_id`; the earliest run when none is attributed, so a schedule-only thread stays Admin-only; `None` only for a thread with no run, which keeps thread birth open). The dated paragraph `**Amended 2026-11-04 (Phase 57, D-02; WINDOWS rows 85-87):**` quotes each replaced sentence (Decisions 1, 4 and 6, the T-53-05 and T-53-10 residuals, the timing residual, the renamed test), says an Admin or unattributed run added later never moves ownership, says legacy mixed-tenant threads go to their earliest attributed tenant, and records Phase 53 SPEC AC-04 as superseded. `53-SPEC.md` itself is not edited.
- Decision 4 names the owner lookup and the skip of a newer-schema owner row. Decision 6 adds `thread_owner_error_for_scope`. T-53-10 is narrowed to the run routes, T-53-05 names the owner run, and Evidence names the 57-02 contract clauses and the 57-03 tests, including the renamed `mixed_tenant_thread_belongs_to_the_earliest_attributed_runs_tenant`. `## Status` stays Accepted with the line `Amended 2026-11-04 (Phase 57).`
- `parley-and-chronicle.md`, `platform-api.md` (both thread passages) and `http-service-host.md` state the rule in operator and client terms, including the newer-schema skip. The `409 thread_busy` sentence (platform-api.md:82, about the active run) is untouched.
- MIGRATION `M-B-24` and `M-B-25` are amended in place; `M-B-25` also states the row 85 behaviour. The two 11.6 passages say the same. 11.8 step 12 tells operators how to find a thread holding runs from two tenants (`GET /v1/runs?thread_id=` under an Admin key) and that after upgrade only the earliest attributed run's tenant and Admin keys see it, with no re-assignment.
- `[Unreleased]` in `crates/paladin-web/CHANGELOG.md`: both ownership bullets amended and a `### Fixed` bullet for row 85. Root `CHANGELOG.md`: one new `### Fixed` bullet (rule, `owner_run_for_thread`, row 85, 11.8 step 12).

## Task Commits

1. **Task 1 (tracer): ADR-0063, parley-and-chronicle.md, MIGRATION M-B-24/M-B-25** - `b57100e63` (docs)
2. **Task 2: platform-api.md, http-service-host.md, MIGRATION 11.6 and 11.8, both changelogs, sweep** - `bfee47716` (docs)

## Tracer gate

tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved. The Task 1 `<verify>` command was re-run on the committed state (`b57100e63`) and printed a pass before Task 2 began.

## Verification

- Task 1 `<verify>` chain: pass. Task 2 `<verify>` greps (all five `earliest attributed run` locations and the part-11 latest-run count of 0): pass.
- `make check-version-strings`: exit 0 (128 files scanned, every copyable version string equals `0.11.0`).
- `pre-commit run trailing-whitespace --files <changed files>`: Passed.
- D-00c freeze check: `git diff -U0` hunks in MIGRATION.md are at lines 14, 15, 77, 93 and 141 only (sections 11.1, 11.6 and 11.8; part 11 ends before line 148). Parts 9 and 10 and section 11.2 are byte-unchanged. In both changelogs every hunk lies above the `## [0.11.0]` heading, and no `[0.11.0]` line changed.
- No code was compiled or run; the plan is docs-only and no cargo command was needed.

## Sweep

`git grep -n -i -E 'latest run|latest-run|latest_run'` over `docs/src`, `MIGRATION.md`, the root and crate changelogs, ADR-0063 and `crates/*/src` + `src`. Every remaining hit and why it stays:

| Hit | Reason kept |
|-----|-------------|
| `.planning/decisions/0063-...md:27` | The amendment paragraph, which quotes the replaced sentences (P-04-2) |
| `.planning/decisions/0063-...md:61` | The test name `permits_thread_admits_all_and_only_the_latest_runs_tenant`, kept by 57-02 (PD-02-4); the line says so |
| `docs/src/api-reference/platform-api.md:82` | `409 thread_busy` concerns the active run, not ownership |
| `crates/paladin-core/CHANGELOG.md:17` | 57-02's entry, which says the latest-run wording was replaced and never shipped |
| `src/application/services/run/inspector.rs:149,256` | `latest_run` serves the thread inspector's "most recent run" display, not ownership |
| `src/application/services/run/http_surface_tests.rs:1054,1057` | Test comments on a fork of an unattributed run; see Handed back |

The `grep -ciE 'belongs to the tenant of its latest run|whose latest run (is attributed|belongs)'` count over MIGRATION part 11 is 0. Parts 9 and 10 were not searched for edits by design.

## Handed back

- `.planning/PROJECT.md` lines 916, 924 and 2171 still state the latest-run rule (and call the ownership transfer an open residual). It is orchestrator-owned; update it with the phase close-out.
- `.planning/STATE.md` decision lines 751, 755, 760 and 862-863 are append-only history of what was decided in Phase 53 and the row 85-87 description; left as written.
- `.planning/WINDOWS.md` rows 86, 87 and the row 58 closure text describe the old rule; 57-11 closes rows 85-87 and should cite `b57100e63` and `bfee47716` as the contract and migration record.
- `src/application/services/run/http_surface_tests.rs:1054` and `:1057` (57-03's file) are test comments that say "latest-run lookup" and "latest run is unattributed" for the fork-of-an-unattributed-run case. They are stale wording only (the test is correct under the owner rule, where an all-unattributed thread's owner is its earliest run). Not edited because this plan is docs-only; fix with the next code-touching change.

## Deviations from Plan

### Auto-fixed Issues

None. One wording note, not a rule deviation: the plan expected a latest-run bullet in the root `CHANGELOG.md` `[Unreleased]`, but that section never carried one (the Phase 53 thread entries live only in the `paladin-web` changelog). The root file therefore gets a new `### Fixed` bullet rather than an in-place amendment, which still satisfies the Task 2 verify.

## Known Stubs

None.

## Threat Flags

None. The edits are documentation only and add no network, auth, file or schema surface.

## Self-Check: PASSED

- Files modified exist: ADR-0063, the three guide pages, MIGRATION.md, CHANGELOG.md and `crates/paladin-web/CHANGELOG.md`.
- Commits `b57100e63` and `bfee47716` exist on `claude/laughing-dirac-e0h2ax` and carry the required two trailer lines.
