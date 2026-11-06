---
phase: 56-dogfood-findings
plan: 08
subsystem: dogfood-findings
tags: [dogfood, keyed-walk, secret-scan, findings-log, paladin-cli, onboarding, setup-check, quickstart, dogf-04, df-18, df-19]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-03 keyed-walk runbook, 56-06 keyless walk evidence, 56-07 contained docs fixes (DF-15 install line)"
provides:
  - "56-FIRST-RUN-WALK.md ## Keyed half (maintainer): nine K-rows and the redacted K1..K9 output"
  - "56-RUNBOOK-KEYED-WALK.md slots K1..K9 filled with the maintainer's redacted output"
  - "DF-18 (S3, out, re-homed to a pending todo) and DF-19 (S2, open, promoted for Stage C) in the findings log"
  - ".planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md"
affects: [Stage C, 56-10, SHIP-10]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-08-SUMMARY.md
    - .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md
  modified:
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-KEYED-WALK.md
    - .planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "DF-18 (S3) is dispositioned out (re-homed), not promoted: the root cause is 109 sites in 54 files plus a missing gate, which is not a contained fix under D-05 (rule applied: D-05 S3/S4 clause; PD-08-2 allows only the four named docs files). The Quickstart line alone was left unfixed on purpose so the log does not close on one of 109 sites"
  - "DF-19 (S2) is open -> promote in v0.11.1 for Stage C (D-05 S1/S2 clause, D-06); no code was changed in this plan"
  - "No docs file and no CHANGELOG line changed: L2 (DF-15) was already closed by 56-07, so K4 exit 0 is recorded as a second proof, not as a new fix"
  - "DF-15, DF-16 and DF-17 are not reopened and got no extra note bullet; the keyed run reproduced all three and the walk table cites them"

requirements-completed: [DOGF-04]

duration: ~45min (this continuation); Task 1 ran 2026-11-02
completed: 2026-11-03
---

# Phase 56 Plan 08: Keyed first-run walk Summary

