---
phase: 27-platform-api
verified: 2026-09-08T14:30:00Z
status: passed
score: 6/6 must-have truth clusters verified (all ROADMAP success criteria + all six requirements PLAT-01..06)
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 12/17 must-have truth clusters (5 confirmed CI-evidenced gaps)
  gaps_closed:
    - "PLAT-FR-03 redelivery: Redis attempt-counter off-by-one on first dequeue (plan 27-19, claim-marker fix + 27-26 run_all fresh-queue fix)"
    - "PLAT-01 Postgres timestamp round-trip: nanosecond vs microsecond precision loss (plan 27-20, storage_timestamp truncation contract)"
    - "sdk-clients CI job failure: missing package-lock.json, real unauthenticated OpenAI provider, weak terminal-status assertion (plan 27-21, hermetic mock-llm smoke)"
    - "api-surface CI job failure: toolchain-dependent auto-trait bound ordering (plan 27-24, normalize-api-bounds.py canonicalization)"
    - "e2e_platform_api never run by any CI job (plan 27-24, new e2e-platform-api job)"
    - "CR-01 (code review): unbounded webhook response body read before truncation (plan 27-22, read_bounded_body)"
    - "WR-01 (code review): webhook signed with silently-empty key on run-lookup failure (plan 27-22, reschedule instead of send)"
    - "WR-04 (code review): LeaseHeartbeat busy-loop on zero-duration lease (plan 27-23, early-return guard)"
  gaps_remaining: []
  regressions:
    - "contract_tests::run_all shared one queue across its 8 clauses, causing ack_removes_message_permanently to observe leftover depth from earlier clauses once 27-19's fix made the later clauses reachable for the first time (discovered on intermediate CI run 34238527001, fixed by gap-closure plan 27-26, not present in the original 18 plans' behavior)"
deferred:
  - truth: "WR-02: Agent-kind runs never emit SSE events or webhook deliveries"
    addressed_in: "Accepted as a documented limitation for this phase, tracked as WINDOWS.md row 31 (open, not phase-27-blocking per plan 27-23's own decision)"
    evidence: "worker.rs field docs + run_agent doc comments, webhook/mod.rs WebhookPayload docs, docs/src/api-reference/platform-api.md 'Known limitations' subsection, and a pinning test agent_kind_run_with_a_webhook_enqueues_no_delivery"
  - truth: "WR-03: GET /runs, GET /runs/{id}, GET /runs/{id}/webhook-deliveries have no per-caller/tenant scoping"
    addressed_in: "Accepted for v0.10 as a single-tenant/mutually-trusted-principal deployment model, tracked as WINDOWS.md row 32 (open, not phase-27-blocking per plan 27-23's own decision)"
    evidence: "run_controller.rs 'Read scope' module-doc section and docs/src/api-reference/platform-api.md 'Authentication and scopes' section"
  - truth: "IN-01/IN-02 (code review info-level notes: Run::attempt serde default mismatch, fork-edit empty-key silent drop)"
    addressed_in: "Explicitly deferred by plan 27-23 as out of gap-closure scope (info-severity, no reachable production path)"
    evidence: "27-23-SUMMARY.md key-decisions: 'IN-01 ... deferred', 'IN-02 ... deferred'"
---

# Phase 27: Platform API Verification Report

**Phase Goal:** Runs execute durably in the background on a worker pool, integrate with Parley pauses and live streaming, and are managed through versioned assistants, cron schedules and webhooks — all reachable over a production-shaped HTTP API.
**Verified:** 2026-09-08T14:30:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (8 gap-closure plans: 27-19 … 27-26)

## Goal Achievement

This is a re-verification. The previous VERIFICATION.md (2026-09-08T12:10:00Z) recorded 5
CI-evidenced gaps and 2 human_verification items, all against live-CI run `34222317640`
(baseline, conclusion: failure). This re-verification checks the 8 gap-closure plans against both
(a) a fresh local sweep re-run in this session, and (b) `27-CI-EVIDENCE.md`'s live-CI proof at
evidence run `34245093476` / SHA `2bf43cd28829a5c957f5bd45b1abdd7c5c4212aa` — the binding Tier-2
evidence per D-51 (local self-skip is not evidence; a recorded CI run is).

