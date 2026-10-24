---
phase: 44-legacy-clean-break-removal
plan: 09
subsystem: documentation
tags: [aegis, mdbook, docs, per-attempt-timeout, transience, conclave, clean-break]
status: complete

requires:
  - phase: 44-01
    provides: BattalionConfig.aegis / with_aegis / validate_aegis and the per-pattern contract quoted in the guides
  - phase: 44-03
    provides: PaladinError::LlmFailure and transience(); llm_failure::to_paladin_error
  - phase: 44-06
    provides: BattalionResult.node_errors as structured NodeError (node_id = Paladin name)
  - phase: 44-07
    provides: Commander with no execute() wrapper (300 s per-attempt default); Conclave on engine backoff
  - phase: 44-08
    provides: the compiled formation anchor in crates/doc-examples/src/orchestration.rs
provides:
  - twelve mdBook pages plus docs/DEMOS.md teaching Aegis, per-attempt timeouts, the engine backoff and the typed PaladinError taxonomy
  - new fault-tolerance.md subsection "Aegis on the classic Battalion patterns"
  - both fictional battalion: YAML blocks removed from getting-started/configuration.md
affects: [44-10, 44-11, 44-13]

tech-stack:
  added: []
  patterns:
    - "Every Battalion timeout description states per attempt, no whole-Battalion clock, ChainOfCommand/Grove unbounded, caller wraps Commander::execute() in tokio::time::timeout"
    - "Removed items are described by concept, never by identifier, so the 44-13 source-tree guard stays clean"

key-files:
  created: []
  modified:
    - docs/src/user-guides/orchestration.md
    - docs/src/user-guides/battalion-patterns.md
    - docs/src/user-guides/fault-tolerance.md
    - docs/src/getting-started/configuration.md
    - docs/DEMOS.md
    - docs/src/appendix/conclave-pattern.md
    - docs/src/deployment/production.md
    - docs/src/appendix/battalion-vision-support.md
    - docs/src/appendix/cli-usage.md
    - docs/src/user-guides/paladin-agents.md
    - docs/src/user-guides/paladin-configuration.md
    - docs/src/contributing/architecture-decisions.md
    - docs/src/user-guides/tool-integration.md

key-decisions:
  - "No Aegis YAML schema invented: configuration.md points to programmatic BattalionConfig.aegis and says config.yml has no battalion: section"
  - "Conclave backoff documented as 500 ms doubling, 60 s cap, jitter up to one base delay (engine::retry::backoff_delay), count from retry_attempts, Transient-only"
  - "architecture-decisions.md snippets call to_paladin_error(&self.classify_http_error(e)) because the raw reqwest error is not an LlmError; classify_http_error is stated to be the adapter's own helper"

patterns-established:
  - "orchestration.md Configuration Reference holds the single per-pattern Aegis table; other pages link to it"

requirements-completed: []
requirements-advanced: [LEGACY-02, LEGACY-03, LEGACY-04]  # REQUIREMENTS.md left unchecked; ticked by the removal plans and the guard

duration: ~35min
completed: 2026-10-11
---

# Phase 44 Plan 09: mdBook pages on Aegis and the typed taxonomy Summary

**Thirteen doc files now teach Battalion fault handling through `BattalionConfig.aegis` (per-attempt timeouts with their consequences stated), the engine backoff for the Conclave, and `PaladinError::LlmFailure` / `transience()`; the false "input is passed through" claim and both fictional `battalion:` YAML blocks are gone.**

## Accomplishments

- **Task 1 (commit `3833be60`)**
  - `orchestration.md`: the Formation paragraph now describes fail-fast versus `Absorb` and corrects the old claim (the next Paladin receives an **empty input**). The Configuration Reference was rewritten around a `with_aegis(Aegis { retry, timeout, on_error, .. })` snippet, a per-pattern table (Formation / Phalanx / Campaign / Conclave / ChainOfCommand, Grove, Council), the `max_attempts` = total-attempts note, the per-attempt rule (a ten-step Formation may run ten times the bound; ChainOfCommand and Grove unbounded; wrap `Commander::execute()` in `tokio::time::timeout`; `CommanderBuilder` default 300 s per attempt) and a `node_errors` description. The `{{#include ...orchestration.rs:formation}}` line is kept. The queue paragraph's fictional `battalion.retry` / `battalion.default_timeout_seconds` reference now points to `BattalionConfig.aegis`.
  - `battalion-patterns.md`: error-handling table and snippet are Aegis; Campaign has no continue mode; Conclave retries Transient only with `retry_attempts` as the count.
  - `fault-tolerance.md`: new subsection "Aegis on the classic Battalion patterns" (no code symbols beyond existing types).
  - `configuration.md`: the `## Battalion` section's YAML block and `APP_BATTALION_*` line replaced by a short pointer; the `battalion:` stanza in the Complete Example deleted; other stanzas untouched.
  - `DEMOS.md`: "an `Absorb` error handler that continues past a failed Paladin".