**The maintainer walked the published `0.11.0` first-run path with a real OpenAI key (and an Anthropic key for K8); the redacted output is on the record with no credential in it, and the walk produced two new findings: DF-18 (the Quickstart's `.model("gpt-4")` is deprecated, 109 sites, S3, re-homed) and DF-19 (`setup-check` and `onboarding` fail a valid Anthropic key because the probe hard-codes `claude-3-haiku-20240307`, S2, promoted for Stage C).**

## Performance

- **Tasks:** 3 of 3 (1 tracer, 1 checkpoint:human-action, 1 auto)
- **Commits:** `d6f5f894c` (Task 1), `77b636f2e` (Task 3). Task 2 is the checkpoint and has no commit of its own. The orchestrator's `dc190dba6` touched STATE.md only.
- **No cargo build was needed.** No source, manifest, workflow, doc page or CHANGELOG line changed.

## Checkpoint reply

- **Reply text:** `keyed walk pasted`
- **Date:** 2026-11-03 (the continuation was spawned at 2026-11-03T18:22Z; the maintainer's walk ran about 12:52-13:03 local that day)
- **Channel:** relayed through chat. The maintainer did not edit the runbook file; the orchestrator placed the verbatim, already-redacted text in a scratch file and the agent copied each `## Kn` section into the matching `### Kn` slot.
- **K1:** command block not captured (derived values only: rustc 1.97.1 from the setup-check row, Linux, the run date).
- **K8:** run (an Anthropic key was supplied).
- **Where it ran:** `onboarding` and `setup-check` ran from `$WALK/my-paladin-agent`, not `$WALK`. Operator slip, not a finding.
- No credential appears in the reply, in this SUMMARY, or in any committed file.

## Secret scan (P2 gate, T-56-29)

PD-08-1 scanner: python3 stdlib `re`, scratch only, never committed; prints `file:line: slot`, never the matched text.

| run | files | hits |
|-----|-------|------|
| Task 1, planted copy | runbook copy with 20 `x` after `sk-` in K2 and `sk-abcdREDACTED` in K6 | 1 (K2, line 266); the `REDACTED` line was not flagged |
| Task 1, real runbook (slots empty) | `56-RUNBOOK-KEYED-WALK.md` | 0 |
| Task 3 step 1, before reading or copying anything | the relayed slot file | 0 |
| Task 3 step 1 | runbook, walk file, findings log | 0 |
| Task 3 step 6, over every file staged | runbook (filled), walk file, findings log, new todo | 0 |

A second, looser `grep` for `sk-` + 8 characters, `sk-ant` and `API_KEY=<value>` over the same files matched only the runbook's own redaction instructions and two pre-existing `<fake-key>` placeholders in DF-07/DF-08. One privacy edit beyond the scan: the local account name in the K6 directory listing was replaced with `<user>`.

## Slot reader

Before the paste, all nine slots read `empty`. After the copy, all nine read `filled` (K1 as "not captured").

## Findings

| DF | Severity | Surface | Disposition | Root cause (verified on HEAD dc190dba6) |
|----|----------|---------|-------------|------------------------------------------|
| DF-18 | S3 | docs | `out (re-homed: .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md)` | `quickstart.md:53` `.model("gpt-4")`, same at tag `v0.11.0`. `grep -rn '"gpt-4"' docs/src examples README.md` gives **109 sites in 54 files** (40 in 18 `docs/src` files, 69 in 36 `examples/` files, 0 in README). The relay said "~109 across 10 pages"; the site count holds, the file count was wrong. The todo carries the count, the registry-preferred models (`gpt-4o-mini`, `gpt-5.4-nano`) and a `check-doc-models` gate suggestion |
| DF-19 | S2 | cli | `open -> promote in v0.11.1 (S2, D-05; fix in Stage C, D-06)` | `setup_check.rs:101` and `onboarding.rs:99` post `"model": "claude-3-haiku-20240307"`, the only `"model"` literals in either file, identical at tag `v0.11.0`. The CLI discards the 404 body, so the retired model id is the inferred cause, not one read from the response. The registry knows `claude-haiku-5-5` and `claude-opus-4-8`. Resolves research lead L8 and assumption A6 |

Already final, reproduced by the keyed run, not reopened and not given a note bullet: DF-15 (bare `cargo install` built for 4m12s and installed nothing), DF-16 (bare `Service:` heading, section order differs between the default and `--verbose` runs), DF-17 (`--version` prints `paladin 0.11.0`; Next Steps boxes say `paladin ...`).

Not findings (evidence only, in the walk file): K3 `none` (the Quickstart installs no tracing subscriber, so it proves nothing); `onboarding` run from a subdirectory; `openssl-sys` needed OpenSSL headers on the host (not reproduced as a failure); K1 not captured. No `unprofiled_model` or `usage_unreconciled` was observed, and a pro-tier OpenAI 404 did not occur.

**Rule applied to DF-18:** CONTEXT D-05 (S3/S4 is `in v0.11.1` only when the fix is contained) and PD-08-2 (only docs fixes in the plan's four docs files ride this plan). The Quickstart file is one of the four, but fixing its single line would leave 108 sites and the missing gate, so the whole root cause was re-homed with a filed todo instead.

## L2 outcome

K4's runbook line exited 0 (8m 04s, `Installed package paladin-ai v0.11.0 (executable paladin-cli)`). L2 is DF-15, which plan 56-07 had already fixed and closed, so the plan-step-4 condition ("L2's entry still ends `open`") was false: no docs file and no CHANGELOG line changed, and `make check-doc-config` was not needed.

## Verification

```
$ make check-version-strings
OK: 128 file(s) scanned, every copyable version string equals '0.11.0' (class 1 Cargo pins: 42; class 2 Current version lines: 1; class 3 k8s labels and image tags: 8; class 4 openapi.json info.version: 1; exempt files: 1)
exit 0

$ ./scripts/check-findings-log.sh      (exit 1, as expected: Stage C is not empty)
Findings-log check failed (BAD)
.project/v0.11.1/02-dogfood-findings-log.md:173: DF-07: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:204: DF-08: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:476: DF-19: OPEN (the last Disposition bullet reads 'open')
parsed 19 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

`git diff --numstat` on the findings log: 57 additions, 0 deletions. DF-07 and DF-08 are untouched. The walk file's Keyed half section has nine K-rows. The post-commit deletion check found no deleted files.

## Deviations from Plan

**1. [Process] The first commit carried the wrong co-author trailer.** The task text specified `Claude Fable 5.1`; the harness attribution for this session is `Claude Sonnet 5.5`. The commit was unpushed, so it was amended before anything else used it (`87b17d900` became `77b636f2e`, message otherwise identical).

**2. [Interpretation] Slot content adapted to the fenced layout.** The relay used nested code fences, which cannot sit inside the runbook's single fenced slots, so inner fence lines were dropped and the text kept in order. The orchestrator's own HEAD-check paragraph under K8 was left out of the slot (it is maintainer output only) and appears in DF-19 after being re-verified. No other change to the maintainer's text beyond `<user>`.

Otherwise: plan executed as written.

## Auth gates

Task 2 was the planned `checkpoint:human-action` (a real key and a TTY are needed). It is the plan's normal flow, not a deviation.

## Known Stubs

None. No code or UI was written.

## Threat Flags

None. No file touched introduces new network, auth or file-access surface. T-56-29 (a pasted key reaching the tree) was mitigated by the scan above; T-56-30 by the `cleaned` line in K9; T-56-31 by every K-row naming a DF or `no stumble`.

## Stage C inputs

Every `open -> promote` id in the log after this plan:

- **DF-07** (S1): `paladin` binary `Debug`-formats the whole `Settings`, printing `http.auth.api_keys[].key`.
- **DF-08** (S2): `paladin-server` never calls `Settings::validate()`.
- **DF-19** (S2, new): replace the hard-coded `claude-3-haiku-20240307` in `src/application/cli/commands/setup_check.rs:101` and `src/application/cli/commands/onboarding.rs:99` with a model the registry knows (`claude-haiku-5-5`), and make the failure say which model was probed and what the provider answered (body or error type). A shared constant for both call sites would keep them from drifting; check the `#[ignore]` live tests and any pinned output before changing printed text.

DF-18 is not a Stage C input: it is out (re-homed). It can be picked up from its todo at any time.

## Self-Check: PASSED

- FOUND: `.planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md` (`## Keyed half (maintainer)`, 9 K-rows)
- FOUND: `.planning/phases/56-dogfood-findings/56-RUNBOOK-KEYED-WALK.md` (9 slots filled)
- FOUND: `.planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md`
- FOUND: DF-18 and DF-19 in `.project/v0.11.1/02-dogfood-findings-log.md`
- FOUND: commits `d6f5f894c` and `77b636f2e`
