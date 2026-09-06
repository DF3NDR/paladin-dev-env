---
status: complete
phase: 25-node-level-fault-tolerance
source: 25-01-SUMMARY.md, 25-02-SUMMARY.md, 25-03-SUMMARY.md, 25-04-SUMMARY.md, 25-05-SUMMARY.md, 25-06-SUMMARY.md, 25-07-SUMMARY.md, 25-08-SUMMARY.md, 25-09-SUMMARY.md, 25-10-SUMMARY.md, 25-11-SUMMARY.md, 25-12-SUMMARY.md, 25-13-SUMMARY.md, 25-14-SUMMARY.md
started: 2026-09-06T14:30:23Z
updated: 2026-09-06T19:41:12Z
---

## Current Test

[testing complete]

## Tests

### 1. Redis node cache runs the contract suite against a live Redis server
expected: The redis-cache-integration CI job (added by plan 25-04) is green on a phase-25 commit: the three live-server tests (redis_node_cache_runs_the_full_contract_suite, redis_keys_are_namespaced_by_the_configured_prefix, redis_ttl_is_set_on_the_server_not_only_in_the_payload) execute against a real Redis rather than self-skipping, and the job's SKIP-detection assertion passes. Branch is 120 commits ahead of origin so this job has not yet run on phase-25 code.
result: issue
reported: "THe last time a `ci.yml` action ran on github it failed: https://github.com/DF3NDR/paladin-dev-env/actions/runs/33965717959 however this was on API Surface Tracking and was part of phase 24 not phase 25.  We have now pushed the latest commit up and it too is showing the same problem while it is still running: https://github.com/DF3NDR/paladin-dev-env/actions/runs/34042790005/job/101512438612  This needs to be fixed before we can proceed."
severity: blocker
note: run 34042790005 on 332614d4 -- Redis Node Cache Contract Suite (live server) job 101512438673 completed/success; redis_node_cache_runs_the_full_contract_suite, redis_keys_are_namespaced_by_the_configured_prefix, redis_ttl_is_set_on_the_server_not_only_in_the_payload all ok, 9 passed, SKIP-detection step passed
coverage_id: 25-04/D4
requirement: FT-06
reason_for_human: human_judgment — Docker/Redis absent in the devcontainer; live-server tier is CI-only and has never executed here

### 2. Shared HTTP status mapper classifies every non-2xx response by typed status
expected: cargo test -p paladin-llm --all-features http_status passes all six tests (an unmapped status becomes ProviderError carrying the status as a typed u16; 401/429/402/404/400 keep their dedicated variants; an unknown 4xx is ProviderError not ProcessingError; the body excerpt is redacted before it is bounded; a multibyte body is bounded on char boundaries; an empty body still classifies) and cargo test --doc -p paladin-llm --all-features passes its 6 doc tests.
result: pass
coverage_id: 25-05/D1
requirement: FT-01
reason_for_human: validation_failed — 25-05-SUMMARY.md coverage block uses kind: doc on verification[1], which is not a recognised kind (unit, integration, e2e, automated_ui, manual_procedural, other); the entry is presented rather than dropped

### 3. Waypoint attempt history and Failed node_error round-trip on Postgres
expected: The postgres-integration CI job runs waypoint_with_attempt_history_round_trips and failed_waypoint_with_node_error_round_trips from crates/paladin-storage/src/waypoint/postgres.rs against a real Postgres and both pass. The InMemory and SQLite tiers already pass locally via cargo test -p paladin-storage --features sqlite,postgres --lib. As with test 1, no phase-25 commit has been pushed, so this job has not yet run on this code.
result: pass
note: run 34042790005 on 332614d4 -- Postgres Waypoint Contract Suite (live server) job 101512438665 completed/success; waypoint_with_attempt_history_round_trips and failed_waypoint_with_node_error_round_trips both ok, 37 passed, SKIP-detection step passed
coverage_id: 25-07/D3
requirement: FT-02
reason_for_human: verification_not_passing — Postgres tier is Docker-gated, CI-only, status unknown locally

### 4. Every X-10/X-11 gate is green on the phase's final commit
expected: On the phase's final commit semver (11/11 packages against 0.9.0), MSRV 1.88, make security, clippy -D warnings, cargo fmt --check and cargo llvm-cov --fail-under-lines 82 (recorded 89.34%) all exit 0 as the Gate Evidence table in 25-14-SUMMARY.md records; the manual credential-handling review is recorded; R-23-01 is re-listed as accepted; and the Redis/Postgres CI tiers are green. Context: the gate figures were measured locally on 462a1442 and 13 commits landed after it (five code-review fixes, each re-running fmt, clippy -D warnings and the affected crates' tests per 25-REVIEW-FIX.md, but not semver, MSRV, make security or llvm-cov); the last CI run on this branch (pre-phase 4498210b) failed its API Surface Tracking job; no phase-25 commit has run in CI.
result: pass
note: run 34051074633 on 0e5c106c (push, 2026-09-06T18:14Z) concluded success -- 31 jobs green including API Surface Tracking, Semver Checks (vs v0.9.0), MSRV (Rust 1.88), Coverage, Security Audit, License & Dependency Policy, Redis and Postgres live-server suites; 0 failures; 3 conditional jobs skipped. Verdict delegated to the CI watch by the user.
coverage_id: 25-14/D4
requirement: FT-06
reason_for_human: verification_not_passing — Redis/Postgres CI tiers carry status unknown; gate figures are local, not CI

### 5. 25-01 D1: A node whose execution fails with a retry-eligible error is re-executed inside the same superstep and the run reaches RunOutcome::Completed
expected: A node whose execution fails with a retry-eligible error is re-executed inside the same superstep and the run reaches RunOutcome::Completed
result: pass
source: automated
coverage_id: 25-01/D1
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::transient_function_node_failure_is_retried_and_run_completes

### 6. 25-01 D2: A failed attempt's delta never reaches the merged Battlefield; each attempt observes an identical snapshot
expected: A failed attempt's delta never reaches the merged Battlefield; each attempt observes an identical snapshot
result: pass
source: automated
coverage_id: 25-01/D2
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::failed_attempt_delta_never_reaches_the_battlefield ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::each_attempt_reads_an_identical_battlefield_snapshot

### 7. 25-01 D3: Interceptors run once per attempt (not once per node); Skip/Fail decisions are never retried as if they were the node's own transient fault
expected: Interceptors run once per attempt (not once per node); Skip/Fail decisions are never retried as if they were the node's own transient fault
result: pass
source: automated
coverage_id: 25-01/D3
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptors_run_once_per_attempt_not_once_per_node ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptor_fail_decision_is_not_retried ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptor_skip_decision_produces_exactly_one_attempt

### 8. 25-01 D4: A node with no Aegis behaves byte-identically to pre-phase-25 behavior; a node's own Aegis wins wholesale over the graph's default_aegis
expected: A node with no Aegis behaves byte-identically to pre-phase-25 behavior; a node's own Aegis wins wholesale over the graph's default_aegis
result: pass
source: automated
coverage_id: 25-01/D4
requirement: FT-01
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::node_without_aegis_behaves_exactly_as_before ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::set_aegis_per_node_wins_wholesale_over_default_aegis

### 9. 25-01 D5: NodeError serialises with a stable, declaration-ordered field layout (node_id, attempt, transience, source) and round-trips through serde
expected: NodeError serialises with a stable, declaration-ordered field layout (node_id, attempt, transience, source) and round-trips through serde
result: pass
source: automated
coverage_id: 25-01/D5
requirement: FT-01
verification: crates/paladin-core/src/platform/container/node_error.rs#tests::node_error_round_trips_through_serde_with_stable_field_order

