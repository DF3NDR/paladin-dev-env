---
phase: 50-provider-parity-config-driven-model-swap
reviewed: 2026-10-16T00:00:00Z
depth: standard
files_reviewed: 70
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .project/current-exports.txt
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/src/engine/test_support.rs
  - crates/paladin-battalion/src/llm_decision.rs
  - crates/paladin-battalion/src/llm_failure.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-llm/examples/live_vendor_smoke.rs
  - crates/paladin-llm/src/anthropic/adapter.rs
  - crates/paladin-llm/src/cadence.rs
  - crates/paladin-llm/src/compat/engine.rs
  - crates/paladin-llm/src/compat/mod.rs
  - crates/paladin-llm/src/compat/types.rs
  - crates/paladin-llm/src/config/bridge.rs
  - crates/paladin-llm/src/config/layers.rs
  - crates/paladin-llm/src/config/llm.rs
  - crates/paladin-llm/src/config/mod.rs
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
  - crates/paladin-llm/src/pricing.rs
  - crates/paladin-llm/src/provider_factory.rs
  - crates/paladin-llm/src/qwen/adapter.rs
  - crates/paladin-llm/src/redaction.rs
  - crates/paladin-llm/src/registry/mod.rs
  - crates/paladin-llm/src/registry/seeded.rs
  - crates/paladin-llm/src/response_guards.rs
  - crates/paladin-llm/src/sampling.rs
  - crates/paladin-llm/src/services/commissary.rs
  - crates/paladin-llm/src/test_support.rs
  - crates/paladin-llm/src/turn.rs
  - crates/paladin-llm/src/window.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/output/llm_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/model_profile.rs
  - crates/paladin-ports/src/output/paladin_port.rs
  - docs/src/SUMMARY.md
  - docs/src/appendix/provider-expansion.md
  - docs/src/appendix/sentinel.md
  - docs/src/contributing/contributing-providers.md
  - docs/src/user-guides/switching-providers.md
  - examples/vision_analysis.rs
  - examples/vision_battalion.rs
  - src/application/services/paladin/middleware/history.rs
  - src/application/services/paladin/middleware/resilience.rs
  - src/application/services/paladin/paladin_builder.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/config/agent_runtime.rs
  - src/infrastructure/adapters/llm/config_bridge.rs
  - src/infrastructure/cadence.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/lib.rs
  - tests/cli/environment_tests.rs
  - tests/consumer_surface_muster.rs
  - tests/integration/provider_switching_test.rs
  - tests/provider_swap_matrix.rs
  - tests/unit/handoff_service_test.rs
  - tests/unit/llm/anthropic_adapter_test.rs
  - tests/unit/llm/deepseek_adapter_test.rs
findings:
  critical: 2
  warning: 4
  info: 6
  total: 12
status: issues_found
---

# Phase 50: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 70
**Status:** issues_found

## Summary

The phase is, on the whole, carefully built. The credential-handling rules in
`security.instructions.md` hold on every path I traced:

- Every adapter config has a manual `Debug` that routes the key through `RedactedKey`.
- Refusal text goes through `diagnostic_excerpt` (redact, then bound).
- Every HTTP client that carries a credential header uses `Policy::none()`.
- Model ids are shape-checked before they reach a URL (Gemini) or a request body.
- No new `unwrap()`, `expect()` or `panic!` appears in non-test library code.

The `paladin-llm` lib suite (1118 tests, nine adapter features plus `mock` and `vision`) passes.

Two defects are real and were reproduced against the built code:

- **CR-01.** The new typed `Refusal` is retried by the Anthropic and shared-engine retry loops.
- **CR-02.** The streaming paths drop SSE events that are split across network chunks. The phase's new terminal-chunk guarantees (refusal, empty completion, metadata) all sit on top of that parser.

The remaining items are a behavioural regression for agents that use stop words, unbounded retry and backoff knobs, a stale harness whose own guard test fails, and a few quality issues.

