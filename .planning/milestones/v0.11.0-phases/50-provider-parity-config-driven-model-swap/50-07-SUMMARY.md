---
phase: 50-provider-parity-config-driven-model-swap
plan: 07
subsystem: api
tags: [rust, paladin-llm, commissary, history-trimmer, context-window, model-registry, adr-0061, parity]

requires:
  - phase: 50
    provides: "plan 50-03: ModelRegistry, ModelOverride, LlmPort::model_profile consumers; plan 50-06: seeded per-model windows (kimi-k3 1,048,576, kimi-k2.6 262,144) and adapters whose declared window is the default model's"
provides:
  - "paladin_llm::window::effective_capabilities(port, model): the port's ProviderCapabilities with max_context_tokens replaced by the model profile's context_window when it has one (also re-exported from the facade)"
  - "Commissary::for_model(port, model, counter, plan): builds the Commissary over effective_capabilities, sets model_hint, labels with the port's provider name; Commissary::new and from_port unchanged"
  - "HistoryTrimmer::resolve_limit resolves through effective_capabilities; config table and Default policy untouched"
  - "no WindowSource variant added (ALL still has four entries)"
affects: [50-08, 50-09, 50-10, 50-15, 50-16]

tech-stack:
  added: []
  patterns:
    - "Overlay the per-model window onto the port's capabilities before the shared resolver, instead of a fifth WindowSource variant: the precedence order, the labels and the public enum are all unchanged"

key-files:
  created: []
  modified:
    - crates/paladin-llm/src/window.rs
    - crates/paladin-llm/src/services/commissary.rs
    - src/application/services/paladin/middleware/history.rs
    - src/lib.rs
    - .project/current-exports.txt
    - CHANGELOG.md

key-decisions:
  - "Overlay, not a WindowSource variant: WindowSource is public, not non_exhaustive, with ALL: [WindowSource; 4]; the profile window travels as the ProviderCapabilities step, so HistoryTrimmer's logged source label is unchanged"
  - "effective_capabilities trusts the profile window (A3-50-07); plan 50-08 folds the adapter-wide operator knob into that profile, so it reaches both readers with no branch here"
  - "Commissary::for_model uses the port's own get_provider_name as the provider label (A2-50-07)"
  - "effective_capabilities is re-exported from the facade next to resolve_context_window, so the API baseline tracks it"

patterns-established:
  - "Override test without adapter plumbing: a stub LlmPort over a ModelRegistry built with_overrides stands in for an adapter until plan 50-08 threads overrides through the config"

requirements-completed: [PARITY-04]

coverage:
  - id: D1
    description: "effective_capabilities overlays the model profile's window on the port's capabilities and leaves them untouched when there is no profile or no profile window"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "effective_capabilities_overlays_the_profile_window; effective_capabilities_changes_only_the_window; the_overlay_adds_no_window_source_variant (window::tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Commissary::for_model rations kimi-k3 against its seeded window exactly, an override beats the seed, the reservation boundary is W refused / W-1 accepted, Strict still refuses without any window, an unknown model uses the adapter-wide window"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "kimi_k3_is_rationed_against_its_seeded_window; for_model_prefers_an_override_over_the_seed; for_model_reservation_boundary; strict_still_refuses_without_any_window; an_unknown_model_uses_the_adapter_wide_window; for_model_reads_the_profile_window_and_sets_the_model_hint; new_still_reads_only_the_capabilities_it_is_given (commissary::tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "the HistoryTrimmer rations against the per-model window; its config table still wins and its Default policy still applies"
    requirement: "PARITY-04"
    verification:
      - kind: unit
        ref: "history_trimmer_reads_the_per_model_window; the_config_table_still_beats_the_profile_window; the_default_policy_still_applies_when_nothing_declares_a_window (history::tests, 28 history tests pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "nothing about the existing window API or the consumer surface breaks"
    requirement: "PARITY-04"
    verification:
      - kind: integration
        ref: "make check-consumer-surface (Commissary::new pinned), make api-surface (4275 items, baseline refreshed), make check-api-examples (105 entry points), cargo test --test provider_swap_matrix --features llm-all (40 passed, 9 ignored for later plans), cargo test -p paladin-llm --all-features --lib (793 passed)"
        status: pass
    human_judgment: false

duration: ~40min
completed: 2026-10-16
status: complete
---

