---
phase: 24-pause-resume-history-graceful-shutdown
verified: 2026-09-05T09:15:42Z
status: gaps_found
score: 5/6 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "Phase-closing documentation (CHANGELOG.md, MIGRATION.md §9.6, docs/src/user-guides/parley-and-chronicle.md) accurately reflects the final, as-shipped authorization posture of POST /v1/threads/{id}/resume."
    status: partial
    reason: >
      24-REVIEW.md's CR-01 (IDOR-class: any authenticated role could resume any thread) was fixed
      in commit 00b1e552, gating resume_thread behind require_admin and correctly documenting the
      narrowing in thread_controller.rs's own module rustdoc. That fix commit lands AFTER plan
      24-12's documentation commits (86a4c85b, c81805c8) closed the phase's CHANGELOG/MIGRATION/
      user-guide obligations, and no commit since has touched any of the three files to reflect
      the narrower posture. All three now state or imply the pre-fix "any authenticated role"
      behavior for resume, which is false against the current tree.
    artifacts:
      - path: "CHANGELOG.md:117-124"
        issue: "States the three thread routes sit \"behind the same authentication middleware /v1/agents/* already uses\" with no mention that POST .../resume additionally requires an admin role (CR-01) — reads as if resume is open to any authenticated role, which it is not."
      - path: "MIGRATION.md:200-206 (§9.6 status-code table)"
        issue: "POST /v1/threads/{id}/resume's status-code list is `202, 400, 401, 404, 409, 501` — omits `403`, which the admin-gate now returns for a non-admin caller. The table is the X-10-style registry this phase itself uses elsewhere to declare a change complete; it is now incomplete for its own resume row."
      - path: "docs/src/user-guides/parley-and-chronicle.md:299-303"
        issue: "The \"Interim authorization posture\" callout says thread routes \"accept any authenticated caller regardless of role — there is no admin/writer scope distinction on who may answer an approval gate or inspect a thread's history yet\". This is now false for resume: answering an approval gate (POST .../resume) requires an admin-role credential as of commit 00b1e552."
    missing:
      - "Add a CHANGELOG.md Unreleased bullet (or amend the existing HITL-05 entry) noting POST /v1/threads/{id}/resume requires an admin-role credential pending PLAT-06, and that GET .../state and .../history remain open to any authenticated role."
      - "Add `403` to MIGRATION.md §9.6's status-code list for POST /v1/threads/{id}/resume, with a one-line note pointing at the admin-gate and its PLAT-06 successor (mirroring thread_controller.rs's own module doc)."
      - "Update docs/src/user-guides/parley-and-chronicle.md's \"Interim authorization posture\" callout to say resume specifically requires an admin-role credential (reads stay any-role), matching the CR-01 fix's actual shipped behavior."
human_verification:
  - test: "Read the CR-02 fix diff (crates/paladin-battalion/src/engine/superstep.rs, commit 9802ce60) end to end: the muster_task_aborted bookkeeping, the aborted_muster_progress construction in the Halted-Waypoint branch, and the newly-conditional completed-task fold (`if !muster_task_aborted`)."
    expected: "The merge-ordering interaction between a shutdown-grace abort mid-Muster and the pre-existing 'fold this muster's completed deltas into `deltas`' step is correct for every reachable case (a muster round aborted with zero, some, or all sibling tasks already complete; a muster round that finishes normally in the same superstep another abort happens elsewhere), not just the one scenario the new regression test (`shutdown_grace_abort_mid_muster_preserves_progress_for_resume`) exercises."
    why_human: "The phase's own code-review-fix report (24-REVIEW-FIX.md) self-flags this exact fix as \"requires human verification\": a logic/state-handling change to the core superstep join loop, covered by one new deterministic test plus the full existing suite (564 unit tests, 6 named integration binaries — all independently re-run and confirmed green during this verification), but subtle enough that the fixer itself asked for a human diff read before the phase proceeds. This is a judgment call about a state-merging edge case, not something an additional grep or test run resolves.
---

# Phase 24: Pause/Resume, History & Graceful Shutdown Verification Report