Known ledgered items are not re-reported: the two `run_api_wiring` failures (row 73), `total_tokens = prompt + completion` (row 74), OpenAI sending `max_tokens` (row 75), Kimi's fallback profile versus the probe (row 76), and the `new(..)` harmonisation todo.

## Critical Issues

### CR-01: A vendor refusal is retried as if it were transient (billed up to 4 times)

**File:** `crates/paladin-llm/src/anthropic/adapter.rs:677-692` (also `crates/paladin-llm/src/compat/engine.rs:850-862`)

**Issue:** `classify_completion` runs inside the retried `operation` closure for both Anthropic (`parse_response` at line 745) and `CompatEngine` (line 953). Both retry loops hold a hand-written non-retryable set:
`AuthenticationError | InvalidPrompt | EmptyCompletion | UsageLimitExceeded | RateLimitExceeded`.

The new `LlmError::Refusal` is not in either set. A refusal is classified `Transience::Permanent` in the port, in `fallback.rs`, in the doc comment on the variant ("Do not retry the same request against the same provider") and in the conformance case name `refusal_is_typed_and_permanent`. The two adapter loops disagree with all of that.

I reproduced it against `KimiAdapter` (`CompatEngine`, default `max_retries = 3`). A mock returning one HTTP 200 body with `message.refusal` set was hit 4 times, not once, and `generate()` took about 950 ms of back-off before returning the same `Refusal`. Anthropic does the same up to `max_retries` attempts, with 1 s, 2 s sleeps. Each retry re-bills the full prompt, and on a frontier model each can be a multi-minute generation. `EmptyCompletion` was placed in the same set for exactly that reason (D-04), so omitting `Refusal` is an oversight, not a decision.

The conformance case `refusal_is_typed_and_permanent` (`conformance.rs:1275`) mounts its mock with no `.expect(1)`, so it cannot detect this. That is why the suite is green.

**Fix:** Add `Refusal` to both non-retryable sets. Better, derive the early-return decision from the port's own classification so the two cannot drift again:
```rust
// anthropic/adapter.rs execute_with_retry, compat/engine.rs call_api_with_retry
if matches!(
    e,
    LlmError::AuthenticationError(_)
        | LlmError::InvalidPrompt(_)
        | LlmError::EmptyCompletion(_)
        | LlmError::Refusal { .. }
        | LlmError::UsageLimitExceeded { .. }
        | LlmError::RateLimitExceeded { .. }
) {
    return Err(e);
}
```
Then pin the attempt count in the conformance suite: have `refusal_is_typed_and_permanent` register the mock with `.expect(1)` and call `mock.assert_async().await`. Also update the "Non-retryable set" doc comment at `compat/engine.rs:822`.

---

### CR-02: A streamed SSE event split across two network chunks is silently dropped, so the stream can end "successfully" with truncated text

**File:** `crates/paladin-llm/src/anthropic/adapter.rs:834-871`; `crates/paladin-llm/src/gemini/adapter.rs:1193,1586-1620`; `crates/paladin-llm/src/compat/engine.rs:1105`; `crates/paladin-llm/src/openai/adapter.rs:935`; `crates/paladin-llm/src/deepseek/adapter.rs:1043`

**Issue:** Every streaming adapter parses each `bytes_stream()` chunk on its own: `String::from_utf8_lossy(&bytes)`, then `text.lines()`, then `strip_prefix("data: ")`. There is no carry-over buffer for a partial line. An SSE event whose bytes arrive in two TCP/TLS reads is then handled as follows:

- **Anthropic and Gemini.** The first fragment fails JSON parsing and is `continue`d past (Anthropic) or mishandled (Gemini). The second fragment no longer starts with `data: ` and is skipped.
- **Compat engine, OpenAI and DeepSeek.** The first fragment becomes a `ProcessingError` item, and the second is skipped.

This mechanism pre-dates the phase. The phase made it load-bearing. Its new guarantees are decided only at the terminal frame (`message_stop`, `[DONE]`, the `finishReason` frame): the shared refusal and empty-output guard, the held stop reason, the metadata stamp and `run_metadata`. If the dropped event is the terminal-bearing one, the guard never runs. If it is a content delta, the guard sees less text than the model produced and can report a false `EmptyCompletion`, or a false `Ok`.

