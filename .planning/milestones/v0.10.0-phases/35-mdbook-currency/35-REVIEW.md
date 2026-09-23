---
phase: 35-mdbook-currency
reviewed: 2026-09-17T14:51:27Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - crates/doc-examples/Cargo.toml
  - crates/doc-examples/src/arsenal_tools.rs
  - crates/doc-examples/src/battalion_patterns.rs
  - crates/doc-examples/src/herald_output.rs
  - crates/doc-examples/src/lib.rs
  - crates/doc-examples/src/paladin_agents.rs
  - crates/doc-examples/src/sanctum_vector_memory.rs
  - crates/doc-examples/src/superstep_engine.rs
findings:
  critical: 1
  warning: 2
  info: 1
  total: 4
status: issues_found
---

# Phase 35: Code Review Report

**Reviewed:** 2026-09-17T14:51:27Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Reviewed the `paladin-doc-examples` crate — the compile-verified source for the mdBook
`{{#include}}` anchors touched in this phase. Anchor hygiene is clean: every `// ANCHOR: name`
in the eight files has a matching `// ANCHOR_END: name`, no orphans or mismatched names.
No hardcoded credentials, no `unsafe`, no bare `unwrap()`/`expect()`/`panic!()` in any
reviewed file (grep-confirmed), consistent with the project's "avoid panics in library code"
rule — good, since these snippets are the ones readers copy verbatim.

The one real defect worth blocking on is in `superstep_engine.rs`: the `configure_limits` →
`run_engine` example is the guide's canonical demonstration of bounding a cyclic graph's
iteration (the whole reason `LoopUntil` exists), but the custom `EngineLimits` it computes are
computed and then thrown away — `run_engine` executes a graph built with
`EngineLimits::default()`, not the configured one. A reader who copies this pattern to enforce
a production iteration cap gets silent no-enforcement. Two lower-severity findings round out
the CSV Herald example (inconsistent/incomplete comma-escaping in a formatter explicitly
presented as a full reference implementation) and one paladin_agent stub function.

## Critical Issues

### CR-01: Configured `EngineLimits` are computed then discarded before the graph runs

**File:** `crates/doc-examples/src/superstep_engine.rs:149-160`
**Issue:** The doc comment on `run_engine` (lines 142-148) explicitly promises that the run
demonstrates `EngineLimits::max_supersteps` being exhausted, and `configure_limits`'s own doc
comment (lines 110-118) states its `EngineLimits` return value is "the `EngineLimits` a
`WarGraph` is constructed with." In practice:

```rust
pub async fn run_engine()
-> Result<(RunOutcome, Arc<InMemoryWaypointStore>, ThreadId), Box<dyn std::error::Error>> {
    let graph = build_graph()?;
    let (_limits, durability) = configure_limits()?;
    ...
```

`build_graph()` (lines 76-103) constructs its `WarGraph` with a hardcoded
`EngineLimits::default()` (line 89), never taking a limits parameter. `run_engine` then calls
`configure_limits()` only to bind its `EngineLimits` half to `_limits` — an explicitly
underscore-prefixed, immediately-discarded binding — and uses only the `WaypointDurability`
half. The custom bounds shown as the headline example (`max_supersteps: 20`,
`max_node_visits: 10`) are never wired into the graph that actually executes. Because this
module exists specifically to demonstrate bounded cyclic execution (`LoopUntil` self-loops
until a condition clears), a reader who copies this pattern to cap iteration in production
will find their configured cap silently has no effect — the graph always runs under
`EngineLimits::default()` regardless of what `configure_limits()` computed. This is the kind
of "wrong API usage a reader would inherit" this review was scoped to catch: it isn't a
compile error (hence green `cargo check`), it's a semantic disconnect between the documented
intent and the actual wiring.
**Fix:** Thread the configured limits into graph construction, e.g. change `build_graph` to
accept `EngineLimits` as a parameter (or add a `build_graph_with_limits(limits: EngineLimits)`
used by `run_engine`), and use it:
```rust
pub fn build_graph(limits: EngineLimits) -> Result<WarGraph, Box<dyn std::error::Error>> {
    ...
    let mut graph = WarGraph::new(schema, limits);
    ...
}

pub async fn run_engine()
-> Result<(RunOutcome, Arc<InMemoryWaypointStore>, ThreadId), Box<dyn std::error::Error>> {
    let (limits, durability) = configure_limits()?;
    let graph = build_graph(limits)?;
    ...
}
```
If `build_graph()`'s no-argument signature is load-bearing for another anchor/page that
includes it standalone, at minimum update the `run_engine` doc comment to stop claiming the
configured limits are what bounds this particular run, and stop discarding the binding as
`_limits` (a silent discard is exactly what makes this easy to miss on read-through).

