# Phase 56: Dogfood Findings - Discussion Log

> **Audit trail only.** Do not use this file as input to planning, research or execution agents.
> Decisions are in CONTEXT.md. This log keeps the alternatives that were considered.

**Date:** 2026-11-02
**Phase:** 56-dogfood-findings
**Mode:** `--auto`. Every gray area was auto-selected and every question resolved to the
recommended option, with no interactive prompts.
**Areas discussed:** Findings-log mechanics, Triage policy, Config round-trip test design,
First-run walk, web3sec return, Local coverage walk

**Auto-mode actions logged:**
- `[auto] Context does not exist — creating.` No checkpoint was found and no plans exist.
- `[auto] Todo fold (score ≥ 0.4):`
  - folded `2026-08-13-verify-local-coverage-reproduction` (0.9)
  - folded `2026-10-14-web3sec-single-bump-to-v0-11-0` (0.6)
  - `2026-10-20-openai-responses-api-for-pro-tier` (0.6) was **overridden to not-folded**,
    because it conflicts with the locked milestone decision "OpenAI Responses API out"
    (2026-10-27)
- `[--auto] Selected all gray areas:` Findings-log mechanics, Triage policy, Config round-trip test
  design, First-run walk, web3sec return, Local coverage walk.

---

## Findings-log mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| Append a dated disposition note; the last `Disposition` bullet wins | Honours the log's "append; never rewrite" rule | ✓ |
| Edit the `Disposition:` line in place | Simpler to read, but rewrites earlier entries | |

| Option | Description | Selected |
|--------|-------------|----------|
| DF-01…04 → `in v0.11.1`, pointer to the owning requirement and its proving test | Owners are done in Phases 53-55; re-checked on HEAD first | ✓ |
| DF-01…04 → `duplicate of` | No other DF entry exists to duplicate | |
| Require a fresh live repro for each | Redundant with the owners' failing-first tests | |

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone checker script and make target, run at verification and SHIP-10, not in `check-gates` | Mid-phase `open` entries are legitimate | ✓ |
| CI gate in `make check-gates` | Would turn every new finding into a red build | |
| Manual read | Not provable | |

**Choice:** all recommended. → D-01, D-02, D-03

---

## Triage policy

| Option | Description | Selected |
|--------|-------------|----------|
| S3/S4: contained fixes in, the rest `out` with a filed todo | Keeps the milestone bounded and drops nothing | ✓ |
| Fix every S3/S4 | Unbounded scope | |
| Defer every S3/S4 | Leaves cheap doc fixes on the table | |

| Option | Description | Selected |
|--------|-------------|----------|
| S1/S2 fixes as inline plans by default; decimal `56.x` only when a fix needs its own design call | Matches the roadmap scope note | ✓ |
| A decimal phase for every fix | Heavy ceremony for small fixes | |
| All inline, always | Hides design calls inside fix plans | |

| Option | Description | Selected |
|--------|-------------|----------|
| Three stages: agent-runnable, human-gated, fixes from the log | Stage A never waits on the maintainer | ✓ |
| Plan everything up front | Content is unknown today | |

**Choice:** all recommended. → D-04 … D-07

---

## Config round-trip test design

| Option | Description | Selected |
|--------|-------------|----------|
| Production loader(s) (`Settings::load_from_file` plus every other prod path) | The bug class lives in the config crate's handling | ✓ |
| Raw `serde_yaml` | Misses the loader quirks | |

| Option | Description | Selected |
|--------|-------------|----------|
| Committed full fixture plus a drift guard against the example | Keeps the example's fail-closed shape | ✓ |
| Uncomment the keys in `config.example.yml` | Changes what a verbatim copy does | |
| Text-munge the comments in-test | Fragile | |

| Option | Description | Selected |
|--------|-------------|----------|
| Re-serialise and diff key paths; `KNOWN_GAPS` rows cite DF ids (`serde_ignored` allowed as a substitute) | No new dependency by default; self-maintaining | ✓ |
| Rely on load errors only | Unknown keys are silently dropped everywhere except treasurer | |

**Choice:** all recommended. → D-08 … D-12

---

## First-run walk (DOGF-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Agent runs the keyless half; the maintainer runs the keyed half from a runbook, on the README's provider | The key never enters the repo; matches the docs verbatim | ✓ |
| Maintainer runs the whole walk | Wastes an agent-runnable half | |
| Agent runs with a key | No key is available, and that would be unsafe | |

**Choice:** recommended. → D-13, D-14

---

## web3sec return (DOGF-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Never block Stages A and C; maintainer decision checkpoint at close (hold the tag or re-home with owner and date) | Matches SHIP-10's admission rule | ✓ |
| Block the phase until the return arrives | Stalls agent-runnable work | |
| Agent re-homes it by default | It is the maintainer's call | |

**Choice:** recommended. → D-15, D-16

---

## Local coverage walk (DOGF-05)

| Option | Description | Selected |
|--------|-------------|----------|
| Whole-percent agreement with the CI `coverage` job on the same commit, both ≥ 82 | The todo's own bar | ✓ |
| An exact match to two decimals | Run-to-run noise | |

**Choice:** recommended. → D-17

---

## Claude's Discretion

- Names of the script, make target, fixture and evidence files.
- How the key-path walker is written.
- The test layout: per-section or table-driven, and whether it lives in `src/config` or `tests/`.
- Runbook layout.
- How Stage C groups findings into plans.

## Deferred Ideas

- An `APP_…` env-overlay round-trip.
- `deny_unknown_fields` across every config section. This is a milestone-level behavioural call.
- Ollama / OpenAI-compatible live probes and walks on non-README providers.
- OpenAI Responses API todo (reviewed, not folded).
