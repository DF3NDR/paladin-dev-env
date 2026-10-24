---
status: diagnosed
trigger: "G-50-5c (kimi-fallback-profile-stale): The Kimi adapter-wide fallback profile (applies to unseeded Kimi model ids) declares temperature Fixed(1.0), top_p Fixed(0.95) and penalties -2.0..2.0 from a 2026-08-22 measurement that the 2026-10-15 probe contradicts (WINDOWS.md row 76). The owner chose to align the fallback with the seeded Kimi profiles: temperature, top_p, frequency_penalty and presence_penalty not sent."
created: 2026-10-17T16:15:24Z
updated: 2026-10-17T16:55:00Z
goal: find_root_cause_only
---

## Current Focus

bug_class: Bohrbug (deterministic: a stale compiled declaration; every unseeded Kimi id resolves to it)
hypothesis: CONFIRMED -- the fallback is stale because two independent sites in KimiAdapter::new encode the 2026-08-22 measurement (Fixed overrides for temperature/top_p; request_parameters penalty flags -> fallback_from_compat Range(-2,2)), and nothing compares the fallback to the seeded Kimi rows.
test: code reading of the resolution chain + `cargo test -p paladin-llm --features kimi --lib -- kimi::` (75 pass, incl. the four tests that pin the stale values) + switching_page_tests (13 pass, fallback values unchecked)
expecting: n/a (diagnosed)
next_action: return ROOT CAUSE FOUND to caller (goal: find_root_cause_only); fix belongs to a gap-closure plan for G-50-5c

reasoning_checkpoint:
  hypothesis: "Unseeded Kimi ids admit temperature 1.0, top_p 0.95 and penalties -2..2 because KimiAdapter::new overrides the fallback with Fixed(1.0)/Fixed(0.95) and declares request_parameters penalties carried, which fallback_from_compat maps to Range(-2,2)"
  confirming_evidence:
    - "adapter.rs:337-339 chains .with_temperature(Fixed(1.0)).with_top_p(Fixed(0.95)) onto the fallback"
    - "adapter.rs:295-301 request_parameters all true; registry/mod.rs:679-686 maps carried penalties to Range(-2.0, 2.0)"
    - "kimi_model_profile_declares_the_two_measured_single_values and generate_omits_top_p_from_the_moonshot_request_body pass today on UNSEEDED_MODEL, observing Fixed(1.0)/Fixed(0.95) and penalties on the wire"
    - "seeded.rs:221-252 kimi-k2.6/kimi-k3 declare NotSent for all four"
  falsification_test: "If an unseeded Kimi id resolved to NotSent for any of the four, kimi_model_profile_declares_the_two_measured_single_values would fail / penalties would be absent from the wire in generate_omits_top_p_from_the_moonshot_request_body -- both pass, so the hypothesis stands"
  fix_rationale: "Declaring the four keys NotSent on the fallback (preferably by setting the four request_parameters flags false and deleting the Fixed overrides, the Grok idiom) removes the stale values at their source; the wire filter and build check already treat NotSent exactly as for the seeded entries"
  blind_spots: "No 2026-10-15 temperature probe was sent with thinking enabled, so the vendor's thinking-on legal value is unmeasured (the 2026-08-22 'only 1' may still hold in that mode); NotSent sidesteps this. Fallback reasoning=false and json_schema=false also differ from the seeded rows but are outside the owner's decision."
  candidate_causes:
    - "code: explicit Fixed(1.0)/Fixed(0.95) overrides in KimiAdapter::new (confirmed)"
    - "config: preset declaration request_parameters penalty flags = true feeding fallback_from_compat (confirmed)"
    - "data/environment: vendor's single legal value is thinking-mode dependent and changed between measurements (2026-08-22 'only 1' vs 2026-10-15 thinking-disabled 'only 0.6') (confirmed as the reason a Fixed value cannot be right)"
    - "process/gate: drift test, conformance capability case and lib.rs invariant all skip the Kimi fallback (confirmed contributing; why it was not caught)"
  and_gate: "yes -- the stale profile is the conjunction of two independent code sites (Fixed overrides for temperature/top_p AND carried-penalty flags for the penalties); fixing either alone leaves the fallback half-stale, so root_cause is a set"

## Symptoms

