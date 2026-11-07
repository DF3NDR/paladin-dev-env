---
phase: 56-dogfood-findings
plan: 06
subsystem: dogfood-findings
tags: [first-run-walk, published-crates, cargo-install, paladin-cli, findings-log, dogf-04, df-10, df-11, df-12, df-13, df-14, df-15, df-16, df-17]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-03 56-RUNBOOK-KEYED-WALK.md (K1..K9 slots); 56-05 findings log through DF-09 and scripts/check-findings-log.sh"
provides:
  - "56-FIRST-RUN-WALK.md: provenance (registry source line), published README vs HEAD, 12-row step table, keyless observations, disk ledger"
  - "DF-10..DF-17 in the findings log, each with a D-05 disposition; one re-home todo"
  - "56-RUNBOOK-KEYED-WALK.md ## Verified by the keyless half: commands with exit codes, the proven install line, dated K-step corrections"
affects: [56-07 (contained docs fixes), 56-08 (keyed walk), Stage C, SHIP-10]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md
    - .planning/phases/56-dogfood-findings/56-06-SUMMARY.md
    - .planning/todos/pending/2026-11-02-setup-check-first-run-output.md
  modified:
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-KEYED-WALK.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "The tag's Quickstart pins 0.10.0 (not 0.11.0 as the orchestrator note said), so the walk resolved it as written (0.10.1), then used 0.11.0 for the paladin-* crates as the keyed runbook K2 allows; the pin is DF-13, a duplicate of DF-02 (HEAD pins 0.11.0 and make check-version-strings is green)"
  - "The README has no cargo add line on the tag or on HEAD, but the [dependencies] snippet is a complete way to add the crate, so that absence is recorded as no stumble; the snippet's stale 0.5 requirement is DF-10 (duplicate of DF-02)"
  - "DF-16 (setup-check HashMap section order, bare headings) and the in-binary 'paladin' strings need source changes, so DF-16 is re-homed to a todo and DF-17 (docs naming the wrong binary) is left for 56-07 with the in-binary strings noted as re-homed in the same todo"
  - "DOGF-04 NOT marked complete: the keyed half (plan 56-08) is still to run"

requirements-completed: []

duration: ~20min
completed: 2026-11-02
---

# Phase 56 Plan 06: Keyless first-run walk on published 0.11.0 Summary

**The keyless first-run path was walked on crates.io `0.11.0` in a scratch crate outside any repository, and eight stumbles were logged (DF-10..DF-17): the README snippet resolves `paladin-ai 0.5.1`, the Quickstart pins `0.10.0`, the installation page names a nonexistent `llm-openai` feature, no doc gives a working `cargo install` line for the CLI (and the working form, `--features cli --locked`, is now proven), and the docs and the CLI itself call the binary `paladin` while the installed one is `paladin-cli`.**

## Performance

- **Plan-start SHA:** `a39a338dd`
- **Tasks:** 2 of 2 (1 tracer, 1 auto)
- **Commits:** `4c4d16a60` (Task 1, tracer: README path, DF-10, DF-11), `a813bda35` (Task 2: steps 4-11, DF-12..DF-17, todo, runbook section)
- **Log:** +148 lines in Task 2 and +40 in Task 1, 0 deletions (`git diff --numstat`)
- **Tracer gate (auto mode):** the tracer's verify was re-run after its commit and passed; `Tracer verified end-to-end, expanding`.

## Provenance

```
paladin-ai 0.11.0 registry+https://github.com/rust-lang/crates.io-index
```

`cargo metadata` in the scratch crate (`workspace_root` is the scratch crate itself). The scratch root was created under the session scratchpad with `mktemp -d`; `git -C "$WALK" rev-parse --show-toplevel` failed (exit 128); `grep -c 'path = '` on `Cargo.toml` printed `0`; no `[patch]`. The published README read for the walk is byte-identical to `git show v0.11.0:README.md`.

## Step results

| # | step | exit | result | DF |
|---|------|------|--------|----|
| 1 | scratch crate, `cargo add paladin-ai@0.11.0` | 0 | locked 419 packages | no stumble |
| 2 | README `[dependencies]` snippet as published | 0 | resolves `paladin-ai 0.5.1`; README says `Current version: **0.8.0**` | DF-10 (dup DF-02), DF-11 |
| 3 | README Quick Example | 0 | prints `Hello from Paladin!` | no stumble |
| 4 | `basic_paladin` from the published source | 101, then 0 | needs `paladin-ports`, which the README never asks for | DF-12 |
| 5 | Quickstart with no key | 1 | tag block pins `0.10.0` (resolves 0.10.1); with `0.11.0`: `Error: "OPENAI_API_KEY environment variable not set"` (names the variable) | DF-13 (dup DF-02) |
| 5b | lead L1, installation block | 101, then 0 | `paladin-llm` has no feature `llm-openai`; with `openai` it resolves | DF-14 |
| 6 | delete scratch build | 0 | 4327 MB to 5476 MB free | no stumble |
| 7 | lead L2, `cargo install` | 101, then 0 | without `--features cli`: requires the features `cli`; with `--features cli --locked`: installed in 5m 12s | DF-15 |
| 8 | `paladin-cli --version` / `--help` | 0 | `paladin 0.11.0`; 13 subcommands | (cause of DF-17) |
| 9 | `setup-check` keyless (default, `--verbose`, `--quiet`) | 2 | section order differs run to run; bare `Service:` / `LLM Provider:` headings; exit 2 matches the documented warning exit | DF-16 |
| 10 | `onboarding` with no TTY | 2 | `Error: User input error: IO error: not a terminal`; writes nothing, does not hang | no stumble |
| 11 | docs' `paladin onboarding` / `paladin setup-check` | 127 | `paladin: command not found`; `paladin-cli` is on PATH | DF-17 |

