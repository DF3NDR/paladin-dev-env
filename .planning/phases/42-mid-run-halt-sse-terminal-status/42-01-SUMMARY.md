---
phase: 42-mid-run-halt-sse-terminal-status
plan: 01
status: in-progress
---

# Phase 42 Plan 01: Mid-run halt design gate and ADR-0057 Summary

## Checkpoint decision

**Selection: `option-b`.** Approve the consolidated design as proposed (all 16 items stand as written, no redirect) AND extend item 12 so that a true streamed `execute/stream` call's terminal `done` additionally carries an informational `halt_reason` object when the terminal chunk's usage crossed the derived figure. There is no behavioural halt on that path -- the call already finished -- and non-halt streams stay byte-identical.

Operator response, verbatim: `option-b`.

Meaning for item 12 and D-12: D-12's stream clause ("the agent stream's terminal `done` carries the same object") holds for BOTH the buffered fallback of `execute/stream` (whose `done` is the serialized `ExecuteResponse`, carrying `stop_reason: "allowance_halted"` and `halt_reason`) AND the true stream (informational crossing report only). ADR-0057 records that scope, and the G2 WINDOWS.md row still records that a single streamed call is not cut mid-flight.

Selected by the operator on 2026-10-06 at the plan 42-01 design gate presented by the execute-phase orchestrator.

Plans 42-07 and 42-08 (the derived budget, the agent routes and the streamed `done`) and 42-12 (the WINDOWS.md G2 row) read this recorded outcome; 42-02..42-12 read it through ADR-0057. This heading was written before any file in the plan's `files_modified` was touched.