### Observable Truths (ROADMAP success criteria)

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | `POST /runs` → 202, `RunRepositoryPort` persists every status transition, monotonic status machine with typed illegal-transition errors (PLAT-01) | ✓ VERIFIED | Status machine + CAS `UPDATE` code-verified (run.rs, run/sqlite.rs, run/postgres.rs). **Locally re-run this session:** `cargo test -p paladin-storage --features sqlite --lib run::` → 46 passed, 0 failed, including `storage_timestamp_truncates_sub_microsecond_digits_toward_zero`/`storage_timestamp_is_identity_at_microsecond_resolution`/`insert_then_get_round_trips_every_field`. **CI evidence (27-CI-EVIDENCE.md, job 102125436566):** Postgres suite `87 passed; 0 failed`, including the new `postgres_run_timestamps_round_trip_at_microsecond_precision` clause — the previous gap's exact failing assertions now pass. |
| 2 | Worker pool executes runs via `RunQueuePort` (InMemory/Redis) with lease heartbeats, at-least-once redelivery that resumes, cross-instance cancellation, `409 ThreadBusy` under 10 concurrent submits (PLAT-02) | ✓ VERIFIED | Heartbeat, resume-not-restart dispatch, `CancellationProbe` wiring, partial unique index all code-verified (unchanged from prior pass). **Locally re-run this session:** `cargo test -p paladin-storage --features redis-queue --lib claim_marker` → 3 passed (claim-marker guard tests pinning the fix without a live server). **CI evidence (job 102125436850):** Redis suite `19 passed; 0 failed`, log confirms `All run_queue::redis tests exercised the live server` — the previous gap's exact failing assertions (`attempt == 1` on first dequeue) now pass. `contract_tests::run_all`'s suite-isolation defect (discovered mid-verification loop on intermediate CI run 34238527001) is fixed by plan 27-26 (`fresh_queue()` factory, verified in code at `contract_tests.rs:449-463`) and reflected in the clean evidence run. |
| 3 | `AwaitingInput` releases the worker, `POST /threads/{id}/resume` re-enqueues under the same `run_id`, `GET /runs/{id}/stream` bridges live TraceSink events to SSE with degraded mode and 15s heartbeats (PLAT-03) | ✓ VERIFIED | Unchanged from prior pass (already fully verified then). **Locally re-run this session:** `cargo test --features web-server --test e2e_platform_api` → `test result: ok. 1 passed` in 12.19s. **CI evidence (job 102125436985, new `e2e-platform-api` job added by plan 27-24):** `test result: ok. 1 passed` — this is now continuously proven in CI, closing the prior "locally proven, not CI-wired" gap. |
| 4 | Assistants are append-only immutable versions (no PUT, ever), `latest` frozen at submit time, `WarGraphDoc` compiles through registry-resolving `compile()` with restart-stable fingerprint round-trip (PLAT-04) | ✓ VERIFIED | Unchanged from prior pass (already fully verified then — Postgres assistant suite was already green at baseline). **Locally re-run this session:** `cargo test -p paladin-web --lib no_put_or_patch_route_exists` → 1 passed; `cargo test -p paladin-battalion --lib fingerprint` → 35 passed including `fingerprint_golden_hex_v6`, `resume_with_checks_graph_fingerprint`, `replay_rejects_fingerprint_mismatch`. |
| 5 | Cron schedules survive restart without duplicate/missed-then-double firing; HMAC-signed webhook delivery with bounded retry and SSRF guard; every new endpoint carries auth/rate limiting/scopes/pagination; `openapi.json` regenerated; Python/TS clients generated and smoke-tested in CI (PLAT-05, PLAT-06) | ✓ VERIFIED | `claim_tick`, HMAC signing, SSRF guard, pagination clamps all code-verified (unchanged from prior pass). **New this session — CR-01/WR-01 hardening (plan 27-22):** `cargo test -p paladin-ai --lib bounded_body_` → 3 passed (`bounded_body_stops_at_the_cap`, `bounded_body_returns_a_small_body_whole`, `bounded_body_at_exactly_the_cap_is_not_truncated`) proving the webhook response-body read is now capped at 64 KiB before the attacker controls memory growth; `service.rs:225` confirmed the signing-key-load-failure path now reschedules (`Retrying`) instead of sending with an empty key. **New this session — sdk-clients (plan 27-21):** `python3 scripts/sdk-smoke/mock-llm.py --self-test` and `smoke.py --self-test` both pass locally; **CI evidence (job 102125436564):** both Python and TypeScript smokes report `terminal status 'completed' reached` — the previous gap (missing lockfile + real unauthenticated OpenAI provider + weak success assertion) is closed. **New this session — api-surface (plan 27-24):** `./scripts/check-api-surface.sh .project/current-exports.txt` re-run locally → `✅ API surface unchanged`; **CI evidence (job 102125436884):** same result on CI's own toolchain, closing the toolchain-drift gap. |

