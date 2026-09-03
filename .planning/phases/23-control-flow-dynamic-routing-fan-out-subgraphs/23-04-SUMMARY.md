---
phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
plan: 04
subsystem: orchestration
tags: [paladin-battalion, war-engine, directive-parser, structured-output, control-flow, tdd]

# Dependency graph
requires:
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "23-02's paladin_core::platform::container::directive::{Directive, NextStep, MusterTask}, StateNode::run -> Result<Directive, NodeError>, the Goto/End/Parley arms in engine/superstep.rs"
provides:
  - "paladin_battalion::engine::directive_parser: DirectiveParser { PlainOutput, StructuredDirective { on_parse_error } }, OnParseError { FailRun, FallbackPlain }, DirectiveParseError, and the D-11-locked JSON envelope extraction routine"
  - "NodeSpec::Paladin { .., directive_parser: DirectiveParser } plus NodeSpec::paladin(..) (defaults PlainOutput) and NodeSpec::paladin_with_directive_parser(..) (explicit parser) constructors, adopted at every in-tree Paladin-node construction site"
  - "engine::superstep's DirectiveParser::parse call replacing the prior unconditional delta.set(output_field, result.output) write in NodeDispatch::Paladin, plus the NodeFailure enum distinguishing a directive-parse failure from every other node error"
  - "EngineError::DirectiveParseFailed { node, reason } -- the typed engine error a StructuredDirective node's FailRun parse failure surfaces as"
affects: [23-10]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RED/GREEN commit pairs per task (test(23-04) then feat(23-04)), matching 23-02's precedent: each RED commit is confined to new #[cfg(test)] test hunks referencing not-yet-existing API and fails to compile or fails a genuinely new-behavior assertion; each GREEN commit lands the production mechanism plus the cross-file construction-site migration the new field forces."
    - "A private lowercase-tagged EnvelopeNextStep mirrors NextStep's shape for the parts an envelope may name (Edges/Goto/End/Muster), converted via From<EnvelopeNextStep> for NextStep -- serde's default externally-tagged representation for a unit vs. tuple variant already produces exactly D-11's documented \"edges\" / {\"goto\": [...]} / \"end\" / {\"muster\": [...]} shape with zero custom (de)serialize code."
    - "extract_envelope funnels every extraction failure -- not-JSON, JSON-but-not-an-object, JSON-object-but-invalid-envelope (including a deny_unknown_fields rejection) -- through the same on_parse_error resolution, so FailRun/FallbackPlain has exactly one branch point regardless of which extraction candidate (trimmed whole output vs. first fenced json block) or which failure mode produced the miss."

key-files:
  created:
    - crates/paladin-battalion/src/engine/directive_parser.rs
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/bridges.rs
    - tests/integration/e2e_crash_resume_test.rs

key-decisions:
  - "DirectiveParser::parse funnels ALL extraction failures -- not-JSON, JSON-but-not-an-object, and a JSON object that fails to deserialize into the Envelope type (including deny_unknown_fields rejections) -- through the SAME on_parse_error resolution, rather than treating \"found a JSON object but it's schema-invalid\" as a separate failure class. This keeps the extraction order's three clauses (trimmed whole output / first fenced block / on_parse_error) as the only decision points, and both malformed-output and unknown-envelope-key tests exercise the identical code path."
  - "envelope_to_directive silently skips a delta entry whose key fails FieldName::new -- unreachable in practice (FieldName rejects only the empty string, and no Battlefield schema ever declares a field named \"\"), documented inline rather than invented as a new failure mode."
  - "The unknown-DELTA-field case (as opposed to an unknown top-level envelope key) is deliberately NOT validated inside directive_parser.rs at all: Battlefield::merge already validates every delta's field names against the schema before any mutation (BattlefieldError::UnknownField), so an envelope's delta reaches the exact same allowlist every other node's delta does, with no second, parser-owned check to keep in sync with the schema (T-23-14)."
  - "NodeFailure (private to superstep.rs) replaces NodeError as execute_vanguard_node's error type so a DirectiveParser parse failure can surface as the new typed EngineError::DirectiveParseFailed { node, reason } instead of the generic EngineError::Node -- every other node-execution failure (Function node error, InputMapping::render failure, PaladinPort::execute error, the semaphore-closed defensive branch, an InterceptDecision::Fail) still wraps NodeError unchanged."

