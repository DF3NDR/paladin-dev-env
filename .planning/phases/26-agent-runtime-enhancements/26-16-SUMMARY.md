---
phase: 26-agent-runtime-enhancements
plan: 16
subsystem: agent-runtime
tags: [arsenal, vault, namespace-confinement, in-process-tools, composite-port, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 13)
    provides: "ConfinedVault (paladin-ports), RunScope, PaladinExecutionService::confined_vault/execute_scoped -- the grant-resolution machinery this plan's VaultTools reads through"
  - phase: 26-agent-runtime-enhancements (plan 15)
    provides: "ModelCallContext.vault, the confined_vault threading through execute_scoped/execute_bounded/execute_internal -- the same resolved grant this plan reuses for the tool-call arsenal, not a second resolution"
provides:
  - "InProcessArsenal (src/application/services/arsenal/in_process_arsenal.rs): a public ArsenalPort over registered Armament definitions + async closures -- the in-process counterpart of ArsenalExecutionService's MCP-routed invoke; validate_call reuses paladin_core::structured::shape_check (plan 26-12); invoke never wraps a handler in catch_unwind (documented house rule)"
  - "CompositeArsenalPort (src/application/services/arsenal/composite_arsenal.rs): unions several ArsenalPorts behind one, first-registration-of-a-name-wins with every duplicate logged at warn; an empty composite lists nothing and denies every call with a typed ArsenalError::ToolNotFound"
  - "VaultTools::new(confined) -> InProcessArsenal (src/application/services/arsenal/vault_tools.rs): vault_get/vault_put Armaments with JSON-Schema parameters, absolute namespace addressing, Namespace::new validation before any ConfinedVault call, a documented `{\"found\":false}` not-found shape, and VaultError -> tool-result-error rendering"
  - "PaladinExecutionService::enable_vault_tools() / effective_arsenal(): the opt-in flag (off by default) and the per-run resolution that composes the configured arsenal with a fresh VaultTools scoped to that run's own ConfinedVault -- a run with no grant sees no vault tools regardless of the flag"
  - "tests/integration/vault_confinement_test.rs: PRD 05 section 3.4's attack test end to end through the real reasoning loop, plus the X-05 concurrency sweep"
affects: []

tech-stack:
  added: []
  patterns:
    - "InProcessArsenal is the in-process sibling of ArsenalExecutionService's MCP-routed ArsenalPort -- a boxed async closure stands in for the MCP round-trip, sharing the same list_armaments/invoke/validate_call contract"
    - "CompositeArsenalPort follows fallback.rs's chain-of-ports-as-one-port shape: validate_call's per-member ToolNotFound signals routing, so invoke/validate_call and the async list_armaments union all resolve collisions the same first-wins way without needing an async lookup inside a sync trait method"
    - "A tool handler returns Result<Value, String> rather than Result<Value, ArsenalError> -- a domain error (VaultError) has no natural ArsenalError variant, and forcing one would be an arbitrary, usually-wrong choice; InProcessArsenal::invoke converts Err(message) into a failed ArmamentResult, never a panic or a propagated Err"

key-files:
  created:
    - src/application/services/arsenal/in_process_arsenal.rs
    - src/application/services/arsenal/composite_arsenal.rs
    - src/application/services/arsenal/vault_tools.rs
    - tests/integration/vault_confinement_test.rs
  modified:
    - src/application/services/arsenal/mod.rs
    - src/lib.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - tests/integration/mod.rs

