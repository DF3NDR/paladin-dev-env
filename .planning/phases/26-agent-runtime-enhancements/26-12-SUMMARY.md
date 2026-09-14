---
phase: 26-agent-runtime-enhancements
plan: 12
subsystem: agent-runtime
tags: [structured-output, json-schema, schemars, thiserror, tdd, hexagonal-architecture]

# Dependency graph
requires:
  - phase: 26-agent-runtime-enhancements
    provides: "26-03: PaladinError's #[non_exhaustive] structured-variant discipline (X-06) and D-19 defaulted-method pattern precedent"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "D-11's JSON-envelope extraction rule, originally private to DirectiveParser::StructuredDirective"
provides:
  - "paladin_core::platform::container::structured: Structured<T>, StructuredOptions, SchemaRef, ShapeError, extract_json, shape_check, render_instruction_block -- pure, no new paladin-core dependency"
  - "paladin_ports::output::structured_executor_port: StructuredExecutorPort trait (object-safe at the JSON level) and run_structured, the generic bounded repair-loop driver"
  - "PaladinError::StructuredOutputInvalid { attempts, last_error, raw_output }"
  - "DirectiveParser::StructuredDirective refactored onto extract_json -- exactly one JSON-envelope-extraction implementation in the workspace"
  - "schemars = \"1.2\" as a direct facade dependency, zero new lockfile packages"
affects: ["26-17 (PaladinExecutionService's StructuredExecutorExt, native execute_structured<T>)", "26-18 (WarEngine node dispatch through execute_json_schema_observed)", "26-19 (ToolCallProtocolMiddleware, a third extract_json consumer)", "26-21 (MIGRATION.md 9.2 row extension for StructuredOutputInvalid)"]

# Tech tracking
tech-stack:
  added: ["schemars 1.2 (facade-only, pinned to the version already resolved via rmcp 2.1.0)"]
  patterns:
    - "Bounded repair-loop driver parameterized over an execute_fn closure so one implementation serves both a service (26-17) and an engine node path (26-18)"
    - "D-19 defaulted-method pattern reused a third time: execute_json_schema_observed defaults to execute_json_schema and beats nothing (a correct claim, not a stub)"
    - "Documented partial JSON Schema shape_check (7 keywords) instead of pulling in a full jsonschema validator dependency"
    - "Non-exhaustive value struct requiring an explicit ::new() constructor, since #[non_exhaustive] blocks both struct-literal and ..Default::default() functional update cross-crate"

key-files:
  created:
    - crates/paladin-core/src/platform/container/structured.rs
    - crates/paladin-ports/src/output/structured_executor_port.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/lib.rs
    - crates/paladin-core/src/platform/container/paladin_error.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-battalion/src/engine/directive_parser.rs
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "extract_json is a lift, not a rewrite: DirectiveParser's Envelope-specific deserialization stays local to directive_parser.rs; only the raw JSON-extraction rule (D-11) moved to paladin-core"
  - "shape_check enforces exactly type/required/properties/additionalProperties:false/enum/items/anyOf-with-null and documents everything it does NOT check in its own rustdoc (D-30)"
  - "schemars added to the facade only, pinned to \"1.2\" -- Cargo.lock diff is a single added line, confirmed no third schemars version entered the graph"
  - "StructuredOptions::new(u32) added beyond the plan's explicit file list (Rule 3 deviation) -- #[non_exhaustive] made the struct otherwise unconstructible from paladin-ports or any future facade code"

patterns-established:
  - "Repair-prompt wording re-inserts the offending model output as explicitly-labelled quoted DATA, never as instructions (T-26-40, D-41's delimited-section discipline)"

requirements-completed: [RT-05]

