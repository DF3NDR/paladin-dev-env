---
status: testing
phase: 38-design-seams-pricing-cost-producer
source: [38-01-SUMMARY.md, 38-02-SUMMARY.md, 38-03-SUMMARY.md, 38-04-SUMMARY.md, 38-05-SUMMARY.md, 38-06-SUMMARY.md, 38-07-SUMMARY.md, 38-08-SUMMARY.md, 38-09-SUMMARY.md]
started: 2026-09-27T15:47:12Z
updated: 2026-09-27T15:47:12Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 1
name: Cold Start Smoke Test
expected: |
  Kill any running paladin-server. Start it from scratch with a config.yml that carries a valid `treasurer:` section (currency + at least one priced model). The server boots without errors, logs no `invalid treasurer configuration` line, and a health check or basic API call returns live data.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running paladin-server. Start it from scratch with a config.yml that carries a valid `treasurer:` section (currency + at least one priced model). The server boots without errors, logs no `invalid treasurer configuration` line, and a health check or basic API call returns live data.
result: [pending]

### 2. 38-02/D1: A priced streamed agent call reaches ExecutionMetadata.cost_estimate (
expected: Run a streamed agent call against a model that has a price row (e.g. gpt-4 at 2.50/10.00 per 1M with 1,000 prompt / 2,000 completion tokens). The markdown herald's Execution Metadata block ends with `Cost: 0.0225 USD` (currency code, four decimals, no dollar sign), and ExecutionMetadata.cost_estimate is Some(0.0225). (Automated: `cargo test -p paladin-ai --lib streamed_cost_tests` covers this; confirm the rendered output looks right to you.)
result: [pending]

### 3. 38-02/D2: An unpriced model's streamed call yields cost None end-to-end (never a
expected: Run the same streamed call against a model with no price row. The herald prints no Cost line at all (never `0.0000`), cost_estimate is None, and the log carries exactly one `warn` line naming the unpriced model, no matter how many calls you make. (Automated: covered by the same test target and `pricing.rs` warn-once tests; confirm the log looks right to you.)
result: [pending]

### 4. 38-03/D7: paladin-server refuses to start with 'invalid treasurer configuration:
expected: Start paladin-server with a config.yml whose `treasurer.pricing` has a bad price (e.g. `prompt: "-1.00"` or `prompt: "abc"`). The process refuses to start and prints `invalid treasurer configuration: ...` naming `treasurer.pricing` and the offending model/axis, before any agent, engine or provider is built.

Why a human: No integration test spawns the actual paladin-server binary against a malformed config.yml and asserts process exit; the ordering and message-format claim is verified by source inspection and the unit-level HostBuildError/ProvisionError tests (D6) that share the exact same TreasurerConfig::validate() call. A human running `PALADIN_CONFIG=<bad-price-config> paladin-server` and observing the refusal would close this gap fully.
result: [pending]

### 5. 38-08/D5: RunWorkerPool::with_herald composes a HeraldTraceSink alongside build_
expected: Start paladin-server with `herald:` configured (e.g. markdown) alongside the treasurer table and run one engine (graph) run to completion. The run's finished metadata, including the currency cost, is handed to the herald and appears in the server log once per run. Then start with an unknown herald formatter name: startup fails naming the `herald` config.

Why a human: No dedicated RunWorkerPool-level integration test exercises the composed-CompositeSink path with a herald wired in -- the plan's own designed test list (herald_sink_hands_run_finished_to_the_herald, priced/unpriced_engine_run_*, run_model_label_names_single_mixed_or_none) proves the producer and its WarEngine wiring directly, not RunWorkerPool's own sink-combination logic. Verified by code review, successful compilation and cargo clippy -D warnings across the composition branch (both-None/one-Some/both-Some), and consistency with the pre-existing build_run_sink combination pattern it mirrors.
result: [pending]

