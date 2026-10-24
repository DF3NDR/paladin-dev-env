---
phase: 40-tenant-identity-run-read-scoping
plan: 04
subsystem: treasury-ledger
tags: [ledger-scope, tenant-attribution, run-worker, agent-execute, executor-port, axum]

# Dependency graph
requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: "40-01: TenantId, RunAttribution, PrincipalRef, Run.submitted_by, Principal { id, role, tenant_id }"
  - phase: 39-spend-ledger
    provides: "LedgerScope, SettlementContext, TreasuryLedgerPort, the worker/agent-loop settle writers stamping the sentinel"
provides:
  - "LedgerScope::from_attribution(Option<&RunAttribution>) -- the single attribution-to-scope mapping (D-15)"
  - "RunScope.ledger_scope + RunScope::with_ledger_scope (additive, serde-defaulted, D-16)"
  - "PaladinExecutorPort::execute_scoped and StreamingExecutorPort::execute_stream_scoped -- defaulted, scope-ignoring delegates (X-10.4)"
  - "PaladinExecutionService overrides of both; AgentLoopLedgerTarget { run_id, scope } through the buffered loop; scoped stream settlement"
  - "Principal::ledger_scope() and the execute / stream (both branches) / jobs handlers passing it"
  - "Worker engine SettlementContext.scope and agent-kind RunScope built from run.submitted_by"
affects: [40-05, 40-06, 41-allowances]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One mapping function (LedgerScope::from_attribution) on every writer path; callers never spell (tenant, api_key_id) themselves"
    - "Defaulted trait method as a correct claim of no capability (PaladinPort::execute_scoped / insert_with_latest precedent) -- overriding is how an implementor opts in"
    - "Resolve ledger identity once per execution into a private target struct, carry it unchanged through the loop"
    - "Scope built only from the authenticated Principal, never from the request body (T-40-16)"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/treasury_ledger.rs
    - crates/paladin-core/src/platform/container/run_scope.rs
    - src/application/services/run/worker.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - crates/paladin-ports/src/output/paladin_executor_port.rs
    - crates/paladin-ports/src/output/streaming_executor_port.rs
    - crates/paladin-web/src/agent_auth.rs
    - crates/paladin-web/src/agent_controller.rs
    - src/infrastructure/web/agent_host.rs

key-decisions:
  - "An unattributed agent-kind run's RunScope carries Some(LedgerScope::unattributed()), not None -- the worker always goes through from_attribution, so the agent loop's None-fallback is reached only by callers that never pass a scope (embedded library use)"
  - "The streamed path's scope rides as a third tuple element in stream_settlement, resolved once before the spawn, so the spawned task still owns no &self"
  - "Trait override calls the inherent method by fully qualified path (PaladinExecutionService::execute_scoped(self, .., None, scope)) so name resolution cannot recurse into the trait method"
  - "API-surface baseline and MIGRATION.md 9.2 rows for RunScope / PaladinExecutorPort / StreamingExecutorPort are left to 40-06 as the phase's artifact table assigns them"

patterns-established:
  - "LedgerScope::from_attribution is the only place a recorded principal becomes a ledger scope; grep for LedgerScope::new outside it should find only tests"
  - "ScopeRecordingExecutor / ScopeRecordingStreamer test doubles in agent_controller.rs pin what scope each handler hands its executor"

requirements-completed: [TENANT-02]

