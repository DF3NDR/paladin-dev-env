---
phase: 57-register-hygiene-close-out
plan: 10
subsystem: review-disposition-register
tags: [reg-05, register, review-dispositions, owner-triage, gap-closure]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-05/57-06 fixes and WINDOWS rows 88-93; 57-07 hand-off (none); 57-08 checker and register shape; 57-09 rows for Phases 38, 44, 47, 51"
provides:
  - "19 register rows for Phases 41, 43 and 45; the register now covers all 51 findings (closed 36, waived 0, open 15)"
  - "appended '## Dispositions (Phase 57)' pointer sections in 41, 43 and 45 REVIEW.md (0 lines deleted)"
  - "the owner's triage (option-a) in 57-REVIEW-DISPOSITIONS.md '## Triage (Phase 57, 2026-11-04)': 7 fix-in-phase, 8 waive"
  - "'## Gaps for closure' below: seven gap entries that /gsd-plan-phase 57 --gaps consumes"
affects: [57-11, gap-closure plans]

tech-stack:
  added: []
  patterns:
    - "triage recorded as its own register section, outside '## Dispositions', so the checker never reads it as rows; rows stay open until 57-11"

key-files:
  created: []
  modified:
    - .planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md
    - .planning/milestones/v0.11.0-phases/41-admission-time-allowance-enforcement/41-REVIEW.md
    - .planning/milestones/v0.11.0-phases/43-rate-pacing/43-REVIEW.md
    - .planning/milestones/v0.11.0-phases/45-rustfs-swap-platform-observability-deviations/45-REVIEW.md

key-decisions:
  - "Owner chose option-a (D-14 as proposed): 7 correctness/security items fixed in this phase, 8 other-class items waived; no overrides, no exceptions"
  - "The waived register rows are NOT flipped to 'waived' here: the plan assigns minting and waiving to 57-11, so this plan records the outcome and the proposed D-03 owner/revisit/basis only"
  - "43 WR-03 and 43 WR-06 stay closed as recorded (documentation-only closures)"

requirements-completed: []

