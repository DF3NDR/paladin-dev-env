# ADR-0060: The base-layer wrapper becomes Entity<T>, "node" is reserved for the graph, and the rename lands as a clean break in v0.11.0

## Status

Accepted

**Date:** 2026-10-13

**Phase:** 48 (Base Entity Rename), plan 48-02; Code Conformance is measured at the closeout, plan 48-09.

## Context

The word "node" means two unrelated things in this tree. In the base layer it names the generic wrapper `paladin_core::base::entity::node::Node<T>` that carries `uuid`, `created`, `modified`, `name`, a payload field also called `node`, and a `version` flag; `Paladin`, `Council`, `Grove`, `User`, `Log`, `ContentItem` and `PromptItem` are built on it, and the survey counted 117 import sites and roughly 650 payload accesses. In the engine it names a vertex of a WarGraph or Campaign graph (`NodeId`, `NodeSpec`, `StateNode`, `NodeContext`, the node cache, `TraceEvent::Node*`), a vocabulary about five times larger that arrived with the v0.10.0 superstep engine and is part of public wire formats: `docs/schemas/wargraph-doc.schema.json`, the trace stream, persisted Waypoints and `NodeExecutionRecord`, and the Redis node-cache keys.

The two senses already collide in the code. `NodeSpec::Paladin` is a graph node that wraps an entity, so its rustdoc has to say "a `Node<PaladinData>`", and a single engine file holds payload access (`paladin.node`), graph lookup (`graph.node(node_id)`) and a progress id comparison side by side. The entity side is renamed and the graph side is kept, because the graph sense is the one every reader and dependency expects, it is the larger one, and it sits in wire formats where a rename breaks JSON schemas, trace streams, cache keys and configuration rather than only Rust. The entity rename is mechanical, and the module is already called `base::entity`.