coverage:
  - id: D1
    description: "LedgerScope::from_attribution maps Some(a) to (a.tenant_id, a.api_key_id) and None to the unattributed sentinel"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/treasury_ledger.rs#from_attribution_maps_tenant_and_api_key"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/treasury_ledger.rs#from_attribution_none_is_the_unattributed_sentinel"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --doc treasury_ledger"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunScope.ledger_scope defaults to None, is set by with_ledger_scope, and is omitted from the serialized form when None"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/run_scope.rs#run_scope_default_has_no_ledger_scope"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/run_scope.rs#with_ledger_scope_sets_the_scope"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/run_scope.rs#default_scope_serializes_without_a_ledger_scope_key"
        status: pass
    human_judgment: false
  - id: D3
    description: "Worker engine runs settle under the run row's recorded submitter; principal-less runs settle under the sentinel; agent-kind runs carry the attribution in their RunScope"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "src/application/services/run/worker.rs#attributed_engine_run_settles_under_its_submitting_principal_scope"
        status: pass
      - kind: unit
        ref: "src/application/services/run/worker.rs#unattributed_engine_run_settles_under_the_unattributed_sentinel"
        status: pass
      - kind: unit
        ref: "src/application/services/run/worker.rs#agent_kind_run_carries_its_attribution_in_the_run_scope"
        status: pass
    human_judgment: false
  - id: D4
    description: "PaladinExecutionService settles agent-loop calls under scope.ledger_scope (buffered and streamed), falling back to the sentinel only when the scope carries none"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#agent_loop_settles_under_the_run_scope_ledger_scope"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#agent_loop_without_a_ledger_scope_settles_unattributed"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#executor_port_execute_scoped_settles_under_the_scope_ledger_scope"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#stream_scoped_settles_under_the_scope_ledger_scope"
        status: pass
    human_judgment: false
  - id: D5
    description: "Defaulted PaladinExecutorPort::execute_scoped / StreamingExecutorPort::execute_stream_scoped keep every existing implementor compiling and behaving unchanged"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ports --doc executor_port"
        status: pass
      - kind: other
        ref: "cargo clippy --workspace --all-targets --all-features -- -D warnings (StubExecutor, doc-examples MockExecutor, handoff executors unchanged)"
        status: pass
    human_judgment: false
  - id: D6
    description: "POST /v1/agents/{id}/execute, /execute/stream (streamed and buffered-fallback) and /jobs hand the executor the calling principal's ledger scope; open access settles under (open-access, anonymous)"
    requirement: TENANT-02
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/agent_auth.rs#ledger_scope_is_the_principals_tenant_and_id"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#execute_agent_attributes_spend_to_the_callers_principal"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#execute_agent_stream_attributes_spend_to_the_callers_principal"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#execute_agent_stream_fallback_attributes_spend_to_the_callers_principal"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#enqueue_job_attributes_spend_to_the_callers_principal"
        status: pass
      - kind: unit
        ref: "crates/paladin-web/src/agent_controller.rs#open_access_execute_attributes_spend_to_the_open_access_tenant"
        status: pass
    human_judgment: false
  - id: D7
    description: "The 007 ledger migrations, TreasuryLedgerPort, ledger adapters and the treasury CLI are byte-unchanged from 629ef660 (D-00a/D-17)"
    requirement: TENANT-02
    verification:
      - kind: other
        ref: "git diff --quiet 629ef660 -- crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql crates/paladin-ports/src/output/treasury_ledger_port.rs crates/paladin-storage/src/treasury src/application/cli/commands/treasury.rs"
        status: pass
    human_judgment: false

# Metrics
duration: 17min
completed: 2026-09-29
status: complete
---

# Phase 40 Plan 04: Ledger Scope Source Swap Summary

**Every ledger settlement now names the tenant and API key of the principal that caused it -- the run row's recorded submitter on the worker path (engine superstep and agent-kind), the authenticated caller on the HTTP agent-execute routes -- through one core mapping (`LedgerScope::from_attribution`), one additive `RunScope.ledger_scope` field and two defaulted executor-port methods; the Phase 39 `unattributed` sentinel is stamped only where no principal exists.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-09-29T00:38:11Z
- **Completed:** 2026-09-29T00:55:38Z
- **Tasks:** 2 (both TDD: RED commit + GREEN commit each)
- **Files modified:** 9

## Accomplishments

