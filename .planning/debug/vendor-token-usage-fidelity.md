---
status: diagnosed
trigger: "G-50-5a (vendor-token-usage-fidelity): Token usage mapping in paladin-llm discards each vendor's own reported total and may drop billable token categories, so Treasurer pricing and allowance checks can understate what the vendor actually bills (WINDOWS.md row 74)."
created: 2026-10-17T00:00:00Z
updated: 2026-10-17T01:00:00Z
goal: find_root_cause_only
---

## Current Focus

bug_class: Bohrbug (deterministic data-contract mismatch; reproduced every run)
hypothesis: CONFIRMED - TokenUsage's inclusive contract (prompt = all billed input, completion = all billed output) is enforced by nothing at the adapter boundary; map_compat_usage applies OpenAI semantics to all five compat presets, so Grok (completion_tokens EXCLUDES reasoning_tokens; vendor total = prompt + completion + reasoning) loses every reasoning token from total_tokens, and the vendor total that would expose the gap is discarded by design (D-02, #[allow(dead_code)]). cost_of_call's reasoning.min(completion) clamp then silently prices the invariant violation down. Gemini omits toolUsePromptTokenCount (latent today: no tools sent). Independently, every adapter drops usage entirely on classify_completion errors.
test: out-of-tree repro through real GrokAdapter/GeminiAdapter + cost_of_call (done)
expecting: n/a - diagnose-only
next_action: Return ROOT CAUSE FOUND to caller (goal find_root_cause_only); no source change, no commit
reasoning_checkpoint:
  hypothesis: "Grok/xAI calls under-report output tokens because map_compat_usage copies xAI's completion_tokens (which excludes reasoning_tokens) into TokenUsage.completion_tokens and recomputes total_tokens = prompt + completion, discarding the vendor total_tokens that includes reasoning; cost_of_call then clamps reasoning to completion and prices the remainder away."
  confirming_evidence:
    - "Repro through the real GrokAdapter: vendor 663/50/reasoning 124/total 837 -> TokenUsage total 713, reasoning 124 > completion 50"
    - "cost_of_call on that TokenUsage = 1_163_000 nanos vs 2_403_000 nanos the vendor bills at the same price row (52% under)"
    - "Vertex AI Grok partner docs and Glama grok-api docs both show total = prompt + completion + reasoning for xAI chat completions"
    - "CompatUsage.total_tokens is #[allow(dead_code)]; compat_usage_fields_are_copied_exactly pins total = prompt + completion against a disagreeing vendor 9999"
  falsification_test: "A live Grok 4.x response whose total_tokens == prompt_tokens + completion_tokens while completion_tokens_details.reasoning_tokens > 0 would disprove the xAI-exclusive semantics (the 2026-10-15 probe captured field names only, not values)."
  fix_rationale: "Parse the vendor total and reconcile per wire: where a vendor reports reasoning outside completion (xAI) or a billed prompt category outside promptTokenCount (Gemini toolUsePrompt), fold it into the inclusive parent so prompt + completion == vendor total; surface any residual mismatch instead of discarding it. That keeps D-02 while making the vendor total the check on it."
  blind_spots: "xAI exclusivity is secondary-sourced (docs.x.ai fetch blocked by proxy, probe recorded no values); Kimi and Qwen reasoning/cache_write containment unmeasured; Ollama/openai-compatible servers vary by backend; Grok non-token charges (num_sources_used live search, server-side tools) are only visible through cost_in_usd_ticks."
  candidate_causes:
    - "code: map_compat_usage one-size-fits-all OpenAI-semantics mapping for all compat presets (Grok)"
    - "code: GeminiUsageMetadata lacks toolUsePromptTokenCount (latent)"
    - "design/contract: D-02 recompute + dead_code vendor total = no reconciliation signal; TokenUsage has no slot for vendor total or vendor-reported cost"
    - "data/vendor semantics: completion_tokens meaning differs per vendor (xAI excludes reasoning)"
    - "control flow: classify_completion error short-circuit drops usage of billed-but-failed calls"
  and_gate: "yes for the money channel - the Grok cost undercount needs the mapper copying exclusive completion_tokens AND cost_of_call clamping reasoning to completion (without the clamp the invariant violation would have been visible); the total_tokens undercount needs only the mapper. Detection failure needs the vendor-total discard AND OpenAI-only conformance fixtures."

