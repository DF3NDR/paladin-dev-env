# ADR-0052: Mid-run Treasurer enforcement attachment point

## Status

Accepted

**Date:** 2026-09-25

## Context

Paladin has two production run paths, and both ultimately reach an LLM through the same
`Arc<dyn LlmPort>` resolution.

The **engine path**: `WarEngine`'s superstep loop dispatches a `NodeSpec::Paladin` node through
`PaladinPort::execute_scoped`, which the `EngineExecutionPort` adapter
(`src/infrastructure/web/facade_provisioner.rs`) forwards to a single `PaladinExecutionService`
built once at boot by `paladin_port_from_settings` and shared by every concurrent run the engine
executes — there is no per-run instance on this path.

The **agent loop**: `build_agent`/`build_agent_with_llm`
(`src/infrastructure/web/agent_host.rs`) construct a `PaladinExecutionService` per agent, whose
`execute`/`execute_stream` methods back the HTTP agent routes.

Both paths resolve their `Arc<dyn LlmPort>` through the identical `LlmProviderFactory::create`
call site (research Pattern 3), which is why the pricing decorator (D-09) can be installed once
and reach both.

**Zero production callers, stated plainly.** A full-tree grep confirms `AgentRuntimeConfig::
build_chain` has **zero production callers** — only its own unit tests in `src/config/
agent_runtime.rs` and one `examples/agent_runtime_middleware.rs` file call it — and
`Settings.agent_runtime` is read nowhere in production outside `agent_runtime.rs`/`settings.rs`
themselves (research Pitfall 6). The `TokenBudget` `after_model` cutoff
(`StopReason::TokenBudget`, `src/application/services/paladin/middleware/limits.rs`) is
code-complete and unit-tested (12+ tests, including one asserting `is_successful()` is true for a
budget stop), but it is wired into **no production run today, on either path**. Phase 42 is the
first phase to wire it — this ADR does not describe that cutoff as already live in production; it
describes the mechanism this phase's ADRs and Phase 42's plans build on.

Milestone 14 now builds code under the `Treasurer` name ADR-0050 reserved — see that ADR's own
dated note below for the first code symbols.

## Decision

**Metering.** Per-call cost is computed by a pricing decorator (`PricingLlmAdapter`,
`crates/paladin-llm/src/pricing.rs`, built by Phase 38 plan 38-02) wrapping `Arc<dyn LlmPort>` at
the two `LlmProviderFactory::create` call sites, always composed OUTSIDE any `FallbackLlmAdapter`
so the response that gets priced is the one actually served (D-09). The per-call `Cost` rides
beside `TokenUsage` from `LlmResponse` through `PaladinResult` and `TraceEvent::NodeFinished` to
`TraceEvent::RunFinished` (D-10). Streams are priced from the terminal chunk's usage against the
request's model string, the only model identity a stream carries.

**Engine-path halt.** Phase 42 raises the halt at the `WarEngine` superstep boundary — the point
where the existing cancellation path (`crates/paladin-battalion/src/engine/superstep.rs`) already
writes a `WaypointStatus::Halted` Waypoint and returns `RunOutcome::Halted`, which
`run_finish_status` (`crates/paladin-battalion/src/engine/mod.rs`) maps to `RunFinishStatus::
Halted` and which the worker in turn maps to `RunStatus::Halted`. This is a consistent restart
point `resume` already continues from. `AgentRuntimeConfig::build_chain` stays unwired for the
engine path.

**Agent-loop halt.** Phase 42 reuses the existing `TokenBudget` `after_model` cutoff
(`StopReason::TokenBudget`, `src/application/services/paladin/middleware/limits.rs`), with the
Treasurer deriving the per-run budget value from the remaining allowance (ALLOW-05). Phase 42
performs the FIRST production wiring of that middleware on the agent loop.

## Considered Options

- **Chosen split**: metering at the `LlmPort` boundary on both paths (D-09), halt raised where
  each path can checkpoint — the engine's superstep boundary for `WarEngine`, the existing
  `TokenBudget` `after_model` cutoff for the agent loop.
- **Wire `build_chain` into the engine path** (rejected): a `TokenBudget` stop inside an engine
  node returns a successful `PaladinResult` (`StopReason::TokenBudget`'s `is_successful()` is
  `true`), so its partial output would merge into the Battlefield as if the node succeeded, not
  produce a resumable halt; and the engine's `EngineExecutionPort` wraps one boot-time
  `PaladinExecutionService` shared by every concurrent run, so a chain installed there is not
  per-run.
- **Refuse the call inside the pricing decorator with an `LlmError`** (rejected): an `LlmError`
  fails the node — or consumes an Aegis retry, or triggers a `FallbackLlmAdapter` hop to another
  provider — instead of writing a checkpointed halt, and a port below the engine has no run
  identity (the `FallbackHop { node_id: None }` precedent in `crates/paladin-llm/src/
  fallback.rs`, where the adapter cannot know its node).
- **A separate enforcement mechanism per path with a new trait** (rejected): duplicates the
  metering seam D-09 already gives both paths for no additional benefit.

## Code Locations

- `crates/paladin-llm/src/pricing.rs` — the pricing decorator this ADR's metering half attaches to
- `crates/paladin-llm/src/fallback.rs` — `FallbackLlmAdapter`, the composition-order precedent and the `FallbackHop { node_id: None }` evidence for the rejected refuse-in-decorator option
- `src/infrastructure/web/agent_host.rs` — `build_agent`/`build_agent_with_llm`, the agent-loop `Arc<dyn LlmPort>` resolution site
- `src/infrastructure/web/facade_provisioner.rs` — `paladin_port_from_settings`, `EngineExecutionPort`, the engine-path boot-time shared service
- `crates/paladin-battalion/src/engine/superstep.rs` — the `WaypointStatus::Halted` write at the superstep boundary
- `crates/paladin-battalion/src/engine/mod.rs` — `RunOutcome::Halted`, `run_finish_status`, the `RunFinished` emission
- `src/application/services/paladin/middleware/limits.rs` — `TokenBudget`, `after_model`, `StopReason::TokenBudget`
- `src/config/agent_runtime.rs` — `AgentRuntimeConfig::build_chain`, confirmed to have zero production callers
- `src/application/services/run/worker.rs` — the worker that maps `RunFinishStatus::Halted` to `RunStatus::Halted`

## Code Conformance

must change

Metering conforms once Phase 38 plans 38-02..38-08 land the pricing decorator and its downstream
carriers. The halt half is built by Phase 42 (ALLOW-03, ALLOW-05), which must cite this ADR rather
than re-open the attachment question.

## Downstream Consumers

- **Phase 41** — the admission slice of the Treasurer facade, which reads the balance this ADR's metering half feeds
- **Phase 42** — both halts (engine superstep boundary, agent-loop `TokenBudget` cutoff); must cite ADR-0052 rather than re-derive the attachment point
- **Phase 43** — the pacing decorator composes at the same `LlmPort` boundary, as a sibling of the pricing decorator this ADR names
</content>
