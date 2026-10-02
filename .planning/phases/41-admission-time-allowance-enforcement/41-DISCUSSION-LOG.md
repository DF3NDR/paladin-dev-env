# Phase 41: Admission-Time Allowance Enforcement - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-02
**Phase:** 41-admission-time-allowance-enforcement
**Areas discussed:** Window semantics & allowance shape, Admission check & coverage, Refusal
contract, Warn notice delivery & dedup

---

## Window semantics & allowance shape

### Q1 — Window shape

| Option | Description | Selected |
|--------|-------------|----------|
| Tumbling, period-aligned (Recommended) | Fixed slots aligned to the UTC epoch computed from the store clock; stable `(scope, window_start)` key for once-per-window; predictable reset; 2x burst across a boundary | ✓ |
| Trailing sliding window | `[store_now - period, store_now)`; smoothest limiting, most literal "rolling", but no stable window key and no reset instant to report | |
| Calendar-aligned | `hourly/daily/weekly/monthly` anchored to UTC calendar boundaries; variable month length, enum grammar | |

**User's choice:** Tumbling, period-aligned.

### Q2 — Config grammar

| Option | Description | Selected |
|--------|-------------|----------|
| Duration string + decimal amount (Recommended) | `period: "24h"`/`"7d"`, `amount: "25.00"` parsed by the same exact-integer grammar as prices | ✓ |
| Enum period + decimal amount | `hourly \| daily \| weekly \| monthly`; `monthly` is calendar-shaped, `weekly` needs an anchor | |
| Integer seconds + integer nano-units | `period_secs`, `amount_nanos`; zero parsing but no operator writes nano-units | |

**User's choice:** Duration string + decimal amount.

### Q3 — Composition of tenant/key, window/lifetime

| Option | Description | Selected |
|--------|-------------|----------|
| All configured limits must fit (Recommended) | Up to four ceilings; any exhausted refuses and is named; unconfigured scope is unlimited; lifetime is `[epoch, +inf)` | ✓ |
| Most specific wins | Key allowance, if present, is the only one checked; defeats per-tenant governance | |
| Tenant window only, key lifetime only | Fewer keys but narrows ALLOW-01 | |

**User's choice:** All configured limits must fit.

### Q4 — Tenant-wide balance source

| Option | Description | Selected |
|--------|-------------|----------|
| Add one read method to the port (Recommended) | Additive `balance(scope filter, window)` on `TreasuryLedgerPort`, all three adapters, contract suite; Phase 42 reuses | ✓ |
| Reuse `spend()` as-is | Sums settled `charged_nanos` only; ignores reservations once Phase 42 places them | |
| Let the planner decide | Researcher confirms whether `spend()` suffices for the settle-only ledger | |

**User's choice:** Add one read method to the port.
**Notes:** Continuation check answered "Next area".

---

## Admission check & coverage

### Q5 — Check vs hold

| Option | Description | Selected |
|--------|-------------|----------|
| Check only, no hold (Recommended) | Read balance, refuse when `balance >= ceiling`; no ledger write; Phase 42 owns reservations | ✓ |
| Reserve a configurable minimum hold | Race-proof today but pulls Phase 42's settle/release attachment forward | |
| Zero-amount reserve for the serialized SUM | Writes a zero row per admission; covers only the (tenant, key) pair | |

**User's choice:** Check only, no hold.

### Q6 — Which paths are gated

| Option | Description | Selected |
|--------|-------------|----------|
| Every principal-bearing path (Recommended) | `POST /runs`, fork, and `/agents/{id}/execute[/stream]`; one shared function; `requested_by: None` never gated | ✓ |
| RunSubmissionService only | Leave agent execute ungated, record a WINDOWS.md row | |
| Every path including schedules | Also give schedule-fired runs the creator's principal (migration) | |

**User's choice:** Every principal-bearing path.

### Q7 — Schedule-fired runs

| Option | Description | Selected |
|--------|-------------|----------|
| Stay ungated, record the gap (Recommended) | No schedule change; WINDOWS.md row; creator-principal stays deferred | |
| Fold creator-principal into this phase | `created_by` on the schedule row (new migration), stamped from the creating Principal; fired runs gated | ✓ |
| Refuse to create a schedule when auth is on | Fail-closed 501-style; removes a working feature | |

**User's choice:** Fold creator-principal into this phase (departure from the recommendation).

### Q8 — Admin bypass

| Option | Description | Selected |
|--------|-------------|----------|
| No bypass: allowances bind every principal (Recommended) | Admin key with an entry is refused like any other; unlimited = no entry | ✓ |
| Admin bypasses allowances | Mirrors Phase 40 D-11; an Admin key becomes an unbounded spend path | |

**User's choice:** No bypass.
**Notes:** Continuation check answered "Next area".

---

## Refusal contract

### Q9 — HTTP status

| Option | Description | Selected |
|--------|-------------|----------|
| 429 Too Many Requests (Recommended) | Quota semantics clients back off from; helper exists; `Retry-After` to window reset; body `code` distinguishes from the rate limiter | ✓ |
| 402 Payment Required | Semantically exact but no helper, proxy/SDK handling uneven, reads as billing | |
| 403 Forbidden | Reuses `Forbidden`; conflates authorization with budget; clients will not retry | |

