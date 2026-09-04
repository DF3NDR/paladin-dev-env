# Phase 24: Pause/Resume, History & Graceful Shutdown - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-04
**Phase:** 24-pause-resume-history-graceful-shutdown
**Mode:** `--auto` — every gray area auto-selected; every question answered with the recommended option (no `AskUserQuestion` calls). Review CONTEXT.md D-01…D-30 and override at plan review if any default is wrong.
**Areas discussed:** Parley payload & multi-parley Waypoint shape; Gate node & response delivery; Resume validation, partial answers & expiry; Chronicle lineage & fork identity; Shutdown grace mechanics & process wiring; HTTP surface placement & resume semantics; Test placement, tiers & program-gate obligations

---

## Parley payload & multi-parley Waypoint shape

| Option | Description | Selected |
|--------|-------------|----------|
| `AwaitingInput { parleys, responses }`, `ParleyId` newtype, `on_expire` on the request, `Parleyed` outcome | Partial answers persisted on the Waypoint; UUIDv7 newtype mirrors `WaypointId`; per-request policy per HITL-FR-06 (recommended) | ✓ |
| `AwaitingInput { parleys }` only, answers held in process memory | Rejected — partial answers would not survive process termination (HITL-FR-02) | |
| Bare `Uuid` for `parley_id` exactly as the PRD sketch | Rejected — inconsistent with `WaypointId`/`ThreadId`; transparent newtype is wire-identical | |

`[auto] Parley payload — Q: "Where do partial answers live?" → Selected: "on the AwaitingInput Waypoint" (recommended default)`
`[auto] Parley payload — Q: "Parley inside a nested Battalion child?" → Selected: "typed ParleyInChildUnsupported this phase, propagation deferred" (recommended default; PRD 03 silent)`

**Notes:** The `MIGRATION.md` §9.2 `Waypoint` row already sanctions reshaping `AwaitingInput` before first release.

---

## Gate node & response delivery

| Option | Description | Selected |
|--------|-------------|----------|
| Resume = ordinary superstep N+1 with the parleying nodes as Vanguard; Gate writes `output_field`; `parley.` InputMapping namespace; envelope `{"parley": …}`; fingerprint `v4` | Uniform for Gate/Function/Paladin nodes; ENG-FR-11 holds unchanged; mirrors the `muster.` rule (recommended) | ✓ |
| Dedicated "merge responses" pseudo-superstep with no node records | Rejected — a second Waypoint kind and a new ENG-FR-11 clarification | |
| Deliver the response through the Battlefield (a reserved field) | Rejected — leaks the response into shared state and every merge rule | |

`[auto] Gate — Q: "Gate output_field typing?" → Selected: "required for Approval/Choice/FreeText, None for StateEdit, validated at graph validate" (recommended default)`
`[auto] Gate — Q: "Edge evaluation input for a Gate source?" → Selected: "its output_field value, like a Paladin node" (recommended default)`

---

## Resume validation, partial answers & expiry

| Option | Description | Selected |
|--------|-------------|----------|
| Validate everything before any write; `ParleyAlreadyAnswered`; plain `resume` on AwaitingInput is a typed error; `FailRun` persists a `Failed` Waypoint; `ResumeWithDefault` validated early and marked `defaulted` | Total validation, durable consumption, honest expiry (recommended) | ✓ |
| `FailRun` returns the error but leaves the thread suspended | Rejected — the name says the run fails; an unresumable-but-`AwaitingInput` thread would be protected from retention forever | |
| Clock abstraction for expiry tests | Rejected this phase — set `expires_at` in the past explicitly | |

`[auto] Resume — Q: "When are responses consumed?" → Selected: "only when the first post-resume Waypoint persists" (recommended default)`

---

## Chronicle lineage & fork identity

| Option | Description | Selected |
|--------|-------------|----------|
| `fork_of` = branch root, propagated to every Waypoint on the branch; fork's first Waypoint parents to `from`; `latest` unchanged (newest across branches); `replay`/`fork` on `WarEngine`; `ChronicleService` in the facade over `WaypointPort`; forks derive child threads with the branch root | Branch is a queryable attribute; tree reconstructible from summaries (recommended) | ✓ |
| `fork_of` only on the fork's first Waypoint | Rejected — `latest_on_branch` becomes a lineage walk through the port on every call | |
| `ChronicleService` in `paladin-battalion` | Rejected — it needs no engine; a facade application service lets `paladin-web` share the same port reads | |