### 6. 38-09/D7: Manual credential-handling review over the full Phase 38 diff: no API
expected: Skim the Phase 38 diff (`git diff 74969e15^..HEAD -- crates src`) for credential handling: no API key is logged or Debug-formatted, the pricing warn-once lines interpolate only a bare model name, and no new HTTP client was added. You should reach the same conclusion the summary records: clean.

Why a human: This is a manual source-inspection review per security.instructions.md, not something a unit test asserts -- grep-scanned the whole phase diff (git diff 8d76aa2a~1..HEAD -- crates src) for credential-shaped identifiers and reqwest client construction, then read pricing.rs's two log::warn! call sites directly to confirm only `model` (a bare model-name string) is interpolated. Findings are stated in prose below; a human reviewer re-reading the same diff would reach the same two conclusions (clean; two log call sites, both benign).
result: [pending]

### 7. 38-01/D1: ADR-0052 recorded: mid-run Treasurer enforcement attachment point across WarEngine and Pal
expected: ADR-0052 recorded: mid-run Treasurer enforcement attachment point across WarEngine and PaladinExecutionService, with the build_chain zero-production-callers fact and two rejected alternatives
result: pass
source: automated
coverage_id: 38-01/D1
verification: node .claude/gsd-core/bin/lib/adr-parser.cjs --input .planning/decisions/0052-mid-run-treasurer-enforcement.md (status: accepted, 7 headings)

### 8. 38-01/D2: ADR-0053 recorded: append-only derive-on-read ledger, row kinds, i64 nano-unit amounts, an
expected: ADR-0053 recorded: append-only derive-on-read ledger, row kinds, i64 nano-unit amounts, and settlement key/granularity resolved by the operator's checkpoint decision (superstep-aggregate)
result: pass
source: automated
coverage_id: 38-01/D2
verification: node .claude/gsd-core/bin/lib/adr-parser.cjs --input .planning/decisions/0053-ledger-balance-model.md (status: accepted, 7 headings)

### 9. 38-01/D3: PROMOTION.md numbering index carries rows 0052/0053, next-free line reads 0054, existing r
expected: PROMOTION.md numbering index carries rows 0052/0053, next-free line reads 0054, existing rows 0049-0051 unchanged
result: pass
source: automated
coverage_id: 38-01/D3
verification: grep '| 0052 |' / '| 0053 |' / 'Next free ADR number: 0054' .planning/decisions/PROMOTION.md; git diff shows no removed pre-existing rows

### 10. 38-01/D4: PRICE-02 and ROADMAP Phase 38 success criterion 2 nano-unit wording reconciled at source w
expected: PRICE-02 and ROADMAP Phase 38 success criterion 2 nano-unit wording reconciled at source with a dated amend note
result: pass
source: automated
coverage_id: 38-01/D4
verification: grep 'Amended 2026-09-25, Phase 38 plan 38-01' .planning/REQUIREMENTS.md .planning/ROADMAP.md

### 11. 38-02/D3: cost_of_call implements the D-06 formula exactly across every axis, both fallback rules, a
expected: cost_of_call implements the D-06 formula exactly across every axis, both fallback rules, and the None-sub-count rule
result: pass
source: automated
coverage_id: 38-02/D3
verification: crates/paladin-core/src/platform/container/cost.rs#tests (prompt_completion_only, cache_axes_subtract_from_base, full_cache_hit_differs_from_no_cache, omitted_cache_prices_bill_at_prompt_price, reasoning_axis_subtracts_from_base, none_sub_counts_are_zero)

### 12. 38-02/D4: Fixed-point precision: i128 intermediates, single half-up rounding per call (not per axis)
expected: Fixed-point precision: i128 intermediates, single half-up rounding per call (not per axis), sub-micro prices never round to zero
result: pass
source: automated
coverage_id: 38-02/D4
verification: crates/paladin-core/src/platform/container/cost.rs#tests (sub_micro_price_does_not_round_to_zero, half_up_rounding_boundaries, rounds_once_per_call_not_per_axis)