duration: ~1h across two sessions (tasks 1-2 earlier; triage continuation after the owner's reply)
completed: 2026-11-04
---

# Phase 57 Plan 10: Phases 41, 43, 45 dispositions and the owner triage Summary

**The register is complete at 51 findings (36 closed, 15 open and live), and the owner's option-a triage sends seven correctness/security items to gap closure before the 57-11 seal and waives eight other-class items.**

REG-05 is not ticked here: it spans 57-09, 57-10 and 57-11, and 57-11 ticks it.

## Performance

- **Tasks:** 3 (1 tracer, 1 auto, 1 checkpoint:decision)
- **Commits:** `09f6c5184` (Task 1), `abb91deb6` (Task 2), `74bc5300e` (Task 3, triage); this SUMMARY and STATE/ROADMAP in the metadata commit
- **Tracer gate:** verify re-run passed; orchestrator-approved (unattended session), not owner-approved.

## Per-finding evidence

Every row's pointer was checked in this phase (D-13). Full evidence text is in `57-REVIEW-DISPOSITIONS.md`.

| Phase | Id | Disposition | Evidence (short) |
|-------|----|-------------|------------------|
| 45 | CR-01 | closed | 57-06 `b5ba82365`; `cargo test -p paladin-storage --features s3 --lib minio::` (17 ok); WINDOWS row 90. RustFS contract case is Docker-gated: read from CI |
| 45 | WR-01 | closed | 57-06 `6d0de8905` (doc-only); WINDOWS row 92 |
| 45 | WR-02 | closed | 57-06 `6d0de8905`; `s3_err_summary` at 12 sites; WINDOWS row 91 |
| 45 | WR-03 | closed | 57-06 `ac7841931` (test-only, `--no-run` compiles); WINDOWS row 93; the test itself is Docker-gated |
| 45 | WR-04 | open, LIVE correctness | `worker.rs:1793`/`:1844`: terminal `RunFinished` emitted before `update_status`/`record_outcome`/`ack` and before `persist_failure` |
| 45 | WR-05 | closed | 57-05 `43d740c34`; two named tests (2 ok); WINDOWS row 88 |
| 45 | WR-06 | open, LIVE correctness | `worker.rs:1838-1839`: `TokenUsage::default()` and `cost: None` on the Err arm |
| 41 | WR-01 | open, LIVE correctness (judgment call) | `treasurer/mod.rs:251-299`, `:559`, `:568`: enqueue failure swallowed, notice not given back; dropped handler future loses the claim |
| 41 | WR-02 | open, LIVE other | `run_api_wiring.rs:1106`: boot refuses on a transient DNS failure for the optional webhook host |
| 41 | WR-03 | open, LIVE other | `config/treasurer.rs:865`: unparsable `APP_TREASURER_ALLOWANCE_WARN_AT` silently ignored (documented convention) |
| 41 | WR-04 | open, LIVE other | `treasurer/mod.rs:289`: store clock used for `next_attempt_at`, process clock for the claim |
| 41 | WR-05 | closed | 57-05 `d26c871d8`; empty-secret tests (3 ok); WINDOWS row 89 |
| 43 | WR-01 | open, LIVE correctness | `node_cache/redis.rs:306` vs `cadence/redis.rs:556`: marker TTL can outlive the fencing counter |
| 43 | WR-02 | open, LIVE other | `configuration.md:632`, `config/treasurer.rs:44`: `enabled: false` "restores previous behaviour" is wrong after the first-429 surfacing |
| 43 | WR-03 | closed | documented by `998ae11ee`; `default_deps_carry_an_in_process_cadence` (ok) |
| 43 | WR-04 | open, LIVE correctness | `cadence.rs:275`: 429 recorded inline, lost on caller cancellation |
| 43 | WR-05 | open, LIVE correctness | `cadence.rs:291-301`: zero delay becomes a zero-length gate |
| 43 | WR-06 | closed | ADR-0058:88-92, MIGRATION.md:629-636; `permanent_error_short_circuits_after_one_call` (ok) |
| 43 | WR-07 | open, LIVE other | `node_cache/redis.rs:160`: marker key `<key>:fence` collides with an entry key (unreachable from in-tree producers) |

Checker (register state before the triage commit, unchanged by it): `--pending-ok` over all 13 files exits 0 with `checked 51 finding(s) across 13 review file(s); 0 failure(s)`; strict reports 15 `FAIL OPEN` and nothing else. The three archive pointer sections add lines only (0 deleted).

## Live set

The 57-07 hand-off was "None", so the live set is the 15 open register rows. The owner decision column is the verbatim option-a outcome.

| Phase | Id | Class | Proposed outcome | Owner decision (option-a) |
|-------|----|-------|------------------|---------------------------|
| 51 | WR-01 | security | fix | fix-in-phase |
| 45 | WR-04 | correctness | fix | fix-in-phase |
| 45 | WR-06 | correctness | fix | fix-in-phase |
| 41 | WR-01 | correctness (judgment call) | fix | fix-in-phase |
| 43 | WR-01 | correctness | fix | fix-in-phase |
| 43 | WR-04 | correctness | fix | fix-in-phase |
| 43 | WR-05 | correctness | fix | fix-in-phase |
| 38 | WR-01 | other | waive | waive |
| 38 | WR-02 | other | waive | waive |
| 51 | WR-02 | other | waive | waive |
| 41 | WR-02 | other | waive | waive |
| 41 | WR-03 | other | waive | waive |
| 41 | WR-04 | other | waive | waive |
| 43 | WR-02 | other | waive | waive |
| 43 | WR-07 | other | waive | waive |

Counts: security 1, correctness 6, other 8.

## Decision

Verbatim, from the owner (am0rfu5@protonmail.com), relayed by the orchestrator, 2026-11-04:

> option-a

Selected: option-a, "Apply D-14 as proposed: every correctness/security/credential/tenant item is fixed in this phase (gap closure), every other item is waived". No row overrides and no exceptions were given; 43 WR-03 and 43 WR-06 stay closed as recorded. Recorded in `57-REVIEW-DISPOSITIONS.md` `## Triage (Phase 57, 2026-11-04)` (commit `74bc5300e`).

The eight waivers are minted and waived by 57-11 in the D-03 shape (`Re-homed: owner=..., revisit=..., basis=...`). The proposed revisit conditions and bases are in the triage table; the owner chose option-a without naming per-item values, so they come from each row's own evidence.

## Gaps for closure

Gap source for `/gsd-plan-phase 57 --gaps`. Each entry is a TDD fix (D-00a, D-00b, D-14): the regression test is written first and must fail on the head. Entries are in the VERIFICATION `gaps:` shape.

```yaml
gaps:
  - truth: "51 WR-01 (security): PaladinBuilder::effort_refusal never renders free vendor text from an LlmError into a ConfigurationError. Any non-UnsupportedParameter error from check_reasoning is reported with a fixed per-variant label only, and the doc comment at :1188-1189 is true."
    reason: "LIVE src/application/services/paladin/paladin_builder.rs:1202 (security): the `other` arm formats `{other}` into 'reasoning_effort for model ... could not be checked: {other}' (:1203). FallbackLlmAdapter::check_reasoning forwards any non-UnsupportedParameter error unchanged (crates/paladin-llm/src/fallback.rs:981). LlmError::ProcessingError(String) and the HTTP-status variant carry free vendor text, unredacted. Latent (no in-tree port returns such an error) but an out-of-tree port can; security.instructions.md requires redaction before text enters an error or log."
    artifacts:
      - src/application/services/paladin/paladin_builder.rs (effort_refusal, :1188-1209)
      - crates/paladin-llm/src/fallback.rs (check_reasoning forward, :981)
      - crates/paladin-llm/src/redaction.rs (redact-before-bound helper, if the label carries any text)
    missing: "Replace `{other}` with a fixed label per LlmError variant (variant kind only, no payload). Tests to add (red first), in the paladin_builder tests: (1) a stub LlmPort whose check_reasoning returns LlmError::ProcessingError containing a planted secret ('sk-...') and a second case with the HTTP-status variant; build() must return ConfigurationError whose text does not contain the secret or the vendor body; (2) the same through a FallbackLlmAdapter wrapping that stub; (3) the UnsupportedParameter arm still renders model, provider, value and reason unchanged. Run the manual credential-handling review for the arm."
  - truth: "45 WR-04 (correctness): run_agent publishes the terminal RunFinished only after the durable outcome is committed. On success: after update_status, record_outcome and ack succeed. On failure: after persist_failure has durably recorded the failed row, never on its repository-error nack branch. A failed write or nack leaves no terminal event for a run whose row is still running and will be redelivered."
    reason: "LIVE src/application/services/run/worker.rs:1793 (correctness): emit_agent_run_finished(Completed) at :1793 (defined :204) runs BEFORE update_status/record_outcome/ack (:1796-1814); the Err arm emits RunFinished Failed at :1844 BEFORE persist_failure (:1845), whose repository-error branch only warns and nacks (:2051-2059). Subscribers and the persisted trace see a terminal done/error while the row is running and the message is redelivered. The retry-seq collision is already closed by 42 WR-6 (trace_seq_origin, :1066). No later phase or test covers the window; the graph path documents its narrower window (:1507-1511), the agent path has no such note."
    artifacts:
      - src/application/services/run/worker.rs (run_agent success path :1793-1814, Err arm :1838-1845, persist_failure :2024-2059, emit_agent_run_finished :204-216)
      - src/application/services/run/worker_tests.rs
    missing: "Move the terminal emission after the durable write on both paths, keeping it inside the best-effort drain window and before unbind (T-45-11), and emitting exactly once. Tests to add (red first), in worker_tests.rs: (1) a repository whose update_status (and separately record_outcome, and ack) fails: the event bus has NO terminal RunFinished and the message is nacked; (2) a repository whose persist_failure write fails: no Failed RunFinished is emitted, nack happens; (3) the happy and persisted-failure paths still emit exactly one terminal event (agent_kind_run_emits_exactly_one_terminal_event stays green). Same file and arm as 45 WR-06: plan them in one plan or in serial waves."
  - truth: "45 WR-06 (correctness): a failed agent run reports the usage and cost it accumulated before failing to NodeFinished, RunFinished, the herald and the persisted trace, not zero."
    reason: "LIVE src/application/services/run/worker.rs:1838 (correctness): the Err arm emits NodeFinished with usage: TokenUsage::default() and cost: None (:1838-1839); RunFinished takes dispatcher.total_usage()/total_cost() (:210-216), which only fold NodeFinished events, so an agent loop that failed on a later iteration reports zero spend to the SSE error event, the herald and the trace. No fix, doc sentence or test pins this (the only usage assertions are on the success shape, worker_tests.rs:1268-1276); the review's own fallback (document and pin) was not taken."
    artifacts:
      - src/application/services/run/worker.rs (Err arm :1830-1845)
      - src/application/services/paladin/paladin_execution_service.rs (where the cost_tally and usage live; the error path must carry the partial figures)
      - src/application/services/run/worker_tests.rs
    missing: "Carry the partial usage and cost out of the failed agent loop and emit them on the Err arm's NodeFinished. Open design point for the gap plan: how the partial figures cross the error (a field on the error, or a returned tally); if that touches the public surface, run make api-surface-update and add a CHANGELOG entry (CLAUDE.md pre-commit rule), and raise it as an architectural question if it is more than additive. Tests to add (red first): an agent whose first LLM call succeeds with known usage and cost and whose second call fails: the NodeFinished and RunFinished usage equal the first call's usage and the cost is Some; a first-call failure still reports zero and None."
  - truth: "41 WR-01 (correctness): a won once-per-window operator allowance notice is never lost silently. If enqueueing the operator delivery fails (serialization, thread-id or enqueue), the notice row is given back so the next admission in the same window wins it again. If the handler future is dropped between claim and confirm/abandon (client disconnect), the notice is delivered or given back, not left claimed."
    reason: "LIVE src/application/services/treasurer/mod.rs:251 (correctness; judgment call, could be read as other, the owner's option-a fixes it): enqueue_operator_delivery returns () and logs and swallows a serialization, thread-id or enqueue failure (:251-299) with no discard of the notice row, so every later admission in the window reads AlreadyRecorded and emits nothing. confirm (:559) and abandon (:568) are awaited inline from submission.rs:405-406 with no drop guard or detached task, so a dropped future leaves a claimed notice neither delivered nor given back. The module docs accept only the crash case. No later phase or test covers either path."
    artifacts:
      - src/application/services/treasurer/mod.rs (enqueue_operator_delivery :251-299, confirm :559, abandon :568, notify_operator)
      - src/application/services/run/submission.rs (:405-406)
      - src/application/services/run/webhook (delivery enqueue)
    missing: "Make enqueue_operator_delivery report failure and discard (give back) the notice on failure, and make confirm/abandon survive cancellation (drop guard that gives back, or a detached task for confirm/abandon). Tests to add (red first): (1) an enqueue that fails: a second admission in the same window emits the notice again; (2) the handler future dropped after claim and before confirm: the notice is given back (next admission wins it) or delivered; (3) success path unchanged: one notice per window. Update the module docs that accept only the crash case."
  - truth: "43 WR-01 (correctness): the node-cache fence marker never outlives the cadence fencing counter that numbers it, so a valid put_fenced write after a counter expiry is never refused by a restarted token."
    reason: "LIVE crates/paladin-storage/src/node_cache/redis.rs:306 (correctness): the marker TTL is max(2 x entry ttl, 1 h) (:306, MIN_FENCE_TTL_MS :60) while the cadence fencing counter TTL is max(10 x lock ttl, 1 h) (crates/paladin-storage/src/cadence/redis.rs:556, test fence_ttl_is_ten_times_the_lock_and_at_least_an_hour). For a CachePolicy ttl above 30 minutes the counter can expire first, the next lock restarts at token 1, put_fenced sees 1 < N and silently drops the write, and the node re-runs uncached. Needs the Redis cache and cadence backends and a prior epoch that left a token above 1. ADR-0058 D-14 records the counter TTL, not this interaction. No Redis test covers it."
    artifacts:
      - crates/paladin-storage/src/node_cache/redis.rs (fence_ttl_ms computation :306, MIN_FENCE_TTL_MS :60)
      - crates/paladin-storage/src/cadence/redis.rs (fence_ttl_ms :556)
      - .planning/decisions/0058-rate-pacing-cadence.md (D-14 sentence on the interaction)
    missing: "Enforce the invariant marker TTL <= counter TTL (cap the marker TTL, or lengthen or refresh the counter TTL), by whichever side the gap plan judges smaller; keep it one source of truth. Tests to add (red first): a runnable pure-function test over the two TTL computations for entry ttls spanning below and above the 30-minute threshold asserting the counter outlives the marker; plus a Docker-gated Redis integration test with a prior epoch that left token > 1 and an entry ttl above the threshold, asserting the next fenced write is accepted. 57-11 reads the Docker-gated test from CI."
  - truth: "43 WR-04 (correctness): a 429 observed by a paced call is recorded even when the caller's future is dropped right after the response (outer timeout, run cancel, grace-deadline abort), so the fleet backs off."
    reason: "LIVE crates/paladin-llm/src/cadence.rs:275 (correctness): note_failure awaits record_rate_limited inline (called from generate at :407 and from the stream path) with no tokio::spawn and no synchronous in-process write first. A caller future dropped in the up-to-1-second Redis window never records the 429 and the fleet keeps sending into the limit. No test drops the future mid-record. Narrow window, reachable exactly where 429s cluster."
    artifacts:
      - crates/paladin-llm/src/cadence.rs (note_failure :273-282, generate call site :407, stream path)
      - crates/paladin-ports/src/output/cadence_port.rs
    missing: "Record the 429 so cancellation cannot lose it: write the in-process gate synchronously before the first await, and/or run record_rate_limited as a detached owned task. Tests to add (red first), covering generate and stream: a CadencePort test double whose record_rate_limited blocks on a gate; drop the call future after the 429 response; release the gate; assert the port recorded the 429 (and that the in-process gate is set before the drop)."
  - truth: "43 WR-05 (correctness): a zero provider delay (Retry-After: 0, a past HTTP-date, or an exhausted-dimension reset of 0) never produces a zero-length pacing gate. A zero minimum is treated as no provider delay, and the gate is at least the base/backoff delay with the streak incremented. A non-zero provider delay below base is still honoured."
    reason: "LIVE crates/paladin-llm/src/cadence.rs:291 (correctness): minimum_delay returns err.retry_after() unfiltered (:296-301), so Some(Duration::ZERO) reaches the port; delay_for returns the provided delay as is (crates/paladin-ports/src/output/cadence_port.rs:339-341, pinned by the unit test at :599 that even below base it is not raised), so the gate is zero-length with the streak incremented. effective_retry_after (crates/paladin-llm/src/rate_limit_hints.rs:248) can return a zero reset. A gateway sending Retry-After: 0 on every 429 disables pacing."
    artifacts:
      - crates/paladin-llm/src/cadence.rs (minimum_delay :288-301)
      - crates/paladin-ports/src/output/cadence_port.rs (delay_for :339-341, test :599)
      - crates/paladin-llm/src/rate_limit_hints.rs (effective_retry_after :248)
    missing: "Filter Some(Duration::ZERO) to None in minimum_delay (or floor in delay_for), leaving the :599 test's non-zero-below-base behaviour intact. Tests to add (red first): Retry-After: 0, a past HTTP-date, and a reset-0 header each yield a gate of at least the base delay with the streak incremented; a Retry-After below base but non-zero is still honoured as given; a zero delay on repeated 429s still escalates the streak."
```

## Next

Run `/gsd-plan-phase 57 --gaps` (gap source: this file's `## Gaps for closure`; the pass also adds each gap plan id to 57-11's `depends_on` and recomputes its wave), then `/gsd-execute-phase 57`. The gap plans run first and 57-11 seals last.

Planning notes for the gaps pass:

- 45 WR-04 and 45 WR-06 edit the same `run_agent` Err arm in `worker.rs`; plan them as one plan or serial waves.
- 41 WR-01, 43 WR-01, 43 WR-04 and 43 WR-05 touch disjoint files and can run in parallel with the others.
- Docker-gated tests 57-11 must read from CI before it closes anything: 43 WR-01's Redis test (added by its gap plan), 45 CR-01's RustFS contract case (WINDOWS row 90), 45 WR-03's presigned-URL test (WINDOWS row 93).
- 57-11 mints and waives the eight waive items in the D-03 shape from the triage table (owner and proposed revisit/basis are in `57-REVIEW-DISPOSITIONS.md` `## Triage`), and sets the seven fix rows to `closed` with each gap plan's fix commit, test and a WINDOWS row (D-14).

## Deviations from Plan

None - plan executed as written. The Task 3 checkpoint was answered with the prescribed `option-a` form.

## Known Stubs

None (documentation and planning artifacts only).

## Threat Flags

None. T-57-34 (a live security defect waived without a decision) is mitigated: the one security-class item (51 WR-01) is fix-in-phase by the owner's decision, and no correctness or security row is waived.

## Self-Check

- FOUND: commits `09f6c5184`, `abb91deb6`, `74bc5300e`
- FOUND: `## Triage (Phase 57, 2026-11-04)` in `57-REVIEW-DISPOSITIONS.md`
- `./scripts/check-review-dispositions.sh --pending-ok`: `checked 51 finding(s) across 13 review file(s); 0 failure(s)` after the triage commit

## Self-Check: PASSED
