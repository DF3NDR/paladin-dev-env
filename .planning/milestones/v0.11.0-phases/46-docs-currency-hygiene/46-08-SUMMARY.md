---
phase: 46-docs-currency-hygiene
plan: 08
subsystem: battalion-config
tags: [serde, try_from, legacy-removal, migration, d-23, adr-0059]

requires:
  - phase: 44
    provides: the removal of timeout_seconds / retry_policy / error_strategy and the Aegis replacement (ADR-0059)
  - phase: 46-01
    provides: CHANGELOG headings the Unreleased entries sit under
provides:
  - "battalion/legacy_keys.rs: private wire structs, RemovedConfigKeys, the present helper and the refusal tests"
  - "BattalionConfig and ConclaveConfig refuse a stored document carrying a removed v0.10 key, naming each key, its aegis.* replacement and M-B-06"
  - "ADR-0059 dated amendment; MIGRATION M-B-06 / BattalionConfig / ConclaveConfig / DR-8 text; both CHANGELOG entries"
  - "cli_conclave_yaml_timeout_seconds_still_loads: the CLI YAML key pinned as supported"
affects: [46-10 and 46-11 (move these rows verbatim into section 10), 46-12, 46-14]

tech-stack:
  added: []
  patterns:
    - "#[serde(try_from = <private wire struct>)] with deserialize_with = present (IgnoredAny) members to refuse named legacy keys without changing the public struct, Serialize or the Rust surface"

key-files:
  created:
    - crates/paladin-core/src/platform/container/battalion/legacy_keys.rs
  modified:
    - crates/paladin-core/src/platform/container/battalion/mod.rs
    - crates/paladin-core/src/platform/container/battalion/conclave.rs
    - src/application/cli/config/battalion_config.rs
    - .planning/decisions/0059-legacy-clean-break-removal.md
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - .planning/todos/completed/2026-10-11-reject-removed-v0-10-battalion-config-keys.md

key-decisions:
  - "Refusal, not compatibility: a removed key is never mapped onto its replacement (D-00c / D-00e intact); the error names keys and fixed replacement text only, never a value"
  - "No deny_unknown_fields: only the named removed keys are refused; an unrelated unknown key and every v0.11 document still load"
  - "Key names and message text live only in legacy_keys.rs, outside both of legacy_removal_guard's scoped lists (rules 5 and 6)"
  - "The error cites 'MIGRATION.md section 10.1, row M-B-06' as the plan specifies; plan 46-10 renames section 9 to 10, so the citation is correct once that lands (M-B-06 sits in 9.1 until then)"

patterns-established:
  - "Refusing a removed serde key: a private Wire struct plus Option<()> present-members, converted by TryFrom with a thiserror error"

requirements-completed: [CURR-23]

duration: ~35min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 08: Refuse removed v0.10 Battalion and Conclave config keys Summary

**A stored v0.10 `BattalionConfig` or `ConclaveConfig` that still carries `timeout_seconds`, `retry_policy` or `error_strategy` now fails to load with one error naming each key, its `aegis.*` replacement and MIGRATION row M-B-06, via a private `try_from` wire struct that leaves the public shapes, `Serialize` and the API surface unchanged.**

## Performance

- **Duration:** ~35 min (start time not captured at launch; estimated from the preceding commit at 14:21:54Z to the last code commit at 14:38:09Z)
- **Completed:** 2026-10-24
- **Tasks:** 2 (Task 1 tracer, Task 2 auto; both TDD)
- **Files modified:** 9 (1 created, 1 renamed)

## Accomplishments

- `BattalionConfig` refuses the three removed keys (alone, combined, and with a `null` value), listing only the keys present and never a value.
- `ConclaveConfig` refuses `timeout_seconds` (replacement `battalion_config.aegis.timeout.run_timeout`); a removed key nested in `battalion_config` is refused through the `BattalionConfig` conversion.
- Unrelated unknown keys and v0.11 documents still load; `Serialize` output is unchanged and carries no removed key; struct literals still compile (D-00b: no section 10.2 row, no allowlist entry).
- The CLI's own conclave YAML `timeout_seconds` is pinned as still supported.
- ADR-0059, MIGRATION.md (M-B-06, `BattalionConfig` row, `ConclaveConfig` row, DR-8) and both CHANGELOGs record the behaviour in the same commits as the code; the folded todo is closed.

## Task Commits

1. **Task 1 (tracer): BattalionConfig refusal, ADR amendment, register rows, CHANGELOGs** - `bcec5e8e` (feat)
2. **Task 2: ConclaveConfig refusal, CLI YAML pin, todo closed** - `b0429436` (feat)

**Plan metadata:** committed separately after this summary (docs).

## TDD Record

**Task 1 RED** (`cargo test --offline -p paladin-ai-core --lib battalion`): 8 failed, 111 passed. Failing:
`timeout_key_alone_is_refused_naming_its_replacement_and_row`, `retry_key_alone_is_refused_naming_its_replacement`,
`error_key_alone_is_refused_naming_its_replacement`, `all_three_keys_fail_once_naming_every_key_and_replacement`,
`only_the_keys_present_are_listed`, `a_removed_key_with_a_null_value_is_refused`,
`the_refusal_never_echoes_a_value`, and the inverted
`battalion_config_v0_10_document_is_refused_naming_each_removed_key` (the old v0.10 document loaded). The
pass-through cases (bare document, empty `aegis`, unknown key, serialized shape and round trip) passed already, as
expected. **GREEN:** 119 passed.

