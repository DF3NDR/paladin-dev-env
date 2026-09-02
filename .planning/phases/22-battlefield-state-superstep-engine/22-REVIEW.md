---
phase: 22-battlefield-state-superstep-engine
reviewed: 2026-09-02T00:00:00Z
depth: standard
files_reviewed: 44
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .github/workflows/ci.yml
  - benches/engine_benchmarks.rs
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/engine/bridges.rs
  - crates/paladin-battalion/src/engine/dispatch_registry.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/input_mapping.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/lib.rs
  - crates/paladin-content/Cargo.toml
  - crates/paladin-core/Cargo.toml
  - crates/paladin-core/src/platform/container/battlefield.rs
  - crates/paladin-core/src/platform/container/battlefield_error.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-herald/Cargo.toml
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-memory/Cargo.toml
  - crates/paladin-notifications/Cargo.toml
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-ports/src/output/waypoint_port.rs
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-storage/migrations/postgres/001_create_waypoints_table.sql
  - crates/paladin-storage/migrations/sqlite/001_create_waypoints_table.sql
  - crates/paladin-storage/src/lib.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/mod.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/redact.rs
  - crates/paladin-storage/src/waypoint/retention.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - crates/paladin-web/Cargo.toml
  - docker/docker-compose.test.yml
  - examples/war_engine_memory_baseline.rs
  - src/config/mod.rs
  - src/config/waypoint_retention.rs
  - tests/integration/e2e_crash_resume_test.rs
  - tests/integration/golden_bridge_equivalence_test.rs
  - tests/integration/war_engine_tracer_test.rs
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 22: Code Review Report

**Reviewed:** 2026-09-02T00:00:00Z
**Depth:** standard
**Files Reviewed:** 44
**Status:** issues_found

## Summary

Phase 22 lands the Battlefield/Waypoint/WarEngine superstep engine plus SQLite/Postgres
`WaypointPort` backends and legacy Formation/Phalanx/Campaign bridges. The bulk of the code is
disciplined: bound-parameter SQL everywhere (no injection surface), credential redaction applied
before truncation on every backend error path (matching `security.instructions.md`), structured
error types with no raw value leakage (`BattlefieldError`/`WaypointError` carry type/field names
only), deterministic merge/fingerprint logic backed by real determinism tests (20-seed shuffle,
insertion-order-independence), and no stray `unwrap()`/`panic!` in library code outside
documented, provably-safe invariants.