**User's choice:** 429 Too Many Requests.

### Q10 — Error body

| Option | Description | Selected |
|--------|-------------|----------|
| Figures + scope kind + reset time (Recommended) | `code: allowance_exhausted`, details with scope, kind, balance, ceiling, window bounds; never the key value | ✓ |
| Message only | Leaks nothing but the caller cannot tell when to retry | |
| Figures, no window | Simpler DTO, weaker contract | |

**User's choice:** Figures + scope kind + reset time.

### Q11 — Error type home

| Option | Description | Selected |
|--------|-------------|----------|
| One core refusal value, wrapped per port (Recommended) | `AllowanceRefusal` in `paladin-core`; `RunSubmissionError::AllowanceExhausted(..)`; agent path maps the same value through one shared helper; trace carries it | ✓ |
| Separate variants per error enum | Two definitions to keep aligned | |

**User's choice:** One core refusal value, wrapped per port.

### Q12 — Ledger outage at admission

| Option | Description | Selected |
|--------|-------------|----------|
| Fail closed when an allowance applies (Recommended) | `Backend` → 500, no run persisted; principals with no allowance are never consulted | ✓ |
| Fail open with an error-level log | Matches 39 D-08's settle posture but turns a DB blip into unbounded spend | |

**User's choice:** Fail closed when an allowance applies.
**Notes:** Continuation check answered "Next area".

---

## Warn notice delivery & dedup

### Q13 — Threshold shape

| Option | Description | Selected |
|--------|-------------|----------|
| One `warn_at` percent per allowance, global default (Recommended) | `treasurer.allowance.warn_at: 80`, per-entry override, `0` disables; integer percent compared in nano-units | ✓ |
| A ladder of thresholds | `[50, 80, 95]`; once per rung; ALLOW-04 reads singular | |

**User's choice:** One `warn_at` percent, global default.

### Q14 — Webhook target

| Option | Description | Selected |
|--------|-------------|----------|
| Operator-level URL via the existing delivery service (Recommended) | `treasurer.allowance.webhook { url, secret? }`, SSRF at boot and send, durable row, HMAC, no-redirect, new `allowance_warning` payload | ✓ |
| The admitted run's own caller webhook | New `RunEventKind::AllowanceWarning`; audience is the tenant, not the operator | |
| Both | Operator URL always plus the run's webhook when subscribed; two payload shapes | |

**User's choice:** Operator-level URL via the existing delivery service.

### Q15 — Once-per-window dedup

| Option | Description | Selected |
|--------|-------------|----------|
| Store-backed, new notices table (Recommended) | `treasury_notices` with UNIQUE index + `INSERT … ON CONFLICT DO NOTHING`; winner emits; holds across replicas and restarts | ✓ |
| In-process memory | `HashSet` on the facade; per-process only, re-fires on restart | |

**User's choice:** Store-backed notices table.

### Q16 — Herald notice

| Option | Description | Selected |
|--------|-------------|----------|
| New trace event, rendered by the heralds (Recommended) | `TraceEvent::AllowanceWarning` on the admitted run's stream; `HeraldTraceSink` folds it into `ExecutionMetadata.metadata`; one line in all three heralds | ✓ |
| Log line only, no herald change | Cheapest but ALLOW-04 names the herald | |

**User's choice:** New trace event, rendered by the heralds.
**Notes:** Continuation check answered "Wrap up"; closing check answered "I'm ready for context".

---

## Claude's Discretion

- Exact type/module names and whether core types share `treasury_ledger.rs` or a new module.
- Deterministic ceiling evaluation order; short-circuit on first refusal while still evaluating
  every warn crossing on admit.
- Required vs defaulted `balance` trait method.
- `period` grammar edges (seconds for tests, maximum, non-divisor periods).
- Trace-emission mechanics on the submit path (emit at admission vs worker reads the notices row).
- How schedule attribution travels through `SubmitRun`; `010` column names; principal-less
  `create_schedule`.
- Whether a refused admission logs (recommended yes, no key value) and/or traces (recommended no).
- Transactionality between the notices insert and the webhook enqueue.
- Test topology; in-memory store-clock simulation; migration ordering `010`/`011`.
- The boot-time cross-check of allowance ids against the key mapping was offered as a
  "more questions" candidate and not taken; it is recorded as decision D-11 (fail closed) with
  the reasoning stated in CONTEXT.md.

## Deferred Ideas

- Warn-threshold ladder.
- Notice on the admitted run's own caller webhook (`RunEventKind::AllowanceWarning`).
- `paladin-cli treasury allowance` remaining-allowance view.
- Mid-run hold/halt, `TokenBudget` derivation, SSE `done` status (Phase 42); rate pacing
  (Phase 43); Treasurer mdBook page (Phase 46).
- `PATCH /schedules/{id}` re-assigning `created_by`; backfilling pre-existing schedules.
- Tenant registry, per-API-key read narrowing (FUT-05); thread-route scoping; FX (FUT-12).
- Reviewed todo not folded: `2026-08-13-verify-local-coverage-reproduction.md` (score 0.2).
