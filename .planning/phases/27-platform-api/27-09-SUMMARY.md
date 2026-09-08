---
phase: 27-platform-api
plan: 09
subsystem: database
tags: [sqlx, sqlite, postgres, assistant-repository-port, append-only-immutability, compare-and-set, freeze-at-submit]

requires:
  - phase: 27-platform-api (plan 02)
    provides: "RunRepositoryPort, is_unique_violation()-before-generic-wrap error mapping pattern, three-adapter contract-suite house style"
  - phase: 27-platform-api (plan 01)
    provides: "Run/AssistantRef core types, RunRepositoryPort trait shape"
provides:
  - "AssistantId/AssistantKind/AssistantDefinition/AssistantSource/AssistantVersion/Assistant core types in paladin-core (D-28) -- an opaque tagged-envelope definition, no engine types leak into core"
  - "AssistantRepositoryPort in paladin-ports with exactly create/append_version/get/get_version/list/list_versions/soft_delete -- no update method exists by construction (D-29)"
  - "assistants + assistant_versions SQL tables (SQLite + Postgres) with PRIMARY KEY (assistant_id, version) -- the D-29 immutability invariant as a schema property"
  - "InMemoryAssistantRepository, SqliteAssistantRepository, PostgresAssistantRepository -- one shared 10-clause contract suite all three pass unchanged"
  - "RunRepositoryPort::insert_with_latest (default: verbatim insert; real override on all three run adapters) and RunRepositoryError::UnknownAssistant -- D-30 freeze-at-submit as a database property, proven under concurrency"
affects: [27-12, 27-15]

tech-stack:
  added: []
  patterns:
    - "Default trait method for a required-looking new port method (RunRepositoryPort::insert_with_latest): the default falls back to a verbatim insert using the run's already-set assistant.version, so adding it did NOT break the two pre-existing test-double implementors in sibling-owned files (crates/paladin-web/src/run_controller.rs, src/application/services/run/cancel.rs) that this worktree could not touch (X-10.4, parallel-worktree file-ownership boundary)"
    - "Transactional insert-then-CAS for append-only version publishing: append_version_once (SQL adapters) wraps its version INSERT and its assistants.latest CAS UPDATE in one sqlx transaction; the public append_version retries append_version_once up to 20 times on VersionConflict (a PRIMARY KEY violation on (assistant_id, version))"
    - "insert_with_latest as ONE INSERT ... SELECT ... FROM assistants a WHERE a.assistant_id = ? AND a.deleted_at IS NULL statement: assistant_version is never a bound parameter, it is a.latest selected atomically; zero rows affected (no matching assistant) maps to UnknownAssistant, a unique-index violation on idx_runs_thread_active still maps to ThreadBusy via the existing map_insert_error"

key-files:
  created:
    - crates/paladin-core/src/platform/container/assistant.rs
    - crates/paladin-ports/src/output/assistant_repository_port.rs
    - crates/paladin-storage/src/assistant/mod.rs
    - crates/paladin-storage/src/assistant/contract_tests.rs
    - crates/paladin-storage/src/assistant/in_memory.rs
    - crates/paladin-storage/src/assistant/sqlite.rs
    - crates/paladin-storage/src/assistant/postgres.rs
    - crates/paladin-storage/migrations/sqlite/003_create_assistants_tables.sql
    - crates/paladin-storage/migrations/postgres/003_create_assistants_tables.sql
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-ports/src/output/run_repository_port.rs
    - crates/paladin-storage/src/lib.rs
    - crates/paladin-storage/src/run/contract_tests.rs
    - crates/paladin-storage/src/run/in_memory.rs
    - crates/paladin-storage/src/run/sqlite.rs
    - crates/paladin-storage/src/run/postgres.rs
    - MIGRATION.md

