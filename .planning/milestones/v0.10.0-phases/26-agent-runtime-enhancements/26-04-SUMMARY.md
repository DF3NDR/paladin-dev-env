---
phase: 26-agent-runtime-enhancements
plan: 04
subsystem: memory
tags: [vault, namespace, key-value-store, hexagonal-ports, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 01)
    provides: "no direct code dependency; sequenced in wave 2 alongside 26-02/26-03 per depends_on: [26-01]"
provides:
  - "Namespace, VaultRecord, ScoredVaultRecord, Page and VaultError in paladin_core::platform::container::vault, with no new paladin-core dependency (D-18, ADR-0015, ADR-0016)"
  - "VaultPort in paladin_ports::output::vault_port -- object-safe, Send+Sync, put/get/delete/list/search with a correct Unsupported default for search, re-exporting the core value types, carrying the Vault/Garrison/Waypoint three-way table on its rustdoc"
  - "InMemoryVault in paladin-memory, ungated, passing a shared nine-case VaultPort contract suite (put/get/overwrite/delete/list-scoping/prefix/pagination/isolation/value-bound) plus a search-unsupported capability assertion"
  - "Namespace::is_prefix_of -- the segment-wise confinement primitive plan 26-13's ConfinedVault decorator is built on"
affects: [26-09, 26-13, 26-15, 26-16, 26-19]

tech-stack:
  added: []
  patterns:
    - "Core value types + port trait + adapter split across three crates (paladin-core -> paladin-ports -> paladin-memory), mirroring GarrisonEntry/GarrisonPort/InMemoryGarrison exactly"
    - "Shared adapter contract suite as plain (not #[cfg(test)]) pub async fn-per-clause module, mirroring paladin-storage's node_cache::contract_tests -- instantiated unchanged by every adapter's own #[tokio::test]s"
    - "Segment-wise Vec<String> comparison for namespace confinement, never a string-prefix check on the Display form"

key-files:
  created:
    - crates/paladin-core/src/platform/container/vault.rs
    - crates/paladin-ports/src/output/vault_port.rs
    - crates/paladin-memory/src/vault/mod.rs
    - crates/paladin-memory/src/vault/in_memory.rs
    - crates/paladin-memory/src/vault/contract_tests.rs
  modified:
    - crates/paladin-core/src/lib.rs
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-memory/src/lib.rs
    - crates/paladin-memory/src/prelude.rs

key-decisions:
  - "Page::new's out-of-range limit (>1000) returns VaultError::Storage rather than a new dedicated variant -- pagination is a request-shaping concern, not a domain invariant on stored data, and the plan's own VaultError variant list (D-19) has no pagination-specific member. Documented on Page::new's own rustdoc rather than silently choosing a variant."
  - "InMemoryVault::list reads the exact joined-namespace inner BTreeMap directly (never a cross-namespace scan), so 'exactly ns, not descendants' is structural rather than a post-hoc filter -- makes the sibling/descendant contract cases (list_returns_only_this_namespace_ordered_by_key, namespaces_are_isolated) correct by construction rather than by a filter someone could later loosen."
  - "contract_tests::assert_search_unsupported is a named case in the shared suite (case 10) rather than folded into the put/get/delete/list run_all_shared_clauses rollup -- SemanticVault (plan 26-09) will call the other nine unchanged but NOT this one, since it asserts search actually works instead."

patterns-established:
  - "A new Vault adapter is not 'done' until it is instantiated against paladin-memory::vault::contract_tests's full case list (plan 26-09's SqliteVault and SemanticVault inherit this obligation)"
  - "VaultError::Storage/Serialization are the only two adapter-boundary variants carrying free-form text, and any text entering them must be redacted before it is bounded (D-34) -- stated once on VaultError's own rustdoc so no future adapter has to rediscover the rule"

requirements-completed: [RT-04]

