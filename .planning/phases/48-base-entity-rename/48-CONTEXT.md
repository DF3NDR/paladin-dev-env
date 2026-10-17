# Phase 48: Base Entity Rename - Context

**Gathered:** 2026-10-13 (`/gsd-discuss-phase 48`, owner answers) — supersedes the 2026-10-10 /
2026-10-12 survey draft, whose ground truth is kept below unchanged
**Status:** Ready for planning — **the nine plans written on 2026-10-13 before this discussion are
superseded** (D-15): three owner answers overturn their premises (D-05 keep-all, D-10 citadel
unification, D-07 graph pins). Run `/gsd-plan-phase 48` again from this file.
**Mode:** owner-requested phase; decisions below are the owner's (2026-10-13) except where marked
*Claude's discretion*

<domain>
## Phase Boundary

One rename, its fallout, and the cleanup it exposes:

1. **The rename (ENTITY-01, ENTITY-02, ENTITY-03).** The base-layer wrapper
   `paladin_core::base::entity::node::Node<T>` becomes `paladin_core::base::entity::Entity<T>`;
   its payload field `node` becomes `data`. Every alias (`Paladin`, `Council`, `Grove`, `User`,
   `Log`) and every wrapper struct (`ContentItem`, `PromptItem` — their `node` field becomes
   `entity`) follows. The version-history family `NodeVersion` / `NodeVersionMetadata` /
   `NodeVersionRepository` / `NodeVersionService` / `InMemoryNodeVersionRepository`
   (`base/service/node_version_service.rs`) becomes `EntityVersion*`. The old name and module
   are removed outright (clean break, D-02); persisted JSON keeps loading through serde
   read-aliases (D-03, D-08, D-09) that stay for the whole 0.x line (D-11).

2. **The base-layer disposition (ENTITY-04).** `Field`, `Collection`, `Element`,
   `FieldVersionService` and `CollectionVersionService` have zero consumers outside `base/`.
   **All five are kept, with a written reason** (D-05): they are the scaffolding of the unfinished
   content / general-data-orchestration system, and a follow-up review decides their fate. The
   `version: bool` flag on the wrapper gets a rustdoc contract (D-04). The citadel module's
   placeholder `PaladinData` / `PaladinStatus` and its duplicate `Paladin` alias are unified with
   the real ones in `paladin.rs` (D-10).

3. **The vocabulary rule (ENTITY-05).** "Node" is reserved for the graph sense. ADR-0060 says so;
   CLAUDE.md / `.github/copilot-instructions.md`'s naming table and the mdBook architecture
   pages agree; `MIGRATION.md` §9.2 and the semver allowlist carry the breaks; a repository-scan
   guard pins both the absence of the old entity paths and the graph vocabulary's definition
   sites (D-07).

**Not in this phase:** any change to the graph vocabulary (`NodeId`, `NodeSpec`, `StateNode`,
`NodeContext`, `NodeCache*`, `NodeInterceptor`, `NodeExecutionRecord`, `NodeOutcomeKind`,
`NodeProgressKind`, `TraceEvent::Node*`, `docs/schemas/wargraph-doc.schema.json`, the
`node_cache` config section, Redis node-cache keys); the legacy `battalion::NodeError`
removal (already done — Phase 44, LEGACY-01, sealed 2026-10-11, pinned by
`tests/legacy_removal_guard.rs`); removing any of the five retained base items (that needs the
review in the todo named under D-05 first); migrating the owned downstream system (it adopts
the milestone at once per the clean-break policy); designing a general revisioning system for
the platform (see Deferred); renaming `platform/container/` or splitting it; a separate PR for
this phase (D-06).

</domain>

<ground_truth>
## What the survey found (tree at `61de2de`, 2026-10-10)

### The base layer today

`crates/paladin-core/src/base/` is laid out as originally designed — `entity/` (node,
collection, field, message), `component/` (action, element, event), `service/` (five
services) — but most of it is dormant:

| Base item | Consumers outside `base/` |
|---|---|
| `Node<T>` (`base/entity/node.rs`, 188 lines) | heavy — see below |
| `Message`, `Action`, `Event`, `MessageService`, `AnalysisService` | 7–21 files each |
| `Field`, `Collection`, `Element` | **0** |
| `FieldVersionService`, `CollectionVersioningService` | **0** |
| `NodeVersionService` / `NodeVersionRepository` | only `src/core/platform/manager/content_service.rs` (root crate) and `tests/functional/content_{lifecycle,fetching_pipeline}_test.rs` |

