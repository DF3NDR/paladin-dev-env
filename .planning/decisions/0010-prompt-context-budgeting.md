# ADR-0010: Prompt/context budgeting against declared `max_context_tokens`

## Status

Accepted

**Date:** 2026-08-04

## Context

`ProviderCapabilities.max_context_tokens` (`crates/paladin-ports/src/output/llm_port.rs:827`,
doc "Maximum context window size in tokens (None if unlimited or unknown)") is declared by every
LLM adapter shipped in this workspace — Anthropic (`adapter.rs:554` = `200_000`), OpenAI
(`adapter.rs:652` = `128000`), DeepSeek (`adapter.rs:811` = `64000`), Mock (`mock.rs:270` =
`4096`) — and is exposed by the non-async, infallible `LlmPort::get_capabilities()`
(`llm_port.rs:1333`). Before this decision, it had **zero production readers**. Every
non-`Default` reference in the tree was a doc comment or a test fixture.

Downstream (`/workspace`, the audit application built on this framework), four agents each
hand-rolled an incompatible, uncoordinated bounding scheme against this same gap: `tribunal.rs`
(8000/1200/240), `fuzz.rs` (4000), `design_doc_enrichment.rs` (2048), and, after quick task
`260803-o7o`, `deductive.rs` with a hardcoded mirror of DeepSeek's `64000`. The measured failure
that motivated closing the gap at the framework layer rather than adding a fifth hand-rolled
scheme: an unbounded deductive-agent prompt of 465,346 characters (≈93k-166k tokens depending on
tokenizer assumption) was sent against DeepSeek's declared 64,000-token window, twice, in the
same live sweep, both times returning zero output. The control that proves the cause was prompt
size rather than a low ceiling: the triage agent, same provider, same model, same 24,000-token
completion budget, empty source context, still registered 58 candidates in that sweep — the
ceiling itself was never the constraint; the unmeasured, unenforced prompt was.

`PaladinBuilder` already had an ADR-blessed precedent for exactly this shape one field over:
`paladin_builder.rs:1112-1131` reads `get_capabilities().temperature_range` pre-flight and
**errors rather than silently clamping** an out-of-range value (ADR-0004). No equivalent existed
for the input side — the assembled prompt — despite the declared window sitting on the same
struct, read through the same method, with the same "declared but unenforced" shape.

## Decision

**Measurement and enforcement of the provider's declared context window are FRAMEWORK concerns,
owned by a new `Quartermaster` service in `paladin-llm`. WHICH material to shed under scarcity is
caller-supplied policy — no audit-specific (or any other application-specific) priority scheme
crosses into this framework.**

- `Quartermaster::verify_fits` is the pre-flight GUARD: it measures an already-assembled prompt
  against the provider's declared window (minus the caller's reserved completion budget) and
  returns `QuartermasterError::ContextOverflow { measured_tokens, allotted_tokens, provider }`
  when it would overflow. It never trims. This explicitly inherits ADR-0004's stance — fail
  loudly pre-flight, never silently clamp — applied to the input side instead of the
  output-side `temperature`.
- `Quartermaster::apportion` is a bounded ALLOCATOR: given fixed (non-sheddable) material and a
  `Convoy` of caller-prioritised, shed-or-truncate-able `ConvoyItem`s, it returns an `Allotment`
  in which every retained item is clamped to a per-item byte share (truncated with a visible
  marker when a cut was needed) and every shed item is recorded in `Allotment.shed` with its
  label, priority, and original size. **Nothing is dropped silently.** This explicitly rejects
  the pre-existing anti-pattern at
  `paladin-memory/src/services/rag_retrieval_service.rs:171`'s `truncate_to_token_budget`, which
  drops lowest-scoring items with no marker and no record.
- **A provider that declares no window (`max_context_tokens: None`) gets no invented window.**
  `Quartermaster::new` returns `QuartermasterError::UndeclaredContextWindow` unless the caller
  supplies `AllotmentConfig::fallback_context_tokens` explicitly. Guessing a window here would
  produce a false sense of enforcement — worse than no enforcement, because it looks like one.
