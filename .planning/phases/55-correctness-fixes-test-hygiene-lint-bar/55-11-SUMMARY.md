---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 11
subsystem: docs
tags: [rustdoc, intra-doc-links, adr-0033, cirel-08, d-11, d-12]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-08 (paladin-ports) and 55-10 (paladin-llm, -storage, -notifications) removed their five suppressions; 55-02, 55-03, 55-06 and 55-09 finished editing facade sources"
provides:
  - "the facade (paladin-ai) reads zero rustdoc diagnostics under both ADR-0033 bars with no crate-level suppression; all eight suppressions are gone workspace-wide"
  - "ADR-0033 amended in place (dated 2026-11-01 block, additions only) closing the kept-for-v0.10.0 disposition and superseding the 309 figure with the measured 474 / 455"
  - "docs/src/appendix/doc-coverage-report.md regenerated as a dated measurement record at commit 14ee125e7"
affects: [55-13, 58]

tech-stack:
  added: []
  patterns:
    - "A module's own //! docs resolve links in the parent scope: bare names that look in scope fail, so link with an explicit crate:: / paladin_ports:: / paladin_core:: / paladin_battalion:: path (label kept; super::-prefixed labels shortened to the item name)"
    - "A method that exists only as a trait implementation is not found under the type's path: link the trait method"
    - "Feature-gated, private, pub(crate) and cfg(test) targets are plain code spans"
    - "The workspace bars unify features across members and hide links a per-crate cargo doc -p run flags: run -p paladin-ai in default, --all-features and --no-default-features forms too"

key-files:
  created: []
  modified:
    - src/lib.rs
    - src/**/*.rs (37 files, doc comments only plus the three removed attribute lines)
    - .planning/decisions/0033-cargo-doc-warning-bar.md
    - docs/src/appendix/doc-coverage-report.md

key-decisions:
  - "D-11 applied as locked: three facade suppression lines removed, every diagnostic fixed where written, no doc sentence deleted, no rustdoc allow or expect at any level"
  - "Methods that exist only as trait implementations (InProcessArsenal::invoke, DocGraphRegistry::resolve, RunSubmissionService::cancel, RunEventStreamService::stream, ScheduleService::create/patch) keep their label and link to the trait method, because the type path does not resolve them"
  - "doc-coverage-report.md drops the unmeasured >= 90% crate list (PD-11-1) and records only what the bars and the missing_docs roots enforce; no coverage tool introduced (D-12)"

requirements-completed: [CIREL-08]

coverage:
  - id: D1
    description: "Facade rustdoc diagnostics fixed (140 default / 135 all-features to 0) and its three crate-level suppressions removed; no rustdoc allow or expect remains in crates/*/src or src"
    requirement: "CIREL-08"
    verification:
      - kind: other
        ref: "make doc-check (exit 0 at 14ee125e7)"
        status: pass
      - kind: other
        ref: "grep -rnE 'allow\\(rustdoc|expect\\(rustdoc' crates/*/src src prints nothing; grep -rln 'warn(rustdoc' crates/*/src src prints only crates/paladin-ports/src/lib.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "ADR-0033 amended in place with the 2026-11-01 block (additions only)"
    requirement: "CIREL-08"
    verification:
      - kind: other
        ref: "git diff of the ADR shows no removed line; grep 'Amendment, 2026-11-01, Phase 55 / CIREL-08'"
        status: pass
    human_judgment: false
  - id: D3
    description: "doc-coverage-report.md regenerated as the dated measurement record (bars, twelve missing_docs roots, eight removed suppressions)"
    requirement: "CIREL-08"
    verification:
      - kind: other
        ref: "make check-doc-config and make check-version-strings (exit 0); banner, bar commands, SHA and no >= 90% figure checked by grep"
        status: pass
    human_judgment: true
    rationale: "Whether the page reads as an adequate release-review record is a reader judgment; the factual claims are each backed by a command above"

duration: 40min
completed: 2026-11-01
---

