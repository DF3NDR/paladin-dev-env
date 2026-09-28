---
status: complete
phase: 39-spend-ledger
source: [39-01-SUMMARY.md, 39-02-SUMMARY.md, 39-03-SUMMARY.md, 39-04-SUMMARY.md, 39-05-SUMMARY.md, 39-06-SUMMARY.md, 39-07-SUMMARY.md, 39-08-SUMMARY.md]
started: 2026-09-28T14:00:10Z
updated: 2026-09-28T15:41:35Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running paladin-server. Delete the ephemeral SQLite run-store file (or point APP_RUN_STORE_PATH at a fresh path) so no `treasury_ledger` table exists yet. Start `paladin-server` with `APP_RUN_STORE_BACKEND=sqlite`. The server boots without errors, the embedded migrator applies `007_create_treasury_ledger_table` on first open, `GET /v1/runs` returns 200 with an empty list, and `paladin-cli treasury spend --format json` against the same file prints `[]` rather than failing.
result: pass

### 2. One-way 007 schema decision was yours
expected: The `treasury_ledger` schema shipped in `crates/paladin-storage/migrations/{sqlite,postgres}/007_create_treasury_ledger_table.sql` (scope columns with the `unattributed` sentinel, `breakdown` column, attribution-instant column, partial unique settlement index `(run_id, superstep, attempt) WHERE kind = 'settle'`) is exactly the option-a design you approved at the 39-01 Task 1 checkpoint. Nothing was changed after your sign-off without your knowledge.
result: pass
coverage_id: 39-01/D4
rationale: Operator sign-off on a one-way schema decision is inherently a human judgment call, not something a test can classify.

### 3. Credential-handling review holds up
expected: Reading `crates/paladin-storage/src/treasury/{sqlite,postgres}.rs` you see every connection error routed through `redact_database_url_password` before any other handling; the settle log lines in `crates/paladin-battalion/src/engine/settlement.rs` and `paladin_execution_service.rs` interpolate only run_id/superstep/attempt/ordinal/nanos/currency, never `api_key_id` or `tenant_id`; and no new `reqwest::Client` appears anywhere in the phase diff. The findings recorded in 39-08-SUMMARY.md §Credential-handling review match what you see.
result: pass
coverage_id: 39-08/D8
rationale: Manual source-inspection review per security.instructions.md; no merge-gating Rust SAST exists.

### 4. TreasuryLedgerPort domain types and trait exist in paladin-core/paladin-ports
expected: Ledger value types, port trait, error enum and compiling rustdoc mock present and tested.
result: pass
source: automated
coverage_id: 39-01/D1

### 5. 007 SQLite migration and SqliteTreasuryLedger settle a spend
expected: Bound parameters, partial unique index, ON CONFLICT DO NOTHING; 14 storage tests pass.
result: pass
source: automated
coverage_id: 39-01/D2

### 6. paladin-cli treasury spend reads settlements from the SQLite ledger
expected: CLI tracer test passes; empty migrated file prints `[]`.
result: pass
source: automated
coverage_id: 39-01/D3

### 7. LEDGR-02 N-1-of-N race clause (16 concurrent reserves → 15 Ok / 1 Refused) on in-memory and on-disk SQLite
expected: `reserve_race_admits_exactly_n_minus_one` passes on both adapters.
result: pass
source: automated
coverage_id: 39-02/D1

### 8. reserve/release/settle follow ADR-0053 signed-contribution balance
expected: 62 sqlite-feature storage tests pass.
result: pass
source: automated
coverage_id: 39-02/D2

### 9. Settlement idempotency: duplicate SettlementKey charged once, concurrent duplicates charged once
expected: `concurrent_duplicate_settles_charge_once_on_disk` and 21-clause in-memory suite pass.
result: pass
source: automated
coverage_id: 39-02/D3

### 10. spend groups by tenant/api-key/run/model with window and ordering
expected: Storage spend tests and port doctest pass.
result: pass
source: automated
coverage_id: 39-02/D4

### 11. Postgres 007 migration twins the approved SQLite schema
expected: `settle_arbiter_predicate_matches_the_migration` passes; BIGINT/TIMESTAMPTZ/JSONB, four indexes.
result: pass
source: automated
coverage_id: 39-03/D1