key-decisions:
  - "CompositeArsenalPort's invoke/validate_call route by asking each member's own (synchronous) validate_call whether it returns ArsenalError::ToolNotFound, rather than awaiting each member's async list_armaments from inside the synchronous validate_call trait method (which is structurally impossible -- ArsenalPort::validate_call is fn, not async fn). The FIRST member whose validate_call is not ToolNotFound wins, whether that result is Ok or a real validation error -- which also naturally implements first-registration-wins for invoke without a separate lookup table."
  - "VaultTools::new is a factory function returning InProcessArsenal, not a constructor of Self (VaultTools is a stateless marker type with nothing to construct) -- annotated #[allow(clippy::new_ret_no_self)] with a rustdoc comment explaining why, rather than renamed away from the plan's literal `VaultTools::new(confined) -> InProcessArsenal` signature."
  - "PaladinExecutionService::effective_arsenal(&self, confined_vault: Option<&ConfinedVault>) is a new private resolution method, computed in execute_internal from a CLONE of confined_vault taken before it is moved into middleware_cx.vault -- so the tool-call branch and VaultRecallMiddleware (plan 26-15) observe the exact same grant for one run, never two independently-resolved handles."
  - "vault_get/vault_put handlers return Err(String), not Err(ArsenalError) or Err(VaultError) -- InProcessArsenal's ToolHandler type is Fn(..) -> Future<Output = Result<Value, String>>, so a handler built on ANY domain error (VaultError here, something else for a future built-in tool) can report failure without inventing an ArsenalError variant that doesn't fit."

patterns-established:
  - "A tool handler closure reports failure as Result<Value, String>, not a typed error -- InProcessArsenal::invoke is the single place that turns Err(message) into a failed ArmamentResult, so every future in-process tool (not just vault_get/vault_put) gets 'never a panic, never a propagated Err' for free."

requirements-completed: [RT-04]