- **Honesty clause.** Neither `claude-*` nor `deepseek-*` has an exact tokenizer available
  offline. For these, `Quartermaster` measures through `EstimatingTokenCounter`, a deliberately
  pessimistic (over-counting) estimator pinned at `PESSIMISTIC_TOKENS_PER_1000_BYTES = 358` — the
  reciprocal of the most pessimistic end of the measured 2.8-5.0 bytes/token range recorded during
  debug session `deductive-32000-zero-output` (no DeepSeek tokenizer was available offline to
  measure exactly). `TokenCounter::is_exact()` and the propagated `Allotment.exact_tally` make
  this an observable property of the type rather than an assumption a future reader has to
  rediscover: a guard built on an estimate must be conservative, and this ADR says so rather than
  letting a future reader mistake the number for exact. Separately: a provider's declared window
  is the provider's own metadata and can itself go stale — enforcement here is against what the
  adapter declares, which is the best available truth, and is now at least enforced rather than
  silently ignored.
- **The `TokenCounter` trait is LIFTED to `paladin-ports`, not renamed.** It previously lived at
  `paladin-memory/src/garrison/token_counter.rs:12`, gated behind `paladin-memory`'s optional
  `content-processing` feature (which pulls in `tiktoken-rs`). `paladin-llm` needed to measure
  material without depending on `paladin-memory` or on `tiktoken-rs` — so the CONTRACT moved to
  `paladin-ports::output::token_counter_port`, while the tiktoken-backed ADAPTER
  (`TiktokenCounter`) stays in `paladin-memory` exactly where it was. This is a **naming
  exception**, recorded here so it does not read as an oversight: renaming a pre-existing,
  three-times-re-exported public trait for a move with no functional gain would be a needless
  breaking change. The three existing re-export sites (`paladin-memory/src/prelude.rs`,
  `src/infrastructure/adapters/garrison/mod.rs`) compile unchanged against the lifted trait.
- **Vocabulary.** `Quartermaster` (the officer who allocates supplies under scarcity),
  `Convoy`/`ConvoyItem` (the material carried in for apportioning), `apportion()` (the verb),
  `Allotment` (the result). Three earlier candidate names were rejected on research, not taste,
  and are recorded here so they are not re-proposed: `Muster` collides with the framework's own
  troop-assembly vocabulary (`paladin muster` CLI command, `Commands::Muster`); `provision()`
  collides with `SandboxPort::provision` (container lifecycle, a Phase 1 success criterion
  downstream); `ContextRation`/`Allocation` collide with live domain vocabulary downstream
  (`rationale` is a load-bearing judge/decision term; `allocation` is the Prover's on-chain
  entitlement invariant, `paid(x) <= allocation(x)`).

## Considered Options

- **Framework-owned measurement + enforcement, caller-owned shedding policy** (chosen) — matches
  the ADR-0004 precedent already shipped one field over on the same struct; keeps `paladin-llm`
  free of any audit-specific (or other application-specific) priority logic; gives every current
  and future adapter one shared, tested implementation instead of a fifth (and sixth, and
  seventh) hand-rolled bounding scheme.
- **Leave bounding entirely to each downstream agent** — rejected. This is the status quo that
  produced the measured failure: four incompatible schemes already existed downstream with no
  shared measurement or enforcement, and Phase 39's planned specialist roster would have made it
  five or six without a shared primitive to converge onto.
- **Silently clamp an over-budget prompt at the framework layer** — rejected. Clamping a caller's
  material without telling them is exactly the anti-pattern ADR-0004 already rejected for
  `temperature`, and exactly the anti-pattern already shipped at
  `rag_retrieval_service.rs:171`, which this ADR explicitly names as the failure mode `apportion`
  must not repeat.
- **Guess a context window when the provider declares none** — rejected. A guessed window
  produces a false sense of enforcement, which is worse than an explicit, typed refusal
  (`UndeclaredContextWindow`) that names the gap and asks the caller to supply policy explicitly
  via `fallback_context_tokens`.