**Score:** 5/5 ROADMAP success criteria fully green. All five gaps from the previous VERIFICATION.md
are closed, each independently re-confirmed in this session (local re-run where runnable, CI
evidence cited otherwise) rather than trusted from SUMMARY.md claims alone.

### Deferred Items

Items explicitly accepted as documented, tracked limitations rather than closed — not phase-27-blocking per the gap-closure plans' own decisions (WINDOWS.md rows 31/32).

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | WR-02: `Agent`-kind runs bypass SSE bus and webhook delivery | Documented + pinned test, WINDOWS.md row 31 | `agent_kind_run_with_a_webhook_enqueues_no_delivery` test; `docs/src/api-reference/platform-api.md:350` "Known limitations" section (confirmed present at grep) |
| 2 | WR-03: Run/webhook-delivery read routes have no per-caller/tenant scoping | Documented, WINDOWS.md row 32 | `run_controller.rs` "Read scope" module doc; `platform-api.md` "Authentication and scopes" section |
| 3 | IN-01/IN-02: minor serde-default and fork-edit-key info notes | Explicitly deferred, no reachable production path | 27-23-SUMMARY.md key-decisions |

### Required Artifacts (representative sample, including all gap-closure artifacts)

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/paladin-storage/src/run_queue/redis.rs` | Corrected `RUN_QUEUE_CLAIM_LUA`/`RUN_QUEUE_NACK_LUA` with claim-marker gating | ✓ VERIFIED | 3 claim-marker guard tests pass locally; CI Redis suite 19/19 |
| `crates/paladin-storage/src/run_queue/contract_tests.rs` | `run_all` takes a fresh-queue-per-clause factory | ✓ VERIFIED | Code inspection confirms `fn run_all<F, Fut, Q>(fresh_queue: F)` calling `fresh_queue().await` before each of 8 clauses (lines 449-463) |
| `crates/paladin-storage/src/run/mod.rs` + `run/postgres.rs` | `storage_timestamp` microsecond-truncation contract applied at every bind site | ✓ VERIFIED | Unit tests pass locally; CI Postgres suite 87/87 including the new round-trip test |
| `scripts/sdk-smoke/{mock-llm.py,lib-boot.sh,smoke-http.sh,package-lock.json}` | Hermetic loopback LLM stub, committed lockfile, exact-`completed` assertion | ✓ VERIFIED | `mock-llm.py --self-test` and `smoke.py --self-test` pass locally; CI job green with both clients reaching `completed` |
| `src/application/services/run/webhook/client.rs` | `read_bounded_body` capping response reads at 64 KiB | ✓ VERIFIED | 3 bounded-body tests pass locally (`bounded_body_stops_at_the_cap`, etc.) |
| `src/application/services/run/webhook/service.rs` | Signing-key-load failure reschedules instead of sending | ✓ VERIFIED | Code inspection confirms `Retrying` outcome on `Err` branch (service.rs:225), no send |
| `src/application/services/run/worker.rs` | `LeaseHeartbeat::spawn` guarded against non-positive lease | ✓ VERIFIED | `lease_heartbeat_with_a_zero_lease_never_extends` passes locally |
| `scripts/normalize-api-bounds.py` + `.project/current-exports.txt` | Toolchain-order-independent API baseline | ✓ VERIFIED | `--self-test` passes; `check-api-surface.sh` reports unchanged locally and on CI |
| `.github/workflows/ci.yml` (`e2e-platform-api` job) | Runs `--features web-server --test e2e_platform_api` on every push/PR | ✓ VERIFIED | CI evidence job 102125436985 green, 1 passed |
| `.planning/phases/27-platform-api/27-CI-EVIDENCE.md` | Single evidentiary record tying all 5 gaps + 2 human_verification items to named CI jobs/runs/log lines | ✓ VERIFIED | Present, substantive, pre-filled-then-populated per its own anti-gaming design (T-27-25-03) |

### Key Link Verification (gap-closure deltas)

| From | To | Via | Status |
|---|---|---|---|
| `run_queue/redis.rs` claim/nack Lua | claim-marker JSON key | embedded literal in both scripts, read back by `#[cfg(test)]` introspection | ✓ WIRED |
| `run_queue/contract_tests.rs::run_all` | `in_memory.rs` / `redis.rs` callers | `fresh_queue()` factory closure per clause | ✓ WIRED |
| `run/postgres.rs` INSERT/UPDATE binds | `storage_timestamp()` | every `DateTime<Utc>` bind routed through it (INSERT_RUN, INSERT_RUN_WITH_LATEST, update_status) | ✓ WIRED |
| `webhook/service.rs` non-2xx handler | `client.rs::read_bounded_body` | replaces `response.text()` | ✓ WIRED |
| `.github/workflows/ci.yml` | `tests/integration/e2e_platform_api_test.rs` | new `e2e-platform-api` job, `--features web-server --test e2e_platform_api` | ✓ WIRED |
| `.github/workflows/ci.yml` (`api-surface` job) | `scripts/normalize-api-bounds.py` | `extract-public-api.sh` pipes through the normalizer before diffing | ✓ WIRED |