# Phase 55 Plan 11: facade rustdoc links, ADR-0033 amendment and the measurement record Summary

**The facade dropped from 140 / 135 rustdoc diagnostics (default / all-features) to 0 with its three crate-level suppressions deleted, so the whole workspace meets ADR-0033's two bars with none of the eight suppressions left; ADR-0033 is amended in place and `doc-coverage-report.md` is now a dated measurement record at `14ee125e7`.**

Plan-start SHA: `2c62fbe0ba4da39a59932000b2cfdb41ed3583d4`

## Performance

- **Duration:** about 40 minutes
- **Tasks:** 3 of 3 (Task 1 is the tracer)
- **Files modified:** 37 under `src/` (131 insertions, 134 deletions, all doc-comment lines plus the three removed attributes), the ADR and the appendix page

## Accomplishments

- `#![allow(rustdoc::broken_intra_doc_links)]`, `#![allow(rustdoc::redundant_explicit_links)]` and `#![allow(rustdoc::invalid_html_tags)]` are gone from `src/lib.rs`. `grep -rnE 'allow\(rustdoc|expect\(rustdoc' crates/*/src src` prints nothing and `grep -rn 'warn(rustdoc' crates/*/src src` prints only `crates/paladin-ports/src/lib.rs:51`.
- `make doc-check` exits 0 on `14ee125e7`: both bars plus `cargo test --workspace --doc`, with a `Documenting` line for all 13 workspace members (`target/doc` was removed first so every member was rebuilt).
- ADR-0033 gained the dated amendment block after its 2026-09-18 amendment: the per-crate table, the two classes, zero `invalid_html_tags`, the fix rules, "no suppression is needed and re-adding one requires a reason recorded here", and the commit SHA. `git diff` shows additions only.
- `doc-coverage-report.md` has the `Regenerated 2026-11 (Phase 55, CIREL-08)` banner, the two bar commands with results at the named SHA, the twelve `#![warn(missing_docs)]` roots, the eight-row removed-suppression table and a one-paragraph history; the `SUMMARY.md` link is unchanged.

## Task Commits

1. **Task 1 (tracer): headings, redundant links, core and config** - `1cd7d0302` (docs). Suppressions still present; both workspace bars green on this head.
2. **Task 2: the rest of the links; the three suppression lines removed** - `14ee125e7` (docs). `make doc-check` green on this head.
3. **Task 3: ADR-0033 amendment and the regenerated report** - `6398857dd` (docs).

Tracer feedback gate: the tracer's `<verify>` (both workspace bars) was run end to end on the Task 1 tree before expanding and passed.

## Measured before (paladin-ai)

Measured at plan start with the three suppression lines removed in the working tree, as `cargo doc --workspace --no-deps --offline` and `cargo doc --workspace --all-features --no-deps --offline` (the second without `-D`, so it lists everything).

| Bar | Total | Unresolved link | Redundant explicit link target | `invalid_html_tags` |
|-----|-------|-----------------|--------------------------------|---------------------|
| default | 140 | 127 | 13 | 0 |
| all-features | 135 | 122 | 13 | 0 |

