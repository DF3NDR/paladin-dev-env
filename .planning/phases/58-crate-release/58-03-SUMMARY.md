---
phase: 58-crate-release
plan: 03
subsystem: release-docs-gate
tags: [shell-guard, docs, examples, model-id, df-18, tdd, ship-10]

requires:
  - phase: 58-crate-release
    provides: plan 58-01 CI-health baseline
provides:
  - scripts/check-doc-models.sh, an offline gate that fails on the retired OpenAI id in any copyable form in README.md, docs/src and examples/
  - make check-doc-models, a member of make check-gates, enforced in CI through the harness's real-tree case
  - 42 docs/src and 74 examples/ token-only edits so every copyable model id is one the registry seeds
  - count-free wording at docs/src/appendix/release-checklist.md:25
affects: [58-05]

tech-stack:
  added: []
  patterns:
    - "Fixture-proven gate: committed failing first (RED), then the script (GREEN)"
    - "A sweep is proven mechanical by a word diff whose only tokens are the old and new id"
    - "The harness's last case runs the real tree, so make test-shell-guards is the CI enforcement"

key-files:
  created:
    - scripts/check-doc-models.sh
    - scripts/check-doc-models.allowlist
    - tests/scripts/check-doc-models_test.sh
    - tests/scripts/fixtures/check-doc-models/ (7 fixture trees)
  modified:
    - Makefile
    - docs/src/ (18 files)
    - examples/ (38 files)
    - docs/src/appendix/release-checklist.md

key-decisions:
  - "Allowlist holds one entry: the sentinel.md vision-troubleshooting line that names the retired non-vision id as the wrong choice; swapping a served id in would make its 'no vision' claim false"
  - "gpt-4o-mini is the replacement everywhere except two docs/src sites where prose contrasts a stronger model (gpt-5.4); examples/ has no such site"
  - "Three unflagged printed or commented descriptions of the same configuration were swept with their code so the example text matches it"

patterns-established:
  - "DEPRECATED_IDS is one bash array; a later deprecation is a one-line change"
  - "An allowlist entry that exempts no hit fails STALE_ALLOW; zero files scanned fails ZERO_FILES"

requirements-completed: []
requirements-progress: "SHIP-10 gate tooling for DF-18 is in place; SHIP-10 itself is earned by plan 58-14 after the tag"

duration: not measured
completed: 2026-11-05
status: complete

coverage:
  - id: D1
    description: "The gate flags the retired id in double-quoted, single-quoted, backticked, YAML-value and --model forms, and not prefix lookalikes or illustrative output lines"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "RED after 1c71e2b24 (script absent, harness exit 1); GREEN after bf0c25ab2: harness 7 assertion(s) passed; shellcheck clean (no severity filter)"
        status: pass
    human_judgment: false
  - id: D2
    description: "No copyable deprecated model id remains in README.md, docs/src or examples/, and every edit is token-only"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "gate on the real tree: OK: 184 file(s) scanned, 0 deprecated model literal(s) (allowlisted: 1); word diffs of 8cfd7ad33 and 2bcc91263 hold only -gpt-4 and +gpt-4o-mini or +gpt-5.4"
        status: pass
    human_judgment: false
  - id: D3
    description: "The gate is held in make check-gates and in CI through the harness's real-tree case"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "e809453d3: make check-gates exit 0; make test-shell-guards exit 0 (check-doc-models harness: 8 assertion(s) passed); no file under src, crates, tests/fixtures or tests/helpers changed"
        status: pass
    human_judgment: false
---

# Phase 58 Plan 03: Deprecated model-id gate and sweep Summary