### Behavioral Spot-Checks (this session, all re-run fresh, not trusted from SUMMARY)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Redis claim-marker guard (Tier-1, no live server) | `cargo test -p paladin-storage --features redis-queue --lib claim_marker` | 3 passed; 0 failed | ✓ PASS |
| Postgres timestamp truncation + SQLite run suite | `cargo test -p paladin-storage --features sqlite --lib run::` | 46 passed; 0 failed | ✓ PASS |
| PRD 06 acceptance-1 full lifecycle, now CI-wired | `cargo test --features web-server --test e2e_platform_api` | 1 passed in 12.19s | ✓ PASS |
| Public API surface baseline | `./scripts/check-api-surface.sh .project/current-exports.txt` | `✅ API surface unchanged` (3763 items) | ✓ PASS |
| Webhook bounded-body read (CR-01) | `cargo test -p paladin-ai --lib bounded_body_` | 3 passed; 0 failed | ✓ PASS |
| LeaseHeartbeat zero-lease guard (WR-04) | `cargo test -p paladin-ai --lib lease_heartbeat_with_a_zero_lease` | 1 passed; 0 failed | ✓ PASS |
| sdk-smoke mock LLM self-test | `python3 scripts/sdk-smoke/mock-llm.py --self-test` | `self-test: ok` | ✓ PASS |
| sdk-smoke terminal-status decision self-test | `python3 scripts/sdk-smoke/smoke.py --self-test` | `self-test: ok (6/6 cases)` | ✓ PASS |
| api-bounds normalizer self-test | `python3 scripts/normalize-api-bounds.py --self-test` | `ok` | ✓ PASS |
| Formatting | `cargo fmt --all -- --check` | exit 0, no output | ✓ PASS |
| No-PUT assistant route (regression) | `cargo test -p paladin-web --lib no_put_or_patch_route_exists` | 1 passed | ✓ PASS |
| WarGraphDoc fingerprint suite (regression) | `cargo test -p paladin-battalion --lib fingerprint` | 35 passed | ✓ PASS |

### Tier-2 CI Evidence (not re-run in this session, per D-51 — cited from 27-CI-EVIDENCE.md's live run)