`Node<T>` carries `uuid`, `created`, `modified`, `name: Option<String>`, the payload field
`node: T`, and `version: bool`. The flag only records whether versioning is "enabled"; nothing
connects a Paladin, Council, Grove, User or Log to a revision store. The only honoured path is
`ContentItem` → `NodeVersionService`. When Phase 27 needed versioned Assistants it built
`AssistantVersion { version: u32, … }` independently.

`platform/manager/` inside `paladin-core` still holds only `user_service.rs`.

### What is built on `Node<T>`

Only 8 of the 70 files in `crates/paladin-core/src/platform/container/` use it:

- `paladin.rs:229` — `pub type Paladin = Node<PaladinData>;`
- `citadel.rs:48` — a **duplicate** `pub type Paladin = Node<PaladinData>;` (remove)
- `battalion/council.rs:268` — `Council`; `battalion/grove.rs:291` — `Grove`
- `user.rs:165` — `User`; `log.rs:397` — `Log`
- `content.rs:12` and `prompt.rs:11` — `ContentItem` / `PromptItem` hold a `node: Node<…>` field

Everything newer stands alone with its own typed id and `schema_version`: `Run`, `Assistant`,
`Waypoint`, `VaultRecord`, `Battlefield`, `TraceEvent`, the treasury ledger, `CachedDelta`.
`platform/container` has become a general folder of domain types, not a layer extending base.

### Where the graph sense came from, and how big it is

The graph vocabulary arrived with the v0.10.0 superstep engine (`WarGraph`, Phase 22,
2026-09-02), grew in Phase 25 ("Node-Level Fault Tolerance") and again in Phase 43 (Redis node
cache). The older `Campaign` already used petgraph's `NodeIndex`. Git history is squashed, so
these dates come from `.planning/ROADMAP.md`.

| Graph sense (workspace `.rs` at `25dc267`, excluding `base/entity/node.rs`) | Occurrences | Files |
|---|---|---|
| `NodeId` | 1,910 | 91 |
| `node_id` | 1,151 | 85 |
| `NodeSpec` | 1,034 | 64 |
| `StateNode` | 410 | 42 |
| `NodeError` (one type since Phase 44) | 396 | 56 |
| `NodeCache` / `NodeCacheKey` / `NodeCachePort` | 378 / 96 / 65 | 20 / 8 / 12 |
| `NodeOutcomeKind` | 208 | 27 |
| `NodeContext` | 192 | 47 |
| `NodeExecutionRecord` | 72 | 15 |
| `NodeInterceptor` | 47 | 8 |
| `NodeDoc` (graph export) | 44 | 3 |
| `NodeProgressKind` | 22 | 6 |

| Entity sense | Occurrences | Files |
|---|---|---|
| `use …base::entity::node` | 148 | 117 |
| `Node::new` | 334 | 117 |
| `.node` payload access (a handful are `graph.node(..)`) | 739 | 107 |
| `Node<` in signatures | 25 | 17 |
| mdBook / README mentions of `Node<T>` | — | 9 (`docs/src/architecture/{domain-model,design-patterns,overview,crate-map}.md`, `docs/src/api-reference/{crate-map,stable-api}.md`, `docs/src/assets/LayerArchitecture.svg`, `README.md`, `crates/paladin-core/README.md`) |
| `examples/`, `doc-examples`, `benches/` | — | 13 |
| `api_surface_current.txt` lines naming `node_version_service` | 9 | 1 |

Public wire formats that carry the graph sense and must not move: `wargraph-doc.schema.json`
(`nodes`, `max_node_visits`), `TraceEvent::NodeStarted/NodeFinished/NodeProgress`, persisted
`Waypoint` / `NodeExecutionRecord`, `CachedDelta` + Redis keys, `config.example.yml`'s
`node_cache` section and `src/config/node_cache.rs`. Migrations mention it only in a comment
(`crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql:17`).

### The collision is already in the code

- `NodeSpec::Paladin { paladin: Box<Paladin> }` (`crates/paladin-battalion/src/engine/graph.rs:42`)
  — a graph node wrapping an entity Node; its rustdoc has to say "a `Node<PaladinData>`".
