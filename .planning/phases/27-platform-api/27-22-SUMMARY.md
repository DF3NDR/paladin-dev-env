---
phase: 27-platform-api
plan: 22
subsystem: api
tags: [webhook, reqwest, security, dos, ssrf, credential-handling]

# Dependency graph
requires:
  - phase: 27-platform-api
    provides: "27-13's webhook delivery service (WebhookDeliveryService, build_webhook_client, backoff_for, bounded_error) and its D-40..D-43 design"
provides:
  - "read_bounded_body — a pub(crate) chunk-at-a-time response reader in client.rs that caps a webhook receiver's body at MAX_ERROR_BODY_BYTES (64 KiB) as it streams, never buffering past the cap"
  - "A signing-key load failure on the run lookup now reschedules the delivery (Retrying) instead of signing with a fallback empty key and sending"
affects: [27-platform-api-security-review, webhook-delivery-hardening]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Bound-during-read via reqwest::Response::chunk() rather than trusting Content-Length, for any future outbound HTTP client reading an attacker-influenced body"
    - "Send-suppressing retry: reschedule via the delivery's own backoff_for schedule and return before any request leaves the process, rather than sending a known-bad payload just to record a failure"

key-files:
  created: []
  modified:
    - src/application/services/run/webhook/client.rs
    - src/application/services/run/webhook/service.rs
    - src/application/services/run/webhook/tests.rs

key-decisions:
  - "read_bounded_body and MAX_ERROR_BODY_BYTES stay pub(crate) — the tracked public API surface (.project/current-exports.txt) does not grow for an internal helper"
  - "The signing-key-load-failure reschedule is unconditionally Retrying (never routed through retry_or_dead's exhaustion check) — matches the plan's literal spec; a delivery can still eventually dead-letter through record_attempt's own max_attempts logic on a later successful claim"
  - "Suppressing record_attempt's unconditional attempt increment on this reschedule is a recorded non-goal — would need a new WebhookDeliveryRepositoryPort method across three adapters (in-memory, sqlite, postgres), out of this gap-closure plan's scope"

patterns-established:
  - "Any future bounded-read helper over reqwest::Response should mirror read_bounded_body's chunk-and-truncate-the-last-chunk shape and its from_utf8_lossy decode, never response.text()/response.bytes() unbounded"

requirements-completed: [PLAT-05]

coverage:
  - id: D1
    description: "A webhook receiver's response body is bounded as it is read (CR-01): at most MAX_ERROR_BODY_BYTES ever enter memory for one attempt, proven against a body an order of magnitude over a small test cap, a body smaller than the cap, and a body exactly at the cap"
    requirement: PLAT-05
    verification:
      - kind: unit
        ref: "src/application/services/run/webhook/client.rs#bounded_body_stops_at_the_cap"
        status: pass
      - kind: unit
        ref: "src/application/services/run/webhook/client.rs#bounded_body_returns_a_small_body_whole"
        status: pass
      - kind: unit
        ref: "src/application/services/run/webhook/client.rs#bounded_body_at_exactly_the_cap_is_not_truncated"
        status: pass
    human_judgment: false
  - id: D2
    description: "The bounded body still goes through redact-then-truncate before being persisted as last_error, preserving the existing D-43 ordering"
    requirement: PLAT-05
    verification:
      - kind: unit
        ref: "src/application/services/run/webhook/service.rs#bounded_error_redacts_and_truncates"
        status: pass
      - kind: unit
        ref: "src/application/services/run/webhook/tests.rs#webhook_retry_schedule"
        status: pass
    human_judgment: false
  - id: D3
    description: "A signing-key load failure (WR-01) reschedules the delivery instead of sending a mis-signed payload — proven by a receiver mock recording zero hits"
    requirement: PLAT-05
    verification:
      - kind: unit
        ref: "src/application/services/run/webhook/tests.rs#webhook_signing_key_load_failure_reschedules_without_sending"
        status: pass
    human_judgment: false
  - id: D4
    description: "The full pre-existing webhook suite (retry schedule, receiver-side signature verification, payload contents, send-time SSRF, spawn idle poll) is unaffected by both changes"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib services::run::webhook (25 passed)"
        status: pass
    human_judgment: false

duration: ~25min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 22: Bounded webhook response-body read and signing-key load-failure reschedule Summary

