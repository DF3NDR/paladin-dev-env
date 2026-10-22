---
phase: 51-adjustable-reasoning-effort
reviewed: 2026-10-22T00:00:00Z
depth: standard
files_reviewed: 85
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/src/council_service.rs
  - crates/paladin-battalion/src/engine/cache_key.rs
  - crates/paladin-battalion/src/engine/export/overlay.rs
  - crates/paladin-battalion/src/engine/graph_doc.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-battalion/tests/export_golden.rs
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/Cargo.toml
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/paladin.rs
  - crates/paladin-core/src/platform/container/reasoning.rs
  - crates/paladin-core/src/platform/container/token_usage.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/tests/assertion_snapshots.rs
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-llm/examples/live_vendor_smoke.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/compat/mod.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/config/bridge.rs
  - crates/paladin-llm/src/config/layers.rs
  - crates/paladin-llm/src/config/llm.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/ollama/adapter.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/provider_factory.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/reasoning.rs
  - crates/paladin-llm/src/registry/mod.rs
  - crates/paladin-llm/src/registry/seeded.rs
  - crates/paladin-llm/src/sampling.rs
  - crates/paladin-llm/src/test_support.rs
  - crates/paladin-llm/tests/golden/reasoning_unset/anthropic.json
  - crates/paladin-llm/tests/golden/reasoning_unset/deepseek.json
  - crates/paladin-llm/tests/golden/reasoning_unset/gemini.json
  - crates/paladin-llm/tests/golden/reasoning_unset/grok.json
  - crates/paladin-llm/tests/golden/reasoning_unset/kimi.json
  - crates/paladin-llm/tests/golden/reasoning_unset/ollama.json
  - crates/paladin-llm/tests/golden/reasoning_unset/openai-compatible.json
  - crates/paladin-llm/tests/golden/reasoning_unset/openai.json
  - crates/paladin-llm/tests/golden/reasoning_unset/qwen.json
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/model_profile.rs
  - docs/src/SUMMARY.md
  - docs/src/appendix/provider-expansion.md
  - docs/src/user-guides/reasoning-effort.md
  - docs/src/user-guides/switching-providers.md
  - src/application/services/assistant/validator.rs
  - src/application/services/paladin/paladin_builder.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/events.rs
  - src/application/services/run/inspector.rs
  - src/application/services/run/stream_tests.rs
  - src/application/services/run/worker.rs
  - src/config/agents.rs
  - src/core/mod.rs
  - src/core/platform/mod.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/telemetry/otel_sink.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - tests/cli/run_export_test.rs
  - tests/cli_isolation_test.rs
  - tests/consumer_surface_muster.rs
  - tests/integration/otel_transport_test.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 51: Code Review Report

**Reviewed:** 2026-10-22
**Depth:** standard
**Files Reviewed:** 85
**Status:** issues_found

## Summary

The phase's hunks were reviewed with `git diff b6ccecc5..HEAD` per file. The core design is sound: one admission rule (`EffortSupport::resolve`) shared by all nine adapters, the builder, the fallback pre-flight and the docs drift test; fail-closed defaults (`Unmeasured`, `Unsupported` wire); wire masking via `mask_effort`; byte-identical unset bodies pinned by golden files; no `unwrap`/`expect`/`panic!` in non-test code of the touched files (checked mechanically against the diff, every hit is in `#[cfg(test)]`, `test_support` or `conformance` harness code); error texts name provider, model and effort only; the probe example uses `redirect(Policy::none())` and redacts before truncation. No credential-leak or injection issue was found.

The defects are all in how the three precedence levels (request, agent, provider) are combined for the **fallback** half of the setting. The adapters implement the documented field-by-field precedence correctly (`reasoning::in_force`), but three other call sites that pre-judge a request (the YAML host check, `PaladinBuilder::build`, `FallbackLlmAdapter` pre-flight) do not reproduce it, so they can accept what send-time refuses or refuse what send-time would accept. One trace emission site also drops the effort the adapter reported.

## Critical Issues

### CR-01: Host check of a provider-default effort resolves the fallback with inverted precedence