- In `engine/superstep.rs`, `paladin.node` (entity payload), `graph.node(node_id)` (graph
  lookup) and `progress.node == NodeId::new("planner")` (graph id) appear in the same file.
- At survey time two unrelated types were both called `NodeError`: the legacy
  `battalion::NodeError { node_name, error }` (`battalion/mod.rs:520`) and the structured
  `platform::container::node_error::NodeError` (`node_error.rs:152`), with a warning comment and
  two pinning tests telling them apart. **Phase 44 (LEGACY-01, sealed 2026-10-11) removed the
  legacy one** — re-verified on `846a5cf`: `node_error.rs:152` is now the only `pub struct
  NodeError` in the workspace, and `BattalionResult.node_errors` was retyped to it (44-06).

### Why rename the entity and not the graph

1. The graph sense is the one every reader and every dependency (petgraph, LangGraph-style
   docs) expects; renaming it would work against both.
2. The graph sense is ~5× larger and is in public wire formats; renaming it breaks JSON
   schemas, trace streams, cache keys and config, not just Rust.
3. The entity rename is mechanical, the module is already called `base::entity`, and the owner
   described nodes as "a type of entity".
4. The one argument the other way — the owned downstream system uses "node" in the entity sense
   — is answered by the deprecated alias (ENTITY-02): it keeps compiling and migrates with the
   same find-and-replace later.

</ground_truth>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases — not re-asked)

- **D-00a:** Shipped tree outranks any document. 82 % workspace line-coverage floor (ADR-0006).
  `make clean-code`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an
  intentional public-surface change; the gated baseline is `.project/current-exports.txt`, not
  the stale `api_surface_current.txt` ENTITY-05 names), `make security`, `make check-api-examples`
  and the manual credential-handling review gate every commit.
- **D-00b:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when
  marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one published crate name
  (`paladin-ai-core` for `crates/paladin-core`) and one lint, kept set-equal by
  `make check-migration-allowlist`. Lints are measured, not guessed (the Phase 44 D-27
  diagnostic: temporarily drop the crate-wide `allow` lines, run `cargo semver-checks
  check-release --package <crate> --baseline-version 0.10.1 --release-type minor`, restore).
  X-03 as written ("deprecations allowed, removals are not, before v0.11.0") has been superseded
  per phase by ADR-0051 (Phases 31-33) and ADR-0059 (Phase 44); each supersession is its own
  record, never inherited. `MIGRATION.md` §9.7 Deprecations is empty and stays empty here.
- **D-00c:** The two-officer model and the Treasurer vocabulary (ADR-0049/0050, Phase 30 VOCAB)
  are untouched; this phase adds a vocabulary rule, it does not revisit one.
- **D-00d:** `paladin-core` has zero dependencies (ADR-0015); nothing in this phase may add one.
- **D-00e:** Phase 44 sealed 2026-10-11 (ADR-0059; `tests/legacy_removal_guard.rs`). It touched
  none of this phase's files. Its guard bans a *declaration or alias* of `NodeError` in the core
  battalion module and the `battalion::NodeError` path anywhere; every push must keep
  `cargo test --test legacy_removal_guard` green.

### The rename and the old name's fate (ENTITY-01, ENTITY-02)

- **D-01:** The wrapper is `Entity<T>`, public path `paladin_core::base::entity::Entity`, payload
  field `data`. Generic on purpose: the base layer predates the Medieval Military theme and is
  meant to sit under any system; the theme stays on domain concepts. Rejected: `Record<T>`
  (collides with `VaultRecord` in prose), `Versioned<T>` (over-promises, see D-04),
  `Aggregate<T>` (`Paladin` is the aggregate root, the wrapper is not), a themed name.
  — **Reversibility:** costly — 117 import sites and ~650 payload accesses change; the name is in
  the published `paladin-ai-core` surface once Phase 47 tags v0.11.0.
- **D-02:** Clean break, no alias. `Node<T>` and the `base::entity::node` module are removed
  outright; ADR-0060 records the Phase-48-only X-03 supersession in the ADR-0059 shape;
  `MIGRATION.md` §9.7 stays empty. The `#[deprecated]` alias alternative was rejected on the
  measured ground (48-RESEARCH.md Pitfall 9) that a type alias cannot alias the renamed *field*,
  so every downstream payload access and struct literal breaks anyway while each surviving use
  fails the downstream's own `-D warnings` build. — **Reversibility:** one-way once v0.11.0 is
  published (Phase 47) — `paladin-ai-core` 0.10.1 on crates.io exports the old name and the
  downstream consumer migrates against the published surface.
