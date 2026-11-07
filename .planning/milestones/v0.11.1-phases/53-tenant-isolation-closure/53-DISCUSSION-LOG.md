# Phase 53: Tenant Isolation Closure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-30
**Phase:** 53-tenant-isolation-closure
**Mode:** interactive selection of all four areas (first session, paused at question 1), then
`--auto` resume: the recommended option was taken for every question without prompting. The
WHAT-level contracts were answered interactively in `/gsd-spec-phase 53` the same day.
**Areas discussed:** Thread ownership source + version call; Gate shape + run-less threads;
Corrupt row / Serialization policy; Jobs, boot assertion, small guards

---

## Thread ownership source + version call

| Option | Description | Selected |
|--------|-------------|----------|
| Derive from the thread's runs | Latest run's `submitted_by`; no schema or public-type change; stays v0.11.1; N+1 on list | ✓ (auto, recommended) |
| Record a tenant on the thread | Column on `waypoints` or a threads table + pub fields on `Waypoint`/`ThreadSummary` + `WaypointPort` change; registered break; v0.12.0 | |
| Derive now, record later | Option 1 plus a deferred idea | (deferred idea recorded anyway) |

Follow-ups (auto): reshape-to-additive before registering any break; rule lives as an inherent
method on `RunReadScope`; list filtering by bounded over-fetch rather than a new port method.

---

## Gate shape + run-less threads

| Option | Description | Selected |
|--------|-------------|----------|
| `load_visible_thread` helper in `thread_controller.rs` | Mirrors `load_visible_run`; service half stays in `ensure_thread_visible` | ✓ (auto, recommended) |
| Application-layer visibility port/service | One shared service used by controller and submission | |

Follow-ups (auto): gate before `require_admin` on resume/delete (SC1 names them); no run
repository wired ⇒ fail closed for non-admin; route matrix mirrors `published_run_id_operations`.

---

## Corrupt row / Serialization policy

| Option | Description | Selected |
|--------|-------------|----------|
| `Serialization` role-dependent, others fixed 500 | 404 for `Tenant` scope, fixed 500 for `All`; outages stay 500 for all; lists stay 500 | ✓ (auto, recommended) |
| All repository errors role-dependent | Any error hidden as 404 for non-admin | |

(The observable contract itself — 404 non-admin / fixed 500 Admin — was chosen by the user in
spec-phase round 1.)

---

## Jobs, boot assertion, small guards

| Option | Description | Selected |
|--------|-------------|----------|
| Job owner inside `JobStore` private state, additive methods | `JobRecord` unchanged; `create_owned`/`get_visible` | ✓ (auto, recommended) |
| Owner field on `JobRecord` | Public struct change on paladin-web — a break | |
| Pure assertion in `build_run_api` + pool inspector | Named error before listener bind; pure-function tests | ✓ (auto, recommended) |
| Assertion inside `RunWorkerPool::spawn` | Signature change — a break | |
| Reserved literals as exported core constant, checked in `AuthConfig::validate` | `TenantId::new` unchanged | ✓ (auto, recommended) |
| Reject in `TenantId::new` | Behavior change on a published constructor | |
| `NULL created_by` scan pages the existing `list` | No port change; advisory; stop on page error | ✓ (auto, recommended) |
| New `list_unattributed` port method | Trait change — a break | |
| Short ADR-0063 amending ADR-0054 | Records thread, job, fail-closed and corrupt-row rules | ✓ (auto, recommended) |

---

## Claude's Discretion

Method and function names; D-04 page size and helper sharing; module-doc wording; evidence
file naming; whether WINDOWS rows 58/63 close here or in Phase 57 with the evidence pointer.

## Deferred Ideas

Record the tenant on the thread; repository-side scoped thread listing; tenant-namespaced thread
ids; `created_by` backfill/PATCH (TENANT-11). Four keyword-matched todos reviewed and not folded.