coverage:
  - id: D1
    description: "paladin-core's structured module: Structured<T>, StructuredOptions (default max_repair_attempts=1), SchemaRef, ShapeError, extract_json, shape_check, render_instruction_block -- pure, no new dependency"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/structured.rs#tests (9 tests: extract_json_takes_a_bare_json_object, extract_json_takes_the_first_fenced_json_block, extract_json_returns_none_for_non_json, shape_check_enforces_exactly_the_documented_subset, shape_check_accepts_a_conforming_value, render_instruction_block_is_deterministic, structured_and_options_round_trip)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc (86 passed, includes structured.rs's 3 doc examples)"
        status: pass
    human_judgment: false
  - id: D2
    description: "DirectiveParser::StructuredDirective refactored onto extract_json with no behaviour change -- all 16 pre-existing tests pass unmodified"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib directive_parser (16 passed, same count as before the lift)"
        status: pass
      - kind: other
        ref: "git diff HEAD~1 -- crates/paladin-battalion/src/engine/directive_parser.rs | grep -c '^-.*#\\[test\\]' == 0; grep -c 'fn extract_envelope' == 0; grep -c 'extract_json' >= 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "StructuredExecutorPort (object-safe at the JSON level, D-27) and run_structured, the generic bounded repair-loop driver with a typed PaladinError::StructuredOutputInvalid exhaustion error preserving raw output"
    requirement: "RT-05"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/structured_executor_port.rs#tests (8 tests: first_attempt_appends_the_instruction_block, valid_first_response_returns_without_a_repair, repair_succeeds_on_attempt_two, exhaustion_returns_the_typed_error_with_raw_preserved, zero_repair_attempts_means_one_call, a_shape_failure_repairs_like_a_parse_failure, the_driver_is_stateless_across_invocations, structured_executor_port_is_object_safe_and_send_sync)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib paladin_error (paladin_error_transience_table extended with StructuredOutputInvalid => Permanent)"
        status: pass
    human_judgment: false
  - id: D4
    description: "schemars = \"1.2\" added as a direct facade dependency, pinned to the version already resolved via rmcp, zero new lockfile packages, no new cargo feature, jsonschema not added"
    requirement: "RT-05"
    verification:
      - kind: other
        ref: "git diff --stat -- Cargo.lock (1 line added: 'schemars 1.2.1' under paladin-ai's deps); grep -c '^name = \"schemars\"$' Cargo.lock == 2; grep -c jsonschema Cargo.toml == 0"
        status: pass
    human_judgment: false

# Metrics
duration: ~50min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 12: Structured Output Core Machinery Summary

**Pure structured-output value types, extraction, and shape-checking land in `paladin-core`; a generic bounded repair-loop driver with a typed exhaustion error lands in `paladin-ports`; `DirectiveParser` is refactored onto the shared extraction rule; `schemars` becomes a pinned direct facade dependency adding zero new lockfile packages.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-09-07
- **Tasks:** 2 (both TDD, RED then GREEN)
- **Files modified:** 8 (2 created, 6 modified)

## Accomplishments

- `paladin_core::platform::container::structured` module: `Structured<T> { value, raw }`, `StructuredOptions { max_repair_attempts: 1 default }` (plus a `::new()` constructor), `SchemaRef::{Inline, Registered}`, `ShapeError`, `extract_json`, `shape_check`, `render_instruction_block` — all pure, zero new `paladin-core` dependency (ADR-0015 intact).
- `extract_json` is the Phase 23 D-11 rule lifted verbatim out of `DirectiveParser`'s former private `extract_envelope`/`first_fenced_json_block` helpers — exactly one implementation of "find the JSON in this model output" now exists in the workspace. `DirectiveParser::StructuredDirective` calls it directly; all 16 pre-existing tests pass unmodified.
- `shape_check` enforces a documented 7-keyword JSON Schema subset (`type`, `required`, `properties` recursively, `additionalProperties: false`, `enum`, `items`, `anyOf`-with-null nullability) and states in its own rustdoc exactly what it does not check (`minLength`, `pattern`, `format`, etc.) — no `jsonschema` dependency added.
- `paladin_ports::output::structured_executor_port` module: `StructuredExecutorPort` trait (object-safe at the JSON level, D-27, with a D-19-style defaulted `execute_json_schema_observed`) and `run_structured`, the generic bounded repair-loop driver parameterized over an `execute_fn` closure — one implementation plans 26-17 and 26-18 will both call.
- `PaladinError::StructuredOutputInvalid { attempts, last_error, raw_output }` — free under the pre-existing `#[non_exhaustive]` attribute (no new MIGRATION.md §9.2 row), classified `Permanent` in `transience()`.
- `schemars = "1.2"` added as a direct dependency of the root facade only, pinned to the minor version already resolved via `rmcp 2.1.0` — `Cargo.lock` diff is a single added line.

## Task Commits

Each task followed RED-then-GREEN (TDD):