- **D-03:** Serde compatibility by read-alias, not by version bump. The payload field carries
  `#[serde(alias = "node")]`: a document with the old key or the new key loads, one with both is
  rejected (`duplicate field data`), one with neither is rejected (`missing field data`); new
  writes emit only `data`. No `schema_version` is added to `Entity<T>`. A fixture test loads
  captured pre-rename `Paladin`, `PaladinState` (Citadel snapshot), `ContentItem` and
  `PromptItem` JSON from the unmodified tree. The "key stays `node` on the wire" option
  (`#[serde(rename = "node")]`) was rejected: the owner wants persisted documents to stop saying
  "node" in the entity sense too. — **Reversibility:** one-way for downgrade — a v0.10 binary
  cannot read JSON written by v0.11; `MIGRATION.md` §9.4 says so.
- **D-08:** `ContentItem.node` and `PromptItem.node` become `entity`, each with
  `#[serde(alias = "node")]`. Chains read `item.entity.data.title` (82 sites). Rejected: keeping
  `node` on the two structs (leaves two public entity-sense fields and a guard carve-out) and
  `inner` (less self-describing). — **Reversibility:** costly — two public fields and their JSON
  keys.
- **D-09:** The version family follows the rename: module `entity_version_service`;
  `EntityVersion`, `EntityVersionMetadata`, `EntityVersionRepository`, `EntityVersionService`,
  `InMemoryEntityVersionRepository`; variant `VersioningError::EntityNotFound`; fields
  `entity_id` / `entity_data` on `EntityVersion`, `entity_id` on `VersionHistory` and
  `VersionComparison`, `VersioningConfig.max_versions_per_entity`; `#[serde(alias = "node_id")]`
  and `#[serde(alias = "node_data")]` on `EntityVersion` (its only serializable type).
  `VersionHistory`, `VersionComparison`, `VersioningConfig`, `ChangeType`, `HashAlgorithm` and
  `VersioningError` keep their names. The entity-sense `node_id` rename is confined to the four
  files that carry it; the 1,151 graph-sense `node_id` occurrences are untouched.
  — **Reversibility:** costly — public fields in `paladin-ai-core` plus the root crate's
  `ContentItemService` signatures.
- **D-11:** The serde read-aliases from D-03, D-08 and D-09 are kept for the whole 0.x line;
  removing them needs its own ADR. They are *data* compatibility for persisted JSON and keep no
  removed Rust name resolvable, so they do not conflict with ADR-0059's "no API shim" rule —
  ADR-0060 restates that rule as "no API shim" and carves the aliases out explicitly.

### The base layer (ENTITY-03, ENTITY-04)

- **D-04:** `Entity<T>::version` keeps its name and gains a rustdoc contract, not a feature: the
  flag declares that an entity *wants* revision history and is honoured only by a caller that
  routes writes through `EntityVersionService` (today `ContentItem` via the root crate's
  `ContentItemService`). Removing or renaming the flag was rejected as a break for no behaviour
  change. Platform-wide revisioning stays deferred.
- **D-05:** **All five dormant base items are kept** — `Field` (`base/entity/field.rs`),
  `Collection` / `CollectionType` / `Item` (`base/entity/collection.rs`), the comment-only
  `element` module (`base/component/element.rs`), `FieldVersionService`
  (`base/service/field_version_service.rs`) and `CollectionVersionService`
  (`base/service/collection_versioning_service.rs`; ENTITY-04's `CollectionVersioningService`
  is the module's name, not the type's). Owner's reason (2026-10-13): they are part of the
  `content` / `general data orchestration` system that was started with the base layer and
  never finished; a consumer count must not decide their fate. The review that will decide it
  is `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`. In
  this phase: the reason is written once, in `crates/paladin-core/src/base/mod.rs` rustdoc,
  naming the system and the todo; ADR-0060 records the disposition as *retained pending
  review* with the survey's removal option under Considered Options; nothing in the five files
  changes beyond what the `Node<T>` → `Entity<T>` sweep touches; no §9.2 row (nothing breaks).
  The survey's removal recommendation and its ~1,800-line / coverage argument are void.
  — **Reversibility:** reversible — removal later is a Phase-44-shaped clean break with its own
  ADR.
