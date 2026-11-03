---
created: 2026-11-03T00:00:00Z
title: Replace the deprecated `"gpt-4"` model literal across the docs and examples, and gate it
area: docs
severity: minor
files:
  - docs/src/getting-started/quickstart.md
  - docs/src/
  - examples/
  - scripts/
owner: repo maintainer
dispositioned_by: Phase 56 plan 56-08 (DF-18)
---

## Problem

Found by the maintainer's keyed first-run walk on the published `0.11.0` crates (DF-18, K2). The
Quickstart's `.model("gpt-4")` (`docs/src/getting-started/quickstart.md:53`, identical at tag `v0.11.0`)
fails against a real OpenAI key:

```
Error: LlmFailure { transience: Permanent, status: None, provider: None, message: "Model not available: ...
  \"message\": \"The model `gpt-4` has been deprecated, learn more here: https://platform.openai.com/docs/deprecations\",
  \"code\": \"model_not_found\" ... }
```

The same program with `.model("gpt-4o-mini")` returns a completion (32 tokens, 1390 ms).

The exact literal `"gpt-4"` appears at **109 sites in 54 files** on HEAD dc190dba6 (counted with
`grep -rn '"gpt-4"' docs/src examples README.md`): 40 sites in 18 files under `docs/src`, 69 sites in 36
files under `examples/`, none in `README.md`. Other `gpt-4*` ids in the same trees: `gpt-4o` (72 mentions),
`gpt-4o-mini` (15), `gpt-4-turbo` (4), `gpt-4-vision-preview` (2), `gpt-4-32k` (2); whether each of those is
still served was not checked.

## Why it is parked (S3, DF-18)

A sweep of 109 sites across 54 files, 36 of them compiled example files, is more than a contained docs
change (D-05): each site needs a decision (a model that the surrounding prose, token counts and any
mock-based test still fit), and a one-off sweep goes stale again without a gate. The Quickstart line alone
is a contained one-line edit, but fixing one of 109 would leave the root cause in place.

## When picked up

1. Pick the replacement from the seeded registry, not from memory. The seeded OpenAI rows
   (`crates/paladin-llm/src/registry/mod.rs`) prefer `gpt-4o-mini` and `gpt-5.4-nano` (also `gpt-5.4`,
   `gpt-5.4-mini`); the maintainer's walk proved `gpt-4o-mini` end to end.
2. Replace `"gpt-4"` in `docs/src` and `examples/` (Quickstart first), then re-run `make check-doc-examples`
   and the examples build.
3. Add a `check-doc-models` gate (a script plus a Makefile target, in the style of
   `scripts/check-doc-config.sh` and `scripts/check-version-strings.sh`) that fails when a copyable
   OpenAI model literal in `docs/src`, `examples/` or `README.md` is not a model the seeded registry knows.
   An allowlist is needed for pages that deliberately show a deprecated id as a migration example.