coverage:
  - id: D1
    description: "Namespace, VaultRecord, ScoredVaultRecord, Page and VaultError exist in paladin_core::platform::container::vault with no new paladin-core dependency; VaultPort exists in paladin_ports::output::vault_port re-exporting the value types"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/vault.rs#tests (16 tests), crates/paladin-ports/src/output/vault_port.rs#tests (4 tests)"
        status: pass
      - kind: other
        ref: "git diff 45f00728..HEAD -- crates/paladin-core/Cargo.toml crates/paladin-ports/Cargo.toml Cargo.lock (0 lines changed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Namespace validates all documented invariants (1-16 segments, 1-64 chars each, no '/', not '.'/'..'', no control chars) and is_prefix_of compares segments element-by-element, provably rejecting the sibling-namespace case ['user','alice'] vs ['user','alice2']"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/vault.rs#tests::namespace_rejects_every_invalid_shape, tests::is_prefix_of_is_segment_wise_not_string_wise"
        status: pass
    human_judgment: false
  - id: D3
    description: "VaultPort is object-safe and Send+Sync, has PRD 05 §2.3's exact five methods with a correct Unsupported default for search, and its rustdoc carries the Vault/Garrison/Waypoint table with a compiled doc test naming a Namespace, a GarrisonEntry, and a ThreadId in one snippet"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/vault_port.rs#tests::vault_port_is_object_safe, tests::vault_port_is_send_and_sync, tests::search_defaults_to_unsupported"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/vault_port.rs (doc test on VaultPort trait, line 87)"
        status: pass
    human_judgment: false
  - id: D4
    description: "InMemoryVault passes a nine-case shared VaultPort contract suite (put/get/overwrite/delete/list-scoping-excludes-descendants/prefix-filter/pagination/empty-namespace/namespace-isolation/value-bound) plus the search-unsupported capability assertion, with no new cargo feature or dependency"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/vault/in_memory.rs#tests (11 tests: 9 contract clauses + search_is_unsupported_on_in_memory + run_all_shared_clauses_smoke_aggregate)"
        status: pass
      - kind: other
        ref: "git diff 45f00728..HEAD -- crates/paladin-memory/Cargo.toml Cargo.lock (0 lines changed)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Full workspace check/clippy/fmt/doc are clean with no new broken intra-doc links"
    requirement: "RT-04"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --all --check; cargo doc --workspace --no-deps (no vault-related warnings)"
        status: pass
    human_judgment: false

duration: ~1h 30min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 04: Vault Value Types, VaultPort and InMemoryVault Summary

**The Vault's vocabulary (`Namespace`/`VaultRecord`/`ScoredVaultRecord`/`Page`/`VaultError`) and its first adapter (`InMemoryVault`), with `Namespace::is_prefix_of` proven to compare path segments rather than strings on the exact sibling-namespace attack the PRD names.**

## Performance

- **Duration:** ~1h 30min
- **Tasks:** 3 (all `tdd="true"`)
- **Files modified:** 10 (5 created, 5 modified)

## Accomplishments

