---
phase: 41-admission-time-allowance-enforcement
verified: 2026-10-04T12:30:00Z
status: human_needed
score: 3/3 roadmap success criteria verified; 54/57 plan truths verified (3 backstop truths abstain by design, 0 failed)
behavior_unverified: 0
overrides_applied: 0
re_verification: null
gaps: []
deferred: []
human_verification:
  - test: "Operator UAT walkthrough (41-VALIDATION manual-only row; the SUMMARYs say it was not run)"
    expected: "With treasurer.allowance.api_keys.ci-runner { period: 1h, amount: 2.50 } and the key's ledger balance at 2.5000 USD, the next POST /v1/runs answers 429 code allowance_exhausted, Retry-After equals the seconds left in the current UTC hour, details names balance/ceiling/window_start/window_end, and `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same 2.5000 USD figure. Then set an allowance warn crossing (80%) with a webhook target and confirm one operator POST, one trace event and one herald line, and nothing on a second admission in the same window."
    why_human: "Needs a running paladin-server, a real operator config and a receiver. The automated tracers (allowance_admission_tracer, allowance_warn_path_tracer) prove the same chain in-process over on-disk SQLite, but no one has driven the real binary and CLI."
  - test: "CI-authoritative gates: coverage job (>= 82% workspace line coverage, ADR-0006) and postgres-integration job (no SKIP: lines)"
    expected: "Both jobs green on the pushed branch. PostgreSQL legs for balance, treasury_notices (011/012), run_schedules.created_by (010) and the operator webhook delivery row must execute rather than skip."
    why_human: "PostgreSQL is not running in this environment, so the env-gated PostgreSQL tests skip here. The SUMMARYs of 41-02, 41-05, 41-06 and 41-08 record real runs on a throwaway cluster (32, 14, 147 and 11 passed, 0 SKIP). I did not re-execute them. The coverage figure is only computed in CI."
  - test: "Accepted over-admission race between two admissions in the same instant (41-02 backstop truth)"
    expected: "Confirm ADR-0056 records the race and its closure by Phase 42's reservation at the superstep boundary, and accept it as a known property of check-only admission (D-05)."
    why_human: "Cannot be proven or prevented mechanically under D-05. The Treasurer proofs show admission is read-only and concurrency-safe, but two simultaneous admissions can both pass. ADR-0056 does record it."
  - test: "Crash between a won notice claim and the run insert (41-06 backstop truth) and RunWorkerPool::with_treasury_notices inside build_run_api (41-08 backstop truth)"
    expected: "Accept that a crash in that window can lose one window's notice but never duplicates it or persists a run, and that the production-builder attachment of the notice store to the worker pool has no observable output of its own. The worker tests and the integrated tracer prove the same code path with the same builder call."
    why_human: "Both are `verification: backstop` truths (non-inferable by construction) and so abstain rather than pass."
---

# Phase 41: Admission-Time Allowance Enforcement Verification Report

**Phase Goal:** A tenant or API key that has exhausted its allowance is stopped before a run is ever persisted, and the operator gets an early warning before that happens.
**Verified:** 2026-10-04T12:30:00Z
**Status:** human_needed
**Re-verification:** No (initial verification; no previous 41-VERIFICATION.md existed)

The phase goal is achieved in the code. Nothing failed. The status is `human_needed` only because the decision tree routes every human-verification item (operator UAT, CI-only gates, three `backstop` truths) to that status, not because anything is missing.

## Goal Achievement

### Roadmap Success Criteria (the contract)

