---
phase: 42-mid-run-halt-sse-terminal-status
plan: 03
subsystem: storage
tags: [allowance, mid-run-halt, halt-reason, migration, run-repository, webhook, openapi]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: HaltReason, AllowanceRefusal::details_json, HaltCause::Spend and the cause-aware map_outcome (plan 42-02); ADR-0057 groups b-c (plan 42-01)
  - phase: 41-admission-time-allowance-enforcement
    provides: the 429 allowance_exhausted details object whose figures halt_reason now shares
provides:
  - migration 013 (runs.halt_reason, nullable, TEXT on SQLite and JSONB on PostgreSQL)
  - Run.halt_reason and RunOutcomeRecord.halt_reason persisted and read back by the in-memory, SQLite and PostgreSQL run adapters
  - contract clauses halt_reason_round_trips_on_record_outcome, legacy_row_reads_back_without_a_halt_reason, record_outcome_before_status_flip_is_accepted
  - worker ordering: record_outcome before update_status for Halted and Cancelled transitions (G14)
  - RunResponse.halt_reason and RunResponse.final_waypoint_id on GET /v1/runs/{id} and GET /v1/runs
  - WebhookPayload.halt_reason on the caller's halted webhook
  - ApiError::allowance_exhausted built from AllowanceRefusal::details_json (one builder for the 429 and the halt reason)
affects: [42-04, 42-05, 42-06, 42-08, 42-10, 42-12]

tech-stack:
  added: []
  patterns:
    - "One wire builder: the 429 details, GET /runs halt_reason and the webhook key all render through AllowanceRefusal::details_json and HaltReason::wire_json"
    - "Outcome before status for halting transitions, so no reader can observe halted without its reason"
    - "Additive nullable column plus serde-defaulted field: legacy rows read back None, schema version stays v1"

key-files:
  created:
    - crates/paladin-storage/migrations/sqlite/013_add_run_halt_reason.sql
    - crates/paladin-storage/migrations/postgres/013_add_run_halt_reason.sql
  modified:
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-ports/src/output/run_repository_port.rs
    - crates/paladin-storage/src/run/sqlite.rs
    - crates/paladin-storage/src/run/postgres.rs
    - crates/paladin-storage/src/run/in_memory.rs
    - crates/paladin-storage/src/run/contract_tests.rs
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - src/application/services/run/webhook/mod.rs
    - src/application/services/run/webhook/tests.rs
    - src/application/services/run/http_surface_tests.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/error.rs
    - crates/paladin-web/openapi.json
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "halt_reason is the LAST column of both run INSERT statements (SQLite and PostgreSQL), so every existing attribution and assistant-predicate placeholder number is unchanged; only the PostgreSQL trailing a.assistant_id placeholder moved from $20 to $21 and its pinning test was updated"
  - "record_outcome keeps overwriting all of its columns, halt_reason included (an outcome without a reason clears it), matching how error, output and final_waypoint_id already behave"
  - "The webhook key is attached only when the transition status is Halted, so even a mis-passed reason cannot leak onto another event; non-halted payload bytes are proven identical"
  - "Cancelled is ordered record-first together with Halted (the plan names both); the pool still records no reason for a cancel"

patterns-established:
  - "Run adapters write a typed JSON column through a small free serialiser (halt_reason_to_sql / halt_reason_to_json) bound as a parameter, never interpolated"
  - "A recording RunRepositoryPort double (OrderRecordingRepository in worker_tests.rs) proves write ordering and what the row held at each write"

requirements-completed: [ALLOW-03, PLAT-09]

