---
phase: 51-adjustable-reasoning-effort
plan: 11
subsystem: llm-adapters
tags: [reasoning-effort, kimi, qwen, grok, seeded-registry, effort-probe, silent-no-op-regression]
status: complete

requires:
  - phase: 51-01
    provides: "EffortSupport, EffortLevels, BudgetBounds, ModelProfile::with_effort / with_effort_verified_on"
  - phase: 51-03
    provides: "paladin_llm::reasoning (ReasoningWire, admit, mask_effort)"
  - phase: 51-04
    provides: "compat engine per-wire mapping; the Kimi / Qwen / Grok starting positions"
  - phase: 51-06
    provides: "provider-level reasoning default (Kimi provider-default test restated here)"
  - phase: 51-10
    provides: "51-EFFORT-PROBE.md, the only source effort data may be seeded from"
provides:
  - "Kimi, Qwen and Grok declared wires restated from the verdict table (Kimi ThinkingObject, Qwen EnableThinking, Grok TopLevelEffort)"
  - "Probe-evidenced effort data on every probed Kimi, Qwen and Grok row, each with a `// Source for the effort data` comment and effort_verified_on 2026-10-22"
  - "Kimi silent-no-op regression `kimi_preset_never_emits_*`, pinned in the probe's direction (thinking.effort, never a top-level reasoning_effort)"
  - "kimi_k3/kimi_k2_6/qwen/grok `*_effort_seed_matches_the_transcript` tests"
affects: [51-12, 51-13, 51-14]

tech-stack:
  added: []
  patterns:
    - "effort rows carry a `// Source for the effort data: 51-EFFORT-PROBE.md, <section>` comment directly above `.with_effort(..)`"
    - "per-vendor seed tests type the expected table from the transcript and read it through the adapter's model_profile"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/registry/seeded.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/qwen/adapter.rs
    - crates/paladin-llm/src/grok/adapter.rs
    - crates/paladin-llm/src/provider_factory.rs
    - crates/paladin-llm/CHANGELOG.md
    - CHANGELOG.md

key-decisions:
  - "Kimi ships thinking.effort and never a top-level reasoning_effort (owner, 2026-10-22, apply-verdicts)."
  - "Grok ships the top-level reasoning_effort (owner, 2026-10-22); grok-4.5 xhigh is a coerced level."
  - "Qwen keeps EnableThinking by an explicit owner exception to the mechanical D-04 rule; no named level is ever converted to a budget."
  - "Qwen budget max is 32768, the vendor-documented figure, the lowest of the three figures in the file."

patterns-established:
  - "Plans 51-12 and later reuse the effort source-comment style and the module-docs paragraph in seeded.rs."

requirements-completed: []
requirements-progress: [REASON-02, REASON-03, REASON-04, REASON-06]

duration: about 2h (cold worktree build included)
completed: 2026-10-22
---

# Phase 51 Plan 11: Kimi, Qwen and Grok probe-confirmed wires and measured effort Summary

**Kimi, Qwen and Grok now ship only what `51-EFFORT-PROBE.md` evidences: Kimi's `thinking.effort` (pinned against the dead top-level field), Qwen's `enable_thinking` / `thinking_budget` with per-model off, auto and budget, and Grok's newly confirmed top-level `reasoning_effort` with per-model levels and one coerced level.**

REASON-02, -03, -04 and -06 advance for these three vendors but are not marked complete: the four native adapters (plan 51-12), the docs page and ADR (51-13) and the surface register (51-14) remain. `requirements-completed` is empty and REQUIREMENTS.md is untouched.

## Checkpoint decision

Task 1 (`checkpoint:decision`, gate blocking) was resolved by the owner before this run began. The orchestrator prepared the facts from `51-EFFORT-PROBE.md`'s `## Verdict table` and per-vendor sections, presented them, answered the owner's follow-up questions, and the owner decided on **2026-10-22**. Task 1 built nothing and has no commit of its own. Recorded verbatim as handed to me.

Owner's words (2026-10-22): "apply-verdicts; Qwen: ship EnableThinking with off/auto/budget seeded from the probe"

Decisions, per vendor, as derived from the verdict table and decided by the owner:

- Kimi: `ThinkingObject` (`thinking.effort`) -- confirmed (invalid value drew 400 on both models; kimi-k3 low=40 / high=48 / max=116; `minimal`, `medium`, `xhigh` rejected 400; off and auto accepted). The top-level `reasoning_effort` form is NOT shipped (invalid value accepted with 200 on both models: the silent no-op). This matches roadmap success criterion 3 as written, so the regression test is the handoff's "never emits top-level `reasoning_effort`" direction; restate the pre-probe `kimi_pre_probe_*` tests from the verdict table per the plan.
- Grok: `TopLevelEffort` -- confirmed (invalid value 400 on 5 of 5; levels `minimal`..`xhigh` accepted on grok-4.3/4.5/4.6/4.7, `max` rejected everywhere; `grok-4.20-reasoning` rejected every level and seeds no levels).
- Qwen: keep the `EnableThinking` wire and seed per-model `off`, `auto` and `budget` from the probe (owner's explicit decision). The mechanical D-04 rule cannot mark a budget-only wire `confirmed` because it has no level rows; the owner judged that `enable_thinking` off (accepted, no reasoning tokens) and auto (accepted, reasoning tokens) on all 13 models, plus `thinking_budget` 1 accepted and 0 / -1 rejected on 12 of 13, constitute measured confirmation of the wire's three behaviours. The named levels refuse on Qwen because the vendor has no level field (never convert a level to a budget: D-03/D-06). Budget min comes from the evidence (1 accepted, 0 rejected); a max comes only from the vendor-documented section with its URL and date, else no `budget` bound and `Budget` refuses, per the plan's seed rules. The `qwen3.7-max` off row comes from the disclosed 8-call retry (run 7). Record the owner's rationale beside the verdict rows; if the plan's own acceptance criterion ("an override naming a form the verdict table does not mark confirmed is refused") makes you unable to implement this in good conscience, stop and return a checkpoint report stating exactly which rule conflicts -- do not silently downgrade Qwen to `Unsupported`.
- For plan 51-12 (recorded here so it runs unattended): OpenAI top-level `reasoning_effort` confirmed (17 of 17 parsed, 14 of 17 effect); DeepSeek top-level `reasoning_effort` confirmed (2 of 2), with its `thinking` off/auto rows as measured; Anthropic `output_config.effort` confirmed (11 of 11 parsed, 7 of 9 effect), pairing set (models whose thinking tokens appear only when paired with `thinking: {type: adaptive}`, i.e. `enable_thinking_with_level`): `claude-opus-4-8`, `claude-opus-4-7`, `claude-opus-4-6`, `claude-sonnet-4-6`; the 4.6-generation `thinking.budget_tokens` rows drew 400 on every model probed; Gemini lowercase `thinkingLevel` confirmed (12 of 12 parsed, 5 of 5 effect), uppercase `HIGH` also accepted everywhere -- keep the lowercase spelling; `minimal` rejected on 11 of 12.
- D-02 (from plan 51-10): `unmeasured` -- an unprobed seeded row stays `EffortSupport::Unmeasured`.

### Verdict rows the decisions rest on (`51-EFFORT-PROBE.md`, `## Verdict table`, "Per vendor and form")

| vendor | form | invalid value drew 400/422 | field parsed | models where highest accepted level reports more tokens than lowest | effect | confirmed |
|---|---|---|---|---|---|---|
| Kimi | `thinking_object` | 2 of 2 | yes | 1 of 2 | yes | yes |
| Kimi | `top_level_effort` | 0 of 2 (accepted 2xx on 2 of 2) | no | 2 of 2 | yes | no |
| Grok | `top_level_effort` | 5 of 5 | yes | 2 of 4 | yes | yes |
| Qwen | `enable_thinking` | no invalid row of its own | n/a | no level rows | n/a | n/a (owner exception below) |
| Qwen | `thinking_budget` | 12 of 13 | partial | no level rows | n/a | n/a (owner exception below) |

### Owner's Qwen rationale and how it was implemented

The owner's reading is recorded beside the rows above and in the `request_parameters` comment of `crates/paladin-llm/src/qwen/adapter.rs` (the comment says the owner granted the wire although the mechanical D-04 rule cannot mark a budget-only wire `confirmed`). I did not hit a rule conflict that forced a stop: the plan's acceptance criterion about an `override` naming an unconfirmed form concerns the `override` option, and the owner chose `apply-verdicts` with an explicit Qwen carve-out, so Qwen was neither downgraded nor silently confirmed. Two facts in the owner's wording differ slightly from the file and are reported under Deviations (auto on 11, not 13, models; budget on 11, not 12).

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Owner confirms the wire decisions (checkpoint:decision) | none (resolved by the owner before this run; recorded above) |
| 2 | Tracer: `kimi-k3` ships its probe-confirmed wire and measured levels, pinned by the silent-no-op regression | `5441363f` |
| 3 | Qwen and Grok probe-confirmed wires, measured levels and budget bounds, CHANGELOG record | `859422af` |

The Task 2 tracer gate was consumed by the owner's checkpoint, as the orchestrator directed: after its own verify steps passed (Kimi tests, `registry::`, the grep guards, clippy, fmt, the unset-effort goldens) I proceeded to Task 3 without pausing.

## What was built

- **Kimi.** Wire stays `ThinkingObject`; its comment is re-cited to the verdict rows. `kimi-k3` and `kimi-k2.6` carry `Supported(levels [low, high, max], off, auto)` with `effort_verified_on` `2026-10-22`. `kimi-k2.7-code` is not seeded and stays `Unmeasured` (D-02).
- **Qwen.** Wire stays `EnableThinking`. All 13 probed models carry `Supported` with no levels; per model:

  | model | off | auto | budget |
  |---|---|---|---|
  | qwen3.8-flash, qwen3.8-max, qwen3.7-plus, qwen3.7-max, qwen3.7-flash, qwen3.6-plus, qwen3.6-flash, qwen3.5-plus, qwen3.5-flash, qwen-plus, qwen-flash | yes | yes | 1..=32768 |
  | qwen-max | yes | no (auto 200 but no reasoning-token count) | none (budgets 1, 0 and -1 all accepted: field ignored) |
  | qwen-plus-latest | yes | no (400, "enable_thinking only support stream call") | none (budget 1 also 400) |

- **Grok.** Wire changes from `Unsupported` to `TopLevelEffort`.

  | model | levels | coerced | auto | off |
  |---|---|---|---|---|
  | grok-4.3, grok-4.6, grok-4.7 | minimal, low, medium, high, xhigh | none | yes (default call reported reasoning tokens) | no (no row; docs: "cannot be disabled") |
  | grok-4.5 | minimal, low, medium, high | xhigh (docs: treated as high) | yes | no |
  | grok-4.20-reasoning | none (every level drew 400) | none | yes (351 reasoning tokens by default) | no |
  | grok-4.20-non-reasoning | not probed: `Unmeasured`, no date (D-02) | | | |

- **`seeded.rs` module docs** gain an "Effort data (Phase 51)" section naming `51-EFFORT-PROBE.md` (run date, commit) as the single source of every effort field, restating the seed rules and that an unmeasured model refuses an explicit effort (D-03). The opening paragraph now says sampling, limit and capability values come from the two seed probes and effort fields from the third.
- **CHANGELOG** `### Changed` entries in the root and `crates/paladin-llm/CHANGELOG.md`: which models now accept which efforts, with the probe date.

## Restated pre-probe tests (checker revision 1, A1-51-04)

| old name | new name | verdict row it now cites |
|---|---|---|
| `kimi_pre_probe_declares_the_thinking_object_wire` | `kimi_declares_the_thinking_object_wire` | Kimi `thinking_object`: parsed 2 of 2, confirmed |
| `kimi_pre_probe_high_sends_thinking_effort_and_no_top_level_effort` | `kimi_accepted_levels_reach_the_wire_exactly` (low, high, max on both models) plus `kimi_preset_never_emits_top_level_reasoning_effort` | transcript rows 5, 9, 13 (levels accepted), row 16 (top-level field accepted 200 for the invalid value: the silent no-op) |
| `kimi_pre_probe_off_and_auto_ride_the_thinking_type` | `kimi_off_and_auto_ride_the_thinking_type` | rows 14 (off) and 15 (auto) |
| `kimi_pre_probe_unmeasured_model_refuses_naming_its_override_before_http` | `kimi_unseeded_model_refuses_naming_its_override_before_http` (model changed to `kimi-k2.7-code`, because `kimi-k2.6` is seeded now) | D-02/D-03: no probe row, so `Unmeasured` |
| `kimi_pre_probe_model_profile_reports_only_what_the_wire_carries` | `kimi_model_profile_reports_only_what_the_wire_carries` | the `thinking` object has no budget field |
| `provider_factory ... kimi_pre_probe_applies_the_provider_default_as_thinking_effort` | `kimi_applies_the_provider_default_as_thinking_effort` | Kimi `thinking_object`, confirmed |
| `grok_declares_no_reasoning_wire_until_the_probe_confirms_one` | `grok_declares_the_top_level_effort_wire` | Grok `top_level_effort`: parsed 5 of 5, confirmed |
| `grok_refuses_every_explicit_effort_even_with_an_override_and_never_hits_the_wire` (contradicted the decision; rewritten, not deleted) | `grok_evidenced_levels_reach_the_wire_exactly`, `grok_refuses_what_the_probe_did_not_evidence_before_http`, `grok_4_5_xhigh_is_refused_as_coerced_whatever_the_fallback`, `a_grok_body_never_carries_a_second_effort_field` | rows 2-7 of each Grok section; docs coercion for grok-4.5 |
| `provider_factory ... grok_refuses_its_provider_default_before_http` (contradicted the decision) | `grok_applies_the_provider_default_as_reasoning_effort` | Grok `top_level_effort`, confirmed |
| `qwen::...::a_budget_sends_enable_thinking_and_thinking_budget` (override-driven) | same name, now driven by the seeded row and a raised output cap | `### qwen3.8-max` rows 2-5 |
| `qwen::...::an_out_of_bounds_budget_and_a_level_are_refused_before_http` | `an_out_of_bounds_budget_is_refused_before_http` and `a_named_level_refuses_on_every_model_whatever_the_fallback` | documented budget max; no level field |
| `qwen_declares_the_enable_thinking_wire` | unchanged name, re-cited | owner decision |

`grep -nE 'kimi_pre_probe_|superseded by plan 51-11' crates/paladin-llm/src/kimi/adapter.rs` prints nothing, and no test in the crate pins the other Kimi direction. The new regression asserts, for all nine `ReasoningEffort` values under all three fallbacks on `kimi-k3` and `kimi-k2.6`, that a body Kimi sends carries `thinking` and none of `reasoning_effort`, `enable_thinking`, `thinking_budget`, and that every refused value sends nothing.

## Seed trace (manual review, recorded for the verification step)

Every effort value in `seeded.rs` traces to a quoted `51-EFFORT-PROBE.md` row or to its `## Vendor-documented bounds and coercions` section:

- Kimi levels / off / auto: `### kimi-k3` (run 4) and `### kimi-k2.6` (run 5) rows 3-5, 7, 9, 11, 13-15, 17.
- Qwen off / auto / budget min: each model's `### qwen..` section, rows 2-5 (run 6; `qwen3.7-max` off from run 7 row 2). Budget max 32768: the Alibaba Cloud Model Studio "deep thinking" entry (URL and 2026-10-22 access date are in the row comments).
- Grok levels / auto: each `### grok-..` section, rows 1-7. Coerced `xhigh` on grok-4.5: the docs.x.ai reasoning entry (2026-10-22).
- Nothing is seeded from memory or from the vendor docs outside that section (the xAI docs' level lists were not used to add or remove a level; the probe's accepted levels decide).

## Verification

- `cargo test --offline -p paladin-llm --all-features`: lib 1432 passed, 0 failed; `--doc`: 35 passed. Task verify commands: `--lib kimi` (100 passed), `--lib registry::` (73 passed), `--lib qwen`, `--lib grok`, and `--lib unset_reasoning_leaves_the_wire_body_unchanged` (9 passed: all nine goldens) all green.
- `cargo clippy --offline -p paladin-llm --all-targets --all-features -- -D warnings` clean; `cargo fmt --check -p paladin-llm` clean; `make api-surface` exits 0 ("API surface unchanged", 4323 items): no public item was added or changed.
- Not run: the full `make clean-code` chain and `make security` (no dependency was added, no dependency file touched); the orchestrator runs the hook chain over the merged wave.
- Manual credential-handling review (security.instructions.md): no client, header, redirect policy or error-body path was touched. Refusal text is built from provider, model, effort and override-key names only. No vendor call was made; no key value was read, printed or committed.

## TDD record

- **Task 2:** RED run first (6 of 11 new Kimi tests failed: `Unmeasured` seeds, no effort reaching the wire), then GREEN (seeded rows, comments), then the 11 Kimi tests plus `registry::` passed.
- **Task 3:** RED run first (10 of 14 Qwen / Grok tests failed; the four that passed were refusal cases that already held), then GREEN. One test failed once on its own setup (a `Budget(32768)` request against the synthetic request's 1000-token output cap, the engine's budget-below-`max_tokens` rule); the test now raises the cap, the refusal case still reaches the profile's bounds first.

## Deviations from Plan

### Auto-fixed Issues

None needing a rule.

### Notes against the plan's and the checkpoint's wording

1. **Qwen auto and budget counts differ from the checkpoint wording.** The owner's decision text says off and auto were accepted with reasoning tokens "on all 13 models" and the budget form on "12 of 13". The file's rows say: `auto` carried reasoning tokens on 11 of 13 (`qwen-max` returned 200 with no reasoning-token count; `qwen-plus-latest` drew 400 "enable_thinking only support stream call"), and budget 1 accepted / 0 rejected on 11 of 13 (`qwen-max` accepted 1, 0 and -1 alike; `qwen-plus-latest` drew the same stream-only 400; the verdict table's "12 of 13" is the count of models whose `-1` invalid row drew 400). The plan's seed rules decide ("`auto` = the auto form accepted with reasoning tokens"; "`budget` only when the min was accepted and min - 1 rejected"), so `qwen-max` and `qwen-plus-latest` carry `off` only. This is the more conservative reading and refuses where the evidence stops; the owner may widen those two rows.
2. **Qwen budget max is the console figure, 32768.** The probe file's vendor-documented section gives "Value range 1~32768" for the Alibaba console panel and names no per-model API bound; the transcript's own rejection bodies name larger figures (262144, 81920 for `qwen-flash`), which the file expressly does not reconcile. The plan allows a max only from the documented section, so 32768 is seeded: the lowest of the figures, so a budget the seed admits sits inside all of them, at the cost of refusing a budget between 32768 and the larger observed bounds. If the owner prefers no budget bound (every `Budget` refused) or the observed per-model bounds, it is a one-constant change (`THINKING_BUDGET_DOCUMENTED_MAX` in `seeded.rs`).
3. **Kimi `off` on `kimi-k2.6` read as "no reasoning tokens".** Row 14 reports 1 reasoning token (against 164 by default and 140 on `auto`; 4 completion tokens). The owner's text says "off ... accepted"; I read 1 token as the off form working and seeded `off: true`. `kimi-k3` row 14 reports no count. Flagged here because the seed rule literally says "no reasoning tokens".
4. **`NoReasoning` was not used.** `qwen-max` never reports reasoning tokens in any form, but Qwen has no level rows to be "every level rejected", so the D-02 `NoReasoning` measured state was not claimed; it carries `Supported(off)`, the mechanical result.
5. **Grok `auto` is seeded from the default call**, as the plan's rule states for a top-level form (every probed Grok model reported reasoning tokens by default). `grok-4.20-reasoning` therefore takes `auto` only. No Grok `off` row exists, so none is seeded.
6. **Test files / locations.** The per-vendor seed tests live in the adapters' test modules (they read `model_profile(id)`), not in `registry/mod.rs`; `provider_factory.rs` was touched to restate the Kimi and Grok provider-default cases (it is not in the plan's `files_modified`, but the plan requires those restatements). `registry/mod.rs` is unchanged.
7. **Commit hooks:** commits used `--no-verify` as the wave allows; trailer follows the attribution reminder (`Claude Sonnet 5.5`).

## Authentication Gates

None. No live vendor call was made.

## Known Stubs

None. Deliberate limits, all in the CHANGELOG: models the probe did not cover (`kimi-k2.7-code*`, `qwen-turbo`, `grok-4.20-non-reasoning`) refuse an explicit effort until probed.

## Threat Flags

None. No endpoint, auth path or file access was added. T-51-34 (an unconfirmed wire or desk-research level seeded) is mitigated by the owner checkpoint, the seed rules, the per-row source comments and the seed-matches-transcript tests; T-51-35 (the Kimi regression pinned against the probe) is mitigated by writing it after the decision, from quoted rows.

## Self-Check: PASSED

- Files: this SUMMARY and every file in `key-files.modified` present; `git status` clean before this file.
- Commits `5441363f` and `859422af` present on `worktree-agent-a97c54fd9121928c0`.
- `grep -nE 'kimi_pre_probe_|superseded by plan 51-11' crates/paladin-llm/src/kimi/adapter.rs` prints nothing; `kimi_preset_never_emits_top_level_reasoning_effort` exists and passes.
