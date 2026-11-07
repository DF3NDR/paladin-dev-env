---
phase: 58-crate-release
plan: 06
subsystem: release-gates
tags: [release, register, todos, semver, ship-10, ship-09, ship-08, pre-bump]
status: complete

requires:
  - phase: 58-crate-release
    provides: plan 58-05 register and the maintainer's answers; plan 58-15 credential-class fixes (D-07 amended)
provides:
  - "41 new dated pending todos (recheck_by 2026-11-09), one per re-homed register row without a todo; V-13 cites R-88's"
  - "the four acknowledged todos dated; the three folded todos moved to todos/completed/ with resolution notes"
  - "58-AUDIT-DISPOSITIONS.md locked: 135 rows, fixed 51, waived 37, re-homed 47, strict verifier 0 failures"
  - "R-95: the four same-class Serialize types left by 58-15 (F-58-06) re-homed with a todo"
  - "D-11 semver green on SHA-S: 12 packages, 223 checks each, 0 failures at --release-type patch against 0.11.0"
  - "pre-bump offline sweep green on SHA-S; SHA-P pushed (plain) and its four queued runs recorded"
  - ".continue-here.md: the pre-bump resume file (condition: every run on SHA-P completed)"
affects: [58-07, 58-08, 58-10, 58-12, 58-14]

tech-stack:
  added: []
  patterns:
    - "One pending todo per re-homed row, owner and recheck_by checked against the row by the strict verifier"
    - "Semver gate read on the plan's own head with the zero-check tripwire before any bump"

key-files:
  created:
    - .planning/phases/58-crate-release/58-06-SUMMARY.md
    - .planning/phases/58-crate-release/.continue-here.md
    - .planning/todos/pending/ (41 files dated 2026-11-05; paths are the Todo column of the register)
  modified:
    - .planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md
    - .planning/phases/58-crate-release/58-CI-EVIDENCE.md
    - .planning/todos/pending/2026-10-20-openai-responses-api-for-pro-tier.md
    - .planning/todos/pending/2026-11-02-setup-check-first-run-output.md
    - .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md
    - .planning/todos/pending/2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md
    - .planning/todos/completed/ (3 folded todos moved in)

key-decisions:
  - "R-95 re-homed (owner v0.12.0 Phase 1, 2026-11-09) rather than fixed or waived: serialising output of published paladin-storage types changes, a behaviour change for a minor release (D-07), and no code change belongs in the pre-bump window (D-14, D-17)"
  - "Key rows R-83, R-84, R-85 were already final from 58-05 (fixed, fixed, waived) and are unchanged; the maintainer's rotation and PURGE=none answers are cited, not re-derived"
  - "SHA-P is the semver-and-sweep evidence commit 35e696cc9; the resume-file commit rides the next push"

requirements-completed: []

duration: "about 21 minutes (2026-11-05T17:11Z to 17:32Z)"
completed: 2026-11-05
---

# Phase 58 Plan 06: Register Lock, D-11 Semver and the Pre-bump Push Summary

**The disposition register is locked with 41 dated re-home todos, `check-semver.sh` reads 12 packages at 223 checks and 0 failures on the head the bump builds on, the offline sweep is green, and `SHA-P` is on origin awaiting its CI runs.**

## What was done

**Task 1 (tracer), commit `fb6d50da0`.** The maintainer's recorded answers (`DATE=2026-11-09`, `KEY=rotated:2026-11-05`, `PURGE=none`) were applied from `## Maintainer decisions (58-05)`.
- 41 new todos in `.planning/todos/pending/2026-11-05-<slug>.md`, house frontmatter plus `recheck_by: 2026-11-09`, `dispositioned_by: Phase 58 plan 58-06 (<R-nn>)`, body `## Problem`, `## What is already true`, `## What to do`, `## Source`. Owners are exactly the row owners (`repo maintainer` or `v0.12.0 Phase 1`). V-13 cites R-88's todo.
- Four acknowledged todos (R-62..R-65): `recheck_by: 2026-11-09` and an appended `## Update 2026-11-05` line; no other line changed.
- Three folded todos (R-59, R-60, R-61): `## Resolved 2026-11-05 (Phase 58)` naming `86213aee2`, `8cfd7ad33`/`2bcc91263`/`bf0c25ab2` and `364f08607`, moved by `gsd-tools query todo complete`. The handler put `completed:` above the opening `---` and inserted blank lines into the `files:` list, so each file was rebuilt from its original with `completed: 2026-11-05` inside the frontmatter (house form of the phase-44 todo).
- Register: every `pending 58-05 decision` and `to be written in 58-06` marker replaced; the header paragraph rewritten as locked; Sources row 21 and row R-95 added.
- Strict `58-check-register.sh`: `register: checked 135 row(s) (R: 95, V: 40); fixed 51, waived 37, re-homed 47; 0 failure(s)`. Every re-homed row's owner and date equal its todo's `owner:` and `recheck_by:` (checked separately). `windows status` `open_count` is 0 and `WINDOWS.md` is unchanged.