## Warnings

### WR-01: `CsvHerald` escapes commas on one field only, undermining the "reference implementation" framing

**File:** `crates/doc-examples/src/herald_output.rs:19-56`
**Issue:** `format_paladin_result` escapes commas in `result.output` (`.replace(',', ";")`,
line 23) but no other method in the same `impl Herald for CsvHerald` applies the same
treatment:
- `format_error` (line 45-47) interpolates `error` directly via `{error}` with no escaping,
  even though `PaladinError`'s `Display` messages (per this file's own imports and the
  project's `thiserror`-based error convention) commonly contain free text that can include
  commas or newlines (e.g. `"Configuration error: {0}"` wrapping arbitrary strings).
- `format_battalion_result` (line 30-32) returns `result.final_output.clone()` unescaped and
  with no trailing newline, inconsistent with every other row-producing method here.
- `finalize_stream` (line 38-43) interpolates `metadata.duration_ms` via `{:?}` unescaped.

The module doc comment frames this as "a bespoke CSV formatter implementing the full
seven-method `Herald` trait" — i.e., presented as a complete, correct reference — but a reader
who copies it as their own `Herald` implementation inherits a formatter that is only
selectively CSV-safe and will emit malformed rows the moment an error message, `stop_reason`
debug rendering, or battalion output contains a comma.
**Fix:** Either escape consistently across all methods (factor the escaping into a small
`csv_escape(s: &str) -> String` helper used everywhere a field is interpolated), or add a
one-line comment acknowledging the omission is intentionally simplified for brevity so readers
don't treat it as escaping-complete:
```rust
fn csv_escape(field: &str) -> String {
    field.replace(',', ";")
}

fn format_error(&self, error: &PaladinError) -> String {
    format!("error,{}\n", csv_escape(&error.to_string()))
}
```

### WR-02: `validate_call` checks key presence but not type, then `invoke` silently defaults on type mismatch

**File:** `crates/doc-examples/src/arsenal_tools.rs:41-66`
**Issue:** `validate_call` (lines 58-66) only checks `call.arguments.contains_key("expression")`
and returns `Ok(())` if the key exists, regardless of its JSON type. `invoke` (lines 41-56)
then reads it with `.and_then(|v| v.as_str()).unwrap_or_default()` — if `"expression"` is
present but not a JSON string (e.g. a number or object, which `validate_call` would have
accepted), `invoke` silently proceeds with an empty string rather than returning
`ArsenalError::InvalidArguments`. Per this project's own error-handling convention
("validate function arguments and return appropriate errors for invalid input"), an Arsenal
tool reference implementation should reject the malformed call rather than silently
substitute a default. A reader modeling their own `ArsenalPort` on this example inherits the
same silent-default gap.
**Fix:**
```rust
fn validate_call(&self, call: &ArmamentCall) -> Result<(), ArsenalError> {
    match call.arguments.get("expression") {
        Some(v) if v.is_string() => Ok(()),
        Some(_) => Err(ArsenalError::InvalidArguments(
            "expression must be a string".into(),
        )),
        None => Err(ArsenalError::InvalidArguments(
            "expression is required".into(),
        )),
    }
}
```

## Info

### IN-01: `CalculatorTool::invoke` never evaluates the argument it extracts

**File:** `crates/doc-examples/src/arsenal_tools.rs:41-56`
**Issue:** `let expr = ...` (lines 43-47) extracts the `expression` argument, and the inline
comment `// ... evaluate \`expr\` ...` (line 48) signals this is intentionally elided, but the
function always returns the hardcoded `output: Some(serde_json::json!(42))` regardless of
`expr`'s value. This is clearly flagged as a stub by the comment, so it is not misleading on
close reading, but a skimming reader could copy this and be surprised that "42" is returned
for every expression. Purely cosmetic given the explicit comment; no fix required beyond
awareness.
**Fix:** Optional — strengthen the comment to something like `// Stub: always returns 42;
replace with a real expression evaluator.` to remove any ambiguity for skimmers.

---

_Reviewed: 2026-09-17T14:51:27Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
