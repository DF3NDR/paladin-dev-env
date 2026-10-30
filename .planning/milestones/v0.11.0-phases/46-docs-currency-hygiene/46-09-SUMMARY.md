---
phase: 46-docs-currency-hygiene
plan: 09
subsystem: battalion-commander
tags: [commander, maneuver, aegis, warn, rustdoc, semver-checks, migration, d-24, wr-04, wr-05]

requires:
  - phase: 44
    provides: the Aegis policy, the Commander's Maneuver bridge (D-04) and the Phase 44 review follow-ups folded here
  - phase: 46-08
    provides: legacy_keys.rs and the try_from BattalionConfig, left untouched here
provides:
  - "Commander private pure fn maneuver_ignored_aegis_policies and a one-time warn! per Maneuver execution when aegis.on_error or aegis.retry is set"
  - "Maneuver bullet on BattalionConfig.aegis; Commander, CommanderBuilder, build() and default_maneuver_config rustdoc state the Commander adds no bound of its own"
  - "Dated WR-05 removal notes above both cargo-semver-checks lint tables"
  - "IN-05 circuit breaker and IN-N1 execute_with_cancellation rustdoc"
  - "MIGRATION.md M-B-11 sentence, CHANGELOG and paladin-battalion CHANGELOG entries"
  - "Phase 44 review follow-ups todo closed (moved to completed/)"
affects: [46-10 and 46-11 (M-B-11 moves into section 10 already correct), 46-12, 46-14]

tech-stack:
  added: []
  patterns:
    - "Pure decision function for a log-only behaviour, unit-tested directly because the repository does not capture log output"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/commander.rs
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-core/Cargo.toml
    - crates/paladin-battalion/Cargo.toml
    - src/infrastructure/resilience/circuit_breaker.rs
    - crates/paladin-battalion/src/phalanx_service.rs
    - .planning/todos/completed/2026-10-11-phase-44-review-follow-ups.md

key-decisions:
  - "maneuver_ignored_aegis_policies is a module-level private fn (not an associated fn) so the test module reaches it through use super::*; it returns fields in the fixed order aegis.on_error, aegis.retry and never names the timeout Maneuver honours"
  - "The warn! sits in the Maneuver branch before the maneuver_config line and uses the Campaign message's shape; it fires even when an explicit ManeuverConfig is supplied, because that config replaces only the derived Maneuver settings, not the Battalion's aegis"
  - "No allow line added or removed: WR-05 is comments only, so make check-migration-allowlist stays green"
  - "IN-01..IN-04 and IN-N2 (test tightening and small cleanups) stay deferred, as 46-CONTEXT D-24 allows"

patterns-established:
  - "A policy a pattern cannot honour is warned about once per execution and documented per pattern on BattalionConfig.aegis"

requirements-completed: [CURR-23]

duration: ~25min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 09: Phase 44 review follow-ups (D-24) Summary

**A Commander running the Maneuver strategy now logs one `warn!` per execution when `aegis.on_error` or `aegis.retry` is set, from a tested pure function `maneuver_ignored_aegis_policies`, and every doc a consumer reads (BattalionConfig.aegis, Commander, CommanderBuilder, M-B-11) states which `aegis` fields each pattern honours; WR-05, IN-05 and IN-N1 are closed and the Phase 44 follow-ups todo is moved to `completed/`.**

## Performance

- **Duration:** ~25 min (start time not captured at launch; estimated from the preceding docs commit `dd2c2312` to the last verification run at 14:53Z)
- **Completed:** 2026-10-24
- **Tasks:** 2 (Task 1 tracer, Task 2 auto)
- **Files modified:** 10 (no files created)

## Accomplishments

- **WR-04 (behaviour).** `maneuver_ignored_aegis_policies(&Aegis) -> Vec<&'static str>` returns the set fields in the order `aegis.on_error`, `aegis.retry`; the Maneuver branch of the Commander calls it with `&self.config.aegis` and emits `Maneuver {name} ignores {fields joined by " and "}: Maneuver reads only aegis.timeout.run_timeout, as one whole-flow clock`. No `timeout(` call and no removed whole-run field name was added to the production lines of `commander.rs` (legacy guard rule 5 stays green).
- **WR-04 (docs).** `BattalionConfig.aegis` gains a **Maneuver** bullet after the Conclave bullet; the `Commander` feature list, `CommanderBuilder` "Optional Fields" and `build()` rustdoc say the Commander adds no bound of its own (ChainOfCommand and Grove do not read `aegis`, so the 300 s default does not bound them); `default_maneuver_config` documents that the per-attempt `run_timeout` becomes a whole-flow clock.
- **Recorded with the code.** MIGRATION.md row M-B-11 gains the Phase 46 D-24 sentence; CHANGELOG `[Unreleased]` `### Changed` and the paladin-battalion CHANGELOG `[Unreleased]` `### Changed` each gain a bullet.
- **WR-05.** Dated "2026-10-24 (Phase 46, D-24 / WR-05)" comments above `[package.metadata.cargo-semver-checks.lints]` in paladin-core (names `struct_missing`, `enum_missing`, `enum_variant_missing`) and paladin-battalion (names `function_missing`, `module_missing`): remove once 0.11.0 is the published semver baseline; until then the allows also mask an accidental public removal. Allow-line diff against the plan base is 0.
- **IN-05.** Circuit breaker "Which failures count" rustdoc: a `Permanent` or `Unknown` failure leaves the state unchanged (confirmed against `call`, which only invokes `on_failure` for `Transient`): no count increment or reset in `Closed`, no count against or re-open of a `HalfOpen` probe. The code matches the review's wording.
- **IN-N1.** `execute_with_cancellation` rustdoc: only waiting and backing-off Paladins stop; an in-flight attempt finishes and keeps its permit; `FirstSuccess` does not read `max_concurrency`.
- **Todo closed.** `git mv` to `.planning/todos/completed/`, `completed: 2026-10-24` added to the frontmatter and a Resolution line naming WR-04, WR-05, IN-05, IN-N1 done and IN-01..IN-04, IN-N2 deferred per 46-CONTEXT D-24.

