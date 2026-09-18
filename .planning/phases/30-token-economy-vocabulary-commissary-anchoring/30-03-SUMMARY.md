---
phase: 30-token-economy-vocabulary-commissary-anchoring
plan: 03
subsystem: docs
tags: [rustdoc, configuration-guide, vocabulary, cost-estimate, quartermaster-purge]

# Dependency graph
requires:
  - phase: 30-token-economy-vocabulary-commissary-anchoring
    provides: "ADR-0049 (Commissary design), ADR-0050 (Treasurer reservation, Milestone 14 / FUT-08, symbol-scoped 0/0 invariant with the herald.rs successor-state note already anticipated)"
provides:
  - "docs/src/getting-started/configuration.md: one new Token Budget Terminology section disambiguating the four independent meanings of max_tokens (Garrison store cap, RAG injection cap, per-request completion cap, run-level budget cap) plus the reserved allowance key for any future spend-governance cap"
  - "crates/paladin-core/src/platform/container/herald.rs: all five cost_estimate rustdoc sites (field, doc-comment bullet, total_cost() accessor, builder method, doc-test example comment) marked reserved for the Treasurer (Milestone 14 / FUT-08), agreeing with ADR-0050; the previously-false total_cost() claim about a fallback estimate corrected"
  - "src/lib.rs: Commissary re-export provenance comment reworded to drop the retired Quartermaster/Convoy/apportion vocabulary while keeping the D-01/D-02 citation and adding a pointer to ADR-0049"
  - ".project/project-management/paladin-project-plan-final.md: historical annotation on the SirQuartermaster example name, citing ADR-0049"
  - "grep -rniE '\\bQuartermaster\\b' crates src now returns nothing (exit 1)"
affects: [Milestone_14-Treasurer, 31-lossless-token-accounting]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reservation-agreement rustdoc: five doc sites on one field all carry the identical marker phrase (Milestone 14 / FUT-08) so the reservation is greppable and idempotently re-appliable, matching the successor state ADR-0050 already named up front."
    - "Per-commit phase-scope guard: Task 3 computes the phase base as the parent of the oldest docs(30): commit and diffs *.rs across the whole phase, catching scope creep from a documentation phase into code at the mechanical level rather than trusting task-local diffs alone."

key-files:
  created: []
  modified:
    - docs/src/getting-started/configuration.md
    - crates/paladin-core/src/platform/container/herald.rs
    - src/lib.rs
    - .project/project-management/paladin-project-plan-final.md

key-decisions:
  - "The new table's markdown separator row was written as `| --- | --- | --- |` (space after each leading pipe) rather than the PATTERNS.md example's `|---|---|---|`, because the task's own acceptance criterion requires exactly 6 lines beginning with the literal string `| ` (pipe-space) in the new section — a separator without the space would only produce 5 matching lines. Table renders identically either way; this is a grep-contract fix, not a content change."
  - "total_cost()'s accessor doc previously claimed it 'calculates a basic estimate based on token usage' when it does no such thing (it returns the stored field verbatim) — Task 2 replaced the false claim with an accurate one per the task's explicit instruction and threat T-30-09, rather than leaving a false statement next to a newly-added reservation note."
  - "The .project/project-management/paladin-project-plan-final.md annotation was placed as an italicized prose line immediately after the closing yaml fence (not as an HTML comment inside the fence), because the document uses no HTML comments anywhere else and a prose annotation reads naturally outside the code block while staying within the required 15-line window of the SirQuartermaster occurrence."

requirements-completed: [VOCAB-05, VOCAB-06]