- **Treat the non-exact-tokenizer estimate as if it were exact** — rejected. `claude-*` and
  `deepseek-*` have no offline exact tokenizer; presenting an estimate as an exact count would be
  a false assurance on the very guard meant to prevent one. `is_exact()`/`exact_tally` make the
  distinction a type-level property instead.

## Code Locations

- `crates/paladin-ports/src/output/token_counter_port.rs` — the lifted `TokenCounter` port trait,
  `TokenCountError`, `PESSIMISTIC_TOKENS_PER_1000_BYTES`, `EstimatingTokenCounter`.
- `crates/paladin-memory/src/garrison/token_counter.rs` — `TiktokenCounter`, re-pointed to the
  lifted trait/error, no longer returning `GarrisonError`.
- `crates/paladin-llm/src/services/quartermaster.rs` — `Quartermaster`, `AllotmentConfig`,
  `Convoy`/`ConvoyItem`, `Allotment`/`ApportionedItem`/`ShedItem`, `QuartermasterError`.
  `Quartermaster::from_port` (`:352`) reads `LlmPort::get_capabilities()` — the first production
  reader of `max_context_tokens`. `Quartermaster::verify_fits` (`:521`) is the pre-flight
  enforcement point.
- `src/lib.rs` — the unconditional re-export block making `Quartermaster` and its supporting
  types available at `paladin::` alongside the existing `LlmProviderFactory` short-path alias.
- `crates/paladin-llm/src/deepseek/adapter.rs` — `DeepSeekCompletionTokensDetails` and
  `DeepSeekUsage.completion_tokens_details`, surfacing `reasoning_tokens` on the
  `EmptyCompletion` annotation path (a related but separately-scoped observability fix, landed in
  the same plan; does not touch `TokenUsage`).

## Code Conformance

must change

**LLMR-04** (Phase 41, downstream `/workspace` requirement) is the executing requirement. This
plan (41-09) lands the framework-side capability — `Quartermaster::from_port` gives
`max_context_tokens` its first production reader, `verify_fits` and `apportion` give it
enforcement. **LLMR-04 itself remains `Pending`** at the close of this plan: it spans both this
plan and 41-10 (the downstream consumer, `crates/audit-agents/src/deductive.rs`), and is only
satisfied once a live Tare sweep confirms the discriminating prediction that measured/enforced
budgeting actually prevents the class of failure this ADR's Context section describes.

## Downstream Consumers

- **Plan 41-10** (`crates/audit-agents/src/deductive.rs`) — the first confirmed consumer,
  replacing the hardcoded `DEDUCTIVE_PESSIMISTIC_TOKENS_PER_1000_BYTES` mirror of DeepSeek's
  `64000` with a `Quartermaster::from_port`-driven budget.
- **Deferred to Phase 39 / SKILL-01**: retrofitting `crates/audit-agents/src/tribunal.rs`
  (8000/1200/240), `fuzz.rs` (4000), and `design_doc_enrichment.rs` (2048) onto the
  `Quartermaster`. Their existing hand-rolled bounds are functional and were live-verified;
  converging them is a coordinated multi-stage change with its own regression surface, and Phase
  39 / SKILL-01 is also where the new specialist roster (`run_analysis_phalanx`) would otherwise
  add a fifth or sixth hand-rolled scheme if this capability did not already exist.
- **Deferred, framework-internal**: migrating this framework's own existing call sites
  (`paladin_execution_service`, `prompt_generation_service`,
  `rag_retrieval_service.rs:171`'s silent drop-tail) onto the `Quartermaster` — a larger,
  separately-scoped change; this ADR records the intended direction without scheduling it.
- **Not a consumer of this ADR**: judge prompt SEMANTICS (Phase 37's tribunal ratio blowup). This
  capability would have prevented the *economics* half of that incident (a 28,262-byte corpus
  against a ~500-byte candidate) but not its corpus-independent second root cause — a
  `SYSTEM_PROMPT` that never defined what earns `uphold`. Prompt economics are a framework
  concern; prompt semantics stay per-agent. This ADR does not touch judge rubric text.