## Task Commits

1. **Task 1 (tracer): Commander Maneuver warning and the bound documentation** - `3cf57a00` (feat)
2. **Task 2: WR-05 notes, IN-05 and IN-N1 rustdoc, todo closed** - `68d03cdb` (docs)

**TDD (Task 1):** RED was the test `maneuver_ignored_aegis_policies_names_only_the_fields_maneuver_drops` written first; `cargo test --offline -p paladin-battalion --lib maneuver_ignored_aegis_policies` failed to compile with `E0425: cannot find function maneuver_ignored_aegis_policies` (5 errors, one per call site). GREEN added the function; the test then passed. RED and GREEN landed in one commit, as the plan's single commit message specifies, so there is no separate `test(...)` commit.

**Tracer gate (auto mode):** after the commit the tracer `<verify>` was re-run from the committed tree (`maneuver_ignored_aegis_policies` 1 passed, `commander` 62 passed, `legacy_removal_guard` 3 passed, `make check-migration-allowlist` green); it passed, so the expansion task proceeded.

## Verification

| Check | Result |
|-------|--------|
| `cargo test --offline -p paladin-battalion --lib maneuver_ignored_aegis_policies` | pass (1) |
| `cargo test --offline -p paladin-battalion --lib commander` | pass (62) |
| `cargo test --offline --test legacy_removal_guard` | pass (3), run after each task |
| Task 1 greps (call on `&self.config.aegis`, `**Maneuver**`, M-B-11 mentions D-24, zero `TBD` in MIGRATION.md) | pass |
| Task 2 verify (both WR-05 notes, allow-line delta 0, todo moved) | pass |
| `make check-migration-allowlist` | pass (after each task) |
| `cargo clippy --offline -p paladin-battalion --all-targets -- -D warnings` and `-p paladin-ai` | pass |
| `make clean-code` (fmt, clippy, check, doc-check) | pass |
| `make doc-check` | pass (zero warnings) |
| `make api-surface` | pass (surface unchanged, 4323 items; the helper is private) |
| `make check-api-examples`, `make check-gates`, `make check-consumer-surface` | pass |
| `make security` | not run: fails on the pre-existing RUSTSEC-2026-0386 (tokio) advisory unrelated to this phase, per the environment notes; no dependency was added |
| Coverage (82 %, ADR-0006) | via CI |

## Deviations from Plan

None - plan executed exactly as written.

Observations, not deviations:

- paladin-core's lint table also carries `module_missing` and `trait_missing` allows; the plan's WR-05 note names only the three struct/enum lints, and the note was written as specified.
- Both Cargo.toml notes cite `make check-migration-allowlist` and the mirrored `.cargo/semver-checks-allowlist.toml`, which is where those allows are cross-checked.
- M-B-11 still sits in section 9.1 of MIGRATION.md; plan 46-10 renames the section, and the row moves once, already correct.

## Deferred Issues

IN-01 (retry `warn!` says "retrying" when backoff is then cancelled), IN-02 (dead `_started_at` binding in `formation_service.rs`), IN-03 (`test_formation_retry_then_continue` asserts only `log.len() >= 4`), IN-04 (duplicate Paladin names ambiguous in `node_errors`) and IN-N2 (permit-race and cancelled FirstSuccess/Majority test gaps) stay deferred, as 46-CONTEXT D-24 allows. They are recorded in the closed todo's Resolution line.

## Authentication Gates

None.

## Known Stubs

None. Scanned the touched files; no placeholder values or unwired data were introduced.

## Threat Flags

None. No new network endpoint, auth path, file access or schema change. T-46-17 (Maneuver silently dropping `on_error` / `retry`) is mitigated by the tested warning; T-46-18 (crate-wide semver allows) is accepted with the dated removal condition recorded.

## Issues Encountered

None. All `--offline` cargo runs succeeded from the local cache (no online fallback needed).

## Next Phase Readiness

Ready for 46-10 (MIGRATION.md section 10 rename), which carries M-B-11 forward with the D-24 sentence already in place.

## Self-Check: PASSED

- Modified files exist: commander.rs, battalion/mod.rs, MIGRATION.md, both CHANGELOGs, both Cargo.toml, circuit_breaker.rs, phalanx_service.rs, and the completed todo; the pending todo path is gone.
- Commits `3cf57a00` and `68d03cdb` are in `git log`; neither contains an unexpected file deletion (the only removal is the intentional todo rename).
- Every task acceptance criterion was re-run and passed; plan-level verification results are in the table above.
