---
phase: 26-agent-runtime-enhancements
reviewed: 2026-09-07T16:09:49Z
depth: standard
files_reviewed: 119
files_reviewed_list:
  - crates/doc-examples/src/agent_runtime.rs
  - crates/doc-examples/src/lib.rs
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/engine/directive_parser.rs
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/node.rs
  - crates/paladin-battalion/src/engine/registries.rs
  - crates/paladin-battalion/src/engine/retry.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-battalion/src/grove_service.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-core/src/lib.rs
  - crates/paladin-core/src/platform/container/aegis.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/garrison.rs
  - crates/paladin-core/src/platform/container/mod.rs
  - crates/paladin-core/src/platform/container/paladin_error.rs
  - crates/paladin-core/src/platform/container/run_scope.rs
  - crates/paladin-core/src/platform/container/structured.rs
  - crates/paladin-core/src/platform/container/vault.rs
  - crates/paladin-core/src/platform/container/waypoint.rs
  - crates/paladin-llm/benches/llm_serialization_benchmarks.rs
  - crates/paladin-llm/examples/live_vendor_smoke.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/anthropic/vision.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/conformance.rs
  - crates/paladin-llm/src/deepseek/adapter.rs
  - crates/paladin-llm/src/fallback.rs
  - crates/paladin-llm/src/gemini/adapter.rs
  - crates/paladin-llm/src/grok/adapter.rs
  - crates/paladin-llm/src/kimi/adapter.rs
  - crates/paladin-llm/src/lib.rs
  - crates/paladin-llm/src/llm_analysis_service.rs
  - crates/paladin-llm/src/mock.rs
  - crates/paladin-llm/src/ollama/adapter.rs
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/src/openai_compatible/adapter.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/redaction.rs
  - crates/paladin-memory/migrations/002_add_garrison_is_summary.sql
  - crates/paladin-memory/migrations/003_create_vault_tables.sql
  - crates/paladin-memory/src/garrison/in_memory_garrison.rs
  - crates/paladin-memory/src/garrison/sqlite_garrison.rs
  - crates/paladin-memory/src/garrison/token_counter.rs
  - crates/paladin-memory/src/lib.rs
  - crates/paladin-memory/src/migrations.rs
  - crates/paladin-memory/src/prelude.rs
  - crates/paladin-memory/src/services/memory_extraction_service.rs
  - crates/paladin-memory/src/token_counter/heuristic.rs
  - crates/paladin-memory/src/token_counter/mod.rs
  - crates/paladin-memory/src/vault/contract_tests.rs
  - crates/paladin-memory/src/vault/in_memory.rs
  - crates/paladin-memory/src/vault/mod.rs
  - crates/paladin-memory/src/vault/redact.rs
  - crates/paladin-memory/src/vault/semantic.rs
  - crates/paladin-memory/src/vault/sqlite.rs
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - crates/paladin-ports/src/output/structured_executor_port.rs
  - crates/paladin-ports/src/output/token_counter_port.rs
  - crates/paladin-ports/src/output/vault_confined.rs
  - crates/paladin-ports/src/output/vault_port.rs
  - crates/paladin-ports/src/output/vision_llm_port.rs
  - crates/paladin-web/src/agent_controller.rs
  - src/application/cli/formatters/output.rs
  - src/application/services/arsenal/composite_arsenal.rs
  - src/application/services/arsenal/in_process_arsenal.rs
  - src/application/services/arsenal/mod.rs
  - src/application/services/arsenal/vault_tools.rs
  - src/application/services/paladin/middleware/chain.rs
  - src/application/services/paladin/middleware/context.rs
  - src/application/services/paladin/middleware/guardrail.rs
  - src/application/services/paladin/middleware/history.rs
  - src/application/services/paladin/middleware/limits.rs
  - src/application/services/paladin/middleware/mod.rs
  - src/application/services/paladin/middleware/resilience.rs
  - src/application/services/paladin/middleware/summarization.rs
  - src/application/services/paladin/middleware/tool_protocol.rs
  - src/application/services/paladin/middleware/vault_recall.rs
  - src/application/services/paladin/mod.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/paladin/planning_service.rs
  - src/application/services/paladin/prompt_generation_service.rs
  - src/application/services/paladin/structured.rs
  - src/application/services/paladin/temperature_service.rs
  - src/application/services/paladin/vault_confined.rs
  - src/config/agent_runtime.rs
  - src/config/mod.rs
  - src/config/settings.rs
  - src/config/user_config.rs
  - src/infrastructure/adapters/arsenal/tool_result_formatter.rs
  - src/infrastructure/resilience/circuit_breaker.rs
  - src/lib.rs
  - src/prelude.rs
  - src/presets/mod.rs
  - tests/helpers/mock_llm_adapter.rs
  - tests/integration/anthropic_provider_test.rs
  - tests/integration/cli_real_providers_test.rs
  - tests/integration/deepseek_provider_test.rs
  - tests/integration/llm_live_api_tests.rs
  - tests/integration/middleware_under_engine_test.rs
  - tests/integration/mod.rs
  - tests/integration/ollama_docker_test.rs
  - tests/integration/openai_content_analysis_integration_test.rs
  - tests/integration/openai_provider_test.rs
  - tests/integration/provider_switching_test.rs
  - tests/integration/reasoning_agent_test.rs
  - tests/integration/structured_engine_node_test.rs
  - tests/integration/vault_confinement_test.rs
  - tests/integration/vision_integration_test.rs
  - tests/unit/llm/anthropic_adapter_test.rs
  - tests/unit/llm/deepseek_adapter_test.rs
  - tests/unit/mock_llm_adapter_test.rs