coverage:
  - id: D1
    description: "configuration.md carries one Token Budget Terminology section with exactly four rows naming the four real owners (garrison.max_tokens, rag.max_tokens, the two per-request surfaces, agent_runtime.token_budget.max_tokens) plus the one allowance sentence; no pre-existing line changed; the book builds."
    requirement: "VOCAB-05"
    verification:
      - kind: other
        ref: "Task 1's own <verify> automated command (heading count, 6-row region check, four owner strings, Garrison store cap + allowance strings, zero pre-existing lines removed, mdbook build docs/) — re-run at Self-Check time"
        status: pass
    human_judgment: false
  - id: D2
    description: "All five cost_estimate doc sites in herald.rs say the field is reserved for the Treasurer (Milestone 14 / FUT-08) with no in-tree producer; field/accessor/builder signatures byte-identical; diff is /// lines only; cargo doc -p paladin-ai-core --no-deps and cargo fmt --check both exit 0."
    requirement: "VOCAB-05"
    verification:
      - kind: other
        ref: "Task 2's own <verify> automated command (grep -c 5, all-/// check, no in-tree producer string, no Epic 5 string, three signature greps, example-comment grep, comment-only diff check, cargo doc, cargo fmt --check) — re-run at Self-Check time"
        status: pass
      - kind: other
        ref: "cargo test --doc -p paladin-ai-core: platform::container::herald::ExecutionMetadata (line 454) ... ok — the edited doc-test example still compiles and runs"
        status: pass
    human_judgment: false
  - id: D3
    description: "grep -rniE '\\bQuartermaster\\b' crates src returns nothing; src/lib.rs's provenance comment still explains the Commissary export and its diff is comment-only; the plan-final example is annotated as historical with a citation to ADR-0049 and is not deleted; across all of Phase 30 exactly two .rs files changed, both comment-only."
    requirement: "VOCAB-06"
    verification:
      - kind: other
        ref: "Task 3's own <verify> automated command (Quartermaster grep exit 1, 8-line provenance window, comment-only diff on src/lib.rs, SirQuartermaster still present with ADR-0049 citation within 15 lines, phase-wide *.rs scope = exactly herald.rs + lib.rs, zero Cargo.toml/Cargo.lock/MIGRATION.md changes in the phase, cargo fmt --check, cargo check --workspace) — re-run at Self-Check time"
        status: pass
    human_judgment: false

# Metrics
duration: 24min
completed: 2026-09-14
status: complete
---

# Phase 30 Plan 03: Vocabulary Disambiguation, Treasurer Reservation & Quartermaster Purge Summary

**Disambiguates the four meanings of `max_tokens` in one configuration-guide table, marks
`ExecutionMetadata.cost_estimate` reserved for the Treasurer at all five rustdoc sites in
`herald.rs`, and retires the last two `Quartermaster` prose references from `src/lib.rs` and the
project-plan example — a doc-comment-only change with no field, signature, or config key
modified.**

## Performance

- **Duration:** 24 min
- **Started:** 2026-09-14T19:08:00Z (approx.)
- **Completed:** 2026-09-14T19:17:21Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- `docs/src/getting-started/configuration.md` gained a `## Token Budget Terminology` section
  (placed between `## Herald (Output Formatting)` and `## Autonomous Features`) with exactly
  four rows naming the four independent `max_tokens` senses and their real config-key owners,
  plus one sentence reserving `allowance` as the distinct future spend-governance key. No
  pre-existing line in the file was touched (verified by `git diff` showing zero removed lines).
- `crates/paladin-core/src/platform/container/herald.rs`'s five `cost_estimate` rustdoc sites
  (field doc, `ExecutionMetadata` field-list bullet, `total_cost()` accessor doc, builder-method
  doc, doc-test example trailing comment) now all carry the identical `Milestone 14 / FUT-08`
  marker, agreeing with ADR-0050's reservation. The `total_cost()` doc's previously-inaccurate
  claim ("calculates a basic estimate") was corrected to state the field is returned as stored.
  No field, accessor, or builder signature changed; the doc-test example at line 454 still
  compiles and passes.
- `src/lib.rs`'s Commissary re-export provenance comment was reworded to drop the retired
  `Quartermaster`/`Convoy`/`apportion` vocabulary while keeping the `D-01`/`D-02` citation and
  adding a pointer to `.planning/decisions/0049-commissary-design-and-rename.md`.