### 10. 25-01 D6: The exact backoff sequence (500/1000/2000/4000ms), the max_interval cap, jitter bounds, three-way predicate gating, max_attempts==0 typed rejection, and cancellation-aware waiting are pinned under a paused clock
expected: The exact backoff sequence (500/1000/2000/4000ms), the max_interval cap, jitter bounds, three-way predicate gating, max_attempts==0 typed rejection, and cancellation-aware waiting are pinned under a paused clock
result: pass
source: automated
coverage_id: 25-01/D6
requirement: FT-02
verification: crates/paladin-battalion/src/engine/retry.rs#tests::backoff_sequence_is_exact_with_jitter_off ; crates/paladin-battalion/src/engine/retry.rs#tests::backoff_is_capped_at_max_interval ; crates/paladin-battalion/src/engine/retry.rs#tests::backoff_with_jitter_stays_within_bounds ; crates/paladin-battalion/src/engine/retry.rs#tests::permanent_error_under_transient_only_takes_one_attempt ; crates/paladin-battalion/src/engine/retry.rs#tests::transient_error_is_retried_and_unknown_is_gated_by_the_predicate ; crates/paladin-battalion/src/engine/retry.rs#tests::max_attempts_zero_is_a_typed_validation_error ; crates/paladin-battalion/src/engine/retry.rs#tests::backoff_wait_returns_early_when_the_run_is_cancelled

### 11. 25-02 D1: PaladinError::transience() and LlmError::transience() classify every variant from typed fields only, with a table-driven test asserting one row per variant (landed by Task 1, 2a70f579)
expected: PaladinError::transience() and LlmError::transience() classify every variant from typed fields only, with a table-driven test asserting one row per variant (landed by Task 1, 2a70f579)
result: pass
source: automated
coverage_id: 25-02/D1
requirement: FT-01
verification: crates/paladin-core/src/platform/container/paladin_error.rs#tests::paladin_error_transience_table ; crates/paladin-ports/src/output/llm_port.rs#tests::llm_error_transience_table ; crates/paladin-ports/src/output/llm_port.rs#tests::provider_error_status_boundaries_classify_by_value

### 12. 25-02 D2: BattalionError::Node(NodeError) carries the structured node_error::NodeError (never the legacy battalion::NodeError summary), is Clone, and its Display names the node id and source summary
expected: BattalionError::Node(NodeError) carries the structured node_error::NodeError (never the legacy battalion::NodeError summary), is Clone, and its Display names the node id and source summary
result: pass
source: automated
coverage_id: 25-02/D2
requirement: FT-01
verification: crates/paladin-core/src/platform/container/battalion/mod.rs#tests::battalion_error_node_carries_the_structured_node_error ; crates/paladin-core/src/platform/container/battalion/mod.rs#tests::battalion_error_node_uses_the_core_node_error_not_the_legacy_summary ; crates/paladin-core/src/platform/container/battalion/mod.rs#tests::legacy_battalion_node_error_summary_is_unchanged

### 13. 25-02 D3: PaladinError, LlmError and BattalionError are all #[non_exhaustive]; every in-tree exhaustive match gained a wildcard arm; the workspace builds and lints clean across all targets and features
expected: PaladinError, LlmError and BattalionError are all #[non_exhaustive]; every in-tree exhaustive match gained a wildcard arm; the workspace builds and lints clean across all targets and features
result: pass
source: automated
coverage_id: 25-02/D3
requirement: FT-01
verification: cargo build --workspace --all-features --all-targets (exit 0) ; cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0) ; cargo fmt --check (exit 0)

### 14. 25-02 D4: The X-10 register (MIGRATION.md §9.2 rows + .cargo/semver-checks-allowlist.toml entries + per-crate lint suppressions) is complete and set-equal: 4 allowlist entries, 3 enum_marked_non_exhaustive lints, 4 §9.2 rows marked Y
expected: The X-10 register (MIGRATION.md §9.2 rows + .cargo/semver-checks-allowlist.toml entries + per-crate lint suppressions) is complete and set-equal: 4 allowlist entries, 3 enum_marked_non_exhaustive lints, 4 §9.2 rows marked Y
result: pass
source: automated
coverage_id: 25-02/D4
requirement: FT-01
verification: test \\"$(grep -c '^[[entry]]' .cargo/semver-checks-allowlist.toml)\\" -eq 4 (pass) ; test \\"$(grep -c 'enum_marked_non_exhaustive' .cargo/semver-checks-allowlist.toml)\\" -eq 3 (pass)

### 15. 25-03 D1: RetryPredicate::Custom(name) and ErrorHandlerSpec::Custom(name) resolve through registries registered on WarEngine (with_retry_predicate, with_error_handler); an unregistered name fails graph validation before any node executes, listing every offender in one error
expected: RetryPredicate::Custom(name) and ErrorHandlerSpec::Custom(name) resolve through registries registered on WarEngine (with_retry_predicate, with_error_handler); an unregistered name fails graph validation before any node executes, listing every offender in one error
result: pass
source: automated
coverage_id: 25-03/D1
requirement: FT-02
verification: crates/paladin-battalion/src/engine/graph.rs#tests::unregistered_custom_retry_predicate_fails_validation ; crates/paladin-battalion/src/engine/graph.rs#tests::unregistered_custom_error_handler_fails_validation ; crates/paladin-battalion/src/engine/graph.rs#tests::validation_lists_every_unregistered_name_not_just_the_first ; crates/paladin-battalion/src/engine/graph.rs#tests::registered_custom_names_validate_cleanly

### 16. 25-03 D2: The three registries travel as one EngineRegistries { edge_evaluators, retry_predicates, error_handlers } bundle passed to WarGraph::validate, replacing the old two-registry positional signature
expected: The three registries travel as one EngineRegistries { edge_evaluators, retry_predicates, error_handlers } bundle passed to WarGraph::validate, replacing the old two-registry positional signature
result: pass
source: automated
coverage_id: 25-03/D2
requirement: FT-02
verification: crates/paladin-battalion/src/engine/registries.rs#tests::default_bundle_has_empty_registries ; crates/paladin-battalion/src/engine/registries.rs#tests::clone_is_independent_arc_backed_snapshot

### 17. 25-03 D3: ErrorHandler::handle is async, Send + Sync, taking (&NodeError, &Battlefield) and returning Result<Directive, NodeError> -- the same async trait-object shape EdgeConditionEvaluator already uses
expected: ErrorHandler::handle is async, Send + Sync, taking (&NodeError, &Battlefield) and returning Result<Directive, NodeError> -- the same async trait-object shape EdgeConditionEvaluator already uses
result: pass
source: automated
coverage_id: 25-03/D3
requirement: FT-02
verification: crates/paladin-battalion/src/error_handler.rs#tests::name_lookup_is_exact_byte_equality_case_sensitive

### 18. 25-03 D4: Paladin and Function nodes accept the full Aegis; a Battalion node accepts timeout/on_error but rejects retry/cache; a Gate node rejects any Aegis
expected: Paladin and Function nodes accept the full Aegis; a Battalion node accepts timeout/on_error but rejects retry/cache; a Gate node rejects any Aegis
result: pass
source: automated
coverage_id: 25-03/D4
requirement: FT-02
verification: crates/paladin-battalion/src/engine/graph.rs#tests::battalion_node_rejects_retry_and_cache_but_accepts_timeout_and_on_error ; crates/paladin-battalion/src/engine/graph.rs#tests::gate_node_rejects_any_aegis

### 19. 25-03 D5: set_aegis on an undeclared node is a typed validation error listing all offenders
expected: set_aegis on an undeclared node is a typed validation error listing all offenders
result: pass
source: automated
coverage_id: 25-03/D5
requirement: FT-02
verification: crates/paladin-battalion/src/engine/graph.rs#tests::aegis_on_undeclared_node_is_a_validation_error

