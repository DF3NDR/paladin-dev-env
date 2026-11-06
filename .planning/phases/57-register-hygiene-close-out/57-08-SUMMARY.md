---
phase: 57-register-hygiene-close-out
plan: 08
subsystem: review-disposition-register
tags: [reg-05, register, checker, shell-guard, review-dispositions]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-CONTEXT D-10..D-13 (register shape and evidence rule) and the Phase 53 TENANT evidence the Phase 40 closures point at"
provides:
  - "scripts/check-review-dispositions.sh: a never-vacuous REG-05 gate that derives every WR/CR id (and Phase 40 IN-01, IN-04) from the archive and fails on 15 named classes"
  - "tests/scripts/check-review-dispositions_test.sh with 22 committed fixture directories (one firing fixture per class, plus ok, --pending-ok and --phases cases)"
  - "57-REVIEW-DISPOSITIONS.md with 20 rows (Phases 40, 42, 46, 48, 50.1; Phase 50 recorded as 0 warnings), all closed, none live"
  - "an appended '## Dispositions (Phase 57)' pointer section in the six archived REVIEW.md files (40, 42, 46, 48, 50, 50.1)"
affects: [57-09, 57-10, 57-11]

tech-stack:
  added: []
  patterns:
    - "bash wrapper plus embedded python3 (3.8-compatible) checker in the check-findings-log.sh style: fence rule, named failures, fixed last-line summary"
    - "fixture directories (archive/ + register.md + optional args) pointed at the checker through --archive and --register"

key-files:
  created:
    - scripts/check-review-dispositions.sh
    - tests/scripts/check-review-dispositions_test.sh
    - tests/scripts/fixtures/check-review-dispositions/ (22 case directories)
    - .planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md
  modified:
    - .planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-REVIEW.md
    - .planning/milestones/v0.11.0-phases/42-mid-run-halt-sse-terminal-status/42-REVIEW.md
    - .planning/milestones/v0.11.0-phases/46-docs-currency-hygiene/46-REVIEW.md
    - .planning/milestones/v0.11.0-phases/48-base-entity-rename/48-REVIEW.md
    - .planning/milestones/v0.11.0-phases/50-provider-parity-config-driven-model-swap/50-REVIEW.md
    - .planning/milestones/v0.11.0-phases/50.1-consumer-surface-readiness/50.1-REVIEW.md

key-decisions:
  - "The checker reads table rows only inside the register's '## Dispositions' section, so 'Live in tree' and header prose can never be parsed as rows"
  - "A requirement-id pointer for a closed row excludes WR-, CR- and IN- prefixes: those point back at a finding, not at evidence"
  - "Totals are counted across every well-formed register row, in or out of --phases scope, because the register is one document"
  - "A listed --phases value with no review file is ZERO_FILES (never a silent pass)"

patterns-established:
  - "Expected ids come from the archive, never the register; a register can only agree with the archive or fail"

requirements-completed: []

duration: ~35min
completed: 2026-11-04
---

# Phase 57 Plan 08: Review-disposition checker and the first register sweep Summary

**A never-vacuous `check-review-dispositions.sh` that derives 51 findings across 13 archived reviews, a regression test with a firing fixture per failure class, and 20 register rows (Phases 40, 42, 46, 48, 50.1) all closed on commits and tests verified in this phase.**

REG-05 is not ticked here: it spans 57-09, 57-10 and 57-11, and 57-11 ticks it (orchestrator instruction, same call as 57-05).

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-11-04
- **Tasks:** 3 (1 tracer, 1 TDD auto, 1 auto)
- **Commits:** 3 task commits

## Accomplishments

- The checker derives expected ids from the archive: 51 findings (47 WR + 2 CR + 40 IN-01/IN-04) across 13 files today. With no flags it fails (57 failures: 45 MISSING and 12 NO_POINTER_SECTION on the head after Task 1), so it is visibly not vacuous.
- `--phases 40,42,46,48,50,50.1 --pending-ok` exits 0 with `checked 20 finding(s) across 6 review file(s); 0 failure(s)`, and so does the strict run (no row is open).
- The test has 25 assertions and passes under `make test-shell-guards`; `shellcheck --severity=warning` and `make lint-shell` are clean.
- Six archived REVIEW.md files each gained one `## Dispositions (Phase 57)` section; `git diff --numstat` shows 0 deleted lines on all six (P-08-2).

## Task Commits

1. **Task 1 (tracer): checker, register opened, Phase 40 closed** - `bfaa8436a` (feat)
2. **Task 2 (TDD): fixtures and test for every failure class** - `e6a60af93` (test)
3. **Task 3: Phases 42, 46, 48, 50, 50.1 dispositions and pointers** - `83629e6ae` (docs)

## Per-finding evidence

Re-run date for all tests: 2026-11-04, on this head. Commits older than the shallow clone (before 2026-10-08) were verified with `gh api repos/DF3NDR/paladin-dev-env/commits/<sha>`; the rest with `git cat-file` and `git log -1`.