coverage:
  - id: D1
    description: "InProcessArsenal is a public ArsenalPort over registered Armament definitions plus async closures: list_armaments/invoke/validate_call mirror ArsenalExecutionService's MCP-routed contract, validate_call reuses paladin_core::structured::shape_check rather than a second validator, invoke never wraps a handler in catch_unwind (documented), and a handler's Err becomes a failed ArmamentResult rather than a panic or propagated Err"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/arsenal/in_process_arsenal.rs#tests (4 tests: in_process_arsenal_lists_and_invokes_a_registered_closure, in_process_arsenal_rejects_an_unknown_tool, validate_call_checks_arguments_against_the_declared_schema, a_closure_error_becomes_an_armament_result_error_not_a_panic)"
        status: pass
      - kind: other
        ref: "grep -c shape_check src/application/services/arsenal/in_process_arsenal.rs >= 1; grep -rv '^\\s*//|^\\s*///' src/application/services/arsenal/ | grep -c 'catch_unwind(' == 0; grep -c 'InProcessArsenal|CompositeArsenalPort' src/lib.rs >= 2; cargo test -p paladin-ai --doc in_process (1 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "CompositeArsenalPort unions several ArsenalPorts: list_armaments unions with first-registration-of-a-name-wins and logs every duplicate at warn; invoke/validate_call route by the same first-wins resolution; an empty composite lists nothing and denies every call with a typed ArsenalError::ToolNotFound"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/arsenal/composite_arsenal.rs#tests (3 tests: composite_unions_list_armaments_first_registration_wins, composite_routes_invoke_and_validate_by_name, composite_of_zero_arsenals_lists_nothing_and_denies_everything)"
        status: pass
    human_judgment: false
  - id: D3
    description: "VaultTools::new(confined) builds vault_get/vault_put Armaments with JSON-Schema parameters, absolute (not grant-relative) namespace addressing, Namespace::new validation of the model-supplied namespace before any ConfinedVault call, a documented {\"found\":false} not-found result for vault_get, and every VaultError rendered as a tool-result error naming the requested/granted namespaces"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/arsenal/vault_tools.rs#tests (6 tests: vault_get_returns_the_value_for_a_granted_namespace, vault_get_returns_a_documented_not_found_result, vault_put_stores_within_the_grant, tools_declare_json_schemas_for_their_arguments, a_malformed_namespace_argument_is_a_typed_tool_error [covers non-array, empty array, `..`, empty segment, over-long segment individually], vault_tools_are_not_listed_without_a_grant [decorator-level: an out-of-grant call is denied])"
        status: pass
      - kind: other
        ref: "grep -c parameters src/application/services/arsenal/vault_tools.rs >= 2; grep -c 'Namespace::new|Namespace::parse' src/application/services/arsenal/vault_tools.rs >= 1"
        status: pass
    human_judgment: false
  - id: D4
    description: "PaladinExecutionService::enable_vault_tools() is opt-in (off by default) and effective_arsenal() resolves what a run dispatches through: unchanged when the flag is off or the run has no grant (vault tools not listed and no vault call reachable for that run), else a CompositeArsenalPort of the configured arsenal plus a fresh VaultTools scoped to that run's own grant; the ADR-0039 HTTP-topology consequence is recorded in enable_vault_tools's own rustdoc"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests (3 tests: vault_tools_are_not_listed_without_a_grant, vault_tools_are_opt_in, enable_vault_tools_composes_with_an_existing_arsenal)"
        status: pass
      - kind: other
        ref: "grep -c 'pub fn enable_vault_tools(' src/application/services/paladin/paladin_execution_service.rs == 1; grep -c '0039|no Arsenal' src/application/services/paladin/paladin_execution_service.rs >= 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "PRD 05 section 3.4's attack test proven end to end (scripted model -> reasoning loop -> composite arsenal -> VaultTools -> ConfinedVault -> store): a hostile vault_put to a sibling, a string-prefix-lookalike sibling, and a parent namespace are each denied with the backing store's call count asserted exactly 0 via a counting VaultPort wrapper; a `..`/empty/over-long namespace segment fails Namespace::new before ConfinedVault is ever consulted (a different, documented layer); a denied call does not poison the run -- a subsequent in-grant vault_put still lands"
    requirement: "RT-04"
    verification:
      - kind: integration
        ref: "tests/integration/vault_confinement_test.rs (hostile_tool_call_to_a_sibling_namespace_is_denied, hostile_tool_call_to_a_lookalike_sibling_is_denied, hostile_tool_call_to_the_parent_is_denied, a_traversal_segment_never_constructs, a_granted_call_still_works_after_a_denied_one)"
        status: pass
    human_judgment: false
  - id: D6
    description: "X-05 concurrency obligation: 5 concurrent runs under distinct grants, sharing one Vault backend, each invoking vault_put 20 times, produce exactly 20 records per namespace and every record's value matches only its own run -- multi-thread flavor with a 30s timeout guard and exact-count/exact-value assertions"
    requirement: "RT-04"
    verification:
      - kind: integration
        ref: "tests/integration/vault_confinement_test.rs#concurrent_confined_tool_writes_produce_zero_cross_namespace_records (#[tokio::test(flavor = \"multi_thread\", worker_threads = 4)], tokio::time::timeout(30s) guard)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Full workspace gates stay green: cargo check --workspace --all-targets --all-features, cargo fmt --all --check, cargo clippy --workspace --all-targets --all-features -- -D warnings, cargo test -p paladin-ai --lib (681/681), cargo test -p paladin-ai --doc (126/126, 18 ignored pre-existing), cargo test --test lib vault_confinement (6/6)"
    requirement: "RT-04"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features (exit 0); cargo fmt --all --check (exit 0); cargo clippy --workspace --all-targets --all-features -- -D warnings (exit 0)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai --lib (681 passed, 0 failed); cargo test -p paladin-ai --doc (126 passed, 0 failed, 18 ignored -- pre-existing)"
        status: pass
      - kind: integration
        ref: "cargo test --test lib vault_confinement (6 passed, 0 failed)"
        status: pass
    human_judgment: false

duration: ~2h 15min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 16: InProcessArsenal, CompositeArsenalPort and the Vault Tools Attack Test Summary

