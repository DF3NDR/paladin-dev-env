# Phase 44: Legacy Clean-Break Removal - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-09
**Phase:** 44-legacy-clean-break-removal
**Areas discussed:** BattalionConfig after the cut, Continue-past-failure & node_errors, Conclave retry on the taxonomy, ADR-0059 & the register

---

## BattalionConfig after the cut

### What replaces timeout_seconds / retry_policy / error_strategy on BattalionConfig?

| Option | Description | Selected |
|--------|-------------|----------|
| aegis: Aegis field | One `#[serde(default)] aegis: Aegis` field; Formation/Phalanx/Campaign honour `aegis.timeout` and `aegis.retry`; one vocabulary across engine and legacy patterns | ✓ |
| Drop, no replacement | Config keeps name / description / metadata_output_dir; legacy patterns run untimed and fail-fast | |
| Timeout only, as Aegis TimeoutPolicy | `timeout: Option<TimeoutPolicy>`, no retry, no error strategy | |

**User's choice:** aegis: Aegis field (Recommended)

### If an Aegis timeout governs the legacy patterns, what does it bound?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-Paladin attempt | Engine semantics: `run_timeout` bounds each Paladin execution, `idle_timeout` degrades to a wall clock | ✓ |
| Whole battalion run | Today's semantics, sourced from `aegis.timeout.run_timeout` | |
| You decide | Planner picks after reading the superstep's TimeoutPolicy application | |

**User's choice:** Per-Paladin attempt (Recommended)

### Commander execute() timeout wrapper, ConclaveConfig::with_timeout and BattalionError::Timeout(u64) in scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, same treatment | Commander and Conclave move to the Aegis-sourced timeout; `BattalionError::Timeout(u64)` retired | ✓ |
| No, only Formation/Phalanx/Campaign | LEGACY-02 names three patterns; the rest stays | |
| You decide | Planner scopes from the dependency graph | |

**User's choice:** Yes, same treatment (Recommended)

### What feeds the Commander's ConclaveConfig / default ManeuverConfig bridging after the cut?

| Option | Description | Selected |
|--------|-------------|----------|
| Derive from aegis | `aegis.timeout.run_timeout` → Conclave/Maneuver timeout, `aegis.retry.max_attempts` → Conclave retry_attempts; Maneuver error_strategy falls back to its Default | ✓ |
| Drop the bridge | Conclave and Maneuver use their own Defaults unless set explicitly on the builder | |
| You decide | Planner keeps whichever keeps existing Commander tests meaningful | |

**User's choice:** Derive from aegis (Recommended)
**Notes:** User moved to the next area without follow-up questions.

---

## Continue-past-failure & node_errors

### How does a Formation/Phalanx/Campaign express "continue past a failed Paladin"?

| Option | Description | Selected |
|--------|-------------|----------|
| aegis.on_error Absorb | `Some(Absorb { .. })` = continue-and-collect, `None` = fail-fast; Route/Custom rejected on legacy patterns; RetryThenContinue = retry + Absorb | ✓ |
| continue_on_error: bool | A plain bool on BattalionConfig (the citadel BattalionCheckpointConfig shape) | |
| Gone: fail-fast only | Legacy patterns stop at the first failure | |

**User's choice:** aegis.on_error Absorb (Recommended)

### What does BattalionResult.node_errors become once battalion::NodeError is removed?

| Option | Description | Selected |
|--------|-------------|----------|
| Structured node_error::NodeError | Retype to `Vec<node_error::NodeError>` (node_id, attempt, transience, source); one NodeError in the tree | ✓ |
| Drop the field | Failures live only in failed PaladinResult entries / the returned error | |
| Rename the summary type | Keep `{ node_name, error }` under a new name | |

**User's choice:** Structured node_error::NodeError (Recommended)

### How should a run that continued past failures report its status?

| Option | Description | Selected |
|--------|-------------|----------|
| Completed + non-empty node_errors | Today's contract; no BattalionStatus change | ✓ |
| New PartialSuccess variant | New serialized enum variant, api-surface + §9.2 row | |
| You decide | Planner keeps whatever herald/Commander tests assert | |

**User's choice:** Completed + non-empty node_errors (Recommended)

### Herald / CLI rendering of the per-node error block after the retype?

| Option | Description | Selected |
|--------|-------------|----------|
| Same block, richer line | Keep the block; each line shows node id, attempt, transience plus Display text; JSON shape change noted in §9.1 | ✓ |
| Same block, same two fields | Project back to name + message so Markdown/Table output is byte-identical | |
| You decide | Planner chooses after reading the herald golden tests | |

**User's choice:** Same block, richer line (Recommended)
**Notes:** User moved to the next area without follow-up questions.

---

## Conclave retry on the taxonomy

### With the LlmError arm gone, what shape does Conclave's is_retryable_error take?

| Option | Description | Selected |
|--------|-------------|----------|
| Collapse to transience() | `error.transience() == Transient`, no per-variant arms; the table is the single source; any changed answer is a §9.1 row | ✓ |
| Keep explicit arms minus LlmError | Delete only the string-matching arm | |
| You decide | Planner diffs arms against the table | |