### 13. 38-02/D5: Boundary safety: zero-token priced calls yield Some(0) not None; malformed containment (ca
expected: Boundary safety: zero-token priced calls yield Some(0) not None; malformed containment (cache/reasoning sub-counts exceeding the parent) is clamped, never panics; u32::MAX tokens at i64::MAX prices saturate without overflow
result: pass
source: automated
coverage_id: 38-02/D5
verification: crates/paladin-core/src/platform/container/cost.rs#tests (zero_tokens_on_priced_row_is_some_zero, containment_violations_are_clamped, saturates_at_i64_max)

### 14. 38-02/D6: CostTally: empty run has no cost; priced calls sum; any unpriced call poisons the run tota
expected: CostTally: empty run has no cost; priced calls sum; any unpriced call poisons the run total permanently even if later calls are priced; a currency mismatch poisons; a neutral (default-usage, no-cost) node does not affect the tally; the sum saturates at i64::MAX
result: pass
source: automated
coverage_id: 38-02/D6
verification: crates/paladin-core/src/platform/container/cost.rs#tests::cost_tally_rules; crates/paladin-core/src/platform/container/cost.rs#tests::cost_checked_add_and_option_sum

### 15. 38-02/D7: PriceRow rejects negative axis prices naming the axis; zero is a valid (free-tier) price;
expected: PriceRow rejects negative axis prices naming the axis; zero is a valid (free-tier) price; CurrencyCode validates ISO-4217-shaped codes at construction and at deserialization; PriceTable lookup is exact and case-sensitive
result: pass
source: automated
coverage_id: 38-02/D7
verification: crates/paladin-core/src/platform/container/cost.rs#tests (price_row_rejects_negative_axis, currency_code_validation, price_table_lookup_is_exact_and_case_sensitive)

### 16. 38-03/D1: An operator can write a treasurer: section (currency + per-model pricing map with prompt/c
expected: An operator can write a treasurer: section (currency + per-model pricing map with prompt/completion required, cache_read/cache_write/reasoning optional) copied verbatim from a provider sheet, and it round-trips through Settings::load_from_file byte-identically for dotted/mixed-case model keys
result: pass
source: automated
coverage_id: 38-03/D1
verification: src/config/treasurer.rs#tests::default_treasurer_config_is_inert; src/config/treasurer.rs#tests::optional_axes_parse_and_default_to_parent; src/config/treasurer.rs#tests::model_keys_survive_loading_verbatim

### 17. 38-03/D2: Omitting the treasurer section changes nothing: Settings.treasurer == TreasurerConfig::def
expected: Omitting the treasurer section changes nothing: Settings.treasurer == TreasurerConfig::default(), validate() Ok, and config.example.yml's active treasurer: block round-trips to the same default
result: pass
source: automated
coverage_id: 38-03/D2
verification: src/config/treasurer.rs#tests::v0_10_config_resolves_treasurer_inert; src/config/treasurer.rs#tests::example_config_treasurer_block_round_trips_to_default

### 18. 38-03/D3: A negative, malformed, too-fine (>9 decimal places), or overflowing price, a bad currency,
expected: A negative, malformed, too-fine (>9 decimal places), or overflowing price, a bad currency, an empty model key, a missing required axis, or an unknown axis name is rejected at config validation with a path-precise error, never silently accepted or reinterpreted
result: pass
source: automated
coverage_id: 38-03/D3
verification: src/config/treasurer.rs#tests::validate_rejects_malformed_prices; src/config/treasurer.rs#tests::validate_rejects_too_fine_and_overflowing_prices; src/config/treasurer.rs#tests::validate_rejects_bad_currency; src/config/treasurer.rs#tests::validate_rejects_empty_model_key; src/config/treasurer.rs#tests::missing_required_axis_or_unknown_axis_fails_to_load