- Matches RESEARCH at `a78338ddb` (140 / 135, 127 + 13 / 122 + 13, 0 `invalid_html_tags`). Only `paladin-ai` produced diagnostics.
- Default bar only (5; all-features reads 0 for them): `cli` (`application/mod.rs:38`), `paladin_content` and `paladin_notifications` (`infrastructure/mod.rs:18-19`), and `PostgresTreasuryLedger` twice (`infrastructure/web/run_api_wiring.rs:389,448`). All-features only: none.
- 27 of the 140 carried a source span; the other 113 were located by matching the printed line text against `//!` and `///` lines in `src` (every one matched exactly one site).
- By area (default bar): `application/services/run/` 54 (`inspector.rs` 12, `worker.rs` 6, `events.rs` 5, `cancel.rs` 4, `schedule/` 6, plus `resolver.rs`, `submission.rs`), `application/services/treasurer/mod.rs` 12, `application/services/assistant/` 14, `application/services/paladin/` 21 (`middleware/` 20, `paladin_execution_service.rs` 1), `application/services/arsenal/` 4, `application/mod.rs` 3, `application/services/orchestration` 1, `application/services/parley` 1, `infrastructure/cadence.rs` 9, `infrastructure/mod.rs` 6, `infrastructure/telemetry/` 12, `infrastructure/web/` 16, `config/agent_runtime.rs` 4, `core/mod.rs` 2, `presets/mod.rs` 3.
- The 13 redundant links: `arsenal/in_process_arsenal.rs:20`, `orchestration/mod.rs:369`, `paladin/paladin_execution_service.rs:3696`, `parley/adapter.rs:4`, `run/worker.rs:815`, `telemetry/herald_sink.rs:6,46,50`, `telemetry/persisting_sink.rs:11`, `web/agent_host.rs:594,637`, `web/facade_provisioner.rs:103,128`.

Intermediate re-measure (Task 1 step 3, lines still removed, after the Task 1 fixes): 118 diagnostics default and 114 all-features (`paladin-ai` rows), none redundant, none in `src/core/` or `src/config/`. That is 140 - 13 - 2 (core) - 3 (application headings) - 4 (config) = 118.

## Measured after (workspace)

- Commit: `14ee125e7696c2258adbd9645fa3eb0e1779e95c` (Task 2), suppressions deleted.
- Default bar, `cargo doc --workspace --no-deps`: no `warning:` line (facade 140 -> 0).
- All-features bar, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`: exit 0 (facade 135 -> 0).
- `make doc-check`: exit 0 (both bars plus `cargo test --workspace --doc`).
- `cargo doc -p paladin-ai` with default features, `--all-features` and `--no-default-features`: 0 warnings each.
- `cargo fmt --check` clean. `make api-surface`: `API surface unchanged` (4325 items).
- Plan diff check: `git diff 1cd7d0302 14ee125e7 -- src`, doc-comment lines excluded, prints exactly the three removed attribute lines.

Five-crate before and after (from the 55-08, 55-10 and this plan's measurements):

| Crate | Default before | All-features before | Classes | After (both bars) | Commit |
|-------|----------------|---------------------|---------|-------------------|--------|
| `paladin-ports` | 187 | 187 | 170 unresolved + 17 redundant | 0 | `309774be8` |
| `paladin-llm` | 117 | 105 | unresolved | 0 | `8ea9f1357` |
| `paladin-storage` | 27 | 28 | unresolved | 0 | `d53d4233c` |
| `paladin-notifications` | 3 | 0 | unresolved | 0 | `23b6426d8` |
| `paladin-ai` (facade) | 140 | 135 | 127 + 13 / 122 + 13; 0 `invalid_html_tags` | 0 | `14ee125e7` |
| Total | 474 | 455 | | 0 | |

## How each class was fixed

- Unresolved, public and unconditional targets: explicit path, label kept. Facade items `crate::application::services::...` or `crate::infrastructure::...`; ports `paladin_ports::{input,output}::<module>::Item`; core `paladin_core::platform::container::<file>::Item`; battalion `paladin_battalion::engine::Item`. Labels written as `super::PromptSection` and similar were shortened to the item name (`PromptSection`) so the rendered text is readable; no sentence changed.
- Trait-implemented methods that the type path does not resolve (`InProcessArsenal::invoke`, `DocGraphRegistry::resolve`, `RunSubmissionService::cancel`, `RunEventStreamService::stream`, `ScheduleService::create` / `patch`, nine links): label kept, target is the trait method (`ArsenalPort::invoke`, `GraphResolver::resolve`, `RunSubmissionPort::cancel`, `RunEventStreamPort::stream`, `ScheduleAdminPort::create` / `patch`).
- Plain code spans: `cfg(test)` module targets (`tests::...` in `config/agent_runtime.rs`, `PaladinExecutionService::execute_json_schema`), private and `pub(crate)` items (`STORED_NOTES_NOT_INSTRUCTIONS`, `effective_history`, `build_agent`), the `otel`-gated `OtelTraceSink`, `PostgresTreasuryLedger`, and the feature-gated rows of the `infrastructure/mod.rs` table (`paladin_content`, `paladin_notifications`, `paladin_web`, `web`).
- Headings: `## [base]`, `## [platform]`, `## [services]`, `## [errors]`, `## [cli] (feature `cli`)` now read `## \[name\]`, which renders exactly the text the author wrote.
- 13 redundant: each explicit target removed so the shorthand stays, using the span rustdoc printed; each shorthand re-checked in the same run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The workspace bars hide a link a per-crate run flags**
- **Found during:** Task 2 per-crate re-runs
- **Issue:** `infrastructure/mod.rs:20` linked `[`paladin_web`]`; the workspace runs resolve it (another member enables the feature) but `cargo doc -p paladin-ai` in default and `--no-default-features` forms did not.
- **Fix:** the row became a code span like its `paladin_content` and `paladin_notifications` neighbours. All three per-crate forms now read 0.
- **Files modified:** `src/infrastructure/mod.rs`
- **Commit:** `14ee125e7`

