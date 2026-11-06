---
phase: 56-dogfood-findings
plan: 02
subsystem: config-loader
tags: [config-roundtrip, key-path-diff, drift-guard, fixtures, dogf-03, tdd, serde_yaml]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "plan 56-01: scripts/check-findings-log.sh, so every DF entry appended here is checked to parse"
provides:
  - "tests/config_example_roundtrip.rs: 14 tests (13 named plus normalise_decimal) loading the shipped example, a full fixture and a Redis-cadence fixture through Settings::load_from_file and Settings::new, diffing input key paths against re-serialised Settings"
  - "tests/fixtures/config/config.example.full.yml: the example with every commented optional block enabled plus the D-11 probe rows"
  - "tests/fixtures/config/config.example.redis-cadence.yml: the second treasurer.cadence.backend shape"
  - "DF-05 (fixed) and DF-06 (out, todo filed) in the dogfood findings log"
affects: [56-04 prove-then-lock dispositions, 56-10 close gate]

tech-stack:
  added: []
  patterns:
    - "Key-path diff, never value diff: segment vectors (not dotted strings) so gpt-5.5.1 and gpt-5 -> 5 -> 1 stay distinct; one-directional because Option::None serialises as null"
    - "EXEMPT / KNOWN_GAPS / PROSE_COMMENT_LINES tables with a shared stale_rows check, so a row that stops matching fails loudly"
    - "Settings::new() driven from a temp dir under #[serial] with a Drop guard restoring the cwd; no APP_* variable touched"

key-files:
  created:
    - tests/config_example_roundtrip.rs
    - tests/fixtures/config/config.example.full.yml
    - tests/fixtures/config/config.example.redis-cadence.yml
    - .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md
    - .planning/phases/56-dogfood-findings/deferred-items.md
  modified:
    - Cargo.toml
    - config.example.yml
    - tests/fixtures/config/README.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "Followed PD-02-1..PD-02-6 as planned (serde_json::Value walker, serde_yaml dev-dependency for independent input paths, one integration file, pattern-with-wildcard table rows, section/leaf drift guard, temp-dir Settings::new path)"
  - "DF-06 (19-digit unquoted price) graded S4 and re-homed to a todo rather than fixed: the only contained fix would be an example comment edit, which the plan's acceptance criteria rule out (example diff must be exactly the one steps line)"
  - "Added a 14th test, normalise_decimal_trims_only_the_fractional_tail, to pin the helper the unquoted-price test relies on"

patterns-established:
  - "Fixtures derived from a documented example live beside frozen blobs with a README section that says which is which and names the drift guard"

requirements-completed: [DOGF-03]

metrics:
  duration: "~15 min"
  completed: 2026-11-02
  tasks: 3
  files: 9
---

# Phase 56 Plan 02: Config example round-trip Summary

**A key-path diff test that loads the shipped example and a full every-optional-key fixture through the production loaders and proves the only drops are the deliberately unserialised secrets, plus a drift guard that keeps the fixture in step with the example.**

Plan-start SHA: `4f0858a63130acbe79ccd557e5d2491b0f1ab5a5`.

## Tasks and commits

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: verbatim example through `Settings::load_from_file`, walker, planted typo watched red | `7e4766147` |
| 2 | Full fixture, Redis-cadence fixture and `Settings::new` path with stale-checked tables | `20808e4ae` |
| 3 | Drift guard, claimed-complete sections (L5), unquoted-price precision probe | `d22f143c5` |

The tracer gate ran in autonomous mode: the tracer `<verify>` was re-run green before Task 2 started.

## Measured results

- **Missing-path counts.** Verbatim example: 8 (the `llm.<provider>.api_key` paths for openai, anthropic, deepseek, kimi, qwen, grok, gemini and openai-compatible; ollama carries none in the example). Full fixture: 10, i.e. the 9 `llm.<provider>.api_key` paths (the same eight plus ollama) and `treasurer.allowance.webhook.secret`. The plan's "9" for the full fixture predates the webhook secret probe row; the forward diff is exactly the `EXEMPT` set.
- **Loader gaps: 0 loader gaps.** Nothing the fixture writes is rejected or silently dropped by `Settings::load_from_file` or `Settings::new`. `KNOWN_GAPS` is empty. Input paths 258, output paths 319.
- **Watched-red runs.**
  - Typo test: with the walker stubbed to return an empty set, all 3 tracer tests failed; `walker_reports_a_planted_typo_key_through_the_loader` reported `left: {}`, `right: {["timeouts", "defualt_seconds"]}`. Real walker restored: green.
  - Drift guard: with the `max_tokens: 4096` line removed from the full fixture, `full_fixture_carries_every_example_key` failed with `llm / max_tokens`. Fixture restored. A synthetic case (`drift_guard_reports_a_commented_key_missing_from_the_fixture`) also pins that it fires.
  - L5: see below.
- **L5 pre-fix output, verbatim** (`example_sections_claimed_complete_write_every_field` on the unmodified example):

  ```
  config.example.yml claims these sections are written out in full, but the loaded Settings serialises these paths the example does not write:
    agent_runtime.model_fallback.steps
  ```

  Logged as DF-05 (S3, docs) and fixed by writing `steps: []` after `providers: []` in the example's `model_fallback:` block. `git diff --numstat 4f0858a63 -- config.example.yml` is `1 0`. The example's own unit tests, including `example_config_agent_runtime_block_round_trips_to_default`, pass.
