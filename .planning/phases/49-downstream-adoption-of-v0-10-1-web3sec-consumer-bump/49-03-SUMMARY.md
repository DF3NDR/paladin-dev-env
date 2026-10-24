---
phase: 49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump
plan: 03
subsystem: testing
tags: [rust, cargo-check, consumer-surface, red-proof, mutation, api-stability]

requires:
  - phase: 49-01
    provides: "tests/consumer_surface_muster.rs and its [[test]] table, the guard under proof"
  - phase: 49-02
    provides: "MIGRATION/CHANGELOG library-consumer note (Wave 1 sibling; ordering only)"
provides:
  - "49-MUSTER-RED-EVIDENCE.md: the proven-red record for the consumer-surface muster (ADOPT-04)"
  - "Experiment 1: bool re-added to Commissary::new fails only the muster (E0308, E0061)"
  - "Experiment 3: ConsignmentItem dropped from the facade re-export fails only the muster (E0432)"
affects: [49-04, 49-05, 49-06, phase-50, phase-51]

tech-stack:
  added: []
  patterns:
    - "Scripted reversible red proof: baseline guard, EXIT-trap git checkout before the first edit, mutation-applied guard, verdict by counts and presence only"
    - "A proof is valid only when exactly one 'could not compile' line names the muster target (no library crate failed first)"

key-files:
  created:
    - .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-MUSTER-RED-EVIDENCE.md
  modified: []

key-decisions:
  - "Both proof scripts live outside the repository (/tmp/49-red-proof/) and are not committed; their full text is recorded in the evidence file so a verifier can recreate them (A2-49-03)."
  - "The widened token-usage builder experiment (research Experiment 2) and the trait-implementer pins were deliberately not run: the first fails the library before the muster, the second is covered by construction."

patterns-established:
  - "Red-proof acceptance is mechanical: non-zero cargo exit, exactly one 'could not compile' line naming the muster, at least one '--> tests/consumer_surface_muster.rs:' location, restoration verified by empty git status/diff"

requirements-completed: [ADOPT-04]

coverage:
  - id: D1
    description: "Experiment 1: a bool re-added to Commissary::new (both library callers patched) fails only the muster"
    requirement: "ADOPT-04"
    verification:
      - kind: other
        ref: "bash /tmp/49-red-proof/red_proof_commissary.sh: RED-PROVEN; cargo check rc=101; one 'could not compile `paladin-ai` (test \"consumer_surface_muster\")' line; E0308 at tests/consumer_surface_muster.rs:111:48, E0061 at :125:13"
        status: pass
    human_judgment: false
  - id: D2
    description: "Experiment 3: dropping ConsignmentItem from the facade re-export fails only the muster"
    requirement: "ADOPT-04"
    verification:
      - kind: other
        ref: "bash /tmp/49-red-proof/red_proof_facade.sh: RED-PROVEN; cargo check rc=101; one 'could not compile' line naming the muster; E0432 unresolved import paladin::ConsignmentItem at tests/consumer_surface_muster.rs:63:5"
        status: pass
    human_judgment: false
  - id: D3
    description: "Tree restored after every experiment; no commit names crates/ or src/"
    requirement: "ADOPT-04"
    verification:
      - kind: other
        ref: "git status --porcelain -- crates src empty after each run; git diff --quiet -- crates src exit 0; git diff --name-only 91be2280 -- crates src empty; (49-03) commits carry 0 crates/src paths; muster 1 passed afterwards"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-14
status: complete
---

# Phase 49 Plan 03: Proven-red consumer-surface muster Summary

**Two scripted, self-restoring mutations (a `bool` re-added to `Commissary::new`, and `ConsignmentItem` dropped from the facade re-export) each fail only the consumer-surface muster, at muster-file locations, while every library crate still compiles; the transcripts are recorded in `49-MUSTER-RED-EVIDENCE.md`.**

## Performance

- **Duration:** about 25 min (dominated by two non-incremental check builds)
- **Tasks:** 2 of 2
- **Files modified:** 1 created (the evidence file), none under `crates/` or `src/`

## Accomplishments

