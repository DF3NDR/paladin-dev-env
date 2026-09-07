---
phase: 26-agent-runtime-enhancements
plan: 13
subsystem: agent-runtime
tags: [vault, namespace-confinement, run-scope, war-engine, node-context, hexagonal-architecture, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 04)
    provides: "Namespace/VaultRecord/ScoredVaultRecord/Page/VaultError, VaultPort, Namespace::is_prefix_of -- the confinement primitive this plan's ConfinedVault is built on"
  - phase: 26-agent-runtime-enhancements (plan 09)
    provides: "SqliteVault/SemanticVault -- concrete VaultPort adapters a host can pass to WarEngine::with_vault / PaladinExecutionService::with_vault"
  - phase: 26-agent-runtime-enhancements (plan 11)
    provides: "The paladin_execution_service.rs shape (with_* builder methods, execute/execute_observed/execute_bounded funnel) this plan's execute_scoped refactor extends"
provides:
  - "ConfinedVault { inner, granted } in paladin_ports::output::vault_confined -- the Vault's enforcement point: every VaultPort method denies a namespace outside its grant BEFORE touching inner, proven by a zero-inner-call assertion"
  - "RunScope { vault_namespace } in paladin_core::platform::container::run_scope -- non_exhaustive + Default, the host grant that travels into a run"
  - "PaladinPort::execute_scoped -- a second defaulted trait method (after Phase 25's execute_observed) whose default body delegates to execute_observed, correctly claiming no scoped capability"
  - "PaladinExecutionService::execute_scoped / with_vault / confined_vault -- the run's grant resolves to scope.vault_namespace, else the service default, else NO grant (never a root-granted fallback)"
  - "WarEngine::with_vault(vault, base) -- grants base to every node of every run on that engine via NodeContext.vault; NodeSpec::Paladin nodes receive the same grant through execute_scoped"
  - "NodeContext.vault: Option<ConfinedVault> + vault() accessor -- PartialEq compares by granted namespace, not by Arc identity"
affects: [26-15, 26-16, 26-19, 26-21]

tech-stack:
  added: []
  patterns:
    - "A pure decorator over a port trait, needing no adapter dependency, lives beside the trait in the ports crate (paladin-ports) rather than in the facade or an adapter crate -- this is what lets both the facade (paladin-ai) and an inward-dependency crate (paladin-battalion) hold the same concrete type without either depending on the other"
    - "A second defaulted PaladinPort method (execute_scoped) whose default delegates to the FIRST defaulted method (execute_observed), not to the base execute -- a chain of correct, capability-claiming defaults rather than every method reimplementing the fallback"
    - "Resolution order ending explicitly at 'no value', never a synthesized default with elevated privilege (no grant means denied, never root) -- the same shape D-08's StopReason asymmetry and D-19's Namespace validation use: an explicit absence branch, not a convenient fallback"

key-files:
  created:
    - crates/paladin-core/src/platform/container/run_scope.rs
    - crates/paladin-ports/src/output/vault_confined.rs
  modified:
    - crates/paladin-core/src/lib.rs
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - src/application/services/paladin/vault_confined.rs
    - src/application/services/paladin/mod.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/prelude.rs
    - crates/paladin-battalion/src/engine/node.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - MIGRATION.md

