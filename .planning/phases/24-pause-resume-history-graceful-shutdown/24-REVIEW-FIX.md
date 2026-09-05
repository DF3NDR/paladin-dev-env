---
phase: 24-pause-resume-history-graceful-shutdown
fixed_at: 2026-09-05T09:02:23Z
review_path: .planning/phases/24-pause-resume-history-graceful-shutdown/24-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 24: Code Review Fix Report

**Fixed at:** 2026-09-05T09:02:23Z
**Source review:** .planning/phases/24-pause-resume-history-graceful-shutdown/24-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (2 critical, 3 warning; `fix_scope: critical_warning`)
- Fixed: 5
- Skipped: 0

## Fixed Issues

### CR-01: `/v1/threads/*` routes have no per-thread authorization

**Files modified:** `crates/paladin-web/src/thread_controller.rs`, `crates/paladin-web/openapi.json`
**Commit:** `00b1e552`
**Status:** fixed

**Applied fix:** Extracted `Extension<Principal>` in all three thread handlers
(`get_thread_state`, `resume_thread`, `get_thread_history`) and gated the one
*mutating* route, `POST /v1/threads/{id}/resume`, behind
`crate::agent_auth::require_admin`, matching `agent_controller.rs`'s existing
admin-gated idiom. `GET .../state` and `GET .../history` remain
authenticated-any-role, exactly as D-24 specified — this narrows D-24's "any
role" default for the mutating route only, pending PLAT-06's real per-thread
ownership scoping (documented in the module doc-comment and each handler's
own rustdoc, plus a rationale note directly on `resume_thread`).

Added two oneshot HTTP-level tests (`post_resume_with_non_admin_role_is_403`,
`post_resume_with_admin_role_is_202`) asserting 403-vs-202 by role, plus
`get_thread_state_with_non_admin_role_is_200_not_403` pinning that reads stay
open to any authenticated role. Regenerated the committed `openapi.json`
baseline via the existing `UPDATE_OPENAPI=1` drift-guard mechanism — only the
resume path gained a `403` response entry; the pre-existing agent paths were
verified byte-identical by the existing
`openapi_pre_existing_agent_paths_are_unchanged` drift-guard test, which still
passes.

**Note on scope narrowing:** this deliberately narrows D-24's locked decision
("the same auth middleware as `/v1/agents/*`, authenticated callers, any
role; scopes are PLAT-06") for the resume route specifically. The review
(CR-01) and the fix instructions for this task explicitly called for this
narrowing ahead of PLAT-06; it is recorded in `thread_controller.rs`'s module
doc under a new "Authorization (narrows D-24 pending PLAT-06)" section so the
divergence from the locked decision is discoverable, not silent.

### CR-02: A shutdown-grace abort mid-Muster discards `MusterProgress`

**Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
**Commit:** `9802ce60`
**Status:** fixed: requires human verification

**Applied fix:** Tracked which aborted dispatch-order ids came from a Muster
task separately from ordinary aborted vanguard nodes (`muster_task_aborted:
bool`, alongside the pre-existing `aborted_node_ids: Vec<NodeId>`). The
Halted-Waypoint branch now (1) skips re-adding a Muster worker's `NodeId` to
`halted_vanguard` — it is recovered via `MusterProgress::unfinished_tasks()`
instead — and (2) preserves the round's in-flight `MusterProgress` (built
from the in-scope `muster_node`/`muster_tasks`/`muster_completed_so_far`)
instead of hard-coding `None`.

**Beyond the review's illustrative snippet:** implementing this fix surfaced
a second, related bug the review's sketch did not call out: the existing
"fold this muster's completed-task deltas into `deltas`" step
(`superstep.rs`, immediately before the unconditional `battlefield.merge`
call) ran regardless of whether the round actually finished. Combined with
the CR-02 fix as originally sketched, this double-merged every already-
completed sibling task's delta on resume (first merged into the aborted
round's own `battlefield` before the Halt, then re-merged from
`MusterProgress.completed` on the resumed round) — caught by the new
regression test failing with duplicated results
(`["a","a","b","b","c","d","d","e","e"]` instead of
`["a","b","c","d","e"]`) before this second fix was added. The fold step is
now additionally gated on `!muster_task_aborted`, deferring it to the
resumed round exactly once, matching the already-tested crash/resume
contract. Ordinary (non-muster) peers that complete in the same aborted
round are unaffected — their deltas already merge unconditionally and
correctly, per the pre-existing `two_slow_nodes_share_one_deadline` test.

Flagged `requires human verification` per this agent's own verification
protocol: this is a logic/state-handling fix in the core superstep loop, and
while it is covered end-to-end by a new, deterministic regression test
(`shutdown_grace_abort_mid_muster_preserves_progress_for_resume`) plus the
full existing `paladin-battalion` suite (564 unit tests) and the six named
integration binaries, the merge-ordering interaction it touches is subtle
enough to warrant a human read of the diff before this phase proceeds to
verification.

