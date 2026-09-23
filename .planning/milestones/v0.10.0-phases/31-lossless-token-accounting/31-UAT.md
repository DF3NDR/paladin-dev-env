---
status: complete
phase: 31-lossless-token-accounting
source: 31-01-SUMMARY.md, 31-02-SUMMARY.md, 31-03-SUMMARY.md, 31-04-SUMMARY.md, 31-05-SUMMARY.md, 31-06-SUMMARY.md, 31-07-SUMMARY.md
started: 2026-09-15T12:22:37.168Z
updated: 2026-09-15T12:44:51.412Z
---

## Current Test

[testing complete]

## Tests

### 1. Confirm auto-covered deliverables (36 items)
expected: |
  36 of the phase's 37 coverage deliverables are deterministically covered by passing tests recorded in each SUMMARY's coverage block, so they are auto-passed rather than asked one at a time. Confirm you accept the automated coverage as the evidence for these deliverables (reply yes), or name any deliverable you want to re-examine by hand.
    31-01 (ACCT-01) — 4 deliverables:
      - D1: TokenUsage carries three new Option<u32> sub-counts (cache_read_tokens, cache_write_tokens, reasoning_tokens) … [2 test refs]
      - D2: TokenUsage arithmetic (Add/AddAssign/Sum) saturates at u32::MAX, never panics, recomputes total_tokens as prom… [4 test refs]
      - D3: Legacy three-key JSON deserialises with all three optionals None… [3 test refs]
      - D4: Every in-tree full TokenUsage struct literal migrated to TokenUsage::new(..)… [4 test refs]
    31-02 (ACCT-02) — 6 deliverables:
      - D1: PaladinResult.usage, NodeExecutionRecord.usage, TraceEvent::NodeFinished.usage and TraceEvent::RunFinished.usa… [3 test refs]
      - D2: A cache-hit node records TokenUsage::default() on NodeExecutionRecord and NodeFinished and contributes nothing… [2 test refs]
      - D3: RunFinished.usage is the saturating TokenUsage sum over every NodeFinished.usage the TraceDispatcher saw this … [2 test refs]
      - D4: TokenUsage::from_total no longer exists anywhere in the tree and no #[deprecated] bare-count accessor replaced… [2 test refs]
      - D5: A PaladinResult or NodeExecutionRecord JSON document written before this phase deserialises with usage == Toke… [3 test refs]
      - D6: Full workspace compiles, tests, formats and lints clean on the new carrier chain (cargo check/test/fmt/clippy … [5 test refs]
    31-03 (ACCT-03) — 7 deliverables:
      - D1: StreamingResponse.usage: Option<TokenUsage> with #[serde(default)], #[non_exhaustive], and doc-tested delta()/… [2 test refs]
      - D2: OpenAI-family adapters (CompatEngine, openai, deepseek) hold a finish_reason frame and a separately-arriving u… [5 test refs]
      - D3: Delta text assembly order and content are unaffected by the usage contract… [2 test refs]
      - D4: OpenAI and DeepSeek map cache-read/reasoning sub-counts on BOTH the streaming and non-streaming path when the … [4 test refs]
      - D5: ChunkMetadata.usage is populated only on the is_final PaladinStreamChunk from the provider's terminal-chunk us… [1 test ref]
      - D6: When a streamed call's terminal chunk reports no usage, the final chunk's ChunkMetadata.usage is None and the … [1 test ref]
      - D7: Full workspace compiles, tests, formats and lints clean on the new streaming usage contract (cargo test/fmt/cl… [4 test refs]
    31-04 (ACCT-03) — 5 deliverables:
      - D1: Anthropic reports the same cache-inclusive TokenUsage on both generate() and generate_stream(), attaching it t… [5 test refs]
      - D2: Gemini reports the same TokenUsage on both paths including cached-content and thoughts sub-counts (completion_… [6 test refs]
      - D3: One shared conformance case (streaming_usage_equals_non_streaming_usage, CASE_COUNT 8->9) proves streaming usa… [3 test refs]
      - D4: The D-16 exception (generic OpenAiCompatibleAdapter's server-dependent streamed usage) is documented as an exp… [2 test refs]
      - D5: Full workspace compiles, lints, formats and tests clean on the completed streaming usage contract [3 test refs]
    31-05 (ACCT-04) — 4 deliverables:
      - D1: JsonHerald emits the full six-key usage object (with explicit nulls for unreported optionals) for a PaladinRes… [3 test refs]
      - D2: MarkdownHerald renders a 'Token Usage' block with Prompt/Completion/Total always and Cache read/Cache write/Re… [3 test refs]
      - D3: The CLI prints a total with its prompt/completion split (cache/reasoning appended only when reported) in human… [3 test refs]
      - D4: None of the eleven grep-hit documentation pages shows a stale bare Paladin/battalion result token count or nam… [3 test refs]
    31-06 (ACCT-05, ACCT-02) — 6 deliverables:
      - D1: TokenUsageResponse (six fields, utoipa::ToSchema, From<TokenUsage>) added in paladin-web… [3 test refs]
      - D2: crates/paladin-web/openapi.json is regenerated via make openapi in the same commit as the DTO change… [2 test refs]
      - D3: CompletedRow.usage: Option<TokenUsage> replaces the bare token_count on the run-inspector port (core type, no … [2 test refs]
      - D4: The dev-UI InspectorView page embeds the full six-key usage object for an executed row and null for a cache-se… [2 test refs]
      - D5: SSE node_finished and run_finished wire payloads (map_trace_event) carry a usage object with all six TokenUsag… [2 test refs]
      - D6: Full workspace compiles, tests, formats and lints clean on the new DTO/port/payload shapes (cargo check/test/f… [6 test refs]
    31-07 (ACCT-05) — 4 deliverables:
      - D1: Every touched public type has a MIGRATION.md §9.2 row and, where deliberate-breaking, a row-level-matched carg… [4 test refs]
      - D2: CHANGELOG.md [0.10.0] records the carrier change and both corrected under-reports with the Anthropic before/af… [2 test refs]
      - D3: api-coverage.verify-pre passes on the phase directory… [1 test ref]
      - D4: make clean-code, the full test suite (all-features, no-fail-fast), the doc-test suite, the 82% coverage floor … [7 test refs]
result: pass

### 2. Rustdoc warning debt decision (31-07 D5, ACCT-05)
expected: |
  Plan 31-07 D5 (ACCT-05) is flagged for human judgment because it is a RED gate, not a green one. Running `cargo doc --workspace --no-deps --all-features` produces pre-existing rustdoc warnings (SUMMARY reports 77; re-measured live during this UAT on 2026-09-15: 77 — battalion 36, ai-core 14, llm 9, web 8, ai 7, storage/ports/memory 1 each; symbol-by-symbol confirmed unrelated to any type or field Phase 31 touched; tracked at WINDOWS.md entry #36 and deferred-items.md). The phase deliberately did NOT absorb fixing them (20+ files across 8 unrelated subsystems). Decide: accept the 77 warnings as tracked tech debt for this phase (reply yes), or state that they must be fixed before Phase 31 is complete (which becomes a gap).
result: pass
note: "User accepted the 77 pre-existing rustdoc warnings as tracked tech debt (WINDOWS.md #36, deferred-items.md); not a Phase 31 gap"
coverage_id: 31-07/D5

### 3. 31-01 D1 (ACCT-01)
expected: TokenUsage carries three new Option<u32> sub-counts (cache_read_tokens, cache_write_tokens, reasoning_tokens) with #[serde(default)], documented inclusive-total contract, and doc-tested with_* builders
result: pass
source: automated
coverage_id: 31-01/D1
verification:
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::builders_set_optionals_and_leave_prompt_completion_total_untouched
  - cargo test -p paladin-ai-core --doc token_usage (3 builder doctests)

### 4. 31-01 D2 (ACCT-01)
expected: TokenUsage arithmetic (Add/AddAssign/Sum) saturates at u32::MAX, never panics, recomputes total_tokens as prompt+completion after every add, and follows the None/Some option-merge rule
result: pass
source: automated
coverage_id: 31-01/D2
verification:
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::add_saturates_at_u32_max_without_panicking
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::default_is_the_additive_identity
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::option_merge_some_plus_some_saturating_adds
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::sum_over_iterator_equals_sequential_add

### 5. 31-01 D3 (ACCT-01)
expected: Legacy three-key JSON deserialises with all three optionals None; a six-key document round-trips byte-identically with the optionals always emitted (as null when None)
result: pass
source: automated
coverage_id: 31-01/D3
verification:
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::legacy_json_without_optionals_deserialises_to_none
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::six_key_json_round_trips_and_serialises_all_six_keys
  - crates/paladin-core/src/platform/container/token_usage.rs#tests::none_optionals_serialise_as_null_not_omitted

### 6. 31-01 D4 (ACCT-01)
expected: Every in-tree full TokenUsage struct literal migrated to TokenUsage::new(..); the whole workspace compiles, tests, formats and lints clean on the six-field shape with no numeric fixture value changed
result: pass
source: automated
coverage_id: 31-01/D4
verification:
  - cargo check --workspace --all-targets --all-features (exit 0)
  - cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md)
  - cargo fmt --all -- --check (exit 0)
  - cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)

### 7. 31-02 D1 (ACCT-02)
expected: PaladinResult.usage, NodeExecutionRecord.usage, TraceEvent::NodeFinished.usage and TraceEvent::RunFinished.usage all carry a full TokenUsage in place of their former bare counts; BattalionResult.per_paladin_tokens values are the Paladin's real split
result: pass
source: automated
coverage_id: 31-02/D1
verification:
  - crates/paladin-battalion/src/engine/mod.rs#tests::d30_round_trip_sums_full_usage_across_two_paladin_nodes
  - crates/paladin-battalion/src/formation_service.rs#tests::test_formation_aggregates_per_paladin_times_and_tokens
  - crates/paladin-battalion/src/phalanx_service.rs#tests::test_phalanx_per_paladin_tokens

### 8. 31-02 D2 (ACCT-02)
expected: A cache-hit node records TokenUsage::default() on NodeExecutionRecord and NodeFinished and contributes nothing to RunFinished.usage; a zero-Paladin-node run finishes with RunFinished.usage == TokenUsage::default()
result: pass
source: automated
coverage_id: 31-02/D2
verification:
  - crates/paladin-battalion/src/engine/mod.rs#tests::cache_hit_paladin_node_records_default_usage_and_contributes_nothing
  - crates/paladin-battalion/src/engine/mod.rs#tests::zero_paladin_node_run_finishes_with_default_usage

### 9. 31-02 D3 (ACCT-02)
expected: RunFinished.usage is the saturating TokenUsage sum over every NodeFinished.usage the TraceDispatcher saw this run, exact the instant TraceDispatcher::emit returns
result: pass
source: automated
coverage_id: 31-02/D3
verification:
  - crates/paladin-battalion/src/engine/mod.rs#tests::d30_round_trip_sums_full_usage_across_two_paladin_nodes (field-by-field RunFinished assertion including both optionals)
  - crates/paladin-battalion/src/engine/mod.rs#tests::node_finished_carries_real_outcome_and_cost (trace/Waypoint usage agreement)

### 10. 31-02 D4 (ACCT-02)
expected: TokenUsage::from_total no longer exists anywhere in the tree and no #[deprecated] bare-count accessor replaced it; Formation and Phalanx insert result.usage.clone() into per_paladin_tokens and add u64::from(result.usage.total_tokens) to BattalionResult.total_tokens
result: pass
source: automated
coverage_id: 31-02/D4
verification:
  - grep -rn 'from_total' --include=*.rs crates src tests examples benches (0 matches)
  - grep -rn '#[deprecated' crates/paladin-core/src/platform/container/token_usage.rs crates/paladin-core/src/platform/container/execution_result.rs (0 matches)

### 11. 31-02 D5 (ACCT-02)
expected: A PaladinResult or NodeExecutionRecord JSON document written before this phase deserialises with usage == TokenUsage::default() — no legacy-shape deserializer maps the retired key into usage.total_tokens
result: pass
source: automated
coverage_id: 31-02/D5
verification:
  - crates/paladin-core/src/platform/container/execution_result.rs#tests::legacy_json_deserialises_with_served_by_none
  - crates/paladin-core/src/platform/container/waypoint.rs#tests::legacy_json_deserialises_with_default_usage
  - crates/paladin-ports/src/output/paladin_port.rs#tests::test_paladin_result_deserialization_backward_compatibility

### 12. 31-02 D6 (ACCT-02)
expected: Full workspace compiles, tests, formats and lints clean on the new carrier chain (cargo check/test/fmt/clippy --workspace --all-targets --all-features, plus make clean-code)
result: pass
source: automated
coverage_id: 31-02/D6
verification:
  - cargo check --workspace --all-targets --all-features --keep-going (exit 0)
  - cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md by plan 31-01)
  - cargo fmt --all -- --check (exit 0)
  - cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)
  - make clean-code (fmt + clippy + shellcheck + check, exit 0)

### 13. 31-03 D1 (ACCT-03)
expected: StreamingResponse.usage: Option<TokenUsage> with #[serde(default)], #[non_exhaustive], and doc-tested delta()/terminal()/with_usage() constructors replace every in-tree StreamingResponse literal outside llm_port.rs itself
result: pass
source: automated
coverage_id: 31-03/D1
verification:
  - crates/paladin-ports/src/output/llm_port.rs doctests: StreamingResponse::delta, StreamingResponse::terminal, StreamingResponse::with_usage (cargo test -p paladin-ports --doc)
  - grep -rn 'StreamingResponse {' --include=*.rs crates src tests examples -> matches only crates/paladin-ports/src/output/llm_port.rs

### 14. 31-03 D2 (ACCT-03)
expected: OpenAI-family adapters (CompatEngine, openai, deepseek) hold a finish_reason frame and a separately-arriving usage frame across SSE lines and emit both together on exactly one terminal chunk at [DONE]; a stream lacking the usage frame yields a terminal chunk with usage: None
result: pass
source: automated
coverage_id: 31-03/D2
verification:
  - crates/paladin-llm/src/compat/engine.rs#tests::generate_stream_holds_finish_reason_and_usage_for_the_one_terminal_chunk
  - crates/paladin-llm/src/compat/engine.rs#tests::generate_stream_terminal_usage_equals_the_non_streaming_usage_field_for_field
  - crates/paladin-llm/src/compat/engine.rs#tests::generate_stream_without_a_usage_frame_yields_terminal_chunk_with_usage_none
  - crates/paladin-llm/src/openai/adapter.rs#tests::streaming_usage_wiring::streaming_terminal_chunk_carries_the_same_usage_as_the_non_streaming_body
  - crates/paladin-llm/src/deepseek/adapter.rs#tests::generate_stream_holds_finish_reason_and_usage_for_the_one_terminal_chunk

### 15. 31-03 D3 (ACCT-03)
expected: Delta text assembly order and content are unaffected by the usage contract; the pre-existing openai_compatible wire-order conformance case passes unchanged
result: pass
source: automated
coverage_id: 31-03/D3
verification:
  - crates/paladin-llm/src/openai_compatible/adapter.rs conformance_suite::streaming_assembles_in_wire_order_with_a_terminal_stop (unmodified case, still passing)
  - crates/paladin-llm/src/compat/engine.rs#tests::generate_stream_holds_finish_reason_and_usage_for_the_one_terminal_chunk (assembled delta assertion)

### 16. 31-03 D4 (ACCT-03)
expected: OpenAI and DeepSeek map cache-read/reasoning sub-counts on BOTH the streaming and non-streaming path when the provider's payload carries them, and leave the figure None (never Some(0)) when the payload omits it
result: pass
source: automated
coverage_id: 31-03/D4
verification:
  - crates/paladin-llm/src/openai/adapter.rs#tests::streaming_usage_wiring::generate_maps_cache_and_reasoning_when_the_payload_carries_them
  - crates/paladin-llm/src/openai/adapter.rs#tests::streaming_usage_wiring::generate_leaves_cache_and_reasoning_none_when_the_payload_omits_them
  - crates/paladin-llm/src/deepseek/adapter.rs#tests::map_usage_maps_cache_hit_and_reasoning_when_the_payload_carries_them
  - crates/paladin-llm/src/deepseek/adapter.rs#tests::map_usage_leaves_optionals_none_when_the_payload_omits_them

### 17. 31-03 D5 (ACCT-03)
expected: ChunkMetadata.usage is populated only on the is_final PaladinStreamChunk from the provider's terminal-chunk usage; execute() and execute_stream() report the identical TokenUsage for an equivalently-configured mock call
result: pass
source: automated
coverage_id: 31-03/D5
verification:
  - src/application/services/paladin/paladin_execution_service.rs#streamed_usage_tests::streamed_final_chunk_usage_equals_the_non_streamed_result_usage

### 18. 31-03 D6 (ACCT-03)
expected: When a streamed call's terminal chunk reports no usage, the final chunk's ChunkMetadata.usage is None and the configured TokenCounterPort is never consulted -- no estimate is ever substituted for a billed figure
result: pass
source: automated
coverage_id: 31-03/D6
verification:
  - src/application/services/paladin/paladin_execution_service.rs#streamed_usage_tests::streamed_final_chunk_with_no_reported_usage_never_consults_the_token_counter

### 19. 31-03 D7 (ACCT-03)
expected: Full workspace compiles, tests, formats and lints clean on the new streaming usage contract (cargo test/fmt/clippy --workspace --all-features)
result: pass
source: automated
coverage_id: 31-03/D7
verification:
  - cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict already logged in deferred-items.md by plan 31-01)
  - cargo fmt --all -- --check (exit 0)
  - cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)
  - cargo doc -p paladin-ports -p paladin-llm --no-deps --all-features (12 warnings before and after this plan's changes -- no new warning introduced)

### 20. 31-04 D1 (ACCT-03)
expected: Anthropic reports the same cache-inclusive TokenUsage on both generate() and generate_stream(), attaching it to the message_stop terminal chunk; a new non-zero-cache fixture proves the D-20 prompt_tokens correction actually fires (none of the three pre-existing fixtures could catch it, since all report explicit zeros)
result: pass
source: automated
coverage_id: 31-04/D1
verification:
  - crates/paladin-llm/src/anthropic/adapter.rs#tests::test_cache_inclusive_prompt_tokens_correction_fires_on_non_zero_cache_fixture
  - crates/paladin-llm/src/anthropic/adapter.rs#tests::test_thinking_text_maps_reasoning_and_reports_explicit_zero_cache_figures
  - crates/paladin-llm/src/anthropic/adapter.rs#tests::streaming_usage_wiring::message_stop_is_the_only_usage_bearing_chunk_and_equals_the_non_streaming_usage
  - crates/paladin-llm/src/anthropic/adapter.rs#tests::streaming_usage_wiring::message_stop_carries_usage_none_when_no_usage_payload_ever_arrives
  - crates/paladin-llm/src/anthropic/adapter.rs#tests -- all three pre-existing captured-fixture tests still pass with their original expected values

### 21. 31-04 D2 (ACCT-03)
expected: Gemini reports the same TokenUsage on both paths including cached-content and thoughts sub-counts (completion_tokens = candidates + thoughts, D-02), attached to the last (finish-reason-bearing) streaming frame, with the Default derive intact so a response with no usageMetadata key still parses
result: pass
source: automated
coverage_id: 31-04/D2
verification:
  - crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_maps_cached_content_and_thoughts_into_completion_and_optionals
  - crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_leaves_cache_and_reasoning_none_when_the_payload_omits_them
  - crates/paladin-llm/src/gemini/adapter.rs#tests::parse_response_with_no_usage_metadata_key_still_parses_via_default
  - crates/paladin-llm/src/gemini/adapter.rs#tests::generate_stream_last_frame_usage_equals_the_non_streaming_usage
  - crates/paladin-llm/src/gemini/adapter.rs#tests::generate_stream_without_usage_metadata_yields_terminal_chunk_with_usage_none
  - crates/paladin-llm/src/gemini/adapter.rs#tests::conformance_suite::streaming_assembles_in_wire_order_with_a_terminal_stop -- pre-existing case still passes unchanged

### 22. 31-04 D3 (ACCT-03)
expected: One shared conformance case (streaming_usage_equals_non_streaming_usage, CASE_COUNT 8->9) proves streaming usage equals non-streaming usage for every adapter with a real streaming parser; instantiated for openai/deepseek/grok/kimi/qwen (new) plus gemini/ollama/openai_compatible (extended); Anthropic and the mock adapter each get a dedicated stand-in test with a rustdoc explaining why
result: pass
source: automated
coverage_id: 31-04/D3
verification:
  - crates/paladin-llm/src/conformance.rs#tests::suite_generates_the_full_case_list_for_a_fixture (pins CASE_COUNT == 9)
  - cargo test -p paladin-llm --lib conformance (82 tests, includes the new case for every instantiated fixture)
  - crates/paladin-llm/src/mock.rs#tests::streaming_usage_equals_non_streaming_usage_for_the_default_stream_path

### 23. 31-04 D4 (ACCT-03)
expected: The D-16 exception (generic OpenAiCompatibleAdapter's server-dependent streamed usage) is documented as an explicit exception in the adapter's own rustdoc AND in the mdBook provider feature matrix, using exactly one of three permitted values across every listed provider, never a fourth 'partial' value
result: pass
source: automated
coverage_id: 31-04/D4
verification:
  - grep -c 'server-dependent' docs/src/appendix/provider-expansion.md == 1; grep -ci 'partial' docs/src/appendix/provider-expansion.md == 0; grep -c 'Streamed usage' docs/src/appendix/provider-expansion.md == 1
  - mdbook build docs/ exits 0 with the book's error-level warning policy unchanged

### 24. 31-04 D5 (ACCT-03)
expected: Full workspace compiles, lints, formats and tests clean on the completed streaming usage contract
result: pass
source: automated
coverage_id: 31-04/D5
verification:
  - cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict already logged by plan 31-01 in deferred-items.md)
  - cargo fmt --all -- --check (exit 0)
  - cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)

### 25. 31-05 D1 (ACCT-04)
expected: JsonHerald emits the full six-key usage object (with explicit nulls for unreported optionals) for a PaladinResult and for ExecutionMetadata; per_paladin_tokens entries carry real non-zero splits; the object round-trips into an equal TokenUsage
result: pass
source: automated
coverage_id: 31-05/D1
verification:
  - crates/paladin-herald/src/json_herald.rs#tests::test_usage_object_key_set_is_stable_with_nulls_when_optionals_unreported
  - crates/paladin-herald/src/json_herald.rs#tests::test_per_paladin_tokens_carry_real_non_zero_splits
  - crates/paladin-herald/src/json_herald.rs#tests::test_usage_object_deserializes_back_into_equal_token_usage

### 26. 31-05 D2 (ACCT-04)
expected: MarkdownHerald renders a 'Token Usage' block with Prompt/Completion/Total always and Cache read/Cache write/Reasoning only when Some, plus a per-Paladin usage table under a BattalionResult's total-tokens summary
result: pass
source: automated
coverage_id: 31-05/D2
verification:
  - crates/paladin-herald/src/markdown_herald.rs#tests::test_token_usage_block_omits_unreported_optionals
  - crates/paladin-herald/src/markdown_herald.rs#tests::test_token_usage_block_renders_optionals_when_reported
  - crates/paladin-herald/src/markdown_herald.rs#tests::test_per_paladin_usage_table_renders_real_non_zero_splits

### 27. 31-05 D3 (ACCT-04)
expected: The CLI prints a total with its prompt/completion split (cache/reasoning appended only when reported) in human-readable output, and its JSON mode emits the usage object rather than a bare number
result: pass
source: automated
coverage_id: 31-05/D3
verification:
  - src/application/cli/formatters/output.rs#tests::test_format_paladin_result_human_output_shows_split
  - src/application/cli/formatters/output.rs#tests::test_format_paladin_result_json_emits_usage_object_not_scalar
  - src/application/cli/formatters/output.rs#tests::test_format_battalion_result_json_emits_usage_object_per_paladin

### 28. 31-05 D4 (ACCT-04)
expected: None of the eleven grep-hit documentation pages shows a stale bare Paladin/battalion result token count or names a token-carrier field that does not exist on the post-phase result types; doctests, the mdBook build and cargo doc all run clean modulo pre-existing out-of-scope warnings
result: pass
source: automated
coverage_id: 31-05/D4
verification:
  - grep -rn 'token_count' docs/src --include='*.md' | grep -v memory-management.md | grep -v domain-model.md | grep -v api-reference/ (0 matches)
  - grep -n 'token_usage: TokenUsage' docs/src/user-guides/battalion-patterns.md (0 matches)
  - cargo test --workspace --doc --all-features (exit 0); mdbook build docs/ (exit 0)

### 29. 31-06 D1 (ACCT-05)
expected: TokenUsageResponse (six fields, utoipa::ToSchema, From<TokenUsage>) added in paladin-web; ExecuteResponse.usage replaces the bare token_count field, converted through the one From<PaladinResult> conversion point; paladin-core gains no utoipa dependency
result: pass
source: automated
coverage_id: 31-06/D1
verification:
  - crates/paladin-web/src/agent_controller.rs#tests::token_usage_response_from_token_usage_maps_all_six_fields_unchanged
  - crates/paladin-web/src/agent_controller.rs#tests::execute_response_serializes_usage_object_with_six_keys
  - grep -c 'utoipa' crates/paladin-core/Cargo.toml (0 matches)

### 30. 31-06 D2 (ACCT-05)
expected: crates/paladin-web/openapi.json is regenerated via make openapi in the same commit as the DTO change; the committed-baseline test passes and git diff --exit-code is clean
result: pass
source: automated
coverage_id: 31-06/D2
verification:
  - crates/paladin-web/src/openapi.rs#tests::openapi_matches_committed_baseline
  - git diff --exit-code crates/paladin-web/openapi.json (after make openapi, exit 0)

### 31. 31-06 D3 (ACCT-02)
expected: CompletedRow.usage: Option<TokenUsage> replaces the bare token_count on the run-inspector port (core type, no paladin-web dependency introduced into paladin-ports); None on a cache-hit, Some with a real prompt+completion split on an executed attempt
result: pass
source: automated
coverage_id: 31-06/D3
verification:
  - src/application/services/run/inspector.rs#tests::completed_row_partial_values_are_representable
  - grep -c 'paladin-web' crates/paladin-ports/Cargo.toml (0 matches)

### 32. 31-06 D4 (ACCT-02)
expected: The dev-UI InspectorView page embeds the full six-key usage object for an executed row and null for a cache-served one, in the actual rendered page JSON
result: pass
source: automated
coverage_id: 31-06/D4
verification:
  - crates/paladin-web/src/dev_ui_controller.rs#tests::dev_ui_page_embeds_cache_hit_row_with_no_duration_or_tokens
  - crates/paladin-web/src/dev_ui_controller.rs#tests::dev_ui_page_embeds_executed_row_with_full_usage_object

### 33. 31-06 D5 (ACCT-02)
expected: SSE node_finished and run_finished wire payloads (map_trace_event) carry a usage object with all six TokenUsage keys, proven against the actual serialized JSON
result: pass
source: automated
coverage_id: 31-06/D5
verification:
  - src/application/services/run/events.rs#tests::node_finished_payload_carries_six_key_usage_object
  - src/application/services/run/events.rs#tests::run_finished_payload_carries_six_key_usage_object

### 34. 31-06 D6 (ACCT-05)
expected: Full workspace compiles, tests, formats and lints clean on the new DTO/port/payload shapes (cargo check/test/fmt/clippy --workspace --all-targets --all-features, plus make clean-code); the pre-existing SHIP-02 golden-diff gate stays green via one narrowly-scoped, proven-narrow exception
result: pass
source: automated
coverage_id: 31-06/D6
verification:
  - cargo check --workspace --all-targets --all-features (exit 0)
  - cargo test --workspace --all-features --no-fail-fast (all green except the pre-existing, unrelated cli_isolation/--all-features conflict logged in deferred-items.md by plan 31-01)
  - cargo fmt --all -- --check (exit 0)
  - cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)
  - make clean-code (fmt + clippy + shellcheck + check, exit 0)
  - crates/paladin-web/tests/openapi_golden_v0_9.rs (all 7 tests pass, including new execute_response_exception_is_narrowly_scoped)