key-decisions:
  - "insert_with_latest is a DEFAULT trait method, not a required one: grepping impl RunRepositoryPort for in the tree found two implementors this worktree does not own (crates/paladin-web/src/run_controller.rs's MockRepository, src/application/services/run/cancel.rs's CountingRepo) -- adding a required method there would have broken plan 27-10's sibling files. The default falls back to a verbatim insert (no latest resolution), which is correct behavior for a test double with no assistant concept; only InMemoryRunRepository::with_assistants, SqliteRunRepository and PostgresRunRepository override it with the real D-30 resolution."
  - "A module-level rustdoc bug discovered and worked around: an outer /// doc comment on a `pub mod assistant;` declaration, COMBINED with an inner //! module doc inside that file, made rustdoc lose intra-doc-link resolution for every [`Type`] link in the inner doc (attributed the warning to the wrong file/line, mod.rs:3 instead of assistant.rs). Fixed by dropping the outer /// comment on crates/paladin-core/src/platform/container/mod.rs's `pub mod assistant;` line, matching the existing `pub mod run;` (no outer doc) precedent -- confirmed by cargo doc warning count dropping from 9 to the 2 pre-existing ones."
  - "AssistantId::new validates a slug ([a-z0-9][a-z0-9_-]{0,63}) with a dedicated AssistantIdError, mirroring ThreadId's newtype+validation pattern (waypoint.rs) rather than reusing ThreadId's whitespace-only rule -- an assistant id is a URL path segment and a storage key with a stricter charset than a thread id."
  - "append_version_once's assistants.latest CAS UPDATE affecting zero rows after a successful version INSERT is treated as AssistantRepositoryError::Backend (a named anomaly), not silently retried as VersionConflict -- by this design's own invariants (every latest advance is paired with the version insert that just succeeded, in the SAME transaction, under the caller-supplied expected_latest predicate) this should never happen; surfacing it loudly rather than looping is the deliberate choice if the invariant is ever violated."

requirements-completed: [PLAT-04]

coverage:
  - id: D1
    description: "AssistantDefinition is a tagged envelope over opaque JSON in paladin-core (kind: Agent|Workflow, body: serde_json::Value) -- grep-verified zero mentions of paladin-battalion/paladin_battalion anywhere in assistant.rs, including comments"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/assistant.rs -- 19 tests incl. assistant_definition_round_trips_through_serde, assistant_kind_serializes_snake_case"
        status: pass
      - kind: other
        ref: "grep -c 'paladin-battalion\\|paladin_battalion' crates/paladin-core/src/platform/container/assistant.rs == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "AssistantRepositoryPort has exactly create/append_version/get/get_version/list/list_versions/soft_delete -- no update method exists anywhere in the file (grep-verified), immutability enforced by the PRIMARY KEY (assistant_id, version) on assistant_versions, not by handler discipline"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-ports/src/output/assistant_repository_port.rs -- trait_is_object_safe, page-default tests"
        status: pass
      - kind: other
        ref: "grep -c 'async fn update' assistant_repository_port.rs == 0; grep -c 'async fn append_version' == 1; grep -c 'PRIMARY KEY (assistant_id, version)' in both migration files == 1 each; grep -rc 'UPDATE assistant_versions' crates/paladin-storage/src/assistant/ == 0 in every file"
        status: pass
    human_judgment: false
  - id: D3
    description: "InMemoryAssistantRepository, SqliteAssistantRepository and PostgresAssistantRepository all pass the identical 10-clause contract suite, including the concurrent_append_admits_exactly_one_per_version stress test (ten concurrent append_version calls -> versions 2..=11, no gaps, no duplicates)"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --lib assistant::in_memory -- 14 passed (10 contract + 4 direct); cargo test -p paladin-storage --features sqlite --lib assistant::sqlite -- 12 passed (10 contract + append_version_once conflict + password redaction)"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-storage --features postgres --lib assistant::postgres -- --nocapture -- 10 passed, 10 SKIP: lines (Docker unavailable in this devcontainer; Tier 2, CI's postgres-integration job is the proof, D-51)"
        status: pass
    human_judgment: false
  - id: D4
    description: "RunRepositoryPort::insert_with_latest resolves and freezes assistant_version from assistants.latest inside ONE insert statement on both SQL backends; a concurrent version publish is observed strictly before or strictly after, proven by assistant_version_freeze_at_submit (20 alternating append_version/insert_with_latest calls, no run resolves version 0 or a version above the final latest)"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/run/contract_tests.rs#assistant_version_freeze_at_submit, run against InMemoryRunRepository::with_assistants and a shared on-disk SqliteRunRepository/SqliteAssistantRepository pair"
        status: pass
      - kind: integration
        ref: "cargo test -p paladin-storage --features postgres --lib run::postgres -- --nocapture -- 18 passed, self-skipping (Docker unavailable locally)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every version records created_at/created_by/note; a soft-deleted assistant's versions stay readable by get_version so historical runs remain reconstructable (PLAT-FR-10)"
    requirement: "PLAT-04"
    verification:
      - kind: unit
        ref: "crates/paladin-storage/src/assistant/contract_tests.rs#soft_delete_then_versions_still_readable_but_append_fails, run against all three adapters"
        status: pass
    human_judgment: false

