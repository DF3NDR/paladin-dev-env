---
status: diagnosed
trigger: "G-50-5b (openai-max-completion-tokens): The OpenAI adapter sends `max_tokens`, which the seeded default model `gpt-5.4-nano` rejects with HTTP 400 (WINDOWS.md row 75). The owner chose to fix it by having the OpenAI adapter always send `max_completion_tokens`."
created: 2026-10-17T16:20:00Z
updated: 2026-10-17T16:50:00Z
goal: find_root_cause_only
---

## Current Focus

bug_class: Bohrbug (deterministic; Data Shape / API Contract -- wrong wire field name)
hypothesis: CONFIRMED -- private `OpenAIRequest` (openai/adapter.rs:222-248) serializes the output-token limit under the serde field name `max_tokens` (line 229, no rename); build_request (529-578) is its only constructor and feeds generate (1022), generate_stream (1062) and json_object/json_schema requests alike, so every OpenAI chat-completions request with a limit (always, by default) carries `max_tokens`, which gpt-5.4-nano rejects.
test: done -- existing tests + scratch mockito capture
expecting: n/a
next_action: return ROOT CAUSE FOUND to caller (find_root_cause_only); fix belongs to the G-50-5b gap-closure plan

reasoning_checkpoint:
  hypothesis: "gpt-5.4-nano returns HTTP 400 because OpenAIRequest's field `max_tokens: Option<u32>` serializes as wire key `max_tokens`, and that model only accepts `max_completion_tokens`"
  confirming_evidence:
    - "scratch capture: gpt-5.4-nano generate / generate_stream / json_schema bodies all carry max_tokens=1000, max_completion_tokens absent"
    - "50-SEED-PROBE.md gpt-5.4-nano probe 11 (max_tokens alone) REJECTED 400 unsupported_parameter; probe 12 (max_completion_tokens alone) ACCEPTED 200"
    - "existing unit tests assert body[\"max_tokens\"] and pass"
  falsification_test: "a captured body for gpt-5.4-nano containing max_completion_tokens and no max_tokens would refute it -- not observed"
  fix_rationale: "renaming the wire key (serde rename or field rename) changes exactly the field the vendor rejects; the domain/config name stays max_tokens"
  blind_spots: "max_completion_tokens acceptance is measured only for gpt-5.4-nano and gpt-4o-mini; older ids used in live tests (gpt-3.5-turbo, gpt-4, gpt-4o) and any non-OpenAI host an operator points OPENAI_BASE_URL at are unmeasured"
  candidate_causes:
    - "code: OpenAIRequest serde field name `max_tokens` (CONFIRMED)"
    - "config: OpenAIConfig::new/from_env default max_tokens Some(4096) + PromptParameters default Some(1000) guarantee the key is always present (contributing, not causal)"
    - "environment/vendor: OpenAI newer (gpt-5 family) models deprecate max_tokens for max_completion_tokens (external contract change; the trigger)"
    - "test/gate: live smoke defaults to gpt-4o-mini (accepts max_tokens), so no live gate exercised gpt-5.4-nano through the adapter"
  and_gate: "yes -- 400 requires (adapter emits `max_tokens`) AND (target model rejects it, e.g. gpt-5.4-nano) AND (a limit is present, which defaults guarantee). Only the first is in our control and is the fix point."

## Symptoms

expected: The OpenAI adapter sends the output-token limit as `max_completion_tokens`, so `gpt-5.4-nano` accepts requests built from `OpenAIConfig::new` / `OpenAIConfig::from_env` defaults (which set max_tokens = 4096).
actual: crates/paladin-llm/src/openai/adapter.rs (~line 540-563) serializes `max_tokens`; 50-SEED-PROBE.md probe 11 for `gpt-5.4-nano` shows HTTP 400 "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead." Probe 12 with `max_completion_tokens` was accepted.
errors: HTTP 400 invalid_request_error, param max_tokens, code unsupported_parameter.
reproduction: Test 5 in .planning/phases/50-provider-parity-config-driven-model-swap/50-UAT.md; WINDOWS.md row 75; 50-SEED-PROBE.md "Probes: gpt-5.4-nano".
started: Carried unfixed from plans 50-06, 50-08, 50-09; discovered during Phase 50 UAT (2026-10-17).

