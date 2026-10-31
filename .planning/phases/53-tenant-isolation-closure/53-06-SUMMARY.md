---
phase: 53-tenant-isolation-closure
plan: 06
subsystem: tenant-isolation
tags: [rust, tenant-isolation, boot-guards, treasurer, schedules, config-validation, tdd]

requires:
  - phase: 53-tenant-isolation-closure
    provides: 53-01 principal.rs module conventions (permits_thread, sentinel docs), 53-05 MIGRATION sections 11.5/11.6/11.7 scaffolding
provides:
  - "RunWorkerPool::has_engine_factory and the assert_treasurer_enforced boot refusal (TENANT-08)"
  - "Advisory, read-only NULL created_by schedule scan: one warn, one error at most, boot never blocked (TENANT-10)"
  - "RESERVED_PRINCIPAL_LITERALS / RESERVED_KEY_NAME_LITERALS in paladin-ai-core and their rejection in AuthConfig::validate (TENANT-09)"
affects: [53-07, 53-08]

tech-stack:
  added: []
  patterns:
    - "Scan split into a pure scan, a pure formatter and a thin emitter so 'exactly one warn' is proven on the formatter output without a second process-wide logger"
    - "Recording repository wrapper that delegates list and errors on every other method proves a read-only scan"
    - "Boot assertion as a pure function over two booleans, called after the last builder and before spawn"

key-files:
  created: []
  modified:
    - src/application/services/run/worker.rs
    - src/infrastructure/web/run_api_wiring.rs
    - src/config/agents.rs
    - crates/paladin-core/src/platform/container/principal.rs
    - .project/current-exports.txt
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - MIGRATION.md

key-decisions:
  - "Refusal fires exactly on 'Treasurer configured AND no engine factory' (D-00f, PD-06-1); the 'factory present but with_treasurer dropped' case is an accepted residual (T-53-32), covered by review"
  - "The schedule scan runs whenever the schedule repository exists, whatever schedules.enabled says (PD-06-2); the run-store-disabled arm returns first and logs nothing"
  - "scan_unattributed_schedules delegates to a private scan_unattributed_schedules_paged(repo, page_size) so tests can force several pages over a few rows; production always uses SCHEDULE_SCAN_PAGE = 100"
  - "The key-name-only reserved literal is 'anonymous' (paladin_web OPEN_ACCESS_PRINCIPAL_ID), not 'open-access' (PD-06-4); drift test lives in the root crate because core cannot import paladin-web"

patterns-established:
  - "Reserved-literal check is exact == membership with no trimming or case folding; a padded literal is still rejected by the pre-existing TenantId::new whitespace rule"

requirements-completed: [TENANT-08, TENANT-09, TENANT-10]

coverage:
  - id: D1
    description: "assert_treasurer_enforced refuses exactly (Treasurer configured, no engine factory) with the D-12 message and nothing else"
    requirement: "TENANT-08"
    verification:
      - kind: unit
        ref: "run_api_wiring::tests::treasurer_assertion_refuses_a_configured_treasurer_without_an_engine_factory"
        status: pass
    human_judgment: false
  - id: D2
    description: "A real RunWorkerPool with a Treasurer and no engine factory is refused; the same pool with a factory passes"
    requirement: "TENANT-08"
    verification:
      - kind: unit
        ref: "run_api_wiring::tests::pool_built_without_an_engine_factory_is_refused_with_a_treasurer"
        status: pass
    human_judgment: false
  - id: D3
    description: "Shipped build_run_api wiring with treasurer.allowance still boots with a Treasurer; Treasurer-less deployment unaffected"
    requirement: "TENANT-08"
    verification:
      - kind: integration
        ref: "run_api_wiring::tests::shipped_wiring_with_a_treasurer_boots"
        status: pass
      - kind: integration
        ref: "run_api_wiring::tests::build_run_api_wires_a_treasurer_only_when_allowances_are_configured"
        status: pass
    human_judgment: false
  - id: D4
    description: "No configuration switch or no-op factory escape hatch lets a configured Treasurer boot unenforced (AC-N5)"
    requirement: "TENANT-08"
    verification:
      - kind: other
        ref: "grep -n engine_factory src/config/*.rs finds nothing; has_engine_factory reads the real field"
        status: pass
    human_judgment: true
  - id: D5
    description: "Schedule scan reports nothing / exactly one warn (count, ids ascending, POST /v1/schedules) and survives an unreadable page with one fixed error line"
    requirement: "TENANT-10"
    verification:
      - kind: unit
        ref: "run_api_wiring::tests::schedule_scan_reports_nothing_when_every_schedule_is_attributed"
        status: pass
      - kind: unit
        ref: "run_api_wiring::tests::schedule_scan_reports_one_unattributed_schedule"
        status: pass
      - kind: unit
        ref: "run_api_wiring::tests::schedule_scan_reports_every_unattributed_schedule_ascending_in_one_line"
        status: pass
      - kind: unit
        ref: "run_api_wiring::tests::schedule_scan_page_error_is_one_error_line_and_boot_continues"
        status: pass
    human_judgment: false
  - id: D6
    description: "The scan never writes: only list is called, and the SQLite run_schedules table is identical before and after build_run_api over an attributed, an unattributed and a half-attributed row"
    requirement: "TENANT-10"
    verification:
      - kind: unit
        ref: "run_api_wiring::tests::schedule_scan_never_writes"
        status: pass
      - kind: integration
        ref: "run_api_wiring::tests::schedule_scan_boot_over_sqlite_is_ok_and_leaves_the_table_untouched"
        status: pass
    human_judgment: false
  - id: D7
    description: "AuthConfig::validate rejects reserved literals in every position, names the first offender, accepts case variants, never prints a key"
    requirement: "TENANT-09"
    verification:
      - kind: unit
        ref: "config::agents::tests::validate_rejects_reserved_literals_in_every_position"
        status: pass
      - kind: unit
        ref: "config::agents::tests::validate_reports_the_first_reserved_offender"
        status: pass
      - kind: unit
        ref: "config::agents::tests::validate_accepts_case_variants_of_reserved_literals"
        status: pass
    human_judgment: false
  - id: D8
    description: "The core constants are the one source and 'anonymous' tracks paladin-web's OPEN_ACCESS_PRINCIPAL_ID"
    requirement: "TENANT-09"
    verification:
      - kind: unit
        ref: "config::agents::tests::reserved_key_name_matches_the_open_access_principal_id"
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-10-31
status: complete
---