key-decisions:
  - "ConfinedVault moved from the facade (src/application/services/paladin/vault_confined.rs, per Task 1's literal file assignment) to paladin-ports (crates/paladin-ports/src/output/vault_confined.rs) -- Rule 3 blocking-issue fix. paladin-battalion depends only on paladin-core + paladin-ports and NEVER on the facade crate (paladin-ai), which itself depends on paladin-battalion; Task 3 requires NodeContext/WarEngine (paladin-battalion) to hold a ConfinedVault, which is structurally impossible if the type lives in the facade. ConfinedVault needs nothing beyond VaultPort/Namespace (already in paladin-ports/paladin-core) -- no adapter dependency -- so it fits naturally beside VaultPort. The original facade path is kept as a `pub use` re-export shim for source compatibility."
  - "PaladinExecutionService::execute_scoped takes heartbeat: Option<&HeartbeatHandle> (matching execute_bounded's existing shape), not the trait's mandatory &HeartbeatHandle -- because the inherent method must serve execute() (no heartbeat) as well as execute_observed() (Some(heartbeat)); it is a distinct, service-level entry point from PaladinPort::execute_scoped (the trait method), which PaladinExecutionService does not implement in this tree (it implements PaladinExecutorPort/StreamingExecutorPort instead, per Phase 25 precedent)."
  - "confined_vault(&self, scope) -> Option<ConfinedVault> is the single resolution point: scope.vault_namespace, else the service's with_vault default, else None -- with NO ConfinedVault constructed in the None case, so 'no grant' is structurally 'no handle exists', never a handle silently granted a root namespace. This is the literal mechanism behind D-21's 'no grant means denied, never root' rule."
  - "The engine's Vault grant (WarEngine::with_vault) is inherited wholesale by NodeSpec::Battalion child runs via ChildEngineResources.vault, following the exact precedent node_cache (plan 25-13) and every other engine resource already set -- cross-thread/cross-run memory is the stated point of D-21, so a nested child run observing the SAME grant is intentional, not an oversight."
  - "The test-only VaultPort backing plan 26-13's engine-level tests (TestVault, in crates/paladin-battalion/src/engine/mod.rs's vault_tests submodule) is a small local HashMap-backed implementor, not paladin_memory::vault::InMemoryVault -- avoids adding paladin-memory as a paladin-battalion dev-dependency for a handful of tests, mirroring the crate's existing pattern of paladin-storage/paladin-llm as dev-dependencies only, never paladin-memory."

patterns-established:
  - "A new WarEngine builder method that grants a shared resource (with_vault, following with_node_cache's exact wiring) must thread its parameter through run()/run_with_namespace()/ChildEngineResources/base_ctx as a genuinely new trailing parameter on the low-level superstep::run family -- every one of that family's ~18 call sites (4 in engine/mod.rs, 1 in engine/graph.rs, 13 in superstep.rs's own test module) needs the new argument, mechanically, the same cost every prior WarEngine engine-resource addition (node_cache, shutdown_grace) already paid."
  - "PartialEq on a decorator wrapping an Arc<dyn Port> trait object is written by hand, comparing only the decorator's OWN identifying field (here: the granted Namespace) -- this is what lets a struct holding the decorator (NodeContext) keep a derived PartialEq without hand-writing one, exactly the same shape HeartbeatHandle's own hand-written 'any two handles compare equal' PartialEq already established for this file."

requirements-completed: [RT-04]