## Eliminated

- hypothesis: another adapter (DeepSeek, compat presets kimi/qwen/grok/ollama/openai-compatible) shares OpenAIRequest, so a rename would leak to them
  evidence: OpenAIRequest is a private struct in openai/adapter.rs with one constructor (build_request); DeepSeek uses its own DeepSeekRequest, the five compat presets use CompatRequest (compat/types.rs). No cross-module reference exists.
  timestamp: 2026-10-17T16:29:00Z

- hypothesis: the 400 comes from the probe sending BOTH max_tokens and max_completion_tokens (a "doubled cap" artefact), not from max_tokens itself
  evidence: live_vendor_smoke.rs request_body replaces the baseline cap for probe 11 (unit test the_max_tokens_probe_replaces_the_baseline_cap_instead_of_doubling_it asserts max_completion_tokens absent when max_tokens is set); the vendor message names max_tokens as unsupported for the model.
  timestamp: 2026-10-17T16:30:00Z

- hypothesis: the default config value (OpenAIConfig.max_tokens = Some(4096)) is the root cause and setting None avoids it
  evidence: scratch repro shows PromptItem::new's default PromptParameters.max_tokens = Some(1000) still reaches the wire as `max_tokens`; the default only guarantees a key is present, the key NAME is the defect. Config None is at best a workaround that also removes the output cap.
  timestamp: 2026-10-17T16:45:00Z

## Evidence

- timestamp: 2026-10-17T16:22:00Z
  checked: knowledge base (.planning/debug/knowledge-base.md) and MemPalace
  found: no knowledge-base.md exists; .planning/debug/ was empty; no prior resolved sessions
  implication: no known-pattern candidate; proceed with open investigation

- timestamp: 2026-10-17T16:25:00Z
  checked: crates/paladin-llm/src/openai/adapter.rs lines 222-248 (struct OpenAIRequest)
  found: private `struct OpenAIRequest` (derive Serialize, no rename attributes) has field `max_tokens: Option<u32>` with `skip_serializing_if = "Option::is_none"` at line 229. serde emits the Rust field name verbatim, so the wire key is `max_tokens`.
  implication: the wire key name is fixed by this one struct field

- timestamp: 2026-10-17T16:26:00Z
  checked: adapter.rs build_request (lines 529-578) and callers
  found: build_request is the ONLY constructor of OpenAIRequest (line 559). Line 540 `let max_tokens = params.max_tokens.or(self.config.max_tokens);` then line 563 `max_tokens,` into the body. Called from LlmPort::generate (line 1022, stream=false) and LlmPort::generate_stream (line 1062, stream=true). JSON-mode/json_schema requests use the same builder (response_format field). There is NO tool-calling path (declared_capabilities: supports_tool_calling=false, supports_function_calling=false; no `tools` field on OpenAIRequest).
  implication: one serde field change covers non-streaming, streaming and structured-output paths at once

- timestamp: 2026-10-17T16:27:00Z
  checked: OpenAIConfig::new / from_env (adapter.rs 177-200), OPENAI_DEFAULT_MAX_TOKENS (line 43)
  found: both set `max_tokens: Some(OPENAI_DEFAULT_MAX_TOKENS)` = Some(4096). So with no caller max_tokens, every request still carries a token-limit key (4096).
  implication: defaults alone guarantee the 400 on gpt-5.4-nano; the caller cannot avoid it short of setting config.max_tokens = None AND leaving PromptParameters.max_tokens unset

