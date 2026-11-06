---
phase: 56-dogfood-findings
plan: 07
subsystem: dogfood-findings
tags: [docs, contained-fixes, installation, paladin-cli, quickstart, changelog, dogf-04, dogf-01, df-11, df-12, df-14, df-15, df-17]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-06 walk evidence (56-FIRST-RUN-WALK.md) and DF-10..DF-17 logged with 'contained docs fix: plan 56-07'"
provides:
  - "installation.md names paladin-llm feature openai; the block resolves on crates.io 0.11.0"
  - "cargo install paladin-ai --features cli --locked --bin paladin-cli in installation.md and cli-onboarding.md"
  - "README and crate-map default-features comment lists all three defaults"
  - "six CLI appendix pages invoke paladin-cli onboarding / setup-check"
  - "Quickstart note on running basic_paladin from the published package"
  - "56-FIRST-RUN-WALK.md ## Contained fixes (5 proof rows); one CHANGELOG [Unreleased] Documentation entry"
affects: [Stage C, SHIP-10, 56-08]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-07-SUMMARY.md
  modified:
    - docs/src/getting-started/installation.md
    - docs/src/getting-started/quickstart.md
    - docs/src/appendix/cli-onboarding.md
    - docs/src/appendix/cli-setup-check.md
    - docs/src/appendix/cli-muster.md
    - docs/src/appendix/cli-council.md
    - docs/src/appendix/cli-configuration.md
    - docs/src/appendix/cli-usage.md
    - README.md
    - docs/src/api-reference/crate-map.md
    - CHANGELOG.md
    - .planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "cli-usage.md (the sixth page DF-17 names, absent from the plan's files_modified) got the same paladin-cli correction, because DF-17 is one root cause and closing it with a page still wrong would be a false close; docs only, no design call"
  - "Only the onboarding and setup-check subcommand lines were renamed, as the plan and DF-17 specify; other subcommand lines on the same pages (paladin muster, paladin agent run, paladin features, paladin --version) are left, see Stage C inputs"
  - "The Quickstart note carries no version pin (paladin-ai-<version> path, cargo add paladin-ports without @version) so check-version-strings has nothing to rot against"
  - "L2 written because 56-06 step 7(b) proved the line with --version 0.11.0 (exit 0, A1 proven); the documented line has no --version"

requirements-completed: [DOGF-04, DOGF-01]

duration: ~25min
completed: 2026-11-02
---

# Phase 56 Plan 07: Contained first-run docs fixes Summary

**Five contained docs findings from the published-crate walk are fixed and closed in the log, each with a recorded proof: the installation guide's `paladin-llm` feature is `openai` (the old block fails `cargo metadata` against crates.io, the new one resolves), the docs carry the proven `cargo install paladin-ai --features cli --locked --bin paladin-cli` line, the CLI appendix names the `paladin-cli` binary, the default-features comments list all three defaults, and the Quickstart tells a crates.io user how to run `basic_paladin`. No source, manifest, workflow, ADR, `MIGRATION.md` or `[0.11.0]` changelog line changed.**

## Performance

- **Plan-start SHA:** `15392e9aa` (HEAD at start; the 56-06 walk was started from `a39a338dd`)
- **Tasks:** 3 of 3 (1 tracer, 2 auto)
- **Commits:** `cb5b5e6b4`, `c8dcc0557`, `302c0c743`

## 56-06 entries targeted

The log entries whose last bullet named `plan 56-07` at plan start: DF-11, DF-12, DF-14, DF-15, DF-17. Candidates from the plan's expected set and what 56-06 logged: L1 = DF-14, L2 = DF-15, CLI binary name = DF-17, S4 default-features = DF-11, `basic_paladin` crates.io note = DF-12. All five were logged; none was skipped as "not logged by 56-06".

## Fixes, proofs and closed ids

| DF | Fix | Proof (row in `56-FIRST-RUN-WALK.md` `## Contained fixes`) |
|----|-----|------|
| DF-14 (L1) | `installation.md`: `features = ["openai"]`. No other `paladin-llm` line in `docs/src` or `README.md` used an `llm-` feature (`quickstart.md:26`, `crate-map.md:230` already `openai`) | scratch crate, block extracted from the page: `cargo metadata` exit 0, six `paladin-*` crates at `0.11.0`; same block with `llm-openai`: exit 101, `depends on paladin-llm with feature llm-openai but paladin-llm does not have that feature` |
| DF-15 (L2) | the `--version`-less install line in `installation.md:141` and `cli-onboarding.md:22`, with the `cli`/`--locked` sentence | 56-06 step 7(b) ran it with `--version 0.11.0`, exit 0 (A1); step 7(a) without `--features cli` exit 101. The `--version`-less form was not re-run here (see Deviations) |
| DF-11 (S4) | `README.md` and `crate-map.md` comment lists `llm-openai`, `llm-anthropic`, `llm-deepseek` | `tomllib` prints `['llm-openai', 'llm-anthropic', 'llm-deepseek']` from the root manifest; both comments match |
| DF-17 | `paladin onboarding` / `paladin setup-check` becomes `paladin-cli ...` on six appendix pages (code blocks, inline, `$ ` transcripts, link text, headings; two `cli-usage.md` anchors follow the headings) | anchored grep: 0 bare-`paladin` invocations, 38 lines naming `paladin-cli`; walk row 11 showed `paladin onboarding` exit 127 |
| DF-12 | Quickstart note: `cargo run --example` needs a checkout; copy the example from the package's `examples/`, add `paladin-ports`, run it | walk row 4: the same steps, exit 101 then 0 |

