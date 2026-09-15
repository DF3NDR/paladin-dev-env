---
phase: 32-unified-token-primitives
plan: 04
subsystem: llm
tags: [rust, token-counting, context-window, precedence-resolver, hexagonal-architecture]

# Dependency graph
requires:
  - phase: 32-02
    provides: "paladin_llm::window::resolve_context_window, WindowFallbackPolicy, WindowSource -- the single shared precedence walk with six D-13 equivalence fixtures committed green against the pre-resolver code"
provides:
  - "Commissary::new resolves its context window through exactly one call to resolve_context_window, under WindowFallbackPolicy::Strict with an absent config table (D-03), storing the result in a new private resolved_window field"
  - "HistoryTrimmer::resolve_limit as a thin call-through to resolve_context_window under WindowFallbackPolicy::Default, returning the shared WindowSource directly"
  - "The facade's local three-variant LimitSource enum deleted workspace-wide -- one context-window source type in the workspace, not two"
  - "Zero inline capability-or-fallback precedence expressions remaining anywhere in commissary.rs"
affects: [32-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Resolve-once-at-construction, read-back accessor: Commissary::new calls the shared resolver exactly once and stores the count on a private field; the private window() accessor reads the field instead of re-walking the precedence order on every allowance query"
    - "Explicit-branch handling of an always-Ok Result under a lenient policy, documented as unreachable rather than asserted (CLAUDE.md no-unwrap/expect/panic rule) -- HistoryTrimmer::resolve_limit's Err arm falls back to the caller's own configured default"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/services/commissary.rs
    - src/application/services/paladin/middleware/history.rs

key-decisions:
  - "The D-13 equivalence fixture's own doc comment (written by plan 32-02) quoted the literal pre-resolver expression `capabilities.max_context_tokens.or(config.fallback_context_tokens)`, which trips this task's own hard `! git grep -qn '\\.or('` verify gate once the resolver is wired. Reworded only that doc-comment line (outside the test's function body and its asserted rows/figures) to describe the same fallback in prose without the literal `.or(` substring -- the same pitfall plan 32-02 already solved for window.rs's own module doc (see its SUMMARY's key-decisions)."
  - "Commissary::new passes `&config.model_hint` as the resolver's `model` argument and `None` as its config table (D-03) -- CommissaryPlan gains no new field, so step one of the resolver's four-step walk is a permanent no-op for Commissary and every window it resolves stays identical to the pre-resolver value. A one-line comment at the call site records this as deliberate."
  - "HistoryTrimmer::resolve_limit's Err arm (unreachable under WindowFallbackPolicy::Default, which always resolves) is handled with an explicit match arm falling back to `self.config.default_context_tokens` paired with `WindowSource::Default`, rather than `.unwrap()`/`.expect()` -- satisfies CLAUDE.md's no-panic-in-library-code rule even though the arm can never execute."

patterns-established: []

requirements-completed: [PRIM-04]

coverage:
  - id: D1
    description: "Commissary::new resolves its context window via exactly one call to resolve_context_window under WindowFallbackPolicy::Strict (config.model_hint as model, None as config table, config.fallback_context_tokens as caller_fallback); the resolved count is stored on a new resolved_window field and returned by the private window() accessor without re-walking the precedence order."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary (20 tests, 0 failed, including window_and_allowance_equivalence_snapshot_pre_resolver)"
        status: pass
      - kind: other
        ref: "git grep -n 'resolve_context_window(' -- crates/paladin-llm/src/services/commissary.rs (exactly one call site, at Commissary::new); ! git grep -qn '\\.or(' -- crates/paladin-llm/src/services/commissary.rs (exit 1, no match)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The resolver's UnknownContextWindow error is mapped inside Commissary::new into the pre-existing CommissaryError::UndeclaredContextWindow variant with its #[error(...)] Display text and field byte-identical to the pre-plan definition, naming the provider."
    requirement: "PRIM-04"
    verification:
      - kind: other
        ref: "git diff HEAD~2 -- crates/paladin-llm/src/services/commissary.rs shows no change inside the CommissaryError enum definition"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-llm --lib commissary -- window_and_allowance_equivalence_snapshot_pre_resolver (asserts the UndeclaredContextWindow variant and that the message names 'deepseek')"
        status: pass
    human_judgment: false
  - id: D3
    description: "HistoryTrimmer::resolve_limit is a thin call-through to resolve_context_window under WindowFallbackPolicy::Default(config.default_context_tokens) with self.config.model_context_limits as the config table; the facade's local LimitSource enum and its label accessor are deleted workspace-wide in favour of the shared WindowSource."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib history (25 tests, 0 failed, including the three limit_resolution_* precedence tests and kept_set_equivalence_snapshot_pre_resolver)"
        status: pass
      - kind: other
        ref: "! git grep -qn 'LimitSource' -- src crates (exit 1, no match); git grep -c 'resolve_context_window' -- src/application/services/paladin/middleware/history.rs (one call site, at resolve_limit)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The six D-13 equivalence fixtures committed in plan 32-02 (three Commissary window/allowance cases, three HistoryTrimmer kept-set cases) pass unedited (body and asserted figures untouched) after both consumers are rewired, and no unwrap/expect/panic was introduced in either file outside its #[cfg(test)] module."
    requirement: "PRIM-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-llm --lib window (14 tests, 0 failed); cargo test -p paladin-llm --doc and cargo test -p paladin-ai --doc (8 and 147 passed respectively)"
        status: pass
      - kind: other
        ref: "cargo fmt --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0); cargo check --workspace --all-targets (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~35min
completed: 2026-09-15
status: complete
---

# Phase 32 Plan 04: Consumers Wired to the Shared Context-Window Resolver Summary

**`Commissary::new` and `HistoryTrimmer::resolve_limit` both now call `paladin_llm::window::resolve_context_window` -- one shared four-step precedence walk consumed by a strict caller (Commissary, absent config table, D-03) and a lenient caller (HistoryTrimmer, config table plus framework default), collapsing the two independent inline precedence walks plan 32-02 left in place while proving the collapse changed no behaviour via the pre-committed D-13 equivalence fixtures.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-15 (Task 1 commit)
- **Completed:** 2026-09-15 (Task 2 commit)
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `Commissary::new` (`crates/paladin-llm/src/services/commissary.rs`) replaced its inline `capabilities.max_context_tokens.or(config.fallback_context_tokens)` guard with exactly one call to `resolve_context_window`, passing `&config.model_hint` as the model, `None` as the config table (D-03 -- `CommissaryPlan` gains no new field, so this step is a permanent no-op for Commissary), a reference to the already-held `capabilities`, and `WindowFallbackPolicy::Strict { caller_fallback: config.fallback_context_tokens }`. The resolver's `UnknownContextWindow` is mapped via `map_err` into the pre-existing `CommissaryError::UndeclaredContextWindow { provider }`, whose `#[error(...)]` Display text is byte-identical to the pre-plan wording (confirmed by diff -- the enum definition is untouched). The resolved count is stored on a new private `resolved_window: u32` field, set once in `new`, and the private `window()` accessor now returns that stored value instead of re-walking the precedence order (its own duplicate `.or(...)` expression is deleted). `allotted_tokens()`, the reservation guard, and `from_port` are unchanged.
- `HistoryTrimmer::resolve_limit` (`src/application/services/paladin/middleware/history.rs`) is now a thin call-through to the same `resolve_context_window`, passing `model` unchanged, `Some(&self.config.model_context_limits)` as the config table, the port's own freshly-read `get_capabilities()`, and `WindowFallbackPolicy::Default(self.config.default_context_tokens)`. Its local three-variant `LimitSource` enum and `as_str` label accessor are deleted; `resolve_limit` returns `(u32, WindowSource)` directly -- confirmed `LimitSource` no longer exists anywhere in `src` or `crates` (the promote decision recorded in plan 32-02's assumption-delta block, now applied). The resolver's `Result` is always `Ok` under the lenient policy, but its `Err` arm is still handled with an explicit branch (falling back to `self.config.default_context_tokens` paired with `WindowSource::Default`, commented as unreachable-but-handled) rather than `.unwrap()`/`.expect()`, per CLAUDE.md's no-panic-in-library-code rule.
- Both module docs were realigned to name the shared resolver as the authority: `Commissary::new`'s `# Errors` bullet and the private `window()` accessor's rustdoc for the Commissary side; `history.rs`'s module-level "Limit resolution" section header and paragraph for the HistoryTrimmer side (D-14/D-15 citations kept, the WHY-log-source sentence kept, the trimming-semantics paragraph and every trim test untouched).
- All six D-13 equivalence fixtures from plan 32-02 pass unedited: `window_and_allowance_equivalence_snapshot_pre_resolver` (commissary.rs, three rows: `8_444`/`1_901`/`UndeclaredContextWindow`) and `kept_set_equivalence_snapshot_pre_resolver` (history.rs, three cases: two/ten/seven kept entries) -- confirmed by `git diff` showing no change inside either test's function body or asserted figures, only a reworded doc comment above the commissary.rs fixture (see Decisions Made). The three pre-existing `limit_resolution_*` precedence tests in history.rs also pass unedited.
- Full verification: `cargo test -p paladin-llm --lib commissary` (20/20), `cargo test -p paladin-llm --lib window` (14/14), `cargo test -p paladin-ai --lib history` (25/25), `cargo test -p paladin-llm --doc` (8/8) and `cargo test -p paladin-ai --doc` (147/147, 18 ignored) all green; `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets` all exit 0. `cargo test --workspace --all-features --no-fail-fast` shows exactly one pre-existing, unrelated failure (`-p paladin-ai --test cli_isolation`, `test_cli_feature_is_not_default`) -- see Deviations.