- timestamp: 2026-10-17T16:28:00Z
  checked: crates/paladin-llm/src/openai/vision.rs lines 109-116, 371-381, 482-491
  found: SECOND OpenAI wire struct `OpenAIVisionApiRequest` with non-optional `max_tokens: usize` (always serialized), built in VisionCapableLlm::generate_with_vision (VisionConfig default 4096) and VisionPort::analyze_image (`max_tokens.unwrap_or(1000)`), POSTed to `{base_url}/chat/completions`. Guarded by VISION_MODELS = gpt-4o, gpt-4o-mini, gpt-4-vision-preview, gpt-4-turbo, gpt-4-turbo-2024-04-09 -- gpt-5.4-nano cannot reach it today.
  implication: not the cause of the reported 400, but a second OpenAI site that also sends `max_tokens`; "always send max_completion_tokens" should cover it for consistency (gpt-4o-mini accepted max_completion_tokens in probe 12)

- timestamp: 2026-10-17T16:29:00Z
  checked: who reuses OpenAIRequest -- grep across crates/paladin-llm/src
  found: OpenAIRequest is private to openai/adapter.rs; no other module references it. DeepSeek has its own private `DeepSeekRequest` (deepseek/adapter.rs:216-223, `max_tokens`). Kimi/Qwen/Grok/Ollama/openai-compatible all share `CompatRequest` (compat/types.rs:13-20, `pub max_tokens`) via CompatEngine. Gemini uses `maxOutputTokens`; Anthropic its own `max_tokens: u32`. config/bridge.rs only references OPENAI_DEFAULT_MAX_TOKENS and OpenAIConfig, not the wire struct.
  implication: renaming the OpenAIRequest wire key cannot leak to DeepSeek/compat presets; they keep `max_tokens`

- timestamp: 2026-10-17T16:30:00Z
  checked: 50-SEED-PROBE.md "## OpenAI" and "Probes: gpt-4o-mini" / "Probes: gpt-5.4-nano"; examples/live_vendor_smoke.rs 868-997, 2125-2131
  found: OpenAI section states "every call carries `max_completion_tokens` = 256" (baseline). Probe 11 REPLACES the baseline cap with max_tokens=64 (test the_max_tokens_probe_replaces_the_baseline_cap_instead_of_doubling_it). gpt-4o-mini: probe 11 max_tokens ACCEPTED (200), probe 12 max_completion_tokens=64 ACCEPTED (200). gpt-5.4-nano: probe 11 REJECTED (400) "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead." (param max_tokens, code unsupported_parameter); probe 12 max_completion_tokens=64 ACCEPTED (200). All other probes on both models (1-10, 13, 14) also carried max_completion_tokens=256 and were accepted except the `stop` probes on nano.
  implication: both seeded OpenAI models accept max_completion_tokens alone; switching unconditionally is safe for the probed set

- timestamp: 2026-10-17T16:31:00Z
  checked: why UAT test 4 (live_vendor_smoke plain mode) passed despite the bug
  found: live_vendor_smoke.rs:200 OPENAI_PROBE_DEFAULT_MODEL = "gpt-4o-mini"; the plain smoke drives the real OpenAIAdapter (build_openai, line 234-236) with gpt-4o-mini, which accepts max_tokens (probe 11 ACCEPTED). gpt-5.4-nano is seeded first in seeded::OPENAI (registry/seeded.rs:99-110) and is the "operator-identified current model" (adapter.rs:1194).
  implication: the smoke never exercised the adapter against gpt-5.4-nano, so no live gate caught it; wiremock tests assert `max_tokens` so they lock in the defect

- timestamp: 2026-10-17T16:32:00Z
  checked: .planning/WINDOWS.md row 75
  found: "the OpenAI adapter sends max_tokens, which gpt-5.4-nano (the probed default model) rejects with HTTP 400 ... The ModelProfile has no field for the token-limit parameter name. Carried unfixed from 50-06, 50-08 and 50-09; documented in the Switching providers page and the CHANGELOG known limitations." status open
  implication: confirms the deviation was known and ledgered; owner has now chosen option (a) unconditional rename