- **D-10:** The citadel module's placeholder types are **unified with the real ones**:
  `citadel::PaladinData`, `citadel::PaladinStatus` and the duplicate `pub type Paladin =` are
  removed; `PaladinState.paladin` becomes `super::paladin::Paladin` (= `Entity<PaladinData>`);
  the five external importers of the placeholder types
  (`crates/paladin-ports/src/output/citadel_port.rs`, `crates/paladin-memory/src/citadel/
  file_citadel.rs`, `src/application/services/paladin/paladin_builder.rs`,
  `tests/integration/citadel_integration_test.rs`, `examples/battalion_checkpoint_recovery.rs`)
  switch to `platform::container::paladin::{PaladinData, PaladinStatus}`; no re-export is left
  in `citadel` (clean break, same rule as D-02). Wire: the two `PaladinData` structs have the
  same 13 fields in the same order (the real one adds `#[serde(default)]` on the last four, so
  v0.10 snapshots load); the real `PaladinStatus` adds `Reasoning`, so a v0.11 snapshot taken
  mid-reasoning is unreadable by v0.10 — recorded in §9.4 beside D-03's downgrade asymmetry. The
  fixture test's `PaladinState` case proves the v0.10 snapshot still loads through
  `FileCitadel`. — **Reversibility:** costly — a second public break on `paladin-ai-core`
  (two removed types, one changed field type), each a §9.2 row with a measured allowlist entry.

### Vocabulary, record and delivery (ENTITY-05)

- **D-06:** Sequencing and delivery: after Phase 44 (sealed), before Phase 46 (so the
  v0.10 → v0.11 migration guide, CURR-23, carries the rename) and Phase 47 (so v0.11.0 ships
  it). The phase lands on the milestone branch (`claude/laughing-dirac-e0h2ax`) in the same PR
  to `main` as Phases 44-47; no separate PR, no `/gsd-pr-branch` for this phase alone. The
  survey's open question 4 is answered: Phase 44 is already on this branch's lineage.
- **D-07:** The graph side is frozen and the old names stay gone, pinned by
  `tests/entity_rename_guard.rs` in the exact `tests/legacy_removal_guard.rs` shape (std only;
  forbidden strings assembled with `concat!` so the file never contains one whole; two positive
  controls over temporary trees; file-scoped rules report a missing target; the same skip set).
  Rules: (a) no `base::entity::node`, `base::service::node_version_service` path and no
  `Node::new` / `Node<` entity use anywhere; (b) exactly one `pub type Paladin =` in the
  workspace; (c) **every graph identifier under *Not in this phase* is still declared in its
  current file** (`NodeId`, `NodeSpec`, `StateNode`, `NodeContext`, `NodeCache`, `NodeCacheKey`,
  `NodeCachePort`, `NodeInterceptor`, `NodeExecutionRecord`, `NodeOutcomeKind`,
  `NodeProgressKind`, the `TraceEvent::Node*` variants, `wargraph-doc.schema.json`'s `nodes`
  key, the `node_cache` config section) — a moved or renamed definition fails the guard, the
  way Phase 44's rules 5/6 pin their files; (d) the retained base items' paths are **not**
  banned. ADR-0060 names the rule: *"node" = a vertex in a WarGraph/Campaign graph; "Entity" =
  the base wrapper.* CLAUDE.md's crate table and `.github/copilot-instructions.md`'s naming
  table get an `Entity` row with that sentence.
- **D-12:** One ADR. `.planning/decisions/0060-base-entity-rename.md` covers the Phase-48-only
  X-03 supersession, the `Entity` design (D-01, D-03, D-04, D-08..D-11, D-13), the base-layer
  disposition (D-05, D-10), and the vocabulary rule (D-07), with the serde carve-out from D-11
  restated so ADR-0059 and ADR-0060 do not contradict. `PROMOTION.md` gains one row and its
  next-free line advances from 0060 to 0061. Two ADRs were rejected.
- **D-14:** The register lands once, measured: §9.2 rows, allowlist entries and any new
  crate-wide `allow` lines in `crates/paladin-core/Cargo.toml` land in one commit from one D-27
  diagnostic on the final tree (the Phase 44 shape: removals first, register last). CI's
  `semver` job is expected red on the intermediate commits and green from the register commit;
  the branch merges as a whole with the milestone (D-06). Expected lint families:
  `struct_missing` / `enum_missing` / `module_missing` / `trait_missing` /
  `struct_pub_field_missing` on `paladin-ai-core` (D-02, D-08, D-09, D-10) and
  `function_signature_changed` on the root crate's `ContentItemService` (D-09); the exact set is
  measured, never guessed.