- `.project/project-management/paladin-project-plan-final.md` gained a one-line historical
  annotation on the `SirQuartermaster` example, citing ADR-0049; the example itself is
  unmodified and undeleted.
- `grep -rniE '\bQuartermaster\b' crates src` now returns nothing (exit 1) — the term is fully
  retired from the compiled tree.

## Task Commits

Each task was committed atomically:

1. **Task 1: The four meanings of `max_tokens`, as one table in the configuration guide** -
   `04d6f12d` (docs)
2. **Task 2: Mark `cost_estimate` reserved in rustdoc — field, accessor, builder and example** -
   `2fe2b0d0` (docs)
3. **Task 3: Retire the last two prose references to the superseded budgeting term** -
   `5f45927a` (docs)

## Files Created/Modified
- `docs/src/getting-started/configuration.md` - New `## Token Budget Terminology` section (4 rows + `allowance` sentence)
- `crates/paladin-core/src/platform/container/herald.rs` - Rustdoc text only at five `cost_estimate` doc sites
- `src/lib.rs` - Commissary re-export provenance comment reworded (comment-only)
- `.project/project-management/paladin-project-plan-final.md` - One historical annotation near the `SirQuartermaster` example

## Decisions Made
- Wrote the new table's separator row as `| --- | --- | --- |` (space after each pipe) rather
  than the PATTERNS.md example's `|---|---|---|`, so the region contains exactly 6 lines
  matching the literal `^| ` grep the acceptance criterion checks — a purely mechanical fix, the
  rendered table is identical either way.
- Corrected `total_cost()`'s inaccurate doc claim (it does not "calculate a basic estimate";
  it returns the stored field) per the task's explicit instruction and threat T-30-09, rather
  than leaving a false statement beside the newly-added reservation note.
- Placed the `paladin-project-plan-final.md` annotation as an italicized prose line after the
  closing yaml fence rather than an HTML comment inside it, matching the document's existing
  style (no HTML comments used anywhere else in that file) while staying within the required
  15-line window of the `SirQuartermaster` occurrence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Package-name substitution: `paladin-core` → `paladin-ai-core`**
- **Found during:** Task 2 (rustdoc verification)
- **Issue:** The crate at `crates/paladin-core/` is published as package `paladin-ai-core`; the
  plan's `<verify>` commands read `cargo doc -p paladin-core --no-deps`. This is a known,
  pre-documented repo trap (see `repo_operating_rules` in the executor prompt), not a genuine
  package-legitimacy question, so it did not route to the package-install checkpoint gate.
- **Fix:** Substituted `-p paladin-ai-core` for `-p paladin-core` in every `cargo doc` and
  `cargo clippy` invocation for this plan.
- **Files modified:** None (command-line substitution only, no file changed as a result)
- **Verification:** `cargo doc -p paladin-ai-core --no-deps` exits 0 with no warning mentioning
  `herald.rs`; `cargo clippy -p paladin-ai-core --all-targets -- -D warnings` exits 0.
- **Committed in:** N/A (verification-only substitution, no code artifact)

---

**Total deviations:** 1 auto-fixed (1 blocking, package-name substitution — pre-documented repo
trap, not a genuine package-legitimacy question).
**Impact on plan:** No scope or content impact; purely a command-line correction needed to run
the plan's own verification commands against this repo's actual package name.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- VOCAB-05 and VOCAB-06 are both satisfied: the four `max_tokens` meanings are documented in one
  table, all five `cost_estimate` rustdoc sites agree with ADR-0050's Treasurer reservation, and
  the retired `Quartermaster` term is gone from `crates` and `src`.
- Phase-level scope guard (`git diff --name-only <phase-base>..HEAD -- '*.rs'`, phase base =
  `27e457aa6b1c4e98b64bff6d456af398227ee457`, the parent of the oldest `docs(30):` commit
  `d843bc8e`) lists exactly `crates/paladin-core/src/platform/container/herald.rs` and
  `src/lib.rs` — matching the plan's own exit condition. Both diffs are comment-only.