Phase 40 (TENANT rows read from `.planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md`):
- 40 WR-01 -> TENANT-04. `gh api` confirmed `6d7efb16` ("WR-01 reject submit/fork on another tenant's thread"); `git cat-file` confirmed `2b16763d0`, `66002e74b`. `cargo test --lib --features web-server -- --exact` ran `submit_on_another_tenants_thread_is_rejected_as_unknown_thread` and `fork_on_another_tenants_thread_is_rejected_as_unknown_thread`: ok.
- 40 WR-02 -> TENANT-05. `gh api` confirmed `fa054746`. Same run: `cancel_of_another_tenants_run_is_not_found_and_writes_nothing` ok.
- 40 WR-03 -> TENANT-06. `gh api` confirmed `05612748` and `9afaef2b`; `git cat-file` confirmed `30b532813` and `839a8119a`; `009_add_run_attribution_check.sql` exists with constraint `runs_attribution_all_or_none`. `cargo test -p paladin-web --lib` ran `corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal`: ok. The Postgres test `migration_009_rejects_a_half_attributed_row` is Docker-gated and was not run here.
- 40 WR-04 -> TENANT-06. `05612748` via `gh api`; `internal_repo_error` at `crates/paladin-web/src/run_controller.rs:694`; `get_run_repository_error_is_a_generic_500_without_backend_text` ok.
- 40 IN-01 -> TENANT-09. `4de0bcbc2` ("refuse reserved principal literals at config load") confirmed locally; `validate_rejects_reserved_literals_in_every_position` ok.
- 40 IN-04 -> TENANT-07. `cc0077527` and `010ee3158` confirmed locally; `job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass` ok.

Phase 42 (`42-REVIEW-FIX.md`):
- 42 WR-5. `gh api` confirmed `89182bad`; `later_record_of_run_exists` at `src/application/services/run/events.rs:802`; four replay tests in `stream_tests.rs` ok (`..._later_halted_on_spend`, `..._later_cancelled`, `replay_keeps_a_last_reasonless_halted_record_beside_a_cancelled_row`, `replay_skips_a_stale_halted_record_found_at_a_page_boundary`).
- 42 WR-6. `gh api` confirmed `7f1c4970`; `with_seq_origin` (`hooks.rs:464`), `max_seq` (`run_trace_port.rs:272`), `trace_seq_origin` (`worker.rs:1066`) present; `a_second_run_on_a_thread_keeps_its_trace_records`, `seq_origin_starts_the_sequence_after_the_origin`, `default_max_seq_pages_through_read` ok.

Phase 46 (`46-REVIEW-FIX.md`):
- 46 WR-01. `df85c999` confirmed locally; `maneuver_warned_aegis_policies` at `commander.rs:593`; the three new commander tests ok.
- 46 WR-02. `065d4542` confirmed locally; read `CHANGELOG.md:288` and `crates/paladin-core/CHANGELOG.md:154` (both now say the key is refused at load); `grep "still deserializes"` finds nothing in either file.
- 46 WR-03. `5186f128` confirmed locally; read the per-strategy rustdoc at `phalanx_service.rs:183-188` (doc-only fix, no test to run).

Phase 48 (`48-REVIEW-FIX.md`):
- 48 WR-01..WR-05. `cd6e95c0`, `6a48b71e`, `cdefd67c`, `ceeb148e`, `41837b56` confirmed locally; `declares` (551), `SKIPPED_DOT_DIRECTORIES` (239), `DOCKER_EXCLUDED_DIRS` (163), `opens_test_module` (495), `normalize_declaration` (349) and `blank_non_code` (393) present in `tests/entity_rename_guard.rs`; `cargo test --test entity_rename_guard`: 8 passed, including the named controls for WR-01, WR-02, WR-03 and WR-04.

Phase 50: `50-REVIEW.md` frontmatter `critical: 0`, `warning: 0`; the checker derives 0 ids from it. No rows.

Phase 50.1 (`50.1-REVIEW-FIX.md`):
- 50.1 WR-01, WR-02. `9d0eaa3d`, `ec3fdc92` confirmed locally; both are doc-only, so the reworded text was read at `kimi/adapter.rs:84,206` and `registry/seeded.rs:2348`.
- 50.1 WR-03. `5ebd315b` confirmed locally; `is_http_url` at `registry/mod.rs:169`, used in all nine adapters (`grep -c`); `cargo test -p paladin-llm --lib --all-features -- non_http_base_url`: 9 passed.
- 50.1 WR-04. `c5bdea75` confirmed locally; read `.github/workflows/ci.yml:1664` (`executed=$(...) || true`). CI-only, not run here.

Caveat carried from the fix reports: several of these are marked "fixed: requires human verification". The disposition is `closed` on the commit, the tree and the re-run tests; it is not an owner sign-off.

## Live in tree

None among Phases 40, 42, 46, 48, 50 and 50.1. No row is `open`, so none carries a `LIVE <file:line>` entry and no row needed a class (correctness, security or other).