## Task Commits

Each task was committed atomically:

1. **Task 1: Commissary resolves its window through the shared resolver, once, in the constructor** - `ddd47172` (refactor)
2. **Task 2: HistoryTrimmer resolves through the same function and the facade's duplicate source enum is deleted** - `ba429719` (refactor)

**Plan metadata:** committed as part of this SUMMARY commit (see below).

## Files Created/Modified

- `crates/paladin-llm/src/services/commissary.rs` - `Commissary::new` calls `resolve_context_window` once under `WindowFallbackPolicy::Strict` with an absent config table; new private `resolved_window` field; `window()` accessor reads it back; rustdoc realigned; D-13 fixture's doc comment reworded (body/figures unedited)
- `src/application/services/paladin/middleware/history.rs` - `HistoryTrimmer::resolve_limit` is a call-through to `resolve_context_window` under `WindowFallbackPolicy::Default`, returning `(u32, WindowSource)`; local `LimitSource` enum and its label accessor deleted; module doc's resolution-order paragraph realigned

## Decisions Made

- Reworded the D-13 equivalence fixture's doc comment in `commissary.rs` (written by plan 32-02) to avoid the literal substring `.or(` -- it quoted the exact pre-resolver expression as documentation, which trips this task's own hard `! git grep -qn '\.or('` verify gate once the resolver is wired in below it. Only the doc-comment prose above the test function changed; the test's body, its three-row table, and every asserted figure are byte-identical to the pre-plan commit (confirmed by diff). This mirrors plan 32-02's own recorded decision to avoid the literal substring `strict: bool` in `window.rs`'s module doc for the identical reason (its acceptance check greps the whole file text, not just executable code).
- `Commissary::new` passes `&config.model_hint` (not the provider string) as the resolver's `model` argument -- the resolver's `model` parameter is only consulted for the config-table lookup (step one), which is always `None` for Commissary (D-03), so the choice is inert for behaviour but keeps the call site honest about which field the resolver would key on if a table were ever added.
- `HistoryTrimmer::resolve_limit`'s `Err` arm (structurally unreachable under `WindowFallbackPolicy::Default`) falls back to `self.config.default_context_tokens` paired with `WindowSource::Default` rather than `.unwrap()` or `.expect()` -- an explicit, commented branch satisfies CLAUDE.md's no-panic-in-library-code rule even though the branch can never execute given the lenient policy's own contract.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Reworded a pre-existing doc comment quoting the literal `.or(` expression to unblock Task 1's own hard verify gate**
- **Found during:** Task 1
- **Issue:** The D-13 equivalence fixture's doc comment (committed by plan 32-02, immediately above `window_and_allowance_equivalence_snapshot_pre_resolver`) quotes `` `capabilities.max_context_tokens.or(config.fallback_context_tokens)` `` verbatim as documentation of the pre-resolver behaviour it snapshots. Task 1's own `<verify><automated>` command requires `! git grep -qn '\.or(' -- crates/paladin-llm/src/services/commissary.rs` to pass (exit 1, no match) once the inline `.or(...)` calls in `new()`/`window()` are removed -- but this pre-existing doc-comment string alone would keep the grep matching.
- **Fix:** Reworded only that doc-comment line to describe the same fallback in prose ("`capabilities.max_context_tokens`, falling back to `config.fallback_context_tokens` when the provider declared none") without the literal `.or(` substring. The test's function body, its three-row table, and every asserted figure are unchanged (confirmed by diff).
- **Files modified:** crates/paladin-llm/src/services/commissary.rs
- **Verification:** `git grep -n '\.or(' -- crates/paladin-llm/src/services/commissary.rs` now exits 1 (no match); `cargo test -p paladin-llm --lib commissary` still reports 20/20 passed including the fixture; `git diff` shows the change confined to the doc-comment line, outside the test's `fn` body.
- **Committed in:** `ddd47172` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** No scope creep -- the fix touches only prose documentation quoting old code, required to satisfy the task's own verify command, and does not alter the fixture's proof value (the D-13 equivalence is still enforced by the unedited test body).

