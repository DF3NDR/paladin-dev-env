---
phase: 32-unified-token-primitives
plan: 02
subsystem: llm
tags: [rust, token-counting, context-window, precedence-resolver, hexagonal-architecture, ports-and-adapters]

# Dependency graph
requires:
  - phase: 32-01
    provides: "Commissary::new/from_port with the caller-supplied exactness argument removed; Stockpile.exact_tally read live from the port"
provides:
  - "paladin_llm::window::resolve_context_window(model, config_table, capabilities, policy) -> Result<ResolvedWindow, UnknownContextWindow> -- the single shared four-step precedence walk"
  - "WindowFallbackPolicy (Default(u32) / Strict { caller_fallback: Option<u32> }) -- strictness as a type, not a bool"
  - "WindowSource (ConfigTable/ProviderCapabilities/Default/CallerFallback) with an exhaustive as_str() label and an ALL constant array"
  - "Six D-13 equivalence fixtures (three Commissary window/allowance cases, three HistoryTrimmer kept-set cases) committed green against the pre-resolver inline walks, before paladin_llm::window existed"
  - "paladin_llm::window re-exported from the facade src/lib.rs alongside the Commissary block"
affects: [32-04, 32-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fallback-policy enum (WindowFallbackPolicy) instead of a strict bool + separate Option<u32> -- the strict/lenient distinction is a type, so a caller cannot hold a flag that disagrees with its own option"
    - "Pre-refactor equivalence snapshot (D-13): a test committed green against the CURRENT code in its own commit, before the refactor that will replace that code exists -- the commit order (fixture commit precedes resolver commit) is the proof of equivalence, not just the assertions"
    - "Exhaustive label match with no wildcard arm (WindowSource::as_str) -- a new enum variant fails to compile until it is given a label, so an operator-facing log string can never silently go missing"

key-files:
  created:
    - crates/paladin-llm/src/window.rs
  modified:
    - crates/paladin-llm/src/services/commissary.rs
    - src/application/services/paladin/middleware/history.rs
    - crates/paladin-llm/src/lib.rs
    - src/lib.rs

key-decisions:
  - "Task 1 (the equivalence snapshot) was committed as its own test(32) commit, entirely before Task 2's window module existed on disk -- the commit order itself is the D-13 proof, not just the six fixture assertions."
  - "The four 'precedence_*'-named tests map exactly to the plan's acceptance criteria (config-table precedence, capability precedence, lenient default, strict refusal); a fifth test not in that named set (strict_policy_resolves_to_the_callers_fallback_when_present) is the only test that actually exercises WindowSource::CallerFallback -- without it no test would ever produce that source, since the label-invariant test only walks the enum array at compile/assert time, not through the resolver."
  - "Module-doc prose was written to avoid the literal substring 'strict: bool' anywhere in window.rs (including comments), since the plan's own acceptance check greps the whole file text for that anti-pattern, not just executable code -- the doc explanation uses 'boolean flag' instead."

patterns-established:
  - "D-13 equivalence-snapshot ordering: write and commit fixtures against the pre-refactor code FIRST, in their own commit, before the replacement code is created -- applies to any future phase collapsing two independent implementations of the same logic into one."

requirements-completed: [PRIM-04]

coverage:
  - id: D1
    description: "Six pre-resolver equivalence fixtures (three Commissary window/allowance cases: 8_444, 1_901, UndeclaredContextWindow naming the provider; three HistoryTrimmer kept-set cases: 1_234/two kept, 8_765/ten kept, 4_321/seven kept) committed green against the two still-unreplaced inline precedence walks, in a commit touching no production code."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary (window_and_allowance_equivalence_snapshot_pre_resolver, 20 tests total, 0 failed)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib history (kept_set_equivalence_snapshot_pre_resolver, 25 tests total, 0 failed)"
        status: pass
      - kind: other
        ref: "git show --name-only --format= 0e7434ba -- lists only commissary.rs and history.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "paladin_llm::window created: resolve_context_window (four-step precedence, pure, synchronous), WindowFallbackPolicy (Default/Strict, no bool flag), WindowSource (four variants, exhaustive as_str label, ALL array), ResolvedWindow, UnknownContextWindow -- all fully rustdoc'd, no #[allow(missing_docs)] on the module."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib window (9 tests, 0 failed)"
        status: pass
      - kind: other
        ref: "! git grep -qnE 'strict: *bool|is_strict' -- crates/paladin-llm/src/window.rs (exit 1, no match); ! git grep -qn '_ =>' -- crates/paladin-llm/src/window.rs (exit 1, no match)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Four named precedence tests individually selectable (config-table precedence, capability precedence, lenient default, strict refusal), plus the equal-values, absent-vs-empty, purity, caller-fallback-resolution and source-label-invariant tests -- nine total."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib window -- --list (9 tests, 4 with a precedence_ prefix)"
        status: pass
    human_judgment: false
  - id: D4
    description: "window module registered as a top-level pub mod window; in paladin-llm's ungated module block (after services) and re-exported from the facade src/lib.rs in the same pub use paladin_llm:: block style as the Commissary types."
    requirement: "PRIM-04"
    verification:
      - kind: other
        ref: "git grep -c 'pub mod window;' -- crates/paladin-llm/src/lib.rs (1); git grep -c 'paladin_llm::window' -- src/lib.rs (1); cargo build -p paladin-ai --all-features (exit 0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "make clean-code green (cargo fmt, cargo clippy --workspace --all-targets --all-features -D warnings, shellcheck, cargo check --workspace --all-targets) on the full resolver commit; the six equivalence fixtures still pass unmodified."
    requirement: "PRIM-04"
    verification:
      - kind: other
        ref: "cargo fmt --check; cargo clippy --workspace --all-targets --all-features -- -D warnings; make lint-shell; cargo check --workspace --all-targets (all exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary; cargo test -p paladin-ai --lib history (both still 0 failed after the resolver commit)"
        status: pass
    human_judgment: false

# Metrics
duration: ~52min
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 02: Shared Context-Window Resolver (PRIM-04) Summary

**`paladin_llm::window::resolve_context_window` — one pure, four-step precedence function (config table → provider capabilities → framework default or caller fallback) replacing two independent inline walks, with an explicit two-variant `WindowFallbackPolicy` instead of a strict-bool flag; landed only after six D-13 equivalence fixtures were committed green against the pre-refactor code.**

## Performance

- **Duration:** ~52 min
- **Started:** 2026-09-15T15:48:20Z (Task 1 commit)
- **Completed:** 2026-09-15T16:40:14Z (Task 2 commit)
- **Tasks:** 2
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments
- Committed a D-13 equivalence snapshot BEFORE any resolver code existed: three `Commissary` window/allowance cases (`8_444`, `1_901` tokens, and an `UndeclaredContextWindow` naming the provider `deepseek`) built directly against `Commissary::new`'s current inline `.or()` guard, and three `HistoryTrimmer` kept-set cases (`1_234`/two kept, `8_765`/ten kept, `4_321`/seven kept) over one fixed ten-entry, 2_468-character-per-entry history (exactly 617 tokens/entry under the heuristic counter, so every resolved limit divides evenly) — all six asserted on `allotted_tokens()`/the kept-set content, not on any private helper, so they survive the internal restructure plan 32-04 performs.
- Created `crates/paladin-llm/src/window.rs`: `resolve_context_window` (synchronous, pure, four-step precedence), `WindowFallbackPolicy` (`Default(u32)` always resolves, `Strict { caller_fallback: Option<u32> }` refuses when `None` — no bool flag anywhere), `WindowSource` (four variants, an exhaustive `as_str()` label match with no wildcard arm, and an `ALL` constant array), `ResolvedWindow`, `UnknownContextWindow`. All public items fully rustdoc'd with no `#[allow(missing_docs)]` on the module (the crate's `doctest = false` means every behavior is proven by a `#[test]`, not a doc example).
- Nine tests: four individually-selectable `precedence_*` tests matching the plan's named requirement (config-table beats capability, capability beats caller fallback, lenient policy resolves to its default, strict policy with no fallback errors), plus the equal-values edge (identical table/capability values still report the config-table source), the absent-vs-empty-table edge, the purity edge (repeated/interleaved calls agree), the one test that actually reaches `WindowSource::CallerFallback` (strict policy with a fallback present), and the source-label invariant walking `WindowSource::ALL` (every label non-empty, unique, and carrying its required substring — `config`, `provider`+`capabilities`, `default`).
- Registered `pub mod window;` in `paladin-llm`'s ungated module block (positioned after `services`, per the plan) and re-exported `ResolvedWindow, UnknownContextWindow, WindowFallbackPolicy, WindowSource, resolve_context_window` from the facade `src/lib.rs`, in the same block style as the existing Commissary re-export, with a one-line explanatory comment.
- Verified the resolver commit touches only its three owned files (`window.rs`, `paladin-llm/src/lib.rs`, `src/lib.rs`) and leaves both equivalence fixtures — and every other pre-existing `commissary`/`history` test — passing unmodified: 20/20 `commissary` tests, 25/25 `history`-module-adjacent tests, 9/9 `window` tests, `cargo build -p paladin-ai --all-features` and `make clean-code` (fmt, clippy `-D warnings`, shellcheck, check) all green.

