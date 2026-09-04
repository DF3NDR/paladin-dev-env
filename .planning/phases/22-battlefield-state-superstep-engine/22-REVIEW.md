---
phase: 22-battlefield-state-superstep-engine
reviewed: 2026-09-02T22:22:39Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - .github/workflows/ci.yml
  - Makefile
  - .project/v0.10.0/01-battlefield-state-and-execution-engine.md
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-ports/src/output/waypoint_port.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/mod.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/retention.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - src/application/services/mod.rs
  - src/application/services/waypoint_retention.rs
  - src/config/waypoint_retention.rs
  - tests/integration/e2e_crash_resume_test.rs
  - tests/integration/waypoint_retention_fault_injection_test.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 22: Code Review Report

**Reviewed:** 2026-09-02T22:22:39Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

This phase adds `delete_waypoint`/`prune_thread` to `WaypointPort` across three backends (in-memory,
SQLite, Postgres), rewrites Waypoint retention onto that keep-set primitive with an
application-layer protected-set service, adds eligible-set reachability validation
(`WarGraph::validate`, ENG-FR-02a / BUG-02) with the `mark_dynamic_target` escape hatch, and wires a
live Postgres CI job. The bulk of the work — the `prune_thread`/`delete_waypoint` contract suite,
the monotone/crash-safe retention rewrite, and the reachability fixpoint in `graph.rs` — is careful
and thoroughly tested, including a real fault-injection acceptance test and an end-to-end
crash-resume test.

Two real defects survived that care, both in code this phase's own conventions explicitly warn
against reintroducing:

1. `WarGraph::fingerprint()` — the value `WarEngine::resume`'s safety check is built on — does not
   hash the `defer_flags` this phase's sibling scheduling logic (`compute_next_vanguard`) reads live
   from the graph, so a defer-flag change between an interrupted run and its resume is invisible to
   the fingerprint check that is documented to make "a divergence between fresh and resumed
   execution... structurally impossible."
2. `InMemoryWaypointStore::list_threads` determines a thread's "latest" Waypoint by `Vec` position
   rather than by `created_at`/`superstep` order, unlike every sibling method in the same file and
   unlike both SQL backends' window-function queries — an inconsistency the `WaypointPort::save`
   upsert contract this very phase documents can trigger directly.

A third, process-level finding: the `Makefile`'s local `test-integration-docker` target starts
`postgres-test` and runs the Postgres suite with none of the readiness-wait or SKIP-detection
machinery the equivalent CI job (added in this same phase) uses — the exact "green because nothing
was exercised" failure shape this codebase's own CI comments repeatedly call out for Ollama and
now Postgres.

## Critical Issues

### CR-01: `WarGraph::fingerprint()` omits `defer_flags`, breaking resume's "structurally impossible divergence" guarantee

**File:** `crates/paladin-battalion/src/engine/graph.rs:398-437` (fingerprint), cf. `crates/paladin-battalion/src/engine/graph.rs:120-121,163-167` (`defer_flags`) and `crates/paladin-battalion/src/engine/superstep.rs:742,756` (`compute_next_vanguard` reading `graph.is_deferred`)

**Issue:** `WarEngine::resume`/`resume_with_options` (`crates/paladin-battalion/src/engine/mod.rs:448-461`) documents that resuming from a Waypoint "continu[es] from the superstep after the loaded Waypoint's, so a divergence between fresh and resumed execution is structurally impossible." That guarantee rests entirely on the `graph_fingerprint` equality check: a mismatch fails resume (or requires an explicit `allow_graph_change` opt-in). But `WarGraph::fingerprint()` hashes only node ids, edge specs (`from`/`to`/`condition`), and schema field names — it never touches `self.defer_flags` (`WarGraph::add_deferred_node`, ENG-FR-06). `defer_flags` is not cosmetic: `superstep::compute_next_vanguard` reads `graph.is_deferred(target)` live, every superstep, from whatever `WarGraph` instance is passed to `run` — including on resume, where it is read from the caller-supplied graph, not from anything persisted in the Waypoint.

Concretely: construct graph G1 with node X registered via `add_node` (not deferred), run it partway, crash. Construct graph G2 — identical node ids, edges, and schema, so `G2.fingerprint() == G1.fingerprint()` — but with X registered via `add_deferred_node` instead. `WarEngine::resume(&G2, thread)` passes the fingerprint check (nothing detects the change) and proceeds to schedule remaining supersteps using G2's defer semantics: X now waits until no non-deferred node is ready, exactly the aggregate-after-all-branches behavior ENG-FR-06 describes — a real scheduling divergence from what continuing the original G1 run would have produced, with no error and no signal that anything changed. `mark_dynamic_target` (`dynamic_targets`, also new in this phase) has the same fingerprint blind spot, though its practical impact is smaller since `validate()` is always re-run fresh on both `start` and `resume` and would independently reject an unreachable node.

