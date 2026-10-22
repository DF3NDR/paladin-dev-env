# Deferred items (found during plan 51-13, not caused by it)

Found by `make clean-code` on the 51-13 tree (`doc-check`, step 1 of 3, exits 2; fmt, clippy `-D warnings` and `make check-gates` pass). None of these lines is touched by plan 51-13.

| Where | Finding | Introduced by |
|-------|---------|---------------|
| `crates/paladin-battalion/src/engine/cache_key.rs` (doc comment) | rustdoc `unresolved link to NODE_CACHE_KEY_VERSION` (the constant is not in scope at the doc site) | plan 51-09 |
| `crates/paladin-llm/src/reasoning.rs` lines 11 and 34 | rustdoc `redundant explicit link target` on the `ReasoningFallback` and `ReasoningEffort` links | plan 51-03 |
| `crates/paladin-llm/src/conformance.rs` line 280 | `thinking_effort_wire_value` is never used in the `paladin-llm` lib test build with its default feature set (a `dead_code` warning; it is used only when a feature that calls it is on) | plan 51-12 |

The ADR-0033 zero-warning rustdoc bar (`make doc-check`) fails until the first two are fixed; the third is a warning in the default-feature test build. Each is a one-line change in a file another plan owns.

## Resolved in plan 51-14

The three findings above were fixed in plan 51-14 (one line each) so `make clean-code` could exit 0: `NODE_CACHE_KEY_VERSION` is now a path-qualified link in `cache_key.rs`; the two redundant link targets in `reasoning.rs` are plain `[`ReasoningFallback`]` / `[`ReasoningEffort`]` links; `thinking_effort_wire_value` in `conformance.rs` is gated on the three adapter features (`kimi`, `ollama`, `openai-compatible`) whose fixtures call it.

## Found during plan 51-14 (not fixed; outside the gate)

| Where | Finding | Introduced by |
|-------|---------|---------------|
| `crates/paladin-llm/src/provider_factory.rs` line 1935 (test module `reasoning_defaults`) | `unused import: ReasoningFallback` in the `paladin-llm` lib test build with its default feature set (a plain `warning:`; the import is used when a feature that exercises it is on). `make clean-code` is unaffected because clippy and the doc build run with all features. | plan 51-06 |
