# Changelog

All notable changes to `paladin-battalion` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

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