findings:
  critical: 2
  warning: 3
  info: 1
  total: 6
status: issues_found
---

# Phase 26: Code Review Report

**Reviewed:** 2026-09-07T16:09:49Z
**Depth:** standard
**Files Reviewed:** 119
**Status:** issues_found

## Summary

Reviewed the phase-26 diff (`c8093f36..HEAD`) against the parent commit, prioritizing
`paladin_execution_service.rs`, the new `middleware/` tree, `ConfinedVault`/`VaultTools`,
the Vault storage/redaction stack, `redaction.rs`, the engine's `output_schema` wiring
(`graph.rs`/`superstep.rs`), `config/agent_runtime.rs`, and `presets/mod.rs`.

The overall implementation is unusually well-disciplined for its size: the
`ExecutionMiddleware` onion-chain driver (`chain.rs`), the `Namespace`/`ConfinedVault`
confinement primitive, the fail-closed `output_schema` graph validation, and the
provider adapters' pre-existing redirect-refusal/credential-header controls all held up
under adversarial reading, including deliberately-probed edge cases (prefix-collision
namespaces, zero/negative budgets, concurrent runs sharing one `Arc<dyn
ExecutionMiddleware>`, empty chains, disabled sections). Test coverage for the new
surfaces is genuinely thorough and mostly matches the code's own claims.

Two real defects surfaced despite that:

1. A security control the phase explicitly built (redact-then-bound tool-error text,
   D-33/D-34) is applied to only one of the two paths a failed tool result can take
   through `ToolResultFormatter` — the pre-existing `format_result` path (a *business*
   failure returned as `ArmamentResult { success: false, .. }`) still embeds `error` raw,
   with no redaction and no bounding, directly contradicting this phase's own stated
   security posture for tool-error text.
2. The reasoning loop silently drops a `FinalResult` returned by a later-firing
   `after_model` hook when the run is already finishing because an *earlier* hook's
   `before_model` requested `Finish` — an onion-chain contract violation that is handled
   correctly on the sibling (post-model-call) code path just below it, making the
   asymmetry clearly unintentional rather than a documented limitation.

Neither is a full compromise, but both are concrete, provable defects in code this
phase added, not pre-existing debt.

## Critical Issues

### CR-01: `ToolResultFormatter::format_result`'s error text bypasses redact-then-bound entirely

**File:** `src/infrastructure/adapters/arsenal/tool_result_formatter.rs:187-194`
**Also:** `src/application/services/paladin/paladin_execution_service.rs:2680-2703` (`handle_tool_call`, the only call site for the success/failure `ArmamentResult` path)

**Issue:**

Plan 26-19 added `ToolResultFormatter::format_error` (lines 243-259) specifically to
apply `redact_secret_patterns` before `bounded_excerpt` to tool-failure text before it
reaches the model — this is the exact security control
`.github/instructions/security.instructions.md` and this phase's own module docs (D-33,
D-34, T-26-03) require: "redact-then-bound... never the reverse." But that new function
is wired into only ONE of the two places a tool failure's text reaches the model.