Documented residual, not a live finding: 42 WR-6's PostgreSQL `max_seq` override was Docker-gated and not run by the fixer, and two runs dispatched concurrently on one thread can still collide (`42-REVIEW-FIX.md`). It is written under `## Live in tree` in the register so 57-10 can see it.

## Checker

Fixtures (all under `tests/scripts/fixtures/check-review-dispositions/`, one directory each):
- Firing, one per class: `missing`, `unknown`, `duplicate`, `bad_disposition`, `open`, `no_evidence` (TBD, none, "see above" in three rows), `closed_no_pointer`, `waived_no_owner`, `totals`, `totals_absent`, `no_pointer_section`, `zero_files`, `zero_rows`, `unreadable` (non-UTF-8 register), `unclosed_fence` (archive), `unclosed_fence_register`, `malformed_row`.
- Flag cases: `open_no_live` (`--pending-ok`, fires OPEN), `zero_files_listed_phase` (`--phases 10,99`, fires ZERO_FILES).
- Silent: `ok` (a heading inside a fence in the archive and a row inside a fence in the register must be ignored), `open_pending_ok` (`LIVE crates/x.rs:12 — correctness: ...` passes), `scoped` (`--phases 10` with a missing 40 row and a 40 file lacking the pointer section passes).
- Plus one assertion that the real archive derives `51 finding(s) across 13 review file(s)`, and one that no fixture is modified by a run. Every run also asserts the `checked ...` summary is the last line.

RED record. Task 1's checker was written with every class in view, so the first run of the new test passed 25 of 25; there was no genuine RED for the checker as built. To show the assertions bite, the checker was mutated one change at a time and the test re-run (checker restored byte-for-byte after each, `cmp` clean):

| Mutation | Result |
|----------|--------|
| UNCLOSED_FENCE detection disabled | 2 assertions failed (archive and register fence cases) |
| LIVE rule under `--pending-ok` disabled | 1 failed (`open_no_live`) |
| Placeholder detection reduced to empty string | 1 failed (`no_evidence`) |
| Fenced lines no longer ignored | 4 failed (`ok`, WR-99 absence, `open_pending_ok`, `scoped`) |
| Phase 40 extra ids (IN-01, IN-04) dropped | 4 failed (`missing`, `ok`, `open_pending_ok`, real-archive count) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] A fifteenth failure class, MALFORMED_ROW**
- **Found during:** Task 1 (checker design)
- **Issue:** A phase-numbered table row with other than six cells would otherwise be skipped silently and surface only as a misleading MISSING for its id.
- **Fix:** Added `MALFORMED_ROW`, with its own fixture (`malformed_row`). The 14 plan classes are all present and tested; the plan's verify loop still passes.
- **Files modified:** `scripts/check-review-dispositions.sh`, `tests/scripts/check-review-dispositions_test.sh`
- **Commit:** `bfaa8436a`, `e6a60af93`

**2. [Rule 2 - Missing critical functionality] WR-/CR-/IN- ids do not count as a closed-row pointer**
- **Found during:** Task 1 (PD-08-3 implementation)
- **Issue:** The plan's requirement-id pattern `[A-Z]+-[0-9]+` would accept `WR-01` as evidence for a finding named `WR-01`, which is a self-reference, not a pointer (P-08-1, P-08-3).
- **Fix:** The pointer test ignores `WR`, `CR` and `IN` prefixes; every other requirement id still counts. Also the commit-id pattern requires a digit, so a hex-lettered word cannot pass as a commit.
- **Commit:** `bfaa8436a`

**3. [Plan reading] Register rows are read only in the `## Dispositions` section**
- The plan did not say where rows live. Restricting parsing to that section keeps the header and `## Live in tree` prose from being read as rows.

### Tracer gate

Tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved.

The Task 1 `<verify>` command was re-run against the committed tracer (`bfaa8436a`) and passed before Tasks 2 and 3 began.

### Authentication gates

None.

## Issues Encountered

- Phase 40 and Phase 42 REVIEW-FIX commits predate the shallow clone, so they were checked with `gh api` rather than `git cat-file`. All six resolved.
- `make api-surface` was not run: no Rust source, public item or crate file changed in this plan (shell, fixtures and planning files only).

## Known Stubs

None. The register has no placeholder row; the checker rejects them by construction.

## Threat Flags

None. The plan adds an offline, check-only script and planning text; no network endpoint, credential path or schema.

## Next Phase Readiness

- 57-09 and 57-10 extend the same register for Phases 38, 41, 43, 44, 45, 47 and 51 (and any `open` row they mint uses `LIVE <file:line> — <class>: ...`).
- 57-11 must run `./scripts/check-review-dispositions.sh` with no flags (strict) and tick REG-05 in REQUIREMENTS.md.
- The checker stays outside `make check-gates` and CI, like `check-findings-log` (Phase 56 D-03).

## Self-Check: PASSED