- **Task 2 (commit `d015a4d0`)**
  - `conclave-pattern.md`: every whole-Conclave timeout is on `battalion_config` via `with_aegis`; legacy timeout field removed from the struct listing; the 2^n seconds table replaced by the engine backoff; the removed timeout match arm replaced by an `AggregatorFailed` arm that explains expert timeouts (Transient, retried) versus aggregator timeouts; CLI YAML keys kept with a per-attempt sentence; the Paladin builder `.timeout_seconds(600)` at the aggregator troubleshooting site left alone (it is a Paladin's own timeout).
  - `production.md`: retry snippet rebuilt on `engine::retry::{backoff_delay, wait_backoff}` and the Aegis `RetryPolicy`, with a sentence pointing to `BattalionConfig.aegis.retry`.
  - `battalion-vision-support.md`, `cli-usage.md`: per-attempt wording.
- **Task 3 (commit `b452ffdd`)**: `paladin-agents.md` Transience column and `LlmFailure` row plus the "retry decisions read `transience()`" sentence; `paladin-configuration.md` match arm on `LlmFailure { transience, provider, message, .. }`; `architecture-decisions.md` two erasure snippets via `llm_failure::to_paladin_error`; `tool-integration.md` comment on `transience()` and the closure renamed `is_transient`.

## Verification Results (all actually run)

| Check | Result |
| ----- | ------ |
| `make check-doc-config` | exit 0, 152 YAML blocks checked, 0 failed |
| `make check-doc-examples` | exit 0, 0 checked, 617 skipped, 0 failed ("All doc code examples pass validation") |
| Task 1 / 2 / 3 `git grep` removed-identifier checks (as written in the plan) | print nothing |
| Plan-level `git grep -nw 'ContinueOnError\|RetryThenContinue' -- docs ':!docs/src/api-reference'` | one hit, `docs/assets/recordings/battalion-formation.cast` (see Deferred) |
| Plan-level second `git grep` (retry policy builder, both removed timeout variants, retry module path, delay helper, removed `PaladinError` variant) | prints nothing |
| Broader scan of the 13 files for strategy enum, builders, timeout variants, retry module, `LlmError(`, `is_retryable`, `is_terminal` | only `PaladinBuilder::timeout_seconds` (the Paladin's own, still current) |
| Acceptance greps: `with_aegis`, `per attempt`, `empty input`, `ChainOfCommand`, `Grove`, `tokio::time::timeout` in `orchestration.md`; `BattalionConfig.aegis` in `configuration.md`; `^battalion:` / `retry_then_continue` / `APP_BATTALION` count 0; other stanzas count 8; `AggregatorFailed`, `500 ms`, `retry_attempts` in `conclave-pattern.md`; `2^` count 0; `backoff_delay` in `production.md`; `LlmFailure` and `Transience` in `paladin-agents.md`; `to_paladin_error` twice in `architecture-decisions.md` | all satisfied |

**`mdbook` is not installed in this environment**, so the book was not built and rendered links/anchors were not machine-checked. The two make gates are syntax scans (the `check-doc-examples` run compiled nothing: 617 blocks skipped as `rust,ignore`/fragments). Link targets I added (`fault-tolerance.md`, `orchestration.md#configuration-reference`, `maneuver-flow-dsl.md`) were confirmed to exist by file name and heading.

## Deviations from Plan

**1. [Rule 3 - Blocking, self-inflicted] Wrong slice duplicated `production.md` content.** My first scripted edit of `production.md` located `## Monitoring` at its first occurrence (earlier in the file than the retry section), which duplicated about 400 lines. Caught by `git diff --stat` before committing; reverted the file with `git checkout -- docs/src/deployment/production.md` (single file, my own edit) and redid the edit with the search anchored after the retry heading. The committed result is a 25/16 line diff.

**2. [Rule 2 - Missing accuracy] Extra fictional reference fixed.** `orchestration.md` also named `battalion.retry` / `battalion.default_timeout_seconds` configuration in its queue paragraph (not in the plan's list for that page); reworded to `BattalionConfig.aegis` since it taught the same non-existent config surface.

**3. [Plan reading] `architecture-decisions.md` snippets.** The plan asks for `.map_err(|e| to_paladin_error(&e))?`, but in those snippets `e` is a `reqwest` error, not an `LlmError`. I wrote `to_paladin_error(&self.classify_http_error(e))` with a comment that `classify_http_error` is the adapter's own helper producing the typed `LlmError`. The two `to_paladin_error` occurrences required by the acceptance check are intact; the prose avoids naming it a third time.

No auth gates, no source or Rust changes, no disk-space workaround.

## Deferred / Out-of-Scope Observations

- `docs/assets/recordings/battalion-formation.cast` contains a captured terminal line reading "Demonstrating ContinueOnError strategy..." from the pre-44-08 example output. It is a binary-ish recording artifact outside this plan's files; re-recording belongs with whoever refreshes `docs/DEMOS.md` assets. 44-13's guard should exclude `docs/assets` or the cast should be regenerated first.
- `docs/src/user-guides/battalion-patterns.md` Performance Notes still says Phalanx concurrency is capped by `BattalionConfig::max_concurrency`, a field that does not exist (`orchestration.md` correctly says `Phalanx::with_max_concurrency`). Pre-existing inaccuracy, not touched.
- `ConclaveConfig::with_timeout` and its `timeout_seconds` field still exist in code until 44-10; the Conclave page no longer teaches them.

## Known Stubs

None.

## Threat Flags

None. T-44-23 mitigated: every timeout description in these pages says per attempt, names the unbounded patterns and shows the `tokio::time::timeout` wrapper. T-44-24 mitigated by the targeted greps above.

## Requirements

LEGACY-02, LEGACY-03 and LEGACY-04 are advanced, not completed (the removal in 44-10, the MIGRATION rows in 44-11 and the guard in 44-13 remain). `requirements.mark-complete` was not run.

## Self-Check: PASSED

Commits `3833be60`, `d015a4d0`, `b452ffdd` exist on `claude/laughing-dirac-e0h2ax`; all thirteen files in the frontmatter exist and are modified.