I reproduced it against `AnthropicAdapter` (mockito `with_chunked_body`). The stream carries the text deltas `"Hello world"` and `" tail"`, and the first delta is split mid-JSON across writes. The caller received a terminal `Some(Stop)` and the assembled text `" tail"`: the words "Hello world" were lost with no `Err` item. The only trace is a `log::warn!`.

`from_utf8_lossy` per chunk has a related defect: a multi-byte character split across a chunk boundary is replaced with U+FFFD, corrupting delta text (and refusal detail).

**Fix:** Buffer bytes across chunks and parse only complete events. Sketch for one shared helper used by all five sites:
```rust
struct SseBuffer { pending: Vec<u8> }
impl SseBuffer {
    /// Feed raw bytes; return every complete `data:` payload (split on a blank line / '\n').
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=pos).collect();
            if let Ok(text) = std::str::from_utf8(&line) {
                if let Some(data) = text.trim_end_matches(['\r', '\n']).strip_prefix("data: ") {
                    out.push(data.to_string());
                }
            }
        }
        out
    }
}
```
Keep the buffer in the `flat_map` closure state next to `StreamCompletion`. Also decide what a stream that reaches EOF with no terminal frame should yield. Today it yields nothing at all. An `Err(ProcessingError("stream ended without a terminal event"))` would make the phase's guard total instead of best-effort. Add a conformance case that splits a terminal frame across two writes.

## Warnings

### WR-01: Explicit `stop_words` are now sent as vendor stop sequences, which defeats the client-side `StopWordDetected` check; and the build refuses them where the framework previously coped

**File:** `src/application/services/paladin/paladin_builder.rs:1235-1247`; `crates/paladin-llm/src/compat/types.rs:31`; `src/application/services/paladin/paladin_execution_service.rs:2151,2430`

**Issue:** In the execution service, `Paladin.data.stop_words` are a forbidden-word detector. `check_stop_words` scans the accumulated output and returns `Err(PaladinError::StopWordDetected)` if a word appears. The same list is also copied into `PromptParameters.stop_sequences` (lines 2843, 3032, 3620).

Before this phase the five `CompatEngine` presets (Kimi, Qwen, Grok, Ollama, OpenAI-compatible) had no `stop` field on `CompatRequest`, so the list was never sent and the client-side check worked. Phase 50 adds `stop` to the wire for all five. A vendor that honours it terminates generation at the stop word and returns the text without it. `check_stop_words` then never fires, and the run silently ends with truncated output instead of the documented error.

The same applies to the four bespoke adapters, which already sent `stop`, but this phase widens it to five more providers.

The builder (`validate`, line 1235) compounds this. For any model whose profile is `StopSupport::NotSent` it now refuses the build. That covers every Grok model, `gpt-5.4-nano`, and unseeded Grok through the fallback. Those agents previously built and ran, with the client-side check still working. That is a regression against the phase goal that one agent definition runs on any provider by configuration alone.

**Fix:** Decide the semantics explicitly. Either:
1. Do not forward `Paladin.data.stop_words` as wire `stop_sequences` at all, since the framework enforces them itself.
2. Or keep forwarding them, but do not treat an unsupported `stop` as a build refusal when `check_stop_words` still enforces the list client-side.

If option 2 is chosen, document in ADR-0061 that a wire stop truncates rather than aborts. Add a test that an agent with stop words on a stop-capable model still raises `StopWordDetected`.

### WR-02: Operator-settable `max_retries` and `timeout_seconds` are unvalidated; large values overflow or sleep for hours, and `0` timeout is accepted

**File:** `crates/paladin-llm/src/openai/adapter.rs:812`; `crates/paladin-llm/src/compat/engine.rs:867`; `crates/paladin-llm/src/deepseek/adapter.rs:838`; `crates/paladin-llm/src/config/layers.rs:295-318`