expected: An unseeded Kimi model id resolves to a fallback profile matching the seeded `kimi-k2.6` / `kimi-k3` profiles -- temperature, top_p, frequency_penalty and presence_penalty are not sent (an explicit operator value is handled the same way the seeded profiles handle it under ADR-0061).
actual: crates/paladin-llm/src/kimi/adapter.rs ~line 320-343 builds `fallback_from_compat(...)` then `.with_temperature(ParamSupport::Fixed(1.0)).with_top_p(ParamSupport::Fixed(0.95))`; capabilities declare `temperature_range: Some((1.0, 1.0))` (~line 274); docs/src/user-guides/switching-providers.md line ~216 and note [f] (~251) describe it as unverified. The 2026-10-15 probe (50-SEED-PROBE.md) shows temperature only 0.6 allowed, top_p only 0.95, frequency/presence penalty only 0.
errors: Vendor 400s such as "invalid temperature: only 0.6 is allowed for this model" when the stale fallback admits 1.0.
reproduction: Test 5 in .planning/phases/50-provider-parity-config-driven-model-swap/50-UAT.md; WINDOWS.md row 76.
started: Carried unreconciled from plans 50-06 and 50-15; discovered during Phase 50 UAT (2026-10-17).

## Eliminated

## Evidence

- timestamp: 2026-10-17T16:20:00Z
  checked: Phase 0 knowledge base (.planning/debug/knowledge-base.md) and MemPalace
  found: .planning/debug/ did not exist before this session; no knowledge-base.md; no prior resolved sessions. MemPalace not queried (no tool in this agent).
  implication: no known-pattern candidate; proceed normally.

- timestamp: 2026-10-17T16:21:00Z
  checked: 50-UAT.md Test 5 + gap G-50-5c; .planning/WINDOWS.md row 76
  found: Owner chose option (a): align the adapter-wide Kimi fallback with the seeded Kimi profiles (temperature, top_p, frequency_penalty, presence_penalty not sent). Row 76 is open, file crates/paladin-llm/src/kimi/adapter.rs, cites temperature Fixed(1.0), top_p Fixed(0.95), declared range (1.0, 1.0) from 2026-08-22.
  implication: target state is defined by the owner; the investigation is to locate every place the stale fallback is encoded.

- timestamp: 2026-10-17T16:24:00Z
  checked: crates/paladin-llm/src/registry/seeded.rs kimi_table (lines 192-255)
  found: kimi-k2.6 and kimi-k3 (verified_on 2026-10-15) both declare .with_temperature(NotSent).with_top_p(NotSent).with_frequency_penalty(NotSent).with_presence_penalty(NotSent), stop Supported{max:None}, json_object true, json_schema true, reasoning true. Doc comment: every probed temperature (1.00, 0.50, 1.01) rejected "only 0.6 is allowed"; top_p 0.5 rejected "only 0.95"; penalties rejected "only 0 is allowed".
  implication: the exact target ParamSupport for all four keys is ParamSupport::NotSent.

- timestamp: 2026-10-17T16:26:00Z
  checked: crates/paladin-llm/src/kimi/adapter.rs lines 244-348 (KimiAdapter::new)
  found: capabilities.temperature_range = Some((1.0, 1.0)) (line 274); request_parameters = {temperature: true, top_p: true, max_tokens: true, frequency_penalty: true, presence_penalty: true} (lines 295-301); fallback = fallback_from_compat(caps, request_parameters, Stop Supported{max:None}) then .with_temperature(Fixed(1.0)).with_top_p(Fixed(0.95)) (lines 326-339). Comment at 262-273 still quotes "only 1 is allowed" (2026-08-22 text).
  implication: the fallback's four sampling fields come from two sources: temperature/top_p from the explicit Fixed overrides; frequency/presence penalty from fallback_from_compat because request_parameters.*_penalty = true -> Range(-2.0, 2.0).

- timestamp: 2026-10-17T16:28:00Z
  checked: crates/paladin-llm/src/registry/mod.rs fallback_from_compat (lines 642-694)
  found: temperature = Range(caps.temperature_range or (0,1)) if params.temperature else NotSent; top_p = Range(0,1) if params.top_p else NotSent; penalties = Range(-2,2) if carried else NotSent; stop = caller's; json_object true, json_schema false; context_window = caps.max_context_tokens.
  implication: setting request_parameters.{temperature,top_p,frequency_penalty,presence_penalty}=false makes fallback_from_compat emit NotSent for all four with no override, and decouples temperature_range from the fallback profile.

- timestamp: 2026-10-17T16:30:00Z
  checked: crates/paladin-llm/src/compat/engine.rs CompatRequestParameters docs (109-180) and build_request (548-628)
  found: "Since Phase 50 the per-model profile decides what is sent; this flag now feeds the adapter-wide fallback profile ... not a separate gate." Only max_tokens is still gated directly by request_parameters (line 562). temperature/top_p/penalties go through sampling::apply(&profile, params) only.
  implication: request_parameters.{temperature,top_p,penalties}=false does NOT block a seeded entry or a ModelOverride from admitting a value (registry resolves override over seed over fallback; the wire filter reads only the profile). Either fix shape (flags false, or explicit NotSent overrides) yields identical wire behaviour; max_tokens must stay true.