## Task Commits

Each task was committed atomically:

1. **Task 1: commit the pre-resolver equivalence snapshot green** - `0e7434ba` (test)
2. **Task 2: create paladin_llm::window — one precedence walk with an explicit fallback policy** - `52b7447a` (feat)

**Plan metadata:** committed as part of this SUMMARY commit (see below).

_Note: Task 2 carries `tdd="true"` in the plan — the nine tests above were written against the
not-yet-existing `resolve_context_window`/`WindowFallbackPolicy`/`WindowSource` shapes first
(RED), then the implementation was written to make them pass (GREEN), both landing in the single
`52b7447a` commit per the plan's own instruction ("Run `cargo fmt` and clippy, then commit") —
there was no separate REFACTOR step needed._

## Files Created/Modified
- `crates/paladin-llm/src/window.rs` - new: the shared context-window precedence resolver (`WindowFallbackPolicy`, `WindowSource`, `ResolvedWindow`, `UnknownContextWindow`, `resolve_context_window`), nine tests
- `crates/paladin-llm/src/services/commissary.rs` - added `window_and_allowance_equivalence_snapshot_pre_resolver`, the D-13 pre-resolver fixture (Task 1 only; untouched by Task 2)
- `src/application/services/paladin/middleware/history.rs` - added `fixed_length_entry` helper and `kept_set_equivalence_snapshot_pre_resolver`, the D-13 pre-resolver fixture (Task 1 only; untouched by Task 2)
- `crates/paladin-llm/src/lib.rs` - registered `pub mod window;` in the ungated module block, after `services`
- `src/lib.rs` - re-exported the window module's public types alongside the Commissary block

