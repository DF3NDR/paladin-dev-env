---
phase: 56-dogfood-findings
plan: 04
subsystem: dogfood-findings
tags: [findings-log, prove-then-lock, dogf-01, df-01, df-02, df-03, df-04]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-01 scripts/check-findings-log.sh (last-Disposition-bullet grammar) and 56-02 DF-05/DF-06 entries"
provides:
  - "56-DOGFOOD-EVIDENCE.md: prove-then-lock table (14 rows) for DF-01..DF-04, every proof re-run on HEAD 6212e3ecf"
  - "four appended 'in v0.11.1' dispositions in .project/v0.11.1/02-dogfood-findings-log.md; check-findings-log.sh exits 0"
affects: [56-05..56-10 (findings log is now fully dispositioned for the pre-seeded entries), SHIP-10]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-DOGFOOD-EVIDENCE.md
  modified:
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "Closing bullets cite the evidence file and the short HEAD SHA the proofs ran on (6212e3ecf), per PD-04-3, so each disposition is traceable to a run rather than to a prior phase's claim"
  - "DF-01 is closed under the boot-refusal remedy (FIRST-01), the second of the two remedies the entry offered; yaml_env_placeholder_is_not_expanded is cited because it pins that no expansion happens"

requirements-completed: [DOGF-01]

duration: ~15min
completed: 2026-11-02
---

# Phase 56 Plan 04: Close the four pre-seeded findings Summary

**DF-01..DF-04 re-proved on HEAD 6212e3ecf (12 named tests plus 3 gates and 4 file/grep checks, all green) and closed with dated `in v0.11.1` dispositions naming FIRST-01, FIRST-02/03, FIXES-01 and TENANT-03/04/05/07; `./scripts/check-findings-log.sh` now exits 0.**

## Performance

- **Tasks:** 2 of 2 (1 tracer, 1 auto)
- **Files:** 1 created, 1 modified (log: +4 lines, 0 deletions)
- **Commits:** `9e0563279` (Task 1, DF-03), `68cd1b47b` (Task 2, DF-01/02/04)

## Proofs re-run (HEAD 6212e3ecf, 2026-11-02 UTC)

| DF | command | exit | result line |
|----|---------|------|-------------|
| DF-03 | `make check-no-stub-panics` | 0 | `OK: scanned 624 file(s) (19 test-only), 0 finding(s)` |
| DF-03 | `bash tests/scripts/check-no-stub-panics_test.sh` | 0 | `88 assertion(s) passed.` |
| DF-03 | `test ! -e crates/paladin-content/src/services/content_filtering_service.rs` | 0 | absent |
| DF-02 | `make check-version-strings` | 0 | `OK: 128 file(s) scanned, every copyable version string equals '0.11.0'` |
| DF-02 | `./tests/scripts/check-version-strings_test.sh` | 0 | `37 assertion(s) passed.` |
| DF-02 | `grep -n 'Current version' README.md` | 0 | `185:Current version: **0.11.0**.` |
| DF-01/DF-04 | `cargo test -p paladin-ai --lib --features web-server -- <9 names>` | 0 | `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1486 filtered out` |
| DF-01 | `cargo test -p paladin-ai --bin paladin-server --features web-server -- build_auth_config_refuses_an_unexpanded_placeholder_key` | 0 | `test result: ok. 1 passed; 0 failed; ... 20 filtered out` |
| DF-01 | example-key grep over `config.example.yml` | 1 (expected) | no output |
| DF-01 | `test ! -e k8s/server/configmap.yaml` | 0 | absent |
| DF-04 | `cargo test -p paladin-web --lib -- <2 names>` | 0 | `test result: ok. 2 passed; 0 failed; ... 327 filtered out` |
| DF-04 | `cargo test -p paladin-ai-core --lib -- permits_thread_admits_all_and_only_the_latest_runs_tenant` | 0 | `test result: ok. 1 passed; 0 failed; ... 750 filtered out` |

All twelve named tests appeared as `<name> ... ok` in the captured output (PD-04-2), so no pass rests on a
filter that matched nothing.

## Checker output

After Task 1 (exit 1, as planned: DF-03 gone, the other three still named):

```
.project/v0.11.1/02-dogfood-findings-log.md:61: DF-01: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:73: DF-02: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:94: DF-04: OPEN (the last Disposition bullet reads 'open')
parsed 6 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

After Task 2 (exit 0):

```
OK: every entry's last Disposition is a final form (parsed 6 entries from .project/v0.11.1/02-dogfood-findings-log.md)
```

No id is still named: DF-05 and DF-06 (appended by 56-02) were already final (`in v0.11.1` and `out (re-homed: ...)`).

## Deviations from Plan

None - plan executed exactly as written. No proof failed, so the D-00e failure path (new DF entry, withheld
closing bullet) was not triggered and no entry was added to the log. The prescribed disk control never fired:
free space on `/` went from 2993M to 2402M across the four cargo runs (cached deps, 0.3s to 17s builds), no
ENOSPC, no cleanup, no retry.

Notes on plan mechanics (not deviations):

- The Task 2 `<automated>` verify asserts the log `--numstat` deletion column is `0` against the index; after the
  Task 1 commit that diff is the working tree only. The plan-start comparison (`git diff --numstat 6212e3ecf`)
  was also run and reads `4 0`, matching the acceptance criterion of exactly 4 insertions and 0 deletions.
- The Task 1 verify greps `DF-01: OPEN`; the checker prints it as `DF-01: OPEN (...)`, which matches.

## Known Stubs

None.

## Threat Flags

None. No network endpoint, auth path or schema was touched; the plan edits one planning document and one log.

## Self-Check: PASSED

- FOUND: .planning/phases/56-dogfood-findings/56-DOGFOOD-EVIDENCE.md
- FOUND: .project/v0.11.1/02-dogfood-findings-log.md (4 `Disposition (2026-11-02): in v0.11.1 — fixed by` bullets, 0 deletions)
- FOUND commit 9e0563279 (Task 1) and 68cd1b47b (Task 2)
- `./scripts/check-findings-log.sh` exit 0