**Verification run:** `cargo test -p paladin-battalion` (564 passed, 0
failed) plus `cargo test --test e2e_crash_resume`,
`e2e_muster_defer_order`, `subgraph_formation_in_campaign`,
`e2e_approval_gate`, `multi_parley_suspension`, `parley_resume_stress` — all
green.

### WR-01: `GraphRegistry` panics on lock poisoning via `.unwrap()`

**Files modified:** `src/application/services/parley/registry.rs`
**Commit:** `b643bb6a`
**Status:** fixed

**Applied fix:** Replaced both `.write().unwrap()` and `.read().unwrap()`
with `.unwrap_or_else(std::sync::PoisonError::into_inner)`, exactly as the
review's fix suggestion specified — safe here since a plain `HashMap` cannot
be left torn by a panic mid-`insert`/`get`. Added
`register_and_resolve_survive_a_poisoned_lock`, which poisons the lock from a
dedicated panicking thread and asserts both `register` and `resolve` still
work afterward instead of panicking.

### WR-02: `shadow_validate` has no automated drift guard against the real engine

**Files modified:** `src/application/services/parley/adapter.rs`
**Commit:** `19d2e820`
**Status:** fixed

**Applied fix:** Added `shadow_validate_agrees_with_the_real_engine_across_fixtures`,
a fixture-table test running both `shadow_validate` and a real,
directly-constructed `WarEngine::resume_with` call (bypassing the adapter
entirely) over seven parley/response/expiry combinations — a complete
single-approval submission, a partial two-outstanding-parley submission, the
three rejection kinds (unknown parley id, already answered, invalid response
shape), and both `OnExpire` policies (`FailRun` short-circuit,
`ResumeWithDefault` completing the round) — asserting `shadow_validate`'s
`Complete` prediction agrees with whether the real engine's worker node
actually ran (measured via a run-counting `StateNode`, not by interpreting
the `RunOutcome` variant). A future validation-ordering or `EngineError`/
`ParleyKind`/`OnExpire` change in `paladin-battalion` that silently desyncs
this adapter now fails in this crate's own test suite.

### WR-03: A losing concurrent `resume_with` race is silently discarded

**Files modified:** `src/application/services/parley/adapter.rs`
**Commit:** `d8827122`
**Status:** fixed

**Applied fix:** The spawned background task's `engine.resume_with(...)`
result is now inspected; on `Err`, logs at `log::warn!` with the thread id
and the error, rather than discarding it via `let _ = ...`. This is the
minimum observability fix the review asked for — the caller already received
`202 Accepted` and cannot be told, but an operator can now detect the race
from server logs.

Added `losing_background_race_is_discarded_without_panicking`, using a new
`RaceSimulatingWaypointStore` test double (a `WaypointPort` wrapper that
fabricates an already-`Completed` status on every `latest()` call after the
first) to deterministically force the spawned background call to lose,
asserting: the caller still receives `202`, the `ShutdownCoordinator` still
drains the guard (no stuck/leaked registration), and the underlying store is
left untouched by the discarded error (never overwritten with something
else). The log message's *content* is not independently asserted — this
crate has no existing log-capture test harness (checked: no
`testing_logger`/`logtest`/`log::set_logger`-based test infrastructure
anywhere in the codebase, and adding a new test-only logging dependency for
one line felt disproportionate) — but the exact code path the fix adds is
exercised deterministically by this test.

## Skipped Issues

None — every in-scope finding was fixed.

## Post-fix verification

All commands specified in this task's `<project_constraints>` were run after
every commit and again at the end, on the final tree:

- `cargo fmt --all --check` — clean
- `cargo clippy --workspace --all-targets --features web-server -- -D warnings` — clean
- `cargo test -p paladin-web` — 138 + 5 + 0 (doctests) passed, 0 failed
- `cargo test --features web-server --bin paladin-server` — 11 passed, 0 failed
- `make test` (`cargo test --workspace --lib --bins`) — every reported suite green (552, 11, 456, 564, 96, 1, 43, 110, 76, 0, 111, 118, 138 passed; 0 failed across all)
- `cargo test -p paladin-battalion` — 564 unit + 45 doc tests passed, 0 failed
- `cargo test --test e2e_crash_resume` / `e2e_muster_defer_order` / `subgraph_formation_in_campaign` / `e2e_approval_gate` / `multi_parley_suspension` / `parley_resume_stress` — all green

No pre-existing public field was added to `Settings` or `AgentApiState`; no
`paladin-web` → `paladin-battalion` dependency was introduced; no
`unwrap()`/`expect()`/`panic!` was added to library code (WR-01 removed the
only newly-flagged instance); every new/changed error path uses the
project's existing typed error enums.

---

_Fixed: 2026-09-05T09:02:23Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