### 20. 25-03 D6: on_error and cache enter WarGraph::fingerprint() sorted by node id and length-prefixed; retry and timeout are excluded, so tightening a retry policy or a timeout never invalidates a stored Waypoint's fingerprint comparison on resume
expected: on_error and cache enter WarGraph::fingerprint() sorted by node id and length-prefixed; retry and timeout are excluded, so tightening a retry policy or a timeout never invalidates a stored Waypoint's fingerprint comparison on resume
result: pass
source: automated
coverage_id: 25-03/D6
requirement: FT-04
verification: crates/paladin-battalion/src/engine/graph.rs#tests::changing_on_error_changes_the_fingerprint ; crates/paladin-battalion/src/engine/graph.rs#tests::changing_cache_policy_changes_the_fingerprint ; crates/paladin-battalion/src/engine/graph.rs#tests::tuning_retry_does_not_change_the_fingerprint ; crates/paladin-battalion/src/engine/graph.rs#tests::tuning_timeout_does_not_change_the_fingerprint ; crates/paladin-battalion/src/engine/graph.rs#tests::aegis_hashing_is_sorted_by_node_id ; crates/paladin-battalion/src/engine/graph.rs#tests::resume_still_matches_after_a_retry_tuning_edit

### 21. 25-03 D7: GRAPH_FINGERPRINT_VERSION moves from v4 to v5 in the same commit as the hash-input change; the golden hex test and the version test are re-pinned together so a v4-tagged stored fingerprint is recognised as stale
expected: GRAPH_FINGERPRINT_VERSION moves from v4 to v5 in the same commit as the hash-input change; the golden hex test and the version test are re-pinned together so a v4-tagged stored fingerprint is recognised as stale
result: pass
source: automated
coverage_id: 25-03/D7
requirement: FT-04
verification: crates/paladin-battalion/src/engine/graph.rs#tests::fingerprint_golden_hex_v5 ; crates/paladin-battalion/src/engine/graph.rs#tests::fingerprint_version_is_v5

### 22. 25-03 D8: A child Battalion inherits the parent's registries wholesale, matching Phase 23 D-21's inherit-the-engine rule
expected: A child Battalion inherits the parent's registries wholesale, matching Phase 23 D-21's inherit-the-engine rule
result: pass
source: automated
coverage_id: 25-03/D8
requirement: FT-02
verification: crates/paladin-battalion/src/engine/graph.rs#tests::child_battalion_inherits_parent_registries

### 23. 25-03 D9: An Aegis whose RetryPolicy has max_attempts == 0, or whose TimeoutPolicy has Some(Duration::ZERO) on either field, is a typed validation error before execution; a TimeoutPolicy with both fields None is a valid no-op
expected: An Aegis whose RetryPolicy has max_attempts == 0, or whose TimeoutPolicy has Some(Duration::ZERO) on either field, is a typed validation error before execution; a TimeoutPolicy with both fields None is a valid no-op
result: pass
source: automated
coverage_id: 25-03/D9
requirement: FT-02
verification: crates/paladin-battalion/src/engine/graph.rs#tests::retry_policy_with_zero_attempts_fails_validation ; crates/paladin-battalion/src/engine/graph.rs#tests::timeout_policy_with_zero_duration_fails_validation

### 24. 25-04 D1: CachedDelta persisted record with its own schema_version (X-04) and a closed TTL boundary (an entry read at exactly its expires_at instant is a miss)
expected: CachedDelta persisted record with its own schema_version (X-04) and a closed TTL boundary (an entry read at exactly its expires_at instant is a miss)
result: pass
source: automated
coverage_id: 25-04/D1
requirement: FT-06
verification: crates/paladin-core/src/platform/container/node_cache.rs#tests::cached_delta_round_trips_through_serde_with_schema_version ; crates/paladin-core/src/platform/container/node_cache.rs#tests::is_expired_at_boundary_is_closed

### 25. 25-04 D2: NodeCachePort async trait (get/put/invalidate) in paladin-ports, Send+Sync and object-safe, best-effort by construction per D-29
expected: NodeCachePort async trait (get/put/invalidate) in paladin-ports, Send+Sync and object-safe, best-effort by construction per D-29
result: pass
source: automated
coverage_id: 25-04/D2
requirement: FT-06
verification: crates/paladin-ports/src/output/node_cache_port.rs#tests::trait_is_object_safe ; crates/paladin-ports/src/output/node_cache_port.rs#tests::mock_cache_implements_trait

### 26. 25-04 D3: InMemoryNodeCache plus the shared 9-case contract suite (hit, miss, TTL expiry, closed boundary, invalidate-prefix, overwrite, concurrent put, empty delta, cross-graph non-collision) covering all four FT-06 edge assumptions
expected: InMemoryNodeCache plus the shared 9-case contract suite (hit, miss, TTL expiry, closed boundary, invalidate-prefix, overwrite, concurrent put, empty delta, cross-graph non-collision) covering all four FT-06 edge assumptions
result: pass
source: automated
coverage_id: 25-04/D3
requirement: FT-06
verification: crates/paladin-storage/src/node_cache/in_memory.rs#tests::run_all_contract_functions_smoke_aggregate ; crates/paladin-storage/src/node_cache/in_memory.rs#tests::entry_at_exactly_expires_at_is_expired ; crates/paladin-storage/src/node_cache/in_memory.rs#tests::concurrent_put_of_the_same_key_leaves_exactly_one_entry ; crates/paladin-storage/src/node_cache/in_memory.rs#tests::an_empty_delta_round_trips_as_a_hit_not_a_miss ; crates/paladin-storage/src/node_cache/in_memory.rs#tests::keys_differing_only_by_graph_prefix_do_not_collide

### 27. 25-04 D5: NodeCacheConfig (X-09 shape: Default/validate/EnvOverridable), disabled by default with the InMemory backend, never Debug-printing redis_password, introducing no Aegis policy config surface
expected: NodeCacheConfig (X-09 shape: Default/validate/EnvOverridable), disabled by default with the InMemory backend, never Debug-printing redis_password, introducing no Aegis policy config surface
result: pass
source: automated
coverage_id: 25-04/D5
requirement: FT-06
verification: src/config/node_cache.rs#tests::default_node_cache_config_is_disabled ; src/config/node_cache.rs#tests::env_overrides_apply_for_every_field ; src/config/node_cache.rs#tests::validate_rejects_an_empty_key_prefix_and_a_zero_port ; src/config/node_cache.rs#tests::redis_backend_without_a_host_fails_validation ; src/config/node_cache.rs#tests::debug_rendering_never_prints_the_password

### 28. 25-05 D2: All nine adapters route their non-2xx branches through the helper; no stringly HTTP fallback and no status-by-substring assertion remains
expected: All nine adapters route their non-2xx branches through the helper; no stringly HTTP fallback and no status-by-substring assertion remains
result: pass
source: automated
coverage_id: 25-05/D2
requirement: FT-01
verification: test \\"$(grep -rl 'map_http_status(' crates/paladin-llm/src/*/adapter.rs | wc -l)\\" -eq 9 (pass, 9) ; ! grep -rnE 'ProcessingError\\\\(format!\\\\(\\"HTTP' crates/paladin-llm/src/ (pass, no matches) ; ! grep -rnE 'contains\\\\(\\"(4[0-9][0-9]|5[0-9][0-9])\\"\\\\)' crates/paladin-llm/src/ tests/unit/llm/deepseek_adapter_test.rs tests/unit/mock_llm_adapter_test.rs (pass, no matches) ; per-adapter *_non_2xx_routes_through_the_shared_mapper (9), kimi_http_500_carries_a_typed_status, streaming_non_2xx_routes_through_the_shared_mapper, gemini_error_envelope_still_parses_before_mapping, ollama_local_server_4xx_maps_without_a_dedicated_variant

### 29. 25-05 D3: Workspace builds, tests, lints and formats clean across all targets and features
expected: Workspace builds, tests, lints and formats clean across all targets and features
result: pass
source: automated
coverage_id: 25-05/D3
requirement: FT-01
verification: cargo check --workspace --all-targets --all-features (exit 0) ; cargo test -p paladin-llm --lib (68 default-feature) and --all-features (347) (exit 0) ; cargo test --test lib (692 passed, 14 ignored/self-skipped, exit 0) ; cargo fmt --all -- --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)