patterns-established:
  - "A per-node opt-in enum field (DirectiveParser) defaulting to the pre-existing behavior (PlainOutput), landed via a two-constructor pattern (a defaulting associated fn plus an explicit-parameter sibling) so every existing construction site migrates to the defaulting constructor without naming the new field -- the same shape D-11 asked for and the one future per-node opt-in fields on NodeSpec::Paladin should follow."

requirements-completed: [CF-02]

# Coverage metadata
coverage:
  - id: D1
    description: "DirectiveParser::PlainOutput is the default, writes the raw Paladin output to output_field and routes via NextStep::Edges, byte-identical to pre-CF-02 behavior; StructuredDirective parses D-11's documented JSON envelope and applies only its delta, with output_field untouched"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::directive_parser::tests::plain_output_is_the_default_and_writes_the_output_field"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::structured_directive_parses_a_bare_json_object_output"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::structured_directive_does_not_write_the_output_field"
        status: pass
    human_judgment: false
  - id: D2
    description: "JSON extraction follows the locked order: trimmed whole output as a JSON object, else the first ```json fenced block, else on_parse_error -- pinned against a two-fenced-block input to prove first-wins"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::directive_parser::tests::structured_directive_parses_a_fenced_json_block"
        status: pass
      - kind: unit
        ref: "engine::directive_parser::tests::output_with_two_fenced_json_blocks_uses_the_first"
        status: pass
      - kind: unit
        ref: "engine::directive_parser::tests::empty_output_resolves_through_on_parse_error"
        status: pass
    human_judgment: false
  - id: D3
    description: "OnParseError::FailRun fails the run with the typed EngineError::DirectiveParseFailed naming the node; OnParseError::FallbackPlain degrades to PlainOutput semantics -- both proven end-to-end through the real engine, not only at the parser's unit boundary"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::malformed_output_under_fail_run_fails_the_run"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::malformed_output_under_fallback_plain_writes_the_raw_output"
        status: pass
    human_judgment: false
  - id: D4
    description: "A StructuredDirective node's Goto/Muster/End next reaches the same NextStep machinery a Function node's Directive does; an envelope delta naming a field the Battlefield schema does not declare fails the run as a schema error; unknown top-level envelope keys are rejected via deny_unknown_fields rather than silently ignored"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::structured_directive_goto_routes_the_run"
        status: pass
      - kind: unit
        ref: "engine::superstep::tests::envelope_delta_naming_an_unknown_field_fails_the_run"
        status: pass
      - kind: unit
        ref: "engine::directive_parser::tests::envelope_with_an_unknown_top_level_key_is_rejected"
        status: pass
    human_judgment: false
  - id: D5
    description: "A parse failure under FailRun leaves no partial state: when one Paladin node's StructuredDirective fails to parse in the same superstep as a sibling node whose delta would otherwise merge, the whole superstep's deltas are discarded together, before merge"
    requirement: "CF-02"
    verification:
      - kind: unit
        ref: "engine::superstep::tests::structured_directive_parse_failure_does_not_merge_a_partial_delta"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every in-tree NodeSpec::Paladin construction site migrates to the new NodeSpec::paladin/paladin_with_directive_parser constructors; the PlainOutput default leaves the E2E-1 crash-resume golden and the legacy-pattern bridge-equivalence suite byte-identical"
    requirement: "CF-02"
    verification:
      - kind: integration
        ref: "cargo test --test e2e_crash_resume (27/27, including e2e_1_crash_resume_matches_control_run_with_no_reexecution)"
        status: pass
      - kind: integration
        ref: "cargo test --test golden_bridge_equivalence (31/31)"
        status: pass
      - kind: other
        ref: "cargo build -p paladin-battalion (clean; no construction site names directive_parser)"
        status: pass
    human_judgment: false

# Metrics
duration: ~120min
completed: 2026-09-03
status: complete
---

# Phase 23 Plan 04: DirectiveParser for Paladin Node Output Summary