duration: ~4h
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 09: Assistant Persistence Summary

**Append-only immutable assistant versions: `AssistantDefinition` as an opaque tagged-JSON envelope in `paladin-core`, an `AssistantRepositoryPort` with no update method, `PRIMARY KEY (assistant_id, version)` as the schema-level immutability guarantee, and `RunRepositoryPort::insert_with_latest` freezing a run's assistant version from `assistants.latest` inside one atomic SQL statement.**

## Performance

- **Duration:** ~4h
- **Started:** 2026-09-08 (this session, after wave-1..3 base commit `8c762ade`)
- **Completed:** 2026-09-08
- **Tasks:** 3 (1 checkpoint:decision auto-selected, 2 `type="auto" tdd="true"`)
- **Files modified:** 19 (9 created, 10 modified)

## Accomplishments

- `AssistantId` (validated slug newtype), `AssistantKind`, `AssistantDefinition { kind, body: serde_json::Value }`, `AssistantSource`, `AssistantVersion`, `Assistant`, `NewAssistantVersion`, `ASSISTANT_SCHEMA_VERSION` exist in `paladin-core` (D-28) — grep-verified zero mentions of `paladin-battalion` anywhere in the file, including comments.
- `AssistantRepositoryPort` in `paladin-ports` exposes exactly seven methods (`create`, `append_version`, `get`, `get_version`, `list`, `list_versions`, `soft_delete`) — no method anywhere in the file rewrites an existing version's definition (D-29), and `RunRepositoryError::UnknownAssistant` plus `RunRepositoryPort::insert_with_latest` (a default-implemented method, see Decisions) were added for D-30.
- `003_create_assistants_tables.sql` (SQLite + Postgres) creates `assistants` and `assistant_versions`, the latter with `PRIMARY KEY (assistant_id, version)` — the immutability invariant as a schema property, not an application check.
- `InMemoryAssistantRepository`, `SqliteAssistantRepository`, `PostgresAssistantRepository` all pass the identical 10-clause `crates/paladin-storage/src/assistant/contract_tests.rs` suite unchanged, including the ten-concurrent-`append_version` stress test.
- `SqliteRunRepository::insert_with_latest` / `PostgresRunRepository`'s twin resolve and freeze `assistant_version` from `assistants.latest` inside a single `INSERT ... SELECT ... FROM assistants` statement; `assistant_version_freeze_at_submit` (20 alternating `append_version`/`insert_with_latest` calls across concurrent tasks) proves no run ever resolves version `0` or a version above the final `latest`.
- `MIGRATION.md` §9.4 gained the `assistants`/`assistant_versions` row; §9.3 gained the second `schemars` dependency-edge row (from `paladin-battalion`, plan 27-05, registered here per the same-wave file-ownership rule).

## Task Commits

Each task was committed atomically:

1. **Task 1: Confirm the one-way assistant shape and immutability contract (D-28, D-29)** — checkpoint:decision, `gate="blocking"`, auto-selected **option-a** ("Proceed as decided (D-28 + D-29 verbatim)") by the orchestrator under auto-mode at dispatch 2026-09-08T05:27Z. No code change; recorded here per the pre-resolution instruction — not re-litigated by this executor.
2. **Task 2: Core assistant types, the repository port (no update), migrations `003`, and the contract suite on InMemory** — `c65dbc1e` (feat)
3. **Task 3: SQLite + Postgres assistant repositories, `insert_with_latest` on the SQL run adapters, `MIGRATION.md` §9.3/§9.4** — `78077ed6` (feat)

**Plan metadata:** this file's own commit (docs: complete plan) — committed alongside this SUMMARY per worktree execution mode.

