---
created: 2026-11-05T00:00:00Z
title: "Thirteen unflagged illustrative gpt-4 tokens remain in docs and examples, including the pricing-table key at docs/src..."
area: docs-models
severity: minor
files:
  - docs/src
  - examples
owner: v0.12.0 Phase 1
recheck_by: 2026-11-09
dispositioned_by: Phase 58 plan 58-06 (R-81)
---

## Problem

Thirteen unflagged illustrative gpt-4 tokens remain in docs and examples, including the pricing-table key at docs/src/getting-started/configuration.md:445.

Carried into the v0.11.1 pre-tag audit register as R-81; it is not fixed in v0.11.1 (the milestone is a no-new-features patch).

## What is already true

Re-proof recorded by plan 58-05 on head 9eba21d9b: ran the gate on 9eba21d9b: OK with 1 allowlisted line; 58-03 listed the 13 tokens (grove.md:944, cli-setup-check.md:106,281, cli-usage.md:234, cli-onboarding.md:102,118, configuration.md:445, troubleshooting.md:290, paladin_full_config.yaml:63,263, basic_paladin.yaml:40, advanced_paladin.yaml:42, sentinel.md:877)

## What to do

a docs-accuracy call; whether the pricing-table key names a served id.

Re-check by 2026-11-09 (the maintainer's `DATE`, plan 58-05, `58-CI-EVIDENCE.md` `## Maintainer decisions (58-05)`); owner v0.12.0 Phase 1.

## Source

- Finding: 58-03 hand-off (13 exact tokens, deliberately not flagged)
- Register row: R-81 in `.planning/phases/58-crate-release/58-AUDIT-DISPOSITIONS.md`