## Symptoms

expected: Every billable token category a vendor reports reaches a TokenUsage sub-count (prompt, completion, cache-read, cache-write, reasoning), so Treasurer pricing (`cost_of_call`, crates/paladin-core/src/platform/container/cost.rs:375) and the web allowance check (crates/paladin-web/src/agent_controller.rs:632, `usage.total_tokens > budget.max_tokens`) see the full billed usage. The vendor's own total is parsed and reconciled against prompt + completion instead of silently discarded. D-02 invariant `total_tokens == prompt_tokens + completion_tokens` is KEPT.
actual: Owner: "don't dismiss the provider's total token counts -- those are what the vendor charges for, and the Treasurer and Commissary depend on these counts." compat/engine.rs `map_compat_usage` ignores vendor total by design; Gemini adapter marks `totalTokenCount` dead_code and does not parse `toolUsePromptTokenCount`. Commissary appears to use a pre-call TokenCounterPort estimate only (to confirm).
errors: None (silent undercount).
reproduction: Test 5 in .planning/phases/50-provider-parity-config-driven-model-swap/50-UAT.md; plan 50-12 SUMMARY deviation 1; WINDOWS.md row 74.
started: Discovered during Phase 50 UAT (2026-10-17).

## Eliminated

- hypothesis: OpenAI chat mapper drops billed tokens from prompt/completion
  evidence: OpenAI prompt_tokens includes cached+audio and completion_tokens includes reasoning+audio+rejected_prediction; total == prompt + completion. Only the audio-rate split is unparsed (pricing granularity, not count).
  timestamp: 2026-10-17T00:15:00Z
- hypothesis: Anthropic mapper drops cache read/write tokens from prompt_tokens
  evidence: map_claude_usage adds cache_read_input_tokens + cache_creation_input_tokens to input_tokens (adapter.rs:1116-1124); test asserts 85+512+128 = 725.
  timestamp: 2026-10-17T00:17:00Z
- hypothesis: DeepSeek mapper drops cache-miss tokens
  evidence: DeepSeek prompt_tokens = prompt_cache_hit_tokens + prompt_cache_miss_tokens; mapper copies prompt_tokens and uses hit as cache_read.
  timestamp: 2026-10-17T00:19:00Z
- hypothesis: Gemini mapper drops thinking tokens (Gemini reports thoughtsTokenCount outside candidatesTokenCount)
  evidence: map_gemini_usage adds thoughts into completion_tokens (adapter.rs:1369-1372); repro shows completion=30 for candidates 20 + thoughts 10.
  timestamp: 2026-10-17T00:40:00Z
- hypothesis: The Commissary consumes response TokenUsage and is mis-fed by the mapper
  evidence: commissary.rs reads only TokenCounterPort (pre-call estimate) and the declared window; LlmResponse appears only in #[cfg(test)] stubs.
  timestamp: 2026-10-17T00:52:00Z

## Evidence

- timestamp: 2026-10-17T00:00:00Z
  checked: .planning/WINDOWS.md row 74, 50-12-SUMMARY.md deviation 1, STATE.md line 546
  found: Row 74 (open) records "vendor total_tokens not copied into TokenUsage (D-02 invariant kept; plan truth 'never recomputed' unimplemented)". 50-12 SUMMARY says compat, OpenAI and DeepSeek mappers all comment the vendor total is "deliberately unread"; test compat_usage_fields_are_copied_exactly asserts total = prompt + completion even when vendor reports 9999 vs 1801.
  implication: The vendor-total discard is a deliberate, tested design choice, not an accident; the question is whether it hides billed tokens.

- timestamp: 2026-10-17T00:10:00Z
  checked: crates/paladin-core/src/platform/container/token_usage.rs (full)
  found: TokenUsage has 6 fields (prompt, completion, total, cache_read?, cache_write?, reasoning?). Rustdoc contract - prompt_tokens is EVERY billed input token (cache read+write included), completion_tokens is EVERY output token (reasoning included); total == prompt + completion always; cache_read+cache_write <= prompt; reasoning <= completion. No field carries a vendor total, a vendor cost, audio/image modality split, cache-write TTL split, tool-use prompt tokens, or per-request billable units (search sources). Add/Sum recompute total.
  implication: The type is only correct if every mapper folds every billed category into prompt or completion. Any mapper that copies a vendor count which EXCLUDES a billed category silently loses it, and the D-02 recompute erases the only signal (vendor total) that would reveal the loss.