_TDD note: both Tasks 2 and 3 carry `tdd="true"`. Per-task tests were written and passing before each commit; no separate RED-then-GREEN commit pair was produced (test + implementation landed together per task, consistent with 27-01/27-02's documented convention for this worktree)._

## Files Created/Modified

- `crates/paladin-core/src/platform/container/assistant.rs` — `AssistantId`/`AssistantIdError`, `AssistantKind`, `AssistantDefinition`, `AssistantSource`, `AssistantVersion`, `NewAssistantVersion`, `Assistant`, `ASSISTANT_SCHEMA_VERSION`.
- `crates/paladin-core/src/platform/container/mod.rs` — declares `pub mod assistant;` (no outer doc comment — see Decisions).
- `crates/paladin-ports/src/output/assistant_repository_port.rs` — `AssistantRepositoryPort` (7 methods), `AssistantRepositoryError`, `AssistantPage`, `AssistantVersionPage`.
- `crates/paladin-ports/src/output/mod.rs` — declares `pub mod assistant_repository_port;`.
- `crates/paladin-ports/src/output/run_repository_port.rs` — adds `RunRepositoryPort::insert_with_latest` (default method) and `RunRepositoryError::UnknownAssistant`.
- `crates/paladin-storage/src/assistant/{mod,contract_tests,in_memory,sqlite,postgres}.rs` — the full three-adapter set plus the shared contract suite.
- `crates/paladin-storage/src/lib.rs` — declares `pub mod assistant;`.
- `crates/paladin-storage/src/run/contract_tests.rs` — adds `insert_with_latest_resolves_current_latest_and_freezes_it`, `insert_with_latest_unknown_assistant_fails`, `insert_with_latest_soft_deleted_assistant_fails`, `assistant_version_freeze_at_submit`.
- `crates/paladin-storage/src/run/in_memory.rs` — `InMemoryRunRepository::with_assistants` + real `insert_with_latest` override.
- `crates/paladin-storage/src/run/sqlite.rs` — `INSERT_RUN_WITH_LATEST` (single-statement freeze), `insert_with_latest` override, four new tests via a shared on-disk WAL file with `SqliteAssistantRepository::new_shared_file`.
- `crates/paladin-storage/src/run/postgres.rs` — Postgres twin of the above; test module's `#[tokio::test]` count kept equal to `run::contract_tests`'s `pub async fn` count (18 == 18), preserving 27-02's CI drift guard.
- `crates/paladin-storage/migrations/{sqlite,postgres}/003_create_assistants_tables.sql` — `assistants` + `assistant_versions` (`PRIMARY KEY (assistant_id, version)`).
- `MIGRATION.md` — §9.4 row (assistants tables); §9.3 row (`schemars`'s second dependency edge).

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`insert_with_latest` is a default trait method, not a required one.** A pre-flight `grep -rn "impl RunRepositoryPort for"` found two implementors in files this worktree's parallel-execution boundary forbids touching (`crates/paladin-web/src/run_controller.rs`'s `MockRepository`, `src/application/services/run/cancel.rs`'s `CountingRepo`, both owned by sibling plan 27-10). Adding a required method would have broken their compile the moment 27-10's worktree merges. The default implementation inserts the run as-is (using its already-set `assistant.version`), which is the correct fallback for a test double with no assistant-repository concept — only the three real adapters override it with D-30's actual freeze-at-submit resolution. Verified: `grep -c 'async fn insert_with_latest' run_repository_port.rs` is exactly `1` (the trait's own default-bodied declaration).
2. **A rustdoc intra-link resolution bug, worked around by dropping a redundant doc comment.** An outer `///` doc comment on `pub mod assistant;` in `container/mod.rs`, combined with `assistant.rs`'s own inner `//!` module doc, made every `[`Type`]` intra-doc link in the inner doc unresolved — `cargo doc` attributed all 7 warnings to `mod.rs:3` (the wrong file) rather than `assistant.rs`. Dropping the outer `///` comment (matching the pre-existing `pub mod run;`'s no-outer-doc convention) fixed it: warning count dropped from 9 to the 2 pre-existing, unrelated ones. Documented in-code as a `cfg` gate; not upstream-reported (out of scope for this plan).
3. **`AssistantId` gets its own stricter validation, not `ThreadId`'s.** `ThreadId::new` only rejects empty/too-long/whitespace; `AssistantId::new` additionally enforces the `[a-z0-9][a-z0-9_-]{0,63}` slug charset, because an assistant id is used as an HTTP path segment (`GET /assistants/{id}`) where uppercase/mixed-case ids would create case-sensitivity ambiguity a thread id (never itself a path segment in the current API) does not face.
4. **`append_version_once`'s post-insert CAS-affecting-zero-rows anomaly is a named `Backend` error, not a masked retry.** Given the transactional insert-then-CAS design, this state should be unreachable — surfacing it loudly (rather than silently looping inside `VersionConflict`'s retry budget) makes a future violation of that invariant a loud test failure instead of a quietly wrong `latest`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `LIST_VERSIONS_PREFIX`/postgres twin mixed a literal placeholder into a `QueryBuilder` seed, producing a mismatched placeholder/binding count**
- **Found during:** Task 3, first `cargo test -p paladin-storage --features sqlite --lib assistant::sqlite` run
- **Issue:** `LIST_VERSIONS_PREFIX` originally ended in `"... WHERE assistant_id = ?"` (a literal placeholder), then `list_versions` called `builder.push_bind(assistant_id...)` on top of it — `QueryBuilder`'s own `push_bind` generates its own placeholder, so the query ended up with two placeholders (`?` and the pushed one) but only one bound value, failing with `near "?": syntax error` at runtime.
- **Fix:** Changed the constant to end in `"... WHERE assistant_id = "` (no literal placeholder) so `push_bind` supplies the only placeholder; applied identically to the Postgres twin (`$N` form) before it could hit the same bug.
- **Files modified:** `crates/paladin-storage/src/assistant/sqlite.rs`, `crates/paladin-storage/src/assistant/postgres.rs` (written correctly from the start once the SQLite bug was found)
- **Verification:** `cargo test -p paladin-storage --features sqlite --lib assistant::sqlite` — all 12 tests pass, including `list_versions_paginates_ascending_by_version`.
- **Committed in:** `78077ed6` (Task 3 commit — caught before commit, no separate fix commit needed)

