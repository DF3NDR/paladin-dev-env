# Phase 39: Spend Ledger - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-27
**Phase:** 39-spend-ledger
**Mode:** `--auto` — every question below was answered by selecting the recommended option
without an operator in the loop. `[auto]` lines are the audit record.
**Areas discussed:** Scope identity before Phase 40, Per-model spend under superstep-aggregate
settlement, Port API shape, Idempotency and attempt counter, Production settle writer, Query
surface (CLI / HTTP), Contract suite and migration shape, Per-backend serialization

**Pre-flight:** no `.continue-here.md` (no blocking anti-patterns); no SPEC.md; no prior
CONTEXT.md, checkpoint or plans for Phase 39; no spike/sketch findings; `discuss:pre` and
`discuss:post` hook lists empty; advisor mode off (no USER-PROFILE.md).

---

## Scope identity before Phase 40

| Option | Description | Selected |
|--------|-------------|----------|
| Final schema now, sentinel scope until Phase 40 | `tenant_id`/`api_key_id` columns and a `LedgerScope` parameter from day one; the Phase 39 writer stamps `unattributed` until Phase 40 supplies the run's principal | ✓ |
| Nullable scope columns | Leave scope NULL until Phase 40 and backfill | |
| Defer production writes to Phase 41 | Ship port/adapters only; ledger stays empty in production | |

`[auto] Scope identity — Q: "How do ledger rows carry tenant/API key before Principal has a tenant?" → Selected: "Final schema now, sentinel scope until Phase 40" (recommended default)`
**Notes:** Rejected NULL scopes because the balance SUM and the LEDGR-04 grouping must never see
an ambiguous scope; rejected deferring writes because roadmap SC4 needs real rows for the CLI view.

---

## Per-model spend under superstep-aggregate settlement

| Option | Description | Selected |
|--------|-------------|----------|
| `model_breakdown` JSON column on the settlement row | One row per settlement key (ADR-0053 intact); per-model map folded in Rust | ✓ |
| Child `settlement_lines` table | Relational per-model lines; second table in `007` | |
| Derive per-model view from `run_traces` | Reuse `NodeFinished.cost`; depends on opt-in trace persistence | |
| Single nullable `model` column | Lossy for mixed-model supersteps | |

`[auto] Per-model spend — Q: "How does the CLI answer 'spend per model' when one settlement covers several nodes?" → Selected: "model_breakdown JSON column" (recommended default)`

---

## Port API shape

| Option | Description | Selected |
|--------|-------------|----------|
| Policy-free port: caller supplies `ceiling` and window bounds; store-clock method; typed `Refused` error; duplicate settle is `AlreadySettled` success | Mirrors `RunRepositoryPort`/X-06; Phase 41 owns allowance policy | ✓ |
| Port reads allowance config itself | Couples storage to the `treasurer:` config section | |
| `reserve` returns `Ok(bool)` | Bare bool outcome (X-06 anti-pattern) | |

`[auto] Port API — Q: "Where does the ceiling come from and how is a refusal reported?" → Selected: "Policy-free port with typed Refused" (recommended default)`
`[auto] Port API — Q: "Does the port expose the store clock?" → Selected: "Yes, store_now()" (recommended default, ALLOW-01)`

---

## Idempotency and the attempt counter

| Option | Description | Selected |
|--------|-------------|----------|
| Store-enforced partial unique index + `ON CONFLICT DO NOTHING`; engine `attempt` = persisted `runs.attempt` | DB guarantees LEDGR-03; resolves ADR-0053's open item with an existing counter | ✓ |
| Application-level check-then-insert | Race window between check and insert | |
| New engine-owned attempt counter | Invents a second counter beside `Run.attempt` | |

`[auto] Idempotency — Q: "Where is duplicate settlement rejected?" → Selected: "Partial unique index in the store" (recommended default)`
`[auto] Attempt counter — Q: "Which counter is the engine-path attempt?" → Selected: "runs.attempt (D-23)" (recommended default)`

---

## Production settle writer

| Option | Description | Selected |
|--------|-------------|----------|
| Settle-only writer at the ADR-0052 superstep boundary (engine) and after each priced call (agent loop), synchronous, failures logged not fatal | Ledger fills in production; halt/reserve attach in the same place later | ✓ |
| Settle from a `TraceSink` on `RunFinished`/`NodeFinished` | Reuses `HeraldTraceSink` shape but the channel is drop-oldest | |
| No production writer in Phase 39 | Leaves LEDGR-04's CLI view empty | |

`[auto] Settle writer — Q: "What writes ledger rows in ordinary operation during Phase 39?" → Selected: "Synchronous settle-only writer at the superstep boundary" (recommended default)`

---

## Query surface (CLI / HTTP)

| Option | Description | Selected |
|--------|-------------|----------|
| `paladin-cli treasury spend` reading the store via `RunStoreConfig`; `GET /runs*` `cost` derived from the ledger; agent response exposes `PaladinResult.cost` | Follows `run export` precedent; one source of truth | ✓ |
| CLI calls a new HTTP spend endpoint | Requires a running server and an out-of-scope endpoint | |
| Persist cost columns on `runs` | Duplicates the ledger; second source of truth | |

`[auto] Query surface — Q: "How does the operator view spend from the CLI?" → Selected: "Direct store read via RunStoreConfig" (recommended default)`
`[auto] Query surface — Q: "Where does GET /runs/{id} get its cost?" → Selected: "Derived from the ledger" (recommended default)`

---

## Contract suite and migration shape

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror `crate::run` per-clause generic functions; `treasury::{postgres}` module naming for CI discovery; `007_create_treasury_ledger_table.sql` in `006` header style | House pattern, zero CI edits | ✓ |
| Declarative macro suite | Failures name a line, not a clause (rejected by the run suite's own rationale) | |

`[auto] Contract suite — Q: "Suite shape?" → Selected: "Per-clause generic functions mirroring crate::run" (recommended default)`

---

## Per-backend serialization

| Option | Description | Selected |
|--------|-------------|----------|
| Postgres `pg_advisory_xact_lock(hashtext(scope))`; SQLite `BEGIN IMMEDIATE`; in-memory single mutex | ADR-0053 §5, no lock table | ✓ |
| Postgres per-scope lock row with `SELECT … FOR UPDATE` | Needs a second table and an upsert | |

`[auto] Serialization — Q: "Which Postgres lock?" → Selected: "Transaction-scoped advisory lock" (recommended default)`

---

## Claude's Discretion

Column list beyond the fixed quantities; engine-side aggregation mechanism at the boundary;
SQL-vs-Rust grouping split; pagination/row cap for `spend`; `CostDto` field names; fixture
shapes; in-memory clock simulation.

## Deferred Ideas

Reservation placement and ceilings (41/42); real scope from the run's principal (40); HTTP spend
endpoint/dashboard (out of scope); ledger retention; `SpendSettled` trace event (rejected as
duplication); FX (FUT-12). RustFS todo reviewed and not folded (Phase 45 scope; auto-fold rule
deliberately not applied — recorded in CONTEXT.md).