No existing test exercises this: `fingerprint_is_deterministic_across_calls` and `fingerprint_is_unchanged_by_insertion_order` (`graph.rs:637-678`) cover determinism and insertion-order-independence, never a defer-flag or dynamic-target difference.

**Fix:** Fold `defer_flags` (and, for completeness, `dynamic_targets`) into the fingerprint's hashed bytes, sorted the same deterministic way the existing node/edge/field lists already are:

```rust
pub fn fingerprint(&self) -> GraphFingerprint {
    // ...existing node_ids / edges / field_names...

    let mut deferred: Vec<&NodeId> = self.defer_flags.iter().collect();
    deferred.sort();
    let mut dynamic: Vec<&NodeId> = self.dynamic_targets.iter().collect();
    dynamic.sort();

    let mut buf = Vec::new();
    // ...existing nodes/edges/schema sections...
    buf.extend_from_slice(b";deferred:");
    for id in &deferred {
        buf.extend_from_slice(id.as_str().as_bytes());
        buf.push(b'|');
    }
    buf.extend_from_slice(b";dynamic_targets:");
    for id in &dynamic {
        buf.extend_from_slice(id.as_str().as_bytes());
        buf.push(b'|');
    }

    GraphFingerprint::from_canonical_bytes(&buf)
}
```

Add a regression test mirroring `fingerprint_is_unchanged_by_insertion_order`: two graphs identical in nodes/edges/schema, differing only in whether one node is registered via `add_deferred_node`, must produce **different** fingerprints.

## Warnings

### WR-01: `InMemoryWaypointStore::list_threads` picks "latest" by storage position, not by time — inconsistent with `latest()`/`history()` and with both SQL backends

**File:** `crates/paladin-storage/src/waypoint/in_memory.rs:132-160`

**Issue:** `latest()` (lines 71-82) and `history()` (lines 95-130) both explicitly sort by `(created_at, superstep)` descending, with an inline comment on `history` noting "do not rely on insertion order, which `save`'s upsert can disturb." `list_threads` does not follow its own file's rule:

```rust
let mut summaries: Vec<ThreadSummary> = threads
    .iter()
    .filter_map(|(thread_id, wps)| {
        wps.last().map(|latest| ThreadSummary { ... })
    })
    .collect();
```

`wps.last()` returns the physically last-pushed entry of the thread's `Vec<Waypoint>`. `save`'s upsert (lines 55-69) replaces an existing `waypoint_id` **in place** — it never moves the updated entry to the end of the `Vec` — so `wps.last()` is "most recently appended," not "greatest `created_at`/`superstep`." `SqliteWaypointStore`/`PostgresWaypointStore::list_threads` both use a `ROW_NUMBER() OVER (PARTITION BY thread_id ORDER BY created_at DESC, superstep DESC)` window function — the correct, order-independent definition — so this is also a cross-backend inconsistency, in violation of this module's own "identical suite across backends" convention (ENG-FR-17, enforced everywhere else via the shared `contract_tests` suite).

