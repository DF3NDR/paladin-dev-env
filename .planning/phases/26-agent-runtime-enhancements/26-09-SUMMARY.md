---
phase: 26-agent-runtime-enhancements
plan: 09
subsystem: memory
tags: [vault, sqlite, sanctum, embedding, sqlx, hexagonal-ports, rust]

requires:
  - phase: 26-agent-runtime-enhancements (plan 04)
    provides: "Namespace/VaultRecord/ScoredVaultRecord/Page/VaultError, VaultPort, InMemoryVault, and the shared contract_tests suite this plan instantiates two more adapters against"
  - phase: 26-agent-runtime-enhancements (plan 07)
    provides: "crates/paladin-memory/src/migrations.rs -- the one shared embedded sqlx migrator this plan's 003 migration and SqliteVault ride"
provides:
  - "003_create_vault_tables.sql -- vault_records (ns, key, value, created_at, updated_at, PRIMARY KEY (ns, key)) plus an index on (ns, key), numbered after 002 in the one shared paladin-memory migrator sequence"
  - "SqliteVault in paladin-memory::vault -- persistent VaultPort adapter behind the existing sqlite feature, parameter-bound SQL throughout, exact-namespace list scoping, opaque-cursor pagination, idempotent construction"
  - "SemanticVault in paladin-memory::vault -- ungated VaultPort adapter composing an existing VaultPort store + SanctumPort + EmbeddingPort; gives search a real implementation with namespace confinement re-established in-process"
  - "contract_tests::search_returns_scored_records_from_the_store -- shared case 11, the first search-capable contract case, instantiated only by SemanticVault"
  - "crates/paladin-memory/src/vault/redact.rs -- generic credential-shape redact-then-bound helper (D-34) shared by SqliteVault and SemanticVault"
affects: [26-13, 26-15, 26-16, 26-19]

tech-stack:
  added: []
  patterns:
    - "A third VaultPort adapter instantiated against the same plan-26-04 contract_tests suite -- put/get/delete/list identical across InMemoryVault, SqliteVault, SemanticVault; search deliberately differs (Unsupported vs a real, re-filtered implementation)"
    - "One shared embedded sqlx migrator serving two unrelated adapters (SqliteGarrison, SqliteVault) in one crate, proven safe via IF NOT EXISTS and a same-file cross-adapter table-presence test"
    - "A composition adapter (SemanticVault) built entirely from Arc<dyn Port> trait objects, so it needs no concrete backend's cargo feature -- mirrors FallbackLlmAdapter's chain-of-ports shape"
    - "Re-establish a security invariant (namespace confinement) in the composing adapter's own code after every backend call, never trusting a backend's own filter argument"

key-files:
  created:
    - crates/paladin-memory/migrations/003_create_vault_tables.sql
    - crates/paladin-memory/src/vault/sqlite.rs
    - crates/paladin-memory/src/vault/semantic.rs
    - crates/paladin-memory/src/vault/redact.rs
  modified:
    - crates/paladin-memory/src/vault/mod.rs
    - crates/paladin-memory/src/vault/contract_tests.rs
    - crates/paladin-memory/src/prelude.rs
    - crates/paladin-memory/src/lib.rs

key-decisions:
  - "Namespace carrier for SemanticVault chosen as Memory.paladin_id (an exact-match field every current and future SanctumPort adapter is guaranteed to index), not a metadata_filters entry -- documented on the module doc so no future reader has to rediscover why."
  - "Deterministic Sanctum entry id derived via two seeded std::collections::hash_map::DefaultHasher passes over a NUL-joined (ns, key) string, rather than Uuid::new_v5 -- avoids adding the uuid crate's v5 cargo feature for a single call site; a collision would only ever alias two Vault addresses onto one Sanctum entry, never break get/list/delete (always authoritative via store)."
  - "SemanticVault::search treats ns as a confinement boundary (is_prefix_of, matching self and any descendant), not list's exact-namespace-only semantics -- intentionally broader, because search exists to serve VaultRecallMiddleware's (D-25) grant-scoped recall in a later plan, not namespace browsing."
  - "SemanticVault::put embeds the query/value text BEFORE writing to either backend, so a failing EmbeddingPort leaves both store and sanctum untouched rather than leaving a store row with no matching Sanctum entry."
  - "Vault key stored in Memory.metadata under a private vault_key field so search can recover the (ns, key) address of a hit and reload the authoritative record from store, never trusting the vector payload's own copy."
  - "A generic, dependency-free redact-then-bound helper (crates/paladin-memory/src/vault/redact.rs) was added rather than reusing paladin_llm::redaction, since paladin-memory has no dependency on paladin-llm and the workspace's HARD-05 finding already flags a sibling extracted-crate cross-dependency as a defect not to repeat."