**Issue:** PARITY-03 made `max_retries` a knob (block, env var `OPENAI_MAX_RETRIES`, `with_max_retries`). It was a hard-coded `3` before. Three retry loops compute their back-off with unchecked exponentiation:

- `openai/adapter.rs:812`: `base_delay * 2_u32.pow(attempt)`. At `attempt >= 32` this overflows `u32`. It panics in debug builds. In release it wraps, so the delay collapses to 0 and the loop retries with no sleep.
- `compat/engine.rs:867` and `deepseek/adapter.rs:838`: `Duration::from_millis(100 * 2_u64.pow(attempt))` is uncapped. `max_retries = 15` sleeps about 54 minutes on the last attempt. `attempt` of 57 or more overflows.
- Nothing bounds `max_retries` (`u32::MAX` is accepted) or rejects `timeout_seconds = 0`. A zero `reqwest` timeout fails every request immediately.

Anthropic and Gemini cap their delay at 10 s and are not affected.

**Fix:** Clamp the exponent and cap the delay in all three loops, and validate the knobs at construction:
```rust
let exp = attempt.min(16);
let backoff = Duration::from_millis((100u64 << exp).min(10_000));
```
In `*Config::validate()` (or `layers::resolve`), reject `timeout_seconds == 0` and put a documented ceiling on `max_retries` (for example 10), with an error naming the provider and the key.

### WR-03: `live_vendor_smoke` restates stale model constants and its own drift test fails

**File:** `crates/paladin-llm/examples/live_vendor_smoke.rs:186-209,1981-2007`

**Issue:** The harness hard-codes `ANTHROPIC_MODELS` (eight retired claude-2/3 ids), `DEEPSEEK_MODELS` (`deepseek-chat`, `deepseek-coder`), `ANTHROPIC_DEFAULT_MODEL = "claude-3-5-sonnet-20241022"` and `DEEPSEEK_DEFAULT_MODEL = "deepseek-chat"`.

Since plan 50-06 (commit 52ac8f09) both adapters return their seeded tables (`["claude-haiku-5-5"]` and `["deepseek-flash"]`) from `get_available_models`, and their defaults are `claude-haiku-5-5` and `deepseek-flash`.

The unit test `the_static_model_lists_match_the_adapters`, whose purpose is to keep those constants in step, therefore fails. I ran it:
`cargo test -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible the_static_model_lists` reports `assert_eq!(a, ANTHROPIC_MODELS)` failing.

Cargo does not run example test targets by default, so CI never sees it. The dry-run output meanwhile reports retired model ids for two of the nine providers, and the curated-list discrimination for those two providers compares against the wrong list. This is the harness PARITY-07 relies on as a guard against vacuous passes.

**Fix:** Import the real constants (`paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL`, `paladin_llm::deepseek::adapter::DEEPSEEK_DEFAULT_MODEL`) instead of restating them. Derive the "static list" from the adapter (`get_available_models()`) instead of a hand-copy. Run the example's unit tests in CI (`cargo test -p paladin-llm --examples --features ...`), or move the drift test into the crate's `tests/`.

### WR-04: `LlmProviderConfig` redacts the key in `Debug` but still serialises it in plaintext

**File:** `crates/paladin-llm/src/config/llm.rs:24-26,160-161`

**Issue:** The phase adds a redacting manual `Debug` (D-00a) and a doc comment explaining why. `LlmProviderConfig` (and through it `LlmConfig`, `Settings` and `AgentRuntimeDeps`) still `#[derive(Serialize)]` with `api_key: String` and no `skip_serializing`.

`security.instructions.md` states the requirement as: "no config type carrying one is `Debug`-formatted **or serialised outward**". Today I found no call site that serialises `Settings` outward, so this is a latent exposure rather than a live leak. Any future config-dump endpoint, `--print-config` flag or serde-based snapshot would emit every provider key.

**Fix:** Add a serialiser that never writes the secret, so the type cannot leak by construction:
```rust
#[serde(default, serialize_with = "serialize_redacted_key")]
pub api_key: String,
// fn serialize_redacted_key<S: Serializer>(k: &str, s: S) -> Result<S::Ok, S::Error> {
//     s.serialize_str(if k.is_empty() { "" } else { "<redacted>" })
// }
```
Check that nothing round-trips the block through `Serialize` then `Deserialize` before choosing this. Otherwise use `skip_serializing`.

