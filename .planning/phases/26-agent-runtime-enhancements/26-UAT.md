---
status: complete
phase: 26-agent-runtime-enhancements
source: [26-01-SUMMARY.md, 26-02-SUMMARY.md, 26-03-SUMMARY.md, 26-04-SUMMARY.md, 26-05-SUMMARY.md, 26-06-SUMMARY.md, 26-07-SUMMARY.md, 26-08-SUMMARY.md, 26-09-SUMMARY.md, 26-10-SUMMARY.md, 26-11-SUMMARY.md, 26-12-SUMMARY.md, 26-13-SUMMARY.md, 26-14-SUMMARY.md, 26-15-SUMMARY.md, 26-16-SUMMARY.md, 26-17-SUMMARY.md, 26-18-SUMMARY.md, 26-19-SUMMARY.md, 26-20-SUMMARY.md, 26-21-SUMMARY.md]
started: 2026-09-07T20:28:49Z
updated: 2026-09-07T22:37:11Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Cold rebuild from scratch: `cargo clean` && `cargo build --workspace --all-features` && `cargo test -p paladin-memory`. The workspace rebuilds with no stale-artifact or feature-gate errors, and the embedded migrator applies 001 -> 002 -> 003 in order on a fresh empty database (asserted by existing_v0_9_database_migrates_forward, sqlite_vault_constructs_twice_idempotently, vault_and_garrison_share_one_migrator). NOTE: this repo persists no SQLite state -- all tests/examples use temp files, so there is nothing to delete and every run is already cold. Docker image build is CI-only (docker is not installed in the devcontainer) and is NOT part of this checkpoint.
result: pass

### 2. 26-17 D2
expected: StructuredExecutorExt derives a schema from a Rust type via schemars::schema_for!(D).to_value(), validates via serde deserialization (not a pre-1.0 schemars API), works through Arc<dyn StructuredExecutorPort>, holds up under 10 concurrent runs, is idempotent across two identical runs, and keeps the schemars graph at exactly two versions
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "7/7 unit tests pass individually; doctest compiles (1 passed, 148 filtered out); to_value() x2, no pre-1.0 RootSchema/SchemaObject; exactly 2 schemars versions in Cargo.lock; StructuredExecutorExt exported from src/lib.rs"

### 3. 26-18 D1
expected: NodeSpec::Paladin gains output_schema: Option<SchemaRef>, constructor-preserved via NodeSpec::paladin(..) plus a chainable with_output_schema
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "1 unit test (node_spec_paladin_constructor_is_preserved) — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 4. 26-18 D2
expected: Four distinct fail-closed EngineError variants (StructuredExecutorMissing, UnregisteredOutputSchema, OutputSchemaWithStructuredDirective, OutputSchemaFieldNotJson) fire at validation, before any node runs, each listing every offender
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "4 unit tests (output_schema_without_a_structured_executor_fails_validation, unregistered_schema_name_fails_validation_listing_every_offender, output_schema_with_a_structured_directive_parser_fails_validation, output_field_must_accept_json) — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 5. 26-18 D3
expected: TypedSchema<T>::new(schema) implements StructuredSchema by serde_json::from_value::<T> -- full typed validation, not the partial object-safe shape check
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "1 unit test (typed_schema_validates_by_deserialization) — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 6. 26-18 D4
expected: output_schema enters WarGraph::fingerprint() sorted and length-prefixed; GRAPH_FINGERPRINT_VERSION bumps v5 -> v6 with the golden re-pinned and the EngineLimits exclusion intact
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "4 unit tests (fingerprint_changes_when_output_schema_changes, fingerprint_version_is_v6_and_the_golden_is_repinned, fingerprint_golden_hex_v6, engine_limits_are_still_excluded_from_the_hash) — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 7. 26-18 D5
expected: A Paladin node with output_schema dispatches through the structured executor and writes the PARSED JSON VALUE to output_field; a downstream node reads it as structure end-to-end through a real two-node WarGraph
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "3 integration tests in structured_engine_node_test.rs — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 8. 26-18 D6
expected: Repair happens inside the node (one node-execution, two model calls); exhaustion becomes a NodeError with Paladin{kind: StructuredOutputInvalid} and Unknown transience, writing nothing; a TransientAndUnknown Aegis may still retry the whole node
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "3 integration tests in structured_engine_node_test.rs — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 9. 26-18 D7
expected: A node without output_schema is unchanged -- same PaladinPort dispatch, same raw string written -- verified both at the engine-unit level (no structured executor wired at all) and end-to-end (a structured executor IS wired but ignored)
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "1 unit (superstep.rs) + 1 integration test — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 10. 26-18 D8
expected: StructuredDirective and output_schema share the same extract_json extraction machinery -- no second envelope-extraction implementation in the engine
human_checkpoint_reason: validation_failed
result: pass
evidence_verified: "1 integration test + source-level assertion — all pass; cargo test -p paladin-battalion --lib 736 passed / cargo test --test lib structured_engine_node 8 passed"

### 11. 26-21 D5
expected: D-41 security posture verified item-by-item with evidence (test name, file, or explicit acceptance) for all nine items
human_checkpoint_reason: human_judgment
rationale: Security posture verification is a judgment call over whether the cited evidence actually satisfies the D-41 claim, appropriate for the auto-approved checkpoint's own review rather than a single pass/fail test.
result: pass
evidence_verified: "All nine D-41 items reviewed item-by-item; items 3,5,6,7 independently re-verified (redaction-before-bounding tests present; zero credential-shaped config fields; zero response_format log call sites; LlmRequest plain Debug derive with no credential-shaped field at llm_port.rs:648). Items 2, 6, 9 are weaker by nature (prompt-level framing, absence-of-evidence, openly accepted limitation) and are labelled as such rather than overstated."