**An in-process `ArsenalPort` and a first-wins composite give an agent hands to actually run a Rust closure, `vault_get`/`vault_put` Armaments read and write only a run's granted Vault subtree, and an end-to-end attack test proves a hostile namespace never touches the store -- with a call-counting double, not an inference.**

## Performance

- **Duration:** ~2h 15min
- **Tasks:** 3 (all `tdd="true"`, combined RED+GREEN per commit)
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- `InProcessArsenal` (`src/application/services/arsenal/in_process_arsenal.rs`) is a public `ArsenalPort` over registered `Armament` definitions plus boxed async closures -- the in-process counterpart of `ArsenalExecutionService`'s MCP-routed `invoke` (every other shipped `ArsenalPort` in the tree routes to an MCP client). `validate_call` reuses `paladin_core::platform::container::structured::shape_check` (plan 26-12) rather than a second validator; `invoke` deliberately carries no `catch_unwind` around a handler (documented house rule: a panicking handler is a bug to fix, not a condition this port converts into a value); a handler's `Err` becomes a failed `ArmamentResult`, never a propagated `Err` or a panic.
- `CompositeArsenalPort` (`src/application/services/arsenal/composite_arsenal.rs`) unions several `ArsenalPort`s behind one, following `fallback.rs`'s chain-of-ports-as-one-port shape. `list_armaments` unions with first-registration-of-a-name-wins and logs every duplicate at `warn` naming both the tool and the member index; `invoke`/`validate_call` route by asking each member's own synchronous `validate_call` whether it returns `ToolNotFound` (working around `validate_call`'s `fn`, not `async fn`, signature, which rules out awaiting each member's `list_armaments` from inside it) -- the FIRST member that recognizes the tool wins, whether Ok or a real validation error. An empty composite lists nothing and denies every call with a typed `ArsenalError::ToolNotFound`, proven so it never silently no-ops.
- `VaultTools::new(confined) -> InProcessArsenal` (`src/application/services/arsenal/vault_tools.rs`) builds `vault_get`/`vault_put` with JSON-Schema `parameters` naming `namespace`/`key`(/`value`). The `namespace` argument is **absolute**, not relative to the grant (so the PRD's attack test can literally name a sibling namespace). Both handlers construct a `Namespace` through `Namespace::new` before ever calling `ConfinedVault` -- a `..`/empty/over-long segment or an empty segment list is rejected by the type's own invariants, never reaching the confinement check. `vault_get` returns a documented `{"found":false}` not-found shape (never an error) written once as a private const and quoted verbatim in the armament's description; every `VaultError` becomes a tool-result error text naming the requested/granted namespaces.
- `PaladinExecutionService::enable_vault_tools()` is the opt-in flag (off by default, matching the PRD). A new private `effective_arsenal(confined_vault: Option<&ConfinedVault>)` resolves, per run, what the tool-call branch actually dispatches through: unchanged (the configured arsenal, possibly `None`) when the flag is off or the run has no grant; a fresh `CompositeArsenalPort` of the configured arsenal plus a `VaultTools` scoped to that run's own `ConfinedVault` when both are true. `enable_vault_tools`'s rustdoc records the ADR-0039 consequence -- HTTP-served agents have no Arsenal at all, so vault tools are unreachable over `/v1/agents/*` by construction, stated rather than worked around.
- `tests/integration/vault_confinement_test.rs` proves PRD 05 section 3.4's attack test through the WHOLE path (a scripted `MockLlmAdapter`, the real reasoning loop, the composite arsenal, `VaultTools`, `ConfinedVault`, a call-counting `InMemoryVault` wrapper) rather than re-testing the decorator in isolation (already covered by plan 26-13): a sibling, a string-prefix-lookalike sibling, and a parent namespace are each denied with the store's call count asserted **exactly 0**; a traversal/empty/over-long segment fails at a documented different layer (`Namespace::new`, before confinement is even consulted); a denied call does not poison the run. `concurrent_confined_tool_writes_produce_zero_cross_namespace_records` runs 5 concurrent grants against one shared backend, 20 `vault_put` calls each, under `#[tokio::test(flavor = "multi_thread")]` with a 30s timeout guard and exact per-namespace count/value assertions (X-05, D-39).

## Task Commits

1. **Task 1: InProcessArsenal and CompositeArsenalPort**
   - `dfe4ec12` (feat) -- both types, registered in `arsenal/mod.rs`, exported from `src/lib.rs`; 7 tests (4 + 3)
2. **Task 2: VaultTools and the opt-in wiring**
   - `5d4b809d` (feat) -- `VaultTools`, `PaladinExecutionService::enable_vault_tools`/`effective_arsenal`, wired ahead of the existing tool-call arsenal branch; 9 tests (6 in `vault_tools.rs` + 3 wiring tests in `paladin_execution_service.rs`)
3. **Task 3: The PRD namespace-confinement attack test and the concurrency sweep**
   - `4e005d1f` (test) -- `tests/integration/vault_confinement_test.rs`, registered in `tests/integration/mod.rs`; 6 tests

Both Task 1 and Task 2 combine their RED+GREEN cycle into one `feat` commit (tests and implementation authored together), matching the precedent already recorded in `26-11-SUMMARY.md` and `26-15-SUMMARY.md` for this phase. Task 3 is a pure `test` commit (no production code changed).

**Plan metadata:** this commit (SUMMARY.md)

## Files Created/Modified

- `src/application/services/arsenal/in_process_arsenal.rs` (new) -- `InProcessArsenal`, `ToolHandler`, 4 tests
- `src/application/services/arsenal/composite_arsenal.rs` (new) -- `CompositeArsenalPort`, 3 tests
- `src/application/services/arsenal/vault_tools.rs` (new) -- `VaultTools`, armament builders, namespace/key parsing helpers, `render_vault_error`, 6 tests
- `src/application/services/arsenal/mod.rs` -- registers all three new modules
- `src/lib.rs` -- exports `InProcessArsenal`/`CompositeArsenalPort` from the facade's stable public API
- `src/application/services/paladin/paladin_execution_service.rs` -- `vault_tools_enabled` field, `enable_vault_tools()`, `effective_arsenal()`, wired into `execute_internal` ahead of the tool-call dispatch site; 3 new tests
- `tests/integration/vault_confinement_test.rs` (new) -- `CountingVault`, 6 tests
- `tests/integration/mod.rs` -- registers the new test module

## Decisions Made

- **`CompositeArsenalPort` routes by asking each member's synchronous `validate_call` for `ToolNotFound`**, not by awaiting each member's async `list_armaments` from inside the (necessarily synchronous) `validate_call` trait method -- a structural constraint, not a style choice, and it happens to implement first-registration-wins for free.
- **`VaultTools::new` is a factory returning `InProcessArsenal`, not `Self`** -- annotated `#[allow(clippy::new_ret_no_self)]` with an explaining comment, keeping the plan's literal `VaultTools::new(confined) -> InProcessArsenal` signature rather than renaming the method.
- **`effective_arsenal` is computed from a clone of `confined_vault` taken BEFORE it moves into `middleware_cx.vault`** -- so the tool-call branch and `VaultRecallMiddleware` (plan 26-15) read the exact same grant for one run, never two independently-resolved handles that could in principle diverge.
- **Tool handlers return `Result<Value, String>`, not a typed error** -- `VaultError` has no natural `ArsenalError` variant, and forcing one would be an arbitrary, often-wrong mapping; `InProcessArsenal::invoke` is the single place `Err(message)` becomes a failed `ArmamentResult`.

## Deviations from Plan

None requiring a Rule 1-4 classification -- no production-code bug, missing functionality, blocking issue, or architectural change was found beyond what the plan's own `<action>` text anticipated (the `CompositeArsenalPort` routing mechanism and the `Result<Value, String>` handler signature are both implementation choices within the plan's stated shape, not deviations from it).