Closed in the log (one dated `in v0.11.1 — fixed in plan 56-07` bullet each, no existing line touched): DF-11, DF-12, DF-14, DF-15, DF-17. The log diff over the plan is 5 insertions and 0 deletions.

## Checker output

`./scripts/check-findings-log.sh` after the last entry (exit 1, expected):

```
.project/v0.11.1/02-dogfood-findings-log.md:173: DF-07: OPEN (the last Disposition bullet reads 'open')
.project/v0.11.1/02-dogfood-findings-log.md:204: DF-08: OPEN (the last Disposition bullet reads 'open')
parsed 17 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

Exactly DF-07 and DF-08, the Stage C entries, are open. No 56-07 entry waits on K4 and none was re-homed in this plan.

Other gates: `make check-version-strings` exit 0 after every task (128 files, all `0.11.0`); `make check-doc-config` exit 0 (161 YAML blocks, 0 failed); `make check-changelogs` exit 0. `git diff --quiet HEAD -- MIGRATION.md` clean; the CHANGELOG diff has 0 deleted lines and touches only `[Unreleased]`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] `cli-usage.md` also corrected**
- **Found during:** Task 3
- **Issue:** DF-17 names six pages including `cli-usage.md`, but the plan's `files_modified` and its anchored verify grep list five. Closing DF-17 with `cli-usage.md` still saying `paladin onboarding` (and its table-of-contents anchors) would leave the reader-facing defect in place.
- **Fix:** same substitution on `cli-usage.md`, plus the two heading anchors (`#paladin-cli-onboarding`, `#paladin-cli-setup-check`). Docs only.
- **Files modified:** `docs/src/appendix/cli-usage.md`
- **Commit:** `302c0c743`

**2. [Rule 3 - Proof scope] The `--version`-less install line was not re-run**
- The plan accepts 56-06's `--version 0.11.0` run as the proof (PD-07-1). A second 5-minute release build against ~5.4 GB free disk was not justified: the two forms differ only in which published `paladin-ai` cargo resolves, and `0.11.0` is the latest published. Recorded in the DF-15 proof row.

**3. [Process] The scratch proof crates for DF-14 were built twice**
- The first block extraction included a second `[dependencies]` header (`duplicate key`, exit 101 for both blocks, before cargo read any dependency). The extraction was corrected and both runs repeated; the recorded exits are from the corrected runs.

Otherwise: plan executed as written. No auth gates.

## Stage C inputs

- **Other CLI subcommand lines still say `paladin`.** DF-17 and the plan cover `onboarding` and `setup-check`. The same appendix pages (`cli-muster.md`, `cli-onboarding.md` next-steps, `cli-setup-check.md:303` `paladin --version`, and others) invoke `paladin muster`, `paladin agent run`, `paladin features` and `paladin --version` on the same wrong binary name. Walk row 8 shows the real clap name prints `paladin 0.11.0` from `paladin-cli`, so the cleanest single fix may be a code change (give the CLI clap name `paladin-cli`, which also fixes the in-binary strings re-homed with DF-16) rather than a docs sweep. That is a decision, not a contained fix. Recommended: decide it together with `.planning/todos/pending/2026-11-02-setup-check-first-run-output.md`.
- DF-07 and DF-08 remain open for Stage C, untouched.
- No other input.

## Known Stubs

None. No code or placeholder data was introduced; every edit is documentation text or planning evidence.

## Threat Flags

None. No network endpoint, auth path, file-access pattern or schema changed. Threat model T-56-26 (install line carries `--locked` and the exact package name `paladin-ai`), T-56-27 (each closing bullet names a `## Contained fixes` row) and T-56-28 (`MIGRATION.md` and `[0.11.0]` untouched, verified) are mitigated as planned.

## Self-Check: PASSED

- Commits `cb5b5e6b4`, `c8dcc0557`, `302c0c743` exist on `HEAD`.
- `## Contained fixes` is in `56-FIRST-RUN-WALK.md` with five rows; `grep -rn 'cargo install paladin-ai --features cli --locked --bin paladin-cli'` finds `installation.md` and `cli-onboarding.md`.
- `git diff --stat 15392e9aa..HEAD` lists only docs, `README.md`, `CHANGELOG.md`, the walk evidence and the log (no `src/`, `crates/`, `Cargo.toml` or `.github/` path).