- **Note on the phase-base diff and `.planning/phases/`:** the same base-to-HEAD diff also shows
  three pre-existing `.gitkeep` stub files under `.planning/phases/31-lossless-token-accounting/`,
  `32-unified-token-primitives/`, and `33-commissary-in-tree-adoption/`. These were added by an
  earlier, already-merged Phase 30 planning commit (`f22661e3`, "mark Phase 30 planned — wave
  annotations...") that pre-dates this plan's execution and this worktree entirely — they are
  Phase 30's own roadmap-scaffolding for the three phases it unblocks, not a rewrite of another
  phase's history (the prohibition this task's acceptance criterion guards against, per Milestone
  13 overview §5.4). This plan's own commits (`04d6f12d`, `2fe2b0d0`, `5f45927a`) touch none of
  those three phase directories — verified separately below.
- No blockers. All three commits touch no `Cargo.toml`/`Cargo.lock`/`MIGRATION.md` path anywhere
  in the phase.
- Phase 30 (all three plans) is now complete; ready for orchestrator wave-merge and the
  phase-close step.

## Self-Check: PASSED

Files verified to exist on disk:
- FOUND: `docs/src/getting-started/configuration.md` (modified)
- FOUND: `crates/paladin-core/src/platform/container/herald.rs` (modified)
- FOUND: `src/lib.rs` (modified)
- FOUND: `.project/project-management/paladin-project-plan-final.md` (modified)

Commits verified in `git log --oneline`:
- FOUND: `04d6f12d` — `docs(30-03): name the four meanings of max_tokens in the configuration guide`
- FOUND: `2fe2b0d0` — `docs(30-03): mark ExecutionMetadata.cost_estimate reserved for the Treasurer`
- FOUND: `5f45927a` — `docs(30-03): retire the last Quartermaster references from the compiled tree`

Verification commands re-run at Self-Check time:
- Task 1: heading count = 1; region row count = 6; all four owner strings present; `Garrison
  store cap` and `allowance` present; zero pre-existing lines removed; `mdbook build docs/`
  exit 0 (linkcheck: "No broken links found").
- Task 2: `grep -c 'Milestone 14 / FUT-08' herald.rs` = 5, all on `///` lines; `no in-tree
  producer` present; `Epic 5` absent; all three signatures (`pub cost_estimate: Option<f64>`,
  `pub fn total_cost(&self) -> Option<f64>`, `pub fn cost_estimate(mut self, cost_estimate: f64)
  -> Self`) unchanged; example line `.cost_estimate(0.045)` intact; diff is `///`-only;
  `cargo doc -p paladin-ai-core --no-deps` exit 0 (no new warning for `herald.rs`); `cargo fmt
  --all -- --check` exit 0; `cargo clippy -p paladin-ai-core --all-targets -- -D warnings` exit
  0; `cargo test --doc -p paladin-ai-core` — 91 passed, 0 failed (including the edited example
  at line 454).
- Task 3: `grep -rniE '\bQuartermaster\b' crates src` exit 1 (no match); 8-line provenance
  window above the `pub use` line contains a `Commissary`-mentioning `//` line; `src/lib.rs`
  diff is comment-only; `SirQuartermaster` still present with the ADR-0049 citation within 15
  lines; phase-wide `*.rs` diff (base `27e457aa`..HEAD) = exactly `herald.rs` + `lib.rs`; zero
  `Cargo.toml`/`Cargo.lock`/`MIGRATION.md` changes in the phase; `cargo fmt --all -- --check`
  exit 0; `cargo check --workspace` exit 0 (2m10s, background-completed).
- Plan-level: `mdbook build docs/` re-run after all three commits — exit 0, "No broken links
  found".

---
*Phase: 30-token-economy-vocabulary-commissary-anchoring*
*Completed: 2026-09-14*