# Phase 50 Plan 07: Per-model windows reach the Commissary and the HistoryTrimmer Summary

**`effective_capabilities` overlays a model profile's context window on the port's capabilities, and both window readers (`Commissary::for_model` and `HistoryTrimmer`) resolve through it, so `kimi-k3` is rationed against its seeded 1,048,576 tokens instead of the adapter-wide 262,144, with no new `WindowSource` variant.**

## Performance

- **Duration:** about 40 min (start time not captured; estimated from the first build and the commit timestamps)
- **Completed:** 2026-10-16
- **Tasks:** 2 of 2
- **Files modified:** 6 (none created)

## Accomplishments

- `window::effective_capabilities(port, model)`: starts from `port.get_capabilities()` and, when `port.model_profile(model)` carries a `context_window`, replaces `max_context_tokens` with it. Everything else in the capabilities is the port's own. The rustdoc explains why this is an overlay rather than a fifth `WindowSource` and carries an `# Examples` block.
- `Commissary::for_model(port, model, counter, plan)`: sets `plan.model_hint = model`, labels the Commissary with `port.get_provider_name()` and delegates to `Commissary::new` with the overlaid capabilities. `new` and `from_port` are byte-for-byte unchanged; the window is still resolved once, so a different model needs a new Commissary (documented).
- `HistoryTrimmer::resolve_limit` calls `effective_capabilities(self.llm_port.as_ref(), model)` before `resolve_context_window`; the module rustdoc's resolution-order sentence is updated. The config table still wins and the `Default` policy still applies.
- 13 new tests (3 window, 7 Commissary of which 2 are Kimi, 3 trimmer; see coverage), the Kimi pair under `#[cfg(feature = "kimi")]`.
- Root CHANGELOG `### Added` bullet; `.project/current-exports.txt` refreshed (4274 to 4275 items).

## Task Commits

1. **Task 1: effective_capabilities and Commissary::for_model** - `c0890a62` (feat)
2. **Task 2: HistoryTrimmer per-model window, facade re-export, baseline, CHANGELOG** - `5b115bb6` (feat)

**Plan metadata:** the docs commit that carries this SUMMARY, STATE.md, ROADMAP.md and REQUIREMENTS.md.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 793 passed (12 `window::`, 27 `commissary`).
- `cargo test --lib history`: 28 passed, including `history_trimmer_reads_the_per_model_window`.
- `cargo test --workspace --lib --bins --no-fail-fast`: every binary green except the two known baseline failures (`build_run_api_persists_no_run_traces_by_default`, `..._persists_run_traces_when_trace_persist_is_set`, WINDOWS.md row 73); the root crate reports 1360 passed, 2 failed. No new failure.
- `cargo test --test provider_swap_matrix --features llm-all`: 40 passed, 9 ignored (later plans).
- `cargo fmt --check` clean. `cargo clippy --workspace --all-features -- -D warnings` clean; `cargo clippy -p paladin-llm --all-features --all-targets -- -D warnings` clean; `--lib` clean for `--no-default-features` and for `kimi` alone. `make api-surface`, `make check-consumer-surface`, `make check-api-examples` pass. `make clean-code` was not run end to end (shellcheck is missing, per the baseline note); its fmt and clippy parts were run directly.
- The doc examples for `effective_capabilities` and `Commissary::for_model` were compiled and run once with `doctest = true` temporarily set in `crates/paladin-llm/Cargo.toml` (2 passed), then the setting was restored to `false` (see Deviations 1).
- Manual credential-handling review (D-00a): no credential path touched. The diff adds a pure function over already-resolved capabilities, a constructor wrapper, one resolver argument change and tests; no HTTP client, header, key or response body is read or logged.
- Threat model: T-50-07-01 (widened window) is bounded because windows come only from the evidenced seeds (50-06) and validated overrides (50-03), and the vendor's own overflow status still maps to `TokenLimitExceeded`; T-50-07-02 (zero or missing window) because zero windows are rejected at seed and override validation and the Strict refusal is unchanged and tested (`strict_still_refuses_without_any_window`).

## Decisions Made