- timestamp: 2026-10-17T16:32:00Z
  checked: paladin-ports model_profile.rs ParamSupport::admits; crates/paladin-llm/src/sampling.rs apply/admit_numeric; src/application/services/paladin/paladin_builder.rs validate()
  found: NotSent admits nothing. Wire: a set-but-inadmissible value is omitted and recorded (warn! once + paladin.params_omitted). Build: an EXPLICIT .temperature(x) is refused by name (ConfigurationError) when profile.temperature does not admit it; the fabricated 0.7 and auto-selected values are never refused at build. Builder uses temperature_range ONLY when model_profile() returns None; KimiAdapter::model_profile always returns Some.
  implication: with NotSent, an unseeded Kimi id behaves exactly like kimi-k2.6/kimi-k3: explicit operator temperature refused at build, defaults omitted+recorded at the wire. Kimi's capabilities.temperature_range does not drive admission for Kimi at all.

- timestamp: 2026-10-17T16:34:00Z
  checked: crates/paladin-llm/src/lib.rs test_every_adapter_declares_a_temperature_range (lines 342-375) and capability_invariants_new_providers (378-544)
  found: the temperature-range invariant constructs ONLY openai, anthropic, deepseek. The new-providers module (which constructs Kimi) checks only tool/function-calling flags. Kimi's adapter.rs comment (272-273) and test comment (1103-1105) claim the lib.rs invariant covers Kimi -- it does not.
  implication: no compiled test forces Kimi's temperature_range to be Some(_); the constraint is the invariant's stated intent ("every shipped adapter must declare Some((min, max)), never None") plus ProviderCapabilities::temperature_range being Option<(f32,f32)>, which has no NotSent representation.

- timestamp: 2026-10-17T16:36:00Z
  checked: crates/paladin-llm/src/anthropic/adapter.rs fallback_profile (303-306) and capabilities (285)
  found: Anthropic's fallback declares temperature NotSent while get_capabilities() still reports temperature_range Some((0.0, 1.0)).
  implication: precedent exists for a NotSent fallback temperature beside a Some(_) temperature_range; the two are already decoupled in-tree.

- timestamp: 2026-10-17T16:40:00Z
  checked: crates/paladin-llm/src/registry/mod.rs switching_page_tests::check_page_text (1773-1784)
  found: for a row whose model cell is "(adapter-wide fallback)" the check only requires the verified-on cell to read "unverified", then `continue`s -- no fallback cell value is compared against code. Ran `cargo test -p paladin-llm --features <all nine> --lib -- switching_page_tests`: 13 passed.
  implication: the drift test neither caught the stale Kimi fallback row nor will break when it changes; docs line 216 and note [f] are hand-maintained and must be edited by hand.

- timestamp: 2026-10-17T16:42:00Z
  checked: crates/paladin-llm/src/conformance.rs capability_declarations_match_the_wire (1093-1200)
  found: measures F::default_model() only; Kimi fixture default is KIMI_DEFAULT_MODEL (kimi-k2.6, seeded). It also checks max_context_tokens vs profile window but never temperature_range vs profile.temperature.
  implication: the conformance gate never exercises the Kimi fallback (50-15-SUMMARY line 46/197 records the same).

- timestamp: 2026-10-17T16:45:00Z
  checked: ran `cargo test -p paladin-llm --features kimi --lib -- kimi::` (75 passed)
  found: these PASS today and pin the stale fallback: kimi_model_profile_declares_the_two_measured_single_values (adapter.rs:1304-1322, asserts Fixed(1.0)/Fixed(0.95) on UNSEEDED_MODEL), kimi_fixed_values_follow_the_profile (1324-1344, asserts explicit 1.0/0.95 reach the wire), generate_omits_top_p_from_the_moonshot_request_body (1193-1263, asserts body contains frequency_penalty and presence_penalty for UNSEEDED_MODEL -- PromptParameters::default() sets both to 0.0), get_capabilities_reports_the_measured_degenerate_temperature_range (1101-1116, asserts Some((1.0,1.0))).
  implication: the tests encode the stale behaviour as the specification; any fix must rewrite/replace them.

- timestamp: 2026-10-17T16:48:00Z
  checked: other consumers -- tests/provider_swap_matrix.rs (kimi cases use KIMI_DEFAULT_MODEL), crates/paladin-llm/src/test_support.rs send_with_max_tokens (all sampling None), fallback.rs / agent_runtime.rs (MockLlmAdapter or config-only), provider_factory.rs:1661 (kimi-k3, seeded), paladin_builder.rs tests (own mock profiles; comments at 1169, 1199, 1574 call Fixed(1.0) "Kimi-shaped")
  found: none constructs a real KimiAdapter on an unseeded id except the four adapter tests above plus kimi_wire/kimi_parameters helpers, kimi_max_tokens_default_reaches_the_wire and kimi_context_window_override_is_declared (both unaffected: they check max_tokens / window only).
  implication: blast radius of the fix is contained to kimi/adapter.rs tests; matrix, conformance and registry tests are unaffected.