- timestamp: 2026-10-17T00:12:00Z
  checked: 50-SEED-PROBE.md observed usage field names per provider/model (2026-10-15 live probe; names only, no values)
  found: OpenAI - prompt_tokens, completion_tokens, total_tokens, prompt_tokens_details.{cached_tokens,audio_tokens}, completion_tokens_details.{reasoning_tokens,audio_tokens,accepted_prediction_tokens,rejected_prediction_tokens}. Anthropic (claude-haiku-5-5) - input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens, cache_creation.{ephemeral_5m_input_tokens,ephemeral_1h_input_tokens}, output_tokens_details.thinking_tokens, service_tier, inference_geo, speed (NO vendor total). DeepSeek - prompt_tokens, completion_tokens, total_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_tokens_details.cached_tokens, completion_tokens_details.reasoning_tokens (deepseek-flash). Kimi - prompt_tokens, completion_tokens, total_tokens, prompt_tokens_details.{cached_tokens,cache_write_tokens}, completion_tokens_details.reasoning_tokens, top-level cached_tokens (kimi-k2.6). Qwen - prompt_tokens, completion_tokens, total_tokens, prompt_tokens_details.{cached_tokens,text_tokens}, completion_tokens_details.{reasoning_tokens,text_tokens}. Grok (4.3/4.5/4.6) - prompt_tokens, completion_tokens, total_tokens, prompt_tokens_details.{text,audio,image,cached}_tokens, completion_tokens_details.{reasoning,audio,accepted_prediction,rejected_prediction}_tokens, cost_in_usd_ticks, num_sources_used. Gemini - promptTokenCount, candidatesTokenCount, thoughtsTokenCount, totalTokenCount, promptTokensDetails, serviceTier. Ollama / openai-compatible not probed.
  implication: The probe recorded NAMES only, so whether vendor total == prompt + completion cannot be read from it; it must come from vendor docs/semantics. Grok returns a vendor-computed cost (cost_in_usd_ticks) and per-request billable units (num_sources_used) that nothing parses.

- timestamp: 2026-10-17T00:15:00Z
  checked: OpenAI chat mapper crates/paladin-llm/src/openai/adapter.rs:345-376 (OpenAIUsage) and :761-776 (map_usage); used by non-streaming :1047 and streaming terminal :981-982
  found: Reads prompt_tokens, completion_tokens, prompt_tokens_details.cached_tokens -> cache_read, completion_tokens_details.reasoning_tokens -> reasoning. total_tokens deserialised but #[allow(dead_code)] "Deliberately unread". Not deserialised - audio_tokens (prompt + completion), accepted_prediction_tokens, rejected_prediction_tokens.
  implication: OpenAI documents prompt_tokens as inclusive of cached+audio and completion_tokens as inclusive of reasoning+audio+rejected_prediction, and total = prompt + completion, so no billed TOKENS are lost from totals; only the audio-rate split is lost (priced at text rate). Vendor total discarded with no reconciliation.

- timestamp: 2026-10-17T00:17:00Z
  checked: Anthropic mapper crates/paladin-llm/src/anthropic/adapter.rs:1070-1170 (ClaudeUsage, map_claude_usage, merge_claude_stream_usage)
  found: prompt_tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens (D-20 correction); completion = output_tokens; cache_read, cache_write, thinking_tokens -> reasoning mapped. Not deserialised - cache_creation.ephemeral_5m_input_tokens / ephemeral_1h_input_tokens, service_tier, speed, inference_geo, server_tool_use. Anthropic sends no vendor total.
  implication: Totals are complete for Anthropic. Lost: the 5m-vs-1h cache-write split (1h writes bill at a higher multiplier than 5m) and service_tier/speed pricing modifiers - pricing fidelity, not token count.

- timestamp: 2026-10-17T00:19:00Z
  checked: DeepSeek mapper crates/paladin-llm/src/deepseek/adapter.rs:309-381, error path :940-950
  found: Reads prompt_tokens, completion_tokens, prompt_cache_hit_tokens -> cache_read, completion_tokens_details.reasoning_tokens -> reasoning. total_tokens is read ONLY by annotate_with_usage (folded into an error string). prompt_cache_miss_tokens and prompt_tokens_details.cached_tokens not read.
  implication: DeepSeek prompt_tokens = hit + miss, so totals are complete on success. On a classify_completion error (empty completion / refusal / content filter) the whole usage is turned into prose inside LlmError and never reaches TokenUsage - billed tokens with no TokenUsage at all.

