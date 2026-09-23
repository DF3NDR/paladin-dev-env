---
phase: 24-pause-resume-history-graceful-shutdown
reviewed: 2026-09-05T12:01:21Z
depth: standard
files_reviewed: 51
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/v0.10.0/08-traceability-matrix.md
  - crates/paladin-battalion/src/engine/directive_parser.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/input_mapping.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/shutdown.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-core/src/platform/container/directive.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/parley.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-ports/src/input/mod.rs
  - crates/paladin-ports/src/input/parley_port.rs
  - crates/paladin-ports/src/output/waypoint_port.rs
  - crates/paladin-storage/src/waypoint/contract_tests.rs
  - crates/paladin-storage/src/waypoint/in_memory.rs
  - crates/paladin-storage/src/waypoint/postgres.rs
  - crates/paladin-storage/src/waypoint/retention.rs
  - crates/paladin-storage/src/waypoint/sqlite.rs
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_auth.rs
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/lib.rs
  - crates/paladin-web/src/openapi.rs
  - crates/paladin-web/src/thread_controller.rs
  - docs/src/SUMMARY.md
  - docs/src/deployment/kubernetes.md
  - docs/src/deployment/production.md
  - docs/src/user-guides/parley-and-chronicle.md
  - k8s/README.md
  - k8s/deployment.yaml
  - k8s/server/deployment.yaml
  - src/application/services/chronicle.rs
  - src/application/services/mod.rs
  - src/application/services/parley/adapter.rs
  - src/application/services/parley/mod.rs
  - src/application/services/parley/registry.rs
  - src/application/services/waypoint_retention.rs
  - src/bin/paladin-server.rs
  - src/config/engine.rs
  - src/config/mod.rs
  - src/config/setup/service_runner.rs
  - src/config/waypoint_store.rs
  - tests/integration/e2e_approval_gate_test.rs
  - tests/integration/multi_parley_suspension_test.rs
  - tests/integration/parley_resume_stress_test.rs
  - tests/integration/subgraph_formation_in_campaign_test.rs
  - tests/integration/waypoint_retention_fault_injection_test.rs
findings:
  critical: 0
  warning: 3
  info: 0
  total: 3
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-09-05T12:01:21Z
**Depth:** standard
**Files Reviewed:** 51
**Status:** issues_found

## Summary

This is the post-fix re-review of Phase 24 (Parley pause/resume, Chronicle history/branch
inspection, graceful shutdown, and the `paladin-web` thread HTTP surface). The prior review
(2026-09-05T08:25Z) found 2 critical and 3 warning findings; `24-REVIEW-FIX.md` reports all five
fixed. I independently re-verified both critical fixes against the current tree rather than
trusting the fix report:

- **CR-01** (`/v1/threads/*` had no per-thread authorization): confirmed fixed.
  `resume_thread` (`crates/paladin-web/src/thread_controller.rs:548-594`) now calls
  `crate::agent_auth::require_admin(&principal)` before doing anything else, and the accompanying
  HTTP-level tests (`post_resume_with_non_admin_role_is_403`, `post_resume_with_admin_role_is_202`)
  exercise the 403/202 split. The reads (`get_thread_state`, `get_thread_history`) remain
  authenticated-any-role by deliberate, documented design (see WR-03 below for the residual risk
  this narrowing leaves open).
- **CR-02** (a shutdown-grace abort mid-Muster discarded `MusterProgress`): confirmed fixed.
  `crates/paladin-battalion/src/engine/superstep.rs` now tracks `muster_task_aborted` separately
  from ordinary `aborted_node_ids` (lines 1645-1720), skips re-adding the aborted Muster worker's
  `NodeId` to `halted_vanguard`, preserves the round's `MusterProgress` (lines 2129-2168) instead
  of hard-coding `None`, and additionally gates the completed-task fold-into-`deltas` step on
  `!muster_task_aborted` (lines 2009-2028) to avoid the double-merge-on-resume regression the fix
  report describes finding while implementing this. The regression test
  `shutdown_grace_abort_mid_muster_preserves_progress_for_resume` covers it.
- **WR-01/WR-02/WR-03** (registry lock `.unwrap()`, `shadow_validate` drift guard, silent losing
  race): all three fixes are present and match their described shape in
  `src/application/services/parley/registry.rs` and
  `src/application/services/parley/adapter.rs`.

Since that review, only `crates/paladin-web/src/openapi.rs` changed in the reviewed scope (a
`"403"` status literal added to the resume-path status-set test, plus a rustdoc sentence) — this
is consistent with the CR-01 fix and introduces no new issue.