# Phase 53 Plan 06: Boot guards for Treasurer wiring, unattributed schedules and reserved literals Summary

**Three boot-time guards in the root crate: a configured Treasurer refuses to boot without the engine-factory path, boot logs one read-only `warn!` naming every schedule with no recorded creator, and `open-access` / `unattributed` / `anonymous` are refused as key or tenant names at config load.**

## Performance

- **Duration:** about 14 min wall clock
- **Started:** 2026-10-31T13:06:53Z
- **Completed:** 2026-10-31T13:21Z
- **Tasks:** 3 (Task 1 tracer, Tasks 2 and 3 auto, all TDD)
- **Files modified:** 8
- **Plan start SHA:** `b18fd657a67f47069b8d16bb8934f81be9846250`

## Accomplishments

- **TENANT-08.** `RunWorkerPool::has_engine_factory(&self) -> bool` (additive, with a doctest) and a private pure `assert_treasurer_enforced(treasurer_configured, has_engine_factory) -> Result<(), String>` returning the exact D-12 message `treasurer configured but the run worker pool has no engine factory; settlement, spend guard and cancel probe would not attach`. `build_run_api_with_cadence` calls it after the `with_treasurer` block and before `Arc::new(pool)` / `spawn`, so refusal is before any worker; `paladin-server` builds the run API before `TcpListener::bind`. No config key, flag or default factory was added.
- **TENANT-10.** `scan_unattributed_schedules` pages `list(100, cursor)` to exhaustion and collects `created_by == None` ids; `scan_report` is a pure formatter (at most one `Warn` with count, ascending ids and `re-create each via POST /v1/schedules`, at most one `Error` with a fixed sentence and the cursor position or `start`); `report_unattributed_schedules` logs each line once. It is called right after the repository quartet is built, so it runs whenever the run store is enabled and is unreachable on the `Disabled` arm. It never returns an error and never writes.
- **TENANT-09.** `RESERVED_PRINCIPAL_LITERALS = [TenantId::OPEN_ACCESS, LedgerScope::UNATTRIBUTED]` and `RESERVED_KEY_NAME_LITERALS = ["anonymous"]` in `paladin-ai-core` (with doctests); `AuthConfig::validate` rejects them as key `name` (`http.auth.api_keys[#<index>]`), key `tenant` (`http.auth.api_keys[<name>]`) and enabled bearer `tenant` (`http.auth.bearer_token`), exact and case-sensitive, never interpolating a key value. Every pre-existing message, check and order is unchanged.
- MIGRATION §11.1 `M-B-27` (boot refusal) and `M-B-28` (boot warning), §11.5 (reserved literals), §11.8 (operator step `### Operators: v0.11.0 -> v0.11.1`); §11.2 untouched. Root and `paladin-core` CHANGELOGs gained `[Unreleased]` entries; `.project/current-exports.txt` refreshed.

## Task Commits

1. **Task 1 (tracer): Treasurer-wiring assertion from the pool inspector through build_run_api** - `6b73e11b0` (feat)
2. **Task 2: advisory NULL created_by schedule scan** - `99f74eea1` (feat)
3. **Task 3: reserved principal literals refused at config load** - `4de0bcbc2` (feat)