- timestamp: 2026-10-17T00:21:00Z
  checked: Compat engine crates/paladin-llm/src/compat/types.rs:116-155 (CompatUsage) and compat/engine.rs:641-658 (map_compat_usage), non-streaming :965, streaming terminal :1158-1159; kimi/qwen/grok/ollama/openai_compatible adapters all delegate to CompatEngine (no own usage mapping)
  found: Reads prompt_tokens, completion_tokens, prompt_tokens_details.{cached_tokens,cache_write_tokens}, completion_tokens_details.reasoning_tokens. total_tokens Option, #[allow(dead_code)]. Not deserialised - cost_in_usd_ticks, num_sources_used (Grok), *_tokens_details.{text,audio,image}_tokens, accepted/rejected_prediction_tokens, top-level cached_tokens (Kimi). Test compat_usage_fields_are_copied_exactly (engine.rs:3346) pins total = 1234+567 when vendor says 9999.
  implication: One mapper serves five providers with differing semantics for completion_tokens; correctness depends on every one of them reporting completion_tokens INCLUSIVE of reasoning. Need to check xAI semantics (suspected exclusive).

- timestamp: 2026-10-17T00:23:00Z
  checked: Gemini mapper crates/paladin-llm/src/gemini/adapter.rs:1331-1381 (GeminiUsageMetadata, map_gemini_usage); request struct :1247-1253 (no tools field)
  found: Reads promptTokenCount, candidatesTokenCount, cachedContentTokenCount -> cache_read, thoughtsTokenCount -> reasoning AND added into completion (candidates + thoughts). totalTokenCount dead_code. toolUsePromptTokenCount not deserialised at all despite rustdoc quoting total = prompt + candidates + toolUsePrompt + thoughts and claiming the mapping "agrees with" it. Request body has no tools/googleSearch/codeExecution, so today toolUsePromptTokenCount is expected absent/0.
  implication: Gemini mapper correctly folds thoughts (which Gemini reports EXCLUSIVE of candidates). toolUsePrompt tokens are a latent loss: unreachable on current request shape, but the rustdoc agreement claim is unverified and any built-in-tool support added later would undercount silently, with totalTokenCount (the reconciling figure) already thrown away.

- timestamp: 2026-10-17T00:25:00Z
  checked: Vision/embedding side paths - openai/vision.rs:138-141,428-431,506-509; anthropic/vision.rs:90-92,422-425,499-502; openai/embedding.rs:184,231,271-274
  found: OpenAI vision TokenUsage::new(prompt, completion) only - no cache/reasoning sub-counts; VisionTokenUsage copies vendor total_tokens verbatim. Anthropic vision ClaudeVisionUsage has only input_tokens/output_tokens - cache_read/cache_creation NOT added to prompt (the D-20 correction from the chat adapter is absent here). Embedding uses prompt_tokens as token_count.
  implication: Anthropic vision path would undercount billed input if any prompt caching applies (no cache_control is sent, so low-probability today). Inconsistent with chat adapter.

- timestamp: 2026-10-17T00:35:00Z
  checked: xAI Grok chat-completions usage semantics (web research - Google Vertex AI "Reasoning for Grok Models" partner-model doc; Glama grok-api-mcp reasoning doc; xAI docs cost-tracking page via search summary)
  found: Grok reports reasoning_tokens SEPARATELY from completion_tokens - Vertex example prompt 663, completion 50, completion_tokens_details.reasoning_tokens 124, total_tokens 837 (= 663+50+124); Glama example 50+200+150 = 400. xAI cost-tracking doc - usage.cost_in_usd_ticks (1 USD = 1e10 ticks) is the actual amount billed after discounts, inclusive of all token costs AND server-side tool invocation costs. Direct docs.x.ai fetch blocked by proxy (403), so xAI-authored confirmation of completion-exclusivity is secondary-sourced.
  implication: For Grok (all seeded 4.x ids are reasoning models; grok-4.3 is the provider default) completion_tokens EXCLUDES reasoning, so vendor total > prompt + completion on every reasoning call. map_compat_usage assumes OpenAI's inclusive semantics for every compat preset.

