---
phase: 39
slug: spend-ledger
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-09-28
verified: 2026-09-28
register_authored_at_plan_time: true
---

# Phase 39 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register consolidated from the `<threat_model>` blocks of all eight PLAN.md files (39-01..39-08);
> the same `T-39-NN` id appears in several plans where a threat recurs per component, and is
> listed once below with every component it was mitigated on. Verified by `/gsd-secure-phase 39`
> at ASVS L1 (grep-depth: each control located in source, each named test located in the tree,
> each test confirmed green by `39-VERIFICATION.md`'s live run on this tree).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Operator shell → `paladin-cli treasury spend` → ledger store | Local operator command opening the SQLite file / Postgres URL from `RunStoreConfig` directly, no server | Database path / URL (may embed a password); spend rows (amounts, opaque scope labels) |
| Engine / agent loop → `TreasuryLedgerPort` | In-process settle writers (`SpendHook::settle_boundary`, `settle_agent_loop_call`) | run id, superstep, attempt, `Cost`, per-model breakdown; `LedgerScope` (the `unattributed` sentinel this phase) |
| `paladin-server` process → Postgres / SQLite | `sqlx` pool built from a URL or path; embedded `007` migrator on open | Connection credential in the URL; every ledger statement (bound parameters only) |
| HTTP client → `GET /v1/runs`, `GET /v1/runs/{id}`, `POST /v1/agents/{id}/execute` | Authenticated run-read / agent-execute routes now carrying a ledger-derived `cost` | `CostDto` (display string + nanos + currency); never a scope label, never a credential |
| CI runner → live Postgres (`postgres-integration`) | Docker-gated contract suite; sandbox runs used a local `pg_ctlcluster` cluster instead | Test URL from `STORAGE_POSTGRES_TEST_URL`; the suite's `SKIP:` line when unreachable |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-39-01 | Tampering (SQL injection) | SQLite + Postgres ledger queries; CLI | high | mitigate | Every statement is a `&'static str` const with `?` / `$N` binds (advisory-lock key is a bound value); `model_breakdown` via `serde_json`; grep for `format!("SELECT/INSERT/UPDATE/DELETE` over both adapters returns 0 | closed |
| T-39-02 | Tampering (overspend via check-then-act race) | `reserve` on SQLite / in-memory / Postgres | high | mitigate | SUM + INSERT inside one `begin_with("BEGIN IMMEDIATE")` (sqlite.rs:247); `pg_advisory_xact_lock(hashtext($1)::bigint)` before the SUM (postgres.rs:64); one `tokio::sync::Mutex` per method body (in-memory); `reserve_race_admits_exactly_n_minus_one` (16-way, on-disk WAL) green on all three | closed |
| T-39-03 | Repudiation (double charge / missed re-spend) | `settle` on all adapters; worker attempt capture | high | mitigate | Partial unique index `(run_id, superstep, attempt) WHERE kind = 'settle'` in both `007` migrations + `INSERT … ON CONFLICT … DO NOTHING` with a textually matching predicate (`settle_arbiter_predicate_matches_the_migration` on both SQL adapters); `HashSet<SettlementKey>` in-memory; worker passes the persisted attempt (`redelivered_running_run_settles_under_the_bumped_attempt`); `duplicate_settle_is_already_settled_and_charges_once`, `bumped_attempt_is_a_distinct_settlement`, 10-way `concurrent_duplicate_settles_charge_once` green | closed |
| T-39-04 | Information Disclosure (DB URL / password in errors) | adapter + CLI + `build_treasury_ledger` error text | medium | mitigate | `redact_database_url_password(&err.to_string(), database_url)` is the first handling of every connection error (sqlite.rs:196, postgres.rs:188), no truncation before it; `connection_error_redacts_password_from_database_url` in both adapters; `build_treasury_ledger` errors name the env var / path only | closed |
| T-39-05 | Tampering (audit-trail mutation) | all three adapters | medium | mitigate | Append-only: grep for `UPDATE` / `DELETE` statements over `treasury/{sqlite,postgres,in_memory}.rs` returns 0; prohibition recorded in plans 39-02/39-03 | closed |
| T-39-06 | Denial of Service (ledger outage breaks runs / agents / run reads / boot) | `SpendHook::settle_boundary`, `settle_agent_loop_call`, `run_costs`, worker | high | mitigate | Settle writers never propagate: `Err` → `log::error!`, `AlreadySettled` → `log::warn!` (settlement.rs:199-232); `failing_treasury_ledger_never_fails_the_run`, `ledger_failure_never_fails_the_agent_loop`; HTTP degrades to `cost: null` (`get_run_cost_is_null_when_the_ledger_errors`); only a configured backend's construction failure fails boot, as the run repository already does | closed |
| T-39-07 | Denial of Service (SQLite write-lock starvation) | SQLite reserve/settle/release transactions | medium | mitigate | Transaction body limited to the listed statements with no foreign `.await` while open (module doc sqlite.rs:25); race clause runs under a 10 s timeout | closed |
| T-39-08 | Information Disclosure (spend of other callers' runs) | `GET /runs*` `cost`, `ExecuteResponse.cost`, `RunApiState` ledger | medium | accept | `cost` rides the existing authenticated run-read / agent-execute authorization; per-principal run-read scoping is PLAT-07, Phase 40 (see Accepted Risks R-39-01) | closed |
| T-39-09 | Information Disclosure (scope labels / content in logs or rows) | settle log lines; `LedgerScope` labels | low | mitigate (+ accept on rows) | Settle logs interpolate only run id, superstep/ordinal, attempt, nanos, currency, error — grep for `api_key_id` / `tenant_id` inside any log macro in settlement.rs and paladin_execution_service.rs returns 0; row labels are the documented opaque `unattributed` sentinel (R-39-02) | closed |
| T-39-10 | Denial of Service (CLI spend over a huge window) | `paladin-cli treasury spend` | low | accept | Operator-invoked, local file access only, fold bounded by settle rows in the window, no network surface (R-39-03) | closed |
| T-39-11 | Tampering (currency confusion) | spend fold; reserve/settle; superstep total; `CostDto` | medium | mitigate | Grouped by (group, currency), never summed across codes (`treasury_spend_prints_one_row_per_currency`); foreign-currency probe inside the serialized transaction → `CurrencyMismatch` (`reserve_refuses_a_second_currency_in_scope_and_window`); `Cost::checked_add` mismatch → no row + error log; mixed-currency run → `cost: null` (`get_run_cost_is_null_for_mixed_currencies`) | closed |
| T-39-12 | Spoofing (spend attributed to another run) | `RunScope.run_id`; `run_agent` RunScope | medium | mitigate | No HTTP DTO maps to `RunScope` (grep over `crates/paladin-web/src` non-test code returns 0); only the worker sets `run_id`, from the repository's own `Run` row (`agent_kind_run_passes_its_run_id_in_the_run_scope`); `PlatformRunsOnly` settles nothing without it | closed |
| T-39-13 | Repudiation (double count / lost spend across engine and agent-loop writers) | superstep accumulation; shared engine service | high | mitigate | Per-attempt accumulator drained at each superstep boundary, never the whole-run tally (`one_settlement_per_superstep_attempt_across_parallel_nodes_and_retries`, `treasury_ledger_settles_once_per_superstep_with_model_breakdown`); child hooks never settle (`child_settle_boundary_makes_no_ledger_call_and_leaves_the_accumulator_intact`, `nested_battalion_child_spend_rolls_into_the_parent_superstep`); engine's shared service in `PlatformRunsOnly` (`platform_runs_only_settles_only_scoped_runs`, `engine_execution_port_forwards_the_run_scope_to_the_ledger`) | closed |
| T-39-14 | Denial of Service (advisory-lock key collision) | `hashtext` 32-bit lock key | low | accept | A collision only serializes two unrelated scopes; documented in postgres.rs module header (ADR-0053 §5 chose no lock table) (R-39-04) | closed |
| T-39-15 | Repudiation (false assurance from a skipped suite / unmeasured coverage) | Postgres contract suite; coverage | medium | mitigate | `store_or_skip()` prints a named `SKIP:` reason (postgres.rs:712/721) instead of passing silently; 39-03 and 39-08 SUMMARYs state the live local run; `39-VERIFICATION.md` independently re-ran it live (23 passed, 0 `SKIP:`); coverage attributed to CI's `coverage` job, never claimed locally | closed |
| T-39-16 | Denial of Service (mutex held across await) | `SpendHook` | medium | mitigate | `std::sync::Mutex` guard scoped to push/take only, dropped before `ledger.settle(..).await`; poison recovered with `PoisonError::into_inner` (settlement.rs:174/193) | closed |
| T-39-17 | Denial of Service (N+1 ledger queries) | `list_runs` | low | mitigate | One `spend()` call per page (`list_runs_derives_costs_with_one_spend_call_per_page` counts calls) | closed |
| T-39-18 | Repudiation (unrecorded public-API break) | MIGRATION.md §9.2 / semver allowlist | medium | mitigate | Empirical `cargo semver-checks` against 0.9.0 (all 11 CI packages) and 0.10.1 (6 changed packages); every reported lint registered; `./scripts/check-migration-allowlist.sh` + `make check-gates` exit 0 (39-08 SUMMARY D1–D3) | closed |
| T-39-19 | Tampering (public-surface drift) | `.project/current-exports.txt` | medium | mitigate | `make api-surface-update` with the CI-pinned `nightly-2026-09-20` in the same commit as the CHANGELOG entries; `make api-surface` exits 0 (39-08 SUMMARY D5) | closed |
| T-39-20 | Elevation / Tampering (vulnerable dependency) | dependency graph | low | mitigate | `make security` passed on the final tree (9 pre-existing allowlisted advisories, 0 new); `git diff --stat 15be9f79 HEAD -- Cargo.lock` is empty — the phase added no crate | closed |
| T-39-21 | Information Disclosure (credential leakage in the phase diff) | full Phase 39 diff | medium | mitigate | Manual credential-handling review per `security.instructions.md` recorded in 39-08 SUMMARY (redact-first on both SQL adapters, no scope identity in settle logs, no new `reqwest::Client`); re-confirmed by the operator as UAT test 3 in `39-UAT.md` | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-39-01 | T-39-08 | Run cost is exposed only on routes already gated by the run-read / agent-execute authorization; per-principal scoping of run reads is a separate requirement (PLAT-07) scheduled for Phase 40, which will scope the `cost` field along with every other run-read field. Accepted in plans 39-06 and 39-07. | operator (plan-time disposition, 39-06/39-07) | 2026-09-27 |
| R-39-02 | T-39-09 (rows) | `tenant_id` / `api_key_id` on ledger rows are opaque labels — the `unattributed` sentinel in this phase — never a secret; Phase 40 replaces the label's source, not the column. Documented on `LedgerScope`. | operator (plan-time disposition, 39-01) | 2026-09-27 |
| R-39-03 | T-39-10 | `treasury spend` is an operator-invoked local read with no network surface; the fold is bounded by the rows in the requested window. | operator (plan-time disposition, 39-01) | 2026-09-27 |
| R-39-04 | T-39-14 | A 32-bit `hashtext` collision merely serializes two unrelated scopes' reserves behind one advisory lock (a liveness cost, never a correctness one); ADR-0053 §5 chose this over a dedicated lock table. Documented in the adapter's module header. | operator (plan-time disposition, 39-03) | 2026-09-27 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-28 | 21 | 21 | 0 | /gsd-secure-phase 39 (orchestrator, ASVS L1 short-circuit — register authored at plan time, no open threats, no auditor spawn required) |

**Evidence basis.** L1 verification located every mitigate control in source by grep (statement
constants and binds, `ON CONFLICT` arbiter and matching migration predicate, `BEGIN IMMEDIATE`,
`pg_advisory_xact_lock`, redaction-first `wrap`, `PoisonError::into_inner`, log-macro contents,
absence of `UPDATE`/`DELETE`, absence of any `RunScope` in HTTP DTO code, empty `Cargo.lock`
diff) and every named test in the tree; test greenness rests on `39-VERIFICATION.md`'s live run
on this same tree (storage 62/62 + Postgres 23/23 with 0 `SKIP:`, engine 561/561, execution
service 71/71, worker 32/32, tracer_e2e 5/5, paladin-web 233/233). Human-judgment rows
(T-39-21 credential review) were additionally confirmed by the operator in `39-UAT.md`.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-28