## Decisions Made
- The fixture commit (`0e7434ba`) and the resolver commit (`52b7447a`) were kept strictly separate, with the fixture commit landing first — this ordering IS the D-13 equivalence proof (see plan objective), not an incidental sequencing choice.
- `WindowSource::CallerFallback` needed its own dedicated test (`strict_policy_resolves_to_the_callers_fallback_when_present`) beyond the four named precedence tests, because none of those four exercise that source through the resolver — without it, the source would exist in the enum and its label but never actually be produced by a passing test.
- Module-doc prose was phrased to say "boolean flag" rather than the literal string `strict: bool`, because the plan's acceptance check (`! git grep -qnE 'strict: *bool|is_strict' -- crates/paladin-llm/src/window.rs`) greps the whole file text, not just code — an earlier draft's doc comment describing the anti-pattern being avoided would have tripped its own anti-pattern check.

## Deviations from Plan

None — plan executed exactly as written. Both tasks matched their `<verify>` and `<acceptance_criteria>` blocks on the first pass; no Rule 1-4 fix was needed.

## Issues Encountered

The `Write` tool returned a "PreToolUse hook did not respond before its timeout" error twice in a row when first creating `crates/paladin-llm/src/window.rs` (a ~400-line new file) — no file was written on either attempt (confirmed via `ls`/`test -f` between retries). A third `Write` call with identical content succeeded. Not a plan or code issue; recorded in case the same transient hook-timeout resurfaces on other large new-file creations in this environment.

## Known Stubs

None — no stub patterns (hardcoded empty values feeding UI, placeholder text, unwired data
sources) apply to this plan's scope: a pure library function and its tests, with no consumer
wired to it yet (by design — plan 32-04 wires `Commissary`/`HistoryTrimmer` to call through it).

## Threat Flags

None — this plan's threat model (T-32-05 through T-32-08, T-32-SC) covers exactly the surface
this plan touches (the resolver's terminal step, its reported source, the config-table lookup,
and the new rustdoc/error text), and no new surface outside that register was introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PRIM-04's resolver half is complete: `paladin_llm::window::resolve_context_window` exists, is
  fully tested (9/9), builds clean across the workspace, and is re-exported from the facade —
  but has no consumer yet, by design.
- Plan 32-04 can now rewire `Commissary::new` (strict policy, mapping `UnknownContextWindow` into
  the existing `CommissaryError::UndeclaredContextWindow { provider }` variant, discarding the
  resolver's own `model` field per the plan's own note) and `HistoryTrimmer::resolve_limit`
  (lenient policy, deleting the now-redundant private `LimitSource` enum and consuming
  `WindowSource` directly per this plan's promotion decision) to call through the shared resolver.
- The six equivalence fixtures committed in Task 1 are the byte-identical snapshot plan 32-04
  must reproduce after rewiring both consumers — they are deliberately left unedited and
  unmodified by this plan's Task 2 commit, confirmed by `git show --name-only` on `52b7447a`.
- No blockers.

## Self-Check: PASSED

All files listed under Files Created/Modified confirmed present on disk (`window.rs` created;
`commissary.rs`, `history.rs`, `paladin-llm/src/lib.rs`, `src/lib.rs` modified). Both commit
hashes (`0e7434ba`, `52b7447a`) confirmed in `git log --oneline -5`.

---
*Phase: 32-unified-token-primitives*
*Completed: 2026-09-15*