coverage:
  - id: D1
    description: "A spend-halted engine run's GET /v1/runs/{id} and its row in GET /v1/runs carry status halted, error null, final_waypoint_id equal to the Halted Waypoint's id and halt_reason with exactly the keys balance, ceiling, kind, reason, scope, window_end, window_start (reason allowance_exhausted, scope api_key, kind window, balance and ceiling 1.0000 USD); the raw bodies contain no key value"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#engine_spend_halt_tracer (cargo test -p paladin-ai --lib --features web-server engine_spend_halt_tracer: 1 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The persisted reason round-trips on every run adapter, a ceiling of 1_000_000_001 nano-units reads back as 1_000_000_001, legacy rows and non-spend outcomes read back None, and the reason may be recorded before the status flips"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "contract clauses in crates/paladin-storage/src/run/contract_tests.rs run by the in-memory suite (34 passed) and the SQLite suite (74 passed with the feature, cargo test -p paladin-storage --features sqlite --lib run::)"
        status: pass
      - kind: unit
        ref: "PostgreSQL legs of the same clauses: compiled (cargo check -p paladin-storage --features postgres --all-targets, clippy -D warnings) but NOT executed; no PostgreSQL server in this sandbox so the store_or_skip gate self-skips them. CI postgres-integration is their only runner"
        status: skipped
    human_judgment: false
  - id: D3
    description: "A corrupt stored halt_reason (not JSON, or JSON that is not a known HaltReason) is a typed RunRepositoryError::Serialization on get and list, never a panic"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/sqlite.rs#a_corrupt_stored_halt_reason_is_a_serialization_error_not_a_panic"
        status: pass
    human_judgment: false
  - id: D4
    description: "Halted and Cancelled transitions write record_outcome before update_status (the row already holds the reason when the status flips); Completed keeps status-then-outcome; map_outcome records the spend reason with error None"
    requirement: "PLAT-09"
    verification:
      - kind: unit
        ref: "src/application/services/run/worker_tests.rs#halting_transition_records_the_outcome_before_the_status_flip and worker.rs#map_outcome_spend_halt_records_the_reason (cargo test -p paladin-ai --lib --features web-server application::services::run: 216 passed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The halted webhook payload carries halt_reason equal to wire_json; every other event's payload is byte-identical to before; the X-Paladin-Signature is the HMAC over the stored bytes with the new key present"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "worker.rs#halted_webhook_payload_carries_the_halt_reason_object, #non_halted_webhook_payload_never_carries_a_halt_reason_and_is_byte_identical; webhook/tests.rs#halted_webhook_signature_covers_the_halt_reason; webhook/mod.rs#webhook_payload_includes_halt_reason_only_when_present"
        status: pass
    human_judgment: false
  - id: D6
    description: "The 429 details object and halt_reason are built by one function, and the 429 body is byte-identical to before"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/error.rs#allowance_exhausted_details_equal_details_json plus the three pre-existing allowance_exhausted tests, unchanged (cargo test -p paladin-web --lib: 282 passed)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The new surface is registered: four MIGRATION.md 9.2 rows, the 9.4 migration paragraph, the 9.6 read-field paragraph, platform-api.md, the CHANGELOG entry, the regenerated openapi.json and the refreshed public-API baseline"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (exit 0); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface (exit 0, unchanged after api-surface-update); cargo test -p paladin-web --lib openapi_matches_committed_baseline and --test openapi_golden_v0_9 (8 passed)"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 03: Persisted halt reason, served with the fork point Summary

**A spend-halted run now carries its typed reason durably on the run row (migration 013, three adapters), the worker writes it before the status flips, and `GET /v1/runs/{id}`, `GET /v1/runs` and the caller's `halted` webhook all serve it from the same builder the Phase 41 `429` uses, beside `final_waypoint_id` as the fork point.**

## Precondition check (Task 1)

The 42-01 SUMMARY has a `## Checkpoint decision` heading whose recorded selection is `option-b` (all 16 design items approved as written, item 12 extended). It names no redirect on item 3 (the `013` migration shape), so the one-way migration proceeded without a plan revision.

## Performance

- **Duration:** ~40 min (18:13Z to 18:29Z for the three task commits, plus pre-flight reading, closeout checks and this summary)
- **Started:** 2026-10-06T18:10Z
- **Completed:** 2026-10-06T18:29Z (code); summary and state updates follow
- **Tasks:** 3 (all auto, Tasks 1-2 TDD)
- **Files modified:** 20 (2 created)

## Accomplishments

