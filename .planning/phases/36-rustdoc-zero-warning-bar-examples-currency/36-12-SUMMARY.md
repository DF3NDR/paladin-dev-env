---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 12
subsystem: infra
tags: [rustdoc, adr-0033, ci, pre-commit, make, examples, gate]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-02-SUMMARY.md through 36-08-SUMMARY.md (the 143 RD-nn rustdoc link/HTML
      fixes across all ten library crates plus the facade, which made the workspace
      zero-warning bar reachable in the first place)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-09-SUMMARY.md, 36-10-SUMMARY.md (platform_api_client, webhook_receiver,
      node_result_cache, observability_otel_export -- the four gated example targets this
      plan wires into CI's Example Muster and the local check-examples mirror)
provides:
  - scripts/check-all-examples.sh rewritten to run the same 7 feature-split invocations
    (1 default bulk build + 6 explicit gated builds) the CI Example Muster job runs, and
    exposed as `make check-examples`
  - Makefile `doc-check` target -- the single local source of truth for ADR-0033's two
    bar commands plus `cargo test --workspace --doc`, now a `clean-code` dependency
  - `.pre-commit-config.yaml` `doc-check` pre-push hook, same files filter as
    `check-api-surface`
  - `.github/workflows/ci.yml` lint-job step "Check documentation (all features, -D
    warnings)"; Example Muster job now builds all 8 gated example targets (was 4) and its
    explanatory comment/binary-count assertion are corrected to the real 62-file/8-target
    counts
  - closing zero measurement: both ADR-0033 bar commands exit 0, doctests at the 462/0/210
    baseline unchanged, `make api-surface` unchanged (3959 items)
  - deferred-items.md (new phase-local register) recording the orchestrator-requested,
    record-only measurement of what the eight pre-existing crate-level rustdoc
    `#[allow(...)]` suppressions hide (309 content diagnostics), left untouched
affects: [36-13]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A local `make <gate>` target that reproduces a CI-required-check expression
      byte-identically (copied from ci.yml, not paraphrased) is how this repo keeps a
      pre-push gate from drifting apart from the CI gate it mirrors -- same pattern as
      `check-api-surface`, now applied to `doc-check`."
    - "A local examples-check script that mirrors CI's per-feature-split build steps
      one-for-one, rather than a single all-features sweep, is the only way to catch a
      `required-features` gate a bulk `cargo build --examples` selector silently skips
      (D-14) -- verified live by rewriting scripts/check-all-examples.sh to the CI
      invocation list and confirming the two never disagree."

key-files:
  created:
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/deferred-items.md
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-check-examples.txt
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-closing-measurement.txt
  modified:
    - scripts/check-all-examples.sh
    - Makefile
    - .pre-commit-config.yaml
    - .github/workflows/ci.yml