- **19-digit price probe (one-off, not committed).** A temp config with unquoted `prompt: 1234567890.123456789` loaded as the string `1234567890.1234567`: the decimal value changed (the `config` crate holds the number as an `f64`). Logged as DF-06, S4 (no credible per-1M-token price has 19 significant digits; the example already shows prices quoted), disposition `out (re-homed: .planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md)`. Credible unquoted values keep their decimal value (`2.50` as `"2.5"`, `10.00` as `"10"`, `0.075`, `15`), pinned by `unquoted_credible_prices_keep_their_decimal_value`; quoted values arrive byte-identical, pinned by `quoted_money_strings_arrive_byte_identical`.
- **`Cargo.lock` no-change check.** `git diff --stat 4f0858a63 -- Cargo.lock` is empty. The only manifest change is `serde_yaml = "0.9"` under `[dev-dependencies]`, already a locked package via the optional `cli` dependency.

## DF entries appended

- **DF-05** (S3, docs): `config.example.yml` claims `agent_runtime` is written out in full but omits `model_fallback.steps`. Disposition: `in v0.11.1`, fixed in this plan.
- **DF-06** (S4, library): an unquoted treasurer price with 19 significant digits loses digits on load. Disposition: `out`, re-homed to a todo.

`./scripts/check-findings-log.sh` parses 6 entries and lists DF-05 and DF-06 as accepted; the only OPEN entries are the pre-seeded DF-01..DF-04. The log diff against the plan-start SHA is `37 0` (no deleted lines).

## Verification

- `cargo test -p paladin-ai --test config_example_roundtrip`: 14 passed, 0 failed. The 13 named tests match the acceptance grep.
- `cargo test -p paladin-ai --lib --features web-server -- example`: 6 passed. `cargo test -p paladin-llm --lib -- example_config`: 1 passed.
- `cargo fmt --check` and `cargo clippy -p paladin-ai --test config_example_roundtrip -- -D warnings`: clean. Clippy flagged a collapsible `if` and a needless `clone` in the new drift-guard code in Task 3; both were fixed before the commit.
- Fixtures: every non-comment `key:` / `api_key:` / `secret:` line carries a `test-key-not-a-secret` value (grep prints `0` for the others); the full fixture has no `${`; no test formats `Settings`, its Debug form or its JSON; no `APP_*` variable is set or read (only mentioned in the module doc).
- No library code changed, so `make api-surface` is unaffected.
- Disk: scoped builds only. Free space went from 4219M to 2031M across the plan; no ENOSPC occurred and no cleanup was needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `stale_rows` and `Row.note` unused at Task 1 commit**
- **Found during:** Task 1, `cargo clippy -D warnings`
- **Issue:** the Task 1 file defined the stale-row helper and the `note` field but nothing used them until Task 2, so clippy failed on dead code.
- **Fix:** moved `exempt_and_known_gap_rows_go_stale_loudly` (a Task 2 behaviour test) into Task 1.
- **Files modified:** `tests/config_example_roundtrip.rs`
- **Commit:** `7e4766147`

**2. [Rule 1 - Bug] Clippy findings in the new drift-guard code**
- **Found during:** Task 3, `cargo clippy -D warnings`
- **Issue:** `collapsible_if` and `cloned_ref_to_slice_refs` in `commented_key_lines` / `commented_pairs_missing`.
- **Fix:** collapsed the condition with `split_once(..).filter(..)` and used `std::slice::from_ref`.
- **Files modified:** `tests/config_example_roundtrip.rs`
- **Commit:** `d22f143c5`

### Plan-precision notes

- The full fixture's measured drop count is 10, not the plan's 9, because the fixture also carries the allowance webhook `secret` probe row; both are `EXEMPT` rows, so the forward diff equals the `EXEMPT` set.
- `normalise_decimal_trims_only_the_fractional_tail` is a 14th test beyond the 13 named in the plan.
- The todo file for DF-06 and `deferred-items.md` are outside the plan's `files_modified` list; D-05 requires the former and the scope boundary rule the latter.

## Deferred Issues

- **`make security` fails on RUSTSEC-2026-0429** (`zip` 3.0.0, severity 7.5, solution `>=9.0.0`; build-dependency of `utoipa-swagger-ui` 9.0.2 via `paladin-web`). `make audit` and `make deny` both report it. `Cargo.lock` is unchanged by this plan and the advisory was published 2026-10-22, so it is not caused by this work and was not fixed. Recorded in `.planning/phases/56-dogfood-findings/deferred-items.md`; it needs an owner decision (bump or an explicit `.cargo/audit.toml` allowlist entry with a reason). The plan's verification line "`make security` passes with the unchanged lockfile" is therefore not met for this reason.

## Known Stubs

None. `KNOWN_GAPS` is intentionally empty (0 loader gaps).

## Threat Flags

None. The plan adds a test, fixtures and one dev-dependency; no new endpoint, auth path or schema.

## Self-Check: PASSED

- Files exist: `tests/config_example_roundtrip.rs`, `tests/fixtures/config/config.example.full.yml`, `tests/fixtures/config/config.example.redis-cadence.yml`, `.planning/todos/pending/2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md`, `.planning/phases/56-dogfood-findings/deferred-items.md`.
- Commits exist: `7e4766147`, `20808e4ae`, `d22f143c5`.