### 12. PostgresTreasuryLedger serializes reserve per scope via advisory lock
expected: 16-way race clause and 23 postgres tests pass live.
result: pass
source: automated
coverage_id: 39-03/D2

### 13. Postgres settlement idempotency store-enforced by partial unique index
expected: 10-way concurrent duplicate settle and duplicate-settle clauses pass live.
result: pass
source: automated
coverage_id: 39-03/D3

### 14. Full 21-clause contract suite passes unmodified on Postgres (0 SKIP lines)
expected: Live run with STORAGE_POSTGRES_TEST_URL, 23 passed, no `SKIP:`.
result: pass
source: automated
coverage_id: 39-03/D4

### 15. paladin-cli treasury spend Postgres arm reads URL from configured env
expected: `cargo build --features "cli storage-postgres" --bin paladin-cli` succeeds; cfg gating present.
result: pass
source: automated
coverage_id: 39-03/D5

### 16. SuperstepSpend accumulates priced attempts and per-model share
expected: settlement unit tests for accumulate/take/mismatch/model-key pass.
result: pass
source: automated
coverage_id: 39-04/D1

### 17. WarEngine::with_treasury_ledger settles exactly once per superstep with model breakdown
expected: `treasury_ledger_settles_once_per_superstep_with_model_breakdown` and doctest pass.
result: pass
source: automated
coverage_id: 39-04/D2

### 18. Unpriced superstep writes no settlement; engine without ledger unchanged
expected: 562 engine tests pass, 0 failed.
result: pass
source: automated
coverage_id: 39-04/D3

### 19. Nested Battalion child spend rolls into the parent superstep
expected: `nested_battalion_child_spend_rolls_into_the_parent_superstep` passes.
result: pass
source: automated
coverage_id: 39-04/D4

### 20. One settlement per superstep attempt across parallel nodes and retries
expected: `one_settlement_per_superstep_attempt_across_parallel_nodes_and_retries` passes.
result: pass
source: automated
coverage_id: 39-04/D5

### 21. Ledger settle failure never fails, retries or halts a run
expected: `failing_treasury_ledger_never_fails_the_run` passes.
result: pass
source: automated
coverage_id: 39-04/D6

### 22. Pre-existing settlement at a key is kept unchanged (AlreadySettled is not an error)
expected: `already_settled_superstep_is_not_an_error` passes.
result: pass
source: automated
coverage_id: 39-04/D7

### 23. cargo check -p paladin-battalion (no dev-dependencies) exits 0
expected: paladin-storage is not a runtime dependency of the engine crate.
result: pass
source: automated
coverage_id: 39-04/D8

### 24. RunScope.run_id is additive and serde-omitted when None
expected: run_scope unit tests and doctests pass.
result: pass
source: automated
coverage_id: 39-05/D1

### 25. PaladinExecutionService::with_treasury_ledger settles per priced call
expected: agent_loop_cost_tests pass.
result: pass
source: automated
coverage_id: 39-05/D2

### 26. Agent-loop run id is RunScope.run_id when supplied, else execution id
expected: execution service tests pass.
result: pass
source: automated
coverage_id: 39-05/D3

### 27. PlatformRunsOnly settles only calls whose RunScope names a run (no engine double-charge)
expected: execution service and facade_provisioner tests pass.
result: pass
source: automated
coverage_id: 39-05/D4

### 28. Ledger Err never fails, retries or alters an agent-loop execution
expected: `ledger_failure_never_fails_the_agent_loop` passes.
result: pass
source: automated
coverage_id: 39-05/D5

### 29. Streamed priced call settles once at its terminal chunk
expected: `streamed_priced_call_settles_once` passes.
result: pass
source: automated
coverage_id: 39-05/D6

### 30. build_agent_with_llm/build_agent install EveryCall when a ledger is supplied
expected: agent_host tests pass with --features web-server.
result: pass
source: automated
coverage_id: 39-05/D7

### 31. FacadeProvisioner::with_treasury_ledger prices and settles provisioned Paladins
expected: facade_provisioner tests pass; paladin-server builds.
result: pass
source: automated
coverage_id: 39-05/D8

