---
phase: 27-platform-api
plan: 23
subsystem: api
tags: [tokio, run-worker, webhook, axum, mdbook, broken-windows-ledger]

requires:
  - phase: 27-platform-api
    provides: RunWorkerPool, LeaseHeartbeat, run_controller read routes, webhook delivery hook (plans 27-04, 27-12, 27-13)
provides:
  - "LeaseHeartbeat::spawn guarded against a non-positive lease (WR-04 closed in code)"
  - "The Agent-kind carve-out from webhook delivery and the live event bus, documented in three places and pinned by a test (WR-02 closed as a tracked decision)"
  - "The unscoped run-read model, published in the controller module docs and the API reference, and tracked in the ledger (WR-03 closed as a tracked decision)"
affects: [platform-api, webhook-delivery, run-worker]

tech-stack:
  added: []
  patterns:
    - "Option<JoinHandle<()>> guard pattern for a background-task constructor that must tolerate a degenerate input without spinning"
    - "Documented-and-pinned limitation: a carve-out is recorded in the code's own doc comments, mirrored in the published reference, and pinned by a named test that goes red the moment the behavior changes without the docs"

key-files:
  created: []
  modified:
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/run/webhook/mod.rs
    - crates/paladin-web/src/run_controller.rs
    - docs/src/api-reference/platform-api.md
    - .planning/WINDOWS.md

key-decisions:
  - "WR-04 (DoS): fixed in code. LeaseHeartbeat::spawn returns early on a non-positive lease, logging a warn and starting no task at all, rather than computing a zero interval that would spin. The private handle field became Option<JoinHandle<()>> with no change to the public spawn signature."
  - "WR-02 (repudiation, webhook silence on Agent-kind runs): accepted and documented, not fixed. Wiring the delivery hook and live bus into the legacy Runnable::Agent path is out of this plan's scope; the carve-out is now stated in three places (worker.rs field docs + run_agent, webhook/mod.rs's WebhookPayload docs, platform-api.md's new Known limitations subsection) and pinned by agent_kind_run_with_a_webhook_enqueues_no_delivery, and tracked as WINDOWS.md row 31."
  - "WR-03 (information disclosure, unscoped run/webhook-delivery reads): accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model, not fixed. Per-tenant scoping spans RunQuery, three repository adapters and the controller -- a phase-sized change, not gap closure. Documented in run_controller.rs's new Read scope module-doc section and in platform-api.md's Authentication and scopes section, and tracked as WINDOWS.md row 32."
  - "IN-01 (Run::attempt serde default of 0 vs. the constructor's 1) deferred: no persisted row omits the field, the repository INSERTs bind it explicitly, and the fix touches a core type and its existing test -- not gap-closure scope."
  - "IN-02 (a fork edit key that fails FieldName::new is dropped silently) deferred: the only reachable case is an empty-string key, and the function's own rustdoc already records the behavior."

patterns-established:
  - "A `### Known limitations` subsection under a feature's section in platform-api.md is the pattern for publishing an accepted-risk deferral in user-facing terms, matched to a code-level doc comment and a pinning test."

requirements-completed: [PLAT-02, PLAT-03, PLAT-05, PLAT-06]