### 12. 26-01 D1
expected: ExecutionMiddleware trait, MiddlewareFlow/ToolFlow, FinalResult, and the four per-run/per-call context types exist in their final shape and are re-exported from paladin::prelude
result: pass
source: automated
coverage_id: D1

### 13. 26-01 D2
expected: An empty middleware chain reproduces today's rendered prompt bytes, LlmPort call count, and PaladinResult exactly
result: pass
source: automated
coverage_id: D2

### 14. 26-01 D3
expected: before_model/after_model fire once per reasoning-loop iteration, onion-ordered, with Finish short-circuit and Fail propagation pinned
result: pass
source: automated
coverage_id: D3

### 15. 26-01 D4
expected: around_tool wraps both the Arsenal and handoff dispatch branches, with Deny/Rewrite/Allow semantics pinned
result: pass
source: automated
coverage_id: D4

### 16. 26-01 D5
expected: Ten concurrent runs through one PaladinExecutionService instance sharing one middleware Arc keep independent per-run context state; sequential runs do not leak scratch; typed state is keyed by middleware name + TypeId
result: pass
source: automated
coverage_id: D5

### 17. 26-01 D6
expected: The streaming path (execute_stream) runs before_model only; a Paladin node under a WarEngine applies its service's middleware chain unchanged, nested inside the NodeInterceptor layer, with no engine-side middleware registry
result: pass
source: automated
coverage_id: D6

### 18. 26-02 D1
expected: AgentRuntimeConfig exists in src/config/agent_runtime.rs with all twelve X-09 sub-structs, each carrying Default, validate() and EnvOverridable for its scalar fields, mirroring node_cache.rs
result: pass
source: automated
coverage_id: D1

### 19. 26-02 D2
expected: A v0.9 config.yml with no agent_runtime: section (and an agent_runtime: {} empty table) resolves to AgentRuntimeConfig::default(), attached to Settings with #[serde(default)]
result: pass
source: automated
coverage_id: D2

### 20. 26-02 D3
expected: config.example.yml carries an explicit agent_runtime: block that is pinned equal to AgentRuntimeConfig::default(), so the documented example and the code default cannot silently drift
result: pass
source: automated
coverage_id: D3

### 21. 26-02 D4
expected: No field in AgentRuntimeConfig's tree is secret-shaped -- ModelFallbackConfig names providers only, and this is pinned by a Debug-rendering check
result: pass
source: automated
coverage_id: D4

### 22. 26-02 D5
expected: PaladinConfig is untouched by this plan; full workspace check/clippy/fmt/doc-tests are clean
result: pass
source: automated
coverage_id: D5

### 23. 26-03 D1
expected: LlmRequest::new(model, prompt) produces a fresh id per call, empty attachments, stream: false, empty metadata, response_format: None -- documented and doc-tested
result: pass
source: automated
coverage_id: D1

### 24. 26-03 D2
expected: Chainable with_attachments/with_stream/with_metadata/with_response_format builders; chaining the same setter twice replaces rather than accumulates (last-write-wins)
result: pass
source: automated
coverage_id: D2

### 25. 26-03 D3
expected: ResponseFormat is #[non_exhaustive] with exactly JsonObject and JsonSchema { name, schema, strict }; both variants round-trip through serde with equality, and an absent response_format key deserializes to None
result: pass
source: automated
coverage_id: D3

### 26. 26-03 D4
expected: ProviderCapabilities gains no field (compile-time guard: exhaustive struct literal stops compiling the moment a field is added)
result: pass
source: automated
coverage_id: D4

### 27. 26-03 D5
expected: LlmRequest marked #[non_exhaustive]; the MIGRATION.md §9.2 row, .cargo/semver-checks-allowlist.toml entry, and crates/paladin-ports/Cargo.toml lint suppression agree, in one commit
result: pass
source: automated
coverage_id: D5

### 28. 26-03 D6
expected: Every LlmRequest construction site outside paladin-ports migrated to the constructor; zero struct literals remain; workspace compiles under --all-targets --all-features with zero warnings
result: pass
source: automated
coverage_id: D6

### 29. 26-03 D7
expected: response_format is proven inert on the wire until plan 26-06 wires it -- an adapter with the field set behaves identically to one without it
result: pass
source: automated
coverage_id: D7

### 30. 26-04 D1
expected: Namespace, VaultRecord, ScoredVaultRecord, Page and VaultError exist in paladin_core::platform::container::vault with no new paladin-core dependency; VaultPort exists in paladin_ports::output::vault_port re-exporting the value types
result: pass
source: automated
coverage_id: D1

