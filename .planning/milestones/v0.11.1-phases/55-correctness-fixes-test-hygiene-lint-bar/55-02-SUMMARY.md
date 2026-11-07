---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 02
subsystem: run-worker
tags: [treasurer, halt-reason, model-unpriced, openapi, adr-0057, fixes-03]
status: complete

requires:
  - phase: 42-allowance-enforcement
    provides: "HaltReason, wire_json as the one caller-facing builder, the worker halt path (persist_agent_halt)"
provides:
  - "HaltReason::ModelUnpriced { model } (wire tag model_unpriced), the one intentional public addition of the phase"
  - "an agent-kind run whose model lost its treasurer.pricing row between admission and dispatch records Halted, not Failed, on the row, the SSE terminal event and the webhook"
  - "OpenAPI description, Platform API reference, Treasurer guide and ADR-0057 note naming the third halt reason"
affects: [55-12, 55-13]

tech-stack:
  added: []
  patterns:
    - "A no-call dispatch outcome is a DispatchBudget::Halt(reason); the private enum has no outcome that records Failed"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/allowance.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/openapi.json
    - docs/src/api-reference/platform-api.md
    - docs/src/architecture/treasurer.md
    - .planning/decisions/0057-mid-run-halt-contract.md

key-decisions:
  - "The new variant carries the model name only (D-00g); one builder, wire_json, serves every caller-facing surface"
  - "ADR-0057 decision text is unchanged; a dated amendment note records the Failed-to-Halted change (PD-02-2)"

patterns-established:
  - "Prove-then-lock for a flipped test: record the old test passing, watch the new assertions fail to compile, then go green"

requirements-completed: [FIXES-03]

duration: ~45 min
completed: 2026-11-01
---

# Phase 55 Plan 02: model_unpriced halt Summary

**An agent-kind run whose model lost its `treasurer.pricing` row between admission and dispatch now halts resumably with `HaltReason::ModelUnpriced { model }` instead of failing, with no LLM call, and the OpenAPI document, Platform API reference, Treasurer guide and ADR-0057 agree.**

## Plan-start SHA

`772bb1b0e149ee298e31e60c5357cb5fe696da1c`

## Performance

- **Tasks:** 2 of 2
- **Files modified:** 8 (0 created)

## Accomplishments

- `HaltReason` gains `ModelUnpriced { model: String }` (still `#[non_exhaustive]`; serde form `{"reason":"model_unpriced","model":"gpt-4"}`), with `as_str`, `herald_line` and `wire_json` arms and a doctest assertion on the enum and each of the three methods.
- `derive_dispatch_budget` maps `AdmissionError::ModelUnpriced` to `DispatchBudget::Halt(HaltReason::ModelUnpriced { model })`. The private variant that recorded `Failed` and its match arm are deleted, so the one `Halt` arm serves all three no-call outcomes. The enum, method and comment text say so.
- The worker regression test is flipped and renamed to `agent_kind_run_whose_model_became_unpriced_halts_without_calling_the_llm`. It asserts `call_count() == 0`, row `Halted` with `error` and `output` both `None`, the exact `HaltReason` value, exactly one `Done` terminal event with `status: "halted"` and `halt_reason == wire_json()`, and a `Halted` webhook delivery with the same `halt_reason`.
- Published text updated: `RunResponse.halt_reason` field doc, regenerated `openapi.json` (one description string), `platform-api.md` (Halted runs, SSE `done`, dispatch bullet), `treasurer.md`, and a dated amendment note in ADR-0057.

## Task Commits

1. **Task 1 (tracer): domain variant, worker mapping, flipped test** - `95ce3641a` (fix)
2. **Task 2: OpenAPI, Platform API, Treasurer guide, ADR-0057 note** - `5b24e53b7` (docs)

## Prove-then-lock (D-00e)

Run before any change, at plan-start SHA `772bb1b`:

`cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced`
-> `agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm ... ok` (1 passed). The existing test passed asserting `Failed`, so the mapping existed.

Watched failing (tests written first, variant absent):

- `cargo test -p paladin-ai-core --offline --lib allowance::` -> `error[E0599]: no variant named 'ModelUnpriced' found for enum 'allowance::HaltReason'` (allowance.rs:1386; `could not compile paladin-ai-core (lib test)`)
- `cargo test -p paladin-ai --features web-server --lib --offline agent_kind_run_whose_model_became_unpriced` -> `error[E0599]: no variant named 'ModelUnpriced' found for enum 'paladin_core::platform::container::allowance::HaltReason'` (worker_tests.rs:2991; `could not compile paladin-ai (lib test)`)

