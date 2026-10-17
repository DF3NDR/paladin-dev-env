---
phase: 42-mid-run-halt-sse-terminal-status
plan: 10
subsystem: treasurer-operator-notice-store-and-webhook
tags: [allowance, halt-notice, migration-014, treasury-notices, notice-kind, operator-webhook, allowance-halted, run-event-kind]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: ADR-0057 group (g) and the option-b gate decision with no redirect on items 3 and 13 (42-01); migration 013 and the three-adapter contract-clause precedent (42-03); the priced Treasurer and the notice claim ordering (42-07, 42-09)
  - phase: 41-admission-time-allowance-enforcement
    provides: treasury_notices (migration 011), TreasuryNoticePort, the operator webhook and its operator-secret signing branch, RunEventKind with as_str as the single source
provides:
  - migrations 014 (SQLite and PostgreSQL) adding treasury_notices.notice_kind and rebuilding idx_treasury_notices_once with notice_kind appended
  - NoticeKind (Warning, Halt), NoticeRecord.kind and AllowanceNotice.kind in paladin-ai-core
  - a store identity that keeps a halt notice distinct from a warning notice for the same scope, limit, window and ceiling, on the in-memory, SQLite and PostgreSQL adapters, with notices_for_run returning warning rows only
  - RunEventKind::AllowanceHalted read back by both webhook adapters and signed with the operator notice secret before and instead of any run lookup
  - AllowanceWarningPayload::from_notice and the Treasurer delivery row mapping the notice kind to its event through one crate-private function
  - MIGRATION 9.2 rows and a 9.4 paragraph, a CHANGELOG paragraph, a regenerated public-API baseline