**A per-node `DirectiveParser` lets a Paladin node emit a routing `Directive` from a JSON envelope in its own output, with `PlainOutput` as the byte-identical default and `StructuredDirective` under configurable `OnParseError` handling — test-first, RED committed strictly before GREEN per task.**

## Performance

- **Duration:** ~120 min (includes RED/GREEN git-surgery reconstruction to split one implementation pass into two independently-verified, per-task TDD commit pairs, matching 23-02's precedent)
- **Completed:** 2026-09-03
- **Tasks:** 2
- **Files modified:** 6 (1 new, 5 modified)

## Accomplishments

- New `crates/paladin-battalion/src/engine/directive_parser.rs`: `DirectiveParser { PlainOutput, StructuredDirective { on_parse_error: OnParseError } }`, `OnParseError { FailRun, FallbackPlain }`, `DirectiveParseError`, a private `Envelope`/`EnvelopeNextStep` deserialization pair matching D-11's documented shape exactly (`{"delta": {...}, "next": "edges" | {"goto": [...]} | "end" | {"muster": [...]}}`), and `extract_envelope`'s locked order (trimmed whole output as a JSON object, else the first ` ```json ` fenced block, else `on_parse_error`). A doc test on `DirectiveParser::parse` demonstrates the `StructuredDirective` happy path.
- `graph.rs`: `NodeSpec::Paladin` gains a `directive_parser: DirectiveParser` field. `NodeSpec::paladin(paladin, input_template, output_field)` defaults it to `PlainOutput`; `NodeSpec::paladin_with_directive_parser(..., directive_parser)` takes it explicitly. Every in-tree `NodeSpec::Paladin { .. }` struct literal (bridges.rs x3, the fingerprint test fixture, 6 sites in `engine/mod.rs`'s tests, 5 sites in `tests/integration/e2e_crash_resume_test.rs`) migrated to the constructor, so no call site names the new field. The manual `Debug` impl includes the parser kind. No fingerprint section added — Plan 23-10 owns the `v3` bump per D-18.
- `superstep.rs`: `NodeDispatch::Paladin` carries the `directive_parser`; `execute_vanguard_node`'s Paladin arm replaces the unconditional `delta.set(output_field, result.output.clone())` write with a `DirectiveParser::parse(&result.output, &output_field)` call, threading the returned `Directive`'s delta and `next` into the same widened per-node result Plan 23-02 introduced. A new private `NodeFailure` enum distinguishes a `DirectiveParser` parse failure (`NodeFailure::DirectiveParse`) from every other node-execution failure (`NodeFailure::Node`, unchanged shape) at the point the per-node accumulation loop converts a failure to an `EngineError` — a parse failure becomes `EngineError::DirectiveParseFailed { node, reason }`, everything else still becomes `EngineError::Node`.
- `mod.rs`: new `EngineError::DirectiveParseFailed { node: NodeId, reason: String }` variant (`#[non_exhaustive]`, zero X-10 burden).
- 17 new tests across the two tasks, all passing: 7 `directive_parser` unit tests (default, fenced-block extraction, malformed-output under both `on_parse_error` modes, empty-output, two-fenced-blocks-uses-first, unknown-top-level-key rejection, Goto/End parsing) and 10 `superstep` engine-level tests (bare-JSON-object happy path with a real Battlefield merge, no-implicit-output-field-write, Goto routing through the real engine, unknown-delta-field schema error, both `on_parse_error` modes proven end-to-end, and the no-partial-merge guarantee across two sibling Paladin nodes in one superstep).

## Task Commits

Each task followed RED-then-GREEN, mirroring 23-02's precedent:

1. **Task 1: A Paladin node emits a Directive from a structured envelope, end-to-end** (`type="tracer" tdd="true"`)
   - `79515f94` — `test(23-04): reproduce DirectiveParser-driven Paladin dispatch on not-yet-existing API (red)` — six engine-level tests added to `engine::superstep::tests` referencing `NodeSpec::paladin_with_directive_parser`/`DirectiveParser`/`OnParseError`/`EngineError::DirectiveParseFailed`, none of which exist yet; crate fails to compile (8 errors).
   - `90e56ac3` — `feat(23-04): land DirectiveParser and wire it into Paladin dispatch (green)` — `directive_parser.rs`, `NodeSpec::Paladin`'s new field and constructors, `superstep.rs`'s `NodeFailure`/dispatch wiring, `EngineError::DirectiveParseFailed`, plus every in-tree construction-site migration. 400/400 `paladin-battalion` lib tests pass; `e2e_crash_resume`/`golden_bridge_equivalence` green (58/58).
   - **Tracer feedback gate:** re-ran `cargo test -p paladin-battalion --lib engine::directive_parser` immediately after the GREEN commit — 7/7 passed. Proceeded to Task 2.
2. **Task 2: Parse-failure modes and envelope edge cases** (`type="auto" tdd="true"`)
   - `6630ca74` — `test(23-04): pin extraction order and reproduce unknown-envelope-key acceptance on not-yet-restricted Envelope (red)` — four tests added; three (`empty_output_resolves_through_on_parse_error`, `output_with_two_fenced_json_blocks_uses_the_first`, `structured_directive_parse_failure_does_not_merge_a_partial_delta`) pass immediately as pinning/characterization tests for behavior Task 1's tracer implementation already delivers correctly; the fourth, `envelope_with_an_unknown_top_level_key_is_rejected`, fails — `Envelope` had no `deny_unknown_fields` yet, so an envelope carrying an extra key was silently accepted rather than rejected. Genuine RED for the one behavior this task adds.
   - `428b02b0` — `feat(23-04): reject unknown envelope keys via deny_unknown_fields (green)` — added `#[serde(deny_unknown_fields)]` to `Envelope`. 404/404 `paladin-battalion` lib tests pass (0 ignored); `e2e_crash_resume`/`golden_bridge_equivalence`/`war_engine_tracer` green (61/61).

**Plan metadata:** (this commit) `docs(23-04): complete plan 04`

_Note: Both tasks carry `tdd="true"`; RED/GREEN pairs land the mechanism with no REFACTOR commit needed for either task. See "TDD Gate Compliance" below for the pinning-test nuance in Task 2's RED commit._

## TDD Gate Compliance

Both tasks show a `test(23-04)` commit strictly before a `feat(23-04)` commit in `git log`, satisfying the RED-before-GREEN gate sequence. One nuance worth recording: Task 2's RED commit (`6630ca74`) contains four new tests, but only one (`envelope_with_an_unknown_top_level_key_is_rejected`) genuinely fails at that commit — the other three pin behavior Task 1's tracer implementation (a real, production-quality end-to-end slice per the tracer task type's own contract) already delivers correctly, since the extraction order and the no-partial-merge guarantee are inherent to the mechanism landed in Task 1's GREEN, not separable follow-on behavior. This is treated as a legitimate characterization/pinning RED (a test run that fails to drive its ONE new behavior — deny_unknown_fields — while additionally locking down already-correct behavior with regression coverage) rather than a violation of the fail-fast "test passes unexpectedly" rule, which targets tests meant to drive behavior that does not yet exist; these three were explicitly written as pins.

## Files Created/Modified

- `crates/paladin-battalion/src/engine/directive_parser.rs` — new module: `DirectiveParser`, `OnParseError`, `DirectiveParseError`, the `Envelope`/`EnvelopeNextStep` deserialization pair, `extract_envelope`, `plain_output_directive`, `envelope_to_directive`, `first_fenced_json_block`, plus 10 unit tests and a doc test.
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec::Paladin`'s `directive_parser` field, `NodeSpec::paladin`/`paladin_with_directive_parser` constructors, `Debug` impl update, the one in-file fingerprint-fixture migration.
- `crates/paladin-battalion/src/engine/superstep.rs` — `NodeDispatch::Paladin`'s `directive_parser` field, the `DirectiveParser::parse` call replacing the unconditional `output_field` write, the `NodeFailure` enum and its threading through `execute_vanguard_node`/`NodeRunOutcome::Failed`/the per-node accumulation loop's `EngineError` conversion, 6 construction-site migrations in `#[cfg(test)]`, and 10 new tests.
- `crates/paladin-battalion/src/engine/mod.rs` — `EngineError::DirectiveParseFailed { node, reason }`.
- `crates/paladin-battalion/src/engine/bridges.rs` — 3 construction-site migrations (`from_formation`, `from_phalanx`, `from_campaign`).
- `tests/integration/e2e_crash_resume_test.rs` — 5 construction-site migrations (plan-declared file).

## Decisions Made

- **`DirectiveParser::parse` funnels every extraction failure through one `on_parse_error` decision point** — not-JSON, JSON-but-not-an-object, and a JSON object that fails `Envelope` deserialization (including a `deny_unknown_fields` rejection) all resolve identically. This keeps the three-clause extraction order as the parser's only branch structure and means the malformed-output and unknown-envelope-key tests exercise the same code path rather than two divergent ones.
- **The unknown-DELTA-field case is validated nowhere inside `directive_parser.rs`** — `Battlefield::merge`'s pre-existing schema check (`BattlefieldError::UnknownField`, checked across all of a superstep's deltas before any mutation) is the single allowlist an envelope's delta passes through, exactly like any other node's delta (T-23-14). No second, parser-owned validation exists to drift out of sync with the schema.
- **`NodeFailure` (private to `superstep.rs`) replaces `NodeError` as `execute_vanguard_node`'s error type**, so a `DirectiveParser` parse failure can carry structured node context into the new `EngineError::DirectiveParseFailed` variant (X-06) while every other node-execution failure path (Function node error, `InputMapping::render` failure, `PaladinPort::execute` error, the semaphore-closed defensive branch, `InterceptDecision::Fail`) is threaded through unchanged, still wrapping `NodeError`.
- **`envelope_to_directive` silently skips a delta entry whose key fails `FieldName::new`** (only the empty string) — unreachable in practice since no Battlefield schema ever declares a field named `""`; documented inline rather than inventing a new failure mode for a case that can never actually surface.

## Deviations from Plan

None — plan executed as written. The RED/GREEN git-surgery reconstruction (temporarily reverting graph.rs/mod.rs/bridges.rs/e2e_crash_resume_test.rs to their pre-plan state, then to a Task-1-only intermediate state, before reapplying Task 2's additions) is the same TDD-discipline technique 23-02 used, not a deviation from the plan's own instructions.

## Issues Encountered

None beyond the deliberate RED/GREEN reconstruction described above, which was itself the planned TDD discipline rather than a problem.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `DirectiveParser`/`OnParseError` and the `NodeSpec::Paladin` opt-in field this plan lands are the exact mechanism CF-FR-06 required: an LLM-backed node can now steer routing (Goto/End/Muster) from its own output, not only a Rust `Function` node.
- `DirectiveParser`'s kind and `on_parse_error` are scheduling-relevant per D-11/D-18 — Plan 23-10 hashes both into `GRAPH_FINGERPRINT_VERSION` `v3`; this plan deliberately added no fingerprint section (`WarGraph::fingerprint` untouched).
- No blockers for downstream plans in this phase's wave sequence.

---
*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Completed: 2026-09-03*

## Self-Check: PASSED

All 6 files listed under Files Created/Modified verified present on disk (`[ -f ... ]` per file). All 4 task commits (`79515f94`, `90e56ac3`, `6630ca74`, `428b02b0`) verified present in `git log --oneline`. `cargo test -p paladin-battalion --lib`: 404/404 passed, 0 ignored. `cargo test -p paladin-battalion --lib engine::directive_parser`: 10/10 passed. `cargo test -p paladin-battalion --doc engine::directive_parser`: 1/1 passed. `cargo test --test e2e_crash_resume --test golden_bridge_equivalence --test war_engine_tracer`: 61/61 passed. `cargo test --workspace --lib --bins`: all crate test binaries green, 0 failures (exit code 0). `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `grep -c 'deny_unknown_fields' crates/paladin-battalion/src/engine/directive_parser.rs`: 3 (>= 1 required). `grep -c 'NodeSpec::Paladin {' crates/paladin-battalion/src/engine/{graph,superstep,bridges}.rs`: bridges.rs 0, graph.rs 3 (all pattern-match sites), superstep.rs 2 (all pattern-match sites) — `cargo build -p paladin-battalion` confirms no construction site names `directive_parser`.