- timestamp: 2026-10-17T00:40:00Z
  checked: Out-of-tree scratch reproduction (scratchpad/repro, path-deps on paladin-llm[grok,gemini], paladin-ai-core, paladin-ports; std TcpListener mock; NO source modified) driving the real GrokAdapter::generate and GeminiAdapter::generate, then cost_of_call at $1/M in, $10/M out
  found: GROK vendor prompt=663 completion=50 reasoning=124 total=837 -> mapped prompt=663 completion=50 total=713 reasoning=Some(124); TokenUsage invariant reasoning<=completion VIOLATED (124 > 50); cost_of_call = 1_163_000 nanos vs 2_403_000 nanos the vendor bills at the same row (52% under). GEMINI vendor prompt=100 candidates=20 toolUsePrompt=300 thoughts=10 total=430 -> mapped prompt=100 completion=30 total=130 (300 tool-use prompt tokens lost, 70% of the call).
  implication: CONFIRMED (deterministic, Bohrbug). Grok: 124 billed reasoning tokens vanish from total_tokens, and cost_of_call's T-38-05 clamp (reasoning.min(completion), cost.rs:386) then prices only 50 of the 124 reasoning tokens and zero base completion - the clamp meant to tolerate a "malformed provider response" silently converts Grok's normal response into an undercharge. Gemini: undercount is real in the mapper but latent (GeminiRequest has no tools field, so toolUsePromptTokenCount should not appear today).

- timestamp: 2026-10-17T00:50:00Z
  checked: Spend consumers - crates/paladin-llm/src/pricing.rs:127-190 (PricingLlmAdapter), crates/paladin-core/src/platform/container/cost.rs:375-410 (cost_of_call), :479 (PriceTable::price), :541-562 (CostTally record_call/record_node); src/application/services/paladin/paladin_execution_service.rs:1851-1866 (agent loop), :414-440 (settle_agent_loop_call); crates/paladin-battalion/src/engine/settlement.rs (SuperstepSpend/SpendHook record Cost per attempt); crates/paladin-battalion/src/engine/hooks.rs:347 (record_node); src/application/services/treasurer/{mod,evaluate,derive,guard}.rs; src/application/services/paladin/middleware/limits.rs:135-200 (TokenBudget); crates/paladin-web/src/agent_controller.rs:620-640
  found: PricingLlmAdapter sets LlmResponse.cost = PriceTable::price(model, usage) = cost_of_call, which reads prompt_tokens, completion_tokens, cache_read_tokens, cache_write_tokens, reasoning_tokens (NOT total_tokens) and clamps reasoning to completion. Agent loop folds response.cost into CostTally and settles exactly that Cost to TreasuryLedgerPort; battalion engine settles each attempt's Cost at the superstep boundary. Treasurer (admission, SpendGuard, derive_budget) reads only ledger balances (Cost) and PriceRow axes - never TokenUsage. derive_budget turns remaining nanos into DerivedTokenBudget.max_tokens; TokenBudget middleware compares cumulative usage.total_tokens (paladin_execution_service.rs:1866) > limit; web streaming done-event compares usage.total_tokens > budget.max_tokens. OTel, battalion result totals, CLI formatters read total_tokens for reporting only.
  implication: Two spend channels, both fed by the mapper. Money channel (ledger/Treasurer balance) = cost_of_call over prompt/completion/sub-counts; token channel (TokenBudget, web allowance crossing) = total_tokens. A mapper undercount in completion_tokens undercounts BOTH; the clamp makes the money channel undercount even when reasoning_tokens is correct.

- timestamp: 2026-10-17T00:52:00Z
  checked: Commissary crates/paladin-llm/src/services/commissary.rs (1509 lines) and its callers (crates/paladin-memory/src/services/rag_retrieval_service.rs:233-244)
  found: Commissary reads only ProviderCapabilities/ModelProfile max_context_tokens, CommissaryPlan.reserved_completion_tokens, and TokenCounterPort::count / is_exact over the assembled prompt BEFORE the call. The only LlmResponse references are inside #[cfg(test)] (line 677+) stub ports. No TokenUsage read anywhere.
  implication: Confirmed - the Commissary does not consume vendor-reported usage; it is unaffected by the mapper defect. The owner's "Commissary depends on these counts" holds only indirectly (a future post-call calibration would); today the dependency is Treasurer + TokenBudget + web allowance.