**Phase Goal:** A workflow can pause indefinitely for human input without holding compute, resume
from a different process, expose an inspectable and forkable Chronicle, shut down without losing
in-flight work, and be driven over HTTP.
**Verified:** 2026-09-05T09:15:42Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria, HITL-01…05)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A node (or a first-class `Gate` node) raising a `ParleyRequest` suspends the run, persists an `AwaitingInput` Waypoint carrying all of the superstep's parleys, releases every resource, and is resumable from a different process sharing the backend (HITL-01) | ✓ VERIFIED | `parley.rs` (333 lines), `WaypointStatus::AwaitingInput{parleys,responses}` at `waypoint.rs:574`, `NodeSpec::Gate` in `graph.rs`; `tests/integration/e2e_approval_gate_test.rs` (`e2e2_approval_branch_survives_process_drop`, `e2e2_denial_branch_survives_process_drop`, `e2e2_suspended_thread_holds_no_engine_resources`) and `multi_parley_suspension_test.rs` re-run clean (30 + 31 passed, 0 failed) |
| 2 | Program scenario E2E-2 passes: `resume_with` validates typed responses per kind with typed errors, honors `expires_at`, routes both branches of an approval gate across a process drop/recreate (HITL-02) | ✓ VERIFIED | `WarEngine::resume_with` at `mod.rs:1350`; `e2e_approval_gate` test binary re-run clean; `parley_resume_stress_test.rs` (10-thread concurrency, X-05) re-run clean (29 passed) |
| 3 | History/inspect over `WaypointPort` supports `replay` and `fork`-with-edit, `fork_of` lineage, byte-identical original chain, branch-aware `latest` (HITL-03) | ✓ VERIFIED | `WarEngine::replay`/`fork` at `mod.rs:1648`/`1670`; `Waypoint.fork_of` at `waypoint.rs:703`; `ChronicleService` (389 lines, `src/application/services/chronicle.rs`, `cargo tree -p paladin-web -e normal` confirms no `paladin-battalion` edge); `subgraph_formation_in_campaign` test binary re-run clean (33 passed) |
| 4 | Graceful shutdown finishes the in-flight superstep within `shutdown_grace`, records over-grace nodes `Skipped` and re-lists them, `resume` continues a `Halted` thread, SIGTERM/SIGINT wired with `k8s/` manifests + docs + disable switch (HITL-04) | ✓ VERIFIED | `ShutdownCoordinator` (493 lines, `engine/shutdown.rs`); `terminationGracePeriodSeconds: 60` in both `k8s/deployment.yaml` and `k8s/server/deployment.yaml`; `resume_continues_a_halted_thread_after_process_shutdown` test re-run clean; CR-02's mid-Muster abort fix re-verified live in code (see Human Verification below) |
| 5 | `GET/POST/GET /v1/threads/{id}/{state,resume,history}` reachable over HTTP with 409/400/404 semantics, `openapi.json` regenerated (HITL-05) | ✓ VERIFIED | `thread_controller.rs` (1437 lines); three `/v1/threads/*` paths present in `openapi.json`; `openapi_lists_the_three_thread_paths`/`openapi_thread_paths_document_every_status` re-run clean; `cargo tree -p paladin-web -e normal` shows no `paladin-battalion` |
| 6 | Phase-closing documentation accurately reflects the final, as-shipped authorization posture (derived from D-29/D-30's registration obligation and plan 24-12's own must-haves) | ✗ FAILED | See Gaps — CHANGELOG.md, MIGRATION.md §9.6 and the mdBook user guide were all last touched before CR-01's fix landed and still describe the pre-fix "any authenticated role" posture for `POST /v1/threads/{id}/resume` |

