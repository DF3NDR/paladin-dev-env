---
phase: 57-register-hygiene-close-out
plan: 09
subsystem: review-disposition-register
tags: [reg-05, register, review-dispositions, evidence-hunt]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-08 checker (scripts/check-review-dispositions.sh), register shape, LIVE convention and the appended-section wording"
provides:
  - "12 register rows for Phases 38, 44, 47 and 51 in 57-REVIEW-DISPOSITIONS.md (8 closed, 4 open with LIVE markers)"
  - "an appended '## Dispositions (Phase 57)' pointer section in 38, 44, 47 and 51 REVIEW.md (16 lines added, 0 deleted)"
  - "four live findings classified for the 57-10 triage: 38 WR-01 and WR-02 (other), 51 WR-01 (security), 51 WR-02 (other)"
affects: [57-10, 57-11]

tech-stack:
  added: []
  patterns:
    - "closure only on a commit confirmed with git or gh api, a head file:line, and a test re-run in this phase; otherwise a LIVE row"

key-files:
  created: []
  modified:
    - .planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md
    - .planning/milestones/v0.11.0-phases/38-design-seams-pricing-cost-producer/38-REVIEW.md
    - .planning/milestones/v0.11.0-phases/44-legacy-clean-break-removal/44-REVIEW.md
    - .planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-REVIEW.md
    - .planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-REVIEW.md

key-decisions:
  - "38 CR-01 is closed on the fix commit and the head code path, with the caveat recorded in the row that no test pins the before_model early return"
  - "38 SECURITY.md calling WR-01 and WR-02 'advisory' is not an owner waiver (no owner is named), so both stay open"
  - "51 WR-01 is classed security (free vendor text from an out-of-tree port reaches a ConfigurationError unredacted); latent, because no in-tree port returns such an error"
  - "44 WR-05 closes on 2d3cf119a, which removed the crate-wide allows entirely, not on the dated notes the todo recorded"

patterns-established: []

requirements-completed: []

duration: ~35min
completed: 2026-11-04
---

# Phase 57 Plan 09: Phases 38, 44, 47 and 51 review dispositions Summary

**Twelve register rows for the four phases whose warnings had no REVIEW-FIX trail: 8 closed on commits and tests checked today, 4 left open and live (one security-class), with the cumulative checker green over 32 findings in 10 files.**

REG-05 is not ticked here: it spans 57-09, 57-10 and 57-11, and 57-11 ticks it (orchestrator instruction).

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-11-04
- **Tasks:** 2 (1 tracer, 1 auto)
- **Commits:** 2 task commits

## Accomplishments

- Every disposition rests on a command run in this phase. Nothing was closed because a later phase "probably" fixed it (P-09-1).
- `./scripts/check-review-dispositions.sh --phases 38,40,42,44,46,47,48,50,50.1,51 --pending-ok` exits 0: `checked 32 finding(s) across 10 review file(s); 0 failure(s)`.
- The register totals line reads `closed: 28, waived: 0, open: 4`.
- The four archived REVIEW.md files each gained one appended section; `git diff --numstat` over the two commits shows 16 lines added and 0 deleted (P-09-2). No changelog and no archived SUMMARY or VERIFICATION file was edited.

## Task Commits

1. **Task 1 (tracer): Phase 38 hunted and dispositioned** - `c75d654fa` (docs)
2. **Task 2: Phases 44, 47, 51 dispositioned** - `98d96639a` (docs)

## Per-finding evidence

Re-run date for every test: 2026-11-04, on this head. The clone is shallow (boundary 2026-10-08), so the one pre-boundary commit (`db7f086d3`) was checked with `gh api repos/DF3NDR/paladin-dev-env/commits/db7f086d3`; all others with `git log -1` or `git show --stat`.