**Task 2 RED** (`cargo test --offline -p paladin-ai-core --lib legacy_keys`): 2 failed
(`conclave_timeout_key_is_refused_naming_its_replacement_and_row`,
`conclave_timeout_key_with_a_null_value_is_refused_without_echoing_values`). The nested-battalion-config refusal
already passed through Task 1's conversion. `cli_conclave_yaml_timeout_seconds_still_loads` passed already: it is a
regression pin, not new behaviour. **GREEN:** 124 battalion tests passed.

## Files Created/Modified

- `crates/paladin-core/src/platform/container/battalion/legacy_keys.rs` - new: `present`, `RemovedConfigKeys`, `BattalionConfigWire`, `ConclaveConfigWire`, both `TryFrom` impls, 15 tests
- `crates/paladin-core/src/platform/container/battalion/mod.rs` - `mod legacy_keys;`, `try_from` attribute, "Removed v0.10 keys" rustdoc, inverted fixture test
- `crates/paladin-core/src/platform/container/battalion/conclave.rs` - `try_from` attribute and rustdoc paragraph only
- `src/application/cli/config/battalion_config.rs` - `cli_conclave_yaml_timeout_seconds_still_loads`
- `.planning/decisions/0059-legacy-clean-break-removal.md` - "Amended 2026-10-24" note
- `MIGRATION.md` - M-B-06 change and action cells, `BattalionConfig` and `ConclaveConfig` register rows, DR-8
- `CHANGELOG.md`, `crates/paladin-core/CHANGELOG.md` - `[Unreleased]` entries
- `.planning/todos/completed/2026-10-11-reject-removed-v0-10-battalion-config-keys.md` - moved from `pending/`, `completed:` first line plus resolution note

## Decisions Made

See `key-decisions` above. The error text is built from key names and fixed replacement strings only, so a planted
`sk-planted-value` never appears (tested for all three Battalion keys and the Conclave key).

## Deviations from Plan

None - plan executed exactly as written. Two notes that are not deviations: the `cargo test --offline` commands all
worked from the local cache (no online fallback needed), and `refuse_removed` takes a slice rather than a fixed-size
array so one helper serves both the three-key Battalion and one-key Conclave cases.

## Issues Encountered

- The first Task 2 `git add` aborted because a path named the already-staged `git mv` source; re-staged without it. No commit was affected.
- Commit trailers: the task prompt asked for a different model name in the `Co-Authored-By` line than the harness attribution reminder specifies; the commits carry the harness reminder's `Co-Authored-By: Claude Sonnet 5.5` line plus the `Claude-Session` line. Flagged for the orchestrator.

## Verification Run

- Task 1 and Task 2 `<verify>` commands: both pass (battalion lib tests 124, `cli_conclave_yaml_timeout_seconds_still_loads`, `legacy_removal_guard` 3/3, greps, todo location).
- `cargo test --offline --test legacy_removal_guard --test entity_rename_guard`: 3 + 8 passed.
- `cargo test --offline -p paladin-battalion --lib`: 892 passed (nothing else deserializes a removed key).
- `cargo clippy --offline -p paladin-ai-core --all-targets -- -D warnings` and `cargo clippy --offline -p paladin-ai --features cli --lib -- -D warnings`: clean. `cargo fmt --check`: clean.
- `make api-surface` (unchanged, 4323 items), `make check-migration-allowlist`, `make check-gates`, `make check-api-examples`, `make check-doc-config`, `make check-consumer-surface`, `make doc-check`, `make clean-code`: all exit 0.
- `grep -c TBD MIGRATION.md` is 0; `deny_unknown_fields` appears only in rustdoc prose (described as not used), never on a code line.
- Not run: `make security` (pre-existing cargo-deny advisory RUSTSEC-2026-0386 on tokio, unrelated; recorded and left to CI); the 82 % `coverage` job is CI's gate, and every new `try_from` branch is named by a test.
- Manual credential-handling review (security.instructions.md): the refusal text carries only key names and fixed replacement text; no value is formatted into it; no HTTP client, log statement or config `Debug` path was touched.

## Known Stubs

None.

## Threat Flags

None. The change adds no network endpoint, auth path or file access; T-46-14 (silent unbounded load), T-46-15 (value echo) and T-46-16 (unknown-key forward-compat break) are mitigated and tested.

## User Setup Required

None.

## Next Phase Readiness

Ready for 46-09. Plans 46-10 and 46-11 move the M-B-06, `BattalionConfig`, `ConclaveConfig` and DR-8 rows verbatim into section 10; the refusal message already cites "MIGRATION.md section 10.1, row M-B-06" and becomes accurate when 46-10 renames the headings.

## Self-Check: PASSED

- FOUND: crates/paladin-core/src/platform/container/battalion/legacy_keys.rs
- FOUND: .planning/todos/completed/2026-10-11-reject-removed-v0-10-battalion-config-keys.md (pending copy absent)
- FOUND commits: bcec5e8e, b0429436
- Acceptance criteria of both tasks re-run and passing (see Verification Run)