**Score:** 5/6 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/paladin-core/src/platform/container/parley.rs` | Parley value types | ✓ VERIFIED | 333 lines, `ParleyId`/`ParleyKind`/`OnExpire`/`ParleyRequest`/`ParleyResponse` |
| `crates/paladin-core/src/platform/container/waypoint.rs` | `AwaitingInput{parleys,responses}`, `fork_of`, fingerprint v4 | ✓ VERIFIED | `WaypointStatus::AwaitingInput` line 574; `fork_of` line 703; `GRAPH_FINGERPRINT_VERSION = "v4"` line 361 |
| `crates/paladin-battalion/src/engine/mod.rs` | `resume_with`, `replay`, `fork`, `with_shutdown_grace` | ✓ VERIFIED | all four present at lines 1350/1648/1670/1049 |
| `crates/paladin-battalion/src/engine/shutdown.rs` | `ShutdownCoordinator`/`RunGuard` | ✓ VERIFIED | 493 lines, created this phase |
| `crates/paladin-ports/src/input/parley_port.rs` | `ParleyPort`, `ParleyError` | ✓ VERIFIED | 390 lines; signature matches D-25 (core types only) |
| `src/application/services/chronicle.rs` | `ChronicleService` | ✓ VERIFIED | 389 lines, port-only (`Arc<dyn WaypointPort>`), no `paladin-battalion` dependency |
| `src/application/services/parley/{adapter,registry}.rs` | Facade adapter, `GraphRegistry` | ✓ VERIFIED | 1134 + 174 lines; poisoned-lock recovery (WR-01) confirmed live |
| `crates/paladin-web/src/thread_controller.rs` | Thread HTTP routes | ✓ VERIFIED | 1437 lines; admin-gate on resume (CR-01) confirmed live |
| `crates/paladin-web/openapi.json` | Regenerated with 3 thread paths | ✓ VERIFIED | `/v1/threads/{id}/history`, `/resume`, `/state` all present |
| `k8s/deployment.yaml`, `k8s/server/deployment.yaml` | `terminationGracePeriodSeconds: 60` | ✓ VERIFIED | both files, line 34 |
| `docs/src/user-guides/parley-and-chronicle.md` | mdBook page, wired into SUMMARY.md | ⚠️ STALE | Exists (345 lines) and is wired in, but its authorization callout is stale (see Gaps) |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `NextStep::Parley` (superstep.rs) | `AwaitingInput` Waypoint | `build_waypoint`/`persist_waypoint` | ✓ WIRED | confirmed in `superstep.rs`, exercised by `parley_suspends_run_and_persists_awaiting_input` |
| `WarEngine::resume_with` | paused node continuation | `NodeContext.parley_response` | ✓ WIRED | `resume_with_rejects_wrong_shape_per_kind` and E2E-2 tests pass |
| `ThreadId::child_on_branch` | fork's subgraph child thread | branch-scoped id derivation | ✓ WIRED | `subgraph_formation_in_campaign` suite (`fork_does_not_touch_mainline_child_waypoints`, `latest_on_a_fork_child_thread_does_not_resolve_the_mainline_child`) passes |
| SIGTERM/SIGINT | `ShutdownCoordinator::cancel_and_wait` | `shutdown_signal`/`ServiceRunner::wait_for_shutdown` | ✓ WIRED | `resume_continues_a_halted_thread_after_process_shutdown` passes |
| `ParleyPort` (paladin-ports) | `WarEngine::resume_with` | `ParleyPortAdapter` facade | ✓ WIRED | `shadow_validate_agrees_with_the_real_engine_across_fixtures` (WR-02 drift guard) passes |
| `thread_router` | `require_authentication` + `require_admin` | `route_layer` in `thread_controller.rs` | ✓ WIRED | `post_resume_with_non_admin_role_is_403`, `post_resume_with_admin_role_is_202`, `get_thread_state_with_non_admin_role_is_200_not_403` all pass |
| Shipped code's authorization posture | Phase-closing docs (CHANGELOG/MIGRATION/user-guide) | doc registration | ✗ NOT_WIRED | code changed (CR-01) after docs were written; docs never regenerated/updated |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| HITL-01 | 24-01, 24-02, 24-03, 24-05, 24-12 | Pause without holding compute; `Gate` node; cross-process resume | ✓ SATISFIED | Suspension spine + Gate node live; `e2e_approval_gate`/`multi_parley_suspension` pass |
| HITL-02 | 24-01, 24-03, 24-04, 24-05, 24-12 | `resume_with` typed validation, `expires_at`/`on_expire` | ✓ SATISFIED | Validation matrix in `mod.rs`; E2E-2 passes |
| HITL-03 | 24-06, 24-07, 24-12 | Chronicle `replay`/`fork`, `fork_of` lineage, byte-identical mainline | ✓ SATISFIED | `replay`/`fork` + `ChronicleService`; `subgraph_formation_in_campaign` passes |
| HITL-04 | 24-08, 24-09, 24-12 | Graceful shutdown, `ShutdownCoordinator`, k8s wiring | ✓ SATISFIED | `shutdown.rs`; CR-02 fix verified live (human read still requested, see below) |
| HITL-05 | 24-10, 24-11, 24-12 | Thread HTTP routes, `ParleyPort`, `openapi.json` | ✓ SATISFIED | `thread_controller.rs`; three paths in `openapi.json`; CR-01 admin-gate live |

No orphaned requirements — every ID plans declare (`requirements:` frontmatter) matches REQUIREMENTS.md's Phase 24 assignment exactly (HITL-01…05, all five).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| CHANGELOG.md | 117-124 | Stale claim: resume sits behind "the same authentication middleware /v1/agents/* already uses" | ⚠️ Warning | Misleads an integrator into believing resume is open to any authenticated role |
| MIGRATION.md | ~204 | Incomplete status-code registry (missing `403`) for POST resume | ⚠️ Warning | X-10-style registry incomplete for its own subject |
| docs/src/user-guides/parley-and-chronicle.md | 299-303 | Stale "no admin/writer scope distinction" callout | ⚠️ Warning | Directly contradicts shipped code (`require_admin` on resume) |

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers found in any of the ~35 phase-modified source files scanned. No stub patterns (`return null`/empty-body handlers/hardcoded-empty stub props) found in the checked artifacts.

### Behavioral Spot-Checks / Test Re-Runs

All re-run independently in this verification session (not taken from SUMMARY claims):

| Behavior | Command | Result | Status |
|---|---|---|---|
| CR-02 regression (mid-Muster shutdown-grace abort) | `cargo test -p paladin-battalion shutdown_grace_abort_mid_muster_preserves_progress_for_resume` | 1 passed | ✓ PASS |
| CR-01 admin-gate (403/202/200) | `cargo test -p paladin-web --all-features --lib thread_controller::tests::{post_resume_with_non_admin_role_is_403,post_resume_with_admin_role_is_202,get_thread_state_with_non_admin_role_is_200_not_403}` | 3 passed | ✓ PASS |
| WR-01 poisoned-lock recovery | `cargo test application::services::parley::registry::tests::register_and_resolve_survive_a_poisoned_lock` | 1 passed | ✓ PASS |
| WR-02 shadow-validate drift guard | `cargo test application::services::parley::adapter::tests::shadow_validate_agrees_with_the_real_engine_across_fixtures` | 1 passed | ✓ PASS |
| WR-03 losing-race logging | `cargo test application::services::parley::adapter::tests::losing_background_race_is_discarded_without_panicking` | 1 passed | ✓ PASS |
| Six named integration binaries | `cargo test --test {e2e_crash_resume,e2e_approval_gate,multi_parley_suspension,parley_resume_stress,subgraph_formation_in_campaign,e2e_muster_defer_order,waypoint_retention_fault_injection}` | all green (27/30/31/29/33/30/3 passed) | ✓ PASS |
| `paladin-web` has no `paladin-battalion` dependency | `cargo tree -p paladin-web -e normal \| grep battalion` | no output | ✓ PASS |
| `cargo fmt --all --check` | — | clean | ✓ PASS |
| `cargo clippy --workspace --all-targets --features web-server -- -D warnings` | — | clean | ✓ PASS |
| Postgres contract cases self-skip locally (D-28) | `cargo test -p paladin-storage --features postgres awaiting_input_payload_round_trips` | passes trivially (no Docker) | ✓ PASS |

### Human Verification Required

1 item (does not affect overall `gaps_found` status, which is already forced by the documentation gap above, but must still be surfaced):

1. **CR-02 mid-Muster shutdown-grace abort fix — self-flagged by the fixer as requiring a human diff read**
   - **Test:** Read `crates/paladin-battalion/src/engine/superstep.rs`'s CR-02 diff (commit `9802ce60`) — the `muster_task_aborted` bookkeeping, `aborted_muster_progress` construction, and the newly-conditional `if !muster_task_aborted` fold step — against every reachable Muster-abort shape (zero/some/all siblings already complete when the abort lands; a Muster round completing normally in the same superstep as an unrelated node's abort).
   - **Expected:** No double-merge, no dropped sibling-task progress, no re-dispatch of an aborted worker with `NodeContext.muster == None`, across every shape — not just the one scenario `shutdown_grace_abort_mid_muster_preserves_progress_for_resume` (re-run clean in this verification) exercises.
   - **Why human:** `24-REVIEW-FIX.md` itself records this fix's status as `"fixed: requires human verification"` — a state-merging edge case in the core superstep join loop that the fixer judged subtle enough to ask for a human read before the phase proceeds, despite full existing-suite green (564 unit tests + 6 integration binaries, independently re-confirmed here).

### Gaps Summary

The phase's functional delivery is real and independently re-verified: every one of the five
roadmap Success Criteria (HITL-01 through HITL-05) is backed by substantive, wired code and
passing tests that this verification re-ran itself rather than trusting SUMMARY.md's claims. The
two BLOCKER-class findings from `24-REVIEW.md` (CR-01's missing per-thread authorization, CR-02's
dropped `MusterProgress` on a mid-Muster shutdown abort) and all three WARNING-class findings
(WR-01/02/03) are fixed in commits `00b1e552` through `19d2e820`, confirmed live in the current
tree, and independently re-tested here (not merely re-read from `24-REVIEW-FIX.md`'s prose).

The one gap blocking a clean `passed` verdict is process, not functional: the phase's three
documentation deliverables that are supposed to be the authoritative record of what shipped
(CHANGELOG.md, MIGRATION.md §9.6, and the mdBook user guide) were all last edited by plan 24-12
*before* the code review found and fixed CR-01, and none has been touched since. All three still
describe (or fail to update) the pre-fix "any authenticated role" posture for
`POST /v1/threads/{id}/resume`, which the shipped code no longer implements — `resume_thread` now
requires an admin-role credential. This is a straightforward three-line-diff fix (add the 403 row
and a note to MIGRATION.md §9.6, add a CHANGELOG bullet, update the mdBook callout), not a design
or engineering gap, but it leaves the phase's own security-relevant documentation actively
contradicting its shipped behavior — exactly the kind of drift a verifier should not wave through
silently.

---

_Verified: 2026-09-05T09:15:42Z_
_Verifier: Claude (gsd-verifier)_
