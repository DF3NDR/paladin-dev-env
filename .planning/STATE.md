---
gsd_state_version: 1.0
milestone: v0.11.0
milestone_name: Crate Release
current_phase: 43
current_phase_name: Rate Pacing
status: executing
stopped_at: Completed 43-10-PLAN.md
last_updated: "2026-10-08T23:56:05.835Z"
last_activity: 2026-10-08
last_activity_desc: Phase 43 execution started
progress:
  total_phases: 7
  completed_phases: 6
  total_plans: 64
  completed_plans: 61
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-30 after Phase 40)

**Core value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.
**Current focus:** Phase 43 — Rate Pacing
Phase 45 was resequenced ahead of, Phase 45 D-01). Milestone v0.11.0: 10 phases (38-47), 35/35
requirements mapped; Phases 38, 39, 40 and 45 sealed (Phase 40 verified 2026-09-30 once Phase 45's
RustFS swap delivered the green CI run its UAT test 4 was blocked on). Source of truth:
`.project/Milestone_14-Treasurer/` plus the supporting scope in PROJECT.md *Current Milestone*.

**Progress:** [████████████████████] 30/30 plans ([██████████] 95%) of the phases planned so far (38, 39, 40, 45);
Phases 41-44, 46 and 47 are not yet planned.

**Previous milestone:** v0.10.0 "Durable Agent Execution Runtime" closed 2026-09-23 — 19 phases
(22-37.1), 231 plans, 574 tasks, 88/89 requirements (SHIP-05 superseded by SHIP-06), 1,678 commits
(`495483ef..6a08c293`, 2026-09-01 → 2026-09-23). Archived to `milestones/v0.10.0-ROADMAP.md`,
`v0.10.0-REQUIREMENTS.md`, `v0.10.0-MILESTONE-AUDIT.md` (status `tech_debt`, 0 gaps) and
`v0.10.0-phases/`. Released as two tags: `v0.10.0` on merge commit `1d4a9724` (2026-09-18;
published 3 of 12 crates, since yanked) and `v0.10.1` on merge commit `f7dae267` (2026-09-21; run
`35659477719`, all 12 publishable crates on crates.io at `0.10.1`). Closeout type
`override_closeout` — see *Deferred Items*. Full record: MILESTONES.md.

**Prior:** v0.9.0 "Security Tooling" shipped 2026-09-01 (4 phases, 25 plans, 20/20 requirements,
240 commits; tag `v0.9.0` on `0b5d4106`, all eleven crates at `0.9.0`); v0.8.0 shipped 2026-08-24
(14 phases, 149 plans, 65/65); v0.7.1 shipped 2026-08-04 (4 phases, 38 plans, 25/25). All archived
under `milestones/`.

## Current Position

Phase: 43 (Rate Pacing) — EXECUTING
Plan: 11 of 13
Status: Ready to execute
Last activity: 2026-10-08 — Phase 43 execution started

## Performance Metrics

**Velocity:**

- Total plans completed: 457
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 11 | - | - |
| 02 | 11 | - | - |
| 3 | 8 | - | - |
| 05 | 13 | - | - |
| 06 | 10 | - | - |
| 07 | 13 | - | - |
| 08 | 9 | - | - |
| 10 | 11 | - | - |
| 11 | 5 | - | - |
| 12 | 4 | - | - |
| 13 | 13 | - | - |
| 14 | 8 | - | - |
| 15 | 10 | - | - |
| 17 | 22 | - | - |
| 16 | 14 | - | - |
| 18 | 7 | - | - |
| 19 | 5 | - | - |
| 20 | 7 | - | - |
| 21 | 6 | - | - |
| 22 | 17 | - | - |
| 22.1 | 7 | - | - |
| 23 | 12 | - | - |
| 24 | 14 | - | - |
| 25 | 14 | - | - |
| 26 | 21 | - | - |
| 27 | 26 | - | - |
| 28 | 17 | - | - |
| 29 | 9 | - | - |
| 30 | 3 | - | - |
| 31 | 7 | - | - |
| 32 | 5 | - | - |
| 33 | 6 | - | - |
| 34 | 9 | - | - |
| 35 | 10 | - | - |
| 36 | 13 | - | - |
| 36.1 | 14 | - | - |
| 37 | 8 | - | - |
| 37.1 | 16 | - | - |
| 38 | 9 | - | - |
| 39 | 8 | - | - |
| 45 | 7 | - | - |
| 40 | 6 | - | - |
| 42 | 12 | - | - |

*Updated after each plan completion*

**Recent Trend:**

- Last 5 plans: —
- Trend: —

**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P09 | ~27min + human review cycle | 3 tasks | 2 files |
| Phase 01 P10 | 20min | 2 tasks | 1 files |
| Phase 01 P12 | 40min | 4 tasks | 4 files |
| Phase 03 P01 | 16min | 2 tasks | 1 files |
| Phase 3 P2 | 25min | 2 tasks | 5 files |
| Phase 03 P03 | ~15min | 3 tasks | 2 files |
| Phase 03 P05 | ~25min | 2 tasks | 1 files |
| Phase 03 P06 | ~20min | 2 tasks | 3 files |
| Phase 03 P04 | 35min | 3 tasks | 3 files |
| Phase 03 P07 | 14min | 2 tasks | 2 files |
| Phase 03 P08 | ~30min | 3 tasks | 4 files |
| Phase 06 P07 | ~50min | 3 tasks | 4 files |
| Phase 12 P02 | ~20min | 2 tasks | 3 files |
| Phase 12 P03 | ~20min | 2 tasks | 3 files |
| Phase 12 P04 | ~25min | 2 tasks | 3 files |
| Phase 20 P07 | 35min | 3 tasks | 3 files |
| Phase 34 P01 | 17min | 2 tasks | 9 files |
| Phase 34 P02 | 15min | 1 tasks | 3 files |
| Phase 34 P03 | 18min | 2 tasks | 3 files |
| Phase 34 P04 | 28min | 2 tasks | 3 files |
| Phase 34 P05 | ~60min | 2 tasks | 2 files |
| Phase 34 P06 | ~18min | 2 tasks | 5 files |
| Phase 34 P07 | 16min | 2 tasks | 16 files |
| Phase 34 P08 | ~90min | 2 tasks | 3 files |
| Phase 34 P09 | ~75min | 2 tasks | 4 files |
| Phase 36 P01 | ~45min | 2 tasks | 9 files |
| Phase 36 P02 | ~55min | 3 tasks | 10 files |
| Phase 36 P03 | ~20min | 2 tasks | 5 files |
| Phase 36 P04 | ~25min | 2 tasks | 8 files |
| Phase 36 P05 | 35min | 2 tasks | 8 files |
| Phase 36 P06 | 30min | 2 tasks | 3 files |
| Phase 36 P07 | ~55min | 2 tasks | 3 files |
| Phase 36 P08 | ~2h10min | 3 tasks | 4 files |
| Phase 36 P09 | ~1h30min | 3 tasks | 6 files |
| Phase 36 P10 | ~1h50min | 3 tasks | 6 files |
| Phase 36 P11 | ~1h10min | 3 tasks | 2 files |
| Phase 36 P12 | ~1h | 2 tasks | 8 files |
| Phase 36 P13 | 25min | 3 tasks | 4 files |
| Phase 36.1 P01 | 20min | 2 tasks | 3 files |
| Phase 36.1 P02 | 15min | 2 tasks | 2 files |
| Phase 36.1 P03 | 20min | 3 tasks | 4 files |
| Phase 36.1 P04 | ~35min | 3 tasks | 9 files |
| Phase 36.1 P05 | ~15min | 3 tasks | 7 files |
| Phase 36.1 P06 | 6min | 3 tasks | 4 files |
| Phase 36.1 P07 | ~15min | 3 tasks | 4 files |
| Phase 36.1 P08 | ~20min | 2 tasks | 3 files |
| Phase 36.1 P09 | ~35min | 2 tasks | 3 files |
| Phase 36.1 P10 | ~15min | 2 tasks | 3 files |
| Phase 36.1 P11 | ~11min | 2 tasks | 7 files |
| Phase 36.1 P12 | ~20min | 3 tasks | 6 files |
| Phase 36.1 P13 | ~28min | 3 tasks | 3 files |
| Phase 36.1 P14 | ~20min | 2 tasks | 1 files |
| Phase 37 P01 | 35min | 3 tasks | 2 files |
| Phase 37 P02 | 50min | 1 tasks | 2 files |
| Phase 37 P03 | 53min | 2 tasks | 1 files |
| Phase 37 P04 | 7min | 2 tasks | 5 files |
| Phase 37 P05 | 14min | 2 tasks | 2 files |
| Phase 37 P07 | 25min | 2 tasks | 1 files |
| Phase 37.1 P01 | ~20min | 2 tasks | 2 files |
| Phase 37.1 P02 | ~40min | 3 tasks | 5 files |
| Phase 37.1 P03 | ~25min | 3 tasks | 3 files |
| Phase 37.1 P04 | ~16min | 3 tasks | 2 files |
| Phase 37.1 P05 | ~9min | 3 tasks | 3 files |
| Phase 37.1 P06 | ~28min | 3 tasks | 29 files |
| Phase 37.1 P07 | ~2h30min | 3 tasks | 3 files |
| Phase 37.1 P08 | ~35min | 2 tasks | 3 files |
| Phase 37.1 P09 | ~1h10min | 2 tasks | 2 files |
| Phase 37.1 P10 | ~20min | 2 tasks | 2 files (uncommitted) |
| Phase 38 P01 | ~5min | 2 tasks | 7 files |
| Phase 38 P02 | 57min | 2 tasks | 10 files |
| Phase 38 P03 | 23min | 2 tasks | 10 files |
| Phase 38 P04 | 95min | 2 tasks | 43 files |
| Phase 38 P05 | 9min | 2 tasks | 2 files |
| Phase 38 P06 | ~28min | 2 tasks | 18 files |
| Phase 38 P07 | ~25min | 3 tasks | 8 files |
| Phase 38 P08 | ~26min | 2 tasks | 5 files |
| Phase 38 P09 | ~110min | 2 tasks | 10 files |
| Phase 39 P01 | ~30min | 2 tasks | 11 files |
| Phase 39 P02 | ~30min | 2 tasks | 6 files |
| Phase 39 P03 | ~42min | 2 tasks | 4 files |
| Phase 39 P04 | ~55min | 2 tasks | 4 files |
| Phase 39 P05 | ~50min | 2 tasks | 4 files |
| Phase 39 P06 | ~20min | 2 tasks | 4 files |
| Phase 39 P07 | ~50min | 2 tasks | 4 files |
| Phase 39 P08 | ~55min | 2 tasks | 13 files |
| Phase 40 P40-01 | ~2h (continuation) | 3 tasks | 29 files |
| Phase 40 P02 | ~1h | 2 tasks | 7 files |
| Phase 40 P03 | ~35m | 2 tasks | 3 files |
| Phase 40 P04 | 17 min | 2 tasks | 9 files |
| Phase 40 P05 | 15min | 2 tasks | 5 files |
| Phase 40 P06 | 60min | 3 tasks | 14 files |
| Phase 45 P01 | 55min | 3 tasks | 9 files |
| Phase 45 P02 | 45min | 3 tasks | 7 files |
| Phase 45 P03 | 40min | 3 tasks | 6 files |
| Phase 45 P04 | 95min | 3 tasks | 15 files |
| Phase 45 P05 | 40min | 3 tasks | 16 files |
| Phase 45 P06 | 50min | 3 tasks | 10 files |
| Phase 45 P07 | n/a (human measurement) | 2 tasks | 10 files |
| Phase 41 P01 | 45min | 4 tasks | 28 files |
| Phase 41 P02 | 40min | 2 tasks | 6 files |
| Phase 41 P03 | 75min | 2 tasks | 11 files |
| Phase 41 P04 | 60min | 3 tasks | 12 files |
| Phase 41 P05 | 75min | 3 tasks | 22 files |
| Phase 41 P06 | 75min | 3 tasks | 24 files |
| Phase 41 P07 | 65min | 2 tasks | 17 files |
| Phase 41 P08 | ~2h | 3 tasks | 21 files |
| Phase 41 P09 | 40min | 2 tasks | 12 files |
| Phase 42 P01 | 10min | 2 tasks | 3 files |
| Phase 42 P02 | 58 | 3 tasks | 19 files |
| Phase 42 P03 | 40min | 3 tasks | 20 files |
| Phase 42 P04 | 25min | 2 tasks | 9 files |
| Phase 42 P05 | 40min | 3 tasks | 23 files |
| Phase 42 P06 | 45min | 2 tasks | 14 files |
| Phase 42 P07 | 1h | 3 tasks | 12 files |
| Phase 42 P08 | 45min | 3 tasks | 11 files |
| Phase 42 P09 | 66min | 2 tasks | 16 files |
| Phase 42 P10 | 25min | 3 tasks | 25 files |
| Phase 42 P11 | 35min | 3 tasks | 10 files |
| Phase 42 P12 | 35min | 3 tasks | 18 files |
| Phase 43 P01 | multi-session | 3 tasks | 18 files |
| Phase 43 P02 | ~35min | 3 tasks | 35 files |
| Phase 43 P03 | ~25min | 3 tasks | 9 files |
| Phase 43 P05 | 2h | 3 tasks | 7 files |
| Phase 43 P04 | 1h | 2 tasks | 6 files |
| Phase 43 P06 | 100min | 2 tasks | 7 files |
| Phase 43 P07 | 75min | 2 tasks | 11 files |
| Phase 43 P08 | 1h | 2 tasks | 8 files |
| Phase 43 P09 | 90min | 2 tasks | 11 files |
| Phase 43 P10 | 1h45m | 2 tasks | 14 files |