| # | Success criterion | Status | Evidence |
|---|-------------------|--------|----------|
| 1 | Operator can configure a rolling-period allowance (optional lifetime cap) per tenant and per API key, distinct from every `max_tokens` meaning; window boundaries from the store/server clock, never a worker's local clock | VERIFIED | Grammar in `src/config/treasurer.rs`: `AllowanceConfig { warn_at, webhook, tenants, api_keys }`, `AllowanceEntryConfig { period, amount, lifetime, warn_at }`, `deny_unknown_fields` on every struct, exact-integer `parse_decimal_nanos` (no `f64`), `parse_period_secs` (`<int><m\|h\|d>`, 1m..366d). It lives under `treasurer.allowance`, a different key from the four documented `max_tokens` senses (`docs/src/getting-started/configuration.md` line ~497 lists the four; the new section is separate). Window math is `window_for(now, period)` in `src/application/services/treasurer/window.rs` (pure, epoch-aligned, half-open). `Treasurer::admit` takes `now` from one `ledger.store_now()` call truncated to seconds. A grep for `Utc::now`, `SystemTime`, `Instant::now`, `f64` over the treasurer module (`mod.rs`, `policy.rs`, `window.rs`) returns nothing. SQLite `store_now` is the database's own `STORE_NOW` query and PostgreSQL is `SELECT now()`. Test `store_clock_selects_the_window_not_the_local_clock` scripts a store clock in 2001 and passes. Ran: `config::treasurer` allowance tests (all pass). |
| 2 | Submitting a run while the caller's tenant or API-key allowance is exhausted is refused with a typed error before any run row is written | VERIFIED | `RunSubmissionService::submit` and `fork` both call `admit_and_persist` (`src/application/services/run/submission.rs` ~362-386, called at ~457 and ~630) which runs `treasurer.admit` BEFORE `persist_and_enqueue` (`insert_with_latest`/`insert` + `enqueue`), after the SSRF guard, `resolve`, `authorize_invocation` and the thread-visibility check. A refusal is the typed `RunSubmissionError::AllowanceExhausted(AllowanceRefusal)`, mapped to `429 allowance_exhausted` plus `Retry-After` (omitted for lifetime) by `ApiError::allowance_exhausted` in `crates/paladin-web/src/error.rs`, used by `run_controller::map_submission_error`, the fork route and the agent handlers. `balance >= ceiling` is refused (D-05); any ledger failure for an allowanced principal fails closed (`Backend`, 500); a principal with no entry never touches the ledger. Ran: `allowance_admission_tracer` (real router + on-disk SQLite ledger: 429, `Retry-After` within one window, six-key details, zero runs persisted, queue empty, sibling key with no entry gets 202) passes. Also gated: `POST /agents/{id}/execute`, `/execute/stream`, `/jobs` (`admit_principal` before executor and before `jobs.create`), `POST /threads/{id}/fork`, and schedule-fired runs (`attributed_to: schedule.created_by`, `AllowanceExhausted` becomes a counted `Skipped`). |
| 3 | Crossing a configurable warn threshold (e.g. 80%) emits exactly one trace event plus a herald and webhook notice per window, without blocking the run | VERIFIED | Dedup is store-enforced: migration `011_create_treasury_notices.sql` (both backends) has `UNIQUE (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)`, adapters use `INSERT ... ON CONFLICT DO NOTHING`, `''`/epoch sentinels avoid the NULL-in-UNIQUE trap (C5). Crossing test is integer-only (`crosses_warn_threshold`, i128). Claims happen only after every ceiling admits; a notice-store failure is logged and swallowed (never blocks); `abandon` gives notices back if the run is never persisted. Trace: the worker emits one `TraceEvent::AllowanceWarning` per won notice on the run's first dispatch (`Queued` only) before `RunStarted` (`worker.rs` `emit_allowance_warnings`). Herald: one shared `allowance_warning_display` rendered by the markdown, JSON and table heralds. Webhook: `Treasurer::confirm` enqueues one `allowance_warning` delivery on the existing durable queue. Ran: `allowance_warn_path_tracer` (one admission at 80%: one notice row, one signed POST, one trace record before `RunStarted`, one herald line, run completes; a second admission adds nothing) and `build_run_api_wires_the_allowance_warn_path` (production builder) pass; `sixteen_concurrent_crossings_yield_exactly_one_notice` passes. |

### Plan-level truths (57 across nine plans)

54 verified, 3 abstain (`verification: backstop`, listed under Human Verification), 0 failed. The core ones I checked directly in code and by running the named tests:

| Truth | Status | Evidence |
|-------|--------|----------|
| 41-01 tracer: 429 body carries no tenant, key name or key value | VERIFIED | `allowance_exhausted` builds `details` from only the refused ceiling's six figures; tracer asserts on raw bytes. |
| 41-01/02 `balance` on all adapters, window half-open, signed SUM, currency mismatch refused | VERIFIED | All three adapters override it (in-memory, SQLite, PostgreSQL); static SQL prefixes plus `push_bind`; ran `treasury::` on in-memory and SQLite (105 passed). PostgreSQL leg not re-run here. |
| 41-02 ordering, short-circuit, fail-closed, read-only, no role input | VERIFIED | `policy.rs` fixes key-window, key-lifetime, tenant-window, tenant-lifetime; `admit` has no role input; ran `application::services::treasurer` (43 passed). |
| 41-03 grammar strictness, boot coherence (D-11), Treasurer built in `build_run_api` before the `Disabled` early return | VERIFIED | `validate_against` runs at `run_api_wiring.rs` ~558 before any early return; `known_allowance_targets`; ran `run_api_wiring` tests (24 passed incl. the unknown-key/tenant/disabled-store rejections). |
| 41-04 agent routes, fork, jobs gated; OpenAPI 429 published with a narrow v0.9 golden exception | VERIFIED | `admit_principal` in all three handlers; `allowance_429_exception_is_narrowly_scoped` and the golden suite pass (8 passed). |
| 41-05 schedule creator persisted (010), fired runs attributed and admitted, role check unchanged | VERIFIED | `RunSchedule.created_by`, `SubmitRun.attributed_to`, `authorize_invocation` reads `requested_by` only; ran schedule allowance tests (4 pass). SQLite 008 to 010 gap documented. |
| 41-06 once-per-window notices, abandon, 012 index additive | VERIFIED | Migrations 011/012 present, `007` byte-untouched (git diff empty); ran the contract (in-memory + SQLite) via `treasury::`. |
| 41-07 trace event, herald line, `Queued`-only emission, no re-emit on redelivery/resume | VERIFIED | `worker_tests::allowance_warnings` (5 tests incl. redelivery and resume) pass. |
| 41-08 operator webhook (twelve keys, option-b), signed with operator secret before any run lookup, never listed under the run, SSRF at boot | VERIFIED | See D-17 section below; payload and delivery-service tests pass. |
| 41-09 ADR-0056 exists, `PROMOTION.md` at 0057, facade re-export, registers | VERIFIED | `.planning/decisions/0056-allowance-admission-model.md` exists; `Next free ADR number: 0057`; `pub use ...::allowance;` in `src/core/platform/mod.rs`; `check-migration-allowlist.sh` exits 0 (set-equal). |

### D-17 option-b amendment (checkpoint outcome) honoured

`AllowanceWarningPayload` (`src/application/services/run/webhook/mod.rs`) serializes exactly twelve keys: `event, scope, kind, balance, ceiling, window_start, window_end, warn_at, run_id, timestamp, tenant_id, api_key_id`. Test `allowance_warning_payload_serializes_with_exactly_the_documented_keys` pins the sorted key set for both scopes and `allowance_warning_payload_has_no_secret_or_input` guards the forbidden keys. `api_key_id` is the key name (`ApiKeyConfig.name`), never a value. `docs/src/api-reference/platform-api.md` (line ~384, "exactly these twelve keys") and `MIGRATION.md` 9.6 (line ~777) both state twelve keys and the option-b amendment. The decision is recorded in `41-01-SUMMARY.md` and `41-CONTEXT.md` D-17 and in ADR-0056.

### RESEARCH corrections C1..C14 (pitfall check)