- **Storage half (Task 1).** Migration pair `013_add_run_halt_reason.sql` adds one nullable `runs.halt_reason` (`TEXT NULL` on SQLite, `JSONB NULL` on PostgreSQL, the same types as `runs.output`); no other migration file changed. `Run.halt_reason: Option<HaltReason>` (`#[serde(default, skip_serializing_if = ...)]`, `RUN_SCHEMA_VERSION` stays `v1`) and `RunOutcomeRecord.halt_reason` (with a compiling doc example) are persisted and read back by the in-memory, SQLite and PostgreSQL adapters. The reason is bound as serialized JSON through a small free serialiser, never interpolated; a corrupt stored value is a typed `Serialization` error on `get` and `list`. Three new contract clauses run on all three adapters; SQLite also gained a column-shape test, an SQL-constant test, an insert round-trip and the corrupt-value test.
- **Ordering and carriage (Task 2).** `map_outcome` records `HaltCause::Spend(reason)` into the outcome; the worker writes `record_outcome` before `update_status` for `Halted` and `Cancelled` and keeps today's order for every other transition (G14). The stale D-14 comment claiming the SSE `done` collapses `cancelled` into `halted` is replaced with a note that 42-05 and 42-06 own the SSE status. `WebhookPayload` gained an optional `halt_reason` key present only on a `halted` event.
- **Served from one builder.** `RunResponse` gained always-serialised `halt_reason` and `final_waypoint_id`; `ApiError::allowance_exhausted` now calls `AllowanceRefusal::details_json`, so the `429` object and the halt reason cannot drift. `crates/paladin-web/openapi.json` was regenerated with `make openapi` (two nullable properties, nothing removed; the v0.9 golden is untouched).
- **Tracer extended.** `engine_spend_halt_tracer` now asserts the exact sorted key set, `reason`/`scope`/`kind`/`balance`/`ceiling` values, `final_waypoint_id == latest Halted Waypoint id`, an identical `halt_reason` and `final_waypoint_id` on the `GET /v1/runs?thread_id=` row, and that neither body contains the key value.
- **Registered (Task 3).** Four `MIGRATION.md` 9.2 rows, the 9.4 migration paragraph, the 9.6 paragraph, `platform-api.md` (a "Halted runs" section and the webhook key), the CHANGELOG bullet extended, and the public-API baseline refreshed (one added line: `WebhookPayload::halt_reason`).

## Task Commits

1. **Task 1: persist the typed halt reason on the run row (migration 013)** - `38079c33` (feat)
2. **Task 2: serve and carry a halted run's reason and fork point** - `15274928` (feat)
3. **Task 3: register and document the persisted halt reason** - `c69d3a1a` (docs)

## Files Created/Modified

See the frontmatter `key-files` list. The two new files are the SQLite and PostgreSQL `013_add_run_halt_reason.sql` migrations.

## Verification