Tracer feedback gate (auto mode active): the tracer's `<verify>` commands (the four `run_api_wiring` tests and `make api-surface`) were green after the code landed and before Task 2 began, so expansion proceeded.

## Semver probe

```
semver: paladin-ai-core: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
semver: paladin-ai: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

(Both run with `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package <name>`.) `make api-surface` exits 0; the baseline diff adds `has_engine_factory` (two lines, as every `RunWorkerPool` method appears twice in that file's format, plus the header timestamp and item count).

## Verification

- `cargo test --lib --features web-server run_api_wiring`: 37 passed, 2 failed. The two failures are the known pre-existing network-dependent `build_run_api_persists_run_traces_when_trace_persist_is_set` and `build_run_api_persists_no_run_traces_by_default` (TESTS-01, Phase 55); not touched, not counted.
- `cargo test --lib config::agents`: 29 passed. `cargo test --lib --features web-server config`: 288 passed.
- `cargo test -p paladin-ai-core`: 743 lib, 11 and 153 doc tests passed (38 ignored as before).
- `cargo fmt --check`, `cargo clippy -p paladin-ai-core --all-targets -- -D warnings`, `cargo clippy --lib --tests --features web-server -- -D warnings`, `make check-migration-allowlist`, `make check-changelogs`, `make api-surface`: all exit 0.
- Acceptance greps: the `assert_treasurer_enforced(treasurer.is_some()` call precedes `.spawn(RunWorkerOptions::from`; `build_run_api_with_cadence(` precedes `TcpListener::bind` in `paladin-server.rs`; `grep engine_factory src/config/*.rs` finds nothing; no new message in `agents.rs` interpolates `entry.key`.
- Manual credential-handling review of every new message: the refusal text is static; the scan lines carry ids and a cursor only (no row text, no error text, asserted by negative checks in the page-error test); validation messages carry the entry name or index, and the reserved literal itself, never a key value.

## Decisions Made

See `key-decisions`. All four were pre-made in the plan (PD-06-1 to PD-06-4) and followed as written.

## Deviations from Plan

None - plan executed exactly as written.

Notes that are not deviations:
- Task 2's tests were written before the implementation but I did not run a separate compile-red step for them (the scan code went in before the first `cargo test`). Their ability to fail was shown instead by a local, uncommitted mutation (`.filter(.. || true)` on the unattributed check): four of the six scan tests failed, then the file was restored from a backup copy. Tasks 1 and 3 were watched red as compile errors (`cannot find function assert_treasurer_enforced`, unresolved `RESERVED_*` imports).
- `schedule_scan_boot_over_sqlite_is_ok_and_leaves_the_table_untouched` is a lock rather than a red-first test: it would also pass if the scan were removed. The ordering test additionally feeds `scan_report` a deliberately shuffled `ScanOutcome`, since the stores already hand ids over ascending.
- The plan's "exactly one line added to `current-exports.txt`" holds per the file's own format: the item is listed twice (as for every sibling `RunWorkerPool` method), plus the regenerated header and count.
- The plan names the §11.8 heading with `->`; the older v0.10.1 heading uses an arrow glyph. I used the plan's ASCII form, which its acceptance grep expects.

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path, file access or schema change. T-53-30, T-53-31 (grep part), T-53-33, T-53-34, T-53-35 and T-53-36 are mitigated and evidenced above; T-53-32 (factory present but `with_treasurer` dropped) is accepted per D-00f and recorded for ADR-0063. AC-N5 (a no-op factory) stays a judgment check for code review and `/gsd-secure-phase`, because `has_engine_factory` cannot tell what a factory builds.

## Next Phase Readiness

Plan 53-07 can cite `M-B-27` and `M-B-28`, the T-53-32 residual and the AC-N5 judgment item in ADR-0063 and the review dispositions, and can mark WINDOWS row 63's operator signal as fixed by the boot warning. Plan 53-08's semver sweep has two more passing probe lines to build on.

## Self-Check: PASSED

- FOUND: src/application/services/run/worker.rs (`pub fn has_engine_factory(&self) -> bool`)
- FOUND: src/infrastructure/web/run_api_wiring.rs (`fn assert_treasurer_enforced(`, `scan_unattributed_schedules(`, `fn scan_report(`)
- FOUND: src/config/agents.rs (`RESERVED_PRINCIPAL_LITERALS`, `RESERVED_KEY_NAME_LITERALS`)
- FOUND: crates/paladin-core/src/platform/container/principal.rs (`pub const RESERVED_KEY_NAME_LITERALS`)
- FOUND: commits `6b73e11b0`, `99f74eea1`, `4de0bcbc2`
