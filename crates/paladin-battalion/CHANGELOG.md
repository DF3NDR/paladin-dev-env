# Changelog

All notable changes to `paladin-battalion` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- Phase 42 (mid-run halt, ALLOW-03; ADR-0057): `HaltCause` (`CancelRequested`, `Token`,
  `Spend(HaltReason)`) on `RunOutcome::Halted`, and `WarEngine::with_spend_guard`, which consults
  the attached `SpendGuard` at every superstep boundary (a check-only read: a halt keeps the
  Halted Waypoint, runs no node of that superstep, and a child battalion run halts its parent).
  `RunOutcome::Halted` gains a `cause` field (see `MIGRATION.md` §9.2).

- `TraceDispatcher::total_cost()`, the synchronous twin of `total_usage()`, folding every priced
  `TraceEvent::NodeFinished` via `CostTally::record_node` inside `emit()` itself (PRICE-03).
- `WarEngine::with_treasury_ledger(ledger, SettlementContext)` — settles exactly one aggregated
  ledger row per superstep attempt, synchronously and awaited at the superstep boundary, including
  nested Battalion child runs (rolled into the parent superstep, never settling on their own) and
  Aegis-retried nodes (only the eventual successful attempt contributes); a ledger failure is
  logged and never fails, retries or halts the run (LEDGR-03, LEDGR-04).

### Fixed

- A boundary settlement that cannot be written (a ledger error or a currency mismatch) is now
  reported to an attached `SpendGuard` through `SpendGuard::note_unsettled_spend`, so a metering
  guard can halt the run instead of letting it spend unmetered (Phase 42 review WR-4). Settlement
  itself is unchanged: it still never fails, retries or halts a run.
- A nested `NodeSpec::Battalion` child that halts (cancel, drain or a spend halt) is no longer
  recorded as a successful node with an empty delta. The parent now halts in the same superstep
  with the child's own `HaltCause`, and its `Halted` Waypoint re-lists the Battalion node so a
  resume re-enters the child's own `Halted` Waypoint. Previously a TERMINAL Battalion node let the
  parent finish `Completed`, and a non-terminal one was resumed past the child's unfinished work
  (Phase 42 review CR-1).

### Changed

- All five `WarEngine` `RunFinished` emission sites now populate `cost: trace.total_cost()`
  beside `usage: trace.total_usage()`, and each real Paladin-attempt `NodeFinished.cost` carries
  that attempt's own priced `PaladinResult.cost` — never a placeholder — so a run's total cost is
  a real, end-to-end figure (PRICE-03).

## [0.10.1] - 2026-09-20

### Fixed

- The two workspace `[dev-dependencies]` (`paladin-llm`, `paladin-storage`) that carried a
  version requirement, and so pointed forward in the publish order — this crate publishes before
  either of them — are now path-only. Cargo omits a path-only dev-dependency with no version
  requirement from the published manifest entirely, so this crate's published manifest genuinely
  differs from `0.10.0`'s even though no library source changed; local dev/test builds still
  resolve both by path, unchanged. This was the defect that stopped the real `v0.10.0` release
  after 3 of 12 crates; see the root `CHANGELOG.md`'s `[0.10.1]` section.

## [0.10.0] - 2026-09-10

### Added
- `WarEngine`/`WarGraph`, the new graph/superstep execution engine (ENG-01…ENG-08): per-superstep
  `Waypoint` checkpointing, `EngineLimits` (`max_supersteps`, `max_node_visits`,
  `max_muster_tasks`), dynamic routing/fan-out/subgraphs (`StateNode`'s `Directive` return type,
  `Muster` worker templates, Phase 23 CF), pause/resume "Parley" human-in-the-loop gates
  (Phase 24 HITL), and the `Aegis` per-node fault-tolerance sidecar — retry, per-attempt
  `run_timeout`/`idle_timeout`, `Route`/`Absorb`/`Custom` error handlers, node result caching,
  all opt-in per node via `WarGraph::set_aegis`/`with_default_aegis` (Phase 25 FT, D-30). Legacy
  `FormationExecutionService`/`PhalanxExecutionService`/`CampaignExecutionService`/`Commander`
  paths are untouched by this addition (X-03).
- `EdgeConditionEvaluator` registry — `CampaignEdge`/`WarGraph` custom-edge evaluation is now
  fail-closed: an unregistered `EdgeCondition::Custom(name)` fails validation instead of silently
  evaluating `true` (CF-FR-01, fixes BUG-01; see root `MIGRATION.md` M-B-01).

### Changed
- `Commander`/`CommanderBuilder` gained a new `StrategySelection` option via an additive
  `CommanderBuilder::strategy_selection` builder method; no public field was added to `Commander`
  (CF-FR-19).
- `CampaignExecutionService` gained a new evaluator-registry builder method
  (`with_evaluator`); its `new(paladin_port)` constructor is unchanged (CF-FR-01).

See root `MIGRATION.md` §9.2 for the full X-10 compatibility register and requirement IDs.

## [0.9.0] - 2026-09-01

## [0.8.1-rc.5] - 2026-08-31

## [0.8.1-rc.4] - 2026-08-29

### Added
- Crate-level release artifacts for Epic 4 API stabilization.
- Changelog tracking for orchestration patterns (Formation, Phalanx, Campaign, Chain of Command, Conclave, Council, Grove, Maneuver, Commander).

### Changed
- Public API stability documentation aligned with crate-tier stability expectations.

### Fixed
- Crate metadata and README linkage validated for crates.io release preparation.