- `LedgerScope::from_attribution(Option<&RunAttribution>)` in `paladin-core` is the single place a recorded principal becomes a ledger scope (D-15); `None` yields the sentinel (D-10). `LedgerScope`'s and `UNATTRIBUTED`'s "until Phase 40" rustdoc is rewritten to the present tense.
- `RunScope` gains `ledger_scope: Option<LedgerScope>` (serde-defaulted, `skip_serializing_if`) and `with_ledger_scope` (D-16), documented in the module's forward-compatibility paragraph.
- The run worker builds both the engine `SettlementContext.scope` and the agent-kind `RunScope` from `run.submitted_by` via `from_attribution`; the two "until Phase 40" rustdoc passages describe the submitter-derived scope. Production `worker.rs` no longer spells `LedgerScope::unattributed()` anywhere.
- `PaladinExecutionService` resolves the scope once in `execute_scoped` (`scope.ledger_scope` or the sentinel) into a private `AgentLoopLedgerTarget { run_id, scope }` carried through `execute_bounded`/`execute_internal`; `settle_agent_loop_call` and `settle_model_call` take the resolved scope. The streamed path (`execute_stream_inner`) gains a `ledger_scope: Option<LedgerScope>` argument and settles under it.
- `PaladinExecutorPort::execute_scoped` and `StreamingExecutorPort::execute_stream_scoped` are new defaulted methods whose bodies ignore the scope and delegate (X-10.4, RESEARCH Pitfall 1). `PaladinExecutionService` overrides both. `StubExecutor`, the doc-examples `MockExecutor` and the handoff executors compile unchanged (workspace clippy with `--all-targets --all-features` exits 0).
- `Principal::ledger_scope()` in `paladin-web` goes through `PrincipalRef::from(self).attribution()` and `from_attribution`; `execute_agent`, both branches of `execute_agent_stream` and the task spawned by `enqueue_job` build `RunScope::default().with_ledger_scope(principal.ledger_scope())` and call the scoped methods. No `.execute(entry.paladin` call site remains. Open access settles under `(open-access, anonymous)`, never the sentinel.
- `build_agent_registry_with_ledger`'s rustdoc describes the caller-attributed scope and confines the sentinel to callers that pass no scope.
- The 007 ledger schema, `TreasuryLedgerPort`, the ledger adapters and the treasury CLI are byte-identical to `629ef660` (D-00a/D-17).

## Task Commits

Each task was committed atomically (TDD: RED then GREEN):

1. **Task 1: Worker path -- runs settle under their recorded submitter**
   - `6f04a81c` (test) -- failing `from_attribution`, `RunScope.ledger_scope`, worker and agent-loop attribution tests
   - `cb3c2906` (feat) -- `from_attribution`, `RunScope.ledger_scope`/`with_ledger_scope`, worker wiring, `AgentLoopLedgerTarget`
2. **Task 2: HTTP agent-execute path -- defaulted scoped executor-port methods, service overrides, handlers**
   - `850cea2c` (test) -- failing `Principal::ledger_scope`, five `*_attributes_spend_to_*` controller tests, two trait-object service tests
   - `57bb628a` (feat) -- defaulted port methods, service overrides, scoped stream settlement, `Principal::ledger_scope`, handler wiring, `agent_host.rs` rustdoc

**Plan metadata:** see the `docs(40-04)` commit that lands this SUMMARY with STATE.md/ROADMAP.md.

## Files Created/Modified

- `crates/paladin-core/src/platform/container/treasury_ledger.rs` -- `LedgerScope::from_attribution`; present-tense `LedgerScope`/`UNATTRIBUTED` rustdoc; 2 tests
- `crates/paladin-core/src/platform/container/run_scope.rs` -- `ledger_scope` field, `with_ledger_scope` builder with doc test, forward-compat paragraph; 3 tests
- `src/application/services/run/worker.rs` -- `SettlementContext.scope` and agent-kind `RunScope` from `run.submitted_by`; rustdoc rewrite; `RecordingTreasuryLedger` records scopes; `submit_priced_run_with`; 3 tests
- `src/application/services/paladin/paladin_execution_service.rs` -- `AgentLoopLedgerTarget`; scope-taking settle helpers; `execute_scoped` resolves the target; `execute_stream_inner(.., ledger_scope)`; scoped `stream_settlement`; trait overrides for `execute_scoped`/`execute_stream_scoped`; 4 tests
- `crates/paladin-ports/src/output/paladin_executor_port.rs` -- defaulted `execute_scoped` with `no_run` example
- `crates/paladin-ports/src/output/streaming_executor_port.rs` -- defaulted `execute_stream_scoped` with `no_run` example
- `crates/paladin-web/src/agent_auth.rs` -- `Principal::ledger_scope`; 1 test
- `crates/paladin-web/src/agent_controller.rs` -- scoped calls in `execute_agent`, `execute_agent_stream` (both branches), `enqueue_job`; `ScopeRecordingExecutor`/`ScopeRecordingStreamer` doubles; 5 tests
- `src/infrastructure/web/agent_host.rs` -- `build_agent_registry_with_ledger` rustdoc

## Decisions Made