### 19. 38-03/D4: Exact integer decimal-string parsing: boundary values (0, 0.000000001, i64::MAX at 9223372
expected: Exact integer decimal-string parsing: boundary values (0, 0.000000001, i64::MAX at 9223372036.854775807) parse exactly; APP_TREASURER_CURRENCY overrides the file value and is validated identically to a file value
result: pass
source: automated
coverage_id: 38-03/D4
verification: src/config/treasurer.rs#tests::parses_decimal_prices_exactly; src/config/treasurer.rs#tests::env_override_currency

### 20. 38-03/D5: Both production run paths are priced whenever the table is non-empty: build_agent_registry
expected: Both production run paths are priced whenever the table is non-empty: build_agent_registry (config-loaded + runtime-provisioned agents via build_agent) and paladin_port_from_settings (run engine) each wrap the resolved LlmPort with with_pricing, proven end-to-end with a real MockLlmAdapter stream reaching the exact cost 22_500_000 nanos USD
result: pass
source: automated
coverage_id: 38-03/D5
verification: src/infrastructure/web/agent_host.rs#tests::priced_agent_stream_reports_cost

### 21. 38-03/D6: An invalid treasurer price aborts each build path with an error naming treasurer.pricing b
expected: An invalid treasurer price aborts each build path with an error naming treasurer.pricing before any provider is resolved: build_agent_registry, paladin_port_from_settings, and FacadeProvisioner::provision
result: pass
source: automated
coverage_id: 38-03/D6
verification: src/infrastructure/web/agent_host.rs#tests::build_agent_registry_rejects_invalid_treasurer_price; src/infrastructure/web/facade_provisioner.rs#tests::paladin_port_from_settings_rejects_invalid_treasurer_price; src/infrastructure/web/facade_provisioner.rs#tests::provisioner_rejects_invalid_treasurer_price

### 22. 38-04/D1: LlmResponse.cost: Option<Cost> is additive and byte-identical for legacy JSON: a document
expected: LlmResponse.cost: Option<Cost> is additive and byte-identical for legacy JSON: a document with no cost key deserializes to cost == None, and a None cost never appears as a JSON key
result: pass
source: automated
coverage_id: 38-04/D1
verification: crates/paladin-ports/src/output/llm_port.rs#tests::llm_response_without_cost_key_deserializes_to_none

### 23. 38-04/D2: Every in-tree LlmResponse struct literal (provider adapters, mock, conformance harness, se
expected: Every in-tree LlmResponse struct literal (provider adapters, mock, conformance harness, services, tests, benches, examples, doctests) compiles with cost: None in the same commit as the field
result: pass
source: automated
coverage_id: 38-04/D2
verification: cargo check --workspace --all-targets --all-features; cargo test -p paladin-ports --doc; cargo test -p paladin-llm --doc

### 24. 38-04/D3: PricingLlmAdapter::generate prices a non-streaming response from its own served model and
expected: PricingLlmAdapter::generate prices a non-streaming response from its own served model and usage: gpt-4 at 2.50/10.00 per 1M with 1,000/2,000 tokens returns Some(22,500,000 nanos USD); an unpriced model returns None with the same once-per-model warn dedup rule the stream path uses
result: pass
source: automated
coverage_id: 38-04/D3
verification: crates/paladin-llm/src/pricing.rs#tests::generate_prices_the_response_model; crates/paladin-llm/src/pricing.rs#tests::generate_unpriced_model_reports_none

### 25. 38-04/D4: Composed as Pricing(Fallback(primary, backup)), a call that hops from a failing primary to
expected: Composed as Pricing(Fallback(primary, backup)), a call that hops from a failing primary to a backup serving a differently-named model is priced against the backup's SERVED model -- or None when that model has no row -- never against the originally requested model
result: pass
source: automated
coverage_id: 38-04/D4
verification: crates/paladin-llm/src/pricing.rs#tests::prices_the_served_model_after_fallback_hop

