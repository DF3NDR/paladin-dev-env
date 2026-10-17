---
phase: 42
slug: mid-run-halt-sse-terminal-status
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
threats_total: 48
threats_closed: 48
created: 2026-10-07
verified: 2026-10-07
---

# Phase 42 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> Source register: the `<threat_model>` blocks of plans 42-01 through 42-12 (48 threats, authored at
> plan time) plus the `## Threat Flags` section of each of the twelve SUMMARYs. Verification depth:
> ASVS L1 (grep-level presence of each named control, test or document in the tree at
> `e4cddd5f`), per `workflow.security_asvs_level: 1`. The Phase 42 code review (42-REVIEW.md,
> 42-REVIEW-FIX.md) ran before this audit; its security-adjacent fixes (WR-3, WR-4, WR-5, IN-6)
> are folded into the evidence below.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| operator decision -> design record | the 42-01 checkpoint outcome (ADR-0057) is the contract every later plan implements | design decisions D-01..D-21 |
| engine -> facade policy | the engine asks the `SpendGuard` port; only the facade's Treasurer reads the ledger | `RunAttribution` (tenant id, key NAME), run id; never a role or key value |
| facade -> ledger / store clock | balance and clock reads at every superstep boundary that may fail | balances, window figures (server-derived scope identifiers) |
| facade -> notice store | once-per-window claims keyed by server-derived scope identifiers | scope kind, tenant id, key id, limit kind, window start, ceiling, notice kind |
| run row -> HTTP response | `halt_reason` / `final_waypoint_id` leave the process through `load_visible_run` tenant scoping | halted ceiling's own figures only |
| stored column -> typed read | a stored `halt_reason` is parsed on read; a corrupt value reads as `None` | serialized `HaltReason` JSON |
| engine trace -> SSE subscriber | `done` leaves the process to a stream subscriber already authorized for the run | status, `halt_reason` figures |
| stored `run_traces` -> replay | persisted records are re-read and re-emitted; non-terminal rows are skipped | trace records |
| worker shutdown -> SSE subscribers | a drain must never look like a finish | reasonless `Halted` (dropped) |
| caller -> cancel route | an authorized caller asks a run to stop (unchanged gate) | cancel request |
| model response -> cutoff | provider-reported usage drives the cumulative token count the derived budget compares | token usage |
| HTTP client -> agent routes / `POST /v1/runs` / `POST /threads/{id}/fork` | untrusted body; principal server-derived; model from the registered agent or resolved assistant, never the request | request body |
| queue -> worker dispatch | a run admitted earlier is re-evaluated against the ledger at dispatch | ledger figures |
| worker -> caller webhook receiver | caller-chosen URL (SSRF-guarded, no redirects, signed over the stored bytes) | `halted` payload with `halt_reason` |
| delivery row -> operator webhook URL | operator-configured target (SSRF-checked at boot and at send), signed with the operator secret | `allowance_halted` payload (key NAMES and tenant ids only) |
| replicas of mixed versions -> `webhook_deliveries.event` | a new event value read by an older build | `allowance_halted` |
| run trace -> herald output | the halt line leaves the process wherever the operator's herald writes | scope kind, figures, window end |
| repository tree -> vocabulary guard | the guard test reads every source file | file contents (test-only) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-42-01 | Repudiation | ADR-0057 vs the shipped code | low | mitigate | `.planning/decisions/0057-mid-run-halt-contract.md` "Code Conformance" section names the test holding each decision; 42-12 closeout re-read it (42-12-SUMMARY) | closed |
| T-42-02 | Tampering | a later plan silently diverging from the gate outcome | medium | mitigate | outcome recorded verbatim in 42-01-SUMMARY.md; every later plan's `read_first` names it | closed |
| T-42-03 | Elevation of Privilege | over-admission race recorded as accepted (D-01) | medium | accept | ADR-0057 D-01 (rejected alternative: per-superstep hold); WINDOWS.md row 65 — see AR-42-01 | closed |
| T-42-04 | Elevation of Privilege | allowance bypass mid-run | high | mitigate | `SpendGuard` port (`crates/paladin-ports/src/output/spend_guard.rs`) takes a `RunAttribution` (`crates/paladin-core/src/platform/container/principal.rs`), never a role; `Treasurer::spend_guard` (`src/application/services/treasurer/guard.rs`); `engine_spend_halt_tracer` (`src/application/services/run/http_surface_tests.rs`) | closed |
| T-42-05 | Elevation of Privilege | fail-open on a ledger error at the boundary | high | mitigate | `SpendDecision::Halt(HaltReason::LedgerUnavailable)` memoised (`guard.rs`); `guard_fails_closed_with_ledger_unavailable_when_balance_errs`, `guard_memoises_a_ledger_unavailable_halt_too` (`treasurer/tests.rs`); WR-4 extended fail-closed to an unwritable charge | closed |
| T-42-06 | Information Disclosure | halt log lines | medium | mitigate | `guard.rs` halt line carries run id, scope kind, limit kind, tenant id and `format_cost` figures only; the guard holds a key NAME via `RunAttribution`, never a value | closed |
| T-42-07 | Tampering | balance SQL | low | accept | unchanged Phase 41 adapters with bound parameters; the guard adds no SQL — see AR-42-02 | closed |
| T-42-08 | Denial of Service | per-boundary balance reads | low | accept | at most four reads plus one clock read per boundary (D-02 locks no caching); migration 012 tenant index — see AR-42-03 | closed |
| T-42-09 | Information Disclosure | `halt_reason` on `GET /runs*` | medium | mitigate | served only through `load_visible_run` (`crates/paladin-web/src/run_controller.rs`); `HaltReason::wire_json` carries the ceiling's figures only; `http_surface_tests.rs` asserts the raw body and run list carry no key value | closed |
| T-42-10 | Tampering | `record_outcome` SQL | high | mitigate | static `const` SQL in `crates/paladin-storage/src/run/{sqlite,postgres}.rs`; `halt_reason` bound via `halt_reason_to_sql` as serialized JSON, never interpolated; `halt_reason_round_trips_on_record_outcome` contract test | closed |
| T-42-11 | Denial of Service | corrupt stored reason | low | mitigate | `run/sqlite.rs` reads an unparseable `halt_reason` as `None` with a run-id-only log line, never a panic or page failure (review IN-6, commit `ab7ee51d`); `legacy_row_reads_back_without_a_halt_reason` on all three adapters | closed |
| T-42-12 | Spoofing | webhook signature after adding a key | high | mitigate | payload serialized once, stored verbatim, signed from that buffer; `halted_webhook_signature_covers_the_halt_reason` (`run/webhook/tests.rs`); `recompute_signature` over captured raw bytes (`http_surface_tests.rs`) | closed |
| T-42-13 | Repudiation | `done`/row race (status before reason) | medium | mitigate | `record_outcome` before `update_status` for halting transitions (G14, `run/worker.rs`); `worker_tests.rs` "halting transitions write the outcome before the status" block; review IN-8 documents the accepted window | closed |
| T-42-14 | Elevation of Privilege | resume via fork while exhausted | high | mitigate | `fork_by_an_exhausted_principal_is_refused_and_touches_nothing` (`run/submission.rs`); `fork_route_maps_allowance_exhausted_to_429` (`thread_controller.rs`) | closed |
| T-42-15 | Elevation of Privilege | fail-open on ledger/store-clock/currency errors | high | mitigate | three named tests: `guard_fails_closed_with_ledger_unavailable_when_balance_errs`, `..._when_the_store_clock_errs`, `..._on_a_currency_mismatch` | closed |
| T-42-16 | Tampering | child halt silently passed by the parent after a window roll | medium | mitigate | memoised first halt shared into child runs; `child_battalion_halt_on_spend_halts_the_parent` (`crates/paladin-battalion/src/engine/superstep.rs`); review CR-1 and IN-12 | closed |
| T-42-17 | Elevation of Privilege | concurrent runs racing past the ceiling | medium | accept | D-01 accepted race, bounded to one top-level superstep per run (WR-1); WINDOWS.md row 65 — see AR-42-01 | closed |
| T-42-18 | Information Disclosure | `ledger_unavailable` error log | low | mitigate | `fail_closed_message` (`guard.rs`) names run, scope kind, tenant id and the redacted backend error only; `fail_closed_log_line_names_run_scope_tenant_and_error_only` | closed |
| T-42-19 | Spoofing | replayed `done` for a drained (still running) run | medium | mitigate | replay skips a `RunFinished` whose row is not terminal (`run/events.rs`); `replay_skips_a_drained_halted_record_when_the_run_later_{completed,halted_on_spend,cancelled}` (`stream_tests.rs`, WR-2/WR-5) | closed |
| T-42-20 | Information Disclosure | `halt_reason` on the stream | medium | mitigate | stream routes gated by `load_visible_run`; object carries the halted ceiling's own figures only (`wire_json`) | closed |
| T-42-21 | Tampering | stored-trace compatibility | low | mitigate | `#[serde(default)]` on the new field; `legacy_run_finished_row_reads_back_without_a_halt_reason` contract test on in-memory, SQLite and PostgreSQL (`crates/paladin-storage/src/run_trace/`) | closed |
| T-42-22 | Information Disclosure | OTel export of the reason | low | mitigate | `src/infrastructure/telemetry/otel_sink.rs` adds no `halt_reason` span attribute (only test fixtures mention the field) | closed |
| T-42-23 | Spoofing | a terminal `done` for a still-running (drained) run | medium | mitigate | bus sink drops the reasonless `Halted` while shutting down (`run/events.rs` D-15/G1c); `drain_streams_no_done_and_leaves_the_run_running` (`stream_tests.rs`); `shutdown_drains_in_flight_run` (`worker_tests.rs`) | closed |
| T-42-24 | Repudiation | cancelled run reported as halted | medium | mitigate | `PerRunCancelProbe` + `RunFinishStatus::Cancelled` (`run/cancel.rs`, `run/events.rs`); `same_instance_cancel_streams_done_cancelled`, `cross_instance_cancel_streams_done_cancelled` (`cancel_tests.rs`); review IN-1 | closed |
| T-42-25 | Denial of Service | probe ORing a local token | low | accept | in-memory flag read; the debounced DB probe is unchanged — see AR-42-04 | closed |
| T-42-26 | Tampering | shutdown-instant race (A7) | low | accept | backstop truth; the degraded path converges on the row — see AR-42-05 | closed |
| T-42-27 | Elevation of Privilege | spend past the allowance on the agent loop | high | mitigate | `derive.rs` derived budget from the dearest axis with floor division; cutoff after each response (`StopReason::AllowanceHalted`, `paladin/middleware/limits.rs`); `admit_for_model_derives_from_the_remaining_allowance_at_the_dearest_price`; HTTP and worker end-to-end in 42-08/42-09 | closed |
| T-42-28 | Elevation of Privilege | unpriced model under an allowance | high | mitigate | `AdmissionError::ModelUnpriced` at admission; `unpriced_model_is_refused_422_on_execute_stream_and_jobs` (`agent_controller.rs`); `an_unpriced_model_under_a_ceiling_is_refused_before_any_notice_claim` | closed |
| T-42-29 | Tampering | arithmetic overflow or rounding in the derivation | medium | mitigate | `i128` products, floor division, saturating `u32` (`derive.rs`); table tests including `derive_max_tokens(i64::MAX, 1) == Some(u32::MAX)` and non-divisible pairs | closed |
| T-42-30 | Denial of Service | operator budget newly effective on HTTP routes | low | accept | documented in `docs/src/getting-started/configuration.md` and CHANGELOG `### Changed` (42-08); inert unless the operator enabled it — see AR-42-06 | closed |
| T-42-31 | Information Disclosure | `halt_reason`/422 bodies on JSON and SSE `done` | low | mitigate | one `wire_json` builds the figures; 422 names the model only; `agent_host.rs` asserts `!raw.contains("key-a")` on the raw body | closed |
| T-42-32 | Elevation of Privilege | concurrent agent calls racing the allowance | medium | accept | same accepted race as admission (D-01); backstop truth — see AR-42-01 | closed |
| T-42-33 | Elevation of Privilege | agent-kind spend past the allowance via the worker | high | mitigate | re-derive at dispatch; `agent_kind_run_with_zero_budget_at_dispatch_halts_without_calling_the_llm`, `agent_kind_run_with_an_unreadable_ledger_at_dispatch_halts_ledger_unavailable_without_calling_the_llm` (`worker_tests.rs`) | closed |
| T-42-34 | Tampering | engine nodes capped by a shared budget | high | mitigate | Treasurer-only mode on the shared service; `shared_service_never_caps_an_engine_node` (`facade_provisioner.rs`) | closed |
| T-42-35 | Elevation of Privilege | unpriced agent model at `POST /runs` | high | mitigate | `admit_for_model` refusal before any row; `submit_agent_kind_with_an_unpriced_model_is_refused_before_any_row` (`submission.rs`); `map_submission_error_maps_model_unpriced_to_422` | closed |
| T-42-36 | Repudiation | agent-kind halts not resumable from a checkpoint | low | accept | D-08 by construction; WINDOWS.md row 64; MIGRATION.md 9.6 — see AR-42-07 | closed |
| T-42-37 | Information Disclosure | 422 body and halt logs | low | mitigate | 422 carries the model name only; dispatch log lines name run id and ledger error text only; end-to-end asserts neither key value nor tenant in the body (42-09-SUMMARY, manual credential review) | closed |
| T-42-38 | Denial of Service | operator notification storm from many halting runs | medium | mitigate | store-enforced dedup with `notice_kind` in the unique key (migration 014, `crates/paladin-storage/src/treasury/`); `guard_halt_claims_one_halt_notice_and_one_operator_delivery`; 42-11 end-to-end (run B adds no POST) | closed |
| T-42-39 | Denial of Service | a notice insert at every boundary | low | mitigate | in-run claim memo (`guard.rs`, `Arc`-held); WR-3 keeps a failed write un-memoised so it retries; single-store-write and sticky-halt unit tests (42-11) | closed |
| T-42-40 | Spoofing | operator delivery signed with an empty or caller key | high | mitigate | operator branch extended to `RunEventKind::AllowanceHalted` (`run/webhook/mod.rs`, `operator_event_for_maps_each_notice_kind_to_its_event`); HMAC recomputed with `op-secret` over the captured raw bytes (`http_surface_tests.rs`) | closed |
| T-42-41 | Information Disclosure | operator payload contents | medium | mitigate | `allowance_halted_payload_has_the_same_twelve_keys` (`webhook/mod.rs`); "the same twelve keys on both events" end-to-end assertion; key NAMES and tenant ids only (D-00g); raw bytes asserted free of `op-secret` and key value | closed |
| T-42-42 | Tampering | notice SQL | high | mitigate | `NOTICE_INSERT`, `NOTICES_FOR_RUN`, `NOTICE_DELETE` static constants with bound parameters (`treasury/{sqlite,postgres}.rs`); arbiter tests tie the conflict target to migration 014 | closed |
| T-42-43 | Denial of Service | mixed-version replicas cannot parse `allowance_halted` | low | accept | rollout caveat documented in MIGRATION.md (`RunEventKind::AllowanceHalted` row, section 9.6) and `docs/src/api-reference/platform-api.md` — see AR-42-08 | closed |
| T-42-44 | Information Disclosure | herald halt line | medium | mitigate | scope kind, figures and window end only; `halt_reason_herald_line_names_no_tenant_or_key` (`crates/paladin-core/src/platform/container/allowance.rs`) | closed |
| T-42-45 | Repudiation | windows silently left open | low | mitigate | WINDOWS.md rows 65, 66, 67 with rationale (table and JSON block); ADR-0057 conformance re-read in 42-12 | closed |
| T-42-46 | Tampering | vocabulary guard that cannot fail | low | mitigate | `scanner_reports_a_planted_downstream_use` positive control (`tests/treasurer_vocabulary_guard.rs`), seen to report 0 of 4 against a stub scanner | closed |
| T-42-47 | Elevation of Privilege | coverage or PostgreSQL regressions not run locally | medium | transfer | CI `coverage` (82 % floor) and `postgres-integration` jobs in `.github/workflows/ci.yml` are authoritative; 42-12-SUMMARY names them as not run locally | closed |
| T-42-48 | Tampering | derived budget loosening or replacing another limit | medium | mitigate | tightest-wins rule only ever lowers the limit; `the_tightest_ceiling_binds_the_derived_budget` (`treasurer/tests.rs`); `operator_figure_above_the_derived_one_never_loosens_it` (`middleware/limits.rs`) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Tally:** 48 threats — 13 high, 18 medium, 17 low; 37 mitigate, 10 accept, 1 transfer; 48 closed, 0 open.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-42-01 | T-42-03, T-42-17, T-42-32 | D-01 (ADR-0057): the mid-run boundary check is a check-only balance read with no reservation, so runs or agent calls admitted in the same instant can all start; each halts at its first boundary, bounding the overshoot to one top-level superstep per run (WR-1). A per-superstep hold is the deferred mitigation. Recorded in WINDOWS.md row 65. | Operator at the 42-01 checkpoint (ADR-0057) | 2026-10-06 |
| AR-42-02 | T-42-07 | The guard adds no SQL; balance reads go through the unchanged Phase 41 adapters, which use bound parameters only. | Plan 42-02 threat model; 42-02-SUMMARY | 2026-10-06 |
| AR-42-03 | T-42-08 | At most four balance reads plus one store-clock read per boundary; D-02 locks no caching; migration 012's tenant index serves them. | Plan 42-02 threat model; 42-02-SUMMARY | 2026-10-06 |
| AR-42-04 | T-42-25 | `PerRunCancelProbe` ORs an in-memory flag read into the unchanged debounced DB probe; no new I/O per poll. | Plan 42-06 threat model; 42-06-SUMMARY | 2026-10-06 |
| AR-42-05 | T-42-26 | Shutdown-instant race (A7): the run row is the backstop truth and the degraded SSE path converges on it. | Plan 42-06 threat model; 42-06-SUMMARY | 2026-10-06 |
| AR-42-06 | T-42-30 | The operator `agent_runtime.token_budget` becomes effective on the HTTP agent routes; inert unless the operator enabled it; documented in configuration.md and CHANGELOG `### Changed`. | Plan 42-08 threat model; 42-08-SUMMARY | 2026-10-07 |
| AR-42-07 | T-42-36 | D-08: worker-dispatched agent-kind runs write no Waypoint, so a Treasurer-halted agent-kind run is resumed by a fresh `POST /v1/runs`, not a fork. WINDOWS.md row 64; MIGRATION.md and docs. | Plan 42-09 threat model; 42-09-SUMMARY; UAT D-08 wording override (`96bec0bb`) | 2026-10-07 |
| AR-42-08 | T-42-43 | Mixed-version replicas: an older build cannot parse the `allowance_halted` event value until all replicas upgrade; documented rollout caveat in MIGRATION.md 9.6 and platform-api.md (Phase 41 precedent). | Plan 42-11 threat model; 42-11-SUMMARY | 2026-10-07 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-07 | 48 | 48 | 0 | /gsd-secure-phase orchestrator (Claude Code), ASVS L1 grep-depth, tree at `e4cddd5f` |

### Audit notes (2026-10-07)

- All twelve PLAN files carried a `<threat_model>` block, so the register was authored at plan
  time; the auditor subagent was not spawned (short-circuit rule: `threats_open: 0`,
  plan-time register, ASVS L1).
- No SUMMARY raised a threat flag beyond its plan's register. Every SUMMARY's manual
  credential-handling review (security.instructions.md) reported no key value in any new log,
  error, body or payload.
- The Phase 42 code review's security-adjacent fixes landed before this audit and strengthen
  T-42-05 (WR-4 fail-closed on an unwritable charge), T-42-39 (WR-3 un-memoised failed notice
  claim), T-42-19/T-42-23 (WR-2/WR-5 replay scoping), T-42-16 (CR-1/IN-12 nested child halt) and
  T-42-11 (IN-6 unreadable reason reads as none).
- Known SAST gap unchanged: no merge-gating static taint analysis of first-party Rust
  (security.instructions.md); the manual review above remains the primary control.
- Not run in this audit: `make security` (cargo-audit, cargo-deny) and the full test suite were
  not re-executed here; the phase's own SUMMARYs and CI remain the authority for those, and the
  T-42-47 transfer covers the coverage and PostgreSQL legs.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-07