## Findings logged

| id | title (short) | severity | disposition |
|----|---------------|----------|-------------|
| DF-10 | README snippet resolves `paladin-ai 0.5.1` | S3 | duplicate of DF-02 |
| DF-11 | README `default features` comment lists one of three | S4 | open -> in v0.11.1 (contained docs fix: plan 56-07) |
| DF-12 | `basic_paladin` not runnable from a consumer crate, no crates.io route documented | S4 | open -> in v0.11.1 (contained docs fix: plan 56-07) |
| DF-13 | Quickstart and Installation at the tag pin `0.10.0` | S3 | duplicate of DF-02 |
| DF-14 | `llm-openai` is not a `paladin-llm` feature (lead L1) | S3 | open -> in v0.11.1 (contained docs fix: plan 56-07) |
| DF-15 | no working `cargo install` line documented (lead L2) | S3 | open -> in v0.11.1 (contained docs fix: plan 56-07) |
| DF-16 | `setup-check` unstable section order and bare headings | S3 | out (re-homed: .planning/todos/pending/2026-11-02-setup-check-first-run-output.md) |
| DF-17 | docs and CLI messages name `paladin`, binary is `paladin-cli` | S3 | open -> in v0.11.1 (contained docs fix: plan 56-07) |

Assumption A1 (`cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli` succeeds) is **proven** (exit 0), so plan 56-07 may write the `--version`-less form. Assumption A4 is replaced by the recorded behaviour: `onboarding` without a TTY fails at once with a clear error and writes nothing; `setup-check` without keys exits 2 (not 0), with `Skip` rows shown only under `--verbose`.

## Deviations from Plan

### Auto-fixed Issues

None.

### Notes

- **Tag Quickstart pins `0.10.0`.** The orchestrator note and the plan's interface block said the tag Quickstart pins `0.11.0`. It pins `0.10.0` (the keyed runbook's K2 had already warned). The walk resolved the block as written, recorded the result (DF-13), then used `0.11.0` as the plan's `cargo add` commands intend.
- **`setup-check` exit code** is 2 on a clean keyless run, not 0 as research assumption A4 guessed. This matches the documented exit-code table, so it is not a finding.
- **Workspace disk outputs.** `target/debug/examples` and `target/debug/incremental` were already empty (4 KB each), so there was nothing to free before the first build.
- **Unrun by design:** the plan said nothing about the `paladin` binary installed beside `paladin-cli`; DF-17 states from `src/main.rs` (not from a run) that it takes only `--config`.

## Disk ledger

| point | free on `/` |
|-------|-------------|
| start | 5513 MB |
| after step-3 build | 4338 MB |
| after steps 4-5 | 4327 MB |
| after deleting `$WALK/target` | 5476 MB |
| install build finished | 4316 MB |
| after deleting `install-target` | 5455 MB |
| final, scratch root removed | 5476 MB |

No `No space left on device`; no step is 'not run: disk'. No workspace `cargo build` or `cargo test` was run.

## Stage C inputs

None from this plan. No stumble was graded S1 or S2, so none is `open -> promote in v0.11.1`. The open entries from this plan (DF-11, DF-12, DF-14, DF-15, DF-17) are contained docs fixes for plan 56-07. The Stage C inputs remain DF-07 and DF-08 from plan 56-05.

## Known Stubs

None.

## Threat Flags

None. The walk added no network endpoint, auth path or schema; no provider key was set, used or recorded.

## Self-Check: PASSED

- `56-FIRST-RUN-WALK.md` exists, with `## Provenance`, `## Published README vs HEAD`, `## Step table` (12 rows), `## Keyless observations`, `## Disk ledger`.
- `56-RUNBOOK-KEYED-WALK.md` `## Verified by the keyless half` is filled; the K-steps are unchanged (`git diff --numstat`: additions only).
- Commits `4c4d16a60` and `a813bda35` exist; neither deleted a file.
- The findings log gained DF-10..DF-17 with 0 deleted lines; `scripts/check-findings-log.sh` parses 17 entries and is red on DF-07, DF-08, DF-11, DF-12, DF-14, DF-15 and DF-17 as intended.
- The scratch root no longer exists; secret scan of the three tracked files found only the runbook's own redaction templates.