key-decisions:
  - "The per-crate all-features sweep is deliberately NOT added as a fourth `doc-check`
    step (the Claude's-discretion item CONTEXT left open, resolved here): the workspace
    all-features command already fails on the first red crate, which is sufficient as a
    gate, and the sweep's value is per-crate enumeration during debugging rather than
    gating -- adding it would duplicate work `doc-check` step 2 already does at workspace
    granularity."
  - "webhook_receiver was folded into the existing `web-server` CI/script step (alongside
    http_service_host) rather than given its own step, since both declare the identical
    required-features list `[\"web-server\"]` -- the acceptance criterion counts DISTINCT
    required-features lists (6), not gated targets (8), and this keeps the CI job and the
    script's invocation counts equal without an unnecessary extra step."
  - "The orchestrator's record-only pre-existing-allows measurement (309 hidden content
    diagnostics across 5 crates) is recorded in a new deferred-items.md, not in
    WINDOWS.md, even though it was briefly appended there via `gsd-tools windows append`
    and then reverted -- this plan's own project-execution-notes explicitly prohibit
    modifying `.planning/WINDOWS.md` in this plan (rows 36/37 are reserved for plan
    36-13's flip), and that prohibition was read as covering new rows too, not only the
    two named ones."

requirements-completed: [CURR-11, CURR-12, CURR-13, CURR-15]

coverage:
  - id: D1
    description: "scripts/check-all-examples.sh rewritten to run the same 7 CI Example Muster invocations (no cargo check --example, no --all-features) plus the binary-count assertion; make check-examples target added, not wired into clean-code or the pre-push hook"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "make check-examples (exit 0, 62/62 binaries); make lint-shell (exit 0); grep -c 'cargo check --example' scripts/check-all-examples.sh = 0; grep -c -- '--all-features' scripts/check-all-examples.sh = 0; grep -c 'clean-code: .*check-examples' Makefile = 0; git diff --stat HEAD~1 -- src/ crates/ examples/ empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "make doc-check wired as the single local source of truth for both ADR-0033 bar commands plus cargo test --workspace --doc, in order; clean-code now depends on doc-check; test-doc and doc (--open) kept unchanged"
    requirement: "CURR-11"
    verification:
      - kind: other
        ref: "make doc-check (exit 0, three labelled steps); grep -qE '^clean-code: .*doc-check' Makefile; grep -c '^test-doc:' Makefile = 1; grep -c '^doc:' Makefile = 1"
        status: pass
    human_judgment: false
  - id: D3
    description: ".pre-commit-config.yaml gained a doc-check pre-push hook (same files filter as check-api-surface); ci.yml lint job gained 'Check documentation (all features, -D warnings)' immediately after the existing documentation step, with the workspace doctest step NOT duplicated"
    requirement: "CURR-12"
    verification:
      - kind: other
        ref: "grep -q 'id: doc-check' .pre-commit-config.yaml; grep -q 'Check documentation (all features, -D warnings)' .github/workflows/ci.yml; grep -c 'cargo test --workspace --doc' .github/workflows/ci.yml = 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "CI Example Muster job now builds all 8 gated example targets (platform_api_client, node_result_cache, observability_otel_export added; webhook_receiver folded into the existing web-server step) and its explanatory comment / binary-count assertion are corrected to the real 62-file / 8-target / 54-auto-discovered counts, matching scripts/check-all-examples.sh's invocation list one-for-one"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "grep -c 'run: cargo build --example' .github/workflows/ci.yml = 7 (matches script); find examples -name '*.rs' | wc -l = 62 matches the comment; grep -c '^\\[\\[example\\]\\]' Cargo.toml = 8 matches the comment"
        status: pass
    human_judgment: false
  - id: D5
    description: "Closing zero measurement: both ADR-0033 bar commands exit 0 on the gate commit, cargo test --workspace --doc holds at the 462 passed / 0 failed / 210 ignored baseline exactly, make check-examples and make api-surface (3959 items) are unchanged, and the gate-wiring edits plus this measurement land in one commit so git bisect never lands on a commit where the gate exists but fails (D-13)"
    requirement: "CURR-11"
    verification:
      - kind: other
        ref: "cargo doc --workspace --no-deps | tee ... && ! grep -q warning: (exit 0, 0 lines); RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --all-features --no-deps (exit 0); cargo test --workspace --doc (462/0/210); ./scripts/check-api-surface.sh (3959 items unchanged); git show --stat HEAD lists only Makefile, .pre-commit-config.yaml, .github/workflows/ci.yml and the closing-measurement evidence file, no .rs file"
        status: pass
    human_judgment: false

# Metrics
duration: ~1h
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 12: Wire the Rustdoc Zero-Warning Gate into make, pre-push and CI Summary

**Both ADR-0033 rustdoc bars and `cargo test --workspace --doc` are now enforced by `make clean-code`, a pre-push hook, and a new CI lint-job step; every one of the 8 gated example targets (4 added by plans 36-09/36-10) builds in both CI's Example Muster job and the rewritten `scripts/check-all-examples.sh`, and the closing measurement confirms zero warnings, baseline doctests, and no API-surface drift -- all in the gate-wiring commit itself.**

## Performance

- **Duration:** ~1h
- **Started:** 2026-09-17T23:19:30Z
- **Completed:** 2026-09-17T23:39:49Z
- **Tasks:** 2
- **Files modified:** 8 (2 rewritten/wired existing files across the two tasks that are also
  touched a second time -- Makefile touched in both tasks; net unique files: 7)

## Accomplishments

- Rewrote `scripts/check-all-examples.sh` from a per-file `cargo check --example <name>
  --all-features` loop (which silently satisfied every `required-features` gate) to the same
  7 invocations `.github/workflows/ci.yml`'s Example Muster job runs -- 1 default-features bulk
  build covering all 54 auto-discovered targets, plus 6 explicit `cargo build --example ...
  --features "..."` invocations, one per distinct required-features list across the 8 gated
  `[[example]]` targets -- followed by the binary-count assertion. Exposed as `make
  check-examples`, deliberately not wired into `clean-code` or the pre-push hook (several full
  example builds are too slow for a push hook, D-14). Confirmed 62/62 binaries present, exit 0.
- Added `make doc-check`: the single local source of truth running, in order, (1) `ci.yml:63`'s
  exact default-features bar expression, (2) `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
  --all-features --no-deps`, (3) `cargo test --workspace --doc`. `clean-code` now depends on
  `doc-check`; `test-doc` and `doc --open` are unchanged (D-11). The per-crate all-features sweep
  was deliberately not added as a fourth step -- the workspace command already fails on the first
  red crate, and per-crate enumeration is a debugging aid, not a gating requirement.
- Added a `doc-check` pre-push hook to `.pre-commit-config.yaml`, same `files:` filter and
  `stages: [pre-push]` shape as the existing `check-api-surface` hook, with a comment explaining
  the placement (cost, and GSD executor commits bypass the commit stage entirely).
- Added a CI lint-job step "Check documentation (all features, -D warnings)" immediately after
  the existing documentation step, running the all-features bar command. The workspace doctest
  step (already present in the `test` job) was not duplicated.
- Extended the CI Example Muster job to build all 8 gated example targets: `webhook_receiver`
  folded into the existing `web-server` step (identical required-features list to
  `http_service_host`), and three new steps for `platform_api_client` (`web-server,dev-ui`),
  `node_result_cache` (`redis-cache`), and `observability_otel_export` (`otel`). Corrected the
  explanatory comment and the binary-count assertion step's name from the stale 47-file/4-target/
  43-auto-discovered figures to the real 62-file/8-target/54-auto-discovered figures, recomputed
  from the tree rather than trusted from any prior document.
- Captured the closing zero measurement in `36-evidence/36-12-closing-measurement.txt`: both bar
  commands exit 0 individually and inside `make doc-check`; `cargo test --workspace --doc` holds
  exactly at the 462 passed / 0 failed / 210 ignored baseline; `make check-examples` and `make
  api-surface` (3959 items) are unchanged; `cargo fmt --all -- --check` and `cargo check
  --workspace --all-targets --all-features` are both green. No residual diagnostic was found, so
  no new `RD-nn`/`EX-nn` row was minted.
- Per the orchestrator's record-only evidence request, measured (without fixing) what the eight
  pre-existing crate-level `#![allow(rustdoc::...)]` attributes hide: with them temporarily
  disabled via `//`-prefixed comments (restored immediately after capture, confirmed clean via
  `git status --porcelain -- src crates`), `cargo doc --workspace --no-deps` produced 309 content
  diagnostics across 5 crates (`paladin-ports` 119, facade `paladin-ai` 108, `paladin-llm` 69,
  `paladin-storage` 10, `paladin-notifications` 3 -- 314 total `warning:` lines including the 5
  per-crate summaries). Recorded in the new `deferred-items.md` with a proposed classification
  and recommended owner (Phase 36.1 or a standalone quick task); none of the eight attributes were
  touched in any committed state.

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite scripts/check-all-examples.sh to mirror the CI Example Muster, and expose it as make check-examples** - `fed7b72e` (docs)
2. **Task 2: The single gate-wiring commit — make doc-check, clean-code, the pre-push hook, the CI lint step, the CI example coverage, and the closing zero measurement** - `09bc7d96` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified

- `scripts/check-all-examples.sh` - rewritten to the 7 CI-mirroring invocations + binary-count assertion
- `Makefile` - new `check-examples` target (Task 1); new `doc-check` target and `clean-code` dependency (Task 2)
- `.pre-commit-config.yaml` - new `doc-check` pre-push hook
- `.github/workflows/ci.yml` - new lint-job all-features documentation step; 4 new/updated Example Muster build steps; corrected comment and assertion step name
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-check-examples.txt` - Task 1's full `make check-examples` run + acceptance-criteria greps
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-closing-measurement.txt` - the full closing measurement (both bars, doctests, api-surface, fmt/check, toolchain/SHA, and the pre-existing-allows probe)
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/deferred-items.md` - new phase register recording the pre-existing-allows finding

## Decisions Made

- The per-crate all-features sweep stays out of `doc-check` (Claude's-discretion item resolved: workspace-level all-features already fails on the first red crate; per-crate enumeration is a debugging convenience, not a gating need).
- `webhook_receiver` shares the `web-server` CI/script step with `http_service_host` rather than getting its own step, since the acceptance criterion counts distinct required-features lists (6), not gated targets (8).
- The pre-existing-allows measurement is recorded in `deferred-items.md`, not `WINDOWS.md` — this plan's own instructions reserve `WINDOWS.md` for plan 36-13's row 36/37 flip, so a `gsd-tools windows append` entry was created and then reverted rather than left in place.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Comment text in check-all-examples.sh and ci.yml accidentally matched the plan's own literal grep acceptance checks**
- **Found during:** Task 1 and Task 2's own `<verify>` acceptance-criteria greps
- **Issue:** Explanatory prose in the rewritten script's header comment used the literal substrings `cargo check --example` and `--all-features` to describe the *old*, replaced behavior; similarly, a CI comment used the literal substring `cargo test --workspace --doc` to explain the doctest step is not duplicated. Both accidentally satisfied the acceptance criterion's own grep pattern in the wrong direction (the greps expect 0/1 occurrences of the real invocation, not comment prose that happens to contain the same words).
- **Fix:** Reworded both comments to describe the same facts without using the literal flag/command substrings verbatim (e.g. "enabling every workspace feature at once" instead of `--all-features`; "the workspace doctest run already executes" instead of `cargo test --workspace --doc`).
- **Files modified:** `scripts/check-all-examples.sh`, `.github/workflows/ci.yml`
- **Verification:** Re-ran the exact acceptance-criteria greps after the reword; all returned the expected counts (0, 0, 1).
- **Committed in:** `fed7b72e` (script comment) and `09bc7d96` (ci.yml comment)

---

**Total deviations:** 1 auto-fixed (1 bug/self-inflicted-grep-collision)
**Impact on plan:** Cosmetic-only; no behavior change. No scope creep.

## Issues Encountered

- An initial attempt to measure the pre-existing rustdoc allow suppressions used `sed` to prefix the attribute lines with `# DISABLED-36-12:`, which produces invalid Rust (`#` is not a Rust line-comment character) rather than a disabled attribute. Caught before running `cargo doc` against it by inspecting the sed output; reverted via `git checkout --` and redone with a valid `//`-prefixed Rust line comment, which correctly disables the attribute while remaining syntactically valid.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both ADR-0033 bars and the doctest run are now enforced locally (`make clean-code`, pre-push)
  and in CI (lint job), so the warning count this phase spent eight plans clearing cannot silently
  regrow.
- Plan 36-13 (phase close) still needs to: push the branch and capture a real CI run proving the
  new lint-job step and the expanded Example Muster job green on GitHub Actions (D-12's own
  requirement, this plan could not satisfy locally); flip WINDOWS.md rows 36 and 37 to `fixed`
  via `gsd-tools`; and decide whether/when to act on this plan's `deferred-items.md` entry (the
  309 hidden diagnostics behind the eight pre-existing rustdoc allows) — recommended as Phase
  36.1 or a standalone quick task, not blocking this phase's own close.

## Self-Check: PASSED

- FOUND: `scripts/check-all-examples.sh`
- FOUND: `Makefile`
- FOUND: `.pre-commit-config.yaml`
- FOUND: `.github/workflows/ci.yml`
- FOUND: `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-check-examples.txt`
- FOUND: `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-closing-measurement.txt`
- FOUND: `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/deferred-items.md`
- FOUND commit: `fed7b72e` (Task 1)
- FOUND commit: `09bc7d96` (Task 2)

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*
