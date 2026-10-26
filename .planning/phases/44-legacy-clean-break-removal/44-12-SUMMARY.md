---
phase: 44-legacy-clean-break-removal
plan: 12
subsystem: release-record
tags: [changelog, adr, conformance, phase-gate, credential-review, clean-break]
status: complete

requires:
  - phase: 44-11
    provides: MIGRATION.md 9.1 M-B-06..M-B-11, the ten Phase 44 9.2 rows, the measured allowlist block (set-equal)
  - phase: 44-13
    provides: tests/legacy_removal_guard.rs (rules 1-6, positive controls, clean scan)
provides:
  - Root CHANGELOG [Unreleased] Phase 44 Removed and Changed blocks (D-21) citing ADR-0059, LEGACY-01..04 and MIGRATION.md 9.1 / 9.2
  - Phase 44 entries in the paladin-core, paladin-battalion and paladin-herald CHANGELOGs
  - ADR-0059 with Code Locations reconciled against the shipped diff and Code Conformance "conforms"
  - PROJECT.md ADR-0059 Outcome "Implemented -- Phase 44 (2026-10-11)"
  - The full phase gate and the manual credential-handling review on record
affects: [46, 47]

tech-stack:
  added: []
  patterns:
    - "Closeout plan reconciles an ADR's Code Locations mechanically: git diff --name-only against the phase base compared with the backticked paths in the section"

key-files:
  created: []
  modified:
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - .planning/decisions/0059-legacy-clean-break-removal.md
    - .planning/PROJECT.md

key-decisions:
  - "ADR-0059 Code Conformance is conforms: every decision group (a) to (j) names its proving tests, all of which exist and pass"
  - "LEGACY-04 is ticked: ADR, ten 9.2 rows, fifteen allowlist entries (set-equal) and updated examples and mdBook pages all exist and the guard pins the tree"

requirements-completed: [LEGACY-04]

duration: ~1h
completed: 2026-10-11
---

# Phase 44 Plan 12: CHANGELOG record, ADR-0059 conformance and the phase gate Summary

**Phase 44 is recorded and sealed: the root and three crate CHANGELOGs name every removal and behaviour change with its replacement, ADR-0059 reads `conforms` with the proving tests per decision, and the full gate (workspace tests, doctests, clippy, fmt, clean-code, security, release gates, API surface, examples, MSRV 1.88, both guards) is green apart from the two known pre-phase baseline failures.**

## Accomplishments