Corpus rule X-03 ("deprecations allowed, removals are not") still governs the tree. ADR-0051 superseded it for Phases 31-33 only and ADR-0059 superseded it for Phase 44 only, and the Out of Scope entry in `PROJECT.md` says a later removal records its own supersession and does not inherit an earlier one. Phase 48 removes public API that the published `paladin-ai-core` 0.10.1 exports (`Node<T>`, the `node` module, the `NodeVersion*` family, the citadel module's placeholder types), so it needs its own record. This ADR is that record. ADR-0051 and ADR-0059 are the shape it copies (a scope sentence, a "what still applies" list, a version identity), never its licence.

The decisions below are the owner's answers at `/gsd-discuss-phase 48` on 2026-10-13 (D-00a..D-00e, D-01..D-12, D-14, D-15 in `48-CONTEXT.md`), which superseded the earlier survey draft (D-15), plus D-13, which `48-CONTEXT.md` leaves to the planner under Claude's Discretion. Where an owner decision carries a reversibility rating in `48-CONTEXT.md`, the subsection below repeats it. No decision was re-opened. The shipped tree outranks any document (D-00a): two details in the survey did not survive measurement and are corrected where they appear, namely that the gated API-surface baseline is `.project/current-exports.txt` (the `api_surface_current.txt` named in ENTITY-05 is a stale file no gate reads) and that the graph-sense `node_cache` configuration is defined by `src/config/node_cache.rs` rather than by a section of `config.example.yml`.

## Decision

### (a) X-03 is superseded for Phase 48 only (D-02, D-00a, D-00b, D-00c, D-00d, D-00e)

**X-03 is superseded for Phase 48 only**, and exactly for the removals recorded in subsections (b), (d), (e) and (h) below: the base wrapper's old name and module, the old version family, the citadel module's placeholder types and alias, and the two wrapper fields. Every other public API in the tree is still governed by X-03 exactly as written. A later removal records its own supersession; it does not inherit this one. ADR-0051 and ADR-0059 appear here only as the shape this record copies, never as its licence, and no Phase 48 removal cites either of them or treats Phase 48 as covered by an earlier supersession. `MIGRATION.md` 9.7 (Deprecations) stays empty, because nothing in this phase is deprecated: the old names are removed outright.

**What still applies.**

- **D-00a.** The shipped tree outranks any document. Gates are unchanged: the 82 percent workspace line-coverage floor (ADR-0006), `make clean-code`, `make api-surface` (with `make api-surface-update` and a CHANGELOG entry for the intentional surface change, against `.project/current-exports.txt`), `make security`, `make check-api-examples`, and the manual credential-handling review in `security.instructions.md`.
- **D-00b.** Every deliberate break gets a `MIGRATION.md` 9.2 row and, when the row is marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry per lint naming one published crate name (`paladin-ai-core` for `crates/paladin-core`), set-equal in both directions (`make check-migration-allowlist`). The lints are measured by the D-27 diagnostic on the final tree, never guessed.
- **D-00c.** The two-officer token model and the Treasurer vocabulary (ADR-0049, ADR-0050, Phase 30) are untouched. This ADR adds a vocabulary rule and revisits none.
- **D-00d.** `paladin-core` has zero dependencies (ADR-0015); nothing in this phase adds one.
- **D-00e.** Phase 44 is sealed and touched none of this phase's files; its guard `tests/legacy_removal_guard.rs` stays green on every push.
- **The rule is `no API shim`** (a restatement for this phase of the rule ADR-0059 applied to its own). There is no `#[deprecated]` bridge, no type alias, no re-export that keeps a removed name resolvable. Rows and allowlist entries are documentation for the downstream system's refactor, never a compatibility layer. **The one carve-out is data, not API (D-11):** the serde read-aliases in subsections (c), (d) and (e) exist so that JSON already persisted by v0.10 keeps loading. They are not a deprecation, they keep no removed Rust name resolvable, and they are no reason for the downstream system to skip its migration of Rust code.

**Why the deprecated-alias alternative was rejected (D-02).** A `#[deprecated] pub type Node<T> = Entity<T>` would keep the old type name and the old module path resolvable, and nothing more. The payload field rename `node` to `data` cannot be aliased in Rust: a field has one name, so every payload access (`paladin.node.name`), every struct literal that names the old field, and the `ContentItem` / `PromptItem` field still break downstream. Each surviving use of the alias would additionally raise a deprecation warning, which fails the downstream system's own `-D warnings` build. The alias would save the type name only, at the price of a non-empty 9.7 and a second removal later, so the clean break is the cheaper honest option. **Reversibility: one-way** once v0.11.0 is published (Phase 47): `paladin-ai-core` 0.10.1 on crates.io exports the old name, and the downstream consumer migrates against the published surface.

### (b) `Entity<T>` (D-01, D-13)

The wrapper is `Entity<T>` with public path `paladin_core::base::entity::Entity` and payload field `data`. The name is generic on purpose: the base layer predates the Medieval Military theme and is meant to sit under any system, and the theme stays on domain concepts. The struct lives in a private module `base/entity/wrapper.rs` (a git move of `node.rs`) re-exported as `pub use wrapper::Entity;`, because a module named `entity` declared inside `base/entity/` fails `clippy::module_inception` (D-13, the planner's call; the module name is free to change if a better one is found). The field set and order stay `uuid, created, modified, data, name, version`: the order is the wire order and the derived `Ord` contract and must not change. `Paladin`, `Council`, `Grove`, `User` and `Log` become aliases of `Entity<...>`. **Reversibility: costly** (D-01): 117 import sites and about 650 payload accesses change, and the name is in the published `paladin-ai-core` surface once v0.11.0 is tagged.

### (c) Persisted JSON (D-03, D-11)

Serde compatibility is by read-alias, not by version bump. The payload field carries `#[serde(alias = "node")]`: a document with the old key or the new key loads; a document with both keys is rejected with `duplicate field`; a document with neither is rejected with `missing field`; new writes emit only `data`. No `schema_version` is added to `Entity<T>`, and the `#[serde(rename = "node")]` option that would keep the old key on the wire forever was rejected, because the owner wants persisted documents to stop saying "node" in the entity sense too. Downgrade is asymmetric: a v0.10 binary cannot read JSON written by v0.11 (`MIGRATION.md` 9.1 `M-B-12` and 9.4 say so). No in-tree SQL store persists entity JSON; the real persisted case is Citadel snapshot files, so the fixture tests (plan 48-01) load captured pre-rename `Paladin`, `PaladinState`, `ContentItem`, `PromptItem` and version-record JSON from the unmodified tree. The aliases stay for the whole 0.x line; removing any of them needs its own ADR (D-11). **Reversibility: one-way for downgrade** (D-03).

### (d) The wrapper fields (D-08)

`ContentItem.node` and `PromptItem.node` become `entity`, each with `#[serde(alias = "node")]`. Chains read `item.entity.data.<field>` (82 sites). Keeping `node` on the two structs would leave two public entity-sense fields and a guard carve-out, and `inner` is less self-describing. **Reversibility: costly** (D-08): two public fields and their JSON keys.

### (e) The version family (D-09)

The module becomes `entity_version_service` (a git move of `node_version_service.rs`). The types become `EntityVersion`, `EntityVersionMetadata`, `EntityVersionRepository`, `EntityVersionService` and `InMemoryEntityVersionRepository`; the variant becomes `VersioningError::EntityNotFound`; the fields become `entity_id` and `entity_data` on `EntityVersion`, `entity_id` on `VersionHistory` and `VersionComparison`, and `VersioningConfig.max_versions_per_entity`. `EntityVersion`, the family's only serializable type, carries `#[serde(alias = "node_id")]` and `#[serde(alias = "node_data")]` for the v0.10 keys. `VersionHistory`, `VersionComparison`, `VersioningConfig`, `ChangeType`, `HashAlgorithm` and `VersioningError` keep their names. The entity-sense `node_id` rename is confined to the four files that carry it; the roughly 1,150 graph-sense `node_id` occurrences are untouched. **Reversibility: costly** (D-09): public fields in `paladin-ai-core` plus the root crate's `ContentItemService` signatures.

### (f) The revisioning contract (D-04)

`Entity<T>::version` keeps its name and gains a rustdoc contract, not a feature. The flag declares that an entity wants revision history; only `EntityVersionService::create_version` reads it (it returns `VersioningError::VersioningDisabled` when the flag is false), and it is honoured only by a caller that routes writes through that service. Today only `ContentItem` is revisioned, through the root crate's `ContentItemService`. `Assistant` versions itself with its own `AssistantVersion`. Removing or renaming the flag was rejected as a break for no behaviour change, and platform-wide revisioning is deferred.

### (g) The base-layer disposition: retained pending review (D-05)

Five base items have no consumer outside `crates/paladin-core/src/base/`: `Field` (`base/entity/field.rs`); `CollectionType` and `Item` with the collection types in `base/entity/collection.rs`; the comment-only `element` module (`base/component/element.rs`); `FieldVersionService` (`base/service/field_version_service.rs`); and `CollectionVersionService` (`base/service/collection_versioning_service.rs`; ENTITY-04's `CollectionVersioningService` is the module's name, not the type's). All five are **retained pending review**. The owner's reason (2026-10-13): they are part of the content / general data orchestration system that was started with the base layer and never finished, and a consumer count must not decide their fate. The review that will decide it is `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`. The reason is written once, in the `crates/paladin-core/src/base/mod.rs` rustdoc (plan 48-03). Nothing in the five files changes beyond what the rename sweep touches (the version module's import path and prose that names the old wrapper). No 9.2 row is written for them, because nothing breaks. The survey's removal recommendation and its line-count and coverage argument are void and appear under Considered Options only. **Reversibility: reversible** (D-05): a later removal would be a Phase-44-shaped clean break with its own ADR.

**Bounded exception to the vocabulary rule.** Because the five items are not edited beyond the sweep, these entity-sense identifiers stay inside them until the review decides the items' fate: `Field.nid`, `FieldVersion.node_id`, `FieldVersionHistory.node_id`, `FieldVersionComparison.node_id`, `FieldVersionRepository::get_node_field_versions` and `FieldVersionService::get_node_field_versions`. They are the one stated gap in rule (i), they are listed in the review todo so the review decides them rather than discovers them, renaming them would be a public break on `paladin-ai-core` with its own 9.2 rows, and the guard does not ban them (rule family (d) in subsection (i)).

### (h) The citadel unification (D-10)

The citadel module's placeholder `PaladinData` and `PaladinStatus` and its duplicate `pub type Paladin =` are removed, with no re-export left in `citadel`. `PaladinState.paladin` becomes the real `Paladin` (`Entity<PaladinData>` from `platform::container::paladin`). The importers of the placeholder types switch to `platform::container::paladin::{PaladinData, PaladinStatus}`; the measured list is in plan 48-04's SUMMARY, because the survey's list of five no longer matched the tree at planning (one listed example imports none of the types). Wire identity: the two `PaladinData` structs have the same 13 fields in the same order, and the real one adds `#[serde(default)]` on the last four, so v0.10 snapshots load, which the `PaladinState` fixture proves through `FileCitadel`. The real `PaladinStatus` adds a `Reasoning` variant, so a v0.11 snapshot taken mid-reasoning is unreadable by v0.10; this downgrade asymmetry is recorded in `MIGRATION.md` 9.4 beside the one in subsection (c). **Reversibility: costly** (D-10): a second public break on `paladin-ai-core` (two removed types, one changed field type), each with a 9.2 row and a measured allowlist entry.

### (i) The vocabulary rule (D-07)

**"node" means a vertex in a WarGraph or Campaign graph; "Entity" means the base-layer wrapper `Entity<T>` that `Paladin`, `Council`, `Grove`, `User`, `Log`, `ContentItem` and `PromptItem` are built on.** `Entity` is a deliberately generic base-layer term, not a Medieval Military one; the theme stays on domain concepts.

The graph vocabulary is frozen and this phase changes none of it: `NodeId`, `NodeSpec`, `StateNode`, `NodeContext`, `NodeCacheKey`, `NodeCachePort`, `NodeCacheError`, `InMemoryNodeCache`, `RedisNodeCache`, `NodeCacheConfig` and `NodeCacheBackend` (defined in `src/config/node_cache.rs`), `NodeInterceptor`, `NodeExecutionRecord`, `NodeOutcomeKind`, `NodeProgressKind`, `NodeDoc`, `TraceEvent::NodeStarted` / `NodeProgress` / `NodeFinished`, the structured `NodeError` (the only one since Phase 44), and `docs/schemas/wargraph-doc.schema.json` with its `nodes` and `max_node_visits` keys.

The rule is enforced by `tests/entity_rename_guard.rs` (plan 48-08), in the exact shape of `tests/legacy_removal_guard.rs` (std only, forbidden strings assembled with `concat!`, positive controls over temporary trees, file-scoped rules that report a missing target). It has four rule families: (a) the old entity paths and generic forms are absent (`base::entity::node`, `base::service::node_version_service`, `Node::new`, `Node<` as an entity); (b) exactly one `pub type Paladin =` in the workspace, plus one `PaladinData`, one `PaladinStatus` and one `NodeError`; (c) every graph identifier above is still declared in its current file, so a moved or renamed definition fails the guard; (d) the retained base items' paths are not banned. The `CLAUDE.md` crate table and the `.github/copilot-instructions.md` naming table gain an `Entity` row carrying the sentence above (plan 48-06).

### (j) The register (D-00b, D-14)

One 9.2 row per `crate | first backticked identifier` pair, because the set-equality gate keys a row on the first backticked identifier of its Type cell. The gated baseline is `.project/current-exports.txt`. The rows, the allowlist entries and any new crate-wide `allow` lines in `crates/paladin-core/Cargo.toml` land in one commit from one D-27 diagnostic on the final tree (plan 48-07), the Phase 44 shape: removals first, register last. CI's `semver` job is expected red on the intermediate commits between plan 48-03 and plan 48-07 and green from the register commit. Expected lint families are `struct_missing`, `enum_missing`, `module_missing`, `trait_missing` and `struct_pub_field_missing` on `paladin-ai-core`, and `function_signature_changed` on the root crate's `ContentItemService`; the exact set is measured. `scripts/check-deprecations.sh` is not engaged, since no `#[deprecated]` item is added.

### (k) Sequencing, delivery and one record (D-06, D-12, D-15)

The phase runs after Phase 44 (sealed) and before Phases 46 and 47, so the v0.10 to v0.11 migration guide (CURR-23) carries the rename and the v0.11.0 tag ships it. It lands on the milestone branch in the same pull request to `main` as Phases 44-47, with no separate pull request. This is the single ADR for the supersession, the design, the disposition and the vocabulary rule (D-12): two ADRs were rejected. The nine plans written before the owner's answers were superseded by them, and this plan set was regenerated from `48-CONTEXT.md` (D-15).

## Considered Options

- `Record<T>` as the wrapper name (rejected: collides with `VaultRecord` in prose).
- `Versioned<T>` (rejected: over-promises, the flag is only honoured through `EntityVersionService`, see D-04).
- `Aggregate<T>` (rejected: `Paladin` is the aggregate root, the wrapper is not).
- A themed Medieval Military name for the wrapper (rejected: the base layer predates the theme and is meant to sit under any system).
- A `#[deprecated]` type alias for `Node<T>` kept for one release (rejected: a type alias keeps only the type name and path, the payload field rename cannot be aliased in Rust, so every payload access and struct literal still breaks, and each surviving use fails the downstream `-D warnings` build; it would also make 9.7 non-empty).
- `#[serde(rename = "node")]` to keep the old wire key forever (rejected: the owner wants persisted documents to stop saying "node" in the entity sense).
- A `schema_version` on the wrapper (rejected: a read-alias gives the same load behaviour with no new field on every entity).
- A hand-written `Deserialize` or a JSON pre-rewriter for old documents (rejected: more code and more failure modes than one serde attribute, and the attribute already rejects ambiguous documents).
- Keeping the field name `node` on `ContentItem` and `PromptItem` (rejected: leaves two public entity-sense fields and a guard carve-out).
- `inner` as that field's name (rejected: less self-describing than `entity`).
- Keeping `node_id`, `node_data` and the old not-found variant in the version family (rejected: they are entity-sense names and would survive the rename as exactly the confusion it removes).
- Removing or renaming the `version` flag (rejected: a break for no behaviour change; the flag gets a rustdoc contract instead).
- Removing all five unused base items, the survey's recommendation with its line-count and coverage argument (void: the owner kept them pending the content / general data orchestration review, D-05; the review todo holds any later case for removal).
- Keeping some of the five for the downstream system only (rejected: the owner's reason covers all five, and a consumer count must not decide their fate).
- Deleting the citadel `Paladin` alias but keeping the placeholder `PaladinData` and `PaladinStatus` (48-RESEARCH.md Pitfall 4; rejected by the owner, D-10: it would leave two types with one name's worth of meaning in the tree).
- A module named after its parent, `base/entity/entity.rs` (rejected: fails `clippy::module_inception`; hence `wrapper.rs`).
- Two ADRs, one for the supersession and one for the design (rejected: the design is the reason for the supersession and the two would drift).
- Registering breaks per plan instead of once (rejected: intermediate lint sets are guesses; one measured diagnostic on the final tree is exact).
- A guard that bans the old paths only, without pinning the graph definitions (rejected: it could not catch a graph identifier being moved or renamed by mistake).
- A separate pull request for this phase (rejected: the migration guide and the tag in Phases 46 and 47 must carry the rename, so it travels with the milestone).
- Removing the serde read-aliases at v0.12.0 (rejected: they stay for the whole 0.x line, and removal needs its own ADR, D-11).
- Renaming the graph sense instead of the entity sense (rejected: the graph sense is about five times larger and sits in public wire formats).

## Code Locations

- `crates/paladin-core/src/base/entity/{mod,wrapper}.rs` - the wrapper module (git move of `node.rs`) and its re-export.
- `crates/paladin-core/src/base/mod.rs` - the single rustdoc statement of the retained-items reason.
- `crates/paladin-core/src/base/service/entity_version_service.rs` - the renamed version family (git move of `node_version_service.rs`).
- `crates/paladin-core/src/platform/container/{content,prompt,citadel,paladin,user,log}.rs` and `crates/paladin-core/src/platform/container/battalion/{council,grove}.rs` - the aliases, the wrapper structs and the citadel unification.
- `src/core/platform/manager/content_service.rs` - the root crate's `ContentItemService`, the one wired consumer of the version family.
- `crates/paladin-memory/src/citadel/file_citadel.rs` and `crates/paladin-ports/src/output/citadel_port.rs` - importers of the citadel placeholder types.
- `crates/paladin-core/tests/entity_serde_compat.rs` and `crates/paladin-core/tests/fixtures/pre_rename/` - the persisted-JSON fixtures and their tests.
- `tests/entity_rename_guard.rs` - the repository-scan guard.
- `MIGRATION.md` 9.1, 9.2, 9.4 and 9.7, `.cargo/semver-checks-allowlist.toml`, `crates/paladin-core/Cargo.toml`, `.project/current-exports.txt`, `CHANGELOG.md` and `crates/paladin-core/CHANGELOG.md` - the register, the allowlist, the measured lint lines, the API baseline and the changelogs.
- `.planning/decisions/0060-base-entity-rename.md`, `.planning/decisions/PROMOTION.md`, `.planning/PROJECT.md` and `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` - this record, its index entries and the review it names.
- `.planning/phases/48-base-entity-rename/48-CONTEXT.md` and `.planning/phases/48-base-entity-rename/48-RESEARCH.md` - the sources of the decisions and the measurements.

## Code Conformance

must change -- the rename is not yet in the tree. It is executed under ENTITY-01 to ENTITY-05 by plan 48-03 (the version family and the retained-items rustdoc), plan 48-04 (the citadel unification and the type rename), plan 48-05 (the payload and wrapper-field rename and the contract rustdoc), plan 48-06 (the documentation and naming tables), plan 48-07 (the register) and plan 48-08 (the guard). Plan 48-09 measures the result, replaces this verdict with `conforms` and records the evidence.

## Downstream Consumers

- The owned downstream system - pins a submodule pointer, adopts the milestone at once, and follows the 9.2 rows as its refactor checklist for `Entity<T>`, the `data` and `entity` fields and the `EntityVersion*` family.
- Phase 46 (CURR-23) - the v0.10 to v0.11 migration guide must carry the rename and both downgrade notes (the entity JSON keys and the `Reasoning` status).
- Phase 47 - the v0.11.0 tag is the point at which the published contract changes.
- Operators holding Citadel snapshot files - old snapshots keep loading, new ones are not readable by a v0.10 binary (`MIGRATION.md` 9.4).
- The content / general data orchestration review (`.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`) - decides the fate of the five retained base items and of the entity-sense identifiers listed in subsection (g).