### 26. 38-04/D5: PricingLlmAdapter's identity methods (get_provider_name, get_capabilities, validate_model,
expected: PricingLlmAdapter's identity methods (get_provider_name, get_capabilities, validate_model, get_available_models) are unchanged pass-throughs to the inner port
result: pass
source: automated
coverage_id: 38-04/D5
verification: crates/paladin-llm/src/pricing.rs#tests::pricing_is_transparent

### 27. 38-04/D6: The LlmResponse break is registered in the same commit as the field: MIGRATION.md §9.2 row
expected: The LlmResponse break is registered in the same commit as the field: MIGRATION.md §9.2 row (paladin-ports | LlmResponse, PRICE-03), a matching .cargo/semver-checks-allowlist.toml entry, and constructible_struct_adds_field = \"allow\" in crates/paladin-ports/Cargo.toml, kept set-equal by the offline gate mirror
result: pass
source: automated
coverage_id: 38-04/D6
verification: ./scripts/check-migration-allowlist.sh; make check-gates

### 28. 38-04/D7: The LlmResponse rustdoc 'Tracking Token Usage' example no longer teaches a bare-total floa
expected: The LlmResponse rustdoc 'Tracking Token Usage' example no longer teaches a bare-total floating-point cost estimate; it reads response.cost instead
result: pass
source: automated
coverage_id: 38-04/D7
verification: grep -c 'total_tokens as f64' crates/paladin-ports/src/output/llm_port.rs; cargo test -p paladin-ports --doc

### 29. 38-05/D1: JSON herald's finalize_stream emits a currency field beside cost_estimate: 0.045/USD for a
expected: JSON herald's finalize_stream emits a currency field beside cost_estimate: 0.045/USD for a priced 45,000,000-nano Cost, both null when unpriced, with every pre-existing key still present
result: pass
source: automated
coverage_id: 38-05/D1
verification: crates/paladin-herald/src/json_herald.rs#tests::finalize_stream_emits_currency_beside_cost_estimate; crates/paladin-herald/src/json_herald.rs#tests::finalize_stream_unpriced_has_null_currency; crates/paladin-herald/src/json_herald.rs#tests::test_finalize_stream

### 30. 38-05/D2: Table herald's finalize_stream renders the real ExecutionMetadata it is given (model, real
expected: Table herald's finalize_stream renders the real ExecutionMetadata it is given (model, real duration, prompt/completion tokens, reported cache-read sub-count, total) instead of the four fixed placeholder rows
result: pass
source: automated
coverage_id: 38-05/D2
verification: crates/paladin-herald/src/table_herald.rs#tests::finalize_stream_uses_real_metadata

### 31. 38-05/D3: Table herald renders a currency-coded Cost row ('0.0450 USD') via cost_display() when pric
expected: Table herald renders a currency-coded Cost row ('0.0450 USD') via cost_display() when priced, and omits the Cost row entirely when unpriced — never a dollar sign, never a fabricated zero
result: pass
source: automated
coverage_id: 38-05/D3
verification: crates/paladin-herald/src/table_herald.rs#tests::finalize_stream_renders_currency_cost_row; crates/paladin-herald/src/table_herald.rs#tests::finalize_stream_omits_cost_row_when_unpriced

### 32. 38-05/D4: Public API surface is unchanged by this plan (no new public symbol)
expected: Public API surface is unchanged by this plan (no new public symbol)
result: pass
source: automated
coverage_id: 38-05/D4
verification: PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 ./scripts/check-api-surface.sh (4013 items, unchanged)

