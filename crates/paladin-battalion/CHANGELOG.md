# Changelog

All notable changes to `paladin-battalion` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project follows lockstep workspace versioning.

## [Unreleased]

### Added

- Phase 51 (REASON-05; plan 51-09): the superstep engine fills `TraceEvent::NodeFinished`'s
  `reasoning_requested` and `reasoning_effective` from the attempt's own `run_metadata`
  (`paladin.reasoning_requested` / `paladin.reasoning_effective`) on a Paladin attempt and, from
  `structured.raw.run_metadata`, on a structured-output attempt. A function or battalion node, a
  cache hit and a failed attempt leave both `None`.

- Phase 50 (G-50-5a remainder (b), PARITY-05; plan 50-29): the superstep engine counts the spend of
  an attempt however it ends. `race_attempt` runs every attempt inside a fresh
  `paladin_ports::output::spend_scope::ATTEMPT_SPEND` scope held outside the attempt future; an
  attempt that fails with any `PaladinError`, or that the run deadline or idle bound cuts off,
  reports the usage and cost its Paladin recorded (every call it was billed for) on its
  `NodeFinished` and the superstep boundary settles it once per attempt (LEDGR-03/04, retries never
  double counted), where before it reported the neutral pair and settled nothing. The
  structured-output arm reports the recorded sum of every repair call on success too (its `raw`
  carries only the last call), falling back to `raw`'s figures when nothing was recorded. A
  successful Paladin attempt still reports `PaladinResult.cost` alone. No public item changes.

- Phase 43 (rate pacing, the Cadence; PACE-04; ADR-0058): `WarEngine::with_cadence(port,
  lock_ttl)`, which puts the Cadence stampede lock around the node cache's miss-to-store window.
  With a node cache attached, concurrent identical cache misses execute a cache-policy node once and
  the other dispatches serve the stored delta as a cache hit; the winner stores through
  `NodeCachePort::put_fenced` and releases the lock on every exit, a waiter whose holder released
  without writing takes over within one poll, and a lock error, an elapsed `lock_ttl` or a
  cancelled run never fails a node. The engine depends on the `CadencePort` trait only.

- `TraceDispatcher::with_seq_origin(origin)`, which starts the dispatcher's `seq` after `origin`
  (default `0`, so every existing caller is unchanged). The run worker seeds it from the thread's
  largest persisted `seq`, so a fork or a requeued drained run on the same thread numbers its trace
  records after the earlier run's instead of colliding with them (Phase 42 review WR-6).

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

- A Battalion node whose nested child run halted (a spend ceiling or a cancel) is now recorded
  `Skipped { reason: "child_halted" }` on the Halted Waypoint and in its `NodeFinished` trace
  event, where the CR-1 fix reused `"shutdown"` and so contradicted the run's own halt reason.
  Resume keys off the re-listed vanguard, not the string (Phase 42 review IN-12).
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

- Phase 46 (D-24 / WR-04; plan 46-09): a `Commander` running a Maneuver logs one `warn!` per
  execution when `aegis.on_error` or `aegis.retry` is set, naming the Battalion and the ignored
  field(s), because Maneuver reads only `aegis.timeout.run_timeout` (as one whole-flow clock). The
  decision is the private pure function `maneuver_ignored_aegis_policies`. The `Commander` and
  `CommanderBuilder` rustdoc state that the Commander adds no bound of its own.

- Phase 51 (REASON-01; plan 51-09): the node-cache key of a Paladin node appends the agent's
  `reasoning_effort` and `reasoning_fallback` when set (research Pitfall 7), so an answer produced
  at one effort is never served at another. An agent that sets neither composes the same key as
  before; `NODE_CACHE_KEY_VERSION` is still `k1`.

- Phase 50 (PARITY-05; ADR-0061): the LLM failure classifiers handle the new `LlmError::Refusal`
  variant. `llm_failure` converts it to a `PaladinError::LlmFailure` that names its provider by
  value with no HTTP status and `Permanent` transience (a fallback chain stops on it unless built
  with `hop_on_refusal(true)`), and `LlmDecisionEvaluator`'s error class label for it is `"refusal"`. No
  public item of this crate changed.

- All five `WarEngine` `RunFinished` emission sites now populate `cost: trace.total_cost()`
  beside `usage: trace.total_usage()`, and each real Paladin-attempt `NodeFinished.cost` carries
  that attempt's own priced `PaladinResult.cost` — never a placeholder — so a run's total cost is
  a real, end-to-end figure (PRICE-03).
- Phase 44 (legacy clean break, LEGACY-01, LEGACY-02; ADR-0059): Formation, Phalanx, Campaign and
  the Conclave run every Paladin, expert or aggregator attempt through one crate-private runner
  (`aegis_attempt`), bounded per attempt by `BattalionConfig.aegis.timeout` (no whole-Battalion
  clock) and retried under `aegis.retry` with the engine's backoff. Formation and Phalanx continue
  past a failure under `aegis.on_error = Absorb`, record the typed `NodeError` in
  `BattalionResult.node_errors` and report `Completed`; a timed-out final attempt is a
  `BattalionError::Node` (Formation, Campaign) or an entry in `AggregationError` (Phalanx).
  Campaign has no continue mode and warns once when `on_error` is set. See `MIGRATION.md` 9.1
  M-B-06 and M-B-07.
- Phase 44 (LEGACY-01, LEGACY-03; ADR-0059): the Commander no longer wraps `execute()` in a
  timeout; a bare `CommanderBuilder` bounds each attempt at 300 s; it bridges `aegis` into the
  Conclave (`retry_attempts = max_attempts - 1`, clamped to 5) and defaults its Maneuver to
  `maneuver::ErrorStrategy::FailFast`. The Conclave retries an expert only when
  `PaladinError::transience()` is `Transient`, waits with `engine::retry::backoff_delay`, and
  reports an aggregator timeout as `ConclaveError::AggregatorFailed`. See `MIGRATION.md` 9.1
  M-B-09 and M-B-11.

### Removed

- Phase 44 (LEGACY-01; ADR-0059; a clean break with no shim, see `MIGRATION.md` 9.2): the public
  `paladin_battalion::retry` module with its `should_retry` and `calculate_retry_delay`, and the
  facade's matching re-export. Use `aegis.retry` and `engine::retry::backoff_delay`.

## [0.10.1] - 2026-09-21

### Fixed

- The two workspace `[dev-dependencies]` (`paladin-llm`, `paladin-storage`) that carried a
  version requirement, and so pointed forward in the publish order — this crate publishes before
  either of them — are now path-only. Cargo omits a path-only dev-dependency with no version
  requirement from the published manifest entirely, so this crate's published manifest genuinely
  differs from `0.10.0`'s even though no library source changed; local dev/test builds still
  resolve both by path, unchanged. This was the defect that stopped the real `v0.10.0` release
  after 3 of 12 crates; see the root `CHANGELOG.md`'s `[0.10.1]` section.

## [0.10.0] - 2026-09-18

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