coverage:
  - id: D1
    description: "ConfinedVault { inner: Arc<dyn VaultPort>, granted: Namespace } implements VaultPort and rejects every call whose namespace does not have granted as a SEGMENT-WISE prefix with VaultError::NamespaceDenied, proven by a call-count mock showing zero backend calls on denial (siblings, parents, unrelated namespaces, and per-method); wrapping narrows, never widens; the module doc records the rejected relative-namespace alternative"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/vault_confined.rs#tests (8 tests: confined_vault_allows_the_grant_and_its_descendants, confined_vault_denies_a_sibling_namespace, confined_vault_denies_an_unrelated_namespace, confined_vault_denies_a_parent_namespace, every_port_method_is_gated, denial_names_both_namespaces, confined_vault_is_a_vault_port_and_composes, confined_vault_equality_and_accessors)"
        status: pass
      - kind: other
        ref: "grep -c is_prefix_of vault_confined.rs >= 1; grep -v '^\\s*//' vault_confined.rs | grep -c starts_with == 0; grep -c NamespaceDenied vault_confined.rs >= 5"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunScope { vault_namespace: Option<Namespace> } is non-exhaustive with Default, lives in paladin-core beside the vault types with a with_vault_namespace builder; PaladinPort::execute_scoped is a defaulted method whose body delegates to execute_observed (a correct claim of no scoped capability, X-10.4); every existing PaladinPort implementor compiles unmodified"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/run_scope.rs#tests (3 tests) + 2 doc tests; crates/paladin-ports/src/output/paladin_port.rs#tests::paladin_port_execute_scoped_default_delegates"
        status: pass
      - kind: other
        ref: "grep -B3 'pub struct RunScope' run_scope.rs | grep -q non_exhaustive; grep -A6 'async fn execute_scoped' paladin_port.rs | grep -v '^\\s*//' | grep -c 'unimplemented!\\|todo!' == 0; cargo check --workspace --all-targets --all-features exits 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "PaladinExecutionService::execute_scoped is the real entry point execute/execute_observed both fund into with RunScope::default(); with_vault(vault, default_namespace) installs the store; confined_vault resolves scope.vault_namespace, else the service default, else NO grant -- never a root-granted fallback"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests (execute_is_execute_scoped_with_default_scope, scope_namespace_becomes_the_run_grant, service_default_namespace_applies_when_the_scope_has_none, scope_namespace_overrides_the_service_default, no_grant_means_denied_not_root, no_vault_store_means_no_confined_vault_regardless_of_scope -- 6 tests)"
        status: pass
    human_judgment: false
  - id: D4
    description: "WarEngine::with_vault(vault, base) grants base to every node of every run on the engine via NodeContext.vault/vault(); a NodeSpec::Paladin node receives the same grant through the now-execute_scoped Paladin-arm dispatch; an engine without with_vault gives every node None, never a root-granted handle; NodeContext's PartialEq compares the vault field by granted namespace"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::vault_tests (engine_grants_the_base_namespace_to_every_node, a_state_node_can_read_and_write_within_its_grant, a_paladin_node_receives_the_same_grant_through_execute_scoped, an_engine_without_with_vault_gives_nodes_no_vault -- 4 tests); crates/paladin-battalion/src/engine/node.rs#tests::node_context_equality_compares_the_grant"
        status: pass
      - kind: other
        ref: "grep -c 'pub fn vault(' node.rs == 1; grep -c 'pub fn with_vault(' mod.rs == 1; grep -c execute_scoped superstep.rs >= 1; grep -v '^\\s*//' superstep.rs | grep -c 'execute_observed(' == 0 (the one remaining match is a test fn NAME, not a call -- verified by direct grep -n)"
        status: pass
    human_judgment: false
  - id: D5
    description: "N=5 concurrent WarEngine runs under distinct grants, sharing one backend, each writing 20 records; a multi-thread-flavor test with a 30s timeout guard proves every namespace holds exactly 20 records and every record's value is that namespace's own run index -- zero cross-namespace records under real concurrency"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::vault_tests::concurrent_confined_writes_produce_zero_cross_namespace_records (#[tokio::test(flavor = \"multi_thread\", worker_threads = 4)], tokio::time::timeout(30s) guard, exact-count + exact-value assertions per namespace)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Full workspace gates stay green after every change: cargo check/clippy/fmt across the whole workspace, and the complete test suites of all four touched crates"
    requirement: "RT-04"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --all --check; cargo doc -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-ai --no-deps (zero vault/run_scope/confined-related warnings; pre-existing unrelated warnings untouched)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib (524 passed); cargo test -p paladin-ports --lib (148 passed); cargo test -p paladin-battalion --lib (727 passed, 0 filtered); cargo test -p paladin-ai --lib (640 passed)"
        status: pass
    human_judgment: false

duration: ~2h 45min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 13: RunScope, ConfinedVault and WarEngine Vault Wiring Summary

**The Vault's access-control half of RT-04: `ConfinedVault` denies every out-of-grant namespace before touching the backend, `RunScope` carries a host grant through a second defaulted `PaladinPort::execute_scoped` method, and `WarEngine::with_vault`/`NodeContext::vault()` let an engine node read and write memory within its subtree -- with a mid-plan architectural fix moving `ConfinedVault` from the facade to `paladin-ports` so the engine crate can actually hold one.**

## Performance

- **Duration:** ~2h 45min
- **Tasks:** 3 (all `tdd="true"`), plus one Rule-3 architectural fix between Tasks 1 and 3
- **Files modified:** 14 (2 created, 12 modified)

## Accomplishments

- `ConfinedVault { inner: Arc<dyn VaultPort>, granted: Namespace }` (`crates/paladin-ports/src/output/vault_confined.rs`) implements `VaultPort` and gates **every** method (`put`/`get`/`delete`/`list`/`search`) with `Namespace::is_prefix_of` as the FIRST statement, denying siblings, parents and unrelated namespaces with `VaultError::NamespaceDenied { requested, granted }` before the inner port is ever touched -- proven by an 8-test suite including a call-count mock showing zero backend calls on every denial, and a composition test proving a `ConfinedVault` wrapping another `ConfinedVault` narrows, never widens.
- `RunScope { vault_namespace: Option<Namespace> }` (`crates/paladin-core/src/platform/container/run_scope.rs`) is `#[non_exhaustive]` with `Default` and a `with_vault_namespace` builder, re-exported from `paladin-core`'s crate prelude. `PaladinPort` (`crates/paladin-ports/src/output/paladin_port.rs`) gains a SECOND defaulted method, `execute_scoped`, whose default body delegates to `execute_observed` (Phase 25's own defaulted method) -- a correct claim of no scoped capability, so every existing `PaladinPort` implementor across the workspace compiles and behaves unchanged.
- `PaladinExecutionService` gains `with_vault(vault, default_namespace)`, `confined_vault(&scope) -> Option<ConfinedVault>` and the real `execute_scoped` entry point; `execute`/`execute_observed` are now `execute_scoped` with `RunScope::default()` (one implementation, three doors). The grant resolves in order -- `scope.vault_namespace`, else the service default, else **no grant at all**, with no `ConfinedVault` constructed in that last case -- so "no grant" is structurally "no handle exists," never a handle silently scoped to a root namespace.
- `WarEngine::with_vault(vault, base)` (`crates/paladin-battalion/src/engine/mod.rs`) grants `base` to every node of every run on the engine, including `NodeSpec::Battalion` child runs (inherited wholesale via `ChildEngineResources.vault`, the same pattern `node_cache` already established). `NodeContext` (`crates/paladin-battalion/src/engine/node.rs`) gains `vault: Option<ConfinedVault>` + a `vault()` accessor, with a one-line comment recording that `BATTLEFIELD_SCHEMA_VERSION` does NOT bump (the field is a live handle, never serialized). The superstep engine's Paladin arm (`crates/paladin-battalion/src/engine/superstep.rs`) now dispatches through `execute_scoped`, carrying a `RunScope` built from `ctx.vault`'s granted namespace, instead of `execute_observed`.
- **Mid-plan architectural fix (Rule 3):** Task 1 originally placed `ConfinedVault` in the facade crate (`src/application/services/paladin/vault_confined.rs`), but `paladin-battalion` (Task 3's crate) depends only on `paladin-core` + `paladin-ports` and never on the facade -- which itself depends on `paladin-battalion`. Holding a facade-defined type in `NodeContext` was structurally impossible. `ConfinedVault` was moved to `crates/paladin-ports/src/output/vault_confined.rs` (it needs nothing beyond `VaultPort`/`Namespace`, both already there); the original facade path became a `pub use` re-export shim, so `crate::application::services::paladin::vault_confined::ConfinedVault` and the facade prelude export both still resolve.
- A new `vault_tests` submodule in `crates/paladin-battalion/src/engine/mod.rs` proves the full stack end-to-end through real `WarEngine::start` runs: a base-namespace grant reaching every node, a `StateNode` write-within-grant/denied-outside-grant boundary, a `NodeSpec::Paladin` node's `execute_scoped` dispatch carrying the grant (via a scope-recording `PaladinPort` test double), an engine with no `with_vault` call giving every node `None`, and a `#[tokio::test(flavor = "multi_thread")]` stress test proving 5 concurrent engine runs under distinct grants, sharing one backend, produce zero cross-namespace records.

## Task Commits

1. **Task 1: ConfinedVault — the enforcement point**
   - RED: `fe1c030d` (test: failing denial/composition tests against a deliberately no-op `check()`)
   - GREEN: `1771145e` (feat: the real segment-wise gate, all 8 tests passing)
2. **Task 2: RunScope, execute_scoped on the service, and the defaulted PaladinPort method**
   - RED: `40e7697e` (test: failing `no_grant_means_denied_not_root` against a deliberate root-namespace fallback)
   - GREEN: `9a0c45dc` (feat: the real "ends at None, never root" resolution, all tests passing)
3. **Mid-plan fix: move ConfinedVault to paladin-ports (Rule 3, architectural blocker)**
   - `8899eb96` (fix: relocate the type so paladin-battalion can hold it without an illegal reverse dependency)
4. **Task 3: Engine wiring — WarEngine::with_vault, NodeContext::vault(), and the confinement stress test**
   - RED: `5917b598` (test: failing engine-level vault tests against `base_ctx.vault` deliberately hardcoded to `None`)
   - GREEN: `e38bcd72` (feat: the real `vault.clone()` wiring, all 6 new tests + 466 pre-existing engine tests passing)
5. **Follow-up fix:** `24b402e9` (docs: corrected a stale `execute_observed` dispatch comment on a pre-existing test whose name/doc referenced the old dispatch method, found via the acceptance-criteria grep sweep)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/run_scope.rs` (new) -- `RunScope`, `with_vault_namespace`, 3 tests + 2 doc tests
- `crates/paladin-core/src/platform/container/mod.rs` -- `pub mod run_scope;`
- `crates/paladin-core/src/lib.rs` -- prelude re-export of `RunScope`
- `crates/paladin-ports/src/output/vault_confined.rs` (new) -- `ConfinedVault`, `impl VaultPort for ConfinedVault`, 8 tests (moved here from the facade per the mid-plan fix)
- `crates/paladin-ports/src/output/mod.rs` -- `pub mod vault_confined;`
- `crates/paladin-ports/src/output/paladin_port.rs` -- `PaladinPort::execute_scoped` (defaulted), 1 test
- `src/application/services/paladin/vault_confined.rs` -- now a `pub use paladin_ports::output::vault_confined::ConfinedVault;` re-export shim
- `src/application/services/paladin/mod.rs` -- `pub mod vault_confined;` (unchanged position, content changed)
- `src/application/services/paladin/paladin_execution_service.rs` -- `vault`/`default_vault_namespace` fields, `with_vault`, `confined_vault`, `execute_scoped`; `execute`/`execute_observed` refactored to delegate; 7 new tests
- `src/prelude.rs` -- re-export of `ConfinedVault` from the facade path
- `crates/paladin-battalion/src/engine/node.rs` -- `NodeContext.vault` field + `vault()` accessor, schema-version-decision comment, 1 test
- `crates/paladin-battalion/src/engine/hooks.rs` -- test helper `ctx()` updated for the new field
- `crates/paladin-battalion/src/engine/graph.rs` -- one test call site updated for the new `run()` trailing parameter
- `crates/paladin-battalion/src/engine/mod.rs` -- `WarEngine.vault` field, `with_vault`, 4 call sites forwarding `self.vault.clone()`, new `vault_tests` submodule (6 tests)
- `crates/paladin-battalion/src/engine/superstep.rs` -- `run`/`run_with_namespace` gain a trailing `vault` parameter (13 test call sites updated), `ChildEngineResources.vault`, `base_ctx.vault`, Paladin arm dispatches `execute_scoped`
- `MIGRATION.md` -- `PaladinPort` row gains the `execute_scoped` default-method line (marked `N`)

## Decisions Made

- **`ConfinedVault` lives in `paladin-ports`, not the facade** -- see the mid-plan fix above; documented in the module's own rustdoc so a future reader does not have to rediscover the dependency-direction reasoning.
- **`PaladinExecutionService::execute_scoped`'s `heartbeat` parameter is `Option<&HeartbeatHandle>`**, matching `execute_bounded`'s existing shape, not the trait's mandatory `&HeartbeatHandle` -- the inherent method serves both `execute()` (no heartbeat) and `execute_observed()` (`Some`), and is a distinct entry point from the trait method (`PaladinExecutionService` implements `PaladinExecutorPort`, not `PaladinPort`, in this tree).
- **The Vault grant is inherited wholesale by `NodeSpec::Battalion` child runs**, following `node_cache`'s exact precedent -- cross-thread/cross-run memory is D-21's stated point, so this is intentional, not an oversight.
- **Test-only `TestVault`, not `paladin_memory::vault::InMemoryVault`**, backs the new engine-level tests -- avoids adding `paladin-memory` as a `paladin-battalion` dev-dependency (which currently has only `paladin-storage`/`paladin-llm` as dev-dependencies) for a handful of tests.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking Issue] Moved `ConfinedVault` from the facade to `paladin-ports`**
- **Found during:** Starting Task 3 (reading `paladin-battalion`'s `Cargo.toml` to plan `NodeContext.vault`'s type)
- **Issue:** Task 1's plan text places `ConfinedVault` at `src/application/services/paladin/vault_confined.rs` (the facade crate, `paladin-ai`). Task 3 requires `paladin-battalion`'s `NodeContext`/`WarEngine` to hold a `ConfinedVault`. But `paladin-battalion` depends only on `paladin-core` and `paladin-ports` -- confirmed by reading its `Cargo.toml`, which explicitly comments "no cycle" beside its two dev-dependencies (`paladin-storage`, `paladin-llm`) precisely because neither depends back on `paladin-battalion`. The facade crate, in contrast, DOES depend on `paladin-battalion` (it is the top-level application crate). A type defined in the facade is therefore unreachable from `paladin-battalion` without introducing an illegal reverse/circular dependency -- this is not a workaround-able type error but a structural impossibility.
- **Fix:** Moved `ConfinedVault`'s full implementation and its 8-test suite to `crates/paladin-ports/src/output/vault_confined.rs`, registered via `pub mod vault_confined;` in `paladin-ports`'s `output/mod.rs`. `ConfinedVault` needs nothing beyond `VaultPort` and `Namespace`, both already in `paladin-ports`/`paladin-core` -- no adapter dependency was traded away. The original facade file became a one-line `pub use paladin_ports::output::vault_confined::ConfinedVault;` re-export shim, so any code written against the plan's originally-sketched path (including the facade prelude's own re-export) keeps compiling unchanged.
- **Files modified:** `crates/paladin-ports/src/output/vault_confined.rs` (new, real implementation), `crates/paladin-ports/src/output/mod.rs`, `src/application/services/paladin/vault_confined.rs` (became a shim)
- **Verification:** `cargo check --workspace --all-targets --all-features` exits 0; `cargo test -p paladin-ports --lib vault_confined` (8 passed); `cargo test -p paladin-ai --lib vault_confined` now runs 0 tests at the facade path (the shim has none) -- this is the plan's own literal Task-1 verification command for that path, and it is a harmless consequence of the move, not a regression: the real 8-test coverage moved with the implementation.
- **Committed in:** `8899eb96`

**2. [Rule 1 - Bug/Accuracy] Corrected a stale doc comment on a pre-existing test**
- **Found during:** Final acceptance-criteria grep sweep (`grep -v '^\s*//' superstep.rs | grep -c 'execute_observed('` expected `0`, found `1`)
- **Issue:** The one remaining match was the test function NAME `the_engine_always_calls_execute_observed` (a substring match, not a call), whose doc comment also asserted the engine dispatches through `execute_observed` -- true before this plan, no longer true now that the Paladin arm calls `execute_scoped`. The test itself still passes for a documented reason: its `ObservedCallRecordingPort` double overrides `execute_observed`, not `execute_scoped`, so `execute_scoped`'s default body (which delegates to `execute_observed`) still reaches it.
- **Fix:** Updated the doc comment to describe the actual dispatch chain (`execute_scoped` -> default delegates to `execute_observed`, which the recording double overrides). No behavior change; the test's own assertions and name were left as-is (renaming a passing pre-existing test is out of this plan's scope).
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
- **Verification:** `cargo test -p paladin-battalion --lib the_engine_always_calls_execute_observed` exits 0 (unchanged pass); `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` both clean
- **Committed in:** `24b402e9`

---

**Total deviations:** 2 auto-fixed (1 Rule 3 architectural/blocking fix, 1 Rule 1 doc-accuracy fix)
**Impact on plan:** The Rule 3 fix changed WHERE `ConfinedVault` lives (not its shape, API, or test coverage) and was necessary for Task 3 to compile at all; every acceptance criterion that names `ConfinedVault`'s behavior is still met, just at `crates/paladin-ports/src/output/vault_confined.rs` instead of the facade path the plan originally named. The Rule 1 fix was cosmetic. No scope creep beyond these two.

## Issues Encountered

- **The plan's low-level `superstep::run`/`run_with_namespace` free functions have ~18 call sites** (4 in `engine/mod.rs`, 1 in `engine/graph.rs`, 13 inside `superstep.rs`'s own test module) that all needed the new trailing `vault` parameter -- the same mechanical cost every prior engine-resource addition (`node_cache`, `shutdown_grace`) already paid, per those additions' own code comments. Handled via a verified Python paren-matching pass (with a first attempt that double-inserted at 7 sites, caught by a spot-check, corrected by reverting `superstep.rs` to its last commit and redoing the pass in one corrected script run) rather than by hand, given the volume; every insertion point was individually verified against the actual closing-paren line before and after.
- No other issues beyond the deviations documented above.

## Known Stubs

None. `ConfinedVault`, `RunScope`, `PaladinExecutionService::execute_scoped`/`with_vault`/`confined_vault`, and `WarEngine::with_vault`/`NodeContext::vault()` are all fully implemented per the plan's `<action>`/`<done>` clauses, with real (not placeholder) tests for every `<behavior>` item across all three tasks.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `ConfinedVault`/`RunScope`/`execute_scoped`/`WarEngine::with_vault`/`NodeContext::vault()` are locked in their final public shape at `crates/paladin-ports/src/output/vault_confined.rs` and `crates/paladin-core/src/platform/container/run_scope.rs` -- plan 26-16's Armament-level attack test (`vault_get`/`vault_put` tools) and plan 26-15's arsenal wiring can build directly on `PaladinExecutionService::confined_vault` without further seam changes.
- The facade re-export shim at `src/application/services/paladin/vault_confined.rs` means any plan or doc written against that original path keeps working; new code should prefer `paladin_ports::output::vault_confined::ConfinedVault` directly.
- `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 7 worktree agents complete.
- No blockers for later waves. The one thing a reader of a future plan should know: `ConfinedVault`'s crate location moved mid-phase from what 26-CONTEXT.md's Integration Points list implies (`src/application/services/paladin/vault_confined.rs`) to `crates/paladin-ports/src/output/vault_confined.rs` -- both paths resolve to the same type via the shim, but new code should use the `paladin-ports` path.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `crates/paladin-core/src/platform/container/run_scope.rs` -- FOUND, contains `pub struct RunScope` and `#[non_exhaustive]`
- `crates/paladin-ports/src/output/vault_confined.rs` -- FOUND, contains `pub struct ConfinedVault` and `impl VaultPort for ConfinedVault`
- `src/application/services/paladin/vault_confined.rs` -- FOUND, is a re-export shim (`pub use paladin_ports::output::vault_confined::ConfinedVault;`)
- `crates/paladin-ports/src/output/paladin_port.rs` -- FOUND, contains `async fn execute_scoped(`
- `src/application/services/paladin/paladin_execution_service.rs` -- FOUND, contains `pub async fn execute_scoped(`, `pub fn with_vault(`, `pub fn confined_vault(`
- `crates/paladin-battalion/src/engine/node.rs` -- FOUND, contains `vault: Option<ConfinedVault>` and `pub fn vault(`
- `crates/paladin-battalion/src/engine/mod.rs` -- FOUND, contains `pub fn with_vault(`
- Commit `fe1c030d` -- FOUND in `git log --oneline`
- Commit `1771145e` -- FOUND in `git log --oneline`
- Commit `40e7697e` -- FOUND in `git log --oneline`
- Commit `9a0c45dc` -- FOUND in `git log --oneline`
- Commit `8899eb96` -- FOUND in `git log --oneline`
- Commit `5917b598` -- FOUND in `git log --oneline`
- Commit `e38bcd72` -- FOUND in `git log --oneline`
- Commit `24b402e9` -- FOUND in `git log --oneline`
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo test -p paladin-ai-core --lib` -- 524 passed, 0 failed
- `cargo test -p paladin-ports --lib` -- 148 passed, 0 failed
- `cargo test -p paladin-battalion --lib` -- 727 passed, 0 failed, 0 filtered (full run, no test removed)
- `cargo test -p paladin-ai --lib` -- 640 passed, 0 failed
