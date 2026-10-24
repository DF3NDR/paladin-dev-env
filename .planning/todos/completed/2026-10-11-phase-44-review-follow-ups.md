---
completed: 2026-10-24
created: 2026-10-11T18:00:00Z
title: Phase 44 code review follow-ups (Commander/Maneuver bounds docs, crate-wide semver allows, info items)
area: battalion
severity: minor
files:
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-core/Cargo.toml
  - crates/paladin-battalion/Cargo.toml
  - src/infrastructure/resilience/circuit_breaker.rs
owner: repo maintainer
source: Phase 44 code review, 44-REVIEW.md WR-04, WR-05, IN-01..IN-05; fix re-review 44-REVIEW-FIX.md IN-N1, IN-N2
recheck_by: before the Phase 47 v0.11.0 publish
---

## Problem

The Phase 44 code review raised five warnings. WR-01 and WR-02 were fixed on 2026-10-11 (commits
229a28a5 RED and cf0be3db GREEN). WR-03 has its own todo
(`2026-10-11-reject-removed-v0-10-battalion-config-keys.md`). These remain:

- **WR-04:** the Commander no longer bounds ChainOfCommand or Grove, yet its 300 s builder default
  reads as a general bound. `default_maneuver_config` turns the per-attempt `run_timeout` into a
  whole-flow clock. The `BattalionConfig.aegis` rustdoc omits Maneuver, and Maneuver silently
  ignores `on_error` Absorb and `retry`, where Campaign logs a one-time `warn!`.
- **WR-05:** the crate-wide semver allow lines (`struct_missing`, `enum_missing`,
  `enum_variant_missing` in paladin-core; `function_missing`, `module_missing` in paladin-battalion)
  switch those lints off for the whole crate. A future accidental public-API removal would pass
  the semver gate. Add a dated "remove after 0.11.0" note and remove them once 0.11.0 is the
  published baseline.
- **Info:** IN-01 the retry `warn!` says "retrying" even when the backoff is then cancelled; IN-02
  a dead `_started_at` binding in `formation_service.rs`; IN-03
  `test_formation_retry_then_continue` asserts only `log.len() >= 4`; IN-04 duplicate Paladin
  names are ambiguous in `node_errors` (option-a); IN-05 the circuit breaker's rustdoc does not say
  that a Permanent failure neither resets the count nor re-opens a HalfOpen probe.

Raised by the re-review of the WR-01/WR-02 fix (its WR-N1 panic above `Semaphore::MAX_PERMITS`
was fixed in `dc55f18e`):

- **IN-N1:** the `execute_with_cancellation` rustdoc does not say that an attempt already in flight
  finishes and keeps its permit; FirstSuccess ignores `max_concurrency` (pre-existing).
- **IN-N2:** no test pins the permit-granted-as-the-token-fires race or a cancelled
  FirstSuccess/Majority run, and the queued-Paladin test would also pass on the runner's
  pre-attempt check alone, so it does not isolate the permit `select!`.

## Solution

Docs and a one-time `warn!` for WR-04, the dated note for WR-05, and optional cleanups for the info
items. A good fit for `/gsd-quick` or Phase 46 (docs currency).

## Resolution

Resolved by Phase 46 plan 46-09: WR-04, WR-05, IN-05, IN-N1 done; IN-01..IN-04 and IN-N2 (test tightening) deferred per 46-CONTEXT D-24.