Phase 38 (no REVIEW-FIX exists; the evidence is hunted):
- 38 CR-01 closed. `gh api` confirmed `db7f086d3` ("fix(38): report accumulated cost on the before_model early-return path", 2026-09-26, touches `paladin_execution_service.rs` +4 -1). Read the head: the early return reads `cost: cost_tally.total()` at `src/application/services/paladin/paladin_execution_service.rs:1960`, and all four return paths use it (lines 1960, 2117, 2405, 2434). `CARGO_INCREMENTAL=0 cargo test -p paladin-ai --lib agent_loop_`: 25 passed, including `agent_loop_sums_cost_across_calls`. **Caveat:** that test pins the tally across calls, not the `before_model` early return. The review's suggested regression test (a middleware finishing the run on loop 2) was never added, and `db7f086d3` added none. The row says so; the closure rests on the commit and the file state.
- 38 WR-01 open. Read `Cost::new` at `crates/paladin-core/src/platform/container/cost.rs:147` (no validation), the `Add`/`AddAssign` impls (`:179-199`) and the type's rustdoc (`:114-120`, no statement about sign). Non-test callers (`treasurer/derive.rs:167`, `evaluate.rs:121`, `mod.rs:320`, `run_api_wiring.rs:4181`) all pass non-negative figures, so the gap is latent. `38-SECURITY.md:113` calls it "advisory", which names no owner and is not a waiver.
- 38 WR-02 open. `UNPRICED_MODEL_WARNING_CAPACITY` is still `const ... = 256` at `crates/paladin-llm/src/pricing.rs:69`, feeding one `LazyLock` at `:118`. No reset or per-adapter set exists.

Phase 44 (`44-REVIEW-FIX.md` for WR-01/02; completed todos for WR-03..05):
- 44 WR-01 closed. `cf0be3dbe` (GREEN) and `229a28a51` (RED) exist. `cargo test -p paladin-battalion --lib -- cancelled_phalanx_does_not_start_queued_paladins phalanx_rejects_zero_max_concurrency cancelled_before_the_first_attempt`: 3 passed (test at `phalanx_service.rs:903`).
- 44 WR-02 closed. `cf0be3dbe`, and `dc55f18ea` (limit above `Semaphore::MAX_PERMITS`) exist. `validate_concurrency_limit` at `phalanx_service.rs:517`; `phalanx_rejects_zero_max_concurrency_before_any_paladin_runs` (`:943`) passed in the same run.
- 44 WR-03 closed. Completed todo `2026-10-11-reject-removed-v0-10-battalion-config-keys.md` (completed 2026-10-24; "Resolved by Phase 46 plan 46-08 (option B)"). Commits `bcec5e8e8`, `b04294363` exist. Head: `#[serde(try_from = "legacy_keys::BattalionConfigWire")]` at `crates/paladin-core/src/platform/container/battalion/mod.rs:69`. `cargo test -p paladin-ai-core --lib -- legacy_keys battalion_config_v0_10_document_is_refused`: 16 passed, including `battalion_config_v0_10_document_is_refused_naming_each_removed_key`.
- 44 WR-04 closed. Completed todo `2026-10-11-phase-44-review-follow-ups.md` ("Resolved by Phase 46 plan 46-09"). `3cf57a003` exists. Head: `maneuver_ignored_aegis_policies` at `commander.rs:315`, warn site `:991`; the `aegis` rustdoc at `battalion/mod.rs:102-109` lists Maneuver and says ChainOfCommand, Grove and Council are not time-bounded; `docs/src/user-guides/orchestration.md:332` says the same. `cargo test -p paladin-battalion --lib -- maneuver_ignored_aegis_policies maneuver_warning_fires`: 2 passed. Not done: the review's optional build-time warning for ChainOfCommand/Grove, and a Maneuver row in the user-guide table (the todo asked for neither).
- 44 WR-05 closed. The todo's dated notes landed in `68d03cdba`; the allows were then removed outright by `2d3cf119a` (52-05, 2026-10-28, 327 lines deleted across the root and 5 crate manifests). Head: `grep -rn cargo-semver-checks Cargo.toml crates/*/Cargo.toml` prints nothing, and `.cargo/semver-checks-allowlist.toml` has no `struct_missing` or `enum_missing` line.

Phase 47 (no REVIEW-FIX; published `[0.11.0]` sections are immutable, D-00c):
- 47 WR-01 closed. `466390bcd` (54-05, FIRST-05, `[x]` in REQUIREMENTS.md) adds a "Correction to the `[0.11.0]` entry" under `## [Unreleased]` in `crates/paladin-memory/CHANGELOG.md:8-12` and `crates/paladin-content/CHANGELOG.md:8-12`, both read today. The published `[0.11.0]` text ("No functional change...") is untouched.
- 47 WR-02 closed. `70a14ba8e` (54-08, FIRST-07) is a one-line deletion of the `[Unreleased]:` link. Head: `grep -c '^\[Unreleased\]:' CHANGELOG.md` prints 0. The footer still defines only `[0.2.0]` and `[0.1.0]`; the review offered deleting the line as the fix, so this is closed with that noted in the row.