### 32. GET /runs/{run_id} returns ledger-derived cost (null on no ledger, error, mixed currency)
expected: four run_controller get_run cost tests pass.
result: pass
source: automated
coverage_id: 39-06/D1

### 33. GET /runs derives every item's cost from exactly one spend call per page
expected: `list_runs_derives_costs_with_one_spend_call_per_page` passes.
result: pass
source: automated
coverage_id: 39-06/D2

### 34. POST /agents/{id}/execute carries cost from PaladinResult.cost
expected: `execute_response_carries_cost_when_priced` / `_is_null_when_unpriced` pass.
result: pass
source: automated
coverage_id: 39-06/D3

### 35. OpenAPI baseline documents CostDto/cost; SSE payloads carry no spend field
expected: openapi baseline, golden v0.9 and `sse_payloads_carry_no_spend_field` pass.
result: pass
source: automated
coverage_id: 39-06/D4

### 36. RunWorkerPool::with_treasury_ledger settles under the run id and first attempt
expected: `engine_run_settles_under_its_run_id_and_first_attempt` passes.
result: pass
source: automated
coverage_id: 39-07/D1

### 37. Running redelivery bumps attempt and settles under the bumped key
expected: `redelivered_running_run_settles_under_the_bumped_attempt` passes.
result: pass
source: automated
coverage_id: 39-07/D2

### 38. Agent-kind Platform run passes its run id in the RunScope
expected: `agent_kind_run_passes_its_run_id_in_the_run_scope` passes.
result: pass
source: automated
coverage_id: 39-07/D3

### 39. Pool without a treasury ledger performs no ledger call
expected: `pool_without_a_treasury_ledger_settles_nothing` passes; 148 run-service tests pass.
result: pass
source: automated
coverage_id: 39-07/D4

### 40. End-to-end: POST /v1/runs → ledger → GET /v1/runs/{id} shows 0.0450 USD
expected: `priced_run_cost_reaches_get_run_through_the_ledger` passes.
result: pass
source: automated
coverage_id: 39-07/D5

### 41. build_treasury_ledger builds the ledger from RunStoreConfig alone
expected: run_api_wiring disabled/sqlite/wires tests pass.
result: pass
source: automated
coverage_id: 39-07/D6

### 42. paladin-server builds RunStoreConfig and the ledger ahead of the run API
expected: paladin-server bin tests pass; storage-postgres build succeeds.
result: pass
source: automated
coverage_id: 39-07/D7

### 43. All 11 CI packages pass cargo semver-checks against v0.9.0; allowlist set-equal; check-gates passes
expected: semver loop, check-migration-allowlist.sh and make check-gates exit 0.
result: pass
source: automated
coverage_id: 39-08/D1

### 44. v0.10.1 baseline measurement registered for the 6 changed packages
expected: semver-checks --release-type minor run recorded in MIGRATION.md §9.2.
result: pass
source: automated
coverage_id: 39-08/D2

### 45. MIGRATION.md §9.2 ExecuteResponse row extended; RunResponse row added (N/A)
expected: allowlist stays set-equal (16 pairs).
result: pass
source: automated
coverage_id: 39-08/D3

### 46. CHANGELOG.md and five crate CHANGELOGs carry [Unreleased] Phase 39 entries
expected: one `## [Unreleased]` in root; per-crate treasury/cost bullets present.
result: pass
source: automated
coverage_id: 39-08/D4

### 47. Facade re-exports treasury_ledger; API baseline refreshed; make api-surface exits 0
expected: current-exports.txt refreshed with nightly-2026-09-20; TreasuryLedgerPort present.
result: pass
source: automated
coverage_id: 39-08/D5

### 48. Final tree passes every commit gate (test, fmt, clean-code, security, check-gates, openapi no drift)
expected: all gates exit 0 on the phase's final commit.
result: pass
source: automated
coverage_id: 39-08/D6

### 49. Herald/trace clause unchanged from Phase 38
expected: `run_finished_cost_sums_priced_paladin_nodes` and paladin-herald suite pass; trace.rs/herald.rs untouched.
result: pass
source: automated
coverage_id: 39-08/D7

## Summary

total: 49
passed: 49
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
