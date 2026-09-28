---
phase: 39-spend-ledger
verified: 2026-09-28T02:29:03Z
status: passed
score: 4/4 roadmap success criteria verified; 8/8 plans confirmed against codebase
behavior_unverified: 0
overrides_applied: 0
---

# Phase 39: Spend Ledger Verification Report

**Phase Goal:** A durable, race-proof spend ledger exists that every later Treasurer phase can draw
against and query.

**Verified:** 2026-09-28T02:29:03Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Phase 39 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | One shared `TreasuryLedgerPort` contract-test suite passes unmodified against in-memory, SQLite and Postgres adapters, backed by a `007` migration in both `crates/paladin-storage/migrations/{sqlite,postgres}/` | ✓ VERIFIED | `crates/paladin-storage/src/treasury/contract_tests.rs` (21 generic clause functions over `&dyn TreasuryLedgerPort`). Each adapter (`in_memory.rs`, `sqlite.rs`, `postgres.rs`) invokes every clause unchanged from a same-named `#[tokio::test]`. Ran live: `cargo test -p paladin-storage --features sqlite treasury::` → 62 passed, 0 failed (in-memory + SQLite + validation clauses). Postgres: started the sandbox's local Postgres 16 cluster (`pg_ctlcluster 16 main start`, pre-existing `paladin`/`paladin_treasury_test` role/db from the phase's own session) and ran `STORAGE_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5432/paladin_treasury_test cargo test -p paladin-storage --features postgres --lib treasury::postgres -- --test-threads=1 --nocapture` → **23 passed, 0 failed, 0 `SKIP:` lines** — a genuine live run, not a skip. Both `007_create_treasury_ledger_table.sql` files exist (SQLite: TEXT/BIGINT/TEXT-JSON; Postgres: BIGINT/TIMESTAMPTZ/JSONB) with textually-identical table shape, CHECKs and four indexes, including the settlement partial-unique index whose `WHERE kind = 'settle'` predicate textually matches each adapter's `ON CONFLICT` arbiter (confirmed by `settle_arbiter_predicate_matches_the_migration` passing on both SQL adapters). |
| 2 | When N concurrent draws race against a balance that only N−1 of them fit, exactly N−1 succeed and one is refused, on every adapter | ✓ VERIFIED | `reserve_race_admits_exactly_n_minus_one` (16-way race, ceiling 15) is one of the 21 shared clauses; passed live on in-memory, SQLite (`new_shared_file`, on-disk WAL, multiple pooled connections, `BEGIN IMMEDIATE`) and Postgres (`pg_advisory_xact_lock(hashtext($1)::bigint)` taken before the SUM, in the same live Postgres run above). Edge clauses `reserve_admits_at_the_ceiling_and_refuses_one_past_it` and `reserve_rejects_invalid_requests` (checked-add overflow, negative hold/ceiling, currency mismatch) also pass on all three. |
| 3 | Settling the same run/superstep/attempt twice — via lease redelivery, resume, retry or model fallback — charges it exactly once | ✓ VERIFIED | Store-enforced by a partial unique index `(run_id, superstep, attempt) WHERE kind = 'settle'` + `INSERT … ON CONFLICT … DO NOTHING` on both SQL adapters, and a `HashSet<SettlementKey>` on in-memory — never an application-side check-then-act. `duplicate_settle_is_already_settled_and_charges_once`, `bumped_attempt_is_a_distinct_settlement` and the 10-way `concurrent_duplicate_settles_charge_once` clause all pass on every adapter (live runs above). Production wiring confirmed: engine path settles once per superstep at the boundary (`settlement.rs`'s `settle_boundary`, called after `completed_records` is sorted and before any outcome branch persists the Waypoint — read directly at `superstep.rs` line ~3467); a nested `NodeSpec::Battalion` child never settles on its own (`SpendHook::child`, `ledger: None`, proven by `child_settle_boundary_makes_no_ledger_call_and_leaves_the_accumulator_intact`). Agent loop settles once per priced call (`settle_agent_loop_call`), with `AgentLoopSettlement::PlatformRunsOnly` never settling an engine node's internal dispatch (no run id in scope) so engine and agent-loop writers can never double-charge the same call — read directly in `paladin_execution_service.rs::ledger_run_id`. Worker wiring (`RunWorkerPool::with_treasury_ledger`) passes the *persisted* `Run.attempt` (bumped by `bump_attempt` on redelivery, by `record_resume` on resume) into the engine's `SettlementContext`, never inventing a counter — proven by `engine_run_settles_under_its_run_id_and_first_attempt` and `redelivered_running_run_settles_under_the_bumped_attempt` (both pass live). A ledger error/`AlreadySettled` is logged at `error`/`warn` and never fails, retries or halts a run on any of the three writers — confirmed by reading `settlement.rs::settle_boundary`'s match arms and by `pool_without_a_treasury_ledger_settles_nothing` passing. |
| 4 | Operator can view spend per tenant, API key, run and model over a time window from the CLI, and that spend appears in herald output and trace events | ✓ VERIFIED | CLI: `paladin-cli treasury spend` (`src/application/cli/commands/treasury.rs`, 502 lines) reads `RunStoreConfig` directly (no server), supports `--group-by tenant\|api-key\|run\|model`, `--since`/`--until`, `--format table\|json`; live tests `treasury_spend_tracer_reads_settlements_from_sqlite_ledger`, `treasury_spend_prints_one_row_per_currency`, `treasury_spend_empty_window_prints_no_spend_line`, `treasury_spend_rejects_inverted_window` all pass (`cargo test -p paladin-ai --lib --features "cli storage-postgres" treasury::` → 4 passed). `format_cost` renders `0.0450 USD`, byte-identical to `ExecutionMetadata::cost_display` (regression test `format_cost_matches_the_herald_cost_display` passes; doctest passes). HTTP surface: `GET /runs/{id}`/`GET /runs` derive `cost` from the ledger at read time (`run_controller.rs`, one `spend()` call per request/page, `null` on no ledger / no settlements / mixed currency — never persisted, never combined) — `get_run_includes_ledger_cost`, `get_run_cost_is_null_for_mixed_currencies`, `get_run_cost_is_null_when_the_ledger_errors`, `list_runs_derives_costs_with_one_spend_call_per_page` all pass; `POST /agents/{id}/execute` carries `cost` from `PaladinResult.cost` (`execute_response_carries_cost_when_priced`/`execute_response_cost_is_null_when_unpriced` pass, replacing the Phase 38 deferral test). Herald/trace: this phase adds no new field or `TraceEvent` variant by design (D-08 scope note) — confirmed unchanged: `run_finished_cost_sums_priced_paladin_nodes` and `sse_payloads_carry_no_spend_field` both still pass live. End-to-end proof: `priced_run_cost_reaches_get_run_through_the_ledger` (a run submitted through the worker → settled through the ledger → read back via `GET /v1/runs/{id}` as `0.0450 USD`) passes. |

**Score:** 4/4 roadmap success criteria verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-core/src/platform/container/treasury_ledger.rs` | `LedgerScope`, `SettlementKey`, `ReservationId`, `LedgerEntryKind`, `Reserve/SettleRequest`, `SettleOutcome`, `SpendGroupBy/Query/Row`, `SettlementContext`, `format_cost` | ✓ VERIFIED | 483 lines; all types present; 6 doctests + unit tests pass |
| `crates/paladin-ports/src/output/treasury_ledger_port.rs` | `TreasuryLedgerPort` trait (reserve/settle/release/spend/store_now), `TreasuryLedgerError` (Refused/CurrencyMismatch/UnknownReservation), compiling rustdoc mock | ✓ VERIFIED | 324 lines; full surface present; rustdoc mock doctest passes |
| `crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql` | `treasury_ledger` table + 4 indexes | ✓ VERIFIED | Present, matches D-01/D-02/D-06/D-11/D-12 exactly, applied on construction via embedded migrator |
| `crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql` | Postgres twin (BIGINT/TIMESTAMPTZ/JSONB) | ✓ VERIFIED | Present, textually-matching predicate, identical index set |
| `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs` | Three adapters + shared contract suite | ✓ VERIFIED | 5 files, 3169 combined lines; 21 clauses × 3 adapters all pass live (in-memory/SQLite: 62 tests; Postgres: 23 tests, live server, 0 SKIP) |
| `src/application/cli/commands/treasury.rs` | `treasury spend` CLI verb | ✓ VERIFIED | 502 lines; SQLite + Postgres arms; 4 CLI tests pass |
| `crates/paladin-battalion/src/engine/settlement.rs` | `SuperstepSpend`, `SpendHook` | ✓ VERIFIED | 342 lines; superstep-boundary settle wired into `superstep.rs`/`mod.rs`/`graph.rs`; 6 settlement tests + 561 engine tests (full crate) pass, 0 regressions |
| `crates/paladin-core/src/platform/container/run_scope.rs` | `RunScope.run_id` + `with_run_id` | ✓ VERIFIED | Present, additive, `#[non_exhaustive]` intact |
| `src/application/services/paladin/paladin_execution_service.rs` | `AgentLoopSettlement`, `with_treasury_ledger` | ✓ VERIFIED | Present; 71 service tests pass incl. `streamed_priced_call_settles_once` |
| `src/infrastructure/web/{agent_host,facade_provisioner}.rs` | `build_agent_registry_with_ledger`, `with_treasury_ledger`, `execute_scoped` forwarding | ✓ VERIFIED | Present at cited line numbers |
| `crates/paladin-web/src/{run_controller,agent_controller}.rs` | `CostDto`, ledger-derived `RunResponse.cost`/`ExecuteResponse.cost` | ✓ VERIFIED | Present; 233/233 paladin-web lib tests pass incl. openapi baseline; 7/7 golden-v0.9 tests pass |
| `src/application/services/run/worker.rs` | `RunWorkerPool::with_treasury_ledger` | ✓ VERIFIED | Present; 32/32 worker tests pass |
| `src/infrastructure/web/run_api_wiring.rs` | `build_treasury_ledger` | ✓ VERIFIED | Present; 8/8 tests pass with `--features web-server` |
| `src/application/services/run/tracer_e2e.rs` | `priced_run_cost_reaches_get_run_through_the_ledger` | ✓ VERIFIED | Present; 5/5 tracer_e2e tests pass |
| `CHANGELOG.md`, `.project/current-exports.txt`, `src/core/platform/mod.rs`, `MIGRATION.md` | Release-record entries | ✓ VERIFIED | `TreasuryLedgerPort`/`treasury_ledger` present in CHANGELOG, current-exports.txt, facade re-export (`pub use paladin_core::platform::container::treasury_ledger`); MIGRATION.md §9.2 rows for `ExecuteResponse` (extended) and `RunResponse` (new-in-0.10, N/A) present with `LEDGR-04` cited |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `treasury.rs` (CLI) | `sqlite.rs`/`postgres.rs` | `RunStoreConfig` selects backend, opens adapter directly | ✓ WIRED | `build_postgres_treasury_ledger` / `SqliteTreasuryLedger::new`, cfg-gated exactly like `run.rs` |
| `sqlite.rs`/`postgres.rs` | `007_*.sql` migrations | embedded `sqlx::migrate!` + `ON CONFLICT … WHERE kind = 'settle'` | ✓ WIRED | `settle_arbiter_predicate_matches_the_migration` passes on both |
| `superstep.rs` | `settlement.rs` | `hook.record()` per attempt, `hook.settle_boundary().await` at the boundary | ✓ WIRED | Read directly; test `one_settlement_per_superstep_attempt_across_parallel_nodes_and_retries` passes |
| `settlement.rs` | `treasury_ledger_port.rs` | `TreasuryLedgerPort::settle` (unreserved) | ✓ WIRED | Read directly in `settle_boundary` |
| `superstep.rs` (child) | `settlement.rs` (`SpendHook::child`) | `ChildEngineResources.spend` | ✓ WIRED | `child_settle_boundary_makes_no_ledger_call_and_leaves_the_accumulator_intact` passes |
| `paladin_execution_service.rs` | `treasury_ledger_port.rs` | `settle_agent_loop_call` after `cost_tally.record_call` | ✓ WIRED | `streamed_priced_call_settles_once` passes |
| `facade_provisioner.rs` | `paladin_execution_service.rs` | `EngineExecutionPort::execute_scoped` forwards `RunScope` | ✓ WIRED | Read directly; `a_paladin_node_receives_the_same_grant_through_execute_scoped`-style coverage present |
| `run_controller.rs` | `treasury_ledger_port.rs` | `spend(SpendQuery{group_by: Run, run_ids, ..})` | ✓ WIRED | One call per request/page — `list_runs_derives_costs_with_one_spend_call_per_page` passes |
| `worker.rs` | `engine/mod.rs` | `engine.with_treasury_ledger(ledger, SettlementContext{..attempt})` | ✓ WIRED | `engine_run_settles_under_its_run_id_and_first_attempt`, `redelivered_running_run_settles_under_the_bumped_attempt` pass |
| `run_api_wiring.rs` | `facade_provisioner.rs` / `agent_host.rs` | `paladin_port_from_settings_with_ledger` / `build_agent_registry_with_ledger` | ✓ WIRED | `build_run_api_wires_the_treasury_ledger` passes |

### Behavioral Spot-Checks / Live Test Runs

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Shared contract suite, in-memory + SQLite | `cargo test -p paladin-storage --features sqlite treasury::` | 62 passed, 0 failed | ✓ PASS |
| Shared contract suite, Postgres (live server started in this verification session) | `STORAGE_POSTGRES_TEST_URL=... cargo test -p paladin-storage --features postgres --lib treasury::postgres -- --test-threads=1 --nocapture` | 23 passed, 0 failed, 0 `SKIP:` lines | ✓ PASS |
| Engine settlement + full engine regression | `cargo test -p paladin-battalion --lib engine::` | 561 passed, 0 failed | ✓ PASS |
| Agent-loop settlement | `cargo test -p paladin-ai --lib paladin_execution_service::` | 71 passed, 0 failed | ✓ PASS |
| Worker ledger wiring | `cargo test -p paladin-ai --lib worker::` | 32 passed, 0 failed | ✓ PASS |
| End-to-end run→ledger→GET /runs | `cargo test -p paladin-ai --lib tracer_e2e::` | 5 passed, 0 failed | ✓ PASS |
| run_api_wiring ledger builder (needs `web-server` feature) | `cargo test -p paladin-ai --lib --features web-server run_api_wiring::` | 8 passed, 0 failed | ✓ PASS |
| CLI `treasury spend` | `cargo test -p paladin-ai --lib --features "cli storage-postgres" treasury::` | 4 passed, 0 failed | ✓ PASS |
| Web HTTP cost surface + OpenAPI baseline | `cargo test -p paladin-web --lib` / `--test openapi_golden_v0_9` | 233 passed / 7 passed | ✓ PASS |
| Herald/trace regression (unchanged) | `cargo test -p paladin-battalion --lib engine::tests::run_finished_cost_sums_priced_paladin_nodes` / `cargo test -p paladin-ai --lib sse_payloads_carry_no_spend_field` | pass | ✓ PASS |
| Doctests | `cargo test -p paladin-ai-core --doc treasury_ledger` / `cargo test -p paladin-ports --doc treasury_ledger` | 6 passed / 1 passed | ✓ PASS |
| Lint | `cargo fmt --check`; `cargo clippy -p paladin-storage --features "sqlite postgres" -- -D warnings`; `cargo clippy -p paladin-battalion -- -D warnings`; `cargo clippy -p paladin-web -- -D warnings` | clean | ✓ PASS |
| Debt markers | `grep -nE "TBD|FIXME|XXX"` over every file this phase created/modified | none found | ✓ PASS |
| Tree freshness vs. the SUMMARY-cited full-workspace pass (commit `980caf94`) | `git diff --stat 980caf94 HEAD` | only `.planning/` docs changed (ROADMAP.md, STATE.md, 39-08-SUMMARY.md) | ✓ PASS — no code drift since the recorded full-suite green run |

Note: the Postgres contract suite took the `SKIP:` path when this session's own shell first ran it (`STORAGE_POSTGRES_TEST_URL` unset, matching the sandbox default noted in the task's runtime_paths). Rather than accept the SKIP as sufficient, this verification started the sandbox's pre-existing local Postgres 16 cluster and re-ran the suite live — 23/23 passed with 0 `SKIP:` lines — independently corroborating 39-03-SUMMARY.md's own claimed live run rather than trusting it on narrative alone.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LEDGR-01 | 39-01, 39-02, 39-03, 39-08 | `TreasuryLedgerPort` + 3 adapters, shared contract suite, `007` migration | ✓ SATISFIED | See Truth #1 |
| LEDGR-02 | 39-02, 39-03, 39-08 | Race-proof reserve-then-settle, N−1-of-N | ✓ SATISFIED | See Truth #2 |
| LEDGR-03 | 39-02, 39-03, 39-04, 39-05, 39-07, 39-08 | Settlement idempotency, keyed on run/superstep/attempt | ✓ SATISFIED | See Truth #3 |
| LEDGR-04 | 39-01, 39-04, 39-05, 39-06, 39-07, 39-08 | Queryable spend (CLI + HTTP), herald/trace unchanged | ✓ SATISFIED | See Truth #4 |