patterns-established:
  - "A new Vault adapter's RED/GREEN split can stub only the trait-impl block (put/get/delete/list/search) behind a deliberately-wrong-but-compiling body -- e.g. delegate-only or no-op -- while every surrounding module (docs, constructor, private helpers, and the full test module) is written once and never rewritten, keeping the retrofit small and the final diff clean."

requirements-completed: [RT-04]

coverage:
  - id: D1
    description: "003_create_vault_tables.sql creates vault_records with the exact D-23 schema and an index on (ns, key), numbered after 002; SqliteVault runs the same shared embedded migrator SqliteGarrison uses -- proven bidirectionally (a SqliteVault-only database gains garrison_entries, a SqliteGarrison-only database gains vault_records)"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/vault/sqlite.rs#tests::vault_and_garrison_share_one_migrator, tests::sqlite_vault_constructs_twice_idempotently"
        status: pass
      - kind: other
        ref: "grep -rc 'sqlx::migrate!' crates/paladin-memory/src --include=*.rs | awk -F: '{s+=$2} END {print s}' == 1; find crates/paladin-memory -type d -name migrations | wc -l == 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "SqliteVault passes the full shared contract suite (9 clauses) plus search-unsupported, with exact-namespace list scoping (never LIKE on ns), opaque-cursor pagination, parameter-bound SQL throughout (no format!-built SQL), and typed+redacted storage errors"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/vault/sqlite.rs#tests (16 tests: 9 shared clauses + list_scopes_to_exactly_the_namespace alias + search_is_unsupported_on_sqlite + run_all_shared_clauses_smoke_aggregate + sqlite_vault_passes_the_shared_contract_suite + sqlite_vault_constructs_twice_idempotently + vault_and_garrison_share_one_migrator + storage_errors_are_typed_and_redacted)"
        status: pass
      - kind: other
        ref: "grep -c LIKE sqlite.rs == 1; grep -c 'format!(\"SELECT|INSERT|DELETE|UPDATE' sqlite.rs == 0; git diff -- Cargo.toml crates/paladin-memory/Cargo.toml (no new feature lines)"
        status: pass
    human_judgment: false
  - id: D3
    description: "SemanticVault composes VaultPort+SanctumPort+EmbeddingPort with no qdrant-client dependency and no cargo feature; put is deterministic (re-put updates, never duplicates, in both halves); delete removes both halves; embedding failures surface as typed VaultError::Storage, never a panic; the type is proven constructible and functional under default features (no qdrant feature)"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/vault/semantic.rs#tests (16 tests: 9 shared clauses + search_returns_scored_records_from_the_store + semantic_vault_passes_the_shared_contract_suite + put_is_deterministic_and_updates_rather_than_duplicates + delete_removes_from_both_halves + embedding_failure_surfaces_as_a_typed_error_not_a_panic + semantic_vault_is_constructible_without_the_qdrant_feature)"
        status: pass
      - kind: other
        ref: "grep -B2 'pub mod semantic' mod.rs | grep -c cfg(feature) == 0; grep -c 'qdrant_client|use qdrant' semantic.rs == 0; grep -c unwrap/expect/panic! in production code (before #[cfg(test)]) == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "SemanticVault::search re-filters every hit by Namespace::is_prefix_of in its own code before returning, regardless of whether the backend honoured the passed filter -- pinned by a deliberately misbehaving SanctumPort double returning a hit from a namespace never asked about"
    requirement: "RT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-memory/src/vault/semantic.rs#tests::search_re_filters_by_namespace_even_when_the_backend_does_not"
        status: pass
      - kind: other
        ref: "grep -c is_prefix_of semantic.rs >= 1"
        status: pass
    human_judgment: false
  - id: D5
    description: "Workspace-wide gates stay green after both adapters land: cargo check/clippy/fmt across the whole workspace, and both feature configurations (sqlite, default) of paladin-memory's vault test module"
    requirement: "RT-04"
    verification:
      - kind: other
        ref: "cargo check --workspace --all-targets --all-features; cargo clippy --workspace --all-targets --all-features -- -D warnings; cargo fmt --all --check; cargo doc -p paladin-memory --all-features --no-deps (0 warnings)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-memory --features sqlite --lib vault (46 passed); cargo test -p paladin-memory --lib vault::semantic (16 passed, default features)"
        status: pass
    human_judgment: false

