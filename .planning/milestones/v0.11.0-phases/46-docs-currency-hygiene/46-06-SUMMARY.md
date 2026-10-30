---
phase: 46-docs-currency-hygiene
plan: 06
subsystem: testing
tags: [consumer-surface, muster, compile-time-pins, web3sec, prelude, model-profile]

requires:
  - phase: 49-consumer-surface-muster
    provides: tests/consumer_surface_muster.rs (compile-time-only pin file, red-muster rule)
provides:
  - four D-04 compile-time pins closing web3sec handoff section 3 (Stockpile::render, BattalionResult counters, prelude glob, llm_port::ModelProfile re-export)
affects: [46-10 MIGRATION v0.11 guide, 46-14 muster header citation retarget]

tech-stack:
  added: []
  patterns:
    - "Glob-import pin in a private module whose only use is the glob, using several items so unused_imports cannot fire"
    - "Type-equality pin through a re-export path (fn taking the re-export, returning the canonical path)"

key-files:
  created: []
  modified:
    - tests/consumer_surface_muster.rs

key-decisions:
  - "Test-only change: no library item, re-export or visibility touched; make api-surface unchanged"
  - "Glob pin isolated in private mod prelude_glob so it cannot clash with the file's explicit imports"

requirements-completed: [CURR-23]

coverage:
  - id: D1
    description: "Stockpile::render pinned as fn(&Stockpile) -> String"
    requirement: CURR-23
    verification:
      - kind: unit
        ref: "make check-consumer-surface (tests/consumer_surface_muster.rs#stockpile_render_pin)"
        status: pass
    human_judgment: false
  - id: D2
    description: "BattalionResult paladin_success_count / paladin_failure_count (usize) and per_paladin_times (HashMap<String, u64>) read pin"
    requirement: CURR-23
    verification:
      - kind: unit
        ref: "make check-consumer-surface (tests/consumer_surface_muster.rs#battalion_result_counts_pin)"
        status: pass
    human_judgment: false
  - id: D3
    description: "use paladin::prelude::*; glob import site resolving PaladinBuilder, LlmPort, BattalionResult"
    requirement: CURR-23
    verification:
      - kind: unit
        ref: "make check-consumer-surface (tests/consumer_surface_muster.rs#prelude_glob::prelude_glob_resolves)"
        status: pass
    human_judgment: false
  - id: D4
    description: "ModelProfile through paladin_ports::output::llm_port re-export is the same type as output::model_profile::ModelProfile"
    requirement: CURR-23
    verification:
      - kind: unit
        ref: "make check-consumer-surface (tests/consumer_surface_muster.rs#llm_port_model_profile_reexport_pin)"
        status: pass
    human_judgment: false

duration: 3min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 06: Consumer-Surface Muster Gaps Summary

**Four compile-time pins added to the consumer-surface muster (Stockpile::render, the three BattalionResult counters, the prelude glob, the llm_port::ModelProfile re-export), each shown red then green, with no library or public-surface change.**

## Performance

- **Duration:** about 3 min
- **Started:** 2026-10-24T13:51:30Z
- **Completed:** 2026-10-24T13:54:32Z
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files modified:** 1 (`tests/consumer_surface_muster.rs`, +38 lines)

## Accomplishments

- `stockpile_render_pin`: fn-pointer coercion `fn(&Stockpile) -> String = Stockpile::render`, after `stockpile_read_pin`.
- `battalion_result_counts_pin`: reads `paladin_success_count: usize`, `paladin_failure_count: usize`, `per_paladin_times: HashMap<String, u64>`, after `node_error_read_pin`.
- Private `mod prelude_glob` (only `use paladin::prelude::*;`) with `prelude_glob_resolves`, using `PaladinBuilder`, `LlmPort` and `BattalionResult` only through the glob, after `prelude_aegis_names_agree`.
- `llm_port_model_profile_reexport_pin`: takes `paladin_ports::output::llm_port::ModelProfile`, returns the `output::model_profile::ModelProfile` already imported, so the two paths must name one type; placed beside `prelude_and_ports_agree`.
- The single `#[test] the_consumer_surface_still_resolves` body is unchanged (comments only); every new pin is in a never-called function or private module.