- **D-15:** The nine plans written on 2026-10-13 (`453f0ec`) are superseded by this file; the
  planner regenerates them with `/gsd-plan-phase 48`. What changes against them: no removal plan
  (old 48-03 shrinks to the `EntityVersion*` rename plus the `base/mod.rs` rustdoc), the citadel
  unification joins the Stage A plan (old 48-04), the register rows change (old 48-07), the guard
  gains the graph-pin rules and loses the retained-path bans (old 48-08), and the closeout's
  wording fixes (old 48-09) now also correct ENTITY-04's "removed" expectation to "retained
  pending review". Hand-editing the four plans was rejected: their must_haves, file lists and
  wave dependencies were derived from the old premises.

### Claude's Discretion

- **D-13 (planner's call, carried over):** `Entity<T>` lives in a private module
  `base/entity/wrapper.rs` (a git move of `node.rs`) re-exported as `pub use wrapper::Entity;`,
  because `base/entity/entity.rs` declared `mod entity;` fails `clippy::module_inception`
  (verified in research). The module name is free to change if the planner finds better.
- The `entity_version_service` module name; the order of the two rename stages (type/path first,
  then field, per 48-RESEARCH.md Pattern 1) and whether they are one commit or two; the exact
  fixture set beyond the four named in D-03; when in the sequence the D-27 diagnostic runs.
- The docs-only wording fixes owed to inherited errors, done at closeout with IDs unchanged:
  ENTITY-05's `api_surface_current.txt` → `.project/current-exports.txt`; ENTITY-04's
  `CollectionVersioningService` → `CollectionVersionService` and its "removed" option →
  "retained pending review"; ENTITY-01 / ROADMAP's "SQLite/Postgres rows" (no in-tree SQL store
  persists entity JSON; Citadel snapshot files are the real case).
- How `base/mod.rs`'s rustdoc words the retained-system reason, as long as it names the content /
  general-data-orchestration system and the todo path.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The licence and its shape
- `.planning/decisions/0059-legacy-clean-break-removal.md` — ADR-0059: the Phase-44-only X-03
  supersession whose *shape* ADR-0060 copies (never its licence); D-00c "no shim" rule that
  ADR-0060 restates as "no API shim" with the serde carve-out (D-11).
- `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — ADR-0051: the first
  per-phase supersession and the clean-break policy's reasoning (one coordinated downstream
  consumer that adopts a whole milestone at once).
- `.project/v0.10.0/00-program-overview.md` line 44 — corpus rule X-03 as written.
- `.planning/decisions/PROMOTION.md` — ADR index; next free number is **0060** (line 81); the
  row and the advance to 0061 land in the ADR commit (D-12).
- `.planning/decisions/0006-coverage-gate.md` — the 82 % floor (D-00a).
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` — `paladin-core` has zero
  dependencies (D-00d).

### The register
- `MIGRATION.md` §9.1 (behaviour rows, M-B-12 for the downgrade asymmetry), §9.2 (one row per
  `crate | first backticked identifier`; columns `Crate | Type / Trait | Change | Mitigation |
  Deliberate-breaking? | Req`; Crate cell = published name), §9.4 (serde note: v0.11 reads both
  keys, writes the new one; `Reasoning` status), §9.7 (stays empty).
- `.cargo/semver-checks-allowlist.toml` — `[[entry]] crate / lint / migration_row /
  requirement_id / justification`; set-equal with §9.2 `Y` rows via
  `scripts/check-migration-allowlist.sh` (`make check-migration-allowlist`).
- `.project/current-exports.txt` — the gated API-surface baseline (`make api-surface`,
  refreshed by `make api-surface-update`); the research rehearsal measured exactly 14 removed +
  14 added lines for the `EntityVersion*` rename alone, before D-08/D-10 were decided.
- `crates/paladin-core/Cargo.toml` — the crate-wide semver `allow` lines D-14 may extend.
- `CHANGELOG.md` and `crates/paladin-core/CHANGELOG.md` — `[Unreleased]` entries under v0.11.0.

