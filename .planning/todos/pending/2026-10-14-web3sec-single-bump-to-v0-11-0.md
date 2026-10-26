---
created: 2026-10-14T22:10:00Z
title: web3sec moves once, straight to v0.11.0, after Phases 50 and 51
area: downstream-consumer
severity: major
files:
  - MIGRATION.md (section 10.8, "Library consumers: v0.10.1 -> v0.11.0")
  - tests/consumer_surface_muster.rs
  - .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-HANDOFF-web3sec.md
owner: repo maintainer (relay to web3sec)
deferred_past: v0.11.0 tag (Phase 47)
recheck_by: Phase 47 close
dispositioned_by: Phase 49 scope cut (2026-10-14)
---

## Problem

Phase 49 was built from the web3sec handoff of 2026-10-08, which targets `v0.10.1`. The tree the
consumer actually meets next is `v0.11.0`, which already carries the Phase 44 and Phase 48 breaks
on top of the handoff's B-1/B-2/B-3; the Phase 49 research counted roughly three times the
handoff's break list. Moving web3sec to `v0.10.1` now would break it again at `v0.11.0` and
contradict the Milestone 13 section 5 rule the handoff cites (one coordinated pointer bump).

## What to do (consumer side, after Phases 50 and 51 land and v0.11.0 is tagged)

1. Pin the `paladin` submodule detached on the `v0.11.0` tag, one commit carrying the pointer,
   the adaptation and `Cargo.lock`. The local-only submodule commit `4b7bed87` is superseded
   (its seven planning files are byte-identical in `v0.10.1`).
2. Use `MIGRATION.md` section 9.8 "Library consumers: v0.10.1 -> v0.11.0" (DR-1..DR-11 plus the
   no-total-only `TokenUsage` pitfall) as the adaptation spec, and extend it with whatever
   Phases 50 and 51 add to section 9.2 before the tag. Carry a full `TokenUsage` through
   `RoutineReport` (the handoff's D-2 lossless route), not a total-only value.
3. Before the bump, inventory web3sec's real `use` lines and reconcile them with
   `tests/consumer_surface_muster.rs`: the handoff's Appendix A is as of pin `35fd839` and is wrong
   on at least one item (`mcp_protocol::looks_like_auth_failure` is private at every revision).
   Add any missing import to the muster; drop pins the consumer no longer takes.
4. Consumer gates on the new pin: `cargo fmt --check`, `cargo clippy --workspace --all-targets
   -- -D warnings`, `cargo check --workspace`, `cargo test --workspace` (baseline 2,592 / 0 / 2).
5. Return to Paladin: consumer commit SHA, D-2 choice, gate totals, divergences (handoff section 9).

## What Paladin keeps from Phase 49

- `tests/consumer_surface_muster.rs` with its `[[test]]` table (`required-features =
  ["llm-anthropic", "llm-deepseek"]`) and `make check-consumer-surface`: a compile-only guard
  over the surface the consumer imports, proven red in `49-MUSTER-RED-EVIDENCE.md`. Phases 50 and
  51 change that surface, so expect the muster to go red there and update it as part of those
  phases' registered breaks.
- `MIGRATION.md` section 9.8 library-consumer note, an input to Phase 46's migration guide.

## Update 2026-10-16 (Phase 50 closeout, plan 50-16)

Phase 50's breaks and additions are now in `MIGRATION.md` 9.2 and the 9.8 consumer table, and in the
root and per-crate CHANGELOGs: 9.8 gains DR-12 through DR-16 (the nine `*Config` types,
`LlmProviderConfig`, `ModelFallbackConfig` and `AgentRuntimeDeps`, `PaladinResult.run_metadata` with
the streaming `metadata` fields, `FallbackChainError`) plus a behaviour list for what the consumer
meets at run time (an explicit Anthropic temperature is refused at build, `EmptyCompletion` and
`LlmError::Refusal` on every provider, the 300 s reasoning default, adapter-wide `context_window`
beating seeded windows). `tests/consumer_surface_muster.rs` now also pins `create_from_config`,
`LlmConfig`, `LlmProviderConfig`, `ModelOverride`, `ModelProfile`, a `LlmError::Refusal` arm,
`FallbackStep` and `from_steps`, `LlmResponse::new` and the `AnthropicConfig` knob builders.

Still open on the consumer side, so the todo stays pending until the consumer moves after Phase 51:
the bump itself, its own gates, and the choice among the drifted `new(..)` signatures
(`2026-10-16-harmonise-llm-config-new-signatures.md`), which is a second break and not in v0.11.0
unless the owner rides it on this bump.

## Update 2026-10-19 (Phase 50.1 inserted)

web3sec pre-measured the bump against the Phase 50 tip `21426037` (112 errors in 3 crates, all on
registered rows, suite back at 2,592 / 0 / 2; re-measured at the sealed `a1128fc1`: 147 with DR-17's
35, workspace clean once fixed) and sent a fourth handoff (2026-10-18, re-verified 2026-10-19) asking for six
consumer-surface fixes before the tag. It is filed as **Phase 50.1** (SURFACE-01..06), sequenced
after Phase 50 and before Phase 51. Consequences for this todo:

- The harmonisation of the nine `new(..)` signatures **does** ride this bump now (SURFACE-02, owner
  decision 2026-10-18); the consumer meets it together with DR-1..DR-18 and Phase 51.
- Step 2's spec gains SURFACE-04's four §9.8 additions (pre-`v0.10.0` P31/P32/P33 pointers, 429 /
  Cadence, decorator `model_profile` forwarding, adapter-wide context window).
- Step 3's reconciliation is pre-done upstream by SURFACE-03 (handoff Appendix B pinned, red proof
  recorded); the consumer returns its real import inventory against that muster.
