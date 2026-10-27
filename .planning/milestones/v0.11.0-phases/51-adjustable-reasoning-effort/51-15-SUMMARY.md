---
phase: 51-adjustable-reasoning-effort
plan: 15
subsystem: run-worker
tags: [reasoning, trace, gap-closure, G-1, REASON-05, D-11]
requires: ["51-07", "51-09"]
provides:
  - "single-agent NodeFinished carries reasoning_requested / reasoning_effective from the call's own run_metadata"
affects: [run_traces, SSE node_finished, OTel trace export]
tech-stack:
  added: []
  patterns: ["read-only-from-the-result trace stamping, same rule as AttemptReasoning::from_run_metadata"]
key-files:
  created: []
  modified:
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
decisions:
  - "Fields are copied byte for byte from result.run_metadata under the two core constants; never parsed, never read from PaladinData"
metrics:
  completed: 2026-10-23
requirements: [REASON-05]
status: complete
---

# Phase 51 Plan 15: Single-agent trace carries the call's effort Summary

The `/v1/agents` run path (`RunWorkerPool::run_agent`) now emits `NodeFinished` with `reasoning_requested` / `reasoning_effective` copied verbatim from the call's own `PaladinResult.run_metadata` (`paladin.reasoning_requested` / `paladin.reasoning_effective`), closing verification gap G-1 (REVIEW WR-02).

## What changed

- `worker.rs`: imports the two core metadata-key constants; the `Ok(result)` arm reads them with `result.run_metadata.get(KEY).cloned()` (borrow-and-clone, before `result.output` is moved). The `Err(error)` arm keeps both fields `None`. `run_agent` rustdoc gains one sentence on the fields' source.
- `worker_tests.rs`: helper `agent_pool_with_traces`, doubles `ReasoningStampingPaladinPort` and `ConfiguredEffortAgentResolver`, readers `read_agent_trace` / `node_finished_reasoning` / `metadata`, and four tests.

## Tests

| Test | Role |
|------|------|
| `agent_run_node_finished_carries_the_calls_own_reasoning_metadata` | tracer, red first; `xhigh`/`high`, `high`/`high`, `budget:2048`/`budget:2048` verbatim; seqs `[1,2,3,4]` |
| `agent_run_node_finished_reads_only_the_two_reasoning_keys` | `(Some("high"), None)`; `paladin.served_by` never reaches the trace |
| `agent_run_node_finished_never_reads_the_agents_configured_effort` | configured `High` + `NearestLower`, unstamped call: `(None, None)` |
| `failed_agent_run_node_finished_carries_no_reasoning` | `Failed` outcome, `(None, None)`, `AGENT_FAILURE_TEXT` in no row |

**RED evidence:** on the unmodified `worker.rs`, `agent_run_node_finished_carries_the_calls_own_reasoning_metadata` failed at `worker_tests.rs:1551` with ``assertion `left == right` failed: requested  left: None  right: Some("xhigh")``.

**Tracer re-verify (auto mode):** after the fix the tracer plus `agent_kind_run_streams_done_live` and `agent_kind_run_emits_exactly_one_terminal_event` passed (3/3); the tracer was verified end-to-end before expanding.

## Verification

- `cargo test --offline -p paladin-ai --lib --all-features -- agent_run_node_finished failed_agent_run_node_finished_carries_no_reasoning`: 4 passed.
- `... -- application::services::run`: 261 passed, 0 failed (no `cancel_tests` flake seen).
- `cargo fmt --all` applied; `cargo clippy --offline -p paladin-ai --all-targets --all-features -- -D warnings`: clean.
- `make api-surface`: unchanged (4323 items).
- Manual credential-handling review (`security.instructions.md`): only two adapter-written ASCII effort spellings reach the trace; no key, header, prompt or response body is read; the failure arm is unchanged and its text reaches no trace row (test-pinned).
- `grep -c 'reasoning_requested: None' worker.rs` prints `1` (the `Err` arm only).

## Deviations from Plan

- **[Rule 1 - Bug] `NodeOutcomeKind` is not `Copy`.** The first draft of `node_finished_reasoning` used `*outcome`; changed to `outcome.clone()`. Test-only.
- **[Rule 1 - Lint] clippy `field_reassign_with_default`** in `ConfiguredEffortAgentResolver`; switched to struct-update syntax. Test-only.
- **Acceptance grep form.** The criterion `grep -n 'result.run_metadata.get(REASONING_REQUESTED_METADATA_KEY)'` expects a single-line expression; rustfmt (chain-width limit) splits it as `result\n.run_metadata\n.get(KEY)\n.cloned()`, so the literal one-line grep does not match. The behaviour is identical and `.get(REASONING_REQUESTED_METADATA_KEY)` / `.get(REASONING_EFFECTIVE_METADATA_KEY)` each appear once.
- **Execution environment.** The session crashed from CPU overheating; the resumed run capped every cargo call with `CARGO_BUILD_JOBS=4` (the `nice` wrapper was blocked by the harness; the coordinator authorized the plain env prefix). Task commits used `--no-verify` as sanctioned for this wave.

## Known Stubs

None.

## Threat Flags

None. T-51-43 (repudiation) and T-51-44 (information disclosure) are mitigated and test-pinned; no new surface.

## Commits

- bdfa704a: fix(51-15): single-agent NodeFinished carries the call's requested and effective effort (G-1, D-11)
- 1a9e4bca: test(51-15): agent-run trace reasoning guards (D-11)

## Self-Check: PASSED