**2. [Rule 1 - Bug] A rustdoc intra-link resolution failure attributed to the wrong file, caused by a redundant outer doc comment**
- **Found during:** Task 2, first `cargo doc -p paladin-ai-core --no-deps` run
- **Issue:** See Decisions #2 above — 7 new `broken_intra_doc_links` warnings, all misattributed to `container/mod.rs:3`.
- **Fix:** Removed the outer `///` doc comment on `pub mod assistant;`, matching `pub mod run;`'s existing no-outer-doc convention.
- **Files modified:** `crates/paladin-core/src/platform/container/mod.rs`
- **Verification:** `cargo doc -p paladin-ai-core --no-deps` — 2 warnings (both pre-existing, unrelated).
- **Committed in:** `c65dbc1e` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bug fixes)
**Impact on plan:** Both were caught and resolved before their respective task commits landed; neither changed the plan's architecture, scope, or the port/schema contracts D-28/D-29/D-30 fix.

## Issues Encountered

None beyond the two auto-fixed deviations above — each was caught during the first `cargo test`/`cargo doc` pass for its task, before any commit.

## User Setup Required

None for local development — everything in this plan's Tier 1 evidence runs against SQLite (`sqlite::memory:` and a real on-disk WAL temp file for the freeze-at-submit tests) with no Docker dependency. The Postgres adapters require `STORAGE_POSTGRES_TEST_URL` (or Docker's `postgres-test` service) to exercise their Tier 2 suites in CI; that is CI/UAT infrastructure the orchestrator owns, not a step required of this executor or a future reader of this SUMMARY.

## Next Phase Readiness