### 35. 31-07 D1 (ACCT-05)
expected: Every touched public type has a MIGRATION.md §9.2 row and, where deliberate-breaking, a row-level-matched cargo semver-checks allowlist entry whose lint id was derived empirically; the row-level set-equality gate passes locally in both directions
result: pass
source: automated
coverage_id: 31-07/D1
verification:
  - cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 (exit 0)
  - cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 (exit 0)
  - cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0 (exit 0)
  - local reproduction of ci.yml's awk-based allowlist <-> §9.2 set-equality comparison, both directions empty diff

### 36. 31-07 D2 (ACCT-05)
expected: CHANGELOG.md [0.10.0] records the carrier change and both corrected under-reports with the Anthropic before/after formula; [Unreleased] untouched; no v0.11.0 string anywhere
result: pass
source: automated
coverage_id: 31-07/D2
verification:
  - grep -c 'PaladinResult\\|StreamingResponse\\|ExecuteResponse' within CHANGELOG.md [0.10.0] section == 6
  - grep -rn 'v0\\.11\\.0' CHANGELOG.md docs/src/api-reference (0 matches)

### 37. 31-07 D3 (ACCT-05)
expected: api-coverage.verify-pre passes on the phase directory; COVERAGE.md retains all 53 capability rows, every OPT-OUT reason non-empty and under 200 chars, with the one reconciled decision (grok.reasoning) documented
result: pass
source: automated
coverage_id: 31-07/D3
verification:
  - gsd-tools query check api-coverage.verify-pre .planning/phases/31-lossless-token-accounting -> passed: true, 53 capabilities, 12 opt-out

