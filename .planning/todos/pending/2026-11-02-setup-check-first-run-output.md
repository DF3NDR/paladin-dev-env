---
created: 2026-11-02T00:00:00Z
title: paladin-cli setup-check section order is unstable, keyless runs show bare headings, and the CLI's own messages name `paladin`
area: cli
severity: minor
files:
  - src/application/cli/commands/setup_check.rs
  - src/application/cli/commands/onboarding.rs
  - src/application/cli/templates/env.rs
  - docs/src/appendix/cli-setup-check.md
owner: repo maintainer
dispositioned_by: Phase 56 plan 56-06 (DF-16, DF-17)
---

## Problem

Found walking the published `0.11.0` CLI with no provider key (DF-16):

- `setup_check.rs` groups its results in a `HashMap<String, Vec<_>>` and prints the map in iteration
  order, so the sections (`System`, `LLM Provider`, `Service`, `Environment`) come out in a different
  order on every run.
- Without `--verbose` the `Skip` rows are not printed, so a keyless user sees `Service:` and
  `LLM Provider:` as bare headings with no rows. The only sign that no provider key is set is the
  `.env` warning. The command exits 2, which `cli-setup-check.md` documents for warnings.
- The sample output in `docs/src/appendix/cli-setup-check.md` is a different layout from the real one
  (`✓ Paladin CLI: v0.1.0` against the real `✓ Paladin CLI v0.11.0`; `Providers:` and
  `Services (Optional):` against `LLM Provider:` and `Service:`).
- The strings the binary prints name `paladin`, not `paladin-cli` (DF-17): the `.env` warning and the
  Next Steps box in `setup_check.rs`, `onboarding.rs` lines that say "Run 'paladin onboarding'" and
  "Run 'paladin setup-check'", and the `.env` header comments in `templates/env.rs`. Plan 56-07 fixes
  the docs lines only; these are code changes.

## Why it is parked (S3, DF-16)

Every fix touches source (`setup_check.rs`, `onboarding.rs`, `env.rs`) and some of it changes printed
output that a test may pin. That is more than a contained docs or example edit (D-05), and none of it
affects a result a script reads (the exit codes are right).

## When picked up

Print the categories in a fixed order (a `Vec` of categories, or a `BTreeMap` keyed by an order), print
a one-line "not configured" row for each skipped provider even without `--verbose`, replace `paladin`
with `paladin-cli` in the binary's own messages, then regenerate the sample on `cli-setup-check.md`
from a real run.