| # | Correction | Status |
|---|-----------|--------|
| C1 web cannot name `Treasurer` | Honoured: `AllowanceAdmissionPort` input port in `paladin-ports`; `AgentApiState.treasurer: Option<Arc<dyn AllowanceAdmissionPort>>`. |
| C2 SQLite has no `009` | Honoured: SQLite goes 008 to 010; `sqlx::migrate!` accepts the gap (SQLite suite opens a fresh DB). |
| C3 operator webhook vs delivery row | Honoured (Option A): `RunEventKind::AllowanceWarning` (`#[non_exhaustive]`), fresh correlation `RunId` plus fixed thread `treasurer-notices`, operator branch before `runs.get`; caller event parsers still reject it (tests pass). |
| C4 `jobs` was ungated | Honoured: `enqueue_job` gated before `jobs.create`. |
| C5 NULL in UNIQUE | Honoured: `api_key_id NOT NULL` with `''`, epoch for lifetime. |
| C6 trace `seq` collision | Honoured: worker emits on the run's own dispatcher, first dispatch only. |
| C7 no `${VAR}` expansion | Honoured: env override for the secret, `yaml_env_placeholder_is_not_expanded` pins it, WINDOWS.md row 62 filed. |
| C8 `Retry-After` from store clock | Honoured: `evaluated_at` on the refusal; no clock read in `paladin-web`. |
| C9 golden v0.9 gate | Honoured: third narrow exception, scope-guard test. |
| C10 `BalanceQuery` currency and `Option` bounds | Honoured. |
| C11 register disposition | `TreasuryLedgerPort::balance` is a defaulted method (rows `N`); the defaulted body fails closed with `InvalidRequest`. All production implementors override it. |
| C12 removed creator key | Honoured: tenant allowance only, tested. |
| C13 `SubmitRun.attributed_to` | Honoured; schedule-fired runs keep skipping the role check (written decision, tested). |
| C14 `build_run_api` early return | Honoured: coherence check precedes it; `RunApiHandles.treasurer` handed to `AgentApiState` in `paladin-server.rs`. |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/paladin-core/src/platform/container/allowance.rs` | VERIFIED | 824 lines; `AllowanceRefusal`, `AllowanceWarning`, `AllowanceNotice`, `Admission`, `NoticeRecord`, `crosses_warn_threshold`; wired into trace/herald/webhook/web. |
| `crates/paladin-ports/src/input/allowance_admission_port.rs`, `output/treasury_notice_port.rs` | VERIFIED | Ports implemented by `Treasurer` and the three storage adapters. |
| `src/application/services/treasurer/{mod,policy,window,tests}.rs` | VERIFIED | Substantive and wired from `run_api_wiring.rs`. |
| Migrations `010`, `011`, `012` (SQLite and PostgreSQL) | VERIFIED | Present; `007` untouched. |
| `src/config/treasurer.rs` allowance grammar | VERIFIED | |
| `TraceEvent::AllowanceWarning`, three heralds, `HeraldTraceSink` fold | VERIFIED | |
| `AllowanceWarningPayload`, `with_operator_notice_secret`, `OperatorNoticeTarget` | VERIFIED | |
| ADR-0056, `PROMOTION.md`, `MIGRATION.md` 9.2/9.4/9.5/9.6, CHANGELOGs, `config.example.yml`, docs | VERIFIED | `make check-gates` allowlist script exits 0; no `TBD` in `MIGRATION.md`. |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `build_run_api` | `Treasurer` then `RunSubmissionService::with_treasurer` | WIRED | One submission service is shared by `/runs`, fork and the schedule service. |
| `build_run_api` | `RunApiHandles.treasurer` then `AgentApiState::with_treasurer` | WIRED | `paladin-server.rs` ~240. |
| `submit`/`fork` | `Treasurer::admit` before insert/enqueue | WIRED | `admit_and_persist`. |
| `RunSubmissionError::AllowanceExhausted` | 429 + `Retry-After` | WIRED | `map_submission_error`, fork route, agent handlers. |
| `Treasurer::admit` | `TreasuryNoticePort::record` then worker `notices_for_run` then `TraceEvent` then herald | WIRED | `with_treasury_notices` attached in `build_run_api`. |
| `Treasurer::confirm` | `webhook_deliveries` enqueue then `WebhookDeliveryService` operator branch | WIRED | Secret held only on the delivery service. |
| schedule fire site | `SubmitRun.attributed_to` | WIRED | `schedule/service.rs` ~356. |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| Refusal figures | `balance` | `TreasuryLedgerPort::balance` SQL SUM over `treasury_ledger` | Yes (tracer seeds a real 2.50 USD settle) | FLOWING |
| Warn trace/herald | `AllowanceWarning` | `treasury_notices` row read by the worker | Yes (`allowance_warn_path_tracer`) | FLOWING |
| Operator webhook | payload | `AllowanceNotice` from the won claim | Yes (HMAC verified over captured bytes in tracer) | FLOWING |

### Behavioral Spot-Checks (targeted runs only; the full `make test` was left to the background job)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Allowance behaviors in the facade crate | `cargo test -p paladin-ai --lib allowance` | 41 passed, 0 failed (includes `allowance_admission_tracer` and `allowance_warn_path_tracer`) | PASS |
| Treasurer rule proofs | `cargo test -p paladin-ai --lib application::services::treasurer` | 43 passed | PASS |
| Core types, trace, herald, scope | `cargo test -p paladin-ai-core --lib allowance` | 26 passed | PASS |
| Ledger balance + notices contract (in-memory, SQLite) | `cargo test -p paladin-storage --features sqlite --lib treasury::` | 105 passed | PASS |
| 429 mapping, caller cannot subscribe to the operator event | `cargo test -p paladin-web --lib allowance` | 8 passed | PASS |
| Frozen v0.9 OpenAPI golden | `cargo test -p paladin-web --test openapi_golden_v0_9` | 8 passed | PASS |
| Production builder wiring | `cargo test -p paladin-ai --features web-server --lib infrastructure::web::run_api_wiring` | 24 passed | PASS |
| Register set-equality | `./scripts/check-migration-allowlist.sh` | exit 0 | PASS |
| Treasurer module has no local clock or float | grep `Utc::now\|SystemTime\|Instant::now\|f64\|f32` | none | PASS |
| `007` migrations, engine superstep, `limits.rs` untouched (D-00b) | `git diff 8be4972^..HEAD -- <those paths>` | 0 lines | PASS |

### Probe Execution

No probe scripts are declared by any PLAN or SUMMARY; step 7c SKIPPED.

### Requirements Coverage

| Requirement | Source plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| ALLOW-01 | 41-01, 41-02, 41-03, 41-09 | Per-tenant and per-key allowance, rolling period, optional lifetime cap, distinct from `max_tokens`, store-clock windows | SATISFIED | Success criterion 1. `REQUIREMENTS.md` marks it `[x]`, traceability row "Phase 41 / Complete". |
| ALLOW-02 | 41-01, 41-02, 41-04, 41-05, 41-09 | Exhausted allowance refused at admission with a typed error, no run persisted | SATISFIED | Success criterion 2. |
| ALLOW-04 | 41-06, 41-07, 41-08, 41-09 | Warn threshold: one trace event plus herald and webhook notice per window, non-blocking | SATISFIED | Success criterion 3. |

All three IDs declared across the nine PLAN frontmatters (ALLOW-01, ALLOW-02, ALLOW-04) are accounted for. ORPHANED: none. `REQUIREMENTS.md` maps only these three to Phase 41; ALLOW-03 and ALLOW-05 are mapped to Phase 42 and were correctly not claimed.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| (all added lines in the phase diff) | `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`todo!`/`unimplemented!` | none | Grep over the added lines of `8be4972^..HEAD` (excluding `.planning` and the API baseline) returns nothing. `MIGRATION.md` has 0 `TBD`. |
| Treasurer production code | `unwrap`/`expect`/`panic!` | none | The only hits (`policy.rs` 219, 223) are inside `#[cfg(test)]`. |