See `key-decisions`. Flagged assumptions: A1-50-07 does not apply (Moonshot was probed, so `kimi_k3_is_rationed_against_its_seeded_window` uses the seeded value, and the test also pins that the table says 1,048,576, matching the 2026-10-15 listing `context_length=1048576`); A2 and A3 applied as written.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan step not executable as written] the `--doc effective_capabilities` verify command runs nothing**
- **Found during:** Task 1
- **Issue:** `crates/paladin-llm/Cargo.toml` sets `[lib] doctest = false`, so `cargo test -p paladin-llm --all-features --doc effective_capabilities` selects no doctests. The `# Examples` blocks the plan requires would never be compiled by CI.
- **Fix:** left the crate setting alone (it is a deliberate, crate-wide choice, outside this plan's files). Verified both new examples by temporarily flipping it to `true`: `cargo test -p paladin-llm --all-features --doc -- effective_capabilities for_model` ran 2 tests, both passed; then restored `false` (`git diff` on the manifest is empty). The `for_model` example uses a local `TokenCounterPort` rather than `paladin_memory` because `paladin-llm` has no such dependency.
- **Files modified:** none retained

**2. [Rule 2 - Missing critical] `effective_capabilities` re-exported from the root facade**
- **Found during:** Task 2
- **Issue:** the plan's acceptance `grep -c effective_capabilities .project/current-exports.txt` at least 1 cannot hold unless the root crate re-exports it (the baseline tracks only the root crate, the same limitation plans 50-03 and 50-06 recorded). The facade already re-exports its sibling `resolve_context_window` with the comment "Facade users see one surface for window resolution".
- **Fix:** added `effective_capabilities` to that existing `pub use paladin_llm::window::{..}` block, ran `make api-surface-update`, and wrote the CHANGELOG entry (`.project/current-exports.txt` +1 line, `src/lib.rs` +1 name). `src/lib.rs` is not in the plan's `files_modified`. `Commissary::for_model` is a method on an already re-exported type, so it adds no baseline line.
- **Files modified:** `src/lib.rs`, `.project/current-exports.txt`
- **Commit:** `5b115bb6`

### Notes, not deviations

- The override test builds a stub port over `ModelRegistry::with_overrides` rather than a `KimiAdapter`, because adapters take no overrides until plan 50-08 threads them through configuration. It uses the same registry type the adapter holds.
- `cargo clippy -p paladin-llm --no-default-features --lib --tests -- -D warnings` reports four dead-code errors in `cadence.rs` (`PinnedSpread`, `pin_waiter_spread`) and `fallback.rs` (`ProfiledStubPort`) test helpers. They exist on the plan's base commit and are not touched here; `--lib` without `--tests` is clean. Left as is (scope boundary).

## Issues Encountered

None blocking.

## Observations for later plans (not fixed here)

- `paladin-memory`'s RAG retrieval service builds its Commissary with `Commissary::new("rag", ..)` over a synthetic capability whose window is the injection budget, not a provider window. It correctly has no per-model reading and is unaffected.
- Only `HistoryTrimmer` and the new `Commissary::for_model` read the overlay. Any future consumer that needs a model's window should call `effective_capabilities` rather than `get_capabilities().max_context_tokens`; several root-crate services still read `max_context_tokens` directly for other purposes (summarization, planning, prompt generation) and were not in this plan's scope.
- `[lib] doctest = false` in `paladin-llm` means none of its `# Examples` blocks run in CI; the plan 50-16 or a hygiene plan may want to revisit it.

## Known Stubs

None.

## Threat Flags

None. No endpoint, auth path, file access or trust boundary was added.

## User Setup Required

None.

## Next Phase Readiness

- Plan 50-08's adapter-wide `context_window` knob and per-model overrides fold into the profile (`ModelRegistry::with_context_window`), so they reach both readers through `effective_capabilities` with no change here; its `context_window_override_beats_a_seeded_window` can assert through that function.
- Plan 50-16's switching-providers page can state that the Commissary (via `for_model`) and the history trimmer ration against the model's own window.

## Self-Check: PASSED

- Files: `crates/paladin-llm/src/window.rs`, `crates/paladin-llm/src/services/commissary.rs`, `src/application/services/paladin/middleware/history.rs`, `src/lib.rs`, `.project/current-exports.txt`, `CHANGELOG.md` all modified in the two task commits; `pub fn effective_capabilities` x1 and `pub fn for_model` x1 confirmed; `ALL: [WindowSource; 4]` x1 confirmed.
- Commits: `c0890a62`, `5b115bb6` present in `git log`.