- timestamp: 2026-10-17T16:40:00Z
  checked: ran existing tests `cargo test -p paladin-llm --lib` -- knob_tests::openai_max_tokens_default_reaches_the_wire, response_format_wiring::openai_request_without_response_format_is_byte_identical_to_today, parameter_policy::openai_unset_sampling_values_are_absent, parameter_policy::openai_stream_follows_the_profile
  found: all 4 PASS. They assert body["max_tokens"] == 4096/512/2048/777, `body.get("max_tokens").is_none()` for config None, and an exact key set {model, messages, temperature, max_tokens, top_p, stream}.
  implication: the test suite actively locks in the defective wire key; these must flip to max_completion_tokens (and assert max_tokens absent)

- timestamp: 2026-10-17T16:45:00Z
  checked: scratch reproduction (outside repo) /tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/wire-probe -- OpenAIAdapter::new(OpenAIConfig::new(..)) against a mockito /chat/completions capture, models gpt-5.4-nano and gpt-4o-mini, paths generate / generate_stream / generate with ResponseFormat::JsonSchema
  found: every captured body: `max_tokens=1000 max_completion_tokens=null` (1000 = PromptItem::new's default PromptParameters.max_tokens, which wins over the 4096 config default via `params.max_tokens.or(self.config.max_tokens)`). stream=true body and json_schema body identical in this respect.
  implication: DIRECT OBSERVATION: for the seeded model gpt-5.4-nano the adapter emits `max_tokens` on all three request paths, never `max_completion_tokens` -- exactly the body probe 11 showed the vendor rejects. Note also that a default PromptItem carries max_tokens=Some(1000), so even OpenAIConfig.max_tokens=None does not avoid the key on the normal PaladinBuilder path.

- timestamp: 2026-10-17T16:48:00Z
  checked: other OpenAI-wire references in tests/docs (grep across crates, tests/, src/, docs/, CHANGELOG.md, MIGRATION.md)
  found: (a) adapter.rs wire-key assertions at lines 1890, 2382-2383, 2401, 2494, 3021, 3026, 3033, 3035-3042; (b) doc comments at 38-43, 109-114, 521-524, 538-539 say "sends `max_tokens`"/"no `max_tokens` key"; (c) lines 2633 and 3154 assert the domain/config name `max_tokens` in a check_max_tokens error and in OpenAIConfig Debug -- these name the Paladin parameter/config field and do NOT need changing; (d) docs/src/user-guides/switching-providers.md:189-191 known-limit bullet; (e) CHANGELOG.md:1255-1257 "Known limitations" item (3) under [Unreleased]; (f) .planning/WINDOWS.md row 75 (open). tests/integration/openai_*.rs are live/env-gated tests with no body matchers. examples/live_vendor_smoke.rs 2098/2125-2131 assert the probe harness's own raw bodies, not the adapter -- unchanged. src/application/cli/commands/{setup_check,onboarding}.rs `"max_tokens": 1` are Anthropic Messages bodies, not OpenAI.
  implication: complete update list for the fix plan

## Resolution

root_cause: "crates/paladin-llm/src/openai/adapter.rs:229 -- the private wire struct OpenAIRequest declares `max_tokens: Option<u32>` with no serde rename, so build_request (the single constructor, line 559-571, used by generate, generate_stream and every response_format variant) emits the output-token limit as `max_tokens`; gpt-5.4-nano rejects that key (HTTP 400 unsupported_parameter) and requires `max_completion_tokens`. The defaults (OpenAIConfig max_tokens Some(4096), PromptParameters max_tokens Some(1000)) make the key present on every request. Secondary site: openai/vision.rs:112 OpenAIVisionApiRequest.max_tokens (usize, always sent), currently unreachable for gpt-5.4-nano because VISION_MODELS lists only gpt-4o/gpt-4-turbo ids."
fix: "(not applied -- find_root_cause_only)"
verification: "(not applied)"
files_changed: []
