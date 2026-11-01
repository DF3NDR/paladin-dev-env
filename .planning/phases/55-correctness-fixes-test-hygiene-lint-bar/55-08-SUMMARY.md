---
phase: 55-correctness-fixes-test-hygiene-lint-bar
plan: 08
subsystem: paladin-ports
tags: [rustdoc, intra-doc-links, adr-0033, cirel-08, d-11]
status: complete

requires:
  - phase: 55-correctness-fixes-test-hygiene-lint-bar
    provides: "plans 55-01..55-07 landed; no code dependency"
provides:
  - "paladin-ports reads zero rustdoc diagnostics under both ADR-0033 bars with no crate-level suppression"
  - "before and after counts per bar, class and file, for plan 55-11's ADR-0033 amendment and plan 55-13's evidence"
affects: [55-10, 55-11, 55-13]

tech-stack:
  added: []
  patterns:
    - "A module's own `//!` docs do not resolve its own items by bare name: write `[`Item`](crate::<layer>::<module>::Item)`; core types get `paladin_core::platform::container::<file>::Item`; a higher-layer target is a plain code span"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/lib.rs
    - crates/paladin-ports/src/input/*.rs (7 files)
    - crates/paladin-ports/src/output/*.rs (35 files)
    - .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md

key-decisions:
  - "D-11 applied as locked: both suppression lines removed, the warn line kept, every diagnostic fixed where written, no doc sentence deleted"
  - "PD-08-2 followed: commit 1 keeps the suppressions (green with them), commit 2 removes them once the crate reads zero"
  - "A stale link name was corrected rather than code-spanned: VisionLlmPort -> the real trait VisionCapableLlm"

requirements-completed: [CIREL-08]

duration: 45min
completed: 2026-11-01
---

# Phase 55 Plan 08: paladin-ports rustdoc links Summary

**`paladin-ports` dropped from 187 rustdoc diagnostics to 0 under both CI bars and no longer carries a crate-level suppression: 170 unresolved links got explicit paths or code spans, 17 redundant explicit targets became shorthand, and rustdoc is the only thing that changed.**

Plan-start SHA: `1fb408516da432ebd094492ac763f210b6da9c5c`

## Performance

- **Duration:** about 45 minutes
- **Tasks:** 2 of 2 (Task 1 is the tracer)
- **Files modified:** 43 under `crates/paladin-ports/src` (178 insertions, 179 deletions), plus `deferred-items.md`

## Accomplishments

- Both `#![allow(rustdoc::broken_intra_doc_links)]` and `#![allow(rustdoc::redundant_explicit_links)]` are gone from `crates/paladin-ports/src/lib.rs`; `#![warn(rustdoc::broken_intra_doc_links)]` stays. No rustdoc `allow` or `expect` exists anywhere in the crate.
- The stale plain comment above the `warn` line (it said cross-crate links were "unavailable in this isolated crate" and that the lint was downgraded "so `cargo doc` still succeeds") was rewritten to say what is true now.
- Diagnostics were fixed where written, with a throwaway parser and apply script kept in the session scratchpad (not committed), then every changed link was re-checked by a doc build.

## Task Commits

1. **Task 1 (tracer): crate root, `input/` and every redundant link** - `d08cb6e30` (docs). Suppressions still present; both workspace bars green on this head.
2. **Task 2: output ports and suppression removal** - `309774be8` (docs). Both bars green on this head with the lines removed.

Tracer feedback gate (auto mode active): the tracer's `<verify>` was run end to end on the Task 1 tree before expanding and passed. Tracer verified end-to-end, expanding.

## Measured before

Measured at plan start with the two suppression lines removed, run as `cargo doc --workspace --no-deps` and `cargo doc --workspace --all-features --no-deps --offline` (the second without `-D`, so it lists everything):

| Bar | Total | Unresolved link | Redundant explicit link target |
|-----|-------|-----------------|--------------------------------|
| default (`cargo doc --workspace --no-deps`) | 187 | 170 | 17 |
| all-features (`--all-features`) | 187 | 170 | 17 |

- Matches RESEARCH at `a78338ddb` (187 / 187, 170 / 17). No diagnostic appeared under one bar only: the two outputs are identical after dropping the `Documenting`, `Checking`, `Finished` and `Generated` lines.
- Only `paladin-ports` produced diagnostics (`warning: paladin-ports (lib doc) generated 187 warnings` is the only crate summary line).
- By area: `lib.rs` 1, `input/` 29, `output/` 157 (total 187). 31 of the 187 were located by the span rustdoc printed (the 17 redundant ones and a few unresolved ones); the rest carried no source span and were located by matching the printed line text against the sources. Two of those matched several sites (`crate::application::ports` on four near-identical "Related" lists, and `Namespace` at `vault_port.rs` lines 15 and 78) and were assigned by hand.
- Per file: `input/allowance_admission_port.rs` 6, `assistant_admin_port.rs` 4, `parley_port.rs` 5, `run_event_stream_port.rs` 2, `run_inspector_port.rs` 4, `run_submission_port.rs` 2, `schedule_admin_port.rs` 6; `lib.rs` 1; `output/arsenal_port.rs` 7 (all redundant), `assistant_repository_port.rs` 4, `battalion_port.rs` 3, `cadence_port.rs` 7, `cancellation_probe.rs` 4, `citadel_port.rs` 3 (all redundant), `embedding_port.rs` 4, `file_storage_port.rs` 1, `garrison_port.rs` 6 (1 redundant), `llm_port.rs` 3, `model_profile.rs` 13, `node_cache_port.rs` 4, `notification_port.rs` 1, `orchestrator_port.rs` 2, `paladin_port.rs` 5, `queue_port.rs` 4 (all redundant), `rate_limit_hints.rs` 4, `run_queue_port.rs` 2, `run_repository_port.rs` 6, `run_schedule_repository_port.rs` 2, `run_trace_port.rs` 6, `sanctum_port.rs` 4, `spend_guard.rs` 6, `spend_scope.rs` 19, `streaming_executor_port.rs` 1, `structured_executor_port.rs` 4 (2 redundant), `token_counter_port.rs` 3, `trace_sink_port.rs` 6, `treasury_ledger_port.rs` 4, `treasury_notice_port.rs` 4, `vault_confined.rs` 6, `vault_port.rs` 1, `waypoint_port.rs` 6, `webhook_delivery_port.rs` 1, `workflow_repository_port.rs` 1.
- Redundant links were all in `output/` (`arsenal_port.rs` 7, `citadel_port.rs` 3, `queue_port.rs` 4, `structured_executor_port.rs` 2, `garrison_port.rs` 1), not in `lib.rs` or `input/`.

Intermediate re-measure (Task 1 step 3, lines removed, after the Task 1 fixes): 140 diagnostics under each bar, all `unresolved link`, none in `lib.rs` or `input/`, none redundant. That is 187 - 30 (lib + input) - 17 (redundant) = 140, all in `output/`.

## Measured after

- Head `309774be8f20fb56c3d0daa2dd506eeb76428da3`, suppressions removed:
  - default bar: `cargo doc --workspace --no-deps --offline` prints no `warning:` line (0 for `paladin-ports`; 187 -> 0).
  - all-features bar: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --offline` exits 0 (187 -> 0).
  - per class: unresolved link 170 -> 0, redundant explicit link target 17 -> 0.
- `cargo test -p paladin-ports --doc --offline`: 230 passed, 0 failed, 94 ignored (unchanged by this plan).
- `grep -rnE 'allow\(rustdoc|expect\(rustdoc' crates/paladin-ports/src` prints nothing; `grep -c 'warn(rustdoc::broken_intra_doc_links)' crates/paladin-ports/src/lib.rs` prints 1.
- `cargo fmt --check -p paladin-ports` clean. `make api-surface` reads `API surface unchanged` (4325 items, nightly-2026-09-20).
- Both workspace bars were also green on the Task 1 head (`d08cb6e30`) with the suppressions in place.

## How each class was fixed

- 170 unresolved:
  - `paladin-ports` items named from a module's own `//!` docs (the bulk): explicit `crate::input::<module>::Item` or `crate::output::<module>::Item` target, keeping the label.
  - `paladin-core` types (`RunAttribution`, `ModelProfile` neighbours such as `ReasoningEffort`, `Namespace`, `ThreadId`, `Assistant`, `HaltReason`, ...): explicit `paladin_core::platform::container::<file>::Item` target, found by searching `crates/paladin-core/src`.
  - `[`Duration`]` (two sites): `std::time::Duration`.
  - `paladin_ports::output::<module>` links inside the crate itself (seven sites): label kept, target `crate::output::<module>`.
  - Higher-layer targets became plain code spans: `paladin_battalion::*` (five sites), `[`TraceDispatcher`](paladin_battalion)`, and the four "Application Ports" and five "Infrastructure Adapters" list items now read ``Application Ports (`crate::application::ports`)``.
  - The hyphenated `[`paladin-core`]` in `lib.rs` lost its brackets.
  - ``[`Self::append_version`]...`` in `assistant_admin_port.rs` became a code span (the method does not exist; the sentence already says so).
- 17 redundant: each explicit `(paladin_core::...)` target removed so the shorthand ``[`Name`]`` remains, using the span and `help: remove explicit link target` rustdoc printed. Every shorthand was re-checked in the same run under both bars; none re-surfaced as unresolved.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale link name `VisionLlmPort`**
- **Found during:** Task 2 (`output/llm_port.rs:212`)
- **Issue:** the link target `crate::output::vision_llm_port::VisionLlmPort` points at a trait that does not exist; the module defines `VisionCapableLlm`.
- **Fix:** the link now reads ``[`VisionCapableLlm`](crate::output::vision_llm_port::VisionCapableLlm)``. The trailing description is unchanged. This corrects a name, so no doc sentence was deleted.
- **Commit:** `309774be8`

**2. [Rule 1 - Documentation accuracy] Plain-comment edit beside the kept `warn` line**
- **Found during:** Task 2
- **Issue:** the three-line `//` comment above `#![warn(rustdoc::broken_intra_doc_links)]` described the removed downgrade and would be false once the suppressions were gone.
- **Fix:** rewritten to say that nothing in the crate suppresses the lint and that both bars enforce it. This is the one non-`///` / non-`//!` line in the diff besides the two removed attributes, so the plan's literal doc-comment-only `grep` check prints those comment lines; no code, signature or attribute other than the two removals changed.
- **Commit:** `309774be8`

**3. Execution detail (not a deviation of outcome):** to keep Task 1's head free of Task 2's `output/` link fixes (PD-08-2), I fixed everything in one working pass, saved the full result to the scratchpad, restored the `output/` files to `HEAD` (via `git show HEAD:<path>`), applied only the 17 redundant fixes for the first commit, and copied the saved files back for the second. Both heads were built and checked before committing.

**Total deviations:** 2 auto-fixed (both Rule 1). No impact on the public API.

## Known Stubs

None.

## Issues Encountered

- `crates/paladin-ports/src/output/embedding_port.rs` documents `EmbeddingPort` in one physical `///` line that contains literal `\n///` text, so the trait's examples render as one paragraph and are never compiled as doctests. Only its one unresolved link was given a target. Out of scope (re-flowing would activate never-run doctests); recorded in `deferred-items.md` under "From plan 55-08".
- Disk was not a problem (about 5.3 GB free throughout); no `target/` cleanup was needed. All cargo runs used `CARGO_INCREMENTAL=0` and `--offline`.

## Threat Flags

None. Rustdoc comment text only; T-55-19 is mitigated: no rustdoc suppression exists at any level, no doc text was deleted, and both workspace bars are green on the removal commit.

## Next Phase Readiness

- Plan 55-10 (`paladin-llm`, `paladin-storage`, `paladin-notifications`) and plan 55-11 (facade, ADR-0033 amendment) can reuse the fix patterns above. The ADR amendment's `paladin-ports` row is: 187 -> 0 under each bar (170 unresolved, 17 redundant).

## Self-Check: PASSED

- Commits `d08cb6e30` and `309774be8` exist on `claude/laughing-dirac-e0h2ax`.
- `crates/paladin-ports/src/lib.rs` has no rustdoc `allow` and exactly one `warn(rustdoc::broken_intra_doc_links)` line.
- `deferred-items.md` carries the plan 55-08 entry.