I did not stop at re-verifying the prior findings. A fresh pass over the storage layer, the
graceful-shutdown coordinator, the Parley/Chronicle value types, and the config layer surfaced
three new WARNING-class findings, none of them re-treads of anything already fixed: an ordering
bug in `InMemoryWaypointStore::list_threads` that diverges from the same store's own `latest()`/
`history()` methods and from the SQL backends' correct behavior; a `Mutex::lock().expect(...)`
pair in `paladin-battalion`'s trace dispatcher that is the same "panic on lock poison in library
code" anti-pattern WR-01 fixed elsewhere in this same phase, left unaddressed here; and the
already-flagged, already-documented residual IDOR exposure on the two GET thread routes, which
CR-01's fix deliberately left open pending PLAT-06 and which I am re-surfacing here as a tracked,
open risk rather than a regression.

No SQL injection, hardcoded credential, unsafe deserialization, or new `unwrap()`/`expect()`/
`panic!` in non-test/non-binary library code was found beyond what is listed below. The SQLite and
Postgres `WaypointPort` implementations use bound parameters throughout (verified against their own
adversarial `...with_sql_metacharacters_round_trip_as_data` tests), and the `ThreadId::child`/
`child_on_branch` length-prefixed encoding correctly avoids the delimiter-collision hazard its own
rustdoc describes fixing.

## Warnings

### WR-01: `InMemoryWaypointStore::list_threads` computes "latest" by insertion order, not by the documented `created_at`/`superstep` ordering its own `latest()` and `history()` methods use

**File:** `crates/paladin-storage/src/waypoint/in_memory.rs:133-161` (specifically line 142,
`wps.last()`)

**Issue:** `WaypointPort::list_threads`'s contract (`crates/paladin-ports/src/output/waypoint_port.rs:219-234`)
states: "Ordering is descending `last_updated_at` (the `created_at` of each thread's latest
waypoint)." `InMemoryWaypointStore::latest` (`in_memory.rs:72-83`) and `::history`
(`in_memory.rs:96-131`) both honour this correctly, via an explicit `max_by`/`sort_by` over
`created_at` (with `superstep` as tiebreak) — never insertion order. `::list_threads`, however,
computes each thread's summary via `wps.last()` (plain `Vec` insertion order), assuming the last
element pushed is also the most-recently-created one:

```rust
let mut summaries: Vec<ThreadSummary> = threads
    .iter()
    .filter_map(|(thread_id, wps)| {
        wps.last().map(|latest| ThreadSummary {   // <-- insertion order, not created_at order
            thread_id: thread_id.clone(),
            latest_status: latest.status.clone(),
            last_updated_at: latest.created_at,
        })
    })
    .collect();
```

`save`'s own contract is upsert-in-place for an existing `waypoint_id` and append for a new one
(`in_memory.rs:56-70`), so `wps`'s Vec order is exactly the order `save()` was called in, not the
order `created_at` values fall in. This phase's own `latest_prefers_most_recently_created_across_branches`
contract test (`contract_tests.rs:898-925`) proves the two orders CAN legitimately diverge for a
single thread — a fork's own Waypoint can carry a `created_at` earlier or later than a mainline
waypoint already saved for the same `ThreadId`, independent of which was `save()`d first. The same
`WR-03` scenario this phase's own `adapter.rs` test suite exercises (two concurrent `resume_with`
calls racing against the same thread) is exactly the shape of event that can make append order and
`created_at` order diverge in production: a task's `created_at` is captured before its async
continuation completes, so completion (and thus `save()` call) order is not guaranteed to match
`created_at` order under concurrent writers.

The practical blast radius is bounded today: no HTTP route in this phase's `thread_controller.rs`
calls `list_threads`, and `waypoint_retention.rs`'s `prune()` (the one production caller) only reads
`thread_summary.thread_id` from the result, never `latest_status`/`last_updated_at` — so this bug
does not currently corrupt retention behaviour. But `InMemoryWaypointStore` is this crate's
reference/test-default implementation (used throughout this phase's own test suite as the
substitute for a real backend), and the two production-grade backends
(`sqlite.rs`'s `LIST_THREADS_QUERY_NO_CURSOR`/`_WITH_CURSOR`, `postgres.rs`'s equivalents) compute
this correctly via a `ROW_NUMBER() OVER (PARTITION BY thread_id ORDER BY created_at DESC, superstep
DESC)` window function — making the in-memory store the one backend whose `list_threads` violates
its own trait's documented contract. There is no test in `in_memory.rs`'s own suite analogous to
`latest_prefers_most_recently_created_across_branches` that would have caught this (the existing
`list_threads_empty_then_three_threads_newest_activity_first` test only ever saves ONE waypoint per
thread, in increasing `created_at` AND insertion order, so it cannot distinguish the two orderings).

**Fix:** Compute each thread's most-recent waypoint the same way `latest()` does — `max_by` over
`(created_at, superstep)` — rather than `Vec::last()`:

```rust
let mut summaries: Vec<ThreadSummary> = threads
    .iter()
    .filter_map(|(thread_id, wps)| {
        wps.iter()
            .max_by(|a, b| {
                a.created_at
                    .cmp(&b.created_at)
                    .then_with(|| a.superstep.cmp(&b.superstep))
            })
            .map(|latest| ThreadSummary {
                thread_id: thread_id.clone(),
                latest_status: latest.status.clone(),
                last_updated_at: latest.created_at,
            })
    })
    .collect();
```

Add a regression test mirroring `latest_prefers_most_recently_created_across_branches`, but
asserting on `list_threads`'s `latest_status`/`last_updated_at` for a thread whose waypoints are
saved out of `created_at` order.

### WR-02: `TraceDispatcher`'s background consumer panics on lock poisoning via `.expect(...)`, the same anti-pattern WR-01 fixed elsewhere in this phase

**File:** `crates/paladin-battalion/src/engine/hooks.rs:105,146`

**Issue:** Both the background consumer task and `TraceDispatcher::emit` call
`queue.buffer.lock().expect("trace queue mutex poisoned")`. CLAUDE.md and
`.github/instructions/rust.instructions.md` both state "Avoid `unwrap()`/`expect()` and `panic!` in
library code — return `Result`," and this same phase's own fix for WR-01
(`src/application/services/parley/registry.rs:71-74`) replaced an identical
`RwLock::write().unwrap()` pattern with `.unwrap_or_else(std::sync::PoisonError::into_inner)`,
explicitly reasoning that "a panic elsewhere while some OTHER writer held this lock must not
cascade into every subsequent call ... safe here: the map/buffer itself cannot be left torn by a
panic mid-`insert`/`get`" — an argument that applies identically to `TraceQueue.buffer`, whose only
operations are `push_back`/`pop_front` on a `VecDeque`. If anything ever panics while holding this
lock (e.g. a future change to the critical section, or a panic propagating from an adjacent
`std::sync::Mutex`-guarded operation added later), `TraceDispatcher::emit` — the one function the
whole superstep loop calls for every trace event, on every run sharing this dispatcher — starts
panicking on every subsequent call, for the lifetime of the process. This crate's own module doc
for this exact file states the design goal plainly: "A slow or permanently blocking `TraceSink`
must never stall a run, and a sink erroring on every call must never fail one" — a poisoned-lock
panic is a strictly worse failure mode than either of those, since it is not scoped to one run.

**Fix:** Recover the guard on poison, exactly as WR-01 did for `GraphRegistry`:

```rust
let mut buf = consumer_queue
    .buffer
    .lock()
    .unwrap_or_else(std::sync::PoisonError::into_inner);
```

and the identical change at `emit`'s `queue.buffer.lock()` call. Add a test mirroring
`register_and_resolve_survive_a_poisoned_lock` (`registry.rs`), poisoning `TraceQueue.buffer` from
a panicking thread and asserting `emit`/the consumer still function afterward.

### WR-03: The two read-only thread routes still perform no per-thread authorization — an accepted, documented gap, re-surfaced here as a still-open risk

**File:** `crates/paladin-web/src/thread_controller.rs:470-518` (`get_thread_state`), `596-668`
(`get_thread_history`); module doc `1-46`

**Issue:** CR-01's fix closed the mutating route (`POST .../resume`) behind `require_admin`, but
`get_thread_state` and `get_thread_history` remain authenticated-any-role by deliberate decision,
documented in the module doc, in `docs/src/user-guides/parley-and-chronicle.md:301-308`, and in the
fix report. This is not an oversight — it is a consciously scoped, tracked interim posture pending
Phase 27's `PLAT-06` per-thread ownership. I am re-flagging it, at WARNING rather than CRITICAL,
because the underlying exposure the original CR-01 finding described (any authenticated caller,
including the lowest-privileged configured role, can read the full `ThreadStateResponse` —
including every outstanding `ParleyRequestDto.payload` and `ParleyResponseDto.value`/
`responded_by` — and the entire `GET .../history` page set, for *any* thread by id) is still
exactly true for these two routes today. `ThreadId`/`Waypoint` still carry no owner/principal field
(confirmed: `crates/paladin-core/src/platform/container/waypoint.rs` has no such field), so nothing
in this call path can scope a read to "threads this caller is entitled to see." A deployment that
configures more than one role/tenant behind these routes is trusting every credential holder with
every thread's contents, for reads, indefinitely until `PLAT-06` lands.

**Fix:** No action required to "fix" this finding in isolation — it is accepted, tracked scope
narrowing, not a regression. Confirm `PLAT-06` (Phase 27) is still on the roadmap and scoped to
close this gap; until then, keep the operator-facing warning in
`docs/src/user-guides/parley-and-chronicle.md` prominent, and consider a minimal interim mitigation
(e.g. gating the two read routes behind the same `require_admin` check the resume route now uses,
narrowing the read surface too, at the cost of requiring admin-equivalent credentials for read-only
polling) if a multi-tenant deployment is anticipated before `PLAT-06` ships.

---

_Reviewed: 2026-09-05T12:01:21Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