**2. [Judgment, within D-11] Trait-method targets instead of code spans for implemented methods**
- The plan names code spans for feature-gated and higher-layer targets. Nine links in `run/`, `assistant/` and `arsenal/` name a method that exists only through a trait implementation; they are live links to the trait method rather than spans, and both bars accept them.
- **Commit:** `14ee125e7`

**3. Execution detail (no outcome impact):** a second run of my throwaway apply script, made to add two missing mappings, re-applied links on lines already fixed and doubled their target (`](path)(path)`) in the working tree. I caught it from the diff, removed the duplicated targets with one regex, confirmed `git diff` contained no `)(` sequence and no non-doc line, and rebuilt both bars before committing. Nothing doubled was ever committed.

**Total deviations:** 1 auto-fixed (Rule 3), 1 judgment call, 1 process note. No public API change; `make api-surface` unchanged.

## Known Stubs

None.

## Issues Encountered

- The scripts that parsed rustdoc output and applied the edits live in the session scratchpad and are not committed.
- Disk was not a problem (about 9 GB free). `target/doc` was deleted once (regenerable) so the final bar run documented every member; no tracked file was removed. All cargo runs used `CARGO_INCREMENTAL=0` and `--offline` (`make doc-check` ran with `CARGO_NET_OFFLINE=true`).
- `actionlint`, Docker and network access were not needed.

## Threat Flags

None. Doc-comment text, one ADR and one appendix page only. T-55-26 is mitigated: the SHA the report names (`14ee125e7`) is the commit on which `make doc-check` ran, and the unmeasured percentages are removed. T-55-27 is mitigated: the ADR diff is additions only.

## Next Phase Readiness

- Plan 55-13 can re-run both bars at the phase head against this record. The `paladin-llm --no-default-features` residual noted in 55-10 is not a CI bar and is unchanged.
- ROADMAP success criterion 4 (rustdoc suppressions cleared) has its evidence: the five-crate table above and `make doc-check` at `14ee125e7`.

## Self-Check: PASSED

- Commits `1cd7d0302`, `14ee125e7` and `6398857dd` exist on `claude/laughing-dirac-e0h2ax`.
- `src/lib.rs` has no rustdoc `allow`; the only `warn(rustdoc` line in `crates/*/src` and `src` is in `crates/paladin-ports/src/lib.rs`.
- The ADR has the `Amendment, 2026-11-01, Phase 55 / CIREL-08` block and no removed line; the report has the `Regenerated 2026-11 (Phase 55, CIREL-08)` banner, no `Archived`, no `>= 90` figure; `docs/src/SUMMARY.md` is unchanged; `make check-doc-config` and `make check-version-strings` exit 0.