### 30. 25-06 D1: One conversion function turns every LlmError variant into LlmFailure with transience from LlmError::transience(), status/provider from typed fields (None where absent, never a sentinel), and a message byte-identical to the legacy erasure
expected: One conversion function turns every LlmError variant into LlmFailure with transience from LlmError::transience(), status/provider from typed fields (None where absent, never a sentinel), and a message byte-identical to the legacy erasure
result: pass
source: automated
coverage_id: 25-06/D1
requirement: FT-01
verification: crates/paladin-battalion/src/llm_failure.rs#tests::conversion_preserves_the_rendered_message_exactly (every variant) ; crates/paladin-battalion/src/llm_failure.rs#tests::conversion_carries_transience_from_the_source ; crates/paladin-battalion/src/llm_failure.rs#tests::{provider_error_conversion_carries_status_and_provider,variants_without_a_status_convert_with_none,usage_limit_exceeded_carries_its_provider,all_providers_failed_converts_with_its_last_error_transience,converted_failure_is_retryable_like_the_legacy_variant} ; cargo test -p paladin-battalion --doc llm_failure (1 passed); ! grep -qE 'contains\\\\(|split\\\\(|parse::<u16>' crates/paladin-battalion/src/llm_failure.rs (0 hits)

### 31. 25-06 D2: Every first-party site holding a real LlmError produces the structured LlmFailure; the stream-open and mid-stream sites surface it to callers with Transient/Some(503)/Some(\\"openai\\") for a provider 503 and Permanent/None/None for an authentication failure; the temperature site likewise
expected: Every first-party site holding a real LlmError produces the structured LlmFailure; the stream-open and mid-stream sites surface it to callers with Transient/Some(503)/Some(\\"openai\\") for a provider 503 and Permanent/None/None for an authentication failure; the temperature site likewise
result: pass
source: automated
coverage_id: 25-06/D2
requirement: FT-01
verification: src/application/services/paladin/paladin_execution_service.rs#tests::paladin_execution_service_surfaces_structured_llm_failure ; src/application/services/paladin/paladin_execution_service.rs#tests::permanent_provider_failure_surfaces_as_permanent ; src/application/services/paladin/temperature_service.rs#tests::temperature_service_surfaces_structured_llm_failure ; crates/paladin-battalion/src/conclave_execution_service.rs#tests::conclave_execution_service_surfaces_structured_llm_failure

### 32. 25-06 D3: No rendered error text changed and the circuit breaker's behaviour did not drift: converted failures render `LLM error: {e}` at every observable site, a threshold-1 breaker trips on the first converted failure exactly as it did on the legacy variant, and circuit_breaker.rs has a zero-line diff across the plan
expected: No rendered error text changed and the circuit breaker's behaviour did not drift: converted failures render `LLM error: {e}` at every observable site, a threshold-1 breaker trips on the first converted failure exactly as it did on the legacy variant, and circuit_breaker.rs has a zero-line diff across the plan
result: pass
source: automated
coverage_id: 25-06/D3
requirement: FT-01
verification: src/application/services/paladin/paladin_execution_service.rs#tests::rendered_error_text_at_every_migrated_site_is_unchanged ; src/application/services/paladin/paladin_execution_service.rs#tests::buffered_retry_sites_trip_the_circuit_breaker_like_the_legacy_variant ; git diff --name-only 7b8815e8 HEAD -- src/infrastructure/resilience/circuit_breaker.rs | wc -l == 0

### 33. 25-06 D4: Workspace builds, lints and tests clean across all targets and features after the migration
expected: Workspace builds, lints and tests clean across all targets and features after the migration
result: pass
source: automated
coverage_id: 25-06/D4
requirement: FT-01
verification: cargo check --workspace --all-targets --all-features (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0); cargo fmt --all -- --check (exit 0) ; cargo test --workspace --lib (exit 0, every crate ok); cargo test -p paladin-battalion --lib (616 passed); cargo test -p paladin-ai --lib (550 passed); cargo test --doc -p paladin-battalion (47 passed); cargo test --doc -p paladin-ai (113 passed)

### 34. 25-07 D1: Failed attempts are recorded ascending on NodeExecutionRecord.attempts (failed only; attempt keeps the succeeding number), a first-time success has an empty list, and the fields are #[serde(default)] with BATTLEFIELD_SCHEMA_VERSION unchanged
expected: Failed attempts are recorded ascending on NodeExecutionRecord.attempts (failed only; attempt keeps the succeeding number), a first-time success has an empty list, and the fields are #[serde(default)] with BATTLEFIELD_SCHEMA_VERSION unchanged
result: pass
source: automated
coverage_id: 25-07/D1
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::failed_attempts_are_recorded_in_order ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_node_that_succeeds_first_time_records_an_empty_attempts_list ; crates/paladin-core/src/platform/container/waypoint.rs#tests::{failed_attempts_are_recorded_in_order,record_round_trips_with_the_new_fields_and_without_them,battlefield_schema_version_is_unchanged}

### 35. 25-07 D2: TraceEvent::NodeStarted/NodeFinished fire once per attempt carrying attempt; NodeFinished carries cache_hit, false everywhere with no cache configured
expected: TraceEvent::NodeStarted/NodeFinished fire once per attempt carrying attempt; NodeFinished carries cache_hit, false everywhere with no cache configured
result: pass
source: automated
coverage_id: 25-07/D2
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::node_events_are_emitted_once_per_attempt_with_the_attempt_number ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::cache_hit_defaults_to_false_on_every_record_and_event

### 36. 25-07 D4: An Aegis-governed exhausted failure travels as a structured NodeError: WaypointStatus::Failed.node_error, EngineError::NodeFailed, RunOutcome::node_error() and BattalionError::Node all carry the identical value; the display line is unchanged; no-Aegis and engine-limit failures carry None
expected: An Aegis-governed exhausted failure travels as a structured NodeError: WaypointStatus::Failed.node_error, EngineError::NodeFailed, RunOutcome::node_error() and BattalionError::Node all carry the identical value; the display line is unchanged; no-Aegis and engine-limit failures carry None
result: pass
source: automated
coverage_id: 25-07/D4
requirement: FT-01
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::exhausted_retry_writes_a_failed_waypoint_carrying_the_structured_error ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::the_display_line_on_a_failed_waypoint_is_unchanged ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::pre_aegis_and_limit_failures_carry_none ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_outcome_failed_exposes_the_same_node_error ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::engine_error_node_failed_maps_to_battalion_error_node

### 37. 25-07 D5: A PaladinPort failure becomes NodeErrorSource::Paladin { kind: variant name } and an underlying LlmFailure becomes NodeErrorSource::Llm carrying the typed status/provider and transience -- never Function; NodeError JSON has a stable declaration-ordered field layout
expected: A PaladinPort failure becomes NodeErrorSource::Paladin { kind: variant name } and an underlying LlmFailure becomes NodeErrorSource::Llm carrying the typed status/provider and transience -- never Function; NodeError JSON has a stable declaration-ordered field layout
result: pass
source: automated
coverage_id: 25-07/D5
requirement: FT-01
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_paladin_node_failure_becomes_node_error_source_paladin ; crates/paladin-battalion/src/llm_failure.rs#tests::{llm_failure_becomes_node_error_source_llm_with_its_typed_fields,non_llm_paladin_failures_become_node_error_source_paladin_named_by_variant} ; crates/paladin-core/src/platform/container/node_error.rs#tests::node_error_json_field_order_is_stable

### 38. 25-07 D6: Retries are per task inside a Muster: a failing task retries three times while every sibling runs once, siblings finish while the failing task is still in backoff (paused clock), and each task has its own attempt counter
expected: Retries are per task inside a Muster: a failing task retries three times while every sibling runs once, siblings finish while the failing task is still in backoff (paused clock), and each task has its own attempt counter
result: pass
source: automated
coverage_id: 25-07/D6
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::one_mustered_task_retries_without_re_running_siblings ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::sibling_tasks_do_not_wait_for_a_retrying_task_to_finish ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::each_muster_task_has_its_own_attempt_counter