## Issues Encountered

`cargo test --workspace --all-features --no-fail-fast` reports exactly one failing target: `-p paladin-ai --test cli_isolation`, specifically `test_cli_feature_is_not_default`, which panics because `--all-features` forces the `cli` feature on for a test that asserts it is NOT part of the default feature set. Confirmed unrelated to this plan (the panic message and test name name the `cli` feature in isolation, not anything touched by `commissary.rs` or `history.rs`) and pre-existing: PROJECT.md's Phase 31 close-out already carries this exact item ("the `cli_isolation` `--all-features` conflict (deferred-items.md) -- neither introduced by this phase"). Logged in `.planning/phases/32-unified-token-primitives/deferred-items.md` under "Plan 32-04, verification" per the executor's Scope Boundary rule rather than fixed here (out of scope: the fix belongs to whichever test or Cargo.toml feature-gating change owns `tests/cli_isolation_test.rs`, not this plan's two files). Every per-crate scoped test command the plan's own `<verify>` blocks specify (`cargo test -p paladin-llm --lib commissary`, `--lib window`, `cargo test -p paladin-ai --lib history`) is unaffected and green.

## Known Stubs

None -- both files are complete, real implementations with no placeholder values, unwired data sources, or deferred UI wiring; the equivalence fixtures prove behavioural continuity rather than standing in for missing functionality.

## Threat Flags

None -- this plan's threat model (T-32-13 through T-32-16, T-32-SC) covers exactly the surface this plan touches (the enforced window after the swap, the mapped error's Display text, the logged trim source, and the panic-freedom of both consumers), and no new surface outside that register was introduced. No new dependency was added to `crates/paladin-llm/Cargo.toml` or the root manifest.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PRIM-04 is complete: one shared precedence walk (`paladin_llm::window::resolve_context_window`) is now the ONLY context-window resolution logic in the workspace, consumed by a strict caller (`Commissary`) and a lenient caller (`HistoryTrimmer`); no inline `.or(...)` precedence expression and no duplicate `LimitSource`/`WindowSource`-shaped enum survive anywhere.
- All six D-13 equivalence fixtures plus the three pre-existing `HistoryTrimmer` precedence tests remain green and unedited, proving the collapse changed no observable behaviour.
- Plan 32-05 (the phase's close-out plan) can proceed to the full-phase verification sweep -- `cargo test --workspace --all-features --no-fail-fast` will still show the pre-existing, documented `cli_isolation` item unrelated to this phase's scope; nothing from this plan blocks that sweep.
- No blockers.

## Self-Check: PASSED

Both modified files confirmed present on disk with the expected changes (`crates/paladin-llm/src/services/commissary.rs`, `src/application/services/paladin/middleware/history.rs`). Both task commit hashes (`ddd47172`, `ba429719`) confirmed present via `git log --oneline`.

---
*Phase: 32-unified-token-primitives*
*Completed: 2026-09-15*
