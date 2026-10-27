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

## Review items deferred by the gap-closure plans (51-15 to 51-20)

Dispositions for the 51-REVIEW.md findings that are not verification gaps (CR-01, WR-01, WR-02 and WR-03 are G-1 / G-2 and were closed in code by plans 51-15 to 51-19).

| Finding | Where | Reason | Revisit |
|---------|-------|--------|---------|
| WR-04 provider-level default effort is not in the node-cache key | `crates/paladin-battalion/src/engine/cache_key.rs`; guide "The node cache" | A documented design choice (51-09 Known Stubs, A2-51-09; the guide tells operators to rotate `key_prefix`). Folding the provider default into the key needs a port read at fingerprint time inside `paladin-battalion`'s engine (a Paladin node holds `PaladinData`, not an `LlmPort`) and a `NODE_CACHE_KEY_VERSION` decision that would invalidate every effort-bearing key: a design change outside both gaps. | A phase that revisits `NODE_CACHE_KEY_VERSION` (decide together with IN-02). |
| IN-01 duplicate self-hosted provider list | `crates/paladin-llm/src/reasoning.rs` and `crates/paladin-llm/src/config/layers.rs` | `config/layers.rs` is opened by no gap task; a two-site constant consolidation with no behaviour change. | The Phase 46 hygiene pass. |
| IN-02 cache key `unwrap_or_default()` on a serde string | `crates/paladin-battalion/src/engine/cache_key.rs` | `cache_key.rs` is opened by no gap task; cannot fail for the current shapes (verifier INFO). Switching to `Display` changes the bytes of every effort-bearing key (quoted serde string versus bare spelling), so it needs the same cache-version decision as WR-04. | With WR-04. |

IN-03 (contradictory "nothing public changed" CHANGELOG sentences in `crates/paladin-llm/CHANGELOG.md`) was folded into plan 51-20 and reworded there.

Also noted by plan 51-20: the "set a reasoning_fallback to substitute the nearest level" hint text exists in three copies (`paladin_llm::reasoning::FALLBACK_HINT`, `paladin-ports` `REASONING_FALLBACK_HINT`, and `agent_host::check_block_reasoning`). The wording is identical and each copy is pinned by a test (`the_host_fallback_hint_appears_only_when_refuse_is_in_force` for the host); sharing one constant would need a new public item, so the copies were left alone.