`handle_tool_call` (the code path an `ArsenalPort::invoke` call actually takes on
success) calls the pre-existing `format_result` for **every** `ArmamentResult` it gets
back, success or business-failure:

```rust
// tool_result_formatter.rs:187-194 (format_result, UNCHANGED by this phase)
} else {
    let error_icon = if self.use_emoji { "❌ " } else { "" };
    output.push_str(&format!("{}Result: FAILED\n", error_icon));

    if let Some(ref error) = result.error {
        output.push_str(&format!("Error: {}\n", error));   // <-- no redaction, no bounding
    }
}
```

An `Armament` that returns `Ok(ArmamentResult::failure(id, message, ms))` (e.g. a
web-search or HTTP-calling tool that echoes an upstream error body verbatim, including a
`Authorization: Bearer <token>` header or a `sk-...`/`AKIA...` key it received in its own
response) has that `message` embedded here completely unredacted and unbounded, then:

- injected into `accumulated_output` and sent back to the model on the next loop
  iteration (`paladin_execution_service.rs:2917-2926`, the `Ok(formatted_result)` arm of
  `handle_tool_call`'s caller), and
- persisted into Garrison via `GarrisonEntry::new(ConversationRole::Tool,
  formatted_result)` — so a credential leaked this way is now durably stored in
  conversation history, not just transiently logged.

This is a real gap in exactly the control this phase's own commit history (`feat(26-19):
tool-error policy -- shared formatter, redact-then-bound, FailRun...`) claims to close:
only the `ArsenalError`/handoff-`Err` path (which calls the new `format_error`) is
protected. The `ArmamentResult{success:false}` business-failure path — arguably the MORE
likely path for a tool to surface an upstream response body, since it is the tool
author's own designed error-reporting mechanism rather than an exceptional Rust `Err` —
is not.

**Fix:** Route `format_result`'s error branch through the same
`redact_secret_patterns`/`bounded_excerpt` pair `format_error` uses, e.g.:

```rust
if let Some(ref error) = result.error {
    let redacted = paladin_llm::redaction::redact_secret_patterns(error);
    let sanitized = paladin_llm::redaction::bounded_excerpt(
        &redacted,
        paladin_llm::redaction::RESPONSE_EXCERPT_CHAR_BUDGET,
    );
    output.push_str(&format!("Error: {}\n", sanitized));
}
```

or, more consistently with the rest of the phase's design, have `format_result`'s error
branch delegate to `format_error`'s sanitization step directly so there is exactly one
place tool-error text is sanitized, matching this file's own stated intent.

### CR-02: A `Finish` returned by a later `after_model` hook is silently dropped when the run is already finishing via an earlier `before_model` `Finish`

**File:** `src/application/services/paladin/paladin_execution_service.rs:1392-1435`
**Compare:** `src/application/services/paladin/paladin_execution_service.rs:1462-1500` (the correctly-handled sibling path)

**Issue:**

The onion-chain contract (`chain.rs` module docs, D-06) is: when `before_model` finishes
early from index `i`, `run_after` still runs `after_model` on the reached prefix
`0..i`, and if ANY of those hooks itself returns `MiddlewareFlow::Finish`, `run_after`
returns `Ok(Some(FinalResult))` naming that hook's own output and `stop_reason` — this is
exactly what `chain::run_after`'s doc comment says: *"Returns `Ok(Some(result))` if some
middleware in the prefix requested an early finish via its own `after_model`... its own
`Finish`/`Fail` is terminal for the `after` pass."*

The reasoning loop's `BeforeOutcome::Finish` arm calls `run_after` but discards that
return value entirely:

```rust
// paladin_execution_service.rs:1405-1412
run_after(
    &self.middleware,
    &mut middleware_cx,
    &mut synthetic_view,
    reached,
)
.await?;                                    // <-- Option<FinalResult> thrown away
accumulated_output = synthetic_view.content;
```

...and then unconditionally returns using the ORIGINAL `before_model` Finish's own
`result.stop_reason` (line 1431), regardless of whether a later-firing `after_model` hook
(over the `reached` prefix) requested its OWN finish with a different `stop_reason` (or
different `output`, for a hook that returns `FinalResult::new(text, ..)` without also
mutating `resp.content` in place).

Compare the sibling path 60 lines below, covering the same situation after a REAL model
call instead of a synthetic one, which handles this correctly:

```rust
// paladin_execution_service.rs:1471-1500
if let Some(final_result) = run_after(
    &self.middleware,
    &mut middleware_cx,
    &mut response_view,
    reached,
)
.await?
{
    accumulated_output = response_view.content;
    ...
    return Ok(PaladinResult {
        ...
        stop_reason: final_result.stop_reason,   // <-- correctly threaded through
        ...
    });
}
```

Concretely: a run with `ModelCallLimit` (finishes via `before_model` with
`StopReason::CallLimit`) installed ahead of a `Guardrail` rule whose `on_match` is
`GuardrailAction::Finish` and whose `target` is `Response`/`Both` will, once the call
limit trips, run the Guardrail's `after_model` over the synthetic view. If that rule
matches the synthetic text, `Guardrail::after_model` returns
`MiddlewareFlow::Finish(FinalResult::new(message, StopReason::Completed))` — but the
returned `PaladinResult.stop_reason` will still read `StopReason::CallLimit`, not
`Completed`, silently misreporting why the run actually ended. For any custom
`ExecutionMiddleware` whose `Finish` action does not *also* mutate `resp.content`
in-place (nothing in the trait requires it to), the returned `output` string is lost
outright and the caller receives the stale synthetic content instead of the middleware's
actual intended answer.

This is exercised by no existing test — none of `limits.rs`, `guardrail.rs`, or
`chain.rs`'s test suites combine a `before_model`-Finish middleware with an
`after_model`-Finish middleware to catch this asymmetry.

**Fix:** Mirror the sibling path — capture and honor the `Option<FinalResult>`:

```rust
BeforeOutcome::Finish { result, reached } => {
    let mut synthetic_view = LlmResponseView { content: result.output.clone(), .. };
    let effective_result = run_after(&self.middleware, &mut middleware_cx, &mut synthetic_view, reached)
        .await?
        .unwrap_or(result);
    accumulated_output = synthetic_view.content;
    ...
    return Ok(PaladinResult {
        ...
        stop_reason: effective_result.stop_reason,
        ...
    });
}
```

## Warnings

### WR-01: `redact_secret_patterns`'s `key=`/`token=` markers over-match on ordinary words, corrupting benign tool-error text

**File:** `crates/paladin-llm/src/redaction.rs:130-134` (the `key=`/`token=` passes added by D-34), using `redact_token_after` at `crates/paladin-llm/src/redaction.rs:72-101`

**Issue:** `redact_token_after` locates `marker` via plain substring `find`, with no word
boundary check. The new `"key="`/`"token="` markers therefore match inside any ordinary
word ending in those letters immediately followed by `=`, not just an actual
`api_key=`/`access_token=`-style query parameter: `"monkey=5"`, `"donkey=3"`,
`"turkey=roast"`, `"jockey=true"` (any word ending `...key` or `...token`, directly
followed by `=`) all trip the pattern and have their trailing value replaced with
`[REDACTED]`. Since `redact_secret_patterns` now runs on every tool-error string
(`ToolResultFormatter::format_error`, `tool_result_formatter.rs:249`) as well as every
provider response body, an entirely benign echoed argument like
`{"monkey": 5}` embedded in upstream error JSON becomes `{"monkey=[REDACTED]"` or similar
mangled text fed back to the model, degrading the model's ability to self-correct from a
tool failure. This fails safe (over-redaction, not a leak) but is a real, provable
false-positive with no test guarding against it — none of the new
`redact_secret_patterns_covers_the_documented_set` test's cases probe a benign
`...key=`/`...token=`-suffixed word.

**Fix:** Require a non-alphanumeric boundary (or a fixed small set of known prefixes
like `api_key=`, `access_token=`) immediately before the marker, or switch to a `regex`
with a `\b` word boundary / a preceding `[?&]` for the query-string cases, so `key=`
only matches as a whole query parameter name.

### WR-02: `execute_json_schema`/`execute_structured_call` bypass the entire `ExecutionMiddleware` chain

**File:** `src/application/services/paladin/paladin_execution_service.rs:2818-2996` (the `StructuredExecutorPort` impl and `execute_structured_call`)

**Issue:** `execute_structured_call` builds its own scratch `ModelCallContext` and
dispatches directly through `execute_with_retry_and_temperature`, never through
`run_before`/`run_after`/`run_around_tool` (`chain.rs`). This is correctly documented as
deliberate with respect to NOT re-running the multi-loop reasoning loop, but it has a
side effect the module docs do not call out: `Guardrail` (prompt/response screening),
`VaultRecallMiddleware`, `ToolCallLimit`, `TokenBudget`/`ModelCallLimit`, and any
consumer-installed custom middleware are **silently inert** for every structured-output
call. A deployment that installs a `Guardrail` rule specifically to block a class of
response content, or a `TokenBudget` to cap spend, gets no protection at all on the
`execute_structured`/`execute_json_schema` surface — the same `PaladinExecutionService`
instance enforces its safety policy on `execute()` but not on `execute_structured()`,
with no error, warning, or documentation surfacing that gap to an integrator. Given this
phase's own stated security posture (guardrails/redaction/vault-recall framing are load-
bearing controls, per the module docs of `guardrail.rs` and `vault_recall.rs`), silently
exempting a whole execution surface from all of them is worth flagging even though it is
plausibly an intentional scope cut for this phase.

**Fix:** At minimum, document this exemption prominently on
`StructuredExecutorPort`/`execute_json_schema`'s own rustdoc (not just inferable from
"does not run the reasoning loop"), and/or route `Guardrail` specifically (which needs
no per-iteration state) through the structured path so a deployed content policy still
applies to structured extraction.

### WR-03: `ToolResultFormatter::format_result`'s success `Output:` field is also unredacted/unbounded

**File:** `src/infrastructure/adapters/arsenal/tool_result_formatter.rs:178-186`

**Issue:** Related to CR-01 but distinct: even a *successful* `ArmamentResult.output`
(e.g. a tool that fetches and returns a remote page/API body verbatim) is written into
the model-facing text and Garrison via `format_output_value` with no size bound and no
credential redaction at all — only the *error* path was ever a redaction target anywhere
in this codebase's design docs, but a successful tool call that happens to return
attacker- or upstream-controlled content containing a credential-shaped string (e.g. a
web-search tool returning a page that contains `Authorization: Bearer ...` in its body)
is embedded raw. This existed before this phase, but this phase is the one that
established `redact_secret_patterns` as a key-less, generally-applicable sanitizer
(`redaction.rs`'s own docs say it is "for a caller with no specific configured key to
redact against"), and did not extend it to this equally-applicable call site.

**Fix:** Consider applying `redact_secret_patterns` (not just `bounded_excerpt`, which
`format_output_value` doesn't apply either) to arbitrary string tool output before it is
embedded into the model-facing text and Garrison, matching the redact-then-bound
discipline enforced everywhere else on the provider-response path.

## Info

### IN-01: JWT-triple redaction can false-positive on ordinary multi-label hostnames

**File:** `crates/paladin-llm/src/redaction.rs:159-207` (`redact_jwt_triples`)

**Issue:** `redact_jwt_triples` matches any three dot-separated base64url runs each
`>= JWT_MIN_SEGMENT_LEN` (10) characters, including `-`/`_`. A plausible real-world
string like a Kubernetes-style internal hostname (`some-service-name.some-namespace-01.svc-cluster-internal`)
or a long multi-segment content hash/version string can satisfy this shape and be
replaced wholesale with `[REDACTED]` even though it carries no credential. This is a
false-positive risk in the safe direction (over-redaction), and is lower priority than
WR-01 since 10+ character hyphenated hostname labels chained three deep are less common
in tool-error/log text than a `key=`/`token=` query parameter, but is worth a note for
anyone tuning `JWT_MIN_SEGMENT_LEN` or debugging an unexpectedly redacted diagnostic.

**Fix:** No action required unless real deployments report false positives; if they do,
tightening the JWT heuristic (e.g. requiring the middle segment to itself parse as
base64 JSON, which a real JWT payload always does) would remove this class of
false-positive without weakening the credential-leak protection.

---

_Reviewed: 2026-09-07T16:09:49Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