### 39. 25-07 D7: No Waypoint is written between attempts, muster-progress Waypoints list only completed tasks with the MusterProgress shape unchanged, and a run interrupted mid-retry resumes by re-executing the node from attempt 1
expected: No Waypoint is written between attempts, muster-progress Waypoints list only completed tasks with the MusterProgress shape unchanged, and a run interrupted mid-retry resumes by re-executing the node from attempt 1
result: pass
source: automated
coverage_id: 25-07/D7
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::no_waypoint_is_written_between_attempts ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::muster_progress_waypoints_record_only_completed_tasks ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::resume_re_executes_an_interrupted_node_from_attempt_one ; git diff 2a266e61 -- crates/paladin-core/src/platform/container/waypoint.rs | grep -c MusterProgress == 0

### 40. 25-07 D8: A NextStep::Parley Directive is a success that is never retried (call count 1, no retry budget consumed) and the post-resume re-run of a parleying node starts at attempt 1
expected: A NextStep::Parley Directive is a success that is never retried (call count 1, no retry budget consumed) and the post-resume re-run of a parleying node starts at attempt 1
result: pass
source: automated
coverage_id: 25-07/D8
requirement: FT-02
verification: crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_parley_directive_is_never_retried ; crates/paladin-battalion/src/engine/mod.rs#engine::tests::post_resume_rerun_of_a_parleying_node_starts_at_attempt_one

### 41. 25-08 D1: FallbackLlmAdapter hops on Transient/Unknown only, Permanent short-circuits, exhaustion reports AllProvidersFailed in chain order, every call starts at element 0, concurrent calls share no state
expected: FallbackLlmAdapter hops on Transient/Unknown only, Permanent short-circuits, exhaustion reports AllProvidersFailed in chain order, every call starts at element 0, concurrent calls share no state
result: pass
source: automated
coverage_id: 25-08/D1
requirement: FT-05
verification: cargo test -p paladin-llm --lib fallback (16 tests: three_provider_chain_falls_through_two_transient_failures, permanent_error_short_circuits_after_one_call, unknown_error_hops, exhaustion_returns_all_providers_failed_in_chain_order, every_call_starts_at_the_first_provider, concurrent_calls_share_no_hop_state, ...)

### 42. 25-08 D2: Streaming first-chunk rule: fall through on a call error or first-item Err; after an Ok chunk the error propagates with the prefix and no provider switch
expected: Streaming first-chunk rule: fall through on a call error or first-item Err; after an Ok chunk the error propagates with the prefix and no provider switch
result: pass
source: automated
coverage_id: 25-08/D2
requirement: FT-05
verification: cargo test -p paladin-llm --lib streaming_ (streaming_error_before_the_first_chunk_falls_through, streaming_call_error_falls_through, streaming_error_after_the_first_chunk_propagates_with_the_prefix, streaming_exhaustion_returns_all_providers_failed)

### 43. 25-08 D3: Per-hop observability: TraceEvent::FallbackHop with node_id None and both provider names, plus a warn! log line
expected: Per-hop observability: TraceEvent::FallbackHop with node_id None and both provider names, plus a warn! log line
result: pass
source: automated
coverage_id: 25-08/D3
requirement: FT-05
verification: cargo test -p paladin-llm --lib each_hop_emits_a_trace_event_and_a_warning; cargo test -p paladin-ports --lib fallback_hop_variant_constructs_with_no_node_id

### 44. 25-08 D4: PaladinResult.served_by is additive, legacy JSON byte-identical, Default/new still work, struct not non_exhaustive, fallback-served results name the serving provider
expected: PaladinResult.served_by is additive, legacy JSON byte-identical, Default/new still work, struct not non_exhaustive, fallback-served results name the serving provider
result: pass
source: automated
coverage_id: 25-08/D4
requirement: FT-05
verification: cargo test -p paladin-ai-core --lib execution_result (4 tests); cargo test --test unit served_by (fallback_served_result_records_the_serving_provider, a_non_fallback_result_leaves_served_by_none, paladin_result_is_not_marked_non_exhaustive) ; cargo check --workspace --all-targets --all-features; cargo test --doc -p paladin-ai-core; cargo test --doc -p paladin-ports

### 45. 25-08 D5: Deliberate-breaking register: MIGRATION.md 9.2 PaladinResult row Y, fifth allowlist entry, paladin-core constructible_struct_adds_field suppression, all in the field's commit
expected: Deliberate-breaking register: MIGRATION.md 9.2 PaladinResult row Y, fifth allowlist entry, paladin-core constructible_struct_adds_field suppression, all in the field's commit
result: pass
source: automated
coverage_id: 25-08/D5
requirement: FT-05
verification: test \\"$(grep -c '^\\\\[\\\\[entry\\\\]\\\\]' .cargo/semver-checks-allowlist.toml)\\" -eq 5; ! grep -q 'TBD — owner FT-05' MIGRATION.md

### 46. 25-09 D1: A port beating every 100 ms under idle_timeout 250 ms / run_timeout 10 s completes with one attempt and no timer fires
expected: A port beating every 100 ms under idle_timeout 250 ms / run_timeout 10 s completes with one attempt and no timer fires
result: pass
source: automated
coverage_id: 25-09/D1
requirement: FT-03
verification: crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_port_beating_every_100ms_survives_a_250ms_idle_timeout

### 47. 25-09 D2: The same policy over a port that stalls 300 ms fails Timeout(Idle), Transient, asserted by typed kind
expected: The same policy over a port that stalls 300 ms fails Timeout(Idle), Transient, asserted by typed kind
result: pass
source: automated
coverage_id: 25-09/D2
requirement: FT-03
verification: crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_port_that_stalls_300ms_fails_with_timeout_idle ; crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::the_fired_bound_is_read_from_the_typed_kind

### 48. 25-09 D3: A node that keeps beating is still cut by run_timeout and the failure names Run, not Idle; a timed-out attempt is retried as Transient and its partial delta never merges
expected: A node that keeps beating is still cut by run_timeout and the failure names Run, not Idle; a timed-out attempt is retried as Transient and its partial delta never merges
result: pass
source: automated
coverage_id: 25-09/D3
requirement: FT-03
verification: crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_slow_but_progressing_node_fails_on_run_timeout_not_idle ; crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_timed_out_attempt_is_retried_as_transient ; crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_timed_out_attempts_partial_work_is_discarded

### 49. 25-09 D4: EngineLimits.run_timeout is enforced (RunTimeoutExceeded, same Waypoint path as the other limits), nested with the attempt bound, tightest named, EngineRun recorded on the cut attempt; None means no bound; excluded from the fingerprint; bridges carry no legacy timeout
expected: EngineLimits.run_timeout is enforced (RunTimeoutExceeded, same Waypoint path as the other limits), nested with the attempt bound, tightest named, EngineRun recorded on the cut attempt; None means no bound; excluded from the fingerprint; bridges carry no legacy timeout
result: pass
source: automated
coverage_id: 25-09/D4
requirement: FT-03
verification: crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::{engine_run_timeout_ends_the_run_with_a_typed_error,an_attempt_cut_by_the_engine_bound_records_timeout_enginerun,the_tightest_bound_fires,no_engine_run_timeout_means_no_run_level_bound,run_timeout_is_not_hashed_into_the_fingerprint,bridges_carry_no_legacy_battalion_timeout} ; src/config/engine.rs#config::engine::tests::config_seconds_convert_to_duration_exactly

### 50. 25-09 D5: NodeContext keeps Debug+Clone+PartialEq with a HeartbeatHandle; heartbeat() without idle_timeout is a no-op; ctx.attempt is 2 on the second attempt; execute_observed defaults to execute with no beat; the engine always calls execute_observed; the service beats on LLM completion, stream chunk and Armament call
expected: NodeContext keeps Debug+Clone+PartialEq with a HeartbeatHandle; heartbeat() without idle_timeout is a no-op; ctx.attempt is 2 on the second attempt; execute_observed defaults to execute with no beat; the engine always calls execute_observed; the service beats on LLM completion, stream chunk and Armament call
result: pass
source: automated
coverage_id: 25-09/D5
requirement: FT-03
verification: crates/paladin-battalion/src/engine/node.rs#engine::node::tests::node_context_keeps_its_derives_with_a_heartbeat_handle ; crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::{heartbeat_is_a_no_op_without_an_idle_timeout,node_context_exposes_the_current_attempt,the_engine_always_calls_execute_observed} ; crates/paladin-ports/src/output/paladin_port.rs#output::paladin_port::tests::execute_observed_defaults_to_execute ; src/application/services/paladin/paladin_execution_service.rs#tests::paladin_execution_service_beats_on_llm_completion_stream_chunk_and_armament