## Issues Encountered

- **Test-design discovery, not a production bug:** `PaladinExecutionService::execute_internal` overwrites `accumulated_output` (not accumulates it) at the start of every loop iteration -- `accumulated_output = response_view.content;` runs before that iteration's tool result is appended, and the FINAL loop's text is the only one that survives into the returned `PaladinResult.output`. The first draft of the attack tests placed the hostile tool call on a non-final loop (followed by a trailing `Text` "done" response) and asserted the denial text against the final `result.output`, which does not contain it -- the denial text is real (visible via `--nocapture` debug output during authoring) but gets overwritten by the next loop's response before the run returns. Fixed by putting each attack's tool call on the run's LAST loop (`max_loops` sized to match), so its appended tool-error text is what the run actually returns; Test 5 (`a_granted_call_still_works_after_a_denied_one`) instead verifies the denial's effect directly against the backend (no `bob` record) since its allowed call is what occupies the final loop. Documented in the test file's own comments so a future reader does not rediscover this by a second failing run.
- No other issues.

## Known Stubs

None. `InProcessArsenal`, `CompositeArsenalPort`, `VaultTools`, `PaladinExecutionService::enable_vault_tools`/`effective_arsenal`, and the attack/concurrency tests are all fully implemented per the plan's `<action>`/`<done>` clauses, with real (not placeholder) tests for every `<behavior>` item across all three tasks.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `InProcessArsenal`/`CompositeArsenalPort`/`VaultTools`/`PaladinExecutionService::enable_vault_tools` are locked in their final public shape. Plan 26-19's `ToolCallProtocolMiddleware` (once it lands) can drive the same tool-call dispatch path this plan's tests exercised through `MockLlmAdapter::with_script` -- no seam change needed on this plan's side; the substitution is documented in `tests/integration/vault_confinement_test.rs`'s own module doc.
- Plan 26-19's `ToolResultFormatter::format_error` (redact-then-bound formatting for every tool error) will eventually own the formatting `vault_tools.rs`'s `render_vault_error` currently does inline, minimally and without store internals -- `render_vault_error` is a small, isolated function a future plan can either call into `format_error` or replace outright.
- Any future built-in in-process tool (not just Vault) can reuse `InProcessArsenal`'s `Result<Value, String>` handler contract and `CompositeArsenalPort`'s composition rule without new plumbing.
- No blockers. `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 9 worktree agents complete.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `src/application/services/arsenal/in_process_arsenal.rs` -- FOUND, contains `pub struct InProcessArsenal` and `impl ArsenalPort for InProcessArsenal`
- `src/application/services/arsenal/composite_arsenal.rs` -- FOUND, contains `pub struct CompositeArsenalPort` and `impl ArsenalPort for CompositeArsenalPort`
- `src/application/services/arsenal/vault_tools.rs` -- FOUND, contains `pub struct VaultTools` and `pub fn new(`
- `tests/integration/vault_confinement_test.rs` -- FOUND, registered in `tests/integration/mod.rs`
- `src/application/services/paladin/paladin_execution_service.rs` -- FOUND, contains `pub fn enable_vault_tools(`
- `src/lib.rs` -- FOUND, exports `InProcessArsenal` and `CompositeArsenalPort`
- Commit `dfe4ec12` -- FOUND in `git log --oneline`
- Commit `5d4b809d` -- FOUND in `git log --oneline`
- Commit `4e005d1f` -- FOUND in `git log --oneline`
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo test -p paladin-ai --lib` -- 681 passed, 0 failed
- `cargo test -p paladin-ai --doc` -- 126 passed, 0 failed, 18 ignored (pre-existing)
- `cargo test --test lib vault_confinement` -- 6 passed, 0 failed
- `cargo test -p paladin-ai --lib arsenal` -- 57 passed, 0 failed
