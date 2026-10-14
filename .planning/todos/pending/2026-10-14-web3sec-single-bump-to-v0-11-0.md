---
created: 2026-10-14T22:10:00Z
title: web3sec moves once, straight to v0.11.0, after Phases 50 and 51
area: downstream-consumer
severity: major
files:
  - MIGRATION.md (section 9.8, "Library consumers: v0.10.1 -> v0.11.0")
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
