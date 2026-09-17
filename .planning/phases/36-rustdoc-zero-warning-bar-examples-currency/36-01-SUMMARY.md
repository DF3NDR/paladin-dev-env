---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 01
subsystem: docs
tags: [rustdoc, intra-doc-links, examples-gallery, commissary, token-economy]

# Dependency graph
requires:
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec6 (RD-nn/EX-nn work list, stable IDs)
provides:
  - Bare-shorthand intra-doc link technique, empirically validated (a //! module
    doc's link scope resolves to the CRATE ROOT, not the enclosing submodule --
    a same-file `pub use` re-export needs the full `crate::`-relative path)
  - Three rustdoc crates (paladin-memory, paladin-ports, paladin-storage) documenting
    warning-free under both default and --all-features builds
  - examples/token_economy_commissary.rs -- offline Commissary/window-resolution demo
  - examples/README.md "## Token Economy Examples" section
  - 36-EVIDENCE.md + 36-evidence/ -- the evidence harness plans 36-02..36-13 append to
affects: [36-02, 36-03, 36-04, 36-05, 36-06, 36-07, 36-08, 36-09, 36-10, 36-11, 36-12, 36-13]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Private intra-doc link (D-05): de-link to plain code font, reworded toward
      the public entry point -- never widen visibility, never suppress the lint."
    - "Bare-shorthand link to a same-file pub-use re-export (D-06): use an explicit
      markdown link with the FULL crate-relative path, e.g.
      `[`HeuristicTokenCounter`](crate::token_counter::heuristic::HeuristicTokenCounter)`
      -- a bare `[`HeuristicTokenCounter`]` or a `self::`-prefixed path both fail
      because rustdoc resolves a `//!` module doc's link scope against the CRATE
      ROOT, not the enclosing submodule."

key-files:
  created:
    - examples/token_economy_commissary.rs
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-baseline.txt
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-percrate.txt
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-example-run.txt
  modified:
    - crates/paladin-memory/src/token_counter/mod.rs
    - crates/paladin-ports/src/output/structured_executor_port.rs
    - crates/paladin-storage/src/waypoint/contract_tests.rs
    - examples/README.md

key-decisions:
  - "Bare-shorthand link scope resolves to the crate root for //! module docs, not the enclosing submodule -- discovered empirically, not assumed from 36-RESEARCH.md's engine/mod.rs contrast pair"
  - "Fixed the crate-root-scope discovery via full crate::-relative path + markdown display text, not self:: (tried and failed) or bare shorthand (the original defect)"

patterns-established:
  - "One capability cluster -> one named example program -> one README section (D-15), proven end-to-end on the Commissary/window-resolution cluster"

requirements-completed: [CURR-11, CURR-12, CURR-14, CURR-15]

coverage:
  - id: D1
    description: "paladin-memory documents warning-free under default and --all-features builds (RD-01, RD-66, RD-126 closed)"
    requirement: "CURR-11"
    verification:
      - kind: other
        ref: "RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-memory --all-features --no-deps (exit 0); cargo doc -p paladin-memory --no-deps (0 warning: lines)"
        status: pass
    human_judgment: false
  - id: D2
    description: "paladin-ports documents warning-free under default and --all-features builds (RD-51, RD-127 closed)"
    requirement: "CURR-11"
    verification:
      - kind: other
        ref: "RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-ports --all-features --no-deps (exit 0); cargo doc -p paladin-ports --no-deps (0 warning: lines)"
        status: pass
    human_judgment: false
  - id: D3
    description: "paladin-storage documents warning-free under default and --all-features builds (RD-46, RD-128 closed)"
    requirement: "CURR-11"
    verification:
      - kind: other
        ref: "RUSTDOCFLAGS=\"-D warnings\" cargo doc -p paladin-storage --all-features --no-deps (exit 0); cargo doc -p paladin-storage --no-deps (0 warning: lines)"
        status: pass
    human_judgment: false
  - id: D4
    description: "examples/token_economy_commissary.rs -- offline Commissary/window-resolution capability demo (EX-109, EX-111, EX-112, EX-113, EX-114, EX-115)"
    requirement: "CURR-14"
    verification:
      - kind: other
        ref: "cargo build --example token_economy_commissary (exit 0, no --features); cargo run --example token_economy_commissary with OPENAI_API_KEY/ANTHROPIC_API_KEY/DEEPSEEK_API_KEY unset (exit 0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "examples/README.md gains a Token Economy Examples section with matching TOC bullet"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "grep -q '^### \\[token_economy_commissary\\.rs\\]' examples/README.md"
        status: pass
    human_judgment: false
  - id: D6
    description: "36-EVIDENCE.md + 36-evidence/ evidence harness seeded with the baseline and this plan's closure table"
    requirement: "CURR-12"
    verification:
      - kind: other
        ref: ".planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md and 36-evidence/36-01-*.txt exist"
        status: pass
    human_judgment: false