coverage:
  - id: D1
    description: "LeaseHeartbeat::spawn starts no background task for a non-positive lease and logs a warn naming the misuse (WR-04)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#lease_heartbeat_with_a_zero_lease_never_extends"
        status: pass
    human_judgment: false
  - id: D2
    description: "A positive-lease heartbeat still extends at lease/4 exactly as before (D-10 regression guard)"
    requirement: "PLAT-02"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#heartbeat_extends_at_lease_over_four"
        status: pass
    human_judgment: false
  - id: D3
    description: "An Agent-kind run carrying a webhook spec completes normally and enqueues zero deliveries, proving the carve-out (WR-02)"
    requirement: "PLAT-05"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#agent_kind_run_with_a_webhook_enqueues_no_delivery"
        status: pass
    human_judgment: false
  - id: D4
    description: "The Agent-kind carve-out is stated in worker.rs's event_bus/webhook_deliveries field docs, run_agent's own doc, and webhook/mod.rs's WebhookPayload docs"
    requirement: "PLAT-05"
    verification:
      - kind: other
        ref: "grep -ci 'agent' src/application/services/run/webhook/mod.rs (>=1); rustdoc build (cargo doc -p paladin-web / cargo doc -p paladin-ai) clean of new broken-link warnings"
        status: pass
    human_judgment: false
  - id: D5
    description: "docs/src/api-reference/platform-api.md publishes the Agent-kind webhook/stream carve-out under a new Known limitations subsection"
    requirement: "PLAT-03"
    verification:
      - kind: other
        ref: "grep -c '### Known limitations' docs/src/api-reference/platform-api.md == 1; grep -ci 'code-registered' docs/src/api-reference/platform-api.md >= 1; mdbook build docs (exit 0, No broken links found)"
        status: pass
    human_judgment: false
  - id: D6
    description: "run_controller.rs's module docs state the unscoped read model (every run in the deployment, UUIDv7 enumeration) in a new Read scope section"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "grep -ci 'every run in the deployment' crates/paladin-web/src/run_controller.rs == 1; grep -ci 'uuidv7\\|time-ordered' crates/paladin-web/src/run_controller.rs >= 1"
        status: pass
    human_judgment: false
  - id: D7
    description: "docs/src/api-reference/platform-api.md's Authentication and scopes section states the unscoped read model concretely, replacing the general deferral sentence"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "grep -ci 'single-tenant' docs/src/api-reference/platform-api.md >= 1"
        status: pass
    human_judgment: false
  - id: D8
    description: "run_controller.rs's own unit suite is undisturbed by the docs-only change"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --lib --all-features run_controller (30 passed)"
        status: pass
    human_judgment: false
  - id: D9
    description: "Ledger rows 31 (WR-02) and 32 (WR-03) opened with named closing conditions, counters bumped consistently"
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: ".planning/WINDOWS.md rows 31/32 (markdown table + mirrored JSON block); open_count 21->23, total_count 30->32, verified against a per-status count of the JSON block (23 open + 4 waived + 5 fixed = 32)"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 23: WR-04 heartbeat guard + documented, tested WR-02/WR-03 carve-outs Summary

**A zero-duration `LeaseHeartbeat::spawn` now starts no task instead of spinning, and the Agent-kind webhook/live-bus carve-out plus the unscoped run-read model are both documented in code and the published reference, pinned by a test where a test applies, and tracked in the ledger.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-09-08T13:15:00Z (approx.)
- **Completed:** 2026-09-08T13:55:17Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- `LeaseHeartbeat::spawn` returns early on a non-positive lease (`Duration::ZERO`), logging a warn naming the misuse and starting no `tokio::spawn` task at all — the private `handle` field is now `Option<JoinHandle<()>>`, `Drop` aborts only when `Some`. The public `spawn` signature and the positive-lease `lease / 4` cadence are byte-identical to before.
- `RecordingQueue` extracted from an inline definition inside `heartbeat_extends_at_lease_over_four` to module scope in `worker_tests.rs`, now shared by that test and the new `lease_heartbeat_with_a_zero_lease_never_extends` (an exact-zero-calls assertion, not a bound).
- The `Agent`-kind carve-out from PLAT-FR-14 webhook delivery and the D-24 live event bus is now stated in `worker.rs`'s `event_bus`/`webhook_deliveries` field docs, on `run_agent` itself, and mirrored in `webhook/mod.rs`'s `WebhookPayload` docs — and pinned by `agent_kind_run_with_a_webhook_enqueues_no_delivery`, a new test built on two new doubles (`AgentOnlyResolver` resolving to `Runnable::Agent`, `AlwaysSucceedsPaladinPort`) that no prior test in this suite exercised.
- A new `### Known limitations` subsection under `## Webhooks` in `docs/src/api-reference/platform-api.md` states the same carve-out in user-facing terms: which assistant kind is affected (code-registered agents vs. stored `WarGraphDoc` workflows) and what a caller should do instead (poll the run).
- `run_controller.rs`'s module docs gained a `## Read scope (WR-03)` section stating plainly that the three run read routes are deployment-wide, not per-caller — including that the response carries another caller's webhook target URL and that `run_id` is a time-ordered UUIDv7. `docs/src/api-reference/platform-api.md`'s `## Authentication and scopes` section replaced its general "finer-grained per-resource scope model" deferral with a concrete statement of what a `GET` can see today.
- `.planning/WINDOWS.md` gained rows 31 (WR-02) and 32 (WR-03), each naming its closing condition; `open_count` 21→23, `total_count` 30→32, both the markdown table and its mirrored JSON block updated and validated (23 open + 4 waived + 5 fixed = 32, matches the frontmatter).

## Task Commits

Each task was committed atomically:

1. **Task 1: A zero-duration lease starts no heartbeat instead of spinning** - `845b7b8a` (fix)
2. **Task 2: The `Agent`-kind carve-out from webhook delivery and the live bus is documented and pinned by a test** - `caf74d31` (docs)
3. **Task 3: The unscoped read model is published, and both deferrals are tracked in the ledger** - `3f655471` (docs)

**Plan metadata:** commit pending (this SUMMARY + WINDOWS.md/REQUIREMENTS.md, per worktree convention — STATE.md/ROADMAP.md are excluded and owned by the orchestrator)

_Note: no TDD RED-then-GREEN split was needed — Task 1's test was written and the guard implemented together per the tracer task type's own verify step (test result checked directly against both the new and the pre-existing heartbeat test); Task 2's test was likewise authored alongside its doc changes since the plan's `tdd="true"` marker names a single behavior (zero deliveries) proven by one assertion, not a red/green pair._

## Files Created/Modified

- `src/application/services/run/worker.rs` - `LeaseHeartbeat`'s handle field became `Option<JoinHandle<()>>` with an early-return guard on a non-positive lease (WR-04); `event_bus`/`webhook_deliveries` field docs and `run_agent`'s own doc gained the WR-02 carve-out statement
- `src/application/services/run/worker_tests.rs` - `RecordingQueue` extracted to module scope; added `lease_heartbeat_with_a_zero_lease_never_extends`, `AgentOnlyResolver`, `AlwaysSucceedsPaladinPort`, and `agent_kind_run_with_a_webhook_enqueues_no_delivery`
- `src/application/services/run/webhook/mod.rs` - `WebhookPayload`'s docs gained one paragraph mirroring the WR-02 carve-out beside the unconditional PLAT-FR-14 rule
- `crates/paladin-web/src/run_controller.rs` - new `## Read scope (WR-03)` module-doc section stating the unscoped read model
- `docs/src/api-reference/platform-api.md` - new `### Known limitations` subsection under `## Webhooks` (WR-02); `## Authentication and scopes` deferral sentence replaced with a concrete statement (WR-03)
- `.planning/WINDOWS.md` - rows 31 (WR-02) and 32 (WR-03) appended to both the markdown table and the mirrored JSON block; frontmatter counters bumped

## Decisions Made

- WR-04 fixed in code (a DoS-shaped bug fix), WR-02 and WR-03 accepted and documented rather than fixed (an architectural/schedule-shaped scope decision each spans more than this gap-closure plan, per the plan's own threat register dispositions `T-27-23-02`/`T-27-23-03`).
- `RecordingQueue` shared at module scope rather than duplicated, so the zero-lease and positive-lease heartbeat tests can never drift on what "extend_lease was called" means.
- The `Agent`-kind resolver/PaladinPort test doubles (`AgentOnlyResolver`, `AlwaysSucceedsPaladinPort`) were added fresh rather than reused from an existing test, since no prior test in `worker_tests.rs` exercised the `Runnable::Agent` dispatch path at all.
- IN-01 and IN-02 (informational findings from `27-REVIEW.md`) are recorded as deferred dispositions in this SUMMARY per the plan's own instruction, rather than fixed or given a new ledger row — neither is reachable in production data and both are already documented in-code (IN-02) or structurally guarded (IN-01).

## Deviations from Plan

None - plan executed exactly as written. All four acceptance-criteria grep/test assertions across the three tasks passed on first verification; no Rule 1-4 auto-fixes were needed.

## Issues Encountered

`mdbook build docs` initially failed with "Unable to copy /workspace/.../docs/mermaid.min.js" — a missing generated asset (`mdbook-mermaid`'s installed JS/CSS files), not tracked in git and not present in this worktree's checkout. Ran `mdbook-mermaid install .` inside `docs/` to regenerate it (the files are gitignored, confirmed via `git status --short docs/` showing no new untracked entries after generation), then `mdbook build` succeeded with exit 0 and "No broken links found". This is a devcontainer/worktree bootstrap gap unrelated to this plan's content changes — the generated files are correctly gitignored and nothing about them needs committing.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- WR-04 is closed in code with a regression-proof test (exact-zero-calls assertion).
- WR-02 and WR-03 are closed as tracked decisions: both are visible in the code's own doc comments, in the published API reference, and as open ledger rows (31, 32) with named closing conditions — a future phase or `--gaps` pass has everything it needs to pick either up without re-discovering the root cause.
- No blockers for the remaining phase 27 gap-closure plans (24-25) or for phase close.

---

_Phase: 27-platform-api_
_Completed: 2026-09-08_