| Job | CI Run/SHA | Result | Proof cited |
|---|---|---|---|
| `Redis Run Queue Contract Suite (live server)` | `34245093476` @ `2bf43cd2` | PASS | `19 passed; 0 failed`; live-server log line present |
| `Postgres Storage Contract Suites (live server)` | same | PASS | `87 passed; 0 failed`, incl. new µs round-trip clause |
| `Coverage` | same | PASS | `Lines: 99781/110890 = 89.98%` (floor 82%), no exit 101 |
| `Generated SDK Clients (Python + TypeScript) smoke` | same | PASS | both clients report terminal status `completed` |
| `API Surface Tracking` | same | PASS | `API surface unchanged` |
| `e2e-platform-api` | same | PASS | `1 passed` |
| `Integration Tests` | same | PASS | 5382 passed, 0 failed (same Redis root cause, independently confirmed fixed) |
| `Unit Tests` / `MSRV (1.88)` / `Semver Checks` | same | PASS | regression guards, all green |

This verifier did not re-run CI (per task instructions — evidence is binding, not to be re-derived).
27-CI-EVIDENCE.md quotes exact log lines per job rather than conclusion-only, consistent with its
own `must_haves.prohibitions` (bar not lowered after seeing results) — this was checked by reading
the file, not assumed.

### Requirements Coverage

| Requirement | Description (abridged) | Status | Evidence |
|---|---|---|---|
| PLAT-01 | Run submission decoupled, RunRepositoryPort, monotonic status machine, SQLite+Postgres contract suite | ✓ SATISFIED | Postgres timestamp gap closed (plan 27-20), CI-evidenced 87/87; local SQLite/InMemory suite 46/46. **Traceability note:** REQUIREMENTS.md line 218 checkbox is still `[ ]` despite `27-20-SUMMARY.md` and `27-25-SUMMARY.md` both declaring `requirements-completed: [PLAT-01, ...]` — see Anti-Patterns below (doc-staleness, not a functional gap). |
| PLAT-02 | Durable worker pool, RunQueuePort (InMemory+Redis), lease heartbeats, redelivery, cross-instance cancel, 409 ThreadBusy | ✓ SATISFIED | Redis attempt-counter fixed (27-19) and run_all isolation fixed (27-26), CI-evidenced 19/19; REQUIREMENTS.md line 223 already `[x]` |
| PLAT-03 | Parley/streaming integration | ✓ SATISFIED | Verified in code; e2e_platform_api now CI-wired (27-24), CI-evidenced 1/1; REQUIREMENTS.md line 230 already `[x]` |
| PLAT-04 | Versioned assistants, WarGraphDoc | ✓ SATISFIED | No-PUT + fingerprint suite regression-checked locally; Postgres assistant suite was already green at baseline and remains so. **Traceability note:** REQUIREMENTS.md line 236 checkbox is still `[ ]` despite `27-25-SUMMARY.md` declaring `requirements-completed: [..., PLAT-04, ...]` — see Anti-Patterns below. |
| PLAT-05 | Schedules and webhooks | ✓ SATISFIED | claim_tick, SSRF, HMAC verified; CR-01/WR-01 hardening (27-22) code-confirmed; REQUIREMENTS.md line 243 already `[x]` |
| PLAT-06 | Production-shaped HTTP surface: auth, pagination, openapi, generated-client CI gate | ✓ SATISFIED | sdk-clients (27-21) and api-surface (27-24) gaps closed, CI-evidenced; REQUIREMENTS.md line 251 already `[x]` |