- Task 1: `cargo test -p paladin-storage --lib run::in_memory` 34 passed; `cargo test -p paladin-storage --features sqlite --lib run::` 74 passed (0 failed); `cargo check -p paladin-storage --features postgres --all-targets` ok; `cargo clippy -p paladin-storage --features sqlite,postgres --all-targets -- -D warnings` clean; `cargo test -p paladin-ai-core --lib run` 62 passed; `cargo test -p paladin-ports --doc RunOutcomeRecord` 1 passed; `cargo check --workspace --all-targets --all-features` ok; `git diff --name-only HEAD~1 HEAD -- crates/paladin-storage/migrations` lists only the two `013` files.
- Task 2: `cargo test -p paladin-ai --lib --features web-server application::services::run` 216 passed (worker, webhook, tracer included); `cargo test -p paladin-web --lib` 282 passed; `cargo test -p paladin-web --test openapi_golden_v0_9` 8 passed; `engine_spend_halt_tracer` 1 passed; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --check` clean; `grep 'deliberate, documented' worker.rs` prints nothing.
- Task 3: `./scripts/check-migration-allowlist.sh` exit 0; the 9.4 section names `013_add_run_halt_reason` and the 9.6 section names `final_waypoint_id`; `platform-api.md` contains `halt_reason` (9) and `final_waypoint_id` (3); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exit 0.
- Whole workspace, `cargo test --workspace --all-features --no-fail-fast`: 7,505 passed, 18 failed, all outside this plan (see Issues Encountered).
- **PostgreSQL legs not run.** PostgreSQL is not running in this sandbox, so the PostgreSQL run-adapter contract legs (including the three new clauses) compile and lint locally but self-skip at their `store_or_skip` gate; they are claimed green only by CI's `postgres-integration` job, not by this plan's local run. The Postgres placeholder renumbering (`$21` for the trailing `a.assistant_id` predicate, `$20::jsonb` and `$21::jsonb` for `halt_reason`) is pinned only by DB-free SQL-text tests and by that CI job.
- Manual credential-handling review: `halt_reason` is `HaltReason::wire_json()` of the halted ceiling's own refusal; it carries no tenant id, key name or key value, no log line interpolates one, the webhook key adds no signing value, and the tracer asserts neither the run body nor the list body contains the key value. Webhook signing is unchanged: the payload is still serialised once and signed from that buffer, proven for a payload carrying the new key by `halted_webhook_signature_covers_the_halt_reason`.

## Decisions Made

- The new column is the last `INSERT` column on both backends so existing placeholder numbering and the SQLite "ends with the `a.assistant_id` bind" invariant hold; the one PostgreSQL number that had to move (`$20` to `$21`) is covered by an updated pinning test.
- The webhook key is gated on `status == Halted` inside `webhook_delivery_for_outcome`, in addition to the caller passing the reason only for halted transitions.
- Left `record_outcome`'s overwrite semantics as they were: an outcome with no reason clears `halt_reason`, like the other outcome columns.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] RunOutcomeRecord literals in the run worker added to Task 1**
- **Found during:** Task 1 (`cargo check --workspace --all-targets --all-features`)
- **Issue:** The new `RunOutcomeRecord.halt_reason` field broke 15 struct literals in `src/application/services/run/worker.rs` (production `map_outcome` arms and tests), which Task 1's verify command requires to compile; the plan lists `worker.rs` only under Task 2.
- **Fix:** Added `halt_reason: None` to each literal in the Task 1 commit; Task 2 then set the real value for the spend arm.
- **Files modified:** `src/application/services/run/worker.rs`
- **Commit:** `38079c33`

**2. [Rule 3 - Blocking] Disk full during Task 2 verification**
- **Found during:** Task 2 (`cargo test`)
- **Issue:** `No space left on device` while linking `paladin-web`; the volume had 23 MB free with 8 GB of `target/debug/incremental`.
- **Fix:** Deleted `target/debug/incremental` (rebuildable build cache, untracked) and ran the remaining cargo commands with `CARGO_INCREMENTAL=0`.
- **Files modified:** none tracked

**3. [Plan file list] `worker_tests.rs` modified for the ordering test**
- The G14 ordering test needs the worker-level harness (`RunWorkerPool`, in-memory queue) that lives in `src/application/services/run/worker_tests.rs`, which Task 2's `files` list omits. The test (`halting_transition_records_the_outcome_before_the_status_flip`, with an `OrderRecordingRepository` double) was added there rather than duplicating the harness.

**Total deviations:** 3, none changing the plan's design or any decision. `webhook/tests.rs` was likewise touched for the signature test and to add `halt_reason: None` to existing literals.

## Auth Gates

None.

## Issues Encountered

- **Pre-existing, outside this plan (not fixed):** two `run_api_wiring` tests (`build_run_api_persists_no_run_traces_by_default`, `build_run_api_persists_run_traces_when_trace_persist_is_set`) fail in this sandbox because they need outbound network, already logged in `deferred-items.md` from 42-02. Sixteen `integration::redis_queue_integration_test` tests also fail because no Redis server is running here (they need `make services-up`); they exercise the Redis queue only and none touches the run repository or the code changed here. CI, which has both services, is their authority.
- The PostgreSQL contract legs could not run locally (see Verification).

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or file-access pattern was introduced; the new column, DTO field and webhook key sit inside the plan's registered trust boundaries (T-42-09 through T-42-13), each mitigated as listed in the plan's threat model.

## Self-Check: PASSED

- Created files exist: both `013_add_run_halt_reason.sql` files and this SUMMARY (checked below).
- Commits `38079c33`, `15274928` and `c69d3a1a` are on `claude/laughing-dirac-e0h2ax`.