### The guard and the fixtures
- `tests/legacy_removal_guard.rs` — the repository-scan guard to copy verbatim in structure for
  `tests/entity_rename_guard.rs` (D-07): scanned extensions, skip set, `concat!`-assembled
  forbidden strings, positive controls, file-scoped "still declared here" rules.
- `.planning/phases/48-base-entity-rename/48-RESEARCH.md` — Pattern 1 (two-stage scripted
  rename, rehearsed), Pattern 2 (fixture-driven serde compat test, verified to pass on the
  rehearsal tree), Pattern 3 (guard), Pitfalls 1-10 (especially 2: blind `.node` regex hazards;
  4: the citadel placeholder; 6: alias semantics; 8: the hand-fix prose list). Its D-05 removal
  guidance and Pitfall 4's "do not unify" recommendation are **overridden** by D-05 and D-10
  above.
- `.planning/phases/48-base-entity-rename/48-PATTERNS.md` — file-by-file analogs for the guard,
  the fixture test, ADR-0060, the register rows and the wrapper module.
- `.planning/phases/48-base-entity-rename/48-VALIDATION.md` — the phase's test map; its
  ENTITY-04 "removed" expectation needs the D-05 correction when the planner regenerates.

### The code being renamed
- `crates/paladin-core/src/base/entity/node.rs` — `Node<T>` (188 lines): fields `uuid, created,
  modified, data(node), name, version`; derives `Ord`, so field order is wire and ordering
  contract and must not change.
- `crates/paladin-core/src/base/service/node_version_service.rs` — the version family (D-09).
- `crates/paladin-core/src/base/mod.rs` — gets the retained-system rustdoc (D-05).
- `crates/paladin-core/src/platform/container/citadel.rs` lines 44-66 (placeholder types and
  alias) and 126-160 (`PaladinState`) — D-10; `crates/paladin-core/src/platform/container/
  paladin.rs` lines 124-229 — the real `PaladinStatus` / `PaladinData` / `Paladin`.
- `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` — the
  review that gates any later removal of the five retained items (D-05).

### Docs that must agree (ENTITY-05)
- `docs/src/architecture/domain-model.md` (the shown struct is already wrong — rewrite, do not
  just rename), `docs/src/architecture/design-patterns.md`, `docs/src/architecture/crate-map.md`,
  `docs/src/architecture/overview.md`, `docs/src/api-reference/crate-map.md`,
  `docs/src/api-reference/stable-api.md`, `docs/src/appendix/user-rest-api.md`,
  `docs/src/assets/LayerArchitecture.svg` (text labels inside the SVG), `README.md`,
  `crates/paladin-core/README.md`, `src/core/mod.rs` module docs, `CLAUDE.md` crate table,
  `.github/copilot-instructions.md` naming table. The trees in `crate-map.md` / `overview.md`
  that list `collection.rs` / `field.rs` now stay correct (D-05).
- `crates/doc-examples/src/support.rs` and the 13 example / bench files — compiled by
  `make check-api-examples`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tests/legacy_removal_guard.rs`: the whole guard scaffold (walker, skip set, `concat!`
  string assembly, positive controls, file-scoped rules). Copy, do not reinvent.
- Phase 44's register commit (plan 44-11, `e767fc7`) and ADR-0059 (plan 44-02): the exact §9.2
  row shape, allowlist entry shape and ADR section order to reproduce.
- The D-27 semver diagnostic procedure (Phase 38, 39-08, 43-13, 44 precedent): measured lint
  sets, never guessed.
- `48-RESEARCH.md` Pattern 1's two-stage rename script and Pattern 2's compat test — both
  rehearsed on a scratch tree and verified to pass; the compat test's `PaladinState` case covers
  D-10's wire claim.
- `scripts/check-deprecations.sh` (CI `api-surface` job) is **not** engaged: the clean break
  adds no `#[deprecated]` item.

### Established Patterns
- Clean breaks are per-phase licences (ADR-0051, ADR-0059): own ADR, own supersession sentence,
  rows + allowlist as documentation for the downstream refactor, never a shim.
- Same-commit register rule: §9.2 rows and allowlist entries land together, set-equal, from a
  measured diagnostic.
- Serde data compatibility by `alias` is the house pattern for persisted JSON across a rename
  (now explicitly carved out of the "no shim" rule by D-11 / ADR-0060).
- `#![warn(missing_docs)]` + `clippy -D warnings`: every renamed or retained public item keeps
  rustdoc; `clippy::module_inception` forces the `wrapper.rs` layout (D-13).