No orphaned requirements — PLAT-01…06 are the complete set declared across the 26 plans' `requirements:` frontmatter, matching REQUIREMENTS.md lines 216-255 and ROADMAP.md line 580 verbatim.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `.planning/REQUIREMENTS.md` | 218, 236, 382, 385 | PLAT-01 and PLAT-04 checkboxes/table rows still read `[ ]`/`Pending` | ⚠️ Warning (documentation-traceability gap, not a functional gap) | Both requirements are functionally satisfied and CI-evidenced (see Requirements Coverage above); `27-20-SUMMARY.md` (`requirements-completed: [PLAT-01]`) and `27-25-SUMMARY.md` (`requirements-completed: [PLAT-01, PLAT-02, PLAT-03, PLAT-04, PLAT-05, PLAT-06]`) both declared closure, but no commit in this phase's history ever flipped these two checkboxes — every sibling requirement (PLAT-02/03/05/06) was flipped by its closing plan's commit (e.g. `3242c3be docs(27-19): ... mark PLAT-02 requirement complete`, `d943fd25 docs(27-23): ... mark PLAT-03/05/06 complete`), but no equivalent commit exists for PLAT-01 or PLAT-04. Recommend a follow-up commit updating REQUIREMENTS.md lines 218, 236, 382, 385 before this milestone ships, so the traceability table matches the proven state. Not phase-27-blocking (the underlying capability is proven, independently, in this verification pass) but should not be carried silently into Phase 29's SHIP-03 program acceptance audit. |
| `MIGRATION.md` | 271, 538 | `TBD` markers | ℹ️ Info, not a blocker | Both carry an owning requirement/phase reference (`SHIP-02`/`SHIP-01`, Phase 29) per this phase's own documented structure — unchanged from prior verification pass. |

No other TBD/FIXME/XXX, empty-implementation, or hardcoded-empty-data patterns found in the gap-closure files (`crates/paladin-storage/src/run_queue/{redis,contract_tests}.rs`, `crates/paladin-storage/src/run/{mod,postgres,contract_tests}.rs`, `scripts/sdk-smoke/*`, `src/application/services/run/webhook/{client,service}.rs`, `src/application/services/run/worker.rs`, `scripts/normalize-api-bounds.py`, `.github/workflows/ci.yml`).

### Probe Execution

Not applicable — no `scripts/*/tests/probe-*.sh` convention in this project; verification relies on
named-test execution (this session, local) plus 27-CI-EVIDENCE.md's cited live-CI log lines
(Tier-2, not re-run per task instructions).

### Human Verification Required

None. Both human_verification items the previous VERIFICATION.md left open (re-run CI on the
gap-closing SHA; confirm Redis/Postgres/coverage jobs go green) are closed by 27-CI-EVIDENCE.md's
recorded, human-approved (`checkpoint:human-verify`, approved 2026-09-08) evidence run
`34245093476`. No new human-verification-only truths were introduced by the 8 gap-closure plans —
every gap-closure must-have was either a Tier-1 test this session re-ran directly, or a Tier-2
claim with a cited exact log line in 27-CI-EVIDENCE.md that this verification read and cross-
checked against the plan's own pre-committed proof requirements (T-27-25-03) rather than trusting
a conclusion-only pass.

### Gaps Summary

None remaining. All 5 gaps from the previous VERIFICATION.md (Redis attempt off-by-one, Postgres
timestamp precision, sdk-clients CI failure, api-surface toolchain drift, e2e_platform_api not
CI-wired) and both human_verification items are closed, each independently re-confirmed in this
session: 6 of the 8 gap-closure fixes were re-run locally against the actual test files (not
trusted from SUMMARY.md), and the 4 fixes needing live Redis/Postgres/coverage/sdk-generator
infrastructure are backed by 27-CI-EVIDENCE.md's exact-log-line evidence at a named run/SHA,
itself checked against its own pre-committed (before-the-run) proof table rather than accepted on
narrative. The code-review findings (CR-01, WR-01, WR-04) are also closed in code and locally
re-tested; WR-02 and WR-03 are accepted, documented, ledgered deviations (WINDOWS.md rows 31/32)
per the gap-closure plan's own explicit scope decision, not silent gaps.

One non-blocking documentation-traceability item is flagged above (Anti-Patterns): REQUIREMENTS.md
never flipped the PLAT-01/PLAT-04 checkboxes despite both being functionally proven and declared
closed in two plans' `requirements-completed` frontmatter. This does not affect the phase-goal
verdict (the underlying capability is independently proven above) but should be corrected before
Phase 29's program acceptance audit treats REQUIREMENTS.md as the source of truth.

---

_Verified: 2026-09-08T14:30:00Z_
_Verifier: Claude (gsd-verifier)_