1. **Task 1: The pure core machinery and the extract_json lift**
   - `74d3f7aa` test(26-12): add failing structured-output core machinery tests (RED)
   - `948aa7fd` feat(26-12): implement extract_json and lift DirectiveParser onto it (GREEN)
2. **Task 2: The bounded repair driver, the typed exhaustion error, and schemars as a facade dependency**
   - `4faf1108` test(26-12): add failing bounded repair-loop driver tests (RED)
   - `120944cd` feat(26-12): wire shape_check into the repair loop; pin schemars (GREEN)

**Plan metadata:** (this commit, see below)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/structured.rs` — new: `Structured<T>`, `StructuredOptions`, `SchemaRef`, `ShapeError`, `extract_json`, `shape_check`, `render_instruction_block`, 9 unit tests.
- `crates/paladin-core/src/platform/container/mod.rs` — registers `pub mod structured;`.
- `crates/paladin-core/src/lib.rs` — re-exports `Structured`, `StructuredOptions`, `SchemaRef` from the crate prelude (free functions stay reachable only by full path).
- `crates/paladin-core/src/platform/container/paladin_error.rs` — adds `PaladinError::StructuredOutputInvalid`; extends `transience()`'s match and its test table.
- `crates/paladin-ports/src/output/structured_executor_port.rs` — new: `StructuredExecutorPort` trait, `run_structured`, `repair_prompt`, 8 unit tests.
- `crates/paladin-ports/src/output/mod.rs` — registers `pub mod structured_executor_port;`.
- `crates/paladin-battalion/src/engine/directive_parser.rs` — `extract_envelope`/`first_fenced_json_block` deleted; `DirectiveParser::parse` calls `extract_json` directly, then deserializes the result into the local `Envelope` type.
- `Cargo.toml` / `Cargo.lock` — `schemars = "1.2"` added to the facade's `[dependencies]`.

## Decisions Made

- **`extract_json` is a lift, not a rewrite.** The pure JSON-extraction rule (whole trimmed output if it's a JSON object, else the first ` ```json ` fenced block) moved to `paladin-core`. `Envelope`-specific concerns — `deny_unknown_fields`, the `next` shape, `parley` raise-time validation — stay local to `directive_parser.rs`, unchanged. This is a narrower rule than the module's original combined "parses as object AND deserializes into a valid envelope" first-branch description; no existing test exercises the distinction (a whole-output JSON object that fails Envelope validation while a later fenced block would have supplied one), so behaviour parity holds against the pinned test suite as the plan's contract requires.
- **`shape_check`'s subset is exactly seven keywords, documented as a ceiling, not a floor.** A full `jsonschema` validator was explicitly rejected (D-30, X-11.4) as a new heavyweight default-feature dependency for no acceptance criterion.
- **`schemars` is facade-only.** No `paladin-core` or `paladin-ports` dependency added; `Cargo.lock`'s diff is a single line, confirming the version pin reuses the existing `rmcp`-resolved `1.2.1` rather than introducing a third `schemars` version.
- **`run_structured` is parameterized over `execute_fn: Fn(String) -> Fut`, not a `Paladin` reference.** This keeps the driver a pure, reusable primitive; binding it to a concrete `Paladin`/`PaladinExecutorPort` is deferred to plan 26-17 (the service) and plan 26-18 (the engine node path), both of which supply their own closures.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `StructuredOptions::new(u32)` constructor**
- **Found during:** Task 2 (writing `structured_executor_port.rs`'s test suite)
- **Issue:** `StructuredOptions` is `#[non_exhaustive]` (per the plan's own spec, matching `PaladinResult`'s and `PaladinError`'s existing X-10.3 discipline). `#[non_exhaustive]` blocks BOTH struct-literal construction (`StructuredOptions { max_repair_attempts: 1 }`) AND `..Default::default()` functional-update syntax from outside the defining crate. Without a constructor, `paladin-ports` (this plan's own test suite) — and every future downstream caller in 26-17/26-18 needing a non-default `max_repair_attempts` — would have had no way to construct a custom value at all, only `StructuredOptions::default()`.
- **Fix:** Added `impl StructuredOptions { pub fn new(max_repair_attempts: u32) -> Self { .. } }` to `structured.rs`, doc-tested. All test-suite construction sites in `structured_executor_port.rs` use `StructuredOptions::new(n)`.
- **Files modified:** `crates/paladin-core/src/platform/container/structured.rs` (outside the plan's declared `<files>` list for Task 2, which named `structured_executor_port.rs`, `output/mod.rs`, `paladin_error.rs`, `Cargo.toml` — `structured.rs` itself needed one addition to be usable cross-crate at all)
- **Verification:** `cargo test -p paladin-ports --lib structured_executor_port` — 8/8 pass; `cargo test -p paladin-ai-core --doc` includes the new constructor's doc example, passes.
- **Committed in:** `74d3f7aa` (added in Task 1's RED commit, since it's part of `structured.rs`'s public surface written there) — used starting in Task 2's `4faf1108` RED commit.

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking issue).
**Impact on plan:** Necessary for the plan's own design (`#[non_exhaustive]` on `StructuredOptions`, per the plan text) to be usable at all outside `paladin-core`. No scope creep — the constructor's only purpose is enabling exactly the cross-crate construction the plan's test suite requires.

## Issues Encountered

**`cargo tree -i schemars` is ambiguous on this toolchain (cargo 1.97.1) when two versions are resolved** — it errors `specification is ambiguous` instead of listing both, so the plan's literal verify command (`cargo tree -i schemars | grep -c '^schemars v'`) cannot run as written here. Verified the same underlying fact (exactly two `schemars` versions, no third) via `cargo tree -i schemars@0.9.0` / `cargo tree -i schemars@1.2.1` individually and via `grep -c '^name = "schemars"$' Cargo.lock` (both confirm `2`). Not a defect in the dependency graph — a toolchain-version quirk in `cargo tree -i`'s ambiguity handling. No code change; documented here for the next executor who hits the same command.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. Both `paladin-core`'s pure machinery and `paladin-ports`' repair driver are fully wired end to end (extract → shape-check → repair/exhaust). `schemars` itself has no consumer yet in this plan (by design — plan 26-17's `StructuredExecutorExt` is its first use), which is explicitly called out in D-26 and is not a stub: nothing in this plan's own scope depends on `schemars` being used yet.

## Threat Flags

None. All four threat-register rows for this plan (T-26-12 DoS via unbounded repair, T-26-40 tampering via the repair re-prompt, T-26-41 over-trusting a partial shape check, T-26-42 DoS via deep recursion, T-26-SC schemars supply-chain) are addressed by the implementation as specified in the plan's own `<threat_model>` and required no additional surface: `max_repair_attempts` bounds the loop (pinned by `zero_repair_attempts_means_one_call` and `exhaustion_returns_the_typed_error_with_raw_preserved`); the repair prompt labels the offending output as quoted DATA, not instructions; `shape_check`'s rustdoc documents its partial coverage; recursion is bounded by the caller-authored schema's own depth; `schemars`'s Package Legitimacy Audit verdict (`OK`) is unchanged by this plan.

## Next Phase Readiness

Ready for plan 26-17 (`PaladinExecutionService`'s `StructuredExecutorExt` blanket impl, native `LlmRequest.response_format` wiring, `execute_structured<T>` via `schemars::schema_for!`) and plan 26-18 (`WarEngine` node dispatch through `execute_json_schema_observed`, `NodeSpec::Paladin.output_schema`, fingerprint `v6`). Both consume `run_structured` and the `structured` module's types directly; no additional groundwork needed from this plan. `directive_parser.rs`'s `extract_json` call site is also ready for plan 26-19's `ToolCallProtocolMiddleware` to become the rule's third consumer.

## Self-Check: PASSED

- FOUND: `crates/paladin-core/src/platform/container/structured.rs`
- FOUND: `crates/paladin-ports/src/output/structured_executor_port.rs`
- FOUND: `.planning/phases/26-agent-runtime-enhancements/26-12-SUMMARY.md`
- FOUND commit: `74d3f7aa` (test(26-12): RED, Task 1)
- FOUND commit: `948aa7fd` (feat(26-12): GREEN, Task 1)
- FOUND commit: `4faf1108` (test(26-12): RED, Task 2)
- FOUND commit: `120944cd` (feat(26-12): GREEN, Task 2)

---
*Phase: 26-agent-runtime-enhancements*
*Plan: 12*
*Completed: 2026-09-07*
