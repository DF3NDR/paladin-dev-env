---
phase: 32-unified-token-primitives
plan: 03
subsystem: infra
tags: [rust, tokenization, tiktoken, hexagonal-architecture, api-surface-removal]

# Dependency graph
requires:
  - phase: 32-unified-token-primitives (plan 32-01)
    provides: "TokenCounterPort::is_exact, TiktokenCounter::is_exact() -> true override"
provides:
  - "TokenCounterPort is the only counting contract in paladin-memory -- the legacy fallible TokenCounter trait and TokenCounterFactory no longer exist"
  - "TiktokenCounter::count inlines the BPE lookup and per-string cache directly, with no fallible round-trip"
affects: [32-unified-token-primitives (plan 32-05, clean-baseline exit grep and MIGRATION.md/CHANGELOG closure)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Adapter's only counting path is its own TokenCounterPort impl -- no parallel fallible trait surviving alongside the port"

key-files:
  created: []
  modified:
    - crates/paladin-memory/src/garrison/token_counter.rs
    - crates/paladin-memory/src/garrison/mod.rs
    - crates/paladin-memory/src/prelude.rs
    - crates/paladin-memory/src/lib.rs
    - src/infrastructure/adapters/garrison/mod.rs
    - docs/src/architecture/crate-map.md
    - docs/src/user-guides/memory-management.md

key-decisions:
  - "D-09/D-10/D-11/D-12 (32-CONTEXT.md) executed as written: legacy TokenCounter trait, its impl, and TokenCounterFactory deleted outright with no #[deprecated] shim; TiktokenCounter keeps new/model_name/clear_cache/cache_size plus its TokenCounterPort impl with count inlined and infallible"
  - "Task 1 additionally narrowed crates/paladin-memory/src/garrison/mod.rs and prelude.rs (originally scoped to Task 2's <files>) because Task 1's own <verify> compiles the whole paladin-memory crate under --features content-processing, and those two re-export lines would not resolve otherwise -- documented as a Rule 3 (blocking-issue) deviation pulled forward, not a scope change to D-11's intent"

patterns-established:
  - "When a plan's per-task <verify> requires whole-crate compilation but the task's own <files> list only covers part of a multi-site rename/removal, narrow the minimum set of same-crate re-export sites needed to satisfy that specific task's verify, and leave cross-crate/doc sites to the task the plan already assigned them to"

requirements-completed: [PRIM-03]

coverage:
  - id: D1
    description: "Legacy TokenCounter trait, its impl for TiktokenCounter, and TokenCounterFactory (for_model/supported_models/is_supported) deleted outright from token_counter.rs, no #[deprecated] retention"
    requirement: "PRIM-03"
    verification:
      - kind: unit
        ref: "git grep -qnE '^pub trait |^pub struct TokenCounterFactory' -- crates/paladin-memory/src/garrison/token_counter.rs (exit 1, negated in acceptance criteria) and git grep -qn 'deprecated' (exit 1)"
        status: pass
    human_judgment: false
  - id: D2
    description: "TiktokenCounter's only counting path is its TokenCounterPort impl -- count inlines the BPE lookup and per-string cache directly, no fallible count_tokens survives, is_exact/model_name/new/clear_cache/cache_size all present"
    requirement: "PRIM-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-memory --features content-processing --lib token_counter (19 passed, 0 failed)"
        status: pass
      - kind: unit
        ref: "git grep -c encode_with_special_tokens -- crates/paladin-memory/src/garrison/token_counter.rs == 1, inside the TokenCounterPort impl"
        status: pass
    human_judgment: false
  - id: D3
    description: "All four re-export sites (paladin-memory garrison/mod.rs, prelude.rs, facade garrison/mod.rs top-level and its token_counter sub-module) narrowed to TiktokenCounter only, each unchanged content-processing cfg gate intact"
    requirement: "PRIM-03"
    verification:
      - kind: unit
        ref: "git grep -c 'content-processing' -- crates/paladin-memory/src/garrison/mod.rs crates/paladin-memory/src/prelude.rs src/infrastructure/adapters/garrison/mod.rs (2/1/2, unchanged from pre-task)"
        status: pass
      - kind: integration
        ref: "cargo check --workspace --all-features --all-targets (exit 0) and cargo check --workspace --all-targets, default features (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Doc sweep: paladin-memory lib.rs narrative/feature table, crate-map.md feature table, memory-management.md token-counting comment all name only TiktokenCounter/TokenCounterPort, not the removed pair"
    requirement: "PRIM-03"
    verification:
      - kind: other
        ref: "git grep -nE '\\bTokenCounterFactory\\b|garrison::TokenCounter\\b|is_exact_counter' -- crates src docs/src examples benches tests (exit 1, nothing found)"
        status: pass
      - kind: other
        ref: "mdbook build docs/ (exit 0, 'No broken links found')"
        status: pass
    human_judgment: false
  - id: D5
    description: "Counting behaviour unchanged after the removal: cache hit/clear/empty-string/unicode/multi-model coverage preserved in the rewritten port-facing tests"
    requirement: "PRIM-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-memory --features content-processing --lib (120 passed, 0 failed)"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 03: Retire the legacy TokenCounter trait and factory Summary

**Deleted the duplicate fallible `TokenCounter` trait and `TokenCounterFactory` from `paladin-memory`, inlining the BPE lookup and per-string cache directly into `TiktokenCounter`'s `TokenCounterPort::count`, so `TokenCounterPort` is now the only counting contract in the workspace.**

## Performance

- **Duration:** ~40 min
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Legacy fallible `TokenCounter` trait, its `impl TokenCounter for TiktokenCounter`, and `TokenCounterFactory` (`for_model`/`supported_models`/`is_supported`) deleted outright from `crates/paladin-memory/src/garrison/token_counter.rs` — no `#[deprecated]` shim, no forwarding alias (D-09)
- `TiktokenCounter`'s `impl TokenCounterPort::count` now inlines the cache read, BPE encode, bounded cache write, and thousand-entry ceiling clear directly — no `Result` round-trip through a removed trait method, behaviourally identical to the pre-removal code (D-10)
- Added an inherent `TiktokenCounter::model_name(&self) -> &str` accessor to replace the removed trait method; kept `new`, `clear_cache`, `cache_size`, and the `is_exact() -> true` override from plan 32-01 unchanged
- All four re-export sites (paladin-memory's `garrison/mod.rs` and `prelude.rs`, the facade's `garrison/mod.rs` top-level re-export and its backward-compatible `token_counter` sub-module) narrowed to `TiktokenCounter` only, every `#[cfg(feature = "content-processing")]` gate left untouched (D-11)
- Doc sweep: `paladin-memory`'s crate narrative and feature table, `docs/src/architecture/crate-map.md`'s feature table, and `docs/src/user-guides/memory-management.md`'s garrison-configuration comment now name only `TiktokenCounter`/`TokenCounterPort` (D-12)
- Exit grep for `TokenCounterFactory` / `garrison::TokenCounter` / `is_exact_counter` across `crates`, `src`, `docs/src`, `examples`, `benches`, `tests` returns nothing

## Task Commits

Each task was committed atomically:

1. **Task 1: delete the legacy counting trait and its factory; inline the counting path into the port impl** - `99f13386` (refactor)
2. **Task 2: narrow the four re-export sites and sweep every doc site naming the removed pair** - `7e18cf07` (refactor)

_Note: Task 1's commit also narrowed the two paladin-memory-internal re-export sites (see Deviations)._

## Files Created/Modified

- `crates/paladin-memory/src/garrison/token_counter.rs` - legacy trait/impl/factory deleted; counting inlined into `impl TokenCounterPort for TiktokenCounter`; inherent `model_name` accessor added; struct rustdoc example and tests rewritten against the port
- `crates/paladin-memory/src/garrison/mod.rs` - re-export narrowed to `TiktokenCounter`
- `crates/paladin-memory/src/prelude.rs` - re-export narrowed to `TiktokenCounter`
- `crates/paladin-memory/src/lib.rs` - Garrison narrative bullet and feature-flag table row narrowed
- `src/infrastructure/adapters/garrison/mod.rs` - top-level re-export and the backward-compatible `token_counter` sub-module narrowed
- `docs/src/architecture/crate-map.md` - `content-processing` feature table row narrowed
- `docs/src/user-guides/memory-management.md` - token-counting comment rewritten to name the port and both adapters
- `.planning/phases/32-unified-token-primitives/deferred-items.md` - new file, logs one pre-existing out-of-scope doc bug (see Issues Encountered)

## Decisions Made

- Executed D-09 through D-12 from `32-CONTEXT.md` as written: outright deletion with no deprecated retention, the exact D-10 surviving surface on `TiktokenCounter`, all four re-export sites narrowed inside unchanged cfg gates, and the four named doc sites swept.
- Task 1's commit narrows `crates/paladin-memory/src/garrison/mod.rs` and `crates/paladin-memory/src/prelude.rs` in addition to `token_counter.rs`, even though the plan's `<files>` tag scoped those two lines to Task 2. Task 1's own `<verify>` runs `cargo test -p paladin-memory --features content-processing --lib`, `cargo doc -p paladin-memory --features content-processing --no-deps`, and `cargo clippy -p paladin-memory ...` — all of which compile the whole `paladin-memory` crate, and both re-export lines would fail to resolve (`unresolved imports`) once the trait/factory were deleted. Narrowing them was the minimum change needed to satisfy Task 1's own verify; Task 2 still independently narrowed the facade's re-export site and did the full doc sweep, and its own acceptance criteria (per-file `content-processing` cfg-gate counts, `TiktokenCounter` presence counts) verify all four sites end in the correct narrowed state regardless of which task did the mechanical edit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Narrowed two re-export sites in Task 1 to unblock its own `<verify>`**
- **Found during:** Task 1 (delete the legacy trait/factory)
- **Issue:** Task 1's `<verify>` compiles the whole `paladin-memory` crate under `--features content-processing`. Deleting `TokenCounter`/`TokenCounterFactory` from `token_counter.rs` without touching `garrison/mod.rs`'s and `prelude.rs`'s `pub use token_counter::{TiktokenCounter, TokenCounter, TokenCounterFactory};` lines produced `error[E0432]: unresolved imports` and blocked `cargo test`/`cargo doc`/`cargo clippy` from completing.
- **Fix:** Narrowed both `pub use` lines to `TiktokenCounter` only, inside their existing `#[cfg(feature = "content-processing")]` gates (the exact D-11 narrowing already planned for Task 2, applied one commit earlier than the plan's task boundary assumed).
- **Files modified:** `crates/paladin-memory/src/garrison/mod.rs`, `crates/paladin-memory/src/prelude.rs`
- **Verification:** `cargo test -p paladin-memory --features content-processing --lib token_counter` (19 passed, 0 failed); `cargo clippy -p paladin-memory --all-targets --features content-processing -- -D warnings` (clean); `cargo fmt --check` (clean)
- **Committed in:** `99f13386` (Task 1 commit)

**2. [Scope Boundary - logged, not fixed] Pre-existing broken intra-doc link in `token_counter/mod.rs`**
- **Found during:** Task 1's `cargo doc -p paladin-memory --features content-processing --no-deps` verify step
- **Issue:** `crates/paladin-memory/src/token_counter/mod.rs:3` has an unresolved `[`HeuristicTokenCounter`]` intra-doc link under `RUSTDOCFLAGS="-D warnings"`, with or without `content-processing`. Confirmed via `git stash` to exist identically on the pre-task base commit (`220fc6cd`), introduced in Phase 26 (`69a56dd6`) — not caused by this plan's changes, and not caught by CI's "Check documentation" job because that job runs default features only.
- **Fix:** Not fixed (out of scope — the file is unrelated to Task 1/2's `<files>`). Logged to `.planning/phases/32-unified-token-primitives/deferred-items.md`.
- **Files modified:** none (documentation only, in `deferred-items.md`)
- **Verification:** Confirmed the doc command's only error names `HeuristicTokenCounter`, a symbol untouched by this plan — zero errors reference `TokenCounter`, `TokenCounterFactory`, or any name this plan removed or renamed.

---

**Total deviations:** 2 (1 auto-fixed under Rule 3, 1 logged out-of-scope per the Scope Boundary rule)
**Impact on plan:** The Rule 3 fix is exactly the D-11 narrowing the plan already scheduled, applied one task early to satisfy Task 1's own verify — no scope creep, no behavior change. The logged item does not block PRIM-03's own success criteria (the exit grep and the test suite both pass); it blocks only the crate-wide `cargo doc -D warnings` invocation via an unrelated, pre-existing symbol.

## Issues Encountered

- `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --features content-processing --no-deps` cannot be made to exit 0 without editing `crates/paladin-memory/src/token_counter/mod.rs`, a file outside this plan's `<files_modified>` list, because of the pre-existing broken link described above. All other pieces of this plan's `<verification>` block pass cleanly: `cargo test -p paladin-memory --features content-processing --lib` (120 passed), `cargo check --workspace --all-features --all-targets` (exit 0), `cargo check --workspace --all-targets` default features (exit 0), the exit grep (empty), `mdbook build docs/` (no broken links), and `cargo fmt --check` / `cargo clippy --workspace --all-targets --all-features -- -D warnings` (both clean).

## Next Phase Readiness

- `TokenCounterPort` is now provably the only counting contract in the workspace — no parallel fallible trait for a future adapter to implement instead.
- Plan 32-05's clean-baseline exit grep can reuse the same pattern (`TokenCounterFactory` / `garrison::TokenCounter` / `is_exact_counter`) with the migration/upgrading pages excluded, per this plan's `key_links`.
- Carried concern for a future phase (not blocking PRIM-03): fix the pre-existing broken `[`HeuristicTokenCounter`]` intra-doc link in `crates/paladin-memory/src/token_counter/mod.rs:3` so `cargo doc -p paladin-memory --features content-processing --no-deps` can pass under `-D warnings` — see `deferred-items.md`.

---
*Phase: 32-unified-token-primitives*
*Completed: 2026-09-15*

## Self-Check: PASSED

Supplied by the execute-phase orchestrator's post-wave spot-check (the executor omitted the
section): all seven `key-files.modified` paths exist on the merged tree at `62264f05`; the three
task commits (`99f13386`, `7e18cf07`, `6266ad6c`) are present on `feature/phase-30`; the D-12 exit
grep (`grep -rnE '\bTokenCounterFactory\b|garrison::TokenCounter\b|is_exact_counter' crates src
docs/src examples benches tests`) returns nothing; the post-wave `git hook run pre-commit`
(fmt + workspace clippy `-D warnings`) passed.