No blockers. No stubs: `Treasurer::confirm`, a no-op in 41-01 through 41-07, is a real enqueue as of 41-08.

### Known Limitations (accepted and documented, not gaps)

These are decisions the operator took or ADR-0056 records. They are here so a reader of this report sees them.

1. **Agent-route warn legs are best-effort.** On `POST /agents/{id}/execute` and `/jobs` there is no run stream, so the trace event is emitted only when an agent-path trace emitter is wired and the herald line appears only on the streamed final chunk. The durable notice row and the operator webhook always fire. This is what CONTEXT D-18 asks for on the agent path and ADR-0056 states it. Roadmap criterion 3 is fully proven on the `/runs` path (trace, herald and webhook), which is the run-submission path the criterion is about.
2. **Pre-Phase-41 schedules (`created_by` NULL) fire unattributed and ungated** (D-08, WINDOWS.md row 63, open). Re-creating the schedule through `POST /v1/schedules` brings it under allowances.
3. **Over-admission race and 2x boundary burst** are accepted properties of check-only admission with tumbling windows (D-01, D-05); Phase 42 closes the race with a reservation.
4. **`${VAR}` placeholders are not expanded by the config loader** (WINDOWS.md row 62, open, owner Phase 46); the allowance webhook secret therefore has an explicit env override.
5. **In-memory ledger's `store_now` is the process clock** (the store *is* the process there); SQLite and PostgreSQL read the database clock.

### Human Verification Required

See the `human_verification` list in the frontmatter: (1) operator UAT walkthrough against a real server and CLI, (2) CI `coverage` and `postgres-integration` jobs, (3) acceptance of the over-admission race, (4) acceptance of the two remaining backstop truths.

### Gaps Summary

No gaps. All three roadmap success criteria are observably true in the codebase, all nine plans' artifacts exist, are substantive, wired and carry real data, the D-17 option-b amendment (twelve-key payload with `tenant_id` and `api_key_id` names) is implemented and documented, and the targeted tests I ran (41 + 43 + 26 + 105 + 8 + 8 + 24 test results across the allowance surface) all pass. I did not re-run PostgreSQL legs, the coverage job, or the full workspace suite (the latter was already running). A housekeeping note for the orchestrator: the nine SUMMARYs record mixed commit trailers (`Claude Sonnet 5.5` on some commits, `Claude Fable 5.1` on others) and flag it for normalisation before push; this does not affect goal achievement. `41-VALIDATION.md` still reads `status: draft`, `nyquist_compliant: false`; that is set by `/gsd-validate-phase 41`, not by verification.

---

_Verified: 2026-10-04T12:30:00Z_
_Verifier: Claude (gsd-verifier)_