**User's choice:** Collapse to transience() (Recommended)

### Should Transience::Unknown be retried by the Conclave?

| Option | Description | Selected |
|--------|-------------|----------|
| No, Transient only | Parity with the engine's TransientOnly default | ✓ |
| Yes, Transient + Unknown | FallbackLlmAdapter precedent | |
| You decide | Planner checks existing Conclave retry tests | |

**User's choice:** No, Transient only (Recommended)

### Reuse the engine's backoff instead of Conclave's private calculate_retry_delay?

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse engine retry::backoff_delay | Aegis RetryPolicy via BattalionConfig.aegis + `backoff_delay` / `wait_backoff`; retry_attempts stays as the count | ✓ |
| Keep the private helper | Only the predicate changes | |
| You decide | Planner picks the lower-risk path | |

**User's choice:** Reuse engine retry::backoff_delay (Recommended)

### PaladinError::is_retryable() / is_terminal() in scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Out of scope, keep | Only the LlmError line inside them is removed; circuit breaker contract stays | |
| Remove them too | Delete both, move circuit_breaker.rs to transience(), two §9.2 rows | ✓ |
| Defer to a later phase | Keep now, record as deferred | |

**User's choice:** Remove them too — the user chose the wider blast radius over the recommended narrow scope.

### After circuit_breaker.rs moves to transience(), which failures count toward tripping it?

| Option | Description | Selected |
|--------|-------------|----------|
| Transient only | Same rule as the Conclave and the engine; ExecutionError (Unknown) stops counting; §9.1 row | ✓ |
| Transient + Unknown | Preserves today's answer for ExecutionError | |
| You decide | Planner keeps what circuit_breaker.rs tests encode | |

**User's choice:** Transient only (Recommended)
**Notes:** Assumption stated and not contested: the sibling `HandoffError` / `PromptError` / `PlanningError::LlmError(String)` variants stay untouched.

---

## ADR-0059 & the register

### What does ADR-0059 cover?

| Option | Description | Selected |
|--------|-------------|----------|
| Supersession + design in one ADR | Records the Phase 44-only X-03 supersession and fixes the replacement design | ✓ |
| Two ADRs: 0059 supersession, 0060 design | Separate policy and design records | |
| Supersession-only ADR | Design lives in CONTEXT.md and plans | |

**User's choice:** Supersession + design in one ADR (Recommended)

### ADR-0001 and ADR-0002: what happens to them?

| Option | Description | Selected |
|--------|-------------|----------|
| Mark Superseded | Status → "Superseded by ADR-0059" with a dated note, per PROMOTION.md | ✓ |
| Leave as-is, cite from 0059 | Status stays Accepted | |
| You decide | Planner follows PROMOTION.md | |

**User's choice:** Mark Superseded (Recommended)

### How are the MIGRATION.md §9.2 rows cut?

| Option | Description | Selected |
|--------|-------------|----------|
| One row per crate\|type pair | Matches the allowlist's set-equality key; every fired lint gets its own [[entry]] under the row | ✓ |
| One row per removed item | A row per type, field, variant and method | |
| You decide | Planner mirrors the 43-13 granularity | |

**User's choice:** One row per crate|type pair (Recommended)

### How deep do the example and mdBook rewrites go?

| Option | Description | Selected |
|--------|-------------|----------|
| Show the Aegis equivalents | Examples and pages demonstrate `aegis: Aegis { retry, timeout, on_error }`; examples double as the downstream migration guide | ✓ |
| Delete the legacy lines only | Remove what no longer compiles | |
| You decide | Rewrite where the example's purpose was the removed feature | |

**User's choice:** Show the Aegis equivalents (Recommended)
**Notes:** User declared the area done and chose "I'm ready for context".

---

## Pre-discussion

### Fold the matched todo into this phase?

| Option | Description | Selected |
|--------|-------------|----------|
| Leave it out | Coverage-reproduction walkthrough is a user-owned carried item unrelated to the legacy removal | ✓ |
| Fold it in | Add the local coverage reproduction check to Phase 44 | |

**User's choice:** Leave it out (Recommended)

## Claude's Discretion

- Where `Route` / `Custom` rejection lives and what `Absorb`'s delta means for a string pipeline.
- Paladin-name → `NodeId` mapping and the `NodeErrorSource` for a Paladin failure.
- The structured variant an Aegis per-attempt timeout surfaces as on the legacy patterns.
- Row wording, empirical lint ids, whether a separate `ConclaveConfig` row is needed.
- Plan/wave split and test-helper fallout.

## Deferred Ideas

- Sibling `HandoffError` / `PromptError` / `PlanningError::LlmError(String)` variants (untouched).
- `BattalionStatus::PartialSuccess` (considered, rejected for this phase).
- Reviewed todo not folded: `2026-08-13-verify-local-coverage-reproduction.md`.