### 33. 38-06/D1: NodeFinished.cost and RunFinished.cost are additive Option<Cost> fields with legacy-compat
expected: NodeFinished.cost and RunFinished.cost are additive Option<Cost> fields with legacy-compatible serde: a pre-phase trace record with no cost key deserializes to cost == None, and a priced RunFinished serializes one flat cost object with nanos/currency
result: pass
source: automated
coverage_id: 38-06/D1
verification: crates/paladin-core/src/platform/container/trace.rs#tests::node_and_run_finished_without_cost_key_deserialize_to_none; crates/paladin-core/src/platform/container/trace.rs#tests::priced_run_finished_serializes_cost_object

### 34. 38-06/D2: Every in-tree NodeFinished/RunFinished construction and exhaustive pattern across 18 files
expected: Every in-tree NodeFinished/RunFinished construction and exhaustive pattern across 18 files compiles with the new field, and export goldens and eval snapshots stay byte-identical (no golden/snapshot file touched)
result: pass
source: automated
coverage_id: 38-06/D2
verification: cargo check --workspace --all-targets --all-features; cargo test -p paladin-battalion --test export_golden (4/4, git status on tests/golden empty); cargo test -p paladin-eval (56+12+1 passed, git status on tests/snapshots empty)

### 35. 38-06/D3: The SSE bridge (map_trace_event) exposes no cost on the Run API's stream in this phase: a
expected: The SSE bridge (map_trace_event) exposes no cost on the Run API's stream in this phase: a priced NodeFinished/RunFinished's node_finished/done payload carries no cost key
result: pass
source: automated
coverage_id: 38-06/D3
verification: src/application/services/run/events.rs#tests::sse_payloads_carry_no_spend_field

### 36. 38-06/D4: TraceDispatcher::total_cost is the synchronous twin of total_usage: sums priced NodeFinish
expected: TraceDispatcher::total_cost is the synchronous twin of total_usage: sums priced NodeFinished events and ignores neutral (non-Paladin) ones, is None once any priced call is unpriced (never a partial sum), and is None with no sink configured
result: pass
source: automated
coverage_id: 38-06/D4
verification: crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_sums_priced_nodes_and_ignores_neutral_ones; crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_is_none_once_any_call_is_unpriced; crates/paladin-battalion/src/engine/hooks.rs#tests::total_cost_is_none_without_a_sink

### 37. 38-06/D5: All five WarEngine RunFinished emission sites set cost: trace.total_cost() beside usage: t
expected: All five WarEngine RunFinished emission sites set cost: trace.total_cost() beside usage: trace.total_usage(), so the run total reaches every trace consumer, with no regression across the whole engine test suite
result: pass
source: automated
coverage_id: 38-06/D5
verification: crates/paladin-battalion/src/engine/hooks.rs#tests::run_finished_carries_total_cost; grep -c 'cost: trace.total_cost()' crates/paladin-battalion/src/engine/mod.rs == grep -c 'usage: trace.total_usage()' (5 == 5); cargo test -p paladin-battalion --lib engine (549/549 passed)

### 38. 38-07/D1: PaladinResult.cost: Option<Cost> additive field with legacy-compatible serde (D-25 precede
expected: PaladinResult.cost: Option<Cost> additive field with legacy-compatible serde (D-25 precedent); PaladinResult::new/Default leave it None; every in-tree literal and doctest migrated
result: pass
source: automated
coverage_id: 38-07/D1
verification: crates/paladin-core/src/platform/container/execution_result.rs#tests::paladin_result_without_cost_key_deserializes_to_none; crates/paladin-core/src/platform/container/execution_result.rs#tests::default_still_constructs; cargo check --workspace --all-targets --all-features

### 39. 38-07/D2: The agent loop sums each priced model call's cost into PaladinResult.cost via a per-run Co
expected: The agent loop sums each priced model call's cost into PaladinResult.cost via a per-run CostTally; None propagates when any call was unpriced or no pricing is installed, while usage is still reported in full
result: pass
source: automated
coverage_id: 38-07/D2
verification: src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_sums_cost_across_calls; src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_cost_is_none_when_any_call_unpriced; src/application/services/paladin/paladin_execution_service.rs#agent_loop_cost_tests::agent_loop_cost_is_none_without_pricing