## Accumulated Context

### Decisions

**Cleared at the v0.10.0 close (2026-09-23).** Every phase-level decision this section carried for
Phases 22-37.1 is recorded in its own `NN-CONTEXT.md` (and the SUMMARY decision lists) under
`milestones/v0.10.0-phases/`; the locked ones are ADRs in `.planning/decisions/` and the
`## Key Decisions` table in PROJECT.md. Only what a later milestone must honour is kept here:

- **Two-officer token model** — `Commissary` (input-side rationing) is kept and anchored;
  `Treasurer` (output-side spend governance) is reserved for Milestone 14, not built (ADR-0049,
  ADR-0050). `TokenBudget`, `TokenCounterPort`, `TokenUsage`, `max_tokens` are not renamed.

- **X-03 supersession is scoped** — clean breaks were allowed for Phases 31-33 only (ADR-0051); any
  further removal before v0.11.0 needs its own recorded supersession.

- **`paladin-memory` → `paladin-llm`** (`default-features = false`) is the workspace's one lateral
  adapter-to-adapter edge (Phase 33 D-01/D-03); `reqwest` never enters the normal graph through it.

- **MSRV 1.88, measured** from the `time` ≥ 0.3.47 / `rmcp` `process-wrap` chain, single-sourced in
  `workspace.package.rust-version` and enforced by the `msrv` CI job (Phase 22.1; `MIGRATION.md`
  §9.3).

- **Accepted deviations, to revisit** — tracing overhead re-measured in Phase 45 at +19.36 % (log
  sink) / +16.19 % (composite) against PRD 07's ≤ 3 % bar after the D-17 fixes (was +22.18 % /
  +18.46 %, Phase 28 D-16/D-37); the maintainer accepted the new figure at Phase 45 UAT test 4 and
  `WINDOWS.md` row 61 (amending row 35) is waived with that acceptance text; the remaining cost is
  in the dispatcher/sink path, not serialisation (`45-BENCH-EVIDENCE.md`). Closed in this milestone:
  run-inspection routes are tenant-scoped (Phase 40, ADR-0054, row 32) and legacy `Runnable::Agent`
  runs now emit SSE/webhook events (Phase 45 PLAT-08, row 31 → fixed row 60). Still open: SSE `done`
  reports `halted` for a caller-cancelled run whose persisted status is `Cancelled` (Phase 27 D-14;
  Phase 42).

- **Release mechanics** — tags are cut on `main` merge commits per the Phase 29 two-SHA rule; the
  `.project/v0.10.0/09-program-acceptance-audit.md` sign-off boxes are ticked by a human, never an
  agent (Phase 29 D-17, Phase 37 D-00a); `make publish-dry-run` resolves from the local workspace
  overlay and is blind to registry publish order — `scripts/check-publish-order.sh` in
  `make check-gates` and CI is the gate for that class (Phase 37.1).

- **Partial-publish recovery** — maintainer's option A: a patch release through the same pipeline,
  the failed tag left in place and its GitHub Release bannered/pre-release, orphaned crate
  versions yanked by the maintainer only after the patch is registry-verified, one register row
  per crate in `docs/src/appendix/release-recovery.md` §5 (Phase 37.1 D-02).

The pre-close text of this section (Phase 33/32/29/28/23 decision digests and the tool-appended
`[Phase ?]` bullets back to Phase 3) is in this file's git history at commit `6a08c293`.