### Integration Points
- 117 files import `base::entity::node`; ~106 files access the payload as `.node` — the regex
  must exclude `graph.node(`, `progress.node`, `task.node` and the `node_name` / `node_id`
  fields (`cache_key.rs:240` is entity, `superstep.rs:1923` is graph — review engine hits by
  hand).
- Only 8 of 70 `platform/container/` files are built on the wrapper: `paladin.rs`, `citadel.rs`,
  `battalion/council.rs`, `battalion/grove.rs`, `user.rs`, `log.rs`, `content.rs`, `prompt.rs`.
- The version family is consumed only by `src/core/platform/manager/content_service.rs` (root
  crate) and `tests/functional/content_{lifecycle,fetching_pipeline}_test.rs`.
- The citadel placeholder types are imported by the five files listed in D-10; `citadel::Paladin`
  itself is imported by nothing.
- The engine's existing collision sites (`NodeSpec::Paladin { paladin: Box<Paladin> }` in
  `crates/paladin-battalion/src/engine/graph.rs`; `paladin.node` vs `graph.node(id)` vs
  `progress.node` in `engine/superstep.rs`) are where the rename's payoff is visible and where
  the regex is most dangerous.

</code_context>

<specifics>
## Specific Ideas

- Owner, on the five base items (2026-10-13): *"These are all part of the `content` or `general
  data orchestration` system but that remains unfinished. We need a follow-up todo to fully review
  this system and what is needed to determine if these need to be removed."* — the reason D-05
  quotes in `base/mod.rs`.
- The owner wants "node means one thing" to be fully true, not 99 %: persisted JSON keys change
  (D-03), the two wrapper-struct fields change (D-08), the version family's fields change (D-09),
  and the citadel placeholder goes (D-10).
- The guard should make ENTITY-05's "untouched" claim mechanical, not just asserted (D-07c).

</specifics>

<deferred>
## Deferred Ideas

- **Removing any of the five retained base items** — gated on the review in
  `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`; a later
  milestone, Phase-44-shaped if the review says remove.
- **Platform-wide revisioning.** The original base design wanted every entity revisionable. Today
  only `ContentItem` is, and `Assistant` versions itself. Whether revisioning belongs on
  `Entity<T>` or stays per-aggregate is part of the same review; this phase only documents the
  current contract (D-04).
- **Splitting `platform/container/`.** 70 files, of which 8 are base-derived and the rest are
  self-standing records. Out of scope here.
- **Migrating the owned downstream system.** It adopts the whole milestone at once (clean-break
  policy); the §9.2 rows are its refactor checklist.
- **Renaming `platform/container/log.rs`'s `Log = Entity<LogContainer>`** to something that does
  not read like a log *line* — noticed, not in scope.
- **Removing the serde read-aliases** — needs its own ADR, not before the 0.x line ends (D-11).

### Reviewed Todos (not folded)
- `2026-10-11-phase-44-review-follow-ups.md` — Battalion / Commander docs and crate-wide semver
  allows from the Phase 44 review; only overlap is that D-14 may also touch the crate-wide allow
  lines. Stays with Phase 44's follow-ups.
- `2026-10-11-reject-removed-v0-10-battalion-config-keys.md` — Battalion config key rejection;
  unrelated to the base layer.
- `2026-08-13-verify-local-coverage-reproduction.md` — coverage walkthrough; dispositioned by
  Phase 36.1; unrelated.

</deferred>

<open_questions>
## For the owner — all answered 2026-10-13

1. Name: `Entity<T>` / `.data` — answered, D-01.
2. Old name's fate: clean break, no alias; ADR-0060; §9.7 stays empty — answered, D-02.
3. Dormant base items: **keep all five** pending the content-system review — answered, D-05
   (overrides the survey's recommendation).
4. PR shape: milestone branch, same PR as Phases 44-47 — answered, D-06.

Raised and answered in the same session: JSON key follows the Rust field (D-03); wrapper-struct
field is `entity` (D-08); version-family fields rename (D-09); citadel placeholder unified
(D-10, overrides the survey); aliases kept for the 0.x line (D-11); one ADR (D-12); guard pins
the graph definition sites (D-07); plans regenerated (D-15).
</open_questions>

---

*Phase: 48-base-entity-rename*
*Context gathered: 2026-10-13*