### 40. 38-07/D3: On the engine path each Paladin attempt's NodeFinished.cost is that attempt's own PaladinR
expected: On the engine path each Paladin attempt's NodeFinished.cost is that attempt's own PaladinResult.cost (or the structured result's raw PaladinResult.cost); non-Paladin nodes and unpriced attempts carry None; RunFinished.cost is the sum of priced Paladin nodes, or None when any is unpriced
result: pass
source: automated
coverage_id: 38-07/D3
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_finished_cost_sums_priced_paladin_nodes; crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_finished_cost_is_none_when_a_paladin_node_is_unpriced; cargo test -p paladin-battalion --lib engine (551/551 passed, no regressions)

### 41. 38-07/D4: The HTTP agent response is unchanged: ExecuteResponse built from a priced PaladinResult se
expected: The HTTP agent response is unchanged: ExecuteResponse built from a priced PaladinResult serializes with no cost key; no Run API or CLI surface starts showing cost this phase (D-11)
result: pass
source: automated
coverage_id: 38-07/D4
verification: crates/paladin-web/src/agent_controller.rs#agent_controller::tests::execute_response_carries_no_cost_field; git diff shows no line changed inside impl From<PaladinResult> for ExecuteResponse across this plan's commits

### 42. 38-08/D1: ExecutionMetadata::from_run_finished builds metadata from a RunFinished TraceRecord (execu
expected: ExecutionMetadata::from_run_finished builds metadata from a RunFinished TraceRecord (execution_id, end/start time, duration, usage, cost via the builder's single display-edge conversion, error_count), and returns None for any other event
result: pass
source: automated
coverage_id: 38-08/D1
verification: crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_builds_priced_metadata; crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_unpriced_has_no_cost; crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_ignores_other_events; crates/paladin-core/src/platform/container/herald.rs#tests::from_run_finished_tests::from_run_finished_counts_failure

### 43. 38-08/D2: HeraldTraceSink hands a RunFinished record's produced ExecutionMetadata to a Herald's fina
expected: HeraldTraceSink hands a RunFinished record's produced ExecutionMetadata to a Herald's finalize_stream, capturing the correct priced cost and currency
result: pass
source: automated
coverage_id: 38-08/D2
verification: src/infrastructure/telemetry/herald_sink.rs#tests::herald_sink_hands_run_finished_to_the_herald

### 44. 38-08/D3: End-to-end on the engine path: a WarEngine run of one Paladin node (gpt-4) whose LlmPort i
expected: End-to-end on the engine path: a WarEngine run of one Paladin node (gpt-4) whose LlmPort is priced via PricingLlmAdapter hands HeraldTraceSink an ExecutionMetadata whose cost_estimate is Some(0.0225)/USD and a real MarkdownHerald renders '0.0225 USD'; the same shape on an unpriced model yields cost_estimate None, never Some(0.0)
result: pass
source: automated
coverage_id: 38-08/D3
verification: src/infrastructure/telemetry/herald_sink.rs#tests::priced_engine_run_reaches_the_herald; src/infrastructure/telemetry/herald_sink.rs#tests::unpriced_engine_run_reports_no_cost

### 45. 38-08/D4: run_model_label names an engine run's single declared Paladin model, 'mixed' for more than
expected: run_model_label names an engine run's single declared Paladin model, 'mixed' for more than one distinct model, and 'none' for a graph with no Paladin node
result: pass
source: automated
coverage_id: 38-08/D4
verification: src/application/services/run/worker.rs#tests::run_model_label_names_single_mixed_or_none

### 46. 38-09/D1: Every one of the 11 CI packages passes cargo semver-checks against the CI-pinned v0.9.0 ba
expected: Every one of the 11 CI packages passes cargo semver-checks against the CI-pinned v0.9.0 baseline (--default-features); the loop and the migration-allowlist/check-gates set-equality gate both exit 0 on the phase's final tree
result: pass
source: automated
coverage_id: 38-09/D1
verification: cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 (all 11 CI packages, each exit 0); ./scripts/check-migration-allowlist.sh; make check-gates

### 47. 38-09/D2: The published v0.10.1 baseline (--release-type minor) measurement for every package Phase
expected: The published v0.10.1 baseline (--release-type minor) measurement for every package Phase 38 changed captures the one genuine, unsuppressed break (TraceEvent's enum_struct_variant_field_added) and confirms the two already-suppressed ones (PaladinResult, Settings), all registered in MIGRATION.md §9.2
result: pass
source: automated
coverage_id: 38-09/D2
verification: cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.10.1 --release-type minor (paladin-ai, paladin-ai-core, paladin-ports, paladin-llm, paladin-battalion, paladin-herald, paladin-web)

### 48. 38-09/D3: MIGRATION.md §9.2 rows for PaladinResult, Settings and TraceEvent::NodeFinished/RunFinishe
expected: MIGRATION.md §9.2 rows for PaladinResult, Settings and TraceEvent::NodeFinished/RunFinished carry a dated Phase 38 extension; the allowlist stays set-equal (16 crate|type pairs, unchanged) since no new deliberate-breaking entry was needed
result: pass
source: automated
coverage_id: 38-09/D3
verification: ./scripts/check-migration-allowlist.sh (16 pairs, set-equal)

### 49. 38-09/D4: CHANGELOG.md gains one [Unreleased] section (Added + new Changed) naming every Phase 38 pu
expected: CHANGELOG.md gains one [Unreleased] section (Added + new Changed) naming every Phase 38 public-facing change and pointing at MIGRATION.md §9.2; each of the five changed crates' own CHANGELOG.md [Unreleased] names its own cost/pricing addition
result: pass
source: automated
coverage_id: 38-09/D4
verification: grep -c '^## \\[Unreleased\\]' CHANGELOG.md == 1; per-crate greps for cost|pricing under each Unreleased section, all non-zero

### 50. 38-09/D5: The public-surface baseline is refreshed with the CI-pinned nightly-2026-09-20 toolchain;
expected: The public-surface baseline is refreshed with the CI-pinned nightly-2026-09-20 toolchain; TreasurerConfig and HeraldTraceSink are present; make api-surface exits 0 against the refreshed baseline
result: pass
source: automated
coverage_id: 38-09/D5
verification: PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update && make api-surface

### 51. 38-09/D6: The phase's final tree passes every commit gate CLAUDE.md names: cargo test --workspace (0
expected: The phase's final tree passes every commit gate CLAUDE.md names: cargo test --workspace (0 failures across every test result line), cargo fmt --check, make clean-code (fmt/clippy -D warnings/shell lint/check/rustdoc zero-warning bar/public-API examples gate), make security (audit + deny, 0 new advisories), make check-gates; make openapi produces no diff on crates/paladin-web/openapi.json
result: pass
source: automated
coverage_id: 38-09/D6
verification: cargo test --workspace; cargo fmt --check; make clean-code; make security; make check-gates; make openapi && git diff --exit-code crates/paladin-web/openapi.json

## Summary

total: 51
passed: 45
issues: 0
pending: 6
skipped: 0
blocked: 0

## Coverage Notes

- 45 deliverables auto-passed from the summaries' `coverage:` blocks (source: automated); 6 presented as human checkpoints (1 cold-start smoke test + 5 human-judgment entries).
- 38-02-SUMMARY.md `coverage:` block is malformed for D1 and D2 (`missing_human_judgment` flag); both are kept as human checkpoints (fail-safe) even though `cargo test -p paladin-ai --lib streamed_cost_tests` covers them automatically. Fix the SUMMARY block when convenient.

## Gaps

[none yet]
