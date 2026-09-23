---
phase: 35-mdbook-currency
fixed_at: 2026-09-17T15:00:00Z
review_path: .planning/phases/35-mdbook-currency/35-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 35: Code Review Fix Report

**Fixed at:** 2026-09-17T15:00:00Z
**Source review:** .planning/phases/35-mdbook-currency/35-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (critical + warning; IN-01 excluded per fix_scope)
- Fixed: 3
- Skipped: 0

## Fixed Issues

### CR-01: Configured `EngineLimits` are computed then discarded before the graph runs

**Files modified:** `crates/doc-examples/src/superstep_engine.rs`, `docs/src/user-guides/superstep-engine.md`
**Commit:** `2265b7e6`
**Applied fix:** Changed `build_graph()` to `build_graph(limits: EngineLimits)`, threading the
parameter into `WarGraph::new(schema, limits)` instead of the hardcoded `EngineLimits::default()`.
Updated `run_engine()` to call `configure_limits()` first and pass its `EngineLimits` half into
`build_graph(limits)` (dropping the underscore-discarded `_limits` binding). Updated
`build_graph`'s doc comment to describe the new parameter and point to `configure_limits`.
Anchor names (`build_graph`, `configure_limits`, `run_engine`, `inspect_waypoints`) are
unchanged, so the guide's `{{#include}}`s still resolve. Since `build_graph` is included
standalone in the "Building a Graph" section of `superstep-engine.md`, which appears before the
"EngineConfig, EngineLimits and Bounded Iteration" section that introduces `configure_limits`,
added one clarifying sentence to that section's prose explaining where the `EngineLimits`
argument comes from (`configure_limits`, shown later on the page) or that
`EngineLimits::default()` may be passed directly. `cargo check -p paladin-doc-examples` and
`mdbook build docs/` both pass after the change.

### WR-01: `CsvHerald` escapes commas on one field only, undermining the "reference implementation" framing

**Files modified:** `crates/doc-examples/src/herald_output.rs`
**Commit:** `74efa469`
**Applied fix:** Factored escaping into a `csv_escape(field: &str) -> String` helper (documented
as intentionally minimal — commas only, not RFC 4180-complete) and applied it consistently across
every field-producing method in `impl Herald for CsvHerald`: `format_paladin_result` (output and
the `{:?}`-rendered `stop_reason`), `format_battalion_result` (previously unescaped and missing
its trailing newline — both now match the other methods), `finalize_stream` (the `{:?}`-rendered
`duration_ms`), and `format_error` (the error's `Display` text). `format_stream_chunk` is
unchanged — it returns a raw content chunk, not a CSV row. Helper and edits live inside the
existing `custom_herald` anchor, so no anchor name changed.

### WR-02: `validate_call` checks key presence but not type, then `invoke` silently defaults on type mismatch

**Files modified:** `crates/doc-examples/src/arsenal_tools.rs`
**Commit:** `6ad9a55b`
**Applied fix:** Replaced the `contains_key("expression")` boolean check with a `match` on
`call.arguments.get("expression")` that requires the value to be a JSON string
(`Some(v) if v.is_string()`), returning `ArsenalError::InvalidArguments("expression must be a
string")` on a present-but-wrong-typed value and the existing "expression is required" error on
a missing key — matching the review's suggested fix verbatim. `invoke`'s
`.and_then(|v| v.as_str()).unwrap_or_default()` extraction is unchanged; `validate_call` now
rejects the malformed-type call before `invoke` would ever see it. IN-01 (the `invoke` stub
always returning `42`) is out of scope for this pass (info-level, `fix_scope: critical+warning`)
and was left untouched.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-09-17T15:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