## Info

### IN-01: Stale doc comment on Gemini's retry count

**File:** `crates/paladin-llm/src/gemini/adapter.rs:1095-1097`

**Issue:** `generate_stream`'s doc says it uses "the same `max_retries` literal (`3`)". It now reads `self.config.max_retries`, which is configurable.

**Fix:** Say "the same `config.max_retries`".

### IN-02: Application layer imports adapter-crate constants for metadata key names

**File:** `src/application/services/paladin/paladin_execution_service.rs:93-95`

**Issue:** The execution service imports `paladin_llm::registry::UNPROFILED_MODEL_METADATA_KEY` and `paladin_llm::sampling::PARAMS_OMITTED_METADATA_KEY`. This adds to the existing `SERVED_BY_METADATA_KEY` import. The `paladin.*` key names are a contract between ports and every consumer, so they belong in `paladin-ports` (beside `LlmResponse.metadata`), not in the adapter crate. CLAUDE.md requires dependencies to flow inward only. Precedent exists, so this is debt rather than a new violation.

**Fix:** Move the three key constants into `paladin_ports::output::llm_port` (or a small `metadata_keys` module) and re-export them from `paladin-llm` for compatibility.

### IN-03: `FallbackLlmAdapter::from_steps` validates emptiness after trimming but sends the untrimmed model

**File:** `crates/paladin-llm/src/fallback.rs:369-371`

**Issue:** `step.model.trim().is_empty()` is rejected, but a model such as `" kimi-k2 "` is stored and sent verbatim. The config path (`resolve_steps`) trims first, a direct caller does not. The model also keys Cadence pacing, so a stray space creates a separate gate.

**Fix:** Store `step.model.trim().to_string()`, or reject a model that differs from its trim.

### IN-04: Construction-time config errors are reported as `LlmError::AuthenticationError`

**File:** `crates/paladin-llm/src/anthropic/adapter.rs:343-348` (same pattern in the other eight adapters)

**Issue:** An invalid `models` override or a `context_window` below a seeded maximum output is mapped to `AuthenticationError("Invalid <Provider> configuration: ...")`. The existing `validate()` errors already used that variant, but the new override and window failures are not credential problems. Anything keyed on `AuthenticationError` (a circuit breaker, an alert, a rotate-the-key hint) will mislabel them.

**Fix:** Use a configuration-shaped variant (`InvalidPrompt` is the nearest, or add a `Configuration(String)` variant) for the registry errors.

### IN-05: Model ids are echoed raw into `warn!` lines and into unbounded "already warned" sets

**File:** `crates/paladin-llm/src/sampling.rs:196-206,262-273`; `crates/paladin-llm/src/registry/mod.rs:570-583`

**Issue:** `OmissionLog`, `FormatLog` and `ModelRegistry::note_unprofiled` format `request.model` raw into log lines. The module docs claim the warn text is bounded and never carries a body. The factory path validates model ids (`layers::validate_model_id`), but `LlmRequest.model` set by a direct caller is not validated. A model string with newlines or very long text goes straight into the log. The `HashSet<(String, …)>` keys also grow with every distinct model id seen.

**Fix:** Run the model through the same escape-and-bound helper used by `layers::subject` before logging, and cap the "already warned" sets (stop recording past a fixed size).

### IN-06: `paladin.unprofiled_model` merging splits on commas, but model ids may contain commas

**File:** `src/application/services/paladin/paladin_execution_service.rs:1146-1158`

**Issue:** `merge_run_metadata` treats `paladin.unprofiled_model` as a comma-separated list. A single model id is one value, and `validate_model_id` does not reject `,`. An id like `foo,bar` would be reported as two models.

**Fix:** Either reject `,` in `validate_model_id`, or merge `unprofiled_model` as a set of whole values rather than a split list.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