Two correctness defects rise to BLOCKER: an all-or-nothing waypoint retention routine that is not
actually atomic (a crash or backend failure between its `delete_thread` and its re-`save` loop can
permanently destroy a thread's entire history, including the "never delete" latest/AwaitingInput
waypoints the same module's own doc comment calls "unrecoverable" to lose), and a superstep
scheduler that can silently strand a declared graph node forever (no error, `RunOutcome::Completed`
reported as if the whole graph ran) when a node's only incoming edges trace back to itself with no
externally-anchored path — `WarGraph::validate` performs no reachability check to catch this at
construction time. Four warnings and one info item cover a partially-wired configuration surface,
an undocumented panic precondition, and a narrower data-loss window in the same retention routine.

## Critical Issues

### CR-01: Waypoint retention's delete-then-resave is not atomic — a mid-prune failure can permanently destroy a thread's protected waypoints

**File:** `crates/paladin-storage/src/waypoint/retention.rs:130-151`
**Issue:**
`prune()` implements "keep the latest and every `AwaitingInput` waypoint, delete the rest" by:
1. reading the survivors' full `Waypoint`s via `get()` (lines 137-142),
2. calling `port.delete_thread(&thread_id)` — which deletes **every** waypoint of the thread,
   survivors included (line 144),
3. then `save`-ing each survivor back one at a time (lines 145-147).

This is exactly the sequence the module's own doc comment (lines 1-8) warns against: *"a
retention routine that eats the checkpoint a human is waiting on is unrecoverable."* Yet steps
2-3 are not transactional against any of the three backends (`InMemoryWaypointStore`,
`SqliteWaypointStore`, `PostgresWaypointStore` all implement `WaypointPort` with no cross-call
transaction boundary spanning `delete_thread` + `save`). Concretely:

- If the process crashes, is killed, or the backend connection drops between step 2 and the
  completion of step 3's loop, the thread permanently loses **all** of its waypoints — including
  the one waypoint the routine exists to protect (the latest) and any `AwaitingInput` waypoint a
  human is actively waiting on. There is no recovery path; `delete_thread` has already committed.
- If `port.save(wp)` fails partway through the loop (line 146) — e.g. one transient write error —
  the function returns `Err` via `?`, leaving the thread with only the subset of survivors that
  were re-saved before the failure. The caller has no way to tell from the returned `WaypointError`
  which survivors did or didn't make it back.
- Even without a crash, any concurrent reader (e.g. a live `WarEngine::resume` for this same
  thread, or another `list_threads`/`history` caller) can observe the thread as **completely
  empty** during the window between `delete_thread` and the first `save`, which would surface as
  a spurious `EngineError::ThreadNotFound` for an in-flight workflow.

This directly contradicts the "never a deletion candidate" guarantee documented on `prune`
(lines 69-73) and is a real data-loss risk in a production deployment that enables
`WaypointRetentionConfig` (see WR-02 below for whether anything even calls this today).

**Fix:** Either (a) require backends to expose a per-waypoint delete primitive and delete only the
loser set (never touching survivors at all — the safest fix, though it does reopen the port
surface the module doc says Plan 22-03 already fully specified), or (b) make the
delete-then-resave sequence crash-safe by re-ordering it: `save` every survivor into a *new*
Waypoint-shaped staging record first (or simply skip `delete_thread` entirely and only delete the
loser set one at a time via a new `WaypointPort::delete_waypoint(thread, id)` method), so a
crash at any point leaves the store in a state no worse than "some old waypoints not yet pruned"
rather than "everything gone." At minimum, document the crash-unsafety prominently and make
`prune` retry/verify before returning `Ok`:

```rust
// Prefer: delete only the loser set, never touch survivors.
for id in &delete_ids {
    port.delete_waypoint(&thread_id, id).await?; // new, narrower port primitive
}
// No delete_thread + resave dance, so a crash mid-loop just leaves some
// prunable waypoints un-pruned -- never destroys a survivor.
```

### CR-02: A graph node whose only incoming edges eventually trace back to itself is never marked dead, never becomes ready, and the run silently completes without ever executing it

**File:** `crates/paladin-battalion/src/engine/superstep.rs:582-727` (see also
`crates/paladin-battalion/src/engine/graph.rs:211-249`, `WarGraph::validate`)
**Issue:**
Consider a non-entry node `N` whose only incoming edge is a self-loop (`N -> N`), or more
generally a node whose complete set of incoming edges only ever gets resolved by `N`'s own
execution (directly or transitively through other equally-stranded nodes). Trace the algorithm:

- `Frontier::new` (superstep.rs:586-600) builds `incoming[N] = [self-loop edge index]` and calls
  `propagate_dead`.
- `propagate_dead` (superstep.rs:659-696): `N` is non-entry, `incoming` is non-empty (1 edge), so
  it is not marked dead by the "no incoming edges" rule. `edge_resolution` for the still-`Pending`
  self-loop edge checks `self.dead.contains(source)` where `source == N`; since `N` is not (yet)
  in `dead`, this returns `None` → `any_pending = true` for `N`. Because `any_pending` is true,
  the `!any_pending && !any_fired` dead-marking condition never fires for `N`. `N` is therefore
  **never** marked dead.
- `Frontier::is_ready` (superstep.rs:705-727) for `N`: the only incoming edge is `Pending`
  (`N` has never executed, so its own outgoing self-edge state has never been written) →
  `any_pending = true` → `is_ready` returns `false`.
- `N` can only ever execute if it is placed in the Vanguard, which requires `is_ready(N) == true`
  (via `compute_next_vanguard`, superstep.rs:737-761) or `N` being a declared entry point. Since
  `N` is neither ready nor an entry, `N` never runs — permanently.
- Nothing else in the run depends on `N` failing or blocking: every *other* node's edges resolve
  normally, the run reaches an empty Vanguard, and `superstep::run` returns
  `RunOutcome::Completed` with a `Waypoint` whose `status` is `Completed` — with **zero** signal
  that a declared graph node was never executed.

`WarGraph::validate` (graph.rs:211-249) only checks limits, that every edge/entry endpoint names a
declared node, and that custom dispatch names resolve — it performs no reachability-from-entry
check, so this graph shape passes validation cleanly and produces a "successful" run that silently
omits part of the declared workflow. The only place in the reviewed diff that acknowledges this
failure mode at all is a code comment in
`tests/integration/e2e_crash_resume_test.rs:112-127`, explaining why that test's fixture is
carefully built to avoid it — the engine itself has no guard, and a future `WarGraph` author who
does not know to avoid this shape gets silent data loss (a node's work simply never happens) with
a `Completed` status that looks identical to a fully-successful run.

**Fix:** Add a reachability-from-`entry()` check to `WarGraph::validate` (BFS/DFS over
`graph.edges()` starting from `graph.entry()`, treating a node reachable if some edge from an
already-reachable node targets it): any non-entry node that is not reachable through at least one
non-self edge chain originating at an entry point should fail validation with a new,
specific `EngineError` variant (e.g. `UnreachableNode(NodeId)`) rather than silently completing
without it:

```rust
pub fn validate(&self, custom_dispatch: &CustomDispatchResolver) -> Result<(), EngineError> {
    // ...existing checks...
    let reachable = self.reachable_from_entry(); // BFS over self.edges() from self.entry
    for id in &self.node_order {
        if !self.entry.contains(id) && !reachable.contains(id) {
            return Err(EngineError::UnreachableNode(id.clone()));
        }
    }
    Ok(())
}
```

## Warnings

### WR-01: `WarEngine::with_trace_sink` spawns a background task via `tokio::spawn` with no documented runtime precondition

**File:** `crates/paladin-battalion/src/engine/hooks.rs:98`, `crates/paladin-battalion/src/engine/mod.rs:340-343`
**Issue:** `TraceDispatcher::with_capacity` unconditionally calls `tokio::spawn(...)` (hooks.rs:98)
whenever a sink is supplied. `WarEngine::with_trace_sink` (mod.rs:340-343) calls this from a
plain, non-`async` builder method with no documentation that the caller must be inside an active
Tokio runtime. `tokio::spawn` panics with "there is no reactor running" if called outside one.
Every other `WarEngine` builder method (`with_durability`, `with_parallelism`,
`with_dispatch_rule`, `with_interceptors`, `with_cancellation_token`) has no such precondition, so
this one method is a silent outlier — a caller building a `WarEngine` in a synchronous
constructor/`fn main` (before `#[tokio::main]`'s runtime is entered, or inside a `build()` helper
invoked from sync code) gets an unexplained panic far from any indication that trace-sink
attachment specifically requires it.
**Fix:** Document the precondition on `WarEngine::with_trace_sink`'s rustdoc ("must be called from
within a Tokio runtime context") and/or have `TraceDispatcher::with_capacity` detect the missing
runtime handle (`tokio::runtime::Handle::try_current()`) and return a typed error/no-op with a
clear message instead of letting the `tokio::spawn` panic propagate.

### WR-02: `WaypointRetentionConfig` is fully implemented but never invoked anywhere in the reviewed code

**File:** `src/config/waypoint_retention.rs` (whole file), `src/config/mod.rs:27,49`
**Issue:** `WaypointRetentionConfig` has a complete `Default`, `validate()`, `EnvOverridable`
implementation and is re-exported from `src/config/mod.rs`, but nothing in the reviewed diff (or
the rest of the tree, per a workspace-wide grep for `WaypointRetentionConfig`/`retention::prune`)
ever constructs it from `Settings`, calls `.validate()` outside its own unit tests, or invokes
`paladin_storage::waypoint::retention::prune` with its values. ENG-FR-18 ("retention/cleanup
routine") therefore has a working, tested library function
(`paladin_storage::waypoint::retention::prune`) and a working, tested config type, but no glue
code connecting the two to any scheduler, CLI command, or startup hook — an operator who sets
`APP_WAYPOINT_RETENTION_ENABLED=true` today gets no pruning behavior at all, silently.
**Fix:** Either wire `WaypointRetentionConfig` into a scheduled job (e.g. via `SchedulerConfig`/
`SchedulerPort`, already present in this same `src/config` module) that calls
`retention::prune(&waypoint_port, config.max_age_days, config.max_waypoints_per_thread)` when
`config.enabled`, or — if that wiring is explicitly deferred to a later phase/plan — say so in
this config's module doc so a reader doesn't assume `enabled: true` does something today.

### WR-03: `retention::prune` silently drops a survivor if a concurrent `get()` unexpectedly returns `None`

**File:** `crates/paladin-storage/src/waypoint/retention.rs:137-142`
**Issue:**
```rust
let mut survivors = Vec::with_capacity(keep_ids.len());
for id in &keep_ids {
    if let Some(wp) = port.get(&thread_id, id).await? {
        survivors.push(wp);
    }
}
```
`keep_ids` was just computed from a `history()` call moments earlier, so under normal operation
every `id` should resolve via `get`. But if a concurrent `delete_thread`/eviction/backend
inconsistency causes `get` to return `Ok(None)` for one of these ids, the `if let Some(...)`
silently omits that waypoint from `survivors` — including possibly the thread's *latest* waypoint,
the one this routine's whole contract promises to protect — with no error, warning, or count
recorded anywhere. Combined with CR-01's `delete_thread` step immediately after, that waypoint is
then gone for good.
**Fix:** Treat a `None` here as an unexpected-state error rather than silently skipping it:
```rust
let wp = port.get(&thread_id, id).await?.ok_or_else(|| {
    WaypointError::Backend { source: format!("prune: survivor {id} vanished mid-run for thread {thread_id}").into() }
})?;
survivors.push(wp);
```

### WR-04: `Regex::new` recompiled on every edge-condition evaluation for `EdgeCondition::Regex`

**File:** `crates/paladin-battalion/src/engine/superstep.rs:782-787`
**Issue:** `evaluate_edge_condition` compiles a fresh `Regex` from `pattern` on every call —
once per outgoing edge of every node that completes, every superstep, for the lifetime of a run.
This is not flagged as a performance defect (out of this review's v1 scope), but it is also a
latent correctness/robustness smell: an edge condition with a syntactically invalid pattern is
only ever discovered the first time that edge's source node actually completes and the condition
is evaluated (returning `EngineError::InvalidEdgeCondition` mid-run, after other nodes may already
have produced side effects), rather than at `WarGraph::validate` time, when every other structural
defect in the graph (unknown nodes, bad limits, unregistered custom dispatch) is already caught
up front.
**Fix:** Validate (and ideally pre-compile and cache) every `EdgeCondition::Regex` pattern in
`WarGraph::validate`, so a malformed regex fails fast alongside the graph's other structural
checks rather than mid-run.

## Info

### IN-01: `EngineLimits::run_timeout` is carried and validated but never enforced

**File:** `crates/paladin-battalion/src/engine/graph.rs:85-88`
**Issue:** `EngineLimits.run_timeout` is documented as "Carried and validated but not acted on
this phase — Doc 04 owns timeout semantics," which is an explicit, intentional deferral rather
than an oversight, so this is informational only. Noting it here so a reader of `WarEngine` who
sets `run_timeout` and expects it to be honored today isn't surprised when it silently does
nothing (there is no runtime warning if a caller sets `run_timeout` before Doc 04 lands).
**Fix:** None required for this phase; consider a debug-level log line when a non-`None`
`run_timeout` is configured but not yet enforced, so misconfiguration is at least observable.

---

_Reviewed: 2026-09-02T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