**File:** `src/infrastructure/web/agent_host.rs:200-204` (struct doc at `:183`)
**Issue:** `check_block_reasoning` computes the fallback as `block.fallback.or(def.reasoning_fallback).unwrap_or_default()` and the field doc states the block's fallback "wins over the agent's own". At send time the opposite happens: the agent's fallback is put on every `LlmRequest` (`paladin_execution_service.rs:3150-3155`, `:3366-3371`, `:4003-4008`) and `reasoning::in_force` takes `request_fallback.or(default_fallback)`, so the agent's value wins (the docs also say "a request's own fallback wins"). The load-time verdict therefore disagrees with the runtime verdict whenever both are set and differ:
- block `nearest_lower`, agent `refuse`, block default effort `xhigh` on a model lacking `xhigh`: the check passes, then every call is refused at runtime with `UnsupportedParameter`.
- block `refuse`, agent `nearest_lower`: the check refuses the agent at config load although runtime would have substituted.
The same applies when the agent sets only a fallback and the effort comes from the block. This is exactly the "build-time answer and send-time answer cannot drift" property the phase set out to guarantee (see the comment at `paladin_builder.rs:1355-1358`). The unit tests only cover "block sets fallback, agent sets none" and "agent sets fallback, block sets none", which agree under either order, so the inversion is untested.
**Fix:**
```rust
// agent_host.rs: mirror reasoning::in_force (request/agent value first, provider default second)
let fallback = def
    .reasoning_fallback
    .or(block.fallback)
    .unwrap_or_default();
```
Update the `BlockReasoning.fallback` doc ("which the agent's own fallback wins over") and add a test with block `nearest_lower` + agent `refuse` expecting a refusal, and the reverse expecting a pass.

## Warnings

### WR-01: Build-time and chain pre-flight checks ignore the provider-level `reasoning_fallback`

**File:** `src/application/services/paladin/paladin_builder.rs:1363-1384`, `crates/paladin-llm/src/fallback.rs:700-724` (`preflight_reasoning`)
**Issue:** `reasoning_fallback` is documented as having "the same three places (request, agent, provider) and the same precedence" as the effort (`docs/src/user-guides/reasoning-effort.md:143-144`), and the adapters honour it (`in_force`). But `PaladinBuilder::build` resolves with `self.data.reasoning_fallback.unwrap_or_default()` and `FallbackLlmAdapter::preflight_reasoning` with `request.reasoning_fallback.unwrap_or_default()`; neither can see a hop's or provider's `llm.<provider>.reasoning_fallback`. Result: with `llm.gemini.reasoning_fallback: nearest_lower` and an agent `reasoning_effort: xhigh` (no agent fallback), the build is refused ("set a reasoning_fallback to substitute the nearest level") although the adapter would substitute at send time; likewise a chain request carrying `xhigh` with no request fallback is refused up front by the pre-flight while every hop's configured default would have substituted. The provider fallback key is effectively inert for any caller that goes through these two gates, and the error text tells the operator to set something they have already set. The docs only disclose this for the effort, not the fallback (`reasoning-effort.md:217-218`).
**Fix:** Either expose the provider defaults to the pre-judging call sites (e.g. a `LlmPort::reasoning_defaults(model) -> (Option<ReasoningEffort>, Option<ReasoningFallback>)` default method that the adapters and `FallbackLlmAdapter` implement, and use `request.or(default)` in both gates), or document the limitation explicitly and make the refusal text say that a provider-level fallback is not consulted here.

### WR-02: Single-agent run `NodeFinished` hard-codes the reasoning fields to `None`

**File:** `src/application/services/run/worker.rs:1726-1727` (and the failure site `:1793-1794`, which is correct to be `None`)
**Issue:** REASON-05 / D-11 require the requested and effective effort on the Phase 28 trace stream. The engine path reads them from the attempt's `run_metadata` (`superstep.rs`, `AttemptReasoning::from_run_metadata`), but the run worker's single-agent success path emits `reasoning_requested: None, reasoning_effective: None` even though `result` is a `PaladinResult` carrying `run_metadata` (which `merge_run_metadata` fills with the `paladin.reasoning_*` keys). Every run submitted to a single agent (the `/v1/agents` path) therefore traces no effort, and a substitution is invisible in the trace for those runs. Nothing in the diff documents this as an intentional exclusion; `trace.rs:271-290` says `None` only for non-Paladin nodes, cache hits, failed attempts or no effort in force.
**Fix:**
```rust
use paladin_core::platform::container::reasoning::{
    REASONING_EFFECTIVE_METADATA_KEY, REASONING_REQUESTED_METADATA_KEY,
};
// in the Ok(result) arm
reasoning_requested: result.run_metadata.get(REASONING_REQUESTED_METADATA_KEY).cloned(),
reasoning_effective: result.run_metadata.get(REASONING_EFFECTIVE_METADATA_KEY).cloned(),
```
and add a worker test mirroring `node_finished_carries_the_attempts_own_reasoning_metadata`.