`[auto] Chronicle — Q: "Subgraph children on a fork?" → Selected: "derived child thread including the branch root; never shares mainline child Waypoints" (recommended default)`

---

## Shutdown grace mechanics & process wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Grace deadline inside the superstep; abort over-grace tasks; `Skipped { reason: "shutdown" }`; re-list in Halted vanguard; `with_shutdown_grace` on `WarEngine`; `EngineConfig.shutdown_grace_secs` + `graceful_shutdown`; `ShutdownCoordinator`/`RunGuard` in battalion; both entry points wired; k8s 60 s | Contract locked, mechanism discretionary (recommended) | ✓ |
| `shutdown_grace` in `EngineLimits` | Rejected — a runtime/operator setting, not a graph property; would raise fingerprint questions | |
| Separate `ShutdownConfig` struct | Rejected — one number would live in two places | |
| Coordinator in the facade only | Rejected — embedded-library users and Phase 27's worker pool need it | |
| Add `shutdown` fields to `Settings` | Rejected — `Settings` is pre-existing, all-pub, not `#[non_exhaustive]` (X-10.3); Phase 22/23 precedent keeps config structs standalone | |

---

## HTTP surface placement & resume semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Separate `ThreadApiState` + `thread_router`; `ParleyPort` in `paladin-ports`; facade adapter spawns the continuation under the coordinator; 202 Accepted; fingerprint-keyed `GraphRegistry`; `WaypointStoreConfig` for the binary; 501 when unwired; `limit` + opaque `cursor` | No X-10 event on `AgentApiState`; honours ADR-0031/0037/0038/0039; Phase 27 re-enqueues under the same shape (recommended) | ✓ |
| Add fields to `AgentApiState` | Rejected — pre-existing all-pub struct; X-10.3 register row + semver allowlist entry for no benefit | |
| `paladin-web` depends on `paladin-battalion` | Rejected — default-build leaf-to-leaf edge (ADR-0031) | |
| Run the resume inline in the request handler, return 200 with the outcome | Rejected — holds a connection for the whole continuation; nothing for HITL-FR-15 to protect | |
| `before=<waypoint_id>` query parameter mirroring `WaypointPort::history` | Rejected — PLAT-06 mandates opaque cursors; naming it `cursor` now avoids a later break | |

`[auto] HTTP — Q: "Unregistered graph on resume?" → Selected: "409 conflict, code graph_not_registered" (recommended default)`

---

## Test placement, tiers & program-gate obligations

| Option | Description | Selected |
|--------|-------------|----------|
| All Tier 1 except Postgres contract additions (Tier 2 via CI); E2E-2 as `tests/integration/e2e_approval_gate_test.rs`; §9.2 `Waypoint` row resolved + deliberate-zero note; new mdBook page; semver/msrv/security/coverage green on the final commit | Mirrors Phases 22/23 (recommended) | ✓ |
| Postgres cases marked passed locally | Rejected — Docker unavailable in the devcontainer (STATE.md carried concern) | |

---

## Claude's Discretion

Module layout for Parley types and the shutdown module; exact error variant names/messages; Gate dispatch mechanism; the `defaulted` marker; `child_on_branch` naming and fork/`restart_on_resume` interaction; DTO/port/registry names and the `state` DTO field set; `TraceEvent` untouched (recommended); `BATTLEFIELD_SCHEMA_VERSION` bump; plan/wave decomposition.

## Deferred Ideas

Parley propagation through nested Battalions; worker-pool re-enqueue of resumes (Phase 27); `WarGraphDoc`/assistant registry (Phase 27); opaque-cursor/scopes policy (Phase 27); expiry scheduler sweep (Phase 27); parley/halt `TraceEvent` variants (Phase 28); WarEngine mdBook page (Phase 22 residual); WR-01/WR-02; qdrant rustdoc item. Reviewed todo not folded: coverage-reproduction check (score 0.2).