- timestamp: 2026-10-17T00:54:00Z
  checked: Error paths - compat/engine.rs:955-965, deepseek/adapter.rs:940-953, openai/adapter.rs:1038-1047, anthropic/adapter.rs:528-545, gemini/adapter.rs:680-701; LlmError::EmptyCompletion(String) at crates/paladin-ports/src/output/llm_port.rs:517
  found: Every adapter calls classify_completion(...)? BEFORE mapping usage; on EmptyCompletion / Refusal the function returns Err and the parsed vendor usage is dropped (DeepSeek alone stringifies it into the error message). LlmError variants carry no TokenUsage. Streaming paths likewise replace the terminal chunk (which holds the usage) with the error. CostTally::record_node treats a default-usage/no-cost node as "a failed attempt that billed nothing".
  implication: Separate billable-loss path: a vendor-billed call that is classified as an error (classic case - reasoning model spends the whole max_tokens on thinking and returns empty visible text; DeepSeek test fixture shows total_tokens 63_000 on such a call) contributes ZERO to TokenUsage, cost and the ledger. Independent of the Grok mapping defect (different category: control flow / error contract).

- timestamp: 2026-10-17T00:56:00Z
  checked: Why not caught - crates/paladin-llm/src/conformance.rs:370-390 (usage_body Wire::OpenAiChat) and :1440-1470; compat/engine.rs:3346 test; docs/src/user-guides/switching-providers.md:277-279
  found: The shared conformance suite feeds ONE OpenAI-semantics usage fixture (100 + 40 = 140, reasoning 15 inside completion) to every Wire::OpenAiChat provider, Grok included, and asserts total == prompt + completion - never reasoning <= completion and never vendor total == prompt + completion. compat_usage_fields_are_copied_exactly pins the discard of a disagreeing vendor total (9999). User guide documents "the vendor's own total is not copied".
  implication: No gate could catch a vendor whose completion_tokens excludes reasoning: the only signal (vendor total) is thrown away by design and the fixtures encode OpenAI semantics for all compat presets.

## Resolution

root_cause: TokenUsage's inclusive contract (prompt_tokens = every billed input token, completion_tokens = every billed output token, total = prompt + completion) is assumed but never verified at the adapter boundary, and the one figure that could verify it - the vendor's own total - is deserialised and deliberately discarded (D-02; #[allow(dead_code)] in compat/types.rs:126, openai/adapter.rs:352, gemini/adapter.rs:1340; DeepSeek uses it only in an error string). Consequences - (1) CompatEngine::map_compat_usage (compat/engine.rs:641) applies OpenAI's inclusive semantics to every compat preset; xAI Grok reports completion_tokens EXCLUSIVE of completion_tokens_details.reasoning_tokens (vendor total = prompt + completion + reasoning), so every Grok reasoning call (all seeded grok-4.x, default grok-4.3) loses its reasoning tokens from total_tokens and breaks reasoning <= completion; cost_of_call (cost.rs:386) clamps reasoning to completion, so the Treasurer is charged ~half (repro 1.163M vs 2.403M nanos). Grok's vendor-computed cost_in_usd_ticks and num_sources_used (live-search units) are not parsed. (2) GeminiUsageMetadata (gemini/adapter.rs:1331) never deserialises toolUsePromptTokenCount although its rustdoc quotes total = prompt + candidates + toolUsePrompt + thoughts - latent (GeminiRequest sends no tools today), repro 430 -> 130. (3) Independently, every adapter runs classify_completion(...)? before mapping usage, so a vendor-billed call classified EmptyCompletion/Refusal yields no TokenUsage, no cost and no ledger settle (LlmError carries no usage). OpenAI, Anthropic, DeepSeek and Gemini (thoughts) mappers fold all billed TOKEN categories correctly on success; their remaining losses are pricing-granularity (OpenAI/Grok audio split, Anthropic 5m/1h cache-write split and service_tier/speed). The Commissary reads no response usage.
fix: (diagnose-only mode - not applied)
verification: Root cause reproduced out-of-tree (scratchpad/repro) through the real GrokAdapter and GeminiAdapter plus cost_of_call; no source modified.
files_changed: []
