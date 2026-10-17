# ADR-0002: BattalionResult field set

## Status

Superseded

Superseded by ADR-0059 on 2026-10-09 (Phase 44): `node_errors` now carries the structured `node_error::NodeError`; every other field of the set recorded here is unchanged; the rest of this record is history.

**Date:** 2026-07-31

## Context

Three positions competed for the `BattalionResult` shape: two producer variants (Epic 4 and Epic
5) and one Herald-consumer expectation (Epic 8) that neither producer variant supplied. Run-3
verification found the shipped struct at
`crates/paladin-core/src/platform/container/battalion/mod.rs:549` to be a merged **superset** of
all three, which is why this is a recording task and not a reconciliation task —
`intel/code-verification.md` states explicitly: "Do not plan a reconciliation task."

## Decision

- `crates/paladin-core/src/platform/container/battalion/mod.rs:549` is authoritative.
- The superset chose `per_paladin_times: HashMap<String, u64>` where Epic 5's position
  (`REQ-battalion-result-v2`) specified a top-level `execution_time_ms` field. `execution_time_ms`
  lost: it does not appear on the shipped struct, and its role — Battalion-level timing — is now
  served by summing or reading the per-Paladin breakdown that `per_paladin_times` carries directly.
- The superset chose `node_errors: Vec<NodeError>` where Epic 5's position specified
  `errors: Vec<PaladinError>`. The concrete reason: `BattalionError` derives only `Debug, Clone,
  thiserror::Error` (`battalion/mod.rs:759`) — no `Serialize`/`Deserialize` — while `BattalionResult`
  derives `Serialize, Deserialize` (`battalion/mod.rs:548`). A `Vec<PaladinError>` field could not
  be serialized without adding those derives to the error type, so the superset introduced a new
  plain-data struct, `NodeError { node_name: String, error: String }` (`battalion/mod.rs:538`),
  mirroring `TokenUsage`'s shape rather than reusing the error enum. Verified against the tree on
  2026-07-31: the derive claim holds exactly as stated.
- Epic 8's Herald expectation (`REQ-herald-battalion-result-fields`) is satisfied: the Battalion
  type is carried as `strategy_used: BattalionStrategy`, and aggregated token usage as
  `total_tokens: u64` plus `per_paladin_tokens: HashMap<String, TokenUsage>`.
- Epic 4's field set (`REQ-battalion-result-v1`) contributed no field the shipped struct dropped —
  every Epic-4 field (`battalion_id`, `battalion_name`, timestamps, `final_output`,
  `paladin_results`, `status`, per-Paladin and overall timing) is fully present in the superset, so
  there is no substitution to record for this position; it is wholly subsumed.
- This ADR does not decide *where* `PaladinResult`, `StopReason`, `TokenUsage`, `RegistryError` and
  `HandoffError` live — that is the subject of the Milestone 5 Epic 1 decision document
  (`battalion-result-upward-dependency-decision.md`), whose promotion is owned by Phase 7. That
  document settles the *location* of those five types only and, despite its filename, never mentions
  `BattalionResult` — so the two records do not contradict each other.

## Considered Options

- `REQ-battalion-result-v1` (Epic 4 FR-4.2) — superseded by the shipped superset; every field it
  specified (`battalion_id`, `battalion_name`, timestamps, `final_output`, `paladin_results`,
  `status`, per-Paladin and overall timing) is present in the merged struct.
- `REQ-battalion-result-v2` (Epic 5 FR-5) — superseded; present except `execution_time_ms`
  (displaced by `per_paladin_times`) and `errors: Vec<PaladinError>` (displaced by
  `node_errors: Vec<NodeError>`, for the serialization reason above). Its `metadata` map
  (`strategy_selection_reasoning`, `strategy_selection_time_ms`, `per_paladin_times`,
  `paladin_success_count`, `paladin_failure_count`, `timestamp`) was flattened into top-level
  `BattalionResult` fields rather than kept as a nested struct.
- `REQ-herald-battalion-result-fields` (Epic 8 FR-7) — satisfied, not rejected; the Battalion type
  and aggregated token usage it required are present as `strategy_used`, `total_tokens` and
  `per_paladin_tokens`.
- `REQ-battalion-metadata-extension` (run 2, Epic 22 FR-8) — satisfied by the same three fields; its
  `battalion/battalion_result.rs` module path did not ship (the fields live on `battalion/mod.rs`
  instead), and its `per_paladin_times: Vec<u64>` typing does not match the shipped
  `HashMap<String, u64>`, but the field's presence and name are what this ADR records as
  satisfied — the module-path and container-type mismatches are not separately actioned here.

## Code Locations

- `crates/paladin-core/src/platform/container/battalion/mod.rs:549` — the authoritative
  `BattalionResult` struct declaration.
- `crates/paladin-core/src/platform/container/battalion/mod.rs:497` — the aggregated `TokenUsage`
  struct that `per_paladin_tokens` and the Battalion-level token totals draw from.
- `crates/paladin-core/src/platform/container/herald.rs:49` — the `Herald` trait's
  `format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError>`, the
  consumer whose Epic 8 expectation this struct satisfies.

## Code Conformance

conforms

Phase 2 GAP-07 has no action for this decision — the shipped struct already satisfies all three
source positions, so no code change follows from this ADR.

## Downstream Consumers

- Phase 2 GAP-07 — no action required.
- Phase 7's ARCH-03(c) — the type-ownership question (where `PaladinResult`, `StopReason`,
  `TokenUsage`, `RegistryError` and `HandoffError` live) is decided there, not here.
- The four Battalion producers — Formation, Phalanx, Campaign, Chain of Command — which construct
  `BattalionResult` values.
- The Herald consumer (`crates/paladin-core/src/platform/container/herald.rs`), which formats
  `BattalionResult` via `format_battalion_result`.