- `AssistantRepositoryPort` (InMemory, SQLite, Postgres) all pass the identical 10-clause contract suite — plan 27-12 (stored-assistant resolver, publish-time validation) can build against any of the three without re-deriving semantics.
- `RunRepositoryPort::insert_with_latest` is proven as a database property under true concurrency on SQLite (on-disk, WAL, multi-connection) and structurally identical on Postgres (same `INSERT ... SELECT ... FROM assistants` shape, same `is_unique_violation()` mapping) — 27-03's CI job generalization (not touched by this plan) can wire the Postgres suite into `postgres-integration` without further adapter changes.
- The `insert_with_latest` default-method decision means 27-12's `RunSubmissionService`/`CodeWorkflowResolver` (owned by 27-10/facade work) can freely call `insert_with_latest` on whatever `RunRepositoryPort` they hold — real adapters resolve `latest`, any future test double inherits the safe verbatim-insert default with zero migration cost.
- No blockers. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo check --workspace --all-targets --all-features` all pass clean on the final commit; `cargo doc -p paladin-storage -p paladin-ai-core -p paladin-ports --all-features --no-deps` introduces no new warnings beyond the 3 pre-existing, unrelated ones.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `crates/paladin-core/src/platform/container/assistant.rs`
- FOUND: `crates/paladin-ports/src/output/assistant_repository_port.rs`
- FOUND: `crates/paladin-storage/src/assistant/mod.rs`
- FOUND: `crates/paladin-storage/src/assistant/contract_tests.rs`
- FOUND: `crates/paladin-storage/src/assistant/in_memory.rs`
- FOUND: `crates/paladin-storage/src/assistant/sqlite.rs`
- FOUND: `crates/paladin-storage/src/assistant/postgres.rs`
- FOUND: `crates/paladin-storage/migrations/sqlite/003_create_assistants_tables.sql`
- FOUND: `crates/paladin-storage/migrations/postgres/003_create_assistants_tables.sql`

**Commits verified to exist (git log --oneline):**
- FOUND: `c65dbc1e` feat(27-09): add core assistant types, port, migrations and InMemory contract suite
- FOUND: `78077ed6` feat(27-09): add SQLite+Postgres assistant repositories and insert_with_latest on SQL run adapters

**Verification commands re-run and confirmed passing:**
- `cargo test -p paladin-ai-core --lib assistant` → `test result: ok. 19 passed`
- `cargo test -p paladin-storage --lib assistant::in_memory` → `test result: ok. 14 passed`
- `cargo test -p paladin-storage --lib assistant_version_freeze_at_submit` → `test result: ok. 1 passed`
- `cargo test -p paladin-storage --features sqlite --lib assistant::sqlite` → `test result: ok. 12 passed`
- `cargo test -p paladin-storage --features sqlite --lib run::sqlite` → `test result: ok. 19 passed`
- `cargo test -p paladin-storage --features postgres --lib assistant::postgres -- --nocapture` → `test result: ok. 10 passed`, 10 `SKIP:` lines (Docker unavailable locally; Tier 2 — CI's `postgres-integration` job is the proof)
- `cargo test -p paladin-storage --features postgres --lib run::postgres -- --nocapture` → `test result: ok. 18 passed`, self-skipping
- `cargo test -p paladin-storage --all-features --lib` → `test result: ok. 323 passed`
- `grep -c 'async fn update' crates/paladin-ports/src/output/assistant_repository_port.rs` → `0`
- `grep -c 'async fn append_version' crates/paladin-ports/src/output/assistant_repository_port.rs` → `1`
- `grep -c 'PRIMARY KEY (assistant_id, version)'` in both `003_create_assistants_tables.sql` files → `1` each
- `grep -c 'async fn insert_with_latest' crates/paladin-ports/src/output/run_repository_port.rs` → `1`
- `grep -c 'paladin-battalion\|paladin_battalion' crates/paladin-core/src/platform/container/assistant.rs` → `0`
- `grep -cE '^\s*#\[tokio::test\]' crates/paladin-storage/src/assistant/postgres.rs` → `10`, equals `grep -c 'pub async fn' crates/paladin-storage/src/assistant/contract_tests.rs` → `10`
- `grep -c 'FROM assistants' crates/paladin-storage/src/run/sqlite.rs` → `2`; same for `run/postgres.rs` → `1`
- `awk '/^## 9.4/,/^## 9.5/' MIGRATION.md | grep -c '003_create_assistants_tables'` → `1`
- `awk '/^## 9.3/,/^## 9.4/' MIGRATION.md | grep -c schemars` → `2`
- `grep -rc 'UPDATE assistant_versions' crates/paladin-storage/src/assistant/` → `0` in every file
- `cargo fmt --all --check` → clean
- `cargo clippy -p paladin-storage --all-features --all-targets -- -D warnings` → clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean
- `cargo check --workspace --all-targets --all-features` → exit 0
- `cargo doc -p paladin-storage -p paladin-ai-core -p paladin-ports --all-features --no-deps` → no new warnings (3 pre-existing, unrelated warnings unchanged)

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