### WR-03: Chain `model_profile` silently drops hops that report no profile, so the build-time check is weaker than the send-time check

**File:** `crates/paladin-llm/src/fallback.rs:933-941` (consumed by `paladin_builder.rs:1366-1382`)
**Issue:** `FallbackLlmAdapter::model_profile` uses `filter_map(...).reduce(intersect)`, so a hop whose `model_profile` is `None` is ignored rather than making the chain unmeasured. For an explicit effort, `PaladinBuilder::build` then judges only the hops that do report a profile and can accept the agent, while `preflight_reasoning` refuses the very first request ("it reports no model profile, so an explicit reasoning effort cannot be checked"). The builder docs promise "a level must be usable on every hop" and "a port that reports no profile refuses an explicit effort (D-03)"; for a mixed chain neither holds at build time, so a misconfigured chain is found on its first call, which is what the pre-flight was added to avoid.
**Fix:** In `model_profile`, return `None` if any hop returns `None` for the effort-judging purpose (or add an `effort`-specific accessor that does), e.g. collect into `Option<Vec<_>>` and reduce only when every hop answered. Keep the sampling-profile behaviour unchanged if it is intentionally lenient.

### WR-04: Provider-level default effort is not part of the node-cache key, so changing it serves stale answers

**File:** `crates/paladin-battalion/src/engine/cache_key.rs:36-46, 263-283`
**Issue:** The key covers the agent's `reasoning_effort` / `reasoning_fallback` but, by design, not `llm.<provider>.reasoning_effort`. An operator who changes the provider default (or removes it) keeps hitting cache entries produced at the previous effort for every agent that sets none of its own, with no signal in the response or trace (cache hits stamp no effort, by `superstep.rs` design). The documentation tells operators to rotate `key_prefix` by hand (`reasoning-effort.md:549-553`), which is an easy step to miss and defeats the point of the REASON-01 cache-correctness work (the A/B scenario is only protected for the agent level).
**Fix:** Surface the provider default to the key (for example include the adapter-reported default in the Paladin node's fingerprint via a port method, see WR-01) or, at minimum, log at config load when a provider default is set and the node cache is enabled.

## Info

### IN-01: Duplicate list of self-hosted providers

**File:** `crates/paladin-llm/src/reasoning.rs:255` and `crates/paladin-llm/src/config/layers.rs:342`
**Issue:** `SELF_HOSTED_PROVIDERS` and `REASONING_WIRE_PROVIDERS` are the same `["ollama", "openai-compatible"]` list in two modules; the refusal hint and the config validation can drift apart if a third self-hosted provider is added.
**Fix:** Define one `pub(crate) const` (in `reasoning.rs`) and import it in `layers.rs`.

### IN-02: Cache key swallows a serialisation failure into an empty field

**File:** `crates/paladin-battalion/src/engine/cache_key.rs:274-283`
**Issue:** `serde_json::to_string(effort).unwrap_or_default()` would map any serialisation failure to an empty string, making distinct efforts collide on the same key. It cannot fail for the current `ReasoningEffort`/`ReasoningFallback` shapes, but the enums are `#[non_exhaustive]` and the surrounding code (`max_loops`, `muster.payload`) uses the same idiom.
**Fix:** Use the `Display` spelling (`effort.to_string()`), which is infallible and already the documented canonical spelling, or keep serde and `debug_assert!` the `Ok`.

### IN-03: CHANGELOG entries say "No public item changed" beside public additions and breaks

**File:** `crates/paladin-llm/CHANGELOG.md` (the 51-12, 51-11 and 51-07 entries added by this phase)
**Issue:** Several entries end "No public item changed" while the same changelog section records new public items (`paladin_llm::reasoning`, `ModelOverride.effort`, `with_reasoning_*` builders, `UnsupportedParameter`) and an explicit BREAKING change (`CompatRequestParameters::all(reasoning)`). A reader scanning for semver impact gets contradictory signals.
**Fix:** Reword to "No further public item changed beyond those listed above" or drop the sentence where the entry itself lists public additions.

---

_Reviewed: 2026-10-22_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