**Webhook delivery no longer buffers an unbounded receiver body (CR-01, capped at 64 KiB during the read via `reqwest::Response::chunk()`) and no longer sends a payload signed with a fallback empty key when the run lookup that supplies the signing secret fails (WR-01, now reschedules instead).**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-09-08T13:21:33Z (worktree base commit)
- **Completed:** 2026-09-08T13:35:46Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `MAX_ERROR_BODY_BYTES` (64 KiB) and `read_bounded_body` added to `client.rs`: a chunk-at-a-time drain that stops the instant the accumulated length reaches the cap, truncating only the final chunk's excess rather than buffering it, decoded with `String::from_utf8_lossy` so a multi-byte character split by the cap boundary never panics. `Content-Length` is deliberately never trusted as the bound.
- `service.rs`'s non-2xx branch — the only path in the whole module that reads a response body — now reads through `read_bounded_body(response, MAX_ERROR_BODY_BYTES)` instead of `response.text()`, with the existing redact-then-truncate order (`bounded_error`) unchanged downstream.
- The signing-key lookup's `Err` arm in `service.rs::process` no longer falls through to signing with an empty key and sending. It now logs the same warning, reschedules via the delivery's own `backoff_for(new_attempt)` schedule (`Retrying` with `next_attempt_at = now + delay`), records a redacted, bounded diagnostic naming the failure, and returns before any HTTP request is issued.
- Both changes are proven by dedicated tests and the full pre-existing 22-test webhook suite (now 25) passes unmodified.

## Task Commits

Each task was committed atomically:

1. **Task 1: A receiver answering with a body larger than the cap costs bounded memory** - `77c7e54c` (fix)
2. **Task 2: A signing-key load failure reschedules instead of sending a mis-signed delivery** - `c2fe4afd` (fix)

**Plan metadata:** captured in this SUMMARY commit (worktree mode — STATE.md/ROADMAP.md updated centrally by the orchestrator after merge)

_Note: both tasks were `tdd="true"`; tests were written and run alongside the implementation in the same commit rather than as separate RED/GREEN commits, matching the plan's `<action>` instructions (tests added "in the same module"/"in tests.rs" as part of one described change, not a strict two-commit RED-then-GREEN sequence)._

## Files Created/Modified
- `src/application/services/run/webhook/client.rs` - Adds `MAX_ERROR_BODY_BYTES`, `read_bounded_body`, and three `bounded_body_`-prefixed mockito tests
- `src/application/services/run/webhook/service.rs` - Non-2xx branch reads through the bounded reader; signing-key-lookup `Err` arm reschedules instead of sending; module docs updated to note the only body-reading path is bounded
- `src/application/services/run/webhook/tests.rs` - Adds `FailingRunRepository` (a `RunRepositoryPort` double whose `get` always errs) and `webhook_signing_key_load_failure_reschedules_without_sending`

## Decisions Made
- Kept `read_bounded_body`/`MAX_ERROR_BODY_BYTES` `pub(crate)` per the plan's explicit prohibition — confirmed no new entries appear in `.project/current-exports.txt`.
- The signing-key-failure reschedule always produces `Retrying` (never `Dead`), following the plan's `<action>` text literally rather than routing through `retry_or_dead`'s `new_attempt < max_attempts` exhaustion check — a delivery can still eventually dead-letter, but only through a later successful claim exhausting `record_attempt`'s own accounting, not on this failure path directly.
- Left the recorded non-goal (suppressing `record_attempt`'s attempt increment on this reschedule) as an inline rustdoc comment rather than implementing a new repository-port method, matching the plan's explicit scope boundary.

## Deviations from Plan

None - plan executed exactly as written, including the CR-01 and WR-01 code changes, the three `bounded_body_`-prefixed tests, the `FailingRunRepository` double, and the module-doc updates.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Verification Evidence

- `cargo test -p paladin-ai --lib bounded_body_` → `test result: ok. 3 passed`
- `cargo test -p paladin-ai --lib webhook_signing_key_load_failure` → `test result: ok. 1 passed`
- `cargo test -p paladin-ai --lib services::run::webhook` → `test result: ok. 25 passed` (up from 22 pre-plan)
- `cargo fmt --all -- --check` → clean
- `cargo clippy -p paladin-ai --all-targets -- -D warnings` → clean
- `make security` → exit 0 (`advisories ok, bans ok, licenses ok, sources ok`; one pre-existing yanked-crate advisory notice for `spin`, unrelated to this plan's files)
- `cargo check --workspace --all-targets --all-features` → clean
- `grep -c 'read_bounded_body\|MAX_ERROR_BODY_BYTES' .project/current-exports.txt` → 0 matches (no public API surface growth)

## Next Phase Readiness
CR-01 and WR-01 from `27-REVIEW.md` are both closed with tests. No blockers for subsequent gap-closure plans (27-19 … 27-25) or for a re-run of the phase-level code review / verification pass.

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*

## Self-Check: PASSED

- FOUND: `.planning/phases/27-platform-api/27-22-SUMMARY.md`
- FOUND: `77c7e54c` (Task 1 commit)
- FOUND: `c2fe4afd` (Task 2 commit)