## Task Commits

1. **Task 1 (tracer): pin Stockpile::render** - `d1dd2532` (test)
2. **Task 2: pin counters, prelude glob, llm_port ModelProfile re-export** - `04d97b85` (test)

## Red-then-green proofs (D-00f red-muster rule)

Each pin was mutated once, built with `cargo test --offline --test consumer_surface_muster --no-run`, the error recorded, and the file restored.

1. `Stockpile::render` (return type mis-typed as `&str`):
   ```
   error[E0308]: mismatched types
     --> tests/consumer_surface_muster.rs:637:43
       let _render: fn(&Stockpile) -> &str = Stockpile::render;
                    ----------------------   ^^^^^^^^^^^^^^^^^ expected fn pointer, found fn item
       = note: expected fn pointer `for<'a> fn(&'a Stockpile) -> &'a str`
                     found fn item `for<'a> fn(&'a Stockpile) -> std::string::String {Stockpile::render}`
   ```
2. `BattalionResult` counters (`paladin_success_count` read declared as `u32` in the return tuple):
   ```
   error[E0308]: mismatched types
     --> tests/consumer_surface_muster.rs:524:9
       result.paladin_success_count,
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `u32`, found `usize`
   ```
3. Prelude glob (a glob-reached name, `PaladinBuilder`, renamed to a non-existent `PaladinBuilderMissing`):
   ```
   error[E0425]: cannot find type `PaladinBuilderMissing` in this scope
     --> tests/consumer_surface_muster.rs:553:18
   ```
4. `llm_port` re-export (path mis-typed as `llm_port::ModelProfileMissing`):
   ```
   error[E0425]: cannot find type `ModelProfileMissing` in module `paladin_ports::output::llm_port`
     --> tests/consumer_surface_muster.rs:100:47
   ```

Green after restore: `make check-consumer-surface` exit 0 (`the_consumer_surface_still_resolves ... ok`).

## Verification Results

| Check | Result |
|-------|--------|
| Task 1 verify (greps + `make check-consumer-surface`) | exit 0, re-run after commit as the tracer gate: pass |
| Task 2 verify (greps + `make check-consumer-surface`) | exit 0 |
| `cargo clippy --offline --test consumer_surface_muster -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `make api-surface` | exit 0 (surface unchanged) |
| `make check-gates` | exit 0 |
| `cargo test --offline --test legacy_removal_guard --test entity_rename_guard` | 8 passed, 3 passed |
| `git diff --stat` for the plan | only `tests/consumer_surface_muster.rs` |

`mdbook` legs are not applicable (no docs touched). `make security` was not run; it fails on the pre-existing RUSTSEC-2026-0386 (tokio) advisory unrelated to this plan.

## Deviations from Plan

None - plan executed exactly as written. The `--offline` builds succeeded from the local cache, so no online fallback was needed.

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. Test-only change with no new network, auth, file or schema surface. T-46-09 (consumer build broken by an unpinned surface change) is mitigated by the four pins.

## Next Phase Readiness

The surface web3sec adapted to at `e28e43b5` is now fully pinned, so the v0.10 to v0.11 guide plans (46-10, 46-11) and the 46-14 muster-header citation retarget can proceed. Ready for the next plan.

## Self-Check: PASSED

- `tests/consumer_surface_muster.rs` contains `fn stockpile_render_pin`, `fn battalion_result_counts_pin`, `mod prelude_glob`, `fn llm_port_model_profile_reexport_pin`: FOUND
- Commits `d1dd2532` and `04d97b85` exist on `claude/laughing-dirac-e0h2ax`: FOUND
- No file deletions in either commit.