- Experiment 1 (the ROADMAP's own example): `_is_exact_counter: bool` added to `Commissary::new` with `from_port` and the RAG caller patched. `cargo check --offline --test consumer_surface_muster` exited 101 with exactly one `error: could not compile` line, `` `paladin-ai` (test "consumer_surface_muster") due to 2 previous errors ``, and two errors both inside the muster: `E0308 mismatched types` at `tests/consumer_surface_muster.rs:111:48` (the fn-pointer pin) and `E0061 this function takes 5 arguments but 4 arguments were supplied` at `:125:13` (the call pin).
- Experiment 3: `ConsignmentItem` dropped from the `pub use paladin_llm::services::commissary::{..}` group in `src/lib.rs`. The check exited 101 with one `could not compile` line naming the muster target and one error, `E0432 unresolved import paladin::ConsignmentItem` at `tests/consumer_surface_muster.rs:63:5`.
- Both verdicts were produced by counts and presence only (never diagnostic order), behind a baseline guard (`DIRTY-BASELINE`), an `EXIT` trap installed before the first edit, and a mutation-applied guard (`MUTATION-NOT-APPLIED`).
- The evidence file also records why the widened token-usage builder experiment is not a valid proof and why the trait-implementer pins are covered by construction.

## Task Commits

1. **Task 1: Experiment 1, Commissary exactness argument re-added** - `5c3bd7e3` (docs)
2. **Task 2: Experiment 3, facade re-export dropped, evidence file closed** - `495c43a9` (docs)

**Plan metadata:** recorded in the final `docs(49-03): complete muster red-proof plan` commit.

## Restoration Evidence

| Check | Result |
|-------|--------|
| `git status --porcelain -- crates src` after Experiment 1 | empty |
| `git status --porcelain -- crates src` after Experiment 3 | empty |
| `git diff --quiet -- crates src` after each run | exit 0 |
| `CARGO_INCREMENTAL=0 cargo test --offline --test consumer_surface_muster` afterwards | `1 passed; 0 failed` |
| `git diff --name-only 91be2280 -- crates src` (phase base) | empty |
| `(49-03)` commits naming `crates/` or `src/` paths | 0 |

Every cargo invocation ran with `CARGO_INCREMENTAL=0` and `--offline`; `df -h .` showed 5.6 GB free before each experiment and no ENOSPC occurred.

## Files Created/Modified

- `.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-MUSTER-RED-EVIDENCE.md` - the proven-red record: sections `Experiment 1`, `Experiment 3`, `Not used as proof`, `Trait-implementer pins`, `Restoration`, each experiment with the full script text, tree SHA, exit codes, verbatim `could not compile` line, every `error[E....]` header with its muster location, and the empty post-restore status
- Scripts `/tmp/49-red-proof/red_proof_commissary.sh` and `/tmp/49-red-proof/red_proof_facade.sh` (not committed, by design; full text is in the evidence file)

## Decisions Made

- Scripts stay outside the repository (a tracked `.sh` would enter `make lint-shell`, and shellcheck is absent locally).
- The mutation-applied guard of Experiment 1 additionally requires the `false,` argument line in `from_port`, so a partial application of the three substitutions aborts instead of producing a misleading result.

## Deviations from Plan

### Auto-fixed Issues

None - plan executed exactly as written.

### Process notes

- Commit trailers: the orchestrator asked for only the `Claude-Session` trailer, while the harness attribution guidance also asks for `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Both trailers are present on every commit of this plan (the first commit was amended locally, before any push, to add it).
- The scripts were written to `/tmp/49-red-proof/` (the plan's `${TMPDIR:-/tmp}/49-red-proof/`, since `TMPDIR` is unset), as the plan specifies.

---

**Total deviations:** 0 auto-fixed.
**Impact on plan:** none.

## Known Stubs

None. This plan adds an evidence document only.

## Threat Flags

None. No new network endpoint, auth path, file access or schema surface; the temporary mutations were reverted by the EXIT trap and verified by empty `git status` and `git diff` (T-49-04, T-49-07 mitigated).

## Issues Encountered

None.

## Next Phase Readiness

- ROADMAP success criterion 4 (second half) is met: the muster is proven red by two independent breaks of different error classes.
- Plan 49-04 can re-assert `git diff --name-only 91be2280 -- crates src` empty phase-wide; it is empty now.

## Self-Check: PASSED

- FOUND: 49-MUSTER-RED-EVIDENCE.md, 49-03-SUMMARY.md, /tmp/49-red-proof/red_proof_commissary.sh, /tmp/49-red-proof/red_proof_facade.sh
- FOUND commits: 5c3bd7e3, 495c43a9
- Evidence file contains `RED-PROVEN`, the five required headings, and `--> tests/consumer_surface_muster.rs:` locations