### 51. 25-10 D1: Route { to, error_field } is validated (target declared, not a worker template; error_field declared, non-Sum) and Absorb { fallback_delta } against the schema (empty legal), every clause listing all offenders
expected: Route { to, error_field } is validated (target declared, not a worker template; error_field declared, non-Sum) and Absorb { fallback_delta } against the schema (empty legal), every clause listing all offenders
result: pass
source: automated
coverage_id: 25-10/D1
requirement: FT-04
verification: crates/paladin-battalion/src/engine/graph.rs#tests::route_error_field_must_be_declared_in_the_schema ; crates/paladin-battalion/src/engine/graph.rs#tests::route_error_field_must_not_use_sum_dispatch ; crates/paladin-battalion/src/engine/graph.rs#tests::route_target_must_be_a_declared_node ; crates/paladin-battalion/src/engine/graph.rs#tests::route_target_must_not_be_a_worker_template ; crates/paladin-battalion/src/engine/graph.rs#tests::absorb_fallback_delta_is_validated_against_the_schema ; crates/paladin-battalion/src/engine/graph.rs#tests::absorb_with_an_empty_fallback_delta_validates ; crates/paladin-battalion/src/engine/graph.rs#tests::every_handler_validation_error_lists_all_offenders

### 52. 25-10 D2: A Route target is auto-eligible through validate_eligible_set's fixed-point insertion point with no mark_dynamic_target call
expected: A Route target is auto-eligible through validate_eligible_set's fixed-point insertion point with no mark_dynamic_target call
result: pass
source: automated
coverage_id: 25-10/D2
requirement: FT-04
verification: crates/paladin-battalion/src/engine/graph.rs#tests::a_route_target_reachable_only_by_routing_is_not_stranded ; crates/paladin-battalion/src/engine/graph.rs#tests::a_route_target_reached_late_has_its_own_edges_expanded

### 53. 25-10 D3: Route writes the structured NodeError JSON into error_field (compared as parsed fields) and places `to` in the next Vanguard replacing the failed node's static successors; the run Completes; a terminal target completes normally
expected: Route writes the structured NodeError JSON into error_field (compared as parsed fields) and places `to` in the next Vanguard replacing the failed node's static successors; the run Completes; a terminal target completes normally
result: pass
source: automated
coverage_id: 25-10/D3
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::route_writes_the_structured_error_and_places_the_target ; crates/paladin-battalion/src/engine/superstep.rs#tests::route_replaces_the_failed_nodes_static_successors ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_route_target_with_no_outgoing_edges_completes_the_run_normally

### 54. 25-10 D4: Absorb records outcome: Failed, merges the fallback delta (or nothing for an empty one) and fires static edges as on success
expected: Absorb records outcome: Failed, merges the fallback delta (or nothing for an empty one) and fires static edges as on success
result: pass
source: automated
coverage_id: 25-10/D4
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::absorb_merges_its_delta_and_fires_static_edges ; crates/paladin-battalion/src/engine/superstep.rs#tests::absorb_with_an_empty_fallback_delta_merges_nothing_and_continues

### 55. 25-10 D5: No handler: Failed Waypoint with node_error: Some and RunOutcome::Failed carrying the same NodeError
expected: No handler: Failed Waypoint with node_error: Some and RunOutcome::Failed carrying the same NodeError
result: pass
source: automated
coverage_id: 25-10/D5
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::no_handler_fails_the_run_with_the_structured_error

### 56. 25-10 D6: A handler runs only after retries exhaust (three attempts, one invocation) or immediately on a non-retryable error (one attempt)
expected: A handler runs only after retries exhaust (three attempts, one invocation) or immediately on a non-retryable error (one attempt)
result: pass
source: automated
coverage_id: 25-10/D6
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_does_not_run_while_retries_remain ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_non_retryable_error_reaches_the_handler_immediately

### 57. 25-10 D7: A registered Custom handler receives (&NodeError, &Battlefield pre-failure snapshot) and its Directive is honoured -- Edges merges + fires static edges, Goto places its target, End completes -- while Err fails the run with the handler's own error
expected: A registered Custom handler receives (&NodeError, &Battlefield pre-failure snapshot) and its Directive is honoured -- Edges merges + fires static edges, Goto places its target, End completes -- while Err fails the run with the handler's own error
result: pass
source: automated
coverage_id: 25-10/D7
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_receives_the_structured_error_and_the_battlefield ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_edges_contributes_its_delta ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_goto_places_its_target ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_end_completes_the_run ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_err_fails_the_run_with_that_error

### 58. 25-10 D8: Handler-routed visits count against max_node_visits through the one existing counter; an A->B->A compensation cycle terminates with NodeVisitLimitExceeded under a timeout guard
expected: Handler-routed visits count against max_node_visits through the one existing counter; an A->B->A compensation cycle terminates with NodeVisitLimitExceeded under a timeout guard
result: pass
source: automated
coverage_id: 25-10/D8
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_routed_visit_counts_against_max_node_visits ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_compensation_cycle_terminates_with_the_visit_limit

### 59. 25-11 D1: On a worker template only Absorb and Custom are allowed; Route is a validation error listing every offender and naming the aggregator alternative
expected: On a worker template only Absorb and Custom are allowed; Route is a validation error listing every offender and naming the aggregator alternative
result: pass
source: automated
coverage_id: 25-11/D1
requirement: FT-04
verification: crates/paladin-battalion/src/engine/graph.rs#tests::route_on_a_worker_template_is_rejected_at_validation ; crates/paladin-battalion/src/engine/graph.rs#tests::absorb_on_a_worker_template_validates ; crates/paladin-battalion/src/engine/graph.rs#tests::custom_on_a_worker_template_validates ; crates/paladin-battalion/src/engine/graph.rs#tests::every_worker_template_route_offender_is_listed

### 60. 25-11 D2: A Custom handler inside a Muster must be delta-only: Goto/End/Parley/Muster are MusterHandlerMustBeDeltaOnly naming node and task_key; an Edges delta or an Absorb fallback is the task's aggregation contribution and the task count is unchanged
expected: A Custom handler inside a Muster must be delta-only: Goto/End/Parley/Muster are MusterHandlerMustBeDeltaOnly naming node and task_key; an Edges delta or an Absorb fallback is the task's aggregation contribution and the task count is unchanged
result: pass
source: automated
coverage_id: 25-11/D2
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_edges_contributes_its_delta_to_the_aggregation ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_goto_fails_the_run_with_a_typed_error ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_end_or_parley_or_muster_is_the_same_typed_error ; crates/paladin-battalion/src/engine/superstep.rs#tests::an_absorbed_worker_task_still_appears_in_the_aggregation

### 61. 25-11 D3: A Custom handler may Parley through the existing HITL-01 path; the post-resume re-run is a fresh attempt 1 with ctx.parley_response() set; no retry budget consumed; peers merge normally; rejected inside a Muster
expected: A Custom handler may Parley through the existing HITL-01 path; the post-resume re-run is a fresh attempt 1 with ctx.parley_response() set; no retry budget consumed; peers merge normally; rejected inside a Muster
result: pass
source: automated
coverage_id: 25-11/D3
requirement: FT-04
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_suspends_the_run ; crates/paladin-battalion/src/engine/superstep.rs#tests::the_post_resume_rerun_is_a_fresh_attempt_one ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_does_not_consume_the_retry_budget ; crates/paladin-battalion/src/engine/superstep.rs#tests::the_suspending_supersteps_peers_merge_normally ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_inside_a_muster_is_rejected

