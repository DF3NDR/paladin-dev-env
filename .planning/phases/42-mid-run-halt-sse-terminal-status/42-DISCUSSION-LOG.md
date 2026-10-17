# Phase 42: Mid-Run Halt & SSE Terminal Status - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-06
**Phase:** 42-mid-run-halt-sse-terminal-status
**Areas discussed:** Superstep boundary check, Halt and resume contract, Agent-loop budget derivation, SSE terminal payload, Mid-run warnings and halt notices
**Mode:** interactive (default), five areas selected by the operator, seventeen single-question turns, the recommended option chosen in every turn.

---

## Superstep boundary check

### Q1. What does the Treasurer do at each superstep boundary to decide whether the next draw would overspend?

| Option | Selected |
|--------|----------|
| Check-only balance read (Recommended) | ✓ |
| Reserve a hold per superstep |  |
| Check-only now, hold for concurrency later |  |

**User's choice:** Check-only balance read (Recommended)
**Notes:** Check-only balance read: reuse Phase 41's `balance` read and ceiling order; halt when balance >= ceiling before the next superstep starts. No `reserve` rows; settlement stays exactly as 39-04 wrote it. Overshoot bounded to one superstep's spend (same posture as TokenBudget's one-response overshoot).

---

## Halt and resume contract

### Q1. How does a Treasurer-halted run resume once the allowance is replenished or the window resets?

| Option | Selected |
|--------|----------|
| New run from the halted waypoint (Recommended) | ✓ |
| Re-enqueue the same run id |  |
| New run, plus automatic resubmission |  |

**User's choice:** New run from the halted waypoint (Recommended)
**Notes:** New run from the halted waypoint (recommended): a resume starts a NEW run on the same thread from the halted waypoint through the existing fork lifecycle, which already gates with admit_and_persist (admission re-checked, 429 while still exhausted). Halted stays terminal; no state-machine change; the resumed run records its origin waypoint (fork_of).

### Q2. Where does the Treasurer reason for a halt live on the persisted run, so a client that was not streaming can learn why it stopped?

| Option | Selected |
|--------|----------|
| Typed `halt_reason` column (Recommended) | ✓ |
| Reuse the `error` text column |  |
| Trace and events only |  |

**User's choice:** Typed `halt_reason` column (Recommended)
**Notes:** Typed `halt_reason` column (recommended): nullable column via migration 013 on both backends holding the serialized AllowanceRefusal figures (scope, kind, balance, ceiling, window); GET /runs/{id} gains an optional `halt_reason` object with the same shape as the 429 `details`; `error` stays None because a halt is not a failure.

### Q3. What happens when the superstep-boundary balance read itself fails (ledger backend down) for a run whose principal has a configured ceiling?

| Option | Selected |
|--------|----------|
| Fail closed: halt (Recommended) | ✓ |
| Fail open: log and continue |  |
| Bounded retries, then halt |  |

**User's choice:** Fail closed: halt (Recommended)
**Notes:** Fail closed: halt (recommended). A failed boundary balance read for a principal with a configured ceiling halts the run with a distinct reason (`ledger_unavailable`) and keeps the checkpoint, mirroring Phase 41 D-10 (a ceiling that cannot be evaluated cannot be passed). A principal with no ceiling never reads the ledger and is unaffected. Resumable via fork once the ledger is back.

### Q4. How often does the Treasurer read the ledger during a run?

| Option | Selected |
|--------|----------|
| Every boundary (Recommended) | ✓ |
| Debounced like the cancellation probe |  |
| Every boundary, with a local short-circuit |  |

**User's choice:** Every boundary (Recommended)
**Notes:** Every boundary (recommended): one `balance` read per applicable ceiling at every superstep boundary, no caching; a halt lands at the first boundary after the ceiling is reached. Cost: one indexed SUM per ceiling (up to four) per superstep, on the same path the per-superstep settle already takes.

---

## Agent-loop budget derivation

### Q1. How is the remaining allowance (currency) turned into the per-run `max_tokens` the agent-loop `TokenBudget` enforces?

| Option | Selected |
|--------|----------|
| Dearest axis of the model's row (Recommended) | ✓ |
| Completion price |  |
| Observed blend, re-derived per call |  |

**User's choice:** Dearest axis of the model's row (Recommended)
**Notes:** Dearest axis of the model's row (recommended): max_tokens = floor(remaining_nanos / max(prompt, completion, cache_write, reasoning) per-token price) for the agent's configured model. Assumes every token is the priciest kind, so the derived budget can never let the run overspend the remaining allowance; stops early on prompt-heavy runs; simple to state in docs.

### Q2. What happens when a principal with a configured allowance runs an agent whose model has no price row, so no `max_tokens` can be derived?

| Option | Selected |
|--------|----------|
| Refuse at admission (Recommended) | ✓ |
| Skip derivation, run unmetered |  |
| Reject at boot where possible |  |

**User's choice:** Refuse at admission (Recommended)
**Notes:** Refuse at admission (recommended): a principal with a configured ceiling whose agent model has no `treasurer.pricing` row is refused with a typed error, a new refusal kind beside `allowance_exhausted` (429-shaped or 422, planner's call). An allowance that cannot be metered is a config incoherence caught at the first call rather than silently unbounded. Principals with no ceiling are unaffected; Phase 38 D-08 warn-once stays.

### Q3. How does the Treasurer-derived budget combine with an operator-configured `agent_runtime.token_budget` on the same run?

| Option | Selected |
|--------|----------|
| Tightest wins, one budget (Recommended) | ✓ |
| Two budgets side by side |  |
| Treasurer budget replaces the operator one |  |

**User's choice:** Tightest wins, one budget (Recommended)
**Notes:** Tightest wins, one budget (recommended): one TokenBudget per run whose max_tokens is the smaller of the Treasurer-derived figure and the operator's agent_runtime.token_budget.max_tokens (when enabled). The stop reason records which one won: a Treasurer win maps to a halt, an operator win keeps today's token_budget stop. One middleware, one cutoff, one overshoot rule.

### Q4. How does the agent loop surface a Treasurer cutoff so the run ends `Halted` with a typed reason rather than `Completed`?

| Option | Selected |
|--------|----------|
| New typed stop reason (Recommended) | ✓ |
| Reuse `TokenBudget`, infer at the worker |  |
| Typed error, run Failed |  |

**User's choice:** New typed stop reason (Recommended)
**Notes:** New typed stop reason (recommended): a StopReason variant for a Treasurer cutoff carrying the AllowanceRefusal figures, classed neither successful nor a failure. The worker maps an agent-kind run with it to status Halted plus the halt_reason column; POST /agents/{id}/execute answers 200 with stop_reason "allowance_halted" and a halt_reason object (the 429 details shape); the stream's done carries the same. Partial output is kept, as TokenBudget does today.

---

## SSE terminal payload

### Q1. What does the SSE `done` event carry for a Treasurer halt versus a caller cancel?

| Option | Selected |
|--------|----------|
| Status plus the reason object (Recommended) | ✓ |
| Status only, reason via GET /runs |  |
| Distinct status strings, no object |  |

**User's choice:** Status plus the reason object (Recommended)
**Notes:** Status plus the reason object (recommended): done becomes {status: "halted", waypoint_id, halt_reason: {...}} for a Treasurer halt and {status: "cancelled", waypoint_id} for a caller cancel. halt_reason is the same object GET /runs/{id} returns (the 429 details shape), so a streaming client needs no follow-up read. Plain halted with no reason stays for the bare in-process token halt.

### Q2. How does the terminal event learn whether a halt was a caller cancel, a Treasurer spend halt, or something else?

| Option | Selected |
|--------|----------|
| Typed cause on the engine outcome (Recommended) | ✓ |
| Worker publishes done from persisted status |  |
| Bus sink consults the repository |  |

**User's choice:** Typed cause on the engine outcome (Recommended)
**Notes:** Typed cause on the engine outcome (recommended): RunOutcome::Halted gains a typed cause (caller cancel from the probe, Treasurer spend halt with its figures, ledger unavailable, bare token halt), carried into RunFinished as RunFinishStatus::Cancelled and Halted { reason }. The engine's terminal trace event is correct at the source; the worker's map_outcome reads the cause instead of re-querying the cancel flag; the live done and the persisted halt_reason come from one value. Additive under #[non_exhaustive], #[serde(default)] for stored traces.

### Q3. With the cause typed, what should a stream see when a worker drains on shutdown and requeues the run (today it receives done: halted although the run is still Running)?

| Option | Selected |
|--------|----------|
| No `done` on a drain (Recommended) | ✓ |
| Out of scope, record the gap |  |
| New `requeued` status on `done` |  |

**User's choice:** No `done` on a drain (Recommended)
**Notes:** No done on a drain (recommended): a halt whose cause is the worker's own shutdown token is not terminal. The engine's RunFinished for that cause maps to no done on the bus, the run stays Running and is requeued as today, and a connected stream falls through to the degraded polling path on reconnect. Fixed at the same code region PLAT-09 touches; the typed cause makes it a one-arm change.

### Q4. How is a ledger-unavailable halt reported on the stream, and does the degraded polling path carry the same halt_reason as the live path?

| Option | Selected |
|--------|----------|
| `done` with a reason kind, both paths (Recommended) | ✓ |
| `error` for ledger outages |  |
| Live path only |  |

**User's choice:** `done` with a reason kind, both paths (Recommended)
**Notes:** done with a reason kind, both paths (recommended): a ledger-unavailable halt is a resume point like a spend halt: done with status "halted" and halt_reason { kind: "ledger_unavailable", ... }, no figures; error stays reserved for Failed. The degraded polling path builds the same done from the run row's status and halt_reason, so both stream modes agree byte-for-byte on the terminal payload.

---

## Mid-run warnings and halt notices

### Q1. Should the superstep-boundary check also evaluate the warn threshold, so a run that crosses it mid-flight warns before it halts?

| Option | Selected |
|--------|----------|
| Warn at the boundary too (Recommended) | ✓ |
| Admission-only warnings |  |
| Trace and herald only mid-run |  |

**User's choice:** Warn at the boundary too (Recommended)
**Notes:** Warn at the boundary too (recommended): the superstep-boundary check runs the same ceiling evaluation as admission, so a run that crosses warn_at mid-flight claims the once-per-window notice (store-deduped, Phase 41 D-16) and emits the one AllowanceWarning trace event, herald line and operator webhook. Long runs no longer wait for the next admission to warn; one function per rule.

### Q2. Does a Treasurer halt notify the operator beyond the caller's existing Halted run webhook?

| Option | Selected |
|--------|----------|
| Operator webhook, once per window (Recommended) | ✓ |
| Caller webhook and trace only |  |
| Operator webhook per halted run |  |

**User's choice:** Operator webhook, once per window (Recommended)
**Notes:** Operator webhook, once per window (recommended): a spend halt also enqueues the Phase 41 operator webhook target with a second event, allowance_halted, carrying the same twelve-key shape as allowance_warning plus the halted run_id. Deduped once per window per scope and ceiling through the same notices table (a new limit-kind rung), so twenty runs halting in one window produce one operator notice. The caller's own Halted run webhook still fires per run.

### Q3. How does the halt reason reach the caller's webhook and the run's trace and herald output?

| Option | Selected |
|--------|----------|
| Reason on webhook, trace and herald (Recommended) | ✓ |
| Trace and herald only; webhook unchanged |  |
| Webhook and trace only; no herald line |  |

**User's choice:** Reason on webhook, trace and herald (Recommended)
**Notes:** Reason on webhook, trace and herald (recommended): the caller's Halted run webhook payload gains an optional halt_reason key (the same object as GET /runs/{id} and the SSE done), documented as an additive key under WebhookPayload's key-set discipline; signing unchanged. The RunFinished { Halted { reason } } trace event is folded by HeraldTraceSink into one herald line beside the cost line, rendered by all three heralds.

### Q4. How are this phase's decisions recorded for later phases to cite?

| Option | Selected |
|--------|----------|
| One ADR, written first (Recommended) | ✓ |
| Two ADRs: halt contract, terminal status |  |
| Amend ADR-0052 and ADR-0056 |  |

**User's choice:** One ADR, written first (Recommended)
**Notes:** One ADR, written first (recommended): ADR-0057 'Mid-run halt contract' (next free number per PROMOTION.md, which advances to 0058 in the same commit), recording the check-only boundary, fail-closed ledger posture, typed halt cause and halt_reason column, fork-as-resume, dearest-axis budget derivation and tightest-wins composition, the typed agent stop reason, and the halt notice rung. It cites ADR-0052, ADR-0053 and ADR-0056 rather than re-opening them, and is the phase's opening plan so later plans cite it.

---

## Claude's Discretion

- Port, type and variant names for the boundary guard port, the halt cause, the `halt_reason` encoding and the new `StopReason` variant.
- How the derived budget reaches the per-call `TokenBudget` (recommended: a `RunScope` field read from the `ModelCallContext` scratch).
- Whether `POST /agents/{id}/jobs` (41-RESEARCH C4) is gated with admission and the derived budget in this phase (recommended: yes) or recorded in WINDOWS.md.
- The worker-side attachment of the guard port, the notice-rung encoding for `allowance_halted`, the agent-stream `done` shape, the test topology, and a log line for `ledger_unavailable` halts.

## Deferred Ideas

- Per-superstep reservation hold (closes the Phase 41 D-05 over-admission race) — declined for v0.11.0.
- Automatic resubmission of Treasurer-halted threads when the window resets or the allowance is raised.
- A `requeued` wire status for drained runs; a first-class `POST /runs/{id}/resume` with a `Halted → Queued` edge.
- Re-deriving the agent budget per model call from the observed token blend.
- Carried from Phase 41: warn-threshold ladder, caller-facing allowance warning webhook, `paladin-cli treasury allowance` view, thread-route read scoping.

---

*Phase: 42-mid-run-halt-sse-terminal-status*
*Discussion log generated: 2026-10-06*