duration: ~50min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 01: Tracer -- Bare-Shorthand Link Technique + Two Follow-on Crates Summary

**Proved the bare-shorthand intra-doc link technique on `paladin-memory`'s
`HeuristicTokenCounter` link (the WINDOWS.md row 37 defect), closed two more
private-link rows in `paladin-ports`/`paladin-storage`, shipped a new offline
`token_economy_commissary` example demonstrating the Commissary prompt-budgeting
cluster, and seeded the phase-wide evidence harness -- six atomic commits, zero
`make api-surface` drift, zero doctest drift (462/0).**

## Performance

- **Duration:** ~50 min
- **Tasks:** 2
- **Files modified:** 9 (3 crate source files, 1 new example, 1 README, 4 new evidence files)

## Accomplishments

- Fixed `crates/paladin-memory/src/token_counter/mod.rs:3`'s bare-shorthand
  `[`HeuristicTokenCounter`]` link. The technique took three attempts to land: a bare
  path-qualified shorthand (`[`heuristic::HeuristicTokenCounter`]`) and a
  `self::`-prefixed path both failed with "no item named `heuristic` in scope" /
  "in module `paladin_memory`" -- revealing that a `//!` module doc's link scope
  resolves against the **crate root**, not the enclosing submodule. The working fix is
  an explicit markdown link with the full crate-relative path and separate display
  text: `` [`HeuristicTokenCounter`](crate::token_counter::heuristic::HeuristicTokenCounter) ``.
  This closes RD-01, RD-66 and RD-126.
- Closed RD-51/RD-127 (`paladin-ports::structured_executor_port::run_structured`'s
  link to the private `repair_prompt`) and RD-46/RD-128
  (`paladin-storage::waypoint::contract_tests::muster_progress_round_trips`'s link to
  the private `muster_progress_fixture`), both via the D-05 de-link-to-plain-code-font
  technique -- no visibility widened, no lint suppressed.
- Shipped `examples/token_economy_commissary.rs`: a fully offline (mock-LLM-backed,
  no provider key needed) demonstration of `Commissary::new`/`dispense`,
  `TokenCounterPort::is_exact` (heuristic vs. exact), `resolve_context_window` with
  its `WindowSource`/`WindowFallbackPolicy`, and an Anthropic-shaped `TokenUsage`
  whose `prompt_tokens` already includes cache-read/cache-write tokens. Closes
  EX-109, EX-111, EX-112, EX-113, EX-114, EX-115.
- Added the `## Token Economy Examples` section (with TOC bullet) to
  `examples/README.md`, matching the house shape and carrying no "Code snippet"
  block per D-21.
- Seeded `36-EVIDENCE.md` and `36-evidence/` (baseline, per-crate sweep,
  example-run captures) for the remaining eleven plans in this phase to append to.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer): rustdoc fix** - `4535ca8b` (docs) -- paladin-memory link fix
2. **Task 1: example program** - `7c222e85` (docs) -- token_economy_commissary.rs
3. **Task 1: README section** - `87808a33` (docs) -- Token Economy Examples section
4. **Task 2: paladin-ports fix** - `20e63d9e` (docs)
5. **Task 2: paladin-storage fix** - `81d033fb` (docs)
6. **Task 1/2: evidence harness** - `f2bdd342` (docs) -- 36-EVIDENCE.md + 36-evidence/

**Plan metadata:** this SUMMARY's own commit (docs: complete plan)

## Files Created/Modified

- `crates/paladin-memory/src/token_counter/mod.rs` - module-doc link fixed to the full crate-relative path
- `crates/paladin-ports/src/output/structured_executor_port.rs` - private-link mention de-linked
- `crates/paladin-storage/src/waypoint/contract_tests.rs` - private-link mention de-linked
- `examples/token_economy_commissary.rs` - new offline example demonstrating the Commissary cluster
- `examples/README.md` - new Token Economy Examples section + TOC bullet
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-EVIDENCE.md` - evidence ledger
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-baseline.txt` - baseline capture
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-percrate.txt` - post-fix per-crate sweep
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-01-example-run.txt` - example run capture

## Closure Table (D-24)