**A fixture-proven `check-doc-models` gate now holds README.md, docs/src and examples/ to model ids the registry serves: the retired `gpt-4` is gone from 116 copyable sites (the Quickstart's `.model("gpt-4")` first), and the gate is a `check-gates` member enforced in CI through its harness.**

## Task Commits

| Order | Commit | Message |
|-------|--------|---------|
| 1 | `1c71e2b24` | `test(58-03): fixtures for the deprecated model-id gate (DF-18)` |
| 2 | `bf0c25ab2` | `chore(58-03): add the check-doc-models gate` |
| 3 | `e11d675f2` | `docs(58-03): use a served OpenAI model id in the Quickstart (DF-18)` |
| 4 | `8cfd7ad33` | `docs(58-03): replace the deprecated OpenAI model id across docs/src (DF-18)` |
| 5 | `6148899a9` | `docs(58-03): drop the stale crate count from release-checklist.md (Phase 47 carried finding)` |
| 6 | `2bcc91263` | `docs(58-03): replace the deprecated OpenAI model id across examples/ (DF-18)` |
| 7 | `e809453d3` | `chore(58-03): hold docs and examples to served model ids in check-gates and the harness` |

The `test(58-03)` commit precedes the `chore(58-03)` gate commit. Task 1 is a tracer: its whole `<verify>` is mechanical (exit codes, counts, grep), it passed in full, and the sweep continued without a checkpoint under the orchestrator's pre-approval.

## Recorded results

- **RED** (after `1c71e2b24`): the harness exits 1, `ERROR: guard script not found`.
- **GREEN** (after `bf0c25ab2`): harness `7 assertion(s) passed` (quoted, single_and_backtick, yaml_and_flag, stale_allow, zero_files as failing cases; prefix_lookalikes and allowlisted as passing cases). `shellcheck scripts/check-doc-models.sh tests/scripts/check-doc-models_test.sh` is clean with no severity filter (the first draft carried four SC2001 style notes; `indent_output` removed them).
- **Real tree, red** (after the gate, before any sweep): `FAIL: 115 deprecated model literal(s) in 57 file(s)`. After the Quickstart line: `FAIL: 114 ... in 56 file(s)`.
- **Real tree, green** (after the sweeps): `OK: 184 file(s) scanned, 0 deprecated model literal(s) (allowlisted: 1)`.
- **Final harness** (`e809453d3`): `8 assertion(s) passed` (the eighth is the real-tree case). `make check-gates` exit 0. `make test-shell-guards` exit 0.

## Site counts

| Tree | Before (plan-time head, per gate) | After |
|------|-----------------------------------|-------|
| `docs/src` | 44 hits in 19 files (39 double-quoted after the Quickstart fix, plus `platform-api.md:773` single-quoted, `cli-muster.md:147` YAML value, `cli-usage.md:567` `--model` flag, `cli-usage.md:1038` backticked) | 0 (1 allowlisted) |
| `examples/` | 71 hits in 38 files (69 double-quoted, plus the YAML values at `paladin_with_rag.yaml:27` and `document_processing.yaml:10`) | 0 |
| `README.md` | 0 | 0 |

The todo counted 40 double-quoted sites in 18 `docs/src` files; the gate also finds the four non-double-quoted docs forms the todo listed as "unquoted copyable forms", which is why docs/src reads 44.

Edits: `8cfd7ad33` changed 42 docs/src lines in 18 files (the allowlist file is the 19th file in that commit); `2bcc91263` changed 74 examples lines in 38 files (71 gate sites plus 3 below). The Quickstart edit (`e11d675f2`) is one line.

## Token-only proof (word diff)

`git diff -U0 --word-diff=porcelain --word-diff-regex='[A-Za-z0-9._-]+'` of each sweep commit, changed-token tally:

| Commit | Removed | Added |
|--------|---------|-------|
| `8cfd7ad33` (docs/src) | 42 x `gpt-4` | 40 x `gpt-4o-mini`, 2 x `gpt-5.4` |
| `2bcc91263` (examples/) | 74 x `gpt-4` | 74 x `gpt-4o-mini` |

No other token changed. `make check-doc-config`: `161 YAML block(s) checked, 0 failed`. The twelve changed `examples/` YAML files all parse with PyYAML `safe_load_all` (the parser `check-doc-config.sh` uses): `12 yaml files parsed, 0 failed`. `cargo fmt --check` is clean over the changed example sources.

## Every non-default choice and every allowlist entry

**`gpt-5.4` (prose contrasts a stronger model), 2 sites, both docs/src:**

- `docs/src/appendix/cli-usage.md:1038` - "Use `gpt-5.4` for complex reasoning", set against `gpt-3.5-turbo` for simple tasks on the line above.
- `docs/src/operations/performance-tuning.md:167` - the `complex_tasks` route, set against the `simple_tasks` route on `gpt-3.5-turbo`.

**Token-counting sites, 2, both `gpt-4o-mini`:** `docs/src/getting-started/configuration.md:249` (`tokenizer:`) and `docs/src/user-guides/memory-management.md:159` (`TiktokenCounter::new`). `TiktokenCounter::new` calls `tiktoken_rs::get_bpe_from_model`; tiktoken-rs 0.6.0 maps the `gpt-4o-` prefix to `o200k_base`, so `gpt-4o-mini` resolves (no allowlist entry needed, and it is the right encoding for that model).

**Allowlist, 1 entry** (`scripts/check-doc-models.allowlist`):

- `docs/src/appendix/sentinel.md:877` (substring `// Not .model("gpt-4")  // ❌ No vision`). Reason: the troubleshooting line deliberately names the retired non-vision id as the wrong choice for vision work, directly under `.model("gpt-4o")  // ✅ Supports vision`; swapping in a served id (all of which support vision) would make the "no vision" claim false. This is not a way to avoid an edit (P-03-2); it is a line whose meaning depends on the id.

**Judgment calls on the server-response example:** `docs/src/api-reference/platform-api.md:164`, `:773` and `:774` (the `model_unpriced` halt reason and 422 body) now name `gpt-4o-mini`. They are response examples for a model with no `treasurer.pricing` row, an operator-configured table, so any model id illustrates them; the three sites stay consistent with each other.

## Deviations from Plan

**1. [Rule 1 - Bug] Three unflagged lines swept with the code they describe.** `examples/commander_grove.rs:225` (`println!("   • LLM Model: gpt-4")`), `:238` (a comment saying the config "Matches the "LLM Model: gpt-4" configuration printed above") and `examples/autonomous_full_config.rs:401` (`println!("  model: gpt-4")`) describe the same configuration whose `routing_model` and `.model(...)` the gate flagged and the sweep changed. Left alone, the example would print a model its code no longer uses and a comment would cite text that no longer matches. They are token-only (`gpt-4` to `gpt-4o-mini`, word diff clean), inside the examples commit `2bcc91263`. The gate does not flag them (an output line and a comment are not copyable forms), so the plan's "change nothing else" was read as "change nothing beyond the id token", not "change only gate hits".

**2. [Observation] Shellcheck style notes.** The plan's verify runs bare `shellcheck`, which also fails on style-level notes (the repo's `lint-shell` uses `--severity=warning`). The harness was written to be clean at every severity.