### 62. 25-11 D4: PRD 04 section 3.5 end to end over a real backend: the compensation chain Completes with booking_error's parsed fields and book's record Failed; a permanent failure is not retried; the a/b loop ends NodeVisitLimitExceeded under a timeout guard; a handler parleys a human across a process drop; the handler-less mirror carries node_error: Some
expected: PRD 04 section 3.5 end to end over a real backend: the compensation chain Completes with booking_error's parsed fields and book's record Failed; a permanent failure is not retried; the a/b loop ends NodeVisitLimitExceeded under a timeout guard; a handler parleys a human across a process drop; the handler-less mirror carries node_error: Some
result: pass
source: automated
coverage_id: 25-11/D4
requirement: FT-04
verification: tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node ; tests/integration/e2e_compensation_chain_test.rs#a_permanent_failure_is_not_retried_before_routing ; tests/integration/e2e_compensation_chain_test.rs#compensation_loop_terminates_at_the_visit_limit ; tests/integration/e2e_compensation_chain_test.rs#on_payment_failure_parley_a_human ; tests/integration/e2e_compensation_chain_test.rs#the_failed_waypoint_of_a_handler_less_run_carries_the_structured_error

### 63. 25-12 D1: FaultyPaladinPort::fail_paladin_until_attempt fails one named Paladin for its own first N calls with a Transient LlmFailure { status: Some(503) }, independently per name, composing with -- and not changing -- the global fail_until_attempt counter
expected: FaultyPaladinPort::fail_paladin_until_attempt fails one named Paladin for its own first N calls with a Transient LlmFailure { status: Some(503) }, independently per name, composing with -- and not changing -- the global fail_until_attempt counter
result: pass
source: automated
coverage_id: 25-12/D1
requirement: FT-02
verification: tests/helpers/mock_paladin_port.rs#tests::{fail_paladin_until_attempt_is_scoped_to_one_paladin,fail_paladin_until_attempt_returns_a_transient_llm_failure,the_global_fail_until_attempt_semantics_are_unchanged,the_two_mechanisms_compose,per_paladin_counters_are_independent} (run via cargo test --test e2e_muster_defer_order) ; tests/helpers/mock_paladin_port.rs#tests::faulty_paladin_port_fail_until_attempt_then_succeeds -- pre-existing, unedited (git diff shows 0 removed lines naming it)

### 64. 25-12 D2: E2E-3's recovering worker recovers through a real per-task Aegis retry under the default TransientOnly predicate: exactly 7 port calls, w3 attempt 3 with two AttemptRecords, siblings attempt 1, aggregator once, 5 results in task_key order, one muster-superstep Waypoint plus 5 progress Waypoints and none between attempts; the scripted seam is gone
expected: E2E-3's recovering worker recovers through a real per-task Aegis retry under the default TransientOnly predicate: exactly 7 port calls, w3 attempt 3 with two AttemptRecords, siblings attempt 1, aggregator once, 5 results in task_key order, one muster-superstep Waypoint plus 5 progress Waypoints and none between attempts; the scripted seam is gone
result: pass
source: automated
coverage_id: 25-12/D2
requirement: FT-02
verification: tests/integration/e2e_muster_defer_order_test.rs#{one_worker_recovers_by_real_per_task_retry,the_default_predicate_is_used,without_a_retry_policy_the_same_transient_failure_fails_the_run} (cargo test --test e2e_muster_defer_order) ; grep -c 'PHASE 25 SEAM' tests/integration/e2e_muster_defer_order_test.rs == 0; no one_worker_recovers_by_manual_attempt_scripting, no fail_until_attempt(2), no TransientAndUnknown

### 65. 25-12 D3: X-05: Muster combined with per-task retry holds under real multi-thread concurrency with exact per-task and total counts, exact attempt histories, task_key-ordered aggregation and no cross-task retry-state leakage, under a timeout guard
expected: X-05: Muster combined with per-task retry holds under real multi-thread concurrency with exact per-task and total counts, exact attempt histories, task_key-ordered aggregation and no cross-task retry-state leakage, under a timeout guard
result: pass
source: automated
coverage_id: 25-12/D3
requirement: FT-02
verification: tests/integration/aegis_retry_stress_test.rs#{muster_with_per_task_retry_under_concurrency_has_exact_counts,concurrent_tasks_do_not_share_retry_state} (cargo test --test aegis_retry_stress; 6 consecutive runs green)

### 66. 25-12 D4: A run killed while a node sleeps in a 60 s backoff aborts in under 1 s of virtual time, records the node Skipped { shutdown } re-listed on the Halted vanguard, and resuming from that Waypoint re-executes it at attempt 1 with no Waypoint written between the attempts
expected: A run killed while a node sleeps in a 60 s backoff aborts in under 1 s of virtual time, records the node Skipped { shutdown } re-listed on the Halted vanguard, and resuming from that Waypoint re-executes it at attempt 1 with no Waypoint written between the attempts
result: pass
source: automated
coverage_id: 25-12/D4
requirement: FT-02
verification: tests/integration/aegis_retry_stress_test.rs#{a_run_killed_during_backoff_aborts_immediately,resuming_after_a_kill_during_backoff_restarts_at_attempt_one}

### 67. 25-12 D5: EngineLimits.run_timeout tighter than both per-attempt bounds names Timeout(EngineRun) by value, is never retried and ends the run EngineError::RunTimeoutExceeded against a real SQLite Waypoint backend; the mirror names Timeout(Run), is retried to exhaustion and ends NodeFailed
expected: EngineLimits.run_timeout tighter than both per-attempt bounds names Timeout(EngineRun) by value, is never retried and ends the run EngineError::RunTimeoutExceeded against a real SQLite Waypoint backend; the mirror names Timeout(Run), is retried to exhaustion and ends NodeFailed
result: pass
source: automated
coverage_id: 25-12/D5
requirement: FT-03
verification: tests/integration/aegis_retry_stress_test.rs#{an_engine_run_timeout_tighter_than_both_bounds_names_enginerun,a_node_run_timeout_tighter_than_the_engine_budget_names_run}

### 68. 25-12 D6: Nothing in this plan's tests depends on Docker, Redis or any live service, and every test binary that includes the edited shared helper still passes
expected: Nothing in this plan's tests depends on Docker, Redis or any live service, and every test binary that includes the edited shared helper still passes
result: pass
source: automated
coverage_id: 25-12/D6
requirement: FT-04
verification: grep -qiE 'redis|docker|127.0.0.1:6379' tests/integration/aegis_retry_stress_test.rs -> no match; 13 root [[test]] binaries run green (table below)

### 69. 25-13 D1: FieldSpec.cache: CacheMarker { Allow (default), Deny }, additive, schema version unchanged
expected: FieldSpec.cache: CacheMarker { Allow (default), Deny }, additive, schema version unchanged
result: pass
source: automated
coverage_id: 25-13/D1
requirement: FT-06
verification: crates/paladin-core/src/platform/container/battlefield.rs#tests::field_spec_cache_marker_defaults_to_allow ; crates/paladin-core/src/platform/container/battlefield.rs#tests::schema_version_is_unchanged_by_the_cache_marker ; crates/paladin-storage (three-backend Waypoint contract suite, cargo test -p paladin-storage --features sqlite --lib: 122 passed)

### 70. 25-13 D2: Fail-closed validation: CachePolicy with no backend (incl. inside a Battalion child), on a Deny output_field, on an undeclared Fields name; every offender in one error; Append at Allow validates
expected: Fail-closed validation: CachePolicy with no backend (incl. inside a Battalion child), on a Deny output_field, on an undeclared Fields name; every offender in one error; Append at Allow validates
result: pass
source: automated
coverage_id: 25-13/D2
requirement: FT-06
verification: crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_without_an_engine_cache_fails_validation ; crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_on_a_deny_output_field_fails_validation ; crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::validation_lists_every_cache_offender ; crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::an_append_dispatch_field_may_still_be_cached_when_marked_allow ; crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_inside_a_battalion_child_fails_without_a_backend ; crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_key_spec_naming_an_undeclared_field_fails_validation

