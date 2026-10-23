---
phase: 51-adjustable-reasoning-effort
plan: 16
subsystem: web / agent host
tags: [reasoning-effort, fallback-precedence, gap-closure, G-2a, D-09]
requires: [51-08]
provides:
  - "check_block_reasoning resolves the fallback agent-first (agent, then block, then Refuse), equal to paladin_llm::reasoning::in_force"
affects: [src/infrastructure/web/agent_host.rs]
key-files:
  modified:
    - src/infrastructure/web/agent_host.rs
decisions:
  - "Fallback precedence at host load is field by field: def.reasoning_fallback.or(block.fallback).unwrap_or_default()"
metrics:
  tasks: 2
  files: 1
completed: 2026-10-23
status: complete
---

# Phase 51 Plan 16: Host-load fallback precedence Summary

The agent host's load-time check of a provider block's default reasoning effort now resolves the fallback agent-first (agent's, then block's, then `Refuse`), the same order `paladin_llm::reasoning::in_force` applies on every call.

## What changed

- `check_block_reasoning` (`src/infrastructure/web/agent_host.rs`): the inverted `block.fallback.or(def.reasoning_fallback)` became `def.reasoning_fallback.or(block.fallback)`, then `unwrap_or_default()`. The expression is split over two `let` bindings so rustfmt keeps `def.reasoning_fallback.or(block.fallback)` on one line. The hint condition needed no edit because it reads the in-force fallback.
- Rustdoc for `BlockReasoning.fallback` (the agent's fallback wins over it), `check_block_reasoning` (cites `in_force` as the precedence it mirrors and `EffortSupport::resolve` as the admission rule) and `build_agent_with_llm_and_block_temperature`.
- Tests in `config_driven_tests`: `block_nearest_lower_agent_refuse_is_refused`, `block_refuse_agent_nearest_lower_passes`, `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`, `the_host_fallback_hint_appears_only_when_refuse_is_in_force`.

## RED evidence (Task 1, unmodified expression)

All three precedence tests failed before the fix:
- `block_nearest_lower_agent_refuse_is_refused`: panicked "the agent's refuse beats the block's nearest_lower, so medium is refused" (the build succeeded).
- `block_refuse_agent_nearest_lower_passes`: panicked "the agent's nearest_lower substitutes the block's medium with low" (the build was refused).
- `block_nearest_lower_agent_nearest_higher_takes_the_agents_direction`: the agent's `NearestHigher` half failed ("the agent's nearest_higher steps medium up to high").

After the fix the three new tests and the four 51-08 effort tests pass (7/7).

## Commits

- 5e88990c: fix(51-16): host-load effort check takes the agent's fallback before the block's (G-2a, D-09)
- 2439bee0: docs(51-16): host check precedence matches in_force; pin the fallback hint

## Verification

- `cargo test -p paladin-ai --lib --all-features -- the_host_fallback_hint_appears_only_when_refuse_is_in_force infrastructure::web::agent_host`: 44 passed.
- `cargo clippy -p paladin-ai --all-targets --all-features -- -D warnings` and `cargo fmt --all --check`: clean.
- `make api-surface`: unchanged (4323 items).
- Tracer re-verify: the tracer's `<verify>` command passed (7/7) before the commit and the module suite passed after; the executor expanded to Task 2.
- Not run: `make security` / `cargo test` across the whole workspace (scoped to the touched crate per the thermal cap). The orchestrator runs the full hook chain after the wave merges.
- Manual credential-handling review: the refusal text is built from agent id, model, provider name, effort, fixed refusal wording and the YAML key name only. The tests assert the block's `SECRET` and `agents[a1]` are absent from the refusal.

## Deviations from Plan

None - plan executed as written. The session crashed once from CPU overheating mid-task; the uncommitted test edit survived and execution resumed with `CARGO_BUILD_JOBS=4` and no concurrent cargo runs.

## Known Stubs

None.

## Threat Flags

None. No new network, auth or schema surface.

## Self-Check: PASSED

- src/infrastructure/web/agent_host.rs modified and committed (5e88990c, 2439bee0).
- Acceptance greps: one `def.reasoning_fallback.or(block.fallback)` line inside `check_block_reasoning`; `wins over it` present; `in_force` appears 6 times.