No orphaned requirements: `.planning/REQUIREMENTS.md` maps all four LEDGR-* IDs to Phase 39 only, and all four appear in at least one plan's frontmatter `requirements:` list (confirmed by grep across all 8 `*-PLAN.md` files).

### Anti-Patterns Found

None. Scanned every file this phase created or modified (`treasury_ledger.rs`, `treasury_ledger_port.rs`, all five `treasury/*.rs` storage files, both `007_*.sql` migrations, `treasury.rs` CLI, `settlement.rs`, `run_scope.rs`, `paladin_execution_service.rs`, `agent_host.rs`, `facade_provisioner.rs`, `run_controller.rs`, `agent_controller.rs`, `worker.rs`, `run_api_wiring.rs`, `tracer_e2e.rs`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`/"not yet implemented" — zero matches. No `UPDATE`/`DELETE` SQL statements in any of the three ledger adapters (append-only invariant holds structurally, not just by convention). Every ledger-failure path logs and returns/continues rather than propagating (`settle_boundary`'s `Err` arm, `settle_agent_loop_call`'s equivalent, `get_run`/`list_runs`'s degrade-to-null path) — read directly, not inferred from the SUMMARY.

### Prohibitions Check (all plans' `must_haves.prohibitions`)

All 13 prohibitions across the 8 plans (append-only invariant on all 3 adapters; no currency conversion/netting; Postgres SKIP-path honesty; observational-failure discipline on the engine and agent-loop writers; no double-charge between the engine superstep writer and the `PlatformRunsOnly` agent-loop writer; HTTP cost never persisted/combined/zero-stand-in; ledger read failure never fails a run read; wiring never changes run status transitions; phase not sealed on an unverified Postgres/coverage claim) were checked directly against source and/or a live test run rather than accepted from the plan text. No violation found; see the truths and key-links above for the specific evidence each maps to.

### Minor Documentation Note (non-blocking)

`.planning/phases/39-spend-ledger/39-VALIDATION.md` still shows `status: draft`, `nyquist_compliant: false`, `wave_0_complete: false`, and every per-task row marked `⬜ pending` — this tracking document was never updated after execution (`/gsd-validate-phase 39` appears not to have been run). This is a process/tracking gap, not a functional one: every command that document lists as the per-task verification was independently run in this verification session (or a stronger live equivalent, e.g. the Postgres suite against a real server rather than the doc's own quick-run command) and passed. Does not block phase completion; flagged for the operator's awareness only.

### Human Verification Required

None. Every roadmap success criterion, artifact, key link and prohibition was verifiable by direct source inspection and/or a live, currently-passing test run in this session (including a from-scratch live Postgres run against a real server, not merely trusting the SUMMARY's account of one). No behavior-dependent truth was left unexercised.

### Gaps Summary

No gaps. All 4 roadmap success criteria are VERIFIED with direct evidence (live test runs performed in this verification session, not merely SUMMARY claims), all 4 requirement IDs are SATISFIED, all 8 plans' artifacts and key links are present and wired, no anti-patterns or debt markers exist in phase-touched files, and the one non-code documentation gap (39-VALIDATION.md left in `draft`) is recorded as a non-blocking note rather than a gap.

---

_Verified: 2026-09-28T02:29:03Z_
_Verifier: Claude (gsd-verifier)_