| ID | file:line (cited) | file:line (actual) | kind | fix | commit |
|---|---|---|---|---|---|
| RD-01 | `crates/paladin-memory/src/token_counter/mod.rs:3` | same | unresolved link (bare shorthand to same-file `pub use`) | full crate-relative markdown link | `4535ca8b` |
| RD-66 | same location group | same | default-feature follower | closed by RD-01 fix | `4535ca8b` |
| RD-126 | same location group | same | all-features follower | closed by RD-01 fix | `4535ca8b` |
| RD-51 | `crates/paladin-ports/src/output/structured_executor_port.rs:158` | same | private intra-doc link | de-linked, plain code font | `20e63d9e` |
| RD-127 | same location group | same | all-features follower | closed by RD-51 fix | `20e63d9e` |
| RD-46 | `crates/paladin-storage/src/waypoint/contract_tests.rs:673` | same | private intra-doc link | de-linked, plain code font | `81d033fb` |
| RD-128 | same location group | same | all-features follower | closed by RD-46 fix | `81d033fb` |
| EX-109 | gap row | `examples/token_economy_commissary.rs` | new example | `Commissary::new` post-PRIM-02 signature | `7c222e85` |
| EX-111 | gap row | `examples/token_economy_commissary.rs` | new example | Anthropic-shaped `TokenUsage` via `MockLlmAdapter` | `7c222e85` |
| EX-112 | gap row | `examples/token_economy_commissary.rs` | new example | `TokenCounterPort::is_exact` contrast | `7c222e85` |
| EX-113 | gap row | `examples/token_economy_commissary.rs` | new example | exactness read live from the counter instance | `7c222e85` |
| EX-114 | gap row | `examples/token_economy_commissary.rs` | new example | `resolve_context_window` + `ResolvedWindow` | `7c222e85` |
| EX-115 | gap row | `examples/token_economy_commissary.rs` | new example | `WindowSource` + `WindowFallbackPolicy` | `7c222e85` |

## Decisions Made

- **Bare-shorthand link scope resolves to the crate root, not the enclosing submodule,
  for `//!` module docs.** `36-RESEARCH.md`'s `engine/mod.rs` contrast pair used
  `///` item docs attached to a `pub mod X;` declaration (where `[`graph::WarGraph`]`
  resolves because the doc is attached to the item preceding the submodule
  declaration, in the enclosing module's own scope). `token_counter/mod.rs`'s link is
  in a `//!` INNER doc comment at the top of the file -- rustdoc treats that doc as
  belonging to the crate's documentation tree at the point the module is mounted, and
  resolves bare/`self::`-prefixed paths against the crate root rather than the
  submodule's own item scope. The working fix uses the full `crate::`-relative path
  with an explicit markdown display label. This is new evidence beyond
  `36-RESEARCH.md`'s stated pattern and is the main risk this tracer plan existed to
  retire -- plans 36-02..36-05 should try the full-path form directly for any
  `//!`-doc bare-shorthand row rather than re-deriving this by trial and error.
- Fixed the full-path form with an explicit markdown display label (rather than the
  bare, unreadable full path inline) to keep the rendered prose readable, matching
  `36-RESEARCH.md` D-06's stated pattern (`` [`WarGraph`](crate::engine::WarGraph) ``).

## Deviations from Plan

None - plan executed exactly as written. The three-attempt link-resolution
investigation (bare shorthand -> `self::` -> full `crate::` path) was expected
exploratory work within Task 1's own stated purpose ("the riskiest technical unknown
in this phase is whether a bare-shorthand intra-doc link... can be made to resolve at
all") -- not a deviation from the plan, but the plan's own tracer objective being
fulfilled.

## Issues Encountered

None beyond the link-resolution investigation documented above under Decisions Made.

## User Setup Required

None - no external service configuration required. The new example is fully offline
and reads no environment variable.

## Next Phase Readiness

- The bare-shorthand link technique (full `crate::`-relative path + markdown display
  label) is now empirically proven and documented for plans 36-02 through 36-05,
  which inherit the same pitfall class across `paladin-battalion`'s `engine/mod.rs`
  cluster and others.
- `36-EVIDENCE.md` and `36-evidence/` are seeded and ready for every later plan to
  append its own per-crate sweep and example-run captures to.
- Remaining rustdoc rows (paladin-ai-core, paladin-battalion, paladin-llm,
  paladin-web, paladin-ai facade -- 62 of the original 65 default-feature
  diagnostics) are untouched by this plan and remain the scope of plans 36-02 onward,
  as planned.
- No blockers.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

All 10 created/modified files confirmed present on disk; all 6 task commit hashes
(`4535ca8b`, `7c222e85`, `87808a33`, `20e63d9e`, `81d033fb`, `f2bdd342`) confirmed
present in `git log --oneline --all`.
