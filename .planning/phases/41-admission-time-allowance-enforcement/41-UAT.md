---
status: testing
phase: 41-admission-time-allowance-enforcement
source: [41-VERIFICATION.md]
started: 2026-10-04T12:45:00Z
updated: 2026-10-05T15:20:00Z
---

## Current Test

number: 1
name: Operator UAT walkthrough (41-VALIDATION manual-only row)
expected: |
  With `treasurer.allowance.api_keys.ci-runner { period: 1h, amount: 2.50 }` and the key's
  ledger balance at 2.5000 USD, the next `POST /v1/runs` answers 429 with
  `error.code == "allowance_exhausted"`, `Retry-After` equals the seconds left in the current
  UTC hour, `error.details` names balance / ceiling / window_start / window_end, and
  `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same
  2.5000 USD figure. Then set an 80% warn crossing with a webhook target and confirm one
  operator POST (twelve-key payload, option-b), one trace event and one herald line, and
  nothing on a second admission in the same window.
awaiting: user response

## Tests

### 1. Operator UAT walkthrough (41-VALIDATION manual-only row)
expected: With `treasurer.allowance.api_keys.ci-runner { period: 1h, amount: 2.50 }` and the key's ledger balance at 2.5000 USD, the next `POST /v1/runs` answers 429 `allowance_exhausted` with a `Retry-After` equal to the seconds left in the current UTC hour and `details` naming balance / ceiling / window_start / window_end; `paladin-cli treasury spend --api-key ci-runner --since <window_start>` reports the same 2.5000 USD. With an 80% warn crossing and a webhook target: exactly one operator POST (twelve keys), one trace event and one herald line, and nothing on a second admission in the same window. Why human: needs a running paladin-server, a real operator config and a receiver; the in-process tracers prove the same chain over on-disk SQLite but nobody has driven the real binary and CLI.
result: [pending]
observed: |
  Recorded 2026-10-05 by Claude Code for operator review; `result` is left for the operator.
  Every box of the revised 41-UAT-RUNBOOK.md §4 and §5 was observed to hold. Details under
  "Observations (2026-10-05)" below.

### 2. CI-authoritative gates: coverage (>= 82% workspace line coverage, ADR-0006) and postgres-integration (no SKIP: lines)
expected: Both jobs green on the pushed branch `claude/laughing-dirac-e0h2ax`. The PostgreSQL legs for balance, treasury_notices (migrations 011/012), run_schedules.created_by (010) and the operator webhook delivery row execute rather than skip. Why human: PostgreSQL is not running in the verifier's environment, so env-gated tests skip there; the SUMMARYs of 41-02, 41-05, 41-06 and 41-08 record real runs on a throwaway cluster (32, 14, 147 and 11 passed, 0 SKIP). The coverage figure is only computed in CI.
result: [pending]
observed: |
  Recorded 2026-10-05 by Claude Code for operator review. CI run 37202056513 on `b7f934e3` (the
  pushed branch head): `Coverage` job succeeded, "Lines: 124226/136525 = 90.99%" (floor 82%);
  `Postgres Storage Contract Suites (live server)` succeeded, 157 passed, 0 failed, 0 ignored, and
  its own "no SKIP: line" guard step passed. This run predates the trace wiring fix below, which is
  not yet committed or pushed.

### 3. Accepted over-admission race between two admissions in the same instant (41-02 backstop truth)
expected: Confirm ADR-0056 records the race and its closure by Phase 42's reservation at the superstep boundary, and accept it as a known property of check-only admission (D-05). Why human: cannot be proven or prevented mechanically under D-05; the Treasurer proofs show admission is read-only and concurrency-safe, but two simultaneous admissions can both pass.
result: [pending]

### 4. Backstop truths: crash between a won notice claim and the run insert (41-06) and `RunWorkerPool::with_treasury_notices` inside `build_run_api` (41-08)
expected: Accept that a crash in the window between a won notice claim and the run insert can lose one window's notice but never duplicates it or persists a run, and that the production-builder attachment of the notice store to the worker pool has no observable output of its own. The worker tests and the integrated tracer prove the same code path with the same builder call. Why human: both are `verification: backstop` truths (non-inferable by construction) and so abstain rather than pass.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps

- truth: "`trace.persist: true` in the operator config persists run traces from paladin-server"
  status: fixed (commit 6e0bbc58)
  test: 1
  detail: |
    `build_run_api` never handed `settings.trace` or a trace store to the run worker pool, so the
    server ran on `TraceConfig::default()` and `run_traces` stayed empty. Fixed in
    `src/infrastructure/web/run_api_wiring.rs` (`with_trace_config`, plus `with_run_trace_port`
    when `trace.persist` is set) with two wiring tests; CHANGELOG entry added. The walkthrough
    below ran on a binary built with this fix.
- truth: "a persisted trace record can be read back through `RunTracePort::read`"
  status: fixed
  test: 1
  detail: |
    Not a Phase 41 defect, found during this UAT. A `run_started` record of a Platform API run
    serialises `run_id` twice (the `TraceRecord` envelope field and the flattened
    `TraceEvent::RunStarted.run_id`), and deserialising it fails with "duplicate field `run_id`".
    The row is written and readable with `sqlite3`; `RunTracePort::read` (used by
    `paladin-cli run` trace reads) errors on such a thread. Latent since Phase 28 because the
    server never persisted traces before the fix above.
    Fixed 2026-10-05 in `crates/paladin-core/src/platform/container/trace.rs`: the record writes
    one `run_id` key and reads rows with the doubled key; a contract test covers every trace
    store backend.

## Observations (2026-10-05)

Environment: branch `claude/laughing-dirac-e0h2ax` at `b7f934e3` plus the uncommitted trace wiring
fix; `paladin-server` built `--release --features web-server,llm-ollama`, `paladin-cli` built
`--release --features cli`; SQLite run and waypoint stores; Ollama 0.35.1 serving `qwen2.5:0.5b`
on CPU; pretend price 1000.00 USD per 1M tokens (the revised runbook now recommends 100.00, which
was not itself run). Window: 2026-10-05T15:00:00Z to 16:00:00Z.

Half A, refusal (ci-runner, 2.50 USD per 1h, ledger seeded to 2500000000 nanos):

- `paladin-cli treasury spend --api-key ci-runner --since 2026-10-05T15:00:00Z --group-by api-key`
  reported `ci-runner  2.5000 USD  1` settlement.
- `POST /v1/runs` at 15:04:24Z answered `HTTP/1.1 429 Too Many Requests`, `retry-after: 3336`
  (expected 3336 from the wall clock), body:
  `{"error":{"code":"allowance_exhausted","details":{"balance":"2.5000 USD","ceiling":"2.5000 USD","kind":"window","scope":"api_key","window_end":"2026-10-05T16:00:00Z","window_start":"2026-10-05T15:00:00Z"},"message":"api_key window allowance exhausted: balance 2.5000 USD has reached the ceiling 2.5000 USD (window 2026-10-05T15:00:00Z to 2026-10-05T16:00:00Z)"}}`
- The response contains neither `acme`, `ci-runner` nor the key value.
- Nothing persisted: `GET /v1/runs` listed 0 items and `select count(*) from runs` returned 0.
- Control: `svc-control` (same tenant, no allowance) answered 202 and completed, cost 0.1800 USD,
  settled under `svc-control`; the `ci-runner` balance stayed 2500000000.

Half B, warn path (warn-key, 1.00 USD per 1h, ledger seeded to 800000000 nanos):

- First submit answered 202; run `01a10c98-708f-70c1-bedf-6e3af0f44996` completed, cost 0.1740 USD.
- `treasury_notices`: exactly one row, `api_key|window|800000000|1000000000|80|<that run id>`.
- Receiver: exactly one delivery, twelve keys (`api_key_id, balance, ceiling, event, kind,
  run_id, scope, tenant_id, timestamp, warn_at, window_end, window_start`),
  `event: allowance_warning`, `tenant_id: acme`, `api_key_id: warn-key`, `run_id` the run above.
  `x-paladin-event: allowance_warning`; `x-paladin-signature`
  `sha256=fc40ccbe1e5cb66e04cd6869cb35ec8edbfd2aead30494aca484eae5449372c8` equals the HMAC-SHA256
  computed over the captured body with `op-secret-41`. The body holds no key value, run input or
  secret. `GET /v1/runs/{id}/webhook-deliveries` for the caller returned `{"items":[]}`.
- Trace: one `"kind":"allowance_warning"` line in the log; `run_traces` rows for the thread are
  `1 allowance_warning, 2 run_started, 3 node_started, 4 node_finished, 5 run_finished`.
- Herald: exactly one line,
  `"allowance_warning":"⚠ allowance: 80% of 1.0000 USD (api_key, window resets 2026-10-05T16:00:00Z)"`,
  with no tenant id and no key name.
- Second admission, same window: 202, completed, cost 0.1780 USD. Still one notice row (none for
  the second run), one delivery, one `allowance_warning` trace line; the second run's trace rows
  are `run_started, node_started, node_finished, run_finished`; its herald summary has no
  `allowance:` line.
- Third submit (balance 1152000000 nanos): 429, `balance "1.1520 USD"`, `ceiling "1.0000 USD"`.

Runbook defects found and corrected in 41-UAT-RUNBOOK.md: missing `APP_WAYPOINT_STORE_*`
variables (server would not boot); `llm.ollama` block without the required `api_key`; no `herald:`
section (no herald line); `warn-key` never defined; `"input":{}` sends the literal `{}` as the
task; the poll loop ignored `halted`/`cancelled`; the `"allowance_warning"` log pattern also
matches the herald line; the receiver could overwrite two deliveries arriving in one second.