Otherwise the plan executed as written.

## Hand-offs

- **13 exact `gpt-4` tokens remain in the scanned trees, all deliberately not flagged** (illustrative output or option lists, not copyable model values): `docs/src/appendix/grove.md:944`, `cli-setup-check.md:106` and `:281`, `cli-usage.md:234`, `cli-onboarding.md:102` and `:118`, `getting-started/configuration.md:445` (a pricing-table key `gpt-4:`), `operations/troubleshooting.md:290`, `examples/cli_configs/paladin_full_config.yaml:63` and `:263`, `basic_paladin.yaml:40`, `advanced_paladin.yaml:42`, plus the allowlisted `sentinel.md:877`. Whether the pricing-table key at `configuration.md:445` should name a served id is a docs-accuracy call for a register row in plan 58-05.
- **Rust sites outside this plan's scope (P-03-1):** `"gpt-4"` appears at 643 sites in 126 files under `src/`, `crates/` and `tests/`, including the `GarrisonConfig` default `tokenizer: "gpt-4".to_string()` at `crates/paladin-memory/src/config/garrison.rs:35` and `:106`. None was edited. They are the register row for plan 58-05, with the registry-membership extension of the gate (`gpt-4o` 66 sites, `gpt-3.5-turbo` 6, `gpt-4-turbo` 3, serving status never measured).
- CI's Example Muster job reads the example string-literal changes on the plan 58-06 push; a string literal cannot alter compilation.
- SHIP-10 is not ticked here; plan 58-14 earns it after the tag.

## Known Stubs

None.

## Threat Flags

None. The gate is offline and read-only; no network, auth or schema surface was added.

## Self-Check: PASSED

- `scripts/check-doc-models.sh`, `scripts/check-doc-models.allowlist`, `tests/scripts/check-doc-models_test.sh` and `tests/scripts/fixtures/check-doc-models/` exist.
- `1c71e2b24`, `bf0c25ab2`, `e11d675f2`, `8cfd7ad33`, `6148899a9`, `2bcc91263`, `e809453d3` present in `git log`.
- `git diff --name-only 1c71e2b24~1 HEAD -- src crates tests/fixtures tests/helpers` is empty.