Green after the domain and worker changes: `allowance::` lib tests 37 passed (including `model_unpriced_serde_form_names_only_the_model`, `model_unpriced_herald_line_names_the_model`, `model_unpriced_wire_json_matches_the_serde_form`), `allowance` doctests 20 passed, `worker_tests` module 32 passed (including the flipped test), `ledger_unavailable_halt_resumes_after_recovery` passed, `paladin-storage --features sqlite run::` 74 passed (an unreadable stored `halt_reason` still decodes as `None`), `paladin-herald` 60 passed. The tracer's verify was re-run end to end in auto mode (`workflow._auto_chain_active=true`) before Task 2 and passed.

## openapi.json diff stat

`git diff --stat crates/paladin-web/openapi.json` -> `1 file changed, 1 insertion(+), 1 deletion(-)`; `git diff 772bb1b -- crates/paladin-web/openapi.json | grep -c '^[-+] '` prints `2`: the `halt_reason` description string only, no schema change. `openapi_matches_committed_baseline` and the 12 `openapi` lib tests pass; `make check-version-strings` and `make check-doc-config` exit 0.

## Other gates

- `cargo fmt --check` clean; `cargo clippy -p paladin-ai-core --tests --offline -- -D warnings` and `cargo clippy -p paladin-ai --features web-server --lib --tests --offline -- -D warnings` clean; `cargo doc -p paladin-ai-core --no-deps --offline` prints 0 warnings.
- `make api-surface` reports "API surface unchanged" (4325 items), as PD-02-3 expected. Plan 55-13 owns the `cargo-semver-checks` proof.
- `git diff 772bb1b -- crates/paladin-storage/migrations` is empty; no migration was touched.
- ADR-0057 diff against plan-start has no removed lines (additions only).

## Decisions Made

- The variant carries only `model` (PD-02-1); the tests pin the exact serde, wire and herald strings, so no tenant id, key name or figure can enter it unnoticed (T-55-05).
- The existing test `halt_reason_as_str_names_the_two_tags` was renamed `halt_reason_as_str_names_the_three_tags` and extended, since its name would otherwise be false.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Disk full during clippy**
- **Found during:** Task 1, step 6 (the build volume reported 0 MB free)
- **Issue:** `cargo clippy` failed with "No space left on device" while creating a temp dir under `target/`.
- **Fix:** removed `target/debug/incremental`, which is Cargo's regenerable incremental-compilation cache from my own builds (about 5 GB). The remaining cargo commands ran with `CARGO_INCREMENTAL=0`. No source or tracked file was touched.
- **Files modified:** none (untracked build cache only)

### Plan text corrections (no behaviour change)

**2. ADR-0057 placement:** the plan says to put the amendment "directly after the A4 paragraph". The ADR has no A4 paragraph. The `Failed` clause sits in the **G8** paragraph (section (e)), so the note is placed directly after that paragraph, before A5. Nothing else in the ADR changed.

Otherwise the plan was executed as written.

## Issues Encountered

None beyond the disk-space item above.

## Known Stubs

None. Nothing was appended to `.planning/WINDOWS.md`: no stub, skipped test or unrun `<verify>` arose in this plan.

## Threat Flags

None. The change adds a field to an existing halt object on existing surfaces and no new endpoint, auth path or file access.

## Next Phase Readiness

Plan 55-12 writes the MIGRATION section 11.1 / 11.6 text and the CHANGELOG entries for this addition. Plan 55-13 proves the semver position and runs the phase-head gates.

## Self-Check: PASSED

- FOUND: crates/paladin-core/src/platform/container/allowance.rs (`ModelUnpriced`, 10 occurrences)
- FOUND: src/application/services/run/worker.rs (`DispatchBudget::Halt(HaltReason::ModelUnpriced`; 0 occurrences of `DispatchBudget::Unpriced`)
- FOUND: src/application/services/run/worker_tests.rs (renamed test; the old name has 0 occurrences)
- FOUND: crates/paladin-web/openapi.json, docs/src/api-reference/platform-api.md, docs/src/architecture/treasurer.md, .planning/decisions/0057-mid-run-halt-contract.md (each contains `model_unpriced` or the amendment note)
- FOUND: commits 95ce3641a and 5b24e53b7