**Task 2, commit `35e696cc9`.** `SHA-S` is `fb6d50da0caa5980fca0f70799fe25adb4a830f0` (clean tree). `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` (cargo-semver-checks 0.50.0), exit 0:

```
semver: <each of the 12 packages>: 223 checks: 223 pass, 31 skip
semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

Sweep on `SHA-S`, all exit 0: `make check-gates` (128 version-string files, 627 stub-scan files, 184 doc-model files, 0 findings), `make test-shell-guards` (85 assertions), `cargo fmt --check`, `make security` (9 allowed warnings; advisories, bans, licenses, sources ok), `make api-surface` (4331 items, unchanged), `pre-commit run --all-files` (all hooks Passed), `check-review-dispositions.sh` (51 findings, 0 failures), strict register check (0 failures). `git diff --name-only SHA-S HEAD -- . ':!.planning'` is empty.

**Task 3, commit `3412357c7`.** `main` read as `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e` before and after the push. `SHA-P` = `35e696cc9af1f817da07db9ec233d31e4e553273` pushed plain (`82d82cc5e..35e696cc9`); `git ls-remote` equals it. Runs queued and read once (17:31Z): `ci.yml` 42075372810 (pending), `feature-flags.yml` 42075372714 (queued), `pre-commit` 42075372712 (queued), `codeql.yml` 42075372834 (queued). No open PR, no `v0.11.1*` tag. `.continue-here.md` written with the constraints (flake rule quoted), anti-patterns, `- SHA-P:` line and resume condition.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `todo complete` handler broke the house frontmatter form**
- **Found during:** Task 1
- **Issue:** the handler wrote `completed: <date>` above the opening `---` and padded the `files:` list with blank lines, failing the plan's `head -1 == ---` check.
- **Fix:** rebuilt the three completed files from their committed originals with the `completed:` line inside the frontmatter, plus the resolution notes.
- **Files modified:** the three `.planning/todos/completed/` files. **Commit:** `fb6d50da0`.

**2. [Rule 2 - Missing critical] R-95 row and todo for the four same-class residual types**
- **Found during:** Task 1 (58-15's F-58-06 hands them to this plan's sweep)
- **Issue:** `NodeCacheConfig`, `RedisQueueConfig`, `MinioConfig` and `RedisNodeCacheConfig` still derive `Serialize` with plaintext secrets; an unrowed security finding would be unrepresented in the register. Re-read on `82d82cc5e`; `RedisQueueConfig` and `MinioConfig` also derive an unredacted `Debug`.
- **Fix:** one `re-homed` row R-95 (owner `v0.12.0 Phase 1`, 2026-11-09) and its todo. No code changed (D-14, D-17). This is a disposition choice the maintainer may overrule (fix before the tag instead).
- **Files modified:** the register and one new todo. **Commit:** `fb6d50da0`.

The plan text assumes 49/51 re-homed rows; the committed register had 46 (47 with R-95), and the committed register was followed.

## Auth gates

None.

## Known Stubs

None.

## Threat Flags

None. The only remote write was the plain branch push; no network surface was added.

## Hand-off

- Plan 58-07 Task 1 reads the conclusions of the four runs on `SHA-P` once the resume condition holds (every run `status: completed`). Nothing is watching.
- `SHA-S` and the run date (2026-11-05) are for the MIGRATION section 11.8 paragraph (D-10, plan 58-07 commit 3).
- Maintainer attention: R-95 (above) and the follow-up that `MinioConfig` / `RedisQueueConfig` print secrets under `Debug`.

## Self-Check: PASSED

- Todos: 45 files in `.planning/todos/pending/` each with `owner:` and `recheck_by:`; three folded files in `completed/`, none in `pending/`.
- Commits `fb6d50da0`, `35e696cc9`, `3412357c7` exist; `git ls-remote` equals `SHA-P`.
- The plan's three `<automated>` verify blocks printed OK (Task 1 static part and `open_count` 0; Task 2; Task 3).