### 38. 31-07 D4 (ACCT-05)
expected: make clean-code, the full test suite (all-features, no-fail-fast), the doc-test suite, the 82% coverage floor (both direct and via make coverage), make security, mdbook build, make openapi diff-clean, and the manual credential-handling review are all green or explicitly, honestly recorded
result: pass
source: automated
coverage_id: 31-07/D4
verification:
  - make clean-code (exit 0)
  - cargo test --workspace --all-features --no-fail-fast: 6760 passed, 1 failed (pre-existing test_cli_feature_is_not_default, documented in deferred-items.md by plan 31-01)
  - cargo test --workspace --doc --all-features: 485 passed, 0 failed
  - cargo llvm-cov --workspace --fail-under-lines 82: exit 0, 90.30% lines; make coverage (--features integration-tests,llm-all): exit 0, 90.17% lines (cargo llvm-cov report --summary-only) -- both >> 82% floor, folded todo closed
  - make security: exit 0 (advisories ok, bans ok, licenses ok, sources ok; 10 pre-allowlisted warnings, 0 new)
  - mdbook build docs/: exit 0, no broken links
  - make openapi && git diff --exit-code crates/paladin-web/openapi.json: exit 0, clean

## Summary

total: 38
passed: 38
issues: 0
pending: 0
skipped: 0
blocked: 0
automated: 36

## Gaps

[none yet]