- **Unattributed agent-kind runs carry `Some(sentinel)`, not `None`.** The worker always goes through `from_attribution(run.submitted_by.as_ref())` as the plan specifies, so a principal-less run's `RunScope.ledger_scope` is `Some(LedgerScope::unattributed())`. The agent loop's `None` fallback is therefore reached only by callers that never pass a scope at all (embedded library `execute`/`execute_stream`). Effect on the ledger row is identical; the existing `agent_kind_run_passes_its_run_id_in_the_run_scope` test pins the `Some(sentinel)` shape.
- **Stream scope resolution stays before the spawn.** `stream_settlement` became a `(ledger, scope, run_id)` triple resolved once, so the spawned producer task still owns no `&self` and the settle site is a single free-function call for both paths.
- **Trait override uses a fully qualified inherent call.** `PaladinExecutionService::execute_scoped(self, paladin, input, None, scope)` -- the inherent method takes a heartbeat parameter the trait method does not, so the two never shadow each other, but the qualified path makes that unambiguous at the call site.
- **API-surface baseline and register rows deferred to 40-06 by design.** The phase artifact table assigns `.project/current-exports.txt` and the MIGRATION.md 9.2 rows for `RunScope`, `PaladinExecutorPort` and `StreamingExecutorPort` to 40-06; this plan adds no CHANGELOG or MIGRATION rows (matches the 40-01/02/03 precedent).

## Deviations from Plan

None - plan executed exactly as written.

The only mid-task correction was to a test this plan itself added: the first draft of the extra assertion appended to `agent_kind_run_passes_its_run_id_in_the_run_scope` expected `ledger_scope == None` for an unattributed run; the plan's own `run_agent` wiring yields `Some(sentinel)` (see Decisions Made), so the assertion was corrected before the GREEN commit. No production code changed as a result.

## Issues Encountered

- Disk headroom fell from ~13 GB to ~7.9 GB after the single `--all-features` workspace clippy run (a warm `target/` grew); no ENOSPC. No further heavy builds are required by this plan.
- `make api-surface` will report public-surface drift (two new defaulted trait methods, `RunScope.ledger_scope`, `with_ledger_scope`, `from_attribution`, `Principal::ledger_scope`) until 40-06 refreshes the baseline with its CHANGELOG entries -- expected, the 39-08 and 40-01..03 precedent.
- Postgres-gated tests: none are touched by this plan (no storage changes); nothing was skipped or faked.

## Known Stubs

None. Every new code path is wired to a real data source: the run row's `submitted_by`, the authenticated `Principal`, or the documented sentinel for the genuine no-principal case.

## Threat Flags

None. No new network endpoint, auth path, file access or schema change: the scoped port methods are internal plumbing behind the existing `/v1/agents/{id}/execute[/stream]` and `/jobs` routes, and the scope is derived from the authenticated `Principal` only (T-40-16 mitigated; controller tests pin it). Settle log lines are unchanged (run id, ordinal, nanos, currency -- T-40-17); `api_key_id` is the key's configured name, never the secret.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 40-05 (`RunResponse.submitted_by` DTO) is independent of this plan and can proceed.
- 40-06 must add MIGRATION.md 9.2 rows for `RunScope` (additive field under `#[non_exhaustive]`, marked `N`), `PaladinExecutorPort::execute_scoped` and `StreamingExecutorPort::execute_stream_scoped` (defaulted methods, marked `N`), the facade re-export, CHANGELOG entries naming `LedgerScope::from_attribution`, `RunScope::with_ledger_scope` and `Principal::ledger_scope`, and refresh `.project/current-exports.txt`.
- Phase 41 (per-tenant allowances) can now evaluate ledger rows carrying real `(tenant_id, api_key_id)` pairs from both the worker and the HTTP agent routes. The remaining sentinel writers are schedule-fired runs (D-10 deferral flagged for Phase 41) and embedded library callers.
- TENANT-02 is shared with 40-01, 40-02, 40-05 and 40-06 and is not marked complete here (shared-ID gate); it flips when the last declaring plan lands.

## Self-Check: PASSED

- Commits `6f04a81c`, `cb3c2906`, `850cea2c`, `57bb628a` exist on `claude/laughing-dirac-e0h2ax`.
- All nine `files_modified` exist and contain the plan's `contains` strings (`pub fn from_attribution`, `pub ledger_scope: Option<LedgerScope>`, `async fn execute_scoped`, `async fn execute_stream_scoped`).
- Task 1 and Task 2 `<verify>` blocks and every `<acceptance_criteria>` line were run and passed (see coverage block).

---
*Phase: 40-tenant-identity-run-read-scoping*
*Completed: 2026-09-29*