### 71. 25-13 D3: Key composition per D-28: graph fingerprint, Paladin config fingerprint, stability, full-snapshot default, Fields narrowing, muster payload, exact prefixes, no cross-kind collision
expected: Key composition per D-28: graph fingerprint, Paladin config fingerprint, stability, full-snapshot default, Fields narrowing, muster payload, exact prefixes, no cross-kind collision
result: pass
source: automated
coverage_id: 25-13/D3
requirement: FT-06
verification: crates/paladin-battalion/src/engine/cache_key.rs#tests::key_includes_the_graph_fingerprint ; crates/paladin-battalion/src/engine/cache_key.rs#tests::key_changes_when_the_system_prompt_changes ; crates/paladin-battalion/src/engine/cache_key.rs#tests::key_is_stable_across_runs_for_identical_inputs ; crates/paladin-battalion/src/engine/cache_key.rs#tests::function_node_key_defaults_to_the_full_snapshot ; crates/paladin-battalion/src/engine/cache_key.rs#tests::cache_key_spec_fields_narrows_the_key ; crates/paladin-battalion/src/engine/cache_key.rs#tests::muster_payload_is_included_when_present ; crates/paladin-battalion/src/engine/cache_key.rs#tests::node_prefix_is_exact_under_delimiter_bearing_ids

### 72. 25-13 D4: Hit/miss path: hit merges with zero executions (Succeeded/attempt 1/cache_hit true, one NodeStarted/NodeFinished{cache_hit:true} pair, interceptors bypassed); miss executes once and puts once with the TTL; failures, routing directives and Deny-touching deltas never stored; empty delta is a hit
expected: Hit/miss path: hit merges with zero executions (Succeeded/attempt 1/cache_hit true, one NodeStarted/NodeFinished{cache_hit:true} pair, interceptors bypassed); miss executes once and puts once with the TTL; failures, routing directives and Deny-touching deltas never stored; empty delta is a hit
result: pass
source: automated
coverage_id: 25-13/D4
requirement: FT-06
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_merges_the_stored_delta_with_no_execution ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_emits_node_started_and_finished_with_cache_hit_true ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_miss_executes_and_stores_the_successful_delta_with_the_ttl ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_failed_attempt_is_never_stored ; crates/paladin-battalion/src/engine/superstep.rs#tests::an_empty_cached_delta_is_a_hit_that_merges_nothing ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_function_delta_touching_a_deny_field_is_never_stored ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_bypasses_the_interceptor_chain ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_routing_directive_is_never_stored

### 73. 25-13 D5: Correctness under time, change and backend failure: TTL expiry (paused clock), the closed boundary at exactly expires_at (engine side), prompt change, graph change, put failure -> Completed, get failure -> miss, per-task muster keys, a hit consumes no retry budget
expected: Correctness under time, change and backend failure: TTL expiry (paused clock), the closed boundary at exactly expires_at (engine side), prompt change, graph change, put failure -> Completed, get failure -> miss, per-task muster keys, a hit consumes no retry budget
result: pass
source: automated
coverage_id: 25-13/D5
requirement: FT-06
verification: crates/paladin-battalion/src/engine/superstep.rs#tests::ttl_expiry_re_executes_the_node ; crates/paladin-battalion/src/engine/superstep.rs#tests::an_entry_at_exactly_its_expiry_is_a_miss ; crates/paladin-battalion/src/engine/superstep.rs#tests::changing_the_system_prompt_re_executes ; crates/paladin-battalion/src/engine/superstep.rs#tests::changing_the_graph_re_executes ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_put_failure_leaves_the_run_completed ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_get_failure_is_a_miss_not_an_error ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_cached_node_inside_a_muster_keys_per_task ; crates/paladin-battalion/src/engine/superstep.rs#tests::a_cache_hit_consumes_no_retry_budget

### 74. 25-14 D1: A fault-tolerance guide exists, is registered after the Parley page, states all four limitations (idle-timeout degradation, Append replay hazard, raw-content error_field/Waypoint warning, R-23-01 still accepted), the node-kind matrix and the redact-then-bound rule; every sample compiles; the book builds
expected: A fault-tolerance guide exists, is registered after the Parley page, states all four limitations (idle-timeout degradation, Append replay hazard, raw-content error_field/Waypoint warning, R-23-01 still accepted), the node-kind matrix and the redact-then-bound rule; every sample compiles; the book builds
result: pass
source: automated
coverage_id: 25-14/D1
requirement: FT-01
verification: cargo check -p paladin-doc-examples (exit 0); cd docs && mdbook build (exit 0, 'No broken links found'); docs/src/SUMMARY.md line 25 links user-guides/fault-tolerance.md ; guide contains idle_timeout, Append, error_field, EdgeConditionEvaluator, Battalion, Gate; ! grep -qiE 'R-23-01 (is )?(now )?(closed|mitigated|resolved)' passes

### 75. 25-14 D2: MIGRATION.md 9.2 holds the four FT-owned rows resolved plus a PaladinPort default-method row (N); Y rows and allowlist entries are set-equal in both directions by ci.yml's own script; the Phase 25 deliberate-zero note names every new-in-0.10 type touched; 9.1/9.3/9.4/9.5/9.7 resolved
expected: MIGRATION.md 9.2 holds the four FT-owned rows resolved plus a PaladinPort default-method row (N); Y rows and allowlist entries are set-equal in both directions by ci.yml's own script; the Phase 25 deliberate-zero note names every new-in-0.10 type touched; 9.1/9.3/9.4/9.5/9.7 resolved
result: pass
source: automated
coverage_id: 25-14/D2
requirement: FT-01
verification: scratchpad reproduction of ci.yml's set-equality step: SET-EQUAL PASS, Y rows 5, [[entry]] blocks 5 (FAIL before the crate-name fix) ; 12/12 Task 2 acceptance greps pass (PaladinPort row, 'default method', deliberate zero naming StateNodeError + EngineRegistries, NodeCacheConfig, APP_NODE_CACHE_ENABLED, redis-cache, no 'plumbing-only this phase', 9.7 'no item', CHANGELOG Aegis, audit.toml unchanged)

### 76. 25-14 D3: Traceability rows G-08 and G-10..G-14 name only tests that exist in the named files
expected: Traceability rows G-08 and G-10..G-14 name only tests that exist in the named files
result: pass
source: automated
coverage_id: 25-14/D3
requirement: FT-02
verification: scratchpad verify_matrix_anchors.py parses the written rows: G-08 7, G-10 14, G-11 12, G-12 25, G-13 8, G-14 20 anchors; 86/86 resolve
## Summary

total: 76
passed: 75
issues: 1
pending: 0
skipped: 0
blocked: 0
issues_resolved: 1

## Gaps

- gap_id: G-25-1
  truth: "The ci.yml workflow is green on the phase's final commit, including the API Surface Tracking job, so the CI-only evidence tiers (Redis node cache, Postgres Waypoint) can be read as proof"
  reason: "User reported: API Surface Tracking job fails on run 34042790005 (head 332614d4), same failure as pre-phase run 33965717959; must be fixed before proceeding"
  severity: blocker
  test: 1
  root_cause: "Stale public-API baseline: .project/current-exports.txt was last regenerated at plan 23-12 (2030 items). Phase 24 added 72 public items and phase 25 added 51 more (config::node_cache, heartbeat, run_timeout) without regenerating it, so scripts/check-api-surface.sh diffed 282 added lines (0 removed) and exited 1 on both 4498210b and 332614d4. Not a code defect; purely additive drift."
  artifacts:
    - path: ".project/current-exports.txt"
      issue: "baseline 123 items behind the tree (2030 vs 2153)"
  missing:
    - "Regenerate the baseline with ./scripts/extract-public-api.sh .project/current-exports.txt (cargo-public-api 0.52.0, nightly rustdoc) and commit it"
  debug_session: ""
  status: resolved
  resolved_by: "commit 0e5c106c (inline fix during verify-work; local check-api-surface.sh now reports unchanged at 2153 items, byte-identical to CI's output on run 34042790005)"
  resolved_at: 2026-09-06
  follow_up: "Push the branch so run 34042790005's failure is superseded; the api-surface job must be read green on 0e5c106c before test 4 can pass"