- [Phase ?]: ADR-0052: mid-run Treasurer enforcement — metering at LlmPort pricing decorator both paths, halt at WarEngine superstep boundary + agent-loop TokenBudget cutoff
- [Phase ?]: ADR-0053: append-only derive-on-read ledger; superstep-aggregate settlement selected at plan 38-01 checkpoint, D-15 key (run_id, superstep, attempt) kept unamended
- [Phase ?]: 38-02: CostTally (Empty/Priced/Unknown poisoning accumulator) added for run-level cost totals shared by TraceDispatcher::total_cost (38-06) and the agent loop (38-07); Task 1's cost_of_call arithmetic needed no correction under the 17-test contract.
- [Phase 38]: Hand-rolled exact-integer decimal parser instead of rust_decimal for treasurer.pricing (no new dependency). — Narrow grammar (digits + optional 1-9-digit fraction) is simple to prove exact with checked_mul/checked_add; keeps dependency count flat (no Cargo.lock/deny/audit/MSRV change, no package-legitimacy checkpoint).
- [Phase ?]: PricingLlmAdapter::generate prices response.model/response.usage via a shared price_or_warn helper, surviving FallbackLlmAdapter hops to differently-named models — 38-04: D-09/D-05 -- one shared helper keeps the pricing/warn-once rule identical on both the streaming and non-streaming run paths
- [Phase 38]: All three heralds (markdown, JSON, table) now render cost exclusively through ExecutionMetadata::cost_display()/cost_currency(); table herald acceptance-grep for removed placeholder literals forbids them anywhere in the file, so regression tests must build forbidden strings from non-contiguous fragments rather than writing them verbatim. — 38-05 closes D-04/D-11 for the JSON and table heralds (markdown done in 38-02) and research Pitfall 4 (table herald ignored its argument).
- [Phase 38]: 38-06: TraceDispatcher::total_cost added as the synchronous twin of total_usage, folded via CostTally::record_node inside emit() itself; all five WarEngine RunFinished emission sites populated with cost: trace.total_cost(). The engine's real Paladin-attempt NodeFinished site stays cost: None pending 38-07's per-attempt wiring.
- [Phase 38]: execute_structured_call forwards response.cost.clone() beside usage, even though Task 2's action text names only the reasoning loop -- required so Task 3's structured-output arm (structured.raw.cost) has a real value to read, matching D-10's "cost rides beside usage everywhere usage travels" rule.
- [Phase 38]: 38-08: HeraldTraceSink wired the engine path's ExecutionMetadata producer into RunWorkerPool; end-to-end engine tests attach HeraldTraceSink directly to a bare WarEngine (mirroring tracer_e2e.rs), not through the full RunWorkerPool/queue/repository harness -- proving the price-table-to-rendered-herald-text chain without unrelated harness weight.
- [Phase 38]: 38-09: The CI-pinned cargo-semver-checks --baseline-version 0.9.0 job is currently a no-op for every package (0 checks, 254 skip) because the in-tree version (0.10.1) already reads as a pre-1.0 major-equivalent bump over that two-milestones-old baseline -- not a Phase 38 defect, but a standing gap until the version is bumped for v0.11.0. — Empirically confirmed across all 11 CI packages; ci.yml is not in this plan's files_modified so it is documented, not fixed. Flag for whichever phase next touches the semver job or performs the v0.11.0 version bump.
- [Phase 38]: 38-09: TraceEvent::NodeFinished/RunFinished.cost fires enum_struct_variant_field_added against the published v0.10.1 baseline (a genuinely unsuppressed lint) but is invisible to the CI-pinned v0.9.0 comparison since TraceEvent postdates that baseline -- registered N/A in MIGRATION.md for the v0.10 -> v0.11 migration guide (Phase 46, CURR-23), no allowlist entry. — Confirmed via cargo semver-checks check-release --baseline-version 0.10.1 --release-type minor; the X-10-governed CI gate only tracks breaks against the published v0.9.0 baseline, so no Cargo.toml suppression or allowlist entry applies to a type that did not exist at that baseline.
- [Phase ?]: 39-01: Task 1 checkpoint (007 treasury_ledger schema/index design) approved as proposed (option-a) by the operator before execution -- one-way scope columns and partial unique settlement index, D-12 index keyed on attributed_at per planner deviation
- [Phase ?]: 39-01: TreasuryLedgerPort's Postgres CLI arm returns the same not(storage-postgres)-feature configuration error regardless of feature flags, since PostgresTreasuryLedger does not exist until 39-03
- [Phase ?]: [Phase 39] 39-02: contract_tests.rs authored as one file spanning both tasks' clauses (shared helpers, non-test-gated module); Task 1's commit includes the full suite text, Task 2 wires it into InMemoryTreasuryLedger and the SQLite test module
- [Phase ?]: [Phase 39] 39-02: real balance+hold overflow (checked_add returning None) proven by priming a fresh scope with a small balance then reserving i64::MAX against ceiling i64::MAX -- the plan's literal 'hold i64::MAX ceiling 0' example never overflows from a balance of 0
- [Phase ?]: 39-03: PostgresTreasuryLedger serializes reserve/reserved-settle/release per scope with a transaction-scoped pg_advisory_xact_lock(hashtext($1)::bigint) taken before any SUM or state read; an unreserved settle needs no lock, since idempotency is enforced entirely by the partial unique settlement index.
- [Phase ?]: 39-03: No Docker daemon available in this sandbox; brought up a local Postgres 16 cluster (pg_ctlcluster) with a scratch paladin/paladin_treasury_test role+database to prove all 23 treasury::postgres tests live (0 SKIP:), per the plan's own fallback instruction. CI's postgres-integration job remains the authority for the Docker-gated path.
- [Phase 39]: 39-04: SpendHook::child carried #[allow(dead_code)] in Task 1's commit (removed in Task 2) since its only production call site (ChildEngineResources.spend) is wired in Task 2 -- a documented Rule 3 auto-fix to keep Task 1's own clippy -D warnings gate green standalone.
- [Phase 39]: 39-04: Every superstep::run/run_with_namespace call site (18 total, including 14 test-only helpers across superstep.rs/mod.rs/graph.rs) needed the new spend trailing parameter, not just the 4 named production sites -- found by grepping every call site rather than trusting the plan's named line numbers.
- [Phase ?]: 39-05: settle_agent_loop_call extracted as a free function so execute_stream_inner's spawned task (owns no &self) shares the exact settle logic PaladinExecutionService::settle_model_call delegates to for the buffered loop.
- [Phase ?]: 39-05: the streamed agent-loop path settles only under AgentLoopSettlement::EveryCall (never PlatformRunsOnly) since no execute_stream_scoped exists -- a stream never carries a RunScope to read a Platform run id from, so it always settles under (execution_id, 1, 1).
- [Phase ?]: 39-05: build_agent_with_llm/build_agent gained a trailing Option<Arc<dyn TreasuryLedgerPort>> parameter mirroring the existing trailing price_table parameter, keeping the Treasurer's pricing and ledger concerns parallel at one call site rather than a second composition mechanism.
- [Phase ?]: [Phase 39] 39-06: CostDto shared by RunResponse and ExecuteResponse; run_costs makes exactly one TreasuryLedgerPort::spend(SpendGroupBy::Run) call per GET /runs request/page, degrading to null on any ledger error or mixed-currency run
- [Phase ?]: [Phase 39] 39-06: openapi_golden_v0_9.rs (SHIP-02 frozen v0.9.0 diff, not in the plan's files_modified) needed a second sanctioned ExecuteResponse exception (cost/CostDto) mirroring the Phase 31 usage/token_count one -- discovered by the plan's own cargo test -p paladin-web verify command
- [Phase ?]: 39-07: RunWorkerPool::with_treasury_ledger attaches WarEngine::with_treasury_ledger to every per-run engine_factory-built engine keyed by SettlementContext { scope: LedgerScope::unattributed(), run_id, attempt } where attempt is run.attempt on first dispatch or bump_attempt's return on a Running redelivery (D-07); run_agent now dispatches through execute_scoped with RunScope::default().with_run_id(run.run_id) so an agent-kind run settles under the Platform run id via 39-05's PlatformRunsOnly writer.
- [Phase ?]: 39-07: build_treasury_ledger(&RunStoreConfig) shares the run store's own backend selection (Disabled -> None, Sqlite -> SqliteTreasuryLedger, Postgres -> PostgresTreasuryLedger on storage-postgres); build_run_api wires it into the worker pool, the run engine's PaladinPort (PlatformRunsOnly) and RunApiState, and paladin-server.rs builds it ahead of the agent registry for build_agent_registry_with_ledger and FacadeProvisioner::with_treasury_ledger.
- [Phase ?]: 39-07: build_run_api_wires_the_treasury_ledger added as its own dedicated test (rather than only extending the two existing RunApiState-field tests) so the plan's own acceptance criterion -- a test literally named build_run_api_wires_the_treasury_ledger passing -- is satisfied by name.
- [Phase ?]: [Phase 39] 39-08: no new .cargo/semver-checks-allowlist.toml entry or Cargo.toml lint-table line was needed anywhere -- every lint the 0.10.1 diagnostic run reported (paladin-web's RunResponse.cost, ExecuteResponse.cost) was already covered by an existing crate-wide suppression, confirmed by a temporarily-disabled-and-reverted diagnostic (D-27 method).
- [Phase ?]: [Phase 39] 39-08: RunResponse gained its first MIGRATION.md §9.2 row, marked N/A for CI set-equality -- it is a new-in-0.10 type absent at the v0.9.0 baseline, so its cost field addition is recorded for the v0.10 -> v0.11 migration guide (Phase 46, CURR-23) rather than as a CI-gated row, mirroring the ThreadApiState/ResumeAcceptedResponse precedent.
- [Phase ?]: Task 1 checkpoint: option-a (Principal #[non_exhaustive] + Principal::new) auto-selected under --auto mode
- [Phase ?]: Postgres run-attribution columns deliberately deferred to 40-02; SQLite adapter fully wired this plan
- [Phase ?]: 40-02: RunQuery.scope defaults to RunReadScope::All so internal callers stay unscoped; only list_runs narrows it from Principal::read_scope() (D-12 / Pitfall 8)
- [Phase ?]: 40-02: tenant scope is a bound predicate inside each adapter's own keyset query (SQLite/Postgres WHERE tenant_id = ?, in-memory permits before sort/page), never a Rust post-filter
- [Phase ?]: 40-03: AuthConfig::validate() checks api_keys even when http.auth.enabled is false -- a disabled section's keys take effect the moment auth is re-enabled
- [Phase ?]: 40-03: boot-time duplicate key-value detection compares with plain == (value -> first name map) and names both keys by name only; ct_eq stays on the request path
- [Phase 40]: 40-04: LedgerScope::from_attribution is the only attribution-to-scope mapping; worker (engine + agent-kind) and HTTP agent handlers settle under the submitting principal, the unattributed sentinel only where no principal exists
- [Phase 40]: 40-04: PaladinExecutorPort::execute_scoped and StreamingExecutorPort::execute_stream_scoped are defaulted scope-ignoring delegates (X-10.4); an unattributed agent-kind run's RunScope carries Some(sentinel), the None fallback is reached only by scope-less embedded callers
- [Phase 40]: 40-05: stream/cancel/webhook-deliveries gate on load_visible_run and now answer 501 naming run_store.backend when the run store is unwired; the route matrix enumerates /v1/runs/{run_id}* from the router's own OpenAPI document
- [Phase 40]: 40-05: utoipa descriptions are single-line strings and public handler docs name the private load_visible_run in plain backticks (cargo doc -D warnings)
- [Phase ?]: 40-06: paladin-web | Principal semver allowlist entry reduced to struct_marked_non_exhaustive -- cargo-semver-checks 0.50.0 does not fire constructible_struct_adds_field for a field added to a struct made #[non_exhaustive] in the same change
- [Phase ?]: 40-06: no Cargo.toml semver allow line added for Phase 40 -- every lint the D-27 diagnostic fired is already under an existing crate-wide allow
- [Phase ?]: 40-06: WINDOWS.md row 32 stays waived (tool refuses WINDOWS_ALREADY_RESOLVED); closing condition recorded as met in CHANGELOG and ADR-0054; row 59 filed for the unsatisfiable RunReadScope current-exports grep
- [Phase ?]: 40-06: a pub mod carries an outer /// doc or inner //! docs, never both -- rustdoc merges them and resolves intra-doc links in the parent scope (7 span-less warnings fixed in be3a9030)
- [Phase ?]: Phase 45-01: RustFS passes the FileStoragePort contract suite with the existing rust-s3 adapter (11 cases, native 1.0.0); D-08 second-adapter path stays closed
- [Phase ?]: Phase 45-01: RUSTSEC-2025-0111 register row re-pointed at tokio-tar -> testcontainers; still suppressed because testcontainers 0.24.0 pulls tokio-tar
- [Phase ?]: 45-02: agent runs route failure through RunFinished{Failed} then persist_failure (one terminal event); record_engine_failure keeps its direct publish for graph-path callers
- [Phase ?]: 45-02: WINDOWS row 31 stays waived; closure recorded as fixed row 60 via the ledger tool (40-06 precedent)
- [Phase ?]: 45-03: LogTraceSink stays a Copy unit struct with a thread-local bounded buffer; enablement guard kept outside write_trace_line; TraceDispatcher::emit gets no guard (seq/tallies/replay, D-00f) - rustdoc only
- [Phase ?]: 45-04: s3-storage not added to Coverage job or scripts/coverage.sh (Pitfall 7); contract suite gated by its own compiled-in and passed-count CI steps
- [Phase ?]: 45-04: k8s RustFS runs with console disabled, non-root uid 10001; k8s/minio.yaml renamed to k8s/rustfs.yaml
- [Phase ?]: 45-05: dev compose inherits the pinned RustFS image in the .dev.yml override; console stays on in dev/devcontainer (off in CI/k8s); coverage.sh log label kept
- [Phase ?]: Phase 45-06: ADR-0055 records RustFS as the dev/test and reference object store; interim MinIO re-pin todo superseded not executed; s3-storage stays out of scripts/coverage.sh
- [Phase 45]: 45-07: OBS-05 re-measured, Verdict AMEND: log_sink +19.36 %/composite +16.19 % at point C (target enabled); WINDOWS row 61 open pending maintainer acceptance at Phase 45 UAT, then windows waive 61
- [Phase ?]: [Phase 41] 41-01: TreasuryLedgerPort::balance is a defaulted method (InvalidRequest) so the 9.2 register rows are N with no allowlist entry; admission fails closed on any ledger error and a request with no principal never reaches the Treasurer
- [Phase ?]: [Phase 41] 41-01: design checkpoint resolved option-b -- D-17 operator webhook payload amended to carry tenant_id and api_key_id (twelve keys)
- [Phase ?]: 41-02: No Treasurer production change needed; admission rule proven over a scripted store clock. Over-admission race remains an accepted ADR-0056/Phase 42 backstop
- [Phase ?]: 41-03: allowance webhook secret is env-supplied (APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET) because the config loader does not expand ${VAR}; WINDOWS row 62 tracks the example configs
- [Phase ?]: 41-03: build_run_api runs the D-11 allowance coherence check before the disabled-store early return and builds one Treasurer only when entries exist
- [Phase ?]: 41-04: fork and submit share one private admit_and_persist lifecycle; agent routes admit then confirm back to back via admit_principal (jobs refuses before jobs.create)
- [Phase ?]: 41-04: agent-handler rustdoc left unchanged because utoipa copies it into the frozen v0.9 operation description; golden exception removes only responses[429] on three agent operations
- [Phase 41]: Schedule-fired runs keep skipping authorize_invocation: SubmitRun.attributed_to is identity only, never a role (41-05, D-08/C13)
- [Phase 41]: ScheduleResponse does not expose a schedule's created_by while GET /v1/schedules is not tenant-scoped (41-05, Open Question 7)
- [Phase 41]: Pre-Phase-41 schedules (NULL created_by) fire unattributed and ungated; tracked as open WINDOWS.md row 63 (41-05, D-08)
- [Phase ?]: 41-06: notices dedup is store-enforced (unique idx_treasury_notices_once + ON CONFLICT DO NOTHING); api_key_id '' for tenant scope and epoch window_start for lifetime; claim before insert, abandon discards; notice failure never blocks a run
- [Phase ?]: 41-06: validate_notice rejects a window notice without bounds and a lifetime notice with a window, so adapters agree on lifetime read-back
- [Phase 41]: 41-07: allowance trace event emitted by the worker on the Queued first dispatch before RunStarted; one shared herald line via ExecutionMetadata::allowance_warning_display; notice store opened once in build_run_api and shared by pool and Treasurer
- [Phase 41]: Operator allowance notice rides the durable webhook_deliveries queue via RunEventKind::AllowanceWarning and a correlation run id no run owns (C3 Option A, twelve-key payload, option-b)
- [Phase 41]: Operator webhook secret is held only on WebhookDeliveryService and signs allowance_warning deliveries before any run lookup; never on the row
- [Phase 41]: build_run_api SSRF-checks treasurer.allowance.webhook.url at boot before spawning anything; enable the webhook only after every replica runs this build (Pitfall 12)
- [Phase ?]: ADR-0056 (allowance admission model) accepted: tumbling UTC windows, check-only admission, every-limit composition, no role bypass, fail-closed, store-deduped notices; over-admission race closed by Phase 42
- [Phase ?]: Phase 42-01: option-b design gate; ADR-0057 records check-only boundary, typed HaltCause, fork-as-resume, derived agent budget; true streamed done carries informational halt_reason on crossing
- [Phase 42]: Plan 42-02: the superstep-boundary spend check is a check-only balance read through the one shared Treasurer::evaluate that admission also calls; no reserve row, settlement and the ledger schema untouched — ADR-0057 D-01/D-17: one function per rule keeps admission and the boundary from drifting; the same-instant over-admission race stays accepted and bounded to one superstep per run
- [Phase 42]: Plan 42-02: RunOutcome::Halted carries a typed HaltCause; a spend halt maps to Halted with error None whatever the cancel and shutdown flags say, CancelRequested maps to Cancelled, and the in-process Token cause keeps today's precedence — ADR-0057 D-05: the cause is typed at the source so the worker, the persisted reason (42-03) and the SSE done (42-05) all read one value instead of re-querying flags
- [Phase ?]: 42-03: halt_reason is the last runs INSERT column on both backends; record_outcome precedes update_status for Halted and Cancelled (G14); the halted webhook key is gated on status Halted
- [Phase ?]: 42-04: No new public symbol; the fail-closed log line gained the scope kinds via a pure builder (fail_closed_message) tested without a process-wide logger
- [Phase ?]: 42-04: A halted run resumes only by a forked run that re-runs admission (429 + Retry-After while exhausted; 500 while the ledger is unreadable); the halted run stays terminal
- [Phase ?]: 42-06: PerRunCancelProbe attached on every factory-built engine: a same-instance caller cancel reaches the engine as CancelRequested; RunFinishStatus::Cancelled (non_exhaustive) drives done/cancelled
- [Phase ?]: 42-06: A worker drain emits no done: RunEventBusSink drops a reasonless Halted while the shutdown token is cancelled; a spend halt during shutdown is still emitted
- [Phase ?]: [Phase 42-07]: Admission::is_empty still answers only whether notices were won; a derived budget does not make an admission non-empty
- [Phase ?]: [Phase 42-07]: derive_budget refuses an exhausted ceiling before any pricing lookup; a foreign-currency price table is a Backend error naming both codes, never converted
- [Phase ?]: [Phase 42-07]: TokenBudget tightest-wins: a tie goes to the Treasurer (AllowanceHalted), an operator win keeps TokenBudget; the derived figure lives on ModelCallContext, never the middleware
- [Phase ?]: [Phase 42-07]: halt figures report the binding ceiling at balance equal to ceiling (A5); a zero-figure refusal carries the real balance
- [Phase ?]: 42-08: option-b true-stream done carries an informational halt_reason only on a strict total_tokens crossing of the derived figure; supersedes the byte-identical-in-all-cases reading; no-crossing and no-usage streams stay byte-identical
- [Phase ?]: 42-08: operator agent_runtime.token_budget now takes effect on HTTP agent routes via the one TokenBudget installed per agent service; tightest wins, a tie goes to the allowance
- [Phase ?]: 42-08: v0.9 golden Phase 42 exception is one document-level function (strip_known_v0_11_halt_reason) applied at load; unstripped loaders only for the scope test
- [Phase ?]: 42-09: the budget for a worker-dispatched agent-kind run is re-derived at dispatch from the ledger, never persisted at submit time; a refusal halts with real figures, an unreadable ledger halts ledger_unavailable, an unpriced model records Failed, all without calling the LLM
- [Phase ?]: 42-09: the shared run-engine service installs TokenBudget in Treasurer-only mode (operator figure forced off) so an engine node is never capped; agent-kind runs write no Waypoint and resume by resubmission (WINDOWS.md row 64, D-08)
- [Phase ?]: [42-10] notice_kind is the last column of the rebuilt idx_treasury_notices_once and of every ON CONFLICT arbiter list; the arbiter tests read migration 014
- [Phase ?]: [42-10] operator_event_for(NoticeKind) is crate-private and shared by the payload and the delivery row, so the paladin facade surface is unchanged
- [Phase ?]: [42-10] A pre-014 replica against a migrated database cannot infer the rebuilt index: its notice claim errors (logged, skipped); migrate and upgrade replicas together (MIGRATION 9.4)
- [Phase ?]: [Phase 42-11] The guard's per-run claim memo is keyed like the store's own notice identity (scope, tenant, key name, limit, window start, ceiling, notice kind) and covers the halt claim too; held behind an Arc so the public TreasurerSpendGuard keeps its Freeze auto trait
- [Phase ?]: [Phase 42-11] The worker builds the per-run spend guard after the run's own trace emitter exists; a run with no trace sink still claims and queues its notices and only skips the stream event; a halt notice emits no trace event
- [Phase ?]: Phase 42-12: the halt line states the currency once after the ceiling (25.0000 of 25.0000 USD); ExecutionMetadata stores the rendered line, never the typed HaltReason
- [Phase ?]: Phase 42-12: the G2 windows row states the option-b scope (a true stream's done carries an informational halt_reason only when its terminal usage crossed the derived figure; not byte-identical in every case)
- [Phase ?]: Phase 42-12: the vocabulary guard assembles both forbidden strings at run time and proves it can fail through a planted temporary tree before scanning the repository
- [Phase 43]: 43-01: pacing on by default (treasurer.cadence.enabled false opts out); OpenAI adapter surfaces its first 429; engine-port and shared gate wiring deferred to 43-09
- [Phase ?]: 43-02: an empty RateLimitHints is not stored on the error; the Cadence passes an explicit delay to the port unreduced but clamps a ResetHeader-derived delay to max_backoff; a gate beyond max_wait is refused before any provider call or port write (D-06)
- [Phase ?]: 43-02: paladin-ports gained crate-wide semver allows enum_unit_variant_changed_kind and enum_variant_marked_non_exhaustive (D-27 diagnostic) with two allowlist entries under row paladin-ports | LlmError; .project/current-exports.txt unchanged because the facade baseline lists LlmError as one re-export line
- [Phase ?]: 43-03: an unusable Retry-After falls through to retry-after-ms then the exhausted reset; hints_from_headers returns None when nothing usable parsed; Go durations use exact saturating u128 nanosecond arithmetic
- [Phase ?]: 43-03: quota-class 429s (OpenAI insufficient_quota, Anthropic enforced_spend_limit_reached) map to permanent UsageLimitExceeded before the generic arm; Anthropic surfaces its first 429 (D-02); OpenAI header names and the quota code string stay pending the 43-12 operator checkpoint
- [Phase ?]: 43-05: contract paused clause uses tokio::time::sleep, not advance (test-util is dev-only); waiter spread is internal, additive, capped at min(wait/10, 1 s); InMemoryCadence bounded at 4096 keys
- [Phase ?]: 43-04: D-02 (surface the first 429) extended to the compat engine presets, DeepSeek and Gemini (Open Question 1); reversible, to be recorded in ADR-0058
- [Phase ?]: 43-06: fallback pace budget is wall-clock time from a hop's first 429; a gate that is unreadable, clear or beyond max_wait is not paced (hop rule applies)
- [Phase ?]: 43-06: config-built fallback chains are paced by default via AgentRuntimeDeps.cadence (in-process, None opts out); with_cadence is a no-op on an already-paced chain
- [Phase ?]: 43-07: Redis cadence gates computed in Lua on the server TIME with relative waits; connection lazy with explicit timeouts; provider half of the key escapes % and : so keys are injective
- [Phase ?]: 43-08: the Redis-outage warning is a log::warn! under paladin::cadence (no TraceEvent variant); ResilientCadence latches on the first primary error, probes at most every 5 s, merges the fallback gate on recovery
- [Phase ?]: 43-09: Redis URL never in config/Debug/logs -- CadenceConfig holds only url_env name; boot log names backend kind only
- [Phase ?]: 43-09: one CadenceWiring built in paladin-server shared by registry, provisioner and run API; FacadeProvisioner rejects (not silently downgrades) a redis config on a binary without redis-cadence
- [Phase ?]: 43-10: NodeCachePort gains one defaulted put_fenced (delegates to put) so D-14 fencing reaches the Redis cache without changing D-29 or any implementor; flagged reading of D-00d/D-11 for the operator and ADR-0058
- [Phase ?]: 43-10: Redis lock/counter keys use the %lock and %fence segments so they can never equal a pacing key (a provider named lock would collide with the plan's literal layout)
- [Phase ?]: 43-10: FencingToken carries its source (Distributed/Local) with no Ord; Local tokens are a plain put at the cache and never compared with Distributed ones

### Pending Todos

One of the two todos acknowledged as deferred at the v0.10.0 close (see *Deferred Items*) remains:

- `todos/pending/2026-08-13-verify-local-coverage-reproduction.md` — user-owned; walk the
  documented `make services-up` → `make coverage` procedure on a Docker-capable machine (now
  against the RustFS dev stack) and confirm it reproduces the CI figure (now 90.44 %, not the
  82.39 % the todo quotes). `recheck_by: 2026-10-16`. Re-acknowledged as a follow-up at Phase 45
  UAT test 5.

- ~~`todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`~~ — **closed by Phase 45**
  (plan 45-06, ADR-0055): RustFS `1.0.0` is the dev/test and reference object store everywhere the
  MinIO pin was; moved to `todos/completed/`.

### Blockers/Concerns

**No blockers at the v0.10.0 close (2026-09-23).** All 19 phases `passed`; `WINDOWS.md`
`open_count: 0`; audit `tech_debt` with 0 gaps. The per-phase close notes this section carried for
Phases 23-33 are resolved and archived with their phases (`milestones/v0.10.0-phases/`); the
five-run ingest concern register (run 5's eight verified-open findings and the runs 1-4
carry-forward) was disposed by Phases 5-16 and is preserved in this file's git history at commit
`6a08c293`. Open concerns carried into the next milestone:

- **Coverage is CI-attributed, not locally measurable** — this devcontainer has no Docker; the
  82 % floor (ADR-0006) is read from the CI `coverage` job (90.44 % at PR #56). The local
  reproduction walkthrough is the pending user-owned todo above.

- **Tracing overhead** now accepted at +19.36 % / +16.19 % (Phase 45 re-measure, row 61 waived);
  serialisation is no longer the target — the target-off rows still cost +16-19 %, so any further
  optimisation is in the dispatcher/sink path, or the I/O-bound re-scope Phase 45 D-19 deferred.

- **Webhook SSRF guard does not pin the resolved address** between check and connect — DNS
  rebinding is a documented limitation (`src/application/services/run/webhook/ssrf.rs` module docs,
  `security.instructions.md`).

- **`/v1/threads/*` routes are not tenant-scoped** (Phase 40 D-14 deferral, T-40-23 transferred,
  WINDOWS.md row 58 open) — threads carry no tenant, so an authenticated principal can list any
  thread and read its Waypoint state/history although that tenant's `/runs` routes answer 404; the
  run-side mutation path (`POST /runs {thread_id}`, fork) is closed by WR-01's
  `ensure_thread_visible`. Closing condition: record a tenant on the thread and route every
  `/threads/{id}*` handler through one shared gate like `load_visible_run`. Owner: Phase 41 planning
  if allowance enforcement needs thread ownership, else a v0.11 hygiene phase.

- ~~**Terminal MinIO pin**~~ — **resolved by Phase 45 (2026-09-30):** every live configuration
  runs `rustfs/rustfs:1.0.0` (manifest-list digest recorded), `mc` is gone, the 11-case
  `FileStoragePort` contract suite is green in CI run 36751442387 and the k8s smoke pod is Ready
  (ADR-0055). The image is still a single third-party pin; bump it deliberately.

- **`cargo-semver-checks` 0.50.0 coverage gap** for inherent-method return-type and tool-coverage
  classes — covered by `MIGRATION.md` §9.2 rows instead (Phases 32/33).

- **Nyquist validation** — seven v0.10.0 phases at `VALIDATION.md` `status: draft` (22, 24, 29,
  30, 34, 36, 36.1) and Phase 28 at `nyquist_compliant: false`; archived phases 05-21 likewise
  unreconciled. `/gsd-validate-phase <N>` each; coverage TODO, not a compliance failure.

- **Bookkeeping drift in the corpus acceptance audit** — the seven §11 judgment-tier sign-off boxes
  and the `v0.10.0` tag box in `.project/v0.10.0/09-program-acceptance-audit.md` are still `- [ ]`
  on disk although 29-UAT recorded the pass and the tag was cut; tick by hand or annotate as
  superseded by §13 (v0.10.1, ticked).

- ~~**`release/*` ruleset bypass** granted temporarily on 2026-09-21 for the v0.10.1 push must be
  removed now that PR #56 is merged.~~ **Removed 2026-09-23** — ruleset `20868128` is back to the
  checked-in shape (`bypass_actors: []`, `current_user_can_bypass: never`); `release/*` branches
  accept one push again, so plan repeat pushes as PRs into the branch.

- **Phase 37 reads incomplete to `init.manager`** (8 of 11 plans with SUMMARYs; 37-09..37-11
  superseded) — expected, documented in MILESTONES.md *Known Gaps*; do not "fix" by fabricating
  SUMMARYs.

- cargo-semver-checks 0.50.0's CI-pinned --baseline-version 0.9.0 job is currently a no-op for every one of the 11 checked packages -- confirmed 2026-09-26 during Phase 38 plan 38-09's closeout measurement. The in-tree workspace version (0.10.1, the last real release) already differs from that two-milestones-old baseline by a pre-1.0 "major-equivalent" bump, so the tool skips all lint evaluation (0 checks, 254 skip) unconditionally. This will remain true for every plan in every phase of the v0.11.0 milestone until the version is bumped at the eventual release commit. Not fixed in 38-09 (ci.yml is not in that plan's files_modified) -- flag for whichever phase next touches ci.yml's semver job or performs the v0.11.0 version bump.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260912-whj | Regenerate API surface snapshot so ci.yml API Surface Tracking check passes on feature/v0.10.0-web3sec-dogfooding | 2026-09-12 | 786a3ba5 | [260912-whj-regenerate-api-surface-snapshot-so-ci-ym](./quick/260912-whj-regenerate-api-surface-snapshot-so-ci-ym/) |
| 260913-15w | Pin MinIO service image to quay.io last known-good release after Docker Hub minio/minio removal | 2026-09-13 | 06765765 | [260913-15w-pin-minio-service-image-to-quay-io-last-](./quick/260913-15w-pin-minio-service-image-to-quay-io-last-/) |
| 260913-h7l | Replace dl.min.io mc download in ci.yml with checksum-verified pinned GitHub release asset after MinIO retired community downloads | 2026-09-13 | 9d0aa7a0 | [260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-](./quick/260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-/) |
| 260916-h40 | Regenerate API surface baseline for Phase 32 window exports and add a local pre-push API surface gate | 2026-09-16 | cd185c9b | [260916-h40-regenerate-api-surface-baseline-for-phas](./quick/260916-h40-regenerate-api-surface-baseline-for-phas/) |

### Roadmap Evolution

- Phase 15.1 inserted after Phase 15: Git & CI Governance — branch protection, trigger surface, gitflow model, docs/BRANCH_PROTECTION.md (URGENT)
- Phase 17 added: Additional LLM Provider Adapters — provider-selection study (PROV-01) then feature-gated adapters for survivors (PROV-02..04); first forward phase beyond the ingest
- Phase 18 added: Rust SAST: evaluate and adopt CodeQL — new Security Tooling milestone; SAST-01..04 minted at roadmap time
- Phase 19 added: crates.io Trusted Publishing — replace the long-lived CARGO_REGISTRY_TOKEN with OIDC-issued ephemeral tokens; PUB-01..PUB-05 minted at roadmap time
- Phase 20 added: Release Pipeline Recovery — idempotent re-runs on the same tag, a pre-publish gate over tag/manifest/changelog/CI agreement, and a stuck-halfway runbook with a yank policy; PUBOPS-01..PUBOPS-05 minted at roadmap time
- Phase 21 added: Release Artifacts — Curated Release Notes and Attached Distributables (v0.9.0; ARTIFACT-01..06 minted, twenty-second prefix)
- Phase 22.1 inserted after Phase 22: Engine readiness defect and MSRV follow-up (readiness defect from 22-16 audit; MSRV 1.85 vs rmcp-pinned process-wrap decision; green postgres-integration run confirmation) (URGENT)
- Phase ? changed: Phase 22.1 scope grew on 2026-09-03: BUG-04 (resume rebuilds the Frontier from scratch, losing pre-crash edge resolutions) promoted from CONTEXT.md deferred ideas into the phase at the 22.1-05 checkpoint by developer decision; ENG-04 re-opened for the phase; CI evidence to be re-captured on the final head.
- Phase 30 added: Token-Economy Vocabulary & Commissary Anchoring — vocabulary rule, Commissary ADR + mdBook page, Treasurer reservation ADR, `max_tokens` table, Quartermaster purge, clean-break versioning ADR (VOCAB-01..07 minted, twenty-seventh prefix; docs only; source `.project/Milestone_13-Token-Economy/Epic_1`)
- Phase 31 added: Lossless Token Accounting — full `TokenUsage` carried port→`RunFinished`→herald, optional cache/reasoning fields, `from_total` removed from the battalion path, streaming parity (ACCT-01..05, twenty-eighth prefix; keystone; clean break under the X-03 supersession)
- Phase 32 added: Unified Token Primitives — `TokenCounterPort::is_exact`, `Commissary::new` without `is_exact_counter`, legacy `TokenCounter`/`TokenCounterFactory` retired, one shared window resolver with strict mode (PRIM-01..05, twenty-ninth prefix; clean break)
- Phase 33 added: Commissary In-Tree Adoption — RAG through `Commissary::dispense` with shed records + marker, integration-tested production caller, Phase 29 release gates re-sealed (COMM-01..04, thirtieth prefix). Milestone 14 Treasurer (`.project/Milestone_14-Treasurer/`) reserved, not roadmapped.
- Phase 34 added: Documentation Currency Audit — read-only inventory of mdBook / rustdoc / examples gaps against Phases 22-33 (and any v0.9.0 leftovers); scopes Phases 35-36 (2026-09-17, pre-tag release readiness)
- Phase 35 added: mdBook Currency — close every Phase 34 mdBook gap; `mdbook build` + linkcheck green (2026-09-17)
- Phase 36 added: Rustdoc Zero-Warning Bar & Examples Currency — `cargo doc` 73→0 warnings so CI "Check documentation" is green, 14 `--all-features` intra-doc links resolved, `examples/` + `doc-examples` current (2026-09-17)
- Phase 37 added: v0.10.0 Crate Release — re-seal the Phase 29 gates on the final commit, merge to `main`, `release.yml` tags `v0.10.0`, all publishable crates on crates.io at `0.10.0` (2026-09-17)
- Phase 36.1 inserted after Phase 36: Deferred Items Closure — walk the Phase 31/32/34/35 deferred-items registers, WINDOWS.md #36-37 and the two pending todos; fix, waive with reason, or re-home each; bring WINDOWS.md back into agreement with the registers before Phase 37 tags v0.10.0 (URGENT)
- Phase 37.1 inserted after Phase 37: v0.10.1 Patch Release — tag v0.10.0 published 3/12 crates (battalion versioned dev-dep vs CRATES order); maintainer chose recovery option A (URGENT)

## Deferred Items

### Acknowledged at v0.10.0 milestone close (2026-09-23)

**Verification overrides: 0** — all 19 phases (22-37.1) report `verification_status: passed` with
`behavior_unverified: 0`. **Phase-completion override: 1** — Phase 37 reads `phase_complete: false`
because plans 37-09, 37-10 and 37-11 were superseded by Phase 37.1 (dated notes in the plan files,
no SUMMARY) after the `v0.10.0` tag published 3 of 12 crates; its `37-VERIFICATION.md` is `passed`
5/5 with SC4 stated as superseded. **Requirement gap acknowledged: 1** — SHIP-05 superseded by
SHIP-06, deliberately unticked (MILESTONES.md *Known Gaps*). **Open artifacts acknowledged: 2** —
both pending todos, each already dispositioned as deferred past v0.10.0 by Phase 36.1 (CURR-20)
with `recheck_by: 2026-10-16`. Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| todo | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Walk `make services-up` → `make coverage` on a Docker-capable machine and confirm the CI figure reproduces (CI now 90.44 %). Carries no `resolves_phase` tag by design | v0.8.0 close; re-acknowledged v0.9.0 and v0.10.0 |
| todo | `2026-09-13-evaluate-rustfs-replacement-for-minio` | Open — owner: repo maintainer; FUT-10 in the archived requirements. The quay.io MinIO pin (quick task 260913-15w) is terminal | Phase 36.1 (2026-09-18); acknowledged v0.10.0 close |
| phase | Phase 37 plans 37-09..37-11 | Superseded by Phase 37.1 — never executed; scope (post-publish verification, milestone-close recording for `v0.10.0`) delivered under `v0.10.1` | Phase 37.1 (2026-09-19) |
| requirement | SHIP-05 | Superseded by SHIP-06; amend-at-source note dated 2026-09-22 in `milestones/v0.10.0-REQUIREMENTS.md` | Phase 37.1 |
| testing | Nyquist validation unreconciled for Phases 22, 24, 29, 30, 34, 36, 36.1 (`draft`) and 28 (`nyquist_compliant: false`) | Coverage TODO, not a compliance failure — `/gsd-validate-phase <N>`. Joins the same open item for archived Phases 05-21 | v0.10.0 close |
| debt | Audit `tech_debt` register (tracing overhead D-16; `WINDOWS.md` waived rows 23-25, 29-35, 45, 51-55; §11 sign-off boxes; IN-01/IN-02; SSE `done` collapse; three roadmap-level v2 lines; Milestone 14 reserved) | Inventoried with owners in `milestones/v0.10.0-MILESTONE-AUDIT.md`, not duplicated here | v0.10.0 close |

### Acknowledged at v0.9.0 milestone close (2026-09-01)

**Verification overrides: 0** — all 4 phases (18-21) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1** (the same user-owned pending
todo acknowledged at the v0.8.0 close, unchanged). Closeout type `override_closeout`, on the
strength of that todo rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Unchanged since the v0.8.0 close: verifies the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's 82.39% on a Docker-capable machine. Deliberately carries no `resolves_phase` tag so a close cannot silently absorb it | v0.8.0 close, re-acknowledged v0.9.0 |
| Testing | Nyquist validation unreconciled for Phases 18-21 | All 4 `VALIDATION.md` files read `status: draft` (NOT-VALIDATED per #2117) — coverage TODO, not a compliance failure. Run `/gsd-validate-phase <N>`. Joins the same open item for archived Phases 05-17 | v0.9.0 close |

Every human-verification backstop the phase verifications declared was closed by recorded UAT
before the close: the Phase 19 crates.io token revocation (operator, 2026-08-28, `19-UAT.md`) and
both Phase 21 checks — out-of-band pull-by-digest and `paladin-cli` execution (user, 2026-09-01,
`21-UAT.md`). The remaining v0.9.0 debt items (CodeQL re-probe trigger, `workflow_dispatch`
publish path, `make publish-dry-run`, dead `upload_url` script output) are inventoried in
`milestones/v0.9.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.8.0 milestone close (2026-08-24)

**Verification overrides: 0** — all 14 phases (05-17) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1.** Closeout type
`override_closeout`, on the strength of the pending todo below rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Verifies that the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's measured 82.39% on a Docker-capable machine. The CI half is confirmed (run `31727496744` at commit `e9e3267`); the local half has never been walked end-to-end, because no authoring environment in Phase 15 had Docker or `cargo-llvm-cov`, and none is available at close. **Acknowledged rather than closed by design:** the todo deliberately carries no `resolves_phase` tag precisely so that a phase close cannot silently absorb it | v0.8.0 close |
| Testing | Nyquist validation never reconciled for Phases 05-17 | All 13 `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Phase 06 has no `VALIDATION.md` at all. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase <N>` | v0.8.0 close |
| Security | No static taint analysis for first-party Rust | Snyk was measured and removed 2026-08-18 (0 findings on a four-vulnerability Rust probe vs 3 for identical JavaScript). `cargo-audit`/`cargo-deny` scan dependencies; clippy is a lint. **Not deferred indefinitely — owned by Phase 18 (SAST-01…04) in the v0.9.0 Security Tooling milestone** | v0.8.0 close |

The full debt inventory — 25 recorded items across 10 phases, plus 12 open and 4 waived
`WINDOWS.md` rows — is in `.planning/milestones/v0.8.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.7.1 milestone close (2026-08-04)

**Verification overrides: 1.** Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Verification | Phase 1 verification timestamp stale | `01-VERIFICATION.md` records `passed` 5/5 at 2026-07-31T16:46:51Z; commit `be2ff05` (2026-08-03) later added `01-04-SUMMARY.md`, so `init.manager` reports `verification_status: stale` / `phase_complete: false`. The commit is documentation-only ("No ADR, measurement, or source changes"); accepted as an override rather than re-verified | v0.7.1 close |
| Integration | Herald not reachable from Campaign, Chain of Command, or the Commander router (audit WARN-01) | Formation and Phalanx wire Herald; the other three carry zero references. `format_battalion_result` is pattern-agnostic so no requirement's text is falsified, but the composite Chain-of-Command developer flow does not compose without the caller invoking a Herald directly. Unassigned — candidate for Phase 6. **Closed 2026-08-05, plan 06-07: adopted under CLOSE-02 and closed by plan 06-02** — see `.planning/ROADMAP.md`'s Phase 6 WARN-01 outcome note and `.planning/ledgers/milestone-02-03.md`'s CLOSE-02 scope section (Epic 22 cross-reference) for the full record | v0.7.1 close |
| Testing | Nyquist validation never reconciled for Phases 1-4 | All four `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase 1`…`4` | v0.7.1 close |

### Carried from earlier ingest runs

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | Live-provider-API integration tests (Epic 6 task 7.0, 18 subtasks) | **Un-deferred by run 2** — suite ships behind `live-api-tests`; only the skip-vs-fail semantics remain open (VERIFY-06) | Ingest run 1, revised run 2 |
| Testing | CLI end-to-end tests (Epic 9 tasks 13.4-13.6) | **Un-deferred by run 2** — the blocking mock provider shipped (REQ-mock-llm-adapter) along with the Tier-1 CLI suites | Ingest run 1, revised run 2 |
| Testing | Garrison large-conversation perf test (Epic 2 task 9.14) | Deferred — marked future enhancement | Ingest run 1 |
| Testing | Vision and RAG latency targets never measured (single image < 5 s; retrieval < 500 ms p95; extraction < 3 s p95) | Deferred to v2 — no baseline document exists | Ingest run 2 |
| Tech debt | Oversized service file decomposition (2,757 / 2,294 / 1,840 lines) | Deferred to v2 — no ingested requirement | Ingest run 1 |
| Tech debt | Clone/lock-contention optimization | Deferred to v2 — blocked on Phase 3 benchmarks | Ingest run 1 |
| Tech debt | Single-threaded orchestration scheduler (`orchestration/scheduler.rs`) | Deferred to v2 — `tokio-cron-scheduler` is already a dependency and already adapted in `paladin-storage` | Ingest run 2 |
| Scope | MCP WebSocket transport | Deferred — recorded as a known limitation by the Epic 23 completion summary | Ingest run 2 |
| Scope | Garrison semantic search / vector context retrieval in the CLI path (recency-based selection only) | Deferred — Epic 23 known limitation; superseded in spirit by Sanctum | Ingest run 2 |
| Scope | Grove learning from past routing decisions | Out of scope — Epic 16 NG-3; the release-notes `PerformanceBased` claim is verified absent from the tree | Ingest run 2 |
| Scope | Automatic Garrison-to-Sanctum migration | Out of scope — Epic 11 explicit non-goal | Ingest run 2 |
| Scope | Batch vision API | Out of scope — Epic 20 NG-6; concurrency is a Battalion concern | Ingest run 2 |
| Scope | Registry multi-tenancy, persistence, distribution | Out of scope — Epic 22 explicit non-goals | Ingest run 2 |
| Scope | ~~Milestones 9-12 feature work~~ | **Closed by run 5** — all four milestones ingested and verified shipped (M9 100%, M10 100%, M11 92.0%, M12 99.0%). Recorded in the 120-row *Milestone 9-12 as-shipped ledger*, not deferred | Ingest run 1, narrowed runs 2-4, closed run 5 |
| Tech debt | **D1 — `src/core/` re-export shims** (6 files, 49 facade importers) | **KEEP, by decision** — removal means rewriting 49 files and preserving `platform/mod.rs`'s maneuver/parser injection, which carries real logic. Becomes debt only if a no-alias policy is adopted (ARCH-04) → FACADE-02 | Ingest run 4 |
| Tech debt | **D2 — mis-layered `src/core/platform/manager/` services** (`content_service`, `event_manager`, `user_service`) | Deferred, medium/medium — partly overtaken: reconciliation commit `6704807` found "no user-service split was needed" because `UserServiceTrait` and the DTOs already live in `paladin-core`. Overlaps the run-3 v2 `user_service` relocation item; do not plan twice → FACADE-02 | Ingest run 4 |
| Tech debt | **D3 — entangled Paladin services** (`planning`/`prompt_generation`/`temperature`/`handoff`, ~2,750 LOC) | **KEEP for now**, high/high — needs the `paladin_builder.rs` / `paladin_execution_service.rs` coupling untangled first, and the targets (`paladin-battalion`, `paladin-llm`) are leaf-to-leaf edges gated on HARD-05 → FACADE-02 | Ingest run 4 |
| Tech debt | **D4 — `content_ingestion_service.rs` placement** (~1,211 LOC) | Deferred, medium/medium — M7 Epic 1's PRD listed it as moving to `paladin-content`; the facade kept its own copy. Needs a dependency-coupling review → FACADE-02 | Ingest run 4 |
| Tech debt | **D5 — residual `println!`/`eprintln!`/`dbg!`** | **Verified exact: 17 occurrences across 6 files**, down from ~435 across 36. The register's own quick win; low/low → FACADE-01 | Ingest run 4 |
| Scope | The `paladin user …` CLI command surface (1,065 LOC, 8 subcommands) | Deferred on purpose — it was declared but **never dispatched**, so it compiled and did nothing. Backend intact; reintroduction is "mostly re-wiring", recoverable verbatim from the M8 removal commit on `chore/facade-cleanup-m8-finish` → FACADE-03(a) | Ingest run 4 |
| Scope | The TensorFlow ML adapter and the `ml` feature flag (636 LOC) | Deferred on purpose — a `#[doc(hidden)]` stub nothing consumed. **Reintroduction condition is the load-bearing part**: a dedicated `paladin-ml` leaf crate, never the facade, with the flag on that crate; `MlPort` stays in the workspace → FACADE-03(b) | Ingest run 4 |
| Scope | A future **content-delivery crate** | Reserved by M7 Epic 1 §4.5.2 as the "correct long-term home" for `file_content_repository.rs`; the file was then deleted and no later document mentions the crate. Carried so the idea is not lost silently | Ingest run 4 |
| Scope | `paladin-arsenal` and `paladin-sanctum` crates | Out of scope — named only by a superseded disposition record that contradicts its own governing PRD. Neither exists; Milestone 9 is 100% complete. Triaging the list is FACADE-04 | Ingest run 4 |
| Tech debt | `paladin-core` / `paladin-ports` dependency allowlists brought back in line with reality (declared 6 and 7; ship 14 and 10) | Deferred to v2 — the architectural invariant holds; this is document-versus-code drift. Needs ARCH-03(b) to choose a direction | Ingest run 3 |
| Tech debt | `retry`, `rate_limiter` and `bulkhead` primitives in `src/infrastructure/resilience/`, plus consolidating the retry logic in `mcp_sse_adapter.rs` and `api_content_deliverer.rs` | Deferred — explicitly scoped out by Milestone 6 Epic 4, which shipped the module scaffold only | Ingest run 3 |
| Tech debt | Full `user_service` relocation out of `src/core/platform/manager/` (with `UserServiceFactory`, `user_config.rs`, user CLI commands, user API controller, `SqliteUserRepository`) | Deferred — Milestone 6 Epic 2 scoped it out and flagged it for "a future Epic" | Ingest run 3 |
| Scope | A `paladin-cli` workspace crate | Out of scope — the Milestone 5 overview's target structure named it, the Epic 6 PRD's non-goal rejected it, and the code agrees with the PRD (a `cli` feature plus `[[bin]] paladin-cli`) | Ingest run 3 |
| Scope | MCP feature flags (`mcp-arsenal` / `mcp-transports` / `mcp-stdio` / `mcp-sse`) | Out of scope — eliminated by a dated 2026-04-15 PRD note; Arsenal and its transports compile unconditionally | Ingest run 3 |
| Scope | A `paladin-infra` crate, and a `CircuitBreakerPort` trait abstraction | Out of scope — both explicitly rejected by Milestone 6 Epic 4, which accepted the resulting layering inversion as a pragmatic trade-off inside the facade crate | Ingest run 3 |
| Tech debt | **Deferred-QA Epic 25 — CI/CD pipeline enhancement** (`cli-tests`, `bench-check` and `coverage` jobs, `.codecov.yml`, four Makefile targets, eight deprecated actions, CONTRIBUTING coverage docs) | **Un-deferred by run 5 — verified unbuilt item by item and promoted to Phase 15** (PIPE-01 … PIPE-05). The register's own recommended first epic: "establishes quality gates that validate all subsequent work" | Ingest run 5 |
| Tech debt | **Deferred-QA Epic 26 — documentation and rustdoc** (architecture doc modernization, zero rustdoc warnings in CI, 100% public-API rustdoc, four asciinema demos) | **Un-deferred by run 5 — promoted to Phase 16** (DOCS-02 … DOCS-04). The architecture document is verified frozen at 311 lines with zero of seven newer subsystems | Ingest run 5 |
| Scope | **Deferred-QA Epic 27 — LLM tool calling** (`tools` on `LlmRequest`, `ToolDefinition`, `ToolCall`, `tool_calls` on `LlmResponse`, all three adapters) | **Decision required, not deferred again** — verified entirely absent; it is a **breaking change to the `LlmPort` trait** by the PRD's own admission, both its open questions are unanswered, and Arsenal/MCP already provides tool execution through a different seam → WEB-04. The separable defect (`ProviderCapabilities` over-reporting) is correctable today → WEB-03 | Ingest run 5 |
| Testing | **Deferred-QA Epics 28-29 — platform-services and event-system coverage** (`user_service.rs` ~4.23% → ≥ 80%; the listener orchestrator ~57.83% → ≥ 80%, with concurrency, deadlock, 1000-event-burst and distributed-tracing scope) | **Partially un-deferred by run 5 — promoted to Phase 15** (DEFER-01 … DEFER-03). **Scope real, numbers not**: both module paths are stale and both baselines predate Milestone 9's tests. Blocked on the shared mock infrastructure that does not exist, and `user_service.rs` must be sequenced against M8 deferred item D2 | Ingest run 5 |
| Tech debt | The shared `Send + Sync` mock and async-test infrastructure (`MockUserRepository`, `MockLogPort`, `MockNotificationService`, `MockEventSource`, `MockTriggerExecutor`, Tokio time control) | **Un-deferred by run 5 → DEFER-01.** Named as an unchecked prerequisite by `DEFERRED_COVERAGE.md` and by both coverage Epics; ~6-10 of the 35-45 estimated hours. Placement (`tests/common/` versus the existing `tests/helpers/`) and `mockall`-versus-hand-written are both unanswered | Ingest run 5 |
| Scope | A shared-store `AuthPort` implementation for multi-process serving | **Never deferred — never requirement-ed at all.** M9 Epic 5 §6.1 anticipated it in prose ("a multi-process deployment would later need a shared store") and M12 Epic 7 then shipped `k8s/deployment.yaml`. **No requirement in the 263-document corpus covers it** → WEB-02 | Ingest run 5 |
| Scope | Garrison (memory) and Arsenal (tools/MCP) wiring for HTTP-served agents | Deferred by M12 Epic 2 and restated by Epic 3 — "agents are LLM + prompt only here". **Whether this is planned scope or a permanent property of the topology is undecided**, and the deployment-topologies decision matrix that routes readers between topologies must say which → ORCH-04(b) | Ingest run 5 |
| Scope | Hot-reloading `config.yml`; TLS termination in `paladin-server`; fine-grained scopes beyond `allowed_roles` + admin gate; encrypting config at rest | Out of scope — all four are explicit Milestone 12 non-goals. TLS is a proxy/ingress concern; secrets management is "the operator's responsibility, as with LLM keys" | Ingest run 5 |
| Scope | Benchmark regression **detection** (`critcmp`, `github-action-benchmark`) | Out of scope — Deferred-QA Epic 25 non-goal. Note the inversion: `benchmark-regression-signal` already ships from M7 Epic 3 while the `bench-check` compile prerequisite does not → PIPE-01 | Ingest run 5 |
| Scope | Rewriting the 35 mdbook appendix files | Out of scope — M11 Epic 3 non-goal ("reference/archive material"). **One exception is under decision**: `design-and-architecture.md`, whose relocation into that exempt chapter is precisely why its gap survived → DOCS-02 | Ingest run 5 |

## Session Continuity

**Last session:** 2026-10-08T23:56:05.804Z
**Stopped at:** Completed 43-10-PLAN.md
**Resume file:** None

## Operator Next Steps

- **Phase 45 (RustFS Swap & Platform/Observability Deviations) is sealed 2026-09-30:**
  `45-VERIFICATION.md` passed, `45-UAT.md` complete (5/5: CI run 36751442387 green with the 11-case
  contract suite and the k8s RustFS pod Ready, actionlint green, docs page confirmed, OBS-05 figure
  accepted with WINDOWS.md row 61 waived, follow-ups acknowledged), `45-SECURITY.md` verified (31
  threats closed, 0 open), ROADMAP/REQUIREMENTS ticked (STORE-01..03, PLAT-08, OBS-05). All artifacts
  pushed on `claude/laughing-dirac-e0h2ax`; open a PR to `main` when convenient — that PR is also
  where `docs.yml` (Build MDBook) first runs for the retitled storage page.

- **Phase 40 (Tenant Identity & Run-Read Scoping) is sealed 2026-09-30:** `40-VERIFICATION.md`
  passed, `40-UAT.md` complete (5/5 — WR-01 scope decision, backstop truth, judgment-tier
  prohibitions and the credential-handling review, CI-only evidence on ci.yml run 36770517439 with
  the Postgres contract suites, coverage floor and `e2e_platform_api` green, WR-02..04 disposition),
  `40-SECURITY.md` verified (26 threats closed: 22 mitigated, 3 accepted, T-40-23 transferred to
  WINDOWS.md row 58). ROADMAP row 40 reads 6/6 Complete. `/v1/threads/*` scoping stays open as
  row 58 (owner Phase 41 planning if allowance enforcement needs thread ownership, else a v0.11
  hygiene phase).

- **Next:** `/clear` then `/gsd-discuss-phase 41` (no `41-CONTEXT.md` yet) or `/gsd-plan-phase 41` —
  Phases 41-44 run before 46 and 47 (Phase 45 D-01 resequencing). Phase 41 planning should decide
  WINDOWS.md row 58's ownership.

- **Phase 39 (Spend Ledger) sealed 2026-09-28** (verification passed, UAT 49/49, VALIDATION and
  SECURITY verified); **Phase 38 sealed 2026-09-26.**

- **Still manual:** operator UAT of a real multi-model engine run (`paladin-cli treasury spend --group-by model`
  against a priced `treasurer:` table) — optional, not required for compliance.

- **Housekeeping, no milestone needed:** tick or annotate the seven §11 sign-off boxes and the `v0.10.0` tag box in
  `.project/v0.10.0/09-program-acceptance-audit.md`; `/gsd-validate-phase` 22, 24, 28, 29, 30, 34, 36, 36.1 (advisory).

- **Recheck by 2026-10-16:** the two pending todos (`todos/pending/`).