Trigger: any upsert of an earlier-position Waypoint with a later `created_at` (a legal use of the documented `save` upsert contract — e.g. a correction/backfill, or simply constructing fixtures with explicit `created_at` values as this phase's own `contract_tests::sample_waypoint_at` does throughout) makes `list_threads` report a stale Waypoint's `status`/`last_updated_at` as the thread's most recent activity, while `latest()`/`history()` on the same store correctly report the true latest. The existing contract test (`list_threads_empty_then_three_threads_newest_activity_first`) only exercises one Waypoint per thread, so it cannot catch this.

**Fix:** Compute the per-thread latest the same way `latest()`/`history()` already do, e.g.:

```rust
wps.iter()
    .max_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.superstep.cmp(&b.superstep))
    })
    .map(|latest| ThreadSummary { ... })
```

Add a contract-suite regression case (`prune_thread`'s sibling tests are a good model): seed a thread with an out-of-time-order upsert and assert `list_threads`'s reported `last_updated_at`/`latest_status` match the true newest Waypoint, run against all three backends.

### WR-02: `make test-integration-docker`'s Postgres leg has none of the readiness/SKIP guards the CI job (added in this same phase) requires

**File:** `Makefile:139-146`

**Issue:** The `postgres-integration` CI job (`.github/workflows/ci.yml:811-886`) exists specifically because an earlier, unguarded invocation of this suite "executed in no environment at all while every CI run reported green" (per the job's own comment) — and it fixes that with three explicit guards: wait for `docker inspect ... healthy`, assert `pg_isready`, and fail if the test log contains `SKIP:` (the marker `store_or_skip()` in `postgres.rs` prints when it cannot reach the server). The `Makefile` target added alongside it reintroduces exactly the gap the CI job was built to close:

```make
@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) up -d postgres-test
@WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test \
	$(CARGO) test -p paladin-storage --features postgres --lib waypoint::postgres -- --test-threads=1 --nocapture
```

`up -d` returns as soon as the container is started, not once its healthcheck passes (`docker-compose.test.yml`'s `postgres-test` service does define one: `pg_isready`, 5s interval). If `cargo test` reaches the reachability probe (`postgres_reachable`, 750ms TCP timeout) before Postgres finishes starting, every test in the suite silently takes the `SKIP:` path and the run still exits 0 — `make test-integration-docker` reports success having exercised none of the 27 Postgres contract tests. This target's own Ollama block, three lines above, gets this right (`timeout 300 sh -c 'until docker inspect ... healthy...'`); the Postgres block copies its structure but not its wait.

**Fix:** Mirror the Ollama block's wait, and mirror the CI job's SKIP detection:

```make
@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) up -d postgres-test
@timeout 180 sh -c 'until docker inspect paladin-postgres-test --format="{{.State.Health.Status}}" 2>/dev/null | grep -qx healthy; do sleep 3; done'
@WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test \
	$(CARGO) test -p paladin-storage --features postgres --lib waypoint::postgres -- --test-threads=1 --nocapture 2>&1 | tee /tmp/postgres-waypoint-test.log
@! grep -q 'SKIP:' /tmp/postgres-waypoint-test.log || { echo "postgres suite took the SKIP path"; exit 1; }
```

### WR-03: `superstep::run`'s `RecursionLimitExceeded`/dispatch-conflict `failed_node` attribution is arbitrary

**File:** `crates/paladin-battalion/src/engine/superstep.rs:249-262,489-497`

**Issue:** `WaypointStatus::Failed { failed_node, .. }` requires naming one node, but a recursion-limit failure isn't caused by any single node — `failed_node: vanguard[0].clone()` (line 253) names whichever node happens to be first in `vanguard`'s current order. Similarly, the merge-failure path (line 494) falls back to `vanguard[0]` when `ran` is empty. This is not incorrect per the type's contract (some node must be named), but it can mislead an operator debugging a runaway loop into treating an arbitrary co-participant as "the" cause, especially with multiple concurrently-looping nodes.

**Fix:** Not a required fix given the schema constraint, but consider documenting on `WaypointStatus::Failed::failed_node` (or in a comment at this call site) that for `RecursionLimitExceeded` the named node is representative, not necessarily causal, so downstream tooling/alerting doesn't over-index on it.

## Info

### IN-01: `WaypointRetentionConfig::apply_env_overrides` does not re-run `validate()`

**File:** `src/config/waypoint_retention.rs:64-76`

**Issue:** `validate()` rejects `Some(0)` for either bound, but `apply_env_overrides` writes `APP_WAYPOINT_RETENTION_MAX_AGE_DAYS`/`APP_WAYPOINT_RETENTION_MAX_WAYPOINTS_PER_THREAD` straight into the config with no re-validation. If the surrounding config-loading pipeline calls `validate()` only once, before env overrides are applied (a common ordering bug in layered config systems), an operator setting either env var to `0` would silently reach `WaypointRetentionService::prune` with a policy the type's own `validate()` was written to reject. Not verifiable from the files in this review's scope — the loader that sequences `apply_env_overrides` vs. `validate()` is outside them — so this is flagged for confirmation rather than asserted as a live bug.

**Fix:** If the loader does not already do so, call `validate()` again after `apply_env_overrides()` for this config (and confirm the pattern for its sibling configs, e.g. `CitadelConfig`, which this module says it mirrors).

### IN-02: `WarEngine::with_parallelism(0)` silently becomes 1

**File:** `crates/paladin-battalion/src/engine/superstep.rs:311`

**Issue:** `let limit = parallelism.unwrap_or(vanguard.len()).max(1);` — a caller who explicitly configures `with_parallelism(0)` (perhaps expecting a "pause new work" semantic) instead gets exactly the same behavior as `with_parallelism(1)`, with no warning or error. `EngineLimits::validate` rejects zero for `max_supersteps`/`max_node_visits` explicitly; `WarEngine::with_parallelism` has no equivalent guard and clamps silently instead.

**Fix:** Either document on `with_parallelism` that `0` is treated as `1` (current behavior, made explicit), or reject `0` the same way `EngineLimits`'s zero-checks do, for consistency with how this codebase treats other zero-as-degenerate-config cases.

---

_Reviewed: 2026-09-02T22:22:39Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