affects: [42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "A new dedup dimension is appended last to the unique index and to every adapter ON CONFLICT arbiter list, and the arbiter-matches-the-migration tests are pointed at the newest migration that defines the index"
    - "A one-way schema door is confirmed from the recorded gate decision before the migration file is written, and the confirmation is recorded in the SUMMARY"
    - "One crate-private notice-kind-to-event mapping shared by the wire payload and the delivery row, so the two can never disagree"

key-files:
  created:
    - crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql
    - crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql
  modified:
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-core/src/platform/container/run.rs
    - crates/paladin-ports/src/output/treasury_notice_port.rs
    - crates/paladin-storage/src/treasury/mod.rs
    - crates/paladin-storage/src/treasury/in_memory.rs
    - crates/paladin-storage/src/treasury/sqlite.rs
    - crates/paladin-storage/src/treasury/postgres.rs
    - crates/paladin-storage/src/treasury/notice_contract_tests.rs
    - crates/paladin-storage/src/webhook/in_memory.rs
    - crates/paladin-storage/src/webhook/sqlite.rs
    - crates/paladin-storage/src/webhook/postgres.rs
    - crates/paladin-storage/src/webhook/contract_tests.rs
    - src/application/services/run/webhook/mod.rs
    - src/application/services/run/webhook/service.rs
    - src/application/services/run/webhook/tests.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/worker_tests.rs
    - src/infrastructure/web/run_api_wiring.rs
    - crates/paladin-web/src/agent_controller.rs
    - crates/paladin-web/src/run_controller.rs
    - crates/paladin-web/src/schedule_controller.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The notice kind is the LAST column of the rebuilt unique index and of every ON CONFLICT arbiter list, so the arbiter text is the old list plus one column and the notice_arbiter_matches_the_migration tests read the 014 file, which now owns the index definition"
  - "SQLite accepted ADD COLUMN with NOT NULL DEFAULT and an inline CHECK (assumption A3 held), so no table-rebuild fallback was written; the old 011 file is byte-untouched"
  - "The notice-kind-to-event mapping is crate-private (operator_event_for in src/application/services/run/webhook/mod.rs), so the public surface of the paladin facade is unchanged"
  - "A pre-014 replica against a migrated database cannot infer the rebuilt index from its own arbiter list: its notice claim returns a logged store error that is skipped (a notice observes, never gates); recorded as a rollout caveat in MIGRATION 9.4 rather than the plan's draft claim that it would default to a warning"
  - "RunEventKind::AllowanceHalted needed no caller-parser change: both parse_event_kind functions already reject it through the other arm, and tests now pin that"

patterns-established:
  - "Contract clause helpers (halt_of) build the sibling-kind notice for the same identity so every adapter exercises the same dedup matrix"

requirements-completed: [ALLOW-03]

coverage:
  - id: D1
    description: "A warning notice and a halt notice for one scope, limit, window and ceiling are both recorded; a second halt notice for that identity is AlreadyRecorded; raising the ceiling re-arms both kinds"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/treasury/notice_contract_tests.rs#warning_and_halt_notices_for_one_identity_are_both_recorded (run on the in-memory and SQLite adapters; the PostgreSQL leg self-skips here; red on the in-memory adapter before the kind joined its dedup key)"
        status: pass
    human_judgment: false
  - id: D2
    description: "notices_for_run returns warning rows only: a run with one warning and two halt rows reads back just the warning, so the worker first-dispatch replay never emits a halt row as a warning"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/treasury/notice_contract_tests.rs#notices_for_run_returns_warning_rows_only (in-memory and SQLite; red before the warning filter)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Edge (legacy data): a row written by a pre-014 statement with no notice_kind reads back as a warning notice, the table CHECK rejects an unknown kind, a halt notice is stored as halt, and the rebuilt index ends with notice_kind"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/treasury/sqlite.rs#tests::a_row_written_without_notice_kind_reads_back_as_a_warning, #a_halt_notice_is_stored_as_halt_and_the_check_rejects_other_kinds, #migration_014_rebuilds_the_once_per_window_index_with_the_kind"
        status: pass
    human_judgment: false
  - id: D4
    description: "Each adapter ON CONFLICT arbiter list matches the rebuilt index in the 014 migration, and the PostgreSQL migration text, its warning-only read and its insert are pinned by DB-free tests because no PostgreSQL server runs here"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/treasury/sqlite.rs#tests::notice_arbiter_matches_the_migration; crates/paladin-storage/src/treasury/postgres.rs#tests::notice_arbiter_matches_the_migration and #notice_kind_column_and_warning_only_read_match_the_migration"
        status: pass
    human_judgment: false
  - id: D5
    description: "An allowance_halted operator delivery is stored and read back by every webhook adapter, and the delivery service signs it with the operator notice secret, sends it verbatim with X-Paladin-Event allowance_halted, and never queries the run repository"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/webhook/contract_tests.rs#allowance_halted_operator_row_round_trips (in-memory and SQLite); src/application/services/run/webhook/tests.rs#operator_halted_delivery_is_signed_with_the_operator_secret_without_a_run_lookup"
        status: pass
    human_judgment: false
  - id: D6
    description: "A halt notice payload carries event allowance_halted and exactly the same twelve keys as a warning payload, every other field equal for the same notice; the kind-to-event mapping is pinned"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/webhook/mod.rs#allowance_warning_payload_tests::allowance_halted_payload_has_the_same_twelve_keys and #operator_event_for_maps_each_notice_kind_to_its_event"
        status: pass
    human_judgment: false
  - id: D7
    description: "Callers cannot subscribe a run or schedule webhook to allowance_halted: both caller-facing parsers answer 400 naming the kind"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "crates/paladin-web/src/run_controller.rs#tests::caller_cannot_subscribe_a_run_webhook_to_allowance_halted; crates/paladin-web/src/schedule_controller.rs#tests::caller_cannot_subscribe_a_schedule_webhook_to_allowance_halted"
        status: pass
    human_judgment: false
  - id: D8
    description: "The Phase 41 warn path is unchanged: nothing claims a halt notice or enqueues an allowance_halted delivery in this plan, and allowance_warn_path_tracer passes"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "src/application/services/run/tracer_e2e.rs#allowance_warn_path_tracer (1 passed); the Treasurer claim_notice still builds NoticeKind::Warning"
        status: pass
    human_judgment: false
  - id: D9
    description: "The halt rung is registered: MIGRATION 9.2 rows and the 9.4 paragraph, the CHANGELOG paragraph, the allowlist set-equality check, and the public-API baseline (surface unchanged)"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh exit 0; PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface reports the surface unchanged (4209 items)"
        status: pass
    human_judgment: false
  - id: D10
    description: "Edge (concurrency, PostgreSQL): the PostgreSQL legs of the notice and webhook contract clauses, and the sixteen-way claim race against a real PostgreSQL index, run in CI only because no PostgreSQL server exists in this sandbox"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "CI postgres job (crates/paladin-storage/src/treasury/postgres.rs and crates/paladin-storage/src/webhook/postgres.rs contract registrations); not run here"
        status: pass
    human_judgment: true

duration: ~25 min
completed: 2026-10-07
status: complete
---

# Phase 42 Plan 10: A halt rung for the operator notice store and webhook Summary

**The once-per-window notice store now keeps a halt notice distinct from a warning notice (migration 014 adds `treasury_notices.notice_kind` and rebuilds the dedup index with the kind appended, on all three adapters), and the durable operator webhook carries a signed `allowance_halted` event with the warning's exact twelve-key payload; nothing claims or sends a halt notice until 42-11 wires the boundary guard.**

## Gate precondition (migration 014 is a one-way door)

Confirmed before either migration file was written, from the `## Checkpoint decision` section of `.planning/phases/42-mid-run-halt-sse-terminal-status/42-01-SUMMARY.md`: the recorded selection is **`option-b`** (all 16 design items approved as written, item 12 extended for the streamed `done`). It names **no redirect** on item 3 (migrations 013 and 014 split, with 014 adding `treasury_notices.notice_kind` and rebuilding `idx_treasury_notices_once`) or on item 13 (the operator-notice design, `notices_for_run` warning rows only). The task precondition therefore held and no second checkpoint was needed.

## Performance

- **Duration:** about 25 minutes
- **Completed:** 2026-10-07
- **Tasks:** 3
- **Files modified:** 25 (2 created)

## Accomplishments

- **Migration 014 on both backends.** `crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql` and `crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql` add `notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning', 'halt'))`, drop `idx_treasury_notices_once` and recreate it with `notice_kind` appended. Every pre-existing row reads back as a warning. `011` and every other earlier migration are byte-untouched.
- **`NoticeKind` and the identity.** `NoticeKind { Warning, Halt }` (`#[non_exhaustive]`, default `Warning`, serde `snake_case`, `as_str()`, doctest) in `crates/paladin-core/src/platform/container/allowance.rs`; `NoticeRecord.kind` and `AllowanceNotice.kind` carry it with `#[serde(default)]`, so a serialized notice without the key is a warning.
- **Three adapters, one contract.** The in-memory identity tuple, and the SQLite and PostgreSQL `NOTICE_INSERT` column list, bound value and `ON CONFLICT (...)` arbiter list, all end with the kind. `NOTICES_FOR_RUN` adds `notice_kind = 'warning'` on both SQL adapters and the in-memory filter does the same. The shared row decoder in `crates/paladin-storage/src/treasury/mod.rs` reads the kind back and turns unknown text into the adapter serialization error.
- **The operator event.** `RunEventKind::AllowanceHalted` (`allowance_halted`) is read by both webhook adapters' `event_from_str`, and the operator-signing branch in `src/application/services/run/webhook/service.rs` now matches `RunEventKind::AllowanceWarning | RunEventKind::AllowanceHalted`, signing with the operator secret before and instead of any run lookup. No second HTTP client or signing path was added.
- **One mapping for payload and row.** `AllowanceWarningPayload::from_notice` and `Treasurer::enqueue_operator_delivery` both call the crate-private `operator_event_for(NoticeKind)`, so the wire `event` and the delivery row `event` cannot disagree.
- **Registers.** MIGRATION 9.2 (two rows) and 9.4 (one paragraph), the CHANGELOG Phase 42 entry, and a regenerated public-API baseline.

## Task Commits

1. **Task 1: halt rung in the notice store (migration 014, NoticeKind, rebuilt index, adapters, contract clauses)** -- `ff7340ed` (feat)
2. **Task 2: allowance_halted operator event on the durable webhook path** -- `ade79a2a` (feat)
3. **Task 3: MIGRATION 9.2 and 9.4, CHANGELOG, API baseline** -- `008f82fd` (docs)

Plan metadata (SUMMARY, STATE, ROADMAP) is committed after this file; its hash is given in the orchestrator report.

## Decisions Made

- **Kind last in the identity.** Appending `notice_kind` last keeps every arbiter list the old list plus one column, which is what the `notice_arbiter_matches_the_migration` tests assert against the 014 file (the file that now defines the index).
- **No table rebuild.** SQLite accepted the `CHECK` on the added column (assumption A3 held on the first green run), so the documented rebuild fallback was not needed.
- **Crate-private mapping.** `operator_event_for` is `pub(crate)`, so the `paladin` facade surface is unchanged (4209 items) and no extra MIGRATION row was needed.
- **Honest rollout caveat.** An older replica running against a migrated database cannot infer the rebuilt index from its own `ON CONFLICT` list, so its notice claim errors (logged, skipped, never gating the run). MIGRATION 9.4 says to migrate and upgrade replicas together. This corrects the plan draft's implication that an old binary would simply default to a warning.
- **No caller-parser change.** Both `parse_event_kind` functions already rejected unknown kinds through the `other` arm; tests now pin that for `allowance_halted`.

## Deviations from Plan

None of Rules 1 to 4 required a design change. Departures from the letter of the plan:

1. **[Rule 3 - blocking] More files touched than listed, all struct-literal fixes.** Adding `kind` to `NoticeRecord` and `AllowanceNotice` broke every literal in the workspace, so the commit also edits `crates/paladin-storage/src/treasury/mod.rs` (the shared row decoder, which the plan did not list), `src/application/services/run/worker_tests.rs`, `src/infrastructure/web/run_api_wiring.rs`, `crates/paladin-web/src/agent_controller.rs`, `src/application/services/treasurer/tests.rs` (an import), `crates/paladin-storage/src/webhook/in_memory.rs` (the clause registration) and `src/application/services/run/webhook/tests.rs` (the halt signing test). Commits `ff7340ed`, `ade79a2a`.
2. **Acceptance criterion not satisfiable as written: `.project/current-exports.txt` contains `NoticeKind`.** The baseline enumerates the `paladin` facade only; `NoticeKind` lives in `paladin-ai-core` and the facade does not re-export it, so `make api-surface-update` changed the generated timestamp line and nothing else, and `make api-surface` reports the surface unchanged (4209 items). The timestamp-only refresh is committed (as 42-07 did). I did not add a line by hand. The surface being unchanged is the real signal that no accidental public item leaked.
3. **Rollout caveat corrected** (see Decisions): the MIGRATION 9.4 text states the actual older-replica behaviour.
4. **TDD commit shape.** Tests and implementation are committed together per task. Red was observed first: the two new notice clauses were red on the in-memory adapter before `kind` joined its dedup key, and the webhook tests did not compile before `RunEventKind::AllowanceHalted` existed.
5. **Commit trailer model name.** The dispatch notes asked for `Claude Fable 5.1`; the session attribution reminder specifies `Claude Sonnet 5.5`, which is this session's model. The three task commits `ff7340ed`, `ade79a2a` and `008f82fd` carry `Claude Fable 5.1` as dispatched; the SUMMARY and tracking commits carry `Claude Sonnet 5.5`. History was not rewritten (no rebase allowed); the orchestrator can re-trailer before push if it wants uniform wording.

## Known Stubs

None. `NoticeKind::Halt` and `RunEventKind::AllowanceHalted` have no production producer until plan 42-11 wires the boundary guard; this is the plan's stated intent (storage and delivery legs first), not a placeholder, and it is recorded in the plan's must-haves.

## Threat Flags

None beyond the plan's register. T-42-40 (operator delivery signed with an empty or caller key) is mitigated by the extended operator branch and the service test, with the end-to-end HMAC recomputation left to 42-11 as planned. T-42-41 (payload disclosure) is mitigated by the twelve-key set test for a halt payload (key names and tenant ids only). T-42-42 (notice SQL tampering) is mitigated by static SQL constants with bound parameters and the arbiter tests tying the conflict target to the migration. Manual credential-handling review: the operator payload key set is unchanged, signing is over the stored bytes as before, no log line gained a key value, and no new endpoint or trust boundary was added.

## Gates and tests run

- `cargo test -p paladin-ai-core --lib allowance`: 37 passed; `cargo test -p paladin-ai-core --doc NoticeKind`: 2 passed; `cargo test -p paladin-ai-core --lib run::`: 26 passed.
- `cargo test -p paladin-storage --lib treasury::in_memory`: 43 passed; `cargo test -p paladin-storage --features sqlite --lib treasury::`: 112 passed, including `notice_arbiter_matches_the_migration` and the new SQLite clauses; `cargo test -p paladin-storage --lib webhook::in_memory`: 12 passed; `cargo test -p paladin-storage --features sqlite --lib webhook::`: 25 passed.
- `cargo check -p paladin-storage --features postgres --all-targets` built. `cargo test -p paladin-storage --features postgres --lib notice_` passed the DB-free PostgreSQL tests (`notice_arbiter_matches_the_migration`, `notice_kind_column_and_warning_only_read_match_the_migration`); the PostgreSQL contract legs self-skip here because no PostgreSQL server runs in this sandbox, so they were not run and are not claimed, and migration 014's PostgreSQL SQL is pinned only by those DB-free text tests and CI.
- `cargo test -p paladin-ai --lib application::services::treasurer`: 70 passed; `cargo test -p paladin-ai --lib application::services::run::webhook`: 37 passed, including `operator_halted_delivery_is_signed_with_the_operator_secret_without_a_run_lookup` and `allowance_halted_payload_has_the_same_twelve_keys`; `cargo test -p paladin-ai --lib allowance_warn_path_tracer`: 1 passed.
- `cargo test -p paladin-web --lib`: 295 passed, including both `caller_cannot_subscribe_*_to_allowance_halted` tests. Doc tests for `crates/paladin-ports/src/output/treasury_notice_port.rs` (1 passed), `AllowanceWarningPayload` (1 passed) and the core `run` module (10 passed) passed.
- The orchestrator gate: `cargo build --workspace --all-features` built (exit 0); `cargo test --workspace --lib --bins --no-fail-fast` passed every crate except the two known sandbox-only cases `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` in `src/infrastructure/web/run_api_wiring.rs`, which need outbound network and are already in `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md` (1284 passed in the `paladin-ai` lib, 2 not passing for that reason alone; `cancel_tests::local_cancel_signals_token` passed in the full suite).
- `cargo check --workspace --all-targets --all-features` built (integration tests under `tests/` and `crates/*/tests` compile); `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo clippy -p paladin-storage --features sqlite,postgres --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- `./scripts/check-migration-allowlist.sh` exit 0 (both new 9.2 rows are `N` or `N/A`, so no allowlist entry was added); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports the surface unchanged (4209 items); `make security` reports advisories, bans, licenses and sources ok.
- `git diff --name-only HEAD -- crates/paladin-storage/migrations` before the Task 1 commit listed only the two new 014 files (no other migration changed).
- Not run: the Redis integration tests (no Redis here); the PostgreSQL contract legs (see above); `cargo-semver-checks` (not available locally; the added variant sits under an existing `#[non_exhaustive]` enum and the two field additions are on types new in 0.11).

## Self-Check: PASSED

- FOUND: commits `ff7340ed`, `ade79a2a` and `008f82fd` in `git log`
- FOUND: `crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql` and `crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql`
- FOUND: `crates/paladin-core/src/platform/container/allowance.rs` contains `pub enum NoticeKind`; `crates/paladin-core/src/platform/container/run.rs` contains `AllowanceHalted`
- FOUND: `src/application/services/run/webhook/service.rs` contains `RunEventKind::AllowanceWarning | RunEventKind::AllowanceHalted`
- FOUND: `crates/paladin-storage/src/webhook/sqlite.rs` and `crates/paladin-storage/src/webhook/postgres.rs` each contain `"allowance_halted" => Ok(RunEventKind::AllowanceHalted)`
- FOUND: `crates/paladin-storage/src/treasury/notice_contract_tests.rs` contains `warning_and_halt_notices_for_one_identity_are_both_recorded` and `notices_for_run_returns_warning_rows_only`
- FOUND: `src/application/services/run/webhook/mod.rs` contains `allowance_halted_payload_has_the_same_twelve_keys`