- timestamp: 2026-10-17T16:50:00Z
  checked: 50-SEED-PROBE.md "Probes: kimi-k3"/"Probes: kimi-k2.6" rows 3-8 and "Other observations"; probe header "declared temperature range: 1..1"; examples/live_vendor_smoke.rs:311
  found: all 2026-10-15 probes sent {"thinking":{"type":"disabled"}}; temperature 1.00/0.50/1.01 -> 400 "only 0.6 is allowed"; top_p 0.5 -> 400 "only 0.95 is allowed"; penalties 0.5 -> 400 "only 0 is allowed". No temperature probe was sent with thinking enabled. The smoke harness's ProviderSpec mirrors the adapter's (1.0, 1.0) by hand and derived the probe points from it.
  implication: the vendor's single legal value is mode-dependent (2026-08-22 "only 1" vs 2026-10-15 thinking-disabled "only 0.6"); NotSent is the only mode-independent declaration, which is why seeded.rs (202-207) refuses to pin a value -- and why the owner's NotSent choice is correct.

- timestamp: 2026-10-17T16:52:00Z
  checked: CHANGELOG.md, crates/paladin-llm/CHANGELOG.md, MIGRATION.md, ADR-0061, docs/src/getting-started/configuration.md, .planning/WINDOWS.md
  found: CHANGELOG.md:793-795 ("Kimi now sends an explicit `1.0` temperature and `0.95` `top_p`") and Known limitations (4) at 1257-1259; crates/paladin-llm/CHANGELOG.md:81-82 ("The Kimi preset sends an explicit `1.0` and `0.95`"); MIGRATION.md row M-B-13 (line 38: "Kimi now sends an explicit `1.0` temperature (and `0.95` `top_p`)") and M-B-14 (line 39: "unseeded Kimi models keep the previous adapter-wide values"); ADR-0061 line 155 stated limit (5) naming WINDOWS row 76; ADR-0061 lines 17 and 45 use Kimi's (1.0,1.0)/Fixed 1.0 as historical/illustrative context; configuration.md:138 historical "measured fixed-temperature constraint"; WINDOWS.md row 76 open.
  implication: release notes/migration text describe the stale fallback as shipped behaviour and must be amended (Unreleased section, so an edit not a new entry); ADR-0061 limit (5) and WINDOWS row 76 need closing.

## Eliminated

- hypothesis: lib.rs `test_every_adapter_declares_a_temperature_range` forces Kimi's temperature_range to stay Some(_) and would fail if it changed
  evidence: lib.rs:342-375 constructs only OpenAI, Anthropic and DeepSeek; the Kimi-constructing module (capability_invariants_new_providers) checks tool/function-calling only.
  timestamp: 2026-10-17T16:34:00Z

- hypothesis: CompatRequestParameters.{temperature,top_p,*_penalty}=false would act as a hard wire gate and also block seeded entries / ModelOverrides from sending a value
  evidence: engine.rs:150-151 and 538-543 -- since Phase 50 only max_tokens is gated by the flag; the four sampling keys go through sampling::apply(&profile) and the flags only seed fallback_from_compat.
  timestamp: 2026-10-17T16:30:00Z

- hypothesis: the Switching providers drift test pins the fallback row values, so it would have caught the stale row and will break on the fix
  evidence: registry/mod.rs:1775-1784 skips value comparison for fallback rows; 13/13 switching_page_tests pass.
  timestamp: 2026-10-17T16:40:00Z

## Resolution

root_cause: "The Kimi adapter-wide fallback profile (KimiAdapter::new, crates/paladin-llm/src/kimi/adapter.rs) is built from two stale 2026-08-22 sources that plan 50-06 left in place when it seeded kimi-k2.6/kimi-k3 as NotSent: (1) the explicit `.with_temperature(ParamSupport::Fixed(1.0)).with_top_p(ParamSupport::Fixed(0.95))` overrides at lines 337-339; (2) `request_parameters` declaring frequency_penalty/presence_penalty (and temperature/top_p) carried = true at lines 295-301, which registry::fallback_from_compat (registry/mod.rs:662-694) turns into Range(-2.0, 2.0) penalties. No gate compares the fallback against the seeded profiles or the probe (docs drift test skips fallback-row values; conformance measures the seeded default model; lib.rs temperature-range invariant does not construct Kimi), and four Kimi unit tests pin the stale values as the specification."
fix: (diagnose-only; not applied)
verification: (diagnose-only)
files_changed: []