- `Namespace`, `VaultRecord`, `ScoredVaultRecord`, `Page` and `VaultError` now exist in `paladin_core::platform::container::vault`, with `Namespace`/`VaultRecord`/`VaultError` re-exported from `paladin-core`'s prelude (`Page` deliberately excluded, D-09) -- and `paladin-core` gained no new dependency (confirmed by `git diff` on `Cargo.toml`).
- `Namespace::is_prefix_of` compares `Vec<String>` segments element-by-element, not the joined `Display` string -- proven against the exact sibling case the PRD's attack test exists to catch: `["user","alice"].is_prefix_of(["user","alice2"])` is `false`, even though the joined string `"user/alice2"` starts with `"user/alice"`.
- `VaultPort` exists in `paladin_ports::output::vault_port` as an object-safe, `Send + Sync` async trait with PRD 05 §2.3's exact five methods (`put`/`get`/`delete`/`list`/`search`), a correct `Unsupported` default for `search` (no boilerplate required from an adapter without embeddings), and the Vault/Garrison/Waypoint three-way table on its rustdoc -- pinned by a compiled doc test that names a `Namespace`, a `GarrisonEntry`, and a `ThreadId` in one snippet, so the table cannot rot into prose that no longer matches the types.
- `InMemoryVault` (`crates/paladin-memory/src/vault/in_memory.rs`), ungated, over a `tokio::sync::RwLock<HashMap<String, BTreeMap<String, VaultRecord>>>`, passes a shared nine-case `VaultPort` contract suite (`crates/paladin-memory/src/vault/contract_tests.rs`) -- the same suite plans 26-09's `SqliteVault` and `SemanticVault` will be instantiated against, one case per behavior clause plus a tenth adapter-specific `search`-unsupported assertion.
- No new cargo feature or dependency was introduced anywhere in the plan (`paladin-core`, `paladin-ports` and `paladin-memory` `Cargo.toml`s and the workspace `Cargo.lock` are byte-identical to the plan's starting point).

## Task Commits

1. **Task 1: Vault core value types, with Namespace validation and segment-wise prefix matching**
   - RED: `ebbe3a35` (test: failing `is_prefix_of_is_segment_wise_not_string_wise` against a deliberately naive string-prefix implementation)
   - GREEN: `18dbbb78` (feat: the real segment-wise `is_prefix_of` plus the full value-type vocabulary)
2. **Task 2: VaultPort and the Vault/Garrison/Waypoint three-way table**
   - RED: `cd13be90` (test: failing `search_defaults_to_unsupported` against a deliberately wrong `Ok(vec![])` default)
   - GREEN: `d887fef8` (feat: the correct `Unsupported` default plus the trait, table, and doc test)
3. **Task 3: InMemoryVault and the shared adapter contract suite**
   - RED: `8a8bdf84` (test: failing `list_returns_only_this_namespace_ordered_by_key` against a deliberately wrong descendant-leaking `list`)
   - GREEN: `9bf57f89` (feat: the exact-namespace `list` implementation, all 11 tests passing)
   - Follow-up fix: `58a066b8` (fix: broken intra-doc link in `vault/mod.rs`, found by `cargo doc --workspace --no-deps`)

_Each task's RED/GREEN split is a real, verified failure-then-fix on the exact security- or contract-critical assertion the task exists to prove -- not an approximate split. For Task 1, the RED commit's `is_prefix_of` used `other.to_string().starts_with(&self.to_string())` (the precise anti-pattern the task's rustdoc warns against) and was confirmed failing on the sibling-namespace case before being replaced. For Task 2, the RED commit's `search` default returned `Ok(vec![])` instead of `Err(Unsupported)`, confirmed failing. For Task 3, the RED commit's `list` matched every namespace whose joined form starts with the target (wrongly including `contract-scope/a/b` when listing `contract-scope/a`), confirmed failing, before being replaced with the correct exact-namespace `HashMap` lookup

## Files Created/Modified

- `crates/paladin-core/src/platform/container/vault.rs` -- `Namespace` (validation + `is_prefix_of`), `VaultRecord`, `ScoredVaultRecord`, `Page`, `VaultError`, `DEFAULT_MAX_VALUE_BYTES`, 16 unit tests + 7 doc tests
- `crates/paladin-core/src/platform/container/mod.rs` -- `pub mod vault;` (alphabetical position, between `user_group` and `vision`)
- `crates/paladin-core/src/lib.rs` -- prelude re-export of `Namespace`/`VaultError`/`VaultRecord` (`Page` excluded, D-09)
- `crates/paladin-ports/src/output/vault_port.rs` -- `VaultPort` trait (put/get/delete/list/search), the Vault/Garrison/Waypoint table with a compiled doc test, re-exports of the core value types, 4 unit tests + 2 doc tests
- `crates/paladin-ports/src/output/mod.rs` -- `pub mod vault_port;`
- `crates/paladin-memory/src/vault/mod.rs` -- module registration, `sqlite`/`semantic` noted as plan-26-09 comments (not stub modules)
- `crates/paladin-memory/src/vault/in_memory.rs` -- `InMemoryVault`, `impl VaultPort`, 11 tests
- `crates/paladin-memory/src/vault/contract_tests.rs` -- the shared 9-clause contract suite + `assert_search_unsupported` (case 10) + `run_all_shared_clauses` aggregate
- `crates/paladin-memory/src/lib.rs` -- `pub mod vault;` + crate-doc mention
- `crates/paladin-memory/src/prelude.rs` -- `InMemoryVault` re-export

## Decisions Made

- **`Page::new`'s out-of-range `limit` (>1000) returns `VaultError::Storage`**, not a new dedicated variant. `VaultError`'s seven variants are fixed by D-19's own list and none names pagination specifically; pagination bounds are a request-shaping concern rather than a domain invariant on stored data, so the adapter-boundary `Storage` variant (with a descriptive message) was the closest correct fit. Documented directly on `Page::new`'s rustdoc rather than silently picking a variant.
- **`InMemoryVault::list` reads the exact joined-namespace `BTreeMap` directly** rather than filtering a flattened, cross-namespace iterator. This makes "exactly `ns`, never descendants" structural (the descendant simply lives in a different `HashMap` entry and is never touched) rather than a filter condition a future edit could loosen -- verified directly by the Task 3 RED/GREEN pair, where the deliberately-wrong cross-namespace-scan implementation leaked the `contract-scope/a/b` descendant into a listing of `contract-scope/a`.
- **`assert_search_unsupported` is a standalone case (10), not folded into `run_all_shared_clauses`** -- `InMemoryVault` and (plan 26-09's) `SqliteVault` call it; `SemanticVault` will not, since it asserts `search` actually works. Keeping it separate means the aggregate rollup stays meaningful across all three eventual adapters.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a `clippy::manual_range_contains` lint in the key-length validator**
- **Found during:** Task 1 verification (`cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings`)
- **Issue:** `validate_key`'s bounds check used `char_count < MIN_KEY_CHARS || char_count > MAX_KEY_CHARS`, which clippy correctly flags as a manual reimplementation of `!RangeInclusive::contains`.
- **Fix:** Rewrote as `!(MIN_KEY_CHARS..=MAX_KEY_CHARS).contains(&char_count)`.
- **Files modified:** `crates/paladin-core/src/platform/container/vault.rs`
- **Verification:** `cargo clippy -p paladin-ai-core --all-targets --all-features -- -D warnings` exits 0; all 16 `vault` unit tests still pass
- **Committed in:** `18dbbb78` (Task 1 GREEN commit)

**2. [Rule 1 - Bug] Fixed a broken intra-doc link in `vault/mod.rs`**
- **Found during:** Final plan-level verification (`cargo doc --workspace --no-deps`)
- **Issue:** The module doc used a doc-link (`` [`InMemoryVault`] ``/`` [`in_memory::InMemoryVault`] ``) to reference the sibling adapter type, which rustdoc could not resolve in this module-doc context (an unresolved intra-doc link is a `cargo doc --workspace --no-deps` warning, and the plan's own `<verification>` requires that command to be green with no new broken links).
- **Fix:** Switched to a plain code span (`` `InMemoryVault` ``), matching the exact convention already used in `crates/paladin-storage/src/node_cache/mod.rs` for referencing sibling adapter names in a module doc.
- **Files modified:** `crates/paladin-memory/src/vault/mod.rs`
- **Verification:** `cargo doc --workspace --no-deps` produces zero vault-related warnings; `cargo test -p paladin-memory --lib vault` (11/11) and `cargo clippy -p paladin-memory --all-targets --all-features -- -D warnings` both still clean
- **Committed in:** `58a066b8`

---

**Total deviations:** 2 auto-fixed (both Rule 1 bug fixes, both mechanical and non-behavioral)
**Impact on plan:** Neither changed any public API shape or test assertion. No scope creep.

## Issues Encountered

None beyond the two deviations above. Every acceptance criterion in the plan (struct/enum/method presence, `non_exhaustive` marker, the single `is_prefix_of` implementation, zero `starts_with` usages in non-comment lines of `vault.rs`, zero new dependency lines in any touched `Cargo.toml`, the five-method `VaultPort` surface, zero `unimplemented!`/`todo!` in `vault_port.rs`, the nine-plus contract-case count reported by `cargo test`) was verified directly via the exact grep/test commands the plan specifies.

## Known Stubs

None. `sqlite` and `semantic` adapters for plan 26-09 are noted only as comments in `vault/mod.rs` rustdoc, per the plan's explicit instruction not to create stub modules for unbuilt adapters.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- `Namespace`, `VaultRecord`, `ScoredVaultRecord`, `Page`, `VaultError` and `VaultPort` are locked in their final public shape; plan 26-09's `SqliteVault` and `SemanticVault` adapters, and plan 26-13's `ConfinedVault` decorator, can be built against them without further seam changes.
- `paladin_memory::vault::contract_tests` is ready to be instantiated by plan 26-09's two new adapters unchanged, with `assert_search_unsupported` (case 10) available for `SqliteVault` and deliberately not called by `SemanticVault`.
- `Namespace::is_prefix_of` is the single, tested confinement primitive plan 26-13's `ConfinedVault` decorator will call -- no adapter or decorator needs to reinvent namespace-scoping logic.
- No blockers for wave 2 or wave 3 plans.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

All created files verified present on disk (`crates/paladin-core/src/platform/container/vault.rs`,
`crates/paladin-ports/src/output/vault_port.rs`,
`crates/paladin-memory/src/vault/{mod,in_memory,contract_tests}.rs`, this SUMMARY.md); all eight
commits (`ebbe3a35`, `18dbbb78`, `cd13be90`, `d887fef8`, `8a8bdf84`, `9bf57f89`, `58a066b8`,
`0afb9e34`) verified present in `git log`.
