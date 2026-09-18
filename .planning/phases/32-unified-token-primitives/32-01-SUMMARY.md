---
phase: 32-unified-token-primitives
plan: 01
subsystem: llm
tags: [rust, token-counting, commissary, hexagonal-architecture, ports-and-adapters, mdbook]

# Dependency graph
requires: []
provides:
  - "TokenCounterPort::is_exact(&self) -> bool with a doc-tested false default"
  - "TiktokenCounter::is_exact() -> true (scoped to the encoding resolved at new(model))"
  - "HeuristicTokenCounter relying on the trait default (no override), proven by test"
  - "Commissary::new/from_port with the caller-supplied exactness argument removed; Stockpile.exact_tally read live from counter.is_exact()"
  - "docs/src/architecture/commissary.md realigned to the four-argument constructor and the port-sourced exactness signal"
affects: [32-02, 32-03, 32-04, 32-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Defaulted trait method for a per-instance capability flag (is_exact), proven false-by-default via a doc test on the port itself"
    - "Configurable-exactness test double (MockCounter { exact: bool }, with an exact() constructor) replacing a constructor-argument seam"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/token_counter_port.rs
    - crates/paladin-memory/src/token_counter/heuristic.rs
    - crates/paladin-memory/src/garrison/token_counter.rs
    - crates/paladin-llm/src/services/commissary.rs
    - docs/src/architecture/commissary.md

key-decisions:
  - "Checkpoint auto-selected proceed-on-recorded-authority (ADR-0051 + the operator's 2026-09-14 clean-break decision) — see Checkpoint Decision below."
  - "MockCounter gained a field-based exact: bool (default false) plus an exact() constructor, rather than a second test-double type, keeping the existing counter() helper's default behavior unchanged."
  - "The two exact_tally tests were renamed to describe reading the port (exact_tally_true_is_read_from_an_exact_injected_port / ..._false_is_read_from_an_approximate_injected_port), and the true-case test gained the ordering assertion (two dispense calls on one Commissary agree)."

patterns-established:
  - "A capability signal owned by the producing adapter (is_exact on TokenCounterPort) rather than duplicated as a caller-supplied constructor argument."

requirements-completed: [PRIM-01, PRIM-02]

coverage:
  - id: D1
    description: "TokenCounterPort::is_exact defaults to false, proven by a doc test on the trait itself (bare AlwaysOne impl asserting the inherited answer)."
    requirement: "PRIM-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ports --doc token_counter_port"
        status: pass
    human_judgment: false
  - id: D2
    description: "TiktokenCounter::is_exact() returns true unconditionally, scoped to the encoding resolved at new(model); HeuristicTokenCounter inherits the false default with no override written."
    requirement: "PRIM-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-memory --features content-processing --lib is_exact (garrison::token_counter::tests::tiktoken_counter_is_exact, token_counter::heuristic::tests::heuristic_is_exact_reports_false_through_the_trait_default)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Commissary::new and Commissary::from_port drop the caller-supplied exactness argument; the struct holds no exactness field; Stockpile.exact_tally is read live from self.counter.is_exact() where the stockpile is built; Debug prints counter.name() and the live is_exact() answer; all ten in-tree construction sites migrated; no forwarding constructor, deprecated alias, or compatibility shim added."
    requirement: "PRIM-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary (19 tests, 0 failed)"
        status: pass
      - kind: other
        ref: "cargo check --workspace --all-features --all-targets; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --check"
        status: pass
    human_judgment: false
  - id: D4
    description: "docs/src/architecture/commissary.md's usage sketch and 'Honesty about exactness' section rewritten against the new four-argument constructor and the port-sourced exactness signal; the mirrored line-range reference updated to the real post-task-1 location."
    requirement: "PRIM-02"
    verification:
      - kind: other
        ref: "mdbook build docs/ (exit 0, 'No broken links found')"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 01: Unified Counting Contract (Exactness on the Port) Summary

**`TokenCounterPort` gains a doc-tested, defaulted `is_exact` method; `TiktokenCounter` overrides it true and `HeuristicTokenCounter` inherits the false default; `Commissary::new`/`from_port` drop their caller-supplied exactness argument entirely and read `Stockpile.exact_tally` live from the injected port.**

## Performance

- **Duration:** ~19 min
- **Started:** 2026-09-15T15:11:10Z (base commit)
- **Completed:** 2026-09-15T15:30:14Z
- **Tasks:** 2 (plus 1 checkpoint:decision, auto-resolved)
- **Files modified:** 5

## Checkpoint Decision

**Checkpoint: confirm the two one-way doors this phase walks through** — auto-mode was active
(`workflow._auto_chain_active: true`), the checkpoint's gate is `blocking` (not `blocking-human`),
so per the checkpoint protocol the executor auto-selected:

**Selected option:** `proceed-on-recorded-authority` — land the Phase 32 published-API breaks
exactly as `32-CONTEXT.md` locks them: `Commissary::new`/`Commissary::from_port` permanently lose
their caller-supplied exactness parameter (D-08), and the legacy fallible counting trait plus its
factory are deleted outright in plan 32-03 with no deprecated retention period (D-09).

**Authority cited:** `.planning/decisions/0051-token-economy-versioning-x03-supersession.md`
(ADR-0051 — X-03 superseded for Phases 31-33) plus the operator's 2026-09-14 clean-break decision
recorded therein (X-03 requires deprecation shims; ADR-0051 supersedes that requirement for this
phase's named breaks, including `Commissary::new`'s `is_exact_counter` argument).

`⚡ Auto-selected: proceed-on-recorded-authority` was logged before Task 1 began; no file in this
plan's `files_modified` was touched before the selection was recorded.

## Accomplishments
- `TokenCounterPort` gained `fn is_exact(&self) -> bool { false }`, a `# Contract` bullet, and an
  extended doc test proving the default on a bare `AlwaysOne` impl (D-06).
- `TiktokenCounter` overrides `is_exact` to `true`, rustdoc-scoped to the encoding resolved at
  `TiktokenCounter::new(model)`; `HeuristicTokenCounter` writes no override and relies on the
  trait default — each adapter's answer is proven by a dedicated unit test (D-07).
- `Commissary` lost its private `is_exact_counter` field; `Commissary::new` dropped its fourth
  positional argument and `Commissary::from_port` its third; `Stockpile.exact_tally` is now read
  live from `self.counter.is_exact()` where the stockpile is built; `Debug` now prints
  `counter.name()` and the live exactness answer in place of the removed field (D-08).
- All ten in-tree `Commissary::new`/`from_port` construction sites in `commissary.rs`'s test
  module migrated to the new signatures; `MockCounter` gained a configurable `exact: bool` field
  (default `false`, an `exact()` constructor) so both `exact_tally` directions stay covered; the
  two `exact_tally` tests were renamed to describe reading the port, and the true-case test gained
  an ordering assertion (two `dispense` calls on one `Commissary` agree).
- The module doc's Honesty clause, `new`'s constructor rustdoc, and `Stockpile.exact_tally`'s field
  rustdoc were rewritten to describe the port-sourced signal instead of a caller-supplied flag.
- `docs/src/architecture/commissary.md`'s usage sketch and "Honesty about exactness" section
  rewritten against the new four-argument constructor and the port-sourced exactness signal; the
  mirrored line-range reference updated from the stale `631-686` to the real post-task-1
  `644-698`.

## Task Commits

1. **Checkpoint: confirm the two one-way doors this phase walks through** — auto-selected
   `proceed-on-recorded-authority`; no commit (decision record only).
2. **Task 1: the exactness signal, end to end — port to both adapters to Stockpile** (tracer) -
   `2b40e383` (feat)
3. **Task 2: realign the mdBook Commissary page with the new constructor and the new exactness
   source** - `009b2b82` (docs)

**Plan metadata:** committed as part of this SUMMARY/state commit (see below).

_Note: Task 1 is a `type="tracer"` task — committed once, real implementation and real
`<verify>`, then its own `<verify>` was re-run end-to-end as the tracer feedback gate
(autonomous run: `⚡ Tracer verified end-to-end — expanding`) before Task 2 began._

## Files Created/Modified
- `crates/paladin-ports/src/output/token_counter_port.rs` - added `is_exact(&self) -> bool { false }` with `# Contract` bullet and extended doc test
- `crates/paladin-memory/src/token_counter/heuristic.rs` - added the trait-default assertion test (no override written)
- `crates/paladin-memory/src/garrison/token_counter.rs` - added `is_exact() -> true` override to `impl TokenCounterPort for TiktokenCounter`, plus its unit test
- `crates/paladin-llm/src/services/commissary.rs` - removed `is_exact_counter` field/arguments, read `exact_tally` from the port, rewrote Honesty clause/rustdoc, migrated ten construction sites, gave `MockCounter` configurable exactness
- `docs/src/architecture/commissary.md` - usage sketch and "Honesty about exactness" section rewritten against the new signature

## Decisions Made
- See **Checkpoint Decision** above (the plan's single consolidated one-way-door confirmation, covering both this plan's `Commissary` constructor break and plan 32-03's later legacy-pair removal).
- `MockCounter` (in `commissary.rs`'s test module) gained a field-based `exact: bool` (default `false`) plus an `exact()` constructor rather than a second test-double type — the existing `counter()` helper's behavior is unchanged for every pre-existing call site, and only the two renamed `exact_tally` tests use `exact_counter()`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Regenerated missing mdbook-mermaid gitignored assets before `mdbook build`**
- **Found during:** Task 2 verification
- **Issue:** `mdbook build docs/` failed with `Unable to copy /workspace/.../docs/mermaid.min.js` — a fresh worktree has no `target/`-style generated asset directory for `mdbook-mermaid`; `docs/mermaid.min.js` and `docs/mermaid-init.js` are gitignored (`.gitignore:20-22`, "re-generated at build time via mdbook-mermaid install") and were simply absent in this worktree.
- **Fix:** Ran `mdbook-mermaid install docs/` (a documented, idempotent local asset-regeneration command, not a package install — the tool itself was already present at `/usr/local/cargo/bin/mdbook-mermaid`), which wrote the two gitignored files back.
- **Files modified:** none tracked (both files are gitignored, confirmed via `git status --short docs/` showing only the intended `commissary.md` change both before and after).
- **Verification:** `mdbook build docs/` then exits 0 with `No broken links found`.
- **Committed in:** n/a (gitignored files, not committed)

**Total deviations:** 1 auto-fixed (1 blocking environment-setup fix). No scope creep — the fix regenerates a documented, gitignored build artifact required by every fresh checkout/worktree, not a code or content change.

## Issues Encountered

Two of the plan's own written expectations did not match the actual pre-refactor baseline; both
are noted here as authored-text discrepancies rather than code defects, since the underlying
functional requirements (D-08, "no test deleted", "no forwarding constructor") are all satisfied
and verified:

- **Task 1 acceptance criteria says `cargo test -p paladin-llm --lib commissary` should report "at
  least 20 tests passing".** The pre-refactor file already had exactly 19 `#[test]` functions
  (confirmed via `git show HEAD~2:crates/paladin-llm/src/services/commissary.rs | grep -c
  '#\[test\]'`); this plan renames two existing tests and adds assertions to them, but adds no new
  `#[test]` function, so the count is unchanged at 19 — matching the "no test was deleted to dodge
  the migration" requirement exactly (10 construction sites, 0 failures). The plan's own `<verify>`
  automated command (`grep -qE 'test result: ok\. [1-9][0-9]* passed'`) only requires at least one
  passing test and is satisfied.
- **Both the frontmatter `must_haves.truths` and Task 2's acceptance criteria describe the
  post-refactor `Commissary::new` as taking "five arguments" (`from_port` "four").** The actual,
  verified, compiling signature is `Commissary::new(provider, capabilities, counter, config)` — 4
  arguments — and `Commissary::from_port(llm, counter, config)` — 3 arguments — which is the
  direct, minimal consequence of D-08's own instruction ("`new` and `from_port` both lose the
  argument"): pre-refactor `new` had 5 params (including `is_exact_counter`) and pre-refactor
  `from_port` had 4; removing exactly one param from each yields 4 and 3, not 5 and 4. The mdBook
  construction sketch was written to match the real, tested 4-argument signature rather than the
  plan text's apparent miscount, and the line-range reference was updated to the real post-task-1
  location as instructed.

Neither discrepancy blocked any task or required a design decision — both are plan-authoring
numeric mismatches against a baseline that was itself unambiguous (the pre-existing file on disk),
and the code that shipped is the one D-06/D-07/D-08 actually describe in prose.

## Known Stubs

None — no stub patterns (hardcoded empty values feeding UI, placeholder text, unwired data
sources) apply to this plan's scope (library-internal counting contract and constructor
signature, no UI).

## Threat Flags

None — this plan's threat model (T-32-01 through T-32-04, T-32-SC) covers exactly the surface this
plan touches, and no new surface outside that register was introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PRIM-01 is fully done: the exactness signal flows port → both adapters → `Stockpile.exact_tally`
  with exactly one source of truth, proven by a doc test on the port and a unit test per adapter.
- PRIM-02's code half is done: `Commissary::new`/`from_port` no longer accept an exactness flag,
  hold no exactness field, and every in-tree construction site compiles, formats, and lints clean.
- Plan 32-02 (the shared window resolver, PRIM-04) and plan 32-03 (legacy counter retirement,
  PRIM-03) can proceed; the consolidated checkpoint in this plan already covers plan 32-03's D-09
  legacy-pair removal, so plan 32-03 needs no checkpoint of its own.
- No blockers. The `mdbook-mermaid install docs/` step (undocumented as a prerequisite in this
  worktree) should be noted for any future fresh-worktree `mdbook build` — it is gitignored and
  regenerates instantly, so it is not a structural blocker, only a one-time local step.

## Self-Check: PASSED

All files listed under Files Created/Modified confirmed present on disk; all three commit hashes
(`2b40e383`, `009b2b82`, `01d7f5fe`) confirmed in `git log --oneline -5`.

---
*Phase: 32-unified-token-primitives*
*Completed: 2026-09-15*
