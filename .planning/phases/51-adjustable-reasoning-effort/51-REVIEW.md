---
phase: 51-adjustable-reasoning-effort
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/cadence.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/ollama/adapter.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/reasoning.rs
  - crates/paladin-llm/src/test_support.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/output/llm_port.rs
  - docs/src/user-guides/reasoning-effort.md
  - src/application/services/paladin/paladin_builder.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/infrastructure/web/agent_host.rs
  - tests/consumer_surface_muster.rs
findings:
  critical: 0
  warning: 2
  info: 4
  total: 6
status: issues_found
---

# Phase 51: Code Review Report (gap-closure, plans 51-15..51-20)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Summary

Scope was the diff `598eb945..HEAD` in the listed files: the single-agent `NodeFinished` reasoning
fields (51-15), the host-load fallback precedence fix (51-16), and the new provided method
`LlmPort::check_reasoning` with its nine native/compat overrides, the Pricing/Cadence forwards, the
per-hop `FallbackLlmAdapter` override and the builder delegation (51-17..51-20).

The prior CR-01 (inverted fallback precedence in the host check) and WR-01 (builder ignores the
provider-level `reasoning_fallback`) are fixed. I traced each one:

- `agent_host::check_block_reasoning` now resolves `def.reasoning_fallback.or(block.fallback)`,
  the same order as `reasoning::in_force` (request value first, provider default second).
- `reasoning::check` is `in_force` plus `judge`, and `admit` is `judge` plus the substitution
  record, so build-time and send-time share one verdict function. All nine adapters pass their
  own wire-masked profile and their own config defaults. The compat engine's `check_reasoning`
  reads the same `model_profile` and `default_reasoning` that `build_request` reads.
- The in-tree `LlmPort` decorators (`PricingLlmAdapter`, `CadenceLlmAdapter`) forward
  `check_reasoning`, and `FallbackLlmAdapter` overrides it. I found no other in-tree decorator
  that holds an inner `Arc<dyn LlmPort>` and implements `LlmPort`.
- `FallbackLlmAdapter::model_profile` fails closed on effort for a profile-less hop, and sampling
  still intersects over the hops that report one.
- `worker.rs` copies the two `paladin.reasoning_*` keys verbatim from `run_metadata` on success
  and leaves both `None` on failure. `merge_run_metadata` does carry these keys (any `paladin.*`
  key except `served_by`).

No blocker was found. Remaining items are one drift hazard (the host check is still a hand-rolled
copy of the rule), one error-text hygiene gap against the repo's credential-handling rule, and some
maintainability items.

Credential-handling review of the touched error and refusal text: the refusal strings are built
from provider name, model id, the effort spelling and fixed wording. No API key, header or response
body is rendered on the `UnsupportedParameter` path. The one exception is the `other` arm in
`PaladinBuilder::effort_refusal` (WR-01).

## Structural Findings (fallow)

None provided for this review.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `PaladinBuilder::effort_refusal` interpolates an arbitrary `LlmError` into a `ConfigurationError`

**File:** `src/application/services/paladin/paladin_builder.rs:1201-1206` (and `crates/paladin-llm/src/fallback.rs:981`)
**Issue:** `check_reasoning` is a public, overridable trait method. The builder's `other` arm
renders `{other}`, the `Display` of whatever `LlmError` a port returned:

```rust
other => PaladinError::ConfigurationError(format!(
    "reasoning_effort for model '{}' on provider '{}' could not be checked: {other}", ...
```

`FallbackLlmAdapter::check_reasoning` forwards a non-`UnsupportedParameter` error unchanged
(`Err(other) => return Err(other)`). A third-party port or decorator can return
`ProcessingError(String)`, `ApiError { .. }` or similar with vendor text. That text then lands in
a `ConfigurationError`, which the agent host puts into `HostBuildError::Build` and logs.