- **Task 1 (commit `e94394c6`):**
  - Root `CHANGELOG.md` `[Unreleased]`: four `### Changed` bullets (`BattalionConfig.aegis` with per-attempt timeouts, Aegis retry, `Absorb` and `validate_aegis`; the `node_errors` element shape and herald lines; the Conclave and circuit-breaker classification and engine backoff; the Commander's defaults) and one `### Removed` group with six sub-bullets (the three legacy types; the three `BattalionConfig` fields and builders; `BattalionError::Timeout` and `ConclaveError::Timeout`; `ConclaveConfig::timeout_seconds` / `with_timeout`; `PaladinError::LlmError`, `is_retryable`, `is_terminal`; the `paladin_battalion::retry` module and its facade re-export), each with its replacement. Every group ends with "(ADR-0059; LEGACY-01..04; see MIGRATION.md 9.1 M-B-06..M-B-11 and the Phase 44 9.2 rows)" or the specific rows.
  - `paladin-core`, `paladin-battalion` and `paladin-herald` CHANGELOGs each carry only their own Phase 44 surface (core: aegis field and builders, the `node_errors` retype, the removals; battalion: the runner, Formation / Phalanx / Campaign / Conclave, the Commander, the `retry` module removal; herald: the three herald lines and the JSON element shape).
  - ADR-0059: `## Code Locations` reconciled against `git diff --name-only b362d6cb -- . ':!.planning'` (71 files). One file was missing (`crates/paladin-battalion/src/engine/mod.rs`, an intra-doc link retargeted by 44-10), `retry.rs` is marked deleted, and the root `Cargo.toml` was dropped because it is unchanged (44-11 kept the lint lines in the two crate manifests). `## Code Conformance` is `conforms`, with one bullet per decision group (a) to (j) naming tests, the D-27 per-crate lint headline and the deviations the plan summaries recorded.
  - `PROJECT.md`: the ADR-0059 Outcome cell now starts `✅ Implemented — Phase 44 (2026-10-11)` with key files and test names, in the ADR-0058 row's style.
- **Task 2:** the full gate and the credential review below. No source file changed, so no gate needed a fix.

## Task Commits

1. **Task 1: changelogs, ADR-0059 conformance, PROJECT.md outcome** - `e94394c6` (docs)
2. **Task 2: gate and credential review** - recorded in this SUMMARY (no repository change)

## Phase gate (all run in this plan, on HEAD `e94394c6`)

| Command | Result |
| ------- | ------ |
| `cargo test --workspace --no-fail-fast` | exit 101 only because of the 2 known baseline failures: 6884 passed, 2 failed, 229 ignored across 59 test targets. The only failures are `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (`paladin-ai` lib, 1348 passed / 2 failed), which fail identically on the pre-phase baseline (`deferred-items.md`). **A run whose only failures are those two counts as passing; they were not touched.** No `-- --skip` was applied: Phase 43's "environment-bound web tests" skip was not needed because these two are the only red tests and they are baseline. |
| `cargo test --workspace --doc` | exit 0, 602 passed, 0 failed, 210 ignored (pre-existing `ignore`s) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `make clean-code` (fmt, lint, lint-shell, check, doc-check, check-api-examples) | exit 0; shellcheck clean; doc-check passed (ADR-0033); all 105 D-05 public API entry points carry a plural `# Examples` heading |
| `make security` (`cargo audit` + `cargo deny check`) | exit 0. cargo-audit loaded 1296 advisories from the local DB and scanned 691 crates: no vulnerabilities, 10 allowed (unmaintained) warnings; cargo-deny: advisories ok, bans ok, licenses ok, sources ok |
| `make check-gates` | exit 0 (changelogs, crate names, advisory register, workflow suppressions and triggers, CodeQL dismissals, migration allowlist set-equal, publish order across 12 crates) |
| `make check-doc-examples` | exit 0 (0 checked, 617 skipped as fragments / `ignore`, 0 failed) |
| `make check-doc-config` | exit 0 (152 YAML blocks checked, 0 failed) |
| `make check-examples` | exit 0 (62 of 62 example binaries present, CI feature-split matrix); `target/debug/examples` cleared right after |
| `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | exit 0, "API surface unchanged" (4274 items) |
| `./scripts/check-changelogs.sh` | exit 0 (11 publishable crates checked) |
| `cargo test --test legacy_removal_guard` | exit 0, 3 passed, 0 failed (run before the task-1 commit and again in the gate) |
| `cargo test --test treasurer_vocabulary_guard` | exit 0, 3 passed, 0 failed |
| MSRV 1.88: `rustup toolchain install 1.88 --profile minimal` then `cargo +1.88 check --workspace --all-features` (separate target dir, deleted afterwards) | installed rustc 1.88.0; exit 0 in 3m 10s. The CI `msrv` job remains the authoritative gate |
| 82 percent workspace line coverage (ADR-0006) | **Not run locally: `cargo-llvm-cov` is not installed. CI-gated** by the `coverage` job (`cargo llvm-cov --fail-under-lines`). New test modules covering the new code: `aegis_attempt::tests` (10), the `validate_aegis_*` tests, `formation_service` / `phalanx_service` / `campaign_service` / `commander` / `conclave_execution_service` test modules, the `battalion_result_*` and herald line tests, the breaker and predicate tables, and `tests/legacy_removal_guard.rs` |

Disk: `target/debug/examples` and `target/debug/incremental` were cleared before each full build; no run hit "no space left on device", so no retry was needed.

## Manual credential-handling review (security.instructions.md)

1. **No redaction path changed.** `git diff b362d6cb -- crates/paladin-llm/src/redaction.rs` is empty (0 lines) and nothing under `crates/paladin-llm` changed (`git diff --stat b362d6cb -- crates/paladin-llm` prints nothing). Every error text that reaches `node_errors`, the `AggregationError` message or a herald line comes from an already-redacted `PaladinError` display: `llm_failure::to_paladin_error` (`crates/paladin-battalion/src/llm_failure.rs:86-95`) is unchanged in logic (the diff there is the removed variant's arm and test rewrites) and copies `err.to_string()` of the adapter's own `LlmError`, which the adapters redact before truncating. The herald lines add only node id, attempt and transience to that text, and `herald_node_error_line_adds_no_provider_detail` pins that neither the provider name nor the HTTP status appears.
2. **No new log line interpolates a key, input or output.** Every added `warn!` / `info!` / `debug!` in the phase diff (script over `git diff -U0 b362d6cb -- crates src examples tests`, ten macros): `aegis_attempt.rs:210` (Paladin name, attempt, transience, delay), `campaign_service.rs:243` (Campaign name) and `:269` (id, milliseconds), `formation_service.rs:181` (id), `:240` and `:249` (position, Paladin name, attempt), `:276` (`AggregatedError::summary()`, which is counts and a failure rate only, `error_aggregation.rs:139-146`), `phalanx_service.rs:193` (name), `:480` (counts), `:528` (Paladin name). None carries a key, Paladin input or output. **Correction to the plan's wording:** the Conclave retry logs (`conclave_execution_service.rs:321`, `:332`, `:348`, `:257`, `:130`) are not new and are not name-attempt-transience-delay only; they pre-date the phase (the diff of that file adds no log macro and removes one `warn!`) and interpolate the `PaladinError` / `ConclaveError` display, which is the same already-redacted text as item 1, never an input or an output.
3. **No credential-carrying config type became `Debug`- or `Serialize`-visible.** `git diff -U0 b362d6cb` for `derive(`, `api_key`, `secret`, `bearer` and `password` shows only three added `#[derive(Debug...)]` lines, all on the crate-private runner types in `aegis_attempt.rs` (`AttemptBound`, `AttemptOutcome`, `AttemptFailure`), which hold no key and are never logged; the removed derives belong to the deleted legacy types. `BattalionConfig` gained `aegis: Aegis`, a policy bundle with no credential. No dependency or `Cargo.lock` change: the two manifest diffs are `cargo-semver-checks` lint lines and metadata only.
4. **No HTTP client added, no redirect policy changed.** `git diff b362d6cb -- . ':!.planning' | grep -c 'reqwest::Client\|redirect::Policy'` prints **0**, and no added or removed line mentions `reqwest` or `redirect`. No Snyk step was added; CodeQL stays advisory-only and is not counted as coverage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Table row in `.planning/PROJECT.md` had swallowed a paragraph break.**
- **Found during:** Task 1, while setting the ADR-0059 Outcome cell.
- **Issue:** plan 44-02's commit `b73a4564` replaced the blank line and the heading line `**v0.9.0 (Phases 18-21) minted no new ADRs.** Its decisions were recorded as per-phase locked` with the ADR-0059 table row, leaving that heading text glued to the row as a bogus fifth cell.
- **Fix:** ended the row after the Outcome cell, restored the blank line and put the paragraph's first line back on its own line (bytes identical to the `b362d6cb` text).
- **Files modified:** `.planning/PROJECT.md` (inside this plan's file list)
- **Commit:** `e94394c6`

**2. [Plan wording] Credential-review item 2 reworded, see above.** The plan asserted the Conclave retry logs carry name, attempt, transience and delay only; the code shows they are pre-existing lines that print the redacted error display. The conclusion (no key, input or output is logged) stands.

**3. [Plan reading] Phase 43's two environment-bound web-test skip was not applied.** The only red tests are the two `run_api_wiring` baseline tests; they are reported, not skipped.

**4. [Plan reading] `cargo test --test legacy_removal_guard` was also run before the Task 1 commit** to make sure the new CHANGELOG / ADR text sits inside the guard's history set (it does).

No Rule 2, 3 or 4 deviations. No auth gates. No source file changed.

## Issues Encountered

- The two baseline `paladin-ai` lib failures above (`deferred-items.md`, panic at `run_api_wiring.rs:2369`), unchanged and out of scope.
- `mdbook` is not installed here (as in 44-09), so the book was not rendered; `make check-doc-examples` and `make check-doc-config` are syntax scans.
- `docs/assets/recordings/battalion-formation.cast` still shows the pre-44-08 example output (a recording, outside the guard's scanned extensions); regenerating it belongs with the next `docs/DEMOS.md` asset refresh (carried from 44-09 and 44-10).

## Requirements

**LEGACY-04 is ticked** (`requirements.mark-complete`). Its success criterion is met: the X-03 supersession is recorded in ADR-0059 (`conforms`); every removal has a MIGRATION.md 9.2 row (ten rows, nine `Y`, one register-only `N`) and every `Y` row a semver allowlist entry (`make check-migration-allowlist` set-equal in both directions, inside `make check-gates`); and the examples, doc-examples crate and thirteen mdBook / demo pages were moved to the Aegis forms (44-08, 44-09), with `make check-examples` building all 62 binaries and the guard pinning the tree. LEGACY-01..03 were ticked by 44-10. `phase.complete` was not run; the orchestrator does that after verification.

## Known Stubs

None. No placeholder values, empty data sources or TODO text in the files this plan changed.

## Threat Flags

None. Documentation only; no new endpoint, auth path, file access or schema surface. Threat register: T-44-30 mitigated by the four-item review above, `make security` and clippy `-D warnings`; T-44-31 mitigated (no dependency added or bumped, `Cargo.lock` unchanged, `make security` green); T-44-32 mitigated (CHANGELOG blocks cite ADR-0059 and the MIGRATION.md rows, ADR conformance names the tests).

## Next Phase Readiness

Phase 46 (the v0.10 to v0.11 MIGRATION guide) and Phase 47 (the v0.11.0 release) can build on the 9.1 / 9.2 rows, the allowlist and the CHANGELOG blocks. For the orchestrator: all thirteen Phase 44 plans now have summaries; mark the phase complete after verification.

## Self-Check: PASSED

Verified: the four CHANGELOG files, ADR-0059 and `.planning/PROJECT.md` carry the Task 1 edits; commit `e94394c6` is on `claude/laughing-dirac-e0h2ax`; ADR-0059's Code Locations lists `crates/paladin-battalion/src/aegis_attempt.rs` and `tests/legacy_removal_guard.rs` and its Code Conformance names `commander_campaign_honours_aegis_per_attempt_timeout_end_to_end` and `legacy_surfaces_are_gone_from_the_tree`; every test named in the Code Conformance bullets exists in the tree (checked by `grep -rn "fn <name>"`).
