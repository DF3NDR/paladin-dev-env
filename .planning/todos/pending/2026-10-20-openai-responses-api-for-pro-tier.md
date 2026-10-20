---
created: 2026-10-20T18:00:00Z
title: Add OpenAI Responses API support so the pro tier (gpt-5.5-pro and siblings) is usable
area: llm-adapters
severity: minor
files:
  - crates/paladin-llm/src/openai/adapter.rs
  - crates/paladin-llm/examples/live_vendor_smoke.rs
  - .planning/phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md
owner: repo maintainer
---

## Problem

The owner's goal for the LLM adapters is that the latest active model of every provider, plus a few select others, is
usable. OpenAI's pro tier is one of the select others: `gpt-5.5-pro` (newest), `gpt-5.4-pro`, `gpt-5.2-pro` and
`gpt-5-pro`. It is not reachable through Paladin. The 50.1 seed probe (`50.1-SEED-PROBE.md`, run 1, 2026-10-20, commit
`8ad298dd`) got a 404 on the default call of all four on `POST /v1/chat/completions`:

- three replied "This is not a chat model and thus not supported in the v1/chat/completions endpoint";
- `gpt-5-pro` replied that it is only supported in `v1/responses`.

`OpenAIAdapter` speaks Chat Completions only.

## Solution sketch

1. Add a Responses-API (`/v1/responses`) wire to the OpenAI adapter, selected per model (registry flag or seeded
   profile field), behind the existing `LlmPort` surface so consumers see no API change.
2. Teach `live_vendor_smoke --probe` the Responses wire.
3. Probe the pro tier on the operator's keys and seed the rows measured end to end (ADR-0061 rule).

Owner decision recorded 2026-10-20 during `/gsd-execute-phase 50.1` (see `50.1-PROBE-CANDIDATES.md`
`## Final candidate set`): deferred out of Phase 50.1 to a follow-up phase.