Phase 51 (no REVIEW-FIX; nothing in WINDOWS, STATE, ROADMAP or the pending todos mentions either finding):
- 51 WR-01 open. `paladin_builder.rs:1202` still has `other => ... could not be checked: {other}`; `fallback.rs:981` still forwards `Err(other) => return Err(other)`; the doc comment at `:1188-1189` still says nothing request-derived is rendered; `git grep 'could not be checked'` finds no test.
- 51 WR-02 open. `agent_host.rs:201` `check_block_reasoning` is still the hand-rolled copy; the hint literal is at `agent_host.rs:218`, `reasoning.rs:304` and `llm_port.rs:2630`.

## Live in tree

| Id | file:line | Class | Reason |
|----|-----------|-------|--------|
| 38 WR-01 | `crates/paladin-core/src/platform/container/cost.rs:147` | other | `Cost::new`, `Add` and `AddAssign` still accept a negative amount and the rustdoc is silent on sign. Latent: no producer builds one. Fix is one doc sentence or a validated constructor. |
| 38 WR-02 | `crates/paladin-llm/src/pricing.rs:69` | other | Fixed process-wide 256-name warning cap. Deliberate per the module docs (D-08); costs only a suppressed warn line. A decision record is wanted only if multi-tenant pricing or hot price reload is added. |
| 51 WR-01 | `src/application/services/paladin/paladin_builder.rs:1202` | security | `effort_refusal` renders the whole `LlmError` into a `ConfigurationError`. `ProcessingError(String)` and the HTTP-status variant carry free vendor text, unredacted, from any out-of-tree port, against `security.instructions.md` (redact before embed) and the function's own doc claim. Latent: no in-tree port reaches the arm. |
| 51 WR-02 | `src/infrastructure/web/agent_host.rs:201` | other | Hand-rolled copy of the `check_reasoning` verdict; the hint literal lives in three places and the wording differs from the adapters'. Verdicts agree on `is_ok`. |

The classes are the executor's call under PD-09-1; 57-10's triage is where the owner confirms them. 51 WR-01 is the only security-class item and is the one T-57-32 (high) names.

## Checker results

- Task 1 scoped: `checked 3 finding(s) across 1 review file(s); 0 failure(s)`.
- Cumulative scoped (Task 2): `checked 32 finding(s) across 10 review file(s); 0 failure(s)`.
- Strict (no flags) was run once for information: 26 failures, all expected at this point (Phase 41, 43 and 45 rows belong to 57-10; the four `open` rows here fail strict by design until 57-10 fixes or waives them). 57-11 runs it strict.

## Deviations from Plan

None - plan executed exactly as written.

One plan reading, not a deviation: the plan says to "update `**Totals:**` and `## Live in tree`"; both live in the register (the archive is append-only), so that is where they were updated. The two cited line numbers I first wrote wrongly (`fallback.rs:978`, `paladin_builder.rs:1186`) were corrected to `:981` and `:1188-1189` before the Task 2 commit.

### Tracer gate

Tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved.

The Task 1 `<verify>` command was re-run against the committed tracer (`c75d654fa`) and passed (scoped checker `checked 3 finding(s) across 1 review file(s); 0 failure(s)`, pointer heading present, 0 deleted lines) before Task 2 began.

### Authentication gates

None. `gh api` worked for the one pre-boundary commit.

## Issues Encountered

- A first draft of Task 2's register rows cited two line numbers a few lines off; found by re-reading the files and fixed before committing.
- `make api-surface` was not run: no Rust source, public item or crate file changed in this plan (planning files only).
- `cargo test -p paladin-core` fails to resolve: the package is named `paladin-ai-core`. Used that name.

## Known Stubs

None.

## Threat Flags

None. The plan edits planning text only; no endpoint, credential path or schema. The one security-relevant finding (51 WR-01) pre-exists and is recorded, not introduced.

## Next Phase Readiness

- 57-10 triages the four live rows here, plus its own Phase 41, 43 and 45 rows. 51 WR-01 needs a fix-or-waive decision (D-14); a fix would add a test that plants a secret in the `LlmError` and asserts it is absent from the refusal.
- 57-11 must run the checker with no flags and tick REG-05.

## Self-Check: PASSED

- Commits exist: `c75d654fa`, `98d96639a` (verified with `git log`).
- Files exist: the register, the four REVIEW.md files and this SUMMARY.
- Cumulative scoped checker exit 0 over 32 findings in 10 files; archive diff deletes 0 lines.