- web3sec starts its adaptation when 50.1 seals, on a branch pinned to the sealed SHA, and reports
  divergences before Phase 46 freezes the guide; the return list in step 5 is unchanged.

## Update 2026-10-24 (Phase 46)

The "report divergences before Phase 46 freezes the guide" step is satisfied by
`.planning/phases/46-docs-currency-hygiene/46-FEEDBACK-web3sec.md`. Its seven guide corrections
(G-1..G-7) and four muster gaps are folded in (plans 46-11 and 46-06). The adaptation spec is now
`MIGRATION.md` section 10.8, "Library consumers: v0.10.1 -> v0.11.0" (and "Operators: v0.10.1 ->
v0.11.0"), frozen on 2026-10-24 by commit `fc48c010` (plan 46-11); the earlier mentions of section
9.8 above are history, since section 9 is now the v0.10 record.

The remaining steps belong to web3sec after Phase 47: re-pin the `paladin` submodule detached on the
`v0.11.0` tag, run the consumer's gates on the new pin, and send the return report (step 5). Status
stays pending (D-25).

## Update 2026-10-26 (Phase 47 tag cut, hand-off written)

The tag is cut: `v0.11.0` on merge commit `70657919beadbd341a72be2d0107080017a803f0` (PR #59, merge commit),
release run `40396108246` green, 12 of 12 publishable crates registry-verified on crates.io at `0.11.0` via
Trusted Publishing (registry table in `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`).

The outbound hand-off is `.planning/phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md`. Its section 6 records
that the consumer baseline in step 4 above (2,592 / 0 / 2) is web3sec's own figure and is superseded as the latest
on record by 2,683 / 0 / 2 (web3sec's findings of 2026-10-24, `46-FEEDBACK-web3sec.md`); the consumer reports
against its own latest pre-bump figure.

The remaining steps are web3sec's: re-pin the `paladin` submodule detached on the `v0.11.0` tag, run the
consumer's gates on the new pin, and send the return report per the hand-off's section 6. Status stays pending
until web3sec returns (D-15).