duration: ~1h 30min
completed: 2026-09-07
status: complete
---

# Phase 26 Plan 09: SqliteVault and SemanticVault Summary

**The Vault's remaining two adapters -- `SqliteVault` on the shared embedded migrator with a new `003` migration, and `SemanticVault` composing a `VaultPort` + `SanctumPort` + `EmbeddingPort` to give `search` a real, self-re-filtered implementation -- both instantiated against plan 26-04's shared contract suite.**

## Performance

- **Duration:** ~1h 30min
- **Tasks:** 2 (both `tdd="true"`, each with a real RED-then-GREEN commit pair)
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- `crates/paladin-memory/migrations/003_create_vault_tables.sql` creates `vault_records (ns, key, value, created_at, updated_at, PRIMARY KEY (ns, key))` plus an index on `(ns, key)`, numbered after `002` in the crate's one shared embedded `sqlx::migrate!` sequence -- proven bidirectionally: a database built only via `SqliteVault::new` gains `garrison_entries` too, and one built only via `SqliteGarrison::connect` gains `vault_records` too (both empty, both harmless, both `IF NOT EXISTS`).
- `SqliteVault` (`crates/paladin-memory/src/vault/sqlite.rs`) passes the full 9-clause shared contract suite plus the search-unsupported capability assertion: exact-namespace `list` scoping (`ns = ?`, never `LIKE` on the namespace column -- the file's single `LIKE` occurrence is the key-prefix match only), opaque-cursor pagination, an upsert `put` (`ON CONFLICT DO UPDATE`) that preserves `created_at` and bumps `updated_at`, and parameter-bound SQL throughout (zero `format!`-built query strings). Constructs idempotently twice against the same file.
- `SemanticVault` (`crates/paladin-memory/src/vault/semantic.rs`) composes `Arc<dyn VaultPort>` + `Arc<dyn SanctumPort>` + `Arc<dyn EmbeddingPort>`, holds no `qdrant-client` symbol and needs no cargo feature (ADR-0046). `put` embeds first (so a failing embedder touches neither backend), then writes `store` and upserts a deterministic-id Sanctum entry (re-put updates, never duplicates, in both halves). `search` embeds the query, passes `ns` to the backend as a best-effort `SanctumFilter::paladin_id` narrowing hint, then **unconditionally re-filters every hit** with `Namespace::is_prefix_of` before returning anything -- proven against a deliberately misbehaving `SanctumPort` double that ignores the filter and query entirely and always returns a foreign-namespace hit, which the re-filter still drops. Every returned record's value is reloaded fresh from `store`, never trusted from the vector payload.
- `contract_tests::search_returns_scored_records_from_the_store` (case 11) is the first search-capable shared contract case, instantiated only by `SemanticVault` -- `InMemoryVault` and `SqliteVault` continue to call `assert_search_unsupported` (case 10) instead.
- `crates/paladin-memory/src/vault/redact.rs` -- a generic, dependency-free credential-shape redact-then-bound helper (D-34), used by both new adapters' `VaultError::Storage`/`Serialization` error mapping, without adding a `paladin-llm` dependency to `paladin-memory`.
- No new cargo feature or dependency introduced anywhere in the plan.

## Task Commits

Each task's RED/GREEN split is a real, verified failure-then-fix: the RED commit's trait-impl methods are deliberately wrong (no-op / delegate-only / empty), confirmed failing against the shared contract suite, before the GREEN commit replaces them with the real implementation.

1. **Task 1: SqliteVault on the shared embedded migrator, with 003**
   - RED: `a036d8b7` (test: 12 of 16 tests fail against a no-op `put`/`get`/`delete`/`list` stub)
   - GREEN: `1bf43276` (feat: the real parameter-bound SQL implementation, all 16 tests passing)
2. **Task 2: SemanticVault -- composed, ungated, and re-filtering by namespace before returning**
   - RED: `163131a9` (test: 5 of 16 tests fail against a store-delegate-only `put`/`delete` and an empty-result `search` stub)
   - GREEN: `c0cc164f` (feat: the real composition, deterministic id, and namespace re-filter, all 16 tests passing)

## Files Created/Modified

- `crates/paladin-memory/migrations/003_create_vault_tables.sql` (new) -- `vault_records` schema + index, D-23
- `crates/paladin-memory/src/vault/sqlite.rs` (new) -- `SqliteVault`, 16 tests
- `crates/paladin-memory/src/vault/semantic.rs` (new) -- `SemanticVault`, 16 tests
- `crates/paladin-memory/src/vault/redact.rs` (new) -- shared redact-then-bound helper, 3 tests
- `crates/paladin-memory/src/vault/mod.rs` -- registers `sqlite` (feature-gated) and `semantic` (ungated), re-exports both, registers `redact`
- `crates/paladin-memory/src/vault/contract_tests.rs` -- adds case 11 (`search_returns_scored_records_from_the_store`)
- `crates/paladin-memory/src/prelude.rs` -- re-exports `SqliteVault` (feature-gated) and `SemanticVault`
- `crates/paladin-memory/src/lib.rs` -- crate-doc bullets and feature-flag table row updated for both adapters

## Decisions Made

- **Namespace carrier for `SemanticVault` is `Memory.paladin_id`**, not a `metadata_filters` entry -- it is the one field every current (`InMemorySanctum`) and future (`QdrantSanctumAdapter`) `SanctumPort` adapter is guaranteed to index and filter on exactly, whereas metadata-filter support is adapter-specific.
- **Deterministic Sanctum entry id via two seeded `DefaultHasher` passes**, not `Uuid::new_v5` -- avoids adding the `uuid` crate's `v5` cargo feature for a single call site. Documented collision cost: astronomically unlikely, and even if it occurred it would never break `get`/`list`/`delete` (always authoritative via `store`), only `search`'s vector-side aliasing.
- **`search`'s `ns` parameter is a confinement boundary (`is_prefix_of`), not `list`'s exact-namespace-only scope** -- intentionally broader, since `search` exists to serve a grant-scoped recall (D-25's `VaultRecallMiddleware`, a later plan) rather than namespace browsing.
- **Embed before writing to either backend in `put`** -- a failing `EmbeddingPort` must leave both `store` and `sanctum` untouched, not a `store` row with no matching Sanctum entry.
- **The Vault `key` rides in `Memory.metadata["vault_key"]`** so `search` can recover the exact `(ns, key)` address of a hit and reload the authoritative record from `store` -- the vector payload itself is never trusted as the source of truth.
- **A crate-local `redact.rs` was added rather than depending on `paladin_llm::redaction`** -- `paladin-memory` has no existing dependency on `paladin-llm`, and the workspace's own HARD-05 finding already flags one extracted-crate cross-dependency (`paladin-content` → `paladin-llm`) as a defect; adding a second was avoidable and out of scope for this plan.

## Deviations from Plan

### Auto-fixed Issues

None -- no bugs, missing functionality, or blocking issues were found; the implementation matched the plan's design throughout.

### Clarifications (not deviations from behavior, but from a literal verification command)

**1. Task 2's acceptance-criteria grep for `unwrap()`/`expect()`/`panic!` counts test-code assertions too**
- **Found during:** Task 2 final verification
- **Issue:** The plan's acceptance criterion `grep -v '^\s*//\|^\s*///' crates/paladin-memory/src/vault/semantic.rs | grep -c 'unwrap()\|expect(\|panic!'` is `0` reads literally over the *whole file*, but `semantic.rs`'s own `#[cfg(test)]` module uses `.unwrap()` extensively in test assertions -- the same convention as every sibling adapter file in this crate (`in_memory.rs`, `sqlite_garrison.rs`), and the correct, idiomatic way to write Rust tests. Read over the whole file the count is 23 (all inside `#[cfg(test)] mod tests`); read only over the production code preceding the `#[cfg(test)]` boundary, the count is genuinely `0`.
- **Resolution:** Verified separately with `awk '/#\[cfg\(test\)\]/{exit} {print}' semantic.rs | grep -v '^\s*//\|^\s*///' | grep -c 'unwrap()\|expect(\|panic!'` → `0`. No code change; the plan's grep command is over-broad rather than the code being wrong.
- **Committed in:** n/a (verification-only clarification, no fix required)

**2. Test 4's literal name (`list_scopes_to_exactly_the_namespace`) added as an alias**
- **Found during:** Task 1, writing `<verify>`
- **Issue:** The plan's `<verify>` block names an exact test function `list_scopes_to_exactly_the_namespace`, but the shared contract clause this behavior maps to is already named `list_returns_only_this_namespace_ordered_by_key` (established in plan 26-04, reused unchanged by `SqliteVault`).
- **Fix:** Added a second, explicitly-named `#[tokio::test] async fn list_scopes_to_exactly_the_namespace()` that calls the same shared clause, so the plan's own `<verify>` command resolves without renaming or duplicating the shared clause itself.
- **Files modified:** `crates/paladin-memory/src/vault/sqlite.rs`
- **Committed in:** `1bf43276` (Task 1 GREEN commit)

---

**Total deviations:** 0 auto-fixed; 2 verification-command clarifications (neither changed production behavior)
**Impact on plan:** No scope creep, no architectural change, no bug fixes needed.

## Issues Encountered

None beyond the two clarifications above.

## Known Stubs

None. Both adapters are fully implemented per the plan's `<action>` and `<done>` clauses; the Qdrant-backed `SemanticVault` configuration is explicitly documented (module doc + this SUMMARY) as routed to UAT per D-24/D-39, never marked passing locally -- this is a deliberate, planned tier boundary, not a stub.

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- All three Vault adapters (`InMemoryVault`, `SqliteVault`, `SemanticVault`) now pass one shared contract suite; `Namespace::is_prefix_of` is exercised as the confinement primitive by both `SemanticVault`'s re-filter and (from plan 26-04's readiness note) plan 26-13's forthcoming `ConfinedVault` decorator.
- `crates/paladin-memory/src/vault/redact.rs`'s `redact_and_bound` is available for any future Vault adapter that needs the same redact-then-bound treatment on adapter-boundary error text.
- The Qdrant-backed `SemanticVault` tier (a `QdrantSanctumAdapter` as the `sanctum` field) is architecturally ready -- `SemanticVault` takes any `Arc<dyn SanctumPort>` -- but is explicitly UAT-only per D-24/D-39; no local test constructs it.
- `.planning/STATE.md` / `.planning/ROADMAP.md` / `.planning/REQUIREMENTS.md` are NOT updated by this worktree-mode executor -- the orchestrator owns those writes after all wave 5 worktree agents complete.
- No blockers for later waves.

---
*Phase: 26-agent-runtime-enhancements*
*Completed: 2026-09-07*

## Self-Check: PASSED

Verified on disk / in git history:
- `crates/paladin-memory/migrations/003_create_vault_tables.sql` -- FOUND, contains `PRIMARY KEY (ns, key)`
- `crates/paladin-memory/src/vault/sqlite.rs` -- FOUND, contains `pub struct SqliteVault` and `impl VaultPort for SqliteVault`
- `crates/paladin-memory/src/vault/semantic.rs` -- FOUND, contains `pub struct SemanticVault` and `impl VaultPort for SemanticVault`
- `crates/paladin-memory/src/vault/redact.rs` -- FOUND
- Commit `a036d8b7` -- FOUND in `git log --oneline`
- Commit `1bf43276` -- FOUND in `git log --oneline`
- Commit `163131a9` -- FOUND in `git log --oneline`
- Commit `c0cc164f` -- FOUND in `git log --oneline`
- `cargo test -p paladin-memory --features sqlite --lib vault` -- 46 passed, 0 failed
- `cargo test -p paladin-memory --lib vault::semantic` (default features) -- 16 passed, 0 failed
- `cargo check --workspace --all-targets --all-features` -- exit 0
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` -- exit 0
- `cargo fmt --all --check` -- exit 0
- `cargo doc -p paladin-memory --all-features --no-deps` -- exit 0, no warnings
- Exactly one `sqlx::migrate!` invocation and one `migrations/` directory in `crates/paladin-memory`