This conflicts with the doc comment on the same function ("Built from names, effort spellings and
fixed refusal wording only; nothing credential-shaped or request-derived is rendered"), and with
the repo rule that external text must be redacted before it reaches an error or log
(`security.instructions.md`, `crates/paladin-llm/src/redaction.rs`). No in-tree port can reach this
arm today, so the exposure is latent. The doc claim is false for any out-of-tree port.
**Fix:** Render a fixed string and keep the detail out of the message:

```rust
other => {
    log::debug!(target: "paladin::reasoning", "check_reasoning returned a non-refusal error");
    PaladinError::ConfigurationError(format!(
        "reasoning_effort for model '{}' on provider '{}' could not be checked ({})",
        self.data.model,
        self.llm_port.get_provider_name(),
        llm_error_kind(&other), // a fixed label per variant, never the payload
    ))
}
```

If the payload must be shown, pass it through the redaction helper first. In either case add a
test in which a port returns `LlmError::ProcessingError("sk-secret")` and assert the key is absent.

### WR-02: The agent host's block-default check is still a hand-rolled copy of the rule, and its text diverges from the other gates

**File:** `src/infrastructure/web/agent_host.rs:201-236` (with `crates/paladin-llm/src/reasoning.rs:298`, `crates/paladin-ports/src/output/llm_port.rs:2630`)
**Issue:** Plans 51-17..51-20 make `check_reasoning` the single pre-judging verdict, but
`check_block_reasoning` still re-implements it by hand: `llm.model_profile` plus
`profile.effort.resolve` plus its own copy of the fallback-hint condition. CR-01 in the previous
review was a precedence bug in exactly this duplicate, and the 51-16 fix patches the copy instead
of removing it.

Consequences visible today:

- The refusal text differs from the same verdict at send time and at build time. The adapters'
  `refusal_reason` produces `"... ; this model accepts: levels [low, high]; set a
  reasoning_fallback ..."`, while the host produces only `refusal.to_string()` plus the hint, and
  its no-profile sentence says "a reasoning effort" where the port default says "an explicit
  reasoning effort". The 54-row host grid compares `is_ok()` only, so it cannot see this.
- The hint literal now exists in three places: `reasoning::FALLBACK_HINT`, the private
  `REASONING_FALLBACK_HINT` in `paladin-ports` ("Kept in step" by comment only), and an inline
  string at `agent_host.rs:218`. No test ties them together.
- When the agent sets no effort, the builder's own `check_reasoning` call (which sees the
  adapter's block default) already refuses the same cases, so the host check only adds the
  `HostBuildError` wrapping and the key name.

**Fix:** Delegate to the port and keep only the key attribution, for example:

```rust
fn check_block_reasoning(def: &AgentDefinition, llm: &dyn LlmPort, block: &BlockReasoning)
    -> Result<(), HostBuildError>
{
    llm.check_reasoning(&def.model, None, def.reasoning_fallback)
        .map_err(|error| HostBuildError::Build {
            id: def.id.clone(),
            source: PaladinError::ConfigurationError(error.to_string()),
        })
}
```

The adapter's reason already ends with ``(set by `llm.<provider>.reasoning_effort`)``, so the
`BlockReasoning.key` field can go too. If the explicit rule is kept, at least extend the grid test
to compare the refusal text against `adapter.check_reasoning`, and export the hint constant from
`paladin-ports` so the three copies become one.

## Info

### IN-01: `the_builder_verdict_equals_check_reasoning_for_every_combination` is tautological

**File:** `src/application/services/paladin/paladin_builder.rs:2117-2170`
**Issue:** `PaladinBuilder::validate` now calls `llm_port.check_reasoning(...)` and nothing else
for the effort verdict. The test compares `build().is_ok()` with the same call on the same port, so
it can only fail if `validate` stops delegating. It does guard that, but it reads as a parity proof
and is not one. The real parity proofs are the adapter-level `generate` versus `check_reasoning`
tests and the conformance case.
**Fix:** Say so in the doc comment, or compare against `generate` on the same adapter, as the host
grid does.

### IN-02: `read_agent_trace` gives up silently after 2 s and returns partial rows

**File:** `src/application/services/run/worker_tests.rs:1463-1480`
**Issue:** The poll loop (100 x 20 ms) exits without any indication when `RunFinished` never
arrives and returns whatever rows exist. On a loaded machine (the machine crashed from CPU load
earlier today) the callers then fail on `node_finished_reasoning`'s `expect("an agent run persists
one NodeFinished row")` or on the `[1, 2, 3, 4]` seq assertion. The message points at the wrong
cause, and it is a timing flake.
**Fix:** Assert after the loop that `RunFinished` was seen (`assert!(done, "RunFinished never
persisted within 2 s: {rows:?}")`), or lengthen the budget.

### IN-03: CHANGELOG entry has a vacuous sentence and an over-long line

**File:** `crates/paladin-llm/CHANGELOG.md:246`
**Issue:** The Kimi/Qwen/Grok entry now ends "No further public item changed beyond those listed in
this section." on one very long line. The entry lists no public item, so the sentence refers to
nothing and the line is far past the 100-column convention used around it.
**Fix:** Drop the sentence (the earlier wording was removed for being wrong, not replaced by
anything useful) or wrap it.

### IN-04: The chain-level availability change is documented only as a refusal rule

**File:** `crates/paladin-llm/src/fallback.rs:338-353`, `crates/paladin-llm/CHANGELOG.md:215-221`
**Issue:** With the per-hop `check_reasoning`, a standby hop whose provider default effort its step
model cannot honour now refuses every request up front, even when the request sets no effort and
the primary hop would have served. Before 51-19 the same chain served from hop 0 and failed only
if it reached the bad hop. The change is intentional (D-08) and the CHANGELOG states it, but the
operator-facing guide says only that the chain "refuses the chain before the first call". It does
not tell operators that a misconfigured standby hop now takes down a healthy primary.
**Fix:** Add one sentence to the "Fallback chains" section of
`docs/src/user-guides/reasoning-effort.md` (around line 216-233) naming this consequence and the
fix, which is to set the effort on the agent or the request, or to correct the hop's
`llm.<provider>.reasoning_effort`.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