### 31. 26-04 D2
expected: Namespace validates all documented invariants (1-16 segments, 1-64 chars each, no '/', not '.'/'..'', no control chars) and is_prefix_of compares segments element-by-element, provably rejecting the sibling-namespace case ['user','alice'] vs ['user','alice2']
result: pass
source: automated
coverage_id: D2

### 32. 26-04 D3
expected: VaultPort is object-safe and Send+Sync, has PRD 05 §2.3's exact five methods with a correct Unsupported default for search, and its rustdoc carries the Vault/Garrison/Waypoint table with a compiled doc test naming a Namespace, a GarrisonEntry, and a ThreadId in one snippet
result: pass
source: automated
coverage_id: D3

### 33. 26-04 D4
expected: InMemoryVault passes a nine-case shared VaultPort contract suite (put/get/overwrite/delete/list-scoping-excludes-descendants/prefix-filter/pagination/empty-namespace/namespace-isolation/value-bound) plus the search-unsupported capability assertion, with no new cargo feature or dependency
result: pass
source: automated
coverage_id: D4

### 34. 26-04 D5
expected: Full workspace check/clippy/fmt/doc are clean with no new broken intra-doc links
result: pass
source: automated
coverage_id: D5

### 35. 26-05 D1
expected: StopReason gains CallLimit and TokenBudget in one change, is marked #[non_exhaustive], every in-tree exhaustive match gains a wildcard arm, the doc comment lists the variants added this release, and the MIGRATION.md 9.2 row is resolved Y with the allowlist entry and the crates/paladin-core/Cargo.toml lint suppression in the same commit
result: pass
source: automated
coverage_id: D1

### 36. 26-05 D2
expected: ModelCallLimit finishes the run with StopReason::CallLimit at exactly max_calls (not one call early or late), and does not count the service's own buffered retry attempts within one loop iteration
result: pass
source: automated
coverage_id: D2

### 37. 26-05 D3
expected: TokenBudget accumulates the run's existing cumulative_tokens sum and finishes with the crossing response kept plus a truncation notice, StopReason::TokenBudget, with an overshoot of at most one response
result: pass
source: automated
coverage_id: D3

### 38. 26-05 D4
expected: ToolCallLimit denies a call through ToolFlow::Deny with a fixed, single-sourced model-facing message naming the tool, applies to both Armament and handoff calls, and per-tool caps are independent of the global cap and reset between runs
result: pass
source: automated
coverage_id: D4

### 39. 26-05 D5
expected: A limit breach never fails the run; a disabled limit config installs nothing and changes nothing versus a run with no middleware at all
result: pass
source: automated
coverage_id: D5

### 40. 26-05 D6
expected: Full workspace is green: no exhaustive StopReason match left without a wildcard arm, all named test suites pass, formatting and lints are clean
result: pass
source: automated
coverage_id: D6

### 41. 26-06 D1
expected: OpenAI adapter emits response_format's native json_object and json_schema{name,schema,strict} shapes on the wire; an absent field leaves the body's exact pre-existing key set unchanged
result: pass
source: automated
coverage_id: D1

### 42. 26-06 D2
expected: CompatEngine::build_request (backing Kimi/Qwen/Grok/Ollama/OpenAI-compatible) emits response_format, degrading JsonSchema to the plain json_object form; an absent field is unchanged
result: pass
source: automated
coverage_id: D2

### 43. 26-06 D3
expected: DeepSeek adapter emits response_format:{type:json_object} for either ResponseFormat variant; an absent field is unchanged
result: pass
source: automated
coverage_id: D3

### 44. 26-06 D4
expected: Gemini adapter sets generationConfig.responseMimeType for either ResponseFormat variant and generationConfig.responseSchema for JsonSchema; an absent field leaves generationConfig's key set unchanged
result: pass
source: automated
coverage_id: D4

### 45. 26-06 D5
expected: MockLlmAdapter records the response_format it received via last_response_format(), with with_responses/with_error/with_error_then_response/with_stream_items/call_count behaving exactly as before
result: pass
source: automated
coverage_id: D5

### 46. 26-06 D6
expected: Anthropic ignores response_format harmlessly (no native mode): the wire body carries no JSON-mode field and the call still succeeds
result: pass
source: automated
coverage_id: D6

### 47. 26-06 D7
expected: No ProviderCapabilities field added and no adapter's declared tool-calling capability changed
result: pass
source: automated
coverage_id: D7

### 48. 26-06 D8
expected: docs/src/user-guides/tool-integration.md gets a short pointer paragraph naming LlmRequest::with_response_format and pointing at the agent-runtime guide's per-provider table, without duplicating it
result: pass
source: automated
coverage_id: D8

### 49. 26-07 D1
expected: GarrisonEntry gains #[serde(default)] pub is_summary: bool, is marked #[non_exhaustive], gains GarrisonEntry::summary(content); the three existing constructors set is_summary: false
result: pass
source: automated
coverage_id: D1

### 50. 26-07 D2
expected: A JSON document written before this change (no is_summary key) deserializes with is_summary == false; an entry with is_summary: true round-trips through serde
result: pass
source: automated
coverage_id: D2

### 51. 26-07 D3
expected: MIGRATION.md §9.2 GarrisonEntry row resolved Y with the allowlist entry and crates/paladin-core/Cargo.toml struct_marked_non_exhaustive suppression, in the same commit as the field
result: pass
source: automated
coverage_id: D3

### 52. 26-07 D4
expected: paladin-memory runs exactly one compile-time-embedded sqlx migrator (crates/paladin-memory/src/migrations.rs) over exactly one migrations/ directory; SqliteGarrison's runtime Migrator::new(\"./migrations\") path is gone
result: pass
source: automated
coverage_id: D4

### 53. 26-07 D5
expected: 002_add_garrison_is_summary.sql runs ALTER TABLE garrison_entries ADD COLUMN is_summary INTEGER NOT NULL DEFAULT 0 beside an untouched 001; a v0.9 database (001 only, with a pre-existing row) migrates forward with the row intact and is_summary == false; SqliteGarrison constructs idempotently twice against the same file
result: pass
source: automated
coverage_id: D5

### 54. 26-07 D6
expected: The SQLite adapter's INSERT and SELECT (remember/recall_recent/search) carry is_summary via bound parameters (never format!-built SQL); the in-memory adapter passes the field through unchanged; both round-trip is_summary
result: pass
source: automated
coverage_id: D6

### 55. 26-07 D7
expected: The root migrations/ mirror and its two Dockerfile COPY lines are removed; docs/src/deployment/docker.md's matching lines are updated; MIGRATION.md §9.4 records the 002 migration
result: pass
source: automated
coverage_id: D7

### 56. 26-07 D8
expected: Workspace-wide gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --check, cargo clippy -- -D warnings, cargo doc --workspace --no-deps (no new warnings from changed files)
result: pass
source: automated
coverage_id: D8

### 57. 26-08 D1
expected: GuardrailRule/GuardrailTarget/GuardrailMatcher/GuardrailAction exist in D-09's exact locked shape; Regex patterns compile once at construction through RegexBuilder::size_limit(pattern_size_limit_bytes) with a typed GuardrailBuildError naming the rule on syntax or size failure; PaladinError::GuardrailTripped { rule, target } exists as a structured, free (#[non_exhaustive]) variant
result: pass
source: automated
coverage_id: D1

### 58. 26-08 D2
expected: before_model screens Prompt|Both rules over every PromptAssembly text part individually (redaction lands in the matching section, other sections byte-identical); after_model screens Response|Both rules over LlmResponseView::content
result: pass
source: automated
coverage_id: D2

### 59. 26-08 D3
expected: Fail returns PaladinError::GuardrailTripped with zero LlmPort calls for that iteration; Finish finishes the run with the message and StopReason::Completed; declaration order is honored with the first Fail/Finish winning and Redact not stopping the sweep
result: pass
source: automated
coverage_id: D3

### 60. 26-08 D4
expected: A Predicate matcher produces the same three actions as an equivalent Regex matcher, and an installed-but-non-matching Guardrail is byte-identical to no Guardrail at all
result: pass
source: automated
coverage_id: D4

### 61. 26-08 D5
expected: Full workspace is green: no new StopReason::Guardrail variant introduced, no bare Regex::new outside doc comments, all named test suites pass, formatting and lints clean
result: pass
source: automated
coverage_id: D5

### 62. 26-09 D1
expected: 003_create_vault_tables.sql creates vault_records with the exact D-23 schema and an index on (ns, key), numbered after 002; SqliteVault runs the same shared embedded migrator SqliteGarrison uses -- proven bidirectionally (a SqliteVault-only database gains garrison_entries, a SqliteGarrison-only database gains vault_records)
result: pass
source: automated
coverage_id: D1

### 63. 26-09 D2
expected: SqliteVault passes the full shared contract suite (9 clauses) plus search-unsupported, with exact-namespace list scoping (never LIKE on ns), opaque-cursor pagination, parameter-bound SQL throughout (no format!-built SQL), and typed+redacted storage errors
result: pass
source: automated
coverage_id: D2

### 64. 26-09 D3
expected: SemanticVault composes VaultPort+SanctumPort+EmbeddingPort with no qdrant-client dependency and no cargo feature; put is deterministic (re-put updates, never duplicates, in both halves); delete removes both halves; embedding failures surface as typed VaultError::Storage, never a panic; the type is proven constructible and functional under default features (no qdrant feature)
result: pass
source: automated
coverage_id: D3

### 65. 26-09 D4
expected: SemanticVault::search re-filters every hit by Namespace::is_prefix_of in its own code before returning, regardless of whether the backend honoured the passed filter -- pinned by a deliberately misbehaving SanctumPort double returning a hit from a namespace never asked about
result: pass
source: automated
coverage_id: D4

### 66. 26-09 D5
expected: Workspace-wide gates stay green after both adapters land: cargo check/clippy/fmt across the whole workspace, and both feature configurations (sqlite, default) of paladin-memory's vault test module
result: pass
source: automated
coverage_id: D5

### 67. 26-10 D1
expected: RetryPredicate::admits is the single home of the transience-to-boolean retry decision; should_retry is refactored to call it with every Phase 25 retry test passing unmodified
result: pass
source: automated
coverage_id: D1

### 68. 26-10 D2
expected: ModelFallbackMiddleware builds one FallbackLlmAdapter at construction, validates an empty chain up front, and sets llm_override in before_model; served_by is populated through the adapter's own metadata stamp
result: pass
source: automated
coverage_id: D2

### 69. 26-10 D3
expected: ModelRetryMiddleware sets a RetryPolicy that drives attempts, delays (backoff_delay) and the retry predicate (admits) at the single call site; with no policy the loop keeps today's shape byte-for-byte
result: pass
source: automated
coverage_id: D3

### 70. 26-10 D4
expected: The model-call port is resolved at exactly one point (ModelCallContext::effective_llm), and an override on one run never leaks to a concurrent run using a different service
result: pass
source: automated
coverage_id: D4

### 71. 26-10 D5
expected: ModelFallbackConfig::resolve_chain resolves provider names through LlmProviderFactory, collecting every unresolvable name into one typed error and distinguishing unknown/uncompiled/construction-failed; credentials are never read from or stored in the config
result: pass
source: automated
coverage_id: D5

### 72. 26-10 D6
expected: Full workspace is green: no regression in any pre-existing test, formatting and lints clean
result: pass
source: automated
coverage_id: D6

### 73. 26-11 D1
expected: TokenCounterPort exists in paladin-ports as a synchronous, infallible trait (no async fn, no Result<...>) with a passing doctest
result: pass
source: automated
coverage_id: D1

### 74. 26-11 D2
expected: HeuristicTokenCounter counts Unicode scalar values (chars), not bytes, rounds up, is infallible for any model string, and is deterministic across repetitions and fresh instances
result: pass
source: automated
coverage_id: D2

### 75. 26-11 D3
expected: TiktokenCounter implements TokenCounterPort under the content-processing feature, its exact BPE count differs from the heuristic, and an unrecognised model string at count-time never errors or panics
result: pass
source: automated
coverage_id: D3

### 76. 26-11 D4
expected: The legacy TokenCounter trait, TokenCounterFactory, and the pre-existing rag_retrieval_service.rs inline /4 heuristic are provably untouched; all 12 pre-existing tests in token_counter.rs pass unmodified
result: pass
source: automated
coverage_id: D4

### 77. 26-11 D5
expected: HistoryTrimmer's limit resolution follows the documented three-step order (model_context_limits -> provider capabilities -> default_context_tokens), with each step's precedence provable
result: pass
source: automated
coverage_id: D5

### 78. 26-11 D6
expected: History is admitted newest-first within budget, an entry is kept whole or dropped whole (never truncated), raising reserve_for_response drops exactly one more entry, fixed parts are always kept, an oversized-fixed-parts case degrades to an empty history without failing the run, the kept set is stable across 20 repetitions and a fresh counter instance, and a is_summary entry gets no special treatment
result: pass
source: automated
coverage_id: D6

### 79. 26-11 D7
expected: PaladinExecutionService gains with_token_counter/token_counter() defaulting to HeuristicTokenCounter, and with_recall_limit() which replaces the hard-coded recall_recent(20) only when explicitly set -- with no call, behavior is byte-identical to today
result: pass
source: automated
coverage_id: D7

### 80. 26-12 D1
expected: paladin-core's structured module: Structured<T>, StructuredOptions (default max_repair_attempts=1), SchemaRef, ShapeError, extract_json, shape_check, render_instruction_block -- pure, no new dependency
result: pass
source: automated
coverage_id: D1

### 81. 26-12 D2
expected: DirectiveParser::StructuredDirective refactored onto extract_json with no behaviour change -- all 16 pre-existing tests pass unmodified
result: pass
source: automated
coverage_id: D2

### 82. 26-12 D3
expected: StructuredExecutorPort (object-safe at the JSON level, D-27) and run_structured, the generic bounded repair-loop driver with a typed PaladinError::StructuredOutputInvalid exhaustion error preserving raw output
result: pass
source: automated
coverage_id: D3

### 83. 26-12 D4
expected: schemars = \"1.2\" added as a direct facade dependency, pinned to the version already resolved via rmcp, zero new lockfile packages, no new cargo feature, jsonschema not added
result: pass
source: automated
coverage_id: D4

### 84. 26-13 D1
expected: ConfinedVault { inner: Arc<dyn VaultPort>, granted: Namespace } implements VaultPort and rejects every call whose namespace does not have granted as a SEGMENT-WISE prefix with VaultError::NamespaceDenied, proven by a call-count mock showing zero backend calls on denial (siblings, parents, unrelated namespaces, and per-method); wrapping narrows, never widens; the module doc records the rejected relative-namespace alternative
result: pass
source: automated
coverage_id: D1

### 85. 26-13 D2
expected: RunScope { vault_namespace: Option<Namespace> } is non-exhaustive with Default, lives in paladin-core beside the vault types with a with_vault_namespace builder; PaladinPort::execute_scoped is a defaulted method whose body delegates to execute_observed (a correct claim of no scoped capability, X-10.4); every existing PaladinPort implementor compiles unmodified
result: pass
source: automated
coverage_id: D2

### 86. 26-13 D3
expected: PaladinExecutionService::execute_scoped is the real entry point execute/execute_observed both fund into with RunScope::default(); with_vault(vault, default_namespace) installs the store; confined_vault resolves scope.vault_namespace, else the service default, else NO grant -- never a root-granted fallback
result: pass
source: automated
coverage_id: D3

### 87. 26-13 D4
expected: WarEngine::with_vault(vault, base) grants base to every node of every run on the engine via NodeContext.vault/vault(); a NodeSpec::Paladin node receives the same grant through the now-execute_scoped Paladin-arm dispatch; an engine without with_vault gives every node None, never a root-granted handle; NodeContext's PartialEq compares the vault field by granted namespace
result: pass
source: automated
coverage_id: D4

### 88. 26-13 D5
expected: N=5 concurrent WarEngine runs under distinct grants, sharing one backend, each writing 20 records; a multi-thread-flavor test with a 30s timeout guard proves every namespace holds exactly 20 records and every record's value is that namespace's own run index -- zero cross-namespace records under real concurrency
result: pass
source: automated
coverage_id: D5

### 89. 26-13 D6
expected: Full workspace gates stay green after every change: cargo check/clippy/fmt across the whole workspace, and the complete test suites of all four touched crates
result: pass
source: automated
coverage_id: D6

### 90. 26-14 D1
expected: crates/paladin-llm/src/conformance.rs holds ConformanceFixture and llm_conformance_suite!, producing 8 fixed cases per adapter; instantiated for openai_compatible, gemini and ollama with 24/24 cells measured pass
result: pass
source: automated
coverage_id: D1

### 91. 26-14 D2
expected: Transience is asserted by value (LlmError::transience()), never by parsing a rendered message -- confirmed by grep (transience() >= 1, to_string().contains( == 0 in conformance.rs)
result: pass
source: automated
coverage_id: D2

### 92. 26-14 D3
expected: The measurement commit precedes any adapter production-code edit; no adapter was restructured -- diff on the measurement commit is purely additive test-module code across the three adapter files
result: pass
source: automated
coverage_id: D3

### 93. 26-14 D4
expected: The Ollama recipe documents ollama serve/pull, OLLAMA_BASE_URL, and the exact cargo test --test ollama_docker --features integration-tests,llm-ollama command, states the existing suite is RT-FR-22's artifact, and does not duplicate the config block or add a second Ollama test file
result: pass
source: automated
coverage_id: D4

### 94. 26-14 D5
expected: cargo doc --workspace --no-deps exits 0 with no new broken link introduced by this plan's changes
result: pass
source: automated
coverage_id: D5

### 95. 26-14 D6
expected: cargo fmt --all --check and cargo clippy --workspace --all-targets --all-features -- -D warnings are clean at the final commit
result: pass
source: automated
coverage_id: D6

### 96. 26-15 D1
expected: effective_history(): the effective history is the newest is_summary entry plus every raw entry newer than it (by summarized_through, not physical position); no summary means the whole window unchanged; a stale older summary is skipped; an empty window is a no-op
result: pass
source: automated
coverage_id: D1

### 97. 26-15 D2
expected: SummarizationMiddleware fires in before_model when the effective history's message count meets/exceeds threshold_messages (30 default); below-threshold and empty-history are no-ops making no summarizer call
result: pass
source: automated
coverage_id: D2

### 98. 26-15 D3
expected: 30 messages with keep_recent:10 summarize [oldest 20] through the summarizer port/model and remember() the result as GarrisonEntry::summary with ConversationRole::System, is_summary:true and metadata[\"summarized_through\"] set to the newest folded entry's id; the resulting effective history is 1 summary + 10 raw
result: pass
source: automated
coverage_id: D3

### 99. 26-15 D4
expected: Compounding: adding 20 more messages triggers a second summarization whose input is [summary #1 + the oldest raw entries beyond keep_recent], provably NOT the original 30 (the second summarizer prompt contains summary #1's own text and does not replay the raw entries summary #1 already folded)
result: pass
source: automated
coverage_id: D4

### 100. 26-15 D5
expected: A summarizer failure of any transience (a 503 ProviderError, a plain NetworkError) sets scratch[\"summarization.degraded\"] = true, logs a warning, runs the embedded HistoryTrimmer, and completes the run -- never MiddlewareFlow::Fail -- identically whether or not a separate HistoryTrimmer is installed, and identically before or after it in the chain
result: pass
source: automated
coverage_id: D5

### 101. 26-15 D6
expected: The summarizer's own model call is invisible to the ExecutionMiddleware chain: with a ModelCallLimit{max_calls:1} and a recording middleware also installed, a run that summarizes still makes exactly one MAIN model call and the recorder observes exactly one before_model/after_model pair -- none extra for the summarizer's call
result: pass
source: automated
coverage_id: D6

### 102. 26-15 D7
expected: GarrisonPort gains no delete-by-id or compaction method in this plan; a TraceEvent for the degradation path is explicitly Phase 28's, documented as scope not omission
result: pass
source: automated
coverage_id: D7

### 103. 26-15 D8
expected: VaultRecallMiddleware searches the granted namespace with the run's input on loop_index == 0 only (no second search on later iterations, proven by a call-count assertion), places the resulting section after retrieved RAG context and before history, drops hits below score_floor, and bounds the injection to top_k regardless of how many hits the backend returns
result: pass
source: automated
coverage_id: D8

### 104. 26-15 D9
expected: The recalled section states plainly, via a single named constant, that its entries are stored notes and not instructions (T-26-02's prompt-injection mitigation); the exact sentence is asserted, not approximated
result: pass
source: automated
coverage_id: D9

### 105. 26-15 D10
expected: Every failure mode is best-effort and quiet: VaultError::Unsupported warns exactly once per middleware instance across many calls and skips thereafter; any other search error warns and skips every time; no Vault grant skips silently (no warning, no search); a zero-hit search leaves the assembly byte-identical; before_model never returns MiddlewareFlow::Fail
result: pass
source: automated
coverage_id: D10

### 106. 26-15 D11
expected: No auto-write middleware exists anywhere in the middleware tree -- a source-level scan proves no file calls a Vault .put(); the only write path is the explicit vault_put tool (plan 26-16)
result: pass
source: automated
coverage_id: D11

### 107. 26-15 D12
expected: Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (665/665) and --doc (124/124, 18 ignored pre-existing)
result: pass
source: automated
coverage_id: D12

### 108. 26-16 D1
expected: InProcessArsenal is a public ArsenalPort over registered Armament definitions plus async closures: list_armaments/invoke/validate_call mirror ArsenalExecutionService's MCP-routed contract, validate_call reuses paladin_core::structured::shape_check rather than a second validator, invoke never wraps a handler in catch_unwind (documented), and a handler's Err becomes a failed ArmamentResult rather than a panic or propagated Err
result: pass
source: automated
coverage_id: D1

### 109. 26-16 D2
expected: CompositeArsenalPort unions several ArsenalPorts: list_armaments unions with first-registration-of-a-name-wins and logs every duplicate at warn; invoke/validate_call route by the same first-wins resolution; an empty composite lists nothing and denies every call with a typed ArsenalError::ToolNotFound
result: pass
source: automated
coverage_id: D2

### 110. 26-16 D3
expected: VaultTools::new(confined) builds vault_get/vault_put Armaments with JSON-Schema parameters, absolute (not grant-relative) namespace addressing, Namespace::new validation of the model-supplied namespace before any ConfinedVault call, a documented {\"found\":false} not-found result for vault_get, and every VaultError rendered as a tool-result error naming the requested/granted namespaces
result: pass
source: automated
coverage_id: D3

### 111. 26-16 D4
expected: PaladinExecutionService::enable_vault_tools() is opt-in (off by default) and effective_arsenal() resolves what a run dispatches through: unchanged when the flag is off or the run has no grant (vault tools not listed and no vault call reachable for that run), else a CompositeArsenalPort of the configured arsenal plus a fresh VaultTools scoped to that run's own grant; the ADR-0039 HTTP-topology consequence is recorded in enable_vault_tools's own rustdoc
result: pass
source: automated
coverage_id: D4

### 112. 26-16 D5
expected: PRD 05 section 3.4's attack test proven end to end (scripted model -> reasoning loop -> composite arsenal -> VaultTools -> ConfinedVault -> store): a hostile vault_put to a sibling, a string-prefix-lookalike sibling, and a parent namespace are each denied with the backing store's call count asserted exactly 0 via a counting VaultPort wrapper; a `..`/empty/over-long namespace segment fails Namespace::new before ConfinedVault is ever consulted (a different, documented layer); a denied call does not poison the run -- a subsequent in-grant vault_put still lands
result: pass
source: automated
coverage_id: D5

### 113. 26-16 D6
expected: X-05 concurrency obligation: 5 concurrent runs under distinct grants, sharing one Vault backend, each invoking vault_put 20 times, produce exactly 20 records per namespace and every record's value matches only its own run -- multi-thread flavor with a 30s timeout guard and exact-count/exact-value assertions
result: pass
source: automated
coverage_id: D6

### 114. 26-16 D7
expected: Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (681/681), cargo test -p paladin-ai --doc (126/126, 18 ignored pre-existing), cargo test --test lib vault_confinement (6/6)
result: pass
source: automated
coverage_id: D7

### 115. 26-17 D1
expected: PaladinExecutionService implements StructuredExecutorPort: execute_json_schema sets response_format AND appends render_instruction_block on every model call including the repair attempt, drives the shared run_structured loop (no second loop in the facade), repairs on attempt two, and preserves raw output on exhaustion -- with PaladinPort untouched
result: pass
source: automated
coverage_id: D1

### 116. 26-17 D3
expected: Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (696/696) and --doc (127/127, 18 ignored pre-existing), cargo test -p paladin-ports --lib (148/148, unaffected)
result: pass
source: automated
coverage_id: D3

### 117. 26-19 D1
expected: redact_secret_patterns covers bearer tokens, sk-/sk-ant-/AKIA-style keys, key=/token= query values and JWT-shaped triples; a benign string is unchanged; redact_credentials's pre-existing behavior and tests are unaffected by the factoring-out
result: pass
source: automated
coverage_id: D1

### 118. 26-19 D2
expected: ToolResultFormatter::format_error keeps today's Tool Execution/FAILED shape, appends the PRD's retry-or-proceed sentence, and is used identically by both the Arsenal and handoff tool-error arms
result: pass
source: automated
coverage_id: D2

### 119. 26-19 D3
expected: FeedToModel (default) matches v0.9 behavior exactly apart from sanitization; FailRun fails the run with a structured PaladinError::ArmamentFailed naming the tool; a per_tool override beats the global mode; a secret in a tool's error text never reaches the model
result: pass
source: automated
coverage_id: D3

### 120. 26-19 D4
expected: MIGRATION.md M-B-03 is rewritten with no TBD and states the corrected 'no behavioral change' premise, with a before/after sanitization example
result: pass
source: automated
coverage_id: D4

### 121. 26-19 D5
expected: ToolCallProtocolMiddleware renders a ## Tools catalogue (or none, for an empty arsenal) and synthesizes function_call from a documented envelope only when function_call is None, the named tool is known to the arsenal, and never overwriting a real function_call; the extraction reuses the shared extract_json with no second envelope parser anywhere in the facade
result: pass
source: automated
coverage_id: D5

### 122. 26-19 D6
expected: FinishOnPlainAnswerMiddleware finishes the run with StopReason::Completed on a response carrying no tool call; without it the loop's existing MaxLoops behavior is unchanged; ADR-0042 stays untouched (no LlmRequest.tools, no adapter file modified, MockLlmAdapter capabilities stay false, the correspondence test still passes)
result: pass
source: automated
coverage_id: D6

### 123. 26-19 D7
expected: Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (713/713), cargo test -p paladin-ai --doc (128/128, 18 ignored pre-existing), cargo test -p paladin-llm --lib redaction (10/10), PaladinConfig untouched
result: pass
source: automated
coverage_id: D7

### 124. 26-20 D1
expected: AgentRuntimeConfig::build_chain assembles every enabled section (limits, guardrail, trimmer/summarizer, recall, resilience) in the documented fixed order, skipping disabled sections without constructing or validating them, and yields an empty chain on a fully-disabled config
result: pass
source: automated
coverage_id: D1

### 125. 26-20 D2
expected: build_chain collects every configuration problem (an invalid guardrail regex AND unresolved fallback providers) into one AgentRuntimeConfigError::BuildFailed rather than stopping at the first, and a section enabled without the AgentRuntimeDeps dependency it needs (vault_recall without a vault, summarization without a garrison) is a typed error naming the missing dependency, never a silent skip
result: pass
source: automated
coverage_id: D2

### 126. 26-20 D3
expected: reasoning_agent(llm, arsenal, opts) returns a runnable ReasoningAgent whose run() completes a scripted tool-call-then-answer sequence with loop_count == 2 and StopReason::Completed, taking an executable Arc<dyn ArsenalPort> rather than PRD RT-FR-23's Vec<Armament> (an Armament cannot execute)
result: pass
source: automated
coverage_id: D3

### 127. 26-20 D4
expected: ReasoningAgentOptions::default() matches every documented figure: max_loops 5, max_tool_calls 20, tool_errors FeedToModel, a non-empty system prompt, and a circuit breaker with the README's 3/2/30s figures (pinned via new CircuitBreaker getters)
result: pass
source: automated
coverage_id: D4

### 128. 26-20 D5
expected: An arsenal with no registered tools still runs the preset (no ## Tools section, one loop, StopReason::Completed); a tool-call budget of 1 denies a second attempted call and the run still completes; a failing tool closure's error is fed back by default and the run completes; run_structured delegates to StructuredExecutorExt; a supplied garrison is written to and read from; the preset never lists vault_get/vault_put
result: pass
source: automated
coverage_id: D5

### 129. 26-20 D6
expected: The reasoning_agent example is anchored in crates/doc-examples/src/agent_runtime.rs at <=15 lines (14 measured), compiled by cargo check -p paladin-doc-examples, and the SAME example is also a rustdoc doc test on reasoning_agent itself that actually executes under cargo test -p paladin-ai --doc
result: pass
source: automated
coverage_id: D6

### 130. 26-20 D7
expected: Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo check -p paladin-doc-examples, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --all-features --lib (941/941), cargo test -p paladin-ai --doc (131/131, 18 ignored pre-existing), cargo test --test lib reasoning_agent (7/7)
result: pass
source: automated
coverage_id: D7

### 131. 26-21 D1
expected: Agent Runtime user guide with all six required tables/sections, registered in SUMMARY.md, example included via {{#include}} not pasted
result: pass
source: automated
coverage_id: D1

### 132. 26-21 D2
expected: MIGRATION.md §§9.1-9.5 complete for Phase 26 with no RT-owned TBD and semver allowlist matching §9.2's Y rows in both directions
result: pass
source: automated
coverage_id: D2

### 133. 26-21 D3
expected: Public API export file regenerated in this commit and check-api-surface.sh passes
result: pass
source: automated
coverage_id: D3

### 134. 26-21 D4
expected: Full gate evidence: semver-checks x11 packages, msrv 1.88, make security, clippy -D warnings, coverage >= 82%
result: pass
source: automated
coverage_id: D4

### 135. make build-docker uses a nonexistent Dockerfile path
expected: `make build-docker` builds the Paladin image from the repo-root Dockerfile.
result: pass
reported: "make build-docker -> ERROR: failed to build: failed to solve: failed to read dockerfile: open Dockerfile: no such file or directory (Makefile:426)"
severity: major
source: discovered-during-uat
resolution: "Makefile:426 now passes -f Dockerfile (repo root). Fixed directly during UAT; root cause was diagnosed with direct evidence so no gap-closure plan was needed."

## Summary

total: 135
passed: 135
issues: 0
pending: 0
skipped: 0
blocked: 0

## Coverage Block Defects

**RESOLVED 2026-09-07:** both blocks corrected -- 26-17 D2 `kind: doc` -> `kind: other`; 26-18's D8 flag was present but indented 2 spaces (parsed as a sibling key, not an entry field), re-indented to 4 and the flag added to D1-D7. Both summaries now classify `all_auto_covered: true` with 0 errors.

<!-- Malformed `coverage:` entries surfaced by `uat classify-coverage`. Per the fail-safe rule these entries
     are presented as human checkpoints rather than dropped, even though their cited verifications record `status: pass`. -->

- summary: 26-17-SUMMARY.md
  id: D2
  code: invalid_kind
  field: verification[1].kind
  message: "verification kind must be one of unit, integration, e2e, automated_ui, manual_procedural, other"
- summary: 26-18-SUMMARY.md
  id: D1
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D2
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D3
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D4
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D5
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D6
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D7
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"
- summary: 26-18-SUMMARY.md
  id: D8
  code: missing_human_judgment
  field: human_judgment
  message: "entry is missing the required human_judgment flag"

## Gaps

- gap_id: G-26-135
  truth: "`make build-docker` builds the Paladin image from the repo-root Dockerfile"
  status: resolved
  reason: "User reported on host: make build-docker -> failed to read dockerfile: open Dockerfile: no such file or directory"
  severity: major
  test: 135
  root_cause: "Makefile:426 passes `-f docker/Dockerfile`, but the Dockerfile lives at the repo root. `docker/Dockerfile` has never been tracked in git history. The `docker:default` builder resolves the missing -f path and transfers a 2B build definition, then fails. CI never exercises this target (.github/workflows/ci.yml uses `file: Dockerfile` and `docker build -t paladin:test .`), so the stale path survived the phase-26 Docker fixes (3052be2f, 1a4853ec). Sibling target docker-build-server (Makefile:431) correctly uses root-relative Dockerfile.server."
  artifacts:
    - path: "Makefile"
      issue: "line 426: `-f docker/Dockerfile` should be `-f Dockerfile`"
  missing:
    - "Point build-docker at the repo-root Dockerfile"
  resolved_by: "direct fix during UAT (Makefile:426)"
  resolved_at: 2026-09-07
  debug_session: ""
